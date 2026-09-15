# M58 test plan — see each new feature

Walkthrough for [`M58-plan.md`](M58-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (tool durability, millstone station, viewer-frame OTLP, flour/cart/bellows/table). Next slice: later work at the bottom of [`M58-plan.md`](M58-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `sleep`, `survival`, `telemetry`, `otlp_cli`, `net`); unit tests under `-p viewer` use the module path (`otlp::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `b4e1eac7…`. Default (time on) shipped-objects `aad2121f…`. Catalog-off millstone Place illegal. Axe breaks after 8 successful Gathers. Millstone Place/Pickup 1×1; overlap with tent illegal. Flour Craft needs a placed millstone (pocket millstone not enough). Craft cart/bellows/table. Viewer JSON contains `agentplace.viewer.frame_ns`. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Durability + millstone station

```bash
cargo test -p sim-core --test objects catalog_off_millstone_place_illegal -- --exact --nocapture
cargo test -p sim-core --test objects axe_breaks_after_8_successful_gathers -- --exact --nocapture
cargo test -p sim-core --test objects load_restores_tool_uses -- --exact --nocapture
cargo test -p sim-core --test sleep millstone_place_then_pickup -- --exact --nocapture
cargo test -p sim-core --test sleep millstone_overlap_with_tent_illegal -- --exact --nocapture
cargo test -p sim-core --test sleep load_restores_work_place -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | millstone Place illegal |
| axe | 8 successful veg Gathers consume 1 axe; 7 do not |
| load wear | `tool_uses` restored, no extra break |
| millstone | Place 1×1; Pickup returns it |
| overlap | tent cell rejects millstone |
| load station | `work_places` restored |

---

## 2. Viewer-frame OTLP

```bash
cargo test -p viewer otlp::tests::viewer_frame_json_has_locked_gauge -- --exact --nocapture
cargo test -p sim-cli --test otlp_cli otlp_endpoint_posts_json_and_keeps_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| JSON | contains `agentplace.viewer.frame_ns`; no GPU |
| sim-cli OTLP | still posts tick + process gauges; hash unchanged |

Researcher (imgui / live collector **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects \
  --otlp-endpoint http://127.0.0.1:4318
```

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects flour_needs_placed_millstone -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cart -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bellows -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_table -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| flour | illegal with pocket millstone; legal after Place |
| Craft | cart / bellows / table from locked inputs |
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
| shipped `--no-time` | `b4e1eac7…` |
| default time on | `aad2121f…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 72; sleep 25; telemetry 2) |
| §0 `cargo test -p viewer` | **pass** | 61/61 including `otlp::tests::viewer_frame_json_has_locked_gauge` |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 durability `--exact` ×6 | **pass** | catalog-off Place illegal; axe breaks at 8; load restores uses; millstone Place/Pickup; tent overlap; load work_places |
| §2 OTLP `--exact` ×2 | **pass** | viewer JSON has `agentplace.viewer.frame_ns`; sim-cli OTLP hash-neutral |
| §3 recipes `--exact` ×5 | **pass** | flour needs placed millstone; cart/bellows/table Craft; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=aad2121fe9cd21eae365114d05bf50d978d07f131a95a8d199471ae50e565bec`; `--no-time` `final_hash=b4e1eac7b6dd3b9f368f1d5268c61c789f9fb582e183dc1acf47743456a52475` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
