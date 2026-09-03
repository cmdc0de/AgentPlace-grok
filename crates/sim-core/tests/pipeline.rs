//! M26 periodic Reflect/Plan and replay call-kind.

use sim_core::action::ChosenAction;
use sim_core::llm::{
    ActionChooser, ChooseError, LlmBarrierParams, REPLAY_CALL_PLAN, REPLAY_CALL_REFLECT,
    ReplayTable,
};
use sim_core::memory::MemoryKind;
use sim_core::observation::Observation;
use sim_core::{AgentId, Chooser, ExperimentConfig, Simulation};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
[agents.memory]
capacity = 32
"#
    ))
    .unwrap()
}

fn tmp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "agentplace-{name}-{}-{}.jsonl",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

struct PipelineStub {
    insight: AtomicU32,
    plan: AtomicU32,
    choose: AtomicU32,
    insight_ok: bool,
    plan_ok: bool,
    last_plan: Mutex<Vec<String>>,
}

impl PipelineStub {
    fn new(insight_ok: bool, plan_ok: bool) -> Arc<Self> {
        Arc::new(Self {
            insight: AtomicU32::new(0),
            plan: AtomicU32::new(0),
            choose: AtomicU32::new(0),
            insight_ok,
            plan_ok,
            last_plan: Mutex::new(Vec::new()),
        })
    }
}

impl ActionChooser for PipelineStub {
    fn choose(
        &self,
        _call_seed: u64,
        obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        self.choose.fetch_add(1, Ordering::SeqCst);
        *self.last_plan.lock().unwrap() = obs.plan.clone();
        Ok((ChosenAction::wait(), r#"{"action":"Wait"}"#.into()))
    }

    fn insight(&self, _seed: u64, _obs: &Observation) -> Result<String, ChooseError> {
        self.insight.fetch_add(1, Ordering::SeqCst);
        if self.insight_ok {
            Ok("the village is quiet".into())
        } else {
            Err(ChooseError::Timeout)
        }
    }

    fn plan(
        &self,
        _seed: u64,
        _obs: &Observation,
        max_len: usize,
    ) -> Result<Vec<String>, ChooseError> {
        self.plan.fetch_add(1, Ordering::SeqCst);
        if self.plan_ok {
            Ok(vec!["drink".into(), "eat".into()]
                .into_iter()
                .take(max_len.max(1))
                .collect())
        } else {
            Err(ChooseError::Timeout)
        }
    }
}

#[test]
fn overlay_parses_every_n() {
    let p = LlmBarrierParams::from_config_toml(
        "[llm]\nreflect_every_n_ticks = 10\nplan_every_n_ticks = 8\nplan_length = 3\n",
    );
    assert_eq!(p.reflect_every_n_ticks, 10);
    assert_eq!(p.plan_every_n_ticks, 8);
    assert_eq!(p.plan_length, 3);
    let d = LlmBarrierParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert_eq!(d.reflect_every_n_ticks, 0);
    assert_eq!(d.plan_every_n_ticks, 0);
    assert_eq!(d.plan_length, 4);
}

#[test]
fn mock_every_n_overlay_same_hash() {
    let cfg = tiny(0x26_01);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.llm_reflect_every_n = 2;
    on.llm_plan_every_n = 2;
    off.run_ticks(6);
    on.run_ticks(6);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn custom_insight_ok_on_tick_n() {
    let stub = PipelineStub::new(true, false);
    let mut sim = Simulation::new(tiny(0x26_02)).unwrap();
    sim.chooser = Chooser::Custom(stub.clone());
    sim.llm_reflect_every_n = 2;
    sim.run_ticks(1);
    assert_eq!(sim.tick, 1);
    assert!(!sim
        .agents
        .values()
        .any(|a| a.memory.iter().any(|e| e.kind == MemoryKind::Reflection)));
    assert_eq!(stub.insight.load(Ordering::SeqCst), 0);
    sim.run_ticks(1);
    assert_eq!(sim.tick, 2);
    assert!(stub.insight.load(Ordering::SeqCst) >= 1);
    assert!(
        sim.agents
            .values()
            .any(|a| a.memory.iter().any(|e| e.kind == MemoryKind::Reflection
                && e.text.contains("quiet"))),
        "expected insight Reflection"
    );
}

#[test]
fn custom_plan_ok_visible_to_choose() {
    let stub = PipelineStub::new(false, true);
    let mut sim = Simulation::new(tiny(0x26_03)).unwrap();
    sim.chooser = Chooser::Custom(stub.clone());
    sim.llm_plan_every_n = 2;
    sim.run_ticks(2);
    let id = AgentId(0);
    let plan = &sim.agents.get(&id).unwrap().plan;
    assert!(!plan.is_empty(), "{plan:?}");
    let seen = stub.last_plan.lock().unwrap().clone();
    assert_eq!(seen, *plan);
}

#[test]
fn custom_insight_plan_err_skips() {
    let stub = PipelineStub::new(false, false);
    let mut sim = Simulation::new(tiny(0x26_04)).unwrap();
    sim.chooser = Chooser::Custom(stub.clone());
    sim.llm_reflect_every_n = 2;
    sim.llm_plan_every_n = 2;
    sim.run_ticks(4);
    assert!(stub.insight.load(Ordering::SeqCst) >= 1);
    assert!(stub.plan.load(Ordering::SeqCst) >= 1);
    assert!(!sim
        .agents
        .values()
        .any(|a| a.memory.iter().any(|e| e.kind == MemoryKind::Reflection)));
    assert!(sim.agents.values().all(|a| a.plan.is_empty()));
}

#[test]
fn overlay_off_custom_no_insight_plan_calls() {
    let stub = PipelineStub::new(true, true);
    let mut sim = Simulation::new(tiny(0x26_05)).unwrap();
    sim.chooser = Chooser::Custom(stub.clone());
    sim.run_ticks(4);
    assert_eq!(stub.insight.load(Ordering::SeqCst), 0);
    assert_eq!(stub.plan.load(Ordering::SeqCst), 0);
    assert!(stub.choose.load(Ordering::SeqCst) >= 1);
}

#[test]
fn old_jsonl_without_call_is_choose() {
    let jsonl = r#"{"tick":1,"agent":0,"call_seed":1,"prompt_hash":"ab","response":"{\"action\":\"Wait\"}"}
{"tick":1,"agent":0,"call_seed":2,"prompt_hash":"cd","response":"{\"reflection\":\"sum\"}","call":"reflect"}"#;
    let table = ReplayTable::from_jsonl(jsonl);
    assert!(table.get(1, 0).unwrap().contains("Wait"));
    assert!(
        table
            .get_call(1, 0, REPLAY_CALL_REFLECT)
            .unwrap()
            .contains("sum")
    );
    assert!(table.get_call(1, 0, REPLAY_CALL_PLAN).is_none());
}

#[test]
fn record_replay_insight_plan_same_hash() {
    let rec = tmp("m26-pipe");
    let _ = std::fs::remove_file(&rec);
    let mut cfg = tiny(0x26_06);
    cfg.llm.replay_file = rec.to_string_lossy().into();

    let writer_stub = PipelineStub::new(true, true);
    let mut writer = Simulation::new(cfg.clone()).unwrap();
    writer.chooser = Chooser::Custom(writer_stub);
    writer.llm_reflect_every_n = 2;
    writer.llm_plan_every_n = 2;
    writer.run_ticks(4);
    let hash = writer.state_hash();
    let text = std::fs::read_to_string(&rec).unwrap();
    assert!(text.contains("\"call\":\"reflect\""), "{text}");
    assert!(text.contains("\"call\":\"plan\""), "{text}");

    let replay_stub = PipelineStub::new(true, true);
    let mut replayed = Simulation::new(cfg).unwrap();
    replayed.chooser = Chooser::Custom(replay_stub.clone());
    replayed.llm_reflect_every_n = 2;
    replayed.llm_plan_every_n = 2;
    replayed.run_ticks(4);
    assert_eq!(replayed.state_hash(), hash);
    assert_eq!(replay_stub.insight.load(Ordering::SeqCst), 0);
    assert_eq!(replay_stub.plan.load(Ordering::SeqCst), 0);
    let _ = std::fs::remove_file(&rec);
}
