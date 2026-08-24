use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MemoryKind {
    Observation,
    Sickness,
    ToxinFact,
    Utterance,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub tick: u64,
    pub kind: MemoryKind,
    pub text: String,
    pub importance: u8,
    #[serde(default)]
    pub last_accessed: u64,
    #[serde(default)]
    pub species_tag: u8,
}

impl MemoryEntry {
    pub fn hash_into(&self, hasher: &mut impl Digest) {
        hasher.update(self.tick.to_le_bytes());
        hasher.update([self.kind as u8]);
        hasher.update(self.text.as_bytes());
        hasher.update([self.importance]);
        hasher.update(self.last_accessed.to_le_bytes());
        hasher.update([self.species_tag]);
    }

    pub fn protected(&self) -> bool {
        matches!(self.kind, MemoryKind::Sickness | MemoryKind::ToxinFact)
    }
}

pub fn remember(
    store: &mut Vec<MemoryEntry>,
    cap: u32,
    entry: MemoryEntry,
) {
    store.push(entry);
    evict(store, cap);
}

pub fn evict(store: &mut Vec<MemoryEntry>, cap: u32) {
    let cap = cap as usize;
    while store.len() > cap {
        let mut worst_i = None;
        let mut worst_score = u64::MAX;
        for (i, e) in store.iter().enumerate() {
            if e.protected() {
                continue;
            }
            let recency = e.last_accessed.max(e.tick);
            let score = u64::from(e.importance).saturating_mul(recency.saturating_add(1));
            if score < worst_score {
                worst_score = score;
                worst_i = Some(i);
            }
        }
        let idx = worst_i.or_else(|| {
            store
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.protected())
                .min_by_key(|(_, e)| e.tick)
                .map(|(i, _)| i)
        });
        if let Some(i) = idx {
            store.remove(i);
        } else if !store.is_empty() {
            store.remove(0);
        } else {
            break;
        }
    }
}

pub fn knows_toxin(store: &[MemoryEntry], species_tag: u8) -> bool {
    store
        .iter()
        .any(|e| e.kind == MemoryKind::ToxinFact && e.species_tag == species_tag)
}
