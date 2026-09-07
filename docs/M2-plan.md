# M2 — Restorable experiments (resources, checkpoints, Markdown summaries)

## Context

M1 (commit `ff3d21b`) delivered the spec’s “first picture”:

- Cargo workspace: `sim-core`, `sim-bevy`, `sim-cli`, `viewer`, `shared`
- Hierarchical ChaCha20 seeding and a discrete tick loop
- Integer heightmap world generation
- Agents as `(id, x, y)` with a seeded random walk
- SHA-256 state hashing + same-seed determinism tests
- Headless `sim-cli` and an in-process Bevy viewer (terrain mesh, coloured capsules, follow camera)
- Protocol stubs only (`PROTOCOL_VERSION` + `SnapshotHeader`)

It does **not** yet support the project’s actual experimental loop: *run a baseline, snapshot at tick T, restore, continue bit-identically*. Resource knobs already exist in `configs/default.toml` and `WorldParams` but `World::generate` ignores them. `spawn_mode` other than scattered is unread. Event log is in-memory `Wait`/`Move` only. No save/load.

The specs name this as the next step after the first 3D view:

- `docs/medium-priority-specs.md` §6–7: first picture → **checkpoints + Markdown summaries**
- `docs/simulation-architecture-spec.md` §8: core + seeding + **checkpoints**, then a headless runner that **writes logs + checkpoints**
- `docs/deterministic-seeding-design.md` §4/§6: load checkpoint at T, continue, assert the continuation matches the original continuous run

M2 is that slice. It does **not** add LLM, observation, needs decay, gather/hunt/fish, personality, communication, imgui, or TCP/WebSocket. Those are M3+.

## Goal

A researcher can:

1. Generate a world whose vegetation, minerals, and water match the existing TOML knobs (same seed → same layout).
2. Run N ticks headlessly, writing a versioned binary checkpoint, JSONL event log, and Markdown summaries.
3. Load that checkpoint and continue; `state_hash` at tick T+K matches a continuous run of T+K from the same seed.
4. Open the same checkpoint in the viewer and see terrain, resource markers, and agents.

## Recommended approach

Keep `sim-core` as the authority. Checkpoint I/O, world layers, and event persistence live there. `sim-cli` and `viewer` are thin callers. Do not grow the protocol crate beyond what restore needs — TCP/WebSocket stay stubs.

### 1. World resource layer (fill the M1 hole)

`World` today is a heightmap. M2 adds three deterministic layers generated from the existing `world` RNG stream **after** heights, using `WorldParams.resources`:

| Layer | How it is placed | Viewer |
|---|---|---|
| Fresh water | Height-threshold flood + `water_coverage`, then guarantee `min_fresh_water` cells | Blue cells / shallow tint |
| Vegetation patches | Density scatter, then guarantee `min_vegetation_patches` | Green markers |
| Mineral nodes | Density scatter, then guarantee `min_mineral_nodes` | Grey/brown markers |

Placement rules:

- Same `world` seed + params → identical cells (extend `World::hash_bytes` to cover all layers).
- Minima first, then density fill, as `simulation-and-agents-spec.md` §3 requires. Skip occupied cells.
- Integer cell tags only. No floats in stored world state.
- Animals and fish are **not** in M2 (they belong with needs/hunting in M3). Config comments can note that.

Honor `agents.spawn_mode`:

- `scattered` — current behaviour (land cells only; do not spawn in water).
- `clustered` — pick one land seed cell, spawn remaining agents in a growing neighbourhood.
- `fixed_list` — reject at config load until an explicit spawn list exists (do not silently fall back).

Random walk stays the only behaviour. Agents still do not gather or drink.

### 2. Versioned checkpoint + restore

Envelope (not a raw dump of `Simulation`):

```text
magic        = b"AGTN"
format_version : u32   // M2 = 1
payload        : postcard bytes of CheckpointBody
```

`CheckpointBody` contains everything needed to continue on the same binary/OS:

- `tick`, `master_seed`, SHA-256 of canonical config bytes
- full `World` (height + resource layers)
- agents (current fields; new M3 fields later use `#[serde(default)]`)
- `RngBank` including **ChaCha20 counter/position** (enable `serde` on `rand_chacha`; hashing already clones streams — restore must not re-seed from labels, it must reload stream state)
- in-memory event log (needed for hash identity)
- empty reserved slots: `public_board`, `active_incentives`, `metrics` — so M3/M6 do not force a rewrite of the envelope

Policy from `medium-priority-specs.md` §3:

- Unknown or too-old `format_version` → hard error (`SimError`), never a partial load.
- Markdown summaries are **derived**, not required for restore.
- Checkpoint write failure: retry `checkpoint_write_retries` (default 3), log, **do not halt** the sim.

Add `[checkpoint]` to experiment TOML:

```toml
[checkpoint]
auto_interval_ticks = 500
keep_last_n         = 20
directory           = "checkpoints"
write_markdown_summaries = true
```

Serialization: **postcard** for the body (compact, explicit). Optional JSON sidecar `{tick, config_hash, master_seed, format_version}` for humans. Skip zstd until checkpoints are large.

### 3. Event log + Markdown companions

- Keep the in-memory log as the hash input.
- Also append JSONL `{experiment_id}_events.jsonl` (one object per event) so long runs are inspectable without loading a ckpt.
- On each checkpoint write:
  - `{experiment_id}_tick_{tick}.ckpt`
  - `{experiment_id}_tick_{tick}_summary.md` — tick, seeds, hashes, map size, agent count, water/veg/mineral counts
  - `{experiment_id}_tick_{tick}_agents.md` — one section per agent: id, `(x,y)`, height, cell type
- Summaries are regenerable from a checkpoint (`sim-cli --load PATH --summarize`).

`experiment_id` = short hex of `config_hash` (stable, no clock in the identity).

### 4. CLI and viewer wiring

`sim-cli` (defaults still work as today):

```
sim-cli [--config PATH] [--ticks N] [--quiet]
        [--out-dir DIR]
        [--checkpoint-every K]
        [--load PATH]
        [--summarize]
```

- `--load` restores, then runs `--ticks` more steps (0 ticks = load and print hashes).
- `--out-dir` creates the directory; writes ckpts + markdown + JSONL. If omitted, no files (current behaviour), unless `--checkpoint-every` is set (then default `checkpoints/`).
- Print `world_hash` / `final_hash` as now so existing determinism checks still work.

Viewer:

- `viewer --config PATH` — current path.
- `viewer --load PATH` — restore that checkpoint instead of `Simulation::new`.
- Draw resource markers on the heightmap (instanced cubes or small meshes). No imgui.

### 5. Tests (this is the M2 acceptance bar)

Extend `crates/sim-core/tests/determinism.rs` and add focused unit tests:

| Test | Asserts |
|---|---|
| Same seed → identical resource layers | water/veg/mineral grids equal |
| `world_hash` independent of agent count | still true **and** includes resources |
| Density/minima honored | counts ≥ configured minima |
| Checkpoint round-trip | save at T, load, `state_hash` equal |
| Continuation identity | run T+K vs save at T + load + K → same hash |
| Unknown `format_version` | load returns `SimError` |
| Markdown emitted | both `*_summary.md` and `*_agents.md` exist and contain tick + agent ids |
| Water-safe spawn | no agent starts on a water cell |

`cargo test -p sim-core` and a short `sim-cli` load/continue smoke (`--ticks 20` then `--load` `--ticks 5`) are the verification commands.

## Files / reuse

**Reuse**

- `derive_seed` / `RngBank` / `resolve_seed` in `crates/sim-core/src/seeding.rs`
- `World::generate` + `hash_bytes` in `crates/sim-core/src/world.rs`
- `Simulation::state_hash` / `tick` / `spawn_agents` in `crates/sim-core/src/simulation.rs`
- `ExperimentConfig` + `WorldParams.resources` in `crates/sim-core/src/config.rs` (already deserialized, unused)
- `heightmap_mesh` / `agent_world_pos` in `crates/viewer/src/render.rs`
- `SimPlugin` / `SimState` in `crates/sim-bevy/src/lib.rs`

**Touch**

- `crates/sim-core/src/world.rs` — resource grids, placement, hash
- `crates/sim-core/src/config.rs` — `[checkpoint]`, reject `fixed_list` without list, spawn clustered
- `crates/sim-core/src/simulation.rs` — land-only spawn, clustered spawn, hash includes resources
- `crates/sim-core/src/checkpoint.rs` — **new**: envelope, save/load, markdown, JSONL
- `crates/sim-core/src/event_log.rs` — serde on events
- `crates/sim-core/src/seeding.rs` — serde on `RngBank` streams
- `crates/sim-core/src/lib.rs` — export checkpoint API
- `crates/sim-core/Cargo.toml` — `postcard`, `rand_chacha` serde feature
- `crates/sim-core/tests/determinism.rs` + new `crates/sim-core/tests/checkpoint.rs`
- `crates/sim-cli/src/main.rs` — `--load` / `--out-dir` / `--checkpoint-every` / `--summarize`
- `crates/sim-bevy/src/lib.rs` — construct `SimState` from a loaded `Simulation`
- `crates/viewer/src/main.rs` + `render.rs` — `--load`, resource meshes
- `configs/default.toml` — `[checkpoint]` block
- `Cargo.toml` — workspace dep `postcard`

**Do not touch in M2:** LLM client, imgui, `shared::transport` TCP/WS implementations, personality/memory/goals types, action vocabulary beyond `Wait`/`Move`.

## Key decisions

1. **M2 is restore, not the survival loop.** Checkpoints are the experimental backbone. Needs, gather, observation, and LLM are M3 so this milestone stays reviewable and so `format_version = 1` already includes the resource world those systems will act on.
2. **Postcard + magic/version header**, not raw `bincode` of `Simulation`. Lets us reject old files cleanly and add `#[serde(default)]` fields later without a parser rewrite.
3. **Reload RNG stream state, do not re-derive seeds.** Re-seeding from labels would restart each ChaCha20 stream and break continuation identity.
4. **Resources are cell tags, not entities.** Matches the heightmap, stays integer, hashes cheaply. Animals/fish wait for M3.
5. **Markdown is derived.** Restore never depends on `.md` files.

## Out of scope (later milestones)

M3–M39 are done. **Current next slice:** [`M40-plan.md`](M40-plan.md) (config-owned objects + recipes except agent). Later work is listed at the bottom of that file.

## PR Plan

### PR 1: Deterministic resource layers and land-only spawn

- **Files:** `crates/sim-core/src/world.rs`, `config.rs`, `simulation.rs`, `tests/determinism.rs`, `crates/viewer/src/render.rs`, `main.rs`
- **Dependencies:** none
- **Changes:** Generate water/vegetation/minerals from existing resource params; include them in `world_hash`; spawn on land; implement `clustered`; reject unimplemented `fixed_list`; draw resource markers in the viewer.

### PR 2: Checkpoint envelope and continuation identity

- **Files:** `crates/sim-core/src/checkpoint.rs` (new), `event_log.rs`, `seeding.rs`, `lib.rs`, `Cargo.toml`, `tests/checkpoint.rs` (new)
- **Dependencies:** PR 1
- **Changes:** Magic + `format_version` + postcard body; serialize RNG positions; `Simulation::save` / `load`; tests for round-trip, T+K identity, unknown version rejection.

### PR 3: Durable run artifacts (CLI, Markdown, JSONL, viewer load)

- **Files:** `crates/sim-cli/src/main.rs`, `crates/sim-core/src/checkpoint.rs`, `crates/sim-core/src/config.rs`, `crates/sim-bevy/src/lib.rs`, `crates/viewer/src/main.rs`, `configs/default.toml`
- **Dependencies:** PR 2
- **Changes:** `--out-dir` / `--checkpoint-every` / `--load` / `--summarize`; JSONL event log; Markdown companions; write-failure retries; viewer `--load`; default `[checkpoint]` in config.

## Verification

- `cargo test -p sim-core` — includes continuation-identity test (the M2 gate).
- `cargo run -p sim-cli -- --config configs/default.toml --ticks 50 --out-dir /tmp/m2-run --checkpoint-every 25` then `--load` the tick-25 ckpt with `--ticks 25` and confirm `final_hash` matches a continuous 50-tick run.
- `cargo run -p viewer -- --load <ckpt>` — terrain, resource markers, agents at restored positions; Space / `.` / F still work.
- Confirm a truncated/garbage file and a bumped `format_version` both fail loudly.

No browser verification (native Bevy). Viewer check is a local windowed run.

## Risks

- **ChaCha20 serde:** if `rand_chacha` 0.9 serde does not snapshot counter state correctly, implement an explicit `(seed, bytes_consumed)` wrapper around each stream and restore by `from_seed` + discard. Test this first in PR 2.
- **f64 in height generation** already exists in M1; M2 must not add new float-in-hash fields. Resource placement uses integer cell indices only.
- **Event log unbounded growth** in the in-memory vector. Acceptable for M2 (runs of 10^4–10^5 ticks). Truncation / offset lives with metrics in a later milestone; checkpoint already records the full log so hashes stay honest.
