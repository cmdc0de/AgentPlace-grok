//! Overlay `[conflict]`. Not on `ExperimentConfig` (not hashed).

use serde::Deserialize;

/// Energy paid by the attacker (millipoints).
pub const ATTACK_ENERGY_COST: u32 = 500;
/// Energy removed from the defender (millipoints).
pub const ATTACK_DAMAGE: u32 = 2000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictParams {
    pub enabled: bool,
}

impl Default for ConflictParams {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl ConflictParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            conflict: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.conflict.enabled.unwrap_or(false),
        }
    }
}
