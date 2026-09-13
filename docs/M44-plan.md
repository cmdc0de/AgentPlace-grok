# M44 — WIS board range, CHA support/pair-bond, STR pocket weight

**Status:** implemented  
**Depends on:** M43 complete (`docs/M43-plan.md`, git tag `M43`, commit `0f9377d`)  
**Walkthrough:** [`docs/M44-test-plan.md`](M44-test-plan.md)  
**Specs:** `docs/post-ga-feature-list.md` (PG-4 remaining WIS/CHA/STR)

## Context

M43 shipped WIS toxin detect, CHA speech range/weight, and DEX flee. WIS already adds vision/hear/ident; open board posts still use **identity** range (same cells as naming an author). CHA already weights Influence votes and speech; Support social deltas ignore CHA, and PairBond always succeeds if legal. STR already raises pocket **slots** and worn-pack caps; pockets have no weight check.

M44 **does not** bump `PROTOCOL_VERSION` (stays **5**) or `format_version` (stays **2**). Overlay is not `ExperimentConfig`. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Turn on `[agents.sheet]` / `--sheet` and see WIS **see open board posts farther than identity** (still name authors only within ident). Unused WIS ⇒ board = ident as today.
2. See CHA change **Support social deltas** and **PairBond success**. Unused CHA ⇒ SUPPORT (200, 0, 100, 0) and PairBond always succeeds if legal.
3. See STR add a **pocket weight cap** (slots already STR). Unused STR ⇒ slots only, no pocket weight check (today).
4. `--load` restores scores; do **not** persist derived board cells / odds / weight cap. CI stays `provider = mock`. **`PROTOCOL_VERSION = 5`**.

Idle mock 2-tick hash **stays `cd1e0853…`**.

## In scope

No PROTOCOL bump. No new postcard variants. No new overlay keys. Existing `[agents.sheet] enabled` / `--sheet`. Derived at use time (same as M34–M43). Score 0 ⇒ today’s constants.

### A. WIS board range

Today open proposals use **identity** range (`open_proposal_visible` / `board_view`). Ident already includes WIS via `adjust_range`. Remaining use: board **farther than names**.

| Use | Formula (WIS 0 ⇒ identity) |
|---|---|
| Board cells | `ident + max(0, WIS_mod)` |

- Open post visible if author Chebyshev ≤ board cells (author/self/stance still always visible).
- Author **id** on the post still requires ident (unnamed silhouette beyond ident).
- `public_board_always_visible` / `full_information` unchanged.
- WIS 10 (mod 0) ≡ unused. Low WIS does not shrink board below ident.

### B. CHA proposal support + pair-bond odds

CHA already weights **Influence** votes (`influence_vote_weight`). Equal/respect tallies stay 1 / respect-sum. PairBond today always succeeds if legal (adjacent, unbound, not close-kin). Mock does not pick PairBond.

| Use | Formula (CHA 0 ⇒ identity) |
|---|---|
| Support social | apply `(max(0, 200 + CHA_mod*50), 0, max(0, 100 + CHA_mod*25), 0)` instead of `SUPPORT`. SUPPORT_BACK / OPPOSE unchanged. |
| PairBond | CHA **0** ⇒ always succeed. Else Invent-style `derive_seed(master, "tick_{t}_agent_{id}_pair_bond_0")` (not `RngBank.ensure`): hit if `d20 + CHA_mod >= 10`. Miss ⇒ `Wait`, no bond. |

- Do not change `SUPPORT` const globally; scale at `apply_delta` time.
- Do not change vote **counts**. Support/Oppose still insert the agent id as today.
- CHA 10 (mod 0): social identity; PairBond still rolls `d20 >= 10` (same pattern as DEX accuracy). Unused sheet never rolls.

### C. STR pocket weight cap

Pockets today: **slot cap only** (`try_add_item`). Worn pack already has STR-adjusted weight. Land crate unchanged.

| Use | Formula (STR 0 / mod 0 ⇒ identity) |
|---|---|
| Pocket weight | STR_mod **0** ⇒ no weight check. Else `max(1, 8000 + STR_mod * 250)` milli (crate-like base 80.0). |

- Use in `try_add_item` / legal Gather/Give/Unpack (and any other pocket insert).
- 16 stones = 4800 milli, so unused and STR 18 (20 slots, cap 9000) still fit default cargo; STR 3 (cap 7250) is tighter on heavy stacks.
- Do **not** write a cap onto the agent. `--load` cannot double-apply.

## Out of scope (later)

| Later | What |
|---|---|
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | Done — [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| **M49** | Done — [`M49-plan.md`](M49-plan.md) — object visual scale, extra recipes, invention flavor text |
| **M50** | Done — [`M50-plan.md`](M50-plan.md) — viewer FPS HUD, sqlite run log, extra recipes |
| **M51** | [`M51-plan.md`](M51-plan.md) — sqlite metrics page, weapon combat bonuses, extra recipes |
| After M51 | protobuf/TLS/`wss`; Unix sockets; skeletal animation; wasm32 on Win/mac; Food/Wood as strings; sql.js / ad-hoc SQL; weapon range; spear melee |
| Not M44 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml`; Bevy in the browser; extra LLM; changing Equal vote counts |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new postcard variants. No new overlay keys.
2. Board extra cells are `max(0, WIS_mod)` only. Low WIS does not shrink below ident.
3. PairBond RNG skips when CHA is 0. Invent-style `derive_seed`, not `RngBank.ensure`. Mock does not pick PairBond.
4. Equal vote **counts** unchanged. Support social only.
5. Pocket weight check only when STR_mod ≠ 0. Unused stays slots-only.
6. Do not change shipping `coop.toml`. `format_version = 2`. No TLS.

## Tests (M44 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `cd1e0853…` |
| sheet unused | board = ident; SUPPORT 200/100; PairBond always (if legal); no pocket weight check; same hash as no `--sheet` |
| WIS 18 vs 0 | 18 sees an open post 4 cells past ident; 0 does not; author unnamed past ident |
| WIS 10 | board = ident |
| CHA 18 vs 3 Support | social trust/respect 400/200 vs 50/25; vote id still one supporter |
| CHA 0 PairBond | always succeeds if legal |
| CHA 18 vs 3 PairBond | 18 succeeds more often on the seeded d20; miss is Wait |
| STR 18 vs 3 | pocket weight cap 9000 vs 7250; 3 rejects a stack 18 accepts |
| `--load` | scores restored; no stored board/weight/odds |
| Hello v5 | unchanged |

## PR Plan

### PR 1: WIS board range

- **Files:** helper; `open_proposal_visible` / `board_view`; unused identity tests

### PR 2: CHA Support social + PairBond odds

- **Files:** scale SUPPORT at apply_delta; pair_bond `derive_seed` roll; unused skip; identity tests

### PR 3: STR pocket weight

- **Files:** helper; `try_add_item` / legal pocket inserts; unused no-check tests

## Config / CLI

No shipping experiment TOML change. Overlay is not postcard. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --quiet
```

## Verification

Walkthrough: [`docs/M44-test-plan.md`](M44-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
```

Expect: idle mock hash `cd1e0853…`; unused sheet identity; WIS/CHA/STR change numbers when rolled; Hello v5.

## Risks

- Board must not shrink below ident for low WIS. Extra cells are `max(0, WIS_mod)` only.
- PairBond RNG: **skip when CHA is 0**. `RngBank.ensure` would move unused fingerprints. Mock does not pick PairBond ⇒ idle hash unchanged.
- Do not change Equal vote **counts** (would re-litigate M13). Support social only.
- Pocket weight: STR_mod 0 must **not** start rejecting (today unlimited). Cap only when STR_mod ≠ 0.
- `--load`: never store derived board cells, pair-bond flags, or pocket weight cap.
- Shipping `default.toml` / `coop.toml` unchanged. Overlay is not postcard. No PROTOCOL bump.
