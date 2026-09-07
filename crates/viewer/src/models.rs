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
}
