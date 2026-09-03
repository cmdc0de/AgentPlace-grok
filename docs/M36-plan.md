# M36 — Browser researcher UI (no 3D)

**Status:** implemented  
**Depends on:** M35 complete (`docs/M35-plan.md`, git tag `M35`)  
**Walkthrough:** [`docs/M36-test-plan.md`](M36-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-7), `docs/M35-plan.md` (postcard WS attach)

## Context

M35 `web/index.html` attaches on `ws://` with postcard **v5** and shows tick + `state_hash` (Play/Pause). That is a log tail, not an inspector. The imgui viewer already has per-agent family/sheet/timing; the browser does not.

M36 **does not** bump `PROTOCOL_VERSION` (stays **5**). No TLS/`wss`. No protobuf. No 3D. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Open `web/index.html`, connect to existing `ws://`, and after Snapshot see **every living agent** (id, cell, hunger/thirst/energy, health, sheet, inventory/pack, goals, last action).
2. See **friendships** (trust/affinity/respect/fear), **kinship**, and **board posts** (open + adopted).
3. See **metrics**: last-tick wall-clock stages **and** sim aggregates (hungry/thirsty/tired counts, mean trust, illness).
4. CI stays `provider = mock`. Attach is **hash-neutral**. `format_version = 2`. **`PROTOCOL_VERSION = 5`**.

## In scope

No new overlay. No new `ControlVerb`. `Subscribe` + `RequestSnapshot` already exist.

### A. InspectorView

- JSON built from `Simulation` in `sim-core` (`InspectorView::from_sim`). **Not hashed.** Not `ExperimentConfig`.
- One row per living agent: id, x, y, hunger/thirst/energy (display 0–100), health, sheet scores when non-zero, inventory/pack counts, goals, last primary kind, relationship rows, kinship lines.
- Board: open proposals (id, author, text/rule, support/oppose counts) and adopted rules.
- Metrics: copy of last `TickTiming` if present; counts of agents with hunger/thirst/energy below half max; mean trust; illness count.

### B. Browser tables

- Hello v5 → `RequestSnapshot` → decode checkpoint (wasm wrapping `decode_checkpoint`, or an `inspector_json` helper). `Subscribe { want_events, want_decisions }` so Tick JSON already on the wire fills last action + `Tick.metrics`.
- Tables/text only. **No 3D.** Missing wasm ⇒ page still shows M35 tick/hash and Tick JSON if present; **Rust `InspectorView` is the acceptance bar.**
- Play/Pause still optional with `--allow-control`. Do not add `/set` `/give` this slice.
- Hash-neutral attach. Do not emit a hashed connect event.

## Out of scope (later)

| Later | What |
|---|---|
| After M36 | protobuf/TLS/`wss`; Unix sockets; hashed pipeline events; PG-6 3D; DEX accuracy; STR haul; `/set` `/give` in the page; time-series charts |
| Not M36 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Do not add Tick struct fields (postcard change). Use Snapshot bytes + existing JSON metrics/events.
2. `InspectorView` is display-only and **not** in `state_hash`.
3. wasm decode is optional at runtime; unit tests prove the JSON shape without a GPU or a window.
4. No new overlay or `ControlVerb`.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M36 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| InspectorView | one row per living agent; kin + relationship fields; board open/adopted; timing + hungry counts |
| attach | Hello v5; Snapshot path hash-neutral |
| page | still postcard v5; agent/board/metrics sections (or fixture JSON) |
| Hello v5 | unchanged |

## PR Plan

### PR 1: InspectorView

- **Files:** `from_sim`; unit tests; not hashed

### PR 2: Browser tables

- **Files:** Snapshot/Tick → agent list, posts, metrics; loopback test

## Config / CLI

No shipping TOML change. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
# open web/index.html → Connect
```

## Verification

Walkthrough: [`docs/M36-test-plan.md`](M36-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash unchanged; InspectorView complete; page still v5; attach hash-neutral.

## Risks

- **No PROTOCOL bump.** Do not add Tick fields.
- wasm decode is optional; Rust `InspectorView` is the bar.
- Large Snapshot must not stall CI (headless tests use Rust JSON, not a GPU).
- Attach must stay hash-neutral.
