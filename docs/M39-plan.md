# M39 — Object definition files (visual + LOD + hashed catalog)

**Status:** implemented  
**Depends on:** M38 complete (`docs/M38-plan.md`, git tag `M38`, commit `76f4917`)  
**Walkthrough:** [`docs/M39-test-plan.md`](M39-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-8, PG-6 LOD)

## Context

M38 maps art by **filename stem** (`assets/models/berry_bush.glb`). Local files like `big_low_poly_berry_bush.glb` do not bind. `ItemId` and `Recipe` are Rust enums; a new craftable item needs a code change.

M39 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Visuals are not hashed. Catalog sim fields are hashed only when overlay on. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Drop `configs/objects/berry_bush.toml` that points `glb` at `assets/models/big_low_poly_berry_bush.glb` (and optional LOD). Viewer uses that mesh without renaming. Same `state_hash`.
2. Add a **new craftable item** (`kind = "item"` + `[sim.craft]`). Overlay on ⇒ Craft; inventory `ItemId::Catalog(u16)`. Overlay off ⇒ **idle hash `70e5204d…`**.
3. Change only `[visual]` / LOD and keep the hash. Change `[sim]` (or hold a catalog item) and the hash **does** change.
4. Missing glb / LOD ⇒ coarser LOD, then M38 stem, then primitive. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

Directory: `configs/objects/*.toml`. CLI: `--objects DIR` (viewer always reads visuals). Overlay `[catalog] enabled = true` / `--catalog` enables **sim** fields. `--catalog` does **not** imply `--sheet`.

```toml
id = "berry_bush"
kind = "vegetation"          # vegetation | item | agent | crate | animal | fish | crop

[visual]
glb = "assets/models/big_low_poly_berry_bush.glb"
[visual.lod]
near = "assets/models/big_low_poly_berry_bush.glb"
mid  = "assets/models/optimized/big_low_poly_berry_bush.glb"
far  = "assets/models/optimized/plants_ready.glb"

[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
```

### A. Visual + LOD (hash-neutral)

- Viewer: matching definition `id` ⇒ `visual.glb` or LOD by camera distance (near < 8 cells, mid < 24, else far). Else M38 stem. Else primitive.
- Do not hash the objects directory. `[visual]`-only files are display-only.
- `AGENTPLACE_MODELS` still used for stem files.

### B. Hashed catalog (new items only)

- Do **not** replace Wood/Basket/…. Catalog **adds**.
- Postcard **append** `ItemId::Catalog(u16)` and `Recipe::Catalog(u16)` at the **end**. `u16` = rank of slug among catalog **item** files (sorted).
- Overlay **off** / no item files ⇒ `Catalog` never appears, **same hashes**.
- Overlay **on**: hash slug, weight, craft inputs, output qty. Empty catalog ≡ off for hash.
- Craft legal when ingredients present; execute grants `Catalog(id)`. Mock does not pick new Crafts unless tests `execute_primary`.
- `--load`: restore `Catalog(u16)`; do not re-grant. Unknown leftover `Catalog(n)` stays.
- New vegetation / world species: **not** this slice.

Do not invent extra overlays. Create named `configs/objects/` examples **on implement** only.

## Out of scope (later)

| Later | What |
|---|---|
| **M40** | [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| After M40 | protobuf/TLS; DEX accuracy; STR haul; tech tree; world species from TOML; string ItemId / ckpt bump; agent meshes; `/ckpt` in the page; skeletal animation; hot-reload glb |
| Not M39 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** ItemId/Recipe append is checkpoint postcard, not a wire bump.
2. Visuals never hashed. `[sim]` hashed only with `--catalog` / overlay on.
3. Catalog `u16` from **sorted slugs**.
4. `--catalog` does not imply `--sheet`.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M39 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| visual-only toml | `glb` path used; hash unchanged vs no file |
| LOD | near/mid/far pick; missing mid ⇒ far or glb or primitive |
| stem fallback | no toml ⇒ M38 stem still works |
| catalog off | Craft Catalog not legal; same hash as no files |
| catalog on + new item | Craft with inputs produces `Catalog(n)`; hash ≠ off |
| slug sort | two item files ⇒ stable `u16` |
| `--load` | inventory Catalog restored; no double grant |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Visual definitions + LOD

- **Files:** parse `configs/objects/*.toml` `[visual]`; viewer resolve; stem fallback; hash-neutral tests

### PR 2: Catalog items + Craft

- **Files:** `ItemId::Catalog` / `Recipe::Catalog` append; overlay `--catalog`; execute/legal; load; hash tests

## Config / CLI

No shipping TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
cargo run -p sim-cli -- --config configs/default.toml --catalog --objects configs/objects \
  --ticks 80 --llm mock --quiet
```

## Verification

Walkthrough: [`docs/M39-test-plan.md`](M39-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash unchanged; visual toml hash-neutral; catalog Craft only when overlay on.

## Risks

- **Append only** on `ItemId` / `Recipe`. Do not reorder variants.
- Visual files must not enter `state_hash`.
- Catalog `u16` from sorted slugs so directory order does not matter.
- `--load` must not re-grant crafted qty.
- Large glbs stay gitignored; tests use tiny paths / missing files.
