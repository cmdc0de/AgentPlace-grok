//! M46 telemetry overlay. Hash-neutral.

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
fn telemetry_overlay_off_same_hash() {
    let cfg = tiny(0x46_20);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut flagged = Simulation::new(cfg).unwrap();
    flagged.telemetry_enabled = false;
    off.run_ticks(3);
    flagged.run_ticks(3);
    assert_eq!(off.state_hash(), flagged.state_hash());
    assert_eq!(off.telemetry_ticks.count(), 0);
}

#[test]
fn telemetry_on_records_and_same_hash() {
    let cfg = tiny(0x46_21);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.telemetry_enabled = true;
    off.run_ticks(4);
    on.run_ticks(4);
    assert_eq!(off.state_hash(), on.state_hash(), "telemetry must not hash");
    assert_eq!(on.telemetry_ticks.count(), 4);
    assert!(on.telemetry_ticks.total() > 0);
    assert!(on.telemetry_ticks.min() <= on.telemetry_ticks.max());
    assert_eq!(
        on.telemetry_ticks.average() * 4,
        on.telemetry_ticks.total() - (on.telemetry_ticks.total() % 4)
    );
    assert_eq!(off.telemetry_ticks.count(), 0);
    assert!(on.telemetry_otlp_endpoint.is_empty());
    assert!(off.telemetry_cpu_user_ns.is_none());
    assert!(off.telemetry_disk_read_bytes.is_none());
    if cfg!(target_os = "linux") {
        assert!(on.telemetry_cpu_user_ns.is_some());
        assert!(on.telemetry_cpu_system_ns.is_some());
        assert!(on.telemetry_disk_read_bytes.is_some());
        assert!(on.telemetry_disk_write_bytes.is_some());
    }
}
