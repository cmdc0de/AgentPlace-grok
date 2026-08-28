# M6 — imgui research UI, agent-POV fog-of-war, and viewer commands

**Status:** implemented (git tag `M6`, commit `46c75e0`).  
**Depends on:** M5 complete (`docs/M5-plan.md`, commit `7de52be`)  
**Specs:** `simulation-architecture-spec.md` § GUI / imgui, `decision-observation-llm-economy-metrics-spec.md` § observation (“what the agent sees” = GUI POV), `medium-priority-specs.md` §7 (richer GUI)

## Context

M5 delivered hybrid memory, millipoint relationships, decision JSONL, and per-kind 3D meshes. Chrome is still Bevy `Text`: a status HUD, a left-side legend column, and key-binding instructions that fight each other for screen space. A researcher who wants a food-economy report has to leave the viewer and run `sim-cli --report`.

Specs have always named this slice: **Dear ImGui panels** plus **agent-POV that matches `Observation`**. M6 is that slice, plus a **slash-command console** so `/report` (and kin) work without dropping the 3D view.

M6 does **not** add TCP/WebSocket, incentive injection, timeline scrubbing, or world-mutation cheats (`/set hunger`, `/give`).

## Goal

A researcher can:

1. Use **imgui windows** for legend, help/controls, agent inspector, logs, board, and world snapshot.
2. Type **`/report`** (and a closed command set) in a console while the sim is on screen.
3. Follow an agent and, optionally, see **only what that agent’s `Observation` contains** (fog-of-war).
4. Keep M5 3D meshes and keyboard shortcuts. `sim-core` stays imgui-free; `format_version` stays **2**; `state_hash` is unchanged by UI.

## In scope

| Area | M6 meaning |
|---|---|
| **imgui (viewer only)** | `bevy_mod_imgui` **0.10** on Bevy **0.19**. If it fails to link, stop and swap — do not invent a second UI kit. `sim-core` / `sim-cli` never import imgui. |
| **Legend window** | Dockable imgui: colour swatch + shape name + species, same table as `sim_core::markers`. Replaces Bevy `Text` legend. Toggle `L` + window close. |
| **Help / controls window** | Dockable imgui listing keys and `/commands`. Replaces the instruction string in the HUD. Toggle `H`. |
| **Status** | Tick, pause/run, follow id, short hash, open-proposal count — imgui (bar or small window). Remove the overlapping Bevy instruction `Text`. |
| **Agent inspector** | Followed or picked agent: needs bars, inventory (species names), goals, personality/abilities, allergies, illness, relationship table, last `policy_branch` + chosen action. |
| **Agent list** | All agents; click to follow. Row shows hunger + last branch. |
| **Event log** | Last *N* `SimEvent`s (cap ~200). Filter by agent id and kind. Read-only. |
| **Decision log** | `last_tick_decisions` plus a small **viewer-only** ring of recent ticks (not hashed, not checkpointed). |
| **Board panel** | Open proposals (id, text, rule, yes/no) and adopted rules. Visible even under fog (`public_board_always_visible`). |
| **World snapshot** | Live `--report` numbers: consumption totals, hunger mean, board counts, relationship pairs. No time-series charts. |
| **Command console** | Imgui input + scrollback. Focus `/` or backtick; Enter runs; Esc unfocuses. History (~50). Closed verb set below. |
| **Agent-POV fog-of-war** | Follow on **and** “match observation” checked: hide 3D markers not in `observation::build` for that agent. Unidentified agents stay unnamed capsules. Spectator (no follow) = today’s omniscient 3D view. |
| **Keys still work** | Space pause, `.` step, `F` / `0`–`9` follow, plus `H` `I` `B` `O` `L` `/`. Imgui buttons duplicate pause/step/follow. |

### Command console — closed set

Unknown verb → `unknown: …` and hint `/help`. Errors print in the console; never panic. Default write dir = config `checkpoint.directory` (`checkpoints/`). **`/report` and `/save` do not change `state_hash`.**

| Command | Effect |
|---|---|
| `/help` | List commands (same content as Help window). |
| `/report [dir]` | `build_report` → `{id}_tick_{tick}_report.md` (+ csv if config). Print path. |
| `/summarize` | Print `summary_markdown` in the console (optional write `_summary.md`). |
| `/save [path]` | Existing `write_run_checkpoint` / `save_checkpoint`. |
| `/follow N` / `/follow off` | Same as follow keys. |
| `/pause` `/play` `/step [n]` | Pause, unpause, advance *n* ticks (default 1). |
| `/fog on\|off` | Toggle POV fog-of-war. |
| `/legend` `/inspector` `/board` `/log` | Toggle those windows. |
| `/tick` | Print tick + short hash. |

No `/set`, `/give`, `/inject`. Those wait for M8.

## Out of scope (later)

| Later | What |
|---|---|
| **M7** | Done — [`M7-plan.md`](M7-plan.md) (TCP / WebSocket; remote GUI clients) |
| **M8** | Done — [`M8-plan.md`](M8-plan.md) (incentive A/B + timing + death) |
| **M9** | Done — [`M9-plan.md`](M9-plan.md) — LLM prompts, replay, `--compare` |
| **M10** | Done — [`M10-plan.md`](M10-plan.md) — LLM parse/replay; Transfer/Store; `/give` |
| **M11** | Done — [`M11-plan.md`](M11-plan.md) — mock fills crates; Basket backpack; satchel mesh |
| **M12** | Done — [`M12-plan.md`](M12-plan.md) — public vs hidden incentives |
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
| **M14** | Done — [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done — [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done — [`M20-plan.md`](M20-plan.md) — revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done — [`M21-plan.md`](M21-plan.md) — pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| Later | Metrics charts, browser imgui, live embeddings |

## Key decisions

1. **`bevy_mod_imgui` 0.10** matches Bevy 0.19. Do not fall back to egui silently.
2. **imgui is a viewer concern.** No imgui types in `sim-core`.
3. **3D markers stay.** imgui labels them; it does not replace meshes.
4. **Fog-of-war uses `observation::build`.** Same Chebyshev ranges as the decision loop.
5. **Board stays fully visible in imgui under fog.**
6. **UI state is session-only.** Open windows, follow id, fog toggle, command history are not checkpointed. `format_version = 2`.
7. **Decision/event/report panels are derived.** They do not enter `state_hash`.
8. **Keyboard shortcuts remain.** imgui is additive.
9. **Console is a closed verb set.** File writes + camera/UI only.
10. **Headless CI unchanged.** `cargo test -p sim-core` does not open a window.

## Tests (M6 acceptance bar)

| Test | Asserts |
|---|---|
| `cargo test -p sim-core` still green | no imgui in core |
| `visible_in_observation(obs, x, y)` | true for tiles in `obs.tiles`, false outside |
| Fog helper vs full-info | `full_information` observation includes all land cells the agent can legally see at max range |
| Command parse: `/help` | known verbs listed |
| Command parse: `/nope` | `unknown` |
| `/report` (unit, no window) | writes a report file via the same `write_report` as CLI; hash unchanged |
| Marker table | berry_bush shape ≠ herb (M5 invariant) |

Imgui layout is **manual**: `cargo run -p viewer`. No GPU screenshot CI.

## PR Plan

### PR 1: imgui overlay

- **Files:** `crates/viewer/Cargo.toml`, `main.rs`
- **Changes:** `bevy_mod_imgui` 0.10; empty context; confirm a demo window draws over the 3D view.

### PR 2: Legend + Help; drop Bevy Text chrome

- **Files:** `viewer/src/main.rs`
- **Changes:** Legend window (swatches from `markers`); Help window (keys + commands). Delete `LegendText` and the instruction block. Keep a thin status in imgui.

### PR 3: Inspector + agent list + status controls

- **Files:** `viewer` (new `ui/` module ok)
- **Changes:** Agent list, inspector, pause/step/follow buttons.

### PR 4: Logs, board, world snapshot

- **Files:** `viewer` ui
- **Changes:** Event log (cap 200), decision ring, board panel, live report numbers.

### PR 5: Command console

- **Files:** `viewer` command parse (pure fn, testable without GPU)
- **Changes:** `/` focus; closed verbs; `/report` `/summarize` `/save` `/follow` `/pause` `/step` `/fog` + window toggles.

### PR 6: Fog-of-war

- **Files:** `viewer` marker spawn/visibility; helper `visible_in_observation`
- **Changes:** Follow + fog hides markers not in that observation. Spectator = omniscient.

### PR 7: Tests + README

- **Files:** `crates/sim-core` or `viewer` unit tests for parse + visibility; `README.md` viewer keys
- **Changes:** Acceptance bar; docs point at M6.

## Files / reuse

**Reuse:** M5 `markers` table, `observation::build`, `build_report` / `write_report` / `summary_markdown` / `write_run_checkpoint`, `last_tick_decisions`, follow/pause/step in `sim-bevy`.

**Do not touch in M6:** `shared::transport`, incentive types, `sim-core` HTTP, vote weights, embedding APIs.

## Verification (when M6 is implemented)

```bash
cargo test -p sim-core
cargo run -p viewer -- --config configs/default.toml
# H help, L legend, I inspector, B board, / then report
# follow an agent (F or /follow 0), O fog — distant veg/agents disappear
```

`/report` writes a markdown file; `state_hash` before and after is identical. Headless `sim-cli` 80-tick continuation hash still matches (UI is not in the hash).

## Risks

- **Bevy 0.19 + imgui input.** Mouse capture can steal camera/click-to-follow. Unfocus console on Esc; do not process `/` as a world key when the input is focused.
- **Fog rebuild cost.** Rebuilding every marker every tick is too heavy. Hide/show existing entities (or a visibility component) keyed by cell.
- **Follow without fog vs spectator.** Default fog **off** so existing “look at the whole map” workflow stays. POV is opt-in (`O` or `/fog on`).
- **Command scope creep.** Resist `/set` and incentive inject; they need M8’s explicit experimental protocol.
- **Legend drift.** imgui swatches must call `markers::*` — do not duplicate RGB literals in the UI crate.
