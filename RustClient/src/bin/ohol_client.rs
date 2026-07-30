//! Graphical Open Life / OHOL client (protocol + software renderer).
//!
//! ```text
//! cargo run --release --bin ohol-client   # default features: gpu + audio
//! ```
//!
//! Boot: **Account** soft-FB form (P5#37) prefilled from `.env` / `OHOL_*`,
//! then **Loading** progress (P5#36 / C++ LoadingPage) across prefer_cache banks,
//! then live world. Enter=Connect, Esc=skip when creds present, Tab=field, F2=key/password,
//! **Esc/F3=Settings** (P5#39: Graphics GPU/Soft, Audio on/off, volumes, FPS).
//! Headless `ohol-headless` CLI flags are unchanged (`OHOL_LOAD_PROGRESS=1` logs stages).
//!
//! In-world controls:
//! - Arrows / WASD pan, +/- zoom, Esc quit
//! - LMB → `walk_or_use_tile_hold` (first press USE/clothing; hold repath + blocked-tile slide)
//! - RMB or Q → DROP held / REMV from container under cursor
//! - Keys **1–6** → clothing slots 0..5 (held→`DROP c`; bare→`SELF c` remove; Shift→`SREMV`)
//! - Click/hover worn clothing sprites (soft-FB hitMap) → same as keys for that slot
//! - T → `SAY 0 0 HI#` (L-SAY smoke)
//! - Hover uses soft-FB hitMap (`get_sprite_hit`) for object id + worn clothing
//! - Title bar shows rolling FPS; logs FPS after first presented frame, then every 30s
//! - **Death** (P5#38): on our delete PU → death page; **R/Enter** rebirth reconnect, **Esc** quit
//! - **Settings** (P5#39): **Esc or F3** from Account or Playing; Tab/arrows, mouse, Esc/Back
//! - **Debug tools** (settings.debug): **F9** or bottom-right **SNAP** → play snapshot under `logs/snapshots/`
//!
//! Offline demo if server unavailable.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use minifb::{
    InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions,
};

use ohol_headless::account_page::{
    AccountAction, AccountKey, ClientAppState, ClientScreen,
};
// AccountKey used for nested Settings → Account form typing.
use ohol_headless::settings_page::GraphicsMode;
use ohol_headless::anim_bank::AnimBank;
use ohol_headless::client_map::ClientMap;
use ohol_headless::click_tile::{
    click_drop_clothing, click_remove_clothing, click_sremv_clothing, walk_or_use_tile_hold,
};
use ohol_headless::client_screen::{
    death_key_command, draw_death_screen, note_our_death_if_any, rebirth_session_config, DeathKey,
    ScreenCommand,
};
use ohol_headless::settings_page::{restart_client_process, SettingsAction, SettingsKey};
use ohol_headless::play_snapshot::{
    draw_snapshot_button, snapshot_button_hit, write_play_snapshot, SnapshotViewExtras,
};
use ohol_headless::content::ClientContent;
use ohol_headless::hover_pick::{
    draw_hover_outline, update_scene_hover, update_scene_hover_with_clothing, HoverPick,
    WornClothingPickTarget,
};
use ohol_headless::hud::HudSprites;
use ohol_headless::live_object::{LiveWorld, CLOTHING_SLOT_NAMES};
use ohol_headless::load_bench::resolve_content_root;
use ohol_headless::load_progress::{
    boot_load_prefer_cache, draw_loading_progress, LoadStage, LoadingState,
};
use ohol_headless::music_bank::MusicBank;
use ohol_headless::parse::{FoodChange, HeatChange, LoginOutcome, MapChunkHeader, parse_pu_line};
use ohol_headless::render::{Camera, Framebuffer, SceneRenderer};
use ohol_headless::rmb_action::{click_rmb_tile_ex, our_held_id};
use ohol_headless::session::{ClientSession, SessionConfig};
use ohol_headless::sprite_bank::SpriteBank;

const FB_W: usize = 960;
const FB_H: usize = 540;

/// Cargo package version (bump in Cargo.toml when shipping).
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Build-time stamp from build.rs (seconds since epoch) — proves newest binary.
const CLIENT_BUILD_STAMP: &str = env!("OHOL_BUILD_STAMP");
/// What we are actively fixing / working on (shown in every window title).
const CLIENT_FOCUS: &str = "offline age + skin picker";

/// Prefix for all window titles so you can see version + current work.
fn title_prefix() -> String {
    format!("Open Life v{CLIENT_VERSION} b{CLIENT_BUILD_STAMP} [{CLIENT_FOCUS}]")
}

fn window_title(screen: &str, extra: &str) -> String {
    if extra.is_empty() {
        format!("{} — {}", title_prefix(), screen)
    } else {
        format!("{} — {} | {}", title_prefix(), screen, extra)
    }
}

/// Rising-edge detector for Esc / F3 (more reliable than minifb `is_key_pressed`
/// alone when the frame loop skips a pump).
///
/// After Settings is **opened** with Esc/F3, call [`EscF3Edge::mark_opened`] so the
/// same physical hold cannot also **close** Settings (classic open+instant-close bug).
#[derive(Debug, Default, Clone, Copy)]
struct EscF3Edge {
    was_esc: bool,
    was_f3: bool,
    /// Ignore edges until Esc and F3 are both released (set after open).
    wait_release: bool,
}

impl EscF3Edge {
    /// `true` when Esc or F3 just went down this frame (and not blocked by wait_release).
    fn edge(&mut self, esc_down: bool, f3_down: bool) -> bool {
        if self.wait_release {
            if !esc_down && !f3_down {
                self.wait_release = false;
            }
            self.was_esc = esc_down;
            self.was_f3 = f3_down;
            return false;
        }
        let hit = (esc_down && !self.was_esc) || (f3_down && !self.was_f3);
        self.was_esc = esc_down;
        self.was_f3 = f3_down;
        hit
    }

    /// Call immediately after opening Settings with Esc/F3 (or any open that must not
    /// re-trigger close from a still-held Esc).
    fn mark_opened(&mut self) {
        self.wait_release = true;
    }
}

/// Windows physical key state (works when minifb misses WM_KEYDOWN for Esc/F3).
#[cfg(windows)]
mod win_keys {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
    }
    pub const VK_ESCAPE: i32 = 0x1B;
    pub const VK_F3: i32 = 0x72;
    #[inline]
    pub fn down(vk: i32) -> bool {
        unsafe { (GetAsyncKeyState(vk) as u16) & 0x8000 != 0 }
    }
}

/// True if Esc is currently held (minifb **or** Win32 GetAsyncKeyState fallback).
fn esc_held(window: &Window) -> bool {
    let via_mf = window.is_key_down(Key::Escape)
        || window.is_key_pressed(Key::Escape, KeyRepeat::No)
        || window.get_keys().contains(&Key::Escape)
        || window.get_keys_pressed(KeyRepeat::No).contains(&Key::Escape);
    #[cfg(windows)]
    {
        // Async state does not need &mut Window (minifb is_active does).
        return via_mf || win_keys::down(win_keys::VK_ESCAPE);
    }
    #[cfg(not(windows))]
    {
        via_mf
    }
}

/// True if F3 is currently held (minifb **or** Win32 GetAsyncKeyState fallback).
fn f3_held(window: &Window) -> bool {
    let via_mf = window.is_key_down(Key::F3)
        || window.is_key_pressed(Key::F3, KeyRepeat::No)
        || window.get_keys().contains(&Key::F3)
        || window.get_keys_pressed(KeyRepeat::No).contains(&Key::F3);
    #[cfg(windows)]
    {
        return via_mf || win_keys::down(win_keys::VK_F3);
    }
    #[cfg(not(windows))]
    {
        via_mf
    }
}

/// Windows scancodes when winit `virtual_keycode` is None (GPU path only).
#[cfg(feature = "gpu")]
fn virtual_key_from_scancode(scancode: u32) -> Option<winit::event::VirtualKeyCode> {
    use winit::event::VirtualKeyCode;
    // Hardware scan codes (Set 1) used by winit on Windows.
    match scancode {
        0x01 => Some(VirtualKeyCode::Escape),
        0x3D => Some(VirtualKeyCode::F3), // F3
        0x0F => Some(VirtualKeyCode::Tab),
        0x1C => Some(VirtualKeyCode::Return),
        _ => None,
    }
}

/// Soft minifb: **1×** buffer size (960×540) — loading/account/windowed play.
/// (X2 made UI look “zoomed” and the play window huge.)
fn soft_window_opts() -> WindowOptions {
    WindowOptions {
        resize: true,
        scale: Scale::X1,
        scale_mode: ScaleMode::AspectRatioStretch,
        ..WindowOptions::default()
    }
}

/// Soft play window: Stretch fills the client area with the soft-FB.
///
/// Fullscreen: FitScreen grows the window to the monitor; Stretch (not
/// AspectRatioStretch) ensures no letterbox bars inside that window.
fn soft_play_window_opts(fullscreen: bool) -> WindowOptions {
    WindowOptions {
        resize: true,
        scale: if fullscreen {
            Scale::FitScreen
        } else {
            Scale::X1
        },
        scale_mode: ScaleMode::Stretch,
        borderless: fullscreen,
        topmost: false,
        ..WindowOptions::default()
    }
}

/// Safe mouse position in **soft-FB coordinates** (FB_W×FB_H).
///
/// minifb `get_mouse_pos` only undoes Scale, not Stretch resize (upstream TODO).
/// We take unscaled window pixels and map with [`ohol_headless::map_window_to_fb`]
/// — same Stretch formula as the GPU present path.
///
/// Also avoids minifb Clamp panic when window size is 0 (minimize / transient).
fn safe_mouse_pos(window: &Window) -> Option<(f32, f32)> {
    let (w, h) = window.get_size();
    if w == 0 || h == 0 {
        return None;
    }
    // Unscaled = window client pixels (scale_factor ignored). Stretch fills the window.
    let (wx, wy) = window.get_unscaled_mouse_pos(MouseMode::Pass)?;
    ohol_headless::map_window_to_fb(wx, wy, w as u32, h as u32, FB_W as u32, FB_H as u32)
}

/// Rolling FPS for title bar + stderr log (first frame, then every 30s).
struct FpsMeter {
    /// Frames since last sample window start.
    frames: u32,
    /// Wall time of sample window start.
    window_start: Instant,
    /// Instant of last FPS log line (None until first real present).
    last_log: Option<Instant>,
    /// Last computed FPS for title.
    last_fps: f32,
    /// True after first successful buffer present.
    presented_once: bool,
    label: &'static str,
}

impl FpsMeter {
    fn new(label: &'static str) -> Self {
        Self {
            frames: 0,
            window_start: Instant::now(),
            last_log: None,
            last_fps: 0.0,
            presented_once: false,
            label,
        }
    }

    /// Call once after a successful `update_with_buffer` (real display).
    /// `frame_dt` is the wall time for this frame (seconds), used for instant estimate.
    fn on_presented(&mut self, frame_dt: f32) {
        self.frames = self.frames.saturating_add(1);
        let now = Instant::now();
        let sample_secs = self.window_start.elapsed().as_secs_f32();
        // Refresh rolling FPS about 4×/sec so the title is responsive.
        if sample_secs >= 0.25 && self.frames > 0 {
            self.last_fps = self.frames as f32 / sample_secs;
            self.frames = 0;
            self.window_start = now;
        } else if self.last_fps <= 0.0 && frame_dt > 0.0 {
            self.last_fps = 1.0 / frame_dt.max(0.000_1);
        }

        if !self.presented_once {
            self.presented_once = true;
            self.last_log = Some(now);
            let instant = if frame_dt > 0.0 {
                1.0 / frame_dt.max(0.000_1)
            } else {
                self.last_fps
            };
            eprintln!(
                "fps: first present ({}) — {:.1} FPS instant (target 60, {}x{} soft-FB)",
                self.label, instant, FB_W, FB_H
            );
            return;
        }

        if let Some(t0) = self.last_log {
            if now.duration_since(t0) >= Duration::from_secs(30) {
                self.last_log = Some(now);
                eprintln!(
                    "fps: {} — {:.1} FPS (rolling ~0.25s window, target 60)",
                    self.label, self.last_fps
                );
            }
        }
    }

    fn fps(&self) -> f32 {
        self.last_fps
    }
}

/// Unicode char queue for account-page typing (minifb `InputCallback`).
struct CharQueue {
    chars: Rc<RefCell<Vec<u32>>>,
}

impl InputCallback for CharQueue {
    fn add_char(&mut self, uni_char: u32) {
        self.chars.borrow_mut().push(uni_char);
    }
}

fn main() -> anyhow::Result<()> {
    let t_start = Instant::now();
    let _ = dotenvy::dotenv();
    eprintln!(
        "client: {}  exe={}",
        title_prefix(),
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into())
    );
    // P5#37: Account soft-FB form → SessionConfig → connect (headless CLI unchanged).
    let mut app = ClientAppState::from_env();
    // Graphic timeouts for live play (same as prior hard-coded client defaults).
    app.account.read_timeout = Duration::from_millis(30);
    app.account.write_timeout = Duration::from_secs(5);
    eprintln!(
        "graphics: mode={} (F3 Settings → Graphics; restart to switch)",
        app.settings.graphics_mode.label()
    );

    let t_account0 = Instant::now();
    let cfg = match run_account_boot(&mut app)? {
        Some(cfg) => cfg,
        None => {
            eprintln!("account: quit");
            return Ok(());
        }
    };
    let account_secs = t_account0.elapsed().as_secs_f64();

    eprintln!(
        "account: connect email={} key={} host={}:{}",
        ohol_headless::redact_email(&cfg.email),
        if cfg.account_key.is_empty() {
            "none"
        } else {
            "set"
        },
        cfg.host,
        cfg.port
    );

    // Single init window: load content banks + try server connect + status.
    // Then one main window (live or offline). No second loading flash.
    let t_boot0 = Instant::now();
    let outcome = run_init_boot(&mut app, &cfg)?;
    let boot_secs = t_boot0.elapsed().as_secs_f64();

    match outcome {
        InitOutcome::Live {
            mut session,
            sprites,
            anims,
            loading_secs,
            connect_secs,
        } => {
            eprintln!(
                "connected objects={} map pending",
                session.content.objects.len()
            );
            eprintln!(
                "controls: LMB walk/use/self | RMB/Q drop/remv | 1-6 clothing | WASD pan | +/- or mouse-wheel zoom | Esc/F3 settings"
            );
            eprintln!("death: R/Enter rebirth · Esc quit (after our player dies)");
            app.settings.apply_runtime_globals();
            session.sounds.set_loudness(app.settings.sound_volume);
            session.sounds.set_muted(app.settings.sound_muted);
            let _ = session.set_read_timeout(Some(Duration::from_millis(1)));
            app.enter_playing();
            log_startup_timings(
                account_secs,
                loading_secs,
                connect_secs,
                t_start.elapsed().as_secs_f64(),
                true,
            );
            match app.settings.graphics_mode {
                GraphicsMode::Gpu => {
                    eprintln!(
                        "graphics: GPU present (pixels/wgpu) {}x{}  fullscreen={}",
                        FB_W,
                        FB_H,
                        app.settings.fullscreen
                    );
                    run_session_gpu(session, sprites, anims, cfg, app)
                }
                GraphicsMode::Soft => {
                    eprintln!("graphics: Soft minifb present (CPU buffer)");
                    run_session_from_boot(session, sprites, anims, cfg, app)
                }
            }
        }
        InitOutcome::Offline {
            content,
            sprites,
            anims,
            ground,
            sounds,
            loading_secs,
            connect_secs,
            status,
        } => {
            eprintln!("{status}");
            log_startup_timings(
                account_secs,
                loading_secs,
                connect_secs,
                t_start.elapsed().as_secs_f64(),
                false,
            );
            eprintln!(
                "timing: boot_window={boot_secs:.3}s (single init screen)"
            );
            run_offline_with_banks(content, sprites, anims, ground, sounds)
        }
    }
}

/// Result of the single init window (load + connect).
enum InitOutcome {
    Live {
        session: ClientSession,
        sprites: SpriteBank,
        anims: AnimBank,
        loading_secs: f64,
        connect_secs: f64,
    },
    Offline {
        content: ClientContent,
        sprites: SpriteBank,
        anims: AnimBank,
        ground: ohol_headless::ground_sprites::GroundBank,
        sounds: ohol_headless::sound_bank::SoundBank,
        loading_secs: f64,
        connect_secs: f64,
        status: String,
    },
}

/// Log account / loading / connect / total-to-play timings (stderr + logs file).
fn log_startup_timings(
    account_secs: f64,
    loading_secs: f64,
    connect_secs: f64,
    total_secs: f64,
    live: bool,
) {
    let mode = if live { "live" } else { "offline" };
    let line = format!(
        "timing: mode={mode} account={account_secs:.3}s loading={loading_secs:.3}s connect={connect_secs:.3}s total_to_play={total_secs:.3}s"
    );
    eprintln!("{line}");
    let path = std::path::Path::new("logs").join("client_startup_timings.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let row = format!("{stamp} {line}\n");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = f.write_all(row.as_bytes());
    }
}

/// Present a status line on the shared init loading screen (same window as bank load).
fn present_init_status(
    window: &mut Window,
    fb: &mut Framebuffer,
    buf: &mut [u32],
    title: &str,
    detail: &str,
    fraction: f32,
) {
    let mut state = if fraction >= 0.99 {
        LoadingState::finished()
    } else {
        LoadingState::for_stage(LoadStage::Content, fraction.clamp(0.0, 1.0), Some(detail))
    };
    state.label = detail.into();
    if fraction >= 0.99 {
        state.overall_fraction = 1.0;
        state.done = true;
    }
    draw_loading_progress(fb, &state);
    // Extra status under bar (connect result).
    ohol_headless::ui_font::draw_ui_text(
        fb,
        title,
        FB_W as f32 * 0.5,
        FB_H as f32 * 0.78,
        16.0,
        [200, 210, 230, 255],
        true,
    );
    rgba_to_u32(&fb.pixels, buf);
    window.set_title(&window_title("Starting", detail));
    let _ = window.update_with_buffer(buf, FB_W, FB_H);
}

/// One init window: load banks + try server connect + show connected/offline.
/// Main play/offline is a separate window after this returns.
fn run_init_boot(app: &mut ClientAppState, cfg: &SessionConfig) -> anyhow::Result<InitOutcome> {
    app.screen = ClientScreen::Loading;
    app.loading_msg = "starting…".into();

    let root = resolve_content_root(None).map_err(anyhow::Error::msg)?;
    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        &window_title("Starting", "loading…"),
        FB_W,
        FB_H,
        soft_window_opts(),
    )?;
    window.set_target_fps(60);
    let mut buf = vec![0u32; FB_W * FB_H];

    present_init_status(
        &mut window,
        &mut fb,
        &mut buf,
        "Open Life",
        "Loading content…",
        0.02,
    );

    let t_load0 = Instant::now();
    let mut present = |state: &LoadingState| {
        app.loading_msg = state.label.clone();
        draw_loading_progress(&mut fb, state);
        rgba_to_u32(&fb.pixels, &mut buf);
        let pct = (state.overall_fraction * 100.0).round() as i32;
        let detail = if state.label.is_empty() {
            state.stage.name().to_string()
        } else {
            let d = state.label.as_str();
            if d.len() > 48 {
                format!("{}…", &d[..45])
            } else {
                d.to_string()
            }
        };
        window.set_title(&window_title("Starting", &format!("{pct}% {detail}")));
        let _ = window.update_with_buffer(&buf, FB_W, FB_H);
    };

    let banks = {
        let mut cb = |s: &LoadingState| present(s);
        match boot_load_prefer_cache(&root, Some(&mut cb)) {
            Ok(b) => b,
            Err(e) => {
                present_init_status(
                    &mut window,
                    &mut fb,
                    &mut buf,
                    "Load failed",
                    &e,
                    0.0,
                );
                std::thread::sleep(Duration::from_millis(800));
                return Err(anyhow::Error::msg(e));
            }
        }
    };
    let loading_secs = t_load0.elapsed().as_secs_f64();
    eprintln!(
        "loading: done objects={} transitions={} binary_cache={}",
        banks.content.objects.len(),
        banks.content.transitions.len(),
        banks.used_binary_cache
    );

    // Same window: connecting…
    let host_line = format!("Connecting to {}:{}…", cfg.host, cfg.port);
    present_init_status(
        &mut window,
        &mut fb,
        &mut buf,
        "Connecting",
        &host_line,
        0.92,
    );
    eprintln!("connect: try {}:{} …", cfg.host, cfg.port);

    let t_connect0 = Instant::now();
    let content = banks.content;
    let sprites = banks.sprites;
    let anims = banks.anims;
    let ground = banks.ground;
    let sounds = banks.sounds;
    let _music = banks.music;

    match ClientSession::connect_with_content(cfg, content) {
        Ok(mut session) if matches!(session.login, LoginOutcome::Accepted) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            session.sounds = sounds;
            present_init_status(
                &mut window,
                &mut fb,
                &mut buf,
                "Connected",
                &format!("Online · {}:{}", cfg.host, cfg.port),
                1.0,
            );
            std::thread::sleep(Duration::from_millis(350));
            // Drop init window before main play window.
            drop(window);
            Ok(InitOutcome::Live {
                session,
                sprites,
                anims,
                loading_secs,
                connect_secs,
            })
        }
        Ok(session) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            let status = format!("Offline · login {:?}", session.login);
            present_init_status(
                &mut window,
                &mut fb,
                &mut buf,
                "Offline",
                &status,
                1.0,
            );
            std::thread::sleep(Duration::from_millis(500));
            drop(window);
            Ok(InitOutcome::Offline {
                content: session.content,
                sprites,
                anims,
                ground,
                sounds,
                loading_secs,
                connect_secs,
                status,
            })
        }
        Err(e) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            let status = format!("Offline · connect failed");
            present_init_status(
                &mut window,
                &mut fb,
                &mut buf,
                "Offline",
                &format!("{e}"),
                1.0,
            );
            eprintln!("connect failed: {e}");
            std::thread::sleep(Duration::from_millis(500));
            drop(window);
            // Content was moved into connect attempt — reload content only (banks stay warm).
            // connect_with_content consumes content on all paths; rebuild from root meta.
            let content = ClientContent::load_default_locations().unwrap_or_default();
            Ok(InitOutcome::Offline {
                content,
                sprites,
                anims,
                ground,
                sounds,
                loading_secs,
                connect_secs,
                status,
            })
        }
    }
}

/// Account form loop. Returns `Some(SessionConfig)` on Connect, `None` on Quit.
fn run_account_boot(app: &mut ClientAppState) -> anyhow::Result<Option<SessionConfig>> {
    // Default **on** when unset (launcher / .env default). Set OHOL_AUTO_CONNECT=0 to force form.
    // Selftest always shows Account so Settings UI can be verified.
    let selftest_boot = std::env::var_os("OHOL_SETTINGS_SELFTEST").is_some();
    let auto = !selftest_boot
        && std::env::var("OHOL_AUTO_CONNECT")
            .map(|v| {
                let t = v.trim();
                !(t.is_empty()
                    || t == "0"
                    || t.eq_ignore_ascii_case("false")
                    || t.eq_ignore_ascii_case("no")
                    || t.eq_ignore_ascii_case("off"))
            })
            .unwrap_or(true);
    if auto {
        let cfg = app.account.build_session_config();
        if !cfg.email.is_empty()
            && (!cfg.account_key.is_empty() || !cfg.password.is_empty())
        {
            eprintln!(
                "account: OHOL_AUTO_CONNECT → {}:{} email={}",
                cfg.host,
                cfg.port,
                ohol_headless::redact_email(&cfg.email)
            );
            return Ok(Some(cfg));
        }
        eprintln!("account: OHOL_AUTO_CONNECT on but creds incomplete — showing form");
    } else if selftest_boot {
        eprintln!("account: selftest — showing Account form (auto-connect skipped)");
    } else {
        eprintln!("account: OHOL_AUTO_CONNECT=0 — showing Account form");
    }

    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new("Open Life — Account", FB_W, FB_H, soft_window_opts())?;
    window.set_target_fps(60);

    let chars: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(CharQueue {
        chars: Rc::clone(&chars),
    }));

    let hud = HudSprites::with_default_roots(None);
    let mut buf = vec![0u32; FB_W * FB_H];
    let mut last = Instant::now();
    let mut fps = FpsMeter::new("account");

    eprintln!(
        "account page: mouse select fields | Tab field | Enter Connect | Esc/F3 Settings | F2 key/password | type to edit"
    );

    let mut was_lmb_account = false;
    let mut esc_f3 = EscF3Edge::default();
    // Key actions are sampled **after** update_with_buffer (minifb pumps WM_* then).
    // Pending flags apply on the next loop iteration so Esc/F3 always register.
    let mut pending_open_settings = false;
    let mut pending_close_settings = false;
    // Automated self-test: open Settings after first present, verify, exit.
    let selftest = std::env::var_os("OHOL_SETTINGS_SELFTEST").is_some();
    let mut selftest_frames: u32 = 0;
    let mut selftest_opened = false;

    while window.is_open() {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();

        // Apply key edges detected after last frame's message pump.
        let mut suppress_settings_close = false;
        if pending_open_settings {
            pending_open_settings = false;
            if app.screen.is_account() {
                if app.enter_settings() {
                    suppress_settings_close = true;
                    esc_f3.mark_opened();
                    eprintln!("settings: opened from key/mouse (offline-ok)");
                }
            }
        }
        if pending_close_settings {
            pending_close_settings = false;
            if app.screen.is_settings() {
                app.leave_settings();
                was_lmb_account = false;
                eprintln!("settings: closed from Esc");
            }
        }

        // Self-test: force open without needing a real key (verifies draw + state).
        if selftest && !selftest_opened && selftest_frames >= 5 {
            if app.enter_settings() {
                suppress_settings_close = true;
                esc_f3.mark_opened();
                selftest_opened = true;
                eprintln!("selftest: enter_settings OK screen={}", app.screen.as_str());
            } else {
                eprintln!(
                    "selftest: FAIL enter_settings from screen={}",
                    app.screen.as_str()
                );
                return Ok(None);
            }
        }

        // P5#39 Settings overlay from Account (works without server connection).
        if app.screen.is_settings() {
            match handle_settings_input(
                &window,
                app,
                &mut was_lmb_account,
                suppress_settings_close,
                false, // Esc close handled via pending_close after pump (release-gate)
            ) {
                SettingsLoop::Left => {
                    was_lmb_account = false;
                }
                SettingsLoop::Restart => {
                    restart_client_process();
                }
                SettingsLoop::OpenAccount => {
                    app.enter_account_from_settings();
                    was_lmb_account = false;
                }
                SettingsLoop::Continue => {
                    app.settings.draw(&mut fb, Some(&hud));
                    rgba_to_u32(&fb.pixels, &mut buf);
                    let title = window_title(
                        "Settings",
                        &format!(
                            "{:.0} FPS Esc=Back SFX {:.0}%",
                            fps.fps(),
                            app.settings.sound_volume * 100.0,
                        ),
                    );
                    window.set_title(&title);
                    window.update_with_buffer(&buf, FB_W, FB_H)?;
                    fps.on_presented(dt);
                    // After pump: Esc edge → close next frame (blocked until key release).
                    let esc = esc_held(&window);
                    let f3 = f3_held(&window);
                    let edge = esc_f3.edge(esc, f3);
                    if edge {
                        eprintln!(
                            "input: {} (settings screen)",
                            if f3 { "F3" } else { "ESC" }
                        );
                        if !suppress_settings_close {
                            pending_close_settings = true;
                        }
                    }
                    if selftest && selftest_opened {
                        let dark = fb
                            .pixels
                            .chunks_exact(4)
                            .filter(|c| c[0] < 80 && c[1] < 90 && c[2] < 100)
                            .count();
                        eprintln!(
                            "selftest: settings frame title='{title}' dark_px={dark}"
                        );
                        if dark > 10_000 {
                            eprintln!("selftest: PASS settings visible");
                            return Ok(None);
                        }
                        selftest_frames = selftest_frames.saturating_add(1);
                        if selftest_frames > 30 {
                            eprintln!("selftest: FAIL settings not painted (dark_px={dark})");
                            return Ok(None);
                        }
                    }
                    continue;
                }
            }
            if app.screen.is_settings() {
                continue;
            }
        }

        if selftest {
            selftest_frames = selftest_frames.saturating_add(1);
        }

        app.account.step(dt);

        {
            let mut q = chars.borrow_mut();
            for u in q.drain(..) {
                if let Some(c) = char::from_u32(u) {
                    let _ = app.account.on_key(AccountKey::Char(c));
                }
            }
        }

        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let mut action = AccountAction::None;
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
            action = app.account.on_key(AccountKey::Tab { shift });
        }
        if window.is_key_pressed(Key::Enter, KeyRepeat::No)
            || window.is_key_pressed(Key::NumPadEnter, KeyRepeat::No)
        {
            action = app.account.on_key(AccountKey::Enter);
        }
        if window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
            let _ = app.account.on_key(AccountKey::Backspace);
        }
        if window.is_key_pressed(Key::Delete, KeyRepeat::Yes) {
            let _ = app.account.on_key(AccountKey::Delete);
        }
        if window.is_key_pressed(Key::Left, KeyRepeat::Yes) {
            let _ = app.account.on_key(AccountKey::Left);
        }
        if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
            let _ = app.account.on_key(AccountKey::Right);
        }
        if window.is_key_pressed(Key::Home, KeyRepeat::No) {
            let _ = app.account.on_key(AccountKey::Home);
        }
        if window.is_key_pressed(Key::End, KeyRepeat::No) {
            let _ = app.account.on_key(AccountKey::End);
        }
        if window.is_key_pressed(Key::F2, KeyRepeat::No) {
            let _ = app.account.on_key(AccountKey::ToggleSecretMode);
        }

        let lmb = window.get_mouse_down(MouseButton::Left);
        if lmb && !was_lmb_account {
            if let Some((mx, my)) = safe_mouse_pos(&window) {
                let a = app.account.on_pointer_down(
                    mx,
                    my,
                    FB_W as f32,
                    FB_H as f32,
                    Some(&hud),
                );
                if a != AccountAction::None {
                    action = a;
                }
            }
        }
        was_lmb_account = lmb;

        match action {
            AccountAction::Quit => return Ok(None),
            AccountAction::Connect => {
                // Single loading UI is run_init_boot (next step). No flash screen here.
                let cfg = app.begin_connect();
                eprintln!("account: connect → single init window");
                return Ok(Some(cfg));
            }
            AccountAction::OpenSettings => {
                if app.enter_settings() {
                    suppress_settings_close = true;
                    esc_f3.mark_opened();
                    eprintln!("settings: opened from Settings button");
                }
            }
            AccountAction::Back | AccountAction::Saved => {
                // Nested form only — boot path should not hit these.
                app.return_to_settings_from_account();
            }
            AccountAction::None => {}
        }

        if !window.is_open() {
            break;
        }
        if app.screen.is_settings() {
            app.settings.draw(&mut fb, Some(&hud));
            rgba_to_u32(&fb.pixels, &mut buf);
            window.set_title(&window_title("Settings", "Esc=Back"));
            window.update_with_buffer(&buf, FB_W, FB_H)?;
            fps.on_presented(dt);
            // Sample keys after pump for close next frame (release-gate).
            let esc = esc_held(&window);
            let f3 = f3_held(&window);
            if esc_f3.edge(esc, f3) && !suppress_settings_close {
                eprintln!(
                    "input: {} (close settings)",
                    if f3 { "F3" } else { "ESC" }
                );
                pending_close_settings = true;
            }
            continue;
        }

        app.account.draw(&mut fb, Some(&hud));
        rgba_to_u32(&fb.pixels, &mut buf);
        window.set_title(&window_title(
            "Account",
            &format!("{:.0} FPS F3/Esc=Settings", fps.fps()),
        ));
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);

        // AFTER message pump (+ Win32 async fallback): Esc/F3 open Settings next frame.
        let esc = esc_held(&window);
        let f3 = f3_held(&window);
        if esc_f3.edge(esc, f3) {
            pending_open_settings = true;
            eprintln!(
                "input: {} open Settings (active={} screen={})",
                if f3 { "F3" } else { "ESC" },
                window.is_active(),
                app.screen.as_str()
            );
        }
    }
    Ok(None)
}

/// Outcome of one Settings-page input tick.
enum SettingsLoop {
    Continue,
    Left,
    Restart,
    /// Jump to Account form (from Settings → Account settings row).
    OpenAccount,
}

/// Keyboard for nested Account form (no char queue — caller drains that).
fn pump_account_keys(window: &Window, app: &mut ClientAppState) -> AccountAction {
    let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
    let mut action = AccountAction::None;
    if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
        action = app.account.on_key(AccountKey::Tab { shift });
    }
    if window.is_key_pressed(Key::Enter, KeyRepeat::No)
        || window.is_key_pressed(Key::NumPadEnter, KeyRepeat::No)
    {
        action = app.account.on_key(AccountKey::Enter);
    }
    if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
        action = app.account.on_key(AccountKey::Escape);
    }
    if window.is_key_pressed(Key::Backspace, KeyRepeat::Yes) {
        let _ = app.account.on_key(AccountKey::Backspace);
    }
    if window.is_key_pressed(Key::Delete, KeyRepeat::Yes) {
        let _ = app.account.on_key(AccountKey::Delete);
    }
    if window.is_key_pressed(Key::Left, KeyRepeat::Yes) {
        let _ = app.account.on_key(AccountKey::Left);
    }
    if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
        let _ = app.account.on_key(AccountKey::Right);
    }
    if window.is_key_pressed(Key::Home, KeyRepeat::No) {
        let _ = app.account.on_key(AccountKey::Home);
    }
    if window.is_key_pressed(Key::End, KeyRepeat::No) {
        let _ = app.account.on_key(AccountKey::End);
    }
    if window.is_key_pressed(Key::F2, KeyRepeat::No) {
        let _ = app.account.on_key(AccountKey::ToggleSecretMode);
    }
    action
}

/// Process Settings keyboard + mouse; Back leaves Settings (Account or Playing).
///
/// `suppress_close`: true on the same frame Settings was just opened with Esc/F3 so
/// that key is not immediately treated as Back (minifb can still report the press
/// before the next `update_with_buffer` pump).
///
/// `close_edge`: optional rising-edge for Esc (from [`EscF3Edge`]); when `None`,
/// falls back to `is_key_pressed` / `is_key_down` checks.
fn handle_settings_input(
    window: &Window,
    app: &mut ClientAppState,
    was_lmb: &mut bool,
    suppress_close: bool,
    close_edge: bool,
) -> SettingsLoop {
    let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
    let mut key = SettingsKey::Other;
    if window.is_key_pressed(Key::Key1, KeyRepeat::No) {
        key = SettingsKey::ToggleAudio;
    } else if window.is_key_pressed(Key::Key2, KeyRepeat::No) {
        key = SettingsKey::ToggleMusic;
    } else if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
        key = SettingsKey::Tab { shift };
    } else if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
        key = SettingsKey::Up;
    } else if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
        key = SettingsKey::Down;
    } else if window.is_key_pressed(Key::Left, KeyRepeat::Yes) {
        key = SettingsKey::Left;
    } else if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
        key = SettingsKey::Right;
    } else if window.is_key_pressed(Key::Equal, KeyRepeat::Yes)
        || window.is_key_pressed(Key::NumPadPlus, KeyRepeat::Yes)
    {
        key = SettingsKey::Plus;
    } else if window.is_key_pressed(Key::Minus, KeyRepeat::Yes)
        || window.is_key_pressed(Key::NumPadMinus, KeyRepeat::Yes)
    {
        key = SettingsKey::Minus;
    } else if window.is_key_pressed(Key::Enter, KeyRepeat::No)
        || window.is_key_pressed(Key::NumPadEnter, KeyRepeat::No)
        || window.is_key_pressed(Key::Space, KeyRepeat::No)
    {
        key = SettingsKey::Enter;
    } else if !suppress_close
        && (close_edge || window.is_key_pressed(Key::B, KeyRepeat::No))
    {
        // B = Back. Esc is handled only via EscF3Edge + release-gate after pump
        // (never is_key_pressed here — that caused open+instant-close on Esc hold).
        key = SettingsKey::Escape;
    }

    let mut action = app.settings.on_key(key);

    // Mouse: click toggles / restart / back; drag volume & zoom sliders.
    // On the open frame, ignore mouse entirely so a held click cannot instantly Back.
    let lmb = window.get_mouse_down(MouseButton::Left);
    if !suppress_close {
        if let Some((mx, my)) = safe_mouse_pos(window) {
            if lmb && !*was_lmb {
                let a = app
                    .settings
                    .on_pointer_down(mx, my, FB_W as f32, FB_H as f32);
                if a != SettingsAction::None {
                    action = a;
                }
            } else if lmb && *was_lmb && app.settings.slider_drag.is_some() {
                let a = app
                    .settings
                    .on_pointer_drag(mx, FB_W as f32, FB_H as f32);
                if a != SettingsAction::None {
                    action = a;
                }
            }
        }
    }
    if !lmb {
        app.settings.on_pointer_up();
    }
    *was_lmb = lmb;

    // Never leave on the same frame we opened (keyboard or mouse).
    if suppress_close
        && matches!(
            action,
            SettingsAction::Back | SettingsAction::Restart | SettingsAction::OpenAccount
        )
    {
        action = SettingsAction::None;
    }

    match action {
        SettingsAction::None => SettingsLoop::Continue,
        SettingsAction::Applied => {
            app.settings.apply_runtime_globals();
            SettingsLoop::Continue
        }
        SettingsAction::Back => {
            app.leave_settings();
            SettingsLoop::Left
        }
        SettingsAction::Restart => SettingsLoop::Restart,
        SettingsAction::OpenAccount => SettingsLoop::OpenAccount,
    }
}

/// Live session using banks already loaded by [`run_loading_boot`].
///
/// P5#38 Death / P5#39 Settings from Playing (F3 or Esc).
fn run_session_from_boot(
    mut session: ClientSession,
    mut sprites: SpriteBank,
    mut anims: AnimBank,
    cfg: SessionConfig,
    mut app: ClientAppState,
) -> anyhow::Result<()> {
    let root = session.content.root.clone().unwrap_or_else(|| {
        std::path::PathBuf::from(r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7")
    });
    sprites.preload([19, 33, 144]);
    session.content.setup_eyes_and_mouth(|sid| {
        let m = sprites.ensure_meta(sid);
        Some(m.tag.clone())
    });
    let mut scene = SceneRenderer::default();
    scene.set_content_root(Some(&root));
    // Restore persisted zoom (settings.ini / OHOL_ZOOM).
    scene.camera.zoom = app.settings.zoom.clamp(
        ohol_headless::render::ZOOM_MIN,
        ohol_headless::render::ZOOM_MAX,
    );
    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        "Open Life Rust Client",
        FB_W,
        FB_H,
        soft_play_window_opts(app.settings.fullscreen),
    )?;
    window.set_target_fps(60);
    let chars: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(CharQueue {
        chars: Rc::clone(&chars),
    }));
    let mut buf = vec![0u32; FB_W * FB_H];
    let mut last = Instant::now();
    let mut pan = (0.0f32, 0.0f32);
    let mut was_lmb = false;
    let mut was_rmb = false;
    let mut mouse_down_frames: i32 = 0;
    let mut hover = HoverPick::default();
    let mut last_status = String::new();
    let mut fps = FpsMeter::new("live");
    let settings_hud = HudSprites::with_default_roots(Some(&root));
    let mut was_lmb_account = false;

    // Volume/mute on both banks: scene.sounds drives draw/anim SFX; session for net hooks.
    app.settings.apply_to_banks(Some(&mut session.sounds), None);
    app.settings.apply_to_banks(Some(&mut scene.sounds), None);

    let mut was_lmb_settings = false;
    let mut esc_f3 = EscF3Edge::default();
    let mut pending_open_settings = false;
    let mut pending_close_settings = false;

    while window.is_open() {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();

        // Pending open/close from previous frame's post-pump key sample.
        let mut suppress_settings_close = false;
        if pending_open_settings {
            pending_open_settings = false;
            if app.screen.is_playing() {
                if app.enter_settings() {
                    suppress_settings_close = true;
                    esc_f3.mark_opened();
                    eprintln!("settings: opened in play (key)");
                }
            }
        }
        if pending_close_settings {
            pending_close_settings = false;
            if app.screen.is_settings() {
                app.leave_settings();
                was_lmb_settings = false;
                eprintln!("settings: closed in play (key)");
            }
        }

        // ── P5#39 Settings ───────────────────────────────────────────────
        if app.screen.is_settings() {
            match handle_settings_input(
                &window,
                &mut app,
                &mut was_lmb_settings,
                suppress_settings_close,
                false,
            ) {
                SettingsLoop::Left => {
                    // Apply volume/mute to BOTH banks (scene = anim SFX).
                    app.apply_settings_to_banks(Some(&mut session.sounds), None);
                    app.apply_settings_to_banks(Some(&mut scene.sounds), None);
                    // Apply zoom when leaving settings (also persisted by leave_settings).
                    scene.camera.zoom = app.settings.zoom.clamp(
                        ohol_headless::render::ZOOM_MIN,
                        ohol_headless::render::ZOOM_MAX,
                    );
                    was_lmb_settings = false;
                }
                SettingsLoop::Restart => {
                    app.apply_settings_to_banks(Some(&mut session.sounds), None);
                    app.apply_settings_to_banks(Some(&mut scene.sounds), None);
                    let _ = app.settings.save_default();
                    restart_client_process();
                }
                SettingsLoop::OpenAccount => {
                    app.enter_account_from_settings();
                    was_lmb_settings = false;
                    log_status(&mut last_status, "Account settings");
                }
                SettingsLoop::Continue => {
                    // Live-preview zoom + SFX loudness while adjusting.
                    scene.camera.zoom = app.settings.zoom.clamp(
                        ohol_headless::render::ZOOM_MIN,
                        ohol_headless::render::ZOOM_MAX,
                    );
                    app.apply_settings_to_banks(Some(&mut session.sounds), None);
                    app.apply_settings_to_banks(Some(&mut scene.sounds), None);
                    // World under glass settings overlay.
                    let saved_hl = scene.highlight_tile.take();
                    scene.draw(
                        &mut fb,
                        &mut session.map,
                        &mut session.world,
                        &session.content,
                        &mut sprites,
                        &mut anims,
                        dt,
                    );
                    scene.highlight_tile = saved_hl;
                    app.settings.draw_overlay(&mut fb, Some(&settings_hud));
                    let title = window_title(
                        "Settings",
                        &format!(
                            "{:.0} FPS Esc=Back Zoom {:.0} SFX {:.0}%",
                            fps.fps(),
                            app.settings.zoom,
                            app.settings.sound_volume * 100.0,
                        ),
                    );
                    window.set_title(&title);
                    rgba_to_u32(&fb.pixels, &mut buf);
                    window.update_with_buffer(&buf, FB_W, FB_H)?;
                    fps.on_presented(dt);
                    let esc = esc_held(&window);
                    let f3 = f3_held(&window);
                    if esc_f3.edge(esc, f3) {
                        eprintln!(
                            "input: {} (play settings)",
                            if f3 { "F3" } else { "ESC" }
                        );
                        if !suppress_settings_close {
                            pending_close_settings = true;
                        }
                    }
                    continue;
                }
            }
            if app.screen.is_settings() {
                continue;
            }
        }

        // Nested Account form (from Settings → Account settings).
        if app.screen.is_account() {
            app.account.step(dt);
            {
                let mut q = chars.borrow_mut();
                for u in q.drain(..) {
                    if let Some(c) = char::from_u32(u) {
                        let _ = app.account.on_key(AccountKey::Char(c));
                    }
                }
            }
            let mut action = pump_account_keys(&window, &mut app);
            let lmb = window.get_mouse_down(MouseButton::Left);
            if lmb && !was_lmb_account {
                if let Some((mx, my)) = safe_mouse_pos(&window) {
                    let a = app.account.on_pointer_down(
                        mx,
                        my,
                        FB_W as f32,
                        FB_H as f32,
                        Some(&settings_hud),
                    );
                    if a != AccountAction::None {
                        action = a;
                    }
                }
            }
            was_lmb_account = lmb;
            match action {
                AccountAction::Back | AccountAction::Saved => {
                    app.return_to_settings_from_account();
                    was_lmb_account = false;
                    log_status(&mut last_status, "Account saved");
                }
                AccountAction::OpenSettings => {
                    let _ = app.enter_settings();
                }
                AccountAction::Connect => {
                    // Mid-session: save endpoint only (reconnect next boot).
                    app.account.remember_current_server();
                    app.return_to_settings_from_account();
                    log_status(&mut last_status, "Account saved (reconnect next boot)");
                }
                AccountAction::Quit | AccountAction::None => {}
            }
            if app.screen.is_account() {
                let saved_hl = scene.highlight_tile.take();
                scene.draw(
                    &mut fb,
                    &mut session.map,
                    &mut session.world,
                    &session.content,
                    &mut sprites,
                    &mut anims,
                    dt,
                );
                scene.highlight_tile = saved_hl;
                app.account.draw_overlay(&mut fb, Some(&settings_hud));
                window.set_title(&window_title("Account", "Esc=Back to Settings"));
                rgba_to_u32(&fb.pixels, &mut buf);
                window.update_with_buffer(&buf, FB_W, FB_H)?;
                fps.on_presented(dt);
                continue;
            }
        }

        // ── P5#38 Death page ─────────────────────────────────────────────
        if app.screen.is_death() {
            let mut dkey = DeathKey::Other;
            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                dkey = DeathKey::Rebirth;
            }
            if window.is_key_pressed(Key::Enter, KeyRepeat::No)
                || window.is_key_pressed(Key::NumPadEnter, KeyRepeat::No)
            {
                dkey = DeathKey::Confirm;
            }
            if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                dkey = DeathKey::Quit;
            }
            match death_key_command(app.screen, dkey) {
                ScreenCommand::Quit => break,
                ScreenCommand::Rebirth => {
                    let rcfg = rebirth_session_config(&cfg);
                    let content = session.content.clone();
                    eprintln!("rebirth: reconnect LOGIN {}:{} …", rcfg.host, rcfg.port);
                    match ClientSession::connect_with_content(&rcfg, content) {
                        Ok(mut new_sess) if matches!(new_sess.login, LoginOutcome::Accepted) => {
                            new_sess.content.setup_eyes_and_mouth(|sid| {
                                let m = sprites.ensure_meta(sid);
                                Some(m.tag.clone())
                            });
                            app.settings.apply_to_banks(Some(&mut new_sess.sounds), None);
                            session = new_sess;
                            app.enter_playing_from_death();
                            pan = (0.0, 0.0);
                            was_lmb = false;
                            was_rmb = false;
                            mouse_down_frames = 0;
                            hover = HoverPick::default();
                            last_status = "rebirth ok".into();
                            scene.clear_hud();
                            eprintln!("rebirth: ACCEPTED — back to Playing");
                        }
                        Ok(new_sess) => {
                            last_status = format!("rebirth login {:?}", new_sess.login);
                            eprintln!("rebirth: {:?}", new_sess.login);
                        }
                        Err(e) => {
                            last_status = format!("rebirth err {e}");
                            eprintln!("rebirth failed: {e}");
                        }
                    }
                }
                ScreenCommand::None => {}
            }
            if let Some(summary) = app.death_summary() {
                draw_death_screen(&mut fb, summary);
            } else {
                draw_death_screen(
                    &mut fb,
                    &ohol_headless::DeathSummary::new("Unknown", 0.0, None),
                );
            }
            window.set_title(&window_title(
                "Death",
                &format!(
                    "{:.0} FPS R/Enter=Rebirth Esc=Quit | {}",
                    fps.fps(),
                    if last_status.is_empty() {
                        "you died"
                    } else {
                        last_status.as_str()
                    }
                ),
            ));
            rgba_to_u32(&fb.pixels, &mut buf);
            window.update_with_buffer(&buf, FB_W, FB_H)?;
            fps.on_presented(dt);
            continue;
        }

        // Esc/F3 → Settings handled at top of loop (with suppress_close).

        if window.is_key_pressed(Key::Left, KeyRepeat::Yes) || window.is_key_down(Key::A) {
            pan.0 -= 0.4;
        }
        if window.is_key_pressed(Key::Right, KeyRepeat::Yes) || window.is_key_down(Key::D) {
            pan.0 += 0.4;
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::Yes) || window.is_key_down(Key::W) {
            pan.1 += 0.4;
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::Yes) || window.is_key_down(Key::S) {
            pan.1 -= 0.4;
        }
        // Play-window zoom: +/− (main + numpad) and mouse wheel — not only Settings.
        if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
            scene.camera.zoom = (scene.camera.zoom * 1.03)
                .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
            app.settings.zoom = scene.camera.zoom;
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
            scene.camera.zoom = (scene.camera.zoom / 1.03)
                .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
            app.settings.zoom = scene.camera.zoom;
        }
        if let Some((_sx, sy)) = window.get_scroll_wheel() {
            // Wheel up → zoom in (common desktop mapping).
            if sy.abs() > 1e-6 {
                let factor = if sy > 0.0 { 1.10 } else { 1.0 / 1.10 };
                scene.camera.zoom = (scene.camera.zoom * factor)
                    .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
                app.settings.zoom = scene.camera.zoom;
                let _ = app.settings.save_default();
                log_status(
                    &mut last_status,
                    &format!("zoom {:.0}", scene.camera.zoom),
                );
            }
        }
        // Persist zoom when +/− keys release (avoid writing every frame while held).
        if window.is_key_released(Key::Equal)
            || window.is_key_released(Key::Minus)
            || window.is_key_released(Key::NumPadPlus)
            || window.is_key_released(Key::NumPadMinus)
        {
            app.settings.zoom = scene.camera.zoom;
            let _ = app.settings.save_default();
        }

        // C++ idle KA (15s) — without this the server may drop the socket after a short idle.
        let _ = session.maybe_send_ka();
        for _ in 0..48 {
            match session.poll_event() {
                Ok(_) => {}
                Err(e) => {
                    let k = e.kind();
                    if k == std::io::ErrorKind::WouldBlock
                        || k == std::io::ErrorKind::TimedOut
                    {
                        break;
                    }
                    // UnexpectedEof / connection reset: stop draining this frame.
                    log_status(&mut last_status, &format!("poll: {e}"));
                    break;
                }
            }
        }
        // Fractional path step (C++ per-frame currentPos) — needed for walk anim + smooth motion.
        session.step_move_pos(dt as f64);
        if note_our_death_if_any(&mut app, session.world.our()) {
            last_status = "died".into();
            eprintln!(
                "death: {}",
                app.death_summary()
                    .map(|s| format!("{} age={:.1} {:?}", s.name, s.age_years, s.reason))
                    .unwrap_or_else(|| "?".into())
            );
            continue;
        }

        if let Some(me) = session.world.our() {
            // Fractional mid-path camera (C++ currentPos) when walking.
            if session.move_state.in_motion {
                scene.camera.x = session.move_state.current_pos_x as f32 + pan.0;
                scene.camera.y = session.move_state.current_pos_y as f32 + pan.1;
            } else {
                scene.camera.x = me.x as f32 + pan.0;
                scene.camera.y = me.y as f32 + pan.1;
            }
        }

        let lmb = window.get_mouse_down(MouseButton::Left);
        let rmb = window.get_mouse_down(MouseButton::Right);
        // Edge before was_lmb update — used for debug SNAP button hit.
        let lmb_press = lmb && !was_lmb;
        let drop_key = window.is_key_pressed(Key::Q, KeyRepeat::No);
        if window.is_key_pressed(Key::T, KeyRepeat::No) {
            match session.send_say("HI") {
                Ok(line) => log_status(&mut last_status, &format!("SAY {line}")),
                Err(e) => log_status(&mut last_status, &format!("SAY err {e}")),
            }
        }
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let clothing_keys = [
            Key::Key1,
            Key::Key2,
            Key::Key3,
            Key::Key4,
            Key::Key5,
            Key::Key6,
        ];
        for (slot, key) in clothing_keys.iter().enumerate() {
            if !window.is_key_pressed(*key, KeyRepeat::No) {
                continue;
            }
            let slot = slot as i32;
            let name = CLOTHING_SLOT_NAMES
                .get(slot as usize)
                .copied()
                .unwrap_or("?");
            let held = our_held_id(&session);
            let result = if shift {
                click_sremv_clothing(&mut session, slot, -1)
            } else if held > 0 {
                click_drop_clothing(&mut session, slot)
            } else {
                click_remove_clothing(&mut session, slot)
            };
            match result {
                Ok(r) => log_status(
                    &mut last_status,
                    &format!(
                        "CLOTH[{name}/{slot}] held={held} sent={} {}",
                        r.action_sent, r.action_line
                    ),
                ),
                Err(e) => log_status(
                    &mut last_status,
                    &format!("CLOTH[{name}/{slot}] err {e:?}"),
                ),
            }
            break;
        }

        if let Some((mx, my)) = safe_mouse_pos(&window) {
            scene.hud.set_pointer(mx as f32, my as f32);
            hover = if let Some(me) = session.world.our() {
                let age = me.current_age();
                let worn = WornClothingPickTarget {
                    tile_x: me.x,
                    tile_y: me.y,
                    facing: me.facing,
                    age,
                    clothing: &me.clothing,
                };
                update_scene_hover_with_clothing(
                    &mut scene,
                    &session.map,
                    &session.content,
                    &mut sprites,
                    Some(&worn),
                    mx,
                    my,
                    FB_W as u32,
                    FB_H as u32,
                )
            } else {
                update_scene_hover(
                    &mut scene,
                    &session.map,
                    &session.content,
                    &mut sprites,
                    mx,
                    my,
                    FB_W as u32,
                    FB_H as u32,
                )
            };

            if lmb {
                mouse_down_frames = mouse_down_frames.saturating_add(1);
            } else {
                mouse_down_frames = 0;
            }
            let clothing_slot = hover.clothing_slot;
            let hit_slot = hover.contained_slot;
            // Don't path on SNAP button clicks.
            let on_snap_btn = app.settings.debug
                && safe_mouse_pos(&window)
                    .map(|(mx, my)| {
                        snapshot_button_hit(FB_W as u32, FB_H as u32, mx as i32, my as i32)
                    })
                    .unwrap_or(false);
            if lmb && !on_snap_btn {
                let first = !was_lmb;
                match walk_or_use_tile_hold(
                    &mut session,
                    hover.tile.0,
                    hover.tile.1,
                    !first,
                    mouse_down_frames,
                    clothing_slot,
                    hit_slot,
                ) {
                    Ok(r) => {
                        if first {
                            log_status(
                                &mut last_status,
                                &format!("LMB ({},{}) {:?}", hover.tile.0, hover.tile.1, r),
                            );
                        }
                    }
                    Err(e) => {
                        if first {
                            log_status(&mut last_status, &format!("LMB err {e:?}"));
                        }
                    }
                }
            }
            if (rmb && !was_rmb) || drop_key {
                match click_rmb_tile_ex(
                    &mut session,
                    hover.tile.0,
                    hover.tile.1,
                    clothing_slot,
                    hit_slot,
                ) {
                    Ok(r) => log_status(
                        &mut last_status,
                        &format!(
                            "RMB/Q ({},{}) {}",
                            hover.tile.0, hover.tile.1, r.label()
                        ),
                    ),
                    Err(e) => log_status(&mut last_status, &format!("RMB err {e:?}")),
                }
            }
            was_lmb = lmb;
            was_rmb = rmb;
        }

        let dying = session.world.our().map(|p| p.dying).unwrap_or(false);
        scene.sync_hud_ex(
            session.food.as_ref(),
            session.heat.as_ref(),
            session.curse_tokens,
            session.excess_curse_points,
            dying,
        );

        let saved_hl = scene.highlight_tile.take();
        scene.draw(
            &mut fb,
            &mut session.map,
            &mut session.world,
            &session.content,
            &mut sprites,
            &mut anims,
            dt,
        );
        scene.highlight_tile = saved_hl;
        draw_hover_outline(&mut fb, &scene.camera, hover);

        // Debug play-snapshot tools (settings.debug): F9 or SNAP button.
        if app.settings.debug {
            draw_snapshot_button(&mut fb);
            let f9 = window.is_key_pressed(Key::F9, KeyRepeat::No);
            let snap_click = lmb_press
                && safe_mouse_pos(&window)
                    .map(|(mx, my)| {
                        snapshot_button_hit(FB_W as u32, FB_H as u32, mx as i32, my as i32)
                    })
                    .unwrap_or(false);
            if f9 || snap_click {
                let extras = SnapshotViewExtras {
                    label: "play".into(),
                    camera_x: scene.camera.x,
                    camera_y: scene.camera.y,
                    camera_zoom: scene.camera.zoom,
                    pan_x: pan.0,
                    pan_y: pan.1,
                    hover_tile_x: hover.tile.0,
                    hover_tile_y: hover.tile.1,
                    hover_object_id: hover.object_id,
                    last_status: last_status.clone(),
                    screen: "Playing".into(),
                };
                match write_play_snapshot(&session, &extras, None) {
                    Ok(path) => {
                        log_status(
                            &mut last_status,
                            &format!("SNAP {}", path.display()),
                        );
                        eprintln!("play snapshot: {}", path.display());
                    }
                    Err(e) => log_status(&mut last_status, &format!("SNAP err {e}")),
                }
            }
        }

        let dbg = if app.settings.debug { " F9=SNAP" } else { "" };
        // Last successful server→client read (title bar “top” status).
        let rx_ago = session.secs_since_last_rx();
        let rx_label = if rx_ago < 0.05 {
            "rx now".to_string()
        } else if rx_ago < 10.0 {
            format!("rx {rx_ago:.1}s ago")
        } else {
            format!("rx {rx_ago:.0}s ago")
        };
        let status = if last_status.is_empty() {
            "play"
        } else {
            last_status.as_str()
        };
        let title = window_title(
            "Play",
            &format!(
                "{:.0} FPS | {rx_label} | {status} | Esc=Settings{dbg}",
                fps.fps(),
            ),
        );
        window.set_title(&title);
        rgba_to_u32(&fb.pixels, &mut buf);
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);

        // AFTER pump: Esc/F3 → open Settings next frame (Win32 async fallback).
        let esc = esc_held(&window);
        let f3 = f3_held(&window);
        if esc_f3.edge(esc, f3) {
            pending_open_settings = true;
            eprintln!(
                "input: {} open Settings (play active={})",
                if f3 { "F3" } else { "ESC" },
                window.is_active()
            );
        }
    }
    Ok(())
}

/// Offline demo using banks already loaded by the single init window (no second loading screen).
fn run_offline_with_banks(
    mut content: ClientContent,
    mut sprites: SpriteBank,
    mut anims: AnimBank,
    ground: ohol_headless::ground_sprites::GroundBank,
    sounds: ohol_headless::sound_bank::SoundBank,
) -> anyhow::Result<()> {
    eprintln!("offline demo — using boot banks (no second loading screen)");
    let root = content
        .root
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    content.setup_eyes_and_mouth(|sid| {
        let m = sprites.ensure_meta(sid);
        Some(m.tag.clone())
    });
    let skins = collect_offline_person_skins(&content);
    eprintln!("offline: {} person skins for picker", skins.len());
    let mut skin_idx = skins
        .iter()
        .position(|(id, _)| *id == 19)
        .unwrap_or(0);
    let mut demo_age = 20.0f32;
    let mut age_slider_drag = false;
    let initial_skin = skins.get(skin_idx).map(|(id, _)| *id).unwrap_or(19);

    let mut map = ClientMap::new();
    let h = MapChunkHeader {
        size_x: 20,
        size_y: 20,
        x: 0,
        y: 0,
        binary_raw_size: None,
        binary_compressed_size: None,
    };
    let mut plain = String::new();
    for y in 0..20 {
        for x in 0..20 {
            let biome = if (x * 3 + y) % 11 == 0 { 3 } else { 0 };
            let obj = if (x, y) == (10, 10) {
                33
            } else if (x + y) % 9 == 0 {
                33
            } else {
                0
            };
            plain.push_str(&format!("{biome}:0:{obj} "));
        }
    }
    let _ = map.apply_mc_plaintext(&h, plain.trim());
    let mut world = LiveWorld::new();
    // PU: id=1 display=skin age=20 (invAgeRate 1e9 → age_rate ~0), stand at 10,10
    let pu_line = format!(
        "1 {initial_skin} 0 0 0 0 33 0 0 0 -1 0.5 1 0 10 10 {demo_age:.1} 999999999 3.75 0;0;0;0;0;0 0 0 -1 0 0"
    );
    if let Some(pu) = parse_pu_line(&pu_line) {
        world.apply_pu(&pu);
        world.set_our_id(1);
    }
    apply_offline_player_look(&mut world, initial_skin, demo_age, &content, &mut sprites);
    let mut preload = vec![initial_skin, 33, 144];
    if let Some(def) = content.get(initial_skin) {
        for s in &def.sprites {
            preload.push(s.sprite_id);
        }
    }
    if let Some(def) = content.get(33) {
        for s in &def.sprites {
            preload.push(s.sprite_id);
        }
    }
    sprites.preload(preload);

    let mut scene = SceneRenderer::default();
    scene.set_content_root(Some(&root));
    scene.ground = ground;
    scene.sounds = sounds;
    scene.hud_sprites = HudSprites::with_default_roots(Some(&root));
    scene.camera = Camera {
        x: 10.0,
        y: 10.0,
        zoom: 36.0,
    };
    scene.sync_hud(
        Some(&FoodChange {
            food_store: 8,
            food_capacity: 12,
            last_ate_id: 31,
            last_ate_fill_max: 4,
            move_speed: 3.75,
            responsible_id: -1,
            yum_bonus: 2,
            yum_multiplier: 1,
        }),
        Some(&HeatChange {
            heat: 0.5,
            food_time: 0.0,
            indoor_bonus: 0.0,
        }),
    );

    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        &window_title("Offline", "Esc=Settings"),
        FB_W,
        FB_H,
        soft_window_opts(),
    )?;
    window.set_target_fps(60);
    let chars: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    window.set_input_callback(Box::new(CharQueue {
        chars: Rc::clone(&chars),
    }));
    let mut buf = vec![0u32; FB_W * FB_H];

    let mut app = ClientAppState::from_env();
    app.enter_playing();
    let mut last = Instant::now();
    let mut hover = HoverPick::default();
    let mut fps = FpsMeter::new("offline");
    let mut was_lmb = false;
    let mut was_lmb_account = false;
    let mut esc_f3 = EscF3Edge::default();
    let mut pending_open = false;
    let mut pending_close = false;
    let settings_hud = HudSprites::with_default_roots(Some(&root));
    eprintln!("offline: Esc/F3 opens Settings (no server needed)");

    while window.is_open() {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();

        let mut suppress_close = false;
        if pending_open {
            pending_open = false;
            if app.enter_settings() {
                suppress_close = true;
                esc_f3.mark_opened();
                eprintln!("settings: opened offline");
            }
        }
        if pending_close {
            pending_close = false;
            if app.screen.is_settings() {
                app.leave_settings();
                eprintln!("settings: closed offline");
            }
        }

        if app.screen.is_settings() {
            let mut was_settings_lmb = was_lmb;
            match handle_settings_input(
                &window,
                &mut app,
                &mut was_settings_lmb,
                suppress_close,
                false,
            ) {
                SettingsLoop::Left | SettingsLoop::Continue => {}
                SettingsLoop::Restart => restart_client_process(),
                SettingsLoop::OpenAccount => {
                    app.enter_account_from_settings();
                }
            }
            was_lmb = was_settings_lmb;
            if app.screen.is_settings() {
                // Dimmed offline world + glass settings.
                let saved_hl = scene.highlight_tile.take();
                scene.draw(
                    &mut fb,
                    &mut map,
                    &mut world,
                    &content,
                    &mut sprites,
                    &mut anims,
                    dt,
                );
                scene.highlight_tile = saved_hl;
                app.settings.draw_overlay(&mut fb, Some(&settings_hud));
                window.set_title(&window_title(
                    "Settings",
                    &format!("offline Esc=Back | {:.0} FPS", fps.fps()),
                ));
                rgba_to_u32(&fb.pixels, &mut buf);
                window.update_with_buffer(&buf, FB_W, FB_H)?;
                fps.on_presented(dt);
                let esc = esc_held(&window);
                let f3 = f3_held(&window);
                if esc_f3.edge(esc, f3) {
                    eprintln!(
                        "input: {} (offline settings)",
                        if f3 { "F3" } else { "ESC" }
                    );
                    if !suppress_close {
                        pending_close = true;
                    }
                }
                continue;
            }
        }

        // Nested Account form from Settings.
        if app.screen.is_account() {
            app.account.step(dt);
            {
                let mut q = chars.borrow_mut();
                for u in q.drain(..) {
                    if let Some(c) = char::from_u32(u) {
                        let _ = app.account.on_key(AccountKey::Char(c));
                    }
                }
            }
            let mut action = pump_account_keys(&window, &mut app);
            let lmb = window.get_mouse_down(MouseButton::Left);
            if lmb && !was_lmb_account {
                if let Some((mx, my)) = safe_mouse_pos(&window) {
                    let a = app.account.on_pointer_down(
                        mx,
                        my,
                        FB_W as f32,
                        FB_H as f32,
                        Some(&settings_hud),
                    );
                    if a != AccountAction::None {
                        action = a;
                    }
                }
            }
            was_lmb_account = lmb;
            match action {
                AccountAction::Back | AccountAction::Saved | AccountAction::Connect => {
                    app.return_to_settings_from_account();
                    was_lmb_account = false;
                }
                AccountAction::OpenSettings => {
                    let _ = app.enter_settings();
                }
                AccountAction::Quit | AccountAction::None => {}
            }
            if app.screen.is_account() {
                let saved_hl = scene.highlight_tile.take();
                scene.draw(
                    &mut fb,
                    &mut map,
                    &mut world,
                    &content,
                    &mut sprites,
                    &mut anims,
                    dt,
                );
                scene.highlight_tile = saved_hl;
                app.account.draw_overlay(&mut fb, Some(&settings_hud));
                window.set_title(&window_title("Account", "offline Esc=Back"));
                rgba_to_u32(&fb.pixels, &mut buf);
                window.update_with_buffer(&buf, FB_W, FB_H)?;
                fps.on_presented(dt);
                continue;
            }
        }

        // Offline zoom: +/− / numpad / mouse wheel (same as live play).
        if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
            scene.camera.zoom = (scene.camera.zoom * 1.03)
                .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
            scene.camera.zoom = (scene.camera.zoom / 1.03)
                .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
        }
        if let Some((_sx, sy)) = window.get_scroll_wheel() {
            if sy.abs() > 1e-6 {
                let factor = if sy > 0.0 { 1.10 } else { 1.0 / 1.10 };
                scene.camera.zoom = (scene.camera.zoom * factor)
                    .clamp(ohol_headless::render::ZOOM_MIN, ohol_headless::render::ZOOM_MAX);
            }
        }

        // Offline demo panel: age slider + skin select (mouse).
        let layout = offline_demo_panel_layout(FB_W as f32, FB_H as f32);
        let lmb = window.get_mouse_down(MouseButton::Left);
        let lmb_press = lmb && !was_lmb;
        let mut over_panel = false;
        if let Some((mx, my)) = safe_mouse_pos(&window) {
            over_panel = layout.panel.contains(mx, my);
            if age_slider_drag && lmb {
                let t = ((mx - layout.age_track.x) / layout.age_track.w).clamp(0.0, 1.0);
                demo_age = t * 60.0;
                apply_offline_player_look(
                    &mut world,
                    skins.get(skin_idx).map(|(id, _)| *id).unwrap_or(19),
                    demo_age,
                    &content,
                    &mut sprites,
                );
            } else if lmb_press {
                if layout.age_track.contains(mx, my) || layout.age_row.contains(mx, my) {
                    age_slider_drag = true;
                    let t = ((mx - layout.age_track.x) / layout.age_track.w).clamp(0.0, 1.0);
                    demo_age = t * 60.0;
                    apply_offline_player_look(
                        &mut world,
                        skins.get(skin_idx).map(|(id, _)| *id).unwrap_or(19),
                        demo_age,
                        &content,
                        &mut sprites,
                    );
                } else if layout.skin_prev.contains(mx, my) && !skins.is_empty() {
                    skin_idx = (skin_idx + skins.len() - 1) % skins.len();
                    let sid = skins[skin_idx].0;
                    apply_offline_player_look(
                        &mut world, sid, demo_age, &content, &mut sprites,
                    );
                } else if layout.skin_next.contains(mx, my) && !skins.is_empty() {
                    skin_idx = (skin_idx + 1) % skins.len();
                    let sid = skins[skin_idx].0;
                    apply_offline_player_look(
                        &mut world, sid, demo_age, &content, &mut sprites,
                    );
                }
            }
            if !lmb {
                age_slider_drag = false;
            }
        } else if !lmb {
            age_slider_drag = false;
        }
        was_lmb = lmb;

        // Offline: hitMap hover (no session actions) — skip when using the panel.
        if !over_panel {
            if let Some((mx, my)) = safe_mouse_pos(&window) {
                hover = update_scene_hover(
                    &mut scene,
                    &map,
                    &content,
                    &mut sprites,
                    mx,
                    my,
                    FB_W as u32,
                    FB_H as u32,
                );
            }
        }
        let skin_label = skins
            .get(skin_idx)
            .map(|(id, n)| format!("{n} (#{id})"))
            .unwrap_or_else(|| "skin?".into());
        let hit = if hover.hit_map { "hit" } else { "tile" };
        window.set_title(&window_title(
            "Offline",
            &format!(
                "{:.0} FPS | age {:.0} | {skin_label} | Esc=Settings",
                fps.fps(),
                demo_age,
            ),
        ));
        let _ = hit;
        let saved_hl = scene.highlight_tile.take();
        scene.draw(
            &mut fb,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            dt,
        );
        scene.highlight_tile = saved_hl;
        if !over_panel {
            draw_hover_outline(&mut fb, &scene.camera, hover);
        }
        draw_offline_demo_panel(
            &mut fb,
            &layout,
            demo_age,
            &skin_label,
            skins.len(),
            skin_idx,
            age_slider_drag,
        );
        rgba_to_u32(&fb.pixels, &mut buf);
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);

        // AFTER pump: Esc/F3 open Settings next frame (Win32 async fallback).
        let esc = esc_held(&window);
        let f3 = f3_held(&window);
        if esc_f3.edge(esc, f3) {
            pending_open = true;
            eprintln!(
                "input: {} open Settings (offline active={})",
                if f3 { "F3" } else { "ESC" },
                window.is_active()
            );
        }
    }
    Ok(())
}

fn log_status(last: &mut String, msg: &str) {
    eprintln!("{msg}");
    *last = msg.chars().take(48).collect();
}

// ── Offline demo: age slider + person skin picker ────────────────────────────

#[derive(Clone, Copy)]
struct OfflineHit {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl OfflineHit {
    fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

struct OfflineDemoPanelLayout {
    panel: OfflineHit,
    age_row: OfflineHit,
    age_track: OfflineHit,
    skin_prev: OfflineHit,
    skin_next: OfflineHit,
    skin_label: OfflineHit,
}

fn offline_demo_panel_layout(fb_w: f32, fb_h: f32) -> OfflineDemoPanelLayout {
    let _ = fb_w;
    let panel_w = 320.0f32;
    let panel_h = 118.0f32;
    let panel_x = 12.0f32;
    let panel_y = fb_h - panel_h - 12.0;
    let pad = 14.0f32;
    let age_y = panel_y + 36.0;
    let track_w = panel_w - pad * 2.0 - 56.0;
    let track_h = 12.0f32;
    let skin_y = panel_y + 72.0;
    let btn = 28.0f32;
    OfflineDemoPanelLayout {
        panel: OfflineHit {
            x: panel_x,
            y: panel_y,
            w: panel_w,
            h: panel_h,
        },
        age_row: OfflineHit {
            x: panel_x + pad,
            y: age_y - 4.0,
            w: panel_w - pad * 2.0,
            h: 22.0,
        },
        age_track: OfflineHit {
            x: panel_x + pad,
            y: age_y,
            w: track_w,
            h: track_h,
        },
        skin_prev: OfflineHit {
            x: panel_x + pad,
            y: skin_y,
            w: btn,
            h: btn,
        },
        skin_next: OfflineHit {
            x: panel_x + panel_w - pad - btn,
            y: skin_y,
            w: btn,
            h: btn,
        },
        skin_label: OfflineHit {
            x: panel_x + pad + btn + 6.0,
            y: skin_y,
            w: panel_w - pad * 2.0 - btn * 2.0 - 12.0,
            h: btn,
        },
    }
}

/// Base person objects for the offline skin picker (not clothing dummies).
fn collect_offline_person_skins(content: &ClientContent) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = content
        .objects
        .iter()
        .filter_map(|(&id, def)| {
            if def.person == 0 {
                return None;
            }
            if def.dummy_parent != 0 || def.variable_dummy_parent != 0 {
                return None;
            }
            // Prefer named Female/Male skins; still keep other base persons.
            let name = if def.name.trim().is_empty() {
                format!("#{id}")
            } else {
                def.name.trim().to_string()
            };
            Some((id, name))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    if out.is_empty() {
        out.push((19, "Female001".into()));
    }
    out
}

fn apply_offline_player_look(
    world: &mut LiveWorld,
    display_id: i32,
    age_years: f32,
    content: &ClientContent,
    sprites: &mut SpriteBank,
) {
    let age = age_years.clamp(0.0, 60.0);
    let our_id = world.our().map(|o| o.id);
    if let Some(id) = our_id {
        if let Some(o) = world.get_mut(id) {
            o.display_id = display_id;
            o.age = age;
            // Freeze aging while the offline slider owns the value.
            o.age_rate = 0.0;
            o.last_age_set = Instant::now();
            // Keep idle ground anim on skin change.
            o.moving = false;
        }
    }
    // Warm sprites for the new person so limbs appear immediately.
    if let Some(def) = content.get(display_id) {
        let mut ids: Vec<i32> = def.sprites.iter().map(|s| s.sprite_id).collect();
        ids.push(display_id);
        sprites.preload(ids);
    }
}

fn draw_offline_demo_panel(
    fb: &mut Framebuffer,
    layout: &OfflineDemoPanelLayout,
    age: f32,
    skin_label: &str,
    skin_count: usize,
    skin_idx: usize,
    age_drag: bool,
) {
    use ohol_headless::ui_font::draw_ui_text;
    let p = layout.panel;
    // Glass panel
    fb.fill_rect(
        p.x as i32 + 4,
        p.y as i32 + 6,
        p.w as i32,
        p.h as i32,
        [0, 0, 0, 100],
    );
    fb.fill_rect(p.x as i32, p.y as i32, p.w as i32, p.h as i32, [24, 28, 36, 230]);
    fb.fill_rect(p.x as i32, p.y as i32, p.w as i32, 3, [96, 165, 250, 255]);

    let white = [236, 240, 248, 255];
    let dim = [148, 158, 176, 255];
    let accent = [110, 185, 225, 255];

    draw_ui_text(
        fb,
        "Offline demo",
        p.x + 14.0,
        p.y + 18.0,
        14.0,
        accent,
        false,
    );

    // Age label + value
    draw_ui_text(
        fb,
        &format!("Age  {:.0}", age.clamp(0.0, 60.0)),
        layout.age_track.x,
        layout.age_track.y - 12.0,
        12.0,
        dim,
        false,
    );
    let t = (age / 60.0).clamp(0.0, 1.0);
    let tr = layout.age_track;
    fb.fill_rect(
        tr.x as i32,
        tr.y as i32,
        tr.w as i32,
        tr.h as i32,
        [40, 44, 54, 255],
    );
    let fill_w = (tr.w * t).round() as i32;
    if fill_w > 0 {
        fb.fill_rect(
            tr.x as i32,
            tr.y as i32,
            fill_w,
            tr.h as i32,
            if age_drag {
                [90, 170, 230, 255]
            } else {
                [70, 140, 190, 255]
            },
        );
    }
    let kx = tr.x + tr.w * t - 5.0;
    fb.fill_rect(
        kx.round() as i32,
        (tr.y - 3.0) as i32,
        10,
        tr.h as i32 + 6,
        white,
    );
    draw_ui_text(
        fb,
        "0",
        tr.x,
        tr.y + tr.h + 10.0,
        10.0,
        dim,
        false,
    );
    draw_ui_text(
        fb,
        "60",
        tr.x + tr.w - 14.0,
        tr.y + tr.h + 10.0,
        10.0,
        dim,
        false,
    );

    // Skin select
    draw_ui_text(
        fb,
        &format!(
            "Skin  {}/{}",
            if skin_count == 0 { 0 } else { skin_idx + 1 },
            skin_count
        ),
        layout.skin_prev.x,
        layout.skin_prev.y - 12.0,
        12.0,
        dim,
        false,
    );
    // Prev / next buttons
    for (hit, label) in [
        (layout.skin_prev, "<"),
        (layout.skin_next, ">"),
    ] {
        fb.fill_rect(
            hit.x as i32,
            hit.y as i32,
            hit.w as i32,
            hit.h as i32,
            [48, 58, 74, 255],
        );
        draw_ui_text(
            fb,
            label,
            hit.x + hit.w * 0.5,
            hit.y + hit.h * 0.5,
            16.0,
            white,
            true,
        );
    }
    // Label box (select display)
    let lab = layout.skin_label;
    fb.fill_rect(
        lab.x as i32,
        lab.y as i32,
        lab.w as i32,
        lab.h as i32,
        [18, 22, 30, 255],
    );
    // Truncate long names for display.
    let shown = if skin_label.chars().count() > 22 {
        let s: String = skin_label.chars().take(20).collect();
        format!("{s}…")
    } else {
        skin_label.to_string()
    };
    draw_ui_text(
        fb,
        &shown,
        lab.x + lab.w * 0.5,
        lab.y + lab.h * 0.5,
        12.0,
        white,
        true,
    );
}

/// Present one soft-FB loading frame (P5#36).
fn present_loading(
    window: &mut Window,
    fb: &mut Framebuffer,
    buf: &mut [u32],
    state: &LoadingState,
) -> anyhow::Result<()> {
    draw_loading_progress(fb, state);
    rgba_to_u32(&fb.pixels, buf);
    window.update_with_buffer(buf, FB_W, FB_H)?;
    Ok(())
}

/// Load anim / ground / sprites / sounds / music with soft-FB progress.
///
/// When `content_already_loaded`, reports Content stage complete first (session path).
fn load_graphics_with_progress(
    root: &std::path::Path,
    content_already_loaded: bool,
    window: &mut Window,
    fb: &mut Framebuffer,
    buf: &mut [u32],
) -> anyhow::Result<(SpriteBank, AnimBank, SceneRenderer)> {
    use ohol_headless::emotion::EmotionBank;
    use ohol_headless::ground_sprites::GroundBank;
    use ohol_headless::sound_bank::SoundBank;

    if content_already_loaded {
        present_loading(
            window,
            fb,
            buf,
            &LoadingState::for_stage(LoadStage::Content, 1.0, Some("session")),
        )?;
    }

    let (sprites, anims, ground, sounds) = {
        let mut on_progress = |state: &LoadingState| {
            let _ = present_loading(window, fb, buf, state);
        };

        let anims = AnimBank::load_prefer_cache_with_progress(root, Some(&mut on_progress));

        let mut ground =
            GroundBank::load_prefer_cache_with_progress(root, Some(&mut on_progress));
        let _ = ground.preload_overlays();

        let sprites = SpriteBank::load_prefer_cache_with_progress(root, Some(&mut on_progress));

        let sounds = SoundBank::load_prefer_cache_with_progress(root, Some(&mut on_progress));

        let _music = MusicBank::load_prefer_cache_with_progress(root, Some(&mut on_progress));

        (sprites, anims, ground, sounds)
    };

    let mut scene = SceneRenderer::default();
    scene.ground = ground;
    scene.hud_sprites = HudSprites::with_default_roots(Some(root));
    scene.emotions = EmotionBank::load_from_content_root(root);
    scene.sounds = sounds;

    present_loading(window, fb, buf, &LoadingState::finished())?;
    Ok((sprites, anims, scene))
}

fn rgba_to_u32(rgba: &[u8], out: &mut [u32]) {
    let n = out.len().min(rgba.len() / 4);
    for i in 0..n {
        let o = i * 4;
        let r = rgba[o] as u32;
        let g = rgba[o + 1] as u32;
        let b = rgba[o + 2] as u32;
        // minifb 0x00RRGGBB
        out[i] = (r << 16) | (g << 8) | b;
    }
}

/// Copy soft-FB RGBA into a pixels/wgpu frame (RGBA8).
fn rgba_copy_frame(rgba: &[u8], frame: &mut [u8]) {
    let n = frame.len().min(rgba.len());
    frame[..n].copy_from_slice(&rgba[..n]);
}

/// Live play with **GPU present** (pixels → wgpu texture + hardware scale).
///
/// Scene is still soft-FB authored (same `SceneRenderer`); the CPU buffer is uploaded
/// each frame and scaled by the GPU (smoother than minifb 1:1). Soft mode remains
/// available via Settings → Graphics.
fn run_session_gpu(
    mut session: ClientSession,
    mut sprites: SpriteBank,
    mut anims: AnimBank,
    cfg: SessionConfig,
    mut app: ClientAppState,
) -> anyhow::Result<()> {
    use pixels::{Pixels, SurfaceTexture};
    use winit::dpi::LogicalSize;
    use winit::event::{
        ElementState, Event, MouseButton as WMouse, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    };
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    let root = session.content.root.clone().unwrap_or_else(|| {
        std::path::PathBuf::from(r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7")
    });
    sprites.preload([19, 33, 144]);
    session.content.setup_eyes_and_mouth(|sid| {
        let m = sprites.ensure_meta(sid);
        Some(m.tag.clone())
    });
    let mut scene = SceneRenderer::default();
    scene.set_content_root(Some(&root));
    scene.camera.zoom = app.settings.zoom.clamp(
        ohol_headless::render::ZOOM_MIN,
        ohol_headless::render::ZOOM_MAX,
    );
    // Same soft-FB size as soft path (960×540) — GPU only scales/presents.
    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    app.settings.apply_to_banks(Some(&mut session.sounds), None);
    app.settings.apply_to_banks(Some(&mut scene.sounds), None);
    let want_fullscreen = app.settings.fullscreen;

    let event_loop = EventLoop::new();
    let window = {
        // Windowed: original comfortable size (960×540). Fullscreen: borderless monitor.
        let mut wb = WindowBuilder::new().with_title("Open Life (GPU present)");
        if want_fullscreen {
            wb = wb
                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
                .with_decorations(false);
        } else {
            let size = LogicalSize::new(FB_W as f64, FB_H as f64);
            wb = wb
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_decorations(true);
        }
        wb.build(&event_loop)
            .map_err(|e| anyhow::anyhow!("window: {e}"))?
    };
    // pixels integer-scales its buffer → letterbox on non-integer ratios.
    // Keep soft-FB at FB_W×FB_H for draw, but size the *present* buffer to the
    // window and nearest-stretch each frame so tiles always fill the monitor.
    let mut pixels = {
        let win_size = window.inner_size();
        let pw = win_size.width.max(1);
        let ph = win_size.height.max(1);
        let surface = SurfaceTexture::new(pw, ph, &window);
        Pixels::new(pw, ph, surface).map_err(|e| anyhow::anyhow!("pixels/wgpu: {e}"))?
    };
    pixels.clear_color(pixels::wgpu::Color {
        r: 72.0 / 255.0,
        g: 96.0 / 255.0,
        b: 58.0 / 255.0,
        a: 1.0,
    });

    let mut last = Instant::now();
    let mut pan = (0.0f32, 0.0f32);
    let mut was_lmb = false;
    let mut was_rmb = false;
    let mut mouse_down_frames: i32 = 0;
    let mut hover = HoverPick::default();
    let mut last_status = String::new();
    let mut fps = FpsMeter::new("gpu");
    let mut keys_down = std::collections::HashSet::<VirtualKeyCode>::new();
    let mut keys_pressed = std::collections::HashSet::<VirtualKeyCode>::new();
    let mut lmb = false;
    let mut rmb = false;
    let mut was_lmb_settings = false;
    let mut was_lmb_account = false;
    let mut esc_f3 = EscF3Edge::default();
    let mut cursor = (FB_W as f32 * 0.5, FB_H as f32 * 0.5);
    let mut scroll_y = 0.0f32;
    let mut typed_chars: Vec<char> = Vec::new();
    let settings_hud = HudSprites::with_default_roots(Some(&root));
    let fbw = FB_W as u32;
    let fbh = FB_H as u32;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        // Surface + present buffer match window → 1:1 present (no letterbox).
                        let _ = pixels.resize_surface(size.width, size.height);
                        let _ = pixels.resize_buffer(size.width, size.height);
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    // Prefer virtual_keycode; fall back to hardware scancode (Escape/F3).
                    let code = input
                        .virtual_keycode
                        .or_else(|| virtual_key_from_scancode(input.scancode));
                    if let Some(code) = code {
                        match input.state {
                            ElementState::Pressed => {
                                keys_down.insert(code);
                                keys_pressed.insert(code);
                            }
                            ElementState::Released => {
                                keys_down.remove(&code);
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let size = window.inner_size();
                    if size.width > 0 && size.height > 0 {
                        cursor.0 = (position.x as f32) * (fbw as f32) / size.width as f32;
                        cursor.1 = (position.y as f32) * (fbh as f32) / size.height as f32;
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let down = state == ElementState::Pressed;
                    match button {
                        WMouse::Left => lmb = down,
                        WMouse::Right => rmb = down,
                        _ => {}
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    MouseScrollDelta::LineDelta(_, y) => scroll_y += y,
                    MouseScrollDelta::PixelDelta(p) => scroll_y += (p.y as f32) * 0.01,
                },
                WindowEvent::ReceivedCharacter(c) => {
                    if !c.is_control() {
                        typed_chars.push(c);
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let dt = last.elapsed().as_secs_f32().min(0.05);
                last = Instant::now();

                // Rising-edge Esc/F3 from keys_down (and keys_pressed this frame).
                let esc_down = keys_down.contains(&VirtualKeyCode::Escape)
                    || keys_pressed.contains(&VirtualKeyCode::Escape);
                let f3_down = keys_down.contains(&VirtualKeyCode::F3)
                    || keys_pressed.contains(&VirtualKeyCode::F3);
                let esc_f3_edge = esc_f3.edge(esc_down, f3_down);

                // Esc/F3: open from Playing, or close when already in Settings.
                // mark_opened blocks same-hold close until both keys are released.
                let mut suppress_settings_close = false;
                if esc_f3_edge {
                    if app.screen.is_settings() {
                        app.leave_settings();
                        app.apply_settings_to_banks(Some(&mut session.sounds), None);
                        app.apply_settings_to_banks(Some(&mut scene.sounds), None);
                        scene.camera.zoom = app.settings.zoom.clamp(
                            ohol_headless::render::ZOOM_MIN,
                            ohol_headless::render::ZOOM_MAX,
                        );
                        was_lmb_settings = false;
                    } else if app.screen.is_playing() {
                        if app.enter_settings() {
                            suppress_settings_close = true;
                            esc_f3.mark_opened();
                        }
                    }
                }
                let _lmb_press = lmb && !was_lmb;

                if app.screen.is_settings() {
                    let mut action = SettingsAction::None;
                    if keys_pressed.contains(&VirtualKeyCode::Tab) {
                        action = app.settings.on_key(SettingsKey::Tab {
                            shift: keys_down.contains(&VirtualKeyCode::LShift)
                                || keys_down.contains(&VirtualKeyCode::RShift),
                        });
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Up) {
                        action = app.settings.on_key(SettingsKey::Up);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Down) {
                        action = app.settings.on_key(SettingsKey::Down);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Left)
                        || keys_pressed.contains(&VirtualKeyCode::Minus)
                    {
                        action = app.settings.on_key(SettingsKey::Left);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Right)
                        || keys_pressed.contains(&VirtualKeyCode::Equals)
                    {
                        action = app.settings.on_key(SettingsKey::Right);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Return)
                        || keys_pressed.contains(&VirtualKeyCode::Space)
                    {
                        action = app.settings.on_key(SettingsKey::Enter);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Key1) {
                        action = app.settings.on_key(SettingsKey::ToggleAudio);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Key2) {
                        action = app.settings.on_key(SettingsKey::ToggleMusic);
                    }
                    // B = Back (Esc handled above as toggle with release-gate).
                    if !suppress_settings_close && keys_pressed.contains(&VirtualKeyCode::B) {
                        action = app.settings.on_key(SettingsKey::Escape);
                    }
                    // Mouse click/drag for all settings controls.
                    if lmb && !was_lmb_settings {
                        let a = app.settings.on_pointer_down(
                            cursor.0,
                            cursor.1,
                            FB_W as f32,
                            FB_H as f32,
                        );
                        if a != SettingsAction::None {
                            action = a;
                        }
                    } else if lmb && was_lmb_settings && app.settings.slider_drag.is_some() {
                        let a =
                            app.settings
                                .on_pointer_drag(cursor.0, FB_W as f32, FB_H as f32);
                        if a != SettingsAction::None {
                            action = a;
                        }
                    }
                    if !lmb {
                        app.settings.on_pointer_up();
                    }
                    was_lmb_settings = lmb;

                    match action {
                        SettingsAction::Back => {
                            app.leave_settings();
                            was_lmb_settings = false;
                        }
                        SettingsAction::Restart => {
                            let _ = app.settings.save_default();
                            restart_client_process();
                        }
                        SettingsAction::OpenAccount => {
                            app.enter_account_from_settings();
                            was_lmb_settings = false;
                            last_status = "Account settings".into();
                        }
                        SettingsAction::Applied | SettingsAction::None => {}
                    }
                    app.apply_settings_to_banks(Some(&mut session.sounds), None);
                    app.apply_settings_to_banks(Some(&mut scene.sounds), None);
                    if app.screen.is_settings() {
                        // Dimmed world + glass card (world was drawn last frame; redraw light).
                        app.settings.draw_overlay(&mut fb, Some(&settings_hud));
                        window.set_title(&window_title(
                            "Settings",
                            &format!("Esc=Back | SFX {:.0}%", app.settings.sound_volume * 100.0),
                        ));
                    }
                    scene.camera.zoom = app.settings.zoom.clamp(
                        ohol_headless::render::ZOOM_MIN,
                        ohol_headless::render::ZOOM_MAX,
                    );
                } else if app.screen.is_account() {
                    app.account.step(dt);
                    for c in typed_chars.drain(..) {
                        let _ = app.account.on_key(AccountKey::Char(c));
                    }
                    let shift = keys_down.contains(&VirtualKeyCode::LShift)
                        || keys_down.contains(&VirtualKeyCode::RShift);
                    let mut action = AccountAction::None;
                    if keys_pressed.contains(&VirtualKeyCode::Tab) {
                        action = app.account.on_key(AccountKey::Tab { shift });
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Return) {
                        action = app.account.on_key(AccountKey::Enter);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Escape) {
                        action = app.account.on_key(AccountKey::Escape);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Back) {
                        let _ = app.account.on_key(AccountKey::Backspace);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Delete) {
                        let _ = app.account.on_key(AccountKey::Delete);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Left) {
                        let _ = app.account.on_key(AccountKey::Left);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Right) {
                        let _ = app.account.on_key(AccountKey::Right);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::Home) {
                        let _ = app.account.on_key(AccountKey::Home);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::End) {
                        let _ = app.account.on_key(AccountKey::End);
                    }
                    if keys_pressed.contains(&VirtualKeyCode::F2) {
                        let _ = app.account.on_key(AccountKey::ToggleSecretMode);
                    }
                    if lmb && !was_lmb_account {
                        let a = app.account.on_pointer_down(
                            cursor.0,
                            cursor.1,
                            FB_W as f32,
                            FB_H as f32,
                            Some(&settings_hud),
                        );
                        if a != AccountAction::None {
                            action = a;
                        }
                    }
                    was_lmb_account = lmb;
                    match action {
                        AccountAction::Back
                        | AccountAction::Saved
                        | AccountAction::Connect => {
                            app.return_to_settings_from_account();
                            was_lmb_account = false;
                            last_status = "Account saved".into();
                        }
                        AccountAction::OpenSettings => {
                            let _ = app.enter_settings();
                        }
                        AccountAction::Quit | AccountAction::None => {}
                    }
                    if app.screen.is_account() {
                        let saved_hl = scene.highlight_tile.take();
                        scene.draw(
                            &mut fb,
                            &mut session.map,
                            &mut session.world,
                            &session.content,
                            &mut sprites,
                            &mut anims,
                            dt,
                        );
                        scene.highlight_tile = saved_hl;
                        app.account.draw_overlay(&mut fb, Some(&settings_hud));
                        window.set_title(&window_title("Account", "Esc=Back to Settings"));
                    }
                } else if app.screen.is_death() {
                    if keys_pressed.contains(&VirtualKeyCode::R)
                        || keys_pressed.contains(&VirtualKeyCode::Return)
                    {
                        let rcfg = rebirth_session_config(&cfg);
                        let content = session.content.clone();
                        match ClientSession::connect_with_content(&rcfg, content) {
                            Ok(mut new_sess)
                                if matches!(new_sess.login, LoginOutcome::Accepted) =>
                            {
                                new_sess.content.setup_eyes_and_mouth(|sid| {
                                    let m = sprites.ensure_meta(sid);
                                    Some(m.tag.clone())
                                });
                                app.settings
                                    .apply_to_banks(Some(&mut new_sess.sounds), None);
                                let _ =
                                    new_sess.set_read_timeout(Some(Duration::from_millis(1)));
                                session = new_sess;
                                app.enter_playing_from_death();
                                pan = (0.0, 0.0);
                                last_status = "rebirth ok".into();
                            }
                            Ok(s) => last_status = format!("rebirth {:?}", s.login),
                            Err(e) => last_status = format!("rebirth {e}"),
                        }
                    }
                    if let Some(summary) = app.death_summary() {
                        draw_death_screen(&mut fb, summary);
                    }
                } else {
                    if keys_down.contains(&VirtualKeyCode::A)
                        || keys_down.contains(&VirtualKeyCode::Left)
                    {
                        pan.0 -= 0.4;
                    }
                    if keys_down.contains(&VirtualKeyCode::D)
                        || keys_down.contains(&VirtualKeyCode::Right)
                    {
                        pan.0 += 0.4;
                    }
                    if keys_down.contains(&VirtualKeyCode::W)
                        || keys_down.contains(&VirtualKeyCode::Up)
                    {
                        pan.1 += 0.4;
                    }
                    if keys_down.contains(&VirtualKeyCode::S)
                        || keys_down.contains(&VirtualKeyCode::Down)
                    {
                        pan.1 -= 0.4;
                    }
                    if keys_down.contains(&VirtualKeyCode::Equals)
                        || keys_down.contains(&VirtualKeyCode::NumpadAdd)
                    {
                        scene.camera.zoom = (scene.camera.zoom * 1.03).clamp(
                            ohol_headless::render::ZOOM_MIN,
                            ohol_headless::render::ZOOM_MAX,
                        );
                        app.settings.zoom = scene.camera.zoom;
                    }
                    if keys_down.contains(&VirtualKeyCode::Minus)
                        || keys_down.contains(&VirtualKeyCode::NumpadSubtract)
                    {
                        scene.camera.zoom = (scene.camera.zoom / 1.03).clamp(
                            ohol_headless::render::ZOOM_MIN,
                            ohol_headless::render::ZOOM_MAX,
                        );
                        app.settings.zoom = scene.camera.zoom;
                    }
                    if scroll_y.abs() > 1e-6 {
                        let factor = if scroll_y > 0.0 { 1.10 } else { 1.0 / 1.10 };
                        scene.camera.zoom = (scene.camera.zoom * factor).clamp(
                            ohol_headless::render::ZOOM_MIN,
                            ohol_headless::render::ZOOM_MAX,
                        );
                        app.settings.zoom = scene.camera.zoom;
                        let _ = app.settings.save_default();
                        scroll_y = 0.0;
                    }

                    // C++ idle KA so the server does not drop a quiet socket.
                    let _ = session.maybe_send_ka();
                    for _ in 0..48 {
                        match session.poll_event() {
                            Ok(_) => {}
                            Err(e) => {
                                let k = e.kind();
                                if k == std::io::ErrorKind::WouldBlock
                                    || k == std::io::ErrorKind::TimedOut
                                {
                                    break;
                                }
                                log_status(&mut last_status, &format!("poll: {e}"));
                                break;
                            }
                        }
                    }
                    session.step_move_pos(dt as f64);
                    if note_our_death_if_any(&mut app, session.world.our()) {
                        last_status = "died".into();
                    }

                    if let Some(me) = session.world.our() {
                        // Prefer fractional mid-path pos when walking (Jason currentPos).
                        if session.move_state.in_motion {
                            scene.camera.x =
                                session.move_state.current_pos_x as f32 + pan.0;
                            scene.camera.y =
                                session.move_state.current_pos_y as f32 + pan.1;
                        } else {
                            scene.camera.x = me.x as f32 + pan.0;
                            scene.camera.y = me.y as f32 + pan.1;
                        }
                    }

                    hover = if let Some(me) = session.world.our() {
                        let age = me.current_age();
                        let worn = WornClothingPickTarget {
                            tile_x: me.x,
                            tile_y: me.y,
                            facing: me.facing,
                            age,
                            clothing: &me.clothing,
                        };
                        update_scene_hover_with_clothing(
                            &mut scene,
                            &session.map,
                            &session.content,
                            &mut sprites,
                            Some(&worn),
                            cursor.0,
                            cursor.1,
                            fbw,
                            fbh,
                        )
                    } else {
                        update_scene_hover(
                            &mut scene,
                            &session.map,
                            &session.content,
                            &mut sprites,
                            cursor.0,
                            cursor.1,
                            fbw,
                            fbh,
                        )
                    };

                    if lmb {
                        mouse_down_frames = mouse_down_frames.saturating_add(1);
                    } else {
                        mouse_down_frames = 0;
                    }
                    if lmb {
                        let first = !was_lmb;
                        match walk_or_use_tile_hold(
                            &mut session,
                            hover.tile.0,
                            hover.tile.1,
                            !first,
                            mouse_down_frames,
                            hover.clothing_slot,
                            hover.contained_slot,
                        ) {
                            Ok(r) if first => {
                                log_status(
                                    &mut last_status,
                                    &format!(
                                        "LMB ({},{}) {:?}",
                                        hover.tile.0, hover.tile.1, r
                                    ),
                                );
                            }
                            Err(e) if first => {
                                log_status(&mut last_status, &format!("LMB err {e:?}"));
                            }
                            _ => {}
                        }
                    }
                    let rmb_press = rmb && !was_rmb;
                    if rmb_press || keys_pressed.contains(&VirtualKeyCode::Q) {
                        match click_rmb_tile_ex(
                            &mut session,
                            hover.tile.0,
                            hover.tile.1,
                            hover.clothing_slot,
                            hover.contained_slot,
                        ) {
                            Ok(r) => log_status(
                                &mut last_status,
                                &format!("RMB {:?}", r),
                            ),
                            Err(e) => log_status(&mut last_status, &format!("RMB {e:?}")),
                        }
                    }
                    if keys_pressed.contains(&VirtualKeyCode::T) {
                        match session.send_say("HI") {
                            Ok(_) => log_status(&mut last_status, "SAY HI"),
                            Err(e) => log_status(&mut last_status, &format!("SAY {e}")),
                        }
                    }
                    was_lmb = lmb;
                    was_rmb = rmb;

                    let dying = session.world.our().map(|p| p.dying).unwrap_or(false);
                    scene.sync_hud_ex(
                        session.food.as_ref(),
                        session.heat.as_ref(),
                        session.curse_tokens,
                        session.excess_curse_points,
                        dying,
                    );
                    let saved_hl = scene.highlight_tile.take();
                    scene.draw(
                        &mut fb,
                        &mut session.map,
                        &mut session.world,
                        &session.content,
                        &mut sprites,
                        &mut anims,
                        dt,
                    );
                    scene.highlight_tile = saved_hl;
                    draw_hover_outline(&mut fb, &scene.camera, hover);

                    let rx_ago = session.secs_since_last_rx();
                    let rx_label = if rx_ago < 0.05 {
                        "rx now".to_string()
                    } else if rx_ago < 10.0 {
                        format!("rx {rx_ago:.1}s ago")
                    } else {
                        format!("rx {rx_ago:.0}s ago")
                    };
                    window.set_title(&window_title(
                        "Play GPU",
                        &format!(
                            "{:.0} FPS | {rx_label} | {} | Esc=Settings",
                            fps.fps(),
                            if last_status.is_empty() {
                                "play"
                            } else {
                                last_status.as_str()
                            }
                        ),
                    ));
                }
                // Keep LMB edge state even when Settings/Death stole the frame.
                if !app.screen.is_playing() {
                    was_lmb = lmb;
                    was_rmb = rmb;
                }

                // Stretch soft-FB → full window present buffer (fills entire client area).
                let win = window.inner_size();
                let pw = win.width.max(1);
                let ph = win.height.max(1);
                let frame = pixels.frame_mut();
                if frame.len() == (pw as usize) * (ph as usize) * 4 {
                    ohol_headless::stretch_rgba_nearest(
                        &fb.pixels,
                        FB_W as u32,
                        FB_H as u32,
                        frame,
                        pw,
                        ph,
                    );
                } else if frame.len() == fb.pixels.len() {
                    rgba_copy_frame(&fb.pixels, frame);
                } else {
                    // Size mismatch mid-resize — clear to earth void.
                    for px in frame.chunks_exact_mut(4) {
                        px.copy_from_slice(&[72, 96, 58, 255]);
                    }
                }
                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                }
                fps.on_presented(dt);
                keys_pressed.clear();
            }
            _ => {}
        }
    });
}
