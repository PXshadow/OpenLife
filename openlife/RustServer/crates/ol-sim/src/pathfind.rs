//! Lightweight grid pathfinding for self-play / AI (Haxe Pathfinder + **PathfinderNew**).
//!
//! ## Gate / door walkability exception
//!
//! Content often marks gates and doors with `blocksWalking=1`. For pathfinding and
//! player movement probes we treat any object whose **name** contains `"gate"` or
//! `"door"` (case-insensitive) as **walkable** so AI and GOHOME/PATH can pass
//! through. Owned locks still restrict strangers via [`is_walkable_for_player`].
//!
//! Chat probes: `SAY PATH x y`, `SAY STEPS x y`, `SAY WALKABLE dx dy`.

use ol_ai_pathing::{
    create_path_with_budget, path_to_steps, PathBudget, PATHFINDER_NEW_DEFAULT_RADIUS,
};
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
    let owner_id = world.get_helper(x, y).map(|h| h.owner_id).unwrap_or(0);
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
///
/// Live AI Goto uses Haxe `PathfinderNew` (8-connected, chunk `RAD=32`).
/// [`find_path`] / [`path_steps`] stay 4-conn A* for `SAY STEPS` estimates.
// Haxe: AiHelper.GotoHelper PathfinderNew.CreatePath (AI-PATHFINDER-GOTO)
pub fn next_step(
    _world: &World,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
) -> Option<(i32, i32)> {
    next_step_new(sx, sy, gx, gy, walkable)
}

/// Haxe `PathfinderNew.CreatePath` on a local `2*radius` collision chunk around start.
///
/// Returns successive (dx, dy) steps (8-connected, diagonal 1.4). Dest outside
/// the chunk is unreachable (Haxe collision window). Live [`next_step`] uses this.
// Haxe: AiHelper.GotoHelper PathfinderNew.CreatePath (AI-PATHFINDER-GOTO)
pub fn find_path_new(
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
    radius: i32,
) -> Option<Vec<(i32, i32)>> {
    find_path_new_with_budget(sx, sy, gx, gy, walkable, radius, PathBudget::LIVE)
}

/// [`find_path_new`] with an explicit brute-force [`PathBudget`].
// Haxe: PathfinderNew.timeOut = 100 / CreatePathBruteForceInCircle
pub fn find_path_new_with_budget(
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
    radius: i32,
    budget: PathBudget,
) -> Option<Vec<(i32, i32)>> {
    let r = radius.max(1);
    let width = 2 * r;
    let start = (r, r);
    let dest = (gx - sx + r, gy - sy + r);
    if dest.0 < 0 || dest.1 < 0 || dest.0 >= width || dest.1 >= width {
        return None;
    }
    let local = |x: i32, y: i32| walkable(sx - r + x, sy - r + y);
    let path = create_path_with_budget(start, dest, r, &local, budget)?;
    Some(path_to_steps(&path))
}

/// First 8-connected step from [`find_path_new`], or None if blocked / already there.
pub fn next_step_new(
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    walkable: &dyn Fn(i32, i32) -> bool,
) -> Option<(i32, i32)> {
    find_path_new(sx, sy, gx, gy, walkable, PATHFINDER_NEW_DEFAULT_RADIUS)?
        .into_iter()
        .next()
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

// ── Animal collision chunk (AI-ANIMAL-GOTO / CreateCollisionChunkHelper) ────

/// Haxe `MapData.RAD` used by `CreateCollisionChunkHelper` (half-width of chunk).
// Haxe: MapData.RAD = 32 / AiHelper.CreateCollisionChunkHelper
pub const GOTO_COLLISION_RAD: i32 = PATHFINDER_NEW_DEFAULT_RADIUS;

/// Simplified Haxe `isAnimalDeadlyForMe` for path collision (no biome / hits / weapon).
///
/// Deadly for path when `isAnimal() && deadlyDistance > 0 && damage > 0`.
/// Same as [`is_deadly_animal_for_path_for_player`] with empty hands and no loved biome.
// Haxe: GPI.isAnimalDeadlyForMe ~6302; CreateCollisionChunkHelper ~1508
#[inline]
pub fn is_deadly_animal_for_path(def: &ol_content::ObjectDef) -> bool {
    is_deadly_animal_for_path_for_player(def, false, &[], 0.0, 0.0)
}

/// Haxe `isAnimalDeadlyForMe` for CreateCollisionChunk animal footprints.
///
/// Loved-biome animals with `hits < 0.1` and no weapon are not deadly (path may
/// walk their `moves` square). Weapon or `animal.hits > 0.5` restores deadly.
// Haxe: GPI.isAnimalDeadlyForMe L6302–6324; AiHelper.CreateCollisionChunkHelper ~1508
#[inline]
pub fn is_deadly_animal_for_path_for_player(
    def: &ol_content::ObjectDef,
    holding_weapon: bool,
    loved_biome_animal_ids: &'static [i32],
    player_tile_biome_love: f32,
    animal_tile_biome_love: f32,
) -> bool {
    crate::animal_damage::is_animal_deadly_for_me(crate::animal_damage::AnimalDeadlyForMeInput {
        deadly_distance: def.deadly_distance,
        damage: def.damage,
        is_animal: def.is_animal(),
        check_if_animal: true,
        animal_hits: 0.0,
        holding_weapon,
        animal_parent_id: def.id,
        loved_biome_animal_ids,
        player_tile_biome_love,
        animal_tile_biome_love,
    })
}

/// Player context for animal-footprint path collision (loved biome + weapon).
// Haxe: CreateCollisionChunkHelper player.isAnimalDeadlyForMe(obj)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalPathPlayerCtx {
    pub holding_weapon: bool,
    /// Person race color (`ContentDb.person_color`); 0 skips loved-biome filter.
    pub person_color: i32,
}

/// Haxe animal `moves` footprint: half-open square
/// `[ax - moves, ax + moves + 1) × [ay - moves, ay + moves + 1)`.
// Haxe: AiHelper.CreateCollisionChunkHelper ~1512–1530
#[inline]
pub fn animal_moves_covers_tile(
    animal_x: i32,
    animal_y: i32,
    moves: i32,
    tile_x: i32,
    tile_y: i32,
) -> bool {
    let m = moves.max(0);
    tile_x >= animal_x - m
        && tile_x < animal_x + m + 1
        && tile_y >= animal_y - m
        && tile_y < animal_y + m + 1
}

/// Collect tiles blocked by deadly-animal `moves` footprints in `[min, max)`.
// Haxe: AiHelper.CreateCollisionChunkHelper considerAnimal branch
pub fn collect_deadly_animal_blocked_tiles(
    world: &World,
    content: &ContentDb,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> HashSet<(i32, i32)> {
    collect_deadly_animal_blocked_tiles_for_player(
        world, content, min_x, min_y, max_x, max_y, None, min_x, min_y,
    )
}

/// Same as [`collect_deadly_animal_blocked_tiles`] with Haxe `isAnimalDeadlyForMe`.
// Haxe: CreateCollisionChunkHelper ~1508 player.isAnimalDeadlyForMe
pub fn collect_deadly_animal_blocked_tiles_for_player(
    world: &World,
    content: &ContentDb,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    player: Option<AnimalPathPlayerCtx>,
    player_x: i32,
    player_y: i32,
) -> HashSet<(i32, i32)> {
    let loved_ids: &'static [i32] = if let Some(ctx) = player {
        if ctx.person_color != 0 {
            let loved = crate::yum::loved_biome_for_person_color(ctx.person_color).unwrap_or(-1);
            crate::animal_damage::biome_animals_for_loved_biome(loved)
        } else {
            &[]
        }
    } else {
        &[]
    };
    let holding_weapon = player.map(|c| c.holding_weapon).unwrap_or(false);
    let player_love = if let Some(ctx) = player {
        if ctx.person_color != 0 {
            let biome = i32::from(world.get_biome(player_x, player_y));
            let floor = i32::from(world.get_floor(player_x, player_y));
            crate::biome_love_factor(biome, floor, ctx.person_color, None, None)
        } else {
            0.0
        }
    } else {
        0.0
    };
    let mut blocked = HashSet::new();
    for y in min_y..max_y {
        for x in min_x..max_x {
            let id = world.get_object(x, y);
            if id == 0 {
                continue;
            }
            let Some(def) = content.get(id) else {
                continue;
            };
            let animal_love = if let Some(ctx) = player {
                if ctx.person_color != 0 {
                    let biome = i32::from(world.get_biome(x, y));
                    let floor = i32::from(world.get_floor(x, y));
                    crate::biome_love_factor(biome, floor, ctx.person_color, None, None)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            if !is_deadly_animal_for_path_for_player(
                def,
                holding_weapon,
                loved_ids,
                player_love,
                animal_love,
            ) {
                continue;
            }
            let moves = def.moves;
            let min_yy = (y - moves).max(min_y);
            let min_xx = (x - moves).max(min_x);
            let max_yy = (y + moves + 1).min(max_y);
            let max_xx = (x + moves + 1).min(max_x);
            for yy in min_yy..max_yy {
                for xx in min_xx..max_xx {
                    blocked.insert((xx, yy));
                }
            }
        }
    }
    blocked
}

/// Animal-blocked set centered on player (Haxe collision chunk around `tx,ty`).
// Haxe: CreateCollisionChunkHelper minX/Y = player.tx/ty − RAD
#[inline]
pub fn collect_deadly_animal_blocked_around(
    world: &World,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    rad: i32,
) -> HashSet<(i32, i32)> {
    collect_deadly_animal_blocked_around_for_player(world, content, cx, cy, rad, None)
}

/// Animal-blocked set with Haxe `isAnimalDeadlyForMe` (weapon + loved biome).
// Haxe: CreateCollisionChunkHelper ~1508
#[inline]
pub fn collect_deadly_animal_blocked_around_for_player(
    world: &World,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    rad: i32,
    player: Option<AnimalPathPlayerCtx>,
) -> HashSet<(i32, i32)> {
    let r = rad.max(1);
    collect_deadly_animal_blocked_tiles_for_player(
        world,
        content,
        cx - r,
        cy - r,
        cx + r,
        cy + r,
        player,
        cx,
        cy,
    )
}

/// Base walkability plus optional deadly-animal footprint set.
// Haxe: MapCollision from CreateCollisionChunk (terrain + animals)
pub fn is_walkable_with_animals(
    world: &World,
    content: &ContentDb,
    x: i32,
    y: i32,
    animal_blocked: Option<&HashSet<(i32, i32)>>,
) -> bool {
    if !is_walkable(world, content, x, y) {
        return false;
    }
    if let Some(ab) = animal_blocked {
        if ab.contains(&(x, y)) {
            return false;
        }
    }
    true
}

/// Dual-pass Goto path classification (Haxe `gotoAdv`).
// Haxe: AiHelper.gotoAdv ~1116–1141
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GotoPathOutcome {
    /// Path exists under current animal consideration.
    Reachable,
    /// Failed with animals but succeeds without → mark `hostile_path`.
    BlockedByAnimal,
    /// Failed with and without animals (or animals not considered) → `not_reachable`.
    NotReachable,
}

/// Probe path with optional animal footprints; on fail dual-pass recheck without animals.
// Haxe: AiHelper.gotoAdv Goto(considerAnimals) then Goto(false, move=false)
pub fn goto_path_outcome(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    consider_animals: bool,
) -> GotoPathOutcome {
    let animal_blocked = if consider_animals {
        Some(collect_deadly_animal_blocked_around(
            world,
            content,
            sx,
            sy,
            GOTO_COLLISION_RAD,
        ))
    } else {
        None
    };
    let ab = animal_blocked.as_ref();
    let ok_with = find_path_new(
        sx,
        sy,
        gx,
        gy,
        &|x, y| is_walkable_with_animals(world, content, x, y, ab),
        PATHFINDER_NEW_DEFAULT_RADIUS,
    )
    .is_some();
    if ok_with {
        return GotoPathOutcome::Reachable;
    }
    if !consider_animals {
        return GotoPathOutcome::NotReachable;
    }
    // Dual-pass: animals off (Haxe move=false recheck).
    let ok_without = find_path_new(
        sx,
        sy,
        gx,
        gy,
        &|x, y| is_walkable(world, content, x, y),
        PATHFINDER_NEW_DEFAULT_RADIUS,
    )
    .is_some();
    if ok_without {
        GotoPathOutcome::BlockedByAnimal
    } else {
        GotoPathOutcome::NotReachable
    }
}

/// First step toward goal, inflating deadly-animal footprints when `consider_animals`.
// Haxe: GotoHelper CreateCollisionChunk(considerAnimal) + path first step
pub fn next_step_consider_animals(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    consider_animals: bool,
) -> Option<(i32, i32)> {
    next_step_consider_animals_for_player(world, content, sx, sy, gx, gy, consider_animals, None)
}

/// Same as [`next_step_consider_animals`] with Haxe `isAnimalDeadlyForMe` player context.
// Haxe: CreateCollisionChunkHelper ~1508
pub fn next_step_consider_animals_for_player(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    consider_animals: bool,
    player: Option<AnimalPathPlayerCtx>,
) -> Option<(i32, i32)> {
    let animal_blocked = if consider_animals {
        Some(collect_deadly_animal_blocked_around_for_player(
            world,
            content,
            sx,
            sy,
            GOTO_COLLISION_RAD,
            player,
        ))
    } else {
        None
    };
    let ab = animal_blocked.as_ref();
    next_step(world, sx, sy, gx, gy, &|x, y| {
        is_walkable_with_animals(world, content, x, y, ab)
    })
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
            prestige_factor: 0.5,
            extra_prestige_factor: 0.0,
            min_pickup_age: 0,
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
        let path = find_path(&w, 0, 0, 2, 0, &|x, y| is_walkable(&w, &db, x, y), 500).unwrap();
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
        // Completely seal start — every cardinal neighbor blocked (A* 4-conn).
        w.set_object(1, 0, 999);
        w.set_object(-1, 0, 999);
        w.set_object(0, 1, 999);
        w.set_object(0, -1, 999);
        assert_eq!(
            path_steps(&w, 0, 0, 5, 5, &|x, y| w.get_object(x, y) != 999),
            None
        );
        // PathfinderNew is 8-conn: also seal diagonals for live next_step.
        w.set_object(1, 1, 999);
        w.set_object(1, -1, 999);
        w.set_object(-1, 1, 999);
        w.set_object(-1, -1, 999);
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
        assert!(is_walkable_for_player(&w, &db, 1, 0, 7, &|_, _| false));
        // Stranger cannot.
        assert!(!is_walkable_for_player(&w, &db, 1, 0, 3, &|_, _| false));
        // Ally can.
        assert!(is_walkable_for_player(&w, &db, 1, 0, 3, &|a, b| (a == 3
            && b == 7)
            || (a == 7 && b == 3)));
        // Unowned gate still walkable for anyone.
        w.set_object(2, 0, 20);
        assert!(is_walkable_for_player(&w, &db, 2, 0, 3, &|_, _| false));
    }

    fn animal_def(id: i32, name: &str, moves: i32, deadly: f32, damage: f32) -> ObjectDef {
        let mut d = def(id, name, false);
        d.moves = moves;
        d.deadly_distance = deadly;
        d.damage = damage;
        d
    }

    // Haxe: CreateCollisionChunkHelper animal moves footprint
    #[test]
    fn animal_moves_footprint_and_deadly_gate() {
        let mut wolf = animal_def(418, "Wolf", 2, 0.5, 3.0);
        assert!(is_deadly_animal_for_path(&wolf));
        assert!(animal_moves_covers_tile(5, 5, 2, 5, 5));
        assert!(animal_moves_covers_tile(5, 5, 2, 3, 3)); // 5-2
        assert!(animal_moves_covers_tile(5, 5, 2, 7, 7)); // 5+2 inclusive via half-open max=8
        assert!(!animal_moves_covers_tile(5, 5, 2, 8, 5)); // 5+2+1 = 8 excluded
        assert!(!animal_moves_covers_tile(5, 5, 2, 2, 5)); // 5-2-1

        wolf.damage = 0.0;
        assert!(!is_deadly_animal_for_path(&wolf));
        let rabbit = animal_def(132, "Rabbit", 1, 0.0, 0.0);
        assert!(!is_deadly_animal_for_path(&rabbit));
    }

    #[test]
    fn loved_biome_animal_not_deadly_for_path_unless_weapon() {
        // Haxe: isAnimalDeadlyForMe loved biome + hits<0.1 + no weapon → not deadly
        // Wolf 418 is GREY biome animal (person white).
        let wolf = animal_def(418, "Wolf", 2, 0.5, 3.0);
        let loved = crate::animal_damage::biome_animals_for_loved_biome(3);
        assert!(loved.contains(&418));
        assert!(!is_deadly_animal_for_path_for_player(
            &wolf, false, loved, 1.0, 1.0
        ));
        assert!(is_deadly_animal_for_path_for_player(
            &wolf, true, loved, 1.0, 1.0
        ));
        assert!(is_deadly_animal_for_path_for_player(
            &wolf, false, &[], 0.0, 0.0
        ));
    }

    #[test]
    fn collect_animal_blocked_inflates_moves_radius() {
        let mut w = World::new(32, 32, false);
        let mut db = ContentDb::default();
        db.objects.insert(418, animal_def(418, "Wolf", 2, 0.5, 3.0));
        w.set_object(10, 10, 418);
        let blocked = collect_deadly_animal_blocked_tiles(&w, &db, 0, 0, 32, 32);
        assert!(blocked.contains(&(10, 10)));
        assert!(blocked.contains(&(8, 8)));
        assert!(blocked.contains(&(12, 12)));
        assert!(!blocked.contains(&(13, 10))); // outside moves=2 half-open
        assert!(!blocked.contains(&(0, 0)));
    }

    /// Corridor sealed only by wolf footprint → dual-pass BlockedByAnimal.
    // Haxe: gotoAdv animals-on fail + animals-off success → addHostilePath
    #[test]
    fn goto_dual_pass_animal_only_block_hostile() {
        let mut w = World::new(40, 40, false);
        let mut db = ContentDb::default();
        db.objects.insert(1, def(1, "Wall", true));
        // moves=0 still is_animal false; use moves=1 so footprint is the tile only
        db.objects.insert(418, animal_def(418, "Wolf", 0, 0.5, 3.0));
        // Force animal via moves>0 (is_animal requires moves>0)
        if let Some(d) = db.objects.get_mut(&418) {
            d.moves = 1;
        }
        // Seal everything except a 1-tile-wide corridor on y=5 from x=0..10
        for y in 0..20 {
            for x in 0..20 {
                if y != 5 || !(0..=10).contains(&x) {
                    w.set_object(x, y, 1);
                }
            }
        }
        // Wolf on corridor midpoint — footprint covers (4..6, 4..6) but walls already seal;
        // on-corridor tiles (4,5)(5,5)(6,5) become animal-blocked.
        w.set_object(5, 5, 418);
        let out = goto_path_outcome(&w, &db, 0, 5, 10, 5, true);
        assert_eq!(out, GotoPathOutcome::BlockedByAnimal);
        // Without considerAnimals: wolf tile is not blocks_walking → path ok
        let out2 = goto_path_outcome(&w, &db, 0, 5, 10, 5, false);
        assert_eq!(out2, GotoPathOutcome::Reachable);
    }

    /// Solid wall seal → NotReachable (no animal dual-pass win).
    #[test]
    fn goto_solid_wall_not_reachable() {
        let mut w = World::new(20, 20, false);
        let mut db = ContentDb::default();
        db.objects.insert(1, def(1, "Wall", true));
        // Seal start completely (8-conn PathfinderNew)
        w.set_object(1, 0, 1);
        w.set_object(-1, 0, 1);
        w.set_object(0, 1, 1);
        w.set_object(0, -1, 1);
        w.set_object(1, 1, 1);
        w.set_object(1, -1, 1);
        w.set_object(-1, 1, 1);
        w.set_object(-1, -1, 1);
        assert_eq!(
            goto_path_outcome(&w, &db, 0, 0, 5, 5, true),
            GotoPathOutcome::NotReachable
        );
        assert_eq!(
            goto_path_outcome(&w, &db, 0, 0, 5, 5, false),
            GotoPathOutcome::NotReachable
        );
    }

    #[test]
    fn next_step_consider_animals_avoids_wolf_footprint() {
        let mut w = World::new(40, 40, false);
        let mut db = ContentDb::default();
        db.objects.insert(1, def(1, "Wall", true));
        db.objects.insert(418, animal_def(418, "Wolf", 1, 0.5, 3.0));
        for y in 0..20 {
            for x in 0..20 {
                if y != 5 || !(0..=10).contains(&x) {
                    w.set_object(x, y, 1);
                }
            }
        }
        w.set_object(5, 5, 418);
        // With animals: corridor sealed by wolf footprint
        assert!(next_step_consider_animals(&w, &db, 0, 5, 10, 5, true).is_none());
        // Without animals: can step toward goal along corridor
        assert!(next_step_consider_animals(&w, &db, 0, 5, 10, 5, false).is_some());
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
            let path =
                find_path(&w, 0, 0, 49, 49, &walkable, 10_000).expect("open map should pathfind");
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

    /// Live `next_step` is PathfinderNew (diagonal allowed).
    // Haxe: GotoHelper PathfinderNew 8-conn
    #[test]
    fn next_step_uses_pathfinder_new_diagonal() {
        let w = World::new(16, 16, false);
        let walkable = |_x: i32, _y: i32| true;
        assert_eq!(next_step(&w, 0, 0, 5, 0, &walkable), Some((1, 0)));
        assert_eq!(next_step(&w, 0, 0, 3, 3, &walkable), Some((1, 1)));
    }

    /// PathfinderNew 8-connected first step on open field (AI-PATHFINDER-NEW).
    #[test]
    fn next_step_new_open_field() {
        let walkable = |_x: i32, _y: i32| true;
        assert_eq!(next_step_new(0, 0, 5, 0, &walkable), Some((1, 0)));
        assert_eq!(next_step_new(0, 0, 3, 3, &walkable), Some((1, 1)));
        assert!(next_step_new(0, 0, 0, 0, &walkable).is_none());
        let steps = find_path_new(0, 0, 4, 0, &walkable, 8).expect("path");
        let (mut x, mut y) = (0, 0);
        for (dx, dy) in &steps {
            x += dx;
            y += dy;
        }
        assert_eq!((x, y), (4, 0));
    }

    #[test]
    fn find_path_new_detour_around_wall() {
        let walkable = |x: i32, y: i32| !(x == 2 && y != 4);
        let steps = find_path_new(0, 0, 4, 0, &walkable, 8).expect("detour");
        let (mut x, mut y) = (0, 0);
        for (dx, dy) in &steps {
            x += dx;
            y += dy;
            assert!(
                !(x == 2 && y != 4),
                "stepped into wall at {x},{y} steps={steps:?}"
            );
        }
        assert_eq!((x, y), (4, 0));
    }

    #[test]
    fn find_path_new_zero_budget_times_out() {
        let walkable = |x: i32, y: i32| !(x == 2 && y != 4);
        assert!(
            find_path_new_with_budget(0, 0, 4, 0, &walkable, 8, PathBudget::expansions(0))
                .is_none()
        );
        assert!(
            find_path_new_with_budget(0, 0, 4, 0, &walkable, 8, PathBudget::millis(0)).is_none()
        );
        assert!(find_path_new(0, 0, 4, 0, &walkable, 8).is_some());
    }
}
