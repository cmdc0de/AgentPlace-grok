use crate::action::{PrimaryAction, Recipe};
use crate::agent::{Agent, AgentId, ItemId};
use crate::board::ProposalView;
use crate::event_log::SimEventKind;
use crate::simulation::Simulation;
use crate::species::VegYield;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentView {
    pub id: Option<AgentId>,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeardSpeech {
    pub speaker: Option<AgentId>,
    pub text: String,
    pub shout: bool,
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
}

pub fn chebyshev(ax: u32, ay: u32, bx: u32, by: u32) -> u32 {
    let dx = ax.abs_diff(bx);
    let dy = ay.abs_diff(by);
    dx.max(dy)
}

pub fn effective_range(base: f64, perceptiveness: u8) -> u32 {
    let factor = 0.5 + f64::from(perceptiveness) / 100.0;
    (base * factor).round().max(0.0) as u32
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
        effective_range(
            cfg.observation.base_vision_range,
            agent.personality.perceptiveness,
        )
    };
    let hear = if cfg.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        effective_range(
            cfg.communication
                .base_speech_range
                .max(cfg.observation.base_hearing_range),
            agent.personality.perceptiveness,
        )
    };
    let ident = if cfg.observation.full_information {
        sim.world.width.max(sim.world.height)
    } else {
        effective_range(
            cfg.observation.base_agent_identity_range,
            agent.personality.perceptiveness,
        )
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
            });
        }
    }

    let heard = heard_last_tick(sim, agent, hear, ident);
    let legal = legal_actions(sim, agent);
    let board = board_view(sim, ident, agent);
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
    }
}

fn board_view(sim: &Simulation, ident: u32, agent: &Agent) -> Vec<ProposalView> {
    if !sim.config.proposals.public_board_always_visible {
        return Vec::new();
    }
    sim.board
        .proposals
        .iter()
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
                rule: p.rule,
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
        let range = if *shout {
            effective_range(
                sim.config.observation.base_hearing_range
                    * sim.config.communication.shout_range_multiplier,
                listener.personality.perceptiveness,
            )
        } else {
            hear
        };
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
    for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = agent.x as i32 + dx;
        let ny = agent.y as i32 + dy;
        if sim.world.in_bounds(nx, ny) && sim.world.is_land(nx as u32, ny as u32) {
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
    for (item, qty) in &agent.inventory {
        if *qty > 0 {
            if let ItemId::Food(tag) = item {
                if *tag >= 100 || !sim.board.blocks_eat(*tag) {
                    legal.push(PrimaryAction::Eat { item: *item });
                }
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
    if can_craft(agent, Recipe::Basket) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::Basket,
        });
    }
    if can_craft(agent, Recipe::Spear) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::Spear,
        });
    }
    if can_craft(agent, Recipe::FishingRod) {
        legal.push(PrimaryAction::Craft {
            recipe: Recipe::FishingRod,
        });
    }
    if sim.board.author_open_count(agent.id)
        < sim.config.proposals.max_open_proposals_per_agent as usize
    {
        legal.push(PrimaryAction::Propose {
            text: String::new(),
            rule: None,
        });
    }
    for p in sim.board.open() {
        legal.push(PrimaryAction::Support { proposal_id: p.id });
        legal.push(PrimaryAction::Oppose { proposal_id: p.id });
    }
    legal
}

pub fn can_craft(agent: &Agent, recipe: Recipe) -> bool {
    match recipe {
        Recipe::Basket => agent.inventory.get(&ItemId::Fiber).copied().unwrap_or(0) >= 2,
        Recipe::Spear => {
            agent.inventory.get(&ItemId::Wood).copied().unwrap_or(0) >= 1
                && agent.inventory.get(&ItemId::Stone).copied().unwrap_or(0) >= 1
        }
        Recipe::FishingRod => {
            agent.inventory.get(&ItemId::Wood).copied().unwrap_or(0) >= 1
                && agent.inventory.get(&ItemId::Fiber).copied().unwrap_or(0) >= 1
        }
    }
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
    obs.agents.iter().any(|a| a.id == Some(id) || (a.id.is_none() && a.x == x && a.y == y))
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
