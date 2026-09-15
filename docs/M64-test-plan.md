# M64 test plan — see each new feature

Walkthrough for [`M64-plan.md`](M64-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (non-square footprints, ranged projectile FX, agent idle clip). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test sleep` (or `combat`, `objects`, `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `1b9117a9…`. Default (time on) shipped-objects `f583c391…` (M63 identity; square omit-hash; FX/idle not hashed). Place 2×3 fixture occupies 6 cells; cabin still 2×2; OOB/water/overlap illegal; dawn any cell; Pickup any cell; `--load` restores origin. Dist 1 Attack → Strike; dist 2 → Projectile. Agent glb has idle clip `ArmatureAction.002`; Move this tick pauses idle. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Non-square footprints

```bash
cargo test -p sim-core --test sleep cabin_stays_square_2x2 -- --exact --nocapture
cargo test -p sim-core --test sleep place_2x3_fixture_occupies_six_cells -- --exact --nocapture
cargo test -p sim-core --test sleep place_2x3_oob_or_water_illegal -- --exact --nocapture
cargo test -p sim-core --test sleep place_2x3_overlap_illegal -- --exact --nocapture
cargo test -p sim-core --test sleep dawn_any_cell_of_2x3 -- --exact --nocapture
cargo test -p sim-core --test sleep pickup_2x3_from_non_origin -- --exact --nocapture
cargo test -p sim-core --test sleep load_restores_2x3 -- --exact --nocapture
```

| Test | Success |
|---|---|
| cabin | still 2×2 |
| Place 2×3 | 6 land cells; origin min-(x,y) |
| OOB / water / overlap | Place illegal |
| dawn | bonus on non-origin cell |
| Pickup | any cell; cells free |
| `--load` | origin restored; no second Place |

---

## 2. Ranged projectile FX

```bash
cargo test -p sim-core --test combat combat_fx_jobs_dist_1_is_strike -- --exact --nocapture
cargo test -p sim-core --test combat combat_fx_jobs_dist_2_is_projectile -- --exact --nocapture
cargo test -p sim-core --test combat combat_fx_jobs_empty_without_events -- --exact --nocapture
```

| Test | Success |
|---|---|
| dist 1 | `Strike` |
| dist 2 | `Projectile`, not Strike |
| idle mock | no Attack ⇒ no jobs |

---

## 3. Agent idle clip

```bash
cargo test -p sim-core --test combat agent_idle_this_tick_true_without_move -- --exact --nocapture
cargo test -p sim-core --test combat agent_idle_this_tick_false_on_move -- --exact --nocapture
cargo test -p viewer models::tests::agent_glb_has_idle_clip -- --exact --nocapture
cargo test -p viewer models::tests::no_clip_visual_is_still_static_kind -- --exact --nocapture
```

| Test | Success |
|---|---|
| no Move | idle |
| Move this tick | not idle (pause) |
| shipped glb | ≥1 clip; idle maps to `ArmatureAction.002` |
| sentinel / missing | static kind, no panic |

Native window playing the clip: **not run** (no display).

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
| shipped `--no-time` | `1b9117a9…` |
| default time on | `f583c391…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-15, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 |
| §0 `cargo test -p viewer` | **pass** | 63/63 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 footprints `--exact` ×7 | **pass** | cabin 2×2; 2×3 six cells; OOB/water/overlap illegal; dawn extra; Pickup; load |
| §2 projectile `--exact` ×3 | **pass** | dist 1 Strike; dist 2 Projectile; empty jobs |
| §3 idle clip `--exact` ×4 | **pass** | idle without Move; pause on Move; glb clip + mapping; sentinel static |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=f583c391c19f90f944c56f87fcd4d14c08e3e1b66abfe439847b869dc976a4f9`; `--no-time` `final_hash=1b9117a96d1f454f24ec022f331df1840fe3fe166aaf413dc827e0d28bc58ab3` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
