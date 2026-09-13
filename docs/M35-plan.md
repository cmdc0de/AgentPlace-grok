# M35 — Inventions, browser attach

**Status:** implemented  
**Depends on:** M34 complete (`docs/M34-plan.md`, git tag `M34`)  
**Walkthrough:** [`docs/M35-test-plan.md`](M35-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-5), `docs/M7-plan.md` (WS attach, no browser)

## Context

Craft recipes are fixed. There is no inventor credit and no society-wide unlock. Attach is TCP/WS postcard v5 (imgui viewer or `sim-cli --connect`); there is no browser page.

M35 **does not** bump `PROTOCOL_VERSION` (stays **5**). Overlay is not `ExperimentConfig`. No TLS/`wss`. No protobuf. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on overlay `[inventions] enabled = true` (or `--inventions`) so a living agent may **Invent**. Overlay off ⇒ action illegal, empty table, **same hashes**. Mock never picks Invent.
2. See two payoffs stay distinct: **inventor** gets an immediate private benefit; **society** gets the same effect only after `share_delay_ticks`.
3. Open a **browser page** that attaches to existing `ws://` (Hello v5, postcard), shows tick + hash (read-only unless `--allow-control`). Attach is hash-neutral.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping TOML unchanged.

## In scope

### A. Inventions

Overlay, not `ExperimentConfig`:

```toml
[inventions]
enabled = true
share_delay_ticks = 8        # omit = 8; society unlock after this many ticks
```

CLI: `--inventions` (does **not** imply `--sheet`). INT_mod is 0 if the sheet is unused.

- Append `PrimaryAction::Invent` (end of enum). Legal when overlay on, living, not incapacitated, not a child (if aging on).
- Append `SimEventKind::Invented { inventor, kind }` hash tag **31**.
- Closed v1 kind: **GatherBonus** only (food gather millipoint multiplier). No LLM text, no tech tree.
- Seeded roll `tick_{t}_agent_{id}_invent_0`. Success millipoints `400 + INT_mod * 50` vs `rng % 1000`. Fail → Wait. Tests may `execute_primary(Invent)`. Mock never chooses Invent.
- Table `inventions: BTreeMap<u64, Invention>` in the board blob (`id`, inventor, tick, kind, shared: bool). Hash only when non-empty.
- **Inventor immediately:** `influence_factor` saturating_add 200 (clamped 0–10_000); food gather multiplier 1.2× for that agent (same millipoint path as incentives). Protected memory “invented gather bonus”.
- **Society after `share_delay_ticks`:** `shared = true`; every living agent gets the 1.2× food gather (inventor does not stack to 1.44). Observation: `invention gather_bonus (yours)` for the inventor from tick 0; `invention gather_bonus` for everyone once shared.
- One GatherBonus in flight per run (second Invent → Wait). `--load` restores the table; do **not** re-grant influence.
- Overlay off / empty table ⇒ **same hashes**.

### B. Browser attach

- Static page + `shared` postcard codec (wasm or equivalent) on **existing** `ws://`. **No PROTOCOL bump.** No `wss`/TLS. Token on Hello as today.
- Read-only: Hello → Welcome → Tick `tick` + `state_hash` (same as `sim-cli --connect` log tail). Snapshot optional if decode is cheap.
- Control (`Play` / `Pause`) only when the server has `--allow-control`, same as the CLI tail.
- Hash-neutral attach (existing net tests pattern).
- CI: loopback WS; page/wasm builds without fetching models from the network. Not a Bevy/imgui port. Not PG-6 3D models.

## Out of scope (later)

| Later | What |
|---|---|
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| After M52 | protobuf/TLS/`wss`; Unix sockets; sql.js / ad-hoc SQL; weapon range; spear melee |
| Not M35 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Browser speaks postcard, not JSON.
2. `--inventions` does not imply `--sheet`. Unused INT ⇒ invent millipoints 400.
3. Inventor vs society payoffs must not stack. Empty invention table is not hashed.
4. Mock never picks Invent. Overlay off ⇒ same hashes as today.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M35 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| inventions overlay off | Invent not legal; same hash as today |
| mock + overlay on | same hash as overlay off |
| execute Invent | table + Invented event; inventor influence +200; inventor 1.2× food; others 1.0× |
| after share_delay | `shared`; all living 1.2×; no stack |
| `--load` | table restored; influence not granted twice |
| INT unused vs 18 | unused mod 0; 18 raises invent millipoints (unit) |
| Hello v5 | unchanged |
| browser/ws attach | Welcome + Tick hash; hash-neutral; v5 |

## PR Plan

### PR 1: Inventions

- **Files:** overlay / `--inventions`; `Invent` + `Invented` tag 31; blob table; inventor vs society payoff

### PR 2: Browser attach

- **Files:** static page + postcard-on-WS (PROTOCOL 5); loopback test

## Config / CLI

No shipping TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --inventions --quiet
```

## Verification

Walkthrough: [`docs/M35-test-plan.md`](M35-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged with overlay off; Invent inventor-then-society; browser WS Hello v5 hash-neutral.

## Risks

- Postcard enums **append only** (`Invent`, `Invented`).
- Overlay is **not** checkpointed; the invention table **is**. `--load` must not re-apply inventor influence.
- Empty table / overlay off add **no** hash bytes. Mock must skip Invent.
- Browser must speak **postcard v5**, not JSON (JSON would bump PROTOCOL).
- **No PROTOCOL bump.** Stay at 5. No TLS.
