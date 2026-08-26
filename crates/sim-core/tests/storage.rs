use sim_core::action::PrimaryAction;
use sim_core::event_log::SimEventKind;
use sim_core::execute::execute_primary;
use sim_core::markers;
use sim_core::observation;
use sim_core::{AgentId, ExperimentConfig, ItemId, Simulation};

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

fn park_adjacent(sim: &mut Simulation, a: AgentId, b: AgentId) {
    let land = sim.world.land_cells();
    let (x, y) = land[0];
    let neigh = observation::neighbors4(&sim.world, x, y)
        .into_iter()
        .find(|&(nx, ny)| (nx != x || ny != y) && sim.world.is_land(nx, ny))
        .expect("neighbor");
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = x;
        ag.y = y;
        ag.needs.energy = sim.config.energy_max_milli();
        ag.needs.hunger = sim.config.hunger_max_milli();
        ag.needs.thirst = sim.config.thirst_max_milli();
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = neigh.0;
        ag.y = neigh.1;
        ag.needs.energy = sim.config.energy_max_milli();
        ag.needs.hunger = sim.config.hunger_max_milli() / 4;
        ag.needs.thirst = sim.config.thirst_max_milli();
        ag.inventory.clear();
    }
}

#[test]
fn transfer_adjacent_identified() {
    let mut sim = Simulation::new(tiny(0xA100)).unwrap();
    sim.config.observation.full_information = true;
    let a = AgentId(0);
    let b = AgentId(1);
    park_adjacent(&mut sim, a, b);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.inventory.clear();
        ag.try_add_item(ItemId::Food(1), 2);
    }
    let energy_before = sim.agents.get(&a).unwrap().needs.energy;
    execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Transfer {
            item: ItemId::Food(1),
            qty: 1,
            to: b,
        },
    );
    assert_eq!(
        sim.agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Food(1))
            .copied()
            .unwrap_or(0),
        1
    );
    assert_eq!(
        sim.agents
            .get(&b)
            .unwrap()
            .inventory
            .get(&ItemId::Food(1))
            .copied()
            .unwrap_or(0),
        1
    );
    assert!(sim.agents.get(&a).unwrap().needs.energy < energy_before);
    assert!(
        sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::Transfer { qty: 1, to, .. } if to == b))
    );
}

#[test]
fn transfer_far_waits() {
    let mut sim = Simulation::new(tiny(0xA101)).unwrap();
    let a = AgentId(0);
    let b = AgentId(1);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = 2;
        ag.y = 2;
        ag.inventory.clear();
        ag.try_add_item(ItemId::Food(1), 1);
        ag.needs.energy = sim.config.energy_max_milli();
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = 20;
        ag.y = 20;
    }
    execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Transfer {
            item: ItemId::Food(1),
            qty: 1,
            to: b,
        },
    );
    assert!(matches!(
        sim.events.events.last().map(|e| &e.kind),
        Some(SimEventKind::Wait)
    ));
    assert_eq!(
        sim.agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Food(1))
            .copied()
            .unwrap_or(0),
        1
    );
}

#[test]
fn store_then_retrieve_pays_energy() {
    let mut sim = Simulation::new(tiny(0xA102)).unwrap();
    let id = AgentId(0);
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = land.0;
        a.y = land.1;
        a.inventory.clear();
        a.try_add_item(ItemId::Food(1), 2);
        a.needs.energy = sim.config.energy_max_milli();
    }
    let e0 = sim.agents.get(&id).unwrap().needs.energy;
    execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Store {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    assert!(sim.world.has_stockpile(land.0, land.1));
    let e1 = sim.agents.get(&id).unwrap().needs.energy;
    assert!(e1 < e0);
    execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Retrieve {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    let e2 = sim.agents.get(&id).unwrap().needs.energy;
    assert!(e2 < e1);
    assert!(!sim.world.has_stockpile(land.0, land.1));
    assert_eq!(
        sim.agents
            .get(&id)
            .unwrap()
            .inventory
            .get(&ItemId::Food(1))
            .copied()
            .unwrap_or(0),
        2
    );
}

#[test]
fn store_over_weight_cap_illegal() {
    let mut sim = Simulation::new(tiny(0xA103)).unwrap();
    sim.storage.weight_cap_milli = 40; // less than one stone (300)
    let id = AgentId(0);
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = land.0;
        a.y = land.1;
        a.inventory.clear();
        a.try_add_item(ItemId::Stone, 1);
        a.needs.energy = sim.config.energy_max_milli();
    }
    let obs = observation::build(&sim, id);
    assert!(!obs.legal.iter().any(|a| matches!(
        a,
        PrimaryAction::Store {
            item: ItemId::Stone,
            ..
        }
    )));
    execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Store {
            item: ItemId::Stone,
            qty: 1,
        },
    );
    assert!(matches!(
        sim.events.events.last().map(|e| &e.kind),
        Some(SimEventKind::Wait)
    ));
}

#[test]
fn store_no_energy_not_legal() {
    let mut sim = Simulation::new(tiny(0xA104)).unwrap();
    let id = AgentId(0);
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = land.0;
        a.y = land.1;
        a.inventory.clear();
        a.try_add_item(ItemId::Stone, 1);
        a.needs.energy = 0;
    }
    let obs = observation::build(&sim, id);
    assert!(
        !obs.legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Store { .. }))
    );
}

#[test]
fn marker_helper_tracks_nonempty() {
    let mut sim = Simulation::new(tiny(0xA105)).unwrap();
    let land = sim.world.land_cells()[0];
    assert!(!sim.world.has_stockpile(land.0, land.1));
    assert!(
        sim.world
            .try_store(land.0, land.1, ItemId::Food(1), 1, &sim.storage)
    );
    assert!(sim.world.has_stockpile(land.0, land.1));
    assert_eq!(markers::marker_stockpile().name, "stockpile");
    assert!(sim.world.try_retrieve(land.0, land.1, ItemId::Food(1), 1));
    assert!(!sim.world.has_stockpile(land.0, land.1));
}

#[test]
fn mock_storage_goal_stores() {
    let mut sim = Simulation::new(tiny(0xA106)).unwrap();
    sim.inject_schedule_toml(
        r#"
[[incentives]]
id = "store_goal"
start_tick = 0
applies_to = "all"
[[incentives.effects]]
type = "goal_injection"
goal_text = "keep the shared storage stocked"
scope = "personal"
priority = 0.9
"#,
    )
    .unwrap();
    let id = AgentId(0);
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = land.0;
        a.y = land.1;
        a.inventory.clear();
        a.try_add_item(ItemId::Food(1), 4);
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.thirst = sim.config.thirst_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
        a.personality.agreeableness = 10;
    }
    sim.run_ticks(8);
    let stores = sim
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::Store { .. }))
        .count();
    assert!(
        stores >= 1,
        "expected Store, events={:?}",
        sim.events.events
    );
}

#[test]
fn give_is_hashed() {
    let mut a = Simulation::new(tiny(0xA107)).unwrap();
    let b = Simulation::new(tiny(0xA107)).unwrap();
    a.give_item(AgentId(0), ItemId::Wood, 1).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn m9_style_world_without_stockpiles_field_still_hashes() {
    let sim = Simulation::new(tiny(0xA108)).unwrap();
    assert!(sim.world.stockpiles.is_empty());
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(sim.state_hash(), loaded.state_hash());
}
