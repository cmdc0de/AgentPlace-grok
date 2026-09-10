//! M7 attach tests. Spawn `sim-cli --listen` so we do not put sockets in sim-core.

use shared::PROTOCOL_VERSION;
use shared::protocol::{ClientMessage, ControlVerb, ErrorCode, ServerMessage, hello};
use shared::transport::Connection;
use sim_core::{Simulation, parse_item};
use std::io::{BufRead, BufReader, Write};
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
fn inject_incentive_without_flag_errors() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::InjectIncentive {
        schedule_toml: r#"
[[incentives]]
id = "x"
[[incentives.effects]]
type = "goal_injection"
goal_text = "cooperate"
"#
        .into(),
    })
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
fn inject_incentive_with_flag() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::InjectIncentive {
        schedule_toml: r#"
[[incentives]]
id = "wire_inject"
start_tick = 0
applies_to = "all"
[[incentives.effects]]
type = "goal_injection"
goal_text = "cooperate"
scope = "personal"
priority = 0.9
"#
        .into(),
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("injected"), "{markdown_or_path}");
        }
        other => panic!("expected ReportReady inject, got {other:?}"),
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
    assert_eq!(PROTOCOL_VERSION, 5);
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

#[test]
fn connect_log_tail_hash_neutral() {
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
    let client = Command::new(bin())
        .args(["--connect", &url, "--quiet"])
        .output()
        .expect("connect");
    assert!(
        client.status.success(),
        "connect failed: {}\n{}",
        String::from_utf8_lossy(&client.stderr),
        String::from_utf8_lossy(&client.stdout)
    );
    let cout = String::from_utf8_lossy(&client.stdout);
    assert!(cout.contains("welcome"), "{cout}");
    assert!(cout.contains("snapshot ok"), "{cout}");
    assert!(cout.contains("tick="), "{cout}");
    let hash_b = wait_hash(&mut child, out_h, err_h);
    assert_eq!(hash_a, hash_b, "--connect must not change state_hash");
}

#[test]
fn listen_and_connect_is_error() {
    let out = Command::new(bin())
        .args([
            "--listen",
            "tcp://127.0.0.1:0",
            "--connect",
            "tcp://127.0.0.1:9",
        ])
        .output()
        .expect("run");
    assert!(!out.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(err.contains("listen") && err.contains("connect"), "{err}");
}

#[test]
fn hello_v2_against_v3_is_protocol_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let mut conn = Connection::connect(&url).expect("connect");
    conn.send_msg(&ClientMessage::Hello {
        protocol_version: 2,
        token: None,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Protocol,
            message,
        } => assert!(message.contains("2"), "{message}"),
        other => panic!("expected Protocol error, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn scrub_without_allow_control_is_disabled() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Scrub(4)))
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
fn scrub_forward_from_live() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let mut local = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let start = local.tick;
    let want = start + 2;
    while local.tick < want {
        assert!(local.tick());
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Scrub(want)))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("scrubbed"), "{markdown_or_path}");
        }
        other => panic!("expected ReportReady, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap2: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap2 else {
        panic!("expected Snapshot, got {snap2:?}");
    };
    let remote = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(remote.tick, want);
    assert_eq!(remote.state_hash(), local.state_hash());
    conn.send_msg(&ClientMessage::Control(ControlVerb::Scrub(want)))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap3: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap3 else {
        panic!("expected Snapshot");
    };
    let again = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(again.state_hash(), remote.state_hash());
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn scrub_reload_from_ckpt() {
    let dir = std::env::temp_dir().join(format!("m21-scrub-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_str().unwrap().to_string();
    let (mut child, url, out_h, err_h) = spawn_listen(&[
        "--allow-control",
        "--out-dir",
        &dir_s,
        "--checkpoint-every",
        "2",
    ]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Step(4)))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let at4: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = at4 else {
        panic!("expected Snapshot");
    };
    let live = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert!(live.tick >= 4, "tick {}", live.tick);
    conn.send_msg(&ClientMessage::Control(ControlVerb::Scrub(2)))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match &msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("scrubbed"), "{markdown_or_path}");
        }
        other => panic!("expected ReportReady, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let back: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = back else {
        panic!("expected Snapshot, got {back:?}");
    };
    let rewound = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(rewound.tick, 2);
    let ckpt = dir
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("_tick_2.ckpt"))
        })
        .expect("tick 2 ckpt");
    let file = Simulation::load_checkpoint(&ckpt).unwrap();
    assert_eq!(rewound.state_hash(), file.state_hash());
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn give_without_allow_control_is_disabled() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Give {
        id: 0,
        item: "berry_bush".into(),
        qty: 1,
    }))
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
fn give_berry_changes_hash() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let before = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let hash_a = before.state_hash();
    let item = parse_item("berry_bush", &before.config.world.species).expect("berry_bush");
    let n0 = before
        .agents
        .get(&sim_core::AgentId(0))
        .unwrap()
        .inventory
        .get(&item)
        .copied()
        .unwrap_or(0);
    conn.send_msg(&ClientMessage::Control(ControlVerb::Give {
        id: 0,
        item: "berry_bush".into(),
        qty: 1,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("gave"), "{markdown_or_path}");
        }
        other => panic!("expected ReportReady, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap2: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap2 else {
        panic!("expected Snapshot, got {snap2:?}");
    };
    let after = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let n1 = after
        .agents
        .get(&sim_core::AgentId(0))
        .unwrap()
        .inventory
        .get(&item)
        .copied()
        .unwrap_or(0);
    assert_eq!(n1, n0 + 1);
    assert_ne!(after.state_hash(), hash_a);
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn give_unknown_or_zero_is_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Give {
        id: 0,
        item: "nope_item".into(),
        qty: 1,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            message,
        } => assert!(
            message.contains("unknown") || message.contains("nope"),
            "{message}"
        ),
        other => panic!("expected Internal, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Give {
        id: 0,
        item: "berry_bush".into(),
        qty: 0,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            ..
        } => {}
        other => panic!("expected Internal for qty 0, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn hello_v3_against_v4_is_protocol_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let mut conn = Connection::connect(&url).expect("connect");
    conn.send_msg(&ClientMessage::Hello {
        protocol_version: 3,
        token: None,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Protocol,
            message,
        } => assert!(message.contains("3"), "{message}"),
        other => panic!("expected Protocol error, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn ckpt_next_without_files_is_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::CkptNext))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            ..
        } => {}
        other => panic!("expected Internal, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn ckpt_prev_from_tick_four_loads_two() {
    let dir = std::env::temp_dir().join(format!("m22-ckpt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_str().unwrap().to_string();
    let (mut child, url, out_h, err_h) = spawn_listen(&[
        "--allow-control",
        "--out-dir",
        &dir_s,
        "--checkpoint-every",
        "2",
    ]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Step(4)))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::Control(ControlVerb::CkptPrev))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match &msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("loaded"), "{markdown_or_path}");
        }
        other => panic!("expected ReportReady, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let loaded = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(loaded.tick, 2);
    let ckpt = dir
        .read_dir()
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("_tick_2.ckpt"))
        })
        .expect("tick 2 ckpt");
    let file = Simulation::load_checkpoint(&ckpt).unwrap();
    assert_eq!(loaded.state_hash(), file.state_hash());
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn events_without_jsonl_is_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Events(1)))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            message,
        } => assert!(
            message.contains("jsonl") || message.contains("event"),
            "{message}"
        ),
        other => panic!("expected Internal, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn events_jsonl_display_only() {
    let dir = std::env::temp_dir().join(format!("m22-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = dir.to_str().unwrap().to_string();
    let (mut child, url, out_h, err_h) = spawn_listen(&[
        "--allow-control",
        "--out-dir",
        &dir_s,
        "--checkpoint-every",
        "2",
    ]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Step(2)))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let before = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let hash = before.state_hash();
    let tick = before.tick;
    conn.send_msg(&ClientMessage::Control(ControlVerb::Events(tick)))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(
                markdown_or_path.contains("tick") || markdown_or_path.contains('{'),
                "{markdown_or_path}"
            );
        }
        other => panic!("expected ReportReady, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap2: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap2 else {
        panic!("expected Snapshot, got {snap2:?}");
    };
    let after = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(after.tick, tick);
    assert_eq!(after.state_hash(), hash);
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn start_paused_requires_listen_and_control() {
    let no_listen = Command::new(bin())
        .args(["--start-paused", "--allow-control", "--ticks", "1"])
        .output()
        .expect("run");
    assert!(!no_listen.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&no_listen.stderr),
        String::from_utf8_lossy(&no_listen.stdout)
    );
    assert!(err.contains("--listen"), "{err}");

    let no_control = Command::new(bin())
        .args([
            "--config",
            config().to_str().unwrap(),
            "--listen",
            "tcp://127.0.0.1:0",
            "--start-paused",
            "--ticks",
            "1",
        ])
        .output()
        .expect("run");
    assert!(!no_control.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&no_control.stderr),
        String::from_utf8_lossy(&no_control.stdout)
    );
    assert!(err.contains("--allow-control"), "{err}");
}

#[test]
fn start_paused_holds_tick_until_play() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let (mut conn, welcome, _) = dummy_read_hello(&url, None).unwrap();
    match welcome {
        ServerMessage::Welcome { tick, .. } => assert_eq!(tick, 0),
        other => panic!("{other:?}"),
    }
    thread::sleep(Duration::from_millis(250));
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let sim = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(sim.tick, 0, "must not tick while start-paused");
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn viewer_handshake_does_not_unpause_start_paused() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let (mut conn, welcome, _) = dummy_read_hello(&url, None).unwrap();
    match welcome {
        ServerMessage::Welcome { tick, .. } => assert_eq!(tick, 0),
        other => panic!("{other:?}"),
    }
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })
    .unwrap();
    let report: ServerMessage = conn.recv_msg().unwrap();
    match report {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert_eq!(markdown_or_path, "paused");
        }
        other => panic!("expected paused, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::AckTick(0)).unwrap();
    thread::sleep(Duration::from_millis(300));
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("expected Snapshot, got {snap:?}");
    };
    let sim = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(
        sim.tick, 0,
        "Subscribe+AckTick must not start a --start-paused server"
    );
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn subscribe_reports_paused_when_start_paused() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: false,
        want_decisions: false,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert_eq!(markdown_or_path, "paused");
        }
        other => panic!("expected paused, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn subscribe_reports_playing_when_running() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: false,
        want_decisions: false,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert_eq!(markdown_or_path, "playing", "{markdown_or_path}");
        }
        other => panic!("expected playing, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn hello_v4_against_v5_is_protocol_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let mut conn = Connection::connect(&url).expect("connect");
    conn.send_msg(&ClientMessage::Hello {
        protocol_version: 4,
        token: None,
    })
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Protocol,
            message,
        } => assert!(message.contains("4"), "{message}"),
        other => panic!("expected Protocol error, got {other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

fn spawn_connect(url: &str, extra: &[&str]) -> Child {
    Command::new(bin())
        .args(["--connect", url])
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn connect")
}

fn wait_stdout_contains(client: &mut Child, needle: &str, secs: u64) -> String {
    let out = client.stdout.take().expect("stdout");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let want = needle.to_string();
    thread::spawn(move || {
        let mut all = String::new();
        let reader = BufReader::new(out);
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            all.push_str(&line);
            all.push('\n');
            if all.contains(&want) {
                let _ = tx.send(all.clone());
            }
        }
        let _ = tx.send(all);
    });
    rx.recv_timeout(Duration::from_secs(secs))
        .unwrap_or_else(|_| panic!("timeout waiting for {needle}"))
}

#[test]
fn connect_allow_control_play_unpauses() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let mut client = spawn_connect(&url, &["--allow-control"]);
    {
        let mut stdin = client.stdin.take().expect("stdin");
        stdin.write_all(b"/play\n").unwrap();
        drop(stdin);
    }
    let cout = wait_stdout_contains(&mut client, "tick=", 20);
    assert!(cout.contains("tick="), "{cout}");
    let _ = client.wait();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn connect_allow_control_give_changes_hash() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let mut client = spawn_connect(&url, &["--allow-control"]);
    let mut stdin = client.stdin.take().expect("stdin");
    stdin.write_all(b"/give 0 berry_bush 1\n").unwrap();
    let cout = wait_stdout_contains(&mut client, "gave", 20);
    assert!(cout.contains("gave"), "{cout}");

    let (mut probe, _, _) = dummy_read_hello(&url, None).unwrap();
    probe.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = probe.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("{snap:?}");
    };
    let sim = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let item = parse_item("berry_bush", &sim.config.world.species).expect("berry");
    let n = sim
        .agents
        .get(&sim_core::AgentId(0))
        .map(|a| a.inventory.get(&item).copied().unwrap_or(0))
        .unwrap_or(0);
    assert!(n >= 1, "inventory {n}");
    stdin.write_all(b"/play\n").unwrap();
    drop(stdin);
    let _ = probe.close();
    let _ = client.wait();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn connect_allow_control_without_server_flag_is_disabled() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let mut client = spawn_connect(&url, &["--allow-control"]);
    {
        let mut stdin = client.stdin.take().expect("stdin");
        stdin.write_all(b"/pause\n").unwrap();
        drop(stdin);
    }
    let cout = wait_stdout_contains(&mut client, "error:", 20);
    assert!(
        cout.to_ascii_lowercase().contains("control") || cout.contains("error:"),
        "{cout}"
    );
    let _ = client.wait();
    let _ = wait_hash(&mut child, out_h, err_h);
}

fn drain_until_tick(conn: &mut Connection) -> u64 {
    conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    loop {
        match conn.recv_msg::<ServerMessage>().unwrap() {
            ServerMessage::Tick { tick, .. } => return tick,
            ServerMessage::ReportReady { .. } | ServerMessage::Snapshot { .. } => {}
            other => panic!("expected Tick, got {other:?}"),
        }
    }
}

#[test]
fn ckpt_events_without_allow_control_is_disabled() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::CkptNext))
        .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::ControlDisabled,
            ..
        } => {}
        other => panic!("expected ControlDisabled, got {other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Events(1)))
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
fn set_without_allow_control_is_disabled() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 0,
        field: "hunger".into(),
        toward: None,
        value: 50,
    }))
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
fn set_hunger_changes_hash() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("{snap:?}");
    };
    let before = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let hash_a = before.state_hash();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 0,
        field: "hunger".into(),
        toward: None,
        value: 50,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(
                markdown_or_path.contains("set agent 0"),
                "{markdown_or_path}"
            );
        }
        other => panic!("{other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap2: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap2 else {
        panic!("{snap2:?}");
    };
    let after = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(
        after
            .agents
            .get(&sim_core::AgentId(0))
            .unwrap()
            .needs
            .hunger,
        5000
    );
    assert_ne!(hash_a, after.state_hash());
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn set_respect_changes_hash() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Pause))
        .unwrap();
    let _ = conn.recv_msg::<ServerMessage>();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 0,
        field: "respect".into(),
        toward: Some(1),
        value: 40,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::ReportReady { markdown_or_path } => {
            assert!(markdown_or_path.contains("respect"), "{markdown_or_path}");
        }
        other => panic!("{other:?}"),
    }
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("{snap:?}");
    };
    let sim = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    let r = sim
        .agents
        .get(&sim_core::AgentId(0))
        .unwrap()
        .relationships
        .get(&sim_core::AgentId(1))
        .map(|e| e.respect)
        .unwrap_or(0);
    assert_eq!(r, 4000);
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn set_unknown_or_missing_toward_is_error() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control"]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 0,
        field: "nope".into(),
        toward: None,
        value: 1,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            ..
        } => {}
        other => panic!("{other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 0,
        field: "respect".into(),
        toward: None,
        value: 40,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            message,
        } => assert!(message.contains("toward"), "{message}"),
        other => panic!("{other:?}"),
    }
    conn.send_msg(&ClientMessage::Control(ControlVerb::Set {
        id: 99,
        field: "hunger".into(),
        toward: None,
        value: 1,
    }))
    .unwrap();
    let msg: ServerMessage = conn.recv_msg().unwrap();
    match msg {
        ServerMessage::Error {
            code: ErrorCode::Internal,
            ..
        } => {}
        other => panic!("{other:?}"),
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn lockstep_zero_subscribers_finishes() {
    let (mut child, _url, out_h, err_h) = spawn_listen(&["--lockstep", "--ticks", "2"]);
    let hash = wait_hash(&mut child, out_h, err_h);
    assert!(!hash.is_empty());
}

#[test]
fn lockstep_waits_for_ack_then_advances() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[
        "--allow-control",
        "--start-paused",
        "--lockstep",
        "--ticks",
        "4",
    ]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: false,
        want_decisions: false,
    })
    .unwrap();
    let _ = conn.recv_msg::<ServerMessage>().unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let t1 = drain_until_tick(&mut conn);
    assert_eq!(t1, 1);
    thread::sleep(Duration::from_millis(250));
    conn.send_msg(&ClientMessage::RequestSnapshot).unwrap();
    let snap: ServerMessage = conn.recv_msg().unwrap();
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        panic!("{snap:?}");
    };
    let held = Simulation::decode_checkpoint(&checkpoint_bytes).unwrap();
    assert_eq!(held.tick, 1, "must wait for AckTick");
    conn.send_msg(&ClientMessage::AckTick(1)).unwrap();
    let t2 = drain_until_tick(&mut conn);
    assert_eq!(t2, 2);
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn lockstep_timeout_advances_without_ack() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[
        "--allow-control",
        "--start-paused",
        "--lockstep",
        "--lockstep-timeout-ms",
        "200",
        "--ticks",
        "4",
    ]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: false,
        want_decisions: false,
    })
    .unwrap();
    let _ = conn.recv_msg::<ServerMessage>().unwrap();
    conn.send_msg(&ClientMessage::Control(ControlVerb::Play))
        .unwrap();
    let t1 = drain_until_tick(&mut conn);
    assert_eq!(t1, 1);
    let t2 = drain_until_tick(&mut conn);
    assert!(t2 >= 2, "timeout should advance, got tick {t2}");
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn connect_auto_acks_lockstep() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--lockstep", "--ticks", "6"]);
    let client = Command::new(bin())
        .args(["--connect", &url, "--quiet"])
        .output()
        .expect("connect");
    assert!(
        client.status.success(),
        "{}",
        String::from_utf8_lossy(&client.stderr)
    );
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn connect_inject_applies_schedule() {
    let coop = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/incentives/coop.toml");
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let mut client = spawn_connect(&url, &["--allow-control"]);
    let mut stdin = client.stdin.take().expect("stdin");
    let line = format!("/inject {}\n", coop.display());
    stdin.write_all(line.as_bytes()).unwrap();
    let cout = wait_stdout_contains(&mut client, "injected", 20);
    assert!(cout.contains("injected"), "{cout}");
    stdin.write_all(b"/play\n").unwrap();
    drop(stdin);
    let _ = client.wait();
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn connect_inject_missing_file_is_stderr() {
    let (mut child, url, out_h, err_h) = spawn_listen(&["--allow-control", "--start-paused"]);
    let mut client = spawn_connect(&url, &["--allow-control"]);
    {
        let mut stdin = client.stdin.take().expect("stdin");
        stdin
            .write_all(b"/inject /no/such/incentive.toml\n/play\n")
            .unwrap();
        drop(stdin);
    }
    let err = {
        let mut s = String::new();
        if let Some(mut e) = client.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut e, &mut s);
        }
        s
    };
    let status = client.wait().expect("wait connect");
    assert!(
        err.contains("inject read error") || err.contains("No such file"),
        "status={status:?} err={err}"
    );
    let _ = wait_hash(&mut child, out_h, err_h);
}

#[test]
fn browser_page_ships_protocol_5() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/index.html");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(text.contains("PROTOCOL_VERSION = 5"), "{text}");
    assert!(text.contains("postcard"), "{text}");
    assert!(text.contains("ws://"), "{text}");
    assert!(text.contains("id=\"agents\""), "agents section");
    assert!(text.contains("id=\"board\""), "board section");
    assert!(text.contains("id=\"metrics\""), "metrics section");
    assert!(text.contains("encodeSubscribe"), "{text}");
    assert!(text.contains("encodeRequestSnapshot"), "{text}");
    assert!(text.contains("inspector"), "{text}");
    assert!(text.contains("id=\"inventions\""), "inventions section");
    assert!(text.contains("encodeGive"), "{text}");
    assert!(text.contains("encodeSet"), "{text}");
    assert!(text.contains("encodeInject"), "{text}");
    assert!(text.contains("encodeScrub"), "{text}");
    assert!(text.contains("encodeCkptNext"), "{text}");
    assert!(text.contains("encodeCkptPrev"), "{text}");
    assert!(text.contains("encodeEvents"), "{text}");
    assert!(text.contains("sim_wasm"), "{text}");
}

#[test]
fn tick_metrics_include_inspector() {
    let (mut child, url, out_h, err_h) = spawn_listen(&[]);
    let (mut conn, _, _) = dummy_read_hello(&url, None).unwrap();
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })
    .unwrap();
    conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    loop {
        match conn.recv_msg::<ServerMessage>().unwrap() {
            ServerMessage::Tick { metrics, .. } => {
                let v: serde_json::Value = serde_json::from_slice(&metrics).expect("metrics json");
                assert!(v.get("inspector").is_some(), "{v}");
                let agents = v["inspector"]["agents"].as_array().expect("agents");
                assert!(!agents.is_empty(), "{v}");
                assert!(v["inspector"]["board"].is_object(), "{v}");
                assert!(v["inspector"]["metrics"].is_object(), "{v}");
                let t: sim_core::TickTiming = serde_json::from_slice(&metrics).unwrap();
                assert!(t.tick > 0);
                break;
            }
            ServerMessage::ReportReady { .. } | ServerMessage::Snapshot { .. } => {}
            other => panic!("expected Tick, got {other:?}"),
        }
    }
    let _ = conn.close();
    let _ = wait_hash(&mut child, out_h, err_h);
}
