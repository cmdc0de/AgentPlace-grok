# M14 test plan — see each new feature

Walkthrough for [`M14-plan.md`](M14-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (coalition overlay, `--load DIR` scrubber). Next slice: [`M47-plan.md`](M47-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test incentives` (or `--test checkpoint`); library unit tests need `--lib incentive::tests::…`. A bare `--exact` filter on the package can miss those.

**Success for the slice:** `applies_to = "supporters_of:proposal_N"` (also `supporters_of:N`) puts **current** supporters in scope for per-tick effects; a non-supporter is not; missing proposal id is empty scope (not a load error); `supporters_of:nope` is a load error; same seed twice hashes match. Viewer `--load DIR` lists `{id}_tick_{T}.ckpt` and loads the file **at or before** the requested tick (slider, `[` `]`, `/scrub TICK`, `/ckpt next|prev`). No interpolation, no jump without a file, in-process only (remote attach refuses). `format_version = 2`, `PROTOCOL_VERSION = 2`. Shipping `configs/default.toml` / `coop.toml` unchanged.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p viewer
```

---

## 1. Parse `supporters_of:`

```bash
cargo test -p sim-core --lib incentive::tests::parses_supporters_of_scope -- --exact --nocapture
cargo test -p sim-core --lib incentive::tests::supporters_of_nope_is_load_error -- --exact --nocapture
cargo test -p sim-core --test incentives supporters_of_nope_is_load_error -- --exact --nocapture
```

| Test | Success |
|---|---|
| `supporters_of:proposal_0` and `supporters_of:3` | load |
| `supporters_of:nope` | load error mentioning `applies_to` |

CLI (empty scope is still a valid schedule; bad token is not):

```bash
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coalition.toml --ticks 2 --llm mock --quiet
# expect exit 0

python3 - <<'PY'
from pathlib import Path
Path("/tmp/m14-nope.toml").write_text('''
[[incentives]]
id = "c"
applies_to = "supporters_of:nope"
''')
PY
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives /tmp/m14-nope.toml --ticks 1 --llm mock --quiet
# expect non-zero exit mentioning unsupported applies_to "supporters_of:nope"
```

A default mock run usually has **no** proposal 0, so `coalition.toml` is empty scope for the multiplier. The schedule still starts (`IncentiveApplied`), so the 2-tick hash **differs** from a no-schedule run. That is expected; the unit tests in §2 are the proof a supporter gets 1.4×.

---

## 2. Coalition in scope

Agent 0 Proposes (becomes supporter of proposal 0). Inject `coalition.toml`. Food millipoints: supporter 1400, other agent 1000. Missing id 99 → 1000. Same seed twice → same hash.

```bash
cargo test -p sim-core --test incentives supporters_of_in_scope_for_supporter_only -- --exact --nocapture
cargo test -p sim-core --test incentives supporters_of_missing_proposal_is_empty_scope -- --exact --nocapture
cargo test -p sim-core --test incentives coalition_same_seed_same_hash -- --exact --nocapture
cargo test -p sim-core --test incentives same_seed_no_schedule_same_hash -- --exact --nocapture
```

| Test | Success |
|---|---|
| Agent 0 proposed | 1.4× food; agent 1 stays 1.0× |
| `supporters_of:proposal_99` | 1.0×, no crash |
| Coalition schedule, Wait chooser, same seed | hashes match |
| No schedule, same seed | hashes still match (M8 baseline) |

One-shots (`goal_injection`, `relationship_delta`, `influence_factor_delta`) still apply only at incentive **start** to whoever already supports then. This slice does not track join/leave for those.

---

## 3. Checkpoint helper (at or before)

```bash
cargo test -p sim-core --test checkpoint ckpt_at_or_before_picks_latest_not_after -- --exact --nocapture
```

Dummy files at ticks 10 / 40 / 80: want 50 → 40; want 80 → 80; want 5 → none.

Write a real run directory (viewer `--load DIR` uses these names):

```bash
rm -rf /tmp/m14
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m14 --checkpoint-every 40 --llm mock --quiet
ls /tmp/m14/*.ckpt
```

Expect `{id}_tick_40.ckpt` and `{id}_tick_80.ckpt`. Filename pick (same rule as `ckpt_at_or_before`):

```bash
python3 - <<'PY'
from pathlib import Path
files = []
for p in Path("/tmp/m14").glob("*.ckpt"):
    if "_tick_" not in p.stem:
        continue
    files.append((int(p.stem.rsplit("_tick_", 1)[1]), p.name))
files.sort()
print("listed:", files)
for want in (0, 40, 50, 80, 200):
    picked = [f for f in files if f[0] <= want]
    print(f"want {want} ->", picked[-1] if picked else None)
PY
```

| want | file |
|---|---|
| 0 | none |
| 40 | tick 40 |
| 50 | tick 40 |
| 80 | tick 80 |
| 200 | tick 80 |

Scrub **is** `--load`. Play from that tick is hash-sensitive:

```bash
ID=$(basename /tmp/m14/*_tick_80.ckpt _tick_80.ckpt)
cargo run -p sim-cli -- --load /tmp/m14/${ID}_tick_40.ckpt --ticks 40 --llm mock --quiet
# final_hash must match the uninterrupted 80-tick run
```

`sim-cli --load` is still a **file**. Directory load is the **viewer**.

---

## 4. Viewer scrubber (researcher)

```bash
cargo test -p viewer commands::tests::parse_scrub_and_ckpt -- --exact --nocapture
cargo test -p viewer commands::tests::scrubber_loads_at_or_before_and_steps -- --exact --nocapture
```

| Test | Success |
|---|---|
| `/scrub 40` `/ckpt next` `/ckpt prev` | parse |
| apply(4) with files at 2 and 5 | loads tick 2, pauses, hash matches the tick-2 save |
| next / prev | 5 then 2 |
| `/scrub 0` | error (no file at or before) |
| remote | “in-process only” |

GUI (not automated):

```bash
cargo run -p viewer -- --load /tmp/m14
```

Status slider / `[` `]` / `/scrub 50` / `/ckpt prev` load the ckpt **at or before** that tick and **replace** the in-process sim (paused). `/help` lists `/scrub` and `/ckpt`. Remote `--connect` refuses scrub (no new `ControlVerb`). Vegetation / world meshes may stay stale after a jump (same as live play); that is not this slice.

---

## 5. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M14-plan.md`](M14-plan.md), [`incentive-schedule-format.md`](incentive-schedule-format.md) (`supporters_of:proposal_N`), [`configs/incentives/coalition.toml`](../configs/incentives/coalition.toml), README current slice **M14**.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test incentives supporters_of_in_scope_for_supporter_only` | 1400 vs 1000 |
| 2 | `supporters_of_nope_is_load_error` | error |
| 3 | `ckpt_at_or_before_picks_latest_not_after` | 50 → 40 |
| 4 | §0 `cargo test -p sim-core` / `cargo test -p viewer` | all green |

---

## Execution record

**Ran:** 2026-08-26, repo root. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 13; checkpoint 8; incentives 16; governance 19 |
| §0 `cargo test -p viewer` | **pass** | 7/7 including parse_scrub and scrubber apply |
| §1 parse unit tests (`--exact`) | **pass** | `proposal_0` / `:3` load; `nope` errors |
| §1 CLI `coalition.toml` 2 ticks | **pass** | `final_tick=2` `final_hash=511d0d52…` (differs from no-schedule `70e5204d…` — IncentiveApplied with empty board) |
| §1 CLI `supporters_of:nope` | **pass** | exit 1, `unsupported applies_to "supporters_of:nope"` |
| §2 coalition in-scope tests | **pass** | supporter 1400; missing id 1000; same-seed hashes match |
| §3 helper unit test | **pass** | 50 → 40; 80 → 80; 5 → none |
| §3 write `/tmp/m14` 80 ticks / every 40 | **pass** | `702ebb65922c971a_tick_40.ckpt`, `_tick_80.ckpt`; `final_hash=a9846c96…` |
| §3 filename pick 0/40/50/80/200 | **pass** | none / 40 / 40 / 80 / 80 |
| §3 load tick 40 + 40 ticks | **pass** | `final_tick=80` same `a9846c96…` |
| §4 viewer parse + apply tests | **pass** | `/scrub` `/ckpt`; apply(4) → tick 2; remote refused |
| §4 viewer `--load /tmp/m14` GUI | **not run** | no display automation |
| §5 `--help` | **pass** | `--incentives`, `--inject`, `--load PATH` listed |
