//! Deterministic simulation core. No Bevy, windowing, or network deps.

pub mod action;
pub mod agent;
pub mod board;
pub mod checkpoint;
pub mod combat_fx;
pub mod compare;
pub mod config;
pub mod conflict;
pub mod decision_log;
pub mod error;
pub mod event_log;
pub mod execute;
pub mod haul;
pub mod incentive;
pub mod inspector;
pub mod inventions;
pub mod kinship;
pub mod llm;
pub mod markers;
pub mod memory;
pub mod objects;
pub mod observation;
pub mod policy;
pub mod report;
pub mod seeding;
pub mod sheet;
pub mod simulation;
pub mod social;
pub mod species;
pub mod timing;
pub mod voting;
pub mod world;

pub use agent::{Abilities, Agent, AgentId, HEALTH_MAX, ItemId, Needs, Personality};
pub use board::{
    AdoptedRule, Goal, Proposal, ProposalStatus, ProposalView, PublicBoard, StructuredRule,
};
pub use checkpoint::{
    CHECKPOINT_FORMAT_VERSION, CHECKPOINT_MAGIC, agents_markdown, append_events_jsonl,
    ckpt_at_or_before, decode_checkpoint, encode_checkpoint, experiment_id, list_checkpoints,
    summary_markdown, write_run_checkpoint,
};
pub use combat_fx::{
    CombatFxJob, CombatRole, combat_fx_jobs, combat_hud_line, combat_hud_lines, combat_role,
};
pub use compare::{CompareReport, compare_csv, compare_markdown, compare_runs, load_compare_pair};
pub use config::{CheckpointParams, EvictionPolicy, ExperimentConfig, SeedSpec, SpawnMode};
pub use conflict::ConflictParams;
pub use decision_log::{DecisionRecord, append_decisions_jsonl};
pub use error::SimError;
pub use event_log::{
    SimEvent, SimEventKind, find_events_jsonl, jsonl_lines_for_tick, jsonl_tick_at_or_before,
    list_jsonl_ticks,
};
pub use haul::StorageParams;
pub use incentive::{Incentive, IncentiveSchedule, IncentiveVisibility};
pub use inspector::InspectorView;
pub use inventions::{Invention, InventionKind, InventionsParams};
pub use kinship::{Kinship, PopulationParams};
pub use llm::{
    ActionChooser, ChooseError, Chooser, LLM_SKIP_SENTINEL, LLM_WAIT_SENTINEL, LlmBarrierParams,
    ReplayTable, extract_json_payload, is_llm_wait_response, is_skip_response, parse_choice_json,
    parse_item,
};
pub use objects::{
    CatalogEntry, CatalogParams, ObjectDef, VisualDef, apply_species_defs, catalog_entries,
    load_object_defs, pick_visual_path, visual_for_id,
};
pub use observation::{IncentiveView, InventoryView};
pub use report::{build_report, report_csv, report_markdown, write_report};
pub use seeding::{RngBank, derive_seed};
pub use sheet::{AbilitySheet, SheetParams};
pub use simulation::{Simulation, StateHash};
pub use social::{RelationView, RelationshipSummary};
pub use timing::{AgentTiming, TickTiming, append_timing_jsonl};
pub use voting::{CouncilTally, VoteAccept, VoteWeight, VotingParams};
pub use world::World;
