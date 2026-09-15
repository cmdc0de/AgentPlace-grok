# M49 — Object visual scale, extra recipes, invention flavor text

**Status:** implemented  
**Depends on:** M48 complete (`docs/M48-plan.md`, git tag `M48`, commit `b2b751a`)  
**Walkthrough:** [`docs/M49-test-plan.md`](M49-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-13 scale, PG-9 recipes, PG-5 LLM invention text)

## Context

Object TOML has `[visual] glb` + LOD but no scale. Agent glbs auto-fit to the capsule (1.11). Craft catalog after M46 is cord/plank/charcoal/knife/net plus builtins. Invent success stores kind only; there is no flavor sentence. Mock never HTTP.

M49 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. Scale is **hash-neutral**. New catalog `[sim]` files **do** change the default CLI idle hash (same pattern as M46). Flavor is hashed when non-empty.

## Goal

A researcher can:

1. Set `[visual] scale = N` on an object TOML and see that glb uniformly scaled in the native viewer. Omit ⇒ today (agent **auto-fit**; others **1.0**). Explicit finite `scale > 0` **replaces** agent auto-fit.
2. Turn on `--catalog` / objects dir and **Craft** **hammer, hoe, waterskin, dried_fish** (object TOML only). Catalog-off / empty catalog ⇒ idle no-objects hash unchanged.
3. On successful Invent, get a **flavor string** (mock: `kind.memory_text()`; live: extra `Chooser` call). Timeout / error ⇒ mock text. Overlay off ⇒ no Invent, **same idle hashes**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

No-objects 2-tick hash stays **`70e5204d…`**. Default CLI with shipped objects is **`133ea72e…`**.

## In scope

No PROTOCOL bump. No new `SimEventKind` variant or extra Invented fields (flavor is on the invention **table** only). Postcard enums append-only if anything is added (prefer none).

### A. Object visual scale (PG-13)

`VisualDef.scale: Option<f32>`. Overlay/visual — **not hashed**.

| Value | Viewer |
|---|---|
| omitted / `None` | Agent: M48 capsule auto-fit. Other ids: scale **1.0**. |
| finite `> 0` | Uniform XYZ. **Skip** agent auto-fit. |
| `<= 0` / NaN / inf | Treat as omitted. Do not fail CI. |

- Primitive and sentinel: no TOML scale.
- LOD uses the same object `scale` (no per-LOD scale this slice).
- Native window only. Unit-test: def.scale 2.0 → fit/transform scale 2.0 with fake AABB; omit agent still auto-fits.
- Create **no** shipping scale values unless a file already needs one; omit everywhere by default.

### B. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create these files on **implement**:

| id | inputs | visual |
|---|---|---|
| `hammer` | stone×2 + wood×1 | reuse `stone.glb` / `stones_and_grass.glb` |
| `hoe` | stone×1 + wood×1 | reuse `spear.glb` |
| `waterskin` | fiber×2 | reuse `basket.glb` |
| `dried_fish` | food×1 | reuse `lowpoly_fish.glb` / `fish_animated.glb` |

- Catalog-off / empty catalog / no objects dir ⇒ Craft of these illegal; no-objects hash **`70e5204d…`**.
- Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). v3 slugs remap on `--load`.
- No new gather/hunt **tool** bonuses this slice.
- Missing glb ⇒ M47 sentinel.

### C. Invention flavor text (PG-5 leftover)

On **successful** Invent only:

- Append `flavor: String` on `Invention` (`#[serde(default)]` so old ckpts load empty).
- **Mock / default Chooser:** `kind.memory_text()` (e.g. `invented gather bonus`). Deterministic. No HTTP.
- **Live Chooser:** new `ActionChooser::invent_flavor(seed, kind_slug, obs)` defaulting to mock text. Timeout / `ChooseError` ⇒ mock text (still hashed). Seed `tick_{t}_agent_{id}_invent_flavor_0` via `derive_seed` (not `RngBank.ensure`).
- Hash `flavor` bytes in `hash_table` (empty string hashes as today for old rows).
- Do **not** add Invented event fields (no PROTOCOL bump). Inspector / Markdown may show flavor (hash-neutral display of an already-hashed field).
- Overlay off ⇒ Invent illegal; idle hash **unchanged**.
- `--load` restores flavor; do not re-call the LLM on decode.
- `cargo test` never sets a live endpoint.

## Out of scope (later)

| Later | What |
|---|---|
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| After M59 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL |
| Not M49 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; mouse-drag orbit; per-LOD / non-uniform scale; recipe tool bonuses |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. Invented postcard unchanged.
2. **format_version still writes 3 / reads v2+v3.** Flavor uses serde default.
3. Scale is visual-only. Agent omit ⇒ auto-fit; explicit scale replaces auto-fit.
4. New recipes change **shipped-objects** idle hash; no-objects hash unchanged.
5. Mock flavor is `memory_text()`. Live is extra call; failure falls back to mock.
6. Do not change shipping `coop.toml`. No TLS. CI never needs the network.

## Tests (M49 acceptance bar)

| Test | Asserts |
|---|---|
| no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | shipped objects hash `133ea72e…`; no-objects `70e5204d…` |
| scale omit | agent auto-fit path still used; other id scale 1.0 |
| scale 2.0 | uniform 2.0; agent skips auto-fit |
| scale 0 / NaN | treated as omit |
| scale not hashed | changing only scale leaves `state_hash` unchanged |
| catalog-off | hammer/hoe/waterskin/dried_fish Craft illegal; no-objects hash |
| catalog-on | Craft hammer from 2 stone + 1 wood; dried_fish from 1 food |
| flavor mock | successful Invent stores `invented gather bonus`; hashed |
| flavor load | `--load` keeps flavor; no second call |
| inventions off | Invent illegal; idle hash unchanged |
| Hello v5 / format 3 | unchanged |

GPU window and live LLM are **not** required.

## PR Plan

### PR 1: Visual scale

- **Files:** `VisualDef.scale`; viewer apply; agent auto-fit vs explicit; hash identity

### PR 2: Extra recipes

- **Files:** `configs/objects/{hammer,hoe,waterskin,dried_fish}.toml`; Craft tests; document new idle hash

### PR 3: Invention flavor

- **Files:** `Invention.flavor`; mock `memory_text`; `Chooser::invent_flavor`; hash + load tests

## Config / CLI

No shipping experiment TOML change. No PROTOCOL bump.

On implement: four recipe files; optional `scale` on existing object TOML **only** if a walkthrough needs a demo (prefer omit). Update [`docs/config-reference.md`](config-reference.md) `[visual] scale` and recipe ids. [`docs/cli-reference.md`](cli-reference.md) unchanged unless a new flag appears (none planned).

```bash
cargo test -p sim-core --test objects
cargo test -p sim-core --test inventions
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

## Verification

Walkthrough: [`docs/M49-test-plan.md`](M49-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: no-objects hash `70e5204d…`; shipped-objects idle hash `133ea72e…`; Hello v5; format_version 3 write.

## Risks

- Shipped-objects idle hash **will** change (four new `[sim]` files). Document it.
- Do not hash `scale` / glb bytes / mtime.
- Flavor on the table **is** hashed; mock text must be stable.
- `--load` must not re-call invent_flavor.
- Explicit agent `scale` must not fight capsule auto-fit.
- `dried_fish` input slug is builtin **food**. If Craft parse rejects it, fix the catalog input map — do not add a new ItemId arm.
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never dials an LLM.
