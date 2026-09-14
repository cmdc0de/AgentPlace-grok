//! M27 combat overlay + Attack/Flee.

use sim_core::action::PrimaryAction;
use sim_core::combat_fx::{
    CombatFxJob, CombatRole, combat_fx_jobs, combat_hud_line, combat_hud_lines, combat_role,
};
use sim_core::conflict::{ATTACK_DAMAGE, ATTACK_ENERGY_COST, ConflictParams};
use sim_core::event_log::{SimEvent, SimEventKind};
use sim_core::observation::legal_actions;
use sim_core::{AgentId, ExperimentConfig, HEALTH_MAX, Simulation};

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
    assert!(!p.death_enabled);
    let d = ConflictParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.enabled);
    assert!(!d.death_enabled);
}

#[test]
fn overlay_parses_conflict_death() {
    let p = ConflictParams::from_config_toml("[conflict]\nenabled = true\ndeath_enabled = true\n");
    assert!(p.enabled);
    assert!(p.death_enabled);
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
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::Attack { .. })),
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
    assert!(
        moved || waited,
        "pos ({},{}) vs start ({ax},{ay}) vs other ({bx},{by})",
        ag.x,
        ag.y
    );
    assert!(fled || waited, "{:?}", sim.events.events);
}

#[test]
fn attack_drops_health_and_energy() {
    let mut sim = Simulation::new(tiny(0x28_01)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    let before_e = sim.agents.get(&b).unwrap().needs.energy;
    let before_h = sim.agents.get(&b).unwrap().health;
    assert_eq!(before_h, HEALTH_MAX);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let def = sim.agents.get(&b).unwrap();
    assert_eq!(def.needs.energy, before_e.saturating_sub(ATTACK_DAMAGE));
    assert_eq!(def.health, before_h.saturating_sub(ATTACK_DAMAGE));
    let _ = ATTACK_ENERGY_COST;
}

#[test]
fn health_zero_incapacitates() {
    let mut sim = Simulation::new(tiny(0x28_02)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim.agents.get_mut(&b).unwrap().health = ATTACK_DAMAGE;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let def = sim.agents.get(&b).unwrap();
    assert_eq!(def.health, 0);
    assert!(def.incapacitated);
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Incapacitated { by } if by == a
    )));
    sim.tick();
    let agent = sim.agents.get(&b).unwrap().clone();
    let legal = legal_actions(&sim, &agent);
    assert!(
        !legal
            .iter()
            .any(|x| matches!(x, PrimaryAction::Attack { .. })),
        "{legal:?}"
    );
    let atk = sim.agents.get(&a).unwrap().clone();
    let al = legal_actions(&sim, &atk);
    assert!(
        !al.iter()
            .any(|x| matches!(x, PrimaryAction::Attack { target } if *target == b)),
        "{al:?}"
    );
}

#[test]
fn combat_role_helper_distinct() {
    let a = AgentId(0);
    let b = AgentId(1);
    let events = vec![SimEvent {
        tick: 3,
        agent: a,
        kind: SimEventKind::Attack {
            target: b,
            damage: 1,
        },
    }];
    assert_eq!(combat_role(&events, 3, a), CombatRole::Attacker);
    assert_eq!(combat_role(&events, 3, b), CombatRole::Defender);
    assert_eq!(combat_role(&events, 3, AgentId(9)), CombatRole::None);
    let line = combat_hud_line(&events, 3).unwrap();
    assert!(line.contains("attack"), "{line}");
    assert_ne!(
        format!("{:?}", CombatRole::Attacker),
        format!("{:?}", CombatRole::Defender)
    );
}

#[test]
fn combat_fx_jobs_empty_without_events() {
    assert!(combat_fx_jobs(&[], 1).is_empty());
    let events = vec![SimEvent {
        tick: 2,
        agent: AgentId(0),
        kind: SimEventKind::Wait,
    }];
    assert!(combat_fx_jobs(&events, 2).is_empty());
    assert!(combat_fx_jobs(&events, 1).is_empty());
}

#[test]
fn combat_fx_jobs_from_attack_flee_downed_death() {
    let a = AgentId(0);
    let b = AgentId(1);
    let events = vec![
        SimEvent {
            tick: 4,
            agent: a,
            kind: SimEventKind::Attack {
                target: b,
                damage: 2000,
            },
        },
        SimEvent {
            tick: 4,
            agent: a,
            kind: SimEventKind::Flee,
        },
        SimEvent {
            tick: 4,
            agent: b,
            kind: SimEventKind::Incapacitated { by: a },
        },
        SimEvent {
            tick: 4,
            agent: b,
            kind: SimEventKind::CombatDeath { by: a },
        },
        SimEvent {
            tick: 5,
            agent: a,
            kind: SimEventKind::Attack {
                target: b,
                damage: 1,
            },
        },
    ];
    let jobs = combat_fx_jobs(&events, 4);
    assert_eq!(
        jobs,
        vec![
            CombatFxJob::Strike { from: a, to: b },
            CombatFxJob::Flee { agent: a },
            CombatFxJob::Downed { agent: b },
            CombatFxJob::Death { agent: b },
        ]
    );
    let later = combat_fx_jobs(&events, 5);
    assert_eq!(later, vec![CombatFxJob::Strike { from: a, to: b }]);
    assert!(!later.iter().any(|j| matches!(
        j,
        CombatFxJob::Flee { .. } | CombatFxJob::Downed { .. } | CombatFxJob::Death { .. }
    )));
}

#[test]
fn combat_hud_includes_incapacitate_and_death() {
    let a = AgentId(0);
    let b = AgentId(1);
    let events = vec![
        SimEvent {
            tick: 7,
            agent: b,
            kind: SimEventKind::Incapacitated { by: a },
        },
        SimEvent {
            tick: 8,
            agent: b,
            kind: SimEventKind::CombatDeath { by: a },
        },
    ];
    let down = combat_hud_line(&events, 7).unwrap();
    assert!(down.contains("incapacitated"), "{down}");
    assert!(down.contains("#1"), "{down}");
    let dead = combat_hud_lines(&events, 8);
    assert_eq!(dead.len(), 1);
    assert!(dead[0].contains("combat_death"), "{dead:?}");
    assert!(combat_hud_line(&events, 9).is_none());
}

#[test]
fn mock_conflict_death_overlay_no_attack_same_hash() {
    let cfg = tiny(0x29_10);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.conflict_enabled = true;
    on.conflict_death_enabled = true;
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn health_zero_combat_death_removes() {
    let mut sim = Simulation::new(tiny(0x29_11)).unwrap();
    sim.conflict_enabled = true;
    sim.conflict_death_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim.agents.get_mut(&b).unwrap().health = ATTACK_DAMAGE;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    assert!(sim.agents.get(&b).is_none(), "combat death must remove");
    assert!(sim.agents.get(&a).is_some());
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::CombatDeath { by } if by == a
    )));
    assert!(
        !sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::Incapacitated { .. })),
        "{:?}",
        sim.events.events
    );
}

#[test]
fn checkpoint_round_trip_default_health() {
    let mut sim = Simulation::new(tiny(0x28_03)).unwrap();
    sim.run_ticks(4);
    let restored = Simulation::decode_checkpoint(&sim.encode_checkpoint().unwrap()).unwrap();
    assert!(restored.agents.values().all(|a| a.health == HEALTH_MAX));
    assert!(restored.agents.values().all(|a| !a.incapacitated));
    assert_eq!(restored.state_hash(), sim.state_hash());
}

fn shipped_objects() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/objects")
}

fn give_slug(sim: &mut Simulation, id: AgentId, slug: &str) {
    let item = sim
        .catalog
        .iter()
        .find(|e| e.slug == slug)
        .unwrap_or_else(|| panic!("{slug}"))
        .item;
    let added = sim.agents.get_mut(&id).unwrap().try_add_item(item, 1);
    assert_eq!(added, 1, "give {slug}");
}

#[test]
fn attack_no_weapon_is_base_damage() {
    let mut sim = Simulation::new(tiny(0x51_20)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE));
}

#[test]
fn attack_hold_pike_adds_800() {
    let mut sim = Simulation::new(tiny(0x51_21)).unwrap();
    sim.conflict_enabled = true;
    sim.apply_objects_dir(&shipped_objects()).unwrap();
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    give_slug(&mut sim, a, "pike");
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE + 800));
}

#[test]
fn attack_two_weapons_uses_max_not_sum() {
    let mut sim = Simulation::new(tiny(0x51_22)).unwrap();
    sim.conflict_enabled = true;
    sim.apply_objects_dir(&shipped_objects()).unwrap();
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    give_slug(&mut sim, a, "club");
    give_slug(&mut sim, a, "pike");
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE + 800), "max pike 800 not 300+800");
}

#[test]
fn attack_catalog_off_weapon_no_bonus() {
    let mut sim = Simulation::new(tiny(0x51_23)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE));
}

#[test]
fn attack_hold_spear_adds_500() {
    let mut sim = Simulation::new(tiny(0x54_40)).unwrap();
    sim.conflict_enabled = true;
    sim.apply_objects_dir(&shipped_objects()).unwrap();
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim.agents
        .get_mut(&a)
        .unwrap()
        .try_add_item(sim_core::ItemId::Spear, 1);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE + 500));
}

#[test]
fn attack_catalog_off_spear_is_base_damage() {
    let mut sim = Simulation::new(tiny(0x54_41)).unwrap();
    sim.conflict_enabled = true;
    let (a, b) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    sim.agents
        .get_mut(&a)
        .unwrap()
        .try_add_item(sim_core::ItemId::Spear, 1);
    sim_core::execute::execute_primary(&mut sim, a, &PrimaryAction::Attack { target: b });
    let dmg = sim.events.events.iter().find_map(|e| match e.kind {
        SimEventKind::Attack { target, damage } if target == b => Some(damage),
        _ => None,
    });
    assert_eq!(dmg, Some(ATTACK_DAMAGE));
}

fn park_at_chebyshev(sim: &mut Simulation, a: AgentId, b: AgentId, dist: u32) {
    let (x, y) = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    let mut nx = x.saturating_add(dist);
    let mut ny = y;
    if !sim.world.in_bounds(nx as i32, ny as i32) || !sim.world.is_land(nx, ny) {
        nx = x;
        ny = y.saturating_add(dist);
    }
    if !sim.world.is_land(nx, ny) {
        nx = x.saturating_sub(dist);
        ny = y;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = nx;
        ag.y = ny;
    }
}

#[test]
fn bow_attack_legal_at_chebyshev_3_not_4() {
    let mut sim = Simulation::new(tiny(0x54_42)).unwrap();
    sim.conflict_enabled = true;
    sim.apply_objects_dir(&shipped_objects()).unwrap();
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let a = ids[0];
    let b = ids[1];
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    give_slug(&mut sim, a, "bow");
    park_at_chebyshev(&mut sim, a, b, 3);
    let d = sim_core::observation::chebyshev(
        sim.agents[&a].x,
        sim.agents[&a].y,
        sim.agents[&b].x,
        sim.agents[&b].y,
    );
    assert_eq!(d, 3);
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Attack { target } if *target == b)));
    park_at_chebyshev(&mut sim, a, b, 4);
    let d4 = sim_core::observation::chebyshev(
        sim.agents[&a].x,
        sim.agents[&a].y,
        sim.agents[&b].x,
        sim.agents[&b].y,
    );
    assert_eq!(d4, 4);
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Attack { target } if *target == b)));
}

#[test]
fn unarmed_and_club_attack_only_adjacent() {
    let mut sim = Simulation::new(tiny(0x54_43)).unwrap();
    sim.conflict_enabled = true;
    sim.apply_objects_dir(&shipped_objects()).unwrap();
    let ids: Vec<AgentId> = sim.agents.keys().copied().collect();
    let a = ids[0];
    let b = ids[1];
    sim.agents.get_mut(&a).unwrap().needs.energy = 10_000;
    park_at_chebyshev(&mut sim, a, b, 2);
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Attack { .. })));
    give_slug(&mut sim, a, "club");
    let legal = legal_actions(&sim, sim.agents.get(&a).unwrap());
    assert!(!legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Attack { .. })));
    let (a2, b2) = place_adjacent(&mut sim);
    sim.agents.get_mut(&a2).unwrap().needs.energy = 10_000;
    let legal = legal_actions(&sim, sim.agents.get(&a2).unwrap());
    assert!(legal
        .iter()
        .any(|x| matches!(x, PrimaryAction::Attack { target } if *target == b2)));
}
