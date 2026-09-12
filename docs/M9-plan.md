# M9 — LLM-in-the-loop experiments and readable A/B

**Status:** implemented (git tag `M9`).  
**Depends on:** M8 complete (`docs/M8-plan.md`, git tag `M8`, commit `b0b9d14`)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md` §2–3 (observation contents, LLM contract), `needs-and-survival.md`, `incentive-schedule-format.md`, `deterministic-seeding-design.md` (seeded LLM calls + replay)

## Context

M1–M8 delivered the mechanical experiment: deterministic core, board, postcard attach, incentive inject, death, wall-clock timing. Live HTTP choosers already exist in `sim-llm` (Ollama / OpenAI-compatible / xAI). They are not the experimental brain yet.

The 200-tick mock coop A/B showed **hashes change** (injected goals + `IncentiveApplied`) but **diet and board do not** — mock never drank, ate, or voted. The live prompt **omits hunger, thirst, energy, inventory, and active incentives**, even though the observation spec says the agent should see needs and incentives they are allowed to know.

M9 is not “add HTTP.” M9 is **complete prompts, record/replay, `--compare`, and a mock that can drink before tick-400 thirst-death.**

M9 does **not** rewrite postcard to protobuf, add TLS, timeline scrubbing, `visibility_modifier`, Transfer/Store, or reflection-on-evict.

## Goal

A researcher can:

1. Run `--llm ollama` against **`http://spark-bcce.hlab:11434`** (default `base_url`) and get actions that can see needs and the current schedule.
2. Leave `provider = "mock"` (and empty `base_url` ⇒ mock) so CI never hits the network.
3. Record live replies to `replay_file` and replay them later for the same `state_hash`.
4. `sim-cli --compare dirA dirB` and read **what** changed (hash, deaths, board, consumption, goals, incentives), not only that hashes differ.
5. Run a default mock long enough that agents **Drink** (or else thirst-`Died` by tick 400) instead of ignoring water until they die.

## In scope

### LLM prompt (what the agent knows)

`build_prompt` / `Observation` includes:

| Field | Notes |
|---|---|
| Needs | Hunger, thirst, energy on the **0–100** scale + illness |
| Inventory | Named items |
| Toxins | Allergies + toxin facts from memory |
| Legal actions | **Species names**, not only `Debug` |
| Active incentives | id, description, window; those that `applies_to` this agent. Public; no `visibility_modifier` |
| Existing | Goals, board, heard speech, relationships |

Timeout / malformed → `Wait` + `LlmWait`. **Do not** switch to mock mid-run (would change hashes). Seed still passed on the wire to Ollama/xAI.

### URL / mock fallback

| Situation | Chooser |
|---|---|
| `provider` empty or `mock` | Mock |
| `ollama` / `openai_compatible` / `xai` and **`base_url` empty** | **Mock** (no invented host) |
| `--llm ollama` | Live, URL from config (default spark-bcce) |
| Live HTTP failure | `Wait`, not mock |

`configs/default.toml`:

```toml
[llm]
provider = "mock"                              # CI default
base_url = "http://spark-bcce.hlab:11434"      # used when --llm ollama
```

### Record / replay

If `replay_file` is a path to an existing JSONL, use it as the chooser. If the path is set but missing, the live client **appends** records (already sketched). Tests: fixture JSONL twice → same `state_hash`. No network in `cargo test -p sim-core`.

Optional env-gated smoke (`XAI_API_KEY` or reachable Ollama): 2 agents × 3 ticks; skip if unset.

### `sim-cli --compare` (derived, not hashed)

Diff two `--out-dir`s or two `.ckpt` files at comparable ticks:

- `state_hash` equal / not
- population, `Died` count, first death tick
- board open / accepted / rejected / adopted
- consumption veg/animal/fish/toxic
- hunger / thirst / energy means
- goal-text occupancy
- active incentive ids
- event-kind histogram delta

Markdown (+ optional CSV). Not checkpointed, not in `state_hash`.

### Mock: drink/eat before death

Hash-sensitive tweak so `death_enabled = true` default runs are not “zero Drink, mass thirst death at 400.” Same-seed test still matches. Not a substitute for LLM.

## Out of scope (later)

| Later | What |
|---|---|
| **M10** | Done — [`M10-plan.md`](M10-plan.md) — thinking-model parse + honest replay; Transfer/Store; `/give` |
| **M11** | Done — [`M11-plan.md`](M11-plan.md) — mock fills crates; Basket backpack |
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
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| After M46 | Protobuf / JSON envelope; TLS/`wss` |
| Not M9 | Browser client; physical combat; CI Win/mac matrix; extra LLM reflection calls |

## Key decisions

1. **LLM-quality loop + compare, not a new HTTP crate.**
2. **Empty `base_url` ⇒ mock.** Default live host is `http://spark-bcce.hlab:11434`. Mid-run HTTP failure is `Wait`.
3. **Replay JSONL is how frontier runs stay reproducible** (providers often ignore seed).
4. **Incentives in the prompt are public** this slice.
5. **Keep postcard `PROTOCOL_VERSION = 2`.** Prompts are not on the attach wire except as decision JSONL files.
6. **`format_version` stays 2.** Do not put prompt blobs in AGTN.
7. **Compare is derived.**
8. **Mock remaining CI default** (`provider = "mock"`).

## Config / CLI

```bash
cargo test -p sim-core

cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm ollama \
  --out-dir /tmp/base --quiet

cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm ollama \
  --incentives configs/incentives/coop.toml --out-dir /tmp/coop --quiet

cargo run -p sim-cli -- --compare /tmp/base /tmp/coop
```

## Tests (M9 acceptance bar)

| Test | Asserts |
|---|---|
| Replay fixture twice | same `state_hash` |
| Prompt builder unit | contains hunger/thirst and incentive id when a schedule is active |
| Empty `base_url` + provider ollama | Mock chooser (no network) |
| `--compare` identical dirs | hashes equal |
| `--compare` coop vs baseline (mock) | hashes differ; goal occupancy differs |
| Mock same seed twice (after drink tweak) | same hash |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Prompt + Observation fields

- **Files:** `sim-llm` `build_prompt`, `Observation` if needed, unit tests
- **Changes:** needs, inventory, toxins, named legal actions, active incentives.

### PR 2: Empty URL ⇒ mock; default spark-bcce URL

- **Files:** `sim-llm::chooser_from_config`, `configs/default.toml`
- **Changes:** no invented localhost; document `--llm ollama`.

### PR 3: Replay fixture test + write path

- **Files:** `sim-core` / `sim-cli` replay
- **Changes:** two-pass same hash; live append if file missing.

### PR 4: `--compare`

- **Files:** `sim-cli`, report helpers in `sim-core` if pure
- **Changes:** Markdown diff; mock A/B test.

### PR 5: Mock drink/eat before death

- **Files:** `policy.rs`, survival tests
- **Changes:** seek water/food earlier; same-seed hash test.

### PR 6: README + docs

- **Files:** README, this plan status when implemented
- **Changes:** spark-bcce URL, compare recipe, replay.

## Files / reuse

**Reuse:** `sim-llm` ureq client, `replay_file`, `Chooser::Custom`, `parse_choice_json`, M8 inject + reports, `needs-and-survival.md` display scale.

**Do not touch in M9:** protobuf, TLS, `visibility_modifier`, vote weights, `ExperimentConfig` postcard layout except existing `[llm]` strings, imgui beyond optional death count.

## Verification (when M9 is implemented)

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
cargo run -p sim-cli -- --compare /tmp/base /tmp/coop
```

Step-by-step researcher walkthrough (mock + live Nemotron on Spark): [`M9-test-plan.md`](M9-test-plan.md).

Live Ollama only if spark-bcce is reachable; CI must pass without it.

## Risks

- **Prompt changes live hashes** of LLM runs (expected). Mock hashes change only if PR 5 ships.
- **Spark host down** must not fail `cargo test`. Empty URL / mock provider / skip smoke.
- **Observation bloat** in the prompt: cap relationship/board lines; do not dump full memory.
- **Do not mock-fallback on HTTP error** or A/B runs become mixed-chooser garbage.
