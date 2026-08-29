//! MF-3: pipeline completeness and opt-in LLM barrier retries.

use sim_core::action::ChosenAction;
use sim_core::event_log::SimEventKind;
use sim_core::llm::{ActionChooser, ChooseError, LlmBarrierParams};
use sim_core::observation::Observation;
use sim_core::{Chooser, ExperimentConfig, Simulation};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

fn tiny(seed: u64, agents: u32) -> ExperimentConfig {
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
count = {agents}
"#
    ))
    .unwrap()
}

struct CountingChooser {
    fails: u32,
    calls: AtomicU32,
    err: ChooseError,
}

impl ActionChooser for CountingChooser {
    fn choose(
        &self,
        _call_seed: u64,
        _obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fails {
            Err(self.err)
        } else {
            Ok((ChosenAction::wait(), r#"{"action":"Wait"}"#.into()))
        }
    }
}

struct SleepOk {
    delay: Duration,
}

impl ActionChooser for SleepOk {
    fn choose(
        &self,
        _call_seed: u64,
        _obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        std::thread::sleep(self.delay);
        Ok((ChosenAction::wait(), r#"{"action":"Wait"}"#.into()))
    }
}

struct AlwaysTimeout {
    calls: AtomicU32,
}

impl ActionChooser for AlwaysTimeout {
    fn choose(
        &self,
        _call_seed: u64,
        _obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ChooseError::Timeout)
    }
}

#[test]
fn overlay_barrier_not_on_experiment_config() {
    let p = LlmBarrierParams::from_config_toml(
        r#"
[llm]
provider = "mock"
barrier = true
barrier_retries = 5
"#,
    );
    assert!(p.barrier);
    assert_eq!(p.retries, 5);
    let d = LlmBarrierParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.barrier);
    assert_eq!(d.retries, 3);
}

#[test]
fn mock_pipeline_n_of_n() {
    let mut sim = Simulation::new(tiny(0x23_01, 4)).unwrap();
    sim.tick();
    let t = sim.last_tick_timing.expect("timing");
    assert!(t.pipeline_complete(sim.agents.len()));
    assert_eq!(t.agents.len(), 4);
    for row in &t.agents {
        let _written = row.remember_ns;
    }
}

#[test]
fn sleeping_chooser_blocks_tick_return() {
    let mut sim = Simulation::new(tiny(0x23_02, 2)).unwrap();
    sim.chooser = Chooser::Custom(Arc::new(SleepOk {
        delay: Duration::from_millis(40),
    }));
    let t0 = Instant::now();
    sim.tick();
    assert!(t0.elapsed() >= Duration::from_millis(70));
    let t = sim.last_tick_timing.expect("timing");
    assert!(t.pipeline_complete(2));
    assert!(
        !sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::LlmWait))
    );
}

#[test]
fn barrier_retries_then_success() {
    let chooser = Arc::new(CountingChooser {
        fails: 2,
        calls: AtomicU32::new(0),
        err: ChooseError::Timeout,
    });
    let mut sim = Simulation::new(tiny(0x23_03, 2)).unwrap();
    sim.llm_barrier = true;
    sim.llm_barrier_retries = 3;
    sim.chooser = Chooser::Custom(chooser.clone());
    sim.tick();
    assert_eq!(chooser.calls.load(Ordering::SeqCst), 4);
    assert!(
        !sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::LlmWait))
    );
}

#[test]
fn barrier_timeout_exhausted_then_wait_other_agents_run() {
    let chooser = Arc::new(AlwaysTimeout {
        calls: AtomicU32::new(0),
    });
    let mut sim = Simulation::new(tiny(0x23_04, 2)).unwrap();
    sim.llm_barrier = true;
    sim.llm_barrier_retries = 3;
    sim.chooser = Chooser::Custom(chooser.clone());
    sim.tick();
    assert_eq!(
        chooser.calls.load(Ordering::SeqCst),
        8,
        "2 agents × 4 attempts"
    );
    let waits = sim
        .events
        .events
        .iter()
        .filter(|e| matches!(e.kind, SimEventKind::LlmWait))
        .count();
    assert_eq!(waits, 2);
    let t = sim.last_tick_timing.expect("timing");
    assert!(t.pipeline_complete(2));
}

#[test]
fn barrier_retries_zero_one_attempt() {
    let chooser = Arc::new(AlwaysTimeout {
        calls: AtomicU32::new(0),
    });
    let mut sim = Simulation::new(tiny(0x23_05, 2)).unwrap();
    sim.llm_barrier = true;
    sim.llm_barrier_retries = 0;
    sim.chooser = Chooser::Custom(chooser.clone());
    sim.tick();
    assert_eq!(chooser.calls.load(Ordering::SeqCst), 2);
    assert!(
        sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::LlmWait))
    );
}

#[test]
fn barrier_off_timeout_is_one_attempt() {
    let chooser = Arc::new(AlwaysTimeout {
        calls: AtomicU32::new(0),
    });
    let mut sim = Simulation::new(tiny(0x23_06, 2)).unwrap();
    assert!(!sim.llm_barrier);
    sim.chooser = Chooser::Custom(chooser.clone());
    sim.tick();
    assert_eq!(chooser.calls.load(Ordering::SeqCst), 2);
    assert!(
        sim.events
            .events
            .iter()
            .any(|e| matches!(e.kind, SimEventKind::LlmWait))
    );
}
