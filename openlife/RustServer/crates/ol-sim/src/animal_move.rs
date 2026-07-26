//! Haxe `TimeHelper.doAnimalMovement` / `CalculateNonBlockedTarget` / `CanAnimalEndUpHere`.
//!
//! Optimized single-pass port: same rules, fewer allocations.

use ol_content::ContentDb;
use ol_world::{is_biome_blocking as world_biome_blocking, World};
use rand::Rng;

/// Haxe `ServerSettings.ChanceThatAnimalsCanPassBlockingBiome` (0.03).
pub const CHANCE_PASS_BLOCKING_BIOME: f32 = 0.03;

/// Name substrings that **do not** block animal pathing even when `blocksWalking=1`
/// (Haxe `CalculateNonBlockedTarget` exemptions).
const PASSABLE_NAME_FRAGMENTS: &[&str] = &[
    "Tarry Spot",
    "Tree",
    "Rabbit",
    "Spring",
    "Sugarcane",
    "Pond",
    "Palm",
    "Plant",
    "Iron",
];

/// Haxe `isBiomeBlocking` + floor exception.
#[inline]
pub fn is_biome_blocking(world: &World, x: i32, y: i32) -> bool {
    let biome = world.get_biome(x, y);
    let floor = world.get_floor(x, y) as i32;
    world_biome_blocking(biome, floor)
}

/// True if this tile's object blocks animal **pathing** through it
/// (Haxe movement-tile check with tree/plant exemptions).
pub fn object_blocks_animal_path(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    let Some(def) = content.get(object_id) else {
        return false;
    };
    if !def.blocks_walking {
        return false;
    }
    let desc = if def.description.is_empty() {
        def.name.as_str()
    } else {
        def.description.as_str()
    };
    // Case-sensitive match like Haxe `indexOf` on description.
    for frag in PASSABLE_NAME_FRAGMENTS {
        if desc.contains(frag) || def.name.contains(frag) {
            return false;
        }
    }
    true
}

/// Haxe `CanAnimalEndUpHere` (end tile must not block walking; prefer non-movers).
pub fn can_animal_end_up_here(content: &ContentDb, object_id: i32, rabbit_empty_only: bool) -> bool {
    if rabbit_empty_only && object_id != 0 {
        return false;
    }
    if object_id == 0 {
        return true;
    }
    let Some(def) = content.get(object_id) else {
        return true;
    };
    if def.blocks_walking {
        return false;
    }
    // Don't land on other animals (moving objects). Our animal object ids:
    // checked by caller via empty/non-animal preference; base: no blocksWalking.
    true
}

/// Haxe `CalculateNonBlockedTarget`: walk one step at a time from `(from_x,from_y)`
/// toward `(to_x,to_y)`, stop before blocked tiles; return last valid end tile.
///
/// Returns `None` if movement fully blocked immediately.
pub fn calculate_non_blocked_target<R: Rng>(
    world: &World,
    content: &ContentDb,
    rng: &mut R,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    rabbit_empty_only: bool,
) -> Option<(i32, i32)> {
    let mut tmp_x = from_x;
    let mut tmp_y = from_y;
    let mut last_ok: Option<(i32, i32)> = None;

    for _ in 0..10 {
        if tmp_x == to_x && tmp_y == to_y {
            break;
        }
        if to_x > tmp_x {
            tmp_x += 1;
        } else if to_x < tmp_x {
            tmp_x -= 1;
        }
        if to_y > tmp_y {
            tmp_y += 1;
        } else if to_y < tmp_y {
            tmp_y -= 1;
        }

        let mut biome_block = is_biome_blocking(world, tmp_x, tmp_y);
        if biome_block && CHANCE_PASS_BLOCKING_BIOME > 0.0 {
            // Haxe: isBiomeBlocking = randomFloat() > chance  → often still blocked
            biome_block = rng.gen::<f32>() > CHANCE_PASS_BLOCKING_BIOME;
        }

        let obj = world.get_object(tmp_x, tmp_y);
        let path_block = object_blocks_animal_path(content, obj);

        if biome_block || path_block {
            break;
        }

        if can_animal_end_up_here(content, obj, rabbit_empty_only) {
            last_ok = Some((tmp_x, tmp_y));
        }
    }

    last_ok.filter(|&(x, y)| !(x == from_x && y == from_y))
}

/// Pick a destination like Haxe doAnimalMovement target loop + path trim.
///
/// `move_dist`: already Haxe-adjusted radius (`if move < 3 then move+1`).
/// `empty_preferred`: first half of attempts require empty tile object==0.
pub fn pick_animal_destination<R: Rng>(
    world: &World,
    content: &ContentDb,
    rng: &mut R,
    from_x: i32,
    from_y: i32,
    world_w: i32,
    world_h: i32,
    move_dist: i32,
    rabbit_empty_only: bool,
) -> Option<(i32, i32)> {
    let move_dist = move_dist.clamp(1, 6);
    const MAX_ITERS: i32 = 20;

    for i in 0..MAX_ITERS {
        let to_x = from_x - move_dist + rng.gen_range(0..=move_dist * 2);
        let to_y = from_y - move_dist + rng.gen_range(0..=move_dist * 2);
        if to_x == from_x && to_y == from_y {
            continue;
        }
        if to_x < 0 || to_y < 0 || to_x >= world_w || to_y >= world_h {
            continue;
        }
        if is_biome_blocking(world, to_x, to_y)
            && rng.gen::<f32>() > CHANCE_PASS_BLOCKING_BIOME
        {
            continue;
        }

        let dest_obj = world.get_object(to_x, to_y);
        // Prefer empty tiles in first half of iterations (Haxe).
        if dest_obj != 0 && i < MAX_ITERS / 2 {
            continue;
        }
        if !can_animal_end_up_here(content, dest_obj, rabbit_empty_only) {
            continue;
        }

        // Path from start to candidate, stop at blockers (trees exempt, etc.).
        if let Some(p) = calculate_non_blocked_target(
            world,
            content,
            rng,
            from_x,
            from_y,
            to_x,
            to_y,
            rabbit_empty_only,
        ) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef};
    use ol_world::World;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn content_with_tree() -> ContentDb {
        let mut db = ContentDb::default();
        let mut tree = ObjectDef::empty(100);
        tree.name = "Maple Tree".into();
        tree.description = "Maple Tree".into();
        tree.blocks_walking = true;
        db.objects.insert(100, tree);
        let mut wall = ObjectDef::empty(200);
        wall.name = "Stone Wall".into();
        wall.description = "Stone Wall".into();
        wall.blocks_walking = true;
        db.objects.insert(200, wall);
        db
    }

    #[test]
    fn trees_do_not_block_animal_path() {
        let db = content_with_tree();
        assert!(!object_blocks_animal_path(&db, 100));
        assert!(object_blocks_animal_path(&db, 200));
    }

    #[test]
    fn ocean_biome_blocks() {
        let mut w = World::new(8, 8, false);
        w.set_biome(3, 3, ol_world::OCEAN);
        assert!(is_biome_blocking(&w, 3, 3));
        w.set_biome(4, 4, ol_world::GREEN);
        assert!(!is_biome_blocking(&w, 4, 4));
    }

    #[test]
    fn path_stops_at_wall_not_tree() {
        let db = content_with_tree();
        let mut w = World::new(10, 10, false);
        w.set_object(2, 0, 100);
        w.set_object(4, 0, 200);
        let mut rng = StdRng::seed_from_u64(1);
        let p = calculate_non_blocked_target(&w, &db, &mut rng, 0, 0, 5, 0, false);
        assert!(p.is_some());
        let (x, _) = p.unwrap();
        assert!(x < 4, "must not pass wall, got x={x}");
    }
}
