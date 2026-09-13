# M36 test plan — see each new feature

Walkthrough for [`M36-plan.md`](M36-plan.md). Automated tests prove the slice; the `sim-cli` / viewer / browser steps below are what you **read** (InspectorView JSON, postcard WS tables). Next slice: [`M51-plan.md`](M51-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test inspector`, `--test net`; unit tests under `--lib` need the module path (`protocol::tests::…`).

**Success for the slice:** `InspectorView` is display-only and not hashed. Tick JSON `metrics.inspector` fills agent / board / metrics tables without a PROTOCOL bump. Browser page stays postcard **v5** on `ws://`. Attach is hash-neutral. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. InspectorView

```bash
cargo test -p sim-core --test inspector inspector_one_row_per_living_agent -- --exact --nocapture
cargo test -p sim-core --test inspector inspector_board_and_metrics -- --exact --nocapture
cargo test -p sim-core --test inspector inspector_metrics_json_viewer_still_parses_timing -- --exact --nocapture
```

| Test | Success |
|---|---|
| one row per living agent | `agents.len() == sim.agents.len()`; `state_hash` unchanged |
| board + metrics | open/adopted text; trust 400; household kin; hungry + illness counts |
| TickTiming parse | extra `inspector` key; viewer `TickTiming` still deserializes |

Not hashed. Not `ExperimentConfig`.

---

## 2. Browser tables

```bash
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::subscribe_true_true_postcard_bytes -- --exact --nocapture
cargo test -p shared --lib protocol::tests::request_snapshot_postcard_bytes -- --exact --nocapture
cargo test -p shared --lib protocol::tests::tick_postcard_layout_metrics_at_end -- --exact --nocapture
cargo test -p sim-cli --test net browser_page_ships_protocol_5 -- --exact --nocapture
cargo test -p sim-cli --test net tick_metrics_include_inspector -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net ws_loopback_hello_snapshot -- --exact --nocapture
cargo test -p sim-cli --test net hash_neutral_attach -- --exact --nocapture
```

| Test | Success |
|---|---|
| Hello frame | postcard `0,5,0` (v5, no token) |
| Subscribe | postcard `[1,1,1]` (`want_events` + `want_decisions`) |
| RequestSnapshot | postcard `[2]` |
| Tick layout | metrics JSON after events/decisions (page decoder) |
| `web/index.html` | PROTOCOL 5, postcard, `ws://`, `#agents` `#board` `#metrics` |
| Tick.metrics | `inspector.agents` non-empty; viewer still parses timing |
| `PROTOCOL_VERSION` | **5** |
| WS loopback | Welcome + snapshot |
| attach | hash-neutral |

Page (read, do not require a GUI browser in CI): `web/index.html` → `ws://127.0.0.1:9001`. Missing wasm still shows tick/hash; tables fill from `Tick.metrics.inspector`.

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M35 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
# open web/index.html → Connect (Play if paused)
cargo run -p viewer -- --connect ws://127.0.0.1:9001
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI and desktop browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; inspector 3; inventions 6; other packages unchanged |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 44/44 |
| §0 `cargo test -p shared` | **pass** | 10/10 |
| §1 InspectorView `--exact` ×3 | **pass** | one row; board/metrics; TickTiming ignores inspector |
| §2 browser `--exact` ×9 | **pass** | Hello `0,5,0`; Subscribe/RequestSnapshot bytes; Tick layout; page sections; inspector on wire; PROTOCOL 5; WS loopback; hash-neutral |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
