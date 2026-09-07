# M8 — Incentive A/B inject and processing metrics

**Status:** implemented (git tag `M8`, commit `b0b9d14`).  
**Depends on:** M7 complete (`docs/M7-plan.md`, git tag `M7`, commit `789fc81`)  
**Specs:** `memory-goals-incentives-spec.md` §3 (schedule TOML + effect vocabulary), `deterministic-seeding-design.md` §4/§6 (load T, change only incentives, hashes diverge), `decision-observation-llm-economy-metrics-spec.md` §5 (metrics; M8 adds **wall-clock** timing, not the full social time-series dashboard)

## Context

The experimental loop is: (1) run a baseline and watch governance emerge; (2) load a checkpoint, inject a different incentive schedule, measure what changed. M1–M7 delivered (1) including a hash-neutral remote viewer. `IncentiveState` is still `{ entries: Vec<String> }`. Wire `InjectIncentive` always returns Error. Tick wall-clock is unmeasured.

M8 is the first slice that **intentionally changes `state_hash`**: the A/B intervention. Processing metrics ride along so a researcher can see whether the LLM (or an incentive) made ticks slower. Timing is **hash-neutral** if it never enters `state_hash`.

M8 does **not** rewrite the M7 codec to protobuf or a JSON envelope, add TLS, timeline scrubbing, or `visibility_modifier`.

## Serialization (what M7 shipped; what M8 keeps)

**M7 envelope is postcard, not JSON and not protobuf.**

| Layer | Format |
|---|---|
| TCP / WebSocket frame | `u32` LE length + **postcard** payload (`PROTOCOL_VERSION = 1` today) |
| Snapshot | Opaque **AGTN** checkpoint bytes (postcard body, `format_version = 2`) |
| `Tick.events` / `Tick.decisions` | **JSON `Vec<u8>` nested inside** the postcard Tick (`DecisionRecord.primary` is `serde_json::Value`; postcard cannot `deserialize_any`) |
| Files | JSONL + Markdown (unchanged) |

Protobuf would mean `.proto` + codegen, duplicating every message, while Snapshot stays AGTN postcard and decisions still need a self-describing value type. There is no second-language client yet. Switching in the same milestone as hash-sensitive incentives would throw away the reason `InjectIncentive` was reserved on the existing enum.

**Lock:** keep postcard. Implement `InjectIncentive`. Put timing on the wire as **JSON bytes** on `Tick.metrics` (same nested-JSON pattern as events). A protobuf or JSON-envelope rewrite waits for a non-Rust client.

Postcard cannot add struct fields compatibly. If `Tick` grows `metrics`, bump **`PROTOCOL_VERSION` to 2** and update this tree’s viewer + dummy in the same slice. Checkpoints stay `format_version = 2`.

## Goal

A researcher can:

1. Run a baseline (no schedule) → `final_hash` A.
2. Run the same seed with a TOML schedule from tick 0 → `final_hash` B ≠ A; the same schedule twice → both B.
3. `--load` tick T, `--inject schedule.toml`, continue → a clean counterfactual vs the original continuation.
4. `/inject path.toml` (or wire `InjectIncentive`) if the server was started with `--allow-control`.
5. Read **how long each tick took in real time**, and **how long each agent spent in perceive / retrieve / select / execute / remember**, without those numbers entering `state_hash`.

## In scope

### Incentives (hash-sensitive)

| Area | M8 meaning |
|---|---|
| **Schedule TOML** | Spec format (`[[incentives]]` + `[[incentives.effects]]`). Overlay file/CLI — **not** a new `ExperimentConfig` field (same reason `[network]` stayed out of core: postcard `config_hash` of old checkpoints). |
| **Apply at tick 0** | `sim-cli --incentives path.toml`. Optional overlay `[incentives] schedule = "path"` parsed like `[network]`. |
| **Inject on load** | `sim-cli --load ckpt --inject path.toml --ticks N`. |
| **Live inject** | `InjectIncentive { schedule_toml }` **works**. Requires `--allow-control`. Viewer `/inject [path]`. |
| **When** | Start of each tick: activate/deactivate by `start_tick` / `end_tick`. Mid-run inject takes effect next tick (immediately if `start_tick <= current`). |
| **Scope** | `all` \| `agent:N` \| `archetype:name`. Defer `supporters_of:proposal_N`. |
| **Closed effects** | Millipoints, not f32 in hashed state. Table below. **Not** `visibility_modifier`. |
| **Logging** | `SimEventKind::IncentiveApplied` / `IncentiveEnded`. Enters the event log → enters `state_hash`. |
| **Checkpoint** | `format_version` stays **2**. Pack schedule TOML into existing `IncentiveState.entries` (M4 board-blob pattern). |
| **RNG** | Existing `incentive` ChaCha20 stream if an effect needs a coin flip; most M8 effects are deterministic. |
| **sim-core** | Apply logic lives here. No sockets, no imgui. |

#### Closed effect set

| `type` | M8 meaning |
|---|---|
| `resource_multiplier` | Scale gather/eat yield for named resource (millipoint multiplier, e.g. 1400 = 1.4×). |
| `influence_factor_delta` | Add millipoints to `influence_factor` while active. |
| `memory_importance_boost` | Scale eviction-importance for a memory `kind`. |
| `goal_injection` | Add or raise a personal/public goal. |
| `proposal_threshold_modifier` | Millipoint delta on the majority threshold used in board lifecycle. |
| `relationship_delta` | Millipoint nudge to trust/affinity for scoped pairs. |

Unknown `type` → error at load; do not ignore.

### Processing metrics (hash-neutral)

Wall-clock via `std::time::Instant`. **Never hashed. Never required to restore.**

Instrument the **actual** `tick` / `step_agent` pipeline (not unimplemented Reflect/Plan):

| Clock | What |
|---|---|
| **Tick** | Total wall ns; `world_step`; board lifecycle; incentive apply; sum of agent steps. |
| **Per agent** | `perceive` (`observation::build` + heard memories), `retrieve`, `select` (mock/LLM/replay), `execute` (primary + speak), `remember` (memory/relationship writes). |
| **Export** | `{experiment_id}_timing.jsonl` — one line per tick (optional nested per-agent array). `--quiet` still prints `final_hash`; also `tick_ns_mean=` / `tick_ns_last=` (stderr or report footer). |
| **imgui** | World/status: last tick ms, mean/max over a ring (~64). Inspector: followed agent’s last-step breakdown. |
| **Wire** | `Tick.metrics` = JSON bytes of that tick’s timing (empty if the subscriber did not ask). |
| **Overlay** | `[metrics] timing = true`. Default on when `--out-dir` or in the viewer. Must not sit on `ExperimentConfig` if that would change `config_hash`. |

Gate: 40-tick mock with timing on vs off → **same `final_hash`**.

## Out of scope (later)

| Later | What |
|---|---|
| **M9** | Done — [`M9-plan.md`](M9-plan.md) — LLM prompts (needs + incentives), replay, `--compare`, mock drink-before-death |
| **M10** | Done — [`M10-plan.md`](M10-plan.md) — thinking-model parse + honest replay; Transfer/Store containers |
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
| **M22** | Done — [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| **M23** | Done — [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | Done — [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
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
| **M39** | [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| After M39 | Protobuf/TLS |
| Not M8 | Browser client; time-series charts; per-tick full checkpoints |

## Key decisions

1. **Incentives + timing, not a codec rewrite.** A/B is the goal; the stopwatch operates long runs.
2. **Keep postcard.** Nested JSON for events, decisions, and metrics. No protobuf in M8.
3. **`InjectIncentive` is real** and requires `--allow-control`.
4. **Timing never enters `state_hash` or checkpoint restore.**
5. **Incentive events do enter the hash** (that is the intervention).
6. **Do not add fields to `ExperimentConfig`.** Overlay CLI/TOML like `[network]`.
7. **`format_version` stays 2.** Pack schedule TOML into `IncentiveState.entries`.
8. **`PROTOCOL_VERSION = 2` only if Tick grows `metrics`.** Update viewer + dummy in this slice.
9. **Closed effect vocabulary.** Unknown types fail closed.
10. **Millipoints** for hashed effect magnitudes.

## Config / CLI

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet

cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml --ticks 80 --llm mock --quiet

cargo run -p sim-cli -- --load checkpoints/ID_tick_40.ckpt \
  --inject configs/incentives/coop.toml --ticks 40 --llm mock --quiet

cargo run -p sim-cli -- --listen tcp://127.0.0.1:9000 --allow-control --ticks 100000
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# /inject configs/incentives/coop.toml
```

**File format:** one TOML file is a *schedule* (array of `[[incentives]]`), not one file per incentive. Full fields: [`incentive-schedule-format.md`](incentive-schedule-format.md). Example: `configs/incentives/coop.toml`.

Example schedule (`configs/incentives/coop.toml`):

```toml
[[incentives]]
id = "early_cooperation_bonus"
description = "Reward supporters of public goals"
start_tick = 0
applies_to = "all"

[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
condition = "has_supported_public_goal"

[[incentives.effects]]
type = "proposal_threshold_modifier"
delta = -0.1
```

## Tests (M8 acceptance bar)

| Test | Asserts |
|---|---|
| Same seed, no schedule, twice | same `state_hash` |
| Same seed, same schedule, twice | same `state_hash`, **≠** baseline |
| `--load` T, no inject, continue | matches uninterrupted run |
| `--load` T, `--inject`, continue | **≠** uninterrupted run |
| Unknown effect type | load error |
| `InjectIncentive` without `--allow-control` | Error `ControlDisabled` |
| Timing on vs off, 40-tick mock | **same** `state_hash`; timing JSONL non-empty when on |
| `cargo test -p sim-core` | still green; no sockets in core |

## PR Plan

### PR 1: Timing clocks

- **Files:** `sim-core` tick / `step_agent`; JSONL writer; tests
- **Changes:** wall-clock breakdown; hash-neutral test.

### PR 2: Schedule parse + packing

- **Files:** new `sim-core` incentive module; `IncentiveState.entries`; `SimEventKind`
- **Changes:** TOML load; pack into existing checkpoint slot; apply/end events.

### PR 3: Closed effects

- **Files:** execute / board / memory / social
- **Changes:** millipoint implementations of the six types.

### PR 4: CLI A/B

- **Files:** `sim-cli`
- **Changes:** `--incentives`, `--inject`; overlay parse; baseline vs treated tests.

### PR 5: Wire inject + Tick.metrics

- **Files:** `shared` protocol, `sim-cli` server, dummy tests
- **Changes:** `InjectIncentive` succeeds when allowed; optional metrics bytes; `PROTOCOL_VERSION = 2` if Tick grows a field.

### PR 6: Viewer + docs

- **Files:** viewer `/inject`, imgui timing, README
- **Changes:** console inject; status last-tick ms; README A/B examples.

## Files / reuse

**Reuse:** `InjectIncentive` variant, `incentive` RNG stream, `IncentiveState.entries`, M7 `--allow-control`, event log / `state_hash`, overlay parse pattern from `[network]`.

**Do not touch in M8:** protobuf, TLS, `visibility_modifier`, vote weights, `ExperimentConfig` postcard layout, imgui internals beyond timing + `/inject`.

## Verification (when M8 is implemented)

```bash
cargo test -p sim-core
cargo test -p shared
cargo test -p sim-cli --test net

# hashes A vs B
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml --ticks 80 --llm mock --quiet
```

Two 40-tick mock runs with timing on vs off print the same `final_hash`.

## Risks

- **Postcard `ExperimentConfig`.** A new hashed config field breaks old checkpoint `config_hash`. Overlay only.
- **Postcard `IncentiveState`.** New struct fields break `format_version = 2` files. Pack TOML into `entries`.
- **Wall-clock in checkpoints.** Would make snapshots non-deterministic. Timing stays in JSONL / live Tick.metrics / imgui, not AGTN.
- **Effect scope creep.** Resist `visibility_modifier` and free-form payoff scripts.
- **PROTOCOL_VERSION.** If Tick.metrics is added, bump to 2 in the same PR as the viewer; do not leave a mixed tree.
