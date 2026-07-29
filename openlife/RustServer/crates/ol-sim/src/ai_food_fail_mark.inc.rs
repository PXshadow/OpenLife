// AI-FOOD-FAIL-MARK / food_use_fail_30s — included from ai_path_reach.rs
// Haxe: AiBase.isPickingupFood done==false ~8694–8700

/// Settle async food USE/DROP/REMV after send (Rust net is async; Haxe is sync).
///
/// If hands still empty and tile still food → `mark_food_path_fail` 30s + clear sticky.
/// If held anything → treat as success (no mark).
// Haxe: AiBase.isPickingupFood done==false ~8694–8700 (sync use/remove/drop)
// AI-FOOD-FAIL-MARK / food_use_fail_30s
pub fn settle_pending_food_use_fail(
    maps: &mut AiPathReachMaps,
    sticky_food: &mut Option<StickyFoodTarget>,
    pending_xy: Option<(i32, i32)>,
    held_id: i32,
    tile_still_food: bool,
) -> bool {
    let Some((x, y)) = pending_xy else {
        return false;
    };
    // Picked something up (food or other) — do not mark the prior food tile.
    if held_id != 0 {
        return false;
    }
    if !tile_still_food {
        return false;
    }
    apply_food_action_fail(maps, sticky_food, None, x, y);
    true
}
