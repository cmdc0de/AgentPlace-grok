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
| **M31** | [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| After M31 | protobuf/TLS; Unix sockets; reflection importance-adjust; hashed pipeline events; **browser client** |
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
