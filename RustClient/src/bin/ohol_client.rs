//! Graphical Open Life / OHOL client (protocol + software renderer).
//!
//! ```text
//! cargo run --features gpu --bin ohol-client --release
//! ```
//!
//! Boot: **Account** soft-FB form (P5#37) prefilled from `.env` / `OHOL_*`,
//! then **Loading** progress (P5#36 / C++ LoadingPage) across prefer_cache banks,
//! then live world. Enter=Connect, Esc=skip when creds present, Tab=field, F2=key/password,
//! **F3=Settings** (P5#39: sound/music volume, show FPS).
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
//! - **Settings** (P5#39): F3 from Account or Playing; Tab/arrows, Left/Right adjust, Esc/Back
//! - **Debug tools** (settings.debug): **F9** or bottom-right **SNAP** → play snapshot under `logs/snapshots/`
//!
//! Offline demo if server unavailable.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

use ohol_headless::account_page::{
    AccountAction, AccountKey, AccountPage, ClientAppState, ClientScreen,
};
use ohol_headless::anim_bank::AnimBank;
use ohol_headless::client_map::ClientMap;
use ohol_headless::click_tile::{
    click_drop_clothing, click_remove_clothing, click_sremv_clothing, walk_or_use_tile_hold,
};
use ohol_headless::client_screen::{
    death_key_command, draw_death_screen, note_our_death_if_any, rebirth_session_config, DeathKey,
    ScreenCommand,
};
use ohol_headless::settings_page::{SettingsAction, SettingsKey};
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
    boot_load_prefer_cache, draw_loading_progress, BootBanks, LoadStage, LoadingState,
};
use ohol_headless::music_bank::MusicBank;
use ohol_headless::parse::{FoodChange, HeatChange, LoginOutcome, MapChunkHeader, parse_pu_line};
use ohol_headless::render::{Camera, Framebuffer, SceneRenderer};
use ohol_headless::rmb_action::{click_rmb_tile_ex, our_held_id};
use ohol_headless::session::{ClientSession, SessionConfig};
use ohol_headless::sprite_bank::SpriteBank;

const FB_W: usize = 960;
const FB_H: usize = 540;

/// Safe mouse position for soft-FB hit tests.
///
/// minifb `MouseMode::Clamp` does `x.clamp(0.0, width - 1.0)` and **panics** when the
/// window reports width/height 0 (minimize / transient resize) because max becomes -1.
fn safe_mouse_pos(window: &Window) -> Option<(f32, f32)> {
    let (w, h) = window.get_size();
    if w == 0 || h == 0 {
        return None;
    }
    window.get_mouse_pos(MouseMode::Clamp)
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
    // P5#37: Account soft-FB form → SessionConfig → connect (headless CLI unchanged).
    let mut app = ClientAppState::from_env();
    // Graphic timeouts for live play (same as prior hard-coded client defaults).
    app.account.read_timeout = Duration::from_millis(30);
    app.account.write_timeout = Duration::from_secs(5);

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
        cfg.email,
        if cfg.account_key.is_empty() {
            "(none)"
        } else {
            "(set)"
        },
        cfg.host,
        cfg.port
    );

    // P5#36: soft-FB LoadingPage-style bank boot before TCP login.
    let t_load0 = Instant::now();
    let boot = match run_loading_boot(&mut app) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("loading failed: {e} — offline demo without full boot");
            return run_offline();
        }
    };
    let loading_secs = t_load0.elapsed().as_secs_f64();
    eprintln!(
        "loading: done objects={} transitions={} binary_cache={}",
        boot.content.objects.len(),
        boot.content.transitions.len(),
        boot.used_binary_cache
    );

    let t_connect0 = Instant::now();
    match ClientSession::connect_with_content(&cfg, boot.content) {
        Ok(mut session) if matches!(session.login, LoginOutcome::Accepted) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            // Prefer boot sounds (OLSN index already warm; aiff_opens==0).
            session.sounds = boot.sounds;
            eprintln!(
                "connected objects={} map pending",
                session.content.objects.len()
            );
            eprintln!(
                "controls: LMB walk/use/self | RMB/Q drop/remv | 1-6 clothing | WASD pan | +/- or mouse-wheel zoom | F3 settings | Esc quit"
            );
            eprintln!("death: R/Enter rebirth · Esc quit (after our player dies)");
            app.settings.apply_runtime_globals();
            session.sounds.set_loudness(app.settings.sound_volume);
            session.sounds.set_muted(app.settings.sound_muted);
            // Short polls for the soft-FB frame loop. 30ms blocked idle FPS at ~2–3
            // when the software renderer is already heavy; 1ms keeps WouldBlock snappy
            // without busy-spinning on Windows (0 often means infinite SO_RCVTIMEO).
            let _ = session.set_read_timeout(Some(Duration::from_millis(1)));
            app.enter_playing();
            let to_play_secs = t_start.elapsed().as_secs_f64();
            log_startup_timings(account_secs, loading_secs, connect_secs, to_play_secs, true);
            // Live path reuses pre-booted sprite/anim meta via set_content_root + light preload.
            run_session_from_boot(session, boot.sprites, boot.anims, cfg, app)
        }
        Ok(mut session) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            eprintln!("login: {:?} — offline demo", session.login);
            log_startup_timings(
                account_secs,
                loading_secs,
                connect_secs,
                t_start.elapsed().as_secs_f64(),
                false,
            );
            session.sounds = boot.sounds;
            run_offline_from_boot(boot.sprites, boot.anims, session.content)
        }
        Err(e) => {
            let connect_secs = t_connect0.elapsed().as_secs_f64();
            eprintln!("connect failed: {e} — offline demo");
            log_startup_timings(
                account_secs,
                loading_secs,
                connect_secs,
                t_start.elapsed().as_secs_f64(),
                false,
            );
            // Content already consumed into failed connect; offline reloads via run_offline.
            let _ = (boot.sprites, boot.anims, boot.ground, boot.music);
            run_offline()
        }
    }
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

/// Soft-FB progressive bank load (C++ LoadingPage). Returns prefer_cache banks.
fn run_loading_boot(app: &mut ClientAppState) -> anyhow::Result<BootBanks> {
    app.screen = ClientScreen::Loading;
    app.loading_msg = "prefer_cache…".into();

    let root = resolve_content_root(None).map_err(anyhow::Error::msg)?;
    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        "Open Life — Loading",
        FB_W,
        FB_H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    let mut buf = vec![0u32; FB_W * FB_H];

    // Initial frame so the window is not blank while first stage runs.
    let initial = LoadingState::for_stage(LoadStage::Content, 0.0, Some("starting"));
    draw_loading_progress(&mut fb, &initial);
    rgba_to_u32(&fb.pixels, &mut buf);
    let _ = window.update_with_buffer(&buf, FB_W, FB_H);

    // Present on each progress tick (minifb needs pump; we present every stage update).
    let last_state = std::cell::RefCell::new(initial);
    let mut present = |state: &LoadingState| {
        *last_state.borrow_mut() = state.clone();
        app.loading_msg = state.label.clone();
        draw_loading_progress(&mut fb, state);
        rgba_to_u32(&fb.pixels, &mut buf);
        let pct = (state.overall_fraction * 100.0).round() as i32;
        window.set_title(&format!(
            "Open Life — Loading {pct}%  [{}]",
            state.stage.name()
        ));
        let _ = window.update_with_buffer(&buf, FB_W, FB_H);
    };

    let banks = {
        let mut cb = |s: &LoadingState| present(s);
        boot_load_prefer_cache(&root, Some(&mut cb)).map_err(anyhow::Error::msg)?
    };

    // Final "Ready" frame
    let done = LoadingState::finished();
    present(&done);
    Ok(banks)
}

/// Account form loop. Returns `Some(SessionConfig)` on Connect, `None` on Quit.
fn run_account_boot(app: &mut ClientAppState) -> anyhow::Result<Option<SessionConfig>> {
    // Automated playtest: skip form when creds present (OHOL_AUTO_CONNECT=1/true).
    let auto = std::env::var("OHOL_AUTO_CONNECT")
        .map(|v| {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false);
    if auto {
        let cfg = app.account.build_session_config();
        if !cfg.email.is_empty()
            && (!cfg.account_key.is_empty() || !cfg.password.is_empty())
        {
            eprintln!(
                "account: OHOL_AUTO_CONNECT → {}:{} email={}",
                cfg.host, cfg.port, cfg.email
            );
            return Ok(Some(cfg));
        }
        eprintln!("account: OHOL_AUTO_CONNECT set but creds incomplete — showing form");
    }

    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        "Open Life — Account",
        FB_W,
        FB_H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
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
        "account page: Tab field | Enter Connect | Esc skip-if-creds | F2 key/password | F3 Settings | type to edit"
    );

    while window.is_open() {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();

        // P5#39 Settings overlay from Account (F3)
        if app.screen.is_settings() {
            match handle_settings_keys(&window, app) {
                SettingsLoop::Left => {}
                SettingsLoop::Continue => {
                    app.settings.draw(&mut fb, Some(&hud));
                    rgba_to_u32(&fb.pixels, &mut buf);
                    let title = if app.settings.show_fps {
                        format!(
                            "Open Life — Settings ({:.0} FPS)  Esc=Back | SFX {:.0}% Music {:.0}%",
                            fps.fps(),
                            app.settings.sound_volume * 100.0,
                            app.settings.music_volume * 100.0
                        )
                    } else {
                        "Open Life — Settings  Esc=Back".into()
                    };
                    window.set_title(&title);
                    window.update_with_buffer(&buf, FB_W, FB_H)?;
                    fps.on_presented(dt);
                    continue;
                }
            }
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
        if window.is_key_pressed(Key::F3, KeyRepeat::No) {
            action = app.account.on_key(AccountKey::OpenSettings);
        }

        if window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = safe_mouse_pos(&window) {
                let cx = FB_W as f32 * 0.5;
                let cy = FB_H as f32 * 0.5 + 60.0;
                if (mx - cx).abs() < 70.0 && (my - cy).abs() < 24.0 {
                    action = AccountAction::Connect;
                }
            }
        }

        match action {
            AccountAction::Quit => return Ok(None),
            AccountAction::Connect => {
                let cfg = app.begin_connect();
                AccountPage::draw_loading(&mut fb, Some(&hud), &app.loading_msg);
                rgba_to_u32(&fb.pixels, &mut buf);
                let _ = window.update_with_buffer(&buf, FB_W, FB_H);
                window.set_title("Open Life — Loading");
                return Ok(Some(cfg));
            }
            AccountAction::OpenSettings => {
                let _ = app.enter_settings();
            }
            AccountAction::None => {}
        }

        if !window.is_open() {
            break;
        }
        if app.screen.is_settings() {
            continue;
        }

        app.account.draw(&mut fb, Some(&hud));
        rgba_to_u32(&fb.pixels, &mut buf);
        window.set_title(&format!(
            "Open Life — Account ({:.0} FPS)  [{}]",
            fps.fps(),
            ClientScreen::Account.as_str()
        ));
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);
    }
    Ok(None)
}

/// Outcome of one Settings-page input tick.
enum SettingsLoop {
    Continue,
    Left,
}

/// Process Settings keys; Back leaves Settings (Account or Playing).
fn handle_settings_keys(window: &Window, app: &mut ClientAppState) -> SettingsLoop {
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
    } else if window.is_key_pressed(Key::Escape, KeyRepeat::No)
        || window.is_key_pressed(Key::B, KeyRepeat::No)
    {
        key = SettingsKey::Escape;
    }

    match app.settings.on_key(key) {
        SettingsAction::None => SettingsLoop::Continue,
        SettingsAction::Applied => {
            app.settings.apply_runtime_globals();
            SettingsLoop::Continue
        }
        SettingsAction::Back => {
            app.leave_settings();
            SettingsLoop::Left
        }
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
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
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

    // Apply settings to SFX bank at session start.
    app.settings.apply_to_banks(Some(&mut session.sounds), None);

    while window.is_open() {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();

        // ── P5#39 Settings ───────────────────────────────────────────────
        if app.screen.is_settings() {
            match handle_settings_keys(&window, &mut app) {
                SettingsLoop::Left => {
                    app.apply_settings_to_banks(Some(&mut session.sounds), None);
                    // Apply zoom when leaving settings (also persisted by leave_settings).
                    scene.camera.zoom = app.settings.zoom.clamp(
                        ohol_headless::render::ZOOM_MIN,
                        ohol_headless::render::ZOOM_MAX,
                    );
                }
                SettingsLoop::Continue => {
                    // Live-preview zoom while adjusting the slider.
                    scene.camera.zoom = app.settings.zoom.clamp(
                        ohol_headless::render::ZOOM_MIN,
                        ohol_headless::render::ZOOM_MAX,
                    );
                    app.settings.draw(&mut fb, Some(&settings_hud));
                    let title = if app.settings.show_fps {
                        format!(
                            "Open Life — Settings ({:.0} FPS)  Esc=Back | Zoom {:.0} | SFX {:.0}%",
                            fps.fps(),
                            app.settings.zoom,
                            app.settings.sound_volume * 100.0,
                        )
                    } else {
                        format!(
                            "Open Life — Settings  Esc=Back | Zoom {:.0} | SFX {:.0}%",
                            app.settings.zoom,
                            app.settings.sound_volume * 100.0,
                        )
                    };
                    window.set_title(&title);
                    rgba_to_u32(&fb.pixels, &mut buf);
                    window.update_with_buffer(&buf, FB_W, FB_H)?;
                    fps.on_presented(dt);
                    continue;
                }
            }
            if app.screen.is_settings() {
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
            window.set_title(&format!(
                "Open Life — Death ({:.0} FPS)  R/Enter=Rebirth Esc=Quit | {}",
                fps.fps(),
                if last_status.is_empty() {
                    "you died"
                } else {
                    &last_status
                }
            ));
            rgba_to_u32(&fb.pixels, &mut buf);
            window.update_with_buffer(&buf, FB_W, FB_H)?;
            fps.on_presented(dt);
            continue;
        }

        // Playing: F3 or Esc → Settings (close window to quit).
        if window.is_key_pressed(Key::F3, KeyRepeat::No)
            || window.is_key_pressed(Key::Escape, KeyRepeat::No)
        {
            let _ = app.enter_settings();
            continue;
        }

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

        for _ in 0..48 {
            match session.poll_event() {
                Ok(_) => {}
                Err(_) => break,
            }
        }
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
            scene.camera.x = me.x as f32 + pan.0;
            scene.camera.y = me.y as f32 + pan.1;
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
                let age = me.age + me.age_rate * scene.time;
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
            // Don't path on SNAP button clicks when debug tools are on.
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
            &session.map,
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
        let title = if app.settings.show_fps {
            format!(
                "Open Life ({:.0} FPS) | {rx_label} | {status} | F3=Settings{dbg}",
                fps.fps(),
            )
        } else {
            format!("Open Life | {rx_label} | {status} | F3=Settings{dbg}")
        };
        window.set_title(&title);
        rgba_to_u32(&fb.pixels, &mut buf);
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);
    }
    Ok(())
}

/// Offline demo after a rejected login; banks already warmed by [`run_loading_boot`].
fn run_offline_from_boot(
    _sprites: SpriteBank,
    _anims: AnimBank,
    _content: ClientContent,
) -> anyhow::Result<()> {
    // Re-enter offline with its own progress path so the window/demo stays self-contained.
    // Meta banks were already indexed during boot (warm OS page cache).
    run_offline()
}

fn run_offline() -> anyhow::Result<()> {
    eprintln!("offline demo — content + sprites + animation + food/heat HUD + hitMap hover");
    // P5#36: soft-FB loading before world demo.
    let mut fb = Framebuffer::new(FB_W as u32, FB_H as u32);
    let mut window = Window::new(
        "Open Life Rust Client — Loading",
        FB_W,
        FB_H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    let mut buf = vec![0u32; FB_W * FB_H];

    let mut content = {
        let mut on_progress = |state: &LoadingState| {
            let _ = present_loading(&mut window, &mut fb, &mut buf, state);
        };
        ClientContent::load_default_locations_with_progress(Some(&mut on_progress))
            .unwrap_or_default()
    };
    let root = content
        .root
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let (mut sprites, mut anims, mut scene) =
        load_graphics_with_progress(&root, false, &mut window, &mut fb, &mut buf)?;
    // P3#19: PE eyeEmot placement needs Eyes/Mouth tags + mainEyesOffset
    content.setup_eyes_and_mouth(|sid| {
        let m = sprites.ensure_meta(sid);
        Some(m.tag.clone())
    });
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
    if let Some(pu) = parse_pu_line(
        "1 19 0 0 0 0 33 0 0 0 -1 0.5 1 0 10 10 20.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
    ) {
        world.apply_pu(&pu);
        world.set_our_id(1);
    }
    // Preload person + stone sprites from content defs
    let mut preload = vec![19, 33, 144];
    if let Some(def) = content.get(19) {
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

    scene.camera = Camera {
        x: 10.0,
        y: 10.0,
        zoom: 36.0,
    };
    // Offline demo vitals so food/heat chrome is visible without a server.
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
    // Window / fb / buf already open from loading screen — enter offline loop.
    let mut last = Instant::now();
    let mut hover = HoverPick::default();
    let mut fps = FpsMeter::new("offline");
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last.elapsed().as_secs_f32().min(0.05);
        last = Instant::now();
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
        // Offline: hitMap hover (no session actions).
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
        let hit = if hover.hit_map { "hit" } else { "tile" };
        window.set_title(&format!(
            "Open Life (offline) — {:.0} FPS | zoom {:.0} | ({},{}) id={} [{}]",
            fps.fps(),
            scene.camera.zoom,
            hover.tile.0,
            hover.tile.1,
            hover.object_id,
            hit
        ));
        let saved_hl = scene.highlight_tile.take();
        scene.draw(
            &mut fb,
            &map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            dt,
        );
        scene.highlight_tile = saved_hl;
        draw_hover_outline(&mut fb, &scene.camera, hover);
        rgba_to_u32(&fb.pixels, &mut buf);
        window.update_with_buffer(&buf, FB_W, FB_H)?;
        fps.on_presented(dt);
    }
    Ok(())
}

fn log_status(last: &mut String, msg: &str) {
    eprintln!("{msg}");
    *last = msg.chars().take(48).collect();
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
    for (i, px) in out.iter_mut().enumerate() {
        let o = i * 4;
        if o + 3 < rgba.len() {
            let r = rgba[o] as u32;
            let g = rgba[o + 1] as u32;
            let b = rgba[o + 2] as u32;
            // minifb 0x00RRGGBB
            *px = (r << 16) | (g << 8) | b;
        }
    }
}
