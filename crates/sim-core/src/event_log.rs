use crate::agent::AgentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimEventKind {
    Wait,
    Move { from_x: u32, from_y: u32, to_x: u32, to_y: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimEvent {
    pub tick: u64,
    pub agent: AgentId,
    pub kind: SimEventKind,
}

#[derive(Debug, Clone, Default)]
pub struct EventLog {
    pub events: Vec<SimEvent>,
}

impl EventLog {
    pub fn push(&mut self, event: SimEvent) {
        self.events.push(event);
    }
}
