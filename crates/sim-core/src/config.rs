use crate::error::SimError;
use serde::Deserialize;
use std::fmt;
use std::path::Path;

pub const MIN_MAP_SIZE: u32 = 32;
pub const MIN_MAX_HEIGHT: u32 = 8;
pub const MIN_AGENT_COUNT: u32 = 2;
pub const MIN_MEMORY_CAPACITY: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedSpec {
    Auto,
    Random,
    Explicit(u64),
}

impl Default for SeedSpec {
    fn default() -> Self {
        SeedSpec::Auto
    }
}

impl<'de> Deserialize<'de> for SeedSpec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SeedVisitor;
        impl<'de> serde::de::Visitor<'de> for SeedVisitor {
            type Value = SeedSpec;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("\"auto\", \"random\", or an unsigned integer")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(SeedSpec::Explicit(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                if v < 0 {
                    return Err(E::custom("seed must be non-negative"));
                }
                Ok(SeedSpec::Explicit(v as u64))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "auto" => Ok(SeedSpec::Auto),
                    "random" => Ok(SeedSpec::Random),
                    other => other
                        .parse::<u64>()
                        .map(SeedSpec::Explicit)
                        .map_err(E::custom),
                }
            }
        }
        deserializer.deserialize_any(SeedVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnMode {
    Scattered,
    Clustered,
    FixedList,
}

impl Default for SpawnMode {
    fn default() -> Self {
        SpawnMode::Scattered
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExperimentConfig {
    pub master_seed: u64,
    #[serde(default)]
    pub simulation: SimulationParams,
    #[serde(default)]
    pub world: WorldParams,
    #[serde(default)]
    pub agents: AgentParams,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimulationParams {
    #[serde(default = "default_step_duration")]
    pub step_duration_secs: f64,
    #[serde(default = "default_max_ticks")]
    pub max_ticks: u64,
    #[serde(default)]
    pub pause_when_empty: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            step_duration_secs: default_step_duration(),
            max_ticks: default_max_ticks(),
            pause_when_empty: false,
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldParams {
    #[serde(default)]
    pub seed: SeedSpec,
    #[serde(default = "default_world_width")]
    pub width: u32,
    #[serde(default = "default_world_height")]
    pub height: u32,
    #[serde(default = "default_max_height")]
    pub max_height: u32,
    #[serde(default)]
    pub terrain: TerrainParams,
    #[serde(default)]
    pub resources: ResourceParams,
}

impl Default for WorldParams {
    fn default() -> Self {
        Self {
            seed: SeedSpec::Auto,
            width: default_world_width(),
            height: default_world_height(),
            max_height: default_max_height(),
            terrain: TerrainParams::default(),
            resources: ResourceParams::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TerrainParams {
    #[serde(default = "default_octaves")]
    pub octaves: u32,
    #[serde(default = "default_persistence")]
    pub persistence: f64,
    #[serde(default = "default_lacunarity")]
    pub lacunarity: f64,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            octaves: default_octaves(),
            persistence: default_persistence(),
            lacunarity: default_lacunarity(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceParams {
    #[serde(default = "default_one")]
    pub mineral_density: f64,
    #[serde(default = "default_one")]
    pub vegetation_density: f64,
    #[serde(default = "default_water_coverage")]
    pub water_coverage: f64,
    #[serde(default = "default_min_mineral")]
    pub min_mineral_nodes: u32,
    #[serde(default = "default_min_water")]
    pub min_fresh_water: u32,
    #[serde(default = "default_min_veg")]
    pub min_vegetation_patches: u32,
}

impl Default for ResourceParams {
    fn default() -> Self {
        Self {
            mineral_density: 1.0,
            vegetation_density: 1.0,
            water_coverage: default_water_coverage(),
            min_mineral_nodes: default_min_mineral(),
            min_fresh_water: default_min_water(),
            min_vegetation_patches: default_min_veg(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentParams {
    #[serde(default = "default_agent_count")]
    pub count: u32,
    #[serde(default)]
    pub spawn_mode: SpawnMode,
    #[serde(default)]
    pub spawn_seed: SeedSpec,
    #[serde(default = "default_memory")]
    pub default_memory_capacity: u32,
    #[serde(default = "default_true")]
    pub start_with_basic_needs: bool,
}

impl Default for AgentParams {
    fn default() -> Self {
        Self {
            count: default_agent_count(),
            spawn_mode: SpawnMode::Scattered,
            spawn_seed: SeedSpec::Auto,
            default_memory_capacity: default_memory(),
            start_with_basic_needs: true,
        }
    }
}

fn default_step_duration() -> f64 {
    1.0
}
fn default_max_ticks() -> u64 {
    100_000
}
fn default_log_level() -> String {
    "info".into()
}
fn default_world_width() -> u32 {
    64
}
fn default_world_height() -> u32 {
    64
}
fn default_max_height() -> u32 {
    16
}
fn default_octaves() -> u32 {
    4
}
fn default_persistence() -> f64 {
    0.5
}
fn default_lacunarity() -> f64 {
    2.0
}
fn default_one() -> f64 {
    1.0
}
fn default_water_coverage() -> f64 {
    0.25
}
fn default_min_mineral() -> u32 {
    10
}
fn default_min_water() -> u32 {
    5
}
fn default_min_veg() -> u32 {
    20
}
fn default_agent_count() -> u32 {
    16
}
fn default_memory() -> u32 {
    128
}
fn default_true() -> bool {
    true
}

impl ExperimentConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, SimError> {
        let cfg: Self = toml::from_str(s).map_err(|e| SimError::Config(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load_path(path: impl AsRef<Path>) -> Result<Self, SimError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_toml_str(&text)
    }

    pub fn validate(&self) -> Result<(), SimError> {
        if self.world.width < MIN_MAP_SIZE || self.world.height < MIN_MAP_SIZE {
            return Err(SimError::Config(format!(
                "world size must be at least {MIN_MAP_SIZE}x{MIN_MAP_SIZE}"
            )));
        }
        if self.world.max_height < MIN_MAX_HEIGHT {
            return Err(SimError::Config(format!(
                "max_height must be at least {MIN_MAX_HEIGHT}"
            )));
        }
        if self.world.terrain.octaves < 1 {
            return Err(SimError::Config("octaves must be >= 1".into()));
        }
        if self.agents.count < MIN_AGENT_COUNT {
            return Err(SimError::Config(format!(
                "agents.count must be at least {MIN_AGENT_COUNT}"
            )));
        }
        if self.agents.default_memory_capacity < MIN_MEMORY_CAPACITY {
            return Err(SimError::Config(format!(
                "default_memory_capacity must be at least {MIN_MEMORY_CAPACITY}"
            )));
        }
        if !(0.0..=1.0).contains(&self.world.resources.water_coverage) {
            return Err(SimError::Config(
                "water_coverage must be in [0.0, 1.0]".into(),
            ));
        }
        if self.simulation.step_duration_secs < 0.05 && self.simulation.step_duration_secs != 0.0 {
            return Err(SimError::Config(
                "step_duration_secs must be >= 0.05 (or 0 for pure discrete)".into(),
            ));
        }
        Ok(())
    }
}
