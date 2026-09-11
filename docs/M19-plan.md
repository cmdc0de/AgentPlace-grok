# M19 — SetCouncilTally meta-rule, `/set respect`, backpack + crate scale-by-fill

**Status:** implemented  
**Depends on:** M18 complete (`docs/M18-plan.md`, git tag `M18`, commit `938fe35`)  
**Walkthrough:** [`M19-test-plan.md`](M19-test-plan.md)  
**Specs:** `memory-goals-incentives-spec.md` §2 (council / meta-rules), `M18-plan.md` (`council_tally` overlay only), `M16-plan.md` (`/set` deferred respect), `M11-plan.md` (one Basket pack; crate mesh fixed size)

## Context

M18 overlay `council_tally` is frozen at load; `SetCouncil` rewrites the roster but not unanimous vs majority. `/set` cannot write respect edges. One worn pack (Basket); crate meshes are fixed size.

M19 does **not** rewrite postcard, add TLS, wire Give, or bump `PROTOCOL_VERSION`.

## Goal

A researcher can:

1. With `allow_meta_rules = true`, Propose **SetCouncilTally { unanimous | majority }**; when Accepted, that is the runtime council tally (last-wins). Overlay `council_tally` is the default until one is adopted.
2. Type `/set ID respect TOWARD N` in the in-process viewer to stage a respect A/B (display 0–100 → millipoints ×100; hash-sensitive; remote refuses).
3. Craft/hold a **Backpack** (second worn pack, larger than Basket) and see **crate meshes scale with fill**. Basket behavior unchanged when no Backpack.
4. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`. Shipping `default.toml` / `coop.toml` unchanged ⇒ default hashes unchanged.

## In scope

### A. Meta-rule `council_tally`

`[proposals] allow_meta_rules = true` in the **experiment** TOML (already a field; default false). Do **not** flip shipping `configs/default.toml`.

**New `StructuredRule` variant (append only — after `SetCouncil`):**

| Rule | Meaning when **Accepted** |
|---|---|
| `SetCouncilTally { unanimous \| majority }` | Runtime `council_tally`. Last-wins (`later tick_accepted` wins). Used whenever `effective_vote_accept()` is `Council`. Dormant if accept is majority/unanimous. |

- `allow_meta_rules = false` → Propose SetCouncilTally is **illegal** (Wait).
- Missing / unknown tally string at Propose / parse → **Wait**.
- `hash_rule` tag **9**, then `tally.as_str()` bytes. Old `.ckpt` loadable.
- LLM `parse_rule`: `set_council_tally` / `setcounciltally`; JSON field `tally` (`unanimous` \| `majority`). Unknown still Wait.
- `Simulation.meta_council_tally: Option<CouncilTally>` filled in `refresh_meta` (already called on ckpt load). `effective_council_tally()` = meta unwrap_or overlay `voting.council_tally`. Tick’s council arm uses **that**, not `self.voting.council_tally` raw.
- Overlay `[voting] council_tally` unchanged (M18 parse/errors).
- No new `VoteAccept` variant.

### B. `/set respect` (in-process)

Closed console, same family as `/set` / `/give`:

```
/set ID hunger|thirst|energy|influence N
/set ID respect TOWARD N
```

| Field | Args | Stored |
|---|---|---|
| hunger, thirst, energy, influence | `N` display 0–100 | millipoints `N * 100`, clamp 0..=10_000 (unchanged) |
| respect | `TOWARD` agent id, `N` display 0–100 | edge `ID → TOWARD`, millipoints `N * 100`, clamp 0..=10_000 (positive only) |

- Missing toward / unknown field / bad id → console error, no panic.
- Creates the relationship row if missing (`track_relationships` already on in default).
- **Hash-sensitive** (mutates agent social state).
- **Remote attach refuses** (no new `ControlVerb`).
- Help lists both `/set` forms.
- `/set 0 respect 50` (no toward) is an error.

### C. Second pack (Backpack) + crate scale-by-fill

**Backpack** (code, not overlay TOML):

| | Basket (M11, keep) | Backpack (new) |
|---|---|---|
| Recipe | 2 fiber | **4 fiber** |
| Item | `ItemId::Basket` | `ItemId::Backpack` (append) |
| Recipe enum | `Recipe::Basket` | `Recipe::Backpack` (**append** after `FishingRod`) |
| Pack caps | 8 slots, weight 25.0 | **12 slots, weight 40.0** |
| Worn mesh | existing satchel | slightly **larger / darker** satchel |

- **One worn pack per agent.** If the agent holds a Backpack, that pack is worn (larger caps). Else Basket as today. Extra Basket/Backpack in pockets is cargo, not a second worn slot.
- Pack / Unpack / haul multipliers unchanged (pack 0.1, pockets 0.4). Removing the last worn-pack item Unpack-folds like M11 Basket.
- LLM parse `craft` / `backpack`. `/give ID backpack 1` works (in-process).
- Mock 80-tick default need **not** craft a Backpack (same as Basket). Tests `/give` or execute Craft.
- `format_version` stays **2**. Append-only `ItemId` / `Recipe`. Old ckpts: no Backpack.

**Crate scale-by-fill** (viewer only, **not hashed**):

- Non-empty stockpile mesh scale = lerp **0.40 .. 1.00** by `max(slots_used/16, weight/80)`.
- Empty still despawns. Sync **updates scale** of existing markers (today only spawn/despawn).
- Satchel / backpack meshes do **not** scale by pack fill this slice.

Do **not** change shipping `configs/default.toml`.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M44** | [`M44-plan.md`](M44-plan.md) — WIS board range, CHA support/pair-bond, STR pocket weight |
| After M44 | protobuf/TLS |
| Not M19 | Browser; combat; CI Win/mac; extra LLM reflection/embeddings; `PROTOCOL_VERSION` bump |

## Key decisions

1. **SetCouncilTally** is append-only `StructuredRule` tag 9; last-wins; gated by `allow_meta_rules`. Dormant until accept is council.
2. Overlay `[voting] council_tally` stays M18; no new `VoteAccept` variant.
3. `/set respect` needs a **toward** id; positive display 0–100 only.
4. **One worn pack.** Backpack wins over Basket if both held.
5. Crate scale-by-fill is **viewer-only** (not hashed).
6. Do not change shipping `configs/default.toml` / `coop.toml`.
7. Mock CI. `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## Tests (M19 acceptance bar)

| Test | Asserts |
|---|---|
| `allow_meta_rules = false` | SetCouncilTally Propose → Wait |
| overlay unanimous, adopt SetCouncilTally majority, 3-person council, 2 Support | **Accepted** (after the adopting tick) |
| overlay majority, adopt SetCouncilTally unanimous, 2 of 3 Support | **Open** |
| unknown JSON tally | Wait |
| `/set 0 respect 1 50` | agent 0→1 respect 5000; hash changes |
| `/set 0 respect 50` | console error |
| remote `/set … respect` | refused |
| Craft/give Backpack | pack caps 12 / 40; Move cheaper than same weight in Basket |
| Basket only | still 8 / 25 (M11) |
| crate 1 item vs near-full | viewer scale **smaller** vs **larger** (unit test on fill→scale helper) |
| default.toml mock | hashes match pre-M19 |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: SetCouncilTally meta-rule

- **Files:** `board.rs` `StructuredRule` + `hash_rule` tag 9, `execute.rs` Propose gate, `llm.rs` parse_rule, `simulation.rs` `meta_council_tally` / `effective_council_tally`, `report.rs` label, governance tests

### PR 2: `/set respect`

- **Files:** `simulation.rs` setter, viewer `parse_command` / help / remote refuse, viewer tests

### PR 3: Backpack + crate scale-by-fill

- **Files:** `action.rs` `Recipe`, `agent.rs` `ItemId`, `execute.rs` craft, pack caps, observation legal Craft, viewer satchel vs backpack mesh + stockpile scale helper, this plan + `M19-test-plan.md` when implemented

## Config / CLI

No new `ControlVerb`. No new `ExperimentConfig` postcard fields. Overlay `[voting] council_tally` unchanged. `allow_meta_rules` already on ExperimentConfig (hashed when true).

```bash
cargo test -p sim-core
cargo test -p viewer
```

## Verification (when implemented)

Walkthrough: [`M19-test-plan.md`](M19-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
```

Expect: SetCouncilTally Propose waits unless `allow_meta_rules`; adopted majority-of-council lets 2-of-3 Accept; `/set 0 respect 1 50` writes 5000 milli; Backpack caps 12/40; crate scale helper 1-item &lt; full; default mock hashes match.

## Risks

- **Postcard enum append.** `SetCouncilTally` at the **end** of `StructuredRule`; `Recipe::Backpack` and `ItemId::Backpack` at the **end** of those enums. Do not reorder `CouncilTally`.
- **Dormant meta tally.** Applying SetCouncilTally while accept is not council is a no-op until accept is council (same as SetCouncil).
- **`/set` arity.** `respect` takes four tokens; do not treat `/set 0 respect 50` as toward=50.
- **Do not add `ControlVerb` or bump `PROTOCOL_VERSION`.**
