# M38 — Viewer 3D models, browser /inject /scrub + wasm32 CI

**Status:** implemented  
**Depends on:** M37 complete (`docs/M37-plan.md`, git tag `M37`, commit `d565d66`)  
**Walkthrough:** [`docs/M38-test-plan.md`](M38-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-6, PG-7 later), `docs/M8-plan.md` (`InjectIncentive`), `docs/M21-plan.md` (`Scrub`), `docs/M37-plan.md` (optional wasm)

## Context

The 3D view is primitives + colour (capsules, cubes, distinct `Mesh` kinds). M37 `web/index.html` has `/set` `/give` and optional `import("./pkg/sim_wasm.js")`; Ubuntu CI does not build wasm32. `/inject` and `/scrub` exist on the wire (`InjectIncentive`, `ControlVerb::Scrub`) but not in the page.

M38 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb`. Models are not hashed. No TLS/`wss`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Open the imgui viewer and read agents, food/veg, and items as **authored models** (glTF/glb), not only capsules and cubes. Missing file ⇒ today’s primitive. Same `state_hash`.
2. From `web/index.html` with `--allow-control`, **inject** a schedule TOML and **scrub** to a tick.
3. After Snapshot, fill tables from checkpoint bytes via a **wasm** decode when the module is present. Ubuntu CI **builds** that wasm target. Native `cargo test` still works without it.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**.

## In scope

### A. Viewer 3D models (hash-neutral)

Asset root: `assets/models/` (or `crates/viewer/assets/models/`). One glTF/glb per kind. **Not** in `state_hash` or checkpoints.

| Kind | File stem (lock) |
|---|---|
| Agent | `agent` |
| Veg | one file per veg species tag already in the legend (berry ≠ herb) |
| Animals / fish | `hare`, `perch` |
| Crops | `crop` |
| `ItemId` | `wood`, `fiber`, `stone`, `basket`, `spear`, `fishing_rod`, `backpack` |
| Crate | `crate` |

- Load helper: if the file exists, spawn that mesh/scene; else `mesh_for_shape` / capsule / cuboid as today.
- Fog still hides what Observation cannot see. Optional CON/sheet scale stays hash-neutral.
- `cargo test -p viewer` must **not** download `.glb` and must **not** require a GPU window. Unit-test path map + missing-file fallback.
- Do not ship skeletal animation, photogrammetry, per-agent clothing, household-home / invention / downed poses.

### B. Browser `/inject` `/scrub` + wasm32 CI

Existing wire. **No PROTOCOL bump.**

- Page: textarea (or file) → `ClientMessage::InjectIncentive { schedule_toml }` (variant 4 + string). `/scrub TICK` → `Control(Scrub(u64))` (Control tag 3, verb 6). Document postcard bytes in shared tests.
- Server already requires `--allow-control`. Without flag → `ControlDisabled`. Do not add `/ckpt` `/events` this slice.
- New thin crate **`sim-wasm`** wrapping `InspectorView::from_checkpoint_bytes` → JSON string. Page already `import("./pkg/sim_wasm.js")`.
- **Ubuntu CI** adds `wasm32-unknown-unknown` and builds that crate (wasm-bindgen or equivalent; no network fetch of models). Windows/mac matrix stays native mock tests only.
- Native `cargo test -p sim-core` / `-p viewer` never needs the wasm target. Missing wasm at runtime ⇒ Tick `metrics.inspector` still fills tables.

## Out of scope (later)

| Later | What |
|---|---|
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | Done — [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| **M60** | [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| After M60 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; household-home / invention / downed meshes; wasm32 on Win/mac; sql.js / ad-hoc SQL |
| Not M38 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** InjectIncentive and Scrub already exist.
2. Models never enter `state_hash` or checkpoints. Missing file ⇒ primitive.
3. wasm32 is required on **Ubuntu CI only**, not on every native `cargo test`.
4. `/inject` `/scrub` are hash-sensitive; read-only attach stays hash-neutral.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M38 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| models hash-neutral | with vs without assets, same `state_hash` |
| missing glb | fallback to primitive; `cargo test -p viewer` passes |
| path map | each locked stem maps to a kind (unit) |
| page | PROTOCOL 5; `encodeInject` / `encodeScrub`; wasm import stays |
| `/inject` `/scrub` | `--allow-control` mutates / jumps; without flag `ControlDisabled` |
| Snapshot wasm helper | native `from_checkpoint_bytes` still matches `from_sim`; Ubuntu wasm build exists |
| Hello v5 | unchanged |
| attach | hash-neutral until inject/scrub |

## PR Plan

### PR 1: Viewer models

- **Files:** asset layout + load-or-fallback; path-map tests; hash-neutral

### PR 2: Browser inject/scrub + wasm CI

- **Files:** page forms; postcard byte tests; `sim-wasm`; Ubuntu workflow wasm32 step; loopback net tests

## Config / CLI

No shipping TOML change. No new overlay. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
# open web/index.html → Connect; /inject /scrub if --allow-control
cargo run -p viewer -- --config configs/default.toml
```

## Verification

Walkthrough: [`docs/M38-test-plan.md`](M38-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash unchanged; missing glb falls back; page still v5; inject/scrub ControlDisabled without flag.

## Risks

- **Models never hashed.** Do not put glb bytes in checkpoints.
- Fallback if a file is missing so headless CI without GPU art still passes.
- **No PROTOCOL bump.** InjectIncentive and Scrub already exist.
- wasm32 only required on Ubuntu CI, not on every `cargo test`.
- `/inject` `/scrub` are hash-sensitive; read-only attach must stay hash-neutral.
- Do not fetch art or wasm from the network in tests.
