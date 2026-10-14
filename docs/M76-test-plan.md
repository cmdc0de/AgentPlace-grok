# M76 test plan — see each new feature

Walkthrough for [`M76-plan.md`](M76-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (wasm32 on every CI OS, girt/sleeve/socle/pretzel). Next slice: leftover inventory at [`docs/remaining-features.md`](docs/remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test ci` (or `objects`, `net`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `9abb3e07…`. Default (time on) shipped-objects `e3f2686b…`. `.github/workflows/test.yml` builds `sim-wasm` for `wasm32-unknown-unknown` on Ubuntu, Windows, and macOS with no `runner.os == 'Linux'` gate. The YAML does not change `state_hash`. Craft girt/sleeve/socle/pretzel. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. wasm32 on every matrix OS

```bash
cargo test -p sim-core --test ci github_actions_workflow_exists -- --exact --nocapture
cargo build -p sim-wasm --target wasm32-unknown-unknown
```

The second line runs only when `wasm32-unknown-unknown` is already installed. Do not `rustup target add` (that needs the network).

| Test | Success |
|---|---|
| workflow | three OS names; `wasm32-unknown-unknown`; no Linux-only `if` |
| local wasm32 build | `cargo build` exit 0 if the target is installed |
| Win/mac Actions | **not run** locally |

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_girt -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_sleeve -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_socle -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_pretzel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | girt / sleeve / socle / pretzel |
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
| shipped `--no-time` | `9abb3e07…` |
| default time on | `e3f2686b…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui / Win/mac Actions: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-14, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Local wasm32 build used the already-installed target (no `rustup target add`). Win/mac Actions **not** run.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 188/188; ci 1/1 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 49/49 |
| §0 `cargo test -p shared` | **pass** | 15/15 |
| §1 workflow `--exact` | **pass** | three OSes; wasm32 target; no `runner.os == 'Linux'` |
| §1 `cargo build -p sim-wasm --target wasm32-unknown-unknown` | **pass** | exit 0 in 9.40s |
| §2 recipes `--exact` ×5 | **pass** | Craft girt/sleeve/socle/pretzel; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=e3f2686b9dd1dbe365b288f07865ffb180391b6b16e1a5adf1db46ac4ec27019`; `--no-time` `final_hash=9abb3e07f8481dd7958ec56526f4a8c22253e9e7ff7a8fd0b04651163d0dcd5e` |
| Win/mac Actions | **not run** | no remote runners from this session |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
