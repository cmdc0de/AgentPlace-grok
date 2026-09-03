# Post-GA feature list

Researcher-facing features **after** the current GA line (M1–M28 shipped: world, agents, LLM loop, governance, attach, combat v2-lite). **Not** a milestone plan. **Not** `missing-features.md` (in-use bugs). **Not** the After-M later-table (protobuf/TLS, embeddings, Unix sockets).

When a theme is scheduled, `/spec` picks 1–3 items into `docs/M{N}-plan.md`. Until then this file is the backlog. Milestone plans still win on timing vs this list and vs long-term specs.

Standing unless a later plan picks a bump: CI `provider = mock`; `format_version = 2`; `PROTOCOL_VERSION = 5`; shipping `configs/default.toml` / `coop.toml` unchanged; overlay TOML is not `ExperimentConfig` postcard.

---

## Themes

| ID | Theme | Status | One-liner |
|---|---|---|---|
| PG-1 | Kinship / family relations | Done (M31/M32) | Parent, child, sibling, pair-bond, household, `kin_of` / `household` scopes. |
| PG-2 | Children / reproduction | Done (M31) | PairBond/Reproduce; child sheet calculated. Aging/close-kin in M32/M34. |
| PG-3 | Physical sheet (D&D-like) | Done (M31) | STR/DEX/CON/INT/WIS/CHA rolled for founders; children mixed. |
| PG-4 | Sheet effects on the agent | Open | Remaining score → sim uses (accuracy, invent, haul cap, illness, …). M34 shipped a first set. |
| PG-5 | Inventions | Open | Invented artifacts: private payoff for the inventor vs public payoff for the society. |
| PG-6 | Viewer 3D models | Open | Replace primitive meshes with authored models for agents, food/veg, and every item. |
| PG-7 | Browser researcher UI | Done (M36) | Attach page lists every agent, board posts, and metrics — no 3D required. |

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

**Shipped later:** M31 rolled the six scores + CON `health_max`. M34 applied STR attack damage, DEX move cost, WIS perception range, CHA influence vote weight. Remaining uses live in **PG-4**.

---

## PG-4 — Sheet effects on the agent

**Shipped today (M34):** unused scores (0) keep today’s constants. Overlay `[agents.sheet]` / `--sheet` rolls founders. Derived mods `(score-10)/2`. CON → `health_max = 10000 + CON_mod*500`. STR → Attack damage `2000 + STR_mod*250`. DEX → move energy `base − DEX_mod*40`. WIS → +mod vision/hear/ident cells. CHA → influence vote weight `influence + CHA_mod*100` (not stored). INT unused for sim math.

**Wanted:** the rest of “the sheet changes what the body can do,” still derived (no extra blob fields), still 0-mod when unused.

| Score | Remaining / deeper uses (lock in `/spec`) |
|---|---|
| Constitution | Already health max. Later: energy max, illness duration/chance. |
| Dexterity / agility | Already move cost. Later: **defense vs accuracy** — attacker “to-hit” vs defender DEX so a high-DEX agent is **hit less often** when the attack needs a roll. Flee bonus. |
| Intelligence | **Invent** chance / quality (PG-5). Soft: memory cap, retrieval_k, plan length. |
| Strength | Already melee damage. Later: haul / inventory cap, gather/hunt payoff. |
| Wisdom | Already perception range. Later: toxin detect, board range. |
| Charisma | Already influence votes. Later: speech range/weight, proposal support, pair-bond odds. |

Accuracy sketch (not locked): Attack stays adjacent; overlay-on sheet rolls a seeded check `attacker STR or DEX` vs `defender DEX`. Miss → no damage (or reduced). Overlay off / unused scores ⇒ always hit as today. Mock + unused ⇒ **same hashes**.

INT → invent is **not** a free extra LLM call unless a later plan says so. Prefer a deterministic millipoint chance from INT_mod + a child RNG stream, then PG-5’s Invent action.

Overlay off / score 0 ⇒ current constants. Do not store derived HP/accuracy; recompute from scores so `--load` cannot double-apply.

---

## PG-5 — Inventions

**Shipped today:** Craft recipes (Basket, Spear, FishingRod, Backpack) are **fixed**. No mid-run discovery. No inventor credit. No society-wide unlock.

**Wanted:** an opt-in overlay so an agent can **invent** something new. An invention is a named, hashed record: who, when, what it does. It has **two payoffs** that a `/spec` must keep distinct:

| Audience | What it provides |
|---|---|
| **Inventor** | Private, first-mover: prestige / influence, exclusive Craft/use for *N* ticks, a protected memory, optional monopoly on the recipe. INT_mod (PG-4) raises invent chance or quality. |
| **Society** | Public, after a delay or a Share/Propose: others may Craft or use it; optional adopted rule, incentive, or world recipe unlock. Not automatic omniscience — living agents in range / on the board learn it. |

```toml
[inventions]
enabled = true          # default false
# later: share_delay_ticks, inventor_exclusive_ticks, max_live, …
```

CLI: `--inventions` (does **not** imply `--sheet`; INT bonus is 0 if unused).

Mechanics (sketch for a later `/spec`):

- New `PrimaryAction::Invent` (append). Legal only when overlay **on**. Mock never picks it unless a plan says so.
- Seeded outcome from `tick_{t}_agent_{id}_invent_0` plus INT_mod. Fail → Wait (or a hashed `InventFailed` if the plan wants it).
- Success appends `SimEventKind::Invented { … }` and a small table packed in the board blob (id, inventor, tick, kind/effect). Hash only when non-empty.
- Inventor payoff applies immediately to **that agent**. Society payoff starts later (delay, Share, or adopted rule) so A/B can measure “genius vs commons.”
- Overlay **off** ⇒ action illegal, empty table, **same hashes**.

Out of this theme until picked: full tech trees, stealing recipes, patents as governance meta-rules, LLM-written invention text (extra call).

---

## PG-6 — Viewer 3D models

**Shipped today:** the 3D view is **primitives + colour**. Agents are capsules; vegetation/animals/fish use distinct `Mesh` kinds (M5 legend); crates/packs/combat FX are cubes, flashes, and scaled fill. That is enough to tell berry from hare, not a world that looks like the items.

**Wanted:** authored **3D models** (glTF / Bevy scenes, or equivalent) so a researcher can read the scene without the legend:

| Kind | Model |
|---|---|
| Agent | Humanoid (or species body); optional sheet/CON scale stays hash-neutral. |
| Food / vegetation | One mesh per veg species (berry bush ≠ herb), plus hare, perch, crops. |
| Each `ItemId` | Wood, fiber, stone, basket, spear, fishing rod, backpack — in world, crate, and worn pack. |
| Later | Household home marker, invention artifacts (PG-5), incapacitated/dead poses. |

Constraints for a later `/spec`:

- **Hash-neutral.** Models never enter `state_hash` or checkpoints. Same sim, new art.
- One asset per kind; palette/LOD optional. Fog still hides what Observation cannot see.
- Fallback to today’s primitive if a file is missing (CI / headless must not require GPU art).
- Do not block `cargo test -p viewer` on loading `.glb` from the network.

Out of this theme until picked: skeletal animation cycles, photogrammetry, per-agent clothing from culture.

---

## PG-7 — Browser researcher UI (no 3D)

**Shipped today (M36):** `InspectorView::from_sim` (not hashed) plus `web/index.html` tables for every living agent, board posts, and metrics. Tick JSON extra key `inspector` (no PROTOCOL bump). wasm checkpoint decode still optional.

**Wanted (later):** protobuf/TLS/`wss`; `/set` `/give` in the page; time-series charts; wasm Snapshot decode. The browser client stays **2D / tables / text**. A researcher can inspect:

| Surface | What to show |
|---|---|
| Every agent | id, cell, hunger/thirst/energy, health, sheet, inventory/pack, goals, last action |
| Relationships | trust / affinity / respect / fear toward others (friendships) |
| Kinship | parent / child / sibling / pair-bond / household |
| Board | open and adopted posts (proposals, stances, rules) |
| Metrics | wall-clock tick timing (perceive/retrieve/select/execute) **and** sim needs (hungry, thirsty, tired, mean trust, illness) |
| Later | inventions (PG-5), culture, age/child, combat downed |

Constraints for a later `/spec`:

- **Hash-neutral attach.** Same PROTOCOL **5** postcard; no JSON wire, no `wss` unless a later slice bumps.
- Decode Snapshot checkpoint in-page (wasm/`shared` codec) or a thin read-only summary the server already sends. Do not add hashed events for “client connected.”
- Optional `--allow-control` for Play/Pause/Step only; full `/set` `/give` can wait.
- CI: loopback WS + page still builds without a GPU or a live display.

Out of this theme until picked: PG-6 3D in the browser, charts time-series (M8 parking), mobile layout polish.

---

## How these interact

```
Founders: roll sheet (PG-3) ──► live, relate (PG-1 feelings already shipped)
                              ──► optional pair-bond (PG-1)
                              ──► Reproduce (PG-2)
                                    ├── child sheet calculated (PG-3)
                                    └── kinship parent/child/sibling (PG-1)
```

Ship PG-3 before or with PG-2 so a birth has something to calculate. PG-1 kinship can land with PG-2 (links at birth) or slightly earlier (data model only).

PG-4 applies leftover sheet mods (accuracy, INT→invent, haul, …) on top of the M31 sheet. PG-5 inventions consume INT (PG-4) and write a hashed invention table; inventor vs society payoffs stay separate. PG-6 is viewer-only 3D art. PG-7 is the **browser** researcher UI (agents, posts, metrics) without 3D.

---

## Constraints for every later `/spec`

- Overlay / CLI, not `ExperimentConfig`, unless the slice **is** a hashed config change.
- Postcard enums **append only**. New `SimEventKind` / `PrimaryAction` at the end.
- Mock + overlay off ⇒ idle default hash unchanged.
- `cargo test` never needs the network.
- Do not change shipping `default.toml` / `coop.toml` unless that is the slice.

---

## Parking lot

Empty on purpose. Add rows here (or in the Themes table) as they come up: dialects, seasons, embeddings, Unix sockets, protobuf/TLS, etc. Prefer the After-M later-table when the item is already listed there. Sheet-effect leftovers, inventions, 3D models, and the browser inspector are **PG-4 / PG-5 / PG-6 / PG-7**, not parking-lot one-liners.
