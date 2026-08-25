use crate::action::{PrimaryAction, Recipe};
use crate::agent::{AgentId, ItemId};
use crate::event_log::{SimEvent, SimEventKind};
use crate::memory::{knows_toxin, MemoryEntry, MemoryKind};
use crate::observation::neighbors4;
use crate::simulation::Simulation;
use crate::species::{Crop, Toxicity, VegYield};
use rand::Rng;
use rand_chacha::ChaCha20Rng;

const ILLNESS_TICKS: u32 = 12;

pub fn execute_primary(sim: &mut Simulation, id: AgentId, action: &PrimaryAction) {
    if let Some(reason) = rule_block(sim, id, action) {
        push(sim, id, SimEventKind::RuleBlocked { reason });
        return;
    }
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
        PrimaryAction::Propose { text, rule } => propose(sim, id, text, *rule),
        PrimaryAction::Support { proposal_id } => vote(sim, id, *proposal_id, true),
        PrimaryAction::Oppose { proposal_id } => vote(sim, id, *proposal_id, false),
    }
}

fn rule_block(sim: &Simulation, id: AgentId, action: &PrimaryAction) -> Option<String> {
    match action {
        PrimaryAction::Eat {
            item: crate::agent::ItemId::Food(tag),
        } if *tag < 100 && sim.board.blocks_eat(*tag) => Some(format!("ban eat species {tag}")),
        PrimaryAction::Gather { species } if *species != 0 && sim.board.blocks_gather(*species) => {
            Some(format!("ban gather species {species}"))
        }
        PrimaryAction::Gather { species } if *species != 0 => {
            if let Some(max) = sim.board.max_gather_per_tick() {
                if let Some(a) = sim.agents.get(&id) {
                    if a.gathers_this_tick >= max {
                        return Some("max gather per tick".into());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_legal(sim: &Simulation, id: AgentId, action: &PrimaryAction) -> bool {
    let Some(agent) = sim.agents.get(&id) else {
        return false;
    };
    match action {
        PrimaryAction::Propose { text, .. } => {
            !text.is_empty()
                && sim.board.author_open_count(id)
                    < sim.config.proposals.max_open_proposals_per_agent as usize
        }
        PrimaryAction::Support { proposal_id } | PrimaryAction::Oppose { proposal_id } => {
            sim.board.open().any(|p| p.id == *proposal_id)
        }
        other => crate::observation::legal_actions(sim, agent)
            .iter()
            .any(|a| a == other),
    }
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
    let bonus = if agent.has_tool(ItemId::Basket) {
        15
    } else {
        0
    };
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
    let food_n = {
        let basket = sim
            .agents
            .get(&id)
            .is_some_and(|a| a.has_tool(ItemId::Basket));
        crate::incentive::scale_u32(
            1 + u32::from(basket),
            crate::incentive::resource_mult_milli(sim, id, "food"),
        )
        .max(1)
    };
    if let Some(a) = sim.agents.get_mut(&id) {
        match spec.yield_kind {
            VegYield::Wood => {
                got_item = ItemId::Wood;
                qty = a.try_add_item(ItemId::Wood, spec.wood_yield.max(1));
            }
            VegYield::Food => {
                got_item = ItemId::Food(species);
                qty = a.try_add_item(ItemId::Food(species), food_n);
                if spec.fiber_yield > 0 {
                    let _ = a.try_add_item(ItemId::Fiber, spec.fiber_yield);
                }
            }
        }
    }
    sim.world.set_vegetation(x, y, 0);
    if qty > 0 {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.gathers_this_tick = a.gathers_this_tick.saturating_add(1);
        }
    }
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
    let cap = sim.config.memory_capacity();
    let policy = sim.config.agents.memory.eviction_policy;
    let bonus = sim.config.social_bonus_milli();
    let persist = sim.config.agents.memory.persistent_relationships;
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
    let resource = if is_veg {
        "food"
    } else if is_animal {
        "animal"
    } else if is_fish {
        "fish"
    } else {
        "food"
    };
    let nutr = crate::incentive::scale_u32(
        nutr,
        crate::incentive::resource_mult_milli(sim, id, resource),
    );

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
        agent.remember(
            cap,
            policy,
            bonus,
            persist,
            MemoryEntry {
                tick,
                kind: MemoryKind::Sickness,
                text: format!("ate {name} and felt sick"),
                importance: 90,
                last_accessed: tick,
                species_tag: tag,
                id: 0,
                participants: Vec::new(),
                valence: -80,
            },
        );
        agent.remember(
            cap,
            policy,
            bonus,
            persist,
            MemoryEntry {
                tick,
                kind: MemoryKind::ToxinFact,
                text: format!("{name} is toxic"),
                importance: 95,
                last_accessed: tick,
                species_tag: tag,
                id: 0,
                participants: Vec::new(),
                valence: -90,
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
    let bonus = if agent.has_tool(ItemId::Spear) {
        25
    } else {
        -15
    };
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
    let cell = neighbors4(&sim.world, agent.x, agent.y)
        .into_iter()
        .find(|&(x, y)| {
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
        Recipe::FishingRod => (
            vec![(ItemId::Wood, 1), (ItemId::Fiber, 1)],
            ItemId::FishingRod,
        ),
    };
    let has_all = need
        .iter()
        .all(|(item, n)| agent.inventory.get(item).copied().unwrap_or(0) >= *n);
    let room = agent.inventory_cap.saturating_sub(agent.inventory_count()) >= 1
        || agent.inventory.contains_key(&out);
    if !has_all || !room {
        push(
            sim,
            id,
            SimEventKind::Craft {
                recipe,
                success: false,
            },
        );
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
        push(
            sim,
            id,
            SimEventKind::Craft {
                recipe,
                success: false,
            },
        );
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

fn propose(
    sim: &mut Simulation,
    id: AgentId,
    text: &str,
    rule: Option<crate::board::StructuredRule>,
) {
    let cap = sim.config.proposals.max_open_proposals_per_agent as usize;
    if sim.board.author_open_count(id) >= cap {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let max_len = sim.config.proposals.max_proposal_length as usize;
    let mut text = text.to_string();
    if text.chars().count() > max_len {
        text = text.chars().take(max_len).collect();
    }
    if text.is_empty() {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let pid = sim.board.next_id;
    sim.board.next_id += 1;
    let mut supporters = std::collections::BTreeSet::new();
    supporters.insert(id);
    sim.board.proposals.push(crate::board::Proposal {
        id: pid,
        author: id,
        tick_created: sim.tick,
        text: text.clone(),
        rule,
        supporters,
        opposers: std::collections::BTreeSet::new(),
        status: crate::board::ProposalStatus::Open,
    });
    let tick = sim.tick;
    remember_agent(
        sim,
        id,
        MemoryEntry {
            tick,
            kind: MemoryKind::Proposal,
            text: format!("proposed #{pid}: {text}"),
            importance: 70,
            last_accessed: tick,
            species_tag: 0,
            id: 0,
            participants: Vec::new(),
            valence: 0,
        },
    );
    push(sim, id, SimEventKind::Propose { proposal_id: pid });
}

fn vote(sim: &mut Simulation, id: AgentId, proposal_id: u64, support: bool) {
    let author = sim
        .board
        .proposals
        .iter()
        .find(|p| p.id == proposal_id && p.status == crate::board::ProposalStatus::Open)
        .map(|p| p.author);
    let ok = if let Some(p) = sim
        .board
        .proposals
        .iter_mut()
        .find(|p| p.id == proposal_id && p.status == crate::board::ProposalStatus::Open)
    {
        if support {
            p.opposers.remove(&id);
            p.supporters.insert(id);
        } else {
            p.supporters.remove(&id);
            p.opposers.insert(id);
        }
        true
    } else {
        false
    };
    if !ok {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let tick = sim.tick;
    let mut mid = 0u64;
    remember_agent(
        sim,
        id,
        MemoryEntry {
            tick,
            kind: MemoryKind::Interaction,
            text: format!(
                "{} #{proposal_id}",
                if support { "supported" } else { "opposed" }
            ),
            importance: 60,
            last_accessed: tick,
            species_tag: 0,
            id: 0,
            participants: author.into_iter().collect(),
            valence: if support { 50 } else { -50 },
        },
    );
    if let Some(a) = sim.agents.get(&id) {
        mid = a.memory.last().map(|e| e.id).unwrap_or(0);
    }
    if sim.config.agents.social.track_relationships {
        if let Some(author) = author {
            if author != id {
                let (fwd, back) = if support {
                    (crate::social::SUPPORT, crate::social::SUPPORT_BACK)
                } else {
                    (crate::social::OPPOSE, crate::social::OPPOSE_BACK)
                };
                if let Some(a) = sim.agents.get_mut(&id) {
                    crate::social::apply_delta(
                        &mut a.relationships,
                        author,
                        tick,
                        fwd.0,
                        fwd.1,
                        fwd.2,
                        fwd.3,
                        Some(mid).filter(|x| *x != 0),
                    );
                }
                remember_agent(
                    sim,
                    author,
                    MemoryEntry {
                        tick,
                        kind: MemoryKind::Interaction,
                        text: format!(
                            "was {} on #{proposal_id} by {}",
                            if support { "supported" } else { "opposed" },
                            id.0
                        ),
                        importance: 55,
                        last_accessed: tick,
                        species_tag: 0,
                        id: 0,
                        participants: vec![id],
                        valence: if support { 40 } else { -40 },
                    },
                );
                if let Some(a) = sim.agents.get_mut(&author) {
                    crate::social::apply_delta(
                        &mut a.relationships,
                        id,
                        tick,
                        back.0,
                        back.1,
                        back.2,
                        back.3,
                        None,
                    );
                }
            }
        }
    }
    if support {
        push(sim, id, SimEventKind::Support { proposal_id });
    } else {
        push(sim, id, SimEventKind::Oppose { proposal_id });
    }
}

fn remember_obs(sim: &mut Simulation, id: AgentId, species: u8, x: u32, y: u32) {
    let tick = sim.tick;
    remember_agent(
        sim,
        id,
        MemoryEntry {
            tick,
            kind: MemoryKind::Observation,
            text: format!("saw species {species} at ({x},{y})"),
            importance: 40,
            last_accessed: tick,
            species_tag: species,
            id: 0,
            participants: Vec::new(),
            valence: 0,
        },
    );
}

pub(crate) fn remember_agent(sim: &mut Simulation, id: AgentId, mut entry: MemoryEntry) {
    let cap = sim.config.memory_capacity();
    let policy = sim.config.agents.memory.eviction_policy;
    let bonus = sim.config.social_bonus_milli();
    let persist = sim.config.agents.memory.persistent_relationships;
    let milli = crate::incentive::memory_boost_milli(sim, id, entry.kind);
    entry.importance =
        crate::incentive::scale_u32(u32::from(entry.importance), milli).min(255) as u8;
    if let Some(a) = sim.agents.get_mut(&id) {
        a.remember(cap, policy, bonus, persist, entry);
    }
}

pub fn apply_heard_memories(
    sim: &mut Simulation,
    id: AgentId,
    heard: &[crate::observation::HeardSpeech],
) {
    let tick = sim.tick;
    let species = sim.config.world.species.clone();
    let track = sim.config.agents.social.track_relationships;
    for h in heard {
        let parts: Vec<AgentId> = h.speaker.into_iter().collect();
        let already_knew: Vec<u8> = species
            .vegetation
            .iter()
            .enumerate()
            .filter_map(|(i, spec)| {
                let tag = (i + 1) as u8;
                if h.text.contains(&spec.id)
                    && (h.text.contains("toxic") || h.text.contains("sick"))
                    && sim
                        .agents
                        .get(&id)
                        .is_some_and(|a| knows_toxin(&a.memory, tag))
                {
                    Some(tag)
                } else {
                    None
                }
            })
            .collect();
        remember_agent(
            sim,
            id,
            MemoryEntry {
                tick,
                kind: MemoryKind::Utterance,
                text: h.text.clone(),
                importance: 50,
                last_accessed: tick,
                species_tag: 0,
                id: 0,
                participants: parts.clone(),
                valence: 0,
            },
        );
        for (i, spec) in species.vegetation.iter().enumerate() {
            let tag = (i + 1) as u8;
            if h.text.contains(&spec.id)
                && (h.text.contains("toxic") || h.text.contains("sick"))
                && sim
                    .agents
                    .get(&id)
                    .is_some_and(|a| !knows_toxin(&a.memory, tag))
            {
                remember_agent(
                    sim,
                    id,
                    MemoryEntry {
                        tick,
                        kind: MemoryKind::ToxinFact,
                        text: format!("{} is toxic", spec.id),
                        importance: 90,
                        last_accessed: tick,
                        species_tag: tag,
                        id: 0,
                        participants: parts.clone(),
                        valence: -40,
                    },
                );
            }
        }
        if track {
            if let Some(speaker) = h.speaker {
                if already_knew.iter().any(|_| true) {
                    if let Some(a) = sim.agents.get_mut(&id) {
                        let d = crate::social::TOXIN_CONFIRM;
                        crate::social::apply_delta(
                            &mut a.relationships,
                            speaker,
                            tick,
                            d.0,
                            d.1,
                            d.2,
                            d.3,
                            None,
                        );
                    }
                }
            }
        }
    }
}
