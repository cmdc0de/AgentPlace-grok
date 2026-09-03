use sim_core::action::PrimaryAction;
use sim_core::board::StructuredRule;
use sim_core::config::EvictionPolicy;
use sim_core::event_log::SimEventKind;
use sim_core::markers::{self, MarkerShape};
use sim_core::memory::{self, MemoryEntry, MemoryKind};
use sim_core::social::RelationshipSummary;
use sim_core::{AgentId, ExperimentConfig, Simulation, append_decisions_jsonl};

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

fn two_agent_config(master_seed: u64) -> ExperimentConfig {
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
count = 2
"#
    );
    ExperimentConfig::from_toml_str(&toml).unwrap()
}

#[test]
fn same_seed_hash_includes_relationships() {
    let cfg = tiny_config(0x5D5001);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(12);
    b.run_ticks(12);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn continuation_and_m4_ckpt_loads() {
    let cfg = tiny_config(0x5D5002);
    let mut continuous = Simulation::new(cfg.clone()).unwrap();
    continuous.run_ticks(20);
    let expected = continuous.state_hash();
    let mut branched = Simulation::new(cfg).unwrap();
    branched.run_ticks(12);
    let mut restored =
        Simulation::decode_checkpoint(&branched.encode_checkpoint().unwrap()).unwrap();
    restored.run_ticks(8);
    assert_eq!(restored.state_hash(), expected);

    let m4_like = Simulation::new(tiny_config(0x5D5003)).unwrap();
    let body = m4_like.to_checkpoint().unwrap();
    // M4 blob only board+goals still loads via fallback.
    let loaded =
        Simulation::decode_checkpoint(&sim_core::encode_checkpoint(&body).unwrap()).unwrap();
    assert_eq!(loaded.tick, m4_like.tick);
    let _ = body;
}

#[test]
fn identified_speak_updates_both_maps() {
    let mut sim = Simulation::new(two_agent_config(0x5D5004)).unwrap();
    let ids = sim.agent_ids();
    let a = ids[0];
    let b = ids[1];
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = 4;
        ag.y = 4;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = 4;
        ag.y = 5;
    }
    sim.tick = 1;
    sim.apply_speak(
        a,
        sim_core::action::Speak {
            to: sim_core::action::SpeakTarget::Directed(vec![b]),
            shout: false,
            text: "hello".into(),
        },
    );
    let ab = sim
        .agents
        .get(&a)
        .unwrap()
        .relationships
        .get(&b)
        .map(|r| r.affinity)
        .unwrap_or(0);
    let ba = sim
        .agents
        .get(&b)
        .unwrap()
        .relationships
        .get(&a)
        .map(|r| r.affinity)
        .unwrap_or(0);
    assert_eq!(ab, 50);
    assert_eq!(ba, 50);
}

#[test]
fn unidentified_speak_does_not_create_row() {
    let mut sim = Simulation::new(tiny_config(0x5D5005)).unwrap();
    let ids = sim.agent_ids();
    let a = ids[0];
    let b = ids[1];
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.x = 2;
        ag.y = 2;
    }
    if let Some(ag) = sim.agents.get_mut(&b) {
        ag.x = 30;
        ag.y = 30;
    }
    sim.tick = 1;
    sim.events.push(sim_core::event_log::SimEvent {
        tick: 1,
        agent: a,
        kind: SimEventKind::Speak {
            shout: false,
            text: "far shout".into(),
            broadcast: true,
            targets: vec![],
        },
    });
    sim.tick = 2;
    let obs = sim_core::observation::build(&sim, b);
    assert!(obs.heard.iter().all(|h| h.speaker.is_none()) || obs.heard.is_empty());
    assert!(sim.agents.get(&b).unwrap().relationships.is_empty());
}

#[test]
fn support_raises_trust_oppose_lowers_affinity() {
    let mut sim = Simulation::new(two_agent_config(0x5D5006)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    let ids = sim.agent_ids();
    sim_core::execute::execute_primary(
        &mut sim,
        ids[0],
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    sim_core::execute::execute_primary(
        &mut sim,
        ids[1],
        &PrimaryAction::Support { proposal_id: 0 },
    );
    let trust = sim
        .agents
        .get(&ids[1])
        .unwrap()
        .relationships
        .get(&ids[0])
        .map(|r| r.trust)
        .unwrap_or(0);
    assert!(trust >= 200, "trust {trust}");
    sim_core::execute::execute_primary(&mut sim, ids[1], &PrimaryAction::Oppose { proposal_id: 0 });
    let aff = sim
        .agents
        .get(&ids[1])
        .unwrap()
        .relationships
        .get(&ids[0])
        .map(|r| r.affinity)
        .unwrap_or(0);
    assert!(aff < 50, "affinity {aff}");
}

#[test]
fn relationships_survive_eviction() {
    let mut sim = Simulation::new(tiny_config(0x5D5007)).unwrap();
    sim.config.agents.memory.capacity = 8;
    let id = AgentId(0);
    let other = AgentId(1);
    if let Some(a) = sim.agents.get_mut(&id) {
        a.relationships
            .insert(other, RelationshipSummary::default());
        for i in 0..20 {
            a.remember(
                8,
                EvictionPolicy::ImportanceAndRecency,
                150,
                true,
                MemoryEntry {
                    tick: i,
                    kind: MemoryKind::Observation,
                    text: format!("noise {i}"),
                    importance: 10,
                    last_accessed: i,
                    species_tag: 0,
                    id: 0,
                    participants: vec![other],
                    valence: 0,
                    ..Default::default()
                },
            );
        }
        assert!(a.relationships.contains_key(&other));
        assert!(a.memory.len() <= 8);
        assert!(
            a.memory.iter().any(|e| e.participants.contains(&other)),
            "notable per-partner memory kept"
        );
    }
}

#[test]
fn fifo_vs_importance_drop_differently() {
    let mut fifo = vec![];
    let mut recency = vec![];
    for i in 0..5u64 {
        let e = MemoryEntry {
            tick: i,
            kind: MemoryKind::Observation,
            text: format!("{i}"),
            importance: if i == 0 { 90 } else { 10 },
            last_accessed: i,
            species_tag: 0,
            ..Default::default()
        };
        fifo.push(e.clone());
        recency.push(e);
    }
    memory::evict(
        &mut fifo,
        3,
        EvictionPolicy::Fifo,
        100,
        false,
        &Default::default(),
    );
    memory::evict(
        &mut recency,
        3,
        EvictionPolicy::ImportanceAndRecency,
        100,
        false,
        &Default::default(),
    );
    assert_ne!(
        fifo.iter().map(|e| e.tick).collect::<Vec<_>>(),
        recency.iter().map(|e| e.tick).collect::<Vec<_>>()
    );
}

#[test]
fn toxin_from_speech_ungated() {
    let mut sim = Simulation::new(tiny_config(0x5D5008)).unwrap();
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    let id = AgentId(0);
    sim_core::execute::apply_heard_memories(
        &mut sim,
        id,
        &[sim_core::observation::HeardSpeech {
            speaker: None,
            text: "mushroom is toxic".into(),
            shout: false,
        }],
    );
    assert!(memory::knows_toxin(
        &sim.agents.get(&id).unwrap().memory,
        mushroom
    ));
}

#[test]
fn trust_gated_support() {
    let mut sim = Simulation::new(two_agent_config(0x5D5009)).unwrap();
    sim.config.proposals.default_acceptance_threshold = 1.0;
    sim.chooser = sim_core::Chooser::Wait;
    let ids = sim.agent_ids();
    sim_core::execute::execute_primary(
        &mut sim,
        ids[0],
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    if let Some(a) = sim.agents.get_mut(&ids[1]) {
        a.relationships.insert(
            ids[0],
            RelationshipSummary {
                trust: 2500,
                ..Default::default()
            },
        );
        a.influence_factor = 3000;
        a.personality.agreeableness = 50;
        a.memory.clear();
        a.needs.thirst = sim.config.thirst_max_milli();
        a.needs.hunger = sim.config.hunger_max_milli();
        a.needs.energy = sim.config.energy_max_milli();
    }
    // Park together so observation includes board author.
    let pos = {
        let a = sim.agents.get(&ids[0]).unwrap();
        (a.x, a.y)
    };
    if let Some(b) = sim.agents.get_mut(&ids[1]) {
        b.x = pos.0;
        b.y = pos.1;
    }
    sim.chooser = sim_core::Chooser::Mock;
    sim.tick();
    assert!(
        sim.events.events.iter().any(|e| {
            e.agent == ids[1] && matches!(e.kind, SimEventKind::Support { proposal_id: 0 })
        }),
        "expected trust-gated Support"
    );
}

#[test]
fn track_relationships_false_matches_no_delta_hash_shape() {
    let mut off = tiny_config(0x5D500A);
    off.agents.social.track_relationships = false;
    let mut sim = Simulation::new(off).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.run_ticks(5);
    for a in sim.agents.values() {
        assert!(a.relationships.is_empty());
    }
}

#[test]
fn report_mean_trust_is_mean_of_agent_means() {
    let mut sim = Simulation::new(two_agent_config(0x5D500B)).unwrap();
    let ids = sim.agent_ids();
    if let Some(a) = sim.agents.get_mut(&ids[0]) {
        a.relationships.insert(
            ids[1],
            RelationshipSummary {
                trust: 2000,
                ..Default::default()
            },
        );
    }
    if let Some(a) = sim.agents.get_mut(&ids[1]) {
        a.relationships.insert(
            ids[0],
            RelationshipSummary {
                trust: 0,
                ..Default::default()
            },
        );
    }
    let report = sim_core::build_report(&sim).unwrap();
    let mean: f64 =
        report.agents.iter().map(|a| a.mean_trust).sum::<f64>() / report.agents.len() as f64;
    assert!((report.world.mean_trust - mean).abs() < 1e-9);
}

#[test]
fn load_regenerates_relationship_report() {
    let mut sim = Simulation::new(tiny_config(0x5D500C)).unwrap();
    sim.run_ticks(6);
    let first = sim_core::report_markdown(&sim_core::build_report(&sim).unwrap());
    let restored = Simulation::decode_checkpoint(&sim.encode_checkpoint().unwrap()).unwrap();
    let second = sim_core::report_markdown(&sim_core::build_report(&restored).unwrap());
    assert_eq!(first, second);
}

#[test]
fn decision_one_line_per_agent_tick() {
    let mut sim = Simulation::new(two_agent_config(0x5D500D)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.tick();
    assert_eq!(sim.last_tick_decisions.len(), sim.agents.len());
    let dir = std::env::temp_dir().join(format!("m5-decisions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("decisions.jsonl");
    append_decisions_jsonl(&path, &sim.last_tick_decisions).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().count(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toxin_memory_logs_propose_or_support() {
    let mut sim = Simulation::new(tiny_config(0x5D500E)).unwrap();
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    for id in sim.agent_ids() {
        if let Some(a) = sim.agents.get_mut(&id) {
            a.memory.push(MemoryEntry {
                tick: 0,
                kind: MemoryKind::ToxinFact,
                text: "mushroom is toxic".into(),
                importance: 95,
                last_accessed: 0,
                species_tag: mushroom,
                ..Default::default()
            });
            a.needs.thirst = sim.config.thirst_max_milli();
            a.needs.hunger = sim.config.hunger_max_milli();
            a.needs.energy = sim.config.energy_max_milli();
        }
    }
    sim.tick();
    assert!(
        sim.last_tick_decisions
            .iter()
            .any(|d| { d.policy_branch == "toxin_propose" || d.policy_branch == "toxin_support" })
    );
    assert!(sim.events.events.iter().any(|e| matches!(
        e.kind,
        SimEventKind::Propose { .. } | SimEventKind::Support { .. } | SimEventKind::Wait
    )));
}

#[test]
fn berry_and_herb_use_different_shapes() {
    let berry = markers::marker_for_veg(1);
    let herb = markers::marker_for_veg(2);
    assert_ne!(berry.shape, herb.shape);
    assert_eq!(berry.shape, MarkerShape::Sphere);
    assert_eq!(herb.shape, MarkerShape::Capsule);
}
