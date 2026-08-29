//! Overlay network settings. Not part of `ExperimentConfig` / `state_hash`.

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetworkFile {
    #[serde(default)]
    pub network: NetworkParams,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NetworkParams {
    #[serde(default)]
    pub tcp_listen: String,
    #[serde(default)]
    pub ws_listen: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_control: bool,
    #[serde(default)]
    pub lockstep: bool,
}

impl NetworkParams {
    pub fn from_toml_str(s: &str) -> Self {
        toml::from_str::<NetworkFile>(s)
            .map(|f| f.network)
            .unwrap_or_default()
    }

    pub fn from_path(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .map(|s| Self::from_toml_str(&s))
            .unwrap_or_default()
    }
}
