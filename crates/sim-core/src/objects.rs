//! Object definition files (`configs/objects/*.toml`). Visuals are not hashed.
//! Item `[sim]` (including built-in recipes) is hashed when the catalog is loaded.

use crate::action::Recipe;
use crate::agent::{Agent, AgentId, ItemId};
use crate::error::SimError;
use crate::event_log::SimEvent;
use crate::species::{FaunaSpecies, SpeciesTables, Toxicity, VegYield, VegetationSpecies};
use serde::Deserialize;
use sha2::Digest;
use std::path::{Path, PathBuf};

/// Built-in vegetation ids in today’s 1-based tag order. Extras append after these.
pub const BUILTIN_VEG_IDS: &[&str] = &["berry_bush", "herb", "mushroom", "nightshade", "tree"];
pub const BUILTIN_ANIMAL_IDS: &[&str] = &["hare"];
pub const BUILTIN_FISH_IDS: &[&str] = &["perch"];

pub const LOD_NEAR_CELLS: u32 = 8;
pub const LOD_MID_CELLS: u32 = 24;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogParams {
    pub enabled: bool,
}

impl CatalogParams {
    pub fn from_config_toml(s: &str) -> Self {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            catalog: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            enabled: Option<bool>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        Self {
            enabled: slice.catalog.enabled.unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ObjectDef {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub visual: Option<VisualDef>,
    #[serde(default)]
    pub sim: Option<SimDef>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct VisualDef {
    #[serde(default)]
    pub glb: Option<String>,
    #[serde(default)]
    pub lod: LodDef,
    /// Uniform XYZ. `None` / non-finite / `<= 0` ⇒ omitted (not hashed).
    #[serde(default)]
    pub scale: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LodDef {
    #[serde(default)]
    pub near: Option<String>,
    #[serde(default)]
    pub mid: Option<String>,
    #[serde(default)]
    pub far: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SimDef {
    #[serde(default)]
    pub weight_milli: Option<u32>,
    #[serde(default)]
    pub craft: Option<CraftDef>,
    #[serde(default, rename = "yield")]
    pub yield_kind: Option<VegYield>,
    #[serde(default)]
    pub nutrition: Option<f64>,
    #[serde(default)]
    pub toxicity: Option<Toxicity>,
    #[serde(default)]
    pub allergen_tag: Option<String>,
    #[serde(default)]
    pub wood_yield: Option<u32>,
    #[serde(default)]
    pub fiber_yield: Option<u32>,
    #[serde(default)]
    pub grow_ticks: Option<u64>,
    /// Extra Attack damage millipoints when this item is held. Omit = 0.
    #[serde(default)]
    pub attack_bonus: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CraftDef {
    #[serde(default)]
    pub inputs: Vec<(String, u32)>,
    #[serde(default)]
    pub output_qty: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub slug: String,
    pub item: ItemId,
    pub recipe: Option<Recipe>,
    pub weight_milli: u32,
    pub inputs: Vec<(ItemId, u32)>,
    pub output_qty: u32,
    pub attack_bonus: u32,
}

pub fn builtin_item(slug: &str) -> Option<ItemId> {
    match slug {
        "wood" => Some(ItemId::Wood),
        "fiber" => Some(ItemId::Fiber),
        "stone" => Some(ItemId::Stone),
        "food" => Some(ItemId::Food(1)),
        "basket" => Some(ItemId::Basket),
        "spear" => Some(ItemId::Spear),
        "fishing_rod" => Some(ItemId::FishingRod),
        "backpack" => Some(ItemId::Backpack),
        _ => None,
    }
}

pub fn builtin_recipe(slug: &str) -> Option<Recipe> {
    match slug {
        "basket" => Some(Recipe::Basket),
        "spear" => Some(Recipe::Spear),
        "fishing_rod" => Some(Recipe::FishingRod),
        "backpack" => Some(Recipe::Backpack),
        _ => None,
    }
}

pub fn default_objects_dir() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("configs/objects"),
        PathBuf::from("../configs/objects"),
        PathBuf::from("../../configs/objects"),
    ];
    // cargo run -p viewer from a non-repo cwd still finds shipped TOML.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("../../configs/objects"));
    candidates.push(manifest.join("../configs/objects"));
    candidates.into_iter().find(|p| p.is_dir())
}

pub fn load_object_defs(dir: &Path) -> Result<Vec<ObjectDef>, SimError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut defs = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| SimError::Config(format!("objects dir {}: {e}", dir.display())))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| SimError::Config(format!("{}: {e}", path.display())))?;
        let def: ObjectDef = toml::from_str(&text)
            .map_err(|e| SimError::Config(format!("{}: {e}", path.display())))?;
        if def.id.is_empty() {
            return Err(SimError::Config(format!("{}: missing id", path.display())));
        }
        defs.push(def);
    }
    Ok(defs)
}

fn patch_veg(row: &mut VegetationSpecies, sim: &SimDef) {
    if let Some(y) = sim.yield_kind {
        row.yield_kind = y;
    }
    if let Some(n) = sim.nutrition {
        row.nutrition = n;
    }
    if let Some(t) = sim.toxicity {
        row.toxicity = t;
    }
    if let Some(a) = &sim.allergen_tag {
        row.allergen_tag = a.clone();
    }
    if let Some(w) = sim.wood_yield {
        row.wood_yield = w;
    }
    if let Some(f) = sim.fiber_yield {
        row.fiber_yield = f;
    }
    if let Some(g) = sim.grow_ticks {
        row.grow_ticks = g;
    }
}

fn patch_fauna(row: &mut FaunaSpecies, sim: &SimDef) {
    if let Some(n) = sim.nutrition {
        row.nutrition = n;
    }
    if let Some(t) = sim.toxicity {
        row.toxicity = t;
    }
    if let Some(a) = &sim.allergen_tag {
        row.allergen_tag = a.clone();
    }
}

fn veg_from_sim(id: &str, sim: &SimDef) -> VegetationSpecies {
    VegetationSpecies {
        id: id.to_string(),
        yield_kind: sim.yield_kind.unwrap_or_default(),
        nutrition: sim.nutrition.unwrap_or(20.0),
        toxicity: sim.toxicity.unwrap_or_default(),
        allergen_tag: sim.allergen_tag.clone().unwrap_or_default(),
        wood_yield: sim.wood_yield.unwrap_or(0),
        fiber_yield: sim.fiber_yield.unwrap_or(0),
        grow_ticks: sim.grow_ticks.unwrap_or(40),
    }
}

fn fauna_from_sim(id: &str, sim: &SimDef) -> FaunaSpecies {
    FaunaSpecies {
        id: id.to_string(),
        nutrition: sim.nutrition.unwrap_or(20.0),
        toxicity: sim.toxicity.unwrap_or_default(),
        allergen_tag: sim.allergen_tag.clone().unwrap_or_default(),
    }
}

fn append_extras<'a>(
    defs: &'a [ObjectDef],
    kind: &str,
    builtin: &[&str],
    have: impl Fn(&str) -> bool,
) -> Vec<&'a ObjectDef> {
    let mut extra: Vec<&ObjectDef> = defs
        .iter()
        .filter(|d| {
            d.kind == kind && d.sim.is_some() && !builtin.contains(&d.id.as_str()) && !have(&d.id)
        })
        .collect();
    extra.sort_by(|a, b| a.id.cmp(&b.id));
    extra
}

/// Merge vegetation/animal/fish `[sim]` into species tables.
/// Built-in ids keep today’s tags; new slugs append in sorted id order.
pub fn apply_species_defs(tables: &mut SpeciesTables, defs: &[ObjectDef]) {
    tables.ensure_defaults();
    for def in defs {
        let Some(sim) = &def.sim else {
            continue;
        };
        match def.kind.as_str() {
            "vegetation" => {
                if let Some(row) = tables.vegetation.iter_mut().find(|v| v.id == def.id) {
                    patch_veg(row, sim);
                }
            }
            "animal" => {
                if let Some(row) = tables.animals.iter_mut().find(|v| v.id == def.id) {
                    patch_fauna(row, sim);
                }
            }
            "fish" => {
                if let Some(row) = tables.fish.iter_mut().find(|v| v.id == def.id) {
                    patch_fauna(row, sim);
                }
            }
            _ => {}
        }
    }
    let have_veg: Vec<String> = tables.vegetation.iter().map(|v| v.id.clone()).collect();
    for d in append_extras(defs, "vegetation", BUILTIN_VEG_IDS, |id| {
        have_veg.iter().any(|h| h == id)
    }) {
        tables
            .vegetation
            .push(veg_from_sim(&d.id, d.sim.as_ref().unwrap()));
    }
    let have_an: Vec<String> = tables.animals.iter().map(|v| v.id.clone()).collect();
    for d in append_extras(defs, "animal", BUILTIN_ANIMAL_IDS, |id| {
        have_an.iter().any(|h| h == id)
    }) {
        tables
            .animals
            .push(fauna_from_sim(&d.id, d.sim.as_ref().unwrap()));
    }
    let have_fi: Vec<String> = tables.fish.iter().map(|v| v.id.clone()).collect();
    for d in append_extras(defs, "fish", BUILTIN_FISH_IDS, |id| {
        have_fi.iter().any(|h| h == id)
    }) {
        tables
            .fish
            .push(fauna_from_sim(&d.id, d.sim.as_ref().unwrap()));
    }
}

pub fn catalog_entries(defs: &[ObjectDef]) -> Vec<CatalogEntry> {
    let mut items: Vec<&ObjectDef> = defs
        .iter()
        .filter(|d| d.kind == "item" && d.sim.is_some())
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let extra: Vec<String> = items
        .iter()
        .map(|d| d.id.clone())
        .filter(|id| builtin_item(id).is_none())
        .collect();
    items
        .iter()
        .map(|d| {
            let sim = d.sim.as_ref().unwrap();
            let item = builtin_item(&d.id).unwrap_or_else(|| {
                let i = extra.iter().position(|s| s == &d.id).unwrap_or(0);
                ItemId::Catalog(i as u16)
            });
            let (inputs, output_qty, has_craft) = if let Some(c) = &sim.craft {
                let ins = c
                    .inputs
                    .iter()
                    .filter_map(|(s, n)| {
                        parse_catalog_item(s, &extra).map(|item| (item, (*n).max(1)))
                    })
                    .collect();
                (ins, c.output_qty.unwrap_or(1).max(1), true)
            } else {
                (Vec::new(), 1, false)
            };
            let recipe = if has_craft && !inputs.is_empty() {
                builtin_recipe(&d.id).or_else(|| match item {
                    ItemId::Catalog(n) => Some(Recipe::Catalog(n)),
                    _ => None,
                })
            } else {
                None
            };
            CatalogEntry {
                slug: d.id.clone(),
                item,
                recipe,
                weight_milli: sim.weight_milli.unwrap_or(400),
                inputs,
                output_qty,
                attack_bonus: sim.attack_bonus.unwrap_or(0),
            }
        })
        .collect()
}

pub fn parse_catalog_item(slug: &str, extra_slugs: &[String]) -> Option<ItemId> {
    if let Some(item) = builtin_item(slug) {
        return Some(item);
    }
    extra_slugs
        .iter()
        .position(|s| s == slug)
        .map(|i| ItemId::Catalog(i as u16))
}

pub fn lod_band(dist_cells: u32) -> &'static str {
    if dist_cells < LOD_NEAR_CELLS {
        "near"
    } else if dist_cells < LOD_MID_CELLS {
        "mid"
    } else {
        "far"
    }
}

/// LOD / glb path strings in fallback order (near → coarser → `glb`).
pub fn visual_path_candidates(visual: &VisualDef, dist_cells: u32) -> Vec<&str> {
    let band = lod_band(dist_cells);
    let ordered: Vec<Option<&String>> = match band {
        "near" => vec![
            visual.lod.near.as_ref(),
            visual.lod.mid.as_ref(),
            visual.lod.far.as_ref(),
            visual.glb.as_ref(),
        ],
        "mid" => vec![
            visual.lod.mid.as_ref(),
            visual.lod.far.as_ref(),
            visual.glb.as_ref(),
        ],
        _ => vec![visual.lod.far.as_ref(), visual.glb.as_ref()],
    };
    ordered.into_iter().flatten().map(String::as_str).collect()
}

/// Existing path string, else next coarser, else `glb`.
pub fn pick_visual_path(visual: &VisualDef, dist_cells: u32) -> Option<PathBuf> {
    for p in visual_path_candidates(visual, dist_cells) {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Finite `scale > 0` from `[visual]`. `None` means omit (agent auto-fit / others 1.0).
pub fn visual_effective_scale(visual: &VisualDef) -> Option<f32> {
    match visual.scale {
        Some(s) if s.is_finite() && s > 0.0 => Some(s),
        _ => None,
    }
}

pub fn visual_scale_for(defs: &[ObjectDef], id: &str) -> Option<f32> {
    visual_for_id(defs, id).and_then(visual_effective_scale)
}

pub fn visual_for_id<'a>(defs: &'a [ObjectDef], id: &str) -> Option<&'a VisualDef> {
    defs.iter()
        .find(|d| d.id == id)
        .and_then(|d| d.visual.as_ref())
}

pub fn catalog_slug_vec(entries: &[CatalogEntry]) -> Vec<String> {
    let mut v = Vec::new();
    for e in entries {
        if let ItemId::Catalog(n) = e.item {
            let i = n as usize;
            if v.len() <= i {
                v.resize(i + 1, String::new());
            }
            v[i] = e.slug.clone();
        }
    }
    v
}

pub fn remap_catalog_item(item: ItemId, old_slugs: &[String], entries: &[CatalogEntry]) -> ItemId {
    let ItemId::Catalog(n) = item else {
        return item;
    };
    let Some(slug) = old_slugs.get(n as usize).filter(|s| !s.is_empty()) else {
        return item;
    };
    entries
        .iter()
        .find(|e| e.slug == *slug)
        .map(|e| e.item)
        .unwrap_or(item)
}

fn remap_recipe(recipe: Recipe, old_slugs: &[String], entries: &[CatalogEntry]) -> Recipe {
    let Recipe::Catalog(n) = recipe else {
        return recipe;
    };
    match remap_catalog_item(ItemId::Catalog(n), old_slugs, entries) {
        ItemId::Catalog(m) => Recipe::Catalog(m),
        _ => recipe,
    }
}

pub fn remap_catalog_holdings(
    agents: &mut std::collections::BTreeMap<AgentId, Agent>,
    events: &mut [SimEvent],
    old_slugs: &[String],
    entries: &[CatalogEntry],
) {
    use crate::event_log::SimEventKind;
    for a in agents.values_mut() {
        a.inventory = a
            .inventory
            .iter()
            .map(|(item, qty)| (remap_catalog_item(*item, old_slugs, entries), *qty))
            .collect();
        a.pack = a
            .pack
            .iter()
            .map(|(item, qty)| (remap_catalog_item(*item, old_slugs, entries), *qty))
            .collect();
    }
    for e in events.iter_mut() {
        match &mut e.kind {
            SimEventKind::Gather { item, .. }
            | SimEventKind::Eat { item, .. }
            | SimEventKind::Transfer { item, .. }
            | SimEventKind::Store { item, .. }
            | SimEventKind::Retrieve { item, .. }
            | SimEventKind::Give { item, .. }
            | SimEventKind::Pack { item, .. }
            | SimEventKind::Unpack { item, .. } => {
                *item = remap_catalog_item(*item, old_slugs, entries);
            }
            SimEventKind::Craft { recipe, .. } => {
                *recipe = remap_recipe(*recipe, old_slugs, entries);
            }
            _ => {}
        }
    }
}

/// Max `[sim] attack_bonus` among items in pockets or pack. 0 if none.
pub fn max_held_attack_bonus(agent: &Agent, catalog: &[CatalogEntry]) -> u32 {
    catalog
        .iter()
        .filter(|e| {
            e.attack_bonus > 0
                && (agent.inventory.get(&e.item).copied().unwrap_or(0) > 0
                    || agent.pack.get(&e.item).copied().unwrap_or(0) > 0)
        })
        .map(|e| e.attack_bonus)
        .max()
        .unwrap_or(0)
}

pub fn hash_catalog(entries: &[CatalogEntry], hasher: &mut impl Digest) {
    if entries.is_empty() {
        return;
    }
    for (i, e) in entries.iter().enumerate() {
        hasher.update((i as u16).to_le_bytes());
        hasher.update(e.slug.as_bytes());
        hasher.update(e.weight_milli.to_le_bytes());
        hasher.update(e.output_qty.to_le_bytes());
        hasher.update(e.attack_bonus.to_le_bytes());
        hasher.update((e.inputs.len() as u32).to_le_bytes());
        for (item, n) in &e.inputs {
            crate::event_log::hash_item(hasher, *item);
            hasher.update(n.to_le_bytes());
        }
    }
}

/// Recipes come from object TOML. `None` if the file is missing or has no inputs.
pub fn recipe_spec(
    recipe: Recipe,
    catalog: &[CatalogEntry],
) -> Option<(Vec<(ItemId, u32)>, ItemId, u32)> {
    let entry = match recipe {
        Recipe::Catalog(n) => catalog.iter().find(|e| e.item == ItemId::Catalog(n)),
        other => catalog.iter().find(|e| e.recipe == Some(other)),
    }?;
    if entry.inputs.is_empty() {
        return None;
    }
    Some((entry.inputs.clone(), entry.item, entry.output_qty.max(1)))
}
