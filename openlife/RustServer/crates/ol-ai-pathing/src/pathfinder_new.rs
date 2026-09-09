//! Haxe `openlife.auto.PathfinderNew` — bidirectional wave path on a collision chunk.
//!
//! `CreatePath`: greedy [`create_direct_path`] first; if blocked, paint reverse from dest
//! and expand both waves in concentric rings until they meet (`makePathOneTile`).
//! Brute-force loop respects Haxe `timeOut` (100ms) via [`PathBudget`].
//! // Haxe: PathfinderNew.WriteMapToFile ~94–129 debug dump — omitted (no disk write).
//! // Haxe: CreatePathBruteForce ~262–350 full-grid unused (CreatePath uses InCircle).
//!
//! // Haxe: PathfinderNew.CreatePath ~131–148
//! // Haxe: CreateDirectPath ~352–454
//! // Haxe: CreatePathBruteForceInCircle ~155–188
//! // Haxe: makePathInCircle / makePathOneTile ~190–260
//! // Haxe: AddPathFromCrossing ~456–496
//! // Haxe: CreatePathFromMap ~498–555

/// Haxe `MapData.RAD` — collision chunk radius (grid is `2 * radius`).
pub const PATHFINDER_NEW_DEFAULT_RADIUS: i32 = 32;
/// Cardinal step cost (`xlength` when px==0 || py==0).
pub const PATHFINDER_NEW_CARDINAL: f32 = 1.0;
/// Diagonal step cost (Haxe `1.4`).
pub const PATHFINDER_NEW_DIAGONAL: f32 = 1.4;
/// Haxe `PathfinderNew.timeOut` — brute-force abort (milliseconds).
// Haxe: PathfinderNew.timeOut = 100
pub const PATHFINDER_NEW_TIMEOUT_MS: u64 = 100;
/// Haxe `for (i in 0...width)` at default `RAD=32` (`width = 2 * radius`).
pub const PATHFINDER_NEW_MAX_EXPANSIONS: u32 = (PATHFINDER_NEW_DEFAULT_RADIUS * 2) as u32;

/// Budget for [`create_path_with_budget`] brute-force loop.
///
/// Haxe checks `(Sys.time()-startTime)*1000 > timeOut` each outer iteration.
/// `max_expansions` caps those iterations (Haxe `usedBruteForceIterations` / `0...width`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathBudget {
    pub max_ms: Option<u64>,
    pub max_expansions: Option<u32>,
}

impl Default for PathBudget {
    fn default() -> Self {
        Self::LIVE
    }
}

impl PathBudget {
    /// Live Goto: 100ms and Haxe width-at-RAD expansions (64).
    pub const LIVE: Self = Self {
        max_ms: Some(PATHFINDER_NEW_TIMEOUT_MS),
        max_expansions: Some(PATHFINDER_NEW_MAX_EXPANSIONS),
    };

    /// No time cap; stop brute force after `n` outer iterations (tests).
    pub fn expansions(n: u32) -> Self {
        Self {
            max_ms: None,
            max_expansions: Some(n),
        }
    }

    /// Time cap only (tests). `0` aborts brute force immediately.
    pub fn millis(ms: u64) -> Self {
        Self {
            max_ms: Some(ms),
            max_expansions: None,
        }
    }

    fn brute_timed_out(self, started: std::time::Instant, expansions: u32) -> bool {
        if let Some(max) = self.max_expansions {
            if expansions >= max {
                return true;
            }
        }
        if let Some(ms) = self.max_ms {
            // Haxe: `time > timeOut`. `0` is an immediate test abort (`elapsed >= 0`).
            if ms == 0 || started.elapsed().as_millis() as u64 > ms {
                return true;
            }
        }
        false
    }
}

#[inline]
fn idx(x: i32, y: i32, width: i32) -> usize {
    (x + y * width) as usize
}

#[inline]
fn in_grid(x: i32, y: i32, width: i32) -> bool {
    x >= 0 && y >= 0 && x < width && y < width
}

/// Greedy direct path, writing signed lengths onto `map`.
///
/// `factor` +1 from start, −1 reverse from dest. Returns true if dest reached
/// or reverse wave crossed a positive cell.
// Haxe: PathfinderNew.CreateDirectPath
pub fn create_direct_path(
    start: (i32, i32),
    dest: (i32, i32),
    width: i32,
    radius: i32,
    is_walkable: &dyn Fn(i32, i32) -> bool,
    current: &mut [f32],
    factor: f32,
) -> bool {
    let mut cx = start.0;
    let mut cy = start.1;
    let mut length = factor;
    let mut length_current: f32 = 0.0;
    if !in_grid(cx, cy, width) {
        return false;
    }
    current[idx(cx, cy, width)] = length;

    for _ in 0..radius.max(0) {
        length = current[idx(cx, cy, width)];
        let px = if cx < dest.0 { 1 } else { -1 };
        let py = if cy < dest.1 { 1 } else { -1 };

        // Haxe: currentX < 1 / >= width-1 (edge column/row unused)
        if cx < 1 || cy < 1 || cx >= width - 1 || cy >= width - 1 {
            break;
        }

        // Reverse wave stepped onto a forward-painted tile (detected next iter).
        if factor < 0.0 && length_current > 0.0 {
            current[idx(cx, cy, width)] = length_current;
            return true;
        }

        if cx == dest.0 && cy == dest.1 {
            break;
        }

        if cx == dest.0 && is_walkable(cx, cy + py) {
            cy += py;
            length += factor;
            length_current = current[idx(cx, cy, width)];
            current[idx(cx, cy, width)] = length;
            continue;
        }
        if cy == dest.1 && is_walkable(cx + px, cy) {
            cx += px;
            length += factor;
            length_current = current[idx(cx, cy, width)];
            current[idx(cx, cy, width)] = length;
            continue;
        }
        if is_walkable(cx + px, cy + py) {
            cx += px;
            cy += py;
            length += 1.4 * factor;
            length_current = current[idx(cx, cy, width)];
            current[idx(cx, cy, width)] = length;
            continue;
        }

        if cx == dest.0 {
            if is_walkable(cx - px, cy + py) {
                cx -= px;
                cy += py;
                length += 1.4 * factor;
                length_current = current[idx(cx, cy, width)];
                current[idx(cx, cy, width)] = length;
                continue;
            }
            break;
        }

        if cy == dest.1 {
            if is_walkable(cx + px, cy - py) {
                cx += px;
                cy -= py;
                length += 1.4 * factor;
                length_current = current[idx(cx, cy, width)];
                current[idx(cx, cy, width)] = length;
                continue;
            }
            break;
        }

        if is_walkable(cx, cy + py) {
            cy += py;
            length += factor;
            length_current = current[idx(cx, cy, width)];
            current[idx(cx, cy, width)] = length;
            continue;
        }
        if is_walkable(cx + px, cy) {
            cx += px;
            length += factor;
            length_current = current[idx(cx, cy, width)];
            current[idx(cx, cy, width)] = length;
            continue;
        }
        break;
    }

    cx == dest.0 && cy == dest.1
}

fn make_path_one_tile(
    x: i32,
    y: i32,
    width: i32,
    is_walkable: &dyn Fn(i32, i32) -> bool,
    current: &mut [f32],
    changed: &mut bool,
    changed_from_dest: &mut bool,
) -> Option<(i32, i32)> {
    if !in_grid(x, y, width) {
        return None;
    }
    let length = current[idx(x, y, width)];
    if length == 0.0 {
        return None;
    }
    if !is_walkable(x, y) {
        return None;
    }

    for py in -1..=1 {
        for px in -1..=1 {
            if px == 0 && py == 0 {
                continue;
            }
            if x + px < 0 || y + py < 0 {
                continue;
            }
            // Haxe FIX: width-1 so last biome border is not entered
            if x + px >= width - 1 || y + py >= width - 1 {
                continue;
            }
            let nx = x + px;
            let ny = y + py;
            let current_length = current[idx(nx, ny, width)];

            if (length > 0.0 && current_length < 0.0) || (length < 0.0 && current_length > 0.0) {
                return Some(if current_length > 0.0 {
                    (nx, ny)
                } else {
                    (x, y)
                });
            }

            let xlength = if px == 0 || py == 0 {
                PATHFINDER_NEW_CARDINAL
            } else {
                PATHFINDER_NEW_DIAGONAL
            };
            let newlength = if length > 0.0 {
                length + xlength
            } else {
                length - xlength
            };

            if !is_walkable(nx, ny) {
                continue;
            }
            if length > 0.0 && current_length != 0.0 && current_length <= newlength {
                continue;
            }
            if length < 0.0 && current_length != 0.0 && current_length >= newlength {
                continue;
            }

            if length > 0.0 {
                *changed = true;
            } else {
                *changed_from_dest = true;
            }
            current[idx(nx, ny, width)] = newlength;
        }
    }
    None
}

fn make_path_in_circle(
    start: (i32, i32),
    rad: i32,
    width: i32,
    is_walkable: &dyn Fn(i32, i32) -> bool,
    current: &mut [f32],
    changed: &mut bool,
    changed_from_dest: &mut bool,
) -> Option<(i32, i32)> {
    let y = rad + start.1;
    for xx in -rad..=rad {
        if let Some(c) = make_path_one_tile(
            xx + start.0,
            y,
            width,
            is_walkable,
            current,
            changed,
            changed_from_dest,
        ) {
            return Some(c);
        }
    }
    let y = -rad + start.1;
    for xx in -rad..=rad {
        if let Some(c) = make_path_one_tile(
            xx + start.0,
            y,
            width,
            is_walkable,
            current,
            changed,
            changed_from_dest,
        ) {
            return Some(c);
        }
    }
    let x = rad + start.0;
    for yy in (-rad + 1)..rad {
        if let Some(c) = make_path_one_tile(
            x,
            yy + start.1,
            width,
            is_walkable,
            current,
            changed,
            changed_from_dest,
        ) {
            return Some(c);
        }
    }
    let x = -rad + start.0;
    for yy in (-rad + 1)..rad {
        if let Some(c) = make_path_one_tile(
            x,
            yy + start.1,
            width,
            is_walkable,
            current,
            changed,
            changed_from_dest,
        ) {
            return Some(c);
        }
    }
    None
}

fn add_path_from_crossing(crossing: (i32, i32), width: i32, current: &mut [f32]) {
    let mut next = crossing;
    let mut new_length = current[idx(next.0, next.1, width)];
    let mut best_length: f32 = -1_000_000.0;

    for _ in 0..1000 {
        let (cx, cy) = next;
        let length = current[idx(cx, cy, width)];
        current[idx(cx, cy, width)] = new_length;
        new_length += 1.0; // Haxe: TODO does not consider diagonal
        if (length + 1.0).abs() < f32::EPSILON {
            break;
        }

        for py in -1..=1 {
            for px in -1..=1 {
                if px == 0 && py == 0 {
                    continue;
                }
                let nx = cx + px;
                let ny = cy + py;
                if !in_grid(nx, ny, width) {
                    continue;
                }
                let nlen = current[idx(nx, ny, width)];
                if nlen > -1.0 {
                    continue;
                }
                if nlen <= best_length {
                    continue;
                }
                best_length = nlen;
                next = (nx, ny);
            }
        }
    }
}

fn create_path_from_map(dest: (i32, i32), width: i32, current: &[f32]) -> Vec<(i32, i32)> {
    let dest_len = current[idx(dest.0, dest.1, width)];
    let max_length = dest_len.ceil().max(1.0) as i32;
    let mut reverse: Vec<(i32, i32)> = Vec::new();
    let mut next = dest;
    let mut cur = (i32::MIN / 4, i32::MIN / 4);

    for _ in 0..max_length {
        if cur == next {
            break;
        }
        cur = next;
        let length = current[idx(cur.0, cur.1, width)];
        reverse.push((cur.0 - dest.0, cur.1 - dest.1));
        if (length - 1.0).abs() < 1e-3 {
            break;
        }
        let mut best = length;
        for py in -1..=1 {
            for px in -1..=1 {
                if px == 0 && py == 0 {
                    continue;
                }
                let nx = cur.0 + px;
                let ny = cur.1 + py;
                if !in_grid(nx, ny, width) {
                    continue;
                }
                let nlen = current[idx(nx, ny, width)];
                if nlen < 1.0 {
                    continue;
                }
                if nlen >= best {
                    continue;
                }
                best = nlen;
                next = (nx, ny);
            }
        }
    }

    let mut path = Vec::with_capacity(reverse.len());
    while let Some(c) = reverse.pop() {
        path.push((c.0 + dest.0, c.1 + dest.1));
    }
    path
}

/// Convert a map-local path into successive (dx, dy) steps.
pub fn path_to_steps(path: &[(i32, i32)]) -> Vec<(i32, i32)> {
    path.windows(2)
        .map(|w| (w[1].0 - w[0].0, w[1].1 - w[0].1))
        .collect()
}

/// Haxe `PathfinderNew.CreatePath` with default 100ms brute timeout.
///
/// `start` / `dest` are in collision-chunk coords (`radius` is the origin).
/// `is_walkable` is Haxe `MapCollision.isWalkable`.
// Haxe: PathfinderNew.CreatePath
pub fn create_path(
    start: (i32, i32),
    dest: (i32, i32),
    radius: i32,
    is_walkable: &dyn Fn(i32, i32) -> bool,
) -> Option<Vec<(i32, i32)>> {
    create_path_with_budget(start, dest, radius, is_walkable, PathBudget::LIVE)
}

/// [`create_path`] with an explicit brute-force budget.
// Haxe: CreatePathBruteForceInCircle timeOut / usedBruteForceIterations
pub fn create_path_with_budget(
    start: (i32, i32),
    dest: (i32, i32),
    radius: i32,
    is_walkable: &dyn Fn(i32, i32) -> bool,
    budget: PathBudget,
) -> Option<Vec<(i32, i32)>> {
    let radius = radius.max(1);
    let width = 2 * radius;
    let n = (width * width) as usize;
    let mut current = vec![0.0f32; n];

    if create_direct_path(start, dest, width, radius, is_walkable, &mut current, 1.0) {
        return Some(create_path_from_map(dest, width, &current));
    }

    create_direct_path(dest, start, width, radius, is_walkable, &mut current, -1.0);

    let mut changed = true;
    let mut changed_from_dest = true;
    let mut crossing = None;
    let ring = (width as f32 / 2.0).ceil() as i32;
    let brute_start = std::time::Instant::now();
    let mut expansions: u32 = 0;

    for _i in 0..width {
        if !changed || !changed_from_dest {
            break;
        }
        // Haxe: time = (Sys.time()-startTime)*1000; if (time > timeOut) break
        if budget.brute_timed_out(brute_start, expansions) {
            break;
        }
        changed = false;
        changed_from_dest = false;

        for rad in 0..ring {
            if let Some(c) = make_path_in_circle(
                start,
                rad,
                width,
                is_walkable,
                &mut current,
                &mut changed,
                &mut changed_from_dest,
            ) {
                crossing = Some(c);
                break;
            }
            if let Some(c) = make_path_in_circle(
                dest,
                rad,
                width,
                is_walkable,
                &mut current,
                &mut changed,
                &mut changed_from_dest,
            ) {
                crossing = Some(c);
                break;
            }
        }
        if crossing.is_some() {
            break;
        }
        expansions = expansions.saturating_add(1);
    }

    let crossing = crossing?;
    add_path_from_crossing(crossing, width, &mut current);
    Some(create_path_from_map(dest, width, &current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(_x: i32, _y: i32) -> bool {
        true
    }

    #[test]
    fn direct_horizontal_open_field() {
        let radius = 8;
        let start = (8, 8);
        let dest = (12, 8);
        let path = create_path(start, dest, radius, &open).expect("path");
        assert_eq!(path.first().copied(), Some(start));
        assert_eq!(path.last().copied(), Some(dest));
        for w in path.windows(2) {
            let dx = (w[1].0 - w[0].0).abs();
            let dy = (w[1].1 - w[0].1).abs();
            assert!(dx <= 1 && dy <= 1 && (dx + dy) >= 1);
        }
    }

    #[test]
    fn same_tile_is_single_point() {
        let p = create_path((4, 4), (4, 4), 6, &open).expect("path");
        assert_eq!(p, vec![(4, 4)]);
        assert!(path_to_steps(&p).is_empty());
    }

    #[test]
    fn wall_forces_brute_force_detour() {
        let radius = 8;
        let start = (4, 8);
        let dest = (12, 8);
        // Vertical wall at x=8, y=1..14 except a gap at y=12
        let walk = |x: i32, y: i32| {
            if x == 8 && y != 12 {
                return false;
            }
            true
        };
        let path = create_path(start, dest, radius, &walk).expect("detour");
        assert_eq!(path.first().copied(), Some(start));
        assert_eq!(path.last().copied(), Some(dest));
        assert!(
            path.iter().any(|&(x, y)| x == 8 && y == 12),
            "must pass gap {:?}",
            path
        );
        // Direct line x=4..12 y=8 is blocked at (8,8)
        assert!(!path.iter().any(|&(x, y)| x == 8 && y == 8));
    }

    fn wall_gap_walk(x: i32, y: i32) -> bool {
        !(x == 8 && y != 12)
    }

    #[test]
    fn brute_force_zero_expansions_times_out() {
        let start = (4, 8);
        let dest = (12, 8);
        assert!(
            create_path_with_budget(start, dest, 8, &wall_gap_walk, PathBudget::expansions(0))
                .is_none()
        );
        assert!(create_path(start, dest, 8, &wall_gap_walk).is_some());
    }

    #[test]
    fn brute_force_zero_ms_times_out() {
        let start = (4, 8);
        let dest = (12, 8);
        assert!(
            create_path_with_budget(start, dest, 8, &wall_gap_walk, PathBudget::millis(0))
                .is_none()
        );
        assert!(create_path(start, dest, 8, &wall_gap_walk).is_some());
    }

    #[test]
    fn sealed_wall_returns_none() {
        let radius = 6;
        let start = (3, 6);
        let dest = (9, 6);
        // Solid wall at x=6, all y — waves cannot meet.
        let walk = |x: i32, _y: i32| x != 6;
        assert!(create_path(start, dest, radius, &walk).is_none());
    }

    #[test]
    fn diagonal_direct_uses_1_4_step() {
        let radius = 6;
        let width = 12;
        let mut current = vec![0.0f32; (width * width) as usize];
        let start = (3, 3);
        let dest = (5, 5);
        assert!(create_direct_path(
            start,
            dest,
            width,
            radius,
            &open,
            &mut current,
            1.0
        ));
        // start=1, two diagonals → 1 + 1.4 + 1.4 = 3.8
        let d = current[idx(dest.0, dest.1, width)];
        assert!((d - 3.8).abs() < 0.05, "dest length {d}");
    }

    #[test]
    fn path_to_steps_deltas() {
        let path = [(0, 0), (1, 0), (1, 1)];
        assert_eq!(path_to_steps(&path), vec![(1, 0), (0, 1)]);
    }
}
