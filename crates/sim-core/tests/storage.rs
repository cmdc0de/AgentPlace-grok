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

fn park_on_land(sim: &mut Simulation, id: AgentId) {
    let land = sim.world.land_cells()[0];
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = land.0;
        a.y = land.1;
        a.needs.energy = sim.config.energy_max_milli();
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.thirst = sim.config.thirst_max_milli();
    }
}

#[test]
fn mock_storage_goal_gathers_then_stores() {
    let mut sim = Simulation::new(tiny(0xA110)).unwrap();
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
    park_on_land(&mut sim, id);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.personality.agreeableness = 10;
    }
    sim.run_ticks(40);
    let stores = sim
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::Store { .. }))
        .count();
    let qty: u32 = sim.world.stockpiles.values().map(|c| c.slot_count()).sum();
    assert!(
        stores >= 1 && qty > 0,
        "expected Gather-then-Store, stores={stores} qty={qty} events={:?}",
        sim.events
            .events
            .iter()
            .map(|e| format!("{:?}", e.kind))
            .collect::<Vec<_>>()
    );
}

#[test]
fn two_stores_same_cell_one_crate() {
    let mut sim = Simulation::new(tiny(0xA111)).unwrap();
    let id = AgentId(0);
    park_on_land(&mut sim, id);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Food(1), 3);
    }
    let land = (
        sim.agents.get(&id).unwrap().x,
        sim.agents.get(&id).unwrap().y,
    );
    execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Store {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Store {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    assert_eq!(sim.world.stockpiles.len(), 1);
    assert_eq!(
        sim.world
            .stockpile_at(land.0, land.1)
            .map(|c| c.slot_count())
            .unwrap_or(0),
        2
    );
}

#[test]
fn pack_then_move_cheaper_than_loose() {
    let mut packed = Simulation::new(tiny(0xA112)).unwrap();
    let mut loose = Simulation::new(tiny(0xA112)).unwrap();
    let id = AgentId(0);
    park_on_land(&mut packed, id);
    park_on_land(&mut loose, id);
    if let Some(a) = packed.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Basket, 1);
        a.try_add_item(ItemId::Food(1), 8);
        a.needs.energy = packed.config.energy_max_milli();
    }
    if let Some(a) = loose.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Food(1), 8);
        a.needs.energy = loose.config.energy_max_milli();
    }
    for _ in 0..8 {
        execute_primary(
            &mut packed,
            id,
            &PrimaryAction::Pack {
                item: ItemId::Food(1),
                qty: 1,
            },
        );
    }
    assert_eq!(packed.agents.get(&id).unwrap().pack_count(), 8);
    let e_pack = packed.agents.get(&id).unwrap().needs.energy;
    let e_loose = loose.agents.get(&id).unwrap().needs.energy;
    let obs = observation::build(&packed, id);
    let mv = obs
        .legal
        .iter()
        .find(|a| matches!(a, PrimaryAction::MoveRelative { .. }))
        .cloned()
        .expect("packed move");
    execute_primary(&mut packed, id, &mv);
    let obs_l = observation::build(&loose, id);
    let mv_l = obs_l
        .legal
        .iter()
        .find(|a| matches!(a, PrimaryAction::MoveRelative { .. }))
        .cloned()
        .expect("loose move");
    execute_primary(&mut loose, id, &mv_l);
    let cost_p = e_pack - packed.agents.get(&id).unwrap().needs.energy;
    let cost_l = e_loose - loose.agents.get(&id).unwrap().needs.energy;
    assert!(
        cost_p < cost_l,
        "packed move {cost_p} should be < loose {cost_l}"
    );
}

#[test]
fn no_basket_same_loose_load_higher_move_cost() {
    let mut sim = Simulation::new(tiny(0xA113)).unwrap();
    let id = AgentId(0);
    park_on_land(&mut sim, id);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Stone, 4);
    }
    let with = sim.agents.get(&id).unwrap().move_cost_milli(&sim.storage);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
    }
    let empty = sim.agents.get(&id).unwrap().move_cost_milli(&sim.storage);
    assert!(
        with > empty,
        "cargo should tax Move (with={with} empty={empty})"
    );
}

#[test]
fn removing_last_basket_folds_pack_into_pockets() {
    let mut sim = Simulation::new(tiny(0xA114)).unwrap();
    let a = AgentId(0);
    let b = AgentId(1);
    park_adjacent(&mut sim, a, b);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.inventory.clear();
        ag.try_add_item(ItemId::Basket, 1);
        ag.try_add_item(ItemId::Food(1), 2);
        ag.needs.energy = sim.config.energy_max_milli();
    }
    execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Pack {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Pack {
            item: ItemId::Food(1),
            qty: 1,
        },
    );
    assert_eq!(sim.agents.get(&a).unwrap().pack_count(), 2);
    execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Transfer {
            item: ItemId::Basket,
            qty: 1,
            to: b,
        },
    );
    let sender = sim.agents.get(&a).unwrap();
    assert!(!sender.has_basket());
    assert_eq!(sender.pack_count(), 0);
    assert_eq!(
        sender.inventory.get(&ItemId::Food(1)).copied().unwrap_or(0),
        2
    );
    assert!(sim.agents.get(&b).unwrap().has_basket());
}

#[test]
fn last_basket_overflow_refuses_transfer() {
    let mut sim = Simulation::new(tiny(0xA115)).unwrap();
    let a = AgentId(0);
    let b = AgentId(1);
    park_adjacent(&mut sim, a, b);
    let params = sim.storage;
    let energy_max = sim.config.energy_max_milli();
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.inventory.clear();
        ag.inventory_cap = 2;
        ag.try_add_item(ItemId::Basket, 1);
        ag.try_add_item(ItemId::Wood, 1);
        ag.needs.energy = energy_max;
        for _ in 0..8 {
            ag.try_add_pack(ItemId::Stone, 1, &params);
        }
    }
    sim.storage.slot_cap = 1;
    sim.storage.weight_cap_milli = 200;
    let obs = observation::build(&sim, a);
    assert!(
        !obs.legal.iter().any(|act| matches!(
            act,
            PrimaryAction::Transfer {
                item: ItemId::Basket,
                ..
            }
        )),
        "last Basket must refuse when pack cannot fold, legal={:?}",
        obs.legal
    );
}

#[test]
fn satchel_helper_tracks_basket() {
    let mut sim = Simulation::new(tiny(0xA116)).unwrap();
    let id = AgentId(0);
    assert!(!sim.agents.get(&id).unwrap().shows_satchel());
    if let Some(a) = sim.agents.get_mut(&id) {
        a.try_add_item(ItemId::Basket, 1);
    }
    assert!(sim.agents.get(&id).unwrap().shows_satchel());
    assert_eq!(markers::marker_satchel().name, "satchel");
    assert_ne!(
        markers::marker_satchel().rgb,
        markers::marker_stockpile().rgb
    );
}

#[test]
fn move_unaffordable_not_legal() {
    let mut sim = Simulation::new(tiny(0xA118)).unwrap();
    let id = AgentId(0);
    park_on_land(&mut sim, id);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Stone, 8);
        a.needs.energy = 1;
    }
    let cost = sim.agents.get(&id).unwrap().move_cost_milli(&sim.storage);
    assert!(cost > 1);
    let obs = observation::build(&sim, id);
    assert!(
        !obs.legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::MoveRelative { .. }))
    );
}

#[test]
fn default_coop_80_tick_fills_crates() {
    let cfg_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let sched_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../configs/incentives/coop.toml");
    let cfg = ExperimentConfig::load_path(&cfg_path).expect("default.toml");
    let mut sim = Simulation::new(cfg).unwrap();
    let toml = std::fs::read_to_string(&sched_path).unwrap();
    sim.inject_schedule_toml(&toml).unwrap();
    sim.run_ticks(80);
    let stores = sim
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::Store { .. }))
        .count();
    let qty: u32 = sim.world.stockpiles.values().map(|c| c.slot_count()).sum();
    assert!(
        stores >= 1 && qty > 0,
        "80-tick default+coop should Store, stores={stores} qty={qty}"
    );
}

#[test]
fn pack_over_weight_cap_illegal() {
    let mut sim = Simulation::new(tiny(0xA117)).unwrap();
    let id = AgentId(0);
    park_on_land(&mut sim, id);
    sim.storage.pack_weight_cap_milli = 40;
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Basket, 1);
        a.try_add_item(ItemId::Stone, 1);
        a.needs.energy = sim.config.energy_max_milli();
    }
    let obs = observation::build(&sim, id);
    assert!(!obs.legal.iter().any(|act| matches!(
        act,
        PrimaryAction::Pack {
            item: ItemId::Stone,
            ..
        }
    )));
}
