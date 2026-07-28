//! P5#39 — Settings page (C++ `SettingsPage` stand-in).
//!
//! Soft-FB options: GPU/Soft present, audio device on/off, SFX/music volume + mute,
//! show FPS, host/port/email display.
//! Open: F3 from Account or Playing. Esc/Back returns. Persist: ohol_client_settings.ini.
//! Defaults: **GPU present** + **audio device on**. Switch either off from Settings.
//! Headless CLI / OHOL_* session flags unchanged. No new crates.
//!
//! // C++: SettingsPage soundLoudness / musicLoudness / musicOff (subset)

use std::path::{Path, PathBuf};

use crate::account_page::ClientScreen;
use crate::hud::{draw_pencil_string, HudSprites};
use crate::music_bank::MusicBank;
use crate::render::{Framebuffer, ZOOM_DEFAULT, ZOOM_MAX, ZOOM_MIN};
use crate::sound_bank::SoundBank;

/// Default relative path for client settings (cwd).
pub const CLIENT_SETTINGS_FILE: &str = "ohol_client_settings.ini";

/// How the windowed client presents pixels.
///
/// - **Gpu** — soft-FB scene still authored on CPU, presented via **wgpu/`pixels`**
///   (texture upload + GPU scale). Faster present, smoother upscale; default when available.
/// - **Soft** — classic minifb CPU buffer path (debug / fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GraphicsMode {
    /// GPU-accelerated present (default).
    #[default]
    Gpu,
    /// Software minifb present.
    Soft,
}

impl GraphicsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            GraphicsMode::Gpu => "gpu",
            GraphicsMode::Soft => "soft",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gpu" | "wgpu" | "hardware" | "1" | "true" | "on" => Some(GraphicsMode::Gpu),
            "soft" | "software" | "cpu" | "minifb" | "0" | "false" | "off" => {
                Some(GraphicsMode::Soft)
            }
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GraphicsMode::Gpu => "GPU present",
            GraphicsMode::Soft => "Soft (CPU)",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            GraphicsMode::Gpu => GraphicsMode::Soft,
            GraphicsMode::Soft => GraphicsMode::Gpu,
        }
    }
}

/// Soft-FB Settings form + committed options.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsPage {
    pub sound_volume: f32,
    pub music_volume: f32,
    pub sound_muted: bool,
    pub music_muted: bool,
    pub show_fps: bool,
    /// When true, Playing shows F9/SNAP play-snapshot tools.
    pub debug: bool,
    /// Screen pixels per world tile (camera zoom). Persist + apply on play.
    pub zoom: f32,
    /// Soft vs GPU present (takes effect on next client start). Default: GPU.
    pub graphics_mode: GraphicsMode,
    /// Master device audio on/off (default **on**). Off skips cpal; volumes/mutes still apply when on.
    pub audio_enabled: bool,
    /// Borderless fullscreen for the play window (GPU path; soft uses max scale).
    pub fullscreen: bool,
    pub host: String,
    pub port: u16,
    pub email: String,
    pub window_w: u32,
    pub window_h: u32,
    /// Build-time: crate compiled with `--features audio` (now default).
    pub audio_feature: bool,
    pub focus: SettingsFocus,
    pub status: String,
}

pub type ClientSettings = SettingsPage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsFocus {
    SoundVolume,
    MusicVolume,
    Zoom,
    Graphics,
    Audio,
    Fullscreen,
    SoundMute,
    MusicMute,
    ShowFps,
    Debug,
    Credentials,
    Back,
}

impl SettingsFocus {
    const ALL: [SettingsFocus; 12] = [
        SettingsFocus::SoundVolume,
        SettingsFocus::MusicVolume,
        SettingsFocus::Zoom,
        SettingsFocus::Graphics,
        SettingsFocus::Audio,
        SettingsFocus::Fullscreen,
        SettingsFocus::SoundMute,
        SettingsFocus::MusicMute,
        SettingsFocus::ShowFps,
        SettingsFocus::Debug,
        SettingsFocus::Credentials,
        SettingsFocus::Back,
    ];
    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
    fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsKey {
    ToggleAudio,
    ToggleMusic,
    Back,
    Tab { shift: bool },
    Up,
    Down,
    Left,
    Right,
    Plus,
    Minus,
    Enter,
    Escape,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    None,
    Back,
    Applied,
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            sound_volume: 1.0,
            music_volume: 1.0,
            sound_muted: false,
            music_muted: false,
            show_fps: true,
            debug: false,
            zoom: ZOOM_DEFAULT,
            graphics_mode: GraphicsMode::Gpu,
            audio_enabled: true,
            fullscreen: false,
            host: "127.0.0.1".into(),
            port: 8005,
            email: String::new(),
            window_w: 960,
            window_h: 540,
            audio_feature: crate::sound_bank::audio_feature_enabled(),
            focus: SettingsFocus::SoundVolume,
            status: "Tab=row  Left/Right=adjust  +/-=zoom  Esc=Back".into(),
        }
    }
}

impl SettingsPage {
    pub fn clamp(&mut self) {
        self.sound_volume = self.sound_volume.clamp(0.0, 1.0);
        self.music_volume = self.music_volume.clamp(0.0, 1.0);
        self.zoom = self.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Normalized 0..1 for zoom slider fill.
    pub fn zoom_slider_t(&self) -> f32 {
        let span = (ZOOM_MAX - ZOOM_MIN).max(1e-3);
        ((self.zoom - ZOOM_MIN) / span).clamp(0.0, 1.0)
    }

    pub fn default_path() -> PathBuf {
        PathBuf::from(CLIENT_SETTINGS_FILE)
    }

    pub fn from_env() -> Self {
        let mut s = Self::from_env_map(|k| std::env::var(k).ok());
        if let Ok(text) = std::fs::read_to_string(Self::default_path()) {
            let file = Self::parse_ini(&text);
            if std::env::var_os("OHOL_SFX_VOLUME").is_none()
                && std::env::var_os("OHOL_SOUND_VOLUME").is_none()
            {
                s.sound_volume = file.sound_volume;
            }
            if std::env::var_os("OHOL_MUSIC_VOLUME").is_none() {
                s.music_volume = file.music_volume;
            }
            if std::env::var_os("OHOL_AUDIO_MUTE").is_none() {
                s.sound_muted = file.sound_muted;
            }
            if std::env::var_os("OHOL_MUSIC_MUTE").is_none() {
                s.music_muted = file.music_muted;
            }
            if std::env::var_os("OHOL_SHOW_FPS").is_none() {
                s.show_fps = file.show_fps;
            }
            if std::env::var_os("OHOL_DEBUG").is_none() {
                s.debug = file.debug;
            }
            if std::env::var_os("OHOL_ZOOM").is_none() {
                s.zoom = file.zoom;
            }
            if std::env::var_os("OHOL_GRAPHICS").is_none()
                && std::env::var_os("OHOL_RENDERER").is_none()
            {
                s.graphics_mode = file.graphics_mode;
            }
            if std::env::var_os("OHOL_AUDIO_DISABLE").is_none()
                && std::env::var_os("OHOL_AUDIO").is_none()
            {
                s.audio_enabled = file.audio_enabled;
            }
            if std::env::var_os("OHOL_FULLSCREEN").is_none() {
                s.fullscreen = file.fullscreen;
            }
        }
        if let Ok(g) = std::env::var("OHOL_GRAPHICS").or_else(|_| std::env::var("OHOL_RENDERER")) {
            if let Some(m) = GraphicsMode::from_str_loose(&g) {
                s.graphics_mode = m;
            }
        }
        // Master audio: OHOL_AUDIO_DISABLE always forces off; OHOL_AUDIO=0|1 overrides.
        if std::env::var_os("OHOL_AUDIO_DISABLE").is_some() {
            s.audio_enabled = false;
        } else if let Ok(a) = std::env::var("OHOL_AUDIO") {
            s.audio_enabled = env_truthy(Some(a.as_str()));
        }
        if let Ok(fs) = std::env::var("OHOL_FULLSCREEN") {
            s.fullscreen = env_truthy(Some(fs.as_str()));
        }
        s.apply_runtime_globals();
        s.clamp();
        s
    }

    pub fn from_env_map<F>(mut get: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut s = Self::default();
        if let Some(h) = get("OHOL_HOST") {
            if !h.is_empty() {
                s.host = h;
            }
        }
        if let Some(port_s) = get("OHOL_PORT") {
            if let Ok(n) = port_s.parse::<u16>() {
                s.port = n;
            }
        }
        if let Some(e) = get("OHOL_EMAIL") {
            s.email = e;
        }
        if let Some(v) = get("OHOL_SFX_VOLUME").or_else(|| get("OHOL_SOUND_VOLUME")) {
            if let Some(n) = parse_volume(&v) {
                s.sound_volume = n;
            }
        }
        if let Some(v) = get("OHOL_MUSIC_VOLUME") {
            if let Some(n) = parse_volume(&v) {
                s.music_volume = n;
            }
        }
        if env_truthy(get("OHOL_AUDIO_MUTE").as_deref()) {
            s.sound_muted = true;
        }
        if env_truthy(get("OHOL_MUSIC_MUTE").as_deref()) {
            s.music_muted = true;
        }
        if get("OHOL_AUDIO_DISABLE").is_some() {
            s.audio_enabled = false;
        } else if let Some(v) = get("OHOL_AUDIO") {
            s.audio_enabled = env_truthy(Some(v.as_str()));
        }
        if let Some(v) = get("OHOL_SHOW_FPS") {
            s.show_fps = env_truthy(Some(v.as_str()));
        }
        if let Some(v) = get("OHOL_ZOOM") {
            if let Ok(n) = v.trim().parse::<f32>() {
                s.zoom = n;
            }
        }
        if let Some(v) = get("OHOL_GRAPHICS").or_else(|| get("OHOL_RENDERER")) {
            if let Some(m) = GraphicsMode::from_str_loose(&v) {
                s.graphics_mode = m;
            }
        }
        s.clamp();
        s
    }

    pub fn sync_endpoint_from(&mut self, host: &str, port: u16, email: &str) {
        if !host.is_empty() {
            self.host = host.to_string();
        }
        if port != 0 {
            self.port = port;
        }
        self.email = email.to_string();
    }

    pub fn apply_to_banks(
        &self,
        sounds: Option<&mut SoundBank>,
        music: Option<&mut MusicBank>,
    ) {
        if let Some(sb) = sounds {
            self.apply_audio(sb);
        }
        if let Some(mb) = music {
            self.apply_music(mb);
        }
    }

    pub fn audio_muted(&self) -> bool {
        self.sound_muted
    }

    pub fn apply_audio(&self, bank: &mut SoundBank) {
        self.apply_runtime_globals();
        bank.set_loudness(self.sound_volume);
        bank.set_muted(self.sound_muted);
    }

    pub fn apply_music(&self, bank: &mut MusicBank) {
        let vol = if self.music_muted {
            0.0
        } else {
            self.music_volume.clamp(0.0, 1.0)
        };
        bank.set_loudness(vol);
        bank.set_muted(self.music_muted);
    }

    pub fn apply_runtime_globals(&self) {
        crate::sound_bank::set_sfx_muted(self.sound_muted);
        crate::sound_bank::set_music_muted(self.music_muted);
        // Master device switch (default on). Env OHOL_AUDIO_DISABLE still forces off in device path.
        crate::sound_bank::set_audio_device_enabled(self.audio_enabled);
    }

    pub fn save_default(&self) -> std::io::Result<()> {
        self.save_file(&Self::default_path())
    }

    pub fn save_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.serialize_ini())
    }

    pub fn serialize_ini(&self) -> String {
        format!(
            "# Open Life client settings (P5#39)\n\
             # Defaults: graphics=gpu, audio=1 (device on). Set graphics=soft / audio=0 to turn off.\n\
             sound_volume={:.3}\n\
             music_volume={:.3}\n\
             sound_muted={}\n\
             music_muted={}\n\
             show_fps={}\n\
             debug={}\n\
             zoom={:.3}\n\
             graphics={}\n\
             audio={}\n\
             fullscreen={}\n",
            self.sound_volume.clamp(0.0, 1.0),
            self.music_volume.clamp(0.0, 1.0),
            if self.sound_muted { "1" } else { "0" },
            if self.music_muted { "1" } else { "0" },
            if self.show_fps { "1" } else { "0" },
            if self.debug { "1" } else { "0" },
            self.zoom.clamp(ZOOM_MIN, ZOOM_MAX),
            self.graphics_mode.as_str(),
            if self.audio_enabled { "1" } else { "0" },
            if self.fullscreen { "1" } else { "0" },
        )
    }

    pub fn parse_ini(text: &str) -> Self {
        let mut s = Self::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            match k.as_str() {
                "sound_volume" | "sfx_volume" | "sfx" => {
                    if let Some(n) = parse_volume(v) {
                        s.sound_volume = n;
                    }
                }
                "music_volume" | "music" => {
                    if let Some(n) = parse_volume(v) {
                        s.music_volume = n;
                    }
                }
                "sound_muted" | "audio_muted" | "mute" => s.sound_muted = env_truthy(Some(v)),
                "music_muted" => s.music_muted = env_truthy(Some(v)),
                "show_fps" | "fps" => s.show_fps = env_truthy(Some(v)),
                "debug" | "debug_tools" => s.debug = env_truthy(Some(v)),
                "zoom" | "camera_zoom" | "view_zoom" => {
                    if let Ok(n) = v.parse::<f32>() {
                        s.zoom = n;
                    }
                }
                "graphics" | "graphics_mode" | "renderer" | "render" => {
                    if let Some(m) = GraphicsMode::from_str_loose(v) {
                        s.graphics_mode = m;
                    }
                }
                "audio" | "audio_enabled" | "audio_device" => {
                    s.audio_enabled = env_truthy(Some(v));
                }
                "fullscreen" | "full_screen" | "fs" => s.fullscreen = env_truthy(Some(v)),
                _ => {}
            }
        }
        s.clamp();
        s
    }

    pub fn on_key(&mut self, key: SettingsKey) -> SettingsAction {
        match key {
            SettingsKey::Escape | SettingsKey::Back => SettingsAction::Back,
            SettingsKey::ToggleAudio => {
                self.sound_muted = !self.sound_muted;
                self.status = if self.sound_muted {
                    "SFX muted".into()
                } else {
                    "SFX on".into()
                };
                self.apply_runtime_globals();
                SettingsAction::Applied
            }
            SettingsKey::ToggleMusic => {
                self.music_muted = !self.music_muted;
                self.status = if self.music_muted {
                    "Music muted".into()
                } else {
                    "Music on".into()
                };
                self.apply_runtime_globals();
                SettingsAction::Applied
            }
            SettingsKey::Tab { shift } => {
                self.focus = if shift {
                    self.focus.prev()
                } else {
                    self.focus.next()
                };
                SettingsAction::None
            }
            SettingsKey::Up => {
                self.focus = self.focus.prev();
                SettingsAction::None
            }
            SettingsKey::Down => {
                self.focus = self.focus.next();
                SettingsAction::None
            }
            SettingsKey::Left | SettingsKey::Minus => {
                self.nudge(-1);
                self.apply_runtime_globals();
                SettingsAction::Applied
            }
            SettingsKey::Right | SettingsKey::Plus => {
                self.nudge(1);
                self.apply_runtime_globals();
                SettingsAction::Applied
            }
            SettingsKey::Enter => match self.focus {
                SettingsFocus::SoundMute => {
                    self.sound_muted = !self.sound_muted;
                    self.status = if self.sound_muted {
                        "SFX muted".into()
                    } else {
                        "SFX on".into()
                    };
                    self.apply_runtime_globals();
                    SettingsAction::Applied
                }
                SettingsFocus::MusicMute => {
                    self.music_muted = !self.music_muted;
                    self.status = if self.music_muted {
                        "Music muted".into()
                    } else {
                        "Music on".into()
                    };
                    self.apply_runtime_globals();
                    SettingsAction::Applied
                }
                SettingsFocus::ShowFps => {
                    self.show_fps = !self.show_fps;
                    self.status = if self.show_fps {
                        "FPS in title: on".into()
                    } else {
                        "FPS in title: off".into()
                    };
                    SettingsAction::Applied
                }
                SettingsFocus::Debug => {
                    self.debug = !self.debug;
                    self.status = if self.debug {
                        "Debug tools: on (F9 SNAP)".into()
                    } else {
                        "Debug tools: off".into()
                    };
                    SettingsAction::Applied
                }
                SettingsFocus::Back => SettingsAction::Back,
                SettingsFocus::SoundVolume | SettingsFocus::MusicVolume | SettingsFocus::Zoom => {
                    self.apply_runtime_globals();
                    SettingsAction::Applied
                }
                SettingsFocus::Graphics => {
                    self.graphics_mode = self.graphics_mode.cycle();
                    self.status = format!(
                        "Graphics: {} (restart client to apply)",
                        self.graphics_mode.label()
                    );
                    SettingsAction::Applied
                }
                SettingsFocus::Audio => {
                    self.audio_enabled = !self.audio_enabled;
                    self.status = if self.audio_enabled {
                        "Audio device: ON".into()
                    } else {
                        "Audio device: OFF".into()
                    };
                    self.apply_runtime_globals();
                    SettingsAction::Applied
                }
                SettingsFocus::Fullscreen => {
                    self.fullscreen = !self.fullscreen;
                    self.status = format!(
                        "Fullscreen: {} (restart play window)",
                        if self.fullscreen { "ON" } else { "windowed" }
                    );
                    SettingsAction::Applied
                }
                SettingsFocus::Credentials => {
                    self.status = "Edit email/key on Account page".into();
                    SettingsAction::None
                }
            },
            SettingsKey::Other => SettingsAction::None,
        }
    }

    fn nudge(&mut self, dir: i32) {
        let step = 0.1_f32;
        match self.focus {
            SettingsFocus::SoundVolume => {
                self.sound_volume = (self.sound_volume + step * dir as f32).clamp(0.0, 1.0);
                self.status = format!("SFX {:.0}%", self.sound_volume * 100.0);
            }
            SettingsFocus::MusicVolume => {
                self.music_volume = (self.music_volume + step * dir as f32).clamp(0.0, 1.0);
                self.status = format!("Music {:.0}%", self.music_volume * 100.0);
            }
            SettingsFocus::Zoom => {
                // Discrete steps (~10% of range) so +/- feels like a slider tick.
                let step_z = ((ZOOM_MAX - ZOOM_MIN) * 0.1).max(1.0);
                self.zoom = (self.zoom + step_z * dir as f32).clamp(ZOOM_MIN, ZOOM_MAX);
                self.status = format!("Zoom {:.0} px/tile  (+/- or Left/Right)", self.zoom);
            }
            SettingsFocus::Graphics if dir != 0 => {
                self.graphics_mode = self.graphics_mode.cycle();
                self.status = format!(
                    "Graphics: {} (restart client to apply)",
                    self.graphics_mode.label()
                );
            }
            SettingsFocus::Audio if dir != 0 => {
                self.audio_enabled = !self.audio_enabled;
                self.status = if self.audio_enabled {
                    "Audio device: ON".into()
                } else {
                    "Audio device: OFF".into()
                };
            }
            SettingsFocus::Fullscreen if dir != 0 => {
                self.fullscreen = !self.fullscreen;
                self.status = format!(
                    "Fullscreen: {} (restart play window)",
                    if self.fullscreen { "ON" } else { "windowed" }
                );
            }
            SettingsFocus::SoundMute if dir != 0 => {
                self.sound_muted = !self.sound_muted;
                self.status = if self.sound_muted {
                    "SFX muted".into()
                } else {
                    "SFX on".into()
                };
            }
            SettingsFocus::MusicMute if dir != 0 => {
                self.music_muted = !self.music_muted;
                self.status = if self.music_muted {
                    "Music muted".into()
                } else {
                    "Music on".into()
                };
            }
            SettingsFocus::ShowFps if dir != 0 => {
                self.show_fps = !self.show_fps;
                self.status = if self.show_fps {
                    "FPS in title: on".into()
                } else {
                    "FPS in title: off".into()
                };
            }
            SettingsFocus::Debug if dir != 0 => {
                self.debug = !self.debug;
                self.status = if self.debug {
                    "Debug tools: on (F9 SNAP)".into()
                } else {
                    "Debug tools: off".into()
                };
            }
            _ => {}
        }
        self.clamp();
    }

    pub fn draw(&self, fb: &mut Framebuffer, sprites: Option<&HudSprites>) {
        let _ = sprites;
        draw_settings_screen(fb, self);
    }
}

fn env_truthy(v: Option<&str>) -> bool {
    match v {
        Some(s) => {
            let t = s.trim();
            t == "1"
                || t.eq_ignore_ascii_case("true")
                || t.eq_ignore_ascii_case("yes")
                || t.eq_ignore_ascii_case("on")
                || t.eq_ignore_ascii_case("y")
        }
        None => false,
    }
}

fn parse_volume(s: &str) -> Option<f32> {
    let t = s.trim().trim_end_matches('%').trim();
    let v: f32 = t.parse().ok()?;
    if v > 1.0 {
        Some((v / 100.0).clamp(0.0, 1.0))
    } else {
        Some(v.clamp(0.0, 1.0))
    }
}

/// Soft-FB horizontal slider (track + fill + knob). `t` is 0..1.
fn draw_settings_slider(fb: &mut Framebuffer, cx: f32, y: f32, t: f32, focused: bool) {
    let track_w = 220i32;
    let track_h = 8i32;
    let x0 = (cx - track_w as f32 * 0.5) as i32;
    let y0 = y as i32;
    let track = if focused {
        [80, 90, 110, 255]
    } else {
        [55, 62, 75, 255]
    };
    let fill = if focused {
        [100, 180, 220, 255]
    } else {
        [70, 130, 160, 255]
    };
    let knob = if focused {
        [255, 220, 120, 255]
    } else {
        [200, 210, 220, 255]
    };
    fb.fill_rect(x0, y0, track_w, track_h, track);
    let filled = ((t.clamp(0.0, 1.0) * track_w as f32).round() as i32).clamp(0, track_w);
    if filled > 0 {
        fb.fill_rect(x0, y0, filled, track_h, fill);
    }
    let kx = x0 + filled - 3;
    fb.fill_rect(kx.clamp(x0 - 2, x0 + track_w - 6), y0 - 3, 6, track_h + 6, knob);
}

pub fn draw_settings_screen(fb: &mut Framebuffer, page: &SettingsPage) {
    fb.clear([45, 52, 62, 255]);
    let cx = fb.width as f32 * 0.5;
    let mut y = fb.height as f32 * 0.10;
    let title_scale = 2.5;
    let body = 1.55;
    let hint = 1.25;
    let white = [230, 235, 245, 255];
    let dim = [150, 160, 180, 255];
    let accent = [100, 180, 220, 255];
    let focus_c = [255, 220, 120, 255];
    let on_col = [100, 200, 120, 255];
    let off_col = [200, 100, 90, 255];

    draw_pencil_string(fb, "SETTINGS", cx, y, title_scale, accent, true);
    y += 28.0 * title_scale * 0.35 + 12.0;

    let audio_note = if !page.audio_feature {
        "audio: build without cpal"
    } else if page.audio_enabled {
        "audio: ON (cpal)"
    } else {
        "audio: OFF"
    };
    let gfx_note = match page.graphics_mode {
        GraphicsMode::Gpu => "gfx: GPU",
        GraphicsMode::Soft => "gfx: Soft",
    };
    draw_pencil_string(
        fb,
        &format!(
            "window {}x{}  |  {}  |  {}",
            page.window_w, page.window_h, gfx_note, audio_note
        ),
        cx,
        y,
        hint,
        dim,
        true,
    );
    y += 18.0;

    for &row in &SettingsFocus::ALL {
        let focused = page.focus == row;
        let mark = if focused { ">" } else { " " };
        let line = match row {
            SettingsFocus::SoundVolume => {
                format!("{mark} SFX volume     {:.0}%", page.sound_volume * 100.0)
            }
            SettingsFocus::MusicVolume => {
                format!("{mark} Music volume   {:.0}%", page.music_volume * 100.0)
            }
            SettingsFocus::Zoom => {
                format!(
                    "{mark} Zoom           {:.0} px/tile  (+/-)",
                    page.zoom
                )
            }
            SettingsFocus::Graphics => {
                format!(
                    "{mark} Graphics       {}  (restart)",
                    page.graphics_mode.label()
                )
            }
            SettingsFocus::Audio => {
                let state = if !page.audio_feature {
                    "n/a (no cpal)"
                } else if page.audio_enabled {
                    "ON"
                } else {
                    "OFF"
                };
                format!("{mark} Audio device   {state}")
            }
            SettingsFocus::Fullscreen => {
                format!(
                    "{mark} Fullscreen     {}",
                    if page.fullscreen { "ON" } else { "windowed" }
                )
            }
            SettingsFocus::SoundMute => {
                format!(
                    "{mark} SFX mute       {}",
                    if page.sound_muted { "MUTED" } else { "ON" }
                )
            }
            SettingsFocus::MusicMute => {
                format!(
                    "{mark} Music mute     {}",
                    if page.music_muted { "MUTED" } else { "ON" }
                )
            }
            SettingsFocus::ShowFps => {
                format!(
                    "{mark} Show FPS       {}",
                    if page.show_fps { "ON" } else { "off" }
                )
            }
            SettingsFocus::Debug => {
                format!(
                    "{mark} Debug tools    {}",
                    if page.debug { "ON" } else { "off" }
                )
            }
            SettingsFocus::Credentials => {
                let email = if page.email.trim().is_empty() {
                    "(not set)"
                } else {
                    page.email.as_str()
                };
                format!("{mark} Account  {email} @ {}:{}", page.host, page.port)
            }
            SettingsFocus::Back => format!("{mark} Back (Esc)"),
        };
        let draw_col = if focused {
            focus_c
        } else {
            match row {
                SettingsFocus::Audio => {
                    if page.audio_feature && page.audio_enabled {
                        on_col
                    } else {
                        off_col
                    }
                }
                SettingsFocus::Graphics => {
                    if page.graphics_mode == GraphicsMode::Gpu {
                        on_col
                    } else {
                        white
                    }
                }
                SettingsFocus::SoundMute => {
                    if page.sound_muted {
                        off_col
                    } else {
                        on_col
                    }
                }
                SettingsFocus::MusicMute => {
                    if page.music_muted {
                        off_col
                    } else {
                        on_col
                    }
                }
                _ => white,
            }
        };
        draw_pencil_string(fb, &line, cx, y, body, draw_col, true);
        y += 12.0 * body;

        // Horizontal slider bar under volume / zoom rows.
        let slider_t = match row {
            SettingsFocus::SoundVolume => Some(page.sound_volume.clamp(0.0, 1.0)),
            SettingsFocus::MusicVolume => Some(page.music_volume.clamp(0.0, 1.0)),
            SettingsFocus::Zoom => Some(page.zoom_slider_t()),
            _ => None,
        };
        if let Some(t) = slider_t {
            draw_settings_slider(fb, cx, y, t, focused);
            y += 14.0;
        } else {
            y += 4.0;
        }
    }

    y += 8.0;
    if !page.status.is_empty() {
        draw_pencil_string(fb, &page.status, cx, y, hint, dim, true);
        y += 14.0 * hint;
    }
    draw_pencil_string(
        fb,
        "Esc Back  Tab row  Left/Right adjust  Enter toggle",
        cx,
        y,
        hint * 0.95,
        dim,
        true,
    );
}

pub fn settings_key_command(screen: ClientScreen, key: SettingsKey) -> SettingsAction {
    if screen != ClientScreen::Settings {
        return SettingsAction::None;
    }
    match key {
        SettingsKey::Escape | SettingsKey::Back => SettingsAction::Back,
        SettingsKey::ToggleAudio
        | SettingsKey::ToggleMusic
        | SettingsKey::Left
        | SettingsKey::Right
        | SettingsKey::Plus
        | SettingsKey::Minus
        | SettingsKey::Enter => SettingsAction::Applied,
        _ => SettingsAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_map_host_port_volumes_mutes() {
        let s = SettingsPage::from_env_map(|k| match k {
            "OHOL_HOST" => Some("10.0.0.5".into()),
            "OHOL_PORT" => Some("9005".into()),
            "OHOL_EMAIL" => Some("p@q".into()),
            "OHOL_SFX_VOLUME" => Some("0.4".into()),
            "OHOL_MUSIC_VOLUME" => Some("0.6".into()),
            "OHOL_AUDIO_MUTE" => Some("1".into()),
            "OHOL_MUSIC_MUTE" => Some("true".into()),
            "OHOL_SHOW_FPS" => Some("0".into()),
            _ => None,
        });
        assert_eq!(s.host, "10.0.0.5");
        assert_eq!(s.port, 9005);
        assert_eq!(s.email, "p@q");
        assert!((s.sound_volume - 0.4).abs() < 0.001);
        assert!((s.music_volume - 0.6).abs() < 0.001);
        assert!(s.sound_muted && s.music_muted);
        assert!(!s.show_fps);
    }

    #[test]
    fn audio_disable_env_turns_audio_device_off() {
        let s = SettingsPage::from_env_map(|k| match k {
            "OHOL_AUDIO_DISABLE" => Some("1".into()),
            _ => None,
        });
        assert!(!s.audio_enabled);
        assert!(!s.sound_muted); // master off is not the same as SFX mute
    }

    #[test]
    fn audio_enabled_default_on_and_ini() {
        assert!(SettingsPage::default().audio_enabled);
        let mut s = SettingsPage::default();
        s.audio_enabled = false;
        let p = SettingsPage::parse_ini(&s.serialize_ini());
        assert!(!p.audio_enabled);
        s.focus = SettingsFocus::Audio;
        assert_eq!(s.on_key(SettingsKey::Enter), SettingsAction::Applied);
        assert!(s.audio_enabled);
    }

    #[test]
    fn ini_roundtrip() {
        let s = SettingsPage {
            sound_volume: 0.5,
            music_volume: 0.25,
            sound_muted: true,
            music_muted: false,
            show_fps: false,
            zoom: 48.0,
            ..SettingsPage::default()
        };
        let p = SettingsPage::parse_ini(&s.serialize_ini());
        assert!((p.sound_volume - 0.5).abs() < 0.001);
        assert!((p.music_volume - 0.25).abs() < 0.001);
        assert!(p.sound_muted && !p.music_muted && !p.show_fps);
        assert!((p.zoom - 48.0).abs() < 0.001);
    }

    #[test]
    fn zoom_nudge_and_clamp() {
        let mut s = SettingsPage::default();
        s.focus = SettingsFocus::Zoom;
        s.zoom = 32.0;
        assert_eq!(s.on_key(SettingsKey::Plus), SettingsAction::Applied);
        assert!(s.zoom > 32.0);
        s.zoom = ZOOM_MAX;
        assert_eq!(s.on_key(SettingsKey::Plus), SettingsAction::Applied);
        assert!((s.zoom - ZOOM_MAX).abs() < 1e-3);
        s.zoom = ZOOM_MIN;
        assert_eq!(s.on_key(SettingsKey::Minus), SettingsAction::Applied);
        assert!((s.zoom - ZOOM_MIN).abs() < 1e-3);
    }

    #[test]
    fn graphics_mode_default_gpu_and_ini() {
        assert_eq!(SettingsPage::default().graphics_mode, GraphicsMode::Gpu);
        let mut s = SettingsPage::default();
        s.graphics_mode = GraphicsMode::Soft;
        let p = SettingsPage::parse_ini(&s.serialize_ini());
        assert_eq!(p.graphics_mode, GraphicsMode::Soft);
        s.focus = SettingsFocus::Graphics;
        assert_eq!(s.on_key(SettingsKey::Enter), SettingsAction::Applied);
        assert_eq!(s.graphics_mode, GraphicsMode::Gpu);
    }

    #[test]
    fn toggle_audio_and_apply_to_sound_bank() {
        let mut s = SettingsPage::default();
        assert_eq!(s.on_key(SettingsKey::ToggleAudio), SettingsAction::Applied);
        assert!(s.sound_muted);
        let mut bank = SoundBank::new(".");
        s.apply_audio(&mut bank);
        assert!(bank.muted);
        bank.clear_last_played();
        assert!(!bank.play_usage("1:0.5"));
        assert!(bank.last_played.is_empty());
        crate::sound_bank::set_sfx_muted(false);
        crate::sound_bank::set_music_muted(false);
    }

    #[test]
    fn toggle_music_and_apply_to_music_bank() {
        let mut s = SettingsPage::default();
        s.focus = SettingsFocus::MusicMute;
        assert_eq!(s.on_key(SettingsKey::Enter), SettingsAction::Applied);
        let mut music = MusicBank::new(".");
        music.ensure_pcm(1, 22050, vec![0i16; 64]);
        s.apply_music(&mut music);
        assert!(music.muted);
        assert!(!music.play_block(1));
        s.music_muted = false;
        s.music_volume = 0.7;
        s.apply_music(&mut music);
        assert!(music.play_block(1));
        crate::sound_bank::set_sfx_muted(false);
        crate::sound_bank::set_music_muted(false);
    }

    #[test]
    fn volume_nudge_and_back() {
        let mut s = SettingsPage::default();
        s.focus = SettingsFocus::SoundVolume;
        s.sound_volume = 0.5;
        assert_eq!(s.on_key(SettingsKey::Right), SettingsAction::Applied);
        assert!((s.sound_volume - 0.6).abs() < 0.001);
        assert_eq!(s.on_key(SettingsKey::Escape), SettingsAction::Back);
    }

    #[test]
    fn settings_key_command_only_on_settings_screen() {
        assert_eq!(
            settings_key_command(ClientScreen::Settings, SettingsKey::Escape),
            SettingsAction::Back
        );
        assert_eq!(
            settings_key_command(ClientScreen::Playing, SettingsKey::Escape),
            SettingsAction::None
        );
    }

    #[test]
    fn draw_settings_paints_pixels() {
        let mut fb = Framebuffer::new(320, 200);
        draw_settings_screen(&mut fb, &SettingsPage::default());
        assert!(fb.count_non_color([45, 52, 62, 255]) > 40);
    }

    #[test]
    fn sync_endpoint_from_account() {
        let mut s = SettingsPage::default();
        s.sync_endpoint_from("h.example", 9, "a@b.c");
        assert_eq!(s.host, "h.example");
        assert_eq!(s.port, 9);
        assert_eq!(s.email, "a@b.c");
    }

    #[test]
    fn debug_ini_and_toggle() {
        let s = SettingsPage {
            debug: true,
            ..SettingsPage::default()
        };
        let text = s.serialize_ini();
        assert!(text.contains("debug=1"), "serialize must include debug: {text}");
        let p = SettingsPage::parse_ini(&text);
        assert!(p.debug);
        let mut s = SettingsPage::default();
        assert!(!s.debug);
        s.focus = SettingsFocus::Debug;
        assert_eq!(s.on_key(SettingsKey::Enter), SettingsAction::Applied);
        assert!(s.debug);
        assert_eq!(s.on_key(SettingsKey::Enter), SettingsAction::Applied);
        assert!(!s.debug);
    }
}
