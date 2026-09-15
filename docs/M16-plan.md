# M16 — Council/unanimous votes, `/set`, range-limited board

**Status:** implemented  
**Depends on:** M15 complete (`docs/M15-plan.md`, git tag `M15`, commit `7b00386`)  
**Walkthrough:** [`M16-test-plan.md`](M16-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` §2 (unanimous / council), `M6-plan.md` (`/set` deferred), `decision-observation-llm-economy-metrics-spec.md` §2 (`public_board_always_visible`), `M13-plan.md` / `M15-plan.md` (majority overlay)

## Context

M13/M15 tally is **majority** with `equal` | `influence` | `respect`. The spec’s other acceptance modes — **unanimous** and **council** — are still majority-of-population. `/set` is still missing (M6 deferred it; `/give` exists). `public_board_always_visible = false` currently returns an **empty** board, not a range-limited one.

M16 does **not** rewrite postcard, add TLS, wire Give, or bump `PROTOCOL_VERSION`.

## Goal

A researcher can:

1. Leave `[voting] accept` omitted / `"majority"` so default hashes stay the same.
2. Set `accept = "unanimous"` (every living agent must Support) or `accept = "council"` with `council = [0, 1]` (unanimous among those living ids).
3. Type `/set ID hunger|thirst|energy|influence N` in the in-process viewer to stage an A/B (hash-sensitive; remote refuses).
4. Set `public_board_always_visible = false` and have Observation/prompt show **nearby** open proposals (identity range) instead of nothing; Support/Oppose legal only for those; imgui researcher board stays omniscient.
5. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`.

## In scope

### A. Overlay `[voting]` accept + council

Same overlay as M13/M15 (not `ExperimentConfig`, no postcard `config_hash` change):

```toml
[voting]
weight = "equal"          # unchanged; ignored when accept is unanimous/council
accept = "majority"       # default; omit = majority
# accept = "unanimous"
# accept = "council"
# council = [0, 1]
```

| `accept` | Who must Support to Accept | Reject when |
|---|---|---|
| `majority` (default) | M13/M15 weighted `yes ≥ need` | `no ≥ need` |
| `unanimous` | **Every living** agent is in `supporters` | **Any living** agent is in `opposers` |
| `council` | Every **living** id in `council` is in `supporters` | Any living council id is in `opposers` |

- Unknown `accept` → **load error**.
- `accept = "council"` with missing/empty `council` → **load error**.
- `council` entries are agent ids (`u64`). Non-integers → load error.
- Dead council members are dropped from the living set (not a load error).
- If the living council is **empty** (all dead): stay **Open** until lifetime expires (do not auto-Accept).
- Non-council Support/Oppose still update the sets (hash/display) but **do not** decide Accept/Reject.
- `weight` applies **only** to `majority`. Unanimous/council are stance-complete (heads), not prestige-weighted.
- Do **not** add `[voting]` to shipping `configs/default.toml`.

Kingmaker-style checks: 3 agents, only 0 Supports → unanimous **Open**; all 3 Support → **Accepted**; 1 Oppose → **Rejected**. Council `[0, 1]`, 0 and 1 Support, 2 Wait → **Accepted**; 1 Oppose → **Rejected**. Majority omit still matches M4/M15.

### B. `/set` (in-process)

Closed console, same family as `/give`:

```
/set ID hunger|thirst|energy|influence N
```

| Field | `N` | Stored |
|---|---|---|
| hunger, thirst, energy | display 0–100 integer | millipoints `N * 100`, clamp 0..=10_000 |
| influence | display 0–100 integer | millipoints `N * 100`, clamp 0..=10_000 |

- Unknown field / bad id / missing args → console error, no panic.
- **Hash-sensitive** (mutates agent state).
- **Remote attach refuses** (no new `ControlVerb`).
- No `/set respect` (edges need `toward`; use `esteem.toml`).
- Help lists `/set`.

### C. Range-limited board (+ last action when identified)

Shipping `configs/default.toml` keeps `public_board_always_visible = true` (hashes unchanged).

When **false**:

**Open proposals** in `Observation.board` (and LLM prompt) if **any** of:

- observer is the **author**, or
- observer is already in `supporters` or `opposers`, or
- author is within **identity** Chebyshev range.

Otherwise omit. Adopted rules remain mechanical law (imgui Board stays omniscient). Open-only fog is the experiment.

**Legal** `Support` / `Oppose` only for proposals in that filtered set (no ghost ids).

**Others’ actions:** `AgentView` gains optional last primary kind (`move`, `eat`, …) from that agent’s last `SimEvent` this tick or previous, only when the viewer **identified** them (`id: Some`). Silhouettes (vision but not identity) stay `{ id: None, x, y }` with **no** last action. Observation is not in `state_hash`; default board flag true ⇒ mock Wait hashes stay equal.

Researcher imgui **Board** window stays full. Agent-POV 3D fog unchanged (still `observation::build`).

## Out of scope (later)

| Later | What |
|---|---|
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done — [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done — [`M20-plan.md`](M20-plan.md) — revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done — [`M21-plan.md`](M21-plan.md) — pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| **M23** | [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done — [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | Done — [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| **M60** | [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| After M60 | protobuf/TLS; sql.js / ad-hoc SQL |
| Not M16 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump; `/set respect`; weighted council |

## Key decisions

1. `[voting] accept = majority|unanimous|council` on the existing overlay; `weight` only for majority.
2. Council = unanimous among listed **living** ids; empty living council stays Open until expiry.
3. `/set` is in-process, closed fields, display 0–100 → millipoints ×100.
4. Board fog uses **identity** range (plus author / own stance); default.toml stays `true`.
5. Identified agents may show last action; silhouettes do not.
6. Do not change shipping `configs/default.toml` voting overlay.
7. Mock CI. `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## Tests (M16 acceptance bar)

| Test | Asserts |
|---|---|
| omit / `accept = "majority"` | same as M15 majority |
| `accept = "maybe"` | load error |
| `accept = "council"` no list | load error |
| unanimous, 1 of 3 Support | **Open** |
| unanimous, 3 Support | **Accepted** |
| unanimous, 1 Oppose | **Rejected** |
| council `[0,1]`, both Support | **Accepted** (agent 2 silent) |
| `/set 0 hunger 50` | hunger 5000; hash changes |
| remote `/set` | refused |
| `public_board_always_visible = false` | far author omitted; nearby/self included; Support not legal for omitted |
| default.toml (board true, no `/set`) | hashes match pre-M16 mock |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: `accept` + council tally

- **Files:** `voting.rs`, `board`/`simulation` lifecycle, governance tests
- **Changes:** parse `accept` / `council`; unanimous and council stance-complete; majority unchanged.

### PR 2: `/set`

- **Files:** viewer `commands.rs` / help
- **Changes:** parse + apply; remote refuse; parse tests.

### PR 3: board fog + identified last action

- **Files:** `observation.rs` `board_view` + legal Support/Oppose; `AgentView` last kind; docs status when implemented

## Config / CLI

No new `ExperimentConfig` postcard fields. No new `ControlVerb`.

```bash
cargo test -p sim-core
cargo test -p viewer
```

## Verification (when implemented)

Walkthrough: [`M16-test-plan.md`](M16-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: majority omit unchanged; unanimous/council Accept/Reject as above; `/set` hunger 50 → 5000 milli; board false omits far proposals; default mock hashes match.

## Risks

- **Three items.** Keep each closed; do not add weighted council or `/set respect`.
- **Legal vs Observation.** Fog Support/Oppose together or agents vote on ghost ids.
- **Do not add `ControlVerb` or bump `PROTOCOL_VERSION`.**
