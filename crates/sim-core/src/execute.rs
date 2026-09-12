use crate::action::{PrimaryAction, Recipe};
use crate::agent::{Agent, AgentId, ItemId};
use crate::event_log::{SimEvent, SimEventKind};
use crate::memory::{MemoryEntry, MemoryKind, knows_toxin};
use crate::observation::{crate_cell, neighbors4};
use crate::simulation::Simulation;
use crate::species::{Crop, Toxicity, VegYield};
use rand::Rng;
use rand_chacha::ChaCha20Rng;

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
        PrimaryAction::Propose { text, rule } => propose(sim, id, text, rule.clone()),
        PrimaryAction::Support { proposal_id } => vote(sim, id, *proposal_id, true),
        PrimaryAction::Oppose { proposal_id } => vote(sim, id, *proposal_id, false),
        PrimaryAction::Transfer { item, qty, to } => transfer(sim, id, *item, *qty, *to),
        PrimaryAction::Store { item, qty } => store(sim, id, *item, *qty),
        PrimaryAction::Retrieve { item, qty } => retrieve(sim, id, *item, *qty),
        PrimaryAction::Pack { item, qty } => pack_item(sim, id, *item, *qty),
        PrimaryAction::Unpack { item, qty } => unpack_item(sim, id, *item, *qty),
        PrimaryAction::Attack { target } => attack(sim, id, *target),
        PrimaryAction::Flee => flee(sim, id),
        PrimaryAction::PairBond { target } => pair_bond(sim, id, *target),
        PrimaryAction::Reproduce { with } => reproduce(sim, id, *with),
        PrimaryAction::Invent => invent(sim, id),
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

fn pay_energy(sim: &mut Simulation, id: AgentId, cost: u32) -> bool {
    let Some(a) = sim.agents.get_mut(&id) else {
        return false;
    };
    if a.needs.energy < cost {
        return false;
    }
    a.needs.energy -= cost;
    true
}

fn pay_haul(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32) -> bool {
    let cost = crate::haul::haul_cost_milli(item, qty, sim.storage.haul_milli);
    pay_energy(sim, id, cost)
}

fn source_haul(sim: &Simulation, id: AgentId, item: ItemId) -> u32 {
    let Some(a) = sim.agents.get(&id) else {
        return sim.storage.haul_milli;
    };
    if !Agent::is_pack_carrier(item)
        && a.has_pack(&sim.storage)
        && a.pack.get(&item).copied().unwrap_or(0) > 0
    {
        sim.storage.pack_haul_milli
    } else {
        sim.storage.haul_milli
    }
}

fn unload_pack_after_last_basket(sim: &mut Simulation, id: AgentId) -> bool {
    let Some(agent) = sim.agents.get(&id) else {
        return false;
    };
    if agent.has_pack(&sim.storage) {
        let (slots, w) = agent.worn_pack_caps(&sim.storage);
        return agent.pack_count() <= slots && agent.pack_weight_milli() <= w;
    }
    if agent.pack_count() == 0 {
        return true;
    }
    let (to_pockets, leftover) = agent.split_pack_unload(0);
    let (x, y) = (agent.x, agent.y);
    let params = sim.storage;
    if !leftover.is_empty() && !sim.world.crate_can_take(x, y, None, &leftover, &params) {
        return false;
    }
    let Some(a) = sim.agents.get_mut(&id) else {
        return false;
    };
    for (item, qty) in &to_pockets {
        if !a.take_pack(*item, *qty) {
            return false;
        }
        if a.try_add_item(*item, *qty) < *qty {
            a.try_add_pack(*item, *qty, &params);
            return false;
        }
    }
    for (item, qty) in leftover {
        if !sim
            .agents
            .get_mut(&id)
            .is_some_and(|a| a.take_pack(item, qty))
        {
            return false;
        }
        if !sim.world.try_store(x, y, item, qty, &params) {
            if let Some(a) = sim.agents.get_mut(&id) {
                a.try_add_pack(item, qty, &params);
            }
            return false;
        }
        push(sim, id, SimEventKind::Store { item, qty });
    }
    true
}

fn pack_item(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32) {
    if Agent::is_pack_carrier(item) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let Some(agent) = sim.agents.get(&id) else {
        return;
    };
    if !agent.has_pack(&sim.storage) || agent.inventory.get(&item).copied().unwrap_or(0) < qty {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let params = sim.storage;
    let (slot_cap, weight_cap) = agent.worn_pack_caps(&params);
    if !crate::haul::can_fit(&agent.pack, item, qty, slot_cap, weight_cap) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let cost = crate::haul::haul_cost_milli(item, qty, params.pack_haul_milli);
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let Some(a) = sim.agents.get_mut(&id) else {
        return;
    };
    if !a.take_item(item, qty) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if a.try_add_pack(item, qty, &params) < qty {
        a.try_add_item(item, qty);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    push(sim, id, SimEventKind::Pack { item, qty });
}

fn unpack_item(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32) {
    let Some(agent) = sim.agents.get(&id) else {
        return;
    };
    if !agent.has_pack(&sim.storage) || agent.pack.get(&item).copied().unwrap_or(0) < qty {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if agent.pocket_fit_qty(item) < qty {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let cost = crate::haul::haul_cost_milli(item, qty, sim.storage.pack_haul_milli);
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let Some(a) = sim.agents.get_mut(&id) else {
        return;
    };
    if !a.take_pack(item, qty) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if a.try_add_item(item, qty) < qty {
        a.try_add_pack(item, qty, &sim.storage);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    push(sim, id, SimEventKind::Unpack { item, qty });
}

fn attack(sim: &mut Simulation, id: AgentId, target: AgentId) {
    if id == target || !sim.agents.contains_key(&target) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let Some(atk) = sim.agents.get(&id) else {
        return;
    };
    let Some(def) = sim.agents.get(&target) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let dist = crate::observation::chebyshev(atk.x, atk.y, def.x, def.y);
    if dist != 1 {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if def.incapacitated {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let cost = crate::conflict::ATTACK_ENERGY_COST;
    let str_score = atk.sheet.strength;
    let def_dex = def.sheet.dexterity;
    let hit = crate::sheet::AbilitySheet::attack_hits(
        sim.config.master_seed,
        sim.tick,
        id.0,
        str_score,
        def_dex,
    );
    let damage = if hit { atk.sheet.attack_damage() } else { 0 };
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let mut down = false;
    let mut lethal = false;
    if damage > 0 {
        if let Some(d) = sim.agents.get_mut(&target) {
            d.needs.energy = d.needs.energy.saturating_sub(damage);
            d.health = d.health.saturating_sub(damage);
            if d.health == 0 {
                if sim.conflict_death_enabled {
                    lethal = true;
                } else if !d.incapacitated {
                    d.incapacitated = true;
                    down = true;
                }
            }
        }
    }
    push(sim, id, SimEventKind::Attack { target, damage });
    if down {
        push(sim, target, SimEventKind::Incapacitated { by: id });
    }
    if lethal {
        push(sim, target, SimEventKind::CombatDeath { by: id });
        sim.agents.remove(&target);
    }
}

fn pair_bond(sim: &mut Simulation, id: AgentId, target: AgentId) {
    if id == target || !sim.reproduction_enabled {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let existing = {
        let Some(a) = sim.agents.get(&id) else {
            return;
        };
        let Some(b) = sim.agents.get(&target) else {
            push(sim, id, SimEventKind::Wait);
            return;
        };
        if a.incapacitated || b.incapacitated {
            push(sim, id, SimEventKind::Wait);
            return;
        }
        if crate::observation::chebyshev(a.x, a.y, b.x, b.y) != 1 {
            push(sim, id, SimEventKind::Wait);
            return;
        }
        if a.kinship.pair_bond.is_some() || b.kinship.pair_bond.is_some() {
            push(sim, id, SimEventKind::Wait);
            return;
        }
        if crate::kinship::close_kin(a, b) {
            push(sim, id, SimEventKind::Wait);
            return;
        }
        if !crate::sheet::AbilitySheet::pair_bond_hits(
            sim.config.master_seed,
            sim.tick,
            id.0,
            a.sheet.charisma,
        ) {
            push(sim, id, SimEventKind::Wait);
            return;
        }
        a.kinship.household.or(b.kinship.household)
    };
    let hid = existing.unwrap_or_else(|| {
        let h = sim.next_household_id;
        sim.next_household_id = sim.next_household_id.saturating_add(1);
        h
    });
    if let Some(ag) = sim.agents.get_mut(&id) {
        ag.kinship.pair_bond = Some(target);
        ag.kinship.household = Some(hid);
    }
    if let Some(ag) = sim.agents.get_mut(&target) {
        ag.kinship.pair_bond = Some(id);
        ag.kinship.household = Some(hid);
    }
    if sim.household_crates_enabled && !sim.household_home.contains_key(&hid) {
        if let Some(ag) = sim.agents.get(&id) {
            if sim.world.is_land(ag.x, ag.y) {
                sim.household_home.insert(hid, (ag.x, ag.y));
            }
        }
    }
    push(sim, id, SimEventKind::PairBonded { with: target });
}

fn reproduce(sim: &mut Simulation, id: AgentId, with: AgentId) {
    if id == with || !sim.reproduction_enabled {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let floor = {
        let Some(a) = sim.agents.get(&id) else {
            return;
        };
        a.sheet.energy_max(sim.config.energy_max_milli()) / 2
    };
    let Some(a) = sim.agents.get(&id) else {
        return;
    };
    let Some(b) = sim.agents.get(&with) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    if a.incapacitated || b.incapacitated {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if a.needs.energy < floor || b.needs.energy < floor {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if a.kinship.pair_bond != Some(with) || b.kinship.pair_bond != Some(id) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if crate::observation::chebyshev(a.x, a.y, b.x, b.y) != 1 {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let ax = a.x;
    let ay = a.y;
    let occupied: std::collections::BTreeSet<(u32, u32)> =
        sim.agents.values().map(|ag| (ag.x, ag.y)).collect();
    let mut cell = None;
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = ax as i32 + dx;
            let ny = ay as i32 + dy;
            if !sim.world.in_bounds(nx, ny) {
                continue;
            }
            let ux = nx as u32;
            let uy = ny as u32;
            if sim.world.is_land(ux, uy) && !occupied.contains(&(ux, uy)) {
                cell = Some((ux, uy));
                break;
            }
        }
        if cell.is_some() {
            break;
        }
    }
    let Some((cx, cy)) = cell else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let child_id = AgentId(sim.next_agent_id);
    sim.next_agent_id = sim.next_agent_id.saturating_add(1);
    let (parent_a, parent_b) = if id.0 <= with.0 {
        (id, with)
    } else {
        (with, id)
    };
    let seed = crate::seeding::derive_seed(
        sim.config.master_seed,
        &format!("tick_{}_birth_{}", sim.tick, child_id.0),
    );
    let mut rng = crate::seeding::rng_from_seed(seed);
    let sheet = crate::sheet::AbilitySheet::mix(
        &sim.agents.get(&id).unwrap().sheet,
        &sim.agents.get(&with).unwrap().sheet,
        &mut rng,
    );
    let mut child = crate::agent::Agent::new(child_id, cx, cy);
    child.inventory_cap = sim.config.agents.inventory_capacity;
    child.needs = crate::agent::Needs::maxed(
        sim.config.hunger_max_milli(),
        sim.config.thirst_max_milli(),
        sheet.energy_max(sim.config.energy_max_milli()),
    );
    let pa = sim.agents.get(&id).unwrap();
    let pb = sim.agents.get(&with).unwrap();
    child.abilities = crate::agent::Abilities {
        gather: crate::sheet::mix_stat(pa.abilities.gather, pb.abilities.gather, &mut rng),
        hunt: crate::sheet::mix_stat(pa.abilities.hunt, pb.abilities.hunt, &mut rng),
        fish: crate::sheet::mix_stat(pa.abilities.fish, pb.abilities.fish, &mut rng),
        farm: crate::sheet::mix_stat(pa.abilities.farm, pb.abilities.farm, &mut rng),
        craft: crate::sheet::mix_stat(pa.abilities.craft, pb.abilities.craft, &mut rng),
    };
    child.personality = crate::agent::Personality {
        openness: crate::sheet::mix_stat(
            pa.personality.openness,
            pb.personality.openness,
            &mut rng,
        ),
        conscientiousness: crate::sheet::mix_stat(
            pa.personality.conscientiousness,
            pb.personality.conscientiousness,
            &mut rng,
        ),
        extraversion: crate::sheet::mix_stat(
            pa.personality.extraversion,
            pb.personality.extraversion,
            &mut rng,
        ),
        agreeableness: crate::sheet::mix_stat(
            pa.personality.agreeableness,
            pb.personality.agreeableness,
            &mut rng,
        ),
        neuroticism: crate::sheet::mix_stat(
            pa.personality.neuroticism,
            pb.personality.neuroticism,
            &mut rng,
        ),
        perceptiveness: crate::sheet::mix_stat(
            pa.personality.perceptiveness,
            pb.personality.perceptiveness,
            &mut rng,
        ),
        traits: Vec::new(),
        allergy_tags: Vec::new(),
    };
    child.sheet = sheet;
    child.health = sheet.health_max();
    child.culture = {
        let ca = sim.agents.get(&parent_a).map(|p| p.culture).unwrap_or(0);
        let cb = sim.agents.get(&parent_b).map(|p| p.culture).unwrap_or(0);
        if ca != 0 { ca } else { cb }
    };
    child.kinship.parents = vec![parent_a, parent_b];
    child.kinship.household = sim
        .agents
        .get(&id)
        .and_then(|p| p.kinship.household)
        .or_else(|| sim.agents.get(&with).and_then(|p| p.kinship.household));
    child.age_ticks = 0;
    child.influence_factor = sim.config.influence_milli();
    if sim.config.agents.start_with_basic_needs {
        child.goals = vec![crate::board::Goal {
            id: 0,
            text: "stay fed".into(),
            priority: 80,
            source: "birth".into(),
        }];
    }
    let sibs: Vec<AgentId> = {
        let mut s = Vec::new();
        for p in [id, with] {
            if let Some(ag) = sim.agents.get(&p) {
                for c in &ag.kinship.children {
                    crate::kinship::push_unique(&mut s, *c);
                }
            }
        }
        s
    };
    child.kinship.siblings = sibs.clone();
    for sib in &sibs {
        if let Some(ag) = sim.agents.get_mut(sib) {
            crate::kinship::push_unique(&mut ag.kinship.siblings, child_id);
        }
    }
    if let Some(ag) = sim.agents.get_mut(&id) {
        crate::kinship::push_unique(&mut ag.kinship.children, child_id);
    }
    if let Some(ag) = sim.agents.get_mut(&with) {
        crate::kinship::push_unique(&mut ag.kinship.children, child_id);
    }
    sim.agents.insert(child_id, child);
    let _ = sim.rngs.agent_stream(child_id);
    push(sim, child_id, SimEventKind::Born { parent_a, parent_b });
}

fn invent(sim: &mut Simulation, id: AgentId) {
    if !sim.inventions_enabled {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let Some(kind) = crate::inventions::next_kind(&sim.inventions, sim.invention_tree) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let Some(agent) = sim.agents.get(&id) else {
        return;
    };
    if agent.incapacitated || sim.is_child(agent) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let intel = agent.sheet.intelligence;
    let chance = crate::inventions::invent_chance(intel);
    let seed = crate::seeding::derive_seed(
        sim.config.master_seed,
        &format!("tick_{}_agent_{}_invent_0", sim.tick, id.0),
    );
    let mut rng = crate::seeding::rng_from_seed(seed);
    let roll: u32 = rng.random_range(0..1000);
    if (roll as i32) >= chance {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let mock_flavor = kind.memory_text();
    let flavor_seed = crate::seeding::derive_seed(
        sim.config.master_seed,
        &format!("tick_{}_agent_{}_invent_flavor_0", sim.tick, id.0),
    );
    let flavor = match &sim.chooser {
        crate::llm::Chooser::Custom(ch) => {
            let obs = crate::observation::build(sim, id);
            ch.invent_flavor(flavor_seed, kind.slug(), &obs)
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| mock_flavor.clone())
        }
        _ => mock_flavor,
    };
    let iid = sim.next_invention_id;
    sim.next_invention_id = sim.next_invention_id.saturating_add(1);
    let inv = crate::inventions::Invention {
        id: iid,
        inventor: id,
        tick: sim.tick,
        kind,
        shared: false,
        flavor,
    };
    sim.inventions.insert(iid, inv);
    let gain = crate::inventions::inventor_influence(intel);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.influence_factor = a.influence_factor.saturating_add(gain).min(10_000);
    }
    remember_agent(
        sim,
        id,
        crate::memory::MemoryEntry {
            tick: sim.tick,
            kind: crate::memory::MemoryKind::Reflection,
            text: kind.memory_text(),
            importance: 200,
            last_accessed: sim.tick,
            species_tag: 0,
            id: 0,
            participants: Vec::new(),
            valence: 0,
            ..Default::default()
        },
    );
    push(sim, id, SimEventKind::Invented { inventor: id, kind });
}

fn flee_away_step(sim: &Simulation, ax: u32, ay: u32, ox: u32, oy: u32) -> Option<(i32, i32)> {
    let here = crate::observation::chebyshev(ax, ay, ox, oy);
    let mut best: Option<(i32, i32, u32)> = None;
    for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = ax as i32 + dx;
        let ny = ay as i32 + dy;
        if !sim.world.in_bounds(nx, ny) || !sim.world.is_land(nx as u32, ny as u32) {
            continue;
        }
        let nd = crate::observation::chebyshev(nx as u32, ny as u32, ox, oy);
        if nd <= here {
            continue;
        }
        if best.is_none_or(|(_, _, d)| nd > d) {
            best = Some((dx, dy, nd));
        }
    }
    best.map(|(dx, dy, _)| (dx, dy))
}

fn flee(sim: &mut Simulation, id: AgentId) {
    let Some(agent) = sim.agents.get(&id) else {
        return;
    };
    let ax = agent.x;
    let ay = agent.y;
    let steps = agent.sheet.flee_steps();
    let mut nearest: Option<(u32, u32, u32)> = None;
    for other in sim.agents.values() {
        if other.id == id {
            continue;
        }
        let dist = crate::observation::chebyshev(ax, ay, other.x, other.y);
        match nearest {
            None => nearest = Some((dist, other.x, other.y)),
            Some((d, _, _)) if dist < d => nearest = Some((dist, other.x, other.y)),
            _ => {}
        }
    }
    let Some((_, ox, oy)) = nearest else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let cost = crate::inventions::apply_move_cost(
        agent.move_cost_milli(&sim.storage),
        &sim.inventions,
        id,
    );
    let Some((dx, dy)) = flee_away_step(sim, ax, ay, ox, oy) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    if agent.needs.energy < cost {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let mut x = (ax as i32 + dx) as u32;
    let mut y = (ay as i32 + dy) as u32;
    for _ in 1..steps {
        let Some((sx, sy)) = flee_away_step(sim, x, y, ox, oy) else {
            break;
        };
        x = (x as i32 + sx) as u32;
        y = (y as i32 + sy) as u32;
    }
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = x;
        a.y = y;
    }
    push(sim, id, SimEventKind::Flee);
}

fn transfer(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32, to: AgentId) {
    let Some(sender) = sim.agents.get(&id) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let Some(recv) = sim.agents.get(&to) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let moved = qty.min(recv.pocket_fit_qty(item));
    let have_pockets = sender.inventory.get(&item).copied().unwrap_or(0);
    let have_pack = sender.pack.get(&item).copied().unwrap_or(0);
    if moved == 0 || have_pockets + have_pack < moved {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if Agent::is_pack_carrier(item)
        && !crate::observation::can_drop_worn_carrier(sim, sender, item, moved, None)
    {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let haul = source_haul(sim, id, item);
    let cost = crate::haul::haul_cost_milli(item, moved, haul);
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if sim
        .agents
        .get_mut(&id)
        .and_then(|a| a.take_from_pack_or_pockets(item, moved))
        .is_none()
    {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if Agent::is_pack_carrier(item) && !unload_pack_after_last_basket(sim, id) {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.try_add_item(item, moved);
        }
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let added = sim
        .agents
        .get_mut(&to)
        .map(|a| a.try_add_item(item, moved))
        .unwrap_or(0);
    if added < moved {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.try_add_item(item, moved - added);
        }
    }
    if added == 0 {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    push(
        sim,
        id,
        SimEventKind::Transfer {
            item,
            qty: added,
            to,
        },
    );
}

fn store(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32) {
    let Some(agent) = sim.agents.get(&id) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let Some((x, y)) = crate_cell(sim, agent) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let have_pockets = agent.inventory.get(&item).copied().unwrap_or(0);
    let have_pack = agent.pack.get(&item).copied().unwrap_or(0);
    if have_pockets + have_pack < qty {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if Agent::is_pack_carrier(item)
        && !crate::observation::can_drop_worn_carrier(sim, agent, item, qty, Some((item, qty)))
    {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let params = sim.storage;
    if !sim.world.try_store(x, y, item, qty, &params) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let haul = source_haul(sim, id, item);
    let cost = crate::haul::haul_cost_milli(item, qty, haul);
    if !pay_energy(sim, id, cost) {
        sim.world.try_retrieve(x, y, item, qty);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if sim
        .agents
        .get_mut(&id)
        .and_then(|a| a.take_from_pack_or_pockets(item, qty))
        .is_none()
    {
        sim.world.try_retrieve(x, y, item, qty);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if Agent::is_pack_carrier(item) && !unload_pack_after_last_basket(sim, id) {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.try_add_item(item, qty);
        }
        sim.world.try_retrieve(x, y, item, qty);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    push(sim, id, SimEventKind::Store { item, qty });
}

fn retrieve(sim: &mut Simulation, id: AgentId, item: ItemId, qty: u32) {
    let Some(agent) = sim.agents.get(&id) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    let Some((x, y)) = crate_cell(sim, agent) else {
        push(sim, id, SimEventKind::Wait);
        return;
    };
    if !sim.world.try_retrieve(x, y, item, qty) {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if !pay_haul(sim, id, item, qty) {
        sim.world.try_store(x, y, item, qty, &sim.storage);
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let added = sim
        .agents
        .get_mut(&id)
        .map(|a| a.try_add_item(item, qty))
        .unwrap_or(0);
    if added < qty {
        sim.world.try_store(x, y, item, qty - added, &sim.storage);
    }
    if added == 0 {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    push(sim, id, SimEventKind::Retrieve { item, qty: added });
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
    let base = sim.config.energy_max_milli();
    if let Some(a) = sim.agents.get_mut(&id) {
        let max = a.sheet.energy_max(base);
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
    let cost = crate::inventions::apply_move_cost(
        agent.move_cost_milli(&sim.storage),
        &sim.inventions,
        id,
    );
    if agent.needs.energy < cost {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    if cost == 0 && agent.needs.energy == 0 {
        let fail = sim.rngs.agent_stream(id).random_bool(0.5);
        if fail {
            push(sim, id, SimEventKind::Wait);
            return;
        }
    }
    if !pay_energy(sim, id, cost) {
        push(sim, id, SimEventKind::Wait);
        return;
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
        let str_qty = sim
            .agents
            .get(&id)
            .map(|a| a.sheet.resource_qty(1 + u32::from(basket)))
            .unwrap_or(1);
        crate::incentive::scale_u32(
            str_qty,
            crate::incentive::resource_mult_milli(sim, id, "food"),
        )
        .max(1)
    };
    let params = sim.storage;
    if let Some(a) = sim.agents.get_mut(&id) {
        match spec.yield_kind {
            VegYield::Wood => {
                got_item = ItemId::Wood;
                let n = a.sheet.resource_qty(spec.wood_yield.max(1));
                qty = a.add_to_pockets_or_pack(ItemId::Wood, n, &params);
            }
            VegYield::Food => {
                got_item = ItemId::Food(species);
                qty = a.add_to_pockets_or_pack(ItemId::Food(species), food_n, &params);
                if spec.fiber_yield > 0 {
                    let _ = a.add_to_pockets_or_pack(ItemId::Fiber, spec.fiber_yield, &params);
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
    let params = sim.storage;
    if ok {
        if let Some(a) = sim.agents.get_mut(&id) {
            let n = a.sheet.resource_qty(1);
            qty = a.add_to_pockets_or_pack(ItemId::Stone, n, &params);
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
    let base_cap = sim.config.memory_capacity();
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
    let master = sim.config.master_seed;
    let con = sim
        .agents
        .get(&id)
        .map(|a| a.sheet.constitution)
        .unwrap_or(0);
    let apply_ill = crate::sheet::AbilitySheet::illness_hits(master, tick, id.0, con);

    let Some(agent) = sim.agents.get_mut(&id) else {
        return;
    };
    let cap = agent.sheet.memory_cap(base_cap);
    let illness = agent.sheet.illness_duration();
    if !agent.take_item(item, 1) && !agent.take_pack(item, 1) {
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
        agent.consumption.toxic_events += 1;
        if apply_ill {
            agent.illness_ticks = agent.illness_ticks.max(illness);
            agent.needs.energy = agent.needs.energy.saturating_sub(800);
            let _ = agent.remember(
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
                    ..Default::default()
                },
            );
        }
        let _ = agent.remember(
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
                ..Default::default()
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
        let params = sim.storage;
        if let Some(a) = sim.agents.get_mut(&id) {
            let n = a.sheet.resource_qty(1);
            let _ = a.add_to_pockets_or_pack(ItemId::Food(100), n, &params);
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
        let params = sim.storage;
        if let Some(a) = sim.agents.get_mut(&id) {
            let _ = a.add_to_pockets_or_pack(ItemId::Food(101), 1, &params);
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
    let Some((need, out, qty)) = crate::objects::recipe_spec(recipe, &sim.catalog) else {
        push(
            sim,
            id,
            SimEventKind::Craft {
                recipe,
                success: false,
            },
        );
        return;
    };
    let has_all = need
        .iter()
        .all(|(item, n)| agent.inventory.get(item).copied().unwrap_or(0) >= *n);
    let room = agent.pocket_fit_qty(out) >= qty || agent.inventory.contains_key(&out);
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
    let success = a.try_add_item(out, qty) > 0;
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
    if rule.as_ref().is_some_and(|r| r.is_meta()) && !sim.config.proposals.allow_meta_rules {
        push(sim, id, SimEventKind::Wait);
        return;
    }
    let rule = match rule {
        Some(crate::board::StructuredRule::SetCouncil { ids }) => {
            let ids = crate::board::dedupe_ids(ids);
            if ids.is_empty() {
                push(sim, id, SimEventKind::Wait);
                return;
            }
            Some(crate::board::StructuredRule::SetCouncil { ids })
        }
        other => other,
    };
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
            ..Default::default()
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
            ..Default::default()
        },
    );
    if let Some(a) = sim.agents.get(&id) {
        mid = a.memory.last().map(|e| e.id).unwrap_or(0);
    }
    if sim.config.agents.social.track_relationships {
        if let Some(author) = author {
            if author != id {
                let (fwd, back) = if support {
                    let social = sim
                        .agents
                        .get(&id)
                        .map(|a| a.sheet.support_social())
                        .unwrap_or(crate::social::SUPPORT);
                    (social, crate::social::SUPPORT_BACK)
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
                        ..Default::default()
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
            ..Default::default()
        },
    );
}

pub(crate) fn remember_agent(sim: &mut Simulation, id: AgentId, mut entry: MemoryEntry) {
    let cap = sim
        .agents
        .get(&id)
        .map(|a| a.sheet.memory_cap(sim.config.memory_capacity()))
        .unwrap_or_else(|| sim.config.memory_capacity());
    let policy = sim.config.agents.memory.eviction_policy;
    let bonus = sim.config.social_bonus_milli();
    let persist = sim.config.agents.memory.persistent_relationships;
    let milli = crate::incentive::memory_boost_milli(sim, id, entry.kind);
    entry.importance =
        crate::incentive::scale_u32(u32::from(entry.importance), milli).min(255) as u8;
    if sim.config.agents.memory.enable_embeddings {
        entry.ensure_embedding();
    }
    let dropped = if let Some(a) = sim.agents.get_mut(&id) {
        a.remember(cap, policy, bonus, persist, entry)
    } else {
        return;
    };
    sim.reflect_on_evict(id, dropped);
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
        let importance = {
            let mut sheet = crate::sheet::AbilitySheet::default();
            if let Some(sid) = h.speaker {
                if let Some(sp) = sim.agents.get(&sid) {
                    sheet = sp.sheet;
                }
            }
            sheet.speech_importance(50) as u8
        };
        remember_agent(
            sim,
            id,
            MemoryEntry {
                tick,
                kind: MemoryKind::Utterance,
                text: h.text.clone(),
                importance,
                last_accessed: tick,
                species_tag: 0,
                id: 0,
                participants: parts.clone(),
                valence: 0,
                ..Default::default()
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
                        ..Default::default()
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
