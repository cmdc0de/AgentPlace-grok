# M34 test plan — see each new feature

Walkthrough for [`M34-plan.md`](M34-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (sheet effects, close-kin PairBond). Next slice: [`M53-plan.md`](M53-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test population`; unit tests under `--lib` need the module path (`sheet::tests::…`).

**Success for the slice:** unused sheet keeps today’s damage/move/range. STR/DEX/WIS/CHA change those numbers when scores are set. Parent/child/sibling cannot PairBond; unrelated founders still can. Overlay off / unused sheet, idle mock hash unchanged. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Close-kin PairBond

```bash
cargo test -p sim-core --test population close_kin_pair_bond_illegal -- --exact --nocapture
cargo test -p sim-core --test population unrelated_founders_pair_bond_still_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| siblings / parent-child | PairBond not legal; execute Wait |
| unrelated adjacent + reproduction | PairBond still legal |

No new overlay. Blood links only (not household-only).

---

## 2. Sheet effects

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --lib sheet::tests::str_dex_wis_cha_mods -- --exact --nocapture
cargo test -p sim-core --test population sheet_unused_mods_zero_same_as_today -- --exact --nocapture
cargo test -p sim-core --test population str_18_vs_10_attack_damage -- --exact --nocapture
cargo test -p sim-core --test population dex_18_vs_3_move_cost -- --exact --nocapture
cargo test -p sim-core --test population wis_18_vs_3_range -- --exact --nocapture
cargo test -p sim-core --test population cha_18_influence_vote_weight -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused sheet | STR/DEX/WIS/CHA mods 0; damage 2000; move/range unchanged |
| STR 18 vs 10 | Attack damage 3000 vs 2000 |
| DEX 18 vs 3 | Move cost lower for 18 |
| WIS 18 vs 3 | vision/identity larger for 18 |
| CHA 18 + influence | weight `influence + 400`; equal votes unchanged |
| `PROTOCOL_VERSION` | **5** |

Existing `[agents.sheet]` / `--sheet`. CON health max unchanged.

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M33 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --conflict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 34; barrier 7; checkpoint 9; ci 1; combat 15; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 23; population 25; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 close-kin `--exact` ×2 | **pass** | siblings/parent-child Wait; unrelated founders legal |
| §2 sheet `--exact` ×8 | **pass** | unused constants; STR 3000 vs 2000; DEX cheaper; WIS longer range; CHA +400 influence; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
