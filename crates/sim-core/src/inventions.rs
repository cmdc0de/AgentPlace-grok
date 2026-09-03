//! Overlay `[inventions]`. Not on `ExperimentConfig` (not hashed).

use crate::agent::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GATHER_BONUS_MILLI: u32 = 1200;
pub const INVENTOR_INFLUENCE: u32 = 200;
pub const INVENT_BASE_CHANCE: i32 = 400;
pub const INVENT_INT_CHANCE: i32 = 50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InventionKind {
    #[default]
    GatherBonus = 0,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invention {
    pub id: u64,
    pub inventor: AgentId,
    pub tick: u64,
    pub kind: InventionKind,
    pub shared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventionsParams {
    pub enabled: bool,
    pub share_delay_ticks: u64,
}

impl Default for InventionsParams {
    fn default() -> Self {
        Self {
            enabled: false,
            share_delay_ticks: 8,
        }
    }
}

impl InventionsParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            inventions: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
            share_delay_ticks: Option<u64>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.inventions.enabled.unwrap_or(false),
            share_delay_ticks: slice.inventions.share_delay_ticks.unwrap_or(8),
        }
    }
}

pub fn invent_chance(intelligence: u8) -> i32 {
    (INVENT_BASE_CHANCE + crate::sheet::AbilitySheet::modifier(intelligence) * INVENT_INT_CHANCE)
        .clamp(1, 1000)
}

pub fn food_mult_milli(table: &BTreeMap<u64, Invention>, id: AgentId) -> u32 {
    let Some(inv) = table
        .values()
        .find(|i| matches!(i.kind, InventionKind::GatherBonus))
    else {
        return 1000;
    };
    if inv.shared || inv.inventor == id {
        GATHER_BONUS_MILLI
    } else {
        1000
    }
}

pub fn observation_lines(table: &BTreeMap<u64, Invention>, id: AgentId) -> Vec<String> {
    let mut out = Vec::new();
    for inv in table.values() {
        match inv.kind {
            InventionKind::GatherBonus => {
                if inv.inventor == id {
                    out.push("invention gather_bonus (yours)".into());
                } else if inv.shared {
                    out.push("invention gather_bonus".into());
                }
            }
        }
    }
    out
}

pub fn hash_table(table: &BTreeMap<u64, Invention>, hasher: &mut impl sha2::Digest) {
    if table.is_empty() {
        return;
    }
    for (id, inv) in table {
        hasher.update(id.to_le_bytes());
        hasher.update(inv.inventor.0.to_le_bytes());
        hasher.update(inv.tick.to_le_bytes());
        hasher.update([inv.kind as u8]);
        hasher.update([u8::from(inv.shared)]);
    }
}
