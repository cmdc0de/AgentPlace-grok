---
name: spec
description: >
  Plan the next milestone feature set for AgentPlace-grok: leftover list, pick
  1–3 in-scope items, lock overlay/hash/protocol, write docs/M{N}-plan.md, and
  retarget INDEX, spec banners, and later-tables. Do not implement code or write
  the walkthrough test plan. Use when the user says “plan M18”, “next milestone”,
  “feature set for M17”, “spec the next slice”, /spec, or “back into plan mode”
  to choose the next M-slice.
---

# Spec the next milestone

Docs-only slice for **what we build next**. Not implement, not `M{N}-test-plan.md`, not tag.

If the user has not numbered N, N = last **Done** milestone in `docs/00-INDEX-AND-HANDOFF.md` + 1.

## Standing constraints (unless the user picks them)

- CI `provider = mock`; `cargo test` never needs the network.
- `format_version = 2`, `PROTOCOL_VERSION = 2` unless the slice is a wire/ckpt bump.
- Do not change shipping `configs/default.toml` or `configs/incentives/coop.toml` unless that *is* the slice.
- Overlay TOML (`[voting]`, `[incentives]`, `[storage]`, `[network]`) is not `ExperimentConfig` postcard.
- Long-term specs vs this slice: **the milestone plan wins on timing**.

## Step 1 — Inventory leftovers

Read, do not invent a backlog:

1. INDEX progress table + “After M{N-1}”
2. `docs/M{N-1}-plan.md` **Out of scope**
3. Later-tables in recent `docs/M*-plan.md` (protobuf/TLS, wire Give, etc.)
4. Spec banners still pointing at M{N-1}

Present **leftovers + a recommended 1–2** (same shape as recent slices: one experiment ± one researcher tool). Use a multi-select question. Typical deferrals unless the user picks them: protobuf/TLS, `PROTOCOL_VERSION` bump, wire Give, extra LLM call types, browser, combat.

## Step 2 — Lock the slice

After the user picks, **do not write `docs/M{N}-plan.md` yet**. Write a locked plan (session plan / plan-mode present) that states:

- Goal (what a researcher can do), 3–5 bullets
- In-scope mechanics: TOML/CLI/overlay syntax, who/when, hash effects
- Out of scope table (After M{N} leftovers)
- Acceptance tests table
- PR split (usually 2–3)
- Risks (postcard enum append, double-apply on `--load`, protocol bump)

If they revise (e.g. add overnight Spark A/B), update the locked plan and re-present. Only after they **agree** go to Step 3.

## Step 3 — Write `docs/M{N}-plan.md`

Status: `planned (not yet implemented)`. Depends on M{N-1} tag + commit. Same section order as recent plans: Context, Goal, In scope, Out of scope, Key decisions, Tests, PR Plan, Config/CLI, Verification, Risks.

No code. No `configs/` examples unless the lock already named a new overlay file (create that file only on **implement**).

## Step 4 — Retarget docs

| Place | Change |
|---|---|
| `docs/00-INDEX-AND-HANDOFF.md` | M{N} **Planned — next**; After M{N} leftovers; file list row; suggested next request points at `M{N}-plan.md` |
| Spec `Current slice:` banners | `docs/M{N}-plan.md` (the six files that already have that banner) |
| `docs/M2-plan.md` … `docs/M{N-1}-plan.md` later-tables | M{N-1} **Done**; add M{N} row; **After M{N}** (drop items now in M{N}) |
| Walkthrough “Next slice” lines | `M{N}-plan.md` |
| README | **leave** on M{N-1} until implement |

`docs/incentive-schedule-format.md` stays shipping truth until implement.

## Step 5 — Stop

Summarize the locked slice. Offer commit/push of the **docs-only** plan (no tag). Do not implement until they say implement.

## Commit message (if they ask)

```
Add M{N} plan: <short slice name>.
Docs only. No code.
```
