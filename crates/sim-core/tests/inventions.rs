//! M35 inventions overlay.

use sim_core::action::PrimaryAction;
use sim_core::event_log::SimEventKind;
use sim_core::inventions::{InventionsParams, invent_chance};
use sim_core::observation::legal_actions;
use sim_core::{AgentId, ExperimentConfig, InventionKind, Simulation};

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

fn force_invent(sim: &mut Simulation, id: AgentId) {
    for _ in 0..64 {
        sim_core::execute::execute_primary(sim, id, &PrimaryAction::Invent);
        if sim.inventions.values().any(|i| i.inventor == id) {
            return;
        }
        sim.tick = sim.tick.saturating_add(1);
    }
    panic!("invent never succeeded");
}

#[test]
fn overlay_parses_inventions() {
    let p = InventionsParams::from_config_toml(
        "[inventions]\nenabled = true\nshare_delay_ticks = 3\n",
    );
    assert!(p.enabled);
    assert_eq!(p.share_delay_ticks, 3);
    let d = InventionsParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.enabled);
    assert_eq!(d.share_delay_ticks, 8);
}

#[test]
fn inventions_off_not_legal_same_hash() {
    let cfg = tiny(0x35_01);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_inventions(8);
    let a = off.agents.get(&AgentId(0)).unwrap().clone();
    let legal = legal_actions(&off, &a);
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Invent)));
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn mock_inventions_overlay_same_hash() {
    let cfg = tiny(0x35_02);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_inventions(8);
    off.run_ticks(8);
    on.run_ticks(8);
    assert_eq!(off.state_hash(), on.state_hash());
    assert!(on.inventions.is_empty());
}

#[test]
fn execute_invent_inventor_then_society() {
    let mut sim = Simulation::new(tiny(0x35_03)).unwrap();
    sim.enable_inventions(8);
    let a = AgentId(0);
    let b = AgentId(1);
    let inf0 = sim.agents.get(&a).unwrap().influence_factor;
    force_invent(&mut sim, a);
    assert_eq!(sim.inventions.len(), 1);
    let inv = sim.inventions.values().next().unwrap();
    assert_eq!(inv.inventor, a);
    assert!(!inv.shared);
    assert!(matches!(inv.kind, InventionKind::GatherBonus));
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Invented { inventor, kind: InventionKind::GatherBonus } if inventor == a
    )));
    assert_eq!(
        sim.agents.get(&a).unwrap().influence_factor,
        inf0.saturating_add(200)
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, a, "food"),
        1200
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, b, "food"),
        1000
    );
    let obs = sim_core::observation::build(&sim, a);
    assert!(
        obs.inventions.iter().any(|s| s.contains("(yours)")),
        "{:?}",
        obs.inventions
    );
    let obs_b = sim_core::observation::build(&sim, b);
    assert!(obs_b.inventions.is_empty(), "{:?}", obs_b.inventions);

    sim_core::execute::execute_primary(&mut sim, b, &PrimaryAction::Invent);
    assert_eq!(sim.inventions.len(), 1);

    let invented_tick = sim.inventions.values().next().unwrap().tick;
    while sim.tick < invented_tick.saturating_add(8) {
        sim.tick();
    }
    assert!(sim.inventions.values().next().unwrap().shared);
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, a, "food"),
        1200
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, b, "food"),
        1200
    );
    let obs_b = sim_core::observation::build(&sim, b);
    assert!(
        obs_b.inventions.iter().any(|s| s == "invention gather_bonus"),
        "{:?}",
        obs_b.inventions
    );
}

#[test]
fn inventions_load_does_not_regrant_influence() {
    let mut sim = Simulation::new(tiny(0x35_04)).unwrap();
    sim.enable_inventions(8);
    let a = AgentId(0);
    force_invent(&mut sim, a);
    let inf = sim.agents.get(&a).unwrap().influence_factor;
    let n = sim.inventions.len();
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.enable_inventions(8);
    assert_eq!(loaded.inventions.len(), n);
    assert_eq!(loaded.agents.get(&a).unwrap().influence_factor, inf);
}

#[test]
fn invent_chance_int_18_higher_than_unused() {
    assert_eq!(invent_chance(0), 400);
    assert_eq!(invent_chance(18), 400 + 4 * 50);
    assert!(invent_chance(18) > invent_chance(0));
}
