# CLI reference

Flags for `sim-cli` (headless runner) and `viewer` (Bevy window). Overlay keys that the same flags turn on live in [`config-reference.md`](config-reference.md). Incentive files: [`incentive-schedule-format.md`](incentive-schedule-format.md).

All commands assume the **repo root**. `cargo test` / `--llm mock` never need the network.

`--listen` and `--connect` are mutually exclusive. `--start-paused` requires both `--listen` and `--allow-control`. Overlay TOML is **not** `ExperimentConfig` and is **not** hashed unless the flag’s description says it is.

`PROTOCOL_VERSION = 5`. Checkpoints write `format_version` **3** and still read v2.

---

## sim-cli

```bash
cargo run -p sim-cli -- --help
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

Default `--config` is `configs/default.toml`. Default `--ticks` is **100** (not `max_ticks` in the TOML). `--ticks 0` with `--load` inspects without stepping.

### Run / I/O

| Flag | Default | What it does |
|---|---|---|
| `-c`, `--config PATH` | `configs/default.toml` | Experiment TOML. Hashed tables become `ExperimentConfig`. Overlay tables (`[network]`, `[incentives]`, `[inventions]`, …) are read from the same file. |
| `-n`, `--ticks N` | `100` | How many ticks to run this process. With `--load`, ticks **after** restore. `0` + `--load` = inspect only. |
| `-q`, `--quiet` | off | Print `final_tick`, `final_hash`, and last/mean `tick_ns` only. |
| `-o`, `--out-dir DIR` | off | Write checkpoints, Markdown summaries, JSONL events/decisions, optional timing JSONL. |
| `--sqlite PATH` | off | Extra sink: same events/decisions/timing facts as JSONL, **typed columns** (no JSON blob). Does not replace JSONL. Hash-neutral. |
| `--sqlite-http HOST:PORT` | off | Serve `GET /metrics` JSON from `--sqlite` (CORS `*`). Requires `--sqlite`. Hash-neutral. |
| `--width N` / `--height N` | TOML `[world]` (shipping 64×64) | Override map size for a **new** sim. **32..=256**. Hashed. Ignored with `--load` / `--connect`. |
| `--otlp-endpoint URL` | overlay `otlp_endpoint` empty | POST OTLP/JSON `wall_ns` + RSS to `{url}/v1/metrics`. Implies `--telemetry`. Hash-neutral. No TLS. |
| `--checkpoint-every K` | config `[checkpoint] auto_interval_ticks` if `--out-dir` | Checkpoint every *K* ticks. Implies an out-dir (config `directory` if `-o` omitted). |
| `--load PATH` | off | Restore a `.ckpt` and continue. Overlay flags (`--inventions`, `--pipeline-events`, `--catalog`, …) still apply **after** decode (do not double-grant). |
| `--summarize` | off | Print the Markdown world summary to stdout. With `--listen`, emit when the listen session ends. |
| `--report` | off | Food-economy report (md/csv). Prints markdown if no `--out-dir`. With `--listen`, emit when the listen session ends (does **not** skip the bind). |
| `-h`, `--help` | | This flag list (same as the binary help). |

### LLM

| Flag | Default | What it does |
|---|---|---|
| `--llm PROVIDER` | config `[llm] provider` (shipping = `mock`) | `mock` \| `wait` \| `ollama` \| `openai_compatible`. Empty `[llm] base_url` ⇒ mock. CI stays mock. |
| `--llm-barrier` | overlay `[llm] barrier` = false | Retry timeout/parse then Wait. Remaining agents still finish the tick. |
| `--llm-barrier-retries N` | overlay `3` | Extra attempts after the first. Implies barrier. `0` = one attempt. |
| `--llm-reflect-on-evict` | overlay false | LLM summarises a dropped memory (skipped under mock). |
| `--llm-reflect-every N` | overlay `0` (off) | Insight call every *N* ticks. |
| `--llm-plan-every N` | overlay `0` (off) | Short-term plan every *N* ticks. |
| `--llm-execute-plan` | overlay false | If `plan[0]` is legal action JSON, execute it and skip choose. |
| `--llm-reflect-importance` | overlay false | Extra call after retrieve may rewrite a memory’s importance. |

### Network / attach

| Flag | Default | What it does |
|---|---|---|
| `--listen URL` | overlay `[network] tcp_listen` / `ws_listen` | Repeatable. `tcp://host:port` and/or `ws://host:port`. **No TLS / no `wss`**. Always binds; `--report` / `--summarize` do not skip it. Prints `listen={url}` on **stderr** after the socket is open (still printed with `--quiet`). |
| `--connect URL` | off | Attach as a client: print Welcome/Tick hash tail. Hash-neutral without `--allow-control`. |
| `--allow-control` | overlay `[network] allow_control` | Listen: accept Control. Connect: stdin slash commands send Control. |
| `--start-paused` | off | Listen without ticking until a client sends Play. Needs `--listen` **and** `--allow-control`. Viewer `--connect` does **not** auto-unpause. |
| `--lockstep` | overlay `[network] lockstep` | After each tick, wait for `AckTick` from every subscriber (needed to **paint every tick**). |
| `--lockstep-timeout-ms N` | overlay `0` = forever | Give up waiting for Ack after *N* ms, then advance. |
| `--token SECRET` | overlay `[network] token` | Hello must match (LAN auth, not TLS). |

### Overlay feature flags

These are **or** with the same-named overlay table. `[time]` **omit = true** (clock on by default). `--no-time` no-objects is `70e5204d…`. `--no-time` shipped-objects 2-tick is `3170f273…` (M62 stations + recipes). Default (time on) shipped-objects is `7f2d52db…`.

| Flag | Overlay | What it does |
|---|---|---|
| `--conflict` | `[conflict] enabled` | Attack / Flee legal. |
| `--conflict-death` | `[conflict] death_enabled` | Health 0 **removes** the agent (implies `--conflict`). Off = incapacitate only. |
| `--sheet` | `[agents.sheet] enabled` | Roll founder STR/DEX/CON/INT/WIS/CHA. Score 0 ⇒ modifier 0 / today’s constants. |
| `--reproduction` | `[population] reproduction` | PairBond / Reproduce. Implies `--sheet`. Mock does not pick them. |
| `--aging` | `[population] aging` | Accrue `age_ticks`. Childhood gates PairBond / Reproduce / Attack. |
| `--household-crates` | `[population] household_crates` | PairBond mints a home cell; members Store/Retrieve within Chebyshev 1. Catalog on ⇒ also Place a cabin if 2×2 land is free. |
| `--culture` | `[population] culture` | Founder culture ids `1..=culture_count`; children copy a parent. |
| `--inventions` | `[inventions] enabled` | Invent GatherBonus → MoveBonus → SenseBonus → CraftBonus → RestBonus. Does **not** imply `--sheet`. `tree` / `patent_ticks` come from overlay only. |
| `--pipeline-events` | `[pipeline] hash_events` | One hashed `Pipeline { stages }` per living agent per tick (complete mock = `31`). Does **not** hash wall-clock ns. |
| `--catalog` | `[catalog] enabled` | Hash `[sim]` catalog items loaded from `--objects`. Does not imply `--sheet`. Empty catalog ≡ off for hash. |
| `--objects DIR` | default `configs/objects` if present | Object-definition TOML directory (visuals + hashed `[sim]` when catalog on). |
| `--telemetry` | `[telemetry] enabled` | In-process tick aggregates + RSS + CPU ns/percent + disk + optional `--out-dir` bytes. Hash-neutral. |
| `--time` | `[time] enabled` omit = **true** | Day/night clock (hashed `ticks_per_day`). Redundant with the default. |
| `--no-time` | `[time] enabled = false` | Disable the clock. Rest only. M51 hashes. Wins over `--time`. |

### Incentives / compare

| Flag | What it does |
|---|---|
| `--incentives PATH` | Apply that schedule from tick 0 (overlay, not hashed). |
| `--inject PATH` | **Replace** the current schedule (typical with `--load`). Does not merge files. |
| `--compare A B` | Diff two `--out-dir` folders or `.ckpt` files (markdown). |
| `--csv` | With `--compare`, also print CSV. |

### Typical recipes

Idle hash with shipped objects (time **on** by default):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
# final_hash=7f2d52db…
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
# final_hash=3170f273…
```

`--no-time` without `configs/objects` is `70e5204d…`. Overlay-off telemetry does not change hashes. Night makes Hunt/Farm illegal. `Place` / `Pickup` tent/cabin/house on land (N×N) and millstone/spit 1×1 stations. Spear/sling/bow have Attack range. Hoe/net/axe/hammer raise Farm/Fish/vegetation-Gather/stone-Gather odds and break after 8 uses (bonus-use **or** dawn while held). Transfer and Store move that instance’s wear with the item. Flour Craft needs a placed millstone; bread/stew need a placed spit. Native viewer `--otlp-endpoint` POSTs `agentplace.viewer.frame_ns`. sim-cli OTLP includes `agentplace.process.cpu_percent` and `agentplace.process.out_dir_bytes`.

Listen paused for the viewer (Play in the window / `/play` from a control client):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --quiet
# wait for stderr: listen=ws://127.0.0.1:9001  (cargo compile + sim init happen first)
```

M45 overlays:

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --inventions --pipeline-events --catalog --quiet
```

---

## viewer

```bash
cargo run -p viewer -- --config configs/default.toml
cargo run -p viewer -- --connect ws://127.0.0.1:9001
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
cargo run -p viewer -- --load checkpoints/….ckpt
```

| Flag | Default | What it does |
|---|---|---|
| `-c`, `--config PATH` | `configs/default.toml` (or `../` / `../../` from the crate) | Start a **local** sim from experiment TOML. Overlay tables in that file apply (inventions, pipeline, catalog, sheet, …). |
| `--load PATH` | off | Load a `.ckpt` or a run directory (latest tick at or before the scrubber). Mutually preferred over `--config` if both are passed (`--connect` wins first). |
| `--connect URL` | off | Attach to a listening `sim-cli`. Does **not** unpause a `--start-paused` server. Play/Pause/Step **always send Control**; the **server** `--allow-control` is the gate (the viewer has no `--allow-control` flag). |
| `--token SECRET` | off | Hello token for `--connect`. |
| `--objects DIR` | shipped `configs/objects` (cwd, then crate path) | Object TOML for meshes / catalog. Needed for glb; without defs the viewer uses primitives and prints `objects: no configs/objects dir`. |
| `--catalog` | off | Enable catalog hashing the same as sim-cli `--catalog`. |
| `--time` / `--no-time` | clock on | Same as sim-cli. `--no-time` on `--config` / `--load` / `--connect`. |
| `--width N` / `--height N` | TOML | In-process `--config` only. Ignored with `--load` / `--connect`. 32..=256. |
| `--otlp-endpoint URL` | off | POST last frame ns as OTLP/JSON `agentplace.viewer.frame_ns` (`service.name=agentplace-viewer`). Hash-neutral. In-process and `--connect`. |

There is no viewer `--listen`, `--start-paused`, or `--pipeline-events`. Put those on `sim-cli`, or in the experiment overlay when the viewer **hosts** the sim (`--config`).

`--lockstep` on the **server** is what makes the attached viewer paint every tick.

On startup the viewer prints each unique authored glb to stderr (and the Log pane): path, **file bytes**, then in-game AABB meters once the mesh is ready (`loaded glb …` / `model … size=… m`). A **configured** path that is missing logs `glb miss {id} -> sentinel` and uses a magenta cuboid. Empty / omitted `[visual]` still uses the primitive (not logged). Agent meshes come from `configs/objects/agent.toml` (`[visual]`); no visual ⇒ capsule. Authored agent glbs are uniformly scaled to the capsule height (1.11) once the AABB is known. Changing a loaded `.glb` on disk reloads it (~0.5 s) and logs `glb reload {id} {path}`.

| Key | What it does |
|---|---|
| Arrow keys | Pan the 3D camera on XZ at the current height (native window). First pan cancels `/follow`. |
| `u` | Raise camera (+Y). |
| `d` | Lower camera (−Y), clamped above terrain. **Not** `L` (legend). |
| `L` | Toggle legend. |
| `C` | Toggle Charts (last 256 ticks: wall_ms, living, hungry, thirsty, mean hunger). |
| `F` / `0`–`9` | Follow / follow agent id. |

---

## Slash commands (viewer console and `sim-cli --connect` stdin)

Prefix `/` is optional in the viewer parser.

**Who may mutate:** the **server** `--allow-control` is the gate (`ControlDisabled` without it). `sim-cli --connect` only **sends** Control if you also pass `--allow-control` (otherwise it is a hash-neutral log tail). The **viewer** has no such flag: imgui Play/Pause/Step and remote slash commands always send Control. So `cargo run -p viewer -- --connect ws://…` can Play a server that was started with `--allow-control`. For a read-only 3D attach, omit `--allow-control` on the **server** (then Play is refused). `--start-paused` still requires server `--allow-control`, so a paused listen is always a control-capable server.

| Command | Local viewer | Remote Control (needs `--allow-control`) |
|---|---|---|
| `/help` | List commands | — |
| `/pause` | Pause | Pause |
| `/play` / `/unpause` | Resume | Play |
| `/step` / `/step N` | Advance N ticks (default 1) | Step |
| `/follow ID` / `/follow off` | Camera follow | — |
| `/fog` / `/fog on\|off` | Agent-POV fog | — |
| `/legend` `/inspector` `/board` `/log` `/charts` | Toggle imgui panes | — |
| `/tick` | Show tick / hash | — |
| `/summarize` | World Markdown | Summarize |
| `/report [DIR]` | Food report | Report |
| `/save [PATH]` | Write a checkpoint | Save |
| `/inject PATH` | Replace incentive schedule | Inject (connect path) |
| `/scrub TICK` | Jump to checkpoint at or before TICK | Scrub |
| `/ckpt next` / `/ckpt prev` | Next/previous `.ckpt` in the run dir | CkptNext / CkptPrev |
| `/events TICK` | Print JSONL events for that tick (display-only) | Events |
| `/give ID ITEM [QTY]` | Add to pockets (hash-sensitive) | Give |
| `/set ID hunger\|thirst\|energy\|influence N` | 0–100 display → millipoints | Set |
| `/set ID respect TOWARD N` | Directed respect | Set |

`/give` item names are species / builtin slugs (`berry_bush`, `wood`, `basket`, …). Unknown item or qty 0 is an error.

---

## Hash / overlay rules (every flag)

- Shipping `configs/default.toml` / `configs/incentives/coop.toml` unchanged unless a milestone says otherwise.
- Overlay off + mock ⇒ same hash as no flag. Default CLI (time on, loads `configs/objects`) 2-tick hash `7f2d52db…`. `--no-time` shipped-objects is `3170f273…`.
- Visuals / glb / LOD / imgui / wall-clock ns are **never** hashed.
- Catalog-on hashes `[sim]` (including recipes). Extra catalog files can change catalog-on hashes; v3 checkpoints store **slugs** so holdings remap.
- Do not pass `--llm ollama` in CI.
