//! Snapshot bytes → InspectorView JSON. Native tests; wasm32 export for the page.

use sim_core::InspectorView;

pub fn inspect_checkpoint_json(bytes: &[u8]) -> String {
    InspectorView::from_checkpoint_bytes(bytes)
        .map(|v| v.to_json())
        .unwrap_or_else(|_| "{}".into())
}

#[cfg(target_arch = "wasm32")]
mod wasm_api {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn inspect_checkpoint(bytes: &[u8]) -> String {
        super::inspect_checkpoint_json(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{ExperimentConfig, InspectorView, Simulation};

    #[test]
    fn native_helper_matches_from_sim() {
        let cfg = ExperimentConfig::from_toml_str(
            r#"
master_seed = 38
[simulation]
max_ticks = 10
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
        )
        .unwrap();
        let mut sim = Simulation::new(cfg).unwrap();
        sim.run_ticks(2);
        let bytes = sim.encode_checkpoint().unwrap();
        let json = inspect_checkpoint_json(&bytes);
        let view: InspectorView = serde_json::from_str(&json).unwrap();
        let direct = InspectorView::from_sim(&sim);
        assert_eq!(view.tick, direct.tick);
        assert_eq!(view.agents.len(), direct.agents.len());
    }
}
