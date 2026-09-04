# M32 test plan — see each new feature

Walkthrough for [`M32-plan.md`](M32-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (kin_of incentives, household, aging). Next slice: [`M38-plan.md`](M38-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test population`; unit tests under `--lib` need the module path (`incentive::tests::…`).

**Success for the slice:** overlay off, idle mock hash unchanged. `kin_of:N` hits living relatives not N; `kin_of:nope` load-errors; missing agent empty. Pair-bond mints a household; child inherits; `household:H` is members only. `--aging` stamps founders, increments, children start at 0 and cannot PairBond/Reproduce/Attack; `--load` does not re-stamp. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. kin_of

```bash
cargo test -p sim-core --lib incentive::tests::kin_of_and_household_parse -- --exact --nocapture
cargo test -p sim-core --lib incentive::tests::kin_of_nope_is_load_error -- --exact --nocapture
cargo test -p sim-core --test population kin_of_scope_after_pair_bond_and_birth -- --exact --nocapture
cargo test -p sim-core --test population kin_of_missing_agent_empty_scope -- --exact --nocapture
```

| Test | Success |
|---|---|
| parse | `kin_of:0` / `kin_of:agent_3` / `household:1` load |
| `kin_of:nope` | load error |
| after PairBond+birth | agent 0 **not** in scope; partner and child are |
| missing agent | empty scope (1.0× food) |

Example: `configs/incentives/kin-bonus.toml`. Do not change `coop.toml`.

---

## 2. Household

```bash
cargo test -p sim-core --lib incentive::tests::household_nope_is_load_error -- --exact --nocapture
cargo test -p sim-core --test population pair_bond_mints_household_child_inherits -- --exact --nocapture
```

| Test | Success |
|---|---|
| `household:nope` | load error |
| pair-bond | both share one household id; child inherits; outsider not in `household:H` |

Inspector / Observation: `household #H` when set.

---

## 3. Aging

```bash
cargo test -p sim-core --test population overlay_parses_sheet_and_population -- --exact --nocapture
cargo test -p sim-core --test population aging_off_zero_same_hash -- --exact --nocapture
cargo test -p sim-core --test population aging_stamps_founders_gates_child -- --exact --nocapture
cargo test -p sim-core --test population aging_load_does_not_restamp_founder_age -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `aging`, `childhood_ticks`, `founder_age_ticks` |
| aging off | age 0; same hash as a twin run |
| `--aging` | founders at floor; child 0; child cannot Attack/PairBond/Reproduce; tick increments |
| `--load` | enable_aging does not reset founder age |
| `PROTOCOL_VERSION` | **5** |

CLI: `--aging` does not imply `--sheet`.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M31 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --aging \
  --incentives configs/incentives/kin-bonus.toml --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 32; barrier 7; checkpoint 9; ci 1; combat 15; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 18; population 12; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 kin_of `--exact` ×4 | **pass** | parse; nope load error; relatives in scope not N; missing empty |
| §2 household `--exact` ×2 | **pass** | nope load error; mint+inherit; outsider out of scope |
| §3 aging `--exact` ×5 | **pass** | overlay parse; off age 0; child gated; load no re-stamp; PROTOCOL 5 |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--aging` overnight | **not run** | not asked |
