# M4 — Public board, adopted rules, and food-economy reports

**Status:** implemented (`6b4cf93`).  
**Depends on:** M3 complete (`docs/M3-plan.md`, git tag `M3`)  
**Specs:** `memory-goals-incentives-spec.md` §2, `simulation-and-agents-spec.md` §5.3, `medium-priority-specs.md` §1–2, `decision-observation-llm-economy-metrics-spec.md` §5 and §7 item 5

## Context

M3 delivered the survival + decision loop: limited observation, needs, gather/hunt/fish/farm/craft, toxicity, free secondary speech, mock (and optional live) LLM action selection, v2 checkpoints. Agents can warn neighbours that mushrooms are toxic, but they cannot **formalise** that into a rule others vote on, and a researcher cannot pull a single report of “what was eaten, who is hungry, what food is left, what each agent *thinks* is left.”

M3-plan named M4 as the public proposal board. This document is that slice, plus a first-class **summary report** at world and agent grain (consumed, hunger, food available vs food known).

M4 does **not** add incentives, imgui, TCP/WebSocket, hybrid memory, or relationship graphs.

## Goal

A researcher can:

1. Watch agents **Propose / Support / Oppose** rules on a shared public board.
2. See a proposal **accepted, rejected, or expire**, and become an **adopted rule**.
3. See **structured** adopted rules actually block illegal eats/gathers (`RuleBlocked` → Wait).
4. Generate a **summary report** (world + per-agent) covering food consumed, hunger, standing stocks, and what agents know is available.
5. Checkpoint-restore with mock LLM; same seed ⇒ same `state_hash` (board + goals included).

## In scope

| Area | M4 meaning |
|---|---|
| **Personal goals** | Per-agent list, cap `max_personal_goals` (default 5). `{id, text, priority: u8, source}`. Seeded at spawn (stay fed, avoid known toxins). In observation + LLM prompt. Not a multi-step planner. |
| **Public board** | Fill the reserved checkpoint `public_board`. Holds open `Proposal`s and `AdoptedRule`s. Always in every observation (`public_board_always_visible = true`). |
| **Propose** | **Primary** action (costs the action slot). Cap `max_open_proposals_per_agent`. Text length-capped (`max_proposal_length`, default 200). Optional structured payload from the closed set below. |
| **Support / Oppose** | Primary actions on an open proposal id. Last stance wins (one vote per agent). Writes memories. |
| **Lifecycle** | `Open` → `Accepted` if supporters / population ≥ `default_acceptance_threshold` (0.5); `Rejected` if opposers reach the same; `Expired` after `proposal_lifetime_ticks`. Accepted copies onto `AdoptedRule`. |
| **Structured rules** | Closed set the core can enforce without NLP: `BanEatSpecies { tag }`, `BanGatherSpecies { tag }`, `MaxGatherPerTick { n }`. Free-text-only proposals are social facts only (no mechanical bite). |
| **Soft enforcement** | Adopted structured rule matching an attempted action → `Wait` + `RuleBlocked` event. No HP, exclusion, or stockpiles. |
| **Mock policy** | If memory has “X is toxic” and no open/adopted ban → `Propose` `BanEatSpecies` with canned text. If that proposal is open and the agent knows the toxin → `Support`. Else M3 survival policy. |
| **LLM JSON** | Same one call: `"action": "Propose"|"Support"|"Oppose"`, `proposal_id`, `text`, `rule`. Unknown rule kind → Wait. Board snapshot in the prompt. Speak remains optional secondary. |
| **Summary report** | Derived world + per-agent food-economy report (see below). `sim-cli --report`. Also folded into checkpoint Markdown. |
| **Observation** | `board: Vec<ProposalView>` always present (id, author if identified, text, status, support/oppose counts, structured kind). |
| **Checkpoint** | Keep **`format_version = 2`**. Fill `public_board`; goals use `#[serde(default)]` so M3 ckpts load (empty board, empty goals). |

## Summary report

Derived from live state or a loaded checkpoint. **Not** required for restore.

CLI:

```
sim-cli --report
sim-cli --load PATH --report
sim-cli --load PATH --ticks 0 --report --out-dir DIR
```

Writes `{experiment_id}_tick_{tick}_report.md` and optional `{experiment_id}_tick_{tick}_report.csv` when `[metrics] export_csv = true`.

### World section

| Field | Source |
|---|---|
| **Consumed** | Sum of per-agent `consumption` (vegetation, animal, fish, toxic_events) |
| **Hunger** | Mean / min / max hunger (also thirst, energy); count of agents with hunger < half max |
| **Available (objective)** | Standing edible vegetation **cells by species**, split safe / toxic / allergenic; animal totals; fish totals; unharvested crops |
| **Available (known)** | Union of resource-sighting memories across **all** agents (species + locations the population currently remembers). Can be a subset of objective stocks. |
| **Governance** | Open / accepted / rejected / expired proposal counts; adopted rules list |

### Per-agent section

| Field | Source |
|---|---|
| **Consumed** | That agent’s vegetation / animal / fish / toxic totals |
| **Hunger** | Current hunger, thirst, energy (millipoints and 0–100 display) |
| **Inventory** | Food items on hand (by species / item) |
| **Known available** | Food species/tiles in **that agent’s** memory (and, if reporting a live tick, current observation). This is what the agent believes is out there — it can lag or be wrong. |
| **Goals** | Active personal goals |
| **Board stance** | Proposals this agent authored / supports / opposes |

“Known available” is computed from existing memory/observation. Do not add an omniscient hidden field. Objective stocks stay integer cell/count sums; display may divide millipoints by 100.

## Out of scope (later)

| Later | What |
|---|---|
| **M5** | [`M5-plan.md`](M5-plan.md) — hybrid memory, relationship summaries, influence, decision JSONL, viewer legend + per-kind meshes |
| **M6** | imgui inspectors, agent-POV fog-of-war matching `Observation` |
| **M7** | TCP / WebSocket (`shared::transport`) |
| **M8** | Incentive schedules + load-checkpoint-and-inject |
| Later | Transfer/Store, shared stockpiles, meta-rules, Attack/Flee/death, dialects, lie detection, weighted-by-status voting |

Do not pull incentives into M4. The A/B loop needs a working board and a readable food report first.

## Key decisions

1. **Propose / Support / Oppose are primary actions.** Speak stays a free secondary action.
2. **Acceptance = simple majority of current population** (death still off ⇒ all agents). No status weighting.
3. **One stance per agent per proposal.** Switching Support→Oppose moves the vote.
4. **Structured enforcement = action illegal**, not damage. Unstructured adopted text is visible only.
5. **`allow_meta_rules = false`.** Cannot propose changes to `[proposals]` itself.
6. **No relationship graph in M4.** Support/Oppose write thin memories only.
7. **Integer hashed state.** Proposal ids `u64`; supporter/opposer sets `BTreeSet<AgentId>` (not f32 weights).
8. **Mock proposal strings are fixed literals** for hash stability.
9. **Report is derived**, like M2 Markdown. Restore never depends on `_report.md`.
10. **Keep `format_version = 2`.** Additive defaults; do not refuse M3 checkpoints.

## Config sketch

```toml
[agents.goals]
max_personal_goals = 5
max_public_goals = 3
can_adopt_public_goals = true

[proposals]
max_open_proposals_per_agent = 3
default_acceptance_threshold = 0.5
proposal_lifetime_ticks = 2000
max_proposal_length = 200
allow_meta_rules = false
public_board_always_visible = true

[metrics]
compute_every_n_ticks = 50
export_csv = true
track_consumption = true
track_proposal_stats = true
```

## Action vocabulary (additions)

Primary (in addition to M3):

- `Propose { text, rule: Option<StructuredRule> }`
- `Support { proposal_id }`
- `Oppose { proposal_id }`

`StructuredRule`:

- `BanEatSpecies { species }`
- `BanGatherSpecies { species }`
- `MaxGatherPerTick { n }`

LLM JSON (same object as M3, extra fields):

```json
{
  "action": "Propose",
  "text": "do not eat mushrooms",
  "rule": { "kind": "BanEatSpecies", "species": "mushroom" },
  "speak": null,
  "reasoning": "optional"
}
```

## Decision loop changes

After M3 decay/illness, **before** agent steps: apply proposal expiry; recompute acceptance/rejection; copy newly accepted rules onto the adopted list.

When executing Gather/Eat, consult adopted structured rules first; if blocked → `RuleBlocked` + Wait (ingredients/world unchanged).

Observation always includes the board, even under limited vision.

## Tests (M4 acceptance bar)

| Test | Asserts |
|---|---|
| Same seed + mock → same hash | includes board + goals |
| Continuation identity | M4 state; M3 v2 ckpt still loads (empty board) |
| Propose appears on board and in every observation | |
| Support/Oppose last-stance-wins | |
| Majority accept → AdoptedRule | |
| Expire after `proposal_lifetime_ticks` | |
| `BanEatSpecies` blocks Eat of that tag (`RuleBlocked`) | |
| Free-text adopted rule does **not** block actions | |
| Over-cap Propose → Wait | |
| Mock proposes a ban after toxic memory | |
| Report world consumed = sum of agent consumed | |
| Report objective veg counts match world cells | |
| Report known-available ⊆ objective (or equal) | |
| `--load` + `--report` regenerates the same numbers | |

Live LLM is not a CI requirement.

## PR Plan

### PR 1: Goals + empty board

- **Files:** `agent.rs`, `simulation.rs`, `observation.rs`, `checkpoint.rs`, config `[agents.goals]`
- **Dependencies:** none
- **Changes:** Personal goal vec; `PublicBoard` struct in the reserved slot; observation `board` (empty); M3 ckpts still load.

### PR 2: Propose / Support / Oppose + lifecycle

- **Files:** `action.rs`, execute, event kinds, `policy.rs` (stub), tests
- **Dependencies:** PR 1
- **Changes:** Primary actions; vote sets; accept/reject/expire; memories; mock still survival-only.

### PR 3: Structured rules + `RuleBlocked`

- **Files:** `execute.rs`, adopted-rule check, tests
- **Dependencies:** PR 2
- **Changes:** Three rule kinds; Eat/Gather gated; unstructured adopted text has no mechanical effect.

### PR 4: Mock governance + LLM JSON

- **Files:** `policy.rs`, `llm.rs` parse, sim-llm prompt
- **Dependencies:** PR 3
- **Changes:** Canned `BanEatSpecies` propose/support; parse Propose/Support/Oppose.

### PR 5: Summary report

- **Files:** new `crates/sim-core/src/report.rs`, `sim-cli --report`, checkpoint Markdown, optional CSV
- **Dependencies:** PR 2 (consumption already exists from M3)
- **Changes:** World + per-agent consumed / hunger / objective available / known available; governance counts; regenerable from checkpoint.

### PR 6: Viewer HUD

- **Files:** `viewer/src/main.rs`
- **Dependencies:** PR 4–5
- **Changes:** Followed agent: goals, hunger, consumption, known-food count; global open-proposal count. No imgui.

## Files / reuse

**Reuse**

- M3 `consumption` counters, millipoint needs, thin memory, `Observation`, checkpoint envelope (`public_board` already reserved)
- Mock policy and `parse_choice_json`
- `sim-cli --summarize` / `--load` patterns

**Do not touch in M4:** `shared::transport`, imgui, incentive types, relationship graphs, `sim-core` HTTP.

## Verification (when M4 is implemented)

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m4 --checkpoint-every 40 --report
cargo run -p sim-cli -- --load /tmp/m4/<id>_tick_40.ckpt --ticks 40 --llm mock
# final_hash matches a continuous 80-tick mock run
cargo run -p sim-cli -- --load /tmp/m4/<id>_tick_40.ckpt --ticks 0 --report
```

Viewer follow shows goals + a board line. Report world consumed equals the sum of agent consumed.

## Risks

- **Majority of population** will be slow to accept if most agents never learn the toxin. Mock policy must Support once they have the memory, or boards stall. Tests should force memory + votes.
- **M3 checkpoints** must keep loading: only additive `#[serde(default)]` on agent goals and a default-empty `PublicBoard`.
- **Known-available** depends on memory eviction. Reports can show “knew it, forgot it.” That is correct; do not special-case sightings as immortal unless we add a dedicated protected kind (optional: protect `ResourceSighting` the same way as `ToxinFact`).
- **LLM free-text Propose** without a structured rule will not enforce. Prompt should prefer structured `rule` when banning a species.
