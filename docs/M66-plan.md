# M66 — Per-tick wear, household/invention/downed meshes, extra recipes

**Status:** implemented  
**Depends on:** M65 complete (`docs/M65-plan.md`, git tag `M65`, commit `03ac0cd`)  
**Walkthrough:** [`docs/M66-test-plan.md`](M66-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-2 / RF-1 / RF-PG9

## Context

Tool wear is **dawn-only** (held M61, crates M63), most-worn instance. The viewer has no dedicated household-home / invention / downed object meshes (downed is an M30 cuboid). Catalog after M65 includes beam/scarf/kerb/bun.

M66 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New recipe files **change** shipped-objects idle hashes. `[wear] per_tick` hashes **only when true** (omit = off). Visual-only household/invention/downed files are **not** catalog-hashed. `--no-time` no-objects stays **`70e5204d…`**. Not every-instance decay, not unique authored glbs.

## Goal

A researcher can:

1. **Turn on** per-tick tool wear (`[wear] per_tick = true` / `--per-tick-wear`): each tick, most-worn held then crate instance +1. Dawn energy still runs; **dawn wear is skipped** while per-tick is on. Overlay off ⇒ M65 dawn-only wear. `--no-time` still per-tick if overlay on.
2. **See** hash-neutral viewer meshes: **household** at `household_home`, **invention** at a living inventor, **downed** at an incapacitated agent (replaces the M30 cuboid when the stem resolves). Missing glb ⇒ M47 sentinel.
3. **Craft** four new catalog items (**post, muffler, lintel, scone**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`.

### A. Per-tick wear (RF-2)

```toml
[wear]
per_tick = true    # omit = false
```

CLI: `--per-tick-wear`. Hash `[wear] per_tick` **only when true**.

Each tick after agent steps, before reap: living agents, each held `uses > 0` item, `wear_tool` once; then crates as M63. Most-worn instance only.

When `per_tick`: **do not** run dawn held/crate wear (dawn energy / shelter unchanged). When off: dawn wear as today.

`--load` restores wear; do not extra-wear on decode.

### B. Household / invention / downed meshes (RF-1)

Visual-only object files (no `[sim]` ⇒ not in `hash_catalog`). Create on **implement**:

| id | visual | When |
|---|---|---|
| `household` | reuse house/cabin glb | `household_home` origin cell |
| `invention` | reuse millstone or similar | living inventor’s cell (one mesh per live invention) |
| `downed` | reuse agent glb | `incapacitated` agent; skip M30 downed cuboid if this stem resolves |

Missing path ⇒ sentinel. Native viewer only. Not browser.

### C. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `post` | wood×13 | reuse `wood.glb` |
| `muffler` | fiber×13 | reuse `low_poly_cloth.glb` |
| `lintel` | stone×11 | reuse `stones_and_grass.glb` |
| `scone` | food×10 | reuse `plants_ready.glb` |

No `uses` / `station` / interior. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M66 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Wear leftovers | decay every instance (**RF-3**); per-tick **and** dawn both firing |
| Meshes | unique authored household/invention/downed glbs |
| Also not | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; ammo; millstone foods |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Per-tick **skips** dawn wear so a dawn tick does not double-wear.
4. `[wear] per_tick` omit-hash when false. Recipes still move shipped-objects hashes.
5. Visual-only mesh files have no `[sim]`.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M66 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `bcd36485…` |
| `--no-time` shipped objects | hash `155d4492…` |
| overlay off | dawn wear identity (M61/M63) |
| per-tick on, 1 tick | held axe `[0]` → `[1]`; no dawn needed |
| 8 ticks `uses = 8` | consume 1 |
| per-tick crate | crate axe +1 per tick |
| `--no-time` + per-tick | still wears |
| dawn tick + per-tick | energy refill yes; wear **once** not twice |
| `--load` | no extra wear |
| overlay off idle | hash unchanged from wear field |
| household / invention / downed stems | resolve visual kinds |
| missing downed glb | sentinel |
| Craft post / muffler / lintel / scone | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Per-tick wear

- `[wear] per_tick` / `--per-tick-wear`; skip dawn wear when on; tests

### PR 2: Meshes

- visual-only TOML; viewer spawn; sentinel tests

### PR 3: Recipes

- post/muffler/lintel/scone; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `[wear] per_tick`; `--per-tick-wear`; household/invention/downed TOML; post/muffler/lintel/scone. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M66-test-plan.md`](M66-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `155d4492…`; default (time on) `bcd36485…`; Hello v5; format_version 3 write.

## Risks

- **Double wear.** Dawn + per-tick must not both fire. Skip dawn wear when per-tick on.
- **Catalog hash.** Recipes move idle hashes. Wear overlay omit-hash when false. Visual-only files must not enter catalog.
- **CLI flag.** Same or-with-overlay pattern as `--sheet`.
- **No protocol / format bump.**
