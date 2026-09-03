# High-Priority Runtime Specs
## Agent Decision Loop • Observation Model • LLM Contract • Resources/Needs • Metrics

> **Current slice:** [`M32-plan.md`](M32-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

Related documents:
- `simulation-and-agents-spec.md`
- `memory-goals-incentives-spec.md`
- `deterministic-seeding-design.md`
- `simulation-architecture-spec.md`

---

## 1. Agent Decision / Action Loop

The decision loop is the heartbeat of every agent. It must be deterministic given the same inputs + seed, fully logged, and interruptible by incentives.

### Recommended per-tick pipeline (ordered)

```
1. Perceive          → build current Observation
2. Retrieve          → pull relevant memories (hybrid store)
3. Reflect (optional)→ periodic higher-level insight generation
4. Plan / Rank       → produce or update a short-term plan
5. Select Action     → choose one concrete action (or Wait)
6. Execute           → apply effects to world + self + others
7. Log & Remember    → write MemoryEntries + global event log
```

### Key design choices

- **Synchronous discrete ticks** for the core simulation. All agents run the pipeline in a deterministic order (derived from the turn-order seed).
- **One primary action per tick** (plus free minor acts such as “speak” if the design allows parallel speech). This keeps causality clear.
- **Plan** is a short list of intended future actions or sub-goals (length configurable). It is stored on the agent and can be revised.
- **Reflection** does not run every tick. Trigger it when:
  - a configurable number of new memories have accumulated, or
  - importance of recent events exceeds a threshold, or
  - an incentive forces reflection.
- Every stage emits structured log events so a client can replay “why did this agent do X?”.

### Action vocabulary (initial)

Actions should be structured (not free text) so the world can validate and execute them deterministically:

- `MoveTo(location)` / `MoveRelative(dx, dy)`
- `Gather(vegetation)` / `Hunt(animal)` / `Fish`
- `Drink(water)`
- `Transfer(resource, quantity, target_agent)`
- `Speak(message, target or broadcast)`
- `Propose(proposal_content)`
- `Support(proposal_id)` / `Oppose(proposal_id)`
- `WorkOn(goal_id or joint_project)`
- `Wait` / `Rest`
- `Attack` / `Flee` (only if conflict is enabled)

The LLM (or a rule-based fallback) selects from this vocabulary; the core never executes raw natural language.

---

## 2. Observation / Perception Model

“What the agent sees” must be explicit, configurable, and identical for the GUI’s agent-POV mode.

Perception is **both** a global world setting **and** an individual agent trait. Two agents standing in the same place can receive different observations because one is more perceptive than the other.

### Global defaults

```toml
[observation]
base_vision_range         = 12.0      # world units (before personality modifiers)
base_hearing_range        = 18.0
can_see_through_walls     = false
public_board_always_visible = true    # proposals & adopted rules
resource_discovery        = "must_be_in_range"  # or "global_knowledge"
base_agent_identity_range = 8.0       # beyond this, agents appear as “unknown figure”
```

### Per-agent perception (personality / trait)

Each agent has a `perceptiveness` value (typically 0.0–1.0 or a multiplier) that is part of its personality or a dedicated trait.

```toml
# Example inside an archetype or individual agent
[agents.archetypes.personality]
perceptiveness = 0.75     # 0.0 = very unobservant, 1.0 = highly perceptive
```

**Effect on observation:**
- Effective vision range  = `base_vision_range  * (0.5 + perceptiveness)`
- Effective hearing range = `base_hearing_range * (0.5 + perceptiveness)`
- Effective identity range follows the same pattern
- Higher perceptiveness can also improve the chance of noticing hidden or low-salience details (toxic plants, subtle social cues, distant movement, etc.)

This makes perceptiveness a first-class experimental variable: you can test whether more (or less) perceptive populations form different norms or respond differently to incentives.

### Observation contents (per tick)

An `Observation` given to the agent (and to the GUI when attached to that agent) contains:

- Own state (energy, inventory, active goals, current plan, known allergies/toxins)
- Nearby terrain / edible & toxic resources within *effective* vision range
- Other agents within *effective* vision range (identity if within effective identity range, otherwise anonymous)
- Recent speech that occurred inside *effective* hearing range
- The current public proposal board (if `public_board_always_visible`)
- Any active incentives that the agent is allowed to know about

Everything outside an agent’s personal effective ranges is invisible to that agent. This creates genuine information asymmetry both between locations *and* between individuals.

### Configurability

- Full-information mode (for debugging or certain experiments) should be a single flag that overrides all ranges and perceptiveness.
- Base ranges, discovery rules, and the perceptiveness distribution across the population are first-class experimental variables.

---

## 3. LLM Integration Contract

This is the most critical interface between the deterministic core and the non-deterministic language model.

**Requirement**: The system must support both **frontier / cloud models** (OpenAI, Anthropic, Google, etc.) **and** locally-run models on hardware such as a Spark / GDDR-class GPU box (Ollama, llama.cpp, vLLM, LM Studio, TensorRT-LLM, etc.). The same experiment config should be able to target either class of backend with only provider/model changes.

### Design principles

1. The core never trusts free-form text for world-changing effects.
2. Every LLM call is seeded (see deterministic-seeding design).
3. The LLM may only choose from a declared set of actions or return a structured plan.
4. Failures are handled gracefully and deterministically.
5. The LLM client is a thin, swappable adapter. Switching from a local 7B–70B model to a frontier model (or vice-versa) requires no changes to agent logic or world code.

### Supported backend categories

| Category              | Examples                                      | Typical use                     |
|-----------------------|-----------------------------------------------|---------------------------------|
| Local OpenAI-compatible | Ollama, llama.cpp server, vLLM, LM Studio, Text Generation WebUI | Daily development, large batch runs, privacy |
| Frontier / Cloud      | OpenAI, Anthropic, Google Gemini, Together, Fireworks, etc. | Higher-quality behaviour, final experiments |
| Direct local engines  | llama.cpp (via `llama-cpp-2` or similar), Candle, etc. | Maximum control / minimal overhead |

All backends are accessed through a common trait / interface that exposes:
- chat / completion with seed + temperature
- optional tool / function calling
- timeout and cancellation

### Call types

| Call type          | When used                          | Expected output                     | Temperature (typical) |
|--------------------|------------------------------------|-------------------------------------|-----------------------|
| Action selection   | Most ticks                         | Structured action (JSON / tool call)| 0.0–0.3              |
| Dialogue           | When Speak is chosen               | Natural language utterance          | 0.4–0.7              |
| Reflection         | Periodic or triggered              | Short insight + optional memory importance adjustments | 0.3–0.5 |
| Proposal drafting  | When agent decides to Propose      | Proposal text + optional structured rule | 0.4–0.6         |

### Prompt assembly (Action selection example)

```
System: You are {persona}. Your personality traits are {traits}. 
        Current drives: {drives}. Perceptiveness: {perceptiveness}.
        You must choose exactly one legal action.

Memory (most relevant):
{retrieved_memories}

Current observation:
{observation}

Active personal goals:
{personal_goals}

Known public goals / open proposals:
{public_board}

Active incentives that affect you:
{incentives}

Legal actions this tick:
{action_schema}

Respond with a single JSON object matching the schema.
```

### Structured output

Prefer **tool / function calling** or strict JSON schema mode when the provider supports it.  
Fallback: regex / JSON extraction with a deterministic repair loop (max N retries, then default to `Wait`).

```json
{
  "action": "Gather",
  "target": "edible_vegetation",
  "reasoning": "optional short chain-of-thought for the log"
}
```

(The exact field names are illustrative; the schema will be versioned and shared between core and LLM client.)

### Error & timeout handling

- Timeout or malformed reply → retry with same seed + slightly lower temperature (configurable max retries).
- Still failing → execute `Wait` and log a `LlmFailure` event.
- The failure itself becomes a memory (low importance) so the agent can later reflect on being “confused”.
- Local models may have different latency profiles; timeouts should be configurable per provider.

### Determinism controls

- Explicit `seed` on every call (supported by most local servers and many frontier APIs).
- Temperature low for action selection.
- Exact model name + version (or GGUF hash / snapshot ID) recorded in the experiment config.
- Optional “replay mode”: previously recorded LLM responses can be injected so a run can be reproduced even if the model provider or local weights have changed.

### Config surface

```toml
[llm]
# Provider is the only thing that normally changes between local and frontier runs
provider = "ollama"              # "ollama" | "openai" | "anthropic" | "openai_compatible" | ...
base_url = "http://localhost:11434"  # used by ollama / vLLM / llama.cpp server / etc.
api_key_env = "OPENAI_API_KEY"   # ignored for pure local providers

model = "llama3.2:3b"            # or "gpt-4o", "claude-sonnet-4", "gemini-2.5-pro", etc.
action_temperature = 0.2
dialogue_temperature = 0.6
max_retries = 2
timeout_ms = 12000               # higher for local large models if needed
use_tool_calling = true
replay_file = ""                 # optional path for recorded responses

# Optional per-provider overrides
[llm.provider_options]
num_ctx = 8192                   # local context length, etc.
```

This design lets you develop and run large batches on local hardware (Spark / GDDR-class) and then re-run the exact same configs against frontier models for higher-fidelity results, or the reverse.
---

## 4. Concrete Resources, Needs & Economy

Keep the economy relatively simple and grounded in the natural world. The primary consumable resources are living things that agents can eat; some of them are dangerous.

### Core consumable resources (v1)

| Resource              | Source                     | Edible by default? | Can be toxic / allergenic? | Notes |
|-----------------------|----------------------------|--------------------|----------------------------|-------|
| Edible vegetation     | Plants, berries, roots, etc. | Yes               | Yes (some species)         | Primary plant food |
| Animal life           | Land animals               | Yes               | Yes (some species)         | Requires hunting / scavenging |
| Fish                  | Water bodies               | Yes               | Yes (some species)         | Requires fishing access |
| Water                 | Fresh-water sources        | Yes (drink)       | Rarely contaminated        | Separate thirst need |

**Toxicity & allergies**
- Each specific plant / animal / fish type can be tagged `safe`, `allergenic`, or `toxic`.
- Toxicity can be:
  - **Universal** – harmful to every agent.
  - **Individual** – some agents have allergies (part of their personality / physiology). An agent may safely eat a mushroom that makes another agent sick.
- Consuming a toxic or allergenic item applies immediate or delayed penalties (energy loss, temporary action failure chance, health damage, etc.).
- Agents can learn about toxicity through experience (memory of “I ate X and felt sick”) or through communication / public proposals (“do not eat the red berries”).

### Other basic resources (still useful)

- Wood, stone, simple tools – for building, better gathering, or future expansion. These are secondary in v1.

### Needs

**As implemented (M3–M8):** [`needs-and-survival.md`](needs-and-survival.md). Display 0–100 in TOML; stored as millipoints (`×100`). Mock policy: thirsty if thirst **< half max**, hungry if hunger **< half max**, tired if energy **< one third max**. Decay runs every tick before actions. `death_enabled = true` in default config: hunger or thirst at 0 removes the agent.

```toml
[needs]
hunger_max = 100.0
hunger_decay_per_tick = 0.15
thirst_max = 100.0
thirst_decay_per_tick = 0.25
energy_max = 100.0
energy_decay_per_tick = 0.08
energy_regen_while_resting = 0.4
```

Hunger is satisfied by consuming edible vegetation, animal life, or fish.  
Thirst is satisfied by water.  
Energy is affected by activity, rest, and illness from toxins.

When a need reaches 0 the agent suffers progressive penalties (movement speed, action success chance, eventually death if enabled).

### Economic / survival actions

- `Gather(vegetation)` / `Hunt(animal)` / `Fish`
- `Drink(water)`
- `Transfer` (gifting or trade of any resource)
- `Store` / `Retrieve` from a personal or shared stockpile
- `Craft` (simple tools – optional in the earliest slice)

Shared stockpiles of safe food and water become natural focal points for early public goals and rules (“we will only store plants that everyone can eat”, “no one takes more than X from the common store”).

### Configurability

- Densities and minimum counts of vegetation, animal populations, and fish are part of world generation (see `simulation-and-agents-spec.md`).
- Toxicity probabilities, allergy distributions, nutritional values, and gather rates are all config-driven and can be targeted by incentive effects.

---

## 5. Metrics & Evaluation Framework

Without clear metrics it is impossible to know whether governance emerged or an incentive worked.

### Recommended metric categories

**A. Social / Governance**
- Number of open proposals, acceptance rate, time-to-acceptance
- Number of adopted rules still in force
- Rule compliance rate (when mechanical or social enforcement exists)
- Gini coefficient of proposal authorship and of support
- Presence and stability of specialised roles (measured by action histograms)

**B. Relationship graph**
- Average trust / affinity
- Graph density, clustering coefficient, modularity (polarisation)
- Number of reciprocal high-trust pairs

**C. Consumption & Ecology (explicitly requested)**
- **Per-agent**:
  - Total edible vegetation consumed
  - Total animal life consumed
  - Total fish consumed
  - Total toxic / allergenic items consumed (and resulting illness events)
- **World-level summary**:
  - Aggregate consumption of vegetation, animals, and fish across all agents
  - Remaining standing biomass / population estimates (if tracked)
  - Frequency of toxic consumption events
- These numbers are first-class time series so you can see how diet composition changes under different incentives or norms.

**D. Economic / Survival**
- Population (if death is enabled)
- Total and per-capita resource stocks (including safe vs unsafe food)
- Inequality (Gini of inventory value or of caloric intake)
- Fraction of time spent on cooperative vs selfish actions

**E. Cognitive / Memory**
- Average memory fill ratio
- Fraction of memories that are social vs environmental
- Frequency of reflection events

**F. Incentive-specific**
- Pre/post comparison of any of the above when an incentive is injected at a checkpoint

### Implementation

- Metrics are computed by pure functions over the world state + event log at configurable intervals.
- All metrics are written to a time-series log (and can be visualised in the GUI client).
- A short “experiment summary” is emitted at the end of a run or when a checkpoint is taken.

```toml
[metrics]
compute_every_n_ticks = 50
export_csv = true
track_relationship_graph = true
track_proposal_stats = true
track_consumption = true          # per-agent + world totals for vegetation / animal / fish
```

---

## 6. How These Pieces Fit Together

```
Tick starts
  → Observation built from each agent’s effective senses (base ranges × perceptiveness) + public board
  → Relevant memories retrieved
  → LLM (local or frontier) selects a structured action under the current incentives
  → Action executed → world & agents updated (including possible toxic effects)
  → Memories written, relationships updated, consumption metrics updated
  → Event log grows
```

Because observation is limited *and* individual, communication and public proposals become valuable.  
Because memory is capacity-constrained, important social events (including “that berry made me sick”) must compete for retention.  
Because actions are structured, incentives can reliably change the payoff of cooperative or safe-food actions.  
Because consumption of vegetation / animals / fish is tracked per agent and for the whole world, you can see dietary and ecological consequences of norms and incentives.

---

## 7. Suggested Implementation Order for a Vertical Slice

1. Observation model (including per-agent perceptiveness) + edible vegetation / animal / fish / water + basic needs
2. Structured action vocabulary (`Gather`, `Hunt`, `Fish`, `Drink`, …) + deterministic execution + toxicity/allergy checks
3. LLM action-selection call (provider-agnostic: local Spark-class hardware *or* frontier) with tool/JSON output + failure handling
4. Memory write on every interaction + simple eviction
5. Public proposal board + support/oppose
6. One or two incentive effects (resource multiplier + influence delta)
7. Core metrics (proposal stats, per-agent + world consumption of vegetation/animal/fish, basic relationship averages)

This produces a runnable system in which you can already watch agents forage, hunt, fish, talk, propose rules about food safety, and respond to a simple incentive while the whole trajectory remains reproducible on both local and frontier models.

---

## 8. Open Micro-decisions

- Exact numeric defaults for base vision/hearing ranges, perceptiveness distribution, and need decay.
- Whether speech is a free action or consumes the main action slot.
- How much chain-of-thought the LLM is allowed to emit (log-only vs. also stored as memory).
- Death vs. permanent incapacitation when needs hit zero.
- How finely to differentiate plant/animal/fish species versus broader categories for toxicity.
- First concrete set of craftable tools (if any in the earliest slice).

These can be fixed during the first implementation sprint without blocking progress.
