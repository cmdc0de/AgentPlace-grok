# M70 test plan — see each new feature

Walkthrough for [`M70-plan.md`](M70-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (GET `/sqlite`, sql.js ad-hoc SELECT, peg/band/coping/waffle). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `sqlite_cli`, `net`); unit tests under `-p sim-cli` use `sqlite::tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `3aa327eb…`. Default (time on) shipped-objects `8b76f66e…`. GET `/sqlite` returns a rusqlite-openable DB with `events`. OPTIONS `/sqlite` CORS. GET `/metrics` presets still work. `web/index.html` has locked ids and sql.js `@1.11.0`. Craft peg/band/coping/waffle. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test sqlite_cli
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. sql.js / GET `/sqlite`

```bash
cargo test -p sim-cli sqlite::tests::sqlite_http_get_sqlite_opens -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_http_options_sqlite_cors -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::metrics_http_cors_and_json -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli sqlite_http_requires_sqlite -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli browser_page_has_sqljs_adhoc_ui -- --exact --nocapture
```

| Test | Success |
|---|---|
| GET `/sqlite` | sqlite header; rusqlite `events` |
| OPTIONS `/sqlite` | CORS |
| GET `/metrics` | M51 JSON still |
| `--sqlite-http` without `--sqlite` | process error |
| `web/index.html` | locked ids + sql.js 1.11.0 |

Researcher (not run):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet \
  --sqlite /tmp/m70.sqlite3 --sqlite-http 127.0.0.1:9002
# open web/index.html → Fetch DB → Run SQL
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_peg -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_band -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_coping -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_waffle -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | peg / band / coping / waffle |
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
| shipped `--no-time` | `3aa327eb…` |
| default time on | `8b76f66e…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui / live browser sql.js: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-27, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window and sql.js CDN **not** driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 150/150 |
| §0 `cargo test -p viewer` | **pass** | 64/64 |
| §0 `cargo test -p sim-cli --test sqlite_cli` | **pass** | 3/3 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 sqlite `--exact` ×5 | **pass** | GET `/sqlite` rusqlite `events`; OPTIONS CORS; GET `/metrics` JSON; `--sqlite-http` requires `--sqlite`; `web/index.html` ids + sql.js 1.11.0 |
| §2 recipes `--exact` ×5 | **pass** | Craft peg/band/coping/waffle; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=8b76f66ef0a0118db35fc1cea5d3828899ebcbea10ba971a40508057df1a6022`; `--no-time` `final_hash=3aa327eb0b69b3931ec91c5e0a171126959041b249f8b2569415c3d823b226b2` |
| viewer window / sql.js CDN | **not run** | no display / no network |
| live Spark / overnight | **not run** | not asked |
