# M56 — Axe gather bonus, Draco glTF decode, extra recipes

**Status:** implemented  
**Depends on:** M55 complete (`docs/M55-plan.md`, git tag `M55`, commit `6a6eb05`)  
**Walkthrough:** [`docs/M56-test-plan.md`](M56-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes, PG-6 leftover decode); M55 later-table axe gather bonus / GltfDracoDecoderPlugin

## Context

M55 gave hoe/net Farm/Fish `skill_roll` bonuses. Vegetation Gather is still basket +15 / else 0; axe is a craft with **no** Gather bonus. Stone gather is bonus 0. `bevy_gltf_draco` is a workspace/viewer dependency (`0ae44b0`) but the decoder plugin is **not** registered, so Draco glbs fail like a missing mesh. Catalog after M55 includes rope/needle/bucket/shield.

M56 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and `[sim] gather_bonus` **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Draco is **hash-neutral**.

## Goal

A researcher can:

1. Hold an **axe** and succeed vegetation **Gather** more often (`[sim] gather_bonus = 25`). Catalog-off / not holding ⇒ today (basket +15 / else 0). Stone gather unchanged.
2. Open the native viewer and load **Draco-compressed** glbs (register the already-added decoder plugin). Hash-neutral. Missing/failed glb still M47 sentinel.
3. **Craft** four new catalog items (**fence, mat, snare, spit**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Gather unless tests `execute_primary`.

### A. Axe gather bonus

Hashed `[sim] gather_bonus` (omit = 0), same max-held rule as hoe/net.

| slug | field | value |
|---|---|---|
| `axe` (existing file) | `gather_bonus` | **25** |

Vegetation Gather `skill_roll` bonus today is basket +15 / else 0. Use `max(basket_bonus, catalog gather_bonus)`. Holding both basket and axe ⇒ **25**, not 40. Qty stays today’s STR / basket food extra. **Stone** gather stays bonus 0.

Catalog-off / empty catalog / not holding ⇒ identity. `--load` restores inventory; do not persist derived odds.

### B. GltfDracoDecoderPlugin

Native viewer only. `bevy_gltf_draco` is already a workspace/viewer dependency; it is **not** registered.

On implement: `app.add_plugins(…)` the crate’s decoder plugin next to `DefaultPlugins`. Confirm the exact type name from the crate (expected `GltfDracoDecoderPlugin`).

- **Hash-neutral.** Visuals never enter `state_hash`.
- `cargo test -p viewer` must not need a GPU window. Existing tests still pass. Optional unit-test: plugin is in the `App` plugin list (no window).
- Failed Draco decode ⇒ same M47 sentinel as a missing glb. No new CLI flag.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `fence` | wood×4 | reuse `wood.glb` |
| `mat` | fiber×5 | reuse `low_poly_cloth.glb` |
| `snare` | fiber×2 + wood×1 | reuse `spear.glb` |
| `spit` | wood×2 + fiber×1 | reuse `wood.glb` |

No tool-use bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | Done — [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| **M60** | Done — [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| **M61** | Done — [`M61-plan.md`](M61-plan.md) — time-decay wear, out-dir disk walk, extra recipes |
| **M62** | [`M62-plan.md`](M62-plan.md) — stations for bread/stew, Store wear, extra recipes |
| After M62 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX |
| Not M56 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; axe changing Gather **qty**; stone-gather bonus |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.** Odds are derived; do not store them.
3. `gather_bonus` omit = 0. Max with basket, not sum. Vegetation Gather only; stone stays 0. Qty unchanged.
4. Draco plugin is hash-neutral. Confirm crate type name on implement.
5. Shipped-objects idle hashes **change** (new files + axe `gather_bonus` + field hashed for every catalog row). No-objects `--no-time` stays `70e5204d…`. Draco does not move hashes.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M56 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `6d2df92b…` |
| default CLI 2 ticks (time on) | hash `34f16591…` |
| catalog-off Gather | bonus 0 without basket; +15 with basket |
| hold axe, catalog on | Gather `skill_roll` bonus 25 vs 0 without; basket+axe 25 not 40 |
| stone gather | bonus still 0 with axe |
| viewer plugin | decoder plugin registered; `cargo test -p viewer` still passes |
| Craft fence / mat / snare / spit | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required.

## PR Plan

### PR 1: Axe gather_bonus

- `[sim] gather_bonus`; max with basket; stone identity; axe.toml = 25

### PR 2: Draco plugin

- Register decoder next to `DefaultPlugins`; viewer tests; no GPU

### PR 3: Recipes

- fence/mat/snare/spit TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (axe is catalog; Draco is always-on in the native viewer).

On implement: `gather_bonus` on axe; create fence/mat/snare/spit TOML; register Draco plugin. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M56-test-plan.md`](M56-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `6d2df92b…`; default (time on) `34f16591…`; Hello v5; format_version 3 write.

## Risks

- **Basket + axe is max, not sum.** Do not turn Gather into 40.
- **Stone gather stays 0.** Axe is vegetation Gather only.
- **Catalog `[sim] gather_bonus`** hashed for every entry (omit=0), so shipped-objects hashes move even without Gather happening.
- **Draco plugin type name** confirmed on implement; do not invent a second Draco crate.
- Mock never Gather on default idle. Draco is hash-neutral.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
