# M18 test plan — see each new feature

Walkthrough for [`M18-plan.md`](M18-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (council tally, SetCouncil, `/scrub` catch-up). Next slice: [`M51-plan.md`](M51-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test governance`; library unit tests need `--lib voting::tests::…`.

**Success for the slice:** omit / `council_tally = "unanimous"` keeps M16 unanimous council; `council_tally = "majority"` equal 2-of-3 Accepts and 1-of-3 stays Open; influence kingmaker on a 3-person council Accepts with only agent 0 Support; SetCouncil Propose waits unless `allow_meta_rules`; empty ids Wait; adopted roster `[0,2]` is unanimous among those living ids; `/scrub` between ckpt files lands on the requested tick (idempotent; `/ckpt next` still file-to-file); remote `/scrub` refused. Shipping `default.toml` unchanged. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p viewer
```

---

## 1. Parse `[voting] council_tally`

```bash
cargo test -p sim-core --lib voting::tests::omit_council_tally_is_unanimous -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::config_toml_council_tally_majority -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::unknown_council_tally_errors -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::council_tally_without_council_accept_errors -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit `council_tally` with `accept = "council"` | `CouncilTally::Unanimous` |
| `council_tally = "majority"` + council list | parses |
| `council_tally = "maybe"` | load error |
| `council_tally` without `accept = "council"` | load error |

CLI:

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text() + '\n[voting]\naccept = "council"\ncouncil = [0]\ncouncil_tally = "maybe"\n'
Path("/tmp/m18-bad-tally.toml").write_text(t)
t2 = Path("configs/default.toml").read_text() + '\n[voting]\ncouncil_tally = "majority"\n'
Path("/tmp/m18-tally-no-council.toml").write_text(t2)
PY
cargo run -p sim-cli -- --config /tmp/m18-bad-tally.toml --ticks 1 --llm mock --quiet
# expect non-zero: unknown council_tally "maybe"
cargo run -p sim-cli -- --config /tmp/m18-tally-no-council.toml --ticks 1 --llm mock --quiet
# expect non-zero: council_tally requires voting accept = "council"
```

---

## 2. Majority-of-council tally

3-person council. Author 0 is already a supporter after Propose. Omit `council_tally` stays M16 unanimous (2 of 3 Open).

```bash
cargo test -p sim-core --test governance council_two_support_accepts_with_silent_third -- --exact --nocapture
cargo test -p sim-core --test governance council_majority_two_of_three_accepts -- --exact --nocapture
cargo test -p sim-core --test governance council_majority_one_of_three_stays_open -- --exact --nocapture
cargo test -p sim-core --test governance council_majority_influence_kingmaker -- --exact --nocapture
cargo test -p sim-core --test governance council_unanimous_two_of_three_stays_open -- --exact --nocapture
```

| Test | Success |
|---|---|
| M16 council `[0,1]`, both Support, third silent | **Accepted** |
| `council_tally = majority`, 2 of 3 Support | **Accepted** |
| `council_tally = majority`, 1 of 3 Support | **Open** |
| influence king, only 0 Supports | **Accepted** |
| omit tally, 2 of 3 Support | **Open** (M16) |

---

## 3. SetCouncil meta-rule

```bash
cargo test -p sim-core --test governance meta_set_council_waits_when_flag_off -- --exact --nocapture
cargo test -p sim-core --test governance empty_set_council_propose_waits -- --exact --nocapture
cargo test -p sim-core --test governance meta_set_council_roster_then_unanimous -- --exact --nocapture
cargo test -p sim-core --test governance parse_set_council_json -- --exact --nocapture
```

| Test | Success |
|---|---|
| `allow_meta_rules = false` | SetCouncil Propose → **Wait** |
| empty `ids` | **Wait** |
| adopt SetCouncil `[0,2]`, then 0 and 2 Support | second proposal **Accepted**; 2 Oppose → **Rejected** |
| JSON `set_council` council `[0,2,2]` | deduped `[0,2]`; empty array → Wait |

---

## 4. Jump-to-tick catch-up

```bash
cargo test -p viewer commands::tests::scrubber_catches_up_between_ckpts -- --exact --nocapture
cargo test -p viewer commands::tests::scrubber_loads_at_or_before_and_steps -- --exact --nocapture
```

| Test | Success |
|---|---|
| ckpts at 2 and 4, `/scrub` 3 | `sim.tick == 3`; hash matches ticking the tick-2 ckpt one step |
| `/scrub` 3 twice | same tick/hash (no extra events) |
| `/ckpt next` from 3 | file tick **4** (no catch-up) |
| remote `/scrub` | refused |

Viewer (not automated): `--load DIR` with ckpts at 40 and 80; slider / `/scrub 50` should show tick **50** (not 40). `/ckpt next` jumps to the 80 file. Logs `/events` slider still display-only.

---

## 5. Default mock hash

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** `final_tick=2` `final_hash=70e5204d…` (same as M14/M16/M17 idle default).

---

## 6. Docs / help

```bash
cargo run -p sim-cli -- --help
cargo test -p viewer commands::tests::parse_help -- --exact --nocapture
```

Skim: this file, [`M18-plan.md`](M18-plan.md), README current slice **M18**. `/help` lists `/scrub` catch-up wording.

---

## 7. Overnight Spark live A/B (LLM + M18 council tally)

**Not CI.** Host `http://spark-bcce.hlab:11434` (`spark-bcce.halb` does not resolve). Model `nemotron3:33b`, `timeout_ms = 120000`. Empty `base_url` still mock. Mid-run HTTP failure is `Wait`, not mock.

4 agents × 40 ticks × 2 arms. At ~45 s/call that is **hours**. Optional 2×3 smoke first.

| Arm | Overlay | What it tests |
|---|---|---|
| **A** | `[voting] accept = "council"` `council = [0, 1, 2, 3]` (omit `council_tally` → unanimous) | M16 council under live LLM |
| **B** | same + `council_tally = "majority"` | M18 majority-of-council under live LLM |

Do **not** flip shipping `default.toml`. CI stays `provider = mock`.

### 7a. Spark up?

```bash
curl -sS -m 5 http://spark-bcce.hlab:11434/api/tags | python3 -c \
  'import json,sys; m=[x.get("name","") for x in json.load(sys.stdin).get("models",[])]; print("\n".join(m)); assert any("nemotron3" in x for x in m), m'
```

### 7b. Optional smoke (2 agents × 3 ticks)

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("count = 16", "count = 2", 1)
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
Path("/tmp/m18-live-smoke.toml").write_text(t)
PY
cargo run -p sim-cli -- --config /tmp/m18-live-smoke.toml --ticks 3 --llm ollama \
  --out-dir /tmp/m18-smoke --quiet
# expect final_tick=3, a decisions JSONL, no empty-URL mock
```

### 7c. Overnight A/B (4 agents × 40 ticks)

Run from the **repo root**. Start before leaving; both arms are sequential (arm B waits for A). `nohup` so SSH drop does not kill it:

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("count = 16", "count = 4", 1)
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
vote = '\n[voting]\naccept = "council"\ncouncil = [0, 1, 2, 3]\n'
Path("/tmp/m18-live-a.toml").write_text(t + vote)
Path("/tmp/m18-live-b.toml").write_text(t + vote + 'council_tally = "majority"\n')
PY

mkdir -p /tmp/m18-overnight
nohup bash -lc '
set -euo pipefail
cd /home/cmdc0de/dev/AgentPlace-grok
echo "=== ARM A unanimous council $(date -Is) ==="
cargo run -p sim-cli -- --config /tmp/m18-live-a.toml --ticks 40 --llm ollama \
  --out-dir /tmp/m18-live-a --report --quiet
echo "=== ARM B majority-of-council $(date -Is) ==="
cargo run -p sim-cli -- --config /tmp/m18-live-b.toml --ticks 40 --llm ollama \
  --out-dir /tmp/m18-live-b --report --quiet
echo "=== COMPARE $(date -Is) ==="
cargo run -p sim-cli -- --compare /tmp/m18-live-a /tmp/m18-live-b
echo "=== DONE $(date -Is) ==="
' > /tmp/m18-overnight/run.log 2>&1 &
echo "pid $!  log /tmp/m18-overnight/run.log"
```

**Proof when finished:**

```bash
tail -50 /tmp/m18-overnight/run.log
ls /tmp/m18-live-a/*_decisions.jsonl /tmp/m18-live-b/*_decisions.jsonl
python3 - <<'PY'
from pathlib import Path
for d in ("/tmp/m18-live-a", "/tmp/m18-live-b"):
    js = list(Path(d).glob("*_decisions.jsonl"))
    print(d, "decisions", js[0] if js else None, "lines", sum(1 for _ in open(js[0])) if js else 0)
PY
```

| Check | Success |
|---|---|
| Both arms | `final_tick=40` |
| Decisions JSONL | present; each line has `prompt_hash` |
| `--compare` | hashes **differ** (unanimous council vs majority-of-council) |
| Not mock | Spark was up; not empty-`base_url` fallback |
| `LlmWait` | allowed; all-Wait every tick while Spark is healthy is a **fail** |

To watch: `tail -f /tmp/m18-overnight/run.log`.

Same size as M17 (`4×40`, not `16×80`). If you instead want the old **omit vs `coop.toml`** LLM-health A/B, use the recipe in [`M17-test-plan.md`](M17-test-plan.md) §6 with `/tmp/m18-*` paths.

---

## Execution record

**Ran:** 2026-08-27, repo root. Each `--exact` line invoked separately. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 25; checkpoint 9; compare 2; determinism 11; governance 40; incentives 17; replay 2; social 15; storage 19; survival 18; timing 1; viewer_helpers 2 |
| §0 `cargo test -p viewer` | **pass** | 10/10 |
| §1 parse (`--exact` ×4) | **pass** | omit unanimous; majority parses; maybe / no-accept error |
| §1 CLI `council_tally = "maybe"` | **pass** | exit 1, `unknown council_tally "maybe"` |
| §1 CLI tally without council accept | **pass** | exit 1, `council_tally requires voting accept = "council"` |
| §2 majority-of-council (`--exact` ×5) | **pass** | 2-of-3 Accept; 1-of-3 Open; kingmaker Accept; omit Open |
| §3 SetCouncil (`--exact` ×4) | **pass** | flag off Wait; empty Wait; roster then Accept/Reject |
| §4 catch-up (`--exact` ×2) | **pass** | want 3 → tick 3; twice same hash; next → 4; remote refused |
| §5 default 2 ticks | **pass** | `final_tick=2` `final_hash=70e5204d…` |
| §6 `sim-cli --help` | **pass** | `--incentives`, `--compare` listed |
| §6 `parse_help` | **pass** | `/scrub` listed; help text has tick-forward wording |
| §4 viewer GUI | **not run** | no display automation |
| §7 Spark overnight | **not run** | recipe in §7c; start with `nohup` on a Spark-reachable host |
