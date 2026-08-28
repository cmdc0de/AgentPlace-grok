# M16 test plan — see each new feature

Walkthrough for [`M16-plan.md`](M16-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (board accept vs open, `/set`). Next slice: [`M20-plan.md`](M20-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test governance`; library unit tests need `--lib voting::tests::…`.

**Success for the slice:** omit / `accept = "majority"` matches M4/M15; `unanimous` needs every living Support (any Oppose Rejects); `council = [0, 1]` is unanimous among those living ids; `/set 0 hunger 50` → 5000 milli (hash-sensitive, remote refuses); `public_board_always_visible = false` omits far open proposals from Observation and legal Support. Shipping `default.toml` unchanged. `format_version = 2`, `PROTOCOL_VERSION = 2`.

---

## 0. Safety net

No network.

```bash
cargo test -p sim-core
cargo test -p viewer
```

---

## 1. Parse `[voting] accept`

```bash
cargo test -p sim-core --lib voting::tests::omit_accept_is_majority -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::config_toml_unanimous -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::config_toml_council -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::council_without_list_errors -- --exact --nocapture
cargo test -p sim-core --lib voting::tests::unknown_accept_errors -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit | `VoteAccept::Majority` |
| `accept = "unanimous"` | parses |
| `accept = "council"` + `council = [0, 1]` | parses |
| `accept = "council"` no list | load error |
| `accept = "maybe"` | load error |

CLI:

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text() + '\n[voting]\naccept = "maybe"\n'
Path("/tmp/m16-bad-accept.toml").write_text(t)
t2 = Path("configs/default.toml").read_text() + '\n[voting]\naccept = "council"\n'
Path("/tmp/m16-empty-council.toml").write_text(t2)
PY
cargo run -p sim-cli -- --config /tmp/m16-bad-accept.toml --ticks 1 --llm mock --quiet
# expect non-zero: unknown voting accept "maybe"
cargo run -p sim-cli -- --config /tmp/m16-empty-council.toml --ticks 1 --llm mock --quiet
# expect non-zero: requires a non-empty council list
```

---

## 2. Unanimous / council tally

3 agents. Author 0 is already a supporter after Propose.

```bash
cargo test -p sim-core --test governance unanimous_one_of_three_stays_open -- --exact --nocapture
cargo test -p sim-core --test governance unanimous_all_support_accepts -- --exact --nocapture
cargo test -p sim-core --test governance unanimous_one_oppose_rejects -- --exact --nocapture
cargo test -p sim-core --test governance council_two_support_accepts_with_silent_third -- --exact --nocapture
cargo test -p sim-core --test governance council_one_oppose_rejects -- --exact --nocapture
cargo test -p sim-core --test governance equal_tally_one_of_three_stays_open -- --exact --nocapture
```

| Test | Success |
|---|---|
| Unanimous, 1 of 3 Support | **Open** |
| Unanimous, all 3 Support | **Accepted** |
| Unanimous, 1 Oppose | **Rejected** |
| Council `[0,1]`, both Support, 2 silent | **Accepted** |
| Council `[0,1]`, 1 Oppose | **Rejected** |
| Majority equal, 1 of 3 | still **Open** (M4/M15) |

---

## 3. `/set`

```bash
cargo test -p viewer commands::tests::parse_set -- --exact --nocapture
```

| Test | Success |
|---|---|
| `/set 0 hunger 50` | parse; apply → hunger 5000; hash changes |
| remote | “in-process only” |

---

## 4. Range-limited board + last action

```bash
cargo test -p sim-core --test governance fog_board_omits_far_author -- --exact --nocapture
cargo test -p sim-core --test governance identified_agent_has_last_action -- --exact --nocapture
cargo test -p sim-core --test governance propose_appears_on_board_and_in_every_observation -- --exact --nocapture
```

| Test | Success |
|---|---|
| `public_board_always_visible = false`, far author | observer 1 omits proposal 0; Support not legal |
| Author | still sees own proposal |
| Default true | every agent still sees the board (M4) |
| Identified neighbor after Wait | `last_action = "wait"` |

---

## 5. Overlay CLI (optional see-it)

`[voting]` is on the **experiment** TOML. 3-agent mock often does not Propose in 2 ticks — hashes match omit when no board motion.

```bash
python3 - <<'PY'
from pathlib import Path
t = Path("configs/default.toml").read_text().replace("count = 16", "count = 3", 1)
Path("/tmp/m16-3.toml").write_text(t)
Path("/tmp/m16-unanimous.toml").write_text(t + '\n[voting]\naccept = "unanimous"\n')
PY
cargo run -p sim-cli -- --config /tmp/m16-3.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config /tmp/m16-unanimous.toml --ticks 2 --llm mock --quiet
# hashes should match if nobody Proposes
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

**Success:** 3-agent omit vs unanimous hashes match when idle; default 16-agent run exits 0. Unit tests in §2 are the proof of Accept/Reject.

Viewer (not automated):

```bash
cargo run -p viewer -- --config /tmp/m16-unanimous.toml
# Board (B): accept unanimous
# /set 0 hunger 50
```

---

## 6. Docs / help

```bash
cargo run -p sim-cli -- --help
```

Skim: this file, [`M16-plan.md`](M16-plan.md), [`incentive-schedule-format.md`](incentive-schedule-format.md) (`accept` / `council`), README current slice **M16**.

---

## Suggested 5-minute mock-only order

| Min | Command | You should see |
|---|---|---|
| 1 | `cargo test -p sim-core --test governance unanimous_all_support_accepts` | Accepted |
| 2 | `council_two_support_accepts_with_silent_third` | Accepted |
| 3 | `fog_board_omits_far_author` | far omitted |
| 4 | `commands::tests::parse_set` | 5000 milli |
| 5 | §0 `cargo test -p sim-core` | all green |

---

## Execution record

**Ran:** 2026-08-27, repo root. Viewer GUI not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | lib 21; governance 29; incentives 16 |
| §0 `cargo test -p viewer` | **pass** | 8/8 including parse_set |
| §1 parse (`--exact`) | **pass** | majority omit; unanimous/council parse; maybe / empty council error |
| §1 CLI `accept = "maybe"` | **pass** | exit 1, `unknown voting accept "maybe"` |
| §1 CLI empty council | **pass** | exit 1, `requires a non-empty council list` |
| §2 unanimous/council unit tests | **pass** | Open / Accepted / Rejected as table |
| §3 `/set` | **pass** | hunger 5000; hash changes; remote refused |
| §4 fog + last_action | **pass** | far omitted; `last_action=wait` |
| §5 3-agent omit vs unanimous 2 ticks | **pass** | both `final_hash=c702c487…` |
| §5 default 2 ticks | **pass** | `final_hash=70e5204d…` |
| §5 viewer | **not run** | no display automation |
| §6 `--help` | **pass** | `--incentives`, `--compare` listed |
