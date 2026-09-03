# Missing features

Researcher-facing gaps found in use. **Not** the After-M later-table (protobuf/TLS, combat, wire `/set`). **Not** the post-GA backlog ([`post-ga-feature-list.md`](post-ga-feature-list.md)). Add a row when something is missing or wrong; when a milestone ships it, mark **Done** and point at that plan.

| ID | Area | Status | Gap |
|---|---|---|---|
| MF-1 | Viewer attach HUD | **Done (M22)** | Status line `tick N` is `state.sim.tick` from the **last Snapshot**, not the live server tick. `ServerMessage::Tick` already carries `tick` + `state_hash` every sim step, but `apply_remote` discarded it (`let _ = tick`) and only pulled a Snapshot at most every **200 ms**. Tick frames could also be dropped (`try_send`). **Shipped:** HUD uses a live Tick clock (not dropped) for tick + hash; 3D/world still snapshot-throttled (see MF-2). [`M22-plan.md`](M22-plan.md). |
| MF-2 | Viewer world refresh | **Done (M23)** | Researcher still does **not see every tick** in the 3D view, inspector, or other `state.sim` surfaces. HUD (MF-1) is live; world is not. **Shipped:** Snapshot every Tick, one decode per frame in order, live vs world on Status. [`M23-plan.md`](M23-plan.md). |
| MF-3 | Tick barrier / LLM | **Done (M23)** | Researcher cannot **verify** that every living agent finished every pipeline stage (perceive → retrieve → select → execute → remember) **before** tick *T+1* starts, including while an LLM call is outstanding. Timeout today is `Wait` + continue. **Shipped:** pipeline N/N from timing; opt-in `[llm] barrier` / `--llm-barrier` with default 3 extra retries then Wait. [`M23-plan.md`](M23-plan.md). |

## MF-1 notes

Attach path: `crates/viewer/src/net.rs` `apply_remote`, HUD: `crates/viewer/src/ui.rs` (`"tick {} … hash {short}"`).

In-process viewer is fine: `SimPlugin` ticks the local `Simulation`. This is `--connect` only.

## MF-2 notes

Wanted: attached (and in-process) viewer **shows** tick *T* before it shows *T+1* — agents, packs, crates, board, fog, inspector — not just the Status number.

Today:

- **Attach (`--connect`):** IO thread still `try_send`s every `ServerMessage::Tick` (HUD clock is separate and is **not** dropped). Bevy `apply_remote` uses Tick only for decisions/metrics + to *request* a Snapshot at most every **200 ms**. World state is `Simulation::decode_checkpoint` from that Snapshot. Fast ticks (especially mock) skip 3D frames. A full Bevy channel drops Tick frames entirely, so inspector timing/decisions can skip too.
- **In-process:** `SimPlugin` default `tick_interval_secs = 0.2` (`crates/sim-bevy`). The sim itself only advances on that timer (or `/step`). That is one sim tick per 200 ms wall, not “draw every tick while the sim runs at full speed.”

Not a protocol bump by itself. Likely: apply Tick without Snapshot (or Snapshot every tick), bounded Tick queue instead of `try_send` drop, optional “lockstep” play that waits for the viewer to paint *T* before the server runs *T+1*. In-process: a “tick once per frame / wait for draw” mode.

Do **not** confuse with MF-1 (HUD clock) or MF-3 (sim pipeline barrier).

## MF-3 notes

Wanted: a researcher-facing **proof** that tick *T* does not complete until **every** living agent has run **each** pipeline step, **even if** that agent is blocked on an LLM. No agent skipped; no tick-advance while a choose is still in flight.

What the core already does (`crates/sim-core/src/simulation.rs`):

- `tick()` is synchronous. After world/board, it shuffles turn order and `step_agent` **sequentially**. `tick()` does not return until that loop finishes.
- Each `step_agent` is perceive (`observation::build` + heard memories) → retrieve → select (`chooser.choose`, **blocking**) → execute (primary + optional speak) → remember. Spec Reflect/Plan stages are still unimplemented (M8 timed the actual loop only).
- `Chooser::Custom`: `choose` blocks that agent (and therefore the rest of the tick, because the loop is sequential). `Ok` → that action. `Err` (timeout, HTTP, parse after retries) → `SimEventKind::LlmWait`, `ChosenAction::wait()`, no speak, then **the next agent**. Spec: timeout = do nothing (`docs/medium-priority-specs.md` §4; `llm_timeout_action = "Wait"`). That is **not** “hold tick *T* until this LLM actually answers.”
- Wall clocks exist (`TickTiming` / `{id}_timing.jsonl`, inspector last-step ns). They show how long stages took after the fact. They do **not** assert “N living agents × 5 stages completed, 0 in-flight chooses, then tick advanced.”

Gaps to close (when scheduled):

1. **Policy vs spec.** Holding the tick until every LLM returns a real choice **conflicts** with timeout=`Wait`. Need an explicit mode (e.g. overlay / `--llm-barrier`) that refuses to finish tick *T* while any choose is outstanding — including past `timeout_ms` — vs keep Wait-and-continue for CI/overnight.
2. **Verify.** A check the researcher can run: living-agent count vs `last_tick_timing.agents.len()`, every stage ns present (or an explicit skip reason that is **not** silent), zero `LlmWait` *or* LlmWait counted as “select finished with Wait” depending on the mode. Viewer HUD or `/events` line: `tick T pipeline complete (N/N agents)`. Unit/walkthrough: slow/mock chooser that sleeps; assert tick *T+1* hash/events cannot appear until all chooses return.
3. **Observe.** MF-2 still hides per-tick world; even a correct barrier is hard to *see* while the 3D view jumps. HUD tick (MF-1) is necessary but not sufficient.
