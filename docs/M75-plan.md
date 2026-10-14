# M75 — Dual-station food crafts, extra recipes

**Status:** implemented  
**Depends on:** M74 complete (`docs/M74-plan.md`, git tag `M74`, commit `0225584`)  
**Walkthrough:** [`docs/M75-test-plan.md`](M75-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-23 / RF-PG9

## Context

M68 set cooked_veg, jerky, biscuit, cake, and dried_fish to `station = "millstone"` (they had been spit). Bread and stew stay spit. Flour stays millstone. Catalog after M74 includes batten/wimple/voussoir/bannock.

M75 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Station text is catalog `[sim]`, not overlay and not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. The five station strings and the four new recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not flour-at-spit, not bread-at-millstone, not a third station.

## Goal

A researcher can:

1. **Craft** cooked_veg, jerky, biscuit, cake, or dried_fish next to a placed **millstone or spit**. Either station is enough. Neither ⇒ Craft illegal.
2. Leave **bread and stew spit-only**, and **flour millstone-only**.
3. **Craft** four new catalog items (**noggin, cowl, keystone, farl**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Craft unless tests `execute_primary`. No new CLI flag.

### A. Dual station (RF-23)

Keep one string. A `|` means **or**:

```toml
[sim.craft]
station = "millstone|spit"
```

On **implement**, set that value on:

| file | today | M75 |
|---|---|---|
| `cooked_veg.toml` | millstone | `millstone\|spit` |
| `jerky.toml` | millstone | `millstone\|spit` |
| `biscuit.toml` | millstone | `millstone\|spit` |
| `cake.toml` | millstone | `millstone\|spit` |
| `dried_fish.toml` | millstone | `millstone\|spit` |

Do **not** change `bread.toml`, `stew.toml`, or `flour.toml`.

Who/when: `craft_station_ok` / `legal_actions`. Agent must be Chebyshev ≤ 1 of a placed station whose slug is **any** segment (trim, skip empty). One station is enough. Order in the string does not matter for legality.

Hash: still `craft_station` bytes. The new string **moves** shipped idle hashes. No new hash tag. Single-slug files hash as today.

`--load` restores placed stations. Do not re-Place. Do not Craft on decode. The rule is read from the catalog at load.

Not: a station array, a third station, stations on non-food crafts, blocking Move at a station.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**. These are not the dual-station foods.

| id | inputs | visual |
|---|---|---|
| `noggin` | wood×22 | reuse `wood.glb` |
| `cowl` | fiber×22 | reuse `low_poly_cloth.glb` |
| `keystone` | stone×20 | reuse `stones_and_grass.glb` |
| `farl` | food×19 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M75 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Stations | flour at a spit; bread/stew at a millstone; a third station |
| Also not | Unix sockets (**RF-8**); wasm32 Win/mac (**RF-10**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No postcard append. Station text is catalog TOML.
2. **format_version still writes 3 / reads v2+v3.**
3. `|` is or. One nearby station is enough. Empty segments ignored.
4. Only the five M68 foods become dual. Flour, bread, and stew stay single-station.
5. Do not change shipping `coop.toml` or `default.toml`. No TLS. `cargo test` never needs the network.

## Tests (M75 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `187acad9…` |
| `--no-time` shipped objects | hash `b44e30d7…` |
| cooked_veg + millstone | Craft succeeds (M68 identity) |
| cooked_veg + spit, no millstone | Craft succeeds |
| cooked_veg + neither | illegal |
| jerky or biscuit + spit | Craft succeeds |
| bread + millstone only | illegal |
| flour + spit only | illegal |
| `--load` | placed station remains; no extra Craft |
| Craft noggin / cowl / keystone / farl | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: Dual station

- Split `station` on `|`; five food files; millstone still works; spit works; bread/flour stay single; tests

### PR 2: Recipes

- noggin/cowl/keystone/farl; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `station = "millstone|spit"` on the five foods; noggin/cowl/keystone/farl. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M75-test-plan.md`](M75-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `b44e30d7…`; default (time on) `187acad9…`; Hello v5; format_version 3 write.

## Risks

- **Postcard / protocol.** None. Station is catalog TOML, not ExperimentConfig.
- **`--load`.** Restore work places. Do not re-Place. Do not Craft on decode.
- **Hash.** The five station strings and the new recipes move idle hashes. Single-slug hash shape stays.
- **Pocket cap.** Inputs wood×22 / food×19; tests must not stock 2×32.
- **No protocol / format bump.**
