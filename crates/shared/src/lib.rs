//! Shared protocol and transports.
//!
//! One postcard codec, two sockets (TCP and WebSocket). Blocking I/O, no tokio.

pub const PROTOCOL_VERSION: u16 = 5;

/// Maximum framed payload (snapshot of a 64×64 world is well under this).
pub const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotHeader {
    pub protocol_version: u16,
    pub tick: u64,
    pub agent_count: u32,
    pub world_width: u32,
    pub world_height: u32,
}

pub mod protocol;
pub mod transport;

pub use protocol::{
    ClientMessage, ControlVerb, ErrorCode, ServerMessage, decode_frame, encode_frame, read_frame,
    write_frame,
};
pub use transport::{Connection, Listener, Scheme, TransportError, parse_listen_url};
