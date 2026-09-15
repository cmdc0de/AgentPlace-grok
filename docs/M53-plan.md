# M53 — Sleep places, night Hunt/Farm gating, CON dawn bonus

**Status:** implemented  
**Depends on:** M52 complete (`docs/M52-plan.md`, git tag `M52`, commit `34e2f62`)  
**Walkthrough:** [`docs/M53-test-plan.md`](M53-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-17 sleep places, PG-4 CON leftover); M52 later-table night action gating

## Context

M52 shipped a default-on day/night clock and tiredness-scaled dawn refill. Rest is still anywhere (no bed). M51 `tent` is a **held** catalog item, not a place. Hunt and Farm stay legal at night. CON already raises `energy_max` / illness; it does not change dawn math.

M53 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Time stays default **on**. New catalog `[sim]` fields and cabin/house files **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**.

## Goal

A researcher can:

1. **Craft** tent / cabin / house and **Place** one so it occupies **N×N land cells** (tent 1×1, cabin 2×2, house 4×4). At dawn, an agent standing on **any cell of that footprint** gets extra energy (tent modest, cabin more, house most) on top of M52 tiredness refill.
2. See **Hunt** and **Farm** become **illegal at night** when the clock is on (the default). `--no-time` ⇒ legal as today. Gather / Fish / Attack / Craft / Place stay legal at night.
3. Turn on `--sheet` and see **CON ≠ 0** add a small extra dawn refill. CON 0 / sheet off ⇒ M52 dawn formula only.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. Postcard **append only**: `PrimaryAction::Place` after `Invent`; `SimEventKind::Placed` after `Pipeline` (hash tag **33**).

### A. Sleep places (PG-17)

Object TOML `[sim]` (hashed with catalog):

| key | omit | meaning |
|---|---|---|
| `sleep_bonus` | 0 | millipoints of `energy_max` added at dawn |
| `sleep_size` | 1 | **N×N** cells. Min 1, max **8**. |

| id | inputs | sleep_bonus | sleep_size | visual |
|---|---|---|---|---|
| `tent` (existing file) | fiber×3 + wood×2 (unchanged) | **100** | **1** (1×1) | keep `low_poly_cloth.glb` |
| `cabin` (create on implement) | wood×4 + stone×2 | **200** | **2** (2×2) | reuse `wood.glb` |
| `house` (create on implement) | wood×6 + stone×3 + fiber×2 | **300** | **4** (4×4) | reuse `low_poly_house.glb` if present, else `wood.glb` |

`PrimaryAction::Place { item: ItemId }`. Legal when: catalog on, agent holds qty≥1 of a catalog item with `sleep_bonus > 0`, **origin** = agent’s cell, square `origin .. origin+(N-1)` is in-bounds, **all land**, and **no** existing sleep footprint covers any of those cells. Consumes 1. Origin is the min-(x,y) corner; square grows +x +y. Mock does **not** pick Place.

World: `sleep_places: BTreeMap<(u32,u32), ItemId>` (origin → item) with `#[serde(default)]` so old ckpts load. Size looked up from the **current catalog** (hashed there). **Hashed** when non-empty (origin xy + item). No pickup / destroy this slice. Footprints must not overlap.

Dawn (time on only), after M52 tiredness refill, before clamp:

```
shelter = sleep_bonus of the structure whose N×N contains the agent (0 if none)
con_extra = CON_mod.max(0) as u32 * 250     # 0 if unused CON
energy = min(max, energy + tiredness_refill + max * shelter / 1000 + con_extra)
```

Agent on **any cell of the footprint** counts (not Chebyshev 1 outside it). `--load` restores origins; do not re-apply dawn on decode.

Catalog-off / `--no-time`: Place illegal / no dawn. Rest math stays M52-off when `--no-time`.

Viewer: one mesh at the origin, scaled to N cells (hash-neutral). Missing path ⇒ M47 sentinel.

### B. Night Hunt / Farm gating

When `time_enabled` and `is_night(tod)`: **Hunt** and **Farm** are not in `legal_actions`. Gather, Fish, Attack, Craft, Place, Rest, Move stay legal.

`--no-time` ⇒ no gating (today). No extra hash fields (tick + `ticks_per_day` already hashed).

### C. CON dawn bonus (PG-4 leftover)

Existing `[agents.sheet]` / `--sheet`. Derived at dawn. CON 0 / unused ⇒ extra **0**. CON 18 ⇒ +1000 milli (mod +4 × 250). Do not store the extra. `--load` cannot double-apply. Default CLI does **not** enable sheet, so CON does not move default idle hashes.

## Out of scope (later)

| Later | What |
|---|---|
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| After M59 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; non-square footprints; OTLP protobuf/gRPC |
| Not M53 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; PG-9 recipes beyond cabin/house |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Place is a sim action, not a ControlVerb.
2. **format_version still writes 3 / reads v2+v3.** `sleep_places` uses `#[serde(default)]` like stockpiles. Size lives on catalog, not the ckpt row.
3. Footprint is **N×N** (`sleep_size`), not a single cell. Tent 1, cabin 2, house 4. Origin = agent cell, +x +y.
4. Night gates **Hunt and Farm only**. `--no-time` restores today’s legal set.
5. CON dawn extra is derived; sheet off does not move default hashes.
6. Shipped-objects idle hashes **change** (tent `[sim]` + cabin/house). No-objects `--no-time` stays `70e5204d…`.
7. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M53 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `6e8b124a…` |
| default CLI 2 ticks (time on) | hash `3a294816…` |
| catalog-off | Place illegal; cabin Craft illegal |
| catalog-on | Craft tent / cabin / house from locked inputs |
| Place tent | consumes 1; occupies 1 cell; second Place on that cell fails |
| Place cabin | occupies 2×2; Place in any of those 4 cells fails; Place fails if any of 4 is water / OOB |
| Place house | occupies 4×4 |
| dawn on tent cell | extra vs open-air = `max * 100 / 1000` |
| dawn on cabin **non-origin** cell | still gets cabin bonus |
| dawn off footprint | M52 tiredness only |
| `--load` after Place | origins restored; dawn not doubled |
| night, time on | Hunt and Farm absent from legal_actions; Gather present |
| `--no-time` | Hunt/Farm still legal |
| sheet off / CON 0 | dawn extra 0 |
| CON 18 vs 3 | 18 gets +1000 milli more than unused at dawn (open-air) |
| Hello v5 / format 3 | unchanged |

GPU window not required.

## PR Plan

### PR 1: Night Hunt/Farm gating

- `legal_actions` filters Hunt/Farm when time on + night; `--no-time` identity tests

### PR 2: Place + N×N sleep_places + dawn shelter

- `[sim] sleep_bonus` / `sleep_size`; tent/cabin/house TOML; `Place` / `Placed`; overlap/land tests; dawn; viewer scale to N cells

### PR 3: CON dawn extra

- `CON_mod.max(0)*250` at dawn; unused identity; CON 18 vs 3

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (Place is an action; catalog files + existing `--sheet` / `--time`).

On implement: `sleep_bonus` / `sleep_size` on tent; create cabin/house TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test clock
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M53-test-plan.md`](M53-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `6e8b124a…`; default (time on) `3a294816…`; Hello v5; format_version 3 write.

## Risks

- **Postcard append only** for `Place` / `Placed` (tag 33). Do not reorder `Invent` / `Pipeline`.
- **World origins `#[serde(default)]`**. Size lives on catalog, not the ckpt row. `--load` must not double-apply dawn.
- **Catalog change moves shipped-objects hashes.** No-objects `--no-time` stays `70e5204d…`.
- **N×N must all be land**; house 4×4 can fail on a crowded map — tests use a tiny land patch.
- Mock never Place. Night gating may change hashes if idle Hunt/Farm happens; document.
- CON extra is derived; sheet off does not move default hashes.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
