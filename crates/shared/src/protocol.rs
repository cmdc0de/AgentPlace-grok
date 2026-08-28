//! Length-prefixed postcard frames (`u32` LE length + payload).

use crate::{MAX_FRAME_BYTES, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{Read, Write};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello {
        protocol_version: u16,
        token: Option<String>,
    },
    Subscribe {
        want_events: bool,
        want_decisions: bool,
    },
    RequestSnapshot,
    Control(ControlVerb),
    InjectIncentive {
        schedule_toml: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlVerb {
    Pause,
    Play,
    Step(u32),
    Save,
    Report,
    Summarize,
    Scrub(u64),
    Give { id: u64, item: String, qty: u32 },
    CkptNext,
    CkptPrev,
    Events(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        tick: u64,
        state_hash: [u8; 32],
        experiment_id: String,
    },
    Snapshot {
        checkpoint_bytes: Vec<u8>,
    },
    Tick {
        tick: u64,
        state_hash: [u8; 32],
        /// JSON `Vec<sim_core::SimEvent>` (empty if the client did not ask).
        events: Vec<u8>,
        /// JSON `Vec<sim_core::DecisionRecord>` (empty if the client did not ask).
        decisions: Vec<u8>,
        /// JSON `sim_core::TickTiming` (empty if unmeasured).
        metrics: Vec<u8>,
    },
    ReportReady {
        markdown_or_path: String,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    Protocol,
    Unauthorized,
    ControlDisabled,
    NotImplemented,
    Internal,
}

#[derive(Debug)]
pub enum CodecError {
    Io(std::io::Error),
    Postcard(String),
    FrameTooLarge(usize),
    Truncated,
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::Io(e) => write!(f, "io: {e}"),
            CodecError::Postcard(e) => write!(f, "postcard: {e}"),
            CodecError::FrameTooLarge(n) => write!(f, "frame too large: {n} bytes"),
            CodecError::Truncated => write!(f, "truncated frame"),
        }
    }
}

impl std::error::Error for CodecError {}

impl From<std::io::Error> for CodecError {
    fn from(e: std::io::Error) -> Self {
        CodecError::Io(e)
    }
}

pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, CodecError> {
    let payload = postcard::to_allocvec(msg).map_err(|e| CodecError::Postcard(e.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(CodecError::FrameTooLarge(payload.len()));
    }
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

pub fn decode_frame<T: DeserializeOwned>(frame: &[u8]) -> Result<T, CodecError> {
    if frame.len() < 4 {
        return Err(CodecError::Truncated);
    }
    let len = u32::from_le_bytes(frame[0..4].try_into().unwrap()) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(CodecError::FrameTooLarge(len));
    }
    if frame.len() != 4 + len {
        return Err(CodecError::Truncated);
    }
    postcard::from_bytes(&frame[4..]).map_err(|e| CodecError::Postcard(e.to_string()))
}

pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<(), CodecError> {
    let frame = encode_frame(msg)?;
    writer.write_all(&frame)?;
    writer.flush()?;
    Ok(())
}

pub fn read_frame<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T, CodecError> {
    let mut header = [0u8; 4];
    reader.read_exact(&mut header)?;
    let len = u32::from_le_bytes(header) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(CodecError::FrameTooLarge(len));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    postcard::from_bytes(&payload).map_err(|e| CodecError::Postcard(e.to_string()))
}

pub fn hello(token: Option<String>) -> ClientMessage {
    ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip_client(msg: ClientMessage) {
        let bytes = encode_frame(&msg).unwrap();
        let back: ClientMessage = decode_frame(&bytes).unwrap();
        assert_eq!(msg, back);
    }

    fn round_trip_server(msg: ServerMessage) {
        let bytes = encode_frame(&msg).unwrap();
        let back: ServerMessage = decode_frame(&bytes).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn codec_round_trip_hello_welcome_snapshot_tick_error() {
        round_trip_client(hello(Some("secret".into())));
        round_trip_client(ClientMessage::Subscribe {
            want_events: true,
            want_decisions: false,
        });
        round_trip_client(ClientMessage::RequestSnapshot);
        round_trip_client(ClientMessage::Control(ControlVerb::Step(3)));
        round_trip_client(ClientMessage::Control(ControlVerb::Scrub(50)));
        round_trip_client(ClientMessage::Control(ControlVerb::Give {
            id: 0,
            item: "berry_bush".into(),
            qty: 1,
        }));
        round_trip_client(ClientMessage::Control(ControlVerb::CkptNext));
        round_trip_client(ClientMessage::Control(ControlVerb::Events(2)));
        round_trip_client(ClientMessage::InjectIncentive {
            schedule_toml: "[[incentives]]".into(),
        });

        round_trip_server(ServerMessage::Welcome {
            tick: 12,
            state_hash: [7; 32],
            experiment_id: "deadbeef".into(),
        });
        round_trip_server(ServerMessage::Snapshot {
            checkpoint_bytes: vec![b'A', b'G', b'T', b'N', 1, 2, 3],
        });
        round_trip_server(ServerMessage::Tick {
            tick: 4,
            state_hash: [9; 32],
            events: b"[{\"tick\":1}]".to_vec(),
            decisions: b"[]".to_vec(),
            metrics: b"{}".to_vec(),
        });
        round_trip_server(ServerMessage::Error {
            code: ErrorCode::Protocol,
            message: "bad version".into(),
        });
    }

    #[test]
    fn unknown_protocol_version_is_just_a_field() {
        let msg = ClientMessage::Hello {
            protocol_version: 99,
            token: None,
        };
        let bytes = encode_frame(&msg).unwrap();
        let back: ClientMessage = decode_frame(&bytes).unwrap();
        match back {
            ClientMessage::Hello {
                protocol_version, ..
            } => assert_eq!(protocol_version, 99),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn truncated_frame_errors() {
        assert!(decode_frame::<ClientMessage>(&[1, 0]).is_err());
        let mut frame = encode_frame(&hello(None)).unwrap();
        frame.pop();
        assert!(decode_frame::<ClientMessage>(&frame).is_err());
    }
}
