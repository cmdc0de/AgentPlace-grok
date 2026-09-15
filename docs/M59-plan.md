# M59 — Extra invention kinds, per-instance tool wear, extra recipes

**Status:** implemented  
**Depends on:** M58 complete (`docs/M58-plan.md`, git tag `M58`, commit `56d996f`)  
**Walkthrough:** [`docs/M59-test-plan.md`](M59-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-5 leftover kinds, PG-9 recipes); M58 later-table per-instance durability stacks

## Context

M58 gave axe/hammer/hoe/net `uses = 8` with **one counter per ItemId** (two axes share 8 uses). Invent kinds are GatherBonus / MoveBonus / SenseBonus; a fourth Invent Waits. Catalog after M58 includes flour/cart/bellows/table.

M59 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Invention overlay default off and empty `tool_wear` do **not** move idle hashes by themselves. Per-instance wear is a **thin first slice** (vec per ItemId, most-worn breaks first). Not transferring wear on Give, not time-decay.

## Goal

A researcher can:

1. Turn on `[inventions]` / `--inventions` and Invent **CraftBonus** then **RestBonus** after Gather/Move/Sense (postcard append). Overlay off / empty table ⇒ today. Mock never picks Invent.
2. Hold **two axes** and see them wear **separately**: 8 successful veg Gathers consume **one** axe; the other stays at 0 wear. M58 one-axe 8-uses still consumes 1.
3. **Craft** four new catalog items (**raft, sandals, mortar, jerky**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. `InventionKind` **append only**. No new `PrimaryAction` / `SimEventKind`. Mock never picks Invent / Craft / Gather unless tests `execute_primary`.

### A. Extra invention kinds (PG-5 leftover)

Append on `InventionKind` (u8):

| Tag | Kind | Effect (inventor immediately; society after `share_delay_ticks`) |
|---|---|---|
| 0–2 | Gather / Move / Sense | Unchanged |
| 3 | `CraftBonus` | Craft `skill_roll` bonus **+15** (no stack) |
| 4 | `RestBonus` | Rest energy regen `* 1200 / 1000` (min 1 if regen was > 0) |

- `InventionKind::ALL` gains both. `next_kind` still lowest missing tag. Fourth Invent (after Sense) is CraftBonus; fifth is RestBonus; sixth Waits.
- Tree (`[inventions] tree = true`): CraftBonus needs SenseBonus **shared**; RestBonus needs CraftBonus shared.
- Same chance stream, inventor +200 influence (INT quality as today), protected memory, `Invented { kind }` tag 31 with new u8 values.
- Overlay off / empty table ⇒ **same hashes**. `--load` restores table; do not re-grant influence.
- Observation: `invention craft_bonus (yours)` / `invention rest_bonus`, same pattern.

### B. Per-instance tool wear (M58 leftover, thin)

Replace the per-ItemId scalar `tool_uses: BTreeMap<ItemId, u32>` with **`tool_wear: BTreeMap<ItemId, Vec<u32>>`** (wear counts, 0 = fresh). Length is clamped to held qty (pockets + pack).

Wear (successful bonus-use, same M58 triggers): pad with 0s to qty; increment the **max** slot (most-worn; ties → lowest index). When that slot `>= uses`, remove it and consume **1** qty.

Qty up (craft/give in): extra instances are fresh (implicit 0). Qty down outside wear (Give/Store/take): **truncate from the end** (freshest leave). Wear does **not** transfer to the receiver this slice.

Hash `tool_wear` when non-empty (item + each u32 LE). Idle mock does not wear ⇒ idle hashes unchanged from this field.

**Ckpt:** BoardBlob trailing `tool_wear`. Decode: (1) new blob, (2) M58 blob with `tool_uses: BTreeMap<ItemId, u32>` → `vec![n]` per item, (3) today’s v3 without either. **Do not** bump `format_version`. `--load` restores vecs; do not re-break.

`uses = 8` on axe/hammer/hoe/net unchanged.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `raft` | wood×7 | reuse `wood.glb` |
| `sandals` | fiber×7 | reuse `low_poly_cloth.glb` |
| `mortar` | stone×5 | reuse `stones_and_grass.glb` |
| `jerky` | food×4 | reuse `plants_ready.glb` |

No `uses` / `station` / tool bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M59 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX; transferring wear on Give; time-decay wear; every-recipe stations; CPU percent; out-dir disk walk |
| Not M59 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; reordering InventionKind; requiring stations for bread/stew |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** InventionKind append 3 and 4 only. No new ControlVerb.
2. **format_version still writes 3 / reads v2+v3.** `tool_wear` BoardBlob trailing + M58 `tool_uses` u32 fallback.
3. Most-worn instance breaks first. Two axes: 8 uses consume one; the other is still fresh.
4. CraftBonus +15 craft roll; RestBonus 1.2× rest regen. No stack. Tree continues after SenseBonus.
5. Shipped-objects idle hashes **change** (new files). No-objects `--no-time` stays `70e5204d…`. Inventions overlay off does not move idle hashes.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M59 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `936b6632…` |
| default CLI 2 ticks (time on) | hash `76049695…` |
| inventions off | Invent illegal; empty table; same hash as no flag |
| CraftBonus / RestBonus | fourth Invent is CraftBonus (+15 craft); fifth RestBonus (1.2× regen); sixth Waits |
| tree | CraftBonus blocked until SenseBonus shared |
| two axes | 8 successful veg Gathers consume 1; second axe remains; its wear is 0 |
| one axe | still breaks at 8 (M58 identity) |
| load M58 wear | `tool_uses` u32 3 → vec![3] |
| Craft raft / sandals / mortar / jerky | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Invention kinds

- Append CraftBonus / RestBonus; ALL / next_kind / tree; craft roll + rest regen; observation lines; inventions tests (replace fourth-waits)

### PR 2: Per-instance wear

- `tool_wear` vec; most-worn breaks; BoardBlob + M58 u32 fallback; two-axe test; one-axe identity

### PR 3: Recipes

- raft/sandals/mortar/jerky TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (`--inventions` already exists).

On implement: append kinds; `tool_wear`; create raft/sandals/mortar/jerky TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test inventions
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M59-test-plan.md`](M59-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `936b6632…`; default (time on) `76049695…`; Hello v5; format_version 3 write.

## Risks

- **InventionKind append only.** Do not reorder Gather/Move/Sense. Postcard `Invented.kind` u8 3 and 4.
- **BoardBlob `tool_wear` vs M58 `tool_uses`.** Decode fallback `vec![n]`; do not bump format_version. `--load` must not re-break.
- **Two-axe wear.** Increment the max slot, not a shared scalar (that would still consume after 8 total).
- **Give/Store drops wear on the freshest slot** (end of vec). Documented; do not invent transfer this slice.
- **Catalog files** change shipped-objects hashes even without Invent/Gather.
- **Recipe inputs** stay distinct (wood×7, fiber×7, stone×5, food×4).
