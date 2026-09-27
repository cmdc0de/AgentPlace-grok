# M65 — Interior blocked Move, animation monikers, extra recipes

**Status:** implemented  
**Depends on:** M64 complete (`docs/M64-plan.md`, git tag `M64`, commit `d51c03f`)  
**Walkthrough:** [`docs/M65-test-plan.md`](M65-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-18 / RF-20 / RF-PG9

## Context

Sleep footprints are fully walkable (M64 W×H). Move only checks bounds + water. `[visual.animations]` has **idle** only; Move pauses idle. Catalog after M64 includes pole/cape/paver/tart.

M65 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New recipe files **change** shipped-objects idle hashes. `interior` hashes **only when true**; shipping tent/cabin/house stay false. Animation monikers are `[visual]` ⇒ **hash-neutral**. `--no-time` no-objects stays **`70e5204d…`**. Not wall meshes, not new authored walk clips, not ammo.

## Goal

A researcher can:

1. **Place** a catalog sleep item with `interior = true` and have **Move from outside onto the footprint fail**, except onto the **origin cell (door)**. Move **inside** the footprint and **exit** any edge stay legal. Tent/cabin/house stay enter-anywhere.
2. **Map** standard animation monikers in `[visual.animations]` (`walk`, `melee`, `ranged`, `flee`, `downed`, `death`; `idle` already M64) to glb clip names. Omit a key ⇒ no clip for that action. Missing clip in the glb ⇒ static, not sentinel. Hash-neutral. Native window only. **No new authored clips.**
3. **Craft** four new catalog items (**beam, scarf, kerb, bun**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Place / Craft unless tests `execute_primary`. No new CLI flag.

### A. Interior blocked Move (RF-18)

Optional `[sim] interior = true` (omit = false). Stations stay 1×1. Door = stored origin (min x,y).

```toml
[sim]
interior = true
sleep_size = 3
```

- Enter from outside **only** onto origin. Other footprint cells: enter only from a cell of the **same** footprint.
- Exit: footprint cell → uncovered cell = legal. Inside: Move between cells of the same footprint = legal.
- `hash_catalog`: hash `interior` **only when true**. Do **not** change `tent.toml` / `cabin.toml` / `house.toml`.
- Tests use a **fixture** 3×3 interior object (not a new shipped `configs/objects/` file).
- Place / Pickup / dawn unchanged. `legal_actions` omits MoveRelative whose dest would Wait.
- `--load` restores origin; do not re-Place. `--no-time` still blocks (not a clock feature).

Not this slice: wall meshes, door-on-a-side enum, blocking stations, household-only entry.

### B. Animation monikers (RF-20)

M64: `AnimDef.idle` only; Move pauses idle. Closed keys (omit = no clip):

| Key | When |
|---|---|
| `idle` | no higher-priority action (M64) |
| `walk` | `Move` this tick |
| `melee` | `Attack` Chebyshev == 1 |
| `ranged` | `Attack` Chebyshev > 1 |
| `flee` | `Flee` |
| `downed` | `Incapacitated` |
| `death` | `CombatDeath` |

Priority: **death > downed > flee > melee/ranged > walk > idle**. Unknown keys ignored. Empty / missing clip ⇒ static for that action (walk still pauses idle if no walk clip). Agent stem; same struct on any object TOML. Do **not** add fake clip names to shipping `agent.toml` (keep `idle = "ArmatureAction.002"` only). Tests use a fixture `AnimDef`. No new glb files. `[visual]` is not hashed.

### C. Extra recipes (RF-PG9)

Object TOML only. No new Craft verb. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `beam` | wood×12 | reuse `wood.glb` |
| `scarf` | fiber×12 | reuse `low_poly_cloth.glb` |
| `kerb` | stone×10 | reuse `stones_and_grass.glb` |
| `bun` | food×9 | reuse `plants_ready.glb` |

No `uses` / `station` / interior. Catalog-off / empty catalog ⇒ Craft illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M65 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Interior leftovers | wall meshes; door side other than origin (**RF-21**) |
| Extra clips | new authored walk/melee glb files |
| Also not | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; RF-1 meshes; ammo; per-tick decay |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. `interior` **omit-hash when false**. Recipes still move shipped-objects hashes.
4. Door is origin only. Place already stands on origin.
5. `legal_actions` and `move_rel` agree on blocked dest.
6. Do not map absent clip names on shipping `agent.toml`.
7. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M65 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `80ae9571…` |
| `--no-time` shipped objects | hash `ceb0fcc0…` |
| cabin/house | still enter-anywhere |
| fixture 3×3 interior | outside → non-origin Wait; outside → origin ok |
| inside 3×3 | Move to other footprint cell ok; exit ok |
| `legal_actions` | blocked dest not listed |
| `--load` interior | origin restored; same block rules |
| `--no-time` interior | still blocks |
| moniker helper | TOML walk/melee/… names; omit walk ⇒ none |
| missing clip | static, not sentinel |
| idle | still plays when no Move (M64 identity) |
| Craft beam / scarf / kerb / bun | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Interior Move

- `interior` catalog field; omit-hash; fixture 3×3; `move_rel` + `legal_actions`

### PR 2: Animation monikers

- `AnimDef` keys; viewer priority picker; fixture mapping tests

### PR 3: Recipes

- beam/scarf/kerb/bun TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag.

On implement: `[sim] interior`; `AnimDef` walk/melee/ranged/flee/downed/death; create beam/scarf/kerb/bun TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test sleep
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M65-test-plan.md`](M65-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `ceb0fcc0…`; default (time on) `80ae9571…`; Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** New recipe files move idle hashes. `interior` must omit-hash when false.
- **Door.** Origin-only entry; Place already stands on origin.
- **legal_actions vs execute.** Both must agree or mock Walk-spams Wait.
- **Priority.** death > downed > flee > melee/ranged > walk > idle.
- **No fake clips** on shipping `agent.toml`.
- **No protocol / format bump.**
