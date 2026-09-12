# M26 test plan — see each new feature

Walkthrough for [`M26-plan.md`](M26-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (Reflect/Plan every-N-ticks, record/replay of reflection text). Next slice: [`M47-plan.md`](M47-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test pipeline`, `--test reflect`, or `--test checkpoint`; viewer tests live in the binary crate.

**Success for the slice:** mock + `[llm] reflect_every_n_ticks` / `plan_every_n_ticks` same hash as overlay off. Custom insight Ok writes a protected Reflection on ticks N, 2N. Custom plan Ok is stored and visible to this tick’s `choose`. Err skips. Old JSONL (no `call`) is still action-only. Record + replay matches including evict-reflect. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Periodic Reflect / Plan

```bash
cargo test -p sim-core --test pipeline overlay_parses_every_n -- --exact --nocapture
cargo test -p sim-core --test pipeline mock_every_n_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test pipeline custom_insight_ok_on_tick_n -- --exact --nocapture
cargo test -p sim-core --test pipeline custom_plan_ok_visible_to_choose -- --exact --nocapture
cargo test -p sim-core --test pipeline custom_insight_plan_err_skips -- --exact --nocapture
cargo test -p sim-core --test pipeline overlay_off_custom_no_insight_plan_calls -- --exact --nocapture
cargo test -p sim-core --test checkpoint checkpoint_round_trip_preserves_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `reflect_every_n_ticks = 10`, `plan_every_n_ticks = 8`, `plan_length = 3`; omit ⇒ 0 / 0 / 4 |
| mock + overlay | same hash as off |
| Custom insight Ok, N=2 | Reflection at tick 2; none at tick 1 |
| Custom plan Ok | `agent.plan` non-empty; choose `obs.plan` set this tick |
| Custom Err | skip; no Reflection; plan empty |
| overlay off + Custom | no insight/plan calls |
| ckpt round-trip | hash preserved; empty plan; `format_version` 2 |

Overlay (not hashed):

```toml
[llm]
reflect_every_n_ticks = 10
plan_every_n_ticks = 10
plan_length = 4
```

CLI: `--llm-reflect-every N`, `--llm-plan-every N`. Live Spark **not run** unless asked.

---

## 2. Record / replay of reflection text

```bash
cargo test -p sim-core --test pipeline old_jsonl_without_call_is_choose -- --exact --nocapture
cargo test -p sim-core --test pipeline record_replay_insight_plan_same_hash -- --exact --nocapture
cargo test -p sim-core --test reflect record_replay_reflect_on_evict_same_hash -- --exact --nocapture
cargo test -p sim-core --test reflect replay_skips_reflect -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| old JSONL (no `call`) | still action choose; extra `reflect` line does not overwrite |
| record + replay Custom + both overlays | recording `state_hash` = replay; no live insight/plan on replay |
| record + replay `reflect_on_evict` | recording hash = replay; no live `reflect` call |
| empty replay table | missing `reflect_evict` line skips (no chooser call) |
| `PROTOCOL_VERSION` | **5** |

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M25 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm ollama --llm-reflect-every 10 --llm-plan-every 10 \
  --llm-reflect-on-evict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-02, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; pipeline 8; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 pipeline/checkpoint `--exact` ×7 | **pass** | overlay parse; mock same hash; insight at tick 2; plan visible to choose; Err skip; overlay off no calls; ckpt empty plan |
| §2 replay `--exact` ×5 | **pass** | old JSONL choose; record=replay insight/plan; record=replay evict; empty table skip; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--llm-reflect-every` overnight | **not run** | not asked |
