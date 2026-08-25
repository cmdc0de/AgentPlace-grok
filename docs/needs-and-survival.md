# Needs: hunger, thirst, energy (as implemented)

TOML uses a **0–100 display scale**. The sim stores **millipoints** (`value × 100`, max 10_000). Reports print both: `hunger 70.0 (7000)` means 70 on the config scale.

Defaults below are [`configs/default.toml`](../configs/default.toml) `[needs]`.

## Config

```toml
[needs]
hunger_max = 100.0
hunger_decay_per_tick = 0.15
thirst_max = 100.0
thirst_decay_per_tick = 0.25
energy_max = 100.0
energy_decay_per_tick = 0.08
energy_regen_while_resting = 0.4
death_enabled = true           # hunger or thirst at 0 → agent removed (`Died` event)
```

`[agents] start_with_basic_needs = true` → every agent **starts at max** hunger, thirst, and energy.

## When the bars move

At the **start of every tick** (`world_step`), before agents act:

| Need | Each tick | Full → 0 if they never refill |
|---|---|---|
| Hunger | −0.15 | 100 / 0.15 ≈ **667 ticks** |
| Thirst | −0.25 | 100 / 0.25 = **400 ticks** |
| Energy | −0.08 (−0.16 if `illness_ticks > 0`) | 100 / 0.08 = **1250 ticks** (healthy) |

If they never Eat / Drink / Rest, at **tick 200** hunger is **70**, thirst **50**, energy **84**. That matches the default-config A/B reports (`hunger: mean 70.0`).

Thirst hits the mock “thirsty” line first (see below). Hunger hits it around tick **334** (50.0). Until then the mock often does **not** Gather/Eat, so veg consumed can stay 0.

## When an agent is treated as hungry / thirsty / tired

These thresholds are what the **mock policy** uses (`policy.rs`). The LLM prompt sees the raw numbers; it is not forced to the same cutoffs.

| Label | Condition (display scale) | Millipoints |
|---|---|---|
| **Thirsty** | thirst **< 50** (`thirst_max / 2`) | < 5000 |
| **Hungry** | hunger **< 50** (`hunger_max / 2`) | < 5000 |
| **Tired** | energy **< 33.3** (`energy_max / 3`) | < 3333 |

Priority: **thirst → hunger → tired**. Thirsty agents `Drink` if adjacent to water, else move toward it. Hungry agents `Eat` if they have food, else Gather/Hunt/Fish/move toward food. Tired agents `Rest`.

Reports: `hunger: … (below half: N)` counts agents with display hunger **< 50**.

## How they refill

| Action | Effect |
|---|---|
| **Drink** | Thirst set to **max** (not a partial sip). Must be adjacent to fresh water. |
| **Eat** | Hunger += species `nutrition` (clamped to max). Berry 20, herb 15, mushroom 12, nightshade 10, hare 20, perch 15 (config defaults). Does **not** auto-eat after Gather. |
| **Rest** | Energy += 0.4 per tick (clamped to max). |
| Toxic / allergenic Eat | Also illness (`illness_ticks`), extra energy drain (−8.0 on that eat), toxic_events++. |
| **Shout** | Energy −5.0 if they can pay it; otherwise the utterance is not a shout. |

`resource_multiplier` (incentives) scales **food gather qty** and **eat nutrition** only. It does not change decay.

## At zero

When a need hits **0 millipoints**:

- Gather / Hunt / Fish / Farm / Craft **skill chance −25** (clamped 5–95%).
- Illness separately **−20** on those rolls.
- **Move** with energy 0: 50% chance the move fails (`Wait`).

`death_enabled = true` (default config): after decay, any agent with **hunger = 0 or thirst = 0** is removed and a `Died` event is logged. Energy 0 does not kill. Living population is what the board majority uses. `pause_when_empty = false` keeps ticking an empty world.

With default decay and no Drink, **thirst hits 0 at tick 400**; hunger at ~667. A 200-tick A/B does not reach death.

## Viewer / report

- Inspector bars are millipoints / 10_000.
- Agent list shows hunger on the 0–100 scale.
- `--report` world hunger mean/min/max and per-agent needs.

## Quick sanity numbers (default config, no eating)

| Tick | Hunger | Thirst | Energy | Mock thirsty? | Mock hungry? |
|---|---|---|---|---|---|
| 0 | 100 | 100 | 100 | no | no |
| 200 | 70 | 50 | 84 | no (thirst == 50, need **< 50**) | no |
| 201 | 69.85 | 49.75 | 83.92 | **yes** | no |
| 334 | 49.9 | … | … | yes | **yes** |
