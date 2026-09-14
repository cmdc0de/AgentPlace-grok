# Config directory reference

Every file under `configs/`, what it is for, and every key the sim reads. Overlay tables may live **in** `default.toml` (or a copy) even when shipping leaves them off; they are **not** `ExperimentConfig` and do **not** change checkpoint `config_hash` unless noted.

CLI flags that turn the same features on: [`cli-reference.md`](cli-reference.md). Incentive effect vocabulary: [`incentive-schedule-format.md`](incentive-schedule-format.md). Needs millipoints: [`needs-and-survival.md`](needs-and-survival.md).

**Hashed vs overlay:** tables on `ExperimentConfig` (`[simulation]`, `[world]`, `[agents]`, `[checkpoint]`, `[observation]`, `[needs]`, `[communication]`, `[llm]` provider/url/model, `[proposals]`, `[metrics]`) are hashed. `[network]`, `[incentives]`, `[voting]`, `[storage]`, `[conflict]`, `[agents.sheet]`, `[population]`, `[inventions]`, `[pipeline]`, `[catalog]`, `[telemetry]`, `[time]`, and extra `[llm] barrier*` keys are overlay. `[time]` **is hashed when enabled** (the default). `[telemetry]` is never hashed.

---

## Directory map

| Path | Role |
|---|---|
| [`configs/default.toml`](../configs/default.toml) | Shipping experiment. Default `--config` for `sim-cli` and `viewer`. Idle mock 2-tick (time on) `7b8864e9…`; `--no-time` shipped-objects `35746f95…`. |
| [`configs/incentives/`](../configs/incentives/) | Overlay **schedules** (one file = many `[[incentives]]`). Not hashed. Pass `--incentives` / `--inject` / `/inject`. |
| [`configs/objects/`](../configs/objects/) | One TOML per world/item kind. `[visual]` is hash-neutral. `[sim]` is hashed when `--catalog` / `[catalog] enabled`. |

There is no other shipping experiment besides `default.toml`. Copy it to experiment; do not edit `coop.toml` unless that is the slice.

---

## `configs/default.toml` — ExperimentConfig (hashed)

Display floats (needs, influence, ranges) are stored as **millipoints** (×100) at load.

### Top-level

| Key | Shipping | Meaning |
|---|---|---|
| `master_seed` | `3735928559` | Root of the RNG tree. Same seed + same config ⇒ same hashes. Must fit signed 64-bit TOML integers. |

### `[simulation]`

| Key | Shipping | Meaning |
|---|---|---|
| `step_duration_secs` | `1.0` | Wall-clock hint for a tick; **not hashed** as timing. Sim still steps one discrete tick. |
| `max_ticks` | `100000` | Cap in the config object. **`sim-cli --ticks` is what this process runs** (default 100). |
| `pause_when_empty` | `false` | Reserved; attach pause is `--start-paused`. |
| `log_level` | `"info"` | Log verbosity for the process. |

### `[world]`

| Key | Shipping | Meaning |
|---|---|---|
| `seed` | `"auto"` | `"auto"` (from master), `"random"`, or an integer. World layers. |
| `width` / `height` | `64` / `64` | Map cells. Minimum 32. CLI `--width` / `--height` override for a new sim (32..=256). |
| `max_height` | `16` | Heightmap amplitude. Minimum 8. |

### `[world.terrain]`

| Key | Shipping | Meaning |
|---|---|---|
| `octaves` | `4` | Noise octaves for height. |
| `persistence` | `0.5` | Amplitude decay per octave. |
| `lacunarity` | `2.0` | Frequency multiplier per octave. |

### `[world.resources]`

Densities scale how much of the map is that layer; `min_*` are floors after generation.

| Key | Shipping | Meaning |
|---|---|---|
| `mineral_density` | `1.0` | Stone / mineral presence. |
| `vegetation_density` | `1.0` | Plant cells. |
| `water_coverage` | `0.25` | Water fraction. |
| `min_mineral_nodes` | `10` | Minimum mineral cells. |
| `min_fresh_water` | `5` | Minimum water cells. |
| `min_vegetation_patches` | `20` | Minimum veg cells. |
| `animal_density` | `1.0` | Land animals. |
| `fish_density` | `1.0` | Water fish. |
| `min_animals` / `min_fish` | `8` / `8` | Floors. |

Species **tables** (nutrition, toxicity, …) default in Rust and are overridden by `configs/objects/*.toml` `[sim]` when that directory is loaded (M41). `[world.species]` in experiment TOML is optional; shipping `default.toml` omits it.

### `[agents]`

| Key | Shipping / default | Meaning |
|---|---|---|
| `count` | `16` | Founders. Minimum 2. |
| `spawn_mode` | `"scattered"` | `scattered` \| `clustered`. `fixed_list` is rejected. |
| `spawn_seed` | `"auto"` | Same rules as world seed. |
| `default_memory_capacity` | `128` | Fallback; `[agents.memory] capacity` is what retrieval uses. |
| `start_with_basic_needs` | `true` | Spawn at need maxima. |
| `inventory_capacity` | omit → `16` | Pocket slot cap (STR overlay can raise weight cap; this is count). |

`[[agents.archetypes]]` (optional): `name`, `weight`, `personality`, `abilities`. Shipping file has none (uniform random). Used by incentive `applies_to = "archetype:name"`.

### `[agents.goals]`

| Key | Shipping | Meaning |
|---|---|---|
| `max_personal_goals` | `5` | Cap on personal goal list. |
| `max_public_goals` | `3` | Cap on public goals. |
| `can_adopt_public_goals` | `true` | Agents may copy public goals. |

### `[agents.memory]`

| Key | Shipping | Meaning |
|---|---|---|
| `capacity` | `128` | Entries kept (INT overlay can change this when sheet is on). |
| `eviction_policy` | `"importance_and_recency"` | Also `fifo`, `lowest_importance`. |
| `social_memory_bonus` | `1.5` | Multiplier for social memories. |
| `persistent_relationships` | `true` | Relationship rows survive eviction. |
| `enable_embeddings` | `false` | Local embeddings for retrieve; vectors **not** hashed. |
| `retrieval_k` | `8` | How many memories enter the prompt. |

### `[agents.social]`

| Key | Shipping | Meaning |
|---|---|---|
| `base_influence_factor` | `0.3` | Display; stored milli. CHA overlay can add vote weight. |
| `influence_decay` | `0.01` | Per-tick decay. |
| `track_relationships` | `true` | Trust/affinity/respect/fear maps. |
| `trust_support_threshold` | `20.0` | Below this, Support may be gated. |

### `[checkpoint]`

| Key | Shipping | Meaning |
|---|---|---|
| `auto_interval_ticks` | `500` | Used when `--out-dir` is set without `--checkpoint-every`. |
| `keep_last_n` | `20` | Prune older `.ckpt` in the out-dir. |
| `directory` | `"checkpoints"` | Default out-dir name if `--checkpoint-every` has no `-o`. |
| `write_markdown_summaries` | `true` | `{id}_tick_T_summary.md` + `_agents.md`. |
| `write_retries` | `3` | Atomic write retries. |

Magic `AGTN`. Write `format_version` **3**; read v2 and v3.

### `[observation]`

| Key | Shipping | Meaning |
|---|---|---|
| `base_vision_range` | `12.0` | Cells; × perceptiveness / WIS. |
| `base_hearing_range` | `18.0` | Hear / speech. |
| `base_agent_identity_range` | `8.0` | Name agents within this (WIS board range can exceed it). |
| `full_information` | `false` | If true, Observation covers the whole map (debug). |

### `[needs]`

0–100 display, millipoints internally. Decay runs each tick before agents act. Write-up: [`needs-and-survival.md`](needs-and-survival.md).

| Key | Shipping | Meaning |
|---|---|---|
| `hunger_max` | `100.0` | Cap. Mock “hungry” = hunger below 50. |
| `hunger_decay_per_tick` | `0.15` | Subtracted each tick. |
| `thirst_max` | `100.0` | Mock “thirsty” = thirst below 50. |
| `thirst_decay_per_tick` | `0.25` | |
| `energy_max` | `100.0` | Mock “tired” = energy below max/3. CON overlay can change the cap. |
| `energy_decay_per_tick` | `0.08` | |
| `energy_regen_while_resting` | `0.4` | Rest refill. |
| `death_enabled` | `true` | Remove agent at 0 thirst/hunger (needs death, not combat death). |

### `[communication]`

| Key | Shipping | Meaning |
|---|---|---|
| `speech_is_free_action` | `true` | Speak is secondary; does not consume the primary. |
| `base_speech_range` | `18.0` | CHA overlay can extend. |
| `shout_range_multiplier` | `1.8` | Shout vs talk. |
| `shout_energy_cost` | `5.0` | Energy paid to shout. |
| `max_message_length` | `200` | Truncate longer text. |
| `allow_overhearing` | `true` | Agents in range hear broadcasts. |
| `warn_cooldown_ticks` | `10` | Rate-limit toxin warns. |

### `[llm]` (hashed provider fields)

| Key | Shipping | Meaning |
|---|---|---|
| `provider` | `"mock"` | `mock` \| `wait` \| `ollama` \| `openai_compatible`. CI = mock. |
| `base_url` | Spark Ollama URL | Used when `--llm ollama`. **Empty URL ⇒ mock.** |
| `api_key_env` | `"XAI_API_KEY"` | Env var name for `openai_compatible`. |
| `model` | `"nemotron3:33b"` | Ignored while provider=mock. |
| `action_temperature` | `0.2` | Sampler temperature. |
| `max_retries` | `2` | HTTP retries (separate from overlay barrier). |
| `timeout_ms` | `120000` | Per call. Timeout ⇒ Wait that agent. |
| `replay_file` | `""` | If set, replay recorded JSON instead of calling. |

Barrier / reflect / plan keys on `[llm]` are **overlay** (next section).

### `[proposals]`

| Key | Shipping | Meaning |
|---|---|---|
| `max_open_proposals_per_agent` | `3` | Cap. Over-cap Propose → Wait. |
| `default_acceptance_threshold` | `0.5` | Majority fraction unless `[voting]` overlay. |
| `proposal_lifetime_ticks` | `2000` | Then expire. |
| `max_proposal_length` | `200` | Text cap. |
| `allow_meta_rules` | `false` | SetCouncil / SetCouncilTally Propose. |
| `public_board_always_visible` | `true` | If false, far open posts omit author (board fog). |

### `[metrics]`

| Key | Shipping | Meaning |
|---|---|---|
| `compute_every_n_ticks` | `50` | How often derived metrics refresh. |
| `export_csv` | `true` | CSV beside reports. |
| `track_consumption` | `true` | Food/animal/fish eaten. |
| `track_proposal_stats` | `true` | Board stats. |
| `track_relationship_graph` | `true` | Mean trust etc. |

`timing = true` in comments is overlay (timing JSONL when `--out-dir` is set). Wall-clock ns are **not** hashed.

---

## Overlay tables (same file or a copy of it)

Omit = today’s defaults. CLI flags **or** these keys.

### `[network]`

Not hashed. Prefer CLI `--listen`.

| Key | Default | Meaning |
|---|---|---|
| `tcp_listen` / `ws_listen` | `""` | Listen URLs if CLI omits `--listen`. |
| `token` | `""` | Hello secret. |
| `allow_control` | `false` | Accept Control. |
| `lockstep` | `false` | Wait for AckTick every tick. |
| `lockstep_timeout_ms` | `0` | `0` = wait forever. |

### `[incentives]`

| Key | Default | Meaning |
|---|---|---|
| `schedule` | `""` | Path to a schedule TOML applied at tick 0 if CLI omits `--incentives`. |

### `[voting]`

| Key | Default | Meaning |
|---|---|---|
| `weight` | `"equal"` | `equal` \| `influence` \| `respect`. |
| `accept` | `"majority"` | `majority` \| `unanimous` \| `council`. |
| `council` | `[]` | Agent ids; required when `accept = "council"`. |
| `council_tally` | unanimous-among-council | `unanimous` \| `majority`. Requires `accept = "council"`. |

### `[storage]`

Crate on a land cell (not pockets). Display floats → milli.

| Key | Default | Meaning |
|---|---|---|
| `slot_cap` | `16` | Item-count cap. |
| `weight_cap` | `80.0` | Weight cap. |
| `haul` | `0.4` | Move-cost factor for loose haul. |
| `max_worn_baskets` | `1` | How many Basket count as worn pack. `0` = cargo only. |
| `max_worn_backpacks` | `1` | Same for Backpack. |

### `[conflict]`

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | Attack / Flee legal. |
| `death_enabled` | `false` | Health 0 removes the agent. |

### `[agents.sheet]`

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | Roll founder six scores. `--sheet`. Score 0 ⇒ no modifier. |

### `[population]`

| Key | Default | Meaning |
|---|---|---|
| `reproduction` | `false` | PairBond / Reproduce. Implies sheet. |
| `aging` | `false` | Age ticks + childhood gates. |
| `childhood_ticks` | `80` | Cannot PairBond / Reproduce / Attack while younger. |
| `founder_age_ticks` | `200` | Stamped on founders when aging on. |
| `household_crates` | `false` | Home cell crate for the household. |
| `culture` | `false` | Founder culture ids. |
| `culture_count` | `4` | Ids `1..=n` (clamped 1–255). |

### `[inventions]` (M45)

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | Invent legal. `--inventions`. Mock does not pick Invent. |
| `share_delay_ticks` | `8` | Society share at invent + delay (+ patent). |
| `tree` | `false` | Next kind only if the previous in Gather → Move → Sense is **shared**. Inventor-only Gather does not unlock Move. |
| `patent_ticks` | `0` | Extra exclusive ticks after `share_delay`. |

### `[pipeline]` (M45)

| Key | Default | Meaning |
|---|---|---|
| `hash_events` | `false` | `--pipeline-events`. One `Pipeline { stages }` per living agent per tick. Bits: perceive=1, retrieve=2, select=4, execute=8, remember=16 (complete = 31). Do **not** hash `*_ns`. |

### `[catalog]`

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | `--catalog`. Hash `[sim]` from `--objects`. Empty catalog ≡ off. |

Object `[sim]` sleep (M53, hashed with catalog): `sleep_bonus` (omit 0) millipoints of energy max at dawn; `sleep_size` (omit 1, max 8) is an N×N footprint. `Place` consumes one held item. `Pickup` returns it. Combat: `attack_bonus` (omit 0); `attack_range` (omit 1, Chebyshev).

### `[time]` (M52)

Not in shipping `default.toml`. **Omit = enabled.** `--no-time` turns it off.

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `true` (omit) | Day/night clock. Hashed with `ticks_per_day` when on. |
| `ticks_per_day` | `240` | `day = tick / N`, `tod = tick % N`. Night is `tod >= N*3/4`. Dawn at `tick > 0 && tod == 0`. |

Dawn refill (millipoints): tiredness `max * (200 + remaining_milli * 4 / 10) / 1000` plus shelter `max * sleep_bonus / 1000` plus CON extra `CON_mod.max(0)*250`, then clamp. Rest `+energy_regen` still applies. Night: Hunt/Farm illegal. Viewer light is hash-neutral.

### `[telemetry]` (M46 / M52)

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `false` | `--telemetry`. In-process tick aggregates + RSS. **Not hashed.** |
| `otlp_endpoint` | empty | `--otlp-endpoint URL`. sim-cli POSTs OTLP/JSON to `{url}/v1/metrics`. Empty ⇒ no POST. |

### `[llm]` overlay extras (not on ExperimentConfig)

| Key | Default | Meaning |
|---|---|---|
| `barrier` | `false` | `--llm-barrier`. |
| `barrier_retries` | `3` | Extra attempts. |
| `reflect_on_evict` | `false` | |
| `reflect_every_n_ticks` | `0` | `0` = off. |
| `plan_every_n_ticks` | `0` | `0` = off. |
| `plan_length` | `4` | Steps stored. |
| `execute_plan` | `false` | |
| `reflect_importance` | `false` | |

---

## `configs/incentives/` — overlay schedules

One file, many `[[incentives]]`. Not ExperimentConfig. Full field list: [`incentive-schedule-format.md`](incentive-schedule-format.md).

`--incentives` applies at tick 0. `--inject` / `/inject` **replaces** the live schedule.

| File | What it is for |
|---|---|
| [`coop.toml`](../configs/incentives/coop.toml) | Shipping A/B: 1.4× food if you have supported a public goal, easier proposals, “keep storage stocked” goal. **Do not edit** unless that is the slice. |
| [`hidden-bonus.toml`](../configs/incentives/hidden-bonus.toml) | Same 1.4× food, `visibility = "hidden"` (banner omitted; effects still apply). |
| [`leadership.toml`](../configs/incentives/leadership.toml) | +70 influence on agent 0. Pair with `[voting] weight = "influence"`. |
| [`esteem.toml`](../configs/incentives/esteem.toml) | Everyone’s respect toward agent 0. Pair with `[voting] weight = "respect"`. |
| [`coalition.toml`](../configs/incentives/coalition.toml) | 1.4× food for current `supporters_of:proposal_0`. |
| [`kin-bonus.toml`](../configs/incentives/kin-bonus.toml) | 1.4× food for `kin_of:0`. Use with `--sheet --reproduction`. |
| [`force-reflect.toml`](../configs/incentives/force-reflect.toml) | `force_reflect` every tick for Custom/live LLM (mock skips). |

Per-incentive keys (every file): `id` (required), `description`, `start_tick`, `end_tick`, `applies_to`, `visibility`, `[[incentives.effects]]` with `type`. Scopes: `all`, `agent:N`, `archetype:name`, `supporters_of:proposal_N`, `kin_of:N`, `household:H`. Effect types: `resource_multiplier`, `influence_factor_delta`, `force_reflect`, `memory_importance_boost`, `goal_injection`, `proposal_threshold_modifier`, `relationship_delta`.

---

## `configs/objects/` — definition files

Loaded from `--objects DIR` (default this directory if it exists). **`[visual]` never hashed.** `[sim]` hashed when catalog is on.

Common header:

| Key | Meaning |
|---|---|
| `id` | Slug (`berry_bush`, `basket`, …). Catalog items sort by this string. |
| `kind` | `vegetation` \| `animal` \| `fish` \| `item` \| `crate` \| `crop` \| `agent`. |

### `[visual]` / `[visual.lod]`

| Key | Meaning |
|---|---|
| `glb` | Authored glTF/glb path (repo-relative). Missing configured file ⇒ sentinel; empty path ⇒ primitive. |
| `scale` | Optional uniform XYZ multiplier. Omit / `<= 0` / non-finite ⇒ omitted (agent auto-fits to capsule; others 1.0). Explicit `> 0` skips agent auto-fit. **Not hashed.** |
| `lod.near` / `mid` / `far` | Optional cheaper meshes by camera Chebyshev distance (near ≤ 8 cells, mid ≤ 24, else far). Missing step ⇒ next coarser, then `glb`. |

### `[sim]` by kind

**Vegetation** (`yield`, nutrition, toxicity, allergen, wood/fiber, grow):

| Key | Meaning |
|---|---|
| `yield` | `"food"` or `"wood"` (what Gather produces). |
| `nutrition` | Hunger refill when eaten (food). |
| `toxicity` | `safe` \| `toxic` \| `allergenic`. |
| `allergen_tag` | e.g. `solanaceae`; empty = none. |
| `wood_yield` / `fiber_yield` | Extra items from Gather. |
| `grow_ticks` | Farm / regrow. |

**Animal / fish:** `nutrition`, `toxicity`, `allergen_tag` (Hunt / Fish).

**Item:** `weight_milli`; optional `[sim.craft]`.

| Craft key | Meaning |
|---|---|
| `inputs` | `[["fiber", 2], …]` slug + qty. |
| `output_qty` | How many of this item Craft produces (default 1). |

**Crate / crop:** visuals only (sim caps come from `[storage]` / farm code).

Builtin item slugs stay Rust `ItemId` tags (Food/Wood/Fiber/Stone/Basket/Spear/FishingRod/Backpack). Other item files (`cord`) are `ItemId::Catalog`; v3 checkpoints store the **slug**.

### File list

| File | kind | Sim notes |
|---|---|---|
| `berry_bush.toml` | vegetation | Food 20, fiber 1, grow 40, safe. |
| `herb.toml` | vegetation | Food 15, fiber 1, grow 30, safe. |
| `mushroom.toml` | vegetation | Food 12, toxic, grow 20. |
| `nightshade.toml` | vegetation | Food 10, allergenic `solanaceae`, grow 35. |
| `tree.toml` | vegetation | Wood yield 3, nutrition 0, grow 80. |
| `hare.toml` | animal | Nutrition 30, safe. |
| `perch.toml` | fish | Nutrition 25, safe. |
| `wood.toml` | item | Weight 150. Gather from trees. |
| `fiber.toml` | item | Weight 40. |
| `stone.toml` | item | Weight 300. Minerals. |
| `basket.toml` | item | Craft 2 fiber → 1. Worn pack 8/25. |
| `spear.toml` | item | Craft wood+stone. Hunt tool. `[sim] attack_bonus = 500`, `attack_range = 2`. |
| `fishing_rod.toml` | item | Craft wood+fiber. Fish tool. |
| `backpack.toml` | item | Craft 4 fiber. Worn pack 12/40. |
| `cord.toml` | item | Catalog craft 2 fiber → 1. Extra file; catalog-on only. |
| `plank.toml` | item | Catalog craft 2 wood → 1. |
| `charcoal.toml` | item | Catalog craft 1 wood → 1. |
| `knife.toml` | item | Catalog craft stone+fiber → 1. `[sim] attack_bonus = 200`. |
| `net.toml` | item | Catalog craft 3 fiber → 1. |
| `hammer.toml` | item | Catalog craft 2 stone + 1 wood → 1. |
| `hoe.toml` | item | Catalog craft stone+wood → 1. |
| `waterskin.toml` | item | Catalog craft 2 fiber → 1. |
| `dried_fish.toml` | item | Catalog craft 1 food (`ItemId::Food(1)`) → 1. |
| `satchel.toml` | item | Catalog craft 2 fiber + 1 wood → 1. |
| `rucksack.toml` | item | Catalog craft 3 fiber + 1 wood → 1. |
| `cooked_veg.toml` | item | Catalog craft 1 food → 1. |
| `bowl.toml` | item | Catalog craft 1 stone → 1. |
| `club.toml` | item | Catalog craft 1 wood → 1. `[sim] attack_bonus = 300`. |
| `pike.toml` | item | Catalog craft 2 wood + 1 stone → 1. `[sim] attack_bonus = 800`. |
| `sling.toml` | item | Catalog craft 2 fiber + 1 stone → 1. `[sim] attack_bonus = 400`, `attack_range = 2`. |
| `bow.toml` | item | Catalog craft 2 wood + 1 fiber → 1. `[sim] attack_bonus = 600`, `attack_range = 3`. |
| `torch.toml` | item | Catalog craft 1 wood + 1 fiber → 1. |
| `axe.toml` | item | Catalog craft 2 wood + 1 stone → 1. |
| `jar.toml` | item | Catalog craft 2 stone → 1. |
| `bread.toml` | item | Catalog craft 2 food → 1. |
| `tent.toml` | item | Catalog craft 3 fiber + 2 wood → 1. `[sim] sleep_bonus = 100`, `sleep_size = 1` (1×1). Place on land. |
| `cabin.toml` | item | Catalog craft 4 wood + 2 stone → 1. `sleep_bonus = 200`, `sleep_size = 2` (2×2). |
| `house.toml` | item | Catalog craft 6 wood + 3 stone + 2 fiber → 1. `sleep_bonus = 300`, `sleep_size = 4` (4×4). |
| `millstone.toml` | item | Catalog craft 3 stone → 1. |
| `stew.toml` | item | Catalog craft 2 food → 1. |
| `ladder.toml` | item | Catalog craft 3 wood → 1. |
| `crate.toml` | crate | Land-cell stockpile mesh. |
| `crop.toml` | crop | Growing-plant mesh. |

LOD distances and optimized glb scale: [`art-scale.md`](art-scale.md).
