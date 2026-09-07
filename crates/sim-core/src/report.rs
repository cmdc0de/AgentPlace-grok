//! Derived world + per-agent food-economy / governance report.

use crate::agent::{AgentId, ItemId};
use crate::board::StructuredRule;
use crate::error::SimError;
use crate::memory::MemoryKind;
use crate::observation;
use crate::simulation::Simulation;
use crate::species::{SpeciesTables, Toxicity, VegYield};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct VegCount {
    pub tag: u8,
    pub id: String,
    pub cells: u32,
    pub toxicity: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnownFood {
    pub tag: u8,
    pub id: String,
    pub locations: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldReport {
    pub consumed_vegetation: u32,
    pub consumed_animal: u32,
    pub consumed_fish: u32,
    pub toxic_events: u32,
    pub hunger_mean: f64,
    pub hunger_min: f64,
    pub hunger_max: f64,
    pub thirst_mean: f64,
    pub energy_mean: f64,
    pub hungry_below_half: u32,
    pub veg: Vec<VegCount>,
    pub animals: u32,
    pub fish: u32,
    pub crops: u32,
    pub known: Vec<KnownFood>,
    pub open_proposals: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub expired: u32,
    pub adopted: Vec<String>,
    pub mean_trust: f64,
    pub relationship_pairs: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentReport {
    pub id: u64,
    pub consumed_vegetation: u32,
    pub consumed_animal: u32,
    pub consumed_fish: u32,
    pub toxic_events: u32,
    pub hunger_milli: u32,
    pub thirst_milli: u32,
    pub energy_milli: u32,
    pub inventory: Vec<(String, u32)>,
    pub known: Vec<KnownFood>,
    pub goals: Vec<String>,
    pub authored: Vec<u64>,
    pub supports: Vec<u64>,
    pub opposes: Vec<u64>,
    pub rel_count: u32,
    pub mean_trust: f64,
    pub top_trust: Vec<(u64, i16)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SummaryReport {
    pub experiment_id: String,
    pub tick: u64,
    pub world: WorldReport,
    pub agents: Vec<AgentReport>,
}

pub fn build_report(sim: &Simulation) -> Result<SummaryReport, SimError> {
    let species = &sim.config.world.species;
    let n = sim.agents.len().max(1) as f64;
    let hungers: Vec<f64> = sim
        .agents
        .values()
        .map(|a| a.needs.hunger as f64 / 100.0)
        .collect();
    let thirsts: Vec<f64> = sim
        .agents
        .values()
        .map(|a| a.needs.thirst as f64 / 100.0)
        .collect();
    let energies: Vec<f64> = sim
        .agents
        .values()
        .map(|a| a.needs.energy as f64 / 100.0)
        .collect();
    let hunger_max = sim.config.needs.hunger_max;
    let hungry_below_half = sim
        .agents
        .values()
        .filter(|a| (a.needs.hunger as f64 / 100.0) < hunger_max / 2.0)
        .count() as u32;

    let mut veg_map: BTreeMap<u8, u32> = BTreeMap::new();
    for &tag in &sim.world.vegetation {
        if tag == 0 {
            continue;
        }
        if species
            .veg(tag)
            .is_some_and(|s| s.yield_kind == VegYield::Food)
        {
            *veg_map.entry(tag).or_insert(0) += 1;
        }
    }
    let veg: Vec<VegCount> = veg_map
        .into_iter()
        .map(|(tag, cells)| {
            let spec = species.veg(tag);
            VegCount {
                tag,
                id: spec
                    .map(|s| s.id.clone())
                    .unwrap_or_else(|| tag.to_string()),
                cells,
                toxicity: spec.map(tox_label).unwrap_or("unknown"),
            }
        })
        .collect();

    let mut world_known: BTreeMap<u8, BTreeSet<(u32, u32)>> = BTreeMap::new();
    let mut agents = Vec::new();
    for agent in sim.agents.values() {
        let known_locs = agent_known(sim, agent.id, species);
        for (tag, locs) in &known_locs {
            world_known
                .entry(*tag)
                .or_default()
                .extend(locs.iter().copied());
        }
        let known_map: BTreeMap<u8, u32> = known_locs
            .iter()
            .map(|(tag, set)| (*tag, set.len() as u32))
            .collect();
        let inventory = agent
            .inventory
            .iter()
            .map(|(item, qty)| (item_name(*item, species), *qty))
            .collect();
        let authored: Vec<u64> = sim
            .board
            .proposals
            .iter()
            .filter(|p| p.author == agent.id)
            .map(|p| p.id)
            .collect();
        let supports: Vec<u64> = sim
            .board
            .proposals
            .iter()
            .filter(|p| p.supporters.contains(&agent.id))
            .map(|p| p.id)
            .collect();
        let opposes: Vec<u64> = sim
            .board
            .proposals
            .iter()
            .filter(|p| p.opposers.contains(&agent.id))
            .map(|p| p.id)
            .collect();
        let rel_count = agent.relationships.len() as u32;
        let mean_trust = if agent.relationships.is_empty() {
            0.0
        } else {
            agent
                .relationships
                .values()
                .map(|r| r.trust as f64 / 100.0)
                .sum::<f64>()
                / f64::from(rel_count)
        };
        let mut top_trust: Vec<(u64, i16)> = agent
            .relationships
            .iter()
            .map(|(oid, r)| (oid.0, r.trust))
            .collect();
        top_trust.sort_by_key(|(_, t)| -i32::from(t.abs()));
        top_trust.truncate(3);
        agents.push(AgentReport {
            id: agent.id.0,
            consumed_vegetation: agent.consumption.vegetation,
            consumed_animal: agent.consumption.animal,
            consumed_fish: agent.consumption.fish,
            toxic_events: agent.consumption.toxic_events,
            hunger_milli: agent.needs.hunger,
            thirst_milli: agent.needs.thirst,
            energy_milli: agent.needs.energy,
            inventory,
            known: known_list(&known_map, species),
            goals: agent.goals.iter().map(|g| g.text.clone()).collect(),
            authored,
            supports,
            opposes,
            rel_count,
            mean_trust,
            top_trust,
        });
    }

    let world = WorldReport {
        consumed_vegetation: agents.iter().map(|a| a.consumed_vegetation).sum(),
        consumed_animal: agents.iter().map(|a| a.consumed_animal).sum(),
        consumed_fish: agents.iter().map(|a| a.consumed_fish).sum(),
        toxic_events: agents.iter().map(|a| a.toxic_events).sum(),
        hunger_mean: hungers.iter().sum::<f64>() / n,
        hunger_min: hungers.iter().copied().fold(f64::INFINITY, f64::min),
        hunger_max: hungers.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        thirst_mean: thirsts.iter().sum::<f64>() / n,
        energy_mean: energies.iter().sum::<f64>() / n,
        hungry_below_half,
        veg,
        animals: sim.world.animal_total(),
        fish: sim.world.fish_total(),
        crops: sim.world.crops.len() as u32,
        known: {
            let counts: BTreeMap<u8, u32> = world_known
                .iter()
                .map(|(tag, set)| (*tag, set.len() as u32))
                .collect();
            known_list(&counts, species)
        },
        open_proposals: sim.board.open().count() as u32,
        accepted: sim.board.accepted_count,
        rejected: sim.board.rejected_count,
        expired: sim.board.expired_count,
        mean_trust: if agents.is_empty() {
            0.0
        } else {
            agents.iter().map(|a| a.mean_trust).sum::<f64>() / agents.len() as f64
        },
        relationship_pairs: sim
            .agents
            .values()
            .map(|a| a.relationships.len() as u32)
            .sum(),
        adopted: sim
            .board
            .adopted
            .iter()
            .map(|r| {
                let kind = r
                    .rule
                    .as_ref()
                    .map(|rule| match rule {
                        StructuredRule::BanEatSpecies { species: tag } => {
                            format!("BanEatSpecies({tag})")
                        }
                        StructuredRule::BanGatherSpecies { species: tag } => {
                            format!("BanGatherSpecies({tag})")
                        }
                        StructuredRule::MaxGatherPerTick { n } => {
                            format!("MaxGatherPerTick({n})")
                        }
                        StructuredRule::SetProposalLifetime { ticks } => {
                            format!("SetProposalLifetime({ticks})")
                        }
                        StructuredRule::SetAcceptanceThreshold { milli } => {
                            format!("SetAcceptanceThreshold({milli})")
                        }
                        StructuredRule::SetVoteWeight { weight } => {
                            format!("SetVoteWeight({})", weight.as_str())
                        }
                        StructuredRule::SetVoteAccept { accept } => {
                            format!("SetVoteAccept({})", accept.as_str())
                        }
                        StructuredRule::SetCouncil { ids } => {
                            let list = ids
                                .iter()
                                .map(|id| id.0.to_string())
                                .collect::<Vec<_>>()
                                .join(",");
                            format!("SetCouncil([{list}])")
                        }
                        StructuredRule::SetCouncilTally { tally } => {
                            format!("SetCouncilTally({})", tally.as_str())
                        }
                    })
                    .unwrap_or_else(|| "text".into());
                format!("#{} {kind}: {}", r.proposal_id, r.text)
            })
            .collect(),
    };

    Ok(SummaryReport {
        experiment_id: crate::experiment_id(&sim.config_hash()?),
        tick: sim.tick,
        world,
        agents,
    })
}

fn tox_label(spec: &crate::species::VegetationSpecies) -> &'static str {
    match spec.toxicity {
        Toxicity::Safe => "safe",
        Toxicity::Toxic => "toxic",
        Toxicity::Allergenic => "allergenic",
    }
}

fn item_name(item: ItemId, species: &SpeciesTables) -> String {
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

fn food_name(tag: u8, species: &SpeciesTables) -> String {
    match tag {
        100 => "hare".into(),
        101 => "perch".into(),
        other => species
            .veg(other)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| other.to_string()),
    }
}

fn is_food_tag(tag: u8, species: &SpeciesTables) -> bool {
    tag == 100
        || tag == 101
        || species
            .veg(tag)
            .is_some_and(|s| s.yield_kind == VegYield::Food)
}

fn agent_known(
    sim: &Simulation,
    id: AgentId,
    species: &SpeciesTables,
) -> BTreeMap<u8, BTreeSet<(u32, u32)>> {
    let mut loc: BTreeMap<u8, BTreeSet<(u32, u32)>> = BTreeMap::new();
    if let Some(agent) = sim.agents.get(&id) {
        for mem in &agent.memory {
            if mem.kind != MemoryKind::Observation || !is_food_tag(mem.species_tag, species) {
                continue;
            }
            if let Some((x, y)) = parse_xy(&mem.text) {
                loc.entry(mem.species_tag).or_default().insert((x, y));
            } else {
                loc.entry(mem.species_tag)
                    .or_default()
                    .insert((u32::MAX, u32::MAX));
            }
        }
    }
    let obs = observation::build(sim, id);
    for t in &obs.tiles {
        if t.vegetation != 0 && is_food_tag(t.vegetation, species) {
            loc.entry(t.vegetation).or_default().insert((t.x, t.y));
        }
        if t.animals > 0 {
            loc.entry(100).or_default().insert((t.x, t.y));
        }
        if t.fish > 0 {
            loc.entry(101).or_default().insert((t.x, t.y));
        }
        if t.crop {
            if let Some(crop) = sim.world.crops.get(&(t.x, t.y)) {
                if is_food_tag(crop.species_tag, species) {
                    loc.entry(crop.species_tag).or_default().insert((t.x, t.y));
                }
            }
        }
    }
    loc
}

fn parse_xy(text: &str) -> Option<(u32, u32)> {
    let start = text.find('(')?;
    let end = text.find(')')?;
    let inner = &text[start + 1..end];
    let mut parts = inner.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    Some((x, y))
}

fn known_list(map: &BTreeMap<u8, u32>, species: &SpeciesTables) -> Vec<KnownFood> {
    map.iter()
        .map(|(tag, n)| KnownFood {
            tag: *tag,
            id: food_name(*tag, species),
            locations: *n,
        })
        .collect()
}

pub fn report_markdown(report: &SummaryReport) -> String {
    let w = &report.world;
    let mut out = format!(
        "# Food-economy report\n\n\
         - experiment_id: `{}`\n\
         - tick: {}\n\n\
         ## World\n\n\
         - consumed: veg {} animal {} fish {} toxic_events {}\n\
         - hunger: mean {:.1} min {:.1} max {:.1} (below half: {})\n\
         - thirst mean: {:.1}\n\
         - energy mean: {:.1}\n\
         - available animals: {}\n\
         - available fish: {}\n\
         - unharvested crops: {}\n\
         - board: open {} accepted {} rejected {} expired {}\n\
         - relationships: pairs {} mean trust {:.1}\n",
        report.experiment_id,
        report.tick,
        w.consumed_vegetation,
        w.consumed_animal,
        w.consumed_fish,
        w.toxic_events,
        w.hunger_mean,
        w.hunger_min,
        w.hunger_max,
        w.hungry_below_half,
        w.thirst_mean,
        w.energy_mean,
        w.animals,
        w.fish,
        w.crops,
        w.open_proposals,
        w.accepted,
        w.rejected,
        w.expired,
        w.relationship_pairs,
        w.mean_trust,
    );
    out.push_str("\n### Edible vegetation (objective)\n\n");
    if w.veg.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for v in &w.veg {
            out.push_str(&format!(
                "- {} (tag {}): {} cells ({})\n",
                v.id, v.tag, v.cells, v.toxicity
            ));
        }
    }
    out.push_str("\n### Known available (union of agent memory + current sight)\n\n");
    if w.known.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for k in &w.known {
            out.push_str(&format!(
                "- {} (tag {}): {} remembered/seen locations\n",
                k.id, k.tag, k.locations
            ));
        }
    }
    out.push_str("\n### Adopted rules\n\n");
    if w.adopted.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for a in &w.adopted {
            out.push_str(&format!("- {a}\n"));
        }
    }
    out.push_str("\n## Agents\n");
    for a in &report.agents {
        out.push_str(&format!(
            "\n### Agent {}\n\n\
             - consumed: veg {} animal {} fish {} toxic {}\n\
             - needs: hunger {:.1} ({}) thirst {:.1} ({}) energy {:.1} ({})\n\
             - inventory: {}\n\
             - known available: {}\n\
             - goals: {}\n\
             - authored: {}\n\
             - supports: {}\n\
             - opposes: {}\n\
             - relationships: {} mean trust {:.1} top: {}\n",
            a.id,
            a.consumed_vegetation,
            a.consumed_animal,
            a.consumed_fish,
            a.toxic_events,
            a.hunger_milli as f64 / 100.0,
            a.hunger_milli,
            a.thirst_milli as f64 / 100.0,
            a.thirst_milli,
            a.energy_milli as f64 / 100.0,
            a.energy_milli,
            if a.inventory.is_empty() {
                "(empty)".into()
            } else {
                a.inventory
                    .iter()
                    .map(|(n, q)| format!("{n}×{q}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            if a.known.is_empty() {
                "(none)".into()
            } else {
                a.known
                    .iter()
                    .map(|k| format!("{} ({})", k.id, k.locations))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            if a.goals.is_empty() {
                "(none)".into()
            } else {
                a.goals.join("; ")
            },
            fmt_ids(&a.authored),
            fmt_ids(&a.supports),
            fmt_ids(&a.opposes),
            a.rel_count,
            a.mean_trust,
            if a.top_trust.is_empty() {
                "(none)".into()
            } else {
                a.top_trust
                    .iter()
                    .map(|(id, t)| format!("#{id} {:.1}", *t as f64 / 100.0))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ));
    }
    out
}

fn fmt_ids(ids: &[u64]) -> String {
    if ids.is_empty() {
        "(none)".into()
    } else {
        ids.iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn report_csv(report: &SummaryReport) -> String {
    let mut out = String::from(
        "grain,agent_id,veg_consumed,animal_consumed,fish_consumed,toxic_events,hunger,thirst,energy,known_count,open_proposals\n",
    );
    out.push_str(&format!(
        "world,,{},{},{},{},{:.2},{:.2},{:.2},{},{}\n",
        report.world.consumed_vegetation,
        report.world.consumed_animal,
        report.world.consumed_fish,
        report.world.toxic_events,
        report.world.hunger_mean,
        report.world.thirst_mean,
        report.world.energy_mean,
        report.world.known.len(),
        report.world.open_proposals,
    ));
    for a in &report.agents {
        out.push_str(&format!(
            "agent,{},{},{},{},{},{:.2},{:.2},{:.2},{},\n",
            a.id,
            a.consumed_vegetation,
            a.consumed_animal,
            a.consumed_fish,
            a.toxic_events,
            a.hunger_milli as f64 / 100.0,
            a.thirst_milli as f64 / 100.0,
            a.energy_milli as f64 / 100.0,
            a.known.len(),
        ));
    }
    out
}

pub fn write_report(
    sim: &Simulation,
    dir: impl AsRef<Path>,
) -> Result<(PathBuf, Option<PathBuf>), SimError> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let report = build_report(sim)?;
    let stem = format!("{}_tick_{}_report", report.experiment_id, report.tick);
    let md_path = dir.join(format!("{stem}.md"));
    fs::write(&md_path, report_markdown(&report))?;
    let csv_path = if sim.config.metrics.export_csv {
        let p = dir.join(format!("{stem}.csv"));
        fs::write(&p, report_csv(&report))?;
        Some(p)
    } else {
        None
    };
    Ok((md_path, csv_path))
}
