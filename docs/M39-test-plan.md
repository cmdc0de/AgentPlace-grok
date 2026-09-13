# M39 test plan — see each new feature

Walkthrough for [`M39-plan.md`](M39-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (object TOML visuals + LOD, hashed catalog Craft). Next slice: [`M53-plan.md`](M53-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`, `ci`, …); unit tests under `--lib` need the module path (`protocol::tests::…`). Viewer tests live in the binary (`models::tests::…`).

**Success for the slice:** visual/LOD files are never hashed; catalog Craft only when overlay/`--catalog` is on. Overlay off / empty catalog ⇒ idle mock hash `70e5204d…`. Shipping TOML unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Visual definitions + LOD (hash-neutral)

```bash
cargo test -p sim-core --test objects visual_only_toml_hash_unchanged -- --exact --nocapture
cargo test -p sim-core --test objects lod_picks_near_mid_far_and_missing_mid -- --exact --nocapture
cargo test -p sim-core --test objects shipping_objects_visual_not_hashed -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::path_map_covers_locked_stems -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::missing_glb_falls_back -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::authored_files_are_not_in_state_hash -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::visual_toml_uses_glb_path -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::lod_and_stem_fallback -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::object_defs_are_not_in_state_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| visual-only toml | `glb` path parsed; hash unchanged vs no file |
| LOD | near < 8, mid < 24, else far; missing mid ⇒ far then glb then primitive |
| stem fallback | no toml ⇒ M38 stem still works |
| shipping objects | `configs/objects/berry_bush.toml` does not enter `state_hash` |

Drop a definition under `configs/objects/` pointing `glb` at a local file (large glbs stay gitignored). Missing mesh ⇒ coarser LOD, then stem, then primitive.

```bash
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
```

---

## 2. Hashed catalog (new items only)

```bash
cargo test -p sim-core --test objects overlay_parses_catalog -- --exact --nocapture
cargo test -p sim-core --test objects parse_locked_craft_inputs -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_produces_item -- --exact --nocapture
cargo test -p sim-core --test objects catalog_slug_sort_stable_u16 -- --exact --nocapture
cargo test -p sim-core --test objects catalog_load_restores_inventory_no_double_grant -- --exact --nocapture
cargo test -p sim-core --test objects empty_catalog_on_same_hash_as_off -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog off | Craft Catalog not legal; same hash as no files |
| catalog on + cord | `execute_primary` Craft with fiber produces `ItemId::Catalog(0)` |
| slug sort | two item files ⇒ `u16` from sorted slugs, not directory order |
| `--load` | inventory Catalog restored; no double grant |
| empty catalog | overlay on + no items ≡ off for hash |
| Hello v5 | postcard unchanged |

`--catalog` does **not** imply `--sheet`. Mock does not pick new Crafts; tests use `execute_primary`.

```bash
cargo run -p sim-cli -- --config configs/default.toml --catalog --objects configs/objects \
  --ticks 80 --llm mock --quiet
```

---

## 3. Default mock hash

```bash
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…`.

With catalog overlay on and `configs/objects/cord.toml` present, the 2-tick hash **differs** (catalog table is hashed). Overlay off keeps the idle hash even if the objects directory exists.

```bash
cargo run -p sim-cli -- --config configs/default.toml --catalog --objects configs/objects \
  --ticks 2 --llm mock --quiet
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
```

Live Ollama / overnight / imgui window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-07, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI and desktop browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; objects 11; ci 1; inspector 4 |
| §0 `cargo test -p viewer` | **pass** | 29/29 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 44/44 |
| §0 `cargo test -p shared` | **pass** | 14/14 |
| §1 visual/LOD `--exact` ×9 | **pass** | visual-only hash-neutral; LOD fallback; stem still works |
| §2 catalog `--exact` ×8 + Hello v5 | **pass** | Craft Catalog(0); slug sort; load no re-grant; postcard v5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc` |
| §3 catalog-on 2 ticks | **pass** | `final_hash=8092b495…` (≠ idle; catalog table hashed) |
| §2 catalog-on 80 ticks | **pass** | `final_tick=80` `final_hash=bb44cef7…` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
