# M12 test plan — see each new feature

Walkthrough for [`M12-plan.md`](M12-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (Observation vs inspector, `--compare`, prompt). Next slice: [`M52-plan.md`](M52-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

**Success for the slice:** `visibility = "hidden"` omits the incentive id from Observation and the LLM prompt; effects and injected goals still apply; researcher inspector still lists hidden ids; same-seed mock public vs hidden (same effects) → **same `state_hash`**. `coop.toml` stays public. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
cargo test -p viewer
```

---

## 1. Parse `visibility`

```bash
cargo test -p sim-core --test incentives unknown_visibility_is_load_error -- --nocapture
cargo test -p sim-core --test incentives unknown_effect_type_errors -- --nocapture
cargo test -p sim-core --test incentives omitted_visibility_defaults_public -- --nocapture
cargo test -p sim-core incentive::tests::parses_hidden_visibility -- --nocapture
```

| Test | Success |
|---|---|
| `visibility = "maybe"` | load error |
| `type = "visibility_modifier"` | still a load error (not an effect) |
| omit field | treated as **public** (M11) |
| `visibility = "hidden"` | parses |

---

## 2. Hidden banner vs mechanics

```bash
cargo test -p sim-core --test incentives hidden_incentive_omitted_from_observation_effects_still_apply -- --nocapture
cargo test -p sim-core --test incentives public_incentive_appears_in_observation -- --nocapture
cargo test -p sim-core --test incentives public_vs_hidden_same_effects_same_hash -- --nocapture
cargo test -p sim-core --test incentives hidden_goal_injection_still_in_observation_goals -- --nocapture
```

| Test | Success |
|---|---|
| Hidden food bonus | id **not** in Observation.incentives; `resource_mult` still 1.4×; `IncentiveApplied` logged |
| Public twin | id **is** in Observation.incentives |
| Same seed, public vs hidden, mock | **same `state_hash`**; **different `prompt_hash`** |
| Hidden `goal_injection` | goal still in Observation.goals; mock still Stores |

### See a hidden overlay run

```bash
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/hidden-bonus.toml \
  --ticks 40 --llm mock --out-dir /tmp/m12-hidden --quiet
```

**Success:** exit 0. Copy the overlay, set `visibility = "public"`, run the same 40 ticks, then `--compare`. Mock **`state_hash` equal**. `coop.toml` 80-tick crate-fill is unchanged (public).

---

## 3. Prompt

```bash
cargo test -p sim-llm tests::prompt_contains_needs_and_incentive -- --nocapture
cargo test -p sim-llm tests::prompt_omits_incentive_when_observation_has_none -- --nocapture
```

**Success:** public id appears in `Active incentives`; empty Observation.incentives → `Active incentives: []` (no invented hidden id).

---

## 4. Viewer inspector (researcher)

```bash
cargo run -p viewer -- --config configs/default.toml
```

Then `/inject configs/incentives/hidden-bonus.toml` (in-process; needs control if remote). Inspector: `incentives (researcher)` line `hidden_food_bonus (hidden)`. Follow an agent: goals/prompt dump must **not** list that id as an active incentive banner.

Remote inject still requires `--allow-control`.

---

## 5. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M12-plan.md`](M12-plan.md), [`incentive-schedule-format.md`](incentive-schedule-format.md) (`visibility` field), README current slice **M12**.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test incentives` | 12/12 including hidden/public |
| 2 | `public_vs_hidden_same_effects_same_hash` | hashes equal |
| 3 | `prompt_omits_incentive_when_observation_has_none` | no banner |
| 4 | §2 hidden-bonus 40-tick run | exit 0 |

---

## Execution record

**Ran:** 2026-08-26, repo root. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` / sim-llm / sim-cli ab / viewer | **pass** | incentives 12/12; sim-llm 7/7; ab 3/3 |
| §1 parse / load error (named tests) | **pass** | `maybe` errors; `visibility_modifier` effect still errors; omit=public; `parses_hidden_visibility` |
| §2 named hidden/public tests | **pass** | omit banner; 1.4× still on; mock Stores with hidden goal |
| §2 unit `public_vs_hidden_same_effects_same_hash` | **pass** | state_hash equal; prompt_hash differs |
| §2 `sim-cli` hidden-bonus 40-tick | **pass** | `final_tick=40` `final_hash=9d80dd18…` |
| §2 `--compare` public twin vs hidden | **pass** | hashes **equal** (`9d80dd18…`); `hidden_food_bonus` A=true B=true; Move/Wait identical |
| §3 prompt tests | **pass** | `coop_food` in prompt; empty Observation → `Active incentives: []` |
| §4 inspector GUI | **not run** | no display automation |
| §5 `--help` | **pass** | `--incentives`, `--compare` listed |
