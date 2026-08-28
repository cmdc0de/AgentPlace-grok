# M18 — Weighted council, SetCouncil meta-rule, jump-to-tick catch-up

**Status:** planned (not yet implemented)  
**Depends on:** M17 complete (`docs/M17-plan.md`, git tag `M17`, commit `4d7112d`)  
**Specs:** `memory-goals-incentives-spec.md` §2 (council / weighted-by-status), `M16-plan.md` (council is unanimous among listed living ids), `M17-plan.md` (`SetVoteAccept` cannot rewrite `council = [...]`), `M14-plan.md` (scrubber loads a `.ckpt` at-or-before T; no jump without a file)

## Context

`accept = "council"` is still **unanimous among listed living ids**; `[voting] weight` is ignored there. M17 can `SetVoteAccept { council }` but cannot rewrite the roster. `/scrub TICK` loads the ckpt **at or before** T and stops (`sim.tick` may be &lt; T).

M18 does **not** rewrite postcard, add TLS, wire Give, or bump `PROTOCOL_VERSION`.

## Goal

A researcher can:

1. Set `[voting] accept = "council"` + `council_tally = "majority"` so **living council** votes with `weight` (equal | influence | respect) and the existing acceptance threshold (config / meta / incentive modifier). Omit `council_tally` → M16 unanimous council.
2. With `allow_meta_rules = true`, Propose **SetCouncil { ids }**; when Accepted, that list is the runtime council (last-wins). Overlay `council = [...]` is the default until a SetCouncil is adopted.
3. `--load DIR` + `/scrub TICK` (and the ckpt slider): load the ckpt at-or-before T, then **tick the in-process sim forward** to T. Not interpolation. `/ckpt next|prev` stay file-to-file. JSONL `/events` stays display-only.
4. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`. Shipping `default.toml` / `coop.toml` unchanged ⇒ default hashes unchanged.

## In scope

### A. Weighted council (overlay)

Same `[voting]` overlay as M13–M16 (not `ExperimentConfig`, no postcard `config_hash` change):

```toml
[voting]
weight = "equal"              # equal | influence | respect
accept = "council"
council = [0, 1, 2]
council_tally = "majority"    # omit | "unanimous" = M16; "majority" uses weight
```

| `accept` / `council_tally` | Who decides | Weight |
|---|---|---|
| `majority` (population) | unchanged M13/M15 | yes |
| `unanimous` | every living agent | ignored |
| `council` omit / `"unanimous"` | every **living** council id Supports; any living council Oppose Rejects | ignored (M16) |
| `council` + `"majority"` | majority among **living council** only, `yes ≥ need` / `no ≥ need` | equal \| influence \| respect |

- Unknown `council_tally` → **load error**.
- `council_tally` set when `accept` is not `council` → **load error** (fail loud).
- `accept = "council"` still requires a non-empty overlay `council` list.
- Majority-of-council **total** = sum of weights of **living council ids**, not the whole population. `need = ceil(threshold * that total).max(1)`. Threshold is the existing `proposal_threshold` (config default, replaced by meta `SetAcceptanceThreshold`, plus incentive modifier).
- Non-council Support/Oppose still update the sets (hashed / display) but their weight is **0** for this tally.
- Empty living council (all dead): stay **Open** until expiry (same as M16).
- Do **not** add `[voting]` to shipping `configs/default.toml`.
- Do **not** add `VoteAccept::CouncilMajority` (avoids a postcard enum bump on `SetVoteAccept`). Tally mode is overlay-only this slice; no meta-rule for `council_tally`.

Kingmaker: 3-person council, `weight = "influence"`, agent 0 is the influence king, only 0 Supports → **Accepted**. Equal `council_tally = "majority"`, 2 of 3 Support → **Accepted**; 1 of 3 → **Open**. Omit `council_tally`, 2 of 3 → still **Open** (M16).

### B. Meta-rule council membership

`[proposals] allow_meta_rules = true` in the **experiment** TOML (already a field; default false). Do **not** flip shipping `configs/default.toml`.

**New `StructuredRule` variant (append only — after `SetVoteAccept`):**

| Rule | Meaning when **Accepted** |
|---|---|
| `SetCouncil { ids }` | Runtime `council` list (agent ids). Last-wins (`later tick_accepted` wins). Used whenever `effective_vote_accept()` is `Council`. Dormant if accept is majority/unanimous. |

- `allow_meta_rules = false` → Propose SetCouncil is **illegal** (Wait).
- Empty / missing `ids` at Propose → **Wait** (same spirit as overlay requiring a non-empty list).
- Duplicate ids: **dedupe, first-seen order**.
- Dead ids stay on the list; tally drops them (not a load error).
- `hash_rule` tag **8**, then `u32` count, then each id `u64` LE. Old `.ckpt` loadable.
- LLM `parse_rule`: `set_council` / `setcouncil`; JSON field `council` array of ints. Unknown still Wait.
- `Simulation.meta_council: Option<Vec<AgentId>>` filled in `refresh_meta` (already called on ckpt load). `effective_council()` = meta unwrap_or overlay `voting.council`.
- No meta-rule this slice to rewrite `council_tally`.

### C. Jump-to-tick catch-up (in-process)

`--load DIR` (or a file whose parent has ckpts). **`/scrub TICK`** and the imgui **ckpt tick** slider:

1. `ckpt_at_or_before(dir, want)` as today.
2. If `state.sim.tick == want`, **no-op** (idempotent; do not reload).
3. Else load that ckpt (replace sim, pause).
4. If `sim.tick < want`, call `tick()` / `step_once` until `sim.tick == want` or `tick()` returns false (`max_ticks` / empty).

- **Not interpolation.** Honest continuation of the loaded checkpoint (mock ⇒ deterministic; replay_file ⇒ replay; live LLM ⇒ may hit the network — CI tests use mock).
- `/ckpt next|prev` and `[` `]` stay **adjacent ckpt files** (no catch-up).
- `/events` JSONL slider stays **display-only** (does not tick, does not load).
- Remote attach still **refuses** scrub (no new `ControlVerb`).
- Slider range stays first ckpt tick .. last ckpt tick (dragging 50 with files at 40 and 80 already calls `apply(50)`; M18 makes that land on 50).
- `/scrub` may request a tick **after** the last ckpt (catch-up until `tick()` stops). Help text says so.
- `loaded_tick` = `state.sim.tick` after catch-up (so next/prev still work).
- Vegetation / world meshes may stay stale after a jump (same as M14). Long catch-up may hitch; no extra recording.

## Out of scope (later)

| Later | What |
|---|---|
| After M18 | protobuf/TLS; wire Give; `/set respect`; revert relationship_delta on leave; meta-rule `council_tally`; jump-to-tick on the wire |
| Not M18 | Browser; combat; CI Win/mac; extra LLM reflection/embeddings; `PROTOCOL_VERSION` bump |

## Key decisions

1. **M16 council contract.** Omit / `council_tally = "unanimous"` stays stance-complete; majority-of-council is opt-in.
2. **Council majority total** is living-council weight, not population weight.
3. **SetCouncil** is append-only `StructuredRule` tag 8; last-wins; gated by `allow_meta_rules`.
4. **No `VoteAccept::CouncilMajority`** and no meta-rule for `council_tally` this slice.
5. Jump-to-tick is **tick-forward from a ckpt**, not interpolation, in-process only.
6. Do not change shipping `configs/default.toml` / `coop.toml`.
7. Mock CI. `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## Tests (M18 acceptance bar)

| Test | Asserts |
|---|---|
| omit `council_tally` / `"unanimous"` | council `[0,1]`, both Support, third silent → **Accepted** (M16) |
| `council_tally = "maybe"` | load error |
| `council_tally = "majority"` + `accept = "majority"` | load error |
| equal majority-of-council, 3 ids, 2 Support | **Accepted** |
| equal majority-of-council, 1 of 3 Support | **Open** |
| influence majority-of-council, king 0 Supports only | **Accepted** |
| `allow_meta_rules = false` | SetCouncil Propose → Wait |
| meta SetCouncil `[0,2]` then accept=council unanimous | 0 and 2 Support, 1 silent → **Accepted**; 2 Oppose → **Rejected** |
| empty SetCouncil Propose | Wait |
| `/scrub` ckpts at 2 and 4, want 3 | `sim.tick == 3`; hash matches ticking the tick-2 ckpt one step |
| `/scrub 3` twice | same tick/hash (no extra events) |
| `/ckpt next` from 3 | lands on file tick 4 (no catch-up) |
| remote `/scrub` | refused |
| default.toml mock | hashes match pre-M18 |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Weighted council overlay

- **Files:** `voting.rs` (`council_tally`), `simulation.rs` council majority path (`tick_lifecycle` with council-only weights), governance tests

### PR 2: SetCouncil meta-rule

- **Files:** `board.rs` `StructuredRule` + `hash_rule` tag 8, `execute.rs` Propose gate, `llm.rs` parse_rule + `RuleJson.council`, `simulation.rs` `meta_council` / `effective_council`, `report.rs` label, governance tests

### PR 3: Jump-to-tick catch-up

- **Files:** viewer `CkptScrubber::apply` + help text + tests (`crates/viewer` scrubber test), this plan + `M18-test-plan.md` when implemented

## Config / CLI

No new `ControlVerb`. No new `ExperimentConfig` postcard fields. Overlay `[voting] council_tally` only. `allow_meta_rules` already on ExperimentConfig (hashed when true).

```bash
cargo test -p sim-core
cargo test -p viewer
```

## Verification (when implemented)

Walkthrough: `docs/M18-test-plan.md` (written at implement).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: omit `council_tally` matches M16 unanimous council; `council_tally = "majority"` equal 2-of-3 Accepts; influence kingmaker on a 3-person council; SetCouncil Propose waits unless `allow_meta_rules`; `/scrub` between ckpt files lands on the requested tick; default mock hashes match.

## Risks

- **Postcard enum append.** `SetCouncil` must stay at the **end** of `StructuredRule`. Do not reorder `VoteAccept`.
- **M16 contract.** Default council tally stays unanimous; majority-of-council is opt-in `council_tally`.
- **Double-apply on `/scrub`.** Short-circuit when `sim.tick == want` so catch-up is not replayed.
- **Catch-up LLM.** Live provider may call the network; tests use mock. Do not force-mock in the viewer.
- **Do not add `ControlVerb` or bump `PROTOCOL_VERSION`.**
