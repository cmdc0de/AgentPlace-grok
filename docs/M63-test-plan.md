# M63 test plan — see each new feature

Walkthrough for [`M63-plan.md`](M63-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (crate dawn decay, remaining food stations, pole/cape/paver/tart). Next slice: later work at the bottom of [`M63-plan.md`](M63-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `survival`, `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `1b9117a9…`. Default (time on) shipped-objects `f583c391…`. Crate axe wear `[0]` → `[1]` at dawn; 8 dawns consume 1 from crate; `--no-time` crate wear stays `[0]`. Held axe still decays. cooked_veg/jerky/biscuit/cake/dried_fish illegal without placed spit. Flour millstone; bread/stew spit. Craft pole/cape/paver/tart. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Crate dawn decay

```bash
cargo test -p sim-core --test objects crate_dawn_decays_stored_axe -- --exact --nocapture
cargo test -p sim-core --test objects eight_crate_dawns_consume_one_axe -- --exact --nocapture
cargo test -p sim-core --test objects no_time_does_not_decay_crate_axe -- --exact --nocapture
cargo test -p sim-core --test objects dawn_decays_held_axe -- --exact --nocapture
```

| Test | Success |
|---|---|
| crate dawn | wear `[0]` → `[1]` |
| 8 dawns | crate consumes 1 |
| `--no-time` | crate wear stays `[0]` |
| held | agent axe still +1 |

---

## 2. Remaining food stations

```bash
cargo test -p sim-core --test objects remaining_food_crafts_need_placed_spit -- --exact --nocapture
cargo test -p sim-core --test objects cooked_veg_needs_placed_spit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_jerky -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_biscuit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cake -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_dried_fish -- --exact --nocapture
cargo test -p sim-core --test objects flour_needs_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bread -- --exact --nocapture
```

| Test | Success |
|---|---|
| no spit | five foods illegal |
| + spit | cooked_veg/jerky/biscuit/cake/dried_fish Craft |
| flour | millstone |
| bread | spit |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_pole -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cape -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_paver -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_tart -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | pole / cape / paver / tart from locked inputs |
| catalog-off | Catalog crafts illegal |

---

## 4. Hello / format / idle CLI

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
| shipped `--no-time` | `1b9117a9…` |
| default time on | `f583c391…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-15, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 109; storage 25; telemetry 5; survival 18) |
| §0 `cargo test -p viewer` | **pass** | 61/61 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 crate dawn `--exact` ×4 | **pass** | crate `[0]`→`[1]`; 8 dawns consume 1; `--no-time` stays `[0]`; held still +1 |
| §2 food stations `--exact` ×8 | **pass** | five foods illegal without spit; Craft with spit; flour millstone; bread spit |
| §3 recipes `--exact` ×5 | **pass** | Craft pole/cape/paver/tart; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=f583c391c19f90f944c56f87fcd4d14c08e3e1b66abfe439847b869dc976a4f9`; `--no-time` `final_hash=1b9117a96d1f454f24ec022f331df1840fe3fe166aaf413dc827e0d28bc58ab3` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
