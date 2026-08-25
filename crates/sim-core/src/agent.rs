use crate::species::f64_to_milli;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemId {
    Food(u8),
    Wood,
    Fiber,
    Stone,
    Basket,
    Spear,
    FishingRod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: u32,
    pub thirst: u32,
    pub energy: u32,
}

impl Needs {
    pub fn maxed(hunger_max: u32, thirst_max: u32, energy_max: u32) -> Self {
        Self {
            hunger: hunger_max,
            thirst: thirst_max,
            energy: energy_max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abilities {
    pub gather: u8,
    pub hunt: u8,
    pub fish: u8,
    pub farm: u8,
    pub craft: u8,
}

impl Default for Abilities {
    fn default() -> Self {
        Self {
            gather: 50,
            hunt: 40,
            fish: 40,
            farm: 30,
            craft: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Personality {
    pub openness: u8,
    pub conscientiousness: u8,
    pub extraversion: u8,
    pub agreeableness: u8,
    pub neuroticism: u8,
    pub perceptiveness: u8,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub allergy_tags: Vec<String>,
}

impl Default for Personality {
    fn default() -> Self {
        Self {
            openness: 50,
            conscientiousness: 50,
            extraversion: 50,
            agreeableness: 50,
            neuroticism: 50,
            perceptiveness: 50,
            traits: Vec::new(),
            allergy_tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Consumption {
    pub vegetation: u32,
    pub animal: u32,
    pub fish: u32,
    pub toxic_events: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub x: u32,
    pub y: u32,
    #[serde(default)]
    pub needs: Needs,
    #[serde(default)]
    pub abilities: Abilities,
    #[serde(default)]
    pub personality: Personality,
    #[serde(default)]
    pub inventory: BTreeMap<ItemId, u32>,
    #[serde(default)]
    pub inventory_cap: u32,
    #[serde(default)]
    pub consumption: Consumption,
    #[serde(default)]
    pub illness_ticks: u32,
    #[serde(default)]
    pub memory: Vec<crate::memory::MemoryEntry>,
    #[serde(default)]
    pub last_warn_tick: u64,
    /// Packed into the checkpoint `public_board` blob so M3 agent layouts still decode.
    #[serde(default, skip)]
    pub goals: Vec<crate::board::Goal>,
    #[serde(default, skip)]
    pub gathers_this_tick: u32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 10_000,
            thirst: 10_000,
            energy: 10_000,
        }
    }
}

impl Agent {
    pub fn new(id: AgentId, x: u32, y: u32) -> Self {
        Self {
            id,
            x,
            y,
            needs: Needs::default(),
            abilities: Abilities::default(),
            personality: Personality::default(),
            inventory: BTreeMap::new(),
            inventory_cap: 16,
            consumption: Consumption::default(),
            illness_ticks: 0,
            memory: Vec::new(),
            last_warn_tick: 0,
            goals: Vec::new(),
            gathers_this_tick: 0,
        }
    }

    pub fn inventory_count(&self) -> u32 {
        self.inventory.values().copied().sum()
    }

    pub fn try_add_item(&mut self, item: ItemId, qty: u32) -> u32 {
        if qty == 0 {
            return 0;
        }
        let used = self.inventory_count();
        let room = self.inventory_cap.saturating_sub(used);
        let add = qty.min(room);
        if add > 0 {
            *self.inventory.entry(item).or_insert(0) += add;
        }
        add
    }

    pub fn take_item(&mut self, item: ItemId, qty: u32) -> bool {
        let Some(have) = self.inventory.get_mut(&item) else {
            return false;
        };
        if *have < qty {
            return false;
        }
        *have -= qty;
        if *have == 0 {
            self.inventory.remove(&item);
        }
        true
    }

    pub fn has_tool(&self, tool: ItemId) -> bool {
        self.inventory.get(&tool).copied().unwrap_or(0) > 0
    }

    pub fn perceptiveness_factor(&self) -> f64 {
        0.5 + f64::from(self.personality.perceptiveness) / 100.0
    }

    pub fn hash_bytes(&self, hasher: &mut impl sha2::Digest) {
        hasher.update(self.id.0.to_le_bytes());
        hasher.update(self.x.to_le_bytes());
        hasher.update(self.y.to_le_bytes());
        hasher.update(self.needs.hunger.to_le_bytes());
        hasher.update(self.needs.thirst.to_le_bytes());
        hasher.update(self.needs.energy.to_le_bytes());
        hasher.update([
            self.abilities.gather,
            self.abilities.hunt,
            self.abilities.fish,
            self.abilities.farm,
            self.abilities.craft,
        ]);
        hasher.update([
            self.personality.openness,
            self.personality.conscientiousness,
            self.personality.extraversion,
            self.personality.agreeableness,
            self.personality.neuroticism,
            self.personality.perceptiveness,
        ]);
        for tag in &self.personality.allergy_tags {
            hasher.update(tag.as_bytes());
            hasher.update([0]);
        }
        hasher.update(self.illness_ticks.to_le_bytes());
        hasher.update(self.inventory_cap.to_le_bytes());
        for (item, qty) in &self.inventory {
            hasher.update(item_tag(*item));
            hasher.update(qty.to_le_bytes());
        }
        hasher.update(self.consumption.vegetation.to_le_bytes());
        hasher.update(self.consumption.animal.to_le_bytes());
        hasher.update(self.consumption.fish.to_le_bytes());
        hasher.update(self.consumption.toxic_events.to_le_bytes());
        hasher.update(self.last_warn_tick.to_le_bytes());
        hasher.update(self.gathers_this_tick.to_le_bytes());
        for g in &self.goals {
            g.hash_into(hasher);
        }
        for mem in &self.memory {
            mem.hash_into(hasher);
        }
    }
}

fn item_tag(item: ItemId) -> [u8; 2] {
    match item {
        ItemId::Food(s) => [1, s],
        ItemId::Wood => [2, 0],
        ItemId::Fiber => [3, 0],
        ItemId::Stone => [4, 0],
        ItemId::Basket => [5, 0],
        ItemId::Spear => [6, 0],
        ItemId::FishingRod => [7, 0],
    }
}

pub fn milli_from_config(v: f64) -> u32 {
    f64_to_milli(v)
}
