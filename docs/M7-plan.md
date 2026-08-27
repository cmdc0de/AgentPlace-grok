# M7 — Attachable TCP and WebSocket clients

**Status:** implemented (git tag `M7`, commit `789fc81`).  
**Depends on:** M6 complete (`docs/M6-plan.md`, git tag `M6`, commit `46c75e0`)  
**Specs:** `simulation-architecture-spec.md` §4–5 (attachable clients, TCP **and** WebSocket), `deterministic-seeding-design.md` (hash-identical continuation), `medium-priority-specs.md` §4 (client disconnect: core continues), `00-INDEX-AND-HANDOFF.md` (in-process / TCP / WS)

## Context

The project’s experimental loop is: (1) run a baseline and watch governance emerge; (2) load a checkpoint, inject a different incentive schedule, measure what changed. M1–M6 delivered the in-process half of (1): a deterministic core, a public board, relationships, reports, and an imgui viewer that owns the `Simulation` in the same process.

`crates/shared` still only reserves `transport`. The viewer cannot attach to a headless `sim-cli`. Incentives are still an empty checkpoint slot (`IncentiveState { entries: Vec<String> }`).

The remaining gap is **two** slices. **M7 is attachable clients** (this document). **M8 is incentive inject** — the actual A/B intervention. Mixing them would make “a subscriber did not change `state_hash`” hard to prove.

M7 does **not** implement incentive effects, TLS, or timeline scrubbing.

## Goal

A researcher can:

1. Run **headless** `sim-cli --listen tcp://…` (and/or `ws://…`) for a long experiment.
2. Attach the **same imgui viewer** with `--connect tcp://…` (in-process remains the default).
3. Optionally send **control** verbs already in the M6 console (pause / play / step / save / report / summarize) if the server was started with `--allow-control`.
4. Prove that a **read-only** subscriber leaves `final_hash` identical to a run with no listener. **This is the M7 gate.**

## In scope

| Area | M7 meaning |
|---|---|
| **Protocol (`shared`)** | Versioned, length-prefixed **postcard** frames (`u32` LE length + payload). `PROTOCOL_VERSION = 1`. |
| **Transport trait** | `listen` / `connect` / `send` / `recv` / `close`. **TCP** and **WebSocket**, same codec. URLs: `tcp://host:port`, `ws://host:port`. No `wss` / TLS. |
| **I/O model** | **Blocking sockets + threads**, not tokio. `std::net` for TCP; sync `tungstenite` for WS. Viewer: background reader thread → channel of `Tick`/`Snapshot` (Bevy is sync). |
| **Server** | `sim-cli --listen tcp://127.0.0.1:9000` (repeatable; also `--listen ws://127.0.0.1:9001`). Default: no listen. `--allow-control`. `--token SECRET` checked on Hello. `sim-cli` currently has no `shared` dep — that is the wiring. |
| **Viewer client** | Default in-process. `--connect tcp://127.0.0.1:9000` (or `ws://`) instead of owning a `Simulation`. Same imgui. Third `ViewerSource` beside `--config` / `--load`: plugin starts from the first `Snapshot`; in-process `tick_sim` is skipped. |
| **Hello** | Client `{protocol_version, token?}`. Server `Welcome { tick, state_hash, experiment_id }` or `Error`. |
| **Snapshot** | On connect and on `RequestSnapshot`: AGTN checkpoint bytes from `encode_checkpoint`. Client may `Simulation::from_checkpoint` for display. |
| **Live stream** | After `Subscribe`: each tick `{tick, state_hash, events, decisions}`. Full world via snapshot-on-demand, not a custom delta compressor. 64×64 is fine. |
| **Control (opt-in)** | Pause, Play, Step(n), Save, Report, Summarize. Same closed set as M6. **No** inject / set / give. Error unless `--allow-control`. |
| **Read-only default** | Hello + Subscribe only → bit-identical to no listener. **This is the M7 gate.** |
| **Multiple readers** | N simultaneous read-only subscribers. Disconnect never stops the core. Control is last-write-wins if `--allow-control`. |
| **sim-core** | No sockets, no HTTP, no imgui. Server in `sim-cli` (split `sim-net` only if `sim-cli` becomes unreadable). Types in `shared`. |
| **Reserved wire variant** | `InjectIncentive` exists on the enum, **always returns Error** in M7, so M8 does not rewrite the codec. |
| **Checkpoint** | `format_version` stays **2**. Network protocol ≠ checkpoint format. |

### Wire messages (closed)

```
Client → Server:
  Hello { protocol_version, token? }
  Subscribe { want_events, want_decisions }
  RequestSnapshot
  Control { Pause | Play | Step(n) | Save | Report | Summarize }
  InjectIncentive { .. }   # reserved; server Error

Server → Client:
  Welcome { tick, state_hash, experiment_id }
  Snapshot { checkpoint_bytes }
  Tick { tick, state_hash, events, decisions }
  ReportReady { markdown_or_path }
  Error { code, message }
```

## Out of scope (later)

| Later | What |
|---|---|
| **M8** | Done — [`M8-plan.md`](M8-plan.md) (incentive TOML, `/inject`, processing metrics, death) |
| **M9** | Done — [`M9-plan.md`](M9-plan.md) — LLM prompts, replay, `--compare` |
| **M10** | Done — [`M10-plan.md`](M10-plan.md) — LLM parse/replay; Transfer/Store; in-process `/give` (not on the wire) |
| **M11** | Done — [`M11-plan.md`](M11-plan.md) — mock fills crates; Basket backpack |
| **M12** | Done — [`M12-plan.md`](M12-plan.md) — public vs hidden incentives |
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
| **M14** | [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| Later | TLS / `wss`, Unix sockets, multiple exclusive writers, jump-to-tick without a file, delta compression, browser client, `/set`, wire Give |

Do **not** implement payoff/influence stubs that would change hashes.

### Not in M7 even though the spec names them

- Timeline scrubber / “jump to tick N” (needs a recording or stored checkpoints).
- Incentive schedule editor in imgui (M8).
- A full headless “log-only client” binary — the **dummy TCP subscriber in tests** is that client for CI. Viewer `--connect` is the researcher path. A `sim-cli --connect` log tail can wait.

## Key decisions

1. **Transport, not incentives.** The A/B experiment is M8.
2. **Attaching is hash-neutral** when control is off. Tests must prove it. Connection logs stay **outside** `sim-core` (stderr / a net log file) — a `SimEvent::ClientConnected` would change the hash.
3. **One codec, two sockets.** TCP and WebSocket are both required (spec). Feature flags may slim a binary later, not in M7.
4. **Postcard + u32 LE length prefix.** Matches checkpoints; JSONL files stay files.
5. **In-process viewer stays default.** `--connect` is opt-in; CI does not need a GPU or a server.
6. **Control is opt-in** (`--allow-control`).
7. **Blocking I/O + threads.** No tokio in sim-core **or** as a workspace-wide runtime. Async is not required for postcard frames on LAN.
8. **No TLS.** Token on Hello is LAN auth, not encryption. Document `wss` as later.
9. **Snapshot on demand, stream ticks.** Full checkpoint every tick is too big; events+hash every tick is enough; client requests a snapshot when it joins or desyncs. `format_version` stays 2 — network protocol ≠ checkpoint format.
10. **Dummy subscriber in tests** — no GUI in CI.
11. **Client disconnect:** core continues; re-attach via Hello + Snapshot (`medium-priority-specs.md` §4).

## Config / CLI

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 100000 \
  --listen tcp://127.0.0.1:9000 --listen ws://127.0.0.1:9001 --allow-control

cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

```toml
[network]
tcp_listen = ""
ws_listen = ""
token = ""
allow_control = false
```

## Tests (M7 acceptance bar)

| Test | Asserts |
|---|---|
| Codec round-trip | Hello / Welcome / Snapshot / Tick / Error |
| Unknown `protocol_version` | Error, no panic |
| TCP loopback | Hello + Snapshot |
| WebSocket loopback | same messages |
| Hash-neutral attach | 40-tick mock with dummy subscriber = 40-tick without |
| Control without flag | Error |
| `InjectIncentive` | Error (“not implemented”) |
| Disconnect | server keeps ticking; second client can Hello |
| `cargo test -p sim-core` | still green; no sockets in core |

## PR Plan

### PR 1: Protocol + codec

- **Files:** `crates/shared/src/lib.rs` (protocol, framing)
- **Changes:** message enum, length-prefix encode/decode, unit tests. Add workspace `postcard` dep on `shared`.

### PR 2: TCP transport

- **Files:** `shared::transport::tcp`
- **Changes:** listen/connect; loopback test.

### PR 3: WebSocket transport

- **Files:** `shared::transport::ws`
- **Changes:** same trait; sync `tungstenite`; loopback test.

### PR 4: sim-cli server

- **Files:** `sim-cli`
- **Changes:** depend on `shared`; `--listen`, `--allow-control`, `--token`; Subscribe stream; hash-neutral integration test.

### PR 5: viewer `--connect`

- **Files:** `viewer`
- **Changes:** attach instead of `Simulation::new`; in-process still default; background reader + channel.

### PR 6: Control verbs + docs

- **Files:** protocol Control, tests, README
- **Changes:** pause/step/save/report when allowed; README listen/connect examples.

## Files / reuse

**Reuse:** `encode_checkpoint` / `from_checkpoint`, event log, `last_tick_decisions`, M6 command verbs (control subset), `PROTOCOL_VERSION` placeholder.

**Do not touch in M7:** incentive effect types, imgui internals, `sim-core` HTTP, vote weights.

## Verification (when M7 is implemented)

```bash
cargo test -p shared
cargo test -p sim-core

# terminal A
cargo run -p sim-cli -- --config configs/default.toml --ticks 100000 \
  --listen tcp://127.0.0.1:9000 --quiet

# terminal B
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Two 80-tick mock runs (with vs without a dummy TCP subscriber) print the same `final_hash`.

## Risks

- **Async vs Bevy.** Viewer is sync; a blocking recv on the render thread will hitch. Use a background reader + channel of `Tick`/`Snapshot`.
- **Snapshot size.** Shipping a full checkpoint every connect is OK; shipping one every tick is not. Stream events+hash; snapshot on demand.
- **Hash-neutrality.** Logging “client connected” into `SimEvent` would change the hash. Connection logs stay **outside** `sim-core` (stderr / a net log file).
- **WebSocket deps.** Keep `tungstenite` in `shared` (or `sim-cli`/`viewer` features if the workspace build gets heavy) — never in `sim-core`.
- **Token on the wire is not TLS.** Good enough for LAN; document that.
- **Connect-mode viewer.** The first `Snapshot` must be applied before 3D markers spawn; a late join should `RequestSnapshot` rather than invent a delta.
