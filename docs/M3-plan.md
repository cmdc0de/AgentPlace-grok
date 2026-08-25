# M3 — Survival, observation, speech, abilities, and LLM action selection

**Status:** implemented (mock + optional live OpenAI-compatible LLM).  
**Depends on:** M2 complete (`docs/M2-plan.md`)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md`, `simulation-and-agents-spec.md`, `deterministic-seeding-design.md`, `medium-priority-specs.md` (communication + error handling)

## Context

M2 delivered a restorable world: heightmap, water / vegetation / mineral **presence bits**, land-only spawn, versioned checkpoints (`AGTN` / `format_version = 1`), Markdown summaries, JSONL events, and a Bevy viewer that can `--load` a checkpoint. Agents are still `(id, x, y)` with a seeded random walk. Vegetation has no species. There are no animals, fish, needs, inventory, observation, or LLM.

The original M2-plan deferred LLM and treated M3 as “needs + Gather/Drink only.” That split is revoked. M3 is the spec’s **survival + decision vertical slice**: limited senses, needs, abilities, toxicity, **speech** (so a sick agent can warn others), and a provider-agnostic LLM that may only pick from a core-computed legal action list.

M3 does **not** add the public proposal board, imgui, incentives, or network transports. Those remain later milestones.

## Goal

A researcher can run a seeded population that:

1. **Sees a limited world** (base ranges × per-agent `perceptiveness`).
2. **Gets hungry, thirsty, and tired** (integer needs; Rest regenerates energy).
3. **Uses skills and tools** to Gather, Drink, Eat, Hunt, Fish, Farm, and Craft.
4. **Can be poisoned** — some species are toxic to everyone, some allergenic to individuals.
5. **Can speak** in the same tick as a primary action (length-capped; hearing range × listener perceptiveness) so toxicity and resource knowledge can spread.
6. **Chooses actions** via a deterministic mock policy (tests / CI) or an LLM (Ollama locally, OpenAI-compatible frontier such as xAI). The model never executes free text against the world; utterances are data, not commands.
7. Still **checkpoint-restores** bit-identically on the same binary/OS when the mock (or a replay file) is used.

Same seed + mock LLM ⇒ same `state_hash`. Live LLM is optional and not a CI requirement.

## In scope

| Area | M3 meaning |
|---|---|
| **Observation** | Per-tick `Observation` from Chebyshev vision/hearing/identity ranges. Two agents on the same cell can see different things. `full_information = true` debug flag. |
| **Needs** | Hunger, thirst, energy as millipoints. Decay every tick. At 0: movement/action penalties. Default config now `death_enabled = true` (hunger or thirst at 0 removes the agent). Field-level write-up: [`needs-and-survival.md`](needs-and-survival.md). |
| **Gather / Drink / Eat** | Gather food or wood from a vegetation species on the standing cell or a 4-neighbour. Drink from fresh water in-world. Eat consumes inventory food (Gather does not auto-eat). |
| **Toxicity** | Each food species is `safe`, `toxic` (hurts everyone), or `allergenic` (hurts agents with that allergy tag). Eat applies nutrition and possible illness. |
| **Abilities** | Per-agent 0–100 skills: `gather`, `hunt`, `fish`, `farm`, `craft`. Sampled at spawn from archetypes. Gate success chance and yield. |
| **Hunt / Fish** | Animal counts on land cells, fish counts on water. Hunt/Fish decrement a local count; skill and tools change success. Slow regen from the `event` RNG stream. |
| **Farm** | Plant a known crop species on adjacent empty land; harvestable after `grow_ticks`. Not irrigation, seasons, or plots. |
| **Tools** | Three recipes: **basket** (better gather), **spear** (better hunt), **fishing_rod** (better fish). Costs: wood + stone (minerals) and/or plant fiber. Bare-handed hunt/fish allowed at low success. Failed craft does not consume ingredients. |
| **Speech** | Free **secondary** action (`medium-priority-specs.md` §1). Primary action + optional utterance in the same tick. Directed (named listeners in identity range) or broadcast/shout. Hard `max_message_length` (truncate). Range = hearing × listener perceptiveness; shout costs energy and uses `shout_range_multiplier`. Overhearing on. Writes `Interaction` memories; does **not** yet update trust/affinity. No proposal board, no gestures. |
| **LLM** | One structured call per agent per tick: primary action + optional `speak`. Trait + mock + replay in/near `sim-core`. HTTP in a new `sim-llm` crate. Timeout → `Wait` and **no speech** (no retry). Malformed → retry then `Wait`. |
| **Thin memory** | Capacity-capped structured entries (resource sighting, “ate X and got sick”, heard utterance, last action). Sickness and “X is toxic” memories protected. Not the full hybrid store, embeddings, or relationship summaries. |
| **Metrics** | Per-agent and world consumption of vegetation / animal / fish plus toxic-consumption events. Surfaces in Markdown summaries. |
| **Personality** | Enough for observation and the LLM prompt: `perceptiveness`, Big-Five as 0–100, free-form `traits`, allergy tags. Not social influence. |

## Out of scope (M4+)

| Later | What |
|---|---|
| **M4** | Done — [`M4-plan.md`](M4-plan.md) (public board, Propose/Support/Oppose, structured adopted rules, food-economy report). |
| **M5** | Done — [`M5-plan.md`](M5-plan.md) (hybrid memory, relationships, influence, decision JSONL, viewer legend). |
| **M6** | Done — [`M6-plan.md`](M6-plan.md) (imgui research UI, agent-POV fog-of-war matching `Observation`, slash-command console). |
| **M7** | Done — [`M7-plan.md`](M7-plan.md) (TCP / WebSocket, hash-neutral attach). |
| **M8** | Done — [`M8-plan.md`](M8-plan.md) (incentive A/B + timing + death). |
| **M9** | [`M9-plan.md`](M9-plan.md) — LLM prompts, replay, `--compare`, mock drink-before-death. |
| Later still | Attack / Flee / death, seasons, complex crafting, shared stockpiles, Transfer/Store, lie detection, dialects. |

## Key decisions

1. **M3 includes LLM.** Tests never require a network: default `[llm] provider = "mock"`.
2. **Integer hashed state.** Needs are millipoints (`u32`, 0..=10_000 maps to config `100.0`). Skills and personality facets are `u8` 0–100. Config TOML stays f64 as in the specs and converts at load. Do not put f64 into `state_hash`.
3. **1 cell = 1 world unit.** `effective_vision = round(base_vision_range * (0.5 + perceptiveness))` as Chebyshev range on the grid. Same pattern for hearing and identity range.
4. **Cell tags, not entities.** Vegetation is a species id per cell (0 = none). `animals: Vec<u8>` and `fish: Vec<u8>` are counts. Trees are a vegetation species that yields wood, not calories. M2 mineral bits remain stone for crafting.
5. **Legal actions are computed in core.** The LLM or mock only **selects**. Core executes and treats anything not in the list as `Wait` (logged).
6. **`sim-core` has no HTTP.** `LlmClient` + `MockLlm` + `ReplayLlm` stay with the simulation. New crate `sim-llm` implements OpenAI-compatible chat:
   - local: Ollama (default `http://localhost:11434`)
   - frontier: xAI / SpaceXAI (`https://api.x.ai/v1`, `XAI_API_KEY`). Resolve the current chat model name from https://docs.x.ai/developers/models at implementation time.
7. **Timeout = Wait, no retry** (`medium-priority-specs.md`). Malformed JSON retries `max_retries` at lower temperature, then Wait. Unreachable provider follows the malformed path, then Wait.
8. **Checkpoint `format_version = 2`.** Refuse v1. M2 vegetation has no species; a fake migration would lie about the world.
9. **Farm and tools are thin.** One plant-on-empty-land action; three recipes; no durability.
10. **Eat is a primary action.** Gather fills inventory; Eat spends food. Drink is in-world (no water bottle required in M3).
11. **Speech is a free secondary action, not a primary.** An agent may `Gather` and speak in the same tick. The LLM returns both in one JSON object (no second dialogue round-trip in M3 — keeps tick cost to one call). Text longer than `max_message_length` is truncated and logged.
12. **Utterances become audible on the following tick.** Observation is built at the start of the agent’s step from the event log (speech in range during the previous tick). Avoids mid-tick observation mutation and first-mover hearing from turn order.
13. **Speech is not a world command.** “Don’t eat mushrooms” writes a memory for listeners; it does not change species toxicity. Listeners may then avoid `Eat` / `Gather` of that species if the mock/LLM consults memory. Lies are possible (no detection in M3).
14. **One shared language.** No dialects or private codes.

## World additions

Replace binary vegetation with a species table. Defaults ship a small set (2 safe foods, 1 toxic, 1 allergenic, 1 tree) so a config-less run is still interesting.

```toml
[[world.species.vegetation]]
id = "berry_bush"
yield = "food"            # "food" | "wood"
nutrition = 20            # hunger restored per unit (config f64 → millipoints)
toxicity = "safe"         # "safe" | "toxic" | "allergenic"
allergen_tag = ""
wood_yield = 0
fiber_yield = 1
grow_ticks = 40           # used when this species is farmed

[[world.species.vegetation]]
id = "mushroom"
yield = "food"
toxicity = "toxic"

[[world.species.vegetation]]
id = "tree"
yield = "wood"
wood_yield = 3
```

Animal and fish species follow the same toxicity/nutrition pattern. Place counts after M2 water/veg/mineral layers, using the `world` RNG, with minima + density like M2 resources.

Growing crops: a planted cell stores `(species_id, planted_tick)` until `tick >= planted_tick + grow_ticks`, then it becomes a normal vegetation cell of that species.

## Agent internals

```
Agent
├── id, x, y
├── needs { hunger, thirst, energy }     # millipoints
├── inventory: BTreeMap<ItemId, u32>     # capacity from config
├── abilities { gather, hunt, fish, farm, craft }   # 0–100
├── personality { facets, traits, perceptiveness, allergy tags }
├── memory: Vec<MemoryEntry>             # cap = default_memory_capacity
└── consumption { veg, animal, fish, toxic_events }
```

`start_with_basic_needs = true` → needs at max. Archetypes sample abilities and personality (weights already sketched in `simulation-and-agents-spec.md` §5.1). If no archetypes are configured, use global defaults.

Illness: a short integer timer. While > 0, extra energy drain and lower action success. Toxic hits everyone; allergenic hits only matching `allergy tags`.

## Decision loop (replaces random walk)

Per agent, in the existing turn-order shuffle:

1. Decay needs; tick illness.
2. Build `Observation`: own state, tiles in vision, agents in vision (identity only inside identity range), **speech heard last tick** inside effective hearing range, memories, **legal primary actions**.
3. Select: `LlmClient::choose(call_seed, observation)` or mock policy → primary action + optional `speak`.
4. Execute primary; if `speak` present, validate length/range/targets and emit `SimEventKind::Speak` (truncate over-length; illegal targets dropped, speech still may broadcast to whoever is valid).
5. Write memories: own action, sickness, and — for speaker + each hearer — an `Interaction` / utterance entry. Hearers next tick.
6. Evict if over capacity (importance × recency; sickness and “X is toxic” memories protected).

### Legal action vocabulary (M3)

- `Wait` / `Rest`
- `MoveRelative { dx, dy }` with `|dx|+|dy| = 1`, land only
- `Gather { species }` — veg cell in Chebyshev 1
- `Drink` — water in Chebyshev 1
- `Eat { item }` — item in inventory, food
- `Hunt` — animal count > 0 on standing cell or Chebyshev 1
- `Fish` — fish count > 0 on a neighbouring water cell
- `Farm { species }` — empty land in Chebyshev 1, seeds/food of that species in inventory
- `Craft { recipe }` — `basket` | `spear` | `fishing_rod` if ingredients present

Secondary (does not consume the primary slot; always *legal* as a companion, subject to length/energy):

- `Speak { to: Directed(ids) | Broadcast, shout: bool, text }`

No `Propose`, `Attack`, `Transfer`, or gestures in M3. Direct `to` ids the speaker cannot currently identify are ignored. Shout with insufficient energy degrades to normal broadcast (logged), it does not fail the primary action.

### Mock policy (tests / default)

Deterministic given observation + the agent RNG stream:

1. If thirst high and water in view → `Drink` if adjacent else `Move` toward it.
2. Else if hunger high → `Eat` if food in inventory else `Gather` / `Hunt` / `Fish` / `Move` toward food.
3. Else if energy low → `Rest`.
4. Else `Wait` or a cheap `Move`.
5. Secondary speak (canned strings only, so hashes stay stable): if memory has “species X is toxic” and at least one other **identified** agent is in hearing range and we have not already broadcast that fact in the last `warn_cooldown_ticks` (default 10), attach `Speak { Broadcast, "X is toxic" }`.

## Observation

From `decision-observation-llm-economy-metrics-spec.md` §2:

- `effective_vision = round(base_vision_range * (0.5 + perceptiveness))`
- `effective_hearing = round(base_hearing_range * (0.5 + perceptiveness))` (listeners). Shout uses `hearing * shout_range_multiplier` before the listener’s perceptiveness factor, or equivalently range-check in world units then apply listener multiplier — pick one and keep it integer. **Use:** shout radius = `round(base_hearing_range * shout_range_multiplier * (0.5 + listener.perceptiveness))`.
- Identity range follows the vision pattern.
- `resource_discovery = "must_be_in_range"` by default.
- `full_information = true` overrides all ranges (debug / some experiments) and makes all last-tick speech globally audible.

An observation given to the agent (and later to agent-POV UI) includes own needs/inventory/allergies, nearby terrain and resources, other agents (named or “unknown figure”), **utterances heard last tick** (speaker id if identified else “unknown speaker”, text, shout flag), and the legal primary-action list. Public board is empty in M3.

## LLM contract (action + optional speech)

One call per agent per tick. Prompt: persona, traits, perceptiveness, needs, observation summary (including heard speech), legal **primary** actions as JSON schema. Response:

```json
{
  "action": "Gather",
  "target": "berry_bush",
  "speak": { "to": "broadcast", "shout": false, "text": "mushrooms are toxic" },
  "reasoning": "optional"
}
```

`speak` may be omitted or `null`. `to` is `"broadcast"` or an array of agent ids. `reasoning` is **log-only** in M3. Utterance text is stored as a memory for hearers; it is not executed.

Do **not** add a second dialogue-temperature call in M3. Spec call-type “Dialogue” waits until we care about richer conversation (M4+). `dialogue_temperature` may exist in config unused.

Call seed: `derive_seed(llm_base, format!("tick_{}_agent_{}_call_{}", tick, id, n))`.

Replay JSONL: `{tick, agent, call_seed, prompt_hash, response}` so a live run can be reproduced without the provider.

Config sketch:

```toml
[observation]
base_vision_range = 12.0
base_hearing_range = 18.0
base_agent_identity_range = 8.0
full_information = false

[needs]
hunger_max = 100.0
hunger_decay_per_tick = 0.15
thirst_max = 100.0
thirst_decay_per_tick = 0.25
energy_max = 100.0
energy_decay_per_tick = 0.08
energy_regen_while_resting = 0.4
death_enabled = false

[communication]
speech_is_free_action = true
base_speech_range = 18.0
shout_range_multiplier = 1.8
shout_energy_cost = 5.0
max_message_length = 200
allow_overhearing = true
warn_cooldown_ticks = 10        # mock policy only

[llm]
provider = "mock"                 # "mock" | "ollama" | "openai_compatible"
base_url = "http://localhost:11434"
api_key_env = "XAI_API_KEY"
model = ""
action_temperature = 0.2
max_retries = 2
timeout_ms = 12000
replay_file = ""
```

`sim-cli --llm mock|ollama|openai_compatible` overrides config. `cargo test` always injects `MockLlm`.

## Viewer / CLI

- Followed-agent HUD: needs, inventory, last primary action, last utterance, legal-action count.
- Optional Chebyshev vision overlay while following; speech from others can appear as a short log line on the HUD (not imgui).
- Species-colored vegetation; animal/fish markers; growing-crop tint.
- Markdown agent section: needs, skills, inventory, last sickness, last heard/said line, consumption totals.
- World summary: standing biomass / animal / fish counts, toxic events, utterance count.

## Tests (M3 acceptance bar)

| Test | Asserts |
|---|---|
| Same seed + mock LLM → same hash | two 50-tick runs |
| Continuation identity | save at T, load, +K matches continuous (`format_version = 2`) |
| Observation differs with perceptiveness | two agents, same cell, different vision contents |
| Drink on water lowers thirst | |
| Gather + Eat safe food lowers hunger | |
| Toxic eat penalises everyone; allergenic only tagged agents | |
| Hunt/Fish success is deterministic given skill, tool, and agent RNG | |
| Farm: plant → wait `grow_ticks` → harvestable | |
| Craft spear consumes wood+stone and appears in inventory; failure keeps ingredients | |
| Illegal selected action → `Wait` | |
| Simulated timeout / malformed path → `Wait` event | |
| v1 checkpoint load is refused | |
| Consumption counters increment | |
| Over-length speech is truncated to `max_message_length` | |
| Broadcast is heard next tick only inside listener hearing range | |
| Directed speech: target hears; non-target outside overhear margin does not | |
| Mock warns identified neighbours after a toxic eat (canned text, cooldown) | |
| Timeout/Wait produces no `Speak` event | |

Live LLM is not a CI requirement.

## PR Plan

### PR 1: Living world + agent body

- **Files:** `crates/sim-core/src/world.rs`, `agent.rs`, `config.rs`, `simulation.rs`, `checkpoint.rs`, tests
- **Dependencies:** none
- **Changes:** Vegetation species ids; animal/fish counts; needs, inventory, abilities, personality at spawn; Move/Wait/Rest only; `format_version = 2`; refuse v1.

### PR 2: Observation + legal actions

- **Files:** new observation/action modules, `config.rs` `[observation]`, tests
- **Dependencies:** PR 1
- **Changes:** Build `Observation`; Chebyshev ranges × perceptiveness; full-info flag; legal-action set; perceptiveness inequality test.

### PR 3: Survival execution

- **Files:** action execution, event kinds, thin memory, consumption metrics, tests
- **Dependencies:** PR 2
- **Changes:** Gather/Drink/Eat/Hunt/Fish/Farm/Craft; toxicity/allergies; illness timers; memory write + eviction; mock policy can already drive the loop (no speech yet).

### PR 4: Speech

- **Files:** communication config, `SimEventKind::Speak`, observation “heard last tick”, memory writes for speaker/hearers, mock canned warnings, tests
- **Dependencies:** PR 3
- **Changes:** Secondary `Speak`; truncate; directed vs broadcast vs shout; next-tick hearing; overhearing; energy cost for shout; warn-cooldown mock policy.

### PR 5: LLM trait + `sim-llm`

- **Files:** `crates/sim-core` LLM trait/mock/replay; new `crates/sim-llm`; workspace `Cargo.toml`
- **Dependencies:** PR 4
- **Changes:** `LlmClient::choose` returns primary + optional speak; mock + replay; OpenAI-compatible HTTP client (Ollama + xAI); timeout/malformed → Wait and no speech.

### PR 6: CLI / viewer / Markdown

- **Files:** `sim-cli`, `sim-bevy`, `viewer`, `configs/default.toml`
- **Dependencies:** PR 5
- **Changes:** `--llm` override; HUD + last line of speech + vision overlay + species markers; richer summaries; end-to-end mock continuation smoke.

## Files / reuse

**Reuse**

- `RngBank` / `derive_seed` / agent streams (`crates/sim-core/src/seeding.rs`)
- Turn-order shuffle and `state_hash` (`simulation.rs`)
- Checkpoint envelope (`checkpoint.rs`) — bump version, extend body
- M2 water/mineral layers and land-only movement
- Viewer follow-camera + HUD text (`crates/viewer/src/main.rs`)

**Do not touch in M3:** `shared::transport` TCP/WS, imgui, incentive types, proposal board (speech is informal only).

## Verification (when M3 is implemented)

- `cargo test -p sim-core` includes continuation identity under mock LLM and the survival/toxicity tests above.
- `cargo run -p sim-cli -- --ticks 50 --out-dir /tmp/m3 --checkpoint-every 25` then `--load` + 25 ticks matches a continuous 50-tick mock run.
- Viewer follow shows needs moving, a vision overlay, and the last heard/said line; toxic vs safe plants are distinguishable.
- Loading an M2 (`format_version = 1`) checkpoint fails loudly.

## Risks

- **LLM latency** will dominate tick time on live providers. Mock stays the default; `timeout_ms` must actually abort the HTTP call.
- **Prompt/hash stability.** `prompt_hash` in replay files must hash a canonical observation encoding, not debug `Display`.
- **Species config drift.** Changing the default species table changes `world_hash`; treat the table as part of the experiment record (already hashed via config).
- **Ability + tool success rates** need defaults that let a mock population survive a few hundred ticks on `configs/default.toml` without death (death is off, but 0-need penalties should not freeze everyone immediately).
- **Speech in the hash.** Utterances are in the event log and memories, so mock canned strings must be fixed literals. Do not put timestamps or non-canonical Unicode normalisation in spoken text.
- **One LLM call vs two.** Bundling speak into action selection makes live ticks cheaper; utterances will be dumber than a dedicated dialogue call. Revisit in M4 if talk quality is the experiment.
