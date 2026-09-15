# M62 test plan — see each new feature

Walkthrough for [`M62-plan.md`](M62-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (spit stations, Store wear, rack/wrap/tile/pie). Next slice: later work at the bottom of [`M62-plan.md`](M62-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `storage`, `survival`, `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `3170f273…`. Default (time on) shipped-objects `7f2d52db…`. Bread/stew illegal without a placed spit; pocket spit does not count. Flour still needs placed millstone. Store `[3,0]` qty 1 ⇒ crate `[0]`, agent `[3]`; Retrieve round-trips to `[3,0]`. Craft rack/wrap/tile/pie. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Bread/stew stations

```bash
cargo test -p sim-core --test objects bread_needs_placed_spit -- --exact --nocapture
cargo test -p sim-core --test objects stew_needs_placed_spit -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bread -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_stew -- --exact --nocapture
cargo test -p sim-core --test objects flour_needs_placed_millstone -- --exact --nocapture
```

| Test | Success |
|---|---|
| pocket spit | bread Craft illegal |
| no spit | stew illegal until Place |
| + placed spit | bread/stew Craft |
| flour | still millstone |

---

## 2. Store wear

```bash
cargo test -p sim-core --test objects store_then_retrieve_roundtrips_wear -- --exact --nocapture
cargo test -p sim-core --test objects give_item_mints_fresh_wear -- --exact --nocapture
```

| Test | Success |
|---|---|
| Store/Retrieve | `[3,0]` round-trips |
| `/give` mint | still fresh |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_rack -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_wrap -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_tile -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_pie -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | rack / wrap / tile / pie from locked inputs |
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
| shipped `--no-time` | `3170f273…` |
| default time on | `7f2d52db…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-15, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 100; storage 25; telemetry 5; survival 18) |
| §0 `cargo test -p viewer` | **pass** | 61/61 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 stations `--exact` ×5 | **pass** | pocket spit illegal; stew needs Place; bread/stew Craft with spit; flour millstone |
| §2 Store wear `--exact` ×2 | **pass** | `[3,0]` Store/Retrieve round-trip; `/give` mint fresh |
| §3 recipes `--exact` ×5 | **pass** | Craft rack/wrap/tile/pie; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=7f2d52db8a0b3b74f46a9e75ffa3d6c167d21bcd2f24421efe43aa6c75ac4e24`; `--no-time` `final_hash=3170f273fe922c5f52d0b6fdd02a3a3918bd7ea3d04185d2f88fa42ed7dbac69` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
