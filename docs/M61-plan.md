# M61 — Time-decay wear, out-dir disk walk, extra recipes

**Status:** implemented  
**Depends on:** M60 complete (`docs/M60-plan.md`, git tag `M60`, commit `2ac5a5d`)  
**Walkthrough:** [`docs/M61-test-plan.md`](M61-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes, PG-11 leftover out-dir disk walk); M60 later-table time-decay wear

## Context

M60 transferred the freshest `tool_wear` slot on Give. Tools still only wear on a successful bonus-use. `[telemetry]` records CPU ns/percent and process disk bytes; it does not walk `--out-dir`. Catalog after M60 includes stool/sash/brick/biscuit.

M61 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Time-decay is empty on idle mock (no tools held) ⇒ idle hashes **do not** move from this field. Out-dir walk is **hash-neutral**. Not per-tick decay, not Store transfer, not every-recipe stations.

## Goal

A researcher can:

1. Hold an **axe** overnight and see it **wear at dawn** (+1 on the most-worn instance, same `wear_tool` path as a bonus-use). `--no-time` / clock off ⇒ no decay. 8 dawns with `uses = 8` still consume 1. Idle mock holds no tools ⇒ idle hashes unchanged from this field.
2. Turn on `[telemetry]` / `--telemetry` with `--out-dir DIR` and see **hash-neutral** bytes under that directory next to today’s CPU/disk gauges. Overlay off or no `--out-dir` ⇒ None. OTLP/JSON POST includes the new gauge when `otlp_endpoint` is set. CI never dials.
3. **Craft** four new catalog items (**bench, shawl, cobble, cake**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Craft / Gather unless tests `execute_primary`. No new CLI flag (decay rides `[time]`; disk walk rides `--telemetry` + `--out-dir`).

### A. Time-decay wear (M59 leftover)

Reuse `wear_tool` (most-worn slot +1; break when `>= uses`). No new catalog field. No new overlay.

When `apply_dawn_if_due` runs (`time_enabled` and `tick > 0 && tick % ticks_per_day == 0`): for each **living** agent, for each catalog item with `uses > 0` that the agent **holds** (pockets or pack), call `wear_tool` **once** per ItemId.

- Two axes: only the most-worn slot +1 (same as a successful Gather).
- `--no-time` / `time_enabled = false` ⇒ no dawn ⇒ no decay.
- Dawn energy refill stays as today; decay runs **after** refill so a broken tool cannot affect that dawn’s shelter.
- `--load` restores `tool_wear`; do not re-apply a dawn already in the checkpoint tick.

Hash `tool_wear` already. Idle mock holds no tools ⇒ idle hashes unchanged from this field.

`uses = 8` on axe/hammer/hoe/net unchanged.

### B. Out-dir disk walk (PG-11 / M57 leftover)

Existing overlay. No new flag.

```toml
[telemetry]
enabled = false
# otlp_endpoint = ""     # omit / empty = in-process only
```

CLI: `--telemetry` / `--otlp-endpoint URL` unchanged. `--out-dir DIR` already exists.

When `telemetry_enabled` and an out-dir path is set on the sim (sim-cli copies `-o` / implied checkpoint dir into a **non-hashed** `telemetry_out_dir: String`; empty ⇒ skip):

| Field | Meaning |
|---|---|
| `telemetry_out_dir_bytes` | `Option<u64>`. Sum of regular-file lengths under that directory (recursive). `None` if the path is empty, missing, or unreadable. |

- Do **not** follow symlinks. Skip unreadable entries; do not fail the tick.
- Overlay **off** or empty path ⇒ leave None, do not walk. `--load` / ckpt decode **zeros** (same as RSS). Not hashed, not in BoardBlob.
- Distinct from process `disk_read_bytes` / `disk_write_bytes` (those stay `/proc`).

OTLP/JSON extra gauge (sim-cli):

| name | value |
|---|---|
| `agentplace.process.out_dir_bytes` | last or 0 |

Still OTLP/**JSON** HTTP POST. No protobuf. No gRPC. No TLS. `cargo test` never sets a public collector. Loopback test asserts the new name in the POST body. Overlay off / no flag ⇒ **same hashes**.

Do **not** add OTLP protobuf/gRPC or a telemetry `SimEventKind`.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `bench` | wood×9 | reuse `wood.glb` |
| `shawl` | fiber×9 | reuse `low_poly_cloth.glb` |
| `cobble` | stone×7 | reuse `stones_and_grass.glb` |
| `cake` | food×6 | reuse `plants_ready.glb` |

No `uses` / `station` / tool bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M61 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX; every-recipe stations; household-home / invention / downed meshes; transferring wear on Store |
| Not M61 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; per-tick (non-dawn) decay; decaying every instance not just most-worn; following symlinks |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard `PrimaryAction` append.
2. **format_version still writes 3 / reads v2+v3.** `tool_wear` already on the blob. Out-dir bytes are host-only; do not store them in ckpt.
3. Decay is **dawn only**, one `wear_tool` per held `uses>0` ItemId (most-worn slot). `--no-time` does not decay.
4. Out-dir walk is recursive regular files, no symlinks. Empty / missing path ⇒ None. Hash-neutral.
5. Shipped-objects idle hashes **change** (new files). No-objects `--no-time` stays `70e5204d…`. Decay and telemetry do not move idle hashes by themselves.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M61 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `16804536…` |
| default CLI 2 ticks (time on) | hash `3512dde6…` |
| dawn decay | time on, `ticks_per_day = 2`, hold axe wear `[0]`, run 2 ticks ⇒ wear `[1]` |
| 8 dawns | `uses = 8` consume 1 axe |
| `--no-time` hold axe | many ticks, wear unchanged |
| two axes at dawn | most-worn +1; the other stays 0 |
| telemetry off | same hash; `telemetry_out_dir_bytes` is None |
| telemetry on, no out-dir | same hash; bytes None |
| telemetry on + temp dir with a file | `Some(n)` with `n >=` that file’s len; hash unchanged |
| OTLP JSON | POST includes `agentplace.process.out_dir_bytes` |
| Craft bench / shawl / cobble / cake | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Time-decay wear

- Dawn `wear_tool` per held `uses>0` ItemId; `--no-time` identity; 8-dawn break; two-axe most-worn

### PR 2: Out-dir disk walk

- `dir_size_bytes`; `telemetry_out_dir` / `_bytes`; sim-cli copies `-o`; OTLP gauge; telemetry tests

### PR 3: Recipes

- bench/shawl/cobble/cake TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (`--telemetry` / `--out-dir` / `[time]` already exist).

On implement: dawn decay; out-dir walk; create bench/shawl/cobble/cake TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test telemetry
cargo test -p sim-cli --test otlp_cli
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M61-test-plan.md`](M61-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `16804536…`; default (time on) `3512dde6…`; Hello v5; format_version 3 write.

## Risks

- **Dawn vs per-tick.** Decay only at dawn (`tick % ticks_per_day == 0`, `tick > 0`). `--no-time` must not decay.
- **Most-worn only.** One `wear_tool` per ItemId, not per instance. Two axes: one slot ages.
- **`--load`.** Do not re-fire dawn decay for the restored tick.
- **Out-dir missing.** None, do not fail the tick. Do not follow symlinks.
- **Catalog files** change shipped-objects hashes even without decay/Craft.
- **Recipe inputs** stay distinct (wood×9, fiber×9, stone×7, food×6).
- **No protocol / format bump.** `tool_wear` already on the blob; out-dir bytes are host-only.
