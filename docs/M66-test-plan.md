# M66 test plan — see each new feature

Walkthrough for [`M66-plan.md`](M66-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (per-tick wear, household/invention/downed meshes, post/muffler/lintel/scone). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `155d4492…`. Default (time on) shipped-objects `bcd36485…`. Overlay off: dawn wear identity. `--per-tick-wear`: held/crate +1 per tick; 8 ticks consume 1; `--no-time` still wears; dawn tick wears once not twice; `--load` no extra wear. Household/invention/downed stems resolve; missing downed is sentinel. Craft post/muffler/lintel/scone. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Per-tick wear

```bash
cargo test -p sim-core --test objects overlay_parses_wear -- --exact --nocapture
cargo test -p sim-core --test objects dawn_decays_held_axe -- --exact --nocapture
cargo test -p sim-core --test objects crate_dawn_decays_stored_axe -- --exact --nocapture
cargo test -p sim-core --test objects per_tick_wears_held_axe -- --exact --nocapture
cargo test -p sim-core --test objects eight_per_tick_consume_one_axe -- --exact --nocapture
cargo test -p sim-core --test objects per_tick_wears_crate_axe -- --exact --nocapture
cargo test -p sim-core --test objects no_time_per_tick_still_wears -- --exact --nocapture
cargo test -p sim-core --test objects dawn_plus_per_tick_wears_once_per_tick -- --exact --nocapture
cargo test -p sim-core --test objects load_does_not_extra_wear -- --exact --nocapture
cargo test -p sim-core --test objects wear_overlay_off_idle_hash_ignores_flag -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[wear] per_tick = true` on; omit off |
| overlay off | dawn held/crate identity |
| 1 tick | held `[0]` → `[1]` |
| 8 ticks | consume 1 |
| crate | +1 per tick |
| `--no-time` | still wears |
| dawn + per-tick | wear once per tick; energy refills |
| `--load` | no extra wear |
| overlay on vs off | hashes differ |

---

## 2. Household / invention / downed meshes

```bash
cargo test -p viewer models::tests::household_invention_downed_stems_resolve -- --exact --nocapture
```

| Test | Success |
|---|---|
| stems | household / invention / downed Authored |
| missing downed | Sentinel |

Native window: **not run**.

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_post -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_muffler -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_lintel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_scone -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | post / muffler / lintel / scone |
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
| shipped `--no-time` | `155d4492…` |
| default time on | `bcd36485…` |
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
| §1 wear `--exact` ×10 | **pass** | overlay parse; dawn identity; per-tick held/crate/8/no-time/dawn-once/load/hash |
| §2 meshes `--exact` ×1 | **pass** | household/invention/downed Authored; missing sentinel |
| §3 recipes `--exact` ×5 | **pass** | Craft post/muffler/lintel/scone; catalog-off |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=bcd3648573d414081e320245a5e768f86d81744467562a562e4f5d4a06acdeea`; `--no-time` `final_hash=155d449267a13643920e185a46e5b3e28604276cb160ec5c7542868f32f2e099` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
