//! Overlay `[inventions]`. Not on `ExperimentConfig` (not hashed).

use crate::agent::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GATHER_BONUS_MILLI: u32 = 1200;
pub const MOVE_BONUS_MILLI: u32 = 800;
pub const SENSE_BONUS_CELLS: u32 = 1;
pub const CRAFT_BONUS: i32 = 15;
pub const REST_BONUS_MILLI: u32 = 1200;
pub const INVENTOR_INFLUENCE: u32 = 200;
pub const INVENT_BASE_CHANCE: i32 = 400;
pub const INVENT_INT_CHANCE: i32 = 50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InventionKind {
    #[default]
    GatherBonus = 0,
    MoveBonus = 1,
    SenseBonus = 2,
    CraftBonus = 3,
    RestBonus = 4,
}

impl InventionKind {
    pub const ALL: [InventionKind; 5] = [
        InventionKind::GatherBonus,
        InventionKind::MoveBonus,
        InventionKind::SenseBonus,
        InventionKind::CraftBonus,
        InventionKind::RestBonus,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            InventionKind::GatherBonus => "gather_bonus",
            InventionKind::MoveBonus => "move_bonus",
            InventionKind::SenseBonus => "sense_bonus",
            InventionKind::CraftBonus => "craft_bonus",
            InventionKind::RestBonus => "rest_bonus",
        }
    }

    pub fn memory_text(self) -> String {
        format!("invented {}", self.slug().replace('_', " "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invention {
    pub id: u64,
    pub inventor: AgentId,
    pub tick: u64,
    pub kind: InventionKind,
    pub shared: bool,
    #[serde(default)]
    pub flavor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventionsParams {
    pub enabled: bool,
    pub share_delay_ticks: u64,
    pub tree: bool,
    pub patent_ticks: u64,
}

impl Default for InventionsParams {
    fn default() -> Self {
        Self {
            enabled: false,
            share_delay_ticks: 8,
            tree: false,
            patent_ticks: 0,
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
            tree: Option<bool>,
            patent_ticks: Option<u64>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.inventions.enabled.unwrap_or(false),
            share_delay_ticks: slice.inventions.share_delay_ticks.unwrap_or(8),
            tree: slice.inventions.tree.unwrap_or(false),
            patent_ticks: slice.inventions.patent_ticks.unwrap_or(0),
        }
    }
}

pub fn invent_chance(intelligence: u8) -> i32 {
    (INVENT_BASE_CHANCE + crate::sheet::AbilitySheet::modifier(intelligence) * INVENT_INT_CHANCE)
        .clamp(1, 1000)
}

/// Influence gain on a successful Invent. INT 0 ⇒ today's +200.
pub fn inventor_influence(intelligence: u8) -> u32 {
    if intelligence == 0 {
        INVENTOR_INFLUENCE
    } else {
        (INVENTOR_INFLUENCE as i32
            + crate::sheet::AbilitySheet::modifier(intelligence) * INVENT_INT_CHANCE)
            .max(1) as u32
    }
}

pub fn next_kind(table: &BTreeMap<u64, Invention>, tree: bool) -> Option<InventionKind> {
    for (i, k) in InventionKind::ALL.into_iter().enumerate() {
        if table.values().any(|inv| inv.kind == k) {
            continue;
        }
        if tree && i > 0 {
            let prev = InventionKind::ALL[i - 1];
            if !table.values().any(|inv| inv.kind == prev && inv.shared) {
                return None;
            }
        }
        return Some(k);
    }
    None
}

pub fn entitled(table: &BTreeMap<u64, Invention>, id: AgentId, kind: InventionKind) -> bool {
    table
        .values()
        .any(|i| i.kind == kind && (i.shared || i.inventor == id))
}

pub fn food_mult_milli(table: &BTreeMap<u64, Invention>, id: AgentId) -> u32 {
    if entitled(table, id, InventionKind::GatherBonus) {
        GATHER_BONUS_MILLI
    } else {
        1000
    }
}

/// Move energy after MoveBonus. Zero cost stays 0. No stack.
pub fn apply_move_cost(cost: u32, table: &BTreeMap<u64, Invention>, id: AgentId) -> u32 {
    if cost == 0 {
        return 0;
    }
    if !entitled(table, id, InventionKind::MoveBonus) {
        return cost;
    }
    (cost * MOVE_BONUS_MILLI / 1000).max(1)
}

/// Vision/hear/ident cells after SenseBonus. No stack.
pub fn apply_sense_range(range: u32, table: &BTreeMap<u64, Invention>, id: AgentId) -> u32 {
    if entitled(table, id, InventionKind::SenseBonus) {
        range.saturating_add(SENSE_BONUS_CELLS)
    } else {
        range
    }
}

/// Craft skill_roll bonus after CraftBonus. No stack.
pub fn craft_skill_bonus(table: &BTreeMap<u64, Invention>, id: AgentId) -> i32 {
    if entitled(table, id, InventionKind::CraftBonus) {
        CRAFT_BONUS
    } else {
        0
    }
}

/// Rest energy regen after RestBonus. Zero stays 0. No stack.
pub fn apply_rest_regen(regen: u32, table: &BTreeMap<u64, Invention>, id: AgentId) -> u32 {
    if regen == 0 {
        return 0;
    }
    if !entitled(table, id, InventionKind::RestBonus) {
        return regen;
    }
    (regen * REST_BONUS_MILLI / 1000).max(1)
}

pub fn observation_lines(table: &BTreeMap<u64, Invention>, id: AgentId) -> Vec<String> {
    let mut out = Vec::new();
    for inv in table.values() {
        if inv.inventor == id {
            out.push(format!("invention {} (yours)", inv.kind.slug()));
        } else if inv.shared {
            out.push(format!("invention {}", inv.kind.slug()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventor_influence_unused_and_scores() {
        assert_eq!(invent_chance(0), 400);
        assert_eq!(invent_chance(18), 600);
        assert_eq!(invent_chance(3), 250);
        assert_eq!(inventor_influence(0), 200);
        assert_eq!(inventor_influence(18), 400);
        assert_eq!(inventor_influence(3), 50);
    }
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
        hasher.update(inv.flavor.as_bytes());
    }
}
