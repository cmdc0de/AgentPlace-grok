use crate::agent::{AgentId, ItemId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryAction {
    Wait,
    Rest,
    MoveRelative {
        dx: i32,
        dy: i32,
    },
    Gather {
        species: u8,
    },
    Drink,
    Eat {
        item: ItemId,
    },
    Hunt,
    Fish,
    Farm {
        species: u8,
    },
    Craft {
        recipe: Recipe,
    },
    Propose {
        text: String,
        rule: Option<crate::board::StructuredRule>,
    },
    Support {
        proposal_id: u64,
    },
    Oppose {
        proposal_id: u64,
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
    },
    Flee,
    PairBond {
        target: AgentId,
    },
    Reproduce {
        with: AgentId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Recipe {
    Basket,
    Spear,
    FishingRod,
    Backpack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeakTarget {
    Broadcast,
    Directed(Vec<AgentId>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Speak {
    pub to: SpeakTarget,
    pub shout: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChosenAction {
    pub primary: PrimaryAction,
    #[serde(default)]
    pub speak: Option<Speak>,
}

impl ChosenAction {
    pub fn wait() -> Self {
        Self {
            primary: PrimaryAction::Wait,
            speak: None,
        }
    }

    pub fn primary(primary: PrimaryAction) -> Self {
        Self {
            primary,
            speak: None,
        }
    }
}
