//! Decision source. No HTTP here — live clients live in `sim-llm`.

use crate::action::{ChosenAction, PrimaryAction, Recipe, Speak, SpeakTarget};
use crate::agent::{AgentId, ItemId};
use crate::observation::Observation;
use crate::species::SpeciesTables;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChooseError {
    Timeout,
    Malformed,
    Unreachable,
}

/// Sentinel stored when a live call becomes `LlmWait`. Replay re-emits the event.
pub const LLM_WAIT_SENTINEL: &str = r#"{"__llm_wait__":true}"#;

/// Sentinel for insight / plan / evict-reflect skip (not action-select `LlmWait`).
pub const LLM_SKIP_SENTINEL: &str = r#"{"__skip__":true}"#;

pub const REPLAY_CALL_CHOOSE: &str = "choose";
pub const REPLAY_CALL_REFLECT: &str = "reflect";
pub const REPLAY_CALL_PLAN: &str = "plan";
pub const REPLAY_CALL_REFLECT_EVICT: &str = "reflect_evict";
pub const REPLAY_CALL_IMPORTANCE: &str = "importance";

pub fn is_llm_wait_response(raw: &str) -> bool {
    extract_json_payload(raw).contains("__llm_wait__")
}

pub fn is_skip_response(raw: &str) -> bool {
    extract_json_payload(raw).contains("__skip__")
}

pub fn normalize_replay_call(call: &str) -> String {
    if call.is_empty() {
        REPLAY_CALL_CHOOSE.into()
    } else {
        call.to_string()
    }
}

pub fn insight_record_json(text: &str) -> String {
    serde_json::json!({ "reflection": text }).to_string()
}

pub fn plan_record_json(steps: &[String]) -> String {
    serde_json::json!({ "plan": steps }).to_string()
}

pub fn parse_insight_json(raw: &str) -> Option<String> {
    if is_skip_response(raw) {
        return None;
    }
    let p = extract_json_payload(raw);
    let v: serde_json::Value = serde_json::from_str(&p).ok()?;
    v.get("reflection")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn importance_record_json(id: u64, importance: u8) -> String {
    serde_json::json!({ "id": id, "importance": importance }).to_string()
}

pub fn parse_importance_json(raw: &str) -> Option<(u64, u8)> {
    if is_skip_response(raw) {
        return None;
    }
    let p = extract_json_payload(raw);
    let v: serde_json::Value = serde_json::from_str(&p).ok()?;
    let id = v.get("id")?.as_u64()?;
    let imp = v.get("importance")?.as_u64()?;
    if imp > 255 {
        return None;
    }
    Some((id, imp as u8))
}

pub fn parse_plan_json(raw: &str, max_len: usize) -> Option<Vec<String>> {
    if is_skip_response(raw) {
        return None;
    }
    let p = extract_json_payload(raw);
    let v: serde_json::Value = serde_json::from_str(&p).ok()?;
    let arr = v.get("plan")?.as_array()?;
    let steps: Vec<String> = arr
        .iter()
        .filter_map(|x| {
            if let Some(s) = x.as_str() {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            } else if x.is_object() {
                serde_json::to_string(x).ok()
            } else {
                None
            }
        })
        .take(max_len)
        .collect();
    if steps.is_empty() { None } else { Some(steps) }
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

    /// Summarise dropped memory texts. Default: skip (no extra call).
    fn reflect(&self, seed: u64, dropped: &[String]) -> Result<String, ChooseError> {
        let _ = (seed, dropped);
        Err(ChooseError::Malformed)
    }

    /// Periodic insight. Default: skip.
    fn insight(&self, seed: u64, obs: &Observation) -> Result<String, ChooseError> {
        let _ = (seed, obs);
        Err(ChooseError::Malformed)
    }

    /// Short-term plan (not executed). Default: skip.
    fn plan(
        &self,
        seed: u64,
        obs: &Observation,
        max_len: usize,
    ) -> Result<Vec<String>, ChooseError> {
        let _ = (seed, obs, max_len);
        Err(ChooseError::Malformed)
    }

    /// Rewrite one retrieved memory's importance. Default: skip.
    /// Return JSON `{"id": <memory id>, "importance": 0-255}`.
    fn importance(
        &self,
        seed: u64,
        retrieved: &[(u64, u8, String)],
    ) -> Result<String, ChooseError> {
        let _ = (seed, retrieved);
        Err(ChooseError::Malformed)
    }
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

/// Overlay `[llm] barrier` / `barrier_retries`. Not on `ExperimentConfig` (not hashed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmBarrierParams {
    pub barrier: bool,
    /// Extra attempts after the first. Default 3 when barrier is on.
    pub retries: u32,
    /// Overlay `[llm] reflect_on_evict`. Not hashed.
    pub reflect_on_evict: bool,
    /// Overlay `[llm] reflect_every_n_ticks`. 0 = off.
    pub reflect_every_n_ticks: u64,
    /// Overlay `[llm] plan_every_n_ticks`. 0 = off.
    pub plan_every_n_ticks: u64,
    /// Overlay `[llm] plan_length`. Default 4.
    pub plan_length: u32,
    /// Overlay `[llm] execute_plan`. Not hashed.
    pub execute_plan: bool,
    /// Overlay `[llm] reflect_importance`. Not hashed.
    pub reflect_importance: bool,
}

impl Default for LlmBarrierParams {
    fn default() -> Self {
        Self {
            barrier: false,
            retries: 3,
            reflect_on_evict: false,
            reflect_every_n_ticks: 0,
            plan_every_n_ticks: 0,
            plan_length: 4,
            execute_plan: false,
            reflect_importance: false,
        }
    }
}

impl LlmBarrierParams {
    /// Read `[llm] barrier` and `barrier_retries` from an experiment TOML.
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            llm: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            barrier: Option<bool>,
            barrier_retries: Option<u32>,
            reflect_on_evict: Option<bool>,
            reflect_every_n_ticks: Option<u64>,
            plan_every_n_ticks: Option<u64>,
            plan_length: Option<u32>,
            execute_plan: Option<bool>,
            reflect_importance: Option<bool>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        let mut p = Self::default();
        if let Some(b) = slice.llm.barrier {
            p.barrier = b;
        }
        if let Some(n) = slice.llm.barrier_retries {
            p.retries = n;
        }
        if let Some(r) = slice.llm.reflect_on_evict {
            p.reflect_on_evict = r;
        }
        if let Some(n) = slice.llm.reflect_every_n_ticks {
            p.reflect_every_n_ticks = n;
        }
        if let Some(n) = slice.llm.plan_every_n_ticks {
            p.plan_every_n_ticks = n;
        }
        if let Some(n) = slice.llm.plan_length {
            p.plan_length = n;
        }
        if let Some(e) = slice.llm.execute_plan {
            p.execute_plan = e;
        }
        if let Some(r) = slice.llm.reflect_importance {
            p.reflect_importance = r;
        }
        p
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayRecord {
    pub tick: u64,
    pub agent: u64,
    pub call_seed: u64,
    pub prompt_hash: String,
    pub response: String,
    /// `""` / omitted = choose (old JSONL). Also `choose` | `reflect` | `plan` | `reflect_evict` | `importance`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub call: String,
}

#[derive(Clone, Debug, Default)]
pub struct ReplayTable {
    /// (tick, agent, normalized call) → raw JSON response
    pub by_tick_agent_call: BTreeMap<(u64, u64, String), String>,
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
                table.by_tick_agent_call.insert(
                    (rec.tick, rec.agent, normalize_replay_call(&rec.call)),
                    rec.response,
                );
            }
        }
        table
    }

    pub fn get(&self, tick: u64, agent: u64) -> Option<&str> {
        self.get_call(tick, agent, REPLAY_CALL_CHOOSE)
    }

    pub fn get_call(&self, tick: u64, agent: u64, call: &str) -> Option<&str> {
        self.by_tick_agent_call
            .get(&(tick, agent, normalize_replay_call(call)))
            .map(String::as_str)
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
    #[serde(default)]
    ticks: Option<u64>,
    #[serde(default)]
    milli: Option<u32>,
    #[serde(default)]
    weight: Option<String>,
    #[serde(default)]
    accept: Option<String>,
    #[serde(default)]
    council: Option<Vec<u64>>,
    #[serde(default)]
    tally: Option<String>,
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
            let recipe = if let Some(rest) = recipe.strip_prefix("catalog:") {
                rest.parse::<u16>()
                    .map(Recipe::Catalog)
                    .unwrap_or(Recipe::Spear)
            } else {
                match recipe {
                    "basket" => Recipe::Basket,
                    "backpack" => Recipe::Backpack,
                    "fishing_rod" | "rod" => Recipe::FishingRod,
                    _ => Recipe::Spear,
                }
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
        "attack" => {
            let to = parsed
                .target
                .as_ref()
                .and_then(|v| v.as_u64())
                .or(parsed.proposal_id)
                .unwrap_or(0);
            PrimaryAction::Attack {
                target: AgentId(to),
            }
        }
        "flee" => PrimaryAction::Flee,
        "pair_bond" | "pairbond" => {
            let to = parsed
                .target
                .as_ref()
                .and_then(|v| v.as_u64())
                .or(parsed.proposal_id)
                .unwrap_or(0);
            PrimaryAction::PairBond {
                target: AgentId(to),
            }
        }
        "reproduce" => {
            let to = parsed
                .target
                .as_ref()
                .and_then(|v| v.as_u64())
                .or(parsed.proposal_id)
                .unwrap_or(0);
            PrimaryAction::Reproduce { with: AgentId(to) }
        }
        "invent" => PrimaryAction::Invent,
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

/// Plan-step parse: unknown / illegal / missing action is `None` (do not Wait-substitute).
pub fn try_parse_plan_step(
    raw: &str,
    legal: &[PrimaryAction],
    species: &SpeciesTables,
) -> Option<ChosenAction> {
    let trimmed = extract_json_payload(raw);
    let parsed: LlmJson = serde_json::from_str(&trimmed).ok()?;
    let name = parsed.action.as_deref()?.trim().to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    let choice = parse_choice_json(raw, legal, species).ok()?;
    let explicit_wait = name == "wait";
    if matches!(choice.primary, PrimaryAction::Wait) && !explicit_wait {
        return None;
    }
    if !crate::observation::is_legal_choice(legal, &choice.primary) {
        return None;
    }
    Some(choice)
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
        "setproposallifetime" | "set_lifetime" => {
            Some(crate::board::StructuredRule::SetProposalLifetime {
                ticks: r.ticks.or_else(|| r.n.map(u64::from)).unwrap_or(0),
            })
        }
        "setacceptancethreshold" | "set_threshold" => {
            let milli = r.milli.or(r.n).unwrap_or(5000).clamp(100, 10_000);
            Some(crate::board::StructuredRule::SetAcceptanceThreshold { milli })
        }
        "setvoteweight" | "set_vote_weight" => {
            let w = crate::voting::VoteWeight::parse(r.weight.as_deref().unwrap_or("")).ok()?;
            Some(crate::board::StructuredRule::SetVoteWeight { weight: w })
        }
        "setvoteaccept" | "set_vote_accept" => {
            let a = crate::voting::VoteAccept::parse(r.accept.as_deref().unwrap_or("")).ok()?;
            Some(crate::board::StructuredRule::SetVoteAccept { accept: a })
        }
        "setcouncil" | "set_council" => {
            let raw = r.council.as_ref()?;
            let mut seen = BTreeSet::new();
            let mut ids = Vec::new();
            for n in raw {
                let id = AgentId(*n);
                if seen.insert(id) {
                    ids.push(id);
                }
            }
            if ids.is_empty() {
                return None;
            }
            Some(crate::board::StructuredRule::SetCouncil { ids })
        }
        "setcounciltally" | "set_council_tally" => {
            let raw = r.tally.as_deref()?.trim();
            if raw.is_empty() {
                return None;
            }
            let t = crate::voting::CouncilTally::parse(raw).ok()?;
            Some(crate::board::StructuredRule::SetCouncilTally { tally: t })
        }
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
    if let Some(rest) = s.strip_prefix("catalog:") {
        let n: u16 = rest.parse().ok()?;
        return Some(ItemId::Catalog(n));
    }
    match s {
        "wood" => Some(ItemId::Wood),
        "fiber" => Some(ItemId::Fiber),
        "stone" => Some(ItemId::Stone),
        "basket" => Some(ItemId::Basket),
        "backpack" => Some(ItemId::Backpack),
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
