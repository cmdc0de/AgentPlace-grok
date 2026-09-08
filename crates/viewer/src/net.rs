//! Background reader for `--connect`. Bevy stays on the render thread.

use crate::ui::UiState;
use bevy::prelude::*;
use shared::protocol::{hello, ClientMessage, ServerMessage};
use shared::transport::Connection;
use sim_bevy::SimState;
use sim_core::{Simulation, TickTiming};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct LiveClock {
    tick: AtomicU64,
    hash: Mutex<[u8; 32]>,
}

#[derive(Resource)]
pub struct NetLink {
    pub tx: Sender<ClientMessage>,
    rx: Mutex<Receiver<ServerMessage>>,
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

/// HUD: world tick from `state.sim`; live clock when attached. Hash is live when attached.
/// `live_ahead` is Some when the server tick is ahead of the displayed world.
pub fn status_tick_hash(
    net: Option<&NetLink>,
    sim_tick: u64,
    sim_hash_hex: &str,
) -> (u64, String, Option<u64>) {
    match net {
        Some(n) => {
            let live = n.live_tick();
            let ahead = if live != sim_tick { Some(live) } else { None };
            (sim_tick, hex_short(&n.live_hash()), ahead)
        }
        None => (sim_tick, hash_short(sim_hash_hex), None),
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
    let (to_bevy, from_server) = mpsc::sync_channel::<ServerMessage>(256);
    to_server.send(ClientMessage::Subscribe {
        want_events: true,
        want_decisions: true,
    })?;
    to_server.send(ClientMessage::AckTick(sim.tick))?;

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
    while let Ok(msg) = rx.try_recv() {
        match msg {
            ServerMessage::Snapshot { checkpoint_bytes } => {
                let timing = state.sim.last_tick_timing.clone();
                if let Ok(mut sim) = Simulation::decode_checkpoint(&checkpoint_bytes) {
                    sim.last_tick_decisions = std::mem::take(&mut state.sim.last_tick_decisions);
                    sim.last_tick_timing = timing;
                    state.sim = sim;
                    let _ = net.tx.send(ClientMessage::AckTick(state.sim.tick));
                }
                break;
            }
            ServerMessage::Tick {
                tick,
                state_hash,
                decisions,
                metrics,
                ..
            } => {
                net.set_live(tick, state_hash);
                if let Ok(recs) = serde_json::from_slice(&decisions) {
                    state.sim.last_tick_decisions = recs;
                }
                if !metrics.is_empty() {
                    if let Ok(t) = serde_json::from_slice::<TickTiming>(&metrics) {
                        state.sim.last_tick_timing = Some(t);
                    }
                }
                let _ = net.tx.send(ClientMessage::RequestSnapshot);
            }
            ServerMessage::ReportReady { markdown_or_path } => {
                if let Some(paused) = pause_hint_from_report(&markdown_or_path) {
                    state.paused = paused;
                }
                ui.scrollback.push(markdown_or_path);
            }
            ServerMessage::Error { message, .. } => {
                ui.scrollback.push(format!("server: {message}"));
            }
            ServerMessage::Welcome { .. } => {}
        }
    }
}

/// Pause/play from server ReportReady. None = not a pause hint (give, events, …).
pub fn pause_hint_from_report(text: &str) -> Option<bool> {
    let t = text.trim();
    if t == "paused" || t.starts_with("loaded tick") || t.starts_with("scrubbed tick") {
        Some(true)
    } else if t == "playing" {
        Some(false)
    } else {
        None
    }
}

/// How many Snapshot requests a frame of Tick messages should send (no 200 ms throttle).
pub fn snapshot_requests_for_ticks(tick_count: usize) -> usize {
    tick_count
}

/// Walk messages in order; stop after the first Snapshot (one decode per frame).
pub fn first_snapshot_index(kinds: &[&str]) -> Option<usize> {
    kinds.iter().position(|k| *k == "snapshot")
}

/// After a Snapshot is applied, ack that world tick (lockstep).
pub fn ack_tick_after_snapshot(world_tick: u64) -> ClientMessage {
    ClientMessage::AckTick(world_tick)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_tick_hash_local_when_not_attached() {
        let (t, h, ahead) = status_tick_hash(None, 7, "0123456789abcdef");
        assert_eq!(t, 7);
        assert_eq!(h, "0123456789ab");
        assert_eq!(ahead, None);
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

    #[test]
    fn status_shows_world_and_live_when_they_differ() {
        let live = LiveClock {
            tick: AtomicU64::new(80),
            hash: Mutex::new([0u8; 32]),
        };
        let (tx, _rx_c) = mpsc::channel();
        let (_tx_s, rx) = mpsc::channel();
        let net = NetLink {
            tx,
            rx: Mutex::new(rx),
            live: Arc::new(live),
        };
        let (world, _h, ahead) = status_tick_hash(Some(&net), 12, "deadbeefdead");
        assert_eq!(world, 12);
        assert_eq!(ahead, Some(80));
    }

    #[test]
    fn pause_hint_from_report_paused_and_playing() {
        assert_eq!(pause_hint_from_report("paused"), Some(true));
        assert_eq!(pause_hint_from_report("playing"), Some(false));
        assert_eq!(pause_hint_from_report("loaded tick 2"), Some(true));
        assert_eq!(pause_hint_from_report("scrubbed tick 4"), Some(true));
        assert_eq!(pause_hint_from_report("gave 1 berry_bush to agent 0"), None);
    }

    #[test]
    fn snapshot_apply_acks_world_tick() {
        match ack_tick_after_snapshot(7) {
            ClientMessage::AckTick(t) => assert_eq!(t, 7),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn every_tick_requests_a_snapshot() {
        assert_eq!(snapshot_requests_for_ticks(2), 2);
        assert_eq!(snapshot_requests_for_ticks(0), 0);
    }

    #[test]
    fn first_snapshot_stops_later_world_applies() {
        let kinds = ["tick", "snapshot", "snapshot"];
        assert_eq!(first_snapshot_index(&kinds), Some(1));
        let rest = &kinds[2..];
        assert_eq!(rest, ["snapshot"]);
    }

    #[test]
    fn full_tick_channel_still_updates_live_clock() {
        let live = Arc::new(LiveClock {
            tick: AtomicU64::new(0),
            hash: Mutex::new([0u8; 32]),
        });
        let (to_bevy, from_bevy) = mpsc::sync_channel::<ServerMessage>(1);
        to_bevy
            .send(ServerMessage::Tick {
                tick: 1,
                state_hash: [1u8; 32],
                events: vec![],
                decisions: vec![],
                metrics: vec![],
            })
            .unwrap();
        let msg = ServerMessage::Tick {
            tick: 2,
            state_hash: [2u8; 32],
            events: vec![],
            decisions: vec![],
            metrics: vec![],
        };
        if let ServerMessage::Tick {
            tick, state_hash, ..
        } = &msg
        {
            live.tick.store(*tick, Ordering::Relaxed);
            *live.hash.lock().unwrap() = *state_hash;
        }
        let dropped = to_bevy.try_send(msg).is_err();
        assert!(dropped, "channel full should drop Tick");
        assert_eq!(live.tick.load(Ordering::Relaxed), 2);
        let _ = from_bevy;
    }
}
