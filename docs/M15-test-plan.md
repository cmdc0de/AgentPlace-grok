# M15 test plan — see each new feature

Walkthrough for [`M15-plan.md`](M15-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (board accept vs open). Next slice: [`M36-plan.md`](M36-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test governance`; library unit tests need `--lib voting::tests::…` or `--lib incentive::tests::…`.

**Success for the slice:** omit / `equal` matches M4 one-agent-one-vote; `[voting] weight = "respect"` plus `esteem.toml` lets **one** Support accept a 3-agent 50% board; default respect 0 (no schedule) hashes match equal. `toward = "nope"` is a load error. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p viewer
```

---

## 1. Parse `[voting]` and `toward`

```bash
cargo test -p sim-core --lib voting::tests::unknown_weight_errors -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::omit_and_equal_parse -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::config_toml_respect -- --exact --nocapture
cargo test -p sim-core --lib incentive::tests::toward_nope_is_load_error -- --exact --nocapture
cargo test -p sim-core --lib incentive::tests::parses_relationship_delta_respect_toward -- --exact --nocapture
```

| Test | Success |
|---|---|
| `weight = "maybe"` | load error (mentions equal, influence, or respect) |
| omit / `"equal"` | `VoteWeight::Equal` |
| `"respect"` | `VoteWeight::Respect` |
| `toward = "nope"` | load error mentioning `toward` |
| `respect = 70.0` + `toward = "agent:0"` | parses |

CLI:

```bash
python3 - <<'PY'
from pathlib import Path
Path("/tmp/m15-nope.toml").write_text('''
[[incentives]]
id = "e"
[[incentives.effects]]
type = "relationship_delta"
respect = 70.0
toward = "nope"
''')
PY
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives /tmp/m15-nope.toml --ticks 1 --llm mock --quiet
# expect non-zero exit: unsupported toward "nope"

python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text() + '\n[voting]\nweight = "maybe"\n'
Path("/tmp/m15-bad.toml").write_text(t)
PY
cargo run -p sim-cli -- --config /tmp/m15-bad.toml --ticks 1 --llm mock --quiet
# expect non-zero exit: unknown voting weight "maybe"
```

---

## 2. Kingmaker tally

3 agents, threshold 0.5, only agent 0 Supports (the author). Equal needs 2 heads. Respect + `esteem.toml` (`respect = 70.0` toward agent 0): incoming 14000 vs 1+1, need 7001, **Accept**.

```bash
cargo test -p sim-core --test governance equal_tally_one_of_three_stays_open -- --exact --nocapture
cargo test -p sim-core --test governance equal_with_esteem_still_open -- --exact --nocapture
cargo test -p sim-core --test governance respect_kingmaker_one_support_accepts -- --exact --nocapture
cargo test -p sim-core --test governance influence_kingmaker_one_support_accepts -- --exact --nocapture
cargo test -p sim-core --test governance respect_zero_same_hash_as_equal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Equal, 1 of 3 | stays **Open** |
| Equal + esteem | still **Open** (heads unchanged) |
| Respect + esteem | **Accepted**, adopted rule copied; weights 14000 / 1 / 1 |
| Influence + leadership | still **Accepted** (M13, not broken) |
| Respect, no schedule, Wait | same hash as equal |

---

## 3. Overlay on a config (optional see-it)

`[voting]` is read from the **experiment** TOML, not from `--incentives`. Default respect 0 ⇒ same hash as omit.

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text().replace("count = 16", "count = 3", 1)
Path("/tmp/m15-3.toml").write_text(t)
Path("/tmp/m15-respect.toml").write_text(t + '\n[voting]\nweight = "respect"\n')
PY
cargo run -p sim-cli -- --config /tmp/m15-3.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config /tmp/m15-respect.toml --ticks 2 --llm mock --quiet
# hashes should match (no esteem; respect 0 ≡ equal)

cargo run -p sim-cli -- --config /tmp/m15-respect.toml \
  --incentives configs/incentives/esteem.toml --ticks 2 --llm mock --quiet
```

**Success:** first two hashes match; esteem run exits 0 (hash diverges because of `IncentiveApplied` + relationship writes). The unit tests in §2 are the proof the board Accepts; default 3-agent mock may not Propose on tick 1.

---

## 4. Viewer board (researcher)

Copy `[voting] weight = "respect"` into a local experiment TOML, then:

```bash
cargo run -p viewer -- --config /tmp/m15-respect.toml
```

Board (`B`): line `tally respect  need …`; open proposals show `yes=N (yes_w) no=M (no_w) need=…`. Equal mode (shipping `default.toml`) still shows head counts only.

---

## 5. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M15-plan.md`](M15-plan.md), [`incentive-schedule-format.md`](incentive-schedule-format.md) (`weight = "respect"`, `toward`), [`configs/incentives/esteem.toml`](../configs/incentives/esteem.toml), README current slice **M15**.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test governance respect_kingmaker` | Accepted, 14000 |
| 2 | `equal_with_esteem_still_open` | Open |
| 3 | `toward_nope_is_load_error` | error |
| 4 | §0 `cargo test -p sim-core` | all green |

---

## Execution record

**Ran:** 2026-08-26, repo root. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 16; governance 22; incentives 16 |
| §0 `cargo test -p viewer` | **pass** | 7/7 |
| §1 parse (`--exact`) | **pass** | `respect` parses; `maybe` / `toward=nope` error |
| §1 CLI `toward = "nope"` | **pass** | exit 1, `unsupported toward "nope" (use agent:N)` |
| §1 CLI `weight = "maybe"` | **pass** | exit 1, `unknown voting weight "maybe" (use equal, influence, or respect)` |
| §2 kingmaker unit tests | **pass** | equal Open; equal+esteem Open; respect+esteem Accepted 14000 |
| §2 respect-0 vs equal hash | **pass** | hashes match |
| §3 3-agent omit vs `weight=respect` 2 ticks | **pass** | both `final_hash=c702c487…` |
| §3 respect + esteem 2 ticks | **pass** | `final_tick=2` `final_hash=1e0eb473…` (same as equal+esteem; mock often does not Propose) |
| §4 viewer board | **not run** | no display automation |
| §5 `--help` | **pass** | `--incentives`, `--compare` listed |
