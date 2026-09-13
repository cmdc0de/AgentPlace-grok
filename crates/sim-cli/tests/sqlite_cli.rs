//! CLI --sqlite extra sink. Hash-neutral; no JSON columns.

use rusqlite::Connection;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sim-cli"))
}

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml")
}

#[test]
fn sqlite_cli_without_out_dir_creates_db() {
    let dir = std::env::temp_dir().join(format!("m50-cli-sqlite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("run.sqlite3");
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "2",
            "--llm",
            "mock",
            "--quiet",
            "--sqlite",
            db.to_str().unwrap(),
        ])
        .output()
        .expect("sim-cli");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(db.is_file(), "expected {}", db.display());
    let conn = Connection::open(&db).unwrap();
    for table in ["ticks", "agent_timing", "decisions", "events"] {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(
            names.iter().all(|n| n != "json"),
            "{table} json col {names:?}"
        );
    }
    let n_ticks: i64 = conn
        .query_row("SELECT COUNT(*) FROM ticks", [], |r| r.get(0))
        .unwrap();
    assert!(n_ticks >= 1, "ticks rows={n_ticks}");
    let n_agents: i64 = conn
        .query_row("SELECT COUNT(*) FROM agent_timing", [], |r| r.get(0))
        .unwrap();
    assert!(n_agents >= 1, "agent_timing rows={n_agents}");
    let n_dec: i64 = conn
        .query_row("SELECT COUNT(*) FROM decisions", [], |r| r.get(0))
        .unwrap();
    assert!(n_dec >= 1, "decisions rows={n_dec}");
    let n_legal: i64 = conn
        .query_row("SELECT COUNT(*) FROM decision_legal", [], |r| r.get(0))
        .unwrap();
    assert!(n_legal >= 1, "decision_legal rows={n_legal}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("final_hash="), "{stdout}");
}

#[test]
fn sqlite_http_requires_sqlite() {
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "0",
            "--llm",
            "mock",
            "--sqlite-http",
            "127.0.0.1:0",
        ])
        .output()
        .expect("sim-cli");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--sqlite-http requires --sqlite"),
        "{err}"
    );
}
