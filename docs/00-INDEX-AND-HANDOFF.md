# Multi-Agent Simulation – Specification Handoff for Grok Build

**Project goal (short)**  
Build a deterministic, configurable multi-agent social simulation in Rust (Bevy + wgpu) where agents with personalities, limited perception, memory, and goals interact in a shared world. The primary experimental use is:

1. Run a baseline and observe emergent rules / governance.
2. Load a checkpoint, inject different incentive schedules, and measure what changes.

Supports Windows, Linux (Debian-based), and macOS. Clients attach via in-process, raw TCP, or WebSocket. GUI uses imgui + 3D view (including “see what the agent sees”).

---

## Implementation progress

Read this table first in a new session, then the specs, then the current milestone plan.

| Milestone | Status | Doc | What it is |
|-----------|--------|-----|------------|
| **M1** | Done (`ff3d21b`) | (commit message; no separate plan file) | Workspace, seeded ticks, heightmap, random-walk agents, hash tests, headless `sim-cli`, in-process Bevy viewer |
| **M2** | Done | [`M2-plan.md`](M2-plan.md) | Water/vegetation/mineral layers, land-only spawn, versioned checkpoints + Markdown + JSONL, viewer `--load` |
| **M3** | Done (tag `M3`) | [`M3-plan.md`](M3-plan.md) | Observation, needs, abilities, toxicity, speech, mock + Ollama/xAI action selection |
| **M4** | Done (`6b4cf93`) | [`M4-plan.md`](M4-plan.md) | Public board, Propose/Support/Oppose, structured adopted rules, food-economy summary report |
| **M5** | Done (`7de52be`) | [`M5-plan.md`](M5-plan.md) | Hybrid memory, relationships, influence, decision JSONL, viewer legend + per-kind meshes |
| **M6** | Done (tag `M6`, `46c75e0`) | [`M6-plan.md`](M6-plan.md) | imgui research UI, agent-POV fog-of-war, slash-command console (`/report`) |
| **M7** | Done (tag `M7`, `789fc81`) | [`M7-plan.md`](M7-plan.md) | Attachable TCP and WebSocket clients (hash-neutral read-only attach) |
| **M8** | Done (tag `M8`, `b0b9d14`) | [`M8-plan.md`](M8-plan.md) | Incentive A/B inject + processing (wall-clock) metrics; death |
| **M9** | Done (tag `M9`) | [`M9-plan.md`](M9-plan.md), walkthrough [`M9-test-plan.md`](M9-test-plan.md) | LLM prompts (needs + incentives), replay, `--compare`, mock drink-before-death |
| **M10** | Done (tag `M10`) | [`M10-plan.md`](M10-plan.md), walkthrough [`M10-test-plan.md`](M10-test-plan.md) | Live LLM parse/replay reliability; Transfer/Store containers (weight, energy, 3D marker) |
| **M11** | Done (tag `M11`) | [`M11-plan.md`](M11-plan.md), walkthrough [`M11-test-plan.md`](M11-test-plan.md) | Mock fills crates; worn Basket backpack; Move haul |
| **M12** | Done (tag `M12`) | [`M12-plan.md`](M12-plan.md), walkthrough [`M12-test-plan.md`](M12-test-plan.md) | Public vs hidden incentives (Observation/prompt) |
| **M13** | Done (tag `M13`) | [`M13-plan.md`](M13-plan.md), walkthrough [`M13-test-plan.md`](M13-test-plan.md) | Opt-in influence-weighted votes |
| **M14** | Done (tag `M14`) | [`M14-plan.md`](M14-plan.md), walkthrough [`M14-test-plan.md`](M14-test-plan.md) | Coalition `supporters_of` + viewer checkpoint scrubber |
| **M15** | Done (tag `M15`) | [`M15-plan.md`](M15-plan.md), walkthrough [`M15-test-plan.md`](M15-test-plan.md) | Opt-in respect-weighted votes |
| After M15 | Not started | listed at the bottom of `M15-plan.md` | `/set`, council/unanimous, action/board fog, protobuf/TLS, wire Give |

Specs remain the long-term source of truth. Milestone plans record **what we are building now** and explicitly defer the rest. If a milestone plan and a spec disagree on timing, the milestone plan wins for the current slice; do not silently expand scope.

---

## Specification Files (read in this order)

| Order | File | Contents |
|-------|------|----------|
| 1 | `deterministic-seeding-design.md` | Hierarchical master seed, RNG streams, reproducibility, checkpoint branching |
| 2 | `simulation-architecture-spec.md` | Bevy recommendation, platform support, attachable clients, TCP **and** WebSocket transports, GUI, project layout |
| 3 | `simulation-and-agents-spec.md` | Config-driven world generation, agent population, personality, memory capacity, personal/public goals, social influence |
| 4 | `memory-goals-incentives-spec.md` | Hybrid memory + eviction, relationship summaries, public goals & rule proposal lifecycle, incentive schedule format (long-term) |
| 4b | `incentive-schedule-format.md` | **M8 file format:** one TOML schedule, many `[[incentives]]`, closed effect fields |
| 4c | `needs-and-survival.md` | Hunger / thirst / energy: millipoints, decay, mock “hungry” cutoffs, refill actions |
| 5 | `decision-observation-llm-economy-metrics-spec.md` | Decision loop, per-agent perceptiveness, LLM contract (local **and** frontier models), resources (vegetation/animal/fish + toxicity), metrics including consumption |
| 6 | `medium-priority-specs.md` | Communication (free secondary action + length limit), conflict/sanctions (v1 social, v2-ready), checkpoints + Markdown summaries, error handling (timeout = do nothing), testing strategy, when to wire 3D viewer |
| 7 | `M2-plan.md` | Milestone 2 (done): resources, checkpoints, Markdown summaries |
| 8 | `M3-plan.md` | Milestone 3 (done): observation, needs, abilities, toxicity, speech, LLM |
| 9 | `M4-plan.md` | Milestone 4 (done): public board, adopted rules, food-economy report |
| 10 | `M5-plan.md` | Milestone 5 (done): hybrid memory, relationships, influence, decision logs, viewer legend |
| 11 | `M6-plan.md` | Milestone 6 (done): imgui UI, agent-POV fog-of-war, viewer commands |
| 12 | `M7-plan.md` | Milestone 7 (done): TCP + WebSocket attachable clients |
| 13 | `M8-plan.md` | Milestone 8 (done): incentive A/B inject + processing metrics |
| 14 | `M9-plan.md` | Milestone 9 (done, tag `M9`): LLM-in-the-loop prompts, replay, `--compare` |
| 15 | `M10-plan.md` | Milestone 10 (done, tag `M10`): thinking-model parse + honest replay; Transfer/Store containers |
| 16 | `M11-plan.md` | Milestone 11 (done, tag `M11`): mock fills crates; Basket backpack; Move haul |
| 16b | `M11-test-plan.md` | M11 walkthrough (80-tick coop Store, pack, satchel) |
| 17 | `M12-plan.md` | Milestone 12 (done, tag `M12`): public vs hidden incentives |
| 17b | `M12-test-plan.md` | M12 walkthrough (hidden banner, same-hash mock) |
| 18 | `M13-plan.md` | Milestone 13 (done, tag `M13`): opt-in influence-weighted votes |
| 18b | `M13-test-plan.md` | M13 walkthrough (equal vs influence kingmaker) |
| 19 | `M14-plan.md` | Milestone 14 (done, tag `M14`): coalition targeting + checkpoint scrubber |
| 19b | `M14-test-plan.md` | M14 walkthrough (supporters_of + `--load DIR` scrubber) |
| 20 | `M15-plan.md` | Milestone 15 (done, tag `M15`): opt-in respect-weighted votes |
| 20b | `M15-test-plan.md` | M15 walkthrough (equal vs respect kingmaker) |

---

## How to give this context to Grok Build

**Option A – Recommended**  
Upload or attach the specification files **and** the milestone plans (`M2-plan.md` through `M15-plan.md`) plus this `00-INDEX-AND-HANDOFF.md`. Start the conversation with:

> “Here are the design specifications and milestone plans. Read `00-INDEX-AND-HANDOFF.md` first (progress table). M15 is implemented; do not expand into items it defers (`/set`, council/unanimous, action/board fog, protobuf/TLS, wire Give).”

**Option B – Single file**  
If Grok Build prefers one document, ask the previous chat (or this one) to produce a concatenated `FULL-SPEC.md`. The individual files remain the source of truth.

**Option C – Progressive**  
Start Grok Build with only the Index + Architecture + Seeding docs, then feed the others as you reach those subsystems.

---

## Current design decisions that should not be re-litigated without reason

- Language / engine: **Rust + Bevy** (wgpu under the hood), headless-capable.
- Cross-platform: Windows, Linux (Debian), macOS.
- Transports: **both raw TCP and WebSocket**, same protocol, shared package structure.
- Speaking = free secondary action with hard length limit.
- LLM: provider-agnostic (local on Spark-class hardware **and** frontier models).
- Perception: base ranges × individual `perceptiveness` trait.
- Resources: edible vegetation, animal life, fish (+ water); some toxic/allergenic.
- Timeouts: LLM timeout → agent does nothing that step.
- Checkpoints: binary + Markdown summaries (overall + per-agent).
- Conflict: v1 = social + soft mechanical; code structured so physical combat is a later extension, not a rewrite.
- First 3D view: as soon as agents have positions (early morale + debugging milestone).

---

## Suggested next request to Grok Build

M1–M15 are done (tag `M15`). Later work is listed at the bottom of `M15-plan.md`.

```
Read docs/00-INDEX-AND-HANDOFF.md, then docs/M15-plan.md.
M15 is implemented ([voting] weight = "respect"; relationship_delta respect +
toward). Do not add /set, council/unanimous, board fog, protobuf, TLS,
or wire Give.
CI stays provider=mock with no network.
```

---

*Generated from the design conversation. Detailed requirements live in the specification files; current-slice scope lives in the milestone plans.*
