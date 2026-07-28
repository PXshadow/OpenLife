//! Editor import overlays bank (C++ `overlayBank` / `OverlayRecord`).
//!
//! - **Layout:** `overlays/{tag}/{id}.tga` (tag = subdirectory name).
//! - **Boot:** scan id → tag/path index only (no Image / sprite load).
//! - **Lazy TGA:** [`OverlayBank::ensure_image`] decodes on first use.
//! - **OLO1:** optional binary path index (`cache/olo1_overlays.bin`) for
//!   bake_content / prefer_cache; pixels stay on-demand TGA.
//!
//! Editor add/delete (`addOverlay` / `deleteOverlayFromBank`) out of scope.
//! C++ loads thumbnailSprite + full Image eagerly; we do not.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::tga::{load_tga_path, RgbaImage};

/// OLO1 magic — overlay path index cache (not OLC1 objects).
pub const OLO1_MAGIC: &[u8; 4] = b"OLO1";
/// OLO1 format version (dense id/tag/path records).
pub const OLO1_FORMAT_VERSION: u32 = 1;

/// One overlay index entry (C++ `OverlayRecord` without live Image/sprite).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayRecord {
    pub id: i32,
    /// Tag folder name (C++ `tag`).
    pub tag: String,
    /// Path relative to a content root, e.g. `overlays/Dots/6.tga`.
    pub rel_path: String,
    /// Absolute path when known from disk scan (may be None after OLO1-only load).
    pub abs_path: Option<PathBuf>,
    /// TGA width when known (0 until decode or header fill).
    pub width: u32,
    /// TGA height when known.
    pub height: u32,
}

/// Overlay bank: sparse id map + lazy pixel cache.
pub struct OverlayBank {
    /// Search roots that may contain `overlays/`.
    roots: Vec<PathBuf>,
    by_id: HashMap<i32, OverlayRecord>,
    /// Sorted ascending ids (reverse iteration for empty-search).
    ids: Vec<i32>,
    /// Lazy decoded RGBA (straight alpha).
    images: HashMap<i32, RgbaImage>,
    missing_decode: HashMap<i32, ()>,
    pub data_version: u32,
    /// True when OLO1 or disk scan populated the index.
    pub index_loaded: bool,
}

impl OverlayBank {
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
            by_id: HashMap::new(),
            ids: Vec::new(),
            images: HashMap::new(),
            missing_decode: HashMap::new(),
            data_version: 0,
            index_loaded: false,
        }
    }

    pub fn with_roots(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut b = Self::new();
        b.roots.extend(roots);
        b
    }

    /// Default roots: content root, `OHOL_CONTENT_DIR`, OneLifeData7 siblings.
    pub fn with_default_roots(content_root: Option<&Path>) -> Self {
        Self::with_roots(default_overlay_roots(content_root))
    }

    /// Prefer `content_root/cache/olo1_overlays.bin`; else scan `overlays/` on disk.
    pub fn load_prefer_cache(content_root: impl AsRef<Path>) -> Self {
        let root = content_root.as_ref();
        let mut bank = Self::with_default_roots(Some(root));
        let olo1_path = root.join("cache").join("olo1_overlays.bin");
        if olo1_path.exists() {
            if let Ok(bytes) = fs::read(&olo1_path) {
                if bank.load_olo1(&bytes).is_ok() {
                    return bank;
                }
            }
        }
        let _ = bank.scan_index_from_disk();
        bank
    }

    /// Scan only `root/overlays` (no multi-root). Useful for bake fixtures.
    pub fn scan_from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let mut bank = Self::with_roots([root.to_path_buf()]);
        let _ = bank.scan_index_from_disk();
        bank
    }

    pub fn count(&self) -> usize {
        self.by_id.len()
    }

    pub fn max_id(&self) -> i32 {
        self.ids.last().copied().unwrap_or(0)
    }

    pub fn ids(&self) -> &[i32] {
        &self.ids
    }

    /// C++ `getOverlay(inID)` — index meta only (pixels via [`Self::ensure_image`]).
    pub fn get_overlay(&self, id: i32) -> Option<&OverlayRecord> {
        self.by_id.get(&id)
    }

    /// Whether pixels for `id` are already decoded.
    pub fn has_image(&self, id: i32) -> bool {
        self.images.contains_key(&id)
    }

    /// Lazy TGA decode; caches RGBA. Returns None if missing / bad file.
    pub fn ensure_image(&mut self, id: i32) -> Option<&RgbaImage> {
        if self.images.contains_key(&id) {
            return self.images.get(&id);
        }
        if self.missing_decode.contains_key(&id) {
            return None;
        }
        let path = self.resolve_path(id)?;
        match load_tga_path(&path) {
            Ok(img) => {
                if let Some(rec) = self.by_id.get_mut(&id) {
                    rec.width = img.width;
                    rec.height = img.height;
                    if rec.abs_path.is_none() {
                        rec.abs_path = Some(path);
                    }
                }
                self.images.insert(id, img);
                self.images.get(&id)
            }
            Err(_) => {
                self.missing_decode.insert(id, ());
                None
            }
        }
    }

    /// Drop cached pixels (index remains).
    pub fn free_images(&mut self) {
        self.images.clear();
        self.missing_decode.clear();
    }

    /// C++ `searchOverlays` lite: empty search → reverse-id order; else tag substring (case-insensitive).
    pub fn search_overlays(
        &self,
        search: &str,
        num_to_skip: usize,
        num_to_get: usize,
    ) -> (Vec<i32>, usize) {
        if search.is_empty() {
            let mut matches: Vec<i32> = self.ids.iter().copied().rev().collect();
            let remaining_after_skip = matches.len().saturating_sub(num_to_skip);
            let take = remaining_after_skip.min(num_to_get);
            let out: Vec<i32> = matches
                .drain(num_to_skip..num_to_skip + take)
                .collect();
            let remaining = remaining_after_skip.saturating_sub(take);
            return (out, remaining);
        }
        let q = search.to_lowercase();
        let mut matches: Vec<i32> = self
            .ids
            .iter()
            .copied()
            .filter(|id| {
                self.by_id
                    .get(id)
                    .map(|r| r.tag.to_lowercase().contains(&q))
                    .unwrap_or(false)
            })
            .collect();
        // Tree order in C++ is tag-sorted; we keep ascending id within tag filter.
        let remaining_after_skip = matches.len().saturating_sub(num_to_skip);
        let take = remaining_after_skip.min(num_to_get);
        let out: Vec<i32> = matches
            .drain(num_to_skip..num_to_skip + take)
            .collect();
        let remaining = remaining_after_skip.saturating_sub(take);
        (out, remaining)
    }

    /// Install OLO1 index (does not decode TGA).
    pub fn load_olo1(&mut self, data: &[u8]) -> Result<u32, String> {
        let (ver, entries) = load_olo1(data)?;
        self.by_id.clear();
        self.ids.clear();
        self.images.clear();
        self.missing_decode.clear();
        for e in entries {
            self.ids.push(e.id);
            self.by_id.insert(e.id, e);
        }
        self.ids.sort_unstable();
        self.ids.dedup();
        self.index_loaded = !self.by_id.is_empty();
        self.data_version = ver;
        Ok(ver)
    }

    /// Scan all roots for `overlays/{tag}/{id}.tga` → memory index (no pixels).
    pub fn scan_index_from_disk(&mut self) -> usize {
        let entries = scan_overlay_index(&self.roots);
        self.by_id.clear();
        self.ids.clear();
        for e in entries {
            self.ids.push(e.id);
            // Prefer first root hit; later roots only fill missing ids.
            self.by_id.entry(e.id).or_insert(e);
        }
        self.ids = self.by_id.keys().copied().collect();
        self.ids.sort_unstable();
        self.index_loaded = !self.by_id.is_empty();
        self.by_id.len()
    }

    /// Serialize current index (or re-scan) to OLO1 bytes.
    pub fn write_olo1(&self, data_version: u32) -> Vec<u8> {
        let mut entries: Vec<OverlayRecord> = self.by_id.values().cloned().collect();
        if entries.is_empty() {
            entries = scan_overlay_index(&self.roots);
        }
        write_olo1(&entries, data_version)
    }

    fn resolve_path(&self, id: i32) -> Option<PathBuf> {
        let rec = self.by_id.get(&id)?;
        if let Some(abs) = &rec.abs_path {
            if abs.exists() {
                return Some(abs.clone());
            }
        }
        if !rec.rel_path.is_empty() {
            let p = PathBuf::from(&rec.rel_path);
            if p.is_absolute() && p.exists() {
                return Some(p);
            }
            for root in &self.roots {
                let cand = root.join(&rec.rel_path);
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
        // Fallback: re-derive overlays/{tag}/{id}.tga
        if !rec.tag.is_empty() {
            let rel = format!("overlays/{}/{}.tga", rec.tag, id);
            for root in &self.roots {
                let cand = root.join(&rel);
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
        None
    }
}

impl Default for OverlayBank {
    fn default() -> Self {
        Self::new()
    }
}

// ── OLO1 binary ──────────────────────────────────────────────────────────────

/// Write OLO1 bytes from index entries.
pub fn write_olo1(entries: &[OverlayRecord], data_version: u32) -> Vec<u8> {
    let mut list = entries.to_vec();
    list.sort_by_key(|e| e.id);

    let mut out = Vec::with_capacity(24 + list.len() * 40);
    out.extend_from_slice(OLO1_MAGIC);
    out.extend_from_slice(&OLO1_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&data_version.to_le_bytes());
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes()); // header_crc32

    for e in &list {
        out.extend_from_slice(&e.id.to_le_bytes());
        push_str_u16(&mut out, &e.tag);
        push_str_u16(&mut out, &e.rel_path);
        out.extend_from_slice(&e.width.to_le_bytes());
        out.extend_from_slice(&e.height.to_le_bytes());
    }
    out
}

/// Parse OLO1 → (data_version, entries).
pub fn load_olo1(data: &[u8]) -> Result<(u32, Vec<OverlayRecord>), String> {
    if data.len() < 24 {
        return Err("OLO1 too short".into());
    }
    if &data[0..4] != OLO1_MAGIC {
        return Err("bad OLO1 magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLO1_FORMAT_VERSION {
        return Err(format!("unsupported OLO1 format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let mut off = 24usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if off + 4 > data.len() {
            return Err("OLO1 truncated id".into());
        }
        let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let tag = read_str_u16(data, &mut off)?;
        let rel_path = read_str_u16(data, &mut off)?;
        if off + 8 > data.len() {
            return Err("OLO1 truncated dims".into());
        }
        let width = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let height = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        entries.push(OverlayRecord {
            id,
            tag,
            rel_path,
            abs_path: None,
            width,
            height,
        });
    }
    Ok((data_version, entries))
}

/// Bake OLO1 from `src/overlays` (falls back to default roots only if src has none).
/// Returns (bytes, count).
pub fn bake_olo1_from_root(src: impl AsRef<Path>, data_version: u32) -> (Vec<u8>, usize) {
    let src = src.as_ref();
    let mut roots = vec![src.to_path_buf()];
    let local = scan_overlay_index(&roots);
    let entries = if local.is_empty() {
        // No overlays under src — try default content roots (play data).
        for r in default_overlay_roots(Some(src)) {
            if r != src {
                roots.push(r);
            }
        }
        scan_overlay_index(&roots)
    } else {
        local
    };
    // Dedup by id (first wins).
    let mut by_id: HashMap<i32, OverlayRecord> = HashMap::new();
    for e in entries {
        by_id.entry(e.id).or_insert(e);
    }
    let list: Vec<OverlayRecord> = by_id.into_values().collect();
    let n = list.len();
    (write_olo1(&list, data_version), n)
}

/// Write `out_dir/olo1_overlays.bin`.
pub fn bake_olo1_to_dir(
    src: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
    data_version: u32,
) -> Result<(usize, usize), String> {
    let (bytes, count) = bake_olo1_from_root(src, data_version);
    fs::create_dir_all(out_dir.as_ref()).map_err(|e| e.to_string())?;
    let path = out_dir.as_ref().join("olo1_overlays.bin");
    fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok((bytes.len(), count))
}

/// Default search roots for overlays folder.
pub fn default_overlay_roots(content_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(p) = std::env::var("OHOL_CONTENT_DIR") {
        if !p.is_empty() {
            roots.push(PathBuf::from(p));
        }
    }
    if let Some(c) = content_root {
        roots.push(c.to_path_buf());
        if let Some(parent) = c.parent() {
            roots.push(parent.to_path_buf());
            if let Some(gp) = parent.parent() {
                roots.push(gp.join("OneLifeData7"));
            }
        }
    }
    roots.push(PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"));
    roots.push(PathBuf::from(
        r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
    ));
    let mut seen = HashMap::new();
    roots
        .into_iter()
        .filter(|r| seen.insert(r.clone(), ()).is_none())
        .collect()
}

/// Scan roots for `overlays/{tag}/{id}.tga`.
pub fn scan_overlay_index(roots: &[PathBuf]) -> Vec<OverlayRecord> {
    let mut out = Vec::new();
    let mut seen_ids: HashMap<i32, ()> = HashMap::new();

    for root in roots {
        let overlays_dir = root.join("overlays");
        if !overlays_dir.is_dir() {
            continue;
        }
        let Ok(tag_dirs) = fs::read_dir(&overlays_dir) else {
            continue;
        };
        for tag_ent in tag_dirs.flatten() {
            let tag_path = tag_ent.path();
            if !tag_path.is_dir() {
                continue;
            }
            let tag = tag_ent.file_name().to_string_lossy().into_owned();
            // skip hidden / special
            if tag.starts_with('.') {
                continue;
            }
            let Ok(files) = fs::read_dir(&tag_path) else {
                continue;
            };
            for f in files.flatten() {
                let fp = f.path();
                if fp.is_dir() {
                    continue;
                }
                let name = f.file_name().to_string_lossy().into_owned();
                let lower = name.to_lowercase();
                if !lower.ends_with(".tga") {
                    continue;
                }
                let stem = name.trim_end_matches(".tga").trim_end_matches(".TGA");
                let Ok(id) = stem.parse::<i32>() else {
                    continue;
                };
                if id <= 0 || seen_ids.contains_key(&id) {
                    continue;
                }
                seen_ids.insert(id, ());
                let rel_path = format!("overlays/{tag}/{id}.tga");
                out.push(OverlayRecord {
                    id,
                    tag: tag.clone(),
                    rel_path,
                    abs_path: Some(fp),
                    width: 0,
                    height: 0,
                });
            }
        }
    }
    out.sort_by_key(|e| e.id);
    out
}

fn push_str_u16(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let len = (b.len().min(u16::MAX as usize)) as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&b[..len as usize]);
}

fn read_str_u16(data: &[u8], off: &mut usize) -> Result<String, String> {
    if *off + 2 > data.len() {
        return Err("OLO1 truncated string len".into());
    }
    let len = u16::from_le_bytes(data[*off..*off + 2].try_into().unwrap()) as usize;
    *off += 2;
    if *off + len > data.len() {
        return Err("OLO1 truncated string".into());
    }
    let s = String::from_utf8_lossy(&data[*off..*off + len]).into_owned();
    *off += len;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_root() -> PathBuf {
        PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7")
    }

    #[test]
    fn scan_real_overlays_if_present() {
        let root = content_root();
        if !root.join("overlays").is_dir() {
            return;
        }
        let bank = OverlayBank::scan_from_root(&root);
        assert!(bank.count() >= 1, "expected at least one overlay TGA");
        let rec = bank.get_overlay(6);
        if let Some(r) = rec {
            assert_eq!(r.tag, "Dots");
            assert!(r.rel_path.contains("Dots"));
            assert!(r.rel_path.ends_with("6.tga"));
        }
    }

    #[test]
    fn lazy_tga_decode_if_present() {
        let root = content_root();
        let tga = root.join("overlays").join("Dots").join("6.tga");
        if !tga.exists() {
            return;
        }
        let mut bank = OverlayBank::scan_from_root(&root);
        assert!(!bank.has_image(6));
        let (w, h, px_len) = {
            let img = bank.ensure_image(6).expect("decode overlay 6");
            (img.width, img.height, img.pixels.len())
        };
        assert!(w > 0 && h > 0);
        assert_eq!(px_len, (w * h * 4) as usize);
        assert!(bank.has_image(6));
        // second call hits cache
        let w2 = bank.ensure_image(6).unwrap().width;
        assert_eq!(w2, w);
        let rec = bank.get_overlay(6).unwrap();
        assert_eq!(rec.width, w);
        assert_eq!(rec.height, h);
    }

    #[test]
    fn olo1_roundtrip() {
        let entries = vec![
            OverlayRecord {
                id: 1,
                tag: "Alpha".into(),
                rel_path: "overlays/Alpha/1.tga".into(),
                abs_path: None,
                width: 16,
                height: 32,
            },
            OverlayRecord {
                id: 3,
                tag: "Beta".into(),
                rel_path: "overlays/Beta/3.tga".into(),
                abs_path: None,
                width: 0,
                height: 0,
            },
        ];
        let bytes = write_olo1(&entries, 42);
        assert_eq!(&bytes[0..4], OLO1_MAGIC);
        let (ver, loaded) = load_olo1(&bytes).unwrap();
        assert_eq!(ver, 42);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, 1);
        assert_eq!(loaded[0].tag, "Alpha");
        assert_eq!(loaded[0].width, 16);
        assert_eq!(loaded[1].id, 3);
        assert_eq!(loaded[1].tag, "Beta");

        let mut bank = OverlayBank::new();
        bank.load_olo1(&bytes).unwrap();
        assert_eq!(bank.count(), 2);
        assert_eq!(bank.get_overlay(1).unwrap().tag, "Alpha");
        assert!(bank.get_overlay(2).is_none());
        assert_eq!(bank.data_version, 42);
    }

    #[test]
    fn search_empty_and_tag() {
        let mut bank = OverlayBank::new();
        for (id, tag) in [(1, "Dots"), (2, "Dots"), (5, "Stars")] {
            bank.by_id.insert(
                id,
                OverlayRecord {
                    id,
                    tag: tag.into(),
                    rel_path: format!("overlays/{tag}/{id}.tga"),
                    abs_path: None,
                    width: 0,
                    height: 0,
                },
            );
            bank.ids.push(id);
        }
        bank.ids.sort_unstable();
        bank.index_loaded = true;

        let (ids, rem) = bank.search_overlays("", 0, 10);
        assert_eq!(ids, vec![5, 2, 1]); // reverse id
        assert_eq!(rem, 0);

        let (ids, rem) = bank.search_overlays("", 1, 1);
        assert_eq!(ids, vec![2]);
        assert_eq!(rem, 1);

        let (ids, _) = bank.search_overlays("dot", 0, 10);
        assert_eq!(ids, vec![1, 2]);
        let (ids, _) = bank.search_overlays("STAR", 0, 10);
        assert_eq!(ids, vec![5]);
    }

    #[test]
    fn scan_fixture_tmp() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_ovl_scan_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        let tag_dir = tmp.join("overlays").join("TestTag");
        fs::create_dir_all(&tag_dir).unwrap();
        // minimal valid 1x1 TGA (uncompressed 32-bit, top-left origin)
        let mut tga = vec![0u8; 18];
        tga[2] = 2; // true-color
        tga[12] = 1;
        tga[13] = 0; // w=1
        tga[14] = 1;
        tga[15] = 0; // h=1
        tga[16] = 32;
        tga[17] = 0x20; // top origin
        tga.extend_from_slice(&[10, 20, 30, 255]); // BGRA
        fs::write(tag_dir.join("42.tga"), &tga).unwrap();
        fs::write(tmp.join("overlays").join("nextOverlayNumber.txt"), "43\n").unwrap();

        let mut bank = OverlayBank::scan_from_root(&tmp);
        assert_eq!(bank.count(), 1);
        let r = bank.get_overlay(42).unwrap();
        assert_eq!(r.tag, "TestTag");
        let img = bank.ensure_image(42).expect("decode fixture");
        assert_eq!((img.width, img.height), (1, 1));
        assert_eq!(img.pixel(0, 0), [30, 20, 10, 255]);

        let (bytes, n) = bake_olo1_from_root(&tmp, 7);
        assert_eq!(n, 1);
        let mut bank2 = OverlayBank::with_roots([tmp.clone()]);
        bank2.load_olo1(&bytes).unwrap();
        assert_eq!(bank2.get_overlay(42).unwrap().tag, "TestTag");
        // OLO1 load has no abs_path but resolve via roots works
        let img2 = bank2.ensure_image(42).expect("decode via OLO1 path");
        assert_eq!(img2.pixel(0, 0), [30, 20, 10, 255]);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn bad_olo1_magic() {
        assert!(load_olo1(b"XXXX....................").is_err());
    }
}
