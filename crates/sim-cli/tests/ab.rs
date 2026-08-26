//! CLI A/B: same seed with vs without --incentives.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sim-cli"))
}

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml")
}

fn schedule() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/incentives/coop.toml")
}

fn parse_hash(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|l| l.strip_prefix("final_hash="))
        .expect("final_hash")
        .to_string()
}

#[test]
fn incentives_flag_changes_hash() {
    let base = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "20",
            "--llm",
            "mock",
            "--quiet",
        ])
        .output()
        .expect("baseline");
    assert!(
        base.status.success(),
        "{}",
        String::from_utf8_lossy(&base.stderr)
    );
    let hash_a = parse_hash(&base.stdout);

    let treated = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--incentives",
            schedule().to_str().unwrap(),
            "--ticks",
            "20",
            "--llm",
            "mock",
            "--quiet",
        ])
        .output()
        .expect("treated");
    assert!(
        treated.status.success(),
        "{}",
        String::from_utf8_lossy(&treated.stderr)
    );
    let hash_b = parse_hash(&treated.stdout);
    assert_ne!(hash_a, hash_b, "schedule must change state_hash");

    let treated2 = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--incentives",
            schedule().to_str().unwrap(),
            "--ticks",
            "20",
            "--llm",
            "mock",
            "--quiet",
        ])
        .output()
        .expect("treated2");
    assert!(treated2.status.success());
    assert_eq!(hash_b, parse_hash(&treated2.stdout));
}

fn tmp_dir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("agentplace-m9-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn compare_identical_dirs_hashes_equal() {
    let dir = tmp_dir("same");
    let run = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "8",
            "--llm",
            "mock",
            "--quiet",
            "--out-dir",
            dir.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let out = Command::new(bin())
        .args(["--compare", dir.to_str().unwrap(), dir.to_str().unwrap()])
        .output()
        .expect("compare");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("equal"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn compare_coop_vs_baseline_differs() {
    let base_dir = tmp_dir("base");
    let coop_dir = tmp_dir("coop");
    let base = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "8",
            "--llm",
            "mock",
            "--quiet",
            "--out-dir",
            base_dir.to_str().unwrap(),
        ])
        .output()
        .expect("base");
    assert!(
        base.status.success(),
        "{}",
        String::from_utf8_lossy(&base.stderr)
    );
    let coop = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--incentives",
            schedule().to_str().unwrap(),
            "--ticks",
            "8",
            "--llm",
            "mock",
            "--quiet",
            "--out-dir",
            coop_dir.to_str().unwrap(),
        ])
        .output()
        .expect("coop");
    assert!(
        coop.status.success(),
        "{}",
        String::from_utf8_lossy(&coop.stderr)
    );
    let out = Command::new(bin())
        .args([
            "--compare",
            base_dir.to_str().unwrap(),
            coop_dir.to_str().unwrap(),
        ])
        .output()
        .expect("compare");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("differ"), "{stdout}");
    assert!(
        stdout.contains("keep the shared storage stocked"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&base_dir);
    let _ = std::fs::remove_dir_all(&coop_dir);
}
