# Deterministic Seeding Design for Multi-Agent Simulation

> **Current slice:** [`M34-plan.md`](M34-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

**Purpose**: Enable fully reproducible experimental runs so that baseline behavior (organic rule / government formation) can be cleanly compared against runs that inject different population-level incentives.

This design prioritizes bit-level reproducibility, clean isolation of the incentive variable, mid-run branching via checkpoints, and ease of auditing.

---

## 1. Hierarchical / Structured Seeding

Use a single **master seed** that deterministically derives every other source of randomness. This provides global reproducibility while still allowing isolated re-seeding of subsystems if needed.

```
MasterSeed (u64 or [u8; 32])
├── WorldSeed          → map generation, resource placement, environmental events
├── AgentInitSeed      → initial positions, starting inventories, personality noise (if any)
├── TurnOrderSeed      → permutation of agent execution order each tick / day
├── LLMSeedBase        → base for per-agent, per-call LLM sampling seeds
├── EventSeed          → rare world events, environmental triggers
└── IncentiveSeed      → (optional) any stochastic elements inside incentive mechanisms
```

### Derivation Method (Rust)

```rust
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64; // or siphasher / ahash

fn derive_seed(master: u64, label: &str) -> u64 {
    let mut hasher = XxHash64::with_seed(master);
    label.hash(&mut hasher);
    hasher.finish()
}

// Example usage
let master = 0xDEADBEEFCAFEBABE_u64;
let world_seed   = derive_seed(master, "world");
let agent_seed   = derive_seed(master, "agent_init");
let turn_seed    = derive_seed(master, "turn_order");
let llm_base     = derive_seed(master, "llm");
```

Per-agent seeds can be derived as:
```rust
derive_seed(master, &format!("agent_{}", agent_id))
```

**Recommended RNG**: `ChaCha20Rng` (from `rand_chacha`). It is fast, has excellent statistical properties, and is fully seedable.

---

## 2. Controlling Every Source of Non-Determinism

| Source                    | How to make deterministic                                                                 | Notes |
|---------------------------|-------------------------------------------------------------------------------------------|-------|
| Agent turn order          | Derive a permutation from `TurnOrderSeed` each tick or each day                           | Never rely on `HashMap` iteration order or OS thread scheduling |
| World generation          | All procedural content from `WorldSeed`                                                   | Fixed map size + seed → identical layout |
| Resource respawn / events | Sample from `EventSeed` or a dedicated sub-stream                                         | |
| Agent internal randomness | Each agent owns its own `ChaCha20Rng` seeded from master + agent_id                       | |
| LLM sampling              | Pass an explicit `seed` parameter on every call                                           | Critical – most local backends and many cloud APIs support this |
| Floating-point / ordering | Prefer sorted vectors or `BTreeMap` when iteration order affects decisions                | Avoid unordered collections for anything decision-relevant |
| Parallelism               | Single-threaded for experiments, or deterministic work distribution (e.g. agent_id % n) + barriers at tick boundaries | |

### LLM Call Seeding

Derive a unique seed for every LLM request:

```rust
let call_seed = derive_seed(
    llm_base,
    &format!("tick_{}_agent_{}_call_{}", tick, agent_id, call_counter)
);
```

- Prefer temperature = 0.0 (or very low) during comparative experimental runs.
- When a provider does not support seeding, fall back to temperature = 0 + fixed top_p / top_k and document residual non-determinism, or switch providers for formal experiments.

---

## 3. Configuration & Run Metadata

Every run must be completely described by a single serializable config object:

```rust
#[derive(Serialize, Deserialize, Clone)]
struct ExperimentConfig {
    master_seed: u64,
    num_agents: usize,
    world_params: WorldParams,
    incentive_schedule: IncentiveSchedule,  // the primary experimental variable
    llm_model: String,
    llm_temperature: f32,                   // usually 0.0 for reproducibility
    max_ticks: u64,
    // ... any other parameter that can affect outcomes
}
```

At the start of every run, persist:

- Full config (JSON / TOML / YAML)
- Master seed
- Git commit hash (or binary hash)
- Timestamp
- Optionally the full set of derived seeds (useful for debugging)

This makes any past run exactly reproducible months later.

---

## 4. State Snapshots for Branching Experiments

Support full state checkpoints so a baseline run can be branched:

- Serialize the entire simulation state at chosen ticks:
  - World state
  - Every agent’s memory, relationships, inventory, current plan, internal RNG state
- A treatment run loads a baseline checkpoint at tick `T`, applies a new `IncentiveSchedule`, and continues with the same master seed (or a deterministically derived continuation seed).

Storing the RNG states explicitly is required for bit-identical resumption.

---

## 5. Practical Rust Skeleton

```rust
use rand_chacha::ChaCha20Rng;
use rand::SeedableRng;
use std::collections::HashMap;

struct Simulation {
    master_seed: u64,
    // Structured collection of RNG streams
    rngs: HashMap<String, ChaCha20Rng>,
    // ... world, agents, etc.
}

impl Simulation {
    fn new(config: ExperimentConfig) -> Self {
        let master = config.master_seed;
        let mut rngs = HashMap::new();
        // Pre-derive or lazily create streams
        rngs.insert("world".into(), ChaCha20Rng::seed_from_u64(derive_seed(master, "world")));
        rngs.insert("turn_order".into(), ChaCha20Rng::seed_from_u64(derive_seed(master, "turn_order")));
        // ...
        Self { master_seed: master, rngs, /* ... */ }
    }

    fn agent_rng(&mut self, agent_id: u64) -> &mut ChaCha20Rng {
        let key = format!("agent_{}", agent_id);
        self.rngs.entry(key.clone()).or_insert_with(|| {
            ChaCha20Rng::seed_from_u64(derive_seed(self.master_seed, &key))
        })
    }

    fn next_turn_order(&mut self, agent_ids: &[AgentId]) -> Vec<AgentId> {
        let mut order = agent_ids.to_vec();
        let rng = self.rngs.get_mut("turn_order").unwrap();
        // Use SliceRandom::shuffle
        order.shuffle(rng);
        order
    }
}
```

---

## 6. Testing Reproducibility

Add regression tests:

1. Run simulation A with seed `S` for `N` ticks → record cryptographic hash of final state + key metrics + full transcript hash.
2. Run simulation B with identical config + seed `S` → assert identical hashes.
3. Change only the incentive schedule → assert the hashes diverge in expected ways.
4. Load a checkpoint from run A and continue → assert the continuation matches the original continuous run.

---

## 7. Additional Recommendations

- Keep temperature low (0.0–0.3) for all comparative experimental runs. Higher temperature is acceptable for pure exploration.
- Optionally log the exact prompt + seed + raw LLM response for every decision (valuable when debugging divergence).
- Version the checkpoint serialization format so old baselines remain loadable.
- Prefer deterministic data structures (`BTreeMap`, sorted `Vec`) anywhere iteration order can influence agent decisions or logging.

---

## Summary of Benefits

- Bit-reproducible baselines
- Clean isolation of the incentive variable
- Ability to branch mid-simulation from any checkpoint
- Easy auditing, sharing, and long-term reproducibility of experimental results

This design is intentionally minimal yet complete enough to support rigorous A/B testing of governance emergence under different incentive regimes.
