//! Pure USE/DROP/REMV range gates (Haxe `isClose` / `checkIfNotMovingAndCloseEnough`).

use crate::distance::calculate_distance_sq;

/// Haxe clamp: `useDistance < 1 → 1` (empty hands / missing content → adjacency).
// Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough
#[inline]
pub fn effective_use_distance(use_distance: i32) -> i32 {
    if use_distance < 1 {
        1
    } else {
        use_distance
    }
}

/// Squared-Euclidean range check (Haxe `isClose` without map wrap).
///
/// With default `use_distance = 1`, diagonal tiles fail (`2 ≰ 1`).
/// Prefer [`in_use_range_ex`] when map wrap is known (Haxe always wraps).
// Haxe: GlobalPlayerInstance.isClose → AiHelper.CalculateDistance
#[inline]
pub fn in_use_range(px: i32, py: i32, tx: i32, ty: i32, use_distance: i32) -> bool {
    in_use_range_ex(px, py, tx, ty, use_distance, 0, 0, false)
}

/// Haxe `GlobalPlayerInstance.isClose(x, y, distance)` with optional torus wrap.
// Haxe: GlobalPlayerInstance.isClose L1780
#[inline]
pub fn is_close(
    px: i32,
    py: i32,
    tx: i32,
    ty: i32,
    distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    in_use_range_ex(px, py, tx, ty, distance, map_w, map_h, wrap)
}

/// Squared-Euclidean USE/DROP range with optional torus wrap.
// Haxe: GlobalPlayerInstance.isClose + AiHelper.CalculateDistance
#[inline]
pub fn in_use_range_ex(
    px: i32,
    py: i32,
    tx: i32,
    ty: i32,
    use_distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    let d = effective_use_distance(use_distance) as f64;
    let max_sq = d * d;
    calculate_distance_sq(px, py, tx, ty, map_w, map_h, wrap) <= max_sq
}

/// Haxe `TransitionHelper.checkIfNotMovingAndCloseEnough`.
///
/// Returns `true` when the player may USE/DROP/REMV: not moving and within
/// held `useDistance` (clamped to ≥1) of the target tile.
// Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough
#[inline]
pub fn check_if_not_moving_and_close_enough(
    moving: bool,
    px: i32,
    py: i32,
    tx: i32,
    ty: i32,
    held_use_distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if moving {
        return false;
    }
    in_use_range_ex(
        px,
        py,
        tx,
        ty,
        held_use_distance,
        map_w,
        map_h,
        wrap,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_use_distance_clamps() {
        assert_eq!(effective_use_distance(0), 1);
        assert_eq!(effective_use_distance(-3), 1);
        assert_eq!(effective_use_distance(5), 5);
    }

    #[test]
    fn in_use_range_diagonal_fails_at_one() {
        assert!(in_use_range(0, 0, 1, 0, 1));
        assert!(!in_use_range(0, 0, 1, 1, 1)); // diag sq=2
        assert!(in_use_range(0, 0, 1, 1, 2));
        assert!(is_close(0, 0, 99, 0, 1, 100, 100, true));
        assert!(!is_close(0, 0, 99, 0, 1, 100, 100, false));
    }

    #[test]
    fn check_not_moving_gate() {
        assert!(!check_if_not_moving_and_close_enough(
            true, 0, 0, 1, 0, 1, 0, 0, false
        ));
        assert!(check_if_not_moving_and_close_enough(
            false, 0, 0, 1, 0, 1, 0, 0, false
        ));
    }
}
