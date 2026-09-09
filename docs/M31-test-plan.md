# M31 test plan — see each new feature

Walkthrough for [`M31-plan.md`](M31-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (kinship, reproduction, D&D-like sheet). Next slice: [`M41-plan.md`](M41-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test population`; viewer tests live in the binary crate.

**Success for the slice:** overlay off, idle mock hash unchanged. `--sheet` rolls founder 3–18 and changes hash; ckpt round-trips. Overlay off: PairBond/Reproduce not legal. Mock + reproduction does not birth (same hash as sheet-only). PairBond + Reproduce: new agent, `Born`, parent/child/sibling, calculated child sheet. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Physical sheet

```bash
cargo test -p sim-core --test population overlay_parses_sheet_and_population -- --exact --nocapture
cargo test -p sim-core --test population sheet_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test population sheet_on_rolls_and_changes_hash_ckpt_round_trip -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[agents.sheet] enabled` / `[population] reproduction` |
| overlay off | unused sheet; same hash as a twin run |
| `--sheet` | founders 3–18; hash ≠ off; ckpt restores scores |

CLI: `--sheet`. Overlay is not `ExperimentConfig`.

---

## 2. Kinship + reproduction

```bash
cargo test -p sim-core --test population reproduction_overlay_off_not_legal -- --exact --nocapture
cargo test -p sim-core --test population mock_reproduction_overlay_no_custom_same_hash -- --exact --nocapture
cargo test -p sim-core --test population pair_bond_and_reproduce_writes_kin_and_calculated_sheet -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | PairBond/Reproduce not legal |
| mock + reproduction | same hash as sheet-only (no births) |
| PairBond + Reproduce | new agent; `Born`; parent/child/sibling; child scores around parent mix (not a fresh 3d6) |
| `PROTOCOL_VERSION` | **5** |

`--reproduction` implies `--sheet`. Mock never picks PairBond/Reproduce.

```toml
[agents.sheet]
enabled = true
[population]
reproduction = true
```

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M30 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Inspector: **sheet** (`STR 14 (+2)` …) and **family** when kinship is non-empty.

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 29; barrier 7; checkpoint 9; ci 1; combat 15; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 18; population 6; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 sheet `--exact` ×3 | **pass** | parse overlay; off same hash; 3–18 + ckpt |
| §2 kin/repro `--exact` ×4 | **pass** | not legal off; mock=sheet-only hash; Born+kin+calculated; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--sheet --reproduction` overnight | **not run** | not asked |
