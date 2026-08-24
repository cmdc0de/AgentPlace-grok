use crate::config::WorldParams;
use crate::species::Crop;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Expected resource cells per 1000 map cells at density 1.0.
const VEG_PER_MILLE: u32 = 50;
const MINERAL_PER_MILLE: u32 = 15;
const ANIMAL_PER_MILLE: u32 = 8;
const FISH_PER_MILLE: u32 = 10;

/// Integer heightmap plus cell-tag resource layers.
/// Cell (x, y) is at index `y * width + x`. Tags are 0 (absent) or 1 (present).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub width: u32,
    pub height: u32,
    pub max_height: u32,
    pub heights: Vec<u8>,
    pub world_seed: u64,
    pub water: Vec<u8>,
    /// 0 = none, 1..=N = vegetation species tag.
    pub vegetation: Vec<u8>,
    pub minerals: Vec<u8>,
    #[serde(default)]
    pub animals: Vec<u8>,
    #[serde(default)]
    pub fish: Vec<u8>,
    #[serde(default)]
    pub crops: BTreeMap<(u32, u32), Crop>,
}

impl World {
    pub fn generate(params: &WorldParams, world_seed: u64, rng: &mut ChaCha20Rng) -> Self {
        let width = params.width;
        let height = params.height;
        let max_height = params.max_height;
        let mut perm = [0u8; 256];
        for (i, slot) in perm.iter_mut().enumerate() {
            *slot = i as u8;
        }
        for i in (1..256).rev() {
            let j = rng.random_range(0..=i);
            perm.swap(i, j);
        }

        let octaves = params.terrain.octaves.max(1);
        let persistence = params.terrain.persistence;
        let lacunarity = params.terrain.lacunarity;

        let mut heights = vec![0u8; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let mut amp = 1.0;
                let mut freq = 1.0 / 24.0;
                let mut value = 0.0;
                let mut amp_sum = 0.0;
                for _ in 0..octaves {
                    value += amp * value_noise(x as f64 * freq, y as f64 * freq, &perm);
                    amp_sum += amp;
                    amp *= persistence;
                    freq *= lacunarity;
                }
                let n = if amp_sum > 0.0 { value / amp_sum } else { 0.0 };
                let h = (n.clamp(0.0, 1.0) * max_height as f64).round() as u8;
                heights[(y * width + x) as usize] = h.min(max_height as u8);
            }
        }

        let (water, vegetation, minerals, animals, fish) =
            place_resources(params, rng, width, height, &heights);

        Self {
            width,
            height,
            max_height,
            heights,
            world_seed,
            water,
            vegetation,
            minerals,
            animals,
            fish,
            crops: BTreeMap::new(),
        }
    }

    pub fn height_at(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.heights[(y * self.width + x) as usize]
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn is_water(&self, x: u32, y: u32) -> bool {
        self.water.get(self.idx(x, y)).copied().unwrap_or(0) != 0
    }

    pub fn is_land(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && !self.is_water(x, y)
    }

    pub fn has_vegetation(&self, x: u32, y: u32) -> bool {
        self.vegetation.get(self.idx(x, y)).copied().unwrap_or(0) != 0
    }

    pub fn has_mineral(&self, x: u32, y: u32) -> bool {
        self.minerals.get(self.idx(x, y)).copied().unwrap_or(0) != 0
    }

    pub fn water_count(&self) -> u32 {
        self.water.iter().map(|&v| u32::from(v != 0)).sum()
    }

    pub fn vegetation_count(&self) -> u32 {
        self.vegetation.iter().map(|&v| u32::from(v != 0)).sum()
    }

    pub fn mineral_count(&self) -> u32 {
        self.minerals.iter().map(|&v| u32::from(v != 0)).sum()
    }

    pub fn vegetation_species(&self, x: u32, y: u32) -> u8 {
        self.vegetation.get(self.idx(x, y)).copied().unwrap_or(0)
    }

    pub fn animal_count_at(&self, x: u32, y: u32) -> u8 {
        self.animals.get(self.idx(x, y)).copied().unwrap_or(0)
    }

    pub fn fish_count_at(&self, x: u32, y: u32) -> u8 {
        self.fish.get(self.idx(x, y)).copied().unwrap_or(0)
    }

    pub fn animal_total(&self) -> u32 {
        self.animals.iter().map(|&c| u32::from(c)).sum()
    }

    pub fn fish_total(&self) -> u32 {
        self.fish.iter().map(|&c| u32::from(c)).sum()
    }

    pub fn set_vegetation(&mut self, x: u32, y: u32, tag: u8) {
        let i = self.idx(x, y);
        if i < self.vegetation.len() {
            self.vegetation[i] = tag;
        }
    }

    pub fn add_animal(&mut self, x: u32, y: u32, delta: i8) {
        let i = self.idx(x, y);
        if i < self.animals.len() {
            let v = self.animals[i] as i16 + i16::from(delta);
            self.animals[i] = v.clamp(0, 255) as u8;
        }
    }

    pub fn add_fish(&mut self, x: u32, y: u32, delta: i8) {
        let i = self.idx(x, y);
        if i < self.fish.len() {
            let v = self.fish[i] as i16 + i16::from(delta);
            self.fish[i] = v.clamp(0, 255) as u8;
        }
    }

    pub fn land_cells(&self) -> Vec<(u32, u32)> {
        let mut cells = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_land(x, y) {
                    cells.push((x, y));
                }
            }
        }
        cells
    }

    pub fn hash_bytes(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.world_seed.to_le_bytes());
        hasher.update(self.width.to_le_bytes());
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.max_height.to_le_bytes());
        hasher.update(&self.heights);
        hasher.update(&self.water);
        hasher.update(&self.vegetation);
        hasher.update(&self.minerals);
        hasher.update(&self.animals);
        hasher.update(&self.fish);
        for ((x, y), crop) in &self.crops {
            hasher.update(x.to_le_bytes());
            hasher.update(y.to_le_bytes());
            hasher.update([crop.species_tag]);
            hasher.update(crop.planted_tick.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

fn cell_index(width: u32, x: u32, y: u32) -> usize {
    (y * width + x) as usize
}

fn scaled_count(density: f64, area: u32, per_mille: u32) -> u32 {
    if density <= 0.0 {
        return 0;
    }
    let n = density * f64::from(area) * f64::from(per_mille) / 1000.0;
    n.round().clamp(0.0, f64::from(area)) as u32
}

fn place_resources(
    params: &WorldParams,
    rng: &mut ChaCha20Rng,
    width: u32,
    height: u32,
    heights: &[u8],
) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let area = (width as usize) * (height as usize);
    let mut water = vec![0u8; area];
    let mut vegetation = vec![0u8; area];
    let mut minerals = vec![0u8; area];
    let mut animals = vec![0u8; area];
    let mut fish = vec![0u8; area];

    let mut ranked: Vec<(u8, u32, u32)> = Vec::with_capacity(area);
    for y in 0..height {
        for x in 0..width {
            ranked.push((heights[cell_index(width, x, y)], y, x));
        }
    }
    ranked.sort_unstable();

    let coverage = params.resources.water_coverage.clamp(0.0, 1.0);
    let from_coverage = (coverage * area as f64).round() as u32;
    let water_target = from_coverage
        .max(params.resources.min_fresh_water)
        .min(area as u32);

    for &(_h, y, x) in ranked.iter().take(water_target as usize) {
        water[cell_index(width, x, y)] = 1;
    }

    let mut land: Vec<(u32, u32)> = ranked
        .iter()
        .filter(|&&(_h, y, x)| water[cell_index(width, x, y)] == 0)
        .map(|&(_h, y, x)| (x, y))
        .collect();
    land.shuffle(rng);

    let mineral_target = scaled_count(
        params.resources.mineral_density,
        area as u32,
        MINERAL_PER_MILLE,
    )
    .max(params.resources.min_mineral_nodes)
    .min(land.len() as u32);

    for &(x, y) in land.iter().take(mineral_target as usize) {
        minerals[cell_index(width, x, y)] = 1;
    }

    let remaining: Vec<(u32, u32)> = land
        .iter()
        .copied()
        .skip(mineral_target as usize)
        .collect();
    let veg_target = scaled_count(
        params.resources.vegetation_density,
        area as u32,
        VEG_PER_MILLE,
    )
    .max(params.resources.min_vegetation_patches)
    .min(remaining.len() as u32);

    let n_species = params.species.vegetation.len().max(1) as u8;
    for &(x, y) in remaining.iter().take(veg_target as usize) {
        vegetation[cell_index(width, x, y)] = rng.random_range(1..=n_species);
    }

    let land_after: Vec<(u32, u32)> = remaining
        .iter()
        .copied()
        .skip(veg_target as usize)
        .collect();
    let animal_target = scaled_count(
        params.resources.animal_density,
        area as u32,
        ANIMAL_PER_MILLE,
    )
    .max(params.resources.min_animals)
    .min(land_after.len() as u32);
    for &(x, y) in land_after.iter().take(animal_target as usize) {
        animals[cell_index(width, x, y)] = 1;
    }

    let mut water_cells: Vec<(u32, u32)> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if water[cell_index(width, x, y)] != 0 {
                water_cells.push((x, y));
            }
        }
    }
    water_cells.shuffle(rng);
    let fish_target = scaled_count(params.resources.fish_density, area as u32, FISH_PER_MILLE)
        .max(params.resources.min_fish)
        .min(water_cells.len() as u32);
    for &(x, y) in water_cells.iter().take(fish_target as usize) {
        fish[cell_index(width, x, y)] = 1;
    }

    (water, vegetation, minerals, animals, fish)
}

fn fade(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lattice(ix: i32, iy: i32, perm: &[u8; 256]) -> f64 {
    let mut h = perm[(ix.wrapping_mul(1597).wrapping_add(iy.wrapping_mul(3119)) as u32 as usize) & 255]
        as usize;
    h = perm[(h + (iy.wrapping_mul(197) as u32 as usize)) & 255] as usize;
    perm[h] as f64 / 255.0
}

fn value_noise(x: f64, y: f64, perm: &[u8; 256]) -> f64 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = fade(x - x0 as f64);
    let fy = fade(y - y0 as f64);
    let v00 = lattice(x0, y0, perm);
    let v10 = lattice(x0 + 1, y0, perm);
    let v01 = lattice(x0, y0 + 1, perm);
    let v11 = lattice(x0 + 1, y0 + 1, perm);
    lerp(lerp(v00, v10, fx), lerp(v01, v11, fx), fy)
}
