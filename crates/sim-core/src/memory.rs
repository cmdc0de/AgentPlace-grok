use crate::agent::AgentId;
use crate::config::EvictionPolicy;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Local projection width. Not in `state_hash`.
pub const EMBED_DIM: usize = 32;
const EMBED_SEED: u64 = 0xA6E7_B0DE_E4BE_D001;

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
    Reflection,
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
    /// Local bag-of-bytes projection. Skip serde and `hash_into`.
    #[serde(default, skip)]
    pub embedding: Vec<i16>,
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

    pub fn ensure_embedding(&mut self) {
        if self.embedding.is_empty() {
            self.embedding = project_text(&self.text);
        }
    }

    pub fn protected_kind(&self) -> bool {
        matches!(
            self.kind,
            MemoryKind::Sickness | MemoryKind::ToxinFact | MemoryKind::Reflection
        )
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
) -> Vec<String> {
    store.push(entry);
    evict(store, cap, policy, social_bonus, persist, relationships)
}

pub fn evict(
    store: &mut Vec<MemoryEntry>,
    cap: u32,
    policy: EvictionPolicy,
    social_bonus: u32,
    persist: bool,
    relationships: &BTreeMap<AgentId, crate::social::RelationshipSummary>,
) -> Vec<String> {
    let cap = cap as usize;
    let mut dropped = Vec::new();
    while store.len() > cap {
        let protected = protected_indices(store, persist, relationships);
        let idx = pick_victim(store, policy, social_bonus, &protected);
        if let Some(i) = idx {
            dropped.push(store.remove(i).text);
        } else if !store.is_empty() {
            dropped.push(store.remove(0).text);
        } else {
            break;
        }
    }
    dropped
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

/// Rank by importance × recency. When `query` is `Some`, mix in cosine of a
/// local projection of `text` (no network). Vectors are not hashed.
pub fn retrieve<'a>(
    store: &'a [MemoryEntry],
    k: usize,
    social_bonus: u32,
    query: Option<&str>,
) -> Vec<&'a MemoryEntry> {
    if k == 0 {
        return Vec::new();
    }
    let qvec = query.map(project_text);
    let mut idx: Vec<usize> = (0..store.len()).collect();
    idx.sort_by(|a, b| {
        retrieval_score(&store[*b], social_bonus, qvec.as_deref())
            .cmp(&retrieval_score(&store[*a], social_bonus, qvec.as_deref()))
    });
    idx.into_iter().take(k).map(|i| &store[i]).collect()
}

fn retrieval_score(e: &MemoryEntry, social_bonus: u32, query: Option<&[i16]>) -> u64 {
    let base = eviction_score(e, social_bonus);
    let Some(q) = query else {
        return base;
    };
    let owned;
    let ev = if e.embedding.len() == EMBED_DIM {
        e.embedding.as_slice()
    } else {
        owned = project_text(&e.text);
        owned.as_slice()
    };
    let sim = cosine_milli(q, ev).max(0) as u64;
    // Cosine millipoints mixed in so similar text can outrank a modest
    // importance gap; sim=0 keeps eviction_score order.
    base.saturating_mul(1000)
        .saturating_add(sim.saturating_mul(50_000))
}

/// Seeded bag-of-bytes / n-gram hashing trick. Deterministic; no HTTP.
pub fn project_text(text: &str) -> Vec<i16> {
    let mut acc = vec![0i32; EMBED_DIM];
    let bytes = text.as_bytes();
    add_feature(&mut acc, bytes);
    for window in 1..=3 {
        if bytes.len() < window {
            break;
        }
        for ngram in bytes.windows(window) {
            add_feature(&mut acc, ngram);
        }
    }
    acc.into_iter()
        .map(|v| v.clamp(i16::MIN as i32, i16::MAX as i32) as i16)
        .collect()
}

fn add_feature(acc: &mut [i32], bytes: &[u8]) {
    let mut hasher = XxHash64::with_seed(EMBED_SEED);
    bytes.hash(&mut hasher);
    let h = hasher.finish();
    let idx = (h as usize) % EMBED_DIM;
    let sign = if h & 1 == 0 { 1i32 } else { -1i32 };
    acc[idx] = acc[idx].saturating_add(sign);
}

fn cosine_milli(a: &[i16], b: &[i16]) -> i32 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0;
    }
    let mut dot: i64 = 0;
    let mut na: i64 = 0;
    let mut nb: i64 = 0;
    for i in 0..n {
        let x = i64::from(a[i]);
        let y = i64::from(b[i]);
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0 || nb == 0 {
        return 0;
    }
    let denom = isqrt(na.saturating_mul(nb));
    if denom == 0 {
        return 0;
    }
    ((dot.saturating_mul(1000)) / denom).clamp(-1000, 1000) as i32
}

fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

pub fn knows_toxin(store: &[MemoryEntry], species_tag: u8) -> bool {
    store
        .iter()
        .any(|e| e.kind == MemoryKind::ToxinFact && e.species_tag == species_tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn entry(text: &str, importance: u8) -> MemoryEntry {
        MemoryEntry {
            tick: 1,
            kind: MemoryKind::Observation,
            text: text.into(),
            importance,
            last_accessed: 1,
            ..Default::default()
        }
    }

    #[test]
    fn retrieve_embeddings_off_matches_legacy_order() {
        let store = vec![
            entry("zzzz noise", 50),
            entry("apple orchard harvest", 40),
            entry("banana stand", 40),
        ];
        let got: Vec<&str> = retrieve(&store, 3, 100, None)
            .into_iter()
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(
            got,
            vec!["zzzz noise", "apple orchard harvest", "banana stand"]
        );
    }

    #[test]
    fn retrieve_embeddings_prefer_similar() {
        let store = vec![
            entry("zzzz noise", 50),
            entry("apple orchard harvest", 40),
            entry("banana stand", 40),
        ];
        let got: Vec<&str> = retrieve(&store, 3, 100, Some("apple harvest"))
            .into_iter()
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(got[0], "apple orchard harvest", "{got:?}");
        assert_ne!(project_text("apple"), project_text("zzzz"));
    }

    #[test]
    fn embeddings_ignored_by_hash_into() {
        let mut a = entry("same text", 40);
        let mut b = a.clone();
        a.embedding = vec![1; EMBED_DIM];
        b.embedding = vec![-1; EMBED_DIM];
        let mut ha = Sha256::new();
        let mut hb = Sha256::new();
        a.hash_into(&mut ha);
        b.hash_into(&mut hb);
        assert_eq!(ha.finalize(), hb.finalize());
    }
}
