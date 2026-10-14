# M76 — wasm32 on Windows and macOS, extra recipes

**Status:** implemented  
**Depends on:** M75 complete (`docs/M75-plan.md`, git tag `M75`, commit `8fbcb38`)  
**Walkthrough:** [`docs/M76-test-plan.md`](M76-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-10 / RF-PG9

## Context

`.github/workflows/test.yml` already runs mock tests on Ubuntu, Windows, and macOS. The wasm32 **target** build (`rustup target add` + `cargo build -p sim-wasm --target wasm32-unknown-unknown`) is gated with `if: runner.os == 'Linux'`. Host `cargo test -p sim-wasm` already runs on all three. Catalog after M75 includes noggin/cowl/keystone/farl.

M76 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Shipping `default.toml` / `coop.toml` unchanged. The workflow change is **hash-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not Unix sockets, not browser Bevy, not a new OS.

## Goal

A researcher can:

1. See GitHub Actions **build** `sim-wasm` for `wasm32-unknown-unknown` on **Ubuntu, Windows, and macOS**, same commands, no Linux-only `if`.
2. Keep CI mock-only. No live LLM. No new sim behavior. `state_hash` unchanged by the workflow.
3. **Craft** four new catalog items (**girt, sleeve, socle, pretzel**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No new crate. No browser Bevy.

### A. wasm32 on Win/mac (RF-10, hash-neutral)

In `.github/workflows/test.yml`, drop `if: runner.os == 'Linux'` from the wasm step so the existing matrix runs it:

```yaml
- name: Wasm32 sim-wasm
  shell: bash
  run: |
    rustup target add wasm32-unknown-unknown
    cargo build -p sim-wasm --target wasm32-unknown-unknown
```

Keep `cargo test -p sim-wasm` in the host test step (already all three OSes). Do not add a fourth OS. Do not change `sim-wasm` features.

`crates/sim-core/tests/ci.rs` asserts the workflow still names `ubuntu-latest`, `windows-latest`, `macos-latest`, `wasm32-unknown-unknown`, and `sim-wasm`, and that it does **not** contain `runner.os == 'Linux'`.

Win/mac runners are **not** executed in the local walkthrough. On implement, run the wasm32 build locally only if that target is already installed (no `rustup target add`, which needs the network).

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `girt` | wood×23 | reuse `wood.glb` |
| `sleeve` | fiber×23 | reuse `low_poly_cloth.glb` |
| `socle` | stone×21 | reuse `stones_and_grass.glb` |
| `pretzel` | food×20 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M76 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| CI | a new OS; wasm32 tests beyond `cargo build`; `wasm-pack`; browser Bevy (**RF-15**) |
| Also not | Unix sockets (**RF-8**); PROTOCOL bump; flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. Same wasm32 commands on every matrix OS. Remove the Linux `if`. Do not duplicate the job.
4. Workflow is hash-neutral. Recipes still move shipped idle hashes.
5. Do not change shipping `coop.toml` or `default.toml`. No TLS. `cargo test` never needs the network.

## Tests (M76 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `e3f2686b…` |
| `--no-time` shipped objects | hash `9abb3e07…` |
| `github_actions_workflow_exists` | three OSes; wasm32 target; no `runner.os == 'Linux'` |
| local wasm32 build | run only if the target is already installed; else **not run** |
| Win/mac Actions | **not run** locally |
| Craft girt / sleeve / socle / pretzel | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: wasm32 on all three OSes

- Drop the Linux `if`; extend `ci.rs`

### PR 2: Recipes

- girt/sleeve/socle/pretzel; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. No new CLI flag. No PROTOCOL bump.

On implement: the workflow step above; girt/sleeve/socle/pretzel. Update [`docs/cli-reference.md`](cli-reference.md) only if it names the Linux-only wasm step.

```bash
cargo test -p sim-core --test ci
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M76-test-plan.md`](M76-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `9abb3e07…`; default (time on) `e3f2686b…`; Hello v5; format_version 3 write. The workflow does not move `state_hash`.

## Risks

- **Windows wasm32.** `wasm32-unknown-unknown` is a rustc target (no C toolchain). `sim-wasm` is already a cdylib with `wasm-bindgen` only on that arch. If Actions fails, fix the step, not the sim.
- **Network.** Do not `rustup target add` in `cargo test`. The workflow does that on the runner.
- **Catalog hash.** Recipes move idle hashes. The YAML does not.
- **Pocket cap.** Inputs wood×23 / food×20; tests must not stock 2×32.
- **No protocol / format bump.**
