# M43 — WIS toxin detect, CHA speech, DEX flee

**Status:** implemented  
**Depends on:** M42 complete (`docs/M42-plan.md`, git tag `M42`, commit `6c6d04f`)  
**Walkthrough:** [`docs/M43-test-plan.md`](M43-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 WIS/CHA/DEX remaining uses)

## Context

M42 shipped CON energy max / illness duration and INT memory cap / retrieval_k. WIS already adds vision/hear/ident cells; `obs.toxins` is still only `ToxinFact` memory (eat or hear). CHA already changes influence **votes**; speech range ignores speaker CHA. DEX already reduces walk cost and melee accuracy; Flee is one orthogonal step at walk cost.

M43 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[agents.sheet]` / `--sheet` and see WIS **identify toxic vegetation in observation without eating it first**. Unused WIS ⇒ toxins only from `ToxinFact` memory (today).
2. See CHA change **how far speech travels** and **how much a heard utterance weighs**. Unused CHA ⇒ today’s speech cells and importance 50 / SPEAK affinity 50.
3. See DEX **flee farther** on the same energy as one Move. Unused DEX ⇒ one orthogonal step, same cost as today.
4. `--load` restores scores; do **not** persist derived detect/range/steps. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Idle mock 2-tick hash **stays `cd1e0853…`**.

## In scope

No PROTOCOL bump. No new postcard variants. No new overlay keys. Existing `[agents.sheet] enabled` / `--sheet`. Derived at use time (same as M34/M41/M42). Score 0 ⇒ today’s constants. WIS 10 / CHA 10 / DEX 10 (mod 0) ≡ unused for these formulas. No extra RNG on unused WIS.

### A. WIS toxin detect

Today `obs.toxins` is **only** `MemoryKind::ToxinFact` (eat toxic or hear “X is toxic”). WIS already adds perception range (`adjust_range`). Board range stays later.

| Use | Formula (WIS 0 ⇒ identity) |
|---|---|
| Detect | `WIS_mod > 0` ⇒ union visible `Toxicity::Toxic` vegetation species into `obs.toxins` (by species `id`). `WIS_mod <= 0` ⇒ memory only. |

- **Visible** = that species is on a tile in the agent’s current vision (`chebyshev <= vis`).
- Allergenic species stay personal (allergy + eat path). WIS detect does **not** list allergenic-only rows.
- Do **not** write `ToxinFact` from detect (no extra memory, no persist, no RNG). Eat / hear still write `ToxinFact` as today.
- `avoid_toxic` also drops Eat/Gather of WIS-detected visible toxic species (mock will not eat them while they are in vision). Governance (`toxin_propose` / `toxin_support`) still requires `knows_toxin` memory — detect ≠ board knowledge.
- No extra RNG. Unused / WIS 10 ⇒ same `obs.toxins` and same legal Eat/Gather as no `--sheet`.

### B. CHA speech range/weight

CHA already changes influence **votes** (`influence_vote_weight`). Speech range today is listener (or speaker-broadcast) WIS/`perceptiveness` on `base_speech_range` / hearing; shout × `shout_range_multiplier` (1.8). Speaker CHA is unused.

| Use | Formula (CHA 0 ⇒ identity) |
|---|---|
| Speech cells | add **speaker** `CHA_mod` to the existing hear/shout range (floor 0). Directed targeting still uses identity range. |
| Speech weight | heard-utterance importance = `max(1, 50 + CHA_mod * 5)`; SPEAK affinity applied as `max(0, 50 + CHA_mod * 10)` (shout tuples unchanged). |

- Apply speaker CHA in `heard_last_tick` and in `execute_speak` broadcast partner selection.
- Shout energy cost and shout multiplier **unchanged**. Speaker’s own “said:” memory stays importance 40.
- Do not change `SPEAK` / `SHOUT` consts globally; scale at `apply_delta` / remember-heard time so unused CHA is a no-op.
- CHA 10 (mod 0) ≡ unused.

### C. DEX flee bonus

DEX already reduces **walk** energy (`adjust_move_cost`) and melee accuracy. Flee today: nearest other agent, one 4-neighbor cell that maximizes Chebyshev, pay one `move_cost_milli`, emit `Flee`.

| Use | Formula (DEX 0 ⇒ identity) |
|---|---|
| Flee steps | `1 + max(0, DEX_mod) / 2` (integer). DEX 0/10 ⇒ 1; DEX 14 ⇒ 2; DEX 18 ⇒ 3. |
| Flee cost | **one** Move (already DEX-adjusted). Extra cells are free. |

- Greedy: repeat today’s 4-neighbor pick up to `flee_steps`. If the first cell is impossible or energy < one move ⇒ `Wait` as today. If a later cell is blocked, stop at the last good cell and still emit **one** `Flee`.
- Do **not** call `move_rel` per cell (that would charge N times and emit N `Move`s). Pay once, walk the cells, one `SimEventKind::Flee`.
- Legal Flee stays “any other agent in vision” + energy for one move.
- DEX 10 (mod 0) ≡ unused. Low DEX (mod < 0) does not drop below 1 step.

## Out of scope (later)

| Later | What |
|---|---|
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
| After M63 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; sql.js / ad-hoc SQL |
| Not M43 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; extra LLM; illness **chance** RNG; writing `ToxinFact` from WIS |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard variants. No new overlay keys.
2. No extra RNG on WIS detect. Overlay off / unused sheet ⇒ **same hashes**.
3. Detect is observation + `avoid_toxic` only. Do not write `ToxinFact`; governance still needs memory.
4. Speaker CHA only for speech range. Listener WIS/`perceptiveness` stay as today.
5. Flee pays **once** and emits **one** `Flee`. Extra cells are free. Do not persist derived steps.
6. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M43 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `cd1e0853…` |
| sheet unused | toxins = memory only; speech range/importance 50; flee 1 step + one move cost; same hash as no `--sheet` |
| WIS 18 vs 0, toxic veg in vision, empty memory | 18 lists species in `obs.toxins` and drops Eat/Gather; 0 does not; no `ToxinFact` written |
| WIS 10 | same as unused (mod 0) |
| CHA 18 vs 3 | 18 heard farther; heard importance 70 vs 35; SPEAK affinity 90 vs 20 |
| DEX 18 vs 0 | 18 flees 3 cells, pays one move; 0 flees 1 cell |
| `--load` | scores restored; no stored detect/range/steps |
| Hello v5 | unchanged |

## PR Plan

### PR 1: WIS toxin detect

- **Files:** helpers on `AbilitySheet`; `observation` union; `avoid_toxic` legal filter; unused identity tests; no `ToxinFact` write

### PR 2: CHA speech range/weight

- **Files:** speaker CHA added to hear/shout range; heard importance + SPEAK affinity scale; unused identity tests

### PR 3: DEX flee bonus

- **Files:** `flee_steps` helper; one-pay multi-cell flee; unused identity tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --quiet
```

## Verification

Walkthrough: [`docs/M43-test-plan.md`](M43-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
```

Expect: idle mock hash `cd1e0853…`; unused sheet identity; WIS/CHA/DEX change numbers when rolled; Hello v5.

## Risks

- **No extra RNG** on WIS detect. A chance roll would change unused hashes if the skip is wrong.
- Do **not** write `ToxinFact` from detect — that would persist after the plant leaves vision and change later governance/hashes more than observation.
- Flee must pay **once** and emit **one** `Flee`. Reusing `move_rel` per step would charge DEX move cost N times and leave `Move` events.
- Speaker CHA only (not listener) for speech range. Listener WIS/`perceptiveness` stay as today.
- `--load`: never store derived detect flags, speech cells, or flee steps; recompute from scores.
- Shipping `default.toml` / `coop.toml` unchanged. Overlay is not postcard. No PROTOCOL bump.
