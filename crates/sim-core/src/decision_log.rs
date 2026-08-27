//! Derived per-agent-tick decision records. Not part of `state_hash`.

use crate::action::{ChosenAction, PrimaryAction};
use crate::agent::AgentId;
use crate::error::SimError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub tick: u64,
    pub agent: u64,
    pub chooser: String,
    pub policy_branch: String,
    pub call_seed: u64,
    pub prompt_hash: String,
    #[serde(default)]
    pub retrieved_memory_ids: Vec<u64>,
    #[serde(default)]
    pub relation_ids: Vec<u64>,
    #[serde(default)]
    pub legal: Vec<String>,
    pub primary: serde_json::Value,
    #[serde(default)]
    pub speak: Option<serde_json::Value>,
    #[serde(default)]
    pub reasoning: Option<String>,
}

pub fn legal_names(legal: &[PrimaryAction]) -> Vec<String> {
    legal
        .iter()
        .map(|a| match a {
            PrimaryAction::Wait => "Wait".into(),
            PrimaryAction::Rest => "Rest".into(),
            PrimaryAction::Drink => "Drink".into(),
            PrimaryAction::Hunt => "Hunt".into(),
            PrimaryAction::Fish => "Fish".into(),
            PrimaryAction::MoveRelative { .. } => "MoveRelative".into(),
            PrimaryAction::Gather { .. } => "Gather".into(),
            PrimaryAction::Eat { .. } => "Eat".into(),
            PrimaryAction::Farm { .. } => "Farm".into(),
            PrimaryAction::Craft { .. } => "Craft".into(),
            PrimaryAction::Propose { .. } => "Propose".into(),
            PrimaryAction::Support { .. } => "Support".into(),
            PrimaryAction::Oppose { .. } => "Oppose".into(),
            PrimaryAction::Transfer { .. } => "Transfer".into(),
            PrimaryAction::Store { .. } => "Store".into(),
            PrimaryAction::Retrieve { .. } => "Retrieve".into(),
            PrimaryAction::Pack { .. } => "Pack".into(),
            PrimaryAction::Unpack { .. } => "Unpack".into(),
        })
        .collect()
}

pub fn primary_json(primary: &PrimaryAction) -> serde_json::Value {
    serde_json::to_value(primary).unwrap_or(serde_json::json!({"action":"Wait"}))
}

pub fn speak_json(choice: &ChosenAction) -> Option<serde_json::Value> {
    choice
        .speak
        .as_ref()
        .and_then(|s| serde_json::to_value(s).ok())
}

pub fn append_decisions_jsonl(
    path: impl AsRef<Path>,
    records: &[DecisionRecord],
) -> Result<(), SimError> {
    if records.is_empty() {
        return Ok(());
    }
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
    for rec in records {
        let line = serde_json::to_string(rec).unwrap_or_else(|_| "{}".into());
        writeln!(file, "{line}")?;
    }
    Ok(())
}

pub fn mock_branch_for(primary: &PrimaryAction) -> &'static str {
    match primary {
        PrimaryAction::Drink => "drink",
        PrimaryAction::Eat { .. } => "eat",
        PrimaryAction::Gather { .. } => "gather",
        PrimaryAction::Hunt => "hunt",
        PrimaryAction::Fish => "fish",
        PrimaryAction::Farm { .. } => "farm",
        PrimaryAction::Craft { .. } => "craft",
        PrimaryAction::Rest => "rest",
        PrimaryAction::MoveRelative { .. } => "move",
        PrimaryAction::Propose { .. } => "toxin_propose",
        PrimaryAction::Support { .. } => "toxin_support",
        PrimaryAction::Oppose { .. } => "wait",
        PrimaryAction::Wait => "wait",
        PrimaryAction::Transfer { .. } => "transfer",
        PrimaryAction::Store { .. } => "store",
        PrimaryAction::Retrieve { .. } => "retrieve",
        PrimaryAction::Pack { .. } => "pack",
        PrimaryAction::Unpack { .. } => "unpack",
    }
}

pub fn record(
    tick: u64,
    agent: AgentId,
    chooser: &str,
    policy_branch: &str,
    call_seed: u64,
    prompt_hash: String,
    retrieved_memory_ids: Vec<u64>,
    relation_ids: Vec<u64>,
    legal: &[PrimaryAction],
    choice: &ChosenAction,
    reasoning: Option<String>,
) -> DecisionRecord {
    DecisionRecord {
        tick,
        agent: agent.0,
        chooser: chooser.into(),
        policy_branch: policy_branch.into(),
        call_seed,
        prompt_hash,
        retrieved_memory_ids,
        relation_ids,
        legal: legal_names(legal),
        primary: primary_json(&choice.primary),
        speak: speak_json(choice),
        reasoning,
    }
}
