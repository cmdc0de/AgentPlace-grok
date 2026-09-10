# M25 — Reflection-on-evict, lockstep Ack timeout

**Status:** implemented  
**Depends on:** M24 complete (`docs/M24-plan.md`, git tag `M24`, commit `8980d01`)  
**Walkthrough:** [`M25-test-plan.md`](M25-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` (summarise-then-drop), `M8-plan.md` / `M9-plan.md` (reflection-on-evict deferred), `M24-plan.md` (lockstep wait-forever)

## Context

Memory eviction today **drops** the victim with no summary. Spec “summarise then drop” and the M8/M9 later-table **reflection-on-evict** never shipped. M24 `--lockstep` waits **forever** for `AckTick`.

M25 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb` / `ClientMessage`. It does not add embeddings, TLS, or change `format_version`.

## Goal

A researcher can:

1. Turn on overlay `[llm] reflect_on_evict = true` (or `--llm-reflect-on-evict`) so when a live/`Custom` chooser is in use and a memory is evicted, one extra LLM call summarises the dropped text into a **Reflection** memory (protected). Mock/Wait skip the call ⇒ **same hash** as today.
2. Set `--lockstep-timeout-ms N` (or `[network] lockstep_timeout_ms`) so a lockstep server that does not get `AckTick(T)` within *N* ms **continues** to *T+1* instead of hanging. `0` / omit = M24 wait-forever.
3. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged ⇒ default **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).

## In scope

### A. Reflection-on-evict

Spec (`memory-goals-incentives-spec.md`): when over capacity, lowest-score entries are dropped **or summarised into a reflection**.

**Append** `MemoryKind::Reflection` (after `Norm`). `hash_into` uses `kind as u8` — do **not** reorder. New code loads old ckpts. Mock never writes Reflection ⇒ hashes unchanged.

Overlay **not** on `ExperimentConfig` (would change `config_hash`):

```toml
[llm]
reflect_on_evict = true
```

CLI: `--llm-reflect-on-evict`.

When **on** and `Chooser::Custom`:

- On each eviction victim (before `remove`), collect dropped `text`. After the `while store.len() > cap` loop, **one** `ActionChooser::reflect(call_seed, dropped_texts)` call (not one per victim).
- `Ok(summary)` → insert `MemoryEntry { kind: Reflection, text: summary, importance: high (e.g. 200), … }`. `Reflection` is **protected** (`protected_kind`) so it is not immediately re-evicted. If still over cap, evict **without** a second reflect this tick (no recursion).
- `Err` (timeout / HTTP / parse) → skip summary; victims stay dropped. Do **not** use `--llm-barrier` retries for reflect (one shot). No `LlmWait` event (that is action-select). Do **not** add a hashed `SimEventKind`.
- `Chooser::Mock` / `Wait` / overlay **off**: current drop-only path. Overlay on + mock still drop-only.
- **Skip reflect when `replay` is Some** so replay of action JSONL is unchanged. Live+reflect hashes are not replay-identical until a later slice records reflections.

`ActionChooser::reflect(&self, seed: u64, dropped: &[String]) -> Result<String, ChooseError>` with a default `Err(Malformed)` so existing choosers compile. Live `OpenAiCompatClient` posts a short summarise prompt (JSON `{"reflection":"..."}`). Seed on the wire like `choose`.

Do **not** implement embeddings (`enable_embeddings` stays unused). Do **not** implement periodic Reflect/Plan stages.

### B. Lockstep Ack timeout

No protocol bump. `AckTick` unchanged.

```
sim-cli --listen tcp://… --lockstep --lockstep-timeout-ms 5000
```

Overlay:

```toml
[network]
lockstep = true
lockstep_timeout_ms = 5000
```

- Omit / `0` = wait forever (M24).
- `N > 0`: `wait_lockstep_acks` returns after all subscribed `AckTick(T)` **or** *N* ms elapsed, then the serve loop may run *T+1*.
- Still **do not hold** the hub mutex while waiting.
- Zero subscribers ⇒ no wait (unchanged).
- Hash-neutral (wall clock only). CI mock never requires it.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M42** | [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| After M42 | protobuf/TLS; Unix sockets |
| Not M25 | Browser; combat; CI Win/mac; hashed pipeline events; PROTOCOL bump |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard variants.
2. Reflection-on-evict is overlay + CLI, not `LlmParams`. Mock/Wait/replay skip. One shot, not barrier retries.
3. `MemoryKind::Reflection` is **append-only** and **protected**.
4. Lockstep timeout `0` = M24 forever. Overlay `[network] lockstep_timeout_ms`, not `ExperimentConfig`.
5. Do not change shipping `configs/default.toml` / `coop.toml`. Mock CI. `format_version = 2`.

## Tests (M25 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| overlay `reflect_on_evict` + mock | same hash as overlay off (no extra call) |
| Custom chooser `reflect` Ok + cap 2 + 3 remembers | store has a `Reflection`; dropped texts not all present |
| Custom `reflect` Err | victims dropped; no Reflection; tick still finishes |
| replay table set | no `reflect` call |
| `MemoryKind::Reflection` append | old ckpt still loads |
| `--lockstep-timeout-ms 0` + no Ack | still waits (existing wait test) |
| `--lockstep --lockstep-timeout-ms 200` + subscribed dummy, no Ack | tick advances past 1 within ~1s |
| `--lockstep` 0 subscribers | finishes `--ticks` |
| Hello v5 | unchanged |
| `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: `MemoryKind::Reflection` + reflect-on-evict hook

- **Files:** `memory.rs` kind + protected; `ActionChooser::reflect`; `remember` path calls chooser when overlay on; overlay `[llm] reflect_on_evict` / `--llm-reflect-on-evict`; unit tests with stub chooser

### PR 2: live reflect prompt (sim-llm)

- **Files:** `OpenAiCompatClient::reflect` short summarise prompt; skip if mock URL empty

### PR 3: lockstep timeout

- **Files:** `wait_lockstep_acks` deadline; `--lockstep-timeout-ms` / `[network] lockstep_timeout_ms`; net test dummy no-Ack then tick advances

## Config / CLI

No shipping TOML change. Overlay `[llm] reflect_on_evict` and `[network] lockstep_timeout_ms` are not `ExperimentConfig`. No new postcard fields.

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --lockstep --lockstep-timeout-ms 5000 \
  --llm ollama --llm-reflect-on-evict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

## Verification

Walkthrough: [`M25-test-plan.md`](M25-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock + reflect overlay same hash; Custom reflect writes a protected Reflection; lockstep timeout lets ticks continue; Hello stays v5; default mock hashes match.

## Risks

- **`MemoryKind` reorder would change hashes.** Append only.
- **Reflect events hashed.** Do not add `SimEventKind` this slice.
- **Reflect during replay diverges.** Skip when `replay` is Some.
- **Recursion:** reflection insert evicts again. Protected kind; at most one reflect per remember/evict burst.
- **Barrier × reflect hangs overnight.** Reflect is one-shot; timeout = skip.
- **Lockstep timeout on `ExperimentConfig`.** Overlay + CLI only.
- **No PROTOCOL bump.** Stay at 5.
