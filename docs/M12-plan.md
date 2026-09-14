# M12 — Public vs hidden incentives

**Status:** implemented  
**Depends on:** M11 complete (`docs/M11-plan.md`, git tag `M11`, commit `128b689`)  
**Walkthrough:** [`M12-test-plan.md`](M12-test-plan.md)  
**Specs:** `incentive-schedule-format.md`, `memory-goals-incentives-spec.md` §3 (`visibility_modifier`), `decision-observation-llm-economy-metrics-spec.md` §2 (what Observation may contain)

## Context

M9 put every in-scope incentive into `Observation.incentives` and the LLM “Active incentives” line. M11 made the coop **goal** actually fill crates. There is still no way to run “the bonus is on, but the agent is not told.”

The long-term spec lists `visibility_modifier` as an **effect type** that gates who can see whose actions or proposals. That broader fog-of-governance wait. This slice is the experimental control that M8/M9 already named: **public vs hidden incentive banners**.

M12 does **not** rewrite postcard, add TLS, timeline, vote weighting, `/set`, or wire Give.

## Goal

A researcher can:

1. Set `visibility = "hidden"` on an `[[incentives]]` table and have **effects still apply**, while `Observation.incentives` and the LLM prompt **omit** that id.
2. Keep injected **goals** in Observation/prompt (hidden coop still Stores under the M11 mock).
3. Still **see** hidden incentives as the researcher (inspector, `IncentiveApplied`, `--compare` active set).
4. Same-seed mock, same effects, public vs hidden → **same `state_hash`**. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`.

## In scope

### A. `visibility` field

On each `[[incentives]]` table (overlay, not `ExperimentConfig`):

```toml
visibility = "public"   # default; omit = public (M11)
# visibility = "hidden"
```

Unknown values are a **load error**. Do **not** add `type = "visibility_modifier"` (that stays a load error). Visibility is **who is told**, not a mechanical effect.

Do not change `configs/incentives/coop.toml` (stays public so M11 crate-fill holds).

### B. Agent knowledge vs mechanics

| Surface | Hidden incentive |
|---|---|
| Effects (resource, influence, memory, threshold, relationship, **goal_injection**) | Still apply |
| `Observation.incentives` / LLM “Active incentives” | **Omitted** even if `applies_to` this agent |
| Injected `Goal` in Observation / prompt | **Still listed** |
| Mock storage policy | Unchanged (reads `obs.goals`) |
| `IncentiveApplied`, checkpoints, `--compare` active incentives, viewer inspector | Researcher **still sees** (tag hidden) |
| Mock `state_hash` (same effects, public vs hidden) | **Equal** |
| LLM `prompt_hash` | **Differs** |

Covert payoff A/B uses `resource_multiplier` **without** `goal_injection` (see example file). Goal text is an internal drive, not the incentive banner.

### C. Example overlay

New `configs/incentives/hidden-bonus.toml`:

```toml
[[incentives]]
id = "hidden_food_bonus"
description = "1.4× food; agent is not told"
visibility = "hidden"
applies_to = "all"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
```

A public twin (same effects, `visibility = "public"`) is enough to A/B knowledge, not payoffs.

### D. Viewer

Inspector lists **all** active incentives and marks hidden ones. Agent-POV / prompt dump omits hidden banners. Fog-of-war is unchanged.

## Out of scope (later)

| Later | What |
|---|---|
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
| **M14** | Done — [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
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
| **M55** | [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| After M55 | Protobuf/TLS; sql.js / ad-hoc SQL |
| Not M12 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. **Field** `visibility = "public"|"hidden"` on `[[incentives]]`, default public.
2. **Hidden omits the banner** from Observation/prompt; **effects still apply**.
3. **Injected goals stay** in Observation.
4. **Researcher surfaces still show** hidden incentives.
5. Mock public vs hidden, same effects → **same hash**.
6. **Do not change `coop.toml`.**
7. **Vote weighting / action-fog / protobuf / TLS / timeline / `/set` / wire Give → later.**
8. Mock CI. No new `ControlVerb`.

## Tests (M12 acceptance bar)

| Test | Asserts |
|---|---|
| Omit / `public` | same Observation.incentives as M11 |
| `hidden` + in-scope | effects on; that id **not** in Observation.incentives |
| Hidden `goal_injection` | goal still in Observation.goals; mock still Stores |
| Same seed, public vs hidden, mock, same effects | **same `state_hash`** |
| Prompt | hidden id absent from `build_prompt`; public id present |
| Load error | `visibility = "maybe"` fails parse |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Parse + Observation filter

- **Files:** `incentive.rs`, `observation.rs`, tests
- **Changes:** `visibility` field; skip hidden in `Observation.incentives`; hash-equal public vs hidden mock.

### PR 2: Prompt, inspector, example overlay

- **Files:** `sim-llm` prompt (already driven by Observation), viewer `ui.rs`, `configs/incentives/hidden-bonus.toml`
- **Changes:** hidden tagged in inspector; example schedule.

### PR 3: README + docs

- **Files:** README, this plan status when implemented, `incentive-schedule-format.md` (replace “load error if present”)

## Config / CLI

No new `ExperimentConfig` postcard fields.

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
# when implemented:
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/hidden-bonus.toml --ticks 40 --llm mock --quiet
```

## Verification (when implemented)

Walkthrough: [`M12-test-plan.md`](M12-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p sim-llm
```

Expect: hidden id absent from Observation/prompt; same-seed mock hashes match public twin; `coop.toml` 80-tick crate-fill unchanged.

## Risks

- **Leaking via goals.** Hidden `goal_injection` still shows the goal text. Document it; covert A/B should use mechanical effects only.
- **Mock hash equality.** If anything hashes Observation.incentives into `state_hash`, public vs hidden will diverge — do not add that.
- **Do not add `ControlVerb` or vote weights** in this slice.
