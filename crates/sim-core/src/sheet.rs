//! Overlay `[agents.sheet]`. Not on `ExperimentConfig` (not hashed).

use crate::agent::HEALTH_MAX;
use crate::conflict::ATTACK_DAMAGE;
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

    /// Attack damage millipoints. Unused STR (0) keeps `ATTACK_DAMAGE`.
    pub fn attack_damage(&self) -> u32 {
        (ATTACK_DAMAGE as i32 + Self::modifier(self.strength) * 250).max(1) as u32
    }

    /// Melee hit. Defender DEX 0 ⇒ always hit. Else d20 + STR_mod >= 10 + DEX_mod.
    /// Stream is Invent-style `derive_seed`, not `RngBank.ensure`.
    pub fn attack_hits(
        master: u64,
        tick: u64,
        attacker: u64,
        strength: u8,
        defender_dex: u8,
    ) -> bool {
        if defender_dex == 0 {
            return true;
        }
        let seed = crate::seeding::derive_seed(
            master,
            &format!("tick_{tick}_agent_{attacker}_attack_hit_0"),
        );
        let mut rng = crate::seeding::rng_from_seed(seed);
        let d20: i32 = rng.random_range(1..=20);
        d20 + Self::modifier(strength) >= 10 + Self::modifier(defender_dex)
    }

    /// Move energy after DEX. Unused DEX leaves `base`. Zero base stays 0.
    pub fn adjust_move_cost(&self, base: u32) -> u32 {
        if base == 0 {
            return 0;
        }
        (base as i32 - Self::modifier(self.dexterity) * 40).max(1) as u32
    }

    /// Vision/hearing/identity cells after WIS. Unused WIS leaves `base`.
    pub fn adjust_range(&self, base: u32) -> u32 {
        (base as i32 + Self::modifier(self.wisdom)).max(0) as u32
    }

    /// Influence vote weight. Does not write `influence_factor`.
    pub fn influence_vote_weight(&self, influence_factor: u32) -> u64 {
        (influence_factor as i32 + Self::modifier(self.charisma) * 100).max(1) as u64
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unused_sheet_keeps_constants() {
        let s = AbilitySheet::default();
        assert_eq!(s.attack_damage(), ATTACK_DAMAGE);
        assert!(AbilitySheet::attack_hits(1, 0, 0, 0, 0));
        assert_eq!(s.adjust_move_cost(200), 200);
        assert_eq!(s.adjust_move_cost(0), 0);
        assert_eq!(s.adjust_range(8), 8);
        assert_eq!(s.influence_vote_weight(50), 50);
        assert_eq!(s.influence_vote_weight(0), 1);
    }

    #[test]
    fn str_dex_wis_cha_mods() {
        let high = AbilitySheet {
            strength: 18,
            dexterity: 18,
            constitution: 10,
            intelligence: 10,
            wisdom: 18,
            charisma: 18,
        };
        let mid = AbilitySheet {
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
        };
        let low = AbilitySheet {
            strength: 3,
            dexterity: 3,
            constitution: 10,
            intelligence: 10,
            wisdom: 3,
            charisma: 3,
        };
        assert_eq!(high.attack_damage(), 3000);
        assert_eq!(mid.attack_damage(), 2000);
        assert_eq!(high.adjust_move_cost(200), 40);
        assert!(high.adjust_move_cost(200) < low.adjust_move_cost(200));
        assert_eq!(high.adjust_range(8), 12);
        assert_eq!(low.adjust_range(8), 5);
        assert_eq!(high.influence_vote_weight(1000), 1400);
        assert_eq!(mid.influence_vote_weight(1000), 1000);
    }
}
