# M13 — Influence-weighted votes (opt-in)

**Status:** implemented  
**Depends on:** M12 complete (`docs/M12-plan.md`, git tag `M12`, commit `ba50e22`)  
**Walkthrough:** [`M13-test-plan.md`](M13-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` §2 (acceptance: simple majority vs weighted-by-status), `M4-plan.md` (one-agent-one-vote), `M5-plan.md` (`influence_factor` does not weight votes today)

## Context

M4 accepts a proposal when `supporters.len() ≥ ceil(threshold × living_population)`. M5 added `influence_factor` (plasticity / leadership millipoints) and M8 can change it (`influence_factor_delta`), but **votes are still heads**. A researcher cannot run “does a high-influence agent pass a rule the crowd would not?”

M13 does **not** rewrite postcard, add TLS, timeline, `supporters_of:`, `/set`, or wire Give.

## Goal

A researcher can:

1. Leave tally as **M4 equal** (omit overlay) so default hashes and `majority_accept` stay the same.
2. Set overlay `[voting] weight = "influence"` and have each living agent contribute `max(influence_factor, 1)` millipoints. Accept/reject use **total living weight** (abstainers count).
3. Combine with a **leadership** incentive on `agent:0` and see a 3-agent board **Accept** under influence where equal stays **Open**.
4. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`.

## In scope

### A. Overlay `[voting]`

Not `ExperimentConfig` (no postcard `config_hash` change). Same OverlayFile as `[storage]`:

```toml
[voting]
weight = "equal"       # default; omit = equal
# weight = "influence"
```

Unknown `weight` is a **load error**. Do **not** add `[voting]` to shipping `configs/default.toml`.

`VotingParams` lives on `Simulation` like `StorageParams`.

### B. Tally

| Mode | Weight per living agent | `need` |
|---|---|---|
| `equal` | 1 | `ceil(threshold × N)` |
| `influence` | `max(influence_factor, 1)` milli | `ceil(threshold × sum(weights))` |

`yes` / `no` sum the weights of supporters / opposers. Accept if `yes ≥ need`; reject if `no ≥ need`. Same lifetime/expiry as M4.

- Proposal still `BTreeSet<AgentId>` (no f32 on the board).
- One stance per agent, last-wins, unchanged.
- Mock who-votes unchanged (toxin Propose/Support). Weighting is the **tally**.
- Uniform `base_influence_factor` (3000 milli) ⇒ influence ≡ equal. Weighting bites when influence **diverges**.
- M8 sample `delta = 0.15` (+15 milli) will **not** swing a 16-agent board. The example uses a **large** delta.

### C. Example files (do not change `coop.toml`)

`configs/incentives/leadership.toml`:

```toml
[[incentives]]
id = "kingmaker"
description = "boost agent 0 influence"
applies_to = "agent:0"
[[incentives.effects]]
type = "influence_factor_delta"
delta = 70.0          # +7000 milli → clamp 10000
```

A/B on a tiny 3-agent map, threshold 0.5, only agent 0 Supports: **equal → Open**; **influence + kingmaker → Accepted**.

### D. Viewer / compare

Board UI in influence mode: `yes_w / need` plus head counts. `--compare` still uses board counts; **hash differs** when a proposal would accept under only one mode. Observation/prompt still show supporter **counts**.

## Out of scope (later)

| Later | What |
|---|---|
| **M14** | Done — [`M14-plan.md`](M14-plan.md) — coalition targeting + checkpoint scrubber |
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
| **M16** | Done — [`M16-plan.md`](M16-plan.md) — council/unanimous, `/set`, range-limited board |
| **M17** | Done — [`M17-plan.md`](M17-plan.md) — meta-rules, join/leave one-shots, event JSONL timeline |
| **M18** | Done — [`M18-plan.md`](M18-plan.md) — weighted council, SetCouncil meta-rule, jump-to-tick catch-up |
| **M19** | Done — [`M19-plan.md`](M19-plan.md) — SetCouncilTally, `/set respect`, backpack + crate scale-by-fill |
| **M20** | Done — [`M20-plan.md`](M20-plan.md) — revert relationship_delta on leave, stacked worn packs, attach safety net |
| **M21** | Done — [`M21-plan.md`](M21-plan.md) — pack fill scale, sim-cli --connect, jump-to-tick on the wire |
| **M22** | [`M22-plan.md`](M22-plan.md) — wire Give, remote /ckpt and /events |
| **M23** | [`M23-plan.md`](M23-plan.md) — see every tick, LLM pipeline barrier, sim-cli Control |
| After M23 | protobuf/TLS |
| Not M13 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. Overlay `[voting] weight = "equal"|"influence"`, default equal.
2. Influence weight = `max(influence_factor, 1)` millipoints.
3. Majority of **living population total weight**, not of those who voted.
4. Keep `BTreeSet` stances; no f32 on proposals.
5. Do not change `configs/default.toml` or `coop.toml`.
6. Example uses a **large** `influence_factor_delta`.
7. Respect weights / `supporters_of` / protobuf / TLS / timeline / `/set` / wire Give → later.
8. Mock CI. No new `ControlVerb`.

## Tests (M13 acceptance bar)

| Test | Asserts |
|---|---|
| Omit / `equal` | same accept as M4 `majority_accept` |
| 3 agents, threshold 0.5, agent 0 boosted, only 0 Supports | **equal:** Open; **influence:** Accepted |
| Same seed, equal, no boost | hash matches today’s tally |
| Unknown `weight` | load error |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Tally + overlay

- **Files:** `board.rs`, `simulation.rs`, `overlay.rs`, tests
- **Changes:** `VotingParams`; weighted `tick_lifecycle`; parse `[voting]`; 3-agent kingmaker test.

### PR 2: Board UI + example overlay

- **Files:** viewer `ui.rs`, `configs/incentives/leadership.toml`
- **Changes:** `yes_w / need` when influence mode.

### PR 3: README + docs

- **Files:** README, this plan status when implemented

## Config / CLI

No new `ExperimentConfig` postcard fields. No new CLI flag: `[voting]` on the experiment TOML overlay (like `[storage]`).

```bash
cargo test -p sim-core
cargo test -p sim-cli --test ab
# when implemented: 3-agent kingmaker unit test is the proof
```

## Verification (when implemented)

Walkthrough: [`M13-test-plan.md`](M13-test-plan.md).

```bash
cargo test -p sim-core
```

Expect: equal mode matches M4; influence + kingmaker accepts with one high-weight Support; default overlay omitted ⇒ same hashes as M12 for equal-tally runs.

## Risks

- **Tiny `influence_factor_delta` looks like a no-op.** Document millipoints; example uses `70.0`.
- **f64 `ceil(threshold × weight)`** already exists for heads; keep the same pattern, do not store f32 on proposals.
- **Do not add `ControlVerb` or postcard voting fields.**
