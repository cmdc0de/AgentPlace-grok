//! Decision source. No HTTP here — live clients live in `sim-llm`.

use crate::action::{ChosenAction, PrimaryAction, Recipe, Speak, SpeakTarget};
use crate::agent::{AgentId, ItemId};
use crate::observation::Observation;
use crate::species::SpeciesTables;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChooseError {
    Timeout,
    Malformed,
    Unreachable,
}

/// Sentinel stored when a live call becomes `LlmWait`. Replay re-emits the event.
pub const LLM_WAIT_SENTINEL: &str = r#"{"__llm_wait__":true}"#;

pub fn is_llm_wait_response(raw: &str) -> bool {
    extract_json_payload(raw).contains("__llm_wait__")
}

/// Pull a JSON object out of fences, `<think>` wrappers, or leading prose.
pub fn extract_json_payload(s: &str) -> String {
    let mut t = s.to_string();
    loop {
        let lower = t.to_ascii_lowercase();
        let Some(start) = lower.find("<think>") else {
            break;
        };
        let after = start + "<think>".len();
        let rest_lower = lower[after..].to_string();
        if let Some(end_rel) = rest_lower.find("</think>") {
            t.replace_range(start..after + end_rel + "</think>".len(), " ");
        } else {
            t.replace_range(start..after, " ");
            break;
        }
    }
    let trimmed = strip_fences(t.trim()).to_string();
    if let Some(i) = trimmed.find('{') {
        if let Some(j) = trimmed.rfind('}') {
            if j >= i {
                return trimmed[i..=j].to_string();
            }
        }
    }
    trimmed
}

/// Pluggable chooser used by live / replay backends. Mock stays inside `Simulation`
/// so it can use the agent RNG stream (required for bit-identical hashes).
pub trait ActionChooser: Send + Sync {
    /// Returns the chosen action and the **raw** model text (for replay JSONL).
    fn choose(
        &self,
        call_seed: u64,
        obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError>;
}

#[derive(Clone)]
pub enum Chooser {
    Mock,
    Wait,
    Custom(Arc<dyn ActionChooser>),
}

impl std::fmt::Debug for Chooser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chooser::Mock => write!(f, "Mock"),
            Chooser::Wait => write!(f, "Wait"),
            Chooser::Custom(_) => write!(f, "Custom"),
        }
    }
}

impl Default for Chooser {
    fn default() -> Self {
        Chooser::Mock
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayRecord {
    pub tick: u64,
    pub agent: u64,
    pub call_seed: u64,
    pub prompt_hash: String,
    pub response: String,
}

#[derive(Clone, Debug, Default)]
pub struct ReplayTable {
    /// (tick, agent) → raw JSON response
    pub by_tick_agent: BTreeMap<(u64, u64), String>,
}

impl ReplayTable {
    pub fn from_jsonl(text: &str) -> Self {
        let mut table = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(rec) = serde_json::from_str::<ReplayRecord>(line) {
                table
                    .by_tick_agent
                    .insert((rec.tick, rec.agent), rec.response);
            }
        }
        table
    }

    pub fn get(&self, tick: u64, agent: u64) -> Option<&str> {
        self.by_tick_agent.get(&(tick, agent)).map(String::as_str)
    }
}

pub fn prompt_hash(obs: &Observation) -> String {
    let bytes = crate::observation::encode_for_hash(obs);
    hex::encode(Sha256::digest(&bytes))
}

#[derive(Debug, Deserialize)]
struct LlmJson {
    action: Option<String>,
    #[serde(default)]
    target: Option<serde_json::Value>,
    #[serde(default)]
    dx: Option<i32>,
    #[serde(default)]
    dy: Option<i32>,
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    recipe: Option<String>,
    #[serde(default)]
    speak: Option<SpeakJson>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    proposal_id: Option<u64>,
    #[serde(default)]
    rule: Option<RuleJson>,
    #[serde(default)]
    qty: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RuleJson {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    species: Option<serde_json::Value>,
    #[serde(default)]
    n: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SpeakJson {
    #[serde(default)]
    to: serde_json::Value,
    #[serde(default)]
    shout: bool,
    #[serde(default)]
    text: String,
}

/// Parse an LLM/replay JSON object into a legal `ChosenAction`.
/// Unknown or illegal primaries become `Wait`. Speech is kept even if primary waits,
/// unless the caller strips it on timeout.
pub fn parse_choice_json(
    raw: &str,
    legal: &[PrimaryAction],
    species: &SpeciesTables,
) -> Result<ChosenAction, ChooseError> {
    let trimmed = extract_json_payload(raw);
    if trimmed.contains("__llm_wait__") {
        return Ok(ChosenAction::wait());
    }
    let parsed: LlmJson = serde_json::from_str(&trimmed).map_err(|_| ChooseError::Malformed)?;
    let name = parsed.action.unwrap_or_else(|| "Wait".into());
    let primary = match name.to_ascii_lowercase().as_str() {
        "wait" => PrimaryAction::Wait,
        "rest" => PrimaryAction::Rest,
        "drink" => PrimaryAction::Drink,
        "hunt" => PrimaryAction::Hunt,
        "fish" => PrimaryAction::Fish,
        "moverelative" | "move" => PrimaryAction::MoveRelative {
            dx: parsed.dx.unwrap_or(0),
            dy: parsed.dy.unwrap_or(0),
        },
        "gather" => {
            let tag = resolve_species_target(parsed.target.as_ref(), species).unwrap_or(0);
            PrimaryAction::Gather { species: tag }
        }
        "farm" => {
            let tag = resolve_species_target(parsed.target.as_ref(), species).unwrap_or(1);
            PrimaryAction::Farm { species: tag }
        }
        "eat" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            PrimaryAction::Eat { item }
        }
        "craft" => {
            let recipe = parsed
                .recipe
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .unwrap_or("spear");
            let recipe = match recipe {
                "basket" => Recipe::Basket,
                "fishing_rod" | "rod" => Recipe::FishingRod,
                _ => Recipe::Spear,
            };
            PrimaryAction::Craft { recipe }
        }
        "propose" => match &parsed.rule {
            Some(r) => match parse_rule(r, species) {
                Some(rule) => PrimaryAction::Propose {
                    text: parsed.text.clone().unwrap_or_default(),
                    rule: Some(rule),
                },
                None => PrimaryAction::Wait,
            },
            None => PrimaryAction::Propose {
                text: parsed.text.clone().unwrap_or_default(),
                rule: None,
            },
        },
        "support" => PrimaryAction::Support {
            proposal_id: parsed.proposal_id.unwrap_or(0),
        },
        "oppose" => PrimaryAction::Oppose {
            proposal_id: parsed.proposal_id.unwrap_or(0),
        },
        "transfer" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            let to = parsed
                .target
                .as_ref()
                .and_then(|v| v.as_u64())
                .or(parsed.proposal_id)
                .unwrap_or(0);
            PrimaryAction::Transfer {
                item,
                qty: parsed.qty.unwrap_or(1).max(1),
                to: AgentId(to),
            }
        }
        "store" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            PrimaryAction::Store {
                item,
                qty: parsed.qty.unwrap_or(1).max(1),
            }
        }
        "retrieve" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            PrimaryAction::Retrieve {
                item,
                qty: parsed.qty.unwrap_or(1).max(1),
            }
        }
        "pack" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            PrimaryAction::Pack {
                item,
                qty: parsed.qty.unwrap_or(1).max(1),
            }
        }
        "unpack" => {
            let item = parsed
                .item
                .as_deref()
                .or_else(|| parsed.target.as_ref().and_then(|v| v.as_str()))
                .and_then(|s| parse_item(s, species))
                .unwrap_or(ItemId::Food(1));
            PrimaryAction::Unpack {
                item,
                qty: parsed.qty.unwrap_or(1).max(1),
            }
        }
        _ => PrimaryAction::Wait,
    };
    let primary = if crate::observation::is_legal_choice(legal, &primary) {
        primary
    } else {
        PrimaryAction::Wait
    };
    let speak = parsed.speak.and_then(|s| {
        if s.text.is_empty() {
            return None;
        }
        let to = if s.to.as_str() == Some("broadcast") || s.to.is_null() {
            SpeakTarget::Broadcast
        } else if let Some(arr) = s.to.as_array() {
            let ids: Vec<AgentId> = arr.iter().filter_map(|v| v.as_u64().map(AgentId)).collect();
            SpeakTarget::Directed(ids)
        } else if let Some(n) = s.to.as_u64() {
            SpeakTarget::Directed(vec![AgentId(n)])
        } else {
            SpeakTarget::Broadcast
        };
        Some(Speak {
            to,
            shout: s.shout,
            text: s.text,
        })
    });
    Ok(ChosenAction { primary, speak })
}

fn parse_rule(r: &RuleJson, species: &SpeciesTables) -> Option<crate::board::StructuredRule> {
    let kind = r.kind.as_deref()?.to_ascii_lowercase();
    match kind.as_str() {
        "baneatspecies" | "ban_eat" => {
            let tag = resolve_species_target(r.species.as_ref(), species)?;
            Some(crate::board::StructuredRule::BanEatSpecies { species: tag })
        }
        "bangatherspecies" | "ban_gather" => {
            let tag = resolve_species_target(r.species.as_ref(), species)?;
            Some(crate::board::StructuredRule::BanGatherSpecies { species: tag })
        }
        "maxgatherpertick" | "max_gather" => Some(crate::board::StructuredRule::MaxGatherPerTick {
            n: r.n.unwrap_or(1),
        }),
        _ => None,
    }
}

fn resolve_species_target(v: Option<&serde_json::Value>, species: &SpeciesTables) -> Option<u8> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(n as u8);
    }
    if let Some(s) = v.as_str() {
        if s == "stone" || s == "mineral" {
            return Some(0);
        }
        return species.veg_tag_by_id(s);
    }
    None
}

pub fn parse_item(s: &str, species: &SpeciesTables) -> Option<ItemId> {
    if let Some(rest) = s.strip_prefix("food:") {
        let tag: u8 = rest.parse().ok()?;
        return Some(ItemId::Food(tag));
    }
    match s {
        "wood" => Some(ItemId::Wood),
        "fiber" => Some(ItemId::Fiber),
        "stone" => Some(ItemId::Stone),
        "basket" => Some(ItemId::Basket),
        "spear" => Some(ItemId::Spear),
        "fishing_rod" => Some(ItemId::FishingRod),
        "hare" | "meat" => Some(ItemId::Food(100)),
        "perch" | "fish" => Some(ItemId::Food(101)),
        other => species.veg_tag_by_id(other).map(ItemId::Food),
    }
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        return rest.trim().trim_end_matches("```").trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.trim().trim_end_matches("```").trim();
    }
    s
}

pub fn chosen_to_json(choice: &ChosenAction) -> String {
    serde_json::to_string(choice).unwrap_or_else(|_| "{\"action\":\"Wait\"}".into())
}
