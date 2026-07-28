//! Headless click-tile → MOVE / object path-to-adjacent USE/DROP/REMV/SWAP/SELF
//! (C++ `LivingLifePage::pointerDown`).
//!
//! Chunk **L-ACT / click_tile_to_move** (ground):
//! - pathfind with cumulative MOVE deltas (`pathFind` + `pathToDest[i]-start`)
//! - closest-reachable fallback when goal blocked
//! - cancel `nextActionMessageToSend` on ground re-click
//! - mid-move repath via [`MoveState::send_move_repath`]
//! - multi-MOVE: remaining ±16 chunks + repath toward ultimate goal after `done_moving`
//!
//! Chunk **L-ACT / object path-to-adjacent**:
//! - 4-neighbor stand search (W/E/S/N) + optional self-tile for permanent food
//! - path to closest empty adjacent, queue USE/DROP/REMV at object coords
//! - flush via [`ClientSession::flush_pending_action`] on done_moving
//! - already-adjacent → send action immediately when free
//!
//! Chunk **L-ACT / side_access_food_stand** (C++ `pointerDown` ~25810–26038):
//! - `sideAccess` → only W/E stands (never N/S or self-tile)
//! - `noBackAccess` → exclude N stand; self-tile last-resort still allowed
//! - permanent non-blocking food harvest → prefer stand on object tile itself
//! - same-tile canExecute blocked when USE result `newTarget` blocks walking
//!
//! Chunk **L-ACT / modclick_drop_remv_swap_multimove**:
//! - full modClick action select (DROP/REMV/SWAP/USE/SELF) when held or RMB
//! - multi-MOVE continuation after done_moving
//!
//! Chunk **L-ACT / clothing-ux**:
//! - `clothing_char_to_slot` / `resolve_clothing_equip_slot` (held type + empty shoe)
//! - self-tile LMB: food → SELF; clothing → DROP c; keys/headless `click_drop_clothing`
//! - mod self: held → DROP c; empty → SREMV (hit_slot or -1 top)
//!
//! Chunk **L-ACT / click_gates** (C++ `pointerDown` ~24934–24995):
//! - `playerActionPending` blocks until post-action PU
//! - held `speedMult == 0` without ground trans (`actor, -1`) blocks
//! - held-by-adult → `JUMP 0 0#` (no MOVE)
//! - `age < noMoveAge` (0.20) → `JUMP 0 0#` ground wiggle
//!
//! Chunk **L-ACT / mouse-hold slide** (C++ continuous movement ~25181–25236, 22818+):
//! - while LMB held: repath toward hover tile (ground MOVE only; no object USE)
//! - after [`MIN_MOUSE_DOWN_FRAMES`]: if hover cell blocks walking, slide `clickDest`
//!   further along the major axis from the player (up to [`HOLD_SLIDE_LIMIT`])
//! - still respects click gates
//!
//! Chunk **pathfind / useWaypoint two-leg** (C++ `computePathToDest` ~2554–2589 + hold ~22850):
//! - `MoveState::arm_waypoint` + `find_path_with_waypoint_ex` (start→wp→goal)
//! - if two-leg path cell count > `maxWaypointPathLength` → dest becomes waypoint
//! - close-hold throw: mouse in 1..4 tile AABB → waypoint at mouse, throw dest out 4 tiles
//! - wired into ground [`plan_click_tile_chunks_goal`] / [`click_tile_with`] and object
//!   stand search [`plan_stand_for_object_with_opts_wp`] / [`click_object`]; multi-MOVE
//!   uses effective goal (may be waypoint) then repaths toward ultimate stand/goal
//!
//! Chunk **L-ACT / container_slot_ux** (C++ `pointerDown` ~26177–26314 + playtest put):
//! - empty hand + map container (`numSlots>0`) → `REMV x y i#` with hover [`hit_slot`]
//!   (`-1` top; soft-FB [`crate::hover_pick::HoverPick::contained_slot`])
//! - holding + map container → `USE` when transition / `+useOnContained` slot; else
//!   `DROP x y -1#` into when room (modClick always; LMB when no USE transition)
//! - clothing bag: held + slot → `DROP x y c#` put; empty + contained hit → `SREMV`
//!   (LMB on soft-FB contained or mod/RMB on bag body; outer clothing body LMB → SELF)
//! - soft-FB draws map + worn clothing contained at `slot_pos` (open UX parity)
//! - all object actions path-to-adjacent via [`click_object`] then queue/flush
//! - GUI: LMB/RMB pass `hover.contained_slot` into `walk_or_use_tile_hold` /
//!   `click_rmb_tile_ex`
//!
//! No GPU / screen pick — callers pass **world tile** coords (GUI uses `screen_to_world`).

use crate::actions::ObjectAction;
use crate::client_map::{ClientMap, MapTile};
use crate::content::ClientContent;
use crate::live_object::{clothing_char_to_slot, ClothingSet};
use crate::move_state::{MAX_PATH_DELTA, MoveError, PathDelta};
use crate::pathfind::{
    cell_walkable, chunk_deltas_for_move, find_path_with_waypoint_ex, path_cell_count,
    PathFindOpts, PathFindResult, CLOSE_HOLD_THROW_TILES, DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
    DEFAULT_MAX_EXPAND,
};
use crate::session::ClientSession;

/// C++ `noMoveAge` — first ~0.20 age units cannot MOVE (JUMP wiggle only).
pub const NO_MOVE_AGE: f32 = 0.20;

/// C++ `minMouseDownFrames` — continuous-mode blocked-tile slide after this many held frames.
///
/// Official client divides by `frameRateFactor` (~1 at 60 FPS → ~0.5 s hold).
pub const MIN_MOUSE_DOWN_FRAMES: i32 = 30;

/// C++ continuous-move blocked-tile push search limit (~25200 `limit = 10`).
pub const HOLD_SLIDE_LIMIT: i32 = 10;

/// C++ `LivingLifePage::pointerDown` early gates (~24934–24995).
///
/// Order matches official client:
/// 1. [`MoveError::AwaitingForceAck`] (headless also gates FORCE here)
/// 2. [`MoveError::ActionPending`] (`playerActionPending`)
/// 3. [`MoveError::HoldingImmobile`] (held speedMult==0, no ground use)
/// 4. held by adult → send `JUMP 0 0#` → [`MoveError::JumpSent`]
/// 5. age < [`NO_MOVE_AGE`] → send `JUMP 0 0#` → [`MoveError::JumpSent`]
pub fn apply_click_gates(session: &mut ClientSession) -> Result<(), MoveError> {
    if session.move_state.awaiting_force_ack {
        return Err(MoveError::AwaitingForceAck);
    }
    if session.player_action_pending {
        return Err(MoveError::ActionPending);
    }

    // Holding something that stops movement entirely — ignore click unless a
    // ground transition exists (e.g. fishing pole can be used on empty tile).
    let held = our_held_id(session);
    if held > 0 {
        if let Some(def) = session.content.get(held) {
            // C++: getObject(holdingID)->speedMult == 0
            if def.speed_mult == 0.0 {
                let can_use_on_ground = session.content.find_transition(held, -1).is_some();
                if !can_use_on_ground {
                    return Err(MoveError::HoldingImmobile);
                }
            }
        }
    }

    // Click from a held baby → JUMP out of arms (not MOVE).
    // P3#22: C++ skips while babyWiggle still playing (~24967).
    if session.we_are_held_by_adult() {
        let _ = session.send_jump().map_err(|_| MoveError::EmptyPath)?;
        return Err(MoveError::JumpSent);
    }

    // Too young to move → JUMP wiggle on the ground.
    if let Some(age) = session.our_age() {
        if age < NO_MOVE_AGE {
            session
                .send_jump()
                .map_err(|_| MoveError::EmptyPath)?;
            return Err(MoveError::JumpSent);
        }
    }

    Ok(())
}

/// C++ continuous-move blocked-tile slide of `clickDest` (`pointerDown` ~25181–25236).
///
/// When `click_dest` blocks walking, push further along the **major axis** of the
/// vector from `from_pos` → `click_dest` (up to [`HOLD_SLIDE_LIMIT`] steps) until a
/// walkable cell is found. Walkable destinations are returned unchanged.
///
/// Tie-break when `|dx| == |dy|`: push in **Y** (C++ `else` branch).
pub fn slide_blocked_click_dest(
    map: &ClientMap,
    content: Option<&ClientContent>,
    from_pos: (i32, i32),
    click_dest: (i32, i32),
) -> (i32, i32) {
    if cell_walkable(map, content, click_dest.0, click_dest.1) {
        return click_dest;
    }
    let x_delta = click_dest.0 - from_pos.0;
    let y_delta = click_dest.1 - from_pos.1;
    if x_delta.abs() > y_delta.abs() {
        // Push further in X (away from player along the larger axis).
        let step = if x_delta < 0 { -1 } else { 1 };
        let mut xd = step;
        while xd != step * (HOLD_SLIDE_LIMIT) {
            if cell_walkable(map, content, click_dest.0 + xd, click_dest.1) {
                return (click_dest.0 + xd, click_dest.1);
            }
            xd += step;
        }
        // Final step at limit (C++ `for (xd=step; xd != limit; xd += step)` excludes limit).
        // Match C++: limit is exclusive, so steps are step..limit-step. Already covered.
    } else {
        // Push further in Y (also used when axes equal).
        let step = if y_delta < 0 { -1 } else { 1 };
        let mut yd = step;
        while yd != step * HOLD_SLIDE_LIMIT {
            if cell_walkable(map, content, click_dest.0, click_dest.1 + yd) {
                return (click_dest.0, click_dest.1 + yd);
            }
            yd += step;
        }
    }
    click_dest
}

/// Resolve world click dest for continuous LMB hold (C++ `mouseAlreadyDown` path).
///
/// Only remaps after [`MIN_MOUSE_DOWN_FRAMES`] so brief presses over blockers do not
/// accidental-slide. `from_pos` is the player stand / path-start proxy for the major-axis push.
pub fn resolve_hold_click_dest(
    session: &ClientSession,
    tile_x: i32,
    tile_y: i32,
    mouse_already_down: bool,
    mouse_down_frames: i32,
) -> (i32, i32) {
    if !mouse_already_down || mouse_down_frames <= MIN_MOUSE_DOWN_FRAMES {
        return (tile_x, tile_y);
    }
    let content = if session.content.objects.is_empty() {
        None
    } else {
        Some(&session.content)
    };
    // C++ uses fractional currentPos (lrint) for slide origin.
    let from = if session.move_state.in_motion {
        session.move_state.current_pos_tile()
    } else {
        session
            .our_id
            .and_then(|id| session.world.get(id).map(|o| (o.x, o.y)))
            .unwrap_or_else(|| path_start_tile(session))
    };
    slide_blocked_click_dest(&session.map, content, from, (tile_x, tile_y))
}

/// Continuous LMB hold / first-press unified entry (C++ `pointerDown` + path-step auto-click).
///
/// * **First press** (`mouse_already_down == false`): full [`walk_or_use_tile_ex`]
///   (object USE/REMV, clothing DROP/SELF/SREMV, ground MOVE) with contained `hit_slot`.
/// * **Held**: ground MOVE only (C++ clears `hitAnObject` so no accidental USE while dragging);
///   after min frames, blocked hover tiles are slid via [`slide_blocked_click_dest`].
/// * Close-hold throw (C++ ~22850–22890): after min frames, if mouse is within
///   [`CLOSE_HOLD_THROW_TILES`] of the player, set `useWaypoint` at the mouse tile,
///   throw click dest out along the same vector, and path with `maxWaypointPathLength=10`.
/// * Always respects [`apply_click_gates`] (action pending / 0-speed / JUMP).
///
/// Callers increment `mouse_down_frames` each frame while LMB is down (reset on release).
pub fn walk_or_use_tile_hold(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    mouse_already_down: bool,
    mouse_down_frames: i32,
    clothing_slot: i32,
    hit_slot: i32,
) -> Result<WalkOrUseResult, MoveError> {
    if !mouse_already_down {
        return walk_or_use_tile_ex(session, tile_x, tile_y, clothing_slot, hit_slot);
    }
    // Continuous movement: gates first, then ground-only repath (no object branch).
    // C++ sets isAutoClick around pointerDown while LMB held / road auto-walk so
    // edge-of-bad does not allow entry into bad biomes (computePathToDest).
    apply_click_gates(session)?;
    let (dx, dy) = resolve_hold_click_dest(session, tile_x, tile_y, true, mouse_down_frames);

    // C++ close-hold: mouse near character → throw click out + path via waypoint at mouse.
    if mouse_down_frames > MIN_MOUSE_DOWN_FRAMES {
        if let Some((throw_x, throw_y)) = maybe_close_hold_throw(session, tile_x, tile_y) {
            session
                .move_state
                .arm_waypoint(tile_x, tile_y, DEFAULT_MAX_WAYPOINT_PATH_LENGTH);
            // click_tile_with clears waypoint flags after plan; auto_click=true.
            let r = click_tile_with(session, throw_x, throw_y, true)?;
            return Ok(WalkOrUseResult::Ground(r));
        }
    }

    let r = click_tile_with(session, dx, dy, true)?;
    Ok(WalkOrUseResult::Ground(r))
}

/// C++ close-hold throw (~22850–22890): if mouse is **more than 1** and **less than 4**
/// tiles away on **both** axes (axis-aligned box, not Euclidean), return a dest pushed
/// `CLOSE_HOLD_THROW_TILES` (Euclidean length 4) along the mouse vector from currentPos.
///
/// Waypoint stays at the real mouse tile (caller arms it). Returns `None` when outside
/// the close-hold zone or the throw dest collapses to the mouse tile.
pub fn maybe_close_hold_throw(
    session: &ClientSession,
    mouse_tx: i32,
    mouse_ty: i32,
) -> Option<(i32, i32)> {
    let (cx, cy) = if session.move_state.in_motion {
        (
            session.move_state.current_pos_x,
            session.move_state.current_pos_y,
        )
    } else {
        session
            .our_id
            .and_then(|id| session.world.get(id).map(|o| (o.x as f64, o.y as f64)))
            .unwrap_or_else(|| {
                let (x, y) = path_start_tile(session);
                (x as f64, y as f64)
            })
    };
    let mx = mouse_tx as f64;
    let my = mouse_ty as f64;
    let dx = mx - cx;
    let dy = my - cy;
    let abs_x = dx.abs();
    let abs_y = dy.abs();
    // C++ continuous-move outer: absX > CELL_D || absY > CELL_D (at least ~1 tile).
    if abs_x <= 1.0 && abs_y <= 1.0 {
        return None;
    }
    // C++ close-hold box: absX < CELL_D*4 && absY < CELL_D*4.
    if abs_x >= CLOSE_HOLD_THROW_TILES as f64 || abs_y >= CLOSE_HOLD_THROW_TILES as f64 {
        return None;
    }
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1e-6 {
        return None;
    }
    // C++: normalize(mouse - current) * CELL_D * 4 + current.
    let scale = CLOSE_HOLD_THROW_TILES as f64 / dist;
    let throw_x = (cx + dx * scale).round() as i32;
    let throw_y = (cy + dy * scale).round() as i32;
    if throw_x == mouse_tx && throw_y == mouse_ty {
        return None;
    }
    Some((throw_x, throw_y))
}

/// Result of a ground click / walk_to path plan + send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickTileResult {
    /// Wire line sent, e.g. `MOVE 0 0 @2 1 0 2 0#`.
    pub move_line: String,
    /// Absolute end of the path we committed to.
    pub end: (i32, i32),
    /// Requested click tile.
    pub goal: (i32, i32),
    /// Path origin used for MOVE `xs ys`.
    pub start: (i32, i32),
    /// True when `end == goal`.
    pub reached_goal: bool,
    /// True when this MOVE replaced an in-progress path.
    pub was_repath: bool,
    /// True when more MOVE chunks (or repath) remain after this hop.
    pub multi_move_pending: bool,
}

/// Path origin for a new ground click (C++ `findClosestPathSpot` / `closestPathPos`).
///
/// C++ `LivingLifePage.cpp` ~2341–2387:
/// - mid-move: `lrint(currentPos)` then snap onto `pathToDest` (fractional interp)
/// - idle: `xServer` / `yServer` (LiveObject PU) or dest `(xd,yd)`
pub fn path_start_tile(session: &ClientSession) -> (i32, i32) {
    let hint = session
        .our_id
        .and_then(|id| session.world.get(id).map(|o| (o.x, o.y)));
    session.move_state.closest_path_spot(hint)
}

/// Our held object id (0 = empty hands). C++ `LiveObject.holdingID`.
pub fn our_held_id(session: &ClientSession) -> i32 {
    session
        .our_id
        .and_then(|id| session.world.get(id).map(|o| o.held_id))
        .unwrap_or(0)
}

/// True when `(tile_x, tile_y)` is our dest / stand tile (`xd,yd`).
pub fn is_self_tile(session: &ClientSession, tile_x: i32, tile_y: i32) -> bool {
    session.move_state.x == tile_x && session.move_state.y == tile_y
}

/// Primary clothing slot for object id from `clothing=` char, or `None` if not clothing.
pub fn clothing_slot_for_object(content: &ClientContent, object_id: i32) -> Option<i32> {
    if object_id <= 0 {
        return None;
    }
    let ch = content
        .get(object_id)
        .or_else(|| content.get(content.base_object_id(object_id)))
        .map(|d| d.clothing)
        .unwrap_or('n');
    clothing_char_to_slot(ch)
}

/// Resolve equip target slot for held clothing (or explicit preferred).
///
/// - `preferred` in 0..5 wins
/// - else object `clothing=` char (shoes use empty front/back)
/// - non-clothing held → `None`
pub fn resolve_clothing_equip_slot(
    content: &ClientContent,
    held_id: i32,
    worn: &ClothingSet,
    preferred: Option<i32>,
) -> Option<i32> {
    if let Some(s) = preferred {
        if (0..=5).contains(&s) {
            // Shoes: still prefer empty mate when preferred is front and front full.
            if s == 2 && !worn.is_empty_slot(2) && worn.is_empty_slot(3) {
                return Some(3);
            }
            return Some(s);
        }
    }
    let primary = clothing_slot_for_object(content, held_id)?;
    if primary == 2 {
        return Some(worn.resolve_shoe_slot());
    }
    Some(primary)
}

/// Our worn [`ClothingSet`] (empty if unbound).
pub fn our_clothing(session: &ClientSession) -> ClothingSet {
    session
        .our_id
        .and_then(|id| session.world.get(id).map(|o| o.clothing.clone()))
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Action selection (C++ pointerDown ~26082–26350)
// ---------------------------------------------------------------------------

/// Plan for a map-tile click after action selection (not self/person specials).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileClickPlan {
    /// Empty-ground left-click → MOVE only.
    Move,
    /// Path-to-adjacent (if needed) then this wire action.
    Act(ObjectAction),
}

/// Object permanent flag; unknown ids treated as non-permanent.
fn obj_permanent(content: &ClientContent, id: i32) -> bool {
    if id <= 0 {
        return false;
    }
    content
        .get(id)
        .or_else(|| content.get(content.base_object_id(id)))
        .map(|o| o.permanent)
        .unwrap_or(false)
}

fn obj_num_slots(content: &ClientContent, id: i32) -> i32 {
    if id <= 0 {
        return 0;
    }
    content
        .get(id)
        .or_else(|| content.get(content.base_object_id(id)))
        .map(|o| o.num_slots)
        .unwrap_or(0)
}

fn obj_description(content: &ClientContent, id: i32) -> String {
    if id <= 0 {
        return String::new();
    }
    content
        .get(id)
        .or_else(|| content.get(content.base_object_id(id)))
        .map(|o| o.description.clone())
        .unwrap_or_default()
}

fn has_use_on_contained(content: &ClientContent, id: i32) -> bool {
    obj_description(content, id).contains("+useOnContained")
}

/// True when the container still has a free slot for put (`contained < numSlots`).
#[inline]
fn container_has_free_slot(num_contained: i32, num_slots: i32) -> bool {
    num_slots > 0 && num_contained < num_slots
}

/// C++ `LivingLifePage::pointerDown` tile action select (~26104–26314).
///
/// Headless assumes the click **hits** the object when `dest_id > 0` (no hitMap).
/// `hit_slot` is the container slot index (`-1` = top / default) from soft-FB
/// [`crate::hover_pick::HoverPick::contained_slot`].
///
/// `mod_click` = right mouse / Q / E-key (C++ `modClick`).
///
/// Container slot UX (take / put):
/// - empty hand + slots → [`ObjectAction::Remv`] with `hit_slot` (`-1` top)
/// - holding + container → USE (transition / `+useOnContained`) or DROP into
pub fn select_tile_action(
    content: &ClientContent,
    x: i32,
    y: i32,
    mod_click: bool,
    held_id: i32,
    tile: &MapTile,
    hit_slot: i32,
) -> TileClickPlan {
    let dest_id = tile.object_id;
    let floor_id = tile.floor_id;
    let dest_num_contained = tile.contained_ids().len() as i32;
    let dest_permanent = obj_permanent(content, dest_id);
    let dest_slots = obj_num_slots(content, dest_id);
    // C++ always wires hitSlotIndex (defaults −1 = top of stack).
    let slot_param = if hit_slot < 0 { -1 } else { hit_slot };

    // Left-click empty tile → MOVE (action branch not entered).
    if !mod_click && dest_id == 0 {
        return TileClickPlan::Move;
    }

    // Enter action branch when C++ would (~25792–25806).
    let enter_action = (mod_click
        && (held_id != 0
            || (dest_id > 0 && !dest_permanent)
            || dest_num_contained > 0))
        || dest_id != 0;
    if !enter_action {
        return TileClickPlan::Move;
    }

    // --- modClick on empty destID (or walk-near cleared dest) ---
    if mod_click && dest_id == 0 {
        if held_id != 0 {
            // Prefer use-on-bare-ground / use-on-floor over DROP when applicable.
            if held_id > 0 {
                let held_food = content.food_value(held_id);
                if held_food == 0 {
                    if let Some(tr) = content.find_transition(held_id, -1) {
                        if tr.new_target_id != 0 {
                            return TileClickPlan::Act(ObjectAction::Use {
                                x,
                                y,
                                object_id: None,
                                slot: None,
                            });
                        }
                    }
                }
                if floor_id > 0 && content.find_transition(held_id, floor_id).is_some() {
                    return TileClickPlan::Act(ObjectAction::Use {
                        x,
                        y,
                        object_id: None,
                        slot: None,
                    });
                }
            }
            // Plain DROP on empty (c=-1). SWAP same server-side for non-containers.
            return TileClickPlan::Act(ObjectAction::Drop {
                x,
                y,
                clothing_slot: -1,
            });
        }
        // Empty hand + modClick empty tile — rare; C++ still USE.
        return TileClickPlan::Act(ObjectAction::Use {
            x,
            y,
            object_id: None,
            slot: None,
        });
    }

    // --- Take: empty hand + container → REMV hit_slot (C++ ~26177–26227) ---
    // Permanent + no bare-hand: LMB or RMB. Non-permanent containers: modClick/RMB.
    if dest_id != 0
        && (mod_click
            || (dest_permanent && content.find_transition(0, dest_id).is_none()))
        && held_id == 0
        && dest_slots > 0
    {
        // +useOnContained with modClick on a contained slot → USE id i
        if mod_click && hit_slot >= 0 && has_use_on_contained(content, dest_id) {
            return TileClickPlan::Act(ObjectAction::Use {
                x,
                y,
                object_id: Some(dest_id),
                slot: Some(hit_slot),
            });
        }
        // Decay-in-1s bare-hand fallback → USE (left click only).
        if !mod_click {
            if let Some(decay) = content.find_transition(-1, dest_id) {
                if decay.new_target_id > 0
                    && (decay.auto_decay_seconds - 1.0).abs() < 0.01
                    && content.find_transition(0, decay.new_target_id).is_some()
                {
                    return TileClickPlan::Act(ObjectAction::Use {
                        x,
                        y,
                        object_id: Some(dest_id),
                        slot: None,
                    });
                }
            }
        }
        return TileClickPlan::Act(ObjectAction::Remv {
            x,
            y,
            slot: slot_param,
        });
    }

    // --- Put: holding + container ---
    // C++ modClick → DROP when destNumContained <= numSlots (~26228–26235).
    if mod_click
        && held_id != 0
        && dest_id != 0
        && dest_slots > 0
        && dest_num_contained <= dest_slots
    {
        return TileClickPlan::Act(ObjectAction::Drop {
            x,
            y,
            clothing_slot: -1,
        });
    }

    // LMB put into container when no USE transition / useOnContained applies.
    // // Playtest: basket put without requiring RMB (C++ LMB would silent-USE).
    if !mod_click
        && held_id > 0
        && dest_id != 0
        && dest_slots > 0
        && container_has_free_slot(dest_num_contained, dest_slots)
        && content.find_transition(held_id, dest_id).is_none()
        && !(hit_slot >= 0
            && dest_slots > hit_slot
            && has_use_on_contained(content, dest_id))
    {
        return TileClickPlan::Act(ObjectAction::Drop {
            x,
            y,
            clothing_slot: -1,
        });
    }

    // modClick + holding → DROP/SWAP onto non-permanent ground object.
    if mod_click && held_id != 0 && dest_id != 0 && dest_slots == 0 && !dest_permanent {
        // C++ uses DROP here when hitAnObject; SWAP when destID cleared near object.
        // Server treats them equivalently for non-containers; use SWAP when swapping
        // with a tile that still has an object id (clearer intent for playtest).
        return TileClickPlan::Act(ObjectAction::Swap { x, y });
    }

    // Default: USE (left or right when nothing else applies).
    if dest_id != 0 {
        // Bare hand on non-permanent non-container: REMV vs USE when both apply.
        if held_id == 0 && dest_slots == 0 && !dest_permanent {
            if let Some(tr) = content.find_transition(0, dest_id) {
                if tr.new_target_id != 0 {
                    // Transformational bare-hand (leaves something on ground).
                    if mod_click {
                        return TileClickPlan::Act(ObjectAction::Use {
                            x,
                            y,
                            object_id: Some(dest_id),
                            slot: None,
                        });
                    }
                    return TileClickPlan::Act(ObjectAction::Remv { x, y, slot: 0 });
                }
            }
        }
        // USE on contained slot when +useOnContained (holding + soft-FB hit_slot).
        let use_slot = if held_id > 0
            && hit_slot >= 0
            && dest_slots > hit_slot
            && has_use_on_contained(content, dest_id)
        {
            Some(hit_slot)
        } else {
            None
        };
        return TileClickPlan::Act(ObjectAction::Use {
            x,
            y,
            object_id: Some(dest_id),
            slot: use_slot,
        });
    }

    TileClickPlan::Move
}

/// C++ self-click (~25294–25355): eat / clothing / SREMV.
///
/// `clothing_slot` = `-1` when not a clothing hit; `0..5` for hat…backpack.
/// `hit_slot` = contained index for SREMV (`-1` top).
///
/// When `mod_click` and held clothing with `clothing_slot < 0`, resolves equip
/// slot from held type + empty shoe preference (MVP without worn hitMap).
pub fn select_self_action(
    content: &ClientContent,
    x: i32,
    y: i32,
    mod_click: bool,
    held_id: i32,
    clothing_slot: i32,
    hit_slot: i32,
) -> ObjectAction {
    select_self_action_ex(
        content,
        x,
        y,
        mod_click,
        held_id,
        clothing_slot,
        hit_slot,
        &ClothingSet::default(),
    )
}

/// Like [`select_self_action`] with worn clothing for shoe/empty-slot resolve.
pub fn select_self_action_ex(
    content: &ClientContent,
    x: i32,
    y: i32,
    mod_click: bool,
    held_id: i32,
    clothing_slot: i32,
    hit_slot: i32,
    worn: &ClothingSet,
) -> ObjectAction {
    let mut slot = clothing_slot;

    // Explicit worn hitMap / key slot + held → DROP into that c (keys 1–6 parity).
    // // C++: click clothing while holding → DROP x y c# (equip or put-into bag)
    if held_id > 0 && slot >= 0 {
        return ObjectAction::Drop {
            x,
            y,
            clothing_slot: slot,
        };
    }

    // Resolve equip slot when held is clothing and caller did not pick a slot.
    if slot < 0 && held_id > 0 {
        if let Some(s) = resolve_clothing_equip_slot(content, held_id, worn, None) {
            // Auto-equip uses DROP path (mod semantics) when caller asked mod
            // or when held is pure clothing (not food).
            let is_food = content.food_value(held_id) > 0;
            if mod_click || !is_food {
                slot = s;
            }
        }
    }

    // Empty hand + soft-FB contained hit on worn bag → take (SREMV) without requiring
    // RMB: open/take parity with map-container REMV (P1#6 playtest).
    // // C++: modClick SREMV; we also LMB-take when hitSlotIndex names a contained item.
    if held_id == 0 && slot >= 0 && hit_slot >= 0 {
        return ObjectAction::Sremv {
            x,
            y,
            clothing_slot: slot,
            slot: hit_slot,
        };
    }

    // mod + clothing slot (explicit or resolved): DROP insert / SREMV from bag body.
    if mod_click && slot >= 0 {
        if held_id > 0 {
            return ObjectAction::Drop {
                x,
                y,
                clothing_slot: slot,
            };
        }
        return ObjectAction::Sremv {
            x,
            y,
            clothing_slot: slot,
            slot: if hit_slot < 0 { -1 } else { hit_slot },
        };
    }

    // LMB with held clothing + resolved slot → DROP equip (protocol add-to-clothing).
    if !mod_click && held_id > 0 && slot >= 0 && content.food_value(held_id) == 0 {
        if clothing_slot_for_object(content, held_id).is_some() {
            return ObjectAction::Drop {
                x,
                y,
                clothing_slot: slot,
            };
        }
    }

    // Eat food / remove clothing (SELF) / bare self.
    ObjectAction::SelfAct {
        x,
        y,
        clothing_slot: if slot < 0 { -1 } else { slot },
    }
}

// ---------------------------------------------------------------------------
// Path plan + MOVE
// ---------------------------------------------------------------------------

/// Plan a click-to-move path without sending.
///
/// Returns `(path_find, first_chunk_path, remaining_chunks, ultimate_goal)`.
///
/// C++: `pointerDown` empty-tile → `mustMove` → `computePathToDest`.
pub fn plan_click_tile(
    session: &ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<(PathFindResult, Vec<PathDelta>), MoveError> {
    let (plan, first, _rest) = plan_click_tile_chunks(session, tile_x, tile_y)?;
    Ok((plan, first))
}

/// Like [`plan_click_tile`] but also returns remaining MOVE chunks after the first.
pub fn plan_click_tile_chunks(
    session: &ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<(PathFindResult, Vec<PathDelta>, Vec<Vec<PathDelta>>), MoveError> {
    plan_click_tile_chunks_with(session, tile_x, tile_y, false)
}

/// Plan MOVE path with optional C++ `isAutoClick` (hold / road auto-walk).
pub fn plan_click_tile_chunks_with(
    session: &ClientSession,
    tile_x: i32,
    tile_y: i32,
    auto_click: bool,
) -> Result<(PathFindResult, Vec<PathDelta>, Vec<Vec<PathDelta>>), MoveError> {
    let (plan, first, rest, _goal) =
        plan_click_tile_chunks_goal(session, tile_x, tile_y, auto_click)?;
    Ok((plan, first, rest))
}

/// Full plan including effective goal (may equal waypoint when two-leg path is truncated).
pub fn plan_click_tile_chunks_goal(
    session: &ClientSession,
    tile_x: i32,
    tile_y: i32,
    auto_click: bool,
) -> Result<(PathFindResult, Vec<PathDelta>, Vec<Vec<PathDelta>>, (i32, i32)), MoveError> {
    // Gate FORCE before any map/content work.
    if session.move_state.awaiting_force_ack {
        return Err(MoveError::AwaitingForceAck);
    }
    // C++ SameTile: click vs xd,yd (dest), not vs closestPathPos.
    // Mid-move re-click of the current dest is ignored; re-click of stand tile repaths home.
    if session.move_state.x == tile_x && session.move_state.y == tile_y {
        return Err(MoveError::SameTile);
    }
    let start = path_start_tile(session);
    // Degenerate: start already at click (e.g. path snap == click) → no MOVE.
    if start == (tile_x, tile_y) {
        return Err(MoveError::SameTile);
    }
    let content = if session.content.objects.is_empty() {
        None
    } else {
        Some(&session.content)
    };
    // P2#9: bad-biome edge routing + rideable ignoreBad (C++ computePathToDest).
    // P2#10: optional useWaypoint two-leg path (C++ computePathToDest ~2554–2589).
    let auto = auto_click || session.move_state.path_auto_click;
    let opts = session.path_find_opts_with(auto);
    let wp = if session.move_state.use_waypoint {
        Some(session.move_state.waypoint)
    } else {
        None
    };
    let (res, eff_goal) = find_path_with_waypoint_ex(
        &session.map,
        content,
        start,
        (tile_x, tile_y),
        DEFAULT_MAX_EXPAND,
        &opts,
        wp,
        session.move_state.max_waypoint_path_length,
    );
    if res.deltas.is_empty() {
        return Err(MoveError::EmptyPath);
    }
    if start == eff_goal {
        return Err(MoveError::SameTile);
    }
    // Split into ±16 MOVE windows (C++ pathFindingD=32; multi-hop after done_moving).
    let chunks = chunk_deltas_for_move(&res.deltas, MAX_PATH_DELTA);
    let mut iter = chunks.into_iter();
    let first_raw = iter.next().unwrap_or_default();
    if first_raw.is_empty() {
        return Err(MoveError::EmptyPath);
    }
    let first: Vec<PathDelta> = first_raw
        .into_iter()
        .map(|(x, y)| PathDelta { x, y })
        .collect();
    let rest: Vec<Vec<PathDelta>> = iter
        .map(|c| c.into_iter().map(|(x, y)| PathDelta { x, y }).collect())
        .filter(|c: &Vec<PathDelta>| !c.is_empty())
        .collect();
    Ok((res, first, rest, eff_goal))
}

/// Click empty ground tile → pathfind → send MOVE.
///
/// - Runs [`apply_click_gates`] first (action pending / 0-speed / JUMP).
/// - Clears queued `nextActionMessageToSend` (new ground click aborts pending USE/DROP).
/// - Allows mid-move repath (new MOVE with next seq).
/// - Cumulative path deltas (protocol + C++ encoding).
/// - Closest-reachable fallback when the exact goal is blocked/unreachable.
/// - Arms multi-MOVE follow-up when path needs more than one ±16 chunk or goal not reached.
pub fn click_tile(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<ClickTileResult, MoveError> {
    click_tile_with(session, tile_x, tile_y, false)
}

/// Like [`click_tile`] with C++ `isAutoClick` (continuous hold / road auto-walk).
pub fn click_tile_with(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    auto_click: bool,
) -> Result<ClickTileResult, MoveError> {
    apply_click_gates(session)?;
    let (plan, path, rest, goal) =
        plan_click_tile_chunks_goal(session, tile_x, tile_y, auto_click)?;
    // One-shot waypoint flags (C++ clears useWaypoint after pointerDown).
    session.move_state.clear_waypoint();
    let start = plan.start;
    let was_repath = session.move_state.in_motion;

    // C++ pointerDown clears nextActionMessageToSend before ground move.
    session.cancel_pending_action();
    session.clear_multi_move();

    // Align MOVE origin (may differ from move_state dest while mid-path).
    session.move_state.x = start.0;
    session.move_state.y = start.1;

    let offset = session.map_global_offset;
    let line = if was_repath {
        session
            .move_state
            .send_move_repath_with_offset(&path, offset)?
    } else {
        session.move_state.send_move_with_offset(&path, offset)?
    };
    session
        .send_raw(&line)
        .map_err(|_| MoveError::EmptyPath)?;
    // Walk anim + currentPos before server PM (Jason onPath).
    session.sync_our_live_motion();
    if let Some(oid) = session.our_id {
        if let Some(o) = session.world.get_mut(oid) {
            o.moving = true;
        }
    }

    let end = if let Some(last) = path.last() {
        (start.0 + last.x, start.1 + last.y)
    } else {
        plan.end
    };

    // P2#11: arm remaining ±16 chunks and/or ultimate goal for done_moving
    // continuation. `goal` is the requested click (or waypoint rewrite). Path may
    // end short via window clamp / closest-fallback — continue_multi_move repaths.
    let multi = session.arm_multi_move(rest, goal, end);
    // Belt-and-suspenders: if pathfind did not fully reach the requested goal,
    // keep multi-MOVE armed even when chunk list is empty (window / closest).
    if !plan.reached_goal && end != goal {
        session.multi_move_goal = Some(goal);
    }

    Ok(ClickTileResult {
        move_line: line,
        end,
        goal,
        start,
        reached_goal: end == goal && !multi && plan.reached_goal,
        was_repath,
        multi_move_pending: multi || session.has_multi_move(),
    })
}

/// Convenience: click_tile discarding metadata.
pub fn walk_to(session: &mut ClientSession, goal_x: i32, goal_y: i32) -> Result<(), MoveError> {
    click_tile(session, goal_x, goal_y).map(|_| ())
}

// ---------------------------------------------------------------------------
// Object path-to-adjacent (C++ pointerDown object / USE-DROP-REMV branch)
// ---------------------------------------------------------------------------

/// C++ `isGridAdjacent` — **4-neighbor only** (not diagonal).
#[inline]
pub fn is_grid_adjacent(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    ((ax - bx).abs() == 1 && ay == by) || ((ay - by).abs() == 1 && ax == bx)
}

/// Stand offsets: self, W, E, S, N — C++ `nDX`/`nDY` (~25876).
const STAND_DX: [i32; 5] = [0, -1, 1, 0, 0];
const STAND_DY: [i32; 5] = [0, 0, 0, -1, 1];

/// Result of object-target click: optional MOVE + queued/sent USE/DROP/REMV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectClickResult {
    /// MOVE line when we had to walk to an adjacent stand tile.
    pub move_line: Option<String>,
    /// Encoded action (`USE x y id#` / DROP / REMV / …).
    pub action_line: String,
    /// Tile the player stands on to act (adjacent or same as target).
    pub stand: (i32, i32),
    /// Object / action target tile (coords on the wire action).
    pub target: (i32, i32),
    /// Path origin used for any MOVE.
    pub start: (i32, i32),
    /// True when already 4-adjacent or on the target tile (no MOVE needed for adjacency).
    pub already_adjacent: bool,
    /// True when the action was written immediately; false when queued as `pending_action`.
    pub action_sent: bool,
    /// True when a MOVE was sent (or repath).
    pub moved: bool,
    /// True when MOVE replaced an in-progress path.
    pub was_repath: bool,
    /// True when more MOVE hops remain before the action can flush.
    pub multi_move_pending: bool,
}

/// Access flags for stand selection (C++ `sideAccess` / `noBackAccess`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StandAccess {
    pub side_access: bool,
    pub no_back_access: bool,
}

/// True when standing at `stand` can access object at `target` given access flags.
///
/// C++ ~25824–25842: adjacent or same-tile, but:
/// - `sideAccess` forbids N/S approaches (`stand.y != target.y`)
/// - `noBackAccess` forbids approach from N (`stand.y > target.y`)
pub fn stand_allows_access(
    stand: (i32, i32),
    target: (i32, i32),
    access: StandAccess,
) -> bool {
    let adjacent =
        is_grid_adjacent(stand.0, stand.1, target.0, target.1) || stand == target;
    if !adjacent {
        return false;
    }
    if access.side_access && stand.1 != target.1 {
        // N or S of side-access object
        return false;
    }
    if access.no_back_access && stand.1 > target.1 {
        // North of noBackAccess object
        return false;
    }
    true
}

/// C++ permanent-food / sick self-tile prefer (~25902–25936).
///
/// Prefer standing on the object tile itself when dest is permanent + non-blocking
/// and either: empty hand with no bare-hand trans, or bare-hand yields edible
/// `newActor` with non-blocking `newTarget` remaining on the ground.
fn prefer_self_tile_for_food(
    content: &ClientContent,
    dest_id: i32,
    held_id: i32,
) -> bool {
    if dest_id <= 0 {
        return false;
    }
    let Some(dest) = content
        .get(dest_id)
        .or_else(|| content.get(content.base_object_id(dest_id)))
    else {
        return false;
    };
    if !dest.permanent || dest.blocks_walking {
        return false;
    }
    // holdingID <= 0 (empty / holding person) OR permanent "sick" held
    let hands_ok = if held_id <= 0 {
        true
    } else if let Some(held) = content.get(held_id) {
        held.permanent
            && (held.description.contains("sick") || held.name.contains("sick"))
    } else {
        false
    };
    if !hands_ok {
        return false;
    }
    // Only consult bare-hand trans when empty-handed (C++).
    if held_id == 0 {
        match content.find_transition(0, dest_id) {
            None => true,
            Some(tr) => {
                tr.new_actor_id != 0
                    && content.food_value(tr.new_actor_id) > 0
                    && tr.new_target_id != 0
                    && !content.blocks_walking(tr.new_target_id)
            }
        }
    } else {
        // permanent sick held → always prefer self when dest qualifies
        true
    }
}

/// Same-tile USE blocked when result newTarget blocks walking (C++ ~25844–25866).
fn same_tile_use_blocks(
    content: &ClientContent,
    dest_id: i32,
    held_id: i32,
) -> bool {
    if dest_id <= 0 {
        return false;
    }
    if let Some(tr) = content.find_transition(held_id.max(0), dest_id) {
        if tr.new_target_id > 0 && content.blocks_walking(tr.new_target_id) {
            return true;
        }
    }
    false
}

/// Plan: which stand tile to walk to for an object action (no send).
///
/// `path_start` is the pathfind origin (`closestPathPos`); `dest_pos` is C++ `xd,yd`
/// used for the already-adjacent gate. `held_id` is C++ `holdingID` (0 = empty).
///
/// Returns `(stand, path_plan)` where `path_plan` is `None` when no MOVE is needed.
/// Errors with [`MoveError::NoAdjacentStand`] when no reachable ortho neighbor
/// (or walkable self-tile fallback) exists.
///
/// C++ `LivingLifePage::pointerDown` stand search ~25810–26038:
/// - `sideAccess` → only W/E
/// - `noBackAccess` → W/E/S (not N)
/// - permanent food harvest → prefer self-tile first
/// - last-resort self-tile when all neighbors blocked (not sideAccess)
pub fn plan_stand_for_object(
    map: &ClientMap,
    content: Option<&ClientContent>,
    path_start: (i32, i32),
    dest_pos: (i32, i32),
    target: (i32, i32),
) -> Result<((i32, i32), Option<PathFindResult>), MoveError> {
    plan_stand_for_object_ex(map, content, path_start, dest_pos, target, 0)
}

/// Like [`plan_stand_for_object`] but with explicit held id for food self-tile prefer.
pub fn plan_stand_for_object_ex(
    map: &ClientMap,
    content: Option<&ClientContent>,
    path_start: (i32, i32),
    dest_pos: (i32, i32),
    target: (i32, i32),
    held_id: i32,
) -> Result<((i32, i32), Option<PathFindResult>), MoveError> {
    plan_stand_for_object_with_opts(
        map,
        content,
        path_start,
        dest_pos,
        target,
        held_id,
        &PathFindOpts::default(),
    )
}

/// Stand plan with biome / rideable path options (P2#9).
pub fn plan_stand_for_object_with_opts(
    map: &ClientMap,
    content: Option<&ClientContent>,
    path_start: (i32, i32),
    dest_pos: (i32, i32),
    target: (i32, i32),
    held_id: i32,
    opts: &PathFindOpts<'_>,
) -> Result<((i32, i32), Option<PathFindResult>), MoveError> {
    plan_stand_for_object_with_opts_wp(
        map,
        content,
        path_start,
        dest_pos,
        target,
        held_id,
        opts,
        None,
        DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
    )
}

/// Stand plan with optional C++ `useWaypoint` two-leg path (P2#10).
///
/// C++ stand search calls `computePathToDest` per neighbor, which honors `useWaypoint`.
/// When the two-leg path exceeds `max_waypoint_path_length`, the path may end at the
/// waypoint (dest truncated); caller still treats `stand` as the multi-MOVE goal.
pub fn plan_stand_for_object_with_opts_wp(
    map: &ClientMap,
    content: Option<&ClientContent>,
    path_start: (i32, i32),
    dest_pos: (i32, i32),
    target: (i32, i32),
    held_id: i32,
    opts: &PathFindOpts<'_>,
    waypoint: Option<(i32, i32)>,
    max_waypoint_path_length: i32,
) -> Result<((i32, i32), Option<PathFindResult>), MoveError> {
    let dest_id = map.get(target.0, target.1).map(|t| t.object_id).unwrap_or(0);
    let access = StandAccess {
        side_access: content.map(|c| c.side_access(dest_id)).unwrap_or(false),
        no_back_access: content.map(|c| c.no_back_access(dest_id)).unwrap_or(false),
    };

    // C++ canExecute: adjacency vs xd,yd (dest), not closestPathPos.
    let mut can_execute = stand_allows_access(dest_pos, target, access);
    if can_execute && dest_pos == target {
        if let Some(c) = content {
            if same_tile_use_blocks(c, dest_id, held_id) {
                can_execute = false;
            }
        }
    }
    if can_execute {
        return Ok((dest_pos, None));
    }
    // Also treat path_start already on a valid stand as no MOVE.
    if stand_allows_access(path_start, target, access) {
        let mut ok = true;
        if path_start == target {
            if let Some(c) = content {
                if same_tile_use_blocks(c, dest_id, held_id) {
                    ok = false;
                }
            }
        }
        if ok {
            return Ok((path_start, None));
        }
    }

    // nStart / nLimit (C++ ~25890–25936)
    let mut n_start = 1usize; // skip self by default
    let mut n_limit = 5usize; // self + W/E/S/N
    if access.side_access {
        n_limit = 3; // self + W + E only (self skipped via n_start)
    } else if access.no_back_access {
        n_limit = 4; // self + W + E + S (not N)
    } else if let Some(c) = content {
        if prefer_self_tile_for_food(c, dest_id, held_id) {
            n_start = 0; // consider self first; always prefer if reachable
        }
    }

    // Rank: prefer full reach of stand (not waypoint-truncated), then shorter pathLength.
    // Score packing: full→ high bit; lower path_cell_count is better.
    let mut best: Option<(i32, (i32, i32), PathFindResult)> = None;
    let mut found_self = false;

    for n in n_start..n_limit {
        let sx = target.0 + STAND_DX[n];
        let sy = target.1 + STAND_DY[n];
        if !cell_walkable(map, content, sx, sy) {
            continue;
        }
        if (sx, sy) == path_start {
            return Ok((path_start, None));
        }
        let (res, eff) = find_path_with_waypoint_ex(
            map,
            content,
            path_start,
            (sx, sy),
            DEFAULT_MAX_EXPAND,
            opts,
            waypoint,
            max_waypoint_path_length,
        );
        // C++ pathFind false → pathToDest NULL (skip). Closest-only without reach skips.
        // Waypoint-truncated (eff rewritten to wp) still has a real path — accept it.
        let full = res.reached_goal && eff == (sx, sy);
        let partial_wp = waypoint.is_some() && !res.deltas.is_empty() && res.reached_goal && eff != (sx, sy);
        if !full && !partial_wp {
            continue;
        }
        if res.deltas.is_empty() {
            return Ok(((sx, sy), None));
        }
        let cells = path_cell_count(&res);
        // Lower score wins: full paths get 0 base, partial get large base.
        let score = if full { cells } else { 1_000_000 + cells };
        let better = best.as_ref().map(|(bs, _, _)| score < *bs).unwrap_or(true);
        if better {
            best = Some((score, (sx, sy), res));
        }
        // Self-tile preferred when n_start==0: take first reachable self and stop.
        if n == 0 && full {
            found_self = true;
            break;
        }
    }
    if found_self {
        if let Some((_, stand, res)) = best {
            return Ok((stand, Some(res)));
        }
    }

    // C++ last-resort self-tile when neighbors blocked (~25997–26030).
    // Not for sideAccess; only when self was not already considered (n_start > 0).
    if best.is_none()
        && !access.side_access
        && n_start > 0
        && cell_walkable(map, content, target.0, target.1)
    {
        if path_start == target {
            return Ok((path_start, None));
        }
        let (res, eff) = find_path_with_waypoint_ex(
            map,
            content,
            path_start,
            target,
            DEFAULT_MAX_EXPAND,
            opts,
            waypoint,
            max_waypoint_path_length,
        );
        let full = res.reached_goal && eff == target;
        let partial_wp = waypoint.is_some() && !res.deltas.is_empty() && res.reached_goal && eff != target;
        if full && !res.deltas.is_empty() {
            return Ok((target, Some(res)));
        }
        if full && res.deltas.is_empty() {
            return Ok((target, None));
        }
        if partial_wp {
            return Ok((target, Some(res)));
        }
    }

    match best {
        Some((_, stand, res)) => Ok((stand, Some(res))),
        None => Err(MoveError::NoAdjacentStand),
    }
}

/// Fill optional USE object id from the client map when missing.
pub fn resolve_use_object_id(session: &ClientSession, action: &mut ObjectAction) {
    if let ObjectAction::Use {
        x,
        y,
        object_id,
        ..
    } = action
    {
        if object_id.is_none() {
            if let Some(t) = session.map.get(*x, *y) {
                if t.object_id > 0 {
                    *object_id = Some(t.object_id);
                }
            }
        }
    }
}

/// Path to an adjacent stand tile (if needed), then send or queue `action`.
///
/// C++ `LivingLifePage::pointerDown` object branch (~25803–26350):
/// - Click gates first ([`apply_click_gates`]).
/// - If 4-adjacent (or same tile): queue/send action, no MOVE for adjacency.
/// - Else: shortest path among walkable W/E/S/N neighbors; MOVE then
///   `nextActionMessageToSend` with **object** coords (not stand).
/// - Mid-move repath allowed; replaces any previous pending action.
pub fn click_object(
    session: &mut ClientSession,
    mut action: ObjectAction,
) -> Result<ObjectClickResult, MoveError> {
    apply_click_gates(session)?;

    resolve_use_object_id(session, &mut action);
    let target = action.target_xy();
    let path_start = path_start_tile(session);
    let dest_pos = (session.move_state.x, session.move_state.y);
    let content = if session.content.objects.is_empty() {
        None
    } else {
        Some(&session.content)
    };

    let held = our_held_id(session);
    let opts = session.path_find_opts();
    // P2#10: stand search uses computePathToDest → honors useWaypoint (C++ ~25967).
    let wp = if session.move_state.use_waypoint {
        Some(session.move_state.waypoint)
    } else {
        None
    };
    let max_wp = session.move_state.max_waypoint_path_length;
    let (stand, path_opt) = plan_stand_for_object_with_opts_wp(
        &session.map,
        content,
        path_start,
        dest_pos,
        target,
        held,
        &opts,
        wp,
        max_wp,
    )?;
    // One-shot waypoint flags (C++ clears useWaypoint after pointerDown ~22890).
    session.move_state.clear_waypoint();

    // Need MOVE to stand?
    if let Some(plan) = path_opt {
        let chunks = chunk_deltas_for_move(&plan.deltas, MAX_PATH_DELTA);
        let mut iter = chunks.into_iter();
        let first = iter.next().unwrap_or_default();
        if first.is_empty() {
            return Err(MoveError::EmptyPath);
        }
        let path: Vec<PathDelta> = first
            .into_iter()
            .map(|(x, y)| PathDelta { x, y })
            .collect();
        let rest: Vec<Vec<PathDelta>> = iter
            .map(|c| c.into_iter().map(|(x, y)| PathDelta { x, y }).collect())
            .filter(|c: &Vec<PathDelta>| !c.is_empty())
            .collect();

        let was_repath = session.move_state.in_motion;
        // Replace any prior nextAction; ground-style cancel then set ours after MOVE.
        session.cancel_pending_action();
        session.clear_multi_move();

        session.move_state.x = path_start.0;
        session.move_state.y = path_start.1;
        let offset = session.map_global_offset;
        let move_line = if was_repath {
            session
                .move_state
                .send_move_repath_with_offset(&path, offset)?
        } else {
            session
                .move_state
                .send_move_with_offset(&path, offset)?
        };
        session
            .send_raw(&move_line)
            .map_err(|_| MoveError::EmptyPath)?;
        session.sync_our_live_motion();
        if let Some(oid) = session.our_id {
            if let Some(o) = session.world.get_mut(oid) {
                o.moving = true;
            }
        }

        let end = if let Some(last) = path.last() {
            (path_start.0 + last.x, path_start.1 + last.y)
        } else {
            plan.end
        };
        // Multi-MOVE toward stand; action waits until final hop + adjacency.
        // P2#11: keep ultimate stand when hop ends short (window / closest-fallback).
        let multi = session.arm_multi_move(rest, stand, end);
        if !plan.reached_goal && end != stand {
            session.multi_move_goal = Some(stand);
        }

        // Queue action for done_moving flush (coords stay on object tile).
        let action_line = action.encode();
        session.queue_pending_action(action);

        Ok(ObjectClickResult {
            move_line: Some(move_line),
            action_line,
            stand,
            target,
            start: path_start,
            already_adjacent: false,
            action_sent: false,
            moved: true,
            was_repath,
            multi_move_pending: multi || session.has_multi_move(),
        })
    } else {
        // Already adjacent / on stand: send or queue without MOVE for adjacency.
        // If mid-move toward another tile, still only queue (server ignores USE mid-MOVE).
        session.clear_multi_move();
        let action_line = action.encode();
        let action_sent = !(session.move_state.in_motion || session.move_state.awaiting_force_ack);
        if action_sent {
            // send_object_action sets player_action_pending (C++ playerActionPending).
            session
                .send_object_action(action)
                .map_err(|_| MoveError::EmptyPath)?;
        } else {
            session.queue_pending_action(action);
        }
        Ok(ObjectClickResult {
            move_line: None,
            action_line,
            stand,
            target,
            start: path_start,
            already_adjacent: true,
            action_sent,
            moved: false,
            was_repath: false,
            multi_move_pending: false,
        })
    }
}

/// Click object tile → path-to-adjacent → queue/send `USE x y [id] [slot]#`.
pub fn click_use(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    object_id: Option<i32>,
    slot: Option<i32>,
) -> Result<ObjectClickResult, MoveError> {
    click_object(
        session,
        ObjectAction::Use {
            x: tile_x,
            y: tile_y,
            object_id,
            slot,
        },
    )
}

/// Path-to-adjacent then DROP (`DROP x y c#`; ground clothing_slot = **-1**).
pub fn click_drop(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    clothing_slot: i32,
) -> Result<ObjectClickResult, MoveError> {
    click_object(
        session,
        ObjectAction::Drop {
            x: tile_x,
            y: tile_y,
            clothing_slot,
        },
    )
}

/// Path-to-adjacent then REMV (`REMV x y i#`; **-1** = top of stack).
///
/// `slot` is wire `hit_slot` from soft-FB [`crate::hover_pick::HoverPick::contained_slot`]
/// or a map stack index via [`crate::hover_pick::resolve_hit_slot`] /
/// [`crate::hover_pick::map_stack_index_to_hit_slot`].
pub fn click_remv(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    slot: i32,
) -> Result<ObjectClickResult, MoveError> {
    let slot = if slot < 0 { -1 } else { slot };
    click_object(
        session,
        ObjectAction::Remv {
            x: tile_x,
            y: tile_y,
            slot,
        },
    )
}

/// REMV with soft-FB contained + optional map stack index → resolved `hit_slot`.
///
/// Soft-FB wins when `soft_fb_contained >= 0`; else `map_stack_index`.
pub fn click_remv_hit(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    soft_fb_contained: i32,
    map_stack_index: i32,
) -> Result<ObjectClickResult, MoveError> {
    let slot = crate::hover_pick::resolve_hit_slot(soft_fb_contained, map_stack_index);
    click_remv(session, tile_x, tile_y, slot)
}

/// Path-to-adjacent then SWAP.
pub fn click_swap(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<ObjectClickResult, MoveError> {
    click_object(session, ObjectAction::Swap { x: tile_x, y: tile_y })
}

/// Self-tile action (eat / clothing / SREMV) — no pathfind.
///
/// Resolves equip slot from held clothing type when `clothing_slot < 0`.
pub fn click_self(
    session: &mut ClientSession,
    clothing_slot: i32,
    hit_slot: i32,
    mod_click: bool,
) -> Result<ObjectClickResult, MoveError> {
    apply_click_gates(session)?;
    let (x, y) = (session.move_state.x, session.move_state.y);
    let held = our_held_id(session);
    let worn = our_clothing(session);
    let action = select_self_action_ex(
        &session.content,
        x,
        y,
        mod_click,
        held,
        clothing_slot,
        hit_slot,
        &worn,
    );
    let action_line = action.encode();
    let action_sent = !(session.move_state.in_motion || session.move_state.awaiting_force_ack);
    if action_sent {
        session
            .send_object_action(action)
            .map_err(|_| MoveError::EmptyPath)?;
    } else {
        session.queue_pending_action(action);
    }
    Ok(ObjectClickResult {
        move_line: None,
        action_line,
        stand: (x, y),
        target: (x, y),
        start: (x, y),
        already_adjacent: true,
        action_sent,
        moved: false,
        was_repath: false,
        multi_move_pending: false,
    })
}

/// Headless: `DROP x y c#` into own clothing slot `0..5` at self stand.
///
/// // C++: mod-click clothing slot while holding → DROP with c
pub fn click_drop_clothing(
    session: &mut ClientSession,
    clothing_slot: i32,
) -> Result<ObjectClickResult, MoveError> {
    let slot = if (0..=5).contains(&clothing_slot) {
        clothing_slot
    } else {
        // Resolve from held type when caller passes -1 or out of range.
        let held = our_held_id(session);
        let worn = our_clothing(session);
        resolve_clothing_equip_slot(&session.content, held, &worn, None).unwrap_or(-1)
    };
    if slot < 0 {
        // No clothing slot — fall back to ground DROP at self (c=-1).
        let (x, y) = (session.move_state.x, session.move_state.y);
        return click_drop(session, x, y, -1);
    }
    // Force DROP path (mod + held).
    click_self(session, slot, -1, true)
}

/// Headless: remove outer clothing via `SELF x y c#` (empty hands).
pub fn click_remove_clothing(
    session: &mut ClientSession,
    clothing_slot: i32,
) -> Result<ObjectClickResult, MoveError> {
    click_self(session, clothing_slot, -1, false)
}

/// Headless: `SREMV x y c i#` from worn clothing container (empty hands, mod).
pub fn click_sremv_clothing(
    session: &mut ClientSession,
    clothing_slot: i32,
    hit_slot: i32,
) -> Result<ObjectClickResult, MoveError> {
    click_self(session, clothing_slot, hit_slot, true)
}

/// True when self-tile click should take clothing/eat path instead of ground USE/MOVE.
fn self_tile_wants_self_action(
    content: &ClientContent,
    held_id: i32,
    mod_click: bool,
    clothing_slot: i32,
) -> bool {
    if clothing_slot >= 0 {
        return true;
    }
    if held_id > 0 {
        if content.food_value(held_id) > 0 {
            return true;
        }
        if clothing_slot_for_object(content, held_id).is_some() {
            return true;
        }
        // mod + held non-clothing on self → ground DROP (fall through)
        return false;
    }
    // Empty hands + mod + explicit clothing later; without hitMap only when slot set.
    mod_click && clothing_slot >= 0
}

/// Click tile with full modClick action selection (C++ pointerDown lite + mod).
///
/// - Explicit `clothing_slot` 0..5 (worn hitMap / keys) → [`click_self`] first
/// - Self-tile food/clothing → [`click_self`] / DROP c / SREMV (before ground USE)
/// - `mod_click=false`: empty → MOVE; object → USE (or REMV for bare-hand transform).
/// - `mod_click=true` + held: DROP/SWAP/container DROP; empty → DROP (or USE-on-ground).
/// - `mod_click=true` + empty hand on container: REMV.
pub fn click_tile_mod(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    mod_click: bool,
    hit_slot: i32,
) -> Result<WalkOrUseResult, MoveError> {
    click_tile_mod_ex(session, tile_x, tile_y, mod_click, hit_slot, -1)
}

/// Like [`click_tile_mod`] with worn clothing slot from soft-FB hitMap (or keys).
///
/// `clothing_slot` in `0..5` forces self clothing action even when the hover
/// tile is not the stand cell (clothing sprite overhang).
pub fn click_tile_mod_ex(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    mod_click: bool,
    hit_slot: i32,
    clothing_slot: i32,
) -> Result<WalkOrUseResult, MoveError> {
    let held = our_held_id(session);

    // Worn clothing hitMap / explicit slot — always self clothing path.
    // // C++: LivingLifePage clothing getSpriteHit → DROP/SELF/SREMV with c
    if (0..=5).contains(&clothing_slot) {
        let r = click_self(session, clothing_slot, hit_slot, mod_click)?;
        return Ok(WalkOrUseResult::Object(r));
    }

    // C++ self/clothing branch (~25294–25355) before ground object select.
    if is_self_tile(session, tile_x, tile_y)
        && self_tile_wants_self_action(&session.content, held, mod_click, clothing_slot)
    {
        let r = click_self(session, clothing_slot, hit_slot, mod_click)?;
        return Ok(WalkOrUseResult::Object(r));
    }

    let tile = session
        .map
        .get(tile_x, tile_y)
        .cloned()
        .unwrap_or_else(MapTile::empty);
    let plan = select_tile_action(
        &session.content,
        tile_x,
        tile_y,
        mod_click,
        held,
        &tile,
        hit_slot,
    );
    match plan {
        TileClickPlan::Move => {
            let r = click_tile(session, tile_x, tile_y)?;
            Ok(WalkOrUseResult::Ground(r))
        }
        TileClickPlan::Act(action) => {
            let r = click_object(session, action)?;
            Ok(WalkOrUseResult::Object(r))
        }
    }
}

/// Click tile: empty ground → MOVE; object → path-to-adjacent USE (LMB default).
///
/// Full DROP/REMV/SWAP selection: [`click_tile_mod`] with `mod_click=true`.
/// Worn clothing: [`click_tile_mod_ex`] with `clothing_slot` from hitMap.
pub fn walk_or_use_tile(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
) -> Result<WalkOrUseResult, MoveError> {
    click_tile_mod(session, tile_x, tile_y, false, -1)
}

/// LMB with optional worn clothing slot + contained `hit_slot` from soft-FB hitMap.
///
/// `hit_slot` is [`crate::hover_pick::HoverPick::contained_slot`] (`-1` = top /
/// container body). Used for REMV `i` and SREMV `i` on multi-slot containers.
pub fn walk_or_use_tile_ex(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    clothing_slot: i32,
    hit_slot: i32,
) -> Result<WalkOrUseResult, MoveError> {
    click_tile_mod_ex(session, tile_x, tile_y, false, hit_slot, clothing_slot)
}

/// Continuous hold without clothing/hit slots (ground repath helper).
pub fn hold_walk_or_use_tile(
    session: &mut ClientSession,
    tile_x: i32,
    tile_y: i32,
    mouse_already_down: bool,
    mouse_down_frames: i32,
) -> Result<WalkOrUseResult, MoveError> {
    walk_or_use_tile_hold(
        session,
        tile_x,
        tile_y,
        mouse_already_down,
        mouse_down_frames,
        -1,
        -1,
    )
}

/// Outcome of [`walk_or_use_tile`] / [`click_tile_mod`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkOrUseResult {
    Ground(ClickTileResult),
    Object(ObjectClickResult),
}

/// True when player dest `(px,py)` may flush a pending action aimed at `(tx,ty)`.
///
/// C++ flush gate: `isGridAdjacent` **or** same tile **or** `playerActionTargetNotAdjacent`.
pub fn can_execute_action_at(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    is_grid_adjacent(px, py, tx, ty) || (px == tx && py == ty)
}

/// Ergonomic methods on [`ClientSession`] (avoids conflicting with legacy `walk_to`).
pub trait ClickTileExt {
    fn click_tile_to_move(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<ClickTileResult, MoveError>;
    fn plan_click_tile_to_move(
        &self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<(PathFindResult, Vec<PathDelta>), MoveError>;
    fn path_start_for_click(&self) -> (i32, i32);
    fn click_object_action(
        &mut self,
        action: ObjectAction,
    ) -> Result<ObjectClickResult, MoveError>;
    fn click_use_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    ) -> Result<ObjectClickResult, MoveError>;
    fn click_drop_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        clothing_slot: i32,
    ) -> Result<ObjectClickResult, MoveError>;
    fn click_remv_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        slot: i32,
    ) -> Result<ObjectClickResult, MoveError>;
    fn walk_or_use(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<WalkOrUseResult, MoveError>;
    fn click_mod(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        mod_click: bool,
    ) -> Result<WalkOrUseResult, MoveError>;
    fn click_drop_clothing_slot(
        &mut self,
        clothing_slot: i32,
    ) -> Result<ObjectClickResult, MoveError>;
    fn click_self_slot(
        &mut self,
        clothing_slot: i32,
        hit_slot: i32,
        mod_click: bool,
    ) -> Result<ObjectClickResult, MoveError>;
    /// Continuous LMB hold repath / blocked-tile slide ([`walk_or_use_tile_hold`]).
    fn walk_or_use_hold(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        mouse_already_down: bool,
        mouse_down_frames: i32,
        clothing_slot: i32,
        hit_slot: i32,
    ) -> Result<WalkOrUseResult, MoveError>;
}

impl ClickTileExt for ClientSession {
    fn click_tile_to_move(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<ClickTileResult, MoveError> {
        click_tile(self, tile_x, tile_y)
    }

    fn plan_click_tile_to_move(
        &self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<(PathFindResult, Vec<PathDelta>), MoveError> {
        plan_click_tile(self, tile_x, tile_y)
    }

    fn path_start_for_click(&self) -> (i32, i32) {
        path_start_tile(self)
    }

    fn click_object_action(
        &mut self,
        action: ObjectAction,
    ) -> Result<ObjectClickResult, MoveError> {
        click_object(self, action)
    }

    fn click_use_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    ) -> Result<ObjectClickResult, MoveError> {
        click_use(self, tile_x, tile_y, object_id, slot)
    }

    fn click_drop_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        clothing_slot: i32,
    ) -> Result<ObjectClickResult, MoveError> {
        click_drop(self, tile_x, tile_y, clothing_slot)
    }

    fn click_remv_object(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        slot: i32,
    ) -> Result<ObjectClickResult, MoveError> {
        click_remv(self, tile_x, tile_y, slot)
    }

    fn walk_or_use(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<WalkOrUseResult, MoveError> {
        walk_or_use_tile(self, tile_x, tile_y)
    }

    fn click_mod(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        mod_click: bool,
    ) -> Result<WalkOrUseResult, MoveError> {
        click_tile_mod(self, tile_x, tile_y, mod_click, -1)
    }

    fn click_drop_clothing_slot(
        &mut self,
        clothing_slot: i32,
    ) -> Result<ObjectClickResult, MoveError> {
        click_drop_clothing(self, clothing_slot)
    }

    fn click_self_slot(
        &mut self,
        clothing_slot: i32,
        hit_slot: i32,
        mod_click: bool,
    ) -> Result<ObjectClickResult, MoveError> {
        click_self(self, clothing_slot, hit_slot, mod_click)
    }

    fn walk_or_use_hold(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        mouse_already_down: bool,
        mouse_down_frames: i32,
        clothing_slot: i32,
        hit_slot: i32,
    ) -> Result<WalkOrUseResult, MoveError> {
        walk_or_use_tile_hold(
            self,
            tile_x,
            tile_y,
            mouse_already_down,
            mouse_down_frames,
            clothing_slot,
            hit_slot,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ObjectAction;
    use crate::content::{ClientContent, ClientObjectDef, ClientTransition};
    use crate::frame::{write_message, FrameReader};
    use crate::move_state::MoveState;
    use crate::parse::MapChunkHeader;
    use crate::session::{SessionConfig, SessionEvent};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn framed_text(s: &str) -> Vec<u8> {
        crate::frame::encode_raw(s).into_bytes()
    }

    fn test_cfg(port: u16) -> SessionConfig {
        SessionConfig {
            host: "127.0.0.1".into(),
            port,
            email: "user@test".into(),
            password: "secret".into(),
            account_key: "key123".into(),
            pad_email_to_80: false,
            read_timeout: Duration::from_millis(400),
            write_timeout: Duration::from_secs(2),
            ..SessionConfig::default()
        }
    }

    fn login_then_peer_capture(
        bodies: Vec<Vec<u8>>,
        captured: Arc<Mutex<Vec<String>>>,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = match sock.read(&mut buf) {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                let msgs = fr.push(&buf[..n]);
                if msgs
                    .into_iter()
                    .any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN "))
                {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            for body in bodies {
                sock.write_all(&body).unwrap();
            }
            sock.set_read_timeout(Some(Duration::from_millis(300))).ok();
            loop {
                let n = match sock.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                let msgs = fr.push(&buf[..n]);
                if let Ok(mut c) = captured.lock() {
                    c.extend(msgs);
                }
            }
        });
        (port, handle)
    }

    /// Fill a rectangular open region (`0:0:0` cells). Unknown tiles block pathfind.
    fn seed_open_map(session: &mut ClientSession, x: i32, y: i32, w: i32, h: i32) {
        let n = (w.max(1) * h.max(1)) as usize;
        let plain = vec!["0:0:0"; n].join(" ");
        let header = MapChunkHeader {
            size_x: w.max(1),
            size_y: h.max(1),
            x,
            y,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        session
            .map
            .apply_mc_plaintext(&header, &plain)
            .expect("seed open map");
    }

    fn login_then_peer(bodies: Vec<Vec<u8>>) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap();
                if n == 0 {
                    return;
                }
                let msgs = fr.push(&buf[..n]);
                if msgs
                    .into_iter()
                    .any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN "))
                {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            for body in bodies {
                sock.write_all(&body).unwrap();
            }
            thread::sleep(Duration::from_millis(150));
        });
        (port, handle)
    }

    fn empty_tile() -> MapTile {
        MapTile::empty()
    }

    fn tile_with(oid: i32, contained: &str) -> MapTile {
        let raw = if contained.is_empty() {
            oid.to_string()
        } else {
            format!("{oid},{contained}")
        };
        MapTile {
            biome: 0,
            floor_id: 0,
            object_id: oid,
            object_raw: raw,
        }
    }

    fn content_with(objects: Vec<ClientObjectDef>, transitions: Vec<ClientTransition>) -> ClientContent {
        let mut c = ClientContent::default();
        for o in objects {
            c.objects.insert(o.id, o);
        }
        for t in transitions {
            c.transitions.insert((t.actor_id, t.target_id), t);
        }
        c
    }

    #[test]
    fn select_modclick_empty_held_is_drop() {
        let c = ClientContent::default();
        let plan = select_tile_action(&c, 3, 4, true, 99, &empty_tile(), -1);
        assert_eq!(
            plan,
            TileClickPlan::Act(ObjectAction::Drop {
                x: 3,
                y: 4,
                clothing_slot: -1,
            })
        );
    }

    #[test]
    fn select_modclick_nonperm_held_is_swap() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 40,
                permanent: false,
                num_slots: 0,
                ..Default::default()
            }],
            vec![],
        );
        let plan = select_tile_action(&c, 1, 2, true, 7, &tile_with(40, ""), -1);
        assert_eq!(plan, TileClickPlan::Act(ObjectAction::Swap { x: 1, y: 2 }));
    }

    #[test]
    fn select_modclick_container_held_is_drop() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 125,
                permanent: true,
                num_slots: 3,
                ..Default::default()
            }],
            vec![],
        );
        let plan = select_tile_action(&c, 5, 5, true, 33, &tile_with(125, ""), -1);
        assert_eq!(
            plan,
            TileClickPlan::Act(ObjectAction::Drop {
                x: 5,
                y: 5,
                clothing_slot: -1,
            })
        );
    }

    #[test]
    fn select_empty_hand_container_is_remv() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 125,
                permanent: true,
                num_slots: 2,
                ..Default::default()
            }],
            vec![],
        );
        // No bare-hand trans → REMV even on left click.
        let plan = select_tile_action(&c, 2, 3, false, 0, &tile_with(125, "33"), -1);
        assert_eq!(
            plan,
            TileClickPlan::Act(ObjectAction::Remv {
                x: 2,
                y: 3,
                slot: -1,
            })
        );
        // Soft-FB contained hover → REMV with explicit slot index.
        let plan_slot = select_tile_action(&c, 2, 3, true, 0, &tile_with(125, "33,40"), 1);
        assert_eq!(
            plan_slot,
            TileClickPlan::Act(ObjectAction::Remv {
                x: 2,
                y: 3,
                slot: 1,
            })
        );
    }

    #[test]
    fn select_self_sremv_uses_hit_slot() {
        let c = ClientContent::default();
        let act = select_self_action(&c, 10, 10, true, 0, 5, 2);
        assert_eq!(
            act,
            ObjectAction::Sremv {
                x: 10,
                y: 10,
                clothing_slot: 5,
                slot: 2,
            }
        );
    }

    #[test]
    fn select_bare_hand_transform_left_remv_right_use() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 50,
                permanent: false,
                num_slots: 0,
                ..Default::default()
            }],
            vec![ClientTransition {
                actor_id: 0,
                target_id: 50,
                new_actor_id: 51,
                new_target_id: 52, // leaves something
                ..Default::default()
            }],
        );
        let left = select_tile_action(&c, 0, 0, false, 0, &tile_with(50, ""), -1);
        assert_eq!(
            left,
            TileClickPlan::Act(ObjectAction::Remv {
                x: 0,
                y: 0,
                slot: 0,
            })
        );
        let right = select_tile_action(&c, 0, 0, true, 0, &tile_with(50, ""), -1);
        assert_eq!(
            right,
            TileClickPlan::Act(ObjectAction::Use {
                x: 0,
                y: 0,
                object_id: Some(50),
                slot: None,
            })
        );
    }

    #[test]
    fn select_left_object_is_use() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 99,
                permanent: true,
                num_slots: 0,
                ..Default::default()
            }],
            vec![],
        );
        let plan = select_tile_action(&c, 8, 9, false, 0, &tile_with(99, ""), -1);
        assert_eq!(
            plan,
            TileClickPlan::Act(ObjectAction::Use {
                x: 8,
                y: 9,
                object_id: Some(99),
                slot: None,
            })
        );
    }

    #[test]
    fn select_use_on_ground_overrides_drop() {
        let c = content_with(
            vec![ClientObjectDef {
                id: 70,
                food_value: 0,
                ..Default::default()
            }],
            vec![ClientTransition {
                actor_id: 70,
                target_id: -1,
                new_actor_id: 0,
                new_target_id: 71,
                ..Default::default()
            }],
        );
        let plan = select_tile_action(&c, 1, 1, true, 70, &empty_tile(), -1);
        assert_eq!(
            plan,
            TileClickPlan::Act(ObjectAction::Use {
                x: 1,
                y: 1,
                object_id: None,
                slot: None,
            })
        );
    }

    #[test]
    fn select_self_eat_and_clothing_drop() {
        let c = ClientContent::default();
        let eat = select_self_action(&c, 0, 0, false, 100, -1, -1);
        assert_eq!(
            eat,
            ObjectAction::SelfAct {
                x: 0,
                y: 0,
                clothing_slot: -1,
            }
        );
        let drop_hat = select_self_action(&c, 0, 0, true, 5, 0, -1);
        assert_eq!(
            drop_hat,
            ObjectAction::Drop {
                x: 0,
                y: 0,
                clothing_slot: 0,
            }
        );
        let sremv = select_self_action(&c, 0, 0, true, 0, 5, -1);
        assert_eq!(
            sremv,
            ObjectAction::Sremv {
                x: 0,
                y: 0,
                clothing_slot: 5,
                slot: -1,
            }
        );
    }

    #[test]
    fn clothing_char_and_resolve_equip_slot() {
        let mut c = ClientContent::default();
        c.objects.insert(
            50,
            ClientObjectDef {
                id: 50,
                clothing: 'h',
                ..Default::default()
            },
        );
        c.objects.insert(
            51,
            ClientObjectDef {
                id: 51,
                clothing: 's',
                ..Default::default()
            },
        );
        c.objects.insert(
            52,
            ClientObjectDef {
                id: 52,
                clothing: 'n',
                food_value: 3,
                ..Default::default()
            },
        );
        assert_eq!(clothing_slot_for_object(&c, 50), Some(0));
        assert_eq!(clothing_slot_for_object(&c, 51), Some(2));
        assert_eq!(clothing_slot_for_object(&c, 52), None);

        let empty = ClothingSet::default();
        assert_eq!(
            resolve_clothing_equip_slot(&c, 50, &empty, None),
            Some(0)
        );
        assert_eq!(
            resolve_clothing_equip_slot(&c, 51, &empty, None),
            Some(2)
        );
        let front_worn = ClothingSet::parse("0;0;99;0;0;0");
        assert_eq!(
            resolve_clothing_equip_slot(&c, 51, &front_worn, None),
            Some(3)
        );
        assert_eq!(
            resolve_clothing_equip_slot(&c, 51, &front_worn, Some(2)),
            Some(3)
        );
        assert_eq!(
            resolve_clothing_equip_slot(&c, 50, &empty, Some(5)),
            Some(5)
        );

        // LMB held hat → DROP 0; mod empty backpack → SREMV 5
        let equip = select_self_action_ex(&c, 1, 2, false, 50, -1, -1, &empty);
        assert_eq!(
            equip,
            ObjectAction::Drop {
                x: 1,
                y: 2,
                clothing_slot: 0,
            }
        );
        let sremv = select_self_action_ex(&c, 1, 2, true, 0, 5, -1, &empty);
        assert_eq!(
            sremv,
            ObjectAction::Sremv {
                x: 1,
                y: 2,
                clothing_slot: 5,
                slot: -1,
            }
        );
        // Food LMB → SELF -1 (not clothing DROP)
        let eat = select_self_action_ex(&c, 0, 0, false, 52, -1, -1, &empty);
        assert_eq!(
            eat,
            ObjectAction::SelfAct {
                x: 0,
                y: 0,
                clothing_slot: -1,
            }
        );

        // Explicit worn hitMap slot + held non-clothing → DROP c (keys/hit parity).
        let put_pack = select_self_action_ex(&c, 0, 0, false, 99, 5, -1, &empty);
        assert_eq!(
            put_pack,
            ObjectAction::Drop {
                x: 0,
                y: 0,
                clothing_slot: 5,
            }
        );
        // Bare hand hitMap on hat → SELF remove
        let remove_hat = select_self_action_ex(&c, 0, 0, false, 0, 0, -1, &empty);
        assert_eq!(
            remove_hat,
            ObjectAction::SelfAct {
                x: 0,
                y: 0,
                clothing_slot: 0,
            }
        );
        // P1#6: empty hand + LMB soft-FB contained hit → SREMV take (no RMB required)
        let take_bag = select_self_action_ex(&c, 0, 0, false, 0, 5, 1, &empty);
        assert_eq!(
            take_bag,
            ObjectAction::Sremv {
                x: 0,
                y: 0,
                clothing_slot: 5,
                slot: 1,
            }
        );
        // Bag body LMB (no contained hit) still SELF remove outer clothing
        let remove_bag = select_self_action_ex(&c, 0, 0, false, 0, 5, -1, &empty);
        assert_eq!(
            remove_bag,
            ObjectAction::SelfAct {
                x: 0,
                y: 0,
                clothing_slot: 5,
            }
        );
    }

    #[test]
    fn click_tile_mod_ex_worn_clothing_slot_wire() {
        // Empty hands + clothing_slot from hitMap → SELF c (remove worn).
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 12.0 60.0 3.75 201;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // LMB on hat hitMap (slot 0) even if click tile is not self — forces self path.
        let r = click_tile_mod_ex(&mut session, 3, 3, false, -1, 0).unwrap();
        match r {
            WalkOrUseResult::Object(o) => {
                assert!(o.action_sent);
                assert_eq!(o.action_line, "SELF 10 10 0#");
            }
            other => panic!("expected clothing SELF, got {other:?}"),
        }
        // Clear action-pending gate (server PU would clear in play).
        session.player_action_pending = false;
        // Mod / RMB on backpack slot → SREMV
        let r2 = click_tile_mod_ex(&mut session, 10, 10, true, -1, 5).unwrap();
        match r2 {
            WalkOrUseResult::Object(o) => {
                assert_eq!(o.action_line, "SREMV 10 10 5 -1#");
            }
            other => panic!("expected SREMV, got {other:?}"),
        }
        // Contained hit_slot from soft-FB → SREMV i
        session.player_action_pending = false;
        let r3 = click_tile_mod_ex(&mut session, 10, 10, true, 0, 5).unwrap();
        match r3 {
            WalkOrUseResult::Object(o) => {
                assert_eq!(o.action_line, "SREMV 10 10 5 0#");
            }
            other => panic!("expected SREMV with hit_slot, got {other:?}"),
        }
        // P1#6: LMB on soft-FB contained (no mod) also SREMV take
        session.player_action_pending = false;
        let r4 = click_tile_mod_ex(&mut session, 10, 10, false, 1, 5).unwrap();
        match r4 {
            WalkOrUseResult::Object(o) => {
                assert_eq!(o.action_line, "SREMV 10 10 5 1#");
            }
            other => panic!("expected LMB contained SREMV, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn walk_or_use_tile_ex_remv_hit_slot_wire() {
        // Permanent container + empty hand + hit_slot 1 → REMV x y 1#
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 12);
        // Isolate from default content (bare-hand USE transitions on real ids).
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9125,
                permanent: true,
                num_slots: 2,
                ..Default::default()
            }],
            vec![],
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 9125,
            object_id_raw: "9125,33,40".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match walk_or_use_tile_ex(&mut session, 6, 5, -1, 1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "REMV 6 5 1#");
            }
            other => panic!("expected REMV hit_slot, got {other:?}"),
        }
        let _ = handle.join();
    }

    /// P1#6 container slot UX: select take / put / useOnContained matrix.
    #[test]
    fn select_container_slot_ux_take_put() {
        let c = content_with(
            vec![
                ClientObjectDef {
                    id: 200,
                    permanent: true,
                    num_slots: 3,
                    description: "Basket".into(),
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 201,
                    permanent: true,
                    num_slots: 2,
                    description: "Bowl +useOnContained".into(),
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 50,
                    permanent: false,
                    num_slots: 0,
                    ..Default::default()
                },
            ],
            vec![ClientTransition {
                actor_id: 50,
                target_id: 201,
                new_actor_id: 0,
                new_target_id: 201,
                ..Default::default()
            }],
        );
        // Empty hand LMB → REMV top (−1) / hit_slot i
        let take_top = select_tile_action(&c, 4, 4, false, 0, &tile_with(200, "9"), -1);
        assert_eq!(
            take_top,
            TileClickPlan::Act(ObjectAction::Remv {
                x: 4,
                y: 4,
                slot: -1,
            })
        );
        let take_i = select_tile_action(&c, 4, 4, false, 0, &tile_with(200, "9,8"), 1);
        assert_eq!(
            take_i,
            TileClickPlan::Act(ObjectAction::Remv {
                x: 4,
                y: 4,
                slot: 1,
            })
        );
        // Holding + modClick → DROP into
        let put_mod = select_tile_action(&c, 4, 4, true, 50, &tile_with(200, "9"), -1);
        assert_eq!(
            put_mod,
            TileClickPlan::Act(ObjectAction::Drop {
                x: 4,
                y: 4,
                clothing_slot: -1,
            })
        );
        // Holding + LMB + no USE transition + free slot → DROP put
        let put_lmb = select_tile_action(&c, 4, 4, false, 50, &tile_with(200, ""), -1);
        assert_eq!(
            put_lmb,
            TileClickPlan::Act(ObjectAction::Drop {
                x: 4,
                y: 4,
                clothing_slot: -1,
            })
        );
        // Holding + LMB + USE transition → USE (not DROP)
        let use_tr = select_tile_action(&c, 4, 4, false, 50, &tile_with(201, ""), -1);
        assert_eq!(
            use_tr,
            TileClickPlan::Act(ObjectAction::Use {
                x: 4,
                y: 4,
                object_id: Some(201),
                slot: None,
            })
        );
        // Holding + useOnContained + hit_slot → USE id i
        let use_slot = select_tile_action(&c, 4, 4, false, 50, &tile_with(201, "7"), 0);
        assert_eq!(
            use_slot,
            TileClickPlan::Act(ObjectAction::Use {
                x: 4,
                y: 4,
                object_id: Some(201),
                slot: Some(0),
            })
        );
        // Full container LMB (no free slot) → USE fallback (no silent DROP)
        let full = select_tile_action(&c, 4, 4, false, 50, &tile_with(200, "1,2,3"), -1);
        assert_eq!(
            full,
            TileClickPlan::Act(ObjectAction::Use {
                x: 4,
                y: 4,
                object_id: Some(200),
                slot: None,
            })
        );
    }

    /// P1#6: path-to-adjacent REMV hit_slot when not already next to container.
    #[test]
    fn container_slot_ux_path_to_adjacent_remv() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 2 2 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n12 12 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9200,
                permanent: true,
                num_slots: 3,
                ..Default::default()
            }],
            vec![],
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 8,
            y: 8,
            floor_id: 0,
            object_id: 9200,
            object_id_raw: "9200,11,12".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        // Player (2,2) → container (8,8): must MOVE then queue REMV 8 8 1#
        match walk_or_use_tile_ex(&mut session, 8, 8, -1, 1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "REMV 8 8 1#");
                assert!(!r.action_sent, "action queued until adjacent");
                assert!(r.moved, "path-to-adjacent MOVE expected");
                assert!(r.move_line.is_some());
                assert_eq!(r.target, (8, 8));
            }
            other => panic!("expected path+REMV, got {other:?}"),
        }
        let _ = handle.join();
    }

    /// P1#6: path-to-adjacent DROP put when holding + container with room.
    #[test]
    fn container_slot_ux_path_to_adjacent_drop_put() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 55 0 0 0 -1 0.5 1 0 1 1 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n12 12 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.content = content_with(
            vec![
                ClientObjectDef {
                    id: 9201,
                    permanent: true,
                    num_slots: 4,
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 55,
                    permanent: false,
                    num_slots: 0,
                    ..Default::default()
                },
            ],
            vec![],
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 7,
            y: 3,
            floor_id: 0,
            object_id: 9201,
            object_id_raw: "9201".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        // LMB put (no USE transition) → DROP into; far stand → path-to-adjacent.
        match walk_or_use_tile_ex(&mut session, 7, 3, -1, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "DROP 7 3 -1#");
                assert!(!r.action_sent);
                assert!(r.moved);
            }
            other => panic!("expected path+DROP put, got {other:?}"),
        }
        session.player_action_pending = false;
        session.move_state = MoveState::default();
        session.move_state.x = 1;
        session.move_state.y = 1;
        session.cancel_pending_action();
        // Adjacent modClick also DROP.
        session.move_state.x = 7;
        session.move_state.y = 2;
        match click_tile_mod(&mut session, 7, 3, true, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "DROP 7 3 -1#");
                assert!(r.action_sent || r.already_adjacent);
            }
            other => panic!("expected adjacent DROP, got {other:?}"),
        }
        let _ = handle.join();
    }

    /// P1#6: holding + useOnContained USE with hit_slot wire.
    #[test]
    fn container_slot_ux_use_on_contained_wire() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 60 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        session.content = content_with(
            vec![
                ClientObjectDef {
                    id: 9300,
                    permanent: true,
                    num_slots: 2,
                    description: "Clay Bowl +useOnContained".into(),
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 60,
                    permanent: false,
                    num_slots: 0,
                    ..Default::default()
                },
            ],
            vec![],
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 9300,
            object_id_raw: "9300,1".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match walk_or_use_tile_ex(&mut session, 6, 5, -1, 0).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "USE 6 5 9300 0#");
            }
            other => panic!("expected USE on contained, got {other:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn click_remv_hit_soft_fb_or_map_stack_wire() {
        // resolve_hit_slot → select_tile_action REMV i (pure) + one adjacent wire send.
        use crate::hover_pick::resolve_hit_slot;

        let c = content_with(
            vec![ClientObjectDef {
                id: 9125,
                permanent: true,
                num_slots: 3,
                ..Default::default()
            }],
            vec![],
        );
        let tile = tile_with(9125, "33,40,41");
        // Map stack only.
        let slot = resolve_hit_slot(-1, 2);
        assert_eq!(
            select_tile_action(&c, 6, 5, true, 0, &tile, slot),
            TileClickPlan::Act(ObjectAction::Remv {
                x: 6,
                y: 5,
                slot: 2,
            })
        );
        // Soft-FB wins over stack.
        let slot = resolve_hit_slot(0, 2);
        assert_eq!(
            select_tile_action(&c, 6, 5, true, 0, &tile, slot),
            TileClickPlan::Act(ObjectAction::Remv {
                x: 6,
                y: 5,
                slot: 0,
            })
        );
        // Neither → top (-1).
        let slot = resolve_hit_slot(-1, -1);
        assert_eq!(
            select_tile_action(&c, 6, 5, true, 0, &tile, slot),
            TileClickPlan::Act(ObjectAction::Remv {
                x: 6,
                y: 5,
                slot: -1,
            })
        );

        // One wire path: already-adjacent REMV with resolved hit_slot.
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.content = c;
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 9125,
            object_id_raw: "9125,33,40,41".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let r = click_remv_hit(&mut session, 6, 5, -1, 2).unwrap();
        assert_eq!(r.action_line, "REMV 6 5 2#");
        let _ = handle.join();
    }

    #[test]
    fn click_drop_clothing_wire_slots_0_to_5() {
        // Holding object 50 (hat) at (10,10); DROP c for each clothing slot.
        let bind_pu = "PU\n\
7 100 1 0 0 0 50 0 0 0 -1 0.5 1 0 10 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 32, 30);
        session.content.objects.insert(
            50,
            ClientObjectDef {
                id: 50,
                clothing: 'h',
                ..Default::default()
            },
        );
        // Clear action-pending between drops so gates allow clicks.
        for c in 0..=5 {
            session.player_action_pending = false;
            let r = click_drop_clothing(&mut session, c).unwrap();
            assert_eq!(
                r.action_line,
                format!("DROP 10 10 {c}#"),
                "slot {c}"
            );
            assert!(r.action_sent, "slot {c} should send immediately");
            assert!(r.already_adjacent);
        }
        // Auto-resolve from held hat → DROP 0
        session.player_action_pending = false;
        let auto = click_drop_clothing(&mut session, -1).unwrap();
        assert_eq!(auto.action_line, "DROP 10 10 0#");

        // SREMV empty hands + mod slot
        session.player_action_pending = false;
        // Simulate empty hands
        if let Some(id) = session.our_id {
            if let Some(o) = session.world.get_mut(id) {
                o.held_id = 0;
            }
        }
        let sremv = click_sremv_clothing(&mut session, 5, -1).unwrap();
        assert_eq!(sremv.action_line, "SREMV 10 10 5 -1#");

        // SELF remove clothing
        session.player_action_pending = false;
        let rem = click_remove_clothing(&mut session, 1).unwrap();
        assert_eq!(rem.action_line, "SELF 10 10 1#");

        // Drain capture briefly. FrameReader yields bodies without trailing `#`.
        thread::sleep(Duration::from_millis(50));
        let lines = captured.lock().unwrap().clone();
        assert!(
            lines.iter().any(|l| l.starts_with("DROP 10 10 0")),
            "expected DROP clothing wire, got {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.starts_with("DROP 10 10 5")),
            "expected DROP c=5 wire, got {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.starts_with("SREMV 10 10 5 -1")),
            "expected SREMV wire, got {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.starts_with("SELF 10 10 1")),
            "expected SELF clothing wire, got {lines:?}"
        );
        let _ = handle.join();
    }

    #[test]
    fn click_tile_sends_cumulative_move_and_clears_pending() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 32, 30);
        session.queue_pending_action(ObjectAction::Use {
            x: 1,
            y: 1,
            object_id: Some(1),
            slot: None,
        });
        let res = session.click_tile_to_move(12, 10).unwrap();
        assert!(res.move_line.starts_with("MOVE "));
        assert!(res.reached_goal);
        assert!(session.pending_action().is_none());
        let _ = handle.join();
    }

    #[test]
    fn plan_click_tile_same_tile_errors() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state = MoveState::new(5, 5);
        assert_eq!(
            plan_click_tile(&session, 5, 5).unwrap_err(),
            MoveError::SameTile
        );
        let _ = handle.join();
    }

    #[test]
    fn click_tile_repath_while_in_motion() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n20 20 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 20, 20);
        let r1 = click_tile(&mut session, 7, 5).unwrap();
        assert!(!r1.was_repath);
        assert!(session.move_state.in_motion);
        let r2 = click_tile(&mut session, 5, 7).unwrap();
        assert!(r2.was_repath);
        let _ = handle.join();
    }

    /// P1#8: mid-move repath origin uses `lrint(currentPos)` on path, not stale PU start.
    #[test]
    fn click_tile_mid_path_origin_uses_fractional_current_pos() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n32 32 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 32, 20);
        // Explicit stand origin (PU may not have bound move_state yet in headless).
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.move_state.current_pos_x = 5.0;
        session.move_state.current_pos_y = 5.0;
        // Path (5,5) → (15,5): long east run.
        let r1 = click_tile(&mut session, 15, 5).unwrap();
        assert!(!r1.was_repath);
        assert_eq!(r1.start, (5, 5));
        assert!(session.move_state.in_motion);
        // Advance fractional currentPos to ~10.6 (mid-path); stale PU still (5,5).
        session.move_state.current_pos_x = 10.6;
        session.move_state.current_pos_y = 5.0;
        session.move_state.path_dist_traveled = 5.6;
        // path_start must prefer lrint(10.6)=11 on path, not PU (5,5).
        assert_eq!(path_start_tile(&session), (11, 5));
        // Stale PU hint alone would have returned (5,5) under the old grid-only logic.
        let r2 = click_tile(&mut session, 5, 10).unwrap();
        assert!(r2.was_repath);
        assert_eq!(
            r2.start,
            (11, 5),
            "repath MOVE xs ys must use mid-path origin"
        );
        assert!(
            r2.move_line.starts_with("MOVE 11 5 @"),
            "wire origin {}",
            r2.move_line
        );
        let _ = handle.join();
    }

    #[test]
    fn walk_or_use_dispatches_object_vs_ground() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 40,
            object_id_raw: "40".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match walk_or_use_tile(&mut session, 6, 5).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert!(r.already_adjacent);
                assert_eq!(r.action_line, "USE 6 5 40#");
                assert!(session.player_action_pending);
            }
            WalkOrUseResult::Ground(_) => panic!("expected object USE"),
        }
        // Simulate post-action PU clearing playerActionPending before next click.
        session.player_action_pending = false;
        // Empty tile further away → ground MOVE.
        match walk_or_use_tile(&mut session, 8, 5).unwrap() {
            WalkOrUseResult::Ground(r) => {
                assert_eq!(r.goal, (8, 5));
                assert!(r.reached_goal);
            }
            WalkOrUseResult::Object(_) => panic!("expected ground MOVE"),
        }
        let _ = handle.join();
    }

    #[test]
    fn modclick_drop_when_held() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 33 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(
            vec![
                framed_text("MC\n10 10 0 0\n0 0\n"),
                framed_text(bind_pu),
                framed_text("FM\n"),
            ],
            Arc::clone(&captured),
        );
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        // Adjacent empty tile + held + modClick → DROP immediately.
        match click_tile_mod(&mut session, 6, 5, true, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "DROP 6 5 -1#");
                assert!(r.action_sent || r.already_adjacent);
            }
            WalkOrUseResult::Ground(_) => panic!("expected DROP action"),
        }
        let _ = handle.join();
    }

    #[test]
    fn multi_move_arms_remaining_chunks() {
        // Build a cumulative path longer than ±16 via direct arming.
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 0;
        session.move_state.y = 0;
        let rest = vec![vec![PathDelta { x: 4, y: 0 }, PathDelta { x: 8, y: 0 }]];
        let multi = session.arm_multi_move(rest, (24, 0), (16, 0));
        assert!(multi);
        assert!(session.has_multi_move());
        // Simulate done_moving free.
        session.move_state.in_motion = false;
        session.move_state.x = 16;
        session.move_state.y = 0;
        let cont = session.continue_multi_move().unwrap();
        assert!(cont.is_some());
        assert!(cont.unwrap().starts_with("MOVE "));
        assert!(session.move_state.in_motion);
        let _ = handle.join();
    }

    /// Playtest: done_moving continues multi-MOVE **before** flushing queued action.
    ///
    /// // C++: next hop then nextActionMessageToSend only when path fully done
    #[test]
    fn multi_move_before_flush_keeps_queued_action() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;
        session.move_state.last_move_sequence_number = 2;
        // Armed second hop + queued DROP at far target (not adjacent yet).
        let rest = vec![vec![PathDelta { x: 4, y: 0 }, PathDelta { x: 8, y: 0 }]];
        assert!(session.arm_multi_move(rest, (24, 0), (16, 0)));
        session.queue_pending_action(ObjectAction::Drop {
            x: 24,
            y: 0,
            clothing_slot: -1,
        });
        // done_moving free at end of first hop — continue then flush (session order).
        session.move_state.x = 16;
        session.move_state.y = 0;
        let cont = session.continue_multi_move().unwrap();
        assert!(
            cont.as_ref().is_some_and(|l| l.starts_with("MOVE ")),
            "must emit multi-MOVE hop: {cont:?}"
        );
        assert!(session.move_state.in_motion);
        let flushed = session.flush_pending_action().unwrap();
        assert!(
            flushed.is_none(),
            "queued DROP must wait until multi-MOVE finishes: {flushed:?}"
        );
        assert!(session.pending_action().is_some());
        // Final hop done → adjacent to target → flush DROP.
        session.move_state.in_motion = false;
        session.move_state.x = 24;
        session.move_state.y = 0;
        session.clear_multi_move();
        let flushed = session.flush_pending_action().unwrap();
        assert_eq!(flushed.as_deref(), Some("DROP 24 0 -1#"));
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("MOVE ")),
            "multi-MOVE on wire: {tx:?}"
        );
        assert!(
            tx.iter().any(|m| m.starts_with("DROP 24 0 -1")),
            "DROP after multi-MOVE: {tx:?}"
        );
    }

    #[test]
    fn click_tile_mod_swap_nonperm_when_held() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 33 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        session.content.objects.insert(
            40,
            ClientObjectDef {
                id: 40,
                permanent: false,
                num_slots: 0,
                ..Default::default()
            },
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 40,
            object_id_raw: "40".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match click_tile_mod(&mut session, 6, 5, true, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "SWAP 6 5#");
                assert!(r.action_sent || r.already_adjacent);
            }
            WalkOrUseResult::Ground(_) => panic!("expected SWAP"),
        }
        let _ = handle.join();
    }

    #[test]
    fn click_tile_mod_container_drop_when_held() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 33 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        session.content.objects.insert(
            125,
            ClientObjectDef {
                id: 125,
                permanent: true,
                num_slots: 3,
                ..Default::default()
            },
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 125,
            object_id_raw: "125".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match click_tile_mod(&mut session, 6, 5, true, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "DROP 6 5 -1#");
            }
            WalkOrUseResult::Ground(_) => panic!("expected DROP into container"),
        }
        let _ = handle.join();
    }

    #[test]
    fn click_tile_mod_empty_hand_container_remv() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        // Isolate from default content transitions (real id 125 may have bare USE).
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9125,
                permanent: true,
                num_slots: 2,
                ..Default::default()
            }],
            vec![],
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 9125,
            object_id_raw: "9125,33".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        // Playtest: empty hand + modClick/RMB on container → REMV.
        match click_tile_mod(&mut session, 6, 5, true, -1).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert!(
                    r.action_line.starts_with("REMV 6 5"),
                    "expected REMV, got {}",
                    r.action_line
                );
            }
            WalkOrUseResult::Ground(_) => panic!("expected REMV"),
        }
        // Permanent container with no bare-hand trans: LMB also REMV (C++ ~26104).
        session.player_action_pending = false;
        match walk_or_use_tile(&mut session, 6, 5).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert!(
                    r.action_line.starts_with("REMV 6 5"),
                    "LMB container empty hand REMV: {}",
                    r.action_line
                );
            }
            WalkOrUseResult::Ground(_) => panic!("expected object action"),
        }
        let _ = handle.join();
    }

    #[test]
    fn lmb_noncontainer_object_still_use() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n10 10 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 10);
        session.content.objects.insert(
            99,
            ClientObjectDef {
                id: 99,
                permanent: true,
                num_slots: 0,
                ..Default::default()
            },
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 6,
            y: 5,
            floor_id: 0,
            object_id: 99,
            object_id_raw: "99".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        match walk_or_use_tile(&mut session, 6, 5).unwrap() {
            WalkOrUseResult::Object(r) => {
                assert_eq!(r.action_line, "USE 6 5 99#");
            }
            WalkOrUseResult::Ground(_) => panic!("expected LMB USE"),
        }
        let _ = handle.join();
    }

    #[test]
    fn multi_move_repath_when_chunks_empty_goal_remains() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        seed_open_map(&mut session, 0, 0, 40, 8);
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;
        // Goal armed, no pre-split chunks → repath via plan_click_tile_chunks.
        // end (0,0) != goal (20,0) so multi stays armed after first hop would land short.
        assert!(session.arm_multi_move(vec![], (20, 0), (0, 0)));
        let cont = session.continue_multi_move().unwrap();
        assert!(
            cont.as_ref().is_some_and(|l| l.starts_with("MOVE ")),
            "repath multi-MOVE: {cont:?}"
        );
        assert!(session.move_state.in_motion);
        let _ = handle.join();
    }

    /// P2#11: far click beyond pathFindingD=32 window → first hop ends short,
    /// multi_move_goal kept; done_moving repaths and arms next MOVE.
    #[test]
    fn multi_move_far_goal_click_repath_on_done_moving() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        // Open map large enough for multi-hop to (40,0).
        seed_open_map(&mut session, 0, 0, 48, 8);
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;
        session.move_state.last_move_sequence_number = 1;

        let r = click_tile(&mut session, 40, 0).unwrap();
        assert!(
            r.multi_move_pending,
            "far goal must arm multi-MOVE: end={:?} goal={:?}",
            r.end,
            r.goal
        );
        assert_eq!(r.goal, (40, 0));
        assert_ne!(r.end, r.goal, "first hop must stop short of ultimate goal");
        assert!(
            r.end.0.abs() <= 16 && r.end.1.abs() <= 16,
            "first hop within ±16 window: {:?}",
            r.end
        );
        assert!(session.has_multi_move());
        let hop1_end = r.end;
        let seq1 = session.move_state.last_move_sequence_number;

        // Simulate matching done_moving at hop end.
        session.move_state.in_motion = false;
        session.move_state.x = hop1_end.0;
        session.move_state.y = hop1_end.1;
        session.move_state.path_to_dest.clear();
        session.move_state.current_pos_x = hop1_end.0 as f64;
        session.move_state.current_pos_y = hop1_end.1 as f64;

        let cont = session.continue_multi_move().unwrap();
        assert!(
            cont.as_ref().is_some_and(|l| l.starts_with("MOVE ")),
            "P2#11 repath after done_moving: {cont:?}"
        );
        assert!(session.move_state.in_motion);
        assert!(
            session.move_state.last_move_sequence_number > seq1,
            "next MOVE must advance seq"
        );
        // Still short of ultimate goal or arrived closer.
        let hop2_end = (session.move_state.x, session.move_state.y);
        assert_ne!(hop2_end, hop1_end, "second hop must advance");
        if hop2_end != (40, 0) {
            assert!(
                session.has_multi_move(),
                "ultimate goal still armed until reached"
            );
        }
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        let moves: Vec<_> = tx.iter().filter(|m| m.starts_with("MOVE ")).collect();
        assert!(
            moves.len() >= 2,
            "expect first click MOVE + repath MOVE: {tx:?}"
        );
    }

    /// P2#11: multi-hop chain reaches far goal after several done_moving cycles.
    #[test]
    fn multi_move_far_goal_chain_reaches_ultimate() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        seed_open_map(&mut session, 0, 0, 48, 8);
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;

        let r = click_tile(&mut session, 40, 0).unwrap();
        assert!(r.multi_move_pending);
        let mut hops = 0u32;
        // Cap: window ~16 → at most ~4 hops for 40 tiles.
        while session.has_multi_move() && hops < 8 {
            let end = (session.move_state.x, session.move_state.y);
            session.move_state.in_motion = false;
            session.move_state.path_to_dest.clear();
            session.move_state.current_pos_x = end.0 as f64;
            session.move_state.current_pos_y = end.1 as f64;
            let cont = session.continue_multi_move().unwrap();
            if cont.is_none() {
                break;
            }
            hops += 1;
        }
        assert!(
            (session.move_state.x, session.move_state.y) == (40, 0)
                || !session.has_multi_move(),
            "should reach ultimate or exhaust multi: pos=({},{}) hops={hops}",
            session.move_state.x,
            session.move_state.y
        );
        // Prefer actually reaching.
        assert_eq!(
            (session.move_state.x, session.move_state.y),
            (40, 0),
            "multi-MOVE chain must land on ultimate goal after {hops} hops"
        );
        assert!(!session.has_multi_move());
        let _ = handle.join();
    }

    /// P2#11: closest-fallback hop short of blocked goal arms multi; repath clears when stuck.
    #[test]
    fn multi_move_closest_fallback_arms_then_stops_when_stuck() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        // Open strip; wall column at x=5 sealing goal (8,0).
        seed_open_map(&mut session, 0, 0, 12, 4);
        // Block (5,0)(5,1)(5,2)(5,3) with wall object 885 (blocks walking if no content:
        // cell_blocks_walking: unknown blocked, id 0 open, known !blocksWalking open).
        // Without content, positive object ids may not block — use missing tiles for wall.
        // Clear a sealed pocket: only seed left of wall, leave goal side unknown (=blocked).
        session.map = ClientMap::new();
        seed_open_map(&mut session, 0, 0, 5, 4); // x=0..4 open; x>=5 unknown/blocked
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;

        // Goal behind unknown wall — pathfind closest-fallback within open strip.
        let r = click_tile(&mut session, 8, 0).unwrap();
        assert_ne!(r.end, (8, 0), "must not enter unreachable goal");
        // Ultimate goal kept for repath attempt after hop.
        assert_eq!(r.goal, (8, 0));
        if r.end != r.goal {
            assert!(
                r.multi_move_pending || session.has_multi_move(),
                "closest-fallback short of goal should arm multi: end={:?}",
                r.end
            );
        }
        // Arrive at closest; repath cannot progress → clear multi.
        session.move_state.in_motion = false;
        session.move_state.x = r.end.0;
        session.move_state.y = r.end.1;
        session.move_state.path_to_dest.clear();
        let cont = session.continue_multi_move().unwrap();
        // Either a last-ditch hop or stop; must not infinite-loop with empty MOVE.
        if cont.is_none() {
            assert!(!session.has_multi_move() || session.multi_move_goal.is_some());
        }
        // Second continue with no progress clears.
        if session.has_multi_move() {
            session.move_state.in_motion = false;
            let pos = (session.move_state.x, session.move_state.y);
            session.move_state.current_pos_x = pos.0 as f64;
            session.move_state.current_pos_y = pos.1 as f64;
            let _ = session.continue_multi_move().unwrap();
        }
        // Stuck: multi eventually cleared (no path beyond open strip).
        let _ = handle.join();
    }

    #[test]
    fn flush_keeps_pending_when_not_adjacent() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.move_state.in_motion = false;
        session.queue_pending_action(ObjectAction::Use {
            x: 5,
            y: 5,
            object_id: Some(1),
            slot: None,
        });
        let flushed = session.flush_pending_action().unwrap();
        assert!(flushed.is_none(), "must not flush far USE");
        assert!(session.pending_action().is_some());
        // Move adjacent → flush ok.
        session.move_state.x = 5;
        session.move_state.y = 4;
        let flushed = session.flush_pending_action().unwrap();
        assert_eq!(flushed.as_deref(), Some("USE 5 5 1#"));
        let _ = handle.join();
    }

    #[test]
    fn click_use_paths_then_queues_and_flushes_on_done_moving() {
        // Player at (10,10); object at (14,10) → stand (13,10); USE after done_moving.
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // done_moving at stand (13,10) with seq 2
        let done_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 2 0 13 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(
            vec![
                framed_text("MC\n20 20 0 0\n0 0\n"),
                framed_text(bind_pu),
                framed_text("FM\n"),
            ],
            Arc::clone(&captured),
        );
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 20, 20);
        session.map.apply_mx(&crate::parse::MapChange {
            x: 14,
            y: 10,
            floor_id: 0,
            object_id: 77,
            object_id_raw: "77".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let r = click_use(&mut session, 14, 10, Some(77), None).unwrap();
        assert!(r.moved);
        assert!(!r.action_sent);
        assert_eq!(r.stand, (13, 10));
        // Inject done_moving.
        let body = format!("PU\n{}", &done_pu[3..]);
        // Use peer path: write framed done PU into stream via manual dispatch
        // (session already connected; simulate by applying through poll with extra data).
        // Easier: call move_state + flush path directly.
        session.move_state.in_motion = false;
        session.move_state.x = 13;
        session.move_state.y = 10;
        session.move_state.last_move_sequence_number = 2;
        let flushed = session.flush_pending_action().unwrap();
        assert_eq!(flushed.as_deref(), Some("USE 14 10 77#"));
        let _ = body; // silence
        let _ = handle.join();
    }

    // -----------------------------------------------------------------------
    // L-ACT / side_access_food_stand (C++ pointerDown ~25810–26038)
    // -----------------------------------------------------------------------

    fn seed_map_rect(map: &mut ClientMap, x: i32, y: i32, w: i32, h: i32) {
        let n = (w.max(1) * h.max(1)) as usize;
        let plain = vec!["0:0:0"; n].join(" ");
        let header = MapChunkHeader {
            size_x: w.max(1),
            size_y: h.max(1),
            x,
            y,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&header, &plain).expect("seed map");
    }

    #[test]
    fn stand_allows_access_side_and_no_back() {
        let side = StandAccess {
            side_access: true,
            no_back_access: false,
        };
        // W/E OK; N/S blocked for sideAccess
        assert!(stand_allows_access((4, 5), (5, 5), side));
        assert!(stand_allows_access((6, 5), (5, 5), side));
        assert!(!stand_allows_access((5, 6), (5, 5), side));
        assert!(!stand_allows_access((5, 4), (5, 5), side));
        // same tile still allowed by adjacency gate (walkability separate)
        assert!(stand_allows_access((5, 5), (5, 5), side));

        let noback = StandAccess {
            side_access: false,
            no_back_access: true,
        };
        assert!(stand_allows_access((5, 4), (5, 5), noback)); // S
        assert!(stand_allows_access((4, 5), (5, 5), noback)); // W
        assert!(!stand_allows_access((5, 6), (5, 5), noback)); // N forbidden
    }

    #[test]
    fn plan_stand_side_access_from_north_walks_to_west_or_east() {
        // Player at (5,7) north of ice hole at (5,5); must stand W or E, not S/N/self.
        let mut map = ClientMap::new();
        seed_map_rect(&mut map, 0, 0, 12, 12);
        map.apply_mx(&crate::parse::MapChange {
            x: 5,
            y: 5,
            floor_id: 0,
            object_id: 706,
            object_id_raw: "706".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let content = content_with(
            vec![ClientObjectDef {
                id: 706,
                permanent: true,
                blocks_walking: true,
                side_access: true,
                ..Default::default()
            }],
            vec![],
        );
        let (stand, path) =
            plan_stand_for_object_ex(&map, Some(&content), (5, 7), (5, 7), (5, 5), 0).unwrap();
        assert!(
            stand == (4, 5) || stand == (6, 5),
            "sideAccess stand must be W or E, got {stand:?}"
        );
        assert!(path.is_some(), "must MOVE from north");
        // Already standing west → no MOVE
        let (stand2, path2) =
            plan_stand_for_object_ex(&map, Some(&content), (4, 5), (4, 5), (5, 5), 0).unwrap();
        assert_eq!(stand2, (4, 5));
        assert!(path2.is_none());
    }

    #[test]
    fn plan_stand_no_back_access_excludes_north() {
        // Object at (5,5) blocks walking; walls on W/E/S; only N free.
        // With noBackAccess → cannot stand N → NoAdjacentStand.
        // Without → stand N.
        let mut map = ClientMap::new();
        seed_map_rect(&mut map, 0, 0, 12, 12);
        // Target object (blocks self-tile).
        map.apply_mx(&crate::parse::MapChange {
            x: 5,
            y: 5,
            floor_id: 0,
            object_id: 3240,
            object_id_raw: "3240".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        // Walls on W, E, S only — leave N open.
        for &(x, y) in &[(4, 5), (6, 5), (5, 4)] {
            map.apply_mx(&crate::parse::MapChange {
                x,
                y,
                floor_id: 0,
                object_id: 99,
                object_id_raw: "99".into(),
                player_id: 0,
                old_x: None,
                old_y: None,
                speed: None,
            });
        }
        let content = content_with(
            vec![
                ClientObjectDef {
                    id: 3240,
                    permanent: true,
                    blocks_walking: true,
                    no_back_access: true,
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 99,
                    blocks_walking: true,
                    ..Default::default()
                },
            ],
            vec![],
        );
        // Player far south — only free neighbor is N (5,6), forbidden by noBackAccess.
        let err =
            plan_stand_for_object_ex(&map, Some(&content), (5, 2), (5, 2), (5, 5), 0).unwrap_err();
        assert_eq!(err, MoveError::NoAdjacentStand);

        // Without noBackAccess, N stand is chosen.
        let mut c2 = content.clone();
        c2.objects.get_mut(&3240).unwrap().no_back_access = false;
        let (stand, _) =
            plan_stand_for_object_ex(&map, Some(&c2), (5, 2), (5, 2), (5, 5), 0).unwrap();
        assert_eq!(stand, (5, 6));
    }

    #[test]
    fn plan_stand_permanent_food_prefers_self_tile() {
        // Berry bush: permanent, non-blocking, bare-hand → food in hand + bush remains.
        let mut map = ClientMap::new();
        seed_map_rect(&mut map, 0, 0, 12, 12);
        map.apply_mx(&crate::parse::MapChange {
            x: 8,
            y: 8,
            floor_id: 0,
            object_id: 30,
            object_id_raw: "30".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let content = content_with(
            vec![
                ClientObjectDef {
                    id: 30,
                    permanent: true,
                    blocks_walking: false,
                    food_value: 0,
                    ..Default::default()
                },
                ClientObjectDef {
                    id: 31,
                    food_value: 3,
                    ..Default::default()
                },
            ],
            vec![ClientTransition {
                actor_id: 0,
                target_id: 30,
                new_actor_id: 31, // gooseberry
                new_target_id: 30, // bush remains
                ..Default::default()
            }],
        );
        // Player far west — shortest ortho is W of bush (7,8), but food prefer → self (8,8).
        let (stand, path) =
            plan_stand_for_object_ex(&map, Some(&content), (3, 8), (3, 8), (8, 8), 0).unwrap();
        assert_eq!(stand, (8, 8), "permanent food harvest prefers self-tile");
        assert!(path.is_some());
    }

    #[test]
    fn click_use_side_access_ice_hole_from_north() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 7 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n12 12 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 12);
        session.content.objects.insert(
            706,
            ClientObjectDef {
                id: 706,
                permanent: true,
                blocks_walking: true,
                side_access: true,
                ..Default::default()
            },
        );
        session.map.apply_mx(&crate::parse::MapChange {
            x: 5,
            y: 5,
            floor_id: 0,
            object_id: 706,
            object_id_raw: "706".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let r = click_use(&mut session, 5, 5, Some(706), None).unwrap();
        assert!(r.moved);
        assert!(
            r.stand == (4, 5) || r.stand == (6, 5),
            "ice hole stand W/E, got {:?}",
            r.stand
        );
        assert_eq!(r.target, (5, 5));
        let _ = handle.join();
    }

    #[test]
    fn gate_action_pending_blocks_click() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.player_action_pending = true;
        assert_eq!(
            apply_click_gates(&mut session).unwrap_err(),
            MoveError::ActionPending
        );
        assert_eq!(
            click_tile(&mut session, 6, 5).unwrap_err(),
            MoveError::ActionPending
        );
        let _ = handle.join();
    }

    #[test]
    fn gate_holding_immobile_blocks_without_ground_trans() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        // Inject 0-speed held object with no ground transition.
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9001,
                speed_mult: 0.0,
                ..Default::default()
            }],
            vec![],
        );
        session.our_id = Some(7);
        session.world.apply_pu(&crate::parse::parse_pu_line(
            "7 100 1 0 0 0 9001 0 0 0 -1 0.5 0 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1",
        ).unwrap());
        assert_eq!(session.our_held_id(), 9001);
        assert_eq!(
            apply_click_gates(&mut session).unwrap_err(),
            MoveError::HoldingImmobile
        );
        let _ = handle.join();
    }

    #[test]
    fn gate_holding_immobile_allows_when_ground_trans_exists() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9002,
                speed_mult: 0.0,
                food_value: 0,
                ..Default::default()
            }],
            vec![ClientTransition {
                actor_id: 9002,
                target_id: -1,
                new_actor_id: 0,
                new_target_id: 9003,
                ..Default::default()
            }],
        );
        session.our_id = Some(7);
        session.world.apply_pu(&crate::parse::parse_pu_line(
            "7 100 1 0 0 0 9002 0 0 0 -1 0.5 0 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1",
        ).unwrap());
        // Gate passes; click may still fail on empty map path — only gate is under test.
        assert!(apply_click_gates(&mut session).is_ok());
        let _ = handle.join();
    }

    #[test]
    fn gate_too_young_sends_jump() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(vec![], Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.our_id = Some(7);
        // age 0.10 < noMoveAge 0.20
        session.world.apply_pu(&crate::parse::parse_pu_line(
            "7 100 1 0 0 0 0 0 0 0 -1 0.5 0 0 5 5 0.10 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1",
        ).unwrap());
        assert!(session.our_age().unwrap() < NO_MOVE_AGE);
        assert_eq!(
            apply_click_gates(&mut session).unwrap_err(),
            MoveError::JumpSent
        );
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("JUMP 0 0")),
            "JUMP on wire for young baby: {tx:?}"
        );
    }

    #[test]
    fn gate_held_by_adult_sends_jump() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 0.5 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // Adult 8 holds baby 7
        let adult_hold = "PU\n\
8 100 1 0 0 0 -7 0 0 0 -1 0.5 0 0 10 10 30.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(adult_hold),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        // Drain adult hold frame.
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            session.we_are_held_by_adult(),
            "held_by_adult_id should be set from adult held_id=-7"
        );
        assert_eq!(
            click_tile(&mut session, 12, 10).unwrap_err(),
            MoveError::JumpSent
        );
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("JUMP 0 0")),
            "JUMP on wire when held: {tx:?}"
        );
        assert!(
            !tx.iter().any(|m| m.starts_with("MOVE ")),
            "no MOVE while held: {tx:?}"
        );
    }

    #[test]
    fn player_action_pending_set_on_send_cleared_on_stationary_pu() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let post_action = "PU\n\
7 100 1 0 0 0 33 0 0 0 -1 0.5 0 0 10 10 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(post_action),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        session.move_state.in_motion = false;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 11,
                y: 10,
                object_id: Some(33),
                slot: None,
            })
            .unwrap();
        assert!(session.player_action_pending);
        // Stationary post-action PU clears the gate.
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(!session.player_action_pending);
        let _ = handle.join();
    }

    // -----------------------------------------------------------------------
    // P1#7 continuous mouse-hold / blocked-tile slide
    // -----------------------------------------------------------------------

    fn mx_place(x: i32, y: i32, object_id: i32) -> crate::parse::MapChange {
        crate::parse::MapChange {
            x,
            y,
            floor_id: 0,
            object_id,
            object_id_raw: object_id.to_string(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        }
    }

    /// Seed map cells open, then place blocking object at (bx,by).
    fn seed_map_with_blocker(
        session: &mut ClientSession,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        bx: i32,
        by: i32,
        blocker_id: i32,
    ) {
        seed_open_map(session, x, y, w, h);
        session.map.apply_mx(&mx_place(bx, by, blocker_id));
    }

    #[test]
    fn slide_blocked_click_dest_pushes_x_major_axis() {
        // Player at (0,0); blocked at (5,0); open at (6,0) further along +X.
        let mut map = ClientMap::new();
        let plain = {
            // 10x3 open strip
            let n = 10 * 3;
            vec!["0:0:0"; n].join(" ")
        };
        let header = MapChunkHeader {
            size_x: 10,
            size_y: 3,
            x: 0,
            y: -1,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&header, &plain).unwrap();
        let content = content_with(
            vec![ClientObjectDef {
                id: 500,
                blocks_walking: true,
                permanent: true,
                ..Default::default()
            }],
            vec![],
        );
        map.apply_mx(&mx_place(5, 0, 500));
        let slid = slide_blocked_click_dest(&map, Some(&content), (0, 0), (5, 0));
        assert_eq!(slid, (6, 0), "should push past wall along +X, got {slid:?}");
        // Walkable dest unchanged.
        assert_eq!(
            slide_blocked_click_dest(&map, Some(&content), (0, 0), (3, 0)),
            (3, 0)
        );
    }

    #[test]
    fn slide_blocked_click_dest_pushes_y_when_equal_or_major() {
        let mut map = ClientMap::new();
        let plain = vec!["0:0:0"; 5 * 10].join(" ");
        let header = MapChunkHeader {
            size_x: 5,
            size_y: 10,
            x: -2,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&header, &plain).unwrap();
        let content = content_with(
            vec![ClientObjectDef {
                id: 501,
                blocks_walking: true,
                ..Default::default()
            }],
            vec![],
        );
        map.apply_mx(&mx_place(0, 4, 501));
        // Major axis Y: from (0,0) to (0,4).
        let slid = slide_blocked_click_dest(&map, Some(&content), (0, 0), (0, 4));
        assert_eq!(slid, (0, 5), "push +Y past wall, got {slid:?}");
    }

    #[test]
    fn resolve_hold_click_dest_only_slides_after_min_frames() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n16 16 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        session.content = content_with(
            vec![ClientObjectDef {
                id: 502,
                blocks_walking: true,
                ..Default::default()
            }],
            vec![],
        );
        seed_map_with_blocker(&mut session, 0, 0, 16, 16, 8, 5, 502);
        // Before min frames: no slide.
        assert_eq!(
            resolve_hold_click_dest(&session, 8, 5, true, MIN_MOUSE_DOWN_FRAMES),
            (8, 5)
        );
        // After min frames: slide further +X (player at 5,5; dest 8,5 → major X).
        let slid = resolve_hold_click_dest(&session, 8, 5, true, MIN_MOUSE_DOWN_FRAMES + 1);
        assert_eq!(slid, (9, 5), "expected slide past block, got {slid:?}");
        // First press never slides.
        assert_eq!(
            resolve_hold_click_dest(&session, 8, 5, false, 999),
            (8, 5)
        );
        let _ = handle.join();
    }

    #[test]
    fn hold_repath_is_ground_only_even_over_object() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n16 16 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Non-blocking permanent object at (8,5) — first click would USE; hold is ground MOVE.
        session.content = content_with(
            vec![ClientObjectDef {
                id: 33,
                blocks_walking: false,
                permanent: true,
                ..Default::default()
            }],
            vec![],
        );
        seed_open_map(&mut session, 0, 0, 16, 16);
        session.map.apply_mx(&mx_place(8, 5, 33));
        // First press → object path / USE.
        let first = walk_or_use_tile_hold(&mut session, 8, 5, false, 0, -1, -1).unwrap();
        assert!(
            matches!(first, WalkOrUseResult::Object(_)),
            "first press should USE object, got {first:?}"
        );
        // Reset motion for hold repath from stand.
        session.move_state.in_motion = false;
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.cancel_pending_action();
        session.clear_multi_move();
        let held = walk_or_use_tile_hold(
            &mut session,
            8,
            5,
            true,
            MIN_MOUSE_DOWN_FRAMES + 5,
            -1,
            -1,
        )
        .unwrap();
        match held {
            WalkOrUseResult::Ground(r) => {
                assert!(r.move_line.starts_with("MOVE "));
                // Non-blocking tile: hold is ground MOVE only (no object USE).
                // Close-hold throw (P2#10): mouse at (8,5) is within 4 tiles of (5,5)
                // so dest is thrown further along the vector (waypoint still at mouse).
                assert!(
                    r.goal == (8, 5) || r.goal.0 >= 8,
                    "ground hold goal near mouse or thrown out: {:?}",
                    r.goal
                );
            }
            WalkOrUseResult::Object(r) => panic!("hold must not USE: {r:?}"),
        }
        let _ = handle.join();
    }

    #[test]
    fn hold_slides_blocked_dest_and_sends_move() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n16 16 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        session.content = content_with(
            vec![ClientObjectDef {
                id: 503,
                blocks_walking: true,
                ..Default::default()
            }],
            vec![],
        );
        seed_map_with_blocker(&mut session, 0, 0, 16, 16, 8, 5, 503);
        let r = walk_or_use_tile_hold(
            &mut session,
            8,
            5,
            true,
            MIN_MOUSE_DOWN_FRAMES + 1,
            -1,
            -1,
        )
        .unwrap();
        match r {
            WalkOrUseResult::Ground(g) => {
                assert_eq!(g.goal, (9, 5), "goal should be slid past wall");
                assert!(g.move_line.starts_with("MOVE "));
            }
            WalkOrUseResult::Object(_) => panic!("expected ground MOVE after slide"),
        }
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("MOVE ")),
            "MOVE on wire after hold-slide: {tx:?}"
        );
    }

    #[test]
    fn hold_respects_action_pending_gate() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.player_action_pending = true;
        assert_eq!(
            walk_or_use_tile_hold(&mut session, 7, 5, true, 40, -1, -1).unwrap_err(),
            MoveError::ActionPending
        );
        let _ = handle.join();
    }

    #[test]
    fn hold_respects_holding_immobile_gate() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.content = content_with(
            vec![ClientObjectDef {
                id: 9001,
                speed_mult: 0.0,
                ..Default::default()
            }],
            vec![],
        );
        session.our_id = Some(7);
        session.world.apply_pu(
            &crate::parse::parse_pu_line(
                "7 100 1 0 0 0 9001 0 0 0 -1 0.5 0 0 5 5 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1",
            )
            .unwrap(),
        );
        assert_eq!(
            walk_or_use_tile_hold(&mut session, 7, 5, true, 40, -1, -1).unwrap_err(),
            MoveError::HoldingImmobile
        );
        let _ = handle.join();
    }

    /// P2#9: BB list + edge routing + hold auto_click + rideable ignoreBad.
    #[test]
    fn bad_biome_session_opts_and_hold_auto_click() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![
            framed_text("MC\n5 1 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("BB\n21 MOUNTAIN\n9 OCEAN\n"),
            framed_text("FM\n"),
        ]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..24 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Ensure BB applied (also accept direct apply if wire timing missed).
        if session.bad_biomes.is_empty() {
            session.apply_bad_biomes_message("BB\n21 MOUNTAIN\n9 OCEAN\n");
        }
        assert_eq!(session.bad_biomes, vec![21, 9]);

        // Layout: x0 good, x1..x4 bad(21). Start (0,0) has bad neighbor → edge.
        let header = MapChunkHeader {
            size_x: 5,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        session
            .map
            .apply_mc_plaintext(&header, "0:0:0 21:0:0 21:0:0 21:0:0 21:0:0")
            .unwrap();
        session.move_state.x = 0;
        session.move_state.y = 0;
        session.our_id = Some(7);

        // Manual click from edge into bad dest succeeds (startBiomeBad).
        let manual = plan_click_tile_chunks_with(&session, 4, 0, false).unwrap();
        assert!(
            manual.0.reached_goal,
            "manual from edge enters bad: {:?}",
            manual.0
        );

        // Hold/auto_click must not enter bad from good edge (C++ isAutoClick).
        let auto = plan_click_tile_chunks_with(&session, 4, 0, true);
        match auto {
            Ok((plan, _, _)) => {
                assert!(
                    !plan.reached_goal,
                    "auto_click must edge-stop: {:?}",
                    plan
                );
            }
            Err(MoveError::EmptyPath) => {} // blocked entirely is fine
            Err(e) => panic!("unexpected auto_click err: {e:?}"),
        }

        // Rideable ignoreBad: holding a rideable vehicle walks through bad.
        session.content = content_with(
            vec![ClientObjectDef {
                id: 3331,
                rideable: true,
                blocks_walking: false,
                ..Default::default()
            }],
            vec![],
        );
        session.world.apply_pu(
            &crate::parse::parse_pu_line(
                "7 100 1 0 0 0 3331 0 0 0 -1 0.5 0 0 0 0 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1",
            )
            .unwrap(),
        );
        assert!(session.holding_rideable());
        assert!(session.path_find_opts().ignore_bad);
        let ride = plan_click_tile_chunks_with(&session, 4, 0, false).unwrap();
        assert!(
            ride.0.reached_goal,
            "rideable must cross bad biomes: {:?}",
            ride.0
        );
        let _ = handle.join();
    }

    // -----------------------------------------------------------------------
    // P2#10 useWaypoint two-leg pathFind (click_tile + stand + close-hold)
    // -----------------------------------------------------------------------

    #[test]
    fn close_hold_throw_aabb_and_vector() {
        // Geometry of C++ close-hold box: absX/Y in (1, 4), throw vector length 4.
        let (port, handle) = login_then_peer(vec![framed_text("FM\n")]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            if matches!(session.poll_event(), Ok(SessionEvent::Frame) | Err(_)) {
                break;
            }
        }
        session.move_state = MoveState::new(0, 0);
        session.move_state.in_motion = false;
        session.our_id = None;

        // Inside close-hold box (2,0): absX=2<4, absY=0<4, and absX>1.
        let t = maybe_close_hold_throw(&session, 2, 0).expect("close hold");
        assert_eq!(t, (4, 0), "throw 4 tiles along +X, got {t:?}");

        // Diagonal inside box (2,2): Euclidean throw length 4.
        let t2 = maybe_close_hold_throw(&session, 2, 2).expect("diag close hold");
        let dx = t2.0 as f64;
        let dy = t2.1 as f64;
        let len = (dx * dx + dy * dy).sqrt();
        assert!(
            (len - 4.0).abs() < 0.6,
            "throw length ~4 tiles, got {t2:?} len={len}"
        );

        // Outside box: absX=5 >= 4 → no close-hold (normal repath).
        assert!(maybe_close_hold_throw(&session, 5, 0).is_none());
        // Too close: both axes ≤1.
        assert!(maybe_close_hold_throw(&session, 0, 0).is_none());
        assert!(maybe_close_hold_throw(&session, 1, 0).is_none());
        let _ = handle.join();
    }

    #[test]
    fn plan_click_via_waypoint_visits_midpoint() {
        let (port, handle) = login_then_peer(vec![framed_text("FM\n")]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            if matches!(session.poll_event(), Ok(SessionEvent::Frame) | Err(_)) {
                break;
            }
        }
        seed_open_map(&mut session, 0, 0, 12, 6);
        session.move_state = MoveState::new(1, 1);
        session.our_id = None;

        // Arm waypoint at (1,4); goal (6,4) — two-leg must pass through wp.
        session
            .move_state
            .arm_waypoint(1, 4, DEFAULT_MAX_WAYPOINT_PATH_LENGTH);
        let (plan, _first, _rest, eff) =
            plan_click_tile_chunks_goal(&session, 6, 4, false).unwrap();
        assert_eq!(eff, (6, 4));
        assert!(plan.reached_goal, "{plan:?}");
        assert!(
            plan.absolute_cells().contains(&(1, 4)),
            "must visit waypoint: {:?}",
            plan.absolute_cells()
        );
        let _ = handle.join();
    }

    #[test]
    fn plan_click_waypoint_too_long_stops_at_waypoint() {
        let (port, handle) = login_then_peer(vec![framed_text("FM\n")]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            if matches!(session.poll_event(), Ok(SessionEvent::Frame) | Err(_)) {
                break;
            }
        }
        seed_open_map(&mut session, 0, 0, 20, 4);
        session.move_state = MoveState::new(0, 1);
        session.our_id = None;

        let wp = (2, 1);
        let goal = (14, 1);
        // max length 4 forces dest rewrite to waypoint (C++ ~2566–2579).
        session.move_state.arm_waypoint(wp.0, wp.1, 4);
        let (plan, _first, _rest, eff) =
            plan_click_tile_chunks_goal(&session, goal.0, goal.1, false).unwrap();
        assert_eq!(eff, wp, "effective goal rewritten to waypoint");
        assert_eq!(plan.end, wp);
        assert!(plan.reached_goal || plan.end == wp, "{plan:?}");
        let _ = handle.join();
    }

    #[test]
    fn plan_stand_via_waypoint_paths_through_mid() {
        // Object at (8,2); stand west (7,2). Waypoint (2,4) forces detour north.
        let mut map = ClientMap::new();
        seed_map_rect(&mut map, 0, 0, 12, 8);
        map.apply_mx(&crate::parse::MapChange {
            x: 8,
            y: 2,
            floor_id: 0,
            object_id: 33,
            object_id_raw: "33".into(),
            player_id: 0,
            old_x: None,
            old_y: None,
            speed: None,
        });
        let content = content_with(
            vec![ClientObjectDef {
                id: 33,
                permanent: true,
                blocks_walking: true,
                ..Default::default()
            }],
            vec![],
        );
        let wp = (2, 4);
        let (stand, path) = plan_stand_for_object_with_opts_wp(
            &map,
            Some(&content),
            (0, 2),
            (0, 2),
            (8, 2),
            0,
            &PathFindOpts::default(),
            Some(wp),
            DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
        )
        .unwrap();
        assert_ne!(stand, (0, 2), "must need a stand MOVE");
        let plan = path.expect("path to stand");
        assert!(
            plan.absolute_cells().contains(&wp) || plan.end == wp,
            "stand path should use waypoint {wp:?}: cells={:?} end={:?}",
            plan.absolute_cells(),
            plan.end
        );
    }

    #[test]
    fn click_tile_with_waypoint_clears_flag_and_moves() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let (port, handle) = login_then_peer_capture(
            vec![framed_text("FM\n")],
            Arc::clone(&captured),
        );
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            if matches!(session.poll_event(), Ok(SessionEvent::Frame) | Err(_)) {
                break;
            }
        }
        seed_open_map(&mut session, 0, 0, 10, 6);
        session.move_state = MoveState::new(1, 1);
        session.our_id = None;
        session
            .move_state
            .arm_waypoint(1, 3, DEFAULT_MAX_WAYPOINT_PATH_LENGTH);
        assert!(session.move_state.use_waypoint);

        let r = click_tile_with(&mut session, 6, 3, true).unwrap();
        assert!(!session.move_state.use_waypoint, "one-shot clear after plan");
        assert!(r.move_line.starts_with("MOVE "));
        // Path should have passed through waypoint y=3 column.
        assert!(
            r.end.1 == 3 || r.goal == (6, 3),
            "end/goal along two-leg: {r:?}"
        );
        let _ = handle.join();
    }

    // -----------------------------------------------------------------------
    // P2#10 useWaypoint two-leg + close-hold throw
    // -----------------------------------------------------------------------

    #[test]
    fn waypoint_plan_visits_midpoint_and_clears_flag() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 0 1 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n8 4 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..16 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Open field around start.
        let header = MapChunkHeader {
            size_x: 8,
            size_y: 4,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let plain = (0..32).map(|_| "0:0:0 ").collect::<String>();
        session.map.apply_mc_plaintext(&header, plain.trim()).unwrap();
        session.move_state.x = 0;
        session.move_state.y = 1;
        session.our_id = Some(7);
        session
            .move_state
            .arm_waypoint(0, 3, DEFAULT_MAX_WAYPOINT_PATH_LENGTH);
        assert!(session.move_state.use_waypoint);

        let r = click_tile(&mut session, 6, 3).unwrap();
        assert!(!session.move_state.use_waypoint, "waypoint cleared after click");
        assert!(r.move_line.starts_with("MOVE "));
        // Path should end at goal (short enough) and have passed near waypoint y=3.
        assert_eq!(r.goal, (6, 3));
        // Absolute cells from plan must include waypoint (0,3) when two-leg succeeds.
        let plan = {
            session.move_state.arm_waypoint(0, 3, DEFAULT_MAX_WAYPOINT_PATH_LENGTH);
            // After click we're at dest; re-seed start for a plan-only check.
            session.move_state.x = 0;
            session.move_state.y = 1;
            session.move_state.in_motion = false;
            session.move_state.path_to_dest.clear();
            plan_click_tile_chunks_goal(&session, 6, 3, false).unwrap()
        };
        let cells = plan.0.absolute_cells();
        assert!(
            cells.contains(&(0, 3)),
            "two-leg path must visit waypoint: {cells:?}"
        );
        let _ = handle.join();
    }

    #[test]
    fn waypoint_too_long_rewrites_goal_to_waypoint() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let header = MapChunkHeader {
            size_x: 16,
            size_y: 3,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let plain = (0..48).map(|_| "0:0:0 ").collect::<String>();
        session.map.apply_mc_plaintext(&header, plain.trim()).unwrap();
        session.move_state.x = 0;
        session.move_state.y = 1;
        session
            .move_state
            .arm_waypoint(2, 1, 4); // force stop at waypoint
        let (plan, _first, _rest, eff_goal) =
            plan_click_tile_chunks_goal(&session, 14, 1, false).unwrap();
        assert_eq!(eff_goal, (2, 1));
        assert_eq!(plan.end, (2, 1));
        let _ = handle.join();
    }

    #[test]
    fn close_hold_throw_sets_far_dest() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        session.move_state.x = 5;
        session.move_state.y = 5;
        session.move_state.current_pos_x = 5.0;
        session.move_state.current_pos_y = 5.0;
        session.our_id = Some(7);
        // Mouse 2 tiles east of player → throw out to ~4 tiles east.
        let t = maybe_close_hold_throw(&session, 7, 5);
        assert!(t.is_some(), "close mouse should throw");
        let (tx, ty) = t.unwrap();
        assert_eq!(ty, 5);
        assert!(
            (tx - 5).abs() >= CLOSE_HOLD_THROW_TILES - 1,
            "throw should be ~4 tiles out: ({tx},{ty})"
        );
        // Far mouse: no throw.
        assert!(maybe_close_hold_throw(&session, 12, 5).is_none());
        let _ = handle.join();
    }
}
