//! M36 InspectorView (not hashed).

use sim_core::board::{AdoptedRule, Proposal, ProposalStatus};
use sim_core::social::RelationshipSummary;
use sim_core::{AgentId, ExperimentConfig, InspectorView, Simulation};

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
fn inspector_one_row_per_living_agent() {
    let mut sim = Simulation::new(tiny(0x36_01)).unwrap();
    sim.run_ticks(2);
    let hash = sim.state_hash();
    let view = InspectorView::from_sim(&sim);
    assert_eq!(view.agents.len(), sim.agents.len());
    for a in &view.agents {
        assert!(sim.agents.contains_key(&AgentId(a.id)));
        assert!(a.hunger <= 10_000, "display hunger {}", a.hunger);
        let _ = a.kinship.len();
        let _ = a.relationships.len();
    }
    assert_eq!(sim.state_hash(), hash, "InspectorView must not be hashed");
}

#[test]
fn inspector_board_and_metrics() {
    let mut sim = Simulation::new(tiny(0x36_02)).unwrap();
    sim.board.proposals.push(Proposal {
        id: 1,
        author: AgentId(0),
        tick_created: 0,
        text: "share food".into(),
        rule: None,
        supporters: Default::default(),
        opposers: Default::default(),
        status: ProposalStatus::Open,
    });
    sim.board.adopted.push(AdoptedRule {
        proposal_id: 9,
        tick_accepted: 1,
        text: "be kind".into(),
        rule: None,
    });
    let a = AgentId(0);
    let b = AgentId(1);
    if let Some(ag) = sim.agents.get_mut(&a) {
        ag.relationships.insert(
            b,
            RelationshipSummary {
                trust: 400,
                affinity: 10,
                respect: 20,
                fear: 0,
                interaction_count: 1,
                last_interaction_tick: 0,
                notable: Vec::new(),
            },
        );
        ag.needs.hunger = 100;
        ag.illness_ticks = 2;
        ag.kinship.household = Some(3);
    }
    let view = InspectorView::from_sim(&sim);
    assert!(
        view.board
            .open
            .iter()
            .any(|p| p.text.contains("share food")),
        "{:?}",
        view.board.open
    );
    assert!(
        view.board
            .adopted
            .iter()
            .any(|r| r.text.contains("be kind")),
        "{:?}",
        view.board.adopted
    );
    let row = view.agents.iter().find(|x| x.id == 0).unwrap();
    assert!(
        row.relationships
            .iter()
            .any(|r| r.id == 1 && r.trust == 400)
    );
    assert!(row.kinship.iter().any(|s| s.contains("household")));
    assert!(view.metrics.hungry >= 1);
    assert!(view.metrics.illness >= 1);
    let json = view.to_json();
    assert!(json.contains("\"agents\""), "{json}");
    assert!(json.contains("\"board\""), "{json}");
    assert!(json.contains("\"metrics\""), "{json}");
    sim.run_ticks(1);
    let after = InspectorView::from_sim(&sim);
    assert!(after.metrics.timing.is_some());
}

#[test]
fn inspector_metrics_json_viewer_still_parses_timing() {
    let mut sim = Simulation::new(tiny(0x36_03)).unwrap();
    sim.run_ticks(1);
    let bytes = sim_core::inspector::metrics_json_with_inspector(&sim);
    let t: sim_core::TickTiming = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(t.tick, sim.tick);
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("inspector").is_some(), "{v}");
    assert_eq!(
        v["inspector"]["agents"].as_array().map(|a| a.len()),
        Some(sim.agents.len())
    );
}
