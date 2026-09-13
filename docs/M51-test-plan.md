# M51 test plan — see each new feature

Walkthrough for [`M51-plan.md`](M51-plan.md). Automated tests prove the slice; the `sim-cli` / viewer / page steps below are what you **read** (sqlite `/metrics`, attack_bonus, extra recipes). Next slice: [`M53-plan.md`](M53-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `combat`, `survival`, `net`, `sqlite_cli`); unit tests under `-p sim-cli` use `sqlite::tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--sqlite-http` without `--sqlite` errors. `GET /metrics` on an empty DB returns null medians and empty lists; CORS `Access-Control-Allow-Origin: *`. Fixture median_wall_ns is the middle `ticks.wall_ns`. Attack with no weapon is `ATTACK_DAMAGE`; holding pike is base+800; club+pike uses max not sum. Catalog-off ⇒ tent Craft illegal; no-objects hash `70e5204d…`. Catalog-on ⇒ Craft tent (3 fiber + 2 wood) and stew (2 food). Default `sim-cli` 2-tick hash is `04069600…`. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Sqlite metrics HTTP + page

```bash
cargo test -p sim-cli sqlite::tests::metrics_empty_db_null_medians -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::metrics_fixture_median_and_kinds -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::metrics_http_cors_and_json -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli sqlite_http_requires_sqlite -- --exact --nocapture
```

Researcher (page fetch **not run** here):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sqlite /tmp/m51.sqlite3 --sqlite-http 127.0.0.1:9002 --llm mock --quiet
# open web/index.html, Refresh sqlite metrics URL http://127.0.0.1:9002/metrics
```

| Test | Success |
|---|---|
| empty DB | medians null, lists `[]` |
| fixture | median_wall_ns = 20 (middle of 10,20,30) |
| CORS | response has `Access-Control-Allow-Origin: *` |
| no `--sqlite` | process error `--sqlite-http requires --sqlite` |

---

## 2. Weapon attack_bonus

```bash
cargo test -p sim-core --test combat attack_no_weapon_is_base_damage -- --exact --nocapture
cargo test -p sim-core --test combat attack_hold_pike_adds_800 -- --exact --nocapture
cargo test -p sim-core --test combat attack_two_weapons_uses_max_not_sum -- --exact --nocapture
cargo test -p sim-core --test combat attack_catalog_off_weapon_no_bonus -- --exact --nocapture
```

| Test | Success |
|---|---|
| no weapon | damage = `ATTACK_DAMAGE` |
| hold pike | base + 800 |
| club + pike | max 800, not 1100 |
| catalog-off | bonus 0 |

---

## 3. Extra recipes + Hello / format / idle

```bash
cargo test -p sim-core --test objects catalog_on_craft_tent -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_stew -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| catalog-on tent | 3 fiber + 2 wood → 1 tent |
| catalog-on stew | 2 food → 1 stew |
| catalog-off | Catalog Craft illegal |
| no objects dir | hash `70e5204d…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=04069600…` |

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --sqlite /tmp/m51.sqlite3 --sqlite-http 127.0.0.1:9002 --quiet
```

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

Live Ollama / overnight / imgui / browser Refresh: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window / live page fetch not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 38; objects 36; combat 19) |
| §0 `cargo test -p viewer` | **pass** | 57/57 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 metrics `--exact` ×4 | **pass** | empty nulls; median 20; CORS `*`; CLI requires `--sqlite` |
| §2 attack `--exact` ×4 | **pass** | base; pike +800; max not sum; catalog-off 0 |
| §3 recipes / Hello / idle | **pass** | tent/stew Craft; no-objects `70e5204d…`; format 3; Hello v5 |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=0406960048c1ada1c4910f7bd81bba89ed61a75ec85c2f43abc0d5491dfdff44` |
| viewer / browser metrics Refresh | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
