use sim_core::{
    decode_checkpoint, encode_checkpoint, write_run_checkpoint, ExperimentConfig, Simulation,
    CHECKPOINT_FORMAT_VERSION, CHECKPOINT_MAGIC,
};

fn tiny_config(master_seed: u64) -> ExperimentConfig {
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
count = 6
spawn_mode = "scattered"
"#
    );
    ExperimentConfig::from_toml_str(&toml).expect("valid test config")
}

#[test]
fn checkpoint_round_trip_preserves_hash() {
    let mut sim = Simulation::new(tiny_config(0xC0FFEE)).unwrap();
    sim.run_ticks(25);
    let hash = sim.state_hash();
    let bytes = sim.encode_checkpoint().unwrap();
    let restored = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(restored.tick, 25);
    assert_eq!(restored.state_hash(), hash);
    assert_eq!(restored.world.water, sim.world.water);
    assert_eq!(restored.agents.len(), sim.agents.len());
}

#[test]
fn continuation_matches_uninterrupted_run() {
    let cfg = tiny_config(0xBEEF);
    let mut continuous = Simulation::new(cfg.clone()).unwrap();
    continuous.run_ticks(50);
    let expected = continuous.state_hash();

    let mut branched = Simulation::new(cfg).unwrap();
    branched.run_ticks(30);
    let bytes = branched.encode_checkpoint().unwrap();
    let mut restored = Simulation::decode_checkpoint(&bytes).unwrap();
    restored.run_ticks(20);
    assert_eq!(restored.tick, 50);
    assert_eq!(restored.state_hash(), expected);
}

#[test]
fn rng_stream_position_is_restored() {
    let mut sim = Simulation::new(tiny_config(11)).unwrap();
    sim.run_ticks(12);
    let before = sim.rngs.fingerprint();
    let restored = Simulation::decode_checkpoint(&sim.encode_checkpoint().unwrap()).unwrap();
    assert_eq!(restored.rngs.fingerprint(), before);
}

#[test]
fn unknown_format_version_is_rejected() {
    let mut sim = Simulation::new(tiny_config(3)).unwrap();
    sim.run_ticks(4);
    let mut bytes = sim.encode_checkpoint().unwrap();
    bytes[4..8].copy_from_slice(&(CHECKPOINT_FORMAT_VERSION + 1).to_le_bytes());
    let err = Simulation::decode_checkpoint(&bytes).unwrap_err();
    assert!(
        err.to_string().contains("format_version"),
        "unexpected error: {err}"
    );
}

#[test]
fn truncated_and_bad_magic_are_rejected() {
    assert!(Simulation::decode_checkpoint(&[1, 2, 3]).is_err());
    let mut bytes = Simulation::new(tiny_config(4))
        .unwrap()
        .encode_checkpoint()
        .unwrap();
    bytes[0..4].copy_from_slice(b"XXXX");
    let err = Simulation::decode_checkpoint(&bytes).unwrap_err();
    assert!(err.to_string().contains("magic"), "unexpected error: {err}");
}

#[test]
fn encode_starts_with_magic_and_version() {
    let bytes = Simulation::new(tiny_config(5))
        .unwrap()
        .encode_checkpoint()
        .unwrap();
    assert_eq!(&bytes[0..4], &CHECKPOINT_MAGIC);
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        CHECKPOINT_FORMAT_VERSION
    );
    decode_checkpoint(&bytes).unwrap();
    let _ = encode_checkpoint;
}

#[test]
fn write_run_checkpoint_emits_markdown_and_reloads() {
    let dir = std::env::temp_dir().join(format!("agentplace-m2-{}-{}", std::process::id(), 42));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut sim = Simulation::new(tiny_config(9)).unwrap();
    sim.run_ticks(3);
    let expected = sim.state_hash();
    let path = write_run_checkpoint(&sim, &dir).unwrap();
    assert!(path.exists());
    let stem = path.file_stem().unwrap().to_string_lossy();
    let summary = std::fs::read_to_string(dir.join(format!("{stem}_summary.md"))).unwrap();
    assert!(summary.contains("tick: 3"), "{summary}");
    assert!(summary.contains("vegetation_patches"), "{summary}");
    let agents = std::fs::read_to_string(dir.join(format!("{stem}_agents.md"))).unwrap();
    assert!(agents.contains("## Agent 0"), "{agents}");
    let loaded = Simulation::load_checkpoint(&path).unwrap();
    assert_eq!(loaded.state_hash(), expected);
    let _ = std::fs::remove_dir_all(&dir);
}
