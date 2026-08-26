# M10 test plan — see each new feature

Walkthrough for [`M10-plan.md`](M10-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (events JSONL, `--compare`, crate mesh, `/give`).

All commands assume the **repo root**. Mock / `cargo test` never need the network. Live Ollama needs Spark.

| Host / model | Value |
|---|---|
| Ollama | `http://spark-bcce.hlab:11434` |
| Model | `nemotron3:33b` |
| Default config | `provider = "mock"` (CI). `--llm ollama` uses Spark + Nemotron, `timeout_ms = 120000` |

M9 live 16×80 was overnight. Live here is a **2 agent × 3 tick smoke** unless you deliberately start another 80-tick A/B.

**Success for the slice:** CI green without Spark; mock Store appears under the coop storage goal; Transfer/Store pay energy and respect caps; wait-sentinel replay matches the recording hash; viewer `/give` is in-process only.

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

## 1. Thinking-model parse (`<think>` / `reasoning`)

Live Nemotron often wraps JSON in thinking text. Parse must still pick `Drink` / `Wait` / etc.

### Automated

```bash
cargo test -p sim-core parse_think_wrapped_json -- --exact --nocapture
cargo test -p sim-llm extract_think_and_reasoning_json -- --exact --nocapture
```

**Success:** both pass. The `sim-llm` test feeds empty `content` + JSON in `reasoning` and gets `Rest`.

---

## 2. Honest replay (raw text + `LlmWait` sentinel)

M9 recorded `{"primary":"Wait"}`, so replaying a timeout did **not** re-emit `LlmWait`. M10 records `{"__llm_wait__":true}` on live/wait failures.

### Automated

```bash
cargo test -p sim-core wait_sentinel_recording_matches_replay -- --exact --nocapture
cargo test -p sim-core replay_fixture_twice_same_hash -- --exact --nocapture
```

**Success:** recording hash **equals** replay hash; `LlmWait` counts match.

### See a sentinel file

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace('provider = "mock"', 'provider = "wait"', 1)
t = t.replace('replay_file = ""', 'replay_file = "/tmp/m10-wait-replay.jsonl"')
Path("/tmp/m10-wait.toml").write_text(t)
PY
rm -f /tmp/m10-wait-replay.jsonl
cargo run -p sim-cli -- --config /tmp/m10-wait.toml --ticks 2 --quiet
head -2 /tmp/m10-wait-replay.jsonl
```

**Success:** each line’s `response` contains `__llm_wait__`. A second run with the same config (file now exists) prints the same `final_hash=`.

---

## 3. Empty URL still mock

```bash
cargo test -p sim-llm empty_url_ollama_is_mock -- --exact --nocapture
cargo test -p sim-llm mock_provider_ignores_url -- --exact --nocapture
```

**Success:** empty `base_url` + `ollama` → Mock. CI never hits Spark.

---

## 4. Transfer (adjacent, identified, energy)

### Automated

```bash
cargo test -p sim-core transfer_adjacent_identified -- --exact --nocapture
cargo test -p sim-core transfer_far_waits -- --exact --nocapture
cargo test -p sim-core store_no_energy_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Adjacent + identified | food moves; sender energy drops; `Transfer` event |
| Far / unnamed | `Wait`; inventory unchanged |
| Energy 0 | `Store` not on the legal list |

---

## 5. Store / Retrieve, caps, energy

One container per **land** cell. Slot cap **16**, weight cap **80.0**. Cost `qty × unit_weight × 0.4` (display). Stone is heavy (3.0); food is 0.5.

### Automated

```bash
cargo test -p sim-core store_then_retrieve_pays_energy -- --exact --nocapture
cargo test -p sim-core store_over_weight_cap_illegal -- --exact --nocapture
cargo test -p sim-core marker_helper_tracks_nonempty -- --exact --nocapture
cargo test -p sim-core m9_style_world_without_stockpiles_field_still_hashes -- --exact --nocapture
cargo test -p sim-core four_stones_cost_about_four_point_eight_energy -- --exact --nocapture
```

| Test | Success |
|---|---|
| Store then Retrieve | energy drops **twice**; cell empty after retrieve |
| Weight cap | stone does not Store when cap is below one stone |
| Marker helper | `has_stockpile` true only while qty > 0; legend name `stockpile` |
| M9 ckpt | empty `stockpiles`; round-trip hash matches |
| Haul math | 4 stones → **480** millipoints (4.8 energy) |

---

## 6. Mock + coop storage goal actually Stores

Coop injects `keep the shared storage stocked`. Mock Stores surplus food when that goal is present.

### Automated

```bash
cargo test -p sim-core mock_storage_goal_stores -- --exact --nocapture
cargo test -p sim-core same_seed_same_hash -- --exact --nocapture
```

**Success:** at least one `Store` event on the tiny map; same seed still matches (policy is deterministic).

### See it in a default-config run

```bash
rm -rf /tmp/m10-store
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml \
  --ticks 80 --llm mock --out-dir /tmp/m10-store --quiet
ID=$(basename /tmp/m10-store/*_tick_80.ckpt _tick_80.ckpt)
echo "store:    $(grep -c '"type":"store"' /tmp/m10-store/${ID}_events.jsonl || true)"
echo "retrieve: $(grep -c '"type":"retrieve"' /tmp/m10-store/${ID}_events.jsonl || true)"
echo "transfer: $(grep -c '"type":"transfer"' /tmp/m10-store/${ID}_events.jsonl || true)"
grep -m5 '"type":"store"' /tmp/m10-store/${ID}_events.jsonl || true
```

**Success (80-tick default):** hashes will still differ via goals; **`store` is often 0**. Mock only Stores when hunger ≥ 75% **and** food is already in inventory. Default decay does not make agents hungry (gather) until ~tick 167, and then they Eat the food they just gathered. The unit test `mock_storage_goal_stores` is the proof Store fires (it preloads food at max hunger). A 250-tick default coop run still had store=0, gather=22, eat=17.

Compare vs baseline (no schedule):

```bash
rm -rf /tmp/m10-base
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock \
  --out-dir /tmp/m10-base --quiet
cargo run -p sim-cli -- --compare /tmp/m10-base /tmp/m10-store
```

**Success:** `state_hash` **differ**; goal occupancy has `keep the shared storage stocked` 0 vs 16; event table has `Store` and/or `stockpile qty` on B.

---

## 7. `/give` (in-process, hash-sensitive, not on the wire)

### Automated

```bash
cargo test -p sim-core give_is_hashed -- --exact --nocapture
cargo test -p viewer parse_give -- --exact --nocapture
```

**Success:** giving wood changes `state_hash`; `/give 0 berry_bush 2` parses.

### See it in the viewer

```bash
cargo run -p viewer -- --config configs/default.toml
```

Then in the console (`/`):

```
/give 0 berry_bush 3
/follow 0
```

**Success:** inspector inventory shows berry; a **Give** line in the log. Pause, `/give` again, hash in `/tick` changes.

Remote `/give` over TCP/WebSocket must **refuse** (“in-process only”). Do not add a `ControlVerb` (would bump `PROTOCOL_VERSION`).

---

## 8. Viewer crate mesh

Same viewer run as §7, or after a mock coop run with Stores:

1. Legend (`L`): line `cube  stockpile` (brown).
2. After `/give` + a few ticks with the storage goal, or load `/tmp/m10-store/…_tick_80.ckpt`, look for a **brown cube** on a land cell.
3. Fog (`O`) + follow: crate hides when the tile is outside that agent’s Observation.

**Success:** crate present iff the cell has qty > 0; inspector on a standing agent shows `stockpile slots n/16  weight …/80`. Empty cell: `stockpile: (none on this cell)`.

---

## 9. Live Ollama smoke (optional)

Spark must be up. 2 agents × 3 ticks (about 5–15 min). Compare **Wait-rate** to M9’s ~52% `LlmWait`.

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
t = t.replace("count = 16", "count = 2", 1)
t = t.replace("max_retries = 2", "max_retries = 0", 1)
t = t.replace('replay_file = ""', 'replay_file = "/tmp/m10-live-replay.jsonl"')
Path("/tmp/m10-live.toml").write_text(t)
PY
rm -rf /tmp/m10-live /tmp/m10-live-replay.jsonl
curl -sS -m 5 http://spark-bcce.hlab:11434/api/tags | python3 -c \
  'import json,sys; print("\n".join(x["name"] for x in json.load(sys.stdin)["models"]))'
cargo run -p sim-cli -- --config /tmp/m10-live.toml --ticks 3 --llm ollama \
  --out-dir /tmp/m10-live --quiet
ID=$(basename /tmp/m10-live/*_tick_3.ckpt _tick_3.ckpt)
echo "llm=$(grep -c '"chooser":"llm"' /tmp/m10-live/${ID}_decisions.jsonl || true)"
echo "mock=$(grep -c '"chooser":"mock"' /tmp/m10-live/${ID}_decisions.jsonl || true)"
echo "llm_wait=$(grep -c '"policy_branch":"llm_wait"' /tmp/m10-live/${ID}_decisions.jsonl || true)"
wc -l /tmp/m10-live-replay.jsonl
```

| Check | Pass |
|---|---|
| `final_tick=3`, exit 0 | required |
| `"chooser":"llm"` = 6, mock = 0 | required (HTTP path) |
| Replay JSONL 6 lines | required |
| `llm_wait` **lower than 5/6** if parse+`think:false` worked | desirable; **do not** fail CI if Spark is noisy |
| Overnight 16×80 | optional; same recipe as [`M9-test-plan.md`](M9-test-plan.md) §7 |

---

## 10. Docs / help

```bash
cargo run -p sim-cli -- --help
cargo run -p viewer -- --help 2>/dev/null || true
```

Skim: this file, [`M10-plan.md`](M10-plan.md), [`needs-and-survival.md`](needs-and-survival.md) (Store/Retrieve/Transfer energy row), README current slice **M10**.

---

## Suggested 10-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-llm extract_think_and_reasoning_json -- --exact` | thinking JSON parses |
| 2 | `cargo test -p sim-core wait_sentinel_recording_matches_replay -- --exact` | recording hash = replay |
| 3 | `cargo test -p sim-core --test storage` | Transfer/Store/caps/give |
| 4 | §6 80-tick coop mock + grep `store` | count ≥ 1 |
| 5 | `--compare` base vs coop | hash differ; stockpile/Store rows |
| 6 | viewer `/give 0 berry_bush 3` | inventory + Give event |

Then §9 live smoke if Spark is up.

---

## Execution record

**Ran:** 2026-08-26, repo root. Spark reachable for §9. Viewer GUI not driven (no imgui automation).

| Step | Result | Evidence |
|---|---|---|
| §0 cargo test sim-core / sim-llm / sim-cli ab / viewer | **pass** | all green, no network |
| §1 `parse_think_wrapped_json` | **pass** | ok |
| §1 `extract_think_and_reasoning_json` | **pass** | empty content + reasoning JSON → Rest |
| §2 wait-sentinel unit | **pass** | recording hash = replay; same `LlmWait` count |
| §2 `sim-cli` wait + replay file | **pass** | `__llm_wait__` in JSONL; two replays `final_hash=2836bd08…` |
| §3 empty URL mock | **pass** | `empty_url_ollama_is_mock` |
| §4–5 `--test storage` | **pass** | 9/9 (Transfer, caps, energy, give hash, M9 ckpt) |
| §5 haul 4 stones | **pass** | 480 milli |
| §6 `mock_storage_goal_stores` | **pass** | preloaded food → Store |
| §6 80-tick coop mock grep | **partial** | store=0 retrieve=0 transfer=0 (see note above) |
| §6 `--compare` 80-tick base vs coop | **pass** | hashes **differ**; goal 0 vs 16; stockpile cells 0=0 |
| §7 `give_is_hashed` / `parse_give` | **pass** | |
| §8 viewer crate / `/give` in GUI | **not run** | no display automation |
| §9 live 2×3 Nemotron | **pass with notes** | see below |
| §10 `--help` | **pass** | `--compare`, empty URL mock |

### §9 live smoke (2 agents × 3 ticks, `nemotron3:33b`, ~40 s)

| Metric | Value |
|---|---|
| `final_tick` | 3 |
| `"chooser":"llm"` | **6 / 6** |
| `"chooser":"mock"` | 0 |
| `policy_branch=llm` | 1 (parsed Wait) |
| `policy_branch=llm_wait` | 5 |
| Replay JSONL | 6 lines; waits stored as `{"__llm_wait__":true}` |

Same Wait-rate as the M9 2×3 smoke (5/6). HTTP path is proven; parse+`think:false` did not lift this tiny sample. Overnight 16×80 not re-run.
