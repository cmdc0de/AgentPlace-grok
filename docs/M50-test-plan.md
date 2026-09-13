# M50 test plan — see each new feature

Walkthrough for [`M50-plan.md`](M50-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (FPS HUD, sqlite columns, extra recipes). Next slice: [`M53-plan.md`](M53-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `inventions`, `survival`, `net`, `sqlite_cli`); unit tests under `-p viewer` use the module path (`fps::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** Status FPS helper: 60 × 16.67 ms ⇒ fps ≈ 60; empty ⇒ 0; filling the ring does **not** change `state_hash`. `--sqlite PATH` writes typed columns (`ticks`, `agent_timing`, `decisions`, `decision_legal`, `events`) with **no** `json` column; `--sqlite` without `--out-dir` still creates the DB. Catalog-off ⇒ club/satchel/pike Craft illegal; no-objects hash `70e5204d…`. Catalog-on ⇒ Craft club (1 wood), satchel (2 fiber + 1 wood), pike (2 wood + 1 stone). Default `sim-cli` 2-tick hash is `35faecbd…` (M50 recipes). Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. Viewer FPS / frametime HUD

```bash
cargo test -p viewer fps::tests::fps_helper_sixty_16ms_is_about_60 -- --exact --nocapture
cargo test -p viewer fps::tests::fps_helper_empty_is_zero -- --exact --nocapture
cargo test -p viewer fps::tests::fps_not_hashed -- --exact --nocapture
```

| Test | Success |
|---|---|
| 60 × 16.67 ms | fps ≈ 60, mean_ms ≈ 16.67 |
| empty | fps 0, mean_ms 0 |
| not hashed | frame ring does not change `state_hash` |

Researcher (native window, **not run** here): Status shows `fps=…  frame_ms=…` under tick wall_ms, including while paused.

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

---

## 2. SQLite run log

```bash
cargo test -p sim-cli sqlite::tests::sqlite_no_json_column -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_ticks_and_agent_timing -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_decisions_and_legal -- --exact --nocapture
cargo test -p sim-cli sqlite::tests::sqlite_events_move_columns -- --exact --nocapture
cargo test -p sim-cli --test sqlite_cli sqlite_cli_without_out_dir_creates_db -- --exact --nocapture
```

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --sqlite /tmp/m50.sqlite3 --quiet
```

| Test | Success |
|---|---|
| no json column | `PRAGMA table_info` has no `json` on ticks/agent_timing/decisions/events |
| ticks / agent_timing | INTEGER `wall_ns` / `select_ns` match the struct |
| decisions | `primary_action` is a name; `decision_legal` ≥1 row |
| events Move | `kind=move`, `from_x` set, unused `item` NULL |
| `--sqlite` no out-dir | DB file created; ticks/decisions rows ≥1 |

---

## 3. Extra recipes + Hello / format / idle

```bash
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_club -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_satchel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_pike -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| catalog-off | Catalog Craft illegal |
| catalog-on club | 1 wood → 1 club |
| catalog-on satchel | 2 fiber + 1 wood → 1 satchel |
| catalog-on pike | 2 wood + 1 stone → 1 pike |
| no objects dir | hash `70e5204d…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=35faecbd…` |

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --lockstep \
  --sqlite /tmp/m50-overnight.sqlite3 --quiet
```

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 38; objects 34; inventions 17) |
| §0 `cargo test -p viewer` | **pass** | 57/57 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 fps `--exact` ×3 | **pass** | 60×16.67 ms ⇒ fps≈60; empty 0; not hashed |
| §2 sqlite `--exact` ×5 + `sim-cli --sqlite` | **pass** | no `json` column; ticks/agent_timing INTEGER; `--sqlite` without `--out-dir` creates DB; hash unchanged |
| §3 recipes / Hello / idle | **pass** | club/satchel/pike Craft; no-objects `70e5204d…`; format 3; Hello v5 |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=35faecbd4484eb841e0a46264ca929fec7a08867ab4acbaaa0acbec56c74e4bb` |
| viewer window / imgui FPS | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
