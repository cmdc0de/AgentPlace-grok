//! Item weight and haul energy. Overlay-overridable constants; not ExperimentConfig.

use crate::agent::ItemId;
use crate::species::f64_to_milli;

/// Default container item-count cap.
pub const SLOT_CAP: u32 = 16;
/// Default container weight cap (80.0 display).
pub const WEIGHT_CAP_MILLI: u32 = 8_000;
/// Haul multiplier 0.4 display → millipoints.
pub const HAUL_MILLI: u32 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageParams {
    pub slot_cap: u32,
    pub weight_cap_milli: u32,
    pub haul_milli: u32,
}

impl Default for StorageParams {
    fn default() -> Self {
        Self {
            slot_cap: SLOT_CAP,
            weight_cap_milli: WEIGHT_CAP_MILLI,
            haul_milli: HAUL_MILLI,
        }
    }
}

impl StorageParams {
    pub fn from_display(slot_cap: u32, weight_cap: f64, haul: f64) -> Self {
        Self {
            slot_cap: slot_cap.max(1),
            weight_cap_milli: f64_to_milli(weight_cap).max(1),
            haul_milli: f64_to_milli(haul).max(1),
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
        ItemId::Basket | ItemId::Spear | ItemId::FishingRod => 200,
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
}
