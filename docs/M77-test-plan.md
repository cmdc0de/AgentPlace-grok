# M77 test plan — see each new feature

Walkthrough for [`M77-plan.md`](M77-plan.md). Automated tests prove the slice; the `sim-cli` steps below are what you **read** (Unix attach, baulk/collar/impost/rusk). Next slice: leftover inventory at [`docs/remaining-features.md`](docs/remaining-features.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Shared unit tests use `transport::tests::NAME`. Integration tests need `--test net` (or `objects`). Protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `--no-time` no-objects hash `70e5204d…`. `--no-time` shipped-objects `e624f373…`. Default (time on) shipped-objects `42e90831…`. `unix:///tmp/agentplace.sock` is path `/tmp/agentplace.sock`. `wss://` is still rejected. A Unix loopback Hello is protocol 5 and returns Welcome. Attach without `--allow-control` does not change `state_hash`. `--listen` and `--connect` together are an error. Craft baulk/collar/impost/rusk. Catalog-off those Crafts illegal. Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Unix sockets

```bash
cargo test -p shared transport::tests::parse_unix_absolute_path -- --exact --nocapture
cargo test -p shared transport::tests::reject_wss -- --exact --nocapture
cargo test -p shared transport::tests::unix_loopback_hello_snapshot -- --exact --nocapture
cargo test -p shared transport::tests::tcp_loopback_hello_snapshot -- --exact --nocapture
cargo test -p sim-cli --test net listen_and_connect_is_still_exclusive -- --exact --nocapture
cargo test -p sim-cli --test net unix_loopback_hello_and_hash_neutral -- --exact --nocapture
```

The non-unix unsupported-OS test is `transport::tests::unix_unsupported_on_this_os`. It compiles only on non-unix hosts. This walkthrough is Linux, so that name is **not run** here.

| Test | Success |
|---|---|
| parse | path `/tmp/agentplace.sock`; empty and `@abstract` error |
| `wss://` | rejected |
| unix loopback | Hello protocol 5; Welcome |
| tcp loopback | still passes |
| listen + connect | mutually exclusive |
| unix attach | same 2-tick hash as no listen |

Researcher (not run):

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet --listen unix:///tmp/agentplace.sock
```

---

## 2. Recipes

```bash
cargo test -p sim-core --test objects catalog_on_craft_baulk -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_collar -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_impost -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_rusk -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_new_m50_crafts_not_legal -- --exact --nocapture
```

| Test | Success |
|---|---|
| Craft | baulk / collar / impost / rusk |
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
| shipped `--no-time` | `e624f373…` |
| default time on | `42e90831…` |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |

Live Ollama / overnight / imgui / Windows unix bind: **not run** unless asked.

---

## Execution record

**Ran:** 2026-10-14, repo root (Linux). Each `--exact` line invoked separately. Package-level §0 also run. The non-unix unsupported-OS test is not compiled here.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0; objects 192/192 |
| §0 `cargo test -p viewer` | **pass** | 65/65 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 51/51 |
| §0 `cargo test -p shared` | **pass** | 17/17 |
| §1 Unix `--exact` ×6 | **pass** | path `/tmp/agentplace.sock`; `wss` rejected; unix Hello protocol 5 and Welcome; tcp loopback still passes; listen+connect exclusive; unix attach hash-neutral |
| §2 recipes `--exact` ×5 | **pass** | Craft baulk/collar/impost/rusk; catalog-off |
| §3 Hello / format / CLI | **pass** | format 3; Hello v5; default `final_hash=42e908310b577db105c115e18afb70212581a82fecc7f638661dd2ee24d908ea`; `--no-time` `final_hash=e624f373c1621d1c21941cbe6714c4448317d8a99795521b30c1fd6ea83d53c1` |
| `unix_unsupported_on_this_os` | **not run** | Linux host; the test is `cfg(not(unix))` |
| viewer window | **not run** | no display |
| live Spark / overnight | **not run** | not asked |
