# M72 test plan — see each new feature

Walkthrough for [`M72-plan.md`](M72-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (bonus-use every-instance wear, stud/glove/dado/pikelet). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `53972084…`. Default (time on) shipped-objects `5a14ef91…`. Overlay off: one Gather on two axes `[3, 0]` → `[4, 0]`. `--every-instance-wear`: one Gather → `[4, 1]`; `[7, 7]` consumes 2; Farm/Fish/stone Gather wear both slots; `--load` no extra wear. Craft stud/glove/dado/pikelet. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Bonus-use every-instance

```bash
cargo test -p sim-core --test objects bonus_every_gather_off_most_worn_only -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_gather_wears_all_held -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_gather_consumes_all_at_uses -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_farm_wears_all_hoes -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_fish_wears_all_nets -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_stone_gather_wears_all_hammers -- --exact --nocapture
cargo test -p sim-core --test objects bonus_every_overlay_off_idle_hash_unchanged -- --exact --nocapture
cargo test -p sim-core --test objects load_does_not_extra_bonus_every_wear -- --exact --nocapture
cargo test -p sim-core --test objects two_axes_wear_separately -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off, two axes `[3, 0]`, 1 Gather | `[4, 0]` |
| overlay on, two axes `[3, 0]`, 1 Gather | `[4, 1]` |
| overlay on, `[7, 7]`, `uses = 8`, 1 Gather | consume 2 |
| two hoes, 1 Farm | `[4, 1]` |
| two nets, 1 Fish | `[4, 1]` |
| two hammers, 1 stone Gather | `[4, 1]` |
| overlay off idle | same hash as unset flag |
| `--load` | wear stays `[4, 1]` |
| `two_axes_wear_separately` | M59 most-worn identity |

Researcher (not run):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --every-instance-wear
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_stud -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_glove -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_dado -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_pikelet -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | stud / glove / dado / pikelet |
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
| shipped `--no-time` | `53972084…` |
| default time on | `5a14ef91…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window and `--every-instance-wear` demo **not** driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 166/166 |
| §0 `cargo test -p viewer` | **pass** | 64/64 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 wear `--exact` ×9 | **pass** | overlay off `[4, 0]`; overlay on `[4, 1]`; `[7, 7]` consumes 2; Farm/Fish/stone Gather both slots; overlay-off idle hash; `--load` stays `[4, 1]`; `two_axes_wear_separately` |
| §2 recipes `--exact` ×5 | **pass** | Craft stud/glove/dado/pikelet; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=5a14ef91dbc73643683d5a8bf390b46a072b8fccebe0840c42473a94b632594a`; `--no-time` `final_hash=53972084b862db814f81a7d39a7ee110b03b9c5b25b869fde704b8c250175f1a` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
