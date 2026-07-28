//! Client-side map store (C++ LivingLifePage map + MC/MX).
//!
//! Chunk **L-MAP**: decode zlib MC plaintext (`biome:floor:obj` cells) and apply MX.
//!
//! Wire (server `map_chunk.rs` / Haxe WorldMap.toString):
//! ```text
//! MC
//! sizeX sizeY originX originY
//! raw_size compressed_size
//! #
//! <zlib of space-separated biome:floor:obj …>
//! ```

use std::collections::HashMap;

use crate::frame::inflate_cm;
use crate::parse::{MapChange, MapChunkHeader, parse_leading_i32, parse_mc_header};

/// One tile of client knowledge.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapTile {
    pub biome: u8,
    pub floor_id: i32,
    /// Leading object id (0 = empty).
    pub object_id: i32,
    /// Raw object field (container format preserved).
    pub object_raw: String,
}

impl MapTile {
    pub fn empty() -> Self {
        Self {
            biome: 0,
            floor_id: 0,
            object_id: 0,
            object_raw: "0".into(),
        }
    }

    /// Parse one `biome:floor:obj` cell.
    pub fn parse_cell(cell: &str) -> Self {
        let mut it = cell.splitn(3, ':');
        let biome = it
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let floor_id = it
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let object_raw = it.next().unwrap_or("0").to_string();
        let object_id = parse_leading_i32(&object_raw).unwrap_or(0);
        Self {
            biome,
            floor_id,
            object_id,
            object_raw,
        }
    }

    /// Contained object ids from `object_raw` (top-level only).
    ///
    /// C++/protocol: commas separate contained; `+` nests sub-containers.
    /// Example: `125,33,40+2` → root 125, contained [33, 40] (2 nested under 40).
    pub fn contained_ids(&self) -> Vec<i32> {
        parse_object_raw_contained(&self.object_raw)
            .into_iter()
            .map(|n| n.id)
            .collect()
    }

    /// Full container tree from `object_raw`.
    pub fn object_stack(&self) -> ObjectStackNode {
        parse_object_raw_stack(&self.object_raw)
    }
}

/// One node in a map object container tree (protocol object field).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectStackNode {
    pub id: i32,
    pub contained: Vec<ObjectStackNode>,
}

/// Parse leading object id and top-level contained list from wire object field.
///
/// ```text
/// 0                  → empty
/// 33                 → stone
/// 125,33,40          → basket with 33 and 40
/// 125,33,40+2        → 40 contains 2
/// ```
pub fn parse_object_raw_stack(raw: &str) -> ObjectStackNode {
    let raw = raw.trim();
    if raw.is_empty() || raw == "0" {
        return ObjectStackNode::default();
    }
    let mut parts = raw.split(',');
    let first = parts.next().unwrap_or("0");
    let root_id = parse_leading_i32(first).unwrap_or(0);
    let mut contained = Vec::new();
    for p in parts {
        contained.push(parse_plus_chain(p));
    }
    ObjectStackNode {
        id: root_id,
        contained,
    }
}

/// Top-level contained nodes only (each may carry nested `+` children).
pub fn parse_object_raw_contained(raw: &str) -> Vec<ObjectStackNode> {
    parse_object_raw_stack(raw).contained
}

/// Parse `id` or `id+nested+…` into a containment chain (A contains B contains C).
fn parse_plus_chain(s: &str) -> ObjectStackNode {
    let ids: Vec<i32> = s.split('+').filter_map(parse_leading_i32).collect();
    if ids.is_empty() {
        return ObjectStackNode::default();
    }
    // Build innermost-first to avoid self-referential borrows.
    let mut node = ObjectStackNode {
        id: *ids.last().unwrap(),
        contained: Vec::new(),
    };
    for &id in ids.iter().rev().skip(1) {
        node = ObjectStackNode {
            id,
            contained: vec![node],
        };
    }
    node
}

/// Sparse client map keyed by absolute (wire/world) tile coordinates.
#[derive(Debug, Clone, Default)]
pub struct ClientMap {
    tiles: HashMap<(i32, i32), MapTile>,
    /// Last MC rectangle (origin + size).
    pub last_chunk: Option<MapChunkHeader>,
    /// Cells received in last successful MC decode.
    pub last_chunk_cells: usize,
    /// C++ `mMapAnimationFrameCount` — ground/object anim clock per tile.
    ///
    /// Advanced by [`crate::sound_bank::step_map_ground_anims_with_sounds`]
    /// (P2#14 map object ground-anim sounds). Not drawn state — sound trigger only.
    pub anim_frame_count: HashMap<(i32, i32), f32>,
    /// C++ `mMapFloorAnimationFrameCount`.
    pub floor_anim_frame_count: HashMap<(i32, i32), f32>,
}

impl ClientMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&MapTile> {
        self.tiles.get(&(x, y))
    }

    /// Iterate all known tile coordinates (for map anim sound step).
    pub fn tile_coords(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.tiles.keys().copied()
    }

    pub fn get_or_empty(&self, x: i32, y: i32) -> MapTile {
        self.tiles
            .get(&(x, y))
            .cloned()
            .unwrap_or_else(MapTile::empty)
    }

    pub fn set(&mut self, x: i32, y: i32, tile: MapTile) {
        self.tiles.insert((x, y), tile);
    }

    /// Apply one MX change (floor + object; biome unchanged unless we have no tile).
    pub fn apply_mx(&mut self, ch: &MapChange) {
        let old_obj = self
            .tiles
            .get(&(ch.x, ch.y))
            .map(|t| t.object_id)
            .unwrap_or(0);
        let entry = self.tiles.entry((ch.x, ch.y)).or_insert_with(MapTile::empty);
        entry.floor_id = ch.floor_id;
        entry.object_id = ch.object_id;
        entry.object_raw = ch.object_id_raw.clone();
        // C++ ~17388–17392: empty → placement resets ground anim frame count.
        if old_obj == 0 && ch.object_id > 0 && !ch.is_moving() {
            self.anim_frame_count.insert((ch.x, ch.y), 0.0);
        }
        // Moving objects: clear old tile object when provided.
        if let (Some(ox), Some(oy)) = (ch.old_x, ch.old_y) {
            if (ox, oy) != (ch.x, ch.y) {
                if let Some(old) = self.tiles.get_mut(&(ox, oy)) {
                    old.object_id = 0;
                    old.object_raw = "0".into();
                }
                self.anim_frame_count.remove(&(ox, oy));
            }
        }
    }

    pub fn apply_mx_many(&mut self, changes: &[MapChange]) {
        for ch in changes {
            self.apply_mx(ch);
        }
    }

    /// Decode MC: zlib binary → plaintext cells → store row-major from origin.
    pub fn apply_mc_binary(
        &mut self,
        header: &MapChunkHeader,
        compressed: &[u8],
    ) -> Result<usize, String> {
        let raw_hint = header.binary_raw_size.unwrap_or(0);
        let plain = inflate_cm(compressed, raw_hint).map_err(|e| e.to_string())?;
        self.apply_mc_plaintext(header, &plain)
    }

    /// Apply already-inflated MC cell string.
    pub fn apply_mc_plaintext(
        &mut self,
        header: &MapChunkHeader,
        plain: &str,
    ) -> Result<usize, String> {
        let cells: Vec<&str> = plain.split_whitespace().filter(|s| !s.is_empty()).collect();
        let expect = (header.size_x.max(0) * header.size_y.max(0)) as usize;
        if expect > 0 && cells.len() != expect {
            // Still apply what we have (some servers pad/truncate).
        }
        let mut n = 0usize;
        for (i, cell) in cells.iter().enumerate() {
            let dx = (i as i32) % header.size_x.max(1);
            let dy = (i as i32) / header.size_x.max(1);
            if dy >= header.size_y {
                break;
            }
            let x = header.x + dx;
            let y = header.y + dy;
            self.tiles.insert((x, y), MapTile::parse_cell(cell));
            n += 1;
        }
        self.last_chunk = Some(header.clone());
        self.last_chunk_cells = n;
        Ok(n)
    }

    /// Convenience: parse header text + binary in one step.
    pub fn apply_mc_framed(&mut self, header_text: &str, compressed: &[u8]) -> Result<usize, String> {
        let h = parse_mc_header(header_text).ok_or_else(|| "bad MC header".to_string())?;
        self.apply_mc_binary(&h, compressed)
    }

    /// True if tile blocks walking without full ObjectBank.
    ///
    /// C++ `computePathToDest`: unknown (`mMap == -1`) is **blocked**;
    /// `object_id == 0` is open; common wall-ish ids block (content refines).
    pub fn blocks_walk_heuristic(&self, x: i32, y: i32) -> bool {
        match self.get(x, y) {
            None => true, // unknown blocked (C++ mMap==-1)
            Some(t) => t.object_id > 0 && is_likely_blocking_id(t.object_id),
        }
    }
}

/// Very rough blocking list without full ObjectBank (expanded when content loads).
fn is_likely_blocking_id(id: i32) -> bool {
    // Common wall / tree-ish permanent ids — refined by content later.
    matches!(id, 885 | 886 | 887 | 888 | 889 | 99 | 100 | 33 if false)
        || (id >= 880 && id <= 920)
}

/// Compress helper for fixtures (same as server).
pub fn compress_mc_plain(plain: &str) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(plain.as_bytes()).expect("zlib");
    enc.finish().expect("finish")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_mx_line;

    #[test]
    fn parse_cell() {
        let t = MapTile::parse_cell("2:0:33");
        assert_eq!(t.biome, 2);
        assert_eq!(t.floor_id, 0);
        assert_eq!(t.object_id, 33);
    }

    #[test]
    fn mc_plaintext_2x2() {
        let mut m = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 2,
            size_y: 2,
            x: 10,
            y: 20,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let plain = "1:0:0 1:0:33 2:1:0 2:0:100";
        let n = m.apply_mc_plaintext(&h, plain).unwrap();
        assert_eq!(n, 4);
        assert_eq!(m.get(10, 20).unwrap().object_id, 0);
        assert_eq!(m.get(11, 20).unwrap().object_id, 33);
        assert_eq!(m.get(10, 21).unwrap().floor_id, 1);
        assert_eq!(m.get(11, 21).unwrap().object_id, 100);
    }

    #[test]
    fn mc_binary_roundtrip() {
        let mut m = ClientMap::new();
        let plain = "0:0:0 0:0:5 0:0:0 0:0:0";
        let comp = compress_mc_plain(plain);
        let h = MapChunkHeader {
            size_x: 2,
            size_y: 2,
            x: 0,
            y: 0,
            binary_raw_size: Some(plain.len()),
            binary_compressed_size: Some(comp.len()),
        };
        m.apply_mc_binary(&h, &comp).unwrap();
        assert_eq!(m.get(1, 0).unwrap().object_id, 5);
    }

    #[test]
    fn mx_updates_and_moves() {
        let mut m = ClientMap::new();
        m.set(0, 0, MapTile::parse_cell("1:0:50"));
        let ch = parse_mx_line("1 0 0 50 0 0 0 3.75").unwrap();
        assert!(ch.is_moving());
        m.apply_mx(&ch);
        assert_eq!(m.get(1, 0).unwrap().object_id, 50);
        assert_eq!(m.get(0, 0).unwrap().object_id, 0);
    }

    #[test]
    fn object_raw_container_tree() {
        let t = MapTile::parse_cell("0:0:125,33,40+2");
        assert_eq!(t.object_id, 125);
        let stack = t.object_stack();
        assert_eq!(stack.id, 125);
        assert_eq!(stack.contained.len(), 2);
        assert_eq!(stack.contained[0].id, 33);
        assert!(stack.contained[0].contained.is_empty());
        assert_eq!(stack.contained[1].id, 40);
        assert_eq!(stack.contained[1].contained.len(), 1);
        assert_eq!(stack.contained[1].contained[0].id, 2);
        assert_eq!(t.contained_ids(), vec![33, 40]);
    }
}
