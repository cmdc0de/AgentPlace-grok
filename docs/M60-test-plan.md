# M60 test plan — see each new feature

Walkthrough for [`M60-plan.md`](M60-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (Transfer wear, CPU percent, stool/sash/brick/biscuit). Next slice: [`M63-plan.md`](M63-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `telemetry`, `survival`, `net`, `otlp_cli`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `171a26d2…`. Default (time on) shipped-objects `aacd867d…`. Transfer `[3,0]` + 1 ⇒ giver `[3]`, receiver `[0]`. Single worn `[7]` moves with the item. Store still drops freshest. `/give` mints fresh. Telemetry off identity; Linux after 2 ticks CPU percent `Some(0..=100)`. OTLP JSON has `agentplace.process.cpu_percent`. Craft stool/sash/brick/biscuit. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Transfer wear

```bash
cargo test -p sim-core --test objects transfer_moves_freshest_wear_slot -- --exact --nocapture
cargo test -p sim-core --test objects transfer_moves_single_worn_slot -- --exact --nocapture
cargo test -p sim-core --test objects store_still_drops_freshest_wear -- --exact --nocapture
cargo test -p sim-core --test objects give_item_mints_fresh_wear -- --exact --nocapture
```

| Test | Success |
|---|---|
| Transfer `[3,0]` | giver keeps `[3]`; receiver `[0]` |
| Transfer `[7]` | receiver `[7]`; giver empty |
| Store | still drops freshest |
| `/give` mint | no wear map on receiver |

---

## 2. CPU percent

```bash
cargo test -p sim-core --test telemetry telemetry_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_on_records_and_same_hash -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_cpu_percent_none_until_second_tick -- --exact --nocapture
cargo test -p sim-cli --test otlp_cli otlp_endpoint_posts_json_and_keeps_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | same hash; percent None |
| overlay on | same hash as off; Linux percent `Some(0..=100)` |
| first tick | percent None until the second sample |
| OTLP JSON | POST includes `agentplace.process.cpu_percent` |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_stool -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_sash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_brick -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_biscuit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | stool / sash / brick / biscuit from locked inputs |
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
| shipped `--no-time` | `171a26d2…` |
| default time on | `aacd867d…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-15, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 85; telemetry 3; inventions 19; sleep 25; survival 18) |
| §0 `cargo test -p viewer` | **pass** | 61/61 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 Transfer `--exact` ×4 | **pass** | `[3,0]` → giver `[3]` receiver `[0]`; `[7]` moves; Store drops freshest; `/give` mints fresh |
| §2 CPU percent `--exact` ×4 | **pass** | overlay off identity; on same hash; first tick None; OTLP has `cpu_percent` |
| §3 recipes `--exact` ×5 | **pass** | Craft stool/sash/brick/biscuit; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=aacd867dfaafe7a055f194027b0849e286edc5fea24761c889a0948c14592665`; `--no-time` `final_hash=171a26d292fbbad8d62f54c44f059bbc595c758d70473beea51f0702945fdb57` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
