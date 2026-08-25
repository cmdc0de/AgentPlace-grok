use sim_core::incentive::IncentiveSchedule;
use sim_core::{ExperimentConfig, Simulation};

fn tiny(seed: u64) -> ExperimentConfig {
    ExperimentConfig::from_toml_str(&format!(
        r#"
master_seed = {seed}
[simulation]
max_ticks = 10000
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 4
"#
    ))
    .unwrap()
}

const GOAL_INJECT: &str = r#"
[[incentives]]
id = "coop_goal"
description = "inject a cooperation goal"
start_tick = 0
applies_to = "all"
[[incentives.effects]]
type = "goal_injection"
goal_text = "keep the shared storage stocked"
scope = "personal"
priority = 0.9
"#;

#[test]
fn same_seed_no_schedule_same_hash() {
    let cfg = tiny(0x8001);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(40);
    b.run_ticks(40);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn same_schedule_twice_same_hash_differs_from_baseline() {
    let cfg = tiny(0x8002);
    let mut base = Simulation::new(cfg.clone()).unwrap();
    let mut t1 = Simulation::new(cfg.clone()).unwrap();
    let mut t2 = Simulation::new(cfg).unwrap();
    t1.inject_schedule_toml(GOAL_INJECT).unwrap();
    t2.inject_schedule_toml(GOAL_INJECT).unwrap();
    base.run_ticks(20);
    t1.run_ticks(20);
    t2.run_ticks(20);
    assert_eq!(t1.state_hash(), t2.state_hash());
    assert_ne!(base.state_hash(), t1.state_hash());
    assert!(t1.incentive_active.contains("coop_goal"));
}

#[test]
fn load_without_inject_matches_uninterrupted() {
    let cfg = tiny(0x8003);
    let mut full = Simulation::new(cfg.clone()).unwrap();
    full.run_ticks(40);
    let mut mid = Simulation::new(cfg).unwrap();
    mid.run_ticks(20);
    let bytes = mid.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.run_ticks(20);
    assert_eq!(full.state_hash(), loaded.state_hash());
}

#[test]
fn load_with_inject_diverges() {
    let cfg = tiny(0x8004);
    let mut full = Simulation::new(cfg.clone()).unwrap();
    full.run_ticks(40);
    let mut mid = Simulation::new(cfg).unwrap();
    mid.run_ticks(20);
    let bytes = mid.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.inject_schedule_toml(GOAL_INJECT).unwrap();
    loaded.run_ticks(20);
    assert_ne!(full.state_hash(), loaded.state_hash());
}

#[test]
fn unknown_effect_type_errors() {
    let err = IncentiveSchedule::from_toml_str(
        r#"
[[incentives]]
id = "x"
[[incentives.effects]]
type = "visibility_modifier"
delta = 1.0
"#,
    )
    .unwrap_err();
    let s = err.to_string();
    assert!(s.contains("config") || s.contains("unknown"), "{s}");
}

#[test]
fn checkpoint_round_trips_schedule() {
    let cfg = tiny(0x8005);
    let mut sim = Simulation::new(cfg).unwrap();
    sim.inject_schedule_toml(GOAL_INJECT).unwrap();
    sim.run_ticks(5);
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert!(loaded.incentive_toml.contains("coop_goal"));
    assert_eq!(loaded.incentives.incentives.len(), 1);
}
