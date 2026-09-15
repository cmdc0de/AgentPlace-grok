# M62 — Stations for bread/stew, Store wear, extra recipes

**Status:** planned (not yet implemented)  
**Depends on:** M61 complete (`docs/M61-plan.md`, git tag `M61`, commit `fc26c53`)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes); M58 later-table stations for bread/stew; M60 later-table Store wear

## Context

M61 decayed held tools at dawn. Flour already needs a **placed millstone**. Bread and stew Craft with no station. Store still truncates freshest wear; crates have no wear map. Catalog after M61 includes bench/shawl/cobble/cake. Spit is a craft with no station role.

M62 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and spit/bread/stew `[sim]` station fields **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Store wear is empty on idle mock ⇒ idle hashes **do not** move from this field. Not every remaining food craft, not dawn decay of crate contents.

## Goal

A researcher can:

1. **Craft bread or stew** only when a **placed spit** is Chebyshev ≤1 (same millstone/flour pattern). Pocket spit ≠ placed. Flour still needs placed millstone. Catalog-off those Crafts illegal.
2. **Store** a worn axe and **Retrieve** it with the **same wear slot** (crate holds a wear vec per ItemId). Idle mock does not Store ⇒ idle hashes unchanged from this field.
3. **Craft** four new catalog items (**rack, wrap, tile, pie**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Store / Place unless tests `execute_primary`. No new CLI flag.

### A. Stations for bread/stew (M58 leftover, thin)

Existing millstone/flour machinery. No new Craft verb.

| file | change |
|---|---|
| `spit.toml` (existing) | `[sim] station = true` (Place-able 1×1 workstation, like millstone) |
| `bread.toml` | `[sim.craft] station = "spit"` |
| `stew.toml` | `[sim.craft] station = "spit"` |

Flour / millstone unchanged. Other food crafts (cooked_veg, jerky, biscuit, cake, dried_fish) stay station-less.

Pocket spit ≠ placed. Chebyshev ≤1 of a placed spit, same as `craft_station_ok`. `hash_catalog` already writes `station` + `craft_station` for every row ⇒ shipped-objects idle hashes **move**.

`--load` restores `work_places`; do not re-Place.

### B. Transfer wear on Store (M60 leftover)

Same slot rule as Give: **freshest leave** (vec end).

`Container` gains `tool_wear: BTreeMap<ItemId, Vec<u32>>` (`#[serde(default)]`). Hash when non-empty (item + each u32 LE) after crate qty. Empty omitted.

On **successful Store** of `qty`:

1. Before take: `take_end_wear` (`None` if the agent has no wear map for that item).
2. Take items as today.
3. Append those slots to the **crate** (oldest-first among the taken, same as Transfer-to-agent).
4. Failure paths (haul pay / pack unload) restore items **and** wear to the agent; crate rolls back as today.

On **successful Retrieve** of `added`:

1. After crate `try_retrieve`, split **`added` slots from the crate vec end** (pad implicit 0 if the crate had qty but a short/absent wear vec).
2. Append to the agent. Partial add (`added < qty`) returns leftover items **and** leftover wear to the crate.

Same-agent Store 1 then Retrieve 1 of `[3,0]` round-trips to `[3,0]`.

Researcher `/give` still mints fresh. Transfer agent-to-agent unchanged. Dawn decay still only **held** tools (not crate contents).

Ckpt: serde default empty wear. Decode fallback a Container without `tool_wear` if needed. **Do not** bump `format_version`. `--load` restores crate wear; do not re-Store.

Idle mock does not Store ⇒ idle hashes unchanged from this field.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `rack` | wood×10 | reuse `wood.glb` |
| `wrap` | fiber×10 | reuse `low_poly_cloth.glb` |
| `tile` | stone×8 | reuse `stones_and_grass.glb` |
| `pie` | food×7 | reuse `plants_ready.glb` |

No `uses` / `station` / tool bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M62 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX; household-home / invention / downed meshes; requiring stations for every remaining food craft; dawn decay of crate contents |
| Not M62 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; per-tick decay; decaying every instance not just most-worn; bread/stew using millstone instead of spit |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard `PrimaryAction` append.
2. **format_version still writes 3 / reads v2+v3.** Container `tool_wear` serde-default + decode fallback. Do not bump.
3. Bread/stew use **spit**, not millstone. Flour stays millstone.
4. Store/Retrieve use the same freshest-leave vec as Transfer. Dawn does not decay crate contents.
5. Shipped-objects idle hashes **change** (new files + station fields). No-objects `--no-time` stays `70e5204d…`. Store wear does not move idle hashes by itself.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M62 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash ≠ M61 `16804536…` (lock on implement) |
| default CLI 2 ticks (time on) | hash ≠ M61 `3512dde6…` (document) |
| bread/stew no spit | Craft Wait / illegal without a placed spit |
| bread/stew + placed spit | Craft from locked inputs (food×2) |
| pocket spit | bread Craft still illegal |
| flour | still needs placed millstone (identity) |
| Store then Retrieve wear | agent `[3,0]` Store 1 ⇒ crate `[0]`, agent `[3]`; Retrieve 1 ⇒ agent `[3,0]` |
| `/give` mint | still fresh |
| Craft rack / wrap / tile / pie | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Bread/stew stations

- spit `station = true`; bread/stew `craft.station = "spit"`; tests vs millstone identity

### PR 2: Store wear

- Container `tool_wear`; Store/Retrieve move freshest slots; hash; ckpt default; tests

### PR 3: Recipes

- rack/wrap/tile/pie TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag.

On implement: spit/bread/stew station fields; Container `tool_wear`; create rack/wrap/tile/pie TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test storage
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: `docs/M62-test-plan.md` (written on implement).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; shipped-objects hashes **new** (document); Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** `station` / `craft_station` hashed for every row; spit/bread/stew field changes move idle hashes even without Craft.
- **Container postcard.** Trailing `tool_wear` + serde default; fallback decode without the field. Do not bump format_version.
- **Store failure rollback.** Restore wear to the agent if take/unload fails after crate add.
- **Retrieve partial.** Leftover items **and** wear return to the crate.
- **Dawn.** Crate contents do **not** decay this slice.
- **Recipe inputs** stay distinct (wood×10, fiber×10, stone×8, food×7).
- **No protocol / format bump.**
