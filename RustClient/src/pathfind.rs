//! Client pathfinding (C++ `pathFind.cpp` + `LivingLifePage::computePathToDest` subset).
//!
//! A* on an 8-neighbor grid using [`ClientMap`] + optional [`ClientContent`] blocking.
//!
//! **MOVE wire shape** (C++ `LivingLifePage.cpp` ~26522–26528, `protocol.txt`):
//! path cells after the start are encoded as **cumulative offsets from start**
//! `(path[i].x - start.x, path[i].y - start.y)`, each within ±[`MAX_PATH_DELTA`] (16).
//!
//! Search window matches C++ `pathFindingD = 32` (radius ~16 around start).
//!
//! **Cost map** (C++ `computePathToDest` ~2451–2538):
//! - Unknown map cells are **blocked** (`mMap == -1`).
//! - Walkable iff `object_id == 0` or known object with `!blocksWalking`
//!   (C++ does **not** block on `permanent` alone).
//! - Wide objects expand blocking via `leftBlockingRadius` / `rightBlockingRadius`.
//! - Blocked goal cells are never entered (fail → closest reachable).
//!
//! **Bad biomes** (C++ `isBadBiome` + ~2481–2504, rideable `ignoreBad`):
//! - Floor-less cells whose biome is in the BB list are "bad".
//! - From good terrain, long paths **route around** bad biomes (edge stop).
//! - Standing on a bad-biome edge with a bad dest allows entry.
//! - Standing *in* a bad biome allows same-biome walk; other bad biomes blocked.
//! - Holding a **rideable** sets `ignore_bad` and walks through freely.
//!
//! **useWaypoint two-leg** (C++ `pathFind(start,wp,goal)` + `maxWaypointPathLength`):
//! - [`find_path_via_waypoint_ex`] runs start→waypoint then waypoint→goal on one window.
//! - Both legs must reach; if combined path cells > max, dest becomes the waypoint.
//! - On failure, [`find_path_with_waypoint_ex`] falls back to direct start→goal.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::client_map::{ClientMap, MapTile};
use crate::content::ClientContent;
use crate::move_state::MAX_PATH_DELTA;

/// C++ `pathFindingD` — square window centered on the path start.
pub const PATH_FINDING_D: i32 = 32;

/// Default A* expand cap (~`pathFindingD²`).
pub const DEFAULT_MAX_EXPAND: usize = 1024;

/// C++ `LiveObject::maxWaypointPathLength` default when mouse-hold throws out a long click.
///
/// Path cell count **including start**; if the two-leg path exceeds this, dest becomes the waypoint.
pub const DEFAULT_MAX_WAYPOINT_PATH_LENGTH: i32 = 10;

/// C++ close-hold throw distance in tiles (`CELL_D * 4` / `CELL_D`).
pub const CLOSE_HOLD_THROW_TILES: i32 = 4;

const WIN: usize = PATH_FINDING_D as usize;
const WIN_CELLS: usize = WIN * WIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    f: i32,
    g: i32,
    /// Linear index into the 32×32 window.
    idx: u16,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // min-heap by f, then prefer lower g (C++ isRecordBetter total then estimate)
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| self.idx.cmp(&other.idx))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// C++ `getGridDistance` — Manhattan.
fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}

/// Result of a client path search ready for MOVE encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathFindResult {
    /// Cumulative deltas from `start` for each path cell **after** start
    /// (C++ MOVE pairs: `pathToDest[i] - pathToDest[0]`).
    pub deltas: Vec<(i32, i32)>,
    /// Absolute end tile of the path (start if empty / no move).
    pub end: (i32, i32),
    /// Absolute start used for search.
    pub start: (i32, i32),
    /// True when `end` equals the requested goal.
    pub reached_goal: bool,
    /// Closest reachable absolute tile (equals goal on success; C++ `outClosest`).
    pub closest: (i32, i32),
}

impl PathFindResult {
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// Absolute grid positions along the path, including start.
    pub fn absolute_cells(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::with_capacity(self.deltas.len() + 1);
        out.push(self.start);
        for &(dx, dy) in &self.deltas {
            out.push((self.start.0 + dx, self.start.1 + dy));
        }
        out
    }
}

/// Whether a world cell blocks walking for pathfinding.
///
/// C++ `computePathToDest` blockedMap fill (~2472–2478):
/// - unknown (`mMap == -1` / missing tile) → blocked
/// - `id == 0` → open
/// - known `!getObject(id)->blocksWalking` → open (permanent alone does **not** block)
pub fn cell_blocks_walking(
    map: &ClientMap,
    content: Option<&ClientContent>,
    x: i32,
    y: i32,
) -> bool {
    match map.get(x, y) {
        None => true,
        Some(t) if t.object_id <= 0 => false,
        Some(t) => {
            if let Some(c) = content {
                c.blocks_walking(t.object_id)
            } else {
                map.blocks_walk_heuristic(x, y)
            }
        }
    }
}

/// Walkable inverse of [`cell_blocks_walking`].
pub fn cell_walkable(map: &ClientMap, content: Option<&ClientContent>, x: i32, y: i32) -> bool {
    !cell_blocks_walking(map, content, x, y)
}

/// Options for biome-aware pathfinding (C++ `computePathToDest` bad-biome branch).
#[derive(Debug, Clone, Copy)]
pub struct PathFindOpts<'a> {
    /// Biome ids from server `BB` / `mBadBiomeIndices` (e.g. mountain, ocean).
    pub bad_biomes: &'a [u8],
    /// C++ `ignoreBad`: true when holding a rideable — walk through bad biomes.
    pub ignore_bad: bool,
    /// C++ `isAutoClick`: from good terrain, do not treat edge-of-bad as `startBiomeBad`
    /// (blocks auto-click entry into bad biomes).
    pub auto_click: bool,
}

impl Default for PathFindOpts<'static> {
    fn default() -> Self {
        Self {
            bad_biomes: &[],
            ignore_bad: false,
            auto_click: false,
        }
    }
}

impl PathFindOpts<'_> {
    /// Rideable vehicle: ignore bad-biome routing entirely.
    pub fn rideable() -> PathFindOpts<'static> {
        PathFindOpts {
            bad_biomes: &[],
            ignore_bad: true,
            auto_click: false,
        }
    }

    pub fn with_bad_biomes(bad_biomes: &[u8]) -> PathFindOpts<'_> {
        PathFindOpts {
            bad_biomes,
            ignore_bad: false,
            auto_click: false,
        }
    }
}

/// C++ `LivingLifePage::isBadBiome` — floor-less tile whose biome is on the BB list.
#[inline]
pub fn is_bad_biome_tile(tile: &MapTile, bad_biomes: &[u8]) -> bool {
    tile.floor_id == 0 && bad_biomes.iter().any(|&b| b == tile.biome)
}

/// C++ `isBadBiome` at world `(x,y)` (missing tile → not bad).
pub fn is_bad_biome_at(map: &ClientMap, x: i32, y: i32, bad_biomes: &[u8]) -> bool {
    map.get(x, y)
        .map(|t| is_bad_biome_tile(t, bad_biomes))
        .unwrap_or(false)
}

/// Parse server `BB` / `BAD_BIOMES` message body into `(biome_id, name)` pairs.
///
/// Wire lines after the tag: `21 MOUNTAIN` / `9 OCEAN` (underscores → spaces in names).
pub fn parse_bad_biomes(body: &str) -> Vec<(u8, String)> {
    let mut out = Vec::new();
    for line in body.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let Some(id_s) = it.next() else {
            continue;
        };
        let Ok(id) = id_s.parse::<i32>() else {
            continue;
        };
        if !(0..=255).contains(&id) {
            continue;
        }
        let name = it
            .collect::<Vec<_>>()
            .join(" ")
            .replace('_', " ");
        out.push((id as u8, name));
    }
    out
}

/// Just the biome id list from a `BB` body.
pub fn parse_bad_biome_ids(body: &str) -> Vec<u8> {
    parse_bad_biomes(body).into_iter().map(|(id, _)| id).collect()
}

/// Build C++-style 32×32 blocked window around `start` (local y-major).
///
/// Returns `(blocked, half)` where world `(wx,wy)` maps to local
/// `(wx - start.x + half, wy - start.y + half)`.
///
/// `goal` is used only for bad-biome edge routing (`destBiomeBad`).
fn build_blocked_window(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    opts: &PathFindOpts<'_>,
) -> ([bool; WIN_CELLS], i32) {
    let half = PATH_FINDING_D / 2;
    let mut blocked = [true; WIN_CELLS];

    // C++ start/dest bad-biome flags (~2397–2431).
    let start_point_bad = is_bad_biome_at(map, start.0, start.1, opts.bad_biomes);
    let start_point_bad_biome: Option<u8> = if start_point_bad {
        map.get(start.0, start.1).map(|t| t.biome)
    } else {
        None
    };
    let neigh_bad = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
        is_bad_biome_at(map, start.0 + dx, start.1 + dy, opts.bad_biomes)
    });
    let mut start_biome_bad = start_point_bad || neigh_bad;
    // C++: auto-click from good tile must not treat edge-of-bad as startBiomeBad.
    if opts.auto_click && !start_point_bad {
        start_biome_bad = false;
    }
    let dest_biome_bad = is_bad_biome_at(map, goal.0, goal.1, opts.bad_biomes);

    // C++ first pass: per-cell object walkability + bad biome (~2461–2507).
    for ly in 0..PATH_FINDING_D {
        for lx in 0..PATH_FINDING_D {
            let wx = start.0 - half + lx;
            let wy = start.1 - half + ly;
            let i = (ly * PATH_FINDING_D + lx) as usize;
            blocked[i] = cell_blocks_walking(map, content, wx, wy);

            if opts.ignore_bad || opts.bad_biomes.is_empty() {
                continue;
            }
            let Some(t) = map.get(wx, wy) else {
                continue;
            };
            if !is_bad_biome_tile(t, opts.bad_biomes) {
                continue;
            }
            // Route around bad biomes on long paths (from good / mixed).
            if (!start_biome_bad || !dest_biome_bad) && !start_point_bad {
                blocked[i] = true;
            } else if start_point_bad {
                // Crossing from one bad biome to another while standing in bad.
                if let Some(sb) = start_point_bad_biome {
                    if t.biome != sb {
                        blocked[i] = true;
                    }
                }
            }
        }
    }

    // C++ second pass: wide-object horizontal expansion (~2509–2538).
    if let Some(c) = content {
        for ly in 0..PATH_FINDING_D {
            for lx in 0..PATH_FINDING_D {
                let wx = start.0 - half + lx;
                let wy = start.1 - half + ly;
                let Some(t) = map.get(wx, wy) else {
                    continue;
                };
                if t.object_id <= 0 {
                    continue;
                }
                let base = c.base_object_id(t.object_id);
                let Some(def) = c.get(base) else {
                    continue;
                };
                let left = def.left_blocking_radius;
                let right = def.right_blocking_radius;
                if left <= 0 && right <= 0 {
                    continue;
                }
                for dx in -left..=right {
                    let nlx = lx + dx;
                    if nlx >= 0 && nlx < PATH_FINDING_D {
                        let i = (ly * PATH_FINDING_D + nlx) as usize;
                        blocked[i] = true;
                    }
                }
            }
        }
    }

    // Standing cell is always enterable for expansion (we're already there).
    let sx = half;
    let sy = half;
    blocked[(sy * PATH_FINDING_D + sx) as usize] = false;

    (blocked, half)
}

#[inline]
fn local_of(half: i32, start: (i32, i32), world: (i32, i32)) -> Option<(i32, i32)> {
    let lx = world.0 - start.0 + half;
    let ly = world.1 - start.1 + half;
    if lx >= 0 && lx < PATH_FINDING_D && ly >= 0 && ly < PATH_FINDING_D {
        Some((lx, ly))
    } else {
        None
    }
}

#[inline]
fn idx_of(lx: i32, ly: i32) -> usize {
    (ly * PATH_FINDING_D + lx) as usize
}

#[inline]
fn world_of(half: i32, start: (i32, i32), lx: i32, ly: i32) -> (i32, i32) {
    (start.0 - half + lx, start.1 - half + ly)
}

/// Neighbor deltas: long-axis orthos first, then remaining orthos, then diags.
///
/// C++ `pathFind.cpp` ~287–326: if `|dy| > |dx|` expand N/S before W/E; else W/E first.
fn neighbor_order(x_total_delta: i32, y_total_delta: i32) -> [(i32, i32); 8] {
    if y_total_delta > x_total_delta {
        [
            (0, -1),
            (0, 1),
            (-1, 0),
            (1, 0),
            (-1, -1),
            (-1, 1),
            (1, 1),
            (1, -1),
        ]
    } else {
        [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 1),
            (1, 1),
            (1, -1),
        ]
    }
}

/// Full path search with closest-reachable fallback (C++ `pathFind` + `computePathToDest`).
///
/// - Window: [`PATH_FINDING_D`]×[`PATH_FINDING_D`] centered on `start`.
/// - Unknown tiles are **blocked** (C++ `mMap == -1`).
/// - Goal cell must be unblocked to be entered (blocked goal → closest fallback).
/// - Returns cumulative MOVE deltas; empty when start==goal or totally stuck.
///
/// Uses default [`PathFindOpts`] (no bad biomes). Prefer [`find_path_ex`] when BB / rideable.
pub fn find_path(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
) -> PathFindResult {
    find_path_ex(
        map,
        content,
        start,
        goal,
        max_expand,
        &PathFindOpts::default(),
    )
}

/// Like [`find_path`] with bad-biome edge routing and rideable `ignore_bad`.
pub fn find_path_ex(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
    opts: &PathFindOpts<'_>,
) -> PathFindResult {
    let empty = PathFindResult {
        deltas: Vec::new(),
        end: start,
        start,
        reached_goal: start == goal,
        closest: start,
    };
    if start == goal {
        return empty;
    }

    let (blocked, half) = build_blocked_window(map, content, start, goal, opts);
    find_path_on_blocked(&blocked, half, start, start, goal, max_expand)
}

/// A* on a prebuilt 32×32 blocked window (world coords; window centered on `window_origin`).
///
/// `search_start` may differ from `window_origin` (second leg of a waypoint path still uses
/// the blocked map built around the original path origin — C++ two-leg `pathFind`).
fn find_path_on_blocked(
    blocked: &[bool; WIN_CELLS],
    half: i32,
    window_origin: (i32, i32),
    search_start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
) -> PathFindResult {
    let empty = PathFindResult {
        deltas: Vec::new(),
        end: search_start,
        start: search_start,
        reached_goal: search_start == goal,
        closest: search_start,
    };
    if search_start == goal {
        return empty;
    }

    // Local copy so second-leg search can open a blocked waypoint cell we're standing on.
    let mut blocked = *blocked;

    // Clamp goal into window for search target (C++ end is offset into local map).
    let search_goal_world = (
        goal.0
            .clamp(window_origin.0 - half, window_origin.0 + half - 1),
        goal.1
            .clamp(window_origin.1 - half, window_origin.1 + half - 1),
    );
    let Some((glx, gly)) = local_of(half, window_origin, search_goal_world) else {
        return empty;
    };
    let goal_idx = idx_of(glx, gly);

    // Search start must lie in the window.
    let Some((slx, sly)) = local_of(half, window_origin, search_start) else {
        return empty;
    };
    let start_idx = idx_of(slx, sly);
    // Standing cell is always expandable (same as build_blocked_window for origin).
    blocked[start_idx] = false;

    // C++ never steps onto blockedMap cells — blocked goal cannot be reached.
    let goal_blocked = blocked[goal_idx];

    // Fast path: single adjacent step into a free cell (efficiency; wire-identical).
    let adx = goal.0 - search_start.0;
    let ady = goal.1 - search_start.1;
    if adx.abs() <= 1
        && ady.abs() <= 1
        && (adx != 0 || ady != 0)
        && goal == search_goal_world
    {
        if !goal_blocked {
            return PathFindResult {
                deltas: vec![(adx, ady)],
                end: goal,
                start: search_start,
                reached_goal: true,
                closest: goal,
            };
        }
        // Blocked adjacent goal → empty (closest is start).
        return empty;
    }

    let x_total = (search_goal_world.0 - search_start.0).abs();
    let y_total = (search_goal_world.1 - search_start.1).abs();
    let nbrs = neighbor_order(x_total, y_total);

    // Fixed-window scratch (C++ openMap/doneMap + cost arrays).
    let mut gscore = [i32::MAX; WIN_CELLS];
    let mut closed = [false; WIN_CELLS];
    // Parent linear index; WIN_CELLS means none.
    let mut came = [WIN_CELLS as u16; WIN_CELLS];

    let mut open = BinaryHeap::new();
    let h0 = manhattan(
        search_start.0,
        search_start.1,
        search_goal_world.0,
        search_goal_world.1,
    );
    gscore[start_idx] = 0;
    open.push(Node {
        f: h0,
        g: 0,
        idx: start_idx as u16,
    });

    let mut best_closest_idx = start_idx;
    let mut best_est = h0;
    let mut found_goal = false;
    let mut expands = 0usize;

    while let Some(Node { idx, g, .. }) = open.pop() {
        let idx = idx as usize;
        if closed[idx] {
            continue;
        }
        closed[idx] = true;
        expands += 1;
        if expands > max_expand {
            break;
        }

        let ly = (idx as i32) / PATH_FINDING_D;
        let lx = (idx as i32) % PATH_FINDING_D;
        let (wx, wy) = world_of(half, window_origin, lx, ly);

        let est = manhattan(wx, wy, search_goal_world.0, search_goal_world.1);
        if est < best_est {
            best_est = est;
            best_closest_idx = idx;
        }

        if idx == goal_idx && !goal_blocked {
            found_goal = true;
            break;
        }

        // C++ forbids "down" (incl. diag) when standing on a blocked start cell
        // (~355–361). We force the start cell open, so this rarely triggers;
        // still honor it if `blocked[start]` is re-set by wide expansion.
        let forbid_down = blocked[idx];

        for &(dx, dy) in &nbrs {
            if forbid_down && dy == -1 {
                continue;
            }
            let nlx = lx + dx;
            let nly = ly + dy;
            if nlx < 0 || nlx >= PATH_FINDING_D || nly < 0 || nly >= PATH_FINDING_D {
                continue;
            }
            let nidx = idx_of(nlx, nly);
            // C++: only enter unblocked cells (goal included).
            if blocked[nidx] {
                continue;
            }
            if closed[nidx] {
                continue;
            }
            let ng = g + 1;
            if ng < gscore[nidx] {
                gscore[nidx] = ng;
                came[nidx] = idx as u16;
                let (nwx, nwy) = world_of(half, window_origin, nlx, nly);
                let h = manhattan(nwx, nwy, search_goal_world.0, search_goal_world.1);
                open.push(Node {
                    f: ng + h,
                    g: ng,
                    idx: nidx as u16,
                });
            }
        }
    }

    let (end_idx, reached) = if found_goal {
        (goal_idx, search_goal_world == goal)
    } else if best_closest_idx != start_idx {
        (best_closest_idx, false)
    } else {
        return empty;
    };

    if end_idx == start_idx {
        return empty;
    }

    // Reconstruct absolute cells end→start, then cumulative deltas from search_start.
    let mut cells_rev: Vec<(i32, i32)> = Vec::new();
    let mut cur = end_idx;
    while cur != start_idx {
        let ly = (cur as i32) / PATH_FINDING_D;
        let lx = (cur as i32) % PATH_FINDING_D;
        cells_rev.push(world_of(half, window_origin, lx, ly));
        let p = came[cur] as usize;
        if p >= WIN_CELLS {
            return empty;
        }
        cur = p;
    }
    cells_rev.reverse();

    let deltas: Vec<(i32, i32)> = cells_rev
        .into_iter()
        .map(|(x, y)| (x - search_start.0, y - search_start.1))
        .collect();
    if deltas.is_empty() {
        return empty;
    }

    // Protocol: every cumulative delta must be within ±16.
    let mut truncated = Vec::new();
    for &(dx, dy) in &deltas {
        if dx.abs() > MAX_PATH_DELTA || dy.abs() > MAX_PATH_DELTA {
            break;
        }
        truncated.push((dx, dy));
    }
    if truncated.is_empty() {
        return empty;
    }
    let last = *truncated.last().unwrap();
    let end = (search_start.0 + last.0, search_start.1 + last.1);
    let full = truncated.len() == deltas.len();
    PathFindResult {
        deltas: truncated,
        end,
        start: search_start,
        reached_goal: reached && full && end == goal,
        closest: end,
    }
}

/// C++ path cell count including start (`pathLength`).
#[inline]
pub fn path_cell_count(res: &PathFindResult) -> i32 {
    if res.deltas.is_empty() {
        if res.reached_goal {
            0 // C++ degen start==goal → pathLength 0
        } else {
            1
        }
    } else {
        res.deltas.len() as i32 + 1
    }
}

/// Combine two cumulative-delta legs that share a waypoint into one start-relative path.
///
/// `leg1` is start→waypoint (deltas relative to `start`); `leg2` is waypoint→goal
/// (deltas relative to `waypoint`). Skips the duplicate waypoint cell.
fn combine_waypoint_legs(
    start: (i32, i32),
    waypoint: (i32, i32),
    goal: (i32, i32),
    leg1: &PathFindResult,
    leg2: &PathFindResult,
) -> PathFindResult {
    let mut deltas = leg1.deltas.clone();
    // leg2 deltas are relative to waypoint; re-base to original start and skip index 0 of
    // absolute second path (the waypoint, already last of leg1 if leg1 non-empty).
    for &(dx, dy) in &leg2.deltas {
        let abs = (waypoint.0 + dx, waypoint.1 + dy);
        deltas.push((abs.0 - start.0, abs.1 - start.1));
    }
    // Enforce ±16 protocol truncation on combined path.
    let mut truncated = Vec::new();
    for &(dx, dy) in &deltas {
        if dx.abs() > MAX_PATH_DELTA || dy.abs() > MAX_PATH_DELTA {
            break;
        }
        truncated.push((dx, dy));
    }
    if truncated.is_empty() {
        return PathFindResult {
            deltas: Vec::new(),
            end: start,
            start,
            reached_goal: false,
            closest: start,
        };
    }
    let last = *truncated.last().unwrap();
    let end = (start.0 + last.0, start.1 + last.1);
    let full = truncated.len() == deltas.len();
    PathFindResult {
        deltas: truncated,
        end,
        start,
        reached_goal: full && end == goal && leg1.reached_goal && leg2.reached_goal,
        closest: end,
    }
}

/// Two-leg path through a waypoint (C++ `pathFind(start, waypoint, goal)`).
///
/// Both legs must **reach** their targets (C++ `pathFind` returns false on closest-only).
/// Uses one blocked window centered on `start` for both legs (C++ shared `blockedMap`).
pub fn find_path_via_waypoint(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    waypoint: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
) -> PathFindResult {
    find_path_via_waypoint_ex(
        map,
        content,
        start,
        waypoint,
        goal,
        max_expand,
        &PathFindOpts::default(),
    )
}

/// Two-leg path with biome / rideable options.
pub fn find_path_via_waypoint_ex(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    waypoint: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
    opts: &PathFindOpts<'_>,
) -> PathFindResult {
    let empty = PathFindResult {
        deltas: Vec::new(),
        end: start,
        start,
        reached_goal: start == goal,
        closest: start,
    };

    // Degenerate: no real waypoint needed.
    if waypoint == start {
        return find_path_ex(map, content, start, goal, max_expand, opts);
    }
    if waypoint == goal {
        return find_path_ex(map, content, start, goal, max_expand, opts);
    }
    if start == goal {
        return empty;
    }

    // Shared blocked map around start; destBiomeBad uses ultimate goal (C++ computePathToDest).
    let (blocked, half) = build_blocked_window(map, content, start, goal, opts);

    let leg1 = find_path_on_blocked(&blocked, half, start, start, waypoint, max_expand);
    // C++ firstFound: must actually reach the waypoint (not closest fallback).
    if !leg1.reached_goal {
        return PathFindResult {
            deltas: Vec::new(),
            end: start,
            start,
            reached_goal: false,
            closest: leg1.closest,
        };
    }

    let leg2 = find_path_on_blocked(&blocked, half, start, waypoint, goal, max_expand);
    // C++ secondFound false → discard first leg entirely.
    if !leg2.reached_goal {
        return PathFindResult {
            deltas: Vec::new(),
            end: start,
            start,
            reached_goal: false,
            closest: leg2.closest,
        };
    }

    combine_waypoint_legs(start, waypoint, goal, &leg1, &leg2)
}

/// C++ `computePathToDest` waypoint branch + direct fallback.
///
/// 1. If `waypoint` is `None` → ordinary [`find_path_ex`].
/// 2. Else two-leg through waypoint; if path cell count > `max_waypoint_path_length`,
///    repath **only** to the waypoint (dest becomes waypoint).
/// 3. If two-leg fails (blocked waypoint / second leg), fall back to direct start→goal
///    (C++ caller clears `useWaypoint` and recomputes).
///
/// Returns `(result, effective_goal)` — effective goal may be rewritten to the waypoint
/// when the two-leg path is too long.
pub fn find_path_with_waypoint_ex(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
    opts: &PathFindOpts<'_>,
    waypoint: Option<(i32, i32)>,
    max_waypoint_path_length: i32,
) -> (PathFindResult, (i32, i32)) {
    let Some(wp) = waypoint else {
        return (
            find_path_ex(map, content, start, goal, max_expand, opts),
            goal,
        );
    };

    let two = find_path_via_waypoint_ex(map, content, start, wp, goal, max_expand, opts);
    if two.reached_goal {
        let cells = path_cell_count(&two);
        if max_waypoint_path_length > 0 && cells > max_waypoint_path_length {
            // Path through waypoint too long → stop at waypoint (C++ ~2566–2579).
            let to_wp = find_path_ex(map, content, start, wp, max_expand, opts);
            return (to_wp, wp);
        }
        return (two, goal);
    }

    // Waypoint path failed (blocked waypoint or second leg) → direct, no waypoint.
    (
        find_path_ex(map, content, start, goal, max_expand, opts),
        goal,
    )
}

/// Cumulative MOVE deltas only (empty if no path / same tile).
///
/// Prefer [`find_path`] / [`find_path_ex`] when closest/reached metadata is needed.
pub fn find_path_deltas(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
) -> Vec<(i32, i32)> {
    find_path(map, content, start, goal, max_expand).deltas
}

/// Cumulative deltas with biome / rideable options.
pub fn find_path_deltas_ex(
    map: &ClientMap,
    content: Option<&ClientContent>,
    start: (i32, i32),
    goal: (i32, i32),
    max_expand: usize,
    opts: &PathFindOpts<'_>,
) -> Vec<(i32, i32)> {
    find_path_ex(map, content, start, goal, max_expand, opts).deltas
}

/// Convert adjacent step deltas `(-1..1)` into cumulative MOVE deltas.
pub fn steps_to_cumulative(steps: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(steps.len());
    let mut cx = 0i32;
    let mut cy = 0i32;
    for &(dx, dy) in steps {
        cx += dx;
        cy += dy;
        out.push((cx, cy));
    }
    out
}

/// Convert cumulative MOVE deltas into adjacent steps.
pub fn cumulative_to_steps(cum: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(cum.len());
    let mut px = 0i32;
    let mut py = 0i32;
    for &(cx, cy) in cum {
        out.push((cx - px, cy - py));
        px = cx;
        py = cy;
    }
    out
}

/// Split a long **cumulative** path into MOVE-sized segments.
///
/// Each segment's deltas are re-based relative to that segment's start and stay
/// within `max_abs` (±16). With the default 32 window a single MOVE is usual;
/// multi-hop is for follow-ups after `done_moving` on long travel.
pub fn chunk_deltas_for_move(deltas: &[(i32, i32)], max_abs: i32) -> Vec<Vec<(i32, i32)>> {
    let max_abs = max_abs.max(1);
    if deltas.is_empty() {
        return Vec::new();
    }
    let mut chunks: Vec<Vec<(i32, i32)>> = Vec::new();
    // Absolute offset of current segment origin from original path start.
    let mut seg_origin = (0i32, 0i32);
    let mut cur: Vec<(i32, i32)> = Vec::new();

    for &(ax, ay) in deltas {
        let rdx = ax - seg_origin.0;
        let rdy = ay - seg_origin.1;
        if rdx.abs() > max_abs || rdy.abs() > max_abs {
            if cur.is_empty() {
                // Cannot place even one cell — skip (should not happen for unit steps).
                continue;
            }
            if let Some(&(lx, ly)) = cur.last() {
                seg_origin = (seg_origin.0 + lx, seg_origin.1 + ly);
            }
            chunks.push(std::mem::take(&mut cur));
            let rdx = ax - seg_origin.0;
            let rdy = ay - seg_origin.1;
            if rdx.abs() > max_abs || rdy.abs() > max_abs {
                continue;
            }
            cur.push((rdx, rdy));
        } else {
            cur.push((rdx, rdy));
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
}

/// Fill a rectangle of empty (walkable) map cells for tests / sparse open fields.
pub fn fill_open_rect(map: &mut ClientMap, x0: i32, y0: i32, w: i32, h: i32) {
    use crate::parse::MapChunkHeader;
    let h_hdr = MapChunkHeader {
        size_x: w.max(0),
        size_y: h.max(0),
        x: x0,
        y: y0,
        binary_raw_size: None,
        binary_compressed_size: None,
    };
    let mut plain = String::with_capacity((w.max(0) * h.max(0) * 6) as usize);
    for _y in 0..h.max(0) {
        for _x in 0..w.max(0) {
            plain.push_str("0:0:0 ");
        }
    }
    let _ = map.apply_mc_plaintext(&h_hdr, plain.trim());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_map::ClientMap;
    use crate::content::{parse_object_txt, ClientContent};
    use crate::parse::MapChunkHeader;

    fn open_field_with_wall_gap() -> ClientMap {
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 5,
            size_y: 5,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let mut plain = String::new();
        for y in 0..5 {
            for x in 0..5 {
                if x == 2 && y != 2 {
                    plain.push_str("0:0:885 "); // wall
                } else {
                    plain.push_str("0:0:0 ");
                }
            }
        }
        map.apply_mc_plaintext(&h, plain.trim()).unwrap();
        map
    }

    fn open_line(map: &mut ClientMap, x0: i32, y0: i32, len: i32) {
        let h = MapChunkHeader {
            size_x: len,
            size_y: 1,
            x: x0,
            y: y0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let plain = (0..len).map(|_| "0:0:0 ").collect::<String>();
        map.apply_mc_plaintext(&h, plain.trim()).unwrap();
    }

    #[test]
    fn path_around_wall_cumulative() {
        let map = open_field_with_wall_gap();
        let res = find_path(&map, None, (0, 2), (4, 2), DEFAULT_MAX_EXPAND);
        assert!(!res.deltas.is_empty(), "should path around wall gap at (2,2)");
        assert!(res.reached_goal);
        assert_eq!(res.end, (4, 2));
        let last = *res.deltas.last().unwrap();
        assert_eq!(last, (4, 0));
        for &(dx, dy) in &res.deltas {
            let x = dx; // start (0,2)
            let y = 2 + dy;
            if (x, y) != (4, 2) {
                assert!(
                    !map.blocks_walk_heuristic(x, y),
                    "path cell ({x},{y}) blocked"
                );
            }
        }
    }

    #[test]
    fn empty_when_same_tile() {
        let map = ClientMap::new();
        let res = find_path(&map, None, (1, 1), (1, 1), 100);
        assert!(res.deltas.is_empty());
        assert!(res.reached_goal);
    }

    #[test]
    fn straight_line_cumulative_matches_cpp_move() {
        // Known empty cells (unknown is blocked — C++ mMap==-1).
        let mut map = ClientMap::new();
        open_line(&mut map, 0, 0, 3);
        let res = find_path(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert_eq!(res.deltas, vec![(1, 0), (2, 0)]);
        assert_eq!(res.end, (2, 0));
    }

    #[test]
    fn unknown_tiles_are_blocked() {
        // Empty ClientMap: no path across unknown cells (C++ blocked).
        let map = ClientMap::new();
        let res = find_path(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert!(
            res.deltas.is_empty(),
            "unknown cells must block: got {:?}",
            res.deltas
        );
    }

    #[test]
    fn closest_when_walled_off() {
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 3,
            size_y: 3,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        // Only (0,0) and (1,0) open; goal (1,2) sealed behind walls.
        let plain = "0:0:0 0:0:0 0:0:885 \
                     0:0:885 0:0:885 0:0:885 \
                     0:0:885 0:0:0 0:0:885";
        map.apply_mc_plaintext(&h, plain).unwrap();
        let res = find_path(&map, None, (0, 0), (1, 2), DEFAULT_MAX_EXPAND);
        assert!(!res.reached_goal);
        assert!(
            res.end == (0, 0) || res.end == (1, 0) || res.deltas.is_empty(),
            "closest end={:?}",
            res.end
        );
        if let Some(&last) = res.deltas.last() {
            let end = (0 + last.0, 0 + last.1);
            assert_ne!(end, (1, 2), "must not step onto unreachable goal");
        }
    }

    #[test]
    fn blocked_goal_not_entered() {
        // Goal cell is a wall; A* must not step onto it (C++ blockedMap).
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 3,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&h, "0:0:0 0:0:0 0:0:885").unwrap();
        let res = find_path(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert!(!res.reached_goal);
        assert_ne!(res.end, (2, 0));
        // Closest should be the free cell before the wall.
        assert!(
            res.end == (1, 0) || res.deltas.is_empty() || res.end == (0, 0),
            "end={:?}",
            res.end
        );
    }

    #[test]
    fn permanent_alone_does_not_block_with_content() {
        // C++: only blocksWalking gates path cells, not permanent.
        let mut content = ClientContent::default();
        let txt = "id=50\nBush\npermanent=1\nblocksWalking=0\n";
        let def = parse_object_txt(50, txt).unwrap();
        content.objects.insert(50, def);

        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 3,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&h, "0:0:0 0:0:50 0:0:0").unwrap();
        assert!(!content.blocks_walking(50));
        let res = find_path(&map, Some(&content), (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert!(res.reached_goal, "permanent bush must be walkable");
        assert_eq!(res.end, (2, 0));
    }

    #[test]
    fn blocks_walking_content_blocks() {
        let mut content = ClientContent::default();
        let txt = "id=885\nWall\npermanent=1\nblocksWalking=1\n";
        content.objects.insert(885, parse_object_txt(885, txt).unwrap());
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 3,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&h, "0:0:0 0:0:885 0:0:0").unwrap();
        let res = find_path(&map, Some(&content), (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert!(!res.reached_goal);
    }

    #[test]
    fn wide_object_expands_blocking() {
        let mut content = ClientContent::default();
        let txt = "id=70\nTruck\npermanent=1\nblocksWalking=1\n\
                   leftBlockingRadius=1\nrightBlockingRadius=1\n";
        let def = parse_object_txt(70, txt).unwrap();
        assert_eq!(def.left_blocking_radius, 1);
        assert_eq!(def.right_blocking_radius, 1);
        content.objects.insert(70, def);

        let mut map = ClientMap::new();
        // Row: empty, empty, truck@2, empty, empty — truck wide blocks 1..3
        let h = MapChunkHeader {
            size_x: 5,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&h, "0:0:0 0:0:0 0:0:70 0:0:0 0:0:0")
            .unwrap();
        // From (0,0) to (4,0) blocked by wide truck covering x=1,2,3
        let res = find_path(&map, Some(&content), (0, 0), (4, 0), DEFAULT_MAX_EXPAND);
        assert!(!res.reached_goal, "wide truck should seal the row");
    }

    #[test]
    fn steps_cumulative_roundtrip() {
        let steps = vec![(1, 0), (1, 0), (0, 1)];
        let cum = steps_to_cumulative(&steps);
        assert_eq!(cum, vec![(1, 0), (2, 0), (2, 1)]);
        assert_eq!(cumulative_to_steps(&cum), steps);
    }

    #[test]
    fn chunk_single_fits() {
        let d = vec![(1, 0), (2, 0), (3, 1)];
        let chunks = chunk_deltas_for_move(&d, 16);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], d);
    }

    #[test]
    fn chunk_rebases_when_exceeds() {
        let mut d = Vec::new();
        for i in 1..=20 {
            d.push((i, 0));
        }
        let chunks = chunk_deltas_for_move(&d, 16);
        assert!(chunks.len() >= 2, "chunks={chunks:?}");
        let last1 = chunks[0].last().unwrap();
        assert!(last1.0.abs() <= 16);
        let last2 = chunks[1].last().unwrap();
        assert!(last2.0.abs() <= 16);
    }

    #[test]
    fn adjacent_one_step_fast_path() {
        let mut map = ClientMap::new();
        open_line(&mut map, 0, 0, 2);
        let res = find_path(&map, None, (0, 0), (1, 0), DEFAULT_MAX_EXPAND);
        assert_eq!(res.deltas, vec![(1, 0)]);
        assert!(res.reached_goal);
    }

    // -----------------------------------------------------------------------
    // Bad biome edge routing + rideable ignoreBad (P2#9)
    // -----------------------------------------------------------------------

    /// Synthetic map: biomes in cells, all objects empty. Format `biome:floor:obj`.
    fn synth_biomes(w: i32, h: i32, x0: i32, y0: i32, biome_at: impl Fn(i32, i32) -> u8) -> ClientMap {
        let mut map = ClientMap::new();
        let header = MapChunkHeader {
            size_x: w,
            size_y: h,
            x: x0,
            y: y0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        let mut plain = String::new();
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                plain.push_str(&format!("{}:0:0 ", biome_at(x, y)));
            }
        }
        map.apply_mc_plaintext(&header, plain.trim()).unwrap();
        map
    }

    fn path_visits_biome(map: &ClientMap, start: (i32, i32), deltas: &[(i32, i32)], biome: u8) -> bool {
        for &(dx, dy) in deltas {
            let (x, y) = (start.0 + dx, start.1 + dy);
            if map.get(x, y).map(|t| t.biome) == Some(biome) {
                return true;
            }
        }
        false
    }

    #[test]
    fn parse_bb_message_ids_and_names() {
        let body = "BB\n21 MOUNTAIN\n9 OCEAN\n17 RIVER_BANK\n";
        let entries = parse_bad_biomes(body);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], (21, "MOUNTAIN".into()));
        assert_eq!(entries[1], (9, "OCEAN".into()));
        assert_eq!(entries[2], (17, "RIVER BANK".into()));
        assert_eq!(parse_bad_biome_ids(body), vec![21, 9, 17]);
    }

    #[test]
    fn bad_biome_routes_around_strip() {
        // Row y=0: good(0) good(0) bad(21) good(0) good(0) — path must go around if possible.
        // Use 3-row map so path can detour N/S.
        let map = synth_biomes(5, 3, 0, 0, |x, y| {
            if y == 1 && x == 2 {
                21
            } else {
                0
            }
        });
        let bad = [21u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 1), (4, 1), DEFAULT_MAX_EXPAND, &opts);
        assert!(res.reached_goal, "should route around single bad cell: {:?}", res);
        assert!(
            !path_visits_biome(&map, (0, 1), &res.deltas, 21),
            "path must not step on bad biome: {:?}",
            res.absolute_cells()
        );
    }

    #[test]
    fn bad_biome_edge_stop_when_dest_inside() {
        // Start on good; dest deep in bad strip — should stop at edge (not enter).
        // x: 0=good, 1=good, 2..=4=bad(21)
        let map = synth_biomes(5, 1, 0, 0, |x, _| if x >= 2 { 21 } else { 0 });
        let bad = [21u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 0), (4, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(!res.reached_goal, "must not enter bad dest from good: {:?}", res);
        assert!(
            !path_visits_biome(&map, (0, 0), &res.deltas, 21),
            "must not step into bad: end={:?} cells={:?}",
            res.end,
            res.absolute_cells()
        );
        // Closest free cell is the good edge at x=1.
        assert_eq!(res.end, (1, 0), "edge stop expected at (1,0), got {:?}", res.end);
    }

    #[test]
    fn from_bad_edge_can_enter_bad_dest() {
        // Start adjacent to bad (edge) on good tile; dest inside bad → C++ allows entry.
        // Layout: x0 good, x1 bad, x2 bad. Start (0,0) has bad neighbor → startBiomeBad.
        let map = synth_biomes(3, 1, 0, 0, |x, _| if x >= 1 { 21 } else { 0 });
        let bad = [21u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(
            res.reached_goal,
            "from edge of bad into bad dest should succeed: {:?}",
            res
        );
        assert_eq!(res.end, (2, 0));
    }

    #[test]
    fn standing_in_bad_walks_same_biome() {
        // Entire row is bad biome 21; start and dest same bad.
        let map = synth_biomes(4, 1, 0, 0, |_, _| 21);
        let bad = [21u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 0), (3, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(
            res.reached_goal,
            "same bad biome walk: {:?}",
            res
        );
    }

    #[test]
    fn standing_in_bad_blocks_other_bad_biome() {
        // x0..=1 biome 21, x2..=3 biome 9 (both bad). Start in 21, dest in 9.
        let map = synth_biomes(4, 1, 0, 0, |x, _| if x < 2 { 21 } else { 9 });
        let bad = [21u8, 9u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 0), (3, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(!res.reached_goal, "must not cross into other bad: {:?}", res);
        assert!(
            !path_visits_biome(&map, (0, 0), &res.deltas, 9),
            "must not step on other bad biome"
        );
    }

    #[test]
    fn rideable_ignore_bad_crosses_bad_biomes() {
        // Same edge-stop layout as bad_biome_edge_stop, but ignore_bad (rideable).
        let map = synth_biomes(5, 1, 0, 0, |x, _| if x >= 2 { 21 } else { 0 });
        let bad = [21u8];
        let opts = PathFindOpts {
            bad_biomes: &bad,
            ignore_bad: true,
            auto_click: false,
        };
        let res = find_path_ex(&map, None, (0, 0), (4, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(
            res.reached_goal,
            "rideable must cross bad biomes: {:?}",
            res
        );
        assert!(path_visits_biome(&map, (0, 0), &res.deltas, 21));
    }

    #[test]
    fn floor_over_bad_biome_is_walkable() {
        // biome 21 with floor_id=1 → not isBadBiome (floor exception).
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 3,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        // good, floored mountain, good
        map.apply_mc_plaintext(&h, "0:0:0 21:1:0 0:0:0").unwrap();
        let bad = [21u8];
        let opts = PathFindOpts::with_bad_biomes(&bad);
        let res = find_path_ex(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND, &opts);
        assert!(res.reached_goal, "floor cancels bad biome: {:?}", res);
    }

    #[test]
    fn is_bad_biome_helpers() {
        let t_bad = MapTile {
            biome: 21,
            floor_id: 0,
            object_id: 0,
            object_raw: "0".into(),
        };
        let t_floor = MapTile {
            biome: 21,
            floor_id: 5,
            object_id: 0,
            object_raw: "0".into(),
        };
        let bad = [21u8, 9u8];
        assert!(is_bad_biome_tile(&t_bad, &bad));
        assert!(!is_bad_biome_tile(&t_floor, &bad));
        assert!(!is_bad_biome_tile(&t_bad, &[]));
    }

    #[test]
    fn auto_click_from_good_blocks_bad_entry() {
        // Edge of bad: start (0,0) good, neighbor (1,0) bad; dest (2,0) bad.
        // Without auto_click, startBiomeBad allows entry; with auto_click, blocked.
        let map = synth_biomes(3, 1, 0, 0, |x, _| if x >= 1 { 21 } else { 0 });
        let bad = [21u8];
        let normal = PathFindOpts::with_bad_biomes(&bad);
        let auto = PathFindOpts {
            bad_biomes: &bad,
            ignore_bad: false,
            auto_click: true,
        };
        let res_n = find_path_ex(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND, &normal);
        let res_a = find_path_ex(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND, &auto);
        assert!(res_n.reached_goal, "manual from edge enters: {:?}", res_n);
        assert!(
            !res_a.reached_goal,
            "auto-click must not enter bad from good edge: {:?}",
            res_a
        );
    }

    // -----------------------------------------------------------------------
    // useWaypoint two-leg pathFind (P2#10)
    // -----------------------------------------------------------------------

    #[test]
    fn via_waypoint_visits_midpoint() {
        // Open field; path start→wp→goal must include the waypoint cell.
        let mut map = ClientMap::new();
        fill_open_rect(&mut map, 0, 0, 8, 8);
        let start = (1, 1);
        let wp = (1, 4);
        let goal = (6, 4);
        let res = find_path_via_waypoint(&map, None, start, wp, goal, DEFAULT_MAX_EXPAND);
        assert!(res.reached_goal, "two-leg should reach goal: {:?}", res);
        assert_eq!(res.end, goal);
        let cells = res.absolute_cells();
        assert!(
            cells.contains(&wp),
            "path must pass through waypoint {wp:?}: {cells:?}"
        );
        // Path length = cells including start.
        assert_eq!(path_cell_count(&res) as usize, cells.len());
    }

    #[test]
    fn via_waypoint_combines_legs_cumulative() {
        let mut map = ClientMap::new();
        fill_open_rect(&mut map, 0, 0, 5, 3);
        // start (0,0) → wp (2,0) → goal (2,2)
        let res = find_path_via_waypoint(&map, None, (0, 0), (2, 0), (2, 2), DEFAULT_MAX_EXPAND);
        assert!(res.reached_goal, "{res:?}");
        assert_eq!(res.deltas.last().copied(), Some((2, 2)));
        // All cumulative deltas relative to start.
        for &(dx, dy) in &res.deltas {
            assert!(dx.abs() <= MAX_PATH_DELTA && dy.abs() <= MAX_PATH_DELTA);
        }
    }

    #[test]
    fn via_waypoint_fails_when_waypoint_blocked() {
        let mut map = ClientMap::new();
        let h = MapChunkHeader {
            size_x: 5,
            size_y: 1,
            x: 0,
            y: 0,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        // Wall at waypoint x=2.
        map.apply_mc_plaintext(&h, "0:0:0 0:0:0 0:0:885 0:0:0 0:0:0")
            .unwrap();
        let res = find_path_via_waypoint(&map, None, (0, 0), (2, 0), (4, 0), DEFAULT_MAX_EXPAND);
        assert!(!res.reached_goal);
        assert!(res.deltas.is_empty(), "C++ discards path when first leg fails");
    }

    #[test]
    fn via_waypoint_too_long_stops_at_waypoint() {
        let mut map = ClientMap::new();
        fill_open_rect(&mut map, 0, 0, 16, 4);
        let start = (0, 1);
        let wp = (2, 1);
        let goal = (14, 1);
        // Direct-ish two-leg is long; max length 4 forces stop at waypoint.
        let (res, eff_goal) = find_path_with_waypoint_ex(
            &map,
            None,
            start,
            goal,
            DEFAULT_MAX_EXPAND,
            &PathFindOpts::default(),
            Some(wp),
            4,
        );
        assert_eq!(eff_goal, wp, "dest rewritten to waypoint");
        assert!(
            res.reached_goal || res.end == wp,
            "should path to waypoint: {:?}",
            res
        );
        assert_eq!(res.end, wp);
        assert!(path_cell_count(&res) <= 4 || res.end == wp);
    }

    #[test]
    fn via_waypoint_short_path_keeps_goal() {
        let mut map = ClientMap::new();
        fill_open_rect(&mut map, 0, 0, 6, 3);
        let start = (0, 1);
        let wp = (2, 1);
        let goal = (4, 1);
        let (res, eff_goal) = find_path_with_waypoint_ex(
            &map,
            None,
            start,
            goal,
            DEFAULT_MAX_EXPAND,
            &PathFindOpts::default(),
            Some(wp),
            DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
        );
        assert_eq!(eff_goal, goal);
        assert!(res.reached_goal, "{res:?}");
        assert_eq!(res.end, goal);
        assert!(res.absolute_cells().contains(&wp));
    }

    #[test]
    fn via_waypoint_fallback_direct_when_wp_blocked() {
        // Waypoint blocked → direct path still works around/without it.
        let mut map = ClientMap::new();
        fill_open_rect(&mut map, 0, 0, 5, 3);
        // Block only the waypoint cell with a wall object id (heuristic blocks).
        let h = MapChunkHeader {
            size_x: 1,
            size_y: 1,
            x: 2,
            y: 1,
            binary_raw_size: None,
            binary_compressed_size: None,
        };
        map.apply_mc_plaintext(&h, "0:0:885").unwrap();
        let start = (0, 1);
        let wp = (2, 1);
        let goal = (4, 1);
        let (res, eff_goal) = find_path_with_waypoint_ex(
            &map,
            None,
            start,
            goal,
            DEFAULT_MAX_EXPAND,
            &PathFindOpts::default(),
            Some(wp),
            DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
        );
        assert_eq!(eff_goal, goal);
        // Direct can route around wall via y=0 or y=2.
        assert!(
            res.reached_goal,
            "direct fallback should reach goal: {:?}",
            res
        );
        assert_eq!(res.end, goal);
        // Must not require stepping on the wall waypoint.
        assert!(!res.absolute_cells().contains(&wp) || !map.blocks_walk_heuristic(wp.0, wp.1));
    }

    #[test]
    fn path_cell_count_includes_start() {
        let mut map = ClientMap::new();
        open_line(&mut map, 0, 0, 3);
        let res = find_path(&map, None, (0, 0), (2, 0), DEFAULT_MAX_EXPAND);
        assert_eq!(res.deltas, vec![(1, 0), (2, 0)]);
        assert_eq!(path_cell_count(&res), 3);
        let same = find_path(&map, None, (0, 0), (0, 0), DEFAULT_MAX_EXPAND);
        assert_eq!(path_cell_count(&same), 0);
    }
}
