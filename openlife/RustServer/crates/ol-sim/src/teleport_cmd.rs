//! CURSED-GRAVE-TELEPORT / tcg_tv_teleport — pure helpers for Haxe
//! `GlobalPlayerInstance` teleport bang commands `!TCG`/`!CURSEDGRAVE` and
//! `!TV`/`!VILLAGE`, plus shared `teleport` closest-unblocked pick.
//!
//! Locations come from `WorldMapTimeState.cursed_graves` / `.ovens`
//! (filled by **CURSED-GRAVES-INDEX** map-slice). Live wire lives in `lib.rs`.
//!
//! // Haxe: GlobalPlayerInstance.doServerCommand !TV/!TCG + teleport + doTeleport

use crate::world_time::map_linear_index;

/// Haxe not-found text for empty cursed-grave index.
// Haxe: GlobalPlayerInstance !TCG `No graves found!`
pub const TCG_NOT_FOUND: &str = "No graves found!";

/// Haxe not-found text for empty oven/village index.
// Haxe: GlobalPlayerInstance !TV `No villages with an oven found!`
pub const TV_NOT_FOUND: &str = "No villages with an oven found!";

/// Haxe after all blocked locations exhausted (clears list on next try).
// Haxe: GlobalPlayerInstance.teleport `Tried all locations. Start again!`
pub const TELEPORT_ALL_TRIED: &str = "Tried all locations. Start again!";

/// Haxe `checkIfNotAllowed` private say when `canUseServerCommands == false`.
// Haxe: GlobalPlayerInstance.checkIfNotAllowed `not allowed!`
pub const TELEPORT_NOT_ALLOWED: &str = "not allowed!";

/// Parsed admin teleport bang (uppercase input expected).
// Haxe: GlobalPlayerInstance.doServerCommand !TV / !TCG branches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeleportBang {
    /// `!TCG` exact or text contains `!CURSEDGRAVE`.
    CursedGrave,
    /// `!TV` exact or text contains `!VILLAGE`.
    Village,
}

/// Result of closest-location pick for `teleport`.
// Haxe: GlobalPlayerInstance.teleport L5792-5825
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeleportPick {
    /// No locations in the index at all.
    Empty,
    /// Every candidate is in `blockedTeleportLocations` — clear and retry next SAY.
    AllBlocked,
    /// Closest unblocked: linear map index + absolute `(tx, ty)`.
    Found { index: i32, tx: i32, ty: i32 },
}

/// Parse `!TCG` / `!CURSEDGRAVE` / `!TV` / `!VILLAGE` from uppercase SAY text.
///
/// Matches Haxe:
/// - `text == '!TCG' || text.indexOf('!CURSEDGRAVE') != -1`
/// - `text == '!TV' || text.indexOf('!VILLAGE') != -1`
// Haxe: GlobalPlayerInstance.doServerCommand L5515 / L5577
pub fn parse_teleport_bang(upper: &str) -> Option<TeleportBang> {
    let t = upper.trim();
    if t.is_empty() {
        return None;
    }
    // Cursed grave first — both could theoretically appear; Haxe is sequential.
    if t == "!TCG" || t.contains("!CURSEDGRAVE") {
        return Some(TeleportBang::CursedGrave);
    }
    if t == "!TV" || t.contains("!VILLAGE") {
        return Some(TeleportBang::Village);
    }
    None
}

/// Not-found private-say text for a bang command.
#[inline]
pub fn not_found_text(cmd: TeleportBang) -> &'static str {
    match cmd {
        TeleportBang::CursedGrave => TCG_NOT_FOUND,
        TeleportBang::Village => TV_NOT_FOUND,
    }
}

/// Haxe `AiHelper.CalculateQuadDistanceToObject` without wrap (local maps).
#[inline]
pub fn teleport_quad_distance(px: i32, py: i32, tx: i32, ty: i32) -> f64 {
    teleport_quad_distance_ex(px, py, tx, ty, 0, 0, false)
}

/// Torus-aware squared Euclidean (Haxe `transformX/Y` then quad).
// Haxe: AiHelper.CalculateQuadDistanceToObject L65-69
// TCG-LIVE-WIRE
pub fn teleport_quad_distance_ex(
    px: i32,
    py: i32,
    tx: i32,
    ty: i32,
    mw: i32,
    mh: i32,
    wrap: bool,
) -> f64 {
    let (dx, dy) = ol_move_rules::wrap_delta(px, py, tx, ty, mw, mh, wrap);
    (dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)
}

/// Pick closest location not in `blocked` (Haxe linear `obj.index()` keys).
///
/// `locations`: `(linear_index, (tx, ty))` — same shape as cursed_graves/ovens maps.
// Haxe: GlobalPlayerInstance.teleport L5792-5821
pub fn pick_closest_teleport(
    px: i32,
    py: i32,
    locations: &[(i32, (i32, i32))],
    blocked: &[i32],
) -> TeleportPick {
    pick_closest_teleport_ex(px, py, locations, blocked, 0, 0, false)
}

/// Wrap-aware closest pick (live torus worlds).
// TCG-LIVE-WIRE
pub fn pick_closest_teleport_ex(
    px: i32,
    py: i32,
    locations: &[(i32, (i32, i32))],
    blocked: &[i32],
    mw: i32,
    mh: i32,
    wrap: bool,
) -> TeleportPick {
    if locations.is_empty() {
        return TeleportPick::Empty;
    }
    let mut best: Option<(f64, i32, i32, i32)> = None; // dist, index, tx, ty
    for &(index, (tx, ty)) in locations {
        if blocked.contains(&index) {
            continue;
        }
        let dist = teleport_quad_distance_ex(px, py, tx, ty, mw, mh, wrap);
        match best {
            None => best = Some((dist, index, tx, ty)),
            Some((bd, _, _, _)) if dist < bd => best = Some((dist, index, tx, ty)),
            _ => {}
        }
    }
    match best {
        None => TeleportPick::AllBlocked,
        Some((_, index, tx, ty)) => TeleportPick::Found { index, tx, ty },
    }
}

/// Convenience: locations from a `HashMap` linear-index store (order irrelevant).
pub fn pick_closest_from_index_map(
    px: i32,
    py: i32,
    map: &std::collections::HashMap<i32, (i32, i32)>,
    blocked: &[i32],
) -> TeleportPick {
    pick_closest_from_index_map_ex(px, py, map, blocked, 0, 0, false)
}

/// Wrap-aware HashMap pick.
// TCG-LIVE-WIRE
pub fn pick_closest_from_index_map_ex(
    px: i32,
    py: i32,
    map: &std::collections::HashMap<i32, (i32, i32)>,
    blocked: &[i32],
    mw: i32,
    mh: i32,
    wrap: bool,
) -> TeleportPick {
    let locs: Vec<(i32, (i32, i32))> = map.iter().map(|(&k, &v)| (k, v)).collect();
    pick_closest_teleport_ex(px, py, &locs, blocked, mw, mh, wrap)
}

/// Record a used location on the blocked list (Haxe push after pick).
#[inline]
pub fn push_blocked_teleport(blocked: &mut Vec<i32>, index: i32) {
    if !blocked.contains(&index) {
        blocked.push(index);
    }
}

/// Clear blocked list after all locations tried (Haxe reset + say).
#[inline]
pub fn clear_blocked_teleport(blocked: &mut Vec<i32>) {
    blocked.clear();
}

/// Build linear index for an absolute tile (same key as cursed_graves/ovens).
#[inline]
pub fn teleport_location_index(tx: i32, ty: i32, map_width: i32) -> i32 {
    map_linear_index(tx, ty, map_width)
}

/// True when this player may run `!TCG`/`!TV` (Haxe `canUseServerCommands`).
///
/// Godmode stands in when the account flag is unset.
// Haxe: GlobalPlayerInstance.checkIfNotAllowed
fn teleport_bang_allowed(state: &crate::SimState, conn_id: u64) -> bool {
    let Some(p) = state.players.get(&conn_id) else {
        return false;
    };
    if p.godmode {
        return true;
    }
    state
        .accounts
        .get(&p.email)
        .map(|a| a.can_use_server_commands)
        .unwrap_or(false)
}

/// Live `!TCG`/`!TV`/`!CURSEDGRAVE`/`!VILLAGE` — consume SAY when recognized.
///
/// VOG_UPDATE is parked; still wrap-pick, jump-to-non-blocked, MC, forced PU+FM.
// Haxe: GlobalPlayerInstance.doServerCommand !TV/!TCG + teleport + doTeleport
// TCG-LIVE-WIRE
pub fn try_apply_teleport_bang(
    state: &mut crate::SimState,
    outbound: &ol_net::OutboundHub,
    conn_id: u64,
    upper: &str,
) -> bool {
    let Some(cmd) = parse_teleport_bang(upper) else {
        return false;
    };
    if !teleport_bang_allowed(state, conn_id) {
        crate::send_ps_reply(outbound, conn_id, TELEPORT_NOT_ALLOWED);
        return true;
    }
    let Some(p) = state.players.get(&conn_id) else {
        return true;
    };
    if p.deleted {
        return true;
    }
    let (px, py) = (p.x, p.y);
    let blocked = p.blocked_teleport_locations.clone();
    let (mw, mh, wrap) = match state.world.read() {
        Ok(w) => (w.width_tiles, w.height_tiles, w.wrap),
        Err(_) => return true,
    };
    let pick = match cmd {
        TeleportBang::CursedGrave => pick_closest_from_index_map_ex(
            px,
            py,
            &state.world_map_time.cursed_graves,
            &blocked,
            mw,
            mh,
            wrap,
        ),
        TeleportBang::Village => pick_closest_from_index_map_ex(
            px,
            py,
            &state.world_map_time.ovens,
            &blocked,
            mw,
            mh,
            wrap,
        ),
    };
    match pick {
        TeleportPick::Empty => {
            crate::send_ps_reply(outbound, conn_id, not_found_text(cmd));
        }
        TeleportPick::AllBlocked => {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                clear_blocked_teleport(&mut pl.blocked_teleport_locations);
            }
            crate::send_ps_reply(outbound, conn_id, TELEPORT_ALL_TRIED);
        }
        TeleportPick::Found { index, tx, ty } => {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                push_blocked_teleport(&mut pl.blocked_teleport_locations, index);
            }
            apply_do_teleport(state, outbound, conn_id, tx, ty, mw, mh, wrap);
        }
    }
    true
}

/// Haxe `doTeleport` without VOG_UPDATE (parked).
// Haxe: GlobalPlayerInstance.doTeleport L5828-5843
fn apply_do_teleport(
    state: &mut crate::SimState,
    outbound: &ol_net::OutboundHub,
    conn_id: u64,
    tx: i32,
    ty: i32,
    mw: i32,
    mh: i32,
    wrap: bool,
) {
    let dest = ol_move_rules::wrap_tile(tx, ty, mw, mh, wrap);
    let dest = {
        let world = match state.world.read() {
            Ok(w) => w,
            Err(_) => return,
        };
        let is_blocked = |x: i32, y: i32| {
            !crate::pathfind::is_walkable(&world, &state.content, x, y)
        };
        match crate::jump_bw::plan_jump_to_non_blocked(is_blocked, dest.0, dest.1) {
            None => dest,
            Some((0, 0)) => return, // still blocked
            Some((dx, dy)) => ol_move_rules::wrap_tile(dest.0 + dx, dest.1 + dy, mw, mh, wrap),
        }
    };
    if let Some(pl) = state.players.get_mut(&conn_id) {
        pl.x = dest.0;
        pl.y = dest.1;
    } else {
        return;
    }
    crate::force_send_map_chunk(state, outbound, conn_id);
    crate::send_forced_player_update(state, outbound, conn_id, None);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_tcg_and_cursedgrave() {
        assert_eq!(parse_teleport_bang("!TCG"), Some(TeleportBang::CursedGrave));
        assert_eq!(
            parse_teleport_bang("GO !CURSEDGRAVE NOW"),
            Some(TeleportBang::CursedGrave)
        );
        assert_eq!(parse_teleport_bang("!TCG X"), None); // Haxe exact == for !TCG
                                                         // indexOf CURSEDGRAVE still works with prefix noise
        assert_eq!(
            parse_teleport_bang("X!CURSEDGRAVE"),
            Some(TeleportBang::CursedGrave)
        );
    }

    #[test]
    fn parse_tv_and_village() {
        assert_eq!(parse_teleport_bang("!TV"), Some(TeleportBang::Village));
        assert_eq!(
            parse_teleport_bang("FIND !VILLAGE"),
            Some(TeleportBang::Village)
        );
        assert_eq!(parse_teleport_bang("!TVX"), None);
    }

    #[test]
    fn parse_unrelated_none() {
        assert_eq!(parse_teleport_bang("!TG"), None);
        assert_eq!(parse_teleport_bang("HOME!"), None);
        assert_eq!(parse_teleport_bang(""), None);
    }

    #[test]
    fn pick_empty() {
        assert_eq!(pick_closest_teleport(0, 0, &[], &[]), TeleportPick::Empty);
    }

    #[test]
    fn pick_closest_unblocked() {
        let locs = vec![(10, (5, 0)), (20, (100, 0)), (30, (2, 0))];
        assert_eq!(
            pick_closest_teleport(0, 0, &locs, &[]),
            TeleportPick::Found {
                index: 30,
                tx: 2,
                ty: 0
            }
        );
        // Block closest → next
        assert_eq!(
            pick_closest_teleport(0, 0, &locs, &[30]),
            TeleportPick::Found {
                index: 10,
                tx: 5,
                ty: 0
            }
        );
    }

    #[test]
    fn pick_all_blocked_then_clear() {
        let locs = vec![(1, (3, 3)), (2, (9, 9))];
        let mut blocked = vec![1, 2];
        assert_eq!(
            pick_closest_teleport(0, 0, &locs, &blocked),
            TeleportPick::AllBlocked
        );
        clear_blocked_teleport(&mut blocked);
        assert!(blocked.is_empty());
        assert_eq!(
            pick_closest_teleport(0, 0, &locs, &blocked),
            TeleportPick::Found {
                index: 1,
                tx: 3,
                ty: 3
            }
        );
        push_blocked_teleport(&mut blocked, 1);
        assert_eq!(blocked, vec![1]);
    }

    #[test]
    fn pick_from_index_map() {
        let mut map = HashMap::new();
        map.insert(map_linear_index(8, 1, 100), (8, 1));
        map.insert(map_linear_index(1, 1, 100), (1, 1));
        assert_eq!(
            pick_closest_from_index_map(0, 0, &map, &[]),
            TeleportPick::Found {
                index: map_linear_index(1, 1, 100),
                tx: 1,
                ty: 1
            }
        );
    }

    #[test]
    fn not_found_texts() {
        assert_eq!(not_found_text(TeleportBang::CursedGrave), TCG_NOT_FOUND);
        assert_eq!(not_found_text(TeleportBang::Village), TV_NOT_FOUND);
    }

    #[test]
    fn pick_closest_wrap_prefers_torus_edge() {
        // Plane: 100 is closer than 500. Torus 512: 500 wraps to -12.
        let locs = vec![(1, (100, 0)), (2, (500, 0))];
        assert_eq!(
            pick_closest_teleport_ex(0, 0, &locs, &[], 512, 512, false),
            TeleportPick::Found {
                index: 1,
                tx: 100,
                ty: 0
            }
        );
        assert_eq!(
            pick_closest_teleport_ex(0, 0, &locs, &[], 512, 512, true),
            TeleportPick::Found {
                index: 2,
                tx: 500,
                ty: 0
            }
        );
    }
}
