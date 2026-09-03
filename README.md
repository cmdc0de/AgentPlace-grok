# AgentPlace-grok

Deterministic multi-agent simulation. Current slice: **M36** (`docs/M36-plan.md`). Walkthrough: [`docs/M36-test-plan.md`](docs/M36-test-plan.md).

Needs (when an agent is hungry/thirsty/tired): [`docs/needs-and-survival.md`](docs/needs-and-survival.md). Incentive TOML: [`docs/incentive-schedule-format.md`](docs/incentive-schedule-format.md). Post-GA backlog: [`docs/post-ga-feature-list.md`](docs/post-ga-feature-list.md).

## Test

Unit / integration (no network; mock LLM):

```bash
cargo test -p sim-core
cargo test -p sim-core --test social
cargo test -p sim-core --test governance
```

Headless 80-tick run with checkpoints, food report, event log, and decision log:

```bash
rm -rf /tmp/m5
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --out-dir /tmp/m5 --checkpoint-every 40 --report --llm mock --quiet
```

Expect `final_tick=80` and files `{id}_events.jsonl`, `{id}_decisions.jsonl`, `{id}_tick_40.ckpt`, `{id}_tick_80_report.md`. Decision lines should equal agents × ticks (16 × 80 = 1280 with the default config):

```bash
ID=$(basename /tmp/m5/*_tick_80.ckpt _tick_80.ckpt)
test "$(wc -l < /tmp/m5/${ID}_decisions.jsonl)" -eq 1280
```

Continuation identity (load tick 40, run 40 more; hash must match a continuous 80-tick run):

```bash
ID=$(basename /tmp/m5/*_tick_80.ckpt _tick_80.ckpt)
cargo run -p sim-cli -- --load /tmp/m5/${ID}_tick_40.ckpt --ticks 40 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet
# both prints should share the same final_hash as the first 80-tick run
```

Regenerate the report from a checkpoint (no extra ticks):

```bash
ID=$(basename /tmp/m5/*_tick_40.ckpt _tick_40.ckpt)
cargo run -p sim-cli -- --load /tmp/m5/${ID}_tick_40.ckpt --ticks 0 --report --out-dir /tmp/m5-report
```

3D viewer (imgui: `H` help, `L` legend, `I` inspector, `B` board, `/` console, `O` fog; follow `F` / `0`–`9`; `Esc` quit — window layout saved to `checkpoints/`):

```bash
cargo test -p viewer
cargo run -p viewer -- --config configs/default.toml
# /report   writes a report under checkpoints/
# /follow 0 then O   match that agent's Observation in 3D
cargo run -p viewer -- --load /tmp/m5/${ID}_tick_80.ckpt
# or a run directory: --load /tmp/m5   then slider / [ ] / /scrub TICK / /ckpt next|prev
```

Attachable clients (TCP and WebSocket; read-only attach does not change `state_hash`). Token is LAN auth, not TLS (`wss` is later):

```bash
# terminal A
cargo run -p sim-cli -- --config configs/default.toml --ticks 100000 \
  --listen tcp://127.0.0.1:9000 --listen ws://127.0.0.1:9001 \
  --allow-control --start-paused --quiet

# terminal B
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# or: cargo run -p viewer -- --connect ws://127.0.0.1:9001
# headless log tail: cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --quiet
# stdin Control: cargo run -p sim-cli -- --connect tcp://127.0.0.1:9000 --allow-control
```

Opt-in remote pause/step/save/report/inject/`/scrub`: add `--allow-control` (and optional `--token SECRET` on both sides).

Incentive A/B (postcard wire `PROTOCOL_VERSION = 5`; remote `/give` `/set` `/ckpt` `/events` `/scrub` with `--allow-control`. `--lockstep` waits for `AckTick`. Timing is not hashed).

**A schedule is one TOML file with one or more `[[incentives]]` tables** — not one file per incentive. `--inject` / `/inject` replace the whole schedule with that file. Field reference: [`docs/incentive-schedule-format.md`](docs/incentive-schedule-format.md). Example: [`configs/incentives/coop.toml`](configs/incentives/coop.toml). Hidden banners: `visibility = "hidden"` (effects still apply; LLM/Observation omit the id). Covert payoff example: [`configs/incentives/hidden-bonus.toml`](configs/incentives/hidden-bonus.toml). Coalition targeting: `applies_to = "supporters_of:proposal_N"` ([`configs/incentives/coalition.toml`](configs/incentives/coalition.toml)) — current supporters get per-tick effects (e.g. 1.4× food). Votes default to **equal** (one living agent = 1). Overlay `[voting] weight = "influence"` uses `max(influence_factor, 1)` millipoints; pair with [`configs/incentives/leadership.toml`](configs/incentives/leadership.toml) to A/B a kingmaker. Overlay `[voting] weight = "respect"` uses incoming positive respect from other living agents (floor 1); pair with [`configs/incentives/esteem.toml`](configs/incentives/esteem.toml) (`relationship_delta` toward `agent:0`). Default respect 0 ⇒ same as equal. Overlay `[voting] accept = "unanimous"` or `"council"` (with `council = [0, 1]`) is stance-complete; `weight` applies to majority and to `council_tally = "majority"`. Adopted `SetCouncil` rewrites the roster when `allow_meta_rules`; `SetCouncilTally` switches unanimous vs majority-of-council. `/scrub TICK` loads the ckpt at-or-before T then ticks forward to T (in-process, or remote with `--allow-control`). Worn pack meshes scale with fill (viewer-only). `sim-cli --connect` is a Welcome/Tick hash tail; add `--allow-control` to send `/play` `/give` etc from stdin. Attached 3D applies every Snapshot in order (Status shows world vs live if catching up). Overlay `[llm] barrier = true` retries timeout/parse (default 3 extra) then Wait. Viewer `/set ID hunger|thirst|energy|influence N` and `/set ID respect TOWARD N` work in-process and remote with `--allow-control` (display 0–100 → millipoints ×100). `sim-cli --connect --allow-control` `/inject PATH` sends the schedule. `--lockstep` waits for each subscriber to `AckTick` after paint (`--lockstep-timeout-ms N` continues after N ms). Overlay `[llm] reflect_on_evict = true` summarises dropped memories via an extra LLM call (mock skips). Overlay `[storage] max_worn_baskets` / `max_worn_backpacks` (omit = 1,1) stacks worn pack caps; extras stay cargo. Leave and incentive end revert `relationship_delta` as well as influence (goals stay). `public_board_always_visible = false` shows open proposals in identity range (plus author / own stance).

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml \
  --incentives configs/incentives/coop.toml --ticks 80 --llm mock --quiet
# hashes should differ; a second treated run should match the first treated hash

cargo run -p sim-cli -- --config configs/default.toml --ticks 40 \
  --out-dir /tmp/m8 --checkpoint-every 40 --llm mock --quiet
ID=$(basename /tmp/m8/*_tick_40.ckpt _tick_40.ckpt)
cargo run -p sim-cli -- --load /tmp/m8/${ID}_tick_40.ckpt \
  --inject configs/incentives/coop.toml --ticks 40 --llm mock --quiet

# viewer: /inject configs/incentives/coop.toml
```

```bash
cargo test -p shared
cargo test -p sim-core
cargo test -p sim-cli --test net
cargo test -p sim-cli --test ab
```

Optional live LLM (not required for CI). Default `base_url` is `http://spark-bcce.hlab:11434`, model `nemotron3:33b` (`timeout_ms = 120000`). Empty `base_url` ⇒ mock (no invented host). Mid-run HTTP failure is `Wait`, not mock. Replay JSONL stores raw model text; `{"__llm_wait__":true}` re-emits `LlmWait`. Researcher walkthrough: [`docs/M17-test-plan.md`](docs/M17-test-plan.md). Plan: [`docs/M17-plan.md`](docs/M17-plan.md). Overlay `[proposals] allow_meta_rules = true` lets Accepted structured rules set lifetime, threshold, or vote weight/accept. `supporters_of` one-shots follow join/leave (influence and relationship_delta revert on leave and end; goals stay). Viewer `/events TICK` filters `{id}_events.jsonl` without loading a ckpt.

Shared land-cell **crates** (slot 16, weight 80): `Store` / `Retrieve` / `Transfer` cost energy ∝ item weight (pack source is cheaper, haul 0.1 vs 0.4). **Basket** is a worn backpack (8 slots, weight 25); `Pack` / `Unpack`; Move pays cargo (`loose × 0.4 × 0.05 + pack × 0.1 × 0.05`). Viewer: brown crate on the cell; darker satchel on the capsule when a Basket is held. `/give ID ITEM QTY` in-process only (hash-sensitive). Mock + coop storage goal **Gathers then Stores** (Eat only below 50% hunger while that goal is on).

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm ollama
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm openai_compatible
```

`--compare` two `--out-dir`s or `.ckpt` files (hash, deaths, board, consumption, needs, goals, incentives, event histogram):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock \
  --out-dir /tmp/base --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 --llm mock \
  --incentives configs/incentives/coop.toml --out-dir /tmp/coop --quiet
cargo run -p sim-cli -- --compare /tmp/base /tmp/coop
```
