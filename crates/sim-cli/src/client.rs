//! Attach log tail (`--connect`). With `--allow-control`, stdin slash commands send Control.

use shared::protocol::{ClientMessage, ControlVerb, ServerMessage, hello};
use shared::transport::Connection;
use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn hash_hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse a connect stdin line (`/play`, `play`, …) into a ControlVerb.
pub fn parse_control_line(line: &str) -> Result<ControlVerb, String> {
    let line = line.trim();
    let line = line.strip_prefix('/').unwrap_or(line).trim();
    if line.is_empty() {
        return Err("empty command".into());
    }
    let mut parts = line.split_whitespace();
    let verb = parts.next().unwrap_or("").to_ascii_lowercase();
    let arg = parts.next();
    match verb.as_str() {
        "pause" => Ok(ControlVerb::Pause),
        "play" | "unpause" => Ok(ControlVerb::Play),
        "step" => {
            let n = match arg {
                None => 1,
                Some(s) => s.parse().map_err(|_| format!("bad step count: {s}"))?,
            };
            Ok(ControlVerb::Step(n.max(1)))
        }
        "save" => Ok(ControlVerb::Save),
        "report" => Ok(ControlVerb::Report),
        "summarize" | "summary" => Ok(ControlVerb::Summarize),
        "scrub" => {
            let t = arg.ok_or("scrub requires a tick")?;
            let tick: u64 = t.parse().map_err(|_| format!("bad tick: {t}"))?;
            Ok(ControlVerb::Scrub(tick))
        }
        "give" => {
            let id_s = arg.ok_or("give requires agent id")?;
            let id: u64 = id_s.parse().map_err(|_| format!("bad agent id: {id_s}"))?;
            let item = parts.next().ok_or("give requires item name")?.to_string();
            let qty = match parts.next() {
                None => 1,
                Some(s) => s.parse().map_err(|_| format!("bad qty: {s}"))?,
            };
            Ok(ControlVerb::Give {
                id,
                item,
                qty: qty.max(1),
            })
        }
        "ckpt" => match arg {
            Some("next") => Ok(ControlVerb::CkptNext),
            Some("prev") | Some("previous") => Ok(ControlVerb::CkptPrev),
            Some(s) => Err(format!("ckpt expects next|prev, got {s}")),
            None => Err("ckpt requires next|prev".into()),
        },
        "events" => {
            let t = arg.ok_or("events requires a tick")?;
            let tick: u64 = t.parse().map_err(|_| format!("bad tick: {t}"))?;
            Ok(ControlVerb::Events(tick))
        }
        "set" => Err("set is in-process only".into()),
        "inject" => Err("inject is listen-side".into()),
        other => Err(format!("unknown: /{other}")),
    }
}

pub fn log_tail(
    url: &str,
    token: Option<String>,
    _quiet: bool,
    allow_control: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = Connection::connect(url)?;
    conn.send_msg(&hello(token))?;
    let welcome: ServerMessage = conn.recv_msg()?;
    match welcome {
        ServerMessage::Welcome {
            tick,
            state_hash,
            experiment_id,
        } => {
            println!(
                "welcome tick={tick} hash={} experiment={experiment_id}",
                hash_hex(&state_hash)
            );
        }
        ServerMessage::Error { message, .. } => {
            return Err(format!("server rejected Hello: {message}").into());
        }
        other => return Err(format!("expected Welcome, got {other:?}").into()),
    }
    let snap: ServerMessage = conn.recv_msg()?;
    match snap {
        ServerMessage::Snapshot { .. } => {
            println!("snapshot ok");
        }
        ServerMessage::Error { message, .. } => {
            return Err(format!("server error: {message}").into());
        }
        other => return Err(format!("expected Snapshot, got {other:?}").into()),
    }
    conn.send_msg(&ClientMessage::Subscribe {
        want_events: false,
        want_decisions: false,
    })?;

    if !allow_control {
        return recv_loop(&mut conn, None);
    }

    let (to_io, from_main) = mpsc::channel::<ClientMessage>();
    let (done_tx, done_rx) = mpsc::channel::<Result<(), String>>();
    thread::spawn(move || {
        let r = recv_loop(&mut conn, Some(from_main)).map_err(|e| e.to_string());
        let _ = done_tx.send(r);
    });
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        match parse_control_line(&line) {
            Ok(verb) => {
                if to_io.send(ClientMessage::Control(verb)).is_err() {
                    break;
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }
    match done_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Ok(()),
    }
}

fn recv_loop(
    conn: &mut Connection,
    from_main: Option<mpsc::Receiver<ClientMessage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.set_read_timeout(Some(Duration::from_millis(50)))?;
    loop {
        if let Some(rx) = &from_main {
            while let Ok(msg) = rx.try_recv() {
                conn.send_msg(&msg)?;
            }
        }
        match conn.recv_msg::<ServerMessage>() {
            Ok(ServerMessage::Tick {
                tick, state_hash, ..
            }) => {
                println!("tick={tick} hash={}", hash_hex(&state_hash));
            }
            Ok(ServerMessage::ReportReady { markdown_or_path }) => {
                println!("{markdown_or_path}");
            }
            Ok(ServerMessage::Error { message, .. }) => {
                println!("error: {message}");
            }
            Ok(_) => {}
            Err(shared::transport::TransportError::Closed) => return Ok(()),
            Err(shared::transport::TransportError::Timeout) => continue,
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_play_pause_step_give() {
        assert_eq!(parse_control_line("/play").unwrap(), ControlVerb::Play);
        assert_eq!(parse_control_line("pause").unwrap(), ControlVerb::Pause);
        assert_eq!(parse_control_line("/step 3").unwrap(), ControlVerb::Step(3));
        assert_eq!(
            parse_control_line("/give 0 berry_bush 1").unwrap(),
            ControlVerb::Give {
                id: 0,
                item: "berry_bush".into(),
                qty: 1
            }
        );
        assert_eq!(
            parse_control_line("/ckpt next").unwrap(),
            ControlVerb::CkptNext
        );
        assert_eq!(
            parse_control_line("/events 2").unwrap(),
            ControlVerb::Events(2)
        );
        assert_eq!(
            parse_control_line("/scrub 10").unwrap(),
            ControlVerb::Scrub(10)
        );
    }

    #[test]
    fn parse_set_is_refused() {
        let e = parse_control_line("/set 0 hunger 10").unwrap_err();
        assert!(e.contains("in-process"), "{e}");
    }
}
