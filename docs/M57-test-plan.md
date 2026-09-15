# M57 test plan — see each new feature

Walkthrough for [`M57-plan.md`](M57-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (hammer stone-gather, process CPU/disk, barrel/cloak/pot/lantern). Next slice: [`M63-plan.md`](M63-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `survival`, `telemetry`, `otlp_cli`, `net`); unit tests under `-p viewer` use the module path (`models::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `e2848016…`. Default (time on) shipped-objects `35c79482…`. Catalog-off stone Gather 0. Hammer stone Gather bonus 25; two tools max not sum. Vegetation Gather with hammer unchanged. Stone gather with axe still 0. Telemetry off identity; on Linux CPU/disk `Some`. OTLP JSON includes the four new gauges. Craft barrel/cloak/pot/lantern. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Hammer stone-gather bonus

```bash
cargo test -p sim-core --test objects catalog_off_stone_gather_identity -- --exact --nocapture
cargo test -p sim-core --test objects hammer_stone_gather_bonus_25_max_not_sum -- --exact --nocapture
cargo test -p sim-core --test objects vegetation_gather_with_hammer_unchanged -- --exact --nocapture
cargo test -p sim-core --test objects stone_gather_bonus_stays_zero_with_axe -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | stone Gather bonus 0 |
| hammer | bonus 25; two tools max not sum |
| vegetation | hammer does not add; basket +15; axe 25 |
| axe | stone bonus still 0 |

---

## 2. Process CPU/disk telemetry

```bash
cargo test -p sim-core --test telemetry telemetry_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_on_records_and_same_hash -- --exact --nocapture
cargo test -p sim-cli --test otlp_cli otlp_endpoint_posts_json_and_keeps_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | same hash as no flag |
| overlay on | hash unchanged vs off; on Linux last CPU/disk `Some` |
| OTLP JSON | POST includes `cpu_user_ns` / `cpu_system_ns` / `disk_read_bytes` / `disk_write_bytes` plus `rss_bytes` + `wall_ns` |

Researcher (live collector **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --telemetry --otlp-endpoint http://127.0.0.1:4318 --quiet
```

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_barrel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cloak -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_pot -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_lantern -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | barrel / cloak / pot / lantern from locked inputs |
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
| shipped `--no-time` | `e2848016…` |
| default time on | `35c79482…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 65/65; telemetry 2/2) |
| §0 `cargo test -p viewer` | **pass** | 59/59 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 stone-gather `--exact` ×4 | **pass** | catalog-off 0; hammer 25; two tools max; vegetation unchanged; axe stone 0 |
| §2 telemetry `--exact` ×3 | **pass** | overlay-off identity; on Linux CPU/disk `Some`; OTLP JSON has four new gauges |
| §3 recipes `--exact` ×5 | **pass** | barrel/cloak/pot/lantern Craft; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=35c794827f5e4744ff9dffd4b49dab04b97e85d5d76fef4fe401e04ba847e44d`; `--no-time` `final_hash=e2848016bf0e3c82f7b14cd69f31b25550a066fcebddc541d0b9051be91db0a0` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
