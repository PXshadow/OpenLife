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

/// Local server always listed first in recent endpoints.
pub const LOCAL_SERVER_HOST: &str = "127.0.0.1";
pub const LOCAL_SERVER_PORT: u16 = 8005;
/// Max recent server slots shown (includes local as first).
pub const MAX_RECENT_SERVERS: usize = 5;
/// Persisted recent host:port list (cwd).
pub const RECENT_SERVERS_FILE: &str = "ohol_recent_servers.ini";

/// One server endpoint for recent list / connect.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerEndpoint {
    pub host: String,
    pub port: u16,
}

impl ServerEndpoint {
    pub fn local() -> Self {
        Self {
            host: LOCAL_SERVER_HOST.into(),
            port: LOCAL_SERVER_PORT,
        }
    }

    pub fn label(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn is_local(&self) -> bool {
        let h = self.host.trim().to_ascii_lowercase();
        (h == "127.0.0.1" || h == "localhost" || h == "::1") && self.port == LOCAL_SERVER_PORT
    }

    pub fn same_as(&self, host: &str, port: u16) -> bool {
        self.host.trim().eq_ignore_ascii_case(host.trim()) && self.port == port
    }
}

/// Which field has keyboard focus on the account form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountFocus {
    Email,
    /// Account key **or** password (single secret line; see [`AccountPage::secret_mode`]).
    Secret,
    Host,
    Port,
    /// Recent server slot 0..MAX_RECENT_SERVERS-1
    Recent(u8),
    Connect,
    /// Nested form only: save + return to Settings.
    Back,
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
    /// Build config and start login (Enter / Connect).
    Connect,
    /// Open Settings page (F3) — boot account only.
    OpenSettings,
    /// Nested from Settings: return to Settings (Esc / Back).
    Back,
    /// Nested: saved host/email without full reconnect.
    Saved,
}

/// Editable account form state + soft-FB layout.
#[derive(Debug, Clone)]
pub struct AccountPage {
    pub email: String,
    /// Account key **or** password text shown in the secret field.
    pub secret: String,
    pub secret_mode: SecretMode,
    /// Host / port carried into [`SessionConfig`] (editable on form).
    pub host: String,
    pub port: u16,
    /// Text buffer for port field (synced with [`Self::port`] on blur/apply).
    pub port_text: String,
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
    /// Recent servers: **local first**, then last-used (max [`MAX_RECENT_SERVERS`]).
    pub recent_servers: Vec<ServerEndpoint>,
    /// True when opened from Settings (Esc/Back returns to Settings, not open Settings).
    pub opened_from_settings: bool,
}

impl Default for AccountPage {
    fn default() -> Self {
        let mut p = Self {
            email: String::new(),
            secret: String::new(),
            secret_mode: SecretMode::AccountKey,
            host: LOCAL_SERVER_HOST.into(),
            port: LOCAL_SERVER_PORT,
            port_text: LOCAL_SERVER_PORT.to_string(),
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
            recent_servers: vec![ServerEndpoint::local()],
            opened_from_settings: false,
        };
        p.load_recent_servers();
        p
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
                p.port_text = n.to_string();
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
        p.load_recent_servers();
        p.remember_current_server(); // ensure env host appears in list
        if p.has_usable_creds() {
            p.status = "Enter=Connect  ·  Esc=Settings  ·  click a server to switch".into();
        } else {
            p.status = "Email + key/password  ·  pick a server  ·  Esc=Settings".into();
        }
        p
    }

    /// Parse port_text into `port` (fallback 8005).
    pub fn sync_port_from_text(&mut self) {
        let n = self
            .port_text
            .trim()
            .parse::<u16>()
            .unwrap_or(LOCAL_SERVER_PORT);
        self.port = if n == 0 { LOCAL_SERVER_PORT } else { n };
        self.port_text = self.port.to_string();
    }

    /// Apply host/port from a recent slot.
    pub fn apply_recent(&mut self, index: usize) {
        if let Some(ep) = self.recent_servers.get(index).cloned() {
            self.host = ep.host;
            self.port = ep.port;
            self.port_text = ep.port.to_string();
            self.status = format!("Server → {}:{}", self.host, self.port);
            // Move selected to “last used” (local always stays first).
            self.remember_current_server();
        }
    }

    /// Insert/update current host:port in recent list (local always index 0).
    pub fn remember_current_server(&mut self) {
        self.sync_port_from_text();
        let current = ServerEndpoint {
            host: self.host.trim().to_string(),
            port: self.port,
        };
        if current.host.is_empty() {
            return;
        }
        // Drop duplicates of current (except we'll re-insert).
        self.recent_servers
            .retain(|e| !e.same_as(&current.host, current.port) && !e.is_local());
        // Local first
        let mut out = vec![ServerEndpoint::local()];
        // If current is not local, it's the most recently used non-local.
        if !current.is_local() {
            out.push(current);
        }
        for e in self.recent_servers.drain(..) {
            if out.len() >= MAX_RECENT_SERVERS {
                break;
            }
            if e.is_local() {
                continue;
            }
            if out.iter().any(|x| x.same_as(&e.host, e.port)) {
                continue;
            }
            out.push(e);
        }
        self.recent_servers = out;
        let _ = self.save_recent_servers();
    }

    pub fn load_recent_servers(&mut self) {
        let path = std::path::Path::new(RECENT_SERVERS_FILE);
        let mut list = vec![ServerEndpoint::local()];
        if let Ok(text) = std::fs::read_to_string(path) {
            for raw in text.lines() {
                let line = raw.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let (host, port_s) = if let Some((h, p)) = line.split_once(':') {
                    (h.trim(), p.trim())
                } else if let Some((h, p)) = line.split_once('=') {
                    if h.trim().eq_ignore_ascii_case("server") {
                        // server=host:port
                        if let Some((hh, pp)) = p.trim().split_once(':') {
                            (hh.trim(), pp.trim())
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                let Ok(port) = port_s.parse::<u16>() else {
                    continue;
                };
                if host.is_empty() || port == 0 {
                    continue;
                }
                let ep = ServerEndpoint {
                    host: host.to_string(),
                    port,
                };
                if ep.is_local() {
                    continue;
                }
                if list.iter().any(|x| x.same_as(&ep.host, ep.port)) {
                    continue;
                }
                list.push(ep);
                if list.len() >= MAX_RECENT_SERVERS {
                    break;
                }
            }
        }
        self.recent_servers = list;
    }

    pub fn save_recent_servers(&self) -> std::io::Result<()> {
        let mut body = String::from("# Open Life recent servers (local first, then last used)\n");
        for ep in &self.recent_servers {
            body.push_str(&format!("{}:{}\n", ep.host, ep.port));
        }
        std::fs::write(RECENT_SERVERS_FILE, body)
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
        let port = self
            .port_text
            .trim()
            .parse::<u16>()
            .unwrap_or(self.port)
            .max(1);
        SessionConfig {
            host: self.host.trim().to_string(),
            port,
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
            AccountFocus::Host => Some(self.host.as_str()),
            AccountFocus::Port => Some(self.port_text.as_str()),
            AccountFocus::Connect | AccountFocus::Back | AccountFocus::Recent(_) => None,
        }
    }

    fn clamp_caret(&mut self) {
        let len = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
        if self.caret > len {
            self.caret = len;
        }
    }

    fn focus_order(&self) -> Vec<AccountFocus> {
        let mut v = vec![
            AccountFocus::Email,
            AccountFocus::Secret,
            AccountFocus::Host,
            AccountFocus::Port,
        ];
        for i in 0..self.recent_servers.len().min(MAX_RECENT_SERVERS) {
            v.push(AccountFocus::Recent(i as u8));
        }
        v.push(AccountFocus::Connect);
        if self.opened_from_settings {
            v.push(AccountFocus::Back);
        }
        v
    }

    /// Cycle fields.
    pub fn focus_next(&mut self) {
        if matches!(self.focus, AccountFocus::Port) {
            self.sync_port_from_text();
        }
        let order = self.focus_order();
        let i = order.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = order[(i + 1) % order.len()];
        self.caret = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
    }

    /// Cycle reverse.
    pub fn focus_prev(&mut self) {
        if matches!(self.focus, AccountFocus::Port) {
            self.sync_port_from_text();
        }
        let order = self.focus_order();
        let i = order.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = order[(i + order.len() - 1) % order.len()];
        self.caret = self.focused_text().map(|s| s.chars().count()).unwrap_or(0);
    }

    /// Toggle secret field between account-key and password interpretation.
    pub fn toggle_secret_mode(&mut self) {
        self.secret_mode = match self.secret_mode {
            SecretMode::AccountKey => SecretMode::Password,
            SecretMode::Password => SecretMode::AccountKey,
        };
    }

    fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            AccountFocus::Email => Some(&mut self.email),
            AccountFocus::Secret => Some(&mut self.secret),
            AccountFocus::Host => Some(&mut self.host),
            AccountFocus::Port => Some(&mut self.port_text),
            AccountFocus::Connect | AccountFocus::Back | AccountFocus::Recent(_) => None,
        }
    }

    /// Insert a typed character into the focused field (ignores control chars).
    pub fn type_char(&mut self, ch: char) {
        if ch.is_control() || ch == '\u{7f}' {
            return;
        }
        const MAX_FIELD: usize = 128;
        if matches!(self.focus, AccountFocus::Port) && !ch.is_ascii_digit() {
            return;
        }
        let caret = self.caret;
        let Some(text) = self.focused_text_mut() else {
            return;
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
        let caret = self.caret;
        if caret == 0 {
            return;
        }
        let Some(text) = self.focused_text_mut() else {
            return;
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
        let caret = self.caret;
        let Some(text) = self.focused_text_mut() else {
            return;
        };
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        if i < chars.len() {
            chars.remove(i);
            *text = chars.into_iter().collect();
        }
    }

    /// Activate focused control (Enter / click).
    fn activate_focus(&mut self) -> AccountAction {
        match self.focus {
            AccountFocus::Connect => {
                self.sync_port_from_text();
                self.remember_current_server();
                if self.opened_from_settings {
                    self.status = format!(
                        "Saved {}:{} — reconnect next boot / Connect from Account at start",
                        self.host, self.port
                    );
                    AccountAction::Saved
                } else {
                    AccountAction::Connect
                }
            }
            AccountFocus::Back => AccountAction::Back,
            AccountFocus::Recent(i) => {
                self.apply_recent(i as usize);
                AccountAction::None
            }
            AccountFocus::Email
            | AccountFocus::Secret
            | AccountFocus::Host
            | AccountFocus::Port => {
                // Enter on field → Connect / Save
                self.sync_port_from_text();
                self.remember_current_server();
                if self.opened_from_settings {
                    AccountAction::Saved
                } else {
                    AccountAction::Connect
                }
            }
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
            AccountKey::Enter => self.activate_focus(),
            AccountKey::Escape => {
                if self.opened_from_settings {
                    self.sync_port_from_text();
                    AccountAction::Back
                } else {
                    // Boot: Esc opens Settings (same as F3).
                    AccountAction::OpenSettings
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

    /// Soft-FB layout for mouse hit-tests (matches [`Self::draw`]).
    pub fn layout(fb_w: f32, fb_h: f32) -> AccountLayout {
        account_layout(fb_w, fb_h)
    }

    /// LMB press in soft-FB coords: fields, servers, Connect / Back / Settings.
    pub fn on_pointer_down(
        &mut self,
        mx: f32,
        my: f32,
        fb_w: f32,
        fb_h: f32,
        sprites: Option<&HudSprites>,
    ) -> AccountAction {
        let layout = Self::layout(fb_w, fb_h);
        let scale = 0.95f32;
        if layout.email.contains(mx, my) {
            self.focus = AccountFocus::Email;
            self.caret = caret_index_at_x(sprites, &self.email, layout.email.x + 8.0, mx, scale);
            self.clamp_caret();
            return AccountAction::None;
        }
        if layout.secret.contains(mx, my) {
            self.focus = AccountFocus::Secret;
            let display = mask_secret(&self.secret, self.secret_mode);
            self.caret = caret_index_at_x(sprites, &display, layout.secret.x + 8.0, mx, scale)
                .min(self.secret.chars().count());
            self.clamp_caret();
            return AccountAction::None;
        }
        if layout.host.contains(mx, my) {
            self.focus = AccountFocus::Host;
            self.caret = caret_index_at_x(sprites, &self.host, layout.host.x + 8.0, mx, scale);
            self.clamp_caret();
            return AccountAction::None;
        }
        if layout.port.contains(mx, my) {
            self.focus = AccountFocus::Port;
            self.caret =
                caret_index_at_x(sprites, &self.port_text, layout.port.x + 8.0, mx, scale);
            self.clamp_caret();
            return AccountAction::None;
        }
        for (i, r) in layout.recent.iter().enumerate() {
            if r.contains(mx, my) {
                self.focus = AccountFocus::Recent(i as u8);
                self.apply_recent(i);
                return AccountAction::None;
            }
        }
        if layout.connect.contains(mx, my) {
            self.focus = AccountFocus::Connect;
            return self.activate_focus();
        }
        if self.opened_from_settings {
            if let Some(back) = layout.back {
                if back.contains(mx, my) {
                    self.focus = AccountFocus::Back;
                    return AccountAction::Back;
                }
            }
        } else if layout.settings.contains(mx, my) {
            return AccountAction::OpenSettings;
        }
        AccountAction::None
    }

    /// Soft-FB draw — solid page (boot Account).
    pub fn draw(&self, fb: &mut Framebuffer, sprites: Option<&HudSprites>) {
        self.draw_inner(fb, sprites, true);
    }

    /// Glass overlay (Settings → Account over world).
    pub fn draw_overlay(&self, fb: &mut Framebuffer, sprites: Option<&HudSprites>) {
        self.draw_inner(fb, sprites, false);
    }

    fn draw_inner(&self, fb: &mut Framebuffer, sprites: Option<&HudSprites>, solid: bool) {
        use crate::ui_font::draw_ui_text;
        let _ = sprites;
        let w = fb.width as i32;
        let h = fb.height as i32;
        if solid {
            fb.clear([14, 16, 20, 255]);
        }
        fb.fill_rect(0, 0, w, h, [0, 0, 0, if solid { 40 } else { 150 }]);

        let layout = account_layout(fb.width as f32, fb.height as f32);
        let panel = layout.panel;
        let px = panel.x as i32;
        let py = panel.y as i32;
        let pw = panel.w as i32;
        let ph = panel.h as i32;
        let cx = fb.width as f32 * 0.5;

        fb.fill_rect(px + 10, py + 14, pw, ph, [0, 0, 0, 100]);
        fb.fill_rect(px, py, pw, ph, [28, 32, 40, 230]);
        fb.fill_rect(px, py, pw, 2, [255, 255, 255, 40]);
        fb.fill_rect(px, py, pw, 3, [96, 165, 250, 255]);

        let white = [236, 240, 248, 255];
        let dim = [148, 158, 176, 255];
        let accent = [125, 195, 255, 255];
        let title = if self.opened_from_settings {
            "Account and server"
        } else {
            "Account"
        };
        draw_ui_text(fb, title, cx, panel.y + 28.0, 24.0, accent, true);
        draw_ui_text(
            fb,
            "Login credentials and server connection",
            cx,
            panel.y + 50.0,
            12.0,
            dim,
            true,
        );

        self.draw_field(fb, "Email", &self.email, layout.email, self.focus == AccountFocus::Email);
        let secret_label = match self.secret_mode {
            SecretMode::AccountKey => "Account key (F2 = password)",
            SecretMode::Password => "Password (F2 = account key)",
        };
        let secret_disp = mask_secret(&self.secret, self.secret_mode);
        self.draw_field(
            fb,
            secret_label,
            &secret_disp,
            layout.secret,
            self.focus == AccountFocus::Secret,
        );
        self.draw_field(fb, "Host", &self.host, layout.host, self.focus == AccountFocus::Host);
        self.draw_field(
            fb,
            "Port",
            &self.port_text,
            layout.port,
            self.focus == AccountFocus::Port,
        );

        draw_ui_text(
            fb,
            "Servers (local first - click to switch)",
            layout.recent_label_x,
            layout.recent_label_y,
            12.0,
            dim,
            false,
        );
        for (i, r) in layout.recent.iter().enumerate() {
            let Some(ep) = self.recent_servers.get(i) else {
                break;
            };
            let focused = self.focus == AccountFocus::Recent(i as u8);
            let active = ep.same_as(&self.host, self.port);
            let bg = if focused {
                [50, 70, 100, 220]
            } else if active {
                [36, 72, 110, 210]
            } else {
                [32, 36, 46, 200]
            };
            fb.fill_rect(r.x as i32, r.y as i32, r.w as i32, r.h as i32, bg);
            if active {
                fb.fill_rect(r.x as i32, r.y as i32, 3, r.h as i32, accent);
            }
            let tag = if ep.is_local() {
                format!("Local  {}", ep.label())
            } else {
                format!("Recent {}", ep.label())
            };
            draw_ui_text(
                fb,
                &tag,
                r.x + 12.0,
                r.y + r.h * 0.5,
                13.0,
                if active || focused { white } else { dim },
                false,
            );
        }

        let connect_label = if self.opened_from_settings {
            "Save"
        } else {
            "Connect"
        };
        let c_focus = self.focus == AccountFocus::Connect;
        fb.fill_rect(
            layout.connect.x as i32,
            layout.connect.y as i32,
            layout.connect.w as i32,
            layout.connect.h as i32,
            if c_focus {
                [55, 140, 90, 255]
            } else {
                [40, 110, 70, 240]
            },
        );
        draw_ui_text(
            fb,
            connect_label,
            layout.connect.x + layout.connect.w * 0.5,
            layout.connect.y + layout.connect.h * 0.5,
            15.0,
            white,
            true,
        );

        if self.opened_from_settings {
            if let Some(back) = layout.back {
                let b_focus = self.focus == AccountFocus::Back;
                fb.fill_rect(
                    back.x as i32,
                    back.y as i32,
                    back.w as i32,
                    back.h as i32,
                    if b_focus {
                        [60, 70, 90, 230]
                    } else {
                        [40, 44, 54, 200]
                    },
                );
                draw_ui_text(
                    fb,
                    "Back",
                    back.x + back.w * 0.5,
                    back.y + back.h * 0.5,
                    15.0,
                    white,
                    true,
                );
            }
        } else {
            fb.fill_rect(
                layout.settings.x as i32,
                layout.settings.y as i32,
                layout.settings.w as i32,
                layout.settings.h as i32,
                [50, 90, 140, 240],
            );
            draw_ui_text(
                fb,
                "Settings",
                layout.settings.x + layout.settings.w * 0.5,
                layout.settings.y + layout.settings.h * 0.5,
                15.0,
                white,
                true,
            );
        }

        if !self.status.is_empty() {
            draw_ui_text(fb, &self.status, cx, panel.y + panel.h - 28.0, 12.0, dim, true);
        }
        let help = if self.opened_from_settings {
            "Tab fields  |  click server  |  Save  |  Esc=Back"
        } else {
            "Tab fields  |  click server  |  Enter=Connect  |  Esc=Settings"
        };
        draw_ui_text(fb, help, cx, panel.y + panel.h - 12.0, 11.0, [110, 120, 140, 255], true);
    }

    fn draw_field(
        &self,
        fb: &mut Framebuffer,
        label: &str,
        value: &str,
        rect: AccountHitRect,
        focused: bool,
    ) {
        use crate::ui_font::draw_ui_text;
        let dim = [148, 158, 176, 255];
        let white = [236, 240, 248, 255];
        draw_ui_text(fb, label, rect.x, rect.y - 12.0, 11.0, dim, false);
        fb.fill_rect(
            rect.x as i32,
            rect.y as i32,
            rect.w as i32,
            rect.h as i32,
            if focused {
                [38, 48, 64, 255]
            } else {
                [18, 20, 26, 255]
            },
        );
        draw_field_border(fb, rect.x, rect.y, rect.w, rect.h, focused);
        draw_ui_text(
            fb,
            value,
            rect.x + 10.0,
            rect.y + rect.h * 0.5,
            14.0,
            white,
            false,
        );
        if focused && self.caret_t <= 0.5 {
            let prefix: String = value.chars().take(self.caret).collect();
            let cw = crate::ui_font::measure_ui_text(&prefix, 14.0);
            fb.fill_rect(
                (rect.x + 10.0 + cw) as i32,
                (rect.y + 6.0) as i32,
                2,
                (rect.h - 12.0) as i32,
                [240, 240, 250, 255],
            );
        }
    }

    /// Loading screen without full stage state (connect message only).
    ///
    /// Prefer [`crate::load_progress::draw_loading_progress`] when a
    /// [`crate::load_progress::LoadingState`] is available (P5#36).
    pub fn draw_loading(fb: &mut Framebuffer, sprites: Option<&HudSprites>, msg: &str) {
        let mut state = crate::load_progress::LoadingState::for_stage(
            crate::load_progress::LoadStage::Content,
            0.0,
            if msg.is_empty() { None } else { Some(msg) },
        );
        if !msg.is_empty() {
            state.label = msg.to_string();
        }
        crate::load_progress::draw_loading_progress(fb, &state);
        let _ = sprites;
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



/// Axis-aligned hit rect (soft-FB pixels) for account form widgets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccountHitRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl AccountHitRect {
    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Hit targets for Account page mouse interaction.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountLayout {
    pub panel: AccountHitRect,
    pub email: AccountHitRect,
    pub secret: AccountHitRect,
    pub host: AccountHitRect,
    pub port: AccountHitRect,
    pub recent: Vec<AccountHitRect>,
    pub recent_label_x: f32,
    pub recent_label_y: f32,
    pub connect: AccountHitRect,
    /// Settings button (boot path).
    pub settings: AccountHitRect,
    /// Back button (nested from Settings).
    pub back: Option<AccountHitRect>,
}

fn account_layout(fb_w: f32, fb_h: f32) -> AccountLayout {
    let panel_w = (fb_w * 0.72).clamp(480.0, 620.0);
    let panel_h = (fb_h * 0.92).clamp(440.0, 520.0);
    let panel_x = ((fb_w - panel_w) * 0.5).round();
    let panel_y = ((fb_h - panel_h) * 0.5).round();
    let field_x = panel_x + 36.0;
    let field_w = panel_w - 72.0;
    let field_h = 30.0;
    let mut y = panel_y + 78.0;
    let email = AccountHitRect {
        x: field_x,
        y,
        w: field_w,
        h: field_h,
    };
    y += 52.0;
    let secret = AccountHitRect {
        x: field_x,
        y,
        w: field_w,
        h: field_h,
    };
    y += 52.0;
    let host_w = field_w * 0.68;
    let port_w = field_w * 0.28;
    let host = AccountHitRect {
        x: field_x,
        y,
        w: host_w,
        h: field_h,
    };
    let port = AccountHitRect {
        x: field_x + field_w - port_w,
        y,
        w: port_w,
        h: field_h,
    };
    y += 48.0;
    let recent_label_x = field_x;
    let recent_label_y = y;
    y += 16.0;
    let mut recent = Vec::new();
    let row_h = 26.0f32;
    for _ in 0..MAX_RECENT_SERVERS {
        recent.push(AccountHitRect {
            x: field_x,
            y,
            w: field_w,
            h: row_h,
        });
        y += row_h + 6.0;
    }
    y += 8.0;
    let btn_w = 140.0f32;
    let btn_h = 36.0f32;
    let gap = 14.0f32;
    let connect = AccountHitRect {
        x: field_x,
        y,
        w: btn_w,
        h: btn_h,
    };
    let settings = AccountHitRect {
        x: field_x + btn_w + gap,
        y,
        w: btn_w,
        h: btn_h,
    };
    let back = Some(AccountHitRect {
        x: field_x + btn_w + gap,
        y,
        w: btn_w,
        h: btn_h,
    });
    AccountLayout {
        panel: AccountHitRect {
            x: panel_x,
            y: panel_y,
            w: panel_w,
            h: panel_h,
        },
        email,
        secret,
        host,
        port,
        recent,
        recent_label_x,
        recent_label_y,
        connect,
        settings,
        back,
    }
}

/// Char index under click `mx` for a left-aligned field starting at `field_left`.
fn caret_index_at_x(
    sprites: Option<&HudSprites>,
    text: &str,
    field_left: f32,
    mx: f32,
    scale: f32,
) -> usize {
    if mx <= field_left {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let mut best = chars.len();
    for i in 0..=chars.len() {
        let prefix: String = chars[..i].iter().collect();
        let w = measure_text(sprites, &prefix, scale);
        let edge = field_left + w;
        if mx < edge {
            // Pick closer of i-1 and i by midpoint.
            if i == 0 {
                return 0;
            }
            let prev: String = chars[..i - 1].iter().collect();
            let w_prev = measure_text(sprites, &prev, scale);
            let mid = field_left + (w_prev + w) * 0.5;
            return if mx < mid { i - 1 } else { i };
        }
        best = i;
    }
    best
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
        [100, 180, 220, 255]
    } else {
        [70, 80, 95, 255]
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
        self.account.sync_port_from_text();
        self.account.remember_current_server();
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
    ///
    /// Returns `true` when the screen transition happened. Idempotent when already
    /// on Settings (`false`). Logs a one-line reason when blocked (Death/Loading).
    pub fn enter_settings(&mut self) -> bool {
        if self.screen == ClientScreen::Settings {
            return false;
        }
        if !matches!(self.screen, ClientScreen::Account | ClientScreen::Playing) {
            eprintln!(
                "settings: cannot open from screen={} (need account|playing)",
                self.screen.as_str()
            );
            return false;
        }
        self.settings_return = self.screen;
        self.settings.sync_endpoint_from(
            &self.account.host,
            self.account.port,
            &self.account.email,
        );
        self.settings.focus = crate::settings_page::SettingsFocus::SoundVolume;
        self.settings.status =
            "Mouse · Tab=row · [Account settings] · Esc=Back".into();
        self.settings.apply_runtime_globals();
        self.screen = ClientScreen::Settings;
        eprintln!(
            "settings: opened (return={})",
            self.settings_return.as_str()
        );
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

    /// From Settings → Account form (email / key / server). Keeps `settings_return`.
    pub fn enter_account_from_settings(&mut self) {
        if self.screen != ClientScreen::Settings {
            return;
        }
        self.settings.clamp();
        self.settings.apply_runtime_globals();
        let _ = self.settings.save_default();
        self.account.opened_from_settings = true;
        self.account.load_recent_servers();
        self.account.remember_current_server();
        self.account.focus = AccountFocus::Email;
        self.account.caret = self.account.email.chars().count();
        self.account.port_text = self.account.port.to_string();
        self.account.status =
            "Edit login or pick a server — Save applies, Esc returns to Settings".into();
        self.screen = ClientScreen::Account;
        eprintln!("account: opened from Settings");
    }

    /// Nested Account form → Settings (does not clear `settings_return`).
    pub fn return_to_settings_from_account(&mut self) {
        if self.screen != ClientScreen::Account {
            return;
        }
        self.account.sync_port_from_text();
        self.account.remember_current_server();
        self.account.opened_from_settings = false;
        self.settings.sync_endpoint_from(
            &self.account.host,
            self.account.port,
            &self.account.email,
        );
        self.settings.status = format!(
            "Account · {}:{} · {}",
            self.account.host,
            self.account.port,
            if self.account.email.is_empty() {
                "(no email)"
            } else {
                self.account.email.as_str()
            }
        );
        self.settings.focus = crate::settings_page::SettingsFocus::AccountSettings;
        self.screen = ClientScreen::Settings;
        eprintln!("settings: returned from Account form");
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
        assert_eq!(page.focus, AccountFocus::Host);
        page.on_key(AccountKey::Tab { shift: false });
        assert_eq!(page.focus, AccountFocus::Port);
    }

    #[test]
    fn recent_local_first_and_remember() {
        let mut page = AccountPage::default();
        page.recent_servers = vec![ServerEndpoint::local()];
        page.host = "play.example.com".into();
        page.port = 9001;
        page.port_text = "9001".into();
        page.remember_current_server();
        assert!(page.recent_servers[0].is_local());
        assert_eq!(page.recent_servers[1].host, "play.example.com");
        assert_eq!(page.recent_servers[1].port, 9001);
        page.apply_recent(0);
        assert_eq!(page.host, LOCAL_SERVER_HOST);
        assert_eq!(page.port, LOCAL_SERVER_PORT);
    }

    #[test]
    fn enter_connects_escape_opens_settings() {
        let mut page = AccountPage::default();
        // Esc → Settings (same as F3); window close to quit.
        assert_eq!(page.on_key(AccountKey::Escape), AccountAction::OpenSettings);
        page.email = "e@x".into();
        assert_eq!(page.on_key(AccountKey::Escape), AccountAction::OpenSettings);
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
        let mut fb = Framebuffer::new(640, 480);
        let page = AccountPage {
            email: "a@b.c".into(),
            secret: "KEY".into(),
            ..AccountPage::default()
        };
        page.draw(&mut fb, None);
        let n = fb.count_non_color([14, 16, 20, 255]);
        assert!(n > 100, "expected form chrome, got {n} non-bg pixels");
    }

    #[test]
    fn nested_account_esc_backs_to_settings() {
        let mut page = AccountPage::default();
        page.opened_from_settings = true;
        assert_eq!(page.on_key(AccountKey::Escape), AccountAction::Back);
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

    #[test]
    fn mouse_selects_textfields_and_connect() {
        let mut page = AccountPage::default();
        page.email = "hello@test".into();
        page.secret = "key123".into();
        let fb_w = 960.0;
        let fb_h = 540.0;
        let layout = AccountPage::layout(fb_w, fb_h);

        // Click email field → focus Email, caret somewhere in text.
        let a = page.on_pointer_down(
            layout.email.x + 20.0,
            layout.email.y + 10.0,
            fb_w,
            fb_h,
            None,
        );
        assert_eq!(a, AccountAction::None);
        assert_eq!(page.focus, AccountFocus::Email);
        assert!(page.caret <= page.email.chars().count());

        // Click secret field.
        let a = page.on_pointer_down(
            layout.secret.x + 10.0,
            layout.secret.y + 10.0,
            fb_w,
            fb_h,
            None,
        );
        assert_eq!(a, AccountAction::None);
        assert_eq!(page.focus, AccountFocus::Secret);

        // Click Connect.
        let a = page.on_pointer_down(
            layout.connect.x + layout.connect.w * 0.5,
            layout.connect.y + layout.connect.h * 0.5,
            fb_w,
            fb_h,
            None,
        );
        assert_eq!(a, AccountAction::Connect);
        assert_eq!(page.focus, AccountFocus::Connect);

        // Click Settings button.
        let a = page.on_pointer_down(
            layout.settings.x + layout.settings.w * 0.5,
            layout.settings.y + layout.settings.h * 0.5,
            fb_w,
            fb_h,
            None,
        );
        assert_eq!(a, AccountAction::OpenSettings);
    }

    #[test]
    fn enter_settings_from_account_and_playing() {
        let mut app = ClientAppState::default();
        assert!(app.screen.is_account());
        assert!(app.enter_settings());
        assert!(app.screen.is_settings());
        // Already open — no-op
        assert!(!app.enter_settings());
        app.leave_settings();
        assert!(app.screen.is_account());
        app.enter_playing();
        assert!(app.enter_settings());
        assert!(app.screen.is_settings());
        app.leave_settings();
        assert!(app.screen.is_playing());
    }
}
