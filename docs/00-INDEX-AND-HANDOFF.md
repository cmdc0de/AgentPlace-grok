# Multi-Agent Simulation – Specification Handoff for Grok Build

**Project goal (short)**  
Build a deterministic, configurable multi-agent social simulation in Rust (Bevy + wgpu) where agents with personalities, limited perception, memory, and goals interact in a shared world. The primary experimental use is:

1. Run a baseline and observe emergent rules / governance.
2. Load a checkpoint, inject different incentive schedules, and measure what changes.

Supports Windows, Linux (Debian-based), and macOS. Clients attach via in-process, raw TCP, or WebSocket. GUI uses imgui + 3D view (including “see what the agent sees”).

---

## Specification Files (read in this order)

| Order | File | Contents |
|-------|------|----------|
| 1 | `deterministic-seeding-design.md` | Hierarchical master seed, RNG streams, reproducibility, checkpoint branching |
| 2 | `simulation-architecture-spec.md` | Bevy recommendation, platform support, attachable clients, TCP **and** WebSocket transports, GUI, project layout |
| 3 | `simulation-and-agents-spec.md` | Config-driven world generation, agent population, personality, memory capacity, personal/public goals, social influence |
| 4 | `memory-goals-incentives-spec.md` | Hybrid memory + eviction, relationship summaries, public goals & rule proposal lifecycle, incentive schedule format |
| 5 | `decision-observation-llm-economy-metrics-spec.md` | Decision loop, per-agent perceptiveness, LLM contract (local **and** frontier models), resources (vegetation/animal/fish + toxicity), metrics including consumption |
| 6 | `medium-priority-specs.md` | Communication (free secondary action + length limit), conflict/sanctions (v1 social, v2-ready), checkpoints + Markdown summaries, error handling (timeout = do nothing), testing strategy, when to wire 3D viewer |

---

## How to give this context to Grok Build

**Option A – Recommended**  
Upload or attach **all six `.md` files** plus this `00-INDEX-AND-HANDOFF.md` into the Grok Build session / project. Start the conversation with:

> “Here are the complete design specifications for the multi-agent simulation. Read `00-INDEX-AND-HANDOFF.md` first, then the files in the listed order. We are ready to begin implementation. Propose the initial crate/workspace layout and the first concrete milestone.”

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

## Suggested first request to Grok Build

```
We are starting implementation of the multi-agent simulation described in the attached specs.
Please:
1. Propose a Cargo workspace / crate layout that matches the architecture doc.
2. Define the first vertical-slice milestone that gets us to a deterministic headless tick + minimal 3D view of agent positions.
3. List the exact next implementation steps after that.
Do not re-open settled design decisions unless you see a clear contradiction in the specs.
```

---

*Generated from the design conversation. All detailed requirements live in the six specification files listed above.*
