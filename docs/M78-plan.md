# M78 — Bevy primitives in the browser, extra recipes

**Status:** implemented  
**Depends on:** M77 complete (`docs/M77-plan.md`, git tag `M77`, commit `0a3f818`)  
**Walkthrough:** [`docs/M78-test-plan.md`](M78-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-15 / RF-PG9

## Context

The native `viewer` crate is Bevy 0.19 with imgui, Draco glTF, and jpeg. `sim-wasm` only turns a checkpoint into inspector JSON. The page does not run Bevy. Catalog after M77 includes baulk/collar/impost/rusk.

M78 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Shipping `default.toml` / `coop.toml` unchanged. The browser app is **hash-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. This is a primitive browser scene, not a port of the native viewer. Glbs, imgui, and attach stay later (**RF-27**).

## Goal

A researcher can:

1. Open the existing page and see a **Bevy** canvas: a ground plane and one marker per agent from an in-page `Simulation` (fixed tiny seed, a few ticks). No window on the CI machine.
2. Keep the native `viewer` crate unchanged. Missing browser wasm does not break the tables, SQL, or `ws://` client.
3. **Craft** four new catalog items (**rafter, coif, corbel, cruller**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

## In scope

No PROTOCOL bump. No new ControlVerb. No imgui. No glTF. No `wss`.

### A. Browser Bevy (RF-15, hash-neutral)

New crate `crates/viewer-web`:

- Depends on `sim-core` and `bevy` with **default features off** and only the features needed for a wasm 3D scene. Do not use workspace `bevy` as-is (that enables jpeg). No `bevy_mod_imgui`. No `bevy_gltf_draco`.
- `#[cfg(target_arch = "wasm32")]` entry calls `run()`. It builds one `Simulation` from a fixed tiny config (seed locked in the crate, 8×8 or the crate’s own small map, 2 agents), runs 2 ticks, then spawns a ground plane and one cube (or capsule) per agent at that agent’s cell. A fixed camera looks at the plane.
- A pure helper `agent_marks(sim) -> Vec<(u32, u32)>` is tested on the host with no window and no GPU.
- `web/index.html` loads `./pkg/viewer_web.js` into a canvas when the file exists. On failure, the rest of the page still works (same pattern as optional `sim_wasm.js`).
- CI: the existing wasm step also runs `cargo build -p viewer-web --target wasm32-unknown-unknown` on Ubuntu, Windows, and macOS. Do not `wasm-bindgen` or `rustup target add` inside `cargo test`.
- `ci.rs` asserts the workflow names `viewer-web`.

Not hashed. Not `config_hash`. Does not open a display in `cargo test`.

Not: native viewer changes, agent glbs, skeletal clips, imgui panels, `--connect` inside the browser app, publishing the `.wasm` bytes into git.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `rafter` | wood×25 | reuse `wood.glb` |
| `coif` | fiber×25 | reuse `low_poly_cloth.glb` |
| `corbel` | stone×23 | reuse `stones_and_grass.glb` |
| `cruller` | food×22 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M78 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Browser Bevy | glTF/Draco, imgui, attach, clips, the native viewer (**RF-27**) |
| Also not | Food/Wood strings (**RF-11**); protobuf/TLS (**RF-13**); flipping shipping TOML (**RF-17**) |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. A new crate, not a wasm build of `viewer`. The native app stays native.
4. The browser scene is hash-neutral. Recipes still move shipped idle hashes.
5. Do not change shipping `coop.toml` or `default.toml`. `cargo test` never needs the network or a display.

## Tests (M78 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `7ea74fe4…` |
| `--no-time` shipped objects | hash `7aad440e…` |
| `agent_marks` | one mark per agent at that agent’s cell; no window |
| `github_actions_workflow_exists` | workflow builds `viewer-web` for wasm32 |
| local `cargo build -p viewer-web --target wasm32-unknown-unknown` | run only if the target is already installed; else **not run** |
| Win/mac Actions / live canvas | **not run** locally |
| Craft rafter / coif / corbel / cruller | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: viewer-web

- Crate, primitive scene, host test of `agent_marks`, optional page hook, CI line

### PR 2: Recipes

- rafter/coif/corbel/cruller; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. No new CLI flag. No PROTOCOL bump.

On implement: `crates/viewer-web`; the page hook; the CI line; rafter/coif/corbel/cruller.

```bash
cargo test -p viewer-web
cargo test -p sim-core --test ci
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M78-test-plan.md`](M78-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `7aad440e…`; default (time on) `7ea74fe4…`; Hello v5; format_version 3 write. The browser crate does not move `state_hash`.

## Risks

- **Bevy on wasm.** Default features and jpeg will not compile for `wasm32-unknown-unknown`. Keep the crate’s Bevy features minimal. If Windows CI fails, fix the crate features, not the sim.
- **Display.** `cargo test` must not open a window. Only the pure helper runs in tests.
- **Page.** A missing `viewer_web.js` must not break SQL or the websocket client.
- **Catalog hash.** Recipes move idle hashes. The browser app does not.
- **Pocket cap.** Inputs wood×25 / food×22; tests must not stock 2×32.
- **No protocol / format bump.**
