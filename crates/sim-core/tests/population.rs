//! M31 sheet, kinship, reproduction.

use sim_core::action::PrimaryAction;
use sim_core::event_log::SimEventKind;
use sim_core::kinship::PopulationParams;
use sim_core::observation::legal_actions;
use sim_core::sheet::{AbilitySheet, SheetParams};
use sim_core::{AgentId, ExperimentConfig, Simulation};

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

fn place_adjacent(sim: &mut Simulation) -> (AgentId, AgentId) {
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let a = ids[0];
    let b = ids[1];
    let (x, y) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    let mut nx = x.saturating_add(1);
    let mut ny = y;
    if !sim.world.in_bounds(nx as i32, ny as i32) || !sim.world.is_land(nx, ny) {
        nx = x.saturating_sub(1);
    }
    if !sim.world.is_land(nx, ny) {
        ny = y.saturating_add(1);
        nx = x;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = nx;
        ag.y = ny;
    }
    (a, b)
}

fn fill_energy(sim: &mut Simulation) {
    let emax = sim.config.energy_max_milli();
    for a in sim.agents.values_mut() {
        a.needs.energy = emax;
    }
}

#[test]
fn overlay_parses_sheet_and_population() {
    let s = SheetParams::from_config_toml("[agents.sheet]\nenabled = true\n");
    assert!(s.enabled);
    let off = SheetParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!off.enabled);
    let p = PopulationParams::from_config_toml("[population]\nreproduction = true\n");
    assert!(p.reproduction);
    let po = PopulationParams::from_config_toml("[conflict]\nenabled = true\n");
    assert!(!po.reproduction);
    assert!(!po.aging);
    let ag = PopulationParams::from_config_toml(
        "[population]\naging = true\nchildhood_ticks = 10\nfounder_age_ticks = 50\n",
    );
    assert!(ag.aging);
    assert_eq!(ag.childhood_ticks, 10);
    assert_eq!(ag.founder_age_ticks, 50);
}

#[test]
fn sheet_overlay_off_same_hash() {
    let cfg = tiny(0x31_01);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(6);
    b.run_ticks(6);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(a.agents.values().all(|ag| ag.sheet.is_unused()));
}

#[test]
fn sheet_on_rolls_and_changes_hash_ckpt_round_trip() {
    let cfg = tiny(0x31_02);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_sheet();
    assert!(on.agents.values().all(|a| {
        a.sheet.rows().iter().all(|(_, s)| (3..=18).contains(s))
    }));
    off.run_ticks(4);
    on.run_ticks(4);
    assert_ne!(off.state_hash(), on.state_hash());
    let restored = Simulation::decode_checkpoint(&on.encode_checkpoint().unwrap()).unwrap();
    assert_eq!(restored.state_hash(), on.state_hash());
    for (id, a) in &on.agents {
        assert_eq!(restored.agents.get(id).unwrap().sheet, a.sheet);
    }
}

#[test]
fn reproduction_overlay_off_not_legal() {
    let mut sim = Simulation::new(tiny(0x31_03)).unwrap();
    let (a, _) = place_adjacent(&mut sim);
    let agent = sim.agents.get(&a).unwrap().clone();
    let legal = legal_actions(&sim, &agent);
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::PairBond { .. })));
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Reproduce { .. })));
}

#[test]
fn mock_reproduction_overlay_no_custom_same_hash() {
    let cfg = tiny(0x31_04);
    let mut sheet_only = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    sheet_only.enable_sheet();
    on.enable_reproduction();
    sheet_only.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(sheet_only.state_hash(), on.state_hash());
}

#[test]
fn pair_bond_and_reproduce_writes_kin_and_calculated_sheet() {
    let mut sim = Simulation::new(tiny(0x31_05)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    let ten = AbilitySheet {
        strength: 10,
        dexterity: 10,
        constitution: 10,
        intelligence: 10,
        wisdom: 10,
        charisma: 10,
    };
    sim.agents.get_mut(&a).unwrap().sheet = ten;
    sim.agents.get_mut(&b).unwrap().sheet = ten;
    sim.agents.get_mut(&a).unwrap().health = 10_000;
    sim.agents.get_mut(&b).unwrap().health = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    assert_eq!(sim.agents.get(&a).unwrap().kinship.pair_bond, Some(b));
    assert_eq!(sim.agents.get(&b).unwrap().kinship.pair_bond, Some(a));
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::PairBonded { with } if with == b
    )));
    let before = sim.agents.len();
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    assert_eq!(sim.agents.len(), before + 1);
    let child_id = sim
        .agents
        .keys()
        .copied()
        .find(|id| *id != a && *id != b)
        .unwrap();
    let child = sim.agents.get(&child_id).unwrap();
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Born { parent_a, parent_b }
        if parent_a == a && parent_b == b || parent_a == b && parent_b == a
    )));
    assert!(child.kinship.parents.contains(&a));
    assert!(child.kinship.parents.contains(&b));
    assert!(sim.agents.get(&a).unwrap().kinship.children.contains(&child_id));
    assert!(sim.agents.get(&b).unwrap().kinship.children.contains(&child_id));
    for (_, score) in child.sheet.rows() {
        assert!((9..=11).contains(&score), "calculated around 10, got {score}");
    }
    let obs = sim_core::observation::build(&sim, child_id);
    assert!(obs.kin.iter().any(|s| s.contains("parent #")), "{:?}", obs.kin);
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    let kids: Vec<AgentId> = sim.agents.get(&a).unwrap().kinship.children.clone();
    assert!(kids.len() >= 2, "{kids:?}");
    let c0 = sim.agents.get(&kids[0]).unwrap();
    let c1 = sim.agents.get(&kids[1]).unwrap();
    assert!(c0.kinship.siblings.contains(&kids[1]) || c1.kinship.siblings.contains(&kids[0]));
}

fn tiny3(seed: u64) -> ExperimentConfig {
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
count = 3
"#
    ))
    .unwrap()
}

const KIN_FOOD: &str = r#"
[[incentives]]
id = "kin_food"
applies_to = "kin_of:0"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
"#;

#[test]
fn kin_of_scope_after_pair_bond_and_birth() {
    let mut sim = Simulation::new(tiny(0x32_01)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    assert_eq!(a, AgentId(0));
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    sim.inject_schedule_toml(KIN_FOOD).unwrap();
    sim.run_ticks(1);
    assert_eq!(sim_core::incentive::resource_mult_milli(&sim, a, "food"), 1000);
    assert_eq!(sim_core::incentive::resource_mult_milli(&sim, b, "food"), 1400);
    let child = sim
        .agents
        .keys()
        .copied()
        .find(|id| *id != a && *id != b)
        .unwrap();
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, child, "food"),
        1400
    );
}

#[test]
fn kin_of_missing_agent_empty_scope() {
    let mut sim = Simulation::new(tiny(0x32_02)).unwrap();
    sim.inject_schedule_toml(
        r#"
[[incentives]]
id = "kin_food"
applies_to = "kin_of:99"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
"#,
    )
    .unwrap();
    sim.run_ticks(1);
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, AgentId(0), "food"),
        1000
    );
}

#[test]
fn pair_bond_mints_household_child_inherits() {
    let mut sim = Simulation::new(tiny3(0x32_03)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    let ha = sim.agents.get(&a).unwrap().kinship.household;
    let hb = sim.agents.get(&b).unwrap().kinship.household;
    assert!(ha.is_some());
    assert_eq!(ha, hb);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    let child = sim
        .agents
        .keys()
        .copied()
        .find(|id| *id != a && *id != b && sim.agents.get(id).unwrap().kinship.parents.contains(&a))
        .unwrap();
    assert_eq!(sim.agents.get(&child).unwrap().kinship.household, ha);
    let hid = ha.unwrap();
    let toml = format!(
        r#"
[[incentives]]
id = "house_food"
applies_to = "household:{hid}"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
"#
    );
    sim.inject_schedule_toml(&toml).unwrap();
    sim.run_ticks(1);
    assert_eq!(sim_core::incentive::resource_mult_milli(&sim, a, "food"), 1400);
    let outsider = sim
        .agents
        .keys()
        .copied()
        .find(|id| sim.agents.get(id).unwrap().kinship.household != ha)
        .unwrap();
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, outsider, "food"),
        1000
    );
}

#[test]
fn aging_off_zero_same_hash() {
    let cfg = tiny(0x32_04);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(6);
    b.run_ticks(6);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(a.agents.values().all(|ag| ag.age_ticks == 0));
}

#[test]
fn aging_stamps_founders_gates_child() {
    let mut sim = Simulation::new(tiny(0x32_05)).unwrap();
    sim.enable_reproduction();
    sim.enable_aging(80, 200);
    assert!(sim.agents.values().all(|a| a.age_ticks == 200));
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    let child = sim
        .agents
        .keys()
        .copied()
        .find(|id| sim.agents.get(id).unwrap().kinship.parents.contains(&a))
        .unwrap();
    assert_eq!(sim.agents.get(&child).unwrap().age_ticks, 0);
    sim.conflict_enabled = true;
    let ch = sim.agents.get(&child).unwrap().clone();
    let legal = legal_actions(&sim, &ch);
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Attack { .. })));
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::PairBond { .. })));
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Reproduce { .. })));
    sim.run_ticks(1);
    assert!(sim.agents.get(&a).unwrap().age_ticks >= 201);
}

#[test]
fn aging_load_does_not_restamp_founder_age() {
    let mut sim = Simulation::new(tiny(0x32_06)).unwrap();
    sim.enable_aging(80, 200);
    sim.run_ticks(3);
    let age = sim.agents.get(&AgentId(0)).unwrap().age_ticks;
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.enable_aging(80, 200);
    assert_eq!(loaded.agents.get(&AgentId(0)).unwrap().age_ticks, age);
    assert_ne!(age, 200);
}
