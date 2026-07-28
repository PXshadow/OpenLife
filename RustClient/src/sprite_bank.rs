//! Sprite bank + atlas (C++ `spriteBank` meta + Haxe `Resource.spriteImage` / `BinPack` / `Render`).
//!
//! - Loads `sprites/{id}.tga` on demand into 4096² atlas pages (Haxe `MAX_TEXTURE`).
//! - Parses `sprites/{id}.txt` meta: `tag mult [anchorX anchorY] [author=…]`
//!   (C++ `spriteBank.cpp` tokenize; Haxe `ObjectData.getSpriteData` tokens 0/2/3).
//! - Computes alpha-bbox (`centerXOffset`/`visibleW`/`maxD`) + click `hitMap` (C++ `loadSpriteFromRawTGAData`).
//! - Headless-safe: pack from raw RGBA without GPU; OLS1 meta blob (format 1 + 2).
//! - **Bake:** `bake_content` / [`bake_ols1_from_dir`] → `cache/ols1_sprites.bin` (txt + TGA header w/h;
//!   no pixel pages / hitMaps). Boot: [`SpriteBank::load_prefer_cache`].
//! - **OLSA (P4#40, optional):** full multi-page sprite atlas dump (`olsa_sprite_atlas.bin`).
//!   Bake via [`bake_olsa_to_dir`] / CLI `--bake-sprite-atlas`; load via [`SpriteBank::load_olsa`] /
//!   [`SpriteBank::load_prefer_atlas_cache`]. Not written by default `bake_content` (large).
//!   Timings on [`OlsaBakeStats`] / [`OlsaLoadStats`].
//!
//! Residual: alpha-bbox at meta bake without full TGA; SHA1 manifest gate on prefer_cache.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::binpack::{BinPack, Rect};
use crate::tga::{load_tga_path, RgbaImage};

/// Haxe `Render.hx` `MAX_TEXTURE`.
pub const ATLAS_SIZE: i32 = 4096;

/// OLS1 magic (sprite metadata cache; pixels stay on-demand TGA for now).
pub const OLS1_MAGIC: &[u8; 4] = b"OLS1";
/// Format 1: id,w,h,ax,ay,flags,tag (no author / bbox).
pub const OLS1_FORMAT_VERSION_V1: u32 = 1;
/// Format 2: v1 fields + center offsets, visible size, maxD, author.
pub const OLS1_FORMAT_VERSION: u32 = 2;

/// OLSA magic — optional full multi-page sprite atlas (rects + RGBA pages).
pub const OLSA_MAGIC: &[u8; 4] = b"OLSA";
/// Format 1: header + packed rect records + raw RGBA pages (no hitMaps; rebuilt on load).
pub const OLSA_FORMAT_VERSION: u32 = 1;

/// C++ alpha threshold for hitMap / visible bbox (`bytes[b] < 64` ⇒ transparent).
const HIT_ALPHA_THRESHOLD: u8 = 64;

/// Metadata from `sprites/{id}.txt` plus optional alpha-bbox (C++ `SpriteRecord` subset).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpriteMeta {
    pub id: i32,
    pub tag: String,
    /// C++ `multiplicativeBlend` when second token is `1`.
    pub multiplicative_blend: bool,
    /// C++ `centerAnchorXOffset` / Haxe `inCenterXOffset`.
    pub center_anchor_x: i32,
    /// C++ `centerAnchorYOffset` / Haxe `inCenterYOffset`.
    pub center_anchor_y: i32,
    /// True when tag contains `NoFlip`.
    pub no_flip: bool,
    /// C++ `authorTag` from `author=Name` token.
    pub author_tag: Option<String>,
    /// C++ `centerXOffset` — center of non-transparent area relative to w/2,h/2.
    pub center_x_offset: i32,
    /// C++ `centerYOffset`.
    pub center_y_offset: i32,
    /// C++ `visibleW` — size of a≥0.25 region (maxX−minX).
    pub visible_w: u32,
    /// C++ `visibleH`.
    pub visible_h: u32,
    /// C++ `maxD` — max(w,h).
    pub max_d: u32,
    /// Full image width once packed/measured (0 if meta-only).
    pub width: u32,
    /// Full image height once packed/measured.
    pub height: u32,
}

/// UV rect + draw meta in atlas pixels.
#[derive(Debug, Clone, Copy)]
pub struct SpriteRect {
    pub atlas_index: usize,
    pub rect: Rect,
    pub width: u32,
    pub height: u32,
    pub center_anchor_x: i32,
    pub center_anchor_y: i32,
    pub multiplicative_blend: bool,
    pub no_flip: bool,
    /// C++ `centerXOffset` (alpha-bbox).
    pub center_x_offset: i32,
    /// C++ `centerYOffset`.
    pub center_y_offset: i32,
    pub visible_w: u32,
    pub visible_h: u32,
    pub max_d: u32,
}

/// Alpha-derived geometry + optional hit map (C++ after TGA load).
#[derive(Debug, Clone)]
pub struct SpriteAlphaInfo {
    pub center_x_offset: i32,
    pub center_y_offset: i32,
    pub visible_w: u32,
    pub visible_h: u32,
    pub max_d: u32,
    pub width: u32,
    pub height: u32,
    /// C++ `hitMap` after 3× `expandMap` — 1 = clickable, length w*h.
    pub hit_map: Vec<u8>,
}

/// One atlas page RGBA (CPU side; GPU upload is optional).
#[derive(Debug, Clone)]
pub struct AtlasPage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    packer: BinPack,
}

impl AtlasPage {
    fn new(size: i32) -> Self {
        let s = size as u32;
        Self {
            width: s,
            height: s,
            pixels: vec![0u8; (s as usize) * (s as usize) * 4],
            packer: BinPack::new(size, size),
        }
    }

    /// Haxe `Pixels.blit` replace semantics (write every source pixel including a=0
    /// only when source alpha > 0, so transparent source does not clear atlas).
    /// Packed rects do not overlap, so replace is safe and matches Haxe.
    // Haxe: batchPixels[i].blit(rect.x, rect.y, spritePixels, 0, 0, w, h)
    fn blit(&mut self, x: i32, y: i32, img: &RgbaImage) {
        for py in 0..img.height {
            for px in 0..img.width {
                let [r, g, b, a] = img.pixel(px, py);
                let dx = (x as u32).saturating_add(px) as usize;
                let dy = (y as u32).saturating_add(py) as usize;
                if dx >= self.width as usize || dy >= self.height as usize {
                    continue;
                }
                let i = (dy * self.width as usize + dx) * 4;
                // Replace destination with source (Haxe blit). Skip fully transparent
                // source so unused atlas padding stays zero and overlapping edge
                // samples stay clean if packer ever abuts tightly.
                if a == 0 {
                    continue;
                }
                self.pixels[i] = r;
                self.pixels[i + 1] = g;
                self.pixels[i + 2] = b;
                self.pixels[i + 3] = a;
            }
        }
    }
}

/// Sprite bank with multi-page atlases (Haxe `packers` / `batches` / `spriteMap`).
pub struct SpriteBank {
    root: PathBuf,
    atlas_size: i32,
    pages: Vec<AtlasPage>,
    rects: HashMap<i32, SpriteRect>,
    meta: HashMap<i32, SpriteMeta>,
    /// C++ `SpriteRecord.hitMap` per id (post expandMap ×3).
    hit_maps: HashMap<i32, Vec<u8>>,
    /// Failed ids (missing TGA or oversized).
    missing: HashMap<i32, ()>,
    /// OLS1 / tree data_version when loaded from cache.
    pub data_version: u32,
    /// True when OLS1 or disk scan populated meta (not merely lazy ensure_meta).
    pub index_loaded: bool,
    /// True when pages/rects came from OLSA dump (pages sealed; new packs open new pages).
    pub atlas_loaded: bool,
}

impl SpriteBank {
    pub fn new(content_root: impl AsRef<Path>) -> Self {
        Self::with_atlas_size(content_root, ATLAS_SIZE)
    }

    /// Custom page size (tests use small atlases).
    pub fn with_atlas_size(content_root: impl AsRef<Path>, atlas_size: i32) -> Self {
        let size = atlas_size.max(1);
        Self {
            root: content_root.as_ref().to_path_buf(),
            atlas_size: size,
            pages: vec![AtlasPage::new(size)],
            rects: HashMap::new(),
            meta: HashMap::new(),
            hit_maps: HashMap::new(),
            missing: HashMap::new(),
            data_version: 0,
            index_loaded: false,
            atlas_loaded: false,
        }
    }

    /// Prefer `content_root/cache/ols1_sprites.bin`; rebake if missing/stale vs tree
    /// `dataVersionNumber.txt`; else scan `sprites/*.txt` (still no TGA pixel decode).
    ///
    /// **Default play path:** pixels stay on-demand via [`Self::ensure`].
    /// For optional full atlas restore see [`Self::load_prefer_atlas_cache`].
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
            LoadStage::Sprites,
            0.0,
            Some("prefer_cache"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let root = content_root.as_ref();
        let mut bank = Self::new(root);
        let cache = root.join("cache");
        let ols1_path = cache.join("ols1_sprites.bin");
        let tree_ver = read_data_version_u32(root);

        if ols1_path.exists() {
            if let Ok(bytes) = fs::read(&ols1_path) {
                if let Ok(ver) = bank.load_ols1_meta(&bytes) {
                    if tree_ver.map(|t| t == ver).unwrap_or(true) {
                        report_stage(
                            LoadStage::Sprites,
                            1.0,
                            Some("ols1"),
                            crate::load_progress::reborrow_cb(&mut on_progress),
                        );
                        return bank;
                    }
                    // Stale version — fall through to rebake.
                }
            }
        }

        // Rebake when sprites dir present.
        report_stage(
            LoadStage::Sprites,
            0.4,
            Some("scan_or_bake"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        if root.join("sprites").is_dir() {
            let ver = tree_ver.unwrap_or(0);
            if let Ok((bytes, _n)) = bake_ols1_from_dir(root, ver) {
                let _ = fs::create_dir_all(&cache);
                let _ = fs::write(&ols1_path, &bytes);
                // Best-effort manifest patch is done by full bake_content; here just index.
                let _ = bank.load_ols1_meta(&bytes);
                report_stage(
                    LoadStage::Sprites,
                    1.0,
                    Some("ols1_baked"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return bank;
            }
        }

        // Scan only (no write).
        let _ = bank.scan_meta_from_disk();
        report_stage(
            LoadStage::Sprites,
            1.0,
            Some("scan"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        bank
    }

    /// Prefer optional OLSA full multi-atlas dump, then OLS1 meta, then disk scan.
    ///
    /// Default client boot should keep using [`Self::load_prefer_cache`] (lazy TGA).
    pub fn load_prefer_atlas_cache(content_root: impl AsRef<Path>) -> Self {
        let root = content_root.as_ref();
        let mut bank = Self::new(root);
        let olsa_path = root.join("cache").join("olsa_sprite_atlas.bin");
        let tree_ver = read_data_version_u32(root);
        if olsa_path.exists() {
            if let Ok(bytes) = fs::read(&olsa_path) {
                if let Ok(stats) = bank.load_olsa_timed(&bytes) {
                    if tree_ver.map(|t| t == stats.data_version).unwrap_or(true) {
                        return bank;
                    }
                    // Stale — fall through.
                }
            }
        }
        // Meta-only path (same as load_prefer_cache).
        Self::load_prefer_cache(root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn pages(&self) -> &[AtlasPage] {
        &self.pages
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn packed_count(&self) -> usize {
        self.rects.len()
    }

    /// Number of sprite meta records currently known (OLS1 / scan / ensure).
    pub fn meta_count(&self) -> usize {
        self.meta.len()
    }

    pub fn is_missing(&self, sprite_id: i32) -> bool {
        self.missing.contains_key(&sprite_id)
    }

    pub fn get_rect(&self, sprite_id: i32) -> Option<&SpriteRect> {
        self.rects.get(&sprite_id)
    }

    pub fn get_meta(&self, sprite_id: i32) -> Option<&SpriteMeta> {
        self.meta.get(&sprite_id)
    }

    /// C++ `getSpriteHit` — local pixel in sprite image space (0..w, 0..h).
    /// Returns false for missing / unloaded hit maps or out-of-bounds.
    // C++: spriteBank.cpp getSpriteHit / SpriteRecord.hitMap
    pub fn get_sprite_hit(&self, sprite_id: i32, x: i32, y: i32) -> bool {
        let Some(hm) = self.hit_maps.get(&sprite_id) else {
            return false;
        };
        let Some(r) = self.rects.get(&sprite_id) else {
            return false;
        };
        if x < 0 || y < 0 || x >= r.width as i32 || y >= r.height as i32 {
            return false;
        }
        let idx = (y as u32 * r.width + x as u32) as usize;
        hm.get(idx).copied().unwrap_or(0) != 0
    }

    /// Ensure meta is loaded from `sprites/{id}.txt` (or default).
    pub fn ensure_meta(&mut self, sprite_id: i32) -> &SpriteMeta {
        if !self.meta.contains_key(&sprite_id) {
            let path = self.root.join("sprites").join(format!("{sprite_id}.txt"));
            let m = match std::fs::read_to_string(&path) {
                Ok(text) => parse_sprite_txt(sprite_id, &text),
                Err(_) => SpriteMeta {
                    id: sprite_id,
                    tag: "tag".into(),
                    ..Default::default()
                },
            };
            self.meta.insert(sprite_id, m);
        }
        self.meta.get(&sprite_id).expect("just inserted")
    }

    /// Insert meta without disk (tests / OLS1 load).
    pub fn insert_meta(&mut self, meta: SpriteMeta) {
        self.meta.insert(meta.id, meta);
    }

    /// Scan `sprites/{id}.txt` (+ optional TGA header size) into meta. No pixel decode.
    pub fn scan_meta_from_disk(&mut self) -> usize {
        let entries = scan_sprites_dir(&self.root);
        self.meta.clear();
        for m in entries {
            self.meta.insert(m.id, m);
        }
        self.index_loaded = !self.meta.is_empty();
        self.meta.len()
    }

    /// Ensure sprite is loaded into an atlas; returns UV rect + draw meta.
    pub fn ensure(&mut self, sprite_id: i32) -> Option<SpriteRect> {
        if sprite_id <= 0 {
            return None;
        }
        if let Some(r) = self.rects.get(&sprite_id) {
            return Some(*r);
        }
        if self.missing.contains_key(&sprite_id) {
            return None;
        }
        let path = self.root.join("sprites").join(format!("{sprite_id}.tga"));
        let img = match load_tga_path(&path) {
            Ok(i) => i,
            Err(_) => {
                self.missing.insert(sprite_id, ());
                return None;
            }
        };
        let meta = self.ensure_meta(sprite_id).clone();
        self.pack_image(sprite_id, &img, &meta)
    }

    /// Pack a decoded image (no disk TGA) — headless unit tests.
    pub fn ensure_rgba(
        &mut self,
        sprite_id: i32,
        img: &RgbaImage,
        meta: Option<SpriteMeta>,
    ) -> Option<SpriteRect> {
        if sprite_id <= 0 {
            return None;
        }
        if let Some(r) = self.rects.get(&sprite_id) {
            return Some(*r);
        }
        let meta = meta.unwrap_or_else(|| SpriteMeta {
            id: sprite_id,
            tag: format!("sprite_{sprite_id}"),
            ..Default::default()
        });
        self.meta.insert(sprite_id, meta.clone());
        self.pack_image(sprite_id, img, &meta)
    }

    fn pack_image(
        &mut self,
        sprite_id: i32,
        img: &RgbaImage,
        meta: &SpriteMeta,
    ) -> Option<SpriteRect> {
        let w = img.width as i32;
        let h = img.height as i32;
        if w <= 0 || h <= 0 {
            return None;
        }
        if w > self.atlas_size || h > self.atlas_size {
            // Oversized: cannot pack into page; sticky-missing (no special path yet).
            self.missing.insert(sprite_id, ());
            return None;
        }

        // C++: compute hitMap + centerXOffset/visibleW/maxD from alpha.
        let alpha = compute_alpha_info(img);
        self.hit_maps.insert(sprite_id, alpha.hit_map.clone());

        // Merge bbox into stored meta.
        if let Some(m) = self.meta.get_mut(&sprite_id) {
            m.center_x_offset = alpha.center_x_offset;
            m.center_y_offset = alpha.center_y_offset;
            m.visible_w = alpha.visible_w;
            m.visible_h = alpha.visible_h;
            m.max_d = alpha.max_d;
            m.width = alpha.width;
            m.height = alpha.height;
        }

        let make_sr = |ai: usize, rect: Rect| SpriteRect {
            atlas_index: ai,
            rect,
            width: img.width,
            height: img.height,
            center_anchor_x: meta.center_anchor_x,
            center_anchor_y: meta.center_anchor_y,
            multiplicative_blend: meta.multiplicative_blend,
            no_flip: meta.no_flip,
            center_x_offset: alpha.center_x_offset,
            center_y_offset: alpha.center_y_offset,
            visible_w: alpha.visible_w,
            visible_h: alpha.visible_h,
            max_d: alpha.max_d,
        };

        // Try existing pages (Haxe: loop packers, then new page).
        for ai in 0..self.pages.len() {
            if let Some(rect) = self.pages[ai].packer.pack(w, h) {
                self.pages[ai].blit(rect.x, rect.y, img);
                let sr = make_sr(ai, rect);
                self.rects.insert(sprite_id, sr);
                return Some(sr);
            }
        }
        // New page
        let mut page = AtlasPage::new(self.atlas_size);
        let rect = page.packer.pack(w, h)?;
        page.blit(rect.x, rect.y, img);
        let ai = self.pages.len();
        self.pages.push(page);
        let sr = make_sr(ai, rect);
        self.rects.insert(sprite_id, sr);
        Some(sr)
    }

    /// Preload a set of sprite ids (e.g. all sprites for objects in view).
    pub fn preload(&mut self, ids: impl IntoIterator<Item = i32>) {
        for id in ids {
            let _ = self.ensure(id);
        }
    }

    /// Preload every sprite referenced by the given object definitions (Haxe `loadSprites` loop).
    pub fn preload_from_objects<'a>(
        &mut self,
        objects: impl IntoIterator<Item = &'a crate::content::ClientObjectDef>,
    ) {
        for def in objects {
            for spr in &def.sprites {
                let _ = self.ensure(spr.sprite_id);
            }
        }
    }

    /// Pack every known sprite id that has a TGA (meta keys sorted for deterministic layout).
    ///
    /// Returns number of newly packed sprites (already-packed ids skipped).
    pub fn pack_all_from_meta(&mut self) -> usize {
        let mut ids: Vec<i32> = self.meta.keys().copied().collect();
        ids.sort_unstable();
        let before = self.rects.len();
        for id in ids {
            let _ = self.ensure(id);
        }
        self.rects.len().saturating_sub(before)
    }

    /// Serialize packed rects + RGBA pages to OLSA bytes (optional; not OLS1 meta).
    pub fn write_olsa(&self, data_version: u32) -> Vec<u8> {
        write_olsa_from_bank(self, data_version)
    }

    /// Restore multi-page atlas + rects from OLSA (no TGA required for packed sprites).
    pub fn load_olsa(&mut self, data: &[u8]) -> Result<u32, String> {
        Ok(self.load_olsa_timed(data)?.data_version)
    }

    /// Like [`Self::load_olsa`] but returns [`OlsaLoadStats`] with wall-clock duration.
    pub fn load_olsa_timed(&mut self, data: &[u8]) -> Result<OlsaLoadStats, String> {
        let t0 = Instant::now();
        let (ver, rects, pages, metas) = load_olsa(data)?;
        let page_count = pages.len();
        let packed = rects.len();

        // Rebuild hit maps from page alpha (not stored in OLSA v1).
        let mut hit_maps = HashMap::with_capacity(rects.len());
        for (id, sr) in &rects {
            if let Some(page) = pages.get(sr.atlas_index) {
                if let Some(hm) = hit_map_from_page(page, sr) {
                    hit_maps.insert(*id, hm);
                }
            }
        }

        self.pages = pages;
        self.rects = rects;
        self.hit_maps = hit_maps;
        self.missing.clear();
        // Merge metas from OLSA (tag/anchors) without wiping extra OLS1 fields if already set.
        for (id, m) in metas {
            self.meta.entry(id).or_insert(m);
        }
        self.data_version = ver;
        self.index_loaded = !self.meta.is_empty() || packed > 0;
        self.atlas_loaded = true;
        // Next ensure() that needs a new page will append; sealed pages won't overwrite.
        if self.pages.is_empty() {
            self.pages.push(AtlasPage::new(self.atlas_size));
        }

        Ok(OlsaLoadStats {
            bytes: data.len(),
            packed_sprites: packed,
            page_count,
            data_version: ver,
            duration: t0.elapsed(),
        })
    }

    /// Serialize packed sprite meta to OLS1 bytes (format 2: bbox + author; no pixel pages).
    pub fn write_ols1(&self, data_version: u32) -> Vec<u8> {
        let mut ids: Vec<i32> = self.meta.keys().copied().collect();
        ids.sort_unstable();
        let mut out = Vec::with_capacity(32 + ids.len() * 64);
        out.extend_from_slice(OLS1_MAGIC);
        out.extend_from_slice(&OLS1_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&data_version.to_le_bytes());
        out.extend_from_slice(&(ids.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // flags
        out.extend_from_slice(&0u32.to_le_bytes()); // header_crc32 reserved
        for id in ids {
            let meta = self.meta.get(&id).expect("key");
            let (w, h, ax, ay, cx, cy, vw, vh, md) = if let Some(r) = self.rects.get(&id) {
                (
                    r.width,
                    r.height,
                    r.center_anchor_x,
                    r.center_anchor_y,
                    r.center_x_offset,
                    r.center_y_offset,
                    r.visible_w,
                    r.visible_h,
                    r.max_d,
                )
            } else {
                (
                    meta.width,
                    meta.height,
                    meta.center_anchor_x,
                    meta.center_anchor_y,
                    meta.center_x_offset,
                    meta.center_y_offset,
                    meta.visible_w,
                    meta.visible_h,
                    meta.max_d,
                )
            };
            let mut flags = 0u32;
            if meta.multiplicative_blend {
                flags |= 1;
            }
            if meta.no_flip {
                flags |= 2;
            }
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&w.to_le_bytes());
            out.extend_from_slice(&h.to_le_bytes());
            out.extend_from_slice(&ax.to_le_bytes());
            out.extend_from_slice(&ay.to_le_bytes());
            out.extend_from_slice(&flags.to_le_bytes());
            let tag = meta.tag.as_bytes();
            let tlen = (tag.len() as u16).min(u16::MAX);
            out.extend_from_slice(&tlen.to_le_bytes());
            out.extend_from_slice(&tag[..tlen as usize]);
            // format 2 extras
            out.extend_from_slice(&cx.to_le_bytes());
            out.extend_from_slice(&cy.to_le_bytes());
            out.extend_from_slice(&vw.to_le_bytes());
            out.extend_from_slice(&vh.to_le_bytes());
            out.extend_from_slice(&md.to_le_bytes());
            let author = meta
                .author_tag
                .as_deref()
                .unwrap_or("")
                .as_bytes();
            let alen = (author.len() as u16).min(u16::MAX);
            out.extend_from_slice(&alen.to_le_bytes());
            out.extend_from_slice(&author[..alen as usize]);
        }
        out
    }

    /// Load OLS1 meta into this bank (does not load TGAs / rects / hit maps).
    /// Accepts format 1 (legacy) and format 2 (bbox + author).
    ///
    /// Replaces any previously loaded meta map.
    pub fn load_ols1_meta(&mut self, data: &[u8]) -> Result<u32, String> {
        if data.len() < 24 {
            return Err("OLS1 too short".into());
        }
        if &data[0..4] != OLS1_MAGIC {
            return Err("bad OLS1 magic".into());
        }
        let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if format != OLS1_FORMAT_VERSION_V1 && format != OLS1_FORMAT_VERSION {
            return Err(format!("unsupported OLS1 format {format}"));
        }
        let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        self.meta.clear();
        let mut off = 24usize;
        for _ in 0..count {
            if off + 4 + 4 + 4 + 4 + 4 + 4 + 2 > data.len() {
                return Err("OLS1 truncated record".into());
            }
            let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let w = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let h = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let ax = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let ay = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let flags = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            let tlen = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
            off += 2;
            if off + tlen > data.len() {
                return Err("OLS1 truncated tag".into());
            }
            let tag = String::from_utf8_lossy(&data[off..off + tlen]).into_owned();
            off += tlen;

            let mut center_x_offset = 0i32;
            let mut center_y_offset = 0i32;
            let mut visible_w = 0u32;
            let mut visible_h = 0u32;
            let mut max_d = w.max(h);
            let mut author_tag = None;

            if format >= OLS1_FORMAT_VERSION {
                if off + 4 * 5 + 2 > data.len() {
                    return Err("OLS1 truncated v2 fields".into());
                }
                center_x_offset = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
                off += 4;
                center_y_offset = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
                off += 4;
                visible_w = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
                off += 4;
                visible_h = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
                off += 4;
                max_d = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
                off += 4;
                let alen = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
                off += 2;
                if off + alen > data.len() {
                    return Err("OLS1 truncated author".into());
                }
                if alen > 0 {
                    author_tag = Some(String::from_utf8_lossy(&data[off..off + alen]).into_owned());
                }
                off += alen;
            }

            self.meta.insert(
                id,
                SpriteMeta {
                    id,
                    tag,
                    multiplicative_blend: flags & 1 != 0,
                    center_anchor_x: ax,
                    center_anchor_y: ay,
                    no_flip: flags & 2 != 0,
                    author_tag,
                    center_x_offset,
                    center_y_offset,
                    visible_w,
                    visible_h,
                    max_d,
                    width: w,
                    height: h,
                },
            );
        }
        self.data_version = data_version;
        self.index_loaded = !self.meta.is_empty() || count == 0;
        Ok(data_version)
    }
}

/// Peek TGA width/height from the 18-byte header only (no pixel decode).
fn peek_tga_size(path: &Path) -> Option<(u32, u32)> {
    let mut f = fs::File::open(path).ok()?;
    let mut hdr = [0u8; 18];
    f.read_exact(&mut hdr).ok()?;
    let w = u16::from_le_bytes([hdr[12], hdr[13]]) as u32;
    let h = u16::from_le_bytes([hdr[14], hdr[15]]) as u32;
    if w == 0 || h == 0 {
        None
    } else {
        Some((w, h))
    }
}

/// Scan `root/sprites/{id}.txt` for OLS1 meta. Optionally fills w/h/max_d from TGA header.
///
/// Does **not** decode TGA pixels or compute alpha-bbox / hitMap (runtime `ensure`).
pub fn scan_sprites_dir(root: &Path) -> Vec<SpriteMeta> {
    let sprites_dir = root.join("sprites");
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(&sprites_dir) else {
        return out;
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if path.is_dir() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().into_owned();
        let lower = name.to_lowercase();
        if !lower.ends_with(".txt") {
            continue;
        }
        let stem = match name.rsplit_once('.') {
            Some((s, _)) => s,
            None => continue,
        };
        // Skip non-numeric / special names (nextSpriteNumber.txt etc.).
        let Ok(id) = stem.parse::<i32>() else {
            continue;
        };
        if id <= 0 {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        let mut meta = parse_sprite_txt(id, &text);
        // Cheap size fill from sibling TGA header when present.
        let tga = sprites_dir.join(format!("{id}.tga"));
        if let Some((w, h)) = peek_tga_size(&tga) {
            meta.width = w;
            meta.height = h;
            meta.max_d = w.max(h);
        }
        out.push(meta);
    }
    out.sort_by_key(|m| m.id);
    out
}

/// Bake OLS1 from content root `sprites/`. Returns (bytes, count).
///
/// Meta-only: txt tags/anchors + optional TGA header size. No atlas pages / hitMaps.
pub fn bake_ols1_from_dir(root: &Path, data_version: u32) -> Result<(Vec<u8>, usize), String> {
    let mut bank = SpriteBank::new(root);
    let n = bank.scan_meta_from_disk();
    Ok((bank.write_ols1(data_version), n))
}

/// Write `out_dir/ols1_sprites.bin`.
pub fn bake_ols1_to_dir(root: &Path, out_dir: &Path, data_version: u32) -> Result<usize, String> {
    let (bytes, n) = bake_ols1_from_dir(root, data_version)?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("ols1_sprites.bin"), bytes).map_err(|e| e.to_string())?;
    Ok(n)
}

// ── OLSA binary (optional full multi-atlas dump) ─────────────────────────────

/// Stats from an OLSA bake (pack + serialize + optional disk write).
#[derive(Debug, Clone, Copy, Default)]
pub struct OlsaBakeStats {
    pub bytes: usize,
    pub packed_sprites: usize,
    pub page_count: usize,
    pub meta_count: usize,
    pub pack_duration: Duration,
    pub write_duration: Duration,
    pub total_duration: Duration,
}

impl OlsaBakeStats {
    pub fn report_line(&self) -> String {
        format!(
            "packed={} meta={} pages={} bytes={} pack={:.1}ms write={:.1}ms total={:.1}ms",
            self.packed_sprites,
            self.meta_count,
            self.page_count,
            self.bytes,
            self.pack_duration.as_secs_f64() * 1000.0,
            self.write_duration.as_secs_f64() * 1000.0,
            self.total_duration.as_secs_f64() * 1000.0,
        )
    }
}

/// Stats from loading an OLSA blob.
#[derive(Debug, Clone, Copy, Default)]
pub struct OlsaLoadStats {
    pub bytes: usize,
    pub packed_sprites: usize,
    pub page_count: usize,
    pub data_version: u32,
    pub duration: Duration,
}

impl OlsaLoadStats {
    pub fn report_line(&self) -> String {
        format!(
            "ver={} packed={} pages={} bytes={} load={:.1}ms",
            self.data_version,
            self.packed_sprites,
            self.page_count,
            self.bytes,
            self.duration.as_secs_f64() * 1000.0,
        )
    }
}

fn write_olsa_from_bank(bank: &SpriteBank, data_version: u32) -> Vec<u8> {
    let mut ids: Vec<i32> = bank.rects.keys().copied().collect();
    ids.sort_unstable();
    let record_count = ids.len() as u32;
    let num_pages = bank.pages.len() as u16;

    let mut out = Vec::with_capacity(
        48 + record_count as usize * 48
            + bank
                .pages
                .iter()
                .map(|p| 12 + p.pixels.len())
                .sum::<usize>(),
    );
    out.extend_from_slice(OLSA_MAGIC);
    out.extend_from_slice(&OLSA_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&data_version.to_le_bytes());
    out.extend_from_slice(&record_count.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes()); // header_crc32
    out.extend_from_slice(&(bank.atlas_size as u32).to_le_bytes());
    out.extend_from_slice(&num_pages.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved2

    for id in &ids {
        let sr = bank.rects.get(id).expect("key");
        let meta = bank.meta.get(id);
        let mut flags = 0u32;
        if sr.multiplicative_blend {
            flags |= 1;
        }
        if sr.no_flip {
            flags |= 2;
        }
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(sr.atlas_index as u16).to_le_bytes());
        out.extend_from_slice(&sr.rect.x.to_le_bytes());
        out.extend_from_slice(&sr.rect.y.to_le_bytes());
        out.extend_from_slice(&sr.rect.width.to_le_bytes());
        out.extend_from_slice(&sr.rect.height.to_le_bytes());
        out.extend_from_slice(&(sr.width as u16).to_le_bytes());
        out.extend_from_slice(&(sr.height as u16).to_le_bytes());
        out.extend_from_slice(&sr.center_anchor_x.to_le_bytes());
        out.extend_from_slice(&sr.center_anchor_y.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&sr.center_x_offset.to_le_bytes());
        out.extend_from_slice(&sr.center_y_offset.to_le_bytes());
        out.extend_from_slice(&sr.visible_w.to_le_bytes());
        out.extend_from_slice(&sr.visible_h.to_le_bytes());
        out.extend_from_slice(&sr.max_d.to_le_bytes());
        let tag = meta.map(|m| m.tag.as_bytes()).unwrap_or(b"");
        let tlen = (tag.len() as u16).min(u16::MAX);
        out.extend_from_slice(&tlen.to_le_bytes());
        out.extend_from_slice(&tag[..tlen as usize]);
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

fn load_olsa(
    data: &[u8],
) -> Result<
    (
        u32,
        HashMap<i32, SpriteRect>,
        Vec<AtlasPage>,
        HashMap<i32, SpriteMeta>,
    ),
    String,
> {
    if data.len() < 24 + 12 {
        return Err("OLSA too short".into());
    }
    if &data[0..4] != OLSA_MAGIC {
        return Err("bad OLSA magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLSA_FORMAT_VERSION {
        return Err(format!("unsupported OLSA format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let record_count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let mut off = 24usize;
    let page_size = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as i32;
    off += 4;
    let num_pages = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
    off += 2;
    off += 2; // reserved
    off += 4; // reserved2

    let mut rects = HashMap::with_capacity(record_count);
    let mut metas = HashMap::with_capacity(record_count);
    for _ in 0..record_count {
        // Fixed fields before tag: id4+atlas2+rect16+wh4+axay8+flags4+cxcy8+vw/vh/md12 = 58
        if off + 58 + 2 > data.len() {
            return Err("OLSA truncated record".into());
        }
        let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
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
        let ax = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let ay = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let flags = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let cx = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let cy = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let vw = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let vh = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let md = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let tlen = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
        off += 2;
        if off + tlen > data.len() {
            return Err("OLSA truncated tag".into());
        }
        let tag = String::from_utf8_lossy(&data[off..off + tlen]).into_owned();
        off += tlen;

        let mult = (flags & 1) != 0;
        let no_flip = (flags & 2) != 0;
        rects.insert(
            id,
            SpriteRect {
                atlas_index,
                rect: Rect {
                    x: rx,
                    y: ry,
                    width: rw,
                    height: rh,
                },
                width,
                height,
                center_anchor_x: ax,
                center_anchor_y: ay,
                multiplicative_blend: mult,
                no_flip,
                center_x_offset: cx,
                center_y_offset: cy,
                visible_w: vw,
                visible_h: vh,
                max_d: md,
            },
        );
        metas.insert(
            id,
            SpriteMeta {
                id,
                tag,
                multiplicative_blend: mult,
                center_anchor_x: ax,
                center_anchor_y: ay,
                no_flip,
                author_tag: None,
                center_x_offset: cx,
                center_y_offset: cy,
                visible_w: vw,
                visible_h: vh,
                max_d: md,
                width,
                height,
            },
        );
    }

    let mut pages = Vec::with_capacity(num_pages.max(1));
    for _ in 0..num_pages {
        if off + 12 > data.len() {
            return Err("OLSA truncated page header".into());
        }
        let w = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let h = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let blen = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
        off += 4;
        if off + blen > data.len() {
            return Err("OLSA truncated page pixels".into());
        }
        let pixels = data[off..off + blen].to_vec();
        off += blen;
        pages.push(AtlasPage {
            width: w,
            height: h,
            pixels,
            packer: BinPack::sealed(w as i32, h as i32),
        });
    }
    let _ = page_size; // used for validation optionally
    let _ = off;
    Ok((data_version, rects, pages, metas))
}

/// Rebuild C++-style hitMap from packed page alpha (simple threshold; no expandMap).
fn hit_map_from_page(page: &AtlasPage, sr: &SpriteRect) -> Option<Vec<u8>> {
    let w = sr.width as i32;
    let h = sr.height as i32;
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut hit = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = sr.rect.x + x;
            let dy = sr.rect.y + y;
            if dx < 0 || dy < 0 || dx as u32 >= page.width || dy as u32 >= page.height {
                continue;
            }
            let i = ((dy as u32 * page.width + dx as u32) * 4) as usize;
            let a = page.pixels.get(i + 3).copied().unwrap_or(0);
            if a >= HIT_ALPHA_THRESHOLD {
                hit[(y * w + x) as usize] = 1;
            }
        }
    }
    Some(hit)
}

/// Bake full multi-page sprite atlas (OLSA) from content root `sprites/`.
///
/// Scans meta, packs all existing TGAs, returns OLSA bytes + stats.
/// Large output with full OHOL sprite trees — optional path only.
pub fn bake_olsa_from_dir(
    root: &Path,
    data_version: u32,
) -> Result<(Vec<u8>, OlsaBakeStats), String> {
    bake_olsa_from_dir_with_atlas_size(root, data_version, ATLAS_SIZE)
}

/// Same as [`bake_olsa_from_dir`] with custom atlas page size (tests use small pages).
pub fn bake_olsa_from_dir_with_atlas_size(
    root: &Path,
    data_version: u32,
    atlas_size: i32,
) -> Result<(Vec<u8>, OlsaBakeStats), String> {
    let total0 = Instant::now();
    let mut bank = SpriteBank::with_atlas_size(root, atlas_size);
    let t_pack = Instant::now();
    let meta_count = bank.scan_meta_from_disk();
    let packed = bank.pack_all_from_meta();
    let pack_duration = t_pack.elapsed();

    let t_ser = Instant::now();
    let bytes = bank.write_olsa(data_version);
    let write_duration = t_ser.elapsed();

    let stats = OlsaBakeStats {
        bytes: bytes.len(),
        packed_sprites: packed.max(bank.packed_count()),
        page_count: bank.page_count(),
        meta_count,
        pack_duration,
        write_duration,
        total_duration: total0.elapsed(),
    };
    Ok((bytes, stats))
}

/// Write `out_dir/olsa_sprite_atlas.bin` (optional; not part of default bake_content).
pub fn bake_olsa_to_dir(
    root: &Path,
    out_dir: impl AsRef<Path>,
    data_version: u32,
) -> Result<OlsaBakeStats, String> {
    let total0 = Instant::now();
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let (bytes, mut stats) = bake_olsa_from_dir(root, data_version)?;
    let path = out_dir.join("olsa_sprite_atlas.bin");
    let t_write = Instant::now();
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    stats.write_duration += t_write.elapsed();
    stats.bytes = bytes.len();
    stats.total_duration = total0.elapsed();
    Ok(stats)
}

fn read_data_version_u32(root: &Path) -> Option<u32> {
    fs::read_to_string(root.join("dataVersionNumber.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// C++ `loadSpriteFromRawTGAData` alpha pass: hitMap (pre-expand), bbox, maxD.
pub fn compute_alpha_info(img: &RgbaImage) -> SpriteAlphaInfo {
    let w = img.width as i32;
    let h = img.height as i32;
    let num = (img.width * img.height) as usize;
    let mut hit_map = vec![1u8; num];

    let mut min_x = w;
    let mut max_x = 0i32;
    let mut min_y = h;
    let mut max_y = 0i32;
    let mut any = false;

    for y in 0..h {
        for x in 0..w {
            let p = (y * w + x) as usize;
            let a = img.pixel(x as u32, y as u32)[3];
            // C++: if bytes[b] < 64 → hitMap = 0
            if a < HIT_ALPHA_THRESHOLD {
                hit_map[p] = 0;
            } else {
                any = true;
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }

    // C++: three expandMap passes for friendlier click targets.
    for _ in 0..3 {
        expand_map(&mut hit_map, w, h);
    }

    let (center_x_offset, center_y_offset, visible_w, visible_h) = if any {
        (
            (max_x + min_x) / 2 - w / 2,
            (max_y + min_y) / 2 - h / 2,
            (max_x - min_x) as u32,
            (max_y - min_y) as u32,
        )
    } else {
        (0, 0, 0, 0)
    };

    let max_d = img.width.max(img.height);

    SpriteAlphaInfo {
        center_x_offset,
        center_y_offset,
        visible_w,
        visible_h,
        max_d,
        width: img.width,
        height: img.height,
        hit_map,
    }
}

/// C++ `expandMap` — dilate hit pixels by 4-neighbors (interior only).
// C++: spriteBank.cpp expandMap
pub fn expand_map(map: &mut [u8], w: i32, h: i32) {
    if w < 3 || h < 3 || map.len() != (w * h) as usize {
        return;
    }
    let copy = map.to_vec();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let index = (y * w + x) as usize;
            if copy[index] != 0 {
                map[index - 1] = 1;
                map[index + 1] = 1;
                map[index - w as usize] = 1;
                map[index + w as usize] = 1;
            }
        }
    }
}

/// Parse C++ / Haxe sprite meta line(s).
///
/// Format (single line, space-separated):
/// `tag mult [centerAnchorX centerAnchorY] [author=Name]`
///
/// Examples:
/// - `Rock 0 0 0`
/// - `LegLeftBot 1 -1 -12`
pub fn parse_sprite_txt(id: i32, text: &str) -> SpriteMeta {
    let line = text.lines().next().unwrap_or("").trim();
    let mut tokens = line.split_whitespace();
    let tag = tokens.next().unwrap_or("tag").to_string();
    let mult = tokens
        .next()
        .and_then(|t| t.parse::<i32>().ok())
        .unwrap_or(0);
    let center_anchor_x = tokens
        .next()
        .and_then(|t| t.parse::<i32>().ok())
        .unwrap_or(0);
    let center_anchor_y = tokens
        .next()
        .and_then(|t| t.parse::<i32>().ok())
        .unwrap_or(0);
    let mut author_tag = None;
    // Accept author= anywhere in remaining tokens (C++ may interleave).
    for tok in tokens {
        if let Some(rest) = tok.strip_prefix("author=") {
            if !rest.is_empty() {
                author_tag = Some(rest.to_string());
            }
        }
    }
    let no_flip = tag.contains("NoFlip");
    SpriteMeta {
        id,
        tag,
        multiplicative_blend: mult == 1,
        center_anchor_x,
        center_anchor_y,
        no_flip,
        author_tag,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ClientObjectDef, ObjectSprite};
    use crate::tga::RgbaImage;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for px in pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    /// Opaque square inset in a transparent canvas.
    fn inset_opaque(w: u32, h: u32, x0: u32, y0: u32, bw: u32, bh: u32) -> RgbaImage {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for y in y0..y0 + bh {
            for x in x0..x0 + bw {
                if x < w && y < h {
                    let i = ((y * w + x) * 4) as usize;
                    pixels[i] = 255;
                    pixels[i + 1] = 0;
                    pixels[i + 2] = 0;
                    pixels[i + 3] = 255;
                }
            }
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn parse_sprite_meta_line() {
        let m = parse_sprite_txt(144, "Rock 0 0 0\n");
        assert_eq!(m.tag, "Rock");
        assert!(!m.multiplicative_blend);
        assert_eq!((m.center_anchor_x, m.center_anchor_y), (0, 0));

        let m = parse_sprite_txt(100, "LegLeftBot 1 -1 -12");
        assert_eq!(m.tag, "LegLeftBot");
        assert!(m.multiplicative_blend);
        assert_eq!((m.center_anchor_x, m.center_anchor_y), (-1, -12));

        let m = parse_sprite_txt(2, "WordNoFlip 0 3 4 author=jason");
        assert!(m.no_flip);
        assert_eq!(m.author_tag.as_deref(), Some("jason"));
        assert_eq!((m.center_anchor_x, m.center_anchor_y), (3, 4));
    }

    #[test]
    fn pack_into_small_atlas_and_second_page() {
        let mut bank = SpriteBank::with_atlas_size(".", 32);
        let a = solid(20, 20, [255, 0, 0, 255]);
        let b = solid(20, 20, [0, 255, 0, 255]);
        let c = solid(20, 20, [0, 0, 255, 255]);
        let ra = bank.ensure_rgba(1, &a, None).unwrap();
        let rb = bank.ensure_rgba(2, &b, None).unwrap();
        let rc = bank.ensure_rgba(3, &c, None).unwrap();
        assert_eq!(ra.atlas_index, 0);
        // 32² fits one 20²; next two spill to new pages — same (0,0) UV on each page is OK.
        assert_eq!(bank.packed_count(), 3);
        assert!(bank.page_count() >= 2, "expected multi-page spill");
        // Distinct packing: either different page or different rect.
        assert!(
            ra.atlas_index != rb.atlas_index
                || (ra.rect.x, ra.rect.y) != (rb.rect.x, rb.rect.y)
        );
        let _ = rc;
    }

    #[test]
    fn ols1_roundtrip_meta() {
        let mut bank = SpriteBank::with_atlas_size(".", 64);
        let img = solid(8, 8, [10, 20, 30, 255]);
        bank.ensure_rgba(
            7,
            &img,
            Some(SpriteMeta {
                id: 7,
                tag: "TestNoFlip".into(),
                multiplicative_blend: true,
                center_anchor_x: 2,
                center_anchor_y: -3,
                no_flip: true,
                author_tag: Some("jason".into()),
                ..Default::default()
            }),
        )
        .unwrap();
        let bytes = bank.write_ols1(42);
        assert_eq!(&bytes[0..4], b"OLS1");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLS1_FORMAT_VERSION
        );
        let mut bank2 = SpriteBank::with_atlas_size(".", 64);
        let ver = bank2.load_ols1_meta(&bytes).unwrap();
        assert_eq!(ver, 42);
        let m = bank2.get_meta(7).unwrap();
        assert_eq!(m.tag, "TestNoFlip");
        assert!(m.multiplicative_blend);
        assert!(m.no_flip);
        assert_eq!((m.center_anchor_x, m.center_anchor_y), (2, -3));
        assert_eq!(m.author_tag.as_deref(), Some("jason"));
        assert_eq!(m.width, 8);
        assert_eq!(m.height, 8);
        assert_eq!(m.max_d, 8);
        // full opaque → visible spans 0..7 ⇒ visible 7
        assert_eq!(m.visible_w, 7);
        assert_eq!(m.visible_h, 7);
    }

    #[test]
    fn ols1_v1_legacy_load() {
        // Manually craft format-1 record.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"OLS1");
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&9u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&3i32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&(-2i32).to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes()); // mult
        let tag = b"A";
        bytes.extend_from_slice(&(tag.len() as u16).to_le_bytes());
        bytes.extend_from_slice(tag);
        let mut bank = SpriteBank::with_atlas_size(".", 16);
        let ver = bank.load_ols1_meta(&bytes).unwrap();
        assert_eq!(ver, 9);
        let m = bank.get_meta(3).unwrap();
        assert_eq!(m.tag, "A");
        assert!(m.multiplicative_blend);
        assert_eq!(m.author_tag, None);
        assert_eq!((m.center_anchor_x, m.center_anchor_y), (1, -2));
    }

    #[test]
    fn alpha_bbox_and_hit_map() {
        // 16×16 transparent with 4×4 opaque block at (4,6)
        let img = inset_opaque(16, 16, 4, 6, 4, 4);
        let info = compute_alpha_info(&img);
        assert_eq!(info.max_d, 16);
        // minX=4 maxX=7 → center (5) - 8 = -3
        assert_eq!(info.center_x_offset, (4 + 7) / 2 - 8);
        // minY=6 maxY=9 → center 7 - 8 = -1
        assert_eq!(info.center_y_offset, (6 + 9) / 2 - 8);
        assert_eq!(info.visible_w, 3);
        assert_eq!(info.visible_h, 3);

        let mut bank = SpriteBank::with_atlas_size(".", 64);
        let r = bank.ensure_rgba(50, &img, None).unwrap();
        assert_eq!(r.center_x_offset, info.center_x_offset);
        assert_eq!(r.visible_w, info.visible_w);
        // Opaque interior is a hit.
        assert!(bank.get_sprite_hit(50, 5, 7));
        // Far corner stays miss even after expand (edge-avoiding expand).
        assert!(!bank.get_sprite_hit(50, 0, 0));
        assert!(!bank.get_sprite_hit(50, 100, 100));
    }

    #[test]
    fn oversized_sticky_missing() {
        let mut bank = SpriteBank::with_atlas_size(".", 16);
        let img = solid(32, 8, [255, 0, 0, 255]);
        assert!(bank.ensure_rgba(99, &img, None).is_none());
        assert!(bank.is_missing(99));
        // sticky
        assert!(bank.ensure_rgba(99, &img, None).is_none());
    }

    #[test]
    fn olsa_roundtrip_from_rgba() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsa_rt_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        // Minimal txt meta (no TGA — pack via ensure_rgba then write_olsa).
        fs::write(tmp.join("sprites").join("1.txt"), "Red 0 0 0\n").unwrap();
        fs::write(tmp.join("sprites").join("2.txt"), "Green 1 1 2\n").unwrap();

        let mut bank = SpriteBank::with_atlas_size(&tmp, 64);
        let _ = bank.scan_meta_from_disk();
        bank.ensure_rgba(1, &solid(16, 16, [255, 0, 0, 255]), None)
            .unwrap();
        bank.ensure_rgba(2, &solid(16, 16, [0, 255, 0, 255]), None)
            .unwrap();
        let bytes = bank.write_olsa(7);
        assert!(bytes.starts_with(OLSA_MAGIC));
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLSA_FORMAT_VERSION
        );

        let mut loaded = SpriteBank::with_atlas_size(&tmp, 64);
        let stats = loaded.load_olsa_timed(&bytes).unwrap();
        assert_eq!(stats.data_version, 7);
        assert_eq!(stats.packed_sprites, 2);
        assert!(stats.duration.as_secs_f64() >= 0.0);
        assert!(loaded.atlas_loaded);
        assert_eq!(loaded.page_count(), bank.page_count());
        let r1 = *loaded.get_rect(1).unwrap();
        let r2 = *loaded.get_rect(2).unwrap();
        assert_eq!(r1.width, 16);
        assert_eq!(r2.height, 16);
        // Hit map rebuilt from page alpha.
        assert!(loaded.get_sprite_hit(1, 8, 8));
        // Pixel sample from restored page matches source red.
        let page = &loaded.pages()[r1.atlas_index];
        let px = ((r1.rect.y as u32 * page.width + r1.rect.x as u32) * 4) as usize;
        let red_sample = page.pixels[px..px + 4].to_vec();
        assert_eq!(&red_sample, &[255, 0, 0, 255]);

        // Sealed pages: packing a new id opens a fresh page; does not overwrite restored pixels.
        let pages_before = loaded.page_count();
        let sealed_free = loaded.pages()[0].packer.free_count();
        assert_eq!(sealed_free, 0, "restored page packer must be sealed");
        let r1_atlas = r1.atlas_index;
        loaded
            .ensure_rgba(99, &solid(8, 8, [0, 0, 255, 255]), None)
            .unwrap();
        assert!(
            loaded.page_count() > pages_before,
            "new pack after OLSA load must open a new page"
        );
        // Original red pixel still intact on page 0.
        let page0 = &loaded.pages()[r1_atlas];
        assert_eq!(&page0.pixels[px..px + 4], red_sample.as_slice());
        assert_eq!(loaded.get_rect(99).unwrap().atlas_index, pages_before);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn olsa_load_prefer_atlas_cache() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsa_prefer_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "11\n").unwrap();
        fs::write(tmp.join("sprites").join("3.txt"), "Blue 0 0 0\n").unwrap();

        let mut bank = SpriteBank::with_atlas_size(&tmp, 64);
        bank.ensure_rgba(3, &solid(12, 12, [0, 0, 200, 255]), None)
            .unwrap();
        let bytes = bank.write_olsa(11);
        let cache = tmp.join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("olsa_sprite_atlas.bin"), &bytes).unwrap();

        let prefer = SpriteBank::load_prefer_atlas_cache(&tmp);
        assert!(prefer.atlas_loaded, "matching dataVersion should load OLSA");
        assert_eq!(prefer.data_version, 11);
        assert_eq!(prefer.packed_count(), 1);
        assert!(prefer.get_rect(3).is_some());
        // Default prefer_cache stays OLS1/lazy (does not auto-load OLSA).
        let meta_only = SpriteBank::load_prefer_cache(&tmp);
        assert!(!meta_only.atlas_loaded);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn olsa_version_invalidation() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsa_ver_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        // Tree version 99; blob baked as version 1 → must reject atlas.
        fs::write(tmp.join("dataVersionNumber.txt"), "99\n").unwrap();
        fs::write(tmp.join("sprites").join("5.txt"), "X 0 0 0\n").unwrap();

        let mut bank = SpriteBank::with_atlas_size(&tmp, 64);
        bank.ensure_rgba(5, &solid(8, 8, [9, 9, 9, 255]), None)
            .unwrap();
        let bytes = bank.write_olsa(1);
        let cache = tmp.join("cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("olsa_sprite_atlas.bin"), &bytes).unwrap();

        let prefer = SpriteBank::load_prefer_atlas_cache(&tmp);
        assert!(
            !prefer.atlas_loaded,
            "stale OLSA data_version must not be used"
        );
        // Fall-through may scan meta from sprites/*.txt
        assert!(prefer.meta_count() >= 1 || prefer.packed_count() == 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn olsa_bad_magic() {
        let mut bank = SpriteBank::with_atlas_size(".", 32);
        assert!(bank
            .load_olsa(b"XXXX................................")
            .is_err());
    }

    #[test]
    fn olsa_pack_all_from_meta_sorted() {
        // pack_all_from_meta iterates sorted meta keys (determinism for bake layout).
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsa_pack_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        // Only meta — no TGA → pack_all returns 0 newly packed, but order is sorted.
        fs::write(tmp.join("sprites").join("20.txt"), "B 0\n").unwrap();
        fs::write(tmp.join("sprites").join("10.txt"), "A 0\n").unwrap();
        let mut bank = SpriteBank::with_atlas_size(&tmp, 64);
        let n_meta = bank.scan_meta_from_disk();
        assert_eq!(n_meta, 2);
        let n = bank.pack_all_from_meta();
        assert_eq!(n, 0, "no TGA → nothing packed");
        // With synthetic RGBA after insert, pack_all still skips already-packed.
        bank.ensure_rgba(10, &solid(4, 4, [1, 1, 1, 255]), None)
            .unwrap();
        bank.ensure_rgba(20, &solid(4, 4, [2, 2, 2, 255]), None)
            .unwrap();
        assert_eq!(bank.pack_all_from_meta(), 0);
        let (bytes, stats) = bake_olsa_from_dir_with_atlas_size(&tmp, 3, 64).unwrap();
        // No TGA on disk → bake packs 0, but write still produces valid header.
        assert!(bytes.starts_with(OLSA_MAGIC));
        assert_eq!(stats.meta_count, 2);
        assert!(stats.total_duration.as_secs_f64() >= 0.0);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn preload_from_objects_unique() {
        let mut bank = SpriteBank::with_atlas_size(".", 64);
        let img = solid(4, 4, [1, 2, 3, 255]);
        bank.ensure_rgba(10, &img, None).unwrap();
        bank.ensure_rgba(11, &img, None).unwrap();
        // Pretend objects share sprite 10 twice — still one rect.
        let defs = [
            ClientObjectDef {
                id: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 10,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 11,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ClientObjectDef {
                id: 2,
                sprites: vec![ObjectSprite {
                    sprite_id: 10,
                    ..Default::default()
                }],
                ..Default::default()
            },
        ];
        // Disk ensure for missing ids will fail; only already-packed stay.
        // Preload with synthetic path: call ensure_rgba first, then ensure again via preload.
        bank.preload_from_objects(&defs);
        assert_eq!(bank.packed_count(), 2);
        assert!(bank.get_rect(10).is_some());
        assert!(bank.get_rect(11).is_some());
    }

    #[test]
    fn load_one_sprite_into_atlas() {
        let root = r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7";
        if !Path::new(root).join("sprites").join("144.tga").exists() {
            return;
        }
        let mut bank = SpriteBank::new(root);
        let r = bank.ensure(144).expect("sprite 144");
        assert!(r.width > 0);
        assert_eq!(bank.pages().len(), 1);
        let m = bank.get_meta(144).expect("meta");
        assert_eq!(m.tag, "Rock");
        assert!(r.max_d > 0);
        // Real Rock should have some opaque pixels for hit.
        assert!(bank.get_sprite_hit(144, r.width as i32 / 2, r.height as i32 / 2));
    }

    #[test]
    fn bake_ols1_from_fixture() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_ols1_bake_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "5\n").unwrap();
        fs::write(
            tmp.join("sprites").join("7.txt"),
            "Rock 0 1 -2 author=jason\n",
        )
        .unwrap();
        fs::write(
            tmp.join("sprites").join("9.txt"),
            "LegNoFlip 1 3 4\n",
        )
        .unwrap();
        // non-numeric skip
        fs::write(tmp.join("sprites").join("nextSpriteNumber.txt"), "100\n").unwrap();

        let (bytes, n) = bake_ols1_from_dir(&tmp, 5).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&bytes[0..4], b"OLS1");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLS1_FORMAT_VERSION
        );
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 5);
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 2);

        let mut bank = SpriteBank::new(&tmp);
        let ver = bank.load_ols1_meta(&bytes).unwrap();
        assert_eq!(ver, 5);
        assert_eq!(bank.meta_count(), 2);
        assert!(bank.index_loaded);
        let m7 = bank.get_meta(7).unwrap();
        assert_eq!(m7.tag, "Rock");
        assert_eq!((m7.center_anchor_x, m7.center_anchor_y), (1, -2));
        assert_eq!(m7.author_tag.as_deref(), Some("jason"));
        let m9 = bank.get_meta(9).unwrap();
        assert!(m9.multiplicative_blend);
        assert!(m9.no_flip);

        let n2 = bake_ols1_to_dir(&tmp, &tmp.join("cache"), 5).unwrap();
        assert_eq!(n2, 2);
        assert!(tmp.join("cache").join("ols1_sprites.bin").exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_prefer_cache_writes_ols1_when_missing() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_ols1_pref_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sprites")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "12\n").unwrap();
        fs::write(tmp.join("sprites").join("3.txt"), "A 0 0 0\n").unwrap();

        assert!(!tmp.join("cache").join("ols1_sprites.bin").exists());
        let bank = SpriteBank::load_prefer_cache(&tmp);
        assert_eq!(bank.meta_count(), 1);
        assert_eq!(bank.data_version, 12);
        assert!(tmp.join("cache").join("ols1_sprites.bin").exists());
        assert_eq!(bank.get_meta(3).unwrap().tag, "A");

        // Second load uses cache (no sprites scan needed).
        let bank2 = SpriteBank::load_prefer_cache(&tmp);
        assert_eq!(bank2.meta_count(), 1);
        assert_eq!(bank2.get_meta(3).unwrap().tag, "A");

        // Stale data_version rebakes.
        fs::write(tmp.join("dataVersionNumber.txt"), "13\n").unwrap();
        fs::write(tmp.join("sprites").join("4.txt"), "B 0 0 0\n").unwrap();
        let bank3 = SpriteBank::load_prefer_cache(&tmp);
        assert_eq!(bank3.data_version, 13);
        assert_eq!(bank3.meta_count(), 2);
        assert!(bank3.get_meta(4).is_some());

        let _ = fs::remove_dir_all(&tmp);
    }
}
