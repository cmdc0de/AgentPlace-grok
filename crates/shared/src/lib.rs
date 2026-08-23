//! Shared protocol and transports.
//!
//! M1 only defines versioning and a snapshot DTO. TCP / WebSocket backends
//! land in a later milestone; the module layout is reserved now.

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotHeader {
    pub protocol_version: u16,
    pub tick: u64,
    pub agent_count: u32,
    pub world_width: u32,
    pub world_height: u32,
}

pub mod protocol {
    pub use super::SnapshotHeader;
    pub use super::PROTOCOL_VERSION;
}

pub mod transport {
    //! `TcpTransport` and `WebSocketTransport` will implement a shared trait here.
}
