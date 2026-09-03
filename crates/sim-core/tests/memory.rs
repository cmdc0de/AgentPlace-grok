//! M29 local embeddings: retrieve ranking vs hash.

use sim_core::memory::{self, MemoryEntry, MemoryKind};
use sim_core::{ExperimentConfig, Simulation};

fn tiny(seed: u64) -> ExperimentConfig {
    ExperimentConfig::from_toml_str(&format!(
        r#"
master_seed = {seed}
[simulation]
max_ticks = 1000
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#
    ))
    .unwrap()
}

#[test]
fn enable_embeddings_false_same_retrieve_as_today() {
    let mut store = Vec::new();
    for i in 0..5u64 {
        store.push(MemoryEntry {
            tick: i,
            kind: MemoryKind::Observation,
            text: format!("noise {i}"),
            importance: if i == 0 { 90 } else { 10 },
            last_accessed: i,
            ..Default::default()
        });
    }
    let off = memory::retrieve(&store, 3, 100, None);
    assert_eq!(off[0].tick, 0);
    assert_eq!(off.len(), 3);
}

#[test]
fn enable_embeddings_true_state_hash_ignores_vectors() {
    let cfg = tiny(0x29_20);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.config.agents.memory.enable_embeddings = true;
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
    let ch_off = off.config_hash().unwrap();
    let ch_on = on.config_hash().unwrap();
    assert_ne!(ch_off, ch_on, "flag is on ExperimentConfig");
}
