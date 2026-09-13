# M50 — Viewer FPS HUD, sqlite run log, extra recipes

**Status:** planned (not yet implemented)  
**Depends on:** M49 complete (`docs/M49-plan.md`, git tag `M49`, commit `189630e`)  
**Specs:** `docs/post-ga-feature-list.md` (PG-14 FPS HUD, PG-15 sqlite run log, PG-9 recipes)

## Context

The native Status window shows **sim tick** `wall_ms`, not render FPS. `--out-dir` appends events/decisions/timing **JSONL** only. Catalog crafts after M49 include hammer/hoe/waterskin/dried_fish plus earlier items; there is no club/pike/sling/bow and no satchel/rucksack/cooked_veg/bowl.

M50 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. FPS and sqlite are **hash-neutral**. New catalog `[sim]` files **do** change the default CLI idle hash (same pattern as M46/M49).

## Goal

A researcher can:

1. Open the native Bevy viewer and see **FPS** and **average frametime (ms)** on the Status window, including while the sim is paused. Distinct from sim tick `wall_ms`.
2. Pass **`--sqlite PATH`** on `sim-cli` and query **typed columns** (not a JSON blob) for the same facts JSONL records: world tick times, per-agent pipeline ns, decisions, events. JSONL still writes when `--out-dir` is set.
3. Turn on `--catalog` / objects dir and **Craft** containers/food (**satchel, rucksack, cooked_veg, bowl**) and **weapons** (**club, pike, sling, bow**). Catalog-off ⇒ idle no-objects hash unchanged.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

The attach page does **not** open the DB this slice. Schema is locked so a later slice can run those queries in the researcher UI.

No-objects 2-tick hash stays **`70e5204d…`**. Default CLI with shipped objects **changes** (document on implement).

## In scope

No PROTOCOL bump. No new `SimEventKind`. No new ControlVerb. Do not store FPS samples or sqlite handles on `Simulation`.

### A. Viewer FPS / frametime HUD (PG-14)

Native Bevy window only. Always-on Status line (no overlay, no CLI toggle).

| Number | Meaning |
|---|---|
| **FPS** | `1000 / mean_ms` from the last **60** Bevy `Time::delta` samples (0 if empty). |
| **avg frametime (ms)** | Mean of those 60 deltas, milliseconds. |

Status line (under the existing tick `wall_ms` line): `fps=59.8  frame_ms=16.7`. Update every render frame, including `--start-paused` / Pause.

- **Hash-neutral.** Not in `state_hash` or AGTN. No `SimEventKind`.
- Distinct from Charts / Status **sim tick** `wall_ms`.
- `cargo test -p viewer` must not need a GPU. Unit-test helper: a slice of deltas → fps + mean ms.
- Do not add a Charts polyline for frame time this slice.

### B. SQLite run log (PG-15) — unrolled columns

CLI only, not overlay, not `ExperimentConfig`:

```bash
sim-cli --sqlite PATH
```

**No JSON text column.** Same facts as JSONL, stored as columns so `SELECT` / a later website can compute medians without parsing JSON.

#### `ticks` — one row per sim tick (world)

| Column | Source |
|---|---|
| `tick` INTEGER PK | `TickTiming.tick` |
| `wall_ns` INTEGER | `TickTiming.wall_ns` |
| `world_ns` INTEGER | `world_ns` |
| `board_ns` INTEGER | `board_ns` |
| `incentive_ns` INTEGER | `incentive_ns` |
| `agents_ns` INTEGER | `agents_ns` |

Written when timing JSONL would be written.

#### `agent_timing` — one row per agent per tick

| Column | Source |
|---|---|
| `tick` INTEGER | |
| `agent` INTEGER | `AgentTiming.agent` |
| `perceive_ns` INTEGER | |
| `retrieve_ns` INTEGER | |
| `reflect_ns` INTEGER | |
| `plan_ns` INTEGER | |
| `select_ns` INTEGER | |
| `execute_ns` INTEGER | |
| `remember_ns` INTEGER | |

PK `(tick, agent)`. Same timing-on gate as `ticks`.

#### `decisions` — one row per DecisionRecord

| Column | Source |
|---|---|
| `tick` INTEGER | |
| `agent` INTEGER | |
| `chooser` TEXT | |
| `policy_branch` TEXT | |
| `call_seed` INTEGER | |
| `prompt_hash` TEXT | |
| `primary_action` TEXT | name of `primary` (`Wait`, `Craft`, …) |
| `speak` INTEGER | 0/1 |
| `reasoning` TEXT | nullable |

PK `(tick, agent)`. Nested legal names: child table `decision_legal (tick, agent, action TEXT)` — one row per legal name. Do not stuff arrays into a JSON column.

#### `events` — one row per SimEvent, sparse payload columns

| Column | Source |
|---|---|
| `tick` INTEGER | |
| `agent` INTEGER | |
| `kind` TEXT | JSONL `type` string (`wait`, `move`, `gather`, …) |
| `from_x` `from_y` `to_x` `to_y` INTEGER | Move |
| `x` `y` INTEGER | Farm |
| `item` TEXT | slug / Display of ItemId when present |
| `qty` INTEGER | |
| `success` INTEGER | Hunt/Fish/Craft 0/1 |
| `species` INTEGER | Gather/Farm |
| `toxic` INTEGER | Eat 0/1 |
| `shout` `broadcast` INTEGER | Speak |
| `text` TEXT | Speak |
| `proposal_id` INTEGER | Propose/Support/Oppose |
| `reason` TEXT | RuleBlocked |
| `incentive_id` `detail` TEXT | Incentive* |
| `hunger_zero` `thirst_zero` INTEGER | Died |
| `target` INTEGER | Transfer.to / Attack.target / Incapacitated.by / PairBonded.with / Invented.inventor |
| `damage` INTEGER | Attack |
| `parent_a` `parent_b` INTEGER | Born |
| `invent_kind` INTEGER | Invented.kind as u8 |
| `pipeline_stages` INTEGER | Pipeline |

Unused payload columns are NULL. Index `(tick)`, `(kind)`, `(agent, tick)`.

- **Hash-neutral extra sink.** Do **not** replace JSONL. `--out-dir` without `--sqlite` is today. `--sqlite` without `--out-dir` is DB only.
- Headless **and** `--listen` serve path both insert (same as JSONL).
- `rusqlite` **bundled**, **`sim-cli` only** (not `sim-core` / wasm). Create parent dirs like JSONL.
- `cargo test` uses a temp file. Never needs the network. Missing path / IO ⇒ process error (not a hashed event).
- Mock + no `--sqlite` ⇒ idle hashes **unchanged**. Flag on still does not change hashes.
- Website query UI / wasm-sql in `web/index.html` is **After M50**.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. **No new Attack/damage bonuses** (held catalog items only). Create these files on **implement**:

Containers / food:

| id | inputs | visual |
|---|---|---|
| `satchel` | fiber×2 + wood×1 | reuse `basket.glb` |
| `rucksack` | fiber×3 + wood×1 | reuse `backpack.glb` |
| `cooked_veg` | food×1 | reuse `plants_ready.glb` |
| `bowl` | stone×1 | reuse `stones_and_grass.glb` |

Weapons:

| id | inputs | visual |
|---|---|---|
| `club` | wood×1 | reuse `wood.glb` |
| `pike` | wood×2 + stone×1 | reuse `spear.glb` |
| `sling` | fiber×2 + stone×1 | reuse `stones_and_grass.glb` |
| `bow` | wood×2 + fiber×1 | reuse `fishing_rod.glb` |

Inputs must not clone an existing recipe (spear is wood×1+stone×1; knife is stone×1+fiber×1; hoe is stone×1+wood×1; hammer is stone×2+wood×1).

- Catalog-off / empty catalog / no objects dir ⇒ Craft of these illegal; no-objects hash **`70e5204d…`**.
- Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). v3 slugs remap on `--load`.
- Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M50 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; **website / attach page querying the sqlite file**; weapon combat bonuses |
| Not M50 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; OTel leftover (PG-11); GPU profiler; replacing JSONL; sqlite as checkpoint store; JSON blob column; recipe durability / workstations |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb / `SimEventKind`.
2. **format_version still writes 3 / reads v2+v3.**
3. FPS and sqlite never enter `state_hash` or AGTN.
4. Sqlite is **typed columns**, not a JSON text field. `rusqlite` bundled in **sim-cli only**.
5. New recipes change **shipped-objects** idle hash; no-objects hash unchanged.
6. Do not change shipping `coop.toml`. No TLS. CI never needs the network.
7. Website sqlite queries and weapon combat bonuses are After M50.

## Tests (M50 acceptance bar)

| Test | Asserts |
|---|---|
| no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | shipped-objects hash **changes** (lock on implement); no-objects `70e5204d…` |
| fps helper | 60 × 16.67 ms → fps ≈ 60, mean_ms ≈ 16.67; empty → 0 |
| fps not hashed | frame window does not change `state_hash` |
| sqlite `ticks` | one row per timed tick; `wall_ns` equals `TickTiming.wall_ns` (INTEGER, not JSON) |
| sqlite `agent_timing` | one row per living agent per timed tick; `select_ns` matches the timing struct |
| sqlite no json column | `pragma table_info` has no `json` column on these tables |
| sqlite `decisions` | `primary_action` is a name string; `decision_legal` has ≥1 row when legal is non-empty |
| sqlite `events` | `kind` text; Move fills `from_x`…; unused cols NULL |
| `--sqlite` no out-dir | DB created; no JSONL required |
| no `--sqlite` | no DB; idle hashes unchanged |
| catalog-off | all eight new Crafts illegal |
| catalog-on | Craft club from 1 wood; satchel from 2 fiber + 1 wood; pike from 2 wood + 1 stone |
| Hello v5 / format 3 | unchanged |

GPU window, live LLM, and wiring the browser to sqlite are **not** required.

## PR Plan

### PR 1: FPS HUD

- **Files:** viewer 60-sample frame ring; Status line; helper unit test

### PR 2: sqlite unrolled sink

- **Files:** `sim-cli --sqlite`; rusqlite bundled; `ticks` / `agent_timing` / `decisions` / `decision_legal` / `events`; headless + serve; temp-DB tests including `pragma table_info`

### PR 3: Extra recipes + weapons

- **Files:** `configs/objects/{satchel,rucksack,cooked_veg,bowl,club,pike,sling,bow}.toml`; Craft tests; document new idle hash

## Config / CLI

No shipping experiment TOML change. No PROTOCOL bump.

On implement: eight recipe files; `--sqlite PATH` on sim-cli. Update [`docs/cli-reference.md`](cli-reference.md) (`--sqlite`). Update [`docs/config-reference.md`](config-reference.md) recipe ids. Viewer help: Status shows `fps=` / `frame_ms=`.

```bash
cargo test -p viewer
cargo test -p sim-core --test objects
cargo test -p sim-cli --test net
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --sqlite /tmp/m50.sqlite3 --quiet
```

## Verification

Walkthrough: `docs/M50-test-plan.md` (written on implement).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: no-objects hash `70e5204d…`; shipped-objects idle hash **new** (document); Hello v5; format_version 3 write; sqlite tables have no `json` column.

## Risks

- Shipped-objects idle hash **will** change (eight new `[sim]` files). Document it.
- Do not hash FPS / sqlite I/O / glb bytes.
- rusqlite in sim-core would break wasm — **sim-cli only**, bundled.
- Event payload columns miss a field: lock the list above; new `SimEventKind` later adds nullable cols (append).
- `--load` must not replay sqlite from the checkpoint (this-process append log).
- `cooked_veg` input slug is builtin **food** (same as `dried_fish`).
- Shipping `default.toml` / `coop.toml` unchanged. `cargo test` never needs the network.
