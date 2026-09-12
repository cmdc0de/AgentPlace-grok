# M41 test plan — see each new feature

Walkthrough for [`M41-plan.md`](M41-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (species `[sim]` from objects TOML, DEX melee miss, STR haul caps). Next slice: [`M48-plan.md`](M48-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `population`, `net`); unit tests under `--lib` need the module path (`sheet::tests::…`).

**Success for the slice:** veg/animal/fish `[sim]` in `configs/objects` overrides built-in species without shuffling tags 1–5; extras append. Idle mock 2 ticks stays `cd1e0853…`. Unused DEX always hits; DEX 18 can miss (`damage = 0`). STR changes pocket/pack caps, not crate caps, and does not write `inventory_cap`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. World species from object TOML

```bash
cargo test -p sim-core --test objects shipped_species_sim_matches_defaults -- --exact --nocapture
cargo test -p sim-core --test objects missing_species_sim_keeps_rust_nutrition -- --exact --nocapture
cargo test -p sim-core --test objects extra_vegetation_appends_tag_and_changes_hash -- --exact --nocapture
cargo test -p sim-core --test objects override_nutrition_changes_hash_after_eat -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| shipped `[sim]` | matches `default_species_tables()` |
| missing `[sim]` | berry_bush visual-only keeps nutrition 20 |
| extra vegetation | tag 6 appended; tags 1–5 unchanged; hash ≠ base |
| override nutrition | Gather+Eat hash ≠ shipped |
| idle 2 ticks | `cd1e0853…` |

```bash
cargo run -p sim-cli -- --config configs/default.toml --objects configs/objects \
  --ticks 80 --llm mock --quiet
```

---

## 2. DEX accuracy + STR haul

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --test population sheet_unused_attack_always_hits_pocket_cap_16 -- --exact --nocapture
cargo test -p sim-core --test population dex_18_can_miss_dex_0_always_hit -- --exact --nocapture
cargo test -p sim-core --test population str_18_vs_3_pocket_and_pack_caps_crate_unchanged -- --exact --nocapture
cargo test -p sim-core --test population load_restores_sheet_not_derived_cap -- --exact --nocapture
cargo test -p sim-core --test population sheet_overlay_off_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused sheet | Attack always hits; pocket cap 16 |
| DEX 18 vs 0 | DEX 18 can miss (`damage = 0`); DEX 0 hits |
| STR 18 vs 3 | pocket 20 vs 13; pack caps differ; crate cap same |
| `--load` | scores restored; `inventory_cap` still 16; no extra items |
| overlay off | same hash as no `--sheet` |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --conflict --quiet
```

---

## 3. Default mock hash + Hello v5

```bash
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=cd1e0853…`. Hello postcard v5.

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
cargo run -p viewer -- --config configs/default.toml --objects configs/objects
```

Live Ollama / overnight / imgui window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-07, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | objects 20; population 29; lib + other integration green |
| §0 `cargo test -p viewer` | **pass** | 32/32 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 45/45 |
| §0 `cargo test -p shared` | **pass** | 14/14 |
| §1 species `--exact` ×5 | **pass** | shipped match; extra tag 6; idle `cd1e0853…` |
| §2 DEX/STR `--exact` ×6 | **pass** | miss `damage=0`; STR caps; load no derived write |
| §3 default 2 ticks + Hello v5 | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130`; postcard v5 |
| §1 objects 80 ticks | **pass** | `final_tick=80` `final_hash=006f3624c28a9516d29fc9b40e7977744d755672af9daeb76912ffa780377469` |
| viewer GUI / listen attach | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
