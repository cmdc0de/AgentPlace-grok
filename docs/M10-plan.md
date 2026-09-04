# M10 — Live LLM reliability + Transfer/Store

**Status:** implemented (git tag `M10`).  
**Depends on:** M9 complete (`docs/M9-plan.md`, git tag `M9`, commit `d7da253`)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md` §1/§3 (action vocabulary, LLM contract), `needs-and-survival.md`, `deterministic-seeding-design.md` (replay)

## Context

M9 made the experimental brain real. The Spark `nemotron3:33b` 16×80 A/B (`docs/M9-test-plan.md` execution record) **changed the board** (coop adopted five `BanEatSpecies` vs zero on baseline), agents drank, nobody died. Two holes still block the next experiment:

1. **Reliability.** ~52% of live decisions were `LlmWait`. Compact probes return JSON in `content` plus a `reasoning` field; full-observation calls often fail to parse thinking wrappers. Replay stored `ChosenAction` serde (`{"primary":"Wait"}`), so replaying a timeout does **not** re-emit `LlmWait` and hashes diverge from the recording run.
2. **Economy.** Coop injects `keep the shared storage stocked`, but there is **no Store/Transfer**. Diet did not move (veg 5=5). The goal is text without a legal action.

M10 is **trustworthy live ticks + shared storage that costs energy and has a cap**. It is not a protocol rewrite.

M10 does **not** rewrite postcard to protobuf, add TLS/`wss`, timeline scrubbing, `visibility_modifier`, vote weighting, `/set`, or extra reflection LLM calls.

## Goal

A researcher can:

1. Run `--llm ollama` against Spark/`nemotron3:33b` and have **most** ticks parse to a legal primary (thinking stripped; `content` or `reasoning` JSON). HTTP failure → `Wait` + `LlmWait`, never mock.
2. Record **raw** model text (or a wait sentinel). Replay twice **and vs the recording run** → same `state_hash`, including `LlmWait`.
3. `--compare` includes Transfer / Store / Retrieve and stockpile totals.
4. Agents **Transfer** to an identified neighbor and **Store / Retrieve** in a **capacity-limited** land-cell container, paying **energy ∝ weight × qty**. Observation/prompt lists named contents + remaining slots/weight. 3D crate marker when non-empty (fog matches Observation).
5. In-process viewer `/give <id> <item> <qty>` (hash-sensitive; **not** on the postcard wire). CI stays `provider = mock`.

## In scope

### A. Thinking-model parse + honest replay

| Piece | Behavior |
|---|---|
| Extract JSON | Prefer `message.content`; if empty/non-JSON, try `message.reasoning`. Strip fences, `<think>…</think>`, prose before first `{`. |
| Ollama body | `"think": false` (ignored if unsupported). |
| Failure | Timeout / unreachable / still-malformed → `Wait` + `LlmWait`. **Never** mock mid-run. |
| Record | Raw model string on success. On wait: sentinel e.g. `{"__llm_wait__":"timeout"}` so replay re-emits `LlmWait`. Old M9 ChosenAction JSONL still loads as Wait without `LlmWait`. |

Keep `timeout_ms = 120000` and Spark URL. Empty `base_url` still mock.

### B. Containers, weight, energy

**World:** `stockpiles: BTreeMap<(x,y), Container>`, `#[serde(default)]`, `format_version = 2`. Land only. **One container per cell.** Omit empty keys.

**Limits** (defaults; overlay-overridable, **not** `ExperimentConfig` postcard fields):

| Cap | Default | Notes |
|---|---|---|
| `slot_cap` | 16 | Sum of item quantities |
| `weight_cap` | 80.0 display (8000 milli) | Sum of `qty × unit_weight` |

Store/Retrieve that would exceed slots or weight is **illegal**.

**Unit weight** (display; stored ×100 milli): food 0.5, fiber 0.4, wood 1.5, stone 3.0, basket/spear/fishing_rod 2.0.

**Energy** (hashed via `needs.energy`):

```
cost = qty * unit_weight_milli * haul_milli / 1000
```

`haul` default 0.4 display per weight-unit. Store, Retrieve, **and** Transfer all pay. If `energy < cost` → not legal (no partial spend; same as shout). Energy 0 does not kill.

**`Transfer { item, qty, to }`:** identified target, Chebyshev ≤ 1, sender has qty, receiver inventory room, energy paid by **sender**. Move only what fits the receiver cap; pay for **moved** qty.

**`Store` / `Retrieve`:** current land cell. Store creates the container if missing. Retrieve from empty → illegal.

**Observation / prompt:** named contents on visible tiles, remaining slots/weight, legal Transfer/Store/Retrieve with species names.

**Mock:** hungry + food in container → Retrieve; storage-goal + hunger ≥ 75% + food in inventory → Store if it fits and energy allows; extra food + adjacent hungry identified + agreeableness ≥ 40 → Transfer 1.

### C. Visual indicator (viewer)

- Legend kind: crate **cube**, brown `srgb(0.55, 0.38, 0.18)`, name `stockpile` (M5 marker pattern).
- Spawn only when container qty > 0; remove when emptied. Fog: tile in that agent’s Observation.
- Inspector: contents, `weight / weight_cap`, `slots / slot_cap`.
- Fixed mesh size (no scale-by-fill).

### D. `/give` + compare

- Viewer **in-process only:** `/give` mutates inventory, `SimEventKind::Give`. No energy cost (researcher cheat). Hash-sensitive.
- **Not** a new `ControlVerb` (would force `PROTOCOL_VERSION = 3`).
- `--compare` / `--report`: stockpile count, total weight, Transfer/Store/Retrieve in the event histogram.

## Out of scope (later)

| Later | What |
|---|---|
| **M11** | Done — [`M11-plan.md`](M11-plan.md) — mock fills crates; Basket is worn backpack; Move haul |
| **M12** | Done — [`M12-plan.md`](M12-plan.md) — public vs hidden incentives |
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
| **M14** | Done — [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done — [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done — [`M20-plan.md`](M20-plan.md) — revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done — [`M21-plan.md`](M21-plan.md) — pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| **M23** | [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done — [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| After M37 | Protobuf / JSON envelope; TLS/`wss` |
| Not M10 | Browser client; physical combat; CI Win/mac matrix; extra LLM reflection calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. **Parse/replay + limited weighted storage.** Live A/B can move board **and** stores.
2. **Keep postcard `PROTOCOL_VERSION = 2`.** No new control verbs.
3. **`format_version` stays 2.** Empty stockpiles default on old ckpts.
4. **One container per land cell;** slots **and** weight caps; empty keys omitted.
5. **Energy ∝ qty × unit weight** for Store, Retrieve, and Transfer; unaffordable ⇒ not legal.
6. **Weights/caps overlay or constants**, not new `ExperimentConfig` postcard fields.
7. **Fixed crate mesh** when non-empty; fog = Observation.
8. **Raw replay + wait sentinel.** M9 serde JSONL is Wait-only on replay.
9. **`/give` is in-process only.**
10. **Mock remains CI default.** Never mock-fallback on HTTP error.
11. **Portable Basket-as-backpack deferred.** Cell container is the shared store the coop goal names.

## Config / CLI

No new `ExperimentConfig` postcard fields. Optional overlay (like `[network]`) for slot/weight/haul defaults. `sim-llm` sends `think: false` for Ollama without a config field.

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
cargo test -p viewer
```

Live smoke (2 agents × 3 ticks) then optional 80-tick A/B as in [`M9-test-plan.md`](M9-test-plan.md). Look for Store/Transfer on coop `--compare`.

## Tests (M10 acceptance bar)

| Test | Asserts |
|---|---|
| Thinking-model fixture | `<think>` + `reasoning` JSON → parsed legal action |
| Wait-sentinel replay twice | same `state_hash` and same `LlmWait` count |
| Record + replay | recording hash **equals** replay hash (including waits) |
| Empty `base_url` + ollama | Mock |
| Transfer adjacent identified | items move; energy drops; `Transfer` event |
| Transfer far / unnamed / no energy | illegal → Wait |
| Store over slot or weight cap | illegal |
| Store then Retrieve | energy paid both ways; cap respected |
| M9 ckpt (no stockpiles) | loads |
| Marker helper | non-empty cell is a stockpile marker; empty is not |
| Mock + storage goal | some `Store` on a tiny map |
| Same seed twice | same hash |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Thinking-model parse + `think: false`

- **Files:** `crates/sim-llm/src/lib.rs`, unit tests
- **Changes:** extract JSON from content/reasoning; strip think tags; Ollama `think: false`.

### PR 2: Honest replay

- **Files:** `sim-core` `llm.rs` / `simulation.rs` record path
- **Changes:** raw response + wait sentinel; recording hash equals replay; old JSONL still loads.

### PR 3: Weight table + energy haul

- **Files:** new helper next to `species` / `execute`, unit tests
- **Changes:** millipoint weights; `cost = qty * weight * haul`.

### PR 4: Container Store/Retrieve + Transfer

- **Files:** `world.rs`, `action.rs`, `observation.rs`, `execute.rs`, `event_log.rs`, tests
- **Changes:** one container per land cell; legal + execute; M9 ckpts decode.

### PR 5: Observation/prompt + mock

- **Files:** `policy.rs`, `sim-llm` `build_prompt`
- **Changes:** storage-goal Store, hungry Retrieve, gift Transfer; prompt lists contents and remaining cap.

### PR 6: Viewer crate marker + inspector + `/give`

- **Files:** `markers.rs`, viewer render/legend/inspector, `commands.rs`
- **Changes:** non-empty crate mesh; fog = Observation; in-process `/give` only.

### PR 7: README + docs

- **Files:** README, this plan status when implemented, `needs-and-survival.md` (haul costs)

## Files / reuse

**Reuse:** `sim-llm` ureq client, `parse_choice_json` + fence strip, `ReplayTable`, M9 `--compare` histogram, inventory cap, `ItemId` names, identity range, `format_version = 2` serde default, M5 marker table.

**Do not touch in M10:** protobuf, TLS, `visibility_modifier`, vote weights, `ControlVerb` / `PROTOCOL_VERSION`, `ExperimentConfig` new postcard fields, imgui beyond `/give` + stockpile inspector.

## Verification (when M10 is implemented)

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
cargo test -p viewer
```

Step-by-step walkthrough (mock + optional live): [`M10-test-plan.md`](M10-test-plan.md).

CI must pass without Spark. Live Wait-rate should drop vs M9’s ~52% on a 2×3 smoke (document the number; do not gate CI on it).

## Risks

- **Larger than M7–M9.** Parse+replay is hash-sensitive for LLM runs; Transfer/Store is hash-sensitive for mock. Two same-seed tests.
- **Nemotron may ignore `think: false`.** Parse must still eat `reasoning` / `<think>`.
- **Old replay JSONL** of serde ChosenAction will not emit `LlmWait` — M9 format; new sentinel is M10.
- **Do not add `ControlVerb::Give`** or old viewers break.
- **Mock Store/Transfer changes default hashes** (same class as M9 drink cutoff).
- **Overlay caps vs hashed energy:** keep defaults stable so CI hashes are reproducible.
