# M54 test plan — see each new feature

Walkthrough for [`M54-plan.md`](M54-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (Pickup, auto-cabin, spear range, torch/axe/jar/bread). Next slice: [`M58-plan.md`](M58-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test sleep` (or `objects`, `combat`, `survival`, `net`); unit tests under `-p sim-core` use the module path. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `35746f95…`. Default (time on) shipped-objects `7b8864e9…`. Pickup tent returns the item and frees the cell; full inventory Wait. Pickup cabin from a non-origin cell. `--load` restores origins; Pickup after load does not double-grant. PairBond + `--household-crates` + catalog mints a free cabin when 2×2 land is free; skip if not; overlay off no cabin. Spear +500 damage catalog-on; catalog-off STR only. Bow Attack legal at Chebyshev 3, not 4; club/unarmed adjacent only. Craft torch/axe/jar/bread. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Pickup

```bash
cargo test -p sim-core --test sleep catalog_off_place_illegal -- --exact --nocapture
cargo test -p sim-core --test sleep place_tent_then_pickup -- --exact --nocapture
cargo test -p sim-core --test sleep pickup_full_pockets_and_pack_waits -- --exact --nocapture
cargo test -p sim-core --test sleep pickup_cabin_from_non_origin -- --exact --nocapture
cargo test -p sim-core --test sleep load_then_pickup_tent -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | Place and Pickup illegal; catalog Craft illegal |
| Place tent then Pickup | origin gone; inventory +1 tent; `PickedUp` |
| full pockets | Wait; origin stays |
| cabin non-origin | origin removed; 2×2 frees |
| `--load` | origin restored; Pickup after load +1 |

Researcher (imgui **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

---

## 2. Household auto-cabin

```bash
cargo test -p sim-core --test sleep household_auto_cabin_on_pair_bond -- --exact --nocapture
cargo test -p sim-core --test sleep household_auto_cabin_skips_when_cannot_place -- --exact --nocapture
cargo test -p sim-core --test sleep load_restores_auto_cabin_no_remint -- --exact --nocapture
cargo test -p sim-core --test sleep household_off_pair_bond_no_cabin -- --exact --nocapture
```

| Test | Success |
|---|---|
| household_crates + catalog + PairBond on 2×2 | cabin at home; no wood/stone spent |
| cannot place | home mints; no cabin |
| `--load` | origin + home restored; no second cabin |
| overlay off | PairBond does not insert sleep_places |

---

## 3. Spear melee + weapon range + recipes

```bash
cargo test -p sim-core --test combat attack_hold_spear_adds_500 -- --exact --nocapture
cargo test -p sim-core --test combat attack_catalog_off_spear_is_base_damage -- --exact --nocapture
cargo test -p sim-core --test combat bow_attack_legal_at_chebyshev_3_not_4 -- --exact --nocapture
cargo test -p sim-core --test combat unarmed_and_club_attack_only_adjacent -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_torch -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_axe -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_jar -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bread -- --exact --nocapture
```

| Test | Success |
|---|---|
| hold spear, catalog on | damage = STR + 500 |
| catalog-off spear | STR only |
| bow Chebyshev 3 | Attack legal; dist 4 illegal |
| unarmed / club | Attack legal only at dist 1 |
| Craft | torch / axe / jar / bread from locked inputs |

---

## 4. Hello / format / idle CLI

```bash
cargo test -p sim-core --test objects no_time_shipped_objects_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

| Test | Success |
|---|---|
| no-objects `--no-time` | `70e5204d…` |
| shipped `--no-time` | `35746f95…` |
| default time on | `7b8864e9…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 44; sleep 22; combat 23) |
| §0 `cargo test -p viewer` | **pass** | 58/58 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 Pickup `--exact` ×5 | **pass** | tent Pickup; full Wait; cabin non-origin; load then Pickup |
| §2 auto-cabin `--exact` ×4 | **pass** | mint cabin; skip when cannot place; load no remint; overlay off |
| §3 combat / recipes `--exact` | **pass** | spear +500; catalog-off STR; bow range 3; club adjacent; torch/axe/jar/bread Craft |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=7b8864e94592bc29b6680ed9ffd2eb303dd376b825ddbeceb5e7d8ef3effa0fc`; `--no-time` `final_hash=35746f95698dc1ce00985dcdc3e06160bc3ed8c24f2e45eafed6a168d661785b` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
