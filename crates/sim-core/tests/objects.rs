//! M39 object definitions: visual/LOD hash-neutral; hashed catalog Craft.

use sim_core::action::{PrimaryAction, Recipe};
use sim_core::agent::ItemId;
use sim_core::objects::{
    CatalogParams, ObjectDef, catalog_entries, load_object_defs, lod_band, pick_visual_path,
};
use sim_core::observation::legal_actions;
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::fs;
use std::path::{Path, PathBuf};

const IDLE_2: &str = "70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc";

fn tiny(seed: u64) -> ExperimentConfig {
    ExperimentConfig::from_toml_str(&format!(
        r#"
master_seed = {seed}
[simulation]
max_ticks = 1000
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#
    ))
    .unwrap()
}

fn write_toml(dir: &Path, name: &str, body: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    path
}

fn touch(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"not-a-real-glb").unwrap();
}

fn force_craft_catalog(sim: &mut Simulation, id: AgentId, n: u16) {
    if let Some(a) = sim.agents.get_mut(&id) {
        a.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        a.try_add_item(ItemId::Fiber, 8);
    }
    for _ in 0..256 {
        let have = sim
            .agents
            .get(&id)
            .and_then(|a| a.inventory.get(&ItemId::Catalog(n)).copied())
            .unwrap_or(0);
        sim_core::execute::execute_primary(
            sim,
            id,
            &PrimaryAction::Craft {
                recipe: Recipe::Catalog(n),
            },
        );
        let after = sim
            .agents
            .get(&id)
            .and_then(|a| a.inventory.get(&ItemId::Catalog(n)).copied())
            .unwrap_or(0);
        if after > have {
            return;
        }
    }
    panic!("catalog craft never succeeded");
}

#[test]
fn default_mock_two_ticks_idle_hash() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let cfg = ExperimentConfig::load_path(&path).unwrap();
    let mut sim = Simulation::new(cfg).unwrap();
    sim.run_ticks(2);
    assert_eq!(sim.state_hash().to_string(), IDLE_2);
}

#[test]
fn overlay_parses_catalog() {
    let on = CatalogParams::from_config_toml("[catalog]\nenabled = true\n");
    assert!(on.enabled);
    let off = CatalogParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!off.enabled);
}

#[test]
fn visual_only_toml_hash_unchanged() {
    let cfg = tiny(0x39_01);
    let mut none = Simulation::new(cfg.clone()).unwrap();
    none.run_ticks(3);
    let dir = std::env::temp_dir().join("agentplace-m39-visual-only");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "berry_bush.toml",
        r#"
id = "berry_bush"
kind = "vegetation"
[visual]
glb = "assets/models/missing.glb"
"#,
    );
    let defs = load_object_defs(&dir).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].id, "berry_bush");
    assert!(defs[0].visual.as_ref().unwrap().glb.is_some());
    let mut with_files = Simulation::new(cfg).unwrap();
    let _ = catalog_entries(&defs);
    with_files.run_ticks(3);
    assert_eq!(none.state_hash(), with_files.state_hash());
}

#[test]
fn lod_picks_near_mid_far_and_missing_mid() {
    assert_eq!(lod_band(0), "near");
    assert_eq!(lod_band(7), "near");
    assert_eq!(lod_band(8), "mid");
    assert_eq!(lod_band(23), "mid");
    assert_eq!(lod_band(24), "far");

    let root = std::env::temp_dir().join("agentplace-m39-lod");
    let _ = fs::remove_dir_all(&root);
    let near = root.join("near.glb");
    let mid = root.join("mid.glb");
    let far = root.join("far.glb");
    let glb = root.join("base.glb");
    touch(&near);
    touch(&mid);
    touch(&far);
    touch(&glb);
    let visual = sim_core::VisualDef {
        glb: Some(glb.to_string_lossy().into()),
        lod: sim_core::objects::LodDef {
            near: Some(near.to_string_lossy().into()),
            mid: Some(mid.to_string_lossy().into()),
            far: Some(far.to_string_lossy().into()),
        },
    };
    assert_eq!(
        pick_visual_path(&visual, 0).as_deref(),
        Some(near.as_path())
    );
    assert_eq!(
        pick_visual_path(&visual, 10).as_deref(),
        Some(mid.as_path())
    );
    assert_eq!(
        pick_visual_path(&visual, 40).as_deref(),
        Some(far.as_path())
    );

    fs::remove_file(&mid).unwrap();
    assert_eq!(
        pick_visual_path(&visual, 10).as_deref(),
        Some(far.as_path()),
        "missing mid falls back to far"
    );
    fs::remove_file(&far).unwrap();
    assert_eq!(
        pick_visual_path(&visual, 10).as_deref(),
        Some(glb.as_path()),
        "missing mid+far falls back to glb"
    );
    fs::remove_file(&glb).unwrap();
    assert!(
        pick_visual_path(&visual, 10).is_none(),
        "missing files ⇒ primitive"
    );
}

#[test]
fn catalog_off_not_legal_same_hash() {
    let dir = std::env::temp_dir().join("agentplace-m39-catalog-off");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "cord.toml",
        r#"
id = "cord"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    let defs = load_object_defs(&dir).unwrap();
    let entries = catalog_entries(&defs);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].slug, "cord");

    let cfg = tiny(0x39_02);
    let mut legal_sim = Simulation::new(cfg.clone()).unwrap();
    if let Some(a) = legal_sim.agents.get_mut(&AgentId(0)) {
        a.try_add_item(ItemId::Fiber, 4);
    }
    let legal = legal_actions(&legal_sim, legal_sim.agents.get(&AgentId(0)).unwrap());
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Catalog(_)
        }
    )));

    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut files_not_enabled = Simulation::new(cfg.clone()).unwrap();
    let _ = load_object_defs(&dir).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_catalog(entries);
    off.run_ticks(4);
    files_not_enabled.run_ticks(4);
    assert_eq!(off.state_hash(), files_not_enabled.state_hash());
    on.run_ticks(4);
    assert_ne!(
        off.state_hash(),
        on.state_hash(),
        "catalog on with item files changes hash"
    );
}

#[test]
fn catalog_on_craft_produces_item() {
    let dir = std::env::temp_dir().join("agentplace-m39-catalog-on");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "cord.toml",
        r#"
id = "cord"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    let entries = catalog_entries(&load_object_defs(&dir).unwrap());
    let mut sim = Simulation::new(tiny(0x39_03)).unwrap();
    sim.enable_catalog(entries);
    let a = AgentId(0);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.try_add_item(ItemId::Fiber, 4);
    }
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(
        legal.iter().any(|x| matches!(
            x,
            PrimaryAction::Craft {
                recipe: Recipe::Catalog(0)
            }
        )),
        "{legal:?}"
    );
    force_craft_catalog(&mut sim, a, 0);
    assert_eq!(
        sim.agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Catalog(0))
            .copied()
            .unwrap_or(0),
        1
    );
}

#[test]
fn catalog_slug_sort_stable_u16() {
    let dir = std::env::temp_dir().join("agentplace-m39-slug-sort");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "z_item.toml",
        r#"
id = "zeta"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    write_toml(
        &dir,
        "a_item.toml",
        r#"
id = "alpha"
kind = "item"
[sim]
weight_milli = 100
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    let entries = catalog_entries(&load_object_defs(&dir).unwrap());
    assert_eq!(
        entries.iter().map(|e| e.slug.as_str()).collect::<Vec<_>>(),
        vec!["alpha", "zeta"]
    );
    let reversed = std::env::temp_dir().join("agentplace-m39-slug-sort-rev");
    let _ = fs::remove_dir_all(&reversed);
    write_toml(
        &reversed,
        "00_zeta.toml",
        r#"
id = "zeta"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    write_toml(
        &reversed,
        "99_alpha.toml",
        r#"
id = "alpha"
kind = "item"
[sim]
weight_milli = 100
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    let again = catalog_entries(&load_object_defs(&reversed).unwrap());
    assert_eq!(again[0].slug, "alpha");
    assert_eq!(again[1].slug, "zeta");
    assert_eq!(again[0].weight_milli, 100);
}

#[test]
fn catalog_load_restores_inventory_no_double_grant() {
    let dir = std::env::temp_dir().join("agentplace-m39-load");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "cord.toml",
        r#"
id = "cord"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    let entries = catalog_entries(&load_object_defs(&dir).unwrap());
    let mut sim = Simulation::new(tiny(0x39_04)).unwrap();
    sim.enable_catalog(entries.clone());
    force_craft_catalog(&mut sim, AgentId(0), 0);
    let qty = sim
        .agents
        .get(&AgentId(0))
        .unwrap()
        .inventory
        .get(&ItemId::Catalog(0))
        .copied()
        .unwrap_or(0);
    assert_eq!(qty, 1);
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.enable_catalog(entries);
    assert_eq!(
        loaded
            .agents
            .get(&AgentId(0))
            .unwrap()
            .inventory
            .get(&ItemId::Catalog(0))
            .copied()
            .unwrap_or(0),
        1,
        "load must not re-grant"
    );
}

#[test]
fn empty_catalog_on_same_hash_as_off() {
    let cfg = tiny(0x39_05);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.enable_catalog(Vec::new());
    off.run_ticks(3);
    on.run_ticks(3);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn shipping_objects_visual_not_hashed() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects");
    let defs = load_object_defs(&dir).unwrap();
    assert!(defs.iter().any(|d| d.id == "berry_bush"));
    assert!(defs.iter().any(|d| d.id == "cord" && d.kind == "item"));
    let cfg = tiny(0x39_06);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    let _ = catalog_entries(&defs);
    a.run_ticks(2);
    b.run_ticks(2);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn parse_locked_craft_inputs() {
    let def: ObjectDef = toml::from_str(
        r#"
id = "cord"
kind = "item"
[sim]
weight_milli = 400
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    )
    .unwrap();
    let entries = catalog_entries(&[def]);
    assert_eq!(entries[0].inputs, vec![(ItemId::Fiber, 2)]);
    assert_eq!(entries[0].output_qty, 1);
}
