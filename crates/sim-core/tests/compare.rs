use sim_core::{ExperimentConfig, Simulation, compare_markdown, compare_runs};

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
fn compare_identical_sims_hashes_equal() {
    let cfg = tiny(0xC0);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(12);
    b.run_ticks(12);
    let report = compare_runs(&a, "a", &b, "b");
    assert!(report.hashes_equal());
    let md = compare_markdown(&report);
    assert!(md.contains("equal"), "{md}");
}

#[test]
fn compare_coop_vs_baseline_differs_goals() {
    let cfg = tiny(0xC1);
    let mut base = Simulation::new(cfg.clone()).unwrap();
    let mut coop = Simulation::new(cfg).unwrap();
    coop.inject_schedule_toml(GOAL_INJECT).unwrap();
    base.run_ticks(12);
    coop.run_ticks(12);
    let report = compare_runs(&base, "base", &coop, "coop");
    assert!(!report.hashes_equal());
    let text = "keep the shared storage stocked";
    assert_eq!(report.a.goal_occupancy.get(text).copied().unwrap_or(0), 0);
    assert!(
        report.b.goal_occupancy.get(text).copied().unwrap_or(0) >= 1,
        "{:?}",
        report.b.goal_occupancy
    );
    let md = compare_markdown(&report);
    assert!(md.contains("differ"), "{md}");
    assert!(md.contains(text), "{md}");
}
