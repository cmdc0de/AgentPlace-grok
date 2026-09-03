# M32 — kin_of incentives, household, aging

**Status:** implemented  
**Depends on:** M31 complete (`docs/M31-plan.md`, git tag `M31`, commit `5827f21`)  
**Walkthrough:** [`M32-test-plan.md`](M32-test-plan.md)  
**Specs:** `docs/incentive-schedule-format.md` (`applies_to`), `docs/post-ga-feature-list.md` (PG-1 household), `docs/M31-plan.md` (kinship / reproduction)

## Context

M31 shipped parent/child/sibling/pair-bond and Reproduce. Incentives still cannot target a family. There is no household id. Agents do not age; children can PairBond/Reproduce/Attack immediately.

M32 **does not** bump `PROTOCOL_VERSION` (stays **5**). Overlay is not `ExperimentConfig`. No TLS/protobuf. No browser. Shipping `default.toml` / `coop.toml` unchanged.

## Goal

A researcher can:

1. Target a family with `applies_to = "kin_of:N"` (living parents, children, siblings, pair-bond of agent N — **not** N unless they also appear as a relative).
2. See **household** ids: pair-bond mints/joins a household; children inherit it. `applies_to = "household:H"` hits living members. Inspector shows the id.
3. Turn on overlay `[population] aging = true` (or `--aging`) so agents accrue `age_ticks`; children start at 0, founders at an adult floor. Below `childhood_ticks` they cannot PairBond / Reproduce / Attack. Overlay **off** ⇒ age stays 0, **not hashed**, idle hash still `70e5204d…`.
4. CI stays `provider = mock`. `format_version = 2`. **`PROTOCOL_VERSION = 5`**. Shipping `default.toml` / `coop.toml` unchanged.

## In scope

### A. `kin_of` (schedule overlay)

```toml
applies_to = "kin_of:0"          # also kin_of:agent_0
```

- Living parents, children, siblings, and pair-bond of agent N. **Not** N unless they appear in that map.
- `kin_of:nope` is a load error (same as `supporters_of:nope`). Missing agent id → empty scope.
- Per-tick effects follow the **live** set; one-shots on start/join as today.
- Update `docs/incentive-schedule-format.md` on implement. Example overlay created on implement: `configs/incentives/kin-bonus.toml`. Do **not** change `coop.toml`.

### B. Household

Field on `Kinship`, `#[serde(default, skip)]` + BoardBlob. Hash only if `Some`.

| When | What |
|---|---|
| PairBond | If either has a household, both use that id; if neither, mint `next_household_id` |
| Birth | Child copies a parent’s household |
| Inspector / Observation | `household #H` when set |

```toml
applies_to = "household:1"
```

`household:nope` is a load error. Missing id → empty scope. No extra overlay flag. Empty without pair-bond/birth ⇒ same hashes. No shared-crate rewrite this slice (scope targeting only). Checkpoint `next_household_id` in the board blob.

### C. Aging

Overlay, not `ExperimentConfig`:

```toml
[population]
aging = true
childhood_ticks = 80          # omit = 80
founder_age_ticks = 200       # omit = 200; founders start here when aging on
```

CLI: `--aging` (does **not** imply `--sheet` / `--reproduction`).

- `Agent.age_ticks: u64`, skip + BoardBlob. Hash only if ≠ 0.
- Overlay on: founders get `founder_age_ticks` once (do **not** re-stamp on `--load` if already non-zero). Each tick += 1 for living agents. Newborns start at 0.
- If `age_ticks < childhood_ticks`: PairBond, Reproduce, Attack **not legal** (Flee still is). Overlay off: no age, no gate.
- Inspector: `age T` (and `child` when below the floor).

## Out of scope (later)

| Later | What |
|---|---|
| **M33** | [`M33-plan.md`](M33-plan.md) — household crates, culture inheritance, reflect importance |
| After M33 | protobuf/TLS; Unix sockets; hashed pipeline events; **browser client** |
| Not M32 | PROTOCOL bump; flipping shipping `default.toml` / `coop.toml` |

## Key decisions

1. **`PROTOCOL_VERSION` stays 5.** No new `ControlVerb`. Age/household skip + blob like sheet.
2. `kin_of` does **not** include agent N. `household:H` is members only.
3. `--aging` does not imply sheet/reproduction. Childhood gate is off when aging overlay is off.
4. Empty household / age 0 add **no** hash bytes.
5. Do not change shipping `coop.toml`. `format_version = 2`.

## Tests (M32 acceptance bar)

| Test | Asserts |
|---|---|
| default mock 2 ticks | hash still `70e5204d…` |
| `kin_of:0` after PairBond+birth | parents/children/siblings in scope; agent 0 not unless listed as kin |
| `kin_of:nope` | load error |
| missing agent | empty scope |
| pair-bond | both share one household id; child inherits |
| `household:H` | only members |
| aging off | age 0; same hash as today |
| `--aging` | founders at floor; tick increments; child 0; below childhood cannot PairBond/Reproduce/Attack |
| `--load` + aging | does not re-stamp founder_age |
| Hello v5 | unchanged |

## PR Plan

### PR 1: `kin_of`

- **Files:** incentive `applies_to` parse + scope; format doc; `configs/incentives/kin-bonus.toml`

### PR 2: Household

- **Files:** `Kinship.household` + `next_household_id`; mint on PairBond; inherit on birth; `household:H`; inspector

### PR 3: Aging

- **Files:** overlay `[population] aging` / `--aging`; `age_ticks` blob; childhood gate; `--load` no re-stamp

## Config / CLI

No shipping TOML change. Overlay is not postcard. `--aging` does not imply `--sheet`. No PROTOCOL bump.

```bash
cargo test -p sim-core
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen tcp://127.0.0.1:9000 --allow-control --start-paused \
  --sheet --reproduction --aging \
  --incentives configs/incentives/kin-bonus.toml --quiet
```

## Verification

Walkthrough: written on implement (`docs/M32-test-plan.md`).

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

Expect: mock hashes unchanged with overlays off; kin_of/household scopes; aging gates children; Hello stays v5.

## Risks

- Overlay is **not** checkpointed; age/household **are**. `--load` must not reset founder age.
- `kin_of` / `household` parse must fail closed (`nope` = load error).
- Childhood gate must not fire when aging overlay is off.
- Empty household / age 0 add **no** hash bytes.
- **No PROTOCOL bump.** Stay at 5.
