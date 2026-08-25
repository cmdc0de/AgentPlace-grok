mod render;

use bevy::prelude::*;
use render::{agent_world_pos, heightmap_mesh, resource_world_pos};
use sim_bevy::{SimPlugin, SimState, step_once};
use sim_core::observation::{chebyshev, effective_range};
use sim_core::{AgentId, ExperimentConfig, Simulation};
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

#[derive(Component)]
struct VisionOverlay;

fn main() {
    let plugin = match parse_args() {
        ViewerSource::Config(path) => {
            let config = ExperimentConfig::load_path(&path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            SimPlugin::new(config)
        }
        ViewerSource::Checkpoint(path) => {
            let sim = Simulation::load_checkpoint(&path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            SimPlugin::from_simulation(sim)
        }
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AgentTown viewer".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(plugin)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                handle_input,
                sync_agent_transforms,
                update_camera,
                update_hud,
                update_vision_overlay,
            )
                .chain(),
        )
        .run();
}

enum ViewerSource {
    Config(PathBuf),
    Checkpoint(PathBuf),
}

fn parse_args() -> ViewerSource {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    let mut config = None;
    let mut load = None;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if let Some(path) = args.get(i + 1) {
                    config = Some(PathBuf::from(path));
                    i += 2;
                    continue;
                }
            }
            "--load" => {
                if let Some(path) = args.get(i + 1) {
                    load = Some(PathBuf::from(path));
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if let Some(path) = load {
        ViewerSource::Checkpoint(path)
    } else {
        ViewerSource::Config(config.unwrap_or_else(default_config_path))
    }
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
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    let veg_mesh = meshes.add(Cuboid::new(0.35, 0.45, 0.35));
    let animal_mesh = meshes.add(Cuboid::new(0.22, 0.22, 0.38));
    let fish_mesh = meshes.add(Cuboid::new(0.18, 0.12, 0.28));
    let animal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.55, 0.32),
        perceptual_roughness: 0.7,
        ..default()
    });
    let fish_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.45, 0.75),
        perceptual_roughness: 0.4,
        ..default()
    });
    let mineral_mesh = meshes.add(Cuboid::new(0.32, 0.28, 0.32));
    let mineral_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.52, 0.48),
        perceptual_roughness: 0.6,
        ..default()
    });
    for y in 0..world.height {
        for x in 0..world.width {
            if world.has_vegetation(x, y) {
                let tag = world.vegetation_species(x, y);
                let color = match tag {
                    3 => Color::srgb(0.75, 0.22, 0.18), // mushroom toxic
                    4 => Color::srgb(0.45, 0.15, 0.55), // nightshade
                    5 => Color::srgb(0.32, 0.22, 0.12), // tree
                    _ => Color::srgb(0.18, 0.62, 0.22),
                };
                commands.spawn((
                    Mesh3d(veg_mesh.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        perceptual_roughness: 0.8,
                        ..default()
                    })),
                    Transform::from_translation(resource_world_pos(world, x, y, 0.25)),
                ));
            }
            if world.animal_count_at(x, y) > 0 {
                commands.spawn((
                    Mesh3d(animal_mesh.clone()),
                    MeshMaterial3d(animal_mat.clone()),
                    Transform::from_translation(resource_world_pos(world, x, y, 0.35)),
                ));
            }
            if world.fish_count_at(x, y) > 0 {
                commands.spawn((
                    Mesh3d(fish_mesh.clone()),
                    MeshMaterial3d(fish_mat.clone()),
                    Transform::from_translation(resource_world_pos(world, x, y, 0.15)),
                ));
            }
            if world.has_mineral(x, y) {
                commands.spawn((
                    Mesh3d(mineral_mesh.clone()),
                    MeshMaterial3d(mineral_mat.clone()),
                    Transform::from_translation(resource_world_pos(world, x, y, 0.18)),
                ));
            }
        }
    }

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

fn sync_agent_transforms(state: Res<SimState>, mut query: Query<(&AgentVisual, &mut Transform)>) {
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

fn update_vision_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimState>,
    existing: Query<Entity, With<VisionOverlay>>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }
    let Some(id) = state.follow else {
        return;
    };
    let Some(agent) = state.sim.agents.get(&id) else {
        return;
    };
    let vis = if state.sim.config.observation.full_information {
        state.sim.world.width.max(state.sim.world.height)
    } else {
        effective_range(
            state.sim.config.observation.base_vision_range,
            agent.personality.perceptiveness,
        )
    };
    let mesh = meshes.add(Cuboid::new(0.92, 0.04, 0.92));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.92, 0.2, 0.22),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let x0 = agent.x.saturating_sub(vis);
    let x1 = (agent.x + vis).min(state.sim.world.width.saturating_sub(1));
    let z0 = agent.y.saturating_sub(vis);
    let z1 = (agent.y + vis).min(state.sim.world.height.saturating_sub(1));
    for x in x0..=x1 {
        for z in z0..=z1 {
            if chebyshev(agent.x, agent.y, x, z) != vis {
                continue;
            }
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(resource_world_pos(&state.sim.world, x, z, 0.08)),
                VisionOverlay,
            ));
        }
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
    let needs = if let Some(id) = state.follow {
        state.sim.agents.get(&id).map(|a| {
            let known = a
                .memory
                .iter()
                .filter(|m| m.species_tag != 0)
                .map(|m| m.species_tag)
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let goals = a
                .goals
                .iter()
                .map(|g| g.text.as_str())
                .take(2)
                .collect::<Vec<_>>()
                .join(" | ");
            format!(
                "h:{:.0} t:{:.0} e:{:.0} ill:{} veg:{} an:{} fi:{} tox:{} known:{} goals:{}",
                a.needs.hunger as f32 / 100.0,
                a.needs.thirst as f32 / 100.0,
                a.needs.energy as f32 / 100.0,
                a.illness_ticks,
                a.consumption.vegetation,
                a.consumption.animal,
                a.consumption.fish,
                a.consumption.toxic_events,
                known,
                if goals.is_empty() { "-" } else { &goals }
            )
        })
    } else {
        None
    };
    let open_board = state.sim.board.open().count();
    let last_line = state
        .sim
        .events
        .events
        .iter()
        .rev()
        .find_map(|e| match &e.kind {
            sim_core::SimEventKind::Speak { text, .. } => Some(format!("said: {text}")),
            _ => None,
        })
        .unwrap_or_default();
    let text = format!(
        "tick {}  {}  {}  hash {}\nwater {}  veg {}  mineral {}  animals {}  fish {}  open proposals {}\n{}\n{}\nSpace pause  . step  F follow  0-9 follow agent",
        state.sim.tick,
        paused,
        follow,
        short,
        state.sim.world.water_count(),
        state.sim.world.vegetation_count(),
        state.sim.world.mineral_count(),
        state.sim.world.animal_total(),
        state.sim.world.fish_total(),
        open_board,
        needs.unwrap_or_else(|| "follow an agent for needs".into()),
        last_line,
    );
    for mut hud in &mut query {
        *hud = Text::new(text.clone());
    }
}
