# M31 — Kinship, reproduction, D&D-like sheet

**Status:** implemented  
**Depends on:** M30 complete (`docs/M30-plan.md`, git tag `M30`, commit `2b90dfd`)  
**Walkthrough:** [`M31-test-plan.md`](M31-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-1/2/3), `simulation-and-agents-spec.md` (personality, spawn), `M28-plan.md` (`health` skip + BoardBlob)

## Context

Population is spawned at `Simulation::new` and only shrinks. `RelationshipSummary` is feelings, not kinship. There is no STR/DEX/CON sheet. Post-GA PG-1/2/3 are designed to ship **together**: founders roll a sheet; pair-bond; Reproduce writes parent/child and a **calculated** child sheet.

M31 **does not** bump `PROTOCOL_VERSION` (stays **5**). Overlay is not `ExperimentConfig`. No TLS/protobuf. No browser. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Set overlay `[agents.sheet] enabled = true` (or `--sheet`) so founders **roll** STR/DEX/CON/INT/WIS/CHA (3–18) from `agent_init`. Overlay **off** ⇒ no sheet bytes, idle hash still `70e5204d…`.
2. Set `[population] reproduction = true` (or `--reproduction`, **implies `--sheet`**) so two **pair-bonded** adjacent living agents can `Reproduce`; a new `AgentId` spawns with a **calculated** sheet and kinship links.
3. Inspect **Sheet** (scores + D&D modifiers) and **Family** (parent/child/sibling/pair-bond) in the viewer. Observation/prompt can name “parent #3”.
4. Overlay off / mock (never picks PairBond/Reproduce) ⇒ **same hashes** as M30. CI `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**.

## In scope

### A. Physical sheet (PG-3)

Overlay, not `ExperimentConfig`:

```toml
[agents.sheet]
enabled = true
```

CLI: `--sheet`.

- Six `u8` scores on the agent: STR DEX CON INT WIS CHA. **0 = unused** (not hashed). Overlay on: founders roll **3d6** per score (3–18) from `agent_init`.
- Modifier `(score as i32 - 10) / 2` (D&D: 10–11 → +0, 12–13 → +1).
- `#[serde(default, skip)]` + BoardBlob like `health`. Hash the six bytes only if any ≠ 0.
- Overlay **off:** modifiers 0 / current constants (`HEALTH_MAX` 10_000, current haul/perception). Overlay **on:** CON may set health max as `10000 + mod * 500` (hashed when on, as expected).
- Inspector Sheet pane: `STR 14 (+2)` …. No HTTP, no network roll.

### B. Kinship (PG-1)

Do **not** overload `RelationshipSummary`. A kinship map (parent/child/sibling/pair-bond) packed in BoardBlob; **hash only when non-empty**.

| Link | When |
|---|---|
| parent / child | Written both ways at birth |
| sibling | Shared parents |
| pair-bond | `PairBond { target }` — Chebyshev 1, living, not incapacitated, v1 **monogamy** |

No household. No `applies_to = "kin_of:N"` this slice. Observation/prompt may name “parent #3” when a parent link exists. Overlay off + no births ⇒ empty map ⇒ same hash.

### C. Reproduction (PG-2)

```toml
[population]
reproduction = true
```

CLI: `--reproduction` implies `--sheet`.

- `PrimaryAction::PairBond { target }` and `Reproduce { with }` **append** at the end of the enum.
- `Reproduce` legal only if overlay on, both living, not incapacitated, Chebyshev **1**, **mutual pair-bond**, energy above a small floor (lock on implement, e.g. half max).
- New id = max id ever used + 1. Spawn on a land cell next to a parent. Child RNG label `tick_{t}_birth_{id}`.
- Child sheet: per score `clamp((p1 + p2 + 1) / 2 + noise, 3, 18)` with noise ∈ {−1, 0, +1} from the child stream. Personality / abilities mix the same way (not a fresh 3d6).
- Append `SimEventKind::PairBonded { with }` hash tag **29**, `Born { parent_a, parent_b }` tag **30**.
- Mock **never** chooses PairBond/Reproduce (same pattern as Attack). Custom/LLM may.
- Overlay off: actions not legal; no births; **same hash**.

## Out of scope (later)

| Later | What |
|---|---|
| After M31 | protobuf/TLS; Unix sockets; reflection importance-adjust; hashed pipeline events; **browser client**; `kin_of` incentives; household; aging |
| Not M31 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** Postcard enums **append only**.
2. Overlay `[agents.sheet]` / `[population]` is **not** `ExperimentConfig`. Empty sheet + empty kinship add **no** hash bytes.
3. `--reproduction` implies `--sheet`. Reproduce **requires** mutual pair-bond.
4. Founders **roll** 3d6; children are **calculated**. Do not re-roll the child sheet.
5. Mock never picks PairBond/Reproduce. Shipping TOML unchanged. `format_version = 2`.

## Tests (M31 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| sheet overlay off | no sheet bytes; same hash as today |
| `--sheet` | founders 3–18; hash ≠ overlay off; ckpt round-trip |
| reproduction overlay off | PairBond/Reproduce not legal |
| mock + `--reproduction`, no custom action | same hash as overlay off |
| PairBond + Reproduce | new agent; `Born`; parent/child/sibling; child scores calculated not re-rolled 3d6 |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Sheet

- **Files:** overlay `[agents.sheet]` / `--sheet`; six scores skip+BoardBlob; 3d6 roll; hash if any ≠ 0; inspector Sheet pane

### PR 2: Kinship + PairBond

- **Files:** kinship map BoardBlob; `PairBond` action + tag 29; Family pane; Observation/prompt parent names

### PR 3: Reproduce / Born

- **Files:** `[population] reproduction` / `--reproduction`; `Reproduce` + `Born` tag 30; child spawn, calculated sheet, sibling links

## Config / CLI

No shipping TOML change. Overlay is not postcard. `--reproduction` implies `--sheet`. Event / action enums **append only**. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --quiet
```

## Verification

Walkthrough: written on implement (`docs/M31-test-plan.md`).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged with overlays off; sheet on rolls 3–18; birth writes kinship and a calculated child sheet; Hello stays v5.

## Risks

- **Postcard append only** for `PrimaryAction` and `SimEventKind`. PROTOCOL stays **5**.
- Overlay is **not** checkpointed; sheet/kinship **are** (BoardBlob). `--load` must **not** re-roll founders.
- New agent needs an RNG stream and a land cell; no land neighbor → Wait, no birth.
- Mock must not pick PairBond/Reproduce (same as Attack).
- Empty kinship / all-zero sheet add **no** hash bytes.
- Double-apply on `--load`: do not run founder 3d6 again.
