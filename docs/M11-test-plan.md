# M11 test plan — see each new feature

Walkthrough for [`M11-plan.md`](M11-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (events JSONL, `--compare`, satchel, `/give basket`). Next slice: [`M42-plan.md`](M42-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network. Live Ollama needs Spark.

| Host / model | Value |
|---|---|
| Ollama | `http://spark-bcce.hlab:11434` |
| Model | `nemotron3:33b` |
| Default config | `provider = "mock"` (CI). `--llm ollama` uses Spark + Nemotron, `timeout_ms = 120000` |

**Success for the slice:** CI green without Spark; 80-tick default + coop mock has **Store ≥ 1** and **stockpile qty > 0**; Pack then Move is cheaper than the same weight loose; last Basket folds pack into pockets/cell; satchel shows iff a Basket is held.

`format_version` stays **2**. `PROTOCOL_VERSION` stays **2**. No `ControlVerb::Give`. `visibility_modifier` is M12.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
cargo test -p viewer
```

Expect all green, then continue so you can see behavior, not only test names.

---

## 1. Mock Gather-then-Store (storage goal)

M10’s mock only Stored at hunger ≥ 75% with food already in pockets, so default 80-tick coop had **store=0**. M11 Gathers under the storage goal and Eats only below **50%** while that goal is on.

### Automated

```bash
cargo test -p sim-core mock_storage_goal_stores -- --exact --nocapture
cargo test -p sim-core mock_storage_goal_gathers_then_stores -- --exact --nocapture
cargo test -p sim-core default_coop_80_tick_fills_crates -- --exact --nocapture
cargo test -p sim-core same_seed_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| `mock_storage_goal_stores` | preloaded food + goal → ≥1 `Store` |
| `mock_storage_goal_gathers_then_stores` | no preload; tiny map still Stores |
| `default_coop_80_tick_fills_crates` | `configs/default.toml` + coop, 80 ticks, store ≥ 1 and crate qty > 0 |
| `same_seed_same_hash` | hashes still match |

### See it in a default-config run

```bash
rm -rf /tmp/m11-coop
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml \
  --ticks 80 --llm mock --out-dir /tmp/m11-coop --quiet
ID=$(basename /tmp/m11-coop/*_tick_80.ckpt _tick_80.ckpt)
echo "store:    $(grep -c '"type":"store"' /tmp/m11-coop/${ID}_events.jsonl || true)"
echo "gather:   $(grep -c '"type":"gather"' /tmp/m11-coop/${ID}_events.jsonl || true)"
echo "eat:      $(grep -c '"type":"eat"' /tmp/m11-coop/${ID}_events.jsonl || true)"
echo "pack:     $(grep -c '"type":"pack"' /tmp/m11-coop/${ID}_events.jsonl || true)"
grep -m5 '"type":"store"' /tmp/m11-coop/${ID}_events.jsonl || true
```

**Success (80-tick default + coop):** `store` **≥ 1** (this repo’s run: **75**), `gather` **> 0** (221), `eat` **0** (hunger still ~88, storage-goal Eat cutoff is 50%). `pack` is usually **0** here: mock does not Craft a Basket in 80 ticks. Use §3 / `/give` for the backpack.

Compare vs baseline (no schedule):

```bash
rm -rf /tmp/m11-base
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock \
  --out-dir /tmp/m11-base --quiet
cargo run -p sim-cli -- --compare /tmp/m11-base /tmp/m11-coop
```

**Success:** `state_hash` **differ**; goal occupancy `keep the shared storage stocked` 0 vs 16; **stockpile qty B > A** (0 vs 75); event table has `Store` and `Gather` on B only. Energy mean on B is a bit lower (haul + cargo Move). Diet (`consumed veg`) stays 0 on an 80-tick mock because nobody is below 50% hunger.

---

## 2. Ground crates (keep M10 rules)

One crate per **land** cell. Slot cap **16**, weight cap **80.0**.

```bash
cargo test -p sim-core two_stores_same_cell_one_crate -- --exact --nocapture
cargo test -p sim-core store_over_weight_cap_illegal -- --exact --nocapture
cargo test -p sim-core store_then_retrieve_pays_energy -- --exact --nocapture
cargo test -p sim-core marker_helper_tracks_nonempty -- --exact --nocapture
```

| Test | Success |
|---|---|
| Two Stores same cell | one crate; qty adds |
| Weight cap | stone does not Store when cap is below one stone |
| Store then Retrieve | energy drops twice; cell empty after retrieve |
| Marker helper | `has_stockpile` only while qty > 0; legend name `stockpile` |

---

## 3. Basket backpack, Pack/Unpack, Move haul

Pack caps: **8 slots, weight 25.0**. Pack haul **0.1**; pockets **0.4**. Move: `loose × 0.4 × 0.05 + pack × 0.1 × 0.05` (display).

```bash
cargo test -p sim-core pack_then_move_cheaper_than_loose -- --exact --nocapture
cargo test -p sim-core no_basket_same_loose_load_higher_move_cost -- --exact --nocapture
cargo test -p sim-core pack_over_weight_cap_illegal -- --exact --nocapture
cargo test -p sim-core move_unaffordable_not_legal -- --exact --nocapture
cargo test -p sim-core packed_food_move_cheaper_than_loose -- --exact --nocapture
```

| Test | Success |
|---|---|
| Pack then Move | Move energy **<** same food carried loose |
| No Basket, loose load | Move cost **>** empty pockets |
| Pack over weight | stone not legal when pack cap is tiny |
| Unaffordable Move | cargo cost > energy ⇒ Move not on the legal list |
| Haul unit | packed 10 food Move < loose 10 food |

Removing the last Basket folds pack into pockets (overflow → cell crate; if that would void items, Transfer/Store of that Basket is refused):

```bash
cargo test -p sim-core removing_last_basket_folds_pack_into_pockets -- --exact --nocapture
cargo test -p sim-core last_basket_overflow_refuses_transfer -- --exact --nocapture
```

**Success:** after Transfer of the only Basket, pack is empty and food is in pockets; a stuffed pack + full pockets + full crate cannot give away the last Basket.

---

## 4. Satchel helper + inspector

```bash
cargo test -p sim-core satchel_helper_tracks_basket -- --exact --nocapture
```

**Success:** no Basket ⇒ `shows_satchel() == false`; Basket in pockets ⇒ true. Legend name `satchel`, RGB **not** the ground-crate brown.

### See it in the viewer

```bash
cargo run -p viewer -- --config configs/default.toml
```

Console (`/`):

```
/give 0 basket 1
/follow 0
```

**Success:**

1. Inspector: `pack slots 0/8  weight 0.0/25` (empty pack, Basket in inventory).
2. A **small dark satchel** on the capsule (`srgb(0.35, 0.22, 0.12)`), distinct from a cell crate.
3. Legend (`L`): line `cube  satchel`.
4. Fog (`O`) + follow: satchel hides with the agent.

Then:

```
/give 0 berry_bush 3
```

Inspector inventory shows berry. Pause a few ticks with coop loaded (or `/inject configs/incentives/coop.toml`) if you want a brown **crate** on a cell as well.

Remote `/give` over TCP/WebSocket must still **refuse** (“in-process only”).

---

## 5. Checkpoints / protocol

```bash
cargo test -p sim-core m9_style_world_without_stockpiles_field_still_hashes -- --exact --nocapture
cargo test -p sim-core format_version_is_v2 -- --exact --nocapture
cargo test -p sim-cli --test net protocol_version -- --nocapture
```

**Success:** empty pack on old layouts still hashes; `CHECKPOINT_FORMAT_VERSION = 2`; `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

---

## 6. Docs / help

```bash
cargo run -p sim-cli -- --help
cargo run -p viewer -- --help 2>/dev/null || true
```

Skim: this file, [`M11-plan.md`](M11-plan.md), [`needs-and-survival.md`](needs-and-survival.md) (Pack / Move cargo / storage-goal Eat 50%), README current slice **M11**.

---

## 7. Live Ollama smoke (optional)

Not required for M11. Same 2×3 recipe as [`M10-test-plan.md`](M10-test-plan.md) §9 if Spark is up. Mock crate-fill is the slice; live Wait-rate is unchanged.

---

## Suggested 10-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test storage` | Store/pack/fold/satchel green |
| 2 | §1 80-tick coop + grep `store` | count **≥ 1** (≈75) |
| 3 | `--compare` base vs coop | hash differ; stockpile qty 0 vs **> 0** |
| 4 | `pack_then_move_cheaper_than_loose` | packed Move < loose |
| 5 | viewer `/give 0 basket 1` | satchel on capsule; pack 0/8 |

---

## Execution record

**Ran:** 2026-08-26, repo root. Viewer GUI not driven (no imgui automation).

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | all packages including 19/19 storage |
| §0 sim-llm / viewer / sim-cli ab | **pass** | no network |
| §1 unit Store / Gather-then-Store / 80-tick default coop | **pass** | crates fill |
| §1 80-tick coop grep | **pass** | store=**75**, gather=221, eat=0, pack=0 |
| §1 `--compare` 80-tick base vs coop | **pass** | hashes **differ**; stockpile cells 0 vs **69**; qty 0 vs **75**; Store 0 vs 75 |
| §2 two-stores / caps / retrieve | **pass** | |
| §3 pack Move cheaper / last Basket fold | **pass** | |
| §4 `satchel_helper_tracks_basket` | **pass** | |
| §4 viewer satchel in GUI | **not run** | no display automation |
| §5 format_version / PROTOCOL_VERSION | **pass** (via sim-core + existing net tests) | 2 and 2 |
| §7 live 2×3 | **not run** | mock crate-fill is the slice |

### §1 80-tick A/B (default.toml, mock)

| Metric | Baseline | Coop |
|---|---|---|
| `final_tick` | 80 | 80 |
| `final_hash` | `a9846c96…` | `322c12f1…` |
| Store events | 0 | **75** |
| Gather events | 0 | 221 |
| Eat events | 0 | 0 |
| stockpile cells / qty | 0 / 0 | **69 / 75** |
| hunger mean | 88.0 | 88.0 |
| energy mean | 93.6 | 86.4 |
| goal `keep the shared storage stocked` | 0 | 16 |
