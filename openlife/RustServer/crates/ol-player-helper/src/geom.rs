//! Shared geometry used by AI and player-facing sim helpers.

/// Haxe `CountCloseObjects` half-open square: center `(tx,ty)`, point `(ox,oy)`.
// Haxe: for (ty in baseY-radius...baseY+radius)
#[inline]
pub fn in_count_close_square(tx: i32, ty: i32, ox: i32, oy: i32, radius: i32) -> bool {
    ox >= tx - radius && ox < tx + radius && oy >= ty - radius && oy < ty + radius
}

/// Haxe `AiHelper.CalculateDistance` — squared Euclidean with optional torus wrap.
// Haxe: AiHelper.CalculateDistance
pub fn calculate_distance_sq(
    base_x: i32,
    base_y: i32,
    to_x: i32,
    to_y: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f64 {
    let mut diff_x = (to_x - base_x) as f64;
    let mut diff_y = (to_y - base_y) as f64;
    if wrap && map_w > 0 && map_h > 0 {
        let half_w = map_w as f64 / 2.0;
        let half_h = map_h as f64 / 2.0;
        if diff_x > half_w {
            diff_x -= map_w as f64;
        } else if diff_x < -half_w {
            diff_x += map_w as f64;
        }
        if diff_y > half_h {
            diff_y -= map_h as f64;
        } else if diff_y < -half_h {
            diff_y += map_h as f64;
        }
    }
    diff_x * diff_x + diff_y * diff_y
}

/// Chebyshev distance (re-export style helper; same as [`ol_ai_api::chebyshev`]).
#[inline]
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    ol_ai_api::chebyshev(ax, ay, bx, by)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_close_half_open() {
        assert!(in_count_close_square(0, 0, 0, 0, 10));
        assert!(in_count_close_square(0, 0, -10, 0, 10));
        assert!(!in_count_close_square(0, 0, 10, 0, 10));
    }

    #[test]
    fn distance_sq_no_wrap() {
        let d = calculate_distance_sq(0, 0, 3, 4, 0, 0, false);
        assert!((d - 25.0).abs() < 1e-9);
    }
}
