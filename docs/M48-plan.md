# M48 — INT invent quality, agent meshes, hot-reload glb

**Status:** implemented  
**Depends on:** M47 complete (`docs/M47-plan.md`, git tag `M47`, commit `49e2795`)  
**Walkthrough:** [`docs/M48-test-plan.md`](M48-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 INT quality, PG-6 agent meshes); hot-reload parked on PG-6 / After-M later-tables

## Context

M35 already ships Invent **chance** `400 + INT_mod*50` vs `rng % 1000`. Successful Invent always adds **`INVENTOR_INFLUENCE = 200`** (food 1.2× unchanged). Viewer agents are still the Bevy **capsule** unless `assets/models/agent.glb` exists (stem-only; M40 left agent out of object TOML). Authored glbs load once at spawn; changing a file on disk does nothing until restart. `L` is legend; camera pan is M47.

M48 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Visuals and reload are **hash-neutral**. INT quality is hashed only when Invent actually succeeds with INT ≠ 0.

## Goal

A researcher can:

1. Turn on `[inventions]` / `--inventions` plus `[agents.sheet]` / `--sheet` and see a high-INT inventor get a **larger influence gain** on success. INT 0 ⇒ today’s **+200**. Chance formula **unchanged**.
2. Drop `configs/objects/agent.toml` (visual-only) so the native viewer draws an authored **human** glb instead of the capsule. Empty / omitted visual ⇒ capsule. Configured missing path ⇒ M47 **sentinel**.
3. Edit a loaded `.glb` on disk and see the native viewer **reload** it (~0.5 s poll) without restarting. Log `glb reload {id} {path}`.
4. Idle mock hashes **unchanged**. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Default CLI 2-tick hash stays **`5f231378…`** (shipped objects) / **`70e5204d…`** (no catalog).

## In scope

No PROTOCOL bump. No new `SimEventKind`. No new ControlVerb. Do not store derived influence on the sheet. Do not put reload/mtime on `Simulation` / AGTN.

### A. INT invent quality (PG-4 leftover)

Chance stays `invent_chance` (M35). Food 1.2× unchanged. Mock still never picks Invent. `--inventions` does not imply `--sheet`.

| INT | Influence gain on successful Invent |
|---|---|
| 0 (unused) | **200** (`INVENTOR_INFLUENCE`) |
| ≠ 0 | `max(1, 200 + INT_mod * 50)` — INT 18 → **400**; INT 3 → **50** |

- Invent-style `derive_seed` already used for the chance roll; quality is **derived at apply time**, not stored.
- `--load` restores influence_factor as written; do not add quality again on decode.
- Overlay off ⇒ Invent illegal; idle hash **unchanged**.

### B. Agent meshes (PG-6 leftover)

Create **on implement** (visual only, no `[sim]`):

```toml
# configs/objects/agent.toml
id = "agent"
kind = "agent"
[visual]
glb = "assets/models/optimized/low_poly_human_character.glb"
```

- Viewer resolves `agent` like other ids. Stem fallback remains only if there is **no** configured visual.
- Authored file ⇒ glb (hash-neutral). Empty / omitted `[visual]` ⇒ today’s **capsule**. Configured missing ⇒ **sentinel**.
- Keep per-agent `AgentVisual` / follow / fog. Do not require per-agent clothing or skeletal clips.
- `cargo test -p viewer` must not need a GPU window. Missing glb never fails CI.
- Visual-only TOML must **not** change `state_hash`.

### C. Hot-reload glb

Native Bevy window only. No extra Bevy `file_watcher` feature required (CI stays simple).

- Poll **mtime** of each spawned `ModelLabel.path` every **0.5 s**.
- If mtime increased: `AssetServer.reload` that path (and/or respawn that entity). Log `glb reload {id} {path}` (hash-neutral, first per path per change).
- Ignore when the file is gone (keep last mesh; missing-at-resolve is still sentinel at spawn).
- Object TOML path edits: if a def’s `glb` string changes on a later poll of the objects dir, re-resolve that id and respawn. Optional if mtime-of-glb is enough for the walkthrough; **glb mtime is the acceptance bar**.
- Unit-test `should_reload(prev_mtime, new_mtime) -> bool` with fake times. No GPU.

## Out of scope (later)

| Later | What |
|---|---|
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| After M53 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; weapon range; spear melee |
| Not M48 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; mouse-drag orbit; scroll zoom; DEX defense (already M41); Invent **chance** formula (already M35); recipe durability / workstations |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb / `SimEventKind`.
2. **format_version still writes 3 / reads v2+v3.**
3. Invent **chance** unchanged. Quality is influence gain only. Food 1.2× unchanged.
4. INT 0 ⇒ +200 identity. Overlay off ⇒ idle hashes **unchanged**.
5. Agent TOML is visual-only. Stem fallback stays for no-visual agent.
6. Hot-reload is mtime poll, not a hashed event. Native window only.
7. Do not change shipping `coop.toml`. No TLS. No skeletal animation.

## Tests (M48 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | shipped objects hash `5f231378…`; no-objects `70e5204d…` |
| unused INT invent | influence **+200**; chance 400 |
| INT 18 vs 3 invent | quality 400 vs 50; chance still 600 vs 250 |
| inventions off | Invent illegal; same idle hash |
| `--load` | influence not double-applied |
| agent empty visual | resolve → Primitive (capsule) |
| agent.toml authored | resolve → Authored `low_poly_human_character.glb` |
| agent configured missing | resolve → Sentinel |
| agent visual not hashed | loading agent.toml does not change `state_hash` |
| `should_reload` | newer mtime true; equal/older false |
| Hello v5 / format 3 | unchanged |

GPU window is **not** required. `cargo test -p viewer` uses fake paths / mtimes.

## PR Plan

### PR 1: INT invent quality

- **Files:** `inventor_influence(int)`; execute apply; unused + 18 vs 3 tests; load identity

### PR 2: Agent mesh TOML

- **Files:** `configs/objects/agent.toml`; resolve tests; hash identity

### PR 3: Hot-reload glb

- **Files:** mtime helper + poll; reload log; unit tests without GPU

## Config / CLI

No shipping experiment TOML change. No new overlay table. No PROTOCOL bump.

On implement: `configs/objects/agent.toml` (visual only); viewer help / [`docs/cli-reference.md`](cli-reference.md) note that agent glb comes from object TOML and that a disk change reloads.

```bash
cargo test -p sim-core --test inventions
cargo test -p viewer
cargo run -p viewer -- --connect 127.0.0.1:7000 --objects configs/objects
```

## Verification

Walkthrough: [`docs/M48-test-plan.md`](M48-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: default CLI 2-tick hash `5f231378…`; Hello v5; format_version 3 write; invent quality / agent resolve / reload unit tests without a GPU.

## Risks

- Do not change Invent **chance** (M35 tests `400` / `600`).
- `--load` must not add quality influence twice.
- Agent `[sim]` would change catalog hashes — keep **visual-only**.
- Empty agent visual must stay capsule, not sentinel.
- Do not hash mtime / reload logs / glb bytes.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
