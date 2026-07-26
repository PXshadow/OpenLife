//! Pure toroidal (wrap-around) map math.
//!
//! Mirrors OHOL / OpenLife `wrap` worlds: coordinates live on a torus of
//! size `width × height`. All helpers are side-effect free.

/// Wrap a single axis into `[0, size)` when `size > 0`.
///
/// Negative values wrap correctly via Euclidean remainder.
#[inline]
pub fn wrap_axis(v: i32, size: i32) -> i32 {
    if size <= 0 {
        return v;
    }
    v.rem_euclid(size)
}

/// Wrap tile `(x, y)` into the map rectangle when `wrap` is enabled.
#[inline]
pub fn wrap_tile(x: i32, y: i32, width: i32, height: i32, wrap: bool) -> (i32, i32) {
    if !wrap || width <= 0 || height <= 0 {
        return (x, y);
    }
    (wrap_axis(x, width), wrap_axis(y, height))
}

/// Shortest signed delta on a single toroidal axis.
///
/// Result is in `(-size/2, size/2]` for `size > 0`.
pub fn wrap_delta_1d(from: i32, to: i32, size: i32) -> i32 {
    if size <= 0 {
        return to - from;
    }
    let a = wrap_axis(from, size);
    let b = wrap_axis(to, size);
    let mut d = b - a;
    let half = size / 2;
    // Keep deltas in (-half, half]; when d == -half (even size) leave as-is
    // so size=2 yields ±1 instead of flipping both to +1.
    if d > half {
        d -= size;
    } else if d < -half {
        d += size;
    }
    d
}

/// Shortest signed `(dx, dy)` from `a` to `b` on a torus (or plane if `!wrap`).
pub fn wrap_delta(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    width: i32,
    height: i32,
    wrap: bool,
) -> (i32, i32) {
    if !wrap || width <= 0 || height <= 0 {
        return (bx - ax, by - ay);
    }
    (
        wrap_delta_1d(ax, bx, width),
        wrap_delta_1d(ay, by, height),
    )
}

/// Chebyshev distance with optional toroidal wrap.
pub fn chebyshev_wrap(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    width: i32,
    height: i32,
    wrap: bool,
) -> i32 {
    let (dx, dy) = wrap_delta(ax, ay, bx, by, width, height, wrap);
    dx.abs().max(dy.abs())
}

/// Manhattan distance with optional toroidal wrap.
pub fn manhattan_wrap(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    width: i32,
    height: i32,
    wrap: bool,
) -> i32 {
    let (dx, dy) = wrap_delta(ax, ay, bx, by, width, height, wrap);
    dx.abs() + dy.abs()
}

/// Euclidean distance (f32) with optional toroidal wrap.
pub fn euclidean_wrap(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    width: i32,
    height: i32,
    wrap: bool,
) -> f32 {
    let (dx, dy) = wrap_delta(ax, ay, bx, by, width, height, wrap);
    ((dx as f32) * (dx as f32) + (dy as f32) * (dy as f32)).sqrt()
}

/// Step one tile from `(x, y)` by unit direction `(sx, sy)` (each in -1..=1), wrapping.
pub fn step_wrap(
    x: i32,
    y: i32,
    sx: i32,
    sy: i32,
    width: i32,
    height: i32,
    wrap: bool,
) -> (i32, i32) {
    wrap_tile(x + sx.clamp(-1, 1), y + sy.clamp(-1, 1), width, height, wrap)
}

/// `WRAP x y` — wrapped coordinates for a point (query formatter).
pub fn format_wrap_query(x: i32, y: i32, width: i32, height: i32, wrap: bool) -> String {
    let (wx, wy) = wrap_tile(x, y, width, height, wrap);
    if wrap {
        format!("WRAP {wx} {wy} size={width}x{height}")
    } else {
        format!("WRAP {wx} {wy} nowrap")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_axis_positive_and_negative() {
        assert_eq!(wrap_axis(0, 10), 0);
        assert_eq!(wrap_axis(9, 10), 9);
        assert_eq!(wrap_axis(10, 10), 0);
        assert_eq!(wrap_axis(11, 10), 1);
        assert_eq!(wrap_axis(-1, 10), 9);
        assert_eq!(wrap_axis(-10, 10), 0);
        assert_eq!(wrap_axis(-11, 10), 9);
        assert_eq!(wrap_axis(25, 10), 5);
        // size <= 0: identity
        assert_eq!(wrap_axis(5, 0), 5);
        assert_eq!(wrap_axis(-3, -1), -3);
    }

    #[test]
    fn wrap_tile_enabled_and_disabled() {
        assert_eq!(wrap_tile(12, -1, 10, 20, true), (2, 19));
        assert_eq!(wrap_tile(12, -1, 10, 20, false), (12, -1));
        assert_eq!(wrap_tile(0, 0, 100, 100, true), (0, 0));
        assert_eq!(wrap_tile(99, 99, 100, 100, true), (99, 99));
        assert_eq!(wrap_tile(100, 100, 100, 100, true), (0, 0));
    }

    #[test]
    fn wrap_delta_shortest_path() {
        // Plane
        assert_eq!(wrap_delta(0, 0, 3, 4, 10, 10, false), (3, 4));
        // Torus 10x10: from 0 to 9 is -1 (or +9); shortest is -1
        assert_eq!(wrap_delta(0, 0, 9, 0, 10, 10, true), (-1, 0));
        assert_eq!(wrap_delta(0, 0, 1, 0, 10, 10, true), (1, 0));
        // Half width: size 10, half 5 → from 0 to 5 is +5
        assert_eq!(wrap_delta_1d(0, 5, 10), 5);
        // from 0 to 6 → -4
        assert_eq!(wrap_delta_1d(0, 6, 10), -4);
        // identity
        assert_eq!(wrap_delta(5, 5, 5, 5, 10, 10, true), (0, 0));
    }

    #[test]
    fn chebyshev_and_manhattan_torus() {
        // Across the wrap edge: (0,0) to (9,0) on 10-wide is dist 1
        assert_eq!(chebyshev_wrap(0, 0, 9, 0, 10, 10, true), 1);
        assert_eq!(manhattan_wrap(0, 0, 9, 0, 10, 10, true), 1);
        // Plane would be 9
        assert_eq!(chebyshev_wrap(0, 0, 9, 0, 10, 10, false), 9);
        // Diagonal wrap
        assert_eq!(chebyshev_wrap(0, 0, 9, 9, 10, 10, true), 1);
        assert_eq!(manhattan_wrap(0, 0, 9, 9, 10, 10, true), 2);
        // Same tile
        assert_eq!(chebyshev_wrap(3, 4, 3, 4, 50, 50, true), 0);
        assert_eq!(manhattan_wrap(3, 4, 3, 4, 50, 50, true), 0);
    }

    #[test]
    fn euclidean_wrap_basic() {
        let d = euclidean_wrap(0, 0, 3, 4, 100, 100, false);
        assert!((d - 5.0).abs() < 1e-5);
        let d_wrap = euclidean_wrap(0, 0, 9, 0, 10, 10, true);
        assert!((d_wrap - 1.0).abs() < 1e-5);
    }

    #[test]
    fn step_wrap_clamps_and_wraps() {
        assert_eq!(step_wrap(0, 0, -1, 0, 10, 10, true), (9, 0));
        assert_eq!(step_wrap(9, 5, 1, 0, 10, 10, true), (0, 5));
        assert_eq!(step_wrap(5, 5, 5, 5, 10, 10, true), (6, 6)); // clamp to 1
        assert_eq!(step_wrap(5, 5, -9, 0, 10, 10, true), (4, 5));
        assert_eq!(step_wrap(0, 0, -1, 0, 10, 10, false), (-1, 0));
    }

    #[test]
    fn format_wrap_query_strings() {
        assert_eq!(
            format_wrap_query(12, -1, 10, 20, true),
            "WRAP 2 19 size=10x20"
        );
        assert_eq!(format_wrap_query(12, -1, 10, 20, false), "WRAP 12 -1 nowrap");
    }

    #[test]
    fn wrap_delta_1d_size_one_and_two() {
        assert_eq!(wrap_delta_1d(0, 0, 1), 0);
        // size 2: from 0 to 1 → +1 (half=1, d=1 is not > half)
        assert_eq!(wrap_delta_1d(0, 1, 2), 1);
        assert_eq!(wrap_delta_1d(1, 0, 2), -1);
    }

    #[test]
    fn large_map_identity_near_center() {
        let (dx, dy) = wrap_delta(100, 100, 110, 95, 500, 500, true);
        assert_eq!((dx, dy), (10, -5));
        assert_eq!(chebyshev_wrap(100, 100, 110, 95, 500, 500, true), 10);
    }
}
