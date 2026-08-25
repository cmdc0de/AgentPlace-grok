use crate::agent::AgentId;
use crate::config::EvictionPolicy;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum MemoryKind {
    #[default]
    Observation,
    Sickness,
    ToxinFact,
    Utterance,
    Action,
    Proposal,
    Interaction,
    Norm,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub tick: u64,
    pub kind: MemoryKind,
    pub text: String,
    pub importance: u8,
    #[serde(default)]
    pub last_accessed: u64,
    #[serde(default)]
    pub species_tag: u8,
    /// Packed into the checkpoint blob so M4 memory layouts still decode.
    #[serde(default, skip)]
    pub id: u64,
    #[serde(default, skip)]
    pub participants: Vec<AgentId>,
    #[serde(default, skip)]
    pub valence: i16,
}

impl MemoryEntry {
    pub fn hash_into(&self, hasher: &mut impl Digest) {
        hasher.update(self.tick.to_le_bytes());
        hasher.update([self.kind as u8]);
        hasher.update(self.text.as_bytes());
        hasher.update([self.importance]);
        hasher.update(self.last_accessed.to_le_bytes());
        hasher.update([self.species_tag]);
        hasher.update(self.id.to_le_bytes());
        hasher.update(self.valence.to_le_bytes());
        for p in &self.participants {
            hasher.update(p.0.to_le_bytes());
        }
    }

    pub fn protected_kind(&self) -> bool {
        matches!(self.kind, MemoryKind::Sickness | MemoryKind::ToxinFact)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMeta {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub participants: Vec<AgentId>,
    #[serde(default)]
    pub valence: i16,
}

pub fn remember(store: &mut Vec<MemoryEntry>, cap: u32, entry: MemoryEntry) {
    remember_policy(
        store,
        cap,
        EvictionPolicy::ImportanceAndRecency,
        150,
        true,
        &BTreeMap::new(),
        entry,
    );
}

pub fn remember_policy(
    store: &mut Vec<MemoryEntry>,
    cap: u32,
    policy: EvictionPolicy,
    social_bonus: u32,
    persist: bool,
    relationships: &BTreeMap<AgentId, crate::social::RelationshipSummary>,
    entry: MemoryEntry,
) {
    store.push(entry);
    evict(store, cap, policy, social_bonus, persist, relationships);
}

pub fn evict(
    store: &mut Vec<MemoryEntry>,
    cap: u32,
    policy: EvictionPolicy,
    social_bonus: u32,
    persist: bool,
    relationships: &BTreeMap<AgentId, crate::social::RelationshipSummary>,
) {
    let cap = cap as usize;
    while store.len() > cap {
        let protected = protected_indices(store, persist, relationships);
        let idx = pick_victim(store, policy, social_bonus, &protected);
        if let Some(i) = idx {
            store.remove(i);
        } else if !store.is_empty() {
            store.remove(0);
        } else {
            break;
        }
    }
}

fn protected_indices(
    store: &[MemoryEntry],
    persist: bool,
    relationships: &BTreeMap<AgentId, crate::social::RelationshipSummary>,
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (i, e) in store.iter().enumerate() {
        if e.protected_kind() {
            out.insert(i);
        }
    }
    if !persist {
        return out;
    }
    let mut partners: BTreeSet<AgentId> = relationships.keys().copied().collect();
    for e in store {
        partners.extend(e.participants.iter().copied());
    }
    for partner in partners {
        let best = store
            .iter()
            .enumerate()
            .filter(|(_, e)| e.participants.contains(&partner))
            .max_by_key(|(_, e)| e.importance)
            .map(|(i, _)| i);
        if let Some(i) = best {
            out.insert(i);
        }
    }
    out
}

fn pick_victim(
    store: &[MemoryEntry],
    policy: EvictionPolicy,
    social_bonus: u32,
    protected: &BTreeSet<usize>,
) -> Option<usize> {
    let candidates: Vec<usize> = (0..store.len())
        .filter(|i| !protected.contains(i))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    match policy {
        EvictionPolicy::Fifo => candidates.into_iter().min_by_key(|&i| store[i].tick),
        EvictionPolicy::LowestImportance => {
            candidates.into_iter().min_by_key(|&i| store[i].importance)
        }
        EvictionPolicy::ImportanceAndRecency => candidates
            .into_iter()
            .min_by_key(|&i| eviction_score(&store[i], social_bonus)),
    }
}

pub fn eviction_score(e: &MemoryEntry, social_bonus: u32) -> u64 {
    let recency = e.last_accessed.max(e.tick).saturating_add(1);
    let mut s = u64::from(e.importance).saturating_mul(recency);
    if !e.participants.is_empty() {
        s = s.saturating_mul(u64::from(social_bonus.max(100))) / 100;
    }
    s
}

pub fn retrieve<'a>(store: &'a [MemoryEntry], k: usize, social_bonus: u32) -> Vec<&'a MemoryEntry> {
    if k == 0 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..store.len()).collect();
    idx.sort_by(|a, b| {
        eviction_score(&store[*b], social_bonus).cmp(&eviction_score(&store[*a], social_bonus))
    });
    idx.into_iter().take(k).map(|i| &store[i]).collect()
}

pub fn knows_toxin(store: &[MemoryEntry], species_tag: u8) -> bool {
    store
        .iter()
        .any(|e| e.kind == MemoryKind::ToxinFact && e.species_tag == species_tag)
}
