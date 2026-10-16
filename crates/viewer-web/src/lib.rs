//! Primitive browser scene. Host tests do not open a window.
//! The Bevy app is compiled only for `wasm32`.

use sim_core::{ExperimentConfig, Simulation};

const DEMO_TOML: &str = r#"
master_seed = 78
[simulation]
max_ticks = 10
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#;

pub fn demo_sim() -> Simulation {
    let cfg = ExperimentConfig::from_toml_str(DEMO_TOML).expect("demo config");
    let mut sim = Simulation::new(cfg).expect("demo sim");
    sim.run_ticks(2);
    sim
}

/// Cell of each living agent. Sorted so tests are stable.
pub fn agent_marks(sim: &Simulation) -> Vec<(u32, u32)> {
    let mut marks: Vec<(u32, u32)> = sim.agents.values().map(|a| (a.x, a.y)).collect();
    marks.sort_unstable();
    marks
}

#[cfg(target_arch = "wasm32")]
mod scene {
    use super::agent_marks;
    use bevy::prelude::*;
    use wasm_bindgen::prelude::*;

    #[derive(Resource)]
    struct Marks(Vec<(u32, u32)>);

    #[wasm_bindgen(start)]
    pub fn start() {
        let sim = super::demo_sim();
        let marks = agent_marks(&sim);
        App::new()
            .add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "AgentPlace".into(),
                    canvas: Some("#bevy-canvas".into()),
                    ..default()
                }),
                ..default()
            }))
            .insert_resource(Marks(marks))
            .add_systems(Startup, spawn_scene)
            .run();
    }

    fn spawn_scene(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        marks: Res<Marks>,
    ) {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(32.0, 32.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.25, 0.45, 0.28))),
            Transform::from_xyz(16.0, 0.0, 16.0),
        ));
        let cube = meshes.add(Cuboid::new(0.6, 1.2, 0.6));
        let mat = materials.add(Color::srgb(0.9, 0.45, 0.2));
        for &(x, y) in &marks.0 {
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x as f32 + 0.5, 0.6, y as f32 + 0.5),
            ));
        }
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(16.0, 22.0, 30.0).looking_at(Vec3::new(16.0, 0.0, 16.0), Vec3::Y),
        ));
        commands.spawn((
            PointLight {
                intensity: 1_500_000.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(16.0, 18.0, 16.0),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_marks_match_agents() {
        let sim = demo_sim();
        let marks = agent_marks(&sim);
        assert_eq!(marks.len(), sim.agents.len());
        assert_eq!(marks.len(), 2);
        for a in sim.agents.values() {
            assert!(marks.contains(&(a.x, a.y)));
        }
    }
}
