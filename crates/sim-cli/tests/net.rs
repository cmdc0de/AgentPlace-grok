//! M7 attach tests. Spawn `sim-cli --listen` so we do not put sockets in sim-core.

use shared::protocol::{hello, ClientMessage, ControlVerb, ErrorCode, ServerMessage};
use shared::transport::Connection;
use shared::PROTOCOL_VERSION;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sim-cli"))
}

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml")
}

fn parse_hash(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("final_hash="))
        .expect("final_hash line")
        .to_string()
}

fn spawn_listen(
    extra: &[&str],
) -> (
    Child,
    String,
    thread::JoinHandle<String>,
    thread::JoinHandle<String>,
) {
    let mut child = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "40",
            "--llm",
            "mock",
            "--quiet",
            "--listen",
            "tcp://127.0.0.1:0",
        ])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sim-cli");

    let stderr = child.stderr.take().expect("stderr");
    let stdout = child.stdout.take().expect("stdout");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let err_h = thread::spawn(move || {
        let mut listen = None;
        let reader = BufReader::new(stderr);
        let mut all = String::new();
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            all.push_str(&line);
            all.push('\n');
            if let Some(url) = line.strip_prefix("listen=") {
                let _ = tx.send(url.to_string());
                listen = Some(url.to_string());
            }
        }
        if listen.is_none() {
            let _ = tx.send(String::new());
        }
        all
    });
    let out_h = thread::spawn(move || {
        let mut s = String::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            s.push_str(&line.unwrap_or_default());
            s.push('\n');
        }
        s
    });
    let url = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("listen url on stderr");
    assert!(url.starts_with("tcp://"), "bad listen url {url:?}");
    (child, url, out_h, err_h)
}

fn wait_hash(
    child: &mut Child,
    out_h: thread::JoinHandle<String>,
    err_h: thread::JoinHandle<String>,
) -> String {
    let status = child.wait().expect("wait sim-cli");
    let stdout = out_h.join().unwrap();
    let stderr = err_h.join().unwrap();
    assert!(status.success(), "sim-cli failed: {stderr}\n{stdout}");
    parse_hash(&stdout)
}

fn dummy_read_hello(
    url: &str,
    token: Option<String>,
) -> Result<(Connection, ServerMessage, ServerMessage), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = String::new();
    while Instant::now() < deadline {
        match Connection::connect(url) {
            Ok(mut conn) => {
                conn.send_msg(&hello(token.clone()))
                    .map_err(|e| e.to_string())?;
                let a: ServerMessage = conn.recv_msg().map_err(|e| e.to_string())?;
                let b: ServerMessage = conn.recv_msg().map_err(|e| e.to_string())?;
                return Ok((conn, a, b));
            }
            Err(e) => {
                last = e.to_string();
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Err(last)
}

#[test]
fn hash_neutral_attach() {
    let no = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "40",
            "--llm",
            "mock",
            "--quiet",
        ])
        .output()
        .expect("run without listen");
    assert!(
        no.status.success(),
        "{}",
        String::from_utf8_lossy(&no.stderr)
    );
    let hash_a = parse_hash(&String::from_utf8_lossy(&no.stdout));

    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, welcome, snap) = dummy_read_hello(&url, None).expect("dummy connect");
    match welcome {
        ServerMessage::Welcome { .. } => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    match snap {
        ServerMessage::Snapshot { checkpoint_bytes } => {
            assert!(checkpoint_bytes.starts_with(b"AGTN"), "checkpoint magic");
        }
        other => panic!("expected Snapshot, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })
    .unwrap();
    let hash_b = wait_hash(&mut child, out_h, err_h);
    let _ = conn.close();
    assert_eq!(
        hash_a, hash_b,
        "read-only subscriber must not change state_hash"
    );
}

#[test]
fn unknown_protocol_version_errors() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let mut conn = Connection::connect(&url).expect("connect");
    conn.send_msg(&ClientMessage::Hello {
        protocol_version: 99,
        token: None,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Protocol,
            message,
        } => assert!(message.contains("99"), "{message}"),
        other => panic!("expected Protocol error, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn control_without_flag_errors() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::ControlDisabled,
            ..
        } => {}
        other => panic!("expected ControlDisabled, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn inject_incentive_not_implemented() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::InjectIncentive {
        schedule_toml: "[[incentives]]".into(),
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::NotImplemented,
            message,
        } => assert!(message.contains("not implemented"), "{message}"),
        other => panic!("expected NotImplemented, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn disconnect_then_second_client() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.close().unwrap();
    let (mut conn2, welcome, _) = dummy_read_hello(&url, None).unwrap();
    match welcome {
        ServerMessage::Welcome { .. } => {}
        other => panic!("second client expected Welcome, got {other:?}"),
    }
    conn2
        .send_msg(&ClientMessage::Subscribe {
            want_events: false,
            want_decisions: false,
        })
        .unwrap();
    let _ = conn2.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn control_pause_with_flag() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("paused"), "{markdown_or_path}");
        }
        other => panic!("expected paused, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn protocol_version_constant() {
    assert_eq!(PROTOCOL_VERSION, 1);
}

#[test]
fn token_mismatch_unauthorized() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--token", "s3cret"]);
    let mut conn = Connection::connect(&url).expect("connect");
    conn.send_msg(&hello(Some("nope".into()))).unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Unauthorized,
            ..
        } => {}
        other => panic!("expected Unauthorized, got {other:?}"),
    }
    let _ = conn.close();
    let (conn_ok, welcome, _) = dummy_read_hello(&url, Some("s3cret".into())).unwrap();
    match welcome {
        ServerMessage::Welcome { .. } => {}
        other => panic!("good token should Welcome, got {other:?}"),
    }
    let _ = conn_ok.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn ws_loopback_hello_snapshot() {
    let mut child = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--ticks",
            "40",
            "--llm",
            "mock",
            "--quiet",
            "--listen",
            "ws://127.0.0.1:0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let err_h = thread::spawn(move || {
        let mut all = String::new();
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap_or_default();
            all.push_str(&line);
            all.push('\n');
            if let Some(url) = line.strip_prefix("listen=") {
                let _ = tx.send(url.to_string());
            }
        }
        all
    });
    let out_h = thread::spawn(move || {
        let mut s = String::new();
        for line in BufReader::new(stdout).lines() {
            s.push_str(&line.unwrap_or_default());
            s.push('\n');
        }
        s
    });
    let url = rx.recv_timeout(Duration::from_secs(20)).expect("ws url");
    assert!(url.starts_with("ws://"), "{url}");
    let (conn, welcome, snap) = dummy_read_hello(&url, None).expect("ws dummy");
    match welcome {
        ServerMessage::Welcome { .. } => {}
        other => panic!("{other:?}"),
    }
    match snap {
        ServerMessage::Snapshot { checkpoint_bytes } => {
            assert!(checkpoint_bytes.starts_with(b"AGTN"));
        }
        other => panic!("{other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}
