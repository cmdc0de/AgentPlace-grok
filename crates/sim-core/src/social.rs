//! Relationship summaries and canned millipoint deltas.

use crate::agent::AgentId;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

pub const REL_MIN: i16 = -10_000;
pub const REL_MAX: i16 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub trust: i16,
    pub affinity: i16,
    pub respect: i16,
    pub fear: i16,
    pub interaction_count: u32,
    pub last_interaction_tick: u64,
    #[serde(default)]
    pub notable: Vec<u64>,
}

impl Default for RelationshipSummary {
    fn default() -> Self {
        Self {
            trust: 0,
            affinity: 0,
            respect: 0,
            fear: 0,
            interaction_count: 0,
            last_interaction_tick: 0,
            notable: Vec::new(),
        }
    }
}

impl RelationshipSummary {
    pub fn hash_into(&self, hasher: &mut impl Digest) {
        hasher.update(self.trust.to_le_bytes());
        hasher.update(self.affinity.to_le_bytes());
        hasher.update(self.respect.to_le_bytes());
        hasher.update(self.fear.to_le_bytes());
        hasher.update(self.interaction_count.to_le_bytes());
        hasher.update(self.last_interaction_tick.to_le_bytes());
        for id in &self.notable {
            hasher.update(id.to_le_bytes());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationView {
    pub id: AgentId,
    pub trust: i16,
    pub affinity: i16,
    pub respect: i16,
    pub fear: i16,
}

pub fn clamp_rel(v: i32) -> i16 {
    v.clamp(i32::from(REL_MIN), i32::from(REL_MAX)) as i16
}

pub fn decay_toward_zero(v: i16, step: u32) -> i16 {
    if step == 0 {
        return v;
    }
    let step = step.min(i16::MAX as u32) as i16;
    if v > 0 {
        v.saturating_sub(step).max(0)
    } else if v < 0 {
        v.saturating_add(step).min(0)
    } else {
        0
    }
}

pub fn apply_delta(
    map: &mut BTreeMap<AgentId, RelationshipSummary>,
    other: AgentId,
    tick: u64,
    d_trust: i16,
    d_affinity: i16,
    d_respect: i16,
    d_fear: i16,
    notable: Option<u64>,
) {
    let row = map.entry(other).or_default();
    row.trust = clamp_rel(i32::from(row.trust) + i32::from(d_trust));
    row.affinity = clamp_rel(i32::from(row.affinity) + i32::from(d_affinity));
    row.respect = clamp_rel(i32::from(row.respect) + i32::from(d_respect));
    row.fear = clamp_rel(i32::from(row.fear) + i32::from(d_fear));
    row.interaction_count = row.interaction_count.saturating_add(1);
    row.last_interaction_tick = tick;
    if let Some(id) = notable {
        if !row.notable.contains(&id) {
            row.notable.push(id);
            if row.notable.len() > 3 {
                row.notable.remove(0);
            }
        }
    }
}

pub fn decay_map(map: &mut BTreeMap<AgentId, RelationshipSummary>, step: u32) {
    for row in map.values_mut() {
        row.trust = decay_toward_zero(row.trust, step);
        row.affinity = decay_toward_zero(row.affinity, step);
        row.respect = decay_toward_zero(row.respect, step);
        row.fear = decay_toward_zero(row.fear, step);
    }
}

/// Canned deltas: (trust, affinity, respect, fear) actor→partner.
pub const SPEAK: (i16, i16, i16, i16) = (0, 50, 0, 0);
pub const SPEAK_BACK: (i16, i16, i16, i16) = (0, 50, 0, 0);
pub const SHOUT: (i16, i16, i16, i16) = (0, 20, 0, 30);
pub const SHOUT_BACK: (i16, i16, i16, i16) = (0, 0, 0, 30);
pub const SUPPORT: (i16, i16, i16, i16) = (200, 0, 100, 0);
pub const SUPPORT_BACK: (i16, i16, i16, i16) = (0, 50, 0, 0);
pub const OPPOSE: (i16, i16, i16, i16) = (-150, -50, 0, 0);
pub const OPPOSE_BACK: (i16, i16, i16, i16) = (0, -80, 0, 0);
pub const TOXIN_CONFIRM: (i16, i16, i16, i16) = (300, 0, 0, 0);
