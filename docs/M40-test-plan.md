# M40 test plan — see each new feature

Walkthrough for [`M40-plan.md`](M40-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (config-owned objects + recipes, optimized glb paths). Next slice: [`M50-plan.md`](M50-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`, `ci`, …); unit tests under `--lib` need the module path (`protocol::tests::…`). Viewer tests live in the binary (`models::tests::…`).

**Success for the slice:** recipes come from `configs/objects/*.toml`; no hardcoded Basket/Spear/… tables. Viewer stem fallback is **only** `agent`. Visuals never hashed. Idle mock 2 ticks with shipped objects is `cd1e0853…`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Object TOML + optimized LOD

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
cargo test -p viewer --bin viewer models::tests::visual_optimized_path_when_present -- --exact --nocapture
```

| Test | Success |
|---|---|
| visual-only | hash unchanged vs no file |
| LOD | near/mid/far; missing mid ⇒ far then glb then primitive |
| stem list | fallback is **only** `agent` |
| optimized path | basket / berry_bush resolve when `assets/models/optimized/*.glb` exist |

```bash
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
```

---

## 2. Recipes from config

```bash
cargo test -p sim-core --test objects no_hardcoded_recipes_without_files -- --exact --nocapture
cargo test -p sim-core --test objects shipped_basket_craft_fiber_two -- --exact --nocapture
cargo test -p sim-core --test objects override_recipe_changes_hash_and_inputs -- --exact --nocapture
cargo test -p sim-core --test objects builtin_slugs_are_not_catalog_u16 -- --exact --nocapture
cargo test -p sim-core --test objects parse_locked_craft_inputs -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_produces_item -- --exact --nocapture
cargo test -p sim-core --test objects catalog_slug_sort_stable_u16 -- --exact --nocapture
cargo test -p sim-core --test objects catalog_load_restores_inventory_no_double_grant -- --exact --nocapture
cargo test -p sim-core --test objects load_basket_restores_no_double_grant -- --exact --nocapture
cargo test -p sim-core --test objects empty_catalog_on_same_hash_as_off -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_not_legal_same_hash -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
```

| Test | Success |
|---|---|
| no files | Basket/Spear/FishingRod/Backpack not legal |
| shipped basket.toml | Craft Basket fiber×2 → `ItemId::Basket` |
| override | TOML input change ⇒ hash ≠ shipped |
| builtins vs Catalog | basket is `ItemId::Basket`; cord is `Catalog(0)` |
| `--load` | Basket/Catalog restored; no double grant |
| Hello v5 | postcard unchanged |

```bash
cargo run -p sim-cli -- --config configs/default.toml --objects configs/objects \
  --ticks 80 --llm mock --quiet
```

---

## 3. Default mock hash

```bash
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=cd1e0853…` (new idle hash; shipped item `[sim]` is always loaded).

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
| §0 `cargo test -p sim-core` | **pass** | lib 34; objects 16; ci 1; inspector 4 |
| §0 `cargo test -p viewer` | **pass** | 30/30 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 44/44 |
| §0 `cargo test -p shared` | **pass** | 14/14 |
| §1 visual/LOD `--exact` ×10 | **pass** | stem fallback only agent; optimized path when present |
| §2 recipes `--exact` ×12 + Hello v5 | **pass** | no hardcoded recipes; Basket from TOML; load no re-grant; postcard v5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130` |
| §2 objects 80 ticks | **pass** | `final_tick=80` `final_hash=006f3624…` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
