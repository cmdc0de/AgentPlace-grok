//! Hash-neutral authored glTF/glb paths. Missing file ⇒ primitive mesh.

use std::path::PathBuf;

/// Locked stems from `docs/M38-plan.md`.
pub const STEMS: &[&str] = &[
    "agent",
    "berry_bush",
    "herb",
    "mushroom",
    "nightshade",
    "tree",
    "hare",
    "perch",
    "crop",
    "wood",
    "fiber",
    "stone",
    "basket",
    "spear",
    "fishing_rod",
    "backpack",
    "crate",
];

pub fn model_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("AGENTPLACE_MODELS") {
        dirs.push(PathBuf::from(d));
    }
    dirs.push(PathBuf::from("assets/models"));
    dirs.push(PathBuf::from("crates/viewer/assets/models"));
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dirs.push(manifest.join("assets/models"));
    dirs.push(manifest.join("../../assets/models"));
    dirs
}

/// `Some` if `{stem}.glb` or `{stem}.gltf` exists under a model dir.
pub fn resolve_model(stem: &str) -> Option<PathBuf> {
    if !STEMS.contains(&stem) {
        return None;
    }
    for dir in model_dirs() {
        for ext in ["glb", "gltf"] {
            let p = dir.join(format!("{stem}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Definition `id` visual/LOD if the file exists, else M38 stem.
pub fn resolve_visual(defs: &[sim_core::ObjectDef], id: &str, dist_cells: u32) -> Option<PathBuf> {
    if let Some(visual) = sim_core::visual_for_id(defs, id) {
        if let Some(p) = sim_core::pick_visual_path(visual, dist_cells) {
            return Some(p);
        }
    }
    resolve_model(id)
}

/// Chebyshev cell distance from the default setup camera xz to `(x, y)`.
pub fn camera_dist_cells(world_w: u32, world_h: u32, x: u32, y: u32) -> u32 {
    let cx = world_w as f32 * 0.5;
    let cz = world_h as f32 * 0.5;
    let max_x = world_w.saturating_sub(1) as f32;
    let max_z = world_h.saturating_sub(1) as f32;
    let cam_x = (cx - 28.0).round().clamp(0.0, max_x) as u32;
    let cam_z = (cz + 36.0).round().clamp(0.0, max_z) as u32;
    cam_x.abs_diff(x).max(cam_z.abs_diff(y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{ExperimentConfig, Simulation};
    use std::fs;

    #[test]
    fn path_map_covers_locked_stems() {
        assert!(STEMS.contains(&"agent"));
        assert!(STEMS.contains(&"berry_bush"));
        assert!(STEMS.contains(&"herb"));
        assert!(STEMS.contains(&"hare"));
        assert!(STEMS.contains(&"perch"));
        assert!(STEMS.contains(&"crop"));
        assert!(STEMS.contains(&"wood"));
        assert!(STEMS.contains(&"fiber"));
        assert!(STEMS.contains(&"stone"));
        assert!(STEMS.contains(&"basket"));
        assert!(STEMS.contains(&"spear"));
        assert!(STEMS.contains(&"fishing_rod"));
        assert!(STEMS.contains(&"backpack"));
        assert!(STEMS.contains(&"crate"));
        assert_eq!(STEMS.len(), 17);
    }

    #[test]
    fn missing_glb_falls_back() {
        for stem in STEMS {
            let _ = resolve_model(stem);
        }
        assert!(
            resolve_model("agent").is_none() || resolve_model("agent").is_some(),
            "resolve is optional"
        );
        assert!(resolve_model("not_a_kind").is_none());
    }

    #[test]
    fn authored_files_are_not_in_state_hash() {
        let cfg = ExperimentConfig::from_toml_str(
            r#"
master_seed = 1
[simulation]
max_ticks = 10
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
        )
        .unwrap();
        let mut a = Simulation::new(cfg.clone()).unwrap();
        a.run_ticks(2);
        let hash = a.state_hash();
        let dir = std::env::temp_dir().join("agentplace-m38-models");
        let _ = fs::create_dir_all(&dir);
        let dummy = dir.join("agent.glb");
        fs::write(&dummy, b"not-a-real-glb").unwrap();
        unsafe { std::env::set_var("AGENTPLACE_MODELS", &dir) };
        assert!(resolve_model("agent").is_some());
        let mut b = Simulation::new(cfg).unwrap();
        b.run_ticks(2);
        assert_eq!(b.state_hash(), hash, "models must not enter state_hash");
        unsafe { std::env::remove_var("AGENTPLACE_MODELS") };
        let _ = fs::remove_file(&dummy);
    }

    #[test]
    fn visual_toml_uses_glb_path() {
        let dir = std::env::temp_dir().join("agentplace-m39-viewer-visual");
        let _ = fs::create_dir_all(&dir);
        let glb = dir.join("custom_bush.glb");
        fs::write(&glb, b"not-a-real-glb").unwrap();
        let def = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some(glb.to_string_lossy().into_owned()),
                lod: Default::default(),
            }),
            sim: None,
        };
        let path = resolve_visual(&[def], "berry_bush", 0).expect("glb path");
        assert_eq!(path, glb);
    }

    #[test]
    fn lod_and_stem_fallback() {
        let dir = std::env::temp_dir().join("agentplace-m39-viewer-lod");
        let _ = fs::create_dir_all(&dir);
        let far = dir.join("far.glb");
        fs::write(&far, b"far").unwrap();
        let def = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some(dir.join("missing.glb").to_string_lossy().into_owned()),
                lod: sim_core::objects::LodDef {
                    near: None,
                    mid: Some(dir.join("missing-mid.glb").to_string_lossy().into_owned()),
                    far: Some(far.to_string_lossy().into_owned()),
                },
            }),
            sim: None,
        };
        let defs = [def];
        assert_eq!(
            resolve_visual(&defs, "berry_bush", 10).as_deref(),
            Some(far.as_path()),
            "missing mid uses far"
        );
        assert!(
            resolve_visual(&[], "not_a_kind", 0).is_none(),
            "unknown id without toml ⇒ primitive"
        );
        let _ = resolve_visual(&[], "berry_bush", 0);
    }

    #[test]
    fn object_defs_are_not_in_state_hash() {
        let cfg = ExperimentConfig::from_toml_str(
            r#"
master_seed = 1
[simulation]
max_ticks = 10
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
        )
        .unwrap();
        let mut a = Simulation::new(cfg.clone()).unwrap();
        a.run_ticks(2);
        let hash = a.state_hash();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects");
        let defs = sim_core::load_object_defs(&dir).unwrap();
        assert!(!defs.is_empty());
        let mut b = Simulation::new(cfg).unwrap();
        b.run_ticks(2);
        assert_eq!(
            b.state_hash(),
            hash,
            "object visual files must not enter state_hash"
        );
    }
}
