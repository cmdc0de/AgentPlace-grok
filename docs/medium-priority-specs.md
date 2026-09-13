# Medium-Priority Specifications
## Communication • Conflict & Sanctions • Checkpoints • Error Handling • Testing

> **Current slice:** [`M51-plan.md`](M51-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

Related documents:
- `simulation-architecture-spec.md`
- `simulation-and-agents-spec.md`
- `memory-goals-incentives-spec.md`
- `decision-observation-llm-economy-metrics-spec.md`
- `deterministic-seeding-design.md`

---

## 1. Communication Model

Communication is the main channel through which norms, warnings (e.g. “those berries are toxic”), and proposals spread. It must be limited enough to make information valuable, yet rich enough for governance to emerge.

### Design principles
- **Speaking is a free secondary action** — an agent may both take a primary action (move, gather, hunt, etc.) *and* speak in the same tick.
- **Hard length limit** on every utterance to prevent filibustering / spam. Messages that exceed the limit are truncated or rejected.
- Range is limited and can be modified by personality / perceptiveness / environment.
- Messages are durable records (they create memories on both sides and appear in the global event log).
- The public proposal board is a special, always-visible (or high-visibility) channel; ordinary speech is not.

### Message types

| Type              | Description                                      | Typical cost / range          |
|-------------------|--------------------------------------------------|-------------------------------|
| Directed speech   | Private or small-group utterance to specific agents | Normal hearing range         |
| Broadcast / shout | Heard by everyone inside a larger radius         | Higher energy cost, larger range |
| Public board post | Formal proposal or announcement                  | Instant, global (or faction-limited) |
| Gesture / signal  | Non-verbal (point, wave, threat display)         | Vision range only            |

### Config surface

```toml
[communication]
speech_is_free_action     = true    # secondary action; does not consume the primary action slot
base_speech_range         = 18.0    # modified by perceptiveness of listeners
shout_range_multiplier    = 1.8
shout_energy_cost         = 5.0
max_message_length        = 200     # hard character/token limit – anti-filibuster
allow_overhearing         = true    # agents slightly outside target list may still hear
public_board_visible_to   = "all"   # "all" | "same_faction" | "in_range"
```

### Effects of communication
- Creates `Interaction` memory entries for speaker and all recipients (and overhearers if enabled).
- Can update relationship scores.
- Can transfer knowledge (resource locations, toxicity, open proposals, etc.).
- Can be the vehicle for support/opposition to proposals.
- Because length is capped, agents must be concise; long arguments have to be spread across multiple ticks or formalised as proposals.

### Open micro-decisions
- Whether lies are possible and how (or if) they are detected.
- Whether language is a single shared tongue or agents can develop dialects / private codes (interesting but complex; recommend postponing).

---

## 2. Conflict, Enforcement & Sanctions

For early versions we recommend **social sanctions first**, with optional light mechanical enforcement later. Full combat systems can be added once the social layer is solid.

**Architectural requirement**: The code must be structured so that moving from v1 (social + soft mechanical) to v2 (physical conflict, injury, death) is an *extension*, not a rewrite. Action vocabulary, agent state, and event types should already reserve the necessary hooks.

### Levels of enforcement

| Level | Mechanism                              | When to introduce          | Notes |
|-------|----------------------------------------|----------------------------|-------|
| 0     | None (pure talk)                       | Debug only                 | |
| 1     | Social only (reputation, refusal to cooperate, exclusion from shared stockpiles) | **v1 default** | Sufficient for many norm experiments |
| 2     | Soft mechanical (resource penalties, movement restrictions imposed by adopted rules) | v1 / v1.5 | Still no HP combat |
| 3     | Physical conflict (attack, flee, injury, death) | **v2**                   | Designed for from the start |

### Recommended v1 model (Level 1 + light Level 2)

- Agents can **refuse interactions** with low-trust or rule-breaking agents.
- Adopted rules can carry **soft mechanical consequences** declared in the proposal itself (e.g. “anyone who takes more than X from the common store loses access for 50 ticks”).
- Reputation / relationship scores act as the main enforcement currency.
- A simple `ExcludeFrom(stockpile)` or `RefuseTradeWith(agent)` action is enough to make rules bite.

### Forward-compatible design for v2

Even while `physical_conflict_enabled = false`:

- Keep `Attack` and `Flee` in the action vocabulary (they simply become illegal / no-ops when the flag is off).
- Agent state already contains (or has reserved fields for) `health` / `injury` / `incapacitated`.
- Event log already has slots for `Injury`, `Death`, `Combat` events.
- Sanctions system is data-driven so new sanction types can be registered without touching core loops.

```toml
[conflict]
enabled = false                 # v1 = false; v2 flips this
attack_action_cost = 1
injury_reduces_energy = true
death_enabled = false           # start with incapacitation; permanent death can be a further flag
```

### Config surface for sanctions

```toml
[sanctions]
social_refusal_enabled     = true
rule_can_impose_exclusion  = true
rule_can_impose_resource_penalty = true
physical_conflict_enabled  = false   # must remain a clean toggle for v2
```

---

## 3. Checkpoint / Serialization Format

Checkpoints are the foundation of time-travel experiments (“load day 17 and change the incentive schedule”).

### Requirements
- Bit-reproducible restoration on the same platform / binary.
- Versioned so older checkpoints can still be loaded (or cleanly rejected).
- Contains everything needed to continue the run: world, agents (including full memory & relationship state), RNGs, active incentives, open proposals, metrics counters, and the config hash.
- **Human-readable Markdown summaries** are generated alongside every binary checkpoint:
  - A short overall snapshot summary (tick, population, key metrics, active incentives, open proposals).
  - A per-agent summary (persona, current goals, notable recent memories, inventory highlights, relationship snapshot).

### Recommended contents

```rust
struct Checkpoint {
    format_version: u32,
    tick: u64,
    master_seed: u64,
    config_hash: [u8; 32],          // hash of the full ExperimentConfig
    world: WorldState,
    agents: Vec<AgentState>,        // includes MemoryStore, RelationshipSummary, goals, inventory, needs, plan, …
    public_board: PublicBoard,
    active_incentives: Vec<Incentive>,
    rng_states: RngStates,          // all hierarchical streams
    metrics_accumulators: MetricsState,
    event_log_offset: u64,          // or embedded tail of the log
}
```

### Human-readable companions

For every `{experiment_id}_tick_{tick}.ckpt` the system also writes:

- `{experiment_id}_tick_{tick}_summary.md` – overall state of the world and simulation
- `{experiment_id}_tick_{tick}_agents.md` – one section per agent with a concise, readable status report

These Markdown files are **not** required for restoration; they exist for inspection, debugging, sharing, and later analysis. They can be regenerated from a checkpoint if missing.

### Serialization choices
- **Primary**: `bincode` or `postcard` for speed and compactness (Rust-native).
- **Secondary / debug**: JSON or MessagePack for human inspection of small checkpoints.
- Optional compression (zstd) for large runs.
- Binary checkpoint + Markdown summaries + small JSON sidecar (tick, config hash, timestamp, git commit, model name).

### Compatibility policy
- `format_version` is incremented on any breaking change.
- Loader refuses checkpoints with unknown or too-old versions rather than risking silent corruption.
- A migration tool can be written later if needed.

### Config surface

```toml
[checkpoint]
auto_interval_ticks = 500
keep_last_n         = 20
directory           = "checkpoints"
compression         = "zstd"
write_markdown_summaries = true
```

---

## 4. Error Handling & Robustness

The simulation must degrade gracefully; a single bad LLM reply or transient network glitch must not corrupt a multi-hour experiment.

### Categories of failure

| Failure                        | Handling strategy                                                                 |
|--------------------------------|-----------------------------------------------------------------------------------|
| LLM **timeout**                | Treat as “agent did nothing this step” (equivalent to `Wait` / no-op). Log `LlmTimeout`. No retries on pure timeout. |
| LLM malformed output           | Retry (same seed, lower temperature) up to `max_retries` → then fall back to `Wait` → log `LlmFailure` |
| LLM provider unreachable       | Same as malformed; optional circuit-breaker that temporarily switches to a rule-based policy |
| Illegal action requested       | Reject, log, treat as `Wait`; do not crash                                        |
| Client disconnect              | Core continues; client can re-attach from latest checkpoint or live stream        |
| Checkpoint write failure       | Retry; if persistent, log loudly and continue (never halt the sim for I/O)        |
| Desync between core & client   | Client is always reconcilable from core state; core is authoritative              |
| Panic in a non-critical system | Catch where possible, log, skip the system for that tick                          |

**Timeout policy (explicit)**: If the LLM call simply exceeds `timeout_ms`, the agent is treated as having taken no action that tick. This keeps the simulation moving and avoids cascading delays.

### Logging of failures
Every failure becomes a first-class event in the global log and, where relevant, a low-importance memory for the affected agent. This lets you later analyse how often agents were “confused” or how robust a particular model was.

### Config surface

```toml
[error_handling]
llm_max_retries          = 2
llm_timeout_action       = "Wait"    # pure timeout → agent does nothing this step
llm_fallback_action      = "Wait"    # after retries on malformed output
checkpoint_write_retries = 3
continue_on_client_loss  = true
```

---

## 5. Testing Strategy

Because reproducibility is a core requirement, testing must be unusually rigorous.

### Test layers

**A. Unit tests**
- Seed derivation and hierarchical RNGs
- Memory eviction under different policies
- Proposal acceptance logic
- Toxicity / allergy application
- Incentive effect application
- Observation range calculations (including perceptiveness)

**B. Determinism regression tests**
- Run the same config + seed for N ticks twice → hash of final state + event log must be identical.
- Load a checkpoint and continue → must match the original continuous run.
- These tests should run on CI for every commit that touches core logic.

**C. Scenario / integration tests**
- Small scripted worlds that force specific situations:
  - Two agents, one toxic plant, communication of the danger
  - Resource scarcity that should encourage a sharing proposal
  - Injection of an incentive at a known tick and verification of expected metric shifts
- Can run with a mocked LLM (pre-recorded replies) so they stay fast and deterministic.

**D. Property / fuzz tests** (optional but valuable)
- Random valid configs + seeds → simulation never panics and always produces a valid checkpoint.

**E. Manual / visual tests**
- GUI client attach / detach
- Agent POV matches the observation the agent actually received
- Timeline scrubbing and counterfactual incentive injection

### CI recommendations
- Matrix: Linux (Debian/Ubuntu), Windows, macOS.
- Fast determinism suite on every PR.
- Longer scenario suite nightly or on release branches.
- Optional GPU runner for local-model smoke tests.

### Config / tooling
- A `sim-test` or `cargo test` feature flag that swaps the real LLM client for a deterministic mock.
- Golden-file checkpoints for a few canonical scenarios stored in the repo.

---

## 6. Suggested Priority Order Among Medium Items

For a working vertical slice you already have the high-priority pieces. Among these medium items, the practical order is:

1. **Checkpoint / Serialization** – needed as soon as you want any time-travel or long runs. (Include Markdown summaries from the start.)
2. **Error Handling** – especially LLM timeout = “do nothing this step”; otherwise early experiments will be fragile.
3. **Communication Model** – free secondary action + hard length limit; required for interesting social dynamics and toxicity warnings.
4. **Testing Strategy** – start with determinism tests early; expand scenario tests as features land.
5. **Conflict & Sanctions** – social refusal + rule-based exclusion is enough for a long time; keep the code structured so physical combat (v2) is an extension, not a rewrite.

---

## 7. When to Wire Up the 3D Client / Viewer

**Recommendation: very early — as soon as agents have positions and the world has basic geometry.**

You do **not** need the full decision loop, LLM, memory, or proposals before you can usefully look at something in 3D.

### Practical minimal milestone (“first picture”)

1. Simulation core can advance ticks and maintain agent positions + simple world (terrain height / resource markers).
2. A headless or in-process runner exists.
3. A minimal Bevy viewer can:
   - Connect (in-process is fine for the first version)
   - Spawn a camera
   - Draw the ground / simple terrain
   - Draw agent markers (coloured capsules or billboards)
   - Optionally follow one agent

At that point you already have visual confirmation that the world and agents exist and are moving. Everything else (observation cones, inventory pop-ups, proposal board UI, imgui panels, agent POV, etc.) can be layered on incrementally.

### Suggested placement in the overall sequence

```
High-priority vertical slice (logic)
  → minimal 3D viewer (“first picture”)     ← do this as soon as positions exist
  → checkpoints + Markdown summaries
  → LLM + decision loop
  → communication + proposals
  → richer GUI (imgui, agent POV, metrics)
  → full attachable client protocol
```

Seeing agents move in 3D early is high-motivation and catches coordinate / scale / transform bugs long before they become expensive. Treat the first 3D view as a debugging and morale tool, not as a polished feature.

---

## 8. Open Questions Still Deferred

- Full dialect / private-language emergence
- Complex multi-step crafting trees
- Seasonal / climate cycles affecting resource availability
- Formal legal systems beyond simple adopted rules
- Multi-human collaborative control of different agents

These remain future expansion areas and do not block the medium-priority work above.
