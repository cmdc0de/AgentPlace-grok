# M20 — Revert relationship_delta on leave, stacked worn packs, attach safety net

**Status:** implemented  
**Depends on:** M19 complete (`docs/M19-plan.md`, git tag `M19`, commit `92e408d`)  
**Walkthrough:** [`M20-test-plan.md`](M20-test-plan.md)  
**Specs:** `incentive-schedule-format.md` (one-shots), `M17-plan.md` (leave reverts influence only), `M19-plan.md` (one worn pack; Backpack replaces Basket), `M7-plan.md` (TCP/WS attach)

## Context

M17 leave/end reverts **influence only**; relationship one-shots stick. M19 wears **one** pack (Backpack replaces Basket caps). Milestone walkthroughs do not re-run M7 attach (`sim-cli --test net`).

M20 does **not** rewrite postcard, add TLS, wire Give, or bump `PROTOCOL_VERSION`.

## Goal

A researcher can:

1. Use `supporters_of:` one-shot `relationship_delta` knowing **leave and incentive end undo that delta** (same toward set as apply). Goals still stay. Influence revert unchanged.
2. Set overlay `[storage] max_worn_baskets` / `max_worn_backpacks` so **that many** of each count toward stacked pack caps. Omit both → **1 and 1** (M11/M19 hashes). Extra copies stay cargo in pockets.
3. Rely on the M20 walkthrough (and later `/implement-m`) **§0** to include `cargo test -p sim-cli --test net` and `cargo test -p shared` (TCP/WS loopback). Attach stays hash-neutral read-only.
4. CI stays `provider = mock`. `format_version = 2`, `PROTOCOL_VERSION = 2`. Shipping `default.toml` / `coop.toml` unchanged ⇒ default hashes unchanged.

## In scope

### A. Revert `relationship_delta` on leave (and end)

Same `[[incentives]]` TOML as M17 (`relationship_delta` + `toward`). No new fields.

| Event | One-shots |
|---|---|
| Start / join | apply as today (influence, goals, relationship_delta) |
| **Leave** | revert `influence_factor_delta` **and** `relationship_delta` (negate trust/affinity/respect, same `toward` set, clamp `REL_MIN`/`REL_MAX`). **Goals stay** |
| **End** | same revert for remaining members (today end only undoes influence) |

- Subtract the **original** milli delta (same as influence). Decay may mean the row does not return to pre-apply; clamp, do not record a separate “applied amount.”
- Missing relationship row on revert: no-op (do not create a negative-only row).
- `format_version` stays **2**. `incentive_oneshot` map unchanged.
- Default hashes unchanged (no leave in idle mock).

### B. Stacked worn packs (configurable counts)

Existing `[storage]` overlay (not `ExperimentConfig`, not postcard). **New optional keys:**

```toml
[storage]
max_worn_baskets = 1      # omit = 1 (M11)
max_worn_backpacks = 1    # omit = 1 (M19)
```

| Field | Meaning |
|---|---|
| `max_worn_baskets` | How many **Basket** items count as worn pack (stacked 8 slots / 25.0 each). Extra Baskets stay cargo |
| `max_worn_backpacks` | How many **Backpack** items count as worn pack (stacked 12 slots / 40.0 each). Extra Backpacks stay cargo |

- `0` = that kind never wears (cargo only). Negative / non-integer → **load error**.
- Worn count = `min(inventory count, max_worn_*)`.
- Pack caps:

| Example | Slots | Weight display |
|---|---|---|
| omit / `1,1`, Basket only | 8 | 25 |
| omit / `1,1`, Backpack only | 12 | 40 |
| omit / `1,1`, both | 20 | 65 |
| `max_worn_baskets = 2`, two Baskets | 16 | 50 |
| `max_worn_backpacks = 2`, two Backpacks | 24 | 80 |

- One `Agent.pack` map still. Pack / Unpack / haul 0.1 unchanged. Pack carriers cannot go inside the pack.
- Dropping a carrier so worn count falls: caps shrink. If current pack **exceeds** the new caps → **refuse** that Transfer/Store/Give (same as M19 last-Basket overflow). Empty pack always ok.
- Viewer: at most **one** Basket satchel mesh if any worn basket, **one** Backpack mesh if any worn backpack (not N copies). Fog hides with the agent.
- Do **not** add these keys to shipping `configs/default.toml`.
- `ItemId` / `Recipe` already appended in M19; no new enum variants.

### C. Walkthrough attach safety net (no wire bump)

Not a protocol feature. On **implement**:

- `docs/M20-test-plan.md` §0 includes:
  ```bash
  cargo test -p sim-cli --test net
  cargo test -p shared
  ```
- Update `.grok/skills/implement-m/SKILL.md` Step 3 §0: plus `-p sim-cli --test net` and `-p shared` for every later deliver (loopback only).
- Do **not** add `ControlVerb`. Remote `/give` `/set` `/scrub` still refuse.

## Out of scope (later)

| Later | What |
|---|---|
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
| **M36** | [`M36-plan.md`](M36-plan.md) — browser researcher UI (no 3D) |
| After M36 | protobuf/TLS |
| Not M20 | Browser; combat; CI Win/mac; extra LLM reflection/embeddings |

## Key decisions

1. Leave **and** incentive end revert relationship_delta; goals stay.
2. Overlay `[storage] max_worn_baskets` / `max_worn_backpacks`, omit = **1,1**.
3. Stacked caps = `min(count, max) ×` unit caps; extras are cargo.
4. Cap shrink below fill **refuses** the drop, does not clip.
5. Attach safety net is loopback tests + skill §0, not a `ControlVerb`.
6. Do not change shipping `configs/default.toml` / `coop.toml`.
7. Mock CI. `PROTOCOL_VERSION = 2`. No new `ControlVerb`.

## Tests (M20 acceptance bar)

| Test | Asserts |
|---|---|
| join `relationship_delta` respect toward 0, then leave | joiner respect back (influence already reverted; goal remains) |
| incentive **end** with remaining members | influence **and** relationship_delta undone |
| omit max_worn_* | Basket 8/25; Backpack 12/40; both 20/65 |
| `max_worn_baskets = 2`, two Baskets | 16 / 50; 16 food packs |
| `max_worn_backpacks = 0`, hold Backpack | no pack (cargo only) |
| `max_worn_baskets = -1` | load error |
| last Basket while Backpack held, pack over 12 | Transfer/Store **refused** |
| `cargo test -p sim-cli --test net` | hash-neutral attach |
| `cargo test -p shared` | tcp + ws loopback Hello/Snapshot |
| default.toml mock | hashes match pre-M20 |
| `cargo test -p sim-core` | no network |

## PR Plan

### PR 1: Revert relationship_delta

- **Files:** `incentive.rs` (leave + end revert relationship_delta), incentives tests

### PR 2: Stacked worn-pack caps + overlay counts

- **Files:** `StorageParams` + sim-cli `[storage]` parse, `agent.rs` `worn_pack_caps`, observation/execute last-carrier refuse, viewer one satchel + one backpack mesh, storage tests

### PR 3: Attach safety net

- **Files:** `implement-m` SKILL.md §0, this plan + `M20-test-plan.md`

## Config / CLI

Overlay `[storage]` only (not postcard). No new `ControlVerb`.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

## Verification

Walkthrough: [`M20-test-plan.md`](M20-test-plan.md).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: leave undoes relationship_delta; omit max_worn is 1,1; two Baskets with max 2 pack 16; attach net tests green; default mock hashes match.

## Risks

- **Delta after decay.** Revert subtracts the original milli; row may not match pre-apply. Same as influence.
- **Cap shrink refuse.** Stacked pack then drop a carrier so caps shrink below fill must refuse, not silently clip.
- **Default 1,1.** Omit must match M11/M19 hashes.
- **Do not add `ControlVerb` or bump `PROTOCOL_VERSION`.** Attach tests are loopback only.
