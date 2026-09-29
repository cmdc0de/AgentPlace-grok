# M71 test plan — see each new feature

Walkthrough for [`M71-plan.md`](M71-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (POST `/query`, mutating SQL, stake/cuff/plinth/crumpet). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `sqlite_cli`, `net`); unit tests under `-p sim-cli` use `sqlite::tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `720b5da3…`. Default (time on) shipped-objects `ec74cad0…`. POST `/query` SELECT returns JSON columns/rows. POST INSERT then SELECT sees the row. OPTIONS `/query` CORS includes POST. GET `/metrics` and GET `/sqlite` still work. `web/index.html` has `sqlite-sql-server` plus M70 sql.js ids. Craft stake/cuff/plinth/crumpet. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. POST `/query`

```bash
cargo test -p sim-cli sqlite::tests::sqlite_http_post_query_select -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_http_post_query_insert -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_http_options_query_cors -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_http_get_sqlite_opens -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::metrics_http_cors_and_json -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli sqlite_http_requires_sqlite -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli browser_page_has_sqljs_adhoc_ui -- --exact --nocapture
```

| Test | Success |
|---|---|
| POST `/query` SELECT | JSON `columns` + `rows`; CORS |
| POST INSERT then SELECT | new row visible |
| OPTIONS `/query` | CORS; Allow-Methods includes POST |
| GET `/sqlite` | sqlite header; rusqlite `events` |
| GET `/metrics` | M51 JSON still |
| `--sqlite-http` without `--sqlite` | process error |
| `web/index.html` | `sqlite-sql-server` + M70 ids + sql.js 1.11.0 |

Researcher (not run):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet \
  --sqlite /tmp/m71.sqlite3 --sqlite-http 127.0.0.1:9002
# open web/index.html → Run on server
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_stake -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cuff -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_plinth -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_crumpet -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | stake / cuff / plinth / crumpet |
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
| shipped `--no-time` | `720b5da3…` |
| default time on | `ec74cad0…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui / live browser POST `/query`: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-28, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window and live sql.js / POST `/query` in a browser **not** driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 154/154 |
| §0 `cargo test -p viewer` | **pass** | 64/64 |
| §0 `cargo test -p sim-cli --test sqlite_cli` | **pass** | 3/3 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 (one retry of `connect_log_tail_hash_neutral` after Connection reset) |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 POST `/query` `--exact` ×7 | **pass** | SELECT columns/rows; INSERT then SELECT; OPTIONS POST CORS; GET `/sqlite`; GET `/metrics`; requires `--sqlite`; `sqlite-sql-server` + sql.js 1.11.0 |
| §2 recipes `--exact` ×5 | **pass** | Craft stake/cuff/plinth/crumpet; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=ec74cad0c79ec3602f290650bdc186f462acbdadb4edf99fc025ea13c99a0b99`; `--no-time` `final_hash=720b5da35bd25caa8046059c1804d9355c107b16b1b124551b9d94bd826fe371` |
| viewer window / live browser | **not run** | no display / no network |
| live Spark / overnight | **not run** | not asked |
