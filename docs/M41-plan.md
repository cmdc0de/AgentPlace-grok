# M41 — World species from TOML, DEX accuracy, STR haul

**Status:** implemented  
**Depends on:** M40 complete (`docs/M40-plan.md`, git tag `M40`, commit `5f57ab4`)  
**Walkthrough:** [`docs/M41-test-plan.md`](M41-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-8 species, PG-4 DEX/STR)

## Context

M40 moved item recipes into `configs/objects/*.toml`. Vegetation / animal / fish **visuals** bind from those files; spawn and gather still use Rust `default_species_tables()`. Attack always hits. Pocket slot cap is a stored `inventory_cap` (16); STR does not change carry.

M41 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Visuals stay hash-neutral. Shipping `default.toml` / `coop.toml` unchanged. Agent meshes stay later.

## Goal

A researcher can:

1. Edit `configs/objects/berry_bush.toml` (or hare/perch) `[sim]` and change gather/hunt/fish yields, toxicity, and grow ticks **without a Rust change**. Add a new vegetation/animal/fish file and it gets a new species tag (appended). Missing `[sim]` keeps the Rust default row for that id.
2. Turn on `[agents.sheet]` / `--sheet` + `[conflict]` and see a high-DEX defender **get missed** (Attack still adjacent; energy still paid; `damage = 0`). Unused DEX ⇒ always hit as today.
3. See STR change **how much the agent can carry** (pocket slots and worn-pack caps). Unused STR ⇒ cap 16 / today’s pack numbers. Do not rewrite land-crate caps.
4. `--load` restores scores and species tags; do **not** persist derived caps. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Idle mock 2-tick hash **stays `cd1e0853…`** when shipped object `[sim]` matches today’s `default_species_tables()` and sheets are unused.

## In scope

No PROTOCOL bump. No new postcard variants. `SimEventKind::Attack { target, damage }` stays; miss is `damage = 0`. Overlay TOML is not `ExperimentConfig`. Sheet overlay only **rolls** founders (already shipped); runtime math applies when the score is non-zero. Score 0 ⇒ today’s constants.

Always load `configs/objects` (or `--objects DIR`) as M40. `--catalog` does **not** gate species and still does not imply `--sheet`. Do not invent extra overlays. Create/fill named object `[sim]` blocks **on implement** only.

### A. World species from object TOML

Extend `[sim]` on `kind = "vegetation" | "animal" | "fish"`:

```toml
id = "berry_bush"
kind = "vegetation"
[visual]
glb = "assets/models/optimized/big_low_poly_berry_bush.glb"
[sim]
yield = "food"          # food | wood
nutrition = 20.0
toxicity = "safe"       # safe | toxic | allergenic
allergen_tag = ""
wood_yield = 0
fiber_yield = 1
grow_ticks = 40
```

```toml
id = "hare"
kind = "animal"
[sim]
nutrition = 30.0
toxicity = "safe"
```

- Built-in ids keep **today’s 1-based tags** (veg: berry_bush=1 … tree=5; animal hare=1; fish perch=1). Object `[sim]` **overrides fields** for that id.
- New slugs **append** in sorted id order after the built-ins (do not re-sort built-ins; tags must not shuffle).
- No `[sim]` / no file ⇒ keep `default_species_tables()` row. Empty objects dir ⇒ full Rust defaults.
- Write the merged table into `config.world.species` **before world gen**. Spawn still `1..=vegetation.len()`. Gather/Hunt/Fish/Farm/Eat read that table as today.
- Hash: **do not** add a second catalog hasher for species. Effects already land in world cells, events, and needs.
- Fill shipped veg/animal/fish object files with `[sim]` equal to `default_species_tables()`. Visuals stay hash-neutral.
- Crop stays a vegetation tag. World **resource density** stays `[world.resources]` in experiment TOML.

### B. DEX accuracy (melee)

Existing `[agents.sheet]` / `--sheet` and `[conflict]` / `--conflict`. Attack stays Chebyshev 1. Energy cost stays 500.

| Defender DEX | Result |
|---|---|
| 0 (unused) | Always hit. Damage = `attack_damage()` (STR as M34). |
| ≠ 0 | Seeded d20 from `derive_seed(master, "tick_{t}_agent_{atk}_attack_hit_0")` (same pattern as Invent — **not** `RngBank.ensure`). Hit if `d20 + STR_mod >= 10 + defender DEX_mod`. Miss ⇒ pay energy, `Attack { damage: 0 }`, no defender health/energy change. |

Mock does not pick Attack unless tests `execute_primary`. Overlay off / unused sheets ⇒ **same hashes**.

### C. STR haul

Derived at use time. **Do not write** `agent.inventory_cap`. `--load` cannot double-apply.

| Carry | Formula (STR 0 ⇒ identity) |
|---|---|
| Pockets | slot cap = `max(1, inventory_cap as i32 + STR_mod)` |
| Worn pack | `worn_pack_caps` slots += STR_mod; weight milli += STR_mod × 250; both floor at 1 if base > 0 |
| Land crate | **unchanged** (`StorageParams` / `[storage]`) |

Use the helper in `try_add_item` / `has_carry_room` / legal Pack/Store/Gather/Give. Inspector can show effective cap; no new Observation field required.

## Out of scope (later)

| Later | What |
|---|---|
| **M42** | [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| After M42 | protobuf/TLS/`wss`; Unix sockets; hashed pipeline events; agent meshes; tech tree / patents; string ItemId / ckpt bump; skeletal animation; hot-reload glb; WIS toxin detect; CHA speech; pocket **weight** cap; DEX flee bonus |
| Not M41 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Do not add fields to `Attack`. Miss = `damage = 0`.
2. Visuals never hashed. Species `[sim]` is not a second `hash_catalog` input; shipped numbers matching defaults ⇒ idle hash **unchanged**.
3. Built-in species tags stay 1..=n in today’s order; extras **append**.
4. Sheet overlay only rolls founders. Score 0 ⇒ identity. Do not persist derived haul caps.
5. Hit RNG is Invent-style `derive_seed`, not `RngBank.ensure` (fingerprint must not move when DEX is unused).
6. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M41 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `cd1e0853…` |
| shipped species `[sim]` | matches `default_species_tables()`; hash unchanged vs no extra species |
| override nutrition | change berry_bush nutrition in TOML ⇒ after Gather+Eat (or execute) hash ≠ shipped |
| extra vegetation file | new tag appended; world `n_species` grows; hash ≠ idle; built-in tags 1–5 unchanged |
| missing `[sim]` | berry_bush visual-only file still uses Rust nutrition |
| sheet unused | Attack always hits; pocket cap 16; same hash as no `--sheet` |
| DEX 18 vs DEX 0 | with `--conflict --sheet`, execute Attack: DEX 0 takes damage; DEX 18 can miss (`damage = 0`) on a locked seed |
| STR 18 vs 3 | pocket slot cap 20 vs 13; pack weight/slots differ; crate cap same |
| `--load` | sheet scores restored; `inventory_cap` field still 16; effective cap from STR; no extra items |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Species from object TOML

- **Files:** parse veg/animal/fish `[sim]`; merge-by-id + append extras; fill shipped files; hash-stable idle; override/extra tests

### PR 2: DEX accuracy + STR haul

- **Files:** hit roll (`derive_seed`, damage 0 on miss); effective pocket/pack caps; unused identity tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --objects configs/objects \
  --ticks 80 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --conflict --quiet
```

## Verification

Walkthrough: [`docs/M41-test-plan.md`](M41-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash `cd1e0853…`; species override/extra change hash; unused sheet identity; DEX miss and STR caps when scores set; Hello v5.

## Risks

- **Tag shuffle:** extras must **append**, not alphabetize the whole list, or idle worlds change.
- **Postcard:** do not add `hit: bool` to `Attack`. Miss = `damage = 0`.
- **RngBank fingerprint:** do not `ensure` a new bank stream; unused DEX must not change hash.
- **`--load`:** never assign `inventory_cap = effective_cap`; recompute from STR each use.
- Species `[sim]` must not enter `hash_catalog` or idle hash moves even when numbers match.
- Shipping `default.toml` / `coop.toml` unchanged; only `configs/objects/*.toml` for shipped species `[sim]`.
