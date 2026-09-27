# M68 test plan — see each new feature

Walkthrough for [`M68-plan.md`](M68-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (millstone remaining foods, rail/mitten/quoin/roll). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `979771cc…`. Default (time on) shipped-objects `2e3ed5ab…`. cooked_veg/jerky/biscuit/cake/dried_fish illegal without placed millstone; succeed with millstone; pocket millstone not a station; placed spit only still illegal. Flour still millstone. Bread/stew still spit. Craft rail/mitten/quoin/roll. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Millstone remaining foods

```bash
cargo test -p sim-core --test objects remaining_food_crafts_need_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects remaining_food_illegal_with_only_spit -- --exact --nocapture
cargo test -p sim-core --test objects remaining_food_pocket_millstone_not_station -- --exact --nocapture
cargo test -p sim-core --test objects cooked_veg_needs_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_jerky -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_biscuit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cake -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_dried_fish -- --exact --nocapture
cargo test -p sim-core --test objects flour_needs_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects bread_needs_placed_spit -- --exact --nocapture
cargo test -p sim-core --test objects stew_needs_placed_spit -- --exact --nocapture
```

| Test | Success |
|---|---|
| no millstone | five remaining foods illegal |
| placed spit only | those five still illegal |
| pocket millstone | not a station |
| + placed millstone | cooked_veg / jerky / biscuit / cake / dried_fish Craft |
| flour | still millstone |
| bread / stew | still spit |

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_rail -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_mitten -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_quoin -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_roll -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | rail / mitten / quoin / roll |
| catalog-off | Catalog crafts illegal |

---

## 3. Hello / format / idle CLI

```bash
cargo test -p sim-core --test objects no_time_shipped_objects_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

| Test | Success |
|---|---|
| no-objects `--no-time` | `70e5204d…` |
| shipped `--no-time` | `979771cc…` |
| default time on | `2e3ed5ab…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-27, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 |
| §0 `cargo test -p viewer` | **pass** | 64/64 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 millstone `--exact` ×11 | **pass** | millstone required; spit-only illegal; pocket millstone not a station; flour millstone; bread/stew spit |
| §2 recipes `--exact` ×5 | **pass** | Craft rail/mitten/quoin/roll; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=2e3ed5ab4bf70257defa0b807e16376f9305a36164fb8705073d8e171ce51bb6`; `--no-time` `final_hash=979771cc93bc9ea26287608cf83b278a2ce3dab9859cabc9007103519f75e0ff` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
