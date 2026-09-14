//! M40 object definitions: config-owned recipes; visual/LOD hash-neutral.

use sim_core::action::{PrimaryAction, Recipe};
use sim_core::agent::ItemId;
use sim_core::objects::{
    CatalogParams, ObjectDef, catalog_entries, default_objects_dir, load_object_defs, lod_band,
    pick_visual_path,
};
use sim_core::observation::legal_actions;
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::fs;
use std::path::{Path, PathBuf};

/// `--no-time` shipped-objects 2-tick (M53 catalog: tent sleep + cabin/house).
const IDLE_2_NO_TIME: &str = "6e8b124a46573e18e60f946950922305d60a54b9cbdcf74ff9ebe12234e4c8d5";
/// `--no-time` no-catalog 2-tick (M51 identity).
const IDLE_2_NO_CATALOG_NO_TIME: &str =
    "70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc";
/// Default (time on) shipped-objects 2-tick.
const IDLE_2: &str = "3a294816b006d2e1f62f8a07da79d45caa7db921e88ede000c77827ffba56103";
/// Default (time on) no-catalog 2-tick.
const IDLE_2_NO_CATALOG: &str =
    "9c3b270de2658f24531f05220ec4a40313f3859db1ded003a63b681106882131";

fn shipped_objects() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects")
}

fn apply_shipped(sim: &mut Simulation) {
    sim.apply_objects_dir(&shipped_objects()).unwrap();
}

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
fn default_objects_dir_finds_shipping_from_other_cwd() {
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let dir = default_objects_dir();
    let _ = std::env::set_current_dir(prev);
    let dir = dir.expect("shipping configs/objects via crate path");
    let defs = load_object_defs(&dir).unwrap();
    assert!(
        defs.iter().any(|d| d.id == "berry_bush"),
        "expected berry_bush in {}",
        dir.display()
    );
}

#[test]
fn default_mock_two_ticks_idle_hash() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let cfg = ExperimentConfig::load_path(&path).unwrap();
    let mut sim = Simulation::new(cfg).unwrap();
    apply_shipped(&mut sim);
    assert!(sim.time_enabled);
    sim.run_ticks(2);
    assert_eq!(sim.state_hash().to_string(), IDLE_2);
}

#[test]
fn no_time_shipped_objects_idle_hash() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let cfg = ExperimentConfig::load_path(&path).unwrap();
    let mut sim = Simulation::new(cfg).unwrap();
    apply_shipped(&mut sim);
    sim.time_enabled = false;
    sim.run_ticks(2);
    assert_eq!(sim.state_hash().to_string(), IDLE_2_NO_TIME);
}

#[test]
fn catalog_off_two_ticks_hash_ignores_new_recipes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let cfg = ExperimentConfig::load_path(&path).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    off.time_enabled = false;
    off.run_ticks(2);
    assert_eq!(off.state_hash().to_string(), IDLE_2_NO_CATALOG_NO_TIME);
}

#[test]
fn catalog_off_time_on_two_ticks_idle_hash() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");
    let cfg = ExperimentConfig::load_path(&path).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    off.run_ticks(2);
    assert_eq!(off.state_hash().to_string(), IDLE_2_NO_CATALOG);
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
        scale: None,
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
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Basket
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
    assert_eq!(entries[0].item, ItemId::Catalog(0));
}

#[test]
fn no_hardcoded_recipes_without_files() {
    let mut sim = Simulation::new(tiny(0x40_01)).unwrap();
    if let Some(a) = sim.agents.get_mut(&AgentId(0)) {
        a.try_add_item(ItemId::Fiber, 8);
        a.try_add_item(ItemId::Wood, 2);
        a.try_add_item(ItemId::Stone, 2);
    }
    let legal = legal_actions(&sim, sim.agents.get(&AgentId(0)).unwrap());
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Basket
                | Recipe::Spear
                | Recipe::FishingRod
                | Recipe::Backpack
                | Recipe::Catalog(_)
        }
    )));
}

#[test]
fn shipped_basket_craft_fiber_two() {
    let mut sim = Simulation::new(tiny(0x40_02)).unwrap();
    apply_shipped(&mut sim);
    let a = AgentId(0);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.try_add_item(ItemId::Fiber, 8);
    }
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(
        legal.iter().any(|x| matches!(
            x,
            PrimaryAction::Craft {
                recipe: Recipe::Basket
            }
        )),
        "{legal:?}"
    );
    for _ in 0..256 {
        let have = sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&ItemId::Basket).copied())
            .unwrap_or(0);
        sim_core::execute::execute_primary(
            &mut sim,
            a,
            &PrimaryAction::Craft {
                recipe: Recipe::Basket,
            },
        );
        let after = sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&ItemId::Basket).copied())
            .unwrap_or(0);
        if after > have {
            assert_eq!(after, 1);
            return;
        }
    }
    panic!("basket craft never succeeded");
}

#[test]
fn override_recipe_changes_hash_and_inputs() {
    let shipped = catalog_entries(&load_object_defs(&shipped_objects()).unwrap());
    let mut changed = shipped.clone();
    let basket = changed.iter_mut().find(|e| e.slug == "basket").unwrap();
    basket.inputs = vec![(ItemId::Wood, 3)];
    let mut a = Simulation::new(tiny(0x40_03)).unwrap();
    let mut b = Simulation::new(tiny(0x40_03)).unwrap();
    a.enable_catalog(shipped);
    b.enable_catalog(changed.clone());
    a.run_ticks(2);
    b.run_ticks(2);
    assert_ne!(a.state_hash(), b.state_hash());
    let spec = sim_core::objects::recipe_spec(Recipe::Basket, &changed).unwrap();
    assert_eq!(spec.0, vec![(ItemId::Wood, 3)]);
    assert_eq!(spec.1, ItemId::Basket);
}

#[test]
fn load_basket_restores_no_double_grant() {
    let mut sim = Simulation::new(tiny(0x40_04)).unwrap();
    apply_shipped(&mut sim);
    let a = AgentId(0);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.try_add_item(ItemId::Fiber, 8);
    }
    for _ in 0..256 {
        sim_core::execute::execute_primary(
            &mut sim,
            a,
            &PrimaryAction::Craft {
                recipe: Recipe::Basket,
            },
        );
        if sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&ItemId::Basket).copied())
            .unwrap_or(0)
            >= 1
        {
            break;
        }
    }
    assert_eq!(
        sim.agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Basket)
            .copied()
            .unwrap_or(0),
        1
    );
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    apply_shipped(&mut loaded);
    assert_eq!(
        loaded
            .agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Basket)
            .copied()
            .unwrap_or(0),
        1,
        "load must not re-grant"
    );
}

#[test]
fn builtin_slugs_are_not_catalog_u16() {
    let entries = catalog_entries(&load_object_defs(&shipped_objects()).unwrap());
    let basket = entries.iter().find(|e| e.slug == "basket").unwrap();
    assert_eq!(basket.item, ItemId::Basket);
    assert_eq!(basket.recipe, Some(Recipe::Basket));
    let cord = entries.iter().find(|e| e.slug == "cord").unwrap();
    let ItemId::Catalog(n) = cord.item else {
        panic!("cord should be catalog");
    };
    assert_eq!(cord.recipe, Some(Recipe::Catalog(n)));
    for slug in ["plank", "charcoal", "knife", "net"] {
        let e = entries.iter().find(|x| x.slug == slug).unwrap();
        assert!(
            matches!(e.item, ItemId::Catalog(_)),
            "{slug} should be catalog"
        );
        assert!(e.recipe.is_some(), "{slug} should have a recipe");
    }
}

#[test]
fn shipped_species_sim_matches_defaults() {
    let defs = load_object_defs(&shipped_objects()).unwrap();
    let mut tables = sim_core::species::default_species_tables();
    let before = tables.clone();
    sim_core::apply_species_defs(&mut tables, &defs);
    assert_eq!(tables.vegetation.len(), before.vegetation.len());
    for (a, b) in tables.vegetation.iter().zip(before.vegetation.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.nutrition, b.nutrition);
        assert_eq!(a.toxicity, b.toxicity);
        assert_eq!(a.fiber_yield, b.fiber_yield);
        assert_eq!(a.wood_yield, b.wood_yield);
        assert_eq!(a.grow_ticks, b.grow_ticks);
    }
    assert_eq!(tables.animals[0].id, "hare");
    assert_eq!(tables.animals[0].nutrition, 30.0);
    assert_eq!(tables.fish[0].id, "perch");
    assert_eq!(tables.fish[0].nutrition, 25.0);
}

#[test]
fn missing_species_sim_keeps_rust_nutrition() {
    let dir = std::env::temp_dir().join("agentplace-m41-visual-only-veg");
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
    let mut tables = sim_core::species::default_species_tables();
    sim_core::apply_species_defs(&mut tables, &defs);
    assert_eq!(tables.veg(1).unwrap().nutrition, 20.0);
    assert_eq!(tables.veg(1).unwrap().id, "berry_bush");
}

#[test]
fn extra_vegetation_appends_tag_and_changes_hash() {
    let dir = std::env::temp_dir().join("agentplace-m41-extra-veg");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "apple.toml",
        r#"
id = "apple"
kind = "vegetation"
[sim]
yield = "food"
nutrition = 18.0
toxicity = "safe"
fiber_yield = 0
grow_ticks = 25
"#,
    );
    let defs = load_object_defs(&dir).unwrap();
    let mut cfg = tiny(0x41_02);
    sim_core::apply_species_defs(&mut cfg.world.species, &defs);
    assert_eq!(cfg.world.species.vegetation.len(), 6);
    assert_eq!(cfg.world.species.veg(1).unwrap().id, "berry_bush");
    assert_eq!(cfg.world.species.veg(5).unwrap().id, "tree");
    assert_eq!(cfg.world.species.veg(6).unwrap().id, "apple");
    let extra = Simulation::new(cfg).unwrap();
    let base = Simulation::new(tiny(0x41_02)).unwrap();
    assert_eq!(extra.config.world.species.vegetation.len(), 6);
    assert_ne!(extra.state_hash(), base.state_hash());
}

#[test]
fn override_nutrition_changes_hash_after_eat() {
    let dir = std::env::temp_dir().join("agentplace-m41-override-nut");
    let _ = fs::remove_dir_all(&dir);
    write_toml(
        &dir,
        "berry_bush.toml",
        r#"
id = "berry_bush"
kind = "vegetation"
[sim]
yield = "food"
nutrition = 80.0
toxicity = "safe"
fiber_yield = 1
grow_ticks = 40
"#,
    );
    let mut shipped = Simulation::new(tiny(0x41_03)).unwrap();
    apply_shipped(&mut shipped);
    let mut over = Simulation::new(tiny(0x41_03)).unwrap();
    over.apply_objects_dir(&dir).unwrap();
    force_gather_eat(&mut shipped, 1);
    force_gather_eat(&mut over, 1);
    assert_ne!(shipped.state_hash(), over.state_hash());
}

fn force_gather_eat(sim: &mut Simulation, tag: u8) {
    let a = AgentId(0);
    let (x, y) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    let mut placed = false;
    for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if sim.world.in_bounds(nx, ny) && sim.world.is_land(nx as u32, ny as u32) {
            sim.world.set_vegetation(nx as u32, ny as u32, tag);
            placed = true;
            break;
        }
    }
    assert!(placed, "no land neighbor for gather");
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(
            sim.config.hunger_max_milli(),
            sim.config.thirst_max_milli(),
            sim.config.energy_max_milli(),
        );
        ag.abilities.gather = 100;
    }
    for _ in 0..64 {
        sim_core::execute::execute_primary(sim, a, &PrimaryAction::Gather { species: tag });
        if sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&ItemId::Food(tag)).copied())
            .unwrap_or(0)
            > 0
        {
            break;
        }
    }
    assert!(
        sim.agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&ItemId::Food(tag)).copied())
            .unwrap_or(0)
            > 0,
        "gather never succeeded"
    );
    sim_core::execute::execute_primary(
        sim,
        a,
        &PrimaryAction::Eat {
            item: ItemId::Food(tag),
        },
    );
}

#[test]
fn write_ckpt_is_format_version_3() {
    use sim_core::CHECKPOINT_FORMAT_VERSION;
    assert_eq!(CHECKPOINT_FORMAT_VERSION, 3);
    let bytes = Simulation::new(tiny(0x45_10))
        .unwrap()
        .encode_checkpoint()
        .unwrap();
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 3);
}

#[test]
fn load_v2_catalog_u16_still_decodes() {
    let mut sim = Simulation::new(tiny(0x45_11)).unwrap();
    let a = AgentId(0);
    sim.agents
        .get_mut(&a)
        .unwrap()
        .try_add_item(ItemId::Stone, 3);
    let mut bytes = sim.encode_checkpoint().unwrap();
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(
        loaded
            .agents
            .get(&a)
            .unwrap()
            .inventory
            .get(&ItemId::Stone)
            .copied(),
        Some(3)
    );
}

#[test]
fn load_v3_catalog_slug_survives_extra_file() {
    let dir_a = std::env::temp_dir().join("agentplace-m45-slug-a");
    let dir_b = std::env::temp_dir().join("agentplace-m45-slug-b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    write_toml(
        &dir_a,
        "alpha.toml",
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
    write_toml(
        &dir_a,
        "zeta.toml",
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
    let mut sim = Simulation::new(tiny(0x45_12)).unwrap();
    sim.apply_objects_dir(&dir_a).unwrap();
    let zeta = sim.catalog.iter().find(|e| e.slug == "zeta").unwrap().item;
    let ItemId::Catalog(zeta_n) = zeta else {
        panic!("zeta should be catalog");
    };
    sim.agents
        .get_mut(&AgentId(0))
        .unwrap()
        .try_add_item(zeta, 1);
    let bytes = sim.encode_checkpoint().unwrap();
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 3);

    write_toml(
        &dir_b,
        "alpha.toml",
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
    write_toml(
        &dir_b,
        "mid.toml",
        r#"
id = "mid"
kind = "item"
[sim]
weight_milli = 200
[sim.craft]
inputs = [["fiber", 2]]
output_qty = 1
"#,
    );
    write_toml(
        &dir_b,
        "zeta.toml",
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
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.apply_objects_dir(&dir_b).unwrap();
    let new_zeta = loaded
        .catalog
        .iter()
        .find(|e| e.slug == "zeta")
        .unwrap()
        .item;
    assert_ne!(new_zeta, ItemId::Catalog(zeta_n), "rank shuffled");
    assert_eq!(
        loaded
            .agents
            .get(&AgentId(0))
            .unwrap()
            .inventory
            .get(&new_zeta)
            .copied(),
        Some(1),
        "slug remap keeps zeta"
    );
    assert_eq!(
        loaded
            .agents
            .get(&AgentId(0))
            .unwrap()
            .inventory
            .get(&ItemId::Catalog(zeta_n))
            .copied()
            .unwrap_or(0),
        0
    );
}

#[test]
fn catalog_off_plank_not_legal_same_hash() {
    let cfg = tiny(0x46_10);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut flagged = Simulation::new(cfg).unwrap();
    let a = AgentId(0);
    for sim in [&mut off, &mut flagged] {
        if let Some(ag) = sim.agents.get_mut(&a) {
            ag.try_add_item(ItemId::Wood, 4);
            ag.try_add_item(ItemId::Fiber, 4);
            ag.try_add_item(ItemId::Stone, 2);
        }
    }
    let legal = legal_actions(&off, off.agents.get(&a).unwrap());
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Catalog(_)
        }
    )));
    off.run_ticks(3);
    flagged.run_ticks(3);
    assert_eq!(off.state_hash(), flagged.state_hash());
}

#[test]
fn catalog_on_craft_plank() {
    let mut sim = Simulation::new(tiny(0x46_11)).unwrap();
    apply_shipped(&mut sim);
    let a = AgentId(0);
    let plank = sim.catalog.iter().find(|e| e.slug == "plank").unwrap().item;
    let ItemId::Catalog(n) = plank else {
        panic!("plank should be catalog");
    };
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.try_add_item(ItemId::Wood, 8);
    }
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(
        legal.iter().any(|x| matches!(
            x,
            PrimaryAction::Craft {
                recipe: Recipe::Catalog(k)
            } if *k == n
        )),
        "{legal:?}"
    );
    for _ in 0..64 {
        if let Some(ag) = sim.agents.get_mut(&a) {
            ag.try_add_item(ItemId::Wood, 4);
        }
        sim_core::execute::execute_primary(
            &mut sim,
            a,
            &PrimaryAction::Craft {
                recipe: Recipe::Catalog(n),
            },
        );
        if sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&plank).copied())
            .unwrap_or(0)
            >= 1
        {
            return;
        }
    }
    panic!("plank craft never succeeded");
}

#[test]
fn catalog_off_hammer_not_legal_same_hash() {
    let cfg = tiny(0x49_10);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let a = AgentId(0);
    if let Some(ag) = off.agents.get_mut(&a) {
        ag.try_add_item(ItemId::Stone, 4);
        ag.try_add_item(ItemId::Wood, 4);
        ag.try_add_item(ItemId::Fiber, 4);
        ag.try_add_item(ItemId::Food(1), 2);
    }
    let legal = legal_actions(&off, off.agents.get(&a).unwrap());
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Catalog(_)
        }
    )));
}

fn craft_catalog_slug(sim: &mut Simulation, slug: &str, stock: &[(ItemId, u32)]) {
    let item = sim
        .catalog
        .iter()
        .find(|e| e.slug == slug)
        .unwrap_or_else(|| panic!("{slug} missing"))
        .item;
    let ItemId::Catalog(n) = item else {
        panic!("{slug} should be catalog");
    };
    let a = AgentId(0);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.abilities.craft = 100;
        ag.inventory_cap = 32;
        for (it, q) in stock {
            ag.try_add_item(*it, *q);
        }
    }
    for _ in 0..64 {
        sim_core::execute::execute_primary(
            sim,
            a,
            &PrimaryAction::Craft {
                recipe: Recipe::Catalog(n),
            },
        );
        if sim
            .agents
            .get(&a)
            .and_then(|ag| ag.inventory.get(&item).copied())
            .unwrap_or(0)
            >= 1
        {
            return;
        }
    }
    panic!("{slug} craft never succeeded");
}

#[test]
fn catalog_on_craft_hammer() {
    let mut sim = Simulation::new(tiny(0x49_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "hammer", &[(ItemId::Stone, 4), (ItemId::Wood, 4)]);
}

#[test]
fn catalog_on_craft_dried_fish() {
    let mut sim = Simulation::new(tiny(0x49_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "dried_fish", &[(ItemId::Food(1), 4)]);
}

#[test]
fn catalog_on_craft_club() {
    let mut sim = Simulation::new(tiny(0x50_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "club", &[(ItemId::Wood, 4)]);
}

#[test]
fn catalog_on_craft_satchel() {
    let mut sim = Simulation::new(tiny(0x50_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "satchel", &[(ItemId::Fiber, 4), (ItemId::Wood, 4)]);
}

#[test]
fn catalog_on_craft_pike() {
    let mut sim = Simulation::new(tiny(0x50_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "pike", &[(ItemId::Wood, 4), (ItemId::Stone, 4)]);
}

#[test]
fn catalog_off_new_m50_crafts_not_legal() {
    let cfg = tiny(0x50_10);
    let mut off = Simulation::new(cfg).unwrap();
    let a = AgentId(0);
    if let Some(ag) = off.agents.get_mut(&a) {
        ag.try_add_item(ItemId::Stone, 4);
        ag.try_add_item(ItemId::Wood, 4);
        ag.try_add_item(ItemId::Fiber, 4);
        ag.try_add_item(ItemId::Food(1), 2);
    }
    let legal = legal_actions(&off, off.agents.get(&a).unwrap());
    assert!(!legal.iter().any(|x| matches!(
        x,
        PrimaryAction::Craft {
            recipe: Recipe::Catalog(_)
        }
    )));
}

#[test]
fn catalog_on_craft_tent() {
    let mut sim = Simulation::new(tiny(0x51_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "tent",
        &[(ItemId::Fiber, 6), (ItemId::Wood, 4)],
    );
}

#[test]
fn catalog_on_craft_stew() {
    let mut sim = Simulation::new(tiny(0x51_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "stew", &[(ItemId::Food(1), 4)]);
}

#[test]
fn catalog_on_craft_cabin() {
    let mut sim = Simulation::new(tiny(0x53_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "cabin",
        &[(ItemId::Wood, 8), (ItemId::Stone, 4)],
    );
}

#[test]
fn catalog_on_craft_house() {
    let mut sim = Simulation::new(tiny(0x53_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "house",
        &[(ItemId::Wood, 12), (ItemId::Stone, 6), (ItemId::Fiber, 4)],
    );
}
