//! MAP_CHUNK (MC) framing helpers matching Haxe Open Life / vanilla OHOL.
//!
//! Haxe `WorldMap.toString` emits space-separated `biome:floor:obj` cells,
//! then `haxe.zip.Compress` (zlib). Wire:
//! ```text
//! MC
//! sizeX sizeY x y
//! binary_raw_size binary_compressed_size
//! #
//! <zlib bytes>
//! ```

use flate2::write::ZlibEncoder;
use flate2::Compression;
use ol_world::{ObjectId, World};
use std::io::Write;

/// Build the textual MC header ending with `#` (binary follows immediately).
pub fn format_map_chunk_header(
    size_x: i32,
    size_y: i32,
    origin_x: i32,
    origin_y: i32,
    binary_raw_size: usize,
    binary_compressed_size: usize,
) -> String {
    format!(
        "MC\n{size_x} {size_y} {origin_x} {origin_y}\n{binary_raw_size} {binary_compressed_size}\n#"
    )
}

/// Prefix without `#` (rarely used).
pub fn format_map_chunk_message_prefix(
    size_x: i32,
    size_y: i32,
    origin_x: i32,
    origin_y: i32,
    binary_raw_size: usize,
    binary_compressed_size: usize,
) -> String {
    format!(
        "MC\n{size_x} {size_y} {origin_x} {origin_y}\n{binary_raw_size} {binary_compressed_size}\n"
    )
}

/// Collect object ids row-major for a rectangle with upper-left at (origin_x, origin_y).
pub fn build_region_object_ids(
    world: &World,
    origin_x: i32,
    origin_y: i32,
    size_x: i32,
    size_y: i32,
) -> Vec<ObjectId> {
    let mut out = Vec::with_capacity((size_x * size_y).max(0) as usize);
    for dy in 0..size_y {
        for dx in 0..size_x {
            out.push(world.get_object(origin_x + dx, origin_y + dy));
        }
    }
    out
}

/// Haxe `WorldMap.toString` cell format: `biome:floor:obj` joined by spaces.
///
/// `obj` is Haxe `MapData.stringID` / `ObjectHelper.toString`: bare base id,
/// flat contained `base,c0,c1`, or one-level nest `base,c0:sub0:sub1,c1`
/// (via [`ol_world::ComplexObject::to_map_string_id`]).
/// Row-major over width×height with world upper-left at (origin_x, origin_y).
pub fn build_chunk_plaintext(
    world: &World,
    origin_x: i32,
    origin_y: i32,
    size_x: i32,
    size_y: i32,
) -> String {
    let mut parts = Vec::with_capacity((size_x * size_y).max(0) as usize);
    for dy in 0..size_y {
        for dx in 0..size_x {
            let tx = origin_x + dx;
            let ty = origin_y + dy;
            let biome = world.get_biome(tx, ty);
            let floor = world.get_floor(tx, ty);
            let obj = world.encode_object_for_map(tx, ty);
            parts.push(format!("{biome}:{floor}:{obj}"));
        }
    }
    parts.join(" ")
}

/// Compress plaintext chunk with zlib (same family as Haxe `Compress.run`).
pub fn compress_chunk_plaintext(plain: &str) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(plain.as_bytes()).expect("zlib write");
    enc.finish().expect("zlib finish")
}

/// Full MC packet bytes: ASCII header ending in `#` then compressed body.
///
/// Samples **absolute** world tiles around `world_center_*`. Wire header uses
/// `wire_center_*` (birth-relative for the receiving client).
pub fn build_map_chunk_packet_ex(
    world: &World,
    world_center_x: i32,
    world_center_y: i32,
    wire_center_x: i32,
    wire_center_y: i32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    // Haxe sendMapChunk: x -= width/2; y -= height/2
    let world_origin_x = world_center_x - width / 2;
    let world_origin_y = world_center_y - height / 2;
    let wire_origin_x = wire_center_x - width / 2;
    let wire_origin_y = wire_center_y - height / 2;
    let plain = build_chunk_plaintext(world, world_origin_x, world_origin_y, width, height);
    let compressed = compress_chunk_plaintext(&plain);
    let header = format_map_chunk_header(
        width,
        height,
        wire_origin_x,
        wire_origin_y,
        plain.len(),
        compressed.len(),
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(&compressed);
    out
}

/// Convenience: absolute-only (tests / tools where birth offset is 0).
pub fn build_map_chunk_packet(
    world: &World,
    center_x: i32,
    center_y: i32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    build_map_chunk_packet_ex(world, center_x, center_y, center_x, center_y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_world::World;

    #[test]
    fn header_shape() {
        let h = format_map_chunk_header(32, 30, 10, 20, 100, 50);
        assert_eq!(h, "MC\n32 30 10 20\n100 50\n#");
    }

    #[test]
    fn region_scan() {
        let mut w = World::new(64, 64, false);
        w.set_object(2, 3, 99);
        let ids = build_region_object_ids(&w, 2, 3, 1, 1);
        assert_eq!(ids, vec![99]);
    }

    #[test]
    fn plaintext_and_packet_nonempty() {
        let mut w = World::new(64, 64, false);
        w.set_biome(0, 0, 2);
        w.set_object(0, 0, 33);
        let plain = build_chunk_plaintext(&w, 0, 0, 2, 1);
        // two cells
        assert!(plain.contains("2:0:33"));
        let pkt = build_map_chunk_packet(&w, 0, 0, 4, 4);
        assert!(pkt.starts_with(b"MC\n"));
        assert!(pkt.windows(1).any(|b| b[0] == b'#'));
        // compressed payload after #
        let hash = pkt.iter().position(|&b| b == b'#').unwrap();
        assert!(pkt.len() > hash + 1);
    }

    #[test]
    fn container_tile_not_plain_id_only_in_chunk_plaintext() {
        use ol_world::ComplexObject;

        let mut w = World::new(64, 64, false);
        w.set_biome(0, 0, 1);
        w.set_object_complex(
            0,
            0,
            ComplexObject {
                base_id: 391,
                uses_remaining: 0,
                contained: vec![33, 40],
                nested: Vec::new(),
                owner_id: 0,
                creation_time: 0.0,
                time_to_change: 0.0,
            },
        );
        // Neighbor plain object must stay bare id.
        w.set_object(1, 0, 99);

        let plain = build_chunk_plaintext(&w, 0, 0, 2, 1);
        // Haxe MapData.stringID / ObjectHelper.toString form
        assert!(
            plain.contains("1:0:391,33,40"),
            "container cell must encode contained list, got: {plain}"
        );
        // Must not be bare base id alone for that cell (still may appear in comma form).
        let cells: Vec<&str> = plain.split(' ').collect();
        assert_eq!(cells.len(), 2);
        let obj_part = cells[0].split(':').nth(2).unwrap_or("");
        assert_ne!(obj_part, "391", "must not be plain base id only");
        assert!(obj_part.contains(','), "contained wire form uses commas");
        assert_eq!(obj_part, "391,33,40");
        assert!(plain.contains("0:0:99") || cells[1].ends_with(":99"));
        assert_eq!(cells[1], "0:0:99");
    }

    #[test]
    fn nested_container_cell_uses_colon_subitems_in_chunk_plaintext() {
        use ol_world::ComplexObject;

        let mut w = World::new(64, 64, false);
        w.set_biome(0, 0, 1);
        // Basket with bag (292) and nested berries under the bag: Haxe `391,292:100:101`
        w.set_object_complex(
            0,
            0,
            ComplexObject {
                base_id: 391,
                uses_remaining: 0,
                contained: vec![292],
                nested: vec![vec![100, 101]],
                owner_id: 0,
                creation_time: 0.0,
                time_to_change: 0.0,
            },
        );
        let plain = build_chunk_plaintext(&w, 0, 0, 1, 1);
        let cells: Vec<&str> = plain.split(' ').collect();
        assert_eq!(cells.len(), 1);
        let obj_part = cells[0].split(':').nth(2).unwrap_or("");
        // Note: colon also separates biome:floor:obj, so obj may itself contain ':'.
        // Full cell is `biome:floor:obj` — join remainder after first two colons.
        let rest = cells[0]
            .splitn(3, ':')
            .nth(2)
            .unwrap_or("");
        assert_eq!(rest, "391,292:100:101", "got cell {plain}");
        assert!(rest.contains(':'), "nested wire uses ':' sub-ids");
        let _ = obj_part;
    }
}
