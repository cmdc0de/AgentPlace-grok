# M46 test plan — see each new feature

Walkthrough for [`M46-plan.md`](M46-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry). Next slice: [`M52-plan.md`](M52-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test population` (or `objects`, `telemetry`, `net`); unit tests under `--lib` need the module path (`sheet::tests::…`).

**Success for the slice:** unused CON/INT/STR keep always-sick, plan length 4, and gather qty ×1. CON 18 resists toxic-eat illness more than CON 3. INT 18 vs 3 plan length 8 vs 1. STR 18 vs 3 gather wood qty higher. Catalog-off ⇒ plank/knife not legal, same hash. Catalog-on ⇒ Craft plank from 2 wood. Telemetry off ⇒ same hash; on ⇒ aggregates recorded, hash unchanged. Default `sim-cli` loads shipped objects (now including plank/charcoal/knife/net) so 2-tick hash is `5f231378…`. Without objects dir the hash stays `70e5204d…`. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

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

## 1. Sheet leftovers

```bash
cargo test -p sim-core --lib sheet::tests::unused_sheet_keeps_constants -- --exact --nocapture
cargo test -p sim-core --lib sheet::tests::str_dex_wis_cha_mods -- --exact --nocapture
cargo test -p sim-core --test population unused_sheet_energy_memory_identity -- --exact --nocapture
cargo test -p sim-core --test population con_18_vs_3_illness_chance -- --exact --nocapture
cargo test -p sim-core --test population int_18_vs_3_plan_length -- --exact --nocapture
cargo test -p sim-core --test population str_18_vs_3_gather_qty -- --exact --nocapture
cargo test -p sim-core --test population load_restores_con_int_not_derived -- --exact --nocapture
```

| Test | Success |
|---|---|
| unused | always-sick; plan 4; qty identity |
| CON 18 vs 3 | 18 sick less often; CON 0 always |
| INT 18 vs 3 | plan length 8 vs 1 |
| STR 18 vs 3 | tree wood qty higher for 18 |
| `--load` | scores restored; duration derived |

---

## 2. Extra recipes

```bash
cargo test -p sim-core --test objects catalog_off_plank_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_plank -- --exact --nocapture
cargo test -p sim-core --test objects builtin_slugs_are_not_catalog_u16 -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | Catalog Craft illegal; same hash |
| catalog-on plank | 2 wood → 1 plank |
| slugs | plank/charcoal/knife/net are Catalog; basket still builtin |

---

## 3. Telemetry + Hello / format / idle

```bash
cargo test -p sim-core --lib timing::tests::duration_stats_known_series -- --exact --nocapture
cargo test -p sim-core --lib timing::tests::telemetry_overlay_parses -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_overlay_off_same_hash -- --exact --nocapture
cargo test -p sim-core --test telemetry telemetry_on_records_and_same_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| stats series | count 5, total 150, avg/median 30, min 10, max 50 |
| telemetry off | same hash; count 0 |
| telemetry on | 4 samples; hash = off |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=5f231378…` |

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --sheet --catalog --telemetry --quiet
```

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused \
  --sheet --catalog --telemetry --quiet
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
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 37; population 49; objects 27; telemetry 2) |
| §0 `cargo test -p viewer` | **pass** | 35/35 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 sheet `--exact` ×7 | **pass** | unused identity; CON 18 resists more; plan 8 vs 1; wood qty 18>3 |
| §2 recipes `--exact` ×4 | **pass** | plank Craft; catalog-off identity |
| §3 telemetry / Hello / idle | **pass** | stats series; on hash=off; format 3; Hello v5 |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=5f2313783257960c08253b6717514019459533da6923884a981b1ab2c8068fc2` |
| viewer window / imgui | **not run** | no display automation |
| live Spark / overnight | **not run** | not asked |
