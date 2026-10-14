# M75 test plan — see each new feature

Walkthrough for [`M75-plan.md`](M75-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (millstone or spit, noggin/cowl/keystone/farl). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `b44e30d7…`. Default (time on) shipped-objects `187acad9…`. cooked_veg/jerky/biscuit/cake/dried_fish craft next to a placed millstone **or** spit. Neither station ⇒ illegal. Bread stays illegal at a millstone only. Flour stays illegal at a spit only. `--load` keeps the placed station and does not Craft. Craft noggin/cowl/keystone/farl. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Dual station

```bash
cargo test -p sim-core --test objects cooked_veg_needs_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects cooked_veg_spit_only_crafts -- --exact --nocapture
cargo test -p sim-core --test objects remaining_food_crafts_need_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects remaining_food_legal_with_only_spit -- --exact --nocapture
cargo test -p sim-core --test objects biscuit_spit_only_crafts -- --exact --nocapture
cargo test -p sim-core --test objects bread_millstone_only_illegal -- --exact --nocapture
cargo test -p sim-core --test objects flour_spit_only_illegal -- --exact --nocapture
cargo test -p sim-core --test objects load_station_does_not_extra_craft -- --exact --nocapture
```

| Test | Success |
|---|---|
| cooked_veg + millstone | Craft succeeds |
| cooked_veg + spit only | Craft succeeds |
| no station | the five foods illegal |
| spit only | all five legal |
| biscuit + spit | Craft succeeds |
| bread + millstone | illegal |
| flour + spit | illegal |
| `--load` | station count and event count unchanged |

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_noggin -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cowl -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_keystone -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_farl -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | noggin / cowl / keystone / farl |
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
| shipped `--no-time` | `b44e30d7…` |
| default time on | `187acad9…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-14, repo root. Each `--exact` line invoked separately. Package-level §0 also run. One `sleep` failure was a shared temp-dir race in `apply_rect_hut`; the helper now uses a unique directory. Re-run passed. Live model and the viewer window **not** driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 after the temp-dir fix; objects 184/184; sleep 49/49 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 dual station `--exact` ×8 | **pass** | millstone still crafts cooked_veg; spit-only crafts cooked_veg and biscuit; no station illegal; all five legal at spit; bread millstone-only illegal; flour spit-only illegal; `--load` station and event counts unchanged |
| §2 recipes `--exact` ×5 | **pass** | Craft noggin/cowl/keystone/farl; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=187acad9a9f44cb040a77f09b632f41387500f2bc0591da9095cd58be13bd21d`; `--no-time` `final_hash=b44e30d748f5d210247b27c3addfe3e388af3326371f29086ffeda84fa9e04f2` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
