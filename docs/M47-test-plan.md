# M47 test plan — see each new feature

Walkthrough for [`M47-plan.md`](M47-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (camera pan, missing-asset sentinel, time-series charts). Next slice: [`M58-plan.md`](M58-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `survival`, `net`); unit tests under `-p viewer` use the module path (`camera::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** arrow / `u` / `d` pan helpers (tap 2.0, hold 12 u/s, height clamp, follow cancel). `L` stays legend. Configured missing glb ⇒ Sentinel, not Primitive; empty visual stays Primitive; authored file stays Authored; LOD exhausted ⇒ Sentinel. Chart ring caps at 256, drops oldest, `wall_ms` from `wall_ns`; filling the ring does not change `state_hash`. Default `sim-cli` 2-tick hash is `5f231378…` (shipped objects) / `70e5204d…` without catalog. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. Camera pan

```bash
cargo test -p viewer camera::tests::pan_step_tap_hold_none -- --exact --nocapture
cargo test -p viewer camera::tests::pan_xz_unit_steps_no_y -- --exact --nocapture
cargo test -p viewer camera::tests::height_clamp_does_not_go_below_min -- --exact --nocapture
cargo test -p viewer camera::tests::follow_cancel_on_nonzero_pan -- --exact --nocapture
cargo test -p viewer commands::tests::parse_help -- --exact --nocapture
```

| Test | Success |
|---|---|
| `pan_step` | tap → 2.0; hold dt=0.25 → 3.0; neither → 0.0 |
| `pan_xz` | right/left/forward/back are ground-plane unit steps |
| height clamp | `d` will not go below `terrain + 2.0` |
| follow cancel | non-zero pan clears follow |
| help | arrows / `u` / `d` / `L legend` / `/charts` |

Native window (read, not run here): arrows pan at constant height; `u` raise; `d` lower; `L` legend; first pan cancels `/follow`. Ignore keys while the console has focus.

---

## 2. Missing-asset sentinel

```bash
cargo test -p viewer models::tests::empty_visual_is_primitive -- --exact --nocapture
cargo test -p viewer models::tests::configured_missing_path_is_sentinel -- --exact --nocapture
cargo test -p viewer models::tests::authored_exists_is_authored -- --exact --nocapture
cargo test -p viewer models::tests::lod_exhausted_is_sentinel -- --exact --nocapture
```

| Test | Success |
|---|---|
| empty visual | Primitive |
| configured missing path | Sentinel, not Primitive |
| authored exists | Authored |
| LOD miss then missing | Sentinel |

In the native viewer a configured missing glb is a **magenta cuboid** and logs `glb miss {id} -> sentinel`. Empty `[visual]` still uses today’s primitive.

---

## 3. Time-series charts + Hello / format / idle

```bash
cargo test -p viewer charts::tests::chart_ring_caps_at_256_and_drops_oldest -- --exact --nocapture
cargo test -p viewer charts::tests::charts_not_hashed -- --exact --nocapture
cargo test -p viewer commands::tests::parse_charts -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| chart ring | 300 pushes → len 256; oldest wall_ms 44 |
| charts not hashed | ring fill leaves `state_hash` unchanged |
| `/charts` | parses |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=5f231378…` |
| no catalog | `70e5204d…` |

`C` / `/charts` opens the imgui Charts window (last 256: wall_ms, living, hungry, thirsty, mean hunger). Attach page draws session sparklines under Metrics. Buffer is viewer/page-only.

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
```

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-11, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 37; population 49; objects 27) |
| §0 `cargo test -p viewer` | **pass** | 46/46 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 camera `--exact` ×5 | **pass** | tap 2.0; hold 3.0; clamp; follow cancel; help has arrows/`d`/`L legend` |
| §2 sentinel `--exact` ×4 | **pass** | empty Primitive; missing Sentinel; authored Authored; LOD exhausted Sentinel |
| §3 charts / Hello / idle | **pass** | ring 256; hash unchanged; `/charts`; format 3; Hello v5; no-catalog `70e5204d…` |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=5f2313783257960c08253b6717514019459533da6923884a981b1ab2c8068fc2` |
| viewer window / imgui / attach page | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
