# M21 — Pack fill scale, sim-cli --connect, jump-to-tick on the wire

**Status:** implemented  
**Depends on:** M20 complete (`docs/M20-plan.md`, git tag `M20`, commit `bc78e2d`)  
**Walkthrough:** [`M21-test-plan.md`](M21-test-plan.md)  
**Specs:** `M19-plan.md` (crate scale-by-fill; satchel/backpack do not scale), `M7-plan.md` (`sim-cli --connect` log tail deferred), `M18-plan.md` (in-process `/scrub`; remote refuses)

## Context

M19 crate meshes scale by fill; worn Basket/Backpack meshes do not. Headless attach is tests + viewer `--connect`; `sim-cli --connect` never shipped. M18 `/scrub` is in-process only.

M21 **does** append `ControlVerb::Scrub` and bump **`PROTOCOL_VERSION` to 3**. It does not add TLS, wire Give, or change `format_version`.

## Goal

A researcher can:

1. See worn **Basket** and **Backpack** meshes grow with pack fill (same 0.40..1.00 lerp as crates). Viewer-only, not hashed.
2. Run `sim-cli --connect tcp://…` (or `ws://`) and get Welcome/Snapshot then per-tick hash lines, without the viewer.
3. From a `--connect` viewer (or any control client) with `--allow-control`, `/scrub TICK` jumps the **server** sim: forward-tick if `want ≥ sim.tick`, else load server ckpt at-or-before and tick forward (M18 semantics).
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 3`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged.

## In scope

### A. Pack mesh scale-by-fill (viewer)

Reuse the crate formula with that agent’s `worn_pack_caps` (M20 stacked):

```
fill = max(pack_slots / slot_cap, pack_weight_milli / weight_cap_milli).clamp(0, 1)
scale = 0.40 + 0.60 * fill
```

- Empty pack still shows the worn mesh at **0.40** (do not despawn — they still wear the bag). Full → 1.00 × current Basket/Backpack cuboid.
- If both meshes show, **both use the same fill** (one `Agent.pack` map).
- Sync **updates Transform scale** of existing satchel/backpack entities (today spawn/despawn only). Fog still hides with the agent.
- Not hashed. No `sim-core` hash change. Do not add keys to `default.toml`.
- Slot/weight cap 0 (M20 `max_worn_* = 0`): no worn mesh; do not divide by zero.

### B. `sim-cli --connect` log tail

```bash
sim-cli --connect tcp://127.0.0.1:9000
sim-cli --connect ws://127.0.0.1:9001
# optional: --token SECRET
```

- Mutually exclusive with `--listen` (load error if both).
- Hello with current `PROTOCOL_VERSION`. Print `Welcome` (`tick`, `hash`, `experiment_id`), then Snapshot received, then each `Tick` as `tick=N hash=…` on stdout. `--quiet` prints only those lines.
- Subscribe `want_events=false`, `want_decisions=false`.
- **Read-only.** Does not send `Control`. Disconnect/EOF exits 0; protocol/auth errors exit non-zero.
- Loopback test in `sim-cli --test net` (spawn `--listen`, then `--connect`). Hash-neutral on the server.

### C. Jump-to-tick on the wire

Postcard **append** on `ControlVerb` (after `Summarize`):

```
Scrub(u64)   # want tick
```

**`PROTOCOL_VERSION = 3`** in the same PR as the new variant. Hello mismatch is still a `Protocol` error. Old v2 clients cannot talk to a v3 server.

**`--allow-control` required** (same as Pause). Without flag → `ControlDisabled`. Token still applies.

Server apply (host owns the sim):

| Case | Action |
|---|---|
| `sim.tick == want` | no-op (idempotent) |
| `sim.tick < want` | `tick()` until `want` or `tick()` false (`max_ticks` / empty) |
| `sim.tick > want` | `ckpt_at_or_before(server checkpoint dir, want)`, load, then tick forward to `want`. Missing dir/file → `Error` |

- Not interpolation. Mock ⇒ deterministic.
- `/ckpt next|prev` and `/events` stay **in-process only** (no new verbs).
- After a successful Scrub that changed state (including a reload), **broadcast `Snapshot`** to subscribers so `--connect` viewers resync, plus `ReportReady` to the caller (`scrubbed tick T`).
- Viewer `--connect` `/scrub TICK` **sends** `Control(Scrub(T))` instead of refusing. In-process `/scrub` unchanged (local `CkptScrubber`).
- Wire **Give** still refused. No `ControlVerb::Give`.
- Checkpoint dir = `--out-dir` else config `checkpoint.directory` (same as Save).

## Out of scope (later)

| Later | What |
|---|---|
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| After M22 | protobuf/TLS; extra LLM reflection/embeddings; Unix sockets |
| Not M21 | Browser; combat; CI Win/mac; Unix sockets |

## Key decisions

1. Pack fill scale is **viewer-only** (same lerp as crates). Empty worn pack stays visible at 0.40.
2. `sim-cli --connect` is a **read-only** log tail. `--listen` + `--connect` is a load error.
3. `ControlVerb::Scrub(u64)` is **append-only**. **`PROTOCOL_VERSION = 3`** in the same PR.
4. Remote scrub: forward-tick if `want ≥ sim.tick`; else load server ckpt then tick forward. Idempotent when `want == sim.tick`.
5. `/ckpt` and `/events` stay in-process. No `ControlVerb::Give`.
6. Do not change shipping `configs/default.toml` / `coop.toml`.
7. Mock CI. `format_version = 2`.

## Tests (M21 acceptance bar)

| Test | Asserts |
|---|---|
| pack empty vs full mesh scale | empty **<** full; empty still ≥ 0.40; crate helper unchanged |
| two worn meshes, half-full pack | both scales equal, between 0.40 and 1.00 |
| `sim-cli --connect` loopback | Welcome + Tick lines; server `final_hash` matches no-attach run |
| `--listen` and `--connect` together | load/usage error |
| Hello v2 vs v3 server | Protocol error |
| remote `/scrub` without `--allow-control` | ControlDisabled |
| live `sim.tick=2`, Scrub(4) | tick 4; hash matches ticking two more |
| live tick 4, ckpts at 2, Scrub(2) | tick 2 (reload); hash matches the ckpt |
| Scrub(same tick) twice | same hash (no extra events) |
| remote `/give` `/set` | still refused |
| default.toml mock | hashes match pre-M21 (`70e5204d…` at 2 ticks) |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Pack fill scale

- **Files:** viewer satchel/backpack `Transform` scale + helper next to `crate_fill_scale`, viewer tests

### PR 2: `sim-cli --connect`

- **Files:** `sim-cli` client loop (shared `Connection`), help text, `sim-cli --test net` connect log-tail

### PR 3: Wire Scrub + protocol 3

- **Files:** `shared` `ControlVerb::Scrub` append + `PROTOCOL_VERSION = 3`, `sim-cli` server apply + Snapshot broadcast, viewer remote `/scrub`, net tests (control on/off, forward, reload)

## Config / CLI

No overlay keys. No shipping TOML change. No new `ExperimentConfig` postcard fields.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control
cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000
# viewer --connect: /scrub 50  (server must have --allow-control)
```

## Verification

Walkthrough: [`M21-test-plan.md`](M21-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: pack empty scale < full; `--connect` prints Welcome/Tick and is hash-neutral; remote Scrub with `--allow-control` lands on the want tick; Hello v2 fails; default mock hashes match.

## Risks

- **`PROTOCOL_VERSION` must bump with `ControlVerb::Scrub`.** Same-slice viewer + dummy + `--connect`. Do not leave a mixed tree.
- **Backward scrub needs server ckpts.** `want < sim.tick` with no file → Error (forward-only without `--out-dir`).
- **Double-apply.** `want == sim.tick` no-op.
- **Pack cap 0.** No worn mesh; do not divide by zero.
- **Do not add `ControlVerb::Give`.** Do not change `format_version`.
