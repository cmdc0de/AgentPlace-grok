//! Wall-clock tick / agent-step timings. Never hashed, never required to restore.

use crate::agent::AgentId;
use crate::error::SimError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentTiming {
    pub agent: u64,
    pub perceive_ns: u64,
    pub retrieve_ns: u64,
    #[serde(default)]
    pub reflect_ns: u64,
    #[serde(default)]
    pub plan_ns: u64,
    pub select_ns: u64,
    pub execute_ns: u64,
    pub remember_ns: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TickTiming {
    pub tick: u64,
    pub wall_ns: u64,
    pub world_ns: u64,
    pub board_ns: u64,
    pub incentive_ns: u64,
    pub agents_ns: u64,
    #[serde(default)]
    pub agents: Vec<AgentTiming>,
}

impl TickTiming {
    /// Hash-neutral completeness: one timing row per living agent.
    pub fn pipeline_complete(&self, living: usize) -> bool {
        self.agents.len() == living
    }
}

pub fn ns_since(start: Instant) -> u64 {
    start.elapsed().as_nanos() as u64
}

pub fn append_timing_jsonl(path: impl AsRef<Path>, timing: &TickTiming) -> Result<(), SimError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(timing)
        .map_err(|e| SimError::Checkpoint(format!("timing json: {e}")))?;
    writeln!(file, "{line}")?;
    Ok(())
}

impl AgentTiming {
    pub fn new(id: AgentId) -> Self {
        Self {
            agent: id.0,
            ..Self::default()
        }
    }
}
