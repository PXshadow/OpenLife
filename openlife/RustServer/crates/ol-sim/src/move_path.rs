//! Timed multi-tile movement (Haxe `MoveHelper` subset).
//!
//! When `timed_movement` is off, callers keep using instant [`crate::apply_move_deltas`].
//! When on, paths are accepted into [`MovePath`], advanced by `speed * dt` each tick,
//! and broadcast as PM on accept.

use crate::player::Player;
use crate::pathfind::is_walkable;
use crate::WALK_MOVE_SPEED;
use ol_content::ContentDb;
use ol_world::World;
use std::collections::VecDeque;

/// Default walk speed (tiles/s) — Haxe `InitialPlayerMoveSpeed`.
pub const DEFAULT_MOVE_SPEED: f32 = WALK_MOVE_SPEED;

/// Epsilon for residual distance comparisons.
const EPS: f32 = 1e-5;

/// Active multi-tile path on a player (`Some` ⇔ `Player::moving`).
#[derive(Debug, Clone)]
pub struct MovePath {
    pub start_x: i32,
    pub start_y: i32,
    /// Remaining relative steps from current tile (dx, dy per step).
    /// `VecDeque` avoids O(n²) on long paths (`pop_front`).
    pub remaining: VecDeque<(i32, i32)>,
    /// Frozen at accept.
    pub speed: f32,
    pub length: f32,
    pub total_sec: f32,
    /// Haxe `startingMoveTicks`.
    pub start_tick: u64,
    /// Haxe `timeExactPositionChangedLast` (bookkeeping; advance uses incremental dt).
    pub step_anchor_tick: u64,
    /// Distance already covered along current step `[0, step_len)`.
    pub step_progress: f32,
    pub exact_x: f32,
    pub exact_y: f32,
    /// Client `@seq` or server-assigned.
    pub seq: i32,
    /// 1 if walkability dropped any client delta; 0 if full path accepted.
    pub trunc: i32,
}

/// Why a path start was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveReject {
    NoPlayer,
    Deleted,
    Sleeping,
    Sitting,
    EmptyPath,
    JumpTooFar,
    BlockedStart,
}

impl MoveReject {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoPlayer => "no_player",
            Self::Deleted => "deleted",
            Self::Sleeping => "sleeping",
            Self::Sitting => "sitting",
            Self::EmptyPath => "empty_path",
            Self::JumpTooFar => "jump_too_far",
            Self::BlockedStart => "blocked_start",
        }
    }
}

/// `true` when the player has an active path.
#[inline]
pub fn is_moving(p: &Player) -> bool {
    p.move_path.is_some()
}

/// Resolve path seq: prefer client `@seq` if > 0; else server counter.
pub fn resolve_move_seq(player: &Player, client_seq: Option<i32>) -> i32 {
    client_seq
        .filter(|&s| s > 0)
        .unwrap_or_else(|| player.done_moving_seq.saturating_add(1).max(1))
}

/// Squared-Euclidean range check (Haxe `isClose`).
///
/// With default `use_distance = 1`, diagonal tiles fail (`2 ≰ 1`).
#[inline]
pub fn in_use_range(px: i32, py: i32, tx: i32, ty: i32, use_distance: i32) -> bool {
    let dx = (px - tx) as i64;
    let dy = (py - ty) as i64;
    let d = use_distance as i64;
    dx * dx + dy * dy <= d * d
}

/// Path length over accepted deltas (cardinal 1.0, diagonal √2).
pub fn calculate_length(deltas: &[(i32, i32)]) -> f32 {
    let mut len = 0.0f32;
    for &(dx, dy) in deltas {
        let fx = dx as f32;
        let fy = dy as f32;
        len += (fx * fx + fy * fy).sqrt();
    }
    len
}

/// Round to 2 decimal places (Haxe `Math.round(x*100)/100`).
pub fn round2(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// Haxe BAD_BIOMES / impassable mountain wall.
const BIOME_MOUNTAIN: u8 = 21;

#[inline]
pub fn biome_blocks_move(biome: u8) -> bool {
    biome == BIOME_MOUNTAIN
}

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

/// Inverse of [`client_path_deltas_to_steps`]: steps → start-relative waypoint deltas
/// (for PM wire, which mirrors the client format).
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

/// Walk client path deltas (start-relative waypoints); keep steps while walkable.
///
/// Returns `(accepted_steps, trunc)` where accepted entries are **per-step** deltas
/// for [`advance_path`] / instant apply. `trunc = 1` if any client waypoint dropped.
pub fn truncate_walkable(
    world: &World,
    content: &ContentDb,
    start_x: i32,
    start_y: i32,
    deltas: &[(i32, i32)],
) -> (Vec<(i32, i32)>, i32) {
    let steps = client_path_deltas_to_steps(deltas);
    let mut accepted = Vec::with_capacity(steps.len());
    let mut x = start_x;
    let mut y = start_y;
    for &(dx, dy) in &steps {
        // Zero-length step (duplicate waypoint) — skip without advancing.
        if dx == 0 && dy == 0 {
            continue;
        }
        let (nx, ny) = world.wrap_tile(x + dx, y + dy);
        if biome_blocks_move(world.get_biome(nx, ny)) {
            break;
        }
        if !is_walkable(world, content, nx, ny) {
            break;
        }
        accepted.push((dx, dy));
        x = nx;
        y = ny;
    }
    let trunc = if accepted.len() < steps.iter().filter(|&&(dx, dy)| dx != 0 || dy != 0).count() {
        1
    } else {
        0
    };
    (accepted, trunc)
}

/// Build a `MovePath` from accepted deltas (does not mutate player).
pub fn build_move_path(
    start_x: i32,
    start_y: i32,
    accepted: Vec<(i32, i32)>,
    speed: f32,
    seq: i32,
    trunc: i32,
    tick: u64,
) -> MovePath {
    let speed = if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        DEFAULT_MOVE_SPEED
    };
    let length = calculate_length(&accepted);
    let total_sec = if length > 0.0 {
        length / speed
    } else {
        0.0
    };
    MovePath {
        start_x,
        start_y,
        remaining: accepted.into(),
        speed,
        length,
        total_sec,
        start_tick: tick,
        step_anchor_tick: tick,
        step_progress: 0.0,
        exact_x: start_x as f32,
        exact_y: start_y as f32,
        seq,
        trunc,
    }
}

/// Chebyshev distance.
#[inline]
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Squared Euclidean distance between tiles (Haxe `quadDist` / `xDiff²+yDiff²`).
#[inline]
pub fn quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = (ax - bx) as i64;
    let dy = (ay - by) as i64;
    dx * dx + dy * dy
}

/// Haxe `ServerSettings.MaxMovementQuadJumpDistanceBeforeForce` (default 5).
///
/// Client MOVE start may differ from server by up to this squared distance
/// without CancleMovement — e.g. (2,0)=4 and (2,1)=5 accept; (3,0)=9 rejects.
pub const MAX_MOVE_QUAD_JUMP_BEFORE_FORCE: i64 = 5;

/// One step length in tiles.
#[inline]
pub fn step_len(dx: i32, dy: i32) -> f32 {
    let fx = dx as f32;
    let fy = dy as f32;
    (fx * fx + fy * fy).sqrt()
}

/// Advance a single path by `budget = speed * dt` tiles (incremental residual).
///
/// Returns `(tile_commits, finished)` where `tile_commits` are absolute tiles
/// landed on (after wrap), and `finished` means path completed (remaining empty).
pub fn advance_path(
    path: &mut MovePath,
    tile_x: &mut i32,
    tile_y: &mut i32,
    dt: f32,
    tick: u64,
    wrap: &dyn Fn(i32, i32) -> (i32, i32),
    is_blocked: &dyn Fn(i32, i32) -> bool,
) -> AdvanceResult {
    let mut budget = path.speed * dt.max(0.0);
    let mut commits = Vec::new();
    let mut cancelled = false;

    while !path.remaining.is_empty() && budget > EPS {
        let (dx, dy) = *path.remaining.front().unwrap();
        let sl = step_len(dx, dy);
        if sl <= EPS {
            path.remaining.pop_front();
            path.step_progress = 0.0;
            continue;
        }
        let need = sl - path.step_progress;
        if budget + EPS >= need {
            budget -= need;
            let (nx, ny) = wrap(*tile_x + dx, *tile_y + dy);
            if is_blocked(nx, ny) {
                cancelled = true;
                break;
            }
            *tile_x = nx;
            *tile_y = ny;
            path.remaining.pop_front();
            path.step_progress = 0.0;
            path.step_anchor_tick = tick;
            path.exact_x = *tile_x as f32;
            path.exact_y = *tile_y as f32;
            commits.push((*tile_x, *tile_y));
        } else {
            path.step_progress += budget;
            let t = path.step_progress / sl;
            path.exact_x = *tile_x as f32 + dx as f32 * t;
            path.exact_y = *tile_y as f32 + dy as f32 * t;
            budget = 0.0;
        }
    }

    let finished = path.remaining.is_empty() && !cancelled;
    AdvanceResult {
        commits,
        finished,
        cancelled,
    }
}

#[derive(Debug, Clone, Default)]
pub struct AdvanceResult {
    pub commits: Vec<(i32, i32)>,
    pub finished: bool,
    pub cancelled: bool,
}

/// PM body line (without tag wrapper): `p_id xs ys total eta trunc dx0 dy0 ...`
pub fn format_pm_body(
    p_id: i32,
    xs: i32,
    ys: i32,
    total_sec: f32,
    eta_sec: f32,
    trunc: i32,
    deltas: &[(i32, i32)],
) -> String {
    let total = round2(total_sec);
    let eta = round2(eta_sec);
    let mut s = format!("{p_id} {xs} {ys} {total:.2} {eta:.2} {trunc}");
    for &(dx, dy) in deltas {
        s.push_str(&format!(" {dx} {dy}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_seq_prefers_client() {
        let mut p = Player::new(1, 1, "a@b.c");
        p.done_moving_seq = 3;
        assert_eq!(resolve_move_seq(&p, Some(5)), 5);
        assert_eq!(resolve_move_seq(&p, Some(0)), 4);
        assert_eq!(resolve_move_seq(&p, None), 4);
    }

    #[test]
    fn quad_dist_haxe_jump_threshold() {
        // MaxMovementQuadJumpDistanceBeforeForce = 5
        assert_eq!(quad_dist(0, 0, 0, 0), 0);
        assert_eq!(quad_dist(0, 0, 2, 0), 4); // accept
        assert_eq!(quad_dist(0, 0, 2, 1), 5); // accept (boundary)
        assert_eq!(quad_dist(0, 0, 3, 0), 9); // reject
        assert_eq!(quad_dist(0, 0, 2, 2), 8); // reject
        assert!(quad_dist(0, 0, 2, 1) <= MAX_MOVE_QUAD_JUMP_BEFORE_FORCE);
        assert!(quad_dist(0, 0, 3, 0) > MAX_MOVE_QUAD_JUMP_BEFORE_FORCE);
    }

    #[test]
    fn in_use_range_squared_euclidean() {
        // d=1: orthogonal OK, diagonal fails
        assert!(in_use_range(0, 0, 1, 0, 1));
        assert!(in_use_range(0, 0, 0, 0, 1));
        assert!(!in_use_range(0, 0, 1, 1, 1));
        // d=2: diagonal OK (2 <= 4)
        assert!(in_use_range(0, 0, 1, 1, 2));
        assert!(!in_use_range(0, 0, 2, 2, 2)); // 8 > 4
    }

    #[test]
    fn client_path_deltas_are_start_relative_not_step_sum() {
        // LivingLifePage: MOVE xs ys @seq 1 0 2 0 → dest = start+(2,0), not +(3,0)
        let steps = client_path_deltas_to_steps(&[(1, 0), (2, 0)]);
        assert_eq!(steps, vec![(1, 0), (1, 0)]);
        assert_eq!(steps_to_client_path_deltas(&steps), vec![(1, 0), (2, 0)]);
        let mut x = 488;
        let mut y = 488;
        for (dx, dy) in steps {
            x += dx;
            y += dy;
        }
        assert_eq!((x, y), (490, 488));
    }

    #[test]
    fn length_cardinal_and_diagonal() {
        assert!((calculate_length(&[(1, 0)]) - 1.0).abs() < 1e-5);
        assert!((calculate_length(&[(1, 0), (0, 1)]) - 2.0).abs() < 1e-5);
        let diag = calculate_length(&[(1, 1)]);
        assert!((diag - std::f32::consts::SQRT_2).abs() < 1e-5);
    }

    #[test]
    fn total_sec_cardinal_at_walk_speed() {
        let path = build_move_path(10, 20, vec![(1, 0)], 3.75, 1, 0, 0);
        assert!((path.total_sec - 1.0 / 3.75).abs() < 1e-5);
        assert!((round2(path.total_sec) - 0.27).abs() < 1e-5);
    }

    #[test]
    fn total_sec_diagonal() {
        let path = build_move_path(10, 20, vec![(1, 1)], 3.75, 1, 0, 0);
        let expected = std::f32::consts::SQRT_2 / 3.75;
        assert!((path.total_sec - expected).abs() < 1e-4);
        assert!((round2(path.total_sec) - 0.38).abs() < 1e-5);
    }

    #[test]
    fn advance_one_step_cardinal() {
        let mut path = build_move_path(0, 0, vec![(1, 0)], 3.75, 5, 0, 0);
        let mut x = 0;
        let mut y = 0;
        // full step needs 1/3.75 ≈ 0.2667 s
        let r = advance_path(
            &mut path,
            &mut x,
            &mut y,
            0.3,
            1,
            &|a, b| (a, b),
            &|_, _| false,
        );
        assert!(r.finished);
        assert_eq!((x, y), (1, 0));
        assert_eq!(path.seq, 5);
    }

    #[test]
    fn advance_partial_keeps_progress() {
        let mut path = build_move_path(0, 0, vec![(1, 0)], 1.0, 1, 0, 0);
        let mut x = 0;
        let mut y = 0;
        let r = advance_path(
            &mut path,
            &mut x,
            &mut y,
            0.4,
            1,
            &|a, b| (a, b),
            &|_, _| false,
        );
        assert!(!r.finished);
        assert_eq!((x, y), (0, 0));
        assert!((path.step_progress - 0.4).abs() < 1e-5);
        let r2 = advance_path(
            &mut path,
            &mut x,
            &mut y,
            0.7,
            2,
            &|a, b| (a, b),
            &|_, _| false,
        );
        assert!(r2.finished);
        assert_eq!((x, y), (1, 0));
    }

    #[test]
    fn advance_cancelled_on_block() {
        let mut path = build_move_path(0, 0, vec![(1, 0)], 10.0, 1, 0, 0);
        let mut x = 0;
        let mut y = 0;
        let r = advance_path(
            &mut path,
            &mut x,
            &mut y,
            1.0,
            1,
            &|a, b| (a, b),
            &|nx, _| nx == 1,
        );
        assert!(r.cancelled);
        assert!(!r.finished);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn pm_body_golden_one_step_east() {
        // p_id=7, start (10,20), one step east, speed=3.75 → total=eta=0.27, trunc=0
        let total = round2(1.0 / 3.75);
        let body = format_pm_body(7, 10, 20, total, total, 0, &[(1, 0)]);
        assert_eq!(body, "7 10 20 0.27 0.27 0 1 0");
    }

    #[test]
    fn pm_body_golden_two_steps() {
        let total = round2(2.0 / 3.75);
        let body = format_pm_body(7, 10, 20, total, total, 0, &[(1, 0), (0, 1)]);
        assert_eq!(body, "7 10 20 0.53 0.53 0 1 0 0 1");
    }

    #[test]
    fn pm_body_golden_diagonal() {
        let total = round2(std::f32::consts::SQRT_2 / 3.75);
        let body = format_pm_body(7, 10, 20, total, total, 0, &[(1, 1)]);
        assert_eq!(body, "7 10 20 0.38 0.38 0 1 1");
    }

    #[test]
    fn is_moving_tracks_option() {
        let mut p = Player::new(1, 1, "a@b.c");
        assert!(!is_moving(&p));
        p.move_path = Some(build_move_path(0, 0, vec![(1, 0)], 3.75, 1, 0, 0));
        assert!(is_moving(&p));
    }

    #[test]
    fn advance_multi_step_budget_crosses_steps() {
        // two east steps, speed=1; dt=1.3 → commit first, progress 0.3 on second
        let mut path = build_move_path(0, 0, vec![(1, 0), (1, 0)], 1.0, 1, 0, 0);
        let mut x = 0;
        let mut y = 0;
        let r = advance_path(
            &mut path,
            &mut x,
            &mut y,
            1.3,
            1,
            &|a, b| (a, b),
            &|_, _| false,
        );
        assert!(!r.finished);
        assert_eq!((x, y), (1, 0));
        assert_eq!(path.remaining.len(), 1);
        assert!((path.step_progress - 0.3).abs() < 1e-4);
    }

    #[test]
    fn pm_body_trunc_one() {
        let total = round2(1.0 / 3.75);
        let body = format_pm_body(7, 10, 20, total, total, 1, &[(1, 0)]);
        assert_eq!(body, "7 10 20 0.27 0.27 1 1 0");
    }

    #[test]
    fn truncate_walkable_partial_and_full() {
        use ol_content::ObjectDef;
        use ol_world::World;
        use std::sync::Arc;

        let mut db = ContentDb::default();
        db.objects.insert(
            99,
            ObjectDef {
                id: 99,
                description: "Wall".into(),
                name: "Wall".into(),
                containable: false,
                permanent: true,
                blocks_walking: true,
                food_value: 0,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: vec![],
                num_uses: 0,
                num_slots: 0,
                floor: false,
            dummy_ids: Vec::new(),
            },
        );
        let mut w = World::new(32, 32, false);
        w.set_object(2, 0, 99); // block second east waypoint from (0,0)
        let content = Arc::new(db);
        // Client path deltas are start-relative: (1,0) then (2,0).
        let (acc, trunc) = truncate_walkable(&w, &content, 0, 0, &[(1, 0), (2, 0)]);
        assert_eq!(acc, vec![(1, 0)]);
        assert_eq!(trunc, 1);
        let (full, t0) = truncate_walkable(&w, &content, 0, 0, &[(0, 1)]);
        assert_eq!(full, vec![(0, 1)]);
        assert_eq!(t0, 0);
        let (empty, t1) = truncate_walkable(&w, &content, 1, 0, &[(1, 0)]);
        assert!(empty.is_empty());
        assert_eq!(t1, 1);
    }
}
