//! Dual-center craft search + actor re-anchor (**AI-CRAFT-DUAL** / dual_center_search).
//!
//! Ports Haxe:
//! - `searchCurrentPosition` dual home vs player scan (`addAllObjectsForCraftig` /
//!   `addObjectsForCrafting`)
//! - pile-vs-loose `* 1.5` preference + r=6 re-anchor near craft target
//!   (`craftItemHelper` ~7050–7083)
//!
//! Nested under `craft_item` via `#[path]` or usable as helpers from there.

use super::{closest_craft_obj, craft_chebyshev, CraftWorldObj, AI_MAX_SEARCH_RADIUS};

// ── Constants ───────────────────────────────────────────────────────────────

/// Haxe `GetClosestObjectToTarget(..., 6)` re-anchor radius near craft target.
// Haxe: craftItemHelper ~7057 / ~7072
pub const ACTOR_NEAR_TARGET_R: i32 = 6;

/// Prefer loose actor when `quad(actor) < quad(pile) * 1.5`.
// Haxe: craftItemHelper ~7067
pub const PILE_VS_LOOSE_QUAD_FACTOR: f64 = 1.5;

// ── Geometry ────────────────────────────────────────────────────────────────

/// Squared Euclidean distance (Haxe `CalculateQuadDistanceHelper`).
// Haxe: AiHelper.CalculateQuadDistanceHelper
#[inline]
pub fn craft_quad_distance(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Whether `(ox,oy)` is inside the dual-center craft scan.
///
/// - Home set + `search_current_position=false` → only within `radius` of home
/// - Home set + `search_current_position=true` → home **or** player center
/// - No home → player center only
// Haxe: addAllObjectsForCraftig home always; TODO player when searchCurrentPosition
pub fn craft_obj_in_dual_center(
    ox: i32,
    oy: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current_position: bool,
) -> bool {
    let r = radius.max(0);
    if let Some((hx, hy)) = home {
        if craft_chebyshev(hx, hy, ox, oy) <= r {
            return true;
        }
        if search_current_position && craft_chebyshev(player_x, player_y, ox, oy) <= r {
            return true;
        }
        return false;
    }
    craft_chebyshev(player_x, player_y, ox, oy) <= r
}

// ── Have-set ────────────────────────────────────────────────────────────────

/// Object ids present under dual-center membership (held + ground).
// Haxe: intitObjectsForCraftig / addAllObjectsForCraftig + searchCurrentPosition
pub fn craft_have_set_ex(
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current_position: bool,
) -> std::collections::HashSet<i32> {
    let mut have = std::collections::HashSet::new();
    if held_id > 0 {
        have.insert(held_id);
    }
    have.insert(0);
    for o in objs {
        if o.parent_id <= 0 {
            continue;
        }
        if craft_obj_in_dual_center(
            o.x,
            o.y,
            player_x,
            player_y,
            home,
            radius,
            search_current_position,
        ) {
            have.insert(o.parent_id);
        }
    }
    have
}

// ── Closest under dual-center, ranked by player distance ────────────────────

/// Closest matching `parent_id` in dual-center radius, ranked by **player** distance
/// (Haxe `CalculateQuadDistanceToObject` ranking while scanning home/player boxes).
// Haxe: addObjectsForCrafting + closestObject by player quad distance
pub fn closest_craft_obj_dual_center(
    objs: &[CraftWorldObj],
    parent_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current_position: bool,
    exclude: Option<(i32, i32)>,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        if let Some((ex, ey)) = exclude {
            if o.x == ex && o.y == ey {
                continue;
            }
        }
        if !craft_obj_in_dual_center(
            o.x,
            o.y,
            player_x,
            player_y,
            home,
            radius,
            search_current_position,
        ) {
            continue;
        }
        // Rank by player Chebyshev (stable); Haxe uses quad but same order for grid steps.
        let d = craft_chebyshev(player_x, player_y, o.x, o.y);
        match best {
            None => best = Some((d, *o)),
            Some((bd, bo)) => {
                if d < bd || (d == bd && (o.y < bo.y || (o.y == bo.y && o.x < bo.x))) {
                    best = Some((d, *o));
                }
            }
        }
    }
    best.map(|(_, o)| o)
}

// ── Actor re-anchor near target ─────────────────────────────────────────────

/// Result of pile-vs-loose / r=6 re-anchor for craft actor acquisition.
// Haxe: craftItemHelper ~7050–7083
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftActorReanchor {
    pub actor_x: i32,
    pub actor_y: i32,
    pub from_pile: bool,
    pub pile_id: i32,
}

/// Re-anchor actor pickup: prefer pile near target (r=6), else pile vs loose `*1.5`,
/// else loose actor near target (r=6) so stones/tools are not hauled home.
// Haxe: craftItemHelper pile + GetClosestObjectToTarget r=6
pub fn reanchor_craft_actor_near_target(
    objs: &[CraftWorldObj],
    actor_id: i32,
    actor_x: i32,
    actor_y: i32,
    from_pile: bool,
    pile_id_in: i32,
    target_x: i32,
    target_y: i32,
    player_x: i32,
    player_y: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftActorReanchor {
    if actor_id <= 0 {
        return CraftActorReanchor {
            actor_x,
            actor_y,
            from_pile,
            pile_id: pile_id_in,
        };
    }

    let resolved_pile_id = if pile_id_in > 0 {
        pile_id_in
    } else {
        pile_id_for.map(|f| f(actor_id)).unwrap_or(-1)
    };

    // Haxe: pile from transitionsByObjectId[pileId].closestObject, then near-target override.
    let mut pile: Option<CraftWorldObj> = None;
    if resolved_pile_id > 0 {
        // Prefer pile close to craft target (r=6), not the current actor tile.
        pile = closest_craft_obj(
            objs,
            resolved_pile_id,
            target_x,
            target_y,
            ACTOR_NEAR_TARGET_R,
            Some((actor_x, actor_y)),
        )
        .filter(|p| !(p.x == target_x && p.y == target_y));

        // Fall back to any pile near player (global closest).
        if pile.is_none() {
            pile = closest_craft_obj(
                objs,
                resolved_pile_id,
                player_x,
                player_y,
                AI_MAX_SEARCH_RADIUS,
                Some((actor_x, actor_y)),
            );
        }
        // Original actor may itself be the pile tile chosen by search.
        if pile.is_none() && from_pile && pile_id_in > 0 {
            pile = Some(CraftWorldObj::simple(pile_id_in, actor_x, actor_y));
        }
    }

    if let Some(p) = pile {
        let qd_actor = craft_quad_distance(player_x, player_y, actor_x, actor_y);
        let qd_pile = craft_quad_distance(player_x, player_y, p.x, p.y);
        // Haxe: if (quadDistanceToActor < quadDistanceToPile * 1.5) pile = null;
        // Prefer slightly farther loose objects over piles.
        if (qd_actor as f64) < (qd_pile as f64) * PILE_VS_LOOSE_QUAD_FACTOR {
            // Drop pile preference — keep loose actor (may re-anchor below).
        } else {
            return CraftActorReanchor {
                actor_x: p.x,
                actor_y: p.y,
                from_pile: true,
                pile_id: resolved_pile_id,
            };
        }
    }

    // Haxe: actor close to target (e.g. not bring home round stones), r=6.
    if let Some(o) = closest_craft_obj(
        objs,
        actor_id,
        target_x,
        target_y,
        ACTOR_NEAR_TARGET_R,
        Some((actor_x, actor_y)),
    ) {
        if !(o.x == target_x && o.y == target_y) {
            return CraftActorReanchor {
                actor_x: o.x,
                actor_y: o.y,
                from_pile: false,
                pile_id: -1,
            };
        }
    }

    // No better pile/loose: keep original actor (may still be from_pile).
    // Haxe: pile==null → goto original transActor (does not clear pile source).
    CraftActorReanchor {
        actor_x,
        actor_y,
        from_pile,
        pile_id: pile_id_in,
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_center_home_only_when_search_current_false() {
        // Home at 0,0; object near player at 40,0; radius 15
        assert!(craft_obj_in_dual_center(5, 0, 40, 0, Some((0, 0)), 15, false));
        assert!(!craft_obj_in_dual_center(
            40,
            0,
            40,
            0,
            Some((0, 0)),
            15,
            false
        ));
    }

    #[test]
    fn dual_center_includes_player_when_search_current_true() {
        assert!(craft_obj_in_dual_center(
            40,
            0,
            40,
            0,
            Some((0, 0)),
            15,
            true
        ));
        // Still includes home
        assert!(craft_obj_in_dual_center(5, 0, 40, 0, Some((0, 0)), 15, true));
    }

    #[test]
    fn dual_center_no_home_uses_player() {
        assert!(craft_obj_in_dual_center(3, 0, 0, 0, None, 15, false));
        assert!(!craft_obj_in_dual_center(30, 0, 0, 0, None, 15, false));
    }

    #[test]
    fn have_set_ex_respects_search_current() {
        let objs = vec![
            CraftWorldObj::simple(10, 2, 0),  // near home
            CraftWorldObj::simple(20, 50, 0), // near player only
        ];
        let home = Some((0, 0));
        let home_only = craft_have_set_ex(&objs, 0, 50, 0, home, 15, false);
        assert!(home_only.contains(&10));
        assert!(!home_only.contains(&20));

        let dual = craft_have_set_ex(&objs, 0, 50, 0, home, 15, true);
        assert!(dual.contains(&10));
        assert!(dual.contains(&20));
    }

    #[test]
    fn closest_dual_ranks_by_player_not_home() {
        // Two stones: one closer to home, one closer to player.
        let objs = vec![
            CraftWorldObj::simple(33, 1, 0), // near home
            CraftWorldObj::simple(33, 8, 0), // nearer player at 10,0
        ];
        let o = closest_craft_obj_dual_center(
            &objs,
            33,
            10,
            0,
            Some((0, 0)),
            30,
            true,
            None,
        )
        .unwrap();
        // Player at 10,0 → (8,0) is closer than (1,0)
        assert_eq!((o.x, o.y), (8, 0));
    }

    #[test]
    fn reanchor_prefers_pile_near_target_when_far_loose() {
        // Loose actor far from player; pile next to target.
        let objs = vec![
            CraftWorldObj::simple(33, 40, 0),  // loose far
            CraftWorldObj::simple(133, 5, 1),  // pile near target (5,0)
            CraftWorldObj::simple(2, 5, 0),    // target
        ];
        let pile_fn = |id: i32| if id == 33 { 133 } else { -1 };
        let r = reanchor_craft_actor_near_target(
            &objs,
            33,
            40,
            0,
            false,
            -1,
            5,
            0,
            0,
            0,
            Some(&pile_fn),
        );
        assert!(r.from_pile);
        assert_eq!(r.pile_id, 133);
        assert_eq!((r.actor_x, r.actor_y), (5, 1));
    }

    #[test]
    fn reanchor_prefers_loose_when_closer_than_pile_times_1_5() {
        // Loose at dist²=1, pile at dist²=4; 1 < 4*1.5 → prefer loose.
        let objs = vec![
            CraftWorldObj::simple(33, 1, 0),
            CraftWorldObj::simple(133, 2, 0),
            CraftWorldObj::simple(2, 10, 0),
        ];
        let pile_fn = |id: i32| if id == 33 { 133 } else { -1 };
        let r = reanchor_craft_actor_near_target(
            &objs,
            33,
            1,
            0,
            false,
            -1,
            10,
            0,
            0,
            0,
            Some(&pile_fn),
        );
        assert!(!r.from_pile);
        // May re-anchor loose near target if any within r=6 of target — none of 33 near 10,0
        // within exclude (1,0). Keep original loose.
        assert_eq!((r.actor_x, r.actor_y), (1, 0));
    }

    #[test]
    fn reanchor_loose_near_target_r6() {
        // Far home-side actor; another actor within r=6 of target.
        let objs = vec![
            CraftWorldObj::simple(33, 0, 0),  // original (far from target)
            CraftWorldObj::simple(33, 12, 0), // near target at 10,0 (d=2)
            CraftWorldObj::simple(2, 10, 0),
        ];
        let r = reanchor_craft_actor_near_target(
            &objs, 33, 0, 0, false, -1, 10, 0, 0, 0, None,
        );
        assert!(!r.from_pile);
        assert_eq!((r.actor_x, r.actor_y), (12, 0));
    }

    #[test]
    fn quad_distance_matches_haxe() {
        assert_eq!(craft_quad_distance(0, 0, 3, 4), 25);
        assert_eq!(craft_quad_distance(1, 1, 1, 1), 0);
    }
}
