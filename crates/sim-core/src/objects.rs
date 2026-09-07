//! Object definition files (`configs/objects/*.toml`). Visuals are not hashed.
//! `[sim]` catalog items are hashed only when overlay `[catalog]` is on.

use crate::action::Recipe;
use crate::agent::ItemId;
use crate::error::SimError;
use serde::Deserialize;
use sha2::Digest;
use std::path::{Path, PathBuf};

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
    pub weight_milli: u32,
    pub inputs: Vec<(ItemId, u32)>,
    pub output_qty: u32,
}

pub fn default_objects_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("configs/objects"),
        PathBuf::from("../configs/objects"),
        PathBuf::from("../../configs/objects"),
    ];
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

pub fn catalog_entries(defs: &[ObjectDef]) -> Vec<CatalogEntry> {
    let mut items: Vec<&ObjectDef> = defs
        .iter()
        .filter(|d| d.kind == "item" && d.sim.is_some())
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let slugs: Vec<String> = items.iter().map(|d| d.id.clone()).collect();
    items
        .iter()
        .map(|d| {
            let sim = d.sim.as_ref().unwrap();
            let (inputs, output_qty) = if let Some(c) = &sim.craft {
                let ins = c
                    .inputs
                    .iter()
                    .filter_map(|(s, n)| {
                        parse_catalog_item(s, &slugs).map(|item| (item, (*n).max(1)))
                    })
                    .collect();
                (ins, c.output_qty.unwrap_or(1).max(1))
            } else {
                (Vec::new(), 1)
            };
            CatalogEntry {
                slug: d.id.clone(),
                weight_milli: sim.weight_milli.unwrap_or(400),
                inputs,
                output_qty,
            }
        })
        .collect()
}

pub fn parse_catalog_item(slug: &str, catalog_slugs: &[String]) -> Option<ItemId> {
    match slug {
        "wood" => Some(ItemId::Wood),
        "fiber" => Some(ItemId::Fiber),
        "stone" => Some(ItemId::Stone),
        "basket" => Some(ItemId::Basket),
        "spear" => Some(ItemId::Spear),
        "fishing_rod" => Some(ItemId::FishingRod),
        "backpack" => Some(ItemId::Backpack),
        other => catalog_slugs
            .iter()
            .position(|s| s == other)
            .map(|i| ItemId::Catalog(i as u16)),
    }
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

/// Existing path string, else next coarser, else `glb`.
pub fn pick_visual_path(visual: &VisualDef, dist_cells: u32) -> Option<PathBuf> {
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
    for p in ordered.into_iter().flatten() {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn visual_for_id<'a>(defs: &'a [ObjectDef], id: &str) -> Option<&'a VisualDef> {
    defs.iter()
        .find(|d| d.id == id)
        .and_then(|d| d.visual.as_ref())
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
        hasher.update((e.inputs.len() as u32).to_le_bytes());
        for (item, n) in &e.inputs {
            crate::event_log::hash_item(hasher, *item);
            hasher.update(n.to_le_bytes());
        }
    }
}

/// Built-in recipes plus catalog Craft. `None` if Catalog id is missing or has no inputs.
pub fn recipe_spec(
    recipe: Recipe,
    catalog: &[CatalogEntry],
) -> Option<(Vec<(ItemId, u32)>, ItemId, u32)> {
    match recipe {
        Recipe::Basket => Some((vec![(ItemId::Fiber, 2)], ItemId::Basket, 1)),
        Recipe::Backpack => Some((vec![(ItemId::Fiber, 4)], ItemId::Backpack, 1)),
        Recipe::Spear => Some((
            vec![(ItemId::Wood, 1), (ItemId::Stone, 1)],
            ItemId::Spear,
            1,
        )),
        Recipe::FishingRod => Some((
            vec![(ItemId::Wood, 1), (ItemId::Fiber, 1)],
            ItemId::FishingRod,
            1,
        )),
        Recipe::Catalog(n) => catalog.get(n as usize).and_then(|e| {
            if e.inputs.is_empty() {
                None
            } else {
                Some((e.inputs.clone(), ItemId::Catalog(n), e.output_qty.max(1)))
            }
        }),
    }
}
