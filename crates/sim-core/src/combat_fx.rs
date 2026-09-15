//! Hash-neutral combat roles, HUD lines, and viewer FX jobs. Not part of `state_hash`.

use crate::agent::AgentId;
use crate::event_log::{SimEvent, SimEventKind};
use crate::observation::chebyshev;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRole {
    None,
    Attacker,
    Defender,
    Flee,
}

/// Viewer mesh job for this tick. Not hashed, not checkpointed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CombatFxJob {
    Strike { from: AgentId, to: AgentId },
    Flee { agent: AgentId },
    Downed { agent: AgentId },
    Death { agent: AgentId },
    Projectile { from: AgentId, to: AgentId },
}

pub fn combat_fx_jobs(
    events: &[SimEvent],
    tick: u64,
    pos: impl Fn(AgentId) -> Option<(u32, u32)>,
) -> Vec<CombatFxJob> {
    let mut jobs = Vec::new();
    for e in events {
        if e.tick != tick {
            continue;
        }
        match e.kind {
            SimEventKind::Attack { target, .. } => {
                let dist = match (pos(e.agent), pos(target)) {
                    (Some((ax, ay)), Some((bx, by))) => chebyshev(ax, ay, bx, by),
                    _ => 1,
                };
                if dist > 1 {
                    jobs.push(CombatFxJob::Projectile {
                        from: e.agent,
                        to: target,
                    });
                } else {
                    jobs.push(CombatFxJob::Strike {
                        from: e.agent,
                        to: target,
                    });
                }
            }
            SimEventKind::Flee => jobs.push(CombatFxJob::Flee { agent: e.agent }),
            SimEventKind::Incapacitated { .. } => jobs.push(CombatFxJob::Downed { agent: e.agent }),
            SimEventKind::CombatDeath { .. } => jobs.push(CombatFxJob::Death { agent: e.agent }),
            _ => {}
        }
    }
    jobs
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

pub fn combat_hud_lines(events: &[SimEvent], tick: u64) -> Vec<String> {
    let mut lines = Vec::new();
    for e in events {
        if e.tick != tick {
            continue;
        }
        match e.kind {
            SimEventKind::Attack { target, damage } => {
                lines.push(format!(
                    "tick {tick} attack #{} → #{} dmg {damage}",
                    e.agent.0, target.0
                ));
            }
            SimEventKind::Flee => {
                lines.push(format!("tick {tick} flee #{}", e.agent.0));
            }
            SimEventKind::Incapacitated { by } => {
                lines.push(format!(
                    "tick {tick} incapacitated #{} by #{}",
                    e.agent.0, by.0
                ));
            }
            SimEventKind::CombatDeath { by } => {
                lines.push(format!(
                    "tick {tick} combat_death #{} by #{}",
                    e.agent.0, by.0
                ));
            }
            _ => {}
        }
    }
    lines
}

pub fn combat_hud_line(events: &[SimEvent], tick: u64) -> Option<String> {
    combat_hud_lines(events, tick).into_iter().next()
}
