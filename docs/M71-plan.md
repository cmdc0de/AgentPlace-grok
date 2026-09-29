# M71 — Server POST `/query` (mutating SQL), extra recipes

**Status:** implemented  
**Depends on:** M70 complete (`docs/M70-plan.md`, git tag `M70`, commit `093a91f`)  
**Walkthrough:** [`docs/M71-test-plan.md`](M71-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-25 / RF-PG9

## Context

M70 serves GET `/sqlite` and runs ad-hoc SELECT in the page via sql.js (SELECT/WITH only). Catalog after M70 includes peg/band/coping/waffle.

M71 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. POST `/query` is **hash-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not bonus-use every-instance, not dual-station, not arrow ammo.

## Goal

A researcher can:

1. Run `sim-cli --sqlite PATH --sqlite-http 127.0.0.1:9002`, open `web/index.html`, and **POST ad-hoc SQL** to `{metrics-origin}/query` (including **INSERT/UPDATE/DELETE**). Results render in the same output area. M70 sql.js **Run SQL** stays SELECT/WITH on the in-memory copy.
2. **Craft** four new catalog items (**stake, cuff, plinth, crumpet**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`. Do not store HTTP handles on `Simulation`.

### A. POST `/query` (RF-25, hash-neutral)

M51 GET `/metrics` and M70 GET `/sqlite` + sql.js SELECT stay. This slice adds server-side SQL on the same `--sqlite-http` server.

HTTP (CORS `*`):

| Method | Path | Body |
|---|---|---|
| POST | `/query` | JSON `{"sql":"<one statement>"}` |
| OPTIONS | `/query` | CORS preflight |

`Access-Control-Allow-Methods` becomes `GET, POST, OPTIONS` (today GET, OPTIONS). Same `Access-Control-Allow-Origin: *` / `Allow-Headers: *`.

Request:

- UTF-8 JSON object with string field `sql`. Missing/empty `sql` ⇒ `{"error":"..."}`.
- One statement. Cap **64 KiB** for the HTTP request (headers + body). Today the handler reads 2048 bytes — implement must read `Content-Length` (or until `\r\n\r\n` + body).
- `Content-Type: application/json` preferred; parse the body as JSON regardless if it looks like `{...}`.

Execution (rusqlite on the `--sqlite` path, new connection per request, same as GET `/metrics`):

- Trim; if starts with `SELECT` or `WITH` (case-insensitive) ⇒ query: JSON `{"columns":[...],"rows":[[...],...]}` (`rows` may be `[]`).
- Else ⇒ `execute_batch` / execute (INSERT/UPDATE/DELETE/CREATE/… allowed): JSON `{"ok":true,"changes":N}` where `N` is `Connection::changes()`.
- SQL error or SQLITE_BUSY ⇒ HTTP 200 + `{"error":"..."}` (same as GET `/metrics` rusqlite fail). No extra denylist (`ATTACH` etc. are the researcher’s file).

Not: auth, TLS, multiple named endpoints, replacing `/metrics` presets, vendoring sql.js, changing the in-page SELECT/WITH reject.

`web/index.html` (below **Run SQL**):

- Button **Run on server**, locked id `sqlite-sql-server`
- POSTs the textarea to `{metrics-origin}/query`
- Renders query table, `{ok, changes}`, or error text in existing `sqlite-sql-out`
- Does **not** apply the M70 SELECT/WITH reject on this path (mutating is the point)

`cargo test` never drives a browser. Rust tests: POST SELECT returns columns/rows; POST INSERT then SELECT sees the row; OPTIONS `/query` CORS includes POST; GET `/metrics` and GET `/sqlite` still work; HTML contains `sqlite-sql-server`.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `stake` | wood×18 | reuse `wood.glb` |
| `cuff` | fiber×18 | reuse `low_poly_cloth.glb` |
| `plinth` | stone×16 | reuse `stones_and_grass.glb` |
| `crumpet` | food×15 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M71 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Wear | bonus-use every-instance (**RF-22**) |
| Also not | dual-station (**RF-23**); arrow ammo (**RF-24**); interior walls (**RF-21**); Unix sockets (**RF-8**); wasm32 Win/mac (**RF-10**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Server rusqlite is the engine for POST `/query`. sql.js stays SELECT/WITH on the in-memory copy.
4. Mutating SQL is allowed on the server path only. Hash-neutral HTTP + page.
5. Read the full request (64 KiB cap). Do not keep the 2048-byte GET-only read.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M71 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `ec74cad0…` |
| `--no-time` shipped objects | hash `720b5da3…` |
| POST `/query` SELECT | JSON `columns` + `rows`; CORS |
| POST `/query` INSERT then SELECT | new row visible |
| OPTIONS `/query` | CORS; `Allow-Methods` includes POST |
| GET `/metrics` / GET `/sqlite` | M51 / M70 still |
| `web/index.html` | id `sqlite-sql-server`; sql.js 1.11.0 + M70 ids stay |
| no `--sqlite-http` | no bind; hashes unchanged from HTTP |
| Craft stake / cuff / plinth / crumpet | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live browser **not run**. Live collector **not run**.

## PR Plan

### PR 1: POST `/query`

- Body read + JSON; SELECT vs mutating; CORS POST; page button; tests

### PR 2: Recipes

- stake/cuff/plinth/crumpet; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: POST `/query`; page **Run on server**; stake/cuff/plinth/crumpet. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-cli --test sqlite_cli
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M71-test-plan.md`](M71-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `720b5da3…`; default (time on) `ec74cad0…`; Hello v5; format_version 3 write.

## Risks

- **Body size.** Today `handle_metrics_http` reads 2048 bytes. POST needs `Content-Length` / full body up to 64 KiB or tests will flake.
- **SQLITE_BUSY.** Sim writer and POST mutate the same file. Return `{"error":...}`; tests are sequential. Do not put HTTP on `Simulation`.
- **Catalog hash.** Recipes move idle hashes. HTTP remains hash-neutral.
- **No protocol / format bump.** Postcard enums unchanged. Overlay TOML is not ExperimentConfig.
- **Mutating the log** is intentional and does not affect `state_hash`.
