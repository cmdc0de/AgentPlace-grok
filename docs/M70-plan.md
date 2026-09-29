# M70 — Browser sql.js ad-hoc SQL, extra recipes

**Status:** implemented  
**Depends on:** M69 complete (`docs/M69-plan.md`, git tag `M69`, commit `bd33e20`)  
**Walkthrough:** [`docs/M70-test-plan.md`](M70-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-7 / RF-PG9

## Context

M51 GET `/metrics` serves four preset sqlite queries; the page does not embed sql.js. Catalog after M69 includes joist/veil/ashlar/loaf.

M70 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. sql.js / GET `/sqlite` are **hash-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not mutating SQL, not server-side POST `/query`, not vendoring wasm.

## Goal

A researcher can:

1. Run `sim-cli --sqlite PATH --sqlite-http 127.0.0.1:9002`, open `web/index.html`, **download or pick** the sqlite file, and **run ad-hoc SELECT** in the page via **sql.js** (not the four M51 presets only). Results render as a table. Read-only (SELECT / WITH). No INSERT/UPDATE.
2. **Craft** four new catalog items (**peg, band, coping, waffle**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`. Do not store HTTP handles on `Simulation`.

### A. sql.js / ad-hoc SQL (RF-7, hash-neutral)

M51 GET `/metrics` presets stay. This slice adds:

HTTP (same `--sqlite-http` server, CORS `*`):

| Method | Path | Body |
|---|---|---|
| GET | `/sqlite` | the sqlite file bytes (`application/octet-stream`) |
| OPTIONS | `/sqlite` | CORS preflight |

`web/index.html` Metrics (below Refresh):

- File input to load a `.sqlite3` from disk
- Button to **Fetch DB** from `{metrics-origin}/sqlite`
- SQL textarea (default `SELECT kind, COUNT(*) AS n FROM events GROUP BY kind ORDER BY n DESC`)
- **Run SQL** — sql.js in the page (CDN lock: `https://cdn.jsdelivr.net/npm/sql.js@1.11.0/dist/sql-wasm.js`; wasm from the same `@1.11.0/dist/` tree)
- Result table or error text
- Reject statements that do not start with `SELECT` or `WITH` (case-insensitive, trim)

`cargo test` never fetches the CDN and does **not** drive a browser. Rust tests: GET `/sqlite` opens with rusqlite; HTML contains locked element ids (`sqlite-file`, `sqlite-fetch`, `sqlite-sql`, `sqlite-sql-run`, `sqlite-sql-out`) and the sql.js script src.

Not: POST `/query` on the server (sql.js is the engine). Not mutating SQL. Not replacing `/metrics` presets.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `peg` | wood×17 | reuse `wood.glb` |
| `band` | fiber×17 | reuse `low_poly_cloth.glb` |
| `coping` | stone×15 | reuse `stones_and_grass.glb` |
| `waffle` | food×14 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M70 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| SQL | mutating queries; server-side POST `/query` (**RF-25**); vendoring sql.js wasm in-repo |
| Also not | bonus-use every-instance (**RF-22**); dual-station (**RF-23**); arrow ammo (**RF-24**); interior walls (**RF-21**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Ad-hoc SQL runs **in the browser** (sql.js). sim-cli only serves the DB bytes.
4. Read-only SELECT/WITH. Hash-neutral HTTP + page.
5. CDN sql.js; tests do not hit the network.
6. Do not change shipping `coop.toml`. No TLS.

## Tests (M70 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `8b76f66e…` |
| `--no-time` shipped objects | hash `3aa327eb…` |
| GET `/sqlite` | sqlite header; rusqlite open; `events` table exists after a short run |
| OPTIONS `/sqlite` | CORS |
| GET `/metrics` | M51 presets still work |
| `web/index.html` | locked ids + sql.js `@1.11.0` script src |
| no `--sqlite-http` | no bind; hashes unchanged from HTTP |
| Craft peg / band / coping / waffle | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live browser **not run**. Live collector **not run**.

## PR Plan

### PR 1: sql.js + GET `/sqlite`

- `/sqlite` on sqlite-http; page file/fetch + SELECT; HTML/CLI tests

### PR 2: Recipes

- peg/band/coping/waffle; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: GET `/sqlite`; page sql.js UI; peg/band/coping/waffle. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-cli --test sqlite_cli
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M70-test-plan.md`](M70-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `3aa327eb…`; default (time on) `8b76f66e…`; Hello v5; format_version 3 write.

## Risks

- **CDN.** Tests must not fetch sql.js. Page documents the URL; missing CDN ⇒ error text, not a sim crash.
- **Read-only.** Reject non-SELECT/WITH in the page. GET `/sqlite` is the whole file (researcher already has `--sqlite PATH`).
- **Catalog hash.** Recipes move idle hashes. HTTP remains hash-neutral.
- **No protocol / format bump.**
