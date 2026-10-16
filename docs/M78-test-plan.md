# M78 test plan — see each new feature

Walkthrough for [`M78-plan.md`](M78-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (browser Bevy primitives, rafter/coif/corbel/cruller). Next slice: leftover inventory at [`docs/remaining-features.md`](docs/remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. The viewer-web host test is `tests::agent_marks_match_agents`. Integration tests need `--test ci` (or `objects`, `net`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `7aad440e…`. Default (time on) shipped-objects `7ea74fe4…`. `agent_marks` returns one cell per agent and opens no window. The CI workflow builds `viewer-web` for `wasm32-unknown-unknown`. The browser scene does not change `state_hash`. Craft rafter/coif/corbel/cruller. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` and the native `viewer` crate are unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p viewer-web
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Browser Bevy primitives

```bash
cargo test -p viewer-web tests::agent_marks_match_agents -- --exact --nocapture
cargo test -p sim-core --test ci github_actions_workflow_exists -- --exact --nocapture
cargo build -p viewer-web --target wasm32-unknown-unknown
```

The build runs only when `wasm32-unknown-unknown` is already installed. Do not `rustup target add` (that needs the network). Do not open the page.

| Test | Success |
|---|---|
| `agent_marks` | 2 marks, each an agent cell; no window |
| workflow | names `viewer-web` |
| wasm32 build | exit 0 if the target is installed |
| live canvas / Win/mac Actions | **not run** |

Researcher (not run; needs a built `pkg/viewer_web.js` and a browser):

```text
open web/index.html
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_rafter -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_coif -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_corbel -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_cruller -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | rafter / coif / corbel / cruller |
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
| shipped `--no-time` | `7aad440e…` |
| default time on | `7ea74fe4…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui / the browser canvas: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-14, repo root. Each `--exact` line invoked separately. Package-level §0 also run. The wasm32 target was already installed, so the build did not call `rustup target add`. The page was not opened.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 196/196 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p viewer-web` | **pass** | 1/1 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 51/51 |
| §0 `cargo test -p shared` | **pass** | 17/17 |
| §1 `agent_marks` / workflow | **pass** | 2 agent cells; workflow names `viewer-web` |
| §1 `cargo build -p viewer-web --target wasm32-unknown-unknown` | **pass** | exit 0 (cached after the first successful compile) |
| §2 recipes `--exact` ×5 | **pass** | Craft rafter/coif/corbel/cruller; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=7ea74fe4471b1463de4350ec5ba009e9d74dfb0c609a49c985052425aa20598a`; `--no-time` `final_hash=7aad440edeb779612a07d91f0a82cb4f178bd008cbd55a9018c23c57f22abf4c` |
| browser canvas | **not run** | no `pkg/viewer_web.js` glue and no display |
| Win/mac Actions | **not run** | no remote runners |
| live Spark / overnight | **not run** | not asked |
