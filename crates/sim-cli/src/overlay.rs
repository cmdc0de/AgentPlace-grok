//! Overlay tables that must not live on `ExperimentConfig` (postcard config_hash).

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OverlayFile {
    #[serde(default)]
    pub incentives: IncentivesOverlay,
    #[serde(default)]
    pub metrics: MetricsOverlay,
    #[serde(default)]
    pub storage: StorageOverlay,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StorageOverlay {
    pub slot_cap: Option<u32>,
    pub weight_cap: Option<f64>,
    pub haul: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IncentivesOverlay {
    #[serde(default)]
    pub schedule: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricsOverlay {
    #[serde(default)]
    pub timing: Option<bool>,
}

impl OverlayFile {
    pub fn from_path(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}
