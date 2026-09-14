# M28 — Health/incapacitation, combat viewer FX, force_reflect

**Status:** implemented  
**Depends on:** M27 complete (`docs/M27-plan.md`, git tag `M27`, commit `9fb175e`)  
**Walkthrough:** [`M28-test-plan.md`](M28-test-plan.md)  
**Specs:** `medium-priority-specs.md` §2 (`health` / incapacitation; `injury_reduces_energy`), `decision-observation-llm-economy-metrics-spec.md` §1 (incentive-forced Reflect), `incentive-schedule-format.md` (closed effect types), `M27-plan.md` (energy-only Attack)

## Context

M27 Attack subtracts **energy** only; there is no `health` field and no incapacitation. The viewer does not tint Attack/Flee. Spec Reflect can be forced by an incentive; that effect type was never added.

M28 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb` / `ClientMessage`. It does not add embeddings, TLS, combat death, or change `format_version`.

## Goal

A researcher can:

1. See Attack **hurt**: overlay `[conflict]` still gates combat; defenders have `health` (millipoints, default 10_000). At 0 they become **incapacitated** (Wait only, not an Attack target). Mock + overlay off / no Attack ⇒ **same hash** (health at default is not hashed).
2. **See** combat in the viewer: attacker/defender tints (and a HUD/inspector line) from this tick’s `Attack` / `Flee` events. Render-only; hash-neutral.
3. Force a Reflect from a schedule: `[[incentives.effects]] type = "force_reflect"` while the incentive is active. In-scope live/`Custom` agents get **one** insight this tick (same `reflect` replay slot as every-N). Mock skip.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. Health / incapacitation

- `Agent.health: u32` default **10_000**, `#[serde(default, skip)]`, packed in checkpoint `BoardBlob` (`health: BTreeMap<u64, u32>`, `#[serde(default)]`).
- `Agent.incapacitated: bool` default false, same skip + BoardBlob.
- **Hash:** write health bytes only if `health != 10_000`; write incapacitated only if `true`. Default agents add **no** hash bytes.
- Attack: attacker still pays energy cost. Defender **health** saturating-sub `ATTACK_DAMAGE`. Keep M27 **energy** damage (`injury_reduces_energy`).
- Health hits 0 → `incapacitated = true`, append `SimEventKind::Incapacitated { by }` hash tag **27**. No new death path (`Died` stays hunger/thirst).
- Incapacitated: `step_agent` forces Wait (no LLM); not a legal Attack target; Flee not legal for them.
- Overlay still `[conflict] enabled` / `--conflict`. No `[conflict] death_enabled` this slice.

### B. Combat viewer FX

- Hash-neutral. When applying a Snapshot / in-process tick, if this tick’s events include `Attack` / `Flee` for an agent: tint attacker reddish, defender darker; optional short HUD line `tick T attack #A → #B`.
- Inspector already lists events; keep that. No new PROTOCOL. No new meshes (material color on existing capsules).

### C. Incentive-forced reflect

Append `EffectSpec::ForceReflect` (serde `type = "force_reflect"`). Unknown types still load-error.

```toml
[[incentives]]
id = "reflect_now"
start_tick = 1
[[incentives.effects]]
type = "force_reflect"
```

- While active, in-scope living `Custom` agents: fire **at most one** insight this tick if every-N **or** `force_reflect`. Same replay call `"reflect"` / seed family as M26 insight (no second slot).
- Mock/Wait skip the LLM. Do **not** add a hashed pipeline event. `IncentiveApplied` on start is unchanged (already hashed for treated runs).
- Do **not** change shipping `coop.toml`. Example overlay created on implement: `configs/incentives/force-reflect.toml`.
- `docs/incentive-schedule-format.md` updated on implement (new effect row).

## Out of scope (later)

| Later | What |
|---|---|
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
| **M58** | [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| After M58 | protobuf/TLS; Unix sockets; sql.js / ad-hoc SQL |
| Not M28 | Browser; CI Win/mac; PROTOCOL bump |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard wire variants.
2. Health default is hash-neutral. Incapacitation is event tag 27, append-only.
3. One insight per agent-tick (`reflect` replay key). Force OR every-N, not two calls.
4. Viewer FX is render-only. `force_reflect` is schedule overlay, not `ExperimentConfig`.
5. Do not change shipping `configs/default.toml` / `configs/incentives/coop.toml`. Mock CI. `format_version = 2`.

## Tests (M28 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| conflict overlay + mock, no Attack | same hash as overlay off (health default) |
| Attack overlay on, adjacent | defender health down; energy still drops |
| health → 0 | `incapacitated`; `Incapacitated` event; next tick Wait; not Attack target |
| old ckpt load | health 10_000, not incapacitated |
| viewer color helper | Attack attacker/defender map to distinct tints; no sim hash change |
| `force_reflect` + mock | skip LLM; no extra hashed pipeline event |
| `force_reflect` + Custom | Reflection memory this tick; record + replay match |
| unknown effect type | still load error |
| Hello v5 | unchanged |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Health + incapacitate

- **Files:** `agent.rs` fields; BoardBlob; `hash_bytes`; Attack applies health; `SimEventKind::Incapacitated` tag 27; legal/Wait; combat tests

### PR 2: Viewer FX

- **Files:** viewer agent material tint + HUD from this-tick Attack/Flee; unit helper test (hash-neutral)

### PR 3: force_reflect

- **Files:** `EffectSpec` append; insight trigger; tests; `incentive-schedule-format.md`; `configs/incentives/force-reflect.toml`

## Config / CLI

No shipping TOML change. `[conflict] enabled` already overlay. New effect `force_reflect` lives in incentive schedule files (overlay, not `ExperimentConfig`). Example `configs/incentives/force-reflect.toml` on implement only. Action/event enums **append only**. No PROTOCOL bump.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --incentives configs/incentives/force-reflect.toml \
  --llm ollama --llm-reflect-every 10 --conflict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

## Verification

Walkthrough: [`M28-test-plan.md`](M28-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged; Attack drops health then incapacitates; viewer tints from events; `force_reflect` + Custom writes a Reflection; Hello stays v5.

## Risks

- **Hashing default health** — only hash when ≠ 10_000.
- **`SimEventKind` append only** — tag 27. Reorder would break hashes.
- **`EffectSpec` serde variant** — append `ForceReflect`; schedule is overlay TOML, not ExperimentConfig.
- **Two insights one tick** — one `reflect` replay key; OR every-N, not both.
- **Incapacitated hashed via events** — only when combat actually drops health to 0.
- **Double-apply `--load`** — health from BoardBlob, not re-derived.
- **No PROTOCOL bump.** Stay at 5.
