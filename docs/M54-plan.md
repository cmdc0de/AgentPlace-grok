# M54 — Sleep pickup, household auto-cabin, spear melee + range, extra recipes

**Status:** implemented  
**Depends on:** M53 complete (`docs/M53-plan.md`, git tag `M53`, commit `8f1ca2e`)  
**Walkthrough:** [`docs/M54-test-plan.md`](M54-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-17 leftovers, PG-9 recipes); M51 later-table spear melee / weapon range

## Context

M53 placed tent / cabin / house on N×N land and left them until the run ends. PairBond household homes (M33) mint a **crate cell**, not a cabin. Spear is a hunt tool with **no** Attack bonus. Attack is Chebyshev **1** only. Catalog already has hoe/net/waterskin/charcoal; M53 added cabin/house.

M54 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and `[sim] attack_bonus` / `attack_range` **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**.

## Goal

A researcher can:

1. **Pick up** a placed tent / cabin / house while standing on **any cell** of its N×N footprint. The item returns to inventory and those cells free for a later Place.
2. Turn on **`--household-crates`** (and catalog) so a **PairBond** that mints a home also **places a cabin** on the home cell when a 2×2 land patch is free — no wood/stone consumed (same mint style as the household crate).
3. **Hold a spear** and deal extra Attack damage; **bow / sling / spear** can Attack past Chebyshev 1 using `[sim] attack_range`. Unarmed / club / knife / pike stay adjacent.
4. **Craft** four new catalog items (**torch, axe, jar, bread**). Catalog-off ⇒ Pickup of catalog sleep items illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
5. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. Postcard **append only**: `PrimaryAction::Pickup` after `Place`; `SimEventKind::PickedUp` after `Placed` (hash tag **34**). Mock never picks Place, Pickup, or Craft.

### A. Sleep pickup / destroy (PG-17 leftover)

`PrimaryAction::Pickup` (no item field — the footprint under the agent). Legal when: catalog on, not incapacitated, not a child, agent stands on **any cell** of a sleep footprint, and pockets **or** pack can take qty 1 of that item.

Execute: origin = the stored min-(x,y) for that footprint. Remove that origin from `sleep_places`. Give 1 item (pockets first, else pack). Event `PickedUp { x, y, item }` (origin xy). If neither pockets nor pack can hold it → **Wait**, structure stays.

`--load` restores `sleep_places`; do not re-grant the item. Viewer already syncs by origin (despawn on missing key). Hash-neutral mesh.

### B. Household auto-cabin (PG-17 leftover)

Only when **`household_crates` overlay is on** and catalog has `cabin`. On PairBond, when a new `household_home` is minted (today’s land-cell rule):

- If `can_place` cabin at the actor’s cell → insert `sleep_places` origin there with item `cabin`. **Do not** consume inputs.
- If water / OOB / overlap → skip cabin; home crate still mints as today.

`--load` restores home + origins; **do not re-mint** on decode. Overlay off / catalog-off ⇒ no auto-cabin (those overlay-off hashes stay identity). Pickup of an auto-cabin is **allowed** (it is a real cabin; documented household payoff). `household_home` stays even if the cabin is picked up.

### C. Spear melee + weapon range

`[sim] attack_bonus` on builtin `spear.toml` (catalog already maps `spear` → `ItemId::Spear`). **`attack_bonus = 500`**. Catalog-off / no spear held ⇒ today’s STR-only damage.

New hashed `[sim] attack_range` (omit = **1**, Chebyshev). Hashed with catalog like `attack_bonus` (omit hashes as 1). Legal Attack when `dist <= max held attack_range` (unarmed / omit = 1). Execute matches that (today is `dist == 1`).

| slug | attack_range |
|---|---|
| spear | **2** |
| sling | **2** |
| bow | **3** |
| club / knife / pike / omit | **1** |

Max range among held items, same rule as max bonus (do not sum). Night does not gate Attack.

### D. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (hoe/net/waterskin/charcoal already ship):

| id | inputs | visual |
|---|---|---|
| `torch` | wood×1 + fiber×1 | reuse `wood.glb` |
| `axe` | wood×2 + stone×1 | reuse `spear.glb` |
| `jar` | stone×2 | reuse `stones_and_grass.glb` |
| `bread` | food×2 | reuse `plants_ready.glb` |

No tool-use bonuses this slice (axe does not change gather). Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M54 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; durability / workstations; interiors / non-square footprints; OTLP protobuf/gRPC; process CPU/disk; viewer-frame OTLP; extra invention kinds; DEX to-hit leftover; ammo / projectile FX; GltfDracoDecoderPlugin |
| Not M54 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; hoe/net farm/fish bonuses |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Pickup is a sim action, not a ControlVerb.
2. **format_version still writes 3 / reads v2+v3.** `sleep_places` already `#[serde(default)]`. No new ckpt fields for range (lives on catalog).
3. Pickup from **any cell** of the footprint; origin stored min-(x,y) is removed.
4. Auto-cabin mints **without** consuming materials, only when `household_crates` + `can_place`. Free cabin may be picked up.
5. `attack_range` omit = 1. Max held, not sum. Spear melee is `attack_bonus = 500` on the builtin spear file.
6. Shipped-objects idle hashes **change** (new files + spear bonus/range + `attack_range` hashed for every catalog row). No-objects `--no-time` stays `70e5204d…`.
7. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M54 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `35746f95…` |
| default CLI 2 ticks (time on) | hash `7b8864e9…` |
| catalog-off | Pickup illegal; torch Craft illegal |
| Place tent then Pickup | origin gone; inventory +1 tent; `PickedUp` event |
| Pickup with full pockets+pack | Wait; origin stays |
| Pickup from non-origin cabin cell | still removes origin; 2×2 frees |
| `--load` after Place | origin restored; Pickup after load works; no double item |
| household_crates + catalog + PairBond on 2×2 land | cabin origin at home; no wood/stone spent |
| PairBond cannot place (water) | home mints; no cabin |
| `--load` after auto-cabin | origin + home restored; no second cabin |
| overlay household off | PairBond does not insert sleep_places |
| hold spear, catalog on | Attack damage = STR + 500 |
| catalog-off spear | damage = STR only (today) |
| hold bow, Chebyshev 3 | Attack legal; dist 4 illegal |
| unarmed / club | Attack legal only at dist 1 |
| Craft torch / axe / jar / bread | from locked inputs |
| Hello v5 / format 3 | unchanged |

GPU window not required.

## PR Plan

### PR 1: Pickup

- `Pickup` / `PickedUp` tag 34; legal/execute; full-inventory Wait; load tests; viewer despawn via existing origin sync

### PR 2: Household auto-cabin

- PairBond mint + `can_place` cabin; skip on water/overlap; no re-mint on `--load`; overlay-off identity

### PR 3: Combat + recipes

- spear `attack_bonus` 500; `[sim] attack_range`; torch/axe/jar/bread TOML; range legal/execute tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (Pickup is an action; auto-cabin uses existing `--household-crates`; catalog files + existing `--catalog`).

On implement: `attack_bonus` / `attack_range` on spear/sling/bow; create torch/axe/jar/bread TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test sleep
cargo test -p sim-core --test combat
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M54-test-plan.md`](M54-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `35746f95…`; default (time on) `7b8864e9…`; Hello v5; format_version 3 write.

## Risks

- **Postcard append only** for `Pickup` / `PickedUp` (tag 34). Do not reorder `Place` / `Placed`.
- **`--load` must not re-mint** auto-cabin or re-grant a picked-up item.
- **Free cabin** from PairBond can be picked up (documented household payoff).
- **Catalog `[sim] attack_range`** hashed for every entry (omit=1), so shipped-objects hashes move even without combat.
- Cabin 2×2 still needs a land patch; auto-cabin tests use a land square.
- Mock never Place/Pickup. Default reproduction/conflict off ⇒ idle 2-tick still has no PairBond/Attack; hash move is catalog-only.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
