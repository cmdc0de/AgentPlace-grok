# M27 test plan — see each new feature

Walkthrough for [`M27-plan.md`](M27-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (auto-execute plan, combat). Next slice: [`M42-plan.md`](M42-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test pipeline`, `--test combat`, or `--test checkpoint`; viewer tests live in the binary crate.

**Success for the slice:** mock + `[llm] execute_plan` / `[conflict] enabled` same hash as overlay off. Custom execute_plan pops a legal Wait and skips choose. Unparseable plan falls through. Record + replay matches. Attack drops defender energy when overlay on; Attack is not legal when off. Flee moves away or Wait. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Auto-execute plan

```bash
cargo test -p sim-core --test pipeline overlay_parses_execute_plan -- --exact --nocapture
cargo test -p sim-core --test pipeline mock_execute_plan_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test pipeline execute_plan_pops_legal_wait -- --exact --nocapture
cargo test -p sim-core --test pipeline unparseable_plan_falls_through -- --exact --nocapture
cargo test -p sim-core --test pipeline parse_plan_json_accepts_objects -- --exact --nocapture
cargo test -p sim-core --test pipeline record_replay_execute_plan_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `execute_plan = true`; omit ⇒ false |
| mock + overlay | same hash as off |
| Custom + `{"action":"Wait"}` | that tick skips choose; plan shorter |
| unparseable `plan[0]` | choose runs; plan unchanged |
| plan JSON objects | `parse_plan_json` accepts `{"action":"..."}` elements |
| record + replay | recording `state_hash` = replay |

Overlay (not hashed):

```toml
[llm]
execute_plan = true
```

CLI: `--llm-execute-plan`. Live Spark **not run** unless asked.

---

## 2. Combat (v2-lite)

```bash
cargo test -p sim-core --test combat overlay_parses_conflict -- --exact --nocapture
cargo test -p sim-core --test combat mock_conflict_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test combat overlay_off_attack_not_legal -- --exact --nocapture
cargo test -p sim-core --test combat custom_attack_adjacent_drops_energy -- --exact --nocapture
cargo test -p sim-core --test combat flee_moves_away_or_waits -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[conflict] enabled = true` |
| mock + overlay | same hash as off |
| overlay off | Attack/Flee not in `legal` |
| Attack adjacent | defender energy down; `Attack` event |
| Flee | moves away or Wait if blocked |
| `PROTOCOL_VERSION` | **5** |

```toml
[conflict]
enabled = true
```

CLI: `--conflict`.

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M26 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm ollama --llm-reflect-every 10 --llm-plan-every 10 \
  --llm-execute-plan --conflict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-02, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; combat 5; compare 2; determinism 11; governance 44; incentives 19; pipeline 14; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 execute-plan `--exact` ×6 | **pass** | overlay parse; mock same hash; pop Wait; unparseable fall-through; plan objects; record=replay |
| §2 combat `--exact` ×6 | **pass** | overlay parse; mock same hash; Attack not legal off; energy drop + event; Flee move/Wait; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--llm-execute-plan --conflict` overnight | **not run** | not asked |
