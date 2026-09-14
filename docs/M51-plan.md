# M51 — Sqlite metrics page, weapon combat bonuses, extra recipes

**Status:** implemented  
**Depends on:** M50 complete (`docs/M50-plan.md`, git tag `M50`, commit `35776f1`)  
**Walkthrough:** [`docs/M51-test-plan.md`](M51-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-15 leftover queries, PG-9 recipes); After-M50 weapon combat bonuses

## Context

M50 writes unrolled sqlite columns and ships catalog weapons (club/pike/sling/bow) with **no** Attack damage bonus. `web/index.html` Metrics is Tick.inspector only; the page cannot see the sqlite file. Spear is a hunt tool, not melee.

M51 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Sqlite HTTP + page fetch are **hash-neutral**. `[sim] attack_bonus` and new catalog files **do** change the shipped-objects idle hash.

## Goal

A researcher can:

1. Run `sim-cli --sqlite PATH --sqlite-http 127.0.0.1:9002` and open `web/index.html`, then **Refresh metrics** to see preset query results (median world tick ns, median agent pipeline ns, events by kind, crafts by item) from that sqlite file. CORS allows the local page to fetch.
2. Hold a catalog weapon (**club / pike / sling / bow / knife**) and deal **extra Attack damage** (`[sim] attack_bonus`). No weapon / bonus 0 / catalog-off ⇒ today’s STR-only damage. Attack stays adjacent (no range).
3. Turn on `--catalog` / objects dir and **Craft** **tent, millstone, stew, ladder**. Catalog-off ⇒ idle no-objects hash unchanged.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

No-objects 2-tick hash stays **`70e5204d…`**. Default CLI with shipped objects is **`04069600…`**.

## In scope

No PROTOCOL bump. No new `SimEventKind`. No new ControlVerb. Do not store HTTP handles on `Simulation`.

### A. Website sqlite queries (PG-15 leftover)

Browser cannot read sim-cli’s filesystem. The page **does not** embed sql.js. sim-cli runs the locked SELECTs and serves JSON.

CLI (only with `--sqlite`):

```bash
sim-cli --sqlite /tmp/run.sqlite3 --sqlite-http 127.0.0.1:9002
```

Omit `--sqlite-http` ⇒ today (DB only). `--sqlite-http` without `--sqlite` is an error.

HTTP (hash-neutral, not AGTN, not PROTOCOL):

| Method | Path | Body |
|---|---|---|
| GET | `/metrics` | JSON of the four preset queries below |
| OPTIONS | `/metrics` | CORS preflight |

`Access-Control-Allow-Origin: *`. Bind the given host:port (`127.0.0.1:0` allowed in tests).

Locked SQL (same strings in sim-cli and tests):

```sql
SELECT wall_ns FROM ticks ORDER BY wall_ns LIMIT 1 OFFSET (SELECT COUNT(*) FROM ticks) / 2;

SELECT (perceive_ns+retrieve_ns+select_ns+execute_ns+remember_ns) AS agent_ns
FROM agent_timing ORDER BY 1 LIMIT 1 OFFSET (SELECT COUNT(*) FROM agent_timing) / 2;

SELECT kind, COUNT(*) AS n FROM events GROUP BY kind ORDER BY n DESC, kind;

SELECT item, COUNT(*) AS n FROM events WHERE kind = 'craft' GROUP BY item ORDER BY n DESC, item;
```

JSON shape (lock):

```json
{
  "median_wall_ns": 12345,
  "median_agent_ns": 678,
  "events_by_kind": [{"kind": "wait", "n": 10}],
  "crafts": [{"item": "club", "n": 1}]
}
```

Empty tables ⇒ JSON `null` for medians, `[]` for lists.

`web/index.html` Metrics: URL input default `http://127.0.0.1:9002/metrics`, **Refresh** button, render the four blocks as text/tables. No new JS chart library. `cargo test` does **not** drive a browser; hit `/metrics` from Rust on a temp DB.

Headless `--sqlite` without `--listen` still serves HTTP until the process exits. With `--listen`, HTTP stays up with the websocket.

### B. Weapon combat bonuses

Object TOML `[sim] attack_bonus` (u32 millipoints, omit = 0). Hashed with other `[sim]` when catalog is loaded.

On Attack, after `sheet.attack_damage()`, add **max** `attack_bonus` among items the attacker **holds** (pockets or pack). Do not sum. Miss still 0 damage.

Set on implement (existing M50 weapon files + knife):

| slug | attack_bonus |
|---|---|
| `club` | 300 |
| `knife` | 200 |
| `sling` | 400 |
| `bow` | 600 |
| `pike` | 800 |

Spear stays hunt-tool only (no melee bonus this slice). Attack **range stays adjacent**. Catalog-off / empty catalog / no matching item ⇒ bonus 0, **same damage as today**. `--load` restores inventory; do not persist derived damage.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Distinct inputs. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `tent` | fiber×3 + wood×2 | reuse `low_poly_cloth.glb` |
| `millstone` | stone×3 | reuse `stones_and_grass.glb` |
| `stew` | food×2 | reuse `plants_ready.glb` |
| `ladder` | wood×3 | reuse `wood.glb` |

Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock (default on), world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| After M56 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL in the page; durability / workstations; OTLP protobuf/gRPC; process CPU/disk; viewer-frame OTLP |
| Not M51 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; sqlite as checkpoint store |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Metrics are HTTP, not WS postcard.
2. **format_version still writes 3 / reads v2+v3.**
3. Weapon bonuses are `[sim] attack_bonus`; **max** held, not sum. Spear hunt-only.
4. New recipes + attack_bonus on existing files change **shipped-objects** idle hash; no-objects hash unchanged.
5. Do not change shipping `coop.toml`. No TLS. CI never needs the network.
6. sql.js / ad-hoc SQL are After M56.

## Tests (M51 acceptance bar)

| Test | Asserts |
|---|---|
| no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | shipped-objects hash `04069600…`; no-objects `70e5204d…` |
| `--sqlite-http` without `--sqlite` | process error |
| `/metrics` empty DB | medians `null`, lists `[]` |
| `/metrics` fixture | median_wall_ns is the middle `ticks.wall_ns`; events_by_kind counts match |
| CORS | `Access-Control-Allow-Origin` is `*` |
| no `--sqlite-http` | no bind; idle hashes unchanged |
| attack no weapon | damage = `sheet.attack_damage()` |
| attack hold pike | damage = base + 800; miss still 0 |
| two weapons | max bonus, not sum |
| catalog-off | bonus 0; tent Craft illegal |
| catalog-on | Craft tent from 3 fiber + 2 wood; stew from 2 food |
| Hello v5 / format 3 | unchanged |

GPU window and live browser fetch are **not** required (`/metrics` from Rust is).

## PR Plan

### PR 1: sqlite-http + page

- **Files:** `--sqlite-http`; GET `/metrics`; CORS; `web/index.html` Metrics URL + Refresh; rusqlite query tests

### PR 2: attack_bonus

- **Files:** `SimDef.attack_bonus` / catalog field; Attack uses max held bonus; knife/club/sling/bow/pike TOML; hit/miss tests

### PR 3: Extra recipes

- **Files:** `configs/objects/{tent,millstone,stew,ladder}.toml`; Craft tests; document new idle hash

## Config / CLI

No shipping experiment TOML change. No PROTOCOL bump.

On implement: `--sqlite-http HOST:PORT`; `attack_bonus` on five weapon TOMLs; four recipe files. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-cli --test sqlite_cli
cargo test -p sim-core --test objects
cargo test -p sim-core --test combat
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

## Verification

Walkthrough: [`docs/M51-test-plan.md`](M51-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: no-objects hash `70e5204d…`; shipped-objects idle hash `04069600…`; Hello v5; format_version 3 write.

## Risks

- HTTP sidecar must not enter `state_hash` or AGTN.
- `--sqlite-http` default **off**; tests bind `127.0.0.1:0`.
- Shipped-objects idle hash **will** change (attack_bonus on existing files + four new `[sim]` files). Document it.
- Weapon stack is **max**, not sum. Spear hunt unchanged.
- `--load` restores inventory; do not persist derived damage.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
