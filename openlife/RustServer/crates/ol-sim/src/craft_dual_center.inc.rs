// Haxe: searchCurrentPosition dual-center + pile*1.5 / r=6 re-anchor (AI-CRAFT-DUAL)
// Included into craft_item.rs via include! — same module namespace.

/// Haxe `GetClosestObjectToTarget(..., 6)` re-anchor radius near craft target.
// Haxe: craftItemHelper ~7057 / ~7072
pub const ACTOR_NEAR_TARGET_R: i32 = 6;

/// Prefer loose actor when `quad(actor) < quad(pile) * 1.5`.
// Haxe: craftItemHelper ~7067
pub const PILE_VS_LOOSE_QUAD_FACTOR: f64 = 1.5;

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
// Haxe: addAllObjectsForCraftig home always; player when searchCurrentPosition
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
) -> HashSet<i32> {
    craft_have_set_ex_filtered(
        objs,
        held_id,
        player_x,
        player_y,
        home,
        radius,
        search_current_position,
        &CraftScanFilters::default(),
    )
}

/// Dual-center have-set with hostile / unreachable / full-pile scan filters.
// Haxe: addObjectsForCrafting isObjectNotReachable + container/full pile skips
pub fn craft_have_set_ex_filtered(
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current_position: bool,
    filters: &CraftScanFilters<'_>,
) -> HashSet<i32> {
    let mut have = HashSet::new();
    if held_id > 0 {
        have.insert(held_id);
    }
    have.insert(0);
    for o in objs {
        if o.parent_id <= 0 {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
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

/// Closest matching `parent_id` in dual-center radius, ranked by **player** distance.
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
    closest_craft_obj_dual_center_filtered(
        objs,
        parent_id,
        player_x,
        player_y,
        home,
        radius,
        search_current_position,
        exclude,
        &CraftScanFilters::default(),
    )
}

/// Dual-center closest with path-reach / full-pile scan filters.
// Haxe: addObjectsForCrafting isObjectNotReachable + GetClosestObject* hostile skip
pub fn closest_craft_obj_dual_center_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current_position: bool,
    exclude: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
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
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
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
    reanchor_craft_actor_near_target_filtered(
        objs,
        actor_id,
        actor_x,
        actor_y,
        from_pile,
        pile_id_in,
        target_x,
        target_y,
        player_x,
        player_y,
        pile_id_for,
        &CraftScanFilters::default(),
    )
}

/// Re-anchor with hostile / unreachable scan filters on pile/loose candidates.
// Haxe: GetClosestObject* skips isObjectNotReachable / isObjectWithHostilePath
pub fn reanchor_craft_actor_near_target_filtered(
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
    filters: &CraftScanFilters<'_>,
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

    let mut pile: Option<CraftWorldObj> = None;
    if resolved_pile_id > 0 {
        pile = closest_craft_obj_filtered(
            objs,
            resolved_pile_id,
            target_x,
            target_y,
            ACTOR_NEAR_TARGET_R,
            Some((actor_x, actor_y)),
            filters,
        )
        .filter(|p| !(p.x == target_x && p.y == target_y));

        if pile.is_none() {
            pile = closest_craft_obj_filtered(
                objs,
                resolved_pile_id,
                player_x,
                player_y,
                AI_MAX_SEARCH_RADIUS,
                Some((actor_x, actor_y)),
                filters,
            );
        }
        if pile.is_none() && from_pile && pile_id_in > 0 {
            // Keep original pile only if its tile still passes scan filters.
            let orig = CraftWorldObj::simple(pile_id_in, actor_x, actor_y);
            if craft_obj_passes_scan_filters(&orig, filters) {
                pile = Some(orig);
            }
        }
    }

    if let Some(p) = pile {
        let qd_actor = craft_quad_distance(player_x, player_y, actor_x, actor_y);
        let qd_pile = craft_quad_distance(player_x, player_y, p.x, p.y);
        // Haxe: if (quadDistanceToActor < quadDistanceToPile * 1.5) pile = null;
        if (qd_actor as f64) < (qd_pile as f64) * PILE_VS_LOOSE_QUAD_FACTOR {
            // prefer loose — fall through
        } else {
            return CraftActorReanchor {
                actor_x: p.x,
                actor_y: p.y,
                from_pile: true,
                pile_id: resolved_pile_id,
            };
        }
    }

    // Haxe: actor close to target, r=6 (only when pile preference cleared).
    if let Some(o) = closest_craft_obj_filtered(
        objs,
        actor_id,
        target_x,
        target_y,
        ACTOR_NEAR_TARGET_R,
        Some((actor_x, actor_y)),
        filters,
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

    // No better pile/loose: keep original actor only if still passable.
    // Haxe: pile==null → goto original transActor (does not clear pile source).
    let orig = CraftWorldObj::simple(actor_id, actor_x, actor_y);
    if !from_pile && !craft_obj_passes_scan_filters(&orig, filters) {
        // Original blocked — leave coords; caller still has sticky coords (may fail USE).
    }
    CraftActorReanchor {
        actor_x,
        actor_y,
        from_pile,
        pile_id: pile_id_in,
    }
}

#[cfg(test)]
mod dual_center_tests {
    use super::*;

    #[test]
    fn dual_center_home_only_when_search_current_false() {
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
            CraftWorldObj::simple(10, 2, 0),
            CraftWorldObj::simple(20, 50, 0),
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
        let objs = vec![
            CraftWorldObj::simple(33, 1, 0),
            CraftWorldObj::simple(33, 8, 0),
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
        assert_eq!((o.x, o.y), (8, 0));
    }

    #[test]
    fn closest_dual_filtered_skips_blocked() {
        let objs = vec![
            CraftWorldObj::simple(33, 8, 0),
            CraftWorldObj::simple(33, 1, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((8, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let o = closest_craft_obj_dual_center_filtered(
            &objs,
            33,
            10,
            0,
            Some((0, 0)),
            30,
            true,
            None,
            &scan,
        )
        .unwrap();
        assert_eq!((o.x, o.y), (1, 0));
    }

    #[test]
    fn have_set_ex_filtered_skips_blocked() {
        let objs = vec![
            CraftWorldObj::simple(10, 2, 0),
            CraftWorldObj::simple(20, 3, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((3, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let have = craft_have_set_ex_filtered(&objs, 0, 0, 0, None, 15, true, &scan);
        assert!(have.contains(&10));
        assert!(!have.contains(&20));
    }

    #[test]
    fn reanchor_prefers_pile_near_target_when_far_loose() {
        let objs = vec![
            CraftWorldObj::simple(33, 40, 0),
            CraftWorldObj::simple(133, 5, 1),
            CraftWorldObj::simple(2, 5, 0),
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
        assert_eq!((r.actor_x, r.actor_y), (1, 0));
    }

    #[test]
    fn reanchor_loose_near_target_r6() {
        let objs = vec![
            CraftWorldObj::simple(33, 0, 0),
            CraftWorldObj::simple(33, 12, 0),
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
