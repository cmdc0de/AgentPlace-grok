//! Overlay tables that must not live on `ExperimentConfig` (postcard config_hash).

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OverlayFile {
    #[serde(default)]
    pub incentives: IncentivesOverlay,
    #[serde(default)]
    pub metrics: MetricsOverlay,
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
