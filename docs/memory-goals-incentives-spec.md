# Memory, Public Goals / Rule Proposals, and Incentive Schedules

> **Current slice:** [`M41-plan.md`](M41-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

**Purpose**: Lock down the three open design questions that most directly determine whether interesting governance can emerge and whether incentive interventions are cleanly testable.

Related documents:
- `simulation-and-agents-spec.md`
- `deterministic-seeding-design.md`
- `simulation-architecture-spec.md`

---

## 1. Memory Representation & Eviction

### Recommended model: Hybrid Structured + Embedding Memory

A pure flat list of text snippets is too weak for long-horizon social reasoning. A pure vector store is hard to inspect and to manipulate with incentives. The sweet spot is a **hybrid**:

```
MemoryStore
├── slots: Vec<MemoryEntry>          # hard capacity limit (configurable)
├── relationship_summaries: Map<AgentId, RelationshipSummary>  # never fully evicted
└── optional embedding index         # for semantic retrieval (can be disabled for pure determinism tests)
```

#### MemoryEntry (core record)

```rust
struct MemoryEntry {
    id: MemoryId,
    tick: u64,
    kind: MemoryKind,               // Observation | Interaction | Reflection | GoalRelated | Norm
    content: String,                // natural-language summary (always present)
    structured: Option<StructuredPayload>,  // optional typed data
    importance: f32,                // 0.0–1.0, used by eviction
    emotional_valence: f32,         // -1.0 … +1.0
    participants: Vec<AgentId>,     // who was involved
    embedding: Option<Vec<f32>>,    // optional, for retrieval
    last_accessed: u64,             // for recency
}
```

`StructuredPayload` examples:
- `Interaction { partner, action, outcome, influence_delta }`
- `ResourceSighting { resource_type, location, quantity }`
- `GoalProgress { goal_id, delta }`
- `NormObservation { rule_text, supporters, opposers }`

#### RelationshipSummary (persistent)

A compact, never-evicted (or very high priority) record per known agent:

```rust
struct RelationshipSummary {
    trust: f32,
    affinity: f32,
    respect: f32,
    fear: f32,
    interaction_count: u32,
    last_interaction_tick: u64,
    notable_events: Vec<MemoryId>,  // pointers to the most important shared memories
}
```

This gives agents a stable “who is this person to me?” even when detailed episodic memories are forgotten.

### Eviction Policy

Configurable, with a recommended default of **Importance × Recency with social protection**:

```
score = importance * recency_decay(tick - last_accessed) * social_multiplier
```

- `social_multiplier` > 1.0 for entries that involve other agents (controlled by `social_memory_bonus` in config).
- Relationship summaries and the single highest-importance memory per known agent are protected.
- When over capacity, lowest-score entries are dropped (or summarised into a reflection if desired).

Alternative policies that should also be supported via config:
- pure FIFO
- lowest importance only
- “summarise then drop” (LLM generates a higher-level reflection that replaces several low-value entries)

### Why this design
- Inspectable (you can dump an agent’s memory and understand it).
- Influence and relationship tracking have a durable home.
- Capacity is a real experimental variable (small memory → more forgetting → different norm stability).
- Optional embeddings give good retrieval without making the core non-deterministic if you turn them off.
- Incentives can later target importance, valence, or protection rules directly.

---

## 2. Public Goals & Rule Proposals

Governance emerges from agents making things public and trying to get others to adopt them. We therefore need a clear, first-class “proposal” act.

### Concepts

| Concept            | Description                                                                 | Visibility      |
|--------------------|-----------------------------------------------------------------------------|-----------------|
| Personal Goal      | Private intention of one agent                                              | Private         |
| Public Goal        | A goal that has been deliberately shared / championed                       | Known to others |
| Rule / Norm Proposal | A proposed constraint or convention (“no one takes more than X”, “we elect a leader on day 10”, …) | Known to others |
| Adopted Rule       | A proposal that has crossed an acceptance threshold                         | Enforced / social fact |

### Proposal Lifecycle

1. **Creation**  
   An agent spends an action (and possibly social capital) to publish a `Proposal`.

   ```rust
   struct Proposal {
       id: ProposalId,
       author: AgentId,
       tick_created: u64,
       kind: ProposalKind,          // PublicGoal | Rule | RoleAssignment | ResourceAllocation | ...
       text: String,                // natural language
       structured: Option<StructuredRule>,  // optional machine-checkable form
       supporters: Map<AgentId, f32>,       // strength of support
       opposers: Map<AgentId, f32>,
       status: ProposalStatus,      // Open | Accepted | Rejected | Expired | Superseded
   }
   ```

2. **Visibility**  
   Once published, the proposal enters a shared “public board” that every agent can observe (subject to communication range / information access if those are modelled).

3. **Support / Opposition**  
   Other agents can explicitly support, oppose, or ignore. The act of supporting/opposing is itself an interaction and therefore writes memory and updates relationships.

4. **Acceptance**  
   Configurable threshold, for example:
   - simple majority of living agents, or
   - weighted by relationship / status, or
   - unanimous among a declared “council”, etc.

   Thresholds themselves can be the subject of later meta-rules.

5. **Adoption & Enforcement**  
   When accepted, the proposal becomes an `AdoptedRule` (or a shared public goal).  
   Enforcement can be:
   - purely social (reputation loss, refusal to cooperate),
   - soft mechanical (incentive schedule multiplies payoffs),
   - or hard mechanical (if the simulation later adds formal sanctions).

6. **Memory**  
   Every agent that saw the proposal receives a memory entry. Supporters and opposers receive stronger, higher-importance entries.

### Config surface

```toml
[proposals]
max_open_proposals_per_agent = 3
default_acceptance_threshold = 0.5      # fraction of population
proposal_lifetime_ticks = 2000          # after which it expires if not accepted
allow_meta_rules = true                 # can agents propose changes to the proposal rules themselves?
```

This design makes “who proposed what, who backed it, and whether it stuck” fully legible in the event log and in agent memories — exactly the data needed to study governance emergence.

---

## 3. Incentive Schedule Format

Incentives are the primary experimental intervention. They must be:
- declarative,
- time-varying,
- able to target any of the agent variables we have defined (memory importance, influence factor, goal priorities, proposal thresholds, resource payoffs, etc.),
- fully recorded so a run can be exactly reproduced or counterfactualised.

**M8 shipping format** (what `sim-cli --incentives` / `--inject` actually parse): [`incentive-schedule-format.md`](incentive-schedule-format.md). One file = one schedule = many `[[incentives]]`. `visibility_modifier` as an *effect type* is still a load error (M12 is the `visibility` *field*). `applies_to = "supporters_of:proposal_N"` is implemented in M14.

### Recommended format (TOML)

```toml
[[incentives]]
id = "early_cooperation_bonus"
description = "Reward agents who support the first public storage proposal"

# When this incentive is active
start_tick = 500
end_tick   = 3000          # optional; omit for permanent

# Scope
applies_to = "all"         # "all" | "archetype:builder" | "agent:17" | "supporters_of:proposal_3"

# Effects (can have several)
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
condition = "has_supported_public_goal"

[[incentives.effects]]
type = "influence_factor_delta"
delta = 0.15               # temporary boost to how much others can influence this agent
# or how much this agent influences others – both directions should be expressible

[[incentives.effects]]
type = "memory_importance_boost"
kind = "Interaction"
multiplier = 1.8           # social memories become harder to forget

[[incentives.effects]]
type = "goal_injection"
goal_text = "Keep the shared storage stocked"
scope = "public"
priority = 0.7

[[incentives.effects]]
type = "proposal_threshold_modifier"
delta = -0.1               # make acceptance easier while this incentive is active
```

### Effect vocabulary (initial set)

| Effect type                    | What it manipulates                          | Typical use |
|--------------------------------|----------------------------------------------|-------------|
| `resource_multiplier`          | Payoff of gathering / consuming a resource   | Economic incentives |
| `influence_factor_delta`       | Social plasticity or persuasive power        | Conformity / leadership experiments |
| `memory_importance_boost`      | Eviction resistance of certain memory kinds  | Make social or norm memories stickier |
| `goal_injection`               | Add or raise priority of a personal/public goal | Direct the population’s attention |
| `proposal_threshold_modifier`  | Ease or harden rule adoption                 | Institutional design experiments |
| `relationship_delta`           | Directly nudge trust/affinity between groups | Polarisation or cohesion studies |
| `visibility_modifier`          | Who can see whose actions or proposals       | Information control |

All effects are logged with the tick they start and end, and the exact parameters, so the event log + config fully describe the intervention.

### Application timing
- Incentives can be present from tick 0 (baseline vs. treated worlds).
- They can be injected at a checkpoint (the classic “what if we had changed incentives at day 17?”).
- Multiple incentives can be active simultaneously; the config order or an explicit priority field resolves conflicts.

---

## 4. How the Three Pieces Work Together

```
Interaction occurs
    → writes MemoryEntry (both agents)
    → updates RelationshipSummary
    → may change personal goal priorities (modulated by influence_factor)

Agent decides to publish a Proposal
    → Proposal appears on public board
    → other agents form support/opposition (itself an interaction → more memory)

Incentive schedule can:
    - make certain memories harder to forget
    - amplify or dampen influence_factor
    - inject public goals
    - change how many supporters a proposal needs
    - alter material payoffs for cooperative vs. selfish actions
```

Because every step is logged and every parameter is in the config, you can:
1. Run a baseline with no special incentives.
2. Load a checkpoint.
3. Activate a new incentive schedule.
4. Observe whether different rules form, whether they stabilise, and how memory and relationships mediate the change.

---

## 5. Implementation Notes (Rust / Bevy)

- `MemoryStore` and `RelationshipSummary` are plain components or resources; easy to inspect and serialise for checkpoints.
- `Proposal` and `AdoptedRule` live in a shared world resource (the “public board”) so every agent can query them.
- Incentive effects are applied by a dedicated system that runs at the beginning of each tick (or on schedule boundaries). The system is pure with respect to the deterministic seed.
- All of the above are serialised into checkpoints, preserving the exact memory state, open proposals, and active incentives.

---

## 6. Remaining Smaller Decisions

- Exact numeric ranges and defaults for importance, valence, and influence.
- Whether rejected proposals stay visible as historical memory or are archived out of the active board.
- Whether agents can propose changes to the proposal rules themselves (`allow_meta_rules`).
- How much structured vs. free-text content the LLM is allowed to put into memory entries (affects both quality and determinism).

---

This design keeps memory inspectable and capacity-constrained, makes public goal and rule formation a first-class, observable process, and gives incentives a rich, declarative vocabulary that can target the exact variables that drive social dynamics. Together they provide the experimental control surface the project needs.
