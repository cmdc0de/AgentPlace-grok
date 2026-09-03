use crate::action::Recipe;
use crate::agent::{AgentId, ItemId};
use crate::error::SimError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimEventKind {
    Wait,
    Rest,
    Move {
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
    },
    Gather {
        species: u8,
        item: ItemId,
        qty: u32,
    },
    Drink,
    Eat {
        item: ItemId,
        toxic: bool,
    },
    Hunt {
        success: bool,
    },
    Fish {
        success: bool,
    },
    Farm {
        species: u8,
        x: u32,
        y: u32,
    },
    Craft {
        recipe: Recipe,
        success: bool,
    },
    Speak {
        shout: bool,
        text: String,
        broadcast: bool,
        #[serde(default)]
        targets: Vec<AgentId>,
    },
    LlmWait,
    Propose {
        proposal_id: u64,
    },
    Support {
        proposal_id: u64,
    },
    Oppose {
        proposal_id: u64,
    },
    RuleBlocked {
        reason: String,
    },
    IncentiveApplied {
        id: String,
        #[serde(default)]
        detail: String,
    },
    IncentiveEnded {
        id: String,
    },
    Died {
        hunger_zero: bool,
        thirst_zero: bool,
    },
    Transfer {
        item: ItemId,
        qty: u32,
        to: AgentId,
    },
    Store {
        item: ItemId,
        qty: u32,
    },
    Retrieve {
        item: ItemId,
        qty: u32,
    },
    Give {
        item: ItemId,
        qty: u32,
    },
    Pack {
        item: ItemId,
        qty: u32,
    },
    Unpack {
        item: ItemId,
        qty: u32,
    },
    Attack {
        target: AgentId,
        damage: u32,
    },
    Flee,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimEvent {
    pub tick: u64,
    pub agent: AgentId,
    pub kind: SimEventKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    pub events: Vec<SimEvent>,
}

impl EventLog {
    pub fn push(&mut self, event: SimEvent) {
        self.events.push(event);
    }
}

pub fn hash_kind(kind: &SimEventKind, hasher: &mut impl sha2::Digest) {
    match kind {
        SimEventKind::Wait => hasher.update([0u8]),
        SimEventKind::Rest => hasher.update([1u8]),
        SimEventKind::Move {
            from_x,
            from_y,
            to_x,
            to_y,
        } => {
            hasher.update([2u8]);
            hasher.update(from_x.to_le_bytes());
            hasher.update(from_y.to_le_bytes());
            hasher.update(to_x.to_le_bytes());
            hasher.update(to_y.to_le_bytes());
        }
        SimEventKind::Gather {
            species,
            item: _,
            qty,
        } => {
            hasher.update([3u8]);
            hasher.update([*species]);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Drink => hasher.update([4u8]),
        SimEventKind::Eat { item: _, toxic } => {
            hasher.update([5u8]);
            hasher.update([u8::from(*toxic)]);
        }
        SimEventKind::Hunt { success } => {
            hasher.update([6u8]);
            hasher.update([u8::from(*success)]);
        }
        SimEventKind::Fish { success } => {
            hasher.update([7u8]);
            hasher.update([u8::from(*success)]);
        }
        SimEventKind::Farm { species, x, y } => {
            hasher.update([8u8]);
            hasher.update([*species]);
            hasher.update(x.to_le_bytes());
            hasher.update(y.to_le_bytes());
        }
        SimEventKind::Craft { recipe, success } => {
            hasher.update([9u8]);
            hasher.update([*recipe as u8]);
            hasher.update([u8::from(*success)]);
        }
        SimEventKind::Speak {
            shout,
            text,
            broadcast,
            targets,
        } => {
            hasher.update([10u8]);
            hasher.update([u8::from(*shout), u8::from(*broadcast)]);
            hasher.update(text.as_bytes());
            for id in targets {
                hasher.update(id.0.to_le_bytes());
            }
        }
        SimEventKind::LlmWait => hasher.update([11u8]),
        SimEventKind::Propose { proposal_id } => {
            hasher.update([12u8]);
            hasher.update(proposal_id.to_le_bytes());
        }
        SimEventKind::Support { proposal_id } => {
            hasher.update([13u8]);
            hasher.update(proposal_id.to_le_bytes());
        }
        SimEventKind::Oppose { proposal_id } => {
            hasher.update([14u8]);
            hasher.update(proposal_id.to_le_bytes());
        }
        SimEventKind::RuleBlocked { reason } => {
            hasher.update([15u8]);
            hasher.update(reason.as_bytes());
        }
        SimEventKind::IncentiveApplied { id, detail } => {
            hasher.update([16u8]);
            hasher.update(id.as_bytes());
            hasher.update(detail.as_bytes());
        }
        SimEventKind::IncentiveEnded { id } => {
            hasher.update([17u8]);
            hasher.update(id.as_bytes());
        }
        SimEventKind::Died {
            hunger_zero,
            thirst_zero,
        } => {
            hasher.update([18u8]);
            hasher.update([u8::from(*hunger_zero), u8::from(*thirst_zero)]);
        }
        SimEventKind::Transfer { item, qty, to } => {
            hasher.update([19u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
            hasher.update(to.0.to_le_bytes());
        }
        SimEventKind::Store { item, qty } => {
            hasher.update([20u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Retrieve { item, qty } => {
            hasher.update([21u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Give { item, qty } => {
            hasher.update([22u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Pack { item, qty } => {
            hasher.update([23u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Unpack { item, qty } => {
            hasher.update([24u8]);
            hash_item(hasher, *item);
            hasher.update(qty.to_le_bytes());
        }
        SimEventKind::Attack { target, damage } => {
            hasher.update([25u8]);
            hasher.update(target.0.to_le_bytes());
            hasher.update(damage.to_le_bytes());
        }
        SimEventKind::Flee => hasher.update([26u8]),
    }
}

fn hash_item(hasher: &mut impl sha2::Digest, item: ItemId) {
    match item {
        ItemId::Food(tag) => {
            hasher.update([0u8, tag]);
        }
        ItemId::Wood => hasher.update([1u8]),
        ItemId::Fiber => hasher.update([2u8]),
        ItemId::Stone => hasher.update([3u8]),
        ItemId::Basket => hasher.update([4u8]),
        ItemId::Spear => hasher.update([5u8]),
        ItemId::FishingRod => hasher.update([6u8]),
        ItemId::Backpack => hasher.update([7u8]),
    }
}

pub fn kind_label(kind: &SimEventKind) -> &'static str {
    match kind {
        SimEventKind::Wait => "Wait",
        SimEventKind::Rest => "Rest",
        SimEventKind::Move { .. } => "Move",
        SimEventKind::Gather { .. } => "Gather",
        SimEventKind::Drink => "Drink",
        SimEventKind::Eat { .. } => "Eat",
        SimEventKind::Hunt { .. } => "Hunt",
        SimEventKind::Fish { .. } => "Fish",
        SimEventKind::Farm { .. } => "Farm",
        SimEventKind::Craft { .. } => "Craft",
        SimEventKind::Speak { .. } => "Speak",
        SimEventKind::LlmWait => "LlmWait",
        SimEventKind::Propose { .. } => "Propose",
        SimEventKind::Support { .. } => "Support",
        SimEventKind::Oppose { .. } => "Oppose",
        SimEventKind::RuleBlocked { .. } => "RuleBlocked",
        SimEventKind::IncentiveApplied { .. } => "IncentiveApplied",
        SimEventKind::IncentiveEnded { .. } => "IncentiveEnded",
        SimEventKind::Died { .. } => "Died",
        SimEventKind::Transfer { .. } => "Transfer",
        SimEventKind::Store { .. } => "Store",
        SimEventKind::Retrieve { .. } => "Retrieve",
        SimEventKind::Give { .. } => "Give",
        SimEventKind::Pack { .. } => "Pack",
        SimEventKind::Unpack { .. } => "Unpack",
        SimEventKind::Attack { .. } => "Attack",
        SimEventKind::Flee => "Flee",
    }
}

pub fn kind_slug(kind: &SimEventKind) -> &'static str {
    match kind {
        SimEventKind::Wait => "wait",
        SimEventKind::Rest => "rest",
        SimEventKind::Move { .. } => "move",
        SimEventKind::Gather { .. } => "gather",
        SimEventKind::Drink => "drink",
        SimEventKind::Eat { .. } => "eat",
        SimEventKind::Hunt { .. } => "hunt",
        SimEventKind::Fish { .. } => "fish",
        SimEventKind::Farm { .. } => "farm",
        SimEventKind::Craft { .. } => "craft",
        SimEventKind::Speak { .. } => "speak",
        SimEventKind::LlmWait => "llm_wait",
        SimEventKind::Propose { .. } => "propose",
        SimEventKind::Support { .. } => "support",
        SimEventKind::Oppose { .. } => "oppose",
        SimEventKind::RuleBlocked { .. } => "rule_blocked",
        SimEventKind::IncentiveApplied { .. } => "incentive_applied",
        SimEventKind::IncentiveEnded { .. } => "incentive_ended",
        SimEventKind::Died { .. } => "died",
        SimEventKind::Transfer { .. } => "transfer",
        SimEventKind::Store { .. } => "store",
        SimEventKind::Retrieve { .. } => "retrieve",
        SimEventKind::Give { .. } => "give",
        SimEventKind::Pack { .. } => "pack",
        SimEventKind::Unpack { .. } => "unpack",
        SimEventKind::Attack { .. } => "attack",
        SimEventKind::Flee => "flee",
    }
}

pub fn jsonl_line_tick(line: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    v.get("tick")?.as_u64()
}

pub fn list_jsonl_ticks(path: impl AsRef<Path>) -> Result<Vec<u64>, SimError> {
    let text = std::fs::read_to_string(path)?;
    let mut ticks: Vec<u64> = text.lines().filter_map(jsonl_line_tick).collect();
    ticks.sort_unstable();
    ticks.dedup();
    Ok(ticks)
}

pub fn jsonl_tick_at_or_before(ticks: &[u64], want: u64) -> Option<u64> {
    ticks.iter().copied().filter(|t| *t <= want).next_back()
}

pub fn find_events_jsonl(dir: impl AsRef<Path>) -> Option<PathBuf> {
    let dir = dir.as_ref();
    let mut found: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_str()?;
        if name.ends_with("_events.jsonl") {
            found.push(p);
        }
    }
    found.sort();
    found.pop()
}

pub fn jsonl_lines_for_tick(path: impl AsRef<Path>, tick: u64) -> Result<Vec<String>, SimError> {
    let text = std::fs::read_to_string(path)?;
    Ok(text
        .lines()
        .filter(|l| jsonl_line_tick(l) == Some(tick))
        .map(str::to_string)
        .collect())
}

pub fn is_primary_kind(kind: &SimEventKind) -> bool {
    !matches!(
        kind,
        SimEventKind::Speak { .. }
            | SimEventKind::LlmWait
            | SimEventKind::IncentiveApplied { .. }
            | SimEventKind::IncentiveEnded { .. }
            | SimEventKind::Died { .. }
            | SimEventKind::RuleBlocked { .. }
    )
}
