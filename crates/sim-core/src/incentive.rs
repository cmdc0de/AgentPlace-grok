//! Declarative incentive schedules. Overlay TOML, not `ExperimentConfig`.

use crate::agent::AgentId;
use crate::board::Goal;
use crate::error::SimError;
use crate::event_log::{SimEvent, SimEventKind};
use crate::memory::MemoryKind;
use crate::simulation::Simulation;
use crate::social::{REL_MAX, REL_MIN};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// System-level events use this agent id (not a real spawn).
pub const SYSTEM_AGENT: AgentId = AgentId(u64::MAX);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IncentiveSchedule {
    #[serde(default)]
    pub incentives: Vec<Incentive>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncentiveVisibility {
    #[default]
    Public,
    Hidden,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Incentive {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub start_tick: u64,
    #[serde(default)]
    pub end_tick: Option<u64>,
    #[serde(default = "default_all")]
    pub applies_to: String,
    /// Who is *told*. Effects still apply when `hidden`.
    #[serde(default)]
    pub visibility: IncentiveVisibility,
    #[serde(default)]
    pub effects: Vec<EffectSpec>,
}

impl Incentive {
    pub fn is_hidden(&self) -> bool {
        self.visibility == IncentiveVisibility::Hidden
    }
}

fn default_all() -> String {
    "all".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EffectSpec {
    ResourceMultiplier {
        resource: String,
        multiplier: f64,
        #[serde(default)]
        condition: String,
    },
    InfluenceFactorDelta {
        delta: f64,
    },
    MemoryImportanceBoost {
        kind: String,
        multiplier: f64,
    },
    GoalInjection {
        goal_text: String,
        #[serde(default = "default_personal")]
        scope: String,
        #[serde(default = "default_priority")]
        priority: f64,
    },
    ProposalThresholdModifier {
        delta: f64,
    },
    RelationshipDelta {
        #[serde(default)]
        trust: f64,
        #[serde(default)]
        affinity: f64,
        #[serde(default)]
        respect: f64,
        /// Empty = every other agent. `agent:N` = that id only.
        #[serde(default)]
        toward: String,
    },
}

fn default_personal() -> String {
    "personal".into()
}

fn default_priority() -> f64 {
    0.5
}

impl IncentiveSchedule {
    pub fn from_toml_str(s: &str) -> Result<Self, SimError> {
        let sched: Self = toml::from_str(s).map_err(|e| SimError::Config(e.to_string()))?;
        sched.validate()?;
        Ok(sched)
    }

    pub fn load_path(path: impl AsRef<Path>) -> Result<(Self, String), SimError> {
        let text = std::fs::read_to_string(path)?;
        Ok((Self::from_toml_str(&text)?, text))
    }

    fn validate(&self) -> Result<(), SimError> {
        let mut seen = BTreeSet::new();
        for inc in &self.incentives {
            if inc.id.is_empty() {
                return Err(SimError::Config("incentive id must be non-empty".into()));
            }
            if !seen.insert(inc.id.clone()) {
                return Err(SimError::Config(format!(
                    "duplicate incentive id {}",
                    inc.id
                )));
            }
            if !valid_scope(&inc.applies_to) {
                return Err(SimError::Config(format!(
                    "unsupported applies_to {:?} (use all, agent:N, archetype:name, or supporters_of:proposal_N)",
                    inc.applies_to
                )));
            }
            for e in &inc.effects {
                if let EffectSpec::RelationshipDelta { toward, .. } = e {
                    parse_toward(toward)?;
                }
            }
        }
        Ok(())
    }

    pub fn is_active(&self, id: &str, tick: u64) -> bool {
        self.incentives
            .iter()
            .any(|i| i.id == id && window(i, tick))
    }
}

fn window(inc: &Incentive, tick: u64) -> bool {
    tick >= inc.start_tick && inc.end_tick.map(|e| tick <= e).unwrap_or(true)
}

/// Empty → all others. `agent:N` → that id. Anything else is a load error.
fn parse_toward(s: &str) -> Result<Option<AgentId>, SimError> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    if let Some(rest) = s.strip_prefix("agent:") {
        let n: u64 = rest
            .parse()
            .map_err(|_| SimError::Config(format!("unsupported toward {s:?} (use agent:N)")))?;
        return Ok(Some(AgentId(n)));
    }
    Err(SimError::Config(format!(
        "unsupported toward {s:?} (use agent:N)"
    )))
}

fn valid_scope(s: &str) -> bool {
    s == "all"
        || s.starts_with("agent:")
        || s.starts_with("archetype:")
        || parse_supporters_of(s).is_some()
}

/// `supporters_of:proposal_3` or `supporters_of:3` → Some(3).
fn parse_supporters_of(s: &str) -> Option<u64> {
    let rest = s.strip_prefix("supporters_of:")?;
    let rest = rest.strip_prefix("proposal_").unwrap_or(rest);
    rest.parse().ok()
}

pub fn in_scope(sim: &Simulation, inc: &Incentive, id: AgentId) -> bool {
    let s = inc.applies_to.trim();
    if s == "all" {
        return true;
    }
    if let Some(rest) = s.strip_prefix("agent:") {
        return rest.parse::<u64>().ok() == Some(id.0);
    }
    if let Some(name) = s.strip_prefix("archetype:") {
        let Some(agent) = sim.agents.get(&id) else {
            return false;
        };
        return sim.config.agents.archetypes.iter().any(|a| {
            a.name == name
                && a.abilities == agent.abilities
                && a.personality.agreeableness == agent.personality.agreeableness
                && a.personality.perceptiveness == agent.personality.perceptiveness
        });
    }
    if let Some(pid) = parse_supporters_of(s) {
        return sim
            .board
            .proposals
            .iter()
            .any(|p| p.id == pid && p.supporters.contains(&id));
    }
    false
}

fn scoped_ids(sim: &Simulation, inc: &Incentive) -> Vec<AgentId> {
    sim.agents
        .keys()
        .copied()
        .filter(|&id| in_scope(sim, inc, id))
        .collect()
}

pub fn proposal_threshold(sim: &Simulation) -> f64 {
    let mut th = sim.config.proposals.default_acceptance_threshold;
    for inc in &sim.incentives.incentives {
        if !sim.incentive_active.contains(&inc.id) {
            continue;
        }
        for e in &inc.effects {
            if let EffectSpec::ProposalThresholdModifier { delta } = e {
                th += *delta;
            }
        }
    }
    th.clamp(0.01, 1.0)
}

pub fn resource_mult_milli(sim: &Simulation, id: AgentId, resource: &str) -> u32 {
    let mut milli = 1000u32;
    for inc in &sim.incentives.incentives {
        if !sim.incentive_active.contains(&inc.id) || !in_scope(sim, inc, id) {
            continue;
        }
        for e in &inc.effects {
            if let EffectSpec::ResourceMultiplier {
                resource: res,
                multiplier,
                condition,
            } = e
            {
                if res != resource {
                    continue;
                }
                if !condition_ok(sim, id, condition) {
                    continue;
                }
                let m = (*multiplier * 1000.0).round().clamp(0.0, 10_000.0) as u32;
                milli = milli.saturating_mul(m) / 1000;
            }
        }
    }
    milli.max(1)
}

fn condition_ok(sim: &Simulation, id: AgentId, condition: &str) -> bool {
    let c = condition.trim();
    if c.is_empty() {
        return true;
    }
    if c == "has_supported_public_goal" {
        return sim
            .board
            .proposals
            .iter()
            .any(|p| p.supporters.contains(&id));
    }
    true
}

pub fn memory_boost_milli(sim: &Simulation, id: AgentId, kind: MemoryKind) -> u32 {
    let mut milli = 1000u32;
    let kind_name = format!("{kind:?}").to_ascii_lowercase();
    for inc in &sim.incentives.incentives {
        if !sim.incentive_active.contains(&inc.id) || !in_scope(sim, inc, id) {
            continue;
        }
        for e in &inc.effects {
            if let EffectSpec::MemoryImportanceBoost {
                kind: k,
                multiplier,
            } = e
            {
                if k.to_ascii_lowercase() != kind_name {
                    continue;
                }
                let m = (*multiplier * 1000.0).round().clamp(0.0, 10_000.0) as u32;
                milli = milli.saturating_mul(m) / 1000;
            }
        }
    }
    milli.max(1)
}

pub fn scale_u32(value: u32, milli: u32) -> u32 {
    ((u64::from(value) * u64::from(milli)) / 1000) as u32
}

pub fn sync(sim: &mut Simulation) {
    let tick = sim.tick;
    let list = sim.incentives.incentives.clone();
    let mut desired = BTreeSet::new();
    for inc in &list {
        if window(inc, tick) {
            desired.insert(inc.id.clone());
        }
    }
    let current: Vec<String> = sim.incentive_active.iter().cloned().collect();
    for id in &current {
        if !desired.contains(id) {
            end_incentive(sim, id);
        }
    }
    for inc in &list {
        if desired.contains(&inc.id) && !sim.incentive_active.contains(&inc.id) {
            start_incentive(sim, inc);
        }
    }
}

fn start_incentive(sim: &mut Simulation, inc: &Incentive) {
    sim.incentive_active.insert(inc.id.clone());
    sim.events.push(SimEvent {
        tick: sim.tick,
        agent: SYSTEM_AGENT,
        kind: SimEventKind::IncentiveApplied {
            id: inc.id.clone(),
            detail: inc.description.clone(),
        },
    });
    let ids = scoped_ids(sim, inc);
    for e in &inc.effects {
        match e {
            EffectSpec::InfluenceFactorDelta { delta } => {
                let d = (*delta * 100.0).round() as i32;
                for id in &ids {
                    if let Some(a) = sim.agents.get_mut(id) {
                        let v = a.influence_factor as i32 + d;
                        a.influence_factor = v.clamp(0, 10_000) as u32;
                    }
                }
            }
            EffectSpec::GoalInjection {
                goal_text,
                scope,
                priority,
            } => {
                let prio = (*priority * 100.0).round().clamp(0.0, 100.0) as u8;
                let public = scope.eq_ignore_ascii_case("public");
                let cap = sim.config.agents.goals.max_personal_goals as usize;
                let targets: Vec<AgentId> = if public {
                    sim.agents.keys().copied().collect()
                } else {
                    ids.clone()
                };
                for id in targets {
                    let Some(a) = sim.agents.get_mut(&id) else {
                        continue;
                    };
                    if a.goals.iter().any(|g| g.text == *goal_text) {
                        if let Some(g) = a.goals.iter_mut().find(|g| g.text == *goal_text) {
                            g.priority = g.priority.max(prio);
                        }
                        continue;
                    }
                    let next_id = a
                        .goals
                        .iter()
                        .map(|g| g.id)
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1);
                    if a.goals.len() >= cap && cap > 0 {
                        if let Some(i) = a
                            .goals
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, g)| g.priority)
                            .map(|(i, _)| i)
                        {
                            a.goals.remove(i);
                        }
                    }
                    a.goals.push(Goal {
                        id: next_id,
                        text: goal_text.clone(),
                        priority: prio,
                        source: format!("incentive:{}", inc.id),
                    });
                }
            }
            EffectSpec::RelationshipDelta {
                trust,
                affinity,
                respect,
                toward,
            } => {
                let dt = (*trust * 100.0).round() as i16;
                let da = (*affinity * 100.0).round() as i16;
                let dr = (*respect * 100.0).round() as i16;
                let toward_id = parse_toward(toward).ok().flatten();
                let others: Vec<AgentId> = if let Some(t) = toward_id {
                    if sim.agents.contains_key(&t) {
                        vec![t]
                    } else {
                        Vec::new()
                    }
                } else {
                    sim.agents.keys().copied().collect()
                };
                for id in &ids {
                    for other in &others {
                        if other == id {
                            continue;
                        }
                        if let Some(a) = sim.agents.get_mut(id) {
                            let row = a.relationships.entry(*other).or_default();
                            row.trust = row.trust.saturating_add(dt).clamp(REL_MIN, REL_MAX);
                            row.affinity = row.affinity.saturating_add(da).clamp(REL_MIN, REL_MAX);
                            row.respect = row.respect.saturating_add(dr).clamp(REL_MIN, REL_MAX);
                            row.last_interaction_tick = sim.tick;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn end_incentive(sim: &mut Simulation, id: &str) {
    let inc = sim
        .incentives
        .incentives
        .iter()
        .find(|i| i.id == id)
        .cloned();
    sim.incentive_active.remove(id);
    sim.events.push(SimEvent {
        tick: sim.tick,
        agent: SYSTEM_AGENT,
        kind: SimEventKind::IncentiveEnded { id: id.to_string() },
    });
    let Some(inc) = inc else {
        return;
    };
    let ids = scoped_ids(sim, &inc);
    for e in &inc.effects {
        if let EffectSpec::InfluenceFactorDelta { delta } = e {
            let d = (*delta * 100.0).round() as i32;
            for aid in &ids {
                if let Some(a) = sim.agents.get_mut(aid) {
                    let v = a.influence_factor as i32 - d;
                    a.influence_factor = v.clamp(0, 10_000) as u32;
                }
            }
        }
    }
}

pub fn reconstruct_active(events: &[SimEvent]) -> BTreeSet<String> {
    let mut active = BTreeSet::new();
    for e in events {
        match &e.kind {
            SimEventKind::IncentiveApplied { id, .. } => {
                active.insert(id.clone());
            }
            SimEventKind::IncentiveEnded { id } => {
                active.remove(id);
            }
            _ => {}
        }
    }
    active
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_effect_type_errors() {
        let toml = r#"
[[incentives]]
id = "x"
[[incentives.effects]]
type = "nope_not_real"
delta = 1.0
"#;
        let err = IncentiveSchedule::from_toml_str(toml).unwrap_err();
        assert!(err.to_string().contains("config"), "{err}");
    }

    #[test]
    fn parses_closed_vocabulary() {
        let toml = r#"
[[incentives]]
id = "early_cooperation_bonus"
start_tick = 0
applies_to = "all"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
condition = "has_supported_public_goal"
[[incentives.effects]]
type = "proposal_threshold_modifier"
delta = -0.1
"#;
        let s = IncentiveSchedule::from_toml_str(toml).unwrap();
        assert_eq!(s.incentives.len(), 1);
        assert_eq!(s.incentives[0].effects.len(), 2);
        assert_eq!(s.incentives[0].visibility, IncentiveVisibility::Public);
    }

    #[test]
    fn unknown_visibility_errors() {
        let err = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "x"
visibility = "maybe"
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("config"), "{err}");
    }

    #[test]
    fn toward_nope_is_load_error() {
        let err = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "e"
[[incentives.effects]]
type = "relationship_delta"
respect = 70.0
toward = "nope"
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("toward"), "{err}");
    }

    #[test]
    fn parses_relationship_delta_respect_toward() {
        let s = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "e"
[[incentives.effects]]
type = "relationship_delta"
respect = 70.0
toward = "agent:0"
"#,
        )
        .unwrap();
        match &s.incentives[0].effects[0] {
            EffectSpec::RelationshipDelta {
                respect, toward, ..
            } => {
                assert!((*respect - 70.0).abs() < 1e-9);
                assert_eq!(toward, "agent:0");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parses_supporters_of_scope() {
        let s = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "c"
applies_to = "supporters_of:proposal_0"
"#,
        )
        .unwrap();
        assert_eq!(s.incentives[0].applies_to, "supporters_of:proposal_0");
        let s2 = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "c"
applies_to = "supporters_of:3"
"#,
        )
        .unwrap();
        assert_eq!(s2.incentives[0].applies_to, "supporters_of:3");
    }

    #[test]
    fn supporters_of_nope_is_load_error() {
        let err = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "c"
applies_to = "supporters_of:nope"
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("applies_to"), "{err}");
    }

    #[test]
    fn parses_hidden_visibility() {
        let s = IncentiveSchedule::from_toml_str(
            r#"
[[incentives]]
id = "secret"
visibility = "hidden"
"#,
        )
        .unwrap();
        assert!(s.incentives[0].is_hidden());
    }
}
