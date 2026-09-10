# M42 — CON illness/energy, INT memory, browser /ckpt /events

**Status:** planned (not yet implemented)  
**Depends on:** M41 complete (`docs/M41-plan.md`, git tag `M41`, commit `7d6baff`)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 CON/INT, PG-7 `/ckpt` `/events`)

## Context

M41 shipped DEX accuracy and STR haul. CON still only changes `health_max`. INT already raises Invent chance; memory cap (128) and `retrieval_k` (8) ignore the score. `web/index.html` has `/set` `/give` `/inject` `/scrub`; `/ckpt` and `/events` exist on the wire (`ControlVerb` tags 8–10) but not in the page.

M42 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[agents.sheet]` / `--sheet` and see CON change **energy max** and **how long toxic-eat illness lasts**. Unused CON ⇒ today’s energy cap and `illness_ticks = 12`.
2. See INT change **memory capacity** and **retrieval_k**. Unused INT ⇒ capacity 128 / k 8. Invent chance already uses INT (unchanged).
3. From `web/index.html` with `--allow-control`, send `/ckpt next|prev` and `/events TICK`. Without the flag → `ControlDisabled`.
4. `--load` restores scores; do **not** persist derived energy/memory caps. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Idle mock 2-tick hash **stays `cd1e0853…`**.

## In scope

No PROTOCOL bump. No new postcard variants. No new overlay keys. Existing `[agents.sheet] enabled` / `--sheet`. Derived at use time (same as M34/M41). Score 0 ⇒ today’s constants.

### A. CON illness duration + energy max

`AbilitySheet::modifier` already 0 if score 0. CON `health_max` unchanged.

| Use | Formula (CON 0 ⇒ identity) |
|---|---|
| Energy max | `max(1, energy_max_milli as i32 + CON_mod * 500)` |
| Illness duration | on toxic eat: `illness_ticks = max(1, 12 - CON_mod)` (today `ILLNESS_TICKS = 12`) |

- Rest / eat / regen clamp to the **derived** energy max, not a stored field.
- Illness **chance** stays “always on toxic eat” (no extra RNG). Extra energy decay while ill stays as today.
- CON 10 (mod 0) ≡ unused for these formulas.

### B. INT memory cap + retrieval_k

Invent chance already `400 + INT_mod*50`. Do not change it.

| Use | Formula (INT 0 ⇒ identity) |
|---|---|
| Memory cap | `max(8, memory_capacity() as i32 + INT_mod * 4)` (`MIN_MEMORY_CAPACITY = 8`; default 128) |
| retrieval_k | `max(1, retrieval_k as i32 + INT_mod)` (default 8) |

- `remember` / retrieve / reflect-on-evict use the helper. Do **not** write a cap onto the agent.
- INT 10 (mod 0) ≡ unused.

### C. Browser `/ckpt` `/events`

Existing wire. **No PROTOCOL bump.**

| Command | Postcard |
|---|---|
| `/ckpt next` | `ClientMessage::Control` tag 3, `CkptNext` tag **8** |
| `/ckpt prev` | tag 3, `CkptPrev` tag **9** |
| `/events TICK` | tag 3, `Events(u64)` tag **10** + varint tick |

- Page: next/prev buttons + events-tick input, enabled with `--allow-control` like `/inject` `/scrub`.
- Server already `apply_ckpt_step` / `apply_events`. ReportReady lines go to the page log.
- Without `--allow-control` → `ControlDisabled`.
- Document encode bytes in shared tests (same pattern as `encodeScrub`).

Do not add time-series charts, wasm32 on Win/mac, or `/save` in the page.

## Out of scope (later)

| Later | What |
|---|---|
| After M42 | protobuf/TLS/`wss`; Unix sockets; hashed pipeline events; agent meshes; tech tree / patents; string ItemId / ckpt bump; skeletal animation; hot-reload glb; WIS toxin detect; CHA speech; pocket **weight** cap; DEX flee bonus; time-series charts; wasm32 on Win/mac |
| Not M42 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Do not reorder `ControlVerb`. Page tags 8/9/10 match Rust.
2. No new overlay. Score 0 ⇒ modifier 0 ⇒ today’s constants. Overlay off / unused sheet ⇒ **same hashes**.
3. Do not persist derived energy max or memory cap. `--load` cannot double-apply.
4. Illness duration only (no chance roll). Extra ill energy decay unchanged.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M42 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `cd1e0853…` |
| sheet unused | energy max = config; illness 12; memory 128; k 8; same hash as no `--sheet` |
| CON 18 vs 3 | energy max higher for 18; toxic eat illness ticks 8 vs 15 |
| INT 18 vs 3 | memory cap 144 vs 116; retrieval_k 12 vs 5 |
| `--load` | scores restored; no stored derived cap; no extra illness ticks |
| page encode | `encodeCkptNext` / `encodeCkptPrev` / `encodeEvents` postcard matches Rust |
| `/ckpt` `/events` | `--allow-control` mutates / filters; without flag `ControlDisabled` |
| Hello v5 | unchanged |

## PR Plan

### PR 1: CON energy max + illness duration

- **Files:** helpers on `AbilitySheet`; rest/eat clamp; toxic eat duration; unused identity tests

### PR 2: INT memory + retrieval_k

- **Files:** helpers; remember/retrieve; unused identity tests

### PR 3: Browser `/ckpt` `/events`

- **Files:** `web/index.html` encode + buttons; shared postcard tests; net ControlDisabled

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo test -p shared
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --sheet --quiet
# page: web/index.html  /ckpt next|prev  /events TICK
```

## Verification

Walkthrough: write `docs/M42-test-plan.md` on implement.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash `cd1e0853…`; unused sheet identity; CON/INT change numbers when rolled; page `/ckpt` `/events`; Hello v5.

## Risks

- **`--load`:** never assign derived energy max or memory cap onto the agent; recompute from CON/INT.
- Illness duration must not fire extra RNG (would change unused-sheet hashes if we rolled chance).
- Memory floor 8: INT 3 on a tiny overlay cap must not go below `MIN_MEMORY_CAPACITY`.
- Postcard: do not reorder `ControlVerb`. Page tags 8/9/10 must match Rust.
- Shipping `default.toml` / `coop.toml` unchanged.
