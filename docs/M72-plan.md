# M72 — Bonus-use every-instance wear, extra recipes

**Status:** implemented  
**Depends on:** M71 complete (`docs/M71-plan.md`, git tag `M71`, commit `90f756e`)  
**Walkthrough:** [`docs/M72-test-plan.md`](M72-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-22 / RF-PG9

## Context

M67 `[wear] every_instance` / `--every-instance-wear` increments **every** wear slot on a **decay pass** (dawn, or each tick if `--per-tick-wear`). Successful bonus-use (vegetation Gather / stone Gather / Farm / Fish) still wears **one** most-worn instance. Catalog after M71 includes stake/cuff/plinth/crumpet.

M72 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Reuse existing `[wear] every_instance` (hashed **only when true**, tag `[2u8]`). New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Overlay-off idle hashes stay M71 until recipes land. Not dual-station, not arrow ammo, not a second overlay key.

## Goal

A researcher can:

1. **Turn on** `[wear] every_instance = true` / `--every-instance-wear` and see a **successful bonus-use** (vegetation Gather / stone Gather / Farm / Fish) increment **every** wear slot of the held tool that granted the bonus, then consume each slot that reaches `uses`. Overlay off ⇒ M59 most-worn-only bonus-use. Decay pass (dawn / `--per-tick-wear`) stays M67 every-instance.
2. **Craft** four new catalog items (**stud, glove, dado, pikelet**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`. No new overlay key. No new CLI flag.

### A. Bonus-use every-instance (RF-22)

Reuse M67 overlay. Do **not** add `[wear] bonus_every_instance` or `--bonus-every-instance-wear`.

```toml
[wear]
every_instance = true    # omit = false; already hashed when true, tag [2u8]
```

CLI: `--every-instance-wear` (already exists). Same or-with-overlay as today.

Who/when: the four `execute.rs` bonus-use `wear_tool` sites, **held only**, after the same success conditions as today:

| Action | Bonus field | Tool (shipping) | Wear today |
|---|---|---|---|
| vegetation `Gather` | `gather_bonus` | axe | most-worn |
| stone `Gather` | `stone_gather_bonus` | hammer | most-worn |
| `Farm` | `farm_bonus` | hoe | most-worn |
| `Fish` | `fish_bonus` | net | most-worn |

When `wear_every_instance`: call `wear_all_tools` (already on Agent + crate) instead of `wear_tool`. Same item `held_wear_item` picks (max bonus among held). Increment **all** slots of that item, then remove every slot `>= uses` (high index first).

When off: today’s most-worn `wear_tool`. `two_axes_wear_separately` stays.

Not: Hunt, Attack, Craft, Store, Give, crates (bonus-use is held). Failed Farm/Fish/stone skill_roll still does not wear. Vegetation Gather wear trigger stays as today (after taking the plant).

`--load` restores wear vecs; do not extra-wear on decode. Idle mock never Gather ⇒ this field does not move idle hashes by itself (tag `[2u8]` already does when overlay on).

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `stud` | wood×19 | reuse `wood.glb` |
| `glove` | fiber×19 | reuse `low_poly_cloth.glb` |
| `dado` | stone×17 | reuse `stones_and_grass.glb` |
| `pikelet` | food×16 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M72 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Wear | per-tick **and** dawn both firing; a second overlay key for bonus-use |
| Also not | dual-station (**RF-23**); arrow/bolt ammo (**RF-24**); interior walls (**RF-21**); Unix sockets (**RF-8**); wasm32 Win/mac (**RF-10**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Reuse `[wear] every_instance`. Decay pass already every-instance; this slice is bonus-use only.
4. No new hash tag. Overlay-off idle identity from this field. Recipes still move shipped-objects hashes.
5. Held bonus-use only. Same four triggers. Crate decay stays M67.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M72 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `5a14ef91…` |
| `--no-time` shipped objects | hash `53972084…` |
| overlay off, two axes `[3, 0]`, 1 Gather | `[4, 0]` (M59 identity) |
| overlay on, two axes `[3, 0]`, 1 Gather | `[4, 1]` |
| overlay on, `[7, 7]`, `uses = 8`, 1 Gather | consume 2 |
| overlay on, two hoes, 1 Farm | both hoe slots +1 |
| overlay on, two nets, 1 Fish | both net slots +1 |
| overlay on, two hammers, 1 stone Gather | both hammer slots +1 |
| overlay off idle | hash unchanged from this field (recipes still move) |
| `--load` | no extra wear |
| Craft stud / glove / dado / pikelet | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: Bonus-use every-instance

- Four `execute.rs` sites: `wear_all_tools` when `sim.wear_every_instance`; tests for Gather/Farm/Fish/stone Gather

### PR 2: Recipes

- stud/glove/dado/pikelet; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: four execute sites call `wear_all_tools` when overlay on; stud/glove/dado/pikelet. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md) (bonus-use also every-instance when overlay on).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M72-test-plan.md`](M72-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `53972084…`; default (time on) `5a14ef91…`; Hello v5; format_version 3 write.

## Risks

- **Postcard / protocol.** None. Overlay TOML is not ExperimentConfig.
- **`--load`.** Restore vecs; do not extra-wear on decode.
- **Double-wear.** Decay pass already every-instance. Bonus-use is a separate execute path; do not also call decay inside execute.
- **Catalog hash.** Recipes move idle hashes. Wear overlay omit-hash when false.
- **Pocket cap.** Inputs wood×19 / food×16; tests must not fill 32 slots with 2× recipe stock.
- **No protocol / format bump.**
