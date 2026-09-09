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

/// C++ `mMapMoveOffsets` / `mMapMoveSpeeds` — sliding map objects (thrown / move-trans).
#[derive(Debug, Clone, Copy, Default)]
pub struct MapMoveState {
    /// Offset from cell center in **tiles** (C++ CELL_D units / CELL_D).
    pub offset_x: f32,
    pub offset_y: f32,
    /// Speed in tiles/sec (C++ speed was object-units/sec; we store tile units).
    pub speed: f32,
}

/// C++ `mMapDropOffsets` / `mMapDropRot` — drop-from-hand slide (tile units).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MapDropState {
    pub offset_x: f32,
    pub offset_y: f32,
    pub rot: f32,
}

impl MapDropState {
    pub fn is_sliding(&self) -> bool {
        self.offset_x.abs() > 1e-5 || self.offset_y.abs() > 1e-5 || self.rot.abs() > 1e-5
    }
}

impl MapMoveState {
    pub fn is_moving(self) -> bool {
        self.speed > 1e-6
            && (self.offset_x.abs() > 1e-5 || self.offset_y.abs() > 1e-5)
    }
}

/// C++ `ExtraMapObject` — mover drawn separately while dest stays occupied.
///
/// // LivingLifePage MX ~17074–17094: leave dest object in place; slide `object_id`
/// into `dest_*`; on arrival replace dest with `dest_object_*`.
#[derive(Debug, Clone)]
pub struct ExtraMovingObject {
    pub object_id: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub dest_object_id: i32,
    pub dest_object_raw: String,
    pub offset_x: f32,
    pub offset_y: f32,
    pub speed: f32,
    pub flip: bool,
}

impl ExtraMovingObject {
    pub fn is_moving(&self) -> bool {
        self.speed > 1e-6
            && (self.offset_x.abs() > 1e-5 || self.offset_y.abs() > 1e-5)
    }
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
    /// C++ `mMapMoveOffsets` + `mMapMoveSpeeds` per cell.
    pub move_state: HashMap<(i32, i32), MapMoveState>,
    /// C++ `mMapDropOffsets` — object flying from a player's hand onto this cell.
    pub drop_state: HashMap<(i32, i32), MapDropState>,
    /// C++ `mMapTileFlips` — true = face left (draw flipH).
    pub tile_flips: HashMap<(i32, i32), bool>,
    /// C++ `mMapExtraMovingObjects` (+ dest world pos / dest object ids).
    pub extra_moving: Vec<ExtraMovingObject>,
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

    /// Start / update a sliding map object (C++ `mMapMoveSpeeds[mapI] = speed`).
    ///
    /// `offset_*` are in tiles from the destination cell (object appears offset
    /// toward the old cell and slides to 0).
    pub fn set_map_move(&mut self, x: i32, y: i32, offset_x: f32, offset_y: f32, speed: f32) {
        if speed <= 1e-6 {
            self.move_state.remove(&(x, y));
            return;
        }
        self.move_state.insert(
            (x, y),
            MapMoveState {
                offset_x,
                offset_y,
                speed,
            },
        );
    }

    pub fn map_move(&self, x: i32, y: i32) -> MapMoveState {
        self.move_state.get(&(x, y)).copied().unwrap_or_default()
    }

    /// Whether this MX should start a drop-from-hand slide (C++ ~17455–17573).
    ///
    /// Adapted for MX-before-PU: still animate when `held_id == object_id`.
    /// Skip use-on-bare-ground (`held` is a different tool on an empty cell).
    pub fn should_start_drop_slide(
        player_id: i32,
        object_id: i32,
        is_moving: bool,
        old_object_id: i32,
        player_on_screen: bool,
        player_held_id: i32,
    ) -> bool {
        if player_id <= 0 || object_id <= 0 || is_moving || !player_on_screen {
            return false;
        }
        if player_held_id > 0 && player_held_id != object_id && old_object_id == 0 {
            return false;
        }
        true
    }

    /// Tile-space drop offset so the object is drawn at `held_*` (dest is cell center).
    pub fn drop_offset_from_held(dest_x: i32, dest_y: i32, held_x: f32, held_y: f32) -> (f32, f32) {
        (held_x - dest_x as f32 - 0.5, held_y - dest_y as f32 - 0.5)
    }

    /// C++ `mMapDropOffsets[mapI]` — tile-space offset from dest cell center.
    pub fn set_drop_offset(&mut self, x: i32, y: i32, offset_x: f32, offset_y: f32, rot: f32) {
        if offset_x.abs() < 1e-5 && offset_y.abs() < 1e-5 && rot.abs() < 1e-5 {
            self.drop_state.remove(&(x, y));
            return;
        }
        self.drop_state.insert(
            (x, y),
            MapDropState {
                offset_x,
                offset_y,
                rot,
            },
        );
    }

    pub fn drop_offset(&self, x: i32, y: i32) -> MapDropState {
        self.drop_state.get(&(x, y)).copied().unwrap_or_default()
    }

    pub fn tile_flip(&self, x: i32, y: i32) -> bool {
        self.tile_flips.get(&(x, y)).copied().unwrap_or(false)
    }

    /// C++ ~17066–17070: face right when moving east / E-W special; left when west.
    pub fn set_tile_flip_from_move(&mut self, x: i32, y: i32, old_true_x: f32) {
        if (x as f32) > old_true_x {
            self.tile_flips.insert((x, y), false);
        } else if (x as f32) < old_true_x {
            self.tile_flips.insert((x, y), true);
        }
    }

    /// C++ per-frame map move step — cell slides + ExtraMapObject slides.
    pub fn step_map_moves(&mut self, dt: f32) {
        if dt <= 1e-8 {
            return;
        }
        let keys: Vec<(i32, i32)> = self.move_state.keys().copied().collect();
        for k in keys {
            let Some(m) = self.move_state.get_mut(&k) else {
                continue;
            };
            if m.speed <= 1e-6 {
                self.move_state.remove(&k);
                continue;
            }
            let len = (m.offset_x * m.offset_x + m.offset_y * m.offset_y).sqrt();
            let step = m.speed * dt;
            if len <= step || len < 1e-5 {
                self.move_state.remove(&k);
                // Keep facing after settle (C++ leaves mMapTileFlips).
                continue;
            }
            let s = step / len;
            m.offset_x -= m.offset_x * s;
            m.offset_y -= m.offset_y * s;
        }
        // Extra movers (C++ ~13712)
        let mut i = 0;
        while i < self.extra_moving.len() {
            let e = &mut self.extra_moving[i];
            if e.speed <= 1e-6 {
                self.finish_extra_moving(i);
                continue;
            }
            let len = (e.offset_x * e.offset_x + e.offset_y * e.offset_y).sqrt();
            let step = e.speed * dt;
            if len <= step || len < 1e-5 {
                self.finish_extra_moving(i);
                continue;
            }
            let s = step / len;
            e.offset_x -= e.offset_x * s;
            e.offset_y -= e.offset_y * s;
            i += 1;
        }
        // C++ drawMapCell ~4595: drop offset steps 0.0625 * frameRateFactor toward 0.
        let frf = (dt * 60.0).clamp(0.0, 4.0);
        let drop_keys: Vec<(i32, i32)> = self.drop_state.keys().copied().collect();
        for k in drop_keys {
            let Some(d) = self.drop_state.get(&k).copied() else {
                continue;
            };
            let (nx, ny, landed) =
                crate::anim_draw::step_held_by_drop_offset(d.offset_x, d.offset_y, frf);
            let rot_step = 0.03125 * frf;
            let mut nrot = d.rot;
            if nrot.abs() < rot_step {
                nrot = 0.0;
            } else {
                nrot -= nrot.signum() * rot_step;
            }
            if landed && nrot.abs() < 1e-5 {
                self.drop_state.remove(&k);
            } else {
                self.drop_state.insert(
                    k,
                    MapDropState {
                        offset_x: nx,
                        offset_y: ny,
                        rot: nrot,
                    },
                );
            }
        }
    }

    fn finish_extra_moving(&mut self, index: usize) {
        let Some(e) = self.extra_moving.get(index).cloned() else {
            return;
        };
        let entry = self
            .tiles
            .entry((e.dest_x, e.dest_y))
            .or_insert_with(MapTile::empty);
        entry.object_id = e.dest_object_id;
        entry.object_raw = e.dest_object_raw;
        if e.flip {
            self.tile_flips.insert((e.dest_x, e.dest_y), true);
        } else {
            self.tile_flips.insert((e.dest_x, e.dest_y), false);
        }
        self.extra_moving.remove(index);
    }

    /// Apply one MX change (floor + object; biome unchanged unless we have no tile).
    ///
    /// Moving MX (`old_x old_y speed`): C++ `mMapMoveOffsets/Speeds` — object slides
    /// from old true pos into the new cell. If dest was occupied, C++ ExtraMapObject
    /// path: leave dest occupant, draw mover as extra until arrival.
    pub fn apply_mx(&mut self, ch: &MapChange) {
        let (old_obj, old_raw) = self
            .tiles
            .get(&(ch.x, ch.y))
            .map(|t| (t.object_id, t.object_raw.clone()))
            .unwrap_or((0, "0".into()));

        // Moving into occupied dest → ExtraMapObject (C++ ~17074).
        if let (Some(ox), Some(oy), Some(speed)) = (ch.old_x, ch.old_y, ch.speed) {
            if old_obj > 0 && ch.object_id > 0 && speed > 1e-6 {
                let old_off = self.map_move(ox, oy);
                let old_true_x = ox as f32 + old_off.offset_x;
                let old_true_y = oy as f32 + old_off.offset_y;
                // Floor may still update; keep previous occupant on the cell.
                let entry = self.tiles.entry((ch.x, ch.y)).or_insert_with(MapTile::empty);
                entry.floor_id = ch.floor_id;
                entry.object_id = old_obj;
                entry.object_raw = old_raw;
                if let Some(src) = self.tiles.get_mut(&(ox, oy)) {
                    src.object_id = 0;
                    src.object_raw = "0".into();
                }
                self.move_state.remove(&(ox, oy));
                self.tile_flips.remove(&(ox, oy));
                let flip = (ch.x as f32) < old_true_x;
                self.extra_moving.push(ExtraMovingObject {
                    object_id: ch.object_id,
                    dest_x: ch.x,
                    dest_y: ch.y,
                    dest_object_id: ch.object_id,
                    dest_object_raw: ch.object_id_raw.clone(),
                    offset_x: old_true_x - ch.x as f32,
                    offset_y: old_true_y - ch.y as f32,
                    speed: speed.max(0.0),
                    flip,
                });
                return;
            }
        }

        let entry = self.tiles.entry((ch.x, ch.y)).or_insert_with(MapTile::empty);
        entry.floor_id = ch.floor_id;
        entry.object_id = ch.object_id;
        entry.object_raw = ch.object_id_raw.clone();
        // C++ ~17388–17392: empty → placement resets ground anim frame count.
        if old_obj == 0 && ch.object_id > 0 && !ch.is_moving() {
            self.anim_frame_count.insert((ch.x, ch.y), 0.0);
        }
        // Moving into empty (or same-cell): clear source, set slide on dest.
        if let (Some(ox), Some(oy), Some(speed)) = (ch.old_x, ch.old_y, ch.speed) {
            if (ox, oy) != (ch.x, ch.y) || speed > 1e-6 {
                let old_off = self.map_move(ox, oy);
                let old_true_x = ox as f32 + old_off.offset_x;
                let old_true_y = oy as f32 + old_off.offset_y;
                if let Some(old) = self.tiles.get_mut(&(ox, oy)) {
                    old.object_id = 0;
                    old.object_raw = "0".into();
                }
                if let Some(fc) = self.anim_frame_count.remove(&(ox, oy)) {
                    self.anim_frame_count.insert((ch.x, ch.y), fc);
                }
                self.move_state.remove(&(ox, oy));
                self.tile_flips.remove(&(ox, oy));
                self.set_map_move(
                    ch.x,
                    ch.y,
                    old_true_x - ch.x as f32,
                    old_true_y - ch.y as f32,
                    speed.max(0.0),
                );
                self.set_tile_flip_from_move(ch.x, ch.y, old_true_x);
            }
        } else if !ch.is_moving() {
            self.move_state.remove(&(ch.x, ch.y));
        }
    }

    pub fn apply_mx_many(&mut self, changes: &[MapChange]) {
        for ch in changes {
            self.apply_mx(ch);
        }
    }

    /// MX apply plus C++ moveTrans polish: source visual rewrite + E-W (move 6/7) no-flip.
    pub fn apply_mx_many_with_content(
        &mut self,
        changes: &[MapChange],
        content: &crate::content::ClientContent,
    ) {
        for ch in changes {
            self.apply_mx_with_content(ch, content);
        }
    }

    fn apply_mx_with_content(&mut self, ch: &MapChange, content: &crate::content::ClientContent) {
        // Visual object while moving: decay/move trans may replace source id (~16958).
        let mut vis = ch.clone();
        if vis.is_moving() {
            if let (Some(ox), Some(oy)) = (vis.old_x, vis.old_y) {
                let src_id = self.get(ox, oy).map(|t| t.object_id).unwrap_or(0);
                if src_id > 0 {
                    if let Some(tr) = content.find_transition(-1, src_id) {
                        if tr.move_dist > 0 && tr.new_target_id > 0 {
                            vis.object_id = tr.new_target_id;
                            if vis.object_id_raw.parse::<i32>().is_ok() {
                                vis.object_id_raw = tr.new_target_id.to_string();
                            }
                        }
                    }
                }
            }
        }
        self.apply_mx(&vis);
        // move 6/7 (E-W) → force no flip (~17066, ~17125).
        let dest_id = vis.object_id;
        if dest_id > 0 {
            if let Some(tr) = content.find_transition(-1, dest_id) {
                if tr.move_dist == 6 || tr.move_dist == 7 {
                    self.tile_flips.insert((vis.x, vis.y), false);
                    if let Some(e) = self.extra_moving.last_mut() {
                        if e.dest_x == vis.x && e.dest_y == vis.y {
                            e.flip = false;
                        }
                    }
                }
            }
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
        // Move from (0,0) → (1,0) at 3.75 cells/sec.
        let ch = parse_mx_line("1 0 0 50 0 0 0 3.75").unwrap();
        assert!(ch.is_moving());
        m.apply_mx(&ch);
        assert_eq!(m.get(1, 0).unwrap().object_id, 50);
        assert_eq!(m.get(0, 0).unwrap().object_id, 0);
        let mv = m.map_move(1, 0);
        assert!(mv.is_moving());
        assert!((mv.offset_x - (-1.0)).abs() < 1e-4, "offset_x={}", mv.offset_x);
        assert!(mv.offset_y.abs() < 1e-4);
        assert!((mv.speed - 3.75).abs() < 1e-4);
        // Step toward dest (~0.27 tiles in 1/60s * 3.75 ≈ 0.0625)
        m.step_map_moves(1.0 / 60.0);
        let mv2 = m.map_move(1, 0);
        assert!(mv2.offset_x > -1.0 && mv2.offset_x < 0.0);
        // Finish slide
        for _ in 0..120 {
            m.step_map_moves(1.0 / 60.0);
        }
        assert!(!m.map_move(1, 0).is_moving());
        // Eastward dest from west origin → face right (flip=false).
        assert!(!m.tile_flip(1, 0));
    }

    #[test]
    fn mx_move_west_sets_tile_flip() {
        let mut m = ClientMap::new();
        m.set(5, 0, MapTile::parse_cell("1:0:418"));
        // (5,0) → (4,0): moving west
        let ch = parse_mx_line("4 0 0 418 -1 5 0 1.00").unwrap();
        m.apply_mx(&ch);
        assert!(m.tile_flip(4, 0), "westward move should flipH");
        assert!((m.map_move(4, 0).offset_x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn mx_move_into_occupied_uses_extra_moving() {
        let mut m = ClientMap::new();
        m.set(0, 0, MapTile::parse_cell("1:0:418")); // wolf
        m.set(1, 0, MapTile::parse_cell("1:0:33")); // stone already at dest
        // Wolf moves onto stone cell
        let ch = parse_mx_line("1 0 0 418 -1 0 0 2.00").unwrap();
        m.apply_mx(&ch);
        assert_eq!(m.get(1, 0).unwrap().object_id, 33, "dest occupant stays");
        assert_eq!(m.get(0, 0).unwrap().object_id, 0, "source cleared");
        assert_eq!(m.extra_moving.len(), 1);
        assert_eq!(m.extra_moving[0].object_id, 418);
        assert!((m.extra_moving[0].offset_x - (-1.0)).abs() < 1e-4);
        // Finish slide → dest becomes wolf
        for _ in 0..120 {
            m.step_map_moves(1.0 / 60.0);
        }
        assert!(m.extra_moving.is_empty());
        assert_eq!(m.get(1, 0).unwrap().object_id, 418);
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

    #[test]
    fn drop_offset_slides_from_hand_to_cell() {
        let mut m = ClientMap::new();
        m.set(5, 4, MapTile::parse_cell("0:0:33"));
        // Player tile center (4.5, 4.5) → dest center (5.5, 4.5).
        let (ox, oy) = ClientMap::drop_offset_from_held(5, 4, 4.5, 4.5);
        assert!((ox + 1.0).abs() < 1e-4);
        assert!(oy.abs() < 1e-4);
        m.set_drop_offset(5, 4, ox, oy, 0.0);
        assert!(m.drop_offset(5, 4).is_sliding());
        for _ in 0..120 {
            m.step_map_moves(1.0 / 60.0);
        }
        assert!(!m.drop_offset(5, 4).is_sliding());
    }

    #[test]
    fn drop_slide_skips_use_on_bare_ground() {
        assert!(ClientMap::should_start_drop_slide(
            7, 33, false, 0, true, 33
        ));
        assert!(ClientMap::should_start_drop_slide(7, 33, false, 0, true, 0));
        assert!(!ClientMap::should_start_drop_slide(
            7, 100, false, 0, true, 33
        ));
        assert!(!ClientMap::should_start_drop_slide(
            -7, 33, false, 0, true, 33
        ));
        assert!(!ClientMap::should_start_drop_slide(
            7, 33, true, 0, true, 33
        ));
        assert!(!ClientMap::should_start_drop_slide(
            7, 33, false, 0, false, 33
        ));
    }
}
