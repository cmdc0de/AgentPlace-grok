# M23 test plan — see each new feature

Walkthrough for [`M23-plan.md`](M23-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (every-tick Snapshots, LLM barrier, `sim-cli --connect` Control). Next slice: [`M52-plan.md`](M52-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net` or `--test barrier`; viewer tests live in the binary crate.

**Success for the slice:** attached world applies Snapshots in order (one per frame), no 200 ms throttle. Mock pipeline N/N. `--llm-barrier` retries timeout/parse (default 3 extra) then Wait; remaining agents still finish the tick. `sim-cli --connect` without `--allow-control` stays hash-neutral; with the flag, stdin `/play` `/give` send Control. Remote `/set` still refused. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 4`**.

---

## 0. Safety net

No network. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-bevy
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. MF-2 — see every tick

```bash
cargo test -p viewer net::tests::every_tick_requests_a_snapshot -- --exact --nocapture
cargo test -p viewer net::tests::first_snapshot_stops_later_world_applies -- --exact --nocapture
cargo test -p viewer net::tests::full_tick_channel_still_updates_live_clock -- --exact --nocapture
cargo test -p viewer net::tests::status_tick_hash_local_when_not_attached -- --exact --nocapture
cargo test -p viewer net::tests::status_shows_world_and_live_when_they_differ -- --exact --nocapture
cargo test -p sim-bevy --lib tests::overdue_timer_still_one_tick_per_frame -- --exact --nocapture
```

| Test | Success |
|---|---|
| two Ticks | two Snapshot requests |
| Snapshots in one queue | first applied; later left for next frame |
| full Tick channel | live clock still advances; Tick may drop |
| not attached | HUD tick = local `sim.tick` |
| live ≠ world | Status shows both |
| overdue in-process timer | one `tick()` per frame (`just_finished`, not a times_finished loop) |

Viewer `--connect` (not automated): `/play`; 3D should visit every tick in order. Status `tick N (live M)` if catching up. `pipeline N/N` in Status.

---

## 2. MF-3 — pipeline verify + barrier

```bash
cargo test -p sim-core --test timing timing_does_not_change_hash -- --exact --nocapture
cargo test -p sim-core --test barrier mock_pipeline_n_of_n -- --exact --nocapture
cargo test -p sim-core --test barrier overlay_barrier_not_on_experiment_config -- --exact --nocapture
cargo test -p sim-core --test barrier sleeping_chooser_blocks_tick_return -- --exact --nocapture
cargo test -p sim-core --test barrier barrier_retries_then_success -- --exact --nocapture
cargo test -p sim-core --test barrier barrier_timeout_exhausted_then_wait_other_agents_run -- --exact --nocapture
cargo test -p sim-core --test barrier barrier_retries_zero_one_attempt -- --exact --nocapture
cargo test -p sim-core --test barrier barrier_off_timeout_is_one_attempt -- --exact --nocapture
```

| Test | Success |
|---|---|
| mock timing | `agents.len()` == living; `remember_ns` written; hash-neutral |
| overlay `[llm] barrier` | parsed; default retries 3; not on `ExperimentConfig` |
| sleeping chooser | `tick()` waits; no *T+1* until it returns |
| barrier, fail twice then Ok | no `LlmWait` |
| barrier, always Timeout | 4 attempts/agent (`1+3`); `LlmWait`; both agents timed |
| `--llm-barrier-retries 0` | one attempt; `LlmWait` |
| barrier off + Timeout | one attempt; `LlmWait` |

Overlay (not hashed):

```toml
[llm]
barrier = true
barrier_retries = 3
```

CLI: `--llm-barrier` / `--llm-barrier-retries N`. Do not set these in shipping `default.toml`.

---

## 3. `sim-cli --connect` Control + protocol 4

```bash
cargo test -p sim-cli --test net connect_log_tail_hash_neutral -- --exact --nocapture
cargo test -p sim-cli --test net connect_allow_control_play_unpauses -- --exact --nocapture
cargo test -p sim-cli --test net connect_allow_control_give_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net connect_allow_control_without_server_flag_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli client::tests::parse_play_pause_step_give -- --exact --nocapture
cargo test -p sim-cli client::tests::parse_set_is_refused -- --exact --nocapture
```

| Test | Success |
|---|---|
| `--connect` no `--allow-control` | hash-neutral; no Control sent |
| `--connect --allow-control` `/play` | start-paused server ticks |
| `/give 0 berry_bush 1` | inventory +1 |
| server without `--allow-control` | ControlDisabled / error |
| `PROTOCOL_VERSION` | **4** |
| `/set` | refused (in-process only) |

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M22 idle default). No `[llm] barrier` in `configs/default.toml`.

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm mock --llm-barrier --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# Space or /play; 3D visits every tick; Status world vs live; pipeline N/N

# other terminal
cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --allow-control
# /play
# /give 0 berry_bush 1
# /pause
```

`--start-paused` needs `--listen` and `--allow-control`. Worst case with barrier on and a dead LLM: `(1+3) × timeout_ms` per agent (default.toml 120 s ⇒ 8 min/agent). Mock does not call the network.

Live Spark / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-08-28, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 20/20 |
| §0 `cargo test -p sim-bevy` | **pass** | 1/1 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 29/29 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 MF-2 `--exact` ×6 | **pass** | snapshot-per-tick; one Snapshot/frame; live clock on full channel; HUD world vs live |
| §2 MF-3 `--exact` ×8 | **pass** | N/N timing; overlay parse; sleep blocks; retry then Ok; 4 attempts then Wait; retries 0; barrier off |
| §3 connect Control `--exact` ×7 | **pass** | hash-neutral tail; `/play`; `/give`; ControlDisabled; PROTOCOL 4; `/set` refused |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…`; no `barrier` in `configs/default.toml` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--llm-barrier` overnight | **not run** | not asked |
