# M56 test plan — see each new feature

Walkthrough for [`M56-plan.md`](M56-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (axe gather bonus, Draco decode, fence/mat/snare/spit). Next slice: [`M61-plan.md`](M61-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `survival`, `net`); unit tests under `-p viewer` use the module path (`models::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `6d2df92b…`. Default (time on) shipped-objects `34f16591…`. Catalog-off Gather 0 / basket +15. Axe Gather bonus 25; basket+axe 25 not 40. Stone gather bonus 0 with axe. `GltfDracoDecoderPlugin` registered. Craft fence/mat/snare/spit. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Axe gather bonus

```bash
cargo test -p sim-core --test objects catalog_off_gather_basket_identity -- --exact --nocapture
cargo test -p sim-core --test objects axe_gather_bonus_25_vs_zero_basket_max -- --exact --nocapture
cargo test -p sim-core --test objects stone_gather_bonus_stays_zero_with_axe -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | Gather 0 without basket; +15 with basket |
| axe | bonus 25; basket+axe 25 not 40 |
| stone | bonus 0 with axe |

---

## 2. Draco plugin

```bash
cargo test -p viewer models::tests::draco_decoder_plugin_registers -- --exact --nocapture
```

| Test | Success |
|---|---|
| plugin | `GltfDracoDecoderPlugin` registered; no GPU window |

Researcher (imgui **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_fence -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_mat -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_snare -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_spit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | fence / mat / snare / spit from locked inputs |
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
| shipped `--no-time` | `6d2df92b…` |
| default time on | `34f16591…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 58/58) |
| §0 `cargo test -p viewer` | **pass** | 59/59 including `models::tests::draco_decoder_plugin_registers` |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 gather `--exact` ×3 | **pass** | catalog-off 0 / basket +15; axe 25; basket+axe 25 not 40; stone 0 with axe |
| §2 Draco `--exact` | **pass** | `GltfDracoDecoderPlugin` registered |
| §3 recipes `--exact` ×5 | **pass** | fence/mat/snare/spit Craft; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=34f165913c1abb8efc4ed82988815282d14c536002944e3eed51ef3dcb1a7124`; `--no-time` `final_hash=6d2df92b8eb0e599155cb4c5e820a4b6536d78f74ac5dfde2e4f74b84ee2a207` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
