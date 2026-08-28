//! Item weight and haul energy. Overlay-overridable constants; not ExperimentConfig.

use crate::agent::ItemId;
use crate::species::f64_to_milli;
use std::collections::BTreeMap;

/// Default land-cell crate item-count cap.
pub const SLOT_CAP: u32 = 16;
/// Default crate weight cap (80.0 display).
pub const WEIGHT_CAP_MILLI: u32 = 8_000;
/// Pocket/crate haul multiplier 0.4 display → millipoints.
pub const HAUL_MILLI: u32 = 40;
/// Worn Basket pack: 8 slots.
pub const PACK_SLOT_CAP: u32 = 8;
/// Pack weight cap (25.0 display).
pub const PACK_WEIGHT_CAP_MILLI: u32 = 2_500;
/// Worn Backpack: 12 slots.
pub const BACKPACK_SLOT_CAP: u32 = 12;
/// Backpack weight cap (40.0 display).
pub const BACKPACK_WEIGHT_CAP_MILLI: u32 = 4_000;
/// Pack haul multiplier 0.1 display → millipoints.
pub const PACK_HAUL_MILLI: u32 = 10;
/// Move step factor ≈ 0.05 display per cell.
pub const MOVE_STEP_K_MILLI: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageParams {
    pub slot_cap: u32,
    pub weight_cap_milli: u32,
    pub haul_milli: u32,
    pub pack_slot_cap: u32,
    pub pack_weight_cap_milli: u32,
    pub pack_haul_milli: u32,
    pub move_step_k_milli: u32,
}

impl Default for StorageParams {
    fn default() -> Self {
        Self {
            slot_cap: SLOT_CAP,
            weight_cap_milli: WEIGHT_CAP_MILLI,
            haul_milli: HAUL_MILLI,
            pack_slot_cap: PACK_SLOT_CAP,
            pack_weight_cap_milli: PACK_WEIGHT_CAP_MILLI,
            pack_haul_milli: PACK_HAUL_MILLI,
            move_step_k_milli: MOVE_STEP_K_MILLI,
        }
    }
}

impl StorageParams {
    pub fn from_display(slot_cap: u32, weight_cap: f64, haul: f64) -> Self {
        Self {
            slot_cap: slot_cap.max(1),
            weight_cap_milli: f64_to_milli(weight_cap).max(1),
            haul_milli: f64_to_milli(haul).max(1),
            ..Self::default()
        }
    }
}

/// Display weights ×100. Food 0.5, fiber 0.4, wood 1.5, stone 3.0, tools 2.0.
pub fn item_weight_milli(item: ItemId) -> u32 {
    match item {
        ItemId::Food(_) => 50,
        ItemId::Fiber => 40,
        ItemId::Wood => 150,
        ItemId::Stone => 300,
        ItemId::Basket | ItemId::Spear | ItemId::FishingRod | ItemId::Backpack => 200,
    }
}

/// `qty * unit_weight_milli * haul_milli / 100` (qty is a count, not millipoints).
pub fn haul_cost_milli(item: ItemId, qty: u32, haul_milli: u32) -> u32 {
    if qty == 0 {
        return 0;
    }
    let w = u64::from(item_weight_milli(item));
    let h = u64::from(haul_milli.max(1));
    ((u64::from(qty) * w * h) / 100) as u32
}

pub fn items_weight_milli(items: &[(ItemId, u32)]) -> u32 {
    items
        .iter()
        .map(|(item, qty)| item_weight_milli(*item).saturating_mul(*qty))
        .sum()
}

pub fn map_weight_milli(items: &BTreeMap<ItemId, u32>) -> u32 {
    items
        .iter()
        .map(|(item, qty)| item_weight_milli(*item).saturating_mul(*qty))
        .sum()
}

pub fn map_slot_count(items: &BTreeMap<ItemId, u32>) -> u32 {
    items.values().copied().sum()
}

pub fn can_fit(
    items: &BTreeMap<ItemId, u32>,
    item: ItemId,
    qty: u32,
    slot_cap: u32,
    weight_cap_milli: u32,
) -> bool {
    if qty == 0 {
        return false;
    }
    let slots = map_slot_count(items).saturating_add(qty);
    let weight =
        map_weight_milli(items).saturating_add(item_weight_milli(item).saturating_mul(qty));
    slots <= slot_cap && weight <= weight_cap_milli
}

/// `loose_weight × haul × step_k + pack_weight × pack_haul × step_k` in millipoints.
pub fn move_cargo_cost_milli(
    loose_weight_milli: u32,
    pack_weight_milli: u32,
    haul_milli: u32,
    pack_haul_milli: u32,
    step_k_milli: u32,
) -> u32 {
    let k = u64::from(step_k_milli.max(1));
    let loose = u64::from(loose_weight_milli) * u64::from(haul_milli.max(1)) * k / 10_000;
    let packed = u64::from(pack_weight_milli) * u64::from(pack_haul_milli.max(1)) * k / 10_000;
    (loose + packed) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_stones_cost_about_four_point_eight_energy() {
        // 4 * 3.0 * 0.4 = 4.8 display → 480 milli
        let cost = haul_cost_milli(ItemId::Stone, 4, HAUL_MILLI);
        assert_eq!(cost, 480);
    }

    #[test]
    fn food_is_lighter_than_stone() {
        assert!(item_weight_milli(ItemId::Food(1)) < item_weight_milli(ItemId::Stone));
    }

    #[test]
    fn packed_food_move_cheaper_than_loose() {
        let food_w = item_weight_milli(ItemId::Food(1)) * 10;
        let loose =
            move_cargo_cost_milli(food_w, 0, HAUL_MILLI, PACK_HAUL_MILLI, MOVE_STEP_K_MILLI);
        let packed =
            move_cargo_cost_milli(0, food_w, HAUL_MILLI, PACK_HAUL_MILLI, MOVE_STEP_K_MILLI);
        assert!(packed < loose, "packed={packed} loose={loose}");
    }
}
