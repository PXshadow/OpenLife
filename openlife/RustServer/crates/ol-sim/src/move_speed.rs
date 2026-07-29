//! Floor / road / biome move-speed factors (Haxe `MoveHelper.calculateSpeed` subset).
//!
//! Chunks: **S-MOVE** `road_floor_speed` + **S-MOVE-POLISH** + **MOVE-NEST-SPEED** `held_nest_mult`
//! Anchors: `calculateSpeed` floor/biome/held/contain/held-nest/boat/shoes/hitpoints/temp/grave/enemy/AI,
//! `calculateNewMovements.fullPathHasRoad`, `WorldMap.getBiomeSpeed` water-floor override,
//! `WorldMap.isBiomeBlocking`. Strong half-penalty implements Haxe TODO on contained load.

use ol_content::ContentDb;
use ol_world::{
    biome_speed as world_biome_speed, is_biome_blocking, BiomeId, NestedHelper, World, OCEAN,
    PASSABLE_RIVER, RIVER, SNOWINGREY,
};

use crate::move_path::{client_path_deltas_to_steps, step_len, MAX_CLIENT_PATH_STEPS};
use crate::prestige::PrestigeClass;

/// Haxe `ServerSettings.InitialPlayerMoveSpeed` (tiles/s).
pub const INITIAL_PLAYER_MOVE_SPEED: f32 = 3.75;

/// Haxe `ServerSettings.SpeedFactor` (debug global mult; default 1).
pub const SPEED_FACTOR: f32 = 1.0;

/// Haxe `ServerSettings.MinBiomeSpeedFactor` — ocean/mountain floor clamp.
pub const MIN_BIOME_SPEED_FACTOR: f32 = 0.2;

/// Floor counts as road when `speedMult >= 1.01` (Haxe `onRoad` / path road scan).
/// Stone Road 1596 uses `speedMult=1.5`; plain stone floor 884 stays `1.0`.
pub const ROAD_SPEED_THRESHOLD: f32 = 1.01;

/// Haxe `truncMovementSpeedDiff` — off-road path trunc when biome speed deltas exceed this.
pub const TRUNC_MOVEMENT_SPEED_DIFF: f32 = 0.1;

/// Haxe `ServerSettings.MinSpeedReductionPerContainedObj` (upper clamp per contained).
pub const MIN_SPEED_REDUCTION_PER_CONTAINED: f32 = 0.98;

/// Haxe floor of `calculateObjSpeedMult`.
pub const CONTAINED_SPEED_FLOOR: f32 = 0.6;

/// Horse / car threshold: `heldObject.speedMult >= 1.1`.
pub const HORSE_OR_CAR_SPEED_THRESHOLD: f32 = 1.1;

/// Boat-on-land override: `0.5 * InitialPlayerMoveSpeed`.
pub const BOAT_ON_LAND_SPEED_FACTOR: f32 = 0.5;

// --- S-MOVE-POLISH (Haxe ServerSettings + calculateSpeed tail) ---

/// Haxe `ServerSettings.SpeedWithBothShoes`.
pub const SPEED_WITH_BOTH_SHOES: f32 = 1.1;

/// Haxe `ServerSettings.HitpointsSpeedFactor` (0 = disable).
pub const HITPOINTS_SPEED_FACTOR: f32 = 3.0;

/// Haxe `ServerSettings.GrownUpFoodStoreMax` (full hitpoints baseline).
pub const GROWN_UP_FOOD_STORE_MAX: f32 = 20.0;

/// Haxe `ServerSettings.TemperatureSpeedImpact` (1.0 = no temp speed effect).
pub const TEMPERATURE_SPEED_IMPACT: f32 = 1.0;

/// Haxe extreme heat double-impact gate (`heat > 0.98`).
pub const HEAT_DOUBLE_IMPACT_HIGH: f32 = 0.98;

/// Haxe extreme cold double-impact gate (`heat < 0.02`).
pub const HEAT_DOUBLE_IMPACT_LOW: f32 = 0.02;

/// Haxe `ServerSettings.CloseGraveSpeedMali`.
pub const CLOSE_GRAVE_SPEED_MALI: f32 = 0.9;

/// Haxe `ServerSettings.CloseEnemyWithWeaponSpeedFactor`.
pub const CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR: f32 = 0.8;

/// Haxe `ServerSettings.AISpeedFactorSerf`.
pub const AI_SPEED_FACTOR_SERF: f32 = 0.8;

/// Haxe `ServerSettings.AISpeedFactorCommoner`.
pub const AI_SPEED_FACTOR_COMMONER: f32 = 0.9;

/// Haxe `ServerSettings.AISpeedFactorNoble` (+ King/Emperor).
pub const AI_SPEED_FACTOR_NOBLE: f32 = 1.0;

/// Wooden Floor / Stone Floor / Ancient Stone Floor — water biome walk override.
const FLOOR_WOODEN: i32 = 485;
const FLOOR_STONE: i32 = 884;
const FLOOR_ANCIENT_STONE: i32 = 898;
/// Haxe Pine Floor — does **not** cancel biome block.
const FLOOR_PINE: i32 = 3290;

/// Haxe hard-coded boat ids (`Running Crude Car` 2396, `Delivery Truck` 4655).
const BOAT_IDS: &[i32] = &[2396, 4655];

/// Haxe `WorldMap.getBiomeSpeed` (base table + water-floor → 1.0).
///
/// Floors 485/884/898 over ocean / passable river / river return `1.0`.
#[inline]
pub fn tile_biome_speed(biome: BiomeId, floor_id: i32) -> f32 {
    if (floor_id == FLOOR_WOODEN || floor_id == FLOOR_STONE || floor_id == FLOOR_ANCIENT_STONE)
        && matches!(biome, OCEAN | PASSABLE_RIVER | RIVER)
    {
        return 1.0;
    }
    world_biome_speed(biome)
}

/// True when floor `speedMult` counts as road for path-wide road bonus.
#[inline]
pub fn floor_counts_as_road(floor_speed_mult: f32) -> bool {
    floor_speed_mult >= ROAD_SPEED_THRESHOLD
}

/// Resolve floor `speedMult` from content (missing / id 0 → 1.0).
#[inline]
pub fn floor_speed_mult(content: &ContentDb, floor_id: i32) -> f32 {
    if floor_id <= 0 {
        return 1.0;
    }
    content
        .get(floor_id)
        .map(|d| {
            if d.speed_mult.is_finite() && d.speed_mult > 0.0 {
                d.speed_mult
            } else {
                1.0
            }
        })
        .unwrap_or(1.0)
}

/// Haxe floor/road + biome portion of `calculateSpeed` (multiplier only).
///
/// ```text
/// if !full_path_has_road: floor_speed = 1
/// speed *= floor_speed
/// if (on_floor || is_on_boat) && biome < 0.99: biome = 1
/// biome = max(biome, MinBiomeSpeedFactor)
/// speed *= biome
/// ```
///
/// Does **not** include held/horse/contained/shoes/hitpoints (separate chunks).
pub fn floor_road_biome_factor(
    floor_id: i32,
    floor_speed_mult: f32,
    biome_speed: f32,
    full_path_has_road: bool,
    is_on_boat: bool,
) -> f32 {
    let mut floor_speed = if floor_speed_mult.is_finite() && floor_speed_mult > 0.0 {
        floor_speed_mult
    } else {
        1.0
    };
    // Only apply road bonus when the full path is on road.
    if !full_path_has_road {
        floor_speed = 1.0;
    }

    let on_floor = floor_id > 0;
    let mut biome = if biome_speed.is_finite() {
        biome_speed
    } else {
        1.0
    };
    // Floor (or boat) cancels bad-biome mali.
    if (on_floor || is_on_boat) && biome < 0.99 {
        biome = 1.0;
    }
    if biome < MIN_BIOME_SPEED_FACTOR {
        biome = MIN_BIOME_SPEED_FACTOR;
    }

    SPEED_FACTOR * floor_speed * biome
}

/// Soften slow held-object mult while standing on any floor (Haxe `onFloor && speedModHeldObj < 0.99`).
#[inline]
pub fn soften_held_speed_on_floor(held_speed_mult: f32, on_floor: bool) -> f32 {
    let m = if held_speed_mult.is_finite() {
        held_speed_mult
    } else {
        1.0
    };
    if on_floor && m < 0.99 {
        m.sqrt()
    } else {
        m
    }
}

/// Soften slow contained mult while on floor (Haxe containedObjSpeedMult sqrt on road/floor).
#[inline]
pub fn soften_contained_speed_on_floor(contained_speed_mult: f32, on_floor: bool) -> f32 {
    let m = if contained_speed_mult.is_finite() {
        contained_speed_mult
    } else {
        1.0
    };
    if on_floor && m < 0.99 {
        m.sqrt()
    } else {
        m
    }
}

/// Compose speed from base walk speed × floor/road/biome factor.
///
/// `base_speed` is typically [`INITIAL_PLAYER_MOVE_SPEED`] or the ride/weather-composed base.
#[inline]
pub fn apply_floor_road_to_speed(
    base_speed: f32,
    floor_id: i32,
    floor_speed_mult: f32,
    biome_speed: f32,
    full_path_has_road: bool,
    is_on_boat: bool,
) -> f32 {
    let base = if base_speed.is_finite() && base_speed > 0.0 {
        base_speed
    } else {
        INITIAL_PLAYER_MOVE_SPEED
    };
    base * floor_road_biome_factor(
        floor_id,
        floor_speed_mult,
        biome_speed,
        full_path_has_road,
        is_on_boat,
    )
}

/// Lookup floor + biome at tile and compute Haxe floor/road/biome factor.
pub fn floor_road_factor_at(
    world: &World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    full_path_has_road: bool,
    is_on_boat: bool,
) -> f32 {
    let floor_id = world.get_floor(tx, ty) as i32;
    let biome = world.get_biome(tx, ty);
    let biome_spd = tile_biome_speed(biome, floor_id);
    let floor_spd = floor_speed_mult(content, floor_id);
    floor_road_biome_factor(
        floor_id,
        floor_spd,
        biome_spd,
        full_path_has_road,
        is_on_boat,
    )
}

/// Haxe `Biome.IsWater` — ocean / passable river / river.
#[inline]
pub fn is_water_biome(biome: BiomeId) -> bool {
    matches!(biome, OCEAN | PASSABLE_RIVER | RIVER)
}

/// Haxe `WorldMap.isBiomeBlocking` at tile (any floor except pine 3290 over water walls).
#[inline]
pub fn tile_biome_blocks_move(world: &World, x: i32, y: i32) -> bool {
    let biome = world.get_biome(x, y);
    let floor = world.get_floor(x, y) as i32;
    is_biome_blocking(biome, floor)
}

/// Haxe `heldObject.objectData.isBoat` (+isBoat tag / Sports Car / hard-coded ids).
pub fn object_is_boat(content: &ContentDb, object_id: i32) -> bool {
    if object_id <= 0 {
        return false;
    }
    if BOAT_IDS.contains(&object_id) {
        return true;
    }
    let Some(d) = content.get(object_id) else {
        return false;
    };
    let desc = if !d.description.is_empty() {
        d.description.as_str()
    } else {
        d.name.as_str()
    };
    desc.contains("+isBoat") || desc.contains("Sports Car")
}

/// Held object `speedMult` (missing / empty hands → 1.0).
///
/// COMBAT-BLOODY: bloody weapons use [`crate::weapons::bloody_weapon_speed_mult`]
/// (0.75/0.85/0.6) so vanilla content `speedMult=0.25` is not applied.
// Haxe: ServerSettings.PatchObjectData speedMult 750/3048/749
#[inline]
pub fn held_object_speed_mult(content: &ContentDb, held_id: i32) -> f32 {
    if held_id <= 0 {
        return 1.0;
    }
    // Haxe: PatchObjectData overrides content for bloody knife/sword/bow.
    if let Some(m) = crate::weapons::bloody_weapon_speed_mult(held_id) {
        return m;
    }
    content
        .get(held_id)
        .map(|d| {
            if d.speed_mult.is_finite() && d.speed_mult > 0.0 {
                d.speed_mult
            } else {
                1.0
            }
        })
        .unwrap_or(1.0)
}

/// Haxe `onHorseOrCar = heldObject.objectData.speedMult >= 1.1`.
#[inline]
pub fn is_horse_or_car(held_speed_mult: f32) -> bool {
    held_speed_mult.is_finite() && held_speed_mult >= HORSE_OR_CAR_SPEED_THRESHOLD
}

/// Biome speed after floor/boat cancel + min clamp (used for held/contain rules).
///
/// Haxe: after `getBiomeSpeed`, cancel with floor/boat, then `MinBiomeSpeedFactor`.
pub fn effective_biome_speed(biome_speed: f32, on_floor: bool, is_on_boat: bool) -> f32 {
    effective_biome_speed_ex(biome_speed, on_floor, is_on_boat, MIN_BIOME_SPEED_FACTOR)
}

/// Live-knob variant: `min_biome` = Haxe `MinBiomeSpeedFactor`.
// Haxe: MoveHelper.calculateSpeed biome clamp + ServerSettings.MinBiomeSpeedFactor
// C-SS-TAIL-KNOBS
pub fn effective_biome_speed_ex(
    biome_speed: f32,
    on_floor: bool,
    is_on_boat: bool,
    min_biome: f32,
) -> f32 {
    let mut b = if biome_speed.is_finite() {
        biome_speed
    } else {
        1.0
    };
    if (on_floor || is_on_boat) && b < 0.99 {
        b = 1.0;
    }
    let floor = if min_biome.is_finite() && min_biome >= 0.0 {
        min_biome
    } else {
        MIN_BIOME_SPEED_FACTOR
    };
    if b < floor {
        b = floor;
    }
    b
}

/// Haxe horse/car clamp when path leaves road in a bad biome (`calculateSpeed` ~141-148).
///
/// ```text
/// if !fullPathHasRoad && biomeSpeed < 0.999 && held > 1:
///   >2.50 → 0.9; >1.8 → 1.5; >1.2 → 0.9
/// ```
pub fn clamp_held_speed_bad_biome(
    held_speed_mult: f32,
    full_path_has_road: bool,
    effective_biome: f32,
) -> f32 {
    let mut m = if held_speed_mult.is_finite() {
        held_speed_mult
    } else {
        1.0
    };
    if !full_path_has_road && effective_biome < 0.999 && m > 1.0 {
        if m > 2.50 {
            m = 0.9;
        } else if m > 1.8 {
            m = 1.5;
        } else if m > 1.2 {
            m = 0.9;
        }
    }
    m
}

/// Held speed after bad-biome clamp + floor soften.
pub fn adjust_held_speed_mult(
    held_speed_mult: f32,
    full_path_has_road: bool,
    effective_biome: f32,
    on_floor: bool,
) -> f32 {
    let clamped =
        clamp_held_speed_bad_biome(held_speed_mult, full_path_has_road, effective_biome);
    soften_held_speed_on_floor(clamped, on_floor)
}

/// Haxe `calculateObjSpeedMult` — clamp each contained object mult to \[0.6, 0.98\].
#[inline]
pub fn contained_obj_speed_mult(obj_speed_mult: f32) -> f32 {
    let m = if obj_speed_mult.is_finite() {
        obj_speed_mult
    } else {
        1.0
    };
    m.min(MIN_SPEED_REDUCTION_PER_CONTAINED)
        .max(CONTAINED_SPEED_FLOOR)
}

/// Product of backpack item mults (each via [`contained_obj_speed_mult`]).
pub fn backpack_speed_product(content: &ContentDb, backpack: &[i32]) -> f32 {
    let mut p = 1.0f32;
    for &id in backpack {
        if id <= 0 {
            continue;
        }
        let sm = content
            .get(id)
            .map(|d| d.speed_mult)
            .unwrap_or(1.0);
        p *= contained_obj_speed_mult(sm);
    }
    p
}

/// Haxe containedObjSpeedMult after bad-biome double mali + floor/horse softens.
pub fn adjust_contained_speed_mult(
    product: f32,
    on_floor: bool,
    on_horse_or_car: bool,
    effective_biome: f32,
) -> f32 {
    let mut c = if product.is_finite() && product > 0.0 {
        product
    } else {
        1.0
    };
    // Haxe: if (biomeSpeed < 0.9 && onFloor == false) containedObjSpeedMult *= itself
    if effective_biome < 0.9 && !on_floor {
        c *= c;
    }
    c = soften_contained_speed_on_floor(c, on_floor);
    if on_horse_or_car && c < 0.99 {
        c = c.sqrt();
    }
    c
}

// --- S-MOVE-POLISH pure factors ---

/// Haxe `GlobalPlayerInstance.hasBothShoes`: clothingObjects[2] **and** [3] both non-zero.
///
/// Left shoe = clothing index 2 (also flat `Player.shoes`); right shoe = index 3.
/// A single left shoe alone does **not** grant the 1.1× walk bonus.
// Haxe: GlobalPlayerInstance.hasBothShoes
#[inline]
pub fn has_both_shoes(left_shoe_id: i32, right_shoe_id: i32) -> bool {
    left_shoe_id != 0 && right_shoe_id != 0
}

/// Resolve left/right shoe object ids from flat `shoes` + `clothing_helpers[2,3]`.
// Haxe: clothingObjects[2] / clothingObjects[3]
#[inline]
pub fn shoe_pair_ids(
    shoes_flat: i32,
    clothing_left: Option<i32>,
    clothing_right: Option<i32>,
) -> (i32, i32) {
    let left = clothing_left.filter(|&id| id != 0).unwrap_or(shoes_flat);
    let right = clothing_right.unwrap_or(0);
    (left, right)
}

/// Haxe `hasBothShoes && !onHorseOrCar` → `SpeedWithBothShoes`.
#[inline]
pub fn shoes_speed_factor(has_both_shoes: bool, on_horse_or_car: bool) -> f32 {
    if has_both_shoes && !on_horse_or_car {
        SPEED_WITH_BOTH_SHOES
    } else {
        1.0
    }
}

/// Haxe backpack product √ when both shoes (before held-nest product multiplies).
#[inline]
pub fn shoes_soften_backpack_product(product: f32, has_both_shoes: bool) -> f32 {
    let p = if product.is_finite() && product > 0.0 {
        product
    } else {
        1.0
    };
    if has_both_shoes {
        p.sqrt()
    } else {
        p
    }
}

// MOVE-NEST-SPEED pure helpers (held nest after backpack shoes-√).
include!("move_nest_speed_inc.rs");

/// Haxe hitpoints speed:
/// `(current + (factor-1)*full) / (factor * full)` when `factor > 0`.
pub fn hitpoints_speed_factor(
    current_food_store_max: f32,
    full_hitpoints: f32,
    factor: f32,
) -> f32 {
    if !(factor > 0.0) {
        return 1.0;
    }
    let full = if full_hitpoints.is_finite() && full_hitpoints > 0.0 {
        full_hitpoints
    } else {
        GROWN_UP_FOOD_STORE_MAX
    };
    let curr = if current_food_store_max.is_finite() {
        current_food_store_max.max(0.0)
    } else {
        full
    };
    let den = factor * full;
    if den <= 0.0 {
        return 1.0;
    }
    (curr + (factor - 1.0) * full) / den
}

/// Haxe temperature speed: super-hot / super-cold apply `impact` (or `impact²` at extremes).
///
/// `impact == 1.0` (default ServerSettings) is a no-op multiplier.
pub fn temperature_speed_factor(
    heat: f32,
    is_super_hot: bool,
    is_super_cold: bool,
    impact: f32,
) -> f32 {
    let imp = if impact.is_finite() && impact > 0.0 {
        impact
    } else {
        1.0
    };
    if is_super_hot {
        if heat > HEAT_DOUBLE_IMPACT_HIGH {
            imp * imp
        } else {
            imp
        }
    } else if is_super_cold {
        if heat < HEAT_DOUBLE_IMPACT_LOW {
            imp * imp
        } else {
            imp
        }
    } else {
        1.0
    }
}

/// Coarse super-hot / super-cold from 0..1 heat (Haxe body heat extremes).
#[inline]
pub fn heat_is_super_hot(heat: f32) -> bool {
    heat.is_finite() && heat >= 0.85
}

#[inline]
pub fn heat_is_super_cold(heat: f32) -> bool {
    heat.is_finite() && heat <= 0.15
}

/// Haxe close-grave curse speed mali when curse population gate is open.
#[inline]
pub fn grave_curse_speed_factor(curse_active: bool) -> f32 {
    if curse_active {
        CLOSE_GRAVE_SPEED_MALI
    } else {
        1.0
    }
}

/// Haxe close hostile with weapon: `angryTime < 0` and enemy within 1.5.
#[inline]
pub fn close_enemy_speed_factor(close_hostile_with_weapon: bool) -> f32 {
    close_enemy_speed_factor_ex(
        close_hostile_with_weapon,
        CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,
    )
}

/// Live-knob variant of [`close_enemy_speed_factor`].
// Haxe: ServerSettings.CloseEnemyWithWeaponSpeedFactor
// C-SS-MORE-BATCH5
#[inline]
pub fn close_enemy_speed_factor_ex(
    close_hostile_with_weapon: bool,
    factor: f32,
) -> f32 {
    if close_hostile_with_weapon {
        if factor.is_finite() && factor > 0.0 {
            factor
        } else {
            CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR
        }
    } else {
        1.0
    }
}

/// Haxe AI-only prestige class speed (humans return 1.0).
pub fn ai_class_speed_factor(is_ai: bool, class: PrestigeClass) -> f32 {
    ai_class_speed_factor_ex(
        is_ai,
        class,
        AI_SPEED_FACTOR_SERF,
        AI_SPEED_FACTOR_COMMONER,
        AI_SPEED_FACTOR_NOBLE,
    )
}

/// Live-knob variant of [`ai_class_speed_factor`].
// Haxe: ServerSettings.AISpeedFactorSerf/Commoner/Noble
// C-SS-MORE-BATCH5
pub fn ai_class_speed_factor_ex(
    is_ai: bool,
    class: PrestigeClass,
    serf: f32,
    commoner: f32,
    noble: f32,
) -> f32 {
    if !is_ai {
        return 1.0;
    }
    let s = if serf.is_finite() && serf > 0.0 {
        serf
    } else {
        AI_SPEED_FACTOR_SERF
    };
    let c = if commoner.is_finite() && commoner > 0.0 {
        commoner
    } else {
        AI_SPEED_FACTOR_COMMONER
    };
    let n = if noble.is_finite() && noble > 0.0 {
        noble
    } else {
        AI_SPEED_FACTOR_NOBLE
    };
    match class {
        PrestigeClass::Serf => s,
        PrestigeClass::Commoner | PrestigeClass::NotSet => c,
        PrestigeClass::Noble | PrestigeClass::King | PrestigeClass::Emperor => n,
    }
}

/// Haxe TODO “half penalty for strong”: cut contained slowdown in half.
///
/// `product 0.64` (36% slow) → strong `0.82` (18% slow) = `1 - (1-p)/2`.
#[inline]
pub fn half_penalty_for_strong(contained_mult: f32, is_strong: bool) -> f32 {
    let c = if contained_mult.is_finite() && contained_mult > 0.0 {
        contained_mult
    } else {
        1.0
    };
    if is_strong && c < 1.0 {
        1.0 - (1.0 - c) * 0.5
    } else {
        c
    }
}

/// Inputs for the vitals/social tail of Haxe `calculateSpeed`.
#[derive(Debug, Clone, Copy)]
pub struct VitalsSpeedInput {
    pub has_both_shoes: bool,
    pub on_horse_or_car: bool,
    pub current_food_store_max: f32,
    pub heat: f32,
    pub curse_active: bool,
    pub close_hostile_with_weapon: bool,
    pub is_ai: bool,
    pub prestige_class: PrestigeClass,
    /// Strong body / prestige strength — applies half contained penalty (Haxe TODO).
    pub is_strong: bool,
    /// Haxe heldObject.containedObjects (+1 nest) product; default 1.0 (MOVE-NEST-SPEED).
    pub held_nest_product: f32,
}

impl Default for VitalsSpeedInput {
    fn default() -> Self {
        Self {
            has_both_shoes: false,
            on_horse_or_car: false,
            current_food_store_max: GROWN_UP_FOOD_STORE_MAX,
            heat: 0.5,
            curse_active: false,
            close_hostile_with_weapon: false,
            is_ai: false,
            prestige_class: PrestigeClass::Commoner,
            is_strong: false,
            held_nest_product: 1.0,
        }
    }
}

/// Live knobs for vitals/social speed tail (C-SS-TAIL-KNOBS + C-SS-MORE-BATCH5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalsSpeedLiveKnobs {
    pub grown_up_food_store_max: f32,
    pub hitpoints_speed_factor: f32,
    pub close_enemy_with_weapon_speed_factor: f32,
    pub ai_speed_factor_serf: f32,
    pub ai_speed_factor_commoner: f32,
    pub ai_speed_factor_noble: f32,
}

impl Default for VitalsSpeedLiveKnobs {
    fn default() -> Self {
        Self {
            grown_up_food_store_max: GROWN_UP_FOOD_STORE_MAX,
            hitpoints_speed_factor: HITPOINTS_SPEED_FACTOR,
            close_enemy_with_weapon_speed_factor: CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,
            ai_speed_factor_serf: AI_SPEED_FACTOR_SERF,
            ai_speed_factor_commoner: AI_SPEED_FACTOR_COMMONER,
            ai_speed_factor_noble: AI_SPEED_FACTOR_NOBLE,
        }
    }
}

/// Product of shoes / hitpoints / temp / grave / enemy / AI class factors.
pub fn vitals_speed_product(v: &VitalsSpeedInput) -> f32 {
    vitals_speed_product_ex(v, GROWN_UP_FOOD_STORE_MAX, HITPOINTS_SPEED_FACTOR)
}

/// Live-knob variant: full-HP baseline + HitpointsSpeedFactor from GameplayKnobs.
// Haxe: MoveHelper.calculateSpeed hitpoints + ServerSettings.HitpointsSpeedFactor / GrownUpFoodStoreMax
// C-SS-TAIL-KNOBS
pub fn vitals_speed_product_ex(
    v: &VitalsSpeedInput,
    grown_up_food_store_max: f32,
    hitpoints_speed_factor_knob: f32,
) -> f32 {
    vitals_speed_product_live(
        v,
        &VitalsSpeedLiveKnobs {
            grown_up_food_store_max,
            hitpoints_speed_factor: hitpoints_speed_factor_knob,
            ..VitalsSpeedLiveKnobs::default()
        },
    )
}

/// Full live vitals product (hitpoints + close-enemy + AI class knobs).
// Haxe: MoveHelper.calculateSpeed + ServerSettings.*Speed*
// C-SS-MORE-BATCH5
pub fn vitals_speed_product_live(v: &VitalsSpeedInput, knobs: &VitalsSpeedLiveKnobs) -> f32 {
    let shoes = shoes_speed_factor(v.has_both_shoes, v.on_horse_or_car);
    let full = if knobs.grown_up_food_store_max.is_finite() && knobs.grown_up_food_store_max > 0.0
    {
        knobs.grown_up_food_store_max
    } else {
        GROWN_UP_FOOD_STORE_MAX
    };
    let hp_factor = if knobs.hitpoints_speed_factor.is_finite() {
        knobs.hitpoints_speed_factor
    } else {
        HITPOINTS_SPEED_FACTOR
    };
    let hp = hitpoints_speed_factor(v.current_food_store_max, full, hp_factor);
    let temp = temperature_speed_factor(
        v.heat,
        heat_is_super_hot(v.heat),
        heat_is_super_cold(v.heat),
        TEMPERATURE_SPEED_IMPACT,
    );
    shoes
        * hp
        * temp
        * grave_curse_speed_factor(v.curse_active)
        * close_enemy_speed_factor_ex(
            v.close_hostile_with_weapon,
            knobs.close_enemy_with_weapon_speed_factor,
        )
        * ai_class_speed_factor_ex(
            v.is_ai,
            v.prestige_class,
            knobs.ai_speed_factor_serf,
            knobs.ai_speed_factor_commoner,
            knobs.ai_speed_factor_noble,
        )
}

/// Apply vitals/social polish multipliers to a floor/held/contain base speed.
#[inline]
pub fn apply_vitals_speed_polish(base_speed: f32, v: &VitalsSpeedInput) -> f32 {
    apply_vitals_speed_polish_live(base_speed, v, &VitalsSpeedLiveKnobs::default())
}

/// Live-knob variant of [`apply_vitals_speed_polish`].
// C-SS-MORE-BATCH5
#[inline]
pub fn apply_vitals_speed_polish_live(
    base_speed: f32,
    v: &VitalsSpeedInput,
    knobs: &VitalsSpeedLiveKnobs,
) -> f32 {
    let base = if base_speed.is_finite() && base_speed > 0.0 {
        base_speed
    } else {
        INITIAL_PLAYER_MOVE_SPEED
    };
    base * vitals_speed_product_live(v, knobs)
}

/// Floor/road/biome × held × contained, with boat-on-land override.
///
/// Haxe `calculateSpeed` floor/held/contain core (shoes/hitpoints/temp/grave/AI via polish).
/// `base_speed` is ride/weather/snow/fire/ballast composed walk speed.
///
/// `has_both_shoes` softens backpack product (√). `is_strong` halves remaining contain penalty.
pub fn apply_held_floor_speed(
    base_speed: f32,
    floor_id: i32,
    floor_spd: f32,
    biome_spd: f32,
    full_path_has_road: bool,
    is_on_boat: bool,
    held_speed_mult: f32,
    backpack_product: f32,
    is_water: bool,
) -> f32 {
    apply_held_floor_speed_ex(
        base_speed,
        floor_id,
        floor_spd,
        biome_spd,
        full_path_has_road,
        is_on_boat,
        held_speed_mult,
        backpack_product,
        is_water,
        false,
        false,
        1.0,
    )
}

/// Extended floor/held/contain with shoes backpack soften + strong half-penalty.
///
/// `held_nest_product` is Haxe held `containedObjects` (+ one sub-level) product
/// after backpack shoes-√ (MOVE-NEST-SPEED). Pass `1.0` when empty hands / no nest.
pub fn apply_held_floor_speed_ex(
    base_speed: f32,
    floor_id: i32,
    floor_spd: f32,
    biome_spd: f32,
    full_path_has_road: bool,
    is_on_boat: bool,
    held_speed_mult: f32,
    backpack_product: f32,
    is_water: bool,
    has_both_shoes: bool,
    is_strong: bool,
    held_nest_product: f32,
) -> f32 {
    let on_floor = floor_id > 0;
    let floor_biome = floor_road_biome_factor(
        floor_id,
        floor_spd,
        biome_spd,
        full_path_has_road,
        is_on_boat,
    );
    let eff_biome = effective_biome_speed(biome_spd, on_floor, is_on_boat);
    let held = adjust_held_speed_mult(
        held_speed_mult,
        full_path_has_road,
        eff_biome,
        on_floor,
    );
    let on_horse = is_horse_or_car(held_speed_mult);
    // Haxe: shoes √ backpack first, then *= held nest (shoes do not soften nest).
    let pack = combine_backpack_and_held_nest(
        backpack_product,
        held_nest_product,
        has_both_shoes,
    );
    let contained = half_penalty_for_strong(
        adjust_contained_speed_mult(pack, on_floor, on_horse, eff_biome),
        is_strong,
    );
    let base = if base_speed.is_finite() && base_speed > 0.0 {
        base_speed
    } else {
        INITIAL_PLAYER_MOVE_SPEED
    };
    // Haxe: boat on land replaces speed entirely with 0.5 * Initial
    if is_on_boat && !is_water {
        return BOAT_ON_LAND_SPEED_FACTOR * INITIAL_PLAYER_MOVE_SPEED;
    }
    // Shoes walk bonus (not applied when horse/car).
    let shoe_walk = shoes_speed_factor(has_both_shoes, on_horse);
    base * floor_biome * held * contained * shoe_walk
}

/// World/content lookup variant of [`apply_held_floor_speed`].
pub fn apply_held_floor_speed_at(
    world: &World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    base_speed: f32,
    full_path_has_road: bool,
    held_id: i32,
    backpack: &[i32],
) -> f32 {
    apply_held_floor_speed_at_ex(
        world,
        content,
        tx,
        ty,
        base_speed,
        full_path_has_road,
        held_id,
        backpack,
        None,
        false,
        false,
        1.0,
    )
}

/// World lookup with shoes / strong half-penalty (S-MOVE-POLISH floor core).
///
/// `clothing_pack` — Haxe `getPackpack()` (`clothing_helpers[5]`); preferred over flat ids.
/// `held_nest_product` — MOVE-NEST-SPEED held cargo nest mult (default 1.0).
pub fn apply_held_floor_speed_at_ex(
    world: &World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    base_speed: f32,
    full_path_has_road: bool,
    held_id: i32,
    backpack: &[i32],
    clothing_pack: Option<&NestedHelper>,
    has_both_shoes: bool,
    is_strong: bool,
    held_nest_product: f32,
) -> f32 {
    let floor_id = world.get_floor(tx, ty) as i32;
    let biome = world.get_biome(tx, ty);
    let biome_spd = tile_biome_speed(biome, floor_id);
    let floor_spd = floor_speed_mult(content, floor_id);
    let is_boat = object_is_boat(content, held_id);
    let held_sm = held_object_speed_mult(content, held_id);
    // Haxe getPackpack().containedObjects; flat backpack is legacy fallback.
    let pack = resolve_backpack_speed_product(content, backpack, clothing_pack);
    apply_held_floor_speed_ex(
        base_speed,
        floor_id,
        floor_spd,
        biome_spd,
        full_path_has_road,
        is_boat,
        held_sm,
        pack,
        is_water_biome(biome),
        has_both_shoes,
        is_strong,
        held_nest_product,
    )
}

/// Full Haxe `calculateSpeed` product: floor/held/contain (+ held nest) + vitals polish.
///
/// Set [`VitalsSpeedInput::held_nest_product`] from [`held_nest_speed_product`] for
/// live nest cargo (MOVE-NEST-SPEED). Default 1.0 is a no-op.
///
/// `clothing_pack` — Haxe clothing backpack nest (`clothing_helpers[5]` / getPackpack).
// Haxe: MoveHelper.calculateSpeed
pub fn apply_calculate_speed_full(
    world: &World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    base_speed: f32,
    full_path_has_road: bool,
    held_id: i32,
    backpack: &[i32],
    clothing_pack: Option<&NestedHelper>,
    vitals: &VitalsSpeedInput,
) -> f32 {
    let floor_core = apply_held_floor_speed_at_ex(
        world,
        content,
        tx,
        ty,
        base_speed,
        full_path_has_road,
        held_id,
        backpack,
        clothing_pack,
        vitals.has_both_shoes,
        vitals.is_strong,
        vitals.held_nest_product,
    );
    // Shoes walk bonus is already in floor_core; strip from vitals product to avoid double.
    let mut v = *vitals;
    v.has_both_shoes = false; // already applied in floor_core
    v.on_horse_or_car = false;
    apply_vitals_speed_polish(floor_core, &v)
}

/// Live-knob variant of [`apply_calculate_speed_full`].
// C-SS-MORE-BATCH5
pub fn apply_calculate_speed_full_live(
    world: &World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    base_speed: f32,
    full_path_has_road: bool,
    held_id: i32,
    backpack: &[i32],
    clothing_pack: Option<&NestedHelper>,
    vitals: &VitalsSpeedInput,
    knobs: &VitalsSpeedLiveKnobs,
) -> f32 {
    let floor_core = apply_held_floor_speed_at_ex(
        world,
        content,
        tx,
        ty,
        base_speed,
        full_path_has_road,
        held_id,
        backpack,
        clothing_pack,
        vitals.has_both_shoes,
        vitals.is_strong,
        vitals.held_nest_product,
    );
    let mut v = *vitals;
    v.has_both_shoes = false;
    v.on_horse_or_car = false;
    apply_vitals_speed_polish_live(floor_core, &v, knobs)
}

/// Result of scanning a path for road continuity + off-road biome trunc.
#[derive(Debug, Clone, PartialEq)]
pub struct PathRoadScan {
    /// Steps kept after biome-speed trunc (per-step deltas).
    pub steps: Vec<(i32, i32)>,
    /// 1 if path was shortened (walkability already reflected in input length).
    pub trunc: i32,
    /// Haxe `fullPathHasRoad` — every tile on the kept path has road-grade floor.
    pub full_path_has_road: bool,
}

/// Scan accepted **per-step** path for full-path road and off-road biome trunc.
///
/// Haxe `calculateNewMovements` (after block check):
/// 1. Start with `fullPathHasRoad = true`
/// 2. Each step: if floor speedMult < 1.01 → clear road flag
/// 3. If off-road and |biomeSpeed − startBiomeSpeed|² > 0.1² → keep this step, trunc rest
///
/// `prior_trunc` from walkability is OR'd into the result trunc flag.
pub fn scan_path_road_and_biome(
    world: &World,
    content: &ContentDb,
    start_x: i32,
    start_y: i32,
    accepted_steps: &[(i32, i32)],
    prior_trunc: i32,
) -> PathRoadScan {
    let mut full_path_has_road = true;
    let start_floor = world.get_floor(start_x, start_y) as i32;
    let start_biome_speed = tile_biome_speed(world.get_biome(start_x, start_y), start_floor);
    let mut kept = Vec::with_capacity(accepted_steps.len());
    let mut x = start_x;
    let mut y = start_y;
    let mut trunc = prior_trunc.max(0);

    for (i, &(dx, dy)) in accepted_steps.iter().enumerate() {
        if dx == 0 && dy == 0 {
            continue;
        }
        let (nx, ny) = world.wrap_tile(x + dx, y + dy);
        let floor_id = world.get_floor(nx, ny) as i32;
        let floor_spd = floor_speed_mult(content, floor_id);
        if full_path_has_road && !floor_counts_as_road(floor_spd) {
            full_path_has_road = false;
        }
        let end_biome_speed = tile_biome_speed(world.get_biome(nx, ny), floor_id);
        let biome_delta = end_biome_speed - start_biome_speed;
        if !full_path_has_road
            && biome_delta * biome_delta > TRUNC_MOVEMENT_SPEED_DIFF * TRUNC_MOVEMENT_SPEED_DIFF
        {
            // Keep this transition step then stop (Haxe includes the border tile).
            kept.push((dx, dy));
            // Haxe: if moves.length > 1 → trunc = 1
            if accepted_steps.len() > 1 {
                trunc = 1;
            }
            let _ = i; // index available for debug
            return PathRoadScan {
                steps: kept,
                trunc,
                full_path_has_road,
            };
        }
        kept.push((dx, dy));
        x = nx;
        y = ny;
    }

    PathRoadScan {
        steps: kept,
        trunc,
        full_path_has_road,
    }
}

/// Walkability truncate then road/biome refine (client path → accepted steps).
///
/// Uses Haxe `isBiomeBlocking` + `count > 10` path cap, then
/// [`scan_path_road_and_biome`].
pub fn truncate_path_with_road(
    world: &World,
    content: &ContentDb,
    start_x: i32,
    start_y: i32,
    client_deltas: &[(i32, i32)],
    walk_ok: impl Fn(i32, i32) -> bool,
) -> PathRoadScan {
    let steps = client_path_deltas_to_steps(client_deltas);
    let mut accepted = Vec::with_capacity(steps.len().min(MAX_CLIENT_PATH_STEPS));
    let mut x = start_x;
    let mut y = start_y;
    let mut nonzero = 0usize;
    for &(dx, dy) in &steps {
        if dx == 0 && dy == 0 {
            continue;
        }
        nonzero += 1;
        if nonzero > MAX_CLIENT_PATH_STEPS {
            break;
        }
        let (nx, ny) = world.wrap_tile(x + dx, y + dy);
        // Haxe isBlocked includes isBiomeBlocking — reject ocean without floor.
        if tile_biome_blocks_move(world, nx, ny) {
            break;
        }
        if !walk_ok(nx, ny) {
            break;
        }
        accepted.push((dx, dy));
        x = nx;
        y = ny;
    }
    let prior_trunc = if accepted.len() < nonzero { 1 } else { 0 };
    scan_path_road_and_biome(world, content, start_x, start_y, &accepted, prior_trunc)
}

/// Path length helper (cardinal/diagonal) for tests / callers.
#[inline]
pub fn path_length(steps: &[(i32, i32)]) -> f32 {
    steps.iter().map(|&(dx, dy)| step_len(dx, dy)).sum()
}

#[cfg(test)]
#[path = "move_speed_tests.rs"]
mod tests;
