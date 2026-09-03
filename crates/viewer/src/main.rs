mod commands;
mod net;
mod render;
mod ui;

use bevy::prelude::*;
use commands::{CkptScrubber, crate_fill_scale, pack_fill_scale};
use render::{agent_world_pos, heightmap_mesh, resource_world_pos};
use shared::protocol::ClientMessage;
use sim_bevy::{SimPlugin, SimState, step_once};
use sim_core::markers::{self, MarkerShape, MarkerSpec};
use sim_core::observation::{self, chebyshev, effective_range};
use sim_core::combat_fx::CombatFxJob;
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::env;
use std::path::{Path, PathBuf};
use ui::UiState;

const CAPSULE_RADIUS: f32 = 0.28;
const CAPSULE_LENGTH: f32 = 0.55;

#[derive(Component)]
struct AgentVisual {
    id: AgentId,
}

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct VisionOverlay;

#[derive(Component)]
struct WorldMarker {
    x: u32,
    y: u32,
}

#[derive(Component)]
struct StockpileVisual {
    x: u32,
    y: u32,
}

#[derive(Component)]
struct SatchelVisual {
    id: AgentId,
    backpack: bool,
}

#[derive(Component)]
struct CombatFxVisual {
    job: CombatFxJob,
    x: u32,
    y: u32,
    id: AgentId,
}

const SATCHEL_OFFSET: Vec3 = Vec3::new(0.22, 0.16, -0.10);
const BACKPACK_OFFSET: Vec3 = Vec3::new(-0.18, 0.22, -0.12);

fn main() {
    let parsed = parse_args();
    let mut net_link = None;
    let mut scrub = CkptScrubber::default();
    let plugin = match parsed {
        ViewerSource::Config(path) => {
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!("failed to read {}: {e}", path.display());
            });
            let config = ExperimentConfig::from_toml_str(&text).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            let mut sim = Simulation::new(config).unwrap_or_else(|e| {
                panic!("failed to start sim: {e}");
            });
            sim.storage = sim_core::StorageParams::from_config_toml(&text).unwrap_or_else(|e| {
                panic!("failed to parse [storage]: {e}");
            });
            sim.voting = sim_core::VotingParams::from_config_toml(&text).unwrap_or_else(|e| {
                panic!("failed to parse [voting]: {e}");
            });
            let barrier = sim_core::LlmBarrierParams::from_config_toml(&text);
            sim.llm_barrier = barrier.barrier;
            sim.llm_barrier_retries = barrier.retries;
            sim.llm_reflect_on_evict = barrier.reflect_on_evict;
            sim.llm_reflect_every_n = barrier.reflect_every_n_ticks;
            sim.llm_plan_every_n = barrier.plan_every_n_ticks;
            sim.llm_plan_length = barrier.plan_length;
            sim.llm_execute_plan = barrier.execute_plan;
            let conflict = sim_core::ConflictParams::from_config_toml(&text);
            sim.conflict_enabled = conflict.enabled;
            sim.conflict_death_enabled = conflict.death_enabled;
            SimPlugin::from_simulation(sim)
        }
        ViewerSource::Checkpoint(path) => {
            let load_path = CkptScrubber::initial_path(&path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            let sim = Simulation::load_checkpoint(&load_path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", load_path.display());
            });
            scrub = CkptScrubber::discover(&path);
            SimPlugin::from_simulation(sim)
        }
        ViewerSource::Connect { url, token } => {
            let (sim, link) = net::connect(&url, token).unwrap_or_else(|e| {
                panic!("failed to connect to {url}: {e}");
            });
            net_link = Some(link);
            SimPlugin::from_remote(sim)
        }
    };

    let _ = std::fs::create_dir_all(ui::ui_layout_dir());

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "AgentTown viewer".into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(plugin)
    .add_plugins(bevy_mod_imgui::ImguiPlugin {
        ini_filename: Some(ui::imgui_ini_path()),
        ..Default::default()
    })
    .init_resource::<UiState>()
    .insert_resource(scrub);
    if let Some(link) = net_link {
        app.insert_resource(link);
    }
    app.add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                net::apply_remote,
                handle_input,
                sync_agent_transforms,
                sync_combat_tints,
                sync_combat_fx,
                sync_stockpile_markers,
                sync_satchel_markers,
                update_camera,
                update_vision_overlay,
                update_fog_visibility,
                ui::imgui_ui,
            )
                .chain(),
        )
        .add_systems(Last, ui::persist_ui_on_exit)
        .run();
}

enum ViewerSource {
    Config(PathBuf),
    Checkpoint(PathBuf),
    Connect { url: String, token: Option<String> },
}

fn parse_args() -> ViewerSource {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    let mut config = None;
    let mut load = None;
    let mut connect = None;
    let mut token = None;
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
            "--connect" => {
                if let Some(url) = args.get(i + 1) {
                    connect = Some(url.clone());
                    i += 2;
                    continue;
                }
            }
            "--token" => {
                if let Some(t) = args.get(i + 1) {
                    token = Some(t.clone());
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if let Some(url) = connect {
        ViewerSource::Connect { url, token }
    } else if let Some(path) = load {
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
                    x,
                    y,
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
                    x,
                    y,
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
                    x,
                    y,
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
                    x,
                    y,
                );
            }
            if world.has_stockpile(x, y) {
                spawn_stockpile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    world,
                    x,
                    y,
                    stockpile_scale(world, x, y),
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
                    x,
                    y,
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
            Visibility::default(),
        ));
        let params = state.sim.storage;
        if agent.worn_baskets(&params) > 0 {
            spawn_satchel(
                &mut commands,
                &mut meshes,
                &mut materials,
                world,
                agent.id,
                agent.x,
                agent.y,
                false,
                satchel_scale(agent, &params),
            );
        }
        if agent.worn_backpacks(&params) > 0 {
            spawn_satchel(
                &mut commands,
                &mut meshes,
                &mut materials,
                world,
                agent.id,
                agent.x,
                agent.y,
                true,
                satchel_scale(agent, &params),
            );
        }
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
}

fn spawn_stockpile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut std::collections::HashMap<u8, Handle<Mesh>>,
    world: &sim_core::World,
    x: u32,
    y: u32,
    scale: f32,
) {
    let spec = markers::marker_stockpile();
    let pos = resource_world_pos(world, x, y, 0.28);
    let color = {
        let [r, g, b] = markers::rgb_f32(spec.rgb);
        Color::srgb(r, g, b)
    };
    let mat = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.75,
        ..default()
    });
    let key = 210u8;
    let mesh = cache
        .entry(key)
        .or_insert_with(|| meshes.add(mesh_for_shape(spec.shape)))
        .clone();
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
        WorldMarker { x, y },
        StockpileVisual { x, y },
        Visibility::default(),
    ));
}

fn stockpile_scale(world: &sim_core::World, x: u32, y: u32) -> f32 {
    match world.stockpile_at(x, y) {
        Some(c) => crate_fill_scale(c.slot_count(), c.weight_milli()),
        None => 0.40,
    }
}

fn sync_stockpile_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimState>,
    mut existing: Query<(Entity, &StockpileVisual, &mut Transform)>,
) {
    let live: std::collections::BTreeSet<(u32, u32)> = state
        .sim
        .world
        .stockpiles
        .iter()
        .filter(|(_, c)| !c.is_empty())
        .map(|(k, _)| *k)
        .collect();
    let have: std::collections::BTreeSet<(u32, u32)> =
        existing.iter().map(|(_, v, _)| (v.x, v.y)).collect();
    for (e, v, mut tf) in existing.iter_mut() {
        if !live.contains(&(v.x, v.y)) {
            commands.entity(e).despawn();
        } else {
            let s = stockpile_scale(&state.sim.world, v.x, v.y);
            tf.scale = Vec3::splat(s);
        }
    }
    let mut cache = std::collections::HashMap::new();
    for (x, y) in live {
        if have.contains(&(x, y)) {
            continue;
        }
        spawn_stockpile(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut cache,
            &state.sim.world,
            x,
            y,
            stockpile_scale(&state.sim.world, x, y),
        );
    }
}

fn satchel_scale(agent: &sim_core::Agent, params: &sim_core::StorageParams) -> f32 {
    let (slots, w) = agent.worn_pack_caps(params);
    pack_fill_scale(agent.pack_count(), agent.pack_weight_milli(), slots, w)
}

fn spawn_satchel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world: &sim_core::World,
    id: AgentId,
    x: u32,
    y: u32,
    backpack: bool,
    scale: f32,
) {
    let spec = if backpack {
        markers::marker_backpack()
    } else {
        markers::marker_satchel()
    };
    let [r, g, b] = markers::rgb_f32(spec.rgb);
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        perceptual_roughness: 0.7,
        ..default()
    });
    let pos = agent_world_pos(world, x, y)
        + if backpack {
            BACKPACK_OFFSET
        } else {
            SATCHEL_OFFSET
        };
    let size = if backpack {
        Cuboid::new(0.22, 0.26, 0.16)
    } else {
        Cuboid::new(0.16, 0.18, 0.12)
    };
    commands.spawn((
        Mesh3d(meshes.add(size)),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
        SatchelVisual { id, backpack },
        Visibility::default(),
    ));
}

fn sync_satchel_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimState>,
    existing: Query<(Entity, &SatchelVisual)>,
) {
    let params = state.sim.storage;
    let mut live: std::collections::BTreeSet<(AgentId, bool)> = std::collections::BTreeSet::new();
    for a in state.sim.agents.values() {
        if a.worn_baskets(&params) > 0 {
            live.insert((a.id, false));
        }
        if a.worn_backpacks(&params) > 0 {
            live.insert((a.id, true));
        }
    }
    let have: std::collections::BTreeSet<(AgentId, bool)> =
        existing.iter().map(|(_, v)| (v.id, v.backpack)).collect();
    for (e, v) in existing.iter() {
        if !live.contains(&(v.id, v.backpack)) {
            commands.entity(e).despawn();
        }
    }
    for (id, backpack) in live {
        if have.contains(&(id, backpack)) {
            continue;
        }
        let Some(agent) = state.sim.agents.get(&id) else {
            continue;
        };
        spawn_satchel(
            &mut commands,
            &mut meshes,
            &mut materials,
            &state.sim.world,
            id,
            agent.x,
            agent.y,
            backpack,
            satchel_scale(agent, &params),
        );
    }
}

fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut std::collections::HashMap<u8, Handle<Mesh>>,
    spec: MarkerSpec,
    pos: Vec3,
    x: u32,
    y: u32,
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
        WorldMarker { x, y },
        Visibility::default(),
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
            WorldMarker { x, y },
            Visibility::default(),
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
    mut ui: ResMut<UiState>,
    mut scrub: ResMut<CkptScrubber>,
    net: Option<Res<net::NetLink>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        ui.save_layout();
        exit.write(AppExit::Success);
        return;
    }
    if ui.want_keyboard {
        return;
    }
    if keys.just_pressed(KeyCode::Slash) || keys.just_pressed(KeyCode::Backquote) {
        ui.windows.console = true;
        ui.request_console_focus = true;
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
        if state.remote {
            let verb = if state.paused {
                shared::protocol::ControlVerb::Pause
            } else {
                shared::protocol::ControlVerb::Play
            };
            if let Some(net) = net.as_ref() {
                let _ = net.tx.send(ClientMessage::Control(verb));
            }
        }
    }
    if keys.just_pressed(KeyCode::Period) {
        if state.remote {
            if let Some(net) = net.as_ref() {
                let _ = net
                    .tx
                    .send(ClientMessage::Control(shared::protocol::ControlVerb::Step(
                        1,
                    )));
            }
        } else {
            step_once(&mut state);
        }
    }
    if keys.just_pressed(KeyCode::KeyL) {
        ui.windows.legend = !ui.windows.legend;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        ui.windows.help = !ui.windows.help;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        ui.windows.inspector = !ui.windows.inspector;
    }
    if keys.just_pressed(KeyCode::KeyB) {
        ui.windows.board = !ui.windows.board;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        ui.fog = !ui.fog;
    }
    if !state.remote && scrub.dir.is_some() {
        if keys.just_pressed(KeyCode::BracketLeft) {
            match scrub.prev(&mut state) {
                Ok(t) => ui.scrollback.push(format!("loaded tick {t}")),
                Err(e) => ui.scrollback.push(format!("ckpt error: {e}")),
            }
        }
        if keys.just_pressed(KeyCode::BracketRight) {
            match scrub.next(&mut state) {
                Ok(t) => ui.scrollback.push(format!("loaded tick {t}")),
                Err(e) => ui.scrollback.push(format!("ckpt error: {e}")),
            }
        }
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

fn last_agent_cell(
    sim: &Simulation,
    cache: &mut std::collections::BTreeMap<AgentId, (u32, u32)>,
    id: AgentId,
) -> (u32, u32) {
    if let Some(a) = sim.agents.get(&id) {
        cache.insert(id, (a.x, a.y));
        (a.x, a.y)
    } else {
        cache.get(&id).copied().unwrap_or((0, 0))
    }
}

fn spawn_combat_fx(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    world: &sim_core::World,
    job: CombatFxJob,
    cache: &mut std::collections::BTreeMap<AgentId, (u32, u32)>,
    sim: &Simulation,
) {
    let (mesh, color, transform, x, y, id) = match job {
        CombatFxJob::Strike { from, to } => {
            let (ax, ay) = last_agent_cell(sim, cache, from);
            let (bx, by) = last_agent_cell(sim, cache, to);
            let a = agent_world_pos(world, ax, ay);
            let b = agent_world_pos(world, bx, by);
            let mid = (a + b) * 0.5 + Vec3::Y * 0.25;
            let len = (b - a).length().max(0.15);
            let mut t = Transform::from_translation(mid).looking_at(b + Vec3::Y * 0.25, Vec3::Y);
            t.scale = Vec3::new(0.08, 0.08, len);
            (
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                Color::srgb(0.92, 0.18, 0.10),
                t,
                ax,
                ay,
                from,
            )
        }
        CombatFxJob::Flee { agent } => {
            let (x, y) = last_agent_cell(sim, cache, agent);
            let pos = agent_world_pos(world, x, y) + Vec3::Y * 0.55;
            (
                Mesh3d(meshes.add(Sphere::new(0.16))),
                Color::srgb(0.92, 0.82, 0.18),
                Transform::from_translation(pos),
                x,
                y,
                agent,
            )
        }
        CombatFxJob::Downed { agent } => {
            let (x, y) = last_agent_cell(sim, cache, agent);
            let pos = agent_world_pos(world, x, y) + Vec3::new(0.0, -0.15, 0.0);
            (
                Mesh3d(meshes.add(Cuboid::new(0.55, 0.12, 0.55))),
                Color::srgb(0.18, 0.10, 0.10),
                Transform::from_translation(pos),
                x,
                y,
                agent,
            )
        }
        CombatFxJob::Death { agent } => {
            let (x, y) = last_agent_cell(sim, cache, agent);
            let pos = agent_world_pos(world, x, y) + Vec3::Y * 0.35;
            (
                Mesh3d(meshes.add(Cuboid::new(0.12, 0.85, 0.12))),
                Color::srgb(0.95, 0.45, 0.75),
                Transform::from_translation(pos),
                x,
                y,
                agent,
            )
        }
    };
    let mat = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.45,
        unlit: true,
        ..default()
    });
    commands.spawn((
        mesh,
        MeshMaterial3d(mat),
        transform,
        CombatFxVisual { job, x, y, id },
        Visibility::default(),
    ));
}

fn sync_combat_fx(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<SimState>,
    existing: Query<(Entity, &CombatFxVisual)>,
    mut last_cell: Local<std::collections::BTreeMap<AgentId, (u32, u32)>>,
) {
    for a in state.sim.agents.values() {
        last_cell.insert(a.id, (a.x, a.y));
    }
    let live: std::collections::BTreeSet<CombatFxJob> =
        sim_core::combat_fx_jobs(&state.sim.events.events, state.sim.tick)
            .into_iter()
            .collect();
    let have: std::collections::BTreeSet<CombatFxJob> =
        existing.iter().map(|(_, v)| v.job).collect();
    for (e, v) in existing.iter() {
        if !live.contains(&v.job) {
            commands.entity(e).despawn();
        }
    }
    for job in live {
        if have.contains(&job) {
            continue;
        }
        spawn_combat_fx(
            &mut commands,
            &mut meshes,
            &mut materials,
            &state.sim.world,
            job,
            &mut last_cell,
            &state.sim,
        );
    }
}

fn sync_combat_tints(
    state: Res<SimState>,
    agents: Query<(&AgentVisual, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tick = state.sim.tick;
    let events = &state.sim.events.events;
    for (visual, handle) in &agents {
        let Some(mut mat) = materials.get_mut(&handle.0) else {
            continue;
        };
        let hue = (visual.id.0 as f32 * 47.0) % 360.0;
        let base = Color::hsl(hue, 0.7, 0.55);
        mat.base_color = match sim_core::combat_fx::combat_role(events, tick, visual.id) {
            sim_core::combat_fx::CombatRole::Attacker => Color::srgb(0.85, 0.15, 0.12),
            sim_core::combat_fx::CombatRole::Defender => Color::srgb(0.25, 0.12, 0.12),
            sim_core::combat_fx::CombatRole::Flee => Color::srgb(0.85, 0.75, 0.2),
            sim_core::combat_fx::CombatRole::None => base,
        };
    }
}

fn sync_agent_transforms(
    state: Res<SimState>,
    mut agents: Query<(&AgentVisual, &mut Transform)>,
    mut satchels: Query<(&SatchelVisual, &mut Transform), Without<AgentVisual>>,
) {
    for (visual, mut transform) in &mut agents {
        if let Some(agent) = state.sim.agents.get(&visual.id) {
            transform.translation = agent_world_pos(&state.sim.world, agent.x, agent.y);
        }
    }
    for (visual, mut transform) in &mut satchels {
        if let Some(agent) = state.sim.agents.get(&visual.id) {
            transform.translation = agent_world_pos(&state.sim.world, agent.x, agent.y)
                + if visual.backpack {
                    BACKPACK_OFFSET
                } else {
                    SATCHEL_OFFSET
                };
            transform.scale = Vec3::splat(satchel_scale(agent, &state.sim.storage));
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

fn update_fog_visibility(
    state: Res<SimState>,
    ui: Res<UiState>,
    mut markers: Query<(&WorldMarker, &mut Visibility), Without<AgentVisual>>,
    mut agents: Query<(&AgentVisual, &mut Visibility), Without<WorldMarker>>,
    mut satchels: Query<
        (&SatchelVisual, &mut Visibility),
        (Without<WorldMarker>, Without<AgentVisual>),
    >,
    mut fx: Query<
        (&CombatFxVisual, &mut Visibility),
        (Without<WorldMarker>, Without<AgentVisual>, Without<SatchelVisual>),
    >,
) {
    let fog_obs = if ui.fog {
        state.follow.map(|id| observation::build(&state.sim, id))
    } else {
        None
    };
    for (marker, mut vis) in &mut markers {
        let show = match &fog_obs {
            None => true,
            Some(obs) => observation::visible_in_observation(obs, marker.x, marker.y),
        };
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (visual, mut vis) in &mut agents {
        let show = match &fog_obs {
            None => true,
            Some(obs) => {
                let pos = state
                    .sim
                    .agents
                    .get(&visual.id)
                    .map(|a| (a.x, a.y))
                    .unwrap_or((0, 0));
                observation::agent_visible_in_observation(obs, visual.id, pos.0, pos.1)
            }
        };
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (visual, mut vis) in &mut satchels {
        let show = match &fog_obs {
            None => true,
            Some(obs) => {
                let pos = state
                    .sim
                    .agents
                    .get(&visual.id)
                    .map(|a| (a.x, a.y))
                    .unwrap_or((0, 0));
                observation::agent_visible_in_observation(obs, visual.id, pos.0, pos.1)
            }
        };
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (visual, mut vis) in &mut fx {
        let show = match &fog_obs {
            None => true,
            Some(obs) => {
                observation::agent_visible_in_observation(obs, visual.id, visual.x, visual.y)
            }
        };
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
