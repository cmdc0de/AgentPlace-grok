# M22 — Wire Give, remote /ckpt and /events

**Status:** implemented  
**Depends on:** M21 complete (`docs/M21-plan.md`, git tag `M21`, commit `3389e47`)  
**Walkthrough:** [`M22-test-plan.md`](M22-test-plan.md)  
**Specs:** `M10-plan.md` (in-process `/give`), `M18-plan.md` (`/ckpt` file-to-file), `M17-plan.md` (`/events` JSONL display-only), `M21-plan.md` (remote `/scrub`; `/ckpt` `/events` still refuse)

## Context

M10 `/give` is in-process only. M21 put `/scrub` on the wire (`PROTOCOL_VERSION = 3`); `/ckpt next|prev` and `/events` still refuse remotely.

M22 **does** append `ControlVerb::{Give, CkptNext, CkptPrev, Events}` and bump **`PROTOCOL_VERSION` to 4**. It does not add TLS, wire `/set`, or change `format_version`.

## Goal

A researcher can:

1. From a `--connect` viewer with `--allow-control`, `/give ID ITEM QTY` mutates the **server** sim (pockets, no energy, `SimEventKind::Give`, hash-sensitive) — same as in-process.
2. `/ckpt next` and `/ckpt prev` load the **adjacent `.ckpt` file** on the server checkpoint dir (file-to-file, not catch-up). `/scrub` unchanged.
3. `/events TICK` returns that tick’s JSONL lines from the server `{id}_events.jsonl` (**display-only**; does not replace the sim).
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 4`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged.
5. Attached viewer **Status** shows the live server tick/hash from each `Tick` (MF-1). 3D/world still snapshot-throttled (~200 ms).

## In scope

### A. Wire Give

Postcard **append** on `ControlVerb` (after `Scrub`):

```
Give { id: u64, item: String, qty: u32 }
```

`item` is the same token as `/give` (`basket`, `backpack`, `berry_bush`, `wood`, …). Shared has no `ItemId`; server calls `parse_item` on `sim.config.world.species`. Unknown item / qty 0 / missing agent → `Error` (`Internal`). Partial add returns how many fit (same as `give_item` today).

**`--allow-control` required.** Without flag → `ControlDisabled`.

- Pockets only (not pack). No energy cost. Hash-sensitive. Pushes `Give` event.
- After a successful Give, **broadcast `Snapshot`** to subscribed clients (paused runs would otherwise not Tick). `ReportReady` to the caller (`gave N item to agent ID`).
- Viewer `--connect` `/give` **sends** `Control(Give { … })` instead of refusing. In-process `/give` unchanged.
- `sim-cli --connect` stays **read-only** (does not send Control).

### B. Remote `/ckpt next|prev`

Append (after `Give`):

```
CkptNext
CkptPrev
```

Same file-to-file contract as in-process M18: `list_checkpoints(server checkpoint dir)`, next file with tick **>** `sim.tick` / previous **<** `sim.tick`. Load that `.ckpt` (no tick-forward). Missing dir/file → `Error`.

- Checkpoint dir = `--out-dir` else config `checkpoint.directory` (same as Save / Scrub).
- After a successful load: **pause**, broadcast `Snapshot` to subscribers, `ReportReady` (`loaded tick T`).
- Viewer `--connect` `/ckpt next|prev` sends the verb. In-process unchanged. `/scrub` still catch-up (M21).

### C. Remote `/events TICK`

Append (after `CkptPrev`):

```
Events(u64)
```

**Display-only.** `find_events_jsonl` in the checkpoint dir; `jsonl_lines_for_tick` (or at-or-before, same as in-process `filter_events`). No JSONL / no lines → `Error`. Does **not** load a ckpt or change `sim`.

Reply: `ReportReady` whose text is the filtered JSONL lines (joined by `\n`). Viewer scrollback shows them. No Snapshot.

### D. Attach HUD live tick (MF-1)

`--connect` Status line uses a live clock updated on every `ServerMessage::Tick` in the IO thread (not dropped when the Bevy channel is full). HUD tick + short hash come from that clock. 3D/world still refresh from Snapshot (~200 ms). In-process viewer unchanged (`state.sim.tick`).

## Out of scope (later)

| Later | What |
|---|---|
| **M23** | [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| After M27 | protobuf/TLS; embeddings; Unix sockets; lockstep ack; wire `/set` |
| Not M22 | Browser; combat; CI Win/mac; **wire `/set`** |

## Key decisions

1. **`PROTOCOL_VERSION = 4`** in the same PR as the new verbs. Old v3 Hello is a Protocol error.
2. Give item on the wire is a **string**, not `ItemId`. Unknown token → Error.
3. Give is pockets, no energy, hash-sensitive. Snapshot after success.
4. `/ckpt` is **file-to-file**, not M21 catch-up. `/scrub` unchanged.
5. `/events` is **display-only**. Missing JSONL → Error; hash unchanged.
6. `/set` stays in-process. `sim-cli --connect` stays read-only.
7. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M22 acceptance bar)

| Test | Asserts |
|---|---|
| Give without `--allow-control` | ControlDisabled |
| Give `berry_bush` 1 with flag | inventory +1; hash changes; Snapshot |
| Give unknown item / qty 0 | Error |
| Hello v3 vs v4 server | Protocol error |
| CkptNext/Prev, no files | Error |
| Save/Step to ckpts at 2 and 4, CkptPrev from 4 | tick 2; hash matches file |
| Events, no JSONL | Error |
| Events with JSONL at tick T | ReportReady contains those lines; sim tick/hash unchanged |
| remote `/set` | still refused |
| HUD status tick when attached | live Tick clock, not last Snapshot |
| default.toml mock | hashes match pre-M22 (`70e5204d…` at 2 ticks) |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Wire Give + protocol 4

- **Files:** `shared` `ControlVerb::Give` + `PROTOCOL_VERSION = 4`, server `give_item` + Snapshot, viewer remote `/give`, net tests

### PR 2: Remote `/ckpt next|prev`

- **Files:** `CkptNext` / `CkptPrev` append, server `list_checkpoints` load, viewer remote, net tests

### PR 3: Remote `/events`

- **Files:** `Events(u64)` append, server JSONL filter → `ReportReady`, viewer remote, net tests

All three verbs ship in the **same** protocol-4 tree (do not leave a mixed Hello).

## Config / CLI

No overlay keys. No shipping TOML change. No new `ExperimentConfig` postcard fields.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m22 --checkpoint-every 2 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused --llm mock --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# /give 0 berry_bush 1
# /ckpt prev   /ckpt next
# /events 2
```

## Verification

Walkthrough: [`M22-test-plan.md`](M22-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: remote Give with `--allow-control` changes hash; CkptPrev lands on the previous file; `/events` is display-only; Hello v3 fails; default mock hashes match.

## Risks

- **`PROTOCOL_VERSION` must bump to 4 with the new verbs.** Same-slice viewer + dummy + `--connect` Hello. Do not leave a mixed tree.
- **Item on the wire is a string**, not `ItemId`. Unknown token is Error, not Wait.
- **`/ckpt` is file-to-file**, not M21 catch-up. Empty checkpoint dir → Error.
- **`/events` is display-only.** Missing JSONL → Error; does not mutate hash.
- **Give still pockets**, not pack. No `ControlVerb` for `/set`.
