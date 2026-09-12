# M33 test plan — see each new feature

Walkthrough for [`M33-plan.md`](M33-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (household crates, culture, reflect importance). Next slice: [`M48-plan.md`](M48-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test population`, `--test pipeline`; unit tests under `--lib` need the module path.

**Success for the slice:** overlay off, idle mock hash unchanged. Household home minted on PairBond when `--household-crates`; members Store/Retrieve at that cell within Chebyshev 1; outsider cannot via this rule; `--load` restores the home map. `--culture` assigns founder ids 1..=count; children copy a parent; `--load` does not re-roll. `--llm-reflect-importance` mock-skips; Custom JSON rewrites a named memory; record+replay match. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Household crates

```bash
cargo test -p sim-core --test population overlay_parses_sheet_and_population -- --exact --nocapture
cargo test -p sim-core --test population household_crates_off_no_pairbond_same_hash -- --exact --nocapture
cargo test -p sim-core --test population pair_bond_without_crates_overlay_mints_no_home -- --exact --nocapture
cargo test -p sim-core --test population pair_bond_home_member_store_outsider_cannot -- --exact --nocapture
cargo test -p sim-core --test population household_home_load_does_not_remint -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `household_crates`, `culture`, `culture_count` |
| overlay on, no pair-bond | same hash as overlay off; empty home map |
| pair-bond without overlay | no home minted |
| pair-bond + overlay | both share a home cell; member Stores there; outsider cannot via this rule |
| `--load` | home map restored; enable does not re-mint |

CLI: `--household-crates` does **not** imply `--reproduction`. Inspector / Observation: `home (x,y)` when set.

---

## 2. Culture

```bash
cargo test -p sim-core --test population culture_off_zero_same_hash -- --exact --nocapture
cargo test -p sim-core --test population culture_founders_child_copies_load_no_reroll -- --exact --nocapture
```

| Test | Success |
|---|---|
| culture off | culture 0; same hash as a twin run |
| `--culture` | founders 1..=count; hash ≠ off; child copies lower-id parent; `--load` does not re-roll |

Inspector / Observation: `culture N` when ≠ 0.

---

## 3. Reflect importance

```bash
cargo test -p sim-core --test pipeline overlay_parses_reflect_importance -- --exact --nocapture
cargo test -p sim-core --test pipeline mock_importance_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test pipeline custom_importance_rewrites_named_memory -- --exact --nocapture
cargo test -p sim-core --test pipeline record_replay_importance_same_hash -- --exact --nocapture
cargo test -p sim-core --test pipeline old_jsonl_importance_call_round_trips -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[llm] reflect_importance` |
| mock | same hash as overlay off |
| Custom JSON | named memory importance changes |
| record+replay | hashes match; replay does not call Custom |
| JSONL `call` | `"importance"` |
| `PROTOCOL_VERSION` | **5** |

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M32 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --household-crates --culture \
  --llm-reflect-importance --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 32; barrier 7; checkpoint 9; ci 1; combat 15; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 23; population 18; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 household `--exact` ×5 | **pass** | parse; off same hash; no home without overlay; member Store at home; load no re-mint |
| §2 culture `--exact` ×2 | **pass** | off culture 0; founders 1..=4; child copies; load no re-roll |
| §3 importance `--exact` ×6 | **pass** | overlay parse; mock skip; Custom rewrite; record+replay; JSONL call; PROTOCOL 5 |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
