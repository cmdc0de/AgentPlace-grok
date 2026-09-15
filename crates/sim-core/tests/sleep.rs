//! M53 sleep places, night Hunt/Farm gating, CON dawn extra.

use sim_core::action::PrimaryAction;
use sim_core::agent::ItemId;
use sim_core::clock::{dawn_energy, dawn_refill};
use sim_core::event_log::SimEventKind;
use sim_core::observation::legal_actions;
use sim_core::objects::{can_place, sleep_covers};
use sim_core::sheet::AbilitySheet;
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::path::PathBuf;

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

fn shipped() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects")
}

fn apply_shipped(sim: &mut Simulation) {
    sim.apply_objects_dir(&shipped()).unwrap();
}

fn catalog_item(sim: &Simulation, slug: &str) -> ItemId {
    sim.catalog
        .iter()
        .find(|e| e.slug == slug)
        .unwrap_or_else(|| panic!("{slug}"))
        .item
}

fn land_square(sim: &Simulation, n: u32) -> (u32, u32) {
    let w = sim.world.width;
    let h = sim.world.height;
    for y in 0..=h.saturating_sub(n) {
        for x in 0..=w.saturating_sub(n) {
            let ok = (0..n).all(|dy| {
                (0..n).all(|dx| sim.world.is_land(x + dx, y + dy))
            });
            if ok {
                return (x, y);
            }
        }
    }
    panic!("no {n}x{n} land");
}

fn give(sim: &mut Simulation, id: AgentId, slug: &str) {
    let item = catalog_item(sim, slug);
    let added = sim.agents.get_mut(&id).unwrap().try_add_item(item, 1);
    assert_eq!(added, 1, "give {slug}");
}

fn park(sim: &mut Simulation, id: AgentId, x: u32, y: u32) {
    let a = sim.agents.get_mut(&id).unwrap();
    a.x = x;
    a.y = y;
}

fn put_animal_adjacent(sim: &mut Simulation, id: AgentId) {
    let (x, y) = {
        let a = sim.agents.get(&id).unwrap();
        (a.x, a.y)
    };
    for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if sim.world.in_bounds(nx, ny) && sim.world.is_land(nx as u32, ny as u32) {
            let i = (ny as u32 * sim.world.width + nx as u32) as usize;
            if i < sim.world.animals.len() {
                sim.world.animals[i] = 1;
                return;
            }
        }
    }
    panic!("no adjacent land for animal");
}

#[test]
fn night_hunt_farm_illegal_when_time_on() {
    let mut sim = Simulation::new(tiny(0x53_01)).unwrap();
    sim.time_enabled = true;
    sim.ticks_per_day = 240;
    sim.tick = 180;
    let id = AgentId(0);
    put_animal_adjacent(&mut sim, id);
    sim.agents.get_mut(&id).unwrap().try_add_item(ItemId::Food(1), 1);
    let legal = legal_actions(&sim, sim.agents.get(&id).unwrap());
    assert!(
        !legal.iter().any(|a| matches!(a, PrimaryAction::Hunt)),
        "Hunt at night"
    );
    assert!(
        !legal.iter().any(|a| matches!(a, PrimaryAction::Farm { .. })),
        "Farm at night"
    );
    assert!(legal.iter().any(|a| matches!(a, PrimaryAction::Gather { .. })));
}

#[test]
fn no_time_hunt_still_legal() {
    let mut sim = Simulation::new(tiny(0x53_02)).unwrap();
    sim.time_enabled = false;
    sim.tick = 180;
    let id = AgentId(0);
    put_animal_adjacent(&mut sim, id);
    sim.agents.get_mut(&id).unwrap().try_add_item(ItemId::Food(1), 1);
    let legal = legal_actions(&sim, sim.agents.get(&id).unwrap());
    assert!(legal.iter().any(|a| matches!(a, PrimaryAction::Hunt)));
    assert!(legal.iter().any(|a| matches!(a, PrimaryAction::Farm { .. })));
}

#[test]
fn catalog_off_place_illegal() {
    let sim = Simulation::new(tiny(0x53_10)).unwrap();
    let a = AgentId(0);
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Place { .. })));
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Pickup)));
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: sim_core::action::Recipe::Catalog(_)
        }
    )));
}

#[test]
fn catalog_on_craft_cabin_house() {
    let mut sim = Simulation::new(tiny(0x53_11)).unwrap();
    apply_shipped(&mut sim);
    let cabin = catalog_item(&sim, "cabin");
    let house = catalog_item(&sim, "house");
    assert!(matches!(cabin, ItemId::Catalog(_)));
    assert!(matches!(house, ItemId::Catalog(_)));
    let tent_e = sim.catalog.iter().find(|e| e.slug == "tent").unwrap();
    assert_eq!(tent_e.sleep_bonus, 100);
    assert_eq!(tent_e.sleep_size, 1);
    let cabin_e = sim.catalog.iter().find(|e| e.slug == "cabin").unwrap();
    assert_eq!(cabin_e.sleep_bonus, 200);
    assert_eq!(cabin_e.sleep_size, 2);
    let house_e = sim.catalog.iter().find(|e| e.slug == "house").unwrap();
    assert_eq!(house_e.sleep_bonus, 300);
    assert_eq!(house_e.sleep_size, 4);
}

#[test]
fn place_tent_occupies_one_cell() {
    let mut sim = Simulation::new(tiny(0x53_20)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    give(&mut sim, id, "tent");
    let item = catalog_item(&sim, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    assert_eq!(sim.world.sleep_places.get(&(x, y)), Some(&item));
    assert_eq!(
        sim.agents[&id].inventory.get(&item).copied().unwrap_or(0),
        0
    );
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Placed { x: px, y: py, .. } if px == x && py == y
    )));
    give(&mut sim, id, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    assert_eq!(sim.world.sleep_places.len(), 1);
}

#[test]
fn place_cabin_is_2x2_no_overlap() {
    let mut sim = Simulation::new(tiny(0x53_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 2);
    park(&mut sim, id, x, y);
    let item = catalog_item(&sim, "cabin");
    give(&mut sim, id, "cabin");
    assert!(can_place(&sim.world, &sim.catalog, x, y, item));
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    assert_eq!(sleep_covers(&sim.world, &sim.catalog, x, y), Some(item));
    assert_eq!(
        sleep_covers(&sim.world, &sim.catalog, x + 1, y + 1),
        Some(item)
    );
    park(&mut sim, id, x + 1, y);
    give(&mut sim, id, "tent");
    let tent = catalog_item(&sim, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: tent });
    assert!(sim.world.sleep_places.get(&(x + 1, y)).is_none());
}

#[test]
fn place_cabin_fails_on_water_or_oob() {
    let mut sim = Simulation::new(tiny(0x53_23)).unwrap();
    apply_shipped(&mut sim);
    let item = catalog_item(&sim, "cabin");
    let w = sim.world.width;
    let h = sim.world.height;
    assert!(!can_place(&sim.world, &sim.catalog, w - 1, 0, item));
    assert!(!can_place(&sim.world, &sim.catalog, 0, h - 1, item));
    let mut mixed = false;
    for y in 0..h.saturating_sub(1) {
        for x in 0..w.saturating_sub(1) {
            let cells = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)];
            let any_water = cells.iter().any(|&(cx, cy)| !sim.world.is_land(cx, cy));
            if any_water {
                assert!(!can_place(&sim.world, &sim.catalog, x, y, item));
                mixed = true;
            }
        }
    }
    assert!(mixed, "need a 2x2 that includes water or OOB-equivalent");
}

#[test]
fn place_house_is_4x4() {
    let mut sim = Simulation::new(tiny(0x53_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 4);
    park(&mut sim, id, x, y);
    let item = catalog_item(&sim, "house");
    give(&mut sim, id, "house");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    assert_eq!(sim.world.sleep_places.get(&(x, y)), Some(&item));
    assert_eq!(
        sleep_covers(&sim.world, &sim.catalog, x + 3, y + 3),
        Some(item)
    );
}

#[test]
fn dawn_on_tent_adds_shelter() {
    let cfg = tiny(0x53_30);
    let mut on = Simulation::new(cfg.clone()).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    apply_shipped(&mut on);
    apply_shipped(&mut off);
    on.time_enabled = true;
    off.time_enabled = true;
    on.ticks_per_day = 240;
    off.ticks_per_day = 240;
    let id = AgentId(0);
    let (x, y) = land_square(&on, 1);
    park(&mut on, id, x, y);
    park(&mut off, id, x, y);
    let tent = catalog_item(&on, "tent");
    give(&mut on, id, "tent");
    sim_core::execute::execute_primary(&mut on, id, &PrimaryAction::Place { item: tent });
    let max = on.agents[&id]
        .sheet
        .energy_max(on.config.energy_max_milli());
    let start = max / 2;
    on.agents.get_mut(&id).unwrap().needs.energy = start;
    off.agents.get_mut(&id).unwrap().needs.energy = start;
    on.tick = 239;
    off.tick = 239;
    assert!(on.tick());
    assert!(off.tick());
    let extra = on.agents[&id]
        .needs
        .energy
        .saturating_sub(off.agents[&id].needs.energy);
    let want = dawn_energy(start, max, 100, 0) - dawn_refill(start, max);
    assert_eq!(extra, want);
}

#[test]
fn dawn_cabin_non_origin_gets_bonus() {
    let mut sim = Simulation::new(tiny(0x53_31)).unwrap();
    apply_shipped(&mut sim);
    sim.time_enabled = true;
    sim.ticks_per_day = 240;
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 2);
    park(&mut sim, id, x, y);
    let cabin = catalog_item(&sim, "cabin");
    give(&mut sim, id, "cabin");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: cabin });
    park(&mut sim, id, x + 1, y + 1);
    let max = sim.agents[&id]
        .sheet
        .energy_max(sim.config.energy_max_milli());
    let start = max / 2;
    sim.agents.get_mut(&id).unwrap().needs.energy = start;
    sim.tick = 239;
    assert!(sim.tick());
    let after = sim.agents[&id].needs.energy;
    assert!(after > dawn_refill(start, max) || after == max);
}

#[test]
fn dawn_off_footprint_is_tiredness_only() {
    let cfg = tiny(0x53_33);
    let mut on = Simulation::new(cfg.clone()).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    apply_shipped(&mut on);
    apply_shipped(&mut off);
    on.time_enabled = true;
    off.time_enabled = true;
    on.ticks_per_day = 240;
    off.ticks_per_day = 240;
    let id = AgentId(0);
    let (x, y) = land_square(&on, 1);
    park(&mut on, id, x, y);
    let tent = catalog_item(&on, "tent");
    give(&mut on, id, "tent");
    sim_core::execute::execute_primary(&mut on, id, &PrimaryAction::Place { item: tent });
    let (ax, ay) = land_square(&on, 2);
    let away = if ax == x && ay == y {
        (ax + 1, ay)
    } else {
        (ax, ay)
    };
    park(&mut on, id, away.0, away.1);
    park(&mut off, id, away.0, away.1);
    let max = on.agents[&id]
        .sheet
        .energy_max(on.config.energy_max_milli());
    let start = max / 2;
    on.agents.get_mut(&id).unwrap().needs.energy = start;
    off.agents.get_mut(&id).unwrap().needs.energy = start;
    on.tick = 239;
    off.tick = 239;
    assert!(on.tick());
    assert!(off.tick());
    assert_eq!(
        on.agents[&id].needs.energy,
        off.agents[&id].needs.energy,
        "off-footprint dawn must match open-air"
    );
}

#[test]
fn load_restores_sleep_place() {
    let mut sim = Simulation::new(tiny(0x53_32)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    let tent = catalog_item(&sim, "tent");
    give(&mut sim, id, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: tent });
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(loaded.world.sleep_places.get(&(x, y)), Some(&tent));
}

#[test]
fn con_18_gets_1000_more_than_unused() {
    let cfg = tiny(0x53_40);
    let mut hi = Simulation::new(cfg.clone()).unwrap();
    let mut unused = Simulation::new(cfg).unwrap();
    hi.time_enabled = true;
    unused.time_enabled = true;
    hi.ticks_per_day = 240;
    unused.ticks_per_day = 240;
    let id = AgentId(0);
    hi.agents.get_mut(&id).unwrap().sheet.constitution = 18;
    unused.agents.get_mut(&id).unwrap().sheet.constitution = 0;
    assert_eq!(AbilitySheet::modifier(18), 4);
    let max_u = unused.agents[&id]
        .sheet
        .energy_max(unused.config.energy_max_milli());
    let start = max_u / 2;
    hi.agents.get_mut(&id).unwrap().needs.energy = start;
    unused.agents.get_mut(&id).unwrap().needs.energy = start;
    hi.tick = 239;
    unused.tick = 239;
    assert!(hi.tick());
    assert!(unused.tick());
    let extra = hi.agents[&id]
        .needs
        .energy
        .saturating_sub(unused.agents[&id].needs.energy);
    let hi_max = hi.agents[&id]
        .sheet
        .energy_max(hi.config.energy_max_milli());
    let want = dawn_energy(start, hi_max, 0, 1000)
        .saturating_sub(dawn_energy(start, max_u, 0, 0));
    assert_eq!(extra, want);
}

#[test]
fn con_zero_matches_unused_dawn() {
    let cfg = tiny(0x53_41);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.time_enabled = true;
    b.time_enabled = true;
    a.ticks_per_day = 240;
    b.ticks_per_day = 240;
    let id = AgentId(0);
    a.agents.get_mut(&id).unwrap().sheet.constitution = 0;
    b.agents.get_mut(&id).unwrap().sheet.constitution = 0;
    let max = a.agents[&id]
        .sheet
        .energy_max(a.config.energy_max_milli());
    let start = max / 2;
    a.agents.get_mut(&id).unwrap().needs.energy = start;
    b.agents.get_mut(&id).unwrap().needs.energy = start;
    a.tick = 239;
    b.tick = 239;
    assert!(a.tick());
    assert!(b.tick());
    assert_eq!(a.agents[&id].needs.energy, b.agents[&id].needs.energy);
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

fn ready_pair(sim: &mut Simulation, a: AgentId, b: AgentId) {
    sim.enable_reproduction();
    let emax = sim.config.energy_max_milli();
    for id in [a, b] {
        let ag = sim.agents.get_mut(&id).unwrap();
        ag.sheet.charisma = 0;
        ag.needs.energy = emax;
    }
}

#[test]
fn place_tent_then_pickup() {
    let mut sim = Simulation::new(tiny(0x54_20)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    give(&mut sim, id, "tent");
    let item = catalog_item(&sim, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    assert_eq!(sim.world.sleep_places.get(&(x, y)), Some(&item));
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Pickup);
    assert!(sim.world.sleep_places.is_empty());
    assert_eq!(
        sim.agents[&id].inventory.get(&item).copied().unwrap_or(0),
        1
    );
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::PickedUp { x: px, y: py, .. } if px == x && py == y
    )));
}

#[test]
fn pickup_full_pockets_and_pack_waits() {
    let mut sim = Simulation::new(tiny(0x54_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    give(&mut sim, id, "tent");
    let item = catalog_item(&sim, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item });
    let ag = sim.agents.get_mut(&id).unwrap();
    ag.inventory.clear();
    ag.pack.clear();
    ag.inventory_cap = 1;
    ag.try_add_item(ItemId::Stone, 1);
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Pickup);
    assert_eq!(sim.world.sleep_places.get(&(x, y)), Some(&item));
    assert_eq!(
        sim.agents[&id].inventory.get(&item).copied().unwrap_or(0),
        0
    );
    assert!(sim.events.events.iter().any(|e| matches!(e.kind, SimEventKind::Wait)));
}

#[test]
fn pickup_cabin_from_non_origin() {
    let mut sim = Simulation::new(tiny(0x54_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 2);
    park(&mut sim, id, x, y);
    let cabin = catalog_item(&sim, "cabin");
    give(&mut sim, id, "cabin");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: cabin });
    park(&mut sim, id, x + 1, y + 1);
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Pickup);
    assert!(sim.world.sleep_places.is_empty());
    assert_eq!(
        sleep_covers(&sim.world, &sim.catalog, x + 1, y + 1),
        None
    );
    assert_eq!(
        sim.agents[&id].inventory.get(&cabin).copied().unwrap_or(0),
        1
    );
}

#[test]
fn load_then_pickup_tent() {
    let mut sim = Simulation::new(tiny(0x54_23)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    let tent = catalog_item(&sim, "tent");
    give(&mut sim, id, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: tent });
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.apply_objects_dir(&shipped()).unwrap();
    assert_eq!(loaded.world.sleep_places.get(&(x, y)), Some(&tent));
    let before = loaded.agents[&id]
        .inventory
        .get(&tent)
        .copied()
        .unwrap_or(0);
    sim_core::execute::execute_primary(&mut loaded, id, &PrimaryAction::Pickup);
    assert!(loaded.world.sleep_places.is_empty());
    assert_eq!(
        loaded.agents[&id].inventory.get(&tent).copied().unwrap_or(0),
        before + 1
    );
}

#[test]
fn household_auto_cabin_on_pair_bond() {
    let mut sim = Simulation::new(tiny(0x54_30)).unwrap();
    apply_shipped(&mut sim);
    let (ox, oy) = land_square(&sim, 2);
    park(&mut sim, AgentId(0), ox, oy);
    park(&mut sim, AgentId(1), ox + 1, oy);
    let (a, b) = (AgentId(0), AgentId(1));
    ready_pair(&mut sim, a, b);
    sim.enable_household_crates();
    let wood_before = sim.agents[&a]
        .inventory
        .get(&ItemId::Wood)
        .copied()
        .unwrap_or(0);
    let stone_before = sim.agents[&a]
        .inventory
        .get(&ItemId::Stone)
        .copied()
        .unwrap_or(0);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    let hid = sim.agents[&a].kinship.household.expect("household");
    assert_eq!(sim.household_home.get(&hid), Some(&(ox, oy)));
    let cabin = catalog_item(&sim, "cabin");
    assert_eq!(sim.world.sleep_places.get(&(ox, oy)), Some(&cabin));
    assert_eq!(
        sim.agents[&a]
            .inventory
            .get(&ItemId::Wood)
            .copied()
            .unwrap_or(0),
        wood_before
    );
    assert_eq!(
        sim.agents[&a]
            .inventory
            .get(&ItemId::Stone)
            .copied()
            .unwrap_or(0),
        stone_before
    );
}

#[test]
fn household_auto_cabin_skips_when_cannot_place() {
    let mut sim = Simulation::new(tiny(0x54_31)).unwrap();
    apply_shipped(&mut sim);
    let cabin = catalog_item(&sim, "cabin");
    let w = sim.world.width;
    let h = sim.world.height;
    let mut origin = None;
    for y in 0..h {
        for x in 0..w {
            if sim.world.is_land(x, y) && !can_place(&sim.world, &sim.catalog, x, y, cabin) {
                for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if sim.world.in_bounds(nx, ny) && sim.world.is_land(nx as u32, ny as u32) {
                        origin = Some((x, y, nx as u32, ny as u32));
                        break;
                    }
                }
            }
            if origin.is_some() {
                break;
            }
        }
        if origin.is_some() {
            break;
        }
    }
    let (ax, ay, bx, by) = origin.expect("land cell where cabin cannot place + neighbor");
    park(&mut sim, AgentId(0), ax, ay);
    park(&mut sim, AgentId(1), bx, by);
    ready_pair(&mut sim, AgentId(0), AgentId(1));
    sim.enable_household_crates();
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::PairBond { target: AgentId(1) },
    );
    let hid = sim.agents[&AgentId(0)].kinship.household.expect("home");
    assert_eq!(sim.household_home.get(&hid), Some(&(ax, ay)));
    assert!(sim.world.sleep_places.is_empty());
}

#[test]
fn load_restores_auto_cabin_no_remint() {
    let mut sim = Simulation::new(tiny(0x54_32)).unwrap();
    apply_shipped(&mut sim);
    let (ox, oy) = land_square(&sim, 2);
    park(&mut sim, AgentId(0), ox, oy);
    park(&mut sim, AgentId(1), ox + 1, oy);
    ready_pair(&mut sim, AgentId(0), AgentId(1));
    sim.enable_household_crates();
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::PairBond { target: AgentId(1) },
    );
    let hid = sim.agents[&AgentId(0)].kinship.household.unwrap();
    let cabin = catalog_item(&sim, "cabin");
    assert_eq!(sim.world.sleep_places.len(), 1);
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.apply_objects_dir(&shipped()).unwrap();
    loaded.enable_household_crates();
    assert_eq!(loaded.household_home.get(&hid), Some(&(ox, oy)));
    assert_eq!(loaded.world.sleep_places.get(&(ox, oy)), Some(&cabin));
    assert_eq!(loaded.world.sleep_places.len(), 1);
}

#[test]
fn household_off_pair_bond_no_cabin() {
    let mut sim = Simulation::new(tiny(0x54_33)).unwrap();
    apply_shipped(&mut sim);
    let (a, b) = place_adjacent(&mut sim);
    ready_pair(&mut sim, a, b);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::PairBond { target: b });
    assert!(sim.household_home.is_empty());
    assert!(sim.world.sleep_places.is_empty());
}

#[test]
fn millstone_place_then_pickup() {
    let mut sim = Simulation::new(tiny(0x58_30)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    let mill = catalog_item(&sim, "millstone");
    give(&mut sim, id, "millstone");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: mill });
    assert_eq!(sim.world.work_places.get(&(x, y)), Some(&mill));
    assert!(sim.world.sleep_places.is_empty());
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Pickup);
    assert!(sim.world.work_places.is_empty());
    assert_eq!(sim.agents[&id].inventory.get(&mill).copied(), Some(1));
}

#[test]
fn millstone_overlap_with_tent_illegal() {
    let mut sim = Simulation::new(tiny(0x58_31)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    let tent = catalog_item(&sim, "tent");
    give(&mut sim, id, "tent");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: tent });
    give(&mut sim, id, "millstone");
    let mill = catalog_item(&sim, "millstone");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: mill });
    assert!(sim.world.work_places.is_empty());
    assert_eq!(sim.world.sleep_places.get(&(x, y)), Some(&tent));
}

#[test]
fn load_restores_work_place() {
    let mut sim = Simulation::new(tiny(0x58_32)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let (x, y) = land_square(&sim, 1);
    park(&mut sim, id, x, y);
    let mill = catalog_item(&sim, "millstone");
    give(&mut sim, id, "millstone");
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Place { item: mill });
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(loaded.world.work_places.get(&(x, y)), Some(&mill));
}
