# M58 — Tool durability + millstone station, viewer-frame OTLP, extra recipes

**Status:** implemented  
**Depends on:** M57 complete (`docs/M57-plan.md`, git tag `M57`, commit `f35b302`)  
**Walkthrough:** [`docs/M58-test-plan.md`](M58-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes, PG-11 leftover viewer-frame OTLP); M57 later-table durability / workstations

## Context

M57 gave hammer stone-gather bonus and process CPU/disk OTLP. Tools never break. Place/Pickup is sleep-only (tent/cabin/house). Millstone is a craft with no station role. Native viewer shows FPS HUD but does not POST frame time. Catalog after M57 includes barrel/cloak/pot/lantern.

M58 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and `[sim] uses` / `station` **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Viewer-frame OTLP is **hash-neutral**. Durability+workstations is a **thin first slice** (uses-until-break + Place millstone). Not per-instance stacks, not every recipe needing a station, not time-decay.

## Goal

A researcher can:

1. Hold an **axe / hammer / hoe / net** and see it **break after 8 successful uses** (`[sim] uses = 8`). Catalog-off / `uses` omit 0 ⇒ today (never breaks).
2. **Place a millstone** (1×1 land) as a workstation and **Craft flour** only when Chebyshev ≤1 of that placed millstone. Holding millstone in pockets is not enough. Pickup returns it.
3. Open the **native viewer** with `--otlp-endpoint` and POST hash-neutral **frame** draw time (`agentplace.viewer.frame_ns`). Distinct from sim tick `wall_ns` and from the FPS HUD. GPU window not required for tests.
4. **Craft** four new catalog items (**flour, cart, bellows, table**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
5. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. Reuse `PrimaryAction::Place` / `Pickup` / `Placed` / `PickedUp`. No new `SimEventKind`. Mock never picks Place / Pickup / Craft / Gather / Farm / Fish unless tests `execute_primary`.

### A. Tool durability + millstone station

Hashed catalog fields (omit = never / not a station):

| key | omit | meaning |
|---|---|---|
| `uses` | 0 | successful bonus-uses until one qty is consumed |
| `station` | false | Place-able 1×1 workstation (not a sleep place) |
| `[sim.craft] station` | empty | Craft legal only Chebyshev ≤1 of a **placed** station with that slug |

| slug | field | value |
|---|---|---|
| `axe` | `uses` | **8** |
| `hammer` | `uses` | **8** |
| `hoe` | `uses` | **8** |
| `net` | `uses` | **8** |
| `millstone` (existing file) | `station` | **true** |

**Durability.** On a **successful** vegetation Gather that used catalog `gather_bonus` (axe), stone Gather that used `stone_gather_bonus` (hammer), Farm that used `farm_bonus` (hoe), or Fish that used catalog `fish_bonus` (net): increment per-agent `tool_uses[item]`. When `tool_uses[item] >= uses`, consume **1** qty of that item (pockets then pack) and reset the counter. Max-held tool is the one that wears (same item as the bonus). `uses = 0` / catalog-off / bonus not from that item ⇒ no increment. Qty 0 after break ⇒ that bonus is gone.

Persist `tool_uses` in the checkpoint **BoardBlob** (trailing field, `#[serde(default)]`). Decode: try full blob, then today’s v3 blob without `tool_uses` (empty map). **Do not** bump `format_version`. Hash `tool_uses` when non-empty. `--load` restores counters; do not re-wear on decode.

**Workstations.** World `work_places: BTreeMap<(u32,u32), ItemId>` (`#[serde(default)]`, hashed when non-empty, like `sleep_places`). Place millstone: 1×1 land, in-bounds, no overlap with sleep **or** work footprints. Consumes 1. Pickup from any cell of that 1×1 returns it (same stow rules as sleep Pickup). Dawn **ignores** work_places.

Craft with `[sim.craft] station = "millstone"`: illegal unless a placed millstone origin is Chebyshev ≤1 of the agent. Pocket millstone does not count. Existing crafts (bread, stew, …) keep empty station ⇒ today’s legality.

`hash_catalog` writes `uses` u32 LE, `station` u8 (0/1), and craft-station slug bytes (empty = none) for **every** row.

### B. Viewer-frame OTLP (PG-11 leftover)

Native viewer only. Hash-neutral. FPS HUD unchanged.

CLI (viewer, in-process **and** `--connect`):

| Flag | Meaning |
|---|---|
| `--otlp-endpoint URL` | POST last frame ns after each rendered frame (2s timeout, best-effort, same HTTP helper as sim-cli). Implies frame export on. |
| omit / empty | today (FPS HUD only, no POST) |

Locked gauge: `agentplace.viewer.frame_ns` (last frame, integer ns). Resource `service.name=agentplace-viewer`. Still OTLP/**JSON** HTTP. No protobuf. No gRPC. No TLS.

`cargo test -p viewer` must not need a GPU. Unit-test a helper `viewer_frame_metrics_json(last_ns) -> String` contains the locked name. Optional loopback POST from a test that does not open a window.

Do **not** put frame ns on `Simulation` or in AGTN. Overlay-off sim hashes unchanged. Paused sim still renders ⇒ frames still POST if the endpoint is set.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | station | visual |
|---|---|---|---|
| `flour` | food×3 | **millstone** | reuse `plants_ready.glb` |
| `cart` | wood×6 | none | reuse `wood.glb` |
| `bellows` | fiber×3 + stone×1 | none | reuse `low_poly_cloth.glb` |
| `table` | wood×3 + fiber×1 | none | reuse `wood.glb` |

No tool-use bonuses / `uses` / `station` on these four except flour’s craft-station. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M58 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; extra invention kinds; ammo / projectile FX; per-instance durability stacks; every-recipe stations; time-decay wear; CPU percent; out-dir disk walk |
| Not M58 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; requiring a station for bread/stew; mixing work_places into dawn sleep bonus |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Place/Pickup reused. No new ControlVerb. No postcard `PrimaryAction` append.
2. **format_version still writes 3 / reads v2+v3.** `work_places` serde-default on World. `tool_uses` BoardBlob trailing + decode fallback.
3. `uses` omit = 0 (infinite). Max-held tool wears. Break consumes 1 qty.
4. Millstone is the only shipped station. Flour is the only shipped craft that needs it. Pocket millstone ≠ placed.
5. Shipped-objects idle hashes **change** (new files + millstone `station` + tool `uses` + fields hashed for every catalog row). No-objects `--no-time` stays `70e5204d…`. Viewer OTLP does not move hashes.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M58 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `b4e1eac7…` |
| default CLI 2 ticks (time on) | hash `aad2121f…` |
| catalog-off | axe/hammer never break; millstone Place illegal; flour Craft illegal |
| axe `uses = 8` | 8 successful veg Gathers consume 1 axe; 7 do not |
| millstone Place / Pickup | 1×1 land; overlap with tent illegal; Pickup returns it |
| flour Craft | illegal without placed millstone in Chebyshev 1; legal with; pocket millstone not enough |
| cart / bellows / table Craft | from locked inputs, no station |
| viewer frame JSON | helper contains `agentplace.viewer.frame_ns`; `cargo test -p viewer` no GPU |
| telemetry / OTLP sim-cli | still posts tick + process gauges; hashes unchanged vs no `--otlp-endpoint` |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Durability + millstone station

- `[sim] uses` / `station` / craft `station` slug; hash_catalog; `work_places`; Place/Pickup millstone; tool wear on Gather/Farm/Fish; BoardBlob `tool_uses`; millstone.toml + axe/hammer/hoe/net `uses = 8`

### PR 2: Viewer-frame OTLP

- `--otlp-endpoint` on viewer; `viewer_frame_metrics_json`; POST last frame ns; no GPU tests

### PR 3: Recipes

- flour/cart/bellows/table TOML; flour station gate; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new sim-cli flag. Viewer gains `--otlp-endpoint` (same URL rules as sim-cli).

On implement: `uses` on axe/hammer/hoe/net; `station` on millstone; create flour/cart/bellows/table TOML; viewer frame POST. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p viewer
cargo test -p sim-cli --test otlp_cli
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M58-test-plan.md`](M58-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `b4e1eac7…`; default (time on) `aad2121f…`; Hello v5; format_version 3 write.

## Risks

- **BoardBlob trailing `tool_uses`.** Decode fallback to today’s v3 blob; do not bump format_version. `--load` must restore wear, not re-apply a break.
- **Place overlap.** work_places must not share cells with sleep_places. Dawn must ignore millstones.
- **Pocket millstone ≠ station.** Flour Craft checks placed origin Chebyshev, not inventory.
- **Catalog `uses` / `station` hashed for every row** (omit=0/false), so shipped-objects hashes move even without Place/Gather.
- **OTLP stays JSON.** Viewer POST must not touch `state_hash` or AGTN.
- **Recipe inputs** stay distinct (food×3, wood×6, fiber×3+stone×1, wood×3+fiber×1).
