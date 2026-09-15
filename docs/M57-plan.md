# M57 — Hammer stone-gather bonus, process CPU/disk telemetry, extra recipes

**Status:** implemented  
**Depends on:** M56 complete (`docs/M56-plan.md`, git tag `M56`, commit `852297e`)  
**Walkthrough:** [`docs/M57-test-plan.md`](M57-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes, PG-11 leftover CPU/disk); M56 Not-list stone-gather bonus

## Context

M56 gave axe vegetation Gather `skill_roll` bonus 25. Stone gather is still bonus 0 (`stone_gather_skill_bonus` always returns 0). Hammer is a catalog craft with **no** tool bonus. `[telemetry]` / `--telemetry` records tick aggregates + process RSS and can POST OTLP/JSON `wall_ns` + `rss_bytes`; there is no process CPU or disk. Catalog after M56 includes fence/mat/snare/spit.

M57 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files and `[sim] stone_gather_bonus` **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. CPU/disk telemetry is **hash-neutral**.

## Goal

A researcher can:

1. Hold a **hammer** and succeed **stone Gather** more often (`[sim] stone_gather_bonus = 25`). Catalog-off / not holding ⇒ today (bonus 0). Vegetation Gather / axe unchanged.
2. Turn on `[telemetry]` / `--telemetry` and see **hash-neutral** process **CPU** (user + system ns) and **disk** (read + write bytes) next to today’s RSS. Overlay off ⇒ today. OTLP/JSON POST includes the new gauges when `otlp_endpoint` is set. CI never dials.
3. **Craft** four new catalog items (**barrel, cloak, pot, lantern**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Gather unless tests `execute_primary`. No new CLI flag (hammer is catalog; CPU/disk ride `--telemetry`).

### A. Hammer stone-gather bonus

Hashed `[sim] stone_gather_bonus` (omit = 0). **Do not** reuse `gather_bonus` (that is vegetation only, M56).

| slug | field | value |
|---|---|---|
| `hammer` (existing file) | `stone_gather_bonus` | **25** |

`stone_gather_skill_bonus` today always returns 0. Use **max held** catalog `stone_gather_bonus` (pockets or pack), else 0. Holding two hammers / hammer+other stone tool ⇒ max, not sum.

- Axe `gather_bonus` does **not** apply to stone Gather.
- Hammer `stone_gather_bonus` does **not** apply to vegetation Gather.
- Qty stays today’s STR formula. No basket interaction (basket is vegetation/food).
- Catalog-off / empty catalog / not holding ⇒ identity (0).
- `--load` restores inventory; do not persist derived odds.

`hash_catalog` writes `stone_gather_bonus` u32 LE for **every** row (omit=0), same as `gather_bonus`. Shipped-objects idle hashes move even without Gather happening.

### B. Process CPU/disk telemetry (PG-11 leftover)

Existing overlay. No new flag.

```toml
[telemetry]
enabled = false
# otlp_endpoint = ""     # omit / empty = in-process only
```

CLI: `--telemetry` / `--otlp-endpoint URL` unchanged (`--otlp-endpoint` still implies telemetry on).

Best-effort Linux `/proc`, `None` if the host cannot report (same pattern as `process_rss_bytes`):

| Helper | Source | Stored on Simulation (not hashed, not in ckpt) |
|---|---|---|
| `process_cpu_user_ns` | `/proc/self/stat` field 14 `utime` × (1e9 / CLK_TCK) | `telemetry_cpu_user_ns` last |
| `process_cpu_system_ns` | `/proc/self/stat` field 15 `stime` | `telemetry_cpu_system_ns` last |
| `process_disk_read_bytes` | `/proc/self/io` `read_bytes:` | `telemetry_disk_read_bytes` last |
| `process_disk_write_bytes` | `/proc/self/io` `write_bytes:` | `telemetry_disk_write_bytes` last |

CLK_TCK via `sysconf(_SC_CLK_TCK)`, fallback **100**. Values are **cumulative** last (like RSS last), not percent, not peak.

When `telemetry_enabled`, sample after each tick next to RSS. Overlay **off** ⇒ leave `None`, do not read `/proc`. `--load` / ckpt decode **zeros** these fields (same as RSS). Do not store them on the postcard blob.

OTLP/JSON gauges (sim-cli, names locked), in addition to `agentplace.tick.wall_ns` and `agentplace.process.rss_bytes`:

| name | value |
|---|---|
| `agentplace.process.cpu_user_ns` | last or 0 |
| `agentplace.process.cpu_system_ns` | last or 0 |
| `agentplace.process.disk_read_bytes` | last or 0 |
| `agentplace.process.disk_write_bytes` | last or 0 |

Still OTLP/**JSON** HTTP POST. No protobuf. No gRPC. No TLS. `cargo test` never sets a public collector. Loopback test asserts the four new names in the POST body. Overlay off / no flag ⇒ **same hashes**.

Do **not** add viewer-frame OTLP, process percent CPU, out-dir walk, or a telemetry `SimEventKind`.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `barrel` | wood×5 | reuse `wood.glb` |
| `cloak` | fiber×6 | reuse `low_poly_cloth.glb` |
| `pot` | stone×4 | reuse `stones_and_grass.glb` |
| `lantern` | wood×1 + fiber×1 + stone×1 | reuse `wood.glb` |

No tool-use bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | Done — [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| **M60** | Done — [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| **M61** | Done — [`M61-plan.md`](M61-plan.md) — time-decay wear, out-dir disk walk, extra recipes |
| **M62** | Done — [`M62-plan.md`](M62-plan.md) — stations for bread/stew, Store wear, extra recipes |
| **M63** | [`M63-plan.md`](M63-plan.md) — crate dawn decay, remaining food stations, extra recipes |
| After M63 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX |
| Not M57 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; applying `gather_bonus` to stone or `stone_gather_bonus` to vegetation; CPU percent; out-dir disk walk |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard append.
2. **format_version still writes 3 / reads v2+v3.** Odds and telemetry samples are derived / host; do not store them.
3. `stone_gather_bonus` omit = 0. Max held, not sum. Stone Gather only. Hammer does not change vegetation Gather. Axe does not change stone Gather. Qty unchanged.
4. CPU/disk ride existing `[telemetry]` / `--telemetry`. Hash-neutral. Linux `/proc`; `None` elsewhere. Cumulative last, not percent.
5. Shipped-objects idle hashes **change** (new files + hammer bonus + `stone_gather_bonus` hashed for every catalog row). No-objects `--no-time` stays `70e5204d…`. Telemetry does not move hashes.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M57 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `e2848016…` |
| default CLI 2 ticks (time on) | hash `35c79482…` |
| catalog-off stone Gather | bonus 0 |
| hold hammer, catalog on | stone Gather `skill_roll` bonus 25 vs 0 without; two tools max not sum |
| vegetation Gather with hammer | still basket +15 / axe 25 / else 0 (hammer does not add) |
| stone Gather with axe | still 0 |
| telemetry off | same hash as no flag |
| telemetry on | hash unchanged vs off; on Linux last CPU/disk `Some` after a tick |
| OTLP JSON | POST includes the four new gauge names; still `rss_bytes` + `wall_ns` |
| Craft barrel / cloak / pot / lantern | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Hammer stone_gather_bonus

- `[sim] stone_gather_bonus`; max held; vegetation identity; hammer.toml = 25; hash_catalog field

### PR 2: Process CPU/disk

- `/proc` helpers next to RSS; Simulation last fields; OTLP JSON gauges; overlay-off hash identity; loopback name asserts

### PR 3: Recipes

- barrel/cloak/pot/lantern TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (hammer is catalog; CPU/disk ride `--telemetry`).

On implement: `stone_gather_bonus` on hammer; sample CPU/disk when telemetry on; create barrel/cloak/pot/lantern TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test telemetry
cargo test -p sim-cli --test otlp_cli
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M57-test-plan.md`](M57-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `e2848016…`; default (time on) `35c79482…`; Hello v5; format_version 3 write.

## Risks

- **Stone vs vegetation.** Do not feed `gather_bonus` into stone Gather or `stone_gather_bonus` into vegetation Gather.
- **Catalog `[sim] stone_gather_bonus`** hashed for every entry (omit=0), so shipped-objects hashes move even without Gather happening.
- **CPU/disk never hashed.** `/proc` missing ⇒ `None` / OTLP 0, not a test fail on non-Linux.
- **OTLP stays JSON.** Do not sneak protobuf/gRPC.
- **Recipe inputs** must stay distinct from shipped crafts (wood×5, fiber×6, stone×4, wood+fiber+stone×1).
