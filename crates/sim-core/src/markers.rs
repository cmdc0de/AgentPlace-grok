//! Locked shape+colour pairs for the viewer legend (no Bevy types).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerShape {
    Sphere,
    Capsule,
    Cylinder,
    Cube,
    LongCuboid,
    FlatCuboid,
    Mushroom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkerSpec {
    pub shape: MarkerShape,
    pub rgb: [u8; 3],
    pub name: &'static str,
}

pub fn marker_for_veg(tag: u8) -> MarkerSpec {
    match tag {
        1 => MarkerSpec {
            shape: MarkerShape::Sphere,
            rgb: [46, 158, 56],
            name: "berry_bush",
        },
        2 => MarkerSpec {
            shape: MarkerShape::Capsule,
            rgb: [115, 184, 51],
            name: "herb",
        },
        3 => MarkerSpec {
            shape: MarkerShape::Mushroom,
            rgb: [191, 56, 46],
            name: "mushroom",
        },
        4 => MarkerSpec {
            shape: MarkerShape::Sphere,
            rgb: [115, 38, 140],
            name: "nightshade",
        },
        5 => MarkerSpec {
            shape: MarkerShape::Cylinder,
            rgb: [82, 56, 31],
            name: "tree",
        },
        _ => MarkerSpec {
            shape: MarkerShape::Cube,
            rgb: [46, 158, 56],
            name: "vegetation",
        },
    }
}

pub fn marker_crop() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::Cube,
        rgb: [140, 217, 64],
        name: "crop",
    }
}

pub fn marker_hare() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::LongCuboid,
        rgb: [184, 140, 82],
        name: "hare",
    }
}

pub fn marker_perch() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::FlatCuboid,
        rgb: [64, 115, 191],
        name: "perch",
    }
}

pub fn marker_mineral() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::Cube,
        rgb: [140, 133, 122],
        name: "mineral",
    }
}

pub fn marker_stockpile() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::Cube,
        rgb: [140, 97, 46],
        name: "stockpile",
    }
}

/// Worn Basket satchel. Distinct from the ground-crate brown (`srgb(0.35, 0.22, 0.12)`).
pub fn marker_satchel() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::Cube,
        rgb: [89, 56, 31],
        name: "satchel",
    }
}

/// Worn Backpack: slightly larger/darker than Basket satchel.
pub fn marker_backpack() -> MarkerSpec {
    MarkerSpec {
        shape: MarkerShape::Cube,
        rgb: [64, 38, 20],
        name: "backpack",
    }
}

pub fn legend_entries() -> Vec<(&'static str, MarkerShape, [u8; 3])> {
    vec![
        ("land", MarkerShape::Cube, [56, 107, 41]),
        ("water", MarkerShape::Cube, [31, 87, 158]),
        ("berry_bush", MarkerShape::Sphere, [46, 158, 56]),
        ("herb", MarkerShape::Capsule, [115, 184, 51]),
        ("mushroom", MarkerShape::Mushroom, [191, 56, 46]),
        ("nightshade", MarkerShape::Sphere, [115, 38, 140]),
        ("tree", MarkerShape::Cylinder, [82, 56, 31]),
        ("crop", MarkerShape::Cube, [140, 217, 64]),
        ("hare", MarkerShape::LongCuboid, [184, 140, 82]),
        ("perch", MarkerShape::FlatCuboid, [64, 115, 191]),
        ("mineral", MarkerShape::Cube, [140, 133, 122]),
        ("stockpile", MarkerShape::Cube, [140, 97, 46]),
        ("satchel", MarkerShape::Cube, [89, 56, 31]),
        ("backpack", MarkerShape::Cube, [64, 38, 20]),
        ("agent", MarkerShape::Capsule, [180, 80, 160]),
    ]
}

pub fn shape_name(shape: MarkerShape) -> &'static str {
    match shape {
        MarkerShape::Sphere => "sphere",
        MarkerShape::Capsule => "capsule",
        MarkerShape::Cylinder => "cylinder",
        MarkerShape::Cube => "cube",
        MarkerShape::LongCuboid => "long_cuboid",
        MarkerShape::FlatCuboid => "flat_cuboid",
        MarkerShape::Mushroom => "mushroom",
    }
}

pub fn rgb_f32(rgb: [u8; 3]) -> [f32; 3] {
    [
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
    ]
}
