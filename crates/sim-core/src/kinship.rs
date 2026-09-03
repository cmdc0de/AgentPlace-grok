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
}

impl Kinship {
    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
            && self.children.is_empty()
            && self.siblings.is_empty()
            && self.pair_bond.is_none()
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
}

impl Default for PopulationParams {
    fn default() -> Self {
        Self {
            reproduction: false,
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
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            reproduction: slice.population.reproduction.unwrap_or(false),
        }
    }
}
