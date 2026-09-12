# M22 test plan — see each new feature

Walkthrough for [`M22-plan.md`](M22-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (wire Give, remote `/ckpt`, remote `/events`). Next slice: [`M47-plan.md`](M47-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`; viewer tests live in the binary crate.

**Success for the slice:** remote `/give` with `--allow-control` adds to pockets and changes hash; without the flag, ControlDisabled. Unknown item / qty 0 is Error. `/ckpt prev` from tick 4 with files at 2 and 4 loads tick 2. `/events` is display-only (hash unchanged). Hello v3 vs v4 is Protocol. Remote `/set` still refused. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 4`**.

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

## 1. Wire Give

```bash
cargo test -p sim-cli --test net give_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net give_berry_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net give_unknown_or_zero_is_error -- --exact --nocapture
cargo test -p viewer commands::tests::parse_give -- --exact --nocapture
```

| Test | Success |
|---|---|
| Give without `--allow-control` | ControlDisabled |
| Give `berry_bush` 1 with flag | inventory +1; hash changes |
| unknown item / qty 0 | Error |
| `/give 0 berry_bush 2` | parses |

Viewer `--connect` (not automated): server `--allow-control`; `/give 0 berry_bush 1` updates inspector.

---

## 2. Remote `/ckpt next|prev`

```bash
cargo test -p sim-cli --test net ckpt_next_without_files_is_error -- --exact --nocapture
cargo test -p sim-cli --test net ckpt_prev_from_tick_four_loads_two -- --exact --nocapture
cargo test -p viewer commands::tests::parse_scrub_and_ckpt -- --exact --nocapture
```

| Test | Success |
|---|---|
| CkptNext, no files | Error |
| Step to 4 with ckpts at 2 and 4, CkptPrev | tick 2; hash matches file |
| `/ckpt next` `/ckpt prev` | parse |

`/scrub` is still catch-up (M21), not file-to-file.

---

## 3. Remote `/events` + protocol 4

```bash
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net hello_v3_against_v4_is_protocol_error -- --exact --nocapture
cargo test -p sim-cli --test net events_without_jsonl_is_error -- --exact --nocapture
cargo test -p sim-cli --test net events_jsonl_display_only -- --exact --nocapture
cargo test -p viewer commands::tests::parse_events -- --exact --nocapture
cargo test -p viewer commands::tests::parse_set -- --exact --nocapture
```

| Test | Success |
|---|---|
| `PROTOCOL_VERSION` | **4** |
| Hello v3 vs v4 server | Protocol error |
| Events, no JSONL | Error |
| Events with JSONL | ReportReady has lines; sim hash unchanged |
| `/events 2` | parses |
| remote `/set` | still in-process only |

---

## 3b. Attach HUD live tick (MF-1)

```bash
cargo test -p viewer net::tests::status_tick_hash_local_when_not_attached -- --exact --nocapture
cargo test -p viewer net::tests::status_from_live_uses_server_tick_not_snapshot -- --exact --nocapture
```

| Test | Success |
|---|---|
| not attached | HUD tick = local `sim.tick` |
| live clock | HUD tick = server Tick (e.g. 80), not snapshot |

`--connect` Status `tick N` should advance every server step after `/play`. Meshes may still lag ~200 ms.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M21 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m22 --checkpoint-every 2 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused --llm mock --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# Space or /play to run; /pause to stop
# /give 0 berry_bush 1
# /ckpt prev   /ckpt next
# /events 2
```

`--start-paused` needs `--listen` and `--allow-control`. The server stays at tick 0 until Play.

```bash
cargo test -p sim-cli --test net start_paused_requires_listen_and_control -- --exact --nocapture
cargo test -p sim-cli --test net start_paused_holds_tick_until_play -- --exact --nocapture
```

---

## Execution record

**Ran:** 2026-08-28, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 16/16 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 24/24 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 Give `--exact` ×4 | **pass** | ControlDisabled; berry +1 hash; unknown/qty0 Error; parse |
| §2 ckpt `--exact` ×3 | **pass** | no files Error; CkptPrev 4→2; parse |
| §3 events/protocol `--exact` ×6 | **pass** | PROTOCOL 4; Hello v3 Protocol; no JSONL Error; display-only; `/set` in-process |
| §3b HUD live tick `--exact` ×2 | **pass** | local sim.tick; live clock 80 not snapshot |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
