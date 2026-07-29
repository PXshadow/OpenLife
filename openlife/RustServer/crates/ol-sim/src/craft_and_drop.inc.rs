// Haxe: AiBase.GetCraftAndDropItemsCloseToObj ~893 + craftItemHelper specials ~6759–6902
// AI-CRAFT-LIVE-MORE / craft_multi_step — pure GetCraftAndDrop + adze/froe/goose/kindling/bucket

/// Steel Axe near stump for Domestic Goose craft (GetCraftAndDrop special).
// Haxe: Steel Axe 334 / Stump 338 / Domestic Goose 1256
pub const STEEL_AXE: i32 = 334;
pub const STUMP: i32 = 338;
pub const DOMESTIC_GOOSE: i32 = 1256;

/// Empty / full / partial water bucket + tanks (fillBucket residual).
// Haxe: Empty Bucket 659 / Full 660 / Partial 1099 / Tank 3167 / Tank less-full 3168
pub const EMPTY_BUCKET: i32 = 659;
pub const FULL_BUCKET_WATER: i32 = 660;
pub const PARTIAL_BUCKET_WATER: i32 = 1099;
pub const TANK_OF_WATER: i32 = 3167;
pub const TANK_OF_WATER_LESS_FULL: i32 = 3168;

/// Haxe `CalculateQuadDistance` threshold for goto vs dropHeld near anchor.
// Haxe: if (quadDist > 5) gotoObj else dropHeldObject(5, target)
pub const CRAFT_DROP_GOTO_QUAD_DIST: i32 = 5;

/// Search radius for GetClosestObjectToTarget in GetCraftAndDrop (Haxe 30).
pub const CRAFT_AND_DROP_SEARCH_R: i32 = 30;

/// Default `dist` for GetCraftAndDropItemsCloseToObj (Haxe default 8).
pub const CRAFT_AND_DROP_DEFAULT_DIST: i32 = 8;

/// Adze/froe → bring Butt Log close (Haxe dist=6).
pub const ADZE_FROE_LOG_DIST: i32 = 6;

/// Goose → Steel Axe near stump (Haxe dist=3).
pub const GOOSE_AXE_DIST: i32 = 3;

/// Fire bow → kindling/tinder near shaft (Haxe dist=10).
pub const FIRE_BOW_KINDLING_DIST: i32 = 10;

/// Stump search radius for goose special (Haxe r=20).
pub const GOOSE_STUMP_SEARCH_R: i32 = 20;

/// Default bucket water sources when `ServerSettings.BucketWaterSourceIds` not loaded.
/// Same well family as [`DEFAULT_WATER_SOURCE_IDS`] (Deep 663 / Shallow 662).
// Haxe: ServerSettings.BucketWaterSourceIds (transition-derived from Empty Bucket 659)
pub const DEFAULT_BUCKET_WATER_SOURCE_IDS: [i32; 2] = [663, 662];

/// Pure outcome of Haxe `GetCraftAndDropItemsCloseToObj`.
// Haxe: AiBase.GetCraftAndDropItemsCloseToObj ~893
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftAndDropApply {
    /// `count >= maxCount` near target — no action (continue normal craft).
    AlreadyEnough,
    /// Holding `which_obj` and squared dist to target > 5 → walk toward target.
    GotoDropTarget { target_x: i32, target_y: i32 },
    /// Holding `which_obj` and close enough → drop near target.
    DropNearTarget { target_x: i32, target_y: i32 },
    /// Found free `which_obj` in search band → pickup.
    Pickup {
        object_id: i32,
        x: i32,
        y: i32,
    },
    /// No free object → craft `which_obj` (Haxe `craftItem(whichObjId)`).
    CraftItem { object_id: i32 },
}

/// Squared euclidean distance (Haxe `CalculateQuadDistanceHelper`).
// Haxe: AiHelper.CalculateQuadDistanceHelper
#[inline]
pub fn craft_quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Closest of `parent_id` measured from `anchor`, within `search_r`, Chebyshev ≥ `min_dist`.
// Haxe: GetClosestObjectToPosition(anchor, id, searchDistance, minDistance)
pub fn closest_craft_obj_from_anchor(
    objs: &[CraftWorldObj],
    parent_id: i32,
    anchor_x: i32,
    anchor_y: i32,
    search_r: i32,
    min_dist: i32,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let search_r = search_r.max(0);
    let min_dist = min_dist.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        let d = craft_chebyshev(anchor_x, anchor_y, o.x, o.y);
        if d > search_r || d < min_dist {
            continue;
        }
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

/// Pure `GetCraftAndDropItemsCloseToObj(target, whichObjId, maxCount, dist)`.
///
/// Does **not** recurse into multi-step craft — returns [`CraftAndDropApply::CraftItem`]
/// so the caller can stage `craftItem(whichObjId)` (matches smith forge apply).
// Haxe: AiBase.GetCraftAndDropItemsCloseToObj ~893–916
pub fn get_craft_and_drop_items_close_to_obj(
    objs: &[CraftWorldObj],
    target_x: i32,
    target_y: i32,
    which_obj_id: i32,
    max_count: i32,
    dist: i32,
    held_id: i32,
    player_x: i32,
    player_y: i32,
) -> CraftAndDropApply {
    let max_count = max_count.max(1);
    let dist = dist.max(0);
    let near = count_craft_objs_near(objs, &[which_obj_id], target_x, target_y, dist);
    if near >= max_count {
        return CraftAndDropApply::AlreadyEnough;
    }
    if held_id == which_obj_id {
        let qd = craft_quad_dist(player_x, player_y, target_x, target_y);
        if qd > CRAFT_DROP_GOTO_QUAD_DIST {
            return CraftAndDropApply::GotoDropTarget { target_x, target_y };
        }
        return CraftAndDropApply::DropNearTarget { target_x, target_y };
    }
    // Haxe: GetClosestObjectToTarget(player, target, whichObjId, null, 30, dist)
    // search centered on target; minDistance = dist (outside already-counted band).
    if let Some(o) = closest_craft_obj_from_anchor(
        objs,
        which_obj_id,
        target_x,
        target_y,
        CRAFT_AND_DROP_SEARCH_R,
        dist,
    ) {
        return CraftAndDropApply::Pickup {
            object_id: which_obj_id,
            x: o.x,
            y: o.y,
        };
    }
    CraftAndDropApply::CraftItem {
        object_id: which_obj_id,
    }
}

/// Map [`CraftAndDropApply`] → [`CraftItemDecision`] (AlreadyEnough → None).
// Haxe: GetCraftAndDropItemsCloseToObj return true branches
pub fn craft_and_drop_to_decision(
    apply: CraftAndDropApply,
    product_id: i32,
) -> Option<CraftItemDecision> {
    match apply {
        CraftAndDropApply::AlreadyEnough => None,
        CraftAndDropApply::GotoDropTarget { target_x, target_y } => {
            Some(CraftItemDecision::GotoDropAnchor {
                target_x,
                target_y,
            })
        }
        CraftAndDropApply::DropNearTarget { target_x, target_y } => {
            Some(CraftItemDecision::DropNearAnchor {
                target_x,
                target_y,
            })
        }
        CraftAndDropApply::Pickup {
            object_id,
            x,
            y,
        } => Some(CraftItemDecision::PickupActor { object_id, x, y }),
        CraftAndDropApply::CraftItem { object_id } => Some(CraftItemDecision::SeekIngredient {
            ingredient_id: object_id,
            for_product: product_id,
        }),
    }
}

/// Adze/Froe + Butt Log: bring log to tool (GetCraftAndDrop on transActor).
// Haxe: craftItemHelper ~6761–6771 (actor 462|463 + target 345)
pub fn adze_froe_butt_log_craft_and_drop(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    actor_x: i32,
    actor_y: i32,
    held_id: i32,
    player_x: i32,
    player_y: i32,
    called_craft_item: bool,
    product_id: i32,
) -> Option<CraftItemDecision> {
    if called_craft_item {
        return None;
    }
    if target_id != BUTT_LOG || (actor_id != STEEL_ADZE && actor_id != STEEL_FROE) {
        return None;
    }
    let apply = get_craft_and_drop_items_close_to_obj(
        objs,
        actor_x,
        actor_y,
        BUTT_LOG,
        1,
        ADZE_FROE_LOG_DIST,
        held_id,
        player_x,
        player_y,
    );
    craft_and_drop_to_decision(apply, product_id)
}

/// Empty-hand + Domestic Goose: ensure Steel Axe near closest stump.
// Haxe: craftItemHelper ~6773–6781 (actor 0 + target 1256)
pub fn goose_axe_near_stump_craft_and_drop(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    held_id: i32,
    player_x: i32,
    player_y: i32,
    called_craft_item: bool,
    product_id: i32,
) -> Option<CraftItemDecision> {
    if called_craft_item {
        return None;
    }
    if actor_id != 0 || target_id != DOMESTIC_GOOSE {
        return None;
    }
    let stump = closest_craft_obj(objs, STUMP, player_x, player_y, GOOSE_STUMP_SEARCH_R, None)?;
    // Haxe: if (closest == null) return false — surface as Failed via special enum
    let apply = get_craft_and_drop_items_close_to_obj(
        objs,
        stump.x,
        stump.y,
        STEEL_AXE,
        1,
        GOOSE_AXE_DIST,
        held_id,
        player_x,
        player_y,
    );
    // No stump already returned None above; if AlreadyEnough continue normal path.
    craft_and_drop_to_decision(apply, product_id)
}

/// Fire bow + shaft: GetCraftAndDrop kindling then tinder near shaft.
// Haxe: craftItemHelper ~6890–6902
pub fn fire_bow_kindling_craft_and_drop(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    target_x: i32,
    target_y: i32,
    held_id: i32,
    player_x: i32,
    player_y: i32,
    called_craft_item: bool,
    product_id: i32,
) -> Option<CraftItemDecision> {
    if called_craft_item || actor_id != FIRE_BOW_DRILL || target_id != LONG_STRAIGHT_SHAFT {
        return None;
    }
    // Kindling first (Haxe: if GetCraftAndDrop(..., 72, 1, 10) return true)
    let k = get_craft_and_drop_items_close_to_obj(
        objs,
        target_x,
        target_y,
        KINDLING,
        1,
        FIRE_BOW_KINDLING_DIST,
        held_id,
        player_x,
        player_y,
    );
    if !matches!(k, CraftAndDropApply::AlreadyEnough) {
        return craft_and_drop_to_decision(k, product_id);
    }
    // Then Juniper Tinder
    let t = get_craft_and_drop_items_close_to_obj(
        objs,
        target_x,
        target_y,
        JUNIPER_TINDER,
        1,
        FIRE_BOW_KINDLING_DIST,
        held_id,
        player_x,
        player_y,
    );
    if !matches!(t, CraftAndDropApply::AlreadyEnough) {
        return craft_and_drop_to_decision(t, product_id);
    }
    None
}

/// Pure residual of Haxe `fillBucketIfNeeded` tank/bucket shortCraft ladder.
// Haxe: AiBase.fillBucketIfNeeded ~3515–3545
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillBucketApply {
    /// Not waterbringer / nothing to do.
    None,
    /// Holding full/partial bucket → drop.
    DropHeldFullBucket,
    /// Already have full/partial bucket nearby → stop (Haxe return false).
    AlreadyHaveWaterBucket,
    /// shortCraft(tank, empty bucket).
    ShortCraft {
        actor_id: i32,
        target_id: i32,
    },
    /// shortCraftOnTarget(empty bucket, bucket water source).
    ShortCraftOnSource {
        actor_id: i32,
        source_id: i32,
        source_x: i32,
        source_y: i32,
    },
    /// No bucket water source / tanks.
    NoSource,
}

/// Pure `fillBucketIfNeeded` decision edges (caller gates profession).
// Haxe: fillBucketIfNeeded ~3515
pub fn fill_bucket_if_needed_apply(
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    max_r: i32,
    bucket_water_source_ids: &[i32],
) -> FillBucketApply {
    // Full / partial bucket held → drop
    if held_id == FULL_BUCKET_WATER || held_id == PARTIAL_BUCKET_WATER {
        return FillBucketApply::DropHeldFullBucket;
    }
    // Already have water bucket on ground → false (don't fill more)
    if closest_craft_obj_by_ids(
        objs,
        &[FULL_BUCKET_WATER, PARTIAL_BUCKET_WATER],
        player_x,
        player_y,
        max_r,
    )
    .is_some()
    {
        return FillBucketApply::AlreadyHaveWaterBucket;
    }
    // Tank less-full + empty bucket shortCraft
    if closest_craft_obj(objs, TANK_OF_WATER_LESS_FULL, player_x, player_y, max_r, None).is_some()
        && closest_craft_obj(objs, EMPTY_BUCKET, player_x, player_y, max_r, None).is_some()
    {
        return FillBucketApply::ShortCraft {
            actor_id: TANK_OF_WATER_LESS_FULL,
            target_id: EMPTY_BUCKET,
        };
    }
    // Full tank + empty bucket
    if closest_craft_obj(objs, TANK_OF_WATER, player_x, player_y, max_r, None).is_some()
        && closest_craft_obj(objs, EMPTY_BUCKET, player_x, player_y, max_r, None).is_some()
    {
        return FillBucketApply::ShortCraft {
            actor_id: TANK_OF_WATER,
            target_id: EMPTY_BUCKET,
        };
    }
    // Bucket water source + empty bucket
    if let Some(src) =
        closest_craft_obj_by_ids(objs, bucket_water_source_ids, player_x, player_y, max_r)
    {
        return FillBucketApply::ShortCraftOnSource {
            actor_id: EMPTY_BUCKET,
            source_id: src.parent_id,
            source_x: src.x,
            source_y: src.y,
        };
    }
    FillBucketApply::NoSource
}

#[cfg(test)]
mod craft_and_drop_tests {
    use super::*;

    #[test]
    fn craft_and_drop_already_enough() {
        let objs = vec![CraftWorldObj::simple(BUTT_LOG, 1, 0)];
        // adze at 0,0; log at 1,0 within dist 6
        let a = get_craft_and_drop_items_close_to_obj(&objs, 0, 0, BUTT_LOG, 1, 6, 0, 0, 0);
        assert_eq!(a, CraftAndDropApply::AlreadyEnough);
    }

    #[test]
    fn craft_and_drop_pickup_outside_band() {
        let objs = vec![CraftWorldObj::simple(BUTT_LOG, 10, 0)];
        // log at d=10 ≥ min_dist 6, ≤ 30 → pickup
        let a = get_craft_and_drop_items_close_to_obj(&objs, 0, 0, BUTT_LOG, 1, 6, 0, 0, 0);
        assert_eq!(
            a,
            CraftAndDropApply::Pickup {
                object_id: BUTT_LOG,
                x: 10,
                y: 0
            }
        );
    }

    #[test]
    fn craft_and_drop_held_goto_and_drop() {
        // Holding log, far from adze (quad > 5)
        let a = get_craft_and_drop_items_close_to_obj(&[], 0, 0, BUTT_LOG, 1, 6, BUTT_LOG, 10, 0);
        assert_eq!(
            a,
            CraftAndDropApply::GotoDropTarget {
                target_x: 0,
                target_y: 0
            }
        );
        // Close (quad 4 = 2²)
        let b = get_craft_and_drop_items_close_to_obj(&[], 0, 0, BUTT_LOG, 1, 6, BUTT_LOG, 2, 0);
        assert_eq!(
            b,
            CraftAndDropApply::DropNearTarget {
                target_x: 0,
                target_y: 0
            }
        );
    }

    #[test]
    fn craft_and_drop_craft_when_missing() {
        let a = get_craft_and_drop_items_close_to_obj(&[], 0, 0, BUTT_LOG, 1, 6, 0, 0, 0);
        assert_eq!(a, CraftAndDropApply::CraftItem { object_id: BUTT_LOG });
    }

    #[test]
    fn adze_special_seeks_butt_log() {
        let objs = vec![
            CraftWorldObj::simple(STEEL_ADZE, 0, 0),
            // far log outside min band would pickup; none → craft
        ];
        let d = adze_froe_butt_log_craft_and_drop(
            &objs, STEEL_ADZE, BUTT_LOG, 0, 0, 0, 0, 0, false, 999,
        );
        assert_eq!(
            d,
            Some(CraftItemDecision::SeekIngredient {
                ingredient_id: BUTT_LOG,
                for_product: 999
            })
        );
        // called_craft_item blocks
        assert!(adze_froe_butt_log_craft_and_drop(
            &objs, STEEL_ADZE, BUTT_LOG, 0, 0, 0, 0, 0, true, 999
        )
        .is_none());
        // froe same
        let d2 = adze_froe_butt_log_craft_and_drop(
            &objs, STEEL_FROE, BUTT_LOG, 0, 0, 0, 0, 0, false, 1,
        );
        assert!(matches!(d2, Some(CraftItemDecision::SeekIngredient { .. })));
    }

    #[test]
    fn goose_special_needs_stump_then_axe() {
        // No stump → None (caller may fail path)
        assert!(goose_axe_near_stump_craft_and_drop(
            &[],
            0,
            DOMESTIC_GOOSE,
            0,
            0,
            0,
            false,
            1
        )
        .is_none());
        let objs = vec![
            CraftWorldObj::simple(STUMP, 2, 0),
            CraftWorldObj::simple(STEEL_AXE, 20, 0),
        ];
        let d = goose_axe_near_stump_craft_and_drop(
            &objs,
            0,
            DOMESTIC_GOOSE,
            0,
            0,
            0,
            false,
            1,
        );
        // axe at 20 from stump@2 → d=18 ≥ 3 → pickup
        assert_eq!(
            d,
            Some(CraftItemDecision::PickupActor {
                object_id: STEEL_AXE,
                x: 20,
                y: 0
            })
        );
    }

    #[test]
    fn fire_bow_kindling_drop_path() {
        let shaft_x = 5;
        let shaft_y = 5;
        // Kindling far from shaft → pickup
        let objs = vec![
            CraftWorldObj::simple(LONG_STRAIGHT_SHAFT, shaft_x, shaft_y),
            CraftWorldObj::simple(KINDLING, 20, 5),
        ];
        let d = fire_bow_kindling_craft_and_drop(
            &objs,
            FIRE_BOW_DRILL,
            LONG_STRAIGHT_SHAFT,
            shaft_x,
            shaft_y,
            0,
            0,
            0,
            false,
            75,
        );
        assert_eq!(
            d,
            Some(CraftItemDecision::PickupActor {
                object_id: KINDLING,
                x: 20,
                y: 5
            })
        );
        // Kindling already near → try tinder
        let near_k = vec![
            CraftWorldObj::simple(LONG_STRAIGHT_SHAFT, shaft_x, shaft_y),
            CraftWorldObj::simple(KINDLING, 6, 5),
            CraftWorldObj::simple(JUNIPER_TINDER, 25, 5),
        ];
        let d2 = fire_bow_kindling_craft_and_drop(
            &near_k,
            FIRE_BOW_DRILL,
            LONG_STRAIGHT_SHAFT,
            shaft_x,
            shaft_y,
            0,
            0,
            0,
            false,
            75,
        );
        // kindling already enough; tinder far → pickup tinder
        assert_eq!(
            d2,
            Some(CraftItemDecision::PickupActor {
                object_id: JUNIPER_TINDER,
                x: 25,
                y: 5
            })
        );
        // both near → None (continue fire craft)
        let both = vec![
            CraftWorldObj::simple(LONG_STRAIGHT_SHAFT, shaft_x, shaft_y),
            CraftWorldObj::simple(KINDLING, 6, 5),
            CraftWorldObj::simple(JUNIPER_TINDER, 7, 5),
        ];
        assert!(fire_bow_kindling_craft_and_drop(
            &both,
            FIRE_BOW_DRILL,
            LONG_STRAIGHT_SHAFT,
            shaft_x,
            shaft_y,
            0,
            0,
            0,
            false,
            75
        )
        .is_none());
    }

    #[test]
    fn fill_bucket_tank_and_source() {
        // full held
        assert_eq!(
            fill_bucket_if_needed_apply(&[], FULL_BUCKET_WATER, 0, 0, 30, &DEFAULT_BUCKET_WATER_SOURCE_IDS),
            FillBucketApply::DropHeldFullBucket
        );
        // tank + empty
        let objs = vec![
            CraftWorldObj::simple(TANK_OF_WATER_LESS_FULL, 1, 0),
            CraftWorldObj::simple(EMPTY_BUCKET, 2, 0),
        ];
        assert_eq!(
            fill_bucket_if_needed_apply(&objs, 0, 0, 0, 30, &DEFAULT_BUCKET_WATER_SOURCE_IDS),
            FillBucketApply::ShortCraft {
                actor_id: TANK_OF_WATER_LESS_FULL,
                target_id: EMPTY_BUCKET
            }
        );
        // well source
        let wells = vec![CraftWorldObj::simple(663, 3, 0)];
        assert_eq!(
            fill_bucket_if_needed_apply(&wells, 0, 0, 0, 30, &DEFAULT_BUCKET_WATER_SOURCE_IDS),
            FillBucketApply::ShortCraftOnSource {
                actor_id: EMPTY_BUCKET,
                source_id: 663,
                source_x: 3,
                source_y: 0
            }
        );
    }
}
