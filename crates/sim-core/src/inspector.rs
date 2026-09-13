//! Display-only researcher snapshot. Not hashed, not ExperimentConfig.

use crate::agent::{AgentId, ItemId};
use crate::event_log;
use crate::simulation::Simulation;
use crate::timing::TickTiming;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InspectorView {
    pub tick: u64,
    pub agents: Vec<InspectorAgent>,
    pub board: InspectorBoard,
    pub metrics: InspectorMetrics,
    #[serde(default)]
    pub inventions: Vec<InspectorInvention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tod: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticks_per_day: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorAgent {
    pub id: u64,
    pub x: u32,
    pub y: u32,
    pub hunger: u32,
    pub thirst: u32,
    pub energy: u32,
    pub health: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<BTreeMap<String, u8>>,
    pub inventory: BTreeMap<String, u32>,
    pub pack: BTreeMap<String, u32>,
    pub goals: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_action: Option<String>,
    pub relationships: Vec<InspectorRel>,
    pub kinship: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorRel {
    pub id: u64,
    pub trust: i16,
    pub affinity: i16,
    pub respect: i16,
    pub fear: i16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InspectorBoard {
    pub open: Vec<InspectorProposal>,
    pub adopted: Vec<InspectorAdopted>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorProposal {
    pub id: u64,
    pub author: u64,
    pub text: String,
    pub supporters: usize,
    pub opposers: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorAdopted {
    pub proposal_id: u64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorInvention {
    pub id: u64,
    pub inventor: u64,
    pub kind: String,
    pub shared: bool,
    pub tick: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flavor: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InspectorMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TickTiming>,
    pub hungry: u32,
    pub thirsty: u32,
    pub tired: u32,
    pub mean_trust: i32,
    pub illness: u32,
}

impl InspectorView {
    pub fn from_sim(sim: &Simulation) -> Self {
        let hmax = sim.config.hunger_max_milli();
        let tmax = sim.config.thirst_max_milli();
        let emax = sim.config.energy_max_milli();
        let mut hungry = 0u32;
        let mut thirsty = 0u32;
        let mut tired = 0u32;
        let mut illness = 0u32;
        let mut trust_sum = 0i64;
        let mut trust_n = 0u32;
        let mut agents = Vec::new();
        for a in sim.agents.values() {
            if a.needs.hunger < hmax / 2 {
                hungry += 1;
            }
            if a.needs.thirst < tmax / 2 {
                thirsty += 1;
            }
            let agent_emax = a.sheet.energy_max(emax);
            if a.needs.energy < agent_emax / 2 {
                tired += 1;
            }
            if a.illness_ticks > 0 {
                illness += 1;
            }
            for rel in a.relationships.values() {
                trust_sum += i64::from(rel.trust);
                trust_n += 1;
            }
            let sheet = if a.sheet.is_unused() {
                None
            } else {
                Some(
                    a.sheet
                        .rows()
                        .into_iter()
                        .map(|(n, v)| (n.to_string(), v))
                        .collect(),
                )
            };
            agents.push(InspectorAgent {
                id: a.id.0,
                x: a.x,
                y: a.y,
                hunger: a.needs.hunger / 100,
                thirst: a.needs.thirst / 100,
                energy: a.needs.energy / 100,
                health: a.health,
                sheet,
                inventory: items_map(&a.inventory),
                pack: items_map(&a.pack),
                goals: a.goals.iter().map(|g| g.text.clone()).collect(),
                last_action: last_primary(sim, a.id),
                relationships: a
                    .relationships
                    .iter()
                    .map(|(oid, r)| InspectorRel {
                        id: oid.0,
                        trust: r.trust,
                        affinity: r.affinity,
                        respect: r.respect,
                        fear: r.fear,
                    })
                    .collect(),
                kinship: a.kinship.lines(),
            });
        }
        let mean_trust = if trust_n == 0 {
            0
        } else {
            (trust_sum / i64::from(trust_n)) as i32
        };
        let board = InspectorBoard {
            open: sim
                .board
                .open()
                .map(|p| InspectorProposal {
                    id: p.id,
                    author: p.author.0,
                    text: p.text.clone(),
                    supporters: p.supporters.len(),
                    opposers: p.opposers.len(),
                })
                .collect(),
            adopted: sim
                .board
                .adopted
                .iter()
                .map(|r| InspectorAdopted {
                    proposal_id: r.proposal_id,
                    text: r.text.clone(),
                })
                .collect(),
        };
        let inventions = sim
            .inventions
            .values()
            .map(|i| InspectorInvention {
                id: i.id,
                inventor: i.inventor.0,
                kind: i.kind.slug().to_string(),
                shared: i.shared,
                tick: i.tick,
                flavor: i.flavor.clone(),
            })
            .collect();
        let (day, tod, ticks_per_day) = if sim.time_enabled {
            let (d, t) = crate::clock::day_tod(sim.tick, sim.ticks_per_day);
            (Some(d), Some(t), Some(sim.ticks_per_day))
        } else {
            (None, None, None)
        };
        Self {
            tick: sim.tick,
            agents,
            board,
            metrics: InspectorMetrics {
                timing: sim.last_tick_timing.clone(),
                hungry,
                thirsty,
                tired,
                mean_trust,
                illness,
            },
            inventions,
            day,
            tod,
            ticks_per_day,
        }
    }

    pub fn from_checkpoint_bytes(bytes: &[u8]) -> Result<Self, crate::error::SimError> {
        let sim = Simulation::decode_checkpoint(bytes)?;
        Ok(Self::from_sim(&sim))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

fn last_primary(sim: &Simulation, id: AgentId) -> Option<String> {
    let cur = sim.tick;
    let prev = cur.saturating_sub(1);
    sim.events
        .events
        .iter()
        .rev()
        .find(|e| {
            e.agent == id
                && (e.tick == cur || e.tick == prev)
                && event_log::is_primary_kind(&e.kind)
        })
        .map(|e| event_log::kind_slug(&e.kind).to_string())
}

fn items_map(map: &BTreeMap<ItemId, u32>) -> BTreeMap<String, u32> {
    map.iter()
        .filter(|(_, q)| **q > 0)
        .map(|(item, q)| (item_key(*item), *q))
        .collect()
}

fn item_key(item: ItemId) -> String {
    match item {
        ItemId::Food(t) => format!("food:{t}"),
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

pub fn metrics_json_with_inspector(sim: &Simulation) -> Vec<u8> {
    let timing = sim.last_tick_timing.clone().unwrap_or_default();
    let mut value = serde_json::to_value(&timing).unwrap_or(serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        let ins =
            serde_json::to_value(InspectorView::from_sim(sim)).unwrap_or(serde_json::Value::Null);
        obj.insert("inspector".into(), ins);
    }
    serde_json::to_vec(&value).unwrap_or_default()
}
