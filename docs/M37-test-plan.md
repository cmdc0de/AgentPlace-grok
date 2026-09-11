# M37 test plan — see each new feature

Walkthrough for [`M37-plan.md`](M37-plan.md). Automated tests prove the slice; the `sim-cli` / viewer / browser steps below are what you **read** (extra invention kinds, page `/set` `/give`). Next slice: [`M45-plan.md`](M45-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test inventions`, `--test inspector`, `--test net`; unit tests under `--lib` need the module path (`protocol::tests::…`).

**Success for the slice:** extra kinds MoveBonus / SenseBonus, lowest-missing Invent (first still GatherBonus), inventor-then-society, no stack. Page `/set` `/give` use existing ControlVerbs. Snapshot helper is Rust `from_checkpoint_bytes`; wasm optional. Overlay off / mock ⇒ idle hash unchanged. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Extra invention kinds

```bash
cargo test -p sim-core --test inventions overlay_parses_inventions -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions mock_inventions_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions execute_invent_inventor_then_society -- --exact --nocapture
cargo test -p sim-core --test inventions execute_invent_move_then_sense -- --exact --nocapture
cargo test -p sim-core --test inventions fourth_invent_waits_all_kinds_present -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_load_does_not_regrant_influence -- --exact --nocapture
cargo test -p sim-core --test inventions invent_chance_int_18_higher_than_unused -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `enabled`, `share_delay_ticks` |
| overlay off | Invent not legal; same hash |
| mock + overlay on | same hash as off |
| first Invent | GatherBonus; inventor +200 influence and 1.2× food |
| second / third | MoveBonus cheaper move; SenseBonus +1 range; others wait for share |
| after share | that kind `shared`; no stack |
| fourth Invent | Wait; all three kinds present |
| `--load` | table restored; influence not granted twice |
| INT 18 vs unused | chance 600 vs 400 |

CLI: `--inventions` does **not** imply `--sheet`.

---

## 2. Browser `/set` `/give` + Snapshot helper

```bash
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::give_postcard_bytes -- --exact --nocapture
cargo test -p shared --lib protocol::tests::set_hunger_postcard_bytes -- --exact --nocapture
cargo test -p sim-core --test inspector inspector_from_checkpoint_bytes_matches_from_sim -- --exact --nocapture
cargo test -p sim-cli --test net browser_page_ships_protocol_5 -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net give_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net set_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net give_berry_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net set_hunger_changes_hash -- --exact --nocapture
cargo test -p sim-cli --test net ws_loopback_hello_snapshot -- --exact --nocapture
cargo test -p sim-cli --test net hash_neutral_attach -- --exact --nocapture
```

| Test | Success |
|---|---|
| Hello frame | postcard `0,5,0` (v5, no token) |
| Give / Set bytes | `ClientMessage::Control` postcard layout for the page encoder |
| Snapshot helper | `from_checkpoint_bytes` matches `from_sim` |
| `web/index.html` | PROTOCOL 5, `#inventions`, `encodeGive` / `encodeSet`, optional `sim_wasm` |
| `PROTOCOL_VERSION` | **5** |
| `/give` `/set` without flag | `ControlDisabled` |
| `/give` `/set` with flag | hash-sensitive |
| WS loopback | Welcome + snapshot |
| attach | hash-neutral until mutate |

Page (read, do not require a GUI browser in CI): `web/index.html` → `ws://127.0.0.1:9001`. Missing wasm still shows Tick inspector.

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M36 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --inventions --quiet
# open web/index.html → Connect; /give /set if --allow-control
cargo run -p viewer -- --connect ws://127.0.0.1:9001
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI and desktop browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; inventions 8; inspector 4 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 44/44 |
| §0 `cargo test -p shared` | **pass** | 12/12 |
| §1 inventions `--exact` ×8 | **pass** | parse; off/mock same hash; GatherBonus; MoveBonus/SenseBonus; fourth Wait; load; INT chance |
| §2 browser `--exact` ×12 | **pass** | Hello v5; Give/Set bytes; checkpoint helper; page sections; ControlDisabled; hash-sensitive give/set; WS; hash-neutral |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
