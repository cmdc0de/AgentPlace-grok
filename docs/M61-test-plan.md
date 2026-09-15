# M61 test plan — see each new feature

Walkthrough for [`M61-plan.md`](M61-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (dawn wear, out-dir disk walk, bench/shawl/cobble/cake). Next slice: [`M63-plan.md`](M63-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `telemetry`, `survival`, `net`, `otlp_cli`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `16804536…`. Default (time on) shipped-objects `3512dde6…`. Dawn + held axe wear `[0]` → `[1]`. 8 dawns consume 1. `--no-time` does not decay. Two axes: most-worn +1. Telemetry off / no out-dir ⇒ bytes None. Temp out-dir file ⇒ `Some(n >= len)`. OTLP JSON has `agentplace.process.out_dir_bytes`. Craft bench/shawl/cobble/cake. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Time-decay wear

```bash
cargo test -p sim-core --test objects dawn_decays_held_axe -- --exact --nocapture
cargo test -p sim-core --test objects eight_dawns_consume_one_axe -- --exact --nocapture
cargo test -p sim-core --test objects no_time_does_not_decay_held_axe -- --exact --nocapture
cargo test -p sim-core --test objects dawn_decays_most_worn_axe_only -- --exact --nocapture
```

| Test | Success |
|---|---|
| dawn | wear `[0]` → `[1]` after 2 ticks (`ticks_per_day = 2`) |
| 8 dawns | one axe consumed |
| `--no-time` | wear stays `[0]` |
| two axes | most-worn +1; other stays 0 |

---

## 2. Out-dir disk walk

```bash
cargo test -p sim-core --test telemetry telemetry_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_out_dir_none_without_path -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_out_dir_walks_files_hash_neutral -- --exact --nocapture
cargo test -p sim-cli --test otlp_cli otlp_endpoint_posts_json_and_keeps_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | same hash; bytes None |
| on, no path | same hash; bytes None |
| on + file | `Some(n >= 10)`; hash unchanged |
| OTLP JSON | POST includes `agentplace.process.out_dir_bytes` |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_bench -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_shawl -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cobble -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cake -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | bench / shawl / cobble / cake from locked inputs |
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
| shipped `--no-time` | `16804536…` |
| default time on | `3512dde6…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-15, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 93; telemetry 5; inventions 19; sleep 25; survival 18) |
| §0 `cargo test -p viewer` | **pass** | 61/61 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 dawn wear `--exact` ×4 | **pass** | `[0]`→`[1]`; 8 dawns consume 1; `--no-time` no decay; most-worn +1 |
| §2 out-dir `--exact` ×4 | **pass** | overlay off identity; no path None; file walk `Some(n>=10)`; OTLP has `out_dir_bytes` |
| §3 recipes `--exact` ×5 | **pass** | Craft bench/shawl/cobble/cake; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=3512dde61887c6feb81decefce29b2d1123d4ca24ac72eaedcd37a102894ccc0`; `--no-time` `final_hash=1680453678d57b0e336439e6f9a4f52af34cec8a7abe8ada9ccf2b5bf72c4fd9` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
