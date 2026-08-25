//! Closed slash-command set for the viewer console. No GPU / imgui types.

use shared::protocol::ControlVerb;
use sim_bevy::{step_once, SimState};
use sim_core::{summary_markdown, write_report, write_run_checkpoint, AgentId, ExperimentConfig};
use std::path::{Path, PathBuf};

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
  /tick"
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
