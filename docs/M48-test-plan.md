# M48 test plan — see each new feature

Walkthrough for [`M48-plan.md`](M48-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (INT invent quality, agent meshes, hot-reload glb). Next slice: [`M53-plan.md`](M53-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test inventions` (or `objects`, `survival`, `net`); unit tests under `-p viewer` use the module path (`models::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** unused INT Invent still **+200** influence and chance **400**. INT 18 vs 3 quality **400 vs 50**; chance still **600 vs 250**. Inventions off ⇒ Invent illegal, idle hash unchanged. `--load` does not double-apply quality. Agent empty visual ⇒ Primitive; `agent.toml` ⇒ Authored `low_poly_human_character.glb`; configured missing ⇒ Sentinel; visual-only agent.toml is not hashed. `should_reload` is true only for a newer mtime. Default `sim-cli` 2-tick hash is `5f231378…` (shipped objects) / `70e5204d…` without catalog. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. INT invent quality

```bash
cargo test -p sim-core --lib inventions::tests::inventor_influence_unused_and_scores -- --exact --nocapture
cargo test -p sim-core --test inventions unused_int_invent_influence_200 -- --exact --nocapture
cargo test -p sim-core --test inventions int_18_vs_3_invent_quality -- --exact --nocapture
cargo test -p sim-core --test inventions invent_quality_load_does_not_double -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test inventions invent_chance_int_18_higher_than_unused -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused INT | influence +200; chance 400 |
| INT 18 vs 3 | quality 400 vs 50; chance 600 vs 250 |
| `--load` | influence not doubled |
| overlay off | Invent illegal; same hash |
| chance formula | 400 / 600 unchanged |

---

## 2. Agent meshes

```bash
cargo test -p viewer models::tests::agent_empty_visual_is_primitive -- --exact --nocapture
cargo test -p viewer models::tests::agent_toml_authored_human -- --exact --nocapture
cargo test -p viewer models::tests::agent_configured_missing_is_sentinel -- --exact --nocapture
cargo test -p viewer models::tests::agent_visual_not_hashed -- --exact --nocapture
```

| Test | Success |
|---|---|
| empty visual | Primitive (capsule) |
| agent.toml | Authored `low_poly_human_character.glb` |
| configured missing | Sentinel |
| visual-only | `state_hash` unchanged |

---

## 3. Hot-reload + Hello / format / idle

```bash
cargo test -p viewer models::tests::should_reload_newer_only -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| `should_reload` | newer true; equal/older false |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=5f231378…` |
| no catalog | `70e5204d…` |

Native window (read, not run here): edit a loaded `.glb` on disk; within ~0.5 s the viewer logs `glb reload {id} {path}`.

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --inventions --quiet
```

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-11, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 38; inventions 15; objects 27; population 49) |
| §0 `cargo test -p viewer` | **pass** | 51/51 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 invent quality `--exact` ×6 | **pass** | +200 unused; 400 vs 50 quality; chance 600/250; load identity |
| §2 agent `--exact` ×4 | **pass** | Primitive / Authored human / Sentinel / not hashed |
| §3 reload / Hello / idle | **pass** | newer mtime only; format 3; Hello v5; no-catalog `70e5204d…` |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=5f2313783257960c08253b6717514019459533da6923884a981b1ab2c8068fc2` |
| viewer window / imgui / glb reload | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
