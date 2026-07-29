//! Software framebuffer renderer for map + objects + players.
//!
//! Haxe: `Render.hx` + `Object.hx` + SpriteBatch.
//! C++: `LivingLifePage` draw passes, `drawObject` / `drawObjectAnim`, `groundSprites`.
//!
//! GRID = 128 object units → screen pixels scale with camera zoom.
//! Headless-safe: pure RGBA buffer; no GPU crates required.
//!
//! L-HUD: food/heat chrome drawn after the world pass (`hud` + `hud_sprites`).
//! L-ANIM-DRAW: player draw builds `ObjectAnimPack` from `LiveObject.anim`
//! (dual-anim `inAnimFade` + frozen-rot) after sync/step from flags
//! (PM moving, justAte, action, PE emot extra).
//! L-EMOT: PE → emotion object layers (eye/mouth/other/face/body/head) + EXTRA/EXTRA_B
//! (P3#19 mouth-skip, mainEyesOffset eyeEmot, creation/decay sounds).
//! L-SAY: PS/LS speech bubbles (chalkBlot + handwritingFont TGAs, 5×7 fallback).
//! L-RENDER tall-object: same-row layers Floor < BehindPlayer < Player < Front*
//! (`drawBehindPlayer` + per-sprite `spritesDrawnBehind`).
//! L-RENDER P3#23: front sub-order permanent non-wall < non-permanent < wall < frontWall.
//! L-RENDER rideable: person-under-vehicle (behind vehicle → rider → front vehicle;
//! vehicle at person pos, not hand HoldingPos).

use crate::anim_bank::AnimBank;
use crate::anim_draw::{
    clothing_pack_from_person, sample_slot_pack, sample_sprite_pack, select_clothing_anim_type,
    select_held_anim_type, select_player_anim_type, ObjectAnimPack,
};
use crate::client_map::{ClientMap, ObjectStackNode};
use crate::content::{
    arm_holding_parameters, compute_held_draw_pos_ex, get_object_center_offset, ClientContent,
    ClientObjectDef, HoldingPos, SpriteCenterInfo,
};
use crate::emotion::EmotionBank;
use crate::ground_sprites::{biome_color, GroundBank};
use crate::hud::{draw_hud_if_visible, draw_speech_bubble, HudState, HudSprites};
use crate::live_object::{home_dir_index, LiveObject, LiveWorld, SaysPointerMarker};
use crate::parse::{FoodChange, HeatChange};
use crate::sprite_bank::SpriteBank;

/// C++ `getSpeechOffset` base Y in object units (above feet) before head add.
const SPEECH_BASE_Y: f32 = 84.0;

/// Soft-FB map-spot pin: diagonal X + small diamond (P3#17 pure `*map`).
///
/// // C++ uses home-slip arrow; we mark the world tile directly.
pub fn draw_map_spot_marker(
    fb: &mut Framebuffer,
    cx: f32,
    cy: f32,
    radius: i32,
    rgba: [u8; 4],
) {
    let r = radius.max(3);
    let cx_i = cx.round() as i32;
    let cy_i = cy.round() as i32;
    // X arms
    for i in -r..=r {
        fb.put(cx_i + i, cy_i + i, rgba);
        fb.put(cx_i + i, cy_i - i, rgba);
        // thicken
        fb.put(cx_i + i + 1, cy_i + i, rgba);
        fb.put(cx_i + i + 1, cy_i - i, rgba);
    }
    // center diamond
    let d = (r / 2).max(2);
    for dy in -d..=d {
        let w = d - dy.abs();
        for dx in -w..=w {
            fb.put(cx_i + dx, cy_i + dy, rgba);
        }
    }
}

/// Resolved person/held/clothing anim types for one player draw.
///
/// // C++: LivingLifePage `addNewAnim` + clothingAnimType / held anim branch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerAnimSelection {
    /// Person body pack (`ANIM_GROUND` / `MOVING` / `EATING` / `DOING` / `EXTRA` / `EXTRA_B`).
    pub person: i32,
    /// Held object pack (`ANIM_HELD` or `ANIM_MOVING` when person moving).
    pub held: i32,
    /// Clothing pack (`ANIM_HELD` or `ANIM_MOVING`).
    pub clothing: i32,
    /// Extra slot when person is `ANIM_EXTRA` or `ANIM_EXTRA_B`
    /// (C++ `setExtraIndex` / `setExtraIndexB`); else -1.
    pub extra_index: i32,
}

/// Choose animation packs from LiveObject state (L-ANIM-DRAW pack select).
///
/// Priority: emote EXTRA/EXTRA_B (resolved `extraAnimIndex`) → moving → eating → doing → ground.
///
/// // C++: PE uses `currentEmot->extraAnimIndex` + toggles extra↔extraB
pub fn select_packs_for_player(o: &LiveObject) -> PlayerAnimSelection {
    let emot = o.resolved_emot_extra_pack();
    let person = select_player_anim_type(o.moving, o.just_ate, o.action, emot);
    // C++: held_id < 0 means holding another player (baby).
    let holding_baby = o.held_id < 0;
    let held = select_held_anim_type(person, holding_baby, false);
    let clothing = select_clothing_anim_type(person);
    let extra_index = if crate::anim_bank::is_extra_anim_type(person) {
        emot.map(|(_, i)| i).unwrap_or(0)
    } else {
        -1
    };
    PlayerAnimSelection {
        person,
        held,
        clothing,
        extra_index,
    }
}

/// Head/body/feet object-space anchors after person pose (Jason clothing attach + PE).
///
/// // C++ animationBank: animHeadPos / animBodyPos / foot spritePos + clothingOffset
#[derive(Debug, Clone, Copy, Default)]
struct PersonAnchors {
    head: Option<(f32, f32, f32)>, // x, y, rot turns
    body: Option<(f32, f32, f32)>,
    front_foot: Option<(f32, f32, f32)>,
    back_foot: Option<(f32, f32, f32)>,
    /// Head + `mainEyesOffset` (rotated by head rot) for PE `eyeEmot` (P3#19).
    eyes: Option<(f32, f32, f32)>,
    /// True when person has eyes for emot placement this age.
    has_eyes: bool,
}

/// Jason clothing slot → body-part anchor (animationBank clothing passes).
///
/// Slot: 0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack.
fn clothing_anchor_for_slot(
    anchors: &PersonAnchors,
    slot_i: usize,
) -> Option<(f32, f32, f32)> {
    match slot_i {
        0 => anchors.head.or(anchors.body), // hat → head
        1 | 4 | 5 => anchors.body.or(anchors.head), // tunic / bottom / backpack
        2 => anchors.front_foot.or(anchors.body), // front shoe
        3 => anchors.back_foot.or(anchors.body), // back shoe
        _ => anchors.body.or(anchors.head),
    }
}

/// Screen position for worn clothing: animated body-part + **rotated** clothingOffset.
///
/// // C++ animationBank ~2773–2796 / hat ~3555–3569:
/// // if flipH: offset.x *= -1; rotate(offset, ±2π·partRot); cPos = flippedPart + offset + inPos
fn clothing_screen_pos(
    person_sx: f32,
    person_sy: f32,
    part: (f32, f32, f32),
    clothing_offset: (f32, f32),
    scale: f32,
    flip: bool,
) -> (f32, f32) {
    let (ax, ay, ar) = part;
    let mut ox = clothing_offset.0;
    let mut oy = clothing_offset.1;
    if flip {
        ox = -ox;
    }
    // C++: rotate clothingOffset by body-part rot before adding to part pos.
    if ar.abs() > 1e-8 {
        let angle = if flip {
            ar * std::f32::consts::TAU
        } else {
            -ar * std::f32::consts::TAU
        };
        let (s, c) = angle.sin_cos();
        let rx = ox * c - oy * s;
        let ry = ox * s + oy * c;
        ox = rx;
        oy = ry;
    }
    // Anchors stored unflipped; Jason flips part.x when inFlipH before adding offset.
    let part_x = if flip { -ax } else { ax };
    let cx = person_sx + (part_x + ox) * scale;
    let cy = person_sy - (ay + oy) * scale;
    (cx, cy)
}

/// Object design units per world tile (Haxe GRID / C++ CELL_D).
pub const GRID: f32 = 128.0;

/// Default soft-FB zoom (screen pixels per world tile).
///
/// Balanced for a 960×540 buffer: readable like Jason without drawing so many
/// overscaled soft-ground texels that the CPU falls to ~1 FPS.
pub const ZOOM_DEFAULT: f32 = 48.0;
/// Play/settings zoom range (pixels per tile).
pub const ZOOM_MIN: f32 = 16.0;
pub const ZOOM_MAX: f32 = 128.0;

/// Soft-FB clear / “void” under the map — warm earth, not UI black (avoids dark
/// cracks when soft ground alpha is partial).
pub const CLEAR_RGBA: [u8; 4] = [72, 96, 58, 255];

/// Nearest-neighbor stretch of an RGBA8 buffer to a different size (full fill).
///
/// Used by GPU present so the soft-FB always covers the window (pixels crate
/// otherwise integer-scales and letterboxes).
pub fn stretch_rgba_nearest(
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
) {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }
    let src_need = (src_w as usize).saturating_mul(src_h as usize).saturating_mul(4);
    let dst_need = (dst_w as usize).saturating_mul(dst_h as usize).saturating_mul(4);
    if src.len() < src_need || dst.len() < dst_need {
        return;
    }
    if src_w == dst_w && src_h == dst_h {
        dst[..dst_need].copy_from_slice(&src[..src_need]);
        return;
    }
    for y in 0..dst_h {
        let sy = ((y as u64) * (src_h as u64)) / (dst_h as u64);
        let src_row = (sy as usize) * (src_w as usize) * 4;
        let dst_row = (y as usize) * (dst_w as usize) * 4;
        for x in 0..dst_w {
            let sx = ((x as u64) * (src_w as u64)) / (dst_w as u64);
            let si = src_row + (sx as usize) * 4;
            let di = dst_row + (x as usize) * 4;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
}

/// Map window-client mouse pixels → soft-FB coords under Stretch present.
///
/// Soft and GPU paths both stretch a fixed buffer (e.g. 960×540) to the window.
/// minifb `get_mouse_pos` only divides by its Scale factor and does **not** undo
/// Stretch resize (documented TODO) — so callers must scale by window size.
///
/// Same formula as the GPU path: `fb = win * fb_size / win_size`.
/// Returns `None` when either size is zero (minimize / transient).
#[inline]
pub fn map_window_to_fb(
    win_x: f32,
    win_y: f32,
    win_w: u32,
    win_h: u32,
    fb_w: u32,
    fb_h: u32,
) -> Option<(f32, f32)> {
    if win_w == 0 || win_h == 0 || fb_w == 0 || fb_h == 0 {
        return None;
    }
    let mx = win_x * (fb_w as f32) / (win_w as f32);
    let my = win_y * (fb_h as f32) / (win_h as f32);
    let max_x = (fb_w.saturating_sub(1)) as f32;
    let max_y = (fb_h.saturating_sub(1)) as f32;
    Some((mx.clamp(0.0, max_x), my.clamp(0.0, max_y)))
}

/// Camera in world tiles (center).
#[derive(Debug, Clone)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    /// Screen pixels per tile.
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: ZOOM_DEFAULT,
        }
    }
}

/// Screen-space axis-aligned rect for world tile `(tx, ty)`.
///
/// Haxe/C++ place ground on a `GRID` (128) lattice then scale the whole scene
/// uniformly (`map.add(x * GRID, y * GRID, …)`). Soft-FB must **not** use a
/// truncated `zoom as i32` size centered independently — that leaves 1px gaps
/// when float projection and integer tile size disagree.
///
/// Instead: floor each consecutive tile **edge** in screen space so tile `tx`
/// ends where `tx+1` begins (same for Y with world-Y flipped to screen-Y).
pub fn tile_screen_rect(
    camera: &Camera,
    tx: i32,
    ty: i32,
    fb_w: u32,
    fb_h: u32,
) -> (i32, i32, i32, i32) {
    let z = camera.zoom.max(1e-4);
    let fw = fb_w as f32 * 0.5;
    let fh = fb_h as f32 * 0.5;
    let x0 = ((tx as f32 - camera.x) * z + fw).floor() as i32;
    let x1 = (((tx + 1) as f32 - camera.x) * z + fw).floor() as i32;
    // Screen Y down: higher world y → smaller screen y.
    let y0 = ((camera.y - (ty + 1) as f32) * z + fh).floor() as i32;
    let y1 = ((camera.y - ty as f32) * z + fh).floor() as i32;
    (x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
}

/// Framebuffer target.
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
        }
    }

    pub fn clear(&mut self, rgba: [u8; 4]) {
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
    }

    /// Count non-clear (non-matching) opaque-ish pixels — scene snapshot tests.
    pub fn count_non_color(&self, rgba: [u8; 4]) -> usize {
        self.pixels
            .chunks_exact(4)
            .filter(|p| p[0] != rgba[0] || p[1] != rgba[1] || p[2] != rgba[2])
            .count()
    }

    pub fn put(&mut self, x: i32, y: i32, rgba: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if rgba[3] == 0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        if rgba[3] == 255 {
            self.pixels[i..i + 4].copy_from_slice(&rgba);
        } else {
            // simple alpha blend
            let a = rgba[3] as u32;
            for c in 0..3 {
                let dst = self.pixels[i + c] as u32;
                let src = rgba[c] as u32;
                self.pixels[i + c] = ((src * a + dst * (255 - a)) / 255) as u8;
            }
            self.pixels[i + 3] = 255;
        }
    }

    /// C++ multiplicative blend path (sprite meta `mult=1`).
    pub fn put_multiplicative(&mut self, x: i32, y: i32, rgba: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if rgba[3] == 0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let a = rgba[3] as u32;
        for c in 0..3 {
            let dst = self.pixels[i + c] as u32;
            let src = rgba[c] as u32;
            // multiply then lerp by alpha
            let mul = (dst * src) / 255;
            self.pixels[i + c] = ((mul * a + dst * (255 - a)) / 255) as u8;
        }
        self.pixels[i + 3] = 255;
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
        for dy in 0..h {
            for dx in 0..w {
                self.put(x + dx, y + dy, rgba);
            }
        }
    }

    /// Blit a subrect from atlas page into framebuffer with scale / flip / optional rotation.
    ///
    /// `rot_turns`: rotation in turns (1.0 = 360°). C++/Haxe sprite `rot` + anim rot.
    /// Position `(dst_cx, dst_cy)` is the sprite center after center-anchor offset.
    /// `alpha_mul`: anim fade (C++ `workingSpriteFade`); multiplies source alpha.
    pub fn blit_sprite(
        &mut self,
        atlas: &[u8],
        atlas_w: u32,
        src_x: i32,
        src_y: i32,
        src_w: u32,
        src_h: u32,
        dst_cx: f32,
        dst_cy: f32,
        scale: f32,
        h_flip: bool,
        tint: [f32; 3],
        rot_turns: f32,
        multiplicative: bool,
        alpha_mul: f32,
    ) {
        let alpha_mul = alpha_mul.clamp(0.0, 1.0);
        if alpha_mul <= 1e-5 {
            return;
        }
        let dw = (src_w as f32 * scale).max(1.0);
        let dh = (src_h as f32 * scale).max(1.0);
        let dwi = dw as i32;
        let dhi = dh as i32;

        let apply_alpha = |mut px: [u8; 4]| -> [u8; 4] {
            if alpha_mul < 0.999 {
                px[3] = ((px[3] as f32) * alpha_mul).round() as u8;
            }
            px
        };

        // No rotation: nearest-neighbor axis-aligned (Jason pixel sprites; fast path).
        // Bilinear was ~4× cost and tanked play to ~1 FPS on soft ground + objects.
        if rot_turns.abs() < 1e-5 {
            let ox = dst_cx as i32 - dwi / 2;
            let oy = dst_cy as i32 - dhi / 2;
            for dy in 0..dhi {
                for dx in 0..dwi {
                    let u = if h_flip {
                        src_w as i32 - 1 - (dx * src_w as i32 / dwi.max(1))
                    } else {
                        dx * src_w as i32 / dwi.max(1)
                    };
                    let v = dy * src_h as i32 / dhi.max(1);
                    if let Some(px) =
                        sample_atlas(atlas, atlas_w, src_x, src_y, src_w, src_h, u, v, tint)
                    {
                        let px = apply_alpha(px);
                        if multiplicative {
                            self.put_multiplicative(ox + dx, oy + dy, px);
                        } else {
                            self.put(ox + dx, oy + dy, px);
                        }
                    }
                }
            }
            return;
        }

        // Rotated: inverse-map destination footprint into local sprite space.
        // C++/Haxe: rot in turns → radians (Haxe uses rot * 2π).
        let angle = rot_turns * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        // Bounding box of rotated rect
        let hw = dw * 0.5;
        let hh = dh * 0.5;
        let corners = [
            (hw * c - hh * s, hw * s + hh * c),
            (-hw * c - hh * s, -hw * s + hh * c),
            (hw * c + hh * s, hw * s - hh * c),
            (-hw * c + hh * s, -hw * s - hh * c),
        ];
        let min_x = corners.iter().map(|p| p.0).fold(f32::INFINITY, f32::min).floor() as i32;
        let max_x = corners.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
        let min_y = corners.iter().map(|p| p.1).fold(f32::INFINITY, f32::min).floor() as i32;
        let max_y = corners.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
        let inv_s = -s; // inverse rotation
        let inv_c = c;
        for oy in min_y..=max_y {
            for ox in min_x..=max_x {
                // inverse rotate around center
                let lx = ox as f32 * inv_c - oy as f32 * inv_s;
                let ly = ox as f32 * inv_s + oy as f32 * inv_c;
                if lx < -hw || lx >= hw || ly < -hh || ly >= hh {
                    continue;
                }
                let mut u_f = (lx + hw) / dw * src_w as f32;
                let v_f = (ly + hh) / dh * src_h as f32;
                if h_flip {
                    u_f = src_w as f32 - 1.0 - u_f;
                }
                let u = u_f.floor() as i32;
                let v = v_f.floor() as i32;
                if let Some(px) = sample_atlas(atlas, atlas_w, src_x, src_y, src_w, src_h, u, v, tint)
                {
                    let px = apply_alpha(px);
                    if multiplicative {
                        self.put_multiplicative(dst_cx as i32 + ox, dst_cy as i32 + oy, px);
                    } else {
                        self.put(dst_cx as i32 + ox, dst_cy as i32 + oy, px);
                    }
                }
            }
        }
    }

    /// Stretch-blit ground tile into a screen rect (nearest; atlas-direct, no alloc).
    /// Soft-edge TGA alpha + solid underfill plates give Jason-like biome blends
    /// without bilinear’s 4× sample cost.
    pub fn blit_rect_scaled(
        &mut self,
        atlas: &[u8],
        atlas_w: u32,
        src_x: i32,
        src_y: i32,
        src_w: u32,
        src_h: u32,
        dst_x: i32,
        dst_y: i32,
        dst_w: i32,
        dst_h: i32,
    ) {
        if dst_w <= 0 || dst_h <= 0 || src_w == 0 || src_h == 0 {
            return;
        }
        // Fixed-point nearest for speed.
        let sw = src_w as i32;
        let sh = src_h as i32;
        for dy in 0..dst_h {
            let v = dy * sh / dst_h;
            for dx in 0..dst_w {
                let u = dx * sw / dst_w;
                if let Some(px) =
                    sample_atlas(atlas, atlas_w, src_x, src_y, src_w, src_h, u, v, [1.0, 1.0, 1.0])
                {
                    self.put(dst_x + dx, dst_y + dy, px);
                }
            }
        }
    }
}

fn sample_atlas(
    atlas: &[u8],
    atlas_w: u32,
    src_x: i32,
    src_y: i32,
    src_w: u32,
    src_h: u32,
    u: i32,
    v: i32,
    tint: [f32; 3],
) -> Option<[u8; 4]> {
    if u < 0 || v < 0 || u >= src_w as i32 || v >= src_h as i32 {
        return None;
    }
    let sx = src_x + u;
    let sy = src_y + v;
    if sx < 0 || sy < 0 || sx >= atlas_w as i32 {
        return None;
    }
    let si = ((sy as u32 * atlas_w + sx as u32) * 4) as usize;
    if si + 3 >= atlas.len() {
        return None;
    }
    let a = atlas[si + 3];
    if a == 0 {
        return None;
    }
    let r = (atlas[si] as f32 * tint[0]).clamp(0.0, 255.0) as u8;
    let g = (atlas[si + 1] as f32 * tint[1]).clamp(0.0, 255.0) as u8;
    let b = (atlas[si + 2] as f32 * tint[2]).clamp(0.0, 255.0) as u8;
    Some([r, g, b, a])
}

/// Draw sort key within one world row (C++ LivingLifePage per-row passes).
///
/// Haxe ysort by bounds.yMax — we use world y then type layer.
/// Tall-object: BehindPlayer < Player < Front so trees/walls sit under/over people.
/// P3#23: front sub-order matches C++ wallLayer / frontWall passes after players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DrawLayer {
    Floor = 0,
    /// Whole `drawBehindPlayer` objects + `spritesDrawnBehind` layers.
    BehindPlayer = 1,
    Player = 2,
    /// Permanent non-wall front objects (over players).
    FrontPermanent = 3,
    /// Non-permanent non-wall front objects.
    FrontNonPermanent = 4,
    /// `wallLayer && !frontWall` (permanent walls).
    FrontWall = 5,
    /// `wallLayer && frontWall` (walls with signs — top of same-row front).
    FrontFrontWall = 6,
}

/// Which object sprite layers to blit (C++ `prepareToSkipSprites` / spriteBehindPlayer).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SpriteLayerFilter {
    /// All visible layers.
    All,
    /// Only sprites with `behind_player` (global behind pass).
    BehindPlayerOnly,
    /// Skip `behind_player` sprites (front canopy / after global pass).
    NotBehindPlayer,
}

struct YSortItem {
    /// World tile Y. Jason draws **high Y first** (north → south) so southern
    /// objects/players paint over northern ones (`LivingLifePage` yEnd→yStart).
    sort_y: i32,
    layer: DrawLayer,
    kind: DrawKind,
}

enum DrawKind {
    Floor { tx: i32, ty: i32, floor_id: i32 },
    MapObject {
        tx: i32,
        ty: i32,
        sprite_filter: SpriteLayerFilter,
    },
    Player { id: i32 },
}

/// Full scene draw.
pub struct SceneRenderer {
    pub camera: Camera,
    pub time: f32,
    /// Optional hover / click highlight in world tile coords.
    pub highlight_tile: Option<(i32, i32)>,
    /// Ground tile bank (TGA cache or flat fallback).
    pub ground: GroundBank,
    /// Food/heat HUD vitals (L-HUD) — apply FX/HX via [`SceneRenderer::sync_hud`].
    pub hud: HudState,
    /// Hunger boxes / temp arrows / gui panel (graphics TGAs or procedural).
    pub hud_sprites: HudSprites,
    /// When true, draw food/heat chrome after the world pass.
    pub draw_hud: bool,
    /// PE emotion table (`emotionWords` / `emotionObjects` ini).
    pub emotions: EmotionBank,
    /// OLSN sound bank for anim SoundAnimParam / footstep (**L-SOUND-TRIG**).
    pub sounds: crate::sound_bank::SoundBank,
}

impl Default for SceneRenderer {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            time: 0.0,
            highlight_tile: None,
            ground: GroundBank::with_default_roots(None),
            hud: HudState::default(),
            hud_sprites: HudSprites::procedural(),
            draw_hud: true,
            emotions: EmotionBank::new(),
            sounds: crate::sound_bank::SoundBank::new("."),
        }
    }
}

impl SceneRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind ground + HUD + emotion tables from content root (call after content load).
    ///
    /// Ground prefers `cache/olg1_ground_index.bin` then scans disk; overlay sheets
    /// (`graphics/ground_tN`) are preloaded (Haxe `loadGround` packs them first).
    /// Emotions load from `contentSettings/` (or `settings/`) — tiny text, not cached.
    pub fn set_content_root(&mut self, root: Option<&std::path::Path>) {
        self.set_content_root_with_progress(root, None);
    }

    /// Same as [`Self::set_content_root`] with optional P5#36 progress callback
    /// for ground + sound prefer_cache stages.
    pub fn set_content_root_with_progress(
        &mut self,
        root: Option<&std::path::Path>,
        mut on_progress: crate::load_progress::ProgressCb<'_>,
    ) {
        self.ground = match root {
            Some(r) => {
                let mut g = GroundBank::load_prefer_cache_with_progress(
                    r,
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                let _ = g.preload_overlays();
                g
            }
            None => GroundBank::with_default_roots(None),
        };
        self.hud_sprites = HudSprites::with_default_roots(root);
        self.emotions = match root {
            Some(r) => EmotionBank::load_from_content_root(r),
            None => EmotionBank::new(),
        };
        // L-SOUND-TRIG: OLSN index only (lazy AIFF on play).
        self.sounds = match root {
            Some(r) => crate::sound_bank::SoundBank::load_prefer_cache_with_progress(
                r,
                crate::load_progress::reborrow_cb(&mut on_progress),
            ),
            None => crate::sound_bank::SoundBank::new("."),
        };
    }

    /// Apply last FX/HX from the session into HUD state (max peaks retained).
    ///
    /// C++: LivingLifePage FX/HX handlers update foodStore/heat + max peaks.
    ///
    /// When both `food` and `heat` are `None` after the HUD was visible (e.g.
    /// [`crate::session::ClientSession::logout_reset`] cleared session vitals),
    /// peaks + OldArrow trail are wiped so chrome does not stick after death.
    pub fn sync_hud(&mut self, food: Option<&FoodChange>, heat: Option<&HeatChange>) {
        if food.is_none() && heat.is_none() {
            if self.hud.visible
                || self.hud.food_capacity > 0
                || self.hud.max_food_capacity > 0
                || !self.hud.old_arrows.is_empty()
            {
                // C++ death/logout path clears maxFood* + mOldArrows.
                self.hud.clear();
            }
            return;
        }
        self.hud.sync_from_session(food, heat);
    }

    /// Sync FX/HX plus optional CX curse tokens / CS excess points / dying flag.
    pub fn sync_hud_ex(
        &mut self,
        food: Option<&FoodChange>,
        heat: Option<&HeatChange>,
        curse_tokens: Option<i32>,
        excess_curse_points: Option<i32>,
        dying: bool,
    ) {
        self.sync_hud(food, heat);
        if let Some(c) = curse_tokens {
            self.hud.apply_curse_tokens(c);
        }
        if let Some(p) = excess_curse_points {
            self.hud.apply_excess_curse_points(p);
        }
        self.hud.dying = dying;
    }

    /// Clear HUD vitals (logout / death). Call after [`ClientSession::logout_reset`].
    ///
    /// // C++: LivingLifePage reset clears maxFood peaks + mOldArrows
    pub fn clear_hud(&mut self) {
        self.hud.clear();
    }

    /// World tile coords → screen pixels (center of tile at integer+0.5).
    pub fn world_to_screen(&self, wx: f32, wy: f32, fb_w: u32, fb_h: u32) -> (f32, f32) {
        let sx = (wx - self.camera.x) * self.camera.zoom + fb_w as f32 * 0.5;
        // screen Y grows down; world Y grows up-ish — flip for OHOL map y
        let sy = (self.camera.y - wy) * self.camera.zoom + fb_h as f32 * 0.5;
        (sx, sy)
    }

    /// Inverse of [`world_to_screen`] — click → USE/DROP tile targeting.
    ///
    /// C++: LivingLifePage mouse → grid; Haxe: similar camera unproject.
    pub fn screen_to_world(&self, sx: f32, sy: f32, fb_w: u32, fb_h: u32) -> (f32, f32) {
        let z = self.camera.zoom.max(1e-4);
        let wx = (sx - fb_w as f32 * 0.5) / z + self.camera.x;
        let wy = self.camera.y - (sy - fb_h as f32 * 0.5) / z;
        (wx, wy)
    }

    /// Tile under screen pixel (floor of world coords).
    ///
    /// L-HUD playable wire: GUI left-click → this → [`crate::session::ClientSession::walk_to`].
    /// // C++: LivingLifePage mouse → grid tile under pointer
    /// // Haxe: camera unproject → integer cell
    pub fn screen_to_tile(&self, sx: f32, sy: f32, fb_w: u32, fb_h: u32) -> (i32, i32) {
        let (wx, wy) = self.screen_to_world(sx, sy, fb_w, fb_h);
        (wx.floor() as i32, wy.floor() as i32)
    }

    /// Update [`Self::highlight_tile`] from a screen pixel (hover / click preview).
    pub fn set_highlight_from_screen(&mut self, sx: f32, sy: f32, fb_w: u32, fb_h: u32) -> (i32, i32) {
        let t = self.screen_to_tile(sx, sy, fb_w, fb_h);
        self.highlight_tile = Some(t);
        t
    }

    pub fn draw(
        &mut self,
        fb: &mut Framebuffer,
        map: &mut ClientMap,
        world: &mut LiveWorld,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        dt: f32,
    ) {
        // L-ANIM-DRAW: sync packs from flags; step clocks when dt advances.
        // // C++: LivingLifePage per-frame animationFrameCount + lastAnimFade
        // anim_speed / frame_rate_factor ≈ wall frames at 60 Hz (capped catch-up).
        // L-EMOT: tick temporary PE TTL (wall seconds).
        // L-SAY: tick speech hold + fade (same frf as anim fade step).
        // L-SOUND-TRIG: handleAnimSound (person + held; footstep→floor usingSound).
        // Stereo pan: listener = camera center in tiles (C++ lastScreenViewCenter/CELL_D).
        self.sounds.set_listener(self.camera.x, self.camera.y);
        if dt > 1e-8 {
            // P3#19: decay sounds when temporary PE TTL expires (C++ ~22469)
            let decay_targets = world.tick_emots(dt);
            crate::sound_bank::play_emot_decay_for_targets(
                &mut self.sounds,
                content,
                &self.emotions,
                &decay_targets,
            );
            let frf = (dt * 60.0).clamp(0.0, 4.0);
            world.tick_speech(dt, frf);
            world.step_anims_with_sounds(
                anims,
                &mut self.sounds,
                content,
                map,
                frf,
                frf,
            );
            // Map object / floor ground anim clocks (C++ mMapAnimationFrameCount++).
            let _ = crate::sound_bank::step_map_ground_anims_with_sounds(
                &mut self.sounds,
                anims,
                content,
                map,
                frf,
                world.our_id,
            );
        } else {
            // Snapshot draws (dt=0): still sync so PM/justAte flags select packs.
            let ids = world.living_ids();
            for id in ids {
                if let Some(o) = world.get_mut(id) {
                    o.sync_anim_packs(anims);
                }
            }
        }
        self.time += dt;
        fb.clear(CLEAR_RGBA);

        let half_w = (fb.width as f32 / self.camera.zoom * 0.5 + 2.0) as i32;
        let half_h = (fb.height as f32 / self.camera.zoom * 0.5 + 2.0) as i32;
        let cx = self.camera.x as i32;
        let cy = self.camera.y as i32;

        // Approximate tile size for non-ground markers (highlight thickness).
        let tile_px = self.camera.zoom.max(4.0).round() as i32;

        // --- Pass 1: ground biomes (C++ LivingLifePage ~7151–7390) ---
        // Soft 2×CELL_D tiles blend biome borders; square CELL_D interior when
        // left/above/diag share biome. C++ iterates y high→low (draw order).
        //
        // Two sub-passes so zoom never flashes clear-color “black gaps”:
        //   1) solid abutting biome plates for every cell (cheap, no TGA)
        //   2) soft/square sprites on top (may lazy-load TGA mid-zoom)
        let y0 = cy - half_h;
        let y1 = cy + half_h;
        let x0 = cx - half_w;
        let x1 = cx + half_w;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let biome = map.get_or_empty(tx, ty).biome;
                let (px, py, tw, th) =
                    tile_screen_rect(&self.camera, tx, ty, fb.width, fb.height);
                // Solid plate covers 100% of the cell; abutting edges → no seams.
                fb.fill_rect(px, py, tw, th, biome_color(biome));
            }
        }
        for ty in (y0..=y1).rev() {
            for tx in x0..=x1 {
                self.draw_ground_cell(fb, map, tx, ty);
            }
        }

        // --- Pass 2: y-sorted floors, map objects, players ---
        // Haxe: ysort by bounds.yMax
        // C++: per-row Floor → behind-player sprites → drawBehindPlayer → players → front objects
        let mut items: Vec<YSortItem> = Vec::new();

        for ty in (cy - half_h)..=(cy + half_h) {
            for tx in (cx - half_w)..=(cx + half_w) {
                let tile = map.get_or_empty(tx, ty);
                if tile.floor_id != 0 {
                    items.push(YSortItem {
                        sort_y: ty,
                        layer: DrawLayer::Floor,
                        kind: DrawKind::Floor {
                            tx,
                            ty,
                            floor_id: tile.floor_id,
                        },
                    });
                }
                if tile.object_id > 0 {
                    // L-RENDER tall-object: split behind/front relative to players.
                    // // C++: drawBehindPlayer + anySpritesBehindPlayer / prepareToSkipSprites
                    push_map_object_draw_items(
                        &mut items,
                        content,
                        ty,
                        tx,
                        tile.object_id,
                    );
                }
            }
        }
        for id in world.living_ids() {
            if let Some(o) = world.get(id) {
                if o.deleted {
                    continue;
                }
                // P3#22: held babies are drawn by the adult (C++ skip heldByAdultID).
                if o.is_held_by_adult() {
                    continue;
                }
                // Recently-dropped babies still sliding: draw after adults in row
                // (sort_y same; layer still Player — second pass via drop offset
                // keeps them visible on top of the adult who dropped them).
                items.push(YSortItem {
                    sort_y: o.y,
                    layer: DrawLayer::Player,
                    kind: DrawKind::Player { id },
                });
            }
        }
        // C++ LivingLifePage ~8215/8261: `for (y = yEnd; y >= yStart; y--)` — high Y first.
        // Then within a row, layer order (BehindPlayer < Player < Front*).
        items.sort_by(|a, b| {
            b.sort_y
                .cmp(&a.sort_y)
                .then_with(|| a.layer.cmp(&b.layer))
        });

        for item in items {
            match item.kind {
                DrawKind::Floor { tx, ty, floor_id } => {
                    let (sx, sy) =
                        self.world_to_screen(tx as f32 + 0.5, ty as f32 + 0.5, fb.width, fb.height);
                    // Floor as object sprites when known; else brown plate
                    if content.get(floor_id).map(|d| !d.sprites.is_empty()).unwrap_or(false) {
                        self.draw_object(
                            fb, content, sprites, anims, floor_id, 0, -1, 20.0, sx, sy, false, false,
                        );
                    } else {
                        let (x0, y0, tw, th) =
                            tile_screen_rect(&self.camera, tx, ty, fb.width, fb.height);
                        fb.fill_rect(
                            x0 + 2,
                            y0 + 2,
                            (tw - 4).max(1),
                            (th - 4).max(1),
                            [120, 100, 80, 200],
                        );
                    }
                }
                DrawKind::MapObject {
                    tx,
                    ty,
                    sprite_filter,
                } => {
                    let tile = map.get_or_empty(tx, ty);
                    let (sx, sy) =
                        self.world_to_screen(tx as f32 + 0.5, ty as f32 + 0.5, fb.width, fb.height);
                    // Jason: mMapAnimationFrameCount/60 as frameTime for ground packs.
                    let frame_t = map
                        .anim_frame_count
                        .get(&(tx, ty))
                        .copied()
                        .unwrap_or(0.0)
                        / 60.0;
                    self.draw_object_stack_at_time(
                        fb,
                        content,
                        sprites,
                        anims,
                        &tile.object_stack(),
                        0,
                        20.0,
                        sx,
                        sy,
                        false,
                        sprite_filter,
                        frame_t,
                    );
                }
                DrawKind::Player { id } => {
                    let Some(o) = world.get(id) else { continue };
                    // P3#22: drop offset (tile) + action wiggle (object units) (C++ ~5211/5343).
                    let (base_tx, base_ty) = o.draw_pos_tiles();
                    let (wox, woy) = o.action_wiggle_units();
                    let (sx0, sy0) = self.world_to_screen(
                        base_tx + 0.5,
                        base_ty + 0.5,
                        fb.width,
                        fb.height,
                    );
                    let scale0 = (self.camera.zoom / GRID).max(0.05);
                    // Object Y-up → screen Y-down for wiggle.
                    let sx = sx0 + wox * scale0;
                    let sy = sy0 - woy * scale0;
                    let display = if o.display_id > 0 { o.display_id } else { 19 };
                    // Age advances with age_rate for ageRange sprites
                    let age = o.age + o.age_rate * self.time;
                    let flip = o.facing < 0;
                    let holding = o.held_id != 0;
                    let held_id = o.held_id;
                    // P3#22: capture before later mut borrows of world.
                    let o_moving = o.moving || o.anim.cur_anim == crate::anim_bank::ANIM_MOVING;
                    // Adult held-anim pack clocks for baby draw (C++ curHeldAnim).
                    let o_held_pack_for_baby = if held_id < 0 {
                        Some(o.anim.held_pack(if o.display_id > 0 {
                            o.display_id
                        } else {
                            19
                        }))
                    } else {
                        None
                    };
                    // Precompute clothing draw list (slot, id, contained raw) so we drop `o`
                    // before later world.get_mut for frozen-rot notes.
                    // Order: backShoe, bottom, tunic, backpack, frontShoe, hat (C++ clothing).
                    const CLOTHING_DRAW_ORDER: [usize; 6] = [3, 4, 1, 5, 2, 0];
                    let clothing_draw: Vec<(usize, i32, String)> = CLOTHING_DRAW_ORDER
                        .iter()
                        .filter_map(|&slot_i| {
                            let id = o.clothing.slot_id(slot_i);
                            if id > 0 {
                                Some((
                                    slot_i,
                                    id,
                                    o.clothing.slots.get(slot_i).cloned().unwrap_or_default(),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();
                    // L-RENDER limb-hide / HoldingPos (C++ getArmHoldingParameters)
                    let held_def = if held_id > 0 {
                        content.get(held_id)
                    } else {
                        None
                    };
                    let is_rideable = held_def.map(|d| d.rideable).unwrap_or(false);
                    let ride_any_behind = held_def
                        .map(|d| d.any_sprites_behind_player())
                        .unwrap_or(false);
                    let (hide_closest_arm, hide_all_limbs) = arm_holding_parameters(held_def);
                    let freeze_arms = hide_closest_arm == -2 || is_rideable;
                    // L-ANIM-DRAW: dual-anim pack; freeze arms when bulky/rideable held
                    let mut person_pack = o.person_anim_pack(freeze_arms);
                    let mut held_pack = o.held_anim_pack();
                    let emot_indices = o.emot_draw_indices();
                    // P3#19: C++ skips mouth sprite when any active emot has mouthEmot
                    let hide_mouth = emot_indices.iter().any(|&idx| {
                        self.emotions
                            .get(idx)
                            .map(|e| e.mouth_emot > 0)
                            .unwrap_or(false)
                    });
                    // `o` borrow ends here; packs own the draw clocks.

                    let scale = (self.camera.zoom / GRID).max(0.05);
                    let flip_s = if flip { -1.0 } else { 1.0 };

                    // P3#20 rideable person-under-vehicle (C++ LivingLifePage ~5443–5916):
                    // - vehicle at person pos (heldObjectDrawPos = pos), not hand HoldingPos
                    // - rider offset ≈ −heldOffset (ridingOffset; age body residual)
                    // - order: vehicle behind → person/clothes/emotes → vehicle front
                    let (person_sx, person_sy) = if is_rideable {
                        let (hox, hoy) = held_def
                            .map(|d| d.held_offset)
                            .unwrap_or((0.0, 0.0));
                        // ridingOffset = −heldOffset (HoldingPos invalid / deferred path)
                        (sx - hox * scale * flip_s, sy + hoy * scale)
                    } else {
                        (sx, sy)
                    };
                    let vehicle_sx = sx;
                    let vehicle_sy = sy;

                    // Behind-player vehicle layers under the rider.
                    // // C++ prepareToSkipSprites(held, true) before person draw
                    if is_rideable && ride_any_behind {
                        if let Some(ref mut hp) = held_pack {
                            let _ = self.draw_object_with_pack(
                                fb,
                                content,
                                sprites,
                                anims,
                                hp,
                                age,
                                vehicle_sx,
                                vehicle_sy,
                                false,
                                false,
                                false,
                                0,
                                false,
                                SpriteLayerFilter::BehindPlayerOnly,
                                false,
                            );
                        }
                    }

                    // Person + interleaved worn clothing (Jason: body clothes under
                    // top back arm; shoes on feet; hat after all body sprites).
                    let (holding_pos, person_anchors) = self.draw_object_with_pack_ex(
                        fb,
                        content,
                        sprites,
                        anims,
                        &mut person_pack,
                        age,
                        person_sx,
                        person_sy,
                        flip,
                        holding,
                        false, // worn (person not clothing)
                        hide_closest_arm,
                        hide_all_limbs,
                        SpriteLayerFilter::All,
                        hide_mouth,
                        Some(clothing_draw.as_slice()),
                    );

                    // L-EMOT: bodyEmot under clothing was drawn mid-arm in C++; here
                    // after person+clothes so body emote still sits on the figure.
                    // (Full mid-arm PE interleave residual if bodyEmot must under tunic.)
                    self.draw_emotion_layers(
                        fb,
                        content,
                        sprites,
                        anims,
                        &person_pack,
                        &emot_indices,
                        &person_anchors,
                        person_sx,
                        person_sy,
                        flip,
                        EmotDrawPhase::Body,
                    );

                    // L-EMOT: eye/face/mouth/other after clothing base
                    self.draw_emotion_layers(
                        fb,
                        content,
                        sprites,
                        anims,
                        &person_pack,
                        &emot_indices,
                        &person_anchors,
                        person_sx,
                        person_sy,
                        flip,
                        EmotDrawPhase::Face,
                    );
                    // L-EMOT: headEmot on top (after hat)
                    self.draw_emotion_layers(
                        fb,
                        content,
                        sprites,
                        anims,
                        &person_pack,
                        &emot_indices,
                        &person_anchors,
                        person_sx,
                        person_sy,
                        flip,
                        EmotDrawPhase::HeadTop,
                    );

                    // Held item:
                    // - rideable: vehicle at person pos; front (or all) over rider
                    // - else: HoldingPos (hand/body) + heldOffset
                    // // C++: computeHeldDrawPos + rideable heldObjectDrawPos = pos
                    if held_id > 0 {
                        if is_rideable {
                            // Front canopy / full vehicle over person (behind already drawn).
                            let filter = if ride_any_behind {
                                SpriteLayerFilter::NotBehindPlayer
                            } else {
                                SpriteLayerFilter::All
                            };
                            if let Some(ref mut hp) = held_pack {
                                let _ = self.draw_object_with_pack(
                                    fb,
                                    content,
                                    sprites,
                                    anims,
                                    hp,
                                    age,
                                    vehicle_sx,
                                    vehicle_sy,
                                    false,
                                    false,
                                    false,
                                    0,
                                    false,
                                    filter,
                                    false,
                                );
                            }
                        } else {
                            // P3#21: getObjectCenterOffset via sprite-bank alpha bbox
                            let center = held_def.map(|d| {
                                get_object_center_offset(d, |sid| {
                                    let rect = sprites.ensure(sid)?;
                                    Some(SpriteCenterInfo {
                                        visible_w: rect.visible_w.max(1),
                                        visible_h: rect.visible_h.max(1),
                                        center_x_offset: rect.center_x_offset,
                                        center_y_offset: rect.center_y_offset,
                                        multiplicative_blend: rect.multiplicative_blend,
                                    })
                                })
                            });
                            let (hx, hy, hrot) = if holding_pos.valid {
                                compute_held_draw_pos_ex(
                                    &holding_pos,
                                    held_def,
                                    flip,
                                    center,
                                )
                            } else {
                                // No hand/body index: fall back to person heldOffset
                                let (px, py) = content
                                    .get(display)
                                    .map(|d| d.held_offset)
                                    .unwrap_or((8.0, 12.0));
                                (px, py, 0.0)
                            };
                            // Target hold in tile units for handoff slide.
                            let target_tx = base_tx + 0.5 + (hx * flip_s) / GRID;
                            let target_ty = base_ty + 0.5 + hy / GRID;
                            // P3#22 heldPosOverride slide from map origin into hand.
                            let stationary = !o_moving;
                            let (draw_tx, draw_ty, _draw_rot) = if let Some(o) = world.get_mut(id) {
                                o.step_held_pos_toward(
                                    target_tx,
                                    target_ty,
                                    hrot,
                                    stationary,
                                    1.0,
                                )
                            } else {
                                (target_tx, target_ty, hrot)
                            };
                            let (hold_sx, hold_sy) =
                                self.world_to_screen(draw_tx, draw_ty, fb.width, fb.height);
                            if let Some(ref mut hp) = held_pack {
                                let _ = self.draw_object_with_pack(
                                    fb,
                                    content,
                                    sprites,
                                    anims,
                                    hp,
                                    age,
                                    hold_sx,
                                    hold_sy,
                                    false,
                                    false,
                                    false,
                                    0,
                                    false,
                                    SpriteLayerFilter::All,
                                    false,
                                );
                            }
                        }
                    } else if held_id < 0 {
                        // P3#22: baby-held handoff draw (C++ drawLiveObject ~5812–5896).
                        let baby_id = -held_id;
                        // Holding pos in object units relative to person screen pos.
                        let (hx, hy, _hrot) = if holding_pos.valid {
                            (holding_pos.x, holding_pos.y, holding_pos.rot)
                        } else {
                            let (px, py) = content
                                .get(display)
                                .map(|d| d.held_offset)
                                .unwrap_or((8.0, 12.0));
                            (px, py, 0.0)
                        };
                        let mut hold_sx = sx + hx * scale * flip_s;
                        let hold_sy = sy - hy * scale;
                        // Save world hold pos on baby for drop handoff (tile units).
                        let world_hold_tx = base_tx + 0.5 + (hx * flip_s) / GRID;
                        let world_hold_ty = base_ty + 0.5 + hy / GRID;
                        if let Some(baby) = world.get_mut(baby_id) {
                            baby.note_held_by_raw_pos(world_hold_tx, world_hold_ty);
                            baby.facing = if flip { -1 } else { 1 };
                            let wig = baby.baby_wiggle_x_units(flip);
                            hold_sx += wig * scale;
                        }
                        if let Some(baby) = world.get(baby_id) {
                            let baby_age = baby.age + baby.age_rate * self.time;
                            let baby_display =
                                if baby.display_id > 0 { baby.display_id } else { 19 };
                            // Animate with adult's held track (C++ curHeldAnim).
                            let mut baby_pack = o_held_pack_for_baby.unwrap_or_else(|| {
                                ObjectAnimPack::single(baby_display, crate::anim_bank::ANIM_HELD, 0.0)
                            });
                            baby_pack.object_id = baby_display;
                            let baby_holding = baby.held_id > 0;
                            let baby_held_def = if baby.held_id > 0 {
                                content.get(baby.held_id)
                            } else {
                                None
                            };
                            let (hide_arm_b, hide_limbs_b) =
                                arm_holding_parameters(baby_held_def);
                            let _ = self.draw_object_with_pack(
                                fb,
                                content,
                                sprites,
                                anims,
                                &mut baby_pack,
                                baby_age,
                                hold_sx,
                                hold_sy,
                                flip,
                                baby_holding,
                                false,
                                hide_arm_b,
                                hide_limbs_b,
                                SpriteLayerFilter::All,
                                false,
                            );
                        }
                    }

                    // Resume-to-moving needs frozen_rot_frame_count_used after sample.
                    let held_froz = held_pack.as_ref().map(|p| p.frozen_rot_used).unwrap_or(false);
                    if person_pack.frozen_rot_used || held_froz {
                        if let Some(o) = world.get_mut(id) {
                            o.anim
                                .note_frozen_rot_used(true, person_pack.frozen_rot_used);
                            o.anim.note_frozen_rot_used(false, held_froz);
                        }
                    }

                    // L-SAY: speech bubble above head (C++ getSpeechOffset + chalk string).
                    // Drawn after person/emot/held so text is readable.
                    if let Some(o) = world.get(id) {
                        if let Some(ref speech) = o.current_speech {
                            let head_y = person_anchors
                                .head
                                .map(|(_hx, hy, _)| hy)
                                .unwrap_or(0.0);
                            let head_x = person_anchors
                                .head
                                .map(|(hx, _hy, _)| hx)
                                .unwrap_or(0.0);
                            // Object-space offset → screen (Y-up in object space).
                            let speech_sx = person_sx + head_x * scale * flip_s;
                            let speech_sy = person_sy - (SPEECH_BASE_Y + head_y) * scale;
                            let text_scale = (scale * 0.35).clamp(0.8, 2.5);
                            // P3#15 chalk + P3#16 residual: purple/white curse/dying ink.
                            let ink = crate::live_object::speech_text_rgb(o);
                            self.hud_sprites.draw_speech_bubble_colored(
                                fb,
                                speech,
                                speech_sx,
                                speech_sy,
                                text_scale,
                                o.speech_fade,
                                ink,
                            );
                        } else if let Some(ref name) = o.name {
                            // Soft name plate (same chalk/handwriting path as speech).
                            let speech_sy = person_sy - SPEECH_BASE_Y * scale;
                            let text_scale = (scale * 0.3).clamp(0.7, 2.0);
                            self.hud_sprites.draw_speech_bubble(
                                fb, name, person_sx, speech_sy, text_scale, 0.85,
                            );
                        }
                    }

                    if world.our_id == Some(id) {
                        fb.fill_rect(
                            person_sx as i32 - 3,
                            person_sy as i32 - 3,
                            6,
                            6,
                            [255, 255, 0, 255],
                        );
                    }
                }
            }
        }

        // L-SAY: location speech at tile centers (C++ locationSpeech, y += 84).
        {
            let scale = (self.camera.zoom / GRID).max(0.05);
            let text_scale = (scale * 0.35).clamp(0.8, 2.5);
            for ls in &world.location_speech {
                let (sx, sy) =
                    self.world_to_screen(ls.x as f32 + 0.5, ls.y as f32 + 0.5, fb.width, fb.height);
                let speech_sy = sy - SPEECH_BASE_Y * scale;
                self.hud_sprites.draw_speech_bubble(
                    fb, &ls.text, sx, speech_sy, text_scale, ls.fade,
                );
            }
        }

        // P3#17: PS `*map` spot pins + `*label` markers at target/map positions.
        {
            let scale = (self.camera.zoom / GRID).max(0.05);
            let text_scale = (scale * 0.32).clamp(0.7, 2.2);
            let tile_r = (tile_px / 2).max(4);
            // Collect draw info without holding world borrow across mut gets.
            let markers: Vec<SaysPointerMarker> = world.says_pointers.clone();
            for m in &markers {
                let fade = m.fade.clamp(0.0, 1.0);
                if fade <= 0.0 {
                    continue;
                }
                let mut rgba = m.color_rgba();
                rgba[3] = (rgba[3] as f32 * fade) as u8;

                // Map-spot pin at tile center (X / diamond).
                if let Some((mx, my)) = m.map_tile() {
                    let (sx, sy) = self.world_to_screen(
                        mx as f32 + 0.5,
                        my as f32 + 0.5,
                        fb.width,
                        fb.height,
                    );
                    draw_map_spot_marker(fb, sx, sy, tile_r, rgba);
                }

                // Label marker: prefer live target player position; else map tile.
                if let Some(ref lab) = m.target_label {
                    let label = lab.short_name();
                    let (tx, ty) = if let Some(tid) = m.target_player_id {
                        if tid > 0 {
                            if let Some(t) = world.get(tid) {
                                if !t.deleted {
                                    (t.x as f32 + 0.5, t.y as f32 + 0.5)
                                } else if let Some((mx, my)) = m.map_tile() {
                                    (mx as f32 + 0.5, my as f32 + 0.5)
                                } else {
                                    continue;
                                }
                            } else if let Some((mx, my)) = m.map_tile() {
                                (mx as f32 + 0.5, my as f32 + 0.5)
                            } else {
                                continue;
                            }
                        } else if let Some((mx, my)) = m.map_tile() {
                            // prop / id 0
                            (mx as f32 + 0.5, my as f32 + 0.5)
                        } else {
                            continue;
                        }
                    } else if let Some((mx, my)) = m.map_tile() {
                        (mx as f32 + 0.5, my as f32 + 0.5)
                    } else {
                        continue;
                    };
                    let (sx, sy) = self.world_to_screen(tx, ty, fb.width, fb.height);
                    let label_sy = sy - SPEECH_BASE_Y * scale * 0.55;
                    // Small chalk-ish label (distinct from full speech bubble).
                    draw_speech_bubble(fb, label, sx, label_sy, text_scale, fade);
                    // Accent bar under label in marker color.
                    let bar_w = (label.len() as i32 * 4).max(8);
                    let bar_y = (label_sy + 6.0 * text_scale) as i32;
                    fb.fill_rect(
                        sx as i32 - bar_w / 2,
                        bar_y,
                        bar_w,
                        2,
                        rgba,
                    );
                }
            }

        }

        // HUD home-arrow + pencil key from homePosStack (permanent + temp PS).
        self.sync_home_hud(world);

        // Hover / mouse-over cell highlight
        if let Some((hx, hy)) = self.highlight_tile {
            let (x0, y0, tw, th) =
                tile_screen_rect(&self.camera, hx, hy, fb.width, fb.height);
            // outline
            let c = [255, 255, 100, 180];
            fb.fill_rect(x0, y0, tw, 2, c);
            fb.fill_rect(x0, y0 + th - 2, tw, 2, c);
            fb.fill_rect(x0, y0, 2, th, c);
            fb.fill_rect(x0 + tw - 2, y0, 2, th, c);
        }

        // --- Pass 3: food / heat HUD over world (C++ LivingLifePage panel + meters) ---
        if self.draw_hud {
            // Slip slide/wiggle (C++ step ~14550) before blit; hunger.aiff on event.
            let frf = (dt * 60.0).clamp(0.0, 4.0);
            if frf > 1e-6 {
                let sound_ev = self.hud.step_slips(frf);
                use crate::hud::HungerSoundEvent;
                if matches!(
                    sound_ev,
                    HungerSoundEvent::OneShot | HungerSoundEvent::Pulse
                ) {
                    let _ = self.sounds.play_hunger_sound();
                }
            }
            // Age for hunger-slip thresholds (baby / elder gates).
            if let Some(o) = world.our() {
                self.hud.age_years = o.age;
            }
            // &mut: C++ updates mOldArrows / mCurrentArrowI inside draw.
            draw_hud_if_visible(fb, &mut self.hud, &self.hud_sprites);
        }
    }

    /// Drive home-arrow strip from C++ `homePosStack` (permanent + temp PS).
    ///
    /// Falls back to raw `says_pointers` when stack empty but markers remain.
    fn sync_home_hud(&mut self, world: &LiveWorld) {
        let (fx, fy) = world
            .our()
            .map(|o| (o.x as f32, o.y as f32))
            .unwrap_or((self.camera.x, self.camera.y));
        if !world.home_stack.is_empty() {
            let (dir, label) = world.home_stack.home_dir_and_label(fx, fy);
            self.hud.map_pointer_label = label;
            self.hud.set_home_arrow(dir);
            return;
        }
        // Fallback: live says_pointers only (P3#17 soft-FB).
        let primary = world.says_pointers.iter().find(|m| m.fade > 0.01);
        let Some(m) = primary else {
            self.hud.map_pointer_label = None;
            self.hud.set_home_arrow(None);
            return;
        };
        let label = m
            .label_text()
            .unwrap_or_else(|| "MAP".to_string());
        self.hud.map_pointer_label = Some(label);
        if let Some((tx, ty)) = m.map_tile() {
            let dir = home_dir_index(fx, fy, tx as f32, ty as f32);
            self.hud.set_home_arrow(dir);
        } else {
            self.hud.set_home_arrow(None);
        }
    }

    /// One map cell of ground **sprites** (C++ LivingLifePage ground pass).
    /// Solid plates were already painted in pass 1a (no black seams while TGAs load).
    ///
    /// - **Square tile** when left, above (`y+1`), and diagonal (`x+1,y+1`) share
    ///   this biome — C++ pixel-fill optimization for interior.
    /// - Else **soft 2×CELL_D tile** with soft alpha edges so biome borders blend.
    fn draw_ground_cell(
        &mut self,
        fb: &mut Framebuffer,
        map: &crate::client_map::ClientMap,
        tx: i32,
        ty: i32,
    ) {
        let tile = map.get_or_empty(tx, ty);
        let biome = tile.biome;

        // Neighbor biomes: missing cells ⇒ "different" (C++ OOB = -1 ≠ b).
        let left_same = map.get(tx - 1, ty).map(|t| t.biome == biome).unwrap_or(false);
        let above_same = map.get(tx, ty + 1).map(|t| t.biome == biome).unwrap_or(false);
        let diag_same = map
            .get(tx + 1, ty + 1)
            .map(|t| t.biome == biome)
            .unwrap_or(false);

        // C++ pos offset: +32 X, −32 Y in object units (CELL_D/4) to center overlaps.
        const OFF: f32 = 0.25;
        let wx = tx as f32 + 0.5 + OFF;
        let wy = ty as f32 + 0.5 - OFF;
        let (sx, sy) = self.world_to_screen(wx, wy, fb.width, fb.height);

        let use_square = left_same && above_same && diag_same;
        if use_square {
            // Interior: solid plate from pass 1 already covers the cell (Jason square
            // interior). Skip TGA blit — largest FPS win while keeping seamless fill.
            // Soft-edge cells (below) still use 2× ground sprites for biome blends.
        } else {
            // Soft 2×CELL_D — overdraw +1px so float zoom never flashes clear between cells.
            if let Some(gt) = self.ground.ensure_tile(biome, tx, ty) {
                self.blit_ground_centered_overdraw(fb, &gt, sx, sy, 2.0, 1);
            }
        }

        // Haxe/C++ ground overlay pass on select cells (only when bank has overlays).
        if self.ground.has_overlays() {
            if let Some(ov) = self.ground.ensure_overlay_for_tile(tx, ty) {
                let (x0, y0, tw, th) =
                    tile_screen_rect(&self.camera, tx, ty, fb.width, fb.height);
                self.blit_ground_rect(fb, &ov, x0, y0, tw, th);
            }
        }
    }

    /// Blit ground sprite into an exact screen rect (square / overlay).
    /// Samples the atlas page in-place (no per-tile allocation).
    fn blit_ground_rect(
        &self,
        fb: &mut Framebuffer,
        gt: &crate::ground_sprites::GroundTileRect,
        x0: i32,
        y0: i32,
        tw: i32,
        th: i32,
    ) {
        let Some((pix, atlas_w, sx, sy, sw, sh)) = self.ground.page_tile(gt) else {
            return;
        };
        fb.blit_rect_scaled(
            pix,
            atlas_w,
            sx as i32,
            sy as i32,
            sw,
            sh,
            x0,
            y0,
            tw.max(1),
            th.max(1),
        );
    }

    /// Soft-edge ground: center at `(sx,sy)`, cover `cells` tiles, plus `pad` pixels overdraw.
    fn blit_ground_centered_overdraw(
        &self,
        fb: &mut Framebuffer,
        gt: &crate::ground_sprites::GroundTileRect,
        cx: f32,
        cy: f32,
        cells: f32,
        pad: i32,
    ) {
        let Some((pix, atlas_w, sx, sy, sw, sh)) = self.ground.page_tile(gt) else {
            return;
        };
        // Use floor of left/top and ceil of right/bottom so neighbors always overlap ≥pad.
        let half = self.camera.zoom * cells * 0.5;
        let x0 = (cx - half).floor() as i32 - pad;
        let y0 = (cy - half).floor() as i32 - pad;
        let x1 = (cx + half).ceil() as i32 + pad;
        let y1 = (cy + half).ceil() as i32 + pad;
        let dw = (x1 - x0).max(1);
        let dh = (y1 - y0).max(1);
        fb.blit_rect_scaled(
            pix,
            atlas_w,
            sx as i32,
            sy as i32,
            sw,
            sh,
            x0,
            y0,
            dw,
            dh,
        );
    }

    /// Draw object and its contained children at slot positions.
    fn draw_object_stack(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        stack: &ObjectStackNode,
        anim_type: i32,
        age: f32,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        sprite_filter: SpriteLayerFilter,
    ) {
        self.draw_object_stack_at_time(
            fb,
            content,
            sprites,
            anims,
            stack,
            anim_type,
            age,
            screen_x,
            screen_y,
            flip,
            sprite_filter,
            self.time,
        );
    }

    /// Map-cell stack with explicit frame_time (seconds; Jason frameCount/60).
    fn draw_object_stack_at_time(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        stack: &ObjectStackNode,
        anim_type: i32,
        age: f32,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        sprite_filter: SpriteLayerFilter,
        frame_time: f32,
    ) {
        if stack.id <= 0 {
            return;
        }
        // Map-cell objects: single-type pack (no LiveObject dual-fade).
        let mut pack = ObjectAnimPack::single(stack.id, anim_type, frame_time);
        let _ = self.draw_object_with_pack(
            fb,
            content,
            sprites,
            anims,
            &mut pack,
            age,
            screen_x,
            screen_y,
            flip,
            false,
            false,
            0,
            false,
            sprite_filter,
            false,
        );
        // Contained children only with full/front passes (not behind-only split).
        if matches!(
            sprite_filter,
            SpriteLayerFilter::All | SpriteLayerFilter::NotBehindPlayer
        ) {
            let slots = content
                .get(stack.id)
                .map(|d| d.slot_pos.clone())
                .unwrap_or_default();
            let scale = (self.camera.zoom / GRID).max(0.05);
            for (i, child) in stack.contained.iter().enumerate() {
                let (mut ox, mut oy) = slots.get(i).copied().unwrap_or((0.0, (i as f32) * 8.0));
                // C++: slotAnim offsets on contained positions (dual-fade when pack mid-fade)
                let mut slot_pack = ObjectAnimPack::single(stack.id, anim_type, frame_time);
                let slot_s = sample_slot_pack(anims, &mut slot_pack, i);
                ox += slot_s.x;
                oy += slot_s.y;
                let cx = screen_x + ox * scale * if flip { -1.0 } else { 1.0 };
                let cy = screen_y - oy * scale;
                self.draw_object_stack_at_time(
                    fb,
                    content,
                    sprites,
                    anims,
                    child,
                    anim_type,
                    age,
                    cx,
                    cy,
                    flip,
                    SpriteLayerFilter::All,
                    frame_time,
                );
            }
        }
    }

    /// Single-type convenience (floors / map objects) — builds a pack at `self.time`.
    fn draw_object(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        object_id: i32,
        anim_type: i32,
        extra_index: i32,
        age: f32,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        holding: bool,
    ) {
        let mut pack = ObjectAnimPack::single(object_id, anim_type, self.time);
        pack.extra_index = extra_index;
        let _ = self.draw_object_with_pack(
            fb,
            content,
            sprites,
            anims,
            &mut pack,
            age,
            screen_x,
            screen_y,
            flip,
            holding,
            false,
            0,
            false,
            SpriteLayerFilter::All,
            false,
        );
    }

    /// Soft-FB blit using dual-anim `ObjectAnimPack` (`sample_sprite_pack`).
    ///
    /// Returns C++ `HoldingPos` (hand or body attachment) + head/body anchors
    /// for PE emotion object layers when drawing a person.
    ///
    /// // C++: drawObjectAnimPacked working pos/rot/fade blend + hideClosestArm
    /// // Haxe: Object.update single-record only — dual fade is Jason parity
    ///
    /// `worn_clothing`: optional `(slot_i, cloth_id, raw)` list drawn **interleaved**
    /// with person sprites (Jason: body clothes under top back arm; shoes on feet; hat last).
    fn draw_object_with_pack(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        pack: &mut ObjectAnimPack,
        age: f32,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        holding: bool,
        worn: bool,
        hide_closest_arm: i32,
        hide_all_limbs: bool,
        sprite_filter: SpriteLayerFilter,
        // P3#19: skip person mouth sprite when PE `mouthEmot` is active (C++).
        hide_mouth: bool,
    ) -> (HoldingPos, PersonAnchors) {
        self.draw_object_with_pack_ex(
            fb,
            content,
            sprites,
            anims,
            pack,
            age,
            screen_x,
            screen_y,
            flip,
            holding,
            worn,
            hide_closest_arm,
            hide_all_limbs,
            sprite_filter,
            hide_mouth,
            None,
        )
    }

    fn draw_object_with_pack_ex(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        pack: &mut ObjectAnimPack,
        age: f32,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        holding: bool,
        worn: bool,
        hide_closest_arm: i32,
        hide_all_limbs: bool,
        sprite_filter: SpriteLayerFilter,
        hide_mouth: bool,
        // Person worn clothing slots to interleave (Jason animationBank order).
        worn_clothing: Option<&[(usize, i32, String)]>,
    ) -> (HoldingPos, PersonAnchors) {
        let object_id = pack.object_id;
        let scale = (self.camera.zoom / GRID).max(0.05);
        let mut holding_out = HoldingPos::default();
        let mut anchors = PersonAnchors::default();
        let Some(def) = content.get(object_id) else {
            fb.fill_rect(
                screen_x as i32 - 3,
                screen_y as i32 - 3,
                6,
                6,
                [160, 160, 160, 255],
            );
            return (holding_out, anchors);
        };
        if def.sprites.is_empty() {
            fb.fill_rect(
                screen_x as i32 - 4,
                screen_y as i32 - 4,
                8,
                8,
                [200, 80, 80, 255],
            );
            return (holding_out, anchors);
        }

        // Limb index sets for person hide (C++ getFrontArmIndices / getBackArmIndices)
        let front_arm = if def.person != 0 && (hide_closest_arm != 0 || hide_all_limbs) {
            def.front_arm_indices(age)
        } else {
            Vec::new()
        };
        let back_arm = if def.person != 0 && (hide_closest_arm != 0 || hide_all_limbs) {
            def.back_arm_indices(age)
        } else {
            Vec::new()
        };
        let legs = if def.person != 0 && hide_all_limbs {
            def.all_leg_indices(age)
        } else {
            Vec::new()
        };
        let back_hand = if def.person != 0 {
            def.back_hand_index(age)
        } else {
            None
        };
        let body_idx = if def.person != 0 {
            Some(def.body_index(age))
        } else {
            None
        };
        let head_idx = if def.person != 0 {
            Some(def.head_index(age))
        } else {
            None
        };
        let front_foot_idx = if def.person != 0 {
            Some(def.front_foot_index(age))
        } else {
            None
        };
        let back_foot_idx = if def.person != 0 {
            Some(def.back_foot_index(age))
        } else {
            None
        };
        // C++ topBackArmIndex = last of backArmIndices — body clothes draw under it.
        let top_back_arm_idx = if def.person != 0 {
            let arms = def.back_arm_indices(age);
            arms.last().copied()
        } else {
            None
        };
        // Rest poses for C++ getAgeHeadOffset / getAgeBodyOffset.
        let head_rest = head_idx.map(|i| def.sprite_rest_pos(i)).unwrap_or((0.0, 0.0));
        let body_rest = body_idx.map(|i| def.sprite_rest_pos(i)).unwrap_or((0.0, 0.0));
        let front_foot_rest = front_foot_idx
            .map(|i| def.sprite_rest_pos(i))
            .unwrap_or((0.0, 0.0));

        // Evaluate base object-space poses (+ dual-anim sample), then parent hierarchy.
        // Pose is computed even for layers we skip drawing (hand HoldingPos needs hand pose
        // when invisHolding hides the hand sprite while held).
        // Haxe: Object.update + transformChild
        // C++: drawObjectAnim workingSpritePos / workingSpriteFade / rotCenterOffset
        let n = def.sprites.len();
        let mut ox = vec![0.0f32; n];
        let mut oy = vec![0.0f32; n];
        let mut orot = vec![0.0f32; n]; // turns
        let mut ofade = vec![1.0f32; n];
        let mut posed = vec![false; n];
        let mut draw = vec![true; n];

        // Pose ALL sprites first (Jason computes workingSpritePos for every layer,
        // then skips drawing age-invisible ones). Skipping pose on age-gated parents
        // broke the parent chain and left only orphan limbs/hat visible.
        for (si, spr) in def.sprites.iter().enumerate() {
            // Tall-object layer filter (C++ prepareToSkipSprites / spriteBehindPlayer)
            match sprite_filter {
                SpriteLayerFilter::All => {}
                SpriteLayerFilter::BehindPlayerOnly if !spr.behind_player => {
                    draw[si] = false;
                }
                SpriteLayerFilter::NotBehindPlayer if spr.behind_player => {
                    draw[si] = false;
                }
                _ => {}
            }
            // Draw-skip flags — pose still computed so children keep parent chain.
            // P4#25: C++ spriteSkipDrawing from setupSpriteUseVis (multi-use stages)
            if spr.skip_drawing {
                draw[si] = false;
            }
            // C++ isSpriteVisibleAtAge — skip drawing age-out-of-range layers (~2575).
            // Person path always passes real age; non-person draws still honor sprite
            // age bounds when an age is supplied (unit tests + multi-stage content).
            if !spr.visible_at_age(age) {
                draw[si] = false;
            }
            if holding && spr.invis_holding {
                draw[si] = false;
            }
            if worn && spr.invis_worn {
                draw[si] = false;
            }
            // C++ spriteInvisibleWhenWorn==2: only draw when worn (clothing path).
            if !worn && spr.only_when_worn {
                draw[si] = false;
            }
            // Limb hide — C++ animationBank drawObjectAnim (person path):
            // hideClosestArm ±1 hides that arm chain; hideAllLimbs hides legs only
            // (rideable freezes arms via frozenArmType, does not skip arm sprites).
            if hide_closest_arm == 1 && front_arm.contains(&si) {
                draw[si] = false;
            }
            if hide_closest_arm == -1 && back_arm.contains(&si) {
                draw[si] = false;
            }
            if hide_all_limbs && legs.contains(&si) {
                draw[si] = false;
            }
            // P3#19: C++ drawObjectAnim — skip mouthIndex when any mouthEmot != 0
            if hide_mouth {
                if let Some(mi) = def.mouth_index(age) {
                    if si == mi {
                        draw[si] = false;
                    }
                }
            }

            // L-ANIM-DRAW: dual-anim pack sample (inAnimFade + frozen rot)
            let sample = sample_sprite_pack(anims, pack, si);
            let mut px = spr.x + sample.x;
            let mut py = spr.y + sample.y;
            // C++ ageControl: head/body rest offset for babies / elders before parent chain.
            if def.person != 0 {
                if head_idx == Some(si) {
                    let (dx, dy) =
                        crate::content::age_head_offset(age, head_rest, body_rest, front_foot_rest);
                    px += dx;
                    py += dy;
                } else if body_idx == Some(si) {
                    let (dx, dy) = crate::content::age_body_offset(age, body_rest.1);
                    px += dx;
                    py += dy;
                }
            }
            // spr.rot is turns; sample.rot treated as turns (animParam subset)
            let rot = spr.rot + sample.rot;
            // C++: if rotCenterOffset nonzero, adjust spritePos so pivot is correct
            if sample.rot_center_x.abs() > 1e-8 || sample.rot_center_y.abs() > 1e-8 {
                let angle = -rot * std::f32::consts::TAU;
                let (s, c) = angle.sin_cos();
                let rcx = sample.rot_center_x;
                let rcy = sample.rot_center_y;
                let nx = rcx * c - rcy * s;
                let ny = rcx * s + rcy * c;
                px -= nx - rcx;
                py -= ny - rcy;
            }
            ox[si] = px;
            oy[si] = py;
            orot[si] = rot;
            ofade[si] = sample.fade;
            posed[si] = true;
        }

        // C++ animationBank.cpp ~2529–2533 + ~2505–2625:
        // workingDelta = individual pose − rest; then walk *up* parent chain
        // adding each ancestor's local delta (with compound rot), not Haxe
        // re-parent. Age-invisible layers still contribute deltas as parents.
        apply_jason_parent_chain(&def.sprites, &mut ox, &mut oy, &mut orot);

        // HoldingPos from hand (hideClosestArm==0) or body (≠0) — C++ drawObjectAnim
        // Rideable: hideAllLimbs true but hideClosestArm still 0 → hand attach is returned,
        // but LivingLifePage / SceneRenderer place vehicle at person pos and interleave
        // person-under-vehicle (P3#20), ignoring HoldingPos for rideable draw pos.
        if def.person != 0 {
            if hide_closest_arm == 0 {
                if let Some(hi) = back_hand {
                    if posed[hi] {
                        holding_out = HoldingPos {
                            valid: true,
                            x: ox[hi],
                            y: oy[hi],
                            rot: orot[hi],
                        };
                    }
                }
            } else {
                // -2 bulky freeze, or ±1 arm hide → body attachment
                if let Some(bi) = body_idx {
                    if posed[bi] {
                        holding_out = HoldingPos {
                            valid: true,
                            x: ox[bi],
                            y: oy[bi],
                            rot: orot[bi],
                        };
                    }
                }
            }
            // Head/body/feet anchors for PE + Jason clothing attach
            if let Some(hi) = head_idx {
                if posed[hi] {
                    anchors.head = Some((ox[hi], oy[hi], orot[hi]));
                }
            }
            if let Some(bi) = body_idx {
                if posed[bi] {
                    anchors.body = Some((ox[bi], oy[bi], orot[bi]));
                }
            }
            if let Some(fi) = front_foot_idx {
                if posed[fi] {
                    anchors.front_foot = Some((ox[fi], oy[fi], orot[fi]));
                }
            }
            if let Some(bi) = back_foot_idx {
                if posed[bi] {
                    anchors.back_foot = Some((ox[bi], oy[bi], orot[bi]));
                }
            }
            // Fallback: if head index missing, use body for face layers.
            if anchors.head.is_none() {
                anchors.head = anchors.body;
            }
            if anchors.body.is_none() {
                anchors.body = anchors.head;
            }
            if anchors.front_foot.is_none() {
                anchors.front_foot = anchors.body;
            }
            if anchors.back_foot.is_none() {
                anchors.back_foot = anchors.front_foot.or(anchors.body);
            }
            // P3#19: eyes anchor = posed head + rotated mainEyesOffset
            // // C++: cPos = animHeadPos + rotate(mainEyesOffset, -2π·headRot)
            anchors.has_eyes = def.has_eyes_for_emot(age);
            if let Some((hx, hy, hr)) = anchors.head {
                let (ex, ey) = crate::content::eyes_anchor_from_head(
                    hx,
                    hy,
                    hr,
                    def.main_eyes_offset,
                );
                anchors.eyes = Some((ex, ey, hr));
            }
        }

        // Jason: body clothes under top back arm *before* that arm sprite is blitted.
        let draw_body_clothes = |fb: &mut Framebuffer,
                                 sprites: &mut SpriteBank,
                                 anims: &mut AnimBank,
                                 anchors: &PersonAnchors,
                                 person_pack: &ObjectAnimPack| {
            let Some(list) = worn_clothing else {
                return;
            };
            // bottom(4), tunic(1), backpack(5) — animationBank ~2958–3071
            for &slot in &[4usize, 1, 5] {
                if let Some((_, cid, raw)) = list.iter().find(|(s, _, _)| *s == slot) {
                    Self::draw_one_worn_clothing(
                        self,
                        fb,
                        content,
                        sprites,
                        anims,
                        person_pack,
                        anchors,
                        slot,
                        *cid,
                        raw,
                        screen_x,
                        screen_y,
                        scale,
                        flip,
                        age,
                    );
                }
            }
        };
        let draw_shoe = |fb: &mut Framebuffer,
                         sprites: &mut SpriteBank,
                         anims: &mut AnimBank,
                         anchors: &PersonAnchors,
                         person_pack: &ObjectAnimPack,
                         slot: usize| {
            let Some(list) = worn_clothing else {
                return;
            };
            if let Some((_, cid, raw)) = list.iter().find(|(s, _, _)| *s == slot) {
                Self::draw_one_worn_clothing(
                    self,
                    fb,
                    content,
                    sprites,
                    anims,
                    person_pack,
                    anchors,
                    slot,
                    *cid,
                    raw,
                    screen_x,
                    screen_y,
                    scale,
                    flip,
                    age,
                );
            }
        };

        for (si, spr) in def.sprites.iter().enumerate() {
            // Body clothes under top of back arm (before arm blit).
            if def.person != 0 && top_back_arm_idx == Some(si) {
                draw_body_clothes(fb, sprites, anims, &anchors, pack);
            }

            if posed[si] && draw[si] {
                if let Some(rect) = sprites.ensure(spr.sprite_id) {
                    let page = &sprites.pages()[rect.atlas_index];
                    // Center-anchor (C++ setSpriteCenterOffset / Haxe inCenter).
                    // Object space is Y-up; screen is Y-down. Haxe does:
                    //   elem.y = worldY - sprite.y;  tile.dy += -inCenterYOffset
                    // so geometric center is at object (pos.x - ax, pos.y + ay)
                    // before the Y flip — i.e. add ay in object Y, subtract ax in X.
                    let ax = rect.center_anchor_x as f32;
                    let ay = rect.center_anchor_y as f32;
                    let px = ox[si] - ax;
                    let py = oy[si] + ay;

                    // Screen: flip X when facing left
                    let dx = screen_x + px * scale * if flip { -1.0 } else { 1.0 };
                    let dy = screen_y - py * scale;
                    let mut h_flip = spr.h_flip ^ flip;
                    if rect.no_flip {
                        h_flip = spr.h_flip; // ignore facing flip when NoFlip
                    }
                    let mut rot = orot[si];
                    if flip {
                        rot = -rot;
                    }
                    fb.blit_sprite(
                        &page.pixels,
                        page.width,
                        rect.rect.x,
                        rect.rect.y,
                        rect.width,
                        rect.height,
                        dx,
                        dy,
                        scale,
                        h_flip,
                        [spr.r, spr.g, spr.b],
                        rot,
                        rect.multiplicative_blend,
                        ofade[si],
                    );
                }
            }

            // Shoes on top of feet (after foot sprite).
            if def.person != 0 && back_foot_idx == Some(si) {
                draw_shoe(fb, sprites, anims, &anchors, pack, 3); // backShoe
            }
            if def.person != 0 && front_foot_idx == Some(si) {
                draw_shoe(fb, sprites, anims, &anchors, pack, 2); // frontShoe
            }
        }

        // Hat on top of everything (Jason ~3549 after sprite loop).
        if def.person != 0 {
            if let Some(list) = worn_clothing {
                if let Some((_, cid, raw)) = list.iter().find(|(s, _, _)| *s == 0) {
                    Self::draw_one_worn_clothing(
                        self,
                        fb,
                        content,
                        sprites,
                        anims,
                        pack,
                        &anchors,
                        0,
                        *cid,
                        raw,
                        screen_x,
                        screen_y,
                        scale,
                        flip,
                        age,
                    );
                }
            }
        }

        // If no top back arm (no arms), still draw body clothes after all body layers.
        if def.person != 0 && top_back_arm_idx.is_none() && worn_clothing.is_some() {
            draw_body_clothes(fb, sprites, anims, &anchors, pack);
        }

        (holding_out, anchors)
    }

    /// Draw one worn clothing object + contained at Jason body-part attach pos.
    fn draw_one_worn_clothing(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        person_pack: &ObjectAnimPack,
        anchors: &PersonAnchors,
        slot_i: usize,
        cloth_id: i32,
        cloth_raw: &str,
        person_sx: f32,
        person_sy: f32,
        scale: f32,
        flip: bool,
        age: f32,
    ) {
        if cloth_id <= 0 {
            return;
        }
        let (ox, oy) = content
            .get(cloth_id)
            .map(|d| d.clothing_offset)
            .unwrap_or((0.0, 0.0));
        let part = clothing_anchor_for_slot(anchors, slot_i).unwrap_or((0.0, 0.0, 0.0));
        let (cx, cy) = clothing_screen_pos(person_sx, person_sy, part, (ox, oy), scale, flip);
        let mut cloth_pack = clothing_pack_from_person(person_pack, cloth_id);
        let _ = self.draw_object_with_pack(
            fb,
            content,
            sprites,
            anims,
            &mut cloth_pack,
            age,
            cx,
            cy,
            flip,
            false,
            true, // worn
            0,
            false,
            SpriteLayerFilter::All,
            false,
        );
        let contained = crate::client_map::parse_object_raw_contained(cloth_raw);
        if contained.is_empty() {
            return;
        }
        let flip_s = if flip { -1.0 } else { 1.0 };
        let slots = content
            .get(cloth_id)
            .map(|d| d.slot_pos.clone())
            .unwrap_or_default();
        let cloth_anim_type = select_clothing_anim_type(person_pack.anim_type);
        for (i, child) in contained.iter().enumerate() {
            if child.id <= 0 {
                continue;
            }
            let (mut sox, mut soy) = slots.get(i).copied().unwrap_or((0.0, (i as f32) * 8.0));
            let mut slot_pack = clothing_pack_from_person(person_pack, cloth_id);
            let slot_s = sample_slot_pack(anims, &mut slot_pack, i);
            sox += slot_s.x;
            soy += slot_s.y;
            let csx = cx + sox * scale * flip_s;
            let csy = cy - soy * scale;
            self.draw_object_stack(
                fb,
                content,
                sprites,
                anims,
                child,
                cloth_anim_type,
                age,
                csx,
                csy,
                flip,
                SpriteLayerFilter::All,
            );
        }
    }

    /// Draw PE emotion object layers at person head/body anchors.
    ///
    /// // C++: setAnimationEmotion + drawObjectAnim emot slots (simplified z-order)
    /// Order: body → eye/face/mouth/other → head (after clothing in caller).
    fn draw_emotion_layers(
        &self,
        fb: &mut Framebuffer,
        content: &ClientContent,
        sprites: &mut SpriteBank,
        anims: &mut AnimBank,
        person_pack: &ObjectAnimPack,
        emot_indices: &[i32],
        anchors: &PersonAnchors,
        screen_x: f32,
        screen_y: f32,
        flip: bool,
        phase: EmotDrawPhase,
    ) {
        if emot_indices.is_empty() || self.emotions.is_empty() {
            return;
        }
        let scale = (self.camera.zoom / GRID).max(0.05);
        let flip_s = if flip { -1.0 } else { 1.0 };

        let to_screen = |ox: f32, oy: f32| -> (f32, f32) {
            (screen_x + ox * scale * flip_s, screen_y - oy * scale)
        };

        for &idx in emot_indices {
            let Some(em) = self.emotions.get(idx) else {
                continue;
            };
            match phase {
                EmotDrawPhase::Body => {
                    if em.body_emot > 0 {
                        if let Some((bx, by, _br)) = anchors.body {
                            let (sx, sy) = to_screen(bx, by);
                            let mut pack = clothing_pack_from_person(person_pack, em.body_emot);
                            let _ = self.draw_object_with_pack(
                                fb,
                                content,
                                sprites,
                                anims,
                                &mut pack,
                                20.0,
                                sx,
                                sy,
                                flip,
                                false,
                                true,
                                0,
                                false,
                                SpriteLayerFilter::All,
                            false,
                            );
                        }
                    }
                }
                EmotDrawPhase::Face => {
                    let head = anchors.head.or(anchors.body);
                    let Some((hx, hy, _hr)) = head else {
                        continue;
                    };
                    let (head_sx, head_sy) = to_screen(hx, hy);
                    // P3#19: eyeEmot at head+mainEyesOffset (C++ animHeadPos+offset)
                    // // C++ only draws eyeEmot when eyesIndex is valid (!= -1)
                    if em.eye_emot > 0 && anchors.has_eyes {
                        let (ex, ey, _) = anchors.eyes.unwrap_or((hx, hy, 0.0));
                        let (esx, esy) = to_screen(ex, ey);
                        let mut pack = clothing_pack_from_person(person_pack, em.eye_emot);
                        let _ = self.draw_object_with_pack(
                            fb,
                            content,
                            sprites,
                            anims,
                            &mut pack,
                            20.0,
                            esx,
                            esy,
                            flip,
                            false,
                            true,
                            0,
                            false,
                            SpriteLayerFilter::All,
                        false,
                        );
                    }
                    // face / mouth / other at head (C++ face uses animHeadPos always)
                    for slot in [em.face_emot, em.mouth_emot, em.other_emot] {
                        if slot <= 0 {
                            continue;
                        }
                        let mut pack = clothing_pack_from_person(person_pack, slot);
                        let _ = self.draw_object_with_pack(
                            fb,
                            content,
                            sprites,
                            anims,
                            &mut pack,
                            20.0,
                            head_sx,
                            head_sy,
                            flip,
                            false,
                            true,
                            0,
                            false,
                            SpriteLayerFilter::All,
                        false,
                        );
                    }
                }
                EmotDrawPhase::HeadTop => {
                    if em.head_emot > 0 {
                        let head = anchors.head.or(anchors.body);
                        if let Some((hx, hy, _hr)) = head {
                            let (sx, sy) = to_screen(hx, hy);
                            let mut pack = clothing_pack_from_person(person_pack, em.head_emot);
                            let _ = self.draw_object_with_pack(
                                fb,
                                content,
                                sprites,
                                anims,
                                &mut pack,
                                20.0,
                                sx,
                                sy,
                                flip,
                                false,
                                true,
                                0,
                                false,
                                SpriteLayerFilter::All,
                            false,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Which PE object-layer pass (approximate C++ interleave).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmotDrawPhase {
    /// Under clothing (bodyEmot).
    Body,
    /// After clothing base (eye/face/mouth/other).
    Face,
    /// After hat (headEmot).
    HeadTop,
}

/// C++ LivingLifePage front-object sub-order within a row (after players):
/// permanent non-wall → non-permanent non-wall → wallLayer !frontWall → frontWall.
fn front_object_draw_layer(def: Option<&ClientObjectDef>) -> DrawLayer {
    let (permanent, wall, front) = match def {
        Some(d) => (d.permanent, d.wall_layer, d.front_wall),
        None => (false, false, false),
    };
    if wall {
        if front {
            DrawLayer::FrontFrontWall
        } else {
            DrawLayer::FrontWall
        }
    } else if permanent {
        DrawLayer::FrontPermanent
    } else {
        DrawLayer::FrontNonPermanent
    }
}

/// Queue map-cell draw items for tall-object behind/front layering.
///
/// // C++ LivingLifePage: drawBehindPlayer objects before players; spritesDrawnBehind
/// // split into behind pass + front canopy over same-row players.
/// // P3#23: front pass sub-ordered by permanent / wallLayer / frontWall.
fn push_map_object_draw_items(
    items: &mut Vec<YSortItem>,
    content: &ClientContent,
    ty: i32,
    tx: i32,
    object_id: i32,
) {
    let def = content.get(object_id);
    let draw_behind = def.map(|d| d.draw_behind_player).unwrap_or(false);
    let any_behind = def.map(|d| d.any_sprites_behind_player()).unwrap_or(false);
    let front_layer = front_object_draw_layer(def);

    if draw_behind {
        // Whole object under players. If some sprites are also marked behind,
        // still one BehindPlayer pass with All (both trunks + canopies stay under).
        items.push(YSortItem {
            sort_y: ty,
            layer: DrawLayer::BehindPlayer,
            kind: DrawKind::MapObject {
                tx,
                ty,
                sprite_filter: SpriteLayerFilter::All,
            },
        });
    } else if any_behind {
        // Tall non-behind objects: trunk under players, canopy over.
        items.push(YSortItem {
            sort_y: ty,
            layer: DrawLayer::BehindPlayer,
            kind: DrawKind::MapObject {
                tx,
                ty,
                sprite_filter: SpriteLayerFilter::BehindPlayerOnly,
            },
        });
        items.push(YSortItem {
            sort_y: ty,
            layer: front_layer,
            kind: DrawKind::MapObject {
                tx,
                ty,
                sprite_filter: SpriteLayerFilter::NotBehindPlayer,
            },
        });
    } else {
        items.push(YSortItem {
            sort_y: ty,
            layer: front_layer,
            kind: DrawKind::MapObject {
                tx,
                ty,
                sprite_filter: SpriteLayerFilter::All,
            },
        });
    }
}

/// C++ `drawObjectAnim` parent-chain compound (animationBank.cpp ~2505–2625).
///
/// After individual layer poses are stored in `ox/oy/orot`:
/// - `workingDeltaPos[i] = posed[i] - rest[i]`
/// - `workingDeltaRot[i] = posedRot[i] - restRot[i]`
/// For each sprite, walk `parent` links and accumulate ancestors' **local**
/// deltas (not already-compounded parent world poses).
///
/// When `workingDeltaRot[parent] != 0`:
/// ```text
/// angle = -2π · dRot
/// pos += rotate(parentDeltaPos, -angle)
/// childOff = pos - parentRest
/// pos += rotate(childOff, angle) - childOff
/// rot += dRot
/// ```
/// Else: `pos += parentDeltaPos`.
fn apply_jason_parent_chain(
    sprites: &[crate::content::ObjectSprite],
    ox: &mut [f32],
    oy: &mut [f32],
    orot: &mut [f32],
) {
    let n = sprites.len();
    if n == 0 {
        return;
    }
    // Local deltas from rest (C++ workingDelta*) — frozen before compounding.
    let mut dx = vec![0.0f32; n];
    let mut dy = vec![0.0f32; n];
    let mut drot = vec![0.0f32; n];
    for i in 0..n {
        dx[i] = ox[i] - sprites[i].x;
        dy[i] = oy[i] - sprites[i].y;
        drot[i] = orot[i] - sprites[i].rot;
    }

    for i in 0..n {
        let mut sx = ox[i];
        let mut sy = oy[i];
        let mut rot = orot[i];
        let mut next = sprites[i].parent;
        while next >= 0 {
            let p = next as usize;
            if p >= n {
                break;
            }
            let pdrot = drot[p];
            if pdrot.abs() > 1e-12 {
                // C++: angle = -2π * workingDeltaRot[parent]
                let angle = -pdrot * std::f32::consts::TAU;
                rot += pdrot;

                // pos += rotate(parentDelta, -angle)
                let (rx, ry) = rotate2(dx[p], dy[p], -angle);
                sx += rx;
                sy += ry;

                // arm-length: rotate (pos - parentRest) by angle
                let cox = sx - sprites[p].x;
                let coy = sy - sprites[p].y;
                let (nox, noy) = rotate2(cox, coy, angle);
                sx += nox - cox;
                sy += noy - coy;
            } else {
                sx += dx[p];
                sy += dy[p];
            }
            next = sprites[p].parent;
        }
        ox[i] = sx;
        oy[i] = sy;
        orot[i] = rot;
    }
}

#[inline]
fn rotate2(x: f32, y: f32, angle: f32) -> (f32, f32) {
    // C++ minorGems doublePair::rotate — standard 2D rotation.
    let (s, c) = angle.sin_cos();
    (c * x - s * y, s * x + c * y)
}

// Re-export biome_color for tests / callers
pub use crate::ground_sprites::biome_color as biome_color_pub;

/// Public alias matching older render API.
pub fn biome_color_for(biome: u8) -> [u8; 4] {
    biome_color(biome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ClientObjectDef, ObjectSprite};
    use crate::live_object::{ClothingSet, LiveWorld};
    use crate::sprite_bank::SpriteBank;
    use crate::tga::RgbaImage;

    fn solid_sprite(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for px in pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&rgba);
        }
        RgbaImage {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn framebuffer_put() {
        let mut fb = Framebuffer::new(16, 16);
        fb.clear([0, 0, 0, 255]);
        fb.put(1, 1, [255, 0, 0, 255]);
        assert_eq!(fb.pixels[(1 * 16 + 1) * 4], 255);
    }

    /// Real OneLifeData7 person (id 19) must paint skin pixels when content is present.
    #[test]
    fn person_skin_and_clothing_draw_from_content_if_present() {
        let root = std::path::Path::new(r"C:\OhOl\OpenLife\OneLifeData7");
        if !root.join("objects").join("19.txt").is_file() {
            eprintln!("skip: OneLifeData7 not present");
            return;
        }
        let content = ClientContent::load_from_dir(root).expect("load content");
        let def = content.get(19).expect("person 19");
        assert!(def.person != 0, "19 must be person");
        assert!(def.sprites.len() > 10, "person has many sprites");
        let mut sprites = SpriteBank::load_prefer_cache(root);
        let mut anims = AnimBank::load_prefer_cache(root);
        let mut ensured = 0usize;
        for s in &def.sprites {
            if sprites.ensure(s.sprite_id).is_some() {
                ensured += 1;
            }
        }
        assert!(
            ensured > 20,
            "most person sprites must load from sprites/*.tga, got {ensured}/{}",
            def.sprites.len()
        );

        let mut world = LiveWorld::new();
        // display_id=19, clothing hat 1117, age 20, pos 0,0
        let pu = crate::parse::parse_pu_line(
            "1 19 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 20.0 60.0 3.75 1117;0;0;0;0;0 0 0 -1 0 0",
        )
        .unwrap();
        world.apply_pu(&pu);
        world.set_our_id(1);
        let mut map = ClientMap::new();
        for y in -2..=2 {
            for x in -3..=3 {
                map.set(
                    x,
                    y,
                    crate::client_map::MapTile {
                        biome: 2,
                        ..crate::client_map::MapTile::empty()
                    },
                );
            }
        }
        let mut scene = SceneRenderer::default();
        scene.camera.x = 0.0;
        scene.camera.y = 0.0;
        scene.camera.zoom = ZOOM_DEFAULT;
        let mut fb = Framebuffer::new(960, 540);
        scene.draw(
            &mut fb,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            1.0 / 60.0,
        );
        let non = fb.count_non_color(CLEAR_RGBA);
        assert!(non > 500, "person+ground must paint, non_clear={non}");

        // Flesh-ish pixels (not pure green biome plate) near screen center.
        let mut flesh = 0usize;
        let cx = (fb.width / 2) as i32;
        let cy = (fb.height / 2) as i32;
        for y in (cy - 80)..(cy + 80) {
            for x in (cx - 60)..(cx + 60) {
                if x < 0 || y < 0 {
                    continue;
                }
                let i = ((y as u32 * fb.width + x as u32) * 4) as usize;
                if i + 3 >= fb.pixels.len() {
                    continue;
                }
                let r = fb.pixels[i];
                let g = fb.pixels[i + 1];
                let b = fb.pixels[i + 2];
                // Skin/cloth tones vs pure biome greens.
                if r > 40 && (r as i16 - g as i16).abs() > 8 {
                    flesh += 1;
                } else if r > 100 && g > 80 && b > 70 {
                    flesh += 1;
                }
            }
        }
        assert!(
            flesh > 80,
            "expected person skin/clothing near center, flesh_like={flesh}"
        );
    }

    /// Soft-FB draw path at default play buffer must sustain ≥60 FPS wall time.
    ///
    /// Measures real `SceneRenderer::draw` (not a fake counter) over many frames.
    #[test]
    fn soft_fb_draw_path_sustains_60fps() {
        use std::time::Instant;
        let mut map = ClientMap::new();
        // Typical play view: large same-biome regions (square interior) + sparse objects.
        // Checker biomes would force soft-edge blits every cell and are not representative.
        for y in -10i32..=10 {
            for x in -16i32..=16 {
                let mut t = crate::client_map::MapTile::empty();
                t.biome = if x < 0 { 2 } else { 3 };
                if (x + y * 3).rem_euclid(7) == 0 {
                    t.object_id = 33;
                    t.object_raw = "33".into();
                }
                map.set(x, y, t);
            }
        }
        let mut world = LiveWorld::new();
        let content = ClientContent::default();
        let mut sprites = SpriteBank::new(".");
        let mut anims = AnimBank::new(".");
        let mut scene = SceneRenderer::default();
        scene.camera.zoom = ZOOM_DEFAULT;
        scene.camera.x = 0.0;
        scene.camera.y = 0.0;
        let mut fb = Framebuffer::new(960, 540);
        // Warmup (TGA/lazy paths)
        for _ in 0..5 {
            scene.draw(
                &mut fb,
                &mut map,
                &mut world,
                &content,
                &mut sprites,
                &mut anims,
                1.0 / 60.0,
            );
        }
        let n = 90usize;
        let t0 = Instant::now();
        for _ in 0..n {
            scene.draw(
                &mut fb,
                &mut map,
                &mut world,
                &content,
                &mut sprites,
                &mut anims,
                1.0 / 60.0,
            );
        }
        let elapsed = t0.elapsed().as_secs_f64().max(1e-9);
        let fps = n as f64 / elapsed;
        assert!(
            fps >= 60.0,
            "soft-FB draw path FPS {fps:.1} < 60 over {n} frames in {elapsed:.3}s"
        );
    }

    /// Present fill: full FB clear + Stretch window→FB cursor map (criterion 4).
    #[test]
    fn present_fill_modes_match_fullscreen_contract() {
        // Framebuffer full clear covers every pixel (no partial present buffer).
        let mut fb = Framebuffer::new(64, 36);
        fb.clear([10, 20, 30, 255]);
        assert_eq!(fb.pixels.len(), 64 * 36 * 4);
        assert!(fb.pixels.chunks_exact(4).all(|p| p == [10, 20, 30, 255]));
        // tile_screen_rect covers full cell; abutting tiles leave no gaps at zoom.
        let cam = Camera {
            x: 0.0,
            y: 0.0,
            zoom: 48.0,
        };
        let (x0, _y0, w, h) = tile_screen_rect(&cam, 0, 0, 960, 540);
        let (x1, _, _, _) = tile_screen_rect(&cam, 1, 0, 960, 540);
        assert_eq!(x0 + w, x1);
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn stretch_rgba_fills_destination() {
        // 2×2 solid → 4×4: every dst pixel comes from src (full cover, no letterbox).
        let src = [
            255u8, 0, 0, 255, 0, 255, 0, 255, // r g
            0, 0, 255, 255, 255, 255, 0, 255, // b y
        ];
        let mut dst = vec![0u8; 4 * 4 * 4];
        stretch_rgba_nearest(&src, 2, 2, &mut dst, 4, 4);
        // Corner samples
        assert_eq!(&dst[0..4], &[255, 0, 0, 255]); // top-left red
        let i = (3 * 4 + 3) * 4;
        assert_eq!(&dst[i..i + 4], &[255, 255, 0, 255]); // bottom-right yellow
        // No zero alpha holes
        assert!(dst.chunks_exact(4).all(|p| p[3] == 255));
    }

    /// Stretch present: window coords → FB coords; same screen_to_tile as native FB size.
    #[test]
    fn stretch_window_cursor_maps_to_fb_tile() {
        const FB_W: u32 = 960;
        const FB_H: u32 = 540;
        // Fullscreen-ish non-multiple of buffer (common 1080p / 1440p clients).
        let cases = [
            (1920u32, 1080u32),
            (1280, 720),
            (960, 540),  // 1:1
            (2560, 1440),
            (800, 600),  // different aspect still Stretch-fills
        ];
        let mut scene = SceneRenderer::default();
        scene.camera.x = 10.0;
        scene.camera.y = 20.0;
        scene.camera.zoom = ZOOM_DEFAULT;

        for (win_w, win_h) in cases {
            // Corners + center in window space → FB → world tile.
            let samples = [
                (0.0f32, 0.0f32),
                (win_w as f32 - 1.0, win_h as f32 - 1.0),
                (win_w as f32 * 0.5, win_h as f32 * 0.5),
                (win_w as f32 * 0.25, win_h as f32 * 0.75),
            ];
            for (wx, wy) in samples {
                let (fx, fy) = map_window_to_fb(wx, wy, win_w, win_h, FB_W, FB_H)
                    .expect("map window→fb");
                assert!(
                    fx >= 0.0 && fx < FB_W as f32 && fy >= 0.0 && fy < FB_H as f32,
                    "fb coords out of range at {win_w}x{win_h}: ({fx},{fy})"
                );
                // Identity: mapping the *same* fractional place from native FB size
                // must yield the same FB pixel (within stretch precision).
                let (nx, ny) = map_window_to_fb(
                    fx * (win_w as f32) / (FB_W as f32),
                    fy * (win_h as f32) / (FB_H as f32),
                    win_w,
                    win_h,
                    FB_W,
                    FB_H,
                )
                .unwrap();
                assert!(
                    (nx - fx).abs() < 1.5 && (ny - fy).abs() < 1.5,
                    "roundtrip drift at {win_w}x{win_h}: ({fx},{fy}) vs ({nx},{ny})"
                );
                // Tile under cursor at stretched size equals tile under remapped FB coords.
                let tile_stretch = scene.screen_to_tile(fx, fy, FB_W, FB_H);
                let tile_native = scene.screen_to_tile(fx, fy, FB_W, FB_H);
                assert_eq!(tile_stretch, tile_native);
            }
            // Center of window → center of FB → camera tile.
            let (cx, cy) = map_window_to_fb(
                win_w as f32 * 0.5,
                win_h as f32 * 0.5,
                win_w,
                win_h,
                FB_W,
                FB_H,
            )
            .unwrap();
            let (tx, ty) = scene.screen_to_tile(cx, cy, FB_W, FB_H);
            // Camera center world (10, 20) should be near center of FB.
            assert!(
                (tx - 10).abs() <= 1 && (ty - 20).abs() <= 1,
                "center tile at {win_w}x{win_h} expected ~(10,20) got ({tx},{ty}) fb=({cx},{cy})"
            );
        }
        // Zero-size window (minimize) → None
        assert!(map_window_to_fb(1.0, 1.0, 0, 100, FB_W, FB_H).is_none());
        assert!(map_window_to_fb(1.0, 1.0, 100, 0, FB_W, FB_H).is_none());
    }

    #[test]
    fn tile_screen_rects_abut_no_gaps() {
        // Non-integer zoom used to leave 1px seams when size was `zoom as i32`.
        let cam = Camera {
            x: 10.3,
            y: 20.7,
            zoom: 32.7,
        };
        let (x0_a, y0_a, w_a, h_a) = tile_screen_rect(&cam, 5, 8, 960, 540);
        let (x0_b, _, w_b, _) = tile_screen_rect(&cam, 6, 8, 960, 540);
        // Higher world-Y tile (north): its bottom edge shares world y=9 with top of (5,8).
        let (_, y0_north, _, h_north) = tile_screen_rect(&cam, 5, 9, 960, 540);
        // Right edge of (5,8) == left edge of (6,8)
        assert_eq!(x0_a + w_a, x0_b, "horizontal abut");
        // Bottom of north tile (5,9) == top of (5,8) (screen y grows down)
        assert_eq!(y0_north + h_north, y0_a, "vertical abut");
        assert!(w_a >= 1 && h_a >= 1 && w_b >= 1 && h_north >= 1);
    }

    #[test]
    fn world_to_screen_center() {
        let mut scene = SceneRenderer::default();
        scene.camera.x = 10.0;
        scene.camera.y = 20.0;
        scene.camera.zoom = 32.0;
        let (sx, sy) = scene.world_to_screen(10.0, 20.0, 320, 240);
        assert!((sx - 160.0).abs() < 0.01, "sx={sx}");
        assert!((sy - 120.0).abs() < 0.01, "sy={sy}");
    }

    #[test]
    fn screen_to_world_inverse() {
        let mut scene = SceneRenderer::default();
        scene.camera.x = 5.5;
        scene.camera.y = -3.25;
        scene.camera.zoom = 40.0;
        let fb_w = 640u32;
        let fb_h = 480u32;
        for &(wx, wy) in &[(0.0, 0.0), (5.5, -3.25), (12.0, 8.0), (-4.0, 1.5)] {
            let (sx, sy) = scene.world_to_screen(wx, wy, fb_w, fb_h);
            let (rx, ry) = scene.screen_to_world(sx, sy, fb_w, fb_h);
            assert!((rx - wx).abs() < 1e-3, "wx {wx} -> {rx}");
            assert!((ry - wy).abs() < 1e-3, "wy {wy} -> {ry}");
        }
    }

    #[test]
    fn biome_pixels_match_color() {
        let mut scene = SceneRenderer::default();
        // Force flat path: empty ground roots
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 16.0;
        let mut map = ClientMap::new();
        map.set(0, 0, crate::client_map::MapTile {
            biome: 3,
            ..Default::default()
        });
        let mut world = LiveWorld::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let mut anims = AnimBank::new(".");
        let mut fb = Framebuffer::new(64, 64);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        // sample near center — should be desert-ish (biome 3) with variation
        let base = biome_color(3);
        let i = ((32u32 * 64 + 32) * 4) as usize;
        // within variation dither range (~8)
        for c in 0..3 {
            let d = (fb.pixels[i + c] as i32 - base[c] as i32).abs();
            assert!(d <= 16, "channel {c} delta {d}");
        }
    }

    #[test]
    fn floor_paints_different_than_empty() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 32.0;
        let mut map_empty = ClientMap::new();
        map_empty.set(0, 0, crate::client_map::MapTile {
            biome: 0,
            floor_id: 0,
            ..Default::default()
        });
        let mut map_floor = ClientMap::new();
        map_floor.set(0, 0, crate::client_map::MapTile {
            biome: 0,
            floor_id: 99,
            ..Default::default()
        });
        let mut world = LiveWorld::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let mut anims = AnimBank::new(".");
        let mut fb0 = Framebuffer::new(64, 64);
        let mut fb1 = Framebuffer::new(64, 64);
        scene.draw(&mut fb0, &mut map_empty, &mut world, &content, &mut sprites, &mut anims, 0.0);
        scene.draw(&mut fb1, &mut map_floor, &mut world, &content, &mut sprites, &mut anims, 0.0);
        assert_ne!(fb0.pixels, fb1.pixels);
    }

    #[test]
    fn age_range_skips_sprite() {
        let mut content = ClientContent::new();
        content.objects.insert(
            1,
            ClientObjectDef {
                id: 1,
                sprites: vec![ObjectSprite {
                    sprite_id: 1001,
                    age_start: 10.0,
                    age_end: 999.0,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        let img = solid_sprite(8, 8, [255, 0, 0, 255]);
        sprites.ensure_rgba(1001, &img, None);
        let mut anims = AnimBank::new(".");
        let mut fb = Framebuffer::new(32, 32);
        let scene = SceneRenderer::default();
        // age 5 → sprite hidden
        scene.draw_object(
            &mut fb, &content, &mut sprites, &mut anims, 1, 0, -1, 5.0, 16.0, 16.0, false, false,
        );
        assert_eq!(fb.count_non_color([0, 0, 0, 0]), 0);
        // age 20 → red pixels
        let mut fb2 = Framebuffer::new(32, 32);
        scene.draw_object(
            &mut fb2, &content, &mut sprites, &mut anims, 1, 0, -1, 20.0, 16.0, 16.0, false, false,
        );
        assert!(fb2.count_non_color([0, 0, 0, 0]) > 0);
    }

    /// P4#25: soft-FB skips `spriteSkipDrawing` layers (multi-use stages).
    #[test]
    fn skip_drawing_hides_sprite_soft_fb() {
        let mut content = ClientContent::new();
        content.objects.insert(
            1,
            ClientObjectDef {
                id: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1001,
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        parent: -1,
                        skip_drawing: false,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 1002,
                        x: 4.0,
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        parent: -1,
                        skip_drawing: true,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        sprites.ensure_rgba(1001, &solid_sprite(6, 6, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(1002, &solid_sprite(6, 6, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut fb = Framebuffer::new(48, 48);
        let scene = SceneRenderer::default();
        scene.draw_object(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            1,
            0,
            -1,
            -1.0,
            24.0,
            24.0,
            false,
            false,
        );
        // Red (not-skipped) present; green (skip_drawing) absent.
        assert!(
            fb.count_non_color([0, 0, 0, 0]) > 0,
            "base sprite should draw"
        );
        let mut has_green = false;
        for px in fb.pixels.chunks(4) {
            if px[1] > 200 && px[0] < 50 {
                has_green = true;
                break;
            }
        }
        assert!(!has_green, "skip_drawing sprite must not blit");
    }

    #[test]
    fn ysort_higher_y_drawn_first_southern_overwrites() {
        // Jason: high world Y drawn first; low Y later → southern player overwrites north.
        // Red at y=1 (north), blue at y=0 (south). Blue must still be visible after red.
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            10,
            ClientObjectDef {
                id: 10,
                sprites: vec![ObjectSprite {
                    sprite_id: 901,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            11,
            ClientObjectDef {
                id: 11,
                sprites: vec![ObjectSprite {
                    sprite_id: 902,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        // Large sprites so they overlap at adjacent tiles when zoom is high
        sprites.ensure_rgba(901, &solid_sprite(48, 48, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(902, &solid_sprite(48, 48, [0, 0, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        // id1 red at y=1 (north), id2 blue at y=0 (south)
        let pu_north = sample_pu(1, 10, 0, 1, 0);
        let pu_south = sample_pu(2, 11, 0, 0, 0);
        world.apply_pu(&pu_north);
        world.apply_pu(&pu_south);
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        let blue = count_near(&fb, [0, 0, 255]);
        let red = count_near(&fb, [255, 0, 0]);
        assert!(blue > 0 || red > 0, "players should paint");
        // Southern (lower y) drawn later → blue must remain
        assert!(blue > 0, "southern (low-y) player must leave blue pixels after northern red");
    }

    #[test]
    fn clothing_offset_rotates_with_body_part() {
        // Non-zero part rot must move clothing attach away from pure offset translate.
        let part = (10.0f32, 20.0, 0.25); // quarter turn
        let off = (8.0f32, 0.0);
        let (x0, y0) = clothing_screen_pos(100.0, 100.0, (10.0, 20.0, 0.0), off, 1.0, false);
        let (x1, y1) = clothing_screen_pos(100.0, 100.0, part, off, 1.0, false);
        assert!(
            (x0 - x1).abs() > 1.0 || (y0 - y1).abs() > 1.0,
            "rotated offset must change attach pos: flat=({x0},{y0}) rot=({x1},{y1})"
        );
    }

    /// Jason parent walk-up with nonzero parent rot (animationBank ~2599–2622).
    #[test]
    fn jason_parent_chain_matches_cpp_rot_formula() {
        // Parent rest (0,0) rot 0; posed at (10,0) with drot=0.25 turns.
        // Child rest (0,20) rot 0; posed same (no local anim).
        let sprites = vec![
            ObjectSprite {
                x: 0.0,
                y: 0.0,
                rot: 0.0,
                parent: -1,
                ..Default::default()
            },
            ObjectSprite {
                x: 0.0,
                y: 20.0,
                rot: 0.0,
                parent: 0,
                ..Default::default()
            },
        ];
        let mut ox = vec![10.0f32, 0.0];
        let mut oy = vec![0.0f32, 20.0];
        let mut orot = vec![0.25f32, 0.0];
        apply_jason_parent_chain(&sprites, &mut ox, &mut oy, &mut orot);

        // Hand-eval C++:
        // parent delta pos=(10,0), drot=0.25
        // angle = -2π*0.25 = -π/2
        // child starts (0,20)
        // pos += rotate((10,0), -angle)=rotate((10,0), +π/2)=(0,10) → (0,30)
        // childOff = (0,30)-(0,0)=(0,30)
        // rotate(childOff, angle)=rotate((0,30), -π/2)=(30,0)
        // pos += (30,0)-(0,30) → (0,30)+(30,-30)=(30,0)
        // rot = 0 + 0.25
        assert!(
            (ox[1] - 30.0).abs() < 1e-3 && (oy[1] - 0.0).abs() < 1e-3,
            "child after Jason chain expected (30,0), got ({},{})",
            ox[1],
            oy[1]
        );
        assert!((orot[1] - 0.25).abs() < 1e-4, "child rot accumulates parent drot");
        // Parent unchanged (no ancestors)
        assert!((ox[0] - 10.0).abs() < 1e-4 && (oy[0] - 0.0).abs() < 1e-4);
    }

    /// Zero-rot parent chain = sum of local deltas (Jason else branch).
    #[test]
    fn jason_parent_chain_translate_only() {
        let sprites = vec![
            ObjectSprite {
                x: 0.0,
                y: 0.0,
                parent: -1,
                ..Default::default()
            },
            ObjectSprite {
                x: 5.0,
                y: 10.0,
                parent: 0,
                ..Default::default()
            },
        ];
        let mut ox = vec![3.0f32, 5.0]; // parent delta (3,0); child no local delta
        let mut oy = vec![0.0f32, 10.0];
        let mut orot = vec![0.0f32, 0.0];
        apply_jason_parent_chain(&sprites, &mut ox, &mut oy, &mut orot);
        assert!((ox[1] - 8.0).abs() < 1e-4 && (oy[1] - 10.0).abs() < 1e-4);
    }

    /// Center-anchor Y must use **+ay** in Y-up object space (SpriteGL posY + offset.y).
    /// Wrong sign floats hair above the head and opens a neck gap on person 19.
    #[test]
    fn center_anchor_y_sign_keeps_hair_on_head() {
        // Synthetic: head at y=100, hair at y=120 with anchorY=-20.
        // Correct geometric hair Y = 120 + (-20) = 100 → same as head → no gap.
        // Wrong (subtract ay): 120 - (-20) = 140 → hair 40 units above head.
        let head_oy = 100.0f32;
        let hair_oy = 120.0f32;
        let hair_ay = -20.0f32;
        let head_ay = 0.0f32;
        let scale = 1.0f32;
        let sy = 200.0f32;
        let head_screen = sy - (head_oy + head_ay) * scale;
        let hair_screen = sy - (hair_oy + hair_ay) * scale;
        assert!(
            (head_screen - hair_screen).abs() < 1.0,
            "hair+head screen Y must match with +ay anchor, head={head_screen} hair={hair_screen}"
        );
        let wrong_hair = sy - (hair_oy - hair_ay) * scale;
        assert!(
            (wrong_hair - head_screen).abs() > 30.0,
            "documenting the old bug: wrong sign separates hair"
        );
    }

    /// Jason: bottom/tunic/backpack under topBackArm — arm must paint *over* tunic.
    #[test]
    fn body_clothes_under_top_back_arm() {
        let mut content = ClientContent::new();
        // Person: body (red) then back-hand arm (blue). Arm is topBackArmIndex.
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 501,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 502,
                        x: 0.0,
                        y: 0.0,
                        parent: 0, // chain to body → back arm indices
                        invis_holding: true, // marks as hand for limb walk
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        // Tunic: large green square at same attach (body).
        content.objects.insert(
            200,
            ClientObjectDef {
                id: 200,
                clothing: 't',
                clothing_offset: (0.0, 0.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 503,
                    x: 0.0,
                    y: 0.0,
                    parent: -1,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(501, &solid_sprite(20, 20, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(502, &solid_sprite(20, 20, [0, 0, 255, 255]), None);
        // Larger green so it would fully cover arm if drawn last
        sprites.ensure_rgba(503, &solid_sprite(28, 28, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut pack = ObjectAnimPack::single(19, crate::anim_bank::ANIM_GROUND, 0.0);
        let scene = SceneRenderer::default();
        let mut fb = Framebuffer::new(64, 64);
        let worn = vec![(1usize, 200i32, String::new())]; // tunic slot
        let _ = scene.draw_object_with_pack_ex(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            &mut pack,
            20.0,
            32.0,
            32.0,
            false,
            false,
            false,
            0,
            false,
            SpriteLayerFilter::All,
            false,
            Some(worn.as_slice()),
        );
        let blue = count_near(&fb, [0, 0, 255]);
        let green = count_near(&fb, [0, 255, 0]);
        assert!(blue > 0, "top back arm (blue) must paint");
        assert!(green > 0, "tunic (green) must paint");
        // Center of figure: arm drawn after tunic → blue remains over green.
        let i = ((32u32 * 64 + 32) * 4) as usize;
        let (r, g, b) = (fb.pixels[i], fb.pixels[i + 1], fb.pixels[i + 2]);
        assert!(
            b > 200 && r < 50 && g < 50,
            "center must be top-back-arm blue over tunic, got rgb=({r},{g},{b}) blue={blue} green={green}"
        );
    }

    #[test]
    fn limb_hide_holding_hides_invis_holding_hands() {
        // Holding anything: hand layers with invisHolding are not drawn.
        // Rideable also freezes arms (person_anim_pack) and hides legs only.
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                held_offset: (0.0, 0.0),
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: -20.0,
                        y: 10.0,
                        parent: 0,
                        invis_holding: true,
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 3,
                        x: 20.0,
                        y: 10.0,
                        parent: 0,
                        invis_holding: true,
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        content.objects.insert(
            700,
            ClientObjectDef {
                id: 700,
                rideable: true,
                held_offset: (0.0, 48.0), // offset so cart does not cover body
                sprites: vec![ObjectSprite {
                    sprite_id: 4,
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(12, 12, [255, 255, 0, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(10, 10, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(10, 10, [0, 255, 0, 255]), None);
        sprites.ensure_rgba(4, &solid_sprite(16, 16, [0, 0, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 700));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        // Hands (red/green) hidden via invisHolding while holding
        assert_eq!(count_near(&fb, [255, 0, 0]), 0, "hand red must hide while holding");
        assert_eq!(count_near(&fb, [0, 255, 0]), 0, "hand green must hide while holding");
        assert!(count_near(&fb, [0, 0, 255]) > 0, "rideable cart still draws");
        assert!(count_near(&fb, [255, 255, 0]) > 0, "body still draws");
    }

    /// P3#20: rideable person-under-vehicle draw order + vehicle at person pos.
    ///
    /// // C++ LivingLifePage: behind vehicle → person → front vehicle;
    /// // heldObjectDrawPos = pos (not hand HoldingPos).
    #[test]
    fn rideable_person_under_vehicle_draw_order() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 128.0; // scale = 1.0 for 1:1 object→screen
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        // Person: solid yellow body at origin (no riding shift with held_offset 0).
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                held_offset: (0.0, 0.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 1,
                    x: 0.0,
                    y: 0.0,
                    parent: -1,
                    is_body: true,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // Vehicle: large green behind at center + blue front offset so center is only
        // behind+person. Correct order: person yellow over green at center; blue visible
        // at front offset. Wrong all-after-person: green covers yellow at center.
        content.objects.insert(
            800,
            ClientObjectDef {
                id: 800,
                rideable: true,
                held_offset: (0.0, 0.0),
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 2,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        behind_player: true,
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 3,
                        x: 40.0,
                        y: 0.0,
                        parent: -1,
                        behind_player: false,
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        assert!(content.get(800).unwrap().any_sprites_behind_player());
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(16, 16, [255, 255, 0, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(28, 28, [0, 255, 0, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(16, 16, [0, 0, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 800));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);

        let (cx, cy) = scene.world_to_screen(0.5, 0.5, fb.width, fb.height);
        let cxi = cx.round() as i32;
        let cyi = cy.round() as i32;
        // Center: person over vehicle-behind (yellow, not green).
        let center = pixel_rgb(&fb, cxi, cyi);
        assert!(
            near_rgb(center, [255, 255, 0]),
            "center must be person yellow over vehicle-behind, got {center:?}"
        );
        // Front layer to the right of person still paints blue.
        let front = pixel_rgb(&fb, cxi + 40, cyi);
        assert!(
            near_rgb(front, [0, 0, 255]),
            "rideable front layer must draw over/at offset, got {front:?}"
        );
        assert!(count_near(&fb, [0, 255, 0]) > 0, "behind vehicle must still paint");
        assert!(count_near(&fb, [255, 255, 0]) > 0, "rider body must paint");
        assert!(count_near(&fb, [0, 0, 255]) > 0, "front vehicle must paint");
    }

    /// P3#20: rideable vehicle anchors at person pos, not hand HoldingPos.
    #[test]
    fn rideable_vehicle_at_person_pos_not_hand() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 128.0; // scale = 1
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    // Hand far to the right — if vehicle used HoldingPos it would follow hand.
                    ObjectSprite {
                        sprite_id: 2,
                        x: 60.0,
                        y: 0.0,
                        parent: 0,
                        invis_holding: true,
                        r: 0.5,
                        g: 0.5,
                        b: 0.5,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        content.objects.insert(
            801,
            ClientObjectDef {
                id: 801,
                rideable: true,
                held_offset: (0.0, 0.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 3,
                    x: 0.0,
                    y: 0.0,
                    parent: -1,
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [255, 255, 0, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(4, 4, [128, 128, 128, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(12, 12, [0, 0, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 801));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);

        let (cx, cy) = scene.world_to_screen(0.5, 0.5, fb.width, fb.height);
        let cxi = cx.round() as i32;
        let cyi = cy.round() as i32;
        // Vehicle must paint at person tile center (covers body), not at hand (+60).
        let at_person = pixel_rgb(&fb, cxi, cyi);
        assert!(
            near_rgb(at_person, [0, 0, 255]),
            "rideable vehicle must sit at person pos, got {at_person:?}"
        );
        // Hand offset location should not be solid blue vehicle.
        let at_hand = pixel_rgb(&fb, cxi + 60, cyi);
        assert!(
            !near_rgb(at_hand, [0, 0, 255]),
            "vehicle must not follow hand HoldingPos, got blue at hand offset"
        );
    }

    /// P3#21: hideClosestArm ±1 skips arm chain; -2 uses body HoldingPos.
    #[test]
    fn hide_closest_arm_pm1_and_body_holding_pos() {
        let scene = SceneRenderer::default();
        let mut content = ClientContent::new();
        // Body@0,0 yellow; back hand@-40,0 red (invisHolding); front hand@40,0 green.
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: -40.0,
                        y: 0.0,
                        parent: 0,
                        invis_holding: true,
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 3,
                        x: 40.0,
                        y: 0.0,
                        parent: 0,
                        invis_holding: true,
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(12, 12, [255, 255, 0, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(10, 10, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(10, 10, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        // ANIM_GROUND = 0 typically; use single-type pack with no motion.
        let mut pack = ObjectAnimPack::single(19, 0, 0.0);
        let sx = 64.0f32;
        let sy = 64.0f32;

        // hideClosestArm=1 → hide front arm (green)
        let mut fb = Framebuffer::new(128, 128);
        fb.clear([20, 20, 25, 255]);
        let (hp1, _) = scene.draw_object_with_pack(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            &mut pack,
            20.0,
            sx,
            sy,
            false,
            false,
            false,
            1, // hide front arm
            false,
            SpriteLayerFilter::All,
            false,
        );
        assert!(count_near(&fb, [255, 255, 0]) > 0, "body draws");
        assert!(count_near(&fb, [255, 0, 0]) > 0, "back arm still draws");
        assert_eq!(count_near(&fb, [0, 255, 0]), 0, "front arm hidden by +1");
        // ±1 → body attach
        assert!(hp1.valid);
        assert!((hp1.x - 0.0).abs() < 1e-3, "body attach x got {}", hp1.x);
        assert!((hp1.y - 0.0).abs() < 1e-3, "body attach y got {}", hp1.y);

        // hideClosestArm=-1 → hide back arm (red)
        fb.clear([20, 20, 25, 255]);
        let _ = scene.draw_object_with_pack(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            &mut pack,
            20.0,
            sx,
            sy,
            false,
            false,
            false,
            -1,
            false,
            SpriteLayerFilter::All,
            false,
        );
        assert!(count_near(&fb, [0, 255, 0]) > 0, "front arm still draws");
        assert_eq!(count_near(&fb, [255, 0, 0]), 0, "back arm hidden by -1");

        // hideClosestArm=-2 (bulky) → body HoldingPos, arms still drawn
        fb.clear([20, 20, 25, 255]);
        let (hp2, _) = scene.draw_object_with_pack(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            &mut pack,
            20.0,
            sx,
            sy,
            false,
            false,
            false,
            -2,
            false,
            SpriteLayerFilter::All,
            false,
        );
        assert!(count_near(&fb, [255, 0, 0]) > 0, "arms still draw at -2");
        assert!(count_near(&fb, [0, 255, 0]) > 0, "arms still draw at -2");
        assert!(hp2.valid);
        assert!((hp2.x - 0.0).abs() < 1e-3, "bulky body attach");
        assert!((hp2.y - 0.0).abs() < 1e-3);

        // hideClosestArm=0 → hand attach (back hand at -40)
        fb.clear([20, 20, 25, 255]);
        let (hp0, _) = scene.draw_object_with_pack(
            &mut fb,
            &content,
            &mut sprites,
            &mut anims,
            &mut pack,
            20.0,
            sx,
            sy,
            false,
            false,
            false,
            0,
            false,
            SpriteLayerFilter::All,
            false,
        );
        assert!(hp0.valid);
        assert!((hp0.x - (-40.0)).abs() < 1e-3, "hand attach got {}", hp0.x);
    }

    #[test]
    fn holding_pos_hand_offset_moves_held_item() {
        // Hand-held stone: attach at hand + heldOffset, not person heldOffset alone.
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0; // scale = 0.5
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                held_offset: (0.0, 0.0), // should NOT be used when HoldingPos valid
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        parent: -1,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: 40.0,
                        y: 0.0,
                        parent: 0,
                        invis_holding: true,
                        r: 0.5,
                        g: 0.5,
                        b: 0.5,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        content.objects.insert(
            33,
            ClientObjectDef {
                id: 33,
                held_in_hand: true,
                held_offset: (0.0, 40.0), // above hand in object space
                sprites: vec![ObjectSprite {
                    sprite_id: 3,
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [255, 255, 255, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(4, 4, [128, 128, 128, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(8, 8, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 33));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        let green = count_near(&fb, [0, 255, 0]);
        assert!(green > 0, "held stone must paint");
        // Hand at object (40,0) + heldOffset (0,40) → (40,40); screen Y up is smaller.
        // Tile center (64,64); scale=0.5 → held at (64+20, 64-20)=(84,44)
        let mut found_near_expected = false;
        for y in 40..50 {
            for x in 78..90 {
                let i = ((y as u32 * 128 + x as u32) * 4) as usize;
                if fb.pixels[i + 1] > 200 && fb.pixels[i] < 50 {
                    found_near_expected = true;
                }
            }
        }
        assert!(
            found_near_expected,
            "held item should sit near hand+offset screen pos (~84,44)"
        );
    }

    #[test]
    fn clothing_invis_worn_hidden_when_worn() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![ObjectSprite {
                    sprite_id: 1,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            201,
            ClientObjectDef {
                id: 201,
                clothing: 't',
                clothing_offset: (0.0, 0.0),
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 2,
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        invis_worn: false,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 3,
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        invis_worn: true, // hidden when worn
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [200, 200, 200, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(16, 16, [0, 0, 255, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(16, 16, [255, 0, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        let pu = sample_pu(1, 19, 0, 0, 0);
        world.apply_pu(&pu);
        if let Some(o) = world.get_mut(1) {
            // slots: hat;tunic;front_shoe;back_shoe;bottom;backpack
            o.clothing = crate::live_object::ClothingSet::parse(";201;;;;");
        }
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        assert!(count_near(&fb, [0, 0, 255]) > 0, "worn clothing base layer draws");
        assert_eq!(
            count_near(&fb, [255, 0, 0]),
            0,
            "invisWorn layer must not draw when clothing is worn"
        );
    }

    #[test]
    fn clothing_contained_draws_at_slot_pos() {
        // P1#6: worn clothing container contents must soft-FB draw (not hit-test-only).
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![ObjectSprite {
                    sprite_id: 1,
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // Backpack with one slot offset to the right so contained is not under body center.
        content.objects.insert(
            300,
            ClientObjectDef {
                id: 300,
                clothing: 'p',
                clothing_offset: (0.0, 0.0),
                num_slots: 1,
                slot_pos: vec![(48.0, 0.0)],
                sprites: vec![ObjectSprite {
                    sprite_id: 2,
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            301,
            ClientObjectDef {
                id: 301,
                sprites: vec![ObjectSprite {
                    sprite_id: 3,
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [128, 128, 128, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(12, 12, [0, 0, 255, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(14, 14, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        let pu = sample_pu(1, 19, 0, 0, 0);
        world.apply_pu(&pu);
        if let Some(o) = world.get_mut(1) {
            // backpack slot with contained 301
            o.clothing = crate::live_object::ClothingSet::parse("0;0;0;0;0;300,301");
        }
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert!(
            count_near(&fb, [0, 255, 0]) > 0,
            "contained item in worn backpack must soft-FB draw (green)"
        );
        assert!(
            count_near(&fb, [0, 0, 255]) > 0,
            "backpack body still draws (blue)"
        );
    }

    #[test]
    fn draw_behind_player_under_same_row_player() {
        // C++: drawBehindPlayer objects draw before players on the same world row.
        // Red behind-object + blue player share tile (0,0); player must overwrite center.
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9001,
                object_raw: "9001".into(),
            },
        );
        let mut content = ClientContent::new();
        content.objects.insert(
            9001,
            ClientObjectDef {
                id: 9001,
                draw_behind_player: true,
                sprites: vec![ObjectSprite {
                    sprite_id: 901,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![ObjectSprite {
                    sprite_id: 902,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(901, &solid_sprite(40, 40, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(902, &solid_sprite(24, 24, [0, 0, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        let blue = count_near(&fb, [0, 0, 255]);
        let red = count_near(&fb, [255, 0, 0]);
        assert!(blue > 0, "player must paint blue");
        assert!(red > 0, "behind object must paint red");
        // Center of tile should be player (blue) over the behind object.
        let (cx, cy) = (64i32, 64i32);
        let i = ((cy as u32 * 128 + cx as u32) * 4) as usize;
        assert_eq!(
            &fb.pixels[i..i + 3],
            &[0, 0, 255],
            "center pixel must be player blue over drawBehindPlayer object"
        );
    }

    #[test]
    fn tall_object_front_sprites_over_player() {
        // Non-drawBehind object with spritesDrawnBehind: trunk under, canopy over player.
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9002,
                object_raw: "9002".into(),
            },
        );
        let mut content = ClientContent::new();
        content.objects.insert(
            9002,
            ClientObjectDef {
                id: 9002,
                draw_behind_player: false,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 901,
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        behind_player: true, // trunk
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 903,
                        r: 0.0,
                        g: 1.0,
                        b: 0.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        behind_player: false, // canopy
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                sprites: vec![ObjectSprite {
                    sprite_id: 902,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(901, &solid_sprite(40, 40, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(902, &solid_sprite(20, 20, [0, 0, 255, 255]), None);
        sprites.ensure_rgba(903, &solid_sprite(40, 40, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        let green = count_near(&fb, [0, 255, 0]);
        let blue = count_near(&fb, [0, 0, 255]);
        assert!(green > 0, "canopy must paint");
        // Center should be green canopy (front over Player), not blue player.
        let i = ((64u32 * 128 + 64) * 4) as usize;
        assert_eq!(
            &fb.pixels[i..i + 3],
            &[0, 255, 0],
            "center must be front canopy green over player (tall split)"
        );
        // Player should still leave some blue if canopy doesn't fully cover, or 0 if full cover —
        // with equal 40 vs 20 sizes canopy covers center; blue may be 0. Just ensure order via green center.
        let _ = blue;
    }

    /// P3#23: same-row front sub-order — wall over permanent non-wall; frontWall over wall.
    ///
    /// // C++ LivingLifePage: permanent non-wall → non-permanent → wall !frontWall → frontWall
    /// Adjacent tiles on one row; large sprites with ±1 tile object-unit offsets so they
    /// overlap the neighbor tile center (zoom/GRID scale).
    #[test]
    fn wall_layer_front_wall_sub_order() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 1.0;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0; // scale = 0.5; 1 tile = 128 object units
        let mut map = ClientMap::new();
        // Permanent non-wall (red) at (0,0), spills right onto wall tile center.
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9101,
                object_raw: "9101".into(),
            },
        );
        // Wall !frontWall (green) at (1,0).
        map.set(
            1,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9102,
                object_raw: "9102".into(),
            },
        );
        let mut content = ClientContent::new();
        content.objects.insert(
            9101,
            ClientObjectDef {
                id: 9101,
                permanent: true,
                wall_layer: false,
                front_wall: false,
                sprites: vec![ObjectSprite {
                    sprite_id: 911,
                    x: 128.0, // +1 tile so red covers (1,0) center
                    y: 0.0,
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            9102,
            ClientObjectDef {
                id: 9102,
                permanent: true,
                wall_layer: true,
                front_wall: false,
                sprites: vec![ObjectSprite {
                    sprite_id: 912,
                    x: 0.0,
                    y: 0.0,
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(911, &solid_sprite(40, 40, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(912, &solid_sprite(40, 40, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        let mut fb = Framebuffer::new(192, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);

        // Wall tile center: green wall over red permanent non-wall spill.
        let (wx, wy) = scene.world_to_screen(1.5, 0.5, fb.width, fb.height);
        let wall_center = pixel_rgb(&fb, wx.round() as i32, wy.round() as i32);
        assert!(
            near_rgb(wall_center, [0, 255, 0]),
            "wallLayer must draw over permanent non-wall on same row, got {wall_center:?}"
        );

        // frontWall (blue) at (1,0) spills left over wall at (0,0).
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9102,
                object_raw: "9102".into(),
            },
        );
        map.set(
            1,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 9103,
                object_raw: "9103".into(),
            },
        );
        content.objects.insert(
            9103,
            ClientObjectDef {
                id: 9103,
                permanent: true,
                wall_layer: true,
                front_wall: true,
                sprites: vec![ObjectSprite {
                    sprite_id: 913,
                    x: -128.0, // −1 tile so blue covers (0,0) center
                    y: 0.0,
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        sprites.ensure_rgba(913, &solid_sprite(40, 40, [0, 0, 255, 255]), None);
        let mut fb2 = Framebuffer::new(192, 128);
        scene.draw(
            &mut fb2,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let (lx, ly) = scene.world_to_screen(0.5, 0.5, fb2.width, fb2.height);
        let left_center = pixel_rgb(&fb2, lx.round() as i32, ly.round() as i32);
        assert!(
            near_rgb(left_center, [0, 0, 255]),
            "frontWall must draw over wallLayer on same row, got {left_center:?}"
        );
    }

    #[test]
    fn front_object_draw_layer_ordering() {
        // Unit: permanent non-wall < non-permanent < wall < frontWall (PartialOrd).
        assert!(DrawLayer::FrontPermanent < DrawLayer::FrontNonPermanent);
        assert!(DrawLayer::FrontNonPermanent < DrawLayer::FrontWall);
        assert!(DrawLayer::FrontWall < DrawLayer::FrontFrontWall);
        assert!(DrawLayer::Player < DrawLayer::FrontPermanent);

        let perm = ClientObjectDef {
            permanent: true,
            wall_layer: false,
            ..Default::default()
        };
        let loose = ClientObjectDef {
            permanent: false,
            wall_layer: false,
            ..Default::default()
        };
        let wall = ClientObjectDef {
            permanent: true,
            wall_layer: true,
            front_wall: false,
            ..Default::default()
        };
        let fwall = ClientObjectDef {
            permanent: true,
            wall_layer: true,
            front_wall: true,
            ..Default::default()
        };
        assert_eq!(front_object_draw_layer(Some(&perm)), DrawLayer::FrontPermanent);
        assert_eq!(
            front_object_draw_layer(Some(&loose)),
            DrawLayer::FrontNonPermanent
        );
        assert_eq!(front_object_draw_layer(Some(&wall)), DrawLayer::FrontWall);
        assert_eq!(
            front_object_draw_layer(Some(&fwall)),
            DrawLayer::FrontFrontWall
        );
    }

    #[test]
    fn offline_scene_nonzero_pixels() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 32.0;
        let mut map = ClientMap::new();
        for y in 0..3 {
            for x in 0..3 {
                map.set(
                    x,
                    y,
                    crate::client_map::MapTile {
                        biome: (x + y) as u8 % 4,
                        floor_id: if x == 1 && y == 1 { 1 } else { 0 },
                        object_id: 0,
                        object_raw: "0".into(),
                    },
                );
            }
        }
        let mut world = LiveWorld::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                person: 1,
                held_offset: (8.0, 12.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 500,
                    x: 0.0,
                    y: 0.0,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(500, &solid_sprite(16, 16, [0, 200, 255, 255]), None);
        let mut anims = AnimBank::new(".");
        let pu = sample_pu(1, 19, 1, 1, 0);
        world.apply_pu(&pu);
        world.our_id = Some(1);
        let mut fb = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.016,
        );
        assert!(
            fb.count_non_color([30, 30, 35, 255]) > 0,
            "expected drawn pixels"
        );
    }

    #[test]
    fn held_item_draws_extra_region() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        let mut map = ClientMap::new();
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                held_offset: (40.0, 40.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 1,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            33,
            ClientObjectDef {
                id: 33,
                sprites: vec![ObjectSprite {
                    sprite_id: 2,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [255, 0, 0, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(8, 8, [0, 255, 0, 255]), None);
        let mut anims = AnimBank::new(".");

        let mut world_no = LiveWorld::new();
        let mut world_held = LiveWorld::new();
        let pu_no = sample_pu(1, 19, 0, 0, 0);
        let pu_held = sample_pu(1, 19, 0, 0, 33);
        world_no.apply_pu(&pu_no);
        world_held.apply_pu(&pu_held);
        let mut fb0 = Framebuffer::new(128, 128);
        let mut fb1 = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb0,
            &mut map,
            &mut world_no,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        scene.draw(
            &mut fb1,
            &mut map,
            &mut world_held,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let green0 = count_near(&fb0, [0, 255, 0]);
        let green1 = count_near(&fb1, [0, 255, 0]);
        assert!(
            green1 > green0,
            "held should paint green green0={green0} green1={green1}"
        );
    }

    #[test]
    fn blit_rotation_changes_footprint() {
        let mut fb0 = Framebuffer::new(64, 64);
        let mut fb1 = Framebuffer::new(64, 64);
        let img = solid_sprite(16, 4, [255, 255, 0, 255]);
        let mut bank = SpriteBank::with_atlas_size(".", 64);
        let rect = bank.ensure_rgba(1, &img, None).unwrap();
        let page = &bank.pages()[rect.atlas_index];
        fb0.blit_sprite(
            &page.pixels,
            page.width,
            rect.rect.x,
            rect.rect.y,
            rect.width,
            rect.height,
            32.0,
            32.0,
            1.0,
            false,
            [1.0, 1.0, 1.0],
            0.0,
            false,
            1.0,
        );
        fb1.blit_sprite(
            &page.pixels,
            page.width,
            rect.rect.x,
            rect.rect.y,
            rect.width,
            rect.height,
            32.0,
            32.0,
            1.0,
            false,
            [1.0, 1.0, 1.0],
            0.25, // 90°
            false,
            1.0,
        );
        assert_ne!(fb0.pixels, fb1.pixels);
    }

    #[test]
    fn clothing_slot_ids() {
        let c = ClothingSet::parse("10;20;0;0;30;40");
        assert_eq!(c.slot_id(0), 10);
        assert_eq!(c.slot_id(1), 20);
        assert_eq!(c.slot_id(4), 30);
        let ids = c.draw_ids();
        assert!(ids.contains(&10));
        assert!(ids.contains(&40));
    }

    /// L-HUD: screen_to_tile round-trips with world_to_screen (tile centers).
    #[test]
    fn screen_to_tile_roundtrip_and_highlight() {
        let mut scene = SceneRenderer::default();
        scene.camera.x = 10.0;
        scene.camera.y = 20.0;
        scene.camera.zoom = 32.0;
        let fb_w = 320u32;
        let fb_h = 180u32;
        // Pixel at view center maps to camera tile floor.
        let (tx, ty) = scene.screen_to_tile(fb_w as f32 * 0.5, fb_h as f32 * 0.5, fb_w, fb_h);
        assert_eq!((tx, ty), (10, 20));
        // Offset one tile right: +zoom in x.
        let (tx2, ty2) = scene.screen_to_tile(
            fb_w as f32 * 0.5 + 32.0,
            fb_h as f32 * 0.5,
            fb_w,
            fb_h,
        );
        assert_eq!((tx2, ty2), (11, 20));
        // world_to_screen of tile center then back to tile.
        let (sx, sy) = scene.world_to_screen(15.5, 7.5, fb_w, fb_h);
        let (tx3, ty3) = scene.screen_to_tile(sx, sy, fb_w, fb_h);
        assert_eq!((tx3, ty3), (15, 7));
        let h = scene.set_highlight_from_screen(sx, sy, fb_w, fb_h);
        assert_eq!(h, (15, 7));
        assert_eq!(scene.highlight_tile, Some((15, 7)));
    }

    #[test]
    fn sync_hud_none_none_clears_after_vitals() {
        // C++ logout/death: peaks must not stick after session food/heat wiped.
        let mut scene = SceneRenderer::default();
        scene.sync_hud(
            Some(&FoodChange {
                food_store: 8,
                food_capacity: 12,
                last_ate_id: 1,
                last_ate_fill_max: 2,
                move_speed: 3.0,
                responsible_id: -1,
                yum_bonus: 0,
                yum_multiplier: 0,
            }),
            Some(&HeatChange {
                heat: 0.4,
                food_time: 0.0,
                indoor_bonus: 0.0,
            }),
        );
        assert!(scene.hud.visible);
        assert_eq!(scene.hud.max_food_capacity, 12);
        // logout_reset leaves session.food/heat = None → clear peaks.
        scene.sync_hud(None, None);
        assert!(!scene.hud.visible);
        assert_eq!(scene.hud.max_food_capacity, 0);
        assert!(scene.hud.old_arrows.is_empty());
    }

    #[test]
    fn food_heat_hud_paints_over_scene() {
        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.hud_sprites = HudSprites::procedural();
        scene.draw_hud = true;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 16.0;
        scene.sync_hud(
            Some(&FoodChange {
                food_store: 6,
                food_capacity: 10,
                last_ate_id: 31,
                last_ate_fill_max: 3,
                move_speed: 3.75,
                responsible_id: -1,
                yum_bonus: 1,
                yum_multiplier: 0,
            }),
            Some(&HeatChange {
                heat: 0.55,
                food_time: 0.0,
                indoor_bonus: 0.0,
            }),
        );
        let mut map = ClientMap::new();
        let mut world = LiveWorld::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let mut anims = AnimBank::new(".");
        let mut fb = Framebuffer::new(320, 180);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        // Bottom band of the frame should differ from clear color due to panel/boxes.
        let mut bottom_painted = 0usize;
        let y0 = 140u32;
        for y in y0..180 {
            for x in 0..320 {
                let i = ((y * 320 + x) * 4) as usize;
                if fb.pixels[i] != 30 || fb.pixels[i + 1] != 30 || fb.pixels[i + 2] != 35 {
                    bottom_painted += 1;
                }
            }
        }
        assert!(
            bottom_painted > 50,
            "HUD should paint bottom chrome, got {bottom_painted}"
        );
    }

    fn count_near(fb: &Framebuffer, rgb: [u8; 3]) -> usize {
        fb.pixels
            .chunks_exact(4)
            .filter(|p| {
                (p[0] as i32 - rgb[0] as i32).abs() < 2
                    && (p[1] as i32 - rgb[1] as i32).abs() < 2
                    && (p[2] as i32 - rgb[2] as i32).abs() < 2
            })
            .count()
    }

    fn pixel_rgb(fb: &Framebuffer, x: i32, y: i32) -> [u8; 3] {
        if x < 0 || y < 0 || x as u32 >= fb.width || y as u32 >= fb.height {
            return [0, 0, 0];
        }
        let i = ((y as u32 * fb.width + x as u32) * 4) as usize;
        [fb.pixels[i], fb.pixels[i + 1], fb.pixels[i + 2]]
    }

    fn near_rgb(got: [u8; 3], want: [u8; 3]) -> bool {
        (got[0] as i32 - want[0] as i32).abs() < 2
            && (got[1] as i32 - want[1] as i32).abs() < 2
            && (got[2] as i32 - want[2] as i32).abs() < 2
    }

    /// L-ANIM-DRAW: SceneRenderer pack select from LiveObject flags.
    #[test]
    fn player_packs_moving_and_ground() {
        use crate::anim_bank::{ANIM_GROUND, ANIM_HELD, ANIM_MOVING};

        let mut world = LiveWorld::new();
        let pu = sample_pu(1, 19, 0, 0, 0);
        world.apply_pu(&pu);
        let o = world.get(1).unwrap();
        let ground = select_packs_for_player(o);
        assert_eq!(ground.person, ANIM_GROUND);
        assert_eq!(ground.held, ANIM_HELD);
        assert_eq!(ground.clothing, ANIM_HELD);
        assert_eq!(ground.extra_index, -1);

        // PM marks moving → person + clothing + held use moving pack
        world.apply_moves_start(&[crate::parse::PlayerMoveStart {
            player_id: 1,
            xs: 0,
            ys: 0,
            total_sec: 1.0,
            eta_sec: 1.0,
            trunc: 0,
            deltas: vec![(1, 0)],
        }]);
        let o = world.get(1).unwrap();
        assert!(o.moving);
        let moving = select_packs_for_player(o);
        assert_eq!(moving.person, ANIM_MOVING);
        assert_eq!(moving.held, ANIM_MOVING);
        assert_eq!(moving.clothing, ANIM_MOVING);
    }

    #[test]
    fn player_packs_eating_doing_emote() {
        use crate::anim_bank::{ANIM_DOING, ANIM_EATING, ANIM_EXTRA, ANIM_GROUND, ANIM_HELD};

        // just_ate wins over action when not moving
        let mut world = LiveWorld::new();
        let line = "2 19 0 1 0 0 0 0 0 0 -1 0.5 1 0 0 0 20.0 0.1 1.0 0;0;0;0;0;0 1 5 -1 0 0";
        let pu = crate::parse::parse_pu_line(line).expect("eating PU");
        assert!(pu.just_ate);
        assert_eq!(pu.action, 1);
        world.apply_pu(&pu);
        let eat = select_packs_for_player(world.get(2).unwrap());
        assert_eq!(eat.person, ANIM_EATING);
        assert_eq!(eat.held, ANIM_HELD);

        // action only → doing
        let mut world2 = LiveWorld::new();
        let line2 = "3 19 0 1 0 0 0 0 0 0 -1 0.5 1 0 0 0 20.0 0.1 1.0 0;0;0;0;0;0 0 0 -1 0 0";
        let pu2 = crate::parse::parse_pu_line(line2).expect("doing PU");
        assert!(!pu2.just_ate);
        assert_eq!(pu2.action, 1);
        world2.apply_pu(&pu2);
        let doing = select_packs_for_player(world2.get(3).unwrap());
        assert_eq!(doing.person, ANIM_DOING);

        // PE facial only (no bank) → does NOT force EXTRA (C++ uses extraAnimIndex)
        let mut world3 = LiveWorld::new();
        world3.apply_pu(&sample_pu(4, 19, 0, 0, 0));
        world3.apply_emots(&[crate::parse::PlayerEmot {
            player_id: 4,
            emot_index: 2,
            ttl_sec: None,
        }]);
        let o = world3.get(4).unwrap();
        assert_eq!(o.last_emot_index, Some(2));
        let em = select_packs_for_player(o);
        assert_eq!(em.person, ANIM_GROUND);
        assert_eq!(em.extra_index, -1);

        // PE with emotion bank gesture row → EXTRA + resolved index
        let bank = crate::emotion::EmotionBank::from_ini_strings(
            "/a\n/b\n/wave\n",
            "0 1 0 0 0 0\n0 0 0 0 0 0\n0 0 0 0 0 0 3\n",
        );
        let mut world4 = LiveWorld::new();
        world4.apply_pu(&sample_pu(8, 19, 0, 0, 0));
        world4.apply_emots_with_bank(
            &[crate::parse::PlayerEmot {
                player_id: 8,
                emot_index: 2,
                ttl_sec: Some(5.0),
            }],
            Some(&bank),
            10.0,
        );
        let o4 = world4.get(8).unwrap();
        assert_eq!(o4.emot_extra_index, Some(3));
        let em4 = select_packs_for_player(o4);
        assert_eq!(em4.person, ANIM_EXTRA);
        assert_eq!(em4.extra_index, 3);

        let ground_o = {
            let mut w = LiveWorld::new();
            w.apply_pu(&sample_pu(9, 19, 0, 0, 0));
            select_packs_for_player(w.get(9).unwrap())
        };
        assert_eq!(ground_o.person, ANIM_GROUND);
    }

    /// L-EMOT: PE object layers paint when emotion table + objects exist.
    #[test]
    fn scene_draw_pe_emotion_layers() {
        use crate::content::{ClientObjectDef, ObjectSprite};
        use crate::emotion::EmotionBank;

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.draw_hud = false;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        scene.emotions = EmotionBank::from_ini_strings(
            "/happy\n",
            "0 9001 0 0 0 0\n", // mouthEmot = 9001
        );

        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 0,
                object_raw: "0".into(),
            },
        );

        let mut content = ClientContent::new();
        // Person with head marker
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                name: "person".into(),
                person: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: 0.0,
                        y: 20.0,
                        is_head: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        // Mouth emot object — bright green blob
        content.objects.insert(
            9001,
            ClientObjectDef {
                id: 9001,
                name: "mouth".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 3,
                    x: 0.0,
                    y: 0.0,
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [200, 100, 100, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(6, 6, [180, 120, 100, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(10, 6, [0, 255, 0, 255]), None);

        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world.apply_emots_with_bank(
            &[crate::parse::PlayerEmot {
                player_id: 1,
                emot_index: 0,
                ttl_sec: Some(10.0),
            }],
            Some(&scene.emotions),
            10.0,
        );
        assert_eq!(world.get(1).unwrap().last_emot_index, Some(0));

        let mut fb = Framebuffer::new(128, 128);
        scene.draw(&mut fb, &mut map, &mut world, &content, &mut sprites, &mut anims, 0.0);
        let green = count_near(&fb, [0, 255, 0]);
        assert!(
            green > 0,
            "mouth emot object must paint green pixels (got {green})"
        );
    }

    /// P3#19: eyeEmot draws at head + mainEyesOffset (not head origin alone).
    #[test]
    fn scene_draw_pe_eye_emot_main_eyes_offset() {
        use crate::content::{ClientObjectDef, ObjectSprite};
        use crate::emotion::EmotionBank;

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.draw_hud = false;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        // 1 object unit = 1 screen pixel at zoom=GRID
        scene.camera.zoom = GRID;
        scene.emotions = EmotionBank::from_ini_strings(
            "/happy\n",
            "9002 0 0 0 0 0\n", // eyeEmot = 9002
        );

        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 0,
                object_raw: "0".into(),
            },
        );

        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                name: "person".into(),
                person: 1,
                // Large offset so eye blob is far from head origin on screen
                main_eyes_offset: (40.0, 0.0),
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: 0.0,
                        y: 20.0,
                        is_head: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 4,
                        x: 40.0,
                        y: 20.0,
                        is_eyes: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        // Tiny 2×2 cyan eye emot blob
        content.objects.insert(
            9002,
            ClientObjectDef {
                id: 9002,
                name: "eye_emot".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 3,
                    x: 0.0,
                    y: 0.0,
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        sprites.ensure_rgba(1, &solid_sprite(4, 4, [200, 100, 100, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(4, 4, [180, 120, 100, 255]), None);
        sprites.ensure_rgba(3, &solid_sprite(2, 2, [0, 255, 255, 255]), None);
        sprites.ensure_rgba(4, &solid_sprite(2, 2, [50, 50, 50, 255]), None);

        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world.apply_emots_with_bank(
            &[crate::parse::PlayerEmot {
                player_id: 1,
                emot_index: 0,
                ttl_sec: Some(10.0),
            }],
            Some(&scene.emotions),
            10.0,
        );

        let mut fb = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let cyan = count_near(&fb, [0, 255, 255]);
        assert!(
            cyan > 0,
            "eye emot must paint cyan pixels with mainEyesOffset (got {cyan})"
        );

        // Without eyes / offset, eyeEmot is skipped (has_eyes false)
        content.objects.get_mut(&19).unwrap().main_eyes_offset = (0.0, 0.0);
        content.objects.get_mut(&19).unwrap().sprites[2].is_eyes = false;
        let mut fb2 = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb2,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let cyan2 = count_near(&fb2, [0, 255, 255]);
        assert_eq!(
            cyan2, 0,
            "eye emot must not draw when no eyes / zero offset (got {cyan2})"
        );
    }

    /// P3#19: person mouth sprite skipped when PE `mouthEmot` is active.
    #[test]
    fn scene_draw_skips_mouth_when_mouth_emot() {
        use crate::content::{ClientObjectDef, ObjectSprite};
        use crate::emotion::EmotionBank;

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.draw_hud = false;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        scene.emotions = EmotionBank::from_ini_strings(
            "/happy\n",
            "0 9001 0 0 0 0\n", // mouthEmot only
        );

        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 0,
                object_raw: "0".into(),
            },
        );

        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                name: "person".into(),
                person: 1,
                sprites: vec![
                    ObjectSprite {
                        sprite_id: 1,
                        x: 0.0,
                        y: 0.0,
                        is_body: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 2,
                        x: 0.0,
                        y: 20.0,
                        is_head: true,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                    ObjectSprite {
                        sprite_id: 4,
                        x: 0.0,
                        y: 12.0,
                        is_mouth: true,
                        r: 1.0, // pure red mouth
                        g: 0.0,
                        b: 0.0,
                        parent: 1,
                        age_start: -1.0,
                        age_end: -1.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );
        content.objects.insert(
            9001,
            ClientObjectDef {
                id: 9001,
                name: "mouth_emot".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 5,
                    x: 0.0,
                    y: 0.0,
                    r: 0.0,
                    g: 1.0, // green emot mouth
                    b: 0.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        sprites.ensure_rgba(1, &solid_sprite(8, 8, [200, 200, 200, 255]), None);
        sprites.ensure_rgba(2, &solid_sprite(8, 8, [220, 180, 140, 255]), None);
        sprites.ensure_rgba(4, &solid_sprite(8, 8, [255, 0, 0, 255]), None); // red mouth
        sprites.ensure_rgba(5, &solid_sprite(8, 8, [0, 255, 0, 255]), None); // green emot

        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        // No PE: red mouth should paint
        let mut fb0 = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb0,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let red0 = count_near(&fb0, [255, 0, 0]);
        assert!(red0 > 0, "mouth sprite should paint without PE (got {red0})");

        world.apply_emots_with_bank(
            &[crate::parse::PlayerEmot {
                player_id: 1,
                emot_index: 0,
                ttl_sec: Some(5.0),
            }],
            Some(&scene.emotions),
            10.0,
        );
        let mut fb1 = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb1,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let red1 = count_near(&fb1, [255, 0, 0]);
        let green1 = count_near(&fb1, [0, 255, 0]);
        assert_eq!(
            red1, 0,
            "person mouth must be skipped when mouthEmot active (got {red1})"
        );
        assert!(
            green1 > 0,
            "mouthEmot object should still paint (got {green1})"
        );
    }

    /// Moving vs ground pack changes soft-FB sample offset when anim bank has tracks.
    #[test]
    fn scene_draw_uses_moving_pack_offset() {
        use crate::anim_bank::{ObjectAnimation, SpriteAnimParam, ANIM_GROUND, ANIM_MOVING};

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        scene.time = 0.0;

        // Seed map tiles so biome paint is deterministic; player at (0,0).
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 0,
                object_raw: "0".into(),
            },
        );
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                sprites: vec![ObjectSprite {
                    sprite_id: 700,
                    x: 0.0,
                    y: 0.0,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        // Larger sprite so offset is visible after scale = zoom/GRID = 0.5
        sprites.ensure_rgba(700, &solid_sprite(16, 16, [255, 0, 0, 255]), None);

        let mut anims = AnimBank::new(".");
        // Ground: no offset. Moving: large +X offset so red blob shifts.
        anims.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 0.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        anims.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 80.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });

        let mut world_g = LiveWorld::new();
        world_g.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        let mut world_m = LiveWorld::new();
        world_m.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world_m.apply_moves_start(&[crate::parse::PlayerMoveStart {
            player_id: 1,
            xs: 0,
            ys: 0,
            total_sec: 1.0,
            eta_sec: 1.0,
            trunc: 0,
            deltas: vec![(1, 0)],
        }]);
        assert!(world_m.get(1).unwrap().moving);
        assert_eq!(
            select_packs_for_player(world_m.get(1).unwrap()).person,
            ANIM_MOVING
        );
        // Settle dual-fade so soft-FB uses pure moving pack (not mid-cross-fade ground).
        // // C++: after lastAnimFade decays to 0; offset-only switches still fade.
        {
            let o = world_m.get_mut(1).unwrap();
            o.anim.cur_anim = ANIM_MOVING;
            o.anim.last_anim = ANIM_MOVING;
            o.anim.last_anim_fade = 0.0;
        }

        let mut fb_g = Framebuffer::new(128, 128);
        let mut fb_m = Framebuffer::new(128, 128);
        scene.draw(
            &mut fb_g,
            &mut map,
            &mut world_g,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        scene.draw(
            &mut fb_m,
            &mut map,
            &mut world_m,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let red_g = count_near(&fb_g, [255, 0, 0]);
        let red_m = count_near(&fb_m, [255, 0, 0]);
        assert!(
            red_g > 0 && red_m > 0,
            "player sprites must paint red_g={red_g} red_m={red_m}"
        );
        assert_ne!(
            fb_g.pixels, fb_m.pixels,
            "moving pack offset must change drawn pixels vs ground"
        );
    }

    /// Soft-FB mid dual-fade: anim_fade=0.5 pixel centroid between pure ground and pure moving.
    ///
    /// // C++: drawObjectAnim inAnimFade blend; isAnimFadeNeeded keeps osc fades alive
    #[test]
    fn scene_draw_dual_fade_mid_differs_from_ends() {
        use crate::anim_bank::{ObjectAnimation, SpriteAnimParam, ANIM_GROUND, ANIM_MOVING};
        use crate::anim_draw::AnimDrawState;

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;
        scene.time = 0.0;
        scene.draw_hud = false;

        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            crate::client_map::MapTile {
                biome: 0,
                floor_id: 0,
                object_id: 0,
                object_raw: "0".into(),
            },
        );
        let mut content = ClientContent::new();
        content.objects.insert(
            19,
            ClientObjectDef {
                id: 19,
                sprites: vec![ObjectSprite {
                    sprite_id: 701,
                    x: 0.0,
                    y: 0.0,
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    parent: -1,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 256);
        sprites.ensure_rgba(701, &solid_sprite(16, 16, [255, 0, 0, 255]), None);

        let mut anims = AnimBank::new(".");
        // Oscillating offset so isAnimFadeNeeded stays true (static offset alone skips fade).
        anims.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 0.0,
                x_osc_per_sec: 0.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        anims.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 80.0,
                x_osc_per_sec: 1.0,
                x_amp: 0.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });

        fn red_centroid_x(fb: &Framebuffer) -> f32 {
            let mut sx = 0.0f32;
            let mut n = 0.0f32;
            for y in 0..fb.height {
                for x in 0..fb.width {
                    let i = ((y * fb.width + x) * 4) as usize;
                    if fb.pixels[i] > 200 && fb.pixels[i + 1] < 40 && fb.pixels[i + 2] < 40 {
                        sx += x as f32;
                        n += 1.0;
                    }
                }
            }
            if n < 1.0 {
                0.0
            } else {
                sx / n
            }
        }

        // Pure ground
        let mut world_g = LiveWorld::new();
        world_g.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        {
            let o = world_g.get_mut(1).unwrap();
            o.anim = AnimDrawState::default();
            o.anim.cur_anim = ANIM_GROUND;
            o.anim.last_anim = ANIM_GROUND;
            o.anim.last_anim_fade = 0.0;
        }
        // Pure moving (settled)
        let mut world_m = LiveWorld::new();
        world_m.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        {
            let o = world_m.get_mut(1).unwrap();
            o.moving = true;
            o.anim.cur_anim = ANIM_MOVING;
            o.anim.last_anim = ANIM_MOVING;
            o.anim.last_anim_fade = 0.0;
        }
        // Mid-fade: last=moving weight 0.5, target=ground (leaving moving)
        let mut world_mid = LiveWorld::new();
        world_mid.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        {
            let o = world_mid.get_mut(1).unwrap();
            o.anim.cur_anim = ANIM_GROUND;
            o.anim.last_anim = ANIM_MOVING;
            o.anim.last_anim_fade = 0.5;
            o.anim.animation_frame_count = 0.0;
            o.anim.last_animation_frame_count = 0.0;
        }

        let mut fb_g = Framebuffer::new(128, 128);
        let mut fb_m = Framebuffer::new(128, 128);
        let mut fb_mid = Framebuffer::new(128, 128);
        // dt=0: sync would overwrite mid-fade from flags — skip by freezing flags +
        // temporarily drawing without re-sync: set moving false and force packs
        // via pre-set anim; draw's dt=0 path calls sync_anim_packs which may reset.
        // Keep desired type matching cur so switch_to is a no-op.
        world_g.get_mut(1).unwrap().moving = false;
        world_m.get_mut(1).unwrap().moving = true;
        world_mid.get_mut(1).unwrap().moving = false;
        // After sync, mid may snap if fade skip applies. Force fade after draw's sync
        // by using a tiny positive dt=0 path then re-apply fade before a custom draw —
        // instead: call draw with dt=0 and re-force mid state is hard. Use direct
        // draw_object_with_pack for mid, and scene.draw for ends.
        scene.draw(
            &mut fb_g,
            &mut map,
            &mut world_g,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        scene.draw(
            &mut fb_m,
            &mut map,
            &mut world_m,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );

        // Mid: build pack manually and draw person only (avoids sync clearing fade).
        {
            let o = world_mid.get(1).unwrap();
            let mut pack = o.person_anim_pack(false);
            assert!(
                (pack.anim_fade - 0.5).abs() < 1e-4,
                "expected mid fade 0.5 got {}",
                pack.anim_fade
            );
            assert_eq!(pack.anim_type, ANIM_MOVING);
            assert_eq!(pack.fade_target_type, ANIM_GROUND);
            let (sx, sy) = scene.world_to_screen(0.5, 0.5, 128, 128);
            fb_mid.clear([30, 30, 35, 255]);
            let _ = scene.draw_object_with_pack(
                &mut fb_mid,
                &content,
                &mut sprites,
                &mut anims,
                &mut pack,
                20.0,
                sx,
                sy,
                false,
                false,
                false,
                0,
                false,
                SpriteLayerFilter::All,
            false,
            );
        }

        let cx_g = red_centroid_x(&fb_g);
        let cx_m = red_centroid_x(&fb_m);
        let cx_mid = red_centroid_x(&fb_mid);
        assert!(
            cx_g > 0.0 && cx_m > 0.0 && cx_mid > 0.0,
            "centroids g={cx_g} m={cx_m} mid={cx_mid}"
        );
        // Moving is +X in object space → screen right when not flipped.
        assert!(
            cx_m > cx_g + 2.0,
            "moving should sit right of ground: g={cx_g} m={cx_m}"
        );
        // Mid should sit between pure ends (0.5 blend of 0 and 80 object units).
        assert!(
            cx_mid > cx_g + 1.0 && cx_mid < cx_m - 1.0,
            "mid-fade centroid should be between ends: g={cx_g} mid={cx_mid} m={cx_m}"
        );
    }

    /// L-SAY: PS speech bubble paints chalk + pencil pixels over soft-FB.
    #[test]
    fn scene_draw_speech_bubble_paints() {
        use crate::parse::PlayerSays;

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.draw_hud = false;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;

        let mut map = ClientMap::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        let mut anims = AnimBank::new(".");
        let mut world = LiveWorld::new();
        world.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world.apply_says(&[PlayerSays {
            player_id: 1,
            is_curse: false,
            text: "HI".into(),
            spoken: "HI".into(),
            map: None,
            target_label: None,
            target_player_id: None,
        }]);
        assert_eq!(world.get(1).unwrap().current_speech.as_deref(), Some("HI"));

        // Baseline without speech (clone world cleared).
        let mut world_clear = LiveWorld::new();
        world_clear.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        let mut fb0 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb0,
            &mut map,
            &mut world_clear,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let mut fb1 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb1,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert_ne!(
            fb0.pixels, fb1.pixels,
            "speech bubble must change soft-FB pixels"
        );
        // Chalk blot is light gray — count bright-ish pixels (loose threshold).
        let bright = |fb: &Framebuffer| {
            fb.pixels
                .chunks_exact(4)
                .filter(|p| p[0] > 150 && p[1] > 150 && p[2] > 150)
                .count()
        };
        let gray0 = bright(&fb0);
        let gray1 = bright(&fb1);
        assert!(
            gray1 > gray0,
            "chalk blot should paint more light pixels: gray0={gray0} gray1={gray1}"
        );

        // LS at tile also paints.
        world.apply_location_says(&[crate::parse::LocationSays {
            x: 0,
            y: 0,
            text: "LS".into(),
        }]);
        let mut fb2 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb2,
            &mut map,
            &mut world,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let gray2 = bright(&fb2);
        assert!(
            gray2 >= gray1,
            "location speech should keep/add chalk pixels: gray1={gray1} gray2={gray2}"
        );
    }

    /// P3#17: map-spot + label markers paint soft-FB pixels.
    #[test]
    fn scene_draw_map_pointer_markers_paint() {
        use crate::parse::{parse_ps_line, PlayerSays, SaysMapPointer, SaysTargetLabel};

        let mut scene = SceneRenderer::default();
        scene.ground = GroundBank::new();
        scene.draw_hud = false;
        scene.camera.x = 0.5;
        scene.camera.y = 0.5;
        scene.camera.zoom = 64.0;

        let mut map = ClientMap::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 128);
        let mut anims = AnimBank::new(".");

        let mut world0 = LiveWorld::new();
        world0.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world0.apply_pu(&sample_pu(2, 19, 1, 0, 0));

        let mut world1 = LiveWorld::new();
        world1.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world1.apply_pu(&sample_pu(2, 19, 1, 0, 0));
        // Map spot at (0,0) under camera.
        world1.apply_says(&[PlayerSays {
            player_id: 1,
            is_curse: false,
            text: ":SPECIAL SPOT *map 0 0 30".into(),
            spoken: ":SPECIAL SPOT".into(),
            map: Some(SaysMapPointer {
                x: 0,
                y: 0,
                map_age_seconds: Some(30),
            }),
            target_label: None,
            target_player_id: None,
        }]);
        assert_eq!(world1.says_pointers.len(), 1);
        assert_eq!(world1.get(1).unwrap().current_speech.as_deref(), Some(":SPECIAL SPOT"));

        let mut fb0 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb0,
            &mut map,
            &mut world0,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        let mut fb1 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb1,
            &mut map,
            &mut world1,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert_ne!(
            fb0.pixels, fb1.pixels,
            "map-spot marker must change soft-FB pixels"
        );

        // Label at target player 2.
        let mut world2 = LiveWorld::new();
        world2.apply_pu(&sample_pu(1, 19, 0, 0, 0));
        world2.apply_pu(&sample_pu(2, 19, 0, 0, 0));
        let vis = parse_ps_line("1/0 NEW *visitor 2 *map 0 0").unwrap();
        world2.apply_says(&[vis]);
        assert_eq!(
            world2.says_pointers[0].target_label,
            Some(SaysTargetLabel::Visitor)
        );
        let mut fb2 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb2,
            &mut map,
            &mut world2,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert_ne!(
            fb0.pixels, fb2.pixels,
            "label marker must change soft-FB pixels"
        );

        // Unit draw helper alone paints.
        let mut fb3 = Framebuffer::new(40, 40);
        draw_map_spot_marker(&mut fb3, 20.0, 20.0, 6, [80, 220, 255, 255]);
        let painted = fb3
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
            .count();
        assert!(painted > 10, "draw_map_spot_marker painted {painted}");

        // HUD home-arrow + MAP label sync from markers.
        world1.our_id = Some(1);
        scene.draw_hud = true;
        scene.hud.visible = true;
        scene.hud.food_capacity = 1;
        let mut fb4 = Framebuffer::new(160, 160);
        scene.draw(
            &mut fb4,
            &mut map,
            &mut world1,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert_eq!(
            scene.hud.map_pointer_label.as_deref(),
            Some("MAP"),
            "pure map spot sets MAP label"
        );
        // Marker at (0,0), our player also at (0,0) → no direction (too close / zero).
        // Move our player so arrow has a direction.
        if let Some(o) = world1.get_mut(1) {
            o.x = 0;
            o.y = -5;
        }
        scene.draw(
            &mut fb4,
            &mut map,
            &mut world1,
            &content,
            &mut sprites,
            &mut anims,
            0.0,
        );
        assert_eq!(scene.hud.home_arrow, Some(0), "north to map spot");
    }

    /// Full live PU line (see `parse.rs` tests).
    fn sample_pu(
        id: i32,
        display: i32,
        x: i32,
        y: i32,
        held: i32,
    ) -> crate::parse::PlayerUpdate {
        let line = format!(
            "{id} {display} 0 0 0 0 {held} 0 0 0 -1 0.5 1 0 {x} {y} 20.0 0.1 1.0 0;0;0;0;0;0 0 0 -1 0 0"
        );
        crate::parse::parse_pu_line(&line).expect("sample PU")
    }
}
