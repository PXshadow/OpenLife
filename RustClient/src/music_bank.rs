//! Music bed: lazy index of `music/music_NN.ogg` + optional OGG decode / device play.
//!
//! // C++: `musicPlayer2.cpp` (age-block OGG) · Haxe: `Resource.music(id)` → `music/music_NN.ogg`
//!
//! - **Boot:** directory scan only — **zero** OGG opens / Vorbis decode (`ogg_opens == 0`).
//! - **Index:** `music/music_NN.ogg` → five-year age block `NN` (1, 2, … 12, 99, …).
//! - **Decode:** first [`MusicBank::ensure`] opens the file and decodes via **lewton**
//!   (pure-Rust Vorbis). Stereo is mixed to mono for the existing SFX mixer path.
//! - **Play:** headless records [`MusicBank::last_played`]; with `--features audio`,
//!   queues mono PCM on the cpal device (same soft-fail / `OHOL_AUDIO_DISABLE` path
//!   as SFX). Device open is still lazy on first play.
//!
//! Age selection mirrors C++ `startNextAgeFileRead`: next 5-year boundary after `age`,
//! with a near-boundary skip when within ~60s of aging into the block.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};



// ── Age block helpers (C++ musicPlayer2) ──────────────────────────────────────

/// C++ `startNextAgeFileRead` — five-year music block for current age.
///
/// `ceil(age/5)`, then +1 if that boundary is within `60 * age_rate` years
/// (about one minute of wall time at normal aging). Floor at 1.
pub fn next_music_block(age: f64, age_rate: f64) -> u32 {
    let mut next = (age / 5.0).ceil() as i64;
    if next < 1 {
        next = 1;
    }
    // too close to that age transition → start on next
    if (next as f64) * 5.0 < age + 60.0 * age_rate {
        next += 1;
    }
    if next < 1 {
        next = 1;
    }
    next as u32
}

/// Content-relative path for block `NN` (`music/music_01.ogg`, …).
///
/// // Haxe Resource.music: pad single digit with leading zero
pub fn music_rel_path(block: u32) -> String {
    format!("music/music_{block:02}.ogg")
}

/// Parse `music_NN.ogg` / `music_N.ogg` stem → block number.
pub fn parse_music_filename(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(".ogg")?;
    let num = stem.strip_prefix("music_")?;
    let n: u32 = num.parse().ok()?;
    if n == 0 {
        return None;
    }
    Some(n)
}

// ── Index + PCM ──────────────────────────────────────────────────────────────

/// One music index record (no PCM).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicIndexEntry {
    /// Five-year block id (`01` → 1, `99` → 99).
    pub block: u32,
    /// Relative to content root, e.g. `music/music_01.ogg`.
    pub rel_path: String,
    /// File size from metadata only (0 if unknown).
    pub file_size: u64,
}

/// Decoded mono music PCM (stereo OGG mixed down).
#[derive(Debug, Clone)]
pub struct MusicPcm {
    pub block: u32,
    pub sample_rate: u32,
    /// Mono i16 samples (host endian).
    pub samples: Vec<i16>,
    /// Source channel count before mixdown.
    pub source_channels: u16,
}

impl MusicPcm {
    pub fn num_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.sample_rate as f64
    }
}

/// Music bank: index at boot, lazy OGG decode on ensure/play.
pub struct MusicBank {
    root: PathBuf,
    /// block → index entry
    pub index: HashMap<u32, MusicIndexEntry>,
    /// Decoded mono PCM cache
    pcm: HashMap<u32, MusicPcm>,
    /// Missing / bad OGG blocks
    missing: HashMap<u32, ()>,
    /// True when scan filled the index (may be empty dir).
    pub index_loaded: bool,
    /// Count of OGG files opened for full decode (bench: must be 0 at boot).
    pub ogg_opens: u64,
    /// C++ `musicLoudness` target 0..1 (applied on play).
    pub loudness: f32,
    /// Last block played (headless / unit-test log).
    pub last_played: Vec<u32>,
    last_played_cap: usize,
    /// Whether music bed is considered started (C++ `musicStarted`).
    pub started: bool,
    /// Current age / rate after [`Self::restart_music`].
    pub age: f64,
    pub age_rate: f64,
    /// Block selected for the current journey segment.
    pub current_block: Option<u32>,
    /// P5#39 settings: when true, [`Self::play_block`] no-ops.
    pub muted: bool,
}

impl MusicBank {
    pub fn new(content_root: impl AsRef<Path>) -> Self {
        Self {
            root: content_root.as_ref().to_path_buf(),
            index: HashMap::new(),
            pcm: HashMap::new(),
            missing: HashMap::new(),
            index_loaded: false,
            ogg_opens: 0,
            loudness: 1.0,
            last_played: Vec::new(),
            last_played_cap: 16,
            started: false,
            age: 0.0,
            age_rate: 0.0,
            current_block: None,
            muted: false,
        }
    }

    /// P5#39 — music mute from settings page.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if muted {
            self.started = false;
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn pcm_count(&self) -> usize {
        self.pcm.len()
    }

    pub fn is_missing(&self, block: u32) -> bool {
        self.missing.contains_key(&block)
    }

    pub fn get_index(&self, block: u32) -> Option<&MusicIndexEntry> {
        self.index.get(&block)
    }

    pub fn get_pcm(&self, block: u32) -> Option<&MusicPcm> {
        self.pcm.get(&block)
    }

    /// Scan `music/*.ogg` into index (no open/decode). Returns entry count.
    pub fn scan_index_from_disk(&mut self) -> usize {
        let entries = scan_music_dir(&self.root);
        self.index.clear();
        for e in entries {
            self.index.insert(e.block, e);
        }
        self.index_loaded = true;
        self.index.len()
    }

    /// Prefer scan of `music/` (no cache blob for v1). Boot stays decode-free.
    pub fn load_prefer_scan(content_root: impl AsRef<Path>) -> Self {
        Self::load_prefer_scan_with_progress(content_root, None)
    }

    /// Same as [`Self::load_prefer_scan`] with optional P5#36 progress callback.
    pub fn load_prefer_scan_with_progress(
        content_root: impl AsRef<Path>,
        mut on_progress: crate::load_progress::ProgressCb<'_>,
    ) -> Self {
        use crate::load_progress::{report_stage, LoadStage};
        report_stage(
            LoadStage::Music,
            0.0,
            Some("scan"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let mut bank = Self::new(content_root);
        let _ = bank.scan_index_from_disk();
        report_stage(
            LoadStage::Music,
            1.0,
            Some("index"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        bank
    }

    /// Alias for play-path naming consistency (`load_prefer_cache` style).
    pub fn load_prefer_cache(content_root: impl AsRef<Path>) -> Self {
        Self::load_prefer_scan(content_root)
    }

    /// Progress-aware alias for [`Self::load_prefer_scan_with_progress`].
    pub fn load_prefer_cache_with_progress(
        content_root: impl AsRef<Path>,
        on_progress: crate::load_progress::ProgressCb<'_>,
    ) -> Self {
        Self::load_prefer_scan_with_progress(content_root, on_progress)
    }

    /// C++ `setMusicLoudness` (0..1).
    pub fn set_loudness(&mut self, loudness: f32) {
        self.loudness = loudness.clamp(0.0, 1.0);
    }

    /// C++ `instantStopMusic`.
    pub fn stop(&mut self) {
        self.started = false;
        self.current_block = None;
    }

    /// C++ `restartMusic(age, ageRate, forceNow)` — select age block; optionally
    /// force immediate decode+play of the matching bed.
    ///
    /// Without `force_now`, only records selection (async load residual). With
    /// `force_now`, calls [`Self::play_block`] for the selected block.
    pub fn restart_music(&mut self, age: f64, age_rate: f64, force_now: bool) -> Option<u32> {
        self.age = age;
        self.age_rate = age_rate;
        self.started = true;
        let block = next_music_block(age, age_rate);
        self.current_block = Some(block);
        if force_now {
            if self.play_block(block) {
                return Some(block);
            }
            return None;
        }
        Some(block)
    }

    /// Play bed for age (force now). Returns block on successful fire.
    pub fn play_for_age(&mut self, age: f64, age_rate: f64) -> Option<u32> {
        self.restart_music(age, age_rate, true)
    }

    /// Ensure OGG is decoded; returns mono PCM or None (missing / bad).
    pub fn ensure(&mut self, block: u32) -> Option<&MusicPcm> {
        if block == 0 {
            return None;
        }
        if self.pcm.contains_key(&block) {
            return self.pcm.get(&block);
        }
        if self.missing.contains_key(&block) {
            return None;
        }

        let rel = if let Some(ent) = self.index.get(&block) {
            ent.rel_path.clone()
        } else {
            music_rel_path(block)
        };
        let path = self.root.join(&rel);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                self.missing.insert(block, ());
                return None;
            }
        };
        self.ogg_opens = self.ogg_opens.saturating_add(1);

        match decode_ogg_vorbis_mono(&bytes) {
            Ok(mut pcm) => {
                pcm.block = block;
                // Refresh / insert index meta.
                let size = bytes.len() as u64;
                self.index
                    .entry(block)
                    .and_modify(|e| e.file_size = size)
                    .or_insert(MusicIndexEntry {
                        block,
                        rel_path: rel,
                        file_size: size,
                    });
                self.pcm.insert(block, pcm);
                self.pcm.get(&block)
            }
            Err(_) => {
                self.missing.insert(block, ());
                None
            }
        }
    }

    /// Inject pre-decoded PCM (unit tests — no disk / lewton).
    pub fn ensure_pcm(&mut self, block: u32, sample_rate: u32, samples: Vec<i16>) -> Option<&MusicPcm> {
        if block == 0 {
            return None;
        }
        if self.pcm.contains_key(&block) {
            return self.pcm.get(&block);
        }
        if sample_rate == 0 || samples.is_empty() {
            self.missing.insert(block, ());
            return None;
        }
        self.pcm.insert(
            block,
            MusicPcm {
                block,
                sample_rate,
                samples,
                source_channels: 1,
            },
        );
        if !self.index.contains_key(&block) {
            self.index.insert(
                block,
                MusicIndexEntry {
                    block,
                    rel_path: music_rel_path(block),
                    file_size: 0,
                },
            );
        }
        self.pcm.get(&block)
    }

    /// Play music bed for `block` at [`Self::loudness`].
    ///
    /// - Without `audio`: ensures decode (or synthetic PCM); returns true if PCM exists.
    /// - With `audio`: queues mono PCM on the device (soft-fail silent host still true).
    /// - When [`crate::sound_bank::music_muted`] (P5#39 Settings / C++ `musicOff`), skips
    ///   device queue but still records `last_played` for headless tests.
    pub fn play_block(&mut self, block: u32) -> bool {
        if self.muted || block == 0 {
            return false;
        }
        let vol = self.loudness.clamp(0.0, 1.0);
        let (samples, rate) = {
            let Some(p) = self.ensure(block) else {
                return false;
            };
            (p.samples.clone(), p.sample_rate)
        };
        #[cfg(feature = "audio")]
        {
            if !crate::sound_bank::music_muted() {
                // Device path soft-fails (no device / OHOL_AUDIO_DISABLE) but still true.
                let _ = crate::sound_bank::play_pcm_samples(&samples, rate, vol);
            }
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = (samples, rate, vol);
        }
        self.record_played(block);
        self.current_block = Some(block);
        self.started = true;
        true
    }

    fn record_played(&mut self, block: u32) {
        self.last_played.push(block);
        while self.last_played.len() > self.last_played_cap {
            self.last_played.remove(0);
        }
    }

    pub fn clear_last_played(&mut self) {
        self.last_played.clear();
    }
}

impl Default for MusicBank {
    fn default() -> Self {
        Self::new(".")
    }
}

// ── Scan ─────────────────────────────────────────────────────────────────────

/// Scan `$root/music/music_*.ogg` (metadata only — no file open for payload).
pub fn scan_music_dir(root: &Path) -> Vec<MusicIndexEntry> {
    let dir = root.join("music");
    let mut entries = Vec::new();
    let rd = match fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return entries,
    };
    for ent in rd.flatten() {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        let Some(block) = parse_music_filename(name) else {
            continue;
        };
        let file_size = ent.metadata().map(|m| m.len()).unwrap_or(0);
        entries.push(MusicIndexEntry {
            block,
            rel_path: format!("music/{name}"),
            file_size,
        });
    }
    entries.sort_by_key(|e| e.block);
    entries
}

// ── OGG Vorbis decode (lewton) ───────────────────────────────────────────────

/// Decode OGG Vorbis bytes to mono i16 PCM (stereo → average mixdown).
///
/// Uses **lewton** (pure-Rust Vorbis). Not called at boot — only on ensure/play.
pub fn decode_ogg_vorbis_mono(data: &[u8]) -> Result<MusicPcm, String> {
    use lewton::inside_ogg::OggStreamReader;

    let cursor = Cursor::new(data);
    let mut reader =
        OggStreamReader::new(cursor).map_err(|e| format!("ogg/vorbis open: {e}"))?;
    let channels = reader.ident_hdr.audio_channels as usize;
    if channels == 0 {
        return Err("ogg zero channels".into());
    }
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if sample_rate == 0 {
        return Err("ogg zero sample rate".into());
    }

    let mut samples: Vec<i16> = Vec::new();
    loop {
        match reader.read_dec_packet_itl() {
            Ok(Some(pck)) => {
                if channels == 1 {
                    samples.extend_from_slice(&pck);
                } else {
                    for frame in pck.chunks(channels) {
                        if frame.len() < channels {
                            break;
                        }
                        let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                        samples.push((sum / channels as i32) as i16);
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                // Truncated / trailing garbage: keep what we have if any.
                if samples.is_empty() {
                    return Err(format!("ogg decode: {e}"));
                }
                break;
            }
        }
    }
    if samples.is_empty() {
        return Err("ogg empty pcm".into());
    }
    Ok(MusicPcm {
        block: 0,
        sample_rate,
        samples,
        source_channels: channels as u16,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_N: AtomicU64 = AtomicU64::new(0);

    fn tmp_root() -> PathBuf {
        let n = TMP_N.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ohol_music_bank_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("music")).unwrap();
        p
    }

    #[test]
    fn music_rel_path_zero_pads() {
        assert_eq!(music_rel_path(1), "music/music_01.ogg");
        assert_eq!(music_rel_path(12), "music/music_12.ogg");
        assert_eq!(music_rel_path(99), "music/music_99.ogg");
    }

    #[test]
    fn parse_music_filename_ok() {
        assert_eq!(parse_music_filename("music_01.ogg"), Some(1));
        assert_eq!(parse_music_filename("music_12.OGG"), Some(12));
        assert_eq!(parse_music_filename("music_99.ogg"), Some(99));
        assert_eq!(parse_music_filename("music_1.ogg"), Some(1));
        assert_eq!(parse_music_filename("not_music.ogg"), None);
        assert_eq!(parse_music_filename("music_00.ogg"), None);
        assert_eq!(parse_music_filename("621.ogg"), None);
    }

    #[test]
    fn next_music_block_age_edges() {
        // age 0, normal rate → block 1 (after near-boundary bump from 0)
        assert_eq!(next_music_block(0.0, 0.05), 1);
        // mid-block 7 years → ceil(7/5)=2
        assert_eq!(next_music_block(7.0, 0.0), 2);
        // exactly on boundary 5.0 → ceil=1; with rate 0 stays 1
        assert_eq!(next_music_block(5.0, 0.0), 1);
        // near boundary: age 4.9, rate high enough to bump past 5
        // 1*5=5 < 4.9 + 60*0.05(=3) → 5 < 7.9 → bump to 2
        assert_eq!(next_music_block(4.9, 0.05), 2);
    }

    #[test]
    fn scan_index_no_ogg_open() {
        let root = tmp_root();
        // Touch empty placeholder files (scan uses metadata only).
        fs::write(root.join("music/music_01.ogg"), b"").unwrap();
        fs::write(root.join("music/music_02.ogg"), b"").unwrap();
        fs::write(root.join("music/readme.txt"), b"nope").unwrap();

        let mut bank = MusicBank::new(&root);
        assert_eq!(bank.ogg_opens, 0);
        let n = bank.scan_index_from_disk();
        assert_eq!(n, 2);
        assert_eq!(bank.ogg_opens, 0, "boot scan must not open OGG payload");
        assert!(bank.get_index(1).is_some());
        assert!(bank.get_index(2).is_some());
        assert_eq!(bank.get_index(1).unwrap().rel_path, "music/music_01.ogg");
        // empty file → ensure fails, still counts open
        assert!(bank.ensure(1).is_none());
        assert_eq!(bank.ogg_opens, 1);
        assert!(bank.is_missing(1));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn synthetic_pcm_play_without_disk() {
        let mut bank = MusicBank::new(".");
        assert_eq!(bank.ogg_opens, 0);
        let samples: Vec<i16> = (0..256).map(|i| (i * 17) as i16).collect();
        assert!(bank.ensure_pcm(3, 22050, samples).is_some());
        assert_eq!(bank.ogg_opens, 0, "synthetic inject must not open OGG");
        assert!(bank.play_block(3));
        assert_eq!(bank.last_played, vec![3]);
        assert_eq!(bank.current_block, Some(3));
        assert!(bank.started);
        bank.stop();
        assert!(!bank.started);
        assert!(bank.current_block.is_none());
    }

    #[test]
    fn restart_music_selects_block_without_force() {
        let mut bank = MusicBank::new(".");
        let b = bank.restart_music(12.0, 0.0, false).unwrap();
        assert_eq!(b, 3); // ceil(12/5)=3
        assert!(bank.started);
        assert_eq!(bank.current_block, Some(3));
        assert!(bank.last_played.is_empty(), "no force → no play");
    }

    #[test]
    fn restart_music_force_plays_synthetic() {
        let mut bank = MusicBank::new(".");
        let _ = bank.ensure_pcm(1, 44100, vec![0i16; 64]);
        let b = bank.restart_music(0.0, 0.05, true).unwrap();
        assert_eq!(b, 1);
        assert_eq!(bank.last_played, vec![1]);
    }

    #[test]
    fn load_prefer_scan_empty_dir() {
        let root = tmp_root();
        let bank = MusicBank::load_prefer_scan(&root);
        assert!(bank.index_loaded);
        assert_eq!(bank.len(), 0);
        assert_eq!(bank.ogg_opens, 0);
        let _ = fs::remove_dir_all(&root);
    }

    /// Real content tree: decode first music_*.ogg if present; else skip.
    #[test]
    fn real_music_ogg_decode_if_present() {
        let candidates = [
            PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
            PathBuf::from("../OneLifeData7"),
            PathBuf::from("OneLifeData7"),
        ];
        let root = candidates.into_iter().find(|p| p.join("music").is_dir());
        let Some(root) = root else {
            eprintln!("skip real_music_ogg_decode_if_present: no music dir");
            return;
        };
        let mut bank = MusicBank::load_prefer_scan(&root);
        assert_eq!(bank.ogg_opens, 0, "boot free of full decode");
        if bank.is_empty() {
            eprintln!("skip: music/ scanned empty at {}", root.display());
            return;
        }
        // Prefer block 1, else any indexed block.
        let block = if bank.get_index(1).is_some() {
            1
        } else {
            *bank.index.keys().next().unwrap()
        };
        let pcm = bank.ensure(block);
        if pcm.is_none() {
            eprintln!("skip: ensure({block}) failed (missing/corrupt ogg)");
            return;
        }
        assert!(bank.ogg_opens >= 1);
        let p = bank.get_pcm(block).unwrap();
        assert!(p.sample_rate >= 8000);
        assert!(!p.samples.is_empty());
        assert!(p.duration_secs() > 0.0);
        // Second ensure is cache hit — no extra open.
        let opens = bank.ogg_opens;
        assert!(bank.ensure(block).is_some());
        assert_eq!(bank.ogg_opens, opens);
    }
}
