//! Overlay `[agents.sheet]`. Not on `ExperimentConfig` (not hashed).

use crate::agent::HEALTH_MAX;
use crate::config::MIN_MEMORY_CAPACITY;
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

    /// Energy cap after CON. Unused CON (mod 0) leaves `base`.
    pub fn energy_max(&self, base: u32) -> u32 {
        (base as i32 + Self::modifier(self.constitution) * 500).max(1) as u32
    }

    /// Toxic-eat illness ticks. Unused CON keeps 12.
    pub fn illness_duration(&self) -> u32 {
        (12 - Self::modifier(self.constitution)).max(1) as u32
    }

    /// Memory slot cap after INT. Unused INT (mod 0) leaves `base` (even if < 8).
    /// When INT is used, floor `MIN_MEMORY_CAPACITY`.
    pub fn memory_cap(&self, base: u32) -> u32 {
        let m = Self::modifier(self.intelligence);
        if m == 0 {
            return base;
        }
        (base as i32 + m * 4).max(MIN_MEMORY_CAPACITY as i32) as u32
    }

    /// Retrieval k after INT. Unused INT leaves `base`. Floor 1.
    pub fn retrieval_k(&self, base: u32) -> u32 {
        (base as i32 + Self::modifier(self.intelligence)).max(1) as u32
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

    /// Visible `Toxicity::Toxic` species join observation when WIS_mod > 0.
    pub fn detects_toxins(&self) -> bool {
        Self::modifier(self.wisdom) > 0
    }

    /// Open-board cells. Unused / low WIS leaves `ident` (does not shrink).
    pub fn board_cells(&self, ident: u32) -> u32 {
        ident.saturating_add(Self::modifier(self.wisdom).max(0) as u32)
    }

    /// SUPPORT social tuple after CHA. Unused CHA ⇒ (200, 0, 100, 0).
    pub fn support_social(&self) -> (i16, i16, i16, i16) {
        let m = Self::modifier(self.charisma);
        (
            (200 + m * 50).max(0) as i16,
            0,
            (100 + m * 25).max(0) as i16,
            0,
        )
    }

    /// PairBond hit. CHA 0 ⇒ always. Else d20 + CHA_mod >= 10.
    /// Stream is Invent-style `derive_seed`, not `RngBank.ensure`.
    pub fn pair_bond_hits(master: u64, tick: u64, agent: u64, charisma: u8) -> bool {
        if charisma == 0 {
            return true;
        }
        let seed =
            crate::seeding::derive_seed(master, &format!("tick_{tick}_agent_{agent}_pair_bond_0"));
        let mut rng = crate::seeding::rng_from_seed(seed);
        let d20: i32 = rng.random_range(1..=20);
        d20 + Self::modifier(charisma) >= 10
    }

    /// Toxic-eat applies illness. CON 0 ⇒ always. Else save `d20 + CON_mod >= 12` resists.
    /// Stream is Invent-style `derive_seed`, not `RngBank.ensure`.
    pub fn illness_hits(master: u64, tick: u64, agent: u64, constitution: u8) -> bool {
        if constitution == 0 {
            return true;
        }
        let seed =
            crate::seeding::derive_seed(master, &format!("tick_{tick}_agent_{agent}_illness_0"));
        let mut rng = crate::seeding::rng_from_seed(seed);
        let d20: i32 = rng.random_range(1..=20);
        d20 + Self::modifier(constitution) < 12
    }

    /// Plan step cap. Unused INT (mod 0) leaves `base`. Floor 1.
    pub fn plan_length(&self, base: u32) -> u32 {
        let m = Self::modifier(self.intelligence);
        if m == 0 {
            return base.max(1);
        }
        (base as i32 + m).max(1) as u32
    }

    /// Gather/hunt count after STR. Unused STR leaves `base`. Zero base stays 0.
    pub fn resource_qty(&self, base: u32) -> u32 {
        if base == 0 {
            return 0;
        }
        let m = Self::modifier(self.strength);
        if m == 0 {
            return base;
        }
        ((base as i64) * (1000 + i64::from(m) * 50) / 1000).max(1) as u32
    }

    /// Pocket weight cap milli. STR_mod 0 ⇒ none (unlimited, today).
    pub fn pocket_weight_cap(&self) -> Option<u32> {
        let m = Self::modifier(self.strength);
        if m == 0 {
            None
        } else {
            Some((8000 + m * 250).max(1) as u32)
        }
    }

    /// Speaker CHA added to hear/shout cells. Unused CHA leaves `base`.
    pub fn adjust_speech_range(&self, base: u32) -> u32 {
        (base as i32 + Self::modifier(self.charisma)).max(0) as u32
    }

    /// Heard-utterance importance. Unused CHA leaves `base`. Floor 1.
    pub fn speech_importance(&self, base: u32) -> u32 {
        (base as i32 + Self::modifier(self.charisma) * 5).max(1) as u32
    }

    /// SPEAK affinity millipoints. Unused CHA ⇒ 50.
    pub fn speak_affinity(&self) -> i16 {
        (50 + Self::modifier(self.charisma) * 10).max(0) as i16
    }

    /// Flee orthogonal steps. Unused / low DEX ⇒ 1.
    pub fn flee_steps(&self) -> u32 {
        1 + Self::modifier(self.dexterity).max(0) as u32 / 2
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
        assert_eq!(s.energy_max(10_000), 10_000);
        assert_eq!(s.illness_duration(), 12);
        assert_eq!(s.memory_cap(128), 128);
        assert_eq!(s.retrieval_k(8), 8);
        assert!(!s.detects_toxins());
        assert_eq!(s.adjust_speech_range(18), 18);
        assert_eq!(s.speech_importance(50), 50);
        assert_eq!(s.speak_affinity(), 50);
        assert_eq!(s.flee_steps(), 1);
        assert_eq!(s.board_cells(8), 8);
        assert_eq!(s.support_social(), (200, 0, 100, 0));
        assert!(AbilitySheet::pair_bond_hits(1, 0, 0, 0));
        assert!(AbilitySheet::illness_hits(1, 0, 0, 0));
        assert_eq!(s.plan_length(4), 4);
        assert_eq!(s.resource_qty(3), 3);
        assert_eq!(s.pocket_weight_cap(), None);
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
        let con_hi = AbilitySheet {
            constitution: 18,
            ..mid
        };
        let con_lo = AbilitySheet {
            constitution: 3,
            ..mid
        };
        assert_eq!(con_hi.energy_max(10_000), 12_000);
        assert_eq!(con_lo.energy_max(10_000), 8_500);
        assert_eq!(con_hi.illness_duration(), 8);
        assert_eq!(con_lo.illness_duration(), 15);
        let int_hi = AbilitySheet {
            intelligence: 18,
            ..mid
        };
        let int_lo = AbilitySheet {
            intelligence: 3,
            ..mid
        };
        assert_eq!(int_hi.memory_cap(128), 144);
        assert_eq!(int_lo.memory_cap(128), 116);
        assert_eq!(int_hi.retrieval_k(8), 12);
        assert_eq!(int_lo.retrieval_k(8), 5);
        assert!(high.detects_toxins());
        assert!(!mid.detects_toxins());
        assert!(!low.detects_toxins());
        assert_eq!(high.adjust_speech_range(18), 22);
        assert_eq!(low.adjust_speech_range(18), 15);
        assert_eq!(mid.adjust_speech_range(18), 18);
        assert_eq!(high.speech_importance(50), 70);
        assert_eq!(low.speech_importance(50), 35);
        assert_eq!(high.speak_affinity(), 90);
        assert_eq!(low.speak_affinity(), 20);
        assert_eq!(mid.speak_affinity(), 50);
        assert_eq!(high.flee_steps(), 3);
        assert_eq!(mid.flee_steps(), 1);
        let dex14 = AbilitySheet {
            dexterity: 14,
            ..mid
        };
        assert_eq!(dex14.flee_steps(), 2);
        assert_eq!(low.flee_steps(), 1);
        assert_eq!(high.board_cells(8), 12);
        assert_eq!(mid.board_cells(8), 8);
        assert_eq!(low.board_cells(8), 8);
        assert_eq!(high.support_social(), (400, 0, 200, 0));
        assert_eq!(low.support_social(), (50, 0, 25, 0));
        assert_eq!(mid.support_social(), (200, 0, 100, 0));
        assert_eq!(high.pocket_weight_cap(), Some(9000));
        assert_eq!(low.pocket_weight_cap(), Some(7250));
        assert_eq!(mid.pocket_weight_cap(), None);
        let str_hi = AbilitySheet {
            strength: 18,
            ..mid
        };
        let str_lo = AbilitySheet { strength: 3, ..mid };
        assert_eq!(str_hi.pocket_weight_cap(), Some(9000));
        assert_eq!(str_lo.pocket_weight_cap(), Some(7250));
        assert_eq!(int_hi.plan_length(4), 8);
        assert_eq!(int_lo.plan_length(4), 1);
        assert_eq!(str_hi.resource_qty(3), 3);
        assert_eq!(str_lo.resource_qty(3), 2);
        let mut sick_hi = 0u32;
        let mut sick_lo = 0u32;
        for t in 1..40u64 {
            if AbilitySheet::illness_hits(1, t, 0, 18) {
                sick_hi += 1;
            }
            if AbilitySheet::illness_hits(1, t, 0, 3) {
                sick_lo += 1;
            }
        }
        assert!(
            sick_hi < sick_lo,
            "CON 18 sick {sick_hi} vs CON 3 {sick_lo}"
        );
    }
}
