# M26 — Reflect/Plan every-N-ticks, record/replay of reflection text

**Status:** implemented  
**Depends on:** M25 complete (`docs/M25-plan.md`, commit `35f1560`)  
**Walkthrough:** [`M26-test-plan.md`](M26-test-plan.md)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md` §1 (pipeline Reflect / Plan), `M25-plan.md` (evict-reflect skipped on replay; no periodic stages)

## Context

The spec pipeline is perceive → retrieve → **Reflect** → **Plan** → select → execute → remember. Shipped code still skips periodic Reflect/Plan (M8 timed the actual loop only). M25 added eviction-only `ActionChooser::reflect` and **skips it whenever `replay` is Some**, so live+reflect hashes cannot match a JSONL replay. `ReplayTable` is keyed `(tick, agent)` — a second line for the same agent-tick **overwrites** the choose record.

M26 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb` / `ClientMessage`. It does not add embeddings, TLS, or change `format_version`.

## Goal

A researcher can:

1. Turn on overlay `[llm] reflect_every_n_ticks = N` (or `--llm-reflect-every N`) so a live/`Custom` chooser, on ticks `N, 2N, …`, makes **one insight call** per living agent and inserts a protected `MemoryKind::Reflection`. Mock/Wait skip ⇒ **same hash** as today.
2. Turn on `[llm] plan_every_n_ticks = N` (or `--llm-plan-every N`) so those agents get a short stored **plan** (length from `[llm] plan_length`, default 4). Plan is **not** auto-executed; it is shown to this tick’s `choose`. Mock/Wait skip.
3. Record **all** extra LLM calls (periodic insight, plan, and M25 evict-reflect) into the existing `replay_file` JSONL and replay them **without a second HTTP call**, matching the recording `state_hash`.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. Periodic Reflect / Plan (every N ticks)

Overlay **not** on `ExperimentConfig` (would change `config_hash`). Extend `LlmBarrierParams` (same overlay parser as barrier / `reflect_on_evict`):

```toml
[llm]
reflect_every_n_ticks = 10
plan_every_n_ticks = 10
plan_length = 4
```

CLI: `--llm-reflect-every N`, `--llm-plan-every N`. Omit / `0` = off (current pipeline).

**Who / when** in `step_agent` (spec order): perceive → retrieve → **Reflect** → **Plan** → select → execute → remember.

- Fire when `N > 0 && tick > 0 && tick % N == 0`.
- `Chooser::Custom` only. Mock / Wait: no call, no new Reflection, plan stays empty.
- One-shot per stage (not `--llm-barrier` retries). Timeout / parse / HTTP → skip that stage; action-select still runs. No `LlmWait` for insight/plan (that event is action-select only).
- No hashed `SimEventKind`.
- Seeds: `tick_{t}_agent_{id}_insight_0` and `tick_{t}_agent_{id}_plan_0` (distinct from choose `call_0` and evict `reflect_0`).
- `AgentTiming`: add `reflect_ns` / `plan_ns` with `#[serde(default)]` so old timing JSONL still loads. Not hashed.

**Reflect (insight):**

- New `ActionChooser::insight(&self, seed: u64, obs: &Observation) -> Result<String, ChooseError>` with default `Err(Malformed)`. Keep `reflect(dropped)` for evict-only.
- `Ok` → insert `MemoryKind::Reflection` via the existing remember path (protected; may run M25 evict-reflect **once** if over cap).
- Do **not** adjust other memories’ importance this slice.
- Do **not** rebuild Observation after insight; retrieve already ran. New Reflection is visible next tick.

**Plan:**

- `Agent.plan: Vec<String>` — `#[serde(default, skip)]`, packed into the checkpoint `BoardBlob` (e.g. `plans: BTreeMap<u64, Vec<String>>`) with `#[serde(default)]` so old ckpts load empty.
- `hash_into`: hash each string; **empty plan adds no bytes** ⇒ mock hashes unchanged.
- `ActionChooser::plan(&self, seed: u64, obs: &Observation, max_len: usize) -> Result<Vec<String>, ChooseError>` (default Err).
- After Plan, patch `obs.plan` so **this tick’s** `choose` sees it. Add `Observation.plan: Vec<String>` with `serde(default)`. That changes **prompt_hash** (postcard of Observation) even when empty; it does **not** change `state_hash`. Replay lookup is not keyed on `prompt_hash`.
- Plan is not executed. Cap to `plan_length`.

Do **not** implement embeddings, incentive-forced reflection, or memory importance adjustments.

### B. Record / replay of reflection text

Today `ReplayTable` is `(tick, agent) → response` — a second line overwrites choose.

- Add `ReplayRecord.call` with `#[serde(default)]` (`""` / omitted = **choose**, old JSONL still loads).
- Key: `(tick, agent, call)` where `call` ∈ `choose` | `reflect` | `plan` | `reflect_evict`.
- On record, write a line per extra call. On timeout/skip, write `{"__skip__":true}` (not the choose `LlmWait` sentinel).
- Replay: missing line or skip sentinel ⇒ skip that stage (same as Err). Overlay flags must match the recording run.
- **Change from M25:** when `replay` is Some, evict-reflect is **no longer unconditionally skipped** — it consumes `reflect_evict` lines so live+evict hashes can match.

Live `OpenAiCompatClient`: short JSON `{"reflection":"..."}` / `{"plan":["...", "..."]}`. Seed on the wire like `choose`.

## Out of scope (later)

| Later | What |
|---|---|
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| After M29 | protobuf/TLS; Unix sockets; reflection importance-adjust; hashed pipeline events; combat particles; **browser client**; PG-1/2/3 post-GA |
| Not M26 | Browser; combat; CI Win/mac; PROTOCOL bump; auto-execute plan |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard variants.
2. Every-N is overlay + CLI, not `LlmParams` / `ExperimentConfig`. Mock/Wait skip. One shot, not barrier retries.
3. `Agent.plan` is skip + BoardBlob default (like goals). Empty plan is hash-neutral.
4. Replay grows a `call` field; old JSONL remains action-only. Evict-reflect is replayed, not skipped.
5. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M26 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| overlay every-N + mock | same hash as overlay off |
| Custom insight Ok, N=2, cap large | Reflection present at tick 2; none at tick 1 |
| Custom plan Ok | `agent.plan` non-empty; choose `obs.plan` set this tick |
| Custom insight/plan Err | skip; tick finishes; no Reflection / plan stays empty |
| old JSONL (no `call`) | still action-only replay |
| record + replay Custom + both overlays | recording `state_hash` = replay |
| record + replay `reflect_on_evict` | recording hash = replay (M25 gap closed) |
| overlay off + Custom | no insight/plan calls |
| old ckpt load | empty plan; format_version 2 |
| Hello v5 | unchanged |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Replay call-kind

- **Files:** `llm.rs` `ReplayRecord.call` + table key; skip sentinel; `simulation.rs` record/replay `reflect_evict`; tests with old JSONL + evict-reflect round-trip
- **Changes:** second JSONL line no longer overwrites choose; M25 skip-on-replay removed

### PR 2: Periodic insight + plan

- **Files:** overlay/CLI; `Agent.plan` + BoardBlob; `Observation.plan`; `ActionChooser::insight` / `plan`; `step_agent` stages; stub-chooser tests
- **Dependencies:** PR 1 (replay of the new calls)

### PR 3: live prompts (sim-llm)

- **Files:** `OpenAiCompatClient::insight` / `plan` short JSON prompts; skip if mock URL empty

## Config / CLI

No shipping TOML change. Overlay `[llm] reflect_every_n_ticks`, `plan_every_n_ticks`, `plan_length` are not `ExperimentConfig`. Recording still uses existing `[llm] replay_file`. No new postcard fields. No PROTOCOL bump.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --llm ollama --llm-reflect-every 10 --llm-plan-every 10 \
  --llm-reflect-on-evict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

## Verification

Walkthrough: [`M26-test-plan.md`](M26-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock + every-N overlay same hash; Custom insight writes a protected Reflection on ticks N, 2N; plan is stored and visible to choose; record+replay matches including evict-reflect; Hello stays v5; default mock hashes match.

## Risks

- **One-slot replay overwrite** if extra lines land without a `call` key — PR 1 first.
- **Hashing empty plan with a length prefix** — iterate strings only.
- **`Agent.plan` on the agent postcard body** — skip + BoardBlob default, like goals.
- **`reflect_every_n` on `LlmParams` / `ExperimentConfig`** — overlay only.
- **Barrier × periodic hang** — one-shot; skip on timeout.
- **Postcard Observation new field** — prompt_hash moves; state_hash does not.
- **Periodic Reflection insert evicts** — at most one M25 evict-reflect per remember burst (unchanged).
- **No PROTOCOL bump.** Stay at 5.
