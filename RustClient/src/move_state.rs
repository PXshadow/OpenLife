//! MOVE wire encoding and client-side move sequence / in-motion / FORCE state.
//!
//! Matches official client (`LivingLifePage.cpp`):
//! - `lastMoveSequenceNumber` starts at **1** on birth; first MOVE increments to **2**.
//! - Wire: `MOVE xs ys @seq_num xdelt0 ydelt0 ... xdeltN ydeltN#`
//!   exactly one space between tokens (protocol.txt).
//! - Path deltas are **cumulative offsets from `(xs, ys)`** (not adjacent steps) and
//!   must be within **±16** (C++ `pathToDest[i] - pathToDest[0]`).
//! - Client sets `in_motion = true` when sending MOVE.
//! - Click-to-move may **repath** while mid-move (`send_move_repath`); caller sets
//!   `(x,y)` to the path origin first (C++ `closestPathPos`).
//! - Own-player `in_motion` clears when a PU reports `done_moving_seqNum == lastMoveSequenceNumber`.
//! - On `force=1` PU (or artificial force after truncated dest mismatch): snap position and
//!   return `FORCE x y#` for session to send immediately.
//! - Session **cancels** queued `nextActionMessageToSend` on FORCE (does not flush it);
//!   flushes the queue only on non-force done_moving when free.
//! - Own truncated PM cancels pending action and sets `dest_truncated` (artificial FORCE later).
//! - Multi-MOVE: remaining ±16 chunks + ultimate goal after first hop (`arm_multi_move`).
//! - USE / DROP / REMV are not sent while a MOVE is in progress (client gate; server also ignores).

use crate::actions::encode_force;
use crate::map_global_offset::{encode_move_with_offset, MapGlobalOffset};
use std::collections::VecDeque;
use thiserror::Error;

/// Maximum absolute path-search radius / delta magnitude (protocol.txt).
pub const MAX_PATH_DELTA: i32 = 16;

/// C++ `BASE_SPEED` — default grid tiles/sec for local path interp before PM ETA.
///
/// Used when client sends MOVE before the server's `total_sec` arrives.
pub const BASE_PATH_SPEED: f64 = 3.75;

/// Diagonal step length (C++ `measurePathLength` uses √2).
const PATH_DIAG_LEN: f64 = 1.414_213_562_37;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoveError {
    #[error("path is empty; need at least one destination delta")]
    EmptyPath,
    #[error("path delta ({0},{1}) exceeds ±{MAX_PATH_DELTA}")]
    DeltaOutOfRange(i32, i32),
    #[error("cannot send MOVE while already in motion (wait for done_moving PU)")]
    AlreadyInMotion,
    #[error("cannot send position-sensitive action until FORCE ack is sent")]
    AwaitingForceAck,
    #[error("cannot send USE/DROP/REMV while MOVE is in progress")]
    ActionWhileMoving,
    #[error("click tile is the current standing tile; no MOVE")]
    SameTile,
    /// No walkable 4-neighbor (or self-tile) stand for object path-to-adjacent.
    #[error("no reachable adjacent stand tile for object action")]
    NoAdjacentStand,
    /// C++ `playerActionPending` — wait for post-action PU before next click.
    #[error("player action pending; wait for PU confirmation")]
    ActionPending,
    /// Holding an object with `speedMult == 0` and no ground use transition.
    #[error("holding 0-speed object with no ground transition; click ignored")]
    HoldingImmobile,
    /// Click converted to `JUMP 0 0#` (held baby or age < noMoveAge); wire already sent.
    #[error("JUMP sent (held by adult or age < noMoveAge); no MOVE/action")]
    JumpSent,
}

/// Path point as **cumulative** offset from MOVE start `(xs, ys)`.
///
/// C++: `pathToDest[i].x - pathToDest[0].x` (not adjacent ±1 steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathDelta {
    pub x: i32,
    pub y: i32,
}

/// Client move bookkeeping for one local player.
#[derive(Debug, Clone)]
pub struct MoveState {
    /// Last move sequence number sent (or 1 at birth before any MOVE).
    pub last_move_sequence_number: i32,
    /// True after we send MOVE until matching done_moving PU.
    pub in_motion: bool,
    /// Believed destination / next MOVE origin (`LiveObject.xd`/`yd` parity after path end).
    pub x: i32,
    pub y: i32,
    /// Server forced a position; block moves/actions until FORCE ack is sent.
    pub awaiting_force_ack: bool,
    pub force_x: i32,
    pub force_y: i32,
    /// C++: `LiveObject.destTruncated` — last own path was truncated by the server.
    /// A later PU with a mismatched pos is treated as forced (~18031–18048).
    pub dest_truncated: bool,
    /// C++ `LiveObject.pathToDest` — absolute grid cells of the last MOVE path
    /// (including start). Used by `findClosestPathSpot` for mid-move repath origin.
    pub path_to_dest: Vec<(i32, i32)>,
    /// Remaining MOVE path chunks (rebased ±16 cumulative deltas) after the current hop.
    pub pending_move_chunks: VecDeque<Vec<PathDelta>>,
    /// Ultimate ground/stand goal for multi-MOVE repath when chunks are exhausted
    /// but the first window did not reach the click target.
    pub pending_move_goal: Option<(i32, i32)>,
    /// C++ `LiveObject.currentPos` — fractional display / path-interp position.
    ///
    /// Advanced by [`Self::step_current_pos`] while mid-move. [`Self::closest_path_spot`]
    /// uses `lrint` of this as the primary mid-move origin (not only the last PU grid).
    pub current_pos_x: f64,
    pub current_pos_y: f64,
    /// Distance along `path_to_dest` already traveled (grid units; diag = √2).
    pub path_dist_traveled: f64,
    /// Total length of `path_to_dest` (0 when idle).
    pub path_total_length: f64,
    /// Grid tiles/sec along the path (C++ `currentGridSpeed` lite).
    pub path_speed: f64,
    /// C++ `LiveObject::useWaypoint` — path must pass through [`Self::waypoint`].
    pub use_waypoint: bool,
    /// C++ `waypointX` / `waypointY` (world tiles).
    pub waypoint: (i32, i32),
    /// C++ `maxWaypointPathLength` — if two-leg path is longer, dest becomes waypoint.
    pub max_waypoint_path_length: i32,
    /// C++ `isAutoClick` for one path plan (close-hold throw / auto-walk).
    pub path_auto_click: bool,
}

impl Default for MoveState {
    fn default() -> Self {
        Self {
            // LivingLifePage.cpp sets lastMoveSequenceNumber = 1 on player create.
            last_move_sequence_number: 1,
            in_motion: false,
            x: 0,
            y: 0,
            awaiting_force_ack: false,
            force_x: 0,
            force_y: 0,
            dest_truncated: false,
            path_to_dest: Vec::new(),
            pending_move_chunks: VecDeque::new(),
            pending_move_goal: None,
            current_pos_x: 0.0,
            current_pos_y: 0.0,
            path_dist_traveled: 0.0,
            path_total_length: 0.0,
            path_speed: 0.0,
            use_waypoint: false,
            waypoint: (0, 0),
            // C++ default when arming close-hold throw (pathfind::DEFAULT_MAX_WAYPOINT_PATH_LENGTH).
            max_waypoint_path_length: 10,
            path_auto_click: false,
        }
    }
}

impl MoveState {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            current_pos_x: x as f64,
            current_pos_y: y as f64,
            ..Self::default()
        }
    }

    /// C++ `useWaypoint = true` + waypoint tile + `maxWaypointPathLength`.
    pub fn arm_waypoint(&mut self, wx: i32, wy: i32, max_path_length: i32) {
        self.use_waypoint = true;
        self.waypoint = (wx, wy);
        self.max_waypoint_path_length = max_path_length.max(1);
    }

    /// Clear one-shot waypoint / auto-click path flags after a plan.
    pub fn clear_waypoint(&mut self) {
        self.use_waypoint = false;
        self.path_auto_click = false;
    }

    /// C++ `lrint` stand-in for path-spot rounding of fractional `currentPos`.
    #[inline]
    pub fn lrint_pos(v: f64) -> i32 {
        v.round() as i32
    }

    /// Measure path length with diagonal = √2 (C++ `measurePathLength`).
    pub fn measure_path_length(path: &[(i32, i32)]) -> f64 {
        if path.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        for w in path.windows(2) {
            let (ax, ay) = w[0];
            let (bx, by) = w[1];
            if ax != bx && ay != by {
                total += PATH_DIAG_LEN;
            } else {
                total += 1.0;
            }
        }
        total
    }

    /// Interpolate a fractional position along an absolute path at distance `dist`.
    pub fn position_along_path(path: &[(i32, i32)], dist: f64) -> (f64, f64) {
        if path.is_empty() {
            return (0.0, 0.0);
        }
        if path.len() == 1 || dist <= 0.0 {
            return (path[0].0 as f64, path[0].1 as f64);
        }
        let mut remaining = dist;
        for w in path.windows(2) {
            let (ax, ay) = w[0];
            let (bx, by) = w[1];
            let seg = if ax != bx && ay != by {
                PATH_DIAG_LEN
            } else {
                1.0
            };
            if remaining <= seg || (bx, by) == *path.last().unwrap() {
                let t = if seg > 0.0 {
                    (remaining / seg).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                return (
                    ax as f64 + (bx - ax) as f64 * t,
                    ay as f64 + (by - ay) as f64 * t,
                );
            }
            remaining -= seg;
        }
        let last = *path.last().unwrap();
        (last.0 as f64, last.1 as f64)
    }

    /// Snap fractional pos to integer grid and clear path-speed state.
    pub(crate) fn snap_current_to(&mut self, x: i32, y: i32) {
        self.current_pos_x = x as f64;
        self.current_pos_y = y as f64;
        self.path_dist_traveled = 0.0;
        self.path_total_length = 0.0;
        self.path_speed = 0.0;
    }

    /// Begin path interpolation from the first cell of `path_to_dest`.
    ///
    /// `speed_override`: if `Some` and > 0, use that tiles/sec; else [`BASE_PATH_SPEED`].
    fn begin_path_interp(&mut self, speed_override: Option<f64>) {
        if self.path_to_dest.is_empty() {
            self.path_total_length = 0.0;
            self.path_dist_traveled = 0.0;
            self.path_speed = 0.0;
            return;
        }
        let start = self.path_to_dest[0];
        self.current_pos_x = start.0 as f64;
        self.current_pos_y = start.1 as f64;
        self.path_dist_traveled = 0.0;
        self.path_total_length = Self::measure_path_length(&self.path_to_dest);
        let sp = speed_override
            .filter(|s| *s > 0.0)
            .unwrap_or(BASE_PATH_SPEED);
        self.path_speed = sp;
    }

    /// Advance fractional `currentPos` along the active path (C++ per-frame path step lite).
    ///
    /// Linear along path cells (no turn-smoothing / circling fix). Sufficient for
    /// `findClosestPathSpot` mid-move repath origin.
    pub fn step_current_pos(&mut self, wall_dt: f64) {
        if !self.in_motion || self.path_to_dest.len() < 2 || self.path_speed <= 0.0 || wall_dt <= 0.0
        {
            return;
        }
        self.path_dist_traveled =
            (self.path_dist_traveled + self.path_speed * wall_dt).min(self.path_total_length);
        let (px, py) = Self::position_along_path(&self.path_to_dest, self.path_dist_traveled);
        self.current_pos_x = px;
        self.current_pos_y = py;
    }

    /// Integer hint from fractional currentPos (`lrint` of each axis).
    #[inline]
    pub fn current_pos_tile(&self) -> (i32, i32) {
        (
            Self::lrint_pos(self.current_pos_x),
            Self::lrint_pos(self.current_pos_y),
        )
    }

    /// Refine path speed from own-player PM (`total_sec` / path length).
    ///
    /// C++ `updateMoveSpeed` uses eta + remaining length; we set constant grid speed
    /// from full path / `total_sec` when the PM matches our path origin.
    pub fn on_own_pm_timing(&mut self, xs: i32, ys: i32, deltas: &[(i32, i32)], total_sec: f32) {
        if total_sec <= 0.0 || deltas.is_empty() {
            return;
        }
        // Rebuild absolute path from PM if we have no local path, or PM origin matches.
        let mut path = Vec::with_capacity(deltas.len() + 1);
        path.push((xs, ys));
        for &(dx, dy) in deltas {
            path.push((xs + dx, ys + dy));
        }
        let len = Self::measure_path_length(&path);
        if len <= 0.0 {
            return;
        }
        // If we already have a matching path in flight, keep traveled progress by
        // projecting current_pos onto the new/same path distance.
        let keep_progress = self.in_motion && !self.path_to_dest.is_empty();
        let prev_pos = (self.current_pos_x, self.current_pos_y);
        self.path_to_dest = path;
        self.path_total_length = len;
        self.path_speed = len / total_sec as f64;
        if keep_progress {
            // Approximate distance already covered: nearest projection by walking segs.
            let path_max = self.path_total_length.max(0.0);
            self.path_dist_traveled =
                distance_along_path_nearest(&self.path_to_dest, prev_pos.0, prev_pos.1)
                    .clamp(0.0, path_max);
            let (px, py) =
                Self::position_along_path(&self.path_to_dest, self.path_dist_traveled);
            self.current_pos_x = px;
            self.current_pos_y = py;
        } else if self.in_motion {
            self.path_dist_traveled = 0.0;
            self.current_pos_x = xs as f64;
            self.current_pos_y = ys as f64;
        }
    }

    /// Encode and apply a MOVE: increments seq, sets in_motion, validates deltas.
    ///
    /// Uses identity offset (storage == wire). Prefer [`Self::send_move_with_offset`]
    /// when the session has a non-default [`MapGlobalOffset`].
    ///
    /// Fails with [`MoveError::AlreadyInMotion`] if a prior MOVE is unfinished.
    /// Use [`Self::send_move_repath`] for click-to-move repathing (C++ allows interrupt).
    pub fn send_move(&mut self, path_deltas: &[PathDelta]) -> Result<String, MoveError> {
        self.send_move_with_offset(path_deltas, MapGlobalOffset::ZERO)
    }

    /// Like [`Self::send_move`] but applies C++ `sendX`/`sendY` via `offset`.
    pub fn send_move_with_offset(
        &mut self,
        path_deltas: &[PathDelta],
        offset: MapGlobalOffset,
    ) -> Result<String, MoveError> {
        self.send_move_inner(path_deltas, false, offset)
    }

    /// Like [`Self::send_move`] but allows replacing an in-progress path (ground re-click).
    ///
    /// Caller must set [`Self::x`]/[`Self::y`] to the path origin (`closestPathPos`) first.
    pub fn send_move_repath(&mut self, path_deltas: &[PathDelta]) -> Result<String, MoveError> {
        self.send_move_repath_with_offset(path_deltas, MapGlobalOffset::ZERO)
    }

    /// Like [`Self::send_move_repath`] with C++ `sendX`/`sendY` offset on MOVE start.
    pub fn send_move_repath_with_offset(
        &mut self,
        path_deltas: &[PathDelta],
        offset: MapGlobalOffset,
    ) -> Result<String, MoveError> {
        self.send_move_inner(path_deltas, true, offset)
    }

    fn send_move_inner(
        &mut self,
        path_deltas: &[PathDelta],
        allow_repath: bool,
        offset: MapGlobalOffset,
    ) -> Result<String, MoveError> {
        if self.awaiting_force_ack {
            return Err(MoveError::AwaitingForceAck);
        }
        if self.in_motion && !allow_repath {
            return Err(MoveError::AlreadyInMotion);
        }
        let start_x = self.x;
        let start_y = self.y;
        // Wire start = sendX/sendY (identity when offset is ZERO / storage==wire).
        let line = encode_move_with_offset(
            offset,
            start_x,
            start_y,
            self.last_move_sequence_number + 1,
            path_deltas,
        )?;
        self.last_move_sequence_number += 1;
        self.in_motion = true;
        // Fresh client path is not truncated until the server says so.
        self.dest_truncated = false;
        // C++ pathToDest absolute cells in **storage** frame (start + each cumulative end).
        let mut path = Vec::with_capacity(path_deltas.len() + 1);
        path.push((start_x, start_y));
        for d in path_deltas {
            path.push((start_x + d.x, start_y + d.y));
        }
        self.path_to_dest = path;
        // Start fractional currentPos at path origin; speed refined by PM later.
        self.begin_path_interp(None);
        // Destination = start + last cumulative delta (official path end / xd,yd).
        if let Some(last) = path_deltas.last() {
            self.x = start_x + last.x;
            self.y = start_y + last.y;
        }
        Ok(line)
    }

    /// Whether USE/DROP/REMV may be issued (not mid-move, not waiting on FORCE).
    pub fn can_send_object_action(&self) -> Result<(), MoveError> {
        if self.awaiting_force_ack {
            return Err(MoveError::AwaitingForceAck);
        }
        if self.in_motion {
            return Err(MoveError::ActionWhileMoving);
        }
        Ok(())
    }

    /// Absolute path end from PM wire (`xs + last xdelt`, `ys + last ydelt`).
    pub fn path_end(xs: i32, ys: i32, deltas: &[(i32, i32)]) -> (i32, i32) {
        match deltas.last() {
            Some(&(dx, dy)) => (xs + dx, ys + dy),
            None => (xs, ys),
        }
    }

    /// C++: own-player PM with `trunc=1` (~20139–20506).
    ///
    /// Replaces local dest with the truncated path end, marks `dest_truncated`, and
    /// signals the session to **cancel** (not flush) `nextActionMessageToSend`.
    /// Returns `true` when truncated so the session can clear the pending action.
    pub fn on_own_path_truncated(&mut self, xs: i32, ys: i32, deltas: &[(i32, i32)]) -> bool {
        let (xd, yd) = Self::path_end(xs, ys, deltas);
        // C++ only replaces OUR path when truncated.
        self.x = xd;
        self.y = yd;
        self.dest_truncated = true;
        // Keep fractional progress; rebuild path cells for closestPathSpot snap.
        let mut path = Vec::with_capacity(deltas.len() + 1);
        path.push((xs, ys));
        for &(dx, dy) in deltas {
            path.push((xs + dx, ys + dy));
        }
        if !path.is_empty() {
            self.path_to_dest = path;
            self.path_total_length = Self::measure_path_length(&self.path_to_dest).max(0.0);
            let path_max = self.path_total_length;
            self.path_dist_traveled = distance_along_path_nearest(
                &self.path_to_dest,
                self.current_pos_x,
                self.current_pos_y,
            )
            .clamp(0.0, path_max);
        }
        self.clear_multi_move();
        true
    }

    /// Apply a PLAYER_UPDATE for the local player (done_moving_seqNum, force, x, y).
    ///
    /// Returns `Some(FORCE wire)` when force (wire or artificial) — caller must send it
    /// immediately and call [`Self::acknowledge_force_sent`]. Does **not** touch pending
    /// actions (session cancels them on FORCE; flushes on matching done_moving).
    ///
    /// C++: LivingLifePage.cpp PU handler ~18031 artificial force, ~19311 force snap,
    /// ~19359 FORCE ack, ~19389 in_motion clear only on matching seq.
    pub fn on_player_update(
        &mut self,
        done_moving_seq_num: i32,
        force: bool,
        x: i32,
        y: i32,
    ) -> Option<String> {
        // C++ ~18031–18048: destTruncated + PU pos != local xd,yd → treat as force.
        let mut force = force;
        if self.dest_truncated && (self.x != x || self.y != y) {
            force = true;
        }

        // force snap (protocol + client); done_moving is ignored for the snap itself.
        if force {
            self.x = x;
            self.y = y;
            self.in_motion = false;
            self.awaiting_force_ack = true;
            self.force_x = x;
            self.force_y = y;
            self.dest_truncated = false;
            self.path_to_dest.clear();
            self.snap_current_to(x, y);
            self.clear_multi_move();
            return Some(encode_force(x, y));
        }

        if done_moving_seq_num > 0 {
            // Official client only clears in_motion when done_moving matches last sent seq.
            if done_moving_seq_num == self.last_move_sequence_number {
                self.in_motion = false;
                self.x = x;
                self.y = y;
                self.dest_truncated = false;
                self.path_to_dest.clear();
                self.snap_current_to(x, y);
            }
        }
        None
    }

    /// C++ `findClosestPathSpot` (~2341–2387) with fractional `currentPos`.
    ///
    /// - Mid-move: primary hint is `lrint(currentPos)` (not only last PU grid).
    /// - Prefer that tile when it lies on `path_to_dest`.
    /// - Else snap to nearest path cell while mid-move.
    /// - Idle: `hint` (LiveObject PU) or dest `(x,y)`.
    pub fn closest_path_spot(&self, hint: Option<(i32, i32)>) -> (i32, i32) {
        if self.in_motion && !self.path_to_dest.is_empty() {
            // C++ always starts from lrint(currentPos) then snaps onto pathToDest.
            let server = self.current_pos_tile();
            if self.path_to_dest.iter().any(|&p| p == server) {
                return server;
            }
            let mut best = self.path_to_dest[0];
            let mut best_d2 = i32::MAX;
            for &p in &self.path_to_dest {
                let dx = p.0 - server.0;
                let dy = p.1 - server.1;
                let d2 = dx * dx + dy * dy;
                if d2 < best_d2 {
                    best_d2 = d2;
                    best = p;
                }
            }
            return best;
        }
        // Idle / no path: C++ uses xServer/yServer when pathToDest is NULL.
        hint.unwrap_or((self.x, self.y))
    }

    /// After sending FORCE ack, clear the sync gate.
    pub fn acknowledge_force_sent(&mut self) {
        self.awaiting_force_ack = false;
    }

    /// Build FORCE ack for current force coordinates (caller sends then calls acknowledge_force_sent).
    pub fn force_ack_message(&self) -> String {
        encode_force(self.force_x, self.force_y)
    }

    /// Clear multi-MOVE follow-up (FORCE / trunc / new ground click / logout).
    pub fn clear_multi_move(&mut self) {
        self.pending_move_chunks.clear();
        self.pending_move_goal = None;
    }

    /// True when more MOVE hops are armed after the current path ends.
    pub fn has_multi_move(&self) -> bool {
        !self.pending_move_chunks.is_empty() || self.pending_move_goal.is_some()
    }

    /// Arm multi-MOVE after sending the first ±16 chunk.
    ///
    /// `remaining` = later rebased chunks from `chunk_deltas_for_move`.
    /// `ultimate_goal` = click/stand tile.
    /// `path_end` = absolute end of the MOVE just sent.
    /// Returns true when follow-up hops will run on subsequent done_moving.
    pub fn arm_multi_move(
        &mut self,
        remaining: Vec<Vec<PathDelta>>,
        ultimate_goal: (i32, i32),
        path_end: (i32, i32),
    ) -> bool {
        self.pending_move_chunks = remaining.into_iter().collect();
        if path_end != ultimate_goal || !self.pending_move_chunks.is_empty() {
            self.pending_move_goal = Some(ultimate_goal);
        } else {
            self.pending_move_goal = None;
        }
        self.has_multi_move()
    }

    /// Pop next precomputed chunk if any (does not repath).
    pub fn take_next_move_chunk(&mut self) -> Option<Vec<PathDelta>> {
        while let Some(chunk) = self.pending_move_chunks.pop_front() {
            if !chunk.is_empty() {
                return Some(chunk);
            }
        }
        None
    }
}

/// Distance along `path` of the point on the polyline nearest to `(px, py)`.
fn distance_along_path_nearest(path: &[(i32, i32)], px: f64, py: f64) -> f64 {
    if path.len() < 2 {
        return 0.0;
    }
    let mut best_d2 = f64::MAX;
    let mut best_dist = 0.0;
    let mut prefix = 0.0;
    for w in path.windows(2) {
        let (ax, ay) = (w[0].0 as f64, w[0].1 as f64);
        let (bx, by) = (w[1].0 as f64, w[1].1 as f64);
        let abx = bx - ax;
        let aby = by - ay;
        let ab2 = abx * abx + aby * aby;
        let t = if ab2 > 0.0 {
            ((px - ax) * abx + (py - ay) * aby) / ab2
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let qx = ax + abx * t;
        let qy = ay + aby * t;
        let dx = px - qx;
        let dy = py - qy;
        let d2 = dx * dx + dy * dy;
        let seg_len = if w[0].0 != w[1].0 && w[0].1 != w[1].1 {
            PATH_DIAG_LEN
        } else {
            (ab2).sqrt()
        };
        if d2 < best_d2 {
            best_d2 = d2;
            best_dist = prefix + t * seg_len;
        }
        prefix += seg_len;
    }
    best_dist
}

/// Pure MOVE encoder (does not mutate state).
///
/// `seq_num` is the sequence to put on the wire (first move = 2).
/// `path_deltas` are **cumulative** offsets relative to `(xs, ys)`; at least one required.
pub fn encode_move(
    xs: i32,
    ys: i32,
    seq_num: i32,
    path_deltas: &[PathDelta],
) -> Result<String, MoveError> {
    if path_deltas.is_empty() {
        return Err(MoveError::EmptyPath);
    }
    for d in path_deltas {
        if d.x.abs() > MAX_PATH_DELTA || d.y.abs() > MAX_PATH_DELTA {
            return Err(MoveError::DeltaOutOfRange(d.x, d.y));
        }
    }

    // Exact spacing: "MOVE xs ys @seq" then " xdelt ydelt" pairs, then "#"
    let mut s = format!("MOVE {xs} {ys} @{seq_num}");
    for d in path_deltas {
        s.push_str(&format!(" {} {}", d.x, d.y));
    }
    s.push('#');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_wire_single_step() {
        let line = encode_move(10, 20, 2, &[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(line, "MOVE 10 20 @2 1 0#");
    }

    #[test]
    fn send_move_with_offset_applies_send_xy() {
        // P2#12: path bookkeeping stays in storage frame; wire start uses sendX/Y.
        let mut st = MoveState::new(5, 7);
        let o = MapGlobalOffset {
            set: true,
            x: 100,
            y: 200,
        };
        let line = st
            .send_move_with_offset(&[PathDelta { x: 1, y: 0 }], o)
            .unwrap();
        assert_eq!(line, "MOVE 105 207 @2 1 0#");
        // Storage dest unchanged by offset.
        assert_eq!((st.x, st.y), (6, 7));
        assert_eq!(st.path_to_dest, vec![(5, 7), (6, 7)]);
    }

    #[test]
    fn send_move_zero_offset_matches_plain() {
        let mut a = MoveState::new(488, 488);
        let mut b = MoveState::new(488, 488);
        let deltas = [PathDelta { x: 1, y: 0 }, PathDelta { x: 2, y: 0 }];
        let la = a.send_move(&deltas).unwrap();
        let lb = b
            .send_move_with_offset(&deltas, MapGlobalOffset::ZERO)
            .unwrap();
        assert_eq!(la, lb);
        assert_eq!(la, "MOVE 488 488 @2 1 0 2 0#");
    }

    #[test]
    fn move_wire_multi_step_exact_spaces() {
        let line = encode_move(
            -5,
            3,
            2,
            &[
                PathDelta { x: 1, y: 0 },
                PathDelta { x: 2, y: 1 },
                PathDelta { x: 2, y: 2 },
            ],
        )
        .unwrap();
        assert_eq!(line, "MOVE -5 3 @2 1 0 2 1 2 2#");
        // no double spaces
        assert!(!line.contains("  "));
    }

    #[test]
    fn first_move_seq_is_2() {
        let mut st = MoveState::new(0, 0);
        assert_eq!(st.last_move_sequence_number, 1);
        let line = st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(line, "MOVE 0 0 @2 1 0#");
        assert_eq!(st.last_move_sequence_number, 2);
        assert!(st.in_motion);
        assert_eq!((st.x, st.y), (1, 0));
    }

    #[test]
    fn cumulative_multi_step_sets_dest_from_last() {
        // C++: MOVE 0 0 @2 1 0 2 0# → dest (2,0)
        let mut st = MoveState::new(0, 0);
        let line = st
            .send_move(&[PathDelta { x: 1, y: 0 }, PathDelta { x: 2, y: 0 }])
            .unwrap();
        assert_eq!(line, "MOVE 0 0 @2 1 0 2 0#");
        assert_eq!((st.x, st.y), (2, 0));
    }

    #[test]
    fn repath_while_in_motion() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 2, y: 0 }]).unwrap();
        assert!(st.in_motion);
        // Mid-move repath from origin (caller snaps x,y to closestPathPos).
        st.x = 0;
        st.y = 0;
        let line = st
            .send_move_repath(&[PathDelta { x: 0, y: 1 }])
            .unwrap();
        assert_eq!(line, "MOVE 0 0 @3 0 1#");
        assert_eq!((st.x, st.y), (0, 1));
        assert_eq!(st.last_move_sequence_number, 3);
    }

    #[test]
    fn second_move_seq_is_3_after_done() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        st.on_player_update(2, false, 1, 0);
        assert!(!st.in_motion);
        let line = st.send_move(&[PathDelta { x: 0, y: 1 }]).unwrap();
        assert_eq!(line, "MOVE 1 0 @3 0 1#");
    }

    #[test]
    fn rejects_delta_beyond_16() {
        let err = encode_move(0, 0, 2, &[PathDelta { x: 17, y: 0 }]).unwrap_err();
        assert_eq!(err, MoveError::DeltaOutOfRange(17, 0));
    }

    #[test]
    fn blocks_action_while_moving() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(
            st.can_send_object_action(),
            Err(MoveError::ActionWhileMoving)
        );
    }

    #[test]
    fn force_snaps_and_returns_ack() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 5, y: 0 }]).unwrap();
        let ack = st.on_player_update(0, true, 2, 3).unwrap();
        assert_eq!(ack, "FORCE 2 3#");
        assert_eq!((st.x, st.y), (2, 3));
        assert!(st.awaiting_force_ack);
        assert!(!st.in_motion);
        st.acknowledge_force_sent();
        assert!(!st.awaiting_force_ack);
    }

    #[test]
    fn force_with_mismatched_done_moving_still_snaps() {
        // C++ force snap ignores done_moving match; always acks FORCE at PU coords.
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 5, y: 0 }]).unwrap();
        assert_eq!(st.last_move_sequence_number, 2);
        let ack = st.on_player_update(99, true, 4, 5).unwrap();
        assert_eq!(ack, "FORCE 4 5#");
        assert_eq!((st.x, st.y), (4, 5));
        assert!(!st.in_motion);
        assert!(st.awaiting_force_ack);
        assert!(!st.dest_truncated);
    }

    #[test]
    fn post_force_next_move_uses_forced_origin() {
        let mut st = MoveState::new(10, 10);
        st.send_move(&[PathDelta { x: 3, y: 0 }]).unwrap();
        let _ = st.on_player_update(0, true, 20, 21).unwrap();
        st.acknowledge_force_sent();
        let line = st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(line, "MOVE 20 21 @3 1 0#");
        assert_eq!((st.x, st.y), (21, 21));
    }

    #[test]
    fn done_moving_mismatch_keeps_in_motion() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        // Stale / other seq
        st.on_player_update(1, false, 0, 0);
        assert!(st.in_motion);
        st.on_player_update(2, false, 1, 0);
        assert!(!st.in_motion);
    }

    #[test]
    fn own_path_truncate_sets_dest_and_flag() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 5, y: 0 }]).unwrap();
        assert_eq!((st.x, st.y), (5, 0));
        assert!(st.on_own_path_truncated(0, 0, &[(1, 0), (2, 0)]));
        assert!(st.dest_truncated);
        assert_eq!((st.x, st.y), (2, 0));
    }

    #[test]
    fn artificial_force_when_dest_truncated_pos_mismatch() {
        // C++ ~18031–18048
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 5, y: 0 }]).unwrap();
        st.on_own_path_truncated(0, 0, &[(2, 0)]);
        assert!(st.dest_truncated);
        assert_eq!((st.x, st.y), (2, 0));
        // PU lands at a different pos → artificial force
        let ack = st.on_player_update(2, false, 1, 0).unwrap();
        assert_eq!(ack, "FORCE 1 0#");
        assert!(!st.dest_truncated);
        assert_eq!((st.x, st.y), (1, 0));
        assert!(st.awaiting_force_ack);
    }

    #[test]
    fn dest_truncated_matching_pos_is_not_artificial_force() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 2, y: 0 }]).unwrap();
        st.on_own_path_truncated(0, 0, &[(2, 0)]);
        // Matching dest: no artificial force; matching done_moving ends motion.
        assert!(st.on_player_update(2, false, 2, 0).is_none());
        assert!(!st.in_motion);
        assert!(!st.dest_truncated);
    }

    #[test]
    fn fractional_current_pos_advances_along_path() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[
            PathDelta { x: 1, y: 0 },
            PathDelta { x: 2, y: 0 },
            PathDelta { x: 3, y: 0 },
        ])
        .unwrap();
        assert_eq!((st.current_pos_x, st.current_pos_y), (0.0, 0.0));
        assert!((st.path_total_length - 3.0).abs() < 1e-9);
        assert!((st.path_speed - BASE_PATH_SPEED).abs() < 1e-9);
        // Walk half a second at 3.75 tiles/s → ~1.875 tiles along path.
        st.step_current_pos(0.5);
        assert!((st.current_pos_x - 1.875).abs() < 1e-6);
        assert!((st.current_pos_y - 0.0).abs() < 1e-9);
        assert_eq!(st.current_pos_tile(), (2, 0)); // lrint(1.875) = 2
    }

    #[test]
    fn closest_path_spot_uses_fractional_current_pos() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[
            PathDelta { x: 1, y: 0 },
            PathDelta { x: 2, y: 0 },
            PathDelta { x: 3, y: 0 },
            PathDelta { x: 4, y: 0 },
        ])
        .unwrap();
        // Mid-path: fractional at ~2.5 → lrint 2 or 3 depending; set explicitly.
        st.current_pos_x = 2.6;
        st.current_pos_y = 0.0;
        st.path_dist_traveled = 2.6;
        // hint from stale PU grid (0,0) must NOT win over fractional currentPos.
        let spot = st.closest_path_spot(Some((0, 0)));
        assert_eq!(spot, (3, 0), "lrint(2.6)=3 on path");
        // Off-path fractional snaps to nearest path cell.
        st.current_pos_x = 2.1;
        st.current_pos_y = 0.4;
        let spot2 = st.closest_path_spot(Some((0, 0)));
        assert_eq!(spot2, (2, 0));
    }

    #[test]
    fn idle_closest_path_spot_uses_hint_or_dest() {
        let st = MoveState::new(5, 7);
        assert_eq!(st.closest_path_spot(Some((5, 7))), (5, 7));
        assert_eq!(st.closest_path_spot(None), (5, 7));
    }

    #[test]
    fn done_moving_snaps_current_pos() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 2, y: 0 }]).unwrap();
        st.step_current_pos(0.2);
        assert!(st.current_pos_x > 0.0);
        st.on_player_update(2, false, 2, 0);
        assert!(!st.in_motion);
        assert_eq!((st.current_pos_x, st.current_pos_y), (2.0, 0.0));
        assert_eq!(st.path_speed, 0.0);
    }

    #[test]
    fn on_own_pm_timing_sets_speed_from_total_sec() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }, PathDelta { x: 2, y: 0 }])
            .unwrap();
        // 2 tiles in 1.0 sec → speed 2.0
        st.on_own_pm_timing(0, 0, &[(1, 0), (2, 0)], 1.0);
        assert!((st.path_speed - 2.0).abs() < 1e-9);
        st.step_current_pos(0.5);
        assert!((st.current_pos_x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn measure_path_length_diag() {
        let path = [(0, 0), (1, 1), (2, 1)];
        let len = MoveState::measure_path_length(&path);
        assert!((len - (PATH_DIAG_LEN + 1.0)).abs() < 1e-9);
    }

    #[test]
    fn arm_multi_move_tracks_goal_and_chunks() {
        let mut st = MoveState::new(0, 0);
        let multi = st.arm_multi_move(
            vec![vec![PathDelta { x: 4, y: 0 }]],
            (20, 0),
            (16, 0),
        );
        assert!(multi);
        assert!(st.has_multi_move());
        let c = st.take_next_move_chunk().unwrap();
        assert_eq!(c, vec![PathDelta { x: 4, y: 0 }]);
        assert_eq!(st.pending_move_goal, Some((20, 0)));
        st.clear_multi_move();
        assert!(!st.has_multi_move());
    }
}
