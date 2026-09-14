# M38 test plan — see each new feature

Walkthrough for [`M38-plan.md`](M38-plan.md). Automated tests prove the slice; the `sim-cli` / viewer / browser steps below are what you **read** (authored models with primitive fallback, page `/inject` `/scrub`, Ubuntu wasm32). Next slice: [`M55-plan.md`](M55-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network (except optional `rustup target add wasm32-unknown-unknown` once).

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test ci`; unit tests under `--lib` need the module path (`protocol::tests::…`). Viewer tests live in the binary (`models::tests::…`).

**Success for the slice:** missing glb falls back to primitives; models never hashed. Page `/inject` `/scrub` use existing wire messages. Native `from_checkpoint_bytes` / `sim-wasm` helper match; Ubuntu CI builds wasm32. Shipping TOML unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
cargo test -p sim-wasm
```

---

## 1. Viewer 3D models

```bash
cargo test -p viewer --bin viewer models::tests::path_map_covers_locked_stems -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::missing_glb_falls_back -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::authored_files_are_not_in_state_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| path map | locked stems (agent, veg, hare/perch, items, crate) |
| missing glb | `resolve_model` None ⇒ primitive path |
| hash | dummy `agent.glb` does not change `state_hash` |

Drop `.glb` files under `assets/models/` (see that README). `cargo test -p viewer` must pass with the directory empty.

---

## 2. Browser `/inject` `/scrub` + wasm

```bash
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::inject_postcard_bytes -- --exact --nocapture
cargo test -p shared --lib protocol::tests::scrub_postcard_bytes -- --exact --nocapture
cargo test -p sim-wasm --lib tests::native_helper_matches_from_sim -- --exact --nocapture
cargo test -p sim-core --test inspector inspector_from_checkpoint_bytes_matches_from_sim -- --exact --nocapture
cargo test -p sim-core --test ci github_actions_workflow_exists -- --exact --nocapture
cargo test -p sim-cli --test net browser_page_ships_protocol_5 -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
cargo test -p sim-cli --test net inject_incentive_without_flag_errors -- --exact --nocapture
cargo test -p sim-cli --test net inject_incentive_with_flag -- --exact --nocapture
cargo test -p sim-cli --test net scrub_without_allow_control_is_disabled -- --exact --nocapture
cargo test -p sim-cli --test net scrub_forward_from_live -- --exact --nocapture
cargo test -p sim-cli --test net hash_neutral_attach -- --exact --nocapture
cargo build -p sim-wasm --target wasm32-unknown-unknown
```

| Test | Success |
|---|---|
| Hello | postcard `0,5,0` |
| Inject / Scrub bytes | page encoder layout |
| native wasm helper | JSON matches `from_sim` |
| CI yaml | `wasm32-unknown-unknown` + `sim-wasm` |
| page | PROTOCOL 5; `encodeInject` / `encodeScrub` |
| inject/scrub without flag | `ControlDisabled` |
| inject/scrub with flag | mutates / jumps |
| attach | hash-neutral until inject/scrub |
| wasm32 build | Ubuntu CI command compiles |

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…`.

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
# open web/index.html → Connect; /inject /scrub if --allow-control
cargo run -p viewer -- --config configs/default.toml
```

Live Ollama / overnight / imgui window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-04, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI and desktop browser window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; ci 1; inspector 4 |
| §0 `cargo test -p viewer` | **pass** | 26/26 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 44/44 |
| §0 `cargo test -p shared` | **pass** | 14/14 |
| §0 `cargo test -p sim-wasm` | **pass** | 1/1 |
| §1 models `--exact` ×3 | **pass** | path map; missing fallback; hash-neutral dummy glb |
| §2 browser/wasm `--exact` + wasm32 build | **pass** | Inject/Scrub bytes; CI yaml; ControlDisabled; wasm32 compiles |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc` |
| viewer GUI / desktop browser | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
