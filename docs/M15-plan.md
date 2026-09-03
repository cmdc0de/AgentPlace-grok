# M15 — Respect-weighted votes (opt-in)

**Status:** implemented  
**Depends on:** M14 complete (`docs/M14-plan.md`, git tag `M14`, commit `23e8058`)  
**Walkthrough:** [`M15-test-plan.md`](M15-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` §2 (acceptance: simple majority vs weighted-by-status), `M13-plan.md` (`equal` | `influence`), `M5-plan.md` (`RelationshipSummary.respect` exists, unused in tally)

## Context

M13 tally is `equal` | `influence`. The spec’s other status axis — **relationship respect** — is stored on every `RelationshipSummary` (signed millipoints, default 0) but never enters `vote_weight_of`. A researcher cannot run “does a high-prestige agent pass a rule the crowd would not?”

M15 does **not** rewrite postcard, add TLS, `/set`, wire Give, council/unanimous, or board fog.

## Goal

A researcher can:

1. Leave tally as **M4/M13 equal** (omit overlay) so default hashes stay the same.
2. Set overlay `[voting] weight = "respect"` so each living agent contributes **incoming prestige** (formula below). Accept/reject use **total living weight** (abstainers count), same as M13.
3. Combine with an **esteem** incentive (`relationship_delta` respect toward `agent:0`) and see a 3-agent board **Accept** under respect where equal stays **Open**.
4. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`.

## In scope

### A. Overlay `[voting]`

Same overlay as M13 (not `ExperimentConfig`, no postcard `config_hash` change):

```toml
[voting]
weight = "equal"       # default; omit = equal
# weight = "influence"
# weight = "respect"
```

Unknown `weight` is a **load error**. Do **not** add `[voting]` to shipping `configs/default.toml`.

`VoteWeight::Respect` on `VotingParams` (already on `Simulation`).

### B. Tally

**Incoming prestige:** for living agent `id`, sum `max(other.relationships[id].respect, 0)` over every **other living** agent. Missing row = 0.

| Mode | Weight per living agent | `need` |
|---|---|---|
| `equal` | 1 | `ceil(threshold × N)` |
| `influence` | `max(influence_factor, 1)` milli | `ceil(threshold × sum(weights))` |
| `respect` | `max(incoming_positive_respect_sum, 1)` | `ceil(threshold × sum(weights))` |

- Default respect 0 ⇒ every weight is 1 ⇒ **identical to equal** (hashes match a no-overlay run if the schedule does not change relationships).
- Negative respect does **not** subtract (per-edge clamp to 0, then `max(1, sum)`).
- Dead agents neither vote nor contribute incoming prestige.
- Proposal still `BTreeSet<AgentId>` (no f32 on the board). Last-wins Support/Oppose unchanged.
- Observation/prompt still show supporter **counts** (same as M13).

Kingmaker numbers (3 living, threshold 0.5, only agent 0 Supports; agents 1 and 2 each have respect +7000 toward 0):

- equal: yes=1, need=2 → **Open**
- respect: weights 14000 / 1 / 1, total 14002, need=7001, yes=14000 → **Accepted**

### C. `relationship_delta.respect` + optional `toward`

Today the effect only has `trust` / `affinity`, applied once at start from each in-scope agent to **every other** agent. Status kingmaker needs **directed** incoming respect.

```toml
# configs/incentives/esteem.toml  (new; do not change leadership.toml / coop.toml)
[[incentives]]
id = "esteem_0"
description = "everyone respects agent 0"
applies_to = "all"
[[incentives.effects]]
type = "relationship_delta"
respect = 70.0          # +7000 millipoints
toward = "agent:0"      # only this target; skip self
```

| Field | Meaning |
|---|---|
| `respect` | Optional, same scale as `trust` (`70.0` → +7000, clamp `REL_MIN`/`REL_MAX`). Default 0. |
| `toward` | Optional. Omit = today’s vs-every-other (now including `respect` if set). `toward = "agent:N"` = only that id. `toward = "nope"` → **load error**. Missing agent at start → no-op for that edge. |

One-shot at incentive start (M8), not live join/leave.

Equal + this schedule still **Open** (heads unchanged). Respect + this schedule **Accepts**.

### D. Viewer / compare

Board UI: in `respect` mode show `yes_w / need` plus head counts (same as influence). `--compare` still uses board counts; **hash differs** when a proposal would accept under only one mode.

## Out of scope (later)

| Later | What |
|---|---|
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done — [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done — [`M20-plan.md`](M20-plan.md) — revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done — [`M21-plan.md`](M21-plan.md) — pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| **M23** | [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| After M28 | protobuf/TLS |
| Not M15 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. `[voting] weight = "respect"` on the existing overlay (third `VoteWeight`).
2. Weight = incoming positive respect from other **living** agents; floor 1.
3. `relationship_delta` gains `respect` and optional `toward = "agent:N"`.
4. Do not change `configs/default.toml`, `coop.toml`, or `leadership.toml`.
5. `/set` / council / board fog → M16. Protobuf / TLS / wire Give → later.
6. Mock CI. `PROTOCOL_VERSION = 2`.

## Tests (M15 acceptance bar)

| Test | Asserts |
|---|---|
| `weight = "respect"` | parses; unknown still errors |
| omit / `"equal"` | still `VoteWeight::Equal` |
| 3 agents, 1 Support, equal | **Open** |
| equal + esteem.toml | still **Open** |
| respect + esteem.toml | **Accepted**, adopted rule copied |
| Default respect (all 0), `weight = respect`, Wait | same hash as equal (no schedule) |
| `toward = "nope"` | load error |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: `VoteWeight::Respect` + tally

- **Files:** `voting.rs`, `simulation.rs` `vote_weight_of`, governance tests
- **Changes:** parse `respect`; incoming-prestige weights; kingmaker Accept vs equal Open.

### PR 2: `relationship_delta` respect + `toward`

- **Files:** `incentive.rs`, `configs/incentives/esteem.toml`
- **Changes:** optional `respect` / `toward`; load error on `toward = "nope"`.

### PR 3: Viewer board + docs

- **Files:** viewer board panel, README, this plan status when implemented, `incentive-schedule-format.md` (`weight = "respect"`, `toward`)

## Config / CLI

No new `ExperimentConfig` postcard fields. No new `ControlVerb`.

```bash
cargo test -p sim-core
cargo test -p viewer
```

## Verification (when implemented)

Walkthrough: [`M15-test-plan.md`](M15-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: `weight = "respect"` parses; esteem + respect Accepts a 3-agent 50% board that equal leaves Open; default respect-0 + Wait hashes match equal; `toward = "nope"` is a load error.

## Risks

- **Default 0 is equal.** The mode only bites after relationships (or `esteem.toml`) diverge. Document that.
- **Directed `toward`.** Without it, `applies_to = "all"` + `respect` would boost everyone equally and not kingmake.
- **Do not add `ControlVerb` or bump `PROTOCOL_VERSION`.**
