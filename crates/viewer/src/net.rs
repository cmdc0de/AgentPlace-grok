//! Background reader for `--connect`. Bevy stays on the render thread.

use crate::ui::UiState;
use bevy::prelude::*;
use shared::protocol::{ClientMessage, ServerMessage, hello};
use shared::transport::Connection;
use sim_bevy::SimState;
use sim_core::{Simulation, TickTiming};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct LiveClock {
    tick: AtomicU64,
    hash: Mutex<[u8; 32]>,
}

#[derive(Resource)]
pub struct NetLink {
    pub tx: Sender<ClientMessage>,
    rx: Mutex<Receiver<ServerMessage>>,
    last_snapshot_req: Mutex<Instant>,
    live: Arc<LiveClock>,
}

impl NetLink {
    pub fn live_tick(&self) -> u64 {
        self.live.tick.load(Ordering::Relaxed)
    }

    pub fn live_hash(&self) -> [u8; 32] {
        *self.live.hash.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set_live(&self, tick: u64, hash: [u8; 32]) {
        self.live.tick.store(tick, Ordering::Relaxed);
        if let Ok(mut g) = self.live.hash.lock() {
            *g = hash;
        }
    }
}

/// HUD tick/hash: live Tick clock when attached, else local sim.
pub fn status_tick_hash(net: Option<&NetLink>, sim_tick: u64, sim_hash_hex: &str) -> (u64, String) {
    match net {
        Some(n) => status_from_live(n.live_tick(), n.live_hash()),
        None => (sim_tick, hash_short(sim_hash_hex)),
    }
}

fn status_from_live(tick: u64, hash: [u8; 32]) -> (u64, String) {
    (tick, hex_short(&hash))
}

fn hex_short(h: &[u8; 32]) -> String {
    h.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

fn hash_short(hex: &str) -> String {
    if hex.len() >= 12 {
        hex[..12].to_string()
    } else {
        hex.to_string()
    }
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

    let live = Arc::new(LiveClock {
        tick: AtomicU64::new(sim.tick),
        hash: Mutex::new(sim.state_hash().0),
    });
    let (to_server, from_bevy) = mpsc::channel::<ClientMessage>();
    let (to_bevy, from_server) = mpsc::sync_channel::<ServerMessage>(64);
    to_server.send(ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })?;

    {
        let live = Arc::clone(&live);
        thread::spawn(move || {
            if let Err(e) = io_loop(conn, from_bevy, to_bevy, live) {
                eprintln!("net: viewer io ended: {e}");
            }
        });
    }

    Ok((
        sim,
        NetLink {
            tx: to_server,
            rx: Mutex::new(from_server),
            last_snapshot_req: Mutex::new(Instant::now()),
            live,
        },
    ))
}

fn io_loop(
    mut conn: Connection,
    from_bevy: Receiver<ClientMessage>,
    to_bevy: mpsc::SyncSender<ServerMessage>,
    live: Arc<LiveClock>,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.set_read_timeout(Some(Duration::from_millis(25)))?;
    loop {
        while let Ok(msg) = from_bevy.try_recv() {
            conn.send_msg(&msg)?;
        }
        match conn.recv_msg::<ServerMessage>() {
            Ok(msg) => {
                if let ServerMessage::Tick {
                    tick, state_hash, ..
                } = &msg
                {
                    live.tick.store(*tick, Ordering::Relaxed);
                    if let Ok(mut g) = live.hash.lock() {
                        *g = *state_hash;
                    }
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
                    net.set_live(sim.tick, sim.state_hash().0);
                    state.sim = sim;
                }
            }
            ServerMessage::Tick {
                tick,
                state_hash,
                decisions,
                metrics,
                ..
            } => {
                saw_tick = true;
                net.set_live(tick, state_hash);
                if let Ok(recs) = serde_json::from_slice(&decisions) {
                    state.sim.last_tick_decisions = recs;
                }
                if !metrics.is_empty() {
                    if let Ok(t) = serde_json::from_slice::<TickTiming>(&metrics) {
                        state.sim.last_tick_timing = Some(t);
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_tick_hash_local_when_not_attached() {
        let (t, h) = status_tick_hash(None, 7, "0123456789abcdef");
        assert_eq!(t, 7);
        assert_eq!(h, "0123456789ab");
    }

    #[test]
    fn status_from_live_uses_server_tick_not_snapshot() {
        let mut hash = [0u8; 32];
        hash[0] = 0xab;
        hash[1] = 0xcd;
        let (t, h) = status_from_live(80, hash);
        assert_eq!(t, 80);
        assert_eq!(h, "abcd00000000");
    }
}
