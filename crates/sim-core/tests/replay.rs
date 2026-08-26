//! Record JSONL then replay twice → same state_hash. No network.

use sim_core::{ExperimentConfig, Simulation};
use std::time::{SystemTime, UNIX_EPOCH};

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
count = 3
"#
    ))
    .unwrap()
}

#[test]
fn replay_fixture_twice_same_hash() {
    let rec = std::env::temp_dir().join(format!(
        "agentplace-replay-{}-{}.jsonl",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&rec);
    let mut cfg = tiny(0x91);
    cfg.llm.replay_file = rec.to_string_lossy().into();

    let mut writer = Simulation::new(cfg.clone()).unwrap();
    writer.run_ticks(8);
    assert!(rec.is_file(), "missing replay file {}", rec.display());
    let text = std::fs::read_to_string(&rec).unwrap();
    assert!(
        text.lines().any(|l| !l.trim().is_empty()),
        "replay file should contain JSONL"
    );

    let mut a = Simulation::new(cfg.clone()).unwrap();
    a.run_ticks(8);
    let mut b = Simulation::new(cfg).unwrap();
    b.run_ticks(8);
    assert_eq!(a.state_hash(), b.state_hash());
    let _ = std::fs::remove_file(&rec);
}

#[test]
fn wait_sentinel_recording_matches_replay() {
    use sim_core::Chooser;
    use sim_core::event_log::SimEventKind;

    let rec = std::env::temp_dir().join(format!(
        "agentplace-wait-replay-{}-{}.jsonl",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&rec);
    let mut cfg = tiny(0x92);
    cfg.llm.replay_file = rec.to_string_lossy().into();

    let mut writer = Simulation::new(cfg.clone()).unwrap();
    writer.chooser = Chooser::Wait;
    writer.run_ticks(4);
    let hash = writer.state_hash();
    let waits = writer
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::LlmWait))
        .count();
    let text = std::fs::read_to_string(&rec).unwrap();
    assert!(text.contains("__llm_wait__"), "{text}");

    let mut replayed = Simulation::new(cfg).unwrap();
    replayed.chooser = Chooser::Mock;
    replayed.run_ticks(4);
    assert_eq!(replayed.state_hash(), hash);
    let waits2 = replayed
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::LlmWait))
        .count();
    assert_eq!(waits2, waits);
    let _ = std::fs::remove_file(&rec);
}
