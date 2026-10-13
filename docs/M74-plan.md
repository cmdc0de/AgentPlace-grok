# M74 — System-1 model id for action choose, extra recipes

**Status:** implemented  
**Depends on:** M73 complete (`docs/M73-plan.md`, git tag `M73`, commit `8b12e02`)  
**Walkthrough:** [`docs/M74-test-plan.md`](M74-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-26 / RF-PG9

## Context

One `[llm] model` is sent for choose, insight, plan, reflect, and invent flavor. Mock ignores it. `LlmParams` is on `ExperimentConfig`, so a new field there would change postcard `config_hash` and fail old checkpoint checks. Catalog after M73 includes purlin/gaiter/cornice/bagel plus arrow.

M74 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). `system1_model` is **overlay**, not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. The key is **hash-neutral** and **`config_hash`-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not a heuristic chooser, not dual-station, not a second URL.

## Goal

A researcher can:

1. Set `[llm] system1_model` (or `--llm-system1-model`) so the **action-choose** HTTP call uses that model id. Insight, plan, reflect, invent flavor, and importance keep `[llm] model`. Omit ⇒ choose uses `model` too.
2. Keep CI on `provider = mock`. Mock and `wait` never open a socket. Setting the key does not change `state_hash` or `config_hash`.
3. **Craft** four new catalog items (**batten, wimple, voussoir, bannock**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new `PrimaryAction` / `SimEventKind`. Do **not** add a field to `LlmParams` / `ExperimentConfig`.

### A. System-1 model id (RF-26, hash-neutral, config_hash-neutral)

```toml
[llm]
model = "nemotron3:33b"       # insight, plan, reflect, invent flavor, importance; choose when system1_model is empty
system1_model = "grok-4-fast" # omit or "" = choose uses model
```

CLI: `--llm-system1-model NAME` **or** with the overlay key. Empty CLI does not clear a TOML value. No new shipping line in `default.toml`.

| Call | Model id |
|---|---|
| `choose` | `system1_model` if non-empty after trim, else `model` |
| `insight`, `plan`, `reflect`, `invent_flavor`, `importance` | `model` only |

The chat body (`model`, and the existing grok vs non-grok `think` / `response_format` branch) uses the **model string of that call**, not always `self.model`.

`provider = mock` or `wait`: ignore the key. No request. Same actions as today.

Not hashed into `state_hash`. Not on the postcard config. `config_hash` identical with the key set, omitted, or `--llm-system1-model`.

`--load` does not re-send a choose. The key is read from the experiment TOML / CLI at process start, same as other `[llm]` overlays.

Tests build the JSON body (or a pure helper) and assert the model field. They do **not** call the network.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `batten` | wood×21 | reuse `wood.glb` |
| `wimple` | fiber×21 | reuse `low_poly_cloth.glb` |
| `voussoir` | stone×19 | reuse `stones_and_grass.glb` |
| `bannock` | food×18 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M74 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Decisioning | a non-LLM System-1 heuristic; dropping insight/plan/reflect; a second provider or URL; per-agent model ids |
| Also not | dual-station (**RF-23**); Unix sockets (**RF-8**); wasm32 Win/mac (**RF-10**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No postcard append. `system1_model` is overlay, like `barrier`.
2. **format_version still writes 3 / reads v2+v3.**
3. Choose is the only call that reads `system1_model`. Other calls stay on `model`.
4. Hash-neutral and `config_hash`-neutral. Recipes still move shipped idle hashes.
5. Do not change shipping `coop.toml` or `default.toml`. No TLS. `cargo test` never needs the network.

## Tests (M74 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `62b5ce21…` |
| `--no-time` shipped objects | hash `22e66700…` |
| overlay omit | choose model id = `model` |
| `system1_model` set | choose body uses that id; insight/plan/reflect/flavor stay on `model` |
| grok vs non-grok | `think` / `response_format` follow the **call** model |
| mock + key set | no HTTP; `state_hash` equals omit; `config_hash` equals omit |
| `--load` | no extra choose |
| Craft batten / wimple / voussoir / bannock | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark / live xAI **not run**.

## PR Plan

### PR 1: System-1 model id

- Overlay parse + `--llm-system1-model`; choose uses it; other calls do not; body tests; hash and `config_hash` unchanged

### PR 2: Recipes

- batten/wimple/voussoir/bannock; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `[llm] system1_model`; `--llm-system1-model`; batten/wimple/voussoir/bannock. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M74-test-plan.md`](M74-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `22e66700…`; default (time on) `62b5ce21…`; Hello v5; format_version 3 write. `system1_model` does not move `state_hash` or `config_hash`.

## Risks

- **Postcard / `config_hash`.** A new `LlmParams` field breaks old checkpoint checks. Keep it off `ExperimentConfig`.
- **Request shape.** `post_once` today keys `think` / `json_object` off `self.model`. A grok choose model with a Nemotron `model` must not share one branch.
- **Mock.** Never dial out when the key is set.
- **Catalog hash.** Recipes move idle hashes. The model key does not.
- **Pocket cap.** Inputs wood×21 / food×18; tests must not stock 2×32.
- **No protocol / format bump.**
