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
    Backpack,
    Catalog(u16),
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
    /// Worn Basket pack. Empty unless `has_tool(Basket)`. Trailing `serde default` so M10 ckpts load.
    #[serde(default)]
    pub pack: BTreeMap<ItemId, u32>,
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
    #[serde(default, skip)]
    pub relationships: std::collections::BTreeMap<AgentId, crate::social::RelationshipSummary>,
    #[serde(default, skip)]
    pub next_memory_id: u64,
    #[serde(default, skip)]
    pub influence_factor: u32,
    /// Short-term plan (not executed). Packed in the checkpoint board blob.
    #[serde(default, skip)]
    pub plan: Vec<String>,
    /// Combat HP. Default 10_000; packed in the board blob. Not hashed at default.
    #[serde(default = "default_health", skip)]
    pub health: u32,
    #[serde(default, skip)]
    pub incapacitated: bool,
    /// D&D-like scores. 0 = unused; packed in the board blob.
    #[serde(default, skip)]
    pub sheet: crate::sheet::AbilitySheet,
    #[serde(default, skip)]
    pub kinship: crate::kinship::Kinship,
    /// Aging overlay. 0 = unused; packed in the board blob. Not hashed at 0.
    #[serde(default, skip)]
    pub age_ticks: u64,
    /// Culture id. 0 = unused; packed in the board blob. Not hashed at 0.
    #[serde(default, skip)]
    pub culture: u8,
    /// Successful tool bonus-uses since last break. Packed in the board blob.
    #[serde(default, skip)]
    pub tool_uses: BTreeMap<ItemId, u32>,
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

/// Full health (millipoints). Default is hash-neutral.
pub const HEALTH_MAX: u32 = 10_000;

fn default_health() -> u32 {
    HEALTH_MAX
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
            pack: BTreeMap::new(),
            consumption: Consumption::default(),
            illness_ticks: 0,
            memory: Vec::new(),
            last_warn_tick: 0,
            goals: Vec::new(),
            gathers_this_tick: 0,
            relationships: BTreeMap::new(),
            next_memory_id: 1,
            influence_factor: 0,
            plan: Vec::new(),
            health: HEALTH_MAX,
            incapacitated: false,
            sheet: crate::sheet::AbilitySheet::default(),
            kinship: crate::kinship::Kinship::default(),
            age_ticks: 0,
            culture: 0,
            tool_uses: BTreeMap::new(),
        }
    }

    pub fn remember(
        &mut self,
        cap: u32,
        policy: crate::config::EvictionPolicy,
        social_bonus: u32,
        persist: bool,
        mut entry: crate::memory::MemoryEntry,
    ) -> Vec<String> {
        if entry.id == 0 {
            entry.id = self.next_memory_id;
            self.next_memory_id = self.next_memory_id.saturating_add(1);
        }
        crate::memory::remember_policy(
            &mut self.memory,
            cap,
            policy,
            social_bonus,
            persist,
            &self.relationships,
            entry,
        )
    }

    pub fn inventory_count(&self) -> u32 {
        self.inventory.values().copied().sum()
    }

    pub fn has_basket(&self) -> bool {
        self.has_tool(ItemId::Basket)
    }

    pub fn basket_count(&self) -> u32 {
        self.inventory.get(&ItemId::Basket).copied().unwrap_or(0)
    }

    pub fn has_backpack(&self) -> bool {
        self.has_tool(ItemId::Backpack)
    }

    pub fn backpack_count(&self) -> u32 {
        self.inventory.get(&ItemId::Backpack).copied().unwrap_or(0)
    }

    pub fn worn_baskets(&self, params: &crate::haul::StorageParams) -> u32 {
        self.basket_count().min(params.max_worn_baskets)
    }

    pub fn worn_backpacks(&self, params: &crate::haul::StorageParams) -> u32 {
        self.backpack_count().min(params.max_worn_backpacks)
    }

    pub fn has_pack(&self, params: &crate::haul::StorageParams) -> bool {
        self.worn_baskets(params) > 0 || self.worn_backpacks(params) > 0
    }

    pub fn is_pack_carrier(item: ItemId) -> bool {
        matches!(item, ItemId::Basket | ItemId::Backpack)
    }

    pub fn worn_pack_caps_for(
        basket_count: u32,
        backpack_count: u32,
        params: &crate::haul::StorageParams,
    ) -> (u32, u32) {
        let worn_baskets = basket_count.min(params.max_worn_baskets);
        let worn_backpacks = backpack_count.min(params.max_worn_backpacks);
        (
            worn_baskets
                .saturating_mul(params.pack_slot_cap)
                .saturating_add(worn_backpacks.saturating_mul(crate::haul::BACKPACK_SLOT_CAP)),
            worn_baskets
                .saturating_mul(params.pack_weight_cap_milli)
                .saturating_add(
                    worn_backpacks.saturating_mul(crate::haul::BACKPACK_WEIGHT_CAP_MILLI),
                ),
        )
    }

    pub fn worn_pack_caps(&self, params: &crate::haul::StorageParams) -> (u32, u32) {
        let (slots, weight) =
            Self::worn_pack_caps_for(self.basket_count(), self.backpack_count(), params);
        let m = crate::sheet::AbilitySheet::modifier(self.sheet.strength);
        (adj_str_cap(slots, m), adj_str_cap(weight, m * 250))
    }

    /// Pocket slot cap after STR. Score 0 ⇒ `inventory_cap`. Do not write the field.
    pub fn pocket_slot_cap(&self) -> u32 {
        adj_str_cap(
            self.inventory_cap,
            crate::sheet::AbilitySheet::modifier(self.sheet.strength),
        )
    }

    pub fn pack_count(&self) -> u32 {
        crate::haul::map_slot_count(&self.pack)
    }

    pub fn pack_weight_milli(&self) -> u32 {
        crate::haul::map_weight_milli(&self.pack)
    }

    pub fn pocket_weight_milli(&self) -> u32 {
        crate::haul::map_weight_milli(&self.inventory)
    }

    /// How many of `item` still fit in pockets (slots + optional STR weight).
    pub fn pocket_fit_qty(&self, item: ItemId) -> u32 {
        let slot_room = self
            .pocket_slot_cap()
            .saturating_sub(self.inventory_count());
        if let Some(cap) = self.sheet.pocket_weight_cap() {
            let unit = crate::haul::item_weight_milli(item);
            let wroom = cap.saturating_sub(self.pocket_weight_milli());
            let by_w = if unit == 0 { slot_room } else { wroom / unit };
            slot_room.min(by_w)
        } else {
            slot_room
        }
    }

    pub fn shows_satchel(&self, params: &crate::haul::StorageParams) -> bool {
        self.has_pack(params)
    }

    pub fn try_add_pack(
        &mut self,
        item: ItemId,
        qty: u32,
        params: &crate::haul::StorageParams,
    ) -> u32 {
        if qty == 0 || Self::is_pack_carrier(item) || !self.has_pack(params) {
            return 0;
        }
        let (slot_cap, weight_cap) = self.worn_pack_caps(params);
        let room_slots = slot_cap.saturating_sub(self.pack_count());
        let room_w = weight_cap.saturating_sub(self.pack_weight_milli());
        let unit = crate::haul::item_weight_milli(item);
        let by_weight = if unit == 0 { qty } else { room_w / unit };
        let add = qty.min(room_slots).min(by_weight);
        if add > 0 {
            *self.pack.entry(item).or_insert(0) += add;
        }
        add
    }

    pub fn take_pack(&mut self, item: ItemId, qty: u32) -> bool {
        let Some(have) = self.pack.get_mut(&item) else {
            return false;
        };
        if *have < qty {
            return false;
        }
        *have -= qty;
        if *have == 0 {
            self.pack.remove(&item);
        }
        true
    }

    /// Prefer pack when it holds `qty`, else pockets. `None` if neither can pay.
    pub fn take_from_pack_or_pockets(&mut self, item: ItemId, qty: u32) -> Option<bool> {
        if !Self::is_pack_carrier(item)
            && self.pack.get(&item).copied().unwrap_or(0) >= qty
            && self.take_pack(item, qty)
        {
            return Some(true);
        }
        if self.take_item(item, qty) {
            return Some(false);
        }
        None
    }

    pub fn add_to_pockets_or_pack(
        &mut self,
        item: ItemId,
        qty: u32,
        params: &crate::haul::StorageParams,
    ) -> u32 {
        let added = self.try_add_item(item, qty);
        let rest = qty.saturating_sub(added);
        if rest == 0 {
            return added;
        }
        added + self.try_add_pack(item, rest, params)
    }

    pub fn has_carry_room(&self, params: &crate::haul::StorageParams) -> bool {
        let pocket = self.inventory_count() < self.pocket_slot_cap()
            && self
                .sheet
                .pocket_weight_cap()
                .is_none_or(|cap| self.pocket_weight_milli() < cap);
        pocket
            || (self.has_pack(params) && {
                let (slot_cap, _) = self.worn_pack_caps(params);
                self.pack_count() < slot_cap
            })
    }

    /// Split pack contents into pocket-bound vs leftover after `extra_pocket_slots` free up.
    pub fn split_pack_unload(
        &self,
        extra_pocket_slots: u32,
    ) -> (Vec<(ItemId, u32)>, Vec<(ItemId, u32)>) {
        let mut room = self
            .pocket_slot_cap()
            .saturating_sub(self.inventory_count())
            .saturating_add(extra_pocket_slots);
        let mut to_pockets = Vec::new();
        let mut leftover = Vec::new();
        for (item, qty) in &self.pack {
            if *qty == 0 {
                continue;
            }
            let into = (*qty).min(room);
            if into > 0 {
                to_pockets.push((*item, into));
                room -= into;
            }
            if *qty > into {
                leftover.push((*item, *qty - into));
            }
        }
        (to_pockets, leftover)
    }

    pub fn move_cost_milli(&self, params: &crate::haul::StorageParams) -> u32 {
        let base = crate::haul::move_cargo_cost_milli(
            self.pocket_weight_milli(),
            if self.has_basket() {
                self.pack_weight_milli()
            } else {
                0
            },
            params.haul_milli,
            params.pack_haul_milli,
            params.move_step_k_milli,
        );
        self.sheet.adjust_move_cost(base)
    }

    pub fn try_add_item(&mut self, item: ItemId, qty: u32) -> u32 {
        if qty == 0 {
            return 0;
        }
        let add = qty.min(self.pocket_fit_qty(item));
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

    pub fn hash_bytes(&self, hasher: &mut impl sha2::Digest, catalog_slugs: &[String]) {
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
            hash_item_id(hasher, *item, catalog_slugs);
            hasher.update(qty.to_le_bytes());
        }
        for (item, qty) in &self.pack {
            hash_item_id(hasher, *item, catalog_slugs);
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
        hasher.update(self.next_memory_id.to_le_bytes());
        hasher.update(self.influence_factor.to_le_bytes());
        for (id, rel) in &self.relationships {
            hasher.update(id.0.to_le_bytes());
            rel.hash_into(hasher);
        }
        for mem in &self.memory {
            mem.hash_into(hasher);
        }
        for step in &self.plan {
            hasher.update(step.as_bytes());
            hasher.update([0]);
        }
        if self.health != HEALTH_MAX {
            hasher.update(self.health.to_le_bytes());
        }
        if self.incapacitated {
            hasher.update([1u8]);
        }
        self.sheet.hash_into(hasher);
        self.kinship.hash_into(hasher);
        if self.age_ticks != 0 {
            hasher.update(self.age_ticks.to_le_bytes());
        }
        if self.culture != 0 {
            hasher.update([self.culture]);
        }
        if !self.tool_uses.is_empty() {
            for (item, n) in &self.tool_uses {
                hash_item_id(hasher, *item, catalog_slugs);
                hasher.update(n.to_le_bytes());
            }
        }
    }
}

fn adj_str_cap(base: u32, delta: i32) -> u32 {
    if base == 0 {
        0
    } else {
        (base as i32 + delta).max(1) as u32
    }
}

fn hash_item_id(hasher: &mut impl sha2::Digest, item: ItemId, slugs: &[String]) {
    hasher.update(item_tag(item));
    if let ItemId::Catalog(n) = item {
        if let Some(s) = slugs.get(n as usize).filter(|s| !s.is_empty()) {
            hasher.update(s.as_bytes());
        } else {
            hasher.update(n.to_le_bytes());
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
        ItemId::Backpack => [8, 0],
        ItemId::Catalog(_) => [9, 0],
    }
}

pub fn milli_from_config(v: f64) -> u32 {
    f64_to_milli(v)
}
