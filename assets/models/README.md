# Viewer models

Object meshes are bound from `configs/objects/*.toml` (`[visual]` / `[visual.lod]`), not from filename stems (except **agent**, still later). Missing files fall back to primitives. **Not hashed.**

Scale and pivot: [`docs/art-scale.md`](../../docs/art-scale.md). Agent capsule is **1.11** tall; 1 cell = 1 world unit.

CI does not download models. `cargo test -p viewer` must pass with this directory empty.
