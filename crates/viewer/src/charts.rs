//! Viewer/page-only tick series. Not hashed, not stored on Simulation.

use std::collections::VecDeque;

pub const CHART_CAP: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartSample {
    pub wall_ms: f32,
    pub living: u32,
    pub hungry: u32,
    pub thirsty: u32,
    pub mean_hunger: f32,
}

impl ChartSample {
    pub fn from_parts(
        wall_ns: u64,
        living: u32,
        hungry: u32,
        thirsty: u32,
        mean_hunger: f32,
    ) -> Self {
        Self {
            wall_ms: wall_ns as f32 / 1_000_000.0,
            living,
            hungry,
            thirsty,
            mean_hunger,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ChartRing {
    samples: VecDeque<ChartSample>,
}

impl ChartRing {
    pub fn push(&mut self, sample: ChartSample) {
        self.samples.push_back(sample);
        while self.samples.len() > CHART_CAP {
            self.samples.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn first(&self) -> Option<ChartSample> {
        self.samples.front().copied()
    }

    pub fn col_wall_ms(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.wall_ms).collect()
    }

    pub fn col_living(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.living as f32).collect()
    }

    pub fn col_hungry(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.hungry as f32).collect()
    }

    pub fn col_thirsty(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.thirsty as f32).collect()
    }

    pub fn col_mean_hunger(&self) -> Vec<f32> {
        self.samples.iter().map(|s| s.mean_hunger).collect()
    }

    pub fn wall_ms_mean(&self) -> Option<f32> {
        if self.samples.is_empty() {
            return None;
        }
        let n = self.samples.len() as f32;
        Some(self.samples.iter().map(|s| s.wall_ms).sum::<f32>() / n)
    }

    pub fn wall_ms_max(&self) -> Option<f32> {
        self.samples.iter().map(|s| s.wall_ms).reduce(f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{ExperimentConfig, Simulation};

    #[test]
    fn chart_ring_caps_at_256_and_drops_oldest() {
        let mut ring = ChartRing::default();
        for i in 0..300u64 {
            ring.push(ChartSample::from_parts(i * 1_000_000, 2, 1, 0, i as f32));
        }
        assert_eq!(ring.len(), 256);
        let first = ring.first().expect("first");
        assert!(
            (first.wall_ms - 44.0).abs() < 1e-4,
            "wall_ms={}",
            first.wall_ms
        );
        assert!((first.mean_hunger - 44.0).abs() < 1e-4);
    }

    #[test]
    fn charts_not_hashed() {
        let cfg = ExperimentConfig::from_toml_str(
            r#"
master_seed = 1
[simulation]
max_ticks = 10
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
        )
        .unwrap();
        let mut sim = Simulation::new(cfg).unwrap();
        sim.run_ticks(2);
        let hash = sim.state_hash();
        let mut ring = ChartRing::default();
        for i in 0..300u64 {
            ring.push(ChartSample::from_parts(i * 1_000_000, 2, 1, 0, 10.0));
        }
        assert_eq!(ring.len(), 256);
        assert_eq!(
            sim.state_hash(),
            hash,
            "chart ring must not enter state_hash"
        );
    }
}
