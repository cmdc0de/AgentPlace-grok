//! Species tables and toxicity. Ids are 1-based; 0 means “none” on a cell.

use serde::{Deserialize, Serialize};

pub const MAX_MILLI: u32 = 10_000;

pub fn f64_to_milli(v: f64) -> u32 {
    (v * 100.0).round().clamp(0.0, MAX_MILLI as f64) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Toxicity {
    Safe,
    Toxic,
    Allergenic,
}

impl Default for Toxicity {
    fn default() -> Self {
        Toxicity::Safe
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VegYield {
    Food,
    Wood,
}

impl Default for VegYield {
    fn default() -> Self {
        VegYield::Food
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegetationSpecies {
    pub id: String,
    #[serde(default, rename = "yield")]
    pub yield_kind: VegYield,
    #[serde(default = "default_nutrition")]
    pub nutrition: f64,
    #[serde(default)]
    pub toxicity: Toxicity,
    #[serde(default)]
    pub allergen_tag: String,
    #[serde(default)]
    pub wood_yield: u32,
    #[serde(default)]
    pub fiber_yield: u32,
    #[serde(default = "default_grow")]
    pub grow_ticks: u64,
}

impl VegetationSpecies {
    pub fn nutrition_milli(&self) -> u32 {
        f64_to_milli(self.nutrition)
    }
}

fn default_nutrition() -> f64 {
    20.0
}
fn default_grow() -> u64 {
    40
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaunaSpecies {
    pub id: String,
    #[serde(default = "default_nutrition")]
    pub nutrition: f64,
    #[serde(default)]
    pub toxicity: Toxicity,
    #[serde(default)]
    pub allergen_tag: String,
}

impl FaunaSpecies {
    pub fn nutrition_milli(&self) -> u32 {
        f64_to_milli(self.nutrition)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesTables {
    #[serde(default)]
    pub vegetation: Vec<VegetationSpecies>,
    #[serde(default)]
    pub animals: Vec<FaunaSpecies>,
    #[serde(default)]
    pub fish: Vec<FaunaSpecies>,
}

impl Default for SpeciesTables {
    fn default() -> Self {
        default_species_tables()
    }
}

impl SpeciesTables {
    pub fn ensure_defaults(&mut self) {
        if self.vegetation.is_empty() {
            *self = default_species_tables();
        }
    }

    /// Cell tag 1..=len maps to this slice. 0 = none.
    pub fn veg(&self, tag: u8) -> Option<&VegetationSpecies> {
        if tag == 0 {
            return None;
        }
        self.vegetation.get((tag - 1) as usize)
    }

    pub fn veg_tag_by_id(&self, id: &str) -> Option<u8> {
        self.vegetation
            .iter()
            .position(|s| s.id == id)
            .map(|i| (i + 1) as u8)
    }
}

pub fn default_species_tables() -> SpeciesTables {
    SpeciesTables {
        vegetation: vec![
            VegetationSpecies {
                id: "berry_bush".into(),
                yield_kind: VegYield::Food,
                nutrition: 20.0,
                toxicity: Toxicity::Safe,
                allergen_tag: String::new(),
                wood_yield: 0,
                fiber_yield: 1,
                grow_ticks: 40,
            },
            VegetationSpecies {
                id: "herb".into(),
                yield_kind: VegYield::Food,
                nutrition: 15.0,
                toxicity: Toxicity::Safe,
                allergen_tag: String::new(),
                wood_yield: 0,
                fiber_yield: 1,
                grow_ticks: 30,
            },
            VegetationSpecies {
                id: "mushroom".into(),
                yield_kind: VegYield::Food,
                nutrition: 12.0,
                toxicity: Toxicity::Toxic,
                allergen_tag: String::new(),
                wood_yield: 0,
                fiber_yield: 0,
                grow_ticks: 20,
            },
            VegetationSpecies {
                id: "nightshade".into(),
                yield_kind: VegYield::Food,
                nutrition: 10.0,
                toxicity: Toxicity::Allergenic,
                allergen_tag: "solanaceae".into(),
                wood_yield: 0,
                fiber_yield: 0,
                grow_ticks: 35,
            },
            VegetationSpecies {
                id: "tree".into(),
                yield_kind: VegYield::Wood,
                nutrition: 0.0,
                toxicity: Toxicity::Safe,
                allergen_tag: String::new(),
                wood_yield: 3,
                fiber_yield: 0,
                grow_ticks: 80,
            },
        ],
        animals: vec![FaunaSpecies {
            id: "hare".into(),
            nutrition: 30.0,
            toxicity: Toxicity::Safe,
            allergen_tag: String::new(),
        }],
        fish: vec![FaunaSpecies {
            id: "perch".into(),
            nutrition: 25.0,
            toxicity: Toxicity::Safe,
            allergen_tag: String::new(),
        }],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crop {
    pub species_tag: u8,
    pub planted_tick: u64,
}
