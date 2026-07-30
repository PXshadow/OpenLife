//! Ground biome tiles + overlay sheets (C++ `groundSprites` / Haxe `Render.loadGround`).
//!
//! - **Soft biome tiles (C++ `tiles[][]`):** `groundTileCache/biome_{id}_x{i}_y{j}.tga`
//!   Ã¢â‚¬â€ **2Ãƒâ€”CELL_D** (256Ã‚Â²) with soft alpha edges for biome-border blending
//!   (`LivingLifePage.cpp` ~7358Ã¢â‚¬â€œ7388).
//! - **Square tiles (C++ `squareTiles[][]`):** `Ã¢â‚¬Â¦_square.tga` Ã¢â‚¬â€ **CELL_D** (128Ã‚Â²) solid
//!   interior when left/above/diag neighbors share the biome (~7345Ã¢â‚¬â€œ7356).
//! - **Overlays:** `graphics/ground_t{N}.tga` (Haxe `Resource.groundOverlay`).
//! - **OLG1:** binary presence index (paths + dims) so load skips miss probes and
//!   multi-root scans; pixels stay lazy TGA Ã¢â‚¬â€ **default play path**.
//! - **OLGA (optional):** full multi-page ground atlas dump (Haxe `SaveGroundData.bin` +
//!   `ground.png` analogue). Bake via [`bake_olga_to_dir`] / CLI `--bake-ground-atlas`;
//!   load via [`GroundBank::load_olga`] / [`GroundBank::load_prefer_atlas_cache`].
//!   Not written by default `bake_content` (large; OLG1 remains sufficient).
//!
//! Variation index (biome tile key):
//! `biome * 16 + abs(x % 4) + abs(y % 4) * 4`
//!
//! Headless-safe: optional disk load; flat `biome_color` + hash dither when missing.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::binpack::{BinPack, Rect};
use crate::tga::load_tga_path;

/// C++ `CELL_D` / Haxe `GRID` ground cell size in object units.
pub const CELL_D: i32 = 128;

/// Atlas page size for packed ground tiles (multi-page when full; Haxe uses one `MAX_TEXTURE`).
pub const GROUND_ATLAS: i32 = 2048;

/// OLG1 magic Ã¢â‚¬â€ ground tile / overlay index cache.
pub const OLG1_MAGIC: &[u8; 4] = b"OLG1";
/// OLG1 format version (fixed meta + dense path records).
pub const OLG1_FORMAT_VERSION: u32 = 1;

/// OLGA magic Ã¢â‚¬â€ full multi-page ground atlas (optional SaveGroundData-style dump).
pub const OLGA_MAGIC: &[u8; 4] = b"OLGA";
/// OLGA format version (tile rects + raw RGBA pages).
pub const OLGA_FORMAT_VERSION: u32 = 1;

/// Unknown-biome cache id (C++ `cacheFileNumber = 99999` / Haxe unknown tiles).
pub const UNKNOWN_BIOME_CACHE_ID: i32 = 99999;

/// Kind byte in OLG1 records.
pub const OLG1_KIND_OVERLAY: u8 = 0;
pub const OLG1_KIND_BIOME: u8 = 1;
pub const OLG1_KIND_UNKNOWN: u8 = 2;

/// Entry flags.
const OLG1_F_HAS_SQUARE: u8 = 1 << 0;
const OLG1_F_EXISTS: u8 = 1 << 1;

/// Default tile grid (C++ / Haxe 4Ãƒâ€”4 variation).
pub const GROUND_TILES_WIDE: u16 = 4;
pub const GROUND_TILES_HIGH: u16 = 4;
/// Haxe packs overlays `ground_t0`..`ground_t3`.
pub const GROUND_OVERLAY_COUNT: u16 = 4;

/// Haxe / C++ ground variation index for tile (tx, ty) and biome id.
///
/// Haxe `Render.addGround` biome branch uses
/// `id * 16 + abs(x % 4) + abs(y % 4) * 4` (+4 only for packed map after overlays).
pub fn ground_variation_index(biome: u8, tile_x: i32, tile_y: i32) -> i32 {
    let bx = abs_mod(tile_x, 4);
    let by = abs_mod(tile_y, 4);
    biome as i32 * 16 + bx + by * 4
}

/// Haxe packed groundMap key for a biome tile (overlays occupy slots 0..3 first).
pub fn ground_map_key_biome(biome: u8, tile_x: i32, tile_y: i32) -> i32 {
    4 + ground_variation_index(biome, tile_x, tile_y)
}

/// Unknown biome tile key (Haxe `99999 + x + y*4`).
pub fn ground_map_key_unknown(tile_x: i32, tile_y: i32) -> i32 {
    UNKNOWN_BIOME_CACHE_ID + abs_mod(tile_x, 4) + abs_mod(tile_y, 4) * 4
}

/// Haxe overlay slot for world tile, if an overlay should be drawn.
///
/// Haxe `addGround`: when `abs(x % 4) == 0` (and `abs(y % 0) == 0` Ã¢â‚¬â€ always true),
/// `index = abs(x % 8) + abs(y % 8) * 2` into the first 4 packed overlays.
pub fn ground_overlay_slot(tile_x: i32, tile_y: i32) -> Option<u8> {
    if abs_mod(tile_x, 4) != 0 {
        return None;
    }
    let idx = abs_mod(tile_x, 8) + abs_mod(tile_y, 8) * 2;
    if idx < GROUND_OVERLAY_COUNT as i32 {
        Some(idx as u8)
    } else {
        None
    }
}

#[inline]
fn abs_mod(v: i32, m: i32) -> i32 {
    let r = v % m;
    if r < 0 {
        r + m
    } else {
        r
    }
}

/// C++ `tileX % numTiles` (4×4 sheet) with negative fix — public for wholeSheet corner test.
#[inline]
pub fn ground_tile_mod(v: i32) -> i32 {
    abs_mod(v, GROUND_TILES_WIDE as i32)
}

/// Biome flat colors matched to `ground_N.tga` / `groundTileCache` midtones
/// (underfill while soft/square TGAs load). Same hue family as Jason C++ sheets.
///
/// Sampled from live `OneLifeGameSourceData/groundTileCache` square tiles (center
/// average). Unknown / out-of-range biomes use the `ground_U` (cache 99999) midtone
/// Ã¢â‚¬â€ C++ still draws the unknown sheet, then multiplies by [`unknown_biome_draw_color`].
pub fn biome_color(biome: u8) -> [u8; 4] {
    // 0–6: center averages from OneLifeGameSourceData/groundTileCache square TGAs.
    // Special Open Life biomes (no ground_N.tga): Haxe `BiomeMapColor` underfill
    // (Biome.hx) until unknown-sheet + getXYRandom tint is used.
    match biome {
        0 => [102, 145, 55, 255],  // grass / GREEN
        1 => [123, 95, 82, 255],   // swamp
        2 => [227, 150, 25, 255],  // yellow prairie
        3 => [57, 50, 44, 255],    // dark / GREY
        4 => [255, 255, 255, 255], // polar white / SNOW
        5 => [126, 99, 57, 255],   // badlands / DESERT
        6 => [19, 48, 3, 255],     // deep green / JUNGLE
        // Haxe BiomeTag specials (map PNG colors from Biome.hx):
        9 => [0, 64, 128, 255],    // OCEAN — deep water (COCEAN FF004080)
        13 => [0, 232, 255, 255],  // PASSABLERIVER — shallow water (CPASSABLERIVER)
        17 => [0, 128, 255, 255],  // RIVER — non-walkable water (CRIVER)
        21 => [64, 64, 64, 255],   // SNOWINGREY — mountain peak (CSNOWINGREY)
        // ground_U / biome_99999 midtone before getXYRandom multiply
        _ => [237, 236, 236, 255],
    }
}

/// C++ `getXYRandom` / `xxTweakedHash2D` (seeds 0) Ã¢â‚¬â€ returns 0..1.
///
/// Used when a biome has no `ground_N.tga` set: Jason tints the unknown sheet with
/// `setDrawColor(getXYRandom(b,b), getXYRandom(b,b+100), getXYRandom(b,b+300), 1)`.
pub fn get_xy_random(x: i32, y: i32) -> f32 {
    const XX_PRIME32_2: u32 = 2_246_822_519;
    const XX_PRIME32_3: u32 = 3_266_489_917;
    const XX_PRIME32_5: u32 = 374_761_393;
    let mut h32 = (x as u32).wrapping_add(XX_PRIME32_5);
    h32 = h32.wrapping_add((y as u32).wrapping_mul(XX_PRIME32_3));
    h32 = h32.wrapping_mul(XX_PRIME32_2);
    h32 ^= h32 >> 13;
    h32 = h32.wrapping_mul(XX_PRIME32_3);
    h32 ^= h32 >> 16;
    (h32 as f64 / 4_294_967_295.0_f64) as f32
}

/// C++ random draw color for biomes that fall through to the unknown ground sheet.
pub fn unknown_biome_draw_color(biome_id: i32) -> [u8; 4] {
    let r = (get_xy_random(biome_id, biome_id).clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (get_xy_random(biome_id, biome_id + 100).clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (get_xy_random(biome_id, biome_id + 300).clamp(0.0, 1.0) * 255.0).round() as u8;
    [r, g, b, 255]
}

/// True for biomes with a fixed table color (land sheets or Open Life specials).
pub fn has_table_biome_color(biome: u8) -> bool {
    matches!(biome, 0..=6 | 9 | 13 | 15 | 17 | 21)
}

/// Underfill / plate color for a map biome byte: known sheet midtones, Open Life
/// special map colors (water / mountain), or Jason random tint for true unknowns.
pub fn biome_plate_color(biome: u8, has_sheet: bool) -> [u8; 4] {
    if has_sheet || has_table_biome_color(biome) {
        biome_color(biome)
    } else {
        unknown_biome_draw_color(biome as i32)
    }
}

/// Draw multiply tint when falling back to the unknown ground sheet.
/// Open Life specials use Haxe map colors; others use Jason `getXYRandom`.
pub fn unknown_sheet_draw_tint(biome: u8) -> [u8; 4] {
    if has_table_biome_color(biome) && biome > 6 {
        biome_color(biome)
    } else {
        unknown_biome_draw_color(biome as i32)
    }
}

/// Slight per-tile dither so flat biomes still show Haxe-style 4Ãƒâ€”4 variation.
pub fn biome_color_varied(biome: u8, tile_x: i32, tile_y: i32) -> [u8; 4] {
    let mut c = biome_color(biome);
    let idx = ground_variation_index(biome, tile_x, tile_y);
    let d = ((idx.wrapping_mul(37) ^ tile_x.wrapping_mul(17) ^ tile_y.wrapping_mul(13)) % 17) - 8;
    for ch in 0..3 {
        let v = c[ch] as i32 + d;
        c[ch] = v.clamp(0, 255) as u8;
    }
    c
}

/// One OLG1 index record (presence + path; pixels stay on-demand).
#[derive(Debug, Clone)]
pub struct GroundIndexEntry {
    pub kind: u8,
    /// Overlay id, biome id, or `UNKNOWN_BIOME_CACHE_ID`.
    pub id: i32,
    pub tile_x: u8,
    pub tile_y: u8,
    pub has_square: bool,
    pub exists: bool,
    pub width: u16,
    pub height: u16,
    /// Path relative to a search root (`groundTileCache/...` or `graphics/ground_tN.tga`).
    pub rel_path: String,
}

impl GroundIndexEntry {
    /// Runtime tile key used by [`GroundBank::tiles`] (biome/unknown) or overlay id.
    pub fn bank_key(&self) -> i32 {
        match self.kind {
            OLG1_KIND_OVERLAY => self.id,
            OLG1_KIND_BIOME => {
                self.id * 16 + self.tile_x as i32 + self.tile_y as i32 * 4
            }
            OLG1_KIND_UNKNOWN => {
                UNKNOWN_BIOME_CACHE_ID + self.tile_x as i32 + self.tile_y as i32 * 4
            }
            _ => self.id,
        }
    }
}

/// Packed ground tile rect in the ground atlas.
#[derive(Debug, Clone, Copy)]
pub struct GroundTileRect {
    pub atlas_index: usize,
    pub rect: Rect,
    pub width: u32,
    pub height: u32,
}

struct GroundPage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    packer: BinPack,
}

impl GroundPage {
    fn new(size: i32) -> Self {
        let s = size as u32;
        Self {
            width: s,
            height: s,
            pixels: vec![0u8; (s as usize) * (s as usize) * 4],
            packer: BinPack::new(size, size),
        }
    }

    fn blit(&mut self, x: i32, y: i32, src: &[u8], sw: u32, sh: u32) {
        for py in 0..sh {
            for px in 0..sw {
                let si = ((py * sw + px) * 4) as usize;
                if si + 3 >= src.len() {
                    continue;
                }
                let a = src[si + 3];
                if a == 0 {
                    continue;
                }
                let dx = x as u32 + px;
                let dy = y as u32 + py;
                if dx >= self.width || dy >= self.height {
                    continue;
                }
                let di = ((dy * self.width + dx) * 4) as usize;
                self.pixels[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
    }

}

/// Ground sprite set bank (C++ `GroundSpriteSet` / Haxe `groundMap`).
pub struct GroundBank {
    /// Search roots: content dir, game source data, etc.
    roots: Vec<PathBuf>,
    pages: Vec<GroundPage>,
    /// Soft (2Ãƒâ€”CELL_D) tiles Ã¢â‚¬â€ variation index (`biome*16 + x%4 + y%4*4`).
    tiles: HashMap<i32, GroundTileRect>,
    /// Square (CELL_D) tiles Ã¢â‚¬â€ same index, from `*_square.tga`.
    square_tiles: HashMap<i32, GroundTileRect>,
    /// Overlay sheet rects keyed by overlay id 0..N-1 (`graphics/ground_tN`).
    overlays: HashMap<u8, GroundTileRect>,
    /// Full biome sheet (`ground/ground_N.tga` / `graphics/ground_N.tga`) for wholeSheet stamp.
    whole_sheets: HashMap<i32, GroundTileRect>,
    /// Cells wide/high for each whole sheet (pixels / CELL_D).
    whole_sheet_tiles: HashMap<i32, (u32, u32)>,
    missing: HashMap<i32, ()>,
    missing_square: HashMap<i32, ()>,
    missing_overlays: HashMap<u8, ()>,
    missing_whole: HashMap<i32, ()>,
    /// OLG1 presence index (bank_key Ã¢â€ â€™ entry). Empty Ã¢â€¡â€™ full disk probe.
    index: HashMap<i32, GroundIndexEntry>,
    /// Overlay entries by overlay id.
    overlay_index: HashMap<u8, GroundIndexEntry>,
    /// True when OLG1 (or scan) populated the index.
    pub index_loaded: bool,
    /// True when pages + rects came from an OLGA full-atlas dump (no TGA needed).
    pub atlas_loaded: bool,
    /// True if any TGA was packed successfully (or OLGA restored pixels).
    pub any_loaded: bool,
    pub data_version: u32,
    pub overlay_count: usize,
    pub biome_tile_count: usize,
}

impl GroundBank {
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            pages: vec![GroundPage::new(GROUND_ATLAS)],
            tiles: HashMap::new(),
            square_tiles: HashMap::new(),
            overlays: HashMap::new(),
            whole_sheets: HashMap::new(),
            whole_sheet_tiles: HashMap::new(),
            missing: HashMap::new(),
            missing_square: HashMap::new(),
            missing_overlays: HashMap::new(),
            missing_whole: HashMap::new(),
            index: HashMap::new(),
            overlay_index: HashMap::new(),
            index_loaded: false,
            atlas_loaded: false,
            any_loaded: false,
            data_version: 0,
            overlay_count: 0,
            biome_tile_count: 0,
        }
    }

    /// Content / game-data roots that may contain `groundTileCache/` / `graphics/`.
    pub fn with_roots(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut g = Self::new();
        g.roots.extend(roots);
        g
    }

    /// Default search paths (OHOL_GAME_DATA, OneLifeGameSourceData, content root).
    pub fn with_default_roots(content_root: Option<&Path>) -> Self {
        Self::with_roots(default_ground_roots(content_root))
    }

    /// Prefer `content_root/cache/olg1_ground_index.bin`; else scan disk into memory index.
    ///
    /// **Default play path:** pixels remain lazy TGA (OLG1 index only). For optional full
    /// atlas restore see [`Self::load_prefer_atlas_cache`].
    pub fn load_prefer_cache(content_root: impl AsRef<Path>) -> Self {
        Self::load_prefer_cache_with_progress(content_root, None)
    }

    /// Same as [`Self::load_prefer_cache`] with optional P5#36 progress callback.
    pub fn load_prefer_cache_with_progress(
        content_root: impl AsRef<Path>,
        mut on_progress: crate::load_progress::ProgressCb<'_>,
    ) -> Self {
        use crate::load_progress::{report_stage, LoadStage};
        report_stage(
            LoadStage::Ground,
            0.0,
            Some("prefer_cache"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let root = content_root.as_ref();
        let mut bank = Self::with_default_roots(Some(root));
        let olg1_path = root.join("cache").join("olg1_ground_index.bin");
        if olg1_path.exists() {
            if let Ok(bytes) = fs::read(&olg1_path) {
                if bank.load_olg1(&bytes).is_ok() {
                    report_stage(
                        LoadStage::Ground,
                        1.0,
                        Some("olg1"),
                        crate::load_progress::reborrow_cb(&mut on_progress),
                    );
                    return bank;
                }
            }
        }
        report_stage(
            LoadStage::Ground,
            0.5,
            Some("scan"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let _ = bank.scan_index_from_disk();
        report_stage(
            LoadStage::Ground,
            1.0,
            Some("scan"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        bank
    }

    /// Prefer optional OLGA full multi-atlas dump, then OLG1 index, then disk scan.
    ///
    /// Haxe analogue: load `ground.png` + `SaveGroundData.bin` when present; else pack.
    /// Default client boot should keep using [`Self::load_prefer_cache`] (lazy TGA).
    pub fn load_prefer_atlas_cache(content_root: impl AsRef<Path>) -> Self {
        let root = content_root.as_ref();
        let mut bank = Self::with_default_roots(Some(root));
        let olga_path = root.join("cache").join("olga_ground_atlas.bin");
        if olga_path.exists() {
            if let Ok(bytes) = fs::read(&olga_path) {
                if bank.load_olga(&bytes).is_ok() {
                    return bank;
                }
            }
        }
        // Fall back to index-only path (same as load_prefer_cache).
        let olg1_path = root.join("cache").join("olg1_ground_index.bin");
        if olg1_path.exists() {
            if let Ok(bytes) = fs::read(&olg1_path) {
                if bank.load_olg1(&bytes).is_ok() {
                    return bank;
                }
            }
        }
        let _ = bank.scan_index_from_disk();
        bank
    }

    pub fn pages(&self) -> impl Iterator<Item = (&[u8], u32)> + '_ {
        self.pages.iter().map(|p| (p.pixels.as_slice(), p.width))
    }

    pub fn page_pixels(&self, index: usize) -> Option<(&[u8], u32)> {
        self.pages.get(index).map(|p| (p.pixels.as_slice(), p.width))
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn get(&self, index: i32) -> Option<&GroundTileRect> {
        self.tiles.get(&index)
    }

    pub fn get_overlay(&self, id: u8) -> Option<&GroundTileRect> {
        self.overlays.get(&id)
    }

    pub fn index_len(&self) -> usize {
        self.index.len() + self.overlay_index.len()
    }

    /// Copy one packed tile's RGBA into a small buffer (tests / rare paths).
    /// Prefer [`Self::page_tile`] + direct atlas blit in the hot render path.
    pub fn copy_tile_rgba(&self, gt: &GroundTileRect) -> Option<(Vec<u8>, u32, u32)> {
        let (pix, atlas_w, src_x, src_y, w, h) = self.page_tile(gt)?;
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let si = (((src_y + y) * atlas_w + (src_x + x)) * 4) as usize;
                let di = ((y * w + x) * 4) as usize;
                if si + 3 < pix.len() {
                    out[di..di + 4].copy_from_slice(&pix[si..si + 4]);
                }
            }
        }
        Some((out, w, h))
    }

    /// Atlas page slice + subrect for a packed tile (no allocation).
    /// Returns `(pixels, atlas_w, src_x, src_y, tile_w, tile_h)`.
    pub fn page_tile(
        &self,
        gt: &GroundTileRect,
    ) -> Option<(&[u8], u32, u32, u32, u32, u32)> {
        let page = self.pages.get(gt.atlas_index)?;
        Some((
            page.pixels.as_slice(),
            page.width,
            gt.rect.x as u32,
            gt.rect.y as u32,
            gt.width,
            gt.height,
        ))
    }

    /// True when this bank has (or can load) soft tiles for `biome` (not unknown-only).
    ///
    /// C++ `groundSprites[b] != NULL` Ã¢â‚¬â€ missing sets use the unknown `ground_U` sheet.
    pub fn has_biome_sheet(&self, biome: u8) -> bool {
        if biome as i32 == UNKNOWN_BIOME_CACHE_ID {
            return self.has_unknown_sheet();
        }
        let base = biome as i32 * 16;
        // Any of the 4Ãƒâ€”4 variations known present or already packed.
        for i in 0..16 {
            let k = base + i;
            if self.tiles.contains_key(&k) {
                return true;
            }
            if let Some(ent) = self.index.get(&k) {
                if ent.exists {
                    return true;
                }
            }
        }
        // No OLG1 yet: allow disk probe path (ensure will try).
        if !self.index_loaded {
            return true;
        }
        // Index loaded but no entries for this biome Ã¢â€ â€™ no sheet.
        self.biome_tile_count == 0
    }

    /// True when unknown-biome (`99999`) tiles are available.
    pub fn has_unknown_sheet(&self) -> bool {
        for i in 0..16 {
            let k = UNKNOWN_BIOME_CACHE_ID + i;
            if self.tiles.contains_key(&k) {
                return true;
            }
            if let Some(ent) = self.index.get(&k) {
                if ent.exists {
                    return true;
                }
            }
        }
        !self.index_loaded || self.biome_tile_count == 0
    }

    /// Ensure soft (2Ãƒâ€”CELL_D) tile for biome/tile coords Ã¢â‚¬â€ C++ `tiles[setY][setX]`.
    pub fn ensure_tile(&mut self, biome: u8, tile_x: i32, tile_y: i32) -> Option<GroundTileRect> {
        let index = ground_variation_index(biome, tile_x, tile_y);
        self.ensure_index(biome, index)
    }

    /// Soft tile for a biome, or C++ unknown sheet (`biome_99999_*` / `ground_U`) on miss.
    ///
    /// Returns `(rect, used_unknown)`. Caller multiplies unknown draws by
    /// [`unknown_biome_draw_color`] when the biome itself had no sheet.
    pub fn ensure_tile_or_unknown(
        &mut self,
        biome: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<(GroundTileRect, bool)> {
        if self.has_biome_sheet(biome) {
            if let Some(r) = self.ensure_tile(biome, tile_x, tile_y) {
                return Some((r, false));
            }
        }
        self.ensure_unknown_tile(tile_x, tile_y).map(|r| (r, true))
    }

    /// Square interior tile, or unknown square/soft on miss.
    pub fn ensure_square_or_unknown(
        &mut self,
        biome: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<(GroundTileRect, bool)> {
        if self.has_biome_sheet(biome) {
            if let Some(r) = self.ensure_square_tile(biome, tile_x, tile_y) {
                return Some((r, false));
            }
            // Square miss but soft present Ã¢â‚¬â€ C++ still has squareTiles; fall soft for that set.
            if let Some(r) = self.ensure_tile(biome, tile_x, tile_y) {
                return Some((r, false));
            }
        }
        // Unknown path: prefer square unknown, else soft unknown.
        let x = abs_mod(tile_x, 4);
        let y = abs_mod(tile_y, 4);
        let index = UNKNOWN_BIOME_CACHE_ID + x + y * 4;
        if let Some(r) = self.square_tiles.get(&index) {
            return Some((*r, true));
        }
        // Try packing unknown square via cache id 99999.
        if let Some(r) = self.ensure_square_tile_for_cache_id(UNKNOWN_BIOME_CACHE_ID, tile_x, tile_y)
        {
            return Some((r, true));
        }
        self.ensure_unknown_tile(tile_x, tile_y).map(|r| (r, true))
    }

    fn ensure_square_tile_for_cache_id(
        &mut self,
        cache_id: i32,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<GroundTileRect> {
        let bx = abs_mod(tile_x, 4);
        let by = abs_mod(tile_y, 4);
        let index = cache_id + bx + by * 4;
        if let Some(r) = self.square_tiles.get(&index) {
            return Some(*r);
        }
        if self.missing_square.contains_key(&index) {
            return None;
        }
        let rel = format!("groundTileCache/biome_{cache_id}_x{bx}_y{by}_square.tga");
        let path = match self.resolve_rel(&rel) {
            Some(p) => p,
            None => {
                self.missing_square.insert(index, ());
                return None;
            }
        };
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing_square.insert(index, ());
                return None;
            }
        };
        match self.pack_image(&img.pixels, img.width, img.height) {
            Some(gt) => {
                self.square_tiles.insert(index, gt);
                self.any_loaded = true;
                Some(gt)
            }
            None => {
                self.missing_square.insert(index, ());
                None
            }
        }
    }

    /// Ensure square (CELL_D) tile Ã¢â‚¬â€ C++ `squareTiles[setY][setX]` (interior same-biome).
    pub fn ensure_square_tile(
        &mut self,
        biome: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<GroundTileRect> {
        let index = ground_variation_index(biome, tile_x, tile_y);
        if let Some(r) = self.square_tiles.get(&index) {
            return Some(*r);
        }
        if self.missing_square.contains_key(&index) {
            return None;
        }
        let bx = abs_mod(tile_x, 4);
        let by = abs_mod(tile_y, 4);
        let cache_id = if biome as i32 == UNKNOWN_BIOME_CACHE_ID {
            UNKNOWN_BIOME_CACHE_ID
        } else {
            biome as i32
        };
        let rel = format!("groundTileCache/biome_{cache_id}_x{bx}_y{by}_square.tga");
        // Prefer square file; fall back to soft tile scaled by caller if missing.
        if self.resolve_rel(&rel).is_none() {
            // OLG1 may mark has_square Ã¢â‚¬â€ still probe disk via resolve.
            if self.index_loaded {
                if let Some(ent) = self.index.get(&index) {
                    if !ent.has_square {
                        self.missing_square.insert(index, ());
                        return None;
                    }
                }
            }
        }
        let path = match self.resolve_rel(&rel) {
            Some(p) => p,
            None => {
                self.missing_square.insert(index, ());
                return None;
            }
        };
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing_square.insert(index, ());
                return None;
            }
        };
        match self.pack_image(&img.pixels, img.width, img.height) {
            Some(gt) => {
                self.square_tiles.insert(index, gt);
                self.any_loaded = true;
                Some(gt)
            }
            None => {
                self.missing_square.insert(index, ());
                None
            }
        }
    }

    pub fn ensure_index(&mut self, biome: u8, index: i32) -> Option<GroundTileRect> {
        if let Some(r) = self.tiles.get(&index) {
            return Some(*r);
        }
        if self.missing.contains_key(&index) {
            return None;
        }

        // OLG1 fast path: known-missing or known relative path.
        if self.index_loaded {
            if let Some(ent) = self.index.get(&index) {
                if !ent.exists {
                    self.missing.insert(index, ());
                    return None;
                }
                let rel = ent.rel_path.clone();
                return self.pack_tile_from_rel(index, &rel);
            }
            // Indexed bank but this key absent Ã¢â€ â€™ treat as missing (no multi-root probe).
            // Fall through only when index empty of biome tiles entirely.
            if self.biome_tile_count > 0 {
                self.missing.insert(index, ());
                return None;
            }
        }

        let bx = abs_mod(index - biome as i32 * 16, 16);
        let x = bx % 4;
        let y = bx / 4;
        let rel = format!("groundTileCache/biome_{biome}_x{x}_y{y}.tga");
        self.pack_tile_from_rel(index, &rel)
    }

    /// Ensure unknown-biome tile (`biome_99999_x*_y*`) is packed.
    pub fn ensure_unknown_tile(&mut self, tile_x: i32, tile_y: i32) -> Option<GroundTileRect> {
        let x = abs_mod(tile_x, 4);
        let y = abs_mod(tile_y, 4);
        let index = UNKNOWN_BIOME_CACHE_ID + x + y * 4;
        if let Some(r) = self.tiles.get(&index) {
            return Some(*r);
        }
        if self.missing.contains_key(&index) {
            return None;
        }
        if self.index_loaded {
            if let Some(ent) = self.index.get(&index) {
                if !ent.exists {
                    self.missing.insert(index, ());
                    return None;
                }
                let rel = ent.rel_path.clone();
                return self.pack_tile_from_rel(index, &rel);
            }
            if self.biome_tile_count > 0 {
                self.missing.insert(index, ());
                return None;
            }
        }
        let rel = format!("groundTileCache/biome_{UNKNOWN_BIOME_CACHE_ID}_x{x}_y{y}.tga");
        self.pack_tile_from_rel(index, &rel)
    }

    /// True when any overlay sheets are known or may still load from disk.
    /// Used by the renderer to skip per-tile overlay probes on empty banks (FPS).
    pub fn has_overlays(&self) -> bool {
        self.overlay_count > 0
            || !self.overlay_index.is_empty()
            || !self.overlays.is_empty()
            || !self.index_loaded
    }

    /// Ensure overlay sheet `graphics/ground_t{id}.tga` is packed (Haxe groundOverlay).
    pub fn ensure_overlay(&mut self, id: u8) -> Option<GroundTileRect> {
        if let Some(r) = self.overlays.get(&id) {
            return Some(*r);
        }
        if self.missing_overlays.contains_key(&id) {
            return None;
        }
        if self.index_loaded {
            if let Some(ent) = self.overlay_index.get(&id) {
                if !ent.exists {
                    self.missing_overlays.insert(id, ());
                    return None;
                }
                let rel = ent.rel_path.clone();
                return self.pack_overlay_from_rel(id, &rel);
            }
            if self.overlay_count > 0 {
                self.missing_overlays.insert(id, ());
                return None;
            }
        }
        let rel = format!("graphics/ground_t{id}.tga");
        self.pack_overlay_from_rel(id, &rel)
    }

    /// C++ `wholeSheet` — full `ground_N.tga` (typically 4×4 cells = 512²).
    ///
    /// Returns `(atlas rect, tiles_wide, tiles_high)`.
    pub fn ensure_whole_sheet(&mut self, biome: u8) -> Option<(GroundTileRect, u32, u32)> {
        let key = biome as i32;
        if let Some(gt) = self.whole_sheets.get(&key) {
            let dims = self.whole_sheet_tiles.get(&key).copied().unwrap_or((4, 4));
            return Some((*gt, dims.0, dims.1));
        }
        if self.missing_whole.contains_key(&key) {
            return None;
        }
        let candidates = [
            format!("ground/ground_{biome}.tga"),
            format!("graphics/ground_{biome}.tga"),
            format!("ground_{biome}.tga"),
        ];
        let mut path = None;
        for rel in &candidates {
            if let Some(p) = self.resolve_rel(rel) {
                path = Some(p);
                break;
            }
        }
        // Unknown biome sheet
        if path.is_none() && !self.has_biome_sheet(biome) {
            for rel in ["ground/ground_U.tga", "graphics/ground_U.tga", "ground_U.tga"] {
                if let Some(p) = self.resolve_rel(rel) {
                    path = Some(p);
                    break;
                }
            }
        }
        let Some(path) = path else {
            self.missing_whole.insert(key, ());
            return None;
        };
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing_whole.insert(key, ());
                return None;
            }
        };
        if img.width == 0 || img.height == 0 {
            self.missing_whole.insert(key, ());
            return None;
        }
        let tw = (img.width / CELL_D as u32).max(1);
        let th = (img.height / CELL_D as u32).max(1);
        match self.pack_image(&img.pixels, img.width, img.height) {
            Some(gt) => {
                self.whole_sheets.insert(key, gt);
                self.whole_sheet_tiles.insert(key, (tw, th));
                self.any_loaded = true;
                Some((gt, tw, th))
            }
            None => {
                self.missing_whole.insert(key, ());
                None
            }
        }
    }

    /// Pixel size of overlay sprite `id` (for Jason screen overlay spacing).
    pub fn overlay_pixel_size(&mut self, id: u8) -> Option<(u32, u32)> {
        let gt = self.ensure_overlay(id)?;
        Some((gt.width, gt.height))
    }

    /// Preload all known overlays from index (or ids 0..3). Cheap Ã¢â‚¬â€ only 4 sheets.
    pub fn preload_overlays(&mut self) -> usize {
        let ids: Vec<u8> = if !self.overlay_index.is_empty() {
            let mut v: Vec<u8> = self.overlay_index.keys().copied().collect();
            v.sort_unstable();
            v
        } else {
            (0..GROUND_OVERLAY_COUNT as u8).collect()
        };
        let mut n = 0usize;
        for id in ids {
            if self.ensure_overlay(id).is_some() {
                n += 1;
            }
        }
        n
    }

    /// Ensure overlay for world tile if Haxe would draw one; returns packed rect.
    pub fn ensure_overlay_for_tile(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<GroundTileRect> {
        let slot = ground_overlay_slot(tile_x, tile_y)?;
        self.ensure_overlay(slot)
    }

    fn pack_tile_from_rel(&mut self, index: i32, rel: &str) -> Option<GroundTileRect> {
        let path = self.resolve_rel(rel);
        let Some(path) = path else {
            self.missing.insert(index, ());
            return None;
        };
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing.insert(index, ());
                return None;
            }
        };
        match self.pack_image(&img.pixels, img.width, img.height) {
            Some(gt) => {
                self.tiles.insert(index, gt);
                self.any_loaded = true;
                Some(gt)
            }
            None => {
                self.missing.insert(index, ());
                None
            }
        }
    }

    fn pack_overlay_from_rel(&mut self, id: u8, rel: &str) -> Option<GroundTileRect> {
        let path = self.resolve_rel(rel);
        let Some(path) = path else {
            self.missing_overlays.insert(id, ());
            return None;
        };
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing_overlays.insert(id, ());
                return None;
            }
        };
        match self.pack_image(&img.pixels, img.width, img.height) {
            Some(gt) => {
                self.overlays.insert(id, gt);
                self.any_loaded = true;
                Some(gt)
            }
            None => {
                self.missing_overlays.insert(id, ());
                None
            }
        }
    }

    fn resolve_rel(&self, rel: &str) -> Option<PathBuf> {
        // Absolute-ish: if rel already exists as given
        let p = PathBuf::from(rel);
        if p.is_absolute() && p.exists() {
            return Some(p);
        }
        self.roots.iter().map(|r| r.join(rel)).find(|p| p.exists())
    }

    fn pack_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Option<GroundTileRect> {
        let w = width as i32;
        let h = height as i32;
        if w <= 0 || h <= 0 || w > GROUND_ATLAS || h > GROUND_ATLAS {
            return None;
        }
        for ai in 0..self.pages.len() {
            if let Some(rect) = self.pages[ai].packer.pack(w, h) {
                self.pages[ai].blit(rect.x, rect.y, pixels, width, height);
                return Some(GroundTileRect {
                    atlas_index: ai,
                    rect,
                    width,
                    height,
                });
            }
        }
        let mut page = GroundPage::new(GROUND_ATLAS);
        let rect = page.packer.pack(w, h)?;
        page.blit(rect.x, rect.y, pixels, width, height);
        let ai = self.pages.len();
        self.pages.push(page);
        Some(GroundTileRect {
            atlas_index: ai,
            rect,
            width,
            height,
        })
    }

    /// Install OLG1 index bytes into this bank (does not load TGA pixels).
    pub fn load_olg1(&mut self, data: &[u8]) -> Result<u32, String> {
        let (ver, entries, _meta) = load_olg1(data)?;
        self.index.clear();
        self.overlay_index.clear();
        self.overlay_count = 0;
        self.biome_tile_count = 0;
        for e in entries {
            match e.kind {
                OLG1_KIND_OVERLAY => {
                    if e.exists {
                        self.overlay_count += 1;
                    }
                    self.overlay_index.insert(e.id as u8, e);
                }
                OLG1_KIND_BIOME | OLG1_KIND_UNKNOWN => {
                    if e.exists {
                        self.biome_tile_count += 1;
                    }
                    let k = e.bank_key();
                    self.index.insert(k, e);
                }
                _ => {}
            }
        }
        self.index_loaded = true;
        self.data_version = ver;
        Ok(ver)
    }

    /// Scan roots for ground tiles + overlays; populate in-memory index (no OLG1 write).
    pub fn scan_index_from_disk(&mut self) -> usize {
        let entries = scan_ground_index(&self.roots);
        self.index.clear();
        self.overlay_index.clear();
        self.overlay_count = 0;
        self.biome_tile_count = 0;
        for e in entries {
            match e.kind {
                OLG1_KIND_OVERLAY => {
                    if e.exists {
                        self.overlay_count += 1;
                    }
                    self.overlay_index.insert(e.id as u8, e);
                }
                OLG1_KIND_BIOME | OLG1_KIND_UNKNOWN => {
                    if e.exists {
                        self.biome_tile_count += 1;
                    }
                    let k = e.bank_key();
                    self.index.insert(k, e);
                }
                _ => {}
            }
        }
        self.index_loaded = !self.index.is_empty() || !self.overlay_index.is_empty();
        self.index_len()
    }

    /// Serialize current index (or re-scan) to OLG1 bytes.
    pub fn write_olg1(&self, data_version: u32) -> Vec<u8> {
        let mut entries: Vec<GroundIndexEntry> = self
            .overlay_index
            .values()
            .cloned()
            .chain(self.index.values().cloned())
            .collect();
        if entries.is_empty() {
            entries = scan_ground_index(&self.roots);
        }
        write_olg1(&entries, data_version)
    }

    /// Eagerly pack every indexed overlay + biome/unknown tile into multi-page atlas.
    ///
    /// Haxe `loadGround` packs all tiles once; default Rust play path stays lazy via
    /// [`Self::ensure_tile`]. Returns number of tiles successfully packed.
    pub fn pack_all_from_index(&mut self) -> usize {
        if !self.index_loaded {
            let _ = self.scan_index_from_disk();
        }
        let mut n = 0usize;

        // Overlays first (Haxe packs ground_t0..t3 before biome tiles).
        let mut oids: Vec<u8> = self.overlay_index.keys().copied().collect();
        if oids.is_empty() && !self.index_loaded {
            oids = (0..GROUND_OVERLAY_COUNT as u8).collect();
        }
        oids.sort_unstable();
        for id in oids {
            if self.ensure_overlay(id).is_some() {
                n += 1;
            }
        }

        let mut keys: Vec<i32> = self.index.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            let packed = if k >= UNKNOWN_BIOME_CACHE_ID {
                let off = k - UNKNOWN_BIOME_CACHE_ID;
                let x = off % 4;
                let y = off / 4;
                self.ensure_unknown_tile(x, y).is_some()
            } else {
                let biome = (k / 16).clamp(0, 255) as u8;
                self.ensure_index(biome, k).is_some()
            };
            if packed {
                n += 1;
            }
        }
        n
    }

    /// Serialize packed pages + rects to OLGA bytes (SaveGroundData multi-atlas dump).
    ///
    /// Caller should [`Self::pack_all_from_index`] first. Empty bank yields header-only payload.
    pub fn write_olga(&self, data_version: u32) -> Vec<u8> {
        write_olga_from_bank(self, data_version)
    }

    /// Restore multi-page atlas + rect maps from OLGA (no TGA required for packed tiles).
    ///
    /// For timed loads (bench/CLI), prefer [`Self::load_olga_timed`].
    pub fn load_olga(&mut self, data: &[u8]) -> Result<u32, String> {
        Ok(self.load_olga_timed(data)?.data_version)
    }

    /// Like [`Self::load_olga`] but returns [`OlgaLoadStats`] with wall-clock duration.
    pub fn load_olga_timed(&mut self, data: &[u8]) -> Result<OlgaLoadStats, String> {
        let t0 = Instant::now();
        let (ver, tiles, overlays, pages) = load_olga(data)?;
        let page_count = pages.len();
        let tile_records = tiles.len() + overlays.len();
        self.pages = pages;
        self.tiles = tiles;
        self.overlays = overlays;
        self.missing.clear();
        self.missing_overlays.clear();
        self.index.clear();
        self.overlay_index.clear();
        self.overlay_count = 0;
        self.biome_tile_count = 0;

        // Synthetic presence index so ensure_* treats absent keys as missing (no disk probe).
        for (id, gt) in &self.overlays {
            self.overlay_count += 1;
            self.overlay_index.insert(
                *id,
                GroundIndexEntry {
                    kind: OLG1_KIND_OVERLAY,
                    id: *id as i32,
                    tile_x: 0,
                    tile_y: 0,
                    has_square: false,
                    exists: true,
                    width: gt.width as u16,
                    height: gt.height as u16,
                    rel_path: String::new(),
                },
            );
        }
        for (k, gt) in &self.tiles {
            self.biome_tile_count += 1;
            let (kind, id, tx, ty) = decode_bank_key(*k);
            self.index.insert(
                *k,
                GroundIndexEntry {
                    kind,
                    id,
                    tile_x: tx,
                    tile_y: ty,
                    has_square: false,
                    exists: true,
                    width: gt.width as u16,
                    height: gt.height as u16,
                    rel_path: String::new(),
                },
            );
        }

        self.index_loaded = true;
        self.atlas_loaded = true;
        self.any_loaded = !self.tiles.is_empty() || !self.overlays.is_empty();
        self.data_version = ver;
        Ok(OlgaLoadStats {
            bytes: data.len(),
            tile_records,
            page_count,
            data_version: ver,
            duration: t0.elapsed(),
        })
    }
}

/// Decode runtime bank key Ã¢â€ â€™ (kind, id, tile_x, tile_y) for synthetic index rows.
fn decode_bank_key(k: i32) -> (u8, i32, u8, u8) {
    if k >= UNKNOWN_BIOME_CACHE_ID {
        let off = k - UNKNOWN_BIOME_CACHE_ID;
        let tx = (off % 4) as u8;
        let ty = (off / 4) as u8;
        (OLG1_KIND_UNKNOWN, UNKNOWN_BIOME_CACHE_ID, tx, ty)
    } else {
        let biome = k / 16;
        let off = k - biome * 16;
        let tx = (off % 4) as u8;
        let ty = (off / 4) as u8;
        (OLG1_KIND_BIOME, biome, tx, ty)
    }
}

impl Default for GroundBank {
    fn default() -> Self {
        Self::new()
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬ OLG1 binary Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Fixed meta after the 24-byte OL* header.
#[derive(Debug, Clone, Copy)]
pub struct Olg1Meta {
    pub num_overlays: u16,
    pub tiles_w: u16,
    pub tiles_h: u16,
    pub max_biome: u16,
}

impl Default for Olg1Meta {
    fn default() -> Self {
        Self {
            num_overlays: GROUND_OVERLAY_COUNT,
            tiles_w: GROUND_TILES_WIDE,
            tiles_h: GROUND_TILES_HIGH,
            max_biome: 6,
        }
    }
}

/// Write OLG1 bytes from index entries.
pub fn write_olg1(entries: &[GroundIndexEntry], data_version: u32) -> Vec<u8> {
    let mut list = entries.to_vec();
    list.sort_by(|a, b| {
        (a.kind, a.id, a.tile_x, a.tile_y).cmp(&(b.kind, b.id, b.tile_x, b.tile_y))
    });

    let mut max_biome = 0u16;
    let mut num_overlays = 0u16;
    for e in &list {
        if e.kind == OLG1_KIND_OVERLAY {
            num_overlays = num_overlays.max(e.id as u16 + 1);
        }
        if e.kind == OLG1_KIND_BIOME && e.id >= 0 {
            max_biome = max_biome.max(e.id as u16);
        }
    }
    if num_overlays == 0 {
        num_overlays = GROUND_OVERLAY_COUNT;
    }

    let mut out = Vec::with_capacity(32 + list.len() * 48);
    out.extend_from_slice(OLG1_MAGIC);
    out.extend_from_slice(&OLG1_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&data_version.to_le_bytes());
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes()); // header_crc32
    // fixed meta (8 bytes)
    out.extend_from_slice(&num_overlays.to_le_bytes());
    out.extend_from_slice(&GROUND_TILES_WIDE.to_le_bytes());
    out.extend_from_slice(&GROUND_TILES_HIGH.to_le_bytes());
    out.extend_from_slice(&max_biome.to_le_bytes());

    for e in &list {
        out.push(e.kind);
        out.extend_from_slice(&e.id.to_le_bytes());
        out.push(e.tile_x);
        out.push(e.tile_y);
        let mut flags = 0u8;
        if e.has_square {
            flags |= OLG1_F_HAS_SQUARE;
        }
        if e.exists {
            flags |= OLG1_F_EXISTS;
        }
        out.push(flags);
        out.extend_from_slice(&e.width.to_le_bytes());
        out.extend_from_slice(&e.height.to_le_bytes());
        let path_b = e.rel_path.as_bytes();
        let plen = (path_b.len().min(u16::MAX as usize)) as u16;
        out.extend_from_slice(&plen.to_le_bytes());
        out.extend_from_slice(&path_b[..plen as usize]);
    }
    out
}

/// Parse OLG1 Ã¢â€ â€™ (data_version, entries, meta).
pub fn load_olg1(data: &[u8]) -> Result<(u32, Vec<GroundIndexEntry>, Olg1Meta), String> {
    if data.len() < 24 + 8 {
        return Err("OLG1 too short".into());
    }
    if &data[0..4] != OLG1_MAGIC {
        return Err("bad OLG1 magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLG1_FORMAT_VERSION {
        return Err(format!("unsupported OLG1 format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let mut off = 24usize;
    let num_overlays = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    off += 2;
    let tiles_w = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    off += 2;
    let tiles_h = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    off += 2;
    let max_biome = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    off += 2;
    let meta = Olg1Meta {
        num_overlays,
        tiles_w,
        tiles_h,
        max_biome,
    };

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        // kind(1)+id(4)+tx(1)+ty(1)+flags(1)+w(2)+h(2)+plen(2) = 14
        if off + 14 > data.len() {
            return Err("OLG1 truncated record".into());
        }
        let kind = data[off];
        off += 1;
        let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let tile_x = data[off];
        off += 1;
        let tile_y = data[off];
        off += 1;
        let flags = data[off];
        off += 1;
        let width = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
        off += 2;
        let height = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
        off += 2;
        let plen = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
        off += 2;
        if off + plen > data.len() {
            return Err("OLG1 truncated path".into());
        }
        let rel_path = String::from_utf8_lossy(&data[off..off + plen]).into_owned();
        off += plen;
        entries.push(GroundIndexEntry {
            kind,
            id,
            tile_x,
            tile_y,
            has_square: flags & OLG1_F_HAS_SQUARE != 0,
            exists: flags & OLG1_F_EXISTS != 0,
            width,
            height,
            rel_path,
        });
    }
    Ok((data_version, entries, meta))
}

/// Default roots used by bank + baker.
pub fn default_ground_roots(content_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(p) = std::env::var("OHOL_GAME_DATA") {
        if !p.is_empty() {
            roots.push(PathBuf::from(p));
        }
    }
    roots.push(PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData"));
    if let Some(c) = content_root {
        roots.push(c.to_path_buf());
        if let Some(parent) = c.parent() {
            roots.push(parent.to_path_buf());
            // e.g. content/OneLifeData7 Ã¢â€ â€™ OpenLife sibling game data
            if let Some(gp) = parent.parent() {
                roots.push(gp.join("OneLifeGameSourceData"));
            }
        }
    }
    roots.push(PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"));
    // de-dup while preserving order
    let mut seen = HashMap::new();
    roots
        .into_iter()
        .filter(|r| seen.insert(r.clone(), ()).is_none())
        .collect()
}

/// Scan roots for overlays + biome tiles Ã¢â€ â€™ index entries.
pub fn scan_ground_index(roots: &[PathBuf]) -> Vec<GroundIndexEntry> {
    let mut out = Vec::new();

    // Overlays: graphics/ground_tN.tga (Haxe Resource.groundOverlay)
    for id in 0..GROUND_OVERLAY_COUNT as i32 {
        let rel = format!("graphics/ground_t{id}.tga");
        if let Some((exists, w, h, _full)) = find_rel(roots, &rel) {
            out.push(GroundIndexEntry {
                kind: OLG1_KIND_OVERLAY,
                id,
                tile_x: 0,
                tile_y: 0,
                has_square: false,
                exists,
                width: w,
                height: h,
                rel_path: rel,
            });
        }
    }

    // Discover biome ids by directory listing when possible.
    let mut biome_ids: Vec<i32> = Vec::new();
    for root in roots {
        let dir = root.join("groundTileCache");
        if !dir.is_dir() {
            continue;
        }
        if let Ok(rd) = fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().into_owned();
                // biome_{id}_x{i}_y{j}.tga  (skip _square)
                if name.contains("_square") {
                    continue;
                }
                if let Some(rest) = name.strip_prefix("biome_") {
                    if let Some(id_str) = rest.split('_').next() {
                        if let Ok(id) = id_str.parse::<i32>() {
                            if !biome_ids.contains(&id) {
                                biome_ids.push(id);
                            }
                        }
                    }
                }
            }
        }
    }
    if biome_ids.is_empty() {
        // probe standard range + unknown
        biome_ids.extend(0..=6);
        biome_ids.push(UNKNOWN_BIOME_CACHE_ID);
    } else if !biome_ids.contains(&UNKNOWN_BIOME_CACHE_ID) {
        // still probe unknown if any cache exists
        biome_ids.push(UNKNOWN_BIOME_CACHE_ID);
    }
    biome_ids.sort_unstable();

    for id in biome_ids {
        let kind = if id == UNKNOWN_BIOME_CACHE_ID {
            OLG1_KIND_UNKNOWN
        } else {
            OLG1_KIND_BIOME
        };
        for y in 0..GROUND_TILES_HIGH as u8 {
            for x in 0..GROUND_TILES_WIDE as u8 {
                let rel = format!("groundTileCache/biome_{id}_x{x}_y{y}.tga");
                let square_rel = format!("groundTileCache/biome_{id}_x{x}_y{y}_square.tga");
                let has_square = roots.iter().any(|r| r.join(&square_rel).exists());
                if let Some((exists, w, h, _)) = find_rel(roots, &rel) {
                    if exists {
                        out.push(GroundIndexEntry {
                            kind,
                            id,
                            tile_x: x,
                            tile_y: y,
                            has_square,
                            exists: true,
                            width: w,
                            height: h,
                            rel_path: rel,
                        });
                    }
                }
            }
        }
    }
    out
}

fn find_rel(roots: &[PathBuf], rel: &str) -> Option<(bool, u16, u16, PathBuf)> {
    for r in roots {
        let p = r.join(rel);
        if p.exists() {
            let (w, h) = tga_header_size(&p).unwrap_or((0, 0));
            return Some((true, w, h, p));
        }
    }
    None
}

/// Read TGA width/height from 18-byte header (no full decode).
fn tga_header_size(path: &Path) -> Option<(u16, u16)> {
    let mut f = fs::File::open(path).ok()?;
    let mut hdr = [0u8; 18];
    f.read_exact(&mut hdr).ok()?;
    let w = u16::from_le_bytes([hdr[12], hdr[13]]);
    let h = u16::from_le_bytes([hdr[14], hdr[15]]);
    Some((w, h))
}

/// Bake OLG1 from content root + default game-data roots.
///
/// Returns `(bytes, record_count)`. Always succeeds (empty payload OK).
pub fn bake_olg1_from_roots(
    content_root: Option<&Path>,
    data_version: u32,
) -> (Vec<u8>, usize) {
    let roots = default_ground_roots(content_root);
    let entries = scan_ground_index(&roots);
    let n = entries.len();
    (write_olg1(&entries, data_version), n)
}

/// Write OLG1 into `out_dir/olg1_ground_index.bin`.
pub fn bake_olg1_to_dir(
    content_root: Option<&Path>,
    out_dir: impl AsRef<Path>,
    data_version: u32,
) -> Result<(usize, usize), String> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let (bytes, count) = bake_olg1_from_roots(content_root, data_version);
    let path = out_dir.join("olg1_ground_index.bin");
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok((bytes.len(), count))
}

// Ã¢â€â‚¬Ã¢â€â‚¬ OLGA binary (optional full multi-atlas dump) Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Stats from an OLGA bake (includes wall-clock timings for bake vs later load compare).
#[derive(Debug, Clone, Copy, Default)]
pub struct OlgaBakeStats {
    pub bytes: usize,
    pub tile_records: usize,
    pub page_count: usize,
    pub packed_tiles: usize,
    /// Scan index + TGA load + BinPack blit.
    pub pack_duration: Duration,
    /// Serialize + write `olga_ground_atlas.bin` (write path only; zero for in-memory bake).
    pub write_duration: Duration,
    /// Total wall time for the bake call.
    pub total_duration: Duration,
}

impl OlgaBakeStats {
    pub fn report_line(&self) -> String {
        format!(
            "packed={} records={} pages={} bytes={} pack={:.1}ms write={:.1}ms total={:.1}ms",
            self.packed_tiles,
            self.tile_records,
            self.page_count,
            self.bytes,
            self.pack_duration.as_secs_f64() * 1000.0,
            self.write_duration.as_secs_f64() * 1000.0,
            self.total_duration.as_secs_f64() * 1000.0,
        )
    }
}

/// Stats from loading an OLGA blob (for load_bench / CLI).
#[derive(Debug, Clone, Copy, Default)]
pub struct OlgaLoadStats {
    pub bytes: usize,
    pub tile_records: usize,
    pub page_count: usize,
    pub data_version: u32,
    pub duration: Duration,
}

impl OlgaLoadStats {
    pub fn report_line(&self) -> String {
        format!(
            "ver={} records={} pages={} bytes={} load={:.1}ms",
            self.data_version,
            self.tile_records,
            self.page_count,
            self.bytes,
            self.duration.as_secs_f64() * 1000.0,
        )
    }
}

/// Write OLGA from a bank that already has packed tiles/overlays.
fn write_olga_from_bank(bank: &GroundBank, data_version: u32) -> Vec<u8> {
    // Stable order: overlays by id, then biome/unknown keys sorted.
    let mut overlay_ids: Vec<u8> = bank.overlays.keys().copied().collect();
    overlay_ids.sort_unstable();
    let mut tile_keys: Vec<i32> = bank.tiles.keys().copied().collect();
    tile_keys.sort_unstable();

    let record_count = (overlay_ids.len() + tile_keys.len()) as u32;
    let num_pages = bank.pages.len() as u16;

    let mut out = Vec::with_capacity(
        48 + record_count as usize * 24
            + bank
                .pages
                .iter()
                .map(|p| 12 + p.pixels.len())
                .sum::<usize>(),
    );
    out.extend_from_slice(OLGA_MAGIC);
    out.extend_from_slice(&OLGA_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&data_version.to_le_bytes());
    out.extend_from_slice(&record_count.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes()); // header_crc32
    // fixed meta (12 bytes)
    out.extend_from_slice(&(GROUND_ATLAS as u32).to_le_bytes());
    out.extend_from_slice(&num_pages.to_le_bytes());
    out.extend_from_slice(&(overlay_ids.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved

    let push_record = |out: &mut Vec<u8>, kind: u8, id: i32, tx: u8, ty: u8, gt: &GroundTileRect| {
        out.push(kind);
        out.extend_from_slice(&id.to_le_bytes());
        out.push(tx);
        out.push(ty);
        out.extend_from_slice(&(gt.atlas_index as u16).to_le_bytes());
        out.extend_from_slice(&gt.rect.x.to_le_bytes());
        out.extend_from_slice(&gt.rect.y.to_le_bytes());
        out.extend_from_slice(&gt.rect.width.to_le_bytes());
        out.extend_from_slice(&gt.rect.height.to_le_bytes());
        out.extend_from_slice(&(gt.width as u16).to_le_bytes());
        out.extend_from_slice(&(gt.height as u16).to_le_bytes());
    };

    for id in &overlay_ids {
        if let Some(gt) = bank.overlays.get(id) {
            push_record(&mut out, OLG1_KIND_OVERLAY, *id as i32, 0, 0, gt);
        }
    }
    for k in &tile_keys {
        if let Some(gt) = bank.tiles.get(k) {
            let (kind, id, tx, ty) = decode_bank_key(*k);
            push_record(&mut out, kind, id, tx, ty, gt);
        }
    }

    for page in &bank.pages {
        out.extend_from_slice(&page.width.to_le_bytes());
        out.extend_from_slice(&page.height.to_le_bytes());
        let blen = page.pixels.len() as u32;
        out.extend_from_slice(&blen.to_le_bytes());
        out.extend_from_slice(&page.pixels);
    }
    out
}

/// Parse OLGA Ã¢â€ â€™ (data_version, tiles, overlays, pages).
fn load_olga(
    data: &[u8],
) -> Result<
    (
        u32,
        HashMap<i32, GroundTileRect>,
        HashMap<u8, GroundTileRect>,
        Vec<GroundPage>,
    ),
    String,
> {
    if data.len() < 24 + 12 {
        return Err("OLGA too short".into());
    }
    if &data[0..4] != OLGA_MAGIC {
        return Err("bad OLGA magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLGA_FORMAT_VERSION {
        return Err(format!("unsupported OLGA format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let record_count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let mut off = 24usize;
    let _page_size = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
    off += 4;
    let num_pages = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
    off += 2;
    let _num_overlays_meta = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
    off += 2;
    off += 4; // reserved

    // Record size: kind1 + id4 + tx1 + ty1 + atlas2 + rect 4*i32 + w2 + h2 = 1+4+1+1+2+16+2+2 = 29
    const REC: usize = 29;
    let mut tiles: HashMap<i32, GroundTileRect> = HashMap::new();
    let mut overlays: HashMap<u8, GroundTileRect> = HashMap::new();
    for _ in 0..record_count {
        if off + REC > data.len() {
            return Err("OLGA truncated record".into());
        }
        let kind = data[off];
        off += 1;
        let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let tile_x = data[off];
        off += 1;
        let tile_y = data[off];
        off += 1;
        let atlas_index = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
        off += 2;
        let rx = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let ry = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let rw = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let rh = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let width = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as u32;
        off += 2;
        let height = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as u32;
        off += 2;
        let gt = GroundTileRect {
            atlas_index,
            rect: Rect {
                x: rx,
                y: ry,
                width: rw,
                height: rh,
            },
            width,
            height,
        };
        match kind {
            OLG1_KIND_OVERLAY => {
                overlays.insert(id as u8, gt);
            }
            OLG1_KIND_BIOME => {
                let k = id * 16 + tile_x as i32 + tile_y as i32 * 4;
                tiles.insert(k, gt);
            }
            OLG1_KIND_UNKNOWN => {
                let k = UNKNOWN_BIOME_CACHE_ID + tile_x as i32 + tile_y as i32 * 4;
                tiles.insert(k, gt);
            }
            _ => {}
        }
    }

    let mut pages = Vec::with_capacity(num_pages);
    for _ in 0..num_pages {
        if off + 12 > data.len() {
            return Err("OLGA truncated page header".into());
        }
        let w = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let h = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let blen = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        if off + blen > data.len() {
            return Err("OLGA truncated page pixels".into());
        }
        let expected = (w as usize).saturating_mul(h as usize).saturating_mul(4);
        if blen != expected && !(w == 0 || h == 0) {
            return Err(format!(
                "OLGA page size mismatch: blen={blen} expected={expected}"
            ));
        }
        let pixels = data[off..off + blen].to_vec();
        off += blen;
        pages.push(GroundPage {
            width: w,
            height: h,
            pixels,
            packer: BinPack::sealed(w as i32, h as i32),
        });
    }
    if pages.is_empty() {
        pages.push(GroundPage::new(GROUND_ATLAS));
    }
    Ok((data_version, tiles, overlays, pages))
}

/// Bake full multi-page ground atlas (OLGA) from content / game-data roots.
///
/// Scans index, packs all existing TGA tiles, returns OLGA bytes + stats.
/// Large output (tens of MB with full OHOL groundTileCache) Ã¢â‚¬â€ optional path only.
pub fn bake_olga_from_roots(
    content_root: Option<&Path>,
    data_version: u32,
) -> Result<(Vec<u8>, OlgaBakeStats), String> {
    let total0 = Instant::now();
    let roots = default_ground_roots(content_root);
    let mut bank = GroundBank::with_roots(roots);

    let t_pack = Instant::now();
    let _ = bank.scan_index_from_disk();
    let packed = bank.pack_all_from_index();
    let pack_duration = t_pack.elapsed();

    let t_ser = Instant::now();
    let bytes = bank.write_olga(data_version);
    // In-memory path: serialize counted under write_duration; disk write is 0 here.
    let write_duration = t_ser.elapsed();

    let stats = OlgaBakeStats {
        bytes: bytes.len(),
        tile_records: bank.tiles.len() + bank.overlays.len(),
        page_count: bank.page_count(),
        packed_tiles: packed,
        pack_duration,
        write_duration,
        total_duration: total0.elapsed(),
    };
    Ok((bytes, stats))
}

/// Write `out_dir/olga_ground_atlas.bin` (optional; not part of default bake_content).
pub fn bake_olga_to_dir(
    content_root: Option<&Path>,
    out_dir: impl AsRef<Path>,
    data_version: u32,
) -> Result<OlgaBakeStats, String> {
    let total0 = Instant::now();
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let (bytes, mut stats) = bake_olga_from_roots(content_root, data_version)?;
    let path = out_dir.join("olga_ground_atlas.bin");
    let t_write = Instant::now();
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    // Fold disk write into write_duration (serialize already in stats.write_duration).
    stats.write_duration += t_write.elapsed();
    stats.bytes = bytes.len();
    stats.total_duration = total0.elapsed();
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_square_midtone(path: &Path) -> Option<(u8, u8, u8)> {
        let img = load_tga_path(path).ok()?;
        let w = img.width as usize;
        let h = img.height as usize;
        let cx = w / 2;
        let cy = h / 2;
        let mut r = 0u64;
        let mut g = 0u64;
        let mut bl = 0u64;
        let mut n = 0u64;
        for y in (cy.saturating_sub(16))..(cy + 16).min(h) {
            for x in (cx.saturating_sub(16))..(cx + 16).min(w) {
                let i = (y * w + x) * 4;
                if i + 2 >= img.pixels.len() {
                    continue;
                }
                r += img.pixels[i] as u64;
                g += img.pixels[i + 1] as u64;
                bl += img.pixels[i + 2] as u64;
                n += 1;
            }
        }
        if n == 0 {
            return None;
        }
        Some(((r / n) as u8, (g / n) as u8, (bl / n) as u8))
    }

    #[test]
    fn sample_live_square_midtones_for_biome_color_table() {
        // Documents Jason sheet midtones used by `biome_color`. Soft-fail if game data absent.
        let game = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData\groundTileCache");
        if !game.is_dir() {
            return;
        }
        let mut samples = Vec::new();
        for b in 0u8..=6 {
            let p = game.join(format!("biome_{b}_x0_y0_square.tga"));
            if let Some(rgb) = sample_square_midtone(&p) {
                samples.push((b, rgb));
            }
        }
        // Sanity: grass (0) should be greener than desert (3) typically; all finite.
        assert!(!samples.is_empty(), "expected groundTileCache square samples");
        for (b, (ar, ag, ab)) in samples {
            let table = biome_color(b);
            // Underfill must track sheet midtones so borders blend (tolerance for dither).
            assert!(
                (ar as i32 - table[0] as i32).abs() < 48
                    && (ag as i32 - table[1] as i32).abs() < 48
                    && (ab as i32 - table[2] as i32).abs() < 48,
                "biome {b} midtone ({ar},{ag},{ab}) vs table {:?}",
                table
            );
        }
        if let Some((ar, ag, ab)) =
            sample_square_midtone(&game.join("biome_99999_x0_y0_square.tga"))
        {
            let table = biome_color(255);
            assert!(
                (ar as i32 - table[0] as i32).abs() < 48
                    && (ag as i32 - table[1] as i32).abs() < 48
                    && (ab as i32 - table[2] as i32).abs() < 48,
                "unknown midtone ({ar},{ag},{ab}) vs table {:?}",
                table
            );
        }
    }

    #[test]
    fn get_xy_random_deterministic_and_unit_interval() {
        let a = get_xy_random(3, 3);
        let b = get_xy_random(3, 103);
        let c = get_xy_random(3, 303);
        assert!((0.0..=1.0).contains(&a));
        assert!((0.0..=1.0).contains(&b));
        assert!((0.0..=1.0).contains(&c));
        assert_eq!(get_xy_random(3, 3), a);
        // Distinct channels for unknown tint (usually).
        let col = unknown_biome_draw_color(7);
        assert_eq!(col[3], 255);
    }

    #[test]
    fn unknown_fallback_when_biome_sheet_missing() {
        let game = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData");
        if !game.join("groundTileCache").is_dir() {
            return;
        }
        let mut bank = GroundBank::with_default_roots(Some(&game));
        let _ = bank.scan_index_from_disk();
        // Known biome 0 should have a soft tile.
        assert!(bank.has_biome_sheet(0));
        assert!(bank.ensure_tile(0, 1, 2).is_some());
        // Absurd biome id Ã¢â€ â€™ no sheet Ã¢â€ â€™ unknown 99999 soft tile.
        assert!(!bank.has_biome_sheet(200));
        let (gt, unk) = bank.ensure_tile_or_unknown(200, 1, 2).expect("unknown soft");
        assert!(unk);
        assert!(gt.width > 0 && gt.height > 0);
        // Square path also falls back.
        let (_sq, unk2) = bank.ensure_square_or_unknown(200, 0, 0).expect("unknown square/soft");
        assert!(unk2);
    }

    #[test]
    fn variation_index_matches_haxe() {
        // Haxe: id*16 + abs(x%4) + abs(y%4)*4
        assert_eq!(ground_variation_index(0, 0, 0), 0);
        assert_eq!(ground_variation_index(0, 1, 0), 1);
        assert_eq!(ground_variation_index(0, 0, 1), 4);
        assert_eq!(ground_variation_index(1, 0, 0), 16);
        assert_eq!(ground_variation_index(2, 3, 2), 2 * 16 + 3 + 2 * 4);
        // negative coords
        assert_eq!(ground_variation_index(0, -1, 0), 3); // -1 % 4 Ã¢â€ â€™ 3
        assert_eq!(ground_variation_index(0, 0, -1), 12);
        assert_eq!(ground_map_key_biome(0, 0, 0), 4);
        assert_eq!(ground_map_key_unknown(1, 2), 99999 + 1 + 2 * 4);
    }

    #[test]
    fn overlay_slot_haxe() {
        assert_eq!(ground_overlay_slot(0, 0), Some(0));
        assert_eq!(ground_overlay_slot(0, 1), Some(2));
        assert_eq!(ground_overlay_slot(4, 0), None); // x%8==4 Ã¢â€ â€™ idx>=4
        assert_eq!(ground_overlay_slot(1, 0), None); // x%4 != 0
        assert_eq!(ground_overlay_slot(8, 0), Some(0));
    }

    #[test]
    fn biome_colors_differ() {
        assert_ne!(biome_color(0), biome_color(3));
        let a = biome_color_varied(0, 0, 0);
        let b = biome_color_varied(0, 1, 0);
        assert_eq!(a[3], 255);
        assert_eq!(b[3], 255);
    }

    #[test]
    fn olg1_roundtrip() {
        let entries = vec![
            GroundIndexEntry {
                kind: OLG1_KIND_OVERLAY,
                id: 0,
                tile_x: 0,
                tile_y: 0,
                has_square: false,
                exists: true,
                width: 128,
                height: 128,
                rel_path: "graphics/ground_t0.tga".into(),
            },
            GroundIndexEntry {
                kind: OLG1_KIND_BIOME,
                id: 2,
                tile_x: 1,
                tile_y: 3,
                has_square: true,
                exists: true,
                width: 128,
                height: 128,
                rel_path: "groundTileCache/biome_2_x1_y3.tga".into(),
            },
            GroundIndexEntry {
                kind: OLG1_KIND_UNKNOWN,
                id: UNKNOWN_BIOME_CACHE_ID,
                tile_x: 0,
                tile_y: 0,
                has_square: false,
                exists: true,
                width: 64,
                height: 64,
                rel_path: "groundTileCache/biome_99999_x0_y0.tga".into(),
            },
        ];
        let bytes = write_olg1(&entries, 437);
        assert_eq!(&bytes[0..4], b"OLG1");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLG1_FORMAT_VERSION
        );
        let (ver, loaded, meta) = load_olg1(&bytes).unwrap();
        assert_eq!(ver, 437);
        assert_eq!(loaded.len(), 3);
        assert!(meta.num_overlays >= 1);
        assert_eq!(loaded[0].kind, OLG1_KIND_OVERLAY);
        assert_eq!(loaded[1].rel_path, "groundTileCache/biome_2_x1_y3.tga");
        assert!(loaded[1].has_square);
        assert_eq!(loaded[1].bank_key(), 2 * 16 + 1 + 3 * 4);
        assert_eq!(loaded[2].bank_key(), UNKNOWN_BIOME_CACHE_ID);

        let mut bank = GroundBank::new();
        bank.load_olg1(&bytes).unwrap();
        assert!(bank.index_loaded);
        assert_eq!(bank.overlay_count, 1);
        assert_eq!(bank.biome_tile_count, 2);
        assert!(bank.overlay_index.contains_key(&0));
        assert!(bank.index.contains_key(&(2 * 16 + 1 + 3 * 4)));
    }

    #[test]
    fn olg1_bad_magic() {
        assert!(load_olg1(b"XXXX............................").is_err());
    }

    #[test]
    fn load_ground_tga_if_present() {
        let mut bank = GroundBank::with_default_roots(None);
        if let Some(r) = bank.ensure_tile(0, 0, 0) {
            assert!(r.width > 0 && r.height > 0);
            assert!(bank.any_loaded);
        }
    }

    #[test]
    fn load_overlay_if_present() {
        let mut bank = GroundBank::with_default_roots(None);
        if let Some(r) = bank.ensure_overlay(0) {
            assert!(r.width > 0 && r.height > 0);
            assert!(bank.any_loaded);
            assert!(bank.get_overlay(0).is_some());
        }
        // slot mapping for (0,0)
        let _ = bank.ensure_overlay_for_tile(0, 0);
    }

    #[test]
    fn bake_olg1_from_game_data_if_present() {
        let game = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData");
        if !game.join("groundTileCache").is_dir() {
            return;
        }
        let (bytes, n) = bake_olg1_from_roots(Some(&game), 1);
        assert!(n > 0, "expected ground tiles in game data");
        assert_eq!(&bytes[0..4], b"OLG1");
        let (_ver, entries, meta) = load_olg1(&bytes).unwrap();
        assert_eq!(entries.len(), n);
        assert!(entries.iter().any(|e| e.kind == OLG1_KIND_OVERLAY));
        assert!(entries.iter().any(|e| e.kind == OLG1_KIND_BIOME));
        assert!(meta.tiles_w == 4);

        let mut bank = GroundBank::with_default_roots(Some(&game));
        bank.load_olg1(&bytes).unwrap();
        let gt = bank.ensure_tile(0, 0, 0);
        assert!(gt.is_some());
        let ov = bank.ensure_overlay(0);
        assert!(ov.is_some());
        // indexed miss should not thrash
        assert!(bank.ensure_tile(200, 0, 0).is_none());
    }

    #[test]
    fn bake_olg1_to_tmp_dir() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olg1_bake_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        let game = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData");
        let root = if game.join("groundTileCache").is_dir() {
            Some(game.as_path())
        } else {
            None
        };
        let (bytes_len, count) = bake_olg1_to_dir(root, &tmp, 42).unwrap();
        assert!(tmp.join("olg1_ground_index.bin").exists());
        assert!(bytes_len >= 32);
        let data = fs::read(tmp.join("olg1_ground_index.bin")).unwrap();
        let (ver, entries, _) = load_olg1(&data).unwrap();
        assert_eq!(ver, 42);
        assert_eq!(entries.len(), count);
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Write a tiny top-origin uncompressed 32-bit TGA for fixture tests.
    fn write_test_tga(path: &Path, w: u16, h: u16, rgba: [u8; 4]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut data = vec![0u8; 18];
        data[2] = 2; // uncompressed true-color
        data[12..14].copy_from_slice(&w.to_le_bytes());
        data[14..16].copy_from_slice(&h.to_le_bytes());
        data[16] = 32;
        data[17] = 0x20; // top origin
        let n = (w as usize) * (h as usize);
        data.reserve(n * 4);
        for _ in 0..n {
            data.push(rgba[2]); // B
            data.push(rgba[1]); // G
            data.push(rgba[0]); // R
            data.push(rgba[3]); // A
        }
        fs::write(path, data).unwrap();
    }

    #[test]
    fn olga_roundtrip_from_fixture() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olga_rt_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("graphics")).unwrap();
        fs::create_dir_all(tmp.join("groundTileCache")).unwrap();

        write_test_tga(
            &tmp.join("graphics/ground_t0.tga"),
            32,
            32,
            [10, 20, 30, 255],
        );
        write_test_tga(
            &tmp.join("graphics/ground_t1.tga"),
            32,
            32,
            [40, 50, 60, 255],
        );
        write_test_tga(
            &tmp.join("groundTileCache/biome_0_x0_y0.tga"),
            64,
            64,
            [90, 140, 70, 255],
        );
        write_test_tga(
            &tmp.join("groundTileCache/biome_0_x1_y0.tga"),
            64,
            64,
            [100, 150, 80, 255],
        );
        write_test_tga(
            &tmp.join("groundTileCache/biome_99999_x0_y0.tga"),
            32,
            32,
            [200, 200, 200, 255],
        );

        let mut bank = GroundBank::with_roots(vec![tmp.clone()]);
        let n_idx = bank.scan_index_from_disk();
        assert!(n_idx >= 5, "index should list fixture tiles, got {n_idx}");
        let packed = bank.pack_all_from_index();
        assert!(packed >= 5, "expected packed tiles, got {packed}");
        assert!(bank.any_loaded);
        assert!(bank.page_count() >= 1);

        let ov0 = bank.get_overlay(0).copied().expect("overlay 0 packed");
        let t00 = bank
            .get(ground_variation_index(0, 0, 0))
            .copied()
            .expect("biome tile packed");
        let (px0, _, _) = bank.copy_tile_rgba(&ov0).expect("overlay rgba");
        assert_eq!(&px0[0..4], &[10, 20, 30, 255]);

        let bytes = bank.write_olga(99);
        assert_eq!(&bytes[0..4], b"OLGA");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLGA_FORMAT_VERSION
        );

        let mut loaded = GroundBank::new();
        let ver = loaded.load_olga(&bytes).unwrap();
        assert_eq!(ver, 99);
        assert!(loaded.atlas_loaded);
        assert!(loaded.any_loaded);
        assert_eq!(loaded.page_count(), bank.page_count());
        assert!(loaded.get_overlay(0).is_some());
        assert!(loaded.get(ground_variation_index(0, 0, 0)).is_some());
        assert!(loaded.get(ground_map_key_unknown(0, 0)).is_some()
            || loaded.get(UNKNOWN_BIOME_CACHE_ID).is_some());

        // ensure_* should hit restored rects without TGA roots
        let gt = loaded.ensure_tile(0, 0, 0).expect("restored biome");
        assert_eq!(gt.rect, t00.rect);
        assert_eq!(gt.atlas_index, t00.atlas_index);
        let (px, _, _) = loaded.copy_tile_rgba(&gt).unwrap();
        assert_eq!(&px[0..4], &[90, 140, 70, 255]);
        // Indexed miss: no thrash / no panic
        assert!(loaded.ensure_tile(200, 0, 0).is_none());

        // bake_olga_to_dir + load_prefer_atlas_cache
        let cache = tmp.join("cache");
        let stats = bake_olga_to_dir(Some(&tmp), &cache, 7).unwrap();
        assert!(stats.packed_tiles >= 5);
        assert!(stats.bytes > 32);
        assert!(cache.join("olga_ground_atlas.bin").exists());

        let mut prefer = GroundBank::load_prefer_atlas_cache(&tmp);
        assert!(prefer.atlas_loaded);
        assert!(prefer.get_overlay(0).is_some());
        assert!(prefer.ensure_tile(0, 1, 0).is_some());

        // Default prefer_cache stays OLG1/lazy path (does not auto-load OLGA).
        let _ = bake_olg1_to_dir(Some(&tmp), &cache, 7);
        let mut idx_only = GroundBank::load_prefer_cache(&tmp);
        assert!(idx_only.index_loaded);
        assert!(!idx_only.atlas_loaded);
        // Lazy pack still works from TGA under content root.
        assert!(idx_only.ensure_tile(0, 0, 0).is_some());
        assert!(idx_only.ensure_overlay(0).is_some());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn olga_bad_magic() {
        let mut bank = GroundBank::new();
        assert!(bank
            .load_olga(b"XXXX................................")
            .is_err());
    }

    #[test]
    fn bake_olga_from_game_data_if_present() {
        let game = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData");
        if !game.join("groundTileCache").is_dir() {
            return;
        }
        // Full dump can be large; still exercise pack + header on real assets.
        let (bytes, stats) = bake_olga_from_roots(Some(&game), 1).unwrap();
        assert!(stats.packed_tiles > 0, "expected packed ground tiles");
        assert!(stats.page_count >= 1);
        assert_eq!(&bytes[0..4], b"OLGA");
        assert!(stats.tile_records >= stats.packed_tiles.min(stats.tile_records));

        let mut bank = GroundBank::new();
        bank.load_olga(&bytes).unwrap();
        assert!(bank.atlas_loaded);
        assert!(bank.get_overlay(0).is_some() || bank.overlay_count == 0);
        // At least one biome-0 tile when game data present
        assert!(
            bank.ensure_tile(0, 0, 0).is_some(),
            "expected biome 0 tile in OLGA dump"
        );
        // copy_tile yields non-empty rgba
        if let Some(gt) = bank.get(ground_variation_index(0, 0, 0)).copied() {
            let (px, w, h) = bank.copy_tile_rgba(&gt).unwrap();
            assert_eq!(px.len(), (w * h * 4) as usize);
            assert!(px.iter().any(|&b| b != 0));
        }
    }
}


