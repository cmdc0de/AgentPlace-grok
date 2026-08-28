# M17 test plan — see each new feature

Walkthrough for [`M17-plan.md`](M17-plan.md). Automated tests prove the code; the Spark section is the live LLM proof. CI / `cargo test` never need the network. Next slice: [`M19-plan.md`](M19-plan.md).

`cargo test -- --exact NAME` takes **one** test name.

**Success for the slice:** `allow_meta_rules = false` (default) rejects meta Propose; with the flag on, an adopted `SetAcceptanceThreshold { milli: 1000 }` lets 1-of-3 equal Support Accept; `SetVoteAccept { unanimous }` keeps 1-of-3 Open. Late `supporters_of` joiners get goal + influence one-shots; Oppose reverts influence only. `/events TICK` filters `{id}_events.jsonl` at-or-before without loading a ckpt. Default mock hashes match pre-M17. Overnight Spark A/B (§6) is optional but is how we know `--llm ollama` still works.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test ab
```

---

## 1. Meta-rules

```bash
cargo test -p sim-core --test governance meta_propose_waits_when_flag_off -- --exact --nocapture
cargo test -p sim-core --test governance meta_threshold_lets_one_of_three_accept -- --exact --nocapture
cargo test -p sim-core --test governance meta_unanimous_one_of_three_stays_open -- --exact --nocapture
```

| Test | Success |
|---|---|
| Flag off | meta Propose → **Wait**, no board row |
| Flag on, adopt threshold 0.10, then 1 Support | second proposal **Accepted** |
| Flag on, adopt unanimous, then 1 Support | second proposal **Open** |

---

## 2. Join/leave one-shots

```bash
cargo test -p sim-core --test incentives supporters_of_join_gets_oneshots_leave_reverts_influence -- --exact --nocapture
```

| Test | Success |
|---|---|
| Author after start | goal present; influence = base+15 |
| Non-supporter | no goal |
| After Support + tick | joiner has goal + boost |
| After Oppose + tick | influence back to base; goal remains |

---

## 3. Event JSONL timeline

```bash
cargo test -p sim-core --test checkpoint jsonl_ticks_at_or_before -- --exact --nocapture
cargo test -p viewer commands::tests::parse_events -- --exact --nocapture
```

| Test | Success |
|---|---|
| Dummy ticks 10/40/80, want 50 | **40** |
| `/events 40` | parse; `filter_events(50)` → 40 |

Viewer (not automated): `--load DIR` that contains `*_events.jsonl`; Logs slider / `/events 50`. Does **not** replace the sim (that is `/scrub`).

---

## 4. Default mock hash (no meta flag)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14/M16 idle default).

---

## 5. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M17-plan.md`](M17-plan.md), README current slice **M17**.

---

## 6. Overnight Spark live A/B (LLM integration)

**Not CI.** Host `http://spark-bcce.hlab:11434` (`spark-bcce.halb` does not resolve). Model `nemotron3:33b`, `timeout_ms = 120000`. Empty `base_url` still mock. Mid-run HTTP failure is `Wait`, not mock.

4 agents × 40 ticks × 2 arms. At ~45 s/call that is **hours**, not 16×80 overnight-plus. Optional 2×3 smoke first.

### 6a. Spark up?

```bash
curl -sS -m 5 http://spark-bcce.hlab:11434/api/tags | python3 -c \
  'import json,sys; m=[x.get("name","") for x in json.load(sys.stdin).get("models",[])]; print("\n".join(m)); assert any("nemotron3" in x for x in m), m'
```

### 6b. Optional smoke (2 agents × 3 ticks)

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("count = 16", "count = 2", 1)
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
Path("/tmp/m17-live-smoke.toml").write_text(t)
PY
cargo run -p sim-cli -- --config /tmp/m17-live-smoke.toml --ticks 3 --llm ollama \
  --out-dir /tmp/m17-smoke --quiet
# expect final_tick=3, a decisions JSONL, no "empty URL" mock (hash should differ from mock Wait-only if the model answers)
```

### 6c. Overnight A/B (4 agents × 40 ticks)

Run from the **repo root**. Start before leaving; both arms are sequential (arm B waits for A). `nohup` so SSH drop does not kill it:

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("count = 16", "count = 4", 1)
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
Path("/tmp/m17-live.toml").write_text(t)
PY

mkdir -p /tmp/m17-overnight
nohup bash -lc '
set -euo pipefail
cd /home/cmdc0de/dev/AgentPlace-grok
echo "=== ARM A baseline $(date -Is) ==="
cargo run -p sim-cli -- --config /tmp/m17-live.toml --ticks 40 --llm ollama \
  --out-dir /tmp/m17-live-a --report --quiet
echo "=== ARM B coop $(date -Is) ==="
cargo run -p sim-cli -- --config /tmp/m17-live.toml \
  --incentives configs/incentives/coop.toml --ticks 40 --llm ollama \
  --out-dir /tmp/m17-live-b --report --quiet
echo "=== COMPARE $(date -Is) ==="
cargo run -p sim-cli -- --compare /tmp/m17-live-a /tmp/m17-live-b
echo "=== DONE $(date -Is) ==="
' > /tmp/m17-overnight/run.log 2>&1 &
echo "pid $!  log /tmp/m17-overnight/run.log"
```

**Proof when finished:**

```bash
tail -50 /tmp/m17-overnight/run.log
ls /tmp/m17-live-a/*_decisions.jsonl /tmp/m17-live-b/*_decisions.jsonl
python3 - <<'PY'
from pathlib import Path
for d in ("/tmp/m17-live-a", "/tmp/m17-live-b"):
    js = list(Path(d).glob("*_decisions.jsonl"))
    print(d, "decisions", js[0] if js else None, "lines", sum(1 for _ in open(js[0])) if js else 0)
PY
```

| Check | Success |
|---|---|
| Both arms | `final_tick=40` |
| Decisions JSONL | present; each line has `prompt_hash` |
| `--compare` | hashes **differ** (coop vs omit) |
| Not mock | Spark was up; not empty-`base_url` fallback |
| `LlmWait` | allowed; all-Wait every tick while Spark is healthy is a **fail** |

To watch: `tail -f /tmp/m17-overnight/run.log`.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `meta_propose_waits_when_flag_off` | Wait |
| 2 | `meta_threshold_lets_one_of_three_accept` | Accepted |
| 3 | `supporters_of_join_gets_oneshots_leave_reverts_influence` | influence reverts |
| 4 | `jsonl_ticks_at_or_before` | 50 → 40 |
| 5 | §0 `cargo test -p sim-core` | all green |

---

## Execution record

**Ran:** 2026-08-27, repo root. Viewer GUI not driven. Spark overnight **not started** here (hours).

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | governance 32; incentives 17; checkpoint 9 |
| §0 `cargo test -p viewer` | **pass** | 9/9 including parse_events |
| §0 `cargo test -p sim-cli --test ab` | **not re-run this pass** | previously 3/3; default mock hash unchanged |
| §1 meta unit tests | **pass** | Wait / Accepted / Open |
| §2 join/leave oneshots | **pass** | join boosts; leave reverts influence |
| §3 JSONL helper + `/events` | **pass** | 50 → 40 |
| §4 default mock 2 ticks | **pass** | `final_hash=70e5204d…` |
| §6 Spark overnight | **not run** | recipe in §6c; start with `nohup` on a Spark-reachable host |
