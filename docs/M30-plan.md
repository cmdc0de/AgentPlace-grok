# M30 — Combat particles / meshes

**Status:** implemented  
**Depends on:** M29 complete (`docs/M29-plan.md`, git tag `M29`, commit `7b3a3bf`)  
**Walkthrough:** [`M30-test-plan.md`](M30-test-plan.md)  
**Specs:** `medium-priority-specs.md` §2 (physical combat v2-ready), `M28-plan.md` (capsule tints, no meshes), `M29-plan.md` (`CombatDeath` tag 28)

## Context

M28 tints attacker/defender/flee capsules and prints a HUD line from this tick’s `Attack` / `Flee`. There are **no** strike meshes or particles. M29 can **remove** an agent (`CombatDeath`) with no 3D cue beyond the missing capsule.

M30 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new overlay, CLI flag, event tag, or `ExperimentConfig` field. No TLS/protobuf. No browser.

## Goal

A researcher can:

1. **See** an Attack this tick as a short-lived strike mesh between attacker and defender (M28 capsule tints stay).
2. **See** Flee as a distinct cue; Incapacitated / CombatDeath as a downed or “gone” mesh/flash (not only the Status line).
3. Rely on **hash-neutral** FX: overlay off / mock idle still `70e5204d…` at 2 ticks. No sim change.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged.

## In scope

### A. Combat FX jobs (sim-core, hash-neutral)

Extend `combat_fx` with **jobs** derived from this tick’s events (no Bevy types):

| Event | Job |
|---|---|
| `Attack { target, damage }` | strike attacker → defender |
| `Flee` | flee cue on that agent |
| `Incapacitated { by }` | downed cue on the victim |
| `CombatDeath { by }` | death cue on the victim |

- Empty event list this tick ⇒ no jobs.
- Unit-testable without a window. **Not** hashed. **Not** checkpointed.

Keep existing `combat_role` tints. Add HUD lines for incapacitate/death (same helper family as `combat_hud_line`).

### B. Viewer meshes

Spawn/despawn **Bevy meshes** (simple primitives / short-lived particles) from those jobs:

- Strike: a thin primitive between A and B world positions.
- Flee / downed / death: a distinct primitive or flash on the agent (or last cell, if already removed).
- Despawn when this tick no longer has that event (or after a short frame lifetime).
- Fog: only if the agent is visible in Observation (same as satchels).
- Attach: FX from Snapshot events for the **painted** tick only (same as tints).

**No** new overlay, CLI flag, `ControlVerb`, event tag, or `ExperimentConfig` field. Conflict overlay already gates the events.

## Out of scope (later)

| Later | What |
|---|---|
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
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
| **M55** | [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| After M55 | protobuf/TLS; Unix sockets; sql.js / ad-hoc SQL |
| Not M30 | PROTOCOL bump; PG sheet; reproduction; kinship |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard wire variants.
2. Particles **must not** enter `state_hash` or checkpoints.
3. Jobs live in `sim-core` so CI can test them without a GPU or imgui window.
4. Keep M28 tints. This slice **adds** meshes, it does not replace colours.
5. Do not change shipping `configs/default.toml` / `coop.toml`. `format_version = 2`.

## Tests (M30 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| Attack event this tick | fx helper emits strike A→B |
| Flee / Incapacitated / CombatDeath | matching job; no Attack ⇒ no strike |
| no combat events | empty jobs |
| Hello v5 | unchanged |
| `cargo test -p sim-core` / `-p viewer` | helper tests; no GPU/imgui window required |

## PR Plan

### PR 1: FX jobs

- **Files:** `crates/sim-core/src/combat_fx.rs`; combat tests for jobs + HUD lines

### PR 2: Viewer meshes

- **Files:** `crates/viewer/src/main.rs` (spawn/despawn from jobs); HUD uses new lines

## Config / CLI

No shipping TOML change. No new overlay or `--flag`. Use existing `--conflict` / `--conflict-death` to produce the events the meshes follow. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --conflict --conflict-death --quiet
```

## Verification

Walkthrough: written on implement (`docs/M30-test-plan.md`).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged; jobs match events; Hello stays v5; no window required for tests.

## Risks

- **Particles in `state_hash`** — never; jobs are viewer-only.
- **Postcard enum append** — do not add variants. PROTOCOL stays 5.
- **Viewer tests on Win/mac CI** — no imgui window; jobs tested in sim-core. GPU particles are not CI-driven.
- **Removed agent (CombatDeath)** — death cue must use last known cell; do not panic if the capsule is already gone.
- **No PROTOCOL bump.** Stay at 5.
