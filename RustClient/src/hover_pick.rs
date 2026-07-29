//! Soft-FB hitMap hover pick (L-ACT residual).
//!
//! Uses C++ `getSpriteHit` via [`SpriteBank::get_sprite_hit`] on map-object
//! rest poses (parent chain, center-anchor, flip/rot matching soft-FB blit).
//! Also hit-tests **worn clothing** sprites on our player (keys 1–6 MVP already
//! wires slots; this path selects the slot under the cursor for remove/drop).
//! Contained items in map containers / clothing containers expose
//! [`HoverPick::contained_slot`] (C++ `hitSlotIndex`) for REMV / SREMV `i`.
//! GUI sets [`crate::render::SceneRenderer::highlight_tile`] from the pick;
//! optional outline feedback distinguishes object vs empty ground vs clothing.

use crate::client_map::{parse_object_raw_contained, ClientMap};
use crate::content::{ClientContent, ClientObjectDef, ObjectSprite};
use crate::live_object::ClothingSet;
use crate::render::{Camera, Framebuffer, SceneRenderer, GRID};
use crate::sprite_bank::SpriteBank;

/// Result of mouse → object pick under the soft-FB camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverPick {
    /// Tile targeted for USE/DROP/REMV (object cell when hitMap hits overhang).
    pub tile: (i32, i32),
    /// Map object id under cursor (`0` = empty ground), clothing object id when
    /// [`Self::clothing_slot`] is set, or **contained** object id when
    /// [`Self::contained_slot`] >= 0 (C++ `mCurMouseOverID` on slot hit).
    pub object_id: i32,
    /// True when a sprite hitMap pixel confirmed the pick (vs tile occupancy only).
    pub hit_map: bool,
    /// Worn clothing slot `0..5` when cursor hits our clothing sprite; `-1` none.
    ///
    /// // C++: LivingLifePage clothing click via getSpriteHit on worn layers
    pub clothing_slot: i32,
    /// Container / clothing-container slot index under cursor (`-1` = none / top).
    ///
    /// // C++: `PointerHitRecord.hitSlotIndex` from `getClosestObjectPart`
    /// // Wire: REMV `i` / SREMV `i` / USE-on-contained `i` via [`crate::actions::encode_sremv`].
    pub contained_slot: i32,
}

impl Default for HoverPick {
    fn default() -> Self {
        Self {
            tile: (0, 0),
            object_id: 0,
            hit_map: false,
            clothing_slot: -1,
            contained_slot: -1,
        }
    }
}

impl HoverPick {
    pub fn empty(tile: (i32, i32)) -> Self {
        Self {
            tile,
            object_id: 0,
            hit_map: false,
            clothing_slot: -1,
            contained_slot: -1,
        }
    }

    pub fn is_object(&self) -> bool {
        self.object_id > 0
    }

    /// True when the cursor hit a worn clothing sprite (slot 0..5).
    pub fn is_clothing(&self) -> bool {
        self.clothing_slot >= 0 && self.clothing_slot <= 5
    }

    /// True when the cursor hit a contained item in a container / clothing bag.
    pub fn is_contained(&self) -> bool {
        self.contained_slot >= 0
    }

    /// Wire REMV/SREMV `i` from this soft-FB pick (`-1` when body / top of stack).
    ///
    /// // C++: `PointerHitRecord.hitSlotIndex` → encode_remv / encode_sremv
    pub fn hit_slot(&self) -> i32 {
        resolve_hit_slot(self.contained_slot, -1)
    }

    /// Soft-FB contained index if hit, else explicit map stack index (headless).
    pub fn hit_slot_or_stack(&self, map_stack_index: i32) -> i32 {
        resolve_hit_slot(self.contained_slot, map_stack_index)
    }
}

/// Resolve wire REMV / SREMV `i` (`hit_slot`) from soft-FB and/or map stack index.
///
/// Priority (C++ `hitSlotIndex` + headless stack pick):
/// 1. Soft-FB [`HoverPick::contained_slot`] when `>= 0` (sprite hit on a contained item)
/// 2. Explicit map container stack index when `>= 0` (headless / keyboard / no hitMap)
/// 3. `-1` = top of stack / container body (default when neither specifies a slot)
///
/// Pass result to [`crate::click_tile::click_remv`], [`crate::rmb_action::click_rmb_tile_ex`],
/// [`crate::click_tile::walk_or_use_tile_ex`], or `click_tile_mod_ex`.
///
/// // C++: LivingLifePage pointerDown uses hitSlotIndex for REMV i# / SREMV c i#
pub fn resolve_hit_slot(soft_fb_contained: i32, map_stack_index: i32) -> i32 {
    if soft_fb_contained >= 0 {
        soft_fb_contained
    } else if map_stack_index >= 0 {
        map_stack_index
    } else {
        -1
    }
}

/// Clamp a 0-based map container stack index to a valid wire `hit_slot`.
///
/// Empty stack or out-of-range index → `-1` (server “top of stack”).
/// In-range index is returned unchanged (same order as map `contained_ids()`).
pub fn map_stack_index_to_hit_slot(stack_index: i32, contained_count: usize) -> i32 {
    if stack_index < 0 || contained_count == 0 {
        return -1;
    }
    if (stack_index as usize) >= contained_count {
        return -1;
    }
    stack_index
}

/// Cells around the cursor tile searched for overhanging sprites.
const HIT_SCAN_RADIUS: i32 = 2;

/// Screen pixel → hover object via hitMap (preferred) or map tile object id.
///
/// // C++: LivingLifePage mouse + getSpriteHit on drawn object sprites
/// // Haxe: no full hitMap; tile pick only
pub fn pick_at_screen(
    camera: &Camera,
    map: &ClientMap,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    sx: f32,
    sy: f32,
    fb_w: u32,
    fb_h: u32,
) -> HoverPick {
    let z = camera.zoom.max(1e-4);
    let wx = (sx - fb_w as f32 * 0.5) / z + camera.x;
    let wy = camera.y - (sy - fb_h as f32 * 0.5) / z;
    let cursor_tile = (wx.floor() as i32, wy.floor() as i32);

    // Reverse draw order: higher world-y first so topmost visual wins.
    let mut cells: Vec<(i32, i32)> = Vec::new();
    for dy in -HIT_SCAN_RADIUS..=HIT_SCAN_RADIUS {
        for dx in -HIT_SCAN_RADIUS..=HIT_SCAN_RADIUS {
            cells.push((cursor_tile.0 + dx, cursor_tile.1 + dy));
        }
    }
    cells.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));

    for (tx, ty) in cells {
        let Some(tile) = map.get(tx, ty) else {
            continue;
        };
        let oid = tile.object_id;
        if oid <= 0 {
            continue;
        }
        let scale = (camera.zoom / GRID).max(0.05);
        let (screen_x, screen_y) =
            world_to_screen(camera, tx as f32 + 0.5, ty as f32 + 0.5, fb_w, fb_h);
        let contained = tile.contained_ids();
        // Contained first (drawn on top of container body in soft-FB).
        // // C++: getClosestObjectPart → hitSlotIndex on contained stacks
        if let Some((slot, cid)) = pick_contained_slot_at(
            content,
            sprites,
            oid,
            &contained,
            20.0,
            screen_x,
            screen_y,
            false,
            scale,
            sx,
            sy,
        ) {
            return HoverPick {
                tile: (tx, ty),
                object_id: cid,
                hit_map: true,
                clothing_slot: -1,
                contained_slot: slot,
            };
        }
        if object_hit_map_at(
            camera, content, sprites, oid, tx, ty, sx, sy, fb_w, fb_h,
        ) {
            return HoverPick {
                tile: (tx, ty),
                object_id: oid,
                hit_map: true,
                clothing_slot: -1,
                contained_slot: -1,
            };
        }
    }

    // Tile occupancy fallback (no hitMap / sprites not loaded yet).
    let oid = map
        .get(cursor_tile.0, cursor_tile.1)
        .map(|t| t.object_id)
        .unwrap_or(0);
    HoverPick {
        tile: cursor_tile,
        object_id: oid.max(0),
        hit_map: false,
        clothing_slot: -1,
        contained_slot: -1,
    }
}

/// Input for worn-clothing soft-FB hitMap pick (own player).
///
/// // C++: LivingLifePage draws ClothingSet then hit-tests worn sprites for
/// // clothing slot c in DROP/SELF/SREMV (hat=0 … backpack=5).
#[derive(Debug, Clone, Copy)]
pub struct WornClothingPickTarget<'a> {
    pub tile_x: i32,
    pub tile_y: i32,
    pub facing: i32,
    pub age: f32,
    pub clothing: &'a ClothingSet,
}

/// Reverse of soft-FB clothing draw order so topmost layer wins hit test.
/// Draw: backShoe, bottom, tunic, backpack, frontShoe, hat → hit: hat first.
const CLOTHING_HIT_ORDER: [usize; 6] = [0, 2, 5, 1, 4, 3];

/// Hit-test worn clothing sprites; returns `(slot 0..5, clothing_object_id, contained_slot)`.
///
/// `contained_slot` is `-1` when the outer clothing body was hit (or empty bag);
/// `>= 0` when a contained item sprite under that clothing slot was hit.
///
/// Rest-pose parent chain + center-anchor + flip/rot match soft-FB clothing blit
/// (`clothing_offset`, `worn=true` skips `invisWorn`). No anim sample (hover lite).
///
/// // C++: getSpriteHit on clothing object layers while over own LiveObject
/// // + getClosestObjectPart on clothingContained stacks → hitSlotIndex for SREMV
pub fn pick_worn_clothing_slot(
    camera: &Camera,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    target: &WornClothingPickTarget<'_>,
    sx: f32,
    sy: f32,
    fb_w: u32,
    fb_h: u32,
) -> Option<(i32, i32, i32)> {
    let flip = target.facing < 0;
    let scale = (camera.zoom / GRID).max(0.05);
    let (person_sx, person_sy) = world_to_screen(
        camera,
        target.tile_x as f32 + 0.5,
        target.tile_y as f32 + 0.5,
        fb_w,
        fb_h,
    );

    for &slot in &CLOTHING_HIT_ORDER {
        let cloth_id = target.clothing.slot_id(slot);
        if cloth_id <= 0 {
            continue;
        }
        let (ox, oy) = content
            .get(cloth_id)
            .map(|d| d.clothing_offset)
            .unwrap_or((0.0, 0.0));
        let cx = person_sx + ox * scale * if flip { -1.0 } else { 1.0 };
        let cy = person_sy - oy * scale;

        // Contained items in worn bag (quiver arrows, backpack contents).
        let contained: Vec<i32> = target
            .clothing
            .slots
            .get(slot)
            .map(|raw| {
                parse_object_raw_contained(raw)
                    .into_iter()
                    .map(|n| n.id)
                    .collect()
            })
            .unwrap_or_default();
        if let Some((cslot, cid)) = pick_contained_slot_at(
            content,
            sprites,
            cloth_id,
            &contained,
            target.age,
            cx,
            cy,
            flip,
            scale,
            sx,
            sy,
        ) {
            return Some((slot as i32, cid, cslot));
        }

        if clothing_object_hit_map_at(
            content,
            sprites,
            cloth_id,
            target.age,
            cx,
            cy,
            flip,
            scale,
            sx,
            sy,
        ) {
            return Some((slot as i32, cloth_id, -1));
        }
    }
    None
}

/// Map pick then worn clothing (clothing wins when hitMap confirms a worn sprite).
///
/// Prefer clothing so hat/backpack clicks remove/equip even when a map object
/// sits under the same screen pixel.
pub fn pick_at_screen_with_clothing(
    camera: &Camera,
    map: &ClientMap,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    worn: Option<&WornClothingPickTarget<'_>>,
    sx: f32,
    sy: f32,
    fb_w: u32,
    fb_h: u32,
) -> HoverPick {
    if let Some(w) = worn {
        if let Some((slot, oid, cslot)) =
            pick_worn_clothing_slot(camera, content, sprites, w, sx, sy, fb_w, fb_h)
        {
            return HoverPick {
                tile: (w.tile_x, w.tile_y),
                object_id: oid,
                hit_map: true,
                clothing_slot: slot,
                contained_slot: cslot,
            };
        }
    }
    pick_at_screen(camera, map, content, sprites, sx, sy, fb_w, fb_h)
}

/// Convenience: pick + write [`SceneRenderer::highlight_tile`].
pub fn update_scene_hover(
    scene: &mut SceneRenderer,
    map: &ClientMap,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    sx: f32,
    sy: f32,
    fb_w: u32,
    fb_h: u32,
) -> HoverPick {
    update_scene_hover_with_clothing(scene, map, content, sprites, None, sx, sy, fb_w, fb_h)
}

/// Like [`update_scene_hover`] but also hit-tests worn clothing when `worn` is set.
pub fn update_scene_hover_with_clothing(
    scene: &mut SceneRenderer,
    map: &ClientMap,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    worn: Option<&WornClothingPickTarget<'_>>,
    sx: f32,
    sy: f32,
    fb_w: u32,
    fb_h: u32,
) -> HoverPick {
    let pick = pick_at_screen_with_clothing(
        &scene.camera,
        map,
        content,
        sprites,
        worn,
        sx,
        sy,
        fb_w,
        fb_h,
    );
    scene.highlight_tile = Some(pick.tile);
    pick
}

/// Draw tile outline: cyan object / yellow empty / magenta clothing slot.
///
/// Contained hits use a warmer orange ring so slot-targeted REMV/SREMV is visible.
///
/// Call after [`SceneRenderer::draw`] so the outline sits on top of the world
/// (and under HUD if you draw HUD after this — client draws scene then this).
pub fn draw_hover_outline(
    fb: &mut Framebuffer,
    camera: &Camera,
    pick: HoverPick,
) {
    let (x0, y0, tile_w, tile_h) = crate::render::tile_screen_rect(
        camera,
        pick.tile.0,
        pick.tile.1,
        fb.width,
        fb.height,
    );
    // Contained: orange; clothing: magenta; object hitMap: bright cyan; object tile: cyan; empty: yellow.
    let c = if pick.is_contained() {
        [255, 180, 60, 230]
    } else if pick.is_clothing() {
        [255, 120, 255, 230]
    } else if pick.hit_map {
        [80, 255, 255, 220]
    } else if pick.object_id > 0 {
        [100, 220, 255, 200]
    } else {
        [255, 255, 100, 180]
    };
    // Outer ring (object/clothing/contained gets thicker feedback).
    let thick = if pick.object_id > 0 || pick.is_clothing() || pick.is_contained() {
        3
    } else {
        2
    };
    for t in 0..thick {
        let inset = t;
        let w = tile_w - inset * 2;
        let h = tile_h - inset * 2;
        if w <= 0 || h <= 0 {
            break;
        }
        let x = x0 + inset;
        let y = y0 + inset;
        fb.fill_rect(x, y, w, 1, c);
        fb.fill_rect(x, y + h - 1, w, 1, c);
        fb.fill_rect(x, y, 1, h, c);
        fb.fill_rect(x + w - 1, y, 1, h, c);
    }
}

fn world_to_screen(camera: &Camera, wx: f32, wy: f32, fb_w: u32, fb_h: u32) -> (f32, f32) {
    let sx = (wx - camera.x) * camera.zoom + fb_w as f32 * 0.5;
    let sy = (camera.y - wy) * camera.zoom + fb_h as f32 * 0.5;
    (sx, sy)
}

/// Hit-test top-level contained items at container `slot_pos` offsets.
///
/// Returns `(slot_index, contained_object_id)`. Later indices are tested first
/// (drawn later / on top in soft-FB).
///
/// // C++: getClosestObjectPart + contained stacks → hitSlotIndex
fn pick_contained_slot_at(
    content: &ClientContent,
    sprites: &mut SpriteBank,
    container_id: i32,
    contained_ids: &[i32],
    age: f32,
    screen_x: f32,
    screen_y: f32,
    flip: bool,
    scale: f32,
    mx: f32,
    my: f32,
) -> Option<(i32, i32)> {
    if contained_ids.is_empty() {
        return None;
    }
    let slots = content
        .get(container_id)
        .map(|d| d.slot_pos.clone())
        .unwrap_or_default();
    // Reverse: last drawn contained is on top.
    for (i, &cid) in contained_ids.iter().enumerate().rev() {
        if cid <= 0 {
            continue;
        }
        let (ox, oy) = slots
            .get(i)
            .copied()
            .unwrap_or((0.0, (i as f32) * 8.0));
        let cx = screen_x + ox * scale * if flip { -1.0 } else { 1.0 };
        let cy = screen_y - oy * scale;
        if object_sprites_hit_at(
            content, sprites, cid, age, cx, cy, flip, scale, false, mx, my,
        ) {
            return Some((i as i32, cid));
        }
    }
    None
}

/// True if any sprite of `object_id` at tile `(tx,ty)` hits the screen pixel.
fn object_hit_map_at(
    camera: &Camera,
    content: &ClientContent,
    sprites: &mut SpriteBank,
    object_id: i32,
    tx: i32,
    ty: i32,
    mx: f32,
    my: f32,
    fb_w: u32,
    fb_h: u32,
) -> bool {
    let scale = (camera.zoom / GRID).max(0.05);
    let (screen_x, screen_y) =
        world_to_screen(camera, tx as f32 + 0.5, ty as f32 + 0.5, fb_w, fb_h);
    // map objects use adult-ish age ranges in draw path
    object_sprites_hit_at(
        content, sprites, object_id, 20.0, screen_x, screen_y, false, scale, false, mx, my,
    )
}

/// Worn clothing object at screen center `(screen_x, screen_y)` (person + offset applied).
fn clothing_object_hit_map_at(
    content: &ClientContent,
    sprites: &mut SpriteBank,
    object_id: i32,
    age: f32,
    screen_x: f32,
    screen_y: f32,
    flip: bool,
    scale: f32,
    mx: f32,
    my: f32,
) -> bool {
    // worn=true → skip invisWorn layers (same as soft-FB draw)
    object_sprites_hit_at(
        content, sprites, object_id, age, screen_x, screen_y, flip, scale, true, mx, my,
    )
}

/// Shared rest-pose hitMap sample for map objects and worn clothing.
fn object_sprites_hit_at(
    content: &ClientContent,
    sprites: &mut SpriteBank,
    object_id: i32,
    age: f32,
    screen_x: f32,
    screen_y: f32,
    flip: bool,
    scale: f32,
    worn: bool,
    mx: f32,
    my: f32,
) -> bool {
    let Some(def) = content.get(object_id) else {
        return false;
    };
    if def.sprites.is_empty() {
        return false;
    }

    let (ox, oy, orot, posed) = rest_sprite_poses(def);

    for (si, spr) in def.sprites.iter().enumerate() {
        if !posed[si] || !spr.visible_at_age(age) {
            continue;
        }
        // P4#25: multi-use stages hide sprites (C++ spriteSkipDrawing)
        if spr.skip_drawing {
            continue;
        }
        if worn && spr.invis_worn {
            continue;
        }
        // Ensure pixels + hitMap are loaded.
        let Some(rect) = sprites.ensure(spr.sprite_id) else {
            continue;
        };
        // Match render.rs: geometric = (pos.x - ax, pos.y + ay) in Y-up object space.
        let ax = rect.center_anchor_x as f32;
        let ay = rect.center_anchor_y as f32;
        let px = ox[si] - ax;
        let py = oy[si] + ay;
        let dx = screen_x + px * scale * if flip { -1.0 } else { 1.0 };
        let dy = screen_y - py * scale;
        let mut h_flip = spr.h_flip ^ flip;
        if rect.no_flip {
            h_flip = spr.h_flip;
        }
        let mut rot = orot[si];
        if flip {
            rot = -rot;
        }
        if sprite_hit_at_screen(
            sprites,
            spr.sprite_id,
            rect.width,
            rect.height,
            dx,
            dy,
            scale,
            h_flip,
            rot,
            mx,
            my,
        ) {
            return true;
        }
    }
    false
}

/// Inverse of soft-FB `blit_sprite` center placement → local hitMap sample.
fn sprite_hit_at_screen(
    sprites: &SpriteBank,
    sprite_id: i32,
    src_w: u32,
    src_h: u32,
    dst_cx: f32,
    dst_cy: f32,
    scale: f32,
    h_flip: bool,
    rot_turns: f32,
    mx: f32,
    my: f32,
) -> bool {
    if src_w == 0 || src_h == 0 || scale <= 1e-6 {
        return false;
    }
    let dw = (src_w as f32 * scale).max(1.0);
    let dh = (src_h as f32 * scale).max(1.0);
    let ox = mx - dst_cx;
    let oy = my - dst_cy;

    let (u, v) = if rot_turns.abs() < 1e-5 {
        // Axis-aligned: same as fast blit path (integer top-left).
        let dwi = dw as i32;
        let dhi = dh as i32;
        let top_x = dst_cx as i32 - dwi / 2;
        let top_y = dst_cy as i32 - dhi / 2;
        let dx = mx as i32 - top_x;
        let dy = my as i32 - top_y;
        if dx < 0 || dy < 0 || dx >= dwi || dy >= dhi {
            return false;
        }
        let u = if h_flip {
            src_w as i32 - 1 - (dx * src_w as i32 / dwi.max(1))
        } else {
            dx * src_w as i32 / dwi.max(1)
        };
        let v = dy * src_h as i32 / dhi.max(1);
        (u, v)
    } else {
        let angle = rot_turns * std::f32::consts::TAU;
        let (s, c) = angle.sin_cos();
        let inv_s = -s;
        let inv_c = c;
        let hw = dw * 0.5;
        let hh = dh * 0.5;
        // Inverse rotate screen offset into local sprite space.
        let lx = ox * inv_c - oy * inv_s;
        let ly = ox * inv_s + oy * inv_c;
        if lx < -hw || lx >= hw || ly < -hh || ly >= hh {
            return false;
        }
        let mut u_f = (lx + hw) / dw * src_w as f32;
        let v_f = (ly + hh) / dh * src_h as f32;
        if h_flip {
            u_f = src_w as f32 - 1.0 - u_f;
        }
        (u_f.floor() as i32, v_f.floor() as i32)
    };

    sprites.get_sprite_hit(sprite_id, u, v)
}

/// Rest poses + Jason parent chain (no anim sample — hover lite).
fn rest_sprite_poses(def: &ClientObjectDef) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<bool>) {
    let n = def.sprites.len();
    let mut ox = vec![0.0f32; n];
    let mut oy = vec![0.0f32; n];
    let mut orot = vec![0.0f32; n];
    let mut posed = vec![false; n];
    for (si, spr) in def.sprites.iter().enumerate() {
        ox[si] = spr.x;
        oy[si] = spr.y;
        orot[si] = spr.rot;
        posed[si] = true;
    }
    // Identity deltas → Jason walk-up is a no-op; keep algorithm parity with render.rs.
    apply_jason_parent_chain_hover(&def.sprites, &mut ox, &mut oy, &mut orot);
    (ox, oy, orot, posed)
}

/// Same algorithm as `render::apply_jason_parent_chain` (animationBank.cpp ~2505–2625).
fn apply_jason_parent_chain_hover(
    sprites: &[ObjectSprite],
    ox: &mut [f32],
    oy: &mut [f32],
    orot: &mut [f32],
) {
    let n = sprites.len();
    if n == 0 {
        return;
    }
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
                let angle = -pdrot * std::f32::consts::TAU;
                rot += pdrot;
                let (s, c) = (-angle).sin_cos();
                sx += c * dx[p] - s * dy[p];
                sy += s * dx[p] + c * dy[p];
                let cox = sx - sprites[p].x;
                let coy = sy - sprites[p].y;
                let (s2, c2) = angle.sin_cos();
                let nox = c2 * cox - s2 * coy;
                let noy = s2 * cox + c2 * coy;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_map::MapTile;
    use crate::content::{ClientObjectDef, ObjectSprite};
    use crate::tga::RgbaImage;

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
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
    fn empty_tile_pick() {
        let cam = Camera {
            x: 5.0,
            y: 5.0,
            zoom: 32.0,
        };
        let map = ClientMap::new();
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let pick = pick_at_screen(&cam, &map, &content, &mut sprites, 160.0, 90.0, 320, 180);
        assert_eq!(pick.object_id, 0);
        assert!(!pick.hit_map);
        assert_eq!(pick.contained_slot, -1);
    }

    #[test]
    fn tile_fallback_object_without_sprites() {
        let cam = Camera {
            x: 2.5,
            y: 2.5,
            zoom: 32.0,
        };
        let mut map = ClientMap::new();
        map.set(
            2,
            2,
            MapTile {
                object_id: 77,
                object_raw: "77".into(),
                ..Default::default()
            },
        );
        // No object def → no hitMap; still report tile occupancy.
        let content = ClientContent::new();
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        // Screen center is camera → tile (2,2)
        let pick = pick_at_screen(&cam, &map, &content, &mut sprites, 160.0, 90.0, 320, 180);
        assert_eq!(pick.tile, (2, 2));
        assert_eq!(pick.object_id, 77);
        assert!(!pick.hit_map);
        assert_eq!(pick.contained_slot, -1);
    }

    #[test]
    fn hit_map_confirms_opaque_sprite() {
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 64.0, // 64 px/tile; scale = 64/128 = 0.5
        };
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            MapTile {
                object_id: 50,
                object_raw: "50".into(),
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let img = solid(16, 16, [255, 0, 0, 255]);
        let rect = sprites.ensure_rgba(9001, &img, None).unwrap();
        // Align object-space pos with center-anchor so blit center = tile center.
        let mut content = ClientContent::new();
        content.objects.insert(
            50,
            ClientObjectDef {
                id: 50,
                name: "test".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 9001,
                    x: rect.center_anchor_x as f32,
                    y: rect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let (sx, sy) = world_to_screen(&cam, 0.5, 0.5, 128, 128);
        let pick = pick_at_screen(&cam, &map, &content, &mut sprites, sx, sy, 128, 128);
        assert_eq!(pick.object_id, 50);
        assert!(pick.hit_map, "expected hitMap confirm at sprite center");
        assert_eq!(pick.contained_slot, -1);

        // Far corner of FB should miss hitMap; different tile → empty.
        let miss = pick_at_screen(&cam, &map, &content, &mut sprites, 2.0, 2.0, 128, 128);
        assert_eq!(miss.object_id, 0);
        assert!(!miss.hit_map);
    }

    #[test]
    fn contained_slot_hit_on_map_container() {
        // Basket at (0,0) with stone in slot 0; hit only the contained sprite.
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 64.0,
        };
        let mut map = ClientMap::new();
        map.set(
            0,
            0,
            MapTile {
                object_id: 125,
                object_raw: "125,33".into(),
                ..Default::default()
            },
        );
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let basket_img = solid(8, 8, [0, 255, 0, 255]);
        let stone_img = solid(16, 16, [255, 0, 0, 255]);
        let brect = sprites.ensure_rgba(8001, &basket_img, None).unwrap();
        let srect = sprites.ensure_rgba(8002, &stone_img, None).unwrap();
        let mut content = ClientContent::new();
        content.objects.insert(
            125,
            ClientObjectDef {
                id: 125,
                name: "basket".into(),
                num_slots: 1,
                // Offset contained so it is away from basket body center.
                slot_pos: vec![(40.0, 0.0)],
                sprites: vec![ObjectSprite {
                    sprite_id: 8001,
                    x: brect.center_anchor_x as f32,
                    y: brect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            33,
            ClientObjectDef {
                id: 33,
                name: "stone".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 8002,
                    x: srect.center_anchor_x as f32,
                    y: srect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let scale = (cam.zoom / GRID).max(0.05);
        let (base_sx, base_sy) = world_to_screen(&cam, 0.5, 0.5, 128, 128);
        // Cursor over contained slot offset (object-space * scale).
        let mx = base_sx + 40.0 * scale;
        let my = base_sy;
        let pick = pick_at_screen(&cam, &map, &content, &mut sprites, mx, my, 128, 128);
        assert!(pick.hit_map);
        assert_eq!(pick.contained_slot, 0, "expected contained slot 0");
        assert_eq!(pick.object_id, 33, "hover id is contained object");
        assert_eq!(pick.tile, (0, 0));
    }

    #[test]
    fn outline_draws_on_fb() {
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 32.0,
        };
        let mut fb = Framebuffer::new(64, 64);
        fb.clear([0, 0, 0, 255]);
        draw_hover_outline(
            &mut fb,
            &cam,
            HoverPick {
                tile: (0, 0),
                object_id: 1,
                hit_map: true,
                clothing_slot: -1,
                contained_slot: -1,
            },
        );
        assert!(fb.count_non_color([0, 0, 0, 255]) > 0);
    }

    #[test]
    fn worn_clothing_hit_map_picks_slot() {
        // Soft-FB clothing at person tile (0,0); opaque hat sprite at center.
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 64.0,
        };
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let img = solid(16, 16, [0, 0, 255, 255]);
        let rect = sprites.ensure_rgba(9101, &img, None).unwrap();
        let mut content = ClientContent::new();
        content.objects.insert(
            201,
            ClientObjectDef {
                id: 201,
                name: "test hat".into(),
                clothing: 'h',
                clothing_offset: (0.0, 0.0),
                sprites: vec![ObjectSprite {
                    sprite_id: 9101,
                    x: rect.center_anchor_x as f32,
                    y: rect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // slots: hat;tunic;front_shoe;back_shoe;bottom;backpack
        let clothing = ClothingSet::parse("201;0;0;0;0;0");
        let target = WornClothingPickTarget {
            tile_x: 0,
            tile_y: 0,
            facing: 0,
            age: 20.0,
            clothing: &clothing,
        };
        let (sx, sy) = world_to_screen(&cam, 0.5, 0.5, 128, 128);
        let hit = pick_worn_clothing_slot(&cam, &content, &mut sprites, &target, sx, sy, 128, 128);
        assert_eq!(hit, Some((0, 201, -1)), "hat slot under cursor");

        let map = ClientMap::new();
        let pick = pick_at_screen_with_clothing(
            &cam,
            &map,
            &content,
            &mut sprites,
            Some(&target),
            sx,
            sy,
            128,
            128,
        );
        assert!(pick.is_clothing());
        assert_eq!(pick.clothing_slot, 0);
        assert_eq!(pick.object_id, 201);
        assert!(pick.hit_map);
        assert_eq!(pick.tile, (0, 0));
        assert_eq!(pick.contained_slot, -1);

        // Miss far from person → no clothing slot.
        let miss = pick_worn_clothing_slot(
            &cam,
            &content,
            &mut sprites,
            &target,
            2.0,
            2.0,
            128,
            128,
        );
        assert!(miss.is_none());
    }

    #[test]
    fn worn_clothing_contained_slot_for_sremv() {
        // Backpack with one contained item at offset slot → contained_slot=0.
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 64.0,
        };
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let bag_img = solid(8, 8, [0, 255, 0, 255]);
        let arrow_img = solid(16, 16, [255, 0, 0, 255]);
        let brect = sprites.ensure_rgba(9301, &bag_img, None).unwrap();
        let arect = sprites.ensure_rgba(9302, &arrow_img, None).unwrap();
        let mut content = ClientContent::new();
        content.objects.insert(
            500,
            ClientObjectDef {
                id: 500,
                name: "pack".into(),
                clothing: 'p',
                clothing_offset: (0.0, 0.0),
                num_slots: 1,
                slot_pos: vec![(40.0, 0.0)],
                sprites: vec![ObjectSprite {
                    sprite_id: 9301,
                    x: brect.center_anchor_x as f32,
                    y: brect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        content.objects.insert(
            70,
            ClientObjectDef {
                id: 70,
                name: "arrow".into(),
                sprites: vec![ObjectSprite {
                    sprite_id: 9302,
                    x: arect.center_anchor_x as f32,
                    y: arect.center_anchor_y as f32,
                    age_start: -1.0,
                    age_end: -1.0,
                    parent: -1,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        // backpack in slot 5 with contained 70
        let clothing = ClothingSet::parse("0;0;0;0;0;500,70");
        let target = WornClothingPickTarget {
            tile_x: 0,
            tile_y: 0,
            facing: 0,
            age: 20.0,
            clothing: &clothing,
        };
        let scale = (cam.zoom / GRID).max(0.05);
        let (base_sx, base_sy) = world_to_screen(&cam, 0.5, 0.5, 128, 128);
        let mx = base_sx + 40.0 * scale;
        let my = base_sy;
        let hit = pick_worn_clothing_slot(&cam, &content, &mut sprites, &target, mx, my, 128, 128);
        assert_eq!(
            hit,
            Some((5, 70, 0)),
            "backpack clothing + contained slot 0 for SREMV i"
        );
    }

    #[test]
    fn clothing_hit_order_prefers_hat_over_tunic() {
        let cam = Camera {
            x: 0.5,
            y: 0.5,
            zoom: 64.0,
        };
        let mut sprites = SpriteBank::with_atlas_size(".", 64);
        let img = solid(16, 16, [255, 0, 0, 255]);
        let rect_h = sprites.ensure_rgba(9201, &img, None).unwrap();
        let rect_t = sprites.ensure_rgba(9202, &img, None).unwrap();
        let mut content = ClientContent::new();
        for (id, sid, rect, ch) in [
            (301, 9201, rect_h, 'h'),
            (302, 9202, rect_t, 't'),
        ] {
            content.objects.insert(
                id,
                ClientObjectDef {
                    id,
                    name: format!("c{id}"),
                    clothing: ch,
                    sprites: vec![ObjectSprite {
                        sprite_id: sid,
                        x: rect.center_anchor_x as f32,
                        y: rect.center_anchor_y as f32,
                        age_start: -1.0,
                        age_end: -1.0,
                        parent: -1,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );
        }
        let clothing = ClothingSet::parse("301;302;0;0;0;0");
        let target = WornClothingPickTarget {
            tile_x: 0,
            tile_y: 0,
            facing: 0,
            age: 20.0,
            clothing: &clothing,
        };
        let (sx, sy) = world_to_screen(&cam, 0.5, 0.5, 128, 128);
        let hit = pick_worn_clothing_slot(&cam, &content, &mut sprites, &target, sx, sy, 128, 128);
        // Both overlap center; hat (slot 0) is first in CLOTHING_HIT_ORDER.
        assert_eq!(hit, Some((0, 301, -1)));
    }

    #[test]
    fn resolve_hit_slot_soft_fb_or_map_stack() {
        // Soft-FB contained wins over stack index.
        assert_eq!(resolve_hit_slot(2, 0), 2);
        assert_eq!(resolve_hit_slot(0, 5), 0);
        // No soft-FB → map stack index.
        assert_eq!(resolve_hit_slot(-1, 3), 3);
        assert_eq!(resolve_hit_slot(-1, 0), 0);
        // Neither → top of stack.
        assert_eq!(resolve_hit_slot(-1, -1), -1);
        assert_eq!(resolve_hit_slot(-5, -2), -1);
    }

    #[test]
    fn map_stack_index_to_hit_slot_clamps() {
        assert_eq!(map_stack_index_to_hit_slot(0, 3), 0);
        assert_eq!(map_stack_index_to_hit_slot(2, 3), 2);
        assert_eq!(map_stack_index_to_hit_slot(3, 3), -1); // out of range
        assert_eq!(map_stack_index_to_hit_slot(0, 0), -1); // empty
        assert_eq!(map_stack_index_to_hit_slot(-1, 2), -1);
    }

    #[test]
    fn hover_pick_hit_slot_or_stack() {
        let mut p = HoverPick::empty((1, 2));
        assert_eq!(p.hit_slot(), -1);
        assert_eq!(p.hit_slot_or_stack(1), 1);
        p.contained_slot = 0;
        assert_eq!(p.hit_slot(), 0);
        // Soft-FB wins even when stack override provided.
        assert_eq!(p.hit_slot_or_stack(4), 0);
    }
}
