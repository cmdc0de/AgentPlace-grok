use crate::config::WorldParams;
use rand::Rng;
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};

/// Integer heightmap. Cell (x, y) is at index `y * width + x`.
#[derive(Debug, Clone)]
pub struct World {
    pub width: u32,
    pub height: u32,
    pub max_height: u32,
    pub heights: Vec<u8>,
    pub world_seed: u64,
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

        Self {
            width,
            height,
            max_height,
            heights,
            world_seed,
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

    pub fn hash_bytes(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.world_seed.to_le_bytes());
        hasher.update(self.width.to_le_bytes());
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.max_height.to_le_bytes());
        hasher.update(&self.heights);
        hasher.finalize().into()
    }
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
