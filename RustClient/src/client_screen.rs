//! P5#38 — Death / rebirth soft-FB page.
//!
//! C++: `LivingLifePage` → `RebirthChoicePage` / `FinalMessagePage` on our delete PU.
//! Screen graph enum is [`crate::account_page::ClientScreen`]; death summary on
//! [`ClientAppState::death`]. Settings live in [`crate::settings_page`] (P5#39).
//! Headless probes never enter this path.

use crate::account_page::{ClientAppState, ClientScreen};
use crate::hud::draw_pencil_string;
use crate::live_object::LiveObject;
use crate::render::Framebuffer;
use crate::session::SessionConfig;

/// Summary shown on the death / rebirth page (from last known LiveObject).
#[derive(Debug, Clone, PartialEq)]
pub struct DeathSummary {
    /// Display name (`NM`), or a placeholder when unnamed.
    pub name: String,
    /// In-game age in years (preserved across delete PU).
    pub age_years: f32,
    /// Raw server reason (`reason_hunger`, …) when known.
    pub reason: Option<String>,
}

impl DeathSummary {
    /// Build from a deleted (or still-living) [`LiveObject`] snapshot.
    pub fn from_live_object(o: &LiveObject) -> Self {
        let name = o
            .name
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".into());
        Self {
            name,
            age_years: o.current_age(),
            reason: o.delete_reason.clone(),
        }
    }

    /// Explicit constructor for tests / offline demos.
    pub fn new(name: impl Into<String>, age_years: f32, reason: Option<String>) -> Self {
        Self {
            name: name.into(),
            age_years,
            reason,
        }
    }

    /// Human-readable reason line (`reason_hunger` → `"hunger"`).
    pub fn reason_display(&self) -> Option<String> {
        self.reason.as_ref().map(|r| format_death_reason(r))
    }

    /// Multi-line summary for soft-FB / logs.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(4);
        out.push(format!("Name: {}", self.name));
        out.push(format!("Age: {:.1}", self.age_years));
        if let Some(r) = self.reason_display() {
            out.push(format!("Reason: {r}"));
        }
        out
    }
}

/// Keyboard action on the death page (soft-FB; minifb maps keys in `ohol-client`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathKey {
    /// R — rebirth / continue.
    Rebirth,
    /// Enter / Return — same as Rebirth.
    Confirm,
    /// Esc — leave client (optional quit).
    Quit,
    /// Any other key — ignore.
    Other,
}

/// Result of handling input / transition on the death screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenCommand {
    None,
    /// Full reconnect with same credentials (LOGIN after death; see [`rebirth_session_config`]).
    Rebirth,
    /// Close the window / exit the client loop.
    Quit,
}

/// Map a death-page key to a screen command (only when already on Death).
pub fn death_key_command(screen: ClientScreen, key: DeathKey) -> ScreenCommand {
    if screen != ClientScreen::Death {
        return ScreenCommand::None;
    }
    match key {
        DeathKey::Rebirth | DeathKey::Confirm => ScreenCommand::Rebirth,
        DeathKey::Quit => ScreenCommand::Quit,
        DeathKey::Other => ScreenCommand::None,
    }
}

/// If we are Playing and our LiveObject is deleted, transition app → Death.
///
/// Returns `true` when the screen changed. Safe to call every frame after
/// session poll (headless probes never call this — probes unchanged).
pub fn note_our_death_if_any(app: &mut ClientAppState, our: Option<&LiveObject>) -> bool {
    if app.screen != ClientScreen::Playing {
        return false;
    }
    let Some(me) = our else {
        return false;
    };
    if !me.deleted {
        return false;
    }
    app.enter_death(DeathSummary::from_live_object(me));
    true
}

/// Strip `reason_` prefix and replace `_` with spaces for soft-FB text.
pub fn format_death_reason(raw: &str) -> String {
    let s = raw.trim();
    let s = s
        .strip_prefix("reason_")
        .or_else(|| s.strip_prefix("REASON_"))
        .unwrap_or(s);
    s.replace('_', " ")
}

/// Soft-FB death page: dark fill + pencil summary + rebirth hints.
///
/// No font TGA dependency — uses [`draw_pencil_string`] (5×7 glyphs).
pub fn draw_death_screen(fb: &mut Framebuffer, summary: &DeathSummary) {
    // Deep red-black backdrop (distinct from play clear color).
    fb.clear([18, 8, 10, 255]);

    let cx = fb.width as f32 * 0.5;
    let mut y = fb.height as f32 * 0.28;
    let title_scale = 3.0;
    let body_scale = 2.0;
    let hint_scale = 1.5;
    let white = [240, 230, 220, 255];
    let dim = [180, 160, 150, 255];
    let accent = [220, 80, 70, 255];

    draw_pencil_string(fb, "YOU DIED", cx, y, title_scale, accent, true);
    y += 28.0 * title_scale * 0.35 + 18.0;

    for line in summary.lines() {
        draw_pencil_string(fb, &line, cx, y, body_scale, white, true);
        y += 14.0 * body_scale;
    }

    y += 20.0;
    draw_pencil_string(fb, "R / Enter  Rebirth", cx, y, hint_scale, dim, true);
    y += 14.0 * hint_scale;
    draw_pencil_string(fb, "Esc  Quit", cx, y, hint_scale, dim, true);
}

/// SessionConfig for rebirth after confirmed death: full reconnect, new life (LOGIN).
///
/// Mid-life RLOGIN is available by setting `reconnect: true` on the returned config.
pub fn rebirth_session_config(base: &SessionConfig) -> SessionConfig {
    let mut cfg = base.clone();
    cfg.reconnect = false;
    cfg
}

/// Death helpers on the shared product-page app state.
impl ClientAppState {
    /// Enter death page (idempotent if already Death — keeps first summary).
    pub fn enter_death(&mut self, summary: DeathSummary) {
        if self.screen == ClientScreen::Death {
            return;
        }
        self.death = Some(summary);
        self.screen = ClientScreen::Death;
        self.loading_msg.clear();
    }

    /// After a successful rebirth reconnect, return to Playing.
    pub fn enter_playing_from_death(&mut self) {
        self.death = None;
        self.screen = ClientScreen::Playing;
        self.loading_msg.clear();
    }

    /// Snapshot for soft-FB / tests.
    pub fn death_summary(&self) -> Option<&DeathSummary> {
        self.death.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_object::LiveWorld;
    use crate::parse::parse_pu_line;

    fn living_our() -> LiveWorld {
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(
            "7 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 22.5 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
        )
        .unwrap();
        w.apply_pu(&pu);
        w.set_our_id(7);
        w.apply_names(&[crate::parse::PlayerName {
            player_id: 7,
            first_name: "ADA".into(),
            last_name: "SNOW".into(),
        }]);
        w
    }

    #[test]
    fn playing_to_death_on_our_delete() {
        let mut w = living_our();
        let mut app = ClientAppState {
            screen: ClientScreen::Playing,
            ..ClientAppState::default()
        };
        assert!(!note_our_death_if_any(&mut app, w.our()));

        let del = parse_pu_line("7 19 0 0 0 0 0 0 0 0 -1 0 0 0 X X reason_hunger").unwrap();
        w.apply_pu(&del);
        assert!(note_our_death_if_any(&mut app, w.our()));
        assert_eq!(app.screen, ClientScreen::Death);
        let s = app.death_summary().unwrap();
        assert_eq!(s.name, "ADA SNOW");
        assert!((s.age_years - 22.5).abs() < 0.01);
        assert_eq!(s.reason.as_deref(), Some("reason_hunger"));
        assert_eq!(s.reason_display().as_deref(), Some("hunger"));
    }

    #[test]
    fn other_player_delete_does_not_leave_playing() {
        let mut w = living_our();
        let other = parse_pu_line(
            "8 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 12 10 10.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
        )
        .unwrap();
        w.apply_pu(&other);
        let del = parse_pu_line("8 19 0 0 0 0 0 0 0 0 -1 0 0 0 X X reason_killed").unwrap();
        w.apply_pu(&del);
        let mut app = ClientAppState {
            screen: ClientScreen::Playing,
            ..ClientAppState::default()
        };
        assert!(!note_our_death_if_any(&mut app, w.our()));
        assert_eq!(app.screen, ClientScreen::Playing);
    }

    #[test]
    fn death_keys_rebirth_and_quit() {
        assert_eq!(
            death_key_command(ClientScreen::Death, DeathKey::Rebirth),
            ScreenCommand::Rebirth
        );
        assert_eq!(
            death_key_command(ClientScreen::Death, DeathKey::Confirm),
            ScreenCommand::Rebirth
        );
        assert_eq!(
            death_key_command(ClientScreen::Death, DeathKey::Quit),
            ScreenCommand::Quit
        );
        assert_eq!(
            death_key_command(ClientScreen::Death, DeathKey::Other),
            ScreenCommand::None
        );
        assert_eq!(
            death_key_command(ClientScreen::Playing, DeathKey::Rebirth),
            ScreenCommand::None
        );
    }

    #[test]
    fn enter_death_keeps_first_summary() {
        let mut app = ClientAppState {
            screen: ClientScreen::Playing,
            ..ClientAppState::default()
        };
        app.enter_death(DeathSummary::new("A", 5.0, Some("reason_hunger".into())));
        app.enter_death(DeathSummary::new("B", 9.0, Some("reason_other".into())));
        assert_eq!(app.death_summary().unwrap().name, "A");
        assert_eq!(app.screen, ClientScreen::Death);
    }

    #[test]
    fn enter_playing_from_death_clears_summary() {
        let mut app = ClientAppState {
            screen: ClientScreen::Playing,
            ..ClientAppState::default()
        };
        app.enter_death(DeathSummary::new("A", 1.0, None));
        app.enter_playing_from_death();
        assert_eq!(app.screen, ClientScreen::Playing);
        assert!(app.death_summary().is_none());
    }

    #[test]
    fn format_death_reason_strips_prefix() {
        assert_eq!(format_death_reason("reason_hunger"), "hunger");
        assert_eq!(format_death_reason("reason_disconnected"), "disconnected");
        assert_eq!(format_death_reason("old age"), "old age");
    }

    #[test]
    fn draw_death_screen_paints_pixels() {
        let mut fb = Framebuffer::new(320, 180);
        let summary = DeathSummary::new("ADA SNOW", 22.5, Some("reason_hunger".into()));
        draw_death_screen(&mut fb, &summary);
        let painted = fb.count_non_color([18, 8, 10, 255]);
        assert!(
            painted > 40,
            "expected death text pixels, got painted={painted}"
        );
    }

    #[test]
    fn rebirth_session_config_clears_reconnect_flag() {
        let base = SessionConfig {
            host: "h".into(),
            port: 9,
            email: "e".into(),
            password: "p".into(),
            account_key: "k".into(),
            reconnect: true,
            ..SessionConfig::default()
        };
        let cfg = rebirth_session_config(&base);
        assert!(!cfg.reconnect);
        assert_eq!(cfg.email, "e");
        assert_eq!(cfg.password, "p");
        assert_eq!(cfg.host, "h");
        assert_eq!(cfg.port, 9);
    }
}
