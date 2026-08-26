use crate::action::Recipe;
use crate::agent::{AgentId, ItemId};
use serde::{Deserialize, Serialize};

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
    }
}
