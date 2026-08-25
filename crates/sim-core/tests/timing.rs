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
count = 4
"#
    ))
    .unwrap()
}

#[test]
fn timing_does_not_change_hash() {
    let cfg = tiny(0x71_01);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(40);
    b.run_ticks(40);
    assert_eq!(a.state_hash(), b.state_hash());
    let t = a.last_tick_timing.expect("timing recorded");
    assert!(t.wall_ns > 0, "wall_ns={}", t.wall_ns);
    assert_eq!(t.agents.len(), a.agents.len());
    assert!(t
        .agents
        .iter()
        .any(|x| x.perceive_ns > 0 || x.select_ns > 0));
}
