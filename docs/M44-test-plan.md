# M44 test plan — see each new feature

Walkthrough for [`M44-plan.md`](M44-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (WIS board range, CHA Support/PairBond, STR pocket weight). Next slice: [`M49-plan.md`](M49-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test population` (or `objects`, `net`); unit tests under `--lib` need the module path (`sheet::tests::…`, `protocol::tests::…`).

**Success for the slice:** unused WIS/CHA/STR keep board = ident, SUPPORT (200, 0, 100, 0), PairBond always if legal, and no pocket weight check. WIS 18 sees an open post 4 cells past ident (author unnamed). CHA 18 vs 3 Support is trust/respect 400/200 vs 50/25 (still one Support id). CHA 0 PairBond always succeeds; CHA 18 hits more seeded d20s than CHA 3. STR 18 vs 3 pocket weight 9000 vs 7250. Idle mock 2 ticks stays `cd1e0853…`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. WIS board range

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --lib sheet::tests::str_dex_wis_cha_mods -- --exact --nocapture
cargo test -p sim-core --test population unused_sheet_board_support_pocket_identity -- --exact --nocapture
cargo test -p sim-core --test population wis_18_vs_0_board_past_ident -- --exact --nocapture
cargo test -p sim-core --test population sheet_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test governance fog_board_omits_far_author -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused / WIS 10 | board = ident; far post hidden |
| WIS 18 vs 0 | 18 sees post at dist 14 unnamed; 0 does not |
| overlay off | same hash as no `--sheet` |

---

## 2. CHA Support social + PairBond odds

```bash
cargo test -p sim-core --test population cha_18_vs_3_support_social -- --exact --nocapture
cargo test -p sim-core --test population cha_0_pair_bond_always_if_legal -- --exact --nocapture
cargo test -p sim-core --test population cha_18_vs_3_pair_bond_odds -- --exact --nocapture
cargo test -p sim-core --test population unrelated_founders_pair_bond_still_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| CHA 18 vs 3 Support | trust/respect 400/200 vs 50/25; two supporter ids (author + Support) |
| CHA 0 PairBond | always succeeds if legal |
| CHA 18 vs 3 PairBond | 18 succeeds more often; miss is Wait |

---

## 3. STR pocket weight + `--load` + Hello v5

```bash
cargo test -p sim-core --test population str_18_vs_3_pocket_weight -- --exact --nocapture
cargo test -p sim-core --test population load_restores_wis_cha_str_not_derived -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| STR 18 vs 3 | cap 9000 vs 7250; 3 rejects 25 stones, 18 accepts 30 |
| `--load` | scores restored; board/weight/odds recomputed |
| idle 2 ticks | `final_hash=cd1e0853…` |
| Hello v5 | unchanged |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --quiet
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --sheet --quiet
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-10, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 34; population 46; governance 44) |
| §0 `cargo test -p viewer` | **pass** | 33/33 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 WIS `--exact` ×6 | **pass** | unused/WIS 10 identity; dist 14 unnamed; overlay hash |
| §2 CHA `--exact` ×4 | **pass** | 400/200 vs 50/25; CHA 0 always; 18 hits more than 3 |
| §3 STR / load / Hello / idle hash | **pass** | 9000 vs 7250; scores restored; Hello v5; `cd1e0853…` |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130` |
| viewer window / imgui | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
