# M73 — Arrow ammo, interior door side + wall meshes, extra recipes

**Status:** implemented  
**Depends on:** M72 complete (`docs/M72-plan.md`, git tag `M72`, commit `32ca861`)  
**Walkthrough:** [`docs/M73-test-plan.md`](M73-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-24 / RF-21 / RF-PG9

## Context

M69 bow and sling consume **1 stone** on Attack Chebyshev > 1. M65 `interior = true` lets an agent enter a sleep footprint only onto the **origin cell**. There are no wall meshes. Catalog after M72 includes stud/glove/dado/pikelet.

M73 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Bow/sling `ammo` slug change, the new `arrow` item, and the four recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. `door` omit-hash when absent. Wall meshes are viewer-only and **hash-neutral**. Not a separate bolt, not dual-station, not shipping tent/cabin/house as interior.

## Goal

A researcher can:

1. **Craft an arrow** and have bow and sling ranged Attacks (Chebyshev **> 1**) consume **1 arrow** instead of 1 stone. No arrow ⇒ Attack illegal (Wait). Dist 1 never consumes. Spear still has no ammo.
2. **Set** `[sim] door` on an interior sleep item so the door is a **side** of the footprint (`n` / `e` / `s` / `w`). Omit stays M65: enter from outside only onto the origin cell. **See** a wall mesh on each perimeter cell that is not a door cell (reuse `fence` / `wood.glb`). Tent/cabin/house stay non-interior.
3. **Craft** four new catalog items (**purlin, gaiter, cornice, bagel**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather / Attack / Place unless tests `execute_primary`. No new CLI flag. No new authored glb.

### A. Arrow ammo (RF-24)

One new catalog item. Create on **implement**:

| id | inputs | visual | notes |
|---|---|---|---|
| `arrow` | wood×1 | reuse `wood.glb` | weight small; no `uses` / `attack_*` / `station` / `interior` |

On **implement**, set `ammo = "arrow"` on `bow.toml` and `sling.toml` (replace `"stone"`). **Not** on `spear.toml`.

Who/when: unchanged from M69. Attack Chebyshev **> 1**. Weapon = held catalog item with max `attack_range` among those with `attack_range >= dist`. If that item’s `ammo` slug is non-empty, require `held_qty(ammo) >= 1`; consume 1 (pockets then pack) **after** energy pay, hit or miss. Dist 1: never consume.

`legal_actions`: omit Attack when that ammo check would fail.

Catalog-off: no ammo field ⇒ unarmed range 1, never consume. Holding stone does **not** satisfy bow/sling.

Hash `[sim] ammo` **only when non-empty** (already). Slug change stone→arrow **and** the new `arrow` `[sim]` **move** shipped idle hashes.

`--load` restores inventory; do not extra-consume on decode.

Implement **updates combat tests** that assert stone consume so they assert arrow. Do **not** rewrite `docs/M69-test-plan.md`.

Not: a separate bolt item; ammo on spear; consume on melee; quiver capacity.

### B. Door side + wall meshes (RF-21)

```toml
[sim]
interior = true
door = "n"    # n | e | s | w; omit = origin cell only
```

Footprint origin `(ox, oy)`, size `w`×`h` (same `sleep_w` / `sleep_h` / `sleep_size` as today):

| `door` | Door cells (enter from outside is legal) |
|---|---|
| omit / unknown | `(ox, oy)` only (M65) |
| `s` | south edge `y == oy` |
| `n` | north edge `y == oy + h - 1` |
| `w` | west edge `x == ox` |
| `e` | east edge `x == ox + w - 1` |

Outside → non-door footprint cell: Wait. Already inside the same footprint: Move stays legal, including onto walls and out any edge. Exit any edge stays legal.

Hash `door` **only** when it is `n`/`e`/`s`/`w`, one distinct tag byte each (`n=1, e=2, s=3, w=4` — not the interior `[1]` byte and not ammo slug bytes). Omit / unknown ⇒ no door bytes. `interior` still hashes only when true.

Do **not** set `interior` or `door` on shipping `tent.toml` / `cabin.toml` / `house.toml`. Tests use a **fixture** 3×3 (same as M65), not a new shipped building.

`--load` restores the placed origin; do not re-Place. Door is read from the catalog on load. `--no-time` still blocks.

**Wall meshes** (viewer, hash-neutral): for a placed sleep item with `interior`, spawn one mesh per **perimeter** cell that is **not** a door cell. Stem **`fence`** (existing `wood.glb`). The origin building mesh from `spawn_sleep_place` stays. Missing glb ⇒ M47 sentinel for that wall cell. No walls when `interior` is false. No new glb file.

A pure helper (`door_cells` / `wall_cells`) lives in sim-core so tests do not need a GPU.

### C. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**. These are not the arrow.

| id | inputs | visual |
|---|---|---|
| `purlin` | wood×20 | reuse `wood.glb` |
| `gaiter` | fiber×20 | reuse `low_poly_cloth.glb` |
| `cornice` | stone×18 | reuse `stones_and_grass.glb` |
| `bagel` | food×17 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M73 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Ammo | a separate bolt item; spear ammo; melee consume |
| Interior | shipping tent/cabin/house become interior; door on stations; authored wall glb |
| Also not | dual-station (**RF-23**); Unix sockets (**RF-8**); wasm32 Win/mac (**RF-10**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append. `door` is a catalog string, not a checkpoint enum.
2. **format_version still writes 3 / reads v2+v3.**
3. One ammo item, `arrow`, for both bow and sling. Stone stops being ammo.
4. Omit `door` keeps M65 origin-only entry. A side is every cell on that edge.
5. Walls are visual. They do not add extra Move blocks beyond the door rule.
6. Do not change shipping `coop.toml` or tent/cabin/house. No TLS. `cargo test` never needs the network.

## Tests (M73 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `232b02e6…` |
| `--no-time` shipped objects | hash `cd81a0bf…` |
| melee dist 1, hold bow + arrow | arrow qty unchanged |
| bow dist 2, 1 arrow | arrow 1→0 |
| bow dist 2, 0 arrow, holding stone | Attack illegal / Wait |
| sling dist 2, 1 arrow | arrow 1→0 |
| spear dist 2, 0 arrow | Attack still legal |
| miss still consumes | arrow 1→0 |
| `--load` | no extra consume |
| fixture 3×3, door omit | outside → non-origin Wait; outside → origin ok (M65) |
| fixture 3×3, `door = "n"` | outside → north edge ok; outside → origin Wait |
| fixture 3×3, `door = "e"` | outside → east edge ok |
| inside / exit | still legal |
| `--load` interior | door rule unchanged; no re-Place |
| `--no-time` + door | still blocks |
| `wall_cells` 3×3 door `n` | 8 perimeter cells minus the north edge |
| `interior` false | no wall cells |
| door omit vs `n` (no tools) | hashes differ |
| door omit idle | no extra hash bytes vs M65 field |
| Craft arrow / purlin / gaiter / cornice / bagel | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: Arrow ammo

- `arrow.toml`; bow/sling `ammo = "arrow"`; combat tests follow arrow; hash tests

### PR 2: Door side + walls

- `door` parse + hash tag; `move_into_interior_blocked`; `wall_cells`; viewer spawn fence stem on wall cells; fixture tests

### PR 3: Recipes

- purlin/gaiter/cornice/bagel; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `arrow.toml`; bow/sling `ammo = "arrow"`; `[sim] door`; viewer wall cells; purlin/gaiter/cornice/bagel. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test combat
cargo test -p sim-core --test objects
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M73-test-plan.md`](M73-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `cd81a0bf…`; default (time on) `232b02e6…`; Hello v5; format_version 3 write.

## Risks

- **Postcard / protocol.** None. Door and ammo are catalog TOML, not ExperimentConfig.
- **`--load`.** Restore inventory and sleep origin. Do not extra-consume. Do not re-Place.
- **M65 identity.** Omit `door` must still block every non-origin cell. `door = "s"` is the whole south edge, which is **not** the same as omit on a `w>1` footprint.
- **Hash.** Ammo slug change and new `[sim]` files move idle hashes. `door` omit-hash. Walls are not hashed.
- **Pocket cap.** Arrow is wood×1. Recipe inputs wood×20 / food×17; tests must not stock 2×32.
- **No protocol / format bump.**
