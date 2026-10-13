# M74 test plan — see each new feature

Walkthrough for [`M74-plan.md`](M74-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (`system1_model`, batten/wimple/voussoir/bannock). Next slice: leftover inventory at [`docs/remaining-features.md`](remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test barrier` (or `objects`, `net`). sim-llm unit tests use `tests::NAME`. Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `22e66700…`. Default (time on) shipped-objects `62b5ce21…`. `[llm] system1_model` is the choose-call model only. Insight/plan/reflect/invent flavor/importance stay on `model`. Grok choose bodies set `response_format`; non-grok bodies set `think: false`. Mock and wait ignore the key and do not open a socket. The key does not change `state_hash` or `config_hash`. `--load` does not add events. Craft batten/wimple/voussoir/bannock. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p sim-llm
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. System-1 model id

```bash
cargo test -p sim-llm tests::choose_model_uses_system1_other_calls_do_not -- --exact --nocapture
cargo test -p sim-llm tests::chat_body_follows_the_call_model -- --exact --nocapture
cargo test -p sim-llm tests::mock_and_wait_ignore_system1_model -- --exact --nocapture
cargo test -p sim-core --test barrier system1_model_parses_and_does_not_change_hashes -- --exact --nocapture
cargo test -p sim-core --test barrier load_does_not_extra_choose -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit | choose id = `model` |
| set | choose id = trimmed `system1_model`; other calls stay on `model` |
| grok body | `response_format`, no `think` |
| non-grok body | `think: false`, no `response_format` |
| mock / wait | `Chooser::Mock` / `Wait` |
| hashes | `state_hash` and `config_hash` match omit |
| `--load` | tick and event count unchanged |

Researcher (not run; needs a live model):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm ollama --llm-system1-model grok-4-fast
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_batten -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_wimple -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_voussoir -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_bannock -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | batten / wimple / voussoir / bannock |
| catalog-off | Catalog crafts illegal |

---

## 3. Hello / format / idle CLI

```bash
cargo test -p sim-core --test objects no_time_shipped_objects_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --no-time
```

| Test | Success |
|---|---|
| no-objects `--no-time` | `70e5204d…` |
| shipped `--no-time` | `22e66700…` |
| default time on | `62b5ce21…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-13, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Live model **not** called.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 175/175; barrier 9/9 |
| §0 `cargo test -p sim-llm` | **pass** | 10/10 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 system1 `--exact` ×5 | **pass** | choose uses trimmed id; other calls stay on `model`; grok body has `response_format`; non-grok has `think: false`; mock/wait ignore the key; `state_hash` and `config_hash` match omit; `--load` tick and event count unchanged |
| §2 recipes `--exact` ×5 | **pass** | Craft batten/wimple/voussoir/bannock; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=62b5ce21a511ab7f433c81bd47d352dce6e341b44dfdabf9b65f2250219f44ab`; `--no-time` `final_hash=22e66700f38f673ebce8bae99438cc90666eb865a1a4ffc7821710cd26303fcb` |
| live Ollama / xAI | **not run** | not asked |
| viewer window | **not run** | no display |
