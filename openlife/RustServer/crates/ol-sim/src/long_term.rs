//! Long-term world decay / regrow / wall align (Haxe `TimeHelper.DoWorldLongTermTimeStuff`).
//!
//! Chunks:
//! - **TIME-LONG** — decay / regrow / wall align / seasonal biomes
//! - **FOODSTATS-DISK** residual — optional `ObjectCounts.txt` census dump
//!   (Haxe `TraceCountObjectsToDisk` sibling of `writeFoodStatistics`)
//!
//! Anchors: `DoWorldLongTermTimeStuff`, `DecayFloor`, `DecayObject`, `AlignWalls` /
//! `AlignWall`, `DoRespawnFromOriginal`, `DoSpringStuff`, `DoSeasonalBiomeChanges`
//! (`SpreadSnow` / `RemoveSnow` / `IsProtected`), `WorldMap.write` ObjectCounts dump.

use ol_content::ContentDb;
use ol_world::{ComplexObject, NestedHelper, World, GREEN, OCEAN, PASSABLE_RIVER, RIVER, SNOWINGREY};
use rand::Rng;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Haxe biome ids not all re-exported from ol_world root.
const SWAMP: u8 = 1;
const YELLOW: u8 = 2;
const GREY: u8 = 3;
const BIOME_SNOW: u8 = 4;
const BIOME_DESERT: u8 = 5;
const JUNGLE: u8 = 6;
const BIOME_BORDER_JUNGLE: u8 = 15;

/// Haxe `ServerSettings.WorldTimeParts` (mirrored; avoid world_time import cycle).
const WORLD_TIME_PARTS: i32 = 25;

fn world_time_slice_y_range(height: i32, time_parts: i32, world_map_time_step: u64) -> (i32, i32) {
    let parts = time_parts.max(1);
    let h = height.max(0);
    if h == 0 {
        return (0, 0);
    }
    let part_size = h / parts;
    if part_size <= 0 {
        return (0, h);
    }
    let band = (world_map_time_step % parts as u64) as i32;
    let start = band * part_size;
    let end = start + part_size;
    (start, end.min(h))
}

// ---------------------------------------------------------------------------
// ServerSettings-aligned constants
// ---------------------------------------------------------------------------

/// Haxe long-term uses `WorldTimeParts * 10` Y bands (slower full-map cycle).
pub const LONG_TERM_TIME_PARTS: i32 = WORLD_TIME_PARTS * 10;

/// Haxe `ServerSettings.FloorDecayChance` default (live: `GameplayKnobs.floor_decay_chance`).
pub const FLOOR_DECAY_CHANCE: f32 = 0.00001;
/// Haxe `ServerSettings.ObjDecayChance` default (live: `GameplayKnobs.obj_decay_chance`).
pub const OBJ_DECAY_CHANCE: f32 = 0.00005;

/// Live Haxe `ObjDecayChance` / `FloorDecayChance` / `AnimalDecayFactor` (SETTINGS-LONG-TAIL).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecayChanceKnobs {
    pub obj_decay_chance: f32,
    pub floor_decay_chance: f32,
    pub animal_decay_factor: f32,
    pub obj_decay_factor_for_permanent: f32,
    pub obj_decay_factor_for_food: f32,
    pub obj_decay_factor_for_clothing: f32,
    pub obj_decay_factor_for_walls: f32,
    pub obj_decay_factor_per_tech_level: f32,
    pub decay_factor_in_deep_water: f32,
    pub decay_factor_in_mountain: f32,
    pub decay_factor_in_walkable_water: f32,
    pub decay_factor_in_jungle: f32,
    pub decay_factor_in_swamp: f32,
    /// Compiled fallback is `OBJ_RESPAWN_CHANCE` (0.0005); Live Haxe default is 0.00006.
    pub obj_respawn_chance: f32,
    pub grow_back_plants_increase_if_low_population: f32,
    /// Compiled fallback is `GROW_BACK_ORIGINAL_PLANTS_FACTOR` (1.0); Live Haxe default is 0.02.
    pub grow_back_original_plants_factor: f32,
    /// Haxe `GrowNewPlantsFromExistingFactor` (0.05) — offspring from living plants.
    pub grow_new_from_existing_factor: f32,
}

impl Default for DecayChanceKnobs {
    fn default() -> Self {
        Self {
            obj_decay_chance: OBJ_DECAY_CHANCE,
            floor_decay_chance: FLOOR_DECAY_CHANCE,
            animal_decay_factor: ol_content::ANIMAL_DECAY_FACTOR,
            obj_decay_factor_for_permanent: OBJ_DECAY_FACTOR_FOR_PERMANENT,
            obj_decay_factor_for_food: OBJ_DECAY_FACTOR_FOR_FOOD,
            obj_decay_factor_for_clothing: OBJ_DECAY_FACTOR_FOR_CLOTHING,
            obj_decay_factor_for_walls: OBJ_DECAY_FACTOR_FOR_WALLS,
            obj_decay_factor_per_tech_level: OBJ_DECAY_FACTOR_PER_TECH_LEVEL,
            decay_factor_in_deep_water: DECAY_FACTOR_DEEP_WATER,
            decay_factor_in_mountain: DECAY_FACTOR_MOUNTAIN,
            decay_factor_in_walkable_water: DECAY_FACTOR_WALKABLE_WATER,
            decay_factor_in_jungle: DECAY_FACTOR_JUNGLE,
            decay_factor_in_swamp: DECAY_FACTOR_SWAMP,
            obj_respawn_chance: OBJ_RESPAWN_CHANCE,
            grow_back_plants_increase_if_low_population: GROW_BACK_LOW_POP_BOOST,
            grow_back_original_plants_factor: GROW_BACK_ORIGINAL_PLANTS_FACTOR,
            grow_new_from_existing_factor: GROW_NEW_FROM_EXISTING_FACTOR,
        }
    }
}

/// Content `decayFactor`, overridden by live `AnimalDecayFactor` for patched animal ids.
// Haxe: ServerSettings.PatchObjectData decayFactor = AnimalDecayFactor
pub fn decay_factor_for_object(
    obj_id: i32,
    content_decay_factor: f32,
    animal_decay_factor: f32,
) -> f32 {
    if ol_content::is_animal_decay_factor_id(obj_id) {
        if animal_decay_factor.is_finite() && animal_decay_factor >= 0.0 {
            animal_decay_factor
        } else {
            ol_content::ANIMAL_DECAY_FACTOR
        }
    } else {
        content_decay_factor
    }
}
/// Haxe `ServerSettings.ObjDecayFactorForWalls`.
pub const OBJ_DECAY_FACTOR_FOR_WALLS: f32 = 0.2;
/// Haxe `ServerSettings.ObjDecayFactorForPermanentObjs`.
pub const OBJ_DECAY_FACTOR_FOR_PERMANENT: f32 = 0.2;
/// Haxe `ServerSettings.ObjDecayFactorForFood`.
pub const OBJ_DECAY_FACTOR_FOR_FOOD: f32 = 2.0;
/// Haxe `ServerSettings.ObjDecayFactorForClothing`.
pub const OBJ_DECAY_FACTOR_FOR_CLOTHING: f32 = 2.0;
/// Haxe `ServerSettings.ObjDecayFactorPerTechLevel`.
pub const OBJ_DECAY_FACTOR_PER_TECH_LEVEL: f32 = 10.0;
/// Haxe `ServerSettings.DecayFactorInDeepWater`.
pub const DECAY_FACTOR_DEEP_WATER: f32 = 5.0;
/// Haxe `ServerSettings.DecayFactorInMountain`.
pub const DECAY_FACTOR_MOUNTAIN: f32 = 3.0;
/// Haxe `ServerSettings.DecayFactorInWalkableWater`.
pub const DECAY_FACTOR_WALKABLE_WATER: f32 = 2.0;
/// Haxe `ServerSettings.DecayFactorInJungle`.
pub const DECAY_FACTOR_JUNGLE: f32 = 2.0;
/// Haxe `ServerSettings.DecayFactorInSwamp`.
pub const DECAY_FACTOR_SWAMP: f32 = 2.0;
/// Haxe `ServerSettings.SeasonBiomeChangeChancePerYear`.
pub const SEASON_BIOME_CHANGE_CHANCE_PER_YEAR: f32 = 2.0;
/// Haxe `ServerSettings.SeasonBiomeRestoreFactor`.
pub const SEASON_BIOME_RESTORE_FACTOR: f32 = 2.0;

/// 618 Filled Small Trash Pit — default permanent decay product.
pub const TRASH_PIT_ID: i32 = 618;
/// Pine Floor.
pub const PINE_FLOOR_ID: i32 = 3290;
/// Stone Road.
pub const STONE_ROAD_ID: i32 = 1596;
/// Rabbit count-as id (allowed to decay despite short timeToChange).
pub const RABBIT_COUNT_AS: i32 = 161;
/// Objects deleted when found in water biomes (Hardened Row).
pub const WATER_DELETE_IDS: &[i32] = &[848];
/// Held-on-ground unstuck ids (carts / sheep on rope / riding horse).
pub const UNSTUCK_OBJECT_IDS: &[i32] = &[778, 3158, 3934, 3926, 770];
/// Rope produced by some unstuck transitions.
pub const ROPE_ID: i32 = 59;

// Wall families: [corner, vertical, horizontal] — Haxe AlignWalls
pub const WALL_ADOBE: [i32; 3] = [154, 156, 155];
pub const WALL_PLASTER: [i32; 3] = [1883, 1884, 1885];
pub const WALL_PINE: [i32; 3] = [111, 113, 112];
pub const WALL_SNOW: [i32; 3] = [3266, 3267, 3268];
pub const WALL_STONE: [i32; 3] = [885, 886, 887];
pub const WALL_ANCIENT: [i32; 3] = [895, 897, 896];
/// Fence uses onlyCornerIfWallTopOrBottom=true.
pub const WALL_FENCE: [i32; 3] = [551, 549, 550];

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Haxe `Biome.getBiomeDecayFactor`.
pub fn biome_decay_factor(biome: u8) -> f32 {
    biome_decay_factor_ex(
        biome,
        DECAY_FACTOR_DEEP_WATER,
        DECAY_FACTOR_MOUNTAIN,
        DECAY_FACTOR_WALKABLE_WATER,
        DECAY_FACTOR_JUNGLE,
        DECAY_FACTOR_SWAMP,
    )
}

/// Live-knob variant: deep water / mountain / walkable water / jungle / swamp biome decay.
// Haxe: ServerSettings.DecayFactorInDeepWater / InMountain / InWalkableWater / InJungle / InSwamp
// SETTINGS-LONG-TAIL
pub fn biome_decay_factor_ex(
    biome: u8,
    deep_water: f32,
    mountain: f32,
    walkable_water: f32,
    jungle: f32,
    swamp: f32,
) -> f32 {
    let pos = |v: f32, d: f32| {
        if v.is_finite() && v > 0.0 {
            v
        } else {
            d
        }
    };
    let deep = pos(deep_water, DECAY_FACTOR_DEEP_WATER);
    let mt = pos(mountain, DECAY_FACTOR_MOUNTAIN);
    let walk = pos(walkable_water, DECAY_FACTOR_WALKABLE_WATER);
    let jng = pos(jungle, DECAY_FACTOR_JUNGLE);
    let swp = pos(swamp, DECAY_FACTOR_SWAMP);
    match biome {
        b if b == SWAMP => swp,
        b if b == JUNGLE || b == BIOME_BORDER_JUNGLE => jng,
        b if b == SNOWINGREY => mt,
        b if b == OCEAN || b == RIVER => deep,
        b if b == PASSABLE_RIVER => walk,
        _ => 1.0,
    }
}

/// Haxe `ServerSettings.CanObjectRespawn` — Natural Spring 3030 / Tarry Spot 2285 /
/// Dug Big Rock 503 never respawn (and DecayObject also skips them).
// Haxe: ServerSettings.CanObjectRespawn L484-491
pub fn can_object_respawn(obj_id: i32) -> bool {
    obj_id != 3030 && obj_id != 2285 && obj_id != 503
}

/// Haxe wall families for AlignWalls (id table).
pub fn is_known_wall_id(id: i32) -> bool {
    wall_families().iter().any(|(fam, _)| fam.contains(&id))
}

fn wall_families() -> &'static [([i32; 3], bool)] {
    &[
        (WALL_ADOBE, false),
        (WALL_PLASTER, false),
        (WALL_PINE, false),
        (WALL_SNOW, false),
        (WALL_STONE, false),
        (WALL_ANCIENT, false),
        (WALL_FENCE, true),
    ]
}

/// True when clothing flag is set (Haxe clothing does not start with `n`).
#[inline]
pub fn is_clothing_flag(clothing: &str) -> bool {
    !clothing.is_empty() && !clothing.starts_with('n')
}

/// Haxe `ObjectData.isWall`: not clothing and rValue > 0.
#[inline]
pub fn is_wall_from_data(clothing: &str, r_value: f32) -> bool {
    !is_clothing_flag(clothing) && r_value > 0.0
}

/// Combined wall check: content rValue/clothing path, else known wall id table.
/// Haxe: `ObjectData.isWall` via live ObjectData; families cover align-only ids.
pub fn is_wall(content: &ContentDb, obj_id: i32) -> bool {
    if obj_id <= 0 {
        return false;
    }
    let base = content.resolve_base_id(obj_id);
    if let Some(def) = content.get(base) {
        if def.is_wall() {
            return true;
        }
        // Explicit non-wall from content (rValue=0 clothing=n) still may be fence align family.
        if is_known_wall_id(base) || is_known_wall_id(obj_id) {
            return true;
        }
        return false;
    }
    is_known_wall_id(obj_id) || is_known_wall_id(base)
}

/// Haxe strong wall for surrounding strength: isWall && rValue > 0.1 (fence excluded).
pub fn is_strong_wall(content: &ContentDb, obj_id: i32) -> bool {
    if obj_id <= 0 {
        return false;
    }
    let base = content.resolve_base_id(obj_id);
    if WALL_FENCE.contains(&base) || WALL_FENCE.contains(&obj_id) {
        return false;
    }
    if let Some(def) = content.get(base) {
        return def.is_wall() && def.r_value > 0.1;
    }
    // Known solid walls without content row.
    is_known_wall_id(base) && !WALL_FENCE.contains(&base)
}

/// Wall/floor insulation for IsProtected (Haxe ObjectData.getInsulation / rValue).
pub fn object_insulation(content: &ContentDb, obj_id: i32) -> f32 {
    if obj_id <= 0 {
        return 0.0;
    }
    let base = content.resolve_base_id(obj_id);
    if let Some(def) = content.get(base) {
        return def.insulation_for_protection();
    }
    // Known walls without content ≈ stone wall rValue 0.9
    if is_known_wall_id(base) && !WALL_FENCE.contains(&base) {
        return 0.9;
    }
    0.0
}

/// Floor insulation (Haxe floor ObjectData.getInsulation).
pub fn floor_insulation(content: &ContentDb, floor_id: i32) -> f32 {
    if floor_id <= 0 {
        return 0.0;
    }
    if let Some(def) = content.get(floor_id) {
        return def.insulation_for_protection();
    }
    // Mild default when floor present but no def (legacy proxy 0.2).
    0.2
}

/// Haxe `ObjectHelper.CalculateSurroundingWallStrength` (fence rValue gate ≈ known walls except fence).
pub fn surrounding_wall_strength(
    is_wall_at: impl Fn(i32, i32) -> bool,
    is_strong_wall_at: impl Fn(i32, i32) -> bool,
    x: i32,
    y: i32,
) -> f32 {
    let mut s = if is_wall_at(x, y) { 2.0 } else { 0.0 };
    for (dx, dy, w) in [
        (1, 0, 2.0),
        (-1, 0, 2.0),
        (0, 1, 2.0),
        (0, -1, 2.0),
        (2, 0, 2.0),
        (-2, 0, 2.0),
        (0, 2, 2.0),
        (0, -2, 2.0),
        (3, 0, 1.0),
        (-3, 0, 1.0),
        (0, 3, 1.0),
        (0, -3, 1.0),
    ] {
        if is_strong_wall_at(x + dx, y + dy) {
            s += w;
        }
    }
    s
}

/// Haxe `ObjectHelper.CalculateSurroundingFloorStrength`.
pub fn surrounding_floor_strength(has_floor: impl Fn(i32, i32) -> bool, x: i32, y: i32) -> f32 {
    let mut s = if has_floor(x, y) { 1.0 } else { 0.0 };
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if has_floor(x + dx, y + dy) {
            s += 1.0;
        }
    }
    s
}

/// Floor decay strength factor (Haxe DecayFloor).
///
/// `totalStrength = wall + floor`; if `< 1` → 1; if `> 5` → factor 0 else `1/total`.
pub fn floor_decay_strength_factor(wall_strength: f32, floor_strength: f32) -> f32 {
    let mut total = wall_strength + floor_strength;
    if total < 1.0 {
        total = 1.0;
    }
    if total > 5.0 {
        0.0
    } else {
        1.0 / total
    }
}

/// Pure floor decay chance before RNG (Haxe DecayFloor).
pub fn floor_decay_chance(
    floor_decay_factor: f32,
    biome: u8,
    wall_strength: f32,
    floor_strength: f32,
) -> f32 {
    floor_decay_chance_ex(
        floor_decay_factor,
        biome,
        wall_strength,
        floor_strength,
        FLOOR_DECAY_CHANCE,
    )
}

/// Floor decay chance with live `ServerSettings.FloorDecayChance`.
// Haxe: TimeHelper.DecayFloor `ServerSettings.FloorDecayChance * objData.decayFactor`
pub fn floor_decay_chance_ex(
    floor_decay_factor: f32,
    biome: u8,
    wall_strength: f32,
    floor_strength: f32,
    floor_decay_chance: f32,
) -> f32 {
    if floor_decay_factor <= 0.0 {
        return 0.0;
    }
    let base = if floor_decay_chance.is_finite() && floor_decay_chance >= 0.0 {
        floor_decay_chance
    } else {
        FLOOR_DECAY_CHANCE
    };
    let mut chance = base * floor_decay_factor;
    chance *= biome_decay_factor(biome);
    chance *= floor_decay_strength_factor(wall_strength, floor_strength);
    chance.max(0.0)
}

/// Floor decay product placement (Haxe DecayFloor set path).
///
/// Returns `(new_floor, new_object_if_any)` — object only when floor clears and tile empty.
pub fn floor_decay_result(
    _floor_id: i32,
    obj_id: i32,
    decays_to: i32,
    decays_to_is_floor: bool,
) -> (i32, Option<i32>) {
    let to = if decays_to == 0 {
        TRASH_PIT_ID
    } else {
        decays_to
    };
    if decays_to_is_floor {
        (to, None)
    } else {
        let new_obj = if obj_id == 0 { Some(to) } else { None };
        (0, new_obj)
    }
}

/// Floor factor for object decay (Haxe floorDecayFactor).
pub fn object_floor_decay_factor(floor_id: i32) -> f32 {
    if floor_id <= 0 {
        return 1.0;
    }
    if floor_id == PINE_FLOOR_ID || floor_id == STONE_ROAD_ID {
        return 0.5;
    }
    0.0
}

/// Population gate: natural objects only decay when current ≥ original * 0.8.
pub fn population_allows_decay(current: i32, original: i32) -> bool {
    if original <= 0 {
        return true; // no census → allow (crafted / uncounted)
    }
    (current as f32) >= (original as f32) * 0.8
}

/// Tech factor: `techLevel / (techLevel + crafting_steps)`.
pub fn decay_tech_factor(crafting_steps: i32) -> f32 {
    decay_tech_factor_ex(crafting_steps, OBJ_DECAY_FACTOR_PER_TECH_LEVEL)
}

/// Live-knob variant: `tech` = Haxe ObjDecayFactorPerTechLevel.
// Haxe: ServerSettings.ObjDecayFactorPerTechLevel
// SETTINGS-LONG-TAIL
pub fn decay_tech_factor_ex(crafting_steps: i32, tech: f32) -> f32 {
    let tech = if tech.is_finite() && tech > 0.0 {
        tech
    } else {
        OBJ_DECAY_FACTOR_PER_TECH_LEVEL
    };
    tech / (tech + crafting_steps.max(0) as f32)
}

/// Inputs for object long-term decay chance (Haxe DecayObject core).
#[derive(Debug, Clone, Copy)]
pub struct ObjectDecayInput {
    pub decay_factor: f32,
    pub crafting_steps: i32,
    pub floor_id: i32,
    pub biome: u8,
    pub is_wall: bool,
    pub is_permanent: bool,
    pub is_food: bool,
    pub is_clothing: bool,
    pub is_no_bone_grave: bool,
    pub contains_something: bool,
    pub decays_to_obj: i32,
    /// Haxe timeToChange > 0 on helper.
    pub has_active_time_transition: bool,
    pub count_as: i32,
    pub current_count: i32,
    pub original_count: i32,
}

/// Pure object decay chance; `None` means hard-blocked (do not roll).
pub fn object_decay_chance(inp: &ObjectDecayInput) -> Option<f32> {
    object_decay_chance_ex(inp, OBJ_DECAY_CHANCE)
}

/// Object decay chance with live `ServerSettings.ObjDecayChance`.
// Haxe: TimeHelper.DecayObject `ServerSettings.ObjDecayChance * objData.decayFactor`
pub fn object_decay_chance_ex(inp: &ObjectDecayInput, obj_decay_chance: f32) -> Option<f32> {
    let mut knobs = DecayChanceKnobs::default();
    knobs.obj_decay_chance = obj_decay_chance;
    object_decay_chance_ex2(inp, knobs)
}

/// Live `ObjDecayChance` + permanent / food / clothing / wall decay factors.
// Haxe: TimeHelper.DecayObject ServerSettings.ObjDecayFactor*
pub fn object_decay_chance_ex2(
    inp: &ObjectDecayInput,
    knobs: DecayChanceKnobs,
) -> Option<f32> {
    let obj_decay_chance = knobs.obj_decay_chance;
    let perm_factor = knobs.obj_decay_factor_for_permanent;
    let food_factor = knobs.obj_decay_factor_for_food;
    let clothing_factor = knobs.obj_decay_factor_for_clothing;
    let wall_factor = knobs.obj_decay_factor_for_walls;
    let tech_level = knobs.obj_decay_factor_per_tech_level;
    let deep_water = knobs.decay_factor_in_deep_water;
    let mountain = knobs.decay_factor_in_mountain;
    let walkable_water = knobs.decay_factor_in_walkable_water;
    let jungle = knobs.decay_factor_in_jungle;
    let swamp = knobs.decay_factor_in_swamp;
    if inp.decay_factor <= 0.0 {
        return None;
    }
    if !population_allows_decay(inp.current_count, inp.original_count) {
        return None;
    }
    let floor_df = object_floor_decay_factor(inp.floor_id);
    // Don't decay non-walls on solid floor.
    if floor_df < 0.01 && !inp.is_wall {
        return None;
    }
    if floor_df < 0.01 && inp.contains_something && inp.decays_to_obj < 1 {
        return None;
    }
    // time transition objects without custom decay (except rabbit).
    if inp.has_active_time_transition && inp.decays_to_obj < 1 && inp.count_as != RABBIT_COUNT_AS {
        return None;
    }

    let base = if obj_decay_chance.is_finite() && obj_decay_chance >= 0.0 {
        obj_decay_chance
    } else {
        OBJ_DECAY_CHANCE
    };
    let mut chance = base * inp.decay_factor;
    chance *= decay_tech_factor_ex(inp.crafting_steps, tech_level);
    if inp.is_wall {
        let wf = if wall_factor.is_finite() && wall_factor > 0.0 {
            wall_factor
        } else {
            OBJ_DECAY_FACTOR_FOR_WALLS
        };
        chance *= wf;
    } else {
        chance *= floor_df;
    }
    let mut biome_f =
        biome_decay_factor_ex(inp.biome, deep_water, mountain, walkable_water, jungle, swamp);
    if inp.floor_id != 0 {
        biome_f = 1.0;
    }
    chance *= biome_f;
    if inp.is_no_bone_grave {
        chance *= 0.01;
    }
    if inp.is_food {
        let ff = if food_factor.is_finite() && food_factor > 0.0 {
            food_factor
        } else {
            OBJ_DECAY_FACTOR_FOR_FOOD
        };
        chance *= ff;
    }
    if inp.is_clothing {
        let cf = if clothing_factor.is_finite() && clothing_factor > 0.0 {
            clothing_factor
        } else {
            OBJ_DECAY_FACTOR_FOR_CLOTHING
        };
        chance *= cf;
    }
    if inp.is_permanent {
        let pf = perm_factor;
        chance *= if pf.is_finite() && pf >= 0.0 {
            pf
        } else {
            OBJ_DECAY_FACTOR_FOR_PERMANENT
        };
    }
    Some(chance.max(0.0))
}

/// Resolve decay product id (Haxe DecayObject).
pub fn resolve_object_decay_to(decays_to: i32, permanent: bool) -> i32 {
    if decays_to == 0 && permanent {
        TRASH_PIT_ID
    } else {
        decays_to
    }
}

/// Haxe AlignWall orientation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallOrientation {
    Corner,
    Vertical,
    Horizontal,
}

/// Pure wall orientation from neighbor wall flags (Haxe AlignWall).
pub fn wall_orientation(
    left: bool,
    right: bool,
    north: bool,
    south: bool,
    only_corner_if_top_or_bottom: bool,
) -> WallOrientation {
    let (is_h, is_v) = if only_corner_if_top_or_bottom {
        let mut is_horizontal = !north && !south;
        let mut is_vertical = !left && !right;
        if is_horizontal {
            is_vertical = false;
        }
        (is_horizontal, is_vertical)
    } else {
        let is_horizontal = left && right && !north && !south;
        let is_vertical = !left && !right && north && south;
        (is_horizontal, is_vertical)
    };
    if !is_h && !is_v {
        WallOrientation::Corner
    } else if is_v {
        WallOrientation::Vertical
    } else {
        WallOrientation::Horizontal
    }
}

/// Map orientation to wall family member id.
pub fn wall_id_for_orientation(family: [i32; 3], orient: WallOrientation) -> i32 {
    match orient {
        WallOrientation::Corner => family[0],
        WallOrientation::Vertical => family[1],
        WallOrientation::Horizontal => family[2],
    }
}

/// If `parent_id` is in a wall family, return aligned id (or same if already correct).
pub fn align_wall_id(
    parent_id: i32,
    left: bool,
    right: bool,
    north: bool,
    south: bool,
) -> Option<i32> {
    for &(family, fence_mode) in wall_families() {
        if !family.contains(&parent_id) {
            continue;
        }
        let orient = wall_orientation(left, right, north, south, fence_mode);
        return Some(wall_id_for_orientation(family, orient));
    }
    None
}

/// Haxe `ServerSettings.GrowBackOriginalPlantsFactor` compiled fallback
/// (live: `DecayChanceKnobs.grow_back_original_plants_factor`; Haxe default 0.02).
pub const GROW_BACK_ORIGINAL_PLANTS_FACTOR: f32 = 1.0;
/// Haxe `GrowNewPlantsFromExistingFactor` (offspring from living plants).
pub const GROW_NEW_FROM_EXISTING_FACTOR: f32 = 0.05;
/// Haxe `GrowBackPlantsIncreaseIfLowPopulation` when current < original/2.
pub const GROW_BACK_LOW_POP_BOOST: f32 = 2.0;
/// Haxe `ObjRespawnChance` per empty original tile scan (RespawnObjects).
pub const OBJ_RESPAWN_CHANCE: f32 = 0.0005;
/// Haxe SpawnObject default dist / tries.
pub const SPAWN_NEAR_DIST: i32 = 6;
pub const SPAWN_NEAR_TRIES: i32 = 3;

/// Haxe DoRespawnFromOriginal spring empty-tile rolls + generic springRegrowFactor path.
///
/// Returns object id to place, or None.
///
/// Hardcoded plant rates keep parity with prior tests; any other id with
/// `spring_regrow_factor > 0` uses:
/// `chance = years * spring_regrow * GrowBackOriginalPlantsFactor` (clamped).
pub fn respawn_from_original_roll(
    original_id: i32,
    time_passed_years: f32,
    rand01: f32,
) -> Option<i32> {
    // current=0, original=0 → no population gate (legacy call sites / unit tests).
    respawn_from_original_roll_ex(original_id, time_passed_years, rand01, 0.0, 0.0, 0.0)
}

/// Extended roll with content-driven spring regrow + population gate.
///
/// `current_count` / `original_count` are for the **count_as** id.
/// When `original_count > 0` and `current_count >= original_count`, no spawn
/// (unless `force_from_originals` — empty original tile always uses fromOriginals).
pub fn respawn_from_original_roll_ex(
    original_id: i32,
    time_passed_years: f32,
    rand01: f32,
    spring_regrow_factor: f32,
    current_count: f32,
    original_count: f32,
) -> Option<i32> {
    respawn_from_original_roll_ex2(
        original_id,
        time_passed_years,
        rand01,
        spring_regrow_factor,
        current_count,
        original_count,
        GROW_BACK_LOW_POP_BOOST,
    )
}

/// Same as [`respawn_from_original_roll_ex`] with live `GrowBackPlantsIncreaseIfLowPopulation`.
pub fn respawn_from_original_roll_ex2(
    original_id: i32,
    time_passed_years: f32,
    rand01: f32,
    spring_regrow_factor: f32,
    current_count: f32,
    original_count: f32,
    low_pop_boost: f32,
) -> Option<i32> {
    respawn_from_original_roll_ex3(
        original_id,
        time_passed_years,
        rand01,
        spring_regrow_factor,
        current_count,
        original_count,
        low_pop_boost,
        GROW_BACK_ORIGINAL_PLANTS_FACTOR,
    )
}

/// Same as [`respawn_from_original_roll_ex2`] with live `GrowBackOriginalPlantsFactor`.
pub fn respawn_from_original_roll_ex3(
    original_id: i32,
    time_passed_years: f32,
    rand01: f32,
    spring_regrow_factor: f32,
    current_count: f32,
    original_count: f32,
    low_pop_boost: f32,
    original_plants_factor: f32,
) -> Option<i32> {
    // Haxe CanObjectRespawn: spring / tar / dug rock never grow back.
    if !can_object_respawn(original_id) {
        return None;
    }
    // Population: still allow when original_count==0 (unknown census).
    if original_count > 0.0 && current_count >= original_count {
        return None;
    }

    // true needed time is 4× (spring only); chance uses years directly as Haxe.
    let mut chance = match original_id {
        50 => time_passed_years / 60.0,                 // Milkweed
        136 => time_passed_years / 60.0,                // Sapling
        1261 => time_passed_years / (60.0 * 24.0),      // Goose pond with egg
        211 => time_passed_years / (60.0 * 24.0 * 2.0), // Fertile soil
        _ if spring_regrow_factor > 0.0 => {
            // Haxe: SpringRegrowChance * springRegrowFactor * GrowBackOriginalPlantsFactor
            // SpringRegrowChance is scaled into years by TimeHelper; approximate:
            let factor = if original_plants_factor.is_finite() && original_plants_factor >= 0.0 {
                original_plants_factor
            } else {
                GROW_BACK_ORIGINAL_PLANTS_FACTOR
            };
            (time_passed_years / 60.0) * spring_regrow_factor * factor
        }
        _ => return None,
    };
    if original_count > 0.0 && current_count < original_count * 0.5 {
        let boost = if low_pop_boost.is_finite() && low_pop_boost > 0.0 {
            low_pop_boost
        } else {
            GROW_BACK_LOW_POP_BOOST
        };
        chance *= boost;
    }
    if rand01 < chance {
        Some(original_id)
    } else {
        None
    }
}

/// Haxe `SpawnObject` placement trial: empty tile, no floor, no object south, biome ok.
///
/// Returns `(tx, ty)` if a candidate passes gates (caller places object).
pub fn spawn_near_candidate(
    center_x: i32,
    center_y: i32,
    dist: i32,
    // random offsets in [-dist, dist]
    ox: i32,
    oy: i32,
    obj_at: impl Fn(i32, i32) -> i32,
    floor_at: impl Fn(i32, i32) -> i32,
    biome_ok: impl Fn(i32, i32) -> bool,
) -> Option<(i32, i32)> {
    let tx = center_x + ox.clamp(-dist, dist);
    let ty = center_y + oy.clamp(-dist, dist);
    if obj_at(tx, ty) != 0 {
        return None;
    }
    if obj_at(tx, ty - 1) != 0 {
        return None;
    }
    if floor_at(tx, ty) != 0 {
        return None;
    }
    if !biome_ok(tx, ty) {
        return None;
    }
    Some((tx, ty))
}

/// Haxe RespawnObjects per-tile gate: original non-zero, ObjRespawnChance, under population.
pub fn should_try_respawn_object(
    original_id: i32,
    rand_respawn: f32,
    current_count: i32,
    original_count: i32,
) -> bool {
    should_try_respawn_object_ex(
        original_id,
        rand_respawn,
        current_count,
        original_count,
        OBJ_RESPAWN_CHANCE,
    )
}

/// Same as [`should_try_respawn_object`] with live `ObjRespawnChance`.
pub fn should_try_respawn_object_ex(
    original_id: i32,
    rand_respawn: f32,
    current_count: i32,
    original_count: i32,
    chance: f32,
) -> bool {
    if original_id <= 0 {
        return false;
    }
    // Haxe RespawnObjects: CanObjectRespawn false → skip.
    if !can_object_respawn(original_id) {
        return false;
    }
    let chance = if chance.is_finite() && chance >= 0.0 {
        chance
    } else {
        OBJ_RESPAWN_CHANCE
    };
    if rand_respawn >= chance {
        return false;
    }
    if original_count > 0 && current_count >= original_count {
        return false;
    }
    // Haxe: current/(original+1) > random → skip
    if original_count >= 0 {
        let ratio = current_count as f32 / (original_count as f32 + 1.0);
        // caller should also pass a second random for this; keep simple: allow when under pop
        let _ = ratio;
    }
    true
}

/// Pond 511 → Canada Goose Pond 141 in spring.
pub fn spring_pond_to_goose_roll(time_passed_years: f32, rand01: f32) -> bool {
    rand01 < time_passed_years / 60.0
}

/// Bear cave 630 → awake 648 when hungry grizzlies underpopulated.
pub fn spring_bear_cave_awake_roll(
    hungry_grizzly_count: i32,
    bear_cave_original: i32,
    time_passed_years: f32,
    rand01: f32,
) -> bool {
    if hungry_grizzly_count as f32 >= bear_cave_original as f32 * 0.1 {
        return false;
    }
    rand01 < time_passed_years / 2.0
}

/// Haxe IsProtected biomes that can receive snow.
pub fn snow_can_cover_biome(biome: u8) -> bool {
    matches!(
        biome,
        b if b == GREY || b == YELLOW || b == GREEN || b == SWAMP || b == PASSABLE_RIVER
    )
}

/// Insulation protection roll (Haxe IsProtected core without random double wall).
///
/// Returns true if protected (snow should not land).
pub fn is_protected_by_insulation(
    biome: u8,
    wall_insulation: f32,
    floor_insulation: f32,
    rand_wall1: f32,
    rand_wall2: f32,
    rand_floor: f32,
) -> bool {
    if !snow_can_cover_biome(biome) {
        return true;
    }
    if wall_insulation > rand_wall1 {
        return true;
    }
    if wall_insulation > rand_wall2 {
        return true;
    }
    if floor_insulation > rand_floor {
        return true;
    }
    false
}

/// Spread-snow chance for one tile (Haxe SpreadSnow entry).
pub fn spread_snow_chance(time_passed_years: f32) -> f32 {
    time_passed_years * SEASON_BIOME_CHANGE_CHANCE_PER_YEAR * 4.0
}

/// Remove-snow chance (Haxe RemoveSnow).
pub fn remove_snow_chance(time_passed_years: f32) -> f32 {
    time_passed_years * SEASON_BIOME_CHANGE_CHANCE_PER_YEAR * 4.0 * SEASON_BIOME_RESTORE_FACTOR
}

// ---------------------------------------------------------------------------
// Runtime state
// ---------------------------------------------------------------------------

/// Haxe long-term step counters + object population censuses.
#[derive(Debug, Clone)]
pub struct LongTermState {
    /// Haxe shares `worldMapTimeStep` with map time; we keep a dedicated step for *10 parts.
    pub step: u64,
    /// Haxe `LongTimePassedToDoAllTimeSteps` (seconds for last full long-term cycle).
    pub time_passed_all_steps: f32,
    pub cycle_started_sim_time: f32,
    /// Sparse original object layer (Haxe originalObjects[0]).
    pub original_objects: HashMap<(i32, i32), i32>,
    /// Haxe originalObjectsCount.
    pub original_counts: HashMap<i32, i32>,
    /// Haxe currentObjectsCount.
    pub current_counts: HashMap<i32, i32>,
    /// True after first census from world / seed.
    pub counts_ready: bool,
    /// Local original biome seeds when map-time map not yet filled (non-snow first visit).
    pub original_biomes: HashMap<(i32, i32), u8>,
}

impl Default for LongTermState {
    fn default() -> Self {
        Self {
            step: 0,
            time_passed_all_steps: 1.0,
            cycle_started_sim_time: 0.0,
            original_objects: HashMap::new(),
            original_counts: HashMap::new(),
            current_counts: HashMap::new(),
            counts_ready: false,
            original_biomes: HashMap::new(),
        }
    }
}

/// Haxe ObjectHelper/ObjectData `parentId` for contained nest census.
/// Contained path does **not** apply `countsOrGrowsAs` (Haxe `countObjects` L1152–1156).
// Haxe: ObjectHelper.get_parentId / countObjects nest
pub fn count_parent_id(content: &ContentDb, obj_id: i32) -> i32 {
    if obj_id <= 0 {
        return 0;
    }
    content.resolve_base_id(obj_id)
}

/// Increment census map by one for a non-zero key.
#[inline]
fn bump_count_map(map: &mut HashMap<i32, i32>, key: i32) {
    if key > 0 {
        *map.entry(key).or_insert(0) += 1;
    }
}

/// Add one level of contained nest under a ground helper (Haxe objectHelpers loop).
///
/// Counts `contained` + one sub-contained level (`slots` recursive or wire `nested`).
// Haxe: WorldMap.countObjects L1149–1158
fn add_helper_contained_counts(
    map: &mut HashMap<i32, i32>,
    helper: &ComplexObject,
    content: &ContentDb,
) {
    if !helper.slots.is_empty() {
        for slot in &helper.slots {
            if slot.id <= 0 {
                continue;
            }
            bump_count_map(map, count_parent_id(content, slot.id));
            for sub in &slot.contained {
                if sub.id <= 0 {
                    continue;
                }
                bump_count_map(map, count_parent_id(content, sub.id));
            }
        }
        return;
    }
    for (i, &cid) in helper.contained.iter().enumerate() {
        if cid <= 0 {
            continue;
        }
        bump_count_map(map, count_parent_id(content, cid));
        if let Some(nest) = helper.nested.get(i) {
            for &sid in nest {
                if sid <= 0 {
                    continue;
                }
                bump_count_map(map, count_parent_id(content, sid));
            }
        }
    }
}

/// Haxe `WorldMap.countObjects` — ground tiles (+ optional objectHelpers nest).
///
/// Ground: `countsOrGrowsAs` then parent/base id ([`LongTermState::count_as_of`]).
/// Nest: `parentId` only for contained + sub-contained (no countsOrGrowsAs).
// Haxe: WorldMap.countObjects L1132–1161
pub fn count_objects_from_world(
    world: &World,
    content: &ContentDb,
    include_contained_nest: bool,
) -> HashMap<i32, i32> {
    let mut obj_list: HashMap<i32, i32> = HashMap::new();
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        return obj_list;
    }
    for y in 0..h {
        for x in 0..w {
            let id = world.get_object(x, y);
            if id <= 0 {
                continue;
            }
            bump_count_map(&mut obj_list, LongTermState::count_as_of(content, id));
        }
    }
    if include_contained_nest {
        for helper in world.helpers.values() {
            add_helper_contained_counts(&mut obj_list, helper, content);
        }
    }
    obj_list
}

/// Haxe `ServerSettings.TicksBetweenSaving` — period for `updateObjectCounts`.
// Haxe: ServerSettings.TicksBetweenSaving = 600; TimeHelper L162
pub const OBJECT_COUNTS_RECOMPUTE_TICKS: u64 = 600;

/// True when this tick should run Haxe `updateObjectCounts`.
// Haxe: TimeHelper `(tick + 20) % TicksBetweenSaving == 0`
pub fn should_update_object_counts(tick: u64) -> bool {
    let period = OBJECT_COUNTS_RECOMPUTE_TICKS.max(1);
    (tick.wrapping_add(20)) % period == 0
}

impl LongTermState {
    /// Haxe `countsOrGrowsAs > 0 ? countsOrGrowsAs : parentId` (dummy → base).
    pub fn count_as_of(content: &ContentDb, obj_id: i32) -> i32 {
        if obj_id <= 0 {
            return 0;
        }
        let base = content.resolve_base_id(obj_id);
        if let Some(def) = content.get(base) {
            if def.counts_or_grows_as > 0 {
                return def.counts_or_grows_as;
            }
        }
        base
    }

    pub fn bump_current(&mut self, count_as: i32, delta: i32) {
        if count_as <= 0 {
            return;
        }
        let e = self.current_counts.entry(count_as).or_insert(0);
        *e = (*e + delta).max(0);
    }

    pub fn ensure_original_at(&mut self, x: i32, y: i32, obj_id: i32) {
        if obj_id <= 0 {
            return;
        }
        self.original_objects.entry((x, y)).or_insert(obj_id);
    }

    /// Seed original + current counts from a resident world snapshot (once).
    ///
    /// Haxe load path: `countObjects(originalObjects)` / `countObjects(objects)` — ground
    /// only (no objectHelpers nest). Nest is added later by [`Self::update_object_counts`].
    // Haxe: WorldMap load L994–995 countObjects without helpers
    pub fn seed_from_world_if_needed(&mut self, world: &World, content: &ContentDb) {
        if self.counts_ready {
            return;
        }
        let w = world.width_tiles;
        let h = world.height_tiles;
        if w <= 0 || h <= 0 {
            self.counts_ready = true;
            return;
        }
        // Ground census (countsOrGrowsAs); record per-tile originals.
        for y in 0..h {
            for x in 0..w {
                let id = world.get_object(x, y);
                if id <= 0 {
                    continue;
                }
                self.ensure_original_at(x, y, id);
            }
        }
        let ground = count_objects_from_world(world, content, false);
        self.original_counts = ground.clone();
        self.current_counts = ground;
        self.counts_ready = true;
    }

    /// Full recompute of `current_counts` from world ground + contained nest.
    ///
    /// Haxe: `WorldMap.updateObjectCounts` → `countObjects(objects, objectHelpers)`.
    /// Called periodically (TicksBetweenSaving) so player USE/DROP drift is corrected.
    // Haxe: WorldMap.updateObjectCounts L1567–1570
    pub fn update_object_counts(&mut self, world: &World, content: &ContentDb) {
        self.current_counts = count_objects_from_world(world, content, true);
        // After a full recompute we treat census as ready even if seed was skipped.
        self.counts_ready = true;
    }

    /// Seed if needed, then recompute current with nest (boot / pre-dump path).
    // Haxe: load countObjects + first updateObjectCounts before dump
    pub fn ensure_counts_for_dump(&mut self, world: &World, content: &ContentDb) {
        self.seed_from_world_if_needed(world, content);
        self.update_object_counts(world, content);
    }

    /// Haxe ObjectCounts dump lines (id-sorted).
    ///
    /// Format: `Count object: [id] {desc}: {current} original: {original}`
    // Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk L806–809
    pub fn format_object_counts_lines<F>(&self, mut desc_of: F) -> Vec<String>
    where
        F: FnMut(i32) -> String,
    {
        format_object_counts_lines(&self.current_counts, &self.original_counts, &mut desc_of)
    }

    /// Full ObjectCounts dump body (trailing newline when non-empty).
    // Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk
    pub fn format_object_counts_text<F>(&self, desc_of: F) -> String
    where
        F: FnMut(i32) -> String,
    {
        format_object_counts_text(&self.current_counts, &self.original_counts, desc_of)
    }

    /// Write `ObjectCounts.txt` (or path) — Haxe optional census dump on save.
    // Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk L797–812
    pub fn write_object_counts<F>(&self, path: impl AsRef<Path>, desc_of: F) -> Result<(), String>
    where
        F: FnMut(i32) -> String,
    {
        write_object_counts(&self.current_counts, &self.original_counts, path, desc_of)
    }
}

/// Default ObjectCounts dump filename under save directory.
///
/// Haxe: `ObjectCounts{tmpDataNumber}.txt` when `TraceCountObjectsToDisk`.
/// Rust: fixed `ObjectCounts.txt` (same diagnostic; no slot rotation).
// Haxe: WorldMap.write L803
pub const DEFAULT_OBJECT_COUNTS_FILE: &str = "ObjectCounts.txt";

/// Haxe save-slot ObjectCounts filename.
// Haxe: WorldMap.write L803 `ObjectCounts${tmpDataNumber}.txt`
pub fn haxe_object_counts_slot_filename(tmp_data_number: u32) -> String {
    format!("ObjectCounts{tmp_data_number}.txt")
}

/// One Haxe ObjectCounts dump line.
///
/// `Count object: [{id}] {description}: {current} original: {original}`
// Haxe: WorldMap.writeToDiskHelper L808
pub fn format_object_count_line(
    object_id: i32,
    description: &str,
    current: i32,
    original: i32,
) -> String {
    let desc = if description.is_empty() {
        "object"
    } else {
        description
    };
    format!("Count object: [{object_id}] {desc}: {current} original: {original}")
}

/// Format ObjectCounts lines sorted by object id.
// Haxe: WorldMap.writeToDiskHelper L806–809
pub fn format_object_counts_lines<F>(
    current: &HashMap<i32, i32>,
    original: &HashMap<i32, i32>,
    desc_of: &mut F,
) -> Vec<String>
where
    F: FnMut(i32) -> String,
{
    // Haxe iterates `currentObjectsCount.keys()` only (not originals-only ids).
    let mut ids: Vec<i32> = current.keys().copied().collect();
    ids.sort_unstable();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let cur = current.get(&id).copied().unwrap_or(0);
        // Haxe Map Int default 0 when original key missing
        let orig = original.get(&id).copied().unwrap_or(0);
        let desc = desc_of(id);
        out.push(format_object_count_line(id, &desc, cur, orig));
    }
    out
}

/// Full ObjectCounts dump body (joined `\n`, trailing newline when non-empty).
// Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk
pub fn format_object_counts_text<F>(
    current: &HashMap<i32, i32>,
    original: &HashMap<i32, i32>,
    mut desc_of: F,
) -> String
where
    F: FnMut(i32) -> String,
{
    let lines = format_object_counts_lines(current, original, &mut desc_of);
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// Write ObjectCounts census text dump to disk.
// Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk L797–812
pub fn write_object_counts<F>(
    current: &HashMap<i32, i32>,
    original: &HashMap<i32, i32>,
    path: impl AsRef<Path>,
    desc_of: F,
) -> Result<(), String>
where
    F: FnMut(i32) -> String,
{
    let text = format_object_counts_text(current, original, desc_of);
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("object counts mkdir: {e}"))?;
        }
    }
    fs::write(path, text.as_bytes()).map_err(|e| format!("object counts write: {e}"))
}

/// One tile mutation from long-term pass (for MX fan-out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongTermChange {
    pub x: i32,
    pub y: i32,
    pub object_id: i32,
    pub floor_id: i32,
}

// ---------------------------------------------------------------------------
// Apply DoWorldLongTermTimeStuff
// ---------------------------------------------------------------------------

/// Apply one Y-band of long-term world maintenance.
///
/// Original biomes for snow restore: [`LongTermState::original_biomes`]
/// (caller should merge map-time originals; also seeded on non-snow first visit).
pub fn do_world_long_term_time_stuff(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    season_is_spring: bool,
    season_is_winter: bool,
    season_is_summer: bool,
    sim_time: f32,
    rng: &mut impl Rng,
) -> Vec<LongTermChange> {
    do_world_long_term_time_stuff_ex(
        world,
        content,
        long_term,
        season_is_spring,
        season_is_winter,
        season_is_summer,
        sim_time,
        rng,
        DecayChanceKnobs::default(),
    )
}

/// Long-term band with live decay chance knobs (SETTINGS-LONG-TAIL).
pub fn do_world_long_term_time_stuff_ex(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    season_is_spring: bool,
    season_is_winter: bool,
    season_is_summer: bool,
    sim_time: f32,
    rng: &mut impl Rng,
    decay_knobs: DecayChanceKnobs,
) -> Vec<LongTermChange> {
    let mut changes = Vec::new();
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        long_term.step = long_term.step.wrapping_add(1);
        return changes;
    }

    long_term.seed_from_world_if_needed(world, content);

    // Cycle timing (Haxe worldMapTimeStep % timeParts == 0 on shared step;
    // we recompute when long-term step hits band 0).
    if long_term.step % (LONG_TERM_TIME_PARTS.max(1) as u64) == 0 {
        if long_term.cycle_started_sim_time > 0.0 || long_term.step > 0 {
            long_term.time_passed_all_steps =
                (sim_time - long_term.cycle_started_sim_time).max(0.001);
        } else {
            long_term.time_passed_all_steps = 1.0;
        }
        long_term.cycle_started_sim_time = sim_time;
    }

    let (start_y, end_y) = world_time_slice_y_range(h, LONG_TERM_TIME_PARTS, long_term.step);
    long_term.step = long_term.step.wrapping_add(1);

    // Haxe: timePassedInYears = LongTimePassedToDoAllTimeSteps / 60
    let years = long_term.time_passed_all_steps / 60.0;

    for y in start_y..end_y {
        for x in 0..w {
            let obj_id = world.get_object(x, y);
            let floor_id = world.get_floor(x, y) as i32;
            long_term.ensure_original_at(x, y, obj_id);

            // Seed original biome when tile is not snow (map-time may already have it).
            let biome_now = world.get_biome(x, y);
            if biome_now != BIOME_SNOW && biome_now != SNOWINGREY {
                long_term.original_biomes.entry((x, y)).or_insert(biome_now);
            }

            // Seasonal biome snow spread / restore.
            // Haxe: DoSeasonalBiomeChanges
            if season_is_winter {
                try_spread_snow(world, content, long_term, x, y, years, rng, &mut changes);
            }
            if season_is_summer || season_is_spring {
                try_remove_snow(world, long_term, x, y, years, rng);
            }

            // Spring empty-tile original respawn (GrowBackOriginalPlants / DoRespawnFromOriginal).
            if obj_id == 0 && floor_id == 0 && season_is_spring {
                let orig = long_term
                    .original_objects
                    .get(&(x, y))
                    .copied()
                    .unwrap_or(0);
                if orig > 0 {
                    let ca = LongTermState::count_as_of(content, orig);
                    let cur = long_term.current_counts.get(&ca).copied().unwrap_or(0) as f32;
                    let org = long_term.original_counts.get(&ca).copied().unwrap_or(0) as f32;
                    let spring_f = content
                        .get(content.resolve_base_id(orig))
                        .map(|d| d.spring_regrow_factor)
                        .unwrap_or(0.0);
                    let r: f32 = rng.gen();
                    if let Some(spawn) = respawn_from_original_roll_ex3(
                        orig,
                        years,
                        r,
                        spring_f,
                        cur,
                        org,
                        decay_knobs.grow_back_plants_increase_if_low_population,
                        decay_knobs.grow_back_original_plants_factor,
                    ) {
                        world.set_object(x, y, spawn);
                        let ca = LongTermState::count_as_of(content, spawn);
                        long_term.bump_current(ca, 1);
                        changes.push(LongTermChange {
                            x,
                            y,
                            object_id: spawn,
                            floor_id,
                        });
                        continue;
                    }
                }
            }

            // Haxe RespawnObjects: rare chance to spawn original near its home tile.
            if season_is_spring {
                let orig = long_term
                    .original_objects
                    .get(&(x, y))
                    .copied()
                    .unwrap_or(0);
                if orig > 0 {
                    let ca = LongTermState::count_as_of(content, orig);
                    let cur = long_term.current_counts.get(&ca).copied().unwrap_or(0);
                    let orgc = long_term.original_counts.get(&ca).copied().unwrap_or(0);
                    let r_resp: f32 = rng.gen();
                    if should_try_respawn_object_ex(
                        orig,
                        r_resp,
                        cur,
                        orgc,
                        decay_knobs.obj_respawn_chance,
                    ) {
                        try_spawn_object_near(
                            world,
                            content,
                            long_term,
                            x,
                            y,
                            orig,
                            rng,
                            &mut changes,
                        );
                    }
                }
            }

            if season_is_spring {
                try_spring_stuff(world, content, long_term, x, y, years, rng, &mut changes);
            }

            // Haxe: DecayFloor then DecayObject on same tile (no continue after floor).
            if floor_id != 0 {
                try_decay_floor(
                    world,
                    content,
                    long_term,
                    x,
                    y,
                    years,
                    rng,
                    &mut changes,
                    decay_knobs.floor_decay_chance,
                );
            }

            // GrowNewPlantsFromExistingFactor: neighbor spawn from a living plant.
            if season_is_spring {
                let living = world.get_object(x, y);
                if living != 0 {
                    try_offspring_from_existing(
                        world,
                        content,
                        long_term,
                        x,
                        y,
                        years,
                        rng,
                        &mut changes,
                        decay_knobs,
                    );
                }
            }

            let obj_id = world.get_object(x, y);
            if obj_id != 0 {
                try_decay_object(
                    world,
                    content,
                    long_term,
                    x,
                    y,
                    years,
                    rng,
                    &mut changes,
                    decay_knobs,
                );
                try_decay_contained(
                    world,
                    content,
                    x,
                    y,
                    rng,
                    &mut changes,
                    decay_knobs,
                );
            }

            let obj_id = world.get_object(x, y);
            if obj_id != 0 {
                try_align_wall(world, content, x, y, &mut changes);
            }

            let obj_id = world.get_object(x, y);
            if obj_id != 0 {
                try_clear_held_on_ground(world, content, x, y, obj_id, rng, &mut changes);
                try_delete_in_water(world, content, long_term, x, y, obj_id, &mut changes);
            }
        }
    }

    changes
}

/// Haxe `RespawnOrDecayPlant` non-multi-use spring: offspring from a living plant.
// Haxe: TimeHelper L1326–1344 GrowNewPlantsFromExistingFactor
fn try_offspring_from_existing(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    x: i32,
    y: i32,
    years: f32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
    knobs: DecayChanceKnobs,
) {
    let obj_id = world.get_object(x, y);
    if obj_id == 0 {
        return;
    }
    let base = content.resolve_base_id(obj_id);
    let Some(def) = content.get(base) else {
        return;
    };
    if def.num_uses > 1 {
        return;
    }
    if def.spring_regrow_factor <= 0.0 {
        return;
    }
    let spawn_as = if def.counts_or_grows_as > 0 {
        def.counts_or_grows_as
    } else {
        base
    };
    let current = long_term.current_counts.get(&spawn_as).copied().unwrap_or(0);
    let original = long_term.original_counts.get(&spawn_as).copied().unwrap_or(0);
    if current >= original {
        return;
    }
    let mut factor = if knobs.grow_new_from_existing_factor.is_finite()
        && knobs.grow_new_from_existing_factor > 0.0
    {
        knobs.grow_new_from_existing_factor
    } else {
        GROW_NEW_FROM_EXISTING_FACTOR
    };
    if original > 0 && (current as f32) < (original as f32) / 2.0 {
        let boost = if knobs.grow_back_plants_increase_if_low_population.is_finite()
            && knobs.grow_back_plants_increase_if_low_population > 0.0
        {
            knobs.grow_back_plants_increase_if_low_population
        } else {
            GROW_BACK_LOW_POP_BOOST
        };
        factor *= boost;
    }
    let chance = (years * def.spring_regrow_factor * factor).clamp(0.0, 1.0);
    if rng.gen::<f32>() >= chance {
        return;
    }
    try_spawn_object_near(
        world, content, long_term, x, y, spawn_as, rng, changes,
    );
}

/// Haxe `SpawnObject` — try place `obj_id` near (cx,cy) within SPAWN_NEAR_DIST.
fn try_spawn_object_near(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    cx: i32,
    cy: i32,
    obj_id: i32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    let base = content.resolve_base_id(obj_id);
    let def = content.get(base);
    let biomes = def.map(|d| d.biomes.as_slice()).unwrap_or(&[]);
    for _ in 0..SPAWN_NEAR_TRIES {
        let ox = rng.gen_range(-SPAWN_NEAR_DIST..=SPAWN_NEAR_DIST);
        let oy = rng.gen_range(-SPAWN_NEAR_DIST..=SPAWN_NEAR_DIST);
        let candidate = spawn_near_candidate(
            cx,
            cy,
            SPAWN_NEAR_DIST,
            ox,
            oy,
            |x, y| {
                if !in_bounds(world, x, y) {
                    return 1;
                }
                world.get_object(x, y)
            },
            |x, y| {
                if !in_bounds(world, x, y) {
                    return 1;
                }
                world.get_floor(x, y) as i32
            },
            |x, y| {
                if !in_bounds(world, x, y) {
                    return false;
                }
                if biomes.is_empty() {
                    return true;
                }
                let b = world.get_biome(x, y) as i32;
                biomes.contains(&b)
            },
        );
        let Some((tx, ty)) = candidate else {
            continue;
        };
        // Haxe: current/(original+1) > random → abort
        let ca = LongTermState::count_as_of(content, obj_id);
        let cur = long_term.current_counts.get(&ca).copied().unwrap_or(0);
        let orgc = long_term.original_counts.get(&ca).copied().unwrap_or(0);
        let ratio = cur as f32 / (orgc as f32 + 1.0);
        if ratio > rng.gen::<f32>() {
            return;
        }
        world.set_object(tx, ty, obj_id);
        long_term.bump_current(ca, 1);
        changes.push(LongTermChange {
            x: tx,
            y: ty,
            object_id: obj_id,
            floor_id: world.get_floor(tx, ty) as i32,
        });
        return;
    }
}

/// Resolve original biome for RemoveSnow (Haxe `WorldMap.getOriginalBiomeId`).
///
/// Prefer `LongTermState.original_biomes` (merged from map-time + non-snow first visit).
pub fn resolve_original_biome(long_term: &LongTermState, x: i32, y: i32, fallback: u8) -> u8 {
    long_term
        .original_biomes
        .get(&(x, y))
        .copied()
        .unwrap_or(fallback)
}

fn try_spread_snow(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    tx: i32,
    ty: i32,
    years: f32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    let biome = world.get_biome(tx, ty);
    if biome != BIOME_SNOW && biome != SNOWINGREY {
        return;
    }
    let chance = spread_snow_chance(years);
    let r: f32 = rng.gen();
    if r > chance {
        return;
    }
    let dir = rng.gen_range(0..=3);
    let (mut rx, mut ry) = (tx, ty);
    match dir {
        0 => rx = tx + 1,
        1 => rx = tx - 1,
        2 => ry = ty + 1,
        _ => ry = ty - 1,
    }
    if in_bounds(world, rx, ry) && !tile_is_protected(world, content, rx, ry, rng) {
        world.set_biome(rx, ry, BIOME_SNOW);
    }
    // Diagonal with wall corner gate.
    // Haxe: randomInt(2) → 0..2? Actually randomInt(2) in OpenLife is 0..=2 inclusive often.
    // We keep 0..=2 to match existing port.
    let mut x_off = 1 - rng.gen_range(0..=2);
    let mut y_off = 1 - rng.gen_range(0..=2);
    if is_wall(content, world.get_object(tx + x_off, ty)) {
        x_off = 0;
    }
    if is_wall(content, world.get_object(tx, ty + y_off)) {
        y_off = 0;
    }
    let dx = tx + x_off;
    let dy = ty + y_off;
    if in_bounds(world, dx, dy) && !tile_is_protected(world, content, dx, dy, rng) {
        world.set_biome(dx, dy, BIOME_SNOW);
        // Haxe SpreadSnow stone/flint side-effects on diagonal target when no floor.
        // Haxe: TimeHelper.SpreadSnow
        snow_stone_side_effects(world, content, long_term, tx, ty, dx, dy, rng, changes);
    }
}

/// Haxe SpreadSnow create/decay/move stones under spreading snow (no floor).
fn snow_stone_side_effects(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    let floor_id = world.get_floor(to_x, to_y) as i32;
    if floor_id >= 1 {
        return;
    }
    let from_id = content.resolve_base_id(world.get_object(from_x, from_y));
    let to_id = content.resolve_base_id(world.get_object(to_x, to_y));
    let original_to = long_term
        .original_objects
        .get(&(to_x, to_y))
        .copied()
        .unwrap_or(0);
    let original_to = content.resolve_base_id(original_to);

    // 133 Flint respawn on empty tile that originally had flint.
    if to_id == 0 && original_to == 133 {
        let cur = long_term.current_counts.get(&133).copied().unwrap_or(0);
        let orig = long_term.original_counts.get(&133).copied().unwrap_or(0);
        if rng.gen::<f32>() < 0.05 && cur < orig {
            world.set_object(to_x, to_y, 133);
            long_term.bump_current(133, 1);
            changes.push(LongTermChange {
                x: to_x,
                y: to_y,
                object_id: 133,
                floor_id: 0,
            });
        }
        return;
    }

    // 33 Stone // 34 Sharp Stone // 135 Flint Chip // 848 Hardened Row → decaysTo
    if matches!(to_id, 33 | 34 | 135 | 848) {
        let mut rand = rng.gen::<f32>();
        let cur = long_term.current_counts.get(&to_id).copied().unwrap_or(0);
        let orig = long_term.original_counts.get(&to_id).copied().unwrap_or(0);
        if (cur as f32) < (orig as f32) * 0.8 {
            rand = 1.0; // block decay when underpopulated
        }
        if rand < 0.05 {
            let decays_to = content.get(to_id).map(|d| d.decays_to_obj).unwrap_or(0);
            world.set_object(to_x, to_y, decays_to);
            long_term.bump_current(to_id, -1);
            if decays_to > 0 {
                let nca = LongTermState::count_as_of(content, decays_to);
                long_term.bump_current(nca, 1);
            }
            changes.push(LongTermChange {
                x: to_x,
                y: to_y,
                object_id: decays_to,
                floor_id: 0,
            });
        }
        return;
    }

    // 32 Big Hard Rock adjacent → drop stone 33 on empty tile
    if to_id == 0 && from_id == 32 {
        if rng.gen::<f32>() < 0.05 {
            world.set_object(to_x, to_y, 33);
            long_term.bump_current(33, 1);
            changes.push(LongTermChange {
                x: to_x,
                y: to_y,
                object_id: 33,
                floor_id: 0,
            });
        }
        return;
    }

    // Move Stones: 33 / 34 from source onto empty target
    if to_id == 0 && (from_id == 33 || from_id == 34) {
        if rng.gen::<f32>() < 0.2 {
            world.set_object(from_x, from_y, 0);
            world.set_object(to_x, to_y, 33);
            // census net zero for stone family if both count as 33; sharp 34 may differ.
            let from_ca = LongTermState::count_as_of(content, from_id);
            long_term.bump_current(from_ca, -1);
            long_term.bump_current(33, 1);
            changes.push(LongTermChange {
                x: from_x,
                y: from_y,
                object_id: 0,
                floor_id: world.get_floor(from_x, from_y) as i32,
            });
            changes.push(LongTermChange {
                x: to_x,
                y: to_y,
                object_id: 33,
                floor_id: 0,
            });
        }
    }
}

fn try_remove_snow(
    world: &mut World,
    long_term: &LongTermState,
    tx: i32,
    ty: i32,
    years: f32,
    rng: &mut impl Rng,
) {
    let biome = world.get_biome(tx, ty);
    if biome != BIOME_SNOW {
        return;
    }
    let chance = remove_snow_chance(years);
    if rng.gen::<f32>() > chance {
        return;
    }
    let original = resolve_original_biome(long_term, tx, ty, GREEN);

    // Cardinal neighbor: if neighbor is not snow, restore this tile to original.
    let dir = rng.gen_range(0..=3);
    let (rx, ry) = match dir {
        0 => (tx + 1, ty),
        1 => (tx - 1, ty),
        2 => (tx, ty + 1),
        _ => (tx, ty - 1),
    };
    if in_bounds(world, rx, ry) {
        let nb = world.get_biome(rx, ry);
        if nb != BIOME_SNOW && nb != SNOWINGREY {
            world.set_biome(tx, ty, original);
        }
    }

    // Haxe also diagonal restore (uses original of tx,ty).
    let rx = tx + 1 - rng.gen_range(0..=2);
    let ry = ty + 1 - rng.gen_range(0..=2);
    if in_bounds(world, rx, ry) {
        let nb = world.get_biome(rx, ry);
        if nb != BIOME_SNOW && nb != SNOWINGREY {
            world.set_biome(tx, ty, original);
        }
    }
}

fn tile_is_protected(
    world: &World,
    content: &ContentDb,
    x: i32,
    y: i32,
    rng: &mut impl Rng,
) -> bool {
    let biome = world.get_biome(x, y);
    let obj = world.get_object(x, y);
    // Haxe IsProtected: wall insulation from rValue (double roll); floor from floor rValue.
    let wall_ins = object_insulation(content, obj);
    let floor = world.get_floor(x, y) as i32;
    let floor_ins = floor_insulation(content, floor);
    is_protected_by_insulation(biome, wall_ins, floor_ins, rng.gen(), rng.gen(), rng.gen())
}

fn try_spring_stuff(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    x: i32,
    y: i32,
    years: f32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    let obj_id = world.get_object(x, y);
    if obj_id == 511 {
        if spring_pond_to_goose_roll(years, rng.gen()) {
            let uses = world
                .get_helper(x, y)
                .map(|h| h.uses_remaining)
                .unwrap_or(0);
            if uses > 0 {
                world.set_object_complex(x, y, ComplexObject::with_uses(141, uses));
            } else {
                world.set_object(x, y, 141);
            }
            changes.push(LongTermChange {
                x,
                y,
                object_id: 141,
                floor_id: world.get_floor(x, y) as i32,
            });
        }
        return;
    }
    // Bear Cave 630 → awake 648
    if obj_id == 630 {
        let hungry = long_term.current_counts.get(&631).copied().unwrap_or(0);
        let caves = long_term.original_counts.get(&630).copied().unwrap_or(0);
        if spring_bear_cave_awake_roll(hungry, caves, years, rng.gen()) {
            long_term.bump_current(631, 1);
            world.set_object(x, y, 648);
            changes.push(LongTermChange {
                x,
                y,
                object_id: 648,
                floor_id: world.get_floor(x, y) as i32,
            });
        }
    }
    let _ = content;
}

fn try_decay_floor(
    world: &mut World,
    content: &ContentDb,
    _long_term: &mut LongTermState,
    x: i32,
    y: i32,
    _years: f32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
    floor_decay_chance_knob: f32,
) -> bool {
    let floor_id = world.get_floor(x, y) as i32;
    if floor_id == 0 {
        return false;
    }
    let biome = world.get_biome(x, y);
    let def = content.get(floor_id);
    let decay_factor = def.map(|d| d.decay_factor).unwrap_or(1.0);
    if decay_factor <= 0.0 {
        return false;
    }

    let wall_s = surrounding_wall_strength(
        |tx, ty| is_wall(content, world.get_object(tx, ty)),
        |tx, ty| is_strong_wall(content, world.get_object(tx, ty)),
        x,
        y,
    );
    let floor_s = surrounding_floor_strength(|tx, ty| world.get_floor(tx, ty) > 0, x, y);
    let chance = floor_decay_chance_ex(
        decay_factor,
        biome,
        wall_s,
        floor_s,
        floor_decay_chance_knob,
    );
    if chance <= 0.0 || rng.gen::<f32>() > chance {
        return false;
    }

    // Haxe: decaysToObj == 0 ? 618 : decaysToObj
    let decays_to = def.map(|d| d.decays_to_obj).unwrap_or(0);
    let product = if decays_to == 0 {
        TRASH_PIT_ID
    } else {
        decays_to
    };
    let decays_to_is_floor = content.get(product).map(|d| d.floor).unwrap_or(false);
    let obj_id = world.get_object(x, y);
    let (new_floor, new_obj) = floor_decay_result(floor_id, obj_id, decays_to, decays_to_is_floor);
    world.set_floor(x, y, new_floor as u16);
    if let Some(oid) = new_obj {
        world.set_object(x, y, oid);
    }
    changes.push(LongTermChange {
        x,
        y,
        object_id: world.get_object(x, y),
        floor_id: new_floor,
    });
    true
}

/// Haxe TODO L1479: decay stuff in containers (ids + nested multi-use slots).
fn try_decay_contained(
    world: &mut World,
    content: &ContentDb,
    x: i32,
    y: i32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
    knobs: DecayChanceKnobs,
) {
    let Some(mut helper) = world.get_helper(x, y).cloned() else {
        return;
    };
    let floor_id = world.get_floor(x, y) as i32;
    let biome = world.get_biome(x, y);
    let mut changed = false;
    for slot in helper.slots.iter_mut() {
        if decay_nested_helper(slot, content, floor_id, biome, rng, knobs) {
            changed = true;
        }
    }
    for id in helper.contained.iter_mut() {
        if *id == 0 {
            continue;
        }
        if let Some(new_id) = decay_contained_id(*id, content, floor_id, biome, rng, knobs) {
            *id = new_id;
            changed = true;
        }
    }
    if changed {
        let obj_id = helper.base_id;
        world.set_object_complex(x, y, helper);
        changes.push(LongTermChange {
            x,
            y,
            object_id: obj_id,
            floor_id,
        });
    }
}

fn decay_nested_helper(
    h: &mut NestedHelper,
    content: &ContentDb,
    floor_id: i32,
    biome: u8,
    rng: &mut impl Rng,
    knobs: DecayChanceKnobs,
) -> bool {
    let mut changed = false;
    for c in h.contained.iter_mut() {
        if decay_nested_helper(c, content, floor_id, biome, rng, knobs) {
            changed = true;
        }
    }
    if h.id == 0 {
        return changed;
    }
    let base_id = content.resolve_base_id(h.id);
    let def = content.get(base_id);
    let num_uses = def.map(|d| d.num_uses).unwrap_or(1);
    let Some(chance) = contained_object_decay_chance(content, h.id, floor_id, biome, knobs) else {
        return changed;
    };
    if rng.gen::<f32>() > chance {
        return changed;
    }
    if num_uses > 1 && h.uses_remaining > 1 {
        h.uses_remaining -= 1;
        return true;
    }
    let decays_to = def.map(|d| d.decays_to_obj).unwrap_or(0);
    let is_perm = def.map(|d| d.permanent).unwrap_or(false);
    h.id = resolve_object_decay_to(decays_to, is_perm);
    h.uses_remaining = 0;
    true
}

fn decay_contained_id(
    obj_id: i32,
    content: &ContentDb,
    floor_id: i32,
    biome: u8,
    rng: &mut impl Rng,
    knobs: DecayChanceKnobs,
) -> Option<i32> {
    let Some(chance) = contained_object_decay_chance(content, obj_id, floor_id, biome, knobs) else {
        return None;
    };
    if rng.gen::<f32>() > chance {
        return None;
    }
    let base_id = content.resolve_base_id(obj_id);
    let def = content.get(base_id);
    let decays_to = def.map(|d| d.decays_to_obj).unwrap_or(0);
    let is_perm = def.map(|d| d.permanent).unwrap_or(false);
    Some(resolve_object_decay_to(decays_to, is_perm))
}

fn contained_object_decay_chance(
    content: &ContentDb,
    obj_id: i32,
    floor_id: i32,
    biome: u8,
    knobs: DecayChanceKnobs,
) -> Option<f32> {
    let base_id = content.resolve_base_id(obj_id);
    let def = content.get(base_id);
    let decay_factor = decay_factor_for_object(
        base_id,
        def.map(|d| d.decay_factor).unwrap_or(1.0),
        knobs.animal_decay_factor,
    );
    let inp = ObjectDecayInput {
        decay_factor,
        crafting_steps: def.map(|d| d.crafting_steps.max(0)).unwrap_or(0),
        floor_id,
        biome,
        is_wall: is_wall(content, obj_id),
        is_permanent: def.map(|d| d.permanent).unwrap_or(false),
        is_food: def.map(|d| d.food_value > 0).unwrap_or(false),
        is_clothing: def.map(|d| d.is_clothing()).unwrap_or(false),
        is_no_bone_grave: is_no_bone_grave(
            obj_id,
            def.map(|d| d.description.as_str()).unwrap_or(""),
        ),
        contains_something: false,
        decays_to_obj: def.map(|d| d.decays_to_obj).unwrap_or(0),
        has_active_time_transition: false,
        count_as: LongTermState::count_as_of(content, obj_id),
        current_count: 0,
        original_count: 0,
    };
    object_decay_chance_ex2(&inp, knobs)
}

fn try_decay_object(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    x: i32,
    y: i32,
    _years: f32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
    knobs: DecayChanceKnobs,
) {
    let obj_id = world.get_object(x, y);
    if obj_id == 0 || !can_object_respawn(obj_id) {
        return;
    }
    let base_id = content.resolve_base_id(obj_id);
    let def = content.get(base_id);
    let floor_id = world.get_floor(x, y) as i32;
    let biome = world.get_biome(x, y);
    let count_as = LongTermState::count_as_of(content, obj_id);
    let current = long_term
        .current_counts
        .get(&count_as)
        .copied()
        .unwrap_or(0);
    let original = long_term
        .original_counts
        .get(&count_as)
        .copied()
        .unwrap_or(0);
    let contains = world
        .get_helper(x, y)
        .map(|h| !h.contained.is_empty())
        .unwrap_or(false);
    let has_ttc = world
        .get_helper(x, y)
        .map(|h| h.time_to_change > 0.0)
        .unwrap_or(false);
    let is_food = def.map(|d| d.food_value > 0).unwrap_or(false);
    let is_perm = def.map(|d| d.permanent).unwrap_or(false);
    let is_clothing = def.map(|d| d.is_clothing()).unwrap_or(false);
    let wall = is_wall(content, obj_id);
    // Content-driven decay factor / product; live AnimalDecayFactor for patched animal ids.
    let decay_factor = decay_factor_for_object(
        base_id,
        def.map(|d| d.decay_factor).unwrap_or(1.0),
        knobs.animal_decay_factor,
    );
    let decays_to = def.map(|d| d.decays_to_obj).unwrap_or(0);
    let crafting_steps = def.map(|d| d.crafting_steps.max(0)).unwrap_or(0);

    let inp = ObjectDecayInput {
        decay_factor,
        crafting_steps,
        floor_id,
        biome,
        is_wall: wall,
        is_permanent: is_perm,
        is_food,
        is_clothing,
        is_no_bone_grave: is_no_bone_grave(
            obj_id,
            def.map(|d| d.description.as_str()).unwrap_or(""),
        ),
        contains_something: contains,
        decays_to_obj: decays_to,
        has_active_time_transition: has_ttc,
        count_as,
        current_count: current,
        original_count: original,
    };
    let Some(chance) = object_decay_chance_ex2(&inp, knobs) else {
        return;
    };
    if rng.gen::<f32>() > chance {
        return;
    }

    // Haxe TODO L1479: decay stuff with number of uses > 1 (decrement, don't replace id)
    let num_uses = def.map(|d| d.num_uses).unwrap_or(1);
    let uses = world
        .get_helper(x, y)
        .map(|h| h.uses_remaining)
        .unwrap_or(num_uses);
    if num_uses > 1 && uses > 1 {
        let next = uses - 1;
        if let Some(mut helper) = world.get_helper(x, y).cloned() {
            helper.uses_remaining = next;
            world.set_object_complex(x, y, helper);
        } else {
            world.set_object_complex(x, y, ComplexObject::with_uses(obj_id, next));
        }
        changes.push(LongTermChange {
            x,
            y,
            object_id: obj_id,
            floor_id,
        });
        return;
    }

    let new_id = resolve_object_decay_to(decays_to, is_perm);
    // Spill contained beyond new slots.
    if let Some(mut helper) = world.get_helper(x, y).cloned() {
        let slots = content.get(new_id).map(|d| d.num_slots).unwrap_or(0);
        while helper.contained.len() as i32 > slots {
            if let Some(c) = helper.contained.pop() {
                place_decay_spill(world, content, x, y, c, rng, changes);
            }
        }
        helper.base_id = new_id;
        if new_id == 0 {
            world.set_object(x, y, 0);
        } else {
            world.set_object_complex(x, y, helper);
        }
    } else {
        world.set_object(x, y, new_id);
    }
    long_term.bump_current(count_as, -1);
    if new_id > 0 {
        let nca = LongTermState::count_as_of(content, new_id);
        long_term.bump_current(nca, 1);
    }
    changes.push(LongTermChange {
        x,
        y,
        object_id: new_id,
        floor_id,
    });
}

fn try_align_wall(
    world: &mut World,
    content: &ContentDb,
    x: i32,
    y: i32,
    changes: &mut Vec<LongTermChange>,
) {
    let obj_id = world.get_object(x, y);
    let left = is_wall(content, world.get_object(x - 1, y));
    let right = is_wall(content, world.get_object(x + 1, y));
    let north = is_wall(content, world.get_object(x, y + 1));
    let south = is_wall(content, world.get_object(x, y - 1));
    let Some(new_id) = align_wall_id(obj_id, left, right, north, south) else {
        return;
    };
    if new_id == obj_id {
        return;
    }
    if let Some(mut h) = world.get_helper(x, y).cloned() {
        h.base_id = new_id;
        world.set_object_complex(x, y, h);
    } else {
        world.set_object(x, y, new_id);
    }
    changes.push(LongTermChange {
        x,
        y,
        object_id: new_id,
        floor_id: world.get_floor(x, y) as i32,
    });
}

fn try_clear_held_on_ground(
    world: &mut World,
    content: &ContentDb,
    x: i32,
    y: i32,
    obj_id: i32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    // Haxe: transition (objId, -1); if unstuck list or newActor==rope.
    let Some(tr) = content.find_transition(obj_id, -1) else {
        return;
    };
    if !UNSTUCK_OBJECT_IDS.contains(&obj_id) && tr.new_actor_id != ROPE_ID {
        return;
    }
    let new_target = tr.new_target_id;
    world.set_object(x, y, new_target);
    if tr.new_actor_id > 0 {
        // Haxe: TimeHelper L1568 PlaceObject(tx, ty, newActor)
        place_decay_spill(world, content, x, y, tr.new_actor_id, rng, changes);
    }
    changes.push(LongTermChange {
        x,
        y,
        object_id: new_target,
        floor_id: world.get_floor(x, y) as i32,
    });
}

fn try_delete_in_water(
    world: &mut World,
    content: &ContentDb,
    long_term: &mut LongTermState,
    x: i32,
    y: i32,
    obj_id: i32,
    changes: &mut Vec<LongTermChange>,
) {
    if !WATER_DELETE_IDS.contains(&obj_id) {
        return;
    }
    let biome = world.get_biome(x, y);
    if biome != RIVER && biome != PASSABLE_RIVER && biome != OCEAN {
        return;
    }
    world.set_object(x, y, 0);
    let ca = LongTermState::count_as_of(content, obj_id);
    long_term.bump_current(ca, -1);
    changes.push(LongTermChange {
        x,
        y,
        object_id: 0,
        floor_id: world.get_floor(x, y) as i32,
    });
}

/// Haxe `WorldMap.PlaceObject(x, y, contained)` after DecayObject slot overflow.
// TIME-DECAY-PLACE
fn place_decay_spill(
    world: &mut World,
    content: &ContentDb,
    x: i32,
    y: i32,
    obj_id: i32,
    rng: &mut impl Rng,
    changes: &mut Vec<LongTermChange>,
) {
    if obj_id <= 0 {
        return;
    }
    let Some((px, py)) =
        crate::place_object::place_object_on_world(world, content, x, y, obj_id, rng)
    else {
        return;
    };
    changes.push(LongTermChange {
        x: px,
        y: py,
        object_id: obj_id,
        floor_id: world.get_floor(px, py) as i32,
    });
}

fn in_bounds(world: &World, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < world.width_tiles && y < world.height_tiles
}

fn is_no_bone_grave(obj_id: i32, description: &str) -> bool {
    // Bone graves: 87,88,89,356,357 — no-bone graves are graves that are not those.
    const BONE: &[i32] = &[87, 88, 89, 356, 357];
    if BONE.contains(&obj_id) {
        return false;
    }
    description.contains("origGrave")
}

// Silence unused desert constant warning if not referenced.
#[allow(dead_code)]
fn _biome_desert() -> u8 {
    BIOME_DESERT
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};
    use ol_world::World;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn biome_decay_factors_match_haxe_cases() {
        assert!((biome_decay_factor(SWAMP) - 2.0).abs() < 1e-6);
        assert!((biome_decay_factor(OCEAN) - 5.0).abs() < 1e-6);
        assert!((biome_decay_factor(SNOWINGREY) - 3.0).abs() < 1e-6);
        assert!((biome_decay_factor(GREEN) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn floor_strength_and_chance() {
        let f = surrounding_floor_strength(|x, y| x == 0 && y == 0 || x == 1 && y == 0, 0, 0);
        assert!((f - 2.0).abs() < 1e-6); // self + east
        let sf = floor_decay_strength_factor(0.0, 4.0);
        assert!((sf - 0.25).abs() < 1e-6);
        assert_eq!(floor_decay_strength_factor(6.0, 0.0), 0.0);
        let c = floor_decay_chance(1.0, GREEN, 0.0, 1.0);
        assert!(c > 0.0);
        assert!(c < FLOOR_DECAY_CHANCE * 2.0);
    }

    #[test]
    fn floor_decay_chance_ex_live_override() {
        let c = floor_decay_chance(1.0, GREEN, 0.0, 1.0);
        let c2 = floor_decay_chance_ex(1.0, GREEN, 0.0, 1.0, FLOOR_DECAY_CHANCE * 2.0);
        assert!((c2 - c * 2.0).abs() < 1e-12);
        let off = floor_decay_chance_ex(1.0, GREEN, 0.0, 1.0, 0.0);
        assert_eq!(off, 0.0);
    }

    #[test]
    fn decay_factor_for_object_uses_live_animal_knob() {
        // Haxe PatchObjectData: cow 1458 / wolf 418 get AnimalDecayFactor
        assert!((decay_factor_for_object(1458, 1.0, 0.2) - 0.2).abs() < 1e-6);
        assert!((decay_factor_for_object(418, 0.05, 0.01) - 0.01).abs() < 1e-6);
        // Non-animal ids keep content decay_factor
        assert!((decay_factor_for_object(885, 0.2, 0.01) - 0.2).abs() < 1e-6);
        assert!(
            (decay_factor_for_object(1458, 1.0, f32::NAN) - ol_content::ANIMAL_DECAY_FACTOR).abs()
                < 1e-6
        );
    }

    #[test]
    fn object_decay_blocked_on_floor_non_wall() {
        let inp = ObjectDecayInput {
            decay_factor: 1.0,
            crafting_steps: 0,
            floor_id: 884,
            biome: GREEN,
            is_wall: false,
            is_permanent: false,
            is_food: false,
            is_clothing: false,
            is_no_bone_grave: false,
            contains_something: false,
            decays_to_obj: 0,
            has_active_time_transition: false,
            count_as: 33,
            current_count: 100,
            original_count: 100,
        };
        assert!(object_decay_chance(&inp).is_none());
    }

    #[test]
    fn object_decay_blocked_when_decay_factor_nonpositive() {
        // Haxe DecayObject: `if (objData.decayFactor <= 0) return;`
        // Wells / oil / vein / mine / rig get decayFactor=-1 in PatchObjectData.
        let mut inp = ObjectDecayInput {
            decay_factor: -1.0,
            crafting_steps: 0,
            floor_id: 0,
            biome: GREEN,
            is_wall: false,
            is_permanent: true,
            is_food: false,
            is_clothing: false,
            is_no_bone_grave: false,
            contains_something: false,
            decays_to_obj: 0,
            has_active_time_transition: false,
            count_as: 663,
            current_count: 100,
            original_count: 100,
        };
        assert!(object_decay_chance(&inp).is_none());
        inp.decay_factor = 0.0;
        assert!(object_decay_chance(&inp).is_none());
        inp.decay_factor = 1.0;
        assert!(object_decay_chance(&inp).is_some());
    }

    #[test]
    fn object_decay_population_gate() {
        let mut inp = ObjectDecayInput {
            decay_factor: 1.0,
            crafting_steps: 0,
            floor_id: 0,
            biome: GREEN,
            is_wall: false,
            is_permanent: false,
            is_food: false,
            is_clothing: false,
            is_no_bone_grave: false,
            contains_something: false,
            decays_to_obj: 0,
            has_active_time_transition: false,
            count_as: 33,
            current_count: 50,
            original_count: 100,
        };
        assert!(object_decay_chance(&inp).is_none());
        inp.current_count = 80;
        assert!(object_decay_chance(&inp).is_some());
    }

    #[test]
    fn format_object_count_line_haxe_shape() {
        // Haxe: `Count object: [${key}] ${objData.description}: ${current} original: ${original}`
        let line = format_object_count_line(33, "Wild Gooseberry Bush", 12, 20);
        assert_eq!(
            line,
            "Count object: [33] Wild Gooseberry Bush: 12 original: 20"
        );
        let empty_desc = format_object_count_line(1, "", 0, 5);
        assert!(empty_desc.contains("object"), "line={empty_desc}");
    }

    #[test]
    fn format_object_counts_lines_sorted_current_keys_only() {
        let mut current = HashMap::new();
        current.insert(40, 3);
        current.insert(10, 1);
        let mut original = HashMap::new();
        original.insert(40, 5);
        original.insert(10, 2);
        original.insert(99, 7); // Haxe: originals-only not listed (current.keys only)
        let lines = format_object_counts_lines(&current, &original, &mut |id| match id {
            10 => "A".into(),
            40 => "B".into(),
            _ => String::new(),
        });
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("Count object: [10]"),
            "l0={}",
            lines[0]
        );
        assert!(lines[0].contains("original: 2"), "l0={}", lines[0]);
        assert!(
            lines[1].starts_with("Count object: [40]"),
            "l1={}",
            lines[1]
        );
        assert!(!lines.iter().any(|l| l.contains("[99]")));
    }

    #[test]
    fn write_object_counts_roundtrip_disk() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut state = LongTermState::default();
        state.current_counts.insert(33, 4);
        state.original_counts.insert(33, 10);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ol_objcounts_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(DEFAULT_OBJECT_COUNTS_FILE);
        state
            .write_object_counts(&path, |id| {
                if id == 33 {
                    "Gooseberry".into()
                } else {
                    String::new()
                }
            })
            .unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("Count object: [33] Gooseberry: 4 original: 10"),
            "body={body}"
        );
        assert!(body.ends_with('\n'));
        assert_eq!(haxe_object_counts_slot_filename(3), "ObjectCounts3.txt");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn count_objects_from_world_ground_and_counts_or_grows() {
        let mut db = ContentDb::default();
        db.objects.insert(
            942,
            ObjectDef {
                counts_or_grows_as: 3961,
                ..ObjectDef::empty(942)
            },
        );
        db.objects.insert(3961, ObjectDef::empty(3961));
        db.objects.insert(33, ObjectDef::empty(33));
        let mut world = World::new(4, 4, false);
        world.set_object(0, 0, 33);
        world.set_object(1, 0, 33);
        world.set_object(2, 0, 942);
        let ground = count_objects_from_world(&world, &db, false);
        assert_eq!(ground.get(&33), Some(&2));
        assert_eq!(ground.get(&3961), Some(&1));
        assert!(ground.get(&942).is_none());
    }

    #[test]
    fn count_objects_from_world_includes_contained_nest() {
        let mut db = ContentDb::default();
        db.objects.insert(391, ObjectDef::empty(391)); // basket
        db.objects.insert(33, ObjectDef::empty(33));
        db.objects.insert(40, ObjectDef::empty(40));
        db.objects.insert(999, ObjectDef::empty(999));
        let mut world = World::new(4, 4, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![33, 40];
        h.nested = vec![vec![999], vec![]];
        world.set_object_complex(1, 1, h);
        // Ground only: basket base
        let ground = count_objects_from_world(&world, &db, false);
        assert_eq!(ground.get(&391), Some(&1));
        assert!(ground.get(&33).is_none());
        // With nest: + contained 33,40 + sub 999
        let full = count_objects_from_world(&world, &db, true);
        assert_eq!(full.get(&391), Some(&1));
        assert_eq!(full.get(&33), Some(&1));
        assert_eq!(full.get(&40), Some(&1));
        assert_eq!(full.get(&999), Some(&1));
    }

    #[test]
    fn seed_then_snapshot_matches_ground_census() {
        let mut db = ContentDb::default();
        db.objects.insert(33, ObjectDef::empty(33));
        let mut world = World::new(3, 3, false);
        world.set_object(0, 0, 33);
        world.set_object(1, 1, 33);
        let mut lt = LongTermState::default();
        assert!(!lt.counts_ready);
        // Before seed: empty snapshot / empty text
        let empty_snap = crate::object_counts_share::ObjectCountsSnapshot::from_long_term(&lt);
        assert!(!empty_snap.counts_ready);
        assert!(empty_snap.current_counts.is_empty());
        let empty_text = format_object_counts_text(
            &empty_snap.current_counts,
            &empty_snap.original_counts,
            |_| "x".into(),
        );
        assert!(empty_text.is_empty());

        lt.seed_from_world_if_needed(&world, &db);
        assert!(lt.counts_ready);
        assert_eq!(lt.current_counts.get(&33), Some(&2));
        assert_eq!(lt.original_counts.get(&33), Some(&2));
        let snap = crate::object_counts_share::ObjectCountsSnapshot::from_long_term(&lt);
        let text = format_object_counts_text(&snap.current_counts, &snap.original_counts, |id| {
            if id == 33 {
                "Gooseberry".into()
            } else {
                String::new()
            }
        });
        assert!(
            text.contains("Count object: [33] Gooseberry: 2 original: 2"),
            "text={text}"
        );
    }

    #[test]
    fn update_object_counts_corrects_player_drift_and_adds_nest() {
        let mut db = ContentDb::default();
        db.objects.insert(33, ObjectDef::empty(33));
        db.objects.insert(391, ObjectDef::empty(391));
        db.objects.insert(40, ObjectDef::empty(40));
        let mut world = World::new(4, 4, false);
        world.set_object(0, 0, 33);
        let mut lt = LongTermState::default();
        lt.seed_from_world_if_needed(&world, &db);
        assert_eq!(lt.current_counts.get(&33), Some(&1));

        // Simulated player place without bump_current (drift gap).
        world.set_object(2, 2, 33);
        assert_eq!(
            lt.current_counts.get(&33),
            Some(&1),
            "stale counts before recompute"
        );
        let full = count_objects_from_world(&world, &db, true);
        assert_eq!(full.get(&33), Some(&2));
        assert_ne!(
            lt.current_counts.get(&33).copied().unwrap_or(0),
            full.get(&33).copied().unwrap_or(0),
            "documents drift until update_object_counts"
        );

        // Container nest: basket with 40 inside
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![40];
        world.set_object_complex(1, 0, h);

        lt.update_object_counts(&world, &db);
        assert_eq!(lt.current_counts.get(&33), Some(&2));
        assert_eq!(lt.current_counts.get(&391), Some(&1));
        assert_eq!(lt.current_counts.get(&40), Some(&1));
        // Original still from seed (ground at seed time only)
        assert_eq!(lt.original_counts.get(&33), Some(&1));
    }

    #[test]
    fn ensure_counts_for_dump_seeds_and_includes_nest() {
        let mut db = ContentDb::default();
        db.objects.insert(391, ObjectDef::empty(391));
        db.objects.insert(33, ObjectDef::empty(33));
        let mut world = World::new(3, 3, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![33];
        world.set_object_complex(0, 0, h);
        let mut lt = LongTermState::default();
        lt.ensure_counts_for_dump(&world, &db);
        assert!(lt.counts_ready);
        assert_eq!(lt.current_counts.get(&391), Some(&1));
        assert_eq!(lt.current_counts.get(&33), Some(&1));
        // original from ground-only seed includes basket, not nest
        assert_eq!(lt.original_counts.get(&391), Some(&1));
        assert!(lt.original_counts.get(&33).is_none());
    }

    #[test]
    fn should_update_object_counts_haxe_tick_offset() {
        // Haxe: (tick + 20) % 600 == 0 → tick = 580, 1180, ...
        assert!(should_update_object_counts(580));
        assert!(should_update_object_counts(1180));
        assert!(!should_update_object_counts(0));
        assert!(!should_update_object_counts(600));
        assert!(!should_update_object_counts(20));
    }

    #[test]
    fn wall_align_horizontal_vertical_corner() {
        // left+right walls, no N/S → horizontal
        assert_eq!(
            wall_orientation(true, true, false, false, false),
            WallOrientation::Horizontal
        );
        assert_eq!(
            wall_orientation(false, false, true, true, false),
            WallOrientation::Vertical
        );
        assert_eq!(
            wall_orientation(true, false, true, false, false),
            WallOrientation::Corner
        );
        // Stone wall 885 corner family
        assert_eq!(
            align_wall_id(885, true, true, false, false),
            Some(887) // horizontal
        );
        assert_eq!(
            align_wall_id(885, false, false, true, true),
            Some(886) // vertical
        );
        // Fence mode: only N/S clear → horizontal
        assert_eq!(
            align_wall_id(551, true, false, false, false),
            Some(550) // horizontal (fence)
        );
    }

    #[test]
    fn respawn_original_and_bear_cave() {
        assert_eq!(respawn_from_original_roll(50, 60.0, 0.0), Some(50));
        assert_eq!(respawn_from_original_roll(50, 0.001, 0.9), None);
        assert_eq!(respawn_from_original_roll(99, 100.0, 0.0), None);
        assert!(spring_bear_cave_awake_roll(0, 100, 2.0, 0.0));
        assert!(!spring_bear_cave_awake_roll(20, 100, 2.0, 0.0)); // 20 >= 10
                                                                  // Generic GrowBackOriginalPlants: spring_regrow > 0 + under population.
        assert_eq!(
            respawn_from_original_roll_ex(777, 60.0, 0.0, 1.0, 1.0, 10.0),
            Some(777)
        );
        // At/above original pop → none.
        assert_eq!(
            respawn_from_original_roll_ex(777, 60.0, 0.0, 1.0, 10.0, 10.0),
            None
        );
        assert!(should_try_respawn_object(50, 0.0, 0, 10));
        assert!(!should_try_respawn_object(50, 1.0, 0, 10));
        assert!(!should_try_respawn_object(0, 0.0, 0, 10));
        assert!(should_try_respawn_object_ex(50, 0.005, 0, 10, 0.01));
        assert!(!should_try_respawn_object_ex(50, 0.02, 0, 10, 0.01));
        // Low-pop boost 2 vs 4 doubles chance (base = 6/60 * spring_regrow = 0.1).
        assert_eq!(
            respawn_from_original_roll_ex2(777, 6.0, 0.3, 1.0, 1.0, 10.0, 2.0),
            None
        );
        assert_eq!(
            respawn_from_original_roll_ex2(777, 6.0, 0.3, 1.0, 1.0, 10.0, 4.0),
            Some(777)
        );
        // Live GrowBackOriginalPlantsFactor 0.02: years=60, spring=1 → chance 0.02.
        assert_eq!(
            respawn_from_original_roll_ex3(777, 60.0, 0.5, 1.0, 0.0, 0.0, 2.0, 0.02),
            None
        );
        assert_eq!(
            respawn_from_original_roll_ex3(777, 60.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.02),
            Some(777)
        );
        // Doubling 0.02 vs 0.04 at rand just above 0.02.
        assert_eq!(
            respawn_from_original_roll_ex3(777, 60.0, 0.03, 1.0, 0.0, 0.0, 2.0, 0.02),
            None
        );
        assert_eq!(
            respawn_from_original_roll_ex3(777, 60.0, 0.03, 1.0, 0.0, 0.0, 2.0, 0.04),
            Some(777)
        );
    }

    #[test]
    fn should_try_respawn_object_ex_live_chance() {
        assert!(should_try_respawn_object(50, 0.0, 0, 10));
        assert!(!should_try_respawn_object(50, 1.0, 0, 10));
        assert!(should_try_respawn_object_ex(50, 0.005, 0, 10, 0.01));
        assert!(!should_try_respawn_object_ex(50, 0.02, 0, 10, 0.01));
    }

    #[test]
    fn spawn_near_candidate_gates() {
        let hit = spawn_near_candidate(
            5,
            5,
            6,
            1,
            0,
            |x, y| {
                if x == 6 && y == 5 {
                    0
                } else if x == 6 && y == 4 {
                    0
                } else {
                    0
                }
            },
            |_x, _y| 0,
            |_x, _y| true,
        );
        assert_eq!(hit, Some((6, 5)));
        let blocked = spawn_near_candidate(5, 5, 6, 0, 0, |_, _| 99, |_, _| 0, |_, _| true);
        assert!(blocked.is_none());
    }

    #[test]
    fn floor_decay_result_paths() {
        assert_eq!(
            floor_decay_result(884, 0, 0, false),
            (0, Some(TRASH_PIT_ID))
        );
        assert_eq!(floor_decay_result(884, 33, 0, false), (0, None));
        assert_eq!(floor_decay_result(884, 0, 881, true), (881, None));
    }
    #[test]
    fn long_term_align_wall_on_map() {
        let mut db = ContentDb::default();
        // No special defs needed for known wall ids.
        let mut world = World::new(20, 250, false); // height 250 → long-term band size 1
                                                    // Place stone walls in a horizontal run at y=0 (band 0).
        world.set_object(5, 0, 885);
        world.set_object(4, 0, 885);
        world.set_object(6, 0, 885);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        let mut rng = StdRng::seed_from_u64(1);
        let changes = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, false, false, false, 10.0, &mut rng,
        );
        assert!(
            changes.iter().any(|c| c.x == 5 && c.object_id == 887) || world.get_object(5, 0) == 887,
            "middle wall should become horizontal 887, got {}",
            world.get_object(5, 0)
        );
        let _ = db;
    }

    #[test]
    fn long_term_delete_hardened_row_in_water() {
        let db = ContentDb::default();
        let mut world = World::new(10, 250, false);
        world.set_biome(1, 0, OCEAN);
        world.set_object(1, 0, 848);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.current_counts.insert(848, 1);
        let mut rng = StdRng::seed_from_u64(2);
        let changes = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, false, false, false, 5.0, &mut rng,
        );
        assert_eq!(world.get_object(1, 0), 0);
        assert!(changes.iter().any(|c| c.x == 1 && c.object_id == 0));
    }

    #[test]
    fn long_term_respawn_milkweed_spring() {
        let db = ContentDb::default();
        let mut world = World::new(10, 250, false);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.original_objects.insert((2, 0), 50);
        // Band 0 (y=0): step % parts == 0. Provide cycle_started so reset yields huge years.
        lt.step = 0;
        lt.cycle_started_sim_time = 1.0;
        // sim_time - cycle_started = 3600 → years = 60 → milkweed chance 1.0
        let mut rng = StdRng::seed_from_u64(3);
        let _ = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, true, false, false, 3601.0, &mut rng,
        );
        assert_eq!(world.get_object(2, 0), 50);
    }

    #[test]
    fn long_term_unstuck_cart() {
        let mut db = ContentDb::default();
        db.transitions.insert(
            (778, -1),
            Transition {
                actor_id: 778,
                target_id: -1,
                new_actor_id: 0,
                new_target_id: 780,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
                hungry_work_cost: 0.0,
                hungry_work_temperature: -1.0,
                coin_cost: 0,
                is_forbidden: false,
            },
        );
        let mut world = World::new(10, 250, false);
        world.set_object(3, 0, 778);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        let mut rng = StdRng::seed_from_u64(4);
        let _ = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, false, false, false, 1.0, &mut rng,
        );
        assert_eq!(world.get_object(3, 0), 780);
    }

    #[test]
    fn object_decay_uses_decay_factor_and_custom_product() {
        // Non-default decay_factor and decays_to product path (pure + resolve).
        let mut inp = ObjectDecayInput {
            decay_factor: 0.2,
            crafting_steps: 0,
            floor_id: 0,
            biome: GREEN,
            is_wall: true,
            is_permanent: true,
            is_food: false,
            is_clothing: false,
            is_no_bone_grave: false,
            contains_something: false,
            decays_to_obj: 1853,
            has_active_time_transition: false,
            count_as: 885,
            current_count: 100,
            original_count: 100,
        };
        let c = object_decay_chance(&inp).expect("chance");
        assert!(c > 0.0);
        // Higher decay_factor → higher chance
        inp.decay_factor = 1.0;
        let c2 = object_decay_chance(&inp).unwrap();
        assert!(c2 > c);
        // Live ObjDecayFactorForPermanentObjs: 0.5 vs default 0.2 doubles perm decay.
        let half = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_permanent: 0.2,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let full = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_permanent: 0.4,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((full - half * 2.0).abs() < 1e-9);
        inp.is_permanent = false;
        inp.is_food = true;
        let food_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_food: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let food_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_food: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((food_live - food_def * 2.0).abs() < 1e-9);
        inp.is_food = false;
        inp.is_wall = false;
        inp.is_clothing = true;
        let cloth_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_clothing: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let cloth_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_clothing: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((cloth_live - cloth_def * 2.0).abs() < 1e-9);
        inp.is_clothing = false;
        inp.is_wall = true;
        let wall_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_walls: 0.2,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let wall_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_for_walls: 0.4,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((wall_live - wall_def * 2.0).abs() < 1e-9);
        inp.is_wall = true;
        inp.is_permanent = true;
        inp.crafting_steps = 10;
        let tech_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_per_tech_level: 10.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let tech_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                obj_decay_factor_per_tech_level: 20.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        // 20/(20+10)=2/3 vs 10/(10+10)=1/2 → live/def = (2/3)/(1/2)=4/3
        assert!((tech_live - tech_def * 4.0 / 3.0).abs() < 1e-6);
        inp.crafting_steps = 0;
        inp.biome = OCEAN;
        let deep_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_deep_water: 5.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let deep_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_deep_water: 10.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((deep_live - deep_def * 2.0).abs() < 1e-9);
        inp.biome = SNOWINGREY;
        let mt_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_mountain: 3.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let mt_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_mountain: 6.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((mt_live - mt_def * 2.0).abs() < 1e-9);
        inp.biome = PASSABLE_RIVER;
        let walk_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_walkable_water: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let walk_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_walkable_water: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((walk_live - walk_def * 2.0).abs() < 1e-9);
        inp.biome = JUNGLE;
        let jungle_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_jungle: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let jungle_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_jungle: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((jungle_live - jungle_def * 2.0).abs() < 1e-9);
        inp.biome = BIOME_BORDER_JUNGLE;
        let border_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_jungle: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let border_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_jungle: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((border_live - border_def * 2.0).abs() < 1e-9);
        inp.biome = SWAMP;
        let swamp_def = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_swamp: 2.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        let swamp_live = object_decay_chance_ex2(
            &inp,
            DecayChanceKnobs {
                decay_factor_in_swamp: 4.0,
                ..DecayChanceKnobs::default()
            },
        )
        .unwrap();
        assert!((swamp_live - swamp_def * 2.0).abs() < 1e-9);
        inp.biome = GREEN;
        assert_eq!(resolve_object_decay_to(1853, true), 1853);
        assert_eq!(resolve_object_decay_to(0, true), TRASH_PIT_ID);
        // crafting_steps tech factor
        inp.decay_factor = 1.0;
        inp.crafting_steps = 58;
        let knife = object_decay_chance(&inp).unwrap();
        inp.crafting_steps = 0;
        let rock = object_decay_chance(&inp).unwrap();
        assert!(knife < rock);
        // SETTINGS-LONG-TAIL: live ObjDecayChance scales chance
        let live = object_decay_chance_ex(&inp, OBJ_DECAY_CHANCE * 3.0).unwrap();
        assert!((live - rock * 3.0).abs() < 1e-12);
        assert!(object_decay_chance_ex(&inp, 0.0).unwrap() == 0.0);
    }

    #[test]
    fn permanent_wall_decays_to_settings_product_not_only_trash() {
        let mut db = ContentDb::default();
        db.objects.insert(
            885,
            ObjectDef {
                permanent: true,
                r_value: 0.9,
                clothing: "n".into(),
                decay_factor: 0.2,
                decays_to_obj: 1853,
                ..ObjectDef::empty(885)
            },
        );
        db.objects.insert(1853, ObjectDef::empty(1853));
        let mut world = World::new(10, 250, false);
        world.set_object(1, 0, 885);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.current_counts.insert(885, 100);
        lt.original_counts.insert(885, 100);
        // Force decay: tiny world band; pump chance by running many steps with high factor
        // Use pure path force via direct try — simulate with decay_factor huge via content.
        if let Some(d) = db.objects.get_mut(&885) {
            d.decay_factor = 1_000_000.0; // guaranteed roll
        }
        let mut rng = StdRng::seed_from_u64(42);
        let _ = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, false, false, false, 1.0, &mut rng,
        );
        assert_eq!(
            world.get_object(1, 0),
            1853,
            "stone wall should decay to cut stones not trash"
        );
    }

    #[test]
    fn count_as_uses_counts_or_grows_as_alias() {
        let mut db = ContentDb::default();
        db.objects.insert(
            942,
            ObjectDef {
                counts_or_grows_as: 3961,
                ..ObjectDef::empty(942)
            },
        );
        assert_eq!(LongTermState::count_as_of(&db, 942), 3961);
        assert_eq!(LongTermState::count_as_of(&db, 33), 33);
    }

    #[test]
    fn remove_snow_restores_stored_original_biome() {
        let db = ContentDb::default();
        let mut world = World::new(10, 250, false);
        // Tile (1,0) is snow; neighbor (2,0) is green plains.
        world.set_biome(1, 0, BIOME_SNOW);
        world.set_biome(2, 0, GREEN);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.time_passed_all_steps = 60.0 * 100.0; // years huge → remove chance ~1
        lt.original_biomes.insert((1, 0), YELLOW); // original was desert/yellow
                                                   // Run many seeds until snow melts (RNG-dependent neighbor pick).
        let mut melted = false;
        for seed in 0..40u64 {
            let mut w = world.clone();
            let mut state = lt.clone();
            let mut rng = StdRng::seed_from_u64(seed);
            let _ = do_world_long_term_time_stuff(
                &mut w, &db, &mut state, false, false, true, 1.0, &mut rng,
            );
            if w.get_biome(1, 0) == YELLOW {
                melted = true;
                break;
            }
            if w.get_biome(1, 0) != BIOME_SNOW {
                assert_eq!(w.get_biome(1, 0), YELLOW);
                melted = true;
                break;
            }
        }
        assert!(
            melted,
            "expected RemoveSnow to restore YELLOW original biome"
        );
    }

    #[test]
    fn is_wall_from_rvalue_and_insulation() {
        let mut db = ContentDb::default();
        db.objects.insert(
            9001,
            ObjectDef {
                r_value: 0.98,
                clothing: "n".into(),
                ..ObjectDef::empty(9001)
            },
        );
        db.objects.insert(
            884,
            ObjectDef {
                floor: true,
                r_value: 0.7,
                clothing: "n".into(),
                ..ObjectDef::empty(884)
            },
        );
        assert!(is_wall(&db, 9001));
        // Haxe floors with rValue still report isWall; insulation uses rValue either way.
        assert!(is_wall(&db, 884));
        assert!((object_insulation(&db, 9001) - 0.98).abs() < 1e-5);
        assert!((floor_insulation(&db, 884) - 0.7).abs() < 1e-5);
        // Double-wall protection
        assert!(is_protected_by_insulation(GREEN, 0.9, 0.0, 0.1, 0.5, 1.0));
    }

    #[test]
    fn floor_decay_does_not_skip_same_tile_object_processing() {
        // After floor decay, object decay/align still runs (no early continue).
        let mut db = ContentDb::default();
        db.objects.insert(
            884,
            ObjectDef {
                floor: true,
                decay_factor: 1_000_000.0,
                decays_to_obj: 881,
                r_value: 0.7,
                ..ObjectDef::empty(884)
            },
        );
        db.objects.insert(
            881,
            ObjectDef {
                floor: true,
                ..ObjectDef::empty(881)
            },
        );
        // Horizontal stone wall that should align when neighbors present.
        db.objects.insert(
            885,
            ObjectDef {
                permanent: true,
                r_value: 0.9,
                decay_factor: 0.0, // no object decay this tick
                ..ObjectDef::empty(885)
            },
        );
        let mut world = World::new(10, 250, false);
        world.set_floor(5, 0, 884);
        world.set_object(4, 0, 885);
        world.set_object(5, 0, 885);
        world.set_object(6, 0, 885);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        let mut rng = StdRng::seed_from_u64(7);
        let _ = do_world_long_term_time_stuff(
            &mut world, &db, &mut lt, false, false, false, 1.0, &mut rng,
        );
        // Floor should have decayed to 881 (another floor) or cleared.
        // Wall at (5,0) should still align to horizontal 887 despite floor decay on same tile.
        assert_eq!(
            world.get_object(5, 0),
            887,
            "wall align must run after floor decay on same tile, got {}",
            world.get_object(5, 0)
        );
    }

    #[test]
    fn snow_flint_respawn_side_effect() {
        // Pure helper path: empty tile with original flint under snow diagonal.
        let mut db = ContentDb::default();
        db.objects.insert(133, ObjectDef::empty(133));
        let mut world = World::new(10, 250, false);
        world.set_biome(1, 0, BIOME_SNOW);
        world.set_biome(2, 0, GREEN);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.original_objects.insert((2, 0), 133);
        lt.original_counts.insert(133, 10);
        lt.current_counts.insert(133, 0);
        let mut changes = Vec::new();
        let mut rng = StdRng::seed_from_u64(99);
        // Call side-effect directly many times.
        let mut spawned = false;
        for _ in 0..80 {
            snow_stone_side_effects(&mut world, &db, &mut lt, 1, 0, 2, 0, &mut rng, &mut changes);
            if world.get_object(2, 0) == 133 {
                spawned = true;
                break;
            }
        }
        assert!(spawned, "expected flint respawn under snow");
    }

    #[test]
    fn tech_factor_slows_high_craft_steps() {
        let knife = decay_tech_factor(58);
        let rock = decay_tech_factor(0);
        assert!(knife < rock);
        assert!((rock - 1.0).abs() < 1e-5);
    }

    #[test]
    fn snow_protect_and_chance() {
        assert!(is_protected_by_insulation(OCEAN, 0.0, 0.0, 1.0, 1.0, 1.0));
        assert!(!is_protected_by_insulation(GREEN, 0.0, 0.0, 1.0, 1.0, 1.0));
        assert!(is_protected_by_insulation(GREEN, 0.9, 0.0, 0.1, 1.0, 1.0));
        assert!(spread_snow_chance(1.0) > 0.0);
        assert!(remove_snow_chance(1.0) > spread_snow_chance(1.0));
    }

    #[test]
    fn can_object_respawn_blacklist() {
        assert!(!can_object_respawn(3030));
        assert!(!can_object_respawn(2285));
        assert!(!can_object_respawn(503));
        assert!(can_object_respawn(33));
        assert!(can_object_respawn(50));
        // Haxe RespawnObjects skips the blacklist even when the roll would fire.
        assert!(!should_try_respawn_object(3030, 0.0, 0, 10));
        assert!(!should_try_respawn_object(2285, 0.0, 0, 10));
        assert!(!should_try_respawn_object(503, 0.0, 0, 10));
        assert!(should_try_respawn_object(50, 0.0, 0, 10));
        assert_eq!(respawn_from_original_roll(3030, 60.0, 0.0), None);
        assert_eq!(respawn_from_original_roll_ex(2285, 60.0, 0.0, 1.0, 0.0, 10.0), None);
        assert_eq!(respawn_from_original_roll_ex(503, 60.0, 0.0, 1.0, 0.0, 10.0), None);
    }

    #[test]
    fn long_term_offspring_from_existing_plant() {
        let mut db = ContentDb::default();
        db.objects.insert(
            50,
            ObjectDef {
                spring_regrow_factor: 1.0,
                num_uses: 0,
                biomes: Vec::new(),
                ..ObjectDef::empty(50)
            },
        );
        let mut world = World::new(16, 250, false);
        world.set_object(8, 8, 50);
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.current_counts.insert(50, 1);
        lt.original_counts.insert(50, 10_000);
        let knobs = DecayChanceKnobs {
            grow_new_from_existing_factor: 1.0,
            ..DecayChanceKnobs::default()
        };
        let mut changes = Vec::new();
        for seed in 0..40u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            try_offspring_from_existing(
                &mut world, &db, &mut lt, 8, 8, 10.0, &mut rng, &mut changes, knobs,
            );
            if !changes.is_empty() {
                break;
            }
        }
        let mut n = 0usize;
        for x in 0..16 {
            for y in 0..16 {
                if world.get_object(x, y) == 50 {
                    n += 1;
                }
            }
        }
        assert!(n >= 2, "living plant should spawn a neighbor, count={n}");
        assert!(!changes.is_empty());
    }

    #[test]
    fn object_def_floor_flag_smoke() {
        let d = ObjectDef {
            floor: true,
            ..ObjectDef::empty(1596)
        };
        assert!(d.is_floor());
    }

    #[test]
    fn long_term_multiuse_decrements_uses_not_id() {
        let mut db = ContentDb::default();
        db.objects.insert(
            9001,
            ObjectDef {
                num_uses: 4,
                decay_factor: 1.0,
                ..ObjectDef::empty(9001)
            },
        );
        let mut world = World::new(10, 250, false);
        world.set_object_complex(1, 0, ComplexObject::with_uses(9001, 4));
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.current_counts.insert(9001, 5);
        lt.original_counts.insert(9001, 5);
        let mut rng = StdRng::seed_from_u64(7);
        let knobs = DecayChanceKnobs {
            obj_decay_chance: 1.0,
            ..DecayChanceKnobs::default()
        };
        let mut changes = Vec::new();
        try_decay_object(
            &mut world, &db, &mut lt, 1, 0, 1.0, &mut rng, &mut changes, knobs,
        );
        assert_eq!(world.get_object(1, 0), 9001);
        assert_eq!(world.get_helper(1, 0).unwrap().uses_remaining, 3);
        assert_eq!(changes.len(), 1);
    }

    /// TIME-DECAY-PLACE: DecayObject PlaceObject spills contained past a full 8-neighbor ring.
    // Haxe: TimeHelper.DecayObject L1787–1789 WorldMap.PlaceObject
    #[test]
    fn long_term_decay_place_object_spills_beyond_eight_neighbors() {
        let mut db = ContentDb::default();
        db.objects.insert(
            9003,
            ObjectDef {
                decay_factor: 1.0,
                decays_to_obj: 9004,
                num_slots: 4,
                ..ObjectDef::empty(9003)
            },
        );
        db.objects.insert(
            9004,
            ObjectDef {
                num_slots: 0,
                ..ObjectDef::empty(9004)
            },
        );
        db.objects.insert(33, ObjectDef::empty(33));
        let mut world = World::new(16, 16, false);
        let mut helper = ComplexObject::new_simple(9003);
        helper.contained = vec![33];
        world.set_object_complex(4, 4, helper);
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
        ] {
            world.set_object(4 + dx, 4 + dy, 20);
        }
        let mut lt = LongTermState::default();
        lt.counts_ready = true;
        lt.current_counts.insert(9003, 5);
        lt.original_counts.insert(9003, 5);
        let mut rng = StdRng::seed_from_u64(11);
        let knobs = DecayChanceKnobs {
            obj_decay_chance: 1.0,
            ..DecayChanceKnobs::default()
        };
        let mut changes = Vec::new();
        try_decay_object(
            &mut world, &db, &mut lt, 4, 4, 1.0, &mut rng, &mut changes, knobs,
        );
        assert_eq!(world.get_object(4, 4), 9004);
        let on_ring = [
            (5, 4),
            (3, 4),
            (4, 5),
            (4, 3),
            (5, 5),
            (3, 3),
            (5, 3),
            (3, 5),
        ]
        .iter()
        .any(|&(x, y)| world.get_object(x, y) == 33);
        assert!(!on_ring, "8-neighbor ring is full");
        let found = (0i32..16).any(|y| {
            (0i32..16).any(|x| {
                let ring = x.abs_diff(4) <= 1 && y.abs_diff(4) <= 1;
                !ring && world.get_object(x, y) == 33
            })
        });
        assert!(found, "DecayObject PlaceObject must search past the occupied ring");
    }

    #[test]
    fn long_term_nested_contained_decays_independently() {
        let mut db = ContentDb::default();
        db.objects.insert(
            9002,
            ObjectDef {
                decay_factor: 1.0,
                decays_to_obj: 0,
                ..ObjectDef::empty(9002)
            },
        );
        let mut world = World::new(10, 250, false);
        let mut helper = ComplexObject::new_simple(100);
        helper.contained.push(9002);
        world.set_object_complex(2, 0, helper);
        let mut rng = StdRng::seed_from_u64(8);
        let knobs = DecayChanceKnobs {
            obj_decay_chance: 1.0,
            ..DecayChanceKnobs::default()
        };
        let mut changes = Vec::new();
        try_decay_contained(&mut world, &db, 2, 0, &mut rng, &mut changes, knobs);
        assert_eq!(world.get_helper(2, 0).unwrap().contained[0], 0);
        assert!(!changes.is_empty());
    }
}
