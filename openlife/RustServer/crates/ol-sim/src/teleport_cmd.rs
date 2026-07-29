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
    Found {
        index: i32,
        tx: i32,
        ty: i32,
    },
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
    let dx = (tx - px) as f64;
    let dy = (ty - py) as f64;
    dx * dx + dy * dy
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
    if locations.is_empty() {
        return TeleportPick::Empty;
    }
    let mut best: Option<(f64, i32, i32, i32)> = None; // dist, index, tx, ty
    for &(index, (tx, ty)) in locations {
        if blocked.contains(&index) {
            continue;
        }
        let dist = teleport_quad_distance(px, py, tx, ty);
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
    let locs: Vec<(i32, (i32, i32))> = map.iter().map(|(&k, &v)| (k, v)).collect();
    pick_closest_teleport(px, py, &locs, blocked)
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
        assert_eq!(
            pick_closest_teleport(0, 0, &[], &[]),
            TeleportPick::Empty
        );
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
}
