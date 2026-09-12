# M42 test plan — see each new feature

Walkthrough for [`M42-plan.md`](M42-plan.md). Automated tests prove the slice; the `sim-cli` / browser steps below are what you **read** (CON energy/illness, INT memory, page `/ckpt` `/events`). Next slice: [`M47-plan.md`](M47-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test population` (or `objects`, `net`); unit tests under `--lib` need the module path (`sheet::tests::…`, `protocol::tests::…`).

**Success for the slice:** unused CON/INT keep today’s energy cap, illness 12, memory 128, retrieval_k 8. CON 18 vs 3 changes energy max and toxic-eat illness (8 vs 15). INT 18 vs 3 changes memory 144 vs 116 and k 12 vs 5. Page encodes `/ckpt next|prev` and `/events TICK` (Control tags 8/9/10). Without `--allow-control` → ControlDisabled. Idle mock 2 ticks stays `cd1e0853…`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. CON energy max + illness duration

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --lib sheet::tests::str_dex_wis_cha_mods -- --exact --nocapture
cargo test -p sim-core --test population unused_sheet_energy_memory_identity -- --exact --nocapture
cargo test -p sim-core --test population con_18_vs_3_energy_max_and_illness -- --exact --nocapture
cargo test -p sim-core --test population sheet_overlay_off_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused sheet | energy max = config; illness 12 |
| CON 18 vs 3 | rest energy higher for 18; toxic eat illness 8 vs 15 |
| overlay off | same hash as no `--sheet` |

---

## 2. INT memory + retrieval_k

```bash
cargo test -p sim-core --test population int_18_vs_3_memory_and_retrieval -- --exact --nocapture
cargo test -p sim-core --test population load_restores_con_int_not_derived -- --exact --nocapture
```

| Test | Success |
|---|---|
| INT 18 vs 3 | memory 144 vs 116; k 12 vs 5 |
| `--load` | scores restored; illness not doubled; derived caps recomputed |

---

## 3. Browser `/ckpt` `/events` + Hello v5

```bash
cargo test -p shared --lib protocol::tests::ckpt_events_postcard_bytes -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-cli --test net ckpt_events_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net browser_page_ships_protocol_5 -- --exact --nocapture
cargo test -p sim-cli --test net events_jsonl_display_only -- --exact --nocapture
cargo test -p sim-cli --test net ckpt_prev_from_tick_four_loads_two -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| postcard | CkptNext `[3,8]`; CkptPrev `[3,9]`; Events(2) `[3,10,2]` |
| page | `encodeCkptNext` / `encodeCkptPrev` / `encodeEvents` |
| no `--allow-control` | ControlDisabled |
| with control | `/events` display-only; `/ckpt prev` loads |
| idle 2 ticks | `final_hash=cd1e0853…` |
| Hello v5 | unchanged |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --sheet --quiet
# page: web/index.html  /ckpt next|prev  /events TICK
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --sheet --quiet
```

Live Ollama / overnight / imgui / desktop browser window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-09, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 |
| §0 `cargo test -p viewer` | **pass** | 33/33 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | protocol + transport green |
| §1 CON `--exact` ×5 | **pass** | illness 8 vs 15; unused identity |
| §2 INT `--exact` ×2 | **pass** | 144 vs 116; load no double-apply |
| §3 postcard / page / ControlDisabled / events / ckpt | **pass** | tags 8/9/10; ControlDisabled |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130` |
| browser window / imgui | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
