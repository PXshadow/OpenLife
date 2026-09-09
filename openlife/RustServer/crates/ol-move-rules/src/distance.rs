//! Pure map distance helpers (Haxe AiHelper / MoveHelper distance cores).

/// Squared Euclidean tile distance with optional torus wrap (integer tile coords).
// Haxe: AiHelper.CalculateDistance (squared) + WorldMap wrap
#[inline]
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

/// Haxe `MoveHelper.calculateExactQuadDistance` core — squared float distance with optional wrap.
// Haxe: MoveHelper.calculateExactQuadDistance + WorldMap.transformFloatX/Y
#[inline]
pub fn calculate_exact_quad_distance_f(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f64 {
    let mut dx = ax - bx;
    let mut dy = ay - by;
    if wrap && map_w > 0 && map_h > 0 {
        let half_w = map_w as f64 / 2.0;
        let half_h = map_h as f64 / 2.0;
        if dx > half_w {
            dx -= map_w as f64;
        } else if dx < -half_w {
            dx += map_w as f64;
        }
        if dy > half_h {
            dy -= map_h as f64;
        } else if dy < -half_h {
            dy += map_h as f64;
        }
    }
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_sq_plane_and_wrap() {
        assert!((calculate_distance_sq(0, 0, 3, 4, 100, 100, false) - 25.0).abs() < 1e-9);
        // 10-wide: 0 → 9 is dist 1 → sq 1
        assert!((calculate_distance_sq(0, 0, 9, 0, 10, 10, true) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn exact_quad_wrap() {
        let q = calculate_exact_quad_distance_f(0.0, 0.0, 9.0, 0.0, 10, 10, true);
        assert!((q - 1.0).abs() < 1e-9);
    }
}
