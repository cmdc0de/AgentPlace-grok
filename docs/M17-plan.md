# M17 — Meta-rules, join/leave one-shots, event-log timeline

**Status:** implemented  
**Depends on:** M16 complete (`docs/M16-plan.md`, git tag `M16`, commit `e530b48`)  
**Walkthrough:** [`M17-test-plan.md`](M17-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` §2 (meta-rules), `incentive-schedule-format.md` (one-shots at start), `M14-plan.md` (`supporters_of`), `M6-plan.md` / `M14-plan.md` (viewer log / ckpt scrubber), `M9-plan.md` (Spark `--llm ollama`)

## Context

`allow_meta_rules` already exists on `ExperimentConfig` (default **false**) but Propose cannot carry governance rules. M14 one-shots fire only at incentive start. The viewer scrubs `.ckpt` files, not `{id}_events.jsonl`. Live Ollama on Spark has not been re-checked since the M13–M16 overlay work.

M17 does **not** rewrite postcard, add TLS, wire Give, or bump `PROTOCOL_VERSION`.

## Goal

A researcher can:

1. Set `allow_meta_rules = true` and Propose **closed** structured rules that, when Accepted, change lifetime, threshold, or `[voting] weight|accept` at runtime (not a TOML rewrite).
2. Use `supporters_of:` incentives whose **one-shots follow join/leave** (late joiners get them; leavers revert influence).
3. `--load DIR` with an events JSONL: imgui **event tick** slider shows that tick’s lines **without** replacing the sim (ckpt scrubber unchanged).
4. Run an **overnight live A/B on Spark** (`--llm ollama`, `http://spark-bcce.hlab:11434`, `nemotron3:33b`) so M13–M16 overlays did not silently break the LLM path. CI stays `provider = mock`.
5. `format_version = 2`, `PROTOCOL_VERSION = 2`. Default `allow_meta_rules = false` ⇒ default hashes unchanged.

## In scope

### A. Meta-rules (opt-in)

`[proposals] allow_meta_rules = true` in the **experiment** TOML (already a field; default false). Do **not** flip shipping `configs/default.toml`.

**New `StructuredRule` variants (append only — do not reorder BanEat / BanGather / MaxGather):**

| Rule | Meaning when **Accepted** |
|---|---|
| `SetProposalLifetime { ticks }` | Runtime lifetime for **open** proposals (0 = no expiry) |
| `SetAcceptanceThreshold { milli }` | Runtime majority threshold; `5000` = 0.50. Clamped `[100, 10000]` (0.01–1.0). **Replaces** the config default; incentive `proposal_threshold_modifier` still **adds** on top |
| `SetVoteWeight { equal \| influence \| respect }` | Mutates `Simulation.voting.weight` |
| `SetVoteAccept { majority \| unanimous \| council }` | Mutates `Simulation.voting.accept`. **No** meta-rule to rewrite `council = [...]` this slice |

If `allow_meta_rules = false`, Propose with these types is **illegal** (Wait). Last-wins if several of the same kind are adopted (later `tick_accepted` wins). New variants at the **end** of the enum keep old `.ckpt` loadable. LLM `parse_rule` accepts the new type names; unknown still Wait.

### B. Join/leave one-shots (`supporters_of`)

Per-tick effects already follow `in_scope`. One-shots (`goal_injection`, `relationship_delta`, `influence_factor_delta`) today run only in `start_incentive`.

Keep a `BTreeSet` of agent ids who have **already received** that incentive’s one-shots.

| Event | One-shots |
|---|---|
| Incentive **starts** | Current `in_scope` ids (unchanged) |
| Agent **enters** scope while active | Apply one-shots to that agent |
| Agent **leaves** scope | **Revert** `influence_factor_delta` only. Goals stay. Relationship deltas stay |
| Incentive **ends** | Existing end path (revert influence for remaining members) |

Pack “already applied” as `IncentiveState.entries[1]` JSON (`{id: [agent ids]}`). Old ckpts with only `entries[0]`: seed the set from **current** `in_scope` on load. `format_version` stays **2**.

### C. Event-log timeline (display-only)

When `--load DIR` (or a file whose parent has JSONL), discover `{id}_events.jsonl` beside ckpts. Imgui event-tick slider over ticks **present in the JSONL**; selecting a tick **filters the log panel**. Does **not** load a checkpoint. No interpolation. Remote attach: panel hidden. Parse existing `event_to_jsonl`; no new format.

### D. Overnight Spark live A/B (verification, not a feature)

Recipe in `M17-test-plan.md` when implemented. CI never hits the network.

| | Value |
|---|---|
| Host | `http://spark-bcce.hlab:11434` (`spark-bcce.halb` does not resolve) |
| Model | `nemotron3:33b`, `timeout_ms = 120000` |
| Flag | `--llm ollama` (empty `base_url` still mock) |
| Overnight size | **4 agents × 40 ticks** two arms. Optional 2×3 smoke first |
| Arms | **A** overlay omit; **B** `--incentives configs/incentives/coop.toml` |
| Proof | both `final_tick=40`; decisions JSONL has `prompt_hash`; `--compare` hashes **differ**; not empty-URL mock; `LlmWait` allowed |

Do **not** add extra LLM call types. Mid-run HTTP failure stays `Wait`.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M53** | [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| After M53 | protobuf/TLS; sql.js / ad-hoc SQL; weapon range; spear melee |
| Not M17 | Browser; combat; CI Win/mac; extra LLM reflection; `PROTOCOL_VERSION` bump |

## Key decisions

1. Meta-rules are **opt-in** `allow_meta_rules`; closed `StructuredRule` variants appended only.
2. Adopted meta threshold **replaces** config default; incentive modifier still adds.
3. Join/leave one-shots: apply on enter; revert **influence only** on leave.
4. Event timeline is **display-only** JSONL; not a second `--load`.
5. Spark overnight is **verification**, not a new provider. CI mock.
6. Do not change shipping `configs/default.toml`.
7. Mock CI. `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## Tests (M17 acceptance bar)

| Test | Asserts |
|---|---|
| `allow_meta_rules = false` | meta Propose → Wait |
| meta threshold 0.10, 1 of 3 Support | **Accepted** |
| meta unanimous, 1 of 3 | **Open** |
| late Support + `supporters_of` goal/influence | joiner gets one-shots |
| Oppose after join | influence reverted; goal remains |
| JSONL tick list | 10/40/80 → want 50 filters 40 |
| default.toml mock | hashes match pre-M17 |
| Spark `curl /api/tags` | nemotron reachable (overnight only) |
| live 4×40 A vs coop B | hashes differ; decisions JSONL present; not mock |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Meta-rules

- **Files:** `board.rs` `StructuredRule`, `execute.rs` Propose gate, `llm.rs` parse_rule, governance tests

### PR 2: Join/leave one-shots

- **Files:** `incentive.rs`, checkpoint `IncentiveState.entries[1]`, incentives tests

### PR 3: Event JSONL timeline + Spark recipe

- **Files:** viewer log/commands, helper to list JSONL ticks, this plan + `M17-test-plan.md` overnight section when implemented

## Config / CLI

No new `ControlVerb`. `allow_meta_rules` already on `ExperimentConfig` (hashed when true).

```bash
cargo test -p sim-core
cargo test -p viewer
# overnight: --llm ollama 4×40 A/B on spark-bcce.hlab
```

## Verification (when implemented)

Walkthrough: [`M17-test-plan.md`](M17-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: meta Propose waits unless flag on; threshold 0.10 kingmakes equal 1-of-3; late `supporters_of` joiners get one-shots; JSONL slider filters log; default mock hashes match. Overnight Spark A/B is the live LLM proof.

## Risks

- **Postcard enum append.** New `StructuredRule` variants must stay at the end.
- **Double-apply on `--load`.** Seed applied-set from current scope when `entries[1]` is missing.
- **Overnight cost.** 4×40 not 16×80; 2×3 smoke first.
- **Do not add `ControlVerb` or extra LLM call types.**
