# AgentPlace-grok

Deterministic multi-agent simulation. Current slice: **M13** (`docs/M13-plan.md`). Walkthrough: [`docs/M13-test-plan.md`](docs/M13-test-plan.md).

Needs (when an agent is hungry/thirsty/tired): [`docs/needs-and-survival.md`](docs/needs-and-survival.md). Incentive TOML: [`docs/incentive-schedule-format.md`](docs/incentive-schedule-format.md).

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
```

Attachable clients (TCP and WebSocket; read-only attach does not change `state_hash`). Token is LAN auth, not TLS (`wss` is later):

```bash
# terminal A
cargo run -p sim-cli -- --config configs/default.toml --ticks 100000 \
  --listen tcp://127.0.0.1:9000 --listen ws://127.0.0.1:9001 --quiet

# terminal B
cargo run -p viewer -- --connect tcp://127.0.0.1:9000
# or: cargo run -p viewer -- --connect ws://127.0.0.1:9001
```

Opt-in remote pause/step/save/report/inject: add `--allow-control` (and optional `--token SECRET` on both sides).

Incentive A/B (postcard wire unchanged except `PROTOCOL_VERSION = 2` for `Tick.metrics`; timing is not hashed).

**A schedule is one TOML file with one or more `[[incentives]]` tables** — not one file per incentive. `--inject` / `/inject` replace the whole schedule with that file. Field reference: [`docs/incentive-schedule-format.md`](docs/incentive-schedule-format.md). Example: [`configs/incentives/coop.toml`](configs/incentives/coop.toml). Hidden banners: `visibility = "hidden"` (effects still apply; LLM/Observation omit the id). Covert payoff example: [`configs/incentives/hidden-bonus.toml`](configs/incentives/hidden-bonus.toml). Votes default to **equal** (one living agent = 1). Overlay `[voting] weight = "influence"` uses `max(influence_factor, 1)` millipoints; pair with [`configs/incentives/leadership.toml`](configs/incentives/leadership.toml) to A/B a kingmaker.

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

Optional live LLM (not required for CI). Default `base_url` is `http://spark-bcce.hlab:11434`, model `nemotron3:33b` (`timeout_ms = 120000`). Empty `base_url` ⇒ mock (no invented host). Mid-run HTTP failure is `Wait`, not mock. Replay JSONL stores raw model text; `{"__llm_wait__":true}` re-emits `LlmWait`. Researcher walkthrough: [`docs/M13-test-plan.md`](docs/M13-test-plan.md). Plan: [`docs/M13-plan.md`](docs/M13-plan.md).

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
