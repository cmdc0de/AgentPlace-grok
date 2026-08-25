use crate::agent::AgentId;
use crate::config::SeedSpec;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Derive a subsystem seed from the master seed and a label.
/// Matches `deterministic-seeding-design.md`.
pub fn derive_seed(master: u64, label: &str) -> u64 {
    let mut hasher = XxHash64::with_seed(master);
    label.hash(&mut hasher);
    hasher.finish()
}

pub fn rng_from_seed(seed: u64) -> ChaCha20Rng {
    ChaCha20Rng::seed_from_u64(seed)
}

/// Resolve a config seed spec into a concrete u64, recording it in `resolved`.
pub fn resolve_seed(spec: SeedSpec, master: u64, label: &str, master_rng: &mut ChaCha20Rng) -> u64 {
    match spec {
        SeedSpec::Auto => derive_seed(master, label),
        SeedSpec::Explicit(v) => v,
        SeedSpec::Random => {
            use rand::Rng;
            master_rng.random()
        }
    }
}

/// Named ChaCha20 streams keyed in a BTreeMap so iteration order is stable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RngBank {
    pub master_seed: u64,
    /// Label → derived (or resolved) seed, for run metadata.
    pub derived_seeds: BTreeMap<String, u64>,
    streams: BTreeMap<String, ChaCha20Rng>,
}

impl RngBank {
    pub fn new(master_seed: u64) -> Self {
        let mut bank = Self {
            master_seed,
            derived_seeds: BTreeMap::new(),
            streams: BTreeMap::new(),
        };
        // Direct master stream (not hashed) for SeedSpec::Random draws.
        bank.insert_direct("master", master_seed);
        bank.ensure("world");
        bank.ensure("agent_init");
        bank.ensure("turn_order");
        bank.ensure("llm");
        bank.ensure("event");
        bank.ensure("incentive");
        bank
    }

    fn insert_direct(&mut self, label: &str, seed: u64) {
        self.derived_seeds.insert(label.to_string(), seed);
        self.streams.insert(label.to_string(), rng_from_seed(seed));
    }

    pub fn ensure(&mut self, label: &str) -> &mut ChaCha20Rng {
        if !self.streams.contains_key(label) {
            let seed = derive_seed(self.master_seed, label);
            self.derived_seeds.insert(label.to_string(), seed);
            self.streams.insert(label.to_string(), rng_from_seed(seed));
        }
        self.streams.get_mut(label).expect("just inserted")
    }

    pub fn set_resolved(&mut self, label: &str, seed: u64) {
        self.derived_seeds.insert(label.to_string(), seed);
        self.streams.insert(label.to_string(), rng_from_seed(seed));
    }

    pub fn stream(&mut self, label: &str) -> &mut ChaCha20Rng {
        self.ensure(label)
    }

    pub fn agent_stream(&mut self, id: AgentId) -> &mut ChaCha20Rng {
        let key = format!("agent_{}", id.0);
        self.ensure(&key)
    }

    pub fn fingerprint(&self) -> Vec<(String, u64, u64, u64)> {
        // Clone each stream and take two u64s so the hash depends on full RNG position.
        self.streams
            .iter()
            .map(|(k, rng)| {
                let mut clone = rng.clone();
                use rand::Rng;
                let a: u64 = clone.random();
                let b: u64 = clone.random();
                (
                    k.clone(),
                    a,
                    b,
                    self.derived_seeds.get(k).copied().unwrap_or(0),
                )
            })
            .collect()
    }
}
