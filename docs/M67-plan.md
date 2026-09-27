# M67 — Every-instance wear, extra recipes

**Status:** implemented  
**Depends on:** M66 complete (`docs/M66-plan.md`, git tag `M66`, commit `848835a`)  
**Walkthrough:** [`docs/M67-test-plan.md`](M67-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-3 / RF-PG9

## Context

Tool wear (dawn M61/M63, per-tick M66) increments the **most-worn** instance only. Catalog after M66 includes post/muffler/lintel/scone.

M67 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New recipe files **change** shipped-objects idle hashes. `[wear] every_instance` hashes **only when true** (omit = off; distinct tag from `per_tick`). Overlay off ⇒ M66 most-worn identity. `--no-time` no-objects stays **`70e5204d…`**. Not millstone foods, not ammo, not bonus-use wearing every instance.

## Goal

A researcher can:

1. **Turn on** every-instance tool wear (`[wear] every_instance = true` / `--every-instance-wear`): when a decay pass runs (dawn, or each tick if `--per-tick-wear`), **every** wear slot of that held/crate item +1, then consume each slot that reaches `uses`. Overlay off ⇒ M66 most-worn-only. Successful bonus-use (Gather/Farm/…) still wears **one** most-worn instance.
2. **Craft** four new catalog items (**strut, hood, sill, muffin**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
3. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never Invent / Craft / Gather unless tests `execute_primary`.

### A. Every-instance wear (RF-3)

```toml
[wear]
every_instance = true    # omit = false
```

CLI: `--every-instance-wear`. Same or-with-overlay pattern as `--per-tick-wear`.

Hash `[wear] every_instance` **only when true**, with a **distinct tag** from `per_tick` (`per_tick` → `[1u8]`, `every_instance` → `[2u8]`). Both false ⇒ no extra wear bytes.

Who/when: existing decay pass (`apply_tool_wear`) — dawn when per-tick is off; each tick when per-tick is on (dawn wear still skipped). Held then crates, living agents, `uses > 0` items.

When on: increment **all** slots for that item in one call, then remove every slot `>= uses` (high index first) and `take_item` / crate qty 1 per removed slot.

When off: today’s most-worn `wear_tool`. `dawn_decays_most_worn_axe_only` stays.

Bonus-use in `execute` is **not** every-instance.

`--load` restores wear vecs; do not extra-wear on decode.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `strut` | wood×14 | reuse `wood.glb` |
| `hood` | fiber×14 | reuse `low_poly_cloth.glb` |
| `sill` | stone×12 | reuse `stones_and_grass.glb` |
| `muffin` | food×11 | reuse `plants_ready.glb` |

No `uses` / `station` / interior. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope

| Not M67 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Wear | per-tick **and** dawn both firing; bonus-use wearing every instance (**RF-22**) |
| Also not | millstone remaining foods (**RF-4**); ammo (**RF-19**); interior walls (**RF-21**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Overlay, not a default change. Most-worn remains shipping identity.
4. Distinct hash tags so `per_tick` and `every_instance` cannot collide.
5. Decay pass only. Using a tool still wears one instance.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M67 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `092c3b9e…` |
| `--no-time` shipped objects | hash `9916abd1…` |
| overlay off, two axes `[3, 0]` | dawn → `[4, 0]` (M61 identity) |
| overlay on, 1 dawn, `[3, 0]` | `[4, 1]` |
| overlay on, crate two axes | both crate slots +1 |
| overlay on + `--per-tick-wear`, 1 tick | both held slots +1; no dawn needed |
| overlay on, `[7, 7]`, `uses = 8`, 1 pass | consume 2 |
| `--no-time` + overlay on + per-tick | still wears every instance |
| `--load` | no extra wear |
| overlay off idle | hash unchanged from this field |
| overlay on vs off (no tools) | hashes differ (tag `[2u8]`) |
| Craft strut / hood / sill / muffin | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Every-instance wear

- `[wear] every_instance` / `--every-instance-wear`; distinct hash tag; held+crate; tests

### PR 2: Recipes

- strut/hood/sill/muffin; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `[wear] every_instance`; `--every-instance-wear`; strut/hood/sill/muffin. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M67-test-plan.md`](M67-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `9916abd1…`; default (time on) `092c3b9e…`; Hello v5; format_version 3 write.

## Risks

- **Hash collision.** `every_instance` must not reuse `per_tick`’s `[1u8]` tag.
- **Multi-break.** Increment all, then consume all `>= uses` in one pass.
- **Catalog hash.** Recipes move idle hashes. Wear overlay omit-hash when false.
- **`--load`.** Restore vecs; do not extra-wear on decode.
- **No protocol / format bump.**
