use sim_core::action::PrimaryAction;
use sim_core::board::{AdoptedRule, ProposalStatus, StructuredRule};
use sim_core::event_log::SimEventKind;
use sim_core::memory::{MemoryEntry, MemoryKind};
use sim_core::{
    AgentId, CouncilTally, ExperimentConfig, ItemId, Simulation, VoteAccept, VoteWeight,
    build_report, observation, report_markdown,
};

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

fn three_agent_config(master_seed: u64) -> ExperimentConfig {
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
count = 3
[proposals]
default_acceptance_threshold = 0.5
"#
    );
    ExperimentConfig::from_toml_str(&toml).unwrap()
}

const KINGMAKER: &str = r#"
[[incentives]]
id = "kingmaker"
description = "boost agent 0 influence"
applies_to = "agent:0"
[[incentives.effects]]
type = "influence_factor_delta"
delta = 70.0
"#;

const ESTEEM: &str = r#"
[[incentives]]
id = "esteem_0"
description = "everyone respects agent 0"
applies_to = "all"
[[incentives.effects]]
type = "relationship_delta"
respect = 70.0
toward = "agent:0"
"#;

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
fn same_seed_hash_includes_board_and_goals() {
    let cfg = tiny_config(0x4D4001);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.run_ticks(20);
    b.run_ticks(20);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(!a.agents.values().next().unwrap().goals.is_empty());
}

#[test]
fn propose_appears_on_board_and_in_every_observation() {
    let mut sim = Simulation::new(tiny_config(0x4D4002)).unwrap();
    let ids = sim.agent_ids();
    sim_core::execute::execute_primary(
        &mut sim,
        ids[0],
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    assert_eq!(sim.board.proposals.len(), 1);
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
    for id in ids {
        let obs = observation::build(&sim, id);
        assert!(
            obs.board
                .iter()
                .any(|p| p.id == 0 && p.text.contains("mushroom")),
            "agent {} missing board",
            id.0
        );
        assert!(
            obs.legal
                .iter()
                .any(|a| matches!(a, PrimaryAction::Support { proposal_id: 0 }))
        );
    }
}

#[test]
fn support_oppose_last_stance_wins() {
    let mut sim = Simulation::new(tiny_config(0x4D4003)).unwrap();
    let ids = sim.agent_ids();
    sim_core::execute::execute_primary(
        &mut sim,
        ids[0],
        &PrimaryAction::Propose {
            text: "cap gather".into(),
            rule: Some(StructuredRule::MaxGatherPerTick { n: 1 }),
        },
    );
    sim_core::execute::execute_primary(
        &mut sim,
        ids[1],
        &PrimaryAction::Support { proposal_id: 0 },
    );
    assert!(sim.board.proposals[0].supporters.contains(&ids[1]));
    sim_core::execute::execute_primary(&mut sim, ids[1], &PrimaryAction::Oppose { proposal_id: 0 });
    assert!(!sim.board.proposals[0].supporters.contains(&ids[1]));
    assert!(sim.board.proposals[0].opposers.contains(&ids[1]));
}

#[test]
fn majority_accept_copies_adopted_rule() {
    let mut sim = Simulation::new(two_agent_config(0x4D4004)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    let id = AgentId(0);
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
    assert_eq!(sim.board.adopted.len(), 1);
    assert_eq!(
        sim.board.adopted[0].rule,
        Some(StructuredRule::BanEatSpecies { species: 3 })
    );
}

#[test]
fn proposal_expires_after_lifetime() {
    let mut sim = Simulation::new(two_agent_config(0x4D4005)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.config.proposals.proposal_lifetime_ticks = 1;
    sim.config.proposals.default_acceptance_threshold = 1.0;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "expires".into(),
            rule: None,
        },
    );
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Expired);
    assert!(sim.board.adopted.is_empty());
}

#[test]
fn ban_eat_species_blocks_eat() {
    let mut sim = Simulation::new(tiny_config(0x4D4006)).unwrap();
    let mushroom = sim.config.world.species.veg_tag_by_id("mushroom").unwrap();
    let id = AgentId(0);
    sim.board.adopted.push(AdoptedRule {
        proposal_id: 99,
        tick_accepted: 0,
        text: "do not eat mushroom".into(),
        rule: Some(StructuredRule::BanEatSpecies { species: mushroom }),
    });
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Food(mushroom), 1);
    }
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Eat {
            item: ItemId::Food(mushroom),
        },
    );
    let last = sim.events.events.last().unwrap();
    assert!(
        matches!(last.kind, SimEventKind::RuleBlocked { .. }),
        "{:?}",
        last.kind
    );
    assert_eq!(
        sim.agents
            .get(&id)
            .unwrap()
            .inventory
            .get(&ItemId::Food(mushroom))
            .copied()
            .unwrap_or(0),
        1
    );
}

#[test]
fn free_text_adopted_rule_does_not_block() {
    let mut sim = Simulation::new(tiny_config(0x4D4007)).unwrap();
    let berry = sim.config.world.species.veg_tag_by_id("berry").unwrap_or(1);
    let id = AgentId(0);
    sim.board.adopted.push(AdoptedRule {
        proposal_id: 1,
        tick_accepted: 0,
        text: "be excellent".into(),
        rule: None,
    });
    if let Some(a) = sim.agents.get_mut(&id) {
        a.inventory.clear();
        a.try_add_item(ItemId::Food(berry), 1);
        a.needs.hunger = 100;
    }
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Eat {
            item: ItemId::Food(berry),
        },
    );
    let last = sim.events.events.last().unwrap();
    assert!(
        matches!(last.kind, SimEventKind::Eat { .. }),
        "{:?}",
        last.kind
    );
}

#[test]
fn over_cap_propose_waits() {
    let mut sim = Simulation::new(tiny_config(0x4D4008)).unwrap();
    sim.config.proposals.max_open_proposals_per_agent = 1;
    let id = AgentId(0);
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Propose {
            text: "first".into(),
            rule: None,
        },
    );
    sim_core::execute::execute_primary(
        &mut sim,
        id,
        &PrimaryAction::Propose {
            text: "second".into(),
            rule: None,
        },
    );
    assert_eq!(sim.board.proposals.len(), 1);
    assert!(matches!(
        sim.events.events.last().unwrap().kind,
        SimEventKind::Wait
    ));
}

#[test]
fn mock_proposes_ban_after_toxic_memory() {
    let mut sim = Simulation::new(tiny_config(0x4D4009)).unwrap();
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
        sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::Propose { .. })),
        "expected a Propose from mock policy"
    );
    assert!(sim.board.proposals.iter().any(|p| {
        matches!(&p.rule, Some(StructuredRule::BanEatSpecies { species }) if *species == mushroom)
    }));
}

#[test]
fn parse_propose_json_and_unknown_rule_waits() {
    let sim = Simulation::new(tiny_config(0x4D400A)).unwrap();
    let obs = observation::build(&sim, AgentId(0));
    let ok = sim_core::parse_choice_json(
        r#"{"action":"Propose","text":"do not eat mushroom","rule":{"kind":"BanEatSpecies","species":"mushroom"}}"#,
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    assert!(matches!(
        ok.primary,
        PrimaryAction::Propose {
            rule: Some(StructuredRule::BanEatSpecies { .. }),
            ..
        }
    ));
    let wait = sim_core::parse_choice_json(
        r#"{"action":"Propose","text":"nope","rule":{"kind":"NotARule","species":"mushroom"}}"#,
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    assert!(matches!(wait.primary, PrimaryAction::Wait));
}

#[test]
fn parse_set_council_json() {
    let sim = Simulation::new(tiny_config(0x4D4048)).unwrap();
    let obs = observation::build(&sim, AgentId(0));
    let ok = sim_core::parse_choice_json(
        r#"{"action":"Propose","text":"seat","rule":{"kind":"set_council","council":[0,2,2]}}"#,
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    match ok.primary {
        PrimaryAction::Propose {
            rule: Some(StructuredRule::SetCouncil { ids }),
            ..
        } => {
            assert_eq!(ids, vec![AgentId(0), AgentId(2)]);
        }
        other => panic!("{other:?}"),
    }
    let wait = sim_core::parse_choice_json(
        r#"{"action":"Propose","text":"seat","rule":{"kind":"set_council","council":[]}}"#,
        &obs.legal,
        &sim.config.world.species,
    )
    .unwrap();
    assert!(matches!(wait.primary, PrimaryAction::Wait));
}

#[test]
fn report_world_consumed_is_sum_of_agents() {
    let mut sim = Simulation::new(tiny_config(0x4D400B)).unwrap();
    let berry = sim.config.world.species.veg_tag_by_id("berry").unwrap_or(1);
    let ids = sim.agent_ids();
    if let Some(a) = sim.agents.get_mut(&ids[0]) {
        a.inventory.clear();
        a.try_add_item(ItemId::Food(berry), 2);
        a.needs.hunger = 50;
    }
    sim_core::execute::execute_primary(
        &mut sim,
        ids[0],
        &PrimaryAction::Eat {
            item: ItemId::Food(berry),
        },
    );
    let report = build_report(&sim).unwrap();
    let sum_veg: u32 = report.agents.iter().map(|a| a.consumed_vegetation).sum();
    assert_eq!(report.world.consumed_vegetation, sum_veg);
    assert_eq!(sum_veg, 1);
}

#[test]
fn report_objective_veg_matches_world_cells() {
    let sim = Simulation::new(tiny_config(0x4D400C)).unwrap();
    let report = build_report(&sim).unwrap();
    let mut by_tag = std::collections::BTreeMap::<u8, u32>::new();
    for &tag in &sim.world.vegetation {
        if tag == 0 {
            continue;
        }
        if sim
            .config
            .world
            .species
            .veg(tag)
            .is_some_and(|s| s.yield_kind == sim_core::species::VegYield::Food)
        {
            *by_tag.entry(tag).or_insert(0) += 1;
        }
    }
    assert_eq!(report.world.veg.len(), by_tag.len());
    for v in &report.world.veg {
        assert_eq!(by_tag.get(&v.tag).copied().unwrap_or(0), v.cells);
    }
}

#[test]
fn report_known_available_subset_of_objective() {
    let sim = Simulation::new(tiny_config(0x4D400D)).unwrap();
    let report = build_report(&sim).unwrap();
    let mut objective = std::collections::BTreeSet::new();
    for v in &report.world.veg {
        objective.insert(v.tag);
    }
    if report.world.animals > 0 {
        objective.insert(100);
    }
    if report.world.fish > 0 {
        objective.insert(101);
    }
    for crop in sim.world.crops.values() {
        objective.insert(crop.species_tag);
    }
    for k in &report.world.known {
        assert!(
            objective.contains(&k.tag),
            "known tag {} ({}) not in objective {:?}",
            k.tag,
            k.id,
            objective
        );
    }
}

#[test]
fn load_checkpoint_regenerates_same_report() {
    let mut sim = Simulation::new(tiny_config(0x4D400E)).unwrap();
    sim.run_ticks(8);
    let first = report_markdown(&build_report(&sim).unwrap());
    let restored = Simulation::decode_checkpoint(&sim.encode_checkpoint().unwrap()).unwrap();
    let second = report_markdown(&build_report(&restored).unwrap());
    assert_eq!(first, second);
    assert_eq!(restored.state_hash(), sim.state_hash());
    assert_eq!(
        restored.agents.get(&AgentId(0)).unwrap().goals,
        sim.agents.get(&AgentId(0)).unwrap().goals
    );
}

#[test]
fn m3_empty_board_checkpoint_still_loads() {
    let sim = Simulation::new(tiny_config(0x4D400F)).unwrap();
    let mut body = sim.to_checkpoint().unwrap();
    body.public_board.entries.clear();
    body.config_hash = [0; 32];
    let bytes = sim_core::encode_checkpoint(&body).unwrap();
    let loaded = Simulation::decode_checkpoint(&bytes).unwrap();
    assert!(loaded.board.proposals.is_empty());
    assert!(loaded.agents.get(&AgentId(0)).unwrap().goals.is_empty());
}

fn propose_only_agent0(sim: &mut Simulation) {
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "do not eat mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
}

#[test]
fn equal_tally_one_of_three_stays_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4010)).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
    assert!(sim.board.adopted.is_empty());
}

#[test]
fn influence_kingmaker_one_support_accepts() {
    let mut sim = Simulation::new(three_agent_config(0x4D4011)).unwrap();
    sim.voting.weight = VoteWeight::Influence;
    sim.inject_schedule_toml(KINGMAKER).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(
        sim.board.proposals[0].status,
        ProposalStatus::Accepted,
        "yes_w should meet need; inf0={} need={}",
        sim.vote_weight_of(AgentId(0)),
        sim.vote_need()
    );
    assert_eq!(sim.board.adopted.len(), 1);
}

#[test]
fn equal_with_kingmaker_still_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4012)).unwrap();
    sim.voting.weight = VoteWeight::Equal;
    sim.inject_schedule_toml(KINGMAKER).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
}

#[test]
fn equal_no_boost_same_seed_same_hash() {
    let cfg = three_agent_config(0x4D4013);
    let mut a = Simulation::new(cfg.clone()).unwrap();
    let mut b = Simulation::new(cfg).unwrap();
    a.chooser = sim_core::Chooser::Wait;
    b.chooser = sim_core::Chooser::Wait;
    a.run_ticks(8);
    b.run_ticks(8);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn respect_kingmaker_one_support_accepts() {
    let mut sim = Simulation::new(three_agent_config(0x4D4014)).unwrap();
    sim.voting.weight = VoteWeight::Respect;
    sim.inject_schedule_toml(ESTEEM).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(
        sim.board.proposals[0].status,
        ProposalStatus::Accepted,
        "yes_w should meet need; w0={} need={}",
        sim.vote_weight_of(AgentId(0)),
        sim.vote_need()
    );
    assert_eq!(sim.vote_weight_of(AgentId(0)), 14000);
    assert_eq!(sim.vote_weight_of(AgentId(1)), 1);
    assert_eq!(sim.board.adopted.len(), 1);
}

#[test]
fn equal_with_esteem_still_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4015)).unwrap();
    sim.voting.weight = VoteWeight::Equal;
    sim.inject_schedule_toml(ESTEEM).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
}

#[test]
fn respect_zero_same_hash_as_equal() {
    let cfg = three_agent_config(0x4D4016);
    let mut eq = Simulation::new(cfg.clone()).unwrap();
    let mut rs = Simulation::new(cfg).unwrap();
    eq.chooser = sim_core::Chooser::Wait;
    rs.chooser = sim_core::Chooser::Wait;
    eq.voting.weight = VoteWeight::Equal;
    rs.voting.weight = VoteWeight::Respect;
    eq.run_ticks(8);
    rs.run_ticks(8);
    assert_eq!(eq.state_hash(), rs.state_hash());
}

fn support(sim: &mut Simulation, id: u64, pid: u64) {
    sim_core::execute::execute_primary(
        sim,
        AgentId(id),
        &PrimaryAction::Support { proposal_id: pid },
    );
}

fn oppose(sim: &mut Simulation, id: u64, pid: u64) {
    sim_core::execute::execute_primary(
        sim,
        AgentId(id),
        &PrimaryAction::Oppose { proposal_id: pid },
    );
}

#[test]
fn unanimous_one_of_three_stays_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4020)).unwrap();
    sim.voting.accept = VoteAccept::Unanimous;
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
}

#[test]
fn unanimous_all_support_accepts() {
    let mut sim = Simulation::new(three_agent_config(0x4D4021)).unwrap();
    sim.voting.accept = VoteAccept::Unanimous;
    propose_only_agent0(&mut sim);
    support(&mut sim, 1, 0);
    support(&mut sim, 2, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
    assert_eq!(sim.board.adopted.len(), 1);
}

#[test]
fn unanimous_one_oppose_rejects() {
    let mut sim = Simulation::new(three_agent_config(0x4D4022)).unwrap();
    sim.voting.accept = VoteAccept::Unanimous;
    propose_only_agent0(&mut sim);
    oppose(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Rejected);
}

#[test]
fn council_two_support_accepts_with_silent_third() {
    let mut sim = Simulation::new(three_agent_config(0x4D4023)).unwrap();
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1)];
    propose_only_agent0(&mut sim);
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
}

#[test]
fn council_one_oppose_rejects() {
    let mut sim = Simulation::new(three_agent_config(0x4D4024)).unwrap();
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1)];
    propose_only_agent0(&mut sim);
    oppose(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Rejected);
}

fn council3(sim: &mut Simulation) {
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1), AgentId(2)];
    sim.voting.council_tally = CouncilTally::Majority;
}

#[test]
fn council_majority_two_of_three_accepts() {
    let mut sim = Simulation::new(three_agent_config(0x4D4040)).unwrap();
    council3(&mut sim);
    propose_only_agent0(&mut sim);
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
}

#[test]
fn council_majority_one_of_three_stays_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4041)).unwrap();
    council3(&mut sim);
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
}

#[test]
fn council_majority_influence_kingmaker() {
    let mut sim = Simulation::new(three_agent_config(0x4D4042)).unwrap();
    council3(&mut sim);
    sim.voting.weight = VoteWeight::Influence;
    sim.inject_schedule_toml(KINGMAKER).unwrap();
    propose_only_agent0(&mut sim);
    sim.tick();
    assert_eq!(
        sim.board.proposals[0].status,
        ProposalStatus::Accepted,
        "yes_w should meet council need; inf0={} need={}",
        sim.vote_weight_of(AgentId(0)),
        sim.vote_need()
    );
}

#[test]
fn council_unanimous_two_of_three_stays_open() {
    let mut sim = Simulation::new(three_agent_config(0x4D4043)).unwrap();
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1), AgentId(2)];
    propose_only_agent0(&mut sim);
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Open);
}

#[test]
fn fog_board_omits_far_author() {
    let mut sim = Simulation::new(three_agent_config(0x4D4025)).unwrap();
    sim.config.proposals.public_board_always_visible = false;
    sim.chooser = sim_core::Chooser::Wait;
    if let Some(a) = sim.agents.get_mut(&AgentId(0)) {
        a.x = 0;
        a.y = 0;
    }
    if let Some(a) = sim.agents.get_mut(&AgentId(1)) {
        a.x = 31;
        a.y = 31;
    }
    propose_only_agent0(&mut sim);
    let obs0 = observation::build(&sim, AgentId(0));
    assert!(
        obs0.board.iter().any(|p| p.id == 0),
        "author should see own proposal"
    );
    let obs1 = observation::build(&sim, AgentId(1));
    assert!(
        obs1.board.iter().all(|p| p.id != 0),
        "far agent must not see open proposal"
    );
    assert!(
        !obs1
            .legal
            .iter()
            .any(|a| matches!(a, PrimaryAction::Support { proposal_id: 0 })),
        "Support of fogged id must not be legal"
    );
}

#[test]
fn identified_agent_has_last_action() {
    let mut sim = Simulation::new(three_agent_config(0x4D4026)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    if let Some(a) = sim.agents.get_mut(&AgentId(0)) {
        a.x = 5;
        a.y = 5;
    }
    if let Some(a) = sim.agents.get_mut(&AgentId(1)) {
        a.x = 6;
        a.y = 5;
    }
    sim.tick();
    let obs = observation::build(&sim, AgentId(0));
    let other = obs
        .agents
        .iter()
        .find(|a| a.id == Some(AgentId(1)))
        .expect("identified neighbor");
    assert_eq!(other.last_action.as_deref(), Some("wait"));
}

fn three_agent_meta(master_seed: u64) -> ExperimentConfig {
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
count = 3
[proposals]
default_acceptance_threshold = 0.5
allow_meta_rules = true
"#
    );
    ExperimentConfig::from_toml_str(&toml).unwrap()
}

#[test]
fn meta_propose_waits_when_flag_off() {
    let mut sim = Simulation::new(three_agent_config(0x4D4030)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "lower threshold".into(),
            rule: Some(StructuredRule::SetAcceptanceThreshold { milli: 1000 }),
        },
    );
    assert!(sim.board.proposals.is_empty());
    assert!(matches!(
        sim.events.events.last().unwrap().kind,
        SimEventKind::Wait
    ));
}

#[test]
fn meta_threshold_lets_one_of_three_accept() {
    let mut sim = Simulation::new(three_agent_meta(0x4D4031)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "lower threshold".into(),
            rule: Some(StructuredRule::SetAcceptanceThreshold { milli: 1000 }),
        },
    );
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "ban mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    sim.tick();
    let p = sim.board.proposals.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(p.status, ProposalStatus::Accepted);
}

#[test]
fn meta_unanimous_one_of_three_stays_open() {
    let mut sim = Simulation::new(three_agent_meta(0x4D4032)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "require unanimity".into(),
            rule: Some(StructuredRule::SetVoteAccept {
                accept: VoteAccept::Unanimous,
            }),
        },
    );
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "ban mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    sim.tick();
    let p = sim.board.proposals.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(p.status, ProposalStatus::Open);
}

#[test]
fn meta_set_council_waits_when_flag_off() {
    let mut sim = Simulation::new(three_agent_config(0x4D4044)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "new council".into(),
            rule: Some(StructuredRule::SetCouncil {
                ids: vec![AgentId(0), AgentId(2)],
            }),
        },
    );
    assert!(sim.board.proposals.is_empty());
    assert!(matches!(
        sim.events.events.last().unwrap().kind,
        SimEventKind::Wait
    ));
}

#[test]
fn empty_set_council_propose_waits() {
    let mut sim = Simulation::new(three_agent_meta(0x4D4045)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "empty council".into(),
            rule: Some(StructuredRule::SetCouncil { ids: vec![] }),
        },
    );
    assert!(sim.board.proposals.is_empty());
    assert!(matches!(
        sim.events.events.last().unwrap().kind,
        SimEventKind::Wait
    ));
}

#[test]
fn meta_set_council_roster_then_unanimous() {
    let mut sim = Simulation::new(three_agent_meta(0x4D4046)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1)];
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "seat 0 and 2".into(),
            rule: Some(StructuredRule::SetCouncil {
                ids: vec![AgentId(0), AgentId(2)],
            }),
        },
    );
    support(&mut sim, 1, 0);
    sim.tick();
    assert_eq!(sim.board.proposals[0].status, ProposalStatus::Accepted);
    sim.refresh_meta();
    assert_eq!(sim.effective_council(), &[AgentId(0), AgentId(2)]);
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "ban mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    support(&mut sim, 2, 1);
    sim.tick();
    let p = sim.board.proposals.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(p.status, ProposalStatus::Accepted);

    let mut sim = Simulation::new(three_agent_meta(0x4D4047)).unwrap();
    sim.chooser = sim_core::Chooser::Wait;
    sim.voting.accept = VoteAccept::Council;
    sim.voting.council = vec![AgentId(0), AgentId(1)];
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "seat 0 and 2".into(),
            rule: Some(StructuredRule::SetCouncil {
                ids: vec![AgentId(0), AgentId(2)],
            }),
        },
    );
    support(&mut sim, 1, 0);
    sim.tick();
    sim_core::execute::execute_primary(
        &mut sim,
        AgentId(0),
        &PrimaryAction::Propose {
            text: "ban mushroom".into(),
            rule: Some(StructuredRule::BanEatSpecies { species: 3 }),
        },
    );
    oppose(&mut sim, 2, 1);
    sim.tick();
    let p = sim.board.proposals.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(p.status, ProposalStatus::Rejected);
}
