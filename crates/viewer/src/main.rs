mod render;

use bevy::prelude::*;
use render::{agent_world_pos, heightmap_mesh, resource_world_pos};
use sim_bevy::{SimPlugin, SimState, step_once};
use sim_core::markers::{self, MarkerShape, MarkerSpec};
use sim_core::observation::{chebyshev, effective_range};
use sim_core::{AgentId, ExperimentConfig, ItemId, Simulation};
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
struct LegendText;

#[derive(Component)]
struct VisionOverlay;

#[derive(Resource)]
struct LegendOn(bool);

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
        .insert_resource(LegendOn(true))
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

    let mut mesh_cache: std::collections::HashMap<u8, Handle<Mesh>> =
        std::collections::HashMap::new();
    for y in 0..world.height {
        for x in 0..world.width {
            if world.has_vegetation(x, y) {
                let spec = markers::marker_for_veg(world.vegetation_species(x, y));
                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    spec,
                    resource_world_pos(world, x, y, 0.25),
                );
            }
            if world.crops.contains_key(&(x, y)) {
                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_crop(),
                    resource_world_pos(world, x, y, 0.22),
                );
            }
            if world.animal_count_at(x, y) > 0 {
                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_hare(),
                    resource_world_pos(world, x, y, 0.35),
                );
            }
            if world.fish_count_at(x, y) > 0 {
                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_perch(),
                    resource_world_pos(world, x, y, 0.15),
                );
            }
            if world.has_mineral(x, y) {
                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_mineral(),
                    resource_world_pos(world, x, y, 0.18),
                );
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
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0),
            left: Val::Px(12.0),
            ..default()
        },
        LegendText,
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

fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut std::collections::HashMap<u8, Handle<Mesh>>,
    spec: MarkerSpec,
    pos: Vec3,
) {
    let color = {
        let [r, g, b] = markers::rgb_f32(spec.rgb);
        Color::srgb(r, g, b)
    };
    let mat = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.7,
        ..default()
    });
    let key = spec.shape as u8;
    let mesh = cache
        .entry(key)
        .or_insert_with(|| meshes.add(mesh_for_shape(spec.shape)))
        .clone();
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos),
    ));
    if spec.shape == MarkerShape::Mushroom {
        let stem = cache
            .entry(200)
            .or_insert_with(|| meshes.add(Cylinder::new(0.06, 0.22)))
            .clone();
        commands.spawn((
            Mesh3d(stem),
            MeshMaterial3d(mat),
            Transform::from_translation(pos + Vec3::new(0.0, -0.12, 0.0)),
        ));
    }
}

fn mesh_for_shape(shape: MarkerShape) -> Mesh {
    match shape {
        MarkerShape::Sphere => Sphere::new(0.22).into(),
        MarkerShape::Capsule => Capsule3d::new(0.08, 0.32).into(),
        MarkerShape::Cylinder => Cylinder::new(0.12, 0.7).into(),
        MarkerShape::Cube => Cuboid::new(0.32, 0.28, 0.32).into(),
        MarkerShape::LongCuboid => Cuboid::new(0.22, 0.22, 0.38).into(),
        MarkerShape::FlatCuboid => Cuboid::new(0.18, 0.12, 0.28).into(),
        MarkerShape::Mushroom => Sphere::new(0.2).into(),
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SimState>,
    mut legend: ResMut<LegendOn>,
) {
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
    }
    if keys.just_pressed(KeyCode::Period) {
        step_once(&mut state);
    }
    if keys.just_pressed(KeyCode::KeyL) {
        legend.0 = !legend.0;
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

fn item_label(item: ItemId, species: &sim_core::species::SpeciesTables) -> String {
    match item {
        ItemId::Food(100) => "hare".into(),
        ItemId::Food(101) => "perch".into(),
        ItemId::Food(tag) => species
            .veg(tag)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| format!("food:{tag}")),
        ItemId::Wood => "wood".into(),
        ItemId::Fiber => "fiber".into(),
        ItemId::Stone => "stone".into(),
        ItemId::Basket => "basket".into(),
        ItemId::Spear => "spear".into(),
        ItemId::FishingRod => "fishing_rod".into(),
    }
}

fn update_hud(
    state: Res<SimState>,
    legend: Res<LegendOn>,
    mut hud_q: Query<&mut Text, (With<HudText>, Without<LegendText>)>,
    mut legend_q: Query<&mut Text, (With<LegendText>, Without<HudText>)>,
) {
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
            let mean_trust = if a.relationships.is_empty() {
                0.0
            } else {
                a.relationships.values().map(|r| r.trust as f32 / 100.0).sum::<f32>()
                    / a.relationships.len() as f32
            };
            let inv = a
                .inventory
                .iter()
                .map(|(item, n)| {
                    format!("{}×{n}", item_label(*item, &state.sim.config.world.species))
                })
                .collect::<Vec<_>>()
                .join(",");
            let branch = state
                .sim
                .last_tick_decisions
                .iter()
                .rev()
                .find(|d| d.agent == a.id.0)
                .map(|d| d.policy_branch.as_str())
                .unwrap_or("-");
            format!(
                "h:{:.0} t:{:.0} e:{:.0} ill:{} veg:{} an:{} fi:{} tox:{} known:{} rel:{} mean_trust:{:.1} branch:{} inv:{} goals:{}",
                a.needs.hunger as f32 / 100.0,
                a.needs.thirst as f32 / 100.0,
                a.needs.energy as f32 / 100.0,
                a.illness_ticks,
                a.consumption.vegetation,
                a.consumption.animal,
                a.consumption.fish,
                a.consumption.toxic_events,
                known,
                a.relationships.len(),
                mean_trust,
                branch,
                if inv.is_empty() { "-" } else { &inv },
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
        "tick {}  {}  {}  hash {}\nwater {}  veg {}  mineral {}  animals {}  fish {}  open proposals {}\n{}\n{}\nSpace pause  . step  F follow  L legend  0-9 follow agent",
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
    for mut hud in &mut hud_q {
        *hud = Text::new(text.clone());
    }
    let legend_txt = if legend.0 {
        let mut lines = vec!["legend".to_string()];
        lines.extend(markers::legend_entries().into_iter().map(|(name, shape, _)| {
            format!("{}  {}", markers::shape_name(shape), name)
        }));
        lines.join("\n")
    } else {
        "legend off (L)".into()
    };
    for mut node in &mut legend_q {
        *node = Text::new(legend_txt.clone());
    }
}
