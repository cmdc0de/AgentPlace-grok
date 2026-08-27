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

If they never Eat / Drink / Rest, at **tick 200** hunger is **70**, thirst **50**, energy **84**. That matches the default-config A/B reports (`hunger: mean 70.0`) when nobody refills.

Thirst hits the mock “thirsty” line first (see below) around tick **100**. Hunger hits it around tick **167**. With M9 cutoffs the mock should `Drink` well before tick-400 death if water is in range.

## When an agent is treated as hungry / thirsty / tired

These thresholds are what the **mock policy** uses (`policy.rs`). The LLM prompt sees the raw numbers; it is not forced to the same cutoffs.

| Label | Condition (display scale) | Millipoints |
|---|---|---|
| **Thirsty** | thirst **< 75** (`thirst_max * 3 / 4`) | < 7500 |
| **Hungry** | hunger **< 75** (`hunger_max * 3 / 4`) | < 7500 |
| **Tired** | energy **< 33.3** (`energy_max / 3`) | < 3333 |

Priority: **thirst → (storage-aware hunger) → tired**. Thirsty agents `Drink` if adjacent to water, else move toward it. Tired agents `Rest`.

Without a storage goal, **hungry** still means Eat / Retrieve / Gather / Hunt / Fish (75% cutoff). **With** a storage goal (`keep the shared storage stocked`):

- Eat / Retrieve only when hunger **< 50%**
- Store food to the **cell crate** whenever legal
- Gather (and walk toward plants) to stock that crate even at full hunger
- Pack leftover food into a Basket backpack when legal

M9 moved the hunger/thirst cutoffs from 50% to 75% so default-decay mock agents `Drink` before tick-400 thirst-death (thirsty from tick ~100). M11 keeps that, and only restores the 50% Eat line when the coop storage goal is on so surplus actually hits the crate.

Reports: `hunger: … (below half: N)` counts agents with display hunger **< 50**.

## How they refill

| Action | Effect |
|---|---|
| **Drink** | Thirst set to **max** (not a partial sip). Must be adjacent to fresh water. |
| **Eat** | Hunger += species `nutrition` (clamped to max). Berry 20, herb 15, mushroom 12, nightshade 10, hare 20, perch 15 (config defaults). Does **not** auto-eat after Gather. |
| **Rest** | Energy += 0.4 per tick (clamped to max). |
| Toxic / allergenic Eat | Also illness (`illness_ticks`), extra energy drain (−8.0 on that eat), toxic_events++. |
| **Shout** | Energy −5.0 if they can pay it; otherwise the utterance is not a shout. |
| **Store / Retrieve / Transfer** | Energy `qty × unit_weight × haul`. Source is **pockets** (haul **0.4**) or the **pack** (haul **0.1**) when the item is packed. Unaffordable ⇒ not legal. Stone 3.0, wood 1.5, food 0.5, tools 2.0, fiber 0.4. |
| **Pack / Unpack** | Pockets ↔ worn Basket pack (8 slots, weight 25.0). Haul **0.1**. Cannot pack the Basket itself. |
| **Move** | Extra cargo cost `loose_weight × 0.4 × 0.05 + pack_weight × 0.1 × 0.05` (display per cell). Unaffordable ⇒ not legal. Empty pockets still use the energy-0 50% fail. Rest still regenerates. |

Shared **land-cell containers** (one per cell): slot cap 16 and weight cap 80.0. Empty containers despawn. Viewer: brown **crate** on the cell; darker **satchel** on the agent capsule whenever they hold a Basket (even if the pack is empty). `/give ID ITEM QTY` in the in-process viewer is a hash-sensitive cheat (no energy). Giving a `basket` enables the pack.

Removing the last Basket **Unpacks** the pack into pockets; leftover that will not fit is **Stored** on the current land cell if the crate has room; otherwise Transfer/Store of that last Basket is not legal (nothing is voided).

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

| Tick | Hunger | Thirst | Energy | Mock thirsty? | Mock hungry (no storage goal)? |
|---|---|---|---|---|---|
| 0 | 100 | 100 | 100 | no | no |
| 100 | 85 | 75 | 92 | no (need **< 75**) | no |
| 101 | 84.85 | 74.75 | 91.92 | **yes** | no |
| 167 | 74.95 | … | … | yes | **yes** |
| 200 | 70 | 50 | 84 | yes | yes |
