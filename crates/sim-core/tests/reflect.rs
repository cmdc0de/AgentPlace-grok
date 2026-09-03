//! M25 reflection-on-evict.

use sim_core::action::ChosenAction;
use sim_core::llm::{ActionChooser, ChooseError, LlmBarrierParams};
use sim_core::memory::{MemoryEntry, MemoryKind};
use sim_core::observation::Observation;
use sim_core::{AgentId, Chooser, ExperimentConfig, Simulation};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

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
capacity = 2
"#
    ))
    .unwrap()
}

fn noise(tick: u64, text: &str) -> MemoryEntry {
    MemoryEntry {
        tick,
        kind: MemoryKind::Observation,
        text: text.into(),
        importance: 10,
        last_accessed: tick,
        species_tag: 0,
        id: 0,
        participants: Vec::new(),
        valence: 0,
    }
}

struct ReflectOk {
    calls: AtomicU32,
    summary: String,
}

impl ActionChooser for ReflectOk {
    fn choose(
        &self,
        _call_seed: u64,
        _obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        Ok((ChosenAction::wait(), r#"{"action":"Wait"}"#.into()))
    }

    fn reflect(&self, _seed: u64, dropped: &[String]) -> Result<String, ChooseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(!dropped.is_empty());
        Ok(self.summary.clone())
    }
}

struct ReflectErr {
    calls: AtomicU32,
}

impl ActionChooser for ReflectErr {
    fn choose(
        &self,
        _call_seed: u64,
        _obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        Ok((ChosenAction::wait(), r#"{"action":"Wait"}"#.into()))
    }

    fn reflect(&self, _seed: u64, _dropped: &[String]) -> Result<String, ChooseError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ChooseError::Timeout)
    }
}

#[test]
fn overlay_parses_reflect_on_evict() {
    let p = LlmBarrierParams::from_config_toml("[llm]\nreflect_on_evict = true\n");
    assert!(p.reflect_on_evict);
    let d = LlmBarrierParams::from_config_toml("[llm]\nprovider = \"mock\"\n");
    assert!(!d.reflect_on_evict);
}

#[test]
fn mock_reflect_overlay_same_hash() {
    let cfg = tiny(0x25_01);
    let mut off = Simulation::new(cfg.clone()).unwrap();
    let mut on = Simulation::new(cfg).unwrap();
    on.llm_reflect_on_evict = true;
    off.run_ticks(5);
    on.run_ticks(5);
    assert_eq!(off.state_hash(), on.state_hash());
}

#[test]
fn custom_reflect_ok_writes_protected_memory() {
    let chooser = Arc::new(ReflectOk {
        calls: AtomicU32::new(0),
        summary: "we used to wander".into(),
    });
    let mut sim = Simulation::new(tiny(0x25_02)).unwrap();
    sim.llm_reflect_on_evict = true;
    sim.chooser = Chooser::Custom(chooser.clone());
    let id = AgentId(0);
    sim.remember_entry(id, noise(1, "alpha"));
    sim.remember_entry(id, noise(2, "beta"));
    sim.remember_entry(id, noise(3, "gamma"));
    assert!(chooser.calls.load(Ordering::SeqCst) >= 1);
    let mem = &sim.agents.get(&id).unwrap().memory;
    assert!(
        mem.iter()
            .any(|e| e.kind == MemoryKind::Reflection && e.text.contains("wander")),
        "{mem:?}"
    );
    let texts: Vec<_> = mem.iter().map(|e| e.text.as_str()).collect();
    assert!(
        !(texts.contains(&"alpha") && texts.contains(&"beta") && texts.contains(&"gamma")),
        "not all originals should remain: {texts:?}"
    );
}

#[test]
fn custom_reflect_err_drops_without_reflection() {
    let chooser = Arc::new(ReflectErr {
        calls: AtomicU32::new(0),
    });
    let mut sim = Simulation::new(tiny(0x25_03)).unwrap();
    sim.llm_reflect_on_evict = true;
    sim.chooser = Chooser::Custom(chooser.clone());
    let id = AgentId(0);
    sim.remember_entry(id, noise(1, "alpha"));
    sim.remember_entry(id, noise(2, "beta"));
    sim.remember_entry(id, noise(3, "gamma"));
    assert!(chooser.calls.load(Ordering::SeqCst) >= 1);
    let mem = &sim.agents.get(&id).unwrap().memory;
    assert!(
        !mem.iter().any(|e| e.kind == MemoryKind::Reflection),
        "{mem:?}"
    );
    assert!(mem.len() <= 2);
}

#[test]
fn replay_skips_reflect() {
    let chooser = Arc::new(ReflectOk {
        calls: AtomicU32::new(0),
        summary: "should not appear".into(),
    });
    let mut sim = Simulation::new(tiny(0x25_04)).unwrap();
    sim.llm_reflect_on_evict = true;
    sim.chooser = Chooser::Custom(chooser.clone());
    sim.replay = Some(sim_core::ReplayTable::default());
    let id = AgentId(0);
    sim.remember_entry(id, noise(1, "alpha"));
    sim.remember_entry(id, noise(2, "beta"));
    sim.remember_entry(id, noise(3, "gamma"));
    assert_eq!(chooser.calls.load(Ordering::SeqCst), 0);
    let mem = &sim.agents.get(&id).unwrap().memory;
    assert!(!mem.iter().any(|e| e.kind == MemoryKind::Reflection));
}
