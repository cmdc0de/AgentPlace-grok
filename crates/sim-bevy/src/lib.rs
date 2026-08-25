//! Bevy integration: the simulation remains in `sim-core`.
//! This plugin owns a `Simulation` resource and ticks it on a timer.

use bevy::prelude::*;
use sim_core::{AgentId, ExperimentConfig, Simulation};

#[derive(Resource)]
pub struct SimState {
    pub sim: Simulation,
    pub paused: bool,
    pub follow: Option<AgentId>,
    /// When true the viewer does not tick locally; a remote `sim-cli` is the authority.
    pub remote: bool,
}

#[derive(Resource)]
pub struct SimTickTimer(pub Timer);

pub struct SimPlugin {
    pub sim: Simulation,
    pub tick_interval_secs: f32,
    pub remote: bool,
}

impl SimPlugin {
    pub fn new(config: ExperimentConfig) -> Self {
        Self {
            sim: Simulation::new(config).expect("invalid experiment config"),
            tick_interval_secs: 0.2,
            remote: false,
        }
    }

    pub fn from_simulation(sim: Simulation) -> Self {
        Self {
            sim,
            tick_interval_secs: 0.2,
            remote: false,
        }
    }

    pub fn from_remote(sim: Simulation) -> Self {
        Self {
            sim,
            tick_interval_secs: 0.2,
            remote: true,
        }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimState {
            sim: self.sim.clone(),
            paused: false,
            follow: None,
            remote: self.remote,
        })
        .insert_resource(SimTickTimer(Timer::from_seconds(
            self.tick_interval_secs,
            TimerMode::Repeating,
        )))
        .add_systems(Update, tick_sim);
    }
}

fn tick_sim(time: Res<Time>, mut timer: ResMut<SimTickTimer>, mut state: ResMut<SimState>) {
    if state.remote || state.paused {
        return;
    }
    if timer.0.tick(time.delta()).just_finished() {
        let _ = state.sim.tick();
    }
}

/// Advance exactly one tick regardless of pause/timer. Used for single-step.
pub fn step_once(state: &mut SimState) {
    let _ = state.sim.tick();
}
