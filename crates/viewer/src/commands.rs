//! Closed slash-command set for the viewer console. No GPU / imgui types.

use bevy::prelude::Resource;
use shared::protocol::ControlVerb;
use sim_bevy::{SimState, step_once};
use sim_core::{
    AgentId, ExperimentConfig, Simulation, ckpt_at_or_before, find_events_jsonl,
    jsonl_tick_at_or_before, list_checkpoints, list_jsonl_ticks, summary_markdown, write_report,
    write_run_checkpoint,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Resource)]
pub struct CkptScrubber {
    pub dir: Option<PathBuf>,
    pub ticks: Vec<(u64, PathBuf)>,
    pub loaded_tick: Option<u64>,
    pub events_path: Option<PathBuf>,
    pub event_ticks: Vec<u64>,
    pub event_tick: Option<u64>,
}

impl CkptScrubber {
    pub fn discover(path: &Path) -> Self {
        let dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        };
        let ticks = list_checkpoints(&dir).unwrap_or_default();
        let loaded_tick = if path.is_file() {
            ticks
                .iter()
                .find(|(_, p)| *p == path)
                .map(|(t, _)| *t)
                .or_else(|| ticks.last().map(|(t, _)| *t))
        } else {
            ticks.last().map(|(t, _)| *t)
        };
        let events_path = find_events_jsonl(&dir);
        let event_ticks = events_path
            .as_ref()
            .and_then(|p| list_jsonl_ticks(p).ok())
            .unwrap_or_default();
        let event_tick = event_ticks.last().copied();
        Self {
            dir: Some(dir),
            ticks,
            loaded_tick,
            events_path,
            event_ticks,
            event_tick,
        }
    }

    pub fn refresh(&mut self) {
        if let Some(dir) = &self.dir {
            self.ticks = list_checkpoints(dir).unwrap_or_default();
            self.events_path = find_events_jsonl(dir);
            self.event_ticks = self
                .events_path
                .as_ref()
                .and_then(|p| list_jsonl_ticks(p).ok())
                .unwrap_or_default();
        }
    }

    pub fn filter_events(&mut self, want: u64) -> Result<u64, String> {
        self.refresh();
        if self.events_path.is_none() {
            return Err("no *_events.jsonl in this directory".into());
        }
        let t = jsonl_tick_at_or_before(&self.event_ticks, want)
            .ok_or_else(|| format!("no event lines at or before tick {want}"))?;
        self.event_tick = Some(t);
        Ok(t)
    }

    pub fn apply(&mut self, state: &mut SimState, want: u64) -> Result<u64, String> {
        self.refresh();
        let dir = self.dir.as_ref().ok_or("no checkpoint directory loaded")?;
        let path = ckpt_at_or_before(dir, want)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("no checkpoint at or before tick {want}"))?;
        let listed_tick = self.ticks.iter().find(|(_, p)| *p == path).map(|(t, _)| *t);
        if let Some(listed) = listed_tick {
            if self.loaded_tick == Some(listed) && state.sim.tick == listed {
                return Ok(listed);
            }
        }
        let sim = Simulation::load_checkpoint(&path).map_err(|e| e.to_string())?;
        let tick = sim.tick;
        state.sim = sim;
        state.paused = true;
        if let Some(id) = state.follow {
            if !state.sim.agents.contains_key(&id) {
                state.follow = None;
            }
        }
        self.loaded_tick = Some(tick);
        Ok(tick)
    }

    pub fn next(&mut self, state: &mut SimState) -> Result<u64, String> {
        let cur = self.loaded_tick.unwrap_or(0);
        let want = self
            .ticks
            .iter()
            .map(|(t, _)| *t)
            .find(|t| *t > cur)
            .ok_or("already at last checkpoint")?;
        self.apply(state, want)
    }

    pub fn prev(&mut self, state: &mut SimState) -> Result<u64, String> {
        let cur = self.loaded_tick.unwrap_or(0);
        let want = self
            .ticks
            .iter()
            .map(|(t, _)| *t)
            .rev()
            .find(|t| *t < cur)
            .ok_or("already at first checkpoint")?;
        self.apply(state, want)
    }

    pub fn initial_path(path: &Path) -> Result<PathBuf, String> {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        if path.is_dir() {
            let ckpts = list_checkpoints(path).map_err(|e| e.to_string())?;
            return ckpts
                .last()
                .map(|(_, p)| p.clone())
                .ok_or_else(|| format!("no .ckpt files in {}", path.display()));
        }
        Err(format!("not a file or directory: {}", path.display()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    Help,
    Report { dir: Option<String> },
    Summarize,
    Save { path: Option<String> },
    Follow { id: Option<u64> },
    Pause,
    Play,
    Step { n: u32 },
    Fog { on: Option<bool> },
    ToggleLegend,
    ToggleInspector,
    ToggleBoard,
    ToggleLog,
    Tick,
    Inject { path: Option<String> },
    Give { id: u64, item: String, qty: u32 },
    Set { id: u64, field: String, value: u32 },
    Events { tick: u64 },
    Scrub { tick: u64 },
    CkptNext,
    CkptPrev,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowFlags {
    pub legend: bool,
    pub help: bool,
    pub inspector: bool,
    pub board: bool,
    pub log: bool,
    pub console: bool,
    pub status: bool,
    pub agents: bool,
    pub world: bool,
}

impl Default for WindowFlags {
    fn default() -> Self {
        Self {
            legend: true,
            help: true,
            inspector: true,
            board: true,
            log: true,
            console: true,
            status: true,
            agents: true,
            world: true,
        }
    }
}

pub fn help_text() -> &'static str {
    "\
keys:
  Space pause/play   . step   F follow toggle   0-9 follow agent
  L legend   H help   I inspector   B board   O fog   / console
  Esc quit (saves window layout)
commands:
  /help
  /report [dir]     write food-economy report (does not change hash)
  /summarize        print world summary
  /save [path]      write a checkpoint
  /follow N|off
  /pause  /play  /step [n]
  /fog on|off
  /legend  /inspector  /board  /log
  /tick
  /inject PATH     load incentive TOML (needs --allow-control when remote)
  /give ID ITEM QTY   in-process only; hash-sensitive (berry_bush, wood, …)
  /set ID FIELD N     hunger|thirst|energy|influence 0–100 (in-process)
  /events TICK     filter log to JSONL tick at or before TICK (display-only)
  /scrub TICK      load ckpt at or before TICK (--load DIR, in-process)
  /ckpt next|prev  adjacent checkpoint in the run directory
  [ ] keys         same as /ckpt prev|next when a ckpt dir is loaded"
}

pub fn parse_command(line: &str) -> Result<UiCommand, String> {
    let line = line.trim();
    let line = line.strip_prefix('/').unwrap_or(line);
    if line.is_empty() {
        return Err("empty command".into());
    }
    let mut parts = line.split_whitespace();
    let verb = parts.next().unwrap_or("").to_ascii_lowercase();
    let arg = parts.next();
    match verb.as_str() {
        "help" | "h" => Ok(UiCommand::Help),
        "report" => Ok(UiCommand::Report {
            dir: arg.map(|s| s.to_string()),
        }),
        "summarize" | "summary" => Ok(UiCommand::Summarize),
        "save" => Ok(UiCommand::Save {
            path: arg.map(|s| s.to_string()),
        }),
        "follow" => match arg {
            None | Some("off") | Some("none") => Ok(UiCommand::Follow { id: None }),
            Some(s) => {
                let id: u64 = s.parse().map_err(|_| format!("bad follow id: {s}"))?;
                Ok(UiCommand::Follow { id: Some(id) })
            }
        },
        "pause" => Ok(UiCommand::Pause),
        "play" | "unpause" => Ok(UiCommand::Play),
        "step" => {
            let n = match arg {
                None => 1,
                Some(s) => s.parse().map_err(|_| format!("bad step count: {s}"))?,
            };
            Ok(UiCommand::Step { n: n.max(1) })
        }
        "fog" => match arg {
            None => Ok(UiCommand::Fog { on: None }),
            Some("on") | Some("true") | Some("1") => Ok(UiCommand::Fog { on: Some(true) }),
            Some("off") | Some("false") | Some("0") => Ok(UiCommand::Fog { on: Some(false) }),
            Some(s) => Err(format!("fog expects on|off, got {s}")),
        },
        "legend" => Ok(UiCommand::ToggleLegend),
        "inspector" => Ok(UiCommand::ToggleInspector),
        "board" => Ok(UiCommand::ToggleBoard),
        "log" => Ok(UiCommand::ToggleLog),
        "tick" => Ok(UiCommand::Tick),
        "inject" => Ok(UiCommand::Inject {
            path: arg.map(|s| s.to_string()),
        }),
        "events" => {
            let t = arg.ok_or("events requires a tick")?;
            let tick: u64 = t.parse().map_err(|_| format!("bad tick: {t}"))?;
            Ok(UiCommand::Events { tick })
        }
        "scrub" => {
            let t = arg.ok_or("scrub requires a tick")?;
            let tick: u64 = t.parse().map_err(|_| format!("bad tick: {t}"))?;
            Ok(UiCommand::Scrub { tick })
        }
        "ckpt" => match arg {
            Some("next") => Ok(UiCommand::CkptNext),
            Some("prev") | Some("previous") => Ok(UiCommand::CkptPrev),
            Some(s) => Err(format!("ckpt expects next|prev, got {s}")),
            None => Err("ckpt requires next|prev".into()),
        },
        "give" => {
            let id_s = arg.ok_or("give requires agent id")?;
            let id: u64 = id_s.parse().map_err(|_| format!("bad agent id: {id_s}"))?;
            let item = parts.next().ok_or("give requires item name")?.to_string();
            let qty = match parts.next() {
                None => 1,
                Some(s) => s.parse().map_err(|_| format!("bad qty: {s}"))?,
            };
            Ok(UiCommand::Give {
                id,
                item,
                qty: qty.max(1),
            })
        }
        "set" => {
            let id_s = arg.ok_or("set requires agent id")?;
            let id: u64 = id_s.parse().map_err(|_| format!("bad agent id: {id_s}"))?;
            let field = parts
                .next()
                .ok_or("set requires field (hunger|thirst|energy|influence)")?
                .to_ascii_lowercase();
            let v = parts.next().ok_or("set requires a value 0–100")?;
            let value: u32 = v.parse().map_err(|_| format!("bad value: {v}"))?;
            Ok(UiCommand::Set { id, field, value })
        }
        other => Err(format!("unknown: /{other}  (try /help)")),
    }
}

pub fn remote_control(cmd: &UiCommand) -> Option<ControlVerb> {
    match cmd {
        UiCommand::Pause => Some(ControlVerb::Pause),
        UiCommand::Play => Some(ControlVerb::Play),
        UiCommand::Step { n } => Some(ControlVerb::Step(*n)),
        UiCommand::Save { .. } => Some(ControlVerb::Save),
        UiCommand::Report { .. } => Some(ControlVerb::Report),
        UiCommand::Summarize => Some(ControlVerb::Summarize),
        _ => None,
    }
}

pub fn run_command(
    cmd: UiCommand,
    state: &mut SimState,
    fog: &mut bool,
    windows: &mut WindowFlags,
    scrub: &mut CkptScrubber,
) -> Vec<String> {
    match cmd {
        UiCommand::Help => vec![help_text().into()],
        UiCommand::Report { dir } => {
            if state.remote {
                return vec!["report sent".into()];
            }
            match write_report_cmd(&state.sim, dir.as_deref()) {
                Ok(p) => vec![format!("report={}", p.display())],
                Err(e) => vec![format!("report error: {e}")],
            }
        }
        UiCommand::Summarize => {
            if state.remote {
                return vec!["summarize sent".into()];
            }
            match summary_markdown(&state.sim) {
                Ok(s) => vec![s],
                Err(e) => vec![format!("summarize error: {e}")],
            }
        }
        UiCommand::Save { path } => {
            if state.remote {
                return vec!["save sent".into()];
            }
            match save_cmd(&state.sim, path.as_deref()) {
                Ok(p) => vec![format!("saved {}", p.display())],
                Err(e) => vec![format!("save error: {e}")],
            }
        }
        UiCommand::Follow { id: None } => {
            state.follow = None;
            vec!["follow off".into()]
        }
        UiCommand::Follow { id: Some(n) } => {
            let id = AgentId(n);
            if state.sim.agents.contains_key(&id) {
                state.follow = Some(id);
                vec![format!("follow agent {n}")]
            } else {
                vec![format!("no agent {n}")]
            }
        }
        UiCommand::Pause => {
            state.paused = true;
            if state.remote {
                vec!["pause sent".into()]
            } else {
                vec!["paused".into()]
            }
        }
        UiCommand::Play => {
            state.paused = false;
            if state.remote {
                vec!["play sent".into()]
            } else {
                vec!["playing".into()]
            }
        }
        UiCommand::Step { n } => {
            if state.remote {
                return vec![format!("step {n} sent")];
            }
            for _ in 0..n {
                step_once(state);
            }
            vec![format!("stepped {n}; tick {}", state.sim.tick)]
        }
        UiCommand::Fog { on: None } => {
            *fog = !*fog;
            vec![format!("fog={}", *fog)]
        }
        UiCommand::Fog { on: Some(v) } => {
            *fog = v;
            vec![format!("fog={v}")]
        }
        UiCommand::ToggleLegend => {
            windows.legend = !windows.legend;
            vec![format!("legend={}", windows.legend)]
        }
        UiCommand::ToggleInspector => {
            windows.inspector = !windows.inspector;
            vec![format!("inspector={}", windows.inspector)]
        }
        UiCommand::ToggleBoard => {
            windows.board = !windows.board;
            vec![format!("board={}", windows.board)]
        }
        UiCommand::ToggleLog => {
            windows.log = !windows.log;
            vec![format!("log={}", windows.log)]
        }
        UiCommand::Tick => {
            let hash = state.sim.state_hash().to_string();
            let short = if hash.len() >= 12 { &hash[..12] } else { &hash };
            vec![format!("tick={} hash={}", state.sim.tick, short)]
        }
        UiCommand::Inject { path } => {
            if state.remote {
                return vec!["inject sent".into()];
            }
            let Some(path) = path else {
                return vec!["inject requires a toml path".into()];
            };
            match std::fs::read_to_string(&path) {
                Ok(text) => match state.sim.inject_schedule_toml(&text) {
                    Ok(()) => vec![format!(
                        "injected {} incentive(s)",
                        state.sim.incentives.incentives.len()
                    )],
                    Err(e) => vec![format!("inject error: {e}")],
                },
                Err(e) => vec![format!("inject read error: {e}")],
            }
        }
        UiCommand::Give { id, item, qty } => {
            if state.remote {
                return vec!["give is in-process only (not on the attach wire)".into()];
            }
            let Some(item_id) = sim_core::parse_item(&item, &state.sim.config.world.species) else {
                return vec![format!("unknown item {item}")];
            };
            match state.sim.give_item(sim_core::AgentId(id), item_id, qty) {
                Ok(n) => vec![format!("gave {n} {item} to agent {id}")],
                Err(e) => vec![format!("give error: {e}")],
            }
        }
        UiCommand::Scrub { tick } => {
            if state.remote {
                return vec!["scrub is in-process only (not on the attach wire)".into()];
            }
            match scrub.apply(state, tick) {
                Ok(t) => vec![format!("loaded tick {t}")],
                Err(e) => vec![format!("scrub error: {e}")],
            }
        }
        UiCommand::CkptNext => {
            if state.remote {
                return vec!["ckpt step is in-process only (not on the attach wire)".into()];
            }
            match scrub.next(state) {
                Ok(t) => vec![format!("loaded tick {t}")],
                Err(e) => vec![format!("ckpt error: {e}")],
            }
        }
        UiCommand::CkptPrev => {
            if state.remote {
                return vec!["ckpt step is in-process only (not on the attach wire)".into()];
            }
            match scrub.prev(state) {
                Ok(t) => vec![format!("loaded tick {t}")],
                Err(e) => vec![format!("ckpt error: {e}")],
            }
        }
        UiCommand::Events { tick } => {
            if state.remote {
                return vec!["events timeline is in-process only (not on the attach wire)".into()];
            }
            match scrub.filter_events(tick) {
                Ok(t) => vec![format!("events tick {t}")],
                Err(e) => vec![format!("events error: {e}")],
            }
        }
        UiCommand::Set { id, field, value } => {
            if state.remote {
                return vec!["set is in-process only (not on the attach wire)".into()];
            }
            match state
                .sim
                .set_display_field(sim_core::AgentId(id), &field, value)
            {
                Ok(milli) => vec![format!("set agent {id} {field}={value} ({milli} milli)")],
                Err(e) => vec![format!("set error: {e}")],
            }
        }
    }
}

fn write_report_cmd(
    sim: &sim_core::Simulation,
    dir: Option<&str>,
) -> Result<PathBuf, sim_core::SimError> {
    let dir = dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&sim.config.checkpoint.directory));
    let (md, _) = write_report(sim, dir)?;
    Ok(md)
}

fn save_cmd(sim: &sim_core::Simulation, path: Option<&str>) -> Result<PathBuf, sim_core::SimError> {
    if let Some(p) = path {
        let p = Path::new(p);
        sim.save_checkpoint(p)?;
        Ok(p.to_path_buf())
    } else {
        write_run_checkpoint(sim, &sim.config.checkpoint.directory)
    }
}

#[allow(dead_code)]
pub fn default_config_for_tests() -> ExperimentConfig {
    ExperimentConfig::from_toml_str(
        r#"
master_seed = 1
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
    )
    .expect("test config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::Simulation;

    #[test]
    fn parse_help() {
        let cmd = parse_command("/help").unwrap();
        assert_eq!(cmd, UiCommand::Help);
        let text = help_text();
        assert!(text.contains("/report"));
        assert!(text.contains("/follow"));
        assert!(text.contains("/inject"));
        assert!(text.contains("/give"));
        assert!(text.contains("/set"));
        assert!(text.contains("/events"));
        assert!(text.contains("/scrub"));
        assert!(text.contains("/ckpt"));
    }

    #[test]
    fn parse_scrub_and_ckpt() {
        assert_eq!(
            parse_command("/scrub 40").unwrap(),
            UiCommand::Scrub { tick: 40 }
        );
        assert_eq!(parse_command("/ckpt next").unwrap(), UiCommand::CkptNext);
        assert_eq!(parse_command("/ckpt prev").unwrap(), UiCommand::CkptPrev);
        assert_eq!(
            parse_command("/ckpt previous").unwrap(),
            UiCommand::CkptPrev
        );
        assert!(parse_command("/scrub").unwrap_err().contains("tick"));
        assert!(parse_command("/ckpt").unwrap_err().contains("next|prev"));
        assert!(
            parse_command("/ckpt jump")
                .unwrap_err()
                .contains("next|prev")
        );
    }

    #[test]
    fn scrubber_loads_at_or_before_and_steps() {
        let dir = std::env::temp_dir().join(format!("m14-scrub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut sim = Simulation::new(default_config_for_tests()).unwrap();
        sim.run_ticks(2);
        sim.save_checkpoint(dir.join("run_tick_2.ckpt")).unwrap();
        let hash2 = sim.state_hash();
        sim.run_ticks(3);
        sim.save_checkpoint(dir.join("run_tick_5.ckpt")).unwrap();
        let latest = CkptScrubber::initial_path(&dir).unwrap();
        assert!(
            latest.file_name().unwrap().to_string_lossy().contains("5"),
            "{latest:?}"
        );
        let mut scrub = CkptScrubber::discover(&dir);
        let mut state = SimState {
            sim,
            paused: false,
            follow: None,
            remote: false,
        };
        let t = scrub.apply(&mut state, 4).unwrap();
        assert_eq!(t, 2);
        assert_eq!(state.sim.tick, 2);
        assert_eq!(state.sim.state_hash(), hash2);
        assert!(state.paused);
        let t = scrub.next(&mut state).unwrap();
        assert_eq!(t, 5);
        assert_eq!(state.sim.tick, 5);
        let t = scrub.prev(&mut state).unwrap();
        assert_eq!(t, 2);
        let msgs = run_command(
            UiCommand::Scrub { tick: 0 },
            &mut state,
            &mut false,
            &mut WindowFlags::default(),
            &mut scrub,
        );
        assert!(msgs[0].contains("scrub error"), "{msgs:?}");
        state.remote = true;
        let msgs = run_command(
            UiCommand::Scrub { tick: 5 },
            &mut state,
            &mut false,
            &mut WindowFlags::default(),
            &mut scrub,
        );
        assert!(msgs[0].contains("in-process only"), "{msgs:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_events() {
        assert_eq!(
            parse_command("/events 40").unwrap(),
            UiCommand::Events { tick: 40 }
        );
        let dir = std::env::temp_dir().join(format!("m17-ev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("run_events.jsonl"),
            "{\"tick\":10}\n{\"tick\":40}\n{\"tick\":80}\n",
        )
        .unwrap();
        let mut scrub = CkptScrubber::discover(&dir);
        assert_eq!(scrub.filter_events(50).unwrap(), 40);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_set() {
        assert_eq!(
            parse_command("/set 0 hunger 50").unwrap(),
            UiCommand::Set {
                id: 0,
                field: "hunger".into(),
                value: 50
            }
        );
        assert!(parse_command("/set 0 nope 1").is_ok());
        let mut sim = Simulation::new(default_config_for_tests()).unwrap();
        let before = sim.state_hash();
        let milli = sim
            .set_display_field(sim_core::AgentId(0), "hunger", 50)
            .unwrap();
        assert_eq!(milli, 5000);
        assert_eq!(
            sim.agents.get(&sim_core::AgentId(0)).unwrap().needs.hunger,
            5000
        );
        assert_ne!(before, sim.state_hash());
        let mut state = SimState {
            sim,
            paused: true,
            follow: None,
            remote: true,
        };
        let msgs = run_command(
            UiCommand::Set {
                id: 0,
                field: "hunger".into(),
                value: 10,
            },
            &mut state,
            &mut false,
            &mut WindowFlags::default(),
            &mut CkptScrubber::default(),
        );
        assert!(msgs[0].contains("in-process only"), "{msgs:?}");
    }

    #[test]
    fn parse_give() {
        let cmd = parse_command("/give 0 berry_bush 2").unwrap();
        assert_eq!(
            cmd,
            UiCommand::Give {
                id: 0,
                item: "berry_bush".into(),
                qty: 2
            }
        );
    }

    #[test]
    fn parse_unknown() {
        let err = parse_command("/nope").unwrap_err();
        assert!(err.contains("unknown"), "{err}");
    }

    #[test]
    fn parse_step_and_follow() {
        assert_eq!(parse_command("/step").unwrap(), UiCommand::Step { n: 1 });
        assert_eq!(parse_command("/step 5").unwrap(), UiCommand::Step { n: 5 });
        assert_eq!(
            parse_command("/follow off").unwrap(),
            UiCommand::Follow { id: None }
        );
        assert_eq!(
            parse_command("/follow 3").unwrap(),
            UiCommand::Follow { id: Some(3) }
        );
        assert_eq!(
            parse_command("/fog on").unwrap(),
            UiCommand::Fog { on: Some(true) }
        );
        assert_eq!(
            parse_command("/inject configs/incentives/coop.toml").unwrap(),
            UiCommand::Inject {
                path: Some("configs/incentives/coop.toml".into())
            }
        );
    }

    #[test]
    fn report_does_not_change_hash() {
        let mut sim = Simulation::new(default_config_for_tests()).unwrap();
        sim.run_ticks(2);
        let before = sim.state_hash();
        let dir = std::env::temp_dir().join(format!("m6-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_report_cmd(&sim, Some(dir.to_str().unwrap())).unwrap();
        assert!(path.exists(), "{}", path.display());
        assert_eq!(before, sim.state_hash());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
