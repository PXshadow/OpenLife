//! PNG → biomes + natural object spawn (Haxe WorldMap.generate / generateObjects).

use crate::biome::{
    biome_from_rgba, DESERT, GREEN, JUNGLE, PASSABLE_RIVER, RIVER, SWAMP, YELLOW,
};
use crate::{ComplexObject, World};
use ol_content::{BiomeSpawnTable, ContentDb};
use rand::Rng;
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;
use tracing::info;

/// Haxe `ServerSettings.CreateGreenBiomeDistance`.
// Haxe: ServerSettings.CreateGreenBiomeDistance = 5
pub const CREATE_GREEN_BIOME_DISTANCE: i32 = 5;

/// Haxe `ServerSettings.ChanceForLuckySpot`.
// Haxe: ServerSettings.ChanceForLuckySpot = 0.1
pub const CHANCE_FOR_LUCKY_SPOT: f32 = 0.1;

/// Deterministic generate seed from map size (same as world_boot).
pub fn map_generation_seed(width: i32, height: i32) -> u64 {
    (width as u64)
        .wrapping_mul(1_000_003)
        .wrapping_add(height as u64)
}

/// Weighted pick of a natural object id from a biome spawn table (Haxe generateObjects).
///
/// Returns `None` when the table is empty or has non-positive total chance.
pub fn pick_biome_spawn(table: &BiomeSpawnTable, rng: &mut impl Rng) -> Option<i32> {
    if table.total_chance <= 0.0 || table.entries.is_empty() {
        return None;
    }
    let random = rng.gen::<f32>() * table.total_chance;
    let mut sum = 0.0f32;
    for &(obj_id, chance) in &table.entries {
        sum += chance;
        if random <= sum {
            return Some(obj_id);
        }
    }
    // Floating-point edge: fall back to last entry.
    table.entries.last().map(|(id, _)| *id)
}

/// Place a natural object at `(x,y)` using multi-use complex helper when needed.
///
/// Returns the object id placed, or `None` if `obj_id` cannot be resolved and was not placed
/// (callers still may place bare ids when def is missing — this always places).
pub fn place_natural_object(world: &mut World, content: &ContentDb, x: i32, y: i32, obj_id: i32) {
    if let Some(def) = content.get(obj_id) {
        if def.num_uses > 1 {
            world.set_object_complex(x, y, ComplexObject::with_uses(obj_id, def.num_uses));
            return;
        }
    }
    world.set_object(x, y, obj_id);
}

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("image: {0}")]
    Image(#[from] image::ImageError),
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub wrap: bool,
    /// Haxe: randomFloat() > 0.4 continue → density ~0.4 chance to attempt spawn
    pub density: f32,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            wrap: true,
            density: 0.4,
        }
    }
}

/// Load PNG, map colors to biomes (Y flipped like Haxe), fill world chunks.
pub fn generate_from_png(
    png_path: impl AsRef<Path>,
    opts: &GenerateOptions,
) -> Result<World, GenerateError> {
    let img = image::open(png_path.as_ref())?.to_rgba8();
    let width = img.width() as i32;
    let height = img.height() as i32;
    let mut world = World::new(width, height, opts.wrap);
    world.ensure_full_map_chunks();

    for y in 0..height {
        for x in 0..width {
            // Haxe: p at (x + ((height-1)-y) * width) — flip Y
            let py = (height - 1 - y) as u32;
            let px = x as u32;
            let p = img.get_pixel(px, py);
            let biome = biome_from_rgba(p[0], p[1], p[2]);
            world.set_biome(x, y, biome);
        }
    }

    info!(width, height, path = %png_path.as_ref().display(), "generated biomes from PNG");
    Ok(world)
}

/// Haxe Canada Goose Pond on grassland next to passable river.
// Haxe: WorldMap.generateObjects L1223–1232 object 141
pub const CANADA_GOOSE_POND: i32 = 141;

fn tile_near_passable_river(world: &World, x: i32, y: i32) -> bool {
    world.get_biome(x + 1, y) == PASSABLE_RIVER
        || world.get_biome(x - 1, y) == PASSABLE_RIVER
        || world.get_biome(x, y + 1) == PASSABLE_RIVER
        || world.get_biome(x, y - 1) == PASSABLE_RIVER
}

/// Haxe `generateObjects`: weighted pick from biome_spawn tables.
pub fn spawn_natural_objects(
    world: &mut World,
    content: &ContentDb,
    density: f32,
    rng: &mut impl Rng,
) -> u32 {
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        return 0;
    }
    let mut generated = 0u32;

    for y in 0..h {
        for x in 0..w {
            // skip if object below already
            if y > 0 && world.get_object(x, y - 1) != 0 {
                continue;
            }
            if rng.gen::<f32>() > density {
                continue;
            }
            // Haxe: GREEN + 0.3 + neighbor PASSABLERIVER → Canada Goose Pond 141
            if world.get_biome(x, y) == GREEN
                && rng.gen::<f32>() < 0.3
                && tile_near_passable_river(world, x, y)
            {
                place_natural_object(world, content, x, y, CANADA_GOOSE_POND);
                generated += 1;
                continue;
            }
            let biome = world.get_biome(x, y) as i32;
            let Some(table) = content.biome_spawn.get(&biome) else {
                continue;
            };
            let Some(obj_id) = pick_biome_spawn(table, rng) else {
                continue;
            };
            place_natural_object(world, content, x, y, obj_id);
            generated += 1;
        }
    }
    info!(generated, "natural objects spawned");
    generated
}

/// Haxe `ServerSettings.CanObjectBeLuckySpot` — iron / spring / tar / dug rock skip.
// Haxe: ServerSettings.CanObjectBeLuckySpot L469–476
pub fn can_object_be_lucky_spot(obj: i32) -> bool {
    !matches!(obj, 3030 | 2285 | 503 | 942 | 3961 | 3962)
}

/// Haxe `WorldMap.addExtraBiomes`: river banks → passable river; jungle/swamp fringe → green.
// Haxe: WorldMap.addExtraBiomes L1164–1203
pub fn add_extra_biomes(world: &mut World, dist: i32) {
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        return;
    }
    let dist = dist.max(0);
    let mut placed: HashSet<(i32, i32)> = HashSet::new();
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = world.wrap_tile(x, y);
            if placed.contains(&(sx, sy)) {
                continue;
            }
            let biome = world.get_biome(sx, sy);
            let riverish = biome == RIVER || biome == PASSABLE_RIVER;
            let fringe = riverish || biome == JUNGLE || biome == SWAMP;
            if !fringe {
                continue;
            }
            for iy in -dist..=dist {
                for ix in -dist..=dist {
                    let (nx, ny) = world.wrap_tile(sx + ix, sy + iy);
                    if placed.contains(&(nx, ny)) {
                        continue;
                    }
                    let next = world.get_biome(nx, ny);
                    if riverish && ix * ix < 2 && iy * iy < 2 {
                        if next == GREEN || next == YELLOW || next == DESERT || next == RIVER {
                            if biome == PASSABLE_RIVER || next != RIVER {
                                world.set_biome(nx, ny, PASSABLE_RIVER);
                                placed.insert((nx, ny));
                            }
                        }
                    } else if next == YELLOW || next == DESERT {
                        world.set_biome(nx, ny, GREEN);
                    }
                }
            }
        }
    }
}

/// Haxe `WorldMap.generateExtraStuff` lucky-spot clusters of the same object.
// Haxe: WorldMap.generateExtraStuff L1273–1376
pub fn generate_lucky_spots(
    world: &mut World,
    content: &ContentDb,
    rng: &mut impl Rng,
    chance: f32,
) -> u32 {
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 || !(chance > 0.0) {
        return 0;
    }
    let mut placed: HashSet<(i32, i32)> = HashSet::new();
    let mut extra = 0u32;
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = world.wrap_tile(x, y);
            let obj = world.get_object(sx, sy);
            if obj <= 0 {
                continue;
            }
            if placed.contains(&(sx, sy)) {
                continue;
            }
            if !can_object_be_lucky_spot(obj) {
                continue;
            }
            if world.get_biome(sx, sy) == PASSABLE_RIVER {
                continue;
            }
            if rng.gen::<f32>() >= chance {
                continue;
            }
            let has_time = content.find_transition(-1, obj).is_some();
            // Haxe: 2 + randomInt(timeTransition != null ? 1 : 5); randomInt is inclusive.
            let max_extra = if has_time { 1 } else { 5 };
            let mut remain = 2 + rng.gen_range(0..=max_extra);
            let src_biome = world.get_biome(sx, sy);
            let dist = 9i32;
            for _ in 0..100 {
                if remain <= 0 {
                    break;
                }
                let tx = sx + rng.gen_range(0..=dist * 2) - dist;
                let ty = sy + rng.gen_range(0..=dist * 2) - dist;
                let dx = tx - sx;
                let dy = ty - sy;
                if dx * dx + dy * dy > dist * dist {
                    continue;
                }
                let (tx, ty) = world.wrap_tile(tx, ty);
                if world.get_biome(tx, ty) != src_biome {
                    continue;
                }
                if world.get_object(tx, ty) != 0 {
                    continue;
                }
                if world.get_object(tx, ty - 1) != 0 {
                    continue;
                }
                if world.get_object(tx, ty + 1) != 0 {
                    continue;
                }
                if world.get_object(tx - 1, ty) != 0 {
                    continue;
                }
                placed.insert((tx, ty));
                place_natural_object(world, content, tx, ty, obj);
                extra += 1;
                remain -= 1;
            }
        }
    }
    extra
}

/// Haxe generate order after PNG: extra biomes → objects → lucky spots.
// Haxe: WorldMap.generate L697–705
pub fn populate_fresh_map(
    world: &mut World,
    content: &ContentDb,
    density: f32,
    rng: &mut impl Rng,
) -> u32 {
    add_extra_biomes(world, CREATE_GREEN_BIOME_DISTANCE);
    let n = spawn_natural_objects(world, content, density, rng);
    let extra = generate_lucky_spots(world, content, rng, CHANCE_FOR_LUCKY_SPOT);
    n + extra
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::env;

    #[test]
    fn png_roundtrip_biome() {
        let dir = env::temp_dir().join("ol_png_gen_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tiny.png");
        // 2x2: green + ocean
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([0x00, 0x40, 0x80, 255])); // ocean at top of image → y=1 after flip
        img.put_pixel(1, 0, Rgba([0xB5, 0xE6, 0x1D, 255]));
        img.put_pixel(0, 1, Rgba([0xB5, 0xE6, 0x1D, 255])); // green at bottom → y=0
        img.put_pixel(1, 1, Rgba([0xB5, 0xE6, 0x1D, 255]));
        img.save(&path).unwrap();

        let world = generate_from_png(&path, &GenerateOptions::default()).unwrap();
        assert_eq!(world.width_tiles, 2);
        assert_eq!(world.height_tiles, 2);
        // y=0 comes from image y=1 (flipped) = green
        assert_eq!(world.get_biome(0, 0), crate::biome::GREEN);
        // y=1 from image y=0 = ocean
        assert_eq!(world.get_biome(0, 1), crate::biome::OCEAN);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pick_biome_spawn_weighted_and_empty() {
        use ol_content::BiomeSpawnTable;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let empty = BiomeSpawnTable::default();
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(pick_biome_spawn(&empty, &mut rng), None);

        let table = BiomeSpawnTable {
            total_chance: 1.0,
            entries: vec![(33, 1.0)],
        };
        for _ in 0..8 {
            assert_eq!(pick_biome_spawn(&table, &mut rng), Some(33));
        }
    }

    #[test]
    fn green_next_to_passable_river_can_place_goose_pond() {
        use crate::PASSABLE_RIVER;
        let mut world = World::new(3, 3, false);
        for y in 0..3 {
            for x in 0..3 {
                world.set_biome(x, y, crate::GREEN);
            }
        }
        world.set_biome(2, 1, PASSABLE_RIVER);
        let db = ContentDb::default();
        // Force the density + 0.3 rolls by using a rng that always returns 0.
        struct ZeroRng;
        impl rand::RngCore for ZeroRng {
            fn next_u32(&mut self) -> u32 {
                0
            }
            fn next_u64(&mut self) -> u64 {
                0
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                for b in dest {
                    *b = 0;
                }
            }
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        let n = spawn_natural_objects(&mut world, &db, 1.0, &mut ZeroRng);
        assert!(n >= 1);
        assert_eq!(world.get_object(1, 1), CANADA_GOOSE_POND);
    }

    #[test]
    fn extra_biomes_fringe_green_and_river_bank() {
        let mut world = World::new(11, 11, false);
        for y in 0..11 {
            for x in 0..11 {
                world.set_biome(x, y, crate::biome::YELLOW);
            }
        }
        world.set_biome(5, 5, crate::biome::JUNGLE);
        add_extra_biomes(&mut world, 5);
        assert_eq!(world.get_biome(5, 10), crate::biome::GREEN);
        assert_eq!(world.get_biome(5, 5), crate::biome::JUNGLE);

        let mut river = World::new(5, 5, false);
        for y in 0..5 {
            for x in 0..5 {
                river.set_biome(x, y, crate::biome::GREEN);
            }
        }
        river.set_biome(2, 2, crate::RIVER);
        add_extra_biomes(&mut river, 5);
        assert_eq!(river.get_biome(2, 3), PASSABLE_RIVER);
        assert_eq!(river.get_biome(2, 2), crate::RIVER);
    }

    #[test]
    fn lucky_spot_places_cluster_of_same_id() {
        use rand::SeedableRng;
        let mut world = World::new(21, 21, false);
        for y in 0..21 {
            for x in 0..21 {
                world.set_biome(x, y, crate::biome::GREEN);
            }
        }
        world.set_object(10, 10, 33);
        let db = ContentDb::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let extra = generate_lucky_spots(&mut world, &db, &mut rng, 1.0);
        assert!(extra >= 1, "lucky spots must clone the source object, extra={extra}");
        let mut n33 = 0;
        for y in 0..21 {
            for x in 0..21 {
                if world.get_object(x, y) == 33 {
                    n33 += 1;
                }
            }
        }
        assert!(n33 >= 2, "source + at least one clone, n={n33}");
        assert!(!can_object_be_lucky_spot(3961));
        assert!(can_object_be_lucky_spot(33));
    }
}
