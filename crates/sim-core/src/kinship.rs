//! Kinship links and overlay `[population]`. Not `RelationshipSummary`.

use crate::agent::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kinship {
    #[serde(default)]
    pub parents: Vec<AgentId>,
    #[serde(default)]
    pub children: Vec<AgentId>,
    #[serde(default)]
    pub siblings: Vec<AgentId>,
    #[serde(default)]
    pub pair_bond: Option<AgentId>,
    #[serde(default)]
    pub household: Option<u64>,
}

impl Kinship {
    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
            && self.children.is_empty()
            && self.siblings.is_empty()
            && self.pair_bond.is_none()
            && self.household.is_none()
    }

    pub fn relatives(&self) -> Vec<AgentId> {
        let mut ids = Vec::new();
        for id in self
            .parents
            .iter()
            .chain(self.children.iter())
            .chain(self.siblings.iter())
            .chain(self.pair_bond.iter())
        {
            push_unique(&mut ids, *id);
        }
        ids
    }

    pub fn hash_into(&self, hasher: &mut impl sha2::Digest) {
        if self.is_empty() {
            return;
        }
        hasher.update([1u8]);
        for id in &self.parents {
            hasher.update(id.0.to_le_bytes());
        }
        hasher.update([0xff]);
        for id in &self.children {
            hasher.update(id.0.to_le_bytes());
        }
        hasher.update([0xff]);
        for id in &self.siblings {
            hasher.update(id.0.to_le_bytes());
        }
        hasher.update([0xff]);
        hasher.update(self.pair_bond.map(|id| id.0).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(self.household.unwrap_or(u64::MAX).to_le_bytes());
    }

    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        for id in &self.parents {
            out.push(format!("parent #{}", id.0));
        }
        for id in &self.children {
            out.push(format!("child #{}", id.0));
        }
        for id in &self.siblings {
            out.push(format!("sibling #{}", id.0));
        }
        if let Some(id) = self.pair_bond {
            out.push(format!("pair-bond #{}", id.0));
        }
        if let Some(h) = self.household {
            out.push(format!("household #{h}"));
        }
        out
    }
}

pub fn push_unique(ids: &mut Vec<AgentId>, id: AgentId) {
    if !ids.contains(&id) {
        ids.push(id);
        ids.sort_by_key(|x| x.0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationParams {
    pub reproduction: bool,
    pub aging: bool,
    pub childhood_ticks: u64,
    pub founder_age_ticks: u64,
}

impl Default for PopulationParams {
    fn default() -> Self {
        Self {
            reproduction: false,
            aging: false,
            childhood_ticks: 80,
            founder_age_ticks: 200,
        }
    }
}

impl PopulationParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            population: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            reproduction: Option<bool>,
            aging: Option<bool>,
            childhood_ticks: Option<u64>,
            founder_age_ticks: Option<u64>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            reproduction: slice.population.reproduction.unwrap_or(false),
            aging: slice.population.aging.unwrap_or(false),
            childhood_ticks: slice.population.childhood_ticks.unwrap_or(80),
            founder_age_ticks: slice.population.founder_age_ticks.unwrap_or(200),
        }
    }
}
