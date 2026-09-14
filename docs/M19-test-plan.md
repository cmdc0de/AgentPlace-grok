# M19 test plan — see each new feature

Walkthrough for [`M19-plan.md`](M19-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (SetCouncilTally, `/set respect`, Backpack, crate scale). Next slice: [`M54-plan.md`](M54-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test governance` or `--test storage`; library unit tests need `--lib path::tests::…`.

**Success for the slice:** `allow_meta_rules = false` rejects SetCouncilTally Propose; adopted `SetCouncilTally { majority }` lets 2-of-3 council Support Accept; adopted unanimous keeps 2-of-3 Open. `/set 0 respect 1 50` writes 5000 milli (hash-sensitive); `/set 0 respect 50` errors; remote refuses. Backpack pack caps 12/40; Basket stays 8/25. Crate fill-scale 1-item < full. Shipping `default.toml` unchanged. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p viewer
```

---

## 1. SetCouncilTally meta-rule

```bash
cargo test -p sim-core --test governance meta_set_council_tally_waits_when_flag_off -- --exact --nocapture
cargo test -p sim-core --test governance meta_set_council_tally_majority_two_of_three_accepts -- --exact --nocapture
cargo test -p sim-core --test governance meta_set_council_tally_unanimous_two_of_three_open -- --exact --nocapture
cargo test -p sim-core --test governance parse_set_council_tally_json -- --exact --nocapture
```

| Test | Success |
|---|---|
| Flag off | SetCouncilTally Propose → **Wait** |
| Overlay unanimous, adopt majority, 2 of 3 Support | second proposal **Accepted** |
| Overlay majority, adopt unanimous, 2 of 3 Support | second proposal **Open** |
| JSON `tally=majority` | parses; `tally=nope` / missing → Wait |

---

## 2. `/set respect`

```bash
cargo test -p viewer commands::tests::parse_set_respect -- --exact --nocapture
cargo test -p viewer commands::tests::parse_set -- --exact --nocapture
```

| Test | Success |
|---|---|
| `/set 0 respect 1 50` | parse; apply → respect 5000; hash changes |
| `/set 0 respect 50` | console error (toward required) |
| remote `/set … respect` | refused |

---

## 3. Backpack + crate scale-by-fill

```bash
cargo test -p sim-core --test storage backpack_holds_twelve_basket_holds_eight -- --exact --nocapture
cargo test -p sim-core --test storage pack_then_move_cheaper_than_loose -- --exact --nocapture
cargo test -p viewer commands::tests::crate_fill_scale_one_item_smaller_than_full -- --exact --nocapture
```

| Test | Success |
|---|---|
| Backpack | pack caps 12 / 40; 12 food packs |
| Basket only | still 8 / 25 (M11) |
| Basket pack then Move | cheaper than same weight loose |
| crate 1 item vs full | scale **smaller** vs **1.0** |

Viewer (not automated): `/give 0 backpack 1` satchel looks larger/darker; crate mesh grows as the stockpile fills.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M18 idle default).

---

## 5. Docs / help

```bash
cargo test -p viewer commands::tests::parse_help -- --exact --nocapture
```

Skim: this file, [`M19-plan.md`](M19-plan.md), README current slice **M19**. `/help` lists `/set ID respect TOWARD N`.

---

## Execution record

**Ran:** 2026-08-28, repo root. Each `--exact` line invoked separately. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 25; checkpoint 9; compare 2; determinism 11; governance 44; incentives 17; replay 2; social 15; storage 20; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 12/12 |
| §1 SetCouncilTally (`--exact` ×4) | **pass** | Wait / majority Accept / unanimous Open / JSON |
| §2 `/set respect` (`--exact` ×2) | **pass** | 5000 milli; missing toward errors; remote refused |
| §3 Backpack + scale (`--exact` ×3) | **pass** | 12 vs 8; pack Move cheaper; fill-scale 1-item < full |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| §5 `parse_help` | **pass** | `/set ID respect TOWARD N` in help text |
| §3 viewer GUI | **not run** | no display automation |
