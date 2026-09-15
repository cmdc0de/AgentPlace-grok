---
name: spec
description: >
  Plan the next milestone feature set for AgentPlace-grok: leftover list from
  docs/remaining-features.md, pick 1–3 in-scope items, lock overlay/hash/protocol,
  write docs/M{N}-plan.md, and retarget INDEX plus remaining-features. Do not
  implement code, write the walkthrough test plan, or rewrite historical
  M*-plan later-tables. Use when the user says “plan M18”, “next milestone”,
  “feature set for M17”, “spec the next slice”, /spec, or “back into plan mode”
  to choose the next M-slice.
---

# Spec the next milestone

Docs-only slice for **what we build next**. Not implement, not `M{N}-test-plan.md`, not tag.

If the user has not numbered N, N = last **Done** milestone in `docs/00-INDEX-AND-HANDOFF.md` + 1.

## Standing constraints (unless the user picks them)

- CI `provider = mock`; `cargo test` never needs the network.
- `format_version` writes **3** / reads v2+v3; `PROTOCOL_VERSION = 5` unless the slice is a wire/ckpt bump.
- Do not change shipping `configs/default.toml` or `configs/incentives/coop.toml` unless that *is* the slice.
- Overlay TOML (`[voting]`, `[incentives]`, `[storage]`, `[network]`) is not `ExperimentConfig` postcard.
- Long-term specs vs this slice: **the milestone plan wins on timing**.

## Step 1 — Inventory leftovers

Read, do not invent a backlog:

1. `docs/remaining-features.md` — **Open**, **Standing**, **Scheduled** (not **Done**)
2. INDEX progress table only to compute N (last **Done** + 1)

Do **not** gather leftovers from **Done**, historical `docs/M*-plan.md` later-tables, `M*-test-plan.md` Next-slice lines, spec `Current slice:` banners, `post-ga-feature-list.md` theme essays, or `missing-features.md`. Those are not inventory.

Present **leftovers + a recommended 1–2** from recommendable Open + Standing (same shape as recent slices: one experiment ± one researcher tool; RF-PG9 extra recipes is always a legal extra). Use a multi-select question.

Typical deferrals unless the user picks them: RF-11…RF-17 in remaining-features (Food/Wood as strings, OTLP protobuf/gRPC, protobuf/TLS/`wss`, `PROTOCOL_VERSION` bump, Bevy in the browser, replacing JSONL, flipping shipping TOML), extra LLM call types, new combat systems.

## Step 2 — Lock the slice

After the user picks, **do not write `docs/M{N}-plan.md` yet**. Write a locked plan (session plan / plan-mode present) that states:

- Goal (what a researcher can do), 3–5 bullets
- In-scope mechanics: TOML/CLI/overlay syntax, who/when, hash effects
- Out of scope: this-slice **Not M{N}** deferrals + “full backlog: `docs/remaining-features.md`”. Do **not** copy the whole Open table into the plan.
- Acceptance tests table
- PR split (usually 2–3)
- Risks (postcard enum append, double-apply on `--load`, protocol bump)

If they revise (e.g. add overnight Spark A/B), update the locked plan and re-present. Only after they **agree** go to Step 3.

## Step 3 — Write `docs/M{N}-plan.md`

Status: `planned (not yet implemented)`. Depends on M{N-1} tag + commit. Same section order as recent plans: Context, Goal, In scope, Out of scope, Key decisions, Tests, PR Plan, Config/CLI, Verification, Risks.

Out of scope table is **short**: items considered and deferred this slice, plus a pointer to `remaining-features.md`. Do not duplicate the full backlog.

No code. No `configs/` examples unless the lock already named a new overlay file (create that file only on **implement**).

## Step 4 — Retarget docs

| Place | Change |
|---|---|
| `docs/M{N}-plan.md` | new file (Step 3) |
| `docs/remaining-features.md` | picked **Open** rows → **Scheduled** (cite `M{N}-plan.md`); **Standing** stays; add leftovers this slice created to **Open** |
| `docs/00-INDEX-AND-HANDOFF.md` | M{N} **Planned — next**; After-row stays `remaining-features.md`; file-list row for `M{N}-plan.md`; suggested next request points at remaining-features + `M{N}-plan.md` |
| README | **leave** on M{N-1} until implement |
| Historical `docs/M*-plan.md` later-tables | **do not touch** |
| Historical `docs/M*-test-plan.md` Next-slice lines | **do not touch** |
| Spec `Current slice:` banners | **do not touch** (frozen; they point at INDEX + remaining-features) |
| `post-ga-feature-list.md` / `missing-features.md` | **do not rewrite** unless this slice completes or opens a named theme/bug row |

`docs/incentive-schedule-format.md` stays shipping truth until implement.

## Step 5 — Stop

Summarize the locked slice. Offer commit/push of the **docs-only** plan (no tag). Do not implement until they say implement.

## Commit message (if they ask)

```
Add M{N} plan: <short slice name>.
Docs only. No code.
```
