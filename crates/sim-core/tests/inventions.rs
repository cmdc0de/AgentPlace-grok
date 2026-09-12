//! M35 inventions overlay.

use sim_core::PipelineParams;
use sim_core::action::PrimaryAction;
use sim_core::agent::ItemId;
use sim_core::event_log::SimEventKind;
use sim_core::inventions::{InventionsParams, apply_move_cost, apply_sense_range, invent_chance};
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
    force_invent_kind(sim, id, InventionKind::GatherBonus);
}

fn force_invent_kind(sim: &mut Simulation, id: AgentId, kind: InventionKind) {
    for _ in 0..64 {
        sim_core::execute::execute_primary(sim, id, &PrimaryAction::Invent);
        if sim.inventions.values().any(|i| i.kind == kind) {
            return;
        }
        sim.tick = sim.tick.saturating_add(1);
    }
    panic!("invent {kind:?} never succeeded");
}

#[test]
fn overlay_parses_inventions() {
    let p =
        InventionsParams::from_config_toml("[inventions]\nenabled = true\nshare_delay_ticks = 3\n");
    assert!(p.enabled);
    assert_eq!(p.share_delay_ticks, 3);
    assert!(!p.tree);
    assert_eq!(p.patent_ticks, 0);
    let d = InventionsParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.enabled);
    assert_eq!(d.share_delay_ticks, 8);
    let t = InventionsParams::from_config_toml(
        "[inventions]\nenabled = true\ntree = true\npatent_ticks = 4\n",
    );
    assert!(t.tree);
    assert_eq!(t.patent_ticks, 4);
    assert!(!PipelineParams::from_config_toml("[llm]\nprovider = \"mock\"\n").hash_events);
    assert!(PipelineParams::from_config_toml("[pipeline]\nhash_events = true\n").hash_events);
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

    let invented_tick = sim
        .inventions
        .values()
        .find(|i| i.kind == InventionKind::GatherBonus)
        .unwrap()
        .tick;
    while sim.tick < invented_tick.saturating_add(8) {
        sim.tick();
    }
    assert!(
        sim.inventions
            .values()
            .find(|i| i.kind == InventionKind::GatherBonus)
            .unwrap()
            .shared
    );
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
        obs_b
            .inventions
            .iter()
            .any(|s| s == "invention gather_bonus"),
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

#[test]
fn execute_invent_move_then_sense() {
    let mut sim = Simulation::new(tiny(0x37_01)).unwrap();
    sim.enable_inventions(8);
    let a = AgentId(0);
    let b = AgentId(1);
    sim.agents
        .get_mut(&a)
        .unwrap()
        .try_add_item(ItemId::Stone, 4);
    sim.agents
        .get_mut(&b)
        .unwrap()
        .try_add_item(ItemId::Stone, 4);
    force_invent(&mut sim, a);
    force_invent_kind(&mut sim, a, InventionKind::MoveBonus);
    assert!(
        sim.inventions
            .values()
            .any(|i| i.kind == InventionKind::MoveBonus && i.inventor == a && !i.shared)
    );
    let base_a = sim.agents.get(&a).unwrap().move_cost_milli(&sim.storage);
    let base_b = sim.agents.get(&b).unwrap().move_cost_milli(&sim.storage);
    let cost_a = apply_move_cost(base_a, &sim.inventions, a);
    let cost_b = apply_move_cost(base_b, &sim.inventions, b);
    assert!(base_a > 0 && base_b > 0, "need cargo for move cost");
    assert!(cost_a < base_a, "inventor MoveBonus {cost_a} vs {base_a}");
    assert_eq!(cost_b, base_b, "others full cost until share");
    let obs = sim_core::observation::build(&sim, a);
    assert!(
        obs.inventions
            .iter()
            .any(|s| s.contains("move_bonus") && s.contains("yours")),
        "{:?}",
        obs.inventions
    );

    force_invent_kind(&mut sim, a, InventionKind::SenseBonus);
    let raw = sim_core::observation::perceive_range(
        sim.config.observation.base_vision_range,
        sim.agents.get(&a).unwrap().personality.perceptiveness,
        sim.agents.get(&a).unwrap().sheet.wisdom,
    );
    assert_eq!(apply_sense_range(raw, &sim.inventions, a), raw + 1);
    assert_eq!(apply_sense_range(raw, &sim.inventions, b), raw);
    let obs = sim_core::observation::build(&sim, a);
    assert!(
        obs.inventions
            .iter()
            .any(|s| s.contains("sense_bonus") && s.contains("yours")),
        "{:?}",
        obs.inventions
    );

    let move_tick = sim
        .inventions
        .values()
        .find(|i| i.kind == InventionKind::MoveBonus)
        .unwrap()
        .tick;
    let sense_tick = sim
        .inventions
        .values()
        .find(|i| i.kind == InventionKind::SenseBonus)
        .unwrap()
        .tick;
    while sim.tick
        < move_tick
            .saturating_add(8)
            .max(sense_tick.saturating_add(8))
    {
        sim.tick();
    }
    assert!(
        sim.inventions
            .values()
            .filter(|i| matches!(i.kind, InventionKind::MoveBonus | InventionKind::SenseBonus))
            .all(|i| i.shared)
    );
    let cost_b = apply_move_cost(
        sim.agents.get(&b).unwrap().move_cost_milli(&sim.storage),
        &sim.inventions,
        b,
    );
    let cost_a = apply_move_cost(
        sim.agents.get(&a).unwrap().move_cost_milli(&sim.storage),
        &sim.inventions,
        a,
    );
    assert_eq!(cost_a, cost_b, "no stack after share");
    let raw_b = sim_core::observation::perceive_range(
        sim.config.observation.base_vision_range,
        sim.agents.get(&b).unwrap().personality.perceptiveness,
        sim.agents.get(&b).unwrap().sheet.wisdom,
    );
    assert_eq!(apply_sense_range(raw_b, &sim.inventions, b), raw_b + 1);
    assert_eq!(apply_sense_range(raw, &sim.inventions, a), raw + 1);
}

#[test]
fn fourth_invent_waits_all_kinds_present() {
    let mut sim = Simulation::new(tiny(0x37_02)).unwrap();
    sim.enable_inventions(8);
    let a = AgentId(0);
    force_invent(&mut sim, a);
    force_invent_kind(&mut sim, a, InventionKind::MoveBonus);
    force_invent_kind(&mut sim, a, InventionKind::SenseBonus);
    assert_eq!(sim.inventions.len(), 3);
    let n_events = sim.events.events.len();
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Invent);
    assert_eq!(sim.inventions.len(), 3);
    assert!(
        sim.events.events[n_events..]
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::Wait))
    );
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Invent)));
}

#[test]
fn tree_blocks_move_until_gather_shared() {
    let mut sim = Simulation::new(tiny(0x45_01)).unwrap();
    sim.enable_inventions(1);
    sim.invention_tree = true;
    let a = AgentId(0);
    force_invent(&mut sim, a);
    assert!(
        sim.inventions
            .values()
            .any(|i| i.kind == InventionKind::GatherBonus && !i.shared)
    );
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Invent)));
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Invent);
    assert_eq!(sim.inventions.len(), 1);
    let invented = sim
        .inventions
        .values()
        .find(|i| i.kind == InventionKind::GatherBonus)
        .unwrap()
        .tick;
    while sim.tick < invented.saturating_add(1) {
        sim.tick();
    }
    assert!(
        sim.inventions
            .values()
            .find(|i| i.kind == InventionKind::GatherBonus)
            .unwrap()
            .shared
    );
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(legal.iter().any(|x| matches!(x, PrimaryAction::Invent)));
    force_invent_kind(&mut sim, a, InventionKind::MoveBonus);
    assert!(
        sim.inventions
            .values()
            .any(|i| i.kind == InventionKind::MoveBonus)
    );
}

#[test]
fn patent_ticks_delay_society_share() {
    let mut sim = Simulation::new(tiny(0x45_02)).unwrap();
    sim.enable_inventions(1);
    sim.invention_patent_ticks = 4;
    let a = AgentId(0);
    let b = AgentId(1);
    force_invent(&mut sim, a);
    let invented = sim
        .inventions
        .values()
        .find(|i| i.kind == InventionKind::GatherBonus)
        .unwrap()
        .tick;
    while sim.tick < invented.saturating_add(1) {
        sim.tick();
    }
    assert!(
        !sim.inventions
            .values()
            .find(|i| i.kind == InventionKind::GatherBonus)
            .unwrap()
            .shared,
        "still patented after share_delay"
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, b, "food"),
        1000
    );
    while sim.tick < invented.saturating_add(5) {
        sim.tick();
    }
    assert!(
        sim.inventions
            .values()
            .find(|i| i.kind == InventionKind::GatherBonus)
            .unwrap()
            .shared
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, b, "food"),
        1200
    );
}

#[test]
fn pipeline_overlay_off_same_hash() {
    let cfg = tiny(0x45_03);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut flagged = Simulation::new(cfg).unwrap();
    flagged.pipeline_hash_events = false;
    off.run_ticks(3);
    flagged.run_ticks(3);
    assert_eq!(off.state_hash(), flagged.state_hash());
    assert!(
        !off.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::Pipeline { .. }))
    );
}

#[test]
fn pipeline_on_emits_complete_and_changes_hash() {
    let cfg = tiny(0x45_04);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.pipeline_hash_events = true;
    off.run_ticks(2);
    on.run_ticks(2);
    assert_ne!(off.state_hash(), on.state_hash());
    let pipes: Vec<_> = on
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::Pipeline { .. }))
        .collect();
    assert_eq!(pipes.len(), 4, "2 ticks × 2 living agents");
    assert!(
        pipes
            .iter()
            .all(|e| matches!(e.kind, SimEventKind::Pipeline { stages: 31 }))
    );
}
