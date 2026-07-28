//! P5#37 — Account page (C++ `ExistingAccountPage` stand-in).
//!
//! Soft-FB form: email + account key / password + Connect.
//! Prefills from env / `.env` (`OHOL_*`). Headless CLI path stays on
//! `SessionConfig` + flags in `main.rs` and is unchanged.
//!
//! // C++: ExistingAccountPage email / accountKey fields + login button

use std::time::Duration;

use crate::hud::{draw_pencil_string, pencil_string_width, HudSprites, PencilFontAtlas};
use crate::render::Framebuffer;
use crate::session::SessionConfig;

/// Top-level graphical client screen graph (C-GAME / P5 product pages).
///
/// Shared by Account (P5#37), Loading (P5#36), Death (P5#38), Settings (P5#39).
/// Death summary payload lives on [`ClientAppState::death`], not the enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientScreen {
    Account,
    Loading,
    Playing,
    /// Death / rebirth (P5#38) — summary in `ClientAppState::death`.
    Death,
    /// Settings (P5#39) — options in `ClientAppState::settings`.
    Settings,
}

impl ClientScreen {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Loading => "loading",
            Self::Playing => "playing",
            Self::Death => "death",
            Self::Settings => "settings",
        }
    }

    pub fn is_account(self) -> bool {
        matches!(self, Self::Account)
    }

    pub fn is_loading(self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_playing(self) -> bool {
        matches!(self, Self::Playing)
    }

    pub fn is_death(self) -> bool {
        matches!(self, Self::Death)
    }

    pub fn is_settings(self) -> bool {
        matches!(self, Self::Settings)
    }

    /// Historical stub check — all product screens are live after P5#39.
    pub fn is_stub(self) -> bool {
        false
    }
}

/// Which field has keyboard focus on the account form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountFocus {
    Email,
    /// Account key **or** password (single secret line; see [`AccountPage::secret_mode`]).
    Secret,
    Connect,
}

/// Whether the secret field maps to account key or password on Connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretMode {
    AccountKey,
    Password,
}

/// Result of handling input on the account page for one frame / key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountAction {
    /// No transition.
    None,
    /// User wants to leave the client (Esc with no usable creds).
    Quit,
    /// Build config and start login (Enter / Connect / Esc-with-creds).
    Connect,
    /// Open Settings page (F3).
    OpenSettings,
}

/// Editable account form state + soft-FB layout.
#[derive(Debug, Clone)]
pub struct AccountPage {
    pub email: String,
    /// Account key **or** password text shown in the secret field.
    pub secret: String,
    pub secret_mode: SecretMode,
    /// Host / port carried into [`SessionConfig`] (from env; not edited on form).
    pub host: String,
    pub port: u16,
    pub tutorial_number: i32,
    pub reconnect: bool,
    pub pad_email_to_80: bool,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    /// Fallback when secret is account-key mode (password still required on wire).
    pub password_fallback: String,
    /// Fallback when secret is password mode.
    pub account_key_fallback: String,
    pub focus: AccountFocus,
    /// Status / error line under the form.
    pub status: String,
    /// Blink phase for caret (seconds accumulator).
    pub caret_t: f32,
    /// Cursor index within the focused text field (char index).
    pub caret: usize,
}

impl Default for AccountPage {
    fn default() -> Self {
        Self {
            email: String::new(),
            secret: String::new(),
            secret_mode: SecretMode::AccountKey,
            host: "127.0.0.1".into(),
            port: 8005,
            tutorial_number: 0,
            reconnect: false,
            pad_email_to_80: true,
            read_timeout: Duration::from_millis(30),
            write_timeout: Duration::from_secs(5),
            password_fallback: "x".into(),
            account_key_fallback: String::new(),
            focus: AccountFocus::Email,
            status: String::new(),
            caret_t: 0.0,
            caret: 0,
        }
    }
}

impl AccountPage {
    /// Prefill from process environment (`OHOL_HOST`, `OHOL_PORT`, `OHOL_EMAIL`,
    /// `OHOL_PASSWORD`, `OHOL_ACCOUNT_KEY`). Caller should `dotenvy::dotenv()` first.
    pub fn from_env() -> Self {
        Self::from_env_map(|k| std::env::var(k).ok())
    }

    /// Testable prefill: `get("OHOL_EMAIL")` etc.
    pub fn from_env_map<F>(mut get: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut p = Self::default();
        if let Some(h) = get("OHOL_HOST") {
            if !h.is_empty() {
                p.host = h;
            }
        }
        if let Some(port_s) = get("OHOL_PORT") {
            if let Ok(n) = port_s.parse::<u16>() {
                p.port = n;
            }
        }
        if let Some(e) = get("OHOL_EMAIL") {
            p.email = e;
        }
        let pw = get("OHOL_PASSWORD").unwrap_or_default();
        let key = get("OHOL_ACCOUNT_KEY").unwrap_or_default();
        p.password_fallback = if pw.is_empty() {
            "x".into()
        } else {
            pw.clone()
        };
        p.account_key_fallback = key.clone();
        // Prefer account key in the secret field when present (C++ primary path).
        if !key.is_empty() {
            p.secret = key;
            p.secret_mode = SecretMode::AccountKey;
        } else if !pw.is_empty() && pw != "x" {
            p.secret = pw;
            p.secret_mode = SecretMode::Password;
        } else {
            p.secret = String::new();
            p.secret_mode = SecretMode::AccountKey;
        }
        p.caret = p.email.chars().count();
        p.focus = AccountFocus::Email;
        if p.has_usable_creds() {
            p.status = "Enter=Connect  Esc=skip  Tab=field  F3=Settings".into();
        } else {
            p.status = "Enter email + key/password  Tab=field  F3=Settings".into();
        }
        p
    }

    /// True when the form has enough to attempt a login (email and/or secret).
    pub fn has_usable_creds(&self) -> bool {
        self.creds_present_for_skip()
    }

    /// Creds present for Esc-skip: any non-empty email or secret (typically after `.env` prefill).
    pub fn creds_present_for_skip(&self) -> bool {
        !self.email.trim().is_empty() || !self.secret.trim().is_empty()
    }

    /// Build [`SessionConfig`] for `ClientSession::connect` / `connect_and_login`.
    pub fn build_session_config(&self) -> SessionConfig {
        let email = if self.email.trim().is_empty() {
            "blank_email".into()
        } else {
            self.email.trim().to_string()
        };
        let (password, account_key) = match self.secret_mode {
            SecretMode::AccountKey => {
                let key = if self.secret.trim().is_empty() {
                    self.account_key_fallback.clone()
                } else {
                    self.secret.trim().to_string()
                };
                let pw = self.password_fallback.clone();
                (pw, key)
            }
            SecretMode::Password => {
                let pw = if self.secret.trim().is_empty() {
                    self.password_fallback.clone()
                } else {
                    self.secret.trim().to_string()
                };
                let key = self.account_key_fallback.clone();
                (pw, key)
            }
        };
        // Connect/login must use a multi-second timeout. Soft-FB play uses a short
        // poll timeout (often 30ms) on AccountPage — that is for post-login polls only
        // (`ClientSession::set_read_timeout` after ACCEPTED), not for TCP connect.
        let connect_read = self.read_timeout.max(Duration::from_secs(8));
        let connect_write = self.write_timeout.max(Duration::from_secs(5));
        SessionConfig {
            host: self.host.clone(),
            port: self.port,
            email,
            password,
            account_key,
            tutorial_number: self.tutorial_number,
            reconnect: self.reconnect,
            pad_email_to_80: self.pad_email_to_80,
            read_timeout: connect_read,
            write_timeout: connect_write,
            ..SessionConfig::default()
        }
    }

    fn focused_text(&self) -> Option<&str> {
        match self.focus {
            AccountFocus::Email => Some(self.email.as_str()),
            AccountFocus::Secret => Some(self.secret.as_str()),
            AccountFocus::Connect => None,
        }
    }

    fn clamp_caret(&mut self) {
        let len = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
        if self.caret > len {
            self.caret = len;
        }
    }

    /// Cycle Email → Secret → Connect → Email.
    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            AccountFocus::Email => AccountFocus::Secret,
            AccountFocus::Secret => AccountFocus::Connect,
            AccountFocus::Connect => AccountFocus::Email,
        };
        self.caret = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
    }

    /// Cycle reverse.
    pub fn focus_prev(&mut self) {
        self.focus = match self.focus {
            AccountFocus::Email => AccountFocus::Connect,
            AccountFocus::Secret => AccountFocus::Email,
            AccountFocus::Connect => AccountFocus::Secret,
        };
        self.caret = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
    }

    /// Toggle secret field between account-key and password interpretation.
    pub fn toggle_secret_mode(&mut self) {
        self.secret_mode = match self.secret_mode {
            SecretMode::AccountKey => SecretMode::Password,
            SecretMode::Password => SecretMode::AccountKey,
        };
    }

    /// Insert a typed character into the focused field (ignores control chars).
    pub fn type_char(&mut self, ch: char) {
        if ch.is_control() || ch == '\u{7f}' {
            return;
        }
        // Soft limit so soft-FB field doesn't explode.
        const MAX_FIELD: usize = 128;
        if matches!(self.focus, AccountFocus::Connect) {
            return;
        }
        let caret = self.caret;
        let text = match self.focus {
            AccountFocus::Email => &mut self.email,
            AccountFocus::Secret => &mut self.secret,
            AccountFocus::Connect => return,
        };
        if text.chars().count() >= MAX_FIELD {
            return;
        }
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        chars.insert(i, ch);
        *text = chars.into_iter().collect();
        self.caret = i + 1;
    }

    pub fn backspace(&mut self) {
        if matches!(self.focus, AccountFocus::Connect) {
            return;
        }
        let caret = self.caret;
        if caret == 0 {
            return;
        }
        let text = match self.focus {
            AccountFocus::Email => &mut self.email,
            AccountFocus::Secret => &mut self.secret,
            AccountFocus::Connect => return,
        };
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        if i > 0 {
            chars.remove(i - 1);
            *text = chars.into_iter().collect();
            self.caret = i - 1;
        }
    }

    pub fn delete_forward(&mut self) {
        if matches!(self.focus, AccountFocus::Connect) {
            return;
        }
        let caret = self.caret;
        let text = match self.focus {
            AccountFocus::Email => &mut self.email,
            AccountFocus::Secret => &mut self.secret,
            AccountFocus::Connect => return,
        };
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        if i < chars.len() {
            chars.remove(i);
            *text = chars.into_iter().collect();
        }
    }

    /// High-level key: used by tests and by the graphical client.
    pub fn on_key(&mut self, key: AccountKey) -> AccountAction {
        match key {
            AccountKey::Tab { shift } => {
                if shift {
                    self.focus_prev();
                } else {
                    self.focus_next();
                }
                AccountAction::None
            }
            AccountKey::Enter => {
                if self.focus == AccountFocus::Connect || self.focus != AccountFocus::Connect {
                    // Enter always Connect (task: Enter=Connect).
                    AccountAction::Connect
                } else {
                    AccountAction::Connect
                }
            }
            AccountKey::Escape => {
                if self.creds_present_for_skip() {
                    AccountAction::Connect
                } else {
                    AccountAction::Quit
                }
            }
            AccountKey::Backspace => {
                self.backspace();
                AccountAction::None
            }
            AccountKey::Delete => {
                self.delete_forward();
                AccountAction::None
            }
            AccountKey::Left => {
                if self.caret > 0 {
                    self.caret -= 1;
                }
                AccountAction::None
            }
            AccountKey::Right => {
                let len = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
                if self.caret < len {
                    self.caret += 1;
                }
                AccountAction::None
            }
            AccountKey::Home => {
                self.caret = 0;
                AccountAction::None
            }
            AccountKey::End => {
                self.caret = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
                AccountAction::None
            }
            AccountKey::ToggleSecretMode => {
                self.toggle_secret_mode();
                AccountAction::None
            }
            AccountKey::OpenSettings => AccountAction::OpenSettings,
            AccountKey::Char(c) => {
                self.type_char(c);
                AccountAction::None
            }
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.caret_t += dt;
        if self.caret_t > 1.0 {
            self.caret_t -= 1.0;
        }
        self.clamp_caret();
    }

    /// Soft-FB draw (pencilFont TGA via `sprites` when present, else 5×7).
    pub fn draw(&self, fb: &mut Framebuffer, sprites: Option<&HudSprites>) {
        let w = fb.width as f32;
        let h = fb.height as f32;
        // Warm parchment background (product-page feel).
        fb.clear([210, 200, 175, 255]);
        // Card
        let card_w = 420.0f32;
        let card_h = 260.0f32;
        let card_x = ((w - card_w) * 0.5).round() as i32;
        let card_y = ((h - card_h) * 0.5 - 20.0).round() as i32;
        fb.fill_rect(card_x - 4, card_y - 4, (card_w as i32) + 8, (card_h as i32) + 8, [
            80, 70, 50, 255,
        ]);
        fb.fill_rect(card_x, card_y, card_w as i32, card_h as i32, [235, 228, 205, 255]);

        let ink = [20, 18, 14, 255];
        let ink_dim = [90, 80, 60, 255];
        let focus_bg = [255, 250, 220, 255];
        let field_bg = [250, 245, 230, 255];
        let btn_bg = [90, 120, 70, 255];
        let btn_focus = [120, 160, 90, 255];
        let scale = 1.6f32;

        let title_x = w * 0.5;
        let title_y = card_y as f32 + 28.0;
        draw_text(fb, sprites, "OPEN LIFE", title_x, title_y, scale * 1.2, ink, true);
        draw_text(
            fb,
            sprites,
            "Account",
            title_x,
            title_y + 22.0,
            scale * 0.9,
            ink_dim,
            true,
        );

        let label_x = card_x as f32 + 28.0;
        let field_x = card_x as f32 + 28.0;
        let field_w = card_w - 56.0;
        let field_h = 28.0;

        // Email
        let email_y = card_y as f32 + 80.0;
        draw_text(fb, sprites, "Email", label_x, email_y - 14.0, scale * 0.75, ink_dim, false);
        let email_focused = self.focus == AccountFocus::Email;
        fb.fill_rect(
            field_x as i32,
            email_y as i32,
            field_w as i32,
            field_h as i32,
            if email_focused { focus_bg } else { field_bg },
        );
        // border
        draw_field_border(fb, field_x, email_y, field_w, field_h, email_focused);
        let email_show = if self.email.is_empty() && !email_focused {
            ""
        } else {
            &self.email
        };
        draw_text(
            fb,
            sprites,
            email_show,
            field_x + 8.0,
            email_y + field_h * 0.5,
            scale * 0.85,
            ink,
            false,
        );
        if email_focused {
            draw_caret(
                fb,
                sprites,
                email_show,
                field_x + 8.0,
                email_y + field_h * 0.5,
                scale * 0.85,
                self.caret,
                self.caret_t,
            );
        }

        // Secret
        let secret_y = email_y + 52.0;
        let secret_label = match self.secret_mode {
            SecretMode::AccountKey => "Account key  (F2=password)",
            SecretMode::Password => "Password  (F2=account key)",
        };
        draw_text(
            fb,
            sprites,
            secret_label,
            label_x,
            secret_y - 14.0,
            scale * 0.75,
            ink_dim,
            false,
        );
        let secret_focused = self.focus == AccountFocus::Secret;
        fb.fill_rect(
            field_x as i32,
            secret_y as i32,
            field_w as i32,
            field_h as i32,
            if secret_focused { focus_bg } else { field_bg },
        );
        draw_field_border(fb, field_x, secret_y, field_w, field_h, secret_focused);
        let secret_display = mask_secret(&self.secret, self.secret_mode);
        draw_text(
            fb,
            sprites,
            &secret_display,
            field_x + 8.0,
            secret_y + field_h * 0.5,
            scale * 0.85,
            ink,
            false,
        );
        if secret_focused {
            draw_caret(
                fb,
                sprites,
                &secret_display,
                field_x + 8.0,
                secret_y + field_h * 0.5,
                scale * 0.85,
                self.caret,
                self.caret_t,
            );
        }

        // Connect button
        let btn_w = 140.0f32;
        let btn_h = 34.0f32;
        let btn_x = title_x - btn_w * 0.5;
        let btn_y = secret_y + 48.0;
        let btn_focused = self.focus == AccountFocus::Connect;
        fb.fill_rect(
            btn_x as i32,
            btn_y as i32,
            btn_w as i32,
            btn_h as i32,
            if btn_focused { btn_focus } else { btn_bg },
        );
        draw_text(
            fb,
            sprites,
            "Connect",
            title_x,
            btn_y + btn_h * 0.5,
            scale * 0.95,
            [250, 250, 245, 255],
            true,
        );

        // Host line + status
        let host_line = format!("{}:{}", self.host, self.port);
        draw_text(
            fb,
            sprites,
            &host_line,
            title_x,
            btn_y + 48.0,
            scale * 0.7,
            ink_dim,
            true,
        );
        if !self.status.is_empty() {
            draw_text(
                fb,
                sprites,
                &self.status,
                title_x,
                card_y as f32 + card_h - 18.0,
                scale * 0.7,
                ink_dim,
                true,
            );
        }
    }

    /// Loading screen without full stage state (connect message only).
    ///
    /// Prefer [`crate::load_progress::draw_loading_progress`] when a
    /// [`crate::load_progress::LoadingState`] is available (P5#36).
    pub fn draw_loading(fb: &mut Framebuffer, sprites: Option<&HudSprites>, msg: &str) {
        // Soft bar at 0% with connect detail — full stages use load_progress draw.
        let mut state = crate::load_progress::LoadingState::for_stage(
            crate::load_progress::LoadStage::Content,
            0.0,
            if msg.is_empty() { None } else { Some(msg) },
        );
        if !msg.is_empty() {
            state.label = msg.to_string();
        }
        crate::load_progress::draw_loading_progress(fb, &state);
        let _ = sprites; // reserved if pencil atlas desired later
    }

    /// Draw P5#36 loading bar + label from a live [`LoadingState`].
    pub fn draw_loading_state(
        fb: &mut Framebuffer,
        state: &crate::load_progress::LoadingState,
    ) {
        crate::load_progress::draw_loading_progress(fb, state);
    }

    /// Settings page (P5#39) — soft-FB mute toggles + host/port.
    pub fn draw_settings(
        fb: &mut Framebuffer,
        sprites: Option<&HudSprites>,
        page: &crate::settings_page::SettingsPage,
    ) {
        page.draw(fb, sprites);
    }
}



/// Abstract keys so the lib stays free of minifb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKey {
    Char(char),
    Tab { shift: bool },
    Enter,
    Escape,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    /// F2 — toggle account key vs password.
    ToggleSecretMode,
    /// F3 — open Settings (P5#39).
    OpenSettings,
}

fn mask_secret(secret: &str, mode: SecretMode) -> String {
    match mode {
        SecretMode::Password => {
            if secret.is_empty() {
                String::new()
            } else {
                "*".repeat(secret.chars().count().min(64))
            }
        }
        SecretMode::AccountKey => secret.to_string(),
    }
}

fn draw_field_border(fb: &mut Framebuffer, x: f32, y: f32, w: f32, h: f32, focused: bool) {
    let c = if focused {
        [60, 100, 40, 255]
    } else {
        [120, 110, 90, 255]
    };
    let xi = x as i32;
    let yi = y as i32;
    let wi = w as i32;
    let hi = h as i32;
    fb.fill_rect(xi, yi, wi, 1, c);
    fb.fill_rect(xi, yi + hi - 1, wi, 1, c);
    fb.fill_rect(xi, yi, 1, hi, c);
    fb.fill_rect(xi + wi - 1, yi, 1, hi, c);
}

fn draw_text(
    fb: &mut Framebuffer,
    sprites: Option<&HudSprites>,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    rgba: [u8; 4],
    center: bool,
) {
    if text.is_empty() {
        return;
    }
    if let Some(hud) = sprites {
        if let Some(font) = hud.pencil_font.as_ref() {
            font.draw_string(fb, text, x, y, scale, rgba, center);
            return;
        }
        hud.draw_hud_text(fb, text, x, y, scale, rgba, center, false);
        return;
    }
    draw_pencil_string(fb, text, x, y, scale, rgba, center);
}

fn measure_text(sprites: Option<&HudSprites>, text: &str, scale: f32) -> f32 {
    if let Some(hud) = sprites {
        if let Some(font) = hud.pencil_font.as_ref() {
            return font.measure(text, scale);
        }
    }
    pencil_string_width(text, scale)
}

fn draw_caret(
    fb: &mut Framebuffer,
    sprites: Option<&HudSprites>,
    text: &str,
    field_left: f32,
    mid_y: f32,
    scale: f32,
    caret: usize,
    caret_t: f32,
) {
    // Blink ~2 Hz
    if caret_t > 0.5 {
        return;
    }
    let prefix: String = text.chars().take(caret).collect();
    let w = measure_text(sprites, &prefix, scale);
    let cx = (field_left + w).round() as i32;
    let h = (10.0 * scale).round().max(8.0) as i32;
    let cy = (mid_y - h as f32 * 0.5).round() as i32;
    fb.fill_rect(cx, cy, 2, h, [20, 18, 14, 255]);
}

/// Screen graph helper: Account / Loading / Playing / Death / Settings transitions.
#[derive(Debug, Clone)]
pub struct ClientAppState {
    pub screen: ClientScreen,
    pub account: AccountPage,
    pub loading_msg: String,
    /// Last death summary when `screen == Death` (P5#38).
    pub death: Option<crate::client_screen::DeathSummary>,
    /// Settings page state (P5#39) — volumes, mutes, show FPS, host/port display.
    pub settings: crate::settings_page::SettingsPage,
    /// Screen restored when leaving Settings (Account or Playing).
    pub settings_return: ClientScreen,
}

impl Default for ClientAppState {
    fn default() -> Self {
        Self {
            screen: ClientScreen::Account,
            account: AccountPage::default(),
            loading_msg: String::new(),
            death: None,
            settings: crate::settings_page::SettingsPage::default(),
            settings_return: ClientScreen::Account,
        }
    }
}

impl ClientAppState {
    pub fn from_env() -> Self {
        let account = AccountPage::from_env();
        let mut settings = crate::settings_page::SettingsPage::from_env();
        settings.sync_endpoint_from(&account.host, account.port, &account.email);
        settings.apply_runtime_globals();
        Self {
            screen: ClientScreen::Account,
            account,
            loading_msg: String::new(),
            death: None,
            settings,
            settings_return: ClientScreen::Account,
        }
    }

    pub fn begin_connect(&mut self) -> SessionConfig {
        self.screen = ClientScreen::Loading;
        self.loading_msg = format!(
            "Connecting {}:{}…",
            self.account.host, self.account.port
        );
        self.account.status = "Connecting…".into();
        self.death = None;
        self.account.build_session_config()
    }

    pub fn enter_playing(&mut self) {
        self.screen = ClientScreen::Playing;
        self.loading_msg.clear();
        self.death = None;
    }

    pub fn back_to_account(&mut self, err: impl Into<String>) {
        self.screen = ClientScreen::Account;
        self.account.status = err.into();
        self.loading_msg.clear();
        self.death = None;
    }

    /// Enter Settings from Account or Playing (P5#39).
    pub fn enter_settings(&mut self) -> bool {
        if self.screen == ClientScreen::Settings {
            return false;
        }
        if !matches!(self.screen, ClientScreen::Account | ClientScreen::Playing) {
            return false;
        }
        self.settings_return = self.screen;
        self.settings.sync_endpoint_from(
            &self.account.host,
            self.account.port,
            &self.account.email,
        );
        self.settings.focus = crate::settings_page::SettingsFocus::SoundVolume;
        self.settings.status = "Tab=row  Left/Right=adjust  +/-=zoom  Esc=Back".into();
        self.settings.apply_runtime_globals();
        self.screen = ClientScreen::Settings;
        true
    }

    /// Leave Settings → prior product screen (clamp + best-effort persist).
    pub fn leave_settings(&mut self) {
        if self.screen != ClientScreen::Settings {
            return;
        }
        self.settings.clamp();
        self.settings.apply_runtime_globals();
        let _ = self.settings.save_default();
        self.screen = match self.settings_return {
            ClientScreen::Playing => ClientScreen::Playing,
            ClientScreen::Account => ClientScreen::Account,
            _ => ClientScreen::Account,
        };
    }

    /// Apply volume/mute to optional banks (P5#39).
    pub fn apply_settings_to_banks(
        &self,
        sounds: Option<&mut crate::sound_bank::SoundBank>,
        music: Option<&mut crate::music_bank::MusicBank>,
    ) {
        self.settings.apply_to_banks(sounds, music);
    }
}






// Silence unused import when PencilFontAtlas is only referenced via HudSprites paths.
#[allow(dead_code)]
fn _pencil_font_type_anchor(_: &PencilFontAtlas) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_screen_graph_account_loading_playing() {
        assert_eq!(ClientScreen::Account.as_str(), "account");
        assert_eq!(ClientScreen::Loading.as_str(), "loading");
        assert_eq!(ClientScreen::Playing.as_str(), "playing");
        assert_eq!(ClientScreen::Death.as_str(), "death");
        assert!(ClientScreen::Death.is_death());
        assert!(!ClientScreen::Death.is_stub()); // P5#38 live
        assert!(ClientScreen::Settings.is_settings());
        assert!(!ClientScreen::Settings.is_stub()); // P5#39 live
        assert!(ClientScreen::Account.is_account());
        assert!(ClientScreen::Loading.is_loading());
        assert!(ClientScreen::Playing.is_playing());
        assert!(!ClientScreen::Playing.is_stub());
    }

    #[test]
    fn from_env_map_prefers_account_key() {
        let page = AccountPage::from_env_map(|k| match k {
            "OHOL_HOST" => Some("10.0.0.2".into()),
            "OHOL_PORT" => Some("9001".into()),
            "OHOL_EMAIL" => Some("a@b.c".into()),
            "OHOL_PASSWORD" => Some("secretpw".into()),
            "OHOL_ACCOUNT_KEY" => Some("ABCD-EFGH".into()),
            _ => None,
        });
        assert_eq!(page.host, "10.0.0.2");
        assert_eq!(page.port, 9001);
        assert_eq!(page.email, "a@b.c");
        assert_eq!(page.secret, "ABCD-EFGH");
        assert_eq!(page.secret_mode, SecretMode::AccountKey);
        assert_eq!(page.password_fallback, "secretpw");
        assert!(page.creds_present_for_skip());
    }

    #[test]
    fn from_env_map_password_when_no_key() {
        let page = AccountPage::from_env_map(|k| match k {
            "OHOL_EMAIL" => Some("x@y.z".into()),
            "OHOL_PASSWORD" => Some("mypass".into()),
            "OHOL_ACCOUNT_KEY" => Some(String::new()),
            _ => None,
        });
        assert_eq!(page.secret, "mypass");
        assert_eq!(page.secret_mode, SecretMode::Password);
    }

    #[test]
    fn build_session_config_account_key_mode() {
        let mut page = AccountPage::default();
        page.email = "player@ohol".into();
        page.secret = "KEY-1234".into();
        page.secret_mode = SecretMode::AccountKey;
        page.password_fallback = "x".into();
        page.host = "127.0.0.1".into();
        page.port = 8005;
        let cfg = page.build_session_config();
        assert_eq!(cfg.email, "player@ohol");
        assert_eq!(cfg.account_key, "KEY-1234");
        assert_eq!(cfg.password, "x");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8005);
        assert!(cfg.pad_email_to_80);
    }

    #[test]
    fn build_session_config_password_mode() {
        let mut page = AccountPage::default();
        page.email = "p@q".into();
        page.secret = "hunter2".into();
        page.secret_mode = SecretMode::Password;
        page.account_key_fallback = "FALLBACK".into();
        let cfg = page.build_session_config();
        assert_eq!(cfg.password, "hunter2");
        assert_eq!(cfg.account_key, "FALLBACK");
    }

    #[test]
    fn type_and_focus_cycle() {
        let mut page = AccountPage::default();
        assert_eq!(page.focus, AccountFocus::Email);
        page.type_char('a');
        page.type_char('@');
        page.type_char('b');
        assert_eq!(page.email, "a@b");
        page.on_key(AccountKey::Tab { shift: false });
        assert_eq!(page.focus, AccountFocus::Secret);
        page.type_char('K');
        assert_eq!(page.secret, "K");
        page.on_key(AccountKey::Tab { shift: false });
        assert_eq!(page.focus, AccountFocus::Connect);
        page.on_key(AccountKey::Tab { shift: false });
        assert_eq!(page.focus, AccountFocus::Email);
    }

    #[test]
    fn enter_connects_escape_skips_with_creds() {
        let mut page = AccountPage::default();
        // No creds → Esc quits
        assert_eq!(page.on_key(AccountKey::Escape), AccountAction::Quit);
        page.email = "e@x".into();
        assert_eq!(page.on_key(AccountKey::Escape), AccountAction::Connect);
        assert_eq!(page.on_key(AccountKey::Enter), AccountAction::Connect);
    }

    #[test]
    fn backspace_and_caret() {
        let mut page = AccountPage::default();
        page.email = "ab".into();
        page.caret = 2;
        page.backspace();
        assert_eq!(page.email, "a");
        assert_eq!(page.caret, 1);
        page.type_char('c');
        assert_eq!(page.email, "ac");
    }

    #[test]
    fn app_state_transitions() {
        let mut app = ClientAppState::default();
        assert_eq!(app.screen, ClientScreen::Account);
        app.account.email = "t@t".into();
        app.account.secret = "k".into();
        let cfg = app.begin_connect();
        assert_eq!(app.screen, ClientScreen::Loading);
        assert_eq!(cfg.email, "t@t");
        assert_eq!(cfg.account_key, "k");
        app.enter_playing();
        assert_eq!(app.screen, ClientScreen::Playing);
        app.back_to_account("login denied");
        assert_eq!(app.screen, ClientScreen::Account);
        assert_eq!(app.account.status, "login denied");
    }

    #[test]
    fn draw_account_page_marks_pixels() {
        let mut fb = Framebuffer::new(320, 240);
        let page = AccountPage {
            email: "a@b.c".into(),
            secret: "KEY".into(),
            ..AccountPage::default()
        };
        page.draw(&mut fb, None);
        let n = fb.count_non_color([210, 200, 175, 255]);
        assert!(n > 100, "expected form chrome, got {n} non-bg pixels");
    }

    #[test]
    fn draw_loading_and_settings() {
        let mut fb = Framebuffer::new(160, 120);
        AccountPage::draw_loading(&mut fb, None, "objects…");
        assert!(fb.count_non_color([40, 45, 50, 255]) > 10);
        let page = crate::settings_page::SettingsPage::default();
        AccountPage::draw_settings(&mut fb, None, &page);
        assert!(fb.count_non_color([45, 52, 62, 255]) > 5);
    }

    #[test]
    fn open_settings_key_and_app_transitions() {
        let mut page = AccountPage::default();
        assert_eq!(
            page.on_key(AccountKey::OpenSettings),
            AccountAction::OpenSettings
        );

        let mut app = ClientAppState::default();
        assert_eq!(app.screen, ClientScreen::Account);
        assert!(app.enter_settings());
        assert_eq!(app.screen, ClientScreen::Settings);
        assert_eq!(app.settings_return, ClientScreen::Account);
        app.settings.sound_volume = 0.2;
        app.settings.sound_muted = true;
        app.settings.apply_runtime_globals();
        assert!(crate::sound_bank::sfx_muted());
        app.leave_settings();
        assert_eq!(app.screen, ClientScreen::Account);
        assert!((app.settings.sound_volume - 0.2).abs() < 1e-5);

        app.enter_playing();
        assert!(app.enter_settings());
        assert_eq!(app.screen, ClientScreen::Settings);
        assert_eq!(app.settings_return, ClientScreen::Playing);
        app.leave_settings();
        assert_eq!(app.screen, ClientScreen::Playing);

        crate::sound_bank::set_sfx_muted(false);
        crate::sound_bank::set_music_muted(false);
    }
}
