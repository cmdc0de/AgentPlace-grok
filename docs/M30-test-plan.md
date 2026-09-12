# M30 test plan — see each new feature

Walkthrough for [`M30-plan.md`](M30-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (combat particles / meshes). Next slice: [`M47-plan.md`](M47-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test combat`; viewer tests live in the binary crate.

**Success for the slice:** mock hashes unchanged. FX jobs match this tick’s Attack / Flee / Incapacitated / CombatDeath; empty events ⇒ no jobs. HUD lines cover incapacitate and death. Viewer meshes spawn from jobs (no GPU in CI). Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Combat FX jobs (sim-core)

```bash
cargo test -p sim-core --test combat combat_fx_jobs_empty_without_events -- --exact --nocapture
cargo test -p sim-core --test combat combat_fx_jobs_from_attack_flee_downed_death -- --exact --nocapture
cargo test -p sim-core --test combat combat_hud_includes_incapacitate_and_death -- --exact --nocapture
cargo test -p sim-core --test combat combat_role_helper_distinct -- --exact --nocapture
```

| Test | Success |
|---|---|
| no combat events | empty jobs |
| Attack / Flee / Incapacitated / CombatDeath this tick | strike A→B; flee; downed; death; later tick has only that tick’s Attack |
| HUD | incapacitate and combat_death lines; missing tick is None |
| tints (M28) | attacker vs defender roles still distinct |

Jobs are not hashed and not checkpointed.

---

## 2. Viewer meshes (read)

Meshes spawn/despawn from those jobs: strike rod between A and B, flee flash, downed slab, death spike. M28 capsule tints stay. Fog hides FX like satchels. CombatDeath uses last known cell if the agent is already gone.

```bash
cargo test -p viewer
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| viewer crate | command/net helpers; no imgui window |
| `PROTOCOL_VERSION` | **5** |

---

## 3. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M29 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --conflict --conflict-death --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 29; barrier 7; checkpoint 9; ci 1; combat 15; compare 2; determinism 11; governance 44; incentives 19; memory 2; pipeline 18; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 jobs `--exact` ×4 | **pass** | empty jobs; strike/flee/downed/death; HUD incapacitate+death; M28 tints |
| §2 viewer + PROTOCOL `--exact` | **pass** | 23/23; PROTOCOL 5 |
| §3 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--conflict-death` overnight | **not run** | not asked |
