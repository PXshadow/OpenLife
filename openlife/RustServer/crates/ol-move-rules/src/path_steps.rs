//! Client MOVE path delta ↔ per-step helpers (Haxe calculateNewMovements input).

/// Haxe `calculateNewMovements` path length cap (`count > 10`).
pub const MAX_CLIENT_PATH_STEPS: usize = 10;

/// Convert client MOVE path deltas into **per-step** (dx,dy) from the previous waypoint.
///
/// Official protocol / LivingLifePage: each pair is relative to path **start**
/// `(xs,ys)`, not to the previous step. Example:
/// `deltas [(1,0),(2,0)]` → waypoints start+(1,0), start+(2,0) → steps `[(1,0),(1,0)]`.
pub fn client_path_deltas_to_steps(deltas: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut steps = Vec::with_capacity(deltas.len());
    let mut prev_dx = 0i32;
    let mut prev_dy = 0i32;
    for &(dx, dy) in deltas {
        steps.push((dx - prev_dx, dy - prev_dy));
        prev_dx = dx;
        prev_dy = dy;
    }
    steps
}

/// Inverse of [`client_path_deltas_to_steps`]: steps → start-relative waypoint deltas.
pub fn steps_to_client_path_deltas(steps: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(steps.len());
    let mut ax = 0i32;
    let mut ay = 0i32;
    for &(dx, dy) in steps {
        ax += dx;
        ay += dy;
        out.push((ax, ay));
    }
    out
}

/// Euclidean length of one path step (cardinal or diagonal).
#[inline]
pub fn step_len(dx: i32, dy: i32) -> f32 {
    let fx = dx as f32;
    let fy = dy as f32;
    (fx * fx + fy * fy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_deltas_to_steps_relative_to_start() {
        assert_eq!(
            client_path_deltas_to_steps(&[(1, 0), (2, 0)]),
            vec![(1, 0), (1, 0)]
        );
        assert_eq!(
            steps_to_client_path_deltas(&[(1, 0), (1, 0)]),
            vec![(1, 0), (2, 0)]
        );
    }
}
