# M46 — Sheet leftovers, extra recipes, OpenTelemetry

**Status:** implemented  
**Depends on:** M45 complete (`docs/M45-plan.md`, git tag `M45`, commit `a27146d`)  
**Walkthrough:** [`docs/M46-test-plan.md`](M46-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 remaining CON/INT/STR, PG-9 recipes, PG-11 telemetry)

## Context

M42–M44 shipped CON energy max + illness **duration**, INT memory/retrieval_k, STR haul/pocket weight. Toxic eat still **always** applies illness. `[llm] plan_length` stays 4. Gather/hunt qty ignores STR. Craft catalog is Basket/Spear/FishingRod/Backpack plus `cord.toml`. Tick timing JSONL is hash-neutral; there is no OpenTelemetry, no run-level aggregates, no process RSS.

M46 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[agents.sheet]` / `--sheet` and see **CON illness chance**, **INT plan length**, and **STR gather/hunt payoff**. Unused scores ⇒ today’s always-sick / plan_length 4 / qty ×1.
2. Turn on `--catalog` / objects dir and **Craft** new catalog items **plank, charcoal, knife, net** (object TOML only; cord already ships). Catalog-off / empty catalog ⇒ idle hash unchanged.
3. Turn on `[telemetry] enabled` / `--telemetry` and see **hash-neutral** sim-tick aggregates (count, total, average, median, min, max) plus process RSS. OTLP only if `otlp_endpoint` is set (CI never dials). Overlay off ⇒ no exporter, **same hashes**.
4. `--load` restores scores and catalog slugs; do not persist derived chance/length/payoff; do not emit extra telemetry on decode. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Default `sim-cli` loads shipped objects, so idle mock 2-tick hash is **`5f231378…`** (four new recipes in the catalog). Without an objects dir the hash stays `70e5204d…`. Telemetry off does not change hashes.

## In scope

No PROTOCOL bump. No new `SimEventKind` for telemetry. Postcard: catalog items stay `ItemId::Catalog` + slug (M45). Do not reorder ItemId builtins.

### A. Sheet leftovers (PG-4)

Existing `[agents.sheet] enabled` / `--sheet`. Derived at use time. Score 0 ⇒ today’s constants. Invent-style `derive_seed`, not `RngBank.ensure`. Do not store derived chance/length/qty. `--load` cannot double-apply.

| Use | Score 0 (today) | On |
|---|---|---|
| CON illness **chance** | toxic eat always sets `illness_ticks` (duration already CON) | CON 0 always. Else `d20 + CON_mod >= 12` on `tick_{t}_agent_{id}_illness_0`. Miss ⇒ no illness (duration unused). |
| INT plan length | `[llm] plan_length` default **4** | When INT ≠ 0: `max(1, 4 + INT_mod)`. INT 18 ⇒ 8; INT 3 ⇒ 1. Overlay `plan_length` is the unused/base. |
| STR gather/hunt qty | species qty as today | `qty * (1000 + STR_mod*50) / 1000`, min 1 if any. Fish/farm unchanged. |

- `--sheet` does not imply `--catalog` or `--telemetry`.
- Mock does not pick Craft/Invent/PairBond unless already true.

### B. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create these files on **implement**:

| id | inputs | visual |
|---|---|---|
| `plank` | wood×2 | reuse `wood.glb` |
| `charcoal` | wood×1 | reuse `wood.glb` |
| `knife` | stone×1 + fiber×1 | reuse `spear.glb` |
| `net` | fiber×3 | reuse `fishing_rod.glb` |

- Catalog-off / empty catalog ⇒ Craft of these illegal; idle hash **unchanged**.
- Catalog-on hash **changes** (document). v3 slugs remap on `--load`.
- No new gather/hunt **tool** bonuses (STR payoff is the sheet item).
- Missing glb ⇒ today’s primitive (**PG-10** later).

### C. Telemetry (PG-11)

Overlay, not postcard, not `ExperimentConfig`:

```toml
[telemetry]
enabled = false          # omit = false
# otlp_endpoint = ""     # omit / empty = in-process only; no network
```

CLI: `--telemetry`. Does not imply `--sheet` or `--catalog`.

When **on**:

- Keep existing timing JSONL. Do **not** replace it.
- In-memory run stats from `TickTiming.wall_ns`: **count, total, average, median, min, max**.
- Process **RSS** (and peak) sampled per tick. Hash-neutral.
- Viewer **frame** times: native window only. Unit-test the aggregator with fake samples (no GPU).
- OTLP HTTP/gRPC **only** if `otlp_endpoint` is non-empty. `cargo test` never sets it.

Overlay **off**: no exporter, no extra required threads, **same hashes**. `--load`: restore sim; do not emit telemetry events on decode. Do not hash RSS / ns / frames.

## Out of scope (later)

| Later | What |
|---|---|
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| After M48 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; LLM invention text; Food/Wood as strings |
| Not M46 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; hashing wall-clock ns; recipe durability / workstations |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No telemetry `SimEventKind`.
2. **format_version still writes 3 / reads v2+v3.**
3. Unused CON/INT/STR and telemetry/recipes overlay off ⇒ **same hashes** as M45 idle.
4. CON 0 toxic-eat still always applies illness. Chance is only when CON ≠ 0.
5. Recipes are catalog files, not enum appends. Catalog-on hash change is acceptable and documented.
6. Telemetry default off. OTLP is opt-in URL. CI never needs the network.
7. Do not change shipping `coop.toml`. No TLS.

## Tests (M46 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | shipped objects hash `5f231378…`; no-objects `70e5204d…` |
| unused CON/INT/STR | always-sick; plan length 4; gather qty identity |
| CON 18 vs 3 | 18 resists more seeded illness rolls than 3; CON 0 always sick |
| INT 18 vs 3 | plan length 8 vs 1 |
| STR 18 vs 3 | gather/hunt qty higher for 18 |
| catalog-off | plank/knife not legal; same hash as no extra files |
| catalog-on | Craft plank from 2 wood; catalog-on hash ≠ off |
| telemetry off | same hash as no flag; no OTLP |
| telemetry on | aggregates match a known `wall_ns` series; hash **unchanged** vs off |
| Hello v5 / format 3 | unchanged |

## PR Plan

### PR 1: Sheet leftovers

- **Files:** CON illness chance; INT plan length; STR gather/hunt qty; unused identity tests

### PR 2: Extra recipes

- **Files:** `configs/objects/{plank,charcoal,knife,net}.toml`; catalog Craft tests; catalog-off identity

### PR 3: Telemetry overlay

- **Files:** `[telemetry]` / `--telemetry`; in-process aggregates + RSS; optional `otlp_endpoint` (untested live); hash identity

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --catalog --telemetry --quiet
```

## Verification

Walkthrough: [`docs/M46-test-plan.md`](M46-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: default CLI 2-tick hash `5f231378…`; unused sheet / catalog-off / telemetry-off identity; Hello v5; format_version 3 write.

## Risks

- **Idle hash:** telemetry default off; extra recipes catalog-off; unused sheet identity.
- CON chance must not change CON 0 toxic-eat.
- Catalog-on hash ≠ old catalog-on if the new files load (acceptable; document).
- Do not hash RSS / wall-clock ns / frames (non-deterministic).
- `--load` must not re-roll illness or re-grant crafts.
- Shipping `default.toml` / `coop.toml` unchanged. No PROTOCOL bump. `cargo test` never dials OTLP.
