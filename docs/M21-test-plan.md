# M21 test plan — see each new feature

Walkthrough for [`M21-plan.md`](M21-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (pack fill scale, `--connect` log tail, remote `/scrub`). Next slice: [`M42-plan.md`](M42-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`; library unit tests need `--lib` (viewer tests live in the binary crate).

**Success for the slice:** worn pack mesh scale empty **<** full (empty ≥ 0.40); crate helper unchanged. `sim-cli --connect` prints Welcome/Tick and is hash-neutral. `--listen` + `--connect` errors. Hello v2 vs v3 server is Protocol. Remote `/scrub` without `--allow-control` is ControlDisabled; with it, forward tick and ckpt reload match local hashes; same-tick scrub is idempotent. Remote `/give` `/set` still refused. Shipping `default.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 3`**.

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

## 1. Pack mesh scale-by-fill

```bash
cargo test -p viewer commands::tests::pack_fill_scale_empty_less_than_full -- --exact --nocapture
cargo test -p viewer commands::tests::pack_fill_scale_two_worn_same_mid_fill -- --exact --nocapture
cargo test -p viewer commands::tests::crate_fill_scale_one_item_smaller_than_full -- --exact --nocapture
```

| Test | Success |
|---|---|
| empty vs full pack scale | empty **<** full; empty = 0.40; full = 1.00 |
| two worn meshes, half-full | same scale, between 0.40 and 1.00; cap 0 → 0.40 |
| crate 1-item vs full | still smaller; crate helper unchanged |

Viewer (not automated): `/give 0 basket 1` empty satchel is small; pack food and the mesh grows. Both satchel and backpack use the same fill.

---

## 2. `sim-cli --connect` log tail

```bash
cargo test -p sim-cli --test net connect_log_tail_hash_neutral -- --exact --nocapture
cargo test -p sim-cli --test net listen_and_connect_is_error -- --exact --nocapture
```

Researcher (read; needs a server):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --llm mock --quiet
# other terminal:
cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --quiet
```

| Test | Success |
|---|---|
| `--connect` loopback | `welcome` + `snapshot ok` + `tick=`; server hash matches no-attach |
| `--listen` and `--connect` together | usage/load error |

---

## 3. Jump-to-tick on the wire (`PROTOCOL_VERSION = 3`)

```bash
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net hello_v2_against_v3_is_protocol_error -- --exact --nocapture
cargo test -p sim-cli --test net scrub_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net scrub_forward_from_live -- --exact --nocapture
cargo test -p sim-cli --test net scrub_reload_from_ckpt -- --exact --nocapture
cargo test -p viewer commands::tests::parse_set -- --exact --nocapture
cargo test -p viewer commands::tests::parse_set_respect -- --exact --nocapture
```

| Test | Success |
|---|---|
| `PROTOCOL_VERSION` | **3** |
| Hello v2 vs v3 server | Protocol error |
| remote `/scrub` without `--allow-control` | ControlDisabled |
| live tick N, Scrub(N+2) | tick N+2; hash matches ticking two more |
| live tick 4, ckpts at 2, Scrub(2) | tick 2; hash matches the ckpt |
| Scrub(same tick) twice | same hash |
| remote `/set` | still refused (in-process only) |

Viewer `--connect` (not automated): server `--allow-control`; `/scrub 50` sends `Control(Scrub(50))`. `/give` `/set` `/ckpt` `/events` still refuse.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M20 idle default).

---

## Execution record

**Ran:** 2026-08-28, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 14/14 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 16/16 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 pack fill `--exact` ×3 | **pass** | empty < full; mid-fill same scale; crate helper unchanged |
| §2 `--connect` `--exact` ×2 | **pass** | Welcome/Tick hash-neutral; listen+connect errors |
| §3 wire scrub `--exact` ×7 | **pass** | PROTOCOL 3; Hello v2 Protocol; Scrub without flag disabled; forward+reload; `/set` still in-process |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| §1/§3 viewer GUI | **not run** | no display automation |
