# M37 — Extra invention kinds, browser /set /give

**Status:** implemented  
**Depends on:** M36 complete (`docs/M36-plan.md`, git tag `M36`, commit `77367e3`)  
**Walkthrough:** [`docs/M37-test-plan.md`](M37-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-5, PG-7 later), `docs/M35-plan.md` (GatherBonus), `docs/M24-plan.md` (wire `/set`), `docs/M22-plan.md` (wire Give)

## Context

M35 shipped overlay `[inventions]` with one closed kind: **GatherBonus**. A second Invent Waits. M36 `web/index.html` lists agents, board, and metrics from `Tick.metrics.inspector` (Play/Pause only). Snapshot wasm decode was optional; `/set` `/give` are on the wire already (`ControlVerb`) but not in the page.

M37 **does not** bump `PROTOCOL_VERSION` (stays **5**). No new `ControlVerb`. Overlay is not `ExperimentConfig`. No TLS/`wss`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[inventions]` / `--inventions` and Invent **more than GatherBonus**: closed extra kinds **MoveBonus** and **SenseBonus**, same inventor-then-society split, no stack.
2. See those kinds in Observation and in the browser inspector (id, inventor, kind, shared).
3. From `web/index.html` with `--allow-control`, send existing `/set` and `/give` (same millipoint rules as the CLI). Without the flag → `ControlDisabled`.
4. After Snapshot, fill tables from checkpoint bytes when wasm is present; without wasm, Tick `metrics.inspector` still works (M36). CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**.

## In scope

No new overlay keys. Same `[inventions] enabled` / `share_delay_ticks` / `--inventions`. Mock still never picks Invent.

### A. Extra invention kinds

Append only on `InventionKind` (u8):

| Tag | Kind | Effect (inventor immediately; society after `share_delay_ticks`) |
|---|---|---|
| 0 | `GatherBonus` | Unchanged: 1.2× food gather; +200 influence; no stack |
| 1 | `MoveBonus` | Move energy `cost * 800 / 1000` (min 1 if cost was > 0) |
| 2 | `SenseBonus` | +1 cell on vision / hearing / identity after WIS |

- Invent is legal while **any** of the three kinds is missing. Kind = **lowest missing tag** (empty table ⇒ GatherBonus). M35 `execute Invent` still gets GatherBonus first.
- Chance stream stays `tick_{t}_agent_{id}_invent_0` (`400 + INT_mod*50` vs `rng%1000`). Fail → Wait. Duplicate kind → Wait.
- Each success: new table row; inventor `influence_factor` saturating_add 200 (clamp 10_000); protected memory; `SimEventKind::Invented { inventor, kind }` (tag 31, kind u8 append).
- Society: `shared = true` after `share_delay_ticks` from **that row’s** tick. Inventor does not stack (0.8× then 0.8×, not 0.64×; +1 then +1, not +2).
- `--load` restores the table; do **not** re-grant influence.
- Overlay off / empty table ⇒ **same hashes**.
- Observation lines per kind (`invention move_bonus (yours)` / `invention move_bonus`, same for sense). `InspectorView` lists inventions.

Not this slice: tech tree, patents, LLM invention text, stealing recipes, CraftBonus, STR haul, DEX accuracy.

### B. Browser `/set` `/give` + Snapshot helper

Existing `ControlVerb::Give` / `ControlVerb::Set`. **No PROTOCOL bump.** Postcard `ClientMessage::Control` as today.

- Page forms: `/give ID ITEM QTY`, `/set ID hunger|thirst|energy|influence N`, `/set ID respect TOWARD N`. Encode postcard (document bytes in shared tests). Play/Pause unchanged.
- Server already requires `--allow-control`. Do not add `/inject` `/scrub` this slice.
- `InspectorView::from_checkpoint_bytes` (decode AGTN → `from_sim`). Rust tests are the Snapshot acceptance bar.
- Optional `wasm32` wrapper of that helper. Page `import`s it on Snapshot if present; missing wasm ⇒ M36 Tick inspector. `cargo test` never needs `wasm32-unknown-unknown`.
- Hash-neutral until `/set` `/give` actually mutates.

## Out of scope (later)

| Later | What |
|---|---|
| After M37 | protobuf/TLS/`wss`; Unix sockets; hashed pipeline events; PG-6 3D; DEX accuracy; STR haul; tech tree / patents; `/inject` `/scrub` in the page; time-series charts; requiring wasm32 in CI |
| Not M37 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Give/Set already exist. Do not add Tick struct fields.
2. Lowest-missing kind (not a new RNG stream) so first Invent stays GatherBonus.
3. `--inventions` does not imply `--sheet`. Unused INT ⇒ invent millipoints 400.
4. wasm32 is optional at runtime; Rust `from_checkpoint_bytes` is the Snapshot bar.
5. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M37 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| overlay off | Invent not legal; same hash |
| mock + overlay on | same hash as off |
| first Invent | still GatherBonus; influence +200; 1.2× food |
| second Invent | MoveBonus; inventor cheaper move; others full cost until share |
| third Invent | SenseBonus; inventor +1 range; others unchanged until share |
| after each share_delay | that kind `shared`; no stack |
| fourth Invent | Wait (all kinds present) |
| `--load` | table restored; influence not granted twice |
| page | PROTOCOL 5; `/give` `/set` encode Control; `#inventions` |
| `/set` `/give` | `--allow-control` mutates (hash-sensitive); without flag `ControlDisabled` |
| Snapshot helper | `from_checkpoint_bytes` matches `from_sim`; wasm optional |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Extra kinds

- **Files:** `InventionKind` append; legal/lowest-missing; MoveBonus / SenseBonus millipoints; Observation + InspectorView inventions; tests

### PR 2: Browser control + Snapshot helper

- **Files:** page `/set` `/give`; postcard byte tests; `from_checkpoint_bytes`; optional wasm load; loopback net tests

## Config / CLI

No shipping TOML change. No new overlay keys. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --inventions --quiet
# open web/index.html → Connect; /give /set if --allow-control
```

## Verification

Walkthrough: [`docs/M37-test-plan.md`](M37-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash unchanged; extra kinds inventor-then-society; page still v5; `/set` `/give` ControlDisabled without flag.

## Risks

- **Append only** on `InventionKind`. Tag 0 stays GatherBonus so old ckpts and M35 tests load.
- Lowest-missing kind so first Invent hashes like M35.
- `--load` must not re-grant +200 influence.
- **No PROTOCOL bump.** Give/Set already exist.
- wasm32 is optional; do not fail CI if the target is missing.
- `/set` `/give` are hash-sensitive; read-only attach must stay hash-neutral.
