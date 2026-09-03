# Post-GA feature list

Researcher-facing features **after** the current GA line (M1–M28 shipped: world, agents, LLM loop, governance, attach, combat v2-lite). **Not** a milestone plan. **Not** `missing-features.md` (in-use bugs). **Not** the After-M later-table (protobuf/TLS, embeddings, Unix sockets).

When a theme is scheduled, `/spec` picks 1–3 items into `docs/M{N}-plan.md`. Until then this file is the backlog. Milestone plans still win on timing vs this list and vs long-term specs.

Standing unless a later plan picks a bump: CI `provider = mock`; `format_version = 2`; `PROTOCOL_VERSION = 5`; shipping `configs/default.toml` / `coop.toml` unchanged; overlay TOML is not `ExperimentConfig` postcard.

---

## Themes

| ID | Theme | Status | One-liner |
|---|---|---|---|
| PG-1 | Kinship / family relations | Open | Parent, child, sibling, pair-bond as first-class links, distinct from dyadic trust/respect. |
| PG-2 | Children / reproduction | Open | Agents can have children; new agents spawn mid-run with derived identity. |
| PG-3 | Physical sheet (D&D-like) | Open | Strength/Dex/Con/Int/Wis/Cha (plus a few derived stats). Founders rolled; children **calculated** from parents. |

Add a row when something is a post-GA experiment. When a milestone ships it, mark **Done** and point at that plan.

---

## PG-1 — Kinship / family relations

**Shipped today (M5):** each agent has a `RelationshipSummary` per known other: trust, affinity, respect, fear, counts. That is **how they feel**, not **who they are to each other**.

**Wanted:** a kinship graph that survives memory eviction and is inspectable:

| Link | Meaning |
|---|---|
| parent / child | Directed; set at birth. |
| sibling | Derived from shared parents (or recorded). |
| pair-bond / mate | Optional, opt-in; can be a precondition for PG-2. |
| household | Optional group id for co-residence / shared stores. |

Effects (when scheduled): Observation/prompt can name “parent #3”, “child #12”; votes/incentives can `applies_to = "kin_of:N"`; viewer inspector shows a family pane. Overlay off ⇒ no extra links, **same hashes** as today.

Do not overload `RelationshipSummary` with kinship. Append-only records (or a `Kinship` map packed like goals in the board blob). Hash only when non-empty.

---

## PG-2 — Children / reproduction

**Shipped today:** population is spawned at `Simulation::new` and only shrinks (death / incapacitation). No `Reproduce` action.

**Wanted:** an opt-in overlay so two (or more) living agents can produce a child:

```toml
[population]
reproduction = true          # default false
# later: gestation_ticks, min_energy, max_living, mate_range, …
```

CLI: `--reproduction` (implies overlay on).

Mechanics (sketch for a later `/spec`):

- Legal only when overlay **on**, both agents living, not incapacitated, in range, optional pair-bond (PG-1).
- New `AgentId` = next id (append). Spawn on a land cell next to a parent.
- Child gets **calculated** physicals (PG-3), personality/abilities mixed from parents + a child RNG stream (`agent_init` / `tick_{t}_birth_{id}`).
- Kinship links written both ways (PG-1).
- Hashed: new agent body, kinship, any `Born` event (append `SimEventKind`). Overlay **off** ⇒ action illegal, no births, **same hashes**.
- Mock never chooses Reproduce unless a later plan says so (same pattern as Attack).

Out of this theme until picked: aging, childhood stages, culture inheritance, incest rules beyond “not self”.

---

## PG-3 — Physical sheet (D&D-like)

**Shipped today:** Big-Five + `perceptiveness` + `traits` + ability scores (`gather` / `hunt` / `fish` / `farm` / `craft`) + millipoint needs + `health` (M28, default 10_000). No STR/DEX/CON sheet.

**Wanted:** a compact **character sheet** on every agent, used by combat, haul, perception, and prompts.

### Ability scores

Classic six, stored as integers (e.g. 3–18, millipoint-friendly 300–1800, or 0–100 like Big-Five — lock in `/spec`). Default **modifier** table like D&D (score 10–11 → +0).

| Score | Typical sim use |
|---|---|
| Strength | Haul / inventory cap, melee damage, gather/hunt payoff |
| Dexterity | Move cost, Flee, ranged (if any), craft finesse |
| Constitution | `HEALTH_MAX`, energy max, illness resistance |
| Intelligence | Plan quality, memory capacity / retrieval_k (soft) |
| Wisdom | `perceptiveness`, detect toxin, board range |
| Charisma | Speech, proposal support, influence, pair-bond |

Optional extras (same sheet, later rows): size category, sex/presentation, speed, vision, “hit dice” leftover as flavor in Markdown summaries.

### Founders vs children

| Who | How scores are set |
|---|---|
| **Initial population** | **Random** from a seeded stream (`agent_init`), optionally clamped by `[agents.archetypes]` min/max / 4d6-drop-lowest / 3d6. Same master seed ⇒ same rolls. |
| **Children (PG-2)** | **Calculated** from the two parents: e.g. per score `clamp(round((p1+p2)/2) + noise, min, max)` where `noise` is a small integer from the **child** stream (not re-rolling the whole sheet). Personality / allergies / ability skills mix the same way. |

Do **not** HTTP-roll. No network. Overlay `[agents.sheet] enabled = false` default so mock hashes stay `70e5204d…` until a plan turns it on. When off, derived modifiers are 0 / current constants.

Hash: only when the sheet is enabled **or** a score ≠ the “uninitialized / unused” default. Prefer skip + BoardBlob like `plan` / `health` so old ckpts load.

Viewer: inspector “Sheet” pane (STR 14 (+2) …). 3D capsule scale from size/CON optional, hash-neutral.

---

## How these three interact

```
Founders: roll sheet (PG-3) ──► live, relate (PG-1 feelings already shipped)
                              ──► optional pair-bond (PG-1)
                              ──► Reproduce (PG-2)
                                    ├── child sheet calculated (PG-3)
                                    └── kinship parent/child/sibling (PG-1)
```

Ship PG-3 before or with PG-2 so a birth has something to calculate. PG-1 kinship can land with PG-2 (links at birth) or slightly earlier (data model only).

---

## Constraints for every later `/spec`

- Overlay / CLI, not `ExperimentConfig`, unless the slice **is** a hashed config change.
- Postcard enums **append only**. New `SimEventKind` / `PrimaryAction` at the end.
- Mock + overlay off ⇒ idle default hash unchanged.
- `cargo test` never needs the network.
- Do not change shipping `default.toml` / `coop.toml` unless that is the slice.

---

## Parking lot

Empty on purpose. Add rows here (or in the Themes table) as they come up: aging, dialects, seasons, embeddings, Unix sockets, protobuf/TLS, combat `death_enabled`, etc. Prefer the After-M later-table when the item is already listed there.
