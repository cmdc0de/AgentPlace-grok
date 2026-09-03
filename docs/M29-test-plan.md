# M29 test plan — see each new feature

Walkthrough for [`M29-plan.md`](M29-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (local embeddings, combat death, CI Win/mac). Next slice: [`M32-plan.md`](M32-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test memory`, `--test combat`, `--test ci`; viewer tests live in the binary crate. Unit tests under `--lib` need the module path (`memory::tests::…`).

**Success for the slice:** mock hashes unchanged at default embeddings-off. `enable_embeddings = true` can prefer text-similar memories; vectors are not in `state_hash` and never hit the network. Overlay `[conflict] death_enabled` / `--conflict-death` removes an agent at 0 health (`CombatDeath` tag 28); overlay off keeps M28 incapacitation. `.github/workflows/test.yml` matrices Ubuntu / Windows / macOS. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Local embeddings

```bash
cargo test -p sim-core --lib memory::tests::retrieve_embeddings_off_matches_legacy_order -- --exact --nocapture
cargo test -p sim-core --lib memory::tests::retrieve_embeddings_prefer_similar -- --exact --nocapture
cargo test -p sim-core --lib memory::tests::embeddings_ignored_by_hash_into -- --exact --nocapture
cargo test -p sim-core --test memory enable_embeddings_false_same_retrieve_as_today -- --exact --nocapture
cargo test -p sim-core --test memory enable_embeddings_true_state_hash_ignores_vectors -- --exact --nocapture
```

| Test | Success |
|---|---|
| embeddings off | retrieve order is importance × recency (same as today) |
| embeddings on | query prefers text-similar memories |
| hash_into | different vectors, same text → same digest |
| flag false | high-importance memory still first |
| flag true, mock run | **state_hash** matches embeddings-off; **config_hash** differs |

`[agents.memory] enable_embeddings` stays **false** in `configs/default.toml`. No HTTP embed.

---

## 2. Combat death

```bash
cargo test -p sim-core --test combat overlay_parses_conflict_death -- --exact --nocapture
cargo test -p sim-core --test combat mock_conflict_death_overlay_no_attack_same_hash -- --exact --nocapture
cargo test -p sim-core --test combat health_zero_incapacitates -- --exact --nocapture
cargo test -p sim-core --test combat health_zero_combat_death_removes -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[conflict] death_enabled = true` |
| mock + death overlay, no Attack | same hash as overlay off |
| overlay off + health 0 | incapacitated; agent remains (M28) |
| death overlay + health 0 | agent removed; `CombatDeath`; no `Incapacitated` |
| `PROTOCOL_VERSION` | **5** |

CLI: `--conflict-death` implies `--conflict`. Overlay is not `ExperimentConfig`.

```toml
[conflict]
enabled = true
death_enabled = true
```

---

## 3. CI Win/mac

```bash
cargo test -p sim-core --test ci github_actions_workflow_exists -- --exact --nocapture
```

| Test | Success |
|---|---|
| workflow file | `.github/workflows/test.yml` lists ubuntu/windows/macos and the §0 cargo test packages |

The workflow runs `cargo test -p sim-core`, `-p shared`, `-p sim-cli --test net`, `-p viewer` (loopback / headless). Viewer crate tests are command/net helpers — **no imgui window** in CI (runners may have no display). No network, no live LLM.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M28 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --conflict --conflict-death --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 29; barrier 7; checkpoint 9; ci 1; combat 12; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 18; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 embeddings `--exact` ×5 | **pass** | off = recency order; similar text preferred; vectors not hashed; flag true same `state_hash` |
| §2 combat death `--exact` ×5 | **pass** | overlay parse; mock same hash; M28 incapacitate; CombatDeath removes; PROTOCOL 5 |
| §3 workflow `--exact` ×1 | **pass** | ubuntu/windows/macos + §0 packages in `.github/workflows/test.yml` |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--conflict-death` overnight | **not run** | not asked |
