# M24 test plan — see each new feature

Walkthrough for [`M24-plan.md`](M24-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (wire `/set`, `--connect` `/inject`, lockstep `AckTick`). Next slice: [`M31-plan.md`](M31-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`; viewer tests live in the binary crate.

**Success for the slice:** remote `/set` with `--allow-control` writes millipoints and changes hash; without the flag, ControlDisabled. `/inject PATH` from `sim-cli --connect --allow-control` applies the schedule. `--lockstep` waits for `AckTick` when a subscriber is attached; zero subscribers do not hang. Hello v4 vs v5 is Protocol. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Wire `/set`

```bash
cargo test -p sim-cli --test net set_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net set_hunger_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net set_respect_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net set_unknown_or_missing_toward_is_error -- --exact --nocapture
cargo test -p viewer commands::tests::remote_set_sends_control_verb -- --exact --nocapture
cargo test -p sim-cli client::tests::parse_play_pause_step_give -- --exact --nocapture
cargo test -p sim-cli client::tests::parse_set_respect_requires_toward -- --exact --nocapture
```

| Test | Success |
|---|---|
| Set without `--allow-control` | ControlDisabled |
| Set hunger 50 | millipoints 5000; hash changes |
| Set respect toward 1 value 40 | edge 4000 milli |
| unknown field / respect without toward / bad id | Error |
| viewer remote `/set` | sends `Control(Set)` |

Viewer `--connect` (not automated): `/set 0 hunger 50` `/set 0 respect 1 40`.

---

## 2. `sim-cli --connect` `/inject`

```bash
cargo test -p sim-cli --test net connect_inject_applies_schedule -- --exact --nocapture
cargo test -p sim-cli --test net connect_inject_missing_file_is_stderr -- --exact --nocapture
cargo test -p sim-cli --test net connect_log_tail_hash_neutral -- --exact --nocapture
cargo test -p sim-cli client::tests::parse_inject_path_extracts_file -- --exact --nocapture
```

| Test | Success |
|---|---|
| `/inject` coop.toml | server prints injected |
| missing file | stderr inject read error |
| `--connect` no `--allow-control` | hash-neutral |

```bash
# /inject configs/incentives/coop.toml
```

---

## 3. Lockstep ack + protocol 5

```bash
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net hello_v4_against_v5_is_protocol_error -- --exact --nocapture
cargo test -p sim-cli --test net lockstep_zero_subscribers_finishes -- --exact --nocapture
cargo test -p sim-cli --test net lockstep_waits_for_ack_then_advances -- --exact --nocapture
cargo test -p sim-cli --test net connect_auto_acks_lockstep -- --exact --nocapture
cargo test -p viewer net::tests::snapshot_apply_acks_world_tick -- --exact --nocapture
```

| Test | Success |
|---|---|
| `PROTOCOL_VERSION` | **5** |
| Hello v4 vs v5 | Protocol |
| `--lockstep` 0 subscribers | finishes `--ticks` |
| Tick then no Ack | stays at tick 1 until `AckTick(1)` then tick 2 |
| `--connect` auto-acks | lockstep server finishes |
| Snapshot apply | `AckTick(world tick)` |

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M23 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused --lockstep \
  --llm mock --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# /play
# /set 0 hunger 50
# /set 0 respect 1 40

cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --allow-control
# /inject configs/incentives/coop.toml
# /set 0 thirst 20
```

`--lockstep` without a subscriber still ticks. A v5 viewer that never acks stalls the sim — that is the mode.

---

## Execution record

**Ran:** 2026-08-28, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 41/41 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 Set `--exact` ×7 | **pass** | ControlDisabled; hunger 5000; respect 4000; errors; remote verb; parse |
| §2 inject `--exact` ×4 | **pass** | coop inject; missing file stderr; hash-neutral tail; parse path |
| §3 lockstep/protocol `--exact` ×6 | **pass** | PROTOCOL 5; Hello v4 Protocol; 0-sub finishes; wait then Ack; connect auto-ack |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
