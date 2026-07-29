//! Haxe `TimeHelper.doAnimalMovement` / `CalculateNonBlockedTarget` / `CanAnimalEndUpHere`
//! + chase / loved-biome steering (`TIME-ANIMAL-CHASE`).
//!
//! Optimized single-pass port: same rules, fewer allocations.

use crate::environment::Season;
use ol_content::ContentDb;
use ol_world::{is_biome_blocking as world_biome_blocking, World, GREEN};
use rand::Rng;

// Haxe BiomeTag ids (mirror ol_world::biome; not all re-exported at crate root).
const YELLOW: u8 = 2;
const GREY: u8 = 3;
const SNOW: u8 = 4;
const DESERT: u8 = 5;
const RIVER: u8 = 17;

/// Haxe `ServerSettings.ChanceThatAnimalsCanPassBlockingBiome` (0.03).
pub const CHANCE_PASS_BLOCKING_BIOME: f32 = 0.03;

/// Haxe `ServerSettings.chancePreferredBiome` (0.8).
pub const CHANCE_PREFERRED_BIOME: f32 = 0.8;

/// Pack-alert radius (Haxe `GetClosestObjectToPosition(..., 20, animal)`).
pub const PACK_ALERT_RANGE: i32 = 20;

/// Hits stamped on pack-mates when a deadly animal locks a player target.
pub const PACK_ALERT_HITS: f32 = 0.1;

// --- Object parent ids used by chase tables (Haxe doAnimalMovement) -----------

/// 418 Wolf
pub const PARENT_WOLF: i32 = 418;
/// 631 Hungry Grizzly Bear
pub const PARENT_GRIZZLY: i32 = 631;
/// 764 Rattle Snake
pub const PARENT_RATTLE_SNAKE: i32 = 764;
/// Shot / wounded variants that always chase.
pub const CHASING_ANIMAL_IDS: &[i32] = &[420, 1438, 632, 635, 637];
/// Wild Boar / Bison / snake: no seasonal chase (only when hits>0 or chasing table).
pub const ANIMALS_DONT_CHASE: &[i32] = &[1323, 1328, 1435, 1436, 764];

/// Haxe `ObjectData.IsBoneGrave` id table.
pub const BONE_GRAVE_IDS: &[i32] = &[
    87, 88, 89, 356, 357, 1920, 3051, 3052, 3195, 3196, 752,
];

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

/// Optional chase / loved-biome steering for [`pick_animal_destination`].
///
/// Haxe: `animal.target`, `lovedTx`/`lovedTy`, `gotoTarget`, `lovesCurrentBiome`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnimalSteer {
    /// Content object id used for [`is_spawning_in`] (parent / countsOrGrowsAs resolved).
    pub object_id: i32,
    /// Winter||SNOW bone-grave gate or deadly player lock.
    pub goto_target: bool,
    /// Chase or bone-grave tile; `None` → use loved coords when steering home.
    pub target: Option<(i32, i32)>,
    pub loved_tx: i32,
    pub loved_ty: i32,
    /// True when animal's spawn biomes include its current tile biome.
    pub loves_current_biome: bool,
}

/// Result of pre-move chase resolution (Haxe choose-targets block).
#[derive(Debug, Clone, Copy, Default)]
pub struct ChaseResolve {
    pub goto_target: bool,
    pub target: Option<(i32, i32)>,
    /// When set, stamp [`PACK_ALERT_HITS`] on that pack-mate if its hits ≤ 0.
    pub pack_alert_index: Option<usize>,
}

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

/// Haxe `ObjectData.IsBoneGrave`.
#[inline]
pub fn is_bone_grave(obj_id: i32) -> bool {
    BONE_GRAVE_IDS.contains(&obj_id)
}

/// Haxe `AiHelper.CalculateDistance` without world-wrap (local sim maps).
///
/// Returns squared Euclidean distance (quad-distance).
#[inline]
pub fn quad_distance(ax: i32, ay: i32, bx: i32, by: i32) -> f64 {
    let dx = (bx - ax) as f64;
    let dy = (by - ay) as f64;
    dx * dx + dy * dy
}

/// Haxe `ObjectData.isSpawningIn` — biomes list, via `countsOrGrowsAs` when set.
pub fn is_spawning_in(content: &ContentDb, object_id: i32, biome_id: u8) -> bool {
    let Some(def) = content.get(object_id) else {
        return false;
    };
    let lookup_id = if def.counts_or_grows_as != 0 {
        def.counts_or_grows_as
    } else {
        object_id
    };
    let Some(src) = content.get(lookup_id) else {
        // Fall back to this def's biomes if countsOrGrowsAs missing from DB.
        return def.biomes.iter().any(|&b| b == biome_id as i32);
    };
    src.biomes.iter().any(|&b| b == biome_id as i32)
}

/// Haxe hard-biome preferred-chance:
/// soft (preferred | GREEN | YELLOW) → `chancePreferredBiome`;
/// hard → `(chance + 4) / 5`.
pub fn chance_preferred_biome(is_not_hard_biome: bool) -> f32 {
    if is_not_hard_biome {
        CHANCE_PREFERRED_BIOME
    } else {
        (CHANCE_PREFERRED_BIOME + 4.0) / 5.0
    }
}

/// True when target biome is preferred, GREEN, or YELLOW (Haxe `isNotHardbiome`).
#[inline]
pub fn is_not_hard_biome(target_biome: u8, is_preferred: bool) -> bool {
    is_preferred || target_biome == GREEN || target_biome == YELLOW
}

/// Haxe deadly-animal chase gate: season / hits / tables → (should_try, chase_dist).
///
/// - Rattle snake (764): chase dist 5, loved season Summer (but in animalsDontChase)
/// - Default deadly: dist 20, loved season Winter
/// - animalsDontChase: no seasonal chase (`rightSeason = false`)
/// - chasingAnimals: always try
/// - `hits > 0` always tries
pub fn deadly_chase_gate(parent_id: i32, hits: f32, season: Season) -> (bool, i32) {
    let chase_distance = if parent_id == PARENT_RATTLE_SNAKE {
        5
    } else {
        20
    };
    let loved_season = if parent_id == PARENT_RATTLE_SNAKE {
        Season::Summer
    } else {
        Season::Winter
    };
    let mut right_season = season == loved_season;
    if ANIMALS_DONT_CHASE.contains(&parent_id) {
        right_season = false;
    }
    let should = hits > 0.0 || right_season || CHASING_ANIMAL_IDS.contains(&parent_id);
    (should, chase_distance)
}

/// Haxe `GlobalPlayerInstance.GetClosestPlayerAt` — min Euclidean quad-distance
/// among non-deleted players; max distance is inclusive via `dist² ≤ max²`.
///
/// `players` is `(x, y)` in world tiles. Returns closest position if any in range.
pub fn get_closest_player_at(
    from_x: i32,
    from_y: i32,
    max_distance: i32,
    players: &[(i32, i32)],
) -> Option<(i32, i32)> {
    if max_distance < 0 || players.is_empty() {
        return None;
    }
    let quad_max = (max_distance as f64) * (max_distance as f64);
    let mut best: Option<(f64, i32, i32)> = None;
    for &(px, py) in players {
        let q = quad_distance(from_x, from_y, px, py);
        if q > quad_max {
            continue;
        }
        match best {
            None => best = Some((q, px, py)),
            Some((bq, _, _)) if q < bq => best = Some((q, px, py)),
            // Haxe keeps last when equal (continues only if tmp > best); keep first.
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

/// Haxe `TimeHelper.GetClosestBoneGrave` over a grave position list.
pub fn get_closest_bone_grave(
    from_x: i32,
    from_y: i32,
    graves: &[(i32, i32)],
) -> Option<(i32, i32)> {
    let mut best: Option<(f64, i32, i32)> = None;
    for &(gx, gy) in graves {
        let q = quad_distance(from_x, from_y, gx, gy);
        match best {
            None => best = Some((q, gx, gy)),
            Some((bq, _, _)) if q <= bq => best = Some((q, gx, gy)),
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

/// Scan world for bone-grave object tiles (fallback when no cursedGraves map).
///
/// Limited to axis-aligned box around `(cx,cy)` with Chebyshev `radius` for cost.
pub fn collect_bone_graves_near(
    world: &World,
    cx: i32,
    cy: i32,
    radius: i32,
) -> Vec<(i32, i32)> {
    let ww = world.width_tiles;
    let wh = world.height_tiles;
    let r = radius.max(0);
    let x0 = (cx - r).max(0);
    let y0 = (cy - r).max(0);
    let x1 = (cx + r).min(ww - 1);
    let y1 = (cy + r).min(wh - 1);
    let mut out = Vec::new();
    if ww <= 0 || wh <= 0 {
        return out;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let id = world.get_object(x, y);
            if is_bone_grave(id) {
                out.push((x, y));
            }
        }
    }
    out
}

/// Closest other animal with same `kind_parent_id` within Chebyshev `range`
/// (Haxe pack alert uses parentId object search). Returns slice index.
pub fn closest_pack_mate_index(
    animals_xy_hits: &[(i32, i32, i32, f32)],
    self_index: usize,
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    range: i32,
) -> Option<usize> {
    let mut best: Option<(i32, usize)> = None;
    for (i, &(pid, x, y, _hits)) in animals_xy_hits.iter().enumerate() {
        if i == self_index || pid != parent_id {
            continue;
        }
        let d = (x - from_x).abs().max((y - from_y).abs());
        if d > range {
            continue;
        }
        match best {
            None => best = Some((d, i)),
            Some((bd, _)) if d < bd => best = Some((d, i)),
            _ => {}
        }
    }
    best.map(|(_, i)| i)
}

/// Haxe choose-targets block for one animal (bone grave + deadly player + pack alert).
///
/// `is_deadly` is Haxe `animal.isDeadlyAnimal()`.
/// `bone_graves` only needed for wolf/grizzly winter||snow.
/// `players` are live player tile positions.
/// `pack` is `(parent_id, x, y, hits)` parallel to animal indices.
pub fn resolve_animal_chase(
    parent_id: i32,
    is_deadly: bool,
    hits: f32,
    season: Season,
    current_biome: u8,
    ax: i32,
    ay: i32,
    players: &[(i32, i32)],
    bone_graves: &[(i32, i32)],
    pack: &[(i32, i32, i32, f32)],
    self_index: usize,
    existing_target: Option<(i32, i32)>,
    existing_target_still_bone_grave: bool,
) -> ChaseResolve {
    // Haxe: gotoTarget = Season == Winter || currentbiome == SNOW
    let mut goto_target = season == Season::Winter || current_biome == SNOW;
    let mut target = existing_target;

    // 418 Wolf // 631 Hungry Grizzly Bear → bone grave retarget
    if goto_target && (parent_id == PARENT_WOLF || parent_id == PARENT_GRIZZLY) {
        let need_new = match target {
            None => true,
            Some(_) => !existing_target_still_bone_grave,
        };
        if need_new {
            target = get_closest_bone_grave(ax, ay, bone_graves);
        }
    }

    let mut pack_alert_index = None;

    if is_deadly {
        let (should, chase_dist) = deadly_chase_gate(parent_id, hits, season);
        if should {
            if let Some(p) = get_closest_player_at(ax, ay, chase_dist, players) {
                target = Some(p);
                goto_target = true;
                // Alert closest same parent within 20
                if let Some(idx) =
                    closest_pack_mate_index(pack, self_index, parent_id, ax, ay, PACK_ALERT_RANGE)
                {
                    let mate_hits = pack[idx].3;
                    if mate_hits <= 0.0 {
                        pack_alert_index = Some(idx);
                    }
                }
            }
        }
    }

    ChaseResolve {
        goto_target,
        target,
        pack_alert_index,
    }
}

/// Update loved spawn coords when standing in a spawn biome (original biome in Haxe).
#[inline]
pub fn maybe_update_loved_biome(
    loves_current_original_biome: bool,
    tx: i32,
    ty: i32,
    loved_tx: &mut i32,
    loved_ty: &mut i32,
) {
    if loves_current_original_biome {
        *loved_tx = tx;
        *loved_ty = ty;
    }
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
/// When `steer` is `None`, behaves as pure random wander (legacy).
#[allow(dead_code)] // public API / tests; live tick uses steered variant
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
    pick_animal_destination_steered(
        world,
        content,
        rng,
        from_x,
        from_y,
        world_w,
        world_h,
        move_dist,
        rabbit_empty_only,
        None,
    )
}

/// Full Haxe doAnimalMovement candidate loop with preferred-biome bias and
/// gotoTarget / gotoLovedBiome best-quad-dist selection.
pub fn pick_animal_destination_steered<R: Rng>(
    world: &World,
    content: &ContentDb,
    rng: &mut R,
    from_x: i32,
    from_y: i32,
    world_w: i32,
    world_h: i32,
    move_dist: i32,
    rabbit_empty_only: bool,
    steer: Option<AnimalSteer>,
) -> Option<(i32, i32)> {
    let move_dist = move_dist.clamp(1, 6);
    let mut max_iterations: i32 = 20;
    let mut best_target: Option<(i32, i32)> = None;
    let mut best_quad_dist = f64::MAX;

    let steer = steer.unwrap_or(AnimalSteer {
        object_id: 0,
        goto_target: false,
        target: None,
        loved_tx: 0,
        loved_ty: 0,
        loves_current_biome: true,
    });

    let mut i: i32 = 0;
    while i < max_iterations {
        let to_x = from_x - move_dist + rng.gen_range(0..=move_dist * 2);
        let to_y = from_y - move_dist + rng.gen_range(0..=move_dist * 2);
        if to_x == from_x && to_y == from_y {
            i += 1;
            continue;
        }
        if to_x < 0 || to_y < 0 || to_x >= world_w || to_y >= world_h {
            i += 1;
            continue;
        }

        // Haxe skips fully blocking biomes on the *candidate* (no pass chance here).
        if is_biome_blocking(world, to_x, to_y) {
            i += 1;
            continue;
        }

        let dest_obj = world.get_object(to_x, to_y);
        if !can_animal_end_up_here(content, dest_obj, rabbit_empty_only) {
            i += 1;
            continue;
        }

        let target_biome = world.get_biome(to_x, to_y);
        let is_preferred = if steer.object_id != 0 {
            is_spawning_in(content, steer.object_id, target_biome)
        } else {
            false
        };
        let not_hard = is_not_hard_biome(target_biome, is_preferred);
        let chance_pref = chance_preferred_biome(not_hard);

        // Haxe: skip non-preferred first 5 tries with chancePreferredBiome
        if !is_preferred && i < 5 && rng.gen::<f32>() <= chance_pref {
            i += 1;
            continue;
        }

        // Path trim (Haxe CalculateNonBlockedTarget)
        let Some((px, py)) = calculate_non_blocked_target(
            world,
            content,
            rng,
            from_x,
            from_y,
            to_x,
            to_y,
            rabbit_empty_only,
        ) else {
            i += 1;
            continue;
        };

        // Prefer empty on path-end in first half of iterations
        let end_obj = world.get_object(px, py);
        if end_obj != 0 && i < max_iterations / 2 {
            i += 1;
            continue;
        }
        if !can_animal_end_up_here(content, end_obj, rabbit_empty_only) {
            i += 1;
            continue;
        }

        // Haxe gotoLovedBiome / gotoTarget best-quad selection
        let goto_loved_biome = !steer.goto_target
            && !steer.loves_current_biome
            && !is_preferred
            && (steer.loved_tx != 0 || steer.loved_ty != 0);

        if steer.goto_target || goto_loved_biome {
            let (goal_tx, goal_ty) = if goto_loved_biome || steer.target.is_none() {
                (steer.loved_tx, steer.loved_ty)
            } else {
                steer.target.unwrap()
            };
            let q = quad_distance(goal_tx, goal_ty, px, py);
            if q < best_quad_dist {
                best_quad_dist = q;
                best_target = Some((px, py));
                // try to find better: maxIterations = i + 6
                max_iterations = (i + 6).min(40);
            }
            if i < max_iterations - 2 {
                i += 1;
                continue;
            }
            if let Some(bt) = best_target {
                return Some(bt);
            }
        }

        // Non-steered (or final without best): take first valid path
        return Some((px, py));
    }

    best_target
}

/// Suppress unused biome constants if only used via is_not_hard / docs.
#[allow(dead_code)]
fn _biome_refs() -> [u8; 4] {
    [GREY, SNOW, DESERT, RIVER]
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

    fn content_wolf_snow() -> ContentDb {
        let mut db = ContentDb::default();
        let mut wolf = ObjectDef::empty(PARENT_WOLF);
        wolf.biomes = vec![SNOW as i32, GREY as i32];
        db.objects.insert(PARENT_WOLF, wolf);
        let mut boar = ObjectDef::empty(1323);
        boar.biomes = vec![GREEN as i32, YELLOW as i32];
        db.objects.insert(1323, boar);
        let mut rabbit = ObjectDef::empty(3566);
        rabbit.biomes = vec![GREEN as i32, YELLOW as i32];
        db.objects.insert(3566, rabbit);
        // countsOrGrowsAs proxy
        let mut pup = ObjectDef::empty(9999);
        pup.counts_or_grows_as = PARENT_WOLF;
        db.objects.insert(9999, pup);
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

    #[test]
    fn is_bone_grave_table() {
        assert!(is_bone_grave(87));
        assert!(is_bone_grave(357));
        assert!(is_bone_grave(752));
        assert!(!is_bone_grave(418));
        assert!(!is_bone_grave(0));
    }

    #[test]
    fn is_spawning_in_uses_biomes_and_counts_or_grows_as() {
        let db = content_wolf_snow();
        assert!(is_spawning_in(&db, PARENT_WOLF, SNOW));
        assert!(!is_spawning_in(&db, PARENT_WOLF, GREEN));
        // via countsOrGrowsAs → wolf biomes
        assert!(is_spawning_in(&db, 9999, SNOW));
        assert!(!is_spawning_in(&db, 9999, DESERT));
    }

    #[test]
    fn chance_preferred_hard_biome_boosted() {
        assert!((chance_preferred_biome(true) - 0.8).abs() < 1e-6);
        assert!((chance_preferred_biome(false) - 0.96).abs() < 1e-6);
    }

    #[test]
    fn get_closest_player_at_euclidean() {
        let players = [(10, 10), (30, 0), (5, 0)];
        // from (0,0) max 20: (5,0) dist 5, (10,10) dist ~14.1; (30,0) out
        let p = get_closest_player_at(0, 0, 20, &players);
        assert_eq!(p, Some((5, 0)));
        assert_eq!(get_closest_player_at(0, 0, 4, &players), None);
        // snake-style dist 5
        assert_eq!(get_closest_player_at(0, 0, 5, &[(6, 0)]), None);
        assert_eq!(get_closest_player_at(0, 0, 5, &[(5, 0)]), Some((5, 0)));
    }

    #[test]
    fn get_closest_bone_grave_min_quad() {
        let graves = [(100, 0), (3, 4), (50, 50)];
        assert_eq!(get_closest_bone_grave(0, 0, &graves), Some((3, 4)));
        assert_eq!(get_closest_bone_grave(0, 0, &[]), None);
    }

    #[test]
    fn deadly_chase_gate_wolf_winter_boar_hits() {
        // Wolf: winter season chase, dist 20
        let (ok, d) = deadly_chase_gate(PARENT_WOLF, 0.0, Season::Winter);
        assert!(ok);
        assert_eq!(d, 20);
        // Wolf summer no hits: no chase
        let (ok, _) = deadly_chase_gate(PARENT_WOLF, 0.0, Season::Summer);
        assert!(!ok);
        // hits > 0 any season
        let (ok, _) = deadly_chase_gate(PARENT_WOLF, 0.1, Season::Summer);
        assert!(ok);
        // Boar in animalsDontChase: winter alone not enough
        let (ok, _) = deadly_chase_gate(1323, 0.0, Season::Winter);
        assert!(!ok);
        // Boar when hits > 0
        let (ok, d) = deadly_chase_gate(1323, 1.0, Season::Winter);
        assert!(ok);
        assert_eq!(d, 20);
        // Shot wolf always chases
        let (ok, _) = deadly_chase_gate(420, 0.0, Season::Summer);
        assert!(ok);
        // Rattle snake: dist 5, dont-chase season
        let (ok, d) = deadly_chase_gate(PARENT_RATTLE_SNAKE, 0.0, Season::Summer);
        assert!(!ok);
        assert_eq!(d, 5);
    }

    #[test]
    fn resolve_wolf_winter_player_and_pack_alert() {
        let players = [(8, 0)];
        let pack = [
            (PARENT_WOLF, 0, 0, 0.0),  // self
            (PARENT_WOLF, 5, 0, 0.0),  // pack mate
            (PARENT_WOLF, 50, 0, 0.0), // far
        ];
        let r = resolve_animal_chase(
            PARENT_WOLF,
            true,
            0.0,
            Season::Winter,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert!(r.goto_target);
        assert_eq!(r.target, Some((8, 0)));
        assert_eq!(r.pack_alert_index, Some(1));
    }

    // COMBAT-MOSQUITO-KIND: is_deadly_animal=false → never acquires player target
    // (Haxe isDeadlyAnimal excludes 2156). Winter may still set goto_target for bone-grave
    // path on wolves; mosquito must not latch onto players even with hits.
    #[test]
    fn resolve_mosquito_never_chases_player_winter() {
        let players = [(3, 0)];
        // Mosquito Swarm parent 2156
        let pack = [(2156, 0, 0, 0.0)];
        let r = resolve_animal_chase(
            2156,
            false, // AnimalKind::Mosquito.is_deadly_animal()
            0.0,
            Season::Winter,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert!(r.target.is_none(), "mosquito must not set player target");
        assert!(r.pack_alert_index.is_none());
        // hits>0 still cannot chase when not deadly-animal
        let r2 = resolve_animal_chase(
            2156,
            false,
            1.0,
            Season::Winter,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert!(r2.target.is_none());
        // Contrast: same call with is_deadly=true would chase (wolf-style).
        let r3 = resolve_animal_chase(
            2156,
            true,
            1.0,
            Season::Winter,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert_eq!(r3.target, Some((3, 0)));
    }

    #[test]
    fn resolve_boar_no_season_chase_only_hits() {
        let players = [(5, 0)];
        let pack = [(1323, 0, 0, 0.0)];
        let r = resolve_animal_chase(
            1323,
            true,
            0.0,
            Season::Winter,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert!(!r.goto_target || r.target.is_none());
        // with hits
        let r = resolve_animal_chase(
            1323,
            true,
            0.5,
            Season::Summer,
            GREEN,
            0,
            0,
            &players,
            &[],
            &pack,
            0,
            None,
            false,
        );
        assert!(r.goto_target);
        assert_eq!(r.target, Some((5, 0)));
    }

    #[test]
    fn resolve_wolf_winter_bone_grave_when_no_player() {
        let graves = [(12, 0), (100, 0)];
        let pack = [(PARENT_WOLF, 0, 0, 0.0)];
        let r = resolve_animal_chase(
            PARENT_WOLF,
            true,
            0.0,
            Season::Winter,
            SNOW,
            0,
            0,
            &[], // no players
            &graves,
            &pack,
            0,
            None,
            false,
        );
        assert!(r.goto_target);
        assert_eq!(r.target, Some((12, 0)));
    }

    #[test]
    fn goto_target_picks_min_quad_dist_among_candidates() {
        let db = content_wolf_snow();
        let mut w = World::new(30, 30, false);
        // Open green map
        for y in 0..30 {
            for x in 0..30 {
                w.set_biome(x, y, GREEN);
            }
        }
        let steer = AnimalSteer {
            object_id: PARENT_WOLF,
            goto_target: true,
            target: Some((25, 15)),
            loved_tx: 0,
            loved_ty: 0,
            loves_current_biome: true,
        };
        // From (15,15) move_dist 3 — candidates in box; steered should bias east
        let mut closer_count = 0;
        let mut samples = 0;
        for seed in 0..40u64 {
            let mut s_rng = StdRng::seed_from_u64(seed);
            if let Some((px, py)) = pick_animal_destination_steered(
                &w,
                &db,
                &mut s_rng,
                15,
                15,
                30,
                30,
                3,
                false,
                Some(steer),
            ) {
                samples += 1;
                let q = quad_distance(25, 15, px, py);
                let q_origin = quad_distance(25, 15, 15, 15);
                if q < q_origin {
                    closer_count += 1;
                }
            }
        }
        assert!(samples > 10, "expected some destinations");
        assert!(
            closer_count > samples / 2,
            "steered dest should usually move toward target: closer={closer_count}/{samples}"
        );
    }

    #[test]
    fn goto_loved_biome_steers_toward_loved_tx_ty() {
        let db = content_wolf_snow();
        let mut w = World::new(24, 24, false);
        for y in 0..24 {
            for x in 0..24 {
                // Non-preferred desert so gotoLovedBiome engages
                w.set_biome(x, y, DESERT);
            }
        }
        let steer = AnimalSteer {
            object_id: PARENT_WOLF,
            goto_target: false,
            target: None,
            loved_tx: 20,
            loved_ty: 10,
            loves_current_biome: false,
        };
        let mut closer = 0;
        let mut n = 0;
        for seed in 0..50u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            if let Some((px, py)) = pick_animal_destination_steered(
                &w,
                &db,
                &mut rng,
                10,
                10,
                24,
                24,
                3,
                false,
                Some(steer),
            ) {
                n += 1;
                if quad_distance(20, 10, px, py) < quad_distance(20, 10, 10, 10) {
                    closer += 1;
                }
            }
        }
        assert!(n > 5);
        assert!(
            closer > n / 2,
            "loved-biome steer should move toward loved coords: {closer}/{n}"
        );
    }

    #[test]
    fn preferred_biome_skip_non_spawn_early_iterations() {
        // Soft check: with chance 0.8, non-preferred candidates often skipped early.
        // Use deterministic force: chance_preferred_biome path returns high for hard.
        assert!(chance_preferred_biome(false) > 0.9);
        assert!(is_not_hard_biome(GREEN, false));
        assert!(!is_not_hard_biome(DESERT, false));
        assert!(is_not_hard_biome(DESERT, true));
    }

    #[test]
    fn calculate_non_blocked_still_applies_when_chasing() {
        let db = content_with_tree();
        let mut w = World::new(12, 4, false);
        for y in 0..4 {
            for x in 0..12 {
                w.set_biome(x, y, GREEN);
            }
            // Full wall column so path cannot bypass
            w.set_object(4, y, 200);
        }
        let mut rng = StdRng::seed_from_u64(2);
        // Direct path trim toward far side of wall
        let p = calculate_non_blocked_target(&w, &db, &mut rng, 0, 0, 10, 0, false);
        assert!(p.is_some());
        let (x, _) = p.unwrap();
        assert!(x < 4, "path trim must stop before wall, got x={x}");

        // Steered pick with chase target beyond wall still cannot cross
        let steer = AnimalSteer {
            object_id: 0,
            goto_target: true,
            target: Some((10, 0)),
            loved_tx: 0,
            loved_ty: 0,
            loves_current_biome: true,
        };
        for seed in 0..20u64 {
            let mut s_rng = StdRng::seed_from_u64(seed);
            if let Some((px, _py)) = pick_animal_destination_steered(
                &w, &db, &mut s_rng, 0, 0, 12, 4, 6, false, Some(steer),
            ) {
                assert!(
                    px < 4,
                    "chase must still respect path block, got x={px} seed={seed}"
                );
            }
        }
    }

    #[test]
    fn maybe_update_loved_biome_only_when_loves() {
        let mut lx = 0;
        let mut ly = 0;
        maybe_update_loved_biome(false, 5, 6, &mut lx, &mut ly);
        assert_eq!((lx, ly), (0, 0));
        maybe_update_loved_biome(true, 5, 6, &mut lx, &mut ly);
        assert_eq!((lx, ly), (5, 6));
    }

    #[test]
    fn collect_bone_graves_near_finds_ids() {
        let mut w = World::new(20, 20, false);
        w.set_object(5, 5, 87);
        w.set_object(8, 5, 418); // wolf, not grave
        w.set_object(6, 5, 357);
        let g = collect_bone_graves_near(&w, 5, 5, 3);
        assert!(g.contains(&(5, 5)));
        assert!(g.contains(&(6, 5)));
        assert!(!g.iter().any(|&p| p == (8, 5)));
    }

    #[test]
    fn resolve_animal_chase_winter_wolf_global_grave_beyond_local() {
        // Haxe GetClosestBoneGrave uses full cursedGraves map (not r=80 scan).
        let far_graves = [(120, 0), (200, 0)];
        let chase = resolve_animal_chase(
            PARENT_WOLF,
            true,
            0.0,
            Season::Winter,
            SNOW,
            0,
            0,
            &[], // no players
            &far_graves,
            &[(PARENT_WOLF, 0, 0, 0.0)],
            0,
            None,
            false,
        );
        assert!(chase.goto_target);
        assert_eq!(chase.target, Some((120, 0)));
    }
}
