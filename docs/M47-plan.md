# M47 — Viewer camera pan, missing-asset sentinel, time-series charts

**Status:** implemented  
**Depends on:** M46 complete (`docs/M46-plan.md`, git tag `M46`, commit `8cf39b8`)  
**Walkthrough:** [`docs/M47-test-plan.md`](M47-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-12 camera, PG-10 sentinel); charts parked since M6/M8 / PG-7 attach page

## Context

The native viewer camera starts at a fixed offset (or `/follow` snaps every frame). There is **no** free pan. `L` toggles the imgui **legend**. Configured `glb` / LOD paths that are missing fall back to today’s **primitive**, which looks like “no art on purpose.” Status / World windows show last-tick numbers; `timing_ring` keeps 64 `wall_ns` samples for mean/max **text** only. The attach page prints `inspector.metrics` as a table, not a series.

M47 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. All three items are **hash-neutral**.

## Goal

A researcher can:

1. In the **native** Bevy window, pan the 3D camera with **arrow keys** at constant height; **`u`** raises, **`d`** lowers (clamp above terrain). **`L` stays legend.** First pan/height key **cancels `/follow`**.
2. See one **fixed sentinel mesh** (generated magenta cuboid) when a visual path is configured and the file is missing — not today’s silent primitive. Empty / omitted `[visual]` still uses the primitive.
3. Open an imgui **Charts** window (`C` / `/charts`) of the last **256** ticks (wall_ms, living, hungry, thirsty, mean hunger) and see the same series as **sparklines** on the attach page. Buffer is viewer/page-only.
4. Idle mock hashes **unchanged**. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Default CLI 2-tick hash stays **`5f231378…`** (shipped objects) / **`70e5204d…`** (no catalog).

## In scope

No PROTOCOL bump. No new `SimEventKind`. No new ControlVerb. Do not store camera, sentinel, or chart samples on `Simulation` / AGTN.

### A. Viewer camera pan (PG-12)

Native Bevy window only. Ignore these keys when `ui.want_keyboard` (console focus), same as other shortcuts.

| Key | Motion |
|---|---|
| Arrow **Right** / **Left** | Pan camera-right / camera-left, projected on XZ. **Y unchanged.** |
| Arrow **Up** / **Down** | Pan look-forward / look-back on XZ. **Y unchanged.** Do not dive into terrain. |
| **`u`** | Raise camera (**+Y** only). |
| **`d`** | Lower camera (**−Y** only). Clamp `Y >= height_at(camera xz cell) + 2.0`. |
| **`L`** | **Unchanged** — toggles legend. `/legend` unchanged. |

- Translate only; keep current rotation (do not `look_at` while free-panning).
- **Tap** (`just_pressed`): **2.0** world units. **Hold** (`pressed` and not just_pressed): **12.0** units/sec × `Time::delta`. Helper `pan_step(just_pressed, pressed, dt) -> f32`.
- First arrow / `u` / `d` that produces a non-zero step sets `state.follow = None` so `update_camera` stops snapping.
- No mouse-drag, scroll zoom, gamepad, or browser camera.

### B. Missing-asset sentinel (PG-10)

Hash-neutral. One shared generated mesh (bright **magenta** cuboid in code — no extra `.glb` download). Same mesh for every broken id.

| Case | Mesh |
|---|---|
| No `[visual]` / empty path | Today’s **primitive** (intentional “no art”). |
| Path set, `existing_file` finds none (including LOD exhausted) | **Sentinel**. |
| Path set, file exists | Authored glb as today. LOD miss ⇒ next coarser, then sentinel if **any** path was configured. |

- Resolve helper returns `Authored | Primitive | Sentinel` (not `Option` that collapses miss to primitive).
- Log (hash-neutral, first miss per id is enough): `glb miss {id} -> sentinel`.
- Corrupt file that exists on disk: if `AssetEvent` failed-load is cheap, swap that entity to the sentinel; otherwise path-missing is the acceptance bar (document unloadable-but-present as later).
- `cargo test -p viewer` must not need a GPU window. Missing authored glb never fails CI.

### C. Time-series charts

Hash-neutral. Do **not** put the ring on `Simulation`. Reuse `InspectorMetrics` (hungry, thirsty, timing.wall_ns) plus living count and mean hunger (World window already computes mean hunger).

| Surface | What |
|---|---|
| Native imgui **Charts** | Window toggle **`C`** and `/charts`. Default on (like World). Last **256** samples. Polyline via imgui draw list — **no** extra crate. |
| Attach page | Session sparklines under `#metrics` from Tick `metrics.inspector` already received. **No** new JS chart library. **No** PROTOCOL bump. |

Series (lock): **wall_ms**, **living**, **hungry**, **thirsty**, **mean hunger**. Cap 256; oldest drops. Sample once per sim tick (same `last_tick_seen` gate as `timing_ring`). Grow or replace the 64-sample `timing_ring` text so Charts and Status mean/max share one buffer.

`--load` / connect: start empty; do not invent history from the checkpoint. Overlay off is N/A (no overlay).

## Out of scope (later)

| Later | What |
|---|---|
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| After M55 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL |
| Not M47 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; mouse-drag orbit; scroll zoom; hashing visuals or chart samples; recipe durability / workstations; DEX defense; INT invent chance |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No chart/sentinel `SimEventKind`.
2. **format_version still writes 3 / reads v2+v3.**
3. Camera / sentinel / charts never enter `state_hash` or AGTN. Idle hashes **unchanged**.
4. **`L` stays legend.** Camera down is **`d`**, not `l`.
5. First pan cancels follow. Console focus (`want_keyboard`) ignores pan keys.
6. Sentinel only when a visual path was **configured**. Empty path stays primitive.
7. Charts consume existing inspector / timing fields. Do not add Tick struct fields.
8. Do not change shipping `coop.toml`. No TLS. Native camera only.

## Tests (M47 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | shipped objects hash `5f231378…`; no-objects `70e5204d…` |
| `pan_step` | tap → 2.0; hold dt=0.25 → 3.0; neither → 0.0 |
| `pan_xz` | right/left/forward/back are ground-plane unit steps; Y not in the helper |
| height clamp | `d` will not go below `terrain + 2.0` |
| follow cancel | applying a non-zero pan step clears follow |
| empty visual | resolve → Primitive |
| configured missing path | resolve → Sentinel, not Primitive |
| authored exists | resolve → Authored |
| LOD miss then missing | Sentinel if any path was set |
| chart ring | push 300 samples → len 256; oldest dropped; wall_ms from `wall_ns` |
| charts not hashed | filling the ring does not change `state_hash` |
| Hello v5 / format 3 | unchanged |

GPU window and live OTLP are **not** required. `cargo test -p viewer` uses fake transforms / paths.

## PR Plan

### PR 1: Camera pan

- **Files:** pan helpers + unit tests; `handle_input` arrows / `u` / `d`; follow cancel; help text (`L` still legend)

### PR 2: Missing-asset sentinel

- **Files:** `resolve_visual` fallback enum; magenta cuboid; spawn path; missing-path tests; log line

### PR 3: Time-series charts

- **Files:** 256-sample ring (wall_ms, living, hungry, thirsty, mean hunger); imgui Charts + `C` / `/charts`; attach-page sparklines; hash identity

## Config / CLI

No shipping experiment TOML change. No new overlay table. No PROTOCOL bump.

On implement: viewer help text + [`docs/cli-reference.md`](cli-reference.md) keys (`arrows`, `u`, `d`, `C`, `/charts`). `L` / `/legend` stay as today.

```bash
cargo test -p viewer
cargo test -p sim-core
cargo run -p viewer -- --connect 127.0.0.1:7000 --objects configs/objects
```

## Verification

Walkthrough: [`docs/M47-test-plan.md`](M47-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: default CLI 2-tick hash `5f231378…`; Hello v5; format_version 3 write; pan/sentinel/chart unit tests without a GPU.

## Risks

- **`L` vs camera:** do not bind camera to `KeyL`. Down is `KeyD`.
- Follow snap fighting pan if cancel is not same-frame.
- Empty `[visual]` must stay primitive or every stem-less object becomes magenta.
- Do not hash camera, sentinel, RSS, ns, or chart samples.
- Do not add Tick postcard fields for sparklines (would force a PROTOCOL bump).
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
