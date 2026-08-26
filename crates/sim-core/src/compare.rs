//! Derived A/B compare. Not checkpointed, not in `state_hash`.

use crate::checkpoint::list_checkpoints;
use crate::error::SimError;
use crate::event_log::{SimEventKind, kind_label};
use crate::simulation::Simulation;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct RunSnapshot {
    pub path: String,
    pub tick: u64,
    pub state_hash: String,
    pub population: u32,
    pub died: u32,
    pub first_death_tick: Option<u64>,
    pub board_open: u32,
    pub board_accepted: u32,
    pub board_rejected: u32,
    pub board_adopted: u32,
    pub consumed_vegetation: u32,
    pub consumed_animal: u32,
    pub consumed_fish: u32,
    pub toxic_events: u32,
    pub hunger_mean: f64,
    pub thirst_mean: f64,
    pub energy_mean: f64,
    pub goal_occupancy: BTreeMap<String, u32>,
    pub active_incentives: BTreeSet<String>,
    pub event_histogram: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompareReport {
    pub a: RunSnapshot,
    pub b: RunSnapshot,
}

impl CompareReport {
    pub fn hashes_equal(&self) -> bool {
        self.a.state_hash == self.b.state_hash
    }
}

pub fn snapshot_sim(sim: &Simulation, path: impl Into<String>) -> RunSnapshot {
    let n = sim.agents.len().max(1) as f64;
    let hunger_mean = sim
        .agents
        .values()
        .map(|a| a.needs.hunger as f64 / 100.0)
        .sum::<f64>()
        / n;
    let thirst_mean = sim
        .agents
        .values()
        .map(|a| a.needs.thirst as f64 / 100.0)
        .sum::<f64>()
        / n;
    let energy_mean = sim
        .agents
        .values()
        .map(|a| a.needs.energy as f64 / 100.0)
        .sum::<f64>()
        / n;
    let mut died = 0u32;
    let mut first_death_tick = None;
    let mut event_histogram = BTreeMap::new();
    for e in &sim.events.events {
        *event_histogram
            .entry(kind_label(&e.kind).to_string())
            .or_insert(0) += 1;
        if matches!(e.kind, SimEventKind::Died { .. }) {
            died += 1;
            first_death_tick = Some(first_death_tick.map_or(e.tick, |t: u64| t.min(e.tick)));
        }
    }
    let mut goal_occupancy = BTreeMap::new();
    for agent in sim.agents.values() {
        for g in &agent.goals {
            *goal_occupancy.entry(g.text.clone()).or_insert(0) += 1;
        }
    }
    let consumed_vegetation = sim.agents.values().map(|a| a.consumption.vegetation).sum();
    let consumed_animal = sim.agents.values().map(|a| a.consumption.animal).sum();
    let consumed_fish = sim.agents.values().map(|a| a.consumption.fish).sum();
    let toxic_events = sim
        .agents
        .values()
        .map(|a| a.consumption.toxic_events)
        .sum();
    RunSnapshot {
        path: path.into(),
        tick: sim.tick,
        state_hash: sim.state_hash().to_string(),
        population: sim.agents.len() as u32,
        died,
        first_death_tick,
        board_open: sim.board.open().count() as u32,
        board_accepted: sim.board.accepted_count,
        board_rejected: sim.board.rejected_count,
        board_adopted: sim.board.adopted.len() as u32,
        consumed_vegetation,
        consumed_animal,
        consumed_fish,
        toxic_events,
        hunger_mean,
        thirst_mean,
        energy_mean,
        goal_occupancy,
        active_incentives: sim.incentive_active.clone(),
        event_histogram,
    }
}

pub fn compare_runs(a: &Simulation, path_a: &str, b: &Simulation, path_b: &str) -> CompareReport {
    CompareReport {
        a: snapshot_sim(a, path_a),
        b: snapshot_sim(b, path_b),
    }
}

/// Load two checkpoints or `--out-dir`s at a comparable tick (max shared tick, else latest).
pub fn load_compare_pair(
    path_a: &Path,
    path_b: &Path,
) -> Result<(Simulation, Simulation), SimError> {
    if path_a.is_dir() && path_b.is_dir() {
        let ca = list_checkpoints(path_a)?;
        let cb = list_checkpoints(path_b)?;
        let ticks_a: BTreeSet<u64> = ca.iter().map(|(t, _)| *t).collect();
        if let Some(t) = cb
            .iter()
            .map(|(tick, _)| *tick)
            .filter(|tick| ticks_a.contains(tick))
            .max()
        {
            let pa = ckpt_at(&ca, t)?;
            let pb = ckpt_at(&cb, t)?;
            return Ok((
                Simulation::load_checkpoint(pa)?,
                Simulation::load_checkpoint(pb)?,
            ));
        }
    }
    let pa = resolve_ckpt(path_a, None)?;
    let sa = Simulation::load_checkpoint(&pa)?;
    let pb = resolve_ckpt(path_b, Some(sa.tick))?;
    let sb = Simulation::load_checkpoint(&pb)?;
    Ok((sa, sb))
}

fn ckpt_at(ckpts: &[(u64, PathBuf)], tick: u64) -> Result<PathBuf, SimError> {
    ckpts
        .iter()
        .find(|(t, _)| *t == tick)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| SimError::Checkpoint(format!("no checkpoint at tick {tick}")))
}

fn resolve_ckpt(path: &Path, prefer_tick: Option<u64>) -> Result<PathBuf, SimError> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    if !path.is_dir() {
        return Err(SimError::Checkpoint(format!(
            "compare path does not exist: {}",
            path.display()
        )));
    }
    let mut ckpts = list_checkpoints(path)?;
    if ckpts.is_empty() {
        return Err(SimError::Checkpoint(format!(
            "no .ckpt files in {}",
            path.display()
        )));
    }
    if let Some(t) = prefer_tick {
        if let Some((_, p)) = ckpts.iter().find(|(tick, _)| *tick == t) {
            return Ok(p.clone());
        }
    }
    Ok(ckpts.pop().unwrap().1)
}

pub fn compare_markdown(report: &CompareReport) -> String {
    let a = &report.a;
    let b = &report.b;
    let hash_line = if report.hashes_equal() {
        "equal"
    } else {
        "differ"
    };
    let mut out = format!(
        "# Compare\n\n\
         - A: `{}` tick {}\n\
         - B: `{}` tick {}\n\n\
         | field | A | B | delta |\n\
         |---|---|---|---|\n\
         | state_hash | `{}` | `{}` | {} |\n\
         | population | {} | {} | {} |\n\
         | Died | {} | {} | {} |\n\
         | first death tick | {} | {} | |\n\
         | board open | {} | {} | {} |\n\
         | board accepted | {} | {} | {} |\n\
         | board rejected | {} | {} | {} |\n\
         | board adopted | {} | {} | {} |\n\
         | consumed veg | {} | {} | {} |\n\
         | consumed animal | {} | {} | {} |\n\
         | consumed fish | {} | {} | {} |\n\
         | toxic events | {} | {} | {} |\n\
         | hunger mean | {:.1} | {:.1} | {:.1} |\n\
         | thirst mean | {:.1} | {:.1} | {:.1} |\n\
         | energy mean | {:.1} | {:.1} | {:.1} |\n",
        a.path,
        a.tick,
        b.path,
        b.tick,
        a.state_hash,
        b.state_hash,
        hash_line,
        a.population,
        b.population,
        i64::from(b.population) - i64::from(a.population),
        a.died,
        b.died,
        i64::from(b.died) - i64::from(a.died),
        opt_tick(a.first_death_tick),
        opt_tick(b.first_death_tick),
        a.board_open,
        b.board_open,
        i64::from(b.board_open) - i64::from(a.board_open),
        a.board_accepted,
        b.board_accepted,
        i64::from(b.board_accepted) - i64::from(a.board_accepted),
        a.board_rejected,
        b.board_rejected,
        i64::from(b.board_rejected) - i64::from(a.board_rejected),
        a.board_adopted,
        b.board_adopted,
        i64::from(b.board_adopted) - i64::from(a.board_adopted),
        a.consumed_vegetation,
        b.consumed_vegetation,
        i64::from(b.consumed_vegetation) - i64::from(a.consumed_vegetation),
        a.consumed_animal,
        b.consumed_animal,
        i64::from(b.consumed_animal) - i64::from(a.consumed_animal),
        a.consumed_fish,
        b.consumed_fish,
        i64::from(b.consumed_fish) - i64::from(a.consumed_fish),
        a.toxic_events,
        b.toxic_events,
        i64::from(b.toxic_events) - i64::from(a.toxic_events),
        a.hunger_mean,
        b.hunger_mean,
        b.hunger_mean - a.hunger_mean,
        a.thirst_mean,
        b.thirst_mean,
        b.thirst_mean - a.thirst_mean,
        a.energy_mean,
        b.energy_mean,
        b.energy_mean - a.energy_mean,
    );
    out.push_str("\n## Active incentives\n\n");
    let ids: BTreeSet<_> = a
        .active_incentives
        .union(&b.active_incentives)
        .cloned()
        .collect();
    if ids.is_empty() {
        out.push_str("- (none)\n");
    } else {
        for id in ids {
            let in_a = a.active_incentives.contains(&id);
            let in_b = b.active_incentives.contains(&id);
            out.push_str(&format!("- `{id}` A={in_a} B={in_b}\n"));
        }
    }
    out.push_str("\n## Goal occupancy\n\n");
    let mut goals: BTreeSet<_> = a.goal_occupancy.keys().cloned().collect();
    goals.extend(b.goal_occupancy.keys().cloned());
    if goals.is_empty() {
        out.push_str("- (none)\n");
    } else {
        out.push_str("| text | A | B | delta |\n|---|---|---|---|\n");
        for g in goals {
            let ga = a.goal_occupancy.get(&g).copied().unwrap_or(0);
            let gb = b.goal_occupancy.get(&g).copied().unwrap_or(0);
            out.push_str(&format!(
                "| {} | {ga} | {gb} | {} |\n",
                g,
                i64::from(gb) - i64::from(ga)
            ));
        }
    }
    out.push_str("\n## Event kinds\n\n");
    let mut kinds: BTreeSet<_> = a.event_histogram.keys().cloned().collect();
    kinds.extend(b.event_histogram.keys().cloned());
    out.push_str("| kind | A | B | delta |\n|---|---|---|---|\n");
    for k in kinds {
        let ka = a.event_histogram.get(&k).copied().unwrap_or(0);
        let kb = b.event_histogram.get(&k).copied().unwrap_or(0);
        out.push_str(&format!(
            "| {k} | {ka} | {kb} | {} |\n",
            i64::from(kb) - i64::from(ka)
        ));
    }
    out
}

pub fn compare_csv(report: &CompareReport) -> String {
    let a = &report.a;
    let b = &report.b;
    let mut out = String::from("field,a,b,delta\n");
    row(
        &mut out,
        "tick",
        &a.tick.to_string(),
        &b.tick.to_string(),
        &(i64::try_from(b.tick).unwrap_or(0) - i64::try_from(a.tick).unwrap_or(0)).to_string(),
    );
    row(
        &mut out,
        "state_hash",
        &a.state_hash,
        &b.state_hash,
        if report.hashes_equal() {
            "equal"
        } else {
            "differ"
        },
    );
    irow(&mut out, "population", a.population, b.population);
    irow(&mut out, "Died", a.died, b.died);
    irow(&mut out, "board_open", a.board_open, b.board_open);
    irow(
        &mut out,
        "board_accepted",
        a.board_accepted,
        b.board_accepted,
    );
    irow(
        &mut out,
        "board_rejected",
        a.board_rejected,
        b.board_rejected,
    );
    irow(&mut out, "board_adopted", a.board_adopted, b.board_adopted);
    irow(
        &mut out,
        "consumed_vegetation",
        a.consumed_vegetation,
        b.consumed_vegetation,
    );
    irow(
        &mut out,
        "consumed_animal",
        a.consumed_animal,
        b.consumed_animal,
    );
    irow(&mut out, "consumed_fish", a.consumed_fish, b.consumed_fish);
    irow(&mut out, "toxic_events", a.toxic_events, b.toxic_events);
    row(
        &mut out,
        "hunger_mean",
        &format!("{:.1}", a.hunger_mean),
        &format!("{:.1}", b.hunger_mean),
        &format!("{:.1}", b.hunger_mean - a.hunger_mean),
    );
    row(
        &mut out,
        "thirst_mean",
        &format!("{:.1}", a.thirst_mean),
        &format!("{:.1}", b.thirst_mean),
        &format!("{:.1}", b.thirst_mean - a.thirst_mean),
    );
    row(
        &mut out,
        "energy_mean",
        &format!("{:.1}", a.energy_mean),
        &format!("{:.1}", b.energy_mean),
        &format!("{:.1}", b.energy_mean - a.energy_mean),
    );
    let mut goals: BTreeSet<_> = a.goal_occupancy.keys().cloned().collect();
    goals.extend(b.goal_occupancy.keys().cloned());
    for g in goals {
        let ga = a.goal_occupancy.get(&g).copied().unwrap_or(0);
        let gb = b.goal_occupancy.get(&g).copied().unwrap_or(0);
        irow(&mut out, &format!("goal:{g}"), ga, gb);
    }
    let ids: BTreeSet<_> = a
        .active_incentives
        .union(&b.active_incentives)
        .cloned()
        .collect();
    for id in ids {
        let ia = u32::from(a.active_incentives.contains(&id));
        let ib = u32::from(b.active_incentives.contains(&id));
        irow(&mut out, &format!("incentive:{id}"), ia, ib);
    }
    let mut kinds: BTreeSet<_> = a.event_histogram.keys().cloned().collect();
    kinds.extend(b.event_histogram.keys().cloned());
    for k in kinds {
        let ka = a.event_histogram.get(&k).copied().unwrap_or(0);
        let kb = b.event_histogram.get(&k).copied().unwrap_or(0);
        irow(&mut out, &format!("event:{k}"), ka, kb);
    }
    out
}

fn opt_tick(t: Option<u64>) -> String {
    t.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
}

fn row(out: &mut String, field: &str, a: &str, b: &str, delta: &str) {
    out.push_str(&format!("{field},{a},{b},{delta}\n"));
}

fn irow(out: &mut String, field: &str, a: u32, b: u32) {
    row(
        out,
        field,
        &a.to_string(),
        &b.to_string(),
        &(i64::from(b) - i64::from(a)).to_string(),
    );
}
