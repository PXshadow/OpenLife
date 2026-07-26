//! Lightweight grid pathfinding for self-play / AI (Haxe Pathfinder subset).
//!
//! ## Gate / door walkability exception
//!
//! Content often marks gates and doors with `blocksWalking=1`. For pathfinding and
//! player movement probes we treat any object whose **name** contains `"gate"` or
//! `"door"` (case-insensitive) as **walkable** so AI and GOHOME/PATH can pass
//! through. Owned locks still restrict strangers via [`is_walkable_for_player`].
//!
//! Chat probes: `SAY PATH x y`, `SAY STEPS x y`, `SAY WALKABLE dx dy`.

use ol_content::ContentDb;
use ol_world::World;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    f: i32,
    g: i32,
    x: i32,
    y: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn heuristic(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}

/// Whether pathfinding may step onto tile `(x, y)`.
///
/// Uses content `blocks_walking`. Object **names** containing `"gate"` or `"door"`
/// (case-insensitive) are always walkable so AI can pass through doors/gates even
/// when the content file sets `blocksWalking=1`.
///
/// Does **not** consider ownership locks — see [`is_walkable_for_player`].
pub fn is_walkable(world: &World, content: &ContentDb, x: i32, y: i32) -> bool {
    let id = world.get_object(x, y);
    if id == 0 {
        return true;
    }
    let Some(def) = content.get(id) else {
        return true;
    };
    if !def.blocks_walking {
        return true;
    }
    let name = def.name.to_ascii_lowercase();
    name.contains("gate") || name.contains("door")
}

/// True when object name looks like a gate or door (owned-lock eligible).
pub fn name_is_gate_or_door(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("gate") || n.contains("door")
}

/// Walkability for a specific player, including owned-door lock stub.
///
/// Base rules match [`is_walkable`]. Additionally, if the tile is gate/door-like
/// and has `owner_id != 0`, only the owner or an ally (via `is_ally(player, owner)`)
/// may step on it. Unowned gates remain walkable.
pub fn is_walkable_for_player(
    world: &World,
    content: &ContentDb,
    x: i32,
    y: i32,
    player_id: i32,
    is_ally: &dyn Fn(i32, i32) -> bool,
) -> bool {
    if !is_walkable(world, content, x, y) {
        return false;
    }
    let id = world.get_object(x, y);
    if id == 0 {
        return true;
    }
    let Some(def) = content.get(id) else {
        return true;
    };
    if !name_is_gate_or_door(&def.name) {
        return true;
    }
    let owner_id = world
        .get_helper(x, y)
        .map(|h| h.owner_id)
        .unwrap_or(0);
    if owner_id == 0 || owner_id == player_id {
        return true;
    }
    is_ally(player_id, owner_id)
}

/// A* on 4-connected grid. Returns step deltas (dx,dy) sequence, max `limit` nodes.
///
/// `walkable(x, y)` should return true when the agent may step onto that tile
/// (see [`is_walkable`]).
pub fn find_path(
    world: &World,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
    limit: usize,
) -> Option<Vec<(i32, i32)>> {
    if sx == gx && sy == gy {
        return Some(vec![]);
    }
    let mut open = BinaryHeap::new();
    open.push(Node {
        f: heuristic(sx, sy, gx, gy),
        g: 0,
        x: sx,
        y: sy,
    });
    let mut came: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut gscore: HashMap<(i32, i32), i32> = HashMap::new();
    gscore.insert((sx, sy), 0);
    let mut closed = HashSet::new();
    let mut expanded = 0usize;

    while let Some(Node { g, x, y, .. }) = open.pop() {
        if x == gx && y == gy {
            // reconstruct
            let mut path = Vec::new();
            let mut cur = (gx, gy);
            while cur != (sx, sy) {
                let prev = *came.get(&cur)?;
                path.push((cur.0 - prev.0, cur.1 - prev.1));
                cur = prev;
            }
            path.reverse();
            return Some(path);
        }
        if !closed.insert((x, y)) {
            continue;
        }
        expanded += 1;
        if expanded > limit {
            break;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x + dx;
            let ny = y + dy;
            let (nx, ny) = world.wrap_tile(nx, ny);
            // Start tile is always expandable; other tiles must be walkable.
            if (nx, ny) != (sx, sy) && !walkable(nx, ny) {
                continue;
            }
            let tg = g + 1;
            if gscore.get(&(nx, ny)).copied().unwrap_or(i32::MAX) <= tg {
                continue;
            }
            came.insert((nx, ny), (x, y));
            gscore.insert((nx, ny), tg);
            open.push(Node {
                f: tg + heuristic(nx, ny, gx, gy),
                g: tg,
                x: nx,
                y: ny,
            });
        }
    }
    None
}

/// First step toward goal, or None if blocked/unreachable / already there.
pub fn next_step(
    world: &World,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
) -> Option<(i32, i32)> {
    let path = find_path(world, sx, sy, gx, gy, walkable, 2000)?;
    path.into_iter().next()
}

/// Number of 4-connected steps in the A* path, or `None` if unreachable.
///
/// At the goal returns `Some(0)`. Uses the same expand limit as [`next_step`].
pub fn path_steps(
    world: &World,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
) -> Option<usize> {
    find_path(world, sx, sy, gx, gy, walkable, 2000).map(|p| p.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef};
    use ol_world::World;

    fn def(id: i32, name: &str, blocks: bool) -> ObjectDef {
        ObjectDef {
            id,
            description: name.into(),
            name: name.into(),
            containable: false,
            permanent: true,
            blocks_walking: blocks,
            food_value: 0,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: vec![],
            num_uses: 0,
            num_slots: 0,
            floor: false,
        dummy_ids: Vec::new(),
        }
    }

    #[test]
    fn path_around_block() {
        let mut w = World::new(20, 20, false);
        // Wall between (0,0) and (2,0)
        w.set_object(1, 0, 999);
        let path = find_path(&w, 0, 0, 2, 0, &|x, y| w.get_object(x, y) != 999, 500).unwrap();
        assert!(!path.is_empty());
        // Should not step into (1,0)
        let mut x = 0;
        let mut y = 0;
        for (dx, dy) in &path {
            x += dx;
            y += dy;
            assert_ne!((x, y), (1, 0));
        }
        assert_eq!((x, y), (2, 0));
    }

    #[test]
    fn is_walkable_respects_blocks_walking() {
        let mut w = World::new(8, 8, false);
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, "Tree", true));
        db.objects.insert(11, def(11, "Berry", false));
        w.set_object(1, 1, 10);
        w.set_object(2, 2, 11);
        assert!(!is_walkable(&w, &db, 1, 1));
        assert!(is_walkable(&w, &db, 2, 2));
        assert!(is_walkable(&w, &db, 0, 0)); // empty
    }

    #[test]
    fn is_walkable_allows_gate_and_door_by_name() {
        let mut w = World::new(8, 8, false);
        let mut db = ContentDb::default();
        db.objects.insert(20, def(20, "Vertical Gate", true));
        db.objects.insert(21, def(21, "Pine Door", true));
        db.objects.insert(22, def(22, "Stone Wall", true));
        w.set_object(1, 0, 20);
        w.set_object(2, 0, 21);
        w.set_object(3, 0, 22);
        assert!(is_walkable(&w, &db, 1, 0), "gate name should be walkable");
        assert!(is_walkable(&w, &db, 2, 0), "door name should be walkable");
        assert!(!is_walkable(&w, &db, 3, 0), "plain wall still blocks");
    }

    #[test]
    fn path_through_gate() {
        let mut w = World::new(20, 20, false);
        let mut db = ContentDb::default();
        db.objects.insert(50, def(50, "Open Gate", true));
        // Gate on the direct path (1,0); wall above so only gate corridor works if blocked.
        w.set_object(1, 0, 50);
        let path = find_path(
            &w,
            0,
            0,
            2,
            0,
            &|x, y| is_walkable(&w, &db, x, y),
            500,
        )
        .unwrap();
        let mut x = 0;
        let mut y = 0;
        let mut stepped_on_gate = false;
        for (dx, dy) in &path {
            x += dx;
            y += dy;
            if (x, y) == (1, 0) {
                stepped_on_gate = true;
            }
        }
        assert!(stepped_on_gate, "path should go through gate tile");
        assert_eq!((x, y), (2, 0));
    }

    #[test]
    fn path_steps_counts_and_fail_when_sealed() {
        let mut w = World::new(20, 20, false);
        // Open path: (0,0) -> (2,0) is 2 steps.
        assert_eq!(path_steps(&w, 0, 0, 2, 0, &|_, _| true), Some(2));
        assert_eq!(path_steps(&w, 0, 0, 0, 0, &|_, _| true), Some(0));
        // Completely seal start — every cardinal neighbor blocked.
        w.set_object(1, 0, 999);
        w.set_object(-1, 0, 999);
        w.set_object(0, 1, 999);
        w.set_object(0, -1, 999);
        assert_eq!(
            path_steps(&w, 0, 0, 5, 5, &|x, y| w.get_object(x, y) != 999),
            None
        );
        assert_eq!(
            next_step(&w, 0, 0, 5, 5, &|x, y| w.get_object(x, y) != 999),
            None
        );
    }

    #[test]
    fn owned_gate_blocks_stranger_not_owner_or_ally() {
        use ol_world::ComplexObject;
        let mut w = World::new(8, 8, false);
        let mut db = ContentDb::default();
        db.objects.insert(20, def(20, "Pine Door", true));
        w.set_object_complex(1, 0, ComplexObject::with_owner(20, 7));
        // Owner can pass.
        assert!(is_walkable_for_player(
            &w,
            &db,
            1,
            0,
            7,
            &|_, _| false
        ));
        // Stranger cannot.
        assert!(!is_walkable_for_player(
            &w,
            &db,
            1,
            0,
            3,
            &|_, _| false
        ));
        // Ally can.
        assert!(is_walkable_for_player(
            &w,
            &db,
            1,
            0,
            3,
            &|a, b| (a == 3 && b == 7) || (a == 7 && b == 3)
        ));
        // Unowned gate still walkable for anyone.
        w.set_object(2, 0, 20);
        assert!(is_walkable_for_player(
            &w,
            &db,
            2,
            0,
            3,
            &|_, _| false
        ));
    }

    /// Benchmark-style unit: A* corner-to-corner on empty 50×50 finishes quickly.
    #[test]
    fn pathfind_50x50_empty_is_fast() {
        use std::time::Instant;
        let w = World::new(50, 50, false);
        let walkable = |_x: i32, _y: i32| true;
        let t0 = Instant::now();
        // Run a few times so the bound is stable under debug builds / CI noise.
        let mut last_len = 0usize;
        for _ in 0..5 {
            let path = find_path(&w, 0, 0, 49, 49, &walkable, 10_000)
                .expect("open map should pathfind");
            last_len = path.len();
            // Manhattan path on empty grid is exactly 98 steps.
            assert_eq!(path.len(), 98);
            let mut x = 0i32;
            let mut y = 0i32;
            for (dx, dy) in &path {
                x += dx;
                y += dy;
            }
            assert_eq!((x, y), (49, 49));
        }
        let elapsed = t0.elapsed();
        // Generous for debug + cold caches; empty A* should be well under this.
        assert!(
            elapsed.as_millis() < 500,
            "50x50 empty pathfind x5 took {:?}, last_len={last_len} (budget 500ms)",
            elapsed
        );
    }
}
