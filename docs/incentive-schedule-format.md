# Incentive schedule TOML (M8)

A schedule is **one file**, not one file per incentive. The file is an array of `[[incentives]]` tables. Several can be active at once; unknown `type` values fail at load.

This file is an **overlay**. It is **not** part of `ExperimentConfig` (so it does not change checkpoint `config_hash`). Pass it with `--incentives`, `--inject`, viewer `/inject PATH`, or:

```toml
# in the experiment config (parsed as overlay, ignored by sim-core config)
[incentives]
schedule = "configs/incentives/coop.toml"
```

Worked example: [`configs/incentives/coop.toml`](../configs/incentives/coop.toml). Hidden banner example: [`configs/incentives/hidden-bonus.toml`](../configs/incentives/hidden-bonus.toml). Coalition targeting: [`configs/incentives/coalition.toml`](../configs/incentives/coalition.toml) (`applies_to = "supporters_of:proposal_N"`, M14). Long-term vocabulary: [`memory-goals-incentives-spec.md`](memory-goals-incentives-spec.md) §3 (`type = "visibility_modifier"` as an *effect* is still a load error; M12 is the `visibility` *field*).

## File shape

```toml
[[incentives]]
id = "early_cooperation_bonus"   # required, unique in this file
description = "optional text"
start_tick = 0                   # first sim tick is 1; 0 means “from the first tick”
end_tick = 3000                  # optional; omit = stays on
applies_to = "all"               # see scope below

[[incentives.effects]]
type = "goal_injection"
goal_text = "keep the shared storage stocked"
scope = "personal"
priority = 0.7

[[incentives]]
id = "another_one"
start_tick = 40
applies_to = "agent:0"
[[incentives.effects]]
type = "influence_factor_delta"
delta = 0.15
```

`--incentives path.toml` applies the whole file at tick 0. `--inject` / `/inject` **replaces** the current schedule with that file’s contents (it does not merge files).

## Per-incentive fields

| Field | Required | Default | Meaning |
|---|---|---|---|
| `id` | yes | — | Unique string in this file. |
| `description` | no | `""` | Copied into `IncentiveApplied` events. |
| `start_tick` | no | `0` | Inclusive. Active when `sim.tick >= start_tick`. |
| `end_tick` | no | omitted | Inclusive upper bound. Omit for permanent. |
| `applies_to` | no | `"all"` | Who the effects hit. |
| `visibility` | no | `"public"` | `"public"` or `"hidden"`. Hidden omits the banner from Observation/prompt; **effects still apply**. Injected goals stay visible. Researcher inspector / `IncentiveApplied` / `--compare` still list the id. Unknown values are a load error. |
| `effects` | no | `[]` | One or more `[[incentives.effects]]` tables. |

### `applies_to`

| Value | Who |
|---|---|
| `all` | Every agent. |
| `agent:N` | Agent id `N` (e.g. `agent:0`). |
| `archetype:name` | Agents whose abilities/personality still match that named `[agents.archetypes]` entry. |
| `supporters_of:proposal_N` | Agents currently in that proposal’s `supporters` set (also `supporters_of:N`). Missing id → empty scope. `supporters_of:nope` → load error. Per-tick effects follow the live set; one-shots (`goal_injection`, `relationship_delta`, `influence_factor_delta`) still apply only at incentive start. |

## Effect tables (`[[incentives.effects]]`)

Every effect **must** have `type`. Extra unknown types are a load error.

### `resource_multiplier`

| Field | Required | Notes |
|---|---|---|
| `resource` | yes | `"food"` (veg gather + eat), `"animal"`, or `"fish"`. |
| `multiplier` | yes | e.g. `1.4` → 1.4× yield / nutrition (integer millipoint scale internally). |
| `condition` | no | `""` (always) or `"has_supported_public_goal"` (agent is in any proposal’s supporter set). |

### `influence_factor_delta`

| Field | Required | Notes |
|---|---|---|
| `delta` | yes | Added to `influence_factor` while active (`0.15` → +15 millipoints). Reverted when the incentive ends. |

### `memory_importance_boost`

| Field | Required | Notes |
|---|---|---|
| `kind` | yes | Memory kind name, case-insensitive: `Observation`, `Sickness`, `ToxinFact`, `Utterance`, `Action`, `Proposal`, `Interaction`, `Norm`. |
| `multiplier` | yes | Scales `importance` on write (e.g. `1.8`). |

### `goal_injection`

| Field | Required | Notes |
|---|---|---|
| `goal_text` | yes | If the agent already has this text, priority is raised to `max(existing, priority)`. |
| `scope` | no | `"personal"` (default, scoped agents) or `"public"` (all agents). |
| `priority` | no | `0.0`–`1.0` (default `0.5`) → stored as 0–100. |

Applied once when the incentive **starts**.

### `proposal_threshold_modifier`

| Field | Required | Notes |
|---|---|---|
| `delta` | yes | Added to `[proposals] default_acceptance_threshold` (e.g. `-0.1`). Clamped to `[0.01, 1.0]`. |

### `relationship_delta`

| Field | Required | Notes |
|---|---|---|
| `trust` | no | `0.0` | Applied once at start to every in-scope agent vs every other agent (`0.1` → +10 millipoints). |
| `affinity` | no | `0.0` | Same scale. |
| `respect` | no | `0.0` | Same scale (`70.0` → +7000). |
| `toward` | no | `""` | Omit / empty = every other agent. `toward = "agent:N"` = that id only (skip self). `toward = "nope"` is a load error. Missing agent at start is a no-op for that edge. |

### Not an effect type

`type = "visibility_modifier"` is still a **load error**. Use the `visibility` field on `[[incentives]]` instead (`public` / `hidden`). Covert payoff A/B should use mechanical effects without `goal_injection` (goal text remains in Observation even when the banner is hidden).

Vote **tally** is not an incentive effect. Overlay on the experiment TOML (like `[storage]`), not in this schedule file:

```toml
[voting]
weight = "equal"       # default; omit = equal (one living agent = 1)
# weight = "influence" # max(influence_factor, 1) millipoints
# weight = "respect"   # max(incoming positive respect from other living agents, 1)
```

Unknown `weight` is a load error. Pair `influence` with [`configs/incentives/leadership.toml`](../configs/incentives/leadership.toml) (`influence_factor_delta = 70.0` on `agent:0`) to A/B a kingmaker. Pair `respect` with [`configs/incentives/esteem.toml`](../configs/incentives/esteem.toml) (`relationship_delta` respect toward `agent:0`). Default respect 0 ⇒ same as equal. Do not put `[voting]` in shipping `configs/default.toml`.

## How a file is applied

1. **Tick 0 / `--incentives`:** schedule is loaded before ticks; effects whose window includes the current tick activate at the **start** of that tick.
2. **`--load` + `--inject`:** checkpoint restores world/agents; the new file **replaces** any packed schedule; next ticks use the new window.
3. **Live `/inject` or `InjectIncentive`:** same replace; needs `--allow-control`. Takes effect on the next tick (immediately if `start_tick` ≤ current tick).
4. Checkpoints pack the schedule TOML into `IncentiveState.entries` (`format_version` stays 2).

Baseline vs treated: omit the file (or pass an empty `incentives = []`) for hash A; pass a schedule for hash B.

## Reading an A/B after `--inject` at tick 100

`coop.toml` always injects the goal `keep the shared storage stocked` (check `--report` goals). Food 1.4× only applies if the agent has **supported** a proposal. Mock agents often have not, by tick 200 — veg consumed can stay 0 on both arms. Hunger at tick 200 with default decay and no eating is **70** on both arms (`docs/needs-and-survival.md`). The hash still diverges because of the injected goals + `IncentiveApplied` event.
