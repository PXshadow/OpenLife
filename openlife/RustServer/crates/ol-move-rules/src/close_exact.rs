//! Haxe `MoveHelper.isCloseUseExact` / exact float range (USE bow min-range, kill).

use crate::distance::calculate_exact_quad_distance_f;

/// Haxe bow/ranged USE: refuse when `deadlyDistance > 1.9` and target within 1.5.
// Haxe: TransitionHelper.use L757-765
pub const RANGED_DEADLY_DISTANCE_THRESHOLD: f32 = 1.9;
/// Haxe min exact distance for ranged animal USE (`isCloseUseExact(..., 1.5)`).
// Haxe: TransitionHelper.use L761
pub const RANGED_MIN_USE_DISTANCE: f32 = 1.5;

/// Haxe `isCloseUseExact`: quad distance ≤ max_distance² (integer tile positions).
// Haxe: MoveHelper.isCloseUseExact
#[inline]
pub fn is_close_use_exact(ax: i32, ay: i32, bx: i32, by: i32, max_distance: f32) -> bool {
    is_close_use_exact_f(ax as f64, ay as f64, bx as f64, by as f64, max_distance)
}

/// Integer-tile exact range with map wrap.
// Haxe: MoveHelper.isCloseUseExact + transformFloat
#[inline]
pub fn is_close_use_exact_wrap(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    max_distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    is_close_use_exact_f_wrap(
        ax as f64,
        ay as f64,
        bx as f64,
        by as f64,
        max_distance,
        map_w,
        map_h,
        wrap,
    )
}

/// Haxe `isCloseUseExact` with float exact move positions (`exactTx` / `exactTy`).
// Haxe: MoveHelper.isCloseUseExact / isCloseToPlayerUseExact
#[inline]
pub fn is_close_use_exact_f(ax: f64, ay: f64, bx: f64, by: f64, max_distance: f32) -> bool {
    is_close_use_exact_f_wrap(ax, ay, bx, by, max_distance, 0, 0, false)
}

/// Float exact range with optional torus wrap (Haxe `calculateExactQuadDistance`).
// Haxe: MoveHelper.isCloseUseExact / calculateExactQuadDistance
#[inline]
pub fn is_close_use_exact_f_wrap(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    max_distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    let max_d = if max_distance.is_finite() && max_distance > 0.0 {
        max_distance
    } else {
        1.0
    };
    let q = calculate_exact_quad_distance_f(ax, ay, bx, by, map_w, map_h, wrap);
    q <= (max_d as f64) * (max_d as f64)
}

/// Haxe killHelper max-range slack: `exactQuadDistance > deadlyDistance² + 0.1`.
// Haxe: GlobalPlayerInstance.killHelper L4438
pub const KILL_DEADLY_RANGE_SLACK: f32 = 0.1;

/// Haxe killHelper in-range: `exactQuadDistance <= deadlyDistance² + 0.1`.
///
/// Bare hands (`deadlyDistance == 0`) only reach the same tile (quad `0 <= 0.1`).
/// Adjacent tiles need a weapon with deadly ≥ ~0.95.
// Haxe: GlobalPlayerInstance.killHelper L4433-4442
#[inline]
pub fn kill_in_deadly_range(
    player_x: f64,
    player_y: f64,
    target_x: f64,
    target_y: f64,
    deadly_distance: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    let d = if deadly_distance.is_finite() && deadly_distance >= 0.0 {
        deadly_distance as f64
    } else {
        0.0
    };
    let q = calculate_exact_quad_distance_f(
        player_x, player_y, target_x, target_y, map_w, map_h, wrap,
    );
    q <= d * d + (KILL_DEADLY_RANGE_SLACK as f64)
}

/// Haxe killHelper ranged min-range: deadly held + exact ≤ 1.5 (player targets).
// Haxe: GlobalPlayerInstance.killHelper L4420-4428
#[inline]
pub fn refuse_ranged_kill_too_close(
    held_deadly_distance: f32,
    player_x: f64,
    player_y: f64,
    target_x: f64,
    target_y: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if !(held_deadly_distance.is_finite()
        && held_deadly_distance > RANGED_DEADLY_DISTANCE_THRESHOLD)
    {
        return false;
    }
    is_close_use_exact_f_wrap(
        player_x,
        player_y,
        target_x,
        target_y,
        RANGED_MIN_USE_DISTANCE,
        map_w,
        map_h,
        wrap,
    )
}

/// Haxe TransitionHelper ranged USE refuse: deadly held + animal target too close.
// Haxe: TransitionHelper.use L757-765
#[inline]
pub fn refuse_ranged_use_too_close(
    held_deadly_distance: f32,
    target_is_animal: bool,
    player_x: f64,
    player_y: f64,
    target_x: f64,
    target_y: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if !target_is_animal {
        return false;
    }
    refuse_ranged_kill_too_close(
        held_deadly_distance,
        player_x,
        player_y,
        target_x,
        target_y,
        map_w,
        map_h,
        wrap,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_close_use_exact_boundary_and_wrap() {
        assert!(is_close_use_exact_f(0.0, 0.0, 1.5, 0.0, 1.5));
        assert!(!is_close_use_exact_f(0.0, 0.0, 1.5001, 0.0, 1.5));
        assert!(is_close_use_exact_f_wrap(
            0.0, 0.0, 9.0, 0.0, 1.5, 10, 10, true
        ));
        assert!(!is_close_use_exact_f_wrap(
            0.0, 0.0, 9.0, 0.0, 1.5, 10, 10, false
        ));
    }

    #[test]
    fn refuse_ranged_bow_animal() {
        assert!(refuse_ranged_use_too_close(
            2.0, true, 0.0, 0.0, 1.0, 0.0, 0, 0, false
        ));
        assert!(!refuse_ranged_use_too_close(
            2.0, false, 0.0, 0.0, 1.0, 0.0, 0, 0, false
        ));
        assert!(!refuse_ranged_use_too_close(
            1.5, true, 0.0, 0.0, 1.0, 0.0, 0, 0, false
        ));
    }

    #[test]
    fn kill_in_deadly_range_haxe_slack() {
        // Knife 1.5: adjacent quad=1 ≤ 2.25+0.1; dist 2 quad=4 > 2.35.
        assert!(kill_in_deadly_range(0.0, 0.0, 1.0, 0.0, 1.5, 0, 0, false));
        assert!(!kill_in_deadly_range(0.0, 0.0, 2.0, 0.0, 1.5, 0, 0, false));
        // Bow 4: dist 4 quad=16 ≤ 16.1; dist 5 quad=25 misses.
        assert!(kill_in_deadly_range(0.0, 0.0, 4.0, 0.0, 4.0, 0, 0, false));
        assert!(!kill_in_deadly_range(0.0, 0.0, 5.0, 0.0, 4.0, 0, 0, false));
        // Bare hands: same tile only.
        assert!(kill_in_deadly_range(0.0, 0.0, 0.0, 0.0, 0.0, 0, 0, false));
        assert!(!kill_in_deadly_range(0.0, 0.0, 1.0, 0.0, 0.0, 0, 0, false));
    }
}
