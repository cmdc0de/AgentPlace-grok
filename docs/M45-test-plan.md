# M45 test plan — see each new feature

Walkthrough for [`M45-plan.md`](M45-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (tech tree/patents, hashed pipeline events, format_version 3 catalog slugs). CLI flags: [`cli-reference.md`](cli-reference.md). Config files: [`config-reference.md`](config-reference.md). Next slice: [`M48-plan.md`](M48-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test inventions` (or `objects`, `net`); unit tests under `--lib` need the module path (`protocol::tests::…`).

**Success for the slice:** inventions `tree=false` / `patent_ticks=0` keep today’s invent order and share timing. `tree=true` blocks MoveBonus until GatherBonus is society-shared. `patent_ticks=4` delays society share. Pipeline overlay off ⇒ no `Pipeline` events, same hash. Pipeline on ⇒ one event per living agent per tick with `stages=31`. Checkpoints **write format_version 3**; v2 still loads. Catalog slugs survive an extra catalog file. Idle mock 2 ticks stays `cd1e0853…`. Shipping `default.toml` / `coop.toml` unchanged. **`PROTOCOL_VERSION = 5`**.

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

## 1. Tech tree + patents

```bash
cargo test -p sim-core --test inventions overlay_parses_inventions -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions mock_inventions_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions tree_blocks_move_until_gather_shared -- --exact --nocapture
cargo test -p sim-core --test inventions patent_ticks_delay_society_share -- --exact --nocapture
cargo test -p sim-core --test inventions execute_invent_inventor_then_society -- --exact --nocapture
```

| Test | Success |
|---|---|
| parse | tree/patent default false/0 |
| overlay off | Invent illegal; same hash |
| tree on | MoveBonus illegal until GatherBonus shared |
| patent_ticks=4 | society not entitled until invent+delay+4 |

---

## 2. Hashed pipeline events

```bash
cargo test -p sim-core --test inventions pipeline_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions pipeline_on_emits_complete_and_changes_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| overlay off | no Pipeline events; same hash as no flag |
| overlay on | 2 ticks × 2 agents = 4 events; stages=31; hash ≠ off |

---

## 3. format_version 3 + catalog slug + Hello v5

```bash
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test objects load_v2_catalog_u16_still_decodes -- --exact --nocapture
cargo test -p sim-core --test objects load_v3_catalog_slug_survives_extra_file -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| write ckpt | `format_version = 3` |
| load v2 | builtins restore |
| load v3 slug | extra catalog file does not reshuffle zeta |
| idle 2 ticks | `final_hash=cd1e0853…` |
| Hello v5 | unchanged |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --inventions --pipeline-events --catalog --quiet
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --inventions --pipeline-events --quiet
```

## 4. Viewer objects dir / glb (regression)

Wrong cwd used to load **zero** object defs (primitives, no `loaded glb`). `--connect` must still find shipped `configs/objects` and resolve glbs.

```bash
cargo test -p viewer --bin viewer models::tests::connect_wrong_cwd_still_loads_shipped_glb -- --exact --nocapture
cargo test -p viewer --bin viewer models::tests::shipped_object_toml_resolves_glb -- --exact --nocapture
cargo test -p sim-core --test objects default_objects_dir_finds_shipping_from_other_cwd -- --exact --nocapture
```

| Test | Success |
|---|---|
| wrong cwd omit / relative / missing `--objects` | non-empty defs; berry_bush/hare/crate/tree/basket glb files exist |
| shipped TOML | those ids resolve a glb |
| sim-core default dir | crate-path fallback finds `berry_bush.toml` |

## 5. Researcher CLI / config docs

Read (no extra `cargo test`):

- [`cli-reference.md`](cli-reference.md) — `sim-cli` / viewer flags, slash commands.
- [`config-reference.md`](config-reference.md) — `configs/default.toml`, `configs/incentives/*`, `configs/objects/*`.

| Check | Success |
|---|---|
| files exist | both paths under `docs/` |
| M45 flags | `--pipeline-events`, `[inventions] tree` / `patent_ticks` listed |
| shipping files | `default.toml`, `coop.toml`, each objects TOML named |

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-11, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 35 including v2 blob inventions; inventions 12; objects 23; checkpoint 9) |
| §0 `cargo test -p viewer` | **pass** | 33/33 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 tree/patent `--exact` ×6 | **pass** | parse defaults; overlay off identity; tree blocks MoveBonus; patent_ticks=4 delays share |
| §2 pipeline `--exact` ×2 | **pass** | off same hash / no events; on 4× stages=31 and hash ≠ off |
| §3 ckpt / Hello / idle hash | **pass** | write v3; load v2 builtins; v3 slug remap; Hello v5; `cd1e0853…` |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130` |
| §3 80 ticks inventions+pipeline+catalog | **pass** | `final_tick=80` (smoke; catalog-on hash uses slugs) |
| §4 viewer objects/glb regression | **pass** | `connect_wrong_cwd_still_loads_shipped_glb` |
| §5 CLI / config docs | **pass** | `docs/cli-reference.md`, `docs/config-reference.md` |
| viewer window / imgui | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
