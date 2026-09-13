//! M52 day/night overlay. Default on. `--no-time` restores M51 hashes.

use sim_core::action::PrimaryAction;
use sim_core::clock::{dawn_refill, day_tod};
use sim_core::{ExperimentConfig, InspectorView, Simulation};

fn tiny(seed: u64) -> ExperimentConfig {
    ExperimentConfig::from_toml_str(&format!(
        r#"
master_seed = {seed}
[simulation]
max_ticks = 1000
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#
    ))
    .unwrap()
}

#[test]
fn time_off_same_hash_as_explicit_off() {
    let cfg = tiny(0x52_01);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    a.time_enabled = false;
    let mut b = Simulation::new(cfg).unwrap();
    b.time_enabled = false;
    a.run_ticks(2);
    b.run_ticks(2);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn time_on_hash_differs_from_off() {
    let cfg = tiny(0x52_02);
    let mut on = Simulation::new(cfg.clone()).unwrap();
    on.time_enabled = true;
    let mut off = Simulation::new(cfg).unwrap();
    off.time_enabled = false;
    on.run_ticks(2);
    off.run_ticks(2);
    assert_ne!(on.state_hash(), off.state_hash());
    let (day, tod) = day_tod(on.tick, on.ticks_per_day);
    assert_eq!(day, 0);
    assert_eq!(tod, 2);
}

#[test]
fn inspector_time_keys_when_on() {
    let mut on = Simulation::new(tiny(0x52_03)).unwrap();
    on.time_enabled = true;
    on.run_ticks(2);
    let view = InspectorView::from_sim(&on);
    assert_eq!(view.day, Some(0));
    assert_eq!(view.tod, Some(2));
    assert_eq!(view.ticks_per_day, Some(240));
    let mut off = Simulation::new(tiny(0x52_03)).unwrap();
    off.time_enabled = false;
    off.run_ticks(2);
    let v = InspectorView::from_sim(&off);
    assert!(v.day.is_none());
    assert!(v.tod.is_none());
    assert!(v.ticks_per_day.is_none());
}

#[test]
fn dawn_refill_on_tick_240() {
    let cfg = tiny(0x52_10);
    let mut on = Simulation::new(cfg.clone()).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    on.time_enabled = true;
    on.ticks_per_day = 240;
    off.time_enabled = false;
    on.tick = 239;
    off.tick = 239;
    let id = *on.agents.keys().next().unwrap();
    let max = on.agents[&id]
        .sheet
        .energy_max(on.config.energy_max_milli());
    let start = max / 2;
    on.agents.get_mut(&id).unwrap().needs.energy = start;
    off.agents.get_mut(&id).unwrap().needs.energy = start;
    assert!(on.tick());
    assert!(off.tick());
    assert_eq!(on.tick, 240);
    let extra = on.agents[&id]
        .needs
        .energy
        .saturating_sub(off.agents[&id].needs.energy);
    assert_eq!(extra, dawn_refill(start, max) - start);
}

#[test]
fn dawn_full_energy_stays_clamped() {
    let cfg = tiny(0x52_11);
    let mut on = Simulation::new(cfg.clone()).unwrap();
    let mut off = Simulation::new(cfg).unwrap();
    on.time_enabled = true;
    on.ticks_per_day = 240;
    off.time_enabled = false;
    on.tick = 239;
    off.tick = 239;
    let id = *on.agents.keys().next().unwrap();
    let max = on.agents[&id]
        .sheet
        .energy_max(on.config.energy_max_milli());
    on.agents.get_mut(&id).unwrap().needs.energy = max;
    off.agents.get_mut(&id).unwrap().needs.energy = max;
    assert!(on.tick());
    assert!(off.tick());
    assert_eq!(
        on.agents[&id].needs.energy,
        off.agents[&id].needs.energy,
        "full energy: dawn clamp, same as no-time after the tick"
    );
}

#[test]
fn load_at_dawn_does_not_double_refill() {
    let mut sim = Simulation::new(tiny(0x52_12)).unwrap();
    sim.time_enabled = true;
    sim.ticks_per_day = 240;
    sim.tick = 239;
    let id = *sim.agents.keys().next().unwrap();
    sim.agents.get_mut(&id).unwrap().needs.energy = 0;
    assert!(sim.tick());
    let energy = sim.agents[&id].needs.energy;
    let bytes = sim.encode_checkpoint().unwrap();
    let mut loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    loaded.time_enabled = true;
    loaded.ticks_per_day = 240;
    assert_eq!(loaded.tick, 240);
    assert_eq!(loaded.agents[&id].needs.energy, energy);
}

#[test]
fn rest_still_adds_regen_when_time_on() {
    let mut sim = Simulation::new(tiny(0x52_13)).unwrap();
    sim.time_enabled = true;
    let id = *sim.agents.keys().next().unwrap();
    let regen = sim.config.energy_regen_milli();
    let max = sim.agents[&id]
        .sheet
        .energy_max(sim.config.energy_max_milli());
    let start = max / 2;
    sim.agents.get_mut(&id).unwrap().needs.energy = start;
    sim_core::execute::execute_primary(&mut sim, id, &PrimaryAction::Rest);
    let after = sim.agents[&id].needs.energy;
    assert_eq!(after, (start + regen).min(max));
}
