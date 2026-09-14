//! M52 OTLP/JSON POST. Hash-neutral. Loopback only.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sim-cli"))
}

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml")
}

#[test]
fn otlp_endpoint_posts_json_and_keeps_hash() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let got = Arc::new(Mutex::new(String::new()));
    let got2 = Arc::clone(&got);
    listener.set_nonblocking(false).unwrap();
    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            *got2.lock().unwrap() = req;
        }
    });

    let url = format!("http://{addr}");
    let out = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "1",
            "--llm",
            "mock",
            "--quiet",
            "--no-time",
            "--otlp-endpoint",
            &url,
        ])
        .output()
        .expect("sim-cli");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = handle.join();
    let req = got.lock().unwrap().clone();
    assert!(req.starts_with("POST /v1/metrics"), "{req}");
    assert!(req.contains("agentplace.tick.wall_ns"), "{req}");
    assert!(req.contains("agentplace.process.rss_bytes"), "{req}");
    assert!(req.contains("agentplace.process.cpu_user_ns"), "{req}");
    assert!(req.contains("agentplace.process.cpu_system_ns"), "{req}");
    assert!(req.contains("agentplace.process.disk_read_bytes"), "{req}");
    assert!(req.contains("agentplace.process.disk_write_bytes"), "{req}");
    assert!(req.contains("agentplace-sim"), "{req}");

    let off = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "1",
            "--llm",
            "mock",
            "--quiet",
            "--no-time",
            "--telemetry",
        ])
        .output()
        .expect("sim-cli");
    assert!(off.status.success());
    let a = String::from_utf8_lossy(&out.stdout);
    let b = String::from_utf8_lossy(&off.stdout);
    let hash_a = a.lines().find(|l| l.starts_with("final_hash=")).unwrap();
    let hash_b = b.lines().find(|l| l.starts_with("final_hash=")).unwrap();
    assert_eq!(hash_a, hash_b, "OTLP must not hash");
}
