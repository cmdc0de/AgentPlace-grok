# Viewer art scale

World units are meters. **1 cell = 1×1** on XZ. Terrain height is the same unit on Y. Y-up. Object definitions live in `configs/objects/*.toml` and point at `assets/models/` (usually `optimized/`). Missing glb falls back to a primitive. Visuals are **not hashed**.

Export glTF/glb at this scale. Do not rely on a viewer multiplier.

## Agent (reference)

The placeholder agent is a Bevy capsule (`Capsule3d::new(0.28, 0.55)`). `length` is the cylinder only, not the hemispheres.

| | World units |
|---|---|
| Radius | 0.28 |
| Cylinder length | 0.55 |
| **Standing height** | **1.11** (0.55 + 2×0.28) |
| Footprint | ~0.56 across |
| Pivot | capsule **center**, **0.7** above the terrain cell |

A standing human-scale mesh should be about **1.1 units tall**, origin at **mid-body** (not the feet). +Z forward if facing matters.

## Other primitives (same units)

These are the fallback meshes when a glb is missing. Authored art should be in the same ballpark.

| Object | Primitive | About |
|---|---|---|
| Berry bush / vegetation sphere | `Sphere` r=0.22 | 0.44 tall |
| Herb | `Capsule3d` 0.08 / 0.32 | ~0.48 tall |
| Mushroom cap | `Sphere` r=0.20 | 0.40 tall |
| Tree | `Cylinder` r=0.12, h=0.7 | 0.7 tall, 0.24 across |
| Crop | `Cuboid` 0.32×0.28×0.32 | ~0.3 |
| Hare | `Cuboid` 0.22×0.22×0.38 | small animal |
| Fish / perch | `Cuboid` 0.18×0.12×0.28 | small fish |
| Stone / mineral | `Cuboid` 0.32×0.28×0.32 | ~0.3 |
| Ground crate | `Cuboid` 0.32×0.28×0.32 | then scaled **0.40–1.00** by fill |
| Worn basket | ~0.16×0.18×0.12 | on the agent |
| Worn backpack | ~0.22×0.26×0.16 | on the agent |

Crate and worn-pack meshes lerp fill scale 0.40 (empty) to 1.00 (full). Model the **full** size; the viewer shrinks empty ones.

## LOD

Camera Chebyshev distance in **cells**:

| Band | Distance | Use |
|---|---|---|
| near | &lt; 8 | highest-detail glb |
| mid | &lt; 24 | medium |
| far | ≥ 24 | lowest / combined |

Missing LOD file → next coarser, then `glb`, then primitive.

## Checklist

- [ ] Human / agent ≈ **1.1** tall, pivot mid-body
- [ ] One cell of ground is **1×1**; a tree or bush should fit in about one cell unless it is meant to overhang
- [ ] Hare / fish much smaller than the agent (see table)
- [ ] Y-up glTF; apply scale in the DCC tool before export
- [ ] Point `[visual]` / `[visual.lod]` at the file; do not rename to a stem
