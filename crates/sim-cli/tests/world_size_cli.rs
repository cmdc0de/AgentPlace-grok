//! M52 --width / --height. Hashed. --load ignores size flags.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sim-cli"))
}

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml")
}

fn hash_of(args: &[&str]) -> String {
    let out = Command::new(bin()).args(args).output().expect("sim-cli");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("final_hash="))
        .unwrap()
        .to_string()
}

#[test]
fn width_32_hash_differs_from_default_64() {
    let cfg = config();
    let c = cfg.to_str().unwrap();
    let a = hash_of(&[
        "--config", c, "--ticks", "2", "--llm", "mock", "--quiet", "--no-time",
        "--width", "32", "--height", "32",
    ]);
    let b = hash_of(&[
        "--config", c, "--ticks", "2", "--llm", "mock", "--quiet", "--no-time",
    ]);
    assert_ne!(a, b);
}

#[test]
fn width_32_height_32_prints_world_size() {
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "2",
            "--llm",
            "mock",
            "--no-time",
            "--width",
            "32",
            "--height",
            "32",
        ])
        .output()
        .expect("sim-cli");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stdout.contains("world=32x32"),
        "stdout={stdout}"
    );
}

#[test]
fn width_31_is_error() {
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "0",
            "--llm",
            "mock",
            "--width",
            "31",
        ])
        .output()
        .expect("sim-cli");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--width must be 32..=256"), "{err}");
}

#[test]
fn width_257_is_error() {
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "0",
            "--llm",
            "mock",
            "--width",
            "257",
        ])
        .output()
        .expect("sim-cli");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--width must be 32..=256"), "{err}");
}

#[test]
fn load_ignores_width() {
    let dir = std::env::temp_dir().join(format!("m52-width-load-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let make = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "1",
            "--llm",
            "mock",
            "--quiet",
            "--no-time",
            "--out-dir",
            dir.to_str().unwrap(),
            "--checkpoint-every",
            "1",
        ])
        .output()
        .expect("sim-cli");
    assert!(make.status.success(), "{}", String::from_utf8_lossy(&make.stderr));
    let ckpt = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("ckpt"))
        .expect("ckpt");
    let out = Command::new(bin())
        .args([
            "--load",
            ckpt.to_str().unwrap(),
            "--width",
            "96",
            "--ticks",
            "0",
            "--llm",
            "mock",
        ])
        .output()
        .expect("sim-cli");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stdout.contains("world=64x64"),
        "loaded world must stay 64, stdout={stdout}"
    );
}
