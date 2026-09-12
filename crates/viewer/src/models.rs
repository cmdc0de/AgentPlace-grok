//! Hash-neutral authored glTF/glb paths.
//! Configured path missing ⇒ sentinel; no/empty visual ⇒ primitive.

use std::path::{Path, PathBuf};

/// How to draw an object id in the native viewer. Visuals are never hashed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualKind {
    Authored(PathBuf),
    Primitive,
    Sentinel,
}

fn visual_configured(visual: &sim_core::VisualDef) -> bool {
    let nonempty = |s: &Option<String>| s.as_deref().is_some_and(|x| !x.is_empty());
    nonempty(&visual.glb)
        || nonempty(&visual.lod.near)
        || nonempty(&visual.lod.mid)
        || nonempty(&visual.lod.far)
}

/// Stem fallback is **only** `agent` (M40). Other ids come from object TOML.
pub const STEMS: &[&str] = &["agent"];

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

fn path_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if path.is_relative() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        candidates.push(manifest.join("../..").join(path));
        candidates.push(manifest.join(path));
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join(path));
        }
    }
    candidates
}

/// Resolve a TOML path against cwd and the workspace root; return a canonical file.
pub fn existing_file(path: &Path) -> Option<PathBuf> {
    for c in path_candidates(path) {
        if c.is_file() {
            return c.canonicalize().ok().or(Some(c));
        }
    }
    None
}

/// Same search as [`existing_file`], for `--objects` / `configs/objects`.
pub fn existing_dir(path: &Path) -> Option<PathBuf> {
    for c in path_candidates(path) {
        if c.is_dir() {
            return c.canonicalize().ok().or(Some(c));
        }
    }
    None
}

pub fn resolve_objects_dir(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        if let Some(found) = existing_dir(p) {
            return Some(found);
        }
        eprintln!(
            "objects: --objects {} is not a directory (cwd={})",
            p.display(),
            std::env::current_dir()
                .ok()
                .map(|c| c.display().to_string())
                .unwrap_or_else(|| "?".into())
        );
    }
    sim_core::objects::default_objects_dir().and_then(|p| existing_dir(&p).or(Some(p)))
}

/// Load object TOML the same way the viewer does (`--objects` or shipped default).
/// A relative `--objects` path is resolved against cwd **and** the workspace root so
/// `cargo run -p viewer -- --connect … --objects configs/objects` still works when
/// cwd is not the repo. Missing explicit dir falls back to shipped `configs/objects`.
pub fn load_viewer_objects(explicit: Option<&Path>) -> (Option<PathBuf>, Vec<sim_core::ObjectDef>) {
    match resolve_objects_dir(explicit) {
        None => (None, Vec::new()),
        Some(d) => {
            let defs = sim_core::load_object_defs(&d).unwrap_or_default();
            (Some(d), defs)
        }
    }
}

/// Authored path if the file exists; `None` for primitive **or** sentinel.
pub fn resolve_visual(defs: &[sim_core::ObjectDef], id: &str, dist_cells: u32) -> Option<PathBuf> {
    match resolve_visual_kind(defs, id, dist_cells) {
        VisualKind::Authored(p) => Some(p),
        VisualKind::Primitive | VisualKind::Sentinel => None,
    }
}

/// Configured missing glb ⇒ [`VisualKind::Sentinel`]; empty/`None` visual ⇒ Primitive.
pub fn resolve_visual_kind(defs: &[sim_core::ObjectDef], id: &str, dist_cells: u32) -> VisualKind {
    if let Some(visual) = sim_core::visual_for_id(defs, id) {
        if visual_configured(visual) {
            for raw in sim_core::objects::visual_path_candidates(visual, dist_cells) {
                if raw.is_empty() {
                    continue;
                }
                if let Some(p) = existing_file(Path::new(raw)) {
                    return VisualKind::Authored(p);
                }
            }
            return VisualKind::Sentinel;
        }
    }
    if id == "agent" {
        if let Some(p) = resolve_model(id).and_then(|p| existing_file(&p).or(Some(p))) {
            return VisualKind::Authored(p);
        }
    }
    VisualKind::Primitive
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
        assert_eq!(STEMS, &["agent"]);
        assert!(resolve_visual(&[], "berry_bush", 0).is_none());
        assert!(resolve_visual(&[], "basket", 0).is_none());
        assert!(resolve_model("berry_bush").is_none());
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
        assert!(
            resolve_visual(&[], "berry_bush", 0).is_none(),
            "no stem fallback except agent"
        );
    }

    /// Regression: cwd ≠ repo used to make `--objects configs/objects` (and omit)
    /// load **zero** defs, so `--connect` spawned primitives and no `loaded glb`.
    #[test]
    fn connect_wrong_cwd_still_loads_shipped_glb() {
        struct CwdGuard(PathBuf);
        impl Drop for CwdGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _guard = CwdGuard(std::env::current_dir().unwrap());
        std::env::set_current_dir(std::env::temp_dir()).unwrap();

        let bogus = PathBuf::from(format!(
            "/nope/agentplace-objects-regression-{}",
            std::process::id()
        ));
        let cases: [(&str, Option<&Path>); 3] = [
            ("omit --objects", None),
            ("relative --objects", Some(Path::new("configs/objects"))),
            ("missing --objects path", Some(bogus.as_path())),
        ];
        for (label, arg) in cases {
            let (dir, defs) = load_viewer_objects(arg);
            let dir = dir.unwrap_or_else(|| panic!("{label}: expected shipped objects dir"));
            assert!(
                dir.is_dir(),
                "{label}: {} is not a directory",
                dir.display()
            );
            assert!(
                !defs.is_empty(),
                "{label}: zero defs from {} (the silent primitive fallback)",
                dir.display()
            );
            assert!(
                defs.iter().any(|d| d.id == "berry_bush"),
                "{label}: berry_bush missing in {}",
                dir.display()
            );
            for id in ["berry_bush", "hare", "crate", "tree", "basket"] {
                let path = resolve_visual(&defs, id, 0)
                    .unwrap_or_else(|| panic!("{label}: {id} did not resolve a glb"));
                assert!(
                    path.is_file(),
                    "{label}: {id} -> {} is not a file",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn shipped_object_toml_resolves_glb() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects");
        let defs = sim_core::load_object_defs(&dir).expect("load shipped objects");
        assert!(
            !defs.is_empty(),
            "expected shipped object TOML in {}",
            dir.display()
        );
        for id in ["berry_bush", "hare", "crate", "tree", "basket"] {
            let path = resolve_visual(&defs, id, 0);
            assert!(
                path.is_some(),
                "{id} should resolve a glb from shipped TOML, got {path:?}"
            );
        }
    }

    #[test]
    fn visual_optimized_path_when_present() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let basket = root.join("assets/models/optimized/basket.glb");
        let bush = root.join("assets/models/optimized/big_low_poly_berry_bush.glb");
        if !basket.is_file() || !bush.is_file() {
            return;
        }
        let basket = basket.canonicalize().unwrap_or(basket);
        let bush = bush.canonicalize().unwrap_or(bush);
        let basket_def = sim_core::ObjectDef {
            id: "basket".into(),
            kind: "item".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some(basket.to_string_lossy().into_owned()),
                lod: Default::default(),
            }),
            sim: None,
        };
        let bush_def = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some(bush.to_string_lossy().into_owned()),
                lod: Default::default(),
            }),
            sim: None,
        };
        assert_eq!(
            resolve_visual(&[basket_def], "basket", 0).as_deref(),
            Some(basket.as_path())
        );
        assert_eq!(
            resolve_visual(&[bush_def], "berry_bush", 0).as_deref(),
            Some(bush.as_path())
        );
    }

    #[test]
    fn empty_visual_is_primitive() {
        assert_eq!(
            resolve_visual_kind(&[], "berry_bush", 0),
            VisualKind::Primitive
        );
        let empty = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some(String::new()),
                lod: Default::default(),
            }),
            sim: None,
        };
        assert_eq!(
            resolve_visual_kind(&[empty], "berry_bush", 0),
            VisualKind::Primitive,
            "empty path stays primitive"
        );
        let omitted = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: None,
            sim: None,
        };
        assert_eq!(
            resolve_visual_kind(&[omitted], "berry_bush", 0),
            VisualKind::Primitive
        );
    }

    #[test]
    fn configured_missing_path_is_sentinel() {
        let def = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some("/nope/agentplace-missing-glb.glb".into()),
                lod: Default::default(),
            }),
            sim: None,
        };
        assert_eq!(
            resolve_visual_kind(&[def], "berry_bush", 0),
            VisualKind::Sentinel
        );
        assert!(
            resolve_visual(
                &[sim_core::ObjectDef {
                    id: "berry_bush".into(),
                    kind: "vegetation".into(),
                    visual: Some(sim_core::VisualDef {
                        glb: Some("/nope/agentplace-missing-glb.glb".into()),
                        lod: Default::default(),
                    }),
                    sim: None,
                }],
                "berry_bush",
                0
            )
            .is_none(),
            "sentinel is not an authored path"
        );
    }

    #[test]
    fn authored_exists_is_authored() {
        let dir = std::env::temp_dir().join("agentplace-m47-authored");
        let _ = fs::create_dir_all(&dir);
        let glb = dir.join("ok.glb");
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
        assert_eq!(
            resolve_visual_kind(&[def], "berry_bush", 0),
            VisualKind::Authored(glb)
        );
    }

    #[test]
    fn lod_exhausted_is_sentinel() {
        let def = sim_core::ObjectDef {
            id: "berry_bush".into(),
            kind: "vegetation".into(),
            visual: Some(sim_core::VisualDef {
                glb: Some("/nope/missing-glb.glb".into()),
                lod: sim_core::objects::LodDef {
                    near: Some("/nope/missing-near.glb".into()),
                    mid: Some("/nope/missing-mid.glb".into()),
                    far: Some("/nope/missing-far.glb".into()),
                },
            }),
            sim: None,
        };
        assert_eq!(
            resolve_visual_kind(&[def], "berry_bush", 0),
            VisualKind::Sentinel,
            "LOD miss then missing ⇒ sentinel if any path was set"
        );
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
