//! Background reader for `--connect`. Bevy stays on the render thread.

use crate::ui::UiState;
use bevy::prelude::*;
use shared::protocol::{hello, ClientMessage, ServerMessage};
use shared::transport::Connection;
use sim_bevy::SimState;
use sim_core::{Simulation, TickTiming};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Resource)]
pub struct NetLink {
    pub tx: Sender<ClientMessage>,
    rx: Mutex<Receiver<ServerMessage>>,
    last_snapshot_req: Mutex<Instant>,
}

pub fn connect(
    url: &str,
    token: Option<String>,
) -> Result<(Simulation, NetLink), Box<dyn std::error::Error>> {
    let mut conn = Connection::connect(url)?;
    conn.set_read_timeout(Some(Duration::from_secs(15)))?;
    conn.send_msg(&hello(token))?;
    let welcome: ServerMessage = conn.recv_msg()?;
    match &welcome {
        ServerMessage::Welcome {
            tick,
            experiment_id,
            ..
        } => {
            eprintln!("connected tick={tick} experiment={experiment_id}");
        }
        ServerMessage::Error { message, .. } => {
            return Err(format!("server rejected Hello: {message}").into());
        }
        other => return Err(format!("expected Welcome, got {other:?}").into()),
    }
    let snap: ServerMessage = conn.recv_msg()?;
    let ServerMessage::Snapshot { checkpoint_bytes } = snap else {
        return Err(format!("expected Snapshot, got {snap:?}").into());
    };
    let sim = Simulation::decode_checkpoint(&checkpoint_bytes)?;

    let (to_server, from_bevy) = mpsc::channel::<ClientMessage>();
    let (to_bevy, from_server) = mpsc::sync_channel::<ServerMessage>(64);
    to_server.send(ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })?;

    thread::spawn(move || {
        if let Err(e) = io_loop(conn, from_bevy, to_bevy) {
            eprintln!("net: viewer io ended: {e}");
        }
    });

    Ok((
        sim,
        NetLink {
            tx: to_server,
            rx: Mutex::new(from_server),
            last_snapshot_req: Mutex::new(Instant::now()),
        },
    ))
}

fn io_loop(
    mut conn: Connection,
    from_bevy: Receiver<ClientMessage>,
    to_bevy: mpsc::SyncSender<ServerMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.set_read_timeout(Some(Duration::from_millis(25)))?;
    loop {
        while let Ok(msg) = from_bevy.try_recv() {
            conn.send_msg(&msg)?;
        }
        match conn.recv_msg::<ServerMessage>() {
            Ok(msg) => {
                if matches!(msg, ServerMessage::Tick { .. }) {
                    let _ = to_bevy.try_send(msg);
                } else {
                    let _ = to_bevy.send(msg);
                }
            }
            Err(shared::transport::TransportError::Timeout) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

pub fn apply_remote(
    mut state: ResMut<SimState>,
    mut ui: ResMut<UiState>,
    net: Option<Res<NetLink>>,
) {
    let Some(net) = net else {
        return;
    };
    let Ok(rx) = net.rx.lock() else {
        return;
    };
    let mut saw_tick = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            ServerMessage::Snapshot { checkpoint_bytes } => {
                let timing = state.sim.last_tick_timing.clone();
                if let Ok(mut sim) = Simulation::decode_checkpoint(&checkpoint_bytes) {
                    sim.last_tick_decisions = std::mem::take(&mut state.sim.last_tick_decisions);
                    sim.last_tick_timing = timing;
                    state.sim = sim;
                }
            }
            ServerMessage::Tick {
                tick,
                decisions,
                metrics,
                ..
            } => {
                saw_tick = true;
                if let Ok(recs) = serde_json::from_slice(&decisions) {
                    state.sim.last_tick_decisions = recs;
                }
                if !metrics.is_empty() {
                    if let Ok(t) = serde_json::from_slice::<TickTiming>(&metrics) {
                        state.sim.last_tick_timing = Some(t);
                    }
                }
                let _ = tick;
            }
            ServerMessage::ReportReady { markdown_or_path } => {
                ui.scrollback.push(markdown_or_path);
            }
            ServerMessage::Error { message, .. } => {
                ui.scrollback.push(format!("server: {message}"));
            }
            ServerMessage::Welcome { .. } => {}
        }
    }
    if saw_tick {
        let mut last = net.last_snapshot_req.lock().unwrap();
        if last.elapsed() >= Duration::from_millis(200) {
            *last = Instant::now();
            let _ = net.tx.send(ClientMessage::RequestSnapshot);
        }
    }
}
