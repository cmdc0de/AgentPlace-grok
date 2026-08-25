# AgentPlace-grok

Deterministic multi-agent simulation. Current slice: **M5** (`docs/M5-plan.md`).

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

3D viewer (legend down the left, `L` to hide; follow `F` / `0`–`9`):

```bash
cargo run -p viewer -- --config configs/default.toml
cargo run -p viewer -- --load /tmp/m5/${ID}_tick_80.ckpt
```

Optional live LLM (not required for CI):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm ollama
cargo run -p sim-cli -- --config configs/default.toml --ticks 20 --llm openai_compatible
```
