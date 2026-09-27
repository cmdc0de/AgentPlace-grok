# M65 test plan — see each new feature

Walkthrough for [`M65-plan.md`](M65-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (interior blocked Move, animation monikers, beam/scarf/kerb/bun). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test sleep` (or `objects`, `combat`, `net`); unit tests under `-p sim-core` use `--lib …`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `ceb0fcc0…`. Default (time on) shipped-objects `80ae9571…`. Interior 3×3: outside→non-origin Wait; door origin ok; inside/exit ok; `legal_actions` omits blocked dest; `--load` and `--no-time` still block. Cabin still enter-anywhere. Monikers: omit walk ⇒ none; death beats walk; idle still no-Move. Craft beam/scarf/kerb/bun. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Interior blocked Move

```bash
cargo test -p sim-core --test sleep cabin_still_enter_anywhere -- --exact --nocapture
cargo test -p sim-core --test sleep interior_blocks_outside_non_origin -- --exact --nocapture
cargo test -p sim-core --test sleep interior_door_origin_from_outside -- --exact --nocapture
cargo test -p sim-core --test sleep interior_inside_and_exit -- --exact --nocapture
cargo test -p sim-core --test sleep interior_legal_actions_omits_blocked -- --exact --nocapture
cargo test -p sim-core --test sleep load_restores_interior_block -- --exact --nocapture
cargo test -p sim-core --test sleep no_time_interior_still_blocks -- --exact --nocapture
```

| Test | Success |
|---|---|
| cabin | enter non-origin from outside |
| outside → non-origin | Wait |
| outside → origin | Move ok |
| inside / exit | ok |
| `legal_actions` | blocked dest omitted |
| `--load` | still blocks |
| `--no-time` | still blocks |

---

## 2. Animation monikers

```bash
cargo test -p sim-core --test combat animation_moniker_priority_and_omit -- --exact --nocapture
cargo test -p sim-core --test combat agent_idle_this_tick_true_without_move -- --exact --nocapture
cargo test -p viewer models::tests::agent_glb_has_idle_clip -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit walk | `clip_name_for` none |
| death vs walk | Death wins |
| idle | no Move still idle |
| shipped agent.toml | idle mapped; walk omitted |

Native window: **not run**.

---

## 3. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_beam -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_scarf -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_kerb -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bun -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | beam / scarf / kerb / bun from locked inputs |
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
| shipped `--no-time` | `ceb0fcc0…` |
| default time on | `80ae9571…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-27, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 |
| §0 `cargo test -p viewer` | **pass** | 63/63 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 interior `--exact` ×7 | **pass** | cabin enter-anywhere; block/door/inside/legal/load/no-time |
| §2 monikers `--exact` ×3 | **pass** | omit walk; death > walk; idle identity |
| §3 recipes `--exact` ×5 | **pass** | Craft beam/scarf/kerb/bun; catalog-off illegal |
| §4 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=80ae95712ae46eb767c148f8d8cef26e3b5159a168b8e30e7e13a54dca1608a4`; `--no-time` `final_hash=ceb0fcc0ee3636e65a56caf6231d53724abd61f3361f90ba692b47bc24046802` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
