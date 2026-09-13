# M33 — Household crates, culture inheritance, reflect importance

**Status:** implemented  
**Depends on:** M32 complete (`docs/M32-plan.md`, git tag `M32`, commit `7b14400`)  
**Walkthrough:** [`docs/M33-test-plan.md`](M33-test-plan.md)  
**Specs:** `docs/M32-plan.md` (household), `docs/M31-plan.md` (birth), `docs/M25-plan.md` (reflect-on-evict / extra LLM call)

## Context

M32 minted household ids but members still use only the cell they stand on for Store/Retrieve. There is no culture tag. Extra LLM calls exist for insight/plan/evict, not for rewriting memory importance.

M33 **does not** bump `PROTOCOL_VERSION` (stays **5**). Overlay is not `ExperimentConfig`. No TLS/protobuf. No browser. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on overlay `[population] household_crates = true` (or `--household-crates`) so household members Store/Retrieve at the household **home cell** (minted on PairBond). Overlay off / no household ⇒ **same hashes**.
2. Turn on `[population] culture = true` (or `--culture`) so founders get a culture id from `agent_init` and children **copy** a parent. Overlay off ⇒ id 0, not hashed, idle hash still `70e5204d…`.
3. Turn on `[llm] reflect_importance = true` (or `--llm-reflect-importance`) for one extra LLM call after retrieve that may rewrite a memory’s importance. **Mock skip.** Custom + replay use call `"importance"`.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged.

## In scope

### A. Household crates

Overlay, not `ExperimentConfig`:

```toml
[population]
household_crates = true
```

CLI: `--household-crates` (does **not** imply `--reproduction`; no home until a pair-bond exists).

- On PairBond, if the household has no home yet: home = actor’s land cell. Checkpoint `household_home: BTreeMap<u64,(u32,u32)>` in the board blob. Hash only when non-empty.
- Overlay on: living members may **Store/Retrieve** when Chebyshev ≤ 1 of that home (existing per-cell crate). Overlay off: today’s crate rules only.
- Inspector / Observation: `home (x,y)` when set. No new crate type.

### B. Culture

```toml
[population]
culture = true
culture_count = 4          # omit = 4; founder ids 1..=count
```

CLI: `--culture`.

- `Agent.culture: u8`, 0 = unused, skip + BoardBlob. Hash only if ≠ 0.
- Overlay on: founders `1 + (stream % culture_count)` from `derive_seed(agent_init, "culture_{id}")` if still 0 (no re-stamp on `--load`).
- Birth: child copies parent A’s culture (lower id), or the only parent that has one.
- Inspector / Observation: `culture N` when ≠ 0.

### C. Reflect importance

```toml
[llm]
reflect_importance = true
```

CLI: `--llm-reflect-importance`.

- After retrieve, if overlay on and chooser is **Custom**: one call `importance` (replay key `"importance"`, seed family `tick_{t}_agent_{id}_importance_0`). JSON `{"id": <memory id>, "importance": 0-255}` writes that slot; skip/err leaves memories unchanged.
- Mock/Wait **skip** (same hash as overlay off). Record/replay like reflect-on-evict.
- Do **not** add a hashed pipeline event.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M52** | [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| After M52 | protobuf/TLS; Unix sockets; dialects; seasons; sql.js / ad-hoc SQL; weapon range; spear melee |
| Not M33 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Replay call string is not a postcard wire variant.
2. `--household-crates` does not imply `--reproduction`. Empty home map is not hashed.
3. Culture 0 / unused is not hashed. `--load` does not re-roll founders.
4. Mock skips the extra LLM call. Custom importance writes are hashed (memory importance).
5. Do not change shipping `coop.toml`. `format_version = 2`.

## Tests (M33 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| household_crates off / no pair-bond | same hash as today |
| pair-bond + overlay | both share a home cell; member can Store there; outsider cannot via this rule |
| `--load` | home map restored; no re-mint |
| culture off | culture 0; same hash |
| `--culture` | founders 1..=count; hash ≠ off; child copies parent; `--load` no re-roll |
| reflect_importance + mock | same hash as overlay off |
| Custom importance JSON | named memory importance changes; record+replay match |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Household crates

- **Files:** overlay / `--household-crates`; home on PairBond; Store/Retrieve range; blob + hash

### PR 2: Culture

- **Files:** overlay / `--culture`; founder assign; child copy; inspector/Observation

### PR 3: Importance call

- **Files:** `[llm] reflect_importance` / `--llm-reflect-importance`; replay `"importance"`; mock skip; Custom write

## Config / CLI

No shipping TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --household-crates --culture \
  --llm-reflect-importance --quiet
```

## Verification

Walkthrough: [`docs/M33-test-plan.md`](M33-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged with overlays off; household home Store; culture inherit; importance mock-skip; Hello stays v5.

## Risks

- Overlay is **not** checkpointed; home/culture **are**. `--load` must not re-mint or re-roll.
- Mock must skip the extra LLM call.
- Empty home map / culture 0 add **no** hash bytes.
- Household crates must not change legal Store when overlay is off.
- **No PROTOCOL bump.** Stay at 5.
