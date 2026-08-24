use crate::action::{PrimaryAction, Recipe};
use crate::agent::{AgentId, ItemId};
use crate::event_log::{SimEvent, SimEventKind};
use crate::memory::{knows_toxin, remember, MemoryEntry, MemoryKind};
use crate::observation::neighbors4;
use crate::simulation::Simulation;
use crate::species::{Crop, Toxicity, VegYield};
use rand::Rng;
use rand_chacha::ChaCha20Rng;

const ILLNESS_TICKS: u32 = 12;

pub fn execute_primary(sim: &mut Simulation, id: AgentId, action: &PrimaryAction) {
    if !is_legal(sim, id, action) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    match action {
        PrimaryAction::Wait => push(sim, id, SimEventKind::Wait),
        PrimaryAction::Rest => rest(sim, id),
        PrimaryAction::MoveRelative { dx, dy } => move_rel(sim, id, *dx, *dy),
        PrimaryAction::Gather { species } => gather(sim, id, *species),
        PrimaryAction::Drink => drink(sim, id),
        PrimaryAction::Eat { item } => eat(sim, id, *item),
        PrimaryAction::Hunt => hunt(sim, id),
        PrimaryAction::Fish => fish(sim, id),
        PrimaryAction::Farm { species } => farm(sim, id, *species),
        PrimaryAction::Craft { recipe } => craft(sim, id, *recipe),
    }
}

fn is_legal(sim: &Simulation, id: AgentId, action: &PrimaryAction) -> bool {
    let Some(agent) = sim.agents.get(&id) else {
        return false;
    };
    crate::observation::legal_actions(sim, agent).iter().any(|a| a == action)
}

fn push(sim: &mut Simulation, id: AgentId, kind: SimEventKind) {
    sim.events.push(SimEvent {
        tick: sim.tick,
        agent: id,
        kind,
    });
}

fn skill_roll(
    rng: &mut ChaCha20Rng,
    skill: u8,
    bonus: i32,
    ill: bool,
    hunger: u32,
    thirst: u32,
    energy: u32,
) -> bool {
    let mut chance = i32::from(skill) + bonus;
    if ill {
        chance -= 20;
    }
    if hunger == 0 {
        chance -= 25;
    }
    if thirst == 0 {
        chance -= 25;
    }
    if energy == 0 {
        chance -= 25;
    }
    let chance = chance.clamp(5, 95) as u32;
    rng.random_range(0u32..100) < chance
}

fn rest(sim: &mut Simulation, id: AgentId) {
    let regen = sim.config.energy_regen_milli();
    let max = sim.config.energy_max_milli();
    if let Some(a) = sim.agents.get_mut(&id) {
        a.needs.energy = (a.needs.energy + regen).min(max);
    }
    push(sim, id, SimEventKind::Rest);
}

fn move_rel(sim: &mut Simulation, id: AgentId, dx: i32, dy: i32) {
    let Some(agent) = sim.agents.get(&id) else {
        return;
    };
    let from_x = agent.x;
    let from_y = agent.y;
    let nx = from_x as i32 + dx;
    let ny = from_y as i32 + dy;
    if !sim.world.in_bounds(nx, ny) || sim.world.is_water(nx as u32, ny as u32) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if agent.needs.energy == 0 {
        let fail = sim.rngs.agent_stream(id).random_bool(0.5);
        if fail {
            push(sim, id, SimEventKind::Wait);
            return;
        }
    }
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = nx as u32;
        a.y = ny as u32;
    }
    push(
        sim,
        id,
        SimEventKind::Move {
            from_x,
            from_y,
            to_x: nx as u32,
            to_y: ny as u32,
        },
    );
}

fn gather(sim: &mut Simulation, id: AgentId, species: u8) {
    if species == 0 {
        gather_stone(sim, id);
        return;
    }
    let Some(spec) = sim.config.world.species.veg(species).cloned() else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let cell = neighbors4(&sim.world, agent.x, agent.y)
        .into_iter()
        .find(|&(x, y)| sim.world.vegetation_species(x, y) == species);
    let Some((x, y)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let bonus = if agent.has_tool(ItemId::Basket) { 15 } else { 0 };
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.gather,
            bonus,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    if !ok {
        push(
            sim,
            id,
            SimEventKind::Gather {
                species,
                item: ItemId::Wood,
                qty: 0,
            },
        );
        return;
    }
    let mut got_item = ItemId::Wood;
    let mut qty = 0u32;
    if let Some(a) = sim.agents.get_mut(&id) {
        match spec.yield_kind {
            VegYield::Wood => {
                got_item = ItemId::Wood;
                qty = a.try_add_item(ItemId::Wood, spec.wood_yield.max(1));
            }
            VegYield::Food => {
                got_item = ItemId::Food(species);
                let n = 1 + u32::from(a.has_tool(ItemId::Basket));
                qty = a.try_add_item(ItemId::Food(species), n);
                if spec.fiber_yield > 0 {
                    let _ = a.try_add_item(ItemId::Fiber, spec.fiber_yield);
                }
            }
        }
    }
    sim.world.set_vegetation(x, y, 0);
    remember_obs(sim, id, species, x, y);
    push(
        sim,
        id,
        SimEventKind::Gather {
            species,
            item: got_item,
            qty,
        },
    );
}

fn gather_stone(sim: &mut Simulation, id: AgentId) {
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let cell = neighbors4(&sim.world, agent.x, agent.y)
        .into_iter()
        .find(|&(x, y)| sim.world.has_mineral(x, y));
    let Some((x, y)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.gather,
            0,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    let mut qty = 0;
    if ok {
        if let Some(a) = sim.agents.get_mut(&id) {
            qty = a.try_add_item(ItemId::Stone, 1);
        }
        let i = (y * sim.world.width + x) as usize;
        if i < sim.world.minerals.len() {
            sim.world.minerals[i] = 0;
        }
    }
    push(
        sim,
        id,
        SimEventKind::Gather {
            species: 0,
            item: ItemId::Stone,
            qty,
        },
    );
}

fn drink(sim: &mut Simulation, id: AgentId) {
    let max = sim.config.thirst_max_milli();
    if let Some(a) = sim.agents.get_mut(&id) {
        a.needs.thirst = max;
    }
    push(sim, id, SimEventKind::Drink);
}

fn eat(sim: &mut Simulation, id: AgentId, item: ItemId) {
    let ItemId::Food(tag) = item else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let hunger_max = sim.config.hunger_max_milli();
    let cap = sim.config.agents.default_memory_capacity;
    let tick = sim.tick;

    let (nutr, toxic, allergic, name, is_veg, is_animal, is_fish) = if tag == 100 {
        let n = sim
            .config
            .world
            .species
            .animals
            .first()
            .map(|s| s.nutrition_milli())
            .unwrap_or(3000);
        (n, false, false, "hare".to_string(), false, true, false)
    } else if tag == 101 {
        let n = sim
            .config
            .world
            .species
            .fish
            .first()
            .map(|s| s.nutrition_milli())
            .unwrap_or(2500);
        (n, false, false, "perch".to_string(), false, false, true)
    } else {
        let Some(spec) = sim.config.world.species.veg(tag).cloned() else {
            push(sim, id, SimEventKind::Wait);
            return;
        };
        let allergic = spec.toxicity == Toxicity::Allergenic
            && !spec.allergen_tag.is_empty()
            && sim.agents.get(&id).is_some_and(|a| {
                a.personality
                    .allergy_tags
                    .iter()
                    .any(|t| t == &spec.allergen_tag)
            });
        let toxic = spec.toxicity == Toxicity::Toxic || allergic;
        (
            spec.nutrition_milli(),
            toxic,
            allergic,
            spec.id,
            true,
            false,
            false,
        )
    };
    let _ = allergic;

    let Some(agent) = sim.agents.get_mut(&id) else {
        return;
    };
    if !agent.take_item(item, 1) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    agent.needs.hunger = (agent.needs.hunger + nutr).min(hunger_max);
    if is_veg {
        agent.consumption.vegetation += 1;
    }
    if is_animal {
        agent.consumption.animal += 1;
    }
    if is_fish {
        agent.consumption.fish += 1;
    }
    if toxic {
        agent.illness_ticks = agent.illness_ticks.max(ILLNESS_TICKS);
        agent.consumption.toxic_events += 1;
        agent.needs.energy = agent.needs.energy.saturating_sub(800);
        remember(
            &mut agent.memory,
            cap,
            MemoryEntry {
                tick,
                kind: MemoryKind::Sickness,
                text: format!("ate {name} and felt sick"),
                importance: 90,
                last_accessed: tick,
                species_tag: tag,
            },
        );
        remember(
            &mut agent.memory,
            cap,
            MemoryEntry {
                tick,
                kind: MemoryKind::ToxinFact,
                text: format!("{name} is toxic"),
                importance: 95,
                last_accessed: tick,
                species_tag: tag,
            },
        );
    }
    push(sim, id, SimEventKind::Eat { item, toxic });
}

fn hunt(sim: &mut Simulation, id: AgentId) {
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let cell = neighbors4(&sim.world, agent.x, agent.y)
        .into_iter()
        .find(|&(x, y)| sim.world.animal_count_at(x, y) > 0);
    let Some((x, y)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let bonus = if agent.has_tool(ItemId::Spear) { 25 } else { -15 };
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.hunt,
            bonus,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    if ok {
        sim.world.add_animal(x, y, -1);
        let nutr = sim
            .config
            .world
            .species
            .animals
            .first()
            .map(|s| s.nutrition_milli())
            .unwrap_or(3000);
        if let Some(a) = sim.agents.get_mut(&id) {
            let _ = a.try_add_item(ItemId::Food(100), 1);
        }
        let _ = nutr;
    }
    push(sim, id, SimEventKind::Hunt { success: ok });
}

fn fish(sim: &mut Simulation, id: AgentId) {
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let cell = neighbors4(&sim.world, agent.x, agent.y)
        .into_iter()
        .find(|&(x, y)| sim.world.is_water(x, y) && sim.world.fish_count_at(x, y) > 0);
    let Some((x, y)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let bonus = if agent.has_tool(ItemId::FishingRod) {
        25
    } else {
        -15
    };
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.fish,
            bonus,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    if ok {
        sim.world.add_fish(x, y, -1);
        if let Some(a) = sim.agents.get_mut(&id) {
            let _ = a.try_add_item(ItemId::Food(101), 1);
        }
    }
    push(sim, id, SimEventKind::Fish { success: ok });
}

fn farm(sim: &mut Simulation, id: AgentId, species: u8) {
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let cell = neighbors4(&sim.world, agent.x, agent.y).into_iter().find(|&(x, y)| {
        sim.world.is_land(x, y)
            && sim.world.vegetation_species(x, y) == 0
            && !sim.world.has_mineral(x, y)
            && !sim.world.crops.contains_key(&(x, y))
    });
    let Some((x, y)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.farm,
            0,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    if !ok {
        push(sim, id, SimEventKind::Farm { species, x, y });
        return;
    }
    if let Some(a) = sim.agents.get_mut(&id) {
        if !a.take_item(ItemId::Food(species), 1) {
            push(sim, id, SimEventKind::Wait);
            return;
        }
    }
    sim.world.crops.insert(
        (x, y),
        Crop {
            species_tag: species,
            planted_tick: sim.tick,
        },
    );
    push(sim, id, SimEventKind::Farm { species, x, y });
}

fn craft(sim: &mut Simulation, id: AgentId, recipe: Recipe) {
    let Some(agent) = sim.agents.get(&id).cloned() else {
        return;
    };
    let (need, out) = match recipe {
        Recipe::Basket => (vec![(ItemId::Fiber, 2)], ItemId::Basket),
        Recipe::Spear => (vec![(ItemId::Wood, 1), (ItemId::Stone, 1)], ItemId::Spear),
        Recipe::FishingRod => (vec![(ItemId::Wood, 1), (ItemId::Fiber, 1)], ItemId::FishingRod),
    };
    let has_all = need.iter().all(|(item, n)| {
        agent.inventory.get(item).copied().unwrap_or(0) >= *n
    });
    let room = agent.inventory_cap.saturating_sub(agent.inventory_count()) >= 1
        || agent.inventory.contains_key(&out);
    if !has_all || !room {
        push(sim, id, SimEventKind::Craft {
            recipe,
            success: false,
        });
        return;
    }
    let ok = {
        let rng = sim.rngs.agent_stream(id);
        skill_roll(
            rng,
            agent.abilities.craft,
            0,
            agent.illness_ticks > 0,
            agent.needs.hunger,
            agent.needs.thirst,
            agent.needs.energy,
        )
    };
    if !ok {
        push(sim, id, SimEventKind::Craft {
            recipe,
            success: false,
        });
        return;
    }
    let Some(a) = sim.agents.get_mut(&id) else {
        return;
    };
    for (item, n) in need {
        let _ = a.take_item(item, n);
    }
    let success = a.try_add_item(out, 1) > 0;
    push(sim, id, SimEventKind::Craft { recipe, success });
}

fn remember_obs(sim: &mut Simulation, id: AgentId, species: u8, x: u32, y: u32) {
    let cap = sim.config.agents.default_memory_capacity;
    let tick = sim.tick;
    if let Some(a) = sim.agents.get_mut(&id) {
        remember(
            &mut a.memory,
            cap,
            MemoryEntry {
                tick,
                kind: MemoryKind::Observation,
                text: format!("saw species {species} at ({x},{y})"),
                importance: 40,
                last_accessed: tick,
                species_tag: species,
            },
        );
    }
}

pub fn apply_heard_memories(sim: &mut Simulation, id: AgentId, heard: &[crate::observation::HeardSpeech]) {
    let cap = sim.config.agents.default_memory_capacity;
    let tick = sim.tick;
    let species = sim.config.world.species.clone();
    if let Some(a) = sim.agents.get_mut(&id) {
        for h in heard {
            remember(
                &mut a.memory,
                cap,
                MemoryEntry {
                    tick,
                    kind: MemoryKind::Utterance,
                    text: h.text.clone(),
                    importance: 50,
                    last_accessed: tick,
                    species_tag: 0,
                },
            );
            for (i, spec) in species.vegetation.iter().enumerate() {
                let tag = (i + 1) as u8;
                if h.text.contains(&spec.id)
                    && (h.text.contains("toxic") || h.text.contains("sick"))
                    && !knows_toxin(&a.memory, tag)
                {
                    remember(
                        &mut a.memory,
                        cap,
                        MemoryEntry {
                            tick,
                            kind: MemoryKind::ToxinFact,
                            text: format!("{} is toxic", spec.id),
                            importance: 90,
                            last_accessed: tick,
                            species_tag: tag,
                        },
                    );
                }
            }
        }
    }
}


