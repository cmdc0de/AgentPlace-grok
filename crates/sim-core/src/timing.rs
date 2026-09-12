//! Wall-clock tick / agent-step timings. Never hashed, never required to restore.

use crate::agent::AgentId;
use crate::error::SimError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

/// Overlay `[pipeline]`. Not on `ExperimentConfig` (not hashed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PipelineParams {
    pub hash_events: bool,
}

/// Overlay `[telemetry]`. Not on `ExperimentConfig` (not hashed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TelemetryParams {
    pub enabled: bool,
    pub otlp_endpoint: String,
}

impl TelemetryParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            telemetry: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
            otlp_endpoint: Option<String>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.telemetry.enabled.unwrap_or(false),
            otlp_endpoint: slice
                .telemetry
                .otlp_endpoint
                .unwrap_or_default()
                .trim()
                .to_string(),
        }
    }
}

/// Hash-neutral duration samples (sim ticks or viewer frames).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DurationStats {
    samples: Vec<u64>,
}

impl DurationStats {
    pub fn record(&mut self, ns: u64) {
        self.samples.push(ns);
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    pub fn total(&self) -> u64 {
        self.samples.iter().copied().sum()
    }

    pub fn average(&self) -> u64 {
        if self.samples.is_empty() {
            0
        } else {
            self.total() / self.samples.len() as u64
        }
    }

    pub fn median(&self) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }
        let mut v = self.samples.clone();
        v.sort_unstable();
        v[v.len() / 2]
    }

    pub fn min(&self) -> u64 {
        self.samples.iter().copied().min().unwrap_or(0)
    }

    pub fn max(&self) -> u64 {
        self.samples.iter().copied().max().unwrap_or(0)
    }
}

/// Best-effort RSS in bytes. None if the host cannot report it.
pub fn process_rss_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = text.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages.saturating_mul(4096))
}

impl PipelineParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            pipeline: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            hash_events: Option<bool>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            hash_events: slice.pipeline.hash_events.unwrap_or(false),
        }
    }
}

pub const PIPE_PERCEIVE: u8 = 1;
pub const PIPE_RETRIEVE: u8 = 2;
pub const PIPE_SELECT: u8 = 4;
pub const PIPE_EXECUTE: u8 = 8;
pub const PIPE_REMEMBER: u8 = 16;
pub const PIPE_COMPLETE: u8 = 31;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_stats_known_series() {
        let mut s = DurationStats::default();
        for n in [10u64, 20, 30, 40, 50] {
            s.record(n);
        }
        assert_eq!(s.count(), 5);
        assert_eq!(s.total(), 150);
        assert_eq!(s.average(), 30);
        assert_eq!(s.median(), 30);
        assert_eq!(s.min(), 10);
        assert_eq!(s.max(), 50);
    }

    #[test]
    fn telemetry_overlay_parses() {
        assert!(!TelemetryParams::from_config_toml("").enabled);
        let p = TelemetryParams::from_config_toml(
            "[telemetry]\nenabled = true\notlp_endpoint = \"http://127.0.0.1:4318\"\n",
        );
        assert!(p.enabled);
        assert_eq!(p.otlp_endpoint, "http://127.0.0.1:4318");
        assert!(TelemetryParams::from_config_toml("[telemetry]\nenabled = true\n")
            .otlp_endpoint
            .is_empty());
    }
}
