use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use sim_core::World;

pub fn heightmap_mesh(world: &World) -> Mesh {
    let w = world.width as usize;
    let h = world.height as usize;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(w * h);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(w * h);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(w * h);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(w * h);
    let max_h = world.max_height.max(1) as f32;

    for z in 0..h {
        for x in 0..w {
            let y = world.height_at(x as u32, z as u32) as f32;
            positions.push([x as f32, y, z as f32]);
            uvs.push([x as f32 / (w as f32).max(1.0), z as f32 / (h as f32).max(1.0)]);
            if world.is_water(x as u32, z as u32) {
                colors.push([0.12, 0.34, 0.62, 1.0]);
            } else {
                let t = (y / max_h).clamp(0.0, 1.0);
                colors.push([0.22 + 0.18 * t, 0.42 + 0.22 * t, 0.16 + 0.08 * t, 1.0]);
            }
        }
    }

    for z in 0..h {
        for x in 0..w {
            let y_l = sample(&positions, w, h, x as i32 - 1, z as i32)[1];
            let y_r = sample(&positions, w, h, x as i32 + 1, z as i32)[1];
            let y_d = sample(&positions, w, h, x as i32, z as i32 - 1)[1];
            let y_u = sample(&positions, w, h, x as i32, z as i32 + 1)[1];
            let n = Vec3::new(y_l - y_r, 2.0, y_d - y_u).normalize_or_zero();
            normals.push(n.into());
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for z in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i00 = (z * w + x) as u32;
            let i10 = (z * w + x + 1) as u32;
            let i01 = ((z + 1) * w + x) as u32;
            let i11 = ((z + 1) * w + x + 1) as u32;
            // CCW when viewed from +Y.
            indices.extend_from_slice(&[i00, i01, i10, i10, i01, i11]);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn sample(positions: &[[f32; 3]], w: usize, h: usize, x: i32, z: i32) -> [f32; 3] {
    let x = x.clamp(0, w as i32 - 1) as usize;
    let z = z.clamp(0, h as i32 - 1) as usize;
    positions[z * w + x]
}

pub fn agent_world_pos(world: &World, x: u32, y: u32) -> Vec3 {
    let h = world.height_at(x, y) as f32;
    Vec3::new(x as f32 + 0.5, h + 0.7, y as f32 + 0.5)
}

pub fn resource_world_pos(world: &World, x: u32, y: u32, lift: f32) -> Vec3 {
    let h = world.height_at(x, y) as f32;
    Vec3::new(x as f32 + 0.5, h + lift, y as f32 + 0.5)
}
