//! Twin login page (C++ `TwinPage`).
//!
//! Shared code + party size (twins / triplets / quadruplets / same family).
//! LOGIN sends `sha1(twin_code)` and `twin_count` (`protocol.txt`).
//! `twin_count == 0` is C++ “same family” (radio index 3).

use crate::hud::HudSprites;
use crate::render::Framebuffer;
use crate::ui_font::draw_ui_text;

/// Built-in generate list when `wordList.txt` is missing (C++ hides Generate if < 20).
const FALLBACK_WORDS: &[&str] = &[
    "apple", "river", "stone", "ember", "willow", "copper", "meadow", "flint",
    "cedar", "honey", "amber", "otter", "quilt", "linen", "maple", "pepper",
    "saddle", "thistle", "violet", "yarrow", "barley", "cinder", "daisy", "fern",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinFocus {
    Code,
    Twins,
    Triplets,
    Quads,
    SameFamily,
    Generate,
    Login,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinAction {
    None,
    /// Use code + count and continue to Connect.
    Login,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct TwinPage {
    pub code: String,
    /// 2 twins, 3 triplets, 4 quads, 0 same-family (C++).
    pub twin_count: i32,
    pub focus: TwinFocus,
    pub status: String,
    pub caret: usize,
    seed: u64,
    words: Vec<String>,
}

impl Default for TwinPage {
    fn default() -> Self {
        let mut p = Self {
            code: String::new(),
            twin_count: 2,
            focus: TwinFocus::Code,
            status: "Share this code with your twins, then Login.".into(),
            caret: 0,
            seed: 1,
            words: Vec::new(),
        };
        p.load_words();
        p.reseed();
        p
    }
}

impl TwinPage {
    fn load_words(&mut self) {
        self.words.clear();
        for path in ["wordList.txt", r"C:\OhOl\OpenLife\OneLifeGameSourceData\wordList.txt"] {
            if let Ok(text) = std::fs::read_to_string(path) {
                for w in text.split_whitespace() {
                    let t = w.trim();
                    if t.len() >= 2 {
                        self.words.push(t.to_lowercase());
                    }
                }
                if self.words.len() >= 20 {
                    break;
                }
            }
        }
        if self.words.len() < 20 {
            self.words = FALLBACK_WORDS.iter().map(|s| (*s).to_string()).collect();
        }
    }

    fn reseed(&mut self) {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        self.seed = t | 1;
    }

    fn next_rand(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.seed
    }

    /// C++ generate: three random words from the list.
    pub fn generate(&mut self) {
        if self.words.len() < 3 {
            return;
        }
        let n = self.words.len();
        let ia = (self.next_rand() as usize) % n;
        let ib = (self.next_rand() as usize) % n;
        let ic = (self.next_rand() as usize) % n;
        let a = self.words[ia].clone();
        let b = self.words[ib].clone();
        let c = self.words[ic].clone();
        self.code = format!("{a} {b} {c}");
        self.caret = self.code.chars().count();
        self.status = "Generated a new twin code.".into();
    }

    pub fn trimmed_code(&self) -> String {
        self.code.trim().to_string()
    }

    pub fn can_login(&self) -> bool {
        !self.trimmed_code().is_empty()
    }

    pub fn party_label(&self) -> &'static str {
        match self.twin_count {
            0 => "same family",
            3 => "triplets",
            4 => "quadruplets",
            _ => "twins",
        }
    }

    pub fn on_key(&mut self, key: TwinKey) -> TwinAction {
        match key {
            TwinKey::Tab { shift } => {
                self.cycle_focus(shift);
                TwinAction::None
            }
            TwinKey::Enter => match self.focus {
                TwinFocus::Generate => {
                    self.generate();
                    TwinAction::None
                }
                TwinFocus::Cancel => TwinAction::Cancel,
                TwinFocus::Twins => {
                    self.twin_count = 2;
                    TwinAction::None
                }
                TwinFocus::Triplets => {
                    self.twin_count = 3;
                    TwinAction::None
                }
                TwinFocus::Quads => {
                    self.twin_count = 4;
                    TwinAction::None
                }
                TwinFocus::SameFamily => {
                    self.twin_count = 0;
                    TwinAction::None
                }
                TwinFocus::Code | TwinFocus::Login => {
                    if self.can_login() {
                        TwinAction::Login
                    } else {
                        self.status = "Enter a twin code first.".into();
                        TwinAction::None
                    }
                }
            },
            TwinKey::Escape => TwinAction::Cancel,
            TwinKey::Char(c) => {
                if self.focus == TwinFocus::Code {
                    self.type_char(c);
                }
                TwinAction::None
            }
            TwinKey::Backspace => {
                if self.focus == TwinFocus::Code {
                    self.backspace();
                }
                TwinAction::None
            }
            TwinKey::Other => TwinAction::None,
        }
    }

    fn cycle_focus(&mut self, shift: bool) {
        const ORDER: [TwinFocus; 8] = [
            TwinFocus::Code,
            TwinFocus::Twins,
            TwinFocus::Triplets,
            TwinFocus::Quads,
            TwinFocus::SameFamily,
            TwinFocus::Generate,
            TwinFocus::Login,
            TwinFocus::Cancel,
        ];
        let i = ORDER.iter().position(|&f| f == self.focus).unwrap_or(0);
        self.focus = if shift {
            ORDER[(i + ORDER.len() - 1) % ORDER.len()]
        } else {
            ORDER[(i + 1) % ORDER.len()]
        };
        if self.focus == TwinFocus::Code {
            self.caret = self.code.chars().count();
        }
    }

    fn type_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        if self.code.chars().count() >= 80 {
            return;
        }
        let mut chars: Vec<char> = self.code.chars().collect();
        let i = self.caret.min(chars.len());
        chars.insert(i, ch);
        self.code = chars.into_iter().collect();
        self.caret = i + 1;
    }

    fn backspace(&mut self) {
        if self.caret == 0 {
            return;
        }
        let mut chars: Vec<char> = self.code.chars().collect();
        let i = self.caret.min(chars.len());
        if i == 0 {
            return;
        }
        chars.remove(i - 1);
        self.code = chars.into_iter().collect();
        self.caret = i - 1;
    }

    pub fn on_pointer_down(&mut self, mx: f32, my: f32, fb_w: f32, fb_h: f32) -> TwinAction {
        let l = twin_layout(fb_w, fb_h);
        if l.code.contains(mx, my) {
            self.focus = TwinFocus::Code;
        } else if l.twins.contains(mx, my) {
            self.focus = TwinFocus::Twins;
            self.twin_count = 2;
        } else if l.triplets.contains(mx, my) {
            self.focus = TwinFocus::Triplets;
            self.twin_count = 3;
        } else if l.quads.contains(mx, my) {
            self.focus = TwinFocus::Quads;
            self.twin_count = 4;
        } else if l.same_fam.contains(mx, my) {
            self.focus = TwinFocus::SameFamily;
            self.twin_count = 0;
        } else if l.generate.contains(mx, my) {
            self.focus = TwinFocus::Generate;
            self.generate();
        } else if l.login.contains(mx, my) {
            self.focus = TwinFocus::Login;
            if self.can_login() {
                return TwinAction::Login;
            }
            self.status = "Enter a twin code first.".into();
        } else if l.cancel.contains(mx, my) {
            return TwinAction::Cancel;
        }
        TwinAction::None
    }

    pub fn draw(&self, fb: &mut Framebuffer, _sprites: Option<&HudSprites>) {
        fb.clear([16, 18, 24, 255]);
        let l = twin_layout(fb.width as f32, fb.height as f32);
        let white = [236, 240, 248, 255];
        let dim = [148, 158, 176, 255];
        let cx = fb.width as f32 * 0.5;
        draw_ui_text(fb, "PLAY WITH TWINS", cx, 36.0, 18.0, white, true);
        draw_ui_text(
            fb,
            "Share one code. Everyone logs in with the same words.",
            cx,
            58.0,
            12.0,
            dim,
            true,
        );
        draw_ui_text(fb, "Twin code", l.code.x, l.code.y - 14.0, 11.0, dim, false);
        fill_box(fb, l.code, self.focus == TwinFocus::Code);
        draw_ui_text(
            fb,
            &self.code,
            l.code.x + 10.0,
            l.code.y + l.code.h * 0.5,
            14.0,
            white,
            false,
        );
        self.draw_radio(fb, l.twins, TwinFocus::Twins, "Twins (2)", 2);
        self.draw_radio(fb, l.triplets, TwinFocus::Triplets, "Triplets (3)", 3);
        self.draw_radio(fb, l.quads, TwinFocus::Quads, "Quadruplets (4)", 4);
        self.draw_radio(fb, l.same_fam, TwinFocus::SameFamily, "Same family", 0);
        if self.twin_count == 0 {
            draw_ui_text(
                fb,
                "Same family: born near each other, not necessarily together.",
                cx,
                l.same_fam.y + 36.0,
                11.0,
                dim,
                true,
            );
        }
        draw_btn(fb, l.generate, self.focus == TwinFocus::Generate, "Generate");
        draw_btn(fb, l.login, self.focus == TwinFocus::Login, "Login");
        draw_btn(fb, l.cancel, self.focus == TwinFocus::Cancel, "Cancel");
        if !self.status.is_empty() {
            draw_ui_text(fb, &self.status, cx, fb.height as f32 - 28.0, 12.0, dim, true);
        }
    }

    fn draw_radio(&self, fb: &mut Framebuffer, r: Hit, focus: TwinFocus, label: &str, count: i32) {
        let on = self.twin_count == count;
        let focused = self.focus == focus;
        fill_box(fb, r, focused);
        let mark = if on { "[x]" } else { "[ ]" };
        draw_ui_text(
            fb,
            &format!("{mark}  {label}"),
            r.x + 10.0,
            r.y + r.h * 0.5,
            13.0,
            [236, 240, 248, 255],
            false,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinKey {
    Tab { shift: bool },
    Enter,
    Escape,
    Char(char),
    Backspace,
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

struct TwinLayout {
    code: Hit,
    twins: Hit,
    triplets: Hit,
    quads: Hit,
    same_fam: Hit,
    generate: Hit,
    login: Hit,
    cancel: Hit,
}

fn twin_layout(fb_w: f32, fb_h: f32) -> TwinLayout {
    let cx = fb_w * 0.5;
    let field_w = (fb_w * 0.62).clamp(360.0, 520.0);
    let x = cx - field_w * 0.5;
    let mut y = 96.0;
    let code = Hit {
        x,
        y,
        w: field_w,
        h: 32.0,
    };
    y += 52.0;
    let radio_h = 28.0;
    let twins = Hit {
        x,
        y,
        w: field_w,
        h: radio_h,
    };
    y += 34.0;
    let triplets = Hit {
        x,
        y,
        w: field_w,
        h: radio_h,
    };
    y += 34.0;
    let quads = Hit {
        x,
        y,
        w: field_w,
        h: radio_h,
    };
    y += 34.0;
    let same_fam = Hit {
        x,
        y,
        w: field_w,
        h: radio_h,
    };
    y = (fb_h - 90.0).max(y + 50.0);
    let btn_w = 120.0;
    let btn_h = 34.0;
    let gap = 16.0;
    let total = btn_w * 3.0 + gap * 2.0;
    let bx = cx - total * 0.5;
    TwinLayout {
        code,
        twins,
        triplets,
        quads,
        same_fam,
        generate: Hit {
            x: bx,
            y,
            w: btn_w,
            h: btn_h,
        },
        login: Hit {
            x: bx + btn_w + gap,
            y,
            w: btn_w,
            h: btn_h,
        },
        cancel: Hit {
            x: bx + (btn_w + gap) * 2.0,
            y,
            w: btn_w,
            h: btn_h,
        },
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
            [55, 140, 90, 255]
        } else {
            [40, 90, 70, 230]
        },
    );
    draw_ui_text(
        fb,
        label,
        r.x + r.w * 0.5,
        r.y + r.h * 0.5,
        14.0,
        [236, 240, 248, 255],
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_three_words() {
        let mut p = TwinPage::default();
        p.generate();
        let parts: Vec<_> = p.code.split_whitespace().collect();
        assert_eq!(parts.len(), 3);
        assert!(p.can_login());
    }

    #[test]
    fn radio_sets_count_like_cpp() {
        let mut p = TwinPage::default();
        assert_eq!(p.twin_count, 2);
        p.on_key(TwinKey::Tab { shift: false });
        p.on_key(TwinKey::Tab { shift: false });
        assert_eq!(p.on_key(TwinKey::Enter), TwinAction::None);
        // After code, first tab is twins already selected; two tabs → triplets
        p.focus = TwinFocus::Triplets;
        p.on_key(TwinKey::Enter);
        assert_eq!(p.twin_count, 3);
        p.focus = TwinFocus::SameFamily;
        p.on_key(TwinKey::Enter);
        assert_eq!(p.twin_count, 0);
        p.focus = TwinFocus::Quads;
        p.on_key(TwinKey::Enter);
        assert_eq!(p.twin_count, 4);
    }

    #[test]
    fn empty_code_blocks_login() {
        let mut p = TwinPage::default();
        p.code.clear();
        p.focus = TwinFocus::Login;
        assert_eq!(p.on_key(TwinKey::Enter), TwinAction::None);
    }
}
