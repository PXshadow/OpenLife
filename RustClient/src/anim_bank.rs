//! Animation bank (C++ `animationBank` / Haxe `AnimationRecord`).
//!
//! - **Text:** `animations/{objId}_{type}.txt` and extras `{objId}_7x{i}.txt`
//! - **Binary cache:** OLA1 (`cache/ola1_anims.bin`) — full sprite/slot param tracks
//!
//! Layout follows `docs/port/CONTENT_BINARY.md` (same 24-byte OL* header as OLC1/OLS1).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// C++ `AnimType` numeric values (file names use these).
pub const ANIM_GROUND: i32 = 0;
pub const ANIM_HELD: i32 = 1;
pub const ANIM_MOVING: i32 = 2;
/// Runtime-only alias; C++ maps `ground2` → ground for lookup / draw.
pub const ANIM_GROUND2: i32 = 3;
pub const ANIM_EATING: i32 = 4;
pub const ANIM_DOING: i32 = 5;
/// C++ `extra` — gesture pack slot A (`setExtraIndex` + `7xN` file).
pub const ANIM_EXTRA: i32 = 7;
/// C++ `extraB` — gesture pack slot B (`setExtraIndexB`); same disk records as
/// `ANIM_EXTRA` but a separate runtime type so PE gestures can cross-fade.
pub const ANIM_EXTRA_B: i32 = 8;

/// OLA1 magic — animation bank cache.
pub const OLA1_MAGIC: &[u8; 4] = b"OLA1";
/// OLA1 format version 1 (legacy, no author trailer).
pub const OLA1_FORMAT_VERSION_V1: u32 = 1;
/// OLA1 format version 2 — adds per-record `authorTag` (C++ `author=`).
pub const OLA1_FORMAT_VERSION: u32 = 2;

// Record flags
const AF_RANDOM_START: u8 = 1 << 0;
const AF_FORCE_ZERO_START: u8 = 1 << 1;

/// One sprite/slot animation parameter track (C++ `SpriteAnimationRecord` /
/// Haxe `AnimationParameter`).
#[derive(Debug, Clone)]
pub struct SpriteAnimParam {
    pub offset_x: f32,
    pub offset_y: f32,
    pub start_pause_sec: f32,

    pub x_osc_per_sec: f32,
    pub x_amp: f32,
    pub x_phase: f32,

    pub y_osc_per_sec: f32,
    pub y_amp: f32,
    pub y_phase: f32,

    pub rot_center_x: f32,
    pub rot_center_y: f32,
    pub rot_per_sec: f32,
    pub rot_phase: f32,

    pub rock_osc_per_sec: f32,
    pub rock_amp: f32,
    pub rock_phase: f32,

    pub duration_sec: f32,
    pub pause_sec: f32,

    pub fade_osc_per_sec: f32,
    pub fade_hardness: f32,
    pub fade_min: f32,
    /// C++ `zeroRecord` / scan defaults to 1 (opaque).
    pub fade_max: f32,
    pub fade_phase: f32,
}

impl Default for SpriteAnimParam {
    fn default() -> Self {
        // C++: zeroRecord — fadeMax = 1
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            start_pause_sec: 0.0,
            x_osc_per_sec: 0.0,
            x_amp: 0.0,
            x_phase: 0.0,
            y_osc_per_sec: 0.0,
            y_amp: 0.0,
            y_phase: 0.0,
            rot_center_x: 0.0,
            rot_center_y: 0.0,
            rot_per_sec: 0.0,
            rot_phase: 0.0,
            rock_osc_per_sec: 0.0,
            rock_amp: 0.0,
            rock_phase: 0.0,
            duration_sec: 0.0,
            pause_sec: 0.0,
            fade_osc_per_sec: 0.0,
            fade_hardness: 0.0,
            fade_min: 0.0,
            fade_max: 1.0,
            fade_phase: 0.0,
        }
    }
}

/// Optional sound trigger on an animation (C++ `SoundAnimationRecord` subset).
#[derive(Debug, Clone, Default)]
pub struct SoundAnimParam {
    /// Raw SoundUsage string (e.g. `621:0.250000#622:0.250000`).
    pub usage: String,
    pub repeat_per_sec: f32,
    pub repeat_phase: f32,
    pub age_start: f32,
    pub age_end: f32,
    pub footstep: bool,
}

/// Loaded animation for one object × type (× optional extra index).
#[derive(Debug, Clone)]
pub struct ObjectAnimation {
    pub object_id: i32,
    /// C++ AnimType integer (0 ground, 1 held, 2 moving, 4 eating, 5 doing, 7 extra).
    pub anim_type: i32,
    /// Extra slot index when `anim_type == ANIM_EXTRA`; else -1.
    pub extra_index: i32,
    /// C++ `randomStartPhase` (0/1 stored as float for sample seed convenience).
    pub rand_start_phase: f32,
    pub force_zero_start: bool,
    pub sprite_params: Vec<SpriteAnimParam>,
    pub slot_params: Vec<SpriteAnimParam>,
    pub sound_params: Vec<SoundAnimParam>,
    /// C++ `authorTag` from trailing `author=HASH` line (editor attribution).
    /// OLA1 format 2 stores this; format 1 load leaves `None`.
    pub author_tag: Option<String>,
}

impl Default for ObjectAnimation {
    fn default() -> Self {
        Self {
            object_id: 0,
            anim_type: 0,
            // Non-extra packs key with -1 (must not be 0 or `get` misses inserts).
            extra_index: -1,
            rand_start_phase: 0.0,
            force_zero_start: false,
            sprite_params: Vec::new(),
            slot_params: Vec::new(),
            sound_params: Vec::new(),
            author_tag: None,
        }
    }
}

/// Evaluated draw offset for one sprite at time `t`.
#[derive(Debug, Clone, Copy)]
pub struct AnimSample {
    pub x: f32,
    pub y: f32,
    /// Rotation in full turns (matches object `rot` / soft-FB).
    pub rot: f32,
    /// Fade alpha multiplier (C++ `workingSpriteFade`; 1.0 = fully opaque).
    pub fade: f32,
    /// C++ `rotationCenterOffset` (applied at draw before parent chain).
    pub rot_center_x: f32,
    pub rot_center_y: f32,
}

impl Default for AnimSample {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rot: 0.0,
            fade: 1.0,
            rot_center_x: 0.0,
            rot_center_y: 0.0,
        }
    }
}

impl SpriteAnimParam {
    /// Sample motion at wall-clock seconds `t` with optional start-phase offset
    /// (0..1 fractions of each cycle; bank passes `rand_start_phase`).
    ///
    /// // C++: `getOscOffset` + `processFrameTimeWithPauses` + fade hardness
    /// // Haxe: `Object.getOscOffset` + fadePhase+0.25 power-harden
    pub fn sample(&self, t: f32, start_phase: f32) -> AnimSample {
        // C++ processFrameTimeWithPauses — never drops rot accumulation.
        let ft = self.frame_time(t);

        // C++ getOscOffset(t, offset, osc, amp, phase) = offset + amp*sin((t*osc+phase)*2π)
        let x = self.offset_x
            + self.x_amp * phase_sin(self.x_osc_per_sec, ft, self.x_phase + start_phase);
        let y = self.offset_y
            + self.y_amp * phase_sin(self.y_osc_per_sec, ft, self.y_phase + start_phase);
        // Continuous spin + rock (rockAmp is fraction of full turn).
        let rot = self.rot_phase
            + self.rot_per_sec * ft
            + self.rock_amp
                * phase_sin(self.rock_osc_per_sec, ft, self.rock_phase + start_phase);

        let fade = self.sample_fade(ft, start_phase);

        AnimSample {
            x,
            y,
            rot,
            fade,
            rot_center_x: self.rot_center_x,
            rot_center_y: self.rot_center_y,
        }
    }

    /// C++/Haxe fade hardness: `sin(…, fadePhase+0.25)` then power-square curve.
    ///
    /// hardVersion = sign(s) * |s|^(1/(hardness*10+1)); hardness==1 → square ±1.
    /// fade = (max-min)*(0.5*hard + 0.5) + min
    ///
    /// When fade is unused (`fadeMin==fadeMax==0` and no osc) — common on person
    /// body layers in Jason's anim files — return **1.0** (fully opaque). Treating
    /// that as 0 made player skin invisible while clothing (often fadeMax=1) still drew.
    pub fn sample_fade(&self, frame_time: f32, start_phase: f32) -> f32 {
        // Unused fade channel → full opacity (Jason person body animParams).
        if self.fade_osc_per_sec.abs() < 1e-8
            && self.fade_min.abs() < 1e-8
            && self.fade_max.abs() < 1e-8
        {
            return 1.0;
        }
        // C++: getOscOffset(frameTime, 0, fadeOscPerSec, 1.0, fadePhase + 0.25)
        let sin_val = phase_sin(
            self.fade_osc_per_sec,
            frame_time,
            self.fade_phase + 0.25 + start_phase,
        );
        let hardness = self.fade_hardness;
        let hard_version = if (hardness - 1.0).abs() < 1e-6 {
            if sin_val > 0.0 {
                1.0
            } else {
                -1.0
            }
        } else {
            let abs_sin = sin_val.abs();
            if abs_sin > 1e-12 {
                (sin_val / abs_sin) * abs_sin.powf(1.0 / (hardness * 10.0 + 1.0))
            } else {
                0.0
            }
        };
        (self.fade_max - self.fade_min) * (0.5 * hard_version + 0.5) + self.fade_min
    }

    /// C++ `processFrameTimeWithPauses` for this layer.
    ///
    /// - If `pauseSec==0 && startPauseSec==0`: continuous wall time (even when
    ///   `durationSec>0` — no per-cycle wrap).
    /// - `t < startPause`: freeze at 0.
    /// - In mid-cycle pause: freeze at `(blocks+1)*duration` (keeps rot_per_sec
    ///   progress from completed duration blocks).
    pub fn frame_time(&self, t: f32) -> f32 {
        let t = t.max(0.0);
        // C++: if( pauseSec == 0 && startPauseSec == 0 ) return inFrameTime;
        if self.pause_sec.abs() < 1e-8 && self.start_pause_sec.abs() < 1e-8 {
            return t;
        }

        let dur = self.duration_sec.max(0.0);
        let pause = self.pause_sec.max(0.0);
        let start_pause = self.start_pause_sec.max(0.0);

        // C++: if( inFrameTime < startPause ) return 0;
        if t < start_pause {
            return 0.0;
        }

        let block_time = dur + pause;
        if block_time <= 1e-8 {
            return 0.0;
        }

        let block_fraction = (t - start_pause) / block_time;
        let num_full = block_fraction.floor();
        let this_block_time = (block_fraction - num_full) * block_time;

        if this_block_time > dur {
            // in pause: freeze at end of last duration block
            (num_full + 1.0) * dur
        } else {
            num_full * dur + this_block_time
        }
    }
}

/// C++ `getOscOffset` sine factor: `sin((t*osc + phase) * 2π)`.
fn phase_sin(osc_per_sec: f32, t: f32, phase: f32) -> f32 {
    if osc_per_sec.abs() < 1e-8 && phase.abs() < 1e-8 {
        return 0.0;
    }
    ((osc_per_sec * t + phase) * std::f32::consts::TAU).sin()
}

/// Cache key: (object_id, anim_type, extra_index).
type AnimKey = (i32, i32, i32);

pub struct AnimBank {
    root: PathBuf,
    cache: HashMap<AnimKey, ObjectAnimation>,
    /// When true, miss on cache does not hit disk (pure OLA1 / preloaded bank).
    pub binary_only: bool,
}

impl AnimBank {
    pub fn new(content_root: impl AsRef<Path>) -> Self {
        Self {
            root: content_root.as_ref().to_path_buf(),
            cache: HashMap::new(),
            binary_only: false,
        }
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Prefer `root/cache/ola1_anims.bin` when present and valid; else empty bank
    /// with text lazy-load fallback from `root/animations/`.
    ///
    /// Validates against `cache/manifest.json` when present:
    /// - blob sha1 of `ola1_anims.bin`
    /// - manifest `data_version` vs tree `dataVersionNumber.txt` (if both set)
    /// - OLA1 header `data_version` vs manifest
    ///
    /// Stale/mismatched cache falls through to text lazy-load (does not auto-bake).
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
            LoadStage::Anim,
            0.0,
            Some("prefer_cache"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        let root = content_root.as_ref().to_path_buf();
        let ola1_path = root.join("cache").join("ola1_anims.bin");
        if ola1_path.exists() {
            if let Ok(bytes) = fs::read(&ola1_path) {
                if ola1_cache_valid(&root, &bytes) {
                    if let Ok(bank) = Self::from_ola1(&bytes, &root) {
                        report_stage(
                            LoadStage::Anim,
                            1.0,
                            Some("ola1"),
                            crate::load_progress::reborrow_cb(&mut on_progress),
                        );
                        return bank;
                    }
                }
            }
        }
        report_stage(
            LoadStage::Anim,
            1.0,
            Some("lazy_text"),
            crate::load_progress::reborrow_cb(&mut on_progress),
        );
        Self::new(root)
    }

    /// Build bank from OLA1 bytes (binary-only).
    pub fn from_ola1(data: &[u8], content_root: impl AsRef<Path>) -> Result<Self, String> {
        let records = load_ola1(data)?;
        let mut cache = HashMap::with_capacity(records.len());
        for a in records {
            cache.insert(key_of(&a), a);
        }
        Ok(Self {
            root: content_root.as_ref().to_path_buf(),
            cache,
            binary_only: true,
        })
    }

    /// Insert or replace one animation record.
    pub fn insert(&mut self, anim: ObjectAnimation) {
        self.cache.insert(key_of(&anim), anim);
    }

    /// type 0 = ground, 2 = moving, etc. (C++ AnimType).
    /// // C++: ground2 (3) is runtime-only → resolve as ground (0)
    pub fn get(&mut self, object_id: i32, anim_type: i32) -> Option<&ObjectAnimation> {
        self.get_ex(object_id, anim_type, -1)
    }

    pub fn get_extra(&mut self, object_id: i32, extra_index: i32) -> Option<&ObjectAnimation> {
        self.get_ex(object_id, ANIM_EXTRA, extra_index)
    }

    pub fn get_ex(
        &mut self,
        object_id: i32,
        anim_type: i32,
        extra_index: i32,
    ) -> Option<&ObjectAnimation> {
        let anim_type = resolve_anim_type(anim_type);
        let k = (object_id, anim_type, extra_index);
        if !self.cache.contains_key(&k) && !self.binary_only {
            if let Some(a) = load_animation(&self.root, object_id, anim_type, extra_index) {
                self.cache.insert(k, a);
            }
        }
        self.cache.get(&k)
    }

    pub fn sample_sprite(
        &mut self,
        object_id: i32,
        anim_type: i32,
        sprite_index: usize,
        t: f32,
    ) -> AnimSample {
        self.sample_sprite_ex(object_id, anim_type, -1, sprite_index, t)
    }

    /// Sample with explicit extra index (C++ `setExtraIndex` + `ANIM_EXTRA` / `7xN`).
    pub fn sample_sprite_ex(
        &mut self,
        object_id: i32,
        anim_type: i32,
        extra_index: i32,
        sprite_index: usize,
        t: f32,
    ) -> AnimSample {
        let rec = if resolve_anim_type(anim_type) == ANIM_EXTRA {
            self.get_ex(object_id, ANIM_EXTRA, extra_index.max(0))
        } else {
            self.get(object_id, anim_type)
        };
        rec.and_then(|a| {
            let phase = a.rand_start_phase;
            a.sprite_params
                .get(sprite_index)
                .map(|p| p.sample(t, phase))
        })
        .unwrap_or_default()
    }

    /// Sample a container slot's animation track (C++ `slotAnim[i]`).
    pub fn sample_slot(
        &mut self,
        object_id: i32,
        anim_type: i32,
        slot_index: usize,
        t: f32,
    ) -> AnimSample {
        self.get(object_id, anim_type)
            .and_then(|a| {
                let phase = a.rand_start_phase;
                a.slot_params
                    .get(slot_index)
                    .map(|p| p.sample(t, phase))
            })
            .unwrap_or_default()
    }

    /// Serialize all cached records to OLA1.
    pub fn write_ola1(&self, data_version: u32) -> Vec<u8> {
        write_ola1(self.cache.values(), data_version)
    }

    /// Load every `animations/*.txt` under content root into this bank.
    pub fn load_all_text(&mut self) -> Result<usize, String> {
        let dir = self.root.join("animations");
        if !dir.is_dir() {
            return Ok(0);
        }
        let mut n = 0usize;
        let rd = fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for ent in rd {
            let ent = ent.map_err(|e| e.to_string())?;
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some((obj_id, anim_type, extra)) = parse_anim_filename(stem) else {
                continue;
            };
            let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if let Some(a) = parse_animation_txt(obj_id, anim_type, extra, &text) {
                self.cache.insert(key_of(&a), a);
                n += 1;
            }
        }
        Ok(n)
    }
}

fn key_of(a: &ObjectAnimation) -> AnimKey {
    (a.object_id, a.anim_type, a.extra_index)
}

/// C++ treats `ground2` as a fade target; storage/lookup uses ground records.
/// `extraB` shares the same on-disk EXTRA records (index differs at sample time).
#[inline]
fn resolve_anim_type(anim_type: i32) -> i32 {
    if anim_type == ANIM_GROUND2 {
        ANIM_GROUND
    } else if anim_type == ANIM_EXTRA_B {
        ANIM_EXTRA
    } else {
        anim_type
    }
}

/// True when type is C++ `extra` or `extraB`.
#[inline]
pub fn is_extra_anim_type(anim_type: i32) -> bool {
    anim_type == ANIM_EXTRA || anim_type == ANIM_EXTRA_B
}

/// Validate OLA1 bytes against optional `cache/manifest.json` + tree version.
fn ola1_cache_valid(root: &Path, bytes: &[u8]) -> bool {
    if bytes.len() < 24 || &bytes[0..4] != OLA1_MAGIC {
        return false;
    }
    let format = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    if format != OLA1_FORMAT_VERSION && format != OLA1_FORMAT_VERSION_V1 {
        return false;
    }
    let ola1_data_ver = u32::from_le_bytes(bytes[8..12].try_into().unwrap_or([0; 4]));

    let man_path = root.join("cache").join("manifest.json");
    if !man_path.exists() {
        // No manifest: accept if header parses (from_ola1 will re-check).
        return true;
    }
    let Ok(text) = fs::read_to_string(&man_path) else {
        return true;
    };
    let Ok(man) = crate::content_binary::parse_manifest(&text) else {
        return true;
    };

    if let Some(blob) = man.ola1.as_ref() {
        let h = {
            use sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        };
        if h != blob.sha1 {
            return false;
        }
    }

    // manifest data_version vs tree
    if man.data_version > 0 {
        if let Ok(ver_s) = fs::read_to_string(root.join("dataVersionNumber.txt")) {
            if let Ok(tree_ver) = ver_s.trim().parse::<i32>() {
                if tree_ver != man.data_version {
                    return false;
                }
            }
        }
        if ola1_data_ver != 0 && ola1_data_ver != man.data_version as u32 {
            return false;
        }
    }

    true
}

/// Parse stem like `19_0`, `1007_7x0`, `19_5`.
pub fn parse_anim_filename(stem: &str) -> Option<(i32, i32, i32)> {
    let (id_s, rest) = stem.split_once('_')?;
    let obj_id: i32 = id_s.parse().ok()?;
    if let Some((ty_s, ex_s)) = rest.split_once('x') {
        let anim_type: i32 = ty_s.parse().ok()?;
        let extra: i32 = ex_s.parse().ok()?;
        return Some((obj_id, anim_type, extra));
    }
    let anim_type: i32 = rest.parse().ok()?;
    Some((obj_id, anim_type, -1))
}

fn load_animation(
    root: &Path,
    object_id: i32,
    anim_type: i32,
    extra_index: i32,
) -> Option<ObjectAnimation> {
    let name = if anim_type == ANIM_EXTRA && extra_index >= 0 {
        format!("{object_id}_{anim_type}x{extra_index}.txt")
    } else {
        format!("{object_id}_{anim_type}.txt")
    };
    let path = root.join("animations").join(name);
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    parse_animation_txt(object_id, anim_type, extra_index, &text)
}

/// Parse OHOL animation text (C++ `scanAnimationRecordFromString` field set).
pub fn parse_animation_txt(
    object_id: i32,
    anim_type: i32,
    extra_index: i32,
    text: &str,
) -> Option<ObjectAnimation> {
    let mut anim = ObjectAnimation {
        object_id,
        anim_type,
        extra_index,
        sprite_params: Vec::new(),
        slot_params: Vec::new(),
        sound_params: Vec::new(),
        rand_start_phase: 0.0,
        force_zero_start: false,
        author_tag: None,
    };

    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
    let mut i = 0usize;

    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("id=") {
            if let Ok(id) = v.trim().parse::<i32>() {
                anim.object_id = id;
            }
        }
        i += 1;
    }

    if let Some(line) = lines.get(i) {
        if let Some(rest) = line.strip_prefix("type=") {
            parse_type_line(rest, &mut anim);
        }
        i += 1;
    }

    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("forceZeroStart=") {
            anim.force_zero_start = v.trim().starts_with('1');
            i += 1;
        }
    }

    let mut num_sounds = 0usize;
    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("numSounds=") {
            num_sounds = v.trim().parse().unwrap_or(0);
            i += 1;
            for _ in 0..num_sounds {
                if i >= lines.len() {
                    break;
                }
                if let Some(rest) = lines[i].strip_prefix("soundParam=") {
                    if let Some(s) = parse_sound_param(rest) {
                        anim.sound_params.push(s);
                    }
                }
                i += 1;
            }
        }
    }

    let mut num_sprites = 0usize;
    let mut num_slots = 0usize;
    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("numSprites=") {
            num_sprites = v.trim().parse().unwrap_or(0);
            i += 1;
        }
    }
    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("numSlots=") {
            num_slots = v.trim().parse().unwrap_or(0);
            i += 1;
        }
    }

    for _ in 0..num_sprites {
        if i >= lines.len() {
            break;
        }
        let (p, ni) = parse_param_block(&lines, i);
        anim.sprite_params.push(p);
        i = ni;
    }
    for _ in 0..num_slots {
        if i >= lines.len() {
            break;
        }
        let (p, ni) = parse_param_block(&lines, i);
        anim.slot_params.push(p);
        i = ni;
    }

    if anim.sprite_params.len() < num_sprites {
        anim.sprite_params
            .resize(num_sprites, SpriteAnimParam::default());
    }
    if anim.slot_params.len() < num_slots {
        anim.slot_params
            .resize(num_slots, SpriteAnimParam::default());
    }

    // C++: trailing `author=%s` after sprite/slot param blocks (~431–446).
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if let Some(rest) = line.strip_prefix("author=") {
            let tag = rest
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim();
            if !tag.is_empty() {
                anim.author_tag = Some(tag.to_string());
            }
            break;
        }
        // Skip blank / unknown trailer lines.
        if line.is_empty() {
            continue;
        }
    }

    let _ = num_sounds;
    Some(anim)
}

fn parse_type_line(rest: &str, anim: &mut ObjectAnimation) {
    let (type_part, rsp_part) = rest
        .split_once(",randStartPhase=")
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((rest, None));

    if let Some((ty, ex)) = type_part.split_once(':') {
        anim.anim_type = ty.trim().parse().unwrap_or(anim.anim_type);
        anim.extra_index = ex.trim().parse().unwrap_or(-1);
    } else {
        anim.anim_type = type_part.trim().parse().unwrap_or(anim.anim_type);
        if anim.anim_type != ANIM_EXTRA {
            anim.extra_index = -1;
        }
    }
    if let Some(rsp) = rsp_part {
        let v = rsp
            .split(|c: char| c == ',' || c.is_whitespace())
            .next()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        anim.rand_start_phase = v;
    }
}

fn parse_param_block(lines: &[&str], mut i: usize) -> (SpriteAnimParam, usize) {
    let mut p = SpriteAnimParam {
        fade_max: 1.0,
        duration_sec: 1.0,
        ..Default::default()
    };

    if let Some(line) = lines.get(i) {
        if let Some(rest) = line.strip_prefix("offset=") {
            if let Some((x, y)) = parse_pair(rest) {
                p.offset_x = x;
                p.offset_y = y;
            }
            i += 1;
        }
    }
    if let Some(line) = lines.get(i) {
        if let Some(v) = line.strip_prefix("startPause=") {
            p.start_pause_sec = v.trim().parse().unwrap_or(0.0);
            i += 1;
        }
    }
    if let Some(line) = lines.get(i) {
        if let Some(rest) = line.strip_prefix("animParam=") {
            apply_anim_param_fields(&mut p, rest);
            i += 1;
        }
    }
    (p, i)
}

fn parse_pair(s: &str) -> Option<(f32, f32)> {
    let s = s.trim().trim_start_matches('(').trim_end_matches(')');
    let (a, b) = s.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn apply_anim_param_fields(p: &mut SpriteAnimParam, s: &str) {
    let mut cleaned = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '(' {
            while let Some(c2) = chars.next() {
                if c2 == ')' {
                    break;
                }
                if c2 == ',' {
                    cleaned.push(' ');
                } else {
                    cleaned.push(c2);
                }
            }
        } else {
            cleaned.push(c);
        }
    }
    let vals: Vec<f32> = cleaned
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();

    // Full sprite format (20 floats after pair expansion):
    // xOsc xAmp xPhase yOsc yAmp yPhase rotCx rotCy rotPerSec rotPhase
    // rockOsc rockAmp rockPhase duration pause fadeOsc fadeHard fadeMin fadeMax fadePhase
    if vals.len() >= 6 {
        p.x_osc_per_sec = vals[0];
        p.x_amp = vals[1];
        p.x_phase = vals[2];
        p.y_osc_per_sec = vals[3];
        p.y_amp = vals[4];
        p.y_phase = vals[5];
    }
    if vals.len() >= 10 {
        p.rot_center_x = vals[6];
        p.rot_center_y = vals[7];
        p.rot_per_sec = vals[8];
        p.rot_phase = vals[9];
    }
    if vals.len() >= 13 {
        p.rock_osc_per_sec = vals[10];
        p.rock_amp = vals[11];
        p.rock_phase = vals[12];
    }
    if vals.len() >= 15 {
        p.duration_sec = vals[13];
        p.pause_sec = vals[14];
    }
    if vals.len() >= 20 {
        p.fade_osc_per_sec = vals[15];
        p.fade_hardness = vals[16];
        p.fade_min = vals[17];
        p.fade_max = vals[18];
        p.fade_phase = vals[19];
    } else if vals.len() == 8 {
        // Legacy short slot format: x/y osc + duration/pause
        p.duration_sec = vals[6];
        p.pause_sec = vals[7];
    }
}

fn parse_sound_param(s: &str) -> Option<SoundAnimParam> {
    let mut parts = s.split_whitespace();
    let usage = parts.next()?.to_string();
    let repeat_per_sec: f32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let repeat_phase: f32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let age_start: f32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(-1.0);
    let age_end: f32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(-1.0);
    let footstep = parts
        .next()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(0)
        != 0;
    Some(SoundAnimParam {
        usage,
        repeat_per_sec,
        repeat_phase,
        age_start,
        age_end,
        footstep,
    })
}

// ── OLA1 binary ──────────────────────────────────────────────────────────────

fn push_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}
fn push_str_u16(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let len = (b.len().min(u16::MAX as usize)) as u16;
    push_u16(out, len);
    out.extend_from_slice(&b[..len as usize]);
}

fn read_f32(data: &[u8], off: &mut usize) -> Result<f32, String> {
    if *off + 4 > data.len() {
        return Err("OLA1 truncated f32".into());
    }
    let v = f32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}
fn read_i32(data: &[u8], off: &mut usize) -> Result<i32, String> {
    if *off + 4 > data.len() {
        return Err("OLA1 truncated i32".into());
    }
    let v = i32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}
fn read_u16(data: &[u8], off: &mut usize) -> Result<u16, String> {
    if *off + 2 > data.len() {
        return Err("OLA1 truncated u16".into());
    }
    let v = u16::from_le_bytes(data[*off..*off + 2].try_into().unwrap());
    *off += 2;
    Ok(v)
}
fn read_u8(data: &[u8], off: &mut usize) -> Result<u8, String> {
    if *off >= data.len() {
        return Err("OLA1 truncated u8".into());
    }
    let v = data[*off];
    *off += 1;
    Ok(v)
}
fn read_str_u16(data: &[u8], off: &mut usize) -> Result<String, String> {
    let len = read_u16(data, off)? as usize;
    if *off + len > data.len() {
        return Err("OLA1 truncated string".into());
    }
    let s = String::from_utf8_lossy(&data[*off..*off + len]).into_owned();
    *off += len;
    Ok(s)
}

fn write_param(out: &mut Vec<u8>, p: &SpriteAnimParam) {
    push_f32(out, p.offset_x);
    push_f32(out, p.offset_y);
    push_f32(out, p.start_pause_sec);
    push_f32(out, p.x_osc_per_sec);
    push_f32(out, p.x_amp);
    push_f32(out, p.x_phase);
    push_f32(out, p.y_osc_per_sec);
    push_f32(out, p.y_amp);
    push_f32(out, p.y_phase);
    push_f32(out, p.rot_center_x);
    push_f32(out, p.rot_center_y);
    push_f32(out, p.rot_per_sec);
    push_f32(out, p.rot_phase);
    push_f32(out, p.rock_osc_per_sec);
    push_f32(out, p.rock_amp);
    push_f32(out, p.rock_phase);
    push_f32(out, p.duration_sec);
    push_f32(out, p.pause_sec);
    push_f32(out, p.fade_osc_per_sec);
    push_f32(out, p.fade_hardness);
    push_f32(out, p.fade_min);
    push_f32(out, p.fade_max);
    push_f32(out, p.fade_phase);
}

fn read_param(data: &[u8], off: &mut usize) -> Result<SpriteAnimParam, String> {
    Ok(SpriteAnimParam {
        offset_x: read_f32(data, off)?,
        offset_y: read_f32(data, off)?,
        start_pause_sec: read_f32(data, off)?,
        x_osc_per_sec: read_f32(data, off)?,
        x_amp: read_f32(data, off)?,
        x_phase: read_f32(data, off)?,
        y_osc_per_sec: read_f32(data, off)?,
        y_amp: read_f32(data, off)?,
        y_phase: read_f32(data, off)?,
        rot_center_x: read_f32(data, off)?,
        rot_center_y: read_f32(data, off)?,
        rot_per_sec: read_f32(data, off)?,
        rot_phase: read_f32(data, off)?,
        rock_osc_per_sec: read_f32(data, off)?,
        rock_amp: read_f32(data, off)?,
        rock_phase: read_f32(data, off)?,
        duration_sec: read_f32(data, off)?,
        pause_sec: read_f32(data, off)?,
        fade_osc_per_sec: read_f32(data, off)?,
        fade_hardness: read_f32(data, off)?,
        fade_min: read_f32(data, off)?,
        fade_max: read_f32(data, off)?,
        fade_phase: read_f32(data, off)?,
    })
}

/// Write OLA1 blob from an iterator of animation records.
pub fn write_ola1<'a>(
    records: impl IntoIterator<Item = &'a ObjectAnimation>,
    data_version: u32,
) -> Vec<u8> {
    let mut list: Vec<&ObjectAnimation> = records.into_iter().collect();
    list.sort_by_key(|a| (a.object_id, a.anim_type, a.extra_index));

    let mut out = Vec::with_capacity(24 + list.len() * 128);
    out.extend_from_slice(OLA1_MAGIC);
    push_u32(&mut out, OLA1_FORMAT_VERSION);
    push_u32(&mut out, data_version);
    push_u32(&mut out, list.len() as u32);
    push_u32(&mut out, 0); // flags
    push_u32(&mut out, 0); // header_crc32 reserved

    for a in list {
        write_ola1_record(&mut out, a);
    }
    out
}

fn write_ola1_record(out: &mut Vec<u8>, a: &ObjectAnimation) {
    push_i32(out, a.object_id);
    push_i32(out, a.anim_type);
    push_i32(out, a.extra_index);
    push_f32(out, a.rand_start_phase);
    let mut flags = 0u8;
    if a.rand_start_phase > 0.5 {
        flags |= AF_RANDOM_START;
    }
    if a.force_zero_start {
        flags |= AF_FORCE_ZERO_START;
    }
    push_u8(out, flags);

    let n_sounds = a.sound_params.len().min(u16::MAX as usize) as u16;
    let n_sprites = a.sprite_params.len().min(u16::MAX as usize) as u16;
    let n_slots = a.slot_params.len().min(u16::MAX as usize) as u16;
    push_u16(out, n_sounds);
    push_u16(out, n_sprites);
    push_u16(out, n_slots);

    for s in a.sound_params.iter().take(n_sounds as usize) {
        push_str_u16(out, &s.usage);
        push_f32(out, s.repeat_per_sec);
        push_f32(out, s.repeat_phase);
        push_f32(out, s.age_start);
        push_f32(out, s.age_end);
        push_u8(out, if s.footstep { 1 } else { 0 });
    }
    for p in a.sprite_params.iter().take(n_sprites as usize) {
        write_param(out, p);
    }
    for p in a.slot_params.iter().take(n_slots as usize) {
        write_param(out, p);
    }
    // Format 2 trailer: authorTag (empty string when None).
    push_str_u16(out, a.author_tag.as_deref().unwrap_or(""));
}

/// Load OLA1 → list of records (format 1 legacy + format 2 with authorTag).
pub fn load_ola1(data: &[u8]) -> Result<Vec<ObjectAnimation>, String> {
    if data.len() < 24 {
        return Err("OLA1 too short".into());
    }
    if &data[0..4] != OLA1_MAGIC {
        return Err("bad OLA1 magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLA1_FORMAT_VERSION && format != OLA1_FORMAT_VERSION_V1 {
        return Err(format!("unsupported OLA1 format {format}"));
    }
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let mut off = 24usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_ola1_record(data, &mut off, format)?);
    }
    Ok(out)
}

/// Load OLA1 and return (data_version, records).
pub fn load_ola1_with_version(data: &[u8]) -> Result<(u32, Vec<ObjectAnimation>), String> {
    if data.len() < 24 {
        return Err("OLA1 too short".into());
    }
    if &data[0..4] != OLA1_MAGIC {
        return Err("bad OLA1 magic".into());
    }
    let format = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format != OLA1_FORMAT_VERSION && format != OLA1_FORMAT_VERSION_V1 {
        return Err(format!("unsupported OLA1 format {format}"));
    }
    let data_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let records = load_ola1(data)?;
    Ok((data_version, records))
}

fn read_ola1_record(
    data: &[u8],
    off: &mut usize,
    format: u32,
) -> Result<ObjectAnimation, String> {
    let object_id = read_i32(data, off)?;
    let anim_type = read_i32(data, off)?;
    let extra_index = read_i32(data, off)?;
    let rand_start_phase = read_f32(data, off)?;
    let flags = read_u8(data, off)?;
    let n_sounds = read_u16(data, off)? as usize;
    let n_sprites = read_u16(data, off)? as usize;
    let n_slots = read_u16(data, off)? as usize;

    let mut sound_params = Vec::with_capacity(n_sounds);
    for _ in 0..n_sounds {
        let usage = read_str_u16(data, off)?;
        let repeat_per_sec = read_f32(data, off)?;
        let repeat_phase = read_f32(data, off)?;
        let age_start = read_f32(data, off)?;
        let age_end = read_f32(data, off)?;
        let footstep = read_u8(data, off)? != 0;
        sound_params.push(SoundAnimParam {
            usage,
            repeat_per_sec,
            repeat_phase,
            age_start,
            age_end,
            footstep,
        });
    }

    let mut sprite_params = Vec::with_capacity(n_sprites);
    for _ in 0..n_sprites {
        sprite_params.push(read_param(data, off)?);
    }
    let mut slot_params = Vec::with_capacity(n_slots);
    for _ in 0..n_slots {
        slot_params.push(read_param(data, off)?);
    }

    let author_tag = if format >= OLA1_FORMAT_VERSION {
        let s = read_str_u16(data, off)?;
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    };

    Ok(ObjectAnimation {
        object_id,
        anim_type,
        extra_index,
        rand_start_phase,
        force_zero_start: (flags & AF_FORCE_ZERO_START) != 0,
        sprite_params,
        slot_params,
        sound_params,
        author_tag,
    })
}

/// Bake all text animations under `src/animations` → OLA1 bytes.
pub fn bake_ola1_from_dir(
    src: impl AsRef<Path>,
    data_version: u32,
) -> Result<(Vec<u8>, usize), String> {
    let mut bank = AnimBank::new(src.as_ref());
    let n = bank.load_all_text()?;
    let bytes = bank.write_ola1(data_version);
    Ok((bytes, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_anim_filename_basic() {
        assert_eq!(parse_anim_filename("19_0"), Some((19, 0, -1)));
        assert_eq!(parse_anim_filename("1007_7x0"), Some((1007, 7, 0)));
        assert_eq!(parse_anim_filename("1007_7x12"), Some((1007, 7, 12)));
        assert_eq!(parse_anim_filename("bad"), None);
    }

    #[test]
    fn parse_full_anim_param() {
        let text = r#"id=42
type=2,randStartPhase=1
forceZeroStart=1
numSounds=1
soundParam=621:0.250000 0.270000 0.000000 -1.000000 0.200000 1
numSprites=1
numSlots=1
offset=(1.500000,2.500000)
startPause=0.100000
animParam=3.000000 10.000000 0.250000 1.500000 4.000000 0.500000 (0.500000,-0.500000) 0.100000 0.200000 0.300000 0.400000 0.500000 1.000000 0.000000 2.000000 0.500000 0.100000 0.900000 0.250000
offset=(0.000000,1.000000)
startPause=0.000000
animParam=0.000000 0.000000 0.000000 0.000000 0.000000 0.000000 (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000
"#;
        let a = parse_animation_txt(42, 2, -1, text).unwrap();
        assert_eq!(a.object_id, 42);
        assert_eq!(a.anim_type, 2);
        assert!((a.rand_start_phase - 1.0).abs() < 1e-5);
        assert!(a.force_zero_start);
        assert_eq!(a.sound_params.len(), 1);
        assert!(a.sound_params[0].footstep);
        assert_eq!(a.sprite_params.len(), 1);
        assert_eq!(a.slot_params.len(), 1);
        let p = &a.sprite_params[0];
        assert!((p.offset_x - 1.5).abs() < 1e-5);
        assert!((p.offset_y - 2.5).abs() < 1e-5);
        assert!((p.start_pause_sec - 0.1).abs() < 1e-5);
        assert!((p.x_osc_per_sec - 3.0).abs() < 1e-5);
        assert!((p.x_amp - 10.0).abs() < 1e-5);
        assert!((p.x_phase - 0.25).abs() < 1e-5);
        assert!((p.y_osc_per_sec - 1.5).abs() < 1e-5);
        assert!((p.y_amp - 4.0).abs() < 1e-5);
        assert!((p.rot_center_x - 0.5).abs() < 1e-5);
        assert!((p.rot_center_y + 0.5).abs() < 1e-5);
        assert!((p.rot_per_sec - 0.1).abs() < 1e-5);
        assert!((p.fade_max - 0.9).abs() < 1e-5);
    }

    #[test]
    fn unused_fade_channel_is_full_opacity() {
        // Jason person body layers: fadeMin=fadeMax=0, fadeOsc=0 → fully visible.
        let p = SpriteAnimParam {
            fade_min: 0.0,
            fade_max: 0.0,
            fade_osc_per_sec: 0.0,
            fade_hardness: 0.0,
            fade_phase: 0.0,
            ..Default::default()
        };
        assert!(
            (p.sample_fade(0.0, 0.0) - 1.0).abs() < 1e-5,
            "unused fade must not hide skin"
        );
        assert!((p.sample_fade(1.5, 0.3) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ola1_roundtrip() {
        let text = r#"id=19
type=0,randStartPhase=0
forceZeroStart=0
numSounds=0
numSprites=2
numSlots=0
offset=(0.000000,0.000000)
startPause=0.000000
animParam=3.000000 10.000000 0.250000 0.000000 0.000000 0.000000 (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000
offset=(1.000000,2.000000)
startPause=0.000000
animParam=1.500000 20.000000 0.500000 0.000000 0.000000 0.000000 (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000
author=2BE17D1AC5
"#;
        let a = parse_animation_txt(19, 0, -1, text).unwrap();
        assert_eq!(a.author_tag.as_deref(), Some("2BE17D1AC5"));
        let mut bank = AnimBank::new(".");
        bank.insert(a);
        let extra = ObjectAnimation {
            object_id: 1007,
            anim_type: ANIM_EXTRA,
            extra_index: 0,
            sprite_params: vec![SpriteAnimParam {
                x_amp: 5.0,
                x_osc_per_sec: 2.0,
                duration_sec: 1.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            author_tag: Some("DEADBEEF".into()),
            ..Default::default()
        };
        bank.insert(extra);

        let bytes = bank.write_ola1(437);
        assert!(bytes.starts_with(b"OLA1"));
        let fmt = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(fmt, OLA1_FORMAT_VERSION, "write path is OLA1 v2");
        let (ver, recs) = load_ola1_with_version(&bytes).unwrap();
        assert_eq!(ver, 437);
        assert_eq!(recs.len(), 2);

        let mut bank2 = AnimBank::from_ola1(&bytes, ".").unwrap();
        assert_eq!(bank2.len(), 2);
        let g = bank2.get(19, 0).unwrap();
        assert_eq!(g.sprite_params.len(), 2);
        assert!((g.sprite_params[0].x_amp - 10.0).abs() < 1e-5);
        assert!((g.sprite_params[1].offset_x - 1.0).abs() < 1e-5);
        assert_eq!(g.author_tag.as_deref(), Some("2BE17D1AC5"));
        let ex = bank2.get_extra(1007, 0).unwrap();
        assert!((ex.sprite_params[0].x_amp - 5.0).abs() < 1e-5);
        assert_eq!(ex.author_tag.as_deref(), Some("DEADBEEF"));
    }

    /// P4#33: format 1 blobs (no author trailer) still load.
    #[test]
    fn ola1_v1_legacy_load_without_author() {
        // Minimal hand-built OLA1 v1: one empty anim (0 sprites/slots/sounds).
        let mut out = Vec::new();
        out.extend_from_slice(OLA1_MAGIC);
        push_u32(&mut out, OLA1_FORMAT_VERSION_V1);
        push_u32(&mut out, 99); // data_version
        push_u32(&mut out, 1); // count
        push_u32(&mut out, 0); // flags
        push_u32(&mut out, 0); // crc
        push_i32(&mut out, 5); // object_id
        push_i32(&mut out, 0); // anim_type
        push_i32(&mut out, -1); // extra_index
        push_f32(&mut out, 0.0); // rand_start
        push_u8(&mut out, 0); // flags
        push_u16(&mut out, 0); // n_sounds
        push_u16(&mut out, 0); // n_sprites
        push_u16(&mut out, 0); // n_slots
        // no author trailer in v1
        let recs = load_ola1(&out).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].object_id, 5);
        assert!(recs[0].author_tag.is_none());
    }

    #[test]
    fn parse_author_tag_from_text() {
        let text = "id=1\ntype=0,randStartPhase=0\nnumSounds=0\nnumSprites=0\nnumSlots=0\nauthor=ABC123\n";
        let a = parse_animation_txt(1, 0, -1, text).unwrap();
        assert_eq!(a.author_tag.as_deref(), Some("ABC123"));
        let no = parse_animation_txt(
            1,
            0,
            -1,
            "id=1\ntype=0,randStartPhase=0\nnumSounds=0\nnumSprites=0\nnumSlots=0\n",
        )
        .unwrap();
        assert!(no.author_tag.is_none());
    }

    #[test]
    fn sample_oscillates() {
        let p = SpriteAnimParam {
            x_amp: 10.0,
            x_osc_per_sec: 1.0,
            duration_sec: 0.0,
            fade_max: 1.0,
            ..Default::default()
        };
        let a = p.sample(0.0, 0.0);
        let b = p.sample(0.25, 0.0);
        assert!(a.x.abs() < 1.0);
        assert!(b.x.abs() > 5.0);
    }

    #[test]
    fn parse_anim_file_if_present() {
        let roots = [
            r"C:\OhOl\OpenLife\OneLifeData7",
            r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
        ];
        for root in roots {
            let p = Path::new(root).join("animations").join("19_0.txt");
            if !p.exists() {
                continue;
            }
            let text = fs::read_to_string(&p).unwrap();
            let a = parse_animation_txt(19, 0, -1, &text).unwrap();
            assert_eq!(a.object_id, 19);
            assert!(!a.sprite_params.is_empty());
            assert!(a.sprite_params[0].x_osc_per_sec > 0.0 || a.sprite_params.len() > 1);
            return;
        }
    }

    #[test]
    fn bake_ola1_from_fixture_dir() {
        let tmp = std::env::temp_dir().join(format!("ohol_ola1_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("animations")).unwrap();
        fs::write(
            tmp.join("animations").join("55_0.txt"),
            "id=55\ntype=0,randStartPhase=0\nforceZeroStart=0\nnumSounds=0\n\
             numSprites=1\nnumSlots=0\n\
             offset=(0.000000,0.000000)\nstartPause=0.000000\n\
             animParam=1.000000 5.000000 0.000000 0.000000 0.000000 0.000000 \
             (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 \
             1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000\n",
        )
        .unwrap();
        fs::write(
            tmp.join("animations").join("55_7x0.txt"),
            "id=55\ntype=7:0,randStartPhase=0\nforceZeroStart=0\nnumSounds=0\n\
             numSprites=1\nnumSlots=0\n\
             offset=(0.000000,0.000000)\nstartPause=0.000000\n\
             animParam=0.000000 0.000000 0.000000 0.000000 0.000000 0.000000 \
             (0.000000,0.000000) 0.000000 0.000000 0.000000 0.000000 0.000000 \
             1.000000 0.000000 0.000000 0.000000 0.000000 1.000000 0.000000\n",
        )
        .unwrap();

        let (bytes, n) = bake_ola1_from_dir(&tmp, 99).unwrap();
        assert_eq!(n, 2);
        assert!(bytes.starts_with(b"OLA1"));
        let recs = load_ola1(&bytes).unwrap();
        assert_eq!(recs.len(), 2);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn bad_magic_rejected() {
        assert!(load_ola1(b"XXXX................").is_err());
    }

    /// C++ processFrameTimeWithPauses: mid-pause freezes at end of duration block
    /// so rot_per_sec keeps accumulated progress (not bare rot_phase).
    #[test]
    fn pause_freeze_keeps_end_of_block_rot() {
        let p = SpriteAnimParam {
            duration_sec: 1.0,
            pause_sec: 1.0,
            rot_per_sec: 1.0,
            rot_phase: 0.0,
            fade_max: 1.0,
            ..Default::default()
        };
        // t=1.5 → in pause of first cycle → frame_time = 1.0
        assert!((p.frame_time(1.5) - 1.0).abs() < 1e-5);
        let s = p.sample(1.5, 0.0);
        assert!((s.rot - 1.0).abs() < 1e-4, "got rot {}", s.rot);
        // t=0.5 → mid duration
        assert!((p.frame_time(0.5) - 0.5).abs() < 1e-5);
        // t=3.5 → second cycle pause → frame_time = 2.0
        assert!((p.frame_time(3.5) - 2.0).abs() < 1e-5);
        // startPause holds at 0
        let p2 = SpriteAnimParam {
            start_pause_sec: 0.5,
            pause_sec: 0.1,
            duration_sec: 1.0,
            rot_per_sec: 1.0,
            ..Default::default()
        };
        assert!((p2.frame_time(0.25) - 0.0).abs() < 1e-5);
        assert!((p2.sample(0.25, 0.0).rot - 0.0).abs() < 1e-5);
    }

    /// When pauseSec==0 && startPauseSec==0, C++ uses continuous wall time
    /// even if durationSec>0 (no per-cycle wrap / spin jump).
    #[test]
    fn continuous_time_ignores_duration_when_no_pause() {
        let p = SpriteAnimParam {
            duration_sec: 1.0,
            pause_sec: 0.0,
            start_pause_sec: 0.0,
            rot_per_sec: 1.0,
            fade_max: 1.0,
            ..Default::default()
        };
        assert!((p.frame_time(2.5) - 2.5).abs() < 1e-5);
        let s = p.sample(2.5, 0.0);
        assert!((s.rot - 2.5).abs() < 1e-4, "got rot {}", s.rot);
    }

    /// Fade hardness=1 + phase+0.25 matches C++/Haxe square-wave formula.
    #[test]
    fn fade_hardness_square_and_phase_offset() {
        // hardness=1, fadeOsc=1, phase=0 → sin((t*1+0.25)*2π) = cos(t*2π)
        // at t=0: sin(π/2)=1 → hard=+1 → fade=fadeMax
        // at t=0.5: sin(0.5*2π + π/2)=sin(π+π/2)=-1 → hard=-1 → fade=fadeMin
        let p = SpriteAnimParam {
            fade_osc_per_sec: 1.0,
            fade_hardness: 1.0,
            fade_min: 0.2,
            fade_max: 0.8,
            fade_phase: 0.0,
            duration_sec: 0.0,
            ..Default::default()
        };
        let a = p.sample(0.0, 0.0);
        let b = p.sample(0.5, 0.0);
        assert!((a.fade - 0.8).abs() < 1e-4, "t=0 fade {}", a.fade);
        assert!((b.fade - 0.2).abs() < 1e-4, "t=0.5 fade {}", b.fade);

        // hardness=0 pure sine: at t=0 hardVersion=1 → still fadeMax
        let soft = SpriteAnimParam {
            fade_osc_per_sec: 1.0,
            fade_hardness: 0.0,
            fade_min: 0.0,
            fade_max: 1.0,
            fade_phase: 0.0,
            ..Default::default()
        };
        assert!((soft.sample_fade(0.0, 0.0) - 1.0).abs() < 1e-4);
        // at t=0.25: sin((0.25+0.25)*2π)=sin(π)=0 → fade=0.5
        assert!((soft.sample_fade(0.25, 0.0) - 0.5).abs() < 1e-4);
    }

    // ── P4#34 golden anim sample vectors (C++ processFrameTime / getOscOffset) ──

    /// Always-on golden vectors: analytical C++ formulas (no content tree).
    ///
    /// Locks sample(x,y,rot,fade) at fixed times so CI fails if frame_time /
    /// phase_sin / fade hardness regresses.
    #[test]
    fn golden_anim_sample_vectors_synthetic() {
        // Layer A: xOsc=3, xAmp=10, xPhase=0.25  (matches OneLifeData7 person 19_2 sprite0)
        let a = SpriteAnimParam {
            x_osc_per_sec: 3.0,
            x_amp: 10.0,
            x_phase: 0.25,
            duration_sec: 1.0,
            pause_sec: 0.0,
            fade_max: 1.0,
            ..Default::default()
        };
        // continuous time (no pause) → ft = t
        // x = 10 * sin((3t + 0.25) * 2π)
        assert_near(a.sample(0.0, 0.0).x, 10.0, 1e-4);
        assert_near(a.sample(1.0 / 12.0, 0.0).x, 0.0, 1e-4);
        assert_near(a.sample(1.0 / 6.0, 0.0).x, -10.0, 1e-4);

        // Layer B: yOsc=3, yAmp=2, yPhase=0.25, rockOsc=1.5, rockAmp=0.12
        let b = SpriteAnimParam {
            y_osc_per_sec: 3.0,
            y_amp: 2.0,
            y_phase: 0.25,
            rock_osc_per_sec: 1.5,
            rock_amp: 0.12,
            rock_phase: 0.0,
            duration_sec: 1.0,
            fade_max: 1.0,
            ..Default::default()
        };
        assert_near(b.sample(0.0, 0.0).y, 2.0, 1e-4);
        assert_near(b.sample(1.0 / 6.0, 0.0).rot, 0.12, 1e-4);

        // Layer C: fadeOsc=3, fadeMin=0.85, fadeMax=1, fadePhase=0.5, hardness=0
        let c = SpriteAnimParam {
            fade_osc_per_sec: 3.0,
            fade_hardness: 0.0,
            fade_min: 0.85,
            fade_max: 1.0,
            fade_phase: 0.5,
            duration_sec: 0.0,
            ..Default::default()
        };
        // sample_fade uses phase+0.25 → 0.75; t=0 → sin(1.5π)=-1 → fade=fadeMin
        assert_near(c.sample(0.0, 0.0).fade, 0.85, 1e-4);

        // Pause freeze golden: duration=1 pause=1 rot_per_sec=1 → t=1.5 freezes ft=1
        let d = SpriteAnimParam {
            duration_sec: 1.0,
            pause_sec: 1.0,
            rot_per_sec: 1.0,
            fade_max: 1.0,
            ..Default::default()
        };
        assert_near(d.frame_time(1.5), 1.0, 1e-5);
        assert_near(d.sample(1.5, 0.0).rot, 1.0, 1e-4);
    }

    /// Live OneLifeData7 vectors when the content tree is present (skip otherwise).
    ///
    /// Parses person `19_2` (moving) and asserts:
    /// - sprite count / first animated layers match shipped data
    /// - sample outputs match the synthetic golden formulas above
    /// - OLA1 roundtrip preserves samples bit-for-bit at probe times
    #[test]
    fn golden_anim_sample_vectors_onelife_data7() {
        let roots = [
            r"C:\OhOl\OpenLife\OneLifeData7",
            r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7",
        ];
        let mut root_path: Option<&Path> = None;
        for r in &roots {
            let p = Path::new(r).join("animations").join("19_2.txt");
            if p.exists() {
                root_path = Some(Path::new(r));
                break;
            }
        }
        let Some(root) = root_path else {
            // CI without content tree: synthetic golden still covers formulas.
            return;
        };

        let text = fs::read_to_string(root.join("animations").join("19_2.txt")).unwrap();
        let anim = parse_animation_txt(19, ANIM_MOVING, -1, &text).unwrap();
        assert_eq!(anim.object_id, 19);
        assert_eq!(anim.anim_type, ANIM_MOVING);
        assert_eq!(
            anim.sprite_params.len(),
            92,
            "person 19 moving sprite count"
        );

        // Sprite 0: xOsc=3, xAmp=10, xPhase=0.25 (from shipped OneLifeData7)
        let s0 = &anim.sprite_params[0];
        assert_near(s0.x_osc_per_sec, 3.0, 1e-5);
        assert_near(s0.x_amp, 10.0, 1e-5);
        assert_near(s0.x_phase, 0.25, 1e-5);
        assert_near(s0.sample(0.0, 0.0).x, 10.0, 1e-3);
        assert_near(s0.sample(1.0 / 12.0, 0.0).x, 0.0, 1e-3);
        assert_near(s0.sample(1.0 / 6.0, 0.0).x, -10.0, 1e-3);

        // Sprite 1: xOsc=1.5, xAmp=20, xPhase=0.5
        let s1 = &anim.sprite_params[1];
        assert_near(s1.x_osc_per_sec, 1.5, 1e-5);
        assert_near(s1.x_amp, 20.0, 1e-5);
        assert_near(s1.x_phase, 0.5, 1e-5);
        // t=0: sin(0.5*2π)=0 → x=0
        assert_near(s1.sample(0.0, 0.0).x, 0.0, 1e-3);
        // t=1/3: 1.5*(1/3)+0.5 = 1.0 → sin(2π)=0 → still 0; use t=1/6:
        // 1.5/6 + 0.5 = 0.75 → sin(1.5π)=-1 → x=-20
        assert_near(s1.sample(1.0 / 6.0, 0.0).x, -20.0, 1e-3);

        // Find first layer with yAmp>0 (body rock) and verify golden y at t=0
        let y_layer = anim
            .sprite_params
            .iter()
            .find(|p| p.y_amp > 0.5)
            .expect("19_2 should have y-osc layers");
        assert_near(y_layer.y_osc_per_sec, 3.0, 1e-5);
        assert_near(y_layer.y_amp, 2.0, 1e-5);
        assert_near(y_layer.y_phase, 0.25, 1e-5);
        assert_near(y_layer.sample(0.0, 0.0).y, 2.0, 1e-3);

        // Ground 19_0: first layer xOsc=3, xAmp=0 (static) — sample stays 0
        let g_text = fs::read_to_string(root.join("animations").join("19_0.txt")).unwrap();
        let ground = parse_animation_txt(19, ANIM_GROUND, -1, &g_text).unwrap();
        assert_eq!(ground.sprite_params.len(), 92);
        let g0 = &ground.sprite_params[0];
        assert_near(g0.x_osc_per_sec, 3.0, 1e-5);
        assert_near(g0.x_amp, 0.0, 1e-5);
        assert_near(g0.sample(0.5, 0.0).x, 0.0, 1e-4);

        // OLA1 roundtrip preserves sample vectors at probe times
        let mut bank = AnimBank::new(root);
        bank.insert(anim.clone());
        let bytes = bank.write_ola1(0);
        let recs = load_ola1(&bytes).unwrap();
        let back = recs.iter().find(|r| r.object_id == 19 && r.anim_type == ANIM_MOVING).unwrap();
        let times = [0.0_f32, 1.0 / 12.0, 1.0 / 6.0, 0.5, 1.0];
        for (si, (orig, loaded)) in anim
            .sprite_params
            .iter()
            .zip(back.sprite_params.iter())
            .enumerate()
            .take(12)
        {
            for &t in &times {
                let o = orig.sample(t, 0.0);
                let l = loaded.sample(t, 0.0);
                assert_near(o.x, l.x, 1e-4);
                assert_near(o.y, l.y, 1e-4);
                assert_near(o.rot, l.rot, 1e-4);
                assert_near(o.fade, l.fade, 1e-4);
                let _ = si;
            }
        }
    }

    fn assert_near(got: f32, want: f32, eps: f32) {
        assert!(
            (got - want).abs() <= eps,
            "got {got} want {want} (eps {eps})"
        );
    }

    #[test]
    fn sample_carries_rot_center() {
        let p = SpriteAnimParam {
            rot_center_x: 3.0,
            rot_center_y: -4.0,
            ..Default::default()
        };
        let s = p.sample(0.0, 0.0);
        assert!((s.rot_center_x - 3.0).abs() < 1e-5);
        assert!((s.rot_center_y + 4.0).abs() < 1e-5);
        assert!((s.fade - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ground2_aliases_to_ground() {
        let mut bank = AnimBank::new(".");
        bank.insert(ObjectAnimation {
            object_id: 7,
            anim_type: ANIM_GROUND,
            extra_index: -1,
            sprite_params: vec![SpriteAnimParam {
                x_amp: 9.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        let g = bank.get(7, ANIM_GROUND2).unwrap();
        assert!((g.sprite_params[0].x_amp - 9.0).abs() < 1e-5);
        let s = bank.sample_sprite(7, ANIM_GROUND2, 0, 0.0);
        assert!((s.x - 0.0).abs() < 1e-3); // phase 0 offset
    }

    #[test]
    fn load_prefer_cache_rejects_sha1_mismatch() {
        let tmp = std::env::temp_dir().join(format!("ohol_ola1_sha_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("cache")).unwrap();
        fs::write(tmp.join("dataVersionNumber.txt"), "42\n").unwrap();

        let mut bank = AnimBank::new(&tmp);
        bank.insert(ObjectAnimation {
            object_id: 1,
            anim_type: 0,
            extra_index: -1,
            sprite_params: vec![SpriteAnimParam {
                x_amp: 3.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        let bytes = bank.write_ola1(42);
        fs::write(tmp.join("cache").join("ola1_anims.bin"), &bytes).unwrap();
        // Wrong sha1 in manifest
        fs::write(
            tmp.join("cache").join("manifest.json"),
            r#"{
  "format": 1,
  "data_version": 42,
  "created_utc": "unix:0",
  "source": "test",
  "blobs": {
    "ola1_anims.bin": { "sha1": "deadbeef", "bytes": 1, "count": 1 }
  }
}"#,
        )
        .unwrap();

        let loaded = AnimBank::load_prefer_cache(&tmp);
        // Mismatch → text fallback (empty, binary_only false)
        assert!(loaded.is_empty());
        assert!(!loaded.binary_only);

        // Correct sha1 → loads
        let h = {
            use sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        };
        fs::write(
            tmp.join("cache").join("manifest.json"),
            format!(
                r#"{{
  "format": 1,
  "data_version": 42,
  "created_utc": "unix:0",
  "source": "test",
  "blobs": {{
    "ola1_anims.bin": {{ "sha1": "{h}", "bytes": {}, "count": 1 }}
  }}
}}"#,
                bytes.len()
            ),
        )
        .unwrap();
        let ok = AnimBank::load_prefer_cache(&tmp);
        assert_eq!(ok.len(), 1);
        assert!(ok.binary_only);

        let _ = fs::remove_dir_all(&tmp);
    }
}
