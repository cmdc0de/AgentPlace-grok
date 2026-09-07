# M34 — Sheet effects, close-kin PairBond

**Status:** implemented  
**Depends on:** M33 complete (`docs/M33-plan.md`, git tag `M33`, commit `6a884fb`)  
**Walkthrough:** [`docs/M34-test-plan.md`](M34-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-3 effects, PG-2 incest), `docs/M31-plan.md` (sheet + PairBond)

## Context

M31 shipped STR/DEX/CON/INT/WIS/CHA and inspector modifiers. Only CON changes the sim (`health_max`). PairBond is illegal for self, children-by-age, and existing mates — not for parent/child/sibling.

M34 **does not** bump `PROTOCOL_VERSION` (stays **5**). Overlay is not `ExperimentConfig`. No TLS/protobuf. No browser. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[agents.sheet]` / `--sheet` and see STR/DEX/WIS/CHA **change sim numbers** (not just inspector text). CON health max already ships. Score 0 / unused ⇒ today’s constants.
2. See **close-kin PairBond refused**: parent, child, or sibling cannot PairBond (legal list + execute Wait). Unrelated founders still can. Overlay off / no kinship ⇒ **same hashes**.
3. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged.

## In scope

No new overlay flags. Sheet and kinship already exist. Effects are **derived from scores / links**, not extra blob fields. `--load` cannot double-apply.

### A. Close-kin PairBond

`close_kin(a, b)` iff `b` is in `a`’s parents, children, or siblings (and the reverse). Not pair-bond (already monogamy). Not household-only.

- `legal_actions`: PairBond omitted when `close_kin`.
- `execute::pair_bond`: Wait if close-kin.
- Reproduce still requires mutual pair-bond, so this also blocks incest births.
- Do not dissolve existing bonds. Mock never picks PairBond ⇒ mock hashes unchanged.

### B. Sheet effects

Existing `[agents.sheet] enabled` / `--sheet`. Modifier already `(score-10)/2`, **0 if score 0**. Apply whenever the score is non-zero (same as CON `health_max`). Overlay flag only **rolls** founders; it does not gate runtime math (ckpt scores still work after `--load` without re-roll).

| Score | Effect (integer millipoints / cells) |
|---|---|
| STR | Attack damage = `ATTACK_DAMAGE (2000) + STR_mod * 250`, clamp ≥ 1. Energy cost stays 500. |
| DEX | Move energy = `base_move.saturating_add_signed(-DEX_mod * 40)`, then min 1 if base was > 0. |
| CON | Unchanged (health max already `10000 + CON_mod * 500`). |
| WIS | After `effective_range`, add `WIS_mod` cells (i32, floor 0) to vision / hearing / identity. |
| CHA | Influence **vote weight** = `influence_factor + CHA_mod * 100` (then `.max(1)` as today). **Do not write** `influence_factor`. Equal/respect votes unchanged. |
| INT | **Not this slice** (retrieval_k / memory cap). |

No gather-payoff rewrite, no inventory-cap rewrite, no illness resistance.

Inspector already shows `STR 14 (+2)`. No new Observation field required.

## Out of scope (later)

| Later | What |
|---|---|
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| After M39 | protobuf/TLS/`wss`; Unix sockets; hashed pipeline events; DEX accuracy; STR haul |
| Not M34 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard variants.
2. No new overlay. Score 0 ⇒ modifier 0 ⇒ today’s constants. Overlay off / unused sheet ⇒ **same hashes**.
3. Close-kin is blood links only (parents/children/siblings). Household-only is not close-kin.
4. CHA bonus is derived at vote time, not stored. `--load` does not double-apply.
5. Do not change shipping `coop.toml`. `format_version = 2`.

## Tests (M34 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| sheet unused / overlay off | STR/DEX/WIS/CHA mods 0; same damage/move/range as today |
| STR 18 vs 10 | Attack damage differs (`2000 + 4*250` vs `2000`) |
| DEX 18 vs 3 | Move cost lower for 18 |
| WIS 18 vs 3 | vision/identity range larger for 18 |
| CHA 18 + influence votes | weight uses `influence + 400`; equal votes unchanged |
| siblings / parent-child | PairBond not legal; execute Wait |
| unrelated adjacent + reproduction | PairBond still legal |
| Hello v5 | unchanged |

## PR Plan

### PR 1: Close-kin PairBond

- **Files:** `close_kin` helper; legal + execute; population tests

### PR 2: Sheet effects

- **Files:** `AbilitySheet` helpers; Attack damage; `move_cost_milli`; `effective_range` callers; `vote_weight_of` influence path

## Config / CLI

No shipping TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --conflict --quiet
```

## Verification

Walkthrough: [`docs/M34-test-plan.md`](M34-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged with unused sheets; STR/DEX/WIS/CHA change numbers when rolled; close-kin PairBond Wait; Hello stays v5.

## Risks

- Derived mods must be **0 at score 0** so unused sheets add no hash bytes beyond what M31 already hashes when rolled.
- DEX move changes mock energy when sheet is on (already hash-different via CON health). Overlay **off** must not touch move cost.
- WIS range can change legal actions when sheet is on. Overlay off: same Observation.
- CHA bonus is **not** stored; `--load` + influence votes stay bit-identical if scores restore.
- **No PROTOCOL bump.** Stay at 5.
- Postcard enums: **no new variants**.
