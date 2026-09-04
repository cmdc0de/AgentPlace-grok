# M29 — Local embeddings, combat death, CI Win/mac

**Status:** implemented  
**Depends on:** M28 complete (`docs/M28-plan.md`, git tag `M28`, commit `da75c86`)  
**Walkthrough:** [`M29-test-plan.md`](M29-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` (optional embeddings, not in `state_hash`), `M5-plan.md` (`enable_embeddings = false` default; no network embed), `medium-priority-specs.md` §2 (`death_enabled` after incapacitation)

## Context

`[agents.memory] enable_embeddings` exists on `ExperimentConfig` and is **false** in shipping `default.toml`; retrieval ignores it. M28 Attack at 0 health **incapacitates**; there is no combat death flag. There is **no** `.github` workflow (Linux/Win/mac CI never ran in-repo).

M29 **does not** bump `PROTOCOL_VERSION` (stays **5**). No browser client. It does not flip shipping `enable_embeddings`. No TLS/protobuf.

## Goal

A researcher can:

1. Set `[agents.memory] enable_embeddings = true` so retrieval uses a **local, deterministic** vector (no HTTP, not in `state_hash`). Shipping default stays `false` ⇒ idle **sim hashes** unchanged (`70e5204d…` at 2 ticks mock).
2. Set overlay `[conflict] death_enabled = true` (or `--conflict-death`) so 0 health **removes** the agent (`CombatDeath` event) instead of only incapacitating. Overlay off ⇒ M28 behaviour, same hashes.
3. Rely on **GitHub Actions** `cargo test` on Ubuntu, Windows, and macOS (mock, no network). Viewer GUI is not driven in CI.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged.

## In scope

### A. Local embeddings

`enable_embeddings` already exists on `MemoryParams` / `ExperimentConfig` (default **false**, already in `configs/default.toml`). Do **not** flip the shipping default.

When **true**:

- Each `MemoryEntry` may hold a small vector (`#[serde(default, skip)]`, **not** `hash_into`).
- Fill with a **seeded local projection** of `text` (bag-of-bytes / deterministic hash features). No model download, no network.
- `retrieve` ranks by cosine(query, entry) mixed with existing importance/recency. Query vector from retrieve context, same projection.
- When **false**: today’s importance × recency only. Vectors unused.

Tests never call an embed API. Turning the flag **on** in an experiment TOML **does** change `config_hash` (field already hashed); default file stays false so idle `state_hash` is unchanged.

### B. Combat death

Overlay, not `ExperimentConfig`:

```toml
[conflict]
enabled = true
death_enabled = true
```

CLI: `--conflict-death` (implies `--conflict`).

- **Off (default):** M28 — health 0 ⇒ incapacitated, tag 27, agent stays.
- **On:** health 0 ⇒ do **not** leave them incapacitated for later ticks; reap this tick (with hunger/thirst reap). Append `SimEventKind::CombatDeath { by }` hash tag **28**. Existing `Died { hunger, thirst }` unchanged.
- Overlay off + mock ⇒ **same hash** as M28.

### C. CI Win/mac

Add `.github/workflows/test.yml`:

- `ubuntu-latest`, `windows-latest`, `macos-latest`
- `cargo test -p sim-core`, `-p shared`, `-p sim-cli --test net`, `-p viewer` (loopback / headless only)
- If a runner has no display, skip imgui/windowed viewer tests and document that in the workflow
- No network, no live LLM

**Browser client** is **not** this slice.

## Out of scope (later)

| Later | What |
|---|---|
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| After M38 | protobuf/TLS; Unix sockets; hashed pipeline events |
| Not M29 | PROTOCOL bump; flipping shipping `enable_embeddings` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.**
2. Embeddings are local projection only; vectors never enter `state_hash`. Shipping default stays false.
3. Combat death is overlay; new event tag 28 append-only. Do not add fields to `Died`.
4. CI matrix is mock tests. Browser attach is M35; TLS/`wss` stays later.
5. Do not change shipping `configs/default.toml` / `coop.toml`. `format_version = 2`.

## Tests (M29 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| `enable_embeddings = false` | retrieve order unchanged vs today |
| `enable_embeddings = true`, local vectors | retrieve can prefer text-similar memories; **state_hash** ignores vectors; no network |
| conflict death overlay off + Attack to 0 health | incapacitated, agent remains (M28) |
| `--conflict-death` + Attack to 0 health | agent removed; `CombatDeath` event |
| mock + death overlay, no Attack | same hash as overlay off |
| Hello v5 | unchanged |
| workflow file exists; `cargo test -p sim-core` / `-p viewer` / `-p sim-cli --test net` / `-p shared` | no network |

## PR Plan

### PR 1: Embeddings

- **Files:** `MemoryEntry` skip vector; local projection; `retrieve` mix; unit tests flag on/off

### PR 2: Combat death

- **Files:** overlay `[conflict] death_enabled` / `--conflict-death`; `CombatDeath` tag 28; reap vs incapacitate; combat tests

### PR 3: CI

- **Files:** `.github/workflows/test.yml` ubuntu/windows/macos matrix

## Config / CLI

No shipping TOML change. `enable_embeddings` stays false in `default.toml`. Overlay `[conflict] death_enabled` is not `ExperimentConfig`. `--conflict-death` implies `--conflict`. Event enum **append only**. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --conflict --conflict-death --quiet
```

## Verification

Walkthrough: written on implement (`docs/M29-test-plan.md`).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged; embeddings on does not change `state_hash`; combat death removes the agent; workflow YAML present; Hello stays v5.

## Risks

- **`enable_embeddings` on `ExperimentConfig`** — already hashed; do not default true.
- **Vectors in `state_hash`** — skip; only retrieval changes when on.
- **Neural/HTTP embed** — forbidden in CI; local projection only.
- **`Died` postcard** — do not add fields; new `CombatDeath` variant at end.
- **Viewer tests on Win/mac CI** — may skip imgui if no display; document in the workflow.
- **No PROTOCOL bump.** Stay at 5.
