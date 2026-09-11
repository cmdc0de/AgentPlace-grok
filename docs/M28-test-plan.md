# M28 test plan — see each new feature

Walkthrough for [`M28-plan.md`](M28-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (health/incapacitation, combat viewer FX, force_reflect). Next slice: [`M45-plan.md`](M45-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test net`, `--test pipeline`, `--test combat`, `--test incentives`, or `--test checkpoint`; viewer tests live in the binary crate.

**Success for the slice:** mock hashes unchanged at default health. Attack drops health and energy; health 0 incapacitates. Viewer combat roles map to distinct tints. `force_reflect` + mock skips LLM; Custom writes a Reflection; record+replay matches. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. Health / incapacitation

```bash
cargo test -p sim-core --test combat mock_conflict_overlay_same_hash -- --exact --nocapture
cargo test -p sim-core --test combat attack_drops_health_and_energy -- --exact --nocapture
cargo test -p sim-core --test combat health_zero_incapacitates -- --exact --nocapture
cargo test -p sim-core --test combat checkpoint_round_trip_default_health -- --exact --nocapture
```

| Test | Success |
|---|---|
| conflict overlay + mock | same hash as off |
| Attack adjacent | defender health and energy down |
| health → 0 | incapacitated; event; not Attack target |
| old ckpt / round-trip | health 10_000; not incapacitated; hash preserved |

---

## 2. Combat viewer FX

```bash
cargo test -p sim-core --test combat combat_role_helper_distinct -- --exact --nocapture
```

| Test | Success |
|---|---|
| color helper | attacker vs defender roles distinct; HUD line contains `attack` |

Render-only tints on capsules; Status HUD shows `tick T attack #A → #B` when events exist. No sim hash change.

---

## 3. force_reflect

```bash
cargo test -p sim-core --test pipeline force_reflect_parses -- --exact --nocapture
cargo test -p sim-core --test pipeline force_reflect_mock_skips_llm -- --exact --nocapture
cargo test -p sim-core --test pipeline force_reflect_custom_writes_memory -- --exact --nocapture
cargo test -p sim-core --test pipeline force_reflect_record_replay_same_hash -- --exact --nocapture
cargo test -p sim-core --test incentives unknown_effect_type_errors -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version_constant -- --exact --nocapture
```

| Test | Success |
|---|---|
| parse | `type = "force_reflect"` loads |
| mock | same hash as incentive without the effect; no Reflection |
| Custom | Reflection this tick |
| record + replay | hashes match; no live insight on replay |
| unknown type | still load error |
| `PROTOCOL_VERSION` | **5** |

```toml
[[incentives.effects]]
type = "force_reflect"
```

Example: `configs/incentives/force-reflect.toml`. Do not change `coop.toml`.

---

## 4. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14–M27 idle default).

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --incentives configs/incentives/force-reflect.toml \
  --llm ollama --llm-reflect-every 10 --conflict --quiet
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
```

Live Ollama / overnight: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-03, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 26; barrier 7; checkpoint 9; combat 9; compare 2; determinism 11; governance 44; incentives 19; pipeline 18; reflect 6; replay 2; social 15; storage 25; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 23/23 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 42/42 |
| §0 `cargo test -p shared` | **pass** | 6/6 |
| §1 health `--exact` ×4 | **pass** | mock same hash; health+energy drop; incapacitate; ckpt default health |
| §2 viewer helper `--exact` ×1 | **pass** | attacker vs defender roles distinct |
| §3 force_reflect `--exact` ×6 | **pass** | parse; mock skip; Custom Reflection; record=replay; unknown type; PROTOCOL 5 |
| §4 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| viewer GUI | **not run** | no display automation |
| live Spark / `--conflict` overnight | **not run** | not asked |
