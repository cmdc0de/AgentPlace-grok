# M14 — Coalition targeting + checkpoint scrubber

**Status:** implemented  
**Depends on:** M13 complete (`docs/M13-plan.md`, git tag `M13`, commit `08bd7ad`)  
**Walkthrough:** [`M14-test-plan.md`](M14-test-plan.md)  
**Specs:** `incentive-schedule-format.md` (`applies_to`), `memory-goals-incentives-spec.md` §3, `medium-priority-specs.md` (checkpoints), `M6-plan.md` / `M7-plan.md` (no jump-to-tick without a file)

## Context

M8 incentives can target `all`, `agent:N`, or `archetype:name`. The spec’s `supporters_of:proposal_N` is still a **load error**, so a researcher cannot reward the coalition that backed a rule. The viewer `--load`s **one** `.ckpt`; there is no scrubber over a run directory.

M14 does **not** rewrite postcard, add TLS, `/set`, wire Give, or invent per-tick snapshots.

## Goal

A researcher can:

1. Set `applies_to = "supporters_of:proposal_N"` so **current supporters** of that proposal get per-tick incentive effects (e.g. 1.4× food). An opponent does not.
2. `--load` a **checkpoint directory**, move a **tick slider** (or prev/next) to the nearest `.ckpt` **at or before** that tick, and **replace** the in-process sim with that file (same as today’s single-file `--load`). Play continues from there.
3. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## In scope

### A. `supporters_of:proposal_N`

```toml
applies_to = "supporters_of:proposal_3"   # proposal id 3
# also accepted: "supporters_of:3"
```

- **Who:** agents in that proposal’s current `supporters` set (Open / Accepted / Rejected / Expired — the set remains). Missing id → empty scope (nobody). `supporters_of:nope` → **load error**.
- **Per-tick effects** (`resource_multiplier`, `memory_importance_boost`, `proposal_threshold_modifier`): `in_scope` is evaluated when used. Late joiners get the payoff.
- **One-shots** (`goal_injection`, `relationship_delta`, `influence_factor_delta`): whoever is a supporter **when the incentive starts** (M8). Late joiners do **not** get the one-shot. No join/leave tracking this slice.

Example (new; do not change `coop.toml` / `leadership.toml`):

```toml
# configs/incentives/coalition.toml
[[incentives]]
id = "coalition_food"
applies_to = "supporters_of:proposal_0"
[[incentives.effects]]
type = "resource_multiplier"
resource = "food"
multiplier = 1.4
```

### B. Viewer checkpoint scrubber

M2 writes `{id}_tick_{T}.ckpt`. **M14:** `--load DIR` lists those files. Imgui slider / `[` `]` (or equivalent) loads the ckpt at or before the requested tick.

- **No interpolation**, no extra recording, no jump without a file.
- Load **replaces** the in-process sim. Play from that tick is hash-sensitive (like `--load` a file).
- **In-process only.** Remote attach stays read-only; scrub is not on the wire.
- Not an events-JSONL timeline.

Helper (sim-core, no Bevy): list ticks in a dir; `ckpt_at_or_before(dir, tick) -> Path`.

## Out of scope (later)

| Later | What |
|---|---|
| **M15** | Done — [`M15-plan.md`](M15-plan.md) — opt-in respect-weighted votes |
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
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done — [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| After M33 | protobuf/TLS |
| Not M14 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. `applies_to = "supporters_of:proposal_N"` (and `supporters_of:N`).
2. Dynamic `in_scope` for per-tick effects; one-shots at start only.
3. Scrubber = existing `.ckpt` files only; no interpolation.
4. Scrub in-process only; no new `ControlVerb`.
5. Do not change `configs/default.toml` or `coop.toml`.
6. Respect weights → M15. Board fog / protobuf / TLS / `/set` / wire Give → later.
7. Mock CI. `PROTOCOL_VERSION = 2`.

## Tests (M14 acceptance bar)

| Test | Asserts |
|---|---|
| `supporters_of:proposal_0` | supporter in scope; non-supporter not |
| Missing proposal id | empty scope, no crash |
| `supporters_of:nope` | load error |
| Same seed twice | same hash |
| Ckpt helper | dir list; tick → path at or before |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: `in_scope` + validate

- **Files:** `incentive.rs`, tests
- **Changes:** parse `supporters_of:`; coalition food-multiplier test.

### PR 2: Viewer scrubber

- **Files:** `checkpoint.rs` helper, viewer `main.rs` / `ui.rs` / `commands.rs`
- **Changes:** `--load DIR`; slider / prev-next ckpt; in-process only.

### PR 3: Example + docs

- **Files:** `configs/incentives/coalition.toml`, README, this plan status when implemented

## Config / CLI

No new `ExperimentConfig` postcard fields. No new `ControlVerb`.

```bash
cargo test -p sim-core
cargo test -p viewer
# --load a directory of ckpts in the viewer (in-process)
```

## Verification (when implemented)

Walkthrough: [`M14-test-plan.md`](M14-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: supporter in scope, opponent not; load error on `supporters_of:nope`; helper picks the right ckpt; default runs without a coalition schedule still hash-match.

## Risks

- **One-shots vs late joiners.** Document: `goal_injection` at start only. Per-tick payoffs do follow the set.
- **Scrub is `--load`.** Playing after a scrub continues from that tick; it is not a ghost preview.
- **Do not add `ControlVerb` or per-tick checkpoints.**
