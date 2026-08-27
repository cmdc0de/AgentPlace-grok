# M9 test plan — see each new feature

Walkthrough for [`M9-plan.md`](M9-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (reports, JSONL, `--compare` markdown, live decisions). Current next slice: [`M12-plan.md`](M12-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network. Live Ollama needs Spark.

| Host / model | Value |
|---|---|
| Ollama | `http://spark-bcce.hlab:11434` (`spark-bcce.halb` does not resolve) |
| Model | `nemotron3:33b` (lab default; also `tinyllama:1.1b`, `gemma4:e4b`, …) |
| Measured latency | ~45 s for a tiny JSON action on Spark (2026-08-25) |
| Default config | `provider = "mock"` (CI). `--llm ollama` uses Spark + `nemotron3:33b`, `timeout_ms = 120000` |

`16 agents × 80 ticks` live is **~16 hours** at 45 s/call. The live section therefore uses a **2 agent × 3 tick smoke**. The 80-tick A/B stays as an overnight recipe.

---

## 0. Safety net

Confirms the slice compiles and the new tests pass. No network.

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p sim-cli --test ab
```

Expect all green, then continue so you can see behavior, not only test names.

---

## 1. Observation + LLM prompt (needs, inventory, incentives, named actions)

The agent (and the prompt) now see hunger/thirst/energy on 0–100, inventory names, toxin facts, active incentive ids, and legal actions like `Gather berry_bush` instead of `Gather { species: 1 }`.

### Automated

```bash
cargo test -p sim-core observation_includes_needs_and_incentives -- --exact --nocapture
cargo test -p sim-llm prompt_contains_needs_and_incentive -- --exact --nocapture
```

### See it in a run

```bash
rm -rf /tmp/m9-prompt
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml \
  --ticks 20 --llm mock --report --out-dir /tmp/m9-prompt --quiet
ID=$(basename /tmp/m9-prompt/*_tick_20.ckpt _tick_20.ckpt)
less /tmp/m9-prompt/${ID}_tick_20_report.md
grep incentive_applied /tmp/m9-prompt/${ID}_events.jsonl | head
grep prompt_hash /tmp/m9-prompt/${ID}_decisions.jsonl | head
```

| Surface | Evidence |
|---|---|
| Report | `needs: hunger … thirst … energy …` and `goals: keep the shared storage stocked` |
| Events | `"type":"incentive_applied","id":"early_cooperation_bonus"` |
| Decisions | 16 × 20 = **320** lines, each with `prompt_hash` |
| `sim-llm` unit test | prompt contains `hunger=40`, `thirst=22`, `coop_food`, `Drink`, `berry_bush` |

`sim-cli` does not print the full prompt (only `prompt_hash` in decisions JSONL). The `sim-llm` test is the direct check of prompt text.

---

## 2. Empty URL ⇒ mock; default Spark URL

`provider = "mock"` never calls HTTP. `ollama` / `xai` / `openai_compatible` with an **empty** `base_url` also becomes mock. The default live URL is Spark, used only with `--llm ollama`.

### Automated

```bash
cargo test -p sim-llm empty_url_ollama_is_mock -- --exact --nocapture
cargo test -p sim-llm empty_url_xai_is_mock -- --exact --nocapture
cargo test -p sim-llm mock_provider_ignores_url -- --exact --nocapture
cargo test -p sim-llm ollama_with_url_is_custom -- --exact --nocapture
```

### See the config

```bash
grep -A8 '^\[llm\]' configs/default.toml
```

Expect `provider = "mock"`, `base_url = "http://spark-bcce.hlab:11434"`, `model = "nemotron3:33b"`.

### Mock chooser in decisions (no HTTP)

```bash
rm -rf /tmp/m9-mock
cargo run -p sim-cli -- --config configs/default.toml --ticks 3 \
  --llm mock --out-dir /tmp/m9-mock --quiet
ID=$(basename /tmp/m9-mock/*_tick_3.ckpt _tick_3.ckpt)
head -1 /tmp/m9-mock/${ID}_decisions.jsonl
```

Look for `"chooser":"mock"`. The process must finish with no connection error.

### Empty `base_url` + `provider = ollama` still mocks

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace('provider = "mock"', 'provider = "ollama"', 1)
t = t.replace('base_url = "http://spark-bcce.hlab:11434"', 'base_url = ""')
Path("/tmp/m9-empty-url.toml").write_text(t)
print("wrote /tmp/m9-empty-url.toml")
PY
cargo run -p sim-cli -- --config /tmp/m9-empty-url.toml --ticks 2 --quiet
```

Expect `final_hash=` and **no** connection error (chooser falls back to mock).

---

## 3. Record / replay JSONL

If `replay_file` is a missing path, the run **appends** JSONL. If that file already exists, the next run **replays** it. Two replay passes must share `state_hash`.

### Automated

```bash
cargo test -p sim-core replay_fixture_twice_same_hash -- --exact --nocapture
```

### See a file written, then bit-identical replay

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace('replay_file = ""', 'replay_file = "/tmp/m9-replay.jsonl"')
Path("/tmp/m9-replay.toml").write_text(t)
PY
rm -f /tmp/m9-replay.jsonl
cargo run -p sim-cli -- --config /tmp/m9-replay.toml --ticks 5 --llm mock --quiet
wc -l /tmp/m9-replay.jsonl
head -2 /tmp/m9-replay.jsonl
```

Expect 16 agents × 5 ticks = **80** lines (`tick`, `agent`, `call_seed`, `prompt_hash`, `response`).

Replay twice (file now exists, so both runs replay):

```bash
cargo run -p sim-cli -- --config /tmp/m9-replay.toml --ticks 5 --llm mock --quiet
cargo run -p sim-cli -- --config /tmp/m9-replay.toml --ticks 5 --llm mock --quiet
```

The two `final_hash=` lines must match.

---

## 4. `sim-cli --compare`

Markdown (optional CSV) diff of two `--out-dir`s or `.ckpt` files: hash, deaths, board, consumption, need means, goal occupancy, incentives, event histogram.

### Automated

```bash
cargo test -p sim-core compare_identical_sims_hashes_equal -- --exact --nocapture
cargo test -p sim-core compare_coop_vs_baseline_differs_goals -- --exact --nocapture
cargo test -p sim-cli --test ab compare_identical_dirs_hashes_equal -- --exact --nocapture
cargo test -p sim-cli --test ab compare_coop_vs_baseline_differs -- --exact --nocapture
```

### See the markdown

```bash
rm -rf /tmp/m9-base /tmp/m9-coop
cargo run -p sim-cli -- --config configs/default.toml --ticks 40 --llm mock \
  --out-dir /tmp/m9-base --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 40 --llm mock \
  --incentives configs/incentives/coop.toml --out-dir /tmp/m9-coop --quiet
cargo run -p sim-cli -- --compare /tmp/m9-base /tmp/m9-coop
```

Look for:

- `state_hash` row: **differ**
- Goal occupancy: `keep the shared storage stocked` with A=`0`, B=`16`
- Active incentives: `early_cooperation_bonus` A=`false` B=`true`
- Event kinds: `IncentiveApplied` only on B

Same dir against itself:

```bash
cargo run -p sim-cli -- --compare /tmp/m9-base /tmp/m9-base
```

`state_hash` must say **equal**.

CSV form (markdown first, then `field,a,b,delta`):

```bash
cargo run -p sim-cli -- --compare /tmp/m9-base /tmp/m9-coop --csv | tail
```

---

## 5. Mock drinks / eats before thirst-death

Mock treats thirst/hunger as urgent below **75%** (was 50%). With default decay, agents start seeking water around tick 100 instead of 200, so they can `Drink` before tick-400 death.

### Automated

```bash
cargo test -p sim-core mock_drinks_before_half_thirst -- --exact --nocapture
cargo test -p sim-core same_seed_same_hash -- --exact --nocapture
```

The first parks an agent on water with thirst still **above** the old 50% line and asserts a `Drink`. The second is the determinism guard after that policy tweak.

### See drinks in a default-length run

```bash
rm -rf /tmp/m9-drink
cargo run -p sim-cli -- --config configs/default.toml --ticks 150 --llm mock \
  --out-dir /tmp/m9-drink --report --quiet
ID=$(basename /tmp/m9-drink/*_tick_150.ckpt _tick_150.ckpt)
echo "drinks:  $(grep -c '"type":"drink"' /tmp/m9-drink/${ID}_events.jsonl)"
echo "eats:    $(grep -c '"type":"eat"' /tmp/m9-drink/${ID}_events.jsonl)"
echo "deaths:  $(grep -c '"type":"died"' /tmp/m9-drink/${ID}_events.jsonl || true)"
grep -m3 '"type":"drink"' /tmp/m9-drink/${ID}_events.jsonl
```

Look for at least one `"type":"drink"` (usually many by tick 150), **0** deaths (thirst death is tick 400 if nobody drinks), and a report thirst mean well above 0.

---

## 6. Docs / CLI help

```bash
cargo run -p sim-cli -- --help
```

Confirm `--compare A B`, `--csv`, and `--llm` mentioning empty `base_url` ⇒ mock.

Skim: this file, [`M9-plan.md`](M9-plan.md) (status **implemented**), [`README.md`](../README.md) (Spark URL + compare recipe), [`needs-and-survival.md`](needs-and-survival.md) (thirsty/hungry at **75%**).

---

## 7. Live Ollama — Spark + `nemotron3:33b`

Mid-run HTTP failure is **`Wait` / `LlmWait`**, not mock. Seed is sent on the wire; providers often ignore it — use `replay_file` for bit-identical reruns.

### Probe (must work before a sim run)

```bash
curl -sS -m 5 http://spark-bcce.hlab:11434/api/tags | python3 -c \
  'import json,sys; m=json.load(sys.stdin)["models"]; print("\n".join(x["name"] for x in m))'
```

Expect `nemotron3:33b` in the list.

### Smoke: 2 agents × 3 ticks (about 5–10 min)

Default is 16 agents; do **not** use that here. Generate a tiny live config (2 agents, 120 s timeout, record JSONL):

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("provider = \"mock\"", "provider = \"ollama\"", 1)
t = t.replace("count = 16", "count = 2", 1)
t = t.replace("replay_file = \"\"", 'replay_file = "/tmp/m9-live-replay.jsonl"')
Path("/tmp/m9-live.toml").write_text(t)
print(t[t.find("[llm]"):t.find("[llm]")+400])
PY
rm -rf /tmp/m9-live /tmp/m9-live-replay.jsonl
cargo run -p sim-cli -- --config /tmp/m9-live.toml --ticks 3 --llm ollama \
  --out-dir /tmp/m9-live --quiet
ID=$(basename /tmp/m9-live/*_tick_3.ckpt _tick_3.ckpt)
echo "---- decisions ----"
cat /tmp/m9-live/${ID}_decisions.jsonl
echo "---- replay ----"
wc -l /tmp/m9-live-replay.jsonl
echo "---- events (llm_wait / drink) ----"
grep -E 'llm_wait|"type":"drink"' /tmp/m9-live/${ID}_events.jsonl || true
```

Look for:

| Check | Pass |
|---|---|
| Process exit 0, `final_tick=3` | required |
| `"chooser":"llm"` on most decision lines | required (proves HTTP path, not mock) |
| Replay JSONL has **6** lines (2×3) | required |
| `"chooser":"mock"` | **fail** — wrong fallback |
| Many `"type":"llm_wait"` | Spark timeout / model name wrong; raise `timeout_ms` or confirm `nemotron3:33b` |

Replay the captured file (no second HTTP):

```bash
cargo run -p sim-cli -- --config /tmp/m9-live.toml --ticks 3 --llm ollama --quiet
cargo run -p sim-cli -- --config /tmp/m9-live.toml --ticks 3 --llm ollama --quiet
```

Hashes of those two replay passes must match. They need not match the recording run if the live chooser consumed no agent RNG (they usually **do** match when every tick was served from JSONL).

### Overnight (do not run in a short review)

Default population, 80 ticks, baseline vs coop — `16 × 80 × ~45 s ≈ 16 h` per arm:

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm ollama \
  --out-dir /tmp/m9-live-base --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm ollama \
  --incentives configs/incentives/coop.toml --out-dir /tmp/m9-live-coop --quiet
cargo run -p sim-cli -- --compare /tmp/m9-live-base /tmp/m9-live-coop
```

Here `--compare` is the point: diet/board can finally move because the prompt includes needs and incentives.

---

## Suggested 10-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-llm prompt_contains_needs_and_incentive -- --exact` | needs + `coop_food` + `berry_bush` |
| 2 | `cargo test -p sim-llm empty_url_ollama_is_mock -- --exact` | empty URL is mock |
| 3 | §3 record/replay | 80-line JSONL, two equal hashes |
| 4 | §4 `--compare /tmp/m9-base /tmp/m9-coop` | hash **differ**, coop goal occupancy |
| 5 | §5 drink grep on a 150-tick run | `"type":"drink"` count > 0 |

Then §7 live smoke if Spark is up.

---

## Execution record

**Ran:** 2026-08-25, repo root, host Spark `http://spark-bcce.hlab:11434` model `nemotron3:33b`.  
**Overnight 16×80 live A/B:** not run (~16 h). Everything else below was executed.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | all packages green (incl. replay, compare, survival, incentives) |
| §0 `cargo test -p sim-llm` | **pass** | 5 tests (prompt + empty-URL mock + custom chooser) |
| §0 `cargo test -p sim-cli --test ab` | **pass** | 3 tests (hash A/B + `--compare` identical + coop) |
| §1 20-tick mock + coop + `--report` | **pass** | 320 decision lines; report goals include `keep the shared storage stocked`; `incentive_applied` ×1; needs printed (hunger 97 / thirst 95) |
| §2 mock 3-tick | **pass** | `"chooser":"mock"` ×48, `"chooser":"llm"` ×0 |
| §2 empty `base_url` + provider ollama | **pass** | `final_tick=2` / `final_hash=70e5204d…` no connection error |
| §3 record 5 ticks | **pass** | `/tmp/m9-replay.jsonl` **80** lines |
| §3 replay twice | **pass** | both `final_hash=44a546b3785159351d6f17b8ec6145b64849ce5148143a75afb043ad17002adf` |
| §4 `--compare` coop vs base (40 ticks) | **pass** | `state_hash` **differ**; `early_cooperation_bonus` A=false B=true; goal `keep the shared storage stocked` 0 vs 16 |
| §4 `--compare` same dir | **pass** | `state_hash` **equal** |
| §5 150-tick mock drink | **pass** | **14** `"type":"drink"` starting tick **102** (75% cutoff); eats=0; deaths=0; thirst mean 87.0 |
| §6 `--help` | **pass** | `--compare`, `--csv`, empty `base_url` ⇒ mock |
| §7 probe `/api/tags` | **pass** | `nemotron3:33b` listed (also tinyllama, gemma4, glm-4.7-flash, qwen3.5) |
| §7 live 2×3 `--llm ollama` | **pass with notes** | see below |
| §7 live replay twice | **pass** | both `final_hash=4c4887ed030626e08a6cff0853cc09a0bc6773343fbacd59729a04a2cdf6ae6c` |

### §4 compare excerpt (mock, 40 ticks)

```
| state_hash | `2977a2b4…aeecbb` | `578b3e43…b174e5` | differ |
| population | 16 | 16 | 0 |
- `early_cooperation_bonus` A=false B=true
| keep the shared storage stocked | 0 | 16 | 16 |
```

### §5 drinks

First drinks at ticks 102, 103, 105 (thirsty line is 75% ≈ tick 100). 14 Drink events by tick 150; no deaths.

### §7 live smoke (2 agents × 3 ticks, `nemotron3:33b`, `timeout_ms=120000`, `max_retries=0`)

Standalone chat/completions (compact prompt) on Spark: **7.3 s**, `content={"action":"Drink",…}` plus a `reasoning` field.

Full sim run wall ~30 s:

| Metric | Value |
|---|---|
| `final_tick` | 3 |
| `"chooser":"llm"` | **6 / 6** (HTTP path; not mock) |
| `"chooser":"mock"` | 0 |
| `policy_branch=llm` | 1 (parsed Wait) |
| `policy_branch=llm_wait` | 5 (`LlmWait` events; failure in ~20–90 ms, **not** the 120 s timeout) |
| Replay JSONL | 6 lines |

So Spark is reachable and `--llm ollama` does **not** fall back to mock. Most full-observation calls still became `Wait` (`LlmWait`) on this run; a compact probe of the same model returns legal JSON. Likely cause: thinking-model payload (`content` + `reasoning`) or a short HTTP error after the first long call — **not** empty-URL mock.

Replay of that JSONL twice: hashes **match each other**. Recording-run hash can differ from replay when the record contains `LlmWait` events (replay of serialized `Wait` does not re-emit `LlmWait`). That is expected with the current record format.

### Artifacts

| Path | What |
|---|---|
| `/tmp/m9-prompt/` | §1 report + 320 decisions |
| `/tmp/m9-replay.jsonl` | §3 80-line mock replay |
| `/tmp/m9-base/` `/tmp/m9-coop/` | §4 compare inputs |
| `/tmp/m9-drink/` | §5 150-tick drink run |
| `/tmp/m9-live.toml` | live config (2 agents, ollama, nemotron3:33b) |
| `/tmp/m9-live/` | live ckpt + decisions + events |
| `/tmp/m9-live-replay.jsonl` | 6 live records |

