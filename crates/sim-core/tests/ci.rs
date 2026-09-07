//! M29 CI workflow is in-tree (Ubuntu / Windows / macOS, mock, no network).

use std::path::Path;

#[test]
fn github_actions_workflow_exists() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/test.yml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("missing {}: {e}", path.display());
    });
    assert!(text.contains("ubuntu-latest"), "{text}");
    assert!(text.contains("windows-latest"), "{text}");
    assert!(text.contains("macos-latest"), "{text}");
    assert!(text.contains("cargo test -p sim-core"), "{text}");
    assert!(text.contains("cargo test -p viewer"), "{text}");
    assert!(text.contains("cargo test -p sim-cli --test net"), "{text}");
    assert!(text.contains("cargo test -p shared"), "{text}");
    assert!(text.contains("wasm32-unknown-unknown"), "{text}");
    assert!(text.contains("sim-wasm"), "{text}");
}
