use sim_core::event_log::SimEventKind;
use sim_core::incentive::IncentiveSchedule;
use sim_core::llm::prompt_hash;
use sim_core::observation;
use sim_core::{AgentId, ExperimentConfig, Simulation};

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

fn bonus_toml(visibility: &str) -> String {
    format!(
        r#"
[[incentives]]
id = "food_bonus"
description = "1.4x food"
visibility = "{visibility}"
applies_to = "all"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
"#
    )
}

#[test]
fn hidden_incentive_omitted_from_observation_effects_still_apply() {
    let mut sim = Simulation::new(tiny(0x8010)).unwrap();
    sim.inject_schedule_toml(&bonus_toml("hidden")).unwrap();
    sim.run_ticks(1);
    let id = AgentId(0);
    let obs = observation::build(&sim, id);
    assert!(
        !obs.incentives.iter().any(|i| i.id == "food_bonus"),
        "hidden banner must not appear: {:?}",
        obs.incentives
    );
    assert!(sim.incentive_active.contains("food_bonus"));
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, id, "food"),
        1400
    );
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::IncentiveApplied { ref id, .. } if id == "food_bonus"
    )));
}

#[test]
fn public_incentive_appears_in_observation() {
    let mut sim = Simulation::new(tiny(0x8011)).unwrap();
    sim.inject_schedule_toml(&bonus_toml("public")).unwrap();
    sim.run_ticks(1);
    let obs = observation::build(&sim, AgentId(0));
    assert!(obs.incentives.iter().any(|i| i.id == "food_bonus"));
}

#[test]
fn omitted_visibility_defaults_public() {
    let mut sim = Simulation::new(tiny(0x8012)).unwrap();
    sim.inject_schedule_toml(
        r#"
[[incentives]]
id = "food_bonus"
applies_to = "all"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
"#,
    )
    .unwrap();
    sim.run_ticks(1);
    let obs = observation::build(&sim, AgentId(0));
    assert!(obs.incentives.iter().any(|i| i.id == "food_bonus"));
}

#[test]
fn public_vs_hidden_same_effects_same_hash() {
    let cfg = tiny(0x8013);
    let mut pub_run = Simulation::new(cfg.clone()).unwrap();
    let mut hid_run = Simulation::new(cfg).unwrap();
    pub_run.inject_schedule_toml(&bonus_toml("public")).unwrap();
    hid_run.inject_schedule_toml(&bonus_toml("hidden")).unwrap();
    pub_run.run_ticks(20);
    hid_run.run_ticks(20);
    assert_eq!(pub_run.state_hash(), hid_run.state_hash());
    let hp = prompt_hash(&observation::build(&pub_run, AgentId(0)));
    let hh = prompt_hash(&observation::build(&hid_run, AgentId(0)));
    assert_ne!(hp, hh, "hidden banner must change prompt_hash");
}

#[test]
fn hidden_goal_injection_still_in_observation_goals() {
    let mut sim = Simulation::new(tiny(0x8014)).unwrap();
    sim.inject_schedule_toml(
        r#"
[[incentives]]
id = "secret_coop"
visibility = "hidden"
applies_to = "all"
[[incentives.effects]]
type = "goal_injection"
goal_text = "keep the shared storage stocked"
scope = "personal"
priority = 0.9
"#,
    )
    .unwrap();
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&AgentId(0)) {
        a.x = land.0;
        a.y = land.1;
        a.inventory.clear();
        a.try_add_item(sim_core::ItemId::Food(1), 4);
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.thirst = sim.config.thirst_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
        a.personality.agreeableness = 10;
    }
    sim.run_ticks(8);
    let obs = observation::build(&sim, AgentId(0));
    assert!(
        !obs.incentives.iter().any(|i| i.id == "secret_coop"),
        "{:?}",
        obs.incentives
    );
    assert!(
        obs.goals.iter().any(|g| g.text.contains("shared storage")),
        "goals={:?}",
        obs.goals
    );
    let stores = sim
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::Store { .. }))
        .count();
    assert!(
        stores >= 1,
        "mock should still Store, events={:?}",
        sim.events.events
    );
}

#[test]
fn unknown_visibility_is_load_error() {
    let err = IncentiveSchedule::from_toml_str(
        r#"
[[incentives]]
id = "x"
visibility = "maybe"
"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("config"), "{err}");
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
