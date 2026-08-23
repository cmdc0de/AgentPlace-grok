//! Deterministic simulation core. No Bevy, windowing, or network deps.

pub mod agent;
pub mod config;
pub mod error;
pub mod event_log;
pub mod seeding;
pub mod simulation;
pub mod world;

pub use agent::{Agent, AgentId};
pub use config::{ExperimentConfig, SeedSpec, SpawnMode};
pub use error::SimError;
pub use event_log::{SimEvent, SimEventKind};
pub use seeding::{derive_seed, RngBank};
pub use simulation::{Simulation, StateHash};
pub use world::World;
