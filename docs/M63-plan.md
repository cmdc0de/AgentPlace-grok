# M63 — Crate dawn decay, remaining food stations, extra recipes

**Status:** implemented  
**Depends on:** M62 complete (`docs/M62-plan.md`, git tag `M62`, commit `e850533`)  
**Walkthrough:** [`docs/M63-test-plan.md`](M63-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes); M62 later-table crate dawn decay + remaining food stations

## Context

M62 required a placed spit for bread/stew and moved wear on Store/Retrieve. Held tools decay at dawn; **crate** contents do not. cooked_veg, jerky, biscuit, cake, and dried_fish Craft with no station. Catalog after M62 includes rack/wrap/tile/pie.

M63 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and five `craft.station` fields **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Crate dawn decay is empty on idle mock ⇒ idle hashes **do not** move from this field. Not per-tick decay, not decaying every instance, not millstone for these foods.

## Goal

A researcher can:

1. **Store** a worn axe in a crate overnight and see it **wear at dawn** (+1 on the most-worn crate slot, same as held tools). `--no-time` ⇒ no decay. 8 dawns with `uses = 8` consume 1 from the crate. Held-tool dawn decay unchanged.
2. **Craft** cooked_veg, jerky, biscuit, cake, and dried_fish only next to a **placed spit** (same bread/stew rule). Pocket spit ≠ placed. Flour still needs millstone.
3. **Craft** four new catalog items (**pole, cape, paver, tart**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Store / Place unless tests `execute_primary`. No new CLI flag.

### A. Dawn decay of crate contents (M62 leftover)

Reuse the most-worn `wear_tool` rule on **crates** (Container items + `tool_wear`). No new overlay. No new catalog field.

When `apply_dawn_if_due` runs (after agent energy refill **and** held-tool decay): for each stockpile cell, for each catalog item with `uses > 0` and crate qty > 0, call a crate `wear_tool` **once** per ItemId (most-worn slot +1; consume 1 qty when `>= uses`; drop empty crate).

- Two axes in one crate: only the most-worn slot +1.
- `--no-time` / `time_enabled = false` ⇒ no dawn ⇒ no crate decay.
- Held tools still decay as M61 (before crate decay this slice).
- `--load` restores crate wear; do not re-apply a dawn already in the checkpoint tick.

Hash crate `tool_wear` already (M62). Idle mock has empty crates ⇒ idle hashes unchanged from this field.

`uses = 8` on axe/hammer/hoe/net unchanged.

### B. Stations for remaining food crafts (M62 leftover)

Existing spit/bread machinery. No new Craft verb. Spit already `station = true`.

| file (existing) | change |
|---|---|
| `cooked_veg.toml` | `[sim.craft] station = "spit"` |
| `jerky.toml` | `[sim.craft] station = "spit"` |
| `biscuit.toml` | `[sim.craft] station = "spit"` |
| `cake.toml` | `[sim.craft] station = "spit"` |
| `dried_fish.toml` | `[sim.craft] station = "spit"` |

Bread/stew/flour/millstone unchanged. Pocket spit ≠ placed. Chebyshev ≤1. `hash_catalog` already writes `craft_station` for every row ⇒ shipped-objects idle hashes **move**.

`--load` restores `work_places`; do not re-Place.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `pole` | wood×11 | reuse `wood.glb` |
| `cape` | fiber×11 | reuse `low_poly_cloth.glb` |
| `paver` | stone×9 | reuse `stones_and_grass.glb` |
| `tart` | food×8 | reuse `plants_ready.glb` |

No `uses` / `station` / tool bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M63 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX; household-home / invention / downed meshes |
| Not M63 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; per-tick decay; decaying every instance not just most-worn; remaining food crafts using millstone |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard `PrimaryAction` append.
2. **format_version still writes 3 / reads v2+v3.** Crate `tool_wear` already on Container.
3. Remaining food crafts use **spit**, not millstone. Flour stays millstone.
4. Crate dawn decay is most-worn once per ItemId, after held decay. `--no-time` does not decay.
5. Shipped-objects idle hashes **change** (new files + `craft.station`). No-objects `--no-time` stays `70e5204d…`. Crate decay does not move idle hashes by itself.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M63 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `1b9117a9…` |
| default CLI 2 ticks (time on) | hash `f583c391…` |
| crate dawn | time on, `ticks_per_day = 2`, crate axe wear `[0]`, run 2 ticks ⇒ `[1]` |
| 8 crate dawns | `uses = 8` consume 1 from crate |
| `--no-time` crate | many ticks, crate wear unchanged |
| held still decays | agent-held axe still +1 at dawn (M61 identity) |
| cooked_veg / jerky / biscuit / cake / dried_fish no spit | Craft illegal |
| + placed spit | those Crafts succeed from locked inputs |
| flour | still millstone |
| bread/stew | still spit (identity) |
| Craft pole / cape / paver / tart | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Crate dawn decay

- Crate `wear_tool`; dawn after held decay; `--no-time` identity; 8-dawn break

### PR 2: Remaining food stations

- five `craft.station = "spit"`; tests vs millstone/bread identity

### PR 3: Recipes

- pole/cape/paver/tart TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag.

On implement: crate dawn `wear_tool`; five food `craft.station`; create pole/cape/paver/tart TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M63-test-plan.md`](M63-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `1b9117a9…`; default (time on) `f583c391…`; Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** `craft_station` hashed for every row; five food files move idle hashes even without Craft.
- **Crate empty.** Consume last qty removes the stockpile (same as Retrieve).
- **Dawn order.** Held decay first, then crates. `--load` must not re-fire.
- **Existing craft tests.** `catalog_on_craft_{cooked_veg,jerky,biscuit,cake,dried_fish}` must Place spit (same as bread).
- **Recipe inputs** stay distinct (wood×11, fiber×11, stone×9, food×8).
- **No protocol / format bump.**
