//! Community / review page — Open Life Reborn site + Discord.
//!
//! Jason’s C++ `ReviewPage` posted a text review to his server. This client
//! lists community links instead (editable in Settings).

use crate::hud::HudSprites;
use crate::render::Framebuffer;
use crate::ui_font::draw_ui_text;

/// Default community site (Settings `community_site`).
pub const DEFAULT_COMMUNITY_SITE: &str = "https://openlifereborn.com/";
/// Default Discord invite (Settings `discord_url`).
pub const DEFAULT_DISCORD_URL: &str = "https://discord.gg/XtVEA5RVjX";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFocus {
    Site,
    Discord,
    OpenSite,
    OpenDiscord,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    None,
    Back,
    OpenUrl,
}

#[derive(Debug, Clone)]
pub struct ReviewPage {
    pub site_url: String,
    pub discord_url: String,
    pub focus: ReviewFocus,
    pub status: String,
    pub caret: usize,
}

impl Default for ReviewPage {
    fn default() -> Self {
        Self {
            site_url: DEFAULT_COMMUNITY_SITE.into(),
            discord_url: DEFAULT_DISCORD_URL.into(),
            focus: ReviewFocus::Site,
            status: "Enter opens the focused link. Tab to edit URLs.".into(),
            caret: 0,
        }
    }
}

impl ReviewPage {
    pub fn sync_from_settings(&mut self, site: &str, discord: &str) {
        if !site.trim().is_empty() {
            self.site_url = site.trim().to_string();
        }
        if !discord.trim().is_empty() {
            self.discord_url = discord.trim().to_string();
        }
    }

    fn focused_url_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            ReviewFocus::Site => Some(&mut self.site_url),
            ReviewFocus::Discord => Some(&mut self.discord_url),
            _ => None,
        }
    }

    pub fn active_url(&self) -> Option<&str> {
        match self.focus {
            ReviewFocus::Site | ReviewFocus::OpenSite => Some(self.site_url.trim()),
            ReviewFocus::Discord | ReviewFocus::OpenDiscord => Some(self.discord_url.trim()),
            ReviewFocus::Back => None,
        }
    }

    pub fn on_key(&mut self, key: ReviewKey) -> ReviewAction {
        match key {
            ReviewKey::Tab { shift } => {
                self.cycle(shift);
                ReviewAction::None
            }
            ReviewKey::Enter => match self.focus {
                ReviewFocus::Back => ReviewAction::Back,
                ReviewFocus::OpenSite | ReviewFocus::OpenDiscord | ReviewFocus::Site
                | ReviewFocus::Discord => ReviewAction::OpenUrl,
            },
            ReviewKey::Escape => ReviewAction::Back,
            ReviewKey::Char(c) => {
                self.type_char(c);
                ReviewAction::None
            }
            ReviewKey::Backspace => {
                self.backspace();
                ReviewAction::None
            }
            ReviewKey::Digit1 => {
                self.focus = ReviewFocus::OpenSite;
                ReviewAction::OpenUrl
            }
            ReviewKey::Digit2 => {
                self.focus = ReviewFocus::OpenDiscord;
                ReviewAction::OpenUrl
            }
            ReviewKey::Other => ReviewAction::None,
        }
    }

    fn cycle(&mut self, shift: bool) {
        const ORDER: [ReviewFocus; 5] = [
            ReviewFocus::Site,
            ReviewFocus::Discord,
            ReviewFocus::OpenSite,
            ReviewFocus::OpenDiscord,
            ReviewFocus::Back,
        ];
        let i = ORDER.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = if shift {
            ORDER[(i + ORDER.len() - 1) % ORDER.len()]
        } else {
            ORDER[(i + 1) % ORDER.len()]
        };
        if matches!(self.focus, ReviewFocus::Site | ReviewFocus::Discord) {
            self.caret = self.focused_url_mut().map(|s| s.chars().count()).unwrap_or(0);
        }
    }

    fn type_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        let caret = self.caret;
        let Some(text) = self.focused_url_mut() else {
            return;
        };
        if text.chars().count() >= 200 {
            return;
        }
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        chars.insert(i, ch);
        *text = chars.into_iter().collect();
        self.caret = i + 1;
    }

    fn backspace(&mut self) {
        let caret = self.caret;
        let Some(text) = self.focused_url_mut() else {
            return;
        };
        if caret == 0 {
            return;
        }
        let mut chars: Vec<char> = text.chars().collect();
        let i = caret.min(chars.len());
        if i == 0 {
            return;
        }
        chars.remove(i - 1);
        *text = chars.into_iter().collect();
        self.caret = i - 1;
    }

    pub fn on_pointer_down(&mut self, mx: f32, my: f32, fb_w: f32, fb_h: f32) -> ReviewAction {
        let l = review_layout(fb_w, fb_h);
        if l.site.contains(mx, my) {
            self.focus = ReviewFocus::Site;
        } else if l.discord.contains(mx, my) {
            self.focus = ReviewFocus::Discord;
        } else if l.open_site.contains(mx, my) {
            self.focus = ReviewFocus::OpenSite;
            return ReviewAction::OpenUrl;
        } else if l.open_discord.contains(mx, my) {
            self.focus = ReviewFocus::OpenDiscord;
            return ReviewAction::OpenUrl;
        } else if l.back.contains(mx, my) {
            return ReviewAction::Back;
        }
        ReviewAction::None
    }

    pub fn draw(&self, fb: &mut Framebuffer, _sprites: Option<&HudSprites>) {
        fb.clear([14, 16, 22, 255]);
        let l = review_layout(fb.width as f32, fb.height as f32);
        let white = [236, 240, 248, 255];
        let dim = [148, 158, 176, 255];
        let cx = fb.width as f32 * 0.5;
        draw_ui_text(fb, "COMMUNITY", cx, 40.0, 20.0, white, true);
        draw_ui_text(
            fb,
            "Open Life Reborn — change these URLs in Settings too.",
            cx,
            64.0,
            12.0,
            dim,
            true,
        );
        draw_ui_text(fb, "Website", l.site.x, l.site.y - 14.0, 11.0, dim, false);
        fill_box(fb, l.site, self.focus == ReviewFocus::Site);
        draw_ui_text(
            fb,
            &self.site_url,
            l.site.x + 8.0,
            l.site.y + l.site.h * 0.5,
            13.0,
            white,
            false,
        );
        draw_ui_text(
            fb,
            "Discord",
            l.discord.x,
            l.discord.y - 14.0,
            11.0,
            dim,
            false,
        );
        fill_box(fb, l.discord, self.focus == ReviewFocus::Discord);
        draw_ui_text(
            fb,
            &self.discord_url,
            l.discord.x + 8.0,
            l.discord.y + l.discord.h * 0.5,
            13.0,
            white,
            false,
        );
        draw_btn(
            fb,
            l.open_site,
            self.focus == ReviewFocus::OpenSite,
            "1  Open site",
        );
        draw_btn(
            fb,
            l.open_discord,
            self.focus == ReviewFocus::OpenDiscord,
            "2  Open Discord",
        );
        draw_btn(fb, l.back, self.focus == ReviewFocus::Back, "Back");
        if !self.status.is_empty() {
            draw_ui_text(fb, &self.status, cx, fb.height as f32 - 28.0, 12.0, dim, true);
        }
    }
}

/// Open an http(s) URL with the OS handler. Returns an error string on failure.
pub fn open_http_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if !(u.starts_with("https://") || u.starts_with("http://")) {
        return Err("URL must start with http:// or https://".into());
    }
    if u.chars().any(|c| c.is_control() || c == '"' || c == '%') {
        return Err("URL has invalid characters".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", u])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(opener)
            .arg(u)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewKey {
    Tab { shift: bool },
    Enter,
    Escape,
    Char(char),
    Backspace,
    Digit1,
    Digit2,
    Other,
}

#[derive(Clone, Copy)]
struct Hit {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Hit {
    fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

struct ReviewLayout {
    site: Hit,
    discord: Hit,
    open_site: Hit,
    open_discord: Hit,
    back: Hit,
}

fn review_layout(fb_w: f32, _fb_h: f32) -> ReviewLayout {
    let cx = fb_w * 0.5;
    let field_w = (fb_w * 0.72).clamp(420.0, 640.0);
    let x = cx - field_w * 0.5;
    let site = Hit {
        x,
        y: 110.0,
        w: field_w,
        h: 32.0,
    };
    let discord = Hit {
        x,
        y: 170.0,
        w: field_w,
        h: 32.0,
    };
    let btn_w = 180.0;
    let btn_h = 34.0;
    let open_site = Hit {
        x: cx - btn_w - 10.0,
        y: 230.0,
        w: btn_w,
        h: btn_h,
    };
    let open_discord = Hit {
        x: cx + 10.0,
        y: 230.0,
        w: btn_w,
        h: btn_h,
    };
    let back = Hit {
        x: cx - 70.0,
        y: 280.0,
        w: 140.0,
        h: btn_h,
    };
    ReviewLayout {
        site,
        discord,
        open_site,
        open_discord,
        back,
    }
}

fn fill_box(fb: &mut Framebuffer, r: Hit, focused: bool) {
    fb.fill_rect(
        r.x as i32,
        r.y as i32,
        r.w as i32,
        r.h as i32,
        if focused {
            [38, 48, 64, 255]
        } else {
            [22, 26, 34, 255]
        },
    );
}

fn draw_btn(fb: &mut Framebuffer, r: Hit, focused: bool, label: &str) {
    fb.fill_rect(
        r.x as i32,
        r.y as i32,
        r.w as i32,
        r.h as i32,
        if focused {
            [55, 120, 160, 255]
        } else {
            [40, 70, 100, 230]
        },
    );
    draw_ui_text(
        fb,
        label,
        r.x + r.w * 0.5,
        r.y + r.h * 0.5,
        13.0,
        [236, 240, 248, 255],
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http() {
        assert!(open_http_url("javascript:alert(1)").is_err());
        assert!(open_http_url("ftp://x").is_err());
    }

    #[test]
    fn defaults_match_olr() {
        let p = ReviewPage::default();
        assert!(p.site_url.contains("openlifereborn.com"));
        assert!(p.discord_url.contains("discord.gg"));
    }
}
