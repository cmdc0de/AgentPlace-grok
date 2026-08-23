mod render;

use bevy::prelude::*;
use render::{agent_world_pos, heightmap_mesh};
use sim_bevy::{step_once, SimPlugin, SimState};
use sim_core::{AgentId, ExperimentConfig};
use std::env;
use std::path::{Path, PathBuf};

const CAPSULE_RADIUS: f32 = 0.28;
const CAPSULE_LENGTH: f32 = 0.55;

#[derive(Component)]
struct AgentVisual {
    id: AgentId,
}

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct HudText;

fn main() {
    let config_path = parse_config_path();
    let config = ExperimentConfig::load_path(&config_path).unwrap_or_else(|e| {
        panic!("failed to load {}: {e}", config_path.display());
    });

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AgentTown viewer".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SimPlugin::new(config))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (handle_input, sync_agent_transforms, update_camera, update_hud).chain(),
        )
        .run();
}

fn parse_config_path() -> PathBuf {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        if matches!(args[i].as_str(), "--config" | "-c") {
            if let Some(path) = args.get(i + 1) {
                return PathBuf::from(path);
            }
        }
        i += 1;
    }
    default_config_path()
}

fn default_config_path() -> PathBuf {
    let candidates = [
        Path::new("configs/default.toml"),
        Path::new("../configs/default.toml"),
        Path::new("../../configs/default.toml"),
    ];
    for path in candidates {
        if path.exists() {
            return path.to_path_buf();
        }
    }
    PathBuf::from("configs/default.toml")
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimState>,
) {
    let world = &state.sim.world;
    let terrain = meshes.add(heightmap_mesh(world));
    commands.spawn((
        Mesh3d(terrain),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.52, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    let capsule = meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH));
    for agent in state.sim.agents.values() {
        let hue = (agent.id.0 as f32 * 47.0) % 360.0;
        let color = Color::hsl(hue, 0.7, 0.55);
        commands.spawn((
            Mesh3d(capsule.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.5,
                ..default()
            })),
            Transform::from_translation(agent_world_pos(world, agent.x, agent.y)),
            AgentVisual { id: agent.id },
        ));
    }

    let cx = world.width as f32 * 0.5;
    let cz = world.height as f32 * 0.5;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(cx - 28.0, 34.0, cz + 36.0).looking_at(Vec3::new(cx, 0.0, cz), Vec3::Y),
        FollowCamera,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.7, -0.9)),
    ));

    commands.spawn((
        Text::new("AgentTown"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HudText,
    ));
}

fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<SimState>) {
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }
    if keys.just_pressed(KeyCode::Period) {
        step_once(&mut state);
    }
    if keys.just_pressed(KeyCode::KeyF) {
        state.follow = match state.follow {
            None => Some(AgentId(0)),
            Some(_) => None,
        };
    }
    for (digit, id) in [
        (KeyCode::Digit0, 0u64),
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
        (KeyCode::Digit4, 4),
        (KeyCode::Digit5, 5),
        (KeyCode::Digit6, 6),
        (KeyCode::Digit7, 7),
        (KeyCode::Digit8, 8),
        (KeyCode::Digit9, 9),
    ] {
        if keys.just_pressed(digit) {
            state.follow = Some(AgentId(id));
        }
    }
}

fn sync_agent_transforms(
    state: Res<SimState>,
    mut query: Query<(&AgentVisual, &mut Transform)>,
) {
    for (visual, mut transform) in &mut query {
        if let Some(agent) = state.sim.agents.get(&visual.id) {
            transform.translation = agent_world_pos(&state.sim.world, agent.x, agent.y);
        }
    }
}

fn update_camera(state: Res<SimState>, mut query: Query<&mut Transform, With<FollowCamera>>) {
    let Some(id) = state.follow else {
        return;
    };
    let Some(agent) = state.sim.agents.get(&id) else {
        return;
    };
    let target = agent_world_pos(&state.sim.world, agent.x, agent.y);
    for mut transform in &mut query {
        transform.translation = target + Vec3::new(-12.0, 16.0, 14.0);
        transform.look_at(target, Vec3::Y);
    }
}

fn update_hud(state: Res<SimState>, mut query: Query<&mut Text, With<HudText>>) {
    let follow = match state.follow {
        Some(id) => format!("follow agent {}", id.0),
        None => "free camera".into(),
    };
    let paused = if state.paused { "paused" } else { "running" };
    let hash = state.sim.state_hash().to_string();
    let short = if hash.len() >= 12 { &hash[..12] } else { &hash };
    let text = format!(
        "tick {}  {}  {}  hash {}\nSpace pause  . step  F follow  0-9 follow agent",
        state.sim.tick, paused, follow, short
    );
    for mut hud in &mut query {
        *hud = Text::new(text.clone());
    }
}
