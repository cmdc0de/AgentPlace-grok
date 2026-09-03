//! Hash-neutral combat roles for viewer tint/HUD. Not part of `state_hash`.

use crate::agent::AgentId;
use crate::event_log::{SimEvent, SimEventKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRole {
    None,
    Attacker,
    Defender,
    Flee,
}

pub fn combat_role(events: &[SimEvent], tick: u64, id: AgentId) -> CombatRole {
    let mut role = CombatRole::None;
    for e in events {
        if e.tick != tick {
            continue;
        }
        match e.kind {
            SimEventKind::Attack { target, .. } => {
                if e.agent == id {
                    return CombatRole::Attacker;
                }
                if target == id {
                    role = CombatRole::Defender;
                }
            }
            SimEventKind::Flee if e.agent == id && role == CombatRole::None => {
                role = CombatRole::Flee;
            }
            _ => {}
        }
    }
    role
}

pub fn combat_hud_line(events: &[SimEvent], tick: u64) -> Option<String> {
    for e in events {
        if e.tick != tick {
            continue;
        }
        match e.kind {
            SimEventKind::Attack { target, damage } => {
                return Some(format!(
                    "tick {tick} attack #{} → #{} dmg {damage}",
                    e.agent.0, target.0
                ));
            }
            SimEventKind::Flee => {
                return Some(format!("tick {tick} flee #{}", e.agent.0));
            }
            _ => {}
        }
    }
    None
}
