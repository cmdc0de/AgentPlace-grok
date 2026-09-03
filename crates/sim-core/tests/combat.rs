//! M27 combat overlay + Attack/Flee.

use sim_core::action::PrimaryAction;
use sim_core::conflict::{ATTACK_DAMAGE, ATTACK_ENERGY_COST, ConflictParams};
use sim_core::event_log::SimEventKind;
use sim_core::observation::legal_actions;
use sim_core::{AgentId, ExperimentConfig, Simulation};

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

fn place_adjacent(sim: &mut Simulation) -> (AgentId, AgentId) {
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let a = ids[0];
    let b = ids[1];
    let (x, y) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    let mut nx = x.saturating_add(1);
    let mut ny = y;
    if !sim.world.in_bounds(nx as i32, ny as i32) || !sim.world.is_land(nx, ny) {
        nx = x.saturating_sub(1);
    }
    if !sim.world.is_land(nx, ny) {
        ny = y.saturating_add(1);
        nx = x;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = nx;
        ag.y = ny;
    }
    (a, b)
}

#[test]
fn overlay_parses_conflict() {
    let p = ConflictParams::from_config_toml("[conflict]\nenabled = true\n");
    assert!(p.enabled);
    let d = ConflictParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.enabled);
}

#[test]
fn mock_conflict_overlay_same_hash() {
    let cfg = tiny(0x27_20);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.conflict_enabled = true;
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn overlay_off_attack_not_legal() {
    let mut sim = Simulation::new(tiny(0x27_21)).unwrap();
    let (a, _) = place_adjacent(&mut sim);
    let agent = sim.agents.get(&a).unwrap().clone();
    let legal = legal_actions(&sim, &agent);
    assert!(
        !legal.iter().any(|x| matches!(x, PrimaryAction::Attack { .. })),
        "{legal:?}"
    );
    assert!(!legal.iter().any(|x| matches!(x, PrimaryAction::Flee)));
}

#[test]
fn custom_attack_adjacent_drops_energy() {
    let mut sim = Simulation::new(tiny(0x27_22)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    let before = sim.agents.get(&b).unwrap().needs.energy;
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let after = sim.agents.get(&b).unwrap().needs.energy;
    assert_eq!(after, before.saturating_sub(ATTACK_DAMAGE));
    assert!(
        sim.events.events.iter().any(|e| matches!(
            e.kind,
            SimEventKind::Attack { target, damage }
            if target == b && damage == ATTACK_DAMAGE
        )),
        "{:?}",
        sim.events.events
    );
    let _ = ATTACK_ENERGY_COST;
}

#[test]
fn flee_moves_away_or_waits() {
    let mut sim = Simulation::new(tiny(0x27_23)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    let (ax, ay) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    let (bx, by) = {
        let ag = sim.agents.get(&b).unwrap();
        (ag.x, ag.y)
    };
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Flee);
    let ag = sim.agents.get(&a).unwrap();
    let moved = ag.x != ax || ag.y != ay;
    let waited = sim
        .events
        .events
        .iter()
        .any(|e| e.agent == a && matches!(e.kind, SimEventKind::Wait));
    let fled = sim
        .events
        .events
        .iter()
        .any(|e| e.agent == a && matches!(e.kind, SimEventKind::Flee));
    assert!(moved || waited, "pos ({},{}) vs start ({ax},{ay}) vs other ({bx},{by})", ag.x, ag.y);
    assert!(fled || waited, "{:?}", sim.events.events);
}
