# Simulation Parameters & Agent Model Specification

> **Current slice:** [`M62-plan.md`](M62-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

**Purpose**: Define the configurable surface of the simulation and the internal structure of agents so that experiments (baseline governance emergence vs. incentive interventions) are fully controllable, reproducible, and inspectable.

Related documents:
- `deterministic-seeding-design.md`
- `simulation-architecture-spec.md`

---

## 1. Configuration Philosophy

Everything that can affect outcomes must be expressible in a single configuration file (or a small set of layered config files).

- Preferred format: **TOML** (native Rust support via `serde` + `toml`, human-readable, good for nested tables).
- YAML is an acceptable alternative if preferred.
- The config is loaded once at startup (or at checkpoint restore) and becomes part of the experiment record.
- Every numeric parameter that has a meaningful lower bound must declare a **minimum** (and preferably a maximum / default). The loader rejects values outside the allowed range.
- The master seed (or an explicit “random” flag) is also part of the config so that a complete experiment is self-describing.

A minimal experiment description is therefore:

```
config.toml + master_seed (+ optional checkpoint)
```

---

## 2. Top-Level Simulation Parameters

```toml
[simulation]
# Fixed-duration step used by the deterministic tick loop.
# Minimum: 0.05 s (or 1 tick if using pure discrete mode)
step_duration_secs = 1.0

# Maximum number of ticks before the run stops (0 = unlimited)
max_ticks = 100000

# Whether the simulation pauses when no clients are attached
pause_when_empty = false

# Logging verbosity for the core event log
log_level = "info"          # "trace" | "debug" | "info" | "warn" | "error"
```

### Step time
- The core advances in discrete ticks.
- `step_duration_secs` is the simulated time advanced per tick. It can be used by needs systems (hunger, fatigue, resource regeneration) and by any real-time visualisation.
- For pure discrete experiments the value can be treated as “1 abstract tick”.

---

## 3. World Generation

World generation is driven by a single **world seed** (derived from the master seed or supplied explicitly). The same seed must always produce the identical terrain, resource distribution, vegetation, and water layout.

```toml
[world]
# If set, overrides the derived world seed from the master seed.
# If omitted or set to "random", a new seed is drawn from the master RNG.
seed = "auto"               # "auto" | u64 | "random"

# Map dimensions (in world units or tiles)
width  = 256
height = 256
# Minimum: 32 x 32

# Vertical scale / number of height levels (if using heightmaps or voxels)
max_height = 64
# Minimum: 8

[world.terrain]
# Noise / procedural parameters (exact algorithm is an implementation detail)
octaves          = 4
persistence      = 0.5
lacunarity       = 2.0
# All have sensible minima (e.g. octaves >= 1)

[world.resources]
# Global density multipliers (0.0 = none, 1.0 = default rich world)
mineral_density     = 1.0     # min 0.0
vegetation_density  = 1.0     # min 0.0
water_coverage      = 0.25    # fraction of map, min 0.0, max 1.0

# Per-resource minimum guarantees (even in sparse worlds)
min_mineral_nodes   = 10
min_fresh_water     = 5
min_vegetation_patches = 20

[world.climate]               # optional future expansion
temperature_mean    = 15.0
rainfall            = 1.0
```

### Guarantees
- The world seed alone determines the layout of terrain, minerals, vegetation, and water.
- Density multipliers and explicit minima interact: the generator first places the required minimum nodes, then fills the rest according to the density parameters and the seed.
- Changing any of these values (or the seed) produces a different world; the combination is recorded in the experiment metadata.

---

## 4. Agent Population & Instantiation

```toml
[agents]
count = 24                    # min 2 (need interaction), practical upper bound depends on hardware

# How initial positions are chosen
spawn_mode = "scattered"      # "scattered" | "clustered" | "fixed_list"
spawn_seed = "auto"           # derived or explicit

# Default memory capacity for every agent (can be overridden per-agent or per-personality)
default_memory_capacity = 128 # number of memory slots / embeddings, min 8

# Whether agents start with any pre-loaded memories or goals
start_with_basic_needs = true
```

Agents can also be defined individually or by sampling from personality distributions (see below).

---

## 5. Agent Internal Model

Each agent is a bundle of components. All of the following are configurable either globally, by personality archetype, or per individual agent.

### 5.1 Identity & Personality

```toml
[[agents.archetypes]]
name = "explorer"
weight = 1.0                  # relative frequency when sampling the population

[agents.archetypes.personality]
# Example using a simple Big-Five style or custom facets.
# All values in [0.0, 1.0] unless noted.
openness          = 0.8
conscientiousness = 0.4
extraversion      = 0.7
agreeableness     = 0.5
neuroticism       = 0.3

# Free-form traits that the LLM prompt and decision logic can use
traits = ["curious", "risk-tolerant", "speaks bluntly"]

# Optional numeric drives
curiosity_drive   = 0.9
social_drive      = 0.6
resource_drive    = 0.4
```

Personality influences:
- Prompt construction for the LLM
- Baseline action preferences
- How strongly the agent is affected by social influence
- Speech style and willingness to propose / accept rules

### 5.2 Memory

```toml
[agents.memory]
# Capacity is the maximum number of discrete memory entries (or total token budget)
capacity = 128                # min 8

# How memories are prioritised when capacity is exceeded
eviction_policy = "importance_and_recency"  # or "fifo", "lowest_importance", ...

# Whether social interactions receive a bonus to importance
social_memory_bonus = 1.5

# Whether the agent keeps a separate “relationship summary” that is never fully evicted
persistent_relationships = true
```

**Memory entry types** (conceptual):
- Observation (world fact, resource location, event)
- Interaction (who, what was said/done, outcome, emotional valence)
- Reflection / high-level insight (generated periodically)
- Goal-related (progress, blockage, commitment)

Interactions between agents always generate at least one memory entry on both sides (subject to capacity).

### 5.3 Goals

Goals are first-class and come in two scopes:

| Scope        | Visibility     | Example                                      | Can be modified by incentives? |
|--------------|----------------|----------------------------------------------|--------------------------------|
| **Personal** | Private        | “Accumulate 50 iron”, “Map the northern ridge” | Yes                           |
| **Public**   | Known to others / proposable | “Establish a shared storage”, “Elect a leader” | Yes (core experimental lever) |

```toml
[agents.goals]
# Maximum number of active personal goals
max_personal_goals = 5        # min 1

# Maximum number of public goals an agent will simultaneously champion
max_public_goals   = 3        # min 0

# Whether agents can adopt another agent’s public goal
can_adopt_public_goals = true
```

Goals have:
- Description (natural language + optional structured fields)
- Priority / urgency
- Progress metric (if measurable)
- Source (intrinsic, adopted, imposed by incentive schedule, etc.)

### 5.4 Social Influence & Interaction Memory

One of the key requirements is that agents can influence one another and retain memory of those interactions.

```toml
[agents.social]
# How strongly another agent’s opinion / proposal can shift this agent’s utility or willingness
base_influence_factor = 0.3    # 0.0 = immune, 1.0 = fully plastic

# Decay of influence over time (per tick or per day)
influence_decay = 0.01

# Whether repeated interactions with the same agent create a lasting relationship record
track_relationships = true

# Relationship dimensions that are updated by interactions
relationship_dimensions = ["trust", "affinity", "respect", "fear"]
```

**Interaction effects**
- Every directed interaction (conversation, resource transfer, joint action, conflict, proposal, vote, etc.) produces:
  - A memory entry on both participants
  - An update to the relationship vector (if tracking is enabled)
  - A possible shift in goal priorities or willingness to accept rules (modulated by personality + influence_factor)
- The magnitude of influence can itself be affected by the current incentive schedule (e.g. an incentive that rewards conformity increases effective influence).

### 5.5 Other Configurable Agent Variables

- Energy / hunger / fatigue (if needs are modelled)
- Inventory capacity
- Vision range / observation radius (affects “what the agent sees” in the GUI)
- Communication range or channel access
- Risk tolerance, time preference, fairness preference, etc. (can be derived from personality or set explicitly)

All of the above appear in the config with documented minima and defaults.

---

## 6. Interaction & Influence Pipeline (Conceptual)

1. Agent A decides to interact with Agent B (or broadcasts).
2. The interaction is resolved (dialogue via LLM, resource transfer, joint work, etc.).
3. Both agents receive a structured interaction record.
4. Relationship vectors are updated.
5. Each agent may revise personal/public goals or internal valuations (strength controlled by personality + `base_influence_factor` + any active incentives).
6. The interaction and any resulting goal changes are written to the global event log and to each agent’s memory (subject to capacity).

This pipeline is the substrate on which norms, leadership, and governance structures can emerge, and on which incentive interventions can act.

---

## 7. Example Minimal Config Sketch

```toml
[simulation]
step_duration_secs = 1.0
max_ticks = 50000

[world]
seed = "auto"
width = 128
height = 128

[world.resources]
mineral_density = 0.8
vegetation_density = 1.0
water_coverage = 0.2
min_mineral_nodes = 8
min_fresh_water = 4

[agents]
count = 16
default_memory_capacity = 64

[[agents.archetypes]]
name = "builder"
weight = 1.0
[agents.archetypes.personality]
conscientiousness = 0.8
agreeableness = 0.6
traits = ["cooperative", "planning-oriented"]

[[agents.archetypes]]
name = "scout"
weight = 0.7
[agents.archetypes.personality]
openness = 0.9
extraversion = 0.7
traits = ["curious", "independent"]
```

---

## 8. Design Implications for Reproducibility & Experiments

- Changing any value in the config (including memory capacity, influence factor, personality distribution, or resource minima) produces a different experimental condition.
- The full config is hashed and stored with every run and every checkpoint.
- Incentive schedules (defined elsewhere) can reference agent variables (e.g. “multiply social influence by 1.5” or “add a public goal template to every agent”).
- Because memory capacity and eviction policy are configurable, one can test whether richer or poorer memory changes the stability of emerging norms.

---

## 9. Open Questions / Next Decisions

- Exact memory representation (flat list of embeddings vs. structured slots vs. hierarchical).
- How public goals become visible and whether there is a formal “proposal” act.
- Whether influence is symmetric or can be asymmetric (status, charisma, etc.).
- Concrete list of needs / resources that drive the earliest agent behaviours.
- Whether personality is fixed at birth or can slowly drift through experience.

---

This specification makes the simulation’s world, timing, population, memory, personality, goals, and social influence fully declarative. Combined with the deterministic seeding and checkpoint system, it provides the control surface required for systematic experiments on governance emergence under different incentive regimes.
