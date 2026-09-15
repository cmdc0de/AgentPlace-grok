# M27 — Auto-execute plan, combat

**Status:** implemented  
**Depends on:** M26 complete (`docs/M26-plan.md`, git tag `M26`, commit `4ce0b02`)  
**Walkthrough:** [`M27-test-plan.md`](M27-test-plan.md)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md` §1 (Attack/Flee; core never executes raw NL), `medium-priority-specs.md` §2 (physical conflict v2-lite), `M26-plan.md` (plan stored, not executed)

## Context

M26 stores `Agent.plan` as strings and shows it to `choose`. It is **not** executed. Spec Attack/Flee exist only in the vocabulary doc; `PrimaryAction` has neither, and `physical_conflict_enabled` never shipped.

M27 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb` / `ClientMessage`. It does not add embeddings, TLS, `health`, or change `format_version`.

## Goal

A researcher can:

1. Turn on overlay `[llm] execute_plan = true` (or `--llm-execute-plan`) so a non-empty `Agent.plan` **is followed**: `plan[0]` is parsed as an action; if it is legal this tick it is executed, the step is popped, and live `choose` is skipped. Unparseable / illegal → leave the plan and fall through to today’s choose. Mock/Wait skip (plan empty) ⇒ **same hash**.
2. Turn on overlay `[conflict] enabled = true` (or `--conflict`) so **Attack** / **Flee** are legal and execute. Overlay **off** (default): not in the legal list; mock never picks them ⇒ **same hash**.
3. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. Auto-execute plan

Overlay **not** on `ExperimentConfig` (would change `config_hash`). Extend `LlmBarrierParams`:

```toml
[llm]
execute_plan = true
```

CLI: `--llm-execute-plan`. Omit / false = M26 (plan is shown to choose only).

**Who / when** in `step_agent`: after Plan, **before** select.

- If overlay **on** and `agent.plan` non-empty: `parse_choice_json(plan[0], legal)`. Ok + legal → that `ChosenAction`, **pop front**, skip live/`Custom` choose. Fail → plan unchanged, normal choose.
- Each plan step is an **action JSON object** (stringified), e.g. `{"action":"Drink"}`. M26 free-text steps fail parse → fall through (the core never executes raw natural language).
- Live `OpenAiCompatClient::plan` prompt asks for `{"plan":[{"action":"..."}, ...]}` (objects, not prose). `parse_plan_json` accepts object elements (stringify) as well as strings.
- **Replay:** execute_plan still pops on replay (plan is hashed). If it succeeds, **do not** also apply the choose JSONL line. Record still writes a choose line of the executed action for humans.
- Mock/Wait / overlay off: current path.

### B. Combat (v2-lite)

Append-only (postcard / hash tags):

- `PrimaryAction::Attack { target: AgentId }` and `Flee` at the **end** of the enum.
- `SimEventKind::Attack { target, damage }` tag **25**, `Flee` tag **26**.

Overlay **not** on `ExperimentConfig`:

```toml
[conflict]
enabled = true
```

CLI: `--conflict`.

- **Off (default):** Attack/Flee omitted from `legal_actions`. Mock heuristics never pick them (they Wait/Move, not “any legal”). No events. **Same hash as today.**
- **On:** Attack legal if another living agent is Chebyshev **1**. Flee legal if any other living agent is in vision.
- Attack: attacker pays a small energy cost; defender **energy** drops by a fixed millipoint damage (spec `injury_reduces_energy`). Existing `Died` if energy hits 0 and `death_enabled`. **No new `health` field** this slice.
- Flee: one `MoveRelative` step that increases distance to the nearest other agent; if none, Wait.
- No viewer combat FX. Inspector shows Attack/Flee when they are legal.
- `parse_choice_json`: `"attack"` + target id, `"flee"`.

Do **not** implement embeddings, Unix sockets, protobuf/TLS, HP/incapacitation, or auto-execute of prose plan steps.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M60** | Done — [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| **M61** | Done — [`M61-plan.md`](M61-plan.md) — time-decay wear, out-dir disk walk, extra recipes |
| **M62** | Done — [`M62-plan.md`](M62-plan.md) — stations for bread/stew, Store wear, extra recipes |
| **M63** | [`M63-plan.md`](M63-plan.md) — crate dawn decay, remaining food stations, extra recipes |
| After M63 | protobuf/TLS; Unix sockets; sql.js / ad-hoc SQL |
| Not M27 | Browser; CI Win/mac; PROTOCOL bump |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard wire variants.
2. Execute-plan and conflict are overlay + CLI, not `ExperimentConfig`. Mock skip ⇒ default hashes unchanged.
3. Plan steps that are not action JSON are **not** executed.
4. Combat damages **energy**, not a new hashed `health` field. Event tags append 25/26.
5. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M27 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| `execute_plan` overlay + mock | same hash as off |
| `conflict` overlay + mock | same hash as off |
| Custom + execute_plan, plan `[{"action":"Wait"}]` | that tick is Wait; plan shorter |
| unparseable `plan[0]` | choose runs; plan unchanged |
| record + replay execute_plan | recording `state_hash` = replay |
| Custom Attack, overlay on, adjacent | defender energy down; `Attack` event |
| overlay off | Attack not in `legal` |
| Flee, overlay on, neighbor | moves away or Wait if blocked |
| Hello v5 | unchanged |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Execute-plan overlay

- **Files:** overlay/CLI `execute_plan`; `step_agent` parse `plan[0]`, pop, skip choose; record/replay; `parse_plan_json` objects; stub-chooser tests
- **Changes:** M26 prose steps still fall through

### PR 2: Attack / Flee

- **Files:** append `PrimaryAction` + `SimEventKind` (tags 25/26); overlay `[conflict] enabled` / `--conflict`; `legal_actions` + execute energy damage; unit tests
- **Dependencies:** none (can land beside PR 1)

### PR 3: Parse + live prompts

- **Files:** `parse_choice_json` attack/flee; `OpenAiCompatClient` choose/plan prompts list Attack/Flee when legal and emit plan as action objects

## Config / CLI

No shipping TOML change. Overlay `[llm] execute_plan` and `[conflict] enabled` are not `ExperimentConfig`. No new postcard *config* fields. Action/event enums **append only**. No PROTOCOL bump.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm ollama --llm-reflect-every 10 --llm-plan-every 10 \
  --llm-execute-plan --conflict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

## Verification

Walkthrough: [`M27-test-plan.md`](M27-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock + execute_plan / conflict overlays same hash; Custom execute_plan pops a legal Wait; Attack drops defender energy when overlay on; Hello stays v5; default mock hashes match.

## Risks

- **`PrimaryAction` / `SimEventKind` must append only.** New hash tags 25/26.
- **Hashing a default `health` field** — do not add health this slice; damage energy only.
- **Mock sampling Attack** — mock does not pick from full `legal`; still omit Attack/Flee when overlay off.
- **Replay double-apply** — if execute_plan succeeds, skip choose JSONL on replay.
- **M26 prose plan steps** — fail parse, fall through; do not execute natural language.
- **`execute_plan` / `conflict` on `ExperimentConfig`** — overlay + CLI only.
- **No PROTOCOL bump.** Stay at 5.
