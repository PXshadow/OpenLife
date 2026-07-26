//! PNG → biomes + natural object spawn (Haxe WorldMap.generate / generateObjects).

use crate::biome::biome_from_rgba;
use crate::{ComplexObject, World};
use ol_content::{BiomeSpawnTable, ContentDb};
use rand::Rng;
use std::path::Path;
use thiserror::Error;
use tracing::info;

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
}
