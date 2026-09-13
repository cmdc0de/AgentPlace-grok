# M52 — Day/night clock, world-size CLI, OTLP export

**Status:** planned (not yet implemented)  
**Depends on:** M51 complete (`docs/M51-plan.md`, git tag `M51`, commit `9ef0b42`)  
**Specs:** `docs/post-ga-feature-list.md` (PG-16 day/night, PG-18 world-size CLI, PG-11 OTLP leftover)

## Context

Time is **ticks only**. Rest adds a fixed `energy_regen_while_resting` (0.4 → milli) per Rest action. There is no day index, no night, no viewer lighting change. Map size is hashed `[world] width` / `height` in experiment TOML (shipping 64×64, min 32); there is no CLI override. M46 records hash-neutral tick aggregates + RSS when `[telemetry]` / `--telemetry` is on and **parses** `otlp_endpoint` but never POSTs.

M52 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged (no `[time]` table in those files; **code default is on**). OTLP export is **hash-neutral**. `--time` default-on and `--width`/`--height` **do** change hashes when they apply.

## Goal

A researcher can:

1. Run default `sim-cli` (no extra flags) and get a hashed **day / tick-of-day** clock. At each **dawn**, every living agent gets a **tiredness-scaled energy refill** (more depleted last night ⇒ smaller morning refill). Native viewer dims at night (hash-neutral). `--no-time` / `[time] enabled = false` ⇒ today’s Rest only, **M51 hashes**.
2. Pass **`--width N` / `--height N`** to override hashed `[world]` size without editing shipping TOML. Omit ⇒ TOML (64×64). Min **32**, max **256**. Flags apply at **new** sim only (`--load` / `--connect` use the checkpoint / server world).
3. Pass **`--telemetry`** with **`otlp_endpoint`** (or **`--otlp-endpoint URL`**) and have sim-cli **POST OTLP/JSON** tick `wall_ns` + RSS to that URL. Empty endpoint ⇒ today’s in-process aggregates only (M46). CI never dials. Hash-neutral.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

`--no-time` no-objects 2-tick hash stays **`70e5204d…`**. `--no-time` default CLI with shipped objects stays **`04069600…`**. Default (time **on**) idle hashes **change** (document on implement).

## In scope

No PROTOCOL bump. No new `SimEventKind`. No new ControlVerb. Do not store HTTP handles or OTLP clients on `Simulation`.

### A. Day / night clock (PG-16)

Overlay, not postcard, not `ExperimentConfig`. **On by default** (omit `[time]` ⇒ enabled). Do **not** add `[time]` to shipping `default.toml`.

```toml
[time]
enabled = true           # omit = true (M52 default on)
ticks_per_day = 240      # omit = 240 when enabled; ignored when enabled is false
```

CLI:

| Flag | Meaning |
|---|---|
| `--time` | Force overlay on (redundant with the default). |
| `--no-time` | Force overlay off. Rest only; M51 hashes. |

Does **not** imply `--sheet` / `--catalog` / `--telemetry`. No `--ticks-per-day` flag this slice (overlay only). `--no-time` wins if both are passed.

Derived (not stored on the checkpoint blob):

```
day = tick / ticks_per_day
tod  = tick % ticks_per_day
```

Night is `tod >= ticks_per_day * 3 / 4` (for 240: tod ≥ 180). Dawn is `tick > 0 && tod == 0` (first dawn at tick 240). Tick 0 spawn does **not** apply dawn (agents start maxed).

**Hash when overlay on (the default):** `enabled` + `ticks_per_day` (u64 LE). `tick` is already hashed, so day/tod need not be stored. Overlay **off** (`--no-time`) ⇒ do not hash these fields, **same hashes as M51**.

**Dawn refill** (integer millipoints, overlay on only), inside `tick()` after `tick` is incremented, once per living non-dead agent:

```
max = sheet.energy_max(config.energy_max_milli())
remaining_milli = energy * 1000 / max          # 0..1000; max==0 skip
refill = max * (200 + remaining_milli * 4 / 10) / 1000
energy = min(max, energy + refill)
```

| leftover energy | refill |
|---|---|
| empty (0) | `0.20 * max` |
| half | `0.40 * max` |
| full | `0.60 * max`, then clamp |

`--load` restores `tick` + energy; **do not** re-apply dawn on decode. Dawn runs only when `tick()` advances onto a dawn tick. Process overlay still applies after load (same as `--sheet`): default-on time continues unless `--no-time`.

**Rest action** stays today’s `+energy_regen_while_resting` (0.4 → milli) even when overlay on. Dawn is extra. Night does **not** gate Hunt/Farm/Attack this slice.

**Viewer (hash-neutral):** Status line `day N  tod t/T` when overlay on. Ambient / directional light intensity from a pure helper `light_for_tod(tod, ticks_per_day) -> f32` (day ~1.0, night ~0.15, 10-tick twilight lerp). `cargo test -p viewer` does not need a GPU; unit-test the helper. Viewer `--no-time` on the in-process `--config` path.

**Inspector:** extra JSON keys `day` / `tod` / `ticks_per_day` when overlay on (`skip_serializing_if` off). Tick `inspector` extra key pattern (M36). **No PROTOCOL bump.**

No BoardBlob fields. No format_version bump.

### B. World size CLI (PG-18)

```bash
sim-cli --config configs/default.toml --width 96 --height 96 --ticks 80 --llm mock --quiet
```

| Flag | Meaning |
|---|---|
| `--width N` | Override `[world] width` before `Simulation::new`. Omit ⇒ TOML. |
| `--height N` | Override `[world] height`. Omit ⇒ TOML. Either flag alone is allowed. |

Bounds: `MIN_MAP_SIZE` **32** (already in sim-core) … **256** inclusive. Below/above ⇒ process error (do not clamp silently). Non-integer ⇒ error.

**Hashed:** these write `ExperimentConfig.world.width/height` (world gen). Same master seed + different size ⇒ **different** `state_hash`. Document it. Not overlay-off identity.

Viewer: same flags on in-process `--config` path only. `--load` and `--connect` **ignore** `--width`/`--height` (world comes from ckpt / server). Do not reshape a checkpoint.

Shipping `default.toml` stays 64×64.

### C. OTLP export (PG-11 leftover)

M46 already: `[telemetry] enabled` / `--telemetry`; in-process `DurationStats` (count/total/avg/median/min/max) from `TickTiming.wall_ns`; process RSS last/peak; `otlp_endpoint` **parsed but never POSTed**.

This slice **exports**:

- Overlay key unchanged: `[telemetry] otlp_endpoint = "http://127.0.0.1:4318"`
- New CLI: `--otlp-endpoint URL` (sets endpoint; **implies** telemetry on)
- **sim-cli only** (not sim-core, not wasm), same pattern as rusqlite
- After each tick (and once on process exit if any samples), HTTP **POST** `{endpoint}/v1/metrics` (append `/v1/metrics` if the URL path is empty or `/`)
- **OTLP/JSON** (`Content-Type: application/json`). No TLS. No gRPC.
- Locked gauges (names): `agentplace.tick.wall_ns` (last tick), `agentplace.process.rss_bytes` (last RSS). Resource attribute `service.name=agentplace-sim`.
- Empty endpoint ⇒ M46 in-process only (no bind, no POST)
- `cargo test` never sets a public collector. Loopback: bind `127.0.0.1:0`, tick with that URL, assert one POST and the two metric names
- Overlay off / no flag ⇒ **same hashes**. Export must not enter `state_hash` or AGTN

Do **not** replace JSONL or sqlite. Do not add CPU/disk or viewer-frame OTLP this slice. Do not add a telemetry `SimEventKind`.

## Out of scope (later)

| Later | What |
|---|---|
| After M52 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL in the page; weapon **range**; spear melee; durability / workstations; PG-17 sleep places; night action gating; OTLP protobuf/gRPC; process CPU/disk; viewer-frame OTLP |
| Not M52 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; sqlite as checkpoint store; PG-9 extra recipes |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Clock is inspector/Status, not a Tick postcard field. OTLP is HTTP from sim-cli, not AGTN.
2. **format_version still writes 3 / reads v2+v3.** No BoardBlob time fields; day/tod are derived from `tick`.
3. **`[time] enabled` omit = true.** Default runs have a clock. `--no-time` restores M51 hashes. Do not add `[time]` to shipping `default.toml`.
4. Dawn refill is tiredness-scaled millipoints; Rest `+regen` stays. Night does not gate actions this slice.
5. `--width`/`--height` are ExperimentConfig overrides (hashed). `--load` / `--connect` ignore them.
6. OTLP/JSON POST is sim-cli only, hash-neutral, default off (empty endpoint). CI never dials.
7. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M52 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` default CLI 2 ticks | shipped-objects `04069600…` |
| default CLI 2 ticks (time on) | hash **≠** `04069600…` (lock the new value on implement) |
| `--no-time` | Rest +regen as today; no day/tod in inspector |
| default / `--time`, 2 ticks | `day=0` `tod=2` for `ticks_per_day=240`; hash ≠ `--no-time` |
| dawn refill | overlay on, force `tick=239` then one `tick()`: energy rises by the locked formula; empty leftover gets 20% max; full stays clamped |
| `--load` at tick 240 | energy not double-refilled |
| Rest overlay on | still +`energy_regen` millipoints (dawn is extra) |
| `--width 32 --height 32` | world is 32×32; hash ≠ 64×64 |
| `--width 31` / `--width 257` | process error |
| `--load` + `--width 96` | loaded world size unchanged (flag ignored) |
| `--otlp-endpoint` without live net | loopback POST received; hashes unchanged vs `--telemetry` with empty endpoint |
| telemetry off | same hash as no telemetry flag; no POST |
| Hello v5 / format 3 | unchanged |

GPU window and a real OTel collector are **not** required.

## PR Plan

### PR 1: `[time]` / `--time` / `--no-time` + dawn refill + viewer light helper

- Overlay parse, **omit = true**; hash `ticks_per_day` when on; dawn formula; inspector keys; Status `day/tod`; `light_for_tod` unit test; document new default idle hash

### PR 2: `--width` / `--height`

- sim-cli + viewer config-path flags; bounds 32…256; `--load`/`--connect` ignore; hash tests

### PR 3: OTLP/JSON POST

- sim-cli POST after tick; `--otlp-endpoint`; loopback test; hash identity

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `--time` / `--no-time`; `--width` / `--height`; `--otlp-endpoint URL`. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-cli --test net
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: `docs/M52-test-plan.md` (written on implement).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `04069600…`; default (time on) idle hash **new** (document); Hello v5; format_version 3 write.

## Risks

- **Time default-on changes default idle hashes.** `--no-time` is the M51 identity. Document the new default hash on implement. Do not add `[time]` to shipping `default.toml`.
- **Do not hash wall-clock ns / RSS / HTTP.** Telemetry stays hash-neutral (M46 rule).
- **Day/tod are derived from `tick`.** `--load` must not double-apply dawn. Process overlay after load still defaults on unless `--no-time`.
- **Size flags are ExperimentConfig**, not overlay-off identity.
- **OTLP lives in sim-cli**, not sim-core (wasm/CI). No TLS. Tests bind `127.0.0.1:0`.
- Postcard enums: **no new variants** this slice.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
