# M52 test plan — see each new feature

Walkthrough for [`M52-plan.md`](M52-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (day/night clock, `--width`/`--height`, OTLP/JSON). Next slice: [`M55-plan.md`](M55-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test clock` (or `objects`, `world_size_cli`, `otlp_cli`, `net`); unit tests under `-p sim-core` use `clock::tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `[time]` omit = **true**. `--no-time` shipped-objects hash `04069600…`; no-objects `70e5204d…`. Default (time on) shipped-objects 2-tick is `fed653be…`. Dawn at tick 240 adds a tiredness-scaled refill; Rest still `+regen`. `--width 32 --height 32` is 32×32 and hashes ≠ 64×64; 31 and 257 error; `--load` ignores `--width`. `--otlp-endpoint` POSTs OTLP/JSON on loopback and does not change hashes. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. Day / night clock (default on)

```bash
cargo test -p sim-core --lib clock::tests::omit_time_table_is_enabled -- --exact --nocapture
cargo test -p sim-core --lib clock::tests::day_tod_two_ticks -- --exact --nocapture
cargo test -p sim-core --lib clock::tests::dawn_refill_table -- --exact --nocapture
cargo test -p sim-core --lib clock::tests::light_day_bright_night_dim -- --exact --nocapture
cargo test -p sim-core --test clock time_on_hash_differs_from_off -- --exact --nocapture
cargo test -p sim-core --test clock inspector_time_keys_when_on -- --exact --nocapture
cargo test -p sim-core --test clock dawn_refill_on_tick_240 -- --exact --nocapture
cargo test -p sim-core --test clock dawn_full_energy_stays_clamped -- --exact --nocapture
cargo test -p sim-core --test clock load_at_dawn_does_not_double_refill -- --exact --nocapture
cargo test -p sim-core --test clock rest_still_adds_regen_when_time_on -- --exact --nocapture
cargo test -p sim-core --test objects no_time_shipped_objects_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p viewer light::tests::day_is_bright_night_is_dim -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit `[time]` | enabled, `ticks_per_day=240` |
| `--no-time` shipped objects | `04069600…` |
| `--no-time` no catalog | `70e5204d…` |
| default time on | `fed653be…`; inspector `day=0` `tod=2` |
| dawn | extra refill vs no-time; full energy clamped |
| `--load` at 240 | energy not doubled |
| Rest | still +`energy_regen` |

Researcher (imgui **not run** here):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
# Status: day N  tod t/240; night is dimmer
```

---

## 2. World size CLI

```bash
cargo test -p sim-cli --test world_size_cli width_32_height_32_prints_world_size -- --exact --nocapture
cargo test -p sim-cli --test world_size_cli width_32_hash_differs_from_default_64 -- --exact --nocapture
cargo test -p sim-cli --test world_size_cli width_31_is_error -- --exact --nocapture
cargo test -p sim-cli --test world_size_cli width_257_is_error -- --exact --nocapture
cargo test -p sim-cli --test world_size_cli load_ignores_width -- --exact --nocapture
```

| Test | Success |
|---|---|
| 32×32 | stdout `world=32x32`; hash ≠ 64×64 |
| 31 / 257 | process error `32..=256` |
| `--load` + `--width 96` | still 64×64 |

---

## 3. OTLP/JSON POST

```bash
cargo test -p sim-cli --bin sim-cli otlp::tests::metrics_url_appends_when_path_empty -- --exact --nocapture
cargo test -p sim-cli --test otlp_cli otlp_endpoint_posts_json_and_keeps_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| URL | `http://127.0.0.1:4318` → `…/v1/metrics` |
| loopback POST | `POST /v1/metrics` with `agentplace.tick.wall_ns` and `agentplace.process.rss_bytes`; hash = `--telemetry` empty endpoint |

---

## 4. Hello / format / idle CLI

```bash
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

| Test | Success |
|---|---|
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| default 2 ticks | `final_hash=fed653be…` |
| `--no-time` 2 ticks | `final_hash=04069600…` |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window / live OTLP collector not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 42; objects 38; combat 19; clock 7) |
| §0 `cargo test -p viewer` | **pass** | 58/58 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 clock / idle `--exact` | **pass** | omit=true; dawn formula; `--no-time` `04069600…` / `70e5204d…`; default `fed653be…` |
| §2 world size `--exact` ×5 | **pass** | 32×32; hash ≠ 64; 31/257 error; `--load` ignores `--width` |
| §3 OTLP `--exact` ×2 | **pass** | URL append; loopback POST; hash-neutral |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=fed653be7d5f4b778994fa48df38c34f81bb049461f2a1553f95ec92015382a1`; `--no-time` `final_hash=0406960048c1ada1c4910f7bd81bba89ed61a75ec85c2f43abc0d5491dfdff44` |
| viewer window / live collector | **not run** | no display; no public OTLP |
| live Spark / overnight | **not run** | not asked |
