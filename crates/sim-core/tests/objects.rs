//! M40 object definitions: config-owned recipes; visual/LOD hash-neutral.

use sim_core::action::{PrimaryAction, Recipe};
use sim_core::agent::ItemId;
use sim_core::objects::{
    CatalogParams, ObjectDef, catalog_entries, default_objects_dir, farm_skill_bonus,
    fish_skill_bonus, gather_skill_bonus, load_object_defs, lod_band, pick_visual_path,
    stone_gather_skill_bonus,
};
use sim_core::observation::{legal_actions, neighbors4};
use sim_core::{AgentId, ExperimentConfig, Simulation};
use std::fs;
use std::path::{Path, PathBuf};

/// `--no-time` shipped-objects 2-tick (M60 catalog: extra recipes).
const IDLE_2_NO_TIME: &str = "171a26d292fbbad8d62f54c44f059bbc595c758d70473beea51f0702945fdb57";
/// `--no-time` no-catalog 2-tick (M51 identity).
const IDLE_2_NO_CATALOG_NO_TIME: &str =
    "70e5204df22e5bcb44e4d84e6b5886e418e2f275e865029987c21e2d8dbdb7dc";
/// Default (time on) shipped-objects 2-tick.
const IDLE_2: &str = "aacd867dfaafe7a055f194027b0849e286edc5fea24761c889a0948c14592665";
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

#[test]
fn catalog_on_craft_torch() {
    let mut sim = Simulation::new(tiny(0x54_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "torch",
        &[(ItemId::Wood, 2), (ItemId::Fiber, 2)],
    );
}

#[test]
fn catalog_on_craft_axe() {
    let mut sim = Simulation::new(tiny(0x54_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "axe",
        &[(ItemId::Wood, 4), (ItemId::Stone, 2)],
    );
}

#[test]
fn catalog_on_craft_jar() {
    let mut sim = Simulation::new(tiny(0x54_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "jar", &[(ItemId::Stone, 4)]);
}

#[test]
fn catalog_on_craft_bread() {
    let mut sim = Simulation::new(tiny(0x54_14)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "bread", &[(ItemId::Food(1), 4)]);
}

#[test]
fn catalog_on_craft_rope() {
    let mut sim = Simulation::new(tiny(0x55_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "rope", &[(ItemId::Fiber, 8)]);
}

#[test]
fn catalog_on_craft_needle() {
    let mut sim = Simulation::new(tiny(0x55_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "needle",
        &[(ItemId::Stone, 2), (ItemId::Fiber, 2)],
    );
}

#[test]
fn catalog_on_craft_bucket() {
    let mut sim = Simulation::new(tiny(0x55_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "bucket", &[(ItemId::Wood, 4)]);
}

#[test]
fn catalog_on_craft_shield() {
    let mut sim = Simulation::new(tiny(0x55_14)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "shield", &[(ItemId::Wood, 6)]);
}

#[test]
fn catalog_off_farm_fish_identity() {
    let mut sim = Simulation::new(tiny(0x55_20)).unwrap();
    let a = sim.agents.get(&AgentId(0)).unwrap();
    assert_eq!(farm_skill_bonus(a, &[]), 0);
    assert_eq!(fish_skill_bonus(a, &[]), -15);
    sim.agents
        .get_mut(&AgentId(0))
        .unwrap()
        .try_add_item(ItemId::FishingRod, 1);
    let a = sim.agents.get(&AgentId(0)).unwrap();
    assert_eq!(fish_skill_bonus(a, &[]), 25);
}

#[test]
fn hoe_farm_bonus_25_vs_zero() {
    let mut sim = Simulation::new(tiny(0x55_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(farm_skill_bonus(a, &sim.catalog), 0);
    let item = sim
        .catalog
        .iter()
        .find(|e| e.slug == "hoe")
        .unwrap()
        .item;
    sim.agents.get_mut(&id).unwrap().try_add_item(item, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(farm_skill_bonus(a, &sim.catalog), 25);
}

#[test]
fn net_fish_bonus_25_bare_minus_15_rod_net_max() {
    let mut sim = Simulation::new(tiny(0x55_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(fish_skill_bonus(a, &sim.catalog), -15);
    let net = sim
        .catalog
        .iter()
        .find(|e| e.slug == "net")
        .unwrap()
        .item;
    sim.agents.get_mut(&id).unwrap().try_add_item(net, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(fish_skill_bonus(a, &sim.catalog), 25);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::FishingRod, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(fish_skill_bonus(a, &sim.catalog), 25, "rod+net max not sum");
}

#[test]
fn catalog_off_gather_basket_identity() {
    let mut sim = Simulation::new(tiny(0x56_20)).unwrap();
    let a = sim.agents.get(&AgentId(0)).unwrap();
    assert_eq!(gather_skill_bonus(a, &[]), 0);
    sim.agents
        .get_mut(&AgentId(0))
        .unwrap()
        .try_add_item(ItemId::Basket, 1);
    let a = sim.agents.get(&AgentId(0)).unwrap();
    assert_eq!(gather_skill_bonus(a, &[]), 15);
}

#[test]
fn axe_gather_bonus_25_vs_zero_basket_max() {
    let mut sim = Simulation::new(tiny(0x56_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(gather_skill_bonus(a, &sim.catalog), 0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(axe, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(gather_skill_bonus(a, &sim.catalog), 25);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Basket, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(
        gather_skill_bonus(a, &sim.catalog),
        25,
        "basket+axe max not sum"
    );
}

#[test]
fn stone_gather_bonus_stays_zero_with_axe() {
    let mut sim = Simulation::new(tiny(0x56_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(axe, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(stone_gather_skill_bonus(a, &sim.catalog), 0);
}

#[test]
fn catalog_off_stone_gather_identity() {
    let sim = Simulation::new(tiny(0x57_20)).unwrap();
    let a = sim.agents.get(&AgentId(0)).unwrap();
    assert_eq!(stone_gather_skill_bonus(a, &[]), 0);
}

#[test]
fn hammer_stone_gather_bonus_25_max_not_sum() {
    let mut sim = Simulation::new(tiny(0x57_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(stone_gather_skill_bonus(a, &sim.catalog), 0);
    let hammer = sim.catalog.iter().find(|e| e.slug == "hammer").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(hammer, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(stone_gather_skill_bonus(a, &sim.catalog), 25);
    if let Some(e) = sim.catalog.iter_mut().find(|e| e.slug == "knife") {
        e.stone_gather_bonus = 10;
    }
    let knife = sim.catalog.iter().find(|e| e.slug == "knife").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(knife, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(
        stone_gather_skill_bonus(a, &sim.catalog),
        25,
        "hammer+knife max not sum"
    );
}

#[test]
fn vegetation_gather_with_hammer_unchanged() {
    let mut sim = Simulation::new(tiny(0x57_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let hammer = sim.catalog.iter().find(|e| e.slug == "hammer").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(hammer, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(gather_skill_bonus(a, &sim.catalog), 0, "hammer is not vegetation");
    sim.agents
        .get_mut(&id)
        .unwrap()
        .try_add_item(ItemId::Basket, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(gather_skill_bonus(a, &sim.catalog), 15);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(axe, 1);
    let a = sim.agents.get(&id).unwrap();
    assert_eq!(gather_skill_bonus(a, &sim.catalog), 25);
}

#[test]
fn catalog_on_craft_fence() {
    let mut sim = Simulation::new(tiny(0x56_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "fence", &[(ItemId::Wood, 8)]);
}

#[test]
fn catalog_on_craft_mat() {
    let mut sim = Simulation::new(tiny(0x56_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "mat", &[(ItemId::Fiber, 10)]);
}

#[test]
fn catalog_on_craft_snare() {
    let mut sim = Simulation::new(tiny(0x56_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "snare",
        &[(ItemId::Fiber, 4), (ItemId::Wood, 2)],
    );
}

#[test]
fn catalog_on_craft_spit() {
    let mut sim = Simulation::new(tiny(0x56_14)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "spit",
        &[(ItemId::Wood, 4), (ItemId::Fiber, 2)],
    );
}

#[test]
fn catalog_on_craft_barrel() {
    let mut sim = Simulation::new(tiny(0x57_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "barrel", &[(ItemId::Wood, 10)]);
}

#[test]
fn catalog_on_craft_cloak() {
    let mut sim = Simulation::new(tiny(0x57_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "cloak", &[(ItemId::Fiber, 12)]);
}

#[test]
fn catalog_on_craft_pot() {
    let mut sim = Simulation::new(tiny(0x57_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "pot", &[(ItemId::Stone, 8)]);
}

#[test]
fn catalog_on_craft_lantern() {
    let mut sim = Simulation::new(tiny(0x57_14)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "lantern",
        &[(ItemId::Wood, 2), (ItemId::Fiber, 2), (ItemId::Stone, 2)],
    );
}

fn find_veg(sim: &Simulation) -> (u32, u32, u8) {
    for y in 0..sim.world.height {
        for x in 0..sim.world.width {
            let t = sim.world.vegetation_species(x, y);
            if t != 0 {
                return (x, y, t);
            }
        }
    }
    panic!("no vegetation");
}

fn park_adj_land(sim: &mut Simulation, id: AgentId, tx: u32, ty: u32) {
    for (dx, dy) in [(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = tx as i32 + dx;
        let ny = ty as i32 + dy;
        if nx < 0 || ny < 0 {
            continue;
        }
        let (x, y) = (nx as u32, ny as u32);
        if sim.world.is_land(x, y) {
            let a = sim.agents.get_mut(&id).unwrap();
            a.x = x;
            a.y = y;
            return;
        }
    }
    panic!("no land adjacent to veg");
}

#[test]
fn catalog_off_millstone_place_illegal() {
    let sim = Simulation::new(tiny(0x58_20)).unwrap();
    let a = AgentId(0);
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Place { .. })));
}

#[test]
fn axe_breaks_after_8_successful_gathers() {
    let mut sim = Simulation::new(tiny(0x58_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    {
        let a = sim.agents.get_mut(&id).unwrap();
        a.abilities.gather = 100;
        a.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        a.try_add_item(axe, 1);
    }
    let (vx, vy, tag) = find_veg(&sim);
    park_adj_land(&mut sim, id, vx, vy);
    let mut ok = 0u32;
    for _ in 0..400 {
        sim.world.set_vegetation(vx, vy, tag);
        let before = sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0);
        sim_core::execute::execute_primary(
            &mut sim,
            id,
            &PrimaryAction::Gather { species: tag },
        );
        if sim.events.events.last().is_some_and(|e| {
            matches!(
                e.kind,
                sim_core::event_log::SimEventKind::Gather { species, qty, .. }
                    if species == tag && qty > 0
            )
        }) || sim.agents[&id]
            .tool_wear
            .get(&axe)
            .and_then(|v| v.iter().copied().max())
            .unwrap_or(0)
            > ok
            || sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0) < before
        {
            ok += 1;
        }
        if ok == 7 {
            assert_eq!(
                sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0),
                1,
                "7 uses keep the axe"
            );
        }
        if ok >= 8 {
            break;
        }
    }
    assert!(ok >= 8, "need 8 successful gathers, got {ok}");
    assert_eq!(
        sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0),
        0,
        "8th use consumes the axe"
    );
}

#[test]
fn flour_needs_placed_millstone() {
    let mut sim = Simulation::new(tiny(0x58_22)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let mill = sim
        .catalog
        .iter()
        .find(|e| e.slug == "millstone")
        .unwrap()
        .item;
    let flour = sim.catalog.iter().find(|e| e.slug == "flour").unwrap().item;
    let ItemId::Catalog(n) = flour else {
        panic!("flour catalog");
    };
    {
        let a = sim.agents.get_mut(&id).unwrap();
        a.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        a.abilities.craft = 100;
        a.inventory_cap = 32;
        a.try_add_item(ItemId::Food(1), 6);
        a.try_add_item(mill, 1);
    }
    let legal = legal_actions(&sim, sim.agents.get(&id).unwrap());
    assert!(
        !legal.iter().any(|x| matches!(
            x,
            PrimaryAction::Craft {
                recipe: Recipe::Catalog(k)
            } if *k == n
        )),
        "pocket millstone is not a station"
    );
    let (x, y) = (sim.agents[&id].x, sim.agents[&id].y);
    if !sim.world.is_land(x, y) {
        for yy in 0..sim.world.height {
            for xx in 0..sim.world.width {
                if sim.world.is_land(xx, yy) {
                    sim.agents.get_mut(&id).unwrap().x = xx;
                    sim.agents.get_mut(&id).unwrap().y = yy;
                    break;
                }
            }
        }
    }
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Place { item: mill },
    );
    assert!(sim.world.work_places.values().any(|i| *i == mill));
    craft_catalog_slug(&mut sim, "flour", &[(ItemId::Food(1), 6)]);
}

#[test]
fn catalog_on_craft_cart() {
    let mut sim = Simulation::new(tiny(0x58_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "cart", &[(ItemId::Wood, 12)]);
}

#[test]
fn catalog_on_craft_bellows() {
    let mut sim = Simulation::new(tiny(0x58_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "bellows",
        &[(ItemId::Fiber, 6), (ItemId::Stone, 2)],
    );
}

#[test]
fn catalog_on_craft_table() {
    let mut sim = Simulation::new(tiny(0x58_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(
        &mut sim,
        "table",
        &[(ItemId::Wood, 6), (ItemId::Fiber, 2)],
    );
}

#[test]
fn load_restores_tool_wear() {
    let mut sim = Simulation::new(tiny(0x58_23)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    sim.agents.get_mut(&id).unwrap().try_add_item(axe, 1);
    sim.agents
        .get_mut(&id)
        .unwrap()
        .tool_wear
        .insert(axe, vec![3]);
    let bytes = sim.encode_checkpoint().unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert_eq!(
        loaded.agents[&id].tool_wear.get(&axe).cloned(),
        Some(vec![3])
    );
}

#[test]
fn two_axes_wear_separately() {
    let mut sim = Simulation::new(tiny(0x59_21)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    {
        let a = sim.agents.get_mut(&id).unwrap();
        a.abilities.gather = 100;
        a.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        a.inventory_cap = 32;
        a.try_add_item(axe, 2);
    }
    let (vx, vy, tag) = find_veg(&sim);
    park_adj_land(&mut sim, id, vx, vy);
    let mut ok = 0u32;
    for _ in 0..400 {
        sim.world.set_vegetation(vx, vy, tag);
        let qty_before = sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0);
        let wear_before = sim.agents[&id]
            .tool_wear
            .get(&axe)
            .and_then(|v| v.iter().copied().max())
            .unwrap_or(0);
        sim_core::execute::execute_primary(
            &mut sim,
            id,
            &PrimaryAction::Gather { species: tag },
        );
        let qty = sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0);
        let wear_max = sim.agents[&id]
            .tool_wear
            .get(&axe)
            .and_then(|v| v.iter().copied().max())
            .unwrap_or(0);
        if qty < qty_before || wear_max > wear_before {
            ok += 1;
        }
        if qty == 1 && ok >= 8 {
            break;
        }
    }
    assert!(ok >= 8, "need 8 successful gathers, got {ok}");
    assert_eq!(
        sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0),
        1,
        "one axe remains"
    );
    let leftover = sim.agents[&id]
        .tool_wear
        .get(&axe)
        .cloned()
        .unwrap_or_default();
    assert!(
        leftover.iter().all(|&w| w == 0) || leftover.is_empty(),
        "remaining axe is fresh, got {leftover:?}"
    );
}

#[test]
fn catalog_on_craft_raft() {
    let mut sim = Simulation::new(tiny(0x59_11)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "raft", &[(ItemId::Wood, 14)]);
}

#[test]
fn catalog_on_craft_sandals() {
    let mut sim = Simulation::new(tiny(0x59_12)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "sandals", &[(ItemId::Fiber, 14)]);
}

#[test]
fn catalog_on_craft_mortar() {
    let mut sim = Simulation::new(tiny(0x59_13)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "mortar", &[(ItemId::Stone, 10)]);
}

#[test]
fn catalog_on_craft_jerky() {
    let mut sim = Simulation::new(tiny(0x59_14)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "jerky", &[(ItemId::Food(1), 8)]);
}

fn park_adjacent(sim: &mut Simulation, a: AgentId, b: AgentId) {
    let land = sim.world.land_cells();
    let (x, y) = land[0];
    let neigh = neighbors4(&sim.world, x, y)
        .into_iter()
        .find(|&(nx, ny)| (nx != x || ny != y) && sim.world.is_land(nx, ny))
        .expect("neighbor");
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = x;
        ag.y = y;
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.inventory_cap = 32;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = neigh.0;
        ag.y = neigh.1;
        ag.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        ag.inventory_cap = 32;
        ag.inventory.clear();
    }
}

#[test]
fn transfer_moves_freshest_wear_slot() {
    let mut sim = Simulation::new(tiny(0x60_11)).unwrap();
    apply_shipped(&mut sim);
    sim.config.observation.full_information = true;
    let a = AgentId(0);
    let b = AgentId(1);
    park_adjacent(&mut sim, a, b);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    {
        let ag = sim.agents.get_mut(&a).unwrap();
        ag.try_add_item(axe, 2);
        ag.tool_wear.insert(axe, vec![3, 0]);
    }
    sim_core::execute::execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Transfer {
            item: axe,
            qty: 1,
            to: b,
        },
    );
    assert_eq!(
        sim.agents[&a].inventory.get(&axe).copied().unwrap_or(0),
        1
    );
    assert_eq!(
        sim.agents[&b].inventory.get(&axe).copied().unwrap_or(0),
        1
    );
    assert_eq!(
        sim.agents[&a].tool_wear.get(&axe).cloned(),
        Some(vec![3])
    );
    assert_eq!(
        sim.agents[&b].tool_wear.get(&axe).cloned(),
        Some(vec![0])
    );
}

#[test]
fn transfer_moves_single_worn_slot() {
    let mut sim = Simulation::new(tiny(0x60_12)).unwrap();
    apply_shipped(&mut sim);
    sim.config.observation.full_information = true;
    let a = AgentId(0);
    let b = AgentId(1);
    park_adjacent(&mut sim, a, b);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    {
        let ag = sim.agents.get_mut(&a).unwrap();
        ag.try_add_item(axe, 1);
        ag.tool_wear.insert(axe, vec![7]);
    }
    sim_core::execute::execute_primary(
        &mut sim,
        a,
        &PrimaryAction::Transfer {
            item: axe,
            qty: 1,
            to: b,
        },
    );
    assert_eq!(
        sim.agents[&a].inventory.get(&axe).copied().unwrap_or(0),
        0
    );
    assert!(sim.agents[&a].tool_wear.get(&axe).is_none());
    assert_eq!(
        sim.agents[&b].inventory.get(&axe).copied().unwrap_or(0),
        1
    );
    assert_eq!(
        sim.agents[&b].tool_wear.get(&axe).cloned(),
        Some(vec![7])
    );
}

#[test]
fn store_still_drops_freshest_wear() {
    let mut sim = Simulation::new(tiny(0x60_13)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let land = sim.world.land_cells()[0];
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    {
        let a = sim.agents.get_mut(&id).unwrap();
        a.x = land.0;
        a.y = land.1;
        a.needs = sim_core::Needs::maxed(1000, 1000, 1000);
        a.inventory_cap = 32;
        a.try_add_item(axe, 2);
        a.tool_wear.insert(axe, vec![3, 0]);
    }
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Store {
            item: axe,
            qty: 1,
        },
    );
    assert_eq!(
        sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0),
        1
    );
    assert_eq!(
        sim.agents[&id].tool_wear.get(&axe).cloned(),
        Some(vec![3])
    );
    assert!(sim.world.has_stockpile(land.0, land.1));
}

#[test]
fn give_item_mints_fresh_wear() {
    let mut sim = Simulation::new(tiny(0x60_14)).unwrap();
    apply_shipped(&mut sim);
    let id = AgentId(0);
    let axe = sim.catalog.iter().find(|e| e.slug == "axe").unwrap().item;
    sim.agents.get_mut(&id).unwrap().inventory_cap = 32;
    assert!(sim.agents[&id].tool_wear.get(&axe).is_none());
    let added = sim.give_item(id, axe, 1).unwrap();
    assert_eq!(added, 1);
    assert_eq!(
        sim.agents[&id].inventory.get(&axe).copied().unwrap_or(0),
        1
    );
    assert!(
        sim.agents[&id].tool_wear.get(&axe).is_none(),
        "minted axe is fresh (no wear map)"
    );
}

#[test]
fn catalog_on_craft_stool() {
    let mut sim = Simulation::new(tiny(0x60_31)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "stool", &[(ItemId::Wood, 16)]);
}

#[test]
fn catalog_on_craft_sash() {
    let mut sim = Simulation::new(tiny(0x60_32)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "sash", &[(ItemId::Fiber, 16)]);
}

#[test]
fn catalog_on_craft_brick() {
    let mut sim = Simulation::new(tiny(0x60_33)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "brick", &[(ItemId::Stone, 12)]);
}

#[test]
fn catalog_on_craft_biscuit() {
    let mut sim = Simulation::new(tiny(0x60_34)).unwrap();
    apply_shipped(&mut sim);
    craft_catalog_slug(&mut sim, "biscuit", &[(ItemId::Food(1), 10)]);
}
