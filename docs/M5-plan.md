# M5 — Hybrid memory, relationships, influence, decision logs, and viewer legend

**Status:** implemented (`7de52be`).  
**Depends on:** M4 complete (`docs/M4-plan.md`, commit `6b4cf93`)  
**Specs:** `memory-goals-incentives-spec.md` §1, `simulation-and-agents-spec.md` §5.2–5.4 and §6, `medium-priority-specs.md` §1, `decision-observation-llm-economy-metrics-spec.md` §6–7 items 4 and 6

## Context

M4 delivered a public board: Propose / Support / Oppose, structured adopted rules with `RuleBlocked`, and a food-economy report. Memory is still a **flat capped list**. Speech and votes write thin entries. They do **not** move trust, affinity, respect, or fear. Every identified warning is believed the same way; willingness to Support a ban is “do I have a ToxinFact?”, not “do I trust the author?”

The event log records **what happened** (Gather, Speak, Propose). It does not record **why** the agent chose that primary. The Bevy viewer draws almost every resource as a cuboid — berry_bush and herb are the same green cube — with no legend.

M5 is the social-memory slice **plus** researcher-facing logs and readable 3D cues.

M5 does **not** add incentives, imgui, TCP/WebSocket, vote weighting, LLM-generated reflections, or a live embedding model.

## Goal

A researcher can:

1. Inspect a **hybrid memory store** per agent: capacity-capped episodic slots plus **never-evicted relationship summaries**.
2. See **trust / affinity / respect / fear** move when agents speak (identified) and when they Support / Oppose.
3. See **influence** change mock (and LLM-prompted) willingness to Support a trusted author’s proposal — **without** changing “1 agent = 1 vote”.
4. Read a **decision JSONL** (every agent, every tick) next to the existing event JSONL.
5. Look at the 3D view and tell **berry from mushroom from hare from perch** (unique mesh + colour) with an on-screen **legend**.
6. Checkpoint-restore with mock LLM; same seed ⇒ same `state_hash` (relationships + memory ids included). Decision files are derived and not required for restore.

## In scope

| Area | M5 meaning |
|---|---|
| **Hybrid `MemoryStore`** | Replace the bare `Vec<MemoryEntry>` on `Agent` with `{ slots, relationship_summaries, next_memory_id }`. Capacity still `default_memory_capacity` / `[agents.memory] capacity`. |
| **Richer entries** | Add `id`, `participants: Vec<AgentId>`, `valence` (millipoints, −10000…10000). New kinds: `Interaction`, `Norm`. Keep Observation / Sickness / ToxinFact / Utterance / Action / Proposal. |
| **Relationship summaries** | Per known other agent: `trust, affinity, respect, fear` as **millipoints** (−10000…10000), `interaction_count`, `last_interaction_tick`, `notable` (up to 3 memory ids). **Never fully evicted** while `persistent_relationships = true`. |
| **Integer social state** | No `f32` in hashed state. Config floats (`0.3`, `1.5`) convert with the existing millipoint helper at load. Display divides by 100 (−100.00…100.00). |
| **Influence factor** | Per-agent `influence_factor` millipoints, seeded from `base_influence_factor` (default 0.3 → 3000 milli). **Does not weight votes.** It scales how much a trusted author’s open proposal nudges mock Support, and is included in the LLM prompt. |
| **Relationship decay** | Each tick, each dimension moves toward 0 by `f64_to_milli(influence_decay)` (default `0.01` → 1 milli/tick). Neutral 0 is the rest state. |
| **Interaction pipeline** | Identified Speak, Support, and Oppose write `Interaction` memories on **both** sides (when the partner is known) and apply canned millipoint deltas (below). Unidentified / overheard speech writes `Utterance` only — **no** relationship row. The public board identifies proposal authors even out of vision. |
| **Toxin-from-speech** | **Unchanged from M3/M4:** hearing “X is toxic” still writes `ToxinFact`. Trust does not gate belief of warnings (otherwise M4 mock boards stall). Trust gates **social-proof Support**. |
| **Mock policy** | Keep M4: own ToxinFact → Propose / Support ban. **New:** if an open `BanEatSpecies` is authored by someone with `trust >= trust_support_threshold` (default 2000 milli) and `influence_factor > 0`, Support even without a ToxinFact. Else M4 survival policy. |
| **Eviction** | Default `importance_and_recency` with `social_memory_bonus` (default 1.5 → score × 150/100 when `participants` non-empty). Always protect Sickness, ToxinFact, relationship summaries, and the single highest-importance memory per known partner. Also `fifo`, `lowest_importance`. **No** LLM “summarise then drop”. |
| **Optional embeddings** | `[agents.memory] enable_embeddings = false`. When false, no vector field; retrieval is importance × recency. When true, still **not** in `state_hash` and **not** a network embedding call. M5 CI never requires embeddings. |
| **Prompt retrieval** | LLM prompt includes: top `retrieval_k` (default 8) episodic slots by eviction score, plus relationship summaries for **identified** agents in this observation. JSON actions unchanged from M4. |
| **Observation** | Add `relationships: Vec<RelationView>` for identified nearby agents (id + four millipoint dimensions). Unidentified silhouettes stay anonymous (no row). |
| **Event log** | Keep logging **every executed** `SimEvent` to `{experiment_id}_events.jsonl`. Do not drop kinds. If relationship updates emit events, they are extra lines, not replacements. |
| **Decision log** | One JSONL line **per agent per tick** (`{experiment_id}_decisions.jsonl`) recording the *choice* (see below). Derived; **not** in `state_hash`. |
| **Report / Markdown** | Per-agent: relationship counts, mean trust of known others, top-3 by \|trust\|. World: mean trust, count of directed pairs. Optional CSV when `track_relationship_graph = true`. |
| **Viewer HUD** | Followed agent: `rel:N mean_trust:X`, last `policy_branch`, inventory as species names (`berry_bush×2`, not `Food(1)`). |
| **Viewer legend + shapes** | Distinct **mesh + colour** per kind (table below) and an on-screen legend (`L` toggles). No imgui. |
| **Checkpoint** | Keep **`format_version = 2`**. Pack `MemoryStore` and relationship maps into the reserved `public_board` blob next to goals (M4 pattern). M4 ckpts load → empty relationships, influence from config default. |

## Canned relationship deltas (hash-stable literals)

All identified; clamp −10000…10000 after each update. No RNG in the table.

| Event | Actor → partner | Partner → actor |
|---|---|---|
| Speak (identified, not shout) | affinity +50 | affinity +50 |
| Shout | affinity +20, fear +30 | fear +30 |
| Support their open proposal | trust +200, respect +100 | affinity +50 |
| Oppose their open proposal | trust −150, affinity −50 | affinity −80 |
| Own ToxinFact confirms their warning text | trust +300 | — |

Personality may **gate** (e.g. skip trust-Support if `agreeableness < 20`) but must not inject RNG into the deltas.

## Decision log

Events are mechanical **outcomes**. Decisions are the **choice** that preceded them. A Support decision can still become a Wait event if the proposal closed; log both.

`--out-dir` writes `{experiment_id}_decisions.jsonl` (one line per agent per tick). `--load PATH --ticks 0` does not rewrite history; a live `--report` may quote counts from the file if present.

```json
{
  "tick": 12,
  "agent": 3,
  "chooser": "mock",
  "policy_branch": "trust_support",
  "call_seed": 0,
  "prompt_hash": "",
  "retrieved_memory_ids": [4, 9],
  "relation_ids": [0, 1],
  "legal": ["Wait", "Drink", "Support"],
  "primary": {"action": "Support", "proposal_id": 0},
  "speak": null,
  "reasoning": null
}
```

| Field | Mock | LLM / replay |
|---|---|---|
| `chooser` | `"mock"` | `"llm"` / `"replay"` / `"wait"` |
| `policy_branch` | closed set below | `"llm"`, `"llm_wait"`, or `"replay"` |
| `prompt_hash` | hash of observation used | same |
| `reasoning` | null | optional string from the model JSON |
| `retrieved_memory_ids` | ids actually fed to the chooser | same |

**Closed `policy_branch` set (mock):** `toxin_propose`, `toxin_support`, `trust_support`, `drink`, `eat`, `gather`, `hunt`, `fish`, `farm`, `craft`, `rest`, `move`, `wait`.

Timeout / Wait chooser → `policy_branch: "llm_wait"` (or `"wait"`), no speak. Restore never depends on this file.

## Viewer legend + visual cues

**Today:** every vegetation cell is the same cuboid; berry_bush and herb share green; mushroom / nightshade / tree differ only by colour; hare / perch / mineral are slightly different cuboids; there is **no legend**.

M5 locks **shape and colour** so a screenshot is readable:

| Kind | Mesh | Colour |
|---|---|---|
| Land | heightmap | green ramp (existing) |
| Water | heightmap | blue (existing) |
| berry_bush | sphere | green `srgb(0.18, 0.62, 0.22)` |
| herb | thin capsule / cylinder | yellow-green `srgb(0.45, 0.72, 0.20)` |
| mushroom | sphere on short cylinder | red `srgb(0.75, 0.22, 0.18)` |
| nightshade | sphere | purple `srgb(0.45, 0.15, 0.55)` |
| tree | tall cylinder | brown `srgb(0.32, 0.22, 0.12)` |
| crop | small cube | lime `srgb(0.55, 0.85, 0.25)` |
| hare | elongated cuboid | tan `srgb(0.72, 0.55, 0.32)` |
| perch | flat cuboid | blue `srgb(0.25, 0.45, 0.75)` |
| mineral | cube | grey `srgb(0.55, 0.52, 0.48)` |
| agent | capsule | per-id HSL (existing) |
| vision overlay | yellow ring (existing) | |

HUD **legend** (Bevy `Text`, not imgui): a left-side column, **one kind per line** (`sphere  berry_bush`, `capsule  herb`, …). Always on; `L` toggles. Status HUD sits to the right of it. Followed-agent inventory uses the same names.

## Out of scope (later)

| Later | What |
|---|---|
| **M6** | Done — [`M6-plan.md`](M6-plan.md) (imgui research UI, agent-POV fog-of-war, slash-command console (`/report`)) |
| **M7** | Done — [`M7-plan.md`](M7-plan.md) (TCP / WebSocket, hash-neutral attach) |
| **M8** | Done — [`M8-plan.md`](M8-plan.md) (incentive A/B + timing + death) |
| **M9** | Done — [`M9-plan.md`](M9-plan.md) — LLM prompts, replay, `--compare` |
| **M10** | Done — [`M10-plan.md`](M10-plan.md) — LLM parse/replay; Transfer/Store containers |
| **M11** | Done — [`M11-plan.md`](M11-plan.md) — mock fills crates; Basket backpack |
| **M12** | Done — [`M12-plan.md`](M12-plan.md) — public vs hidden incentives |
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
| **M14** | Done — [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| Later | Live embedding models, LLM reflection-on-evict, dialects, lie detection |

Do **not** pull incentives into M5. Relationships have to exist before an incentive can target them.

## Key decisions

1. **Millipoints, not f32, in hashed social state.** Spec floats are config-only.
2. **Votes stay unweighted.** M4 “1 agent = 1 vote” stands. Influence changes *whether* you Support, not the weight of the vote.
3. **Relationship summaries never evict** when `persistent_relationships = true`. Episodic slots still cap.
4. **Identified interactions only** update the graph. Overhearing is informational, not social credit.
5. **Toxin warnings stay ungated** so M4 mock governance still converges. New behaviour is trust-gated Support of a proposal you do not personally know is needed.
6. **`enable_embeddings = false` by default.** Vectors are not part of `state_hash`. No HTTP embed API in `sim-core`.
7. **No second LLM call** for reflections or dialogue. One action+speak call, now with retrieved memories + relation rows in the prompt.
8. **Keep `format_version = 2`.** Additive blob fields; M4 checkpoints load with empty graphs.
9. **Canned integer deltas** for hash stability (same spirit as M4 canned proposal strings).
10. **Decay toward 0**, not toward `base_influence_factor`. The base factor is a trait; relationships are the decaying state.
11. **Decision log is derived.** Every agent-tick is logged; it is not hashed. Events stay hashed (mechanical truth).
12. **Legend and unique meshes are M5 viewer work**, not imgui. Colour+shape pairs are literals.

## Config sketch

```toml
[agents.memory]
capacity = 128
eviction_policy = "importance_and_recency"  # or "fifo", "lowest_importance"
social_memory_bonus = 1.5
persistent_relationships = true
enable_embeddings = false
retrieval_k = 8

[agents.social]
base_influence_factor = 0.3
influence_decay = 0.01
track_relationships = true
trust_support_threshold = 20.0   # display units; 2000 milli
# relationship_dimensions are fixed in M5: trust, affinity, respect, fear

[metrics]
track_relationship_graph = true
```

Reuse `[agents] default_memory_capacity` as the default for `[agents.memory] capacity` when the nested table is omitted.

`track_relationships = false` → no map, no deltas, M4-identical social behaviour (still write episodic memories and decision lines).

## Decision loop changes

After M4 proposal lifecycle, **before** agent steps: decay relationship millipoints toward 0.

When executing identified Speak / Support / Oppose: write `Interaction` on both sides, apply canned deltas, bump `interaction_count`.

Observation: attach relation views for identified agents.

Mock `choose_governance`: after the M4 ToxinFact path, try trust-gated Support. Record `policy_branch`.

After the chooser returns (and after legality clamp): append one decision-log line, then `execute_primary` (events).

LLM prompt: retrieved slots + relation rows. JSON vocabulary unchanged from M4 (no new primary actions).

## Tests (M5 acceptance bar)

| Test | Asserts |
|---|---|
| Same seed + mock → same hash | includes relationships + memory ids |
| Continuation identity | M5 state; M4 v2 ckpt still loads (empty graph) |
| Identified speak updates both maps | affinity +50 each way |
| Unidentified / overheard speak does not create a relationship row | |
| Support of author’s proposal raises supporter→author trust | |
| Oppose lowers affinity | |
| Relationship summaries survive an eviction storm | fill slots past capacity; map still present |
| `fifo` vs `importance_and_recency` drop different episodic entries | |
| ToxinFact still written from heard warning (ungated) | M4 compat |
| Trust-gated Support: no ToxinFact, trust ≥ threshold, open ban from that author → Support | |
| `track_relationships = false` → hash matches a run that never applies deltas | |
| Report world mean-trust is the mean of per-agent means | |
| `--load` + `--report` regenerates the same relationship numbers | |
| Every agent-tick writes exactly one decision JSONL line | |
| Forced ToxinFact tick logs `policy_branch: toxin_propose` (or `toxin_support` if a ban is already open) | |
| Event JSONL still contains the matching `Propose` / `Support` / `Wait` outcome | |
| Viewer mesh table: berry_bush ≠ herb (different `Mesh` primitive, not only colour) | unit or render helper test |

Live LLM and embeddings are not CI requirements. Full GPU screenshot tests are not required; mesh/colour pairing can be a pure function `marker_for(species) -> (Shape, Color)`.

## PR Plan

### PR 1: Config + millipoint relationship types

- **Files:** `config.rs`, new `crates/sim-core/src/social.rs` (or extend `memory.rs`), `configs/default.toml`
- **Dependencies:** none
- **Changes:** `[agents.memory]`, `[agents.social]`; `RelationshipSummary`; millipoint helpers; defaults.

### PR 2: Hybrid store + eviction policies

- **Files:** `memory.rs`, `agent.rs`, `checkpoint.rs` blob
- **Dependencies:** PR 1
- **Changes:** `MemoryStore`; ids; protect per-partner notable memories; fifo / lowest_importance / importance_and_recency; M4 ckpts load.

### PR 3: Interaction pipeline

- **Files:** `execute.rs`, speak path in `simulation.rs`, vote path
- **Dependencies:** PR 2
- **Changes:** Both-sides `Interaction` memories; canned deltas; decay each tick; skip when unidentified or `track_relationships = false`.

### PR 4: Influence + mock / LLM prompt

- **Files:** `policy.rs`, `observation.rs`, `sim-llm` prompt
- **Dependencies:** PR 3
- **Changes:** Trust-gated Support; observation relation views; retrieve top-k slots into the prompt. No new JSON actions.

### PR 5: Decision JSONL

- **Files:** new `crates/sim-core/src/decision_log.rs`, `simulation.rs` step, `sim-cli`
- **Dependencies:** PR 4
- **Changes:** One record per agent-tick; mock `policy_branch`; LLM `prompt_hash` + reasoning; `{id}_decisions.jsonl` beside events.

### PR 6: Report + viewer HUD / legend / meshes

- **Files:** `report.rs`, checkpoint Markdown, `viewer/src/main.rs`, `viewer/src/render.rs`
- **Dependencies:** PR 3–5
- **Changes:** Relationship snapshot in `--report`; HUD `rel` / `mean_trust` / last branch; unique mesh per kind; legend (`L`); inventory species names.

### PR 7: Tests

- **Files:** `crates/sim-core/tests/social.rs` (new), decision-log tests, existing determinism/checkpoint
- **Dependencies:** PR 5–6
- **Changes:** Acceptance bar above.

## Files / reuse

**Reuse**

- M4 board, votes, `RuleBlocked`, canned BanEat mock, millipoint needs helper, checkpoint blob pattern, `--report`, `{id}_events.jsonl`
- M3 `apply_heard_memories` toxin parse (keep ungated)
- Protected Sickness / ToxinFact eviction
- Viewer heightmap water/land colours, agent capsules, vision overlay

**Do not touch in M5:** `shared::transport`, imgui, incentive types, vote weights, `sim-core` HTTP, embedding HTTP clients.

## Verification (when M5 is implemented)

```bash
# Tests (mock LLM; no network)
cargo test -p sim-core
cargo test -p sim-core --test social
cargo test -p sim-core --test governance

# 80-tick mock run: checkpoints, report, events, decisions
rm -rf /tmp/m5
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m5 --checkpoint-every 40 --report --llm mock --quiet
# note final_hash from this run

ID=$(basename /tmp/m5/*_tick_80.ckpt _tick_80.ckpt)
# 16 agents × 80 ticks = 1280 decision lines with default.toml
test "$(wc -l < /tmp/m5/${ID}_decisions.jsonl)" -eq 1280
test -f /tmp/m5/${ID}_events.jsonl
test -f /tmp/m5/${ID}_tick_80_report.md
grep -q "relationships:" /tmp/m5/${ID}_tick_80_report.md

# Continuation: load tick 40, run 40 more — same final_hash as a continuous 80-tick run
cargo run -p sim-cli -- --load /tmp/m5/${ID}_tick_40.ckpt --ticks 40 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet

# Report from checkpoint with no extra ticks
cargo run -p sim-cli -- --load /tmp/m5/${ID}_tick_40.ckpt --ticks 0 \
  --report --out-dir /tmp/m5-report

# Viewer: L toggles legend, F / 0-9 follow an agent
cargo run -p viewer -- --config configs/default.toml
cargo run -p viewer -- --load /tmp/m5/${ID}_tick_80.ckpt
```

Report per-agent section lists relationship counts. Viewer follow shows `rel` + `mean_trust` + legend. Same seed ⇒ same hash with relationships on. Live LLM (`--llm ollama` / `--llm openai_compatible`) is optional and not a CI requirement.

## Risks

- **Toxin-from-speech vs trust.** Gating belief on trust would freeze M4’s mock majority. Keep warnings ungated; only extra Supports are trust-gated.
- **f32 temptation.** Spec examples use `f32` trust. Putting those in the hash breaks continuation across platforms. Millipoints are mandatory.
- **Decay rate.** `influence_decay = 0.01` → 1 milli/tick is slow enough that a 2000-trust Support bump lasts ~2000 ticks (one proposal lifetime). If graphs look frozen, lower the canned deltas, do not switch to f32.
- **Both-sides write on Support.** Author may not be in identity range. Still update both maps: the public board identifies the author. This is intentional (you know who you voted for).
- **M4 checkpoints.** Empty graph + default influence from parsed config. Hash of a restored M4 ckpt under an M5 binary will **not** match the M4-era hash (new fields in `state_hash`); loading must succeed. Continuation identity is M5-on-M5.
- **Embeddings.** Leaving a disabled flag is enough. Wiring ureq to an embed API would pull HTTP into the wrong crate and non-determinism into CI.
- **Decision log volume.** 16 agents × 100k ticks is large. Same as events: only write when `--out-dir` is set; do not keep the full log in RAM beyond the current tick.
- **Legend vs imgui.** Keep the legend as Bevy `Text`. Do not start M6 imgui to “do the legend properly.”
