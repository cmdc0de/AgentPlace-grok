# M11 — Mock fills the crate + worn backpacks

**Status:** implemented  
**Depends on:** M10 complete (`docs/M10-plan.md`, git tag `M10`, commit `7a17759`)  
**Walkthrough:** [`M11-test-plan.md`](M11-test-plan.md)  
**Specs:** `decision-observation-llm-economy-metrics-spec.md` §4 (Store/Transfer), `needs-and-survival.md`, `incentive-schedule-format.md` (coop storage goal)

## Context

M10 added land-cell crates (one per cell, slot 16 + weight 80), haul energy on Store/Retrieve/Transfer, and a brown crate mesh. The M10 walkthrough still showed **store=0** on default 80-tick (and 250-tick) coop mock: the policy only Stores when hunger ≥ 75% **and** food is already in inventory, but Gather only runs when hungry (&lt; 75%), then Eat. Surplus and inventory never overlap.

`visibility_modifier` and vote weighting stay later (**M12+**). This slice is **economy that the coop goal can actually use**, plus a **portable pack** so hauling cargo is not the same as stuffing pockets.

M11 does **not** rewrite postcard, add TLS, timeline, `/set`, or wire Give.

## Goal

A researcher can:

1. Run `configs/default.toml` + `configs/incentives/coop.toml` + `--ticks 80 --llm mock` and see **Store events** and **stockpile qty &gt; 0** on `--compare` vs baseline (baseline stays empty crates).
2. Craft/hold a **Basket** and get a **worn backpack** (8 slots, weight 25). Packing cargo makes **Move cheaper** than carrying the same weight loose.
3. See a **satchel on the agent capsule** whenever they have a Basket (even if the pack is empty), distinct from the ground crate. Fog = agent visible in Observation.
4. Same-seed mock hashes still match. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`.

## In scope

### A. Mock storage policy

After thirst, before random walk:

| Condition | Action |
|---|---|
| hunger &lt; 50% | Eat / Retrieve / Gather (survival) |
| storage **goal** + food in pockets **or pack** + legal cell Store | **Store to the ground crate** (shared goal) |
| storage goal + legal Gather + inventory/pack room | **Gather** (stock the crate before hungry) |
| food in pockets + legal Pack + has Basket | **Pack** (hands light before Move) |
| hunger &lt; 75% | existing hungry Gather/Hunt |
| agreeableness ≥ 40 + leftover food + Transfer | Transfer 1 |

Eat-all in the 50–75% band is **off** when a storage goal is active.

### B. Ground crates (M10 rules, keep and test)

- **One crate per land cell.** A second Store fills *that* crate or is illegal if full — never two markers.
- **Dual caps:** 16 slots, weight 80.0. Over cap → not legal.
- Empty crate despawns. Brown cube mesh unchanged.

### C. Worn backpack (Basket)

- The existing **Basket** (craft, 2 fiber) **is** the backpack. No new recipe. **One pack per agent.**
- Pack caps: **8 slots, weight 25.0** (smaller than the shared crate so coop storage still wins).
- Basket in pockets ⇔ pack exists. Removing the last Basket **Unpacks all into pockets**; overflow **Store to the current land cell** if it fits; if still overflow, **refuse** Transfer/Give of that last Basket.
- **Pack / Unpack** (qty 1): pockets ↔ pack. Haul uses pack multiplier **0.1**.
- **Move** costs cargo:  
  `loose_weight × 0.4 × step_k + pack_weight × 0.1 × step_k`  
  with `step_k` ≈ 0.05 display per cell. Unaffordable → Move not legal. Rest still regenerates.
- Store / Retrieve / Transfer pay the haul of the **source** (pack cheaper than pockets).

### D. Viewer

- Satchel: small cube offset on the capsule, `srgb(0.35, 0.22, 0.12)` — **not** the ground-crate brown.
- Visible whenever the agent **has a Basket**, empty pack included.
- Inspector: pack slots/weight + contents, separate from pockets.
- Fog: satchel hides with the agent under POV fog.

## Out of scope (later)

| Later | What |
|---|---|
| **M12** | Done — [`M12-plan.md`](M12-plan.md) — public vs hidden incentives |
| **M13** | Done — [`M13-plan.md`](M13-plan.md) — opt-in influence-weighted votes |
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
| **M24** | [`M24-plan.md`](M24-plan.md) — wire /set, connect /inject, lockstep ack |
| **M25** | Done — [`M25-plan.md`](M25-plan.md) — reflection-on-evict, lockstep Ack timeout |
| **M26** | Done — [`M26-plan.md`](M26-plan.md) — Reflect/Plan every-N-ticks, record/replay of reflection text |
| **M27** | Done — [`M27-plan.md`](M27-plan.md) — auto-execute plan, combat |
| **M28** | Done — [`M28-plan.md`](M28-plan.md) — health/incapacitation, combat viewer FX, force_reflect |
| **M29** | Done — [`M29-plan.md`](M29-plan.md) — local embeddings, combat death, CI Win/mac |
| **M30** | Done — [`M30-plan.md`](M30-plan.md) — combat particles / meshes |
| **M31** | Done — [`M31-plan.md`](M31-plan.md) — kinship, reproduction, D&D-like sheet |
| **M32** | Done — [`M32-plan.md`](M32-plan.md) — kin_of incentives, household, aging |
| **M33** | Done — [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| **M34** | Done — [`M34-plan.md`](M34-plan.md) — sheet effects, close-kin PairBond |
| **M35** | Done — [`M35-plan.md`](M35-plan.md) — inventions, browser attach |
| **M36** | Done — [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| **M37** | Done — [`M37-plan.md`](M37-plan.md) — extra invention kinds, browser /set /give |
| **M38** | Done — [`M38-plan.md`](M38-plan.md) — viewer 3D models, browser /inject /scrub + wasm32 CI |
| **M39** | Done — [`M39-plan.md`](M39-plan.md) — object definition files (visual + LOD + hashed catalog) |
| **M40** | Done — [`M40-plan.md`](M40-plan.md) — config-owned objects + recipes (except agent) |
| **M41** | Done — [`M41-plan.md`](M41-plan.md) — world species from TOML, DEX accuracy, STR haul |
| **M42** | Done — [`M42-plan.md`](M42-plan.md) — CON illness/energy, INT memory, browser /ckpt /events |
| **M43** | Done — [`M43-plan.md`](M43-plan.md) — WIS toxin detect, CHA speech, DEX flee |
| **M44** | Done — [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| **M45** | Done — [`M45-plan.md`](M45-plan.md) — tech tree/patents, hashed pipeline events, string ItemId ckpt bump |
| **M46** | Done — [`M46-plan.md`](M46-plan.md) — CON illness chance, INT plan length, STR gather/hunt, extra recipes, telemetry |
| **M47** | Done — [`M47-plan.md`](M47-plan.md) — viewer camera pan, missing-asset sentinel, time-series charts |
| **M48** | [`M48-plan.md`](M48-plan.md) — INT invent quality, agent meshes, hot-reload glb |
| After M48 | Protobuf/TLS |
| Not M11 | Browser; combat; CI Win/mac; extra LLM calls; `PROTOCOL_VERSION` bump |

## Key decisions

1. **Mock Gather-then-Store** under the storage goal so 80-tick default A/B fills crates.
2. **Eat only when hunger &lt; 50%** if a storage goal is active.
3. **Cell crate still 1 per land cell, 16 / 80.**
4. **Basket is the backpack;** one pack per agent; 8 / 25.
5. **Move costs cargo haul;** pack 0.1 vs loose 0.4.
6. **Shared crate is the coop target** (mock Stores to the **cell**, not only the pack).
7. **Satchel visible whenever Basket is held.**
8. **`visibility_modifier` → M12.**
9. Mock CI default. No new `ControlVerb`.

## Tests (M11 acceptance bar)

| Test | Asserts |
|---|---|
| 80-tick default + coop | ≥1 Store; stockpile qty &gt; 0 on `--compare` |
| Two Stores same cell | one crate; qty adds until cap |
| Over slot/weight | illegal |
| Basket ⇒ pack; Pack then Move | Move energy cost &lt; same weight loose |
| No Basket, same loose load | higher Move cost |
| Removing last Basket | pack contents fold into pockets/cell |
| Satchel helper | Basket present ⇒ backpack marker; none ⇒ no satchel |
| Same seed twice | same hash |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Mock Gather/Store policy

- **Files:** `policy.rs`, tests (tiny + default-config 80-tick or equivalent)
- **Changes:** storage-goal Gather/Store; Eat only below 50% when that goal is on.

### PR 2: Agent pack + Pack/Unpack + Move haul

- **Files:** `agent.rs`, `action.rs`, `haul.rs`, `execute.rs`, `observation.rs`, `event_log.rs`
- **Changes:** pack container; legal Pack/Unpack; Move energy; Basket removal fold-in.

### PR 3: Viewer satchel + inspector

- **Files:** `markers.rs`, viewer `main.rs` / `ui.rs`
- **Changes:** satchel mesh on agents with Basket; inspector pack bars.

### PR 4: README + docs

- **Files:** README, this plan status when implemented, `needs-and-survival.md`

## Config / CLI

No new `ExperimentConfig` postcard fields. Pack/crate caps are constants or the existing storage overlay.

```bash
cargo test -p sim-core
cargo test -p sim-cli --test ab
cargo test -p viewer
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml --ticks 80 --llm mock \
  --out-dir /tmp/m11-coop --quiet
# grep '"type":"store"'  ≥ 1
```

## Verification (when implemented)

Walkthrough: [`M11-test-plan.md`](M11-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo run -p sim-cli -- --compare /tmp/m11-base /tmp/m11-coop
```

Expect stockpile qty **B &gt; A** on mock coop vs baseline (80-tick default: qty 0 vs 75).

## Risks

- **Move haul changes default hashes** for every run (like M9 drink cutoff). Same-seed test required.
- **Mock that Gathers from tick 1** may eat more vegetation than M10; diet in `--compare` will move — that is intended.
- **Basket removal overflow** must not duplicate items or void them.
- Do not add `ControlVerb` or `visibility_modifier` in this slice.
