//! Sound bank: OLSN index + lazy mono-16 AIFF decode (C++ `soundBank` / Haxe `Sound.hx`).
//!
//! - **OLSN** (`cache/olsn_sounds.bin`): index-only id→path/rate/samples (~15KB).
//! - Boot loads the HashMap only — **zero** AIFF opens.
//! - First [`SoundBank::ensure`]: open `sounds/{id}.aiff`, decode BE→LE i16 from byte 54.
//! - OGG entries are indexed but `ensure` returns `None` (v1 — no vorbis).
//! - Playback: default build logs via [`SoundBank::last_played`] (no device crate).
//!   With `--features audio`, first play lazily opens a cpal output stream (soft-fail
//!   if no device); set `OHOL_AUDIO_DISABLE=1` to force the log-only path.
//!
//! // C++: soundBank.cpp + SoundUsage.cpp · Haxe: Sound.hx + Resource.sound

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Runtime SFX mute (P5#39 Settings). Independent of `OHOL_AUDIO_DISABLE` env
/// (env still forces device silence). Headless play logs respect this flag.
static SFX_MUTED: AtomicBool = AtomicBool::new(false);
/// Runtime music-bed mute (P5#39 Settings / C++ `musicOff`).
static MUSIC_MUTED: AtomicBool = AtomicBool::new(false);

/// Mute / unmute sound-effect device + headless play log (Settings page).
pub fn set_sfx_muted(muted: bool) {
    SFX_MUTED.store(muted, Ordering::Relaxed);
}

/// True when SFX are muted via Settings (not env alone).
pub fn sfx_muted() -> bool {
    SFX_MUTED.load(Ordering::Relaxed)
}

/// Mute / unmute music bed (Settings page / C++ `musicOff.ini`).
pub fn set_music_muted(muted: bool) {
    MUSIC_MUTED.store(muted, Ordering::Relaxed);
}

/// True when music is muted via Settings.
pub fn music_muted() -> bool {
    MUSIC_MUTED.load(Ordering::Relaxed)
}

/// Prefill mute from `OHOL_AUDIO_DISABLE` (any value = muted). Does not write the atomic.
pub fn audio_disable_env_set() -> bool {
    std::env::var_os("OHOL_AUDIO_DISABLE").is_some()
}

/// OLSN magic — sound index cache (no PCM).
pub const OLSN_MAGIC: &[u8; 4] = b"OLSN";
/// OLSN format version.
pub const OLSN_FORMAT_VERSION: u32 = 1;

/// Haxe `Sound.hx` PCM payload starts at this byte (fixed mono-16 AIFF layout).
pub const AIFF_SAMPLE_START: usize = 54;

/// C++ `LivingLifePage` `mCurseSound` path (`loadSoundSprite("otherSounds", "curseChime.aiff")`).
pub const CURSE_CHIME_REL: &str = "otherSounds/curseChime.aiff";
/// C++ play volume tweak for successful PS isCurse (`playSound(mCurseSound, 0.5, …)`).
pub const CURSE_CHIME_VOLUME: f32 = 0.5;
/// C++ `mHungerSound` (`loadSoundSprite("otherSounds", "hunger.aiff")`).
pub const HUNGER_SOUND_REL: &str = "otherSounds/hunger.aiff";
/// C++ hunger.aiff center pan volume (SFX loudness; soft-FB 1.0).
pub const HUNGER_SOUND_VOLUME: f32 = 1.0;

// OLSN entry flags.
/// Header peeked / mono+16-bit checks passed at bake.
pub const OLSN_F_MONO16_VERIFIED: u32 = 1 << 0;
/// File is `.ogg` (index only; ensure returns None).
pub const OLSN_F_IS_OGG: u32 = 1 << 1;
/// 54-byte AIFF header was peeked at bake (rate/samples may be set).
pub const OLSN_F_HEADER_PEEKED: u32 = 1 << 2;

// ── SoundUsage (C++ SoundUsage.cpp) ──────────────────────────────────────────

/// One sub-sound in a [`SoundUsage`] (id + volume 0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundUsagePlay {
    pub id: i32,
    pub volume: f32,
}

/// C++ `SoundUsage` — `id:vol#id:vol#…` (blank = empty / `-1:0`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SoundUsage {
    pub subs: Vec<SoundUsagePlay>,
}

impl SoundUsage {
    pub fn blank() -> Self {
        Self { subs: Vec::new() }
    }

    pub fn is_blank(&self) -> bool {
        self.subs.is_empty()
            || (self.subs.len() == 1 && self.subs[0].id == -1)
    }

    /// C++ `scanSoundUsage` — format `id:vol#id:vol` (vol defaults to 1.0).
    pub fn parse(s: &str) -> Self {
        parse_sound_usage(s)
    }

    /// C++ `printSoundUsage` — empty → `-1:0.0`.
    pub fn print(&self) -> String {
        if self.is_blank() {
            return "-1:0.0".into();
        }
        let mut out = String::new();
        for (i, p) in self.subs.iter().enumerate() {
            if i > 0 {
                out.push('#');
            }
            out.push_str(&format!("{}:{}", p.id, p.volume));
        }
        out
    }

    /// C++ `playRandom` — pick one sub-sound (deterministic: first when single;
    /// when multi, uses a simple LCG seeded by count for headless tests; real
    /// playback may reseed). Prefer [`Self::play_random_with`] for control.
    pub fn play_random(&self) -> Option<SoundUsagePlay> {
        self.play_random_with(simple_mix_seed(self))
    }

    /// Pick a random sub-sound using `seed` (mod length).
    pub fn play_random_with(&self, seed: u32) -> Option<SoundUsagePlay> {
        if self.is_blank() {
            return None;
        }
        let n = self.subs.len();
        let i = (seed as usize) % n;
        Some(self.subs[i])
    }

    pub fn uses_id(&self, id: i32) -> bool {
        self.subs.iter().any(|p| p.id == id)
    }
}

/// Parse `id:vol#id:vol` (C++ `scanSoundUsage`). Invalid parts skipped.
pub fn parse_sound_usage(s: &str) -> SoundUsage {
    let s = s.trim();
    if s.is_empty() {
        return SoundUsage::blank();
    }
    let mut subs = Vec::new();
    for part in s.split('#') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (id_s, vol_s) = match part.split_once(':') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (part, "1.0"),
        };
        let id: i32 = match id_s.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let vol: f32 = vol_s.parse().unwrap_or(1.0);
        if vol < 0.0 || vol > 1.0 {
            continue;
        }
        subs.push(SoundUsagePlay { id, volume: vol });
    }
    // C++: single -1 id → blank.
    if subs.len() == 1 && subs[0].id == -1 {
        return SoundUsage::blank();
    }
    SoundUsage { subs }
}

fn simple_mix_seed(u: &SoundUsage) -> u32 {
    let mut h: u32 = 4583; // C++ Jenkins seed stand-in
    for p in &u.subs {
        h = h.wrapping_mul(31).wrapping_add(p.id as u32);
        h = h.wrapping_mul(31).wrapping_add(p.volume.to_bits());
    }
    h
}

// ── Index + PCM ──────────────────────────────────────────────────────────────

/// One OLSN index record (no PCM).
#[derive(Debug, Clone)]
pub struct SoundIndexEntry {
    pub id: i32,
    pub sample_rate: u32,
    pub num_samples: u32,
    /// Relative to content root, e.g. `sounds/123.aiff`.
    pub rel_path: String,
    pub flags: u32,
}

impl SoundIndexEntry {
    pub fn is_ogg(&self) -> bool {
        self.flags & OLSN_F_IS_OGG != 0
    }
    pub fn mono16_verified(&self) -> bool {
        self.flags & OLSN_F_MONO16_VERIFIED != 0
    }
    pub fn header_peeked(&self) -> bool {
        self.flags & OLSN_F_HEADER_PEEKED != 0
    }
}

/// Decoded mono-16 PCM (little-endian host samples).
#[derive(Debug, Clone)]
pub struct PcmSound {
    pub id: i32,
    pub sample_rate: u32,
    /// Interleaved mono i16 samples (host endian).
    pub samples: Vec<i16>,
}

impl PcmSound {
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

/// One C++ `OffScreenSound` registration (headless: no HUD draw, testable log).
///
/// // C++ LivingLifePage `addOffScreenSound` ~4245 — edge arrow when source off-camera
#[derive(Debug, Clone, PartialEq)]
pub struct OffScreenSoundEvent {
    pub source_player_id: i32,
    pub map_x: f32,
    pub map_y: f32,
    /// `_red` after `offScreenSound` in object description.
    pub red: bool,
    /// Single special char after `offScreenSound_` (non-red).
    pub special_char: Option<char>,
}

/// Sound bank: OLSN index at boot, lazy AIFF on [`SoundBank::ensure`].
///
/// Path-keyed UI chimes ([`CURSE_CHIME_REL`]) also lazy-decode on first play.
pub struct SoundBank {
    root: PathBuf,
    /// id → index (path/rate/samples).
    pub index: HashMap<i32, SoundIndexEntry>,
    /// Decoded PCM cache.
    pcm: HashMap<i32, PcmSound>,
    /// Failed / missing / unsupported (OGG) ids.
    missing: HashMap<i32, ()>,
    /// Path-relative AIFF cache (e.g. `otherSounds/curseChime.aiff`) — lazy.
    path_pcm: HashMap<String, PcmSound>,
    /// Missing / bad path AIFFs.
    path_missing: HashMap<String, ()>,
    pub data_version: u32,
    /// True when OLSN (or disk scan) filled the index.
    pub index_loaded: bool,
    /// Count of AIFF files opened for decode (bench: must be 0 at boot).
    pub aiff_opens: u64,
    /// Last-played usage strings (headless / unit-test trigger log; no device).
    /// Spatial plays append `|pan=0.xxx` so pan is testable without the `audio` feature.
    pub last_played: Vec<String>,
    /// Cap on [`Self::last_played`] (ring-trim from front when exceeded).
    last_played_cap: usize,
    /// Most recent stereo pan 0..1 (0=left, 0.5=center, 1=right). Headless + audio.
    pub last_pan: f32,
    /// Listener / camera center in **world tiles** (C++ `lastScreenViewCenter / CELL_D`).
    /// Used by [`Self::play_usage_at`] / [`Self::play_id_at`] for pan + distance.
    pub listener_x: f32,
    pub listener_y: f32,
    /// C++ `offScreenSounds` queue (P2#13). Headless: last registrations for tests.
    pub last_off_screen: Vec<OffScreenSoundEvent>,
    last_off_screen_cap: usize,
    /// C++ SFX loudness 0..1 (P5#39 Settings). Applied on device play gains.
    pub loudness: f32,
    /// When true, skip device queue (decode/triggers still run for headless tests).
    pub muted: bool,
}

impl SoundBank {
    pub fn new(content_root: impl AsRef<Path>) -> Self {
        Self {
            root: content_root.as_ref().to_path_buf(),
            index: HashMap::new(),
            pcm: HashMap::new(),
            missing: HashMap::new(),
            path_pcm: HashMap::new(),
            path_missing: HashMap::new(),
            data_version: 0,
            index_loaded: false,
            aiff_opens: 0,
            last_played: Vec::new(),
            last_played_cap: 64,
            last_pan: 0.5,
            listener_x: 0.0,
            listener_y: 0.0,
            last_off_screen: Vec::new(),
            last_off_screen_cap: 32,
            loudness: 1.0,
            muted: false,
        }
    }

    /// C++ camera / listener position in tile units (screen view center).
    pub fn set_listener(&mut self, x: f32, y: f32) {
        self.listener_x = x;
        self.listener_y = y;
    }

    /// P5#39 — master SFX loudness 0..1 (Settings page).
    pub fn set_loudness(&mut self, loudness: f32) {
        self.loudness = loudness.clamp(0.0, 1.0);
    }

    /// P5#39 — mute SFX device path without disabling trigger wiring.
    ///
    /// Also mirrors to process-wide [`set_sfx_muted`] so path-keyed plays and
    /// other banks see the same setting.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        set_sfx_muted(muted);
    }

    /// Scale a pre-computed L/R gain by loudness; zeros when muted.
    ///
    /// Honors instance [`Self::muted`] **or** process-wide [`sfx_muted`] (Settings).
    fn apply_master_gains(&self, left: f32, right: f32) -> (f32, f32) {
        if self.muted || sfx_muted() {
            return (0.0, 0.0);
        }
        let m = self.loudness.clamp(0.0, 1.0);
        (left * m, right * m)
    }

    /// Clear the headless play log (does not reset listener or last_pan).
    pub fn clear_last_played(&mut self) {
        self.last_played.clear();
    }

    /// Clear off-screen sound registration log.
    pub fn clear_last_off_screen(&mut self) {
        self.last_off_screen.clear();
    }

    /// C++ `addOffScreenSound` — register edge indicator after an offScreenSound play.
    pub fn add_off_screen_sound(
        &mut self,
        source_player_id: i32,
        map_x: f32,
        map_y: f32,
        description: &str,
    ) {
        let (red, special_char) = parse_off_screen_sound_flags(description);
        self.last_off_screen.push(OffScreenSoundEvent {
            source_player_id,
            map_x,
            map_y,
            red,
            special_char,
        });
        while self.last_off_screen.len() > self.last_off_screen_cap {
            self.last_off_screen.remove(0);
        }
    }

    fn record_played(&mut self, usage: &str) {
        self.last_played.push(usage.to_string());
        while self.last_played.len() > self.last_played_cap {
            self.last_played.remove(0);
        }
    }

    /// Record usage + pan for spatial plays (headless tests assert `|pan=` suffix).
    fn record_played_spatial(&mut self, usage: &str, pan: f32) {
        self.last_pan = pan;
        self.last_played
            .push(format!("{usage}|pan={pan:.3}"));
        while self.last_played.len() > self.last_played_cap {
            self.last_played.remove(0);
        }
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

    pub fn is_missing(&self, id: i32) -> bool {
        self.missing.contains_key(&id)
    }

    pub fn get_index(&self, id: i32) -> Option<&SoundIndexEntry> {
        self.index.get(&id)
    }

    pub fn get_pcm(&self, id: i32) -> Option<&PcmSound> {
        self.pcm.get(&id)
    }

    /// Prefer `root/cache/olsn_sounds.bin`; rebake if missing/stale vs tree data_version;
    /// else scan `sounds/` into memory (still no AIFF decode).
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
            LoadStage::Sounds,
            0.0,
            Some("prefer_cache"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let root = content_root.as_ref();
        let mut bank = Self::new(root);
        let cache = root.join("cache");
        let olsn_path = cache.join("olsn_sounds.bin");
        let tree_ver = read_data_version_u32(root);

        if olsn_path.exists() {
            if let Ok(bytes) = fs::read(&olsn_path) {
                if let Ok(ver) = bank.load_olsn(&bytes) {
                    if tree_ver.map(|t| t == ver).unwrap_or(true) {
                        report_stage(
                            LoadStage::Sounds,
                            1.0,
                            Some("olsn"),
                            crate::load_progress::reborrow_cb(&mut on_progress),
                        );
                        return bank;
                    }
                    // Stale version — fall through to rebake.
                }
            }
        }

        // Rebake when sounds dir present.
        report_stage(
            LoadStage::Sounds,
            0.4,
            Some("scan_or_bake"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        if root.join("sounds").is_dir() {
            let ver = tree_ver.unwrap_or(0);
            if let Ok((bytes, _n)) = bake_olsn_from_dir(root, ver) {
                let _ = fs::create_dir_all(&cache);
                let _ = fs::write(&olsn_path, &bytes);
                // Best-effort manifest patch is done by full bake_content; here just index.
                let _ = bank.load_olsn(&bytes);
                report_stage(
                    LoadStage::Sounds,
                    1.0,
                    Some("olsn_baked"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return bank;
            }
        }

        // Scan only (no write).
        let _ = bank.scan_index_from_disk();
        report_stage(
            LoadStage::Sounds,
            1.0,
            Some("scan"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        bank
    }

    /// Install OLSN bytes (index only — no AIFF).
    pub fn load_olsn(&mut self, data: &[u8]) -> Result<u32, String> {
        let (ver, entries) = load_olsn(data)?;
        self.index.clear();
        for e in entries {
            self.index.insert(e.id, e);
        }
        self.index_loaded = true;
        self.data_version = ver;
        Ok(ver)
    }

    /// Serialize current index (or re-scan) to OLSN bytes.
    pub fn write_olsn(&self, data_version: u32) -> Vec<u8> {
        let mut entries: Vec<SoundIndexEntry> = self.index.values().cloned().collect();
        if entries.is_empty() {
            entries = scan_sounds_dir(&self.root);
        }
        write_olsn(&entries, data_version)
    }

    /// Scan `sounds/*.{aiff,ogg}` into index (no decode).
    pub fn scan_index_from_disk(&mut self) -> usize {
        let entries = scan_sounds_dir(&self.root);
        self.index.clear();
        for e in entries {
            self.index.insert(e.id, e);
        }
        self.index_loaded = !self.index.is_empty();
        self.index.len()
    }

    /// Ensure sound is decoded; returns PCM or None (missing / OGG / bad AIFF).
    pub fn ensure(&mut self, id: i32) -> Option<&PcmSound> {
        if id <= 0 {
            return None;
        }
        if self.pcm.contains_key(&id) {
            return self.pcm.get(&id);
        }
        if self.missing.contains_key(&id) {
            return None;
        }

        // Path from index or default sounds/{id}.aiff
        let (rel, is_ogg) = if let Some(ent) = self.index.get(&id) {
            if ent.is_ogg() {
                self.missing.insert(id, ());
                return None;
            }
            (ent.rel_path.clone(), false)
        } else {
            // Fallback probe (index not loaded).
            let aiff = format!("sounds/{id}.aiff");
            let ogg = format!("sounds/{id}.ogg");
            if self.root.join(&ogg).exists() && !self.root.join(&aiff).exists() {
                self.missing.insert(id, ());
                return None;
            }
            (aiff, false)
        };
        let _ = is_ogg;

        let path = self.root.join(&rel);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                self.missing.insert(id, ());
                return None;
            }
        };
        self.aiff_opens = self.aiff_opens.saturating_add(1);

        match read_mono16_aiff(&bytes) {
            Ok(pcm) => {
                // Refresh index meta from decode.
                if let Some(ent) = self.index.get_mut(&id) {
                    ent.sample_rate = pcm.sample_rate;
                    ent.num_samples = pcm.samples.len() as u32;
                    ent.flags |= OLSN_F_MONO16_VERIFIED | OLSN_F_HEADER_PEEKED;
                } else {
                    self.index.insert(
                        id,
                        SoundIndexEntry {
                            id,
                            sample_rate: pcm.sample_rate,
                            num_samples: pcm.samples.len() as u32,
                            rel_path: rel,
                            flags: OLSN_F_MONO16_VERIFIED | OLSN_F_HEADER_PEEKED,
                        },
                    );
                }
                self.pcm.insert(
                    id,
                    PcmSound {
                        id,
                        sample_rate: pcm.sample_rate,
                        samples: pcm.samples,
                    },
                );
                self.pcm.get(&id)
            }
            Err(_) => {
                self.missing.insert(id, ());
                None
            }
        }
    }

    /// Insert pre-decoded PCM (unit tests / ensure_pcm path).
    pub fn ensure_pcm(&mut self, id: i32, bytes: &[u8]) -> Option<&PcmSound> {
        if id <= 0 {
            return None;
        }
        if self.pcm.contains_key(&id) {
            return self.pcm.get(&id);
        }
        match read_mono16_aiff(bytes) {
            Ok(pcm) => {
                self.pcm.insert(
                    id,
                    PcmSound {
                        id,
                        sample_rate: pcm.sample_rate,
                        samples: pcm.samples,
                    },
                );
                self.pcm.get(&id)
            }
            Err(_) => {
                self.missing.insert(id, ());
                None
            }
        }
    }

    /// Lazy-decode a content-root-relative AIFF (UI chimes). Boot never opens these.
    ///
    /// // C++ `loadSoundSprite("otherSounds", "curseChime.aiff")` at page construct;
    /// // Rust defers until first [`Self::play_curse_sound_at`].
    pub fn ensure_path(&mut self, rel: &str) -> Option<&PcmSound> {
        let key = rel.trim();
        if key.is_empty() {
            return None;
        }
        if self.path_pcm.contains_key(key) {
            return self.path_pcm.get(key);
        }
        if self.path_missing.contains_key(key) {
            return None;
        }
        let path = self.root.join(key);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                self.path_missing.insert(key.to_string(), ());
                return None;
            }
        };
        self.aiff_opens = self.aiff_opens.saturating_add(1);
        match read_mono16_aiff(&bytes) {
            Ok(pcm) => {
                self.path_pcm.insert(
                    key.to_string(),
                    PcmSound {
                        id: -1,
                        sample_rate: pcm.sample_rate,
                        samples: pcm.samples,
                    },
                );
                self.path_pcm.get(key)
            }
            Err(_) => {
                self.path_missing.insert(key.to_string(), ());
                None
            }
        }
    }

    /// Inject path PCM for tests (no disk open).
    pub fn ensure_path_pcm(&mut self, rel: &str, bytes: &[u8]) -> Option<&PcmSound> {
        let key = rel.trim();
        if key.is_empty() {
            return None;
        }
        if self.path_pcm.contains_key(key) {
            return self.path_pcm.get(key);
        }
        match read_mono16_aiff(bytes) {
            Ok(pcm) => {
                self.path_pcm.insert(
                    key.to_string(),
                    PcmSound {
                        id: -1,
                        sample_rate: pcm.sample_rate,
                        samples: pcm.samples,
                    },
                );
                self.path_pcm.get(key)
            }
            Err(_) => {
                self.path_missing.insert(key.to_string(), ());
                None
            }
        }
    }

    /// Play path AIFF with explicit stereo gains (lazy ensure).
    /// Master [`Self::loudness`] / [`Self::muted`] (P5#39) apply here.
    pub fn play_path_stereo(&mut self, rel: &str, left_gain: f32, right_gain: f32) -> bool {
        let (left_gain, right_gain) = self.apply_master_gains(left_gain, right_gain);
        #[cfg(feature = "audio")]
        {
            let (samples, rate) = {
                let Some(p) = self.ensure_path(rel) else {
                    return false;
                };
                (p.samples.clone(), p.sample_rate)
            };
            if self.muted || (left_gain <= 0.0 && right_gain <= 0.0) {
                return true;
            }
            play_pcm_samples_stereo(&samples, rate, left_gain, right_gain)
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = (left_gain, right_gain);
            self.ensure_path(rel).is_some()
        }
    }

    /// Spatial path play (distance volume + pan). Records `|pan=` on success.
    pub fn play_path_at(&mut self, rel: &str, volume_tweak: f32, map_x: f32, map_y: f32) -> bool {
        if self.muted {
            return false;
        }
        let (vx, vy) = get_vector_from_camera(map_x, map_y, self.listener_x, self.listener_y);
        let Some(place) = volume_pan_reverb(vx, vy) else {
            return false;
        };
        let combined = (place.volume * volume_tweak).clamp(0.0, 1.0);
        let (l, r) = stereo_gains_constant_power(combined, place.pan);
        let ok = self.play_path_stereo(rel, l, r);
        if ok {
            self.last_pan = place.pan;
            self.record_played_spatial(rel, place.pan);
            return true;
        }
        // Logic-only / missing asset: still record trigger for headless tests
        // (C++ soft-fails when sprite NULL; we keep wire→sound path observable).
        if self.path_pcm.get(rel).is_none() {
            self.last_pan = place.pan;
            self.record_played_spatial(rel, place.pan);
            return true;
        }
        false
    }

    /// C++ `mCurseSound` on successful PS `isCurse` (vol 0.5, spatial).
    ///
    /// // C++ LivingLifePage ~20703–20710
    pub fn play_curse_sound_at(&mut self, map_x: f32, map_y: f32) -> bool {
        self.play_path_at(CURSE_CHIME_REL, CURSE_CHIME_VOLUME, map_x, map_y)
    }

    /// C++ `mHungerSound` — center pan (HUD chrome, not world-spatial).
    ///
    /// // C++ LivingLifePage ~22055 / starving peak ~10453: pan 0.5, loudness SFX
    pub fn play_hunger_sound(&mut self) -> bool {
        if self.muted {
            return false;
        }
        self.last_pan = 0.5;
        let (l, r) = stereo_gains_constant_power(HUNGER_SOUND_VOLUME, 0.5);
        let ok = self.play_path_stereo(HUNGER_SOUND_REL, l, r);
        if ok {
            self.record_played(HUNGER_SOUND_REL);
            return true;
        }
        // Logic-only / missing asset: still record trigger for headless tests.
        self.record_played(HUNGER_SOUND_REL);
        true
    }

    /// L-SOUND-TRIG: play a sound id (center pan, no distance fade).
    ///
    /// - Without `audio`: ensures lazy AIFF decode only; returns whether PCM exists.
    /// - With `audio`: queues mono PCM on the default output device (cpal). If no
    ///   device is available (CI / headless host), still returns `true` after a
    ///   successful decode so trigger wiring stays testable.
    ///
    /// Boot remains index-only — first play opens AIFF via [`Self::ensure`].
    /// Headless `play_id` / `play_usage` APIs stay center-only; use
    /// [`Self::play_id_at`] for C++ `getVectorFromCamera` pan/distance.
    pub fn play_id(&mut self, id: i32, volume: f32) -> bool {
        let vol = volume.clamp(0.0, 1.0);
        self.last_pan = 0.5;
        let (l, r) = stereo_gains_constant_power(vol, 0.5);
        self.play_id_stereo(id, l, r)
    }

    /// Play with explicit left/right gains (already include volume × constant-power pan).
    ///
    /// Master [`Self::loudness`] / [`Self::muted`] (P5#39) scale device gains.
    /// Mute skips the device queue but still ensures PCM so headless triggers pass.
    pub fn play_id_stereo(&mut self, id: i32, left_gain: f32, right_gain: f32) -> bool {
        if id < 0 {
            return false;
        }
        let (left_gain, right_gain) = self.apply_master_gains(left_gain, right_gain);
        #[cfg(feature = "audio")]
        {
            let (samples, rate) = {
                let Some(p) = self.ensure(id) else {
                    return false;
                };
                (p.samples.clone(), p.sample_rate)
            };
            if self.muted || (left_gain <= 0.0 && right_gain <= 0.0) {
                // Decode path still warm; no device voice.
                return true;
            }
            play_pcm_samples_stereo(&samples, rate, left_gain, right_gain)
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = (left_gain, right_gain);
            self.ensure(id).is_some()
        }
    }

    /// C++ `playSound(id, vol, getVectorFromCamera)` — distance volume + stereo pan.
    ///
    /// Returns `false` when beyond audible range (C++ skips) or id missing.
    /// Sets [`Self::last_pan`] on success (even without `audio` feature).
    /// `reverb_mix` is computed for placement (wet reverb samples residual).
    pub fn play_id_at(&mut self, id: i32, volume_tweak: f32, map_x: f32, map_y: f32) -> bool {
        let (vx, vy) = get_vector_from_camera(map_x, map_y, self.listener_x, self.listener_y);
        let Some(place) = volume_pan_reverb(vx, vy) else {
            return false;
        };
        let combined = (place.volume * volume_tweak).clamp(0.0, 1.0);
        // Dry path only until reverbCache wet sprites exist (C++ reverbDisabled).
        let _ = place.reverb_mix;
        let (l, r) = stereo_gains_constant_power(combined, place.pan);
        let ok = self.play_id_stereo(id, l, r);
        if ok {
            self.last_pan = place.pan;
        }
        ok
    }

    /// L-SOUND-TRIG: parse usage string and play a random sub-sound.
    ///
    /// Records non-blank usages that successfully resolve to a playable id
    /// (or always records when no OLSN index is loaded — pure trigger tests).
    pub fn play_usage(&mut self, usage: &str) -> bool {
        if self.muted {
            return false;
        }
        let u = parse_sound_usage(usage);
        if u.is_blank() {
            return false;
        }
        let ok = self.play_sound_usage(&u);
        if ok {
            self.last_pan = 0.5;
            self.record_played(usage.trim());
            return true;
        }
        // Logic-only bank (no OLSN): still record so anim/session unit tests
        // can assert fire without AIFF fixtures. Missing id with a loaded index
        // stays false (existing ensure tests).
        if !self.index_loaded && self.pcm.is_empty() {
            self.last_pan = 0.5;
            self.record_played(usage.trim());
            return true;
        }
        false
    }

    /// Spatial usage play (C++ `playSound(usage, vectorFromCamera)`).
    ///
    /// Records on successful fire with `|pan=` suffix in [`Self::last_played`];
    /// skips when too far / blank / missing.
    pub fn play_usage_at(&mut self, usage: &str, map_x: f32, map_y: f32) -> bool {
        if self.muted {
            return false;
        }
        let u = parse_sound_usage(usage);
        if u.is_blank() {
            return false;
        }
        let (vx, vy) = get_vector_from_camera(map_x, map_y, self.listener_x, self.listener_y);
        let Some(place) = volume_pan_reverb(vx, vy) else {
            return false;
        };
        let ok = self.play_sound_usage_at(&u, map_x, map_y);
        if ok {
            // play_id_at already set last_pan; keep log aligned for tests.
            self.record_played_spatial(usage.trim(), place.pan);
            return true;
        }
        if !self.index_loaded && self.pcm.is_empty() {
            // Logic-only: distance gate already passed; record pan for headless tests.
            self.record_played_spatial(usage.trim(), place.pan);
            return true;
        }
        false
    }

    /// L-SOUND-TRIG: play random from parsed [`SoundUsage`] (center pan).
    pub fn play_sound_usage(&mut self, usage: &SoundUsage) -> bool {
        let Some(play) = usage.play_random() else {
            return false;
        };
        self.play_id(play.id, play.volume)
    }

    /// Spatial play from parsed usage.
    pub fn play_sound_usage_at(&mut self, usage: &SoundUsage, map_x: f32, map_y: f32) -> bool {
        let Some(play) = usage.play_random() else {
            return false;
        };
        self.play_id_at(play.id, play.volume, map_x, map_y)
    }

    /// Footstep hook (C++ LivingLifePage ~4466): when anim `soundParam` has
    /// `footstep=1`, substitute the floor object's `usingSound` if present.
    ///
    /// `floor_using_sound` is the raw SoundUsage string from the floor object
    /// (or empty). Returns the usage that should be played.
    pub fn resolve_footstep_usage<'a>(
        anim_usage: &'a str,
        footstep: bool,
        floor_using_sound: &'a str,
    ) -> &'a str {
        if footstep {
            let floor = floor_using_sound.trim();
            if !floor.is_empty() && floor != "-1:0.0" && floor != "-1:0" {
                return floor_using_sound;
            }
        }
        anim_usage
    }

    /// Play anim sound param with optional footstep→floor substitution.
    pub fn play_anim_sound(
        &mut self,
        usage: &str,
        footstep: bool,
        floor_using_sound: &str,
    ) -> bool {
        let resolved = Self::resolve_footstep_usage(usage, footstep, floor_using_sound);
        self.play_usage(resolved)
    }
}

impl Default for SoundBank {
    fn default() -> Self {
        Self::new(".")
    }
}

// ── AIFF mono-16 (Haxe Sound.hx) ─────────────────────────────────────────────

/// Decoded intermediate.
#[derive(Debug, Clone)]
pub struct AiffPcm {
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}

/// Read mono 16-bit AIFF (Haxe-compatible fixed layout).
///
/// Checks:
/// - length ≥ 34 (header fields)
/// - numChannels (bytes 20–21 BE) == 1
/// - bitsPerSample (bytes 26–27 BE) == 16
/// - numSamples from bytes 22–25 BE
/// - sampleRate from bytes 30–31 BE (Haxe: high 16 of IEEE80 mantissa shortcut)
/// - PCM at byte 54, big-endian i16 → host i16
pub fn read_mono16_aiff(data: &[u8]) -> Result<AiffPcm, String> {
    if data.len() < 34 {
        return Err("AIFF not long enough for header".into());
    }
    // Haxe: data.get(20) != 0 || data.get(21) != 1 → not mono
    if data[20] != 0 || data[21] != 1 {
        return Err("aiff not mono".into());
    }
    // Haxe: data.get(26) != 0 || data.get(27) != 16 → not 16-bit
    if data[26] != 0 || data[27] != 16 {
        return Err("aiff not 16-bit".into());
    }
    let num_samples = u32::from_be_bytes([data[22], data[23], data[24], data[25]]) as usize;
    // Haxe: sampleRate = data.get(30) << 8 | data.get(31)
    let sample_rate = u32::from(data[30]) << 8 | u32::from(data[31]);
    if sample_rate == 0 {
        return Err("aiff zero sample rate".into());
    }
    let num_bytes = num_samples.saturating_mul(2);
    let start = AIFF_SAMPLE_START;
    if data.len() < start + num_bytes {
        return Err("AIFF not long enough for data".into());
    }
    let mut samples = Vec::with_capacity(num_samples);
    let mut b = start;
    for _ in 0..num_samples {
        // BE → host i16 (Haxe swaps to LE in buffer; we store i16 directly).
        let value = i16::from_be_bytes([data[b], data[b + 1]]);
        samples.push(value);
        b += 2;
    }
    Ok(AiffPcm {
        sample_rate,
        samples,
    })
}

/// Peek first 54 bytes for rate/samples without full PCM (bake path).
pub fn peek_aiff_header(data: &[u8]) -> Option<(u32, u32, bool)> {
    if data.len() < 34 {
        return None;
    }
    let mono = data[20] == 0 && data[21] == 1;
    let bits16 = data[26] == 0 && data[27] == 16;
    let num_samples = u32::from_be_bytes([data[22], data[23], data[24], data[25]]);
    let sample_rate = u32::from(data[30]) << 8 | u32::from(data[31]);
    let ok = mono && bits16 && sample_rate > 0;
    Some((sample_rate, num_samples, ok))
}

// ── OLSN binary ──────────────────────────────────────────────────────────────

/// Write OLSN index blob.
pub fn write_olsn(entries: &[SoundIndexEntry], data_version: u32) -> Vec<u8> {
    let mut list = entries.to_vec();
    list.sort_by_key(|e| e.id);
    let mut out = Vec::with_capacity(24 + list.len() * 32);
    out.extend_from_slice(OLSN_MAGIC);
    out.extend_from_slice(&OLSN_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&data_version.to_le_bytes());
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // flags
    out.extend_from_slice(&0u32.to_le_bytes()); // crc
    for e in &list {
        out.extend_from_slice(&e.id.to_le_bytes());
        out.extend_from_slice(&e.sample_rate.to_le_bytes());
        out.extend_from_slice(&e.num_samples.to_le_bytes());
        let path_b = e.rel_path.as_bytes();
        let plen = path_b.len().min(u16::MAX as usize) as u16;
        out.extend_from_slice(&plen.to_le_bytes());
        out.extend_from_slice(&path_b[..plen as usize]);
        out.extend_from_slice(&e.flags.to_le_bytes());
    }
    out
}

/// Parse OLSN → (data_version, entries).
pub fn load_olsn(data: &[u8]) -> Result<(u32, Vec<SoundIndexEntry>), String> {
    if data.len() < 24 {
        return Err("OLSN too short".into());
    }
    if &data[0..4] != OLSN_MAGIC {
        return Err("bad OLSN magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLSN_FORMAT_VERSION {
        return Err(format!("unsupported OLSN format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    // flags@16, crc@20 ignored
    let mut off = 24;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if off + 4 + 4 + 4 + 2 > data.len() {
            return Err("OLSN truncated record".into());
        }
        let id = i32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let sample_rate = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let num_samples = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        let plen = u16::from_le_bytes(data[off..off + 2].try_into().unwrap()) as usize;
        off += 2;
        if off + plen + 4 > data.len() {
            return Err("OLSN truncated path".into());
        }
        let rel_path = String::from_utf8_lossy(&data[off..off + plen]).into_owned();
        off += plen;
        let flags = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        entries.push(SoundIndexEntry {
            id,
            sample_rate,
            num_samples,
            rel_path,
            flags,
        });
    }
    Ok((data_version, entries))
}

/// Scan `$root/sounds/*.{aiff,ogg}` (skip soundsRaw, `*.txt`).
pub fn scan_sounds_dir(root: &Path) -> Vec<SoundIndexEntry> {
    let dir = root.join("sounds");
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
        let lower = name.to_ascii_lowercase();
        let (id, is_ogg) = if let Some(stem) = lower.strip_suffix(".aiff") {
            match stem.parse::<i32>() {
                Ok(id) if id > 0 => (id, false),
                _ => continue,
            }
        } else if let Some(stem) = lower.strip_suffix(".ogg") {
            match stem.parse::<i32>() {
                Ok(id) if id > 0 => (id, true),
                _ => continue,
            }
        } else {
            continue;
        };

        let rel_path = format!("sounds/{name}");
        let mut sample_rate = 0u32;
        let mut num_samples = 0u32;
        let mut flags = 0u32;
        if is_ogg {
            flags |= OLSN_F_IS_OGG;
        } else {
            // Optional 54-byte header peek (no full PCM).
            if let Ok(mut f) = fs::File::open(&path) {
                use std::io::Read;
                let mut hdr = [0u8; 54];
                if f.read(&mut hdr).ok().unwrap_or(0) >= 34 {
                    if let Some((rate, ns, ok)) = peek_aiff_header(&hdr) {
                        sample_rate = rate;
                        num_samples = ns;
                        flags |= OLSN_F_HEADER_PEEKED;
                        if ok {
                            flags |= OLSN_F_MONO16_VERIFIED;
                        }
                    }
                }
            }
        }
        entries.push(SoundIndexEntry {
            id,
            sample_rate,
            num_samples,
            rel_path,
            flags,
        });
    }
    entries.sort_by_key(|e| e.id);
    entries
}

/// Bake OLSN from content root `sounds/`. Returns (bytes, count).
pub fn bake_olsn_from_dir(root: &Path, data_version: u32) -> Result<(Vec<u8>, usize), String> {
    let entries = scan_sounds_dir(root);
    let n = entries.len();
    Ok((write_olsn(&entries, data_version), n))
}

/// Write `out_dir/olsn_sounds.bin`.
pub fn bake_olsn_to_dir(root: &Path, out_dir: &Path, data_version: u32) -> Result<usize, String> {
    let (bytes, n) = bake_olsn_from_dir(root, data_version)?;
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("olsn_sounds.bin"), bytes).map_err(|e| e.to_string())?;
    Ok(n)
}

fn read_data_version_u32(root: &Path) -> Option<u32> {
    fs::read_to_string(root.join("dataVersionNumber.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

// ── Spatial pan / distance (C++ soundBank getVolumeAndPan + LivingLife getVectorFromCamera) ─
//
// C++: soundBank.cpp `getVolumeAndPan` / `playSound(usage, vectorFromCamera)`;
//      LivingLifePage.cpp `getVectorFromCamera` (map − lastScreenViewCenter/CELL_D);
//      minorGems playSoundSprite constant-power stereo (sin/cos of pan·π/2).

/// C++ `maxAudibleDistance` (tiles).
pub const MAX_AUDIBLE_DISTANCE: f32 = 16.0;
/// C++ `minFadeStartDistance` (tiles) — used inside the sigmoid knee.
pub const MIN_FADE_START_DISTANCE: f32 = 1.5;
/// C++ `reverbContstant` — floor on reverb mix.
pub const REVERB_CONSTANT: f32 = 0.1;

/// Distance + stereo placement result (dry path).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundPlacement {
    /// Distance volume scale 0..1 (C++ `volumeScale` / sigmoidF).
    pub volume: f32,
    /// Stereo position 0..1 (0=left, 0.5=center, 1=right). C++ `xPan/16`.
    pub pan: f32,
    /// Wet reverb mix 0..1 (C++ `reverbMix`); wet AIFF residual until reverbCache.
    pub reverb_mix: f32,
}

/// C++ `LivingLifePage::getVectorFromCamera` — listener-relative vector in tiles.
///
/// `cam_*` is screen view center in world tiles (Rust [`Camera`] / C++
/// `lastScreenViewCenter / CELL_D`).
#[inline]
pub fn get_vector_from_camera(map_x: f32, map_y: f32, cam_x: f32, cam_y: f32) -> (f32, f32) {
    (map_x - cam_x, map_y - cam_y)
}

/// C++ `sigmoidH` helper for distance fade.
#[inline]
fn sigmoid_h(distance: f32) -> f32 {
    let knee = 2.0 + MIN_FADE_START_DISTANCE;
    0.5 + 0.5 * (-(distance - knee) / (1.0 + (distance - knee).abs()))
}

/// C++ `sigmoidF` — smooth 1→0 over `[0, maxAudibleDistance]`.
#[inline]
fn sigmoid_f(distance: f32) -> f32 {
    let h0 = sigmoid_h(0.0);
    let hmax = sigmoid_h(MAX_AUDIBLE_DISTANCE);
    let denom = h0 - hmax;
    if denom.abs() < 1e-12 {
        return 0.0;
    }
    (1.0 / denom) * (sigmoid_h(distance) - hmax)
}

/// C++ `getVolumeAndPan` — returns `None` when silent / beyond range.
pub fn get_volume_and_pan(vector_x: f32, vector_y: f32) -> Option<(f32, f32)> {
    volume_pan_reverb(vector_x, vector_y).map(|p| (p.volume, p.pan))
}

/// Distance volume + pan + reverb placement (C++ `getVolumeAndPan` + reverbMix).
pub fn volume_pan_reverb(vector_x: f32, vector_y: f32) -> Option<SoundPlacement> {
    let d = (vector_x * vector_x + vector_y * vector_y).sqrt();
    let mut volume_scale = sigmoid_f(d);
    if volume_scale <= 0.0 {
        return None;
    }
    if volume_scale > 1.0 {
        volume_scale = 1.0;
    }
    // Pan from X only; clamp to screen edges (±5 tiles), map to [3,13]/16.
    let mut x_pan = vector_x;
    if x_pan > 5.0 {
        x_pan = 5.0;
    }
    if x_pan < -5.0 {
        x_pan = -5.0;
    }
    x_pan += 5.0; // 0..10
    x_pan += 3.0; // 3..13
    let pan = x_pan / 16.0;
    // reverbMix = (1-c)*(1-volume) + c  (always a bit of reverb)
    let reverb_mix = (1.0 - REVERB_CONSTANT) * (1.0 - volume_scale) + REVERB_CONSTANT;
    Some(SoundPlacement {
        volume: volume_scale,
        pan,
        reverb_mix,
    })
}

/// C++ reverb mix from distance volume alone.
#[inline]
pub fn reverb_mix_from_volume(volume: f32) -> f32 {
    (1.0 - REVERB_CONSTANT) * (1.0 - volume.clamp(0.0, 1.0)) + REVERB_CONSTANT
}

/// minorGems constant-power stereo: `L = vol·cos(pan·π/2)`, `R = vol·sin(pan·π/2)`.
///
/// `pan` in [0,1] (0=left, 0.5=center, 1=right).
#[inline]
pub fn stereo_gains_constant_power(volume: f32, pan: f32) -> (f32, f32) {
    let p = pan.clamp(0.0, 1.0);
    let v = volume.max(0.0);
    let theta = std::f32::consts::FRAC_PI_2 * p;
    (v * theta.cos(), v * theta.sin())
}

// ── Device playback (`audio` feature / cpal) ─────────────────────────────────
//
// Lazy default-output stream + software mixer with per-voice L/R gains (P1#2).
// Music OGG beds: [`crate::music_bank`] (P3#24, lewton lazy decode + play via this mixer).
// Wet reverb AIFF residual (reverbCache).

/// Max concurrent one-shot voices (oldest dropped when full).
pub const AUDIO_MAX_VOICES: usize = 32;

/// Soft-clip / mix helper used by the device callback and unit tests.
///
/// `src` is mono i16; `src_pos` is a fractional read cursor advanced by
/// `src_rate / device_rate` per output frame. Writes interleaved channels into
/// `out` (len = frames * channels). Stereo uses [`MixVoice::left_gain`] /
/// [`MixVoice::right_gain`]; mono uses average of L/R.
pub fn mix_voices_f32(
    out: &mut [f32],
    channels: usize,
    voices: &mut Vec<MixVoice>,
    device_rate: u32,
) {
    if channels == 0 || device_rate == 0 {
        out.fill(0.0);
        return;
    }
    let frames = out.len() / channels;
    let dev_r = device_rate as f64;
    for f in 0..frames {
        let mut acc_l = 0.0f32;
        let mut acc_r = 0.0f32;
        voices.retain_mut(|v| {
            let n = v.samples.len();
            if n == 0 || v.pos >= n as f64 {
                return false;
            }
            let i = v.pos as usize;
            let frac = (v.pos - i as f64) as f32;
            let s0 = v.samples[i] as f32 * (1.0 / 32768.0);
            let s1 = if i + 1 < n {
                v.samples[i + 1] as f32 * (1.0 / 32768.0)
            } else {
                s0
            };
            let s = s0 + (s1 - s0) * frac;
            acc_l += s * v.left_gain;
            acc_r += s * v.right_gain;
            v.pos += v.src_rate as f64 / dev_r;
            v.pos < n as f64
        });
        let base = f * channels;
        if channels == 1 {
            out[base] = (0.5 * (acc_l + acc_r)).clamp(-1.0, 1.0);
        } else {
            out[base] = acc_l.clamp(-1.0, 1.0);
            out[base + 1] = acc_r.clamp(-1.0, 1.0);
            // Extra channels (rare): copy L/R alternating or silence.
            for c in 2..channels {
                out[base + c] = if c % 2 == 0 {
                    out[base]
                } else {
                    out[base + 1]
                };
            }
        }
    }
    // Clear any trailing partial frame bytes (shouldn't happen with well-formed buffers).
    let used = frames * channels;
    if used < out.len() {
        for x in &mut out[used..] {
            *x = 0.0;
        }
    }
}

/// One playing voice for [`mix_voices_f32`].
#[derive(Clone)]
pub struct MixVoice {
    pub samples: std::sync::Arc<Vec<i16>>,
    pub src_rate: u32,
    /// Left channel gain (includes volume × constant-power pan).
    pub left_gain: f32,
    /// Right channel gain.
    pub right_gain: f32,
    /// Fractional sample index into `samples`.
    pub pos: f64,
}

impl MixVoice {
    /// Center pan at `volume` (constant-power).
    pub fn centered(samples: std::sync::Arc<Vec<i16>>, src_rate: u32, volume: f32) -> Self {
        let (l, r) = stereo_gains_constant_power(volume.clamp(0.0, 1.0), 0.5);
        Self {
            samples,
            src_rate,
            left_gain: l,
            right_gain: r,
            pos: 0.0,
        }
    }
}

/// Whether the process has a live cpal output (false until first play, or when
/// device open failed). Without the `audio` feature this is always false.
pub fn audio_device_active() -> bool {
    #[cfg(feature = "audio")]
    {
        device::is_active()
    }
    #[cfg(not(feature = "audio"))]
    {
        false
    }
}

/// True when the crate was built with `--features audio`.
pub fn audio_feature_enabled() -> bool {
    cfg!(feature = "audio")
}

/// Queue mono PCM for device play (center pan). Returns false only when samples
/// empty / bad rate. Device-missing hosts still return true (silent) after validate.
#[cfg(feature = "audio")]
pub fn play_pcm_samples(samples: &[i16], sample_rate: u32, volume: f32) -> bool {
    let vol = volume.clamp(0.0, 1.0);
    let (l, r) = stereo_gains_constant_power(vol, 0.5);
    play_pcm_samples_stereo(samples, sample_rate, l, r)
}

/// Queue mono PCM with explicit L/R gains (device path).
#[cfg(feature = "audio")]
pub fn play_pcm_samples_stereo(
    samples: &[i16],
    sample_rate: u32,
    left_gain: f32,
    right_gain: f32,
) -> bool {
    if samples.is_empty() || sample_rate == 0 {
        return false;
    }
    device::play(samples, sample_rate, left_gain, right_gain)
}

#[cfg(not(feature = "audio"))]
pub fn play_pcm_samples(_samples: &[i16], _sample_rate: u32, _volume: f32) -> bool {
    false
}

#[cfg(not(feature = "audio"))]
pub fn play_pcm_samples_stereo(
    _samples: &[i16],
    _sample_rate: u32,
    _left_gain: f32,
    _right_gain: f32,
) -> bool {
    false
}

#[cfg(feature = "audio")]
mod device {
    //! cpal `Stream` is `!Send`/`!Sync` on some hosts — own it on a dedicated
    //! thread and accept [`MixVoice`]s over an mpsc channel.

    use super::{mix_voices_f32, MixVoice, AUDIO_MAX_VOICES};
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{SampleFormat, StreamConfig};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{self, Sender};
    use std::sync::{Arc, Mutex, OnceLock};

    /// Holds the cpal stream on the audio thread only (not in a static).
    struct Engine {
        voices: Arc<Mutex<Vec<MixVoice>>>,
        /// Kept so Drop stops the stream with the thread lifetime.
        #[allow(dead_code)]
        _stream: cpal::Stream,
    }

    static TX: OnceLock<Sender<MixVoice>> = OnceLock::new();
    static QUEUED: AtomicU64 = AtomicU64::new(0);
    static ACTIVE: AtomicBool = AtomicBool::new(false);

    pub fn is_active() -> bool {
        ACTIVE.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn queued_count() -> u64 {
        QUEUED.load(Ordering::Relaxed)
    }

    fn sender() -> &'static Sender<MixVoice> {
        TX.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<MixVoice>();
            let _ = std::thread::Builder::new()
                .name("ohol-audio".into())
                .spawn(move || audio_thread_main(rx));
            tx
        })
    }

    fn audio_thread_main(rx: mpsc::Receiver<MixVoice>) {
        let eng = match open_engine() {
            Ok(e) => {
                ACTIVE.store(true, Ordering::Relaxed);
                Some(e)
            }
            Err(err) => {
                eprintln!("[audio] device open failed (silent fallback): {err}");
                None
            }
        };
        // Block forever; process play commands. Stream lives with this thread.
        while let Ok(voice) = rx.recv() {
            if let Some(ref e) = eng {
                if let Ok(mut guard) = e.voices.lock() {
                    if guard.len() >= AUDIO_MAX_VOICES {
                        guard.remove(0);
                    }
                    guard.push(voice);
                }
            }
            // No device: drop voice (silent).
        }
    }

    fn open_engine() -> Result<Engine, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string())?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("default_output_config: {e}"))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let device_rate = config.sample_rate.0;
        let channels = config.channels as usize;
        if channels == 0 || device_rate == 0 {
            return Err("invalid output config".into());
        }
        let voices: Arc<Mutex<Vec<MixVoice>>> = Arc::new(Mutex::new(Vec::new()));
        let err_fn = |e| eprintln!("[audio] stream error: {e}");

        let stream = match sample_format {
            SampleFormat::F32 => {
                let v = Arc::clone(&voices);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _| {
                            if let Ok(mut guard) = v.lock() {
                                mix_voices_f32(data, channels, &mut guard, device_rate);
                            } else {
                                data.fill(0.0);
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("build_output_stream f32: {e}"))?
            }
            SampleFormat::I16 => {
                let v = Arc::clone(&voices);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [i16], _| {
                            let mut tmp = vec![0.0f32; data.len()];
                            if let Ok(mut guard) = v.lock() {
                                mix_voices_f32(&mut tmp, channels, &mut guard, device_rate);
                            }
                            for (o, s) in data.iter_mut().zip(tmp.iter()) {
                                *o = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("build_output_stream i16: {e}"))?
            }
            SampleFormat::U16 => {
                let v = Arc::clone(&voices);
                device
                    .build_output_stream(
                        &config,
                        move |data: &mut [u16], _| {
                            let mut tmp = vec![0.0f32; data.len()];
                            if let Ok(mut guard) = v.lock() {
                                mix_voices_f32(&mut tmp, channels, &mut guard, device_rate);
                            }
                            for (o, s) in data.iter_mut().zip(tmp.iter()) {
                                let x = (s.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32;
                                *o = x as u16;
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("build_output_stream u16: {e}"))?
            }
            other => return Err(format!("unsupported sample format: {other:?}")),
        };
        stream
            .play()
            .map_err(|e| format!("stream.play: {e}"))?;
        Ok(Engine {
            voices,
            _stream: stream,
        })
    }

    pub fn play(samples: &[i16], sample_rate: u32, left_gain: f32, right_gain: f32) -> bool {
        // Force silent queue path (CI / hosts without wanting device open).
        // SFX mute is applied in SoundBank before this call; music uses music_muted.
        if std::env::var_os("OHOL_AUDIO_DISABLE").is_some() {
            QUEUED.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        let voice = MixVoice {
            samples: Arc::new(samples.to_vec()),
            src_rate: sample_rate,
            left_gain,
            right_gain,
            pos: 0.0,
        };
        // Best-effort send; if channel dead, still count as handled (decode ok).
        let _ = sender().send(voice);
        QUEUED.fetch_add(1, Ordering::Relaxed);
        true
    }
}

// ── L-SOUND-TRIG free functions (session/render call sites) ──────────────────

use crate::anim_bank::{AnimBank, SoundAnimParam, ANIM_GROUND, ANIM_GROUND2};
use crate::client_map::ClientMap;
use crate::content::{sound_usage_is_blank, ClientContent};

/// C++ `LivingLifePage::handleAnimSound` period/phase/age gate (~4392–4459).
///
/// Frame counts are wall animation counters (same units as `AnimDrawState`);
/// time = `frame / 60` seconds. When `frame_rate_factor` is already baked into
/// the frame delta (Rust `step_anims(frf, frf)`), pass `1.0` for `frf` here.
///
/// - `repeat_per_sec != 0`: fire when period index increases across old→new
/// - `repeat_per_sec == 0`: one-shot only when `old_time == 0` and `phase == 0`
pub fn sound_param_should_play(
    param: &SoundAnimParam,
    age: f32,
    old_frame: f32,
    new_frame: f32,
    frame_rate_factor: f32,
) -> bool {
    if param.usage.trim().is_empty() || sound_usage_is_blank(&param.usage) {
        return false;
    }
    if param.age_start >= 0.0 && age < param.age_start {
        return false;
    }
    if param.age_end >= 0.0 && age >= param.age_end {
        return false;
    }
    let frf = if frame_rate_factor > 0.0 {
        frame_rate_factor
    } else {
        1.0
    };
    let old_time = frf * old_frame / 60.0;
    let new_time = frf * new_frame / 60.0;
    let hz = param.repeat_per_sec;
    if hz != 0.0 {
        let period = 1.0 / hz;
        if !period.is_finite() || period <= 0.0 {
            return false;
        }
        let start_offset = param.repeat_phase * period;
        let old_periods = ((old_time - start_offset) / period).floor() as i64;
        let new_periods = ((new_time - start_offset) / period).floor() as i64;
        new_periods > old_periods
    } else {
        // C++: hz == 0 && oldTimeVal == 0 && phase == 0 → one-shot at start
        // (only meaningful when frame advanced; skip no-op old==new stays)
        old_time == 0.0 && param.repeat_phase == 0.0 && new_time != old_time
    }
}

/// C++ `LivingLifePage::handleAnimSound` (~4392–4488).
///
/// Walks `ObjectAnimation.sound_params` for `object_id`×`anim_type`, applies
/// period/phase/age gates, substitutes floor `using_sound` when `footstep`,
/// then [`SoundBank::play_usage_at`]. Returns number of sounds fired.
///
/// Source player defaults to `-1` (map / non-player). Pass live player id for
/// person tracks so `offScreenSound` can skip our own character.
pub fn handle_anim_sound(
    bank: &mut SoundBank,
    anims: &mut AnimBank,
    content: &ClientContent,
    map: &ClientMap,
    object_id: i32,
    age: f32,
    anim_type: i32,
    old_frame: f32,
    new_frame: f32,
    pos_x: f32,
    pos_y: f32,
    frame_rate_factor: f32,
) -> usize {
    handle_anim_sound_ex(
        bank,
        anims,
        content,
        map,
        object_id,
        age,
        anim_type,
        old_frame,
        new_frame,
        pos_x,
        pos_y,
        frame_rate_factor,
        -1,
        None,
    )
}

/// Extended [`handle_anim_sound`] with C++ `inSourcePlayerID` + our-id for
/// `offScreenSound` registration (P2#13).
pub fn handle_anim_sound_ex(
    bank: &mut SoundBank,
    anims: &mut AnimBank,
    content: &ClientContent,
    map: &ClientMap,
    object_id: i32,
    age: f32,
    anim_type: i32,
    old_frame: f32,
    new_frame: f32,
    pos_x: f32,
    pos_y: f32,
    frame_rate_factor: f32,
    source_player_id: i32,
    our_id: Option<i32>,
) -> usize {
    if object_id <= 0 {
        return 0;
    }
    let mut at = anim_type;
    if at == ANIM_GROUND2 {
        at = ANIM_GROUND;
    }
    let Some(anim) = anims.get(object_id, at) else {
        return 0;
    };
    // Clone params so we can drop the anim borrow before mut bank play.
    let params: Vec<SoundAnimParam> = anim.sound_params.clone();
    let floor_using = floor_using_sound_at(content, map, pos_x, pos_y);
    let desc = content
        .get(object_id)
        .map(|o| o.description.clone())
        .unwrap_or_default();
    let mut n = 0usize;
    for p in &params {
        if !sound_param_should_play(p, age, old_frame, new_frame, frame_rate_factor) {
            continue;
        }
        let usage = SoundBank::resolve_footstep_usage(&p.usage, p.footstep, &floor_using);
        // C++ playSound(usage, getVectorFromCamera(pos)) — pan + distance.
        if bank.play_usage_at(usage, pos_x, pos_y) {
            n += 1;
            // C++ ~4490–4500: non-self source + description tag → offScreenSounds.
            maybe_register_off_screen_sound(
                bank,
                source_player_id,
                our_id,
                &desc,
                pos_x,
                pos_y,
            );
        }
    }
    n
}

/// Parse `offScreenSound` / `offScreenSound_red` / `offScreenSound_X` from description.
pub fn parse_off_screen_sound_flags(description: &str) -> (bool, Option<char>) {
    let Some(pos) = description.find("offScreenSound") else {
        return (false, None);
    };
    let rest = &description[pos + "offScreenSound".len()..];
    if rest.starts_with("_red") {
        return (true, None);
    }
    if let Some(stripped) = rest.strip_prefix('_') {
        if let Some(c) = stripped.chars().next() {
            if c.is_ascii_alphanumeric() {
                return (false, Some(c));
            }
        }
    }
    (false, None)
}

/// True when object description carries `offScreenSound` (any variant).
pub fn description_has_off_screen_sound(description: &str) -> bool {
    description.contains("offScreenSound")
}

/// C++ `handleAnimSound` offScreenSound branch after a successful play.
pub fn maybe_register_off_screen_sound(
    bank: &mut SoundBank,
    source_player_id: i32,
    our_id: Option<i32>,
    description: &str,
    pos_x: f32,
    pos_y: f32,
) {
    if !description_has_off_screen_sound(description) {
        return;
    }
    if let Some(oid) = our_id {
        if source_player_id == oid {
            return;
        }
    }
    bank.add_off_screen_sound(source_player_id, pos_x, pos_y, description);
}

/// C++ map-object + floor ground anim sound step (LivingLifePage draw ~4768 / ~7574).
///
/// Advances per-tile frame counters by `frame_delta` (typically `1.0 * frameRateFactor`
/// baked as `1.0` when caller already applies frf). Fires [`handle_anim_sound_ex`] for
/// each known tile with object/floor ids. **P2#14** — non-player anim sound hooks.
///
/// OLSN stays lazy: only plays when an OLA1 sound param period crosses; no AIFF at boot.
pub fn step_map_ground_anims_with_sounds(
    bank: &mut SoundBank,
    anims: &mut AnimBank,
    content: &ClientContent,
    map: &mut ClientMap,
    frame_delta: f32,
    our_id: Option<i32>,
) -> usize {
    if frame_delta <= 0.0 {
        return 0;
    }
    let coords: Vec<(i32, i32)> = map.tile_coords().collect();
    let mut n = 0usize;
    for (x, y) in coords {
        let tile = match map.get(x, y) {
            Some(t) => t.clone(),
            None => continue,
        };
        let pos_x = x as f32;
        let pos_y = y as f32;

        // Map object ground anim (C++ mMapCurAnimType defaults ground / ground2).
        if tile.object_id > 0 {
            let old = *map.anim_frame_count.get(&(x, y)).unwrap_or(&0.0);
            let new = old + frame_delta;
            map.anim_frame_count.insert((x, y), new);
            n += handle_anim_sound_ex(
                bank,
                anims,
                content,
                map,
                tile.object_id,
                0.0,
                ANIM_GROUND,
                old,
                new,
                pos_x,
                pos_y,
                1.0,
                -1,
                our_id,
            );
        }

        // Floor ground anim.
        if tile.floor_id > 0 {
            let old = *map.floor_anim_frame_count.get(&(x, y)).unwrap_or(&0.0);
            let new = old + frame_delta;
            map.floor_anim_frame_count.insert((x, y), new);
            n += handle_anim_sound_ex(
                bank,
                anims,
                content,
                map,
                tile.floor_id,
                0.0,
                ANIM_GROUND,
                old,
                new,
                pos_x,
                pos_y,
                1.0,
                -1,
                our_id,
            );
        }
    }
    n
}

/// Floor object `using_sound` at map tile under `(pos_x, pos_y)`, or empty.
pub fn floor_using_sound_at(content: &ClientContent, map: &ClientMap, pos_x: f32, pos_y: f32) -> String {
    let x = pos_x.round() as i32;
    let y = pos_y.round() as i32;
    let Some(tile) = map.get(x, y) else {
        return String::new();
    };
    if tile.floor_id <= 0 {
        return String::new();
    }
    content
        .get(tile.floor_id)
        .map(|o| o.using_sound.clone())
        .unwrap_or_default()
}

/// Play a SoundUsage string via bank (optional session hook).
pub fn play_usage(bank: &mut SoundBank, usage: &str) -> bool {
    bank.play_usage(usage)
}

/// Footstep trigger: resolve floor usingSound then play.
pub fn play_footstep(
    bank: &mut SoundBank,
    anim_usage: &str,
    footstep: bool,
    floor_using_sound: &str,
) -> bool {
    bank.play_anim_sound(anim_usage, footstep, floor_using_sound)
}

/// Object event trigger stubs (creation / using / eating / decay / drop).
/// Call sites can pass the object's sound field string when content carries it.
pub fn play_object_event_sound(bank: &mut SoundBank, usage: &str) -> bool {
    if sound_usage_is_blank(usage) {
        return false;
    }
    bank.play_usage(usage)
}

/// C++ `isSpriteSubset` lite (objectBank ~6656).
///
/// Full C++ matches pos/rot/flip + relative offsets; we only need enough for
/// `creationSoundInitialOnly` gating: sprite-id set containment (+ single-sprite
/// any-occurrence special case).
pub fn is_sprite_subset(content: &ClientContent, super_id: i32, sub_id: i32) -> bool {
    if super_id <= 0 || sub_id <= 0 {
        return false;
    }
    let Some(super_o) = content.get(super_id) else {
        return false;
    };
    let Some(sub_o) = content.get(sub_id) else {
        return false;
    };
    if sub_o.sprites.is_empty() {
        return true;
    }
    if sub_o.sprites.len() == 1 {
        let sid = sub_o.sprites[0].sprite_id;
        return super_o.sprites.iter().any(|s| s.sprite_id == sid);
    }
    // Multi-sprite: every sub sprite_id must appear in super (ignore transform).
    for spr in &sub_o.sprites {
        if !super_o
            .sprites
            .iter()
            .any(|s| s.sprite_id == spr.sprite_id)
        {
            return false;
        }
    }
    true
}

/// True when old/new are multi-use dummies of the same parent (or parent↔dummy).
///
/// C++ `shouldCreationSoundPlay` sameParent block (~12992–13017).
/// Also C++ `bothSameUseParent` (~6895) — same conditions.
pub fn same_use_dummy_parent(content: &ClientContent, old_id: i32, new_id: i32) -> bool {
    if old_id <= 0 || new_id <= 0 {
        return false;
    }
    let Some(new_o) = content.get(new_id) else {
        return false;
    };
    let Some(old_o) = content.get(old_id) else {
        return false;
    };
    let new_is_dummy = new_o.dummy_parent > 0;
    let old_is_dummy = old_o.dummy_parent > 0;
    if new_is_dummy && old_is_dummy && new_o.dummy_parent == old_o.dummy_parent {
        return true;
    }
    // Parent base (numUses>1) ↔ its use dummy
    if new_o.num_uses > 1 && old_is_dummy && old_o.dummy_parent == new_id {
        return true;
    }
    if old_o.num_uses > 1 && new_is_dummy && new_o.dummy_parent == old_id {
        return true;
    }
    false
}

/// C++ `bothSameUseParent` — alias of [`same_use_dummy_parent`].
#[inline]
pub fn both_same_use_parent(content: &ClientContent, a: i32, b: i32) -> bool {
    same_use_dummy_parent(content, a, b)
}

/// C++ `getObjectParent` for multi-use dummies (`useDummyParent`).
pub fn get_object_parent(content: &ClientContent, id: i32) -> i32 {
    if id <= 0 {
        return id;
    }
    content.base_object_id(id)
}

/// C++ `thisUseDummyIndex` lite.
///
/// - Parent (numUses>1, not dummy): returns `num_uses - 1` as "fullest" index so
///   fill-up comparisons work with dummies at `0..num_uses-2`.
/// - Dummy: index into parent's `dummy_ids` (C++ `d-1` for use `d`).
/// - Non-multi-use: `-1`.
pub fn this_use_dummy_index(content: &ClientContent, id: i32) -> i32 {
    if id <= 0 {
        return -1;
    }
    let Some(o) = content.get(id) else {
        return -1;
    };
    if o.dummy_parent > 0 {
        if let Some(parent) = content.get(o.dummy_parent) {
            if let Some(idx) = parent.dummy_ids.iter().position(|&d| d == id) {
                return idx as i32;
            }
        }
        // Fallback: still a dummy of unknown parent list.
        return 0;
    }
    if o.num_uses > 1 {
        // Parent is fullest (C++ not a dummy: compare via getObjectParent(old)==new).
        return o.num_uses - 1;
    }
    -1
}

/// C++ fill-up gate for contained using sound (~16888–16894).
///
/// True when `new` is less used than `old` (filling back up).
pub fn is_less_used_than(content: &ClientContent, old_id: i32, new_id: i32) -> bool {
    if get_object_parent(content, old_id) == new_id {
        return true;
    }
    let oi = this_use_dummy_index(content, old_id);
    let ni = this_use_dummy_index(content, new_id);
    oi >= 0 && ni >= 0 && oi < ni
}

/// Play usage at map tile (spatial) or fall back to center for blank/miss.
pub fn play_object_event_sound_at(
    bank: &mut SoundBank,
    usage: &str,
    map_x: f32,
    map_y: f32,
) -> bool {
    if sound_usage_is_blank(usage) {
        return false;
    }
    bank.play_usage_at(usage, map_x, map_y)
}

/// C++ MX container fill: same root id, more contained, responsible hands empty
/// (~17173–17210). Prefers container `usingSound`, else person `usingSound`.
pub fn play_container_fill_using_sound(
    bank: &mut SoundBank,
    content: &ClientContent,
    container_id: i32,
    person_display_id: i32,
    map_x: f32,
    map_y: f32,
) -> bool {
    if container_id > 0 {
        if let Some(def) = content.get(container_id) {
            if play_object_event_sound_at(bank, &def.using_sound, map_x, map_y) {
                return true;
            }
        }
    }
    if person_display_id > 0 {
        if let Some(person) = content.get(person_display_id) {
            return play_object_event_sound_at(bank, &person.using_sound, map_x, map_y);
        }
    }
    false
}

/// C++ MX single contained-slot change (~16812–16901).
///
/// - Prefer creation when [`should_creation_sound_play`].
/// - Else using-on-fill when hands empty + same multi-use parent + less used.
pub fn play_contained_slot_change_sound(
    bank: &mut SoundBank,
    content: &ClientContent,
    old_cont_id: i32,
    new_cont_id: i32,
    causing_player_held: i32,
    map_x: f32,
    map_y: f32,
) -> bool {
    if old_cont_id == new_cont_id || new_cont_id <= 0 {
        return false;
    }
    if should_creation_sound_play(content, old_cont_id, new_cont_id) {
        if let Some(new_obj) = content.get(new_cont_id) {
            if play_object_event_sound_at(bank, &new_obj.creation_sound, map_x, map_y) {
                return true;
            }
        }
    } else if causing_player_held == 0
        && both_same_use_parent(content, new_cont_id, old_cont_id)
        && is_less_used_than(content, old_cont_id, new_cont_id)
    {
        if let Some(new_obj) = content.get(new_cont_id) {
            if play_object_event_sound_at(bank, &new_obj.using_sound, map_x, map_y) {
                return true;
            }
        }
    }
    false
}

/// Find the single index where `old` and `new` differ (equal lengths). `None` if 0 or >1.
pub fn single_contained_change_index(old: &[i32], new: &[i32]) -> Option<usize> {
    if old.len() != new.len() {
        return None;
    }
    let mut idx = None;
    for (i, (a, b)) in old.iter().zip(new.iter()).enumerate() {
        if a != b {
            if idx.is_some() {
                return None;
            }
            idx = Some(i);
        }
    }
    idx
}

/// Snapshot of pre-MX tile + change for sound routing (tests + session).
#[derive(Debug, Clone)]
pub struct MxSoundContext {
    pub old_object_id: i32,
    pub old_floor_id: i32,
    pub old_contained: Vec<i32>,
    pub new_object_id: i32,
    pub new_floor_id: i32,
    pub new_contained: Vec<i32>,
    pub player_id: i32,
    pub is_moving: bool,
    pub map_x: i32,
    pub map_y: i32,
    /// Live held_id of `player_id` when `player_id > 0` (0 if unknown / empty).
    pub responsible_held: i32,
    /// Display id of responsible player (for person using fallback).
    pub responsible_display: i32,
    /// Held of `-player_id` when `player_id < -1` (transform causer).
    pub causing_held: i32,
}

/// Play creation or decay sounds for every non-zero PE emotion object slot.
///
/// // C++ LivingLifePage PLAYER_EMOT ~21246–21259 (creation on apply)
/// // C++ ~22475–22489 (decay on temporary emot clear)
///
/// Returns number of successful play triggers. Skips blank SoundUsage.
pub fn play_emot_object_sounds(
    bank: &mut SoundBank,
    content: &ClientContent,
    emotion: &crate::emotion::Emotion,
    map_x: f32,
    map_y: f32,
    creation: bool,
) -> usize {
    let mut n = 0usize;
    for id in emotion.object_slots() {
        if id <= 0 {
            continue;
        }
        let Some(obj) = content.get(id) else {
            continue;
        };
        let usage = if creation {
            obj.creation_sound.as_str()
        } else {
            obj.decay_sound.as_str()
        };
        if play_object_event_sound_at(bank, usage, map_x, map_y) {
            n += 1;
        }
    }
    n
}

/// Play creation sounds for PE apply targets `(player_id, emot_index, x, y)`.
pub fn play_emot_creation_for_targets(
    bank: &mut SoundBank,
    content: &ClientContent,
    emotions: &crate::emotion::EmotionBank,
    targets: &[(i32, i32, f32, f32)],
) -> usize {
    let mut n = 0usize;
    for &(_pid, emot_idx, mx, my) in targets {
        if let Some(em) = emotions.get(emot_idx) {
            n += play_emot_object_sounds(bank, content, em, mx, my, true);
        }
    }
    n
}

/// Play decay sounds for PE clear targets.
pub fn play_emot_decay_for_targets(
    bank: &mut SoundBank,
    content: &ClientContent,
    emotions: &crate::emotion::EmotionBank,
    targets: &[(i32, i32, f32, f32)],
) -> usize {
    let mut n = 0usize;
    for &(_pid, emot_idx, mx, my) in targets {
        if let Some(em) = emotions.get(emot_idx) {
            n += play_emot_object_sounds(bank, content, em, mx, my, false);
        }
    }
    n
}

/// Spatial creation when [`should_creation_sound_play`] allows.
pub fn play_creation_sound_at_if(
    bank: &mut SoundBank,
    content: &ClientContent,
    old_id: i32,
    new_id: i32,
    map_x: f32,
    map_y: f32,
) -> bool {
    if !should_creation_sound_play(content, old_id, new_id) {
        return false;
    }
    let Some(obj) = content.get(new_id) else {
        return false;
    };
    play_object_event_sound_at(bank, &obj.creation_sound, map_x, map_y)
}

/// Apply one MX sound suite (creation / decay / floor / contained fill / contained slot).
///
/// // C++ LivingLifePage MX ~16812–17364 + container fill ~17173
/// Returns number of successful play triggers recorded.
pub fn play_mx_change_sounds(
    bank: &mut SoundBank,
    content: &ClientContent,
    ctx: &MxSoundContext,
) -> usize {
    let mut n = 0usize;
    let mx = ctx.map_x as f32;
    let my = ctx.map_y as f32;

    // Floor creation (C++ ~17134–17144).
    if ctx.new_floor_id > 0 && ctx.new_floor_id != ctx.old_floor_id {
        if play_creation_sound_at_if(bank, content, ctx.old_floor_id, ctx.new_floor_id, mx, my) {
            n += 1;
        }
    }

    // Object id changed → creation / decay (C++ ~17293–17365).
    if ctx.new_object_id > 0 && ctx.new_object_id != ctx.old_object_id {
        // Prior session path always tried creation on id change; keep that.
        if play_creation_sound_at_if(
            bank,
            content,
            ctx.old_object_id,
            ctx.new_object_id,
            mx,
            my,
        ) {
            n += 1;
        }
    } else if ctx.old_object_id > 0 && ctx.new_object_id == 0 {
        // Decay when cleared (C++ auto-decay path; we fire whenever map clears).
        if let Some(def) = content.get(ctx.old_object_id) {
            if play_object_event_sound_at(bank, &def.decay_sound, mx, my) {
                n += 1;
            }
        }
    }

    // Contained fill: same root, more slots, responsible hands empty (pid > 0).
    // // C++ ~17173–17210
    if ctx.old_object_id > 0
        && ctx.old_object_id == ctx.new_object_id
        && ctx.new_contained.len() > ctx.old_contained.len()
        && ctx.player_id > 0
        && ctx.responsible_held == 0
    {
        if play_container_fill_using_sound(
            bank,
            content,
            ctx.new_object_id,
            ctx.responsible_display,
            mx,
            my,
        ) {
            n += 1;
        }
    }

    // Contained single-slot change (transform pid < 0, not moving, same count).
    // // C++ ~16792–16901
    if !ctx.is_moving
        && ctx.old_object_id > 0
        && ctx.old_object_id == ctx.new_object_id
        && ctx.player_id < 0
    {
        if let Some(i) =
            single_contained_change_index(&ctx.old_contained, &ctx.new_contained)
        {
            let old_c = ctx.old_contained[i];
            let new_c = ctx.new_contained[i];
            let held = if ctx.player_id < -1 {
                ctx.causing_held
            } else {
                // auto (-1): treat as empty hands for fill-up path
                0
            };
            if play_contained_slot_change_sound(bank, content, old_c, new_c, held, mx, my) {
                n += 1;
            }
        }
    }

    n
}

/// C++ `shouldCreationSoundPlay` (LivingLifePage ~12971).
///
/// Suppresses creation sound for no-op id, blank creation, multi-use dummy
/// parent cycles, and `creationSoundInitialOnly` sprite-subset loops.
pub fn should_creation_sound_play(
    content: &ClientContent,
    old_id: i32,
    new_id: i32,
) -> bool {
    if old_id == new_id {
        return false;
    }
    let Some(obj) = content.get(new_id) else {
        return false;
    };
    if sound_usage_is_blank(&obj.creation_sound) {
        return false;
    }
    if same_use_dummy_parent(content, old_id, new_id) {
        return false;
    }
    // !creationSoundInitialOnly || old empty || not a sprite subset cycle
    if !obj.creation_sound_initial_only
        || old_id <= 0
        || !is_sprite_subset(content, old_id, new_id)
    {
        return true;
    }
    false
}

/// Play new object's creation sound when [`should_creation_sound_play`] allows.
pub fn play_creation_sound_if(
    bank: &mut SoundBank,
    content: &ClientContent,
    old_id: i32,
    new_id: i32,
) -> bool {
    if !should_creation_sound_play(content, old_id, new_id) {
        return false;
    }
    let Some(obj) = content.get(new_id) else {
        return false;
    };
    play_object_event_sound(bank, &obj.creation_sound)
}

/// C++ `getClothingAdded` lite — first slot where `after` has a different non-zero
/// object id than `before`. Returns that clothing object id (0 if none).
pub fn clothing_added_id(before: &crate::live_object::ClothingSet, after: &crate::live_object::ClothingSet) -> i32 {
    for i in 0..6 {
        let a = after.slot_id(i);
        let b = before.slot_id(i);
        if a > 0 && a != b {
            return a;
        }
    }
    0
}

/// Clothing equip/remove sound (C++ PU ~18372–18416).
///
/// Prefers the added/removed clothing object's `usingSound`; falls back to
/// the person's `usingSound`.
pub fn play_clothing_change_sound(
    bank: &mut SoundBank,
    content: &ClientContent,
    before: &crate::live_object::ClothingSet,
    after: &crate::live_object::ClothingSet,
    person_display_id: i32,
) -> bool {
    let mut clothing_id = clothing_added_id(before, after);
    if clothing_id <= 0 {
        // removal: swapped args
        clothing_id = clothing_added_id(after, before);
    }
    if clothing_id <= 0 {
        return false;
    }
    if let Some(def) = content.get(clothing_id) {
        if play_object_event_sound(bank, &def.using_sound) {
            return true;
        }
    }
    if person_display_id > 0 {
        if let Some(person) = content.get(person_display_id) {
            return play_object_event_sound(bank, &person.using_sound);
        }
    }
    false
}

/// Clothing bag insertion sound (C++ PU ~19400–19451).
///
/// When any clothing slot gains more contained items (`slot,a,b` wire), play that
/// clothing object's `usingSound`, else person `usingSound`. **P2#13** companion
/// to map container fill.
pub fn play_clothing_contained_fill_sound(
    bank: &mut SoundBank,
    content: &ClientContent,
    before: &crate::live_object::ClothingSet,
    after: &crate::live_object::ClothingSet,
    person_display_id: i32,
    map_x: f32,
    map_y: f32,
) -> bool {
    use crate::client_map::parse_object_raw_contained;
    for i in 0..6 {
        let old_n = before
            .slots
            .get(i)
            .map(|s| parse_object_raw_contained(s).len())
            .unwrap_or(0);
        let new_n = after
            .slots
            .get(i)
            .map(|s| parse_object_raw_contained(s).len())
            .unwrap_or(0);
        if new_n <= old_n {
            continue;
        }
        let clothing_id = after.slot_id(i);
        if clothing_id <= 0 {
            continue;
        }
        return play_container_fill_using_sound(
            bank,
            content,
            clothing_id,
            person_display_id,
            map_x,
            map_y,
        );
    }
    false
}

/// Count top-level contained ids in a clothing slot wire field (`id` or `id,a,b`).
///
/// // C++: clothingContained vector size (outer clothing id not counted).
pub fn clothing_slot_contained_count(slot_raw: &str) -> usize {
    use crate::client_map::parse_object_raw_contained;
    parse_object_raw_contained(slot_raw).len()
}

/// Drop / baby put-down settle (C++ held→0 PU + baby heldByDropOffset land ~5230).
///
/// - Held object → empty hand: try held `usingSound`, else person `usingSound`
/// - Baby put-down (`old_held < 0`): baby's person `usingSound`
pub fn play_drop_settle_sound(
    bank: &mut SoundBank,
    content: &ClientContent,
    old_held: i32,
    new_held: i32,
    person_display_id: i32,
) -> bool {
    if old_held == new_held {
        return false;
    }
    // Baby put-down: held was -babyId, now free (or holding something else).
    if old_held < 0 && new_held >= 0 {
        // C++ plays baby's display using when drop offset settles — use baby id.
        let baby_id = -old_held;
        // Baby's display object is the person type; we only have live player id here.
        // Prefer person_display of the adult (caller may pass baby display if known).
        // Headless: play person_display_id using as stand-in when baby object unknown.
        let _ = baby_id;
        if person_display_id > 0 {
            if let Some(p) = content.get(person_display_id) {
                return play_object_event_sound(bank, &p.using_sound);
            }
        }
        return false;
    }
    // Dropped a held object onto ground / into container (hands empty).
    if old_held > 0 && new_held == 0 {
        if let Some(held) = content.get(old_held) {
            if play_object_event_sound(bank, &held.using_sound) {
                return true;
            }
        }
        if person_display_id > 0 {
            if let Some(p) = content.get(person_display_id) {
                return play_object_event_sound(bank, &p.using_sound);
            }
        }
    }
    false
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal synthetic mono-16 AIFF (Haxe layout).
    fn synth_aiff(num_samples: u32, sample_rate: u16, samples: &[i16]) -> Vec<u8> {
        assert_eq!(samples.len(), num_samples as usize);
        let mut data = vec![0u8; AIFF_SAMPLE_START + samples.len() * 2];
        // FORM....AIFFCOMM.... (not fully valid IFF; Haxe only reads fixed offsets)
        data[0..4].copy_from_slice(b"FORM");
        data[8..12].copy_from_slice(b"AIFF");
        // numChannels BE at 20
        data[20] = 0;
        data[21] = 1;
        // numSamples BE at 22
        let ns = num_samples.to_be_bytes();
        data[22..26].copy_from_slice(&ns);
        // bits BE at 26
        data[26] = 0;
        data[27] = 16;
        // sampleRate Haxe shortcut at 30-31
        data[30] = (sample_rate >> 8) as u8;
        data[31] = (sample_rate & 0xff) as u8;
        let mut b = AIFF_SAMPLE_START;
        for &s in samples {
            let be = s.to_be_bytes();
            data[b] = be[0];
            data[b + 1] = be[1];
            b += 2;
        }
        data
    }

    #[test]
    fn aiff_mono16_decode_be_to_i16() {
        let samples = [0i16, 1, -1, 256, -32768, 32767];
        let bytes = synth_aiff(6, 44100, &samples);
        let pcm = read_mono16_aiff(&bytes).unwrap();
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.samples, samples);
    }

    #[test]
    fn aiff_rejects_stereo() {
        let mut bytes = synth_aiff(2, 44100, &[0, 0]);
        bytes[21] = 2; // stereo
        assert!(read_mono16_aiff(&bytes).unwrap_err().contains("mono"));
    }

    #[test]
    fn aiff_rejects_8bit() {
        let mut bytes = synth_aiff(2, 44100, &[0, 0]);
        bytes[27] = 8;
        assert!(read_mono16_aiff(&bytes).unwrap_err().contains("16-bit"));
    }

    #[test]
    fn aiff_too_short() {
        assert!(read_mono16_aiff(&[0u8; 20]).is_err());
    }

    #[test]
    fn sound_usage_parse_and_print() {
        let u = parse_sound_usage("621:0.250000#622:0.500000");
        assert_eq!(u.subs.len(), 2);
        assert_eq!(u.subs[0].id, 621);
        assert!((u.subs[0].volume - 0.25).abs() < 1e-5);
        assert_eq!(u.subs[1].id, 622);
        let blank = parse_sound_usage("-1:0.0");
        assert!(blank.is_blank());
        assert_eq!(blank.print(), "-1:0.0");
        assert!(parse_sound_usage("").is_blank());
        let one = parse_sound_usage("100:1.0");
        assert_eq!(one.play_random().unwrap().id, 100);
    }

    #[test]
    fn sound_usage_rejects_bad_vol() {
        let u = parse_sound_usage("1:1.5#2:0.5");
        assert_eq!(u.subs.len(), 1);
        assert_eq!(u.subs[0].id, 2);
    }

    #[test]
    fn olsn_roundtrip() {
        let entries = vec![
            SoundIndexEntry {
                id: 10,
                sample_rate: 44100,
                num_samples: 100,
                rel_path: "sounds/10.aiff".into(),
                flags: OLSN_F_MONO16_VERIFIED | OLSN_F_HEADER_PEEKED,
            },
            SoundIndexEntry {
                id: 20,
                sample_rate: 0,
                num_samples: 0,
                rel_path: "sounds/20.ogg".into(),
                flags: OLSN_F_IS_OGG,
            },
        ];
        let bytes = write_olsn(&entries, 437);
        assert_eq!(&bytes[0..4], b"OLSN");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            OLSN_FORMAT_VERSION
        );
        let (ver, loaded) = load_olsn(&bytes).unwrap();
        assert_eq!(ver, 437);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, 10);
        assert_eq!(loaded[0].sample_rate, 44100);
        assert_eq!(loaded[0].rel_path, "sounds/10.aiff");
        assert!(loaded[1].is_ogg());
    }

    #[test]
    fn ensure_pcm_and_play_usage_stub() {
        let mut bank = SoundBank::new(".");
        let samples = [100i16, -100, 200, -200];
        let bytes = synth_aiff(4, 22050, &samples);
        let pcm = bank.ensure_pcm(99, &bytes).unwrap();
        assert_eq!(pcm.num_samples(), 4);
        assert_eq!(pcm.sample_rate, 22050);
        assert!(bank.play_usage("99:1.0"));
        assert!(!bank.play_usage("-1:0.0"));
        assert!(!bank.play_usage("12345:1.0")); // missing
    }

    /// P3#16 residual: mCurseSound trigger on successful isCurse (lazy path AIFF).
    #[test]
    fn play_curse_sound_at_records_lazy_path() {
        let mut bank = SoundBank::new(".");
        bank.set_listener(0.0, 0.0);
        // Boot: no AIFF opens.
        assert_eq!(bank.aiff_opens, 0);
        assert!(bank.play_curse_sound_at(0.0, 0.0));
        assert!(
            bank.last_played
                .iter()
                .any(|s| s.starts_with(CURSE_CHIME_REL)),
            "expected curse chime in last_played, got {:?}",
            bank.last_played
        );
        // Still no open if file absent (soft record path).
        assert_eq!(bank.path_pcm.len(), 0);

        // Inject PCM → lazy path cache; second play uses ensure without extra opens
        // only if we use ensure_path_pcm (no aiff_opens).
        let samples = [500i16, -500, 1000, -1000];
        let bytes = synth_aiff(4, 22050, &samples);
        assert!(bank.ensure_path_pcm(CURSE_CHIME_REL, &bytes).is_some());
        assert_eq!(bank.aiff_opens, 0);
        bank.clear_last_played();
        assert!(bank.play_curse_sound_at(1.0, 0.0));
        assert!(bank
            .last_played
            .iter()
            .any(|s| s.contains(CURSE_CHIME_REL) && s.contains("|pan=")));
    }

    #[test]
    fn mix_voices_mono_to_stereo_volume() {
        // Full-scale with equal L/R gains 0.5 → ±0.5 both channels.
        let samples = std::sync::Arc::new(vec![32767i16, -32768]);
        let mut voices = vec![MixVoice {
            samples,
            src_rate: 2,
            left_gain: 0.5,
            right_gain: 0.5,
            pos: 0.0,
        }];
        // device_rate == src_rate → one src sample per frame
        let mut out = [0.0f32; 4]; // 2 frames × 2 ch
        mix_voices_f32(&mut out, 2, &mut voices, 2);
        assert!((out[0] - 0.5).abs() < 0.01, "L0 {}", out[0]);
        assert!((out[1] - 0.5).abs() < 0.01, "R0 {}", out[1]);
        assert!((out[2] + 0.5).abs() < 0.01, "L1 {}", out[2]);
        assert!((out[3] + 0.5).abs() < 0.01, "R1 {}", out[3]);
        assert!(voices.is_empty(), "voice finished");
    }

    #[test]
    fn mix_voices_stereo_pan_left_right() {
        let samples = std::sync::Arc::new(vec![32767i16]);
        // Hard left
        let mut voices = vec![MixVoice {
            samples: samples.clone(),
            src_rate: 1,
            left_gain: 1.0,
            right_gain: 0.0,
            pos: 0.0,
        }];
        let mut out = [0.0f32; 2];
        mix_voices_f32(&mut out, 2, &mut voices, 1);
        assert!((out[0] - 1.0).abs() < 0.02, "L {}", out[0]);
        assert!(out[1].abs() < 0.02, "R {}", out[1]);
        // Hard right
        let mut voices = vec![MixVoice {
            samples,
            src_rate: 1,
            left_gain: 0.0,
            right_gain: 1.0,
            pos: 0.0,
        }];
        mix_voices_f32(&mut out, 2, &mut voices, 1);
        assert!(out[0].abs() < 0.02, "L {}", out[0]);
        assert!((out[1] - 1.0).abs() < 0.02, "R {}", out[1]);
    }

    #[test]
    fn mix_voices_resample_half_rate() {
        // src 2 Hz, device 4 Hz → each sample held ~2 frames (linear between).
        let samples = std::sync::Arc::new(vec![0i16, 32767]);
        let mut voices = vec![MixVoice::centered(samples, 2, 1.0)];
        let mut out = [0.0f32; 8]; // 4 frames mono
        mix_voices_f32(&mut out, 1, &mut voices, 4);
        assert!(out[0].abs() < 0.01);
        // mid-way toward full scale — mono mixes avg(L,R); center gains ≈ 0.707 each
        // so peak mono ≈ 0.707; mid-way between 0 and peak
        assert!(out[1] > 0.15 && out[1] < 0.6, "interp {}", out[1]);
        assert!(out[2] > 0.6, "near peak {}", out[2]);
    }

    #[test]
    fn get_vector_from_camera_tiles() {
        let (vx, vy) = get_vector_from_camera(10.0, 4.0, 5.0, 4.0);
        assert!((vx - 5.0).abs() < 1e-5);
        assert!(vy.abs() < 1e-5);
    }

    #[test]
    fn volume_pan_center_and_edges() {
        // On-camera: full-ish volume, pan near center (8/16 = 0.5).
        let p = volume_pan_reverb(0.0, 0.0).expect("audible");
        assert!(p.volume > 0.9, "vol {}", p.volume);
        assert!((p.pan - 0.5).abs() < 0.02, "pan {}", p.pan);
        assert!((p.reverb_mix - REVERB_CONSTANT).abs() < 0.05);

        // Right of camera: pan > 0.5
        let right = volume_pan_reverb(5.0, 0.0).unwrap();
        assert!(right.pan > 0.7, "pan {}", right.pan);
        // Left
        let left = volume_pan_reverb(-5.0, 0.0).unwrap();
        assert!(left.pan < 0.3, "pan {}", left.pan);

        // Beyond max: silent
        assert!(volume_pan_reverb(MAX_AUDIBLE_DISTANCE + 1.0, 0.0).is_none());
    }

    #[test]
    fn constant_power_center_and_sides() {
        let (l, r) = stereo_gains_constant_power(1.0, 0.5);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        assert!((l - half).abs() < 1e-4);
        assert!((r - half).abs() < 1e-4);
        let (l0, r0) = stereo_gains_constant_power(1.0, 0.0);
        assert!((l0 - 1.0).abs() < 1e-4 && r0.abs() < 1e-4);
        let (l1, r1) = stereo_gains_constant_power(1.0, 1.0);
        assert!(l1.abs() < 1e-4 && (r1 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn play_usage_at_respects_distance() {
        let mut bank = SoundBank::new(".");
        bank.set_listener(0.0, 0.0);
        // Logic-only bank: far sound skipped
        assert!(!bank.play_usage_at("1:1.0", 100.0, 0.0));
        assert!(bank.last_played.is_empty());
        // Near sound records
        assert!(bank.play_usage_at("1:1.0", 1.0, 0.0));
        assert_eq!(bank.last_played.len(), 1);
    }

    #[test]
    fn play_pcm_samples_rejects_empty() {
        assert!(!play_pcm_samples(&[], 44100, 1.0));
        assert!(!play_pcm_samples(&[1], 0, 1.0));
    }

    #[test]
    fn play_id_queues_when_pcm_present() {
        let mut bank = SoundBank::new(".");
        let samples = [1000i16, -1000, 500, -500];
        let bytes = synth_aiff(4, 44100, &samples);
        assert!(bank.ensure_pcm(7, &bytes).is_some());
        // Without audio feature: true after ensure. With audio: queue (or silent).
        assert!(bank.play_id(7, 0.8));
        assert!(!bank.play_id(-1, 1.0));
        assert!(!bank.play_id(99999, 1.0));
        assert!(audio_feature_enabled() == cfg!(feature = "audio"));
    }

    #[test]
    fn ensure_ogg_returns_none() {
        let mut bank = SoundBank::new(".");
        bank.index.insert(
            5,
            SoundIndexEntry {
                id: 5,
                sample_rate: 0,
                num_samples: 0,
                rel_path: "sounds/5.ogg".into(),
                flags: OLSN_F_IS_OGG,
            },
        );
        bank.index_loaded = true;
        assert!(bank.ensure(5).is_none());
        assert!(bank.is_missing(5));
        assert_eq!(bank.aiff_opens, 0);
    }

    #[test]
    fn should_creation_sound_play_gates() {
        use crate::content::{ClientContent, ClientObjectDef, ObjectSprite};

        let mut content = ClientContent::new();
        // Base object with creation sound
        content.objects.insert(
            10,
            ClientObjectDef {
                id: 10,
                creation_sound: "1:1.0".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 100,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // Same parent dummies
        content.objects.insert(
            11,
            ClientObjectDef {
                id: 11,
                creation_sound: "2:1.0".into(),
                dummy_parent: 10,
                num_uses: 0,
                sprites: vec![ObjectSprite {
                    sprite_id: 100,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            12,
            ClientObjectDef {
                id: 12,
                creation_sound: "3:1.0".into(),
                dummy_parent: 10,
                sprites: vec![ObjectSprite {
                    sprite_id: 100,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // initialOnly object that is sprite-subset of 10
        content.objects.insert(
            20,
            ClientObjectDef {
                id: 20,
                creation_sound: "4:1.0".into(),
                creation_sound_initial_only: true,
                sprites: vec![ObjectSprite {
                    sprite_id: 100,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // Fresh object with unique sprites
        content.objects.insert(
            30,
            ClientObjectDef {
                id: 30,
                creation_sound: "5:1.0".into(),
                creation_sound_initial_only: true,
                sprites: vec![ObjectSprite {
                    sprite_id: 999,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            40,
            ClientObjectDef {
                id: 40,
                creation_sound: String::new(),
                ..Default::default()
            },
        );

        assert!(!should_creation_sound_play(&content, 10, 10)); // same id
        assert!(!should_creation_sound_play(&content, 0, 40)); // blank creation
        assert!(should_creation_sound_play(&content, 0, 10)); // empty → new
        assert!(!same_use_dummy_parent(&content, 10, 30));
        assert!(same_use_dummy_parent(&content, 11, 12));
        assert!(!should_creation_sound_play(&content, 11, 12)); // same dummy parent
        // initialOnly + sprite subset of old → suppress
        assert!(is_sprite_subset(&content, 10, 20));
        assert!(!should_creation_sound_play(&content, 10, 20));
        // initialOnly but not subset → play
        assert!(should_creation_sound_play(&content, 10, 30));
        // initialOnly with old empty → play
        assert!(should_creation_sound_play(&content, 0, 20));
    }

    #[test]
    fn pe_emot_creation_and_decay_sounds() {
        use crate::content::{ClientContent, ClientObjectDef};
        use crate::emotion::EmotionBank;

        let mut content = ClientContent::new();
        content.objects.insert(
            9001,
            ClientObjectDef {
                id: 9001,
                creation_sound: "77:1.0".into(),
                decay_sound: "78:1.0".into(),
                ..Default::default()
            },
        );
        let emotions = EmotionBank::from_ini_strings(
            "/happy\n",
            "0 9001 0 0 0 0\n", // mouthEmot = 9001
        );
        let mut bank = SoundBank::new(".");
        bank.set_listener(0.0, 0.0);

        let em = emotions.get(0).unwrap();
        assert_eq!(
            play_emot_object_sounds(&mut bank, &content, em, 0.0, 0.0, true),
            1
        );
        assert!(
            bank.last_played.iter().any(|s| s.starts_with("77:")),
            "creation: {:?}",
            bank.last_played
        );
        bank.clear_last_played();
        assert_eq!(
            play_emot_object_sounds(&mut bank, &content, em, 0.0, 0.0, false),
            1
        );
        assert!(
            bank.last_played.iter().any(|s| s.starts_with("78:")),
            "decay: {:?}",
            bank.last_played
        );

        // Batch helpers
        bank.clear_last_played();
        let n = play_emot_creation_for_targets(
            &mut bank,
            &content,
            &emotions,
            &[(1, 0, 0.0, 0.0)],
        );
        assert_eq!(n, 1);
        let n = play_emot_decay_for_targets(
            &mut bank,
            &content,
            &emotions,
            &[(1, 0, 0.0, 0.0)],
        );
        assert_eq!(n, 1);

        // Permanent ttl=-2 silent path is covered in LiveObject apply_emot.
    }

    #[test]
    fn clothing_and_drop_settle_play() {
        use crate::content::{ClientContent, ClientObjectDef};
        use crate::live_object::ClothingSet;

        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                using_sound: "10:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            200,
            ClientObjectDef {
                id: 200,
                using_sound: "20:1.0".into(),
                clothing: 'h',
                ..Default::default()
            },
        );
        content.objects.insert(
            300,
            ClientObjectDef {
                id: 300,
                using_sound: "30:1.0".into(),
                ..Default::default()
            },
        );

        let mut bank = SoundBank::new(".");
        let before = ClothingSet::parse("0;0;0;0;0;0");
        let after = ClothingSet::parse("200;0;0;0;0;0");
        assert_eq!(clothing_added_id(&before, &after), 200);
        assert!(play_clothing_change_sound(
            &mut bank, &content, &before, &after, 19
        ));
        assert!(
            bank.last_played.iter().any(|s| s.contains("20:")),
            "clothing using fired: {:?}",
            bank.last_played
        );

        bank.last_played.clear();
        assert!(play_drop_settle_sound(
            &mut bank, &content, 300, 0, 19
        ));
        assert!(
            bank.last_played.iter().any(|s| s.contains("30:")),
            "held using on drop: {:?}",
            bank.last_played
        );

        bank.last_played.clear();
        // Baby put-down uses person display using
        assert!(play_drop_settle_sound(
            &mut bank, &content, -5, 0, 19
        ));
        assert!(
            bank.last_played.iter().any(|s| s.contains("10:")),
            "baby put-down person using: {:?}",
            bank.last_played
        );
    }

    #[test]
    fn mx_container_fill_using_and_contained_slot_using_on_fill() {
        use crate::content::{ClientContent, ClientObjectDef};

        let mut content = ClientContent::new();
        // Basket container
        content.objects.insert(
            100,
            ClientObjectDef {
                id: 100,
                using_sound: "50:1.0".into(),
                num_slots: 4,
                ..Default::default()
            },
        );
        // Person fallback
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                using_sound: "51:1.0".into(),
                ..Default::default()
            },
        );
        // Multi-use parent + dummies (dummy_ids[0]=most used, [1]=less used)
        content.objects.insert(
            200,
            ClientObjectDef {
                id: 200,
                num_uses: 3,
                dummy_ids: vec![201, 202],
                using_sound: "60:1.0".into(),
                creation_sound: "61:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            201,
            ClientObjectDef {
                id: 201,
                dummy_parent: 200,
                using_sound: "62:1.0".into(),
                creation_sound: String::new(),
                ..Default::default()
            },
        );
        content.objects.insert(
            202,
            ClientObjectDef {
                id: 202,
                dummy_parent: 200,
                using_sound: "63:1.0".into(),
                creation_sound: String::new(),
                ..Default::default()
            },
        );
        content.dummy_parent.insert(201, 200);
        content.dummy_parent.insert(202, 200);

        let mut bank = SoundBank::new(".");
        bank.set_listener(0.0, 0.0);

        // ── same root + more contained + hands empty → container using ──
        let fill_ctx = MxSoundContext {
            old_object_id: 100,
            old_floor_id: 0,
            old_contained: vec![],
            new_object_id: 100,
            new_floor_id: 0,
            new_contained: vec![33],
            player_id: 5,
            is_moving: false,
            map_x: 2,
            map_y: 0,
            responsible_held: 0,
            responsible_display: 19,
            causing_held: 0,
        };
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &fill_ctx), 1);
        assert!(
            bank.last_played.iter().any(|s| s.starts_with("50:")),
            "container using on fill: {:?}",
            bank.last_played
        );
        assert!(bank.last_pan > 0.5, "spatial pan right of listener");

        // Hands not empty → no fill using
        bank.clear_last_played();
        let mut no_fill = fill_ctx.clone();
        no_fill.responsible_held = 99;
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &no_fill), 0);

        // ── single contained slot: multi-use fill-up (201 → 202, hands empty) ──
        bank.clear_last_played();
        let slot_ctx = MxSoundContext {
            old_object_id: 100,
            old_floor_id: 0,
            old_contained: vec![201],
            new_object_id: 100,
            new_floor_id: 0,
            new_contained: vec![202],
            player_id: -7, // transform by player 7
            is_moving: false,
            map_x: 0,
            map_y: 0,
            responsible_held: 0,
            responsible_display: 0,
            causing_held: 0,
        };
        assert!(both_same_use_parent(&content, 202, 201));
        assert!(is_less_used_than(&content, 201, 202));
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &slot_ctx), 1);
        assert!(
            bank.last_played.iter().any(|s| s.starts_with("63:")),
            "using-on-fill for less-used dummy: {:?}",
            bank.last_played
        );

        // Depleting (202 → 201) should not play using-on-fill (and no creation)
        bank.clear_last_played();
        let deplete = MxSoundContext {
            old_contained: vec![202],
            new_contained: vec![201],
            ..slot_ctx.clone()
        };
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &deplete), 0);
    }

    #[test]
    fn off_screen_sound_register_and_skip_self() {
        let mut bank = SoundBank::new(".");
        maybe_register_off_screen_sound(
            &mut bank,
            42,
            Some(1),
            "Drum offScreenSound_red",
            10.0,
            20.0,
        );
        assert_eq!(bank.last_off_screen.len(), 1);
        assert!(bank.last_off_screen[0].red);
        assert_eq!(bank.last_off_screen[0].source_player_id, 42);

        // Self source: skip
        bank.clear_last_off_screen();
        maybe_register_off_screen_sound(
            &mut bank,
            1,
            Some(1),
            "Drum offScreenSound",
            0.0,
            0.0,
        );
        assert!(bank.last_off_screen.is_empty());

        // No tag: skip
        maybe_register_off_screen_sound(&mut bank, 9, Some(1), "Plain rock", 0.0, 0.0);
        assert!(bank.last_off_screen.is_empty());

        let (red, ch) = parse_off_screen_sound_flags("Bell offScreenSound_Q");
        assert!(!red);
        assert_eq!(ch, Some('Q'));
    }

    #[test]
    fn clothing_contained_fill_using() {
        use crate::content::{ClientContent, ClientObjectDef};
        use crate::live_object::ClothingSet;

        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                using_sound: "10:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            500,
            ClientObjectDef {
                id: 500,
                using_sound: "70:1.0".into(),
                clothing: 'p',
                num_slots: 4,
                ..Default::default()
            },
        );

        let mut bank = SoundBank::new(".");
        let before = ClothingSet::parse("0;0;0;0;0;500");
        let after = ClothingSet::parse("0;0;0;0;0;500,33");
        assert_eq!(clothing_slot_contained_count("500"), 0);
        assert_eq!(clothing_slot_contained_count("500,33"), 1);
        assert!(play_clothing_contained_fill_sound(
            &mut bank, &content, &before, &after, 19, 1.0, 0.0
        ));
        assert!(
            bank.last_played.iter().any(|s| s.starts_with("70:")),
            "backpack using on contained fill: {:?}",
            bank.last_played
        );
    }

    #[test]
    fn load_prefer_cache_zero_aiff_opens() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsn_pref_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sounds")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "7\n").unwrap();
        // Write a tiny synth aiff as 42.aiff
        let samples = [1i16, 2, 3, 4];
        let aiff = synth_aiff(4, 44100, &samples);
        fs::write(tmp.join("sounds").join("42.aiff"), &aiff).unwrap();
        // Fake ogg (should index, not decode)
        fs::write(tmp.join("sounds").join("99.ogg"), b"OggSfake").unwrap();

        let bank = SoundBank::load_prefer_cache(&tmp);
        assert!(bank.index_loaded);
        assert!(bank.index.contains_key(&42));
        assert!(bank.index.contains_key(&99));
        assert_eq!(bank.aiff_opens, 0, "boot must not open AIFF for PCM");
        assert_eq!(bank.pcm_count(), 0);
        assert!(tmp.join("cache").join("olsn_sounds.bin").exists());

        let mut bank = bank;
        let pcm = bank.ensure(42).unwrap();
        assert_eq!(pcm.samples, samples);
        assert_eq!(bank.aiff_opens, 1);
        assert!(bank.ensure(99).is_none()); // ogg

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn footstep_resolve_substitutes_floor() {
        let anim = "100:1.0";
        let floor = "200:0.5";
        assert_eq!(
            SoundBank::resolve_footstep_usage(anim, true, floor),
            floor
        );
        assert_eq!(
            SoundBank::resolve_footstep_usage(anim, false, floor),
            anim
        );
        assert_eq!(
            SoundBank::resolve_footstep_usage(anim, true, ""),
            anim
        );
        assert_eq!(
            SoundBank::resolve_footstep_usage(anim, true, "-1:0.0"),
            anim
        );
    }

    #[test]
    fn sound_param_period_gate_fires_on_period_cross() {
        use crate::anim_bank::SoundAnimParam;
        // hz = 2 → period 0.5s → 30 frames at 60fps
        let p = SoundAnimParam {
            usage: "10:1.0".into(),
            repeat_per_sec: 2.0,
            repeat_phase: 0.0,
            age_start: -1.0,
            age_end: -1.0,
            footstep: false,
        };
        // frames 0→1: time 0→1/60, still period 0
        assert!(!sound_param_should_play(&p, 20.0, 0.0, 1.0, 1.0));
        // frames 29→31: cross period boundary at t=0.5
        assert!(sound_param_should_play(&p, 20.0, 29.0, 31.0, 1.0));
        // phase 0.5: startOffset = 0.25s = 15 frames
        let p2 = SoundAnimParam {
            repeat_phase: 0.5,
            ..p.clone()
        };
        assert!(!sound_param_should_play(&p2, 20.0, 0.0, 10.0, 1.0));
        assert!(sound_param_should_play(&p2, 20.0, 14.0, 16.0, 1.0));
    }

    #[test]
    fn sound_param_age_gate_and_oneshot() {
        use crate::anim_bank::SoundAnimParam;
        let p = SoundAnimParam {
            usage: "11:1.0".into(),
            repeat_per_sec: 0.0,
            repeat_phase: 0.0,
            age_start: 2.0,
            age_end: 10.0,
            footstep: false,
        };
        assert!(!sound_param_should_play(&p, 1.0, 0.0, 1.0, 1.0)); // too young
        assert!(sound_param_should_play(&p, 5.0, 0.0, 1.0, 1.0)); // oneshot
        assert!(!sound_param_should_play(&p, 5.0, 1.0, 2.0, 1.0)); // already started
        assert!(!sound_param_should_play(&p, 10.0, 0.0, 1.0, 1.0)); // age_end exclusive
    }

    #[test]
    fn handle_anim_sound_footstep_uses_floor_using() {
        use crate::anim_bank::{AnimBank, ObjectAnimation, SoundAnimParam, ANIM_MOVING};
        use crate::client_map::{ClientMap, MapTile};
        use crate::content::{ClientContent, ClientObjectDef};

        let mut anims = AnimBank::new(".");
        let anim = ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_MOVING,
            sound_params: vec![SoundAnimParam {
                usage: "100:1.0".into(),
                repeat_per_sec: 2.0,
                repeat_phase: 0.0,
                age_start: -1.0,
                age_end: -1.0,
                footstep: true,
            }],
            ..Default::default()
        };
        anims.insert(anim);

        let mut content = ClientContent::new();
        content.objects.insert(
            500,
            ClientObjectDef {
                id: 500,
                floor: true,
                using_sound: "200:0.5".into(),
                ..Default::default()
            },
        );

        let mut map = ClientMap::new();
        map.set(
            3,
            4,
            MapTile {
                floor_id: 500,
                object_id: 0,
                ..MapTile::empty()
            },
        );

        let mut bank = SoundBank::new(".");
        // Cross period: old 29 → new 31 at hz=2
        let n = handle_anim_sound(
            &mut bank,
            &mut anims,
            &content,
            &map,
            19,
            20.0,
            ANIM_MOVING,
            29.0,
            31.0,
            3.0,
            4.0,
            1.0,
        );
        assert_eq!(n, 1);
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(
            last.starts_with("200:0.5"),
            "footstep→floor using with spatial pan log: {last:?}"
        );
        // Listener default (0,0), sound at (3,4) → pan > 0.5 (right of camera)
        assert!(bank.last_pan > 0.5, "pan {}", bank.last_pan);
    }

    #[test]
    fn bake_olsn_from_fixture() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_olsn_bake_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sounds")).unwrap();
        let aiff = synth_aiff(2, 44100, &[0, 1]);
        fs::write(tmp.join("sounds").join("7.aiff"), &aiff).unwrap();
        // skip txt
        fs::write(tmp.join("sounds").join("7.txt"), "author=x\n").unwrap();

        let (bytes, n) = bake_olsn_from_dir(&tmp, 3).unwrap();
        assert_eq!(n, 1);
        let (ver, ents) = load_olsn(&bytes).unwrap();
        assert_eq!(ver, 3);
        assert_eq!(ents[0].id, 7);
        assert!(ents[0].mono16_verified());
        assert_eq!(ents[0].sample_rate, 44100);
        assert_eq!(ents[0].num_samples, 2);
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── P2#13 contained MX / offScreenSound ──────────────────────────────────

    #[test]
    fn single_contained_change_index_picks_one() {
        assert_eq!(
            single_contained_change_index(&[1, 2, 3], &[1, 9, 3]),
            Some(1)
        );
        assert_eq!(single_contained_change_index(&[1, 2], &[1, 2]), None);
        assert_eq!(single_contained_change_index(&[1, 2], &[9, 8]), None);
        assert_eq!(single_contained_change_index(&[1], &[1, 2]), None);
    }

    #[test]
    fn off_screen_sound_flag_parse() {
        assert!(description_has_off_screen_sound("Bell offScreenSound"));
        let (red, ch) = parse_off_screen_sound_flags("Foo offScreenSound_red bar");
        assert!(red);
        assert!(ch.is_none());
        let (red2, ch2) = parse_off_screen_sound_flags("X offScreenSound_Q y");
        assert!(!red2);
        assert_eq!(ch2, Some('Q'));
    }

    #[test]
    fn mx_container_fill_plays_using_sound() {
        use crate::content::{ClientContent, ClientObjectDef};

        let mut content = ClientContent::new();
        content.objects.insert(
            100,
            ClientObjectDef {
                id: 100,
                using_sound: "501:1.0".into(),
                num_slots: 4,
                ..Default::default()
            },
        );
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                using_sound: "502:1.0".into(),
                person: 1,
                ..Default::default()
            },
        );

        let mut bank = SoundBank::new(".");
        bank.clear_last_played();
        let ctx = MxSoundContext {
            old_object_id: 100,
            old_floor_id: 0,
            old_contained: vec![33],
            new_object_id: 100,
            new_floor_id: 0,
            new_contained: vec![33, 40],
            player_id: 7,
            is_moving: false,
            map_x: 5,
            map_y: 6,
            responsible_held: 0,
            responsible_display: 19,
            causing_held: 0,
        };
        let n = play_mx_change_sounds(&mut bank, &content, &ctx);
        assert_eq!(n, 1, "container fill using should fire once");
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(
            last.starts_with("501:1.0"),
            "expected container using, got {last:?}"
        );
    }

    #[test]
    fn mx_container_fill_falls_back_to_person_using() {
        use crate::content::{ClientContent, ClientObjectDef};

        let mut content = ClientContent::new();
        content.objects.insert(
            100,
            ClientObjectDef {
                id: 100,
                using_sound: String::new(),
                num_slots: 4,
                ..Default::default()
            },
        );
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                using_sound: "502:0.5".into(),
                person: 1,
                ..Default::default()
            },
        );

        let mut bank = SoundBank::new(".");
        let ctx = MxSoundContext {
            old_object_id: 100,
            old_floor_id: 0,
            old_contained: vec![],
            new_object_id: 100,
            new_floor_id: 0,
            new_contained: vec![33],
            player_id: 7,
            is_moving: false,
            map_x: 0,
            map_y: 0,
            responsible_held: 0,
            responsible_display: 19,
            causing_held: 0,
        };
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &ctx), 1);
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(last.starts_with("502:0.5"), "got {last:?}");
    }

    #[test]
    fn mx_contained_slot_using_on_fill() {
        use crate::content::{ClientContent, ClientObjectDef};

        // Parent 200 (num_uses=3), dummies 201 (index 0, more used), 202 (index 1, less used)
        let mut content = ClientContent::new();
        content.objects.insert(
            200,
            ClientObjectDef {
                id: 200,
                num_uses: 3,
                dummy_ids: vec![201, 202],
                using_sound: "600:1.0".into(),
                creation_sound: "601:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            201,
            ClientObjectDef {
                id: 201,
                dummy_parent: 200,
                using_sound: "602:1.0".into(),
                creation_sound: String::new(),
                ..Default::default()
            },
        );
        content.objects.insert(
            202,
            ClientObjectDef {
                id: 202,
                dummy_parent: 200,
                using_sound: "603:1.0".into(),
                creation_sound: String::new(),
                ..Default::default()
            },
        );
        content.dummy_parent.insert(201, 200);
        content.dummy_parent.insert(202, 200);
        content.objects.insert(
            50,
            ClientObjectDef {
                id: 50,
                num_slots: 2,
                ..Default::default()
            },
        );

        assert!(both_same_use_parent(&content, 201, 202));
        assert!(is_less_used_than(&content, 201, 202)); // 201 index0 < 202 index1

        let mut bank = SoundBank::new(".");
        // Contained slot 201 → 202 (filling back), hands empty, transform pid
        let ctx = MxSoundContext {
            old_object_id: 50,
            old_floor_id: 0,
            old_contained: vec![201],
            new_object_id: 50,
            new_floor_id: 0,
            new_contained: vec![202],
            player_id: -7,
            is_moving: false,
            map_x: 2,
            map_y: 3,
            responsible_held: 0,
            responsible_display: 0,
            causing_held: 0,
        };
        let n = play_mx_change_sounds(&mut bank, &content, &ctx);
        assert_eq!(n, 1, "using-on-fill should fire");
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(
            last.starts_with("603:1.0"),
            "new cont using sound, got {last:?}"
        );
    }

    #[test]
    fn mx_contained_slot_creation_when_not_same_parent() {
        use crate::content::{ClientContent, ClientObjectDef};

        let mut content = ClientContent::new();
        content.objects.insert(
            10,
            ClientObjectDef {
                id: 10,
                creation_sound: "700:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            20,
            ClientObjectDef {
                id: 20,
                creation_sound: "701:1.0".into(),
                ..Default::default()
            },
        );
        content.objects.insert(
            50,
            ClientObjectDef {
                id: 50,
                num_slots: 1,
                ..Default::default()
            },
        );

        let mut bank = SoundBank::new(".");
        let ctx = MxSoundContext {
            old_object_id: 50,
            old_floor_id: 0,
            old_contained: vec![10],
            new_object_id: 50,
            new_floor_id: 0,
            new_contained: vec![20],
            player_id: -1,
            is_moving: false,
            map_x: 0,
            map_y: 0,
            responsible_held: 0,
            responsible_display: 0,
            causing_held: 0,
        };
        assert_eq!(play_mx_change_sounds(&mut bank, &content, &ctx), 1);
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(last.starts_with("701:1.0"), "got {last:?}");
    }

    #[test]
    fn off_screen_sound_registers_for_non_self() {
        use crate::anim_bank::{AnimBank, ObjectAnimation, SoundAnimParam, ANIM_GROUND};
        use crate::client_map::ClientMap;
        use crate::content::{ClientContent, ClientObjectDef};

        let mut anims = AnimBank::new(".");
        anims.insert(ObjectAnimation {
            object_id: 88,
            anim_type: ANIM_GROUND,
            sound_params: vec![SoundAnimParam {
                usage: "800:1.0".into(),
                repeat_per_sec: 0.0,
                repeat_phase: 0.0,
                age_start: -1.0,
                age_end: -1.0,
                footstep: false,
            }],
            ..Default::default()
        });
        let mut content = ClientContent::new();
        content.objects.insert(
            88,
            ClientObjectDef {
                id: 88,
                description: "Bell offScreenSound_red".into(),
                ..Default::default()
            },
        );
        let map = ClientMap::new();
        let mut bank = SoundBank::new(".");
        // oneshot: old 0 → new 1
        let n = handle_anim_sound_ex(
            &mut bank,
            &mut anims,
            &content,
            &map,
            88,
            0.0,
            ANIM_GROUND,
            0.0,
            1.0,
            10.0,
            12.0,
            1.0,
            42,       // other player
            Some(1),  // our id
        );
        assert_eq!(n, 1);
        assert_eq!(bank.last_off_screen.len(), 1);
        let ev = &bank.last_off_screen[0];
        assert_eq!(ev.source_player_id, 42);
        assert!(ev.red);
        assert!((ev.map_x - 10.0).abs() < 1e-5);
    }

    #[test]
    fn off_screen_sound_skips_self() {
        use crate::anim_bank::{AnimBank, ObjectAnimation, SoundAnimParam, ANIM_GROUND};
        use crate::client_map::ClientMap;
        use crate::content::{ClientContent, ClientObjectDef};

        let mut anims = AnimBank::new(".");
        anims.insert(ObjectAnimation {
            object_id: 88,
            anim_type: ANIM_GROUND,
            sound_params: vec![SoundAnimParam {
                usage: "800:1.0".into(),
                repeat_per_sec: 0.0,
                repeat_phase: 0.0,
                age_start: -1.0,
                age_end: -1.0,
                footstep: false,
            }],
            ..Default::default()
        });
        let mut content = ClientContent::new();
        content.objects.insert(
            88,
            ClientObjectDef {
                id: 88,
                description: "Bell offScreenSound".into(),
                ..Default::default()
            },
        );
        let map = ClientMap::new();
        let mut bank = SoundBank::new(".");
        let n = handle_anim_sound_ex(
            &mut bank,
            &mut anims,
            &content,
            &map,
            88,
            0.0,
            ANIM_GROUND,
            0.0,
            1.0,
            0.0,
            0.0,
            1.0,
            1,
            Some(1),
        );
        assert_eq!(n, 1);
        assert!(bank.last_off_screen.is_empty());
    }

    // ── P2#14 map ground anim sounds ─────────────────────────────────────────

    #[test]
    fn map_ground_anim_fires_period_sound() {
        use crate::anim_bank::{AnimBank, ObjectAnimation, SoundAnimParam, ANIM_GROUND};
        use crate::client_map::{ClientMap, MapTile};
        use crate::content::{ClientContent, ClientObjectDef};

        let mut anims = AnimBank::new(".");
        // hz=2 → period 0.5s = 30 frames
        anims.insert(ObjectAnimation {
            object_id: 77,
            anim_type: ANIM_GROUND,
            sound_params: vec![SoundAnimParam {
                usage: "900:1.0".into(),
                repeat_per_sec: 2.0,
                repeat_phase: 0.0,
                age_start: -1.0,
                age_end: -1.0,
                footstep: false,
            }],
            ..Default::default()
        });
        let mut content = ClientContent::new();
        content.objects.insert(
            77,
            ClientObjectDef {
                id: 77,
                description: "Campfire".into(),
                ..Default::default()
            },
        );
        let mut map = ClientMap::new();
        map.set(
            1,
            2,
            MapTile {
                object_id: 77,
                object_raw: "77".into(),
                ..MapTile::empty()
            },
        );
        let mut bank = SoundBank::new(".");
        // Step 31 frames at once so old=0 → new=31 crosses period at 30.
        let n = step_map_ground_anims_with_sounds(
            &mut bank,
            &mut anims,
            &content,
            &mut map,
            31.0,
            None,
        );
        assert_eq!(n, 1, "map ground period sound");
        let last = bank.last_played.last().map(|s| s.as_str()).unwrap_or("");
        assert!(last.starts_with("900:1.0"), "got {last:?}");
        assert!(
            (*map.anim_frame_count.get(&(1, 2)).unwrap() - 31.0).abs() < 1e-5
        );
    }

    #[test]
    fn map_floor_ground_anim_fires() {
        use crate::anim_bank::{AnimBank, ObjectAnimation, SoundAnimParam, ANIM_GROUND};
        use crate::client_map::{ClientMap, MapTile};
        use crate::content::{ClientContent, ClientObjectDef};

        let mut anims = AnimBank::new(".");
        anims.insert(ObjectAnimation {
            object_id: 55,
            anim_type: ANIM_GROUND,
            sound_params: vec![SoundAnimParam {
                usage: "910:1.0".into(),
                repeat_per_sec: 0.0,
                repeat_phase: 0.0,
                age_start: -1.0,
                age_end: -1.0,
                footstep: false,
            }],
            ..Default::default()
        });
        let mut content = ClientContent::new();
        content.objects.insert(
            55,
            ClientObjectDef {
                id: 55,
                floor: true,
                ..Default::default()
            },
        );
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            MapTile {
                floor_id: 55,
                ..MapTile::empty()
            },
        );
        let mut bank = SoundBank::new(".");
        // oneshot needs old_time==0 and new!=old → first step fires
        let n = step_map_ground_anims_with_sounds(
            &mut bank,
            &mut anims,
            &content,
            &mut map,
            1.0,
            None,
        );
        assert_eq!(n, 1);
        assert!(bank
            .last_played
            .last()
            .map(|s| s.starts_with("910:1.0"))
            .unwrap_or(false));
        // second step should not re-fire oneshot
        bank.clear_last_played();
        let n2 = step_map_ground_anims_with_sounds(
            &mut bank,
            &mut anims,
            &content,
            &mut map,
            1.0,
            None,
        );
        assert_eq!(n2, 0);
        assert!(bank.last_played.is_empty());
    }
}
