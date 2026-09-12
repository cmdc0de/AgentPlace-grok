mod camera;
mod charts;
mod commands;
mod models;
mod net;
mod render;
mod ui;

use bevy::asset::AssetPlugin;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use commands::{CkptScrubber, crate_fill_scale, pack_fill_scale};
use render::{agent_world_pos, heightmap_mesh, resource_world_pos};
use shared::protocol::ClientMessage;
use sim_bevy::{SimPlugin, SimState, step_once};
use sim_core::combat_fx::CombatFxJob;
use sim_core::markers::{self, MarkerShape, MarkerSpec};
use sim_core::observation::{self, chebyshev};
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::env;
use std::path::{Path, PathBuf};
use ui::UiState;

#[derive(Clone, Copy, Debug)]
struct AgentFit {
    scale: f32,
    offset: Vec3,
}

#[derive(Component)]
struct AgentVisual {
    id: AgentId,
    /// Set once the authored glb AABB is known so height matches the capsule.
    fit: Option<AgentFit>,
    /// Explicit `[visual] scale`. When set, skip capsule auto-fit.
    toml_scale: Option<f32>,
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

#[derive(Resource, Default, Clone)]
struct ObjectVisuals {
    defs: Vec<sim_core::ObjectDef>,
    sentinel_mesh: Option<Handle<Mesh>>,
    sentinel_mat: Option<Handle<StandardMaterial>>,
}

#[derive(Component, Clone)]
struct ModelLabel {
    id: String,
    path: PathBuf,
    file_bytes: u64,
    mtime: Option<std::time::SystemTime>,
}

#[derive(Resource, Default)]
struct ReportedModels {
    files: std::collections::HashSet<PathBuf>,
    sizes: std::collections::HashSet<PathBuf>,
}

fn apply_viewer_objects(
    sim: &mut Simulation,
    objects: Option<&Path>,
    _catalog_flag: bool,
    _config_text: Option<&str>,
) -> Vec<sim_core::ObjectDef> {
    let (dir, defs) = models::load_viewer_objects(objects);
    match &dir {
        None => {
            eprintln!("objects: no configs/objects dir (cwd or --objects); using primitives");
        }
        Some(d) if defs.is_empty() => {
            eprintln!(
                "objects: 0 defs from {} (check TOML); using primitives",
                d.display()
            );
        }
        Some(d) => {
            eprintln!("objects: {} defs from {}", defs.len(), d.display());
            for id in ["berry_bush", "tree", "hare", "crate", "basket", "agent"] {
                match models::resolve_visual(&defs, id, 0) {
                    Some(p) => eprintln!("  glb {id} -> {}", p.display()),
                    None => eprintln!("  glb {id} -> (primitive)"),
                }
            }
        }
    }
    sim.enable_catalog(sim_core::catalog_entries(&defs));
    defs
}

fn main() {
    let parsed = parse_args();
    let mut net_link = None;
    let mut scrub = CkptScrubber::default();
    let (plugin, object_defs) = match parsed.source {
        ViewerSource::Config(path) => {
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!("failed to read {}: {e}", path.display());
            });
            let mut config = ExperimentConfig::from_toml_str(&text).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            let dir = parsed
                .objects
                .clone()
                .or_else(sim_core::objects::default_objects_dir);
            if let Some(dir) = &dir {
                if let Ok(defs) = sim_core::load_object_defs(dir) {
                    sim_core::apply_species_defs(&mut config.world.species, &defs);
                }
            }
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
            sim.llm_reflect_importance = barrier.reflect_importance;
            let conflict = sim_core::ConflictParams::from_config_toml(&text);
            sim.conflict_enabled = conflict.enabled;
            sim.conflict_death_enabled = conflict.death_enabled;
            if sim_core::SheetParams::from_config_toml(&text).enabled {
                sim.enable_sheet();
            }
            let pop = sim_core::PopulationParams::from_config_toml(&text);
            if pop.reproduction {
                sim.enable_reproduction();
            }
            if pop.aging {
                sim.enable_aging(pop.childhood_ticks, pop.founder_age_ticks);
            }
            if pop.household_crates {
                sim.enable_household_crates();
            }
            if pop.culture {
                sim.enable_culture(pop.culture_count);
            }
            let inv = sim_core::InventionsParams::from_config_toml(&text);
            if inv.enabled {
                sim.enable_inventions(inv.share_delay_ticks);
                sim.invention_tree = inv.tree;
                sim.invention_patent_ticks = inv.patent_ticks;
            }
            sim.pipeline_hash_events =
                sim_core::PipelineParams::from_config_toml(&text).hash_events;
            let tel = sim_core::TelemetryParams::from_config_toml(&text);
            sim.telemetry_enabled = tel.enabled;
            sim.telemetry_otlp_endpoint = tel.otlp_endpoint;
            let defs = apply_viewer_objects(
                &mut sim,
                parsed.objects.as_deref(),
                parsed.catalog,
                Some(&text),
            );
            (SimPlugin::from_simulation(sim), defs)
        }
        ViewerSource::Checkpoint(path) => {
            let load_path = CkptScrubber::initial_path(&path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
            let mut sim = Simulation::load_checkpoint(&load_path).unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", load_path.display());
            });
            let defs =
                apply_viewer_objects(&mut sim, parsed.objects.as_deref(), parsed.catalog, None);
            scrub = CkptScrubber::discover(&path);
            (SimPlugin::from_simulation(sim), defs)
        }
        ViewerSource::Connect { url, token } => {
            let (mut sim, link) = net::connect(&url, token).unwrap_or_else(|e| {
                panic!("failed to connect to {url}: {e}");
            });
            let defs =
                apply_viewer_objects(&mut sim, parsed.objects.as_deref(), parsed.catalog, None);
            net_link = Some(link);
            (SimPlugin::from_remote(sim), defs)
        }
    };

    let _ = std::fs::create_dir_all(ui::ui_layout_dir());

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "AgentTown viewer".into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                // Object TOML points at repo `assets/models/…` (outside crates/viewer/assets).
                unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                ..default()
            }),
    )
    .add_plugins(plugin)
    .add_plugins(bevy_mod_imgui::ImguiPlugin {
        ini_filename: Some(ui::imgui_ini_path()),
        ..Default::default()
    })
    .init_resource::<UiState>()
    .init_resource::<ui::ClickThroughGuard>()
    .init_resource::<ReportedModels>()
    .insert_resource(ObjectVisuals {
        defs: object_defs,
        ..Default::default()
    })
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
                sync_combat_tints,
                sync_combat_fx,
                sync_stockpile_markers,
                sync_satchel_markers,
                reload_changed_glbs,
                report_model_sizes,
                fit_agent_meshes,
                sync_agent_transforms,
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

struct ViewerArgs {
    source: ViewerSource,
    objects: Option<PathBuf>,
    catalog: bool,
}

fn parse_args() -> ViewerArgs {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    let mut config = None;
    let mut load = None;
    let mut connect = None;
    let mut token = None;
    let mut objects = None;
    let mut catalog = false;
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
            "--objects" => {
                if let Some(path) = args.get(i + 1) {
                    objects = Some(PathBuf::from(path));
                    i += 2;
                    continue;
                }
            }
            "--catalog" => {
                catalog = true;
                i += 1;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    let source = if let Some(url) = connect {
        ViewerSource::Connect { url, token }
    } else if let Some(path) = load {
        ViewerSource::Checkpoint(path)
    } else {
        ViewerSource::Config(config.unwrap_or_else(default_config_path))
    };
    ViewerArgs {
        source,
        objects,
        catalog,
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
    assets: Res<AssetServer>,
    state: Res<SimState>,
    mut visuals: ResMut<ObjectVisuals>,
) {
    visuals.sentinel_mesh = Some(meshes.add(Cuboid::new(0.4, 0.4, 0.4)));
    visuals.sentinel_mat = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 1.0),
        perceptual_roughness: 0.4,
        ..default()
    }));
    let world = &state.sim.world;
    eprintln!(
        "setup_scene: {}×{} veg={} animals={} fish={} agents={} object_defs={}",
        world.width,
        world.height,
        world.vegetation_count(),
        world.animal_total(),
        world.fish_total(),
        state.sim.agents.len(),
        visuals.defs.len()
    );
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
                    &assets,
                    &visuals,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    spec,
                    spec.name,
                    resource_world_pos(world, x, y, 0.25),
                    x,
                    y,
                    models::camera_dist_cells(world.width, world.height, x, y),
                );
            }
            if world.crops.contains_key(&(x, y)) {
                spawn_marker(
                    &mut commands,
                    &assets,
                    &visuals,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_crop(),
                    "crop",
                    resource_world_pos(world, x, y, 0.22),
                    x,
                    y,
                    models::camera_dist_cells(world.width, world.height, x, y),
                );
            }
            if world.animal_count_at(x, y) > 0 {
                spawn_marker(
                    &mut commands,
                    &assets,
                    &visuals,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_hare(),
                    "hare",
                    resource_world_pos(world, x, y, 0.35),
                    x,
                    y,
                    models::camera_dist_cells(world.width, world.height, x, y),
                );
            }
            if world.fish_count_at(x, y) > 0 {
                spawn_marker(
                    &mut commands,
                    &assets,
                    &visuals,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_perch(),
                    "perch",
                    resource_world_pos(world, x, y, 0.15),
                    x,
                    y,
                    models::camera_dist_cells(world.width, world.height, x, y),
                );
            }
            if world.has_stockpile(x, y) {
                spawn_stockpile(
                    &mut commands,
                    &assets,
                    &visuals,
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
                    &assets,
                    &visuals,
                    &mut meshes,
                    &mut materials,
                    &mut mesh_cache,
                    markers::marker_mineral(),
                    "stone",
                    resource_world_pos(world, x, y, 0.18),
                    x,
                    y,
                    models::camera_dist_cells(world.width, world.height, x, y),
                );
            }
        }
    }

    let capsule = meshes.add(Capsule3d::new(
        models::AGENT_CAPSULE_RADIUS,
        models::AGENT_CAPSULE_LENGTH,
    ));
    for agent in state.sim.agents.values() {
        let hue = (agent.id.0 as f32 * 47.0) % 360.0;
        let color = Color::hsl(hue, 0.7, 0.55);
        let tf = Transform::from_translation(agent_world_pos(world, agent.x, agent.y));
        if !try_spawn_model(
            &mut commands,
            &assets,
            &visuals,
            "agent",
            models::camera_dist_cells(world.width, world.height, agent.x, agent.y),
            tf,
            AgentVisual {
                id: agent.id,
                fit: None,
                toml_scale: sim_core::visual_scale_for(&visuals.defs, "agent"),
            },
        ) {
            commands.spawn((
                Mesh3d(capsule.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    perceptual_roughness: 0.5,
                    ..default()
                })),
                tf,
                AgentVisual {
                    id: agent.id,
                    fit: None,
                    toml_scale: None,
                },
                Visibility::default(),
            ));
        }
        let params = state.sim.storage;
        if agent.worn_baskets(&params) > 0 {
            spawn_satchel(
                &mut commands,
                &assets,
                &visuals,
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
                &assets,
                &visuals,
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
    assets: &AssetServer,
    visuals: &ObjectVisuals,
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
    let tf = Transform::from_translation(pos).with_scale(Vec3::splat(scale));
    if try_spawn_model(
        commands,
        assets,
        visuals,
        "crate",
        models::camera_dist_cells(world.width, world.height, x, y),
        tf,
        (WorldMarker { x, y }, StockpileVisual { x, y }),
    ) {
        return;
    }
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
    assets: Res<AssetServer>,
    state: Res<SimState>,
    visuals: Res<ObjectVisuals>,
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
            &assets,
            &visuals,
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
    assets: &AssetServer,
    visuals: &ObjectVisuals,
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
    let stem = if backpack { "backpack" } else { "basket" };
    if try_spawn_model(
        commands,
        assets,
        visuals,
        stem,
        models::camera_dist_cells(world.width, world.height, x, y),
        Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
        SatchelVisual { id, backpack },
    ) {
        return;
    }
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
    assets: Res<AssetServer>,
    state: Res<SimState>,
    visuals: Res<ObjectVisuals>,
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
            &assets,
            &visuals,
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

fn log_sentinel_once(id: &str) {
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    let mut seen = SEEN
        .get_or_init(|| Mutex::new(std::collections::HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if seen.insert(id.to_string()) {
        eprintln!("glb miss {id} -> sentinel");
    }
}

fn try_spawn_model(
    commands: &mut Commands,
    assets: &AssetServer,
    visuals: &ObjectVisuals,
    stem: &str,
    dist_cells: u32,
    transform: Transform,
    extra: impl Bundle,
) -> bool {
    match models::resolve_visual_kind(&visuals.defs, stem, dist_cells) {
        models::VisualKind::Authored(path) => {
            let meta = std::fs::metadata(&path).ok();
            let file_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.and_then(|m| m.modified().ok());
            let mut tf = transform;
            if let Some(s) = sim_core::visual_scale_for(&visuals.defs, stem) {
                tf.scale = Vec3::splat(s);
            }
            let handle = assets
                .load_builder()
                .override_unapproved()
                .load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
            commands.spawn((
                WorldAssetRoot(handle),
                tf,
                extra,
                ModelLabel {
                    id: stem.to_string(),
                    path,
                    file_bytes,
                    mtime,
                },
                Visibility::default(),
            ));
            true
        }
        models::VisualKind::Sentinel => {
            let (Some(mesh), Some(mat)) =
                (visuals.sentinel_mesh.clone(), visuals.sentinel_mat.clone())
            else {
                return false;
            };
            log_sentinel_once(stem);
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                transform,
                extra,
                Visibility::default(),
            ));
            true
        }
        models::VisualKind::Primitive => false,
    }
}

fn reload_changed_glbs(
    time: Res<Time>,
    mut acc: Local<f32>,
    assets: Res<AssetServer>,
    mut labels: Query<(&mut ModelLabel, Option<&mut AgentVisual>)>,
    mut reported: ResMut<ReportedModels>,
    mut ui: ResMut<UiState>,
) {
    *acc += time.delta_secs();
    if *acc < 0.5 {
        return;
    }
    *acc = 0.0;
    for (mut label, agent) in &mut labels {
        let Ok(meta) = std::fs::metadata(&label.path) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if !models::should_reload(label.mtime, Some(mtime)) {
            continue;
        }
        assets.reload(label.path.clone());
        let line = format!("glb reload {} {}", label.id, label.path.display());
        eprintln!("{line}");
        ui.scrollback.push(line);
        reported.sizes.remove(&label.path);
        if let Some(mut agent) = agent {
            agent.fit = None;
        }
        label.mtime = Some(mtime);
        label.file_bytes = meta.len();
    }
}

fn format_file_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{n} bytes ({:.1} MiB)", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{n} bytes ({:.1} KiB)", n as f64 / 1024.0)
    } else {
        format!("{n} bytes")
    }
}

fn report_model_sizes(
    roots: Query<(Entity, &ModelLabel, &GlobalTransform)>,
    children: Query<&Children>,
    meshes: Query<(&GlobalTransform, &Mesh3d)>,
    assets: Res<Assets<Mesh>>,
    mut reported: ResMut<ReportedModels>,
    mut ui: ResMut<UiState>,
) {
    for (entity, label, root_tf) in &roots {
        if reported.files.insert(label.path.clone()) {
            let line = format!(
                "loaded glb {} {}  {}",
                label.id,
                label.path.display(),
                format_file_bytes(label.file_bytes)
            );
            eprintln!("{line}");
            ui.scrollback.push(line);
        }
        if reported.sizes.contains(&label.path) {
            continue;
        }
        let Some((w, h, d)) = model_size_meters(entity, root_tf, &children, &meshes, &assets)
        else {
            continue;
        };
        reported.sizes.insert(label.path.clone());
        let line = format!(
            "model {} {} size={:.3}×{:.3}×{:.3} m (x×y×z)",
            label.id,
            label.path.display(),
            w,
            h,
            d
        );
        eprintln!("{line}");
        ui.scrollback.push(line);
    }
}

fn model_aabb_local(
    root: Entity,
    root_tf: &GlobalTransform,
    children: &Query<&Children>,
    meshes: &Query<(&GlobalTransform, &Mesh3d)>,
    assets: &Assets<Mesh>,
) -> Option<(Vec3, Vec3)> {
    let inv = root_tf.affine().inverse();
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut found = false;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if let Ok(kids) = children.get(e) {
            stack.extend(kids.iter());
        }
        let Ok((gt, mesh3d)) = meshes.get(e) else {
            continue;
        };
        let Some(mesh) = assets.get(&mesh3d.0) else {
            continue;
        };
        let Some(attr) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            continue;
        };
        let Some(iter) = attr.as_float3() else {
            continue;
        };
        let local = inv * gt.affine();
        for p in iter {
            let v = local.transform_point3(Vec3::from(*p));
            min = min.min(v);
            max = max.max(v);
            found = true;
        }
    }
    if !found {
        return None;
    }
    Some((min, max))
}

fn model_size_meters(
    root: Entity,
    root_tf: &GlobalTransform,
    children: &Query<&Children>,
    meshes: &Query<(&GlobalTransform, &Mesh3d)>,
    assets: &Assets<Mesh>,
) -> Option<(f32, f32, f32)> {
    let (min, max) = model_aabb_local(root, root_tf, children, meshes, assets)?;
    let s = max - min;
    Some((s.x.abs(), s.y.abs(), s.z.abs()))
}

fn fit_agent_meshes(
    mut agents: Query<(Entity, &ModelLabel, &GlobalTransform, &mut AgentVisual)>,
    children: Query<&Children>,
    meshes: Query<(&GlobalTransform, &Mesh3d)>,
    assets: Res<Assets<Mesh>>,
) {
    for (entity, label, root_tf, mut visual) in &mut agents {
        if visual.fit.is_some() || label.id != "agent" || visual.toml_scale.is_some() {
            continue;
        }
        let Some((min, max)) = model_aabb_local(entity, root_tf, &children, &meshes, &assets)
        else {
            continue;
        };
        let (scale, offset) = models::fit_aabb_to_height(
            [min.x, min.y, min.z],
            [max.x, max.y, max.z],
            models::AGENT_CAPSULE_HEIGHT,
        );
        visual.fit = Some(AgentFit {
            scale,
            offset: Vec3::new(offset[0], offset[1], offset[2]),
        });
        eprintln!(
            "agent fit scale={scale:.4} src_height={:.3} -> {}",
            (max.y - min.y).abs(),
            models::AGENT_CAPSULE_HEIGHT
        );
    }
}

fn spawn_marker(
    commands: &mut Commands,
    assets: &AssetServer,
    visuals: &ObjectVisuals,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut std::collections::HashMap<u8, Handle<Mesh>>,
    spec: MarkerSpec,
    stem: &str,
    pos: Vec3,
    x: u32,
    y: u32,
    dist_cells: u32,
) {
    if try_spawn_model(
        commands,
        assets,
        visuals,
        stem,
        dist_cells,
        Transform::from_translation(pos),
        WorldMarker { x, y },
    ) {
        return;
    }
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

fn xz_ground(v: Vec3) -> [f32; 2] {
    let len = (v.x * v.x + v.z * v.z).sqrt();
    if len < 1e-5 {
        [0.0, 1.0]
    } else {
        [v.x / len, v.z / len]
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut guard: ResMut<ui::ClickThroughGuard>,
    mut state: ResMut<SimState>,
    mut ui: ResMut<UiState>,
    mut scrub: ResMut<CkptScrubber>,
    mut cameras: Query<&mut Transform, With<FollowCamera>>,
    net: Option<Res<net::NetLink>>,
    mut exit: MessageWriter<AppExit>,
) {
    let window = windows.single().ok();
    let focused = window.map(|w| w.focused).unwrap_or(false);
    let cursor_in = window.and_then(|w| w.cursor_position()).is_some();
    guard.tick(
        mouse.pressed(MouseButton::Left),
        mouse.just_released(MouseButton::Left),
        focused,
        cursor_in,
    );
    let remote_ready = !state.remote || guard.armed();
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
        if state.remote {
            // Space only pauses an attach. Toggle-to-Play on window-open /
            // leftover key-up used to start a --start-paused server.
            if !state.paused {
                state.paused = true;
                if let Some(net) = net.as_ref() {
                    let _ = net
                        .tx
                        .send(ClientMessage::Control(shared::protocol::ControlVerb::Pause));
                }
            }
        } else {
            state.paused = !state.paused;
        }
    }
    if keys.just_pressed(KeyCode::Period) {
        if state.remote {
            if remote_ready {
                if let Some(net) = net.as_ref() {
                    let _ =
                        net.tx
                            .send(ClientMessage::Control(shared::protocol::ControlVerb::Step(
                                1,
                            )));
                }
            }
        } else {
            step_once(&mut state);
        }
    }
    if keys.just_pressed(KeyCode::KeyL) {
        ui.windows.legend = !ui.windows.legend;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        ui.windows.charts = !ui.windows.charts;
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

    let dt = time.delta_secs();
    let left = camera::pan_step(
        keys.just_pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::ArrowLeft),
        dt,
    );
    let right = camera::pan_step(
        keys.just_pressed(KeyCode::ArrowRight),
        keys.pressed(KeyCode::ArrowRight),
        dt,
    );
    let forward = camera::pan_step(
        keys.just_pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::ArrowUp),
        dt,
    );
    let back = camera::pan_step(
        keys.just_pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::ArrowDown),
        dt,
    );
    let up = camera::pan_step(
        keys.just_pressed(KeyCode::KeyU),
        keys.pressed(KeyCode::KeyU),
        dt,
    );
    let down = camera::pan_step(
        keys.just_pressed(KeyCode::KeyD),
        keys.pressed(KeyCode::KeyD),
        dt,
    );
    if left == 0.0 && right == 0.0 && forward == 0.0 && back == 0.0 && up == 0.0 && down == 0.0 {
        return;
    }
    state.follow = camera::follow_after_pan(state.follow, true);
    for mut transform in &mut cameras {
        let fwd = xz_ground(*transform.forward());
        let rgt = xz_ground(*transform.right());
        let [dx, dz] = camera::pan_xz(fwd, rgt, left, right, forward, back);
        let w = &state.sim.world;
        let max_x = w.width.saturating_sub(1) as f32;
        let max_z = w.height.saturating_sub(1) as f32;
        let cx = transform.translation.x.floor().clamp(0.0, max_x) as u32;
        let cz = transform.translation.z.floor().clamp(0.0, max_z) as u32;
        let min_y = w.height_at(cx, cz) as f32 + camera::HEIGHT_CLEARANCE;
        transform.translation.x += dx;
        transform.translation.z += dz;
        transform.translation.y = camera::height_step(up, down, transform.translation.y, min_y);
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
            let pos = agent_world_pos(&state.sim.world, agent.x, agent.y);
            if let Some(fit) = visual.fit {
                transform.translation = pos + fit.offset;
                transform.scale = Vec3::splat(fit.scale);
            } else if let Some(s) = visual.toml_scale {
                transform.translation = pos;
                transform.scale = Vec3::splat(s);
            } else {
                transform.translation = pos;
                transform.scale = Vec3::ONE;
            }
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
        sim_core::observation::perceive_range_for(
            &state.sim,
            agent,
            state.sim.config.observation.base_vision_range,
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
        (
            Without<WorldMarker>,
            Without<AgentVisual>,
            Without<SatchelVisual>,
        ),
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
