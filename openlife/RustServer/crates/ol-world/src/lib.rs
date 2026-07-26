//! Versioned world: simple object ids + sparse complex helpers.
//!
//! - **Simple cell:** biome, floor, base object id (`i32`, 0 = empty)
//! - **Complex helper:** uses, contained ids, owner lineage id — only when needed
//! - **Binary save** `OLW1` magic + version field for efficient disk persistence
//! - **PNG biome gen** (Haxe `WorldMap.generate` color → biome id)
//! - **Natural spawn** from `ContentDb.biome_spawn` (Haxe `generateObjects`)

#![forbid(unsafe_code)]

mod biome;
mod generate;
mod journal;
mod persist;

pub use biome::{
    biome_from_rgba, biome_speed, is_biome_blocking, BiomeId, GREEN, OCEAN, PASSABLE_RIVER, RIVER,
    SNOWINGREY,
};
pub use generate::{
    generate_from_png, pick_biome_spawn, place_natural_object, spawn_natural_objects,
    GenerateOptions,
};
pub use journal::{
    JournalEntry, WorldJournal, DEFAULT_JOURNAL_MAX_BYTES, DEFAULT_JOURNAL_PATH,
};
pub use persist::{
    load_world_file, rotate_world_backups, save_world_file, save_world_file_with_options,
    world_backup_path, DEFAULT_BACKUP_KEEP, WORLD_FORMAT_VERSION,
};

use std::collections::HashMap;
use thiserror::Error;

/// Default tiles per chunk side (64×64) for resident streaming.
pub const CHUNK_SIZE: i32 = 64;

pub type ObjectId = i32;
pub type FloorId = u16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorldError {
    #[error("chunk not loaded: {0:?}")]
    ChunkMissing(ChunkCoord),
    #[error("persist: {0}")]
    Persist(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    pub fn from_tile(tx: i32, ty: i32) -> Self {
        Self {
            cx: tx.div_euclid(CHUNK_SIZE),
            cy: ty.div_euclid(CHUNK_SIZE),
        }
    }

    pub fn local(tx: i32, ty: i32) -> (usize, usize) {
        (
            tx.rem_euclid(CHUNK_SIZE) as usize,
            ty.rem_euclid(CHUNK_SIZE) as usize,
        )
    }
}

/// Sparse complex state for multi-use / containers (Haxe ObjectHelper subset).
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexObject {
    pub base_id: ObjectId,
    /// Remaining uses for multi-use objects (0 = N/A).
    pub uses_remaining: i32,
    /// Contained object ids (positive = container slots).
    pub contained: Vec<ObjectId>,
    /// One level of nesting: `nested[i]` is sub-items of `contained[i]`.
    /// Empty vec = no nesting (wire uses commas only). Parallel length when used.
    ///
    /// **Persist:** OLW2 writes nested; OLW1 loads empty nested.
    pub nested: Vec<Vec<ObjectId>>,
    /// Lineage / account owner id if any.
    pub owner_id: i32,
    /// Haxe `creationTimeInTicks` proxy — sim seconds when helper was created/refreshed.
    pub creation_time: f32,
    /// Haxe `timeToChange` — seconds until auto-transition (0 = none / permanent hold).
    pub time_to_change: f32,
}

impl ComplexObject {
    pub fn new_simple(base_id: ObjectId) -> Self {
        Self {
            base_id,
            uses_remaining: 0,
            contained: Vec::new(),
            nested: Vec::new(),
            owner_id: 0,
            creation_time: 0.0,
            time_to_change: 0.0,
        }
    }

    pub fn with_uses(base_id: ObjectId, uses: i32) -> Self {
        Self {
            base_id,
            uses_remaining: uses,
            contained: Vec::new(),
            nested: Vec::new(),
            owner_id: 0,
            creation_time: 0.0,
            time_to_change: 0.0,
        }
    }

    /// Place a simple object owned by `owner_id` (lineage / player id).
    pub fn with_owner(base_id: ObjectId, owner_id: i32) -> Self {
        Self {
            base_id,
            uses_remaining: 0,
            contained: Vec::new(),
            nested: Vec::new(),
            owner_id,
            creation_time: 0.0,
            time_to_change: 0.0,
        }
    }

    pub fn is_complex(&self) -> bool {
        self.uses_remaining > 0
            || !self.contained.is_empty()
            || !self.nested.is_empty()
            || self.owner_id != 0
            || self.time_to_change > 0.0
    }

    /// Haxe `timeUntillChange` — remaining seconds (0 if no timer).
    pub fn time_until_change(&self, sim_time: f32) -> f32 {
        if self.time_to_change <= 0.0 {
            return 0.0;
        }
        let passed = (sim_time - self.creation_time).max(0.0);
        (self.time_to_change - passed).max(0.0)
    }

    /// Haxe `isTimeToChangeReached`.
    pub fn is_time_to_change_reached(&self, sim_time: f32) -> bool {
        self.time_to_change > 0.0 && self.time_until_change(sim_time) <= 0.0
    }

    /// Stamp creation + optional decay timer (container permanence / time-in-container).
    pub fn stamp_time(&mut self, sim_time: f32, time_to_change: f32) {
        self.creation_time = sim_time;
        self.time_to_change = time_to_change.max(0.0);
    }

    /// True when this helper records a non-zero owner matching `p_id`.
    pub fn is_owner(&self, p_id: i32) -> bool {
        self.owner_id != 0 && self.owner_id == p_id
    }

    /// Haxe `ObjectHelper.toString` / `MapData.stringID` for map cells.
    ///
    /// Wire form used inside MAP_CHUNK cells (`biome:floor:obj`):
    /// - bare base: `"391"`
    /// - flat contained: `"391,33,40"` (comma-separated positive ids)
    /// - one-level nest under a slot: `"391,33:100:101,40"` (`:` sub-ids of slot 33)
    pub fn to_map_string_id(&self) -> String {
        if self.nested.is_empty() {
            encode_map_object_string(self.base_id, &self.contained)
        } else {
            encode_map_object_string_nested(self.base_id, &self.contained, &self.nested)
        }
    }
}

/// Encode object + optional contained list as Haxe map-cell object string (flat).
///
/// Matches `MapData.stringID([base, c0, c1, ...])` for non-negative ids:
/// first id, then each contained joined with `,`.
pub fn encode_map_object_string(base_id: ObjectId, contained: &[ObjectId]) -> String {
    encode_map_object_string_nested(base_id, contained, &[])
}

/// Encode object + contained + optional one-level nested sub-items (Haxe style).
///
/// - bare: `391`
/// - flat: `391,33,40`
/// - nested under slot: `391,33:100:101,40` (contained id 33 has sub 100,101)
///
/// `nested[i]` is sub-items of `contained[i]`; missing / empty entries emit no `:`.
pub fn encode_map_object_string_nested(
    base_id: ObjectId,
    contained: &[ObjectId],
    nested: &[Vec<ObjectId>],
) -> String {
    if contained.is_empty() {
        return base_id.to_string();
    }
    let mut s = base_id.to_string();
    for (i, c) in contained.iter().enumerate() {
        s.push(',');
        s.push_str(&c.to_string());
        if let Some(subs) = nested.get(i) {
            for sub in subs {
                s.push(':');
                s.push_str(&sub.to_string());
            }
        }
    }
    s
}

/// Parse Haxe map-cell object string into `(base, contained, nested)`.
///
/// Flat `"391,33,40"` yields empty `nested`. Nested form `"391,33:100:101,40"`
/// yields `nested` parallel to `contained` (`[[100,101],[]]`).
/// Invalid tokens become `0`.
pub fn parse_map_object_string(s: &str) -> (ObjectId, Vec<ObjectId>, Vec<Vec<ObjectId>>) {
    let mut parts = s.split(',');
    let Some(base_tok) = parts.next() else {
        return (0, Vec::new(), Vec::new());
    };
    let base = base_tok.parse().unwrap_or(0);
    let mut contained = Vec::new();
    let mut nested = Vec::new();
    let mut any_nested = false;
    for part in parts {
        let mut segs = part.split(':');
        let id = segs.next().and_then(|t| t.parse().ok()).unwrap_or(0);
        contained.push(id);
        let mut subs = Vec::new();
        for sub in segs {
            if let Ok(v) = sub.parse::<ObjectId>() {
                subs.push(v);
            }
        }
        if !subs.is_empty() {
            any_nested = true;
        }
        nested.push(subs);
    }
    if !any_nested {
        nested.clear();
    }
    (base, contained, nested)
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub biomes: Vec<BiomeId>,
    pub floors: Vec<FloorId>,
    /// Simple base object id per tile (0 empty). Complex data in World.helpers.
    pub objects: Vec<ObjectId>,
    pub last_sim_tick: u64,
    pub dirty: bool,
}

impl Chunk {
    pub fn empty(coord: ChunkCoord) -> Self {
        let n = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        Self {
            coord,
            biomes: vec![0; n],
            floors: vec![0; n],
            objects: vec![0; n],
            last_sim_tick: 0,
            dirty: false,
        }
    }

    pub fn idx(lx: usize, ly: usize) -> usize {
        ly * CHUNK_SIZE as usize + lx
    }

    pub fn object_at_local(&self, lx: usize, ly: usize) -> ObjectId {
        self.objects[Self::idx(lx, ly)]
    }

    pub fn set_object_local(&mut self, lx: usize, ly: usize, id: ObjectId) {
        self.objects[Self::idx(lx, ly)] = id;
        self.dirty = true;
    }

    pub fn biome_at_local(&self, lx: usize, ly: usize) -> BiomeId {
        self.biomes[Self::idx(lx, ly)]
    }

    pub fn set_biome_local(&mut self, lx: usize, ly: usize, b: BiomeId) {
        self.biomes[Self::idx(lx, ly)] = b;
        self.dirty = true;
    }

    pub fn floor_at_local(&self, lx: usize, ly: usize) -> FloorId {
        self.floors[Self::idx(lx, ly)]
    }

    pub fn set_floor_local(&mut self, lx: usize, ly: usize, f: FloorId) {
        self.floors[Self::idx(lx, ly)] = f;
        self.dirty = true;
    }
}

/// In-memory resident set of chunks + sparse complex objects.
#[derive(Debug, Default, Clone)]
pub struct World {
    chunks: HashMap<ChunkCoord, Chunk>,
    /// Key: packed tile (tx, ty) → complex state when not a bare int.
    pub helpers: HashMap<(i32, i32), ComplexObject>,
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub wrap: bool,
    /// Format version last loaded/saved.
    pub format_version: u32,
}

impl World {
    pub fn new(width_tiles: i32, height_tiles: i32, wrap: bool) -> Self {
        Self {
            chunks: HashMap::new(),
            helpers: HashMap::new(),
            width_tiles,
            height_tiles,
            wrap,
            format_version: WORLD_FORMAT_VERSION,
        }
    }

    pub fn wrap_tile(&self, mut tx: i32, mut ty: i32) -> (i32, i32) {
        if self.wrap && self.width_tiles > 0 && self.height_tiles > 0 {
            tx = tx.rem_euclid(self.width_tiles);
            ty = ty.rem_euclid(self.height_tiles);
        }
        (tx, ty)
    }

    pub fn ensure_chunk(&mut self, coord: ChunkCoord) -> &mut Chunk {
        self.chunks
            .entry(coord)
            .or_insert_with(|| Chunk::empty(coord))
    }

    pub fn resident_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn helper_count(&self) -> usize {
        self.helpers.len()
    }

    pub fn get_object(&self, tx: i32, ty: i32) -> ObjectId {
        let (tx, ty) = self.wrap_tile(tx, ty);
        if let Some(h) = self.helpers.get(&(tx, ty)) {
            return h.base_id;
        }
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.chunks
            .get(&coord)
            .map(|c| c.object_at_local(lx, ly))
            .unwrap_or(0)
    }

    pub fn set_object(&mut self, tx: i32, ty: i32, id: ObjectId) {
        let (tx, ty) = self.wrap_tile(tx, ty);
        // Clearing complex when setting bare id
        if id == 0 {
            self.helpers.remove(&(tx, ty));
        } else if let Some(h) = self.helpers.get_mut(&(tx, ty)) {
            h.base_id = id;
            // keep uses/contained unless explicitly simplified
        }
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.ensure_chunk(coord).set_object_local(lx, ly, id);
    }

    /// Place object with optional multi-use / container helper.
    pub fn set_object_complex(&mut self, tx: i32, ty: i32, helper: ComplexObject) {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let base = helper.base_id;
        if helper.is_complex() {
            self.helpers.insert((tx, ty), helper);
        } else {
            self.helpers.remove(&(tx, ty));
        }
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.ensure_chunk(coord).set_object_local(lx, ly, base);
    }

    pub fn get_helper(&self, tx: i32, ty: i32) -> Option<&ComplexObject> {
        let (tx, ty) = self.wrap_tile(tx, ty);
        self.helpers.get(&(tx, ty))
    }

    /// True if the tile has a complex helper owned by `p_id`.
    pub fn is_owner(&self, tx: i32, ty: i32, p_id: i32) -> bool {
        self.get_helper(tx, ty)
            .map(|h| h.is_owner(p_id))
            .unwrap_or(false)
    }

    /// Object id string for MAP_CHUNK / map plaintext cells (Haxe-compatible).
    ///
    /// When the tile has a complex helper with contained (or nested) items, returns
    /// the Haxe object string; otherwise a plain base id.
    pub fn encode_object_for_map(&self, tx: i32, ty: i32) -> String {
        let (tx, ty) = self.wrap_tile(tx, ty);
        if let Some(h) = self.helpers.get(&(tx, ty)) {
            if !h.contained.is_empty() || !h.nested.is_empty() {
                return h.to_map_string_id();
            }
            return h.base_id.to_string();
        }
        self.get_object(tx, ty).to_string()
    }

    /// Like [`Self::encode_object_for_map`] but multi-use tiles use a **dummy id**
    /// from `wire_id` (Haxe `ObjectHelper.dummyId` / client multi-use sprites).
    pub fn encode_object_for_map_wired(
        &self,
        tx: i32,
        ty: i32,
        wire_id: impl Fn(i32, i32) -> i32,
    ) -> String {
        let (tx, ty) = self.wrap_tile(tx, ty);
        if let Some(h) = self.helpers.get(&(tx, ty)) {
            if !h.contained.is_empty() || !h.nested.is_empty() {
                return h.to_map_string_id();
            }
            let uses = h.uses_remaining;
            let id = wire_id(h.base_id, uses);
            return id.to_string();
        }
        let base = self.get_object(tx, ty);
        wire_id(base, 0).to_string()
    }

    pub fn get_biome(&self, tx: i32, ty: i32) -> BiomeId {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.chunks
            .get(&coord)
            .map(|c| c.biome_at_local(lx, ly))
            .unwrap_or(0)
    }

    pub fn set_biome(&mut self, tx: i32, ty: i32, b: BiomeId) {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.ensure_chunk(coord).set_biome_local(lx, ly, b);
    }

    pub fn get_floor(&self, tx: i32, ty: i32) -> FloorId {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.chunks
            .get(&coord)
            .map(|c| c.floor_at_local(lx, ly))
            .unwrap_or(0)
    }

    pub fn set_floor(&mut self, tx: i32, ty: i32, f: FloorId) {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let coord = ChunkCoord::from_tile(tx, ty);
        let (lx, ly) = ChunkCoord::local(tx, ty);
        self.ensure_chunk(coord).set_floor_local(lx, ly, f);
    }

    pub fn touch_radius(&mut self, tx: i32, ty: i32, chunk_radius: i32) {
        let c = ChunkCoord::from_tile(tx, ty);
        for dy in -chunk_radius..=chunk_radius {
            for dx in -chunk_radius..=chunk_radius {
                self.ensure_chunk(ChunkCoord {
                    cx: c.cx + dx,
                    cy: c.cy + dy,
                });
            }
        }
    }

    /// Ensure all chunks covering [0,width) x [0,height) exist (for full-map gen).
    pub fn ensure_full_map_chunks(&mut self) {
        if self.width_tiles <= 0 || self.height_tiles <= 0 {
            return;
        }
        let max_cx = (self.width_tiles - 1).div_euclid(CHUNK_SIZE);
        let max_cy = (self.height_tiles - 1).div_euclid(CHUNK_SIZE);
        for cy in 0..=max_cy {
            for cx in 0..=max_cx {
                self.ensure_chunk(ChunkCoord { cx, cy });
            }
        }
    }

    /// Fast bulk fill from dense row-major arrays (used by OLW1 load).
    /// Arrays must be length width*height.
    pub fn fill_from_dense(
        &mut self,
        biomes: &[BiomeId],
        floors: &[FloorId],
        objects: &[ObjectId],
    ) -> Result<(), String> {
        let w = self.width_tiles;
        let h = self.height_tiles;
        if w <= 0 || h <= 0 {
            return Err("invalid size".into());
        }
        let n = (w as usize).saturating_mul(h as usize);
        if biomes.len() != n || floors.len() != n || objects.len() != n {
            return Err(format!(
                "dense length mismatch want {n} got b={} f={} o={}",
                biomes.len(),
                floors.len(),
                objects.len()
            ));
        }
        self.ensure_full_map_chunks();
        // Write directly into chunk vectors — O(tiles) without HashMap probes per call.
        for y in 0..h {
            for x in 0..w {
                let i = (y as usize) * (w as usize) + (x as usize);
                let coord = ChunkCoord::from_tile(x, y);
                let (lx, ly) = ChunkCoord::local(x, y);
                let idx = Chunk::idx(lx, ly);
                if let Some(chunk) = self.chunks.get_mut(&coord) {
                    chunk.biomes[idx] = biomes[i];
                    chunk.floors[idx] = floors[i];
                    chunk.objects[idx] = objects[i];
                }
            }
        }
        Ok(())
    }

    /// Export dense arrays for fast sequential save.
    pub fn export_dense(&self) -> (Vec<BiomeId>, Vec<FloorId>, Vec<ObjectId>) {
        let w = self.width_tiles.max(0) as usize;
        let h = self.height_tiles.max(0) as usize;
        let n = w * h;
        let mut biomes = vec![0u8; n];
        let mut floors = vec![0u16; n];
        let mut objects = vec![0i32; n];
        for y in 0..self.height_tiles.max(0) {
            for x in 0..self.width_tiles.max(0) {
                let i = (y as usize) * w + (x as usize);
                biomes[i] = self.get_biome(x, y);
                floors[i] = self.get_floor(x, y);
                objects[i] = self.get_object(x, y);
            }
        }
        (biomes, floors, objects)
    }

    /// Put `item` into container at (tx,ty) if space. Returns false if not a container tile.
    pub fn container_put(&mut self, tx: i32, ty: i32, item: ObjectId, max_slots: usize) -> bool {
        self.container_put_timed(tx, ty, item, max_slots, 0.0, 0.0)
    }

    /// Put with Haxe-style creation time stamp (`sim_time`) and optional `time_to_change`.
    ///
    /// Permanent containers typically pass `time_to_change = 0` (no auto-decay on the
    /// container itself). Contained items persist with the helper across OLW2 saves.
    pub fn container_put_timed(
        &mut self,
        tx: i32,
        ty: i32,
        item: ObjectId,
        max_slots: usize,
        sim_time: f32,
        time_to_change: f32,
    ) -> bool {
        if item == 0 {
            return false;
        }
        let (tx, ty) = self.wrap_tile(tx, ty);
        let base = self.get_object(tx, ty);
        if base == 0 {
            return false;
        }
        let mut helper = self
            .helpers
            .remove(&(tx, ty))
            .unwrap_or_else(|| ComplexObject::new_simple(base));
        helper.base_id = base;
        if helper.contained.len() >= max_slots {
            self.helpers.insert((tx, ty), helper);
            return false;
        }
        helper.contained.push(item);
        // Keep nested parallel when nesting is active.
        if !helper.nested.is_empty() {
            helper.nested.push(Vec::new());
        }
        // Refresh time-in-container clock (Haxe creationTime on helper mutation).
        if sim_time > 0.0 || time_to_change > 0.0 {
            helper.stamp_time(sim_time, time_to_change);
        }
        self.set_object_complex(tx, ty, helper);
        true
    }

    /// Put `item` into the nested sub-container of `contained[slot]` (one level deep).
    ///
    /// Ensures `nested` is parallel to `contained`. Fails if the tile has no base,
    /// `slot` is out of range, `item == 0`, or the sub-slot list is at `max_sub_slots`.
    pub fn container_put_nested(
        &mut self,
        tx: i32,
        ty: i32,
        slot: usize,
        item: ObjectId,
        max_sub_slots: usize,
    ) -> bool {
        if item == 0 {
            return false;
        }
        let (tx, ty) = self.wrap_tile(tx, ty);
        let base = self.get_object(tx, ty);
        if base == 0 {
            return false;
        }
        let mut helper = self
            .helpers
            .remove(&(tx, ty))
            .unwrap_or_else(|| ComplexObject::new_simple(base));
        helper.base_id = base;
        if slot >= helper.contained.len() {
            self.helpers.insert((tx, ty), helper);
            return false;
        }
        // Parallel nested rows for every contained slot.
        while helper.nested.len() < helper.contained.len() {
            helper.nested.push(Vec::new());
        }
        let subs = &mut helper.nested[slot];
        if subs.len() >= max_sub_slots {
            self.helpers.insert((tx, ty), helper);
            return false;
        }
        subs.push(item);
        self.set_object_complex(tx, ty, helper);
        true
    }

    /// Remove contained item at slot (or last if slot is None). Returns item id.
    ///
    /// Nested sub-items under that slot are discarded (held is a single id in sim).
    pub fn container_take(&mut self, tx: i32, ty: i32, slot: Option<usize>) -> Option<ObjectId> {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let mut helper = self.helpers.remove(&(tx, ty))?;
        if helper.contained.is_empty() {
            self.helpers.insert((tx, ty), helper);
            return None;
        }
        let idx = slot.unwrap_or(helper.contained.len() - 1);
        if idx >= helper.contained.len() {
            self.helpers.insert((tx, ty), helper);
            return None;
        }
        let item = helper.contained.remove(idx);
        if idx < helper.nested.len() {
            helper.nested.remove(idx);
        }
        // Drop empty nesting so wire stays flat (`base,c0,c1`).
        if helper.nested.iter().all(|s| s.is_empty()) {
            helper.nested.clear();
        }
        self.set_object_complex(tx, ty, helper);
        Some(item)
    }

    /// Take a nested sub-item under `contained[slot]` (one level deep).
    ///
    /// `sub` `None` removes the last sub-item in that pocket. Returns the item id,
    /// or `None` if slot/sub is missing. Parent contained id is left in place.
    pub fn container_take_nested(
        &mut self,
        tx: i32,
        ty: i32,
        slot: usize,
        sub: Option<usize>,
    ) -> Option<ObjectId> {
        let (tx, ty) = self.wrap_tile(tx, ty);
        let mut helper = self.helpers.remove(&(tx, ty))?;
        if slot >= helper.contained.len() {
            self.helpers.insert((tx, ty), helper);
            return None;
        }
        if slot >= helper.nested.len() || helper.nested[slot].is_empty() {
            self.helpers.insert((tx, ty), helper);
            return None;
        }
        let idx = sub.unwrap_or(helper.nested[slot].len() - 1);
        if idx >= helper.nested[slot].len() {
            self.helpers.insert((tx, ty), helper);
            return None;
        }
        let item = helper.nested[slot].remove(idx);
        if helper.nested.iter().all(|s| s.is_empty()) {
            helper.nested.clear();
        }
        self.set_object_complex(tx, ty, helper);
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_and_complex_cells() {
        let mut w = World::new(128, 128, true);
        w.set_object(10, 20, 33);
        assert_eq!(w.get_object(10, 20), 33);
        assert!(w.get_helper(10, 20).is_none());

        w.set_object_complex(
            11,
            21,
            ComplexObject {
                base_id: 391,
                uses_remaining: 3,
                contained: vec![33, 40],
                nested: Vec::new(),
                owner_id: 42,
                creation_time: 0.0,
                time_to_change: 0.0,
            },
        );
        assert_eq!(w.get_object(11, 21), 391);
        let h = w.get_helper(11, 21).unwrap();
        assert_eq!(h.uses_remaining, 3);
        assert_eq!(h.contained, vec![33, 40]);
        assert!(h.nested.is_empty());
    }

    #[test]
    fn map_object_string_encodes_contained() {
        assert_eq!(encode_map_object_string(33, &[]), "33");
        assert_eq!(encode_map_object_string(391, &[33, 40]), "391,33,40");
        let h = ComplexObject {
            base_id: 391,
            uses_remaining: 0,
            contained: vec![33, 40],
            nested: Vec::new(),
            owner_id: 0,
            creation_time: 0.0,
            time_to_change: 0.0,
        };
        assert_eq!(h.to_map_string_id(), "391,33,40");

        let mut w = World::new(64, 64, false);
        w.set_object_complex(1, 1, h);
        assert_eq!(w.encode_object_for_map(1, 1), "391,33,40");
        w.set_object(2, 2, 99);
        assert_eq!(w.encode_object_for_map(2, 2), "99");
    }

    #[test]
    fn map_object_string_nested_encode_decode_roundtrip() {
        // bare
        assert_eq!(encode_map_object_string_nested(391, &[], &[]), "391");
        // flat (empty nested slice)
        assert_eq!(
            encode_map_object_string_nested(391, &[33, 40], &[]),
            "391,33,40"
        );
        // nested under first slot
        let nested = vec![vec![100, 101], vec![]];
        assert_eq!(
            encode_map_object_string_nested(391, &[33, 40], &nested),
            "391,33:100:101,40"
        );

        let h = ComplexObject {
            base_id: 391,
            uses_remaining: 0,
            contained: vec![33, 40],
            nested: nested.clone(),
            owner_id: 0,
            creation_time: 0.0,
            time_to_change: 0.0,
        };
        assert!(h.is_complex());
        assert_eq!(h.to_map_string_id(), "391,33:100:101,40");

        let (base, contained, parsed_nested) = parse_map_object_string("391,33:100:101,40");
        assert_eq!(base, 391);
        assert_eq!(contained, vec![33, 40]);
        assert_eq!(parsed_nested, vec![vec![100, 101], vec![]]);

        // flat parse keeps nested empty
        let (b2, c2, n2) = parse_map_object_string("391,33,40");
        assert_eq!(b2, 391);
        assert_eq!(c2, vec![33, 40]);
        assert!(n2.is_empty());

        // bare
        let (b3, c3, n3) = parse_map_object_string("391");
        assert_eq!(b3, 391);
        assert!(c3.is_empty());
        assert!(n3.is_empty());

        // encode → parse round-trip
        let wire = encode_map_object_string_nested(10, &[20, 30], &[vec![1], vec![2, 3]]);
        assert_eq!(wire, "10,20:1,30:2:3");
        let (pb, pc, pn) = parse_map_object_string(&wire);
        assert_eq!(pb, 10);
        assert_eq!(pc, vec![20, 30]);
        assert_eq!(pn, vec![vec![1], vec![2, 3]]);
    }

    #[test]
    fn container_put_take_nested_and_encode() {
        let mut w = World::new(64, 64, false);
        // Basket 391 with bag 292 in slot 0, then nest berries under the bag.
        w.set_object(1, 1, 391);
        assert!(w.container_put(1, 1, 292, 4));
        assert!(w.container_put(1, 1, 40, 4));
        // Nested put into slot 0 (the bag)
        assert!(w.container_put_nested(1, 1, 0, 100, 4));
        assert!(w.container_put_nested(1, 1, 0, 101, 4));
        // Full sub-slots (max 2; already have 100,101)
        assert!(!w.container_put_nested(1, 1, 0, 102, 2));
        // Bad slot
        assert!(!w.container_put_nested(1, 1, 9, 99, 4));
        // Zero item
        assert!(!w.container_put_nested(1, 1, 0, 0, 4));

        let h = w.get_helper(1, 1).unwrap();
        assert_eq!(h.contained, vec![292, 40]);
        assert_eq!(h.nested, vec![vec![100, 101], vec![]]);
        assert_eq!(h.to_map_string_id(), "391,292:100:101,40");
        assert_eq!(w.encode_object_for_map(1, 1), "391,292:100:101,40");

        // Take nested sub at explicit index
        assert_eq!(w.container_take_nested(1, 1, 0, Some(0)), Some(100));
        assert_eq!(
            w.get_helper(1, 1).unwrap().nested,
            vec![vec![101], vec![]]
        );
        // Take last nested (sub None)
        assert_eq!(w.container_take_nested(1, 1, 0, None), Some(101));
        // Nested emptied → collapsed for flat wire
        let h = w.get_helper(1, 1).unwrap();
        assert!(h.nested.is_empty());
        assert_eq!(h.to_map_string_id(), "391,292,40");
        assert_eq!(w.container_take_nested(1, 1, 0, None), None);

        // Parent take still works
        assert_eq!(w.container_take(1, 1, Some(0)), Some(292));
        assert_eq!(w.get_helper(1, 1).unwrap().contained, vec![40]);
    }

    #[test]
    fn chunk_coord_from_tile() {
        assert_eq!(ChunkCoord::from_tile(64, 0), ChunkCoord { cx: 1, cy: 0 });
    }

    #[test]
    fn wrap_toroidal() {
        let mut w = World::new(100, 50, true);
        w.set_object(0, 0, 7);
        assert_eq!(w.get_object(100, 50), 7);
    }

    #[test]
    fn is_owner_on_complex_and_world() {
        let mut w = World::new(64, 64, false);
        w.set_object_complex(3, 4, ComplexObject::with_owner(33, 99));
        let h = w.get_helper(3, 4).unwrap();
        assert!(h.is_owner(99));
        assert!(!h.is_owner(1));
        assert!(!h.is_owner(0));
        assert!(w.is_owner(3, 4, 99));
        assert!(!w.is_owner(3, 4, 1));
        // Unowned / simple tile
        w.set_object(5, 5, 33);
        assert!(!w.is_owner(5, 5, 99));
        assert!(!ComplexObject::new_simple(33).is_owner(0));
    }
}
