# M69 test plan — see each new feature

Walkthrough for [`M69-plan.md`](M69-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (ranged ammo consume, joist/veil/ashlar/loaf). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `combat`, `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `1910c33a…`. Default (time on) shipped-objects `98abaa85…`. Melee dist 1 with bow does not consume stone. Bow/sling dist 2 consume 1 stone; no stone ⇒ illegal/Wait. Spear dist 2 needs no stone. Miss still consumes. `--load` no extra consume. Catalog-off unarmed Attack dist 1. Craft joist/veil/ashlar/loaf. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Ranged ammo

```bash
cargo test -p sim-core --test combat melee_bow_does_not_consume_stone -- --exact --nocapture
cargo test -p sim-core --test combat bow_ranged_consumes_one_stone -- --exact --nocapture
cargo test -p sim-core --test combat bow_ranged_no_stone_is_wait -- --exact --nocapture
cargo test -p sim-core --test combat sling_ranged_consumes_one_stone -- --exact --nocapture
cargo test -p sim-core --test combat spear_ranged_needs_no_stone -- --exact --nocapture
cargo test -p sim-core --test combat bow_miss_still_consumes_stone -- --exact --nocapture
cargo test -p sim-core --test combat load_does_not_extra_consume_ammo -- --exact --nocapture
cargo test -p sim-core --test combat catalog_off_unarmed_attack_no_ammo -- --exact --nocapture
cargo test -p sim-core --test combat bow_attack_legal_at_chebyshev_3_not_4 -- --exact --nocapture
```

| Test | Success |
|---|---|
| melee + bow + stone | stone unchanged |
| bow dist 2, 1 stone | stone 1→0; Attack |
| bow dist 2, 0 stone | illegal / Wait; energy unchanged |
| sling dist 2 | stone 1→0 |
| spear dist 2 | legal without stone |
| miss | still consumes |
| `--load` | no extra consume |
| catalog-off | unarmed dist 1 |

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_joist -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_veil -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_ashlar -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_loaf -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | joist / veil / ashlar / loaf |
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
| shipped `--no-time` | `1910c33a…` |
| default time on | `98abaa85…` |
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
| §1 ammo `--exact` ×9 | **pass** | melee no consume; bow/sling consume; no stone Wait; spear ok; miss consumes; load; catalog-off |
| §2 recipes `--exact` ×5 | **pass** | Craft joist/veil/ashlar/loaf; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=98abaa858766e2399daad9053ac3e0d34906d8ceee8f2a8827ad169c0c7056ea`; `--no-time` `final_hash=1910c33abbb9b4d5f4a3d9206c250164b371d2a49d53ef4a8946abe3ba5471b3` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
