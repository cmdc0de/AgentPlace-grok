# M55 — Ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes

**Status:** implemented  
**Depends on:** M54 complete (`docs/M54-plan.md`, git tag `M54`, commit `55316ed`)  
**Walkthrough:** [`docs/M55-test-plan.md`](M55-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 DEX leftover, PG-9 recipes); M54 Not-list hoe/net farm/fish bonuses

## Context

M41 already rolls melee to-hit: unused defender DEX always hits; else `d20 + STR_mod >= 10 + DEX_mod`. M54 gave spear/sling/bow `attack_range` but those Attacks still use the **STR** formula. Farm `skill_roll` bonus is 0. Fish is fishing-rod +25 / bare −15. Hoe and net exist as crafts with **no** Farm/Fish bonus. Catalog after M54 includes torch/axe/jar/bread.

M55 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and `[sim] farm_bonus` / `fish_bonus` **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Sheet/conflict stay default off, so DEX does **not** move default idle hashes.

## Goal

A researcher can:

1. Turn on `--sheet` + `--conflict`, hold a **bow / sling / spear** (`attack_range > 1`), and see to-hit use **attacker DEX_mod** vs defender DEX. Club / knife / pike / unarmed stay M41 STR vs DEX.
2. Hold a **hoe** and succeed Farm more often; hold a **net** and succeed Fish more often. Catalog-off / not holding ⇒ today (Farm bonus 0; Fish rod +25 / bare −15).
3. **Craft** four new catalog items (**rope, needle, bucket, shield**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Farm / Fish / Attack unless tests `execute_primary`.

### A. Ranged DEX to-hit (PG-4 leftover)

M41 already: unused defender DEX ⇒ always hit; else `d20 + STR_mod >= 10 + DEX_mod`. Stream `tick_{t}_agent_{atk}_attack_hit_0` (not `RngBank.ensure`).

M55: if **max held `attack_range` > 1** and **attacker DEX ≠ 0**, replace STR_mod with **attacker DEX_mod** in that same inequality. Defender DEX 0 still always hits. Attacker DEX 0 (unused) keeps the M41 STR formula even with a bow.

Miss ⇒ pay energy, `Attack { damage: 0 }` as today. Sheet off / unused scores ⇒ **same hashes**.

Do **not** change the melee STR-vs-DEX formula.

### B. Hoe Farm / net Fish bonuses

Hashed `[sim]` (omit = 0), like `attack_bonus`:

| key | omit | meaning |
|---|---|---|
| `farm_bonus` | 0 | added to Farm `skill_roll` bonus |
| `fish_bonus` | 0 | added to Fish `skill_roll` bonus |

| slug | field | value |
|---|---|---|
| `hoe` (existing file) | `farm_bonus` | **25** |
| `net` (existing file) | `fish_bonus` | **25** |

Max among **held** items (pockets or pack), not sum. Farm today is bonus 0; add that max. Fish today is rod +25 / else −15; use `max(rod_bonus, catalog fish_bonus)` then if that max is 0 keep **−15**. Holding both rod and net ⇒ 25, not 50.

Catalog-off / empty catalog / not holding ⇒ identity. `--load` restores inventory; do not persist derived odds.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (hoe/net/torch/axe/jar/bread already ship):

| id | inputs | visual |
|---|---|---|
| `rope` | fiber×4 | reuse `low_poly_cloth.glb` |
| `needle` | stone×1 + fiber×1 | reuse `spear.glb` |
| `bucket` | wood×2 | reuse `wood.glb` |
| `shield` | wood×3 | reuse `wood.glb` |

No tool-use bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| After M57 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; durability / workstations; interiors / non-square footprints; OTLP protobuf/gRPC; viewer-frame OTLP; extra invention kinds; ammo / projectile FX |
| Not M55 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; changing M41 melee STR-vs-DEX formula |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.** Odds are derived; do not store them.
3. Ranged to-hit only when `attack_range > 1` **and** attacker DEX ≠ 0. Melee stays M41.
4. `farm_bonus` / `fish_bonus` omit = 0. Max held, not sum. Fish bare stays **−15** when neither rod nor net.
5. Shipped-objects idle hashes **change** (new files + hoe/net bonuses + `farm_bonus`/`fish_bonus` hashed for every catalog row). No-objects `--no-time` stays `70e5204d…`. DEX does not move default idle hashes.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M55 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `5028d7ed…` |
| default CLI 2 ticks (time on) | hash `32fc6324…` |
| unused DEX + bow | same hit as M41 STR formula (identity) |
| attacker DEX 18 + bow vs defender DEX 18 | can miss on a locked seed; club (range 1) still uses STR |
| defender DEX 0 + bow | always hit |
| sheet off | idle hashes unchanged by DEX code |
| catalog-off Farm/Fish | Farm bonus 0; Fish rod/+bare as today |
| hold hoe, catalog on | Farm `skill_roll` bonus 25 vs 0 without hoe (same seed) |
| hold net, catalog on | Fish bonus 25; bare still −15; rod+net still 25 not 50 |
| Craft rope / needle / bucket / shield | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required.

## PR Plan

### PR 1: Ranged DEX to-hit

- `attack_range > 1` + attacker DEX ≠ 0 uses DEX_mod; unused identity; club still STR

### PR 2: hoe / net bonuses

- `[sim] farm_bonus` / `fish_bonus`; max held; Fish −15 when none; hoe=25 net=25

### PR 3: Recipes

- rope/needle/bucket/shield TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (DEX uses existing `--sheet` / `--conflict`; hoe/net / recipes are catalog files).

On implement: `farm_bonus` on hoe; `fish_bonus` on net; create rope/needle/bucket/shield TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test combat
cargo test -p sim-core --test population
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M55-test-plan.md`](M55-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `5028d7ed…`; default (time on) `32fc6324…`; Hello v5; format_version 3 write.

## Risks

- **Do not change M41 melee** STR-vs-DEX. Ranged-only.
- **`--load` must not double-apply** derived hit/farm/fish odds (recompute from scores + holdings).
- **Catalog `[sim] farm_bonus` / `fish_bonus`** hashed for every entry (omit=0), so shipped-objects hashes move even without Farm/Fish happening.
- Fish identity: bare −15 unless rod or net. Do not turn “no tool” into 0.
- Mock never Farm/Fish/Attack on default idle. Default sheet/conflict off ⇒ DEX does not move idle hashes.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
