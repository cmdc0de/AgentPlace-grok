//! M31 sheet, kinship, reproduction.

use sim_core::action::{PrimaryAction, Speak, SpeakTarget};
use sim_core::event_log::SimEventKind;
use sim_core::kinship::PopulationParams;
use sim_core::memory::MemoryKind;
use sim_core::observation::{self, legal_actions};
use sim_core::policy;
use sim_core::sheet::{AbilitySheet, SheetParams};
use sim_core::voting::VotingParams;
use sim_core::{AgentId, ExperimentConfig, ItemId, Simulation};

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

/// CHA 0 skips the PairBond d20 so tests that need a bond always get one.
fn zero_cha(sim: &mut Simulation, ids: &[AgentId]) {
    for id in ids {
        if let Some(a) = sim.agents.get_mut(id) {
            a.sheet.charisma = 0;
        }
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
    assert!(!po.household_crates);
    assert!(!po.culture);
    let ag = PopulationParams::from_config_toml(
        "[population]\naging = true\nchildhood_ticks = 10\nfounder_age_ticks = 50\n",
    );
    assert!(ag.aging);
    assert_eq!(ag.childhood_ticks, 10);
    assert_eq!(ag.founder_age_ticks, 50);
    let hc = PopulationParams::from_config_toml(
        "[population]\nhousehold_crates = true\nculture = true\nculture_count = 6\n",
    );
    assert!(hc.household_crates);
    assert!(hc.culture);
    assert_eq!(hc.culture_count, 6);
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
    assert!(
        on.agents
            .values()
            .all(|a| { a.sheet.rows().iter().all(|(_, s)| (3..=18).contains(s)) })
    );
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
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::PairBond { .. }))
    );
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::Reproduce { .. }))
    );
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
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim.agents.get_mut(&a).unwrap().sheet = ten;
    sim.agents.get_mut(&b).unwrap().sheet = ten;
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
    assert!(
        sim.agents
            .get(&a)
            .unwrap()
            .kinship
            .children
            .contains(&child_id)
    );
    assert!(
        sim.agents
            .get(&b)
            .unwrap()
            .kinship
            .children
            .contains(&child_id)
    );
    for (_, score) in child.sheet.rows() {
        assert!(
            (9..=11).contains(&score),
            "calculated around 10, got {score}"
        );
    }
    let obs = sim_core::observation::build(&sim, child_id);
    assert!(
        obs.kin.iter().any(|s| s.contains("parent #")),
        "{:?}",
        obs.kin
    );
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
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    sim.inject_schedule_toml(KIN_FOOD).unwrap();
    sim.run_ticks(1);
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, a, "food"),
        1000
    );
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, b, "food"),
        1400
    );
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
    zero_cha(&mut sim, &[a, b]);
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
    assert_eq!(
        sim_core::incentive::resource_mult_milli(&sim, a, "food"),
        1400
    );
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
    zero_cha(&mut sim, &[a, b]);
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
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::Attack { .. }))
    );
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::PairBond { .. }))
    );
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::Reproduce { .. }))
    );
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

fn land_neighbor(sim: &Simulation, x: u32, y: u32, occupied: &[(u32, u32)]) -> Option<(u32, u32)> {
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if !sim.world.in_bounds(nx, ny) {
                continue;
            }
            let ux = nx as u32;
            let uy = ny as u32;
            if sim.world.is_land(ux, uy) && !occupied.contains(&(ux, uy)) {
                return Some((ux, uy));
            }
        }
    }
    None
}

#[test]
fn household_crates_off_no_pairbond_same_hash() {
    let cfg = tiny(0x33_01);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_household_crates();
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
    assert!(on.household_home.is_empty());
}

#[test]
fn pair_bond_without_crates_overlay_mints_no_home() {
    let mut sim = Simulation::new(tiny(0x33_02)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    assert!(sim.household_home.is_empty());
}

#[test]
fn pair_bond_home_member_store_outsider_cannot() {
    let mut sim = Simulation::new(tiny3(0x33_03)).unwrap();
    sim.enable_reproduction();
    sim.enable_household_crates();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    let hid = sim.agents.get(&a).unwrap().kinship.household.unwrap();
    let home = *sim.household_home.get(&hid).expect("home minted");
    assert_eq!(sim.household_home.get(&hid), Some(&home));
    let actor = sim.agents.get(&a).unwrap();
    assert_eq!((actor.x, actor.y), home);
    let partner = sim.agents.get(&b).unwrap();
    assert_eq!(
        sim_core::observation::chebyshev(partner.x, partner.y, home.0, home.1),
        1
    );

    let outsider = sim
        .agents
        .keys()
        .copied()
        .find(|id| *id != a && *id != b)
        .unwrap();
    let occupied = [
        (sim.agents.get(&a).unwrap().x, sim.agents.get(&a).unwrap().y),
        (sim.agents.get(&b).unwrap().x, sim.agents.get(&b).unwrap().y),
    ];
    let cell = land_neighbor(&sim, home.0, home.1, &occupied).expect("outsider cell");
    if let Some(ag) = sim.agents.get_mut(&outsider) {
        ag.x = cell.0;
        ag.y = cell.1;
        ag.inventory.clear();
        ag.try_add_item(ItemId::Food(1), 2);
        ag.needs.energy = sim.config.energy_max_milli();
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.inventory.clear();
        ag.try_add_item(ItemId::Food(1), 2);
        ag.needs.energy = sim.config.energy_max_milli();
    }

    sim_core::execute::execute_primary(
        &mut sim,
        b,
        &PrimaryAction::Store {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    assert!(
        sim.world.has_stockpile(home.0, home.1),
        "member stores at home"
    );
    let home_qty = sim
        .world
        .stockpile_at(home.0, home.1)
        .and_then(|c| c.items.get(&ItemId::Food(1)).copied())
        .unwrap_or(0);
    assert_eq!(home_qty, 1);

    let member = sim.agents.get(&b).unwrap().clone();
    let legal = legal_actions(&sim, &member);
    assert!(legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Retrieve {
            item: ItemId::Food(1),
            ..
        }
    )));
    let out = sim.agents.get(&outsider).unwrap().clone();
    let out_legal = legal_actions(&sim, &out);
    assert!(!out_legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Retrieve {
            item: ItemId::Food(1),
            ..
        }
    )));

    sim_core::execute::execute_primary(
        &mut sim,
        outsider,
        &PrimaryAction::Store {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    let home_qty2 = sim
        .world
        .stockpile_at(home.0, home.1)
        .and_then(|c| c.items.get(&ItemId::Food(1)).copied())
        .unwrap_or(0);
    assert_eq!(
        home_qty2, 1,
        "outsider must not store at home via household rule"
    );
    let (ox, oy) = {
        let ag = sim.agents.get(&outsider).unwrap();
        (ag.x, ag.y)
    };
    assert_ne!((ox, oy), home);
    assert!(
        sim.world.has_stockpile(ox, oy),
        "outsider stores on standing cell"
    );
}

#[test]
fn household_home_load_does_not_remint() {
    let mut sim = Simulation::new(tiny(0x33_04)).unwrap();
    sim.enable_reproduction();
    sim.enable_household_crates();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    let hid = sim.agents.get(&a).unwrap().kinship.household.unwrap();
    let home = *sim.household_home.get(&hid).unwrap();
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = ag.x.saturating_add(2);
    }
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.enable_household_crates();
    assert_eq!(loaded.household_home.get(&hid), Some(&home));
    assert_eq!(loaded.household_home.len(), 1);
}

#[test]
fn culture_off_zero_same_hash() {
    let cfg = tiny(0x33_05);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(6);
    b.run_ticks(6);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(a.agents.values().all(|ag| ag.culture == 0));
}

#[test]
fn culture_founders_child_copies_load_no_reroll() {
    let cfg = tiny(0x33_06);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_culture(4);
    assert!(on.agents.values().all(|a| (1..=4).contains(&a.culture)));
    off.run_ticks(2);
    on.run_ticks(2);
    assert_ne!(off.state_hash(), on.state_hash());

    let mut sim = Simulation::new(tiny(0x33_07)).unwrap();
    sim.enable_reproduction();
    sim.enable_culture(4);
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    let ca = sim.agents.get(&a).unwrap().culture;
    let cb = sim.agents.get(&b).unwrap().culture;
    zero_cha(&mut sim, &[a, b]);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    let child = sim
        .agents
        .keys()
        .copied()
        .find(|id| sim.agents.get(id).unwrap().kinship.parents.contains(&a))
        .unwrap();
    let (pa, pb) = if a.0 <= b.0 { (ca, cb) } else { (cb, ca) };
    let expect = if pa != 0 { pa } else { pb };
    assert_eq!(sim.agents.get(&child).unwrap().culture, expect);
    let obs = sim_core::observation::build(&sim, a);
    assert!(
        obs.kin.iter().any(|s| s.starts_with("culture ")),
        "{:?}",
        obs.kin
    );

    let before = sim.agents.get(&AgentId(0)).unwrap().culture;
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.enable_culture(4);
    assert_eq!(loaded.agents.get(&AgentId(0)).unwrap().culture, before);
    assert_eq!(
        loaded.agents.get(&child).unwrap().culture,
        sim.agents.get(&child).unwrap().culture
    );
}

fn scores(str: u8, dex: u8, wis: u8, cha: u8) -> AbilitySheet {
    AbilitySheet {
        strength: str,
        dexterity: dex,
        constitution: 10,
        intelligence: 10,
        wisdom: wis,
        charisma: cha,
    }
}

#[test]
fn sheet_unused_mods_zero_same_as_today() {
    let mut sim = Simulation::new(tiny(0x34_01)).unwrap();
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert!(a.sheet.is_unused());
    assert_eq!(a.sheet.attack_damage(), 2000);
    let move0 = a.move_cost_milli(&sim.storage);
    let obs = sim_core::observation::build(&sim, id);
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 10, 10, 10);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(a.sheet.attack_damage(), 2000);
    assert_eq!(a.move_cost_milli(&sim.storage), move0);
    let obs2 = sim_core::observation::build(&sim, id);
    assert_eq!(obs.vision, obs2.vision);
    assert_eq!(obs.identity, obs2.identity);
}

#[test]
fn str_18_vs_10_attack_damage() {
    let mut sim = Simulation::new(tiny(0x34_02)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    sim.agents.get_mut(&a).unwrap().sheet = scores(18, 10, 10, 10);
    sim.agents.get_mut(&b).unwrap().health = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Attack { target, damage } if target == b && damage == 3000
    )));
    sim.agents.get_mut(&a).unwrap().sheet = scores(10, 10, 10, 10);
    fill_energy(&mut sim);
    sim.agents.get_mut(&b).unwrap().health = 10_000;
    sim.agents.get_mut(&b).unwrap().needs.energy = sim.config.energy_max_milli();
    sim.agents.get_mut(&b).unwrap().incapacitated = false;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Attack { target, damage } if target == b && damage == 2000
    )));
}

#[test]
fn dex_18_vs_3_move_cost() {
    let mut sim = Simulation::new(tiny(0x34_03)).unwrap();
    let id = AgentId(0);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Stone, 4);
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 18, 10, 10);
    let hi = sim.agents.get(&id).unwrap().move_cost_milli(&sim.storage);
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 3, 10, 10);
    let lo = sim.agents.get(&id).unwrap().move_cost_milli(&sim.storage);
    assert!(hi < lo, "DEX 18 {hi} vs DEX 3 {lo}");
    assert!(hi > 0, "loaded move cost");
}

#[test]
fn wis_18_vs_3_range() {
    let mut sim = Simulation::new(tiny(0x34_04)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 10, 18, 10);
    let hi = sim_core::observation::build(&sim, id);
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 10, 3, 10);
    let lo = sim_core::observation::build(&sim, id);
    assert!(hi.vision > lo.vision, "vis {} vs {}", hi.vision, lo.vision);
    assert!(
        hi.identity > lo.identity,
        "ident {} vs {}",
        hi.identity,
        lo.identity
    );
}

#[test]
fn cha_18_influence_vote_weight() {
    let mut sim = Simulation::new(tiny(0x34_05)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().influence_factor = 1000;
    sim.agents.get_mut(&id).unwrap().sheet = scores(10, 10, 10, 18);
    sim.voting = VotingParams::influence();
    assert_eq!(sim.vote_weight_of(id), 1400);
    sim.voting = VotingParams::equal();
    assert_eq!(sim.vote_weight_of(id), 1);
    sim.voting = VotingParams::influence();
    sim.agents.get_mut(&id).unwrap().sheet = AbilitySheet::default();
    assert_eq!(sim.vote_weight_of(id), 1000);
}

#[test]
fn close_kin_pair_bond_illegal() {
    let mut sim = Simulation::new(tiny(0x34_06)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Reproduce { with: b });
    let kids: Vec<AgentId> = sim.agents.get(&a).unwrap().kinship.children.clone();
    assert!(kids.len() >= 2, "{kids:?}");
    let c0 = kids[0];
    let c1 = kids[1];
    let (x, y) = {
        let ag = sim.agents.get(&c0).unwrap();
        (ag.x, ag.y)
    };
    if let Some(ag) = sim.agents.get_mut(&c1) {
        let mut nx = x.saturating_add(1);
        let mut ny = y;
        if !sim.world.is_land(nx, ny) {
            nx = x.saturating_sub(1);
        }
        if !sim.world.is_land(nx, ny) {
            ny = y.saturating_add(1);
            nx = x;
        }
        ag.x = nx;
        ag.y = ny;
    }
    let ch = sim.agents.get(&c0).unwrap().clone();
    let legal = legal_actions(&sim, &ch);
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::PairBond { target } if *target == c1))
    );
    fill_energy(&mut sim);
    sim_core::execute::execute_primary(&mut sim, c0, &PrimaryAction::PairBond { target: c1 });
    assert!(matches!(
        sim.events.events.last().map(|e| &e.kind),
        Some(SimEventKind::Wait)
    ));

    sim.agents.get_mut(&a).unwrap().kinship.pair_bond = None;
    sim.agents.get_mut(&b).unwrap().kinship.pair_bond = None;
    let (px, py) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    if let Some(ag) = sim.agents.get_mut(&c0) {
        let mut nx = px.saturating_add(1);
        let ny = py;
        if !sim.world.is_land(nx, ny) || (nx, ny) == (px, py) {
            nx = px.saturating_sub(1);
        }
        ag.x = nx;
        ag.y = ny;
        ag.kinship.pair_bond = None;
    }
    let ch = sim.agents.get(&c0).unwrap().clone();
    let legal = legal_actions(&sim, &ch);
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::PairBond { target } if *target == a))
    );
}

#[test]
fn unrelated_founders_pair_bond_still_legal() {
    let mut sim = Simulation::new(tiny(0x34_07)).unwrap();
    sim.enable_reproduction();
    let (a, _) = place_adjacent(&mut sim);
    let agent = sim.agents.get(&a).unwrap().clone();
    let legal = legal_actions(&sim, &agent);
    assert!(
        legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::PairBond { .. }))
    );
}

#[test]
fn sheet_unused_attack_always_hits_pocket_cap_16() {
    let mut sim = Simulation::new(tiny(0x41_10)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    let cap = sim.agents.get(&a).unwrap().pocket_slot_cap();
    assert_eq!(cap, 16);
    assert_eq!(sim.agents.get(&a).unwrap().inventory_cap, 16);
    sim.agents.get_mut(&b).unwrap().health = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().rev().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(2000), "unused DEX always hits");
}

#[test]
fn dex_18_can_miss_dex_0_always_hit() {
    let mut sim = Simulation::new(tiny(0x41_11)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&b).unwrap().sheet.dexterity = 18;
    let mut missed = false;
    for t in 0..40 {
        sim.tick = t;
        fill_energy(&mut sim);
        sim.agents.get_mut(&b).unwrap().health = 10_000;
        sim.agents.get_mut(&b).unwrap().needs.energy = sim.config.energy_max_milli();
        sim.agents.get_mut(&b).unwrap().incapacitated = false;
        sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
        if sim.events.events.iter().any(|e| {
            matches!(
                e.kind,
                SimEventKind::Attack { target, damage } if target == b && damage == 0
            )
        }) {
            missed = true;
            break;
        }
    }
    assert!(missed, "DEX 18 must miss on some locked tick");

    sim.agents.get_mut(&b).unwrap().sheet.dexterity = 0;
    fill_energy(&mut sim);
    sim.agents.get_mut(&b).unwrap().health = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Attack { target, damage } if target == b && damage == 2000
    )));
}

#[test]
fn str_18_vs_3_pocket_and_pack_caps_crate_unchanged() {
    let mut sim = Simulation::new(tiny(0x41_12)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet.strength = 18;
    assert_eq!(sim.agents.get(&id).unwrap().pocket_slot_cap(), 20);
    assert_eq!(sim.agents.get(&id).unwrap().inventory_cap, 16);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Basket, 1);
    let (slots_hi, w_hi) = sim.agents.get(&id).unwrap().worn_pack_caps(&sim.storage);
    sim.agents.get_mut(&id).unwrap().sheet.strength = 3;
    assert_eq!(sim.agents.get(&id).unwrap().pocket_slot_cap(), 13);
    let (slots_lo, w_lo) = sim.agents.get(&id).unwrap().worn_pack_caps(&sim.storage);
    assert!(slots_hi > slots_lo, "{slots_hi} vs {slots_lo}");
    assert!(w_hi > w_lo, "{w_hi} vs {w_lo}");
    assert_eq!(sim.storage.slot_cap, sim_core::haul::SLOT_CAP);
    assert_eq!(
        sim.storage.weight_cap_milli,
        sim_core::haul::WEIGHT_CAP_MILLI
    );
}

#[test]
fn load_restores_sheet_not_derived_cap() {
    let mut sim = Simulation::new(tiny(0x41_13)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet.strength = 18;
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Fiber, 2);
    assert_eq!(sim.agents.get(&id).unwrap().inventory_cap, 16);
    assert_eq!(sim.agents.get(&id).unwrap().pocket_slot_cap(), 20);
    let qty = sim
        .agents
        .get(&id)
        .unwrap()
        .inventory
        .get(&ItemId::Fiber)
        .copied()
        .unwrap_or(0);
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    let a = loaded.agents.get(&id).unwrap();
    assert_eq!(a.sheet.strength, 18);
    assert_eq!(a.inventory_cap, 16);
    assert_eq!(a.pocket_slot_cap(), 20);
    assert_eq!(a.inventory.get(&ItemId::Fiber).copied().unwrap_or(0), qty);
}

#[test]
fn con_18_vs_3_energy_max_and_illness() {
    let mut sim = Simulation::new(tiny(0x42_01)).unwrap();
    let id = AgentId(0);
    let base = sim.config.energy_max_milli();
    sim.agents.get_mut(&id).unwrap().sheet.constitution = 18;
    assert_eq!(
        sim.agents.get(&id).unwrap().sheet.energy_max(base),
        base + 2000
    );
    sim.agents.get_mut(&id).unwrap().needs.energy = base;
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Rest);
    let hi = sim.agents.get(&id).unwrap().needs.energy;
    sim.agents.get_mut(&id).unwrap().sheet.constitution = 3;
    sim.agents.get_mut(&id).unwrap().needs.energy = base;
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Rest);
    let lo = sim.agents.get(&id).unwrap().needs.energy;
    assert!(hi > lo, "CON 18 energy {hi} vs CON 3 {lo}");
    assert_eq!(lo, sim.agents.get(&id).unwrap().sheet.energy_max(base));

    sim.agents.get_mut(&id).unwrap().sheet.constitution = 18;
    assert_eq!(sim.agents.get(&id).unwrap().sheet.illness_duration(), 8);
    sim.agents.get_mut(&id).unwrap().sheet.constitution = 3;
    assert_eq!(sim.agents.get(&id).unwrap().sheet.illness_duration(), 15);
}

#[test]
fn int_18_vs_3_memory_and_retrieval() {
    let sim = Simulation::new(tiny(0x42_02)).unwrap();
    let id = AgentId(0);
    let base_cap = sim.config.memory_capacity();
    let base_k = sim.config.agents.memory.retrieval_k;
    assert_eq!(base_cap, 128);
    assert_eq!(base_k, 8);
    let mut a = sim.agents.get(&id).unwrap().clone();
    a.sheet.intelligence = 18;
    assert_eq!(a.sheet.memory_cap(base_cap), 144);
    assert_eq!(a.sheet.retrieval_k(base_k), 12);
    a.sheet.intelligence = 3;
    assert_eq!(a.sheet.memory_cap(base_cap), 116);
    assert_eq!(a.sheet.retrieval_k(base_k), 5);
}

#[test]
fn unused_sheet_energy_memory_identity() {
    let mut sim = Simulation::new(tiny(0x42_03)).unwrap();
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert!(a.sheet.is_unused());
    assert_eq!(
        a.sheet.energy_max(sim.config.energy_max_milli()),
        sim.config.energy_max_milli()
    );
    assert_eq!(a.sheet.illness_duration(), 12);
    assert_eq!(a.sheet.memory_cap(sim.config.memory_capacity()), 128);
    assert_eq!(a.sheet.retrieval_k(sim.config.agents.memory.retrieval_k), 8);
    assert_eq!(a.sheet.plan_length(4), 4);
    assert_eq!(a.sheet.resource_qty(3), 3);
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Food(mushroom), 1);
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Eat {
            item: ItemId::Food(mushroom),
        },
    );
    assert_eq!(sim.agents.get(&id).unwrap().illness_ticks, 12);
}

#[test]
fn load_restores_con_int_not_derived() {
    let mut sim = Simulation::new(tiny(0x42_04)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet.constitution = 18;
    sim.agents.get_mut(&id).unwrap().sheet.intelligence = 18;
    sim.agents.get_mut(&id).unwrap().illness_ticks = 8;
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    let a = loaded.agents.get(&id).unwrap();
    assert_eq!(a.sheet.constitution, 18);
    assert_eq!(a.sheet.intelligence, 18);
    assert_eq!(a.illness_ticks, 8);
    assert_eq!(
        a.sheet.energy_max(loaded.config.energy_max_milli()),
        loaded.config.energy_max_milli() + 2000
    );
    assert_eq!(a.sheet.memory_cap(loaded.config.memory_capacity()), 144);
}

fn land_pair_at_dist(sim: &Simulation, dist: u32) -> (u32, u32, u32, u32) {
    for y in 0..sim.world.height {
        for x in 0..sim.world.width {
            if !sim.world.is_land(x, y) {
                continue;
            }
            for y2 in 0..sim.world.height {
                for x2 in 0..sim.world.width {
                    if sim.world.is_land(x2, y2) && observation::chebyshev(x, y, x2, y2) == dist {
                        return (x, y, x2, y2);
                    }
                }
            }
        }
    }
    panic!("no land pair at chebyshev {dist}");
}

fn land_line(sim: &Simulation, len: u32) -> (u32, u32, i32, i32) {
    for y in 0..sim.world.height {
        for x in 0..sim.world.width {
            if !sim.world.is_land(x, y) {
                continue;
            }
            if (0..len).all(|i| {
                let nx = x as i32 + i as i32;
                sim.world.in_bounds(nx, y as i32) && sim.world.is_land(nx as u32, y)
            }) {
                return (x, y, 1, 0);
            }
            if (0..len).all(|i| {
                let ny = y as i32 + i as i32;
                sim.world.in_bounds(x as i32, ny) && sim.world.is_land(x, ny as u32)
            }) {
                return (x, y, 0, 1);
            }
        }
    }
    panic!("no land line of length {len}");
}

#[test]
fn unused_sheet_wis_cha_dex_identity() {
    let mut sim = Simulation::new(tiny(0x43_00)).unwrap();
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert!(a.sheet.is_unused());
    assert!(!a.sheet.detects_toxins());
    assert_eq!(a.sheet.adjust_speech_range(18), 18);
    assert_eq!(a.sheet.speech_importance(50), 50);
    assert_eq!(a.sheet.speak_affinity(), 50);
    assert_eq!(a.sheet.flee_steps(), 1);
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    let (x, y) = {
        let a = sim.agents.get(&id).unwrap();
        (a.x, a.y)
    };
    sim.world.set_vegetation(x, y, mushroom);
    sim.agents.get_mut(&id).unwrap().memory.clear();
    let obs = observation::build(&sim, id);
    assert!(!obs.toxins.iter().any(|t| t == "mushroom"));
    let filtered = policy::avoid_toxic(
        &obs,
        &sim.agents.get(&id).unwrap().memory,
        &sim.config.world.species,
    );
    assert!(
        filtered
            .legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Gather { species } if *species == mushroom))
    );
    assert!(
        !sim.agents
            .get(&id)
            .unwrap()
            .memory
            .iter()
            .any(|m| m.kind == MemoryKind::ToxinFact)
    );
}

#[test]
fn wis_18_vs_0_detects_visible_toxin() {
    let mut sim = Simulation::new(tiny(0x43_01)).unwrap();
    let id = AgentId(0);
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    let nightshade = sim
        .config
        .world
        .species
        .veg_tag_by_id("nightshade")
        .unwrap();
    let (x, y) = {
        let a = sim.agents.get(&id).unwrap();
        (a.x, a.y)
    };
    sim.world.set_vegetation(x, y, mushroom);
    for (dx, dy) in [(1i32, 0), (0, 1), (-1, 0), (0, -1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if sim.world.in_bounds(nx, ny) && sim.world.is_land(nx as u32, ny as u32) {
            sim.world.set_vegetation(nx as u32, ny as u32, nightshade);
            break;
        }
    }
    sim.agents.get_mut(&id).unwrap().memory.clear();
    sim.agents.get_mut(&id).unwrap().sheet.wisdom = 18;
    let obs = observation::build(&sim, id);
    assert!(obs.toxins.iter().any(|t| t == "mushroom"));
    assert!(!obs.toxins.iter().any(|t| t == "nightshade"));
    let mem = sim.agents.get(&id).unwrap().memory.clone();
    let filtered = policy::avoid_toxic(&obs, &mem, &sim.config.world.species);
    assert!(
        !filtered
            .legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Gather { species } if *species == mushroom))
    );
    assert!(!mem.iter().any(|m| m.kind == MemoryKind::ToxinFact));

    sim.agents.get_mut(&id).unwrap().sheet.wisdom = 0;
    let obs0 = observation::build(&sim, id);
    assert!(!obs0.toxins.iter().any(|t| t == "mushroom"));
    let f0 = policy::avoid_toxic(
        &obs0,
        &sim.agents.get(&id).unwrap().memory,
        &sim.config.world.species,
    );
    assert!(
        f0.legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Gather { species } if *species == mushroom))
    );
    assert!(
        !sim.agents
            .get(&id)
            .unwrap()
            .memory
            .iter()
            .any(|m| m.kind == MemoryKind::ToxinFact)
    );
}

#[test]
fn wis_10_toxin_same_as_unused() {
    let mut sim = Simulation::new(tiny(0x43_02)).unwrap();
    let id = AgentId(0);
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    let (x, y) = {
        let a = sim.agents.get(&id).unwrap();
        (a.x, a.y)
    };
    sim.world.set_vegetation(x, y, mushroom);
    sim.agents.get_mut(&id).unwrap().memory.clear();
    sim.agents.get_mut(&id).unwrap().sheet.wisdom = 10;
    assert!(!sim.agents.get(&id).unwrap().sheet.detects_toxins());
    let obs = observation::build(&sim, id);
    assert!(!obs.toxins.iter().any(|t| t == "mushroom"));
    let filtered = policy::avoid_toxic(
        &obs,
        &sim.agents.get(&id).unwrap().memory,
        &sim.config.world.species,
    );
    assert!(
        filtered
            .legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Gather { species } if *species == mushroom))
    );
}

#[test]
fn cha_18_vs_3_speech_range_and_weight() {
    let mut sim = Simulation::new(tiny(0x43_03)).unwrap();
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let speaker = ids[0];
    let listener = ids[1];
    for id in [speaker, listener] {
        let a = sim.agents.get_mut(&id).unwrap();
        a.personality.perceptiveness = 50;
        a.sheet.wisdom = 0;
        a.memory.clear();
        a.relationships.clear();
    }
    let (sx, sy, lx, ly) = land_pair_at_dist(&sim, 1);
    sim.agents.get_mut(&speaker).unwrap().x = sx;
    sim.agents.get_mut(&speaker).unwrap().y = sy;
    sim.agents.get_mut(&listener).unwrap().x = lx;
    sim.agents.get_mut(&listener).unwrap().y = ly;

    sim.agents.get_mut(&speaker).unwrap().sheet.charisma = 18;
    sim.apply_speak(
        speaker,
        Speak {
            to: SpeakTarget::Broadcast,
            shout: false,
            text: "hello".into(),
        },
    );
    assert_eq!(
        sim.agents
            .get(&speaker)
            .unwrap()
            .relationships
            .get(&listener)
            .map(|r| r.affinity),
        Some(90)
    );
    sim.tick = 1;
    let obs = observation::build(&sim, listener);
    assert!(obs.heard.iter().any(|h| h.text == "hello"));
    sim_core::execute::apply_heard_memories(&mut sim, listener, &obs.heard);
    let imp18 = sim
        .agents
        .get(&listener)
        .unwrap()
        .memory
        .iter()
        .find(|m| m.kind == MemoryKind::Utterance && m.text == "hello")
        .map(|m| m.importance)
        .expect("utterance");
    assert_eq!(imp18, 70);

    let mut sim = Simulation::new(tiny(0x43_03)).unwrap();
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let speaker = ids[0];
    let listener = ids[1];
    for id in [speaker, listener] {
        let a = sim.agents.get_mut(&id).unwrap();
        a.personality.perceptiveness = 50;
        a.sheet.wisdom = 0;
        a.memory.clear();
        a.relationships.clear();
    }
    let (sx, sy, lx, ly) = land_pair_at_dist(&sim, 1);
    sim.agents.get_mut(&speaker).unwrap().x = sx;
    sim.agents.get_mut(&speaker).unwrap().y = sy;
    sim.agents.get_mut(&listener).unwrap().x = lx;
    sim.agents.get_mut(&listener).unwrap().y = ly;
    sim.agents.get_mut(&speaker).unwrap().sheet.charisma = 3;
    sim.apply_speak(
        speaker,
        Speak {
            to: SpeakTarget::Broadcast,
            shout: false,
            text: "hello".into(),
        },
    );
    assert_eq!(
        sim.agents
            .get(&speaker)
            .unwrap()
            .relationships
            .get(&listener)
            .map(|r| r.affinity),
        Some(20)
    );
    sim.tick = 1;
    let obs = observation::build(&sim, listener);
    sim_core::execute::apply_heard_memories(&mut sim, listener, &obs.heard);
    let imp3 = sim
        .agents
        .get(&listener)
        .unwrap()
        .memory
        .iter()
        .find(|m| m.kind == MemoryKind::Utterance && m.text == "hello")
        .map(|m| m.importance)
        .expect("utterance");
    assert_eq!(imp3, 35);

    let mut far = Simulation::new(tiny(0x43_03)).unwrap();
    let ids: Vec<AgentId> = far.agents.keys().copied().collect();
    let speaker = ids[0];
    let listener = ids[1];
    for id in [speaker, listener] {
        let a = far.agents.get_mut(&id).unwrap();
        a.personality.perceptiveness = 50;
        a.sheet.wisdom = 0;
    }
    let (sx, sy, lx, ly) = land_pair_at_dist(&far, 17);
    far.agents.get_mut(&speaker).unwrap().x = sx;
    far.agents.get_mut(&speaker).unwrap().y = sy;
    far.agents.get_mut(&listener).unwrap().x = lx;
    far.agents.get_mut(&listener).unwrap().y = ly;
    far.agents.get_mut(&speaker).unwrap().sheet.charisma = 18;
    far.apply_speak(
        speaker,
        Speak {
            to: SpeakTarget::Broadcast,
            shout: false,
            text: "carry".into(),
        },
    );
    far.tick = 1;
    let heard18 = observation::build(&far, listener)
        .heard
        .iter()
        .any(|h| h.text == "carry");
    far.agents.get_mut(&speaker).unwrap().sheet.charisma = 3;
    far.tick = 0;
    far.events.events.clear();
    far.apply_speak(
        speaker,
        Speak {
            to: SpeakTarget::Broadcast,
            shout: false,
            text: "carry".into(),
        },
    );
    far.tick = 1;
    let heard3 = observation::build(&far, listener)
        .heard
        .iter()
        .any(|h| h.text == "carry");
    assert!(heard18, "CHA 18 should be heard at dist 17");
    assert!(!heard3, "CHA 3 should not be heard at dist 17");
}

#[test]
fn dex_18_vs_0_flee_steps() {
    let mut sim = Simulation::new(tiny(0x43_04)).unwrap();
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let fleer = ids[0];
    let threat = ids[1];
    let (x, y, dx, dy) = land_line(&sim, 5);
    let fx = (x as i32 + dx) as u32;
    let fy = (y as i32 + dy) as u32;
    sim.agents.get_mut(&threat).unwrap().x = x;
    sim.agents.get_mut(&threat).unwrap().y = y;
    sim.agents.get_mut(&fleer).unwrap().x = fx;
    sim.agents.get_mut(&fleer).unwrap().y = fy;
    sim.conflict_enabled = true;
    fill_energy(&mut sim);
    let emax = sim.agents.get(&fleer).unwrap().needs.energy;
    let cost0 = sim
        .agents
        .get(&fleer)
        .unwrap()
        .move_cost_milli(&sim.storage);
    sim.agents.get_mut(&fleer).unwrap().sheet.dexterity = 0;
    sim_core::execute::execute_primary(&mut sim, fleer, &PrimaryAction::Flee);
    let a = sim.agents.get(&fleer).unwrap();
    assert_eq!(a.x, (fx as i32 + dx) as u32);
    assert_eq!(a.y, (fy as i32 + dy) as u32);
    assert_eq!(emax - a.needs.energy, cost0);
    let flees = sim
        .events
        .events
        .iter()
        .filter(|e| e.agent == fleer && matches!(e.kind, SimEventKind::Flee))
        .count();
    let moves = sim
        .events
        .events
        .iter()
        .filter(|e| e.agent == fleer && matches!(e.kind, SimEventKind::Move { .. }))
        .count();
    assert_eq!(flees, 1);
    assert_eq!(moves, 0);

    sim.agents.get_mut(&fleer).unwrap().x = fx;
    sim.agents.get_mut(&fleer).unwrap().y = fy;
    fill_energy(&mut sim);
    sim.agents.get_mut(&fleer).unwrap().sheet.dexterity = 18;
    let emax = sim.agents.get(&fleer).unwrap().needs.energy;
    let cost18 = sim
        .agents
        .get(&fleer)
        .unwrap()
        .move_cost_milli(&sim.storage);
    sim.events.events.clear();
    sim_core::execute::execute_primary(&mut sim, fleer, &PrimaryAction::Flee);
    let a = sim.agents.get(&fleer).unwrap();
    assert_eq!(a.x, (fx as i32 + dx * 3) as u32);
    assert_eq!(a.y, (fy as i32 + dy * 3) as u32);
    assert_eq!(emax - a.needs.energy, cost18);
    let flees = sim
        .events
        .events
        .iter()
        .filter(|e| e.agent == fleer && matches!(e.kind, SimEventKind::Flee))
        .count();
    let moves = sim
        .events
        .events
        .iter()
        .filter(|e| e.agent == fleer && matches!(e.kind, SimEventKind::Move { .. }))
        .count();
    assert_eq!(flees, 1);
    assert_eq!(moves, 0);
}

#[test]
fn load_restores_wis_cha_dex_not_derived() {
    let mut sim = Simulation::new(tiny(0x43_05)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet.wisdom = 18;
    sim.agents.get_mut(&id).unwrap().sheet.charisma = 18;
    sim.agents.get_mut(&id).unwrap().sheet.dexterity = 18;
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    let a = loaded.agents.get(&id).unwrap();
    assert_eq!(a.sheet.wisdom, 18);
    assert_eq!(a.sheet.charisma, 18);
    assert_eq!(a.sheet.dexterity, 18);
    assert!(a.sheet.detects_toxins());
    assert_eq!(a.sheet.adjust_speech_range(18), 22);
    assert_eq!(a.sheet.speech_importance(50), 70);
    assert_eq!(a.sheet.speak_affinity(), 90);
    assert_eq!(a.sheet.flee_steps(), 3);
}

#[test]
fn unused_sheet_board_support_pocket_identity() {
    let mut sim = Simulation::new(tiny(0x44_00)).unwrap();
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert!(a.sheet.is_unused());
    assert_eq!(a.sheet.board_cells(8), 8);
    assert_eq!(a.sheet.support_social(), (200, 0, 100, 0));
    assert_eq!(a.sheet.pocket_weight_cap(), None);
    assert_eq!(a.pocket_fit_qty(ItemId::Stone), 16);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Stone, 16);
    assert_eq!(
        sim.agents.get(&id).unwrap().inventory.get(&ItemId::Stone),
        Some(&16)
    );
}

#[test]
fn wis_18_vs_0_board_past_ident() {
    let mut sim = Simulation::new(tiny(0x44_01)).unwrap();
    sim.config.proposals.public_board_always_visible = false;
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let author = ids[0];
    let viewer = ids[1];
    for id in [author, viewer] {
        sim.agents.get_mut(&id).unwrap().personality.perceptiveness = 50;
        sim.agents.get_mut(&id).unwrap().sheet.wisdom = 0;
    }
    let (ax, ay, vx, vy) = land_pair_at_dist(&sim, 14);
    sim.agents.get_mut(&author).unwrap().x = ax;
    sim.agents.get_mut(&author).unwrap().y = ay;
    sim.agents.get_mut(&viewer).unwrap().x = vx;
    sim.agents.get_mut(&viewer).unwrap().y = vy;
    sim_core::execute::execute_primary(
        &mut sim,
        author,
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: None,
        },
    );
    let ident0 =
        observation::perceive_range(sim.config.observation.base_agent_identity_range, 50, 0);
    assert_eq!(ident0, 8);
    sim.agents.get_mut(&viewer).unwrap().sheet.wisdom = 18;
    let obs18 = observation::build(&sim, viewer);
    let post = obs18
        .board
        .iter()
        .find(|p| p.text.contains("mushroom"))
        .expect("WIS 18 sees post");
    assert!(post.author.is_none(), "past ident: unnamed author");
    sim.agents.get_mut(&viewer).unwrap().sheet.wisdom = 0;
    let obs0 = observation::build(&sim, viewer);
    assert!(obs0.board.iter().all(|p| !p.text.contains("mushroom")));
    sim.agents.get_mut(&viewer).unwrap().sheet.wisdom = 10;
    let obs10 = observation::build(&sim, viewer);
    assert!(obs10.board.iter().all(|p| !p.text.contains("mushroom")));
}

#[test]
fn cha_18_vs_3_support_social() {
    let mut sim = Simulation::new(tiny(0x44_02)).unwrap();
    let (a, b) = place_adjacent(&mut sim);
    sim_core::execute::execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Propose {
            text: "rule".into(),
            rule: None,
        },
    );
    let pid = sim.board.proposals[0].id;
    sim.agents.get_mut(&b).unwrap().sheet.charisma = 18;
    sim.agents.get_mut(&b).unwrap().relationships.clear();
    sim_core::execute::execute_primary(&mut sim, b, &PrimaryAction::Support { proposal_id: pid });
    assert!(sim.board.proposals[0].supporters.contains(&b));
    assert_eq!(
        sim.board.proposals[0].supporters.len(),
        2,
        "author plus one Support id, not CHA-weighted extra votes"
    );
    assert_eq!(
        sim.agents
            .get(&b)
            .unwrap()
            .relationships
            .get(&a)
            .map(|r| (r.trust, r.respect)),
        Some((400, 200))
    );

    let mut sim = Simulation::new(tiny(0x44_02)).unwrap();
    let (a, b) = place_adjacent(&mut sim);
    sim_core::execute::execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Propose {
            text: "rule".into(),
            rule: None,
        },
    );
    let pid = sim.board.proposals[0].id;
    sim.agents.get_mut(&b).unwrap().sheet.charisma = 3;
    sim.agents.get_mut(&b).unwrap().relationships.clear();
    sim_core::execute::execute_primary(&mut sim, b, &PrimaryAction::Support { proposal_id: pid });
    assert_eq!(sim.board.proposals[0].supporters.len(), 2);
    assert_eq!(
        sim.agents
            .get(&b)
            .unwrap()
            .relationships
            .get(&a)
            .map(|r| (r.trust, r.respect)),
        Some((50, 25))
    );
}

#[test]
fn cha_0_pair_bond_always_if_legal() {
    let mut sim = Simulation::new(tiny(0x44_03)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    sim.agents.get_mut(&a).unwrap().sheet.charisma = 0;
    assert_eq!(sim.agents.get(&a).unwrap().sheet.charisma, 0);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    assert_eq!(sim.agents.get(&a).unwrap().kinship.pair_bond, Some(b));
    assert_eq!(sim.agents.get(&b).unwrap().kinship.pair_bond, Some(a));
}

#[test]
fn cha_18_vs_3_pair_bond_odds() {
    let mut sim = Simulation::new(tiny(0x44_04)).unwrap();
    sim.enable_reproduction();
    let (a, b) = place_adjacent(&mut sim);
    fill_energy(&mut sim);
    let master = sim.config.master_seed;
    sim.agents.get_mut(&a).unwrap().sheet.charisma = 18;
    let mut hits18 = 0u32;
    for t in 0..40 {
        sim.tick = t;
        sim.agents.get_mut(&a).unwrap().kinship.pair_bond = None;
        sim.agents.get_mut(&b).unwrap().kinship.pair_bond = None;
        sim.agents.get_mut(&a).unwrap().kinship.household = None;
        sim.agents.get_mut(&b).unwrap().kinship.household = None;
        sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
        if sim.agents.get(&a).unwrap().kinship.pair_bond == Some(b) {
            hits18 += 1;
        }
        assert_eq!(
            sim.agents.get(&a).unwrap().kinship.pair_bond.is_some(),
            AbilitySheet::pair_bond_hits(master, t, a.0, 18)
        );
    }
    sim.agents.get_mut(&a).unwrap().sheet.charisma = 3;
    let mut hits3 = 0u32;
    for t in 0..40 {
        sim.tick = t;
        sim.agents.get_mut(&a).unwrap().kinship.pair_bond = None;
        sim.agents.get_mut(&b).unwrap().kinship.pair_bond = None;
        sim.agents.get_mut(&a).unwrap().kinship.household = None;
        sim.agents.get_mut(&b).unwrap().kinship.household = None;
        sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
        if sim.agents.get(&a).unwrap().kinship.pair_bond == Some(b) {
            hits3 += 1;
        }
    }
    assert!(hits18 > hits3, "CHA 18 hits {hits18} vs CHA 3 {hits3}");
    assert!(hits18 >= 20, "CHA 18 should succeed often");
}

#[test]
fn str_18_vs_3_pocket_weight() {
    let mut sim = Simulation::new(tiny(0x44_05)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().inventory_cap = 32;
    sim.agents.get_mut(&id).unwrap().sheet.strength = 3;
    assert_eq!(
        sim.agents.get(&id).unwrap().sheet.pocket_weight_cap(),
        Some(7250)
    );
    let n = sim
        .agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Stone, 25);
    assert_eq!(
        n, 24,
        "STR 3 cap 7250 holds 24 stones (7200), not 25 (7500)"
    );
    sim.agents.get_mut(&id).unwrap().inventory.clear();
    sim.agents.get_mut(&id).unwrap().sheet.strength = 18;
    assert_eq!(
        sim.agents.get(&id).unwrap().sheet.pocket_weight_cap(),
        Some(9000)
    );
    let n = sim
        .agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Stone, 31);
    assert_eq!(n, 30, "STR 18 cap 9000 holds 30 stones (9000), not 31");
}

#[test]
fn load_restores_wis_cha_str_not_derived() {
    let mut sim = Simulation::new(tiny(0x44_06)).unwrap();
    let id = AgentId(0);
    sim.agents.get_mut(&id).unwrap().sheet.wisdom = 18;
    sim.agents.get_mut(&id).unwrap().sheet.charisma = 18;
    sim.agents.get_mut(&id).unwrap().sheet.strength = 18;
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    let a = loaded.agents.get(&id).unwrap();
    assert_eq!(a.sheet.wisdom, 18);
    assert_eq!(a.sheet.charisma, 18);
    assert_eq!(a.sheet.strength, 18);
    assert_eq!(a.sheet.board_cells(8), 12);
    assert_eq!(a.sheet.support_social(), (400, 0, 200, 0));
    assert_eq!(a.sheet.pocket_weight_cap(), Some(9000));
}

#[test]
fn con_18_vs_3_illness_chance() {
    let mushroom = {
        let sim = Simulation::new(tiny(0x46_01)).unwrap();
        sim.config.world.species.veg_tag_by_id("mushroom").unwrap()
    };
    let eat_n = |con: u8| -> u32 {
        let mut sim = Simulation::new(tiny(0x46_01)).unwrap();
        let id = AgentId(0);
        sim.agents.get_mut(&id).unwrap().sheet.constitution = con;
        let mut sick = 0u32;
        for t in 1..40u64 {
            sim.tick = t;
            sim.agents.get_mut(&id).unwrap().illness_ticks = 0;
            sim.agents
                .get_mut(&id)
                .unwrap()
                .try_add_item(ItemId::Food(mushroom), 1);
            sim_core::execute::execute_primary(
                &mut sim,
                id,
                &PrimaryAction::Eat {
                    item: ItemId::Food(mushroom),
                },
            );
            if sim.agents.get(&id).unwrap().illness_ticks > 0 {
                sick += 1;
            }
        }
        sick
    };
    let always = eat_n(0);
    assert_eq!(always, 39, "CON 0 always sick");
    let hi = eat_n(18);
    let lo = eat_n(3);
    assert!(hi < lo, "CON 18 sick {hi} vs CON 3 {lo}");
}

#[test]
fn int_18_vs_3_plan_length() {
    let sim = Simulation::new(tiny(0x46_02)).unwrap();
    let base = sim.llm_plan_length;
    assert_eq!(base, 4);
    let mut a = sim.agents.get(&AgentId(0)).unwrap().clone();
    assert_eq!(a.sheet.plan_length(base), 4);
    a.sheet.intelligence = 18;
    assert_eq!(a.sheet.plan_length(base), 8);
    a.sheet.intelligence = 3;
    assert_eq!(a.sheet.plan_length(base), 1);
}

#[test]
fn str_18_vs_3_gather_qty() {
    let tree = {
        let sim = Simulation::new(tiny(0x46_03)).unwrap();
        sim.config.world.species.veg_tag_by_id("tree").unwrap()
    };
    let gather_wood = |str_: u8| -> u32 {
        let mut sim = Simulation::new(tiny(0x46_03)).unwrap();
        let id = AgentId(0);
        let (x, y) = {
            let a = sim.agents.get(&id).unwrap();
            (a.x, a.y)
        };
        let mut placed = false;
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let (nx, ny) = (nx as u32, ny as u32);
            if sim.world.is_land(nx, ny) {
                sim.world.set_vegetation(nx, ny, tree);
                placed = true;
                break;
            }
        }
        assert!(placed, "no adjacent land for tree");
        sim.agents.get_mut(&id).unwrap().sheet.strength = str_;
        sim.agents.get_mut(&id).unwrap().needs =
            sim_core::Needs::maxed(10_000, 10_000, 10_000);
        let before = sim
            .agents
            .get(&id)
            .unwrap()
            .inventory
            .get(&ItemId::Wood)
            .copied()
            .unwrap_or(0);
        for _ in 0..64 {
            sim_core::execute::execute_primary(
                &mut sim,
                id,
                &PrimaryAction::Gather { species: tree },
            );
            let have = sim
                .agents
                .get(&id)
                .unwrap()
                .inventory
                .get(&ItemId::Wood)
                .copied()
                .unwrap_or(0);
            if have > before {
                return have - before;
            }
            if let Some(a) = sim.agents.get(&id) {
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = a.x as i32 + dx;
                    let ny = a.y as i32 + dy;
                    if nx >= 0 && ny >= 0 {
                        sim.world.set_vegetation(nx as u32, ny as u32, tree);
                    }
                }
            }
        }
        panic!("gather wood never succeeded");
    };
    let hi = gather_wood(18);
    let lo = gather_wood(3);
    let unused = gather_wood(0);
    assert_eq!(unused, 3, "unused STR keeps tree wood_yield");
    assert!(hi > lo, "STR 18 wood {hi} vs STR 3 {lo}");
}
