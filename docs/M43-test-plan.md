# M43 test plan — see each new feature

Walkthrough for [`M43-plan.md`](M43-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (WIS toxin detect, CHA speech range/weight, DEX flee). Next slice: [`M58-plan.md`](M58-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test population` (or `objects`, `net`); unit tests under `--lib` need the module path (`sheet::tests::…`, `protocol::tests::…`).

**Success for the slice:** unused WIS/CHA/DEX keep today’s toxins-from-memory, speech cells/importance 50 / SPEAK affinity 50, and one-step flee. WIS 18 lists visible toxic vegetation in `obs.toxins` and drops Eat/Gather without writing `ToxinFact`; WIS 0 / 10 do not. CHA 18 vs 3: heard farther; importance 70 vs 35; SPEAK affinity 90 vs 20. DEX 18 flees 3 cells for one Move of energy; DEX 0 flees 1 cell. Idle mock 2 ticks stays `cd1e0853…`. Shipping `default.toml` / `coop.toml` unchanged. `format_version = 2`, **`PROTOCOL_VERSION = 5`**.

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

## 1. WIS toxin detect

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --lib sheet::tests::str_dex_wis_cha_mods -- --exact --nocapture
cargo test -p sim-core --test population unused_sheet_wis_cha_dex_identity -- --exact --nocapture
cargo test -p sim-core --test population wis_18_vs_0_detects_visible_toxin -- --exact --nocapture
cargo test -p sim-core --test population wis_10_toxin_same_as_unused -- --exact --nocapture
cargo test -p sim-core --test population sheet_overlay_off_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused / WIS 10 | `obs.toxins` memory only; Gather mushroom still legal; no `ToxinFact` |
| WIS 18 vs 0 | 18 lists mushroom, drops Gather; 0 does not; nightshade (allergenic) omitted |
| overlay off | same hash as no `--sheet` |

---

## 2. CHA speech range/weight

```bash
cargo test -p sim-core --test population cha_18_vs_3_speech_range_and_weight -- --exact --nocapture
```

| Test | Success |
|---|---|
| CHA 18 vs 3 | dist 17: 18 heard, 3 not; importance 70 vs 35; SPEAK affinity 90 vs 20 |

---

## 3. DEX flee + `--load` + Hello v5

```bash
cargo test -p sim-core --test population dex_18_vs_0_flee_steps -- --exact --nocapture
cargo test -p sim-core --test population load_restores_wis_cha_dex_not_derived -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| DEX 18 vs 0 | 18 flees 3 cells, 0 flees 1; both pay one move; one `Flee`, no `Move` |
| `--load` | WIS/CHA/DEX scores restored; detect/range/steps recomputed |
| idle 2 ticks | `final_hash=cd1e0853…` |
| Hello v5 | unchanged |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --quiet
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --sheet --quiet
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-10, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 34; population 39; combat/governance/objects green) |
| §0 `cargo test -p viewer` | **pass** | 33/33 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 WIS `--exact` ×6 | **pass** | unused/WIS 10 identity; 18 lists mushroom; overlay hash |
| §2 CHA `--exact` ×1 | **pass** | dist 17 hear; importance 70 vs 35; affinity 90 vs 20 |
| §3 DEX / load / Hello / idle hash | **pass** | 3 vs 1 cells; scores restored; Hello v5; `cd1e0853…` |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=cd1e085363099fdda8a3abeb848cf7a4182131da12690d3e8d0b0075cabeb130` |
| viewer window / imgui | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
