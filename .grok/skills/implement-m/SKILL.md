---
name: implement-m
description: >
  Implement the current AgentPlace-grok milestone from docs/M{N}-plan.md: code
  the in-scope items, write docs/M{N}-test-plan.md, run every non-GUI walkthrough
  test, and retarget README, INDEX, and plan status to implemented. Do not spec
  the next slice or tag unless asked. Use when the user says “implement M18”,
  “ok implement”, “deliver M17”, /implement-m, or “implement the plan”.
---

# Implement the current milestone

Code + walkthrough for **what `docs/M{N}-plan.md` already locked**. Not `/spec`, not tag unless asked.

If the user has not numbered N, N = the INDEX row **Planned — next**. Stop if that plan file is missing (run `/spec` first) or if status is already `implemented`.

Honor standing constraints in `.grok/skills/spec/SKILL.md` unless this plan picked a bump (wire/ckpt, shipping `default.toml` / `coop.toml`, new `ControlVerb`).

## Step 1 — Read the plan

Read `docs/M{N}-plan.md` end to end. Implement **In scope** only. **Out of scope** stays later. The plan wins on timing vs long-term specs. Follow its Risks (postcard enum append, double-apply on `--load`, protocol bump).

Work order = the plan’s PR sections, in one working tree (not separate git PRs unless they ask).

## Step 2 — Code

- Match locked TOML/CLI/overlay syntax, who/when, and hash effects.
- Create `configs/` files the plan **named**. Do not invent extra overlays. Do not change shipping `configs/default.toml` or `configs/incentives/coop.toml` unless that is the slice.
- Postcard enums: **append only** (new variants at the end). Overlay TOML is not `ExperimentConfig` postcard.
- Tests for every row in the plan’s Tests table. `cargo test -- --exact NAME` takes **one** name. Integration: `--test governance` (or `incentives`, `checkpoint`, …). Unit: `--lib path::tests::…`.
- Default mock hashes match pre-M{N} unless this slice changes defaults.
- CI `provider = mock`; `cargo test` never needs the network.

## Step 3 — Walkthrough + run

Write `docs/M{N}-test-plan.md` in the recent shape:

- Title `M{N} test plan — see each new feature`; link the plan; `--exact` gotcha; **Success for the slice** paragraph (`format_version` / `PROTOCOL_VERSION` as locked).
- §0 safety net (`cargo test -p sim-core`, plus `-p viewer` / `-p sim-cli` if this slice touched them; plus `cargo test -p sim-cli --test net` and `cargo test -p shared` — TCP/WS loopback — every deliver).
- One numbered section per in-scope item: exact `cargo test` lines, success table, `sim-cli` / viewer commands a researcher can **read**.
- Live/overnight recipes as copy-paste commands. **Do not run** them unless the user asked.
- Next-slice line: leftover inventory at [`docs/remaining-features.md`](docs/remaining-features.md). Do **not** point at `M{N+1}-plan.md` or “the bottom of `M{N}-plan.md`”. Do **not** rewrite Next-slice lines on historical test plans.
- **Execution record:** every non-GUI walkthrough command actually run, with pass/fail evidence.

**Run every non-GUI line in that walkthrough before you stop.** That means every `cargo test` (package §0, each `--exact` name, `-p sim-cli --test net`, `-p shared`) and every headless `cargo run -p sim-cli` (default hash, `--compare`, inject). `cargo test -- --exact NAME` still takes **one** name — invoke each listed line separately. If a test fails, **fix the code** (do not paper over it in the walkthrough).

Skip only what cannot run headless or needs a live model: imgui/viewer window, overnight Spark / live LLM. Mark those **not run** in the execution record. Do not skip attach loopback, named `--exact` tests, or the default-hash `sim-cli` run.

## Step 4 — Retarget docs

| Place | Change |
|---|---|
| `docs/M{N}-plan.md` | Status: `implemented`; add **Walkthrough** link to `M{N}-test-plan.md` |
| `README.md` | Current slice **M{N}** + walkthrough (spec left this on M{N-1}) |
| `docs/00-INDEX-AND-HANDOFF.md` | M{N} **Done (tag `M{N}`)** + walkthrough; file-list row for `M{N}-test-plan.md`; suggested next = M{N} implemented, leftovers in `remaining-features.md` |
| `docs/remaining-features.md` | move this slice’s **Scheduled** rows → **Done** (newest first) with `M{N}` linking `M{N}-plan.md`; create the Done table if this is the first row. Add implement-discovered leftovers to **Open**. Do **not** delete RF ids. **Standing** stays. |
| Spec `Current slice:` banners | **do not touch** |
| Historical `M*-plan.md` later-tables | **do not touch** |
| Historical `M*-test-plan.md` Next-slice lines | **do not touch** |
| `docs/incentive-schedule-format.md` | only if this slice changed the shipping format |

Do not invent `docs/M{N+1}-plan.md`.

## Step 5 — Stop

Summarize what shipped and which walkthrough sections were **not** run (only imgui / live Spark, unless they asked). Offer commit / push / tag. Do not `/spec` the next slice.

## Commit / tag (if they ask)

```
Deliver M{N}: <short slice name>.
```

Annotated git tag `M{N}` on that deliver commit **only** if they asked to tag. Docs-only `/spec` commits are never tagged `M{N}`.
