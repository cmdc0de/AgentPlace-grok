//! Dear ImGui panels. Viewer-only; no types leak into sim-core.

use crate::commands::{
    CkptScrubber, WindowFlags, help_text, parse_command, remote_control, run_command,
};
use crate::net::NetLink;
use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;
use serde::{Deserialize, Serialize};
use shared::protocol::ClientMessage;
use sim_bevy::{SimState, step_once};
use sim_core::event_log::SimEventKind;
use sim_core::markers;
use sim_core::{AgentId, ItemId};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const DECISION_RING: usize = 16;
const EVENT_CAP: usize = 200;
const LOG_CAP: usize = 80;
const HISTORY_CAP: usize = 50;

#[derive(Resource)]
pub struct UiState {
    pub windows: WindowFlags,
    pub fog: bool,
    pub cmd: String,
    pub history: Vec<String>,
    pub history_cursor: Option<usize>,
    pub scrollback: Vec<String>,
    pub request_console_focus: bool,
    pub want_keyboard: bool,
    pub last_tick_seen: u64,
    pub decision_ring: VecDeque<(u64, Vec<sim_core::DecisionRecord>)>,
    pub event_filter_agent: String,
    pub event_filter_kind: String,
    pub timing_ring: VecDeque<u64>,
}

impl Default for UiState {
    fn default() -> Self {
        Self::load_or_default()
    }
}

#[derive(Serialize, Deserialize)]
struct UiPersist {
    windows: WindowFlags,
    fog: bool,
}

pub fn ui_layout_dir() -> PathBuf {
    PathBuf::from("checkpoints")
}

pub fn imgui_ini_path() -> PathBuf {
    ui_layout_dir().join("viewer.ini")
}

fn ui_persist_path() -> PathBuf {
    ui_layout_dir().join("viewer_ui.json")
}

impl UiState {
    pub fn load_or_default() -> Self {
        let mut s = Self {
            windows: WindowFlags::default(),
            fog: false,
            cmd: String::new(),
            history: Vec::new(),
            history_cursor: None,
            scrollback: vec!["type /help".into()],
            request_console_focus: false,
            want_keyboard: false,
            last_tick_seen: 0,
            decision_ring: VecDeque::new(),
            event_filter_agent: String::new(),
            event_filter_kind: String::new(),
            timing_ring: VecDeque::new(),
        };
        if let Ok(text) = fs::read_to_string(ui_persist_path()) {
            if let Ok(p) = serde_json::from_str::<UiPersist>(&text) {
                s.windows = p.windows;
                s.fog = p.fog;
            }
        }
        s
    }

    pub fn save_layout(&self) {
        let _ = fs::create_dir_all(ui_layout_dir());
        let persist = UiPersist {
            windows: self.windows.clone(),
            fog: self.fog,
        };
        if let Ok(text) = serde_json::to_string_pretty(&persist) {
            let _ = fs::write(ui_persist_path(), text);
        }
    }
}

pub fn persist_ui_on_exit(ui: Res<UiState>, mut exits: MessageReader<AppExit>) {
    if exits.read().next().is_some() {
        ui.save_layout();
    }
}

pub fn imgui_ui(
    mut context: NonSendMut<ImguiContext>,
    mut state: ResMut<SimState>,
    mut ui: ResMut<UiState>,
    mut scrub: ResMut<CkptScrubber>,
    net: Option<Res<NetLink>>,
) {
    record_decisions(&mut state, &mut ui);
    let imgui_ui = context.ui();
    ui.want_keyboard = imgui_ui.io().want_capture_keyboard;
    let net = net.as_deref();

    if ui.windows.status {
        draw_status(imgui_ui, &mut state, &mut ui, net, &mut scrub);
    }
    if ui.windows.help {
        draw_help(imgui_ui, &mut ui.windows.help);
    }
    if ui.windows.legend {
        draw_legend(imgui_ui, &mut ui.windows.legend);
    }
    if ui.windows.agents {
        draw_agents(imgui_ui, &mut state, &mut ui.windows.agents);
    }
    if ui.windows.inspector {
        draw_inspector(imgui_ui, &state, &mut ui.windows.inspector);
    }
    if ui.windows.board {
        draw_board(imgui_ui, &state, &mut ui.windows.board);
    }
    if ui.windows.log {
        draw_logs(imgui_ui, &state, &mut ui, &mut scrub);
    }
    if ui.windows.world {
        draw_world(imgui_ui, &state, &mut ui.windows.world);
    }
    if ui.windows.console {
        draw_console(imgui_ui, &mut state, &mut ui, net, &mut scrub);
    }
}

fn record_decisions(state: &mut SimState, ui: &mut UiState) {
    let tick = state.sim.tick;
    if tick != ui.last_tick_seen {
        ui.last_tick_seen = tick;
        if !state.sim.last_tick_decisions.is_empty() {
            ui.decision_ring
                .push_back((tick, state.sim.last_tick_decisions.clone()));
            while ui.decision_ring.len() > DECISION_RING {
                ui.decision_ring.pop_front();
            }
        }
        if let Some(t) = &state.sim.last_tick_timing {
            ui.timing_ring.push_back(t.wall_ns);
            while ui.timing_ring.len() > 64 {
                ui.timing_ring.pop_front();
            }
        }
    }
}

fn send_control(net: Option<&NetLink>, msg: ClientMessage) {
    if let Some(net) = net {
        let _ = net.tx.send(msg);
    }
}

fn draw_status(
    ui: &Ui,
    state: &mut SimState,
    us: &mut UiState,
    net: Option<&NetLink>,
    scrub: &mut CkptScrubber,
) {
    ui.window("Status")
        .opened(&mut us.windows.status)
        .size([420.0, 200.0], Condition::FirstUseEver)
        .position([12.0, 12.0], Condition::FirstUseEver)
        .build(|| {
            let hash = state.sim.state_hash().to_string();
            let (tick, short, live_ahead) =
                crate::net::status_tick_hash(net, state.sim.tick, &hash);
            let follow = match state.follow {
                Some(id) => format!("agent {}", id.0),
                None => "free camera".into(),
            };
            let tick_line = match live_ahead {
                Some(live) => format!("tick {tick} (live {live})"),
                None => format!("tick {tick}"),
            };
            ui.text(format!(
                "{tick_line}  {}  follow {follow}  hash {short}",
                if state.paused { "paused" } else { "running" }
            ));
            for line in sim_core::combat_fx::combat_hud_lines(
                &state.sim.events.events,
                state.sim.tick,
            ) {
                ui.text(line);
            }
            ui.text(format!(
                "open proposals {}  water {} veg {} animals {} fish {}",
                state.sim.board.open().count(),
                state.sim.world.water_count(),
                state.sim.world.vegetation_count(),
                state.sim.world.animal_total(),
                state.sim.world.fish_total()
            ));
            if let Some(t) = &state.sim.last_tick_timing {
                let last_ms = t.wall_ns as f64 / 1_000_000.0;
                let (mean_ms, max_ms) = if us.timing_ring.is_empty() {
                    (last_ms, last_ms)
                } else {
                    let n = us.timing_ring.len() as f64;
                    let sum: u64 = us.timing_ring.iter().copied().sum();
                    let max = us.timing_ring.iter().copied().max().unwrap_or(t.wall_ns);
                    (sum as f64 / n / 1_000_000.0, max as f64 / 1_000_000.0)
                };
                ui.text(format!(
                    "tick {last_ms:.2} ms  mean {mean_ms:.2}  max {max_ms:.2}"
                ));
                ui.text(format!(
                    "pipeline {}/{}",
                    t.agents.len(),
                    state.sim.agents.len()
                ));
            }
            if ui.button("Pause") {
                state.paused = true;
                if state.remote {
                    send_control(
                        net,
                        ClientMessage::Control(shared::protocol::ControlVerb::Pause),
                    );
                }
            }
            ui.same_line();
            if ui.button("Play") {
                state.paused = false;
                if state.remote {
                    send_control(
                        net,
                        ClientMessage::Control(shared::protocol::ControlVerb::Play),
                    );
                }
            }
            ui.same_line();
            if ui.button("Step") {
                if state.remote {
                    send_control(
                        net,
                        ClientMessage::Control(shared::protocol::ControlVerb::Step(1)),
                    );
                } else {
                    step_once(state);
                }
            }
            ui.same_line();
            if ui.checkbox("fog (POV)", &mut us.fog) {}
            ui.same_line();
            if ui.button("Unfollow") {
                state.follow = None;
            }
            if !state.remote && scrub.dir.is_some() {
                scrub.refresh();
                if !scrub.ticks.is_empty() {
                    let lo = scrub.ticks.first().map(|(t, _)| *t as i32).unwrap_or(0);
                    let hi = scrub.ticks.last().map(|(t, _)| *t as i32).unwrap_or(lo);
                    let mut want = scrub.loaded_tick.unwrap_or(hi as u64) as i32;
                    ui.text(format!(
                        "ckpt {}  {} files  [ ] prev/next",
                        scrub
                            .loaded_tick
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "-".into()),
                        scrub.ticks.len()
                    ));
                    if lo < hi && ui.slider("ckpt tick", lo, hi, &mut want) {
                        match scrub.apply(state, want as u64) {
                            Ok(t) => us.scrollback.push(format!("loaded tick {t}")),
                            Err(e) => us.scrollback.push(format!("scrub error: {e}")),
                        }
                    }
                    if ui.button("[ prev") {
                        match scrub.prev(state) {
                            Ok(t) => us.scrollback.push(format!("loaded tick {t}")),
                            Err(e) => us.scrollback.push(format!("ckpt error: {e}")),
                        }
                    }
                    ui.same_line();
                    if ui.button("next ]") {
                        match scrub.next(state) {
                            Ok(t) => us.scrollback.push(format!("loaded tick {t}")),
                            Err(e) => us.scrollback.push(format!("ckpt error: {e}")),
                        }
                    }
                }
            }
        });
}

fn draw_help(ui: &Ui, open: &mut bool) {
    ui.window("Help")
        .opened(open)
        .size([380.0, 280.0], Condition::FirstUseEver)
        .position([440.0, 12.0], Condition::FirstUseEver)
        .build(|| {
            ui.text_wrapped(help_text());
        });
}

fn draw_legend(ui: &Ui, open: &mut bool) {
    ui.window("Legend")
        .opened(open)
        .size([260.0, 340.0], Condition::FirstUseEver)
        .position([12.0, 170.0], Condition::FirstUseEver)
        .build(|| {
            for (name, shape, rgb) in markers::legend_entries() {
                let [r, g, b] = markers::rgb_f32(rgb);
                let _ = ui.color_button(name, [r, g, b, 1.0]);
                ui.same_line();
                ui.text(format!("{}  {name}", markers::shape_name(shape)));
            }
        });
}

fn draw_agents(ui: &Ui, state: &mut SimState, open: &mut bool) {
    ui.window("Agents")
        .opened(open)
        .size([280.0, 360.0], Condition::FirstUseEver)
        .position([280.0, 170.0], Condition::FirstUseEver)
        .build(|| {
            ui.text("click to follow");
            let ids: Vec<AgentId> = state.sim.agent_ids();
            for id in ids {
                let Some(a) = state.sim.agents.get(&id) else {
                    continue;
                };
                let branch = state
                    .sim
                    .last_tick_decisions
                    .iter()
                    .rev()
                    .find(|d| d.agent == id.0)
                    .map(|d| d.policy_branch.as_str())
                    .unwrap_or("-");
                let label = format!("#{} h:{:.0} {branch}", id.0, a.needs.hunger as f32 / 100.0);
                let selected = state.follow == Some(id);
                if ui.selectable_config(&label).selected(selected).build() {
                    state.follow = Some(id);
                }
            }
        });
}

fn draw_inspector(ui: &Ui, state: &SimState, open: &mut bool) {
    ui.window("Inspector")
        .opened(open)
        .size([360.0, 420.0], Condition::FirstUseEver)
        .position([840.0, 12.0], Condition::FirstUseEver)
        .build(|| {
            let Some(id) = state.follow else {
                ui.text("follow an agent (F, 0-9, or Agents list)");
                return;
            };
            let Some(a) = state.sim.agents.get(&id) else {
                ui.text("agent missing");
                return;
            };
            ui.text(format!("agent {}  pos=({}, {})", id.0, a.x, a.y));
            if let Some(t) = &state.sim.last_tick_timing {
                if let Some(at) = t.agents.iter().find(|x| x.agent == id.0) {
                    ui.text(format!(
                        "step µs  perc {}  retr {}  sel {}  exec {}  rem {}",
                        at.perceive_ns / 1000,
                        at.retrieve_ns / 1000,
                        at.select_ns / 1000,
                        at.execute_ns / 1000,
                        at.remember_ns / 1000
                    ));
                }
            }
            need_bar(ui, "hunger", a.needs.hunger);
            need_bar(ui, "thirst", a.needs.thirst);
            need_bar(ui, "energy", a.needs.energy);
            ui.text(format!(
                "illness {}  influence {:.2}",
                a.illness_ticks,
                a.influence_factor as f32 / 100.0
            ));
            if a.age_ticks != 0 || state.sim.aging_enabled {
                let child = if state.sim.is_child(a) { "  child" } else { "" };
                ui.text(format!("age {}{child}", a.age_ticks));
            }
            if !a.sheet.is_unused() {
                ui.separator();
                ui.text("sheet");
                for (name, score) in a.sheet.rows() {
                    ui.text(format!(
                        "  {name} {score} ({:+})",
                        sim_core::AbilitySheet::modifier(score)
                    ));
                }
            }
            if !a.kinship.is_empty() {
                ui.separator();
                ui.text("family");
                for line in a.kinship.lines() {
                    ui.text(format!("  {line}"));
                }
                if let Some(hid) = a.kinship.household {
                    if let Some(&(hx, hy)) = state.sim.household_home.get(&hid) {
                        ui.text(format!("  home ({hx},{hy})"));
                    }
                }
            }
            if a.culture != 0 {
                ui.text(format!("culture {}", a.culture));
            }
            ui.separator();
            ui.text(format!(
                "abilities g{} h{} f{} farm{} c{}",
                a.abilities.gather,
                a.abilities.hunt,
                a.abilities.fish,
                a.abilities.farm,
                a.abilities.craft
            ));
            if !a.personality.allergy_tags.is_empty() {
                ui.text(format!(
                    "allergies: {}",
                    a.personality.allergy_tags.join(", ")
                ));
            }
            ui.text(format!(
                "goals: {}",
                if a.goals.is_empty() {
                    "(none)".into()
                } else {
                    a.goals
                        .iter()
                        .map(|g| g.text.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                }
            ));
            ui.separator();
            ui.text("incentives (researcher)");
            let mut any_inc = false;
            for inc in &state.sim.incentives.incentives {
                if !state.sim.incentive_active.contains(&inc.id) {
                    continue;
                }
                any_inc = true;
                let tag = if inc.is_hidden() { "hidden" } else { "public" };
                ui.text(format!("  {} ({tag})", inc.id));
            }
            if !any_inc {
                ui.text("  (none)");
            }
            ui.separator();
            ui.text("inventory");
            if a.inventory.is_empty() {
                ui.text("  (empty)");
            } else {
                for (item, n) in &a.inventory {
                    ui.text(format!(
                        "  {} x{n}",
                        item_label(*item, &state.sim.config.world.species)
                    ));
                }
            }
            ui.separator();
            let params = state.sim.storage;
            if a.has_pack(&params) {
                let (slots, w) = a.worn_pack_caps(&params);
                ui.text(format!(
                    "pack slots {}/{}  weight {:.1}/{}",
                    a.pack_count(),
                    slots,
                    a.pack_weight_milli() as f32 / 100.0,
                    w / 100
                ));
                if a.pack.is_empty() {
                    ui.text("  (empty)");
                } else {
                    for (item, n) in &a.pack {
                        ui.text(format!(
                            "  {} x{n}",
                            item_label(*item, &state.sim.config.world.species)
                        ));
                    }
                }
            } else {
                ui.text("pack: (none)");
            }
            ui.separator();
            if let Some(c) = state.sim.world.stockpile_at(a.x, a.y) {
                let params = state.sim.storage;
                ui.text(format!(
                    "stockpile slots {}/{}  weight {:.1}/{}",
                    c.slot_count(),
                    params.slot_cap,
                    c.weight_milli() as f32 / 100.0,
                    params.weight_cap_milli / 100
                ));
                for (item, n) in &c.items {
                    ui.text(format!(
                        "  {} x{n}",
                        item_label(*item, &state.sim.config.world.species)
                    ));
                }
            } else {
                ui.text("stockpile: (none on this cell)");
            }
            ui.separator();
            ui.text(format!("relationships ({})", a.relationships.len()));
            for (oid, r) in &a.relationships {
                ui.text(format!(
                    "  #{} t:{:.1} a:{:.1} r:{:.1} f:{:.1}",
                    oid.0,
                    r.trust as f32 / 100.0,
                    r.affinity as f32 / 100.0,
                    r.respect as f32 / 100.0,
                    r.fear as f32 / 100.0
                ));
            }
            if let Some(d) = state
                .sim
                .last_tick_decisions
                .iter()
                .rev()
                .find(|d| d.agent == id.0)
            {
                ui.separator();
                ui.text(format!("last: {}  {:?}", d.policy_branch, d.primary));
            }
        });
}

fn need_bar(ui: &Ui, label: &str, milli: u32) {
    let frac = (milli as f32 / 10_000.0).clamp(0.0, 1.0);
    ui.text(format!(
        "{label} {:.1}  [{:.0}%]",
        milli as f32 / 100.0,
        frac * 100.0
    ));
}

fn draw_board(ui: &Ui, state: &SimState, open: &mut bool) {
    ui.window("Board")
        .opened(open)
        .size([360.0, 280.0], Condition::FirstUseEver)
        .position([560.0, 360.0], Condition::FirstUseEver)
        .build(|| {
            ui.text(format!(
                "open {}  accepted {}  rejected {}  expired {}",
                state.sim.board.open().count(),
                state.sim.board.accepted_count,
                state.sim.board.rejected_count,
                state.sim.board.expired_count
            ));
            ui.separator();
            ui.text("proposals");
            if state.sim.voting.accept != sim_core::VoteAccept::Majority {
                ui.text(format!(
                    "accept {}  council {:?}",
                    state.sim.voting.accept.as_str(),
                    state
                        .sim
                        .voting
                        .council
                        .iter()
                        .map(|id| id.0)
                        .collect::<Vec<_>>()
                ));
            }
            let need = state.sim.vote_need();
            let weighted = matches!(
                state.sim.voting.weight,
                sim_core::VoteWeight::Influence | sim_core::VoteWeight::Respect
            );
            if weighted {
                ui.text(format!(
                    "tally {}  need {need}  total {}",
                    state.sim.voting.weight.as_str(),
                    state.sim.living_vote_total()
                ));
            }
            for p in &state.sim.board.proposals {
                let (yes_w, no_w) = state.sim.proposal_yes_no_weight(p);
                if weighted {
                    ui.text(format!(
                        "  #{} {:?} yes={} ({yes_w}) no={} ({no_w}) need={need} {:?}",
                        p.id,
                        p.status,
                        p.supporters.len(),
                        p.opposers.len(),
                        p.rule
                    ));
                } else {
                    ui.text(format!(
                        "  #{} {:?} yes={} no={} {:?}",
                        p.id,
                        p.status,
                        p.supporters.len(),
                        p.opposers.len(),
                        p.rule
                    ));
                }
                ui.text_wrapped(&format!("    {}", p.text));
            }
            ui.separator();
            ui.text("adopted");
            if state.sim.board.adopted.is_empty() {
                ui.text("  (none)");
            } else {
                for r in &state.sim.board.adopted {
                    ui.text(format!("  #{} {:?}", r.proposal_id, r.rule));
                    ui.text_wrapped(&format!("    {}", r.text));
                }
            }
        });
}

fn draw_logs(ui: &Ui, state: &SimState, us: &mut UiState, scrub: &mut CkptScrubber) {
    ui.window("Logs")
        .opened(&mut us.windows.log)
        .size([420.0, 300.0], Condition::FirstUseEver)
        .position([12.0, 520.0], Condition::FirstUseEver)
        .build(|| {
            ui.input_text("filter agent", &mut us.event_filter_agent)
                .build();
            ui.input_text("filter kind", &mut us.event_filter_kind)
                .build();
            if !state.remote && !scrub.event_ticks.is_empty() {
                let lo = *scrub.event_ticks.first().unwrap_or(&0) as i32;
                let hi = *scrub.event_ticks.last().unwrap_or(&0) as i32;
                let mut want = scrub.event_tick.unwrap_or(hi as u64) as i32;
                ui.text(format!(
                    "jsonl tick {}  {} ticks",
                    scrub
                        .event_tick
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "-".into()),
                    scrub.event_ticks.len()
                ));
                if lo < hi && ui.slider("event tick", lo, hi, &mut want) {
                    let _ = scrub.filter_events(want as u64);
                }
            }
            ui.separator();
            ui.text("events");
            let filter_id = us.event_filter_agent.parse::<u64>().ok();
            let kind_f = us.event_filter_kind.to_ascii_lowercase();
            if !state.remote {
                if let (Some(path), Some(tick)) = (&scrub.events_path, scrub.event_tick) {
                    if let Ok(lines) = sim_core::jsonl_lines_for_tick(path, tick) {
                        for line in lines.iter().rev().take(EVENT_CAP) {
                            ui.text_wrapped(line);
                        }
                        ui.separator();
                        ui.text("decisions (recent ticks)");
                        for (tick, recs) in us.decision_ring.iter().rev() {
                            ui.text(format!("tick {tick}"));
                            for d in recs {
                                ui.text(format!("  a{} {}", d.agent, d.policy_branch));
                            }
                        }
                        return;
                    }
                }
            }
            let events = &state.sim.events.events;
            let start = events.len().saturating_sub(EVENT_CAP);
            for e in events.iter().skip(start).rev() {
                if let Some(id) = filter_id {
                    if e.agent.0 != id {
                        continue;
                    }
                }
                let kn = event_kind_name(&e.kind);
                if !kind_f.is_empty() && !kn.to_ascii_lowercase().contains(&kind_f) {
                    continue;
                }
                ui.text(format!("t{} a{} {kn}", e.tick, e.agent.0));
            }
            ui.separator();
            ui.text("decisions (recent ticks)");
            for (tick, recs) in us.decision_ring.iter().rev() {
                ui.text(format!("tick {tick}"));
                for d in recs {
                    ui.text(format!("  a{} {}", d.agent, d.policy_branch));
                }
            }
        });
}

fn draw_world(ui: &Ui, state: &SimState, open: &mut bool) {
    ui.window("World")
        .opened(open)
        .size([300.0, 200.0], Condition::FirstUseEver)
        .position([940.0, 450.0], Condition::FirstUseEver)
        .build(|| {
            let n = state.sim.agents.len().max(1) as f32;
            let veg: u32 = state
                .sim
                .agents
                .values()
                .map(|a| a.consumption.vegetation)
                .sum();
            let animal: u32 = state
                .sim
                .agents
                .values()
                .map(|a| a.consumption.animal)
                .sum();
            let fish: u32 = state.sim.agents.values().map(|a| a.consumption.fish).sum();
            let toxic: u32 = state
                .sim
                .agents
                .values()
                .map(|a| a.consumption.toxic_events)
                .sum();
            let hunger: f32 = state
                .sim
                .agents
                .values()
                .map(|a| a.needs.hunger as f32 / 100.0)
                .sum::<f32>()
                / n;
            let pairs: usize = state
                .sim
                .agents
                .values()
                .map(|a| a.relationships.len())
                .sum();
            ui.text(format!(
                "consumed veg {veg} animal {animal} fish {fish} toxic {toxic}"
            ));
            ui.text(format!("mean hunger {hunger:.1}"));
            ui.text(format!(
                "board open {}  rel pairs {pairs}",
                state.sim.board.open().count()
            ));
            if let Some(t) = &state.sim.last_tick_timing {
                ui.text(format!(
                    "ns wall {} world {} board {} inc {} agents {}",
                    t.wall_ns, t.world_ns, t.board_ns, t.incentive_ns, t.agents_ns
                ));
            }
        });
}

fn draw_console(
    ui: &Ui,
    state: &mut SimState,
    us: &mut UiState,
    net: Option<&NetLink>,
    scrub: &mut CkptScrubber,
) {
    let mut open = us.windows.console;
    ui.window("Console")
        .opened(&mut open)
        .size([520.0, 220.0], Condition::FirstUseEver)
        .position([280.0, 540.0], Condition::FirstUseEver)
        .build(|| {
            ui.child_window("scroll").size([0.0, 140.0]).build(|| {
                for line in &us.scrollback {
                    ui.text_wrapped(line);
                }
            });
            if us.request_console_focus {
                ui.set_keyboard_focus_here();
                us.request_console_focus = false;
            }
            let enter = ui
                .input_text("##cmd", &mut us.cmd)
                .enter_returns_true(true)
                .build();
            ui.same_line();
            let run = ui.button("Run") || enter;
            if run && !us.cmd.trim().is_empty() {
                let line = us.cmd.trim().to_string();
                us.history.push(line.clone());
                if us.history.len() > HISTORY_CAP {
                    us.history.remove(0);
                }
                us.history_cursor = None;
                us.cmd.clear();
                us.scrollback.push(format!("> {line}"));
                match parse_command(&line) {
                    Ok(cmd) => {
                        if state.remote {
                            if let Some(verb) = remote_control(&cmd) {
                                send_control(net, ClientMessage::Control(verb));
                            }
                            if let crate::commands::UiCommand::Inject { path: Some(p) } = &cmd {
                                match std::fs::read_to_string(p) {
                                    Ok(toml) => send_control(
                                        net,
                                        ClientMessage::InjectIncentive {
                                            schedule_toml: toml,
                                        },
                                    ),
                                    Err(e) => us.scrollback.push(format!("inject read error: {e}")),
                                }
                            }
                        }
                        let msgs = run_command(cmd, state, &mut us.fog, &mut us.windows, scrub);
                        us.scrollback.extend(msgs);
                    }
                    Err(e) => us.scrollback.push(e),
                }
                while us.scrollback.len() > LOG_CAP {
                    us.scrollback.remove(0);
                }
            }
        });
    us.windows.console = open;
}

fn event_kind_name(kind: &SimEventKind) -> &'static str {
    match kind {
        SimEventKind::Wait => "wait",
        SimEventKind::Rest => "rest",
        SimEventKind::Move { .. } => "move",
        SimEventKind::Gather { .. } => "gather",
        SimEventKind::Drink => "drink",
        SimEventKind::Eat { .. } => "eat",
        SimEventKind::Hunt { .. } => "hunt",
        SimEventKind::Fish { .. } => "fish",
        SimEventKind::Farm { .. } => "farm",
        SimEventKind::Craft { .. } => "craft",
        SimEventKind::Speak { .. } => "speak",
        SimEventKind::LlmWait => "llm_wait",
        SimEventKind::Propose { .. } => "propose",
        SimEventKind::Support { .. } => "support",
        SimEventKind::Oppose { .. } => "oppose",
        SimEventKind::RuleBlocked { .. } => "rule_blocked",
        SimEventKind::IncentiveApplied { .. } => "incentive_applied",
        SimEventKind::IncentiveEnded { .. } => "incentive_ended",
        SimEventKind::Died { .. } => "died",
        SimEventKind::Transfer { .. } => "transfer",
        SimEventKind::Store { .. } => "store",
        SimEventKind::Retrieve { .. } => "retrieve",
        SimEventKind::Give { .. } => "give",
        SimEventKind::Pack { .. } => "pack",
        SimEventKind::Unpack { .. } => "unpack",
        SimEventKind::Attack { .. } => "attack",
        SimEventKind::Flee => "flee",
        SimEventKind::Incapacitated { .. } => "incapacitated",
        SimEventKind::CombatDeath { .. } => "combat_death",
        SimEventKind::PairBonded { .. } => "pair_bonded",
        SimEventKind::Born { .. } => "born",
        SimEventKind::Invented { .. } => "invented",
    }
}

fn item_label(item: ItemId, species: &sim_core::species::SpeciesTables) -> String {
    match item {
        ItemId::Food(100) => "hare".into(),
        ItemId::Food(101) => "perch".into(),
        ItemId::Food(tag) => species
            .veg(tag)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| format!("food:{tag}")),
        ItemId::Wood => "wood".into(),
        ItemId::Fiber => "fiber".into(),
        ItemId::Stone => "stone".into(),
        ItemId::Basket => "basket".into(),
        ItemId::Spear => "spear".into(),
        ItemId::FishingRod => "fishing_rod".into(),
        ItemId::Backpack => "backpack".into(),
        ItemId::Catalog(n) => format!("catalog:{n}"),
    }
}
