# AgentPlace-grok

Deterministic multi-agent simulation. Current slice: **M7** (`docs/M7-plan.md`).

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

Opt-in remote pause/step/save/report: add `--allow-control` (and optional `--token SECRET` on both sides).

```bash
cargo test -p shared
cargo test -p sim-cli --test net
```

Optional live LLM (not required for CI):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm ollama
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm openai_compatible
```
