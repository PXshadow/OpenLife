//! Modern UI text for Settings / Account (system sans via fontdue).
//!
//! Loads Windows Segoe UI (or Arial / DejaVu) once. Falls back to pencil 5×7
//! when no TTF is available so headless tests still draw.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::hud::draw_pencil_string;
use crate::render::Framebuffer;

/// Rasterized system UI font (lazy).
pub struct UiFont {
    font: fontdue::Font,
}

static UI_FONT: OnceLock<Option<UiFont>> = OnceLock::new();

fn candidate_font_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("OHOL_UI_FONT") {
        let t = p.trim();
        if !t.is_empty() {
            out.push(PathBuf::from(t));
        }
    }
    // Windows modern UI fonts first.
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let fonts = PathBuf::from(windir).join("Fonts");
    for name in [
        "segoeui.ttf",
        "SegoeUI.ttf",
        "arial.ttf",
        "Arial.ttf",
        "calibri.ttf",
        "Calibri.ttf",
    ] {
        out.push(fonts.join(name));
    }
    // Linux common paths (harmless if missing).
    out.push(PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"));
    out.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
    out
}

impl UiFont {
    fn load() -> Option<Self> {
        for path in candidate_font_paths() {
            if !path.is_file() {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => match fontdue::Font::from_bytes(bytes.as_slice(), fontdue::FontSettings::default())
                {
                    Ok(font) => {
                        eprintln!("ui_font: loaded {}", path.display());
                        return Some(Self { font });
                    }
                    Err(e) => {
                        eprintln!("ui_font: parse {} failed: {e}", path.display());
                    }
                },
                Err(_) => {}
            }
        }
        eprintln!("ui_font: no system TTF — using pencil fallback");
        None
    }
}

/// Global UI font (None → pencil fallback).
pub fn ui_font() -> Option<&'static UiFont> {
    UI_FONT.get_or_init(UiFont::load).as_ref()
}

/// Measure text width in soft-FB pixels at `size_px` (em size).
pub fn measure_ui_text(text: &str, size_px: f32) -> f32 {
    if let Some(ui) = ui_font() {
        let mut w = 0.0f32;
        for ch in text.chars() {
            let (metrics, _) = ui.font.rasterize(ch, size_px);
            w += metrics.advance_width;
        }
        return w;
    }
    // pencil 5×7: ~6 design units per char; scale so size_px≈14 → scale≈1.6
    let scale = (size_px / 9.0).max(0.5);
    text.chars().count() as f32 * 6.0 * scale
}

/// Draw modern UI text. `size_px` is approximate pixel height (e.g. 18.0 title, 14.0 body).
pub fn draw_ui_text(
    fb: &mut Framebuffer,
    text: &str,
    x: f32,
    y: f32,
    size_px: f32,
    rgba: [u8; 4],
    align_center: bool,
) {
    if let Some(ui) = ui_font() {
        draw_fontdue(fb, &ui.font, text, x, y, size_px, rgba, align_center);
        return;
    }
    let scale = (size_px / 9.0).max(0.5);
    draw_pencil_string(fb, text, x, y, scale, rgba, align_center);
}

fn draw_fontdue(
    fb: &mut Framebuffer,
    font: &fontdue::Font,
    text: &str,
    x: f32,
    y: f32,
    size_px: f32,
    rgba: [u8; 4],
    align_center: bool,
) {
    let size = size_px.max(8.0);
    let mut total_w = 0.0f32;
    for ch in text.chars() {
        let (m, _) = font.rasterize(ch, size);
        total_w += m.advance_width;
    }
    let mut pen_x = if align_center {
        x - total_w * 0.5
    } else {
        x
    };
    // Baseline-ish: center vertically on y
    let baseline = y + size * 0.35;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen_x + metrics.xmin as f32;
        let gy = baseline - metrics.height as f32 - metrics.ymin as f32;
        let w = metrics.width;
        let h = metrics.height;
        for row in 0..h {
            for col in 0..w {
                let cov = bitmap[row * w + col];
                if cov < 8 {
                    continue;
                }
                let a = ((rgba[3] as u32 * cov as u32) / 255) as u8;
                if a == 0 {
                    continue;
                }
                fb.put(
                    (gx + col as f32).round() as i32,
                    (gy + row as f32).round() as i32,
                    [rgba[0], rgba[1], rgba[2], a],
                );
            }
        }
        pen_x += metrics.advance_width;
    }
}
