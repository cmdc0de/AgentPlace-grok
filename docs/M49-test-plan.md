# M49 test plan — see each new feature

Walkthrough for [`M49-plan.md`](M49-plan.md). Automated tests prove the slice; the `sim-cli` / viewer steps below are what you **read** (object visual scale, extra recipes, invention flavor). Next slice: [`M52-plan.md`](M52-plan.md).

All commands assume the **repo root**. Mock / `cargo test` never need the network.

`cargo test -- --exact NAME` takes **one** test name. Integration tests need `--test objects` (or `inventions`, `survival`, `net`); unit tests under `-p viewer` use the module path (`models::tests::…`). Shared protocol tests use `--lib protocol::tests::…`.

**Success for the slice:** `[visual] scale` omit / invalid ⇒ no explicit scale (agent auto-fit); `scale = 2.0` is used and is **not hashed**. Catalog-off ⇒ hammer Craft illegal; no-objects hash `70e5204d…`. Catalog-on ⇒ Craft hammer (2 stone + 1 wood) and dried_fish (1 food). Successful Invent stores flavor `invented gather bonus`; `--load` keeps it. Default `sim-cli` 2-tick hash is `133ea72e…` (M49 recipes). Shipping `default.toml` / `coop.toml` unchanged. Checkpoints **write format_version 3**; **`PROTOCOL_VERSION = 5`**.

---

## 0. Safety net

No network for tests. TCP/WS tests are loopback only.

```bash
cargo test -p sim-core
cargo test -p viewer
cargo test -p sim-cli --test net
cargo test -p shared
```

---

## 1. Object visual scale

```bash
cargo test -p viewer models::tests::visual_scale_omit_invalid_and_explicit -- --exact --nocapture
cargo test -p viewer models::tests::visual_scale_not_hashed -- --exact --nocapture
```

| Test | Success |
|---|---|
| omit / 0 / NaN | `visual_effective_scale` is `None` (agent auto-fit) |
| scale 2.0 | `Some(2.0)` |
| not hashed | apply scale-only TOML; `state_hash` unchanged |

---

## 2. Extra recipes

```bash
cargo test -p sim-core --test objects catalog_off_hammer_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_hammer -- --exact --nocapture
cargo test -p sim-core --test objects catalog_on_craft_dried_fish -- --exact --nocapture
cargo test -p sim-core --test objects catalog_off_two_ticks_hash_ignores_new_recipes -- --exact --nocapture
```

| Test | Success |
|---|---|
| catalog-off | hammer Catalog Craft illegal |
| catalog-on hammer | 2 stone + 1 wood → 1 hammer |
| catalog-on dried_fish | 1 food → 1 dried_fish |
| no objects dir | hash `70e5204d…` |

---

## 3. Invention flavor + Hello / format / idle

```bash
cargo test -p sim-core --test inventions invent_flavor_mock_is_memory_text -- --exact --nocapture
cargo test -p sim-core --test inventions invent_flavor_load_keeps_text -- --exact --nocapture
cargo test -p sim-core --test inventions inventions_off_not_legal_same_hash -- --exact --nocapture
cargo test -p sim-core --test objects write_ckpt_is_format_version_3 -- --exact --nocapture
cargo test -p sim-core --test survival format_version_is_v3 -- --exact --nocapture
cargo test -p shared --lib protocol::tests::hello_none_frame_is_postcard_v5 -- --exact --nocapture
cargo test -p sim-core --test objects default_mock_two_ticks_idle_hash -- --exact --nocapture
cargo run -p sim-cli -- --config configs/default.toml --ticks 2 --llm mock --quiet
```

| Test | Success |
|---|---|
| flavor mock | `invented gather bonus`; hashed stably |
| `--load` | flavor restored; no second call |
| inventions off | Invent illegal; idle hash unchanged |
| write ckpt | `format_version = 3` |
| Hello v5 | unchanged |
| idle 2 ticks with shipped objects | `final_hash=133ea72e…` |

---

## Researcher attach (read)

```bash
cargo run -p sim-cli -- --config configs/default.toml --ticks 80 \
  --listen ws://127.0.0.1:9001 --allow-control --start-paused --inventions --quiet
```

```bash
cargo run -p viewer -- --connect ws://127.0.0.1:9001 --objects configs/objects
```

Live Ollama / overnight / imgui / desktop viewer window: **not run** unless asked.

---

## Execution record

**Ran:** 2026-09-11, repo root. Each `--exact` line invoked separately. Package-level §0 also run. Viewer window not driven.

| Step | Result | Evidence |
|---|---|---|
| §0 `cargo test -p sim-core` | **pass** | exit 0 (lib 38; inventions 17; objects 30) |
| §0 `cargo test -p viewer` | **pass** | 54/54 |
| §0 `cargo test -p sim-cli --test net` | **pass** | 46/46 |
| §0 `cargo test -p shared` | **pass** | 15/15 protocol + transport |
| §1 scale `--exact` ×2 | **pass** | omit/0/NaN None; 2.0 Some; not hashed |
| §2 recipes `--exact` ×4 | **pass** | hammer Craft; dried_fish from Food(1); no-objects `70e5204d…` |
| §3 flavor / Hello / idle | **pass** | `invented gather bonus`; load keeps flavor; format 3; Hello v5 |
| §3 default 2 ticks `sim-cli` | **pass** | `final_tick=2` `final_hash=133ea72ed5004dcca5321348d14d3d1aa281c7188d30906e76b08f597ef7709b` |
| viewer window / imgui / live LLM flavor | **not run** | no display / no live model |
| live Spark / overnight | **not run** | not asked |
