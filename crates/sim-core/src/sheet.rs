//! Overlay `[agents.sheet]`. Not on `ExperimentConfig` (not hashed).

use crate::agent::HEALTH_MAX;
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilitySheet {
    pub strength: u8,
    pub dexterity: u8,
    pub constitution: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub charisma: u8,
}

impl AbilitySheet {
    pub fn is_unused(&self) -> bool {
        self.strength == 0
            && self.dexterity == 0
            && self.constitution == 0
            && self.intelligence == 0
            && self.wisdom == 0
            && self.charisma == 0
    }

    pub fn modifier(score: u8) -> i32 {
        if score == 0 {
            0
        } else {
            (i32::from(score) - 10) / 2
        }
    }

    pub fn health_max(&self) -> u32 {
        if self.constitution == 0 {
            return HEALTH_MAX;
        }
        let m = Self::modifier(self.constitution);
        (HEALTH_MAX as i32 + m * 500).max(1) as u32
    }

    pub fn roll_3d6(rng: &mut ChaCha20Rng) -> Self {
        Self {
            strength: d6x3(rng),
            dexterity: d6x3(rng),
            constitution: d6x3(rng),
            intelligence: d6x3(rng),
            wisdom: d6x3(rng),
            charisma: d6x3(rng),
        }
    }

    pub fn mix(a: &Self, b: &Self, rng: &mut ChaCha20Rng) -> Self {
        Self {
            strength: mix_score(a.strength, b.strength, rng),
            dexterity: mix_score(a.dexterity, b.dexterity, rng),
            constitution: mix_score(a.constitution, b.constitution, rng),
            intelligence: mix_score(a.intelligence, b.intelligence, rng),
            wisdom: mix_score(a.wisdom, b.wisdom, rng),
            charisma: mix_score(a.charisma, b.charisma, rng),
        }
    }

    pub fn hash_into(&self, hasher: &mut impl sha2::Digest) {
        if self.is_unused() {
            return;
        }
        hasher.update([
            self.strength,
            self.dexterity,
            self.constitution,
            self.intelligence,
            self.wisdom,
            self.charisma,
        ]);
    }

    pub fn rows(&self) -> [(&'static str, u8); 6] {
        [
            ("STR", self.strength),
            ("DEX", self.dexterity),
            ("CON", self.constitution),
            ("INT", self.intelligence),
            ("WIS", self.wisdom),
            ("CHA", self.charisma),
        ]
    }
}

fn d6x3(rng: &mut ChaCha20Rng) -> u8 {
    rng.random_range(1..=6) + rng.random_range(1..=6) + rng.random_range(1..=6)
}

pub fn mix_score(a: u8, b: u8, rng: &mut ChaCha20Rng) -> u8 {
    let mid = (i32::from(a) + i32::from(b) + 1) / 2;
    let noise = rng.random_range(0..3i32) - 1;
    mid.saturating_add(noise).clamp(3, 18) as u8
}

pub fn mix_stat(a: u8, b: u8, rng: &mut ChaCha20Rng) -> u8 {
    let mid = (i32::from(a) + i32::from(b) + 1) / 2;
    let noise = rng.random_range(0..3i32) - 1;
    mid.saturating_add(noise).clamp(0, 100) as u8
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetParams {
    pub enabled: bool,
}

impl Default for SheetParams {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl SheetParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            agents: Agents,
        }
        #[derive(Default, Deserialize)]
        struct Agents {
            #[serde(default)]
            sheet: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.agents.sheet.enabled.unwrap_or(false),
        }
    }
}
