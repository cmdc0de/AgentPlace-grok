# M24 — Wire /set, connect /inject, lockstep ack

**Status:** implemented  
**Depends on:** M23 complete (`docs/M23-plan.md`, git tag `M23`, commit `a90535c`)  
**Walkthrough:** [`M24-test-plan.md`](M24-test-plan.md)  
**Specs:** `M16-plan.md` / `M19-plan.md` (in-process `/set` + `/set respect`; remote refuses), `M8-plan.md` (`InjectIncentive` on the wire), `M23-plan.md` (every-tick Snapshots; no lockstep ack; `--connect` inject refused)

## Context

M16/M19 `/set` is in-process only. Viewer `--connect` already sends `InjectIncentive`; `sim-cli --connect` still errors `inject is listen-side`. M23 shows every Snapshot in order but the server does not wait for paint.

M24 **does** append `ControlVerb::Set` and `ClientMessage::AckTick` and bump **`PROTOCOL_VERSION` to 5**. It does not add TLS, protobuf, or change `format_version`.

## Goal

A researcher can:

1. From a `--connect` viewer (or `sim-cli --connect --allow-control`) type `/set ID hunger|thirst|energy|influence N` or `/set ID respect TOWARD N` and mutate the **server** sim (same millipoint clamp as in-process). Hash-sensitive. `--allow-control` required.
2. From `sim-cli --connect --allow-control`, `/inject PATH` reads that TOML and sends existing `ClientMessage::InjectIncentive` (viewer already can). Server still requires `--allow-control`.
3. Start `sim-cli --listen --lockstep` so after tick *T* the hub **does not** run *T+1* until every subscribed client has `AckTick(T)`. Viewer acks after it applies that tick’s Snapshot. `sim-cli --connect` auto-acks each Tick so a log-tail does not stall the server.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Hello v4 vs v5 is Protocol. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. Wire `/set`

Postcard **append** on `ControlVerb` (after `Events`):

```
Set { id: u64, field: String, toward: Option<u64>, value: u32 }
```

`field` is the same token as `/set`: `hunger` | `thirst` | `energy` | `influence` | `respect`. `toward` is **required** when field is `respect` (else Error). Display 0–100 → millipoints `N * 100`, clamp 0..=10_000 via existing `set_display_field` / `set_respect`.

**`--allow-control` required.** Without flag → `ControlDisabled`.

- Hash-sensitive. Unknown field / missing agent / respect without toward / parse fail → `Error` (`Internal`).
- After success: **broadcast Snapshot** (paused runs would otherwise not Tick). `ReportReady` (`set agent ID field=N …`).
- Viewer `--connect` `/set` **sends** `Control(Set { … })` instead of refusing. In-process `/set` unchanged.
- `sim-cli --connect --allow-control` parses `/set` the same way (today it errors in-process only).

### B. `sim-cli --connect` `/inject`

**No new postcard variant.** `ClientMessage::InjectIncentive { schedule_toml }` already exists (v4+).

```
/inject PATH
```

- `--connect --allow-control` required to send. Server still `--allow-control` or `ControlDisabled`.
- Client **reads the file** and sends the TOML string (same as viewer remote `/inject`). Missing file → stderr, no send.
- `--connect` without `--allow-control` stays hash-neutral (does not send Inject).
- Listen-side `--inject` unchanged. Viewer remote inject unchanged.

`parse_control_line` today returns `ControlVerb` and rejects inject. Widen to `ClientMessage` (Control or InjectIncentive) or a small enum — implementer’s choice; stdin loop already sends `ClientMessage`.

### C. Lockstep ack

Postcard **append** on `ClientMessage` (after `InjectIncentive`):

```
AckTick(u64)
```

**Not a ControlVerb.** Paint sync is hash-neutral. **Does not** require `--allow-control`.

CLI on **listen**:

```
sim-cli --listen tcp://… --lockstep
sim-cli --listen tcp://… --allow-control --start-paused --lockstep
```

Overlay (not `ExperimentConfig`):

```toml
[network]
lockstep = true
```

When lockstep is on:

- After `tick_once()` (Tick already broadcast), the serve loop **waits** until every `subscribed` client has sent `AckTick(sim.tick)`.
- **Do not hold** the hub mutex while waiting (client threads must recv AckTick).
- **Zero subscribers** → do not wait (headless `--lockstep` still ticks).
- Paused → no wait (no tick).
- No timeout. A v5 viewer that never acks stalls the sim — that is the mode. CI mock never sets `--lockstep`.
- Overlay/CLI default **off**. Shipping TOML does not set it.

**Viewer `--connect`:** after `apply_remote` applies a Snapshot (one per frame), send `AckTick(state.sim.tick)`. Do not ack on Tick-only (world not painted yet).

**`sim-cli --connect`:** on each `ServerMessage::Tick { tick, … }`, send `AckTick(tick)` so a hash tail does not deadlock `--lockstep`.

Hello **v4 vs v5** is Protocol. Old v4 clients cannot talk to a lockstep v5 server.

## Out of scope (later)

| Later | What |
|---|---|
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| After M27 | protobuf/TLS; embeddings; Unix sockets |
| Not M24 | Browser; combat; CI Win/mac; hashed pipeline events |

## Key decisions

1. **`PROTOCOL_VERSION = 5`** in the same tree as `ControlVerb::Set` **and** `ClientMessage::AckTick`. Old v4 Hello is a Protocol error. Do not leave a mixed Hello.
2. Set field on the wire is a **string** plus optional `toward`. Same millipoint clamp as in-process. Hash-sensitive. Snapshot after success.
3. `/inject` on `--connect` reuses `InjectIncentive`. Client reads the path. No new verb.
4. `AckTick` is **not** Control. No `--allow-control`. Viewer acks after Snapshot apply, not on Tick.
5. `--lockstep` / `[network] lockstep` default **off**. Zero subscribers ⇒ no wait. Do not hold the hub lock while waiting.
6. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M24 acceptance bar)

| Test | Asserts |
|---|---|
| Set without `--allow-control` | ControlDisabled |
| Set `hunger` 50 with flag | millipoints 5000; hash changes; Snapshot |
| Set `respect` toward 1 value 40 | edge written; hash changes |
| Set respect without toward / unknown field / bad id | Error |
| Hello v4 vs v5 server | Protocol error |
| `PROTOCOL_VERSION` | **5** |
| `--connect` `/set` parses to `Control(Set)` | |
| `--connect` without `--allow-control` | still hash-neutral; no Inject/Set |
| `--connect --allow-control` `/inject` file | server schedule applied; hash may change |
| `/inject` missing file | no send; stderr |
| remote viewer `/set` | sends Set, does not refuse |
| `--lockstep`, 0 subscribers | ticks to `--ticks` (no hang) |
| `--lockstep` + dummy: Tick then no Ack | serve loop does not finish next tick until AckTick(T) |
| `--lockstep` + AckTick(T) | T+1 Tick follows |
| viewer helper: Snapshot apply ⇒ AckTick(world tick) | |
| `--connect` auto-acks Tick | lockstep server can finish with only `--connect` |
| default.toml mock | hashes match pre-M24 (`70e5204d…` at 2 ticks) |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: PROTOCOL 5 + wire Set + AckTick variants

- **Files:** `shared` `PROTOCOL_VERSION = 5`, `ControlVerb::Set`, `ClientMessage::AckTick`, server `set_display_field`/`set_respect` + Snapshot, viewer remote `/set`, sim-cli parse `/set`, Hello v4 tests

Do **not** leave a mixed Hello: AckTick discriminant ships even if lockstep wait is PR 3.

### PR 2: sim-cli `--connect` `/inject`

- **Files:** `client.rs` `/inject PATH` → `InjectIncentive`, net loopback with `--allow-control`

### PR 3: `--lockstep` wait + acks

- **Files:** serve loop wait-without-hub-lock, `--lockstep` / `[network] lockstep`, viewer Ack after Snapshot, `--connect` auto-AckTick, net tests

## Config / CLI

No shipping TOML change. Overlay `[network] lockstep` is not `ExperimentConfig`. No new `ExperimentConfig` postcard fields.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused --lockstep \
  --llm mock --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# /play
# /set 0 hunger 50
# /set 0 respect 1 40

# other terminal
cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --allow-control
# /inject configs/incentives/coop.toml
# /set 0 thirst 20
```

## Verification

Walkthrough: [`M24-test-plan.md`](M24-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: remote `/set` with `--allow-control` changes hash; `/inject` from `--connect` applies the schedule; `--lockstep` waits for `AckTick`; Hello v4 fails; default mock hashes match.

## Risks

- **`PROTOCOL_VERSION` must bump to 5** with `Set` and `AckTick` in the same tree. Same-slice viewer + dummy + `--connect` Hello. Do not leave a mixed tree.
- **Lockstep must not hold the hub mutex** while waiting, or AckTick cannot be received (deadlock).
- **Zero subscribers** must not wait, or headless `--lockstep` hangs.
- **Viewer acks after Snapshot**, not on Tick (world not painted yet).
- **`--connect` auto-acks** so a hash tail does not stall `--lockstep`.
- **Set is hash-sensitive.** Snapshot after success. `/set` stays millipoints ×100.
