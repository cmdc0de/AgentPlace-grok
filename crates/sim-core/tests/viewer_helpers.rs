use sim_core::observation::{self, visible_in_observation};
use sim_core::{AgentId, ExperimentConfig, Simulation};

fn tiny() -> ExperimentConfig {
    ExperimentConfig::from_toml_str(
        r#"
master_seed = 9
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
    )
    .unwrap()
}

#[test]
fn visible_in_observation_matches_tile_list() {
    let sim = Simulation::new(tiny()).unwrap();
    let id = AgentId(0);
    let obs = observation::build(&sim, id);
    for t in &obs.tiles {
        assert!(visible_in_observation(&obs, t.x, t.y));
    }
    assert!(!visible_in_observation(&obs, 10_000, 10_000));
}

#[test]
fn full_information_observation_covers_map() {
    let mut cfg = tiny();
    cfg.observation.full_information = true;
    let sim = Simulation::new(cfg).unwrap();
    let obs = observation::build(&sim, AgentId(0));
    let cells = (sim.world.width * sim.world.height) as usize;
    assert_eq!(obs.tiles.len(), cells);
    assert!(visible_in_observation(&obs, 0, 0));
    assert!(visible_in_observation(
        &obs,
        sim.world.width - 1,
        sim.world.height - 1
    ));
}
