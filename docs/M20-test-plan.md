# M20 test plan — see each new feature

Walkthrough for [`M20-plan.md`](M20-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (relationship leave/end, stacked worn packs, attach loopback). Next slice: [`M27-plan.md`](M27-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test incentives` or `--test storage`; library unit tests need `--lib path::tests::…`.

**Success for the slice:** leave undoes `relationship_delta` (goals stay); incentive end undoes influence **and** relationship_delta. Omit `[storage] max_worn_*` is 1,1 (Basket 8/25, Backpack 12/40, both 20/65). `max_worn_baskets = 2` packs 16; `max_worn_backpacks = 0` is cargo only; negative is a load error. Dropping a worn carrier so caps shrink below fill refuses Transfer/Store. Attach loopback tests stay green. Shipping `default.toml` unchanged. `format_version = 2`, `PROTOCOL_VERSION = 2`.

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

## 1. Revert `relationship_delta` on leave and end

```bash
cargo test -p sim-core --test incentives supporters_of_join_leave_reverts_relationship_delta -- --exact --nocapture
cargo test -p sim-core --test incentives incentive_end_reverts_influence_and_relationship_delta -- --exact --nocapture
cargo test -p sim-core --test incentives supporters_of_join_gets_oneshots_leave_reverts_influence -- --exact --nocapture
```

| Test | Success |
|---|---|
| Join `relationship_delta` respect toward 0, then leave | joiner respect back (original milli subtracted after decay); influence already reverted; **goal remains** |
| Incentive **end** with remaining members | influence **and** relationship_delta undone; **goal remains** |
| M17 leave influence | still reverts influence only for that schedule (no relationship effect) |

---

## 2. Stacked worn packs + overlay counts

```bash
cargo test -p sim-core --test storage backpack_holds_twelve_basket_holds_eight -- --exact --nocapture
cargo test -p sim-core --test storage omit_max_worn_both_packs_twenty_slots -- --exact --nocapture
cargo test -p sim-core --test storage two_worn_baskets_hold_sixteen -- --exact --nocapture
cargo test -p sim-core --test storage max_worn_backpacks_zero_is_cargo_only -- --exact --nocapture
cargo test -p sim-core --test storage max_worn_baskets_negative_is_load_error -- --exact --nocapture
cargo test -p sim-core --test storage last_basket_with_backpack_over_twelve_refuses -- --exact --nocapture
cargo test -p sim-core --lib haul::tests::omit_max_worn_defaults_to_one -- --exact --nocapture
```

| Test | Success |
|---|---|
| Omit max_worn, Basket only | pack caps 8 / 25 |
| Omit max_worn, Backpack only | pack caps 12 / 40 |
| Omit max_worn, both | 20 / 65 |
| `max_worn_baskets = 2`, two Baskets | 16 / 50; 16 food packs |
| `max_worn_backpacks = 0`, hold Backpack | no pack (cargo only); Pack not legal |
| `max_worn_baskets = -1` | load error; `1.5` also errors |
| last Basket while Backpack held, pack over 12 | Transfer **and** Store **refused** |

Overlay (researcher; do not put these keys in shipping `configs/default.toml`):

```toml
[storage]
max_worn_baskets = 2
max_worn_backpacks = 1
```

Viewer (not automated): `/give 0 basket 1` then `/give 0 backpack 1` shows **one** satchel mesh and **one** backpack mesh (not N copies). Fog hides with the agent.

---

## 3. Attach safety net

Loopback only. No `ControlVerb`. Remote `/give` `/set` `/scrub` still refuse.

```bash
cargo test -p sim-cli --test net -- --nocapture
cargo test -p shared -- --nocapture
```

| Test | Success |
|---|---|
| `hash_neutral_attach` | attach does not change sim hash |
| `tcp_loopback_hello_snapshot` / `ws_loopback_hello_snapshot` | Hello → Snapshot on loopback |

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M19 idle default).

---

## Execution record

**Ran:** 2026-08-28, repo root. First pass: package-level §0/§3 + default hash. Second pass: each §1/§2 `--exact` line as its own invocation. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; checkpoint 9; compare 2; determinism 11; governance 44; incentives 19; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 12/12 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 10/10 including `hash_neutral_attach`, `ws_loopback_hello_snapshot` |
| §0 `cargo test -p shared` | **pass** | 6/6 including tcp + ws loopback Hello/Snapshot |
| §1 `--exact` ×3 | **pass** | 1 filtered test each: leave respect, end influence+respect, M17 influence leave |
| §2 `--exact` ×7 | **pass** | 1 filtered test each: 8/12, 20/65, two Baskets 16, max 0 cargo, −1 load error, last Basket refuse, omit defaults |
| §3 attach (same binaries as §0 net/shared) | **pass** | covered by §0 |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| §2 viewer GUI | **not run** | no display automation |
