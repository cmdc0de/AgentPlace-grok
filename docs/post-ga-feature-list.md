# Post-GA feature list

Researcher-facing features **after** the current GA line (M1–M28 shipped: world, agents, LLM loop, governance, attach, combat v2-lite). **Not** a milestone plan. **Not** `missing-features.md` (in-use bugs). **Not** the After-M later-table (protobuf/TLS, embeddings, Unix sockets).

When a theme is scheduled, `/spec` picks 1–3 items into `docs/M{N}-plan.md`. Until then this file is the backlog. Milestone plans still win on timing vs this list and vs long-term specs.

Standing unless a later plan picks a bump: CI `provider = mock`; `format_version` writes **3** / reads v2+v3; `PROTOCOL_VERSION = 5`; shipping `configs/default.toml` / `coop.toml` unchanged; overlay TOML is not `ExperimentConfig` postcard.

---

## Themes

| ID | Theme | Status | One-liner |
|---|---|---|---|
| PG-1 | Kinship / family relations | Done (M31/M32) | Parent, child, sibling, pair-bond, household, `kin_of` / `household` scopes. |
| PG-2 | Children / reproduction | Done (M31) | PairBond/Reproduce; child sheet calculated. Aging/close-kin in M32/M34. |
| PG-3 | Physical sheet (D&D-like) | Done (M31) | STR/DEX/CON/INT/WIS/CHA rolled for founders; children mixed. |
| PG-4 | Sheet effects on the agent | Open | Remaining score → sim uses (accuracy, invent, haul cap, illness, …). M34 shipped a first set. |
| PG-5 | Inventions | Open | Invented artifacts: private payoff for the inventor vs public payoff for the society. |
| PG-6 | Viewer 3D models | Open | Replace primitive meshes with authored models. M38: stem-named `.glb`. Later: per-object config (path + LOD). |
| PG-7 | Browser researcher UI | Done (M36) | Attach page lists every agent, board posts, and metrics — no 3D required. |
| PG-8 | Object / item definition files | Open | One config file per sim object: visuals (glb + LOD) and sim fields so new items/crafts are a file, not a Rust enum. |
| PG-9 | Crafting recipe catalog | Open | Dedicate a milestone to **more recipes** (new object TOML + crafts), not new Craft mechanics. |
| PG-10 | Missing-asset sentinel | Done (M47) | Configured glb missing ⇒ one fixed, unmistakable mesh so a bad path is obvious. Not today’s silent primitive. |
| PG-11 | OpenTelemetry / performance metrics | Open | OTLP export of sim + viewer timings, plus process CPU / disk / memory. Hash-neutral; overlay off in CI. |
| PG-12 | Viewer camera pan (keys) | Done (M47) | Arrow keys pan at constant height; `u` up, `d` down (`L` stays legend). Hash-neutral. Native window only. |
| PG-13 | Object visual scale | Open | Per-object `[visual] scale` on each glb so a researcher can size each mesh without re-exporting. Hash-neutral. |

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

**Shipped today (M38):** if `assets/models/{stem}.glb` (or `.gltf`) exists, the viewer loads it; otherwise today’s primitive. Stem = kind name (`berry_bush`, `agent`, `crate`, …). No per-object config. No LOD. Models are **not hashed**.

**Wanted:** each object’s **visuals** come from a config (PG-8), not from guessing the stem:

| Field | Meaning |
|---|---|
| `mesh` / `glb` | Path to the authored glTF/glb (absolute under `assets/` or relative to the definition file). |
| `lod` | Optional extra meshes by distance (e.g. `near`, `mid`, `far`) so a far berry bush is a cheaper mesh. Missing LOD step ⇒ next coarser, then primitive. |
| Fallback | File missing or path empty ⇒ today’s primitive. **PG-10** replaces “configured path missing” with a dedicated sentinel mesh. |

Constraints for a later `/spec`:

- **Hash-neutral** unless the same file also carries hashed sim fields (then split: visuals overlay vs catalog — see PG-8).
- Fog still hides what Observation cannot see.
- `cargo test -p viewer` must not download `.glb` and must not require a GPU window.
- Do not block CI on GPU art.

Out of this theme until picked: skeletal animation cycles, photogrammetry, per-agent clothing from culture, household-home / invention / downed poses. Missing-path sentinel is **PG-10**. Per-object glb **scale** is **PG-13**.

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

## PG-8 — Object / item definition files

**Shipped today:** kinds are **code**. `ItemId` is a Rust enum (Food/Wood/Fiber/Stone/Basket/Spear/FishingRod/Backpack). Veg species live in world config + `markers.rs` names. Craft recipes are a `match` in `execute`. Viewer art is a **stem** (`berry_bush.glb`) with no per-object file. Adding a new craftable item means a postcard enum append, recipe arm, marker, and a matching filename.

**Wanted:** one **definition file per object** (TOML or equivalent) that a researcher can drop in without a new `ItemId` variant for every experiment. The file describes both **what it is in the sim** and **how it looks**:

```toml
# sketch — lock fields in a later /spec
id = "berry_bush"
kind = "vegetation"          # vegetation | animal | fish | item | agent | crate | …

[sim]
# hashed when this object exists in the run (mass, slots, craft recipe, nutrition, …)
# omit / overlay off ⇒ today’s hardcoded table, same hashes

[visual]
# not hashed
glb = "assets/models/berry_bush.glb"
scale = 1.0                  # omit = 1.0; **PG-13**
[visual.lod]
near = "assets/models/berry_bush.glb"
mid  = "assets/models/berry_bush_lod1.glb"
far  = "assets/models/berry_bush_lod2.glb"
```

| Job | What the file enables |
|---|---|
| **New item / craft** | Add a definition + recipe table; agents can Gather/Craft/Store it. No new Rust `ItemId` arm for that experiment (postcard strategy locked in `/spec`: string id vs append-only enum alias). |
| **Change the look** | Point `glb` / `lod` at different files. Same sim, new art (hash-neutral if `[sim]` unchanged). |
| **LOD** | Viewer picks near/mid/far from camera (or agent) distance. Missing file ⇒ coarser LOD, then primitive. |
| **Scale** | **PG-13** — per-object (and later per-glb) multiplier so each mesh can be sized in TOML. |

Constraints for a later `/spec`:

- Split **hashed sim fields** from **hash-neutral visuals**. Changing only `glb` / `lod` must not change `state_hash`. Adding a new item that agents can hold **does** change the hash (inventory keys, recipes).
- Overlay / directory of definitions, not `ExperimentConfig`, unless that slice **is** a hashed catalog bump. Shipping `default.toml` / `coop.toml` stay as today until a plan says otherwise.
- Old ckpts: unknown item ids skip or map; do not fail magic/`format_version = 2` unless the slice bumps ckpt.
- Mock + catalog-off ⇒ idle hash `70e5204d…`. `cargo test` never needs the network. Missing glb never fails CI.
- Postcard enums stay **append only** if ids stay numeric; a string-id catalog is a `/spec` lock (and may be a PROTOCOL/ckpt bump — do not sneak it).

Out of this theme until picked: full tech tree (PG-5), procedural mesh generation, runtime hot-reload of glb from disk while the window is open. Growing the **number** of crafts is **PG-9** (content milestone). Missing configured glb is **PG-10**. Per-object glb **scale** is **PG-13**.

---

## PG-9 — Crafting recipe catalog

**Shipped today:** Craft is real. Built-ins are Basket, Spear, FishingRod, Backpack. PG-8 / M39–M40 let a researcher add a catalog item with `[sim.craft]` in object TOML (`configs/objects/cord.toml` is the extra). The **count** of useful recipes is still tiny. Mock does not pick Craft unless a goal / overlay drives it.

**Wanted:** **one dedicated milestone** whose in-scope is **more recipes**, not a new Craft verb or a new `ItemId` enum. Drop object TOML + visuals + (optional) catalog overlay so agents can Gather/Craft/Store a broader kit.

Sketch (lock the batch in `/spec`):

| Kind of recipe | Why |
|---|---|
| Intermediate | Cord, plank, charcoal — inputs for later crafts. |
| Tools | Knife, hammer, net, hoe — change gather/hunt/fish/farm odds or energy. |
| Containers / wear | Extra pack, waterskin, satchel — haul/slots like Basket/Backpack. |
| Food processing | Dried fish, cooked veg — hunger vs raw; optional toxin change. |

Constraints for a later `/spec`:

- Use **PG-8 files**, not new Rust recipe arms, unless a leftover builtin must stay an enum tag.
- Overlay / `--catalog` / objects dir. Catalog-off / empty catalog ⇒ idle mock hash **unchanged**.
- New held items **do** change catalog-on hashes (inventory keys, recipes). Document it.
- Each new craft needs a visual path (PG-6/PG-8). Missing file is **PG-10**, not a CI fail.
- Mock still does not have to pick the new crafts unless that slice says so.
- Do not flip shipping `default.toml` / `coop.toml` unless that is the slice.

Out of this theme until picked: durability, workstations, multi-agent crafts, LLM-written recipes, PG-5 invention-unlock of these recipes.

---

## PG-10 — Missing-asset sentinel

**Shipped today (M38/M40):** if the configured `glb` / LOD path is missing, the viewer falls back to a **primitive** (capsule/box/etc.). That looks like “no art on purpose.” A researcher cannot tell a **broken path** from an object that was never given a mesh.

**Wanted:** when a visual **is configured** and the file cannot be found (or fails to load), spawn **one fixed sentinel asset** so the problem is obvious in the 3D view.

| Case | Mesh |
|---|---|
| No `[visual]` / empty path | Today’s primitive (intentional “no art”). |
| Path set, file missing, or glb fails to load | **Sentinel** — one checked-in mesh (e.g. `assets/models/missing.glb`) or a generated magenta/error marker. Same mesh for every broken id. |
| Path set, file exists | Authored glb as today. LOD miss ⇒ next coarser, then sentinel if a path was configured, not primitive. |

Constraints for a later `/spec`:

- **Hash-neutral.** Visuals are never in `state_hash`. Do not add hashed events for “asset missing.”
- One sentinel, shared. Do not invent a per-kind missing mesh.
- `cargo test -p viewer` must not require a GPU window. Unit-test: configured-but-missing path resolves to the sentinel, not `None`/primitive.
- Missing authored glb never fails CI. Sentinel file is in-repo (or generated in code) so CI can see the fallback without downloading.
- Log / HUD line optional (hash-neutral). The 3D mesh is the required tell.

Out of this theme until picked: skeletal animation, hot-reload glb, Bevy in the browser, photogrammetry.

---

## PG-11 — OpenTelemetry / performance metrics

**Shipped today (M8 + TickTiming):** each sim tick can record **hash-neutral** wall-clock ns (`wall_ns`, `world_ns`, `board_ns`, `incentive_ns`, `agents_ns`, per-agent `perceive_ns` / `retrieve_ns` / `select_ns` / `execute_ns` / `remember_ns`). Written to timing JSONL when `--out-dir` is set; optional JSON on `Tick.metrics`. Inspector / page show sim needs and last-tick timing. **Not** in `state_hash`. No OpenTelemetry. No process CPU / RSS / disk. No viewer **render** frame times. No run-level aggregates (count, mean, median, min, max).

**Wanted:** opt-in **OpenTelemetry** (OTLP) so a researcher can scrape or push the same numbers Grafana / Prometheus / an OTel collector already know, plus **technical** process stats and **frame** stats.

### What to emit

| Group | Series (lock names in `/spec`) |
|---|---|
| **Sim tick** | Duration of each simulation tick (`wall_ns` and the pipeline stages). |
| **Sim aggregates** | Over the run (or a sliding window): **count** of ticks, **total** time, **average**, **median**, **min**, **max** (and optional p95/p99). |
| **Viewer render** | Time to draw one Bevy/imgui frame. Same aggregates: count, total, avg, median, min, max. Distinct from sim tick time (a paused server still renders). |
| **Process** | CPU (user/system or percent), **memory** (RSS / peak), **disk** (bytes read/written or out-dir size). Host, not sim-hash. |

Pipeline-stage histograms (perceive vs execute) can reuse today’s `TickTiming` fields. Do **not** replace JSONL; OTel is an extra sink.

### Overlay (sketch)

```toml
[telemetry]
enabled = false          # omit = false; CI stays off
# later: otlp_endpoint, protocol = "http" | "grpc", service_name, …
```

CLI: `--telemetry` (does not imply `--llm` or `--catalog`). Overlay **off** ⇒ no exporter, no extra threads required, **same hashes**.

Constraints for a later `/spec`:

- **Hash-neutral.** CPU, RSS, disk, render ms, and wall-clock ns never enter `state_hash` or AGTN. Same rule as M8 timing / M45 pipeline events (bitmask hashed; ns not).
- Overlay / CLI, not `ExperimentConfig`. Shipping `default.toml` unchanged.
- `cargo test` never needs the network. Default exporter **off**; no OTLP in CI. Unit-test in-process meters (or a mock exporter), not a live collector.
- Viewer render metrics are **native window only** unless a later slice puts Bevy in the browser.
- Time-series **charts** in imgui / the attach page can consume these series later (already parked as charts). OTel export can ship without a new GUI.
- Do not add hashed `SimEventKind` rows for “tick took 7 ms.”

Out of this theme until picked: protobuf/TLS on the **sim wire** (different from OTLP); distributed tracing of every LLM HTTP call unless that `/spec` wants spans; eBPF.

---

## PG-12 — Viewer camera pan (keys)

**Shipped today:** the 3D camera starts at a fixed offset looking at map center. `/follow ID` (or digit keys) snaps the camera to an agent each frame. There is **no** free pan. `L` currently **toggles the legend** (imgui), not camera height.

**Wanted:** a researcher in the native viewer can fly the camera over the map without changing sim state.

| Key | Motion (world XZ, **Y unchanged** unless noted) |
|---|---|
| Arrow **Right** | Pan right (camera-right, projected on the ground plane) |
| Arrow **Left** | Pan left |
| Arrow **Up** (forward) | Pan forward (look direction on XZ; do not dive into the terrain) |
| Arrow **Down** (back) | Pan back |
| `u` | Raise camera (**+Y** only) |
| `l` | Lower camera (**−Y** only), clamp above the terrain |

Constraints for a later `/spec`:

- **Hash-neutral.** Camera transform is viewer-only. No events, no `state_hash`.
- Step size: lock cells-per-tap (or hold-to-repeat) in `/spec`. Stay at the current height for arrows.
- **Key clash:** `L` is legend today. Lock one of: (a) camera `l` wins and legend moves (e.g. `Shift+L` / imgui only), or (b) keep legend on `L` and pick another down key. The wanted binding is `u` / `l` as above.
- While `/follow` is on, first pan **cancels follow** (free cam) so arrows are not fighting the follow snap.
- Ignore these keys when the imgui console has keyboard focus (`want_keyboard`), same as other viewer shortcuts.
- Native Bevy window only. Not the browser page. `cargo test -p viewer` must not need a GPU; unit-test the pan delta helper with fake transforms.
- Do not change shipping TOML. No PROTOCOL bump.

Out of this theme until picked: mouse-drag orbit, scroll zoom, gamepad, cinematic paths, Bevy in the browser.

---

## PG-13 — Object visual scale

**Shipped today:** object TOML has `[visual] glb` + LOD paths (PG-8 / M40). Agent glbs are auto-fit to the capsule height (1.11) in the viewer. Other kinds load at export scale. There is **no** per-object or per-file scale in the definition. Changing size means re-exporting the glb.

**Wanted:** each object file can set a **scale factor** for its authored mesh so a researcher sizes that object without touching the glb.

```toml
# sketch — lock fields in a later /spec
id = "tree"
kind = "vegetation"

[visual]
glb = "assets/models/optimized/maple_tree.glb"
scale = 0.4                 # omit = 1.0; uniform XYZ
[visual.lod]
near = "assets/models/optimized/maple_tree.glb"
# later: per-LOD scale if a lod file needs a different multiplier
```

| Case | Scale |
|---|---|
| `scale` omitted / `1.0` | Today’s load size (agent still auto-fits to the capsule unless a later `/spec` says explicit scale **replaces** auto-fit) |
| `scale = N` | Uniform multiplier on that object’s authored glb (and LOD unless overridden) |
| Primitive / sentinel | Unchanged (no TOML scale) |
| Per-LOD / non-uniform XYZ | Later unless that `/spec` wants it |

Constraints for a later `/spec`:

- **Hash-neutral.** `scale` lives on `[visual]`, never in `state_hash` or AGTN. Same rule as `glb` / LOD.
- One scale **per object file** so each kind can be sized independently (`tree` vs `berry_bush` vs `agent`).
- Native Bevy window only. `cargo test -p viewer` must not need a GPU; unit-test “def.scale 2.0 → transform scale 2.0” with fake defs.
- Missing / non-finite / `<= 0` scale ⇒ treat as `1.0` (do not fail CI or skip the mesh).
- Do not change shipping `default.toml` / `coop.toml`. No PROTOCOL bump.
- Sentinel and empty-path primitives stay unscaled.

Out of this theme until picked: skeletal animation, per-agent clothing, Bevy in the browser, non-uniform XYZ unless the `/spec` locks it.

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

PG-4 applies leftover sheet mods (accuracy, INT→invent, haul, …) on top of the M31 sheet. PG-5 inventions consume INT (PG-4) and write a hashed invention table; inventor vs society payoffs stay separate. PG-6 is viewer-only 3D art (M38 stem files; later config + LOD). PG-7 is the **browser** researcher UI (agents, posts, metrics) without 3D. PG-8 is the **object catalog**: one file per kind so new crafts and new looks are config, with `[sim]` hashed and `[visual]` / LOD not. PG-9 is a **content** slice on top of PG-8: more recipes in one milestone. PG-10 is the viewer missing-path mesh so a bad `glb` is obvious. PG-11 is **OpenTelemetry**: sim tick + viewer frame aggregates and process CPU/disk/memory, hash-neutral, overlay off unless a plan turns it on. PG-12 is **viewer camera pan** (arrows + `u`/`d`), hash-neutral, native window only. PG-13 is **per-object `[visual] scale`** so each glb can be sized in TOML without re-exporting.

---

## Constraints for every later `/spec`

- Overlay / CLI, not `ExperimentConfig`, unless the slice **is** a hashed config change.
- Postcard enums **append only**. New `SimEventKind` / `PrimaryAction` at the end.
- Mock + overlay off ⇒ idle default hash unchanged.
- `cargo test` never needs the network.
- Do not change shipping `default.toml` / `coop.toml` unless that is the slice.

---

## Parking lot

Empty on purpose. Add rows here (or in the Themes table) as they come up: dialects, seasons, embeddings, Unix sockets, protobuf/TLS, etc. Prefer the After-M later-table when the item is already listed there. Sheet-effect leftovers, inventions, 3D models, the browser inspector, object definition files, extra recipes, the missing-asset sentinel, OpenTelemetry, viewer camera pan, and per-object visual scale are **PG-4 / PG-5 / PG-6 / PG-7 / PG-8 / PG-9 / PG-10 / PG-11 / PG-12 / PG-13**, not parking-lot one-liners.
