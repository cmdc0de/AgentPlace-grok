//! Deterministic simulation core. No Bevy, windowing, or network deps.

pub mod action;
pub mod agent;
pub mod board;
pub mod checkpoint;
pub mod config;
pub mod decision_log;
pub mod error;
pub mod event_log;
pub mod execute;
pub mod incentive;
pub mod llm;
pub mod markers;
pub mod memory;
pub mod observation;
pub mod policy;
pub mod report;
pub mod seeding;
pub mod simulation;
pub mod social;
pub mod species;
pub mod timing;
pub mod world;

pub use agent::{Abilities, Agent, AgentId, ItemId, Needs, Personality};
pub use board::{
    AdoptedRule, Goal, Proposal, ProposalStatus, ProposalView, PublicBoard, StructuredRule,
};
pub use checkpoint::{
    agents_markdown, append_events_jsonl, decode_checkpoint, encode_checkpoint, experiment_id,
    summary_markdown, write_run_checkpoint, CHECKPOINT_FORMAT_VERSION, CHECKPOINT_MAGIC,
};
pub use config::{CheckpointParams, EvictionPolicy, ExperimentConfig, SeedSpec, SpawnMode};
pub use decision_log::{append_decisions_jsonl, DecisionRecord};
pub use error::SimError;
pub use event_log::{SimEvent, SimEventKind};
pub use incentive::{Incentive, IncentiveSchedule};
pub use llm::{parse_choice_json, ActionChooser, ChooseError, Chooser, ReplayTable};
pub use report::{build_report, report_csv, report_markdown, write_report};
pub use seeding::{derive_seed, RngBank};
pub use simulation::{Simulation, StateHash};
pub use social::{RelationView, RelationshipSummary};
pub use timing::{append_timing_jsonl, AgentTiming, TickTiming};
pub use world::World;
