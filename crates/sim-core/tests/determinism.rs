use sim_core::{ExperimentConfig, Simulation};

fn tiny_config(master_seed: u64, agent_count: u32) -> ExperimentConfig {
    let toml = format!(
        r#"
master_seed = {master_seed}

[simulation]
max_ticks = 1000

[world]
seed = "auto"
width = 32
height = 32
max_height = 8

[world.terrain]
octaves = 3

[agents]
count = {agent_count}
spawn_mode = "scattered"
"#
    );
    ExperimentConfig::from_toml_str(&toml).expect("valid test config")
}

#[test]
fn same_seed_same_hash() {
    let cfg = tiny_config(0x00C0_FFEE, 8);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(50);
    b.run_ticks(50);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.tick, 50);
}

#[test]
fn different_seed_different_hash() {
    let mut a = Simulation::new(tiny_config(1, 8)).unwrap();
    let mut b = Simulation::new(tiny_config(2, 8)).unwrap();
    a.run_ticks(50);
    b.run_ticks(50);
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn world_hash_independent_of_agent_count() {
    let a = Simulation::new(tiny_config(42, 4)).unwrap();
    let b = Simulation::new(tiny_config(42, 12)).unwrap();
    assert_eq!(a.world_hash(), b.world_hash());
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn config_rejects_tiny_map() {
    let toml = r#"
master_seed = 1
[world]
width = 8
height = 8
"#;
    assert!(ExperimentConfig::from_toml_str(toml).is_err());
}
