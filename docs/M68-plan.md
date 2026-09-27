# M68 — Millstone remaining foods, extra recipes

**Status:** implemented  
**Depends on:** M67 complete (`docs/M67-plan.md`, git tag `M67`, commit `a5da8a3`)  
**Walkthrough:** [`docs/M68-test-plan.md`](M68-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-4 / RF-PG9

## Context

M63 required a placed spit for cooked_veg, jerky, biscuit, cake, and dried_fish. Flour already needs millstone; bread/stew stay spit. Catalog after M67 includes strut/hood/sill/muffin.

M68 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Switching five `craft.station` fields **and** new recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not dual-station, not ammo, not bonus-use every-instance.

## Goal

A researcher can:

1. **Craft** cooked_veg, jerky, biscuit, cake, and dried_fish only next to a **placed millstone** (same flour rule). Pocket millstone ≠ placed. Spit no longer unlocks those five. Bread/stew still need a placed spit. Flour still millstone.
2. **Craft** four new catalog items (**rail, mitten, quoin, roll**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`.

### A. Millstone remaining foods (RF-4)

Object TOML only. On **implement**, set `[sim.craft] station = "millstone"` on:

| file | today | M68 |
|---|---|---|
| `cooked_veg.toml` | spit | millstone |
| `jerky.toml` | spit | millstone |
| `biscuit.toml` | spit | millstone |
| `cake.toml` | spit | millstone |
| `dried_fish.toml` | spit | millstone |

**Instead of** spit, not dual-station. Chebyshev ≤1 of a placed millstone (existing station check). Pocket millstone ≠ placed.

Unchanged: bread/stew `station = "spit"`; flour `station = "millstone"`; millstone/spit `station = true`.

`hash_catalog` already writes `craft_station` ⇒ shipped-objects idle hashes **move**.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `rail` | wood×15 | reuse `wood.glb` |
| `mitten` | fiber×15 | reuse `low_poly_cloth.glb` |
| `quoin` | stone×13 | reuse `stones_and_grass.glb` |
| `roll` | food×12 | reuse `plants_ready.glb` |

No `uses` / `station` / interior. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M68 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Stations | dual-station (spit **or** millstone) (**RF-23**); bread/stew off spit; flour off millstone |
| Also not | ammo (**RF-19**); bonus-use every-instance (**RF-22**); interior walls (**RF-21**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Five M63 foods **switch** to millstone (not “either station”).
4. Bread/stew stay spit. Flour stays millstone.
5. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M68 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `2e3ed5ab…` |
| `--no-time` shipped objects | hash `979771cc…` |
| cooked_veg / jerky / biscuit / cake / dried_fish, no millstone | Craft illegal |
| those five + placed millstone | Craft from locked inputs |
| pocket millstone | not a station |
| placed spit only | those five still illegal |
| flour | still millstone |
| bread / stew | still spit |
| Craft rail / mitten / quoin / roll | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Millstone remaining foods

- five `craft.station = "millstone"`; tests vs spit/flour/bread identity

### PR 2: Recipes

- rail/mitten/quoin/roll; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: five remaining-food `craft.station = "millstone"`; rail/mitten/quoin/roll. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M68-test-plan.md`](M68-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `979771cc…`; default (time on) `2e3ed5ab…`; Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** Station slug + recipes move idle hashes.
- **Existing tests.** `catalog_on_craft_{cooked_veg,jerky,biscuit,cake,dried_fish}` and `remaining_food_crafts_need_placed_spit` must Place millstone / assert millstone, not spit.
- **`--load`.** Station requirement is catalog overlay, not checkpoint extra-apply.
- **No protocol / format bump.**
