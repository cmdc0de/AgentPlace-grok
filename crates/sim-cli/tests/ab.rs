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
