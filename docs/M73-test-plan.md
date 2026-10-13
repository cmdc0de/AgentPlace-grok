# M73 test plan — see each new feature

Walkthrough for [`M73-plan.md`](M73-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (arrow ammo, door side, wall cells, purlin/gaiter/cornice/bagel). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test combat` (or `sleep`, `objects`, `net`). The viewer stem test is `interior_wall_tests::interior_wall_stem_is_fence`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `cd81a0bf…`. Default (time on) shipped-objects `232b02e6…`. Bow/sling ranged Attack consumes 1 arrow; stone does not count; dist 1 does not consume; spear needs no arrow; miss still consumes; `--load` does not extra-consume. Door omit stays origin-only. `door = "n"` opens the north edge and blocks the origin. `door = "e"` opens the east edge. Inside and exit stay legal. `--no-time` still blocks. `wall_cells` for a 3×3 north door is 5 cells (perimeter minus the north edge). Unknown `door` hashes like omit. Craft arrow/purlin/gaiter/cornice/bagel. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` and tent/cabin/house unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Arrow ammo

```bash
cargo test -p sim-core --test combat melee_bow_does_not_consume_arrow -- --exact --nocapture
cargo test -p sim-core --test combat bow_ranged_consumes_one_arrow -- --exact --nocapture
cargo test -p sim-core --test combat bow_ranged_stone_does_not_count -- --exact --nocapture
cargo test -p sim-core --test combat sling_ranged_consumes_one_arrow -- --exact --nocapture
cargo test -p sim-core --test combat spear_ranged_needs_no_arrow -- --exact --nocapture
cargo test -p sim-core --test combat bow_miss_still_consumes_arrow -- --exact --nocapture
cargo test -p sim-core --test combat load_does_not_extra_consume_ammo -- --exact --nocapture
cargo test -p sim-core --test combat catalog_off_unarmed_attack_no_ammo -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_arrow -- --exact --nocapture
```

| Test | Success |
|---|---|
| melee + arrow | arrow qty unchanged; Attack proceeds |
| bow dist 2, 1 arrow | arrow 1→0 |
| bow dist 2, stone only | Wait; energy unchanged; stone stays |
| sling dist 2, 1 arrow | arrow 1→0 |
| spear dist 2, no arrow | Attack legal |
| miss | arrow still 1→0 |
| `--load` | arrow stays 0 |
| catalog-off | unarmed Attack, no ammo |
| Craft arrow | wood×1 |

---

## 2. Door side and wall cells

```bash
cargo test -p sim-core --test sleep door_omit_still_origin_only -- --exact --nocapture
cargo test -p sim-core --test sleep door_north_edge_not_origin -- --exact --nocapture
cargo test -p sim-core --test sleep door_east_edge_from_outside -- --exact --nocapture
cargo test -p sim-core --test sleep door_north_inside_and_exit -- --exact --nocapture
cargo test -p sim-core --test sleep load_keeps_door_rule_without_replace -- --exact --nocapture
cargo test -p sim-core --test sleep no_time_door_north_still_blocks_origin -- --exact --nocapture
cargo test -p sim-core --test sleep wall_cells_north_drops_north_edge -- --exact --nocapture
cargo test -p sim-core --test sleep wall_cells_interior_false_is_empty -- --exact --nocapture
cargo test -p sim-core --test sleep door_north_hash_differs_from_omit -- --exact --nocapture
cargo test -p sim-core --test sleep door_unknown_hashes_like_omit -- --exact --nocapture
cargo test -p sim-core --test sleep interior_blocks_outside_non_origin -- --exact --nocapture
cargo test -p sim-core --test sleep interior_door_origin_from_outside -- --exact --nocapture
cargo test -p viewer interior_wall_tests::interior_wall_stem_is_fence -- --exact --nocapture
```

| Test | Success |
|---|---|
| door omit | outside → non-origin Wait; outside → origin ok |
| `door = "n"` | north edge ok; origin Wait |
| `door = "e"` | east edge ok |
| inside / exit | legal |
| `--load` | one place; north door still works |
| `--no-time` | origin still blocked |
| `wall_cells` 3×3 `n` | 5 cells, no north edge, no center |
| interior false | no wall cells |
| omit vs `n` | hashes differ |
| unknown door | same hash as omit |
| M65 omit tests | still pass |
| viewer stem | `fence` |

Researcher (not run; needs a window):

```bash
cargo run -p viewer -- --config configs/default.toml
```

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_purlin -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_gaiter -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cornice -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bagel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | purlin / gaiter / cornice / bagel |
| catalog-off | Catalog crafts illegal |

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
| shipped `--no-time` | `cd81a0bf…` |
| default time on | `232b02e6…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window **not** opened.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 171/171; combat 39/39; sleep 49/49 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 arrow `--exact` ×9 | **pass** | melee keeps arrow; bow/sling consume 1; stone does not count; spear needs none; miss consumes; `--load` stays 0; catalog-off; Craft arrow |
| §2 door / walls `--exact` ×13 | **pass** | omit origin-only; north edge; east edge; inside/exit; `--load`; `--no-time`; 5 wall cells; interior false empty; hash differs; unknown matches omit; M65 tests; stem `fence` |
| §3 recipes `--exact` ×5 | **pass** | Craft purlin/gaiter/cornice/bagel; catalog-off |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=232b02e6b8d39361fe3c2045811eeac589d8122581034a7a5b412e0c3f9e2d18`; `--no-time` `final_hash=cd81a0bf17f5274381130b79d081a5ee1617ebdfbe9faa45288d76b412554168` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
