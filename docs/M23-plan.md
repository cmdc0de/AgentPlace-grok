# M23 — See every tick, LLM pipeline barrier, sim-cli Control

**Status:** implemented  
**Depends on:** M22 complete (`docs/M22-plan.md`, git tag `M22`, commit `58879b0`)  
**Walkthrough:** [`M23-test-plan.md`](M23-test-plan.md)  
**Specs:** `missing-features.md` (MF-2, MF-3), `M8-plan.md` (perceive/retrieve/select/execute/remember timing), `medium-priority-specs.md` §4 (timeout = Wait), `M21-plan.md` (`sim-cli --connect` read-only)

## Context

M22 HUD shows the live attach tick (MF-1). 3D/world still Snapshot-throttled (~200 ms) and Tick frames can `try_send`-drop. `tick()` is sequential-blocking, but LLM timeout is `Wait` + `LlmWait` with no N/N completeness proof. `sim-cli --connect` is a read-only Welcome/Tick hash tail.

M23 **does not** bump `PROTOCOL_VERSION` (stays **4**). No new `ControlVerb` / `ClientMessage`. It does not add TLS, wire `/set`, or change `format_version`.

## Goal

A researcher can:

1. **See every sim tick** in the attached (and in-process) viewer world — agents, packs, crates, board, fog, inspector — not only the Status number. HUD shows live tick **and** displayed world tick when they differ.
2. **Verify** that every living agent finished perceive → retrieve → select → execute → remember before tick *T+1* starts. Opt-in **barrier** retries timeout/parse (default **3 extra attempts**) then that agent `Wait`s; remaining agents still finish **this** tick. Default off ⇒ mock hashes unchanged.
3. From `sim-cli --connect --allow-control`, type the same remote slash commands as the viewer (`/play`, `/pause`, `/step`, `/give`, `/ckpt`, `/events`, `/scrub`, `/save`, `/report`, `/summarize`) and send existing `ControlVerb`s. Without `--allow-control`, `--connect` stays the M21 hash-neutral log tail.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 4`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. MF-2 — see every tick (viewer)

**Attach (`--connect`):**

- Remove the **200 ms** Snapshot throttle. On every `ServerMessage::Tick`, `RequestSnapshot`.
- Apply Snapshots **in receive order**, **at most one decode per Bevy frame**. Never skip to the latest. Researcher sees *T* then *T+1* even if the HUD live clock is ahead.
- IO thread: **do not drop** Snapshot (`send`, not `try_send`). Tick may still be HUD-only if the Bevy queue is full (live clock already survives that). Enlarge the sync channel if 64 is tight.
- Status: `tick {world}` from `state.sim.tick`; when attached and live ≠ world, also show live (MF-1 clock) so catch-up is visible. Hash on the HUD stays the live hash (MF-1).
- No server lockstep ack. The sim does **not** wait for the viewer to paint. (Ack is [`M24-plan.md`](M24-plan.md).)
- Server already answers `RequestSnapshot`. No new wire messages. Encode-every-tick is accepted cost for researcher attach.

**In-process:**

- At most **one** `sim.tick()` per rendered frame (timer still default `0.2` s). Do not burst multiple ticks in one `Update` if a frame hitch makes the timer overdue.
- `/step` unchanged.

Hash-neutral. Viewer-only + attach request rate.

### B. MF-3 — pipeline barrier / verify

Shipped pipeline (M8, no Reflect/Plan): perceive → retrieve → select → execute → remember. `tick()` already loops `step_agent` sequentially and does not return until all living agents are done. `Chooser::Custom` **blocks** that agent. On `Err` (timeout / HTTP / parse): `LlmWait` + `Wait`, then the next agent — spec `llm_timeout_action = "Wait"`. Today `max_retries = 2` (hashed) retries malformed/unreachable only; **timeout returns immediately**; parse after HTTP 200 can skip remaining attempts.

**Verify (always, hash-neutral):**

- After the agent loop, `last_tick_timing.agents.len()` == living agent count. Time **remember** (today `remember_ns` is never written).
- Completeness is a derived check, **not** a new `SimEventKind` (events are hashed).
- Viewer inspector / Status: `pipeline N/N` from `TickTiming`. Timing JSONL already exists.
- Unit: Custom chooser that sleeps then `Ok`; `tick()` wall time covers the sleep; next tick’s hash/events cannot appear before it returns. Default mock: N/N every tick.

**Barrier (opt-in, default off) — bounded retries, not hang:**

Infinite wait-until-Ok is rejected (down Spark would stall forever). Barrier **retries timeout and parse**, then gives up. It does **not** skip unstepped agents.

Overlay **not** on `ExperimentConfig` (would change `config_hash`):

```toml
[llm]
barrier = true
barrier_retries = 3    # extra attempts after the first; omit ⇒ 3
```

- CLI: `--llm-barrier` (retries default **3**), `--llm-barrier-retries N` (0 = one attempt then Wait).
- When on: each attempt waits up to existing `timeout_ms` (hashed, unchanged). Retry on **Timeout, Malformed, Unreachable, and parse_choice_json failure**. Same `call_seed`; lower temperature only on malformed/parse (not on timeout).
- After `1 + barrier_retries` failures: that agent `LlmWait` + `Wait` (no speak), then **the next agent this tick**, then `tick()` returns. “Move on” = the tick can finish, not “skip the rest of the roster.”
- Default off: today’s spec (timeout = immediate Wait; `max_retries` malformed only). Do **not** change shipping `max_retries = 2`.
- Mock CI never sets barrier. Worst case with barrier on: `(1 + 3) × timeout_ms` per agent (default.toml `timeout_ms = 120000` ⇒ **8 min/agent** if the provider never answers). Bounded, not a hang.

Do **not** implement spec Reflect/Plan. Do **not** add hashed pipeline-complete events.

### C. `sim-cli --connect` sending Control

**No protocol bump.** Reuse existing `ControlVerb::{Pause, Play, Step, Save, Report, Summarize, Scrub, Give, CkptNext, CkptPrev, Events}`.

```
sim-cli --connect tcp://127.0.0.1:9000                  # M21 log tail (unchanged)
sim-cli --connect tcp://127.0.0.1:9000 --allow-control  # stdin slash commands
sim-cli --connect tcp://… --allow-control --token SECRET
```

- `--connect` without `--allow-control`: still read-only Subscribe + print Welcome/Snapshot/Tick hashes. Existing `connect_log_tail_hash_neutral` still passes.
- `--connect --allow-control`: same attach, plus stdin lines parsed like viewer remote commands. Send `ClientMessage::Control`. Print `ReportReady` / `Error` on stdout. Tick hash lines continue.
- Server still requires **its** `--allow-control` (else `ControlDisabled`). Client flag only enables sending.
- `/set` still refused (no verb). `/inject` stays listen-side / viewer (not a `ControlVerb`).
- `--listen` and `--connect` still mutually exclusive. `--start-paused` still requires `--listen`.
- `sim-cli` does not depend on `viewer`. Small command parser in `crates/sim-cli/src/client.rs` matching `remote_control` coverage.

## Out of scope (later)

| Later | What |
|---|---|
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done — [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | Done — [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| **M55** | Done — [`M55-plan.md`](M55-plan.md) — ranged DEX to-hit, hoe/net Farm+Fish bonuses, extra recipes |
| **M56** | Done — [`M56-plan.md`](M56-plan.md) — axe gather bonus, Draco glTF decode, extra recipes |
| **M57** | Done — [`M57-plan.md`](M57-plan.md) — hammer stone-gather bonus, process CPU/disk telemetry, extra recipes |
| **M58** | Done — [`M58-plan.md`](M58-plan.md) — tool durability + millstone station, viewer-frame OTLP, extra recipes |
| **M59** | Done — [`M59-plan.md`](M59-plan.md) — extra invention kinds, per-instance tool wear, extra recipes |
| **M60** | Done — [`M60-plan.md`](M60-plan.md) — transfer wear on Give, CPU percent, extra recipes |
| **M61** | Done — [`M61-plan.md`](M61-plan.md) — time-decay wear, out-dir disk walk, extra recipes |
| **M62** | [`M62-plan.md`](M62-plan.md) — stations for bread/stew, Store wear, extra recipes |
| After M62 | protobuf/TLS; Unix sockets; sql.js / ad-hoc SQL |
| Not M23 | Browser; combat; CI Win/mac; Reflect/Plan LLM stages; hashed pipeline events |

## Key decisions

1. **`PROTOCOL_VERSION` stays 4.** No new postcard variants. Hello v4 unchanged.
2. MF-2 plays every Snapshot **in order**, one per frame. No lockstep ack.
3. MF-3 completeness is **timing/HUD**, not a hashed event.
4. Barrier is **overlay + CLI**, not `LlmParams` (config_hash). Default **off**. `barrier_retries` default **3** (4 attempts). After exhaustion: that agent Wait, rest of this tick still runs.
5. Do **not** change shipping `max_retries = 2` / `timeout_ms`. Timeout retry is barrier-only (spec without barrier: no retry on pure timeout).
6. `--connect --allow-control` **sends** Control; server `--allow-control` **accepts** it. `/set` stays in-process.
7. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M23 acceptance bar)

| Test | Asserts |
|---|---|
| attach: two Ticks, no 200 ms sleep | two Snapshot requests; world `state.sim.tick` visits both values in order (not jump) |
| attach: full Bevy Tick channel | HUD live tick still advances (MF-1); Snapshots not dropped |
| HUD live ≠ world | status shows both |
| in-process overdue timer | at most one `tick()` per frame |
| mock tick timing | `agents.len()` == living count; `remember_ns` field written |
| sleeping Custom chooser | `tick()` does not return until choose `Ok`; no *T+1* events yet |
| default mock 2 ticks | hash still `70e5204d…`; no `[llm] barrier` in shipping TOML |
| `--llm-barrier` + chooser fails twice then `Ok` | no `LlmWait`; action is the success |
| `--llm-barrier` + chooser always Timeout | 4 attempts (`1+3`); then `LlmWait` + Wait; later agents this tick still run; `tick()` returns |
| `--llm-barrier-retries 0` + Timeout | one attempt; `LlmWait` |
| barrier off + timeout Err | immediate `LlmWait` + Wait; no extra attempts (today) |
| `--connect` without `--allow-control` | hash-neutral; no Control sent |
| `--connect --allow-control` + `/play` to `--start-paused` server | ticks advance; `/pause` stops |
| `--connect --allow-control` + `/give 0 berry_bush 1` | server inventory + hash change (server has `--allow-control`) |
| `--connect --allow-control` vs server without flag | ControlDisabled |
| remote `/set` | still refused |
| Hello v4 | unchanged; no v5 |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: MF-2 viewer every tick

- **Files:** `crates/viewer/src/net.rs` (throttle, queue, Snapshot `send`), `ui.rs` (world vs live tick), `sim-bevy` one-tick-per-frame, viewer tests

### PR 2: MF-3 pipeline verify + barrier

- **Files:** `sim-core` remember timing + completeness, overlay `[llm] barrier` / `barrier_retries`, `--llm-barrier` / `--llm-barrier-retries`, viewer pipeline N/N, unit tests (sleep / fail-then-Ok / fail-all)

### PR 3: sim-cli `--connect` Control

- **Files:** `sim-cli` `client.rs` stdin parser + `--allow-control` on connect, help text, `tests/net.rs` loopback `/play` `/give`

No postcard enum append. No `PROTOCOL_VERSION` bump.

## Config / CLI

No shipping TOML change. Overlay `[llm] barrier` / `barrier_retries` is not `ExperimentConfig`. No new postcard fields.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm mock --llm-barrier --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# 3D should visit every tick; Status shows world vs live if catching up
# pipeline N/N in inspector

# other terminal: stdin Control
cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --allow-control
# /play
# /give 0 berry_bush 1
# /pause
```

## Verification

Walkthrough: [`M23-test-plan.md`](M23-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: attach world visits every tick in order; mock pipeline N/N; `--llm-barrier` retries then Wait; `--connect --allow-control` `/play`/`/give` work; Hello stays v4; default mock hashes match.

## Risks

- **Snapshot every tick** is a full ckpt encode. Accepted for attach; mock headless tests do not attach a viewer.
- **Blocking Snapshot `send`** can stall the IO thread. Tick still `try_send` + live clock.
- **`[llm] barrier` on `LlmParams` would change `config_hash`.** Overlay + CLI only; leave hashed `max_retries = 2`.
- **Barrier × `timeout_ms=120s`** is slow (8 min/agent worst case, 4 attempts). Default off; CI mock never sets it; not an infinite hang.
- **New `SimEventKind` would change `state_hash`.** Completeness is timing/HUD only.
- **`--allow-control` means different things** on listen vs connect. Help text: server accepts Control; connect-client **sends** Control.
- **No protocol bump.** Reuse verbs. Hello stays v4.
