//! Read-only attach log tail (`--connect`). Does not send Control.

use shared::protocol::{ClientMessage, ServerMessage, hello};
use shared::transport::Connection;

fn hash_hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn log_tail(
    url: &str,
    token: Option<String>,
    _quiet: bool,
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
    loop {
        match conn.recv_msg::<ServerMessage>() {
            Ok(ServerMessage::Tick {
                tick, state_hash, ..
            }) => {
                println!("tick={tick} hash={}", hash_hex(&state_hash));
            }
            Ok(ServerMessage::Error { message, .. }) => {
                return Err(format!("server error: {message}").into());
            }
            Ok(_) => {}
            Err(shared::transport::TransportError::Closed) => return Ok(()),
            Err(shared::transport::TransportError::Timeout) => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
