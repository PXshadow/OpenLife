//! World sampling for the self-play HTML viewer (Haxe ground + object layers).

use ol_content::ContentDb;
use ol_world::World;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Downsampled full-map overview so the client can show the whole world.
#[derive(Debug, Clone, Serialize)]
pub struct MapOverview {
    pub width: i32,
    pub height: i32,
    pub step: i32,
    pub sample_w: i32,
    pub sample_h: i32,
    /// Biome id per sample cell (row-major).
    pub biomes: Vec<u8>,
    /// 1 if any object in the step×step block, else 0.
    pub object_mask: Vec<u8>,
    /// Approximate object count in each sample cell (capped 255).
    pub object_density: Vec<u8>,
}

/// Zoomed window with object ids + names (Haxe object layer detail).
#[derive(Debug, Clone, Serialize)]
pub struct MapWindow {
    pub origin_x: i32,
    pub origin_y: i32,
    pub w: i32,
    pub h: i32,
    pub biomes: Vec<u8>,
    pub floors: Vec<u16>,
    pub objects: Vec<i32>,
    pub uses: Vec<i32>,
    /// id → short name for ids present in this window.
    pub names: HashMap<String, String>,
}

/// Choose step so max(sample_w, sample_h) ≤ max_side (at least 1).
pub fn overview_step(width: i32, height: i32, max_side: i32) -> i32 {
    let max_side = max_side.max(16);
    if width <= 0 || height <= 0 {
        return 1;
    }
    let m = width.max(height);
    if m <= max_side {
        return 1;
    }
    ((m as f32) / (max_side as f32)).ceil() as i32
}

pub fn build_overview(world: &World, max_side: i32) -> MapOverview {
    let width = world.width_tiles.max(0);
    let height = world.height_tiles.max(0);
    let step = overview_step(width, height, max_side);
    let sample_w = if width == 0 {
        0
    } else {
        (width + step - 1) / step
    };
    let sample_h = if height == 0 {
        0
    } else {
        (height + step - 1) / step
    };
    let n = (sample_w * sample_h).max(0) as usize;
    let mut biomes = vec![0u8; n];
    let mut object_mask = vec![0u8; n];
    let mut object_density = vec![0u8; n];

    for sy in 0..sample_h {
        for sx in 0..sample_w {
            let tx0 = sx * step;
            let ty0 = sy * step;
            let idx = (sy * sample_w + sx) as usize;
            // Biome at block NW corner (stable overview).
            biomes[idx] = world.get_biome(tx0, ty0);
            let mut count = 0u32;
            let x1 = (tx0 + step).min(width);
            let y1 = (ty0 + step).min(height);
            for ty in ty0..y1 {
                for tx in tx0..x1 {
                    if world.get_object(tx, ty) != 0 {
                        count += 1;
                    }
                }
            }
            if count > 0 {
                object_mask[idx] = 1;
            }
            object_density[idx] = count.min(255) as u8;
        }
    }

    MapOverview {
        width,
        height,
        step,
        sample_w,
        sample_h,
        biomes,
        object_mask,
        object_density,
    }
}

pub fn build_window(
    world: &World,
    content: &ContentDb,
    center_x: i32,
    center_y: i32,
    w: i32,
    h: i32,
) -> MapWindow {
    let w = w.clamp(8, 128);
    let h = h.clamp(8, 128);
    let origin_x = center_x - w / 2;
    let origin_y = center_y - h / 2;
    let mut biomes = Vec::with_capacity((w * h) as usize);
    let mut floors = Vec::with_capacity((w * h) as usize);
    let mut objects = Vec::with_capacity((w * h) as usize);
    let mut uses = Vec::with_capacity((w * h) as usize);
    let mut seen: HashSet<i32> = HashSet::new();

    for dy in 0..h {
        for dx in 0..w {
            let tx = origin_x + dx;
            let ty = origin_y + dy;
            biomes.push(world.get_biome(tx, ty));
            floors.push(world.get_floor(tx, ty));
            let id = world.get_object(tx, ty);
            objects.push(id);
            let u = world
                .get_helper(tx, ty)
                .map(|hh| hh.uses_remaining)
                .unwrap_or(0);
            uses.push(u);
            if id != 0 {
                seen.insert(id);
            }
        }
    }

    let mut names = HashMap::new();
    for id in seen {
        let name = content
            .get(id)
            .map(|d| d.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("#{id}"));
        names.insert(id.to_string(), name);
    }

    MapWindow {
        origin_x,
        origin_y,
        w,
        h,
        biomes,
        floors,
        objects,
        uses,
        names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_world::World;

    #[test]
    fn overview_covers_full_extent() {
        let mut world = World::new(100, 50, true);
        world.ensure_full_map_chunks();
        world.set_biome(0, 0, 9);
        world.set_object(99, 49, 33);
        let ov = build_overview(&world, 32);
        assert_eq!(ov.width, 100);
        assert_eq!(ov.height, 50);
        assert!(ov.step >= 1);
        assert_eq!(ov.biomes.len(), (ov.sample_w * ov.sample_h) as usize);
        assert_eq!(ov.object_mask.len(), ov.biomes.len());
        // SE object should appear in last sample cell
        assert_eq!(*ov.object_mask.last().unwrap(), 1);
        assert_eq!(ov.biomes[0], 9);
    }

    #[test]
    fn window_includes_names() {
        let mut world = World::new(64, 64, false);
        world.set_object(10, 10, 100);
        let mut db = ContentDb::default();
        db.objects.insert(
            100,
            ol_content::ObjectDef {
                id: 100,
                description: "White Pine".into(),
                name: "White Pine".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let win = build_window(&world, &db, 10, 10, 16, 16);
        assert_eq!(win.names.get("100").map(|s| s.as_str()), Some("White Pine"));
        let idx = ((10 - win.origin_y) * win.w + (10 - win.origin_x)) as usize;
        assert_eq!(win.objects[idx], 100);
    }
}
