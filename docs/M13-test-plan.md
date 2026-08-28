# M13 test plan — see each new feature

Walkthrough for [`M13-plan.md`](M13-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (board accept vs open, `--compare`). Next slice: [`M21-plan.md`](M21-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

**Success for the slice:** omit / `equal` matches M4 one-agent-one-vote; `[voting] weight = "influence"` plus `leadership.toml` lets **one** boosted Support accept a 3-agent 50% board; default overlay omitted does not change equal-tally hashes. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p sim-cli --test ab
cargo test -p viewer
```

---

## 1. Parse `[voting]`

```bash
cargo test -p sim-core voting::tests::unknown_weight_errors -- --nocapture
cargo test -p sim-core voting::tests::omit_and_equal_parse -- --nocapture
cargo test -p sim-core voting::tests::config_toml_influence -- --nocapture
cargo test -p sim-core voting::tests::config_toml_unknown_errors -- --nocapture
```

| Test | Success |
|---|---|
| `weight = "maybe"` | load error |
| omit / `"equal"` | `VoteWeight::Equal` |
| `"influence"` | `VoteWeight::Influence` |

---

## 2. Kingmaker tally

3 agents, threshold 0.5, only agent 0 Supports (the author). Equal needs 2 heads. Influence + `delta = 70.0` on agent 0: weight 10000 vs 3000+3000, need 8000, **Accept**.

```bash
cargo test -p sim-core --test governance equal_tally_one_of_three_stays_open -- --nocapture
cargo test -p sim-core --test governance equal_with_kingmaker_still_open -- --nocapture
cargo test -p sim-core --test governance influence_kingmaker_one_support_accepts -- --nocapture
cargo test -p sim-core --test governance majority_accept_copies_adopted_rule -- --nocapture
cargo test -p sim-core --test governance equal_no_boost_same_seed_same_hash -- --nocapture
```

| Test | Success |
|---|---|
| Equal, 1 of 3 | stays **Open** |
| Equal + kingmaker boost | still **Open** (heads unchanged) |
| Influence + kingmaker | **Accepted**, adopted rule copied |
| M4 two-agent majority | still Accepts (author counts) |
| Same seed, equal, Wait chooser | hashes match |

---

## 3. Overlay on a config (optional see-it)

`[voting]` is read from the **experiment** TOML (same overlay file as `[storage]`), not from `--incentives`.

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text()
t = t.replace("count = 16", "count = 3", 1)
t += "\n[voting]\nweight = \"influence\"\n"
Path("/tmp/m13-inf.toml").write_text(t)
PY
cargo run -p sim-cli -- --config /tmp/m13-inf.toml \
  --incentives configs/incentives/leadership.toml \
  --ticks 2 --llm mock --quiet
```

**Success:** exit 0. The unit tests in §2 are the proof the board Accepts; default 3-agent mock may not Propose on tick 1.

Unknown weight:

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text() + '\n[voting]\nweight = "maybe"\n'
Path("/tmp/m13-bad.toml").write_text(t)
PY
cargo run -p sim-cli -- --config /tmp/m13-bad.toml --ticks 1 --llm mock --quiet
# expect non-zero exit mentioning unknown voting weight
```

---

## 4. Viewer board (researcher)

Copy `[voting] weight = "influence"` into a local experiment TOML, then:

```bash
cargo run -p viewer -- --config /tmp/m13-inf.toml
```

Board (`B`): line `tally influence  need …`; open proposals show `yes=N (yes_w) no=M (no_w) need=…`. Equal mode (shipping `default.toml`) still shows head counts only.

---

## 5. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M13-plan.md`](M13-plan.md), [`incentive-schedule-format.md`](incentive-schedule-format.md) (`[voting]` overlay), README current slice **M13**.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test governance influence_kingmaker` | Accepted |
| 2 | `equal_tally_one_of_three_stays_open` | Open |
| 3 | `voting::tests::unknown_weight_errors` | error |
| 4 | §0 `cargo test -p sim-core` | all green |

---

## Execution record

**Ran:** 2026-08-26, repo root. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` / sim-cli ab / viewer | **pass** | governance 19/19; ab 3/3 |
| §1 parse (named) | **pass** | `maybe` errors; omit/equal/influence parse |
| §2 kingmaker unit tests | **pass** | equal Open; equal+boost Open; influence Accepted |
| §3 overlay CLI 3 agents + leadership, 2 ticks | **pass** | `final_tick=2` `final_hash=36f28e7d…` |
| §3 unknown `weight = "maybe"` | **pass** | exit 1, `unknown voting weight "maybe"` |
| §4 viewer board | **not run** | no display automation |
| §5 `--help` | **pass** | `--incentives`, `--compare` listed |
