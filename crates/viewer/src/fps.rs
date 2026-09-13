//! Native-window render FPS / frametime. Hash-neutral; not stored on Simulation.

use std::collections::VecDeque;

pub const FRAME_CAP: usize = 60;

#[derive(Clone, Debug, Default)]
pub struct FrameRing {
    samples_s: VecDeque<f32>,
}

impl FrameRing {
    pub fn push(&mut self, dt_s: f32) {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return;
        }
        self.samples_s.push_back(dt_s);
        while self.samples_s.len() > FRAME_CAP {
            self.samples_s.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.samples_s.len()
    }

    /// Mean of the window in milliseconds. Empty ⇒ 0.
    pub fn mean_ms(&self) -> f32 {
        if self.samples_s.is_empty() {
            return 0.0;
        }
        let n = self.samples_s.len() as f32;
        self.samples_s.iter().sum::<f32>() / n * 1000.0
    }

    /// `1000 / mean_ms`. Empty or zero mean ⇒ 0.
    pub fn fps(&self) -> f32 {
        let ms = self.mean_ms();
        if ms <= 0.0 {
            0.0
        } else {
            1000.0 / ms
        }
    }
}

/// Pure helper for tests: a slice of frame deltas (seconds) → (fps, mean_ms).
pub fn fps_from_deltas(deltas_s: &[f32]) -> (f32, f32) {
    let mut ring = FrameRing::default();
    for &dt in deltas_s {
        ring.push(dt);
    }
    (ring.fps(), ring.mean_ms())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{ExperimentConfig, Simulation};

    #[test]
    fn fps_helper_sixty_16ms_is_about_60() {
        let dt = 16.67 / 1000.0;
        let samples: Vec<f32> = (0..60).map(|_| dt).collect();
        let (fps, mean_ms) = fps_from_deltas(&samples);
        assert!(
            (mean_ms - 16.67).abs() < 0.05,
            "mean_ms={mean_ms}"
        );
        assert!((fps - 60.0).abs() < 0.3, "fps={fps}");
    }

    #[test]
    fn fps_helper_empty_is_zero() {
        let (fps, mean_ms) = fps_from_deltas(&[]);
        assert_eq!(fps, 0.0);
        assert_eq!(mean_ms, 0.0);
    }

    #[test]
    fn fps_not_hashed() {
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
        let mut ring = FrameRing::default();
        for _ in 0..60 {
            ring.push(0.016);
        }
        assert_eq!(ring.len(), 60);
        assert_eq!(
            sim.state_hash(),
            hash,
            "frame ring must not enter state_hash"
        );
    }
}
