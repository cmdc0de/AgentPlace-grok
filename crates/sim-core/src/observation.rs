use crate::action::{PrimaryAction, Recipe};
use crate::agent::{Agent, AgentId, ItemId};
use crate::board::ProposalView;
use crate::event_log::SimEventKind;
use crate::incentive;
use crate::memory::MemoryKind;
use crate::simulation::Simulation;
use crate::species::{SpeciesTables, Toxicity, VegYield};
use crate::world::World;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileView {
    pub x: u32,
    pub y: u32,
    pub water: bool,
    pub vegetation: u8,
    pub mineral: bool,
    pub animals: u8,
    pub fish: u8,
    pub crop: bool,
    #[serde(default)]
    pub stockpile: Vec<InventoryView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentView {
    pub id: Option<AgentId>,
    pub x: u32,
    pub y: u32,
    /// Last primary kind (`move`, `eat`, …) when identified. Silhouettes omit this.
    #[serde(default)]
    pub last_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeardSpeech {
    pub speaker: Option<AgentId>,
    pub text: String,
    pub shout: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InventoryView {
    pub item: String,
    pub qty: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IncentiveView {
    pub id: String,
    #[serde(default)]
    pub description: String,
    pub start_tick: u64,
    pub end_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub agent_id: AgentId,
    pub x: u32,
    pub y: u32,
    pub vision: u32,
    pub hearing: u32,
    pub identity: u32,
    pub tiles: Vec<TileView>,
    pub agents: Vec<AgentView>,
    pub heard: Vec<HeardSpeech>,
    #[serde(default)]
    pub board: Vec<ProposalView>,
    #[serde(default)]
    pub goals: Vec<crate::board::Goal>,
    #[serde(default)]
    pub relationships: Vec<crate::social::RelationView>,
    pub legal: Vec<PrimaryAction>,
    /// Hunger on the 0–100 display scale (millipoints / 100).
    #[serde(default)]
    pub hunger: u32,
    #[serde(default)]
    pub thirst: u32,
    #[serde(default)]
    pub energy: u32,
    #[serde(default)]
    pub illness_ticks: u32,
    #[serde(default)]
    pub inventory: Vec<InventoryView>,
    #[serde(default)]
    pub pack: Vec<InventoryView>,
    #[serde(default)]
    pub allergies: Vec<String>,
    /// Named toxin facts from memory, plus WIS-detected visible toxic species.
    #[serde(default)]
    pub toxins: Vec<String>,
    /// Active incentives that `applies_to` this agent and are not `visibility = "hidden"`.
    #[serde(default)]
    pub incentives: Vec<IncentiveView>,
    /// Short-term plan from the Plan stage. Empty when unused.
    #[serde(default)]
    pub plan: Vec<String>,
    /// Kinship names (`parent #3`). Empty when unused.
    #[serde(default)]
    pub kin: Vec<String>,
    /// Invention lines. Empty when unused.
    #[serde(default)]
    pub inventions: Vec<String>,
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            agent_id: AgentId(0),
            x: 0,
            y: 0,
            vision: 0,
            hearing: 0,
            identity: 0,
            tiles: Vec::new(),
            agents: Vec::new(),
            heard: Vec::new(),
            board: Vec::new(),
            goals: Vec::new(),
            relationships: Vec::new(),
            legal: Vec::new(),
            hunger: 0,
            thirst: 0,
            energy: 0,
            illness_ticks: 0,
            inventory: Vec::new(),
            pack: Vec::new(),
            allergies: Vec::new(),
            toxins: Vec::new(),
            incentives: Vec::new(),
            plan: Vec::new(),
            kin: Vec::new(),
            inventions: Vec::new(),
        }
    }
}

pub fn chebyshev(ax: u32, ay: u32, bx: u32, by: u32) -> u32 {
    let dx = ax.abs_diff(bx);
    let dy = ay.abs_diff(by);
    dx.max(dy)
}

/// Cell used for Store/Retrieve. Overlay off: standing land. Overlay on:
/// household home when the agent is a living member within Chebyshev 1.
pub fn crate_cell(sim: &Simulation, agent: &Agent) -> Option<(u32, u32)> {
    if sim.household_crates_enabled {
        if let Some(hid) = agent.kinship.household {
            if let Some(&(hx, hy)) = sim.household_home.get(&hid) {
                if chebyshev(agent.x, agent.y, hx, hy) <= 1 {
                    return Some((hx, hy));
                }
            }
        }
    }
    if sim.world.is_land(agent.x, agent.y) {
        Some((agent.x, agent.y))
    } else {
        None
    }
}

pub fn effective_range(base: f64, perceptiveness: u8) -> u32 {
    let factor = 0.5 + f64::from(perceptiveness) / 100.0;
    (base * factor).round().max(0.0) as u32
}

/// Perception range after personality, then WIS modifier (0 unused).
pub fn perceive_range(base: f64, perceptiveness: u8, wisdom: u8) -> u32 {
    let mut sheet = crate::sheet::AbilitySheet::default();
    sheet.wisdom = wisdom;
    sheet.adjust_range(effective_range(base, perceptiveness))
}

/// Perception range after WIS, then SenseBonus (no stack).
pub fn perceive_range_for(sim: &Simulation, agent: &Agent, base: f64) -> u32 {
    crate::inventions::apply_sense_range(
        perceive_range(base, agent.personality.perceptiveness, agent.sheet.wisdom),
        &sim.inventions,
        agent.id,
    )
}

pub fn neighbors4(world: &World, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut out = vec![(x, y)];
    for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if world.in_bounds(nx, ny) {
            out.push((nx as u32, ny as u32));
        }
    }
    out
}

pub fn build(sim: &Simulation, id: AgentId) -> Observation {
    let agent = sim.agents.get(&id).expect("agent");
    let cfg = &sim.config;
    let vis = if cfg.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        perceive_range_for(sim, agent, cfg.observation.base_vision_range)
    };
    let hear = if cfg.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        perceive_range_for(
            sim,
            agent,
            cfg.communication
                .base_speech_range
                .max(cfg.observation.base_hearing_range),
        )
    };
    let ident = if cfg.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        perceive_range_for(sim, agent, cfg.observation.base_agent_identity_range)
    };

    let mut tiles = Vec::new();
    for y in 0..sim.world.height {
        for x in 0..sim.world.width {
            if chebyshev(agent.x, agent.y, x, y) <= vis {
                tiles.push(TileView {
                    x,
                    y,
                    water: sim.world.is_water(x, y),
                    vegetation: sim.world.vegetation_species(x, y),
                    mineral: sim.world.has_mineral(x, y),
                    animals: sim.world.animal_count_at(x, y),
                    fish: sim.world.fish_count_at(x, y),
                    crop: sim.world.crops.contains_key(&(x, y)),
                    stockpile: sim
                        .world
                        .stockpile_at(x, y)
                        .map(|c| {
                            c.items
                                .iter()
                                .filter(|(_, q)| **q > 0)
                                .map(|(item, qty)| InventoryView {
                                    item: item_display_name(*item, &sim.config.world.species),
                                    qty: *qty,
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                });
            }
        }
    }

    let mut agents = Vec::new();
    for other in sim.agents.values() {
        if other.id == id {
            continue;
        }
        if chebyshev(agent.x, agent.y, other.x, other.y) <= vis {
            let named = chebyshev(agent.x, agent.y, other.x, other.y) <= ident;
            agents.push(AgentView {
                id: named.then_some(other.id),
                x: other.x,
                y: other.y,
                last_action: if named {
                    last_primary_kind(sim, other.id)
                } else {
                    None
                },
            });
        }
    }

    let heard = heard_last_tick(sim, agent, hear, ident);
    let legal = legal_actions(sim, agent);
    let board = board_view(sim, ident, agent.sheet.board_cells(ident), agent);
    let relationships = agents
        .iter()
        .filter_map(|v| {
            let oid = v.id?;
            let row = agent.relationships.get(&oid)?;
            Some(crate::social::RelationView {
                id: oid,
                trust: row.trust,
                affinity: row.affinity,
                respect: row.respect,
                fear: row.fear,
            })
        })
        .collect();
    let species = &sim.config.world.species;
    let inventory = agent
        .inventory
        .iter()
        .filter(|(_, qty)| **qty > 0)
        .map(|(item, qty)| InventoryView {
            item: item_display_name(*item, species),
            qty: *qty,
        })
        .collect();
    let pack = agent
        .pack
        .iter()
        .filter(|(_, qty)| **qty > 0)
        .map(|(item, qty)| InventoryView {
            item: item_display_name(*item, species),
            qty: *qty,
        })
        .collect();
    let mut toxins = Vec::new();
    for mem in &agent.memory {
        if mem.kind != MemoryKind::ToxinFact {
            continue;
        }
        let name = species
            .veg(mem.species_tag)
            .map(|s| s.id.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| mem.text.clone());
        if !name.is_empty() && !toxins.contains(&name) {
            toxins.push(name);
        }
    }
    if agent.sheet.detects_toxins() {
        for tile in &tiles {
            if let Some(spec) = species.veg(tile.vegetation) {
                if spec.toxicity == Toxicity::Toxic
                    && !spec.id.is_empty()
                    && !toxins.contains(&spec.id)
                {
                    toxins.push(spec.id.clone());
                }
            }
        }
    }
    let incentives = sim
        .incentives
        .incentives
        .iter()
        .filter(|inc| {
            sim.incentive_active.contains(&inc.id)
                && incentive::in_scope(sim, inc, id)
                && !inc.is_hidden()
        })
        .map(|inc| IncentiveView {
            id: inc.id.clone(),
            description: inc.description.clone(),
            start_tick: inc.start_tick,
            end_tick: inc.end_tick,
        })
        .collect();
    Observation {
        agent_id: id,
        x: agent.x,
        y: agent.y,
        vision: vis,
        hearing: hear,
        identity: ident,
        tiles,
        agents,
        heard,
        board,
        goals: agent.goals.clone(),
        relationships,
        legal,
        hunger: agent.needs.hunger / 100,
        thirst: agent.needs.thirst / 100,
        energy: agent.needs.energy / 100,
        illness_ticks: agent.illness_ticks,
        inventory,
        pack,
        allergies: agent.personality.allergy_tags.clone(),
        toxins,
        incentives,
        plan: agent.plan.clone(),
        kin: {
            let mut kin = agent.kinship.lines();
            if let Some(hid) = agent.kinship.household {
                if let Some(&(hx, hy)) = sim.household_home.get(&hid) {
                    kin.push(format!("home ({hx},{hy})"));
                }
            }
            if agent.culture != 0 {
                kin.push(format!("culture {}", agent.culture));
            }
            kin
        },
        inventions: crate::inventions::observation_lines(&sim.inventions, id),
    }
}

fn last_primary_kind(sim: &Simulation, id: AgentId) -> Option<String> {
    let cur = sim.tick;
    let prev = cur.saturating_sub(1);
    sim.events
        .events
        .iter()
        .rev()
        .find(|e| {
            e.agent == id
                && (e.tick == cur || e.tick == prev)
                && crate::event_log::is_primary_kind(&e.kind)
        })
        .map(|e| crate::event_log::kind_slug(&e.kind).to_string())
}

fn open_proposal_visible(
    sim: &Simulation,
    agent: &Agent,
    ident: u32,
    p: &crate::board::Proposal,
) -> bool {
    if sim.config.proposals.public_board_always_visible || sim.config.observation.full_information {
        return true;
    }
    if p.author == agent.id || p.supporters.contains(&agent.id) || p.opposers.contains(&agent.id) {
        return true;
    }
    sim.agents
        .get(&p.author)
        .is_some_and(|a| chebyshev(agent.x, agent.y, a.x, a.y) <= ident)
}

fn board_view(sim: &Simulation, ident: u32, board: u32, agent: &Agent) -> Vec<ProposalView> {
    sim.board
        .proposals
        .iter()
        .filter(|p| {
            p.status != crate::board::ProposalStatus::Open
                || open_proposal_visible(sim, agent, board, p)
        })
        .map(|p| {
            let named = ident >= 255
                || sim.config.observation.full_information
                || sim
                    .agents
                    .get(&p.author)
                    .is_some_and(|a| chebyshev(agent.x, agent.y, a.x, a.y) <= ident)
                || p.author == agent.id;
            ProposalView {
                id: p.id,
                author: named.then_some(p.author),
                text: p.text.clone(),
                status: p.status,
                support: p.supporters.len() as u32,
                oppose: p.opposers.len() as u32,
                rule: p.rule.clone(),
                you_support: p.supporters.contains(&agent.id),
                you_oppose: p.opposers.contains(&agent.id),
            }
        })
        .collect()
}

fn heard_last_tick(sim: &Simulation, listener: &Agent, hear: u32, ident: u32) -> Vec<HeardSpeech> {
    if sim.tick == 0 {
        return Vec::new();
    }
    let prev = sim.tick - 1;
    let mut out = Vec::new();
    for event in &sim.events.events {
        if event.tick != prev {
            continue;
        }
        let SimEventKind::Speak {
            shout,
            text,
            broadcast,
            targets,
        } = &event.kind
        else {
            continue;
        };
        let Some(speaker) = sim.agents.get(&event.agent) else {
            continue;
        };
        let range = speaker.sheet.adjust_speech_range(if *shout {
            perceive_range_for(
                sim,
                listener,
                sim.config.observation.base_hearing_range
                    * sim.config.communication.shout_range_multiplier,
            )
        } else {
            hear
        });
        let dist = chebyshev(listener.x, listener.y, speaker.x, speaker.y);
        if sim.config.observation.full_information {
            // audible
        } else if *broadcast {
            if dist > range {
                continue;
            }
        } else {
            let is_target = targets.iter().any(|id| *id == listener.id);
            if is_target {
                if dist > range {
                    continue;
                }
            } else if sim.config.communication.allow_overhearing {
                // Overhear margin: closer than full hearing range.
                if dist > range.saturating_sub(range / 3).max(1) {
                    continue;
                }
            } else {
                continue;
            }
        }
        let named = dist <= ident || sim.config.observation.full_information;
        out.push(HeardSpeech {
            speaker: named.then_some(event.agent),
            text: text.clone(),
            shout: *shout,
        });
    }
    out
}

pub fn legal_actions(sim: &Simulation, agent: &Agent) -> Vec<PrimaryAction> {
    let mut legal = vec![PrimaryAction::Wait, PrimaryAction::Rest];
    let params = sim.storage;
    let energy = agent.needs.energy;
    let move_cost = crate::inventions::apply_move_cost(
        agent.move_cost_milli(&params),
        &sim.inventions,
        agent.id,
    );
    for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = agent.x as i32 + dx;
        let ny = agent.y as i32 + dy;
        if sim.world.in_bounds(nx, ny)
            && sim.world.is_land(nx as u32, ny as u32)
            && energy >= move_cost
        {
            legal.push(PrimaryAction::MoveRelative { dx, dy });
        }
    }
    let near = neighbors4(&sim.world, agent.x, agent.y);
    let mut seen_gather = Vec::new();
    let mut can_drink = false;
    let mut can_hunt = false;
    let mut can_fish = false;
    let mut farm_spots = false;
    for &(x, y) in &near {
        let tag = sim.world.vegetation_species(x, y);
        if tag != 0 && !seen_gather.contains(&tag) && !sim.board.blocks_gather(tag) {
            seen_gather.push(tag);
            if sim
                .board
                .max_gather_per_tick()
                .is_none_or(|m| agent.gathers_this_tick < m)
            {
                legal.push(PrimaryAction::Gather { species: tag });
            }
        }
        if sim.world.has_mineral(x, y) && !seen_gather.contains(&0) {
            seen_gather.push(0);
            legal.push(PrimaryAction::Gather { species: 0 });
        }
        if sim.world.is_water(x, y) {
            can_drink = true;
            if sim.world.fish_count_at(x, y) > 0 {
                can_fish = true;
            }
        }
        if sim.world.animal_count_at(x, y) > 0 {
            can_hunt = true;
        }
        if sim.world.is_land(x, y)
            && tag == 0
            && !sim.world.has_mineral(x, y)
            && !sim.world.crops.contains_key(&(x, y))
        {
            farm_spots = true;
        }
    }
    if can_drink {
        legal.push(PrimaryAction::Drink);
    }
    if can_hunt {
        legal.push(PrimaryAction::Hunt);
    }
    if can_fish {
        legal.push(PrimaryAction::Fish);
    }
    let mut eat_seen = Vec::new();
    for (item, qty) in agent.inventory.iter().chain(agent.pack.iter()) {
        if *qty == 0 {
            continue;
        }
        if let ItemId::Food(tag) = item {
            if eat_seen.contains(item) {
                continue;
            }
            if *tag >= 100 || !sim.board.blocks_eat(*tag) {
                eat_seen.push(*item);
                legal.push(PrimaryAction::Eat { item: *item });
            }
        }
    }
    if farm_spots {
        for (item, qty) in &agent.inventory {
            if *qty == 0 {
                continue;
            }
            if let ItemId::Food(tag) = *item {
                if sim
                    .config
                    .world
                    .species
                    .veg(tag)
                    .is_some_and(|s| s.yield_kind == VegYield::Food)
                {
                    legal.push(PrimaryAction::Farm { species: tag });
                }
            }
        }
    }
    if can_craft(agent, Recipe::Basket, &sim.catalog) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::Basket,
        });
    }
    if can_craft(agent, Recipe::Spear, &sim.catalog) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::Spear,
        });
    }
    if can_craft(agent, Recipe::FishingRod, &sim.catalog) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::FishingRod,
        });
    }
    if can_craft(agent, Recipe::Backpack, &sim.catalog) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::Backpack,
        });
    }
    for entry in &sim.catalog {
        if let Some(Recipe::Catalog(n)) = entry.recipe {
            let recipe = Recipe::Catalog(n);
            if can_craft(agent, recipe, &sim.catalog) {
                legal.push(PrimaryAction::Craft { recipe });
            }
        }
    }
    if sim.board.author_open_count(agent.id)
        < sim.config.proposals.max_open_proposals_per_agent as usize
    {
        legal.push(PrimaryAction::Propose {
            text: String::new(),
            rule: None,
        });
    }
    let ident = if sim.config.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        perceive_range_for(sim, agent, sim.config.observation.base_agent_identity_range)
    };
    for p in sim.board.open() {
        if !open_proposal_visible(sim, agent, ident, p) {
            continue;
        }
        legal.push(PrimaryAction::Support { proposal_id: p.id });
        legal.push(PrimaryAction::Oppose { proposal_id: p.id });
    }
    if let Some((cx, cy)) = crate_cell(sim, agent) {
        let cell = sim
            .world
            .stockpiles
            .get(&(cx, cy))
            .cloned()
            .unwrap_or_default();
        let mut store_seen = Vec::new();
        for item in agent
            .inventory
            .keys()
            .chain(agent.pack.keys())
            .copied()
            .collect::<Vec<_>>()
        {
            if store_seen.contains(&item) {
                continue;
            }
            let in_pack =
                agent.has_pack(&params) && agent.pack.get(&item).copied().unwrap_or(0) > 0;
            let in_pockets = agent.inventory.get(&item).copied().unwrap_or(0) > 0;
            if !in_pack && !in_pockets {
                continue;
            }
            let haul = if in_pack {
                params.pack_haul_milli
            } else {
                params.haul_milli
            };
            let cost = crate::haul::haul_cost_milli(item, 1, haul);
            if energy < cost || !cell.can_add(item, 1, &params) {
                continue;
            }
            if Agent::is_pack_carrier(item)
                && !can_drop_worn_carrier(sim, agent, item, 1, Some((item, 1)))
            {
                continue;
            }
            store_seen.push(item);
            legal.push(PrimaryAction::Store { item, qty: 1 });
        }
        for (item, have) in &cell.items {
            if *have == 0 {
                continue;
            }
            let cost = crate::haul::haul_cost_milli(*item, 1, params.haul_milli);
            if energy >= cost && agent.pocket_fit_qty(*item) >= 1 {
                legal.push(PrimaryAction::Retrieve {
                    item: *item,
                    qty: 1,
                });
            }
        }
    }
    if agent.has_pack(&params) {
        let (slot_cap, weight_cap) = agent.worn_pack_caps(&params);
        for (item, have) in &agent.inventory {
            if *have == 0 || Agent::is_pack_carrier(*item) {
                continue;
            }
            let cost = crate::haul::haul_cost_milli(*item, 1, params.pack_haul_milli);
            if energy >= cost && crate::haul::can_fit(&agent.pack, *item, 1, slot_cap, weight_cap) {
                legal.push(PrimaryAction::Pack {
                    item: *item,
                    qty: 1,
                });
            }
        }
        for (item, have) in &agent.pack {
            if *have == 0 {
                continue;
            }
            let cost = crate::haul::haul_cost_milli(*item, 1, params.pack_haul_milli);
            if energy >= cost && agent.pocket_fit_qty(*item) >= 1 {
                legal.push(PrimaryAction::Unpack {
                    item: *item,
                    qty: 1,
                });
            }
        }
    }
    let ident = if sim.config.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        perceive_range_for(sim, agent, sim.config.observation.base_agent_identity_range)
    };
    for other in sim.agents.values() {
        if other.id == agent.id {
            continue;
        }
        let dist = chebyshev(agent.x, agent.y, other.x, other.y);
        if dist > 1 {
            continue;
        }
        if dist > ident {
            continue;
        }
        if other.inventory_count() >= other.pocket_slot_cap() {
            continue;
        }
        let mut xfer_seen = Vec::new();
        for item in agent
            .inventory
            .keys()
            .chain(agent.pack.keys())
            .copied()
            .collect::<Vec<_>>()
        {
            if xfer_seen.contains(&item) {
                continue;
            }
            let in_pack = agent.has_pack(&params)
                && !Agent::is_pack_carrier(item)
                && agent.pack.get(&item).copied().unwrap_or(0) > 0;
            let in_pockets = agent.inventory.get(&item).copied().unwrap_or(0) > 0;
            if !in_pack && !in_pockets {
                continue;
            }
            let haul = if in_pack {
                params.pack_haul_milli
            } else {
                params.haul_milli
            };
            let cost = crate::haul::haul_cost_milli(item, 1, haul);
            if energy < cost || other.pocket_fit_qty(item) < 1 {
                continue;
            }
            if Agent::is_pack_carrier(item) && !can_drop_worn_carrier(sim, agent, item, 1, None) {
                continue;
            }
            xfer_seen.push(item);
            legal.push(PrimaryAction::Transfer {
                item,
                qty: 1,
                to: other.id,
            });
        }
    }
    if sim.conflict_enabled && !agent.incapacitated {
        let vis = if sim.config.observation.full_information {
            sim.world.width.max(sim.world.height)
        } else {
            perceive_range_for(sim, agent, sim.config.observation.base_vision_range)
        };
        let mut any_visible = false;
        for other in sim.agents.values() {
            if other.id == agent.id || other.incapacitated {
                continue;
            }
            let dist = chebyshev(agent.x, agent.y, other.x, other.y);
            if dist == 1
                && agent.needs.energy >= crate::conflict::ATTACK_ENERGY_COST
                && !sim.is_child(agent)
            {
                legal.push(PrimaryAction::Attack { target: other.id });
            }
            if dist <= vis {
                any_visible = true;
            }
        }
        if any_visible {
            legal.push(PrimaryAction::Flee);
        }
    }
    if sim.reproduction_enabled && !agent.incapacitated {
        let floor = agent.sheet.energy_max(sim.config.energy_max_milli()) / 2;
        for other in sim.agents.values() {
            if other.id == agent.id || other.incapacitated {
                continue;
            }
            if chebyshev(agent.x, agent.y, other.x, other.y) != 1 {
                continue;
            }
            let unbound = agent.kinship.pair_bond.is_none() && other.kinship.pair_bond.is_none();
            if unbound
                && !sim.is_child(agent)
                && !sim.is_child(other)
                && !crate::kinship::close_kin(agent, other)
            {
                legal.push(PrimaryAction::PairBond { target: other.id });
            }
            let mutual = agent.kinship.pair_bond == Some(other.id)
                && other.kinship.pair_bond == Some(agent.id);
            if mutual
                && agent.needs.energy >= floor
                && other.needs.energy >= floor
                && !sim.is_child(agent)
                && !sim.is_child(other)
            {
                legal.push(PrimaryAction::Reproduce { with: other.id });
            }
        }
    }
    if sim.inventions_enabled
        && !agent.incapacitated
        && !sim.is_child(agent)
        && crate::inventions::next_kind(&sim.inventions).is_some()
    {
        legal.push(PrimaryAction::Invent);
    }
    legal
}

pub(crate) fn can_drop_worn_carrier(
    sim: &Simulation,
    agent: &Agent,
    removing: ItemId,
    qty: u32,
    crate_reserved: Option<(crate::agent::ItemId, u32)>,
) -> bool {
    if !Agent::is_pack_carrier(removing) {
        return true;
    }
    let mut baskets = agent.basket_count();
    let mut backpacks = agent.backpack_count();
    match removing {
        ItemId::Basket => baskets = baskets.saturating_sub(qty),
        ItemId::Backpack => backpacks = backpacks.saturating_sub(qty),
        _ => {}
    }
    let (slots, w) = Agent::worn_pack_caps_for(baskets, backpacks, &sim.storage);
    if slots > 0 || w > 0 {
        return agent.pack_count() <= slots && agent.pack_weight_milli() <= w;
    }
    if agent.pack_count() == 0 {
        return true;
    }
    let extra = if crate_reserved.is_some() { 1 } else { qty };
    let leftover = agent.split_pack_unload(extra).1;
    let (cx, cy) = if crate_reserved.is_some() {
        crate_cell(sim, agent).unwrap_or((agent.x, agent.y))
    } else {
        (agent.x, agent.y)
    };
    leftover.is_empty()
        || sim
            .world
            .crate_can_take(cx, cy, crate_reserved, &leftover, &sim.storage)
}

pub fn can_craft(agent: &Agent, recipe: Recipe, catalog: &[crate::objects::CatalogEntry]) -> bool {
    let Some((need, _, _)) = crate::objects::recipe_spec(recipe, catalog) else {
        return false;
    };
    need.iter()
        .all(|(item, n)| agent.inventory.get(item).copied().unwrap_or(0) >= *n)
}

pub fn encode_for_hash(obs: &Observation) -> Vec<u8> {
    postcard::to_allocvec(obs).unwrap_or_default()
}

/// True if cell `(x, y)` appears in this observation's tile list.
pub fn visible_in_observation(obs: &Observation, x: u32, y: u32) -> bool {
    obs.tiles.iter().any(|t| t.x == x && t.y == y)
}

/// True if `id` is the observer or appears (named or as a silhouette) in `obs.agents`.
pub fn agent_visible_in_observation(obs: &Observation, id: AgentId, x: u32, y: u32) -> bool {
    if obs.agent_id == id {
        return true;
    }
    obs.agents
        .iter()
        .any(|a| a.id == Some(id) || (a.id.is_none() && a.x == x && a.y == y))
}

/// Legal-list matching. `Propose` in the list is a placeholder with empty text.
pub fn is_legal_choice(legal: &[PrimaryAction], action: &PrimaryAction) -> bool {
    match action {
        PrimaryAction::Propose { text, .. } => {
            !text.is_empty()
                && legal
                    .iter()
                    .any(|a| matches!(a, PrimaryAction::Propose { .. }))
        }
        PrimaryAction::Support { proposal_id } => legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Support { proposal_id: id } if id == proposal_id)),
        PrimaryAction::Oppose { proposal_id } => legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Oppose { proposal_id: id } if id == proposal_id)),
        other => legal.iter().any(|a| a == other),
    }
}

pub fn item_display_name(item: ItemId, species: &SpeciesTables) -> String {
    match item {
        ItemId::Food(100) => "hare".into(),
        ItemId::Food(101) => "perch".into(),
        ItemId::Food(tag) => species
            .veg(tag)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| format!("food:{tag}")),
        ItemId::Wood => "wood".into(),
        ItemId::Fiber => "fiber".into(),
        ItemId::Stone => "stone".into(),
        ItemId::Basket => "basket".into(),
        ItemId::Spear => "spear".into(),
        ItemId::FishingRod => "fishing_rod".into(),
        ItemId::Backpack => "backpack".into(),
        ItemId::Catalog(n) => format!("catalog:{n}"),
    }
}

/// Species names (not Debug tags) so an LLM can pick Gather/Eat without raw u8s.
pub fn format_primary(action: &PrimaryAction, species: &SpeciesTables) -> String {
    match action {
        PrimaryAction::Wait => "Wait".into(),
        PrimaryAction::Rest => "Rest".into(),
        PrimaryAction::Drink => "Drink".into(),
        PrimaryAction::Hunt => "Hunt".into(),
        PrimaryAction::Fish => "Fish".into(),
        PrimaryAction::MoveRelative { dx, dy } => format!("MoveRelative dx={dx} dy={dy}"),
        PrimaryAction::Gather { species: 0 } => "Gather stone".into(),
        PrimaryAction::Gather { species: tag } => {
            format!("Gather {}", item_display_name(ItemId::Food(*tag), species))
        }
        PrimaryAction::Farm { species: tag } => {
            format!("Farm {}", item_display_name(ItemId::Food(*tag), species))
        }
        PrimaryAction::Eat { item } => format!("Eat {}", item_display_name(*item, species)),
        PrimaryAction::Craft { recipe } => {
            let name = match recipe {
                Recipe::Basket => "basket".into(),
                Recipe::Spear => "spear".into(),
                Recipe::FishingRod => "fishing_rod".into(),
                Recipe::Backpack => "backpack".into(),
                Recipe::Catalog(n) => format!("catalog:{n}"),
            };
            format!("Craft {name}")
        }
        PrimaryAction::Propose { text, rule } => {
            if text.is_empty() {
                "Propose".into()
            } else {
                format!("Propose {text:?} rule={rule:?}")
            }
        }
        PrimaryAction::Support { proposal_id } => format!("Support #{proposal_id}"),
        PrimaryAction::Oppose { proposal_id } => format!("Oppose #{proposal_id}"),
        PrimaryAction::Transfer { item, qty, to } => {
            format!(
                "Transfer {}×{} to #{}",
                item_display_name(*item, species),
                qty,
                to.0
            )
        }
        PrimaryAction::Store { item, qty } => {
            format!("Store {}×{}", item_display_name(*item, species), qty)
        }
        PrimaryAction::Retrieve { item, qty } => {
            format!("Retrieve {}×{}", item_display_name(*item, species), qty)
        }
        PrimaryAction::Pack { item, qty } => {
            format!("Pack {}×{}", item_display_name(*item, species), qty)
        }
        PrimaryAction::Unpack { item, qty } => {
            format!("Unpack {}×{}", item_display_name(*item, species), qty)
        }
        PrimaryAction::Attack { target } => format!("Attack #{}", target.0),
        PrimaryAction::Flee => "Flee".into(),
        PrimaryAction::PairBond { target } => format!("PairBond #{}", target.0),
        PrimaryAction::Reproduce { with } => format!("Reproduce #{}", with.0),
        PrimaryAction::Invent => "Invent".into(),
    }
}

/// Compact visible-resource list for prompts. Caps length; skips empty cells.
pub fn visible_summary(obs: &Observation, species: &SpeciesTables, cap: usize) -> String {
    let mut parts = Vec::new();
    for t in &obs.tiles {
        if parts.len() >= cap {
            break;
        }
        let dx = t.x as i32 - obs.x as i32;
        let dy = t.y as i32 - obs.y as i32;
        if t.water {
            parts.push(format!("water@{dx},{dy}"));
            if parts.len() >= cap {
                break;
            }
        }
        if t.vegetation != 0 {
            let name = species
                .veg(t.vegetation)
                .map(|s| s.id.as_str())
                .unwrap_or("veg");
            parts.push(format!("{name}@{dx},{dy}"));
            if parts.len() >= cap {
                break;
            }
        }
        if t.mineral {
            parts.push(format!("stone@{dx},{dy}"));
            if parts.len() >= cap {
                break;
            }
        }
        if t.animals > 0 {
            parts.push(format!("hare×{}@{dx},{dy}", t.animals));
            if parts.len() >= cap {
                break;
            }
        }
        if t.fish > 0 {
            parts.push(format!("perch×{}@{dx},{dy}", t.fish));
            if parts.len() >= cap {
                break;
            }
        }
        if t.crop {
            parts.push(format!("crop@{dx},{dy}"));
            if parts.len() >= cap {
                break;
            }
        }
        if !t.stockpile.is_empty() {
            let inside: Vec<_> = t
                .stockpile
                .iter()
                .map(|i| format!("{}×{}", i.item, i.qty))
                .collect();
            parts.push(format!("stockpile@{dx},{dy}: {}", inside.join(",")));
        }
    }
    parts.join("; ")
}
