# M69 — Ranged ammo consume, extra recipes

**Status:** implemented  
**Depends on:** M68 complete (`docs/M68-plan.md`, git tag `M68`, commit `b8c2d1f`)  
**Walkthrough:** [`docs/M69-test-plan.md`](M69-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-19 / RF-PG9

## Context

M64 ranged Attack (Chebyshev > 1) is projectile FX only. Spear/bow/sling already have `attack_range` 2–3. No ammo consume. Catalog after M68 includes rail/mitten/quoin/roll.

M69 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Bow/sling `[sim] ammo` **and** new recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not a dedicated arrow item, not ammo on spear, not dual-station.

## Goal

A researcher can:

1. **See** a ranged Attack (Chebyshev **> 1**) **consume 1 ammo** from the weapon that enabled that range. Bow and sling consume **1 stone**. No stone ⇒ Attack illegal (Wait). Adjacent Attack (dist 1) never consumes ammo. Spear has no `ammo` field ⇒ range-2 Attack still works without stone.
2. **Craft** four new catalog items (**joist, veil, ashlar, loaf**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather / Attack unless tests `execute_primary` (conflict overlay as today).

### A. Ranged ammo (RF-19)

```toml
[sim]
ammo = "stone"    # omit = none
```

On **implement**: set `ammo = "stone"` on `bow.toml` and `sling.toml`. **Not** on `spear.toml`.

Who/when: Attack Chebyshev **> 1**. Pick the held catalog item with **max `attack_range` among those with `attack_range >= dist`**. If that item’s `ammo` slug is non-empty, require `held_qty(ammo) >= 1`; consume 1 (pockets then pack) **after** energy pay, hit or miss. Dist 1: never consume. Empty / omitted `ammo`: no consume.

`legal_actions`: omit Attack when that ammo check would fail.

Catalog-off: no ammo field ⇒ unarmed range 1, never consume.

Hash `[sim] ammo` **only when non-empty** (slug bytes). Bow/sling change ⇒ shipped-objects idle hashes **move**.

`--load` restores inventory; do not extra-consume on decode.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `joist` | wood×16 | reuse `wood.glb` |
| `veil` | fiber×16 | reuse `low_poly_cloth.glb` |
| `ashlar` | stone×14 | reuse `stones_and_grass.glb` |
| `loaf` | food×13 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M69 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Combat | dedicated arrow item (**RF-24**); ammo on spear; consume on melee |
| Also not | dual-station (**RF-23**); bonus-use every-instance (**RF-22**); interior walls (**RF-21**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Ammo is catalog `[sim] ammo` slug, not a new overlay. Stone builtin — no arrow item this slice (**RF-24**).
4. Consume on fire (hit or miss), not on hit only. Dist 1 never consumes.
5. Spear is reach, not thrown: no ammo field.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M69 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `98abaa85…` |
| `--no-time` shipped objects | hash `1910c33a…` |
| melee dist 1, hold bow + stone | stone qty unchanged |
| bow dist 2, 1 stone | stone 1→0; Attack proceeds |
| bow dist 2, 0 stone | Attack illegal / Wait; energy unchanged |
| sling dist 2, 1 stone | stone 1→0 |
| spear dist 2, 0 stone | Attack still legal (no ammo field) |
| miss still consumes | stone 1→0 |
| `--load` | no extra consume |
| catalog-off | unarmed Attack dist 1, no ammo |
| Craft joist / veil / ashlar / loaf | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Ranged ammo

- `[sim] ammo` on bow/sling; legal_actions + execute consume; tests

### PR 2: Recipes

- joist/veil/ashlar/loaf; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `bow.toml` / `sling.toml` `ammo = "stone"`; joist/veil/ashlar/loaf. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test combat
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M69-test-plan.md`](M69-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `1910c33a…`; default (time on) `98abaa85…`; Hello v5; format_version 3 write.

## Risks

- **Catalog hash.** Ammo slugs + recipes move idle hashes. Omit-hash when ammo empty.
- **Which weapon.** Max range among items that can reach `dist` — holding bow+spear at dist 2 uses bow (range 3) and needs stone.
- **`--load`.** Restore inventory; do not extra-consume on decode.
- **No protocol / format bump.**
