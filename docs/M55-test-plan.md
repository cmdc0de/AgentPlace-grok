# M55 test plan — see each new feature

Walkthrough for [`M55-plan.md`](M55-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (ranged DEX to-hit, hoe/net bonuses, rope/needle/bucket/shield). Next slice: [`M59-plan.md`](M59-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `combat`, `survival`, `net`); unit tests under `-p sim-core` use the module path. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `5028d7ed…`. Default (time on) shipped-objects `32fc6324…`. Unused DEX + bow keeps M41 STR to-hit. Attacker DEX 18 + bow vs defender DEX 18 can miss; club still uses STR. Defender DEX 0 + bow always hits. Hoe Farm bonus 25; net Fish 25; bare Fish −15; rod+net 25 not 50. Craft rope/needle/bucket/shield. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Ranged DEX to-hit

```bash
cargo test -p sim-core --test combat unused_dex_bow_same_hit_as_str -- --exact --nocapture
cargo test -p sim-core --test combat dex_18_bow_can_miss_club_uses_str -- --exact --nocapture
cargo test -p sim-core --test combat defender_dex_0_bow_always_hits -- --exact --nocapture
cargo test -p sim-core --test population dex_18_can_miss_dex_0_always_hit -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused DEX + bow | same hit/miss as STR melee |
| DEX 18 bow vs DEX 18 | can differ from club STR formula |
| defender DEX 0 + bow | always hit |
| melee DEX 18 | M41 still misses (unchanged) |

Researcher (imgui **not run**):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --conflict --sheet --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

---

## 2. Hoe Farm / net Fish

```bash
cargo test -p sim-core --test objects catalog_off_farm_fish_identity -- --exact --nocapture
cargo test -p sim-core --test objects hoe_farm_bonus_25_vs_zero -- --exact --nocapture
cargo test -p sim-core --test objects net_fish_bonus_25_bare_minus_15_rod_net_max -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | Farm 0; Fish bare −15; rod +25 |
| hoe | Farm bonus 25 vs 0 without |
| net | Fish 25; rod+net 25 not 50 |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_rope -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_needle -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bucket -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_shield -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | rope / needle / bucket / shield from locked inputs |
| catalog-off | Catalog crafts illegal |

---

## 4. Hello / format / idle CLI

```bash
cargo test -p sim-core --test objects no_time_shipped_objects_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_time_on_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

| Test | Success |
|---|---|
| no-objects `--no-time` | `70e5204d…` |
| no-objects time on | `9c3b270d…` (DEX code does not move it) |
| shipped `--no-time` | `5028d7ed…` |
| default time on | `32fc6324…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 51; combat 26) |
| §0 `cargo test -p viewer` | **pass** | 58/58 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 DEX `--exact` ×4 | **pass** | unused DEX+bow identity; DEX 18 bow vs club STR; defender 0 always hit; M41 melee still misses |
| §2 hoe/net `--exact` ×3 | **pass** | catalog-off identity; hoe 25; net 25 / rod+net max |
| §3 recipes `--exact` ×5 | **pass** | rope/needle/bucket/shield Craft; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=32fc63242204f3fd25de981dc672cda08730e32bd8e9078c1fa19745b4549611`; `--no-time` `final_hash=5028d7ed5dd201cda87cfb5ee668e534490951b04373b851c89ccac6301bebec` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
