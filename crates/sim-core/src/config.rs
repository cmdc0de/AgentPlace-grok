use crate::error::SimError;
use crate::species::{SpeciesTables, default_species_tables};
use serde::{Deserialize, Serialize};
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

impl Serialize for SeedSpec {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Always a string so postcard (non-self-describing) can round-trip
        // through the existing string visitor used by TOML.
        match self {
            SeedSpec::Auto => serializer.serialize_str("auto"),
            SeedSpec::Random => serializer.serialize_str("random"),
            SeedSpec::Explicit(v) => serializer.serialize_str(&v.to_string()),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub master_seed: u64,
    #[serde(default)]
    pub simulation: SimulationParams,
    #[serde(default)]
    pub world: WorldParams,
    #[serde(default)]
    pub agents: AgentParams,
    #[serde(default)]
    pub checkpoint: CheckpointParams,
    #[serde(default)]
    pub observation: ObservationParams,
    #[serde(default)]
    pub needs: NeedsParams,
    #[serde(default)]
    pub communication: CommunicationParams,
    #[serde(default)]
    pub llm: LlmParams,
    #[serde(default)]
    pub proposals: ProposalParams,
    #[serde(default)]
    pub metrics: MetricsParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub species: SpeciesTables,
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
            species: default_species_tables(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_one")]
    pub animal_density: f64,
    #[serde(default = "default_one")]
    pub fish_density: f64,
    #[serde(default = "default_min_animals")]
    pub min_animals: u32,
    #[serde(default = "default_min_fish")]
    pub min_fish: u32,
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
            animal_density: 1.0,
            fish_density: 1.0,
            min_animals: default_min_animals(),
            min_fish: default_min_fish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_inv_cap")]
    pub inventory_capacity: u32,
    #[serde(default)]
    pub archetypes: Vec<AgentArchetype>,
    #[serde(default)]
    pub goals: GoalParams,
    #[serde(default)]
    pub memory: MemoryParams,
    #[serde(default)]
    pub social: SocialParams,
}

impl Default for AgentParams {
    fn default() -> Self {
        Self {
            count: default_agent_count(),
            spawn_mode: SpawnMode::Scattered,
            spawn_seed: SeedSpec::Auto,
            default_memory_capacity: default_memory(),
            start_with_basic_needs: true,
            inventory_capacity: default_inv_cap(),
            archetypes: Vec::new(),
            goals: GoalParams::default(),
            memory: MemoryParams::default(),
            social: SocialParams::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvictionPolicy {
    #[default]
    ImportanceAndRecency,
    Fifo,
    LowestImportance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryParams {
    #[serde(default = "default_memory")]
    pub capacity: u32,
    #[serde(default)]
    pub eviction_policy: EvictionPolicy,
    #[serde(default = "default_social_bonus")]
    pub social_memory_bonus: f64,
    #[serde(default = "default_true")]
    pub persistent_relationships: bool,
    #[serde(default)]
    pub enable_embeddings: bool,
    #[serde(default = "default_retrieval_k")]
    pub retrieval_k: u32,
}

impl Default for MemoryParams {
    fn default() -> Self {
        Self {
            capacity: default_memory(),
            eviction_policy: EvictionPolicy::default(),
            social_memory_bonus: default_social_bonus(),
            persistent_relationships: true,
            enable_embeddings: false,
            retrieval_k: default_retrieval_k(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialParams {
    #[serde(default = "default_influence")]
    pub base_influence_factor: f64,
    #[serde(default = "default_influence_decay")]
    pub influence_decay: f64,
    #[serde(default = "default_true")]
    pub track_relationships: bool,
    #[serde(default = "default_trust_thresh")]
    pub trust_support_threshold: f64,
}

impl Default for SocialParams {
    fn default() -> Self {
        Self {
            base_influence_factor: default_influence(),
            influence_decay: default_influence_decay(),
            track_relationships: true,
            trust_support_threshold: default_trust_thresh(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalParams {
    #[serde(default = "default_max_personal")]
    pub max_personal_goals: u32,
    #[serde(default = "default_max_public")]
    pub max_public_goals: u32,
    #[serde(default = "default_true")]
    pub can_adopt_public_goals: bool,
}

impl Default for GoalParams {
    fn default() -> Self {
        Self {
            max_personal_goals: default_max_personal(),
            max_public_goals: default_max_public(),
            can_adopt_public_goals: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalParams {
    #[serde(default = "default_max_open_prop")]
    pub max_open_proposals_per_agent: u32,
    #[serde(default = "default_accept")]
    pub default_acceptance_threshold: f64,
    #[serde(default = "default_prop_life")]
    pub proposal_lifetime_ticks: u64,
    #[serde(default = "default_msg_len")]
    pub max_proposal_length: u32,
    #[serde(default)]
    pub allow_meta_rules: bool,
    #[serde(default = "default_true")]
    pub public_board_always_visible: bool,
}

impl Default for ProposalParams {
    fn default() -> Self {
        Self {
            max_open_proposals_per_agent: default_max_open_prop(),
            default_acceptance_threshold: default_accept(),
            proposal_lifetime_ticks: default_prop_life(),
            max_proposal_length: default_msg_len(),
            allow_meta_rules: false,
            public_board_always_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsParams {
    #[serde(default = "default_metrics_every")]
    pub compute_every_n_ticks: u64,
    #[serde(default = "default_true")]
    pub export_csv: bool,
    #[serde(default = "default_true")]
    pub track_consumption: bool,
    #[serde(default = "default_true")]
    pub track_proposal_stats: bool,
    #[serde(default = "default_true")]
    pub track_relationship_graph: bool,
}

impl Default for MetricsParams {
    fn default() -> Self {
        Self {
            compute_every_n_ticks: default_metrics_every(),
            export_csv: true,
            track_consumption: true,
            track_proposal_stats: true,
            track_relationship_graph: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentArchetype {
    pub name: String,
    #[serde(default = "default_one")]
    pub weight: f64,
    #[serde(default)]
    pub personality: crate::agent::Personality,
    #[serde(default)]
    pub abilities: crate::agent::Abilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationParams {
    #[serde(default = "default_vision")]
    pub base_vision_range: f64,
    #[serde(default = "default_hearing")]
    pub base_hearing_range: f64,
    #[serde(default = "default_identity")]
    pub base_agent_identity_range: f64,
    #[serde(default)]
    pub full_information: bool,
}

impl Default for ObservationParams {
    fn default() -> Self {
        Self {
            base_vision_range: default_vision(),
            base_hearing_range: default_hearing(),
            base_agent_identity_range: default_identity(),
            full_information: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedsParams {
    #[serde(default = "default_need_max")]
    pub hunger_max: f64,
    #[serde(default = "default_hunger_decay")]
    pub hunger_decay_per_tick: f64,
    #[serde(default = "default_need_max")]
    pub thirst_max: f64,
    #[serde(default = "default_thirst_decay")]
    pub thirst_decay_per_tick: f64,
    #[serde(default = "default_need_max")]
    pub energy_max: f64,
    #[serde(default = "default_energy_decay")]
    pub energy_decay_per_tick: f64,
    #[serde(default = "default_energy_regen")]
    pub energy_regen_while_resting: f64,
    #[serde(default)]
    pub death_enabled: bool,
}

impl Default for NeedsParams {
    fn default() -> Self {
        Self {
            hunger_max: default_need_max(),
            hunger_decay_per_tick: default_hunger_decay(),
            thirst_max: default_need_max(),
            thirst_decay_per_tick: default_thirst_decay(),
            energy_max: default_need_max(),
            energy_decay_per_tick: default_energy_decay(),
            energy_regen_while_resting: default_energy_regen(),
            death_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationParams {
    #[serde(default = "default_true")]
    pub speech_is_free_action: bool,
    #[serde(default = "default_hearing")]
    pub base_speech_range: f64,
    #[serde(default = "default_shout_mult")]
    pub shout_range_multiplier: f64,
    #[serde(default = "default_shout_cost")]
    pub shout_energy_cost: f64,
    #[serde(default = "default_msg_len")]
    pub max_message_length: u32,
    #[serde(default = "default_true")]
    pub allow_overhearing: bool,
    #[serde(default = "default_warn_cd")]
    pub warn_cooldown_ticks: u64,
}

impl Default for CommunicationParams {
    fn default() -> Self {
        Self {
            speech_is_free_action: true,
            base_speech_range: default_hearing(),
            shout_range_multiplier: default_shout_mult(),
            shout_energy_cost: default_shout_cost(),
            max_message_length: default_msg_len(),
            allow_overhearing: true,
            warn_cooldown_ticks: default_warn_cd(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParams {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_url")]
    pub base_url: String,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_temp")]
    pub action_temperature: f64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub replay_file: String,
}

impl Default for LlmParams {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            base_url: default_llm_url(),
            api_key_env: default_api_key_env(),
            model: String::new(),
            action_temperature: default_temp(),
            max_retries: default_retries(),
            timeout_ms: default_timeout(),
            replay_file: String::new(),
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
fn default_ckpt_interval() -> u64 {
    500
}
fn default_keep_last() -> u32 {
    20
}
fn default_ckpt_dir() -> String {
    "checkpoints".into()
}
fn default_write_retries() -> u32 {
    3
}
fn default_min_animals() -> u32 {
    8
}
fn default_min_fish() -> u32 {
    8
}
fn default_inv_cap() -> u32 {
    16
}
fn default_vision() -> f64 {
    12.0
}
fn default_hearing() -> f64 {
    18.0
}
fn default_identity() -> f64 {
    8.0
}
fn default_need_max() -> f64 {
    100.0
}
fn default_hunger_decay() -> f64 {
    0.15
}
fn default_thirst_decay() -> f64 {
    0.25
}
fn default_energy_decay() -> f64 {
    0.08
}
fn default_energy_regen() -> f64 {
    0.4
}
fn default_shout_mult() -> f64 {
    1.8
}
fn default_shout_cost() -> f64 {
    5.0
}
fn default_msg_len() -> u32 {
    200
}
fn default_warn_cd() -> u64 {
    10
}
fn default_llm_provider() -> String {
    "mock".into()
}
fn default_llm_url() -> String {
    "http://localhost:11434".into()
}
fn default_api_key_env() -> String {
    "XAI_API_KEY".into()
}
fn default_temp() -> f64 {
    0.2
}
fn default_retries() -> u32 {
    2
}
fn default_timeout() -> u64 {
    12_000
}
fn default_max_personal() -> u32 {
    5
}
fn default_max_public() -> u32 {
    3
}
fn default_max_open_prop() -> u32 {
    3
}
fn default_accept() -> f64 {
    0.5
}
fn default_prop_life() -> u64 {
    2000
}
fn default_metrics_every() -> u64 {
    50
}
fn default_social_bonus() -> f64 {
    1.5
}
fn default_retrieval_k() -> u32 {
    8
}
fn default_influence() -> f64 {
    0.3
}
fn default_influence_decay() -> f64 {
    0.01
}
fn default_trust_thresh() -> f64 {
    20.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointParams {
    #[serde(default = "default_ckpt_interval")]
    pub auto_interval_ticks: u64,
    #[serde(default = "default_keep_last")]
    pub keep_last_n: u32,
    #[serde(default = "default_ckpt_dir")]
    pub directory: String,
    #[serde(default = "default_true")]
    pub write_markdown_summaries: bool,
    #[serde(default = "default_write_retries")]
    pub write_retries: u32,
}

impl Default for CheckpointParams {
    fn default() -> Self {
        Self {
            auto_interval_ticks: default_ckpt_interval(),
            keep_last_n: default_keep_last(),
            directory: default_ckpt_dir(),
            write_markdown_summaries: true,
            write_retries: default_write_retries(),
        }
    }
}

impl ExperimentConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, SimError> {
        let mut cfg: Self = toml::from_str(s).map_err(|e| SimError::Config(e.to_string()))?;
        cfg.world.species.ensure_defaults();
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
        if matches!(self.agents.spawn_mode, SpawnMode::FixedList) {
            return Err(SimError::Config(
                "spawn_mode=fixed_list requires agents.spawn_list (not implemented in M2)".into(),
            ));
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

    pub fn hunger_max_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.hunger_max)
    }
    pub fn thirst_max_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.thirst_max)
    }
    pub fn energy_max_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.energy_max)
    }
    pub fn hunger_decay_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.hunger_decay_per_tick)
    }
    pub fn thirst_decay_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.thirst_decay_per_tick)
    }
    pub fn energy_decay_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.energy_decay_per_tick)
    }
    pub fn energy_regen_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.needs.energy_regen_while_resting)
    }
    pub fn shout_energy_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.communication.shout_energy_cost)
    }
    pub fn memory_capacity(&self) -> u32 {
        if self.agents.memory.capacity > 0 {
            self.agents.memory.capacity
        } else {
            self.agents.default_memory_capacity
        }
    }
    pub fn social_bonus_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.agents.memory.social_memory_bonus)
    }
    pub fn influence_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.agents.social.base_influence_factor)
    }
    pub fn influence_decay_milli(&self) -> u32 {
        crate::species::f64_to_milli(self.agents.social.influence_decay)
    }
    pub fn trust_threshold_milli(&self) -> i16 {
        let v = (self.agents.social.trust_support_threshold * 100.0).round();
        v.clamp(-10_000.0, 10_000.0) as i16
    }
}
