//! LivingLife food / heat HUD (L-HUD).
//!
//! C++: `LivingLifePage` hunger boxes + temp arrows
//! (`hungerBoxes.tga`, `hungerBoxFills.tga`, `tempArrows.tga`, …).
//! Residual chrome: yumSlip1–4, full/hungry/starving slips (slide + wiggle +
//! hunger.aiff), homeArrows, home-pos stack wire, temp-meter food_time tip,
//! FX `responsible_id` deferral (session).
//! Fonts (pencil / pencil-erased / handwriting) load from OLHU format-2 cache
//! or TGA; 5×7 glyphs when missing.
//! Wire: FX (`FoodChange`) + HX (`HeatChange`) already applied on session;
//! this module turns those fields into screen-space draw + max-capacity peaks.
//!
//! Haxe client has no full LivingLife HUD — C++ is the fidelity source.
//! Assets live under `OneLifeGameSourceData/graphics/` (not OLC1 content).
//! Prefer real `graphics/*.tga` when present; procedural fallback otherwise.
//!
//! Chrome is **not** OLC1/OLT1 — OLHU cache / TGA / procedural. Soft-FB only; min deps.

use std::path::{Path, PathBuf};

use crate::parse::{FoodChange, HeatChange};
use crate::render::Framebuffer;
use crate::tga::{load_tga_path, RgbaImage};

/// C++ `NUM_HUNGER_BOX_SPRITES` — strip frames in hunger box TGAs.
pub const NUM_HUNGER_BOX_SPRITES: usize = 20;
/// C++ `NUM_TEMP_ARROWS`.
pub const NUM_TEMP_ARROWS: usize = 6;
/// C++ `NUM_HUNGER_DASHES` (bars + dashes strips).
pub const NUM_HUNGER_DASHES: usize = 6;
/// C++ `NUM_HOME_ARROWS` (N, NE, E, SE, S, SW, W, NW).
pub const NUM_HOME_ARROWS: usize = 8;
/// C++ `NUM_YUM_SLIPS` (yumSlip1..4).
pub const NUM_YUM_SLIPS: usize = 4;
/// Hunger slip slots: full / hungry / starving.
pub const NUM_HUNGER_SLIPS: usize = 3;

/// Official client design view (Jason default ~1280×720). Layout offsets are in these units.
pub const HUD_DESIGN_W: f32 = 1280.0;
pub const HUD_DESIGN_H: f32 = 720.0;

/// First hunger-box center relative to screen center (C++ Y-up → we store as +y = below).
/// C++: `lastScreenViewCenter.x - 590`, `lastScreenViewCenter.y - 334`.
pub const HUNGER_BOX_ORIGIN_X: f32 = -590.0;
pub const HUNGER_BOX_ORIGIN_Y_BELOW: f32 = 334.0;
/// Horizontal pitch between capacity slots.
pub const HUNGER_BOX_PITCH: f32 = 30.0;

/// Temp-arrow base relative to center (C++ `+546, -319` Y-up).
pub const TEMP_ARROW_ORIGIN_X: f32 = 546.0;
pub const TEMP_ARROW_ORIGIN_Y_BELOW: f32 = 319.0;
/// Heat 0..1 maps across this span: `(heat - 0.5) * TEMP_ARROW_SPAN`.
pub const TEMP_ARROW_SPAN: f32 = 120.0;

/// Yum bonus text origin (C++ `-480, -313` Y-up).
pub const YUM_ORIGIN_X: f32 = -480.0;
pub const YUM_ORIGIN_Y_BELOW: f32 = 313.0;

/// Last-ate string origin (C++ `0, -347` Y-up).
pub const ATE_ORIGIN_X: f32 = 0.0;
pub const ATE_ORIGIN_Y_BELOW: f32 = 347.0;

/// Curse token "C+X" origin (C++ `+621, -316` Y-up).
pub const CURSE_TOKEN_ORIGIN_X: f32 = 621.0;
pub const CURSE_TOKEN_ORIGIN_Y_BELOW: f32 = 316.0;

/// Gui panel center Y offset below view center (C++ `242+32+16+6`).
pub const GUI_PANEL_Y_BELOW: f32 = 296.0;

/// C++ `drawHungerMaxFillLine` bar offset from box center: `-12 x`, `-10 y` (Y-up).
pub const HUNGER_BAR_OFFSET_X: f32 = -12.0;
/// Y-down equivalent of C++ `-10` on Y-up axis.
pub const HUNGER_BAR_OFFSET_Y_BELOW: f32 = 10.0;

/// C++ old-arrow fade step per heat-delta event.
pub const OLD_ARROW_FADE_STEP: f32 = 0.01;
/// Max ghost arrows kept (soft cap; C++ vector unbounded but short-lived).
pub const MAX_OLD_ARROWS: usize = 32;

/// C++ `pencilErasedFontExtraFade` (0.75) — erased pencil text dimming.
pub const PENCIL_ERASED_FADE: f32 = 0.75;
/// C++ Font ink alpha threshold for pseudo-kerning.
const FONT_INK_A: u8 = 127;
/// C++ `Font` global `scaleFactor` (1/16) × pencil `mScaleFactor` 16 → 1.0.
pub const PENCIL_BASE_SCALE: f32 = 1.0;
/// C++ pencilFont ctor: charSpacing=3, spaceWidth=6.
pub const PENCIL_CHAR_SPACING: i32 = 3;
pub const PENCIL_SPACE_WIDTH: i32 = 6;

/// C++ yum slip hide offsets (Y-up → +y below).
pub const YUM_SLIP_HIDE_X: f32 = -600.0;
pub const YUM_SLIP_HIDE_Y_BELOW: f32 = 330.0;
/// Pop-in show delta (C++ target.y += 36 on Y-up → show is higher = less below).
pub const YUM_SLIP_SHOW_DY: f32 = 36.0;

/// C++ hunger slip show/hide (Y-up → +y below). Base show y = -250 → 250 below.
pub const HUNGER_SLIP_X: f32 = -558.0;
/// C++ `mHungerSlipShowOffsets` Y-up −250 / −250 / −280 → below 250 / 250 / 280.
pub const HUNGER_SLIP_SHOW_Y: [f32; 3] = [250.0, 250.0, 280.0]; // full / hungry / starving
/// C++ hide −370 / −370 / −390 → below 370 / 370 / 390.
pub const HUNGER_SLIP_HIDE_Y: [f32; 3] = [370.0, 370.0, 390.0];
/// C++ `mHungerSlipWiggleAmp` — full 0, hungry/starving 0.5.
pub const HUNGER_SLIP_WIGGLE_AMP: [f32; 3] = [0.0, 0.5, 0.5];
/// C++ `mHungerSlipWiggleSpeed`.
pub const HUNGER_SLIP_WIGGLE_SPEED: f32 = 0.05;

/// C++ home arrow strip drawn on home slip near bottom-center.
pub const HOME_ARROW_ORIGIN_X: f32 = -41.0;
pub const HOME_ARROW_ORIGIN_Y_BELOW: f32 = 292.0;

/// Result of [`HudState::step_slips`] — hunger.aiff trigger (C++ draw peak / FX).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HungerSoundEvent {
    #[default]
    None,
    /// One-shot play (food entered hungry/starving band with store > 1).
    OneShot,
    /// Pulse at starving-slip wiggle peak while `pulse_hunger_sound`.
    Pulse,
}

/// Center tip line (C++ tipPos y -313 Y-up).
pub const TIP_ORIGIN_Y_BELOW: f32 = 313.0;

/// Max old yum / last-ate fade stack entries kept.
pub const MAX_OLD_TEXT_STACK: usize = 8;
/// Fade step applied each FX when values change.
pub const OLD_TEXT_FADE_STEP: f32 = 0.05;

// --- layout (pure, headless-safe) -------------------------------------------

/// Scale factor so design-space HUD fits the framebuffer (letterbox-safe).
pub fn hud_scale(fb_w: u32, fb_h: u32) -> f32 {
    let sx = fb_w as f32 / HUD_DESIGN_W;
    let sy = fb_h as f32 / HUD_DESIGN_H;
    sx.min(sy).max(0.05)
}

/// Screen-pixel position for hunger box slot `i` (0-based), Y grows down.
pub fn hunger_box_screen_pos(i: i32, fb_w: u32, fb_h: u32) -> (f32, f32) {
    let s = hud_scale(fb_w, fb_h);
    let cx = fb_w as f32 * 0.5;
    let cy = fb_h as f32 * 0.5;
    let x = cx + (HUNGER_BOX_ORIGIN_X + i as f32 * HUNGER_BOX_PITCH) * s;
    let y = cy + HUNGER_BOX_ORIGIN_Y_BELOW * s;
    (x, y)
}

/// Screen-pixel position for the temperature arrow given heat in `[0, 1]`.
///
/// C++: `pos.x += (heat - 0.5) * 120` then `round`.
pub fn temp_arrow_screen_pos(heat: f32, fb_w: u32, fb_h: u32) -> (f32, f32) {
    let s = hud_scale(fb_w, fb_h);
    let cx = fb_w as f32 * 0.5;
    let cy = fb_h as f32 * 0.5;
    let heat = heat.clamp(0.0, 1.0);
    let x = (cx + (TEMP_ARROW_ORIGIN_X + (heat - 0.5) * TEMP_ARROW_SPAN) * s).round();
    let y = cy + TEMP_ARROW_ORIGIN_Y_BELOW * s;
    (x, y)
}

/// Yum `+N` text origin (left-aligned in C++).
pub fn yum_screen_pos(fb_w: u32, fb_h: u32) -> (f32, f32) {
    let s = hud_scale(fb_w, fb_h);
    let cx = fb_w as f32 * 0.5;
    let cy = fb_h as f32 * 0.5;
    (cx + YUM_ORIGIN_X * s, cy + YUM_ORIGIN_Y_BELOW * s)
}

/// Last-ate words origin (left-aligned in C++).
pub fn ate_screen_pos(fb_w: u32, fb_h: u32) -> (f32, f32) {
    let s = hud_scale(fb_w, fb_h);
    let cx = fb_w as f32 * 0.5;
    let cy = fb_h as f32 * 0.5;
    (cx + ATE_ORIGIN_X * s, cy + ATE_ORIGIN_Y_BELOW * s)
}

/// Curse token sigil origin (center-aligned in C++).
pub fn curse_token_screen_pos(fb_w: u32, fb_h: u32) -> (f32, f32) {
    let s = hud_scale(fb_w, fb_h);
    let cx = fb_w as f32 * 0.5;
    let cy = fb_h as f32 * 0.5;
    (
        cx + CURSE_TOKEN_ORIGIN_X * s,
        cy + CURSE_TOKEN_ORIGIN_Y_BELOW * s,
    )
}

// --- state ------------------------------------------------------------------

/// C++ `OldArrow` — ghost temp-arrow left behind when heat moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OldArrow {
    /// Strip frame index used when this ghost was current.
    pub i: usize,
    /// Heat value at ghost spawn (0..1).
    pub heat: f32,
    /// 1.0 = solid ghost; steps down by [`OLD_ARROW_FADE_STEP`] on later heat deltas.
    pub fade: f32,
}

/// Fading stack entry for erased yum bonus / last-ate labels.
#[derive(Debug, Clone, PartialEq)]
pub struct OldHudText {
    pub text: String,
    pub fade: f32,
    /// Optional max-fill for old last-ate bar ghosts.
    pub fill_max: i32,
}

/// Client-side HUD vitals derived from FX/HX (+ max peaks for erased boxes).
///
/// C++ LiveObject: `foodStore`, `foodCapacity`, `maxFoodStore`, `maxFoodCapacity`, `heat`.
#[derive(Debug, Clone, PartialEq)]
pub struct HudState {
    pub food_store: i32,
    pub food_capacity: i32,
    pub max_food_store: i32,
    pub max_food_capacity: i32,
    pub last_ate_id: i32,
    pub last_ate_fill_max: i32,
    pub move_speed: f32,
    pub yum_bonus: i32,
    pub yum_multiplier: i32,
    /// Normalized heat 0..1 (HX / PU heat).
    pub heat: f32,
    pub food_time: f32,
    pub indoor_bonus: f32,
    /// C++ `mCurrentArrowI` — which temp-arrow strip frame is current.
    pub arrow_i: usize,
    /// Sentinel `-1.0` until first heat drawn (C++ `mCurrentArrowHeat`).
    pub current_arrow_heat: f32,
    /// C++ `mOldArrows` — fading trail sprites.
    pub old_arrows: Vec<OldArrow>,
    /// C++ curse token count (CX); `None` = not yet received.
    pub curse_tokens: Option<i32>,
    /// C++ excess curse points (CS).
    pub excess_curse_points: i32,
    /// When true, draw guiBlood over panel (dying, not sick). Soft-FB placeholder tint.
    pub dying: bool,
    pub visible: bool,
    /// C++ `mHungerSlipVisible`: -1 none, 0 full, 1 hungry, 2 starving.
    pub hunger_slip_visible: i32,
    /// C++ `mYumSlipNumberToShow[0]` — multiplier shown on yum slip (0 = hidden).
    pub yum_slip_number: i32,
    /// C++ `mYumSlipNumberToShow[0..1]` dual flip slots (0 = empty).
    pub yum_slip_numbers: [i32; 2],
    /// Active yum slip slot index 0..1 (which number is showing).
    pub yum_slip_active: usize,
    /// C++ `mHungerSlipPosOffset[i].y` as +y below screen center.
    pub hunger_slip_pos_y: [f32; NUM_HUNGER_SLIPS],
    /// C++ `mHungerSlipPosTargetOffset[i].y`.
    pub hunger_slip_target_y: [f32; NUM_HUNGER_SLIPS],
    /// C++ `mHungerSlipWiggleTime`.
    pub hunger_slip_wiggle_time: [f32; NUM_HUNGER_SLIPS],
    /// C++ `mStarvingSlipLastPos` for peak-detect pulse sound (harmonic samples).
    pub starving_slip_last_pos: [f32; 2],
    /// C++ `mPulseHungerSound` — play hunger.aiff each starving wiggle peak.
    pub pulse_hunger_sound: bool,
    /// Edge-triggered one-shot hunger.aiff from FX threshold (store > 1).
    pub hunger_sound_oneshot: bool,
    /// C++ `mYumSlipPosOffset[i].y` as +y below (hide 330; show 330−36=294).
    pub yum_slip_pos_y: [f32; 2],
    /// C++ `mYumSlipPosTargetOffset[i].y`.
    pub yum_slip_target_y: [f32; 2],
    /// C++ `mOldYumBonus` + fades (erased pencil stack).
    pub old_yum_bonus: Vec<OldHudText>,
    /// Current last-ate label (`#id` stand-in until object names wired).
    pub current_last_ate_string: Option<String>,
    /// C++ `mOldLastAteStrings` + fades.
    pub old_last_ate: Vec<OldHudText>,
    /// Solid home-arrow index 0..7 (`getHomeDir`); `None` = no home marker.
    pub home_arrow: Option<usize>,
    /// C++ homeSlip2 / `getHomeDir(..., j=1)` ancient homeland.
    pub ancient_home_arrow: Option<usize>,
    /// Erased trail fades for each home arrow frame (C++ `HomeArrow.fade`).
    pub home_arrow_fades: [f32; NUM_HOME_ARROWS],
    /// P3#17: pencil label under home arrows (`MAP` / `BABY` / `LEAD` / …).
    ///
    /// // C++ `drawHomeSlip` tempPersonKey / "map" string beside arrow strip.
    pub map_pointer_label: Option<String>,
    /// C++ `hideGuiPanel` / `hideGameUI`.
    pub hide_gui: bool,
    /// Screen-space pointer for temp-meter hover tip (design-independent pixels).
    pub pointer_x: f32,
    pub pointer_y: f32,
    /// When true, pointer is valid this frame.
    pub pointer_valid: bool,
    /// Approximate age years for hunger-slip thresholds (adult default).
    pub age_years: f32,
    /// C++ `mBadBiomeNames` / object tip at screen-center (empty = none).
    pub hover_tip: Option<String>,
    /// C++ craving sheet (`setNewCraving`) — `CRAVING: NAME (+N)`.
    pub craving_text: Option<String>,
    /// Typed SAY draft while the note paper is up (`Some` even if empty = focused).
    pub say_draft: Option<String>,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            food_store: 0,
            food_capacity: 0,
            max_food_store: 0,
            max_food_capacity: 0,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 0.0,
            yum_bonus: 0,
            yum_multiplier: 0,
            heat: 0.5,
            food_time: 0.0,
            indoor_bonus: 0.0,
            arrow_i: 0,
            current_arrow_heat: -1.0,
            old_arrows: Vec::new(),
            curse_tokens: None,
            excess_curse_points: 0,
            dying: false,
            visible: false,
            hunger_slip_visible: -1,
            yum_slip_number: 0,
            yum_slip_numbers: [0, 0],
            yum_slip_active: 0,
            hunger_slip_pos_y: HUNGER_SLIP_HIDE_Y,
            hunger_slip_target_y: HUNGER_SLIP_HIDE_Y,
            hunger_slip_wiggle_time: [0.0; NUM_HUNGER_SLIPS],
            starving_slip_last_pos: [0.0, 0.0],
            pulse_hunger_sound: false,
            hunger_sound_oneshot: false,
            yum_slip_pos_y: [YUM_SLIP_HIDE_Y_BELOW; 2],
            yum_slip_target_y: [YUM_SLIP_HIDE_Y_BELOW; 2],
            old_yum_bonus: Vec::new(),
            current_last_ate_string: None,
            old_last_ate: Vec::new(),
            home_arrow: None,
            ancient_home_arrow: None,
            home_arrow_fades: [0.0; NUM_HOME_ARROWS],
            map_pointer_label: None,
            hide_gui: false,
            pointer_x: 0.0,
            pointer_y: 0.0,
            pointer_valid: false,
            age_years: 20.0,
            hover_tip: None,
            craving_text: None,
            say_draft: None,
        }
    }
}

/// C++ `setNewCraving` sheet text: `CRAVING: NAME (+bonus)`.
pub fn craving_hud_line(content: &crate::content::ClientContent, food_id: i32, bonus: i32) -> String {
    let name = content
        .get(food_id)
        .map(object_hud_name)
        .unwrap_or_else(|| format!("#{food_id}"));
    format!("CRAVING: {} (+{bonus})", name.to_ascii_uppercase())
}

/// First token of object name/description without `#tags` (C++ HUD last-ate string).
pub fn object_hud_name(def: &crate::content::ClientObjectDef) -> String {
    let raw = if def.name.is_empty() {
        def.description.as_str()
    } else {
        def.name.as_str()
    };
    let token = raw
        .split(|c: char| c == '#' || c == '\n')
        .next()
        .unwrap_or("")
        .trim();
    if token.is_empty() {
        format!("#{}", def.id)
    } else {
        token.to_string()
    }
}

impl HudState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply FX food change. Tracks max peaks like LivingLifePage FX handler.
    pub fn apply_fx(&mut self, f: &FoodChange) {
        let old_yum = self.yum_bonus;
        let old_mult = self.yum_multiplier;
        let old_ate = self.current_last_ate_string.clone();
        let old_fill = self.last_ate_fill_max;

        self.food_store = f.food_store.max(0);
        self.food_capacity = f.food_capacity.max(0);
        self.last_ate_id = f.last_ate_id;
        self.last_ate_fill_max = f.last_ate_fill_max;
        self.move_speed = f.move_speed;
        self.yum_bonus = f.yum_bonus;
        self.yum_multiplier = f.yum_multiplier;
        if self.food_store > self.max_food_store {
            self.max_food_store = self.food_store;
        }
        if self.food_capacity > self.max_food_capacity {
            self.max_food_capacity = self.food_capacity;
        }
        // Capacity can drop (aging); keep max for erased trailing boxes.
        if self.max_food_capacity < self.food_capacity {
            self.max_food_capacity = self.food_capacity;
        }

        // C++ mOldYumBonus stack when bonus changes.
        if old_yum != self.yum_bonus {
            Self::fade_old_text_stack(&mut self.old_yum_bonus);
            if old_yum != 0 {
                self.old_yum_bonus.push(OldHudText {
                    text: format!("+{old_yum}"),
                    fade: 1.0,
                    fill_max: 0,
                });
            }
            Self::cap_old_text_stack(&mut self.old_yum_bonus);
        }

        // Yum slip dual flip (C++ mYumSlipNumberToShow + target y += 36).
        if old_mult != self.yum_multiplier {
            self.set_yum_slip_multiplier(self.yum_multiplier, old_mult);
        } else if self.yum_multiplier > 0 && self.yum_slip_number == 0 {
            self.set_yum_slip_multiplier(self.yum_multiplier, 0);
        }

        // Last-ate label — `#id` until [`Self::resolve_last_ate_name`] (content names).
        let new_ate = if f.last_ate_id > 0 {
            Some(format!("#{}", f.last_ate_id))
        } else {
            None
        };
        if old_ate != new_ate {
            Self::fade_old_text_stack(&mut self.old_last_ate);
            if let Some(prev) = old_ate {
                if !prev.is_empty() {
                    self.old_last_ate.push(OldHudText {
                        text: prev,
                        fade: 1.0,
                        fill_max: old_fill,
                    });
                }
            }
            Self::cap_old_text_stack(&mut self.old_last_ate);
            self.current_last_ate_string = new_ate;
        }

        self.recompute_hunger_slip();
        self.visible = true;
    }

    /// Replace `#id` last-ate stand-in with object display name (C++ getObject description).
    pub fn resolve_last_ate_name(&mut self, content: &crate::content::ClientContent) {
        if self.last_ate_id <= 0 {
            return;
        }
        let name = content
            .get(self.last_ate_id)
            .map(object_hud_name)
            .unwrap_or_else(|| format!("#{}", self.last_ate_id));
        if self.current_last_ate_string.as_deref() == Some(name.as_str()) {
            return;
        }
        // Don't push fade stack — this is the same eat event, just resolved.
        self.current_last_ate_string = Some(name);
    }

    /// C++ yum multiplier slip flip — hide old slot, show new at hide.y−36.
    fn set_yum_slip_multiplier(&mut self, new_mult: i32, old_mult: i32) {
        let mut old_slip = -1i32;
        let mut new_slip = 0usize;
        for i in 0..2 {
            if self.yum_slip_numbers[i] == old_mult && old_mult != 0 {
                old_slip = i as i32;
                new_slip = (i + 1) % 2;
                break;
            }
        }
        if old_slip >= 0 {
            let oi = old_slip as usize;
            self.yum_slip_target_y[oi] = YUM_SLIP_HIDE_Y_BELOW;
        }
        self.yum_slip_target_y[new_slip] = YUM_SLIP_HIDE_Y_BELOW;
        if new_mult > 0 {
            self.yum_slip_target_y[new_slip] = YUM_SLIP_HIDE_Y_BELOW - YUM_SLIP_SHOW_DY;
        }
        self.yum_slip_numbers[new_slip] = new_mult;
        self.yum_slip_active = new_slip;
        self.yum_slip_number = new_mult.max(0);
    }

    /// C++ FX hunger-slip thresholds + pulse/oneshot hunger.aiff flags.
    ///
    /// // C++ LivingLifePage FOOD_CHANGE ~22017–22085
    pub fn recompute_hunger_slip(&mut self) {
        let store = self.food_store;
        let cap = self.food_capacity;
        let eff = store + self.yum_bonus;
        let age = self.age_years;
        let prev_visible = self.hunger_slip_visible;
        self.hunger_sound_oneshot = false;

        if age < 3.0 {
            // Baby: full unless near-empty → starving.
            if eff <= 2 {
                self.hunger_slip_visible = 2;
                self.pulse_hunger_sound = true;
            } else {
                self.hunger_slip_visible = 0;
                self.pulse_hunger_sound = false;
            }
        } else if cap > 0 && store == cap {
            self.pulse_hunger_sound = false;
            self.hunger_slip_visible = 0; // full
        } else if eff <= 4 && age >= 57.33 {
            // End of life: starving chrome, no hunger sound (song).
            self.hunger_slip_visible = 2;
            self.pulse_hunger_sound = false;
        } else if eff <= 4 {
            self.hunger_slip_visible = 2; // starving
            if store > 0 {
                if store > 1 {
                    // One-shot hunger.aiff (not pulse).
                    self.hunger_sound_oneshot = true;
                    self.pulse_hunger_sound = false;
                } else {
                    self.pulse_hunger_sound = true;
                }
            } else {
                self.pulse_hunger_sound = false;
            }
        } else if eff <= 8 {
            self.hunger_slip_visible = 1; // hungry
            self.pulse_hunger_sound = false;
        } else {
            self.hunger_slip_visible = -1;
            self.pulse_hunger_sound = false;
        }

        if eff > 4 || age >= 57.0 {
            self.pulse_hunger_sound = false;
        }

        // Snap targets for newly visible / hide others (step_slips animates).
        let _ = prev_visible;
        for i in 0..NUM_HUNGER_SLIPS {
            if self.hunger_slip_visible == i as i32 {
                self.hunger_slip_target_y[i] = HUNGER_SLIP_SHOW_Y[i];
            } else {
                self.hunger_slip_target_y[i] = HUNGER_SLIP_HIDE_Y[i];
            }
        }
    }

    /// C++ per-frame hunger/yum slip slide + wiggle (LivingLifePage step ~14550).
    ///
    /// `frame_rate_factor` ≈ wall frames at 60 Hz (`dt * 60`). Returns hunger
    /// sound event for [`crate::sound_bank::SoundBank::play_hunger_sound`].
    pub fn step_slips(&mut self, frame_rate_factor: f32) -> HungerSoundEvent {
        let frf = frame_rate_factor.max(0.0);
        let mut sound = HungerSoundEvent::None;
        if self.hunger_sound_oneshot {
            sound = HungerSoundEvent::OneShot;
            self.hunger_sound_oneshot = false;
        }

        // Hide non-visible first; only raise visible when others are down.
        let mut any_moving_down = false;
        for i in 0..NUM_HUNGER_SLIPS {
            if self.hunger_slip_visible != i as i32
                && (self.hunger_slip_pos_y[i] - HUNGER_SLIP_HIDE_Y[i]).abs() > 0.5
            {
                self.hunger_slip_target_y[i] = HUNGER_SLIP_HIDE_Y[i];
                any_moving_down = true;
            }
        }
        if !any_moving_down {
            if self.hunger_slip_visible >= 0 {
                let vi = self.hunger_slip_visible as usize;
                if vi < NUM_HUNGER_SLIPS {
                    self.hunger_slip_target_y[vi] = HUNGER_SLIP_SHOW_Y[vi];
                }
            }
        }

        for i in 0..NUM_HUNGER_SLIPS {
            let target = self.hunger_slip_target_y[i];
            let pos = self.hunger_slip_pos_y[i];
            if (pos - target).abs() > 0.01 {
                let d = (target - pos).abs();
                if d <= 1.0 {
                    self.hunger_slip_pos_y[i] = target;
                } else {
                    let mut speed = frf * 4.0;
                    if d < 8.0 {
                        speed = (frf * d / 2.0).round();
                    }
                    if speed > d {
                        speed = d.floor();
                    }
                    if speed < 1.0 {
                        speed = 1.0;
                    }
                    let dir = if target > pos { 1.0 } else { -1.0 };
                    self.hunger_slip_pos_y[i] = pos + dir * speed;
                }
                if (self.hunger_slip_target_y[i] - HUNGER_SLIP_HIDE_Y[i]).abs() < 0.5 {
                    self.hunger_slip_wiggle_time[i] = 0.0;
                }
            }
            if (self.hunger_slip_pos_y[i] - HUNGER_SLIP_HIDE_Y[i]).abs() > 0.5 {
                self.hunger_slip_wiggle_time[i] += frf * HUNGER_SLIP_WIGGLE_SPEED;
            }
        }

        // Starving peak pulse (C++ draw path ~10440–10460).
        if self.hunger_slip_visible == 2 {
            let i = 2;
            let amp = HUNGER_SLIP_WIGGLE_AMP[i];
            if amp > 0.0 {
                // Design Y-below: show is less below than hide → dist = hide - pos.
                let dist_from_hidden =
                    (HUNGER_SLIP_HIDE_Y[i] - self.hunger_slip_pos_y[i]).max(0.0);
                let harmonic = (0.5 * (1.0 - self.hunger_slip_wiggle_time[i].cos()))
                    * amp
                    * dist_from_hidden;
                let last0 = self.starving_slip_last_pos[0];
                let last1 = self.starving_slip_last_pos[1];
                if last0 != 0.0 || last1 != 0.0 {
                    let last_dir = last1 - last0;
                    if last_dir > 0.0 {
                        let new_dir = harmonic - last1;
                        if new_dir < 0.0 && self.pulse_hunger_sound {
                            if sound == HungerSoundEvent::None {
                                sound = HungerSoundEvent::Pulse;
                            }
                        }
                    }
                }
                self.starving_slip_last_pos[0] = last1;
                self.starving_slip_last_pos[1] = harmonic;
            }
        }

        // Yum slip slide (C++ ~13940).
        for i in 0..2 {
            let target = self.yum_slip_target_y[i];
            let pos = self.yum_slip_pos_y[i];
            if (pos - target).abs() > 0.01 {
                let d = (target - pos).abs();
                if d <= 1.0 {
                    self.yum_slip_pos_y[i] = target;
                } else {
                    let mut speed = frf * 4.0;
                    if d < 8.0 {
                        speed = (frf * d / 2.0).round();
                    }
                    if speed > d {
                        speed = d.floor();
                    }
                    if speed < 1.0 {
                        speed = 1.0;
                    }
                    let dir = if target > pos { 1.0 } else { -1.0 };
                    self.yum_slip_pos_y[i] = pos + dir * speed;
                }
            }
        }

        sound
    }

    /// Design-space Y below center for hunger slip `i` including wiggle (draw).
    pub fn hunger_slip_draw_y_below(&self, i: usize) -> f32 {
        if i >= NUM_HUNGER_SLIPS {
            return HUNGER_SLIP_HIDE_Y[0];
        }
        let mut y = self.hunger_slip_pos_y[i];
        let amp = HUNGER_SLIP_WIGGLE_AMP[i];
        if amp > 0.0 {
            let dist_from_hidden = (HUNGER_SLIP_HIDE_Y[i] - self.hunger_slip_pos_y[i]).max(0.0);
            let harmonic = (0.5 * (1.0 - self.hunger_slip_wiggle_time[i].cos()))
                * amp
                * dist_from_hidden;
            // C++ Y-up: slipPos.y += harmonic (up). Our +y below: subtract.
            y -= harmonic;
        }
        y
    }

    fn fade_old_text_stack(stack: &mut Vec<OldHudText>) {
        let mut i = 0;
        while i < stack.len() {
            stack[i].fade -= OLD_TEXT_FADE_STEP;
            if stack[i].fade <= 0.0 {
                stack.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn cap_old_text_stack(stack: &mut Vec<OldHudText>) {
        if stack.len() > MAX_OLD_TEXT_STACK {
            let drop = stack.len() - MAX_OLD_TEXT_STACK;
            stack.drain(0..drop);
        }
    }

    /// Screen pointer for temp-meter hover (call each frame from GUI path).
    pub fn set_pointer(&mut self, x: f32, y: f32) {
        self.pointer_x = x;
        self.pointer_y = y;
        self.pointer_valid = true;
    }

    pub fn clear_pointer(&mut self) {
        self.pointer_valid = false;
    }

    /// C++ `getHomeDir` solid arrow index, or `None` to clear.
    pub fn set_home_arrow(&mut self, dir: Option<usize>) {
        // Fade previous solid arrows.
        for i in 0..NUM_HOME_ARROWS {
            if self.home_arrow == Some(i) {
                // keep solid until replaced
            } else if self.home_arrow_fades[i] > 0.0 {
                self.home_arrow_fades[i] = (self.home_arrow_fades[i] - 0.0625).max(0.0);
            }
        }
        if let Some(d) = dir {
            let d = d % NUM_HOME_ARROWS;
            // Demote previous solid to erased trail.
            if let Some(prev) = self.home_arrow {
                if prev != d {
                    self.home_arrow_fades[prev] = 1.0;
                }
            }
            self.home_arrow = Some(d);
            self.home_arrow_fades[d] = 1.0;
        } else {
            if let Some(prev) = self.home_arrow {
                self.home_arrow_fades[prev] = 1.0;
            }
            self.home_arrow = None;
        }
    }

    /// True when pointer is over the temp meter strip (C++ ~tipPos + 480..607).
    pub fn pointer_over_temp_meter(&self, fb_w: u32, fb_h: u32) -> bool {
        if !self.pointer_valid {
            return false;
        }
        let s = hud_scale(fb_w, fb_h);
        let cx = fb_w as f32 * 0.5;
        let cy = fb_h as f32 * 0.5;
        let tip_y = cy + TIP_ORIGIN_Y_BELOW * s;
        let x0 = cx + 480.0 * s;
        let x1 = cx + 607.0 * s;
        let y0 = tip_y - 13.0 * s;
        let y1 = tip_y + 13.0 * s;
        self.pointer_x >= x0
            && self.pointer_x <= x1
            && self.pointer_y >= y0
            && self.pointer_y <= y1
    }

    /// Soft-FB English stand-in for C++ `foodTimeFormatString` / indoor bonus.
    pub fn temp_meter_tip_text(&self) -> Option<String> {
        if self.food_time <= 0.0 {
            return None;
        }
        let mut main = self.food_time;
        let indoor = if self.indoor_bonus > 0.0 {
            main -= self.indoor_bonus;
            format!(" (+{:.0}s indoors)", self.indoor_bonus)
        } else {
            String::new()
        };
        Some(format!("FOOD TIME: {:.0}s{indoor}", main.max(0.0)))
    }

    /// Apply HX heat change (values only).
    ///
    /// C++ rotates `mCurrentArrowI` and pushes `OldArrow` at **draw** time when
    /// `mCurrentArrowHeat != heat` — see [`Self::prepare_temp_arrow`].
    pub fn apply_hx(&mut self, h: &HeatChange) {
        self.heat = h.heat.clamp(0.0, 1.0);
        self.food_time = h.food_time;
        self.indoor_bonus = h.indoor_bonus;
        self.visible = true;
    }

    /// C++ draw-path heat-delta: push ghost, step fades, advance strip index.
    ///
    /// Call once per frame before drawing the arrow (idempotent when heat stable).
    pub fn prepare_temp_arrow(&mut self) {
        let new_heat = self.heat.clamp(0.0, 1.0);
        if self.current_arrow_heat < 0.0 {
            // First draw: latch without trail.
            self.current_arrow_heat = new_heat;
            return;
        }
        if (self.current_arrow_heat - new_heat).abs() <= 1e-6 {
            return;
        }

        // C++: fade existing ghosts by 0.01 per heat-delta event.
        let mut i = 0;
        while i < self.old_arrows.len() {
            self.old_arrows[i].fade -= OLD_ARROW_FADE_STEP;
            if self.old_arrows[i].fade < 0.0 {
                self.old_arrows.remove(i);
            } else {
                i += 1;
            }
        }

        self.old_arrows.push(OldArrow {
            i: self.arrow_i % NUM_TEMP_ARROWS,
            heat: self.current_arrow_heat,
            fade: 1.0,
        });
        if self.old_arrows.len() > MAX_OLD_ARROWS {
            let drop = self.old_arrows.len() - MAX_OLD_ARROWS;
            self.old_arrows.drain(0..drop);
        }

        self.arrow_i = (self.arrow_i + 1) % NUM_TEMP_ARROWS;
        self.current_arrow_heat = new_heat;
    }

    /// Sync from optional session FX/HX (idempotent peak tracking if re-applied).
    pub fn sync_from_session(&mut self, food: Option<&FoodChange>, heat: Option<&HeatChange>) {
        if let Some(f) = food {
            self.apply_fx(f);
        }
        if let Some(h) = heat {
            self.apply_hx(h);
        }
    }

    /// Apply CX curse token count (sigil right of temp meter).
    pub fn apply_curse_tokens(&mut self, count: i32) {
        self.curse_tokens = Some(count.max(0));
        self.visible = true;
    }

    /// Apply CS excess curse points.
    pub fn apply_excess_curse_points(&mut self, points: i32) {
        self.excess_curse_points = points.max(0);
        if points > 0 {
            self.visible = true;
        }
    }

    /// C++ `setNewCraving` — uppercase food name + yum bonus.
    pub fn apply_craving(&mut self, text: Option<String>) {
        self.craving_text = text.filter(|s| !s.is_empty());
        if self.craving_text.is_some() {
            self.visible = true;
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Effective fill drawn in capacity slots: store only
    /// (C++ draws store fills and yum as separate text).
    pub fn display_fill(&self) -> i32 {
        self.food_store.max(0)
    }
}

// --- sprites ----------------------------------------------------------------

/// One strip-sliced HUD sprite (RGBA, independent of content SpriteBank).
#[derive(Debug, Clone)]
pub struct HudStripSprite {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl HudStripSprite {
    pub fn from_rgba(img: &RgbaImage) -> Self {
        Self {
            width: img.width,
            height: img.height,
            pixels: img.pixels.clone(),
        }
    }

    /// C++ `fillWhiteSprite` / `loadWhiteSprite`: red → alpha, RGB white.
    ///
    /// Used for `chalkBlot.tga` (speech underlay).
    pub fn from_white_sprite(img: &RgbaImage) -> Self {
        let n = (img.width * img.height) as usize;
        let mut pixels = vec![0u8; n * 4];
        for i in 0..n {
            let si = i * 4;
            let a = img.pixels.get(si).copied().unwrap_or(0);
            let di = i * 4;
            pixels[di] = 255;
            pixels[di + 1] = 255;
            pixels[di + 2] = 255;
            pixels[di + 3] = a;
        }
        Self {
            width: img.width,
            height: img.height,
            pixels,
        }
    }

    /// Solid procedural chip (fallback when TGA missing).
    pub fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Self {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for px in pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
        Self {
            width: w,
            height: h,
            pixels,
        }
    }
}

/// True pencilFont atlas from `font_pencil_32_32.tga` (C++ `Font` 16×16 grid).
///
/// Red channel → alpha, RGB white; soft-FB tints with draw color (black text).
#[derive(Debug, Clone)]
pub struct PencilFontAtlas {
    pub cell_w: u32,
    pub cell_h: u32,
    /// Index by ASCII code; `None` = blank cell.
    pub glyphs: Vec<Option<HudStripSprite>>,
    pub left_edge: [i32; 256],
    pub char_width: [i32; 256],
    pub char_spacing: i32,
    pub space_width: i32,
    /// Design-space scale (C++ `mScaleFactor/16` = 1.0 for pencil 32).
    pub base_scale: f32,
}

impl PencilFontAtlas {
    /// Build atlas from a 16×16 cell TGA (any size; cell = w/16 × h/16).
    pub fn from_rgba(img: &RgbaImage) -> Option<Self> {
        if img.width < 16 || img.height < 16 {
            return None;
        }
        let cell_w = img.width / 16;
        let cell_h = img.height / 16;
        if cell_w == 0 || cell_h == 0 {
            return None;
        }
        let mut glyphs = Vec::with_capacity(256);
        let mut left_edge = [0i32; 256];
        let mut char_width = [cell_w as i32; 256];
        for i in 0..256 {
            let col = (i % 16) as u32;
            let row = (i / 16) as u32;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let mut pixels = vec![0u8; (cell_w * cell_h * 4) as usize];
            let mut any = false;
            let mut far_left = cell_w as i32;
            let mut far_right = 0i32;
            let mut some_ink = false;
            for y in 0..cell_h {
                for x in 0..cell_w {
                    let src = img.pixel(x0 + x, y0 + y);
                    // C++ Font: red channel → alpha, RGB = white.
                    let a = src[0];
                    let di = ((y * cell_w + x) * 4) as usize;
                    pixels[di] = 255;
                    pixels[di + 1] = 255;
                    pixels[di + 2] = 255;
                    pixels[di + 3] = a;
                    if a != 0 {
                        any = true;
                    }
                    if a > FONT_INK_A {
                        some_ink = true;
                        far_left = far_left.min(x as i32);
                        far_right = far_right.max(x as i32);
                    }
                }
            }
            if any {
                if some_ink {
                    left_edge[i] = far_left;
                    char_width[i] = (far_right - far_left + 1).max(1);
                } else {
                    left_edge[i] = 0;
                    char_width[i] = cell_w as i32;
                }
                glyphs.push(Some(HudStripSprite {
                    width: cell_w,
                    height: cell_h,
                    pixels,
                }));
            } else {
                left_edge[i] = 0;
                char_width[i] = cell_w as i32;
                glyphs.push(None);
            }
        }
        Some(Self {
            cell_w,
            cell_h,
            glyphs,
            left_edge,
            char_width,
            char_spacing: PENCIL_CHAR_SPACING,
            space_width: PENCIL_SPACE_WIDTH,
            base_scale: PENCIL_BASE_SCALE,
        })
    }

    pub fn measure(&self, text: &str, scale: f32) -> f32 {
        let sc = (self.base_scale * scale).max(0.05);
        let mut width = 0.0f32;
        let chars: Vec<char> = text.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            let c = (*ch as u32).min(255) as usize;
            if *ch == ' ' {
                width += self.space_width as f32 * sc;
            } else {
                width += self.char_width[c] as f32 * sc;
            }
            if i + 1 < chars.len() {
                width += self.char_spacing as f32 * sc;
            }
        }
        width
    }

    /// Draw left- or center-aligned string; `rgba` tints white glyph (black text = 0,0,0,a).
    pub fn draw_string(
        &self,
        fb: &mut Framebuffer,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        rgba: [u8; 4],
        align_center: bool,
    ) {
        let sc = (self.base_scale * scale).max(0.05);
        let total = self.measure(text, scale);
        let mut pen_x = if align_center { x - total * 0.5 } else { x };
        // C++ centers glyph sprites; start at left edge of first glyph box center.
        pen_x += self.cell_w as f32 * sc * 0.5;
        for ch in text.chars() {
            let c = (ch as u32).min(255) as usize;
            if ch == ' ' {
                pen_x += self.space_width as f32 * sc + self.char_spacing as f32 * sc;
                continue;
            }
            let mut draw_x = pen_x;
            draw_x -= self.left_edge[c] as f32 * sc;
            if let Some(spr) = self.glyphs.get(c).and_then(|g| g.as_ref()) {
                blit_glyph_tinted(fb, spr, draw_x, y, sc, rgba);
            }
            pen_x += self.char_width[c] as f32 * sc + self.char_spacing as f32 * sc;
        }
    }
}

/// Tint white+alpha glyph with `rgba` (C++ setDrawColor × white sprite).
fn blit_glyph_tinted(
    fb: &mut Framebuffer,
    spr: &HudStripSprite,
    cx: f32,
    cy: f32,
    scale: f32,
    rgba: [u8; 4],
) {
    if spr.width == 0 || spr.height == 0 {
        return;
    }
    let scale = scale.max(0.05);
    let dw = (spr.width as f32 * scale).max(1.0) as i32;
    let dh = (spr.height as f32 * scale).max(1.0) as i32;
    let ox = cx as i32 - dw / 2;
    let oy = cy as i32 - dh / 2;
    for dy in 0..dh {
        for dx in 0..dw {
            let u = (dx * spr.width as i32 / dw.max(1)).clamp(0, spr.width as i32 - 1) as u32;
            let v = (dy * spr.height as i32 / dh.max(1)).clamp(0, spr.height as i32 - 1) as u32;
            let si = ((v * spr.width + u) * 4) as usize;
            if si + 3 >= spr.pixels.len() {
                continue;
            }
            let ga = spr.pixels[si + 3] as f32 / 255.0;
            if ga <= 1e-4 {
                continue;
            }
            let a = ((rgba[3] as f32) * ga).round() as u8;
            if a == 0 {
                continue;
            }
            fb.put(ox + dx, oy + dy, [rgba[0], rgba[1], rgba[2], a]);
        }
    }
}

/// Loaded HUD chrome (strip frames + optional panel + residual slips/fonts).
#[derive(Debug, Clone)]
pub struct HudSprites {
    pub hunger_boxes: Vec<HudStripSprite>,
    pub hunger_fills: Vec<HudStripSprite>,
    pub hunger_boxes_erased: Vec<HudStripSprite>,
    pub hunger_fills_erased: Vec<HudStripSprite>,
    pub temp_arrows: Vec<HudStripSprite>,
    pub temp_arrows_erased: Vec<HudStripSprite>,
    pub hunger_dashes: Vec<HudStripSprite>,
    pub hunger_dashes_erased: Vec<HudStripSprite>,
    pub hunger_bars: Vec<HudStripSprite>,
    pub hunger_bars_erased: Vec<HudStripSprite>,
    pub gui_panel: Option<HudStripSprite>,
    /// C++ `guiBlood.tga` — drawn over panel when dying.
    pub gui_blood: Option<HudStripSprite>,
    /// C++ `pencilFont` (`font_pencil_32_32.tga`).
    pub pencil_font: Option<PencilFontAtlas>,
    /// C++ `pencilErasedFont` (`font_pencil_erased_32_32.tga`).
    pub pencil_font_erased: Option<PencilFontAtlas>,
    /// C++ `yumSlip1..4.tga`.
    pub yum_slips: Vec<HudStripSprite>,
    /// C++ fullSlip / hungrySlip / starvingSlip.
    pub hunger_slips: Vec<HudStripSprite>,
    /// C++ `homeArrows.tga` strip (8 dirs).
    pub home_arrows: Vec<HudStripSprite>,
    pub home_arrows_erased: Vec<HudStripSprite>,
    /// Optional chalk blot for speech (C++ `mChalkBlotSprite` / `chalkBlot.tga`).
    pub chalk_blot: Option<HudStripSprite>,
    /// C++ `handwritingFont` (`font_handwriting_32_32.tga`) — speech / name plates.
    pub handwriting_font: Option<PencilFontAtlas>,
    /// C++ `homeSlip.tga` / `homeSlip2.tga` paper behind home arrows.
    pub home_slips: Vec<HudStripSprite>,
    /// C++ `notePaper.tga` — SAY compose sheet.
    pub note_paper: Option<HudStripSprite>,
    /// True if at least one real TGA was loaded.
    pub from_disk: bool,
    /// True if pencilFont TGA loaded (play-visible text quality).
    pub pencil_from_disk: bool,
    /// True if handwritingFont TGA loaded (speech fidelity).
    pub handwriting_from_disk: bool,
    /// True if chalkBlot.tga loaded from disk (not procedural solid).
    pub chalk_from_disk: bool,
    pub roots: Vec<PathBuf>,
}

impl Default for HudSprites {
    fn default() -> Self {
        Self::procedural()
    }
}

impl HudSprites {
    /// Procedural chips so headless tests / missing assets still draw a meter.
    pub fn procedural() -> Self {
        let box_empty = |i: usize| {
            let shade = 40 + (i as u8 % 5) * 8;
            HudStripSprite::solid(20, 20, [shade, shade, shade, 220])
        };
        let box_fill = |i: usize| {
            let g = 120 + (i as u8 % 5) * 10;
            HudStripSprite::solid(14, 14, [80, g, 60, 255])
        };
        let erased = |i: usize| {
            let shade = 30 + (i as u8 % 4) * 6;
            HudStripSprite::solid(20, 20, [shade, shade, shade, 120])
        };
        let fill_erased = |_| HudStripSprite::solid(14, 14, [60, 60, 50, 100]);
        let arrow = |i: usize| {
            let r = 180 + (i as u8 % 3) * 20;
            HudStripSprite::solid(12, 18, [r, 80, 40, 255])
        };
        let arrow_e = |_| HudStripSprite::solid(12, 18, [100, 60, 40, 80]);
        let dash = |_| HudStripSprite::solid(10, 4, [40, 40, 40, 200]);
        let dash_e = |_| HudStripSprite::solid(10, 4, [35, 35, 35, 100]);
        let bar = |_| HudStripSprite::solid(6, 16, [50, 50, 50, 220]);
        let bar_e = |_| HudStripSprite::solid(6, 16, [40, 40, 40, 100]);
        Self {
            hunger_boxes: (0..NUM_HUNGER_BOX_SPRITES).map(box_empty).collect(),
            hunger_fills: (0..NUM_HUNGER_BOX_SPRITES).map(box_fill).collect(),
            hunger_boxes_erased: (0..NUM_HUNGER_BOX_SPRITES).map(erased).collect(),
            hunger_fills_erased: (0..NUM_HUNGER_BOX_SPRITES).map(fill_erased).collect(),
            temp_arrows: (0..NUM_TEMP_ARROWS).map(arrow).collect(),
            temp_arrows_erased: (0..NUM_TEMP_ARROWS).map(arrow_e).collect(),
            hunger_dashes: (0..NUM_HUNGER_DASHES).map(dash).collect(),
            hunger_dashes_erased: (0..NUM_HUNGER_DASHES).map(dash_e).collect(),
            hunger_bars: (0..NUM_HUNGER_DASHES).map(bar).collect(),
            hunger_bars_erased: (0..NUM_HUNGER_DASHES).map(bar_e).collect(),
            gui_panel: None,
            gui_blood: None,
            pencil_font: None,
            pencil_font_erased: None,
            yum_slips: Vec::new(),
            hunger_slips: Vec::new(),
            home_arrows: Vec::new(),
            home_arrows_erased: Vec::new(),
            chalk_blot: None,
            handwriting_font: None,
            home_slips: Vec::new(),
            note_paper: None,
            from_disk: false,
            pencil_from_disk: false,
            handwriting_from_disk: false,
            chalk_from_disk: false,
            roots: Vec::new(),
        }
    }

    /// Default search roots (`OHOL_GAME_DATA`, OneLifeGameSourceData, …).
    pub fn with_default_roots(content_root: Option<&Path>) -> Self {
        let mut roots = Vec::new();
        if let Ok(p) = std::env::var("OHOL_GAME_DATA") {
            if !p.is_empty() {
                roots.push(PathBuf::from(p));
            }
        }
        roots.push(PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData"));
        if let Some(c) = content_root {
            roots.push(c.to_path_buf());
            if let Some(parent) = c.parent() {
                roots.push(parent.join("OneLifeGameSourceData"));
                roots.push(parent.to_path_buf());
            }
        }
        roots.push(PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"));
        Self::load_from_roots(&roots)
    }

    /// Prefer `cache/olhu_hud.bin` (sliced HUD chrome + fonts); else TGA then write cache.
    pub fn load_prefer_cache(content_root: Option<&Path>) -> Self {
        let cache = content_root
            .map(|c| c.join("cache").join("olhu_hud.bin"))
            .filter(|p| p.exists());
        if let Some(p) = cache {
            if let Ok(mut s) = Self::load_olhu(&p) {
                // OLHU stores strip chrome + fonts; panel / slips / chalk / paper stay TGA.
                let tga = Self::with_default_roots(content_root);
                if s.gui_panel.is_none() {
                    s.gui_panel = tga.gui_panel;
                }
                if s.gui_blood.is_none() {
                    s.gui_blood = tga.gui_blood;
                }
                if s.yum_slips.is_empty() {
                    s.yum_slips = tga.yum_slips;
                }
                if s.hunger_slips.is_empty() {
                    s.hunger_slips = tga.hunger_slips;
                }
                if s.home_slips.is_empty() {
                    s.home_slips = tga.home_slips;
                }
                if s.note_paper.is_none() {
                    s.note_paper = tga.note_paper;
                }
                if s.pencil_font.is_none() {
                    s.pencil_font = tga.pencil_font;
                    s.pencil_font_erased = tga.pencil_font_erased;
                    s.pencil_from_disk = tga.pencil_from_disk;
                }
                if s.handwriting_font.is_none() {
                    s.handwriting_font = tga.handwriting_font;
                    s.handwriting_from_disk = tga.handwriting_from_disk;
                }
                if !s.chalk_from_disk && tga.chalk_from_disk {
                    s.chalk_blot = tga.chalk_blot;
                    s.chalk_from_disk = true;
                }
                if s.pencil_font.is_none() && s.handwriting_font.is_none() {
                    let _ = s.write_olhu(&p);
                }
                return s;
            }
        }
        let s = Self::with_default_roots(content_root);
        if s.from_disk {
            if let Some(root) = content_root {
                let dir = root.join("cache");
                let _ = std::fs::create_dir_all(&dir);
                let _ = s.write_olhu(&dir.join("olhu_hud.bin"));
            }
        }
        s
    }

    pub const OLHU_MAGIC: [u8; 4] = *b"OLHU";
    /// Format 2 = strip groups + pencil / pencil-erased / handwriting font atlases.
    pub const OLHU_FORMAT: u32 = 2;
    pub const OLHU_FORMAT_V1: u32 = 1;

    /// Write sliced strip sprites + HUD fonts for fast boot (format 2).
    pub fn write_olhu(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut buf = Vec::new();
        buf.extend_from_slice(&Self::OLHU_MAGIC);
        buf.extend_from_slice(&Self::OLHU_FORMAT.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // data_version
        let groups: [(&str, &[HudStripSprite]); 12] = [
            ("hunger_boxes", &self.hunger_boxes),
            ("hunger_fills", &self.hunger_fills),
            ("hunger_boxes_erased", &self.hunger_boxes_erased),
            ("hunger_fills_erased", &self.hunger_fills_erased),
            ("temp_arrows", &self.temp_arrows),
            ("temp_arrows_erased", &self.temp_arrows_erased),
            ("hunger_dashes", &self.hunger_dashes),
            ("hunger_dashes_erased", &self.hunger_dashes_erased),
            ("hunger_bars", &self.hunger_bars),
            ("hunger_bars_erased", &self.hunger_bars_erased),
            ("home_arrows", &self.home_arrows),
            ("home_arrows_erased", &self.home_arrows_erased),
        ];
        buf.extend_from_slice(&(groups.len() as u32).to_le_bytes());
        for (name, sprs) in groups {
            let nb = name.as_bytes();
            buf.extend_from_slice(&(nb.len() as u16).to_le_bytes());
            buf.extend_from_slice(nb);
            buf.extend_from_slice(&(sprs.len() as u32).to_le_bytes());
            for s in sprs {
                buf.extend_from_slice(&s.width.to_le_bytes());
                buf.extend_from_slice(&s.height.to_le_bytes());
                buf.extend_from_slice(&(s.pixels.len() as u32).to_le_bytes());
                buf.extend_from_slice(&s.pixels);
            }
        }
        let fonts: [(&str, Option<&PencilFontAtlas>); 3] = [
            ("pencil", self.pencil_font.as_ref()),
            ("pencil_erased", self.pencil_font_erased.as_ref()),
            ("handwriting", self.handwriting_font.as_ref()),
        ];
        let n_fonts = fonts.iter().filter(|(_, f)| f.is_some()).count() as u32;
        buf.extend_from_slice(&n_fonts.to_le_bytes());
        for (name, font) in fonts {
            if let Some(font) = font {
                write_olhu_font(&mut buf, name, font);
            }
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::File::create(path)?;
        f.write_all(&buf)?;
        Ok(())
    }

    pub fn load_olhu(path: &Path) -> std::io::Result<Self> {
        let buf = std::fs::read(path)?;
        if buf.len() < 16 || buf[0..4] != Self::OLHU_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad OLHU magic",
            ));
        }
        let fmt = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        if fmt < Self::OLHU_FORMAT_V1 || fmt > Self::OLHU_FORMAT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "OLHU format",
            ));
        }
        let mut o = 16usize;
        let n_groups = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
        let mut s = Self::procedural();
        s.from_disk = true;
        for _ in 0..n_groups {
            if o + 2 > buf.len() {
                break;
            }
            let nlen = u16::from_le_bytes(buf[o..o + 2].try_into().unwrap()) as usize;
            o += 2;
            if o + nlen + 4 > buf.len() {
                break;
            }
            let name = std::str::from_utf8(&buf[o..o + nlen]).unwrap_or("");
            o += nlen;
            let nspr = u32::from_le_bytes(buf[o..o + 4].try_into().unwrap()) as usize;
            o += 4;
            let mut sprs = Vec::with_capacity(nspr);
            for _ in 0..nspr {
                if o + 12 > buf.len() {
                    break;
                }
                let w = u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
                let h = u32::from_le_bytes(buf[o + 4..o + 8].try_into().unwrap());
                let plen = u32::from_le_bytes(buf[o + 8..o + 12].try_into().unwrap()) as usize;
                o += 12;
                if o + plen > buf.len() {
                    break;
                }
                sprs.push(HudStripSprite {
                    width: w,
                    height: h,
                    pixels: buf[o..o + plen].to_vec(),
                });
                o += plen;
            }
            match name {
                "hunger_boxes" => s.hunger_boxes = sprs,
                "hunger_fills" => s.hunger_fills = sprs,
                "hunger_boxes_erased" => s.hunger_boxes_erased = sprs,
                "hunger_fills_erased" => s.hunger_fills_erased = sprs,
                "temp_arrows" => s.temp_arrows = sprs,
                "temp_arrows_erased" => s.temp_arrows_erased = sprs,
                "hunger_dashes" => s.hunger_dashes = sprs,
                "hunger_dashes_erased" => s.hunger_dashes_erased = sprs,
                "hunger_bars" => s.hunger_bars = sprs,
                "hunger_bars_erased" => s.hunger_bars_erased = sprs,
                "home_arrows" => s.home_arrows = sprs,
                "home_arrows_erased" => s.home_arrows_erased = sprs,
                _ => {}
            }
        }
        if fmt >= 2 && o + 4 <= buf.len() {
            let n_fonts = u32::from_le_bytes(buf[o..o + 4].try_into().unwrap()) as usize;
            o += 4;
            for _ in 0..n_fonts {
                if o + 2 > buf.len() {
                    break;
                }
                let nlen = u16::from_le_bytes(buf[o..o + 2].try_into().unwrap()) as usize;
                o += 2;
                if o + nlen > buf.len() {
                    break;
                }
                let name = std::str::from_utf8(&buf[o..o + nlen]).unwrap_or("");
                o += nlen;
                if let Some(font) = read_olhu_font(&buf, &mut o) {
                    match name {
                        "pencil" => {
                            s.pencil_font = Some(font);
                            s.pencil_from_disk = true;
                        }
                        "pencil_erased" => {
                            s.pencil_font_erased = Some(font);
                        }
                        "handwriting" => {
                            s.handwriting_font = Some(font);
                            s.handwriting_from_disk = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(s)
    }

    pub fn load_from_roots(roots: &[PathBuf]) -> Self {
        let mut s = Self::procedural();
        s.roots = roots.to_vec();
        let mut any = false;

        if let Some(v) = load_strip(roots, "hungerBoxes.tga", NUM_HUNGER_BOX_SPRITES) {
            s.hunger_boxes = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerBoxFills.tga", NUM_HUNGER_BOX_SPRITES) {
            s.hunger_fills = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerBoxesErased.tga", NUM_HUNGER_BOX_SPRITES) {
            s.hunger_boxes_erased = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerBoxFillsErased.tga", NUM_HUNGER_BOX_SPRITES) {
            s.hunger_fills_erased = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "tempArrows.tga", NUM_TEMP_ARROWS) {
            s.temp_arrows = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "tempArrowsErased.tga", NUM_TEMP_ARROWS) {
            s.temp_arrows_erased = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerDashes.tga", NUM_HUNGER_DASHES) {
            s.hunger_dashes = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerDashesErased.tga", NUM_HUNGER_DASHES) {
            s.hunger_dashes_erased = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerBars.tga", NUM_HUNGER_DASHES) {
            s.hunger_bars = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "hungerBarsErased.tga", NUM_HUNGER_DASHES) {
            s.hunger_bars_erased = v;
            any = true;
        }
        if let Some(img) = find_graphics_tga(roots, "guiPanel.tga") {
            s.gui_panel = Some(HudStripSprite::from_rgba(&img));
            any = true;
        }
        if let Some(img) = find_graphics_tga(roots, "guiBlood.tga") {
            s.gui_blood = Some(HudStripSprite::from_rgba(&img));
            any = true;
        }

        // --- residual P1#3 chrome ---
        if let Some(img) = find_graphics_tga(roots, "font_pencil_32_32.tga") {
            if let Some(font) = PencilFontAtlas::from_rgba(&img) {
                s.pencil_font = Some(font);
                s.pencil_from_disk = true;
                any = true;
            }
        }
        if let Some(img) = find_graphics_tga(roots, "font_pencil_erased_32_32.tga") {
            if let Some(mut font) = PencilFontAtlas::from_rgba(&img) {
                // C++ pencilErasedFont->copySpacing(pencilFont)
                if let Some(src) = &s.pencil_font {
                    font.left_edge = src.left_edge;
                    font.char_width = src.char_width;
                    font.char_spacing = src.char_spacing;
                    font.space_width = src.space_width;
                    font.base_scale = src.base_scale;
                }
                s.pencil_font_erased = Some(font);
                any = true;
            }
        }
        let mut yum = Vec::with_capacity(NUM_YUM_SLIPS);
        for i in 1..=NUM_YUM_SLIPS {
            let name = format!("yumSlip{i}.tga");
            if let Some(img) = find_graphics_tga(roots, &name) {
                yum.push(HudStripSprite::from_rgba(&img));
                any = true;
            }
        }
        if yum.len() == NUM_YUM_SLIPS {
            s.yum_slips = yum;
        } else if !yum.is_empty() {
            // Partial load: fill remaining with procedural chips already present.
            for (i, spr) in yum.into_iter().enumerate() {
                if i < s.yum_slips.len() {
                    s.yum_slips[i] = spr;
                }
            }
        }
        let slip_names = ["fullSlip.tga", "hungrySlip.tga", "starvingSlip.tga"];
        let mut hunger_slips = Vec::with_capacity(NUM_HUNGER_SLIPS);
        for name in slip_names {
            if let Some(img) = find_graphics_tga(roots, name) {
                hunger_slips.push(HudStripSprite::from_rgba(&img));
                any = true;
            }
        }
        if hunger_slips.len() == NUM_HUNGER_SLIPS {
            s.hunger_slips = hunger_slips;
        } else if !hunger_slips.is_empty() {
            s.hunger_slips = hunger_slips;
        }
        if let Some(v) = load_strip(roots, "homeArrows.tga", NUM_HOME_ARROWS) {
            s.home_arrows = v;
            any = true;
        }
        if let Some(v) = load_strip(roots, "homeArrowsErased.tga", NUM_HOME_ARROWS) {
            s.home_arrows_erased = v;
            any = true;
        }
        let mut home_slips = Vec::new();
        for name in ["homeSlip.tga", "homeSlip2.tga"] {
            if let Some(img) = find_graphics_tga(roots, name) {
                home_slips.push(HudStripSprite::from_rgba(&img));
                any = true;
            }
        }
        if !home_slips.is_empty() {
            s.home_slips = home_slips;
        }
        if let Some(img) = find_graphics_tga(roots, "notePaper.tga") {
            s.note_paper = Some(HudStripSprite::from_rgba(&img));
            any = true;
        }
        // P3#15 L-SAY: chalk blot + handwriting font (graphics/, not OLC1).
        if let Some(img) = find_graphics_tga(roots, "chalkBlot.tga") {
            s.chalk_blot = Some(HudStripSprite::from_white_sprite(&img));
            s.chalk_from_disk = true;
            any = true;
        }
        // C++ game.cpp: Font("font_handwriting_32_32.tga", 3, 6, false, 16)
        // — same char spacing / scale as pencilFont; reuse PencilFontAtlas.
        if let Some(img) = find_graphics_tga(roots, "font_handwriting_32_32.tga") {
            if let Some(font) = PencilFontAtlas::from_rgba(&img) {
                s.handwriting_font = Some(font);
                s.handwriting_from_disk = true;
                any = true;
            }
        }

        s.from_disk = any;
        s
    }

    /// Draw with true pencil TGA when loaded, else 5×7 bitmap stand-in.
    pub fn draw_hud_text(
        &self,
        fb: &mut Framebuffer,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        rgba: [u8; 4],
        align_center: bool,
        erased: bool,
    ) {
        let mut col = rgba;
        if erased {
            col[3] = ((col[3] as f32) * PENCIL_ERASED_FADE).round() as u8;
        }
        let font = if erased {
            self.pencil_font_erased
                .as_ref()
                .or(self.pencil_font.as_ref())
        } else {
            self.pencil_font.as_ref()
        };
        if let Some(f) = font {
            f.draw_string(fb, text, x, y, scale, col, align_center);
        } else {
            draw_pencil_string(fb, text, x, y, scale, col, align_center);
        }
    }
}

fn write_olhu_font(buf: &mut Vec<u8>, name: &str, font: &PencilFontAtlas) {
    let nb = name.as_bytes();
    buf.extend_from_slice(&(nb.len() as u16).to_le_bytes());
    buf.extend_from_slice(nb);
    buf.extend_from_slice(&font.cell_w.to_le_bytes());
    buf.extend_from_slice(&font.cell_h.to_le_bytes());
    buf.extend_from_slice(&font.char_spacing.to_le_bytes());
    buf.extend_from_slice(&font.space_width.to_le_bytes());
    buf.extend_from_slice(&font.base_scale.to_le_bytes());
    for i in 0..256 {
        match font.glyphs.get(i).and_then(|g| g.as_ref()) {
            Some(g) => {
                buf.push(1);
                buf.extend_from_slice(&(g.pixels.len() as u32).to_le_bytes());
                buf.extend_from_slice(&g.pixels);
            }
            None => buf.push(0),
        }
    }
    for i in 0..256 {
        buf.extend_from_slice(&font.left_edge[i].to_le_bytes());
    }
    for i in 0..256 {
        buf.extend_from_slice(&font.char_width[i].to_le_bytes());
    }
}

fn read_olhu_font(buf: &[u8], o: &mut usize) -> Option<PencilFontAtlas> {
    if *o + 20 > buf.len() {
        return None;
    }
    let cell_w = u32::from_le_bytes(buf[*o..*o + 4].try_into().ok()?);
    let cell_h = u32::from_le_bytes(buf[*o + 4..*o + 8].try_into().ok()?);
    let char_spacing = i32::from_le_bytes(buf[*o + 8..*o + 12].try_into().ok()?);
    let space_width = i32::from_le_bytes(buf[*o + 12..*o + 16].try_into().ok()?);
    let base_scale = f32::from_le_bytes(buf[*o + 16..*o + 20].try_into().ok()?);
    *o += 20;
    let mut glyphs = Vec::with_capacity(256);
    for _ in 0..256 {
        if *o >= buf.len() {
            return None;
        }
        let present = buf[*o];
        *o += 1;
        if present == 0 {
            glyphs.push(None);
            continue;
        }
        if *o + 4 > buf.len() {
            return None;
        }
        let plen = u32::from_le_bytes(buf[*o..*o + 4].try_into().ok()?) as usize;
        *o += 4;
        if *o + plen > buf.len() {
            return None;
        }
        glyphs.push(Some(HudStripSprite {
            width: cell_w,
            height: cell_h,
            pixels: buf[*o..*o + plen].to_vec(),
        }));
        *o += plen;
    }
    if *o + 256 * 8 > buf.len() {
        return None;
    }
    let mut left_edge = [0i32; 256];
    let mut char_width = [cell_w as i32; 256];
    for i in 0..256 {
        left_edge[i] = i32::from_le_bytes(buf[*o..*o + 4].try_into().ok()?);
        *o += 4;
    }
    for i in 0..256 {
        char_width[i] = i32::from_le_bytes(buf[*o..*o + 4].try_into().ok()?);
        *o += 4;
    }
    Some(PencilFontAtlas {
        cell_w,
        cell_h,
        glyphs,
        left_edge,
        char_width,
        char_spacing,
        space_width,
        base_scale,
    })
}

fn find_graphics_tga(roots: &[PathBuf], name: &str) -> Option<RgbaImage> {
    for root in roots {
        let candidates = [
            root.join("graphics").join(name),
            root.join(name),
            root.join("gameSource").join("graphics").join(name),
        ];
        for p in candidates {
            if p.is_file() {
                if let Ok(img) = load_tga_path(&p) {
                    return Some(img);
                }
            }
        }
    }
    None
}

/// C++ `splitAndExpandSprites`: horizontal equal slices of a full TGA.
fn load_strip(roots: &[PathBuf], name: &str, n: usize) -> Option<Vec<HudStripSprite>> {
    let full = find_graphics_tga(roots, name)?;
    if n == 0 || full.width < n as u32 {
        return None;
    }
    let sprite_w = full.width / n as u32;
    let sprite_h = full.height;
    if sprite_w == 0 || sprite_h == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x0 = i as u32 * sprite_w;
        let mut pixels = vec![0u8; (sprite_w * sprite_h * 4) as usize];
        for y in 0..sprite_h {
            for x in 0..sprite_w {
                let src = full.pixel(x0 + x, y);
                let di = ((y * sprite_w + x) * 4) as usize;
                pixels[di..di + 4].copy_from_slice(&src);
            }
        }
        out.push(HudStripSprite {
            width: sprite_w,
            height: sprite_h,
            pixels,
        });
    }
    Some(out)
}

// --- draw -------------------------------------------------------------------

/// Rounded paper / chalk capsule (Y-down). Used when `chalkBlot.tga` is missing.
fn fill_round_rect(fb: &mut Framebuffer, x: f32, y: f32, w: f32, h: f32, rgba: [u8; 4]) {
    if w < 1.0 || h < 1.0 || rgba[3] == 0 {
        return;
    }
    let x0 = x.round() as i32;
    let y0 = y.round() as i32;
    let bw = w.round().max(1.0) as i32;
    let bh = h.round().max(1.0) as i32;
    let r = (bh / 2).max(1);
    let body_w = (bw - 2 * r).max(0);
    if body_w > 0 {
        fb.fill_rect(x0 + r, y0, body_w, bh, rgba);
    }
    // Circular end-caps.
    let r2 = r * r;
    for cap in [x0 + r, x0 + bw - r] {
        for dy in 0..bh {
            let oy = dy - r;
            let span2 = r2 - oy * oy;
            if span2 < 0 {
                continue;
            }
            // integer sqrt
            let mut sx = 0i32;
            while sx * sx <= span2 {
                sx += 1;
            }
            sx -= 1;
            if sx < 0 {
                continue;
            }
            fb.fill_rect(cap - sx, y0 + dy, sx * 2 + 1, 1, rgba);
        }
    }
}

/// Normal alpha blit (gui panel, blood, pencil glyphs).
fn blit_centered(fb: &mut Framebuffer, spr: &HudStripSprite, cx: f32, cy: f32, scale: f32) {
    blit_centered_mode(fb, spr, cx, cy, scale, false, 1.0);
}

/// C++ `toggleMultiplicativeBlend(true)` path for hunger chrome / arrows.
fn blit_centered_mult(fb: &mut Framebuffer, spr: &HudStripSprite, cx: f32, cy: f32, scale: f32) {
    blit_centered_mode(fb, spr, cx, cy, scale, true, 1.0);
}

fn blit_centered_mode(
    fb: &mut Framebuffer,
    spr: &HudStripSprite,
    cx: f32,
    cy: f32,
    scale: f32,
    multiplicative: bool,
    alpha_mul: f32,
) {
    if spr.width == 0 || spr.height == 0 {
        return;
    }
    let scale = scale.max(0.05);
    let alpha_mul = alpha_mul.clamp(0.0, 1.0);
    if alpha_mul <= 1e-5 {
        return;
    }
    let dw = (spr.width as f32 * scale).max(1.0) as i32;
    let dh = (spr.height as f32 * scale).max(1.0) as i32;
    let ox = cx as i32 - dw / 2;
    let oy = cy as i32 - dh / 2;
    for dy in 0..dh {
        for dx in 0..dw {
            let u = (dx * spr.width as i32 / dw.max(1)).clamp(0, spr.width as i32 - 1) as u32;
            let v = (dy * spr.height as i32 / dh.max(1)).clamp(0, spr.height as i32 - 1) as u32;
            let si = ((v * spr.width + u) * 4) as usize;
            if si + 3 >= spr.pixels.len() {
                continue;
            }
            let mut rgba = [
                spr.pixels[si],
                spr.pixels[si + 1],
                spr.pixels[si + 2],
                spr.pixels[si + 3],
            ];
            if alpha_mul < 0.999 {
                rgba[3] = ((rgba[3] as f32) * alpha_mul).round() as u8;
            }
            // C++: hunger boxes/fills/arrows use multiplicative blend over panel.
            if multiplicative {
                fb.put_multiplicative(ox + dx, oy + dy, rgba);
            } else {
                fb.put(ox + dx, oy + dy, rgba);
            }
        }
    }
}

/// C++ `drawHungerMaxFillLine` — bar under capacity slot + dash chain from ate words.
///
/// // C++: LivingLifePage.cpp ~5958–6020
fn draw_hunger_max_fill_line(
    fb: &mut Framebuffer,
    ate_x: f32,
    ate_y: f32,
    max_fill: i32,
    bar_sprites: &[HudStripSprite],
    dash_sprites: &[HudStripSprite],
    scale: f32,
    skip_bar: bool,
    skip_dashes: bool,
    multiplicative: bool,
) {
    if max_fill < 0 || bar_sprites.is_empty() {
        return;
    }
    let s = scale;
    // Bar sits at hunger-box slot `max_fill` with C++ offsets (−12, −10 Y-up).
    let (bx0, by0) = hunger_box_screen_pos(max_fill, fb.width, fb.height);
    // hunger_box_screen_pos already includes scale; bar offset is design-space * s
    // but box pos is already scaled — recompute design-space bar for dash loop.
    let cx = fb.width as f32 * 0.5;
    let cy = fb.height as f32 * 0.5;
    let bar_x = cx
        + (HUNGER_BOX_ORIGIN_X + max_fill as f32 * HUNGER_BOX_PITCH + HUNGER_BAR_OFFSET_X) * s;
    let bar_y = cy + (HUNGER_BOX_ORIGIN_Y_BELOW + HUNGER_BAR_OFFSET_Y_BELOW) * s;
    let _ = (bx0, by0);

    let blit = |fb: &mut Framebuffer, spr: &HudStripSprite, x: f32, y: f32| {
        if multiplicative {
            blit_centered_mult(fb, spr, x, y, s);
        } else {
            blit_centered(fb, spr, x, y, s);
        }
    };

    if !skip_bar {
        let bi = (max_fill as usize) % NUM_HUNGER_DASHES;
        if let Some(spr) = bar_sprites.get(bi) {
            blit(fb, spr, bar_x, bar_y);
        }
    }

    if skip_dashes || dash_sprites.is_empty() {
        return;
    }

    // C++: dashPos = ateWords; dashPos.y -= 6; dashPos.x -= 5; step left by 15.
    let mut dash_x = ate_x - 5.0 * s;
    let dash_y = ate_y + 6.0 * s; // Y-down: C++ y -= 6 on Y-up
    let mut num_dashes: i32 = 0;
    let step = 15.0 * s;
    let bar_right = bar_x + 9.0 * s;

    while dash_x > bar_right {
        let di = (num_dashes as usize) % NUM_HUNGER_DASHES;
        if let Some(spr) = dash_sprites.get(di) {
            blit(fb, spr, dash_x, dash_y);
        }
        dash_x -= step;
        num_dashes += 1;
        // C++: correct shortness of last strip every NUM_HUNGER_DASHES.
        if num_dashes % NUM_HUNGER_DASHES as i32 == 0 {
            dash_x += 3.0 * s;
        }
    }
    // One more dash to connect to bar.
    dash_x = bar_x + 6.0 * s;
    let di = (num_dashes as usize) % NUM_HUNGER_DASHES;
    if let Some(spr) = dash_sprites.get(di) {
        blit(fb, spr, dash_x, dash_y);
    }
}

// --- tiny pencil-style 5×7 glyphs (no font TGA deps; soft-FB) ---------------

/// 5×7 bitmap glyphs for digits, A–Z, and common speech punctuation.
/// Rows are top→bottom; bit 4 = leftmost pixel.
pub fn glyph5x7(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x00, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x06, 0x04, 0x00, 0x04],
        '\'' => [0x0C, 0x0C, 0x08, 0x00, 0x00, 0x00, 0x00],
        '"' => [0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        ';' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x04, 0x08],
        '/' => [0x01, 0x02, 0x04, 0x04, 0x08, 0x10, 0x10],
        '\\' => [0x10, 0x10, 0x08, 0x04, 0x04, 0x02, 0x01],
        '(' => [0x04, 0x08, 0x10, 0x10, 0x10, 0x08, 0x04],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        ']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        '*' => [0x00, 0x0A, 0x04, 0x1F, 0x04, 0x0A, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'A' | 'a' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' | 'b' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' | 'c' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' | 'd' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' | 'e' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' | 'f' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' | 'g' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' | 'h' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' | 'i' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' | 'j' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' | 'k' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' | 'l' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' | 'm' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' | 'n' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' | 'o' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' | 'p' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' | 'q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' | 'r' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' | 's' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' | 't' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' | 'u' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' | 'v' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' | 'w' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' | 'x' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' | 'y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' | 'z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        _ => return None,
    })
}

/// Pixel width of a pencil string at `scale` (6 design-px per char).
pub fn pencil_string_width(text: &str, scale: f32) -> f32 {
    let s = scale.max(0.05);
    text.chars().count() as f32 * 6.0 * s
}

/// Draw left-aligned dark pencil string (C++ `pencilFont` stand-in).
///
/// Used by HUD meters and L-SAY soft-FB speech bubbles.
pub fn draw_pencil_string(
    fb: &mut Framebuffer,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    rgba: [u8; 4],
    align_center: bool,
) {
    let s = scale.max(0.05);
    let cell_w = 6.0 * s;
    let cell_h = 8.0 * s;
    let total_w = text.chars().count() as f32 * cell_w;
    let mut pen_x = if align_center {
        x - total_w * 0.5
    } else {
        x
    };
    let top = y - cell_h * 0.5;
    for ch in text.chars() {
        if let Some(rows) = glyph5x7(ch) {
            for (row_i, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        let px = (pen_x + col as f32 * s).round() as i32;
                        let py = (top + row_i as f32 * s).round() as i32;
                        // 1 design-px → ceil(s) soft-FB pixels for readability
                        let ps = s.ceil().max(1.0) as i32;
                        for oy in 0..ps {
                            for ox in 0..ps {
                                fb.put(px + ox, py + oy, rgba);
                            }
                        }
                    }
                }
            }
        }
        pen_x += cell_w;
    }
}

/// Soft chalk blot + pencil text (C++ `drawChalkBackgroundString` stand-in).
///
/// `cx,cy` is the bubble center; `fade` multiplies alpha (0..1).
/// Uses 5×7 glyphs and a solid chalk rect (no asset tree required).
/// Default ink is black (see [`draw_speech_bubble_colored`] for curse purple).
pub fn draw_speech_bubble(
    fb: &mut Framebuffer,
    text: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    fade: f32,
) {
    draw_speech_bubble_with(fb, text, cx, cy, scale, fade, None);
}

/// Speech bubble with optional HUD sprites (P3#15).
///
/// When `sprites` has disk TGAs:
/// - `chalkBlot.tga` tiled along the line (C++ multi-hit blot strip)
/// - `font_handwriting_32_32.tga` for ink (else 5×7 [`glyph5x7`] fallback)
///
/// Without sprites / missing assets, matches [`draw_speech_bubble`].
pub fn draw_speech_bubble_with(
    fb: &mut Framebuffer,
    text: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    fade: f32,
    sprites: Option<&HudSprites>,
) {
    draw_speech_bubble_colored(fb, text, cx, cy, scale, fade, [0, 0, 0], sprites);
}

/// Speech bubble with explicit ink RGB (alpha from `fade`).
///
/// // C++ LivingLifePage ~4083–4096 — black / white dying / purple curse
/// Pencil (5×7) soft-black when `text_rgb` is pure black; otherwise uses `text_rgb`.
pub fn draw_speech_bubble_colored(
    fb: &mut Framebuffer,
    text: &str,
    cx: f32,
    cy: f32,
    scale: f32,
    fade: f32,
    text_rgb: [u8; 3],
    sprites: Option<&HudSprites>,
) {
    if text.is_empty() || fade <= 0.0 {
        return;
    }
    let s = scale.max(0.5);
    let f = fade.clamp(0.0, 1.0);
    let a = (f * 255.0) as u8;

    let hand = sprites.and_then(|sp| sp.handwriting_font.as_ref());
    // Design-space length (scale 1) for blot count; screen length for layout.
    let len_design = if let Some(font) = hand {
        font.measure(text, 1.0)
    } else {
        pencil_string_width(text, 1.0)
    };
    let tw = len_design * s;
    let th = if let Some(font) = hand {
        font.cell_h as f32 * font.base_scale * s
    } else {
        8.0 * s
    };
    let line_x0 = cx - tw * 0.5;
    let line_y = cy;

    let chalk = sprites
        .filter(|sp| sp.chalk_from_disk)
        .and_then(|sp| sp.chalk_blot.as_ref());
    if let Some(blot) = chalk {
        // C++: numBlots = lrint(0.25 + length / 20) + 1; stretch along line.
        let num_blots = ((0.25 + len_design / 20.0).round() as i32 + 1).max(1) as usize;
        let blot_scale = s.max(0.5);
        let spacing = if num_blots <= 1 {
            0.0
        } else {
            tw / (num_blots - 1) as f32
        };
        let first_x = if num_blots == 1 {
            line_x0 + tw * 0.5
        } else {
            line_x0
        };
        // Vertical multi-hit offsets (C++ ±5 object units); scale with text.
        let dy = 5.0 * s;
        for b in 0..num_blots {
            let bx = first_x + spacing * b as f32;
            // Double-hit center + vertical pair (C++ four draws per blot).
            blit_centered_mode(fb, blot, bx, line_y, blot_scale, false, f);
            blit_centered_mode(fb, blot, bx, line_y, blot_scale, false, f);
            blit_centered_mode(fb, blot, bx, line_y + dy, blot_scale, false, f);
            blit_centered_mode(fb, blot, bx, line_y - dy, blot_scale, false, f);
        }
    } else {
        // Soft chalk pill (C++ tiled chalkBlot stand-in): rounded paper, no 1px box.
        let pad_x = 10.0 * s;
        let pad_y = 6.0 * s;
        let bw = (tw + pad_x * 2.0).ceil().max(22.0);
        let bh = (th + pad_y * 2.0).ceil().max(16.0);
        let x0 = cx - bw * 0.5;
        let y0 = cy - bh * 0.5;
        let shadow_a = ((f * 55.0) as u8).max(10);
        fill_round_rect(fb, x0 + 2.0, y0 + 3.0, bw, bh, [36, 28, 20, shadow_a]);
        let paper_a = ((f * 230.0) as u8).max(50);
        fill_round_rect(fb, x0, y0, bw, bh, [248, 244, 232, paper_a]);
        let hi_a = ((f * 80.0) as u8).max(10);
        fill_round_rect(
            fb,
            x0 + 2.0,
            y0 + 1.0,
            (bw - 4.0).max(4.0),
            (bh * 0.35).max(2.0),
            [255, 252, 246, hi_a],
        );
    }

    // Handwriting: exact RGB. Pencil 5×7: soft-black stand-in when pure black.
    let ink: [u8; 4] = if text_rgb == [0, 0, 0] && hand.is_none() {
        [20, 20, 18, a]
    } else {
        [text_rgb[0], text_rgb[1], text_rgb[2], a]
    };
    if let Some(font) = hand {
        font.draw_string(fb, text, cx, cy, s, ink, true);
    } else {
        draw_pencil_string(fb, text, cx, cy, s, ink, true);
    }
}

impl HudSprites {
    /// Speech bubble using this pack's chalk / handwriting TGAs (or 5×7).
    pub fn draw_speech_bubble(
        &self,
        fb: &mut Framebuffer,
        text: &str,
        cx: f32,
        cy: f32,
        scale: f32,
        fade: f32,
    ) {
        draw_speech_bubble_with(fb, text, cx, cy, scale, fade, Some(self));
    }

    /// Speech bubble with curse/dying ink color (soft-FB).
    pub fn draw_speech_bubble_colored(
        &self,
        fb: &mut Framebuffer,
        text: &str,
        cx: f32,
        cy: f32,
        scale: f32,
        fade: f32,
        text_rgb: [u8; 3],
    ) {
        draw_speech_bubble_colored(fb, text, cx, cy, scale, fade, text_rgb, Some(self));
    }
}

/// C++ note paper while `mSayField` is focused (`mNotePaperHideOffset` + 58).
fn draw_say_note(
    fb: &mut Framebuffer,
    draft: &str,
    sprites: &HudSprites,
    s: f32,
    cx: f32,
    cy: f32,
) {
    let paper_x = cx - 282.0 * s;
    let paper_y = cy + 362.0 * s;
    if let Some(paper) = &sprites.note_paper {
        blit_centered(fb, paper, paper_x, paper_y, s);
    } else {
        let pw = 360.0 * s;
        let ph = 140.0 * s;
        fill_round_rect(
            fb,
            paper_x - pw * 0.5,
            paper_y - ph * 0.5,
            pw,
            ph,
            [248, 244, 232, 235],
        );
    }
    let shown = if draft.is_empty() {
        "_".to_string()
    } else {
        format!("{draft}_")
    };
    let tx = paper_x - 160.0 * s;
    let ty = paper_y - 79.0 * s;
    sprites.draw_hud_text(fb, &shown, tx, ty, s.max(0.85), [20, 18, 14, 255], false, false);
}

/// Draw bottom gui panel + hunger capacity boxes + temperature arrow + yum/ate.
///
/// C++ draw order (subset): hunger/yum slips → guiPanel → [guiBlood] → mult
/// hunger → mult arrows → pencil yum/ate (+ erased stacks) → temp tip → home arrows.
///
/// Mutates `state` for draw-time temp-arrow rotation / OldArrow trail
/// (C++ does this inside the draw path).
pub fn draw_food_heat_hud(fb: &mut Framebuffer, state: &mut HudState, sprites: &HudSprites) {
    if state.hide_gui {
        return;
    }
    if !state.visible && state.food_capacity <= 0 && state.max_food_capacity <= 0 {
        return;
    }
    let s = hud_scale(fb.width, fb.height);
    let cx = fb.width as f32 * 0.5;
    let cy = fb.height as f32 * 0.5;

    // --- Hunger slips (full / hungry / starving) under panel — animated ---
    for si in 0..NUM_HUNGER_SLIPS {
        if (state.hunger_slip_pos_y[si] - HUNGER_SLIP_HIDE_Y[si]).abs() < 0.5 {
            continue;
        }
        if let Some(spr) = sprites.hunger_slips.get(si) {
            let sx = cx + HUNGER_SLIP_X * s;
            let sy = cy + state.hunger_slip_draw_y_below(si) * s;
            blit_centered(fb, spr, sx, sy, s);
        }
    }

    // --- Yum slips (dual flip slots 0..1) ---
    for yi in 0..2 {
        if (state.yum_slip_pos_y[yi] - YUM_SLIP_HIDE_Y_BELOW).abs() < 0.5 {
            continue;
        }
        if let Some(spr) = sprites.yum_slips.get(yi).or_else(|| sprites.yum_slips.first()) {
            let sx = cx + YUM_SLIP_HIDE_X * s;
            let sy = cy + state.yum_slip_pos_y[yi] * s;
            blit_centered(fb, spr, sx, sy, s);
            let n = state.yum_slip_numbers[yi];
            if n > 0 {
                let label = format!("{n}x");
                sprites.draw_hud_text(
                    fb,
                    &label,
                    sx,
                    sy - 4.0 * s,
                    s,
                    [0, 0, 0, 255],
                    true,
                    false,
                );
            }
        }
    }

    // --- Home arrows + P3#17 map-pointer label (C++ drawHomeSlip) ---
    {
        let hx = cx + HOME_ARROW_ORIGIN_X * s;
        let hy = cy + HOME_ARROW_ORIGIN_Y_BELOW * s;
        // C++ slip center is 35 object-px below the arrow (Y-up +35 on arrow).
        let slip_y = hy + 35.0 * s;
        if state.home_arrow.is_some() {
            if let Some(slip) = sprites.home_slips.first() {
                blit_centered(fb, slip, hx, slip_y, s);
            }
        }
        if state.ancient_home_arrow.is_some() {
            let ax = cx + (HOME_ARROW_ORIGIN_X + 71.0) * s;
            if let Some(slip) = sprites.home_slips.get(1).or_else(|| sprites.home_slips.first())
            {
                blit_centered_mode(fb, slip, ax, slip_y, s, false, 0.85);
            }
        }
        for i in 0..NUM_HOME_ARROWS {
            let fade = state.home_arrow_fades[i];
            if fade > 0.01 && state.home_arrow != Some(i) {
                if let Some(spr) = sprites.home_arrows_erased.get(i) {
                    blit_centered_mode(fb, spr, hx, hy, s, false, fade.clamp(0.0, 1.0));
                }
            }
        }
        if let Some(dir) = state.home_arrow {
            let di = dir % NUM_HOME_ARROWS;
            if let Some(spr) = sprites.home_arrows.get(di) {
                blit_centered(fb, spr, hx, hy, s);
            }
        }
        // C++ homeSlip2 (j=1): ancient homeland, offset +71 design-x (30−(-41)).
        if let Some(dir) = state.ancient_home_arrow {
            let ax = cx + (HOME_ARROW_ORIGIN_X + 71.0) * s;
            let ay = hy;
            let di = dir % NUM_HOME_ARROWS;
            if let Some(spr) = sprites.home_arrows.get(di) {
                blit_centered_mode(fb, spr, ax, ay, s, false, 0.85);
            }
        }
        // Pencil label under arrow strip (`MAP` / `BABY` / `LEAD` / …).
        if let Some(ref lab) = state.map_pointer_label {
            if !lab.is_empty() {
                let label_scale = (s * 0.9).max(0.7);
                sprites.draw_hud_text(
                    fb,
                    lab,
                    hx,
                    hy + 22.0 * s,
                    label_scale,
                    [20, 20, 18, 255],
                    true,
                    false,
                );
            }
        }
    }

    // C++ note paper (SAY compose) sits above the gui panel, sliding up from below.
    if let Some(ref draft) = state.say_draft {
        draw_say_note(fb, draft, sprites, s, cx, cy);
    }

    // C++ guiPanel.tga is the only bottom strip. A full-width cream fill sat
    // over/around it as a fake "desk" — only used when the TGA is missing.
    let py = cy + GUI_PANEL_Y_BELOW * s;
    if sprites.gui_panel.is_none() {
        let panel_h = 80.0 * s;
        let desk_y = (py - panel_h * 0.5).round() as i32;
        let desk_h = panel_h.round().max(1.0) as i32;
        fb.fill_rect(
            0,
            desk_y,
            fb.width as i32,
            desk_h,
            [214, 210, 200, 255],
        );
    }

    // Gui panel (normal alpha — not multiplicative).
    if let Some(panel) = &sprites.gui_panel {
        blit_centered(fb, panel, cx, py, s);
    }

    // C++: dying && !sick → guiBlood with multiplicative blend under panel offset.
    if state.dying {
        if let Some(blood) = &sprites.gui_blood {
            let py = cy + (GUI_PANEL_Y_BELOW + 32.0) * s;
            let px = cx - 32.0 * s;
            blit_centered_mult(fb, blood, px, py, s);
        }
    }

    let cap = state.food_capacity.max(0);
    let store = state.display_fill();
    let max_store = state.max_food_store.max(store);
    let max_cap = state.max_food_capacity.max(cap);

    // Active capacity slots — C++ multiplicative blend.
    for i in 0..cap {
        let (x, y) = hunger_box_screen_pos(i, fb.width, fb.height);
        let bi = (i as usize) % NUM_HUNGER_BOX_SPRITES;
        if let Some(spr) = sprites.hunger_boxes.get(bi) {
            blit_centered_mult(fb, spr, x, y, s);
        }
        if i < store {
            if let Some(spr) = sprites.hunger_fills.get(bi) {
                blit_centered_mult(fb, spr, x, y, s);
            }
        } else if i < max_store {
            if let Some(spr) = sprites.hunger_fills_erased.get(bi) {
                blit_centered_mult(fb, spr, x, y, s);
            }
        }
    }

    // Erased trailing capacity (once held higher max).
    for i in cap..max_cap {
        let (x, y) = hunger_box_screen_pos(i, fb.width, fb.height);
        let bi = (i as usize) % NUM_HUNGER_BOX_SPRITES;
        if let Some(spr) = sprites.hunger_boxes_erased.get(bi) {
            blit_centered_mult(fb, spr, x, y, s);
        }
        if i < max_store {
            if let Some(spr) = sprites.hunger_fills_erased.get(bi) {
                blit_centered_mult(fb, spr, x, y, s);
            }
        }
    }

    // C++ draw-time heat delta → OldArrow + arrow_i advance.
    state.prepare_temp_arrow();

    // Ghost arrows (erased strip, fade via alpha; C++ additive whitening approximated).
    for a in &state.old_arrows {
        let (ax, ay) = temp_arrow_screen_pos(a.heat, fb.width, fb.height);
        let ai = a.i % NUM_TEMP_ARROWS;
        if let Some(spr) = sprites.temp_arrows_erased.get(ai) {
            // fade 1→0: visible→gone. C++ uses additive color (1-fade); soft-FB uses α.
            blit_centered_mode(fb, spr, ax, ay, s, true, a.fade.clamp(0.0, 1.0));
        }
    }

    // Current temperature arrow.
    let (ax, ay) = temp_arrow_screen_pos(state.heat, fb.width, fb.height);
    let ai = state.arrow_i % NUM_TEMP_ARROWS;
    if let Some(spr) = sprites.temp_arrows.get(ai) {
        blit_centered_mult(fb, spr, ax, ay, s);
    }

    // Curse token "C+X" to the right of temp meter (C++ pencilFont / erased).
    if let Some(tokens) = state.curse_tokens {
        let (cx_t, cy_t) = curse_token_screen_pos(fb.width, fb.height);
        let erased = tokens <= 0;
        let alpha = if tokens > 0 { 255 } else { 90 };
        let col = [0, 0, 0, alpha];
        sprites.draw_hud_text(fb, "C", cx_t, cy_t, s, col, true, erased);
        sprites.draw_hud_text(fb, "+", cx_t, cy_t, s, col, true, erased);
        sprites.draw_hud_text(fb, "X", cx_t + 6.0 * s, cy_t, s, col, true, erased);
        if state.excess_curse_points > 0 {
            let pts = state.excess_curse_points.to_string();
            sprites.draw_hud_text(
                fb,
                &pts,
                cx_t + 3.0 * s,
                cy_t + 22.0 * s,
                s,
                [0, 0, 0, 255],
                true,
                false,
            );
        }
    }

    // Old yum bonus (erased pencil stack) then current.
    let (yx, yy) = yum_screen_pos(fb.width, fb.height);
    for old in &state.old_yum_bonus {
        let a = (old.fade.clamp(0.0, 1.0) * 255.0) as u8;
        sprites.draw_hud_text(fb, &old.text, yx, yy, s, [0, 0, 0, a], false, true);
    }
    if state.yum_bonus > 0 {
        let text = format!("+{}", state.yum_bonus);
        sprites.draw_hud_text(fb, &text, yx, yy, s, [0, 0, 0, 255], false, false);
    }

    // Last-ate: old erased stack + current pencil + max-fill line.
    let (ate_x, ate_y) = ate_screen_pos(fb.width, fb.height);
    for old in &state.old_last_ate {
        let a = (old.fade.clamp(0.0, 1.0) * 255.0) as u8;
        sprites.draw_hud_text(fb, &old.text, ate_x, ate_y, s, [0, 0, 0, a], false, true);
        if old.fill_max > 0 {
            draw_hunger_max_fill_line(
                fb,
                ate_x,
                ate_y,
                old.fill_max,
                &sprites.hunger_bars_erased,
                &sprites.hunger_dashes_erased,
                s,
                false,
                true,
                true,
            );
        }
    }
    if let Some(label) = state.current_last_ate_string.clone() {
        sprites.draw_hud_text(fb, &label, ate_x, ate_y, s, [0, 0, 0, 255], false, false);
    } else if state.last_ate_id > 0 {
        let label = format!("#{}", state.last_ate_id);
        sprites.draw_hud_text(fb, &label, ate_x, ate_y, s, [0, 0, 0, 255], false, false);
    }
    // C++ craving sheets (handwriting on hint paper, above chat). Soft-FB: line under last-ate.
    if let Some(ref craving) = state.craving_text {
        if !craving.is_empty() {
            let (cx_c, cy_c) = ate_screen_pos(fb.width, fb.height);
            sprites.draw_hud_text(
                fb,
                craving,
                cx_c,
                cy_c - 22.0 * s,
                (s * 0.85).max(0.7),
                [20, 20, 18, 255],
                true,
                false,
            );
        }
    }

    if state.last_ate_fill_max > 0 {
        draw_hunger_max_fill_line(
            fb,
            ate_x,
            ate_y,
            state.last_ate_fill_max,
            &sprites.hunger_bars,
            &sprites.hunger_dashes,
            s,
            false,
            false,
            true,
        );
    }

    // Temp-meter hover tip (food_time / indoor_bonus) wins over biome/object tip.
    let tip_x = cx;
    let tip_y = cy + TIP_ORIGIN_Y_BELOW * s;
    if state.pointer_over_temp_meter(fb.width, fb.height) {
        if let Some(tip) = state.temp_meter_tip_text() {
            sprites.draw_hud_text(fb, &tip, tip_x, tip_y, s, [0, 0, 0, 255], true, false);
        }
    } else if let Some(tip) = state.hover_tip.as_deref() {
        if !tip.is_empty() {
            sprites.draw_hud_text(fb, tip, tip_x, tip_y, s, [0, 0, 0, 255], true, false);
        }
    }
}

/// Convenience: draw only if state has been fed FX/HX (or forced visible).
///
/// Takes `&mut HudState` because draw-time temp-arrow trail mutates state
/// (C++ `mOldArrows` / `mCurrentArrowI` updated in draw).
pub fn draw_hud_if_visible(fb: &mut Framebuffer, state: &mut HudState, sprites: &HudSprites) {
    if state.visible || state.food_capacity > 0 || state.max_food_capacity > 0 {
        draw_food_heat_hud(fb, state, sprites);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fx(store: i32, cap: i32) -> FoodChange {
        FoodChange {
            food_store: store,
            food_capacity: cap,
            last_ate_id: 31,
            last_ate_fill_max: 4,
            move_speed: 3.75,
            responsible_id: -1,
            yum_bonus: 2,
            yum_multiplier: 1,
        }
    }

    #[test]
    fn hunger_layout_left_of_center() {
        let (x, y) = hunger_box_screen_pos(0, 1280, 720);
        assert!(x < 1280.0 * 0.5, "first box left of center, x={x}");
        assert!(y > 720.0 * 0.5, "boxes below center (Y-down), y={y}");
        let (x1, _) = hunger_box_screen_pos(1, 1280, 720);
        assert!((x1 - x - HUNGER_BOX_PITCH).abs() < 0.01);
    }

    #[test]
    fn temp_arrow_cold_left_of_hot() {
        let (xc, _) = temp_arrow_screen_pos(0.0, 1280, 720);
        let (xm, _) = temp_arrow_screen_pos(0.5, 1280, 720);
        let (xh, _) = temp_arrow_screen_pos(1.0, 1280, 720);
        assert!(xc < xm && xm < xh, "cold={xc} mid={xm} hot={xh}");
        // mid should sit near design origin
        let expected_mid = 1280.0 * 0.5 + TEMP_ARROW_ORIGIN_X;
        assert!((xm - expected_mid).abs() < 1.0, "mid={xm} expected≈{expected_mid}");
    }

    #[test]
    fn hud_draws_paper_desk_on_960x540() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(8, 12));
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(960, 540);
        fb.clear([30, 90, 40, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let y = 515u32;
        let mut light = 0usize;
        for x in 0..960u32 {
            let i = ((y * 960 + x) * 4) as usize;
            if fb.pixels[i] > 180 && fb.pixels[i + 1] > 170 && fb.pixels[i + 2] > 160 {
                light += 1;
            }
        }
        assert!(
            light > 400,
            "expected light paper desk under meters, light={light}"
        );
    }

    #[test]
    fn golden_layout_1280x720_box0_and_heat05() {
        // C++ offsets at design resolution, scale=1.
        let (bx, by) = hunger_box_screen_pos(0, 1280, 720);
        assert!((bx - (640.0 - 590.0)).abs() < 0.01, "box0 x={bx}");
        assert!((by - (360.0 + 334.0)).abs() < 0.01, "box0 y={by}");
        let (ax, ay) = temp_arrow_screen_pos(0.5, 1280, 720);
        assert!((ax - (640.0 + 546.0)).abs() < 0.01, "arrow mid x={ax}");
        assert!((ay - (360.0 + 319.0)).abs() < 0.01, "arrow mid y={ay}");
    }

    #[test]
    fn apply_fx_tracks_max_peaks() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(10, 12));
        assert_eq!(hud.food_store, 10);
        assert_eq!(hud.food_capacity, 12);
        assert_eq!(hud.max_food_store, 10);
        assert_eq!(hud.max_food_capacity, 12);
        hud.apply_fx(&sample_fx(4, 10));
        assert_eq!(hud.food_store, 4);
        assert_eq!(hud.food_capacity, 10);
        assert_eq!(hud.max_food_store, 10, "peak store retained");
        assert_eq!(hud.max_food_capacity, 12, "peak capacity retained");
        assert_eq!(hud.yum_bonus, 2);
        assert!(hud.visible);
    }

    #[test]
    fn craving_hud_line_uppercases_name() {
        let mut content = crate::content::ClientContent::new();
        content.objects.insert(
            31,
            crate::content::ClientObjectDef {
                id: 31,
                name: "Cooked Meat".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            craving_hud_line(&content, 31, 3),
            "CRAVING: COOKED MEAT (+3)"
        );
    }

    #[test]
    fn prepare_temp_arrow_rotates_on_heat_delta() {
        let mut hud = HudState::new();
        hud.apply_hx(&HeatChange {
            heat: 0.4,
            food_time: 1.0,
            indoor_bonus: 0.0,
        });
        // First prepare latches without rotating.
        hud.prepare_temp_arrow();
        assert_eq!(hud.arrow_i, 0);
        assert!(hud.old_arrows.is_empty());
        assert!((hud.heat - 0.4).abs() < 1e-5);

        hud.apply_hx(&HeatChange {
            heat: 0.6,
            food_time: 1.0,
            indoor_bonus: 0.1,
        });
        hud.prepare_temp_arrow();
        assert_eq!(hud.arrow_i, 1);
        assert_eq!(hud.old_arrows.len(), 1);
        assert!((hud.old_arrows[0].heat - 0.4).abs() < 1e-5);
        assert!((hud.old_arrows[0].fade - 1.0).abs() < 1e-5);

        // Same heat again — no rotate.
        hud.prepare_temp_arrow();
        assert_eq!(hud.arrow_i, 1);
        assert_eq!(hud.old_arrows.len(), 1);
    }

    #[test]
    fn apply_hx_does_not_rotate_until_prepare() {
        let mut hud = HudState::new();
        hud.apply_hx(&HeatChange {
            heat: 0.4,
            food_time: 1.0,
            indoor_bonus: 0.0,
        });
        hud.prepare_temp_arrow();
        hud.apply_hx(&HeatChange {
            heat: 0.7,
            food_time: 1.0,
            indoor_bonus: 0.0,
        });
        assert_eq!(hud.arrow_i, 0, "apply_hx alone must not advance arrow_i");
        hud.prepare_temp_arrow();
        assert_eq!(hud.arrow_i, 1);
    }

    #[test]
    fn draw_marks_framebuffer() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(5, 8));
        hud.apply_hx(&HeatChange {
            heat: 0.5,
            food_time: 0.0,
            indoor_bonus: 0.0,
        });
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(320, 180);
        fb.clear([10, 10, 12, 255]);
        let before = fb.count_non_color([10, 10, 12, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let after = fb.count_non_color([10, 10, 12, 255]);
        assert!(after > before, "HUD should paint pixels ({before} -> {after})");
    }

    #[test]
    fn multiplicative_darkens_light_underlay() {
        // Hunger chrome mult-blend should darken a light panel differently than alpha alone.
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(3, 4));
        hud.visible = true;
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(1280, 720);
        fb.clear([220, 220, 220, 255]);
        // Paint only one box region via full HUD (panel + mult boxes).
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let (bx, by) = hunger_box_screen_pos(0, 1280, 720);
        let i = ((by as u32 * 1280 + bx as u32) * 4) as usize;
        // Center pixel of first box should be darker than clear after mult.
        assert!(
            fb.pixels[i] < 220 || fb.pixels[i + 1] < 220 || fb.pixels[i + 2] < 220,
            "mult blend should darken light underlay at box0"
        );
    }

    #[test]
    fn old_arrow_trail_drawn_after_heat_move() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(2, 4));
        hud.apply_hx(&HeatChange {
            heat: 0.2,
            food_time: 0.0,
            indoor_bonus: 0.0,
        });
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(1280, 720);
        fb.clear([200, 200, 200, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites); // latch 0.2
        hud.apply_hx(&HeatChange {
            heat: 0.8,
            food_time: 0.0,
            indoor_bonus: 0.0,
        });
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        assert_eq!(hud.old_arrows.len(), 1);
        // Ghost at old heat and current at new heat should both paint.
        let (ox, oy) = temp_arrow_screen_pos(0.2, 1280, 720);
        let (nx, ny) = temp_arrow_screen_pos(0.8, 1280, 720);
        let sample = |fb: &Framebuffer, x: f32, y: f32| {
            let i = ((y as u32 * 1280 + x as u32) * 4) as usize;
            [fb.pixels[i], fb.pixels[i + 1], fb.pixels[i + 2]]
        };
        let g = sample(&fb, ox, oy);
        let c = sample(&fb, nx, ny);
        assert!(
            g != [200, 200, 200] || c != [200, 200, 200],
            "trail or current arrow should leave chrome pixels"
        );
    }

    #[test]
    fn max_fill_line_paints_dashes() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(5, 10));
        // last_ate_fill_max from sample is 4
        assert_eq!(hud.last_ate_fill_max, 4);
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(1280, 720);
        fb.clear([15, 15, 18, 255]);
        let before = fb.count_non_color([15, 15, 18, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let after = fb.count_non_color([15, 15, 18, 255]);
        assert!(after > before + 100, "fill line + boxes should paint");
    }

    #[test]
    fn split_strip_math_matches_cpp() {
        // Fake 60×10 image → 3 frames of 20×10
        let mut pixels = vec![0u8; 60 * 10 * 4];
        for i in 0..3u32 {
            for y in 0..10u32 {
                for x in 0..20u32 {
                    let si = ((y * 60 + i * 20 + x) * 4) as usize;
                    pixels[si] = i as u8 * 40;
                    pixels[si + 3] = 255;
                }
            }
        }
        let full = RgbaImage {
            width: 60,
            height: 10,
            pixels,
        };
        let n = 3usize;
        let sprite_w = full.width / n as u32;
        assert_eq!(sprite_w, 20);
        let part0 = full.pixel(0, 0)[0];
        let part2 = full.pixel(40, 0)[0];
        assert_eq!(part0, 0);
        assert_eq!(part2, 80);
    }

    #[test]
    fn load_real_graphics_if_present() {
        let sprites = HudSprites::with_default_roots(None);
        let path = PathBuf::from(r"C:\OhOl\OpenLife\OneLifeGameSourceData\graphics\hungerBoxes.tga");
        if path.is_file() {
            assert!(sprites.from_disk, "expected TGA HUD sprites on disk");
            assert_eq!(sprites.hunger_boxes.len(), NUM_HUNGER_BOX_SPRITES);
            assert!(sprites.hunger_boxes[0].width > 0);
            assert_eq!(sprites.temp_arrows.len(), NUM_TEMP_ARROWS);
            assert_eq!(sprites.hunger_dashes_erased.len(), NUM_HUNGER_DASHES);
            assert_eq!(sprites.hunger_bars_erased.len(), NUM_HUNGER_DASHES);
            assert!(sprites.gui_blood.is_some());
            assert!(sprites.gui_panel.is_some(), "guiPanel.tga expected");
            assert!(sprites.note_paper.is_some(), "notePaper.tga expected");
            // Residual P1#3 assets when game-data tree present.
            let pencil = PathBuf::from(
                r"C:\OhOl\OpenLife\OneLifeGameSourceData\graphics\font_pencil_32_32.tga",
            );
            if pencil.is_file() {
                assert!(sprites.pencil_from_disk, "pencilFont TGA expected");
                assert!(sprites.pencil_font.is_some());
                assert_eq!(sprites.yum_slips.len(), NUM_YUM_SLIPS);
                assert_eq!(sprites.hunger_slips.len(), NUM_HUNGER_SLIPS);
                assert_eq!(sprites.home_arrows.len(), NUM_HOME_ARROWS);
            }
            // P3#15 speech assets
            let hand = PathBuf::from(
                r"C:\OhOl\OpenLife\OneLifeGameSourceData\graphics\font_handwriting_32_32.tga",
            );
            let chalk = PathBuf::from(
                r"C:\OhOl\OpenLife\OneLifeGameSourceData\graphics\chalkBlot.tga",
            );
            if hand.is_file() {
                assert!(sprites.handwriting_from_disk, "handwritingFont TGA expected");
                assert!(sprites.handwriting_font.is_some());
            }
            if chalk.is_file() {
                assert!(sprites.chalk_from_disk, "chalkBlot TGA expected");
                assert!(sprites.chalk_blot.is_some());
            }
        }
    }

    /// Draw path must work with procedural sprites only (no content / game-data tree).
    #[test]
    fn residual_draw_without_content_tree() {
        let mut hud = HudState::new();
        hud.apply_fx(&FoodChange {
            food_store: 3,
            food_capacity: 12,
            last_ate_id: 99,
            last_ate_fill_max: 5,
            move_speed: 3.75,
            responsible_id: -1,
            yum_bonus: 4,
            yum_multiplier: 3,
        });
        hud.apply_hx(&HeatChange {
            heat: 0.55,
            food_time: 40.0,
            indoor_bonus: 8.0,
        });
        hud.set_home_arrow(Some(2));
        hud.set_pointer(640.0 + 540.0, 360.0 + 313.0); // over temp meter @ 1280×720

        // Empty roots → pure procedural (no TGA).
        let sprites = HudSprites::load_from_roots(&[]);
        assert!(!sprites.from_disk);
        assert!(!sprites.pencil_from_disk);
        // No fake slip/panel rectangles when TGAs are missing.
        assert!(sprites.yum_slips.is_empty());
        assert!(sprites.hunger_slips.is_empty());
        assert!(sprites.gui_panel.is_none());

        let mut fb = Framebuffer::new(1280, 720);
        fb.clear([20, 20, 24, 255]);
        let before = fb.count_non_color([20, 20, 24, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let after = fb.count_non_color([20, 20, 24, 255]);
        assert!(after > before + 200, "residual HUD must paint ({before}->{after})");
        // store 3 + yum 4 = 7 → hungry (≤8); starving only when ≤4.
        assert_eq!(hud.hunger_slip_visible, 1, "store+yum=7 → hungry slip");
        assert_eq!(hud.yum_slip_number, 3);
        assert!(hud.pointer_over_temp_meter(1280, 720));
        assert!(hud.temp_meter_tip_text().is_some());
    }

    #[test]
    fn olhu_roundtrip_strips() {
        let s = HudSprites::procedural();
        let dir = std::env::temp_dir().join("ohol_olhu_test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("olhu_hud.bin");
        s.write_olhu(&p).expect("write");
        let loaded = HudSprites::load_olhu(&p).expect("load");
        assert_eq!(loaded.hunger_boxes.len(), NUM_HUNGER_BOX_SPRITES);
        assert_eq!(
            loaded.hunger_boxes[0].pixels.len(),
            s.hunger_boxes[0].pixels.len()
        );
        assert_eq!(loaded.home_arrows.len(), NUM_HOME_ARROWS);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn olhu_roundtrip_fonts() {
        let mut s = HudSprites::procedural();
        let mut glyphs = vec![None; 256];
        glyphs[b'A' as usize] = Some(HudStripSprite::solid(2, 2, [255, 255, 255, 255]));
        let font = PencilFontAtlas {
            cell_w: 2,
            cell_h: 2,
            glyphs,
            left_edge: [0; 256],
            char_width: [2; 256],
            char_spacing: 1,
            space_width: 2,
            base_scale: 1.0,
        };
        s.pencil_font = Some(font.clone());
        s.handwriting_font = Some(font);
        s.pencil_from_disk = true;
        s.handwriting_from_disk = true;
        let dir = std::env::temp_dir().join("ohol_olhu_font_test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("olhu_hud.bin");
        s.write_olhu(&p).expect("write");
        let loaded = HudSprites::load_olhu(&p).expect("load");
        assert!(loaded.pencil_font.is_some());
        assert!(loaded.handwriting_font.is_some());
        assert!(loaded.pencil_from_disk);
        let a = loaded.pencil_font.as_ref().unwrap();
        assert_eq!(a.cell_w, 2);
        assert!(a.glyphs[b'A' as usize].is_some());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn hover_tip_draws_when_set() {
        let mut hud = HudState::new();
        hud.visible = true;
        hud.food_capacity = 1;
        hud.hover_tip = Some("MOUNTAIN".into());
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(320, 180);
        fb.clear([20, 20, 24, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        assert!(fb.count_non_color([20, 20, 24, 255]) > 0);
    }

    #[test]
    fn last_ate_resolves_object_name() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(8, 12));
        assert_eq!(hud.current_last_ate_string.as_deref(), Some("#31"));
        let mut content = crate::content::ClientContent::default();
        let mut def = crate::content::ClientObjectDef::default();
        def.id = 31;
        def.name = "Wild Onion#food".into();
        def.description = "Wild Onion#food".into();
        content.objects.insert(31, def);
        hud.resolve_last_ate_name(&content);
        assert_eq!(hud.current_last_ate_string.as_deref(), Some("Wild Onion"));
    }

    #[test]
    fn hunger_slip_thresholds_adult() {
        let mut hud = HudState::new();
        hud.age_years = 20.0;
        hud.apply_fx(&sample_fx(12, 12));
        assert_eq!(hud.hunger_slip_visible, 0, "full store → full slip");
        hud.apply_fx(&sample_fx(7, 12));
        // store 7 + yum 2 = 9 → none
        assert_eq!(hud.hunger_slip_visible, -1);
        hud.apply_fx(&FoodChange {
            food_store: 5,
            food_capacity: 12,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 0,
            yum_multiplier: 0,
        });
        assert_eq!(hud.hunger_slip_visible, 1, "store 5 → hungry");
        hud.apply_fx(&FoodChange {
            food_store: 2,
            food_capacity: 12,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 0,
            yum_multiplier: 0,
        });
        assert_eq!(hud.hunger_slip_visible, 2, "store 2 → starving");
        assert!(hud.hunger_sound_oneshot, "store 2 → oneshot hunger.aiff");
        assert!(!hud.pulse_hunger_sound);
    }

    #[test]
    fn slip_step_animates_hunger_toward_show() {
        let mut hud = HudState::new();
        hud.age_years = 20.0;
        hud.apply_fx(&FoodChange {
            food_store: 5,
            food_capacity: 12,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 0,
            yum_multiplier: 2,
        });
        assert_eq!(hud.hunger_slip_visible, 1);
        assert!(
            (hud.hunger_slip_pos_y[1] - HUNGER_SLIP_HIDE_Y[1]).abs() < 0.5,
            "starts hidden"
        );
        // ~20 frames @60Hz should fully show (hide 370 → show 250, speed ≥4).
        for _ in 0..40 {
            let _ = hud.step_slips(1.0);
        }
        assert!(
            (hud.hunger_slip_pos_y[1] - HUNGER_SLIP_SHOW_Y[1]).abs() < 1.5,
            "hungry slip shown pos={} target={}",
            hud.hunger_slip_pos_y[1],
            hud.hunger_slip_target_y[1]
        );
        // Yum show: hide 330 → 294
        assert!(
            (hud.yum_slip_pos_y[hud.yum_slip_active] - (YUM_SLIP_HIDE_Y_BELOW - YUM_SLIP_SHOW_DY))
                .abs()
                < 1.5,
            "yum slip shown"
        );
        assert_eq!(hud.yum_slip_number, 2);
    }

    #[test]
    fn starving_oneshot_then_pulse_at_store_one() {
        let mut hud = HudState::new();
        hud.age_years = 20.0;
        hud.apply_fx(&FoodChange {
            food_store: 3,
            food_capacity: 12,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 0,
            yum_multiplier: 0,
        });
        assert_eq!(hud.hunger_slip_visible, 2);
        let ev = hud.step_slips(1.0);
        assert_eq!(ev, HungerSoundEvent::OneShot);
        // store 1 → pulse mode
        hud.apply_fx(&FoodChange {
            food_store: 1,
            food_capacity: 12,
            last_ate_id: 0,
            last_ate_fill_max: 0,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 0,
            yum_multiplier: 0,
        });
        assert!(hud.pulse_hunger_sound);
        assert!(!hud.hunger_sound_oneshot);
    }

    #[test]
    fn old_yum_stack_on_bonus_change() {
        let mut hud = HudState::new();
        hud.apply_fx(&FoodChange {
            food_store: 5,
            food_capacity: 10,
            last_ate_id: 1,
            last_ate_fill_max: 2,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 5,
            yum_multiplier: 2,
        });
        assert!(hud.old_yum_bonus.is_empty());
        hud.apply_fx(&FoodChange {
            food_store: 5,
            food_capacity: 10,
            last_ate_id: 1,
            last_ate_fill_max: 2,
            move_speed: 3.0,
            responsible_id: -1,
            yum_bonus: 3,
            yum_multiplier: 2,
        });
        assert_eq!(hud.old_yum_bonus.len(), 1);
        assert_eq!(hud.old_yum_bonus[0].text, "+5");
    }

    #[test]
    fn hide_gui_skips_draw() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(4, 8));
        hud.hide_gui = true;
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(320, 180);
        fb.clear([5, 5, 5, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        assert_eq!(fb.count_non_color([5, 5, 5, 255]), 0);
    }

    #[test]
    fn pencil_atlas_from_synthetic_grid() {
        // 64×64 = 16×16 cells of 4×4; put ink on cell for 'A' (row 4, col 1).
        let w = 64u32;
        let h = 64u32;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let cell = 4u32;
        let ci = b'A' as u32;
        let col = ci % 16;
        let row = ci / 16;
        for y in 0..cell {
            for x in 0..cell {
                let px = col * cell + x;
                let py = row * cell + y;
                let i = ((py * w + px) * 4) as usize;
                pixels[i] = 255; // red → alpha
                pixels[i + 1] = 0;
                pixels[i + 2] = 0;
                pixels[i + 3] = 255;
            }
        }
        let img = RgbaImage {
            width: w,
            height: h,
            pixels,
        };
        let font = PencilFontAtlas::from_rgba(&img).expect("atlas");
        assert!(font.glyphs[b'A' as usize].is_some());
        assert!(font.measure("A", 1.0) > 0.0);
        let mut fb = Framebuffer::new(64, 64);
        fb.clear([255, 255, 255, 255]);
        font.draw_string(&mut fb, "A", 32.0, 32.0, 1.0, [0, 0, 0, 255], true);
        assert!(fb.count_non_color([255, 255, 255, 255]) > 0);
    }

    #[test]
    fn clear_resets_visible_and_peaks() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(3, 5));
        hud.apply_hx(&HeatChange {
            heat: 0.3,
            food_time: 1.0,
            indoor_bonus: 0.0,
        });
        hud.prepare_temp_arrow();
        hud.apply_hx(&HeatChange {
            heat: 0.7,
            food_time: 1.0,
            indoor_bonus: 0.0,
        });
        hud.prepare_temp_arrow();
        assert!(!hud.old_arrows.is_empty());
        hud.clear();
        assert!(!hud.visible);
        assert_eq!(hud.food_capacity, 0);
        assert_eq!(hud.max_food_capacity, 0);
        assert_eq!(hud.max_food_store, 0);
        assert!(hud.old_arrows.is_empty());
        assert_eq!(hud.arrow_i, 0);
        assert!((hud.current_arrow_heat + 1.0).abs() < 1e-6);
    }

    #[test]
    fn idempotent_sync_keeps_arrow_and_peaks() {
        let mut hud = HudState::new();
        let fx = sample_fx(6, 10);
        let hx = HeatChange {
            heat: 0.55,
            food_time: 2.0,
            indoor_bonus: 0.1,
        };
        for _ in 0..100 {
            hud.sync_from_session(Some(&fx), Some(&hx));
            hud.prepare_temp_arrow();
        }
        assert_eq!(hud.arrow_i, 0, "stable heat must not rotate");
        assert_eq!(hud.max_food_store, 6);
        assert_eq!(hud.max_food_capacity, 10);
        assert!(hud.old_arrows.is_empty());
    }

    #[test]
    fn yum_pencil_paints_plus_digits() {
        let mut hud = HudState::new();
        hud.apply_fx(&sample_fx(2, 4));
        assert_eq!(hud.yum_bonus, 2);
        let sprites = HudSprites::procedural();
        let mut fb = Framebuffer::new(1280, 720);
        fb.clear([240, 240, 240, 255]);
        draw_food_heat_hud(&mut fb, &mut hud, &sprites);
        let (yx, yy) = yum_screen_pos(1280, 720);
        // Sample near yum origin — should have dark pencil pixels.
        let mut dark = 0usize;
        for dy in -4..12 {
            for dx in 0..40 {
                let x = yx as i32 + dx;
                let y = yy as i32 + dy;
                if x < 0 || y < 0 || x >= 1280 || y >= 720 {
                    continue;
                }
                let i = ((y as u32 * 1280 + x as u32) * 4) as usize;
                if fb.pixels[i] < 40 && fb.pixels[i + 1] < 40 && fb.pixels[i + 2] < 40 {
                    dark += 1;
                }
            }
        }
        assert!(dark > 5, "yum +N pencil should leave dark pixels, got {dark}");
    }

    #[test]
    fn speech_bubble_curse_purple_ink() {
        let mut fb = Framebuffer::new(120, 40);
        fb.clear([255, 255, 255, 255]);
        // Successful curse on uncursed speaker → 0.5 purple [128,0,128].
        draw_speech_bubble_colored(&mut fb, "HI", 60.0, 20.0, 2.0, 1.0, [128, 0, 128], None);
        let purple = fb
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 80 && p[0] < 160 && p[1] < 40 && p[2] > 80 && p[2] < 160)
            .count();
        assert!(purple > 5, "curse purple ink expected, got {purple}");
    }

    #[test]
    fn speech_bubble_paints_chalk_and_letters() {
        let mut fb = Framebuffer::new(120, 40);
        fb.clear([0, 0, 0, 255]);
        draw_speech_bubble(&mut fb, "HI", 60.0, 20.0, 2.0, 1.0);
        let mut chalk = 0usize;
        let mut dark = 0usize;
        for p in fb.pixels.chunks_exact(4) {
            if p[0] > 180 && p[1] > 180 && p[2] > 180 {
                chalk += 1;
            }
            if p[0] < 40 && p[1] < 40 && p[2] < 40 && p[3] > 0 {
                // background is black; skip pure black bg — look for near-black pencil on chalk
            }
            if p[0] < 50 && p[1] < 50 && p[2] < 50 && (p[0] > 0 || p[1] > 0 || p[2] > 0) {
                dark += 1;
            }
        }
        // Dark pencil on chalk: pixels written as [20,20,18]
        let mut ink = 0usize;
        for p in fb.pixels.chunks_exact(4) {
            if p[0] < 40 && p[1] < 40 && p[2] < 40 && p[0] + p[1] + p[2] > 0 {
                ink += 1;
            }
        }
        assert!(chalk > 20, "chalk blot pixels expected, got {chalk}");
        assert!(ink > 5, "pencil letter ink expected, got {ink}");
        assert!(glyph5x7('H').is_some() && glyph5x7('Z').is_some());
        let _ = dark;
    }

    /// P3#15: chalkBlot + handwritingFont from disk when graphics tree present.
    #[test]
    fn speech_bubble_with_handwriting_tga_if_present() {
        let sprites = HudSprites::with_default_roots(None);
        let hand_path = PathBuf::from(
            r"C:\OhOl\OpenLife\OneLifeGameSourceData\graphics\font_handwriting_32_32.tga",
        );
        if !hand_path.is_file() {
            return;
        }
        assert!(sprites.handwriting_from_disk);
        assert!(sprites.handwriting_font.is_some());
        assert!(sprites.chalk_blot.is_some());

        let mut fb = Framebuffer::new(200, 80);
        fb.clear([0, 0, 0, 255]);
        sprites.draw_speech_bubble(&mut fb, "HI", 100.0, 40.0, 1.0, 1.0);
        let bright = fb
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 150 && p[1] > 150 && p[2] > 150)
            .count();
        // Handwriting ink is pure black on chalk — count near-black non-bg (any channel).
        // After alpha blend onto black chalk, ink darkens chalk; count any non-black.
        let painted = fb
            .pixels
            .chunks_exact(4)
            .filter(|p| (p[0] as u32) + (p[1] as u32) + (p[2] as u32) > 0)
            .count();
        assert!(
            painted > 50,
            "handwriting speech should paint pixels, got {painted} (bright={bright})"
        );
        assert!(
            bright > 10 || painted > 100,
            "expect chalk and/or ink coverage bright={bright} painted={painted}"
        );

        // 5×7 path still works with empty roots (no TGA).
        let empty = HudSprites::load_from_roots(&[]);
        assert!(!empty.handwriting_from_disk);
        let mut fb2 = Framebuffer::new(120, 40);
        fb2.clear([0, 0, 0, 255]);
        empty.draw_speech_bubble(&mut fb2, "HI", 60.0, 20.0, 2.0, 1.0);
        let painted2 = fb2
            .pixels
            .chunks_exact(4)
            .filter(|p| (p[0] as u32) + (p[1] as u32) + (p[2] as u32) > 0)
            .count();
        assert!(painted2 > 20, "5x7 fallback must still paint, got {painted2}");
    }
}
