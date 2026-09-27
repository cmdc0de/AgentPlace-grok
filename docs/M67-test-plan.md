# M67 test plan — see each new feature

Walkthrough for [`M67-plan.md`](M67-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (every-instance wear, strut/hood/sill/muffin). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `9916abd1…`. Default (time on) shipped-objects `092c3b9e…`. Overlay off: most-worn dawn identity. `--every-instance-wear`: all held/crate slots +1; `[7, 7]` consumes 2; with `--per-tick-wear` no dawn needed; `--no-time` still wears; `--load` no extra wear. Craft strut/hood/sill/muffin. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Every-instance wear

```bash
cargo test -p sim-core --test objects overlay_parses_wear -- --exact --nocapture
cargo test -p sim-core --test objects dawn_decays_most_worn_axe_only -- --exact --nocapture
cargo test -p sim-core --test objects every_instance_dawn_wears_all_held -- --exact --nocapture
cargo test -p sim-core --test objects every_instance_dawn_wears_crate -- --exact --nocapture
cargo test -p sim-core --test objects every_instance_per_tick_wears_all_held -- --exact --nocapture
cargo test -p sim-core --test objects every_instance_consumes_all_at_uses -- --exact --nocapture
cargo test -p sim-core --test objects no_time_every_instance_per_tick_still_wears -- --exact --nocapture
cargo test -p sim-core --test objects load_does_not_extra_every_instance_wear -- --exact --nocapture
cargo test -p sim-core --test objects every_instance_overlay_on_vs_off_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay parse | `[wear] every_instance = true` on; omit off |
| overlay off | two axes `[3, 0]` dawn → `[4, 0]` |
| overlay on, dawn | `[3, 0]` → `[4, 1]` |
| crate | both slots +1 |
| + `--per-tick-wear` | both held slots +1 in 1 tick |
| `[7, 7]` `uses = 8` | consume 2 |
| `--no-time` | still wears every instance |
| `--load` | no extra wear |
| overlay on vs off | hashes differ |

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_strut -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_hood -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_sill -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_muffin -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | strut / hood / sill / muffin |
| catalog-off | Catalog crafts illegal |

---

## 3. Hello / format / idle CLI

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
| shipped `--no-time` | `9916abd1…` |
| default time on | `092c3b9e…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-27, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 |
| §0 `cargo test -p viewer` | **pass** | 64/64 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 wear `--exact` ×9 | **pass** | overlay parse; most-worn identity; every-instance held/crate/per-tick/consume/no-time/load/hash |
| §2 recipes `--exact` ×5 | **pass** | Craft strut/hood/sill/muffin; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=092c3b9ec73ffa87ffa418fc52242ddc26b0040ef5c919db459f4777fed1aac2`; `--no-time` `final_hash=9916abd1ddab9c8a126d0e987bb085068cd3bce7c39be7092cae7f5c7a3f1198` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
