# M60 — Transfer wear on Give, CPU percent, extra recipes

**Status:** implemented  
**Depends on:** M59 complete (`docs/M59-plan.md`, git tag `M59`, commit `77cf4d1`)  
**Walkthrough:** [`docs/M60-test-plan.md`](M60-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-9 recipes, PG-11 leftover CPU percent); M59 later-table transferring wear on Give

## Context

M59 gave per-instance `tool_wear` (most-worn breaks first). **Transfer** still `clamp_tool_wear`s from the vec end, so the receiver gets a fresh instance. `[telemetry]` / `--telemetry` records cumulative process CPU ns (user + system) and disk bytes; there is no CPU **percent**. Catalog after M59 includes raft/sandals/mortar/jerky.

M60 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. New catalog files **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Transfer wear is empty on idle mock ⇒ idle hashes **do not** move from this field. CPU percent is **hash-neutral**. Not time-decay, not Store transfer, not every-recipe stations.

## Goal

A researcher can:

1. **Transfer** a worn axe to another agent and see the **same wear slot** arrive with the item (Give no longer drops freshest wear). Store still truncates freshest and does not put wear on crates. Researcher `/give` still mints fresh instances.
2. Turn on `[telemetry]` / `--telemetry` and see **hash-neutral** process **CPU percent** (0..=100) next to today’s cumulative CPU ns. Overlay off ⇒ today. OTLP/JSON POST includes the new gauge when `otlp_endpoint` is set. CI never dials.
3. **Craft** four new catalog items (**stool, sash, brick, biscuit**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Mock never picks Transfer / Craft unless tests `execute_primary`. No new CLI flag (wear rides Transfer; percent rides `--telemetry`).

### A. Transfer wear on Give (M59 leftover)

Today `PrimaryAction::Transfer` takes qty then `clamp_tool_wear` **drops** freshest slots (vec end). Receiver `try_add_item` gets implicit 0. Store uses the same take path.

On **successful Transfer** of `added` qty:

1. Before take: split **`added` slots from the giver’s vec end** (same slots M59 would drop). If the vec is shorter than qty, missing slots are **0** (fresh).
2. Take items as today.
3. Append those `added` wear values to the **receiver’s** `tool_wear[item]`, then clamp both agents to held qty.
4. If `added < moved` (receiver pocket-full): return leftover items **and** leftover wear slots to the giver (do not drop them).

Store / Retrieve / Pack / Unpack / Eat / researcher `give_item` (`/give`): **unchanged**. Store still truncates freshest; crates have no wear. `/give` mints fresh (no wear to copy).

Hash `tool_wear` already (item + each u32 LE). Idle mock does not Transfer ⇒ idle hashes unchanged from this field. `--load` restores vecs; do not re-transfer.

Ckpt / format_version unchanged (still trailing `tool_wear` + M58 u32 fallback).

### B. CPU percent telemetry (PG-11 / M57 leftover)

Existing overlay. No new flag.

```toml
[telemetry]
enabled = false
# otlp_endpoint = ""     # omit / empty = in-process only
```

CLI: `--telemetry` / `--otlp-endpoint URL` unchanged (`--otlp-endpoint` still implies telemetry on).

When `telemetry_enabled`, after each tick (after today’s CPU ns sample):

| Field | Meaning |
|---|---|
| `telemetry_cpu_percent` | `Option<u32>`, **0..=100**. `(Δuser_ns + Δsystem_ns) * 100 / wall_ns` of **this tick**, capped at 100. |

- First telemetry tick (no previous ns) or `wall_ns == 0` ⇒ leave **None** (do not store 0 as “measured idle” unless a later tick computes 0).
- Subsequent ticks: Δ vs the last stored `telemetry_cpu_user_ns` / `telemetry_cpu_system_ns` (already cumulative). Then overwrite last ns as today.
- Overlay **off** ⇒ leave None, do not compute. `--load` / ckpt decode **zeros** (same as RSS / CPU ns). Not hashed, not in BoardBlob.
- Non-Linux: CPU ns stay None ⇒ percent stays None.

OTLP/JSON extra gauge (sim-cli), in addition to today’s wall/RSS/cpu ns/disk:

| name | value |
|---|---|
| `agentplace.process.cpu_percent` | last or 0 |

Still OTLP/**JSON** HTTP POST. No protobuf. No gRPC. No TLS. `cargo test` never sets a public collector. Loopback test asserts the new name in the POST body. Overlay off / no flag ⇒ **same hashes**.

Do **not** add out-dir disk walk, OTLP protobuf/gRPC, or a telemetry `SimEventKind`.

### C. Extra recipes (PG-9)

Object TOML only. No new Craft verb. No new Rust `ItemId` arms. Reuse existing glbs. Mock still does not pick Craft. Create on **implement** (distinct inputs vs shipped crafts):

| id | inputs | visual |
|---|---|---|
| `stool` | wood×8 | reuse `wood.glb` |
| `sash` | fiber×8 | reuse `low_poly_cloth.glb` |
| `brick` | stone×6 | reuse `stones_and_grass.glb` |
| `biscuit` | food×5 | reuse `plants_ready.glb` |

No `uses` / `station` / tool bonuses on these four. Catalog-off / empty catalog / no objects dir ⇒ Craft illegal; no-objects hash **`70e5204d…`**. Default `sim-cli` loads shipped objects ⇒ catalog-on idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel.

## Out of scope (later)

| Later | What |
|---|---|
| After M60 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; interiors / non-square footprints; OTLP protobuf/gRPC; ammo / projectile FX; time-decay wear; every-recipe stations; out-dir disk walk; household-home / invention / downed meshes |
| Not M60 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; replacing JSONL; transferring wear on Store; researcher `/give` copying wear; CPU percent uncapped / millipoints |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new ControlVerb. No postcard `PrimaryAction` append.
2. **format_version still writes 3 / reads v2+v3.** `tool_wear` already on the blob. Percent is derived / host; do not store it in ckpt.
3. Transfer moves the **freshest** slots (vec end) — the same ones M59 dropped. Store still drops them.
4. CPU percent is 0..=100 from Δ CPU ns / this tick’s `wall_ns`. First sample None. Hash-neutral.
5. Shipped-objects idle hashes **change** (new files). No-objects `--no-time` stays `70e5204d…`. Transfer wear and telemetry do not move idle hashes by themselves.
6. Do not change shipping `coop.toml`. No TLS. `cargo test` never needs the network.

## Tests (M60 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| `--no-time` shipped objects 2 ticks | hash `171a26d2…` |
| default CLI 2 ticks (time on) | hash `aacd867d…` |
| Transfer wear | giver axe wear `[3,0]` + Transfer 1 ⇒ giver keeps `[3]`; receiver wear `[0]` |
| Transfer most-worn stays | giver `[7]` Transfer 1 ⇒ receiver `[7]`; giver qty 0 / no wear |
| Store | still drops freshest; crate has no `tool_wear` |
| `/give` mint | receiver wear unchanged (fresh) |
| telemetry off | same hash; `telemetry_cpu_percent` is None |
| telemetry on | same hash as off; Linux after 2 ticks: percent `Some(0..=100)` |
| OTLP JSON | POST includes `agentplace.process.cpu_percent` |
| Craft stool / sash / brick / biscuit | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window not required. Live collector **not run**.

## PR Plan

### PR 1: Transfer wear

- Split end slots on Transfer; append to receiver; restore leftover on partial add; Store identity; tests

### PR 2: CPU percent

- Derive percent from Δ CPU ns / tick `wall_ns`; store `telemetry_cpu_percent`; OTLP gauge; telemetry tests

### PR 3: Recipes

- stool/sash/brick/biscuit TOML; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump. No new CLI flag (`--telemetry` already exists).

On implement: Transfer wear move; CPU percent; create stool/sash/brick/biscuit TOML. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-core --test objects
cargo test -p sim-core --test telemetry
cargo test -p sim-cli --test otlp_cli
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M60-test-plan.md`](M60-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `171a26d2…`; default (time on) `aacd867d…`; Hello v5; format_version 3 write.

## Risks

- **Partial Transfer.** `added < moved` must restore leftover items **and** leftover wear to the giver. Do not clamp-drop then lose slots.
- **Store vs Transfer.** Only Transfer moves wear. Store still truncates end.
- **`/give` is mint.** Do not copy wear onto researcher-given items.
- **CPU percent first tick.** None until a previous ns sample exists. `wall_ns == 0` must not divide.
- **Catalog files** change shipped-objects hashes even without Transfer/Craft.
- **Recipe inputs** stay distinct (wood×8, fiber×8, stone×6, food×5).
- **No protocol / format bump.** `tool_wear` already on the blob.
