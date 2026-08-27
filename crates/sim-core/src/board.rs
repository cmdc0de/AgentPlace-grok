//! Public proposal board and adopted rules.

use crate::agent::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProposalStatus {
    Open,
    Accepted,
    Rejected,
    Expired,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        ProposalStatus::Open
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuredRule {
    BanEatSpecies { species: u8 },
    BanGatherSpecies { species: u8 },
    MaxGatherPerTick { n: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub author: AgentId,
    pub tick_created: u64,
    pub text: String,
    #[serde(default)]
    pub rule: Option<StructuredRule>,
    #[serde(default)]
    pub supporters: BTreeSet<AgentId>,
    #[serde(default)]
    pub opposers: BTreeSet<AgentId>,
    #[serde(default)]
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptedRule {
    pub proposal_id: u64,
    pub tick_accepted: u64,
    pub text: String,
    #[serde(default)]
    pub rule: Option<StructuredRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicBoard {
    #[serde(default)]
    pub proposals: Vec<Proposal>,
    #[serde(default)]
    pub adopted: Vec<AdoptedRule>,
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub accepted_count: u32,
    #[serde(default)]
    pub rejected_count: u32,
    #[serde(default)]
    pub expired_count: u32,
}

impl PublicBoard {
    pub fn open(&self) -> impl Iterator<Item = &Proposal> {
        self.proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Open)
    }

    pub fn author_open_count(&self, author: AgentId) -> usize {
        self.open().filter(|p| p.author == author).count()
    }

    pub fn hash_into(&self, hasher: &mut impl sha2::Digest) {
        hasher.update(self.next_id.to_le_bytes());
        hasher.update(self.accepted_count.to_le_bytes());
        hasher.update(self.rejected_count.to_le_bytes());
        hasher.update(self.expired_count.to_le_bytes());
        for p in &self.proposals {
            hasher.update(p.id.to_le_bytes());
            hasher.update(p.author.0.to_le_bytes());
            hasher.update(p.tick_created.to_le_bytes());
            hasher.update(p.text.as_bytes());
            hasher.update([p.status as u8]);
            hash_rule(hasher, p.rule.as_ref());
            for id in &p.supporters {
                hasher.update(id.0.to_le_bytes());
            }
            hasher.update([0xff]);
            for id in &p.opposers {
                hasher.update(id.0.to_le_bytes());
            }
        }
        for r in &self.adopted {
            hasher.update(r.proposal_id.to_le_bytes());
            hasher.update(r.tick_accepted.to_le_bytes());
            hasher.update(r.text.as_bytes());
            hash_rule(hasher, r.rule.as_ref());
        }
    }

    pub fn tick_lifecycle(
        &mut self,
        total_weight: u64,
        weight_of: impl Fn(AgentId) -> u64,
        threshold: f64,
        lifetime: u64,
        tick: u64,
    ) {
        let need = ((threshold * total_weight as f64).ceil() as u64).max(1);
        let mut newly = Vec::new();
        for p in &mut self.proposals {
            if p.status != ProposalStatus::Open {
                continue;
            }
            if lifetime > 0 && tick.saturating_sub(p.tick_created) >= lifetime {
                p.status = ProposalStatus::Expired;
                self.expired_count += 1;
                continue;
            }
            let yes: u64 = p.supporters.iter().map(|id| weight_of(*id)).sum();
            let no: u64 = p.opposers.iter().map(|id| weight_of(*id)).sum();
            if yes >= need {
                p.status = ProposalStatus::Accepted;
                self.accepted_count += 1;
                newly.push(AdoptedRule {
                    proposal_id: p.id,
                    tick_accepted: tick,
                    text: p.text.clone(),
                    rule: p.rule,
                });
            } else if no >= need {
                p.status = ProposalStatus::Rejected;
                self.rejected_count += 1;
            }
        }
        self.adopted.extend(newly);
    }

    pub fn blocks_eat(&self, species: u8) -> bool {
        self.adopted.iter().any(|r| {
            matches!(r.rule, Some(StructuredRule::BanEatSpecies { species: s }) if s == species)
        })
    }

    pub fn blocks_gather(&self, species: u8) -> bool {
        self.adopted.iter().any(|r| {
            matches!(r.rule, Some(StructuredRule::BanGatherSpecies { species: s }) if s == species)
        })
    }

    pub fn max_gather_per_tick(&self) -> Option<u32> {
        self.adopted
            .iter()
            .filter_map(|r| match r.rule {
                Some(StructuredRule::MaxGatherPerTick { n }) => Some(n),
                _ => None,
            })
            .min()
    }

    pub fn has_open_ban_eat(&self, species: u8) -> bool {
        self.open().any(|p| {
            matches!(p.rule, Some(StructuredRule::BanEatSpecies { species: s }) if s == species)
        }) || self.blocks_eat(species)
    }
}

fn hash_rule(hasher: &mut impl sha2::Digest, rule: Option<&StructuredRule>) {
    match rule {
        None => hasher.update([0u8]),
        Some(StructuredRule::BanEatSpecies { species }) => {
            hasher.update([1u8, *species]);
        }
        Some(StructuredRule::BanGatherSpecies { species }) => {
            hasher.update([2u8, *species]);
        }
        Some(StructuredRule::MaxGatherPerTick { n }) => {
            hasher.update([3u8]);
            hasher.update(n.to_le_bytes());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalView {
    pub id: u64,
    pub author: Option<AgentId>,
    pub text: String,
    pub status: ProposalStatus,
    pub support: u32,
    pub oppose: u32,
    pub rule: Option<StructuredRule>,
    #[serde(default)]
    pub you_support: bool,
    #[serde(default)]
    pub you_oppose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub id: u64,
    pub text: String,
    pub priority: u8,
    pub source: String,
}

impl Goal {
    pub fn hash_into(&self, hasher: &mut impl sha2::Digest) {
        hasher.update(self.id.to_le_bytes());
        hasher.update(self.text.as_bytes());
        hasher.update([self.priority]);
        hasher.update(self.source.as_bytes());
    }
}
