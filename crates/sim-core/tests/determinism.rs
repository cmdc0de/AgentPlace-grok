use sim_core::{ExperimentConfig, Simulation, SpawnMode};

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

#[test]
fn config_rejects_fixed_list_spawn() {
    let toml = r#"
master_seed = 1
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 4
spawn_mode = "fixed_list"
"#;
    let err = ExperimentConfig::from_toml_str(toml).unwrap_err();
    assert!(err.to_string().contains("fixed_list"));
}

#[test]
fn same_seed_same_resource_layers() {
    let cfg = tiny_config(0xA11CE, 6);
    let a = Simulation::new(cfg.clone()).unwrap();
    let b = Simulation::new(cfg).unwrap();
    assert_eq!(a.world.water, b.world.water);
    assert_eq!(a.world.vegetation, b.world.vegetation);
    assert_eq!(a.world.minerals, b.world.minerals);
    assert_eq!(a.world_hash(), b.world_hash());
}

#[test]
fn resource_minima_honored() {
    let cfg = tiny_config(99, 4);
    let sim = Simulation::new(cfg.clone()).unwrap();
    assert!(sim.world.water_count() >= cfg.world.resources.min_fresh_water);
    assert!(sim.world.vegetation_count() >= cfg.world.resources.min_vegetation_patches);
    assert!(sim.world.mineral_count() >= cfg.world.resources.min_mineral_nodes);
    assert_eq!(
        sim.world.water_count() + sim.world.land_cells().len() as u32,
        sim.world.width * sim.world.height
    );
}

#[test]
fn world_hash_changes_with_resource_density() {
    let mut rich = tiny_config(7, 4);
    let mut poor = tiny_config(7, 4);
    rich.world.resources.vegetation_density = 1.0;
    poor.world.resources.vegetation_density = 0.0;
    // Density 0 still places minima; raise poor min to keep the type valid but
    // force a different target by changing the minimum itself.
    poor.world.resources.min_vegetation_patches = 8;
    rich.world.resources.min_vegetation_patches = 40;
    let a = Simulation::new(rich).unwrap();
    let b = Simulation::new(poor).unwrap();
    assert_ne!(a.world_hash(), b.world_hash());
}

#[test]
fn agents_spawn_on_land() {
    let sim = Simulation::new(tiny_config(123, 8)).unwrap();
    for agent in sim.agents.values() {
        assert!(
            sim.world.is_land(agent.x, agent.y),
            "agent {} spawned on water at ({},{})",
            agent.id.0,
            agent.x,
            agent.y
        );
    }
}

#[test]
fn agents_stay_off_water() {
    let mut sim = Simulation::new(tiny_config(77, 8)).unwrap();
    sim.run_ticks(200);
    for agent in sim.agents.values() {
        assert!(
            sim.world.is_land(agent.x, agent.y),
            "agent {} walked onto water at ({},{})",
            agent.id.0,
            agent.x,
            agent.y
        );
    }
}

#[test]
fn clustered_spawn_is_deterministic() {
    let toml = r#"
master_seed = 4242
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 8
spawn_mode = "clustered"
"#;
    let cfg = ExperimentConfig::from_toml_str(toml).unwrap();
    assert_eq!(cfg.agents.spawn_mode, SpawnMode::Clustered);
    let a = Simulation::new(cfg.clone()).unwrap();
    let b = Simulation::new(cfg).unwrap();
    for (id, agent) in &a.agents {
        let other = b.agents.get(id).unwrap();
        assert_eq!((agent.x, agent.y), (other.x, other.y));
        assert!(a.world.is_land(agent.x, agent.y));
    }
}
