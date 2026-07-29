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
    /// Original start-relative waypoints (client MOVE form) for mid-path recon.
    /// Haxe: `newMovements.moves` snapshot at accept (before step rewrites).
    // Haxe: MoveHelper.calculateNewPos
    pub original_waypoints: Vec<(i32, i32)>,
    /// `SimState.sim_time` when path was accepted (Haxe startingMoveTicks → seconds).
    // Haxe: MoveHelper.startingMoveTicks
    pub start_sim_time: f32,
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
    /// Haxe `waitForForce` — silent ignore (no CancleMovement).
    WaitForForce,
    /// Haxe `age * 60 < MinMovementAgeInSec` — non-force PU with done_moving seq.
    TooYoung,
    /// Haxe jump rate-limit: `ceil(jumpedTiles) >= MaxJumpsPerTenSec`.
    JumpRateLimited,
    /// Haxe world-wrap CancleMovement already applied (VOG+force); ignore in handler.
    // Haxe: MoveHelper.moveHelper L550-586
    WorldWrapped,
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
            Self::WaitForForce => "wait_for_force",
            Self::TooYoung => "too_young",
            Self::JumpRateLimited => "jump_rate_limited",
            Self::WorldWrapped => "world_wrapped",
        }
    }

    /// Haxe waitForForce / already-cancelled world-wrap: no second CancleMovement.
    #[inline]
    pub fn is_silent(self) -> bool {
        matches!(self, Self::WaitForForce | Self::WorldWrapped)
    }

    /// Haxe age gate: done_moving PU without forced teleport.
    #[inline]
    pub fn is_soft_reject(self) -> bool {
        matches!(self, Self::TooYoung)
    }

    /// Haxe `CancleMovement(..., useTeleport)` default paths that always VOG.
    /// Jump/blocked nearby force uses `cancel_should_use_vog(jump_quad)` instead.
    // Haxe: MoveHelper.CancleMovement useTeleport
    #[inline]
    pub fn cancel_with_vog(self) -> bool {
        matches!(self, Self::EmptyPath | Self::WorldWrapped)
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
    crate::move_live_gates::calculate_distance_sq(px, py, tx, ty, map_w, map_h, wrap) <= max_sq
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

/// Haxe `calculateNewMovements` path length cap (`count > 10`).
pub const MAX_CLIENT_PATH_STEPS: usize = 10;

/// Quick check without floor (biome speed &lt; 0.1). Prefer [`biome_blocks_move_at`].
#[inline]
pub fn biome_blocks_move(biome: u8) -> bool {
    ol_world::biome_speed(biome) < 0.1
}

/// Haxe `WorldMap.isBiomeBlocking` at a tile (floor exception parity).
#[inline]
pub fn biome_blocks_move_at(world: &World, x: i32, y: i32) -> bool {
    let biome = world.get_biome(x, y);
    let floor = world.get_floor(x, y) as i32;
    ol_world::is_biome_blocking(biome, floor)
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
///
/// Haxe `calculateNewMovements`: stop on `isBlocked` **or** `count > 10` (no keep of
/// blocking ocean tile — unlike off-road biome-speed trunc which keeps the border).
pub fn truncate_walkable(
    world: &World,
    content: &ContentDb,
    start_x: i32,
    start_y: i32,
    deltas: &[(i32, i32)],
) -> (Vec<(i32, i32)>, i32) {
    let steps = client_path_deltas_to_steps(deltas);
    let mut accepted = Vec::with_capacity(steps.len().min(MAX_CLIENT_PATH_STEPS));
    let mut x = start_x;
    let mut y = start_y;
    let mut count = 0usize;
    for &(dx, dy) in &steps {
        // Zero-length step (duplicate waypoint) — skip without advancing.
        if dx == 0 && dy == 0 {
            continue;
        }
        count += 1;
        // Haxe: `if (p.isBlocked(tmpX, tmpY) || count > 10)`
        if count > MAX_CLIENT_PATH_STEPS {
            break;
        }
        let (nx, ny) = world.wrap_tile(x + dx, y + dy);
        if biome_blocks_move_at(world, nx, ny) {
            break;
        }
        if !is_walkable(world, content, nx, ny) {
            break;
        }
        accepted.push((dx, dy));
        x = nx;
        y = ny;
    }
    let nonzero = steps.iter().filter(|&&(dx, dy)| dx != 0 || dy != 0).count();
    let trunc = if accepted.len() < nonzero { 1 } else { 0 };
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
    let original_waypoints = steps_to_client_path_deltas(&accepted);
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
        original_waypoints,
        start_sim_time: 0.0,
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
/// Live path uses `GameplayKnobs::max_move_quad_jump_before_force` (f64).
// Haxe: ServerSettings.MaxMovementQuadJumpDistanceBeforeForce = 5
// C-SS-MORE-KNOBS
pub const MAX_MOVE_QUAD_JUMP_BEFORE_FORCE: i64 = 5;

/// True when timed MOVE jump exceeds the force threshold (after floor softener).
// Haxe: MoveHelper.moveHelper L598 quadDist > MaxMovementQuadJumpDistanceBeforeForce
// C-SS-MORE-KNOBS
#[inline]
pub fn jump_exceeds_force_threshold(jump_quad: f64, max_quad: f64) -> bool {
    let max = if max_quad.is_finite() && max_quad > 0.0 {
        max_quad
    } else {
        MAX_MOVE_QUAD_JUMP_BEFORE_FORCE as f64
    };
    jump_quad > max
}

/// Haxe CancleMovement `useTeleport` when jump/blocked: `quadDist > 25`.
// Haxe: MoveHelper.moveHelper L600 / L611
pub const CANCEL_VOG_QUAD_THRESHOLD: i64 = 25;

/// True when CancleMovement should send VOG_UPDATE for a jump-distance reject.
// Haxe: CancleMovement(p, seq, quadDist > 25)
#[inline]
pub fn cancel_should_use_vog(jump_quad: f64) -> bool {
    jump_quad > CANCEL_VOG_QUAD_THRESHOLD as f64
}

/// Fold birth-relative coords when |x| or |y| reaches map size (one fold per axis).
///
/// Haxe `MoveHelper.moveHelper` L550-559 — not full rem_euclid; single ±width/height.
// Haxe: MoveHelper.moveHelper L550-559
pub fn fold_relative_around_world(
    rel_x: i32,
    rel_y: i32,
    width: i32,
    height: i32,
) -> (i32, i32, bool) {
    if width <= 0 || height <= 0 {
        return (rel_x, rel_y, false);
    }
    let mut x = rel_x;
    let mut y = rel_y;
    let mut folded = false;
    if x >= width {
        x -= width;
        folded = true;
    }
    if x <= -width {
        x += width;
        folded = true;
    }
    if y >= height {
        y -= height;
        folded = true;
    }
    if y <= -height {
        y += height;
        folded = true;
    }
    (x, y, folded)
}

/// Apply Haxe world-wrap fold to absolute world coords via birth origin.
///
/// World position changes by ±map size when birth-relative exceeds ±size
/// (same as adjusting Haxe `p.x`/`p.y` while `gx`/`gy` stay fixed).
// Haxe: MoveHelper.moveHelper L550-586
pub fn fold_world_pos_around_world(
    world_x: i32,
    world_y: i32,
    birth_x: i32,
    birth_y: i32,
    width: i32,
    height: i32,
) -> (i32, i32, bool) {
    let rel_x = world_x - birth_x;
    let rel_y = world_y - birth_y;
    let (nx, ny, folded) = fold_relative_around_world(rel_x, rel_y, width, height);
    if !folded {
        return (world_x, world_y, false);
    }
    (birth_x + nx, birth_y + ny, true)
}

/// Haxe `ServerSettings.MinMovementAgeInSec` — reject MOVE when `age * 60 <` this.
// Haxe: ServerSettings.MinMovementAgeInSec / MoveHelper.moveHelper
pub const MIN_MOVEMENT_AGE_IN_SEC: f32 = 14.0;

/// Haxe wait-for-force timeout (seconds) before human MOVE accepted again.
// Haxe: MoveHelper.moveHelper waitForForce ~2s
pub const WAIT_FOR_FORCE_SECS: f32 = 2.0;

/// Haxe `ServerSettings.MaxJumpsPerTenSec`.
// Haxe: ServerSettings.MaxJumpsPerTenSec
pub const MAX_JUMPS_PER_TEN_SEC: f32 = 10.0;

/// Haxe `ServerSettings.ExhaustionOnJump` (multiplied by effective quadDist).
// Haxe: ServerSettings.ExhaustionOnJump
pub const EXHAUSTION_ON_JUMP: f32 = 0.05;

/// Springy Wooden Door closed → open (horizontal).
// Haxe: MoveHelper.OpenDoors parentId 2757 → 2758
pub const SPRINGY_DOOR_CLOSED_H: i32 = 2757;
pub const SPRINGY_DOOR_OPEN_H: i32 = 2758;
/// Springy Wooden Door closed → open (vertical).
// Haxe: MoveHelper.OpenDoors parentId 2759 → 2760
pub const SPRINGY_DOOR_CLOSED_V: i32 = 2759;
pub const SPRINGY_DOOR_OPEN_V: i32 = 2760;

/// Soften jump quadDist when client start tile has a floor (`floorId > 0` → /10).
// Haxe: MoveHelper.moveHelper L595-596
#[inline]
pub fn jump_quad_with_floor(quad_dist: f64, floor_id: i32) -> f64 {
    if floor_id > 0 {
        quad_dist / 10.0
    } else {
        quad_dist
    }
}

/// Haxe `age * 60 >= MinMovementAgeInSec`.
// Haxe: MoveHelper.moveHelper L540
#[inline]
pub fn movement_age_allowed(age_years: f32) -> bool {
    movement_age_allowed_ex(age_years, MIN_MOVEMENT_AGE_IN_SEC)
}

/// Live-knob variant of [`movement_age_allowed`].
// Haxe: ServerSettings.MinMovementAgeInSec
// C-SS-MORE-BATCH3
#[inline]
pub fn movement_age_allowed_ex(age_years: f32, min_movement_age_in_sec: f32) -> bool {
    let min_sec = if min_movement_age_in_sec.is_finite() && min_movement_age_in_sec >= 0.0 {
        min_movement_age_in_sec
    } else {
        MIN_MOVEMENT_AGE_IN_SEC
    };
    age_years * 60.0 >= min_sec
}

/// True while human MOVE must be ignored (`waitForForce` and elapsed < timeout).
// Haxe: MoveHelper.moveHelper L526-537
#[inline]
pub fn still_waiting_for_force(
    wait_for_force: bool,
    time_last_force: f32,
    sim_time: f32,
) -> bool {
    wait_for_force && (sim_time - time_last_force) < WAIT_FOR_FORCE_SECS
}

/// Haxe jump rate gate: `ceil(jumpedTiles) >= MaxJumpsPerTenSec`.
// Haxe: MoveHelper.moveHelper L608
#[inline]
pub fn jump_rate_limited(jumped_tiles: f32) -> bool {
    jumped_tiles.ceil() >= MAX_JUMPS_PER_TEN_SEC
}

/// Apply Haxe jump exhaustion + jumpedTiles accrual after accepting a client start snap.
/// Returns `(new_exhaustion, new_jumped_tiles, is_exhausted)`.
// Haxe: MoveHelper.moveHelper L622-626
pub fn apply_jump_cost(
    exhaustion: f32,
    jumped_tiles: f32,
    food_store_max: f32,
    effective_quad: f64,
    is_human: bool,
) -> (f32, f32, bool) {
    apply_jump_cost_ex(
        exhaustion,
        jumped_tiles,
        food_store_max,
        effective_quad,
        is_human,
        EXHAUSTION_ON_JUMP,
    )
}

/// Live-knob variant of [`apply_jump_cost`] (Haxe `ExhaustionOnJump`).
// Haxe: ServerSettings.ExhaustionOnJump
// C-SS-MORE-BATCH5
pub fn apply_jump_cost_ex(
    exhaustion: f32,
    jumped_tiles: f32,
    food_store_max: f32,
    effective_quad: f64,
    is_human: bool,
    exhaustion_on_jump: f32,
) -> (f32, f32, bool) {
    let cost = if exhaustion_on_jump.is_finite() && exhaustion_on_jump >= 0.0 {
        exhaustion_on_jump
    } else {
        EXHAUSTION_ON_JUMP
    };
    let q = effective_quad.max(0.0) as f32;
    let mut exh = exhaustion;
    if is_human {
        exh += q * cost;
    }
    let is_exhausted = exh > food_store_max / 2.0;
    let add = if is_exhausted { q } else { q / 2.0 };
    (exh, jumped_tiles + add, is_exhausted)
}

/// Haxe TimeHelper jumpedTiles decay: `-= dt * MaxJumpsPerTenSec * 0.1`.
// Haxe: TimeHelper L360
#[inline]
pub fn decay_jumped_tiles(jumped_tiles: f32, dt: f32) -> f32 {
    if jumped_tiles <= 0.0 || dt <= 0.0 {
        return jumped_tiles.max(0.0);
    }
    (jumped_tiles - dt * MAX_JUMPS_PER_TEN_SEC * 0.1).max(0.0)
}

/// Haxe `OpenDoors`: closed springy door parent/id → open id.
// Haxe: MoveHelper.OpenDoors
#[inline]
pub fn springy_door_open_id(obj_or_parent_id: i32) -> Option<i32> {
    match obj_or_parent_id {
        SPRINGY_DOOR_CLOSED_H => Some(SPRINGY_DOOR_OPEN_H),
        SPRINGY_DOOR_CLOSED_V => Some(SPRINGY_DOOR_OPEN_V),
        _ => None,
    }
}

/// One step length in tiles.
#[inline]
pub fn step_len(dx: i32, dy: i32) -> f32 {
    let fx = dx as f32;
    let fy = dy as f32;
    (fx * fx + fy * fy).sqrt()
}

/// Haxe `ServerSettings.LetTheClientCheatLittleBitFactor` (default 1.1).
/// Applied in Haxe *after* `movedLength` is computed — effectively dead for recon.
// Haxe: ServerSettings.LetTheClientCheatLittleBitFactor
pub const LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR: f32 = 1.1;

/// Haxe `MoveHelper.calculateLength` between consecutive path positions.
///
/// Port-as-is: **any** non-diagonal pair is length `1.0` (even multi-tile steps);
/// both axes differ → `√2`. Identical points still return `1.0` (Haxe else branch).
// Haxe: MoveHelper.calculateLength L766-775
#[inline]
pub fn calculate_segment_length_haxe(last: (i32, i32), pos: (i32, i32)) -> f32 {
    if last.0 != pos.0 && last.1 != pos.1 {
        std::f32::consts::SQRT_2
    } else {
        1.0
    }
}

/// Haxe `MoveHelper.calculateNewPos` — mid-path tile recon from elapsed time.
///
/// `waypoints` are **start-relative cumulative** positions (client MOVE form).
/// Returns the last waypoint committed under the half-step rule
/// (`length - thisStep/2 > movedLength` → stay on previous).
///
/// Note: Haxe multiplies elapsed by [`LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR`]
/// **after** computing `movedLength`, so the factor does **not** affect the result
/// (port-as-is dead multiply kept in a discarded local).
// Haxe: MoveHelper.calculateNewPos L853-877
pub fn calculate_new_pos(waypoints: &[(i32, i32)], elapsed_sec: f32, speed: f32) -> (i32, i32) {
    let speed = if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        DEFAULT_MOVE_SPEED
    };
    let mut time_since = elapsed_sec.max(0.0);
    let moved_length = time_since * speed;
    // Haxe L860: time *= cheat — after movedLength; unused for recon (port-as-is).
    time_since *= LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR;
    let _ = time_since;

    let mut last = (0i32, 0i32);
    let mut length = 0.0f32;
    for &wp in waypoints {
        let this_step = calculate_segment_length_haxe(last, wp);
        length += this_step;
        // Haxe: if (length - thisStepLength / 2 > movedLength) return lastPos;
        if length - this_step / 2.0 > moved_length {
            return last;
        }
        last = wp;
    }
    // Whole movement finished → last waypoint (or 0,0 if empty).
    last
}

/// Absolute tile from path start + [`calculate_new_pos`] recon.
///
/// `wrap(x,y) -> (x,y)` should apply world wrap when enabled.
// Haxe: MoveHelper.calculateNewPos + path start (tx,ty)
pub fn reconcile_mid_path_tile(
    path: &MovePath,
    sim_time: f32,
    wrap: &dyn Fn(i32, i32) -> (i32, i32),
) -> (i32, i32) {
    let elapsed = (sim_time - path.start_sim_time).max(0.0);
    let wps = if path.original_waypoints.is_empty() {
        // Fallback: remaining steps as start-relative from path start.
        let steps: Vec<(i32, i32)> = path.remaining.iter().copied().collect();
        steps_to_client_path_deltas(&steps)
    } else {
        path.original_waypoints.clone()
    };
    let (rdx, rdy) = calculate_new_pos(&wps, elapsed, path.speed);
    wrap(path.start_x + rdx, path.start_y + rdy)
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
    use ol_content::ContentDb;
    use ol_world::World;

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
        // C-SS-MORE-KNOBS live threshold
        assert!(!jump_exceeds_force_threshold(5.0, 5.0));
        assert!(jump_exceeds_force_threshold(5.01, 5.0));
        assert!(!jump_exceeds_force_threshold(9.0, 9.0));
        assert!(jump_exceeds_force_threshold(9.0, 5.0));
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

    // Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough + isClose
    #[test]
    fn action_range_use_distance_clamp_and_bow_range() {
        assert_eq!(effective_use_distance(0), 1);
        assert_eq!(effective_use_distance(-3), 1);
        assert_eq!(effective_use_distance(5), 5);
        // held use_distance=5: (3,4) → 25 <= 25
        assert!(in_use_range(0, 0, 3, 4, 5));
        assert!(!in_use_range(0, 0, 4, 4, 5)); // 32 > 25
        // clamp: use_distance 0 treated as 1
        assert!(in_use_range(0, 0, 1, 0, 0));
        assert!(!in_use_range(0, 0, 1, 1, 0));
    }

    // Haxe: AiHelper.CalculateDistance torus wrap in isClose
    #[test]
    fn action_range_torus_wrap() {
        // map 100: tiles 0 and 99 are adjacent when wrap
        assert!(in_use_range_ex(0, 0, 99, 0, 1, 100, 100, true));
        assert!(!in_use_range_ex(0, 0, 99, 0, 1, 100, 100, false));
        assert!(check_if_not_moving_and_close_enough(
            false, 0, 0, 99, 0, 1, 100, 100, true
        ));
        assert!(!check_if_not_moving_and_close_enough(
            true, 0, 0, 0, 0, 1, 100, 100, true
        ));
    }

    #[test]
    fn check_if_not_moving_and_close_enough_gates() {
        assert!(check_if_not_moving_and_close_enough(
            false, 5, 5, 5, 6, 1, 32, 32, false
        ));
        assert!(!check_if_not_moving_and_close_enough(
            false, 5, 5, 7, 7, 1, 32, 32, false
        ));
        assert!(!check_if_not_moving_and_close_enough(
            true, 5, 5, 5, 6, 1, 32, 32, false
        ));
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
    fn truncate_rejects_ocean_without_floor() {
        let db = ContentDb::default();
        let mut world = World::new(16, 16, false);
        world.set_biome(0, 0, ol_world::GREEN);
        world.set_biome(1, 0, ol_world::OCEAN);
        let (acc, trunc) = truncate_walkable(&world, &db, 0, 0, &[(1, 0)]);
        assert!(acc.is_empty());
        assert_eq!(trunc, 1);
        // Any floor except pine allows ocean walk (isBiomeBlocking exception)
        world.set_floor(1, 0, 884);
        let (acc2, trunc2) = truncate_walkable(&world, &db, 0, 0, &[(1, 0)]);
        assert_eq!(acc2, vec![(1, 0)]);
        assert_eq!(trunc2, 0);
    }

    #[test]
    fn truncate_caps_at_10_steps() {
        let db = ContentDb::default();
        let mut world = World::new(64, 64, false);
        for x in 0..20 {
            world.set_biome(x, 0, ol_world::GREEN);
        }
        let deltas: Vec<(i32, i32)> = (1..=11).map(|i| (i, 0)).collect();
        let (acc, trunc) = truncate_walkable(&world, &db, 0, 0, &deltas);
        assert_eq!(acc.len(), MAX_CLIENT_PATH_STEPS);
        assert_eq!(trunc, 1);
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
            use_chance: 0.0,
            speed_mult: 1.0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            r_value: 0.0,
            clothing: "n".into(),
            counts_or_grows_as: 0,
            crafting_steps: 0,
            use_distance: 1,
            deadly_distance: 0.0,
            moves: 0,
            damage: 0.0,
            damage_protection_factor: 1.0,
            wound_factor: 0.5,
            male: false,
            contain_size: 0.0,
            slot_size: 1.0,
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

    // --- TIMED-MOVEMENT-DEFAULT gates ---

    #[test]
    fn movement_age_gate_min_14_sec() {
        // age 0.2 y → 12 s < 14 → blocked; age 0.25 → 15 ≥ 14 → ok
        assert!(!movement_age_allowed(0.2));
        assert!(movement_age_allowed(0.25));
        assert!(movement_age_allowed(14.0));
        assert!(!movement_age_allowed(0.0));
    }

    /// C-SS-MORE-BATCH3: live MinMovementAgeInSec allows/rejects age*60.
    // Haxe: ServerSettings.MinMovementAgeInSec
    #[test]
    fn movement_age_live_min_sec_override() {
        // age 0.3 y → 18 s; min 20 → reject; min 14 → allow
        assert!(!movement_age_allowed_ex(0.3, 20.0));
        assert!(movement_age_allowed_ex(0.3, 14.0));
        assert!(movement_age_allowed_ex(0.3, 18.0));
        assert!(!movement_age_allowed_ex(0.3, 18.01));
    }

    #[test]
    fn wait_for_force_timeout_2s() {
        assert!(still_waiting_for_force(true, 10.0, 11.5));
        assert!(!still_waiting_for_force(true, 10.0, 12.0));
        assert!(!still_waiting_for_force(false, 10.0, 10.5));
    }

    #[test]
    fn floor_softens_jump_quad_by_10() {
        // raw (3,0)=9 > 5 reject; with floor /10 → 0.9 accept
        assert!(jump_quad_with_floor(9.0, 0) > MAX_MOVE_QUAD_JUMP_BEFORE_FORCE as f64);
        assert!(jump_quad_with_floor(9.0, 884) <= MAX_MOVE_QUAD_JUMP_BEFORE_FORCE as f64);
        assert!((jump_quad_with_floor(5.0, 1596) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn jump_rate_and_cost() {
        assert!(jump_rate_limited(9.1)); // ceil 10 ≥ 10
        assert!(jump_rate_limited(10.0));
        assert!(!jump_rate_limited(9.0)); // ceil 9 < 10
        let (exh, jt, exhausted) = apply_jump_cost(0.0, 0.0, 20.0, 4.0, true);
        assert!((exh - 4.0 * EXHAUSTION_ON_JUMP).abs() < 1e-5);
        assert!(!exhausted);
        assert!((jt - 2.0).abs() < 1e-5); // half when not exhausted
        let (_exh2, jt2, exh_flag) = apply_jump_cost(11.0, 0.0, 20.0, 2.0, true);
        assert!(exh_flag); // 11 + cost > food_max/2
        assert!((jt2 - 2.0).abs() < 1e-5); // full quad when exhausted
        // C-SS-MORE-BATCH5 live ExhaustionOnJump=0.1
        let (exh_live, _, _) = apply_jump_cost_ex(0.0, 0.0, 20.0, 4.0, true, 0.1);
        assert!((exh_live - 0.4).abs() < 1e-5);
    }

    #[test]
    fn jumped_tiles_decay() {
        let d = decay_jumped_tiles(10.0, 1.0);
        // 10 - 1 * 10 * 0.1 = 9
        assert!((d - 9.0).abs() < 1e-5);
        assert_eq!(decay_jumped_tiles(0.0, 1.0), 0.0);
    }

    #[test]
    fn springy_doors_map() {
        assert_eq!(springy_door_open_id(2757), Some(2758));
        assert_eq!(springy_door_open_id(2759), Some(2760));
        assert_eq!(springy_door_open_id(2758), None);
        assert_eq!(springy_door_open_id(0), None);
    }

    #[test]
    fn move_reject_silent_and_soft() {
        assert!(MoveReject::WaitForForce.is_silent());
        assert!(!MoveReject::JumpTooFar.is_silent());
        assert!(MoveReject::TooYoung.is_soft_reject());
        assert!(!MoveReject::EmptyPath.is_soft_reject());
    }

    // --- MOVE-MIDPATH / calculateNewPos ---

    // Haxe: MoveHelper.calculateNewPos L853-877
    #[test]
    fn calculate_new_pos_zero_elapsed_stays_origin() {
        // No time → never past half of first step → (0,0)
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 0.0, 1.0), (0, 0));
    }

    #[test]
    fn calculate_new_pos_half_step_rule_cardinal() {
        // speed=1: first segment length 1, midpoint 0.5
        // elapsed 0.4 → moved 0.4 < 0.5 → stay (0,0)
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 0.4, 1.0), (0, 0));
        // elapsed 0.5 → moved 0.5; length-half=0.5; 0.5 > 0.5 is false → commit (1,0)
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 0.5, 1.0), (1, 0));
        // elapsed 1.4 → past midpoint of second step (length 2, mid at 1.5) → (1,0)
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 1.4, 1.0), (1, 0));
        // elapsed 1.5 → commit second → (2,0)
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 1.5, 1.0), (2, 0));
        // elapsed past end → last waypoint
        assert_eq!(calculate_new_pos(&[(1, 0), (2, 0)], 10.0, 1.0), (2, 0));
    }

    #[test]
    fn calculate_new_pos_diagonal_segment() {
        // diagonal length √2 ≈ 1.414; midpoint ≈ 0.707
        let mid = std::f32::consts::SQRT_2 / 2.0;
        assert_eq!(calculate_new_pos(&[(1, 1)], mid - 0.01, 1.0), (0, 0));
        assert_eq!(calculate_new_pos(&[(1, 1)], mid, 1.0), (1, 1));
    }

    #[test]
    fn calculate_new_pos_empty_waypoints() {
        assert_eq!(calculate_new_pos(&[], 5.0, 3.75), (0, 0));
    }

    #[test]
    fn calculate_new_pos_cheat_factor_is_dead() {
        // Haxe multiplies time *after* movedLength — result identical with/without 1.1.
        // At elapsed 0.5, speed 1: commits first of [(1,0)] either way.
        assert_eq!(calculate_new_pos(&[(1, 0)], 0.5, 1.0), (1, 0));
        // If cheat applied to movedLength, elapsed 0.45 * 1.1 = 0.495 < 0.5 → would stay;
        // port-as-is stays dead: 0.45 * 1.0 = 0.45 < 0.5 → (0,0)
        assert_eq!(calculate_new_pos(&[(1, 0)], 0.45, 1.0), (0, 0));
    }

    #[test]
    fn reconcile_mid_path_tile_from_original_waypoints() {
        let mut path = build_move_path(10, 20, vec![(1, 0), (1, 0)], 1.0, 1, 0, 0);
        path.start_sim_time = 100.0;
        // elapsed 1.5 → recon (2,0) relative → absolute (12, 20)
        let (x, y) = reconcile_mid_path_tile(&path, 101.5, &|a, b| (a, b));
        assert_eq!((x, y), (12, 20));
        // elapsed 0.4 → still at start
        let (x0, y0) = reconcile_mid_path_tile(&path, 100.4, &|a, b| (a, b));
        assert_eq!((x0, y0), (10, 20));
    }

    #[test]
    fn build_move_path_stores_original_waypoints() {
        let path = build_move_path(0, 0, vec![(1, 0), (0, 1)], 3.75, 1, 0, 0);
        assert_eq!(path.original_waypoints, vec![(1, 0), (1, 1)]);
        assert_eq!(path.start_sim_time, 0.0);
    }

    // --- MOVE-VOG-WRAP / cancel_wrap ---

    // Haxe: MoveHelper.moveHelper L550-559
    #[test]
    fn fold_relative_around_world_one_fold_per_axis() {
        // No fold inside (-w, w)
        assert_eq!(fold_relative_around_world(0, 0, 100, 80), (0, 0, false));
        assert_eq!(fold_relative_around_world(99, -79, 100, 80), (99, -79, false));
        // >= width / <= -width
        assert_eq!(fold_relative_around_world(100, 0, 100, 80), (0, 0, true));
        assert_eq!(fold_relative_around_world(150, 0, 100, 80), (50, 0, true));
        assert_eq!(fold_relative_around_world(-100, 0, 100, 80), (0, 0, true));
        assert_eq!(fold_relative_around_world(-150, 5, 100, 80), (-50, 5, true));
        // y axis
        assert_eq!(fold_relative_around_world(0, 80, 100, 80), (0, 0, true));
        assert_eq!(fold_relative_around_world(0, -80, 100, 80), (0, 0, true));
        // both axes
        assert_eq!(fold_relative_around_world(100, -80, 100, 80), (0, 0, true));
        // invalid size → no-op
        assert_eq!(fold_relative_around_world(200, 0, 0, 80), (200, 0, false));
    }

    // Haxe: MoveHelper.moveHelper L550-586 via birth origin
    #[test]
    fn fold_world_pos_via_birth_origin() {
        // birth (10,20), world (110,20) → rel x=100 on map 100 → fold world to (10,20)
        let (wx, wy, f) = fold_world_pos_around_world(110, 20, 10, 20, 100, 80);
        assert!(f);
        assert_eq!((wx, wy), (10, 20));
        // no fold
        let (wx2, wy2, f2) = fold_world_pos_around_world(50, 30, 10, 20, 100, 80);
        assert!(!f2);
        assert_eq!((wx2, wy2), (50, 30));
        // negative wrap
        let (wx3, wy3, f3) = fold_world_pos_around_world(-90, 20, 10, 20, 100, 80);
        assert!(f3);
        assert_eq!((wx3, wy3), (10, 20)); // rel -100 → 0
    }

    #[test]
    fn cancel_vog_quad_threshold() {
        assert!(!cancel_should_use_vog(25.0));
        assert!(cancel_should_use_vog(25.1));
        assert!(!cancel_should_use_vog(5.0));
        assert!(MoveReject::EmptyPath.cancel_with_vog());
        assert!(MoveReject::WorldWrapped.cancel_with_vog());
        assert!(!MoveReject::JumpTooFar.cancel_with_vog());
        assert!(MoveReject::WorldWrapped.is_silent());
    }
}
