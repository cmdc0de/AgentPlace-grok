# Remaining features

Pickable backlog for `/spec`. **Not** a milestone plan. **Not** `post-ga-feature-list.md` (theme essays) and **Not** `missing-features.md` (in-use bugs).

`/spec` Step 1 reads **this file** (Open + Standing + Scheduled) and INDEX for N. Do not pick from **Done**. Do not reconstruct leftovers from historical `M*-plan.md` later-tables.

When a slice ships, move that slice’s Scheduled rows to **Done** with `Done (M{N})`. Keep the row (same ID). Milestone plans and INDEX still record the slice; this table is the audit trail of which RF ids shipped.

Standing unless a later plan picks a bump: CI `provider = mock`; `format_version` writes **3** / reads v2+v3; `PROTOCOL_VERSION = 5`; shipping `configs/default.toml` / `configs/incentives/coop.toml` unchanged; overlay TOML is not `ExperimentConfig` postcard.

---

## Lifecycle

| Event | This file |
|---|---|
| `/spec` locks M{N} | Move picked **Open** rows → **Scheduled** (cite `M{N}-plan.md`). **Standing** rows stay. Add leftovers this slice created to **Open**. |
| `/implement-m` ships M{N} | Move that slice’s **Scheduled** rows → **Done** with `Done (M{N})` and a link to `M{N}-plan.md`. Add implement-discovered leftovers to **Open**. |
| User names a leftover in chat | Append an **Open** row (or expand an existing Open id). Do not invent backlog otherwise. |

IDs (`RF-n`) are stable. Do not reuse, including after **Done**. **Standing** (RF-PG9) is never moved to Done; extra-recipe slices still leave it in Standing.

---

## Open

Recommendable for a typical 1–3 pick unless noted. Size is a planning hint (S/M/L), not a promise.

| ID | Theme | Size | Hash / wire | One-liner |
|---|---|---|---|---|
| RF-1 | meshes | S | hash-neutral | household-home / invention / downed meshes (skipped M62 and M63) |
| RF-2 | durability | M | hashed wear | per-tick decay (not dawn) |
| RF-3 | durability | M | hashed wear | decay every instance, not just most-worn |
| RF-4 | stations | S | catalog hash | remaining food crafts using millstone (M63 used spit) |
| RF-7 | browser | M | hash-neutral | sql.js / ad-hoc SQL |
| RF-8 | attach | M | maybe PROTOCOL | Unix sockets |
| RF-10 | CI | M | hash-neutral | wasm32 on Win/mac |
| RF-11 | PG-8 | L | likely format bump | Food/Wood as strings |
| RF-12 | PG-11 | L | not JSON | OTLP protobuf/gRPC |
| RF-13 | wire | L | PROTOCOL | protobuf/TLS/`wss` |
| RF-14 | wire | L | PROTOCOL | `PROTOCOL_VERSION` bump |
| RF-15 | viewer | L | n/a | Bevy in the browser |
| RF-16 | storage | L | n/a | replacing JSONL |
| RF-17 | config | L | hashed defaults | flipping shipping `default.toml` / `coop.toml` |
| RF-18 | world | M | hashed | interiors: walls / blocked Move / doors (RF-5 this slice is non-square footprints only) |
| RF-19 | combat | S | hashed | ammo consume on ranged Attack (RF-6 this slice is projectile FX only) |
| RF-20 | PG-6 | M | hash-neutral | Standard animation monikers → glb clip names in `[visual.animations]` (`agent.toml` / object TOML). M64 shipped `idle` only. |

RF-11…RF-17 stay pickable. `/spec` must **not recommend** them unless the user asks. Same for extra LLM call types and new combat systems not listed here.

### RF-20 — animation monikers (hash-neutral)

M64: `[visual.animations] idle` in `configs/objects/agent.toml` maps to glb clip `ArmatureAction.002`. Missing clip / omit ⇒ first clip or static. Move this tick **pauses** idle (no walk clip yet). `[visual]` is not hashed.

Wanted: a **closed set of monikers** the viewer already understands, mapped per object file so a researcher can point each action at a clip name in that glb (or omit = no clip for that action). Same table on `agent.toml` and on any later skinned object TOML.

Sketch (lock keys in `/spec`; do not invent clips this leftover does not name):

```toml
# configs/objects/agent.toml  (and later other skinned objects)
[visual.animations]
idle   = "ArmatureAction.002"   # shipped M64
walk   = "Walk"                 # Move this tick
melee  = "Melee"                # Attack Chebyshev == 1
ranged = "Ranged"               # Attack Chebyshev > 1
flee   = "Flee"
downed = "Downed"               # Incapacitated
death  = "Death"                # CombatDeath
```

| Moniker | When (native viewer) |
|---|---|
| `idle` | no Move this tick (M64) |
| `walk` | `Move` this tick |
| `melee` | `Attack` dist 1 |
| `ranged` | `Attack` dist > 1 |
| `flee` | `Flee` |
| `downed` | `Incapacitated` |
| `death` | `CombatDeath` |

Omit a key ⇒ that action has no clip (idle pause / static, as M64 walk). Unknown monikers ignored. Clip name missing from the glb ⇒ static, not sentinel. No new authored animations required until a later slice adds files. Not hashed. Not browser 3D. Not PROTOCOL. Not extra LLM.

---

## Standing

Always available, even after a slice picks them. Do not move these to Scheduled.

| ID | Theme | Size | Hash / wire | One-liner |
|---|---|---|---|---|
| RF-PG9 | PG-9 | S | catalog hash | extra recipes (new object TOML; not new Craft verbs) |

---

## Typical deferrals

Do not recommend unless asked: RF-11…RF-17, extra LLM call types, Bevy in the browser, replacing JSONL, flipping shipping TOML, `PROTOCOL_VERSION` bump, protobuf/TLS/`wss`.

---

## Scheduled

None. M64 is implemented. Next `/spec` picks from Open + Standing.

---

## Done

Shipped RF rows. `/spec` does not pick from here. Newest first.

| ID | Theme | Size | Hash / wire | One-liner | Done |
|---|---|---|---|---|---|
| RF-9 | PG-6 | M | hash-neutral | agent glb clip `ArmatureAction.002` is **idle**; pause on Move | [M64](M64-plan.md) |
| RF-6 | combat FX | S | hash-neutral | ranged Attack projectile mesh (Chebyshev > 1); melee Strike unchanged | [M64](M64-plan.md) |
| RF-5 | world | M | hashed | non-square W×H sleep footprints (`sleep_w` / `sleep_h`; square omit-hash) | [M64](M64-plan.md) |
