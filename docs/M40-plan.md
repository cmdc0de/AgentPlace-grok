# M40 — Config-owned objects + recipes (except agent)

**Status:** implemented  
**Depends on:** M39 complete (`docs/M39-plan.md`, git tag `M39`, commit `93d385a`)  
**Walkthrough:** [`docs/M40-test-plan.md`](M40-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-8, PG-6 LOD)

## Context

M39 added `configs/objects/*.toml` and `ItemId::Catalog` / `Recipe::Catalog`, but **catalog only adds**. Basket / Spear / FishingRod / Backpack recipes are still a Rust `match`. Viewer still has an M38 **stem** list (`berry_bush.glb`, …). Local files in `assets/models/optimized/` do not bind unless a definition points at them.

M40 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Visuals stay hash-neutral. Shipping `default.toml` / `coop.toml` unchanged. Agent meshes stay code (later).

## Goal

A researcher can:

1. Point every **non-agent** object at `assets/models/optimized/*.glb` (and LOD) from `configs/objects/*.toml`. Viewer has **no** stem fallback except **`agent`**.
2. Change Basket / Spear / FishingRod / Backpack (and cord) by editing TOML, not Rust. Missing recipe file ⇒ that Craft is illegal.
3. Add a new item the M39 way (`kind = "item"` + `[sim.craft]` → `ItemId::Catalog(u16)`).
4. Change only `[visual]` / LOD and keep the hash. Change `[sim]` (or hold a catalog item) and the hash **does** change.
5. Load old checkpoints: `ItemId::Basket` etc **stay** on the postcard enum (append-only; do not delete variants). Runtime tables come from config. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Idle mock hash **will change** (item `[sim]` always hashed when the objects dir has item files). Lock the new 2-tick value on implement.

## In scope

Always load `configs/objects` (or `--objects DIR`) for visuals **and** item `[sim]`. Hash non-empty item `[sim]` always. `--catalog` / `[catalog] enabled` may remain as an alias (no-op when files are present). `--catalog` still does **not** imply `--sheet`.

Built-in slugs map to existing variants: `basket` → `ItemId::Basket` / `Recipe::Basket` (same for `spear`, `fishing_rod`, `backpack`). New slugs stay `Catalog(u16)`. `recipe_spec` **drops** hardcoded fiber/wood/stone counts; it reads the TOML.

Do not invent extra overlays. Create the named object files **on implement** only.

### A. Visuals for every stem except agent

One TOML per id. Viewer: matching `id` ⇒ `visual.glb` or LOD (near < 8 cells, mid < 24, else far). Else **primitive** (no M38 stem except `agent`). Missing glb never fails CI.

| `id` | kind | near / mid (under `assets/models/optimized/`) |
|---|---|---|
| berry_bush | vegetation | `big_low_poly_berry_bush.glb` / `plants_ready.glb` |
| herb | vegetation | `plants_ready.glb` |
| mushroom | vegetation | `bunch_of_mushrooms.glb` / `lowpoly_mushrooms.glb` |
| nightshade | vegetation | `deadly_nightshade.glb` / `nightshade.glb` |
| tree | vegetation | `maple_tree.glb` / `tree.glb` |
| hare | animal | `rabbit.glb` |
| perch | fish | `lowpoly_fish.glb` / `fish_animated.glb` |
| crop | crop | `plants_ready.glb` |
| wood | item | `wood.glb` / `wood_pile.glb` |
| fiber | item | `low_poly_cloth.glb` |
| stone | item | `stones_and_grass.glb` |
| basket | item | `basket.glb` / `wicker_basket_01.glb` |
| spear | item | `spear.glb` / `herrscher_of_truths_spear.glb` |
| fishing_rod | item | `fishing_rod.glb` / `stylized_old_fishing_rod_low_poly.glb` |
| backpack | item | `backpack.glb` / `hiking_backpack.glb` |
| crate | crate | `simple_crate.glb` / `simple_short_crate.glb` |
| cord | item | optional cloth |

Agent (`human_3.glb`, `low_poly_human_character.glb`) **not** this slice.

### B. Recipes from config

Shipped craft TOML (same inputs as today’s code):

- `basket`: fiber × 2 → `ItemId::Basket`
- `spear`: wood × 1 + stone × 1 → `ItemId::Spear`
- `fishing_rod`: wood × 1 + fiber × 1 → `ItemId::FishingRod`
- `backpack`: fiber × 4 → `ItemId::Backpack`
- `cord`: keep M39 fiber × 2 → `Catalog(n)`

No object files ⇒ Basket/Spear/… **not** legal. Override inputs in TOML ⇒ Craft uses the new inputs; hash ≠ shipped. Mock still does not pick new Catalog Crafts unless tests `execute_primary`. `--load`: restore Basket/Catalog; do not re-grant.

World gather / species **spawn** tables stay in Rust. Vegetation/animal **visuals** are this slice; turning those kinds into hashed world species is later.

## Out of scope (later)

| Later | What |
|---|---|
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
| After M63 | protobuf/TLS; skeletal animation; sql.js / ad-hoc SQL |
| Not M40 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Do not remove `ItemId` / `Recipe` variants.
2. Visuals never hashed. Item `[sim]` always hashed when the objects dir is non-empty of items.
3. Built-in slugs bind to existing enum variants; extra slugs stay `Catalog(u16)` from **sorted** item ids.
4. `--catalog` does not imply `--sheet`. Always load `configs/objects` when present.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.
6. Idle 2-tick hash **changes**; document the new value on implement.

## Tests (M40 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | **new** idle hash (lock on implement); shipping TOML unchanged |
| no hardcoded recipes | without object files, Basket/Spear/FishingRod/Backpack not legal |
| shipped basket.toml | Craft Basket still fiber×2 → `ItemId::Basket` |
| override recipe | change inputs in TOML ⇒ hash ≠ shipped; Craft uses new inputs |
| visual optimized path | berry_bush / basket resolve under `optimized/` when files exist |
| stem list | viewer stem fallback is **only** `agent` |
| `--load` | Basket/Catalog restored; no double grant |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Object TOML + optimized LOD

- **Files:** one definition per non-agent stem; viewer resolve; drop STEMS except `agent`; hash-neutral visual tests

### PR 2: Recipes from config

- **Files:** delete hardcoded `recipe_spec` arms; always hash item `[sim]`; retarget idle-hash tests; `--load`

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
cargo run -p sim-cli -- --config configs/default.toml --objects configs/objects \
  --ticks 80 --llm mock --quiet
```

## Verification

Walkthrough: [`docs/M40-test-plan.md`](M40-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: new idle mock hash (documented); visuals hash-neutral; recipes only from TOML; Hello v5.

## Risks

- **Idle hash changes** because shipped item `[sim]` is always hashed.
- Postcard **append only**. Do not delete Basket/Spear/….
- `--load` must not re-grant crafted qty.
- Missing optimized glb ⇒ primitive; tests must not require the mesh to exist.
- Catalog `u16` from sorted slugs so directory order does not matter.
