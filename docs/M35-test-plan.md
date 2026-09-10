# M35 test plan — see each new feature

Walkthrough for [`M35-plan.md`](M35-plan.md). Automated tests prove the slice; the `sim-cli` / viewer / browser steps below are what you **read** (inventions, postcard WS page). Next slice: [`M42-plan.md`](M42-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test inventions`; unit tests under `--lib` need the module path (`protocol::tests::…`).

**Success for the slice:** overlay off / mock never Invents ⇒ idle hash unchanged. Invent grants inventor 1.2× food + influence immediately; society after `share_delay_ticks`; no stack. Browser page is postcard **v5** on `ws://`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Inventions

```bash
cargo test -p sim-core --test inventions overlay_parses_inventions -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions mock_inventions_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions execute_invent_inventor_then_society -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_load_does_not_regrant_influence -- --exact --nocapture
cargo test -p sim-core --test inventions invent_chance_int_18_higher_than_unused -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `enabled`, `share_delay_ticks` |
| overlay off | Invent not legal; same hash |
| mock + overlay on | same hash as off |
| execute Invent | table + event; inventor +200 influence and 1.2× food; others 1.0×; after delay all 1.2× |
| `--load` | table restored; influence not granted twice |
| INT 18 vs unused | chance 600 vs 400 |

CLI: `--inventions` does **not** imply `--sheet`.

---

## 2. Browser attach

```bash
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-cli --test net browser_page_ships_protocol_5 -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net ws_loopback_hello_snapshot -- --exact --nocapture
cargo test -p sim-cli --test net hash_neutral_attach -- --exact --nocapture
```

| Test | Success |
|---|---|
| Hello frame | postcard `0,5,0` (v5, no token) |
| `web/index.html` | PROTOCOL 5, postcard, `ws://` |
| `PROTOCOL_VERSION` | **5** |
| WS loopback | Welcome + snapshot |
| attach | hash-neutral |

Page (read, do not require a GUI browser in CI): `web/index.html` → `ws://127.0.0.1:9001`.

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M34 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --inventions --quiet
# open web/index.html (Connect to ws://127.0.0.1:9001)
cargo run -p viewer -- --connect ws://127.0.0.1:9001
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI and desktop browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; inventions 6; other packages unchanged from M34 plus inventions |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 43/43 |
| §0 `cargo test -p shared` | **pass** | 7/7 |
| §1 inventions `--exact` ×6 | **pass** | parse; off not legal; mock skip; inventor then society; load no re-grant; INT chance |
| §2 browser `--exact` ×5 | **pass** | Hello `0,5,0`; page v5; PROTOCOL 5; WS loopback; hash-neutral |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
