# M53 test plan — see each new feature

Walkthrough for [`M53-plan.md`](M53-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (Place tent/cabin/house, night Hunt/Farm, CON dawn). Next slice: [`M62-plan.md`](M62-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test sleep` (or `objects`, `clock`, `net`); unit tests under `-p sim-core` use `clock::tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `6e8b124a…`. Default (time on) shipped-objects `3a294816…`. Night + time on ⇒ Hunt/Farm illegal; `--no-time` Hunt legal. Place tent 1×1, cabin 2×2, house 4×4; overlap fails. Dawn on tent extra vs open-air is `max*100/1000`. Cabin non-origin cell still gets bonus. `--load` restores origins. CON 18 extra vs unused matches `dawn_energy` +1000 milli (same start). Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Night Hunt / Farm gating

```bash
cargo test -p sim-core --test sleep night_hunt_farm_illegal_when_time_on -- --exact --nocapture
cargo test -p sim-core --test sleep no_time_hunt_still_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| time on, tod night | Hunt and Farm not legal; Gather still is |
| `--no-time` | Hunt and Farm legal (animal + Food(1) seed) |

---

## 2. Place + N×N sleep + dawn shelter

```bash
cargo test -p sim-core --test sleep catalog_off_place_illegal -- --exact --nocapture
cargo test -p sim-core --test sleep catalog_on_craft_cabin_house -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cabin -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_house -- --exact --nocapture
cargo test -p sim-core --test sleep place_tent_occupies_one_cell -- --exact --nocapture
cargo test -p sim-core --test sleep place_cabin_is_2x2_no_overlap -- --exact --nocapture
cargo test -p sim-core --test sleep place_cabin_fails_on_water_or_oob -- --exact --nocapture
cargo test -p sim-core --test sleep place_house_is_4x4 -- --exact --nocapture
cargo test -p sim-core --test sleep dawn_on_tent_adds_shelter -- --exact --nocapture
cargo test -p sim-core --test sleep dawn_cabin_non_origin_gets_bonus -- --exact --nocapture
cargo test -p sim-core --test sleep dawn_off_footprint_is_tiredness_only -- --exact --nocapture
cargo test -p sim-core --test sleep load_restores_sleep_place -- --exact --nocapture
cargo test -p sim-core --lib clock::tests::dawn_refill_table -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | Place illegal |
| craft | cabin / house from locked inputs |
| tent | 1 cell; second Place fails |
| cabin | 2×2; Place inside footprint fails; water / OOB fails |
| house | 4×4 |
| dawn tent | extra = `max * 100 / 1000` vs open-air |
| cabin non-origin | still shelter |
| dawn off footprint | M52 tiredness only |
| `--load` | origin restored |

Researcher (imgui **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

---

## 3. CON dawn extra

```bash
cargo test -p sim-core --test sleep con_18_gets_1000_more_than_unused -- --exact --nocapture
cargo test -p sim-core --test sleep con_zero_matches_unused_dawn -- --exact --nocapture
```

| Test | Success |
|---|---|
| CON 18 vs unused | extra matches `dawn_energy(..., 1000) - dawn_energy(..., 0)` |
| CON 0 | same as unused |

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
| shipped `--no-time` | `6e8b124a…` |
| default time on | `3a294816…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 40; sleep 14) |
| §0 `cargo test -p viewer` | **pass** | 58/58 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 night `--exact` ×2 | **pass** | Hunt/Farm illegal at night; `--no-time` Hunt and Farm legal |
| §2 Place / dawn `--exact` | **pass** | tent 1×1; cabin 2×2 + water/OOB fail; house 4×4; dawn off-footprint identity; load restores |
| §3 CON `--exact` ×2 | **pass** | CON 18 extra vs unused; CON 0 identity |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=3a294816b006d2e1f62f8a07da79d45caa7db921e88ede000c77827ffba56103`; `--no-time` `final_hash=6e8b124a46573e18e60f946950922305d60a54b9cbdcf74ff9ebe12234e4c8d5` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
