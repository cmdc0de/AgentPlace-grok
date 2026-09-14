# M25 test plan — see each new feature

Walkthrough for [`M25-plan.md`](M25-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (reflection-on-evict, lockstep Ack timeout). Next slice: [`M56-plan.md`](M56-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net` or `--test reflect`; viewer tests live in the binary crate.

**Success for the slice:** mock + `[llm] reflect_on_evict` same hash as overlay off. Custom `reflect` Ok writes a protected Reflection. `reflect` Err drops without Reflection. Replay skips reflect. `--lockstep-timeout-ms 200` advances without Ack. Timeout 0 still waits. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Reflection-on-evict

```bash
cargo test -p sim-core --test reflect overlay_parses_reflect_on_evict -- --exact --nocapture
cargo test -p sim-core --test reflect mock_reflect_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test reflect custom_reflect_ok_writes_protected_memory -- --exact --nocapture
cargo test -p sim-core --test reflect custom_reflect_err_drops_without_reflection -- --exact --nocapture
cargo test -p sim-core --test reflect replay_skips_reflect -- --exact --nocapture
cargo test -p sim-core --test checkpoint checkpoint_round_trip_preserves_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `reflect_on_evict = true` |
| mock + overlay | same hash as off |
| Custom Ok, cap 2, 3 remembers | Reflection present; not all originals |
| Custom Err | no Reflection; cap held |
| replay set | no `reflect` call |
| ckpt round-trip | hash preserved (old layouts load) |

Overlay (not hashed):

```toml
[llm]
reflect_on_evict = true
```

CLI: `--llm-reflect-on-evict`. Live Spark **not run** unless asked.

---

## 2. Lockstep Ack timeout

```bash
cargo test -p sim-cli --test net lockstep_waits_for_ack_then_advances -- --exact --nocapture
cargo test -p sim-cli --test net lockstep_timeout_advances_without_ack -- --exact --nocapture
cargo test -p sim-cli --test net lockstep_zero_subscribers_finishes -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| timeout 0 / omit, no Ack | still waits (tick stays 1 until Ack) |
| `--lockstep-timeout-ms 200`, no Ack | tick ≥ 2 within ~1s |
| 0 subscribers | finishes `--ticks` |
| `PROTOCOL_VERSION` | **5** |

```
sim-cli --listen tcp://… --lockstep --lockstep-timeout-ms 5000
```

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M24 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --lockstep --lockstep-timeout-ms 5000 \
  --llm ollama --llm-reflect-on-evict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-02, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; reflect 5; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 reflect `--exact` ×6 | **pass** | overlay parse; mock same hash; Custom Ok Reflection; Err drop; replay skip; ckpt round-trip |
| §2 lockstep timeout `--exact` ×4 | **pass** | wait-forever still waits; 200 ms advances; 0-sub finishes; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--llm-reflect-on-evict` overnight | **not run** | not asked |
