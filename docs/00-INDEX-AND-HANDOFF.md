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
| **M16** | Done (tag `M16`) | [`M16-plan.md`](M16-plan.md), walkthrough [`M16-test-plan.md`](M16-test-plan.md) | Council/unanimous votes, `/set`, range-limited board |
| **M17** | Done (tag `M17`) | [`M17-plan.md`](M17-plan.md), walkthrough [`M17-test-plan.md`](M17-test-plan.md) | Meta-rules, join/leave one-shots, event-log timeline, Spark overnight A/B |
| **M18** | Done (tag `M18`) | [`M18-plan.md`](M18-plan.md), walkthrough [`M18-test-plan.md`](M18-test-plan.md) | Weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done (tag `M19`) | [`M19-plan.md`](M19-plan.md), walkthrough [`M19-test-plan.md`](M19-test-plan.md) | SetCouncilTally meta-rule, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done (tag `M20`) | [`M20-plan.md`](M20-plan.md), walkthrough [`M20-test-plan.md`](M20-test-plan.md) | Revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done (tag `M21`) | [`M21-plan.md`](M21-plan.md), walkthrough [`M21-test-plan.md`](M21-test-plan.md) | Pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | Done (tag `M22`) | [`M22-plan.md`](M22-plan.md), walkthrough [`M22-test-plan.md`](M22-test-plan.md) | Wire Give, remote /ckpt and /events |
| **M23** | Done (tag `M23`) | [`M23-plan.md`](M23-plan.md), walkthrough [`M23-test-plan.md`](M23-test-plan.md) | See every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | Done (tag `M24`) | [`M24-plan.md`](M24-plan.md), walkthrough [`M24-test-plan.md`](M24-test-plan.md) | Wire /set, connect /inject, lockstep ack |
| **M25** | Done (tag `M25`) | [`M25-plan.md`](M25-plan.md), walkthrough [`M25-test-plan.md`](M25-test-plan.md) | Reflection-on-evict, lockstep Ack timeout |
| **M26** | Done (tag `M26`) | [`M26-plan.md`](M26-plan.md), walkthrough [`M26-test-plan.md`](M26-test-plan.md) | Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done (tag `M27`) | [`M27-plan.md`](M27-plan.md), walkthrough [`M27-test-plan.md`](M27-test-plan.md) | Auto-execute plan, combat |
| **M28** | Done (tag `M28`) | [`M28-plan.md`](M28-plan.md), walkthrough [`M28-test-plan.md`](M28-test-plan.md) | Health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done (tag `M29`) | [`M29-plan.md`](M29-plan.md), walkthrough [`M29-test-plan.md`](M29-test-plan.md) | Local embeddings, combat death, CI Win/mac |
| **M30** | Done (tag `M30`) | [`M30-plan.md`](M30-plan.md), walkthrough [`M30-test-plan.md`](M30-test-plan.md) | Combat particles / meshes |
| **M31** | Done (tag `M31`) | [`M31-plan.md`](M31-plan.md), walkthrough [`M31-test-plan.md`](M31-test-plan.md) | Kinship, reproduction, D&D-like sheet |
| **M32** | Done (tag `M32`) | [`M32-plan.md`](M32-plan.md), walkthrough [`M32-test-plan.md`](M32-test-plan.md) | kin_of incentives, household, aging |
| **M33** | Done (tag `M33`) | [`M33-plan.md`](M33-plan.md), walkthrough [`M33-test-plan.md`](M33-test-plan.md) | Household crates, culture inheritance, reflect importance |
| **M34** | Done (tag `M34`) | [`M34-plan.md`](M34-plan.md), walkthrough [`M34-test-plan.md`](M34-test-plan.md) | Sheet effects, close-kin PairBond |
| **M35** | Done (tag `M35`) | [`M35-plan.md`](M35-plan.md), walkthrough [`M35-test-plan.md`](M35-test-plan.md) | Inventions, browser attach |
| **M36** | Done (tag `M36`) | [`M36-plan.md`](M36-plan.md), walkthrough [`M36-test-plan.md`](M36-test-plan.md) | Browser researcher UI (no 3D) |
| **M37** | Done (tag `M37`) | [`M37-plan.md`](M37-plan.md), walkthrough [`M37-test-plan.md`](M37-test-plan.md) | Extra invention kinds, browser /set /give |
| **M38** | Done (tag `M38`) | [`M38-plan.md`](M38-plan.md), walkthrough [`M38-test-plan.md`](M38-test-plan.md) | Viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done (tag `M39`) | [`M39-plan.md`](M39-plan.md), walkthrough [`M39-test-plan.md`](M39-test-plan.md) | Object definition files (visual + LOD + hashed catalog) |
| **M40** | Planned — next | [`M40-plan.md`](M40-plan.md) | Config-owned objects + recipes (except agent) |
| After M40 | Not started | listed at the bottom of `M40-plan.md` | protobuf/TLS |

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
| 6b | `missing-features.md` | Open researcher gaps found in use (not After-M later-table) |
| 6c | `post-ga-feature-list.md` | Post-GA backlog (kinship, reproduction, D&D-like sheet). Not a milestone plan. |
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
| 21 | `M16-plan.md` | Milestone 16 (done, tag `M16`): council/unanimous, `/set`, range-limited board |
| 21b | `M16-test-plan.md` | M16 walkthrough (unanimous/council, `/set`, board fog) |
| 22 | `M17-plan.md` | Milestone 17 (done, tag `M17`): meta-rules, join/leave one-shots, event JSONL timeline |
| 22b | `M17-test-plan.md` | M17 walkthrough (meta-rules, oneshots, JSONL, Spark overnight A/B) |
| 23 | `M18-plan.md` | Milestone 18 (done, tag `M18`): weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| 23b | `M18-test-plan.md` | M18 walkthrough (majority-of-council, SetCouncil, `/scrub` catch-up) |
| 24 | `M19-plan.md` | Milestone 19 (done, tag `M19`): SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| 24b | `M19-test-plan.md` | M19 walkthrough (SetCouncilTally, `/set respect`, Backpack, crate scale) |
| 25 | `M20-plan.md` | Milestone 20 (done, tag `M20`): revert relationship_delta on leave, stacked worn packs, attach safety net |
| 25b | `M20-test-plan.md` | M20 walkthrough (relationship leave/end, stacked packs, attach net) |
| 26 | `M21-plan.md` | Milestone 21 (done, tag `M21`): pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| 26b | `M21-test-plan.md` | M21 walkthrough (pack fill, `--connect`, remote `/scrub`) |
| 27 | `M22-plan.md` | Milestone 22 (done, tag `M22`): wire Give, remote /ckpt and /events |
| 27b | `M22-test-plan.md` | M22 walkthrough (wire Give, remote `/ckpt`, `/events`) |
| 28 | `M23-plan.md` | Milestone 23 (done, tag `M23`): see every tick, LLM pipeline barrier, sim-cli Control |
| 28b | `M23-test-plan.md` | M23 walkthrough (every-tick Snapshots, LLM barrier, `--connect` Control) |
| 29 | `M24-plan.md` | Milestone 24 (done, tag `M24`): wire /set, connect /inject, lockstep ack |
| 29b | `M24-test-plan.md` | M24 walkthrough (wire `/set`, `--connect` `/inject`, lockstep ack) |
| 30 | `M25-plan.md` | Milestone 25 (done, tag `M25`): reflection-on-evict, lockstep Ack timeout |
| 30b | `M25-test-plan.md` | M25 walkthrough (reflect-on-evict, lockstep Ack timeout) |
| 31 | `M26-plan.md` | Milestone 26 (done, tag `M26`): Reflect/Plan every-N-ticks, record/replay of reflection text |
| 31b | `M26-test-plan.md` | M26 walkthrough (Reflect/Plan every-N-ticks, record/replay of reflection text) |
| 32 | `M27-plan.md` | Milestone 27 (done, tag `M27`): auto-execute plan, combat |
| 32b | `M27-test-plan.md` | M27 walkthrough (auto-execute plan, combat) |
| 33 | `M28-plan.md` | Milestone 28 (done, tag `M28`): health/incapacitation, combat viewer FX, force_reflect |
| 33b | `M28-test-plan.md` | M28 walkthrough (health/incapacitation, combat viewer FX, force_reflect) |
| 34 | `M29-plan.md` | Milestone 29 (done, tag `M29`): local embeddings, combat death, CI Win/mac |
| 34b | `M29-test-plan.md` | M29 walkthrough (local embeddings, combat death, CI Win/mac) |
| 35 | `M30-plan.md` | Milestone 30 (done, tag `M30`): combat particles / meshes |
| 35b | `M30-test-plan.md` | M30 walkthrough (combat particles / meshes) |
| 36 | `M31-plan.md` | Milestone 31 (done, tag `M31`): kinship, reproduction, D&D-like sheet |
| 36b | `M31-test-plan.md` | M31 walkthrough (kinship, reproduction, D&D-like sheet) |
| 37 | `M32-plan.md` | Milestone 32 (done, tag `M32`): kin_of incentives, household, aging |
| 37b | `M32-test-plan.md` | M32 walkthrough (kin_of incentives, household, aging) |
| 38 | `M33-plan.md` | Milestone 33 (done, tag `M33`): household crates, culture inheritance, reflect importance |
| 38b | `M33-test-plan.md` | M33 walkthrough (household crates, culture inheritance, reflect importance) |
| 39 | `M34-plan.md` | Milestone 34 (done, tag `M34`): sheet effects, close-kin PairBond |
| 39b | `M34-test-plan.md` | M34 walkthrough (sheet effects, close-kin PairBond) |
| 40 | `M35-plan.md` | Milestone 35 (done, tag `M35`): inventions, browser attach |
| 40b | `M35-test-plan.md` | M35 walkthrough (inventions, browser attach) |
| 41 | `M36-plan.md` | Milestone 36 (done, tag `M36`): browser researcher UI (no 3D) |
| 41b | `M36-test-plan.md` | M36 walkthrough (InspectorView JSON, postcard WS tables) |
| 42 | `M37-plan.md` | Milestone 37 (done, tag `M37`): extra invention kinds, browser /set /give |
| 42b | `M37-test-plan.md` | M37 walkthrough (MoveBonus/SenseBonus, page /set /give) |
| 43 | `M38-plan.md` | Milestone 38 (done, tag `M38`): viewer 3D models, browser /inject /scrub + wasm32 CI |
| 43b | `M38-test-plan.md` | M38 walkthrough (authored models, page /inject /scrub, Ubuntu wasm32) |
| 44 | `M39-plan.md` | Milestone 39 (done, tag `M39`): object definition files (visual + LOD + hashed catalog) |
| 44b | `M39-test-plan.md` | M39 walkthrough (visual/LOD hash-neutral, hashed catalog Craft) |
| 45 | `M40-plan.md` | Milestone 40 (planned): config-owned objects + recipes (except agent) |

---

## How to give this context to Grok Build

**Option A – Recommended**  
Upload or attach the specification files **and** the milestone plans (`M2-plan.md` through `M40-plan.md`) plus this `00-INDEX-AND-HANDOFF.md`. Start the conversation with:

> “Here are the design specifications and milestone plans. Read `00-INDEX-AND-HANDOFF.md` first (progress table). M40 is the current slice (`M40-plan.md`). Do not expand into items it defers (protobuf/TLS, DEX accuracy, agent meshes).”

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

M1–M39 are done. Current slice is [`M40-plan.md`](M40-plan.md) (planned, not yet implemented).

```
Read docs/00-INDEX-AND-HANDOFF.md, then docs/M40-plan.md.
M40 is the current slice (config-owned objects + recipes except
agent; PROTOCOL_VERSION=5). Later work is at the bottom of that
file. Do not add protobuf, TLS, agent meshes, or Bevy in the
browser. CI stays provider=mock with no network.
```

---

*Generated from the design conversation. Detailed requirements live in the specification files; current-slice scope lives in the milestone plans.*
