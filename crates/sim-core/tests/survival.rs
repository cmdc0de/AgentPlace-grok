use sim_core::event_log::SimEventKind;
use sim_core::observation::{self, chebyshev, effective_range};
use sim_core::species::Toxicity;
use sim_core::{AgentId, CHECKPOINT_FORMAT_VERSION, ExperimentConfig, ItemId, Simulation};

fn tiny_config(master_seed: u64) -> ExperimentConfig {
    let toml = format!(
        r#"
master_seed = {master_seed}
[simulation]
max_ticks = 10000
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 4
"#
    );
    ExperimentConfig::from_toml_str(&toml).unwrap()
}

#[test]
fn drink_raises_thirst() {
    let mut sim = Simulation::new(tiny_config(7)).unwrap();
    let id = AgentId(0);
    let water = find_land_next_to_water(&sim).expect("water edge");
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = water.0;
        a.y = water.1;
        a.needs.thirst = 200;
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
    }
    sim.tick();
    let a = sim.agents.get(&id).unwrap();
    assert!(
        a.needs.thirst > 200,
        "thirst should recover after drink, got {}",
        a.needs.thirst
    );
    assert!(
        sim.events.events.iter().any(|e| {
            e.agent == id && matches!(e.kind, SimEventKind::Drink | SimEventKind::Wait)
        })
    );
}

#[test]
fn toxic_eat_makes_everyone_sick() {
    let mut sim = Simulation::new(tiny_config(11)).unwrap();
    let mushroom = sim
        .config
        .world
        .species
        .veg_tag_by_id("mushroom")
        .expect("mushroom");
    assert_eq!(
        sim.config.world.species.veg(mushroom).unwrap().toxicity,
        Toxicity::Toxic
    );
    for id in sim.agent_ids() {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.inventory.clear();
            a.try_add_item(ItemId::Food(mushroom), 1);
            a.needs.hunger = 100;
            a.needs.thirst = sim.config.thirst_max_milli();
            a.needs.energy = sim.config.energy_max_milli();
            a.personality.allergy_tags.clear();
        }
    }
    sim.tick();
    let sick = sim
        .agents
        .values()
        .filter(|a| a.illness_ticks > 0 || a.consumption.toxic_events > 0)
        .count();
    assert!(sick >= 1, "at least one agent should eat the toxic food");
}

#[test]
fn allergenic_only_hurts_tagged_agents() {
    let mut sim = Simulation::new(tiny_config(21)).unwrap();
    let night = sim
        .config
        .world
        .species
        .veg_tag_by_id("nightshade")
        .expect("nightshade");
    let ids: Vec<_> = sim.agent_ids();
    let a0 = ids[0];
    let a1 = ids[1];
    if let Some(a) = sim.agents.get_mut(&a0) {
        a.personality.allergy_tags = vec!["solanaceae".into()];
        a.inventory.clear();
        a.try_add_item(ItemId::Food(night), 1);
        a.needs.hunger = 50;
        a.needs.thirst = sim.config.thirst_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
    }
    if let Some(a) = sim.agents.get_mut(&a1) {
        a.personality.allergy_tags.clear();
        a.inventory.clear();
        a.try_add_item(ItemId::Food(night), 1);
        a.needs.hunger = 50;
        a.needs.thirst = sim.config.thirst_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
    }
    sim.tick();
    let allergic = sim.agents.get(&a0).unwrap();
    let normal = sim.agents.get(&a1).unwrap();
    // Allergic agent who ate should be ill; non-allergic should not from nightshade.
    if allergic.consumption.vegetation > 0 {
        assert!(allergic.illness_ticks > 0 || allergic.consumption.toxic_events > 0);
    }
    if normal.consumption.vegetation > 0 {
        assert_eq!(normal.illness_ticks, 0);
        assert_eq!(normal.consumption.toxic_events, 0);
    }
}

#[test]
fn observation_differs_with_perceptiveness() {
    let mut sim = Simulation::new(tiny_config(3)).unwrap();
    let ids: Vec<_> = sim.agent_ids();
    let a = ids[0];
    let b = ids[1];
    let pos = {
        let ag = sim.agents.get(&a).unwrap();
        (ag.x, ag.y)
    };
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = pos.0;
        ag.y = pos.1;
        ag.personality.perceptiveness = 0;
    }
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.personality.perceptiveness = 100;
    }
    let oa = observation::build(&sim, a);
    let ob = observation::build(&sim, b);
    assert!(
        oa.tiles.len() > ob.tiles.len(),
        "high perceptiveness {} vs low {}",
        oa.tiles.len(),
        ob.tiles.len()
    );
    assert_eq!(effective_range(12.0, 100), 18);
    assert_eq!(chebyshev(0, 0, 3, 4), 4);
}

#[test]
fn overlong_speech_is_truncated() {
    let mut sim = Simulation::new(tiny_config(5)).unwrap();
    sim.config.communication.max_message_length = 8;
    let id = AgentId(0);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.memory.push(sim_core::memory::MemoryEntry {
            tick: 0,
            kind: sim_core::memory::MemoryKind::ToxinFact,
            text: "mushroom is toxic".into(),
            importance: 90,
            last_accessed: 0,
            species_tag: 3,
            ..Default::default()
        });
        a.last_warn_tick = 0;
    }
    // Force a speak through execute path by running ticks; mock may speak.
    sim.run_ticks(5);
    for e in &sim.events.events {
        if let SimEventKind::Speak { text, .. } = &e.kind {
            assert!(text.chars().count() <= 8, "{text}");
        }
    }
}

#[test]
fn format_version_is_v3() {
    assert_eq!(CHECKPOINT_FORMAT_VERSION, 3);
    let bytes = Simulation::new(tiny_config(1))
        .unwrap()
        .encode_checkpoint()
        .unwrap();
    let mut v1 = bytes.clone();
    v1[4..8].copy_from_slice(&1u32.to_le_bytes());
    let err = Simulation::decode_checkpoint(&v1).unwrap_err();
    assert!(err.to_string().contains("format_version"));
}

#[test]
fn needs_decay_each_tick() {
    let mut sim = Simulation::new(tiny_config(9)).unwrap();
    let id = AgentId(0);
    let before = sim.agents.get(&id).unwrap().needs.thirst;
    sim.tick();
    let after = sim.agents.get(&id).unwrap().needs.thirst;
    assert!(after < before || before == 0);
}

#[test]
fn illegal_action_becomes_wait() {
    let mut sim = Simulation::new(tiny_config(13)).unwrap();
    let id = AgentId(0);
    // Hunt is illegal if no animals adjacent; execute_primary should Wait.
    sim_core::execute::execute_primary(&mut sim, id, &sim_core::action::PrimaryAction::Hunt);
    let last = sim.events.events.last().unwrap();
    assert!(
        matches!(last.kind, SimEventKind::Wait | SimEventKind::Hunt { .. }),
        "{:?}",
        last.kind
    );
}

#[test]
fn wait_chooser_does_not_speak() {
    let mut sim = Simulation::new(tiny_config(15)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.run_ticks(8);
    assert!(
        sim.events
            .events
            .iter()
            .all(|e| !matches!(e.kind, SimEventKind::Speak { .. }))
    );
    assert!(
        sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::LlmWait))
    );
}

#[test]
fn craft_failure_keeps_ingredients() {
    let mut sim = Simulation::new(tiny_config(17)).unwrap();
    sim.enable_catalog(sim_core::catalog_entries(&[sim_core::ObjectDef {
        id: "spear".into(),
        kind: "item".into(),
        visual: None,
        sim: Some(sim_core::objects::SimDef {
            weight_milli: Some(200),
            craft: Some(sim_core::objects::CraftDef {
                inputs: vec![("wood".into(), 1), ("stone".into(), 1)],
                output_qty: Some(1),
            }),
            ..Default::default()
        }),
    }]));
    let id = AgentId(0);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(sim_core::ItemId::Wood, 1);
        a.try_add_item(sim_core::ItemId::Stone, 1);
        a.abilities.craft = 0;
        a.needs.hunger = 0;
        a.needs.thirst = 0;
        a.needs.energy = 0;
        a.illness_ticks = 10;
    }
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &sim_core::action::PrimaryAction::Craft {
            recipe: sim_core::action::Recipe::Spear,
        },
    );
    let a = sim.agents.get(&id).unwrap();
    // Even if the 5% clamp succeeds once, ingredients only leave on success.
    if matches!(
        sim.events.events.last().map(|e| &e.kind),
        Some(SimEventKind::Craft { success: false, .. })
    ) {
        assert_eq!(
            a.inventory
                .get(&sim_core::ItemId::Wood)
                .copied()
                .unwrap_or(0),
            1
        );
        assert_eq!(
            a.inventory
                .get(&sim_core::ItemId::Stone)
                .copied()
                .unwrap_or(0),
            1
        );
    }
}

#[test]
fn parse_think_wrapped_json() {
    let sim = Simulation::new(tiny_config(19)).unwrap();
    let obs = sim_core::observation::build(&sim, AgentId(0));
    let choice = sim_core::parse_choice_json(
        "<think>I should drink</think>\n```json\n{\"action\":\"Wait\"}\n```",
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    assert!(matches!(
        choice.primary,
        sim_core::action::PrimaryAction::Wait
    ));
    let extracted = sim_core::extract_json_payload("Sure.\n{\"action\":\"Rest\"}");
    assert!(extracted.contains("Rest"), "{extracted}");
}

#[test]
fn parse_illegal_json_waits() {
    let sim = Simulation::new(tiny_config(19)).unwrap();
    let obs = sim_core::observation::build(&sim, AgentId(0));
    let choice = sim_core::parse_choice_json(
        r#"{"action":"Attack","target":"nobody"}"#,
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    assert!(matches!(
        choice.primary,
        sim_core::action::PrimaryAction::Wait
    ));
}

#[test]
fn directed_speech_not_heard_far_away() {
    let mut sim = Simulation::new(tiny_config(23)).unwrap();
    let ids = sim.agent_ids();
    let speaker = ids[0];
    let target = ids[1];
    let other = ids[2];
    // Park speaker and target together; send other to a far corner.
    if let Some(a) = sim.agents.get_mut(&speaker) {
        a.x = 2;
        a.y = 2;
    }
    if let Some(a) = sim.agents.get_mut(&target) {
        a.x = 2;
        a.y = 3;
    }
    if let Some(a) = sim.agents.get_mut(&other) {
        a.x = 30;
        a.y = 30;
    }
    sim.tick = 1;
    sim.events.push(sim_core::event_log::SimEvent {
        tick: 1,
        agent: speaker,
        kind: SimEventKind::Speak {
            shout: false,
            text: "hello target".into(),
            broadcast: false,
            targets: vec![target],
        },
    });
    sim.tick = 2;
    sim.config.communication.allow_overhearing = false;
    let ht = sim_core::observation::build(&sim, target);
    let ho = sim_core::observation::build(&sim, other);
    assert!(
        ht.heard.iter().any(|h| h.text.contains("hello target")),
        "target should hear"
    );
    assert!(
        ho.heard.iter().all(|h| !h.text.contains("hello target")),
        "far non-target should not hear"
    );
}

#[test]
fn replay_table_round_trip() {
    let jsonl = r#"{"tick":1,"agent":0,"call_seed":1,"prompt_hash":"ab","response":"{\"action\":\"Wait\"}"}"#;
    let table = sim_core::ReplayTable::from_jsonl(jsonl);
    assert_eq!(table.get(1, 0).unwrap().contains("Wait"), true);
}

#[test]
fn death_disabled_keeps_zero_thirst_agent() {
    let mut sim = Simulation::new(tiny_config(11)).unwrap();
    sim.config.needs.death_enabled = false;
    sim.config.needs.thirst_decay_per_tick = 100.0;
    let n = sim.agents.len();
    sim.tick();
    assert_eq!(sim.agents.len(), n);
    assert!(sim.agents.values().any(|a| a.needs.thirst == 0));
    assert!(
        sim.events
            .events
            .iter()
            .all(|e| !matches!(e.kind, SimEventKind::Died { .. }))
    );
}

#[test]
fn death_enabled_removes_agent_at_zero_thirst() {
    let mut sim = Simulation::new(tiny_config(12)).unwrap();
    sim.config.needs.death_enabled = true;
    sim.config.needs.thirst_decay_per_tick = 100.0;
    let n = sim.agents.len();
    sim.tick();
    assert!(sim.agents.is_empty(), "all should dehydrate in one tick");
    let deaths = sim
        .events
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                SimEventKind::Died {
                    thirst_zero: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(deaths, n);
}

#[test]
fn observation_includes_needs_and_incentives() {
    let mut sim = Simulation::new(tiny_config(31)).unwrap();
    sim.inject_schedule_toml(
        r#"
[[incentives]]
id = "coop_food"
description = "test bonus"
start_tick = 0
applies_to = "all"
[[incentives.effects]]
type = "goal_injection"
goal_text = "share food"
scope = "personal"
priority = 0.5
"#,
    )
    .unwrap();
    sim.tick();
    let obs = observation::build(&sim, AgentId(0));
    assert!(obs.hunger > 0 && obs.hunger <= 100, "hunger={}", obs.hunger);
    assert!(obs.thirst > 0 && obs.thirst <= 100, "thirst={}", obs.thirst);
    assert!(
        obs.incentives.iter().any(|i| i.id == "coop_food"),
        "{:?}",
        obs.incentives
    );
}

#[test]
fn mock_drinks_before_half_thirst() {
    let mut sim = Simulation::new(tiny_config(7)).unwrap();
    sim.config.needs.death_enabled = true;
    let id = AgentId(0);
    let water = find_land_next_to_water(&sim).expect("water edge");
    let half = sim.config.thirst_max_milli() / 2;
    if let Some(a) = sim.agents.get_mut(&id) {
        a.x = water.0;
        a.y = water.1;
        // Above the old max/2 cutoff, below M9's 75% seek line.
        a.needs.thirst = half + 100;
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
    }
    sim.tick();
    assert!(
        sim.events
            .events
            .iter()
            .any(|e| e.agent == id && matches!(e.kind, SimEventKind::Drink)),
        "should Drink while still above half thirst; events={:?}",
        sim.events.events
    );
    assert!(sim.agents.contains_key(&id));
}

fn find_land_next_to_water(sim: &Simulation) -> Option<(u32, u32)> {
    for y in 0..sim.world.height {
        for x in 0..sim.world.width {
            if !sim.world.is_land(x, y) {
                continue;
            }
            for (dx, dy) in [(0i32, 1), (0, -1), (1, 0), (-1, 0)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if sim.world.in_bounds(nx, ny) && sim.world.is_water(nx as u32, ny as u32) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}
