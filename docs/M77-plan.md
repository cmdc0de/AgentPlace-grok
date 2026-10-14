# M77 — Unix sockets for local attach, extra recipes

**Status:** implemented  
**Depends on:** M76 complete (`docs/M76-plan.md`, git tag `M76`, commit `bdb35b4`)  
**Walkthrough:** [`docs/M77-test-plan.md`](M77-test-plan.md)  
**Specs:** [`remaining-features.md`](remaining-features.md) RF-8 / RF-PG9

## Context

Listen and connect today are `tcp://host:port` and `ws://host:port` only. Same postcard frames. `wss` is rejected. `--listen` and `--connect` are mutually exclusive. Catalog after M76 includes girt/sleeve/socle/pretzel.

M77 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (still **writes 3 / reads v2+v3**). Unix listen is overlay, not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged. The socket is **hash-neutral**. New recipes **change** shipped-objects idle hashes. `--no-time` no-objects stays **`70e5204d…`**. Not `wss`, not abstract-namespace sockets, not a protocol bump.

## Goal

A researcher can:

1. **Listen and connect** on a Unix domain socket with the same Hello / Welcome / Tick frames as TCP. `--allow-control` still gates mutation. Without it, attach stays hash-neutral.
2. Keep `tcp://` and `ws://`. Still reject `wss`. **`PROTOCOL_VERSION` stays 5.**
3. **Craft** four new catalog items (**baulk, collar, impost, rusk**). Catalog-off ⇒ those Crafts illegal; `--no-time` no-objects hash stays **`70e5204d…`**.
4. CI stays `provider = mock`.

## In scope

No PROTOCOL bump. No TLS. No new ControlVerb. No new postcard variant.

### A. Unix sockets (RF-8, hash-neutral, protocol 5)

```toml
[network]
unix_listen = "/tmp/agentplace.sock"   # omit = off; not ExperimentConfig
```

CLI, repeatable, same or-with-overlay as tcp/ws:

```text
--listen unix:///tmp/agentplace.sock
--connect unix:///tmp/agentplace.sock
```

Path rule: strip the `unix://` prefix. `unix:///tmp/agentplace.sock` is the absolute path `/tmp/agentplace.sock`. A relative remainder is a relative path. Empty path is an error.

Who/when: same server loop as TCP. Length-prefixed postcard frames. No WebSocket upgrade. Print `listen=unix://…` on stderr after bind. `--listen` and `--connect` stay mutually exclusive. Bind removes a stale socket file at that path before listen. Connect does not create the file.

`cfg(unix)` uses `std::os::unix::net`. On other OSes the binary still compiles; `unix://` returns a clear "not supported on this OS" error and does not bind. Loopback tests are `#[cfg(unix)]`.

Not hashed. Not `config_hash`. `--load` does not open a socket by itself.

Not: abstract-namespace sockets (`unix://@name`), socket permissions beyond the process umask, `wss`.

### B. Extra recipes (RF-PG9)

Object TOML only. Create on **implement**:

| id | inputs | visual |
|---|---|---|
| `baulk` | wood×24 | reuse `wood.glb` |
| `collar` | fiber×24 | reuse `low_poly_cloth.glb` |
| `impost` | stone×22 | reuse `stones_and_grass.glb` |
| `rusk` | food×21 | reuse `plants_ready.glb` |

No `uses` / `station` / interior / ammo / door. Catalog-off illegal. Default CLI shipped-objects idle hash **changes** (document on implement). Missing glb ⇒ M47 sentinel. Keep craft stock under pocket cap (not 2×32).

## Out of scope

| Not M77 | What |
|---|---|
| Full backlog | [`remaining-features.md`](remaining-features.md) |
| Attach | `wss` / TLS; abstract namespace sockets; a protocol bump |
| Also not | Food/Wood strings (**RF-11**); browser Bevy (**RF-15**); flipping shipping TOML |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Unix carries the existing TCP frames. No postcard append.
2. **format_version still writes 3 / reads v2+v3.**
3. `unix://` is a third scheme. Windows builds compile; they do not bind.
4. Hash-neutral and `config_hash`-neutral. Recipes still move shipped idle hashes.
5. Do not change shipping `coop.toml` or `default.toml`. `cargo test` never needs the network.

## Tests (M77 acceptance bar)

| Test | Asserts |
|---|---|
| `--no-time` no-objects 2 ticks | hash `70e5204d…` |
| default CLI 2 ticks | hash `42e90831…` |
| `--no-time` shipped objects | hash `e624f373…` |
| parse `unix:///tmp/agentplace.sock` | path `/tmp/agentplace.sock` |
| `wss://` | still rejected |
| unix loopback Hello | Welcome, protocol 5 (`#[cfg(unix)]`) |
| unix attach without `--allow-control` | hash unchanged |
| `--listen` and `--connect` together | error |
| non-unix `unix://` | clear unsupported error; crate still builds |
| Craft baulk / collar / impost / rusk | from locked inputs |
| catalog-off | those Crafts illegal |
| Hello v5 / format 3 | unchanged |

GPU window / live Spark **not run**.

## PR Plan

### PR 1: Unix listen and connect

- Parse `unix://`; bind/connect on `cfg(unix)`; loopback Hello; hash-neutral; Windows compiles

### PR 2: Recipes

- baulk/collar/impost/rusk; catalog-off illegal; hash tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

On implement: `[network] unix_listen`; `--listen unix://` / `--connect unix://`; baulk/collar/impost/rusk. Update [`docs/cli-reference.md`](cli-reference.md) and [`docs/config-reference.md`](config-reference.md).

```bash
cargo test -p shared
cargo test -p sim-cli --test net
cargo test -p sim-core --test objects
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

## Verification

Walkthrough: [`docs/M77-test-plan.md`](M77-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: `--no-time` no-objects hash `70e5204d…`; `--no-time` shipped-objects `e624f373…`; default (time on) `42e90831…`; Hello v5; format_version 3 write. Unix attach does not move `state_hash`.

## Risks

- **Protocol.** Do not add a frame or a version. A new scheme is not a wire bump.
- **Stale socket.** Unlink before bind. Do not unlink on connect.
- **Windows.** `std::os::unix::net` is unix-only. Gate the bind. Do not fail `cargo test` on Windows for a missing socket.
- **Catalog hash.** Recipes move idle hashes. The socket does not.
- **Pocket cap.** Inputs wood×24 / food×21; tests must not stock 2×32.
- **No protocol / format bump.**
