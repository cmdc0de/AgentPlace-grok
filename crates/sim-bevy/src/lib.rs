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
    pub paused: bool,
}

impl SimPlugin {
    pub fn new(config: ExperimentConfig) -> Self {
        Self {
            sim: Simulation::new(config).expect("invalid experiment config"),
            tick_interval_secs: 0.2,
            remote: false,
            paused: false,
        }
    }

    pub fn from_simulation(sim: Simulation) -> Self {
        Self {
            sim,
            tick_interval_secs: 0.2,
            remote: false,
            paused: false,
        }
    }

    pub fn from_remote(sim: Simulation) -> Self {
        Self {
            sim,
            tick_interval_secs: 0.2,
            remote: true,
            // Until Subscribe reports paused/playing; start-paused servers never Tick.
            paused: true,
        }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimState {
            sim: self.sim.clone(),
            paused: self.paused,
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
    if one_tick_due(&mut timer.0, time.delta()) {
        let _ = state.sim.tick();
    }
}

/// At most one sim tick per rendered frame, even if the timer is overdue.
pub fn one_tick_due(timer: &mut Timer, delta: std::time::Duration) -> bool {
    timer.tick(delta).just_finished()
}

/// Advance exactly one tick regardless of pause/timer. Used for single-step.
pub fn step_once(state: &mut SimState) {
    let _ = state.sim.tick();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn overdue_timer_still_one_tick_per_frame() {
        let mut timer = Timer::from_seconds(0.2, TimerMode::Repeating);
        assert!(one_tick_due(&mut timer, Duration::from_secs(1)));
        assert!(
            timer.times_finished_this_tick() >= 2,
            "hitch overshoots the interval"
        );
        // tick_sim uses just_finished once — it does not loop times_finished_this_tick.
    }

    #[test]
    fn remote_plugin_starts_paused() {
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
        let sim = Simulation::new(cfg).unwrap();
        let local = SimPlugin::from_simulation(sim.clone());
        assert!(!local.paused);
        assert!(!local.remote);
        let remote = SimPlugin::from_remote(sim);
        assert!(remote.paused);
        assert!(remote.remote);
    }
}
