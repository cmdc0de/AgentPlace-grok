//! Hash-neutral attachable server. Connection logs go to stderr, never `SimEvent`.

use crate::network::NetworkParams;
use shared::PROTOCOL_VERSION;
use shared::protocol::{ClientMessage, ControlVerb, ErrorCode, ServerMessage};
use shared::transport::{Connection, Listener, TransportError};
use sim_core::{
    AgentId, SimEvent, Simulation, append_decisions_jsonl, append_events_jsonl,
    append_timing_jsonl, ckpt_at_or_before, experiment_id, find_events_jsonl, jsonl_lines_for_tick,
    jsonl_tick_at_or_before, list_checkpoints, list_jsonl_ticks, parse_item, summary_markdown,
    write_report, write_run_checkpoint,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct Subscriber {
    tx: Sender<ServerMessage>,
    want_events: bool,
    want_decisions: bool,
    subscribed: bool,
    acked_tick: Option<u64>,
}

pub struct Hub {
    pub sim: Simulation,
    paused: bool,
    allow_control: bool,
    token: Option<String>,
    jsonl_path: Option<PathBuf>,
    decisions_path: Option<PathBuf>,
    timing_path: Option<PathBuf>,
    last_event: usize,
    out_dir: Option<PathBuf>,
    interval: u64,
    subscribers: HashMap<u64, Subscriber>,
}

impl Hub {
    fn welcome(&self) -> ServerMessage {
        let id = self
            .sim
            .config_hash()
            .map(|h| experiment_id(&h))
            .unwrap_or_else(|_| "unknown".into());
        ServerMessage::Welcome {
            tick: self.sim.tick,
            state_hash: self.sim.state_hash().0,
            experiment_id: id,
        }
    }

    fn snapshot(&self) -> Result<ServerMessage, String> {
        let bytes = self.sim.encode_checkpoint().map_err(|e| e.to_string())?;
        Ok(ServerMessage::Snapshot {
            checkpoint_bytes: bytes,
        })
    }

    fn tick_once(&mut self) -> bool {
        if !self.sim.tick() {
            return false;
        }
        if let Some(path) = &self.jsonl_path {
            let events = &self.sim.events.events;
            if events.len() > self.last_event {
                let _ = append_events_jsonl(path, &events[self.last_event..]);
                self.last_event = events.len();
            }
        }
        if let Some(path) = &self.decisions_path {
            let _ = append_decisions_jsonl(path, &self.sim.last_tick_decisions);
        }
        if let (Some(path), Some(t)) = (&self.timing_path, &self.sim.last_tick_timing) {
            let _ = append_timing_jsonl(path, t);
        }
        if let Some(dir) = &self.out_dir {
            if self.interval > 0 && self.sim.tick % self.interval == 0 {
                let _ = write_run_checkpoint(&self.sim, dir);
            }
        }
        self.broadcast_tick();
        true
    }

    fn broadcast_tick(&self) {
        let tick = self.sim.tick;
        let state_hash = self.sim.state_hash().0;
        let tick_events: Vec<SimEvent> = self
            .sim
            .events
            .events
            .iter()
            .filter(|e| e.tick == tick)
            .cloned()
            .collect();
        let decisions = &self.sim.last_tick_decisions;
        for sub in self.subscribers.values() {
            if !sub.subscribed {
                continue;
            }
            let events = if sub.want_events {
                serde_json::to_vec(&tick_events).unwrap_or_default()
            } else {
                Vec::new()
            };
            let decisions = if sub.want_decisions {
                serde_json::to_vec(decisions).unwrap_or_default()
            } else {
                Vec::new()
            };
            let metrics = self
                .sim
                .last_tick_timing
                .as_ref()
                .and_then(|t| serde_json::to_vec(t).ok())
                .unwrap_or_default();
            let _ = sub.tx.send(ServerMessage::Tick {
                tick,
                state_hash,
                events,
                decisions,
                metrics,
            });
        }
    }

    fn apply_control(&mut self, verb: ControlVerb) -> ServerMessage {
        if !self.allow_control {
            return ServerMessage::Error {
                code: ErrorCode::ControlDisabled,
                message: "control disabled (start with --allow-control)".into(),
            };
        }
        match verb {
            ControlVerb::Pause => {
                self.paused = true;
                ServerMessage::ReportReady {
                    markdown_or_path: "paused".into(),
                }
            }
            ControlVerb::Play => {
                self.paused = false;
                ServerMessage::ReportReady {
                    markdown_or_path: "playing".into(),
                }
            }
            ControlVerb::Step(n) => {
                let n = n.max(1);
                for _ in 0..n {
                    if !self.tick_once() {
                        break;
                    }
                }
                ServerMessage::ReportReady {
                    markdown_or_path: format!("stepped {n}; tick {}", self.sim.tick),
                }
            }
            ControlVerb::Save => match write_run_checkpoint(&self.sim, self.checkpoint_dir()) {
                Ok(p) => ServerMessage::ReportReady {
                    markdown_or_path: format!("saved {}", p.display()),
                },
                Err(e) => ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: e.to_string(),
                },
            },
            ControlVerb::Report => match write_report(&self.sim, self.checkpoint_dir()) {
                Ok((md, _)) => ServerMessage::ReportReady {
                    markdown_or_path: format!("report={}", md.display()),
                },
                Err(e) => ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: e.to_string(),
                },
            },
            ControlVerb::Summarize => match summary_markdown(&self.sim) {
                Ok(s) => ServerMessage::ReportReady {
                    markdown_or_path: s,
                },
                Err(e) => ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: e.to_string(),
                },
            },
            ControlVerb::Scrub(want) => self.apply_scrub(want),
            ControlVerb::Give { id, item, qty } => self.apply_give(id, &item, qty),
            ControlVerb::CkptNext => self.apply_ckpt_step(true),
            ControlVerb::CkptPrev => self.apply_ckpt_step(false),
            ControlVerb::Events(tick) => self.apply_events(tick),
            ControlVerb::Set {
                id,
                field,
                toward,
                value,
            } => self.apply_set(id, &field, toward, value),
        }
    }

    fn broadcast_snapshot(&self) {
        if let Ok(snap) = self.snapshot() {
            for sub in self.subscribers.values() {
                if sub.subscribed {
                    let _ = sub.tx.send(snap.clone());
                }
            }
        }
    }

    fn apply_set(
        &mut self,
        id: u64,
        field: &str,
        toward: Option<u64>,
        value: u32,
    ) -> ServerMessage {
        let aid = AgentId(id);
        let result = if field == "respect" {
            let Some(t) = toward else {
                return ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: "set respect requires toward agent id".into(),
                };
            };
            self.sim.set_respect(aid, AgentId(t), value)
        } else {
            self.sim.set_display_field(aid, field, value)
        };
        match result {
            Ok(milli) => {
                self.broadcast_snapshot();
                ServerMessage::ReportReady {
                    markdown_or_path: format!("set agent {id} {field}={value} ({milli} milli)"),
                }
            }
            Err(e) => ServerMessage::Error {
                code: ErrorCode::Internal,
                message: e.to_string(),
            },
        }
    }

    fn apply_give(&mut self, id: u64, item: &str, qty: u32) -> ServerMessage {
        let Some(item_id) = parse_item(item, &self.sim.config.world.species) else {
            return ServerMessage::Error {
                code: ErrorCode::Internal,
                message: format!("unknown item {item}"),
            };
        };
        match self.sim.give_item(AgentId(id), item_id, qty) {
            Ok(n) => {
                self.broadcast_snapshot();
                ServerMessage::ReportReady {
                    markdown_or_path: format!("gave {n} {item} to agent {id}"),
                }
            }
            Err(e) => ServerMessage::Error {
                code: ErrorCode::Internal,
                message: e.to_string(),
            },
        }
    }

    fn apply_ckpt_step(&mut self, next: bool) -> ServerMessage {
        let dir = self.checkpoint_dir();
        let ckpts = match list_checkpoints(&dir) {
            Ok(c) => c,
            Err(e) => {
                return ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: e.to_string(),
                };
            }
        };
        let cur = self.sim.tick;
        let pick = if next {
            ckpts.iter().find(|(t, _)| *t > cur)
        } else {
            ckpts.iter().rev().find(|(t, _)| *t < cur)
        };
        let Some((_, path)) = pick else {
            return ServerMessage::Error {
                code: ErrorCode::Internal,
                message: if next {
                    "already at last checkpoint".into()
                } else {
                    "already at first checkpoint".into()
                },
            };
        };
        match Simulation::load_checkpoint(path) {
            Ok(sim) => {
                self.sim = sim;
                self.last_event = self.sim.events.events.len();
                self.paused = true;
                self.broadcast_snapshot();
                ServerMessage::ReportReady {
                    markdown_or_path: format!("loaded tick {}", self.sim.tick),
                }
            }
            Err(e) => ServerMessage::Error {
                code: ErrorCode::Internal,
                message: e.to_string(),
            },
        }
    }

    fn apply_events(&self, want: u64) -> ServerMessage {
        let dir = self.checkpoint_dir();
        let Some(path) = find_events_jsonl(&dir) else {
            return ServerMessage::Error {
                code: ErrorCode::Internal,
                message: format!("no *_events.jsonl in {}", dir.display()),
            };
        };
        let ticks = match list_jsonl_ticks(&path) {
            Ok(t) => t,
            Err(e) => {
                return ServerMessage::Error {
                    code: ErrorCode::Internal,
                    message: e.to_string(),
                };
            }
        };
        let Some(t) = jsonl_tick_at_or_before(&ticks, want) else {
            return ServerMessage::Error {
                code: ErrorCode::Internal,
                message: format!("no event lines at or before tick {want}"),
            };
        };
        match jsonl_lines_for_tick(&path, t) {
            Ok(lines) if !lines.is_empty() => ServerMessage::ReportReady {
                markdown_or_path: lines.join("\n"),
            },
            Ok(_) => ServerMessage::Error {
                code: ErrorCode::Internal,
                message: format!("no event lines at tick {t}"),
            },
            Err(e) => ServerMessage::Error {
                code: ErrorCode::Internal,
                message: e.to_string(),
            },
        }
    }

    fn apply_scrub(&mut self, want: u64) -> ServerMessage {
        let before = self.sim.tick;
        if before == want {
            return ServerMessage::ReportReady {
                markdown_or_path: format!("scrubbed tick {want}"),
            };
        }
        if before < want {
            while self.sim.tick < want {
                if !self.tick_once() {
                    break;
                }
            }
        } else {
            let dir = self.checkpoint_dir();
            let path = match ckpt_at_or_before(&dir, want) {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return ServerMessage::Error {
                        code: ErrorCode::Internal,
                        message: format!(
                            "no checkpoint at or before tick {want} in {}",
                            dir.display()
                        ),
                    };
                }
                Err(e) => {
                    return ServerMessage::Error {
                        code: ErrorCode::Internal,
                        message: e.to_string(),
                    };
                }
            };
            match Simulation::load_checkpoint(&path) {
                Ok(sim) => {
                    self.sim = sim;
                    self.last_event = self.sim.events.events.len();
                    while self.sim.tick < want {
                        if !self.tick_once() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    return ServerMessage::Error {
                        code: ErrorCode::Internal,
                        message: e.to_string(),
                    };
                }
            }
        }
        self.paused = true;
        self.broadcast_snapshot();
        ServerMessage::ReportReady {
            markdown_or_path: format!("scrubbed tick {}", self.sim.tick),
        }
    }

    fn checkpoint_dir(&self) -> PathBuf {
        self.out_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.sim.config.checkpoint.directory))
    }
}

pub struct ServeOpts {
    pub sim: Simulation,
    pub ticks: u64,
    pub listen: Vec<String>,
    pub allow_control: bool,
    pub start_paused: bool,
    pub token: Option<String>,
    pub quiet: bool,
    pub out_dir: Option<PathBuf>,
    pub checkpoint_every: Option<u64>,
    pub write_timing: bool,
    pub lockstep: bool,
}

pub fn serve(mut opts: ServeOpts) -> Result<(), Box<dyn std::error::Error>> {
    if opts.checkpoint_every.is_some() && opts.out_dir.is_none() {
        opts.out_dir = Some(PathBuf::from(&opts.sim.config.checkpoint.directory));
    }

    let mut jsonl_path = None;
    let mut decisions_path = None;
    let mut timing_path = None;
    let last_event = opts.sim.events.events.len();
    if let Some(dir) = &opts.out_dir {
        std::fs::create_dir_all(dir)?;
        let id = experiment_id(&opts.sim.config_hash()?);
        jsonl_path = Some(dir.join(format!("{id}_events.jsonl")));
        decisions_path = Some(dir.join(format!("{id}_decisions.jsonl")));
        if opts.write_timing {
            timing_path = Some(dir.join(format!("{id}_timing.jsonl")));
        }
        if let Some(path) = &jsonl_path {
            append_events_jsonl(path, &opts.sim.events.events)?;
        }
        if opts.sim.tick > 0 {
            write_run_checkpoint(&opts.sim, dir)?;
        }
    }
    let interval = opts
        .checkpoint_every
        .unwrap_or(opts.sim.config.checkpoint.auto_interval_ticks);

    let hub = Arc::new(Mutex::new(Hub {
        sim: opts.sim,
        paused: opts.start_paused,
        allow_control: opts.allow_control,
        token: opts.token,
        jsonl_path,
        decisions_path,
        timing_path,
        last_event,
        out_dir: opts.out_dir.clone(),
        interval,
        subscribers: HashMap::new(),
    }));

    let running = Arc::new(AtomicBool::new(true));
    let next_id = Arc::new(AtomicU64::new(1));
    let mut listener_threads = Vec::new();
    for url in &opts.listen {
        let listener = Listener::bind(url)?;
        let bound = listener.local_url()?;
        eprintln!("listen={bound}");
        let hub = Arc::clone(&hub);
        let running = Arc::clone(&running);
        let next_id = Arc::clone(&next_id);
        listener_threads.push(thread::spawn(move || {
            accept_loop(listener, hub, running, next_id);
        }));
    }
    // Give dummy / GUI clients a moment to Hello before ticks start. Does not
    // enter sim-core and cannot change `state_hash`.
    if !opts.listen.is_empty() {
        thread::sleep(Duration::from_millis(150));
    }

    let mut remaining = opts.ticks;
    while remaining > 0 {
        let did = {
            let mut hub = hub.lock().unwrap();
            if hub.paused {
                false
            } else {
                let ok = hub.tick_once();
                if ok && !opts.quiet {
                    println!("tick={} hash={}", hub.sim.tick, hub.sim.state_hash());
                }
                ok
            }
        };
        if did {
            remaining -= 1;
            if opts.lockstep {
                wait_lockstep_acks(&hub);
            }
        } else {
            thread::sleep(Duration::from_millis(20));
        }
    }

    {
        let hub = hub.lock().unwrap();
        if let Some(dir) = &hub.out_dir {
            let _ = write_run_checkpoint(&hub.sim, dir);
        }
        println!("final_tick={}", hub.sim.tick);
        println!("final_hash={}", hub.sim.state_hash());
        if !opts.quiet {
            for agent in hub.sim.agents.values() {
                println!(
                    "agent {} pos=({},{}) h={} land={}",
                    agent.id.0,
                    agent.x,
                    agent.y,
                    hub.sim.world.height_at(agent.x, agent.y),
                    hub.sim.world.is_land(agent.x, agent.y)
                );
            }
        }
    }

    running.store(false, Ordering::SeqCst);
    for t in listener_threads {
        let _ = t.join();
    }
    Ok(())
}

fn accept_loop(
    listener: Listener,
    hub: Arc<Mutex<Hub>>,
    running: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
) {
    while running.load(Ordering::Relaxed) {
        match listener.accept_nonblocking() {
            Ok(Some(conn)) => {
                let id = next_id.fetch_add(1, Ordering::Relaxed);
                if let Some(addr) = conn.peer_addr() {
                    eprintln!("net: client {id} connected from {addr}");
                } else {
                    eprintln!("net: client {id} connected");
                }
                let hub = Arc::clone(&hub);
                thread::spawn(move || {
                    if let Err(e) = handle_client(id, conn, hub) {
                        if !matches!(e, TransportError::Closed | TransportError::Timeout) {
                            eprintln!("net: client {id} error: {e}");
                        }
                    }
                    eprintln!("net: client {id} disconnected");
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(30)),
            Err(e) => {
                eprintln!("net: accept error: {e}");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_client(
    id: u64,
    mut conn: Connection,
    hub: Arc<Mutex<Hub>>,
) -> Result<(), TransportError> {
    let (out_tx, out_rx) = mpsc::channel::<ServerMessage>();
    conn.set_read_timeout(Some(Duration::from_secs(10)))?;
    let first: ClientMessage = conn.recv_msg()?;
    let ClientMessage::Hello {
        protocol_version,
        token,
    } = first
    else {
        conn.send_msg(&ServerMessage::Error {
            code: ErrorCode::Protocol,
            message: "first message must be Hello".into(),
        })?;
        return Ok(());
    };
    let (welcome, snapshot) = {
        let hub = hub.lock().unwrap();
        if protocol_version != PROTOCOL_VERSION {
            drop(hub);
            conn.send_msg(&ServerMessage::Error {
                code: ErrorCode::Protocol,
                message: format!(
                    "unsupported protocol_version {protocol_version} (server {PROTOCOL_VERSION})"
                ),
            })?;
            return Ok(());
        }
        if let Some(need) = &hub.token {
            if token.as_deref() != Some(need.as_str()) {
                drop(hub);
                conn.send_msg(&ServerMessage::Error {
                    code: ErrorCode::Unauthorized,
                    message: "invalid token".into(),
                })?;
                return Ok(());
            }
        }
        (
            hub.welcome(),
            hub.snapshot().map_err(TransportError::Handshake)?,
        )
    };
    conn.send_msg(&welcome)?;
    conn.send_msg(&snapshot)?;
    {
        let mut hub = hub.lock().unwrap();
        hub.subscribers.insert(
            id,
            Subscriber {
                tx: out_tx,
                want_events: false,
                want_decisions: false,
                subscribed: false,
                acked_tick: None,
            },
        );
    }

    conn.set_read_timeout(Some(Duration::from_millis(40)))?;
    loop {
        flush_out(&mut conn, &out_rx)?;
        match conn.recv_msg::<ClientMessage>() {
            Ok(ClientMessage::Subscribe {
                want_events,
                want_decisions,
            }) => {
                let paused = {
                    let mut hub = hub.lock().unwrap();
                    if let Some(sub) = hub.subscribers.get_mut(&id) {
                        sub.want_events = want_events;
                        sub.want_decisions = want_decisions;
                        sub.subscribed = true;
                    }
                    hub.paused
                };
                conn.send_msg(&ServerMessage::ReportReady {
                    markdown_or_path: if paused {
                        "paused".into()
                    } else {
                        "playing".into()
                    },
                })?;
            }
            Ok(ClientMessage::RequestSnapshot) => {
                let snap = {
                    let hub = hub.lock().unwrap();
                    hub.snapshot()
                };
                match snap {
                    Ok(msg) => conn.send_msg(&msg)?,
                    Err(e) => conn.send_msg(&ServerMessage::Error {
                        code: ErrorCode::Internal,
                        message: e,
                    })?,
                }
            }
            Ok(ClientMessage::Control(verb)) => {
                let reply = {
                    let mut hub = hub.lock().unwrap();
                    hub.apply_control(verb)
                };
                conn.send_msg(&reply)?;
            }
            Ok(ClientMessage::InjectIncentive { schedule_toml }) => {
                let reply = {
                    let mut hub = hub.lock().unwrap();
                    if !hub.allow_control {
                        ServerMessage::Error {
                            code: ErrorCode::ControlDisabled,
                            message: "control disabled (start with --allow-control)".into(),
                        }
                    } else {
                        match hub.sim.inject_schedule_toml(&schedule_toml) {
                            Ok(()) => ServerMessage::ReportReady {
                                markdown_or_path: format!(
                                    "injected {} incentive(s) at tick {}",
                                    hub.sim.incentives.incentives.len(),
                                    hub.sim.tick
                                ),
                            },
                            Err(e) => ServerMessage::Error {
                                code: ErrorCode::Internal,
                                message: e.to_string(),
                            },
                        }
                    }
                };
                conn.send_msg(&reply)?;
            }
            Ok(ClientMessage::AckTick(tick)) => {
                let mut hub = hub.lock().unwrap();
                if let Some(sub) = hub.subscribers.get_mut(&id) {
                    sub.acked_tick = Some(tick);
                }
            }
            Ok(ClientMessage::Hello { .. }) => {
                conn.send_msg(&ServerMessage::Error {
                    code: ErrorCode::Protocol,
                    message: "duplicate Hello".into(),
                })?;
            }
            Err(TransportError::Timeout) => continue,
            Err(e) => {
                let mut hub = hub.lock().unwrap();
                hub.subscribers.remove(&id);
                return Err(e);
            }
        }
    }
}

fn wait_lockstep_acks(hub: &Arc<Mutex<Hub>>) {
    let tick = hub.lock().unwrap().sim.tick;
    loop {
        {
            let h = hub.lock().unwrap();
            let subs: Vec<_> = h.subscribers.values().filter(|s| s.subscribed).collect();
            if subs.is_empty() || subs.iter().all(|s| s.acked_tick == Some(tick)) {
                return;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn flush_out(conn: &mut Connection, rx: &Receiver<ServerMessage>) -> Result<(), TransportError> {
    while let Ok(msg) = rx.try_recv() {
        conn.send_msg(&msg)?;
    }
    Ok(())
}

pub fn merge_listen(net: &NetworkParams, cli: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if !net.tcp_listen.trim().is_empty() {
        out.push(normalize_listen(&net.tcp_listen, "tcp"));
    }
    if !net.ws_listen.trim().is_empty() {
        out.push(normalize_listen(&net.ws_listen, "ws"));
    }
    for u in cli {
        out.push(u.clone());
    }
    out
}

fn normalize_listen(raw: &str, scheme: &str) -> String {
    let raw = raw.trim();
    if raw.contains("://") {
        raw.to_string()
    } else {
        format!("{scheme}://{raw}")
    }
}
