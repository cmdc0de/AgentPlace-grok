//! Deterministic simulation core. No Bevy, windowing, or network deps.

pub mod action;
pub mod agent;
pub mod checkpoint;
pub mod config;
pub mod error;
pub mod event_log;
pub mod execute;
pub mod llm;
pub mod memory;
pub mod observation;
pub mod policy;
pub mod seeding;
pub mod simulation;
pub mod species;
pub mod world;

pub use agent::{Abilities, Agent, AgentId, ItemId, Needs, Personality};
pub use checkpoint::{
    agents_markdown, append_events_jsonl, decode_checkpoint, encode_checkpoint, experiment_id,
    summary_markdown, write_run_checkpoint, CHECKPOINT_FORMAT_VERSION, CHECKPOINT_MAGIC,
};
pub use config::{CheckpointParams, ExperimentConfig, SeedSpec, SpawnMode};
pub use error::SimError;
pub use event_log::{SimEvent, SimEventKind};
pub use llm::{parse_choice_json, ActionChooser, ChooseError, Chooser, ReplayTable};
pub use seeding::{derive_seed, RngBank};
pub use simulation::{Simulation, StateHash};
pub use world::World;
