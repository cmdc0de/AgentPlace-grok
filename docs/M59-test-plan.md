# M59 test plan — see each new feature

Walkthrough for [`M59-plan.md`](M59-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (CraftBonus/RestBonus, per-instance wear, raft/sandals/mortar/jerky). Next slice: [`M62-plan.md`](M62-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `inventions`, `survival`, `net`); unit tests under `-p sim-core` use `--lib checkpoint::m45_blob_tests::…`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `936b6632…`. Default (time on) shipped-objects `76049695…`. Inventions off identity. Fourth Invent is CraftBonus (+15 craft); fifth RestBonus; sixth Waits. Tree blocks CraftBonus until SenseBonus shared. Two axes: 8 gathers consume one; leftover wear 0. One axe still breaks at 8. M58 `tool_uses` u32 3 → vec![3]. Craft raft/sandals/mortar/jerky. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Invention kinds

```bash
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions fourth_invent_is_craft_bonus -- --exact --nocapture
cargo test -p sim-core --test inventions sixth_invent_waits_all_kinds_present -- --exact --nocapture
cargo test -p sim-core --test inventions tree_blocks_craft_until_sense_shared -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | Invent illegal; same hash as no flag |
| CraftBonus | fourth Invent is CraftBonus; +15 craft for inventor |
| RestBonus | fifth Invent; sixth Waits |
| tree | CraftBonus blocked until SenseBonus shared |

---

## 2. Per-instance wear

```bash
cargo test -p sim-core --test objects axe_breaks_after_8_successful_gathers -- --exact --nocapture
cargo test -p sim-core --test objects two_axes_wear_separately -- --exact --nocapture
cargo test -p sim-core --test objects load_restores_tool_wear -- --exact --nocapture
cargo test -p sim-core --lib checkpoint::m45_blob_tests::m58_tool_uses_u32_becomes_vec -- --exact --nocapture
```

| Test | Success |
|---|---|
| one axe | still breaks at 8 |
| two axes | 8 uses consume 1; remaining wear 0 |
| load | vec![3] round-trips |
| M58 blob | u32 3 → vec![3] |

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_raft -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_sandals -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_mortar -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_jerky -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | raft / sandals / mortar / jerky from locked inputs |
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
| shipped `--no-time` | `936b6632…` |
| default time on | `76049695…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-14, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (objects 77; inventions 19; sleep 25; survival 18) |
| §0 `cargo test -p viewer` | **pass** | 61/61 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 inventions `--exact` ×4 | **pass** | overlay off identity; fourth Invent CraftBonus; sixth Waits; tree blocks Craft until Sense shared |
| §2 wear `--exact` ×4 | **pass** | axe breaks at 8; two axes consume 1; load restores vec![3]; M58 u32 3 → vec![3] |
| §3 recipes `--exact` ×5 | **pass** | Craft raft/sandals/mortar/jerky; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=7604969585a15ae5ae2427a6cd2a975b08133244a9acea4db45c41f659c73bb2`; `--no-time` `final_hash=936b663208c4e735d6691fcdda56511f1bd1f92b0173d4f072c1fbdcae23e6bb` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
