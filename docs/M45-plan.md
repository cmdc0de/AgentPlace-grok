# M45 — Tech tree/patents, hashed pipeline events, string ItemId ckpt bump

**Status:** implemented  
**Depends on:** M44 complete (`docs/M44-plan.md`, git tag `M44`, commit `23611b8`)  
**Walkthrough:** [`docs/M45-test-plan.md`](M45-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-5 tree/patents, PG-8 string ItemId), later-tables (hashed pipeline events)  
**Researcher docs:** [`cli-reference.md`](cli-reference.md), [`config-reference.md`](config-reference.md)

## Context

M35–M37 shipped Invent (GatherBonus / MoveBonus / SenseBonus) with inventor-then-society share. There is no prerequisite chain and no extra patent delay. Tick timing (`perceive_ns` …) is hash-neutral. `ItemId::Catalog(u16)` is a sorted-file rank, so adding a catalog file can reshuffle old checkpoints. `format_version` is **2**.

M45 **does not** bump `PROTOCOL_VERSION` (stays **5**; ItemId is not on the wire). **`format_version` writes 3** and still **reads v2**. Shipping `default.toml` / `coop.toml` unchanged. Overlay is not `ExperimentConfig`.

## Goal

A researcher can:

1. Turn on `[inventions] tree = true` and see Invent **require the previous kind to be society-shared** (GatherBonus → MoveBonus → SenseBonus). `patent_ticks` keeps the inventor exclusive past `share_delay`. Overlay off / defaults ⇒ today’s invent order and share timing.
2. Turn on `[pipeline] hash_events` / `--pipeline-events` and see **one hashed pipeline event per living agent per tick** (stage bitmask). Wall-clock ns stay unhashed. Overlay off ⇒ no extra events, **same hashes**.
3. Save checkpoints as **format_version 3** with catalog items as **slugs**, not shuffled `u16`. Load v2 (u16 catalog) and v3 (slug). Built-in Food/Wood/… encoding and hashes **unchanged**.
4. `--load` restores inventions, pipeline events, and catalog slugs without double-grant. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.
5. Open [`cli-reference.md`](cli-reference.md) and [`config-reference.md`](config-reference.md) for every `sim-cli` / viewer flag and every file under `configs/`.

Idle mock 2-tick hash **stays `cd1e0853…`**.

## In scope

Postcard: **append only** for `SimEventKind`. Do not reorder `ItemId` builtins.

### A. Tech tree + patents

Existing `[inventions] enabled` / `--inventions`. Mock still does not pick Invent.

New overlay keys (defaults = today):

```toml
[inventions]
enabled = true
share_delay_ticks = 8
tree = false           # omit = false
patent_ticks = 0       # omit = 0
```

| Key | Today (false / 0) | On |
|---|---|---|
| `tree` | `next_kind` = first missing kind (no prereq) | next kind only if the previous in `ALL` is **shared** (society). Inventor-only GatherBonus does **not** unlock MoveBonus. |
| `patent_ticks` | share at `invent_tick + share_delay` | share at `invent_tick + share_delay + patent_ticks`. Inventor stays `entitled` the whole time. |

- Do not add LLM invention text. Do not add a new `StructuredRule` patent variant.
- Inventions overlay off ⇒ Invent illegal, empty table, **same hashes**.
- `--inventions` does not imply `--sheet` or `tree`.

### B. Hashed pipeline events

New overlay, not ExperimentConfig:

```toml
[pipeline]
hash_events = false    # omit = false
```

CLI: `--pipeline-events`.

When **on**: after remember, append **one** `SimEventKind::Pipeline { stages: u8 }` per living agent (append at end of the enum; hash tag **32**). Bits: perceive=1, retrieve=2, select=4, execute=8, remember=16. A completed mock step is `31`. Skip/Wait still sets the bits for stages that ran.

- **Do not hash** `perceive_ns` / wall clocks. `TickTiming` stays hash-neutral.
- Overlay off: no `Pipeline` events, idle hash **unchanged**.
- `--load`: restore events; do not emit extras on decode.

### C. String catalog ItemId + format_version 3

`CHECKPOINT_FORMAT_VERSION` **writes 3**. Magic `AGTN` unchanged.

| | v2 (read) | v3 (write + read) |
|---|---|---|
| Built-in ItemId | same postcard tags as today | **same** |
| `ItemId::Catalog` | `u16` rank of sorted slugs | **slug string** |
| Unknown leftover | keep Catalog(n) | keep slug; if unknown on load, do not re-grant, do not panic |

- Runtime may still use `Catalog(u16)` internally; v3 **file** stores the slug so adding a later catalog file does not reshuffle old ckpts.
- Hash of builtins unchanged. Catalog-on hash uses **slug bytes**, not u16 (stable if the same slug is held).
- Overlay catalog off / no catalog items ⇒ Catalog never appears ⇒ idle hash **unchanged**.
- PROTOCOL stays 5. Wire does not carry ItemId.

### D. Researcher CLI / config reference

Docs only (no hash / protocol change):

- [`docs/cli-reference.md`](cli-reference.md) — every `sim-cli` and viewer flag, slash commands, hash/overlay rules.
- [`docs/config-reference.md`](config-reference.md) — every file under `configs/`, every hashed experiment key, every overlay table, incentive files, object TOML fields.

## Out of scope (later)

| Later | What |
|---|---|
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | Done — [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| **M52** | Done — [`M52-plan.md`](M52-plan.md) — day/night clock, world-size CLI, OTLP export |
| **M53** | Done — [`M53-plan.md`](M53-plan.md) — sleep places (N×N), night Hunt/Farm gating, CON dawn bonus |
| **M54** | [`M54-plan.md`](M54-plan.md) — sleep pickup, household auto-cabin, spear melee + range, extra recipes |
| After M54 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL |
| Not M45 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; hashing wall-clock ns |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** ItemId is checkpoint postcard, not the wire.
2. **Write format_version 3; read v2 and v3.** M44 `--load` still works.
3. Pipeline and tree/patent default off/identity. Overlay off / unused ⇒ **same hashes**.
4. Do not hash wall-clock ns. Pipeline event is a stage bitmask only.
5. Catalog slug is the string-ItemId slice. Built-in Food/Wood/… stay enum tags.
6. Do not change shipping `coop.toml`. No TLS.

## Tests (M45 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `cd1e0853…` |
| inventions off / tree false / patent 0 | same invent order and share tick as today |
| tree on | MoveBonus illegal until GatherBonus **shared** |
| patent_ticks = 4 | society not entitled until invent+delay+4 |
| pipeline overlay off | no Pipeline events; same hash as no flag |
| pipeline on | one Pipeline per living agent per tick; stages=31 for mock complete; hash ≠ off |
| write ckpt | `format_version = 3` |
| load v2 | Catalog(u16) still decodes; builtins restore |
| load v3 catalog slug | slug restores; extra catalog file does not reshuffle |
| Hello v5 | unchanged |
| CLI / config docs | `docs/cli-reference.md` and `docs/config-reference.md` list flags and `configs/` files |

## PR Plan

### PR 1: Tech tree + patents

- **Files:** `[inventions] tree` / `patent_ticks`; `next_kind` prereq; share clock; unused identity tests

### PR 2: Hashed pipeline events

- **Files:** overlay / CLI; append `Pipeline { stages }`; hash tag 32; ns not hashed; off identity

### PR 3: format_version 3 + catalog slug

- **Files:** write v3; read v2+v3; catalog slug in file; builtin hash identity

### PR 4: Researcher CLI / config docs

- **Files:** `docs/cli-reference.md`, `docs/config-reference.md`; INDEX / README / walkthrough links

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --inventions --pipeline-events --catalog --quiet
```

## Verification

Walkthrough: [`docs/M45-test-plan.md`](M45-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: idle mock hash `cd1e0853…`; tree/patent/pipeline off identity; format_version 3 write; v2 load; Hello v5.

## Risks

- **Idle hash:** pipeline default off; inventions tree/patent default identity; builtin ItemId tags unchanged.
- Do not hash wall-clock ns (non-deterministic).
- Postcard: append `SimEventKind::Pipeline` at the **end**. Do not reorder ItemId builtins.
- v2 load must still work or researchers lose `--load` of M44 runs.
- Catalog slug hash ≠ old u16 hash for catalog-on runs (acceptable; document).
- Shipping `default.toml` / `coop.toml` unchanged. No PROTOCOL bump.
