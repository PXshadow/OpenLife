//! P5#36 Loading progress UI — stages, optional callback, soft-FB bar.
//!
//! Stages mirror the bank load order used by play paths:
//! content (OLC1/OLT1 prefer_cache/rebake) → anim (OLA1) → ground (OLG1) →
//! sprites (OLS1 meta) → sounds (OLSN index) → music (OGG index scan).
//!
//! Prefer-cache / bank hooks accept an optional [`ProgressCb`]; when `None`,
//! load paths are unchanged (no allocation, no I/O). Headless may print lines
//! when `OHOL_LOAD_PROGRESS=1`. Soft-FB draw lives here for ohol-client boot.
//!
//! // C++: LoadingPage::{setCurrentPhase,setCurrentProgress,draw}

use std::path::{Path, PathBuf};

use crate::anim_bank::AnimBank;
use crate::content::ClientContent;
use crate::ground_sprites::GroundBank;
use crate::hud::draw_pencil_string;
use crate::music_bank::MusicBank;
use crate::render::Framebuffer;
use crate::sound_bank::SoundBank;
use crate::sprite_bank::SpriteBank;

/// Ordered boot stages (equal weight in overall fraction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LoadStage {
    Content = 0,
    Anim = 1,
    Ground = 2,
    Sprites = 3,
    Sounds = 4,
    Music = 5,
}

impl LoadStage {
    pub const COUNT: usize = 6;

    pub const ALL: [LoadStage; 6] = [
        LoadStage::Content,
        LoadStage::Anim,
        LoadStage::Ground,
        LoadStage::Sprites,
        LoadStage::Sounds,
        LoadStage::Music,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as u8 as usize
    }

    /// Short stage name for labels / log lines.
    pub fn name(self) -> &'static str {
        match self {
            LoadStage::Content => "content",
            LoadStage::Anim => "anim",
            LoadStage::Ground => "ground",
            LoadStage::Sprites => "sprites",
            LoadStage::Sounds => "sounds",
            LoadStage::Music => "music",
        }
    }

    /// Human-facing label (progress bar / UI).
    pub fn label(self) -> &'static str {
        match self {
            LoadStage::Content => "Loading content…",
            LoadStage::Anim => "Loading animations…",
            LoadStage::Ground => "Loading ground…",
            LoadStage::Sprites => "Loading sprites…",
            LoadStage::Sounds => "Loading sounds…",
            LoadStage::Music => "Loading music…",
        }
    }

    pub fn next(self) -> Option<LoadStage> {
        let i = self.index() + 1;
        if i < Self::COUNT {
            Some(Self::ALL[i])
        } else {
            None
        }
    }
}

/// Snapshot passed to progress callbacks and soft-FB draw.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadingState {
    pub stage: LoadStage,
    /// Within-stage progress in `0.0..=1.0`.
    pub stage_fraction: f32,
    /// Overall boot progress across all stages in `0.0..=1.0`.
    pub overall_fraction: f32,
    /// Display label (stage default or custom detail).
    pub label: String,
    /// True after [`LoadProgress::finish_all`].
    pub done: bool,
}

impl LoadingState {
    /// Build a state for `stage` at `stage_fraction` (clamped).
    pub fn for_stage(stage: LoadStage, stage_fraction: f32, detail: Option<&str>) -> Self {
        let stage_fraction = stage_fraction.clamp(0.0, 1.0);
        let n = LoadStage::COUNT as f32;
        let overall = ((stage.index() as f32) + stage_fraction) / n;
        let label = match detail {
            Some(d) if !d.is_empty() => format!("{} ({})", stage.label(), d),
            _ => stage.label().to_string(),
        };
        Self {
            stage,
            stage_fraction,
            overall_fraction: overall.clamp(0.0, 1.0),
            label,
            done: false,
        }
    }

    /// Fully complete state (overall = 1, done).
    pub fn finished() -> Self {
        Self {
            stage: LoadStage::Music,
            stage_fraction: 1.0,
            overall_fraction: 1.0,
            label: "Ready".to_string(),
            done: true,
        }
    }
}

/// Mutable progress tracker with stage advance helpers.
#[derive(Debug, Clone)]
pub struct LoadProgress {
    state: LoadingState,
}

impl Default for LoadProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadProgress {
    pub fn new() -> Self {
        Self {
            state: LoadingState::for_stage(LoadStage::Content, 0.0, None),
        }
    }

    pub fn state(&self) -> &LoadingState {
        &self.state
    }

    /// Begin / update the given stage at `stage_fraction` with optional detail.
    pub fn set_stage(&mut self, stage: LoadStage, stage_fraction: f32, detail: Option<&str>) {
        self.state = LoadingState::for_stage(stage, stage_fraction, detail);
    }

    /// Start a stage at fraction 0 (stage advance).
    pub fn begin_stage(&mut self, stage: LoadStage) {
        self.set_stage(stage, 0.0, None);
    }

    /// Mark stage complete (fraction 1.0) without advancing to the next stage.
    pub fn complete_stage(&mut self, stage: LoadStage) {
        self.set_stage(stage, 1.0, Some("done"));
    }

    /// Finish current stage and move to the next (if any). Returns the new stage.
    pub fn advance(&mut self) -> Option<LoadStage> {
        let cur = self.state.stage;
        self.complete_stage(cur);
        match cur.next() {
            Some(n) => {
                self.begin_stage(n);
                Some(n)
            }
            None => {
                self.finish_all();
                None
            }
        }
    }

    /// Mark entire boot complete.
    pub fn finish_all(&mut self) {
        self.state = LoadingState::finished();
    }

    /// Emit current state to an optional callback.
    pub fn emit(&self, on_progress: ProgressCb<'_>) {
        if let Some(cb) = on_progress {
            cb(&self.state);
        }
    }

    /// Update stage + emit in one call.
    pub fn report(
        &mut self,
        stage: LoadStage,
        stage_fraction: f32,
        detail: Option<&str>,
        on_progress: ProgressCb<'_>,
    ) {
        self.set_stage(stage, stage_fraction, detail);
        self.emit(on_progress);
    }
}

/// Optional progress sink. `None` = silent (default play / headless).
pub type ProgressCb<'a> = Option<&'a mut dyn FnMut(&LoadingState)>;

/// Short reborrow of a progress callback for one call site.
///
/// Prefer this over `Option::as_deref_mut` — the latter can pin the outer
/// `ProgressCb` lifetime and block subsequent reborrows in the same function.
#[inline]
pub fn reborrow_cb<'a>(on_progress: &'a mut ProgressCb<'_>) -> ProgressCb<'a> {
    match on_progress {
        Some(cb) => Some(&mut **cb),
        None => None,
    }
}

/// Call `cb` if present.
#[inline]
pub fn emit_progress(on_progress: ProgressCb<'_>, state: &LoadingState) {
    if let Some(cb) = on_progress {
        cb(state);
    }
}

/// Report a single-stage load (start → end) through `on_progress`.
///
/// Prefer-cache hooks use this so a full load emits 0.0 then 1.0 for the stage.
pub fn report_stage(
    stage: LoadStage,
    stage_fraction: f32,
    detail: Option<&str>,
    on_progress: ProgressCb<'_>,
) {
    let state = LoadingState::for_stage(stage, stage_fraction, detail);
    emit_progress(on_progress, &state);
}

/// `true` when `OHOL_LOAD_PROGRESS` is `1` / `true` / `yes` / `on` (case-insensitive).
pub fn load_progress_env_enabled() -> bool {
    match std::env::var("OHOL_LOAD_PROGRESS") {
        Ok(v) => {
            let t = v.trim();
            t == "1"
                || t.eq_ignore_ascii_case("true")
                || t.eq_ignore_ascii_case("yes")
                || t.eq_ignore_ascii_case("on")
        }
        Err(_) => false,
    }
}

/// One-line headless log for a progress snapshot.
pub fn format_progress_line(state: &LoadingState) -> String {
    format!(
        "load: {:>8} {:5.1}%  {}",
        state.stage.name(),
        state.overall_fraction * 100.0,
        state.label
    )
}

/// Print progress to stderr (headless `OHOL_LOAD_PROGRESS=1` sink).
pub fn log_progress_line(state: &LoadingState) {
    eprintln!("{}", format_progress_line(state));
}

/// Build a callback that logs when env is enabled. Returns `None` if disabled
/// so callers can skip dyn construction entirely when silent.
///
/// Note: callers typically check [`load_progress_env_enabled`] and close over
/// `log_progress_line` themselves; this helper is for unit tests / simple use.
pub fn env_log_callback() -> Option<Box<dyn FnMut(&LoadingState)>> {
    if load_progress_env_enabled() {
        Some(Box::new(|s: &LoadingState| log_progress_line(s)))
    } else {
        None
    }
}

// ── full prefer_cache boot (shared headless / graphics) ──────────────────────

/// Banks produced by a progressive prefer_cache boot (indexes only where lazy).
pub struct BootBanks {
    pub content: ClientContent,
    pub anims: AnimBank,
    pub sprites: SpriteBank,
    pub ground: GroundBank,
    pub sounds: SoundBank,
    pub music: MusicBank,
    pub content_root: PathBuf,
    /// True when `cache/olc1_objects.bin` exists after load.
    pub used_binary_cache: bool,
}

/// Progressive prefer_cache boot used by ohol-client loading UI + load_bench tests.
///
/// Order: content → anim → ground → sprites → sounds → music.
/// Does **not** open AIFF/TGA/OGG for full decode — index/meta only.
pub fn boot_load_prefer_cache(
    root: impl AsRef<Path>,
    mut on_progress: ProgressCb<'_>,
) -> Result<BootBanks, String> {
    let root = root.as_ref();
    let mut tracker = LoadProgress::new();

    tracker.report(
        LoadStage::Content,
        0.0,
        Some("prefer_cache"),
        reborrow_cb(&mut on_progress),
    );
    let content =
        ClientContent::load_prefer_cache_with_progress(root, reborrow_cb(&mut on_progress))?;
    tracker.report(
        LoadStage::Content,
        1.0,
        Some(&format!(
            "objects={} transitions={}",
            content.objects.len(),
            content.transitions.len()
        )),
        reborrow_cb(&mut on_progress),
    );

    let anims = AnimBank::load_prefer_cache_with_progress(root, reborrow_cb(&mut on_progress));
    let ground = GroundBank::load_prefer_cache_with_progress(root, reborrow_cb(&mut on_progress));
    let sprites = SpriteBank::load_prefer_cache_with_progress(root, reborrow_cb(&mut on_progress));
    let sounds = SoundBank::load_prefer_cache_with_progress(root, reborrow_cb(&mut on_progress));
    debug_assert_eq!(
        sounds.aiff_opens, 0,
        "boot must not open AIFF (lazy OLSN)"
    );
    let music = MusicBank::load_prefer_scan_with_progress(root, reborrow_cb(&mut on_progress));
    debug_assert_eq!(
        music.ogg_opens, 0,
        "boot must not open music OGG (lazy)"
    );

    tracker.finish_all();
    tracker.emit(reborrow_cb(&mut on_progress));

    let used_binary_cache = crate::content_binary::cache_dir_for(root)
        .join("olc1_objects.bin")
        .exists();

    Ok(BootBanks {
        content,
        anims,
        sprites,
        ground,
        sounds,
        music,
        content_root: root.to_path_buf(),
        used_binary_cache,
    })
}

/// Assert collected overall fractions are non-decreasing and finish near 1.0.
pub fn fractions_monotonic(events: &[LoadingState]) -> bool {
    if events.is_empty() {
        return false;
    }
    let mut prev = 0.0f32;
    for e in events {
        if e.overall_fraction + 0.002 < prev {
            return false;
        }
        prev = e.overall_fraction;
    }
    events
        .last()
        .map(|e| e.done || e.overall_fraction >= 0.99)
        .unwrap_or(false)
}

// ── soft-FB draw ─────────────────────────────────────────────────────────────

/// Draw loading bar + label into a soft framebuffer (dark boot screen).
///
/// C++ `LoadingPage::draw`: "LOADING" title, phase name, white-border progress bar.
/// Used by `ohol-client` before the live world loop.
pub fn draw_loading_progress(fb: &mut Framebuffer, state: &LoadingState) {
    // Near-black navy background (C++ LoadingPage dark field).
    fb.clear([16, 16, 24, 255]);
    let w = fb.width as i32;
    let h = fb.height as i32;
    if w < 8 || h < 8 {
        return;
    }

    let scale = (w.min(h) as f32 / 540.0).clamp(0.75, 2.0);
    let cx = w as f32 * 0.5;
    let title_y = h as f32 * 0.38;

    // C++ drawMessage("LOADING") at center
    draw_pencil_string(
        fb,
        "LOADING",
        cx,
        title_y,
        2.4 * scale,
        [235, 235, 230, 255],
        true,
    );

    // Phase label under title (C++ mPhaseName)
    draw_pencil_string(
        fb,
        &state.label,
        cx,
        title_y + 36.0 * scale,
        1.5 * scale,
        [210, 210, 200, 255],
        true,
    );

    // Progress bar — C++ white border, black inner, gray fill (-100..100 × -220..-200).
    let bar_half_w = (100.0 * scale).round() as i32;
    let bar_h = (20.0 * scale).round().max(10.0) as i32;
    let bar_cx = w / 2;
    let bar_cy = (h as f32 * 0.62).round() as i32;
    let x0 = bar_cx - bar_half_w;
    let y0 = bar_cy - bar_h / 2;
    let bw = bar_half_w * 2;
    // Outer white border
    fb.fill_rect(x0, y0, bw, bar_h, [255, 255, 255, 255]);
    let inset = (2.0 * scale).round().max(1.0) as i32;
    fb.fill_rect(
        x0 + inset,
        y0 + inset,
        bw - inset * 2,
        bar_h - inset * 2,
        [0, 0, 0, 255],
    );
    let inner_w = (bw - inset * 2).max(0);
    let fill_w =
        ((state.overall_fraction.clamp(0.0, 1.0) * inner_w as f32).round() as i32).clamp(0, inner_w);
    if fill_w > 0 {
        // C++ gray 0.8 (slightly green-tinted for readability on soft-FB)
        fb.fill_rect(
            x0 + inset,
            y0 + inset,
            fill_w,
            bar_h - inset * 2,
            [204, 210, 200, 255],
        );
    }

    // Overall percent under bar
    let pct = format!("{:.0}%", state.overall_fraction * 100.0);
    draw_pencil_string(
        fb,
        &pct,
        cx,
        bar_cy as f32 + bar_h as f32 * 0.5 + 18.0 * scale,
        1.4 * scale,
        [170, 180, 170, 255],
        true,
    );

    // Stage chips (name of each stage; current highlighted)
    let chip_y = bar_cy as f32 + bar_h as f32 * 0.5 + 42.0 * scale;
    let chip_step = (w as f32) / (LoadStage::COUNT as f32 + 1.0);
    for (i, st) in LoadStage::ALL.iter().enumerate() {
        let x = chip_step * (i as f32 + 1.0);
        let active = *st == state.stage || (state.done && i == LoadStage::COUNT - 1);
        let done_chip = st.index() < state.stage.index() || state.done;
        let col = if active {
            [240, 240, 200, 255]
        } else if done_chip {
            [100, 160, 100, 255]
        } else {
            [90, 90, 110, 255]
        };
        draw_pencil_string(fb, st.name(), x, chip_y, 1.1 * scale, col, true);
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_order_and_labels() {
        assert_eq!(LoadStage::COUNT, 6);
        assert_eq!(LoadStage::Content.index(), 0);
        assert_eq!(LoadStage::Music.index(), 5);
        assert_eq!(LoadStage::Content.next(), Some(LoadStage::Anim));
        assert_eq!(LoadStage::Music.next(), None);
        assert!(!LoadStage::Content.label().is_empty());
        assert_eq!(LoadStage::Sprites.name(), "sprites");
    }

    #[test]
    fn stage_fraction_maps_to_overall() {
        let s0 = LoadingState::for_stage(LoadStage::Content, 0.0, None);
        assert!((s0.overall_fraction - 0.0).abs() < 1e-5);
        let s_mid = LoadingState::for_stage(LoadStage::Content, 0.5, None);
        assert!((s_mid.overall_fraction - 0.5 / 6.0).abs() < 1e-5);
        let s_anim = LoadingState::for_stage(LoadStage::Anim, 0.0, None);
        assert!((s_anim.overall_fraction - 1.0 / 6.0).abs() < 1e-5);
        let s_last = LoadingState::for_stage(LoadStage::Music, 1.0, None);
        assert!((s_last.overall_fraction - 1.0).abs() < 1e-5);
    }

    #[test]
    fn load_progress_advance_stages() {
        let mut p = LoadProgress::new();
        assert_eq!(p.state().stage, LoadStage::Content);
        assert!((p.state().stage_fraction - 0.0).abs() < 1e-5);
        assert!(!p.state().done);

        p.report(LoadStage::Content, 0.5, Some("cache"), None);
        assert_eq!(p.state().stage, LoadStage::Content);
        assert!((p.state().stage_fraction - 0.5).abs() < 1e-5);
        assert!(p.state().label.contains("cache"));

        assert_eq!(p.advance(), Some(LoadStage::Anim));
        assert_eq!(p.state().stage, LoadStage::Anim);
        assert!((p.state().stage_fraction - 0.0).abs() < 1e-5);

        assert_eq!(p.advance(), Some(LoadStage::Ground));
        assert_eq!(p.advance(), Some(LoadStage::Sprites));
        assert_eq!(p.advance(), Some(LoadStage::Sounds));
        assert_eq!(p.advance(), Some(LoadStage::Music));
        assert_eq!(p.advance(), None);
        assert!(p.state().done);
        assert!((p.state().overall_fraction - 1.0).abs() < 1e-5);
        assert_eq!(p.state().label, "Ready");
    }

    #[test]
    fn progress_callback_receives_updates() {
        let mut seen: Vec<(LoadStage, i32)> = Vec::new();
        {
            let mut cb = |s: &LoadingState| {
                seen.push((s.stage, (s.stage_fraction * 100.0).round() as i32));
            };
            let mut p = LoadProgress::new();
            p.report(LoadStage::Content, 0.0, None, Some(&mut cb));
            p.report(LoadStage::Content, 1.0, Some("done"), Some(&mut cb));
            p.report(LoadStage::Anim, 0.25, None, Some(&mut cb));
        }
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0], (LoadStage::Content, 0));
        assert_eq!(seen[1], (LoadStage::Content, 100));
        assert_eq!(seen[2], (LoadStage::Anim, 25));
    }

    #[test]
    fn report_stage_helper_emits() {
        let mut last_overall = -1.0f32;
        {
            let mut cb = |s: &LoadingState| {
                last_overall = s.overall_fraction;
            };
            report_stage(LoadStage::Sprites, 1.0, Some("ols1"), Some(&mut cb));
        }
        // Sprites is index 3 → overall = (3+1)/6 = 4/6
        assert!((last_overall - 4.0 / 6.0).abs() < 1e-5);
    }

    #[test]
    fn format_progress_line_contains_stage() {
        let s = LoadingState::for_stage(LoadStage::Ground, 0.5, Some("olg1"));
        let line = format_progress_line(&s);
        assert!(line.contains("ground"), "{line}");
        assert!(line.contains("olg1") || line.contains("Loading"), "{line}");
    }

    #[test]
    fn draw_loading_progress_paints_bar() {
        let mut fb = Framebuffer::new(320, 180);
        let state = LoadingState::for_stage(LoadStage::Anim, 0.5, None);
        draw_loading_progress(&mut fb, &state);
        // Background is dark navy — green fill should introduce non-navy pixels.
        let non = fb.count_non_color([16, 16, 24, 255]);
        assert!(non > 50, "expected bar/label pixels, got {non}");
        // Finished state still draws.
        draw_loading_progress(&mut fb, &LoadingState::finished());
        let non2 = fb.count_non_color([16, 16, 24, 255]);
        assert!(non2 > 50);
    }

    #[test]
    fn clamp_fraction_out_of_range() {
        let low = LoadingState::for_stage(LoadStage::Content, -1.0, None);
        assert!((low.stage_fraction - 0.0).abs() < 1e-5);
        let high = LoadingState::for_stage(LoadStage::Content, 2.0, None);
        assert!((high.stage_fraction - 1.0).abs() < 1e-5);
    }

    #[test]
    fn boot_load_synthetic_tree_progress_monotonic() {
        let tmp = std::env::temp_dir().join(format!(
            "ohol_boot_load_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("objects")).unwrap();
        std::fs::create_dir_all(tmp.join("transitions")).unwrap();
        std::fs::write(tmp.join("dataVersionNumber.txt"), "1").unwrap();
        std::fs::write(
            tmp.join("objects").join("1.txt"),
            "id=1\nname=Test\ncontainable=0\npermanent=0\n",
        )
        .unwrap();
        std::fs::write(tmp.join("objects").join("nextObjectNumber.txt"), "2").unwrap();

        let mut events = Vec::new();
        {
            let mut cb = |s: &LoadingState| {
                events.push(s.clone());
            };
            let result = boot_load_prefer_cache(&tmp, Some(&mut cb));
            let _ = std::fs::remove_dir_all(&tmp);
            let banks = match result {
                Ok(b) => b,
                Err(e) => panic!("boot_load_prefer_cache failed: {e}"),
            };
            assert_eq!(banks.sounds.aiff_opens, 0);
            assert_eq!(banks.music.ogg_opens, 0);
        }
        assert!(!events.is_empty());
        assert!(
            fractions_monotonic(&events),
            "events={:?}",
            events
                .iter()
                .map(|e| (e.stage.name(), e.overall_fraction))
                .collect::<Vec<_>>()
        );
        assert!(events.iter().any(|e| e.stage == LoadStage::Content));
        assert!(events.iter().any(|e| e.stage == LoadStage::Sounds));
        assert!(events.last().unwrap().done || events.last().unwrap().overall_fraction >= 0.99);
    }

    #[test]
    fn draw_loading_has_white_bar_border() {
        let mut fb = Framebuffer::new(320, 180);
        let state = LoadingState::for_stage(LoadStage::Sprites, 0.75, Some("ols1"));
        draw_loading_progress(&mut fb, &state);
        let mut white = 0usize;
        for chunk in fb.pixels.chunks(4) {
            if chunk[0] == 255 && chunk[1] == 255 && chunk[2] == 255 {
                white += 1;
            }
        }
        assert!(white > 10, "expected C++-style white bar border, white={white}");
    }
}
