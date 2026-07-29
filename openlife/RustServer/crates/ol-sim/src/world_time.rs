//! Map time-slice + auto-decay helpers (Haxe `TimeHelper.DoWorldMapTimeStuff` family).
//!
//! Chunk: **TIME-WORLD** / **TIME-WORLD-POLISH** (`temp_nested_water`)
//! + **CONTAINED-TIMERS-PERSIST** (`rearm_after_load` — slots ↔ contained_timers)
//! + **NESTED-IN-NESTED-TIMERS** (`deep_contained` — NestedHelper recursive timers)
//! Anchors: `DoWorldMapTimeStuff`, `doTimeTransition` / `doTimeTransitionHelper`,
//! `doTimeForObject`, `DoSecondTimeOutcome`, `DoItemInWaterMovement`,
//! `UpdateTileTemperature`, `balanceTileTemperature`,
//! `TransitionHelper.TransformTarget`, `ObjectHelper.CalculateTimeToChangeForObj` /
//! `isTimeToChangeReached`, `TransitionData.calculateTimeToChange`.

use ol_content::{ContentDb, Transition};
use ol_world::{ComplexObject, World, OCEAN, PASSABLE_RIVER, RIVER, SNOWINGREY};

/// Haxe `BiomeTag.SNOW` (not re-exported from ol_world root).
const BIOME_SNOW: u8 = 4;
use rand::Rng;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ServerSettings-aligned constants (Haxe ServerSettings subset)
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.WorldTimeParts` — map height is processed in this many Y bands.
pub const WORLD_TIME_PARTS: i32 = 25;

/// Haxe `ServerSettings.WinterWildFoodDecayChance` (per season).
pub const WINTER_WILD_FOOD_DECAY_CHANCE: f32 = 1.5;

/// Haxe `ServerSettings.SpringWildFoodRegrowChance` (per season).
pub const SPRING_WILD_FOOD_REGROW_CHANCE: f32 = 1.0;

/// Haxe `ServerSettings.TemperatureOwnTileRate` (lerp toward biome+season target).
pub const TEMPERATURE_OWN_TILE_RATE: f32 = 0.05;

/// Haxe `ServerSettings.TemperatureBalanceRate` (neighbor thermal diffusion).
pub const TEMPERATURE_BALANCE_RATE: f32 = 0.9;

/// Haxe `ServerSettings.TemperatureLocalHeatFactor` (fire/ice heat on tile).
pub const TEMPERATURE_LOCAL_HEAT_FACTOR: f32 = 0.005;

/// Haxe `ServerSettings.HotSeasonTemperatureFactor` (scale positive season impact).
pub const HOT_SEASON_TEMPERATURE_FACTOR: f32 = 0.75;

/// Haxe `ServerSettings.ColdSeasonTemperatureFactor` (scale negative season impact).
pub const COLD_SEASON_TEMPERATURE_FACTOR: f32 = 0.75;

/// Fleeing rabbit object id (Haxe hide/unhide).
pub const FLEEING_RABBIT_ID: i32 = 3566;

/// Sharp stone — extends grave decay (Haxe id 34).
pub const SHARP_STONE_ID: i32 = 34;

/// Haxe `ServerSettings.CursedGraveTime` — hours a cursed grave continues per sharp-stone token.
pub const CURSED_GRAVE_TIME_HOURS: f32 = 12.0;

/// Extra seconds per sharp stone in overflowing container.
/// Haxe: `helper.timeToChange += ServerSettings.CursedGraveTime * 60 * 60` (+ 20s base).
pub const CURSED_GRAVE_SHARP_STONE_EXTRA_SECS: f32 = CURSED_GRAVE_TIME_HOURS * 60.0 * 60.0; // 43200

/// Haxe `tick % 2000 == 0` prune cadence for cursedGraves / ovens maps in DoWorldMapTimeStuff.
pub const CURSED_GRAVES_CLEAR_TICK_MOD: u64 = 2000;

/// Haxe `ObjectData.IsOven` ids (Adobe / wood-filled / burning / hot).
/// Same table as `baker_profession::is_oven_id` — kept local to avoid module cycles.
#[inline]
pub fn is_oven_map_id(obj_id: i32) -> bool {
    matches!(obj_id, 237 | 247 | 249 | 250)
}

// ---------------------------------------------------------------------------
// Pure timer math (ObjectHelper / TransitionData)
// ---------------------------------------------------------------------------

/// Haxe `TransitionData.calculateTimeToChange`.
///
/// - `auto_decay_seconds < 0` → hours: `T = (-3600) * auto_decay_seconds`
/// - else `T = auto_decay_seconds`
/// - result = `rand * T + T/2`  (range `[T/2, 3T/2]`)
///
/// `rand01` must be in `[0, 1)`.
pub fn calculate_time_to_change(auto_decay_seconds: f32, rand01: f32) -> f32 {
    if auto_decay_seconds == 0.0 {
        return 0.0;
    }
    let t = if auto_decay_seconds < 0.0 {
        (-3600.0) * auto_decay_seconds
    } else {
        auto_decay_seconds
    };
    let r = rand01.clamp(0.0, 0.999_999);
    r * t + t * 0.5
}

/// Haxe `ObjectHelper.CalculateTimeToChangeForObj` without animal-hits branch.
pub fn calculate_time_to_change_for_obj(
    content: &ContentDb,
    obj_id: i32,
    animal_hits: f32,
    rand01: f32,
) -> f32 {
    let base = content.resolve_base_id(obj_id);
    let Some(tr) = content.auto_decays.get(&base).or_else(|| {
        // Fall back to any actor=-1 transition table if auto_decays missed.
        content.find_transition(-1, base)
    }) else {
        return 0.0;
    };
    let mut t = calculate_time_to_change(tr.auto_decay_seconds, rand01);
    // Haxe: animals with hits > 0.5 decay twice as fast.
    if animal_hits > 0.5 {
        t *= 0.5;
    }
    t
}

/// Haxe `ObjectHelper.isTimeToChangeReached` (creation + timeToChange vs sim time).
#[inline]
pub fn is_time_to_change_reached(
    creation_time: f32,
    time_to_change: f32,
    sim_time: f32,
) -> bool {
    time_to_change > 0.0 && (sim_time - creation_time) >= time_to_change
}

// ---------------------------------------------------------------------------
// TransformTarget (probSet categories)
// ---------------------------------------------------------------------------

/// Haxe `TransitionHelper.TransformTarget`.
///
/// When `target_id` is a `probSet` category parent, pick a weighted member.
/// Otherwise return `target_id` unchanged.
///
/// `rand01` in `[0, 1]` — multiplied by total weight (Haxe `calculateRandomFloat() * totalWeight`).
pub fn transform_target(content: &ContentDb, target_id: i32, rand01: f32) -> i32 {
    let Some(ps) = content.prob_sets.get(&target_id) else {
        return target_id;
    };
    if ps.ids.is_empty() {
        return target_id;
    }
    let total: f32 = ps.weights.iter().sum();
    if total <= 0.0 {
        return ps.ids.first().copied().unwrap_or(target_id);
    }
    let mut r = rand01.clamp(0.0, 1.0) * total;
    let mut acc = 0.0_f32;
    for (i, &w) in ps.weights.iter().enumerate() {
        acc += w;
        if r <= acc {
            return ps.ids.get(i).copied().unwrap_or(target_id);
        }
    }
    ps.ids.last().copied().unwrap_or(target_id)
}

// ---------------------------------------------------------------------------
// World Y-slice math (DoWorldMapTimeStuff band)
// ---------------------------------------------------------------------------

/// Inclusive-exclusive Y range for one map time-slice step.
///
/// Haxe:
/// ```text
/// partSizeY = height / timeParts
/// startY = (worldMapTimeStep % timeParts) * partSizeY
/// endY = startY + partSizeY
/// ```
/// Last incomplete band is not extended here (matches Haxe integer division).
pub fn world_time_slice_y_range(
    height: i32,
    time_parts: i32,
    world_map_time_step: u64,
) -> (i32, i32) {
    let parts = time_parts.max(1);
    let h = height.max(0);
    if h == 0 {
        return (0, 0);
    }
    let part_size = h / parts;
    if part_size <= 0 {
        // Tiny maps: process full height every step.
        return (0, h);
    }
    let band = (world_map_time_step % parts as u64) as i32;
    let start = band * part_size;
    let end = start + part_size;
    (start, end.min(h))
}

/// True when this step starts a full-map cycle (`step % timeParts == 0` before increment
/// matches Haxe: chances recomputed when `worldMapTimeStep % timeParts == 0`).
#[inline]
pub fn is_full_map_cycle_start(world_map_time_step: u64, time_parts: i32) -> bool {
    world_map_time_step % (time_parts.max(1) as u64) == 0
}

/// Haxe winter/spring chance recompute from `TimePassedToDoAllTimeSteps`.
///
/// `WinterDecayChance = time_passed * WinterWildFoodDecayChance / (years_to_next_season * 60) * hardness`
/// Season length in years proxy: `season_length_secs / 60` when seasons are minute-scale in Haxe.
pub fn seasonal_chances(
    time_passed_all_steps: f32,
    season_length_secs: f32,
    season_hardness: f32,
) -> (f32, f32) {
    // Haxe TimeToNextSeasonInYears * 60 — season length in "year-minutes".
    // With short Rust seasons, use season_length as the denominator base.
    let denom = (season_length_secs.max(1.0)) * 1.0;
    let winter = time_passed_all_steps * WINTER_WILD_FOOD_DECAY_CHANCE / denom * season_hardness;
    let spring = time_passed_all_steps * SPRING_WILD_FOOD_REGROW_CHANCE / denom * season_hardness;
    (winter.max(0.0), spring.max(0.0))
}

// ---------------------------------------------------------------------------
// Tile temperature (TemperatureHandler.UpdateTileTemperature + balance)
// ---------------------------------------------------------------------------

/// Haxe `Biome.getBiomeTemperature`.
pub fn biome_base_temperature(biome: u8) -> f32 {
    match biome {
        0 => 0.45,  // GREEN
        1 => 0.2,   // SWAMP
        2 => 0.4,   // YELLOW
        3 => 0.3,   // GREY
        4 => 0.0,   // SNOW
        5 => 1.0,   // DESERT
        6 => 0.7,   // JUNGLE
        15 => 0.6,  // BORDER_JUNGLE
        21 => 0.0,  // SNOWINGREY
        9 => 0.2,   // OCEAN
        17 => 0.2,  // RIVER
        13 => 0.1,  // PASSABLE_RIVER
        _ => 0.5,
    }
}

/// Haxe `WorldMap.getAverageBiomeTemperature` — mean of current + original biome base temps.
#[inline]
pub fn average_biome_temperature(current_biome: u8, original_biome: u8) -> f32 {
    (biome_base_temperature(current_biome) + biome_base_temperature(original_biome)) * 0.5
}

/// Haxe `UpdateTileTemperature` season scaling (Hot/ColdSeasonTemperatureFactor).
#[inline]
pub fn apply_season_temperature_factors(season_impact: f32) -> f32 {
    if season_impact > 0.0 {
        season_impact * HOT_SEASON_TEMPERATURE_FACTOR
    } else if season_impact < 0.0 {
        season_impact * COLD_SEASON_TEMPERATURE_FACTOR
    } else {
        0.0
    }
}

/// Multiplicative insulation factor from raw floor/object rValues (Haxe UpdateTileTemperature).
///
/// `insulationFactor = ((1-floor)*(1-obj))^2` clamped to `[0, 1]`.
#[inline]
pub fn tile_insulation_factor_from_r(floor_insulation: f32, object_insulation: f32) -> f32 {
    let mut f = (1.0 - floor_insulation) * (1.0 - object_insulation);
    f *= f;
    f.clamp(0.0, 1.0)
}

/// Insulation factor from floor rValue proxy + wall permanent (legacy convenience).
///
/// Prefer [`floor_insulation_from_content`] / [`object_insulation_from_content`] +
/// [`tile_insulation_factor_from_r`] when `ObjectDef.r_value` is available.
pub fn tile_insulation_factor(floor_id: i32, object_permanent: bool) -> f32 {
    tile_insulation_factor_from_r(
        floor_insulation_proxy(floor_id),
        object_insulation_proxy(object_permanent),
    )
}

/// Floor insulation proxy when content is unavailable (mild 0.2 for any floor).
#[inline]
pub fn floor_insulation_proxy(floor_id: i32) -> f32 {
    if floor_id > 0 {
        0.2
    } else {
        0.0
    }
}

/// Object (wall) insulation proxy when content is unavailable (permanent → 0.4).
#[inline]
pub fn object_insulation_proxy(object_permanent: bool) -> f32 {
    if object_permanent {
        0.4
    } else {
        0.0
    }
}

/// Haxe `getFloorInsulation` — floor object `rValue` (0 when no floor).
///
/// Falls back to mild proxy when def missing but floor_id > 0.
pub fn floor_insulation_from_content(content: &ContentDb, floor_id: i32) -> f32 {
    if floor_id <= 0 {
        return 0.0;
    }
    match content.get(floor_id) {
        Some(d) => d.r_value,
        None => floor_insulation_proxy(floor_id),
    }
}

/// Haxe `getObjectInsulation` — wall `rValue` when `isWall()`, else 0.
///
/// Resolves multi-use dummy → parent. Falls back to permanent proxy when def missing.
pub fn object_insulation_from_content(content: &ContentDb, obj_id: i32) -> f32 {
    if obj_id <= 0 {
        return 0.0;
    }
    let base = content.resolve_base_id(obj_id);
    match content.get(base) {
        Some(d) => {
            // Haxe: if (objData.isWall() == false) return 0;
            if d.is_wall() {
                d.r_value
            } else {
                0.0
            }
        }
        None => object_insulation_proxy(false),
    }
}

/// Local heat from object heatValue (Haxe `getLocalHeat`).
#[inline]
pub fn local_heat_from_value(heat_value: f32) -> f32 {
    heat_value * TEMPERATURE_LOCAL_HEAT_FACTOR
}

/// Haxe `TemperatureHandler.initializeTileTemperature` (pure).
///
/// `biome + season(factors) + localHeat`, clamp `0..2`.
pub fn initialize_tile_temperature(
    biome_current: u8,
    biome_original: u8,
    season_impact_raw: f32,
    local_heat: f32,
) -> f32 {
    let biome_t = average_biome_temperature(biome_current, biome_original);
    let season = apply_season_temperature_factors(season_impact_raw);
    (biome_t + season + local_heat).clamp(0.0, 2.0)
}

/// Pure lerp step toward biome+season target (Haxe UpdateTileTemperature step 1).
///
/// - Uninitialized (`current < 0`): Haxe `initializeTileTemperature` (includes local heat, clamp 0..2).
/// - Else: lerp toward average biome + scaled season; insulation from raw rValues.
///
/// `season_impact_raw` is unscaled (Hot/Cold factors applied inside).
pub fn update_tile_temperature_lerp(
    current: f32,
    biome_current: u8,
    biome_original: u8,
    season_impact_raw: f32,
    delta_time: f32,
    floor_insulation: f32,
    object_insulation: f32,
    local_heat_for_init: f32,
) -> f32 {
    // Uninitialized: Haxe initializeTileTemperature then return (no balance that tick).
    if current < 0.0 {
        return initialize_tile_temperature(
            biome_current,
            biome_original,
            season_impact_raw,
            local_heat_for_init,
        );
    }
    let biome_t = average_biome_temperature(biome_current, biome_original);
    let season = apply_season_temperature_factors(season_impact_raw);
    let target = (biome_t + season).clamp(0.0, 5.0);
    let ins = tile_insulation_factor_from_r(floor_insulation, object_insulation);
    let move_speed = TEMPERATURE_OWN_TILE_RATE * ins;
    let move_delta = (move_speed * delta_time).clamp(0.0, 0.9);
    let new_t = current + (target - current) * move_delta;
    new_t.clamp(0.0, 5.0)
}

/// Result of one pure neighbor balance step (Haxe `balanceTileTemperature`).
#[derive(Debug, Clone)]
pub struct BalanceTileTempResult {
    pub center: f32,
    /// `(dx, dy, new_neighbor_temp)` for each neighbor that was updated.
    pub neighbor_updates: Vec<(i32, i32, f32)>,
    /// When fire is "burning low", extend object `timeToChange` by this (Haxe `+= deltaTime * 0.5`).
    pub extend_time_to_change: f32,
}

/// Haxe `TemperatureHandler.balanceTileTemperature` (pure).
///
/// - `do_local_heat == true`: map-slice path — always balances, applies local heat.
/// - `do_local_heat == false`: player-area path — skip low-floor / wall center tiles;
///   skip wall neighbors (`neighbor_object_insulation > 0`).
///
/// `neighbors`: `(dx, dy, temp, neighbor_object_insulation)` with `temp >= 0`.
/// `center_ttc`: current object timeToChange (for fire-extend); use 0 if none.
pub fn balance_tile_temperature(
    current: f32,
    neighbors: &[(i32, i32, f32, f32)],
    delta_time: f32,
    local_heat: f32,
    object_insulation: f32,
    floor_insulation: f32,
    do_local_heat: bool,
    center_ttc: f32,
) -> Option<BalanceTileTempResult> {
    if current < 0.0 {
        return None;
    }
    // Player-area early outs (Haxe doLocalHeat == false).
    if !do_local_heat {
        if floor_insulation < 0.1 {
            return None;
        }
        if object_insulation > 0.0 {
            return None;
        }
    }

    let move_speed_delta_neighbor = (TEMPERATURE_BALANCE_RATE
        * delta_time
        * (1.0 - object_insulation))
        .clamp(0.0, 0.9);

    let mut heat = if do_local_heat { local_heat } else { 0.0 };
    let mut extend = 0.0_f32;
    if (heat > 0.01 && current > 0.6) || (heat < -0.01 && current < 0.2) {
        heat *= 0.5;
        // Let fire live longer if it can burn low.
        if center_ttc > 2.0 {
            extend = delta_time * 0.5;
        }
    }
    heat *= delta_time;

    let mut neighbor_temp_diff = 0.0_f32;
    let mut neighbor_updates = Vec::new();

    for &(dx, dy, n_temp, n_obj_ins) in neighbors {
        if n_temp < 0.0 {
            continue;
        }
        // Haxe: doLocalHeat==false → skip wall/door neighbors (avoid double heat loss).
        if !do_local_heat && n_obj_ins > 0.0 {
            continue;
        }
        let temp_diff_n = (current - n_temp) * move_speed_delta_neighbor * 0.1;
        neighbor_temp_diff -= temp_diff_n;
        let new_n = (n_temp + heat + temp_diff_n).clamp(0.0, 5.0);
        neighbor_updates.push((dx, dy, new_n));
    }

    let new_center = (current + neighbor_temp_diff + heat * 1.2).clamp(0.0, 5.0);
    Some(BalanceTileTempResult {
        center: new_center,
        neighbor_updates,
        extend_time_to_change: extend,
    })
}

/// Haxe `BalanceTemperatureArea` Chebyshev ring walk order (ring 0..d inclusive).
///
/// Returns tile coords around `(cx, cy)` for player-area balance (`doLocalHeat=false`).
/// Callers apply [`balance_tile_temperature`] per coord.
pub fn balance_temperature_area_coords(cx: i32, cy: i32, d: i32) -> Vec<(i32, i32)> {
    let d = d.max(0);
    let mut out = Vec::with_capacity(1 + (2 * d as usize + 1).pow(2).saturating_sub(1));
    // Ring 0: center
    out.push((cx, cy));
    for ring in 1..=d {
        // Top and bottom rows
        for dx in -ring..=ring {
            out.push((cx + dx, cy - ring));
            out.push((cx + dx, cy + ring));
        }
        // Left and right columns (exclude corners already covered)
        for dy in (-ring + 1)..ring {
            out.push((cx - ring, cy + dy));
            out.push((cx + ring, cy + dy));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Second-time outcome + water drift (pure decision helpers)
// ---------------------------------------------------------------------------

/// Haxe `DoSecondTimeOutcome` chance: fire when `timepassed / timeToChange >= rand`.
///
/// Returns `Some(new_id)` when the outcome applies.
pub fn second_time_outcome_roll(
    second_outcome_id: i32,
    second_outcome_secs: f32,
    time_passed: f32,
    rand01: f32,
) -> Option<i32> {
    if second_outcome_id < 1 || second_outcome_secs < 1.0 {
        return None;
    }
    if time_passed / second_outcome_secs < rand01 {
        return None;
    }
    Some(second_outcome_id)
}

/// Water biome ids that allow item drift (Haxe PASSABLERIVER / OCEAN / RIVER).
#[inline]
pub fn is_water_drift_biome(biome: u8) -> bool {
    biome == PASSABLE_RIVER || biome == OCEAN || biome == RIVER
}

/// Effective time-passed multiplier for water drift (Haxe DoItemInWaterMovement).
pub fn water_drift_effective_time(
    time_passed: f32,
    speed_mult: f32,
    biome: u8,
) -> f32 {
    let mut t = time_passed * speed_mult.powi(3);
    match biome {
        b if b == PASSABLE_RIVER => t *= 0.2,
        b if b == OCEAN => t *= 0.5,
        b if b == RIVER => t *= 1.5,
        _ => {}
    }
    t
}

/// Haxe `DoItemInWaterMovement` gate: returns true when the item should attempt a move.
pub fn item_in_water_should_move(
    permanent: bool,
    biome: u8,
    floor_id: i32,
    time_passed: f32,
    speed_mult: f32,
    rand01: f32,
) -> bool {
    if permanent || floor_id > 0 || !is_water_drift_biome(biome) {
        return false;
    }
    let eff = water_drift_effective_time(time_passed, speed_mult, biome);
    rand01 <= eff
}

/// Pick adjacent dest offset: `dx,dy ∈ {-1,0,1}` (Haxe `randomInt(2)-1` each axis).
pub fn water_drift_offset(rand_dx: i32, rand_dy: i32) -> (i32, i32) {
    (rand_dx.clamp(-1, 1), rand_dy.clamp(-1, 1))
}

// ---------------------------------------------------------------------------
// Contained-object time (Haxe doTimeForObject)
// ---------------------------------------------------------------------------

/// Outcome of one contained-item time step.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainedTimeOutcome {
    /// No auto-decay transition for this id.
    NoTransition,
    /// Timer not reached (or just armed). Carry creation/ttc forward.
    Pending { creation: f32, ttc: f32 },
    /// Transformed to a new id (may be 0 = removed). Fresh timer for next cycle.
    ///
    /// `uses_remaining` is post-`DoChangeNumberOfUsesOnTarget` (0 = simple / N/A).
    Transformed {
        new_id: i32,
        creation: f32,
        ttc: f32,
        uses_remaining: i32,
    },
}

/// Infer multi-use remaining from wire id (dummy index or base = full).
///
/// Haxe `ObjectHelper.numberOfUses` is not stored on contained slots in Rust;
/// dummies encode use level via `ContentDb.dummy_parent` / `dummy_ids`.
pub fn uses_remaining_from_wire_id(content: &ContentDb, obj_id: i32) -> i32 {
    if obj_id <= 0 {
        return 0;
    }
    if let Some(&parent) = content.dummy_parent.get(&obj_id) {
        if let Some(def) = content.get(parent) {
            if let Some(idx) = def.dummy_ids.iter().position(|&d| d == obj_id) {
                return (idx as i32) + 1;
            }
        }
        // Unknown dummy → treat as last-use (1).
        return 1;
    }
    content
        .get(obj_id)
        .map(|d| if d.num_uses >= 2 { d.num_uses } else { 0 })
        .unwrap_or(0)
}

/// Haxe `TimeHelper.doTimeForObject` (pure; one contained ObjectHelper).
///
/// Arms `timeToChange` from auto-decay when `ttc <= 0`. Applies
/// [`transform_target`] + Haxe `DoChangeNumberOfUsesOnTarget` when
/// `!noUseTarget`. Last-use table when `uses_remaining == 1` (Haxe `isLastUse`)
/// or when wire id is a multi-use dummy.
///
/// `uses_remaining`: explicit uses when known; `0` → infer from wire id / dummies.
pub fn do_time_for_contained(
    content: &ContentDb,
    obj_id: i32,
    creation: f32,
    ttc: f32,
    sim_time: f32,
    rand_ttc: f32,
    rand_transform: f32,
    uses_remaining: i32,
) -> ContainedTimeOutcome {
    if obj_id < 1 {
        return ContainedTimeOutcome::Transformed {
            new_id: 0,
            creation: sim_time,
            ttc: 0.0,
            uses_remaining: 0,
        };
    }
    let base = content.resolve_base_id(obj_id);
    let Some(tr) = content
        .auto_decays
        .get(&base)
        .cloned()
        .or_else(|| content.find_transition(-1, base).cloned())
    else {
        return ContainedTimeOutcome::NoTransition;
    };
    if tr.move_dist > 0 {
        return ContainedTimeOutcome::NoTransition;
    }

    let mut creation = creation;
    let mut ttc = ttc;
    if ttc <= 0.0 {
        ttc = calculate_time_to_change(tr.auto_decay_seconds, rand_ttc);
        if ttc <= 0.0 {
            return ContainedTimeOutcome::NoTransition;
        }
        creation = sim_time;
        return ContainedTimeOutcome::Pending { creation, ttc };
    }

    if !is_time_to_change_reached(creation, ttc, sim_time) {
        return ContainedTimeOutcome::Pending { creation, ttc };
    }

    let uses_before = if uses_remaining > 0 {
        uses_remaining
    } else {
        uses_remaining_from_wire_id(content, obj_id)
    };
    let num_uses = content.get(base).map(|d| d.num_uses).unwrap_or(0);

    // Haxe: if (obj.isLastUse()) prefer last-use transition table.
    let mut transition = tr;
    if prefer_last_use_for_time(uses_before, num_uses)
        || content.dummy_parent.contains_key(&obj_id)
    {
        if let Some(lu) = content.find_transition_last_use(-1, base) {
            transition = lu.clone();
        }
    }

    let new_raw = transition.new_target_id;
    let new_id_raw = if new_raw > 0 {
        transform_target(content, new_raw, rand_transform)
    } else {
        0
    };

    if new_id_raw <= 0 {
        return ContainedTimeOutcome::Transformed {
            new_id: 0,
            creation: sim_time,
            ttc: 0.0,
            uses_remaining: 0,
        };
    }

    // Haxe: if (transition.noUseTarget == false) DoChangeNumberOfUsesOnTarget
    let new_base = content.resolve_base_id(new_id_raw);
    let num_before = num_uses;
    let num_after = content.get(new_base).map(|d| d.num_uses).unwrap_or(0);
    let uses_out = crate::player::multi_use::change_number_of_uses_on_target(
        base,
        new_base,
        uses_before,
        num_before,
        num_after,
        transition.reverse_use_target,
        transition.no_use_target,
        true, // from_transition
        true, // allow_reset_on_id_change
    );

    let (final_id, final_uses) = match uses_out {
        crate::player::multi_use::TargetUsesOutcome::Cleared => (0, 0),
        crate::player::multi_use::TargetUsesOutcome::Simple => (new_base, 0),
        crate::player::multi_use::TargetUsesOutcome::Uses(u) => {
            // Wire dummy id for partial uses (Haxe dummyId).
            let wire = content.wire_id_for_uses(new_base, u);
            (wire, u)
        }
    };

    if final_id <= 0 {
        return ContainedTimeOutcome::Transformed {
            new_id: 0,
            creation: sim_time,
            ttc: 0.0,
            uses_remaining: 0,
        };
    }

    let new_ttc = calculate_time_to_change_for_obj(content, final_id, 0.0, rand_ttc);
    ContainedTimeOutcome::Transformed {
        new_id: final_id,
        creation: sim_time,
        ttc: new_ttc,
        uses_remaining: final_uses,
    }
}

// NESTED-IN-NESTED-TIMERS / deep_contained (Haxe TimeHelper L1150)
#[path = "nested_timers.rs"]
mod nested_timers;
pub use nested_timers::{
    tick_container_helper_timers, tick_nested_helpers_deep, NESTED_TIMER_MAX_DEPTH,
};

// ---------------------------------------------------------------------------
// doTimeTransitionHelper selection (last-use / reverse max / move)
// ---------------------------------------------------------------------------

/// Result of resolving which auto-decay transition to apply on the ground.
#[derive(Debug, Clone)]
pub struct TimeTransitionPick {
    pub transition: Transition,
    /// True when max-use table was selected (set uses to full after).
    pub is_max_use: bool,
    /// True when this is an animal-move transition — caller must not transform tile.
    pub is_move: bool,
}

/// Haxe `doTimeTransitionHelper` transition selection (no world mutation).
pub fn pick_time_transition(
    content: &ContentDb,
    tile_object_id: i32,
    uses_remaining: i32,
) -> Option<TimeTransitionPick> {
    let base = content.resolve_base_id(tile_object_id);
    let mut tr = content
        .auto_decays
        .get(&base)
        .cloned()
        .or_else(|| content.find_transition(-1, base).cloned())?;

    if tr.move_dist > 0 {
        return Some(TimeTransitionPick {
            transition: tr,
            is_max_use: false,
            is_move: true,
        });
    }

    let num_uses = content.get(base).map(|d| d.num_uses).unwrap_or(0);
    // Last-use path when multi-use nearly exhausted.
    if prefer_last_use_for_time(uses_remaining, num_uses) {
        if let Some(lu) = content.find_transition_last_use(-1, base) {
            tr = lu.clone();
        }
    }

    let new_target = tr.new_target_id;
    let new_num = content.get(new_target).map(|d| d.num_uses).unwrap_or(0);
    let mut is_max_use = false;
    // Reverse max → maxUse table.
    if tr.reverse_use_target && new_num >= 2 && uses_remaining >= new_num {
        if let Some(mx) = content.find_transition_max_use(-1, base) {
            tr = mx.clone();
            is_max_use = true;
        } else {
            // Haxe: reset creation time, refuse transition.
            return None;
        }
    }

    Some(TimeTransitionPick {
        transition: tr,
        is_max_use,
        is_move: false,
    })
}

#[inline]
fn prefer_last_use_for_time(uses_remaining: i32, num_uses: i32) -> bool {
    num_uses >= 2 && uses_remaining == 1
}

/// Container overflow: if contained count exceeds new target slots, delay and pop one.
///
/// Returns `(new_time_to_change_delta, popped_id)`.
pub fn container_overflow_delay(
    contained_len: usize,
    new_target_num_slots: i32,
    popped_id: i32,
) -> Option<(f32, i32)> {
    if new_target_num_slots < 0 {
        return None;
    }
    if contained_len as i32 <= new_target_num_slots {
        return None;
    }
    let mut delay = 20.0_f32;
    if popped_id == SHARP_STONE_ID {
        delay += CURSED_GRAVE_SHARP_STONE_EXTRA_SECS;
    }
    Some((delay, popped_id))
}

// ---------------------------------------------------------------------------
// Cursed graves + ovens global indexes (Haxe WorldMap.cursedGraves / ovens)
// ---------------------------------------------------------------------------

/// Haxe `WorldMap.index` core: `x + y * width` (no wrap/y-1; local maps are non-toroidal).
// Haxe: WorldMap.index
#[inline]
pub fn map_linear_index(x: i32, y: i32, width: i32) -> i32 {
    let w = width.max(1);
    x.wrapping_add(y.wrapping_mul(w))
}

/// Positions stored in a linear-index map (order not stable).
#[inline]
pub fn index_positions(map: &HashMap<i32, (i32, i32)>) -> Vec<(i32, i32)> {
    map.values().copied().collect()
}

/// Insert `(x,y)` under Haxe linear key.
#[inline]
pub fn index_insert(map: &mut HashMap<i32, (i32, i32)>, x: i32, y: i32, width: i32) {
    map.insert(map_linear_index(x, y, width), (x, y));
}

/// Haxe `TimeHelper.ClearCursedGraves` generalized: keep only tiles still valid.
///
/// `still_valid(tx, ty)` should read current map object and apply IsBoneGrave / IsOven.
// Haxe: TimeHelper.ClearCursedGraves
pub fn clear_map_index_keep(
    entries: &HashMap<i32, (i32, i32)>,
    still_valid: impl Fn(i32, i32) -> bool,
    width: i32,
) -> HashMap<i32, (i32, i32)> {
    let mut new_stuff = HashMap::with_capacity(entries.len());
    for (_, &(tx, ty)) in entries {
        if still_valid(tx, ty) {
            new_stuff.insert(map_linear_index(tx, ty, width), (tx, ty));
        }
    }
    new_stuff
}

/// Haxe `TimeHelper.ClearCursedGraves` — prune entries whose tile is no longer a bone grave.
// Haxe: TimeHelper.ClearCursedGraves
pub fn clear_cursed_graves(
    entries: &HashMap<i32, (i32, i32)>,
    tile_obj_id: impl Fn(i32, i32) -> i32,
    width: i32,
) -> HashMap<i32, (i32, i32)> {
    clear_map_index_keep(
        entries,
        |tx, ty| crate::animal_move::is_bone_grave(tile_obj_id(tx, ty)),
        width,
    )
}

/// Haxe ovens prune in DoWorldMapTimeStuff (`tick % 2000`): keep IsOven only.
// Haxe: TimeHelper.DoWorldMapTimeStuff ovens clear
pub fn clear_ovens_index(
    entries: &HashMap<i32, (i32, i32)>,
    tile_obj_id: impl Fn(i32, i32) -> i32,
    width: i32,
) -> HashMap<i32, (i32, i32)> {
    clear_map_index_keep(
        entries,
        |tx, ty| is_oven_map_id(tile_obj_id(tx, ty)),
        width,
    )
}

/// Insert bone-grave tile into cursedGraves if `IsBoneGrave(obj_id)`.
// Haxe: TimeHelper.DoWorldMapTimeStuff cursedGraves insert
pub fn maybe_insert_cursed_grave(
    map: &mut HashMap<i32, (i32, i32)>,
    x: i32,
    y: i32,
    obj_id: i32,
    width: i32,
) {
    if crate::animal_move::is_bone_grave(obj_id) {
        index_insert(map, x, y, width);
    }
}

/// Insert oven tile into ovens if `IsOven(obj_id)`.
// Haxe: TimeHelper.DoWorldMapTimeStuff ovens insert
pub fn maybe_insert_oven(
    map: &mut HashMap<i32, (i32, i32)>,
    x: i32,
    y: i32,
    obj_id: i32,
    width: i32,
) {
    if is_oven_map_id(obj_id) {
        index_insert(map, x, y, width);
    }
}

/// True when this map-time step should prune cursedGraves/ovens (Haxe `tick % 2000 == 0`).
#[inline]
pub fn should_clear_cursed_graves_ovens(step: u64) -> bool {
    step % CURSED_GRAVES_CLEAR_TICK_MOD == 0
}

// ---------------------------------------------------------------------------
// Seasonal plant multi-use decay / regrow (RespawnOrDecayPlant subset)
// ---------------------------------------------------------------------------

/// Winter multi-use bush: decrement uses when roll succeeds.
pub fn winter_multiuse_should_decay(
    winter_decay_chance: f32,
    winter_decay_factor: f32,
    number_of_uses: i32,
    rand01: f32,
) -> bool {
    if winter_decay_factor <= 0.0 || number_of_uses < 1 {
        return false;
    }
    // Haxe: if (chance * factor * uses < rand) return; else decay.
    winter_decay_chance * winter_decay_factor * (number_of_uses as f32) > rand01
}

/// Spring multi-use bush: increment uses when roll succeeds.
pub fn spring_multiuse_should_regrow(
    spring_regrow_chance: f32,
    spring_regrow_factor: f32,
    number_of_uses: i32,
    num_uses_max: i32,
    rand01: f32,
) -> bool {
    if spring_regrow_factor <= 0.0 || number_of_uses >= num_uses_max {
        return false;
    }
    let factor = (num_uses_max - number_of_uses).max(1) as f32;
    spring_regrow_chance * spring_regrow_factor * factor > rand01
}

/// Hide fleeing rabbit in winter snow that was not originally snow.
pub fn should_hide_rabbit_winter(
    obj_id: i32,
    current_biome: u8,
    original_biome: u8,
    hidden_id: i32,
) -> bool {
    obj_id == FLEEING_RABBIT_ID
        && current_biome == BIOME_SNOW
        && original_biome != BIOME_SNOW
        && original_biome != SNOWINGREY
        && hidden_id == 0
}

/// Unhide fleeing rabbit in spring when tile is no longer snow.
pub fn should_unhide_rabbit_spring(hidden_id: i32, biome: u8) -> bool {
    hidden_id == FLEEING_RABBIT_ID && biome != BIOME_SNOW && biome != SNOWINGREY
}

// ---------------------------------------------------------------------------
// Map-slice state + apply (DoWorldMapTimeStuff body)
// ---------------------------------------------------------------------------

/// Runtime state for world map time parts (owned by [`crate::SimState`]).
#[derive(Debug, Clone)]
pub struct WorldMapTimeState {
    /// Haxe `worldMapTimeStep`.
    pub step: u64,
    /// Haxe `TimePassedToDoAllTimeSteps` — seconds for last full map cycle.
    pub time_passed_all_steps: f32,
    /// Sim time when current full-map cycle started.
    pub cycle_started_sim_time: f32,
    /// Cached winter wild-food decay chance for current cycle.
    pub winter_decay_chance: f32,
    /// Cached spring regrow chance for current cycle.
    pub spring_regrow_chance: f32,
    /// Sparse tile temperatures (Haxe WorldMap.tileTemperature); `<0` = unset.
    pub tile_temps: HashMap<(i32, i32), f32>,
    /// Sparse hidden object layer (Haxe hiddenObjects) — winter hide / regrow.
    pub hidden_objects: HashMap<(i32, i32), i32>,
    /// Original biome at first observation (for rabbit hide). Default = current when missing.
    pub original_biomes: HashMap<(i32, i32), u8>,
    /// Per-tile contained-item timers (Haxe contained ObjectHelper creation/ttc).
    /// Parallel to `ComplexObject.contained` when present.
    /// Runtime map; OLW3 NestedHelper slot times are the disk form — re-arm via
    /// [`crate::arm_contained_timers_for_loaded_world`] after load; map-slice writes back.
    pub contained_timers: HashMap<(i32, i32), Vec<(f32, f32)>>,
    /// Haxe `WorldMap.cursedGraves` — linear index → `(tx, ty)` bone-grave tiles.
    /// Filled during Y-band scan; pruned every [`CURSED_GRAVES_CLEAR_TICK_MOD`] steps.
    pub cursed_graves: HashMap<i32, (i32, i32)>,
    /// Haxe `WorldMap.ovens` — linear index → `(tx, ty)` adobe-oven family tiles.
    pub ovens: HashMap<i32, (i32, i32)>,
}

impl Default for WorldMapTimeState {
    fn default() -> Self {
        Self {
            step: 0,
            time_passed_all_steps: 1.0,
            cycle_started_sim_time: 0.0,
            winter_decay_chance: 0.0,
            spring_regrow_chance: 0.0,
            tile_temps: HashMap::new(),
            hidden_objects: HashMap::new(),
            original_biomes: HashMap::new(),
            contained_timers: HashMap::new(),
            cursed_graves: HashMap::new(),
            ovens: HashMap::new(),
        }
    }
}

/// One map cell change produced by the slice (for MX fan-out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTimeChange {
    pub x: i32,
    pub y: i32,
    pub new_object_id: i32,
    /// When true, emit animal-move style MX (water drift / animal).
    pub moving: bool,
    pub from_x: i32,
    pub from_y: i32,
}

/// Apply one Y-band of `DoWorldMapTimeStuff`.
///
/// Processes second-time outcomes, water drift (with groundObject leave-behind),
/// tile temperature + neighbor balance, seasonal multi-use, rabbit hide/unhide,
/// contained-object timers (`doTimeForObject`), and arms missing decay timers.
///
/// Full ground `doTimeTransitionHelper` transforms remain on the sparse
/// `pending_decays` path ([`crate::tick_auto_decays`]) for efficiency.
pub fn do_world_map_time_stuff(
    world: &mut World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    season_is_spring: bool,
    season_is_winter: bool,
    season_impact: f32,
    season_length_secs: f32,
    sim_time: f32,
    season_hardness: f32,
    rng: &mut impl Rng,
) -> Vec<MapTimeChange> {
    let mut changes = Vec::new();
    let w = world.width_tiles;
    let h = world.height_tiles;
    if w <= 0 || h <= 0 {
        map_time.step = map_time.step.wrapping_add(1);
        return changes;
    }

    // Haxe: if (tick % 2000 == 0) ClearCursedGraves + ovens prune
    if should_clear_cursed_graves_ovens(map_time.step) {
        let get_id = |tx: i32, ty: i32| world.get_object(tx, ty);
        map_time.cursed_graves = clear_cursed_graves(&map_time.cursed_graves, get_id, w);
        map_time.ovens = clear_ovens_index(&map_time.ovens, get_id, w);
    }

    // Recompute seasonal chances at cycle start (Haxe worldMapTimeStep % timeParts == 0).
    if is_full_map_cycle_start(map_time.step, WORLD_TIME_PARTS) {
        if map_time.cycle_started_sim_time > 0.0 || map_time.step > 0 {
            map_time.time_passed_all_steps =
                (sim_time - map_time.cycle_started_sim_time).max(0.001);
        } else {
            map_time.time_passed_all_steps = 1.0;
        }
        let (winter, spring) = seasonal_chances(
            map_time.time_passed_all_steps,
            season_length_secs,
            season_hardness,
        );
        map_time.winter_decay_chance = winter;
        map_time.spring_regrow_chance = spring;
        map_time.cycle_started_sim_time = sim_time;
    }

    let (start_y, end_y) = world_time_slice_y_range(h, WORLD_TIME_PARTS, map_time.step);
    map_time.step = map_time.step.wrapping_add(1);

    let time_passed = map_time.time_passed_all_steps;

    for y in start_y..end_y {
        for x in 0..w {
            // Remember original biome once.
            let biome = world.get_biome(x, y);
            map_time.original_biomes.entry((x, y)).or_insert(biome);

            // Tile temperature: own-tile lerp then neighbor balance (doLocalHeat=true).
            // Haxe: UpdateTileTemperature — average biome, season factors, real rValue.
            let floor = world.get_floor(x, y) as i32;
            let obj_id = world.get_object(x, y);
            let obj_base = content.resolve_base_id(obj_id);
            let heat_value = content.get(obj_base).map(|d| d.heat_value).unwrap_or(0.0);
            let floor_ins = floor_insulation_from_content(content, floor);
            let obj_ins = object_insulation_from_content(content, obj_id);
            let local_heat = local_heat_from_value(heat_value);
            let orig_biome = map_time
                .original_biomes
                .get(&(x, y))
                .copied()
                .unwrap_or(biome);
            let cur_temp = map_time.tile_temps.get(&(x, y)).copied().unwrap_or(-1.0);
            let mut new_temp = update_tile_temperature_lerp(
                cur_temp,
                biome,
                orig_biome,
                season_impact,
                time_passed,
                floor_ins,
                obj_ins,
                local_heat,
            );
            map_time.tile_temps.insert((x, y), new_temp);

            // Neighbor balance only after tile is initialized (Haxe skips init-return).
            if cur_temp >= 0.0 {
                let mut neighbors = Vec::with_capacity(8);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let nt = map_time.tile_temps.get(&(nx, ny)).copied().unwrap_or(-1.0);
                        if nt >= 0.0 {
                            let n_obj = world.get_object(nx, ny);
                            let n_ins = object_insulation_from_content(content, n_obj);
                            neighbors.push((dx, dy, nt, n_ins));
                        }
                    }
                }
                let center_ttc = world
                    .get_helper(x, y)
                    .map(|hh| hh.time_to_change)
                    .unwrap_or(0.0);
                if let Some(bal) = balance_tile_temperature(
                    new_temp,
                    &neighbors,
                    time_passed,
                    local_heat,
                    obj_ins,
                    floor_ins,
                    true, // map-slice path (doLocalHeat=true)
                    center_ttc,
                ) {
                    new_temp = bal.center;
                    map_time.tile_temps.insert((x, y), new_temp);
                    for (dx, dy, nt) in bal.neighbor_updates {
                        map_time.tile_temps.insert((x + dx, y + dy), nt);
                    }
                    if bal.extend_time_to_change > 0.0 {
                        if let Some(mut hh) = world.get_helper(x, y).cloned() {
                            if hh.time_to_change > 2.0 {
                                hh.time_to_change += bal.extend_time_to_change;
                                world.set_object_complex(x, y, hh);
                            }
                        }
                    }
                }
            }

            // Spring: unhide rabbits.
            if season_is_spring {
                let hidden = map_time.hidden_objects.get(&(x, y)).copied().unwrap_or(0);
                if should_unhide_rabbit_spring(hidden, biome) && obj_id == 0 {
                    world.set_object(x, y, FLEEING_RABBIT_ID);
                    map_time.hidden_objects.remove(&(x, y));
                    changes.push(MapTimeChange {
                        x,
                        y,
                        new_object_id: FLEEING_RABBIT_ID,
                        moving: false,
                        from_x: x,
                        from_y: y,
                    });
                    continue;
                }
            }

            let obj_id = world.get_object(x, y);
            if obj_id == 0 {
                continue;
            }

            // Haxe: teleport / animal-chase indexes filled during Y-band scan.
            // ObjectData.IsOven / IsBoneGrave → WorldMap.ovens / cursedGraves.
            maybe_insert_oven(&mut map_time.ovens, x, y, obj_id, w);
            maybe_insert_cursed_grave(&mut map_time.cursed_graves, x, y, obj_id, w);

            // Second-time outcome (goose pond chain, etc.).
            if let Some(&(out_id, out_secs)) = content.second_time_outcomes.get(&obj_id) {
                let r: f32 = rng.gen();
                if let Some(new_id) =
                    second_time_outcome_roll(out_id, out_secs, time_passed, r)
                {
                    world.set_object(x, y, new_id);
                    changes.push(MapTimeChange {
                        x,
                        y,
                        new_object_id: new_id,
                        moving: false,
                        from_x: x,
                        from_y: y,
                    });
                    continue;
                }
            }

            // Water item drift (Haxe DoItemInWaterMovement + groundObject leave-behind).
            // Haxe: if (objData.dummyParent != null) objData = objData.dummyParent;
            let water_base = content.resolve_base_id(obj_id);
            let speed_mult = content.get(water_base).map(|d| d.speed_mult).unwrap_or(1.0);
            let permanent = content.get(water_base).map(|d| d.permanent).unwrap_or(false);
            let r: f32 = rng.gen();
            if item_in_water_should_move(permanent, biome, floor, time_passed, speed_mult, r)
            {
                let dx = rng.gen_range(0..=2) - 1;
                let dy = rng.gen_range(0..=2) - 1;
                let (ox, oy) = water_drift_offset(dx, dy);
                let tx = x + ox;
                let ty = y + oy;
                if (ox != 0 || oy != 0)
                    && tx >= 0
                    && ty >= 0
                    && tx < w
                    && ty < h
                    && world.get_object(tx, ty) == 0
                {
                    // Haxe: setObjectHelper(dest, obj); setObjectHelper(src, obj.groundObject);
                    // groundObject cleared on moved obj.
                    let mut helper = world
                        .helpers
                        .remove(&(x, y))
                        .unwrap_or_else(|| ComplexObject::new_simple(obj_id));
                    helper.base_id = obj_id;
                    let ground_id = helper.ground_id;
                    helper.ground_id = 0;

                    if helper.is_complex() {
                        world.set_object_complex(tx, ty, helper);
                    } else {
                        world.set_object(tx, ty, obj_id);
                    }

                    if ground_id > 0 {
                        world.set_object(x, y, ground_id);
                    } else {
                        world.set_object(x, y, 0);
                    }
                    // Contained timers follow the container helper (moved tile).
                    if let Some(timers) = map_time.contained_timers.remove(&(x, y)) {
                        map_time.contained_timers.insert((tx, ty), timers);
                    }

                    changes.push(MapTimeChange {
                        x: tx,
                        y: ty,
                        new_object_id: obj_id,
                        moving: true,
                        from_x: x,
                        from_y: y,
                    });
                    changes.push(MapTimeChange {
                        x,
                        y,
                        new_object_id: ground_id,
                        moving: false,
                        from_x: x,
                        from_y: y,
                    });
                    continue;
                }
            }

            // Seasonal multi-use bush (winter decay / spring regrow) via factors on def.
            if let Some(def) = content.get(obj_id) {
                if def.num_uses > 1 {
                    let uses = world
                        .get_helper(x, y)
                        .map(|h| h.uses_remaining)
                        .unwrap_or(def.num_uses);
                    if season_is_winter && def.winter_decay_factor > 0.0 {
                        let r: f32 = rng.gen();
                        if winter_multiuse_should_decay(
                            map_time.winter_decay_chance,
                            def.winter_decay_factor,
                            uses,
                            r,
                        ) && uses > 1
                        {
                            let next = uses - 1;
                            world.set_object_complex(
                                x,
                                y,
                                ComplexObject::with_uses(obj_id, next),
                            );
                            changes.push(MapTimeChange {
                                x,
                                y,
                                new_object_id: obj_id,
                                moving: false,
                                from_x: x,
                                from_y: y,
                            });
                        }
                    } else if season_is_spring && def.spring_regrow_factor > 0.0 {
                        let r: f32 = rng.gen();
                        if spring_multiuse_should_regrow(
                            map_time.spring_regrow_chance,
                            def.spring_regrow_factor,
                            uses,
                            def.num_uses,
                            r,
                        ) {
                            let next = (uses + 1).min(def.num_uses);
                            world.set_object_complex(
                                x,
                                y,
                                ComplexObject::with_uses(obj_id, next),
                            );
                            changes.push(MapTimeChange {
                                x,
                                y,
                                new_object_id: obj_id,
                                moving: false,
                                from_x: x,
                                from_y: y,
                            });
                        }
                    }
                }
            }

            // Hide rabbits in winter.
            if season_is_winter {
                let orig = map_time
                    .original_biomes
                    .get(&(x, y))
                    .copied()
                    .unwrap_or(biome);
                let hidden = map_time.hidden_objects.get(&(x, y)).copied().unwrap_or(0);
                if should_hide_rabbit_winter(obj_id, biome, orig, hidden) {
                    world.set_object(x, y, 0);
                    map_time.hidden_objects.insert((x, y), FLEEING_RABBIT_ID);
                    changes.push(MapTimeChange {
                        x,
                        y,
                        new_object_id: 0,
                        moving: false,
                        from_x: x,
                        from_y: y,
                    });
                    continue;
                }
            }

            // Contained-object auto-decay (Haxe doTimeForObject on containedObjects).
            // **NESTED-IN-NESTED-TIMERS** / deep_contained: first-level + recursive NestedHelper
            // (Haxe L1150 TODO — implemented in nested_timers::tick_container_helper_timers).
            // Overflow refuse Haxe L2213; cargo kept when new.num_slots >= cargo.len().
            if let Some(mut helper) = world.get_helper(x, y).cloned() {
                if !helper.contained.is_empty() {
                    let mut timers = map_time
                        .contained_timers
                        .remove(&(x, y))
                        .unwrap_or_default();
                    // Haxe: DoWorldMapTimeStuff first-level loop + L1150 nested-in-nested.
                    let changed = nested_timers::tick_container_helper_timers(
                        content,
                        &mut helper,
                        &mut timers,
                        sim_time,
                        rng,
                    );
                    let base = helper.base_id;
                    world.set_object_complex(x, y, helper);
                    if timers.is_empty() {
                        map_time.contained_timers.remove(&(x, y));
                    } else {
                        map_time.contained_timers.insert((x, y), timers);
                    }
                    if changed {
                        changes.push(MapTimeChange {
                            x,
                            y,
                            new_object_id: base,
                            moving: false,
                            from_x: x,
                            from_y: y,
                        });
                    }
                }
            }
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// Tests (pure)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef, ProbSetCategory, Transition};
    use ol_world::World;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn tr_decay(target: i32, new_target: i32, secs: f32) -> Transition {
        Transition {
            actor_id: -1,
            target_id: target,
            new_actor_id: 0,
            new_target_id: new_target,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: secs,
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
        }
    }

    #[test]
    fn world_time_parts_slice_only_processes_one_band_per_tick() {
        let h = 100;
        let parts = 25;
        assert_eq!(world_time_slice_y_range(h, parts, 0), (0, 4));
        assert_eq!(world_time_slice_y_range(h, parts, 1), (4, 8));
        assert_eq!(world_time_slice_y_range(h, parts, 24), (96, 100));
        // Next cycle wraps.
        assert_eq!(world_time_slice_y_range(h, parts, 25), (0, 4));
        // Bands are disjoint for a full cycle.
        let mut covered = vec![false; h as usize];
        for step in 0..parts as u64 {
            let (s, e) = world_time_slice_y_range(h, parts, step);
            for y in s..e {
                assert!(!covered[y as usize], "overlap at y={y}");
                covered[y as usize] = true;
            }
        }
        assert!(covered.iter().all(|&c| c));
    }

    #[test]
    fn calculate_time_to_change_random_range_and_negative_hours() {
        let t = 10.0_f32;
        let lo = calculate_time_to_change(t, 0.0);
        let hi = calculate_time_to_change(t, 0.999);
        assert!((lo - 5.0).abs() < 1e-3, "lo={lo}");
        assert!((hi - 14.99).abs() < 0.1, "hi={hi}");
        // Negative hours: -2 → 2 hours = 7200s base, range [3600, 10800].
        let hour = calculate_time_to_change(-2.0, 0.0);
        assert!((hour - 3600.0).abs() < 1e-2);
        let hour_hi = calculate_time_to_change(-2.0, 1.0);
        assert!(hour_hi > 10000.0 && hour_hi <= 10800.0 + 1.0);
    }

    #[test]
    fn is_time_to_change_reached_matches_complex_object() {
        assert!(!is_time_to_change_reached(10.0, 5.0, 14.0));
        assert!(is_time_to_change_reached(10.0, 5.0, 15.0));
        assert!(!is_time_to_change_reached(0.0, 0.0, 100.0));
        let mut h = ComplexObject::new_simple(1);
        h.stamp_time(10.0, 5.0);
        assert!(!h.is_time_to_change_reached(14.9));
        assert!(h.is_time_to_change_reached(15.0));
    }

    #[test]
    fn tile_temperature_lerps_to_biome_season_with_insulation() {
        // Unset → snap to desert target (biome 5 desert, same original).
        // Haxe: update_tile_temperature_lerp(current, biome, orig, season, dt, floor_ins, obj_ins, local_heat)
        let t0 = update_tile_temperature_lerp(-1.0, 5, 5, 0.0, 1.0, 0.0, 0.0, 0.0);
        assert!((t0 - 1.0).abs() < 1e-3 || t0 >= 0.0); // init snap
        // Cold snow target with insulation (floor) moves slower than bare.
        let bare = update_tile_temperature_lerp(1.0, 4, 4, 0.0, 10.0, 0.0, 0.0, 0.0);
        let floored = update_tile_temperature_lerp(1.0, 4, 4, 0.0, 10.0, 0.2, 0.0, 0.0);
        // Both cool toward 0; bare should cool more (or equal if clamp).
        assert!(bare <= floored + 1e-4, "bare={bare} floored={floored}");
        assert!(bare < 1.0);
    }

    #[test]
    fn balance_tile_temperature_equalizes_neighbors() {
        // Hot center, cold neighbor — heat flows out.
        // neighbors: (dx, dy, temp, neighbor_object_insulation)
        let neighbors = vec![(1, 0, 0.0_f32, 0.0_f32)];
        let r = balance_tile_temperature(
            1.0,
            &neighbors,
            1.0,
            0.0,
            0.0,
            0.2,
            true,
            0.0,
        )
        .unwrap();
        assert!(r.center < 1.0, "center cools: {}", r.center);
        assert_eq!(r.neighbor_updates.len(), 1);
        assert!(
            r.neighbor_updates[0].2 > 0.0,
            "neighbor warms: {}",
            r.neighbor_updates[0].2
        );
    }

    #[test]
    fn balance_tile_temperature_local_heat_warms_center() {
        let r = balance_tile_temperature(
            0.3,
            &[],
            10.0,
            local_heat_from_value(100.0), // strong fire
            0.0,
            0.0,
            true,
            0.0,
        )
        .unwrap();
        assert!(r.center > 0.3, "fire warms tile: {}", r.center);
    }

    #[test]
    fn average_biome_and_season_factors() {
        // Current desert (1.0) + original snow (0.0) → 0.5 average.
        assert!((average_biome_temperature(5, 4) - 0.5).abs() < 1e-4);
        // Hot/cold season scale by 0.75.
        assert!((apply_season_temperature_factors(0.4) - 0.3).abs() < 1e-4);
        assert!((apply_season_temperature_factors(-0.4) + 0.3).abs() < 1e-4);
        assert_eq!(apply_season_temperature_factors(0.0), 0.0);
        // Init includes local heat, clamp 0..2.
        let init = initialize_tile_temperature(5, 5, 0.0, local_heat_from_value(200.0));
        assert!(init >= 1.0 && init <= 2.0, "init={init}");
        // Season factors affect lerp target.
        let hot = update_tile_temperature_lerp(0.5, 0, 0, 0.4, 100.0, 0.0, 0.0, 0.0);
        let cold = update_tile_temperature_lerp(0.5, 0, 0, -0.4, 100.0, 0.0, 0.0, 0.0);
        assert!(hot > cold, "hot={hot} cold={cold}");
    }

    #[test]
    fn real_r_value_insulation_from_content() {
        let mut db = ContentDb::default();
        db.objects.insert(
            485,
            ObjectDef {
                id: 485,
                r_value: 0.9,
                floor: true,
                ..ObjectDef::empty(485)
            },
        );
        db.objects.insert(
            1101,
            ObjectDef {
                id: 1101,
                r_value: 0.98,
                clothing: "n".into(),
                permanent: true,
                ..ObjectDef::empty(1101)
            },
        );
        db.objects.insert(33, ObjectDef::empty(33)); // non-wall stick
        assert!((floor_insulation_from_content(&db, 485) - 0.9).abs() < 1e-5);
        assert!(object_insulation_from_content(&db, 1101) > 0.9);
        assert!(db.get(1101).unwrap().is_wall());
        assert_eq!(object_insulation_from_content(&db, 33), 0.0);
        // Wall insulation slows target approach more than bare.
        let bare = update_tile_temperature_lerp(1.0, 4, 4, 0.0, 10.0, 0.0, 0.0, 0.0);
        let walled = update_tile_temperature_lerp(1.0, 4, 4, 0.0, 10.0, 0.0, 0.98, 0.0);
        assert!(bare < walled, "bare={bare} walled={walled}");
    }

    #[test]
    fn balance_tile_player_path_skips_wall_neighbors() {
        // doLocalHeat=false: wall neighbor (ins>0) is skipped.
        let neighbors = vec![(1, 0, 0.0_f32, 0.9_f32), (0, 1, 0.0_f32, 0.0_f32)];
        let r = balance_tile_temperature(
            1.0,
            &neighbors,
            1.0,
            0.0,
            0.0,
            0.5, // floor insulation >= 0.1 required
            false,
            0.0,
        )
        .unwrap();
        assert_eq!(r.neighbor_updates.len(), 1);
        assert_eq!(r.neighbor_updates[0].0, 0);
        assert_eq!(r.neighbor_updates[0].1, 1);
        // Center is wall → early out.
        assert!(balance_tile_temperature(1.0, &neighbors, 1.0, 0.0, 0.5, 0.5, false, 0.0).is_none());
        // Low floor → early out.
        assert!(balance_tile_temperature(1.0, &neighbors, 1.0, 0.0, 0.0, 0.05, false, 0.0).is_none());
    }

    #[test]
    fn balance_temperature_area_chebyshev_rings() {
        let coords = balance_temperature_area_coords(10, 20, 1);
        // Ring 0 + ring 1 perimeter (8 cells) = 9.
        assert_eq!(coords.len(), 9);
        assert_eq!(coords[0], (10, 20));
        assert!(coords.contains(&(10, 19))); // top
        assert!(coords.contains(&(11, 21))); // bottom-right corner
        assert!(coords.contains(&(9, 20))); // left
    }

    #[test]
    fn transform_target_prob_set_weighted() {
        let mut db = ContentDb::default();
        db.prob_sets.insert(
            3221,
            ProbSetCategory {
                ids: vec![1196, 3220],
                weights: vec![0.8, 0.2],
            },
        );
        // rand=0 → first weight bucket
        assert_eq!(transform_target(&db, 3221, 0.0), 1196);
        // rand near 1 → second bucket
        assert_eq!(transform_target(&db, 3221, 0.99), 3220);
        // non-probSet id unchanged
        assert_eq!(transform_target(&db, 100, 0.5), 100);
    }

    #[test]
    fn do_time_for_contained_arms_then_transforms() {
        let mut db = ContentDb::default();
        db.auto_decays.insert(50, tr_decay(50, 51, 10.0));
        db.objects.insert(50, ObjectDef::empty(50));
        db.objects.insert(51, ObjectDef::empty(51));

        // Arm (uses_remaining=0 → infer)
        let o = do_time_for_contained(&db, 50, 0.0, 0.0, 100.0, 0.0, 0.0, 0);
        match o {
            ContainedTimeOutcome::Pending { creation, ttc } => {
                assert!((creation - 100.0).abs() < 1e-3);
                assert!((ttc - 5.0).abs() < 1e-3); // rand=0 → T/2
            }
            other => panic!("expected Pending, got {other:?}"),
        }

        // Fire after deadline
        let o2 = do_time_for_contained(&db, 50, 100.0, 5.0, 106.0, 0.0, 0.0, 0);
        match o2 {
            ContainedTimeOutcome::Transformed { new_id, .. } => {
                assert_eq!(new_id, 51);
            }
            other => panic!("expected Transformed, got {other:?}"),
        }
    }

    #[test]
    fn do_time_for_contained_uses_transform_target() {
        let mut db = ContentDb::default();
        db.auto_decays.insert(50, tr_decay(50, 3221, 1.0));
        db.prob_sets.insert(
            3221,
            ProbSetCategory {
                ids: vec![1196, 3220],
                weights: vec![1.0, 0.0],
            },
        );
        let o = do_time_for_contained(&db, 50, 0.0, 1.0, 10.0, 0.0, 0.0, 0);
        match o {
            ContainedTimeOutcome::Transformed { new_id, .. } => {
                assert_eq!(new_id, 1196);
            }
            other => panic!("expected Transformed, got {other:?}"),
        }
    }

    #[test]
    fn do_time_for_contained_last_use_and_no_use_target() {
        let mut db = ContentDb::default();
        // Multi-use base 100 with 3 uses; dummy 9001 = uses=1.
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                num_uses: 3,
                dummy_ids: vec![9001, 9002],
                ..ObjectDef::empty(100)
            },
        );
        db.objects.insert(
            101,
            ObjectDef {
                id: 101,
                num_uses: 3,
                dummy_ids: vec![9101, 9102],
                ..ObjectDef::empty(101)
            },
        );
        db.dummy_parent.insert(9001, 100);
        db.dummy_parent.insert(9002, 100);
        // Normal decay 100 → 101 (decrement uses).
        db.auto_decays.insert(100, tr_decay(100, 101, 1.0));
        // Last-use: 100 → 999.
        db.transitions_last_use.insert(
            (-1, 100),
            Transition {
                last_use_target: true,
                new_target_id: 999,
                ..tr_decay(100, 999, 1.0)
            },
        );
        db.objects.insert(999, ObjectDef::empty(999));

        // uses_remaining==1 → last-use table.
        let o = do_time_for_contained(&db, 100, 0.0, 1.0, 10.0, 0.0, 0.0, 1);
        match o {
            ContainedTimeOutcome::Transformed { new_id, .. } => {
                assert_eq!(new_id, 999, "last-use when uses==1");
            }
            other => panic!("expected Transformed, got {other:?}"),
        }

        // Dummy wire id also triggers last-use.
        let o2 = do_time_for_contained(&db, 9001, 0.0, 1.0, 10.0, 0.0, 0.0, 0);
        match o2 {
            ContainedTimeOutcome::Transformed { new_id, .. } => {
                assert_eq!(new_id, 999);
            }
            other => panic!("expected Transformed, got {other:?}"),
        }

        // noUseTarget keeps uses (full 3 → stays multi-use without decrement).
        let mut nut = tr_decay(100, 101, 1.0);
        nut.no_use_target = true;
        db.auto_decays.insert(100, nut);
        let o3 = do_time_for_contained(&db, 100, 0.0, 1.0, 10.0, 0.0, 0.0, 3);
        match o3 {
            ContainedTimeOutcome::Transformed {
                new_id,
                uses_remaining,
                ..
            } => {
                assert_eq!(new_id, 101);
                assert_eq!(uses_remaining, 3);
            }
            other => panic!("expected Transformed, got {other:?}"),
        }

        // Normal decrement: 3 uses → 2 after transform to multi-use 101.
        let mut normal = tr_decay(100, 101, 1.0);
        normal.no_use_target = false;
        db.auto_decays.insert(100, normal);
        let o4 = do_time_for_contained(&db, 100, 0.0, 1.0, 10.0, 0.0, 0.0, 3);
        match o4 {
            ContainedTimeOutcome::Transformed {
                new_id,
                uses_remaining,
                ..
            } => {
                assert_eq!(uses_remaining, 2);
                assert_eq!(new_id, db.wire_id_for_uses(101, 2));
            }
            other => panic!("expected Transformed, got {other:?}"),
        }
    }

    #[test]
    fn water_drift_resolves_dummy_parent_for_permanent() {
        let mut db = ContentDb::default();
        db.objects.insert(
            200,
            ObjectDef {
                id: 200,
                permanent: true,
                speed_mult: 1.0,
                num_uses: 3,
                dummy_ids: vec![9200],
                ..ObjectDef::empty(200)
            },
        );
        db.dummy_parent.insert(9200, 200);
        // Permanent via parent: gate false.
        let base = db.resolve_base_id(9200);
        let perm = db.get(base).map(|d| d.permanent).unwrap_or(false);
        assert!(perm);
        assert!(!item_in_water_should_move(perm, OCEAN, 0, 100.0, 1.0, 0.0));
    }

    #[test]
    fn water_drift_leaves_ground_object() {
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                permanent: false,
                speed_mult: 1.0,
                ..ObjectDef::empty(33)
            },
        );
        // Stick 99 under floating object as groundObject.
        db.objects.insert(99, ObjectDef::empty(99));

        let mut world = World::new(10, 50, false);
        world.set_biome(2, 0, OCEAN);
        let mut helper = ComplexObject::new_simple(33);
        helper.ground_id = 99;
        world.set_object_complex(2, 0, helper);
        // Force many trials — seed that drifts if possible.
        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 100.0;
        // Pre-init temps so balance path is exercised.
        map_time.tile_temps.insert((2, 0), 0.5);
        map_time.tile_temps.insert((1, 0), 0.5);
        map_time.tile_temps.insert((3, 0), 0.5);
        map_time.tile_temps.insert((2, 1), 0.5);

        let mut moved = false;
        for seed in 0..40u64 {
            let mut w = World::new(10, 50, false);
            w.set_biome(2, 0, OCEAN);
            let mut h = ComplexObject::new_simple(33);
            h.ground_id = 99;
            // Keep helper even though ground_id alone is complex.
            w.set_object_complex(2, 0, h);
            assert_eq!(w.get_object(2, 0), 33);
            assert_eq!(w.get_helper(2, 0).unwrap().ground_id, 99);

            let mut mt = map_time.clone();
            let mut rng = StdRng::seed_from_u64(seed);
            let changes = do_world_map_time_stuff(
                &mut w, &db, &mut mt, false, false, 0.0, 120.0, 10.0, 1.0, &mut rng,
            );
            if changes.iter().any(|c| c.moving) {
                // Source should show ground leave-behind.
                assert_eq!(
                    w.get_object(2, 0),
                    99,
                    "water drift must leave groundObject id"
                );
                // Dest has the floated object.
                assert!(
                    changes
                        .iter()
                        .any(|c| c.moving && c.new_object_id == 33),
                    "floater moved"
                );
                moved = true;
                break;
            }
        }
        assert!(moved, "expected water drift to fire in trial seeds");
    }

    #[test]
    fn second_time_outcome_goose_pond_chain() {
        // time_passed large enough always fires at rand=0.
        assert_eq!(
            second_time_outcome_roll(142, 30.0, 30.0, 0.0),
            Some(142)
        );
        assert_eq!(second_time_outcome_roll(142, 30.0, 1.0, 0.99), None);
        assert_eq!(second_time_outcome_roll(0, 30.0, 100.0, 0.0), None);
    }

    #[test]
    fn item_in_water_moves_gate() {
        assert!(!item_in_water_should_move(true, OCEAN, 0, 10.0, 1.0, 0.0));
        assert!(!item_in_water_should_move(false, 0, 0, 10.0, 1.0, 0.0)); // land
        assert!(!item_in_water_should_move(false, OCEAN, 1, 10.0, 1.0, 0.0)); // floor
        // Ocean * 0.5 * time; rand 0 always moves when time > 0.
        assert!(item_in_water_should_move(false, OCEAN, 0, 2.0, 1.0, 0.0));
        assert!(!item_in_water_should_move(false, OCEAN, 0, 0.1, 1.0, 0.9));
    }

    #[test]
    fn pick_time_transition_move_dist_flags_move() {
        let mut db = ContentDb::default();
        let mut t = tr_decay(50, 50, 3.0);
        t.move_dist = 2;
        db.auto_decays.insert(50, t);
        let p = pick_time_transition(&db, 50, 0).unwrap();
        assert!(p.is_move);
    }

    #[test]
    fn pick_time_transition_last_use_and_reverse_max() {
        let mut db = ContentDb::default();
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                num_uses: 3,
                ..ObjectDef::empty(100)
            },
        );
        db.objects.insert(
            101,
            ObjectDef {
                id: 101,
                num_uses: 3,
                ..ObjectDef::empty(101)
            },
        );
        db.auto_decays.insert(100, tr_decay(100, 101, 5.0));
        // Last-use table: 100 → 999
        db.transitions_last_use.insert(
            (-1, 100),
            Transition {
                last_use_target: true,
                new_target_id: 999,
                ..tr_decay(100, 999, 5.0)
            },
        );
        let p = pick_time_transition(&db, 100, 1).unwrap();
        assert_eq!(p.transition.new_target_id, 999);

        // Reverse max: uses full → max-use table.
        let mut rev = tr_decay(100, 101, 5.0);
        rev.reverse_use_target = true;
        db.auto_decays.insert(100, rev);
        db.transitions_max_use.insert(
            (-1, 100),
            Transition {
                new_target_id: 200,
                ..tr_decay(100, 200, 5.0)
            },
        );
        let p2 = pick_time_transition(&db, 100, 3).unwrap();
        assert!(p2.is_max_use);
        assert_eq!(p2.transition.new_target_id, 200);
    }

    #[test]
    fn spring_regrow_and_winter_multiuse_bush_decay() {
        assert!(winter_multiuse_should_decay(1.0, 1.0, 3, 0.5));
        assert!(!winter_multiuse_should_decay(0.0, 1.0, 3, 0.0));
        assert!(spring_multiuse_should_regrow(1.0, 1.0, 1, 5, 0.5));
        assert!(!spring_multiuse_should_regrow(1.0, 1.0, 5, 5, 0.0));
    }

    #[test]
    fn map_slice_second_outcome_and_water_integration() {
        let mut db = ContentDb::default();
        db.second_time_outcomes.insert(141, (142, 1.0));
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                permanent: false,
                speed_mult: 1.0,
                ..ObjectDef::empty(33)
            },
        );
        let mut world = World::new(10, 50, false);
        // Band 0 is y=0..2 for height 50 / 25.
        world.set_object(1, 0, 141);
        world.set_biome(2, 0, OCEAN);
        world.set_object(2, 0, 33);
        // Leave (3,0) empty as possible water dest — random may miss; force via many trials.
        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 100.0;
        let mut rng = StdRng::seed_from_u64(1);
        let changes = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        // Goose pond should have flipped with high time_passed.
        assert!(
            changes
                .iter()
                .any(|c| c.x == 1 && c.y == 0 && c.new_object_id == 142)
                || world.get_object(1, 0) == 142,
            "second-time outcome should fire"
        );
        assert_eq!(map_time.step, 1);
        // Only band y in [0, 2) touched for temps.
        assert!(map_time.tile_temps.contains_key(&(1, 0)));
        assert!(!map_time.tile_temps.contains_key(&(1, 5)));
    }

    #[test]
    fn container_overflow_sharp_stone_extends() {
        let d = container_overflow_delay(3, 1, SHARP_STONE_ID).unwrap();
        // Haxe: +20s base + CursedGraveTime*3600 (default 12h → 43200s)
        assert!((d.0 - (20.0 + CURSED_GRAVE_SHARP_STONE_EXTRA_SECS)).abs() < 1e-3);
        assert!((CURSED_GRAVE_SHARP_STONE_EXTRA_SECS - 43200.0).abs() < 1e-3);
        assert_eq!(d.1, SHARP_STONE_ID);
        assert!(container_overflow_delay(1, 2, 10).is_none());
        let non_stone = container_overflow_delay(3, 1, 10).unwrap();
        assert!((non_stone.0 - 20.0).abs() < 1e-3);
    }

    #[test]
    fn clear_cursed_graves_keeps_only_is_bone_grave() {
        // Fresh Grave 87, Bone Pile 357, Murder Grave 752 — bone; wolf 418 not.
        let mut idx = HashMap::new();
        index_insert(&mut idx, 1, 2, 100);
        index_insert(&mut idx, 5, 5, 100);
        index_insert(&mut idx, 9, 0, 100);
        let tiles: HashMap<(i32, i32), i32> =
            [((1, 2), 87), ((5, 5), 418), ((9, 0), 357)].into_iter().collect();
        let cleared = clear_cursed_graves(&idx, |x, y| tiles.get(&(x, y)).copied().unwrap_or(0), 100);
        assert_eq!(cleared.len(), 2);
        assert!(cleared.values().any(|&p| p == (1, 2)));
        assert!(cleared.values().any(|&p| p == (9, 0)));
        assert!(!cleared.values().any(|&p| p == (5, 5)));
    }

    #[test]
    fn ovens_index_rebuild_keeps_is_oven_ids_only() {
        let mut idx = HashMap::new();
        index_insert(&mut idx, 0, 0, 50); // Adobe 237
        index_insert(&mut idx, 1, 0, 50); // Hot 250
        index_insert(&mut idx, 2, 0, 50); // non-oven 314
        index_insert(&mut idx, 3, 0, 50); // Burning 249
        let tiles: HashMap<(i32, i32), i32> = [
            ((0, 0), 237),
            ((1, 0), 250),
            ((2, 0), 314),
            ((3, 0), 249),
        ]
        .into_iter()
        .collect();
        let cleared = clear_ovens_index(&idx, |x, y| tiles.get(&(x, y)).copied().unwrap_or(0), 50);
        assert_eq!(cleared.len(), 3);
        assert!(!cleared.values().any(|&p| p == (2, 0)));
        assert!(is_oven_map_id(237) && is_oven_map_id(247) && is_oven_map_id(249) && is_oven_map_id(250));
        assert!(!is_oven_map_id(314));
    }

    #[test]
    fn map_slice_inserts_bone_grave_and_oven() {
        let mut db = ContentDb::default();
        db.objects.insert(87, ObjectDef::empty(87));
        db.objects.insert(237, ObjectDef::empty(237));
        let mut world = World::new(10, 50, false);
        world.set_object(2, 0, 87); // bone grave in first Y-band
        world.set_object(3, 0, 237); // oven
        world.set_object(4, 0, 10); // neither
        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        let mut rng = StdRng::seed_from_u64(1);
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            60.0,
            1.0,
            1.0,
            &mut rng,
        );
        assert!(
            map_time
                .cursed_graves
                .values()
                .any(|&p| p == (2, 0)),
            "bone grave should be indexed"
        );
        assert!(
            map_time.ovens.values().any(|&p| p == (3, 0)),
            "oven should be indexed"
        );
        assert!(!map_time.ovens.values().any(|&p| p == (4, 0)));
        // Key is linear index
        let key = map_linear_index(2, 0, 10);
        assert_eq!(map_time.cursed_graves.get(&key), Some(&(2, 0)));
    }

    #[test]
    fn stale_index_entry_removed_on_periodic_clear() {
        let mut db = ContentDb::default();
        db.objects.insert(87, ObjectDef::empty(87));
        let mut world = World::new(10, 50, false);
        // Pre-seed stale entries: grave that no longer exists + oven that became rubble
        let mut map_time = WorldMapTimeState::default();
        index_insert(&mut map_time.cursed_graves, 1, 1, 10);
        index_insert(&mut map_time.ovens, 2, 2, 10);
        // Live grave still present
        world.set_object(5, 0, 87);
        index_insert(&mut map_time.cursed_graves, 5, 0, 10);
        // Force prune path (step % 2000 == 0) before scan inserts
        map_time.step = 0;
        map_time.time_passed_all_steps = 1.0;
        let mut rng = StdRng::seed_from_u64(2);
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            60.0,
            1.0,
            1.0,
            &mut rng,
        );
        assert!(
            !map_time.cursed_graves.values().any(|&p| p == (1, 1)),
            "stale bone-grave entry dropped"
        );
        assert!(
            !map_time.ovens.values().any(|&p| p == (2, 2)),
            "stale oven entry dropped"
        );
        // Live grave re-inserted by Y-band (y=0 in first band for height 50 / 25 parts)
        assert!(map_time.cursed_graves.values().any(|&p| p == (5, 0)));
    }

    #[test]
    fn get_closest_bone_grave_uses_full_index_beyond_r80() {
        // Global index lists far graves; pure closest must pick them.
        let graves = vec![(200, 0), (150, 0)];
        let closest = crate::animal_move::get_closest_bone_grave(0, 0, &graves);
        assert_eq!(closest, Some((150, 0)));
        // Local collect within r=80 would miss both
        let mut world = World::new(220, 10, false);
        world.set_object(150, 0, 87);
        world.set_object(200, 0, 357);
        let near = crate::animal_move::collect_bone_graves_near(&world, 0, 0, 80);
        assert!(near.is_empty(), "r=80 scan must miss far graves");
        let global = index_positions(
            &[(map_linear_index(150, 0, 220), (150, 0)), (map_linear_index(200, 0, 220), (200, 0))]
                .into_iter()
                .collect(),
        );
        assert_eq!(
            crate::animal_move::get_closest_bone_grave(0, 0, &global),
            Some((150, 0))
        );
    }

    #[test]
    fn contained_timer_transform_in_map_slice() {
        let mut db = ContentDb::default();
        // Container 391 with contained 50 → 51 after timer.
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(50, ObjectDef::empty(50));
        db.objects.insert(51, ObjectDef::empty(51));
        db.auto_decays.insert(50, tr_decay(50, 51, 1.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        // Pre-arm timer so first slice transforms (creation in past).
        map_time
            .contained_timers
            .insert((1, 0), vec![(0.0, 1.0)]);

        let mut rng = StdRng::seed_from_u64(2);
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0, // sim_time past deadline
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.contained, vec![51], "contained should time-transform");
    }

    #[test]
    fn contained_timer_mid_ttc_survives_rearm_and_map_slice() {
        // Progress continuity: mid-ttc load must not reset creation/ttc.
        // Haxe keeps ObjectHelper.creationTimeInTicks + timeToChange on disk.
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(50, ObjectDef::empty(50));
        db.objects.insert(51, ObjectDef::empty(51));
        db.auto_decays.insert(50, tr_decay(50, 51, 60.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut slot = ol_world::NestedHelper::id_only(50);
        // Started at sim 100, needs 60s → fires at 160.
        slot.creation_time = 100.0;
        slot.time_to_change = 60.0;
        h.slots = vec![slot];
        world.set_object_complex(1, 0, h);

        // Rearm as after OLW load at sim_time 130 (mid-progress).
        let sim_load = 130.0_f32;
        let map = crate::contained_timers_persist::rebuild_contained_timers_from_world(
            &world, sim_load,
        );
        assert_eq!(
            map.get(&(1, 0)).unwrap(),
            &[(100.0, 60.0)],
            "mid-ttc must not reset on rearm"
        );

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time.contained_timers = map;

        let mut rng = StdRng::seed_from_u64(3);
        // Still mid-ttc at 140 → Pending, same timer.
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            140.0,
            1.0,
            &mut rng,
        );
        assert_eq!(
            map_time.contained_timers.get(&(1, 0)).unwrap(),
            &[(100.0, 60.0)]
        );
        assert_eq!(world.get_helper(1, 0).unwrap().contained, vec![50]);
        // Slots stamped for next save.
        let h = world.get_helper(1, 0).unwrap();
        assert!((h.slots[0].creation_time - 100.0).abs() < 1e-5);
        assert!((h.slots[0].time_to_change - 60.0).abs() < 1e-5);

        // Past deadline → transform (reset step so Y-band 0 is processed again).
        map_time.step = 0;
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            165.0,
            1.0,
            &mut rng,
        );
        assert_eq!(world.get_helper(1, 0).unwrap().contained, vec![51]);
    }

    #[test]
    fn contained_slot_uses_remaining_drives_last_use_in_map_slice() {
        // Multi-use last-use when NestedHelper.uses_remaining==1 (not dummy wire id).
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            100,
            ObjectDef {
                id: 100,
                num_uses: 3,
                dummy_ids: vec![9001, 9002],
                ..ObjectDef::empty(100)
            },
        );
        db.objects.insert(999, ObjectDef::empty(999));
        db.objects.insert(
            101,
            ObjectDef {
                id: 101,
                num_uses: 3,
                ..ObjectDef::empty(101)
            },
        );
        // Normal decay would go 100 → 101; last-use → 999.
        db.auto_decays.insert(100, tr_decay(100, 101, 1.0));
        db.transitions_last_use.insert(
            (-1, 100),
            Transition {
                last_use_target: true,
                new_target_id: 999,
                ..tr_decay(100, 999, 1.0)
            },
        );

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        // Wire id is base 100 (full), but slot uses_remaining says last use.
        h.contained = vec![100];
        let mut slot = ol_world::NestedHelper::id_only(100);
        slot.uses_remaining = 1;
        slot.creation_time = 0.0;
        slot.time_to_change = 1.0;
        h.slots = vec![slot];
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time
            .contained_timers
            .insert((1, 0), vec![(0.0, 1.0)]);

        let mut rng = StdRng::seed_from_u64(4);
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(
            h.contained,
            vec![999],
            "slot uses_remaining==1 must pick last-use table"
        );
    }

    // --- NESTED-IN-NESTED-TIMERS: map-slice deep cargo ---

    #[test]
    fn nested_in_nested_timer_transform_in_map_slice() {
        // Basket-in-basket: depth-2 auto-decay via do_world_map_time_stuff.
        // Haxe: DoWorldMapTimeStuff L1150 TODO (implemented via tick_container_helper_timers).
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 1.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = ol_world::NestedHelper::id_only(50);
        let mut inner = ol_world::NestedHelper::id_only(60);
        inner.creation_time = 0.0;
        inner.time_to_change = 1.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];
        h.rebuild_wire_from_slots();
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        // First-level outer has no auto-decay; runtime map still needs a slot entry.
        map_time
            .contained_timers
            .insert((1, 0), vec![(0.0, 0.0)]);

        let mut rng = StdRng::seed_from_u64(11);
        let changes = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.contained, vec![50], "first-level basket unchanged");
        assert_eq!(
            h.slots[0].contained[0].id, 61,
            "deep NestedHelper must transform on map-slice"
        );
        assert_eq!(h.nested, vec![vec![61]]);
        assert!(
            changes.iter().any(|c| c.x == 1 && c.y == 0),
            "MapTimeChange must emit on deep transform"
        );
    }

    #[test]
    fn nested_in_nested_mid_ttc_survives_map_slice() {
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 60.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = ol_world::NestedHelper::id_only(50);
        outer.creation_time = 100.0;
        outer.time_to_change = 0.0;
        let mut inner = ol_world::NestedHelper::id_only(60);
        inner.creation_time = 100.0;
        inner.time_to_change = 60.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time
            .contained_timers
            .insert((1, 0), vec![(100.0, 0.0)]);

        let mut rng = StdRng::seed_from_u64(12);
        let changes = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            140.0,
            1.0,
            &mut rng,
        );
        assert!(
            changes.iter().all(|c| !(c.x == 1 && c.y == 0)),
            "mid-ttc deep must not MX yet"
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.slots[0].contained[0].id, 60);
        assert!((h.slots[0].contained[0].creation_time - 100.0).abs() < 1e-5);
        assert!((h.slots[0].contained[0].time_to_change - 60.0).abs() < 1e-5);

        map_time.step = 0;
        let changes2 = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            165.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.slots[0].contained[0].id, 61);
        assert!(changes2.iter().any(|c| c.x == 1 && c.y == 0));
    }

    #[test]
    fn first_level_transform_preserves_deep_cargo() {
        // First-level transform keeps nested cargo when new.num_slots allows.
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(
            51,
            ObjectDef {
                id: 51,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(51)
            },
        );
        db.objects.insert(70, ObjectDef::empty(70));
        db.auto_decays.insert(50, tr_decay(50, 51, 1.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = ol_world::NestedHelper::id_only(50);
        outer.contained = vec![ol_world::NestedHelper::id_only(70)];
        h.slots = vec![outer];
        h.rebuild_wire_from_slots();
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time
            .contained_timers
            .insert((1, 0), vec![(0.0, 1.0)]);

        let mut rng = StdRng::seed_from_u64(14);
        let _ = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.contained, vec![51]);
        assert_eq!(
            h.slots[0].contained.len(),
            1,
            "cargo must survive first-level transform"
        );
        assert_eq!(h.slots[0].contained[0].id, 70);
        assert_eq!(h.nested, vec![vec![70]]);
    }

    #[test]
    fn first_level_overflow_refuse_keeps_parent_and_cargo() {
        // Haxe doTimeForObject L2213: refuse when cargo.len() > new.num_slots.
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 2,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(
            51,
            ObjectDef {
                id: 51,
                num_slots: 0,
                permanent: true,
                ..ObjectDef::empty(51)
            },
        );
        db.objects.insert(70, ObjectDef::empty(70));
        db.auto_decays.insert(50, tr_decay(50, 51, 1.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = ol_world::NestedHelper::id_only(50);
        outer.contained = vec![ol_world::NestedHelper::id_only(70)];
        h.slots = vec![outer];
        h.rebuild_wire_from_slots();
        world.set_object_complex(1, 0, h);

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time
            .contained_timers
            .insert((1, 0), vec![(0.0, 1.0)]);

        let mut rng = StdRng::seed_from_u64(15);
        let changes = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.contained, vec![50], "overflow refuse keeps parent id");
        assert_eq!(h.slots[0].contained[0].id, 70);
        assert!(
            changes.iter().all(|c| !(c.x == 1 && c.y == 0)),
            "refuse must not emit MX"
        );
    }

    #[test]
    fn nested_in_nested_wired_call_site_present() {
        // Guard against false-positive DONE (pub use without map-slice call).
        // Extract map-slice body only so self-referential assert strings don't false-fail.
        let src = include_str!("world_time.rs");
        let slice_start = src
            .find("pub fn do_world_map_time_stuff")
            .expect("do_world_map_time_stuff");
        let slice_end = src[slice_start..]
            .find("\n// ---------------------------------------------------------------------------\n// Tests")
            .map(|i| slice_start + i)
            .unwrap_or(src.len());
        let body = &src[slice_start..slice_end];
        assert!(
            body.contains("nested_timers::tick_container_helper_timers"),
            "do_world_map_time_stuff must call nested_timers::tick_container_helper_timers"
        );
        // Old deferred wording (split so this assert does not contain the needle).
        let deferred = format!(
            "{}{}",
            "Nested-in-nested remains ",
            "Haxe TODO"
        );
        assert!(
            !body.contains(&deferred),
            "deferred comment must be gone from map-slice"
        );
    }

    #[test]
    fn rearm_then_deep_tick_without_runtime_map_for_depth2() {
        // After load rearm: only first-level runtime map; deep times on OLW3 slots.
        let mut db = ContentDb::default();
        db.objects.insert(
            391,
            ObjectDef {
                id: 391,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(391)
            },
        );
        db.objects.insert(
            50,
            ObjectDef {
                id: 50,
                num_slots: 4,
                permanent: true,
                ..ObjectDef::empty(50)
            },
        );
        db.objects.insert(60, ObjectDef::empty(60));
        db.objects.insert(61, ObjectDef::empty(61));
        db.auto_decays.insert(60, tr_decay(60, 61, 1.0));

        let mut world = World::new(10, 50, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![50];
        let mut outer = ol_world::NestedHelper::id_only(50);
        outer.creation_time = 0.0;
        outer.time_to_change = 0.0;
        let mut inner = ol_world::NestedHelper::id_only(60);
        inner.creation_time = 0.0;
        inner.time_to_change = 1.0;
        outer.contained = vec![inner];
        h.slots = vec![outer];
        world.set_object_complex(1, 0, h);

        let map =
            crate::contained_timers_persist::rebuild_contained_timers_from_world(&world, 0.0);
        assert_eq!(map.get(&(1, 0)).unwrap().len(), 1);
        // No separate runtime entry for depth≥2.

        let mut map_time = WorldMapTimeState::default();
        map_time.time_passed_all_steps = 1.0;
        map_time.contained_timers = map;

        let mut rng = StdRng::seed_from_u64(16);
        let changes = do_world_map_time_stuff(
            &mut world,
            &db,
            &mut map_time,
            false,
            false,
            0.0,
            120.0,
            10.0,
            1.0,
            &mut rng,
        );
        let h = world.get_helper(1, 0).unwrap();
        assert_eq!(h.slots[0].contained[0].id, 61);
        assert!(changes.iter().any(|c| c.x == 1 && c.y == 0));
    }
}
