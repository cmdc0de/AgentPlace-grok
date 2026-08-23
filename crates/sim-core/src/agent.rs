use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agent {
    pub id: AgentId,
    pub x: u32,
    pub y: u32,
}

impl Agent {
    pub fn new(id: AgentId, x: u32, y: u32) -> Self {
        Self { id, x, y }
    }
}
