# M64 — Non-square footprints, ranged projectile FX, agent idle clip

**Status:** planned (not yet implemented)  
**Depends on:** M63 complete (`docs/M63-plan.md`, git tag `M63`, commit `83727f3`)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-5 / RF-6 / RF-9

## Context

Sleep places are **N×N** only (`sleep_size`). Dawn and Pickup already cover every cell of that square; Move is not blocked. Stations are 1×1. Combat FX draws the same strike cuboid for melee and ranged Attack. The shipped agent glb (`low_poly_human_character.glb`) has one skin and one clip (`ArmatureAction.002`); the viewer loads `Scene(0)` and never plays it.

M64 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Square shipped tent/cabin/house stay square; extra footprint dims hash **only when non-square** ⇒ shipped-objects idle hashes **stay M63**. Projectile FX and idle clip are viewer-only ⇒ **hash-neutral**. `--no-time` no-objects stays **`70e5204d…`**. Not interiors-as-walls, not ammo consume, not walk/attack clips.

## Goal

A researcher can:

1. **Place** a catalog sleep item whose footprint is a **rectangle** (e.g. 2×3), not only N×N. Dawn bonus and Pickup still apply on **any cell** of that rectangle. Tent 1×1 / cabin 2×2 / house 4×4 stay square.
2. **See** a ranged Attack (Chebyshev **> 1**) as a **projectile** mesh from attacker to defender. Adjacent Attack keeps today’s strike cuboid. No ammo. Hash-neutral.
3. **See** the shipped agent glb play `ArmatureAction.002` as **idle** when the agent did not Move this tick. No walk clip this slice: moving agents **pause** idle (static pose). Hash-neutral. Native window only.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Place / Attack unless tests `execute_primary`. No new CLI flag.

### A. Non-square footprints (RF-5)

Optional `[sim]` fields. Stations stay **1×1**. Move is still **not** blocked by sleep cells.

```toml
sleep_size = 2          # square default (omit = 1)
sleep_w = 2             # omit = sleep_size
sleep_h = 3             # omit = sleep_size
```

- Both omit ⇒ square `sleep_size` (today).
- Either set ⇒ rectangle `w×h` from origin +x +y. Each dim clamped 1..=8.
- `hash_catalog`: keep hashing `sleep_size`. **Also** hash `sleep_w` / `sleep_h` **only when** `(w,h) != (size,size)`. Square shipped objects ⇒ **same** catalog hash as M63.
- Do **not** change `tent.toml` / `cabin.toml` / `house.toml`.
- Tests use a **fixture** object dir (not a new shipped `configs/objects/` file).
- Place / Pickup / dawn / overlap / OOB / water: same rules on the W×H cells. Viewer sleep mesh scales to W×H (hash-neutral).
- `--load` restores origin; size from the current catalog. Do not re-Place.

Not this slice: walls, door cells, blocking Move, indoor graph, non-square stations.

### B. Ranged projectile FX (RF-6)

Today every `Attack` is `CombatFxJob::Strike` (cuboid). Spear/bow/sling already have `attack_range` 2–3. No ammo item.

- `combat_fx_jobs` emits **Projectile { from, to }** when this tick’s Attack has Chebyshev **> 1** (look up agent positions; do **not** add range to `SimEventKind::Attack`).
- Dist == 1 stays **Strike**.
- Viewer: projectile = small sphere (or thin cuboid) along attacker→defender; melee cuboid unchanged. Tints / HUD unchanged.
- **Not** hashed. **Not** checkpointed. No ammo qty. No new event tag.

### C. Agent idle clip (RF-9)

Shipped agent glb has one clip. Viewer never plays it.

This slice maps that clip to **idle**. Later slices add other action→clip pairs (walk, attack, downed) without changing this mapping.

- Native viewer: load glTF animations. Idle = clip 0 / `ArmatureAction.002` (optional `[visual.animations] idle = "ArmatureAction.002"` in `agent.toml`; omit ⇒ first clip). `[visual]` is not hashed.
- **Idle** = agent did **not** emit `Move` this tick (Rest, Wait, Attack-in-place count as idle). Loop the idle clip.
- **Move this tick** and no walk clip yet ⇒ **pause** idle (static pose). Do not invent a walk cycle.
- No clip / primitive / sentinel ⇒ static as today (not an error).
- Agent stem only (not trees, houses, tools).
- No new authored clips. Not browser 3D.

## Out of scope

| Not M64 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Interiors | walls / blocked Move / doors (**RF-18**) |
| Ammo | consume qty on ranged Attack (**RF-19**) |
| Extra clips | walk / attack / downed action→clip pairs (**RF-20**) |
| Also not | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; extra recipes; RF-1 household meshes; per-tick decay |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard `PrimaryAction` / `SimEventKind` append.
2. **format_version still writes 3 / reads v2+v3.**
3. Extra footprint dims **omit-hash when square** so shipped-objects idle hashes stay M63.
4. Tests use a fixture catalog for 2×3; do not add a shipped object file.
5. Projectile jobs use live positions, not a new Attack field.
6. The one existing clip is **idle**, not “always playing.” Move pauses it.
7. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M64 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks (shipped objects, time on) | hash **M63** `f583c391…` |
| `--no-time` shipped objects 2 ticks | hash **M63** `1b9117a9…` |
| Place 2×3 fixture | occupies 6 land cells; origin stored min-(x,y) |
| cabin 2×2 | identity (still square) |
| 2×3 OOB / water / overlap | Place illegal |
| dawn any cell of 2×3 | sleep_bonus applies |
| Pickup any cell of 2×3 | item returns; cells free |
| `--load` 2×3 | origin restored; no second Place |
| `combat_fx_jobs` dist 1 | `Strike` |
| `combat_fx_jobs` dist 2 | `Projectile`, not Strike |
| idle mock | no Attack ⇒ no projectile jobs; hash unchanged |
| agent glb | shipped file has ≥1 animation clip; idle maps to that clip |
| idle vs Move | no Move this tick ⇒ play idle; Move this tick ⇒ pause |
| no-clip / sentinel | still static spawn (no panic) |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Non-square footprints

- `sleep_w` / `sleep_h`; `footprint_free` / `sleep_covers` W×H; fixture tests; viewer scale W×H

### PR 2: Projectile FX

- `CombatFxJob::Projectile`; viewer mesh; unit tests dist 1 vs 2

### PR 3: Agent idle clip

- Load glTF animations; idle = first clip / `ArmatureAction.002`; loop when no Move this tick; pause on Move; no-clip identity

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag.

On implement: catalog `sleep_w` / `sleep_h`; `CombatFxJob::Projectile`; agent idle clip. Optional `[visual.animations] idle` on `agent.toml` (hash-neutral). Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test sleep
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough on implement: `docs/M64-test-plan.md`.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; shipped-objects hashes **M63** (`f583c391…` / `1b9117a9…`); Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** Extra dims must omit-hash when square or shipped-objects idle hashes move.
- **Cover loops.** Every `sleep_covers` must use W×H, not leftover `sleep_size` only.
- **Stations.** Stay 1×1; do not read `sleep_w` on millstone/spit.
- **Projectile jobs.** Need positions; do not add range onto `SimEventKind::Attack`.
- **Bevy 0.19.** `AnimationPlayer` + graph vs Scene-only spawn; Draco path still works.
- **Idle vs Move.** Without a walk clip, moving agents pause (static). Do not loop idle while walking. Action→clip table must be easy to extend later.
- **No protocol / format bump.**
