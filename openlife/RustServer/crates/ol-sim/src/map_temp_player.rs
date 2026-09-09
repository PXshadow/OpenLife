//! MAP-TEMP-PLAYER / `vitals_tile_temps`
//!
//! Haxe: `TemperatureHandler.BalanceTemperatureArea` + ambient read in
//! `GlobalPlayerInstance.updateTemperature`.
//!
//! Live wire into vitals:
//! 1. Ensure player tile has a temperature in `WorldMapTimeState.tile_temps`
//! 2. Balance Chebyshev area radius 5 (`doLocalHeat=false`) around the player
//! 3. Sample ambient from tile_temps (water halves)
//! 4. Worn clothing rValue insulation / heat-protection matrix
//! 5. Color shift + biome-love boni + held-by + storedWater cool
//! 6. Integrate body `heat` toward ambient (Haxe impact-per-sec subset)

use ol_content::ContentDb;
use ol_world::{World, DESERT, OCEAN, PASSABLE_RIVER};

/// Haxe `BiomeTag.SNOW` / `JUNGLE` (not re-exported from ol_world root).
const BIOME_SNOW: u8 = 4;
const BIOME_JUNGLE: u8 = 6;

use crate::heat_ideal::{
    body_heat_step_ex, clamp_heat, IDEAL_HEAT, TEMPERATURE_IMPACT_PER_SEC,
    TEMPERATURE_IMPACT_PER_SEC_IF_GOOD, TEMPERATURE_IN_WATER_FACTOR,
};
use crate::world_time::{
    apply_season_temperature_factors_ex, balance_temperature_area_coords,
    balance_tile_temperature, biome_base_temperature, floor_insulation_from_content,
    initialize_tile_temperature_ex, local_heat_from_value, object_insulation_from_content,
    COLD_SEASON_TEMPERATURE_FACTOR, HOT_SEASON_TEMPERATURE_FACTOR, WorldMapTimeState,
};

/// Haxe `BalanceTemperatureArea(..., d=5, ...)` from `updateTemperature`.
pub const PLAYER_BALANCE_TEMP_RADIUS: i32 = 5;

/// Haxe `if (timePassed > 5) timePassed = 5`.
pub const PLAYER_TEMP_TIME_PASSED_CAP: f32 = 5.0;

/// Haxe `ServerSettings.TemperatureHeatObjectFactor`.
pub const TEMPERATURE_HEAT_OBJECT_FACTOR: f32 = 1.5;
/// Haxe `ServerSettings.TemperatureHeatObjFactor`.
pub const TEMPERATURE_HEAT_OBJ_FACTOR: f32 = 1.0;
/// Haxe `GetClosestHeatObject` default search (Chebyshev).
pub const CLOSEST_HEAT_SEARCH_DIST: i32 = 2;

/// Haxe `AiHelper.GetClosestHeatObject` + `updateTemperature` heat-object add.
///
/// `quadDistance = 10 + dx²+dy²`; contrib = `heatValue / (factor * quad) * objFactor`.
/// Half impact when already cold and contrib negative, or already hot and contrib positive.
// Haxe: GlobalPlayerInstance.updateTemperature L6356–6377
pub fn closest_heat_object_contribution(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    player_heat: f32,
) -> f32 {
    closest_heat_object_contribution_ex(
        world,
        content,
        px,
        py,
        player_heat,
        CLOSEST_HEAT_SEARCH_DIST,
        TEMPERATURE_HEAT_OBJECT_FACTOR,
        TEMPERATURE_HEAT_OBJ_FACTOR,
    )
}

/// Same as [`closest_heat_object_contribution`] with explicit search / factors.
pub fn closest_heat_object_contribution_ex(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    player_heat: f32,
    search: i32,
    heat_object_factor: f32,
    heat_obj_factor: f32,
) -> f32 {
    let r = search.max(0);
    let factor = if heat_object_factor.is_finite() && heat_object_factor > 0.0 {
        heat_object_factor
    } else {
        TEMPERATURE_HEAT_OBJECT_FACTOR
    };
    let obj_f = if heat_obj_factor.is_finite() {
        heat_obj_factor
    } else {
        TEMPERATURE_HEAT_OBJ_FACTOR
    };
    let mut best_quad = f32::MAX;
    let mut best_heat = 0.0_f32;
    let mut found = false;
    for dy in -r..=r {
        for dx in -r..=r {
            let id = world.get_object(px + dx, py + dy);
            if id == 0 {
                continue;
            }
            let Some(def) = content.get(id) else {
                continue;
            };
            if def.heat_value == 0.0 || !def.heat_value.is_finite() {
                continue;
            }
            let quad = (dx * dx + dy * dy) as f32;
            if found && quad >= best_quad {
                continue;
            }
            found = true;
            best_quad = quad;
            best_heat = def.heat_value;
        }
    }
    if !found {
        return 0.0;
    }
    let quad_distance = 10.0 + best_quad;
    let mut contrib = best_heat / (factor * quad_distance) * obj_f;
    if player_heat < IDEAL_HEAT && contrib < 0.0 {
        contrib -= contrib / 2.0;
    }
    if player_heat > IDEAL_HEAT && contrib > 0.0 {
        contrib -= contrib / 2.0;
    }
    contrib
}

/// Haxe held-object heat: `heatValue / 20 * TemperatureHeatObjFactor`.
// Haxe: GlobalPlayerInstance.updateTemperature L6478–6480
pub fn held_heat_contribution(held_heat_value: f32) -> f32 {
    held_heat_contribution_ex(held_heat_value, TEMPERATURE_HEAT_OBJ_FACTOR)
}

pub fn held_heat_contribution_ex(held_heat_value: f32, heat_obj_factor: f32) -> f32 {
    if held_heat_value == 0.0 || !held_heat_value.is_finite() {
        return 0.0;
    }
    let f = if heat_obj_factor.is_finite() {
        heat_obj_factor
    } else {
        TEMPERATURE_HEAT_OBJ_FACTOR
    };
    (held_heat_value / 20.0) * f
}

/// Read tile temperature without inserting; `None` if unset (`< 0` or missing).
#[inline]
pub fn get_tile_temperature(map_time: &WorldMapTimeState, x: i32, y: i32) -> Option<f32> {
    map_time
        .tile_temps
        .get(&(x, y))
        .copied()
        .filter(|t| *t >= 0.0)
}

/// Ensure sparse `tile_temps[(x,y)]` is initialized (Haxe `initializeTileTemperature` on miss).
///
/// Returns current tile temperature (≥ 0). Records original biome on first touch.
pub fn ensure_tile_temperature(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    x: i32,
    y: i32,
    season_impact_raw: f32,
) -> f32 {
    ensure_tile_temperature_ex(
        world,
        content,
        map_time,
        x,
        y,
        season_impact_raw,
        HOT_SEASON_TEMPERATURE_FACTOR,
        COLD_SEASON_TEMPERATURE_FACTOR,
    )
}

/// Same as [`ensure_tile_temperature`] with live Hot/Cold season factors.
pub fn ensure_tile_temperature_ex(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    x: i32,
    y: i32,
    season_impact_raw: f32,
    hot_factor: f32,
    cold_factor: f32,
) -> f32 {
    if let Some(&t) = map_time.tile_temps.get(&(x, y)) {
        if t >= 0.0 {
            return t;
        }
    }
    let biome = world.get_biome(x, y);
    let orig = *map_time.original_biomes.entry((x, y)).or_insert(biome);
    let obj_id = world.get_object(x, y);
    let obj_base = content.resolve_base_id(obj_id);
    let heat_value = content.get(obj_base).map(|d| d.heat_value).unwrap_or(0.0);
    let local_heat = local_heat_from_value(heat_value);
    let t = initialize_tile_temperature_ex(
        biome,
        orig,
        season_impact_raw,
        local_heat,
        hot_factor,
        cold_factor,
    );
    map_time.tile_temps.insert((x, y), t);
    t
}

/// Haxe `TemperatureHandler.BalanceTemperatureArea` on sparse `tile_temps`.
///
/// Player path: `doLocalHeat = false` (floor/wall early-outs, no local heat inject).
/// Only balances tiles that are already initialized (`temp >= 0`).
pub fn apply_balance_temperature_area(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    cx: i32,
    cy: i32,
    d: i32,
    delta_time: f32,
) {
    let coords = balance_temperature_area_coords(cx, cy, d);
    for (x, y) in coords {
        let current = map_time.tile_temps.get(&(x, y)).copied().unwrap_or(-1.0);
        if current < 0.0 {
            continue;
        }
        let floor = world.get_floor(x, y) as i32;
        let obj_id = world.get_object(x, y);
        let floor_ins = floor_insulation_from_content(content, floor);
        let obj_ins = object_insulation_from_content(content, obj_id);

        let mut neighbors = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                let nt = map_time.tile_temps.get(&(nx, ny)).copied().unwrap_or(-1.0);
                if nt < 0.0 {
                    continue;
                }
                let n_obj = world.get_object(nx, ny);
                let n_ins = object_insulation_from_content(content, n_obj);
                neighbors.push((dx, dy, nt, n_ins));
            }
        }

        if let Some(bal) = balance_tile_temperature(
            current, &neighbors, delta_time, 0.0, obj_ins, floor_ins, false, // player path
            0.0,
        ) {
            map_time.tile_temps.insert((x, y), bal.center);
            for (dx, dy, nt) in bal.neighbor_updates {
                map_time.tile_temps.insert((x + dx, y + dy), nt);
            }
        }
    }
}

/// True for Haxe water biomes that cool ambient (`PASSABLERIVER` / `OCEAN`).
#[inline]
pub fn is_water_biome_temp(biome: u8) -> bool {
    biome == PASSABLE_RIVER || biome == OCEAN
}

/// Haxe water ambient factor: `temperature *= 0.5` when standing in water.
#[inline]
pub fn apply_water_ambient(ambient: f32, biome: u8) -> f32 {
    if is_water_biome_temp(biome) {
        ambient * 0.5
    } else {
        ambient
    }
}

/// Player ambient after local balance (Haxe `updateTemperature` tile read).
///
/// 1. Caps `delta_time` at [`PLAYER_TEMP_TIME_PASSED_CAP`].
/// 2. Ensures center tile is initialized.
/// 3. Runs [`apply_balance_temperature_area`] with radius [`PLAYER_BALANCE_TEMP_RADIUS`].
/// 4. Applies water half-factor.
pub fn player_ambient_from_tile_temps(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    px: i32,
    py: i32,
    season_impact_raw: f32,
    delta_time: f32,
) -> f32 {
    player_ambient_from_tile_temps_ex(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        delta_time,
        HOT_SEASON_TEMPERATURE_FACTOR,
        COLD_SEASON_TEMPERATURE_FACTOR,
    )
}

/// Same as [`player_ambient_from_tile_temps`] with live Hot/Cold season factors.
pub fn player_ambient_from_tile_temps_ex(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    px: i32,
    py: i32,
    season_impact_raw: f32,
    delta_time: f32,
    hot_factor: f32,
    cold_factor: f32,
) -> f32 {
    let dt = delta_time.clamp(0.0, PLAYER_TEMP_TIME_PASSED_CAP);
    let _ = ensure_tile_temperature_ex(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        hot_factor,
        cold_factor,
    );
    apply_balance_temperature_area(
        world,
        content,
        map_time,
        px,
        py,
        PLAYER_BALANCE_TEMP_RADIUS,
        dt,
    );
    let t = ensure_tile_temperature_ex(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        hot_factor,
        cold_factor,
    );
    let biome = world.get_biome(px, py);
    apply_water_ambient(t, biome)
}

/// Haxe `ServerSettings.TemperatureClothingFactor`.
pub const TEMPERATURE_CLOTHING_FACTOR: f32 = 0.1;
/// Haxe `ServerSettings.TemperatureClothingInsulationFactor`.
pub const TEMPERATURE_CLOTHING_INSULATION_FACTOR: f32 = 5.0;
/// Haxe `ServerSettings.TemperatureNaturalHeatInsulation`.
pub const TEMPERATURE_NATURAL_HEAT_INSULATION: f32 = 0.5;

/// Live / compiled knobs for worn-clothing ambient + body-heat clothingFactor.
// Haxe: ServerSettings.TemperatureClothingFactor / InsulationFactor / NaturalHeatInsulation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClothingTempKnobs {
    pub clothing_factor: f32,
    pub clothing_insulation_factor: f32,
    pub natural_heat_insulation: f32,
}

impl Default for ClothingTempKnobs {
    fn default() -> Self {
        Self {
            clothing_factor: TEMPERATURE_CLOTHING_FACTOR,
            clothing_insulation_factor: TEMPERATURE_CLOTHING_INSULATION_FACTOR,
            natural_heat_insulation: TEMPERATURE_NATURAL_HEAT_INSULATION,
        }
    }
}

impl ClothingTempKnobs {
    fn sanitized(self) -> Self {
        Self {
            clothing_factor: if self.clothing_factor.is_finite() && self.clothing_factor >= 0.0 {
                self.clothing_factor
            } else {
                TEMPERATURE_CLOTHING_FACTOR
            },
            clothing_insulation_factor: if self.clothing_insulation_factor.is_finite()
                && self.clothing_insulation_factor >= 0.0
            {
                self.clothing_insulation_factor
            } else {
                TEMPERATURE_CLOTHING_INSULATION_FACTOR
            },
            natural_heat_insulation: if self.natural_heat_insulation.is_finite()
                && self.natural_heat_insulation >= 0.0
            {
                self.natural_heat_insulation
            } else {
                TEMPERATURE_NATURAL_HEAT_INSULATION
            },
        }
    }
}

/// Haxe `calculateClothingInsulation` — sum of worn `ObjectData.getInsulation`.
pub fn clothing_insulation_sum(content: &ContentDb, ids: &[i32]) -> f32 {
    ids.iter()
        .copied()
        .filter(|&id| id > 0)
        .map(|id| content.get(id).map(|d| d.get_insulation()).unwrap_or(0.0))
        .sum()
}

/// Haxe `calculateClothingHeatProtection` — sum of worn `ObjectData.getHeatProtection`.
pub fn clothing_heat_protection_sum(content: &ContentDb, ids: &[i32]) -> f32 {
    ids.iter()
        .copied()
        .filter(|&id| id > 0)
        .map(|id| {
            content
                .get(id)
                .map(|d| d.get_heat_protection())
                .unwrap_or(0.0)
        })
        .sum()
}

/// Haxe clothing ambient: skip in water; clamp heat-protection so it cannot overshoot 0.5.
// Haxe: GlobalPlayerInstance.updateTemperature L6444–6457
pub fn apply_clothing_to_ambient(
    ambient: f32,
    insulation: f32,
    heat_protection: f32,
    in_water: bool,
    clothing_factor: f32,
) -> f32 {
    if in_water {
        return ambient;
    }
    let f = if clothing_factor.is_finite() && clothing_factor >= 0.0 {
        clothing_factor
    } else {
        TEMPERATURE_CLOTHING_FACTOR
    };
    let mut temperature = ambient + insulation * 0.5 * f;
    if temperature > IDEAL_HEAT {
        temperature -= heat_protection * 0.5 * f;
        if temperature < IDEAL_HEAT {
            temperature = IDEAL_HEAT;
        }
    }
    temperature
}

/// Haxe clothingFactor: insulation when ambient < 0.5, else heat-protection + natural.
// Haxe: GlobalPlayerInstance.updateTemperature L6463–6473
pub fn clothing_body_factor(
    ambient_after_clothing: f32,
    insulation: f32,
    heat_protection: f32,
    clothing_insulation_factor: f32,
    natural_heat_insulation: f32,
) -> f32 {
    let k = if clothing_insulation_factor.is_finite() && clothing_insulation_factor >= 0.0 {
        clothing_insulation_factor
    } else {
        TEMPERATURE_CLOTHING_INSULATION_FACTOR
    };
    let natural = if natural_heat_insulation.is_finite() && natural_heat_insulation >= 0.0 {
        natural_heat_insulation
    } else {
        TEMPERATURE_NATURAL_HEAT_INSULATION
    };
    let insulation_factor = 1.0 / (1.0 + insulation.max(0.0) * k);
    let extra_natural = (natural - insulation).max(0.0);
    let heat_protection_factor = 1.0 / (1.0 + (heat_protection + extra_natural).max(0.0) * k);
    if ambient_after_clothing < IDEAL_HEAT {
        insulation_factor
    } else {
        heat_protection_factor
    }
}

/// Haxe `PersonColor` ids (`ObjectData.person`).
pub const PERSON_COLOR_BLACK: i32 = 1;
pub const PERSON_COLOR_BROWN: i32 = 3;
pub const PERSON_COLOR_WHITE: i32 = 4;
pub const PERSON_COLOR_GINGER: i32 = 6;

/// Haxe `ServerSettings.TemperatureShiftForBlack`.
pub const TEMPERATURE_SHIFT_FOR_BLACK: f32 = 0.1;
/// Haxe `ServerSettings.TemperatureShiftForBrown`.
pub const TEMPERATURE_SHIFT_FOR_BROWN: f32 = 0.05;
/// Haxe `ServerSettings.TemperatureShiftForWhite`.
pub const TEMPERATURE_SHIFT_FOR_WHITE: f32 = -0.05;
/// Haxe `ServerSettings.TemperatureShiftForGinger`.
pub const TEMPERATURE_SHIFT_FOR_GINGER: f32 = -0.1;
/// Haxe `ServerSettings.TemperatureLovedBiomeFactor`.
pub const TEMPERATURE_LOVED_BIOME_FACTOR: f32 = 1.0;
/// Haxe `ServerSettings.TemperatureMaxLovedBiomeImpact`.
pub const TEMPERATURE_MAX_LOVED_BIOME_IMPACT: f32 = 0.1;

/// Haxe `getTemperatureShiftForColor` — subtracted from tile ambient.
// Haxe: GlobalPlayerInstance.getTemperatureShiftForColor
pub fn temperature_shift_for_color(person_color: i32) -> f32 {
    match person_color {
        PERSON_COLOR_BLACK => TEMPERATURE_SHIFT_FOR_BLACK,
        PERSON_COLOR_BROWN => TEMPERATURE_SHIFT_FOR_BROWN,
        PERSON_COLOR_WHITE => TEMPERATURE_SHIFT_FOR_WHITE,
        PERSON_COLOR_GINGER => TEMPERATURE_SHIFT_FOR_GINGER,
        _ => 0.0,
    }
}

/// Haxe: `temperatureNew -= colorTemperatureShift`.
pub fn apply_color_temperature_shift(ambient: f32, person_color: i32) -> f32 {
    ambient - temperature_shift_for_color(person_color)
}

/// Haxe loved-biome temperature boni (positive love only; cap 0.1).
// Haxe: GlobalPlayerInstance.updateTemperature L6482–6500
pub fn apply_biome_love_temperature(
    ambient: f32,
    player_heat: f32,
    biome_love_factor: f32,
    loved_biome_factor: f32,
    max_impact: f32,
) -> f32 {
    let factor = if loved_biome_factor.is_finite() {
        loved_biome_factor
    } else {
        TEMPERATURE_LOVED_BIOME_FACTOR
    };
    let cap = if max_impact.is_finite() && max_impact >= 0.0 {
        max_impact
    } else {
        TEMPERATURE_MAX_LOVED_BIOME_IMPACT
    };
    let love = if biome_love_factor.is_finite() {
        biome_love_factor
    } else {
        0.0
    };
    let mut boni = love / 10.0 * factor;
    if boni > cap {
        boni = cap;
    }
    if boni <= 0.0 {
        return ambient;
    }
    let mut t = ambient;
    if player_heat < IDEAL_HEAT && t < IDEAL_HEAT {
        t += boni;
    }
    if player_heat > IDEAL_HEAT && t > IDEAL_HEAT {
        t -= boni;
    }
    t
}

/// Haxe storedWater evaporative cool when `heat > 0.6`.
// Haxe: GlobalPlayerInstance.updateTemperature L6517–6520
pub fn apply_stored_water_cool(heat: f32, stored_water: f32, dt: f32) -> (f32, f32) {
    let heat = if heat.is_finite() { heat } else { IDEAL_HEAT };
    let stored = if stored_water.is_finite() && stored_water > 0.0 {
        stored_water
    } else {
        0.0
    };
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    if heat > 0.6 && stored > 0.0 && dt > 0.0 {
        let reduction = dt * stored * (heat - 0.6) * 0.05;
        (clamp_heat(heat - reduction), (stored - reduction).max(0.0))
    } else {
        (clamp_heat(heat), stored)
    }
}

/// Extra `updateTemperature` inputs (color / biome-love / held-by / places).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempAmbientExtras {
    pub person_color: i32,
    pub biome_love_factor: f32,
    pub held_by_heat: Option<f32>,
    pub warm_place: Option<(i32, i32)>,
    pub cold_place: Option<(i32, i32)>,
}

impl Default for TempAmbientExtras {
    fn default() -> Self {
        Self {
            person_color: 0,
            biome_love_factor: 0.0,
            held_by_heat: None,
            warm_place: None,
            cold_place: None,
        }
    }
}

/// Haxe `temperature > 0.55` seeds / compares warmPlace.
pub const WARM_PLACE_TEMP: f32 = 0.55;
/// Haxe `temperature < 0.45` seeds / compares coldPlace.
pub const COLD_PLACE_TEMP: f32 = 0.45;
/// Haxe fitness denominator bias (`quadDist + 25` / new `/ 25`).
pub const PLACE_FITNESS_BIAS: f32 = 25.0;

/// Haxe remembered warm/cold tiles (`Player.warmPlace` / `coldPlace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TempPlaceState {
    pub warm: Option<(i32, i32)>,
    pub cold: Option<(i32, i32)>,
}

/// Haxe `BiomeTag.DESERT` / `JUNGLE` — warmPlace replacement biomes.
#[inline]
pub fn is_warm_place_biome(biome: u8) -> bool {
    biome == DESERT || biome == BIOME_JUNGLE
}

/// Haxe `PASSABLERIVER` / `SNOW` — coldPlace replacement biomes.
#[inline]
pub fn is_cold_place_biome(biome: u8) -> bool {
    biome == BIOME_SNOW || biome == PASSABLE_RIVER
}

/// Torus-aware squared distance (Haxe `AiHelper.CalculateDistance`).
pub fn torus_quad_distance(
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f32 {
    let mut dx = bx - ax;
    let mut dy = by - ay;
    if wrap && map_w > 0 {
        let half = map_w / 2;
        if dx > half {
            dx -= map_w;
        } else if dx < -half {
            dx += map_w;
        }
    }
    if wrap && map_h > 0 {
        let half = map_h / 2;
        if dy > half {
            dy -= map_h;
        } else if dy < -half {
            dy += map_h;
        }
    }
    (dx * dx + dy * dy) as f32
}

/// Haxe `calculateTemperature` subset: biome base + season factors − color shift.
// Haxe: GlobalPlayerInstance.calculateTemperature (no floor-random / neighbor scan)
pub fn calculate_place_temperature(
    biome: u8,
    person_color: i32,
    season_impact_raw: f32,
    hot_factor: f32,
    cold_factor: f32,
) -> f32 {
    biome_base_temperature(biome) - temperature_shift_for_color(person_color)
        + apply_season_temperature_factors_ex(season_impact_raw, hot_factor, cold_factor)
}

/// Haxe warmPlace seed / jungle-desert fitness replace.
// Haxe: GlobalPlayerInstance.updateTemperature L6383–6406
pub fn consider_warm_place(
    current: Option<(i32, i32)>,
    px: i32,
    py: i32,
    biome: u8,
    felt_temp: f32,
    old_place_temp: f32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> Option<(i32, i32)> {
    if felt_temp <= WARM_PLACE_TEMP {
        return current;
    }
    let Some((ox, oy)) = current else {
        return Some((px, py));
    };
    if !is_warm_place_biome(biome) {
        return current;
    }
    let quad = torus_quad_distance(px, py, ox, oy, map_w, map_h, wrap);
    let old_fit = (old_place_temp - IDEAL_HEAT) / (quad + PLACE_FITNESS_BIAS);
    let new_fit = (felt_temp - IDEAL_HEAT) / PLACE_FITNESS_BIAS;
    if new_fit >= old_fit {
        Some((px, py))
    } else {
        current
    }
}

/// Haxe coldPlace seed / snow-water fitness replace (water ×2).
// Haxe: GlobalPlayerInstance.updateTemperature L6408–6437
pub fn consider_cold_place(
    current: Option<(i32, i32)>,
    px: i32,
    py: i32,
    biome: u8,
    felt_temp: f32,
    old_place_temp: f32,
    old_place_biome: u8,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> Option<(i32, i32)> {
    if felt_temp >= COLD_PLACE_TEMP {
        return current;
    }
    let Some((ox, oy)) = current else {
        return Some((px, py));
    };
    if !is_cold_place_biome(biome) {
        return current;
    }
    let quad = torus_quad_distance(px, py, ox, oy, map_w, map_h, wrap);
    let mut old_fit = (IDEAL_HEAT - old_place_temp) / (quad + PLACE_FITNESS_BIAS);
    let mut new_fit = (IDEAL_HEAT - felt_temp) / PLACE_FITNESS_BIAS;
    if biome == PASSABLE_RIVER {
        new_fit *= 2.0;
    }
    if old_place_biome == PASSABLE_RIVER {
        old_fit *= 2.0;
    }
    if new_fit >= old_fit {
        Some((px, py))
    } else {
        current
    }
}

/// Superbad-heat goto target (Haxe handleTemperature warmPlace / coldPlace fallback).
pub fn plan_temp_place_goto(
    heat: f32,
    warm: Option<(i32, i32)>,
    cold: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    if heat > 0.9 {
        cold
    } else if heat < 0.1 {
        warm
    } else {
        None
    }
}

/// 3-slot presence fallback without content rValue. Live path uses worn ids.
///
/// `insulationFactor` with h=0.4 / t=0.4 / s=0.2 (cold ambient).
pub fn clothing_factor_from_slots(hat: i32, chest: i32, shoes: i32) -> f32 {
    let mut ins = 0.0_f32;
    if hat > 0 {
        ins += 0.4;
    }
    if chest > 0 {
        ins += 0.4;
    }
    if shoes > 0 {
        ins += 0.2;
    }
    clothing_body_factor(
        0.0,
        ins,
        0.0,
        TEMPERATURE_CLOTHING_INSULATION_FACTOR,
        TEMPERATURE_NATURAL_HEAT_INSULATION,
    )
}

/// One player temperature tick: ambient from tile_temps + clothing matrix + body heat.
///
/// Returns `(new_heat, ambient_last_temperature)`.
pub fn update_player_temperature(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    px: i32,
    py: i32,
    season_impact_raw: f32,
    delta_time: f32,
    current_heat: f32,
    clothing_ids: &[i32],
    held_id: i32,
) -> (f32, f32) {
    let (heat, amb, _) = update_player_temperature_ex(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        delta_time,
        current_heat,
        clothing_ids,
        TEMPERATURE_IMPACT_PER_SEC,
        TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
        TEMPERATURE_IN_WATER_FACTOR,
        HOT_SEASON_TEMPERATURE_FACTOR,
        COLD_SEASON_TEMPERATURE_FACTOR,
        held_id,
        ClothingTempKnobs::default(),
        TempAmbientExtras::default(),
    );
    (heat, amb)
}

/// Live-knob player temperature tick.
// Haxe: GlobalPlayerInstance.updateTemperature
// SETTINGS-LONG-TAIL
pub fn update_player_temperature_ex(
    world: &World,
    content: &ContentDb,
    map_time: &mut WorldMapTimeState,
    px: i32,
    py: i32,
    season_impact_raw: f32,
    delta_time: f32,
    current_heat: f32,
    clothing_ids: &[i32],
    impact_per_sec: f32,
    impact_per_sec_if_good: f32,
    in_water_factor: f32,
    hot_factor: f32,
    cold_factor: f32,
    held_id: i32,
    clothing_knobs: ClothingTempKnobs,
    extras: TempAmbientExtras,
) -> (f32, f32, TempPlaceState) {
    let mut ambient = player_ambient_from_tile_temps_ex(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        delta_time,
        hot_factor,
        cold_factor,
    );
    ambient = apply_color_temperature_shift(ambient, extras.person_color);
    ambient += closest_heat_object_contribution(world, content, px, py, current_heat);
    let biome = world.get_biome(px, py);
    let places = {
        let old_warm_temp = extras.warm_place.map(|(x, y)| {
            calculate_place_temperature(
                world.get_biome(x, y),
                extras.person_color,
                season_impact_raw,
                hot_factor,
                cold_factor,
            )
        });
        let old_cold = extras.cold_place.map(|(x, y)| {
            (
                calculate_place_temperature(
                    world.get_biome(x, y),
                    extras.person_color,
                    season_impact_raw,
                    hot_factor,
                    cold_factor,
                ),
                world.get_biome(x, y),
            )
        });
        TempPlaceState {
            warm: consider_warm_place(
                extras.warm_place,
                px,
                py,
                biome,
                ambient,
                old_warm_temp.unwrap_or(0.0),
                world.width_tiles,
                world.height_tiles,
                world.wrap,
            ),
            cold: consider_cold_place(
                extras.cold_place,
                px,
                py,
                biome,
                ambient,
                old_cold.map(|c| c.0).unwrap_or(0.0),
                old_cold.map(|c| c.1).unwrap_or(0),
                world.width_tiles,
                world.height_tiles,
                world.wrap,
            ),
        }
    };
    let in_water = is_water_biome_temp(biome);
    let knobs = clothing_knobs.sanitized();
    let insulation = clothing_insulation_sum(content, clothing_ids);
    let heat_prot = clothing_heat_protection_sum(content, clothing_ids);
    ambient = apply_clothing_to_ambient(
        ambient,
        insulation,
        heat_prot,
        in_water,
        knobs.clothing_factor,
    );
    let clothing_factor = clothing_body_factor(
        ambient,
        insulation,
        heat_prot,
        knobs.clothing_insulation_factor,
        knobs.natural_heat_insulation,
    );
    let held_heat = content.get(held_id).map(|d| d.heat_value).unwrap_or(0.0);
    ambient += held_heat_contribution(held_heat);
    ambient = apply_biome_love_temperature(
        ambient,
        current_heat,
        extras.biome_love_factor,
        TEMPERATURE_LOVED_BIOME_FACTOR,
        TEMPERATURE_MAX_LOVED_BIOME_IMPACT,
    );
    if let Some(holder) = extras.held_by_heat {
        if holder.is_finite() {
            ambient = holder;
        }
    }
    let heat = body_heat_step_ex(
        current_heat,
        ambient,
        delta_time.clamp(0.0, PLAYER_TEMP_TIME_PASSED_CAP),
        clothing_factor,
        in_water,
        impact_per_sec,
        impact_per_sec_if_good,
        in_water_factor,
    );
    (heat, ambient, places)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::ContentDb;
    use ol_world::World;

    #[test]
    fn water_halves_ambient() {
        assert!((apply_water_ambient(0.8, PASSABLE_RIVER) - 0.4).abs() < 1e-5);
        assert!((apply_water_ambient(0.8, 0) - 0.8).abs() < 1e-5);
    }

    #[test]
    fn clothing_factor_slows_with_gear() {
        let bare = clothing_factor_from_slots(0, 0, 0);
        let geared = clothing_factor_from_slots(1, 2, 3);
        assert!((bare - 1.0).abs() < 1e-5);
        assert!(geared < bare);
        assert!(geared > 0.0);
    }

    #[test]
    fn ensure_tile_temperature_inits_and_caches() {
        let mut world = World::new(32, 32, false);
        world.set_biome(5, 5, 5); // desert
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        assert!(get_tile_temperature(&map_time, 5, 5).is_none());
        let t1 = ensure_tile_temperature(&world, &content, &mut map_time, 5, 5, 0.0);
        assert!(t1 >= 0.0 && t1 <= 2.0, "t1={t1}");
        assert!((t1 - 1.0).abs() < 1e-3, "desert base ~1.0 got {t1}");
        let t2 = ensure_tile_temperature(&world, &content, &mut map_time, 5, 5, 0.0);
        assert!((t1 - t2).abs() < 1e-6);
        assert_eq!(get_tile_temperature(&map_time, 5, 5), Some(t1));
    }

    #[test]
    fn ensure_tile_temperature_ex_live_cold_factor() {
        let mut world = World::new(32, 32, false);
        world.set_biome(5, 5, 5);
        let content = ContentDb::default();
        let mut a = WorldMapTimeState::default();
        let mut b = WorldMapTimeState::default();
        let compiled = ensure_tile_temperature(&world, &content, &mut a, 5, 5, -0.4);
        let live = ensure_tile_temperature_ex(
            &world,
            &content,
            &mut b,
            5,
            5,
            -0.4,
            HOT_SEASON_TEMPERATURE_FACTOR,
            0.5,
        );
        // Compiled cold 0.75: season = -0.3; live 0.5: season = -0.2 → live warmer.
        assert!(live > compiled, "live={live} compiled={compiled}");
        assert!((live - compiled - 0.1).abs() < 1e-3, "live={live} compiled={compiled}");
    }

    #[test]
    fn apply_balance_temperature_area_player_path_diffuses() {
        let mut world = World::new(32, 32, false);
        world.set_floor(10, 10, 1);
        world.set_floor(11, 10, 1);
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        map_time.tile_temps.insert((10, 10), 1.0);
        map_time.tile_temps.insert((11, 10), 0.0);
        apply_balance_temperature_area(&world, &content, &mut map_time, 10, 10, 1, 1.0);
        let c = map_time.tile_temps.get(&(10, 10)).copied().unwrap();
        let n = map_time.tile_temps.get(&(11, 10)).copied().unwrap();
        assert!(c < 1.0, "center cools via balance: {c}");
        assert!(n > 0.0, "neighbor warms: {n}");
    }

    #[test]
    fn player_ambient_from_tile_temps_seeds_center() {
        let mut world = World::new(32, 32, false);
        world.set_biome(3, 3, 0);
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        let amb = player_ambient_from_tile_temps(&world, &content, &mut map_time, 3, 3, 0.0, 1.0);
        assert!(amb >= 0.0, "amb={amb}");
        assert!(map_time.tile_temps.contains_key(&(3, 3)));
    }

    #[test]
    fn update_player_temperature_moves_heat_toward_hot_tile() {
        let mut world = World::new(32, 32, false);
        world.set_biome(2, 2, 5); // desert
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        let (heat, amb) = update_player_temperature(
            &world,
            &content,
            &mut map_time,
            2,
            2,
            0.0,
            5.0,
            IDEAL_HEAT,
            &[],
            0,
        );
        assert!(amb > 0.5, "desert ambient hot: {amb}");
        assert!(heat > IDEAL_HEAT, "body warms toward desert: heat={heat}");
        assert!((clamp_heat(heat) - heat).abs() < 1e-6);
    }

    #[test]
    fn balance_radius_constant() {
        assert_eq!(PLAYER_BALANCE_TEMP_RADIUS, 5);
        assert!((PLAYER_TEMP_TIME_PASSED_CAP - 5.0).abs() < 1e-6);
    }

    #[test]
    fn closest_heat_object_adds_fire_ambient() {
        let mut world = World::new(16, 16, false);
        let mut content = ContentDb::default();
        let mut fire = ol_content::ObjectDef::default();
        fire.id = 82;
        fire.name = "Fire".into();
        fire.heat_value = 8.0;
        content.objects.insert(82, fire);
        world.set_object(1, 0, 82);
        let contrib = closest_heat_object_contribution(&world, &content, 0, 0, 0.5);
        // 8 / (1.5 * (10+1)) * 1 ≈ 0.485
        assert!(contrib > 0.3, "contrib={contrib}");
        let held = held_heat_contribution(8.0);
        assert!((held - 0.4).abs() < 1e-6);
    }

    #[test]
    fn closest_heat_half_impact_when_already_hot() {
        let mut world = World::new(16, 16, false);
        let mut content = ContentDb::default();
        let mut fire = ol_content::ObjectDef::default();
        fire.id = 82;
        fire.heat_value = 8.0;
        content.objects.insert(82, fire);
        world.set_object(0, 0, 82);
        let cold = closest_heat_object_contribution(&world, &content, 0, 0, 0.2);
        let hot = closest_heat_object_contribution(&world, &content, 0, 0, 0.8);
        assert!(hot < cold, "hot={hot} cold={cold}");
        assert!((hot - cold / 2.0).abs() < 1e-5);
    }

    #[test]
    fn clothing_ambient_warms_and_skips_water() {
        // Hat rValue 1 → insulation 0.4; +0.4*0.5*0.1 = +0.02
        let dry = apply_clothing_to_ambient(0.0, 0.4, 0.0, false, TEMPERATURE_CLOTHING_FACTOR);
        assert!((dry - 0.02).abs() < 1e-5, "dry={dry}");
        let wet = apply_clothing_to_ambient(0.0, 0.4, 0.0, true, TEMPERATURE_CLOTHING_FACTOR);
        assert!((wet - 0.0).abs() < 1e-6);
        // Heat protection cannot pull below 0.5.
        let clamped = apply_clothing_to_ambient(0.52, 0.0, 2.0, false, TEMPERATURE_CLOTHING_FACTOR);
        assert!((clamped - IDEAL_HEAT).abs() < 1e-5, "clamped={clamped}");
    }

    #[test]
    fn clothing_body_factor_insulation_vs_heat_protection() {
        // Cold: 0.4 insulation → 1/(1+0.4*5) = 1/3
        let cold = clothing_body_factor(0.2, 0.4, 0.0, 5.0, 0.5);
        assert!((cold - 1.0 / 3.0).abs() < 1e-5, "cold={cold}");
        // Hot bare: natural 0.5 → 1/(1+0.5*5) = 1/3.5
        let hot_bare = clothing_body_factor(0.8, 0.0, 0.0, 5.0, 0.5);
        assert!((hot_bare - 1.0 / 3.5).abs() < 1e-5, "hot_bare={hot_bare}");
        let bare_cold = clothing_body_factor(0.2, 0.0, 0.0, 5.0, 0.5);
        assert!((bare_cold - 1.0).abs() < 1e-5);
    }

    #[test]
    fn clothing_sums_use_rvalue_matrix_and_backpack_no_heat_protection() {
        let mut content = ContentDb::default();
        let mut hat = ol_content::ObjectDef::empty(586);
        hat.clothing = "h".into();
        hat.r_value = 1.0;
        content.objects.insert(586, hat);
        let mut pack = ol_content::ObjectDef::empty(198);
        pack.clothing = "p".into();
        pack.r_value = 0.5;
        content.objects.insert(198, pack);
        let ids = [586, 0, 0, 0, 0, 198];
        assert!((clothing_insulation_sum(&content, &ids) - 0.6).abs() < 1e-5);
        assert!((clothing_heat_protection_sum(&content, &ids) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn wool_hat_slows_snow_cooling() {
        let mut world = World::new(32, 32, false);
        world.set_biome(4, 4, 4); // snow
        let mut content = ContentDb::default();
        let mut hat = ol_content::ObjectDef::empty(586);
        hat.clothing = "h".into();
        hat.r_value = 1.0;
        content.objects.insert(586, hat);
        let mut bare_map = WorldMapTimeState::default();
        let mut clad_map = WorldMapTimeState::default();
        let (bare, _) = update_player_temperature(
            &world,
            &content,
            &mut bare_map,
            4,
            4,
            0.0,
            5.0,
            IDEAL_HEAT,
            &[],
            0,
        );
        let (clad, amb) = update_player_temperature(
            &world,
            &content,
            &mut clad_map,
            4,
            4,
            0.0,
            5.0,
            IDEAL_HEAT,
            &[586],
            0,
        );
        assert!(amb > 0.0, "hat warms snow ambient: {amb}");
        assert!(clad > bare, "clad={clad} bare={bare}");
        assert!(bare < IDEAL_HEAT);
    }

    #[test]
    fn temperature_shift_for_person_colors() {
        assert!((temperature_shift_for_color(PERSON_COLOR_BLACK) - 0.1).abs() < 1e-6);
        assert!((temperature_shift_for_color(PERSON_COLOR_BROWN) - 0.05).abs() < 1e-6);
        assert!((temperature_shift_for_color(PERSON_COLOR_WHITE) + 0.05).abs() < 1e-6);
        assert!((temperature_shift_for_color(PERSON_COLOR_GINGER) + 0.1).abs() < 1e-6);
        assert_eq!(temperature_shift_for_color(0), 0.0);
        let shifted = apply_color_temperature_shift(1.0, PERSON_COLOR_BLACK);
        assert!((shifted - 0.9).abs() < 1e-6);
    }

    #[test]
    fn biome_love_warms_when_cold_and_ignores_hate() {
        let warmed = apply_biome_love_temperature(0.2, 0.2, 1.0, 1.0, 0.1);
        assert!((warmed - 0.3).abs() < 1e-5, "warmed={warmed}");
        let capped = apply_biome_love_temperature(0.2, 0.2, 5.0, 1.0, 0.1);
        assert!((capped - 0.3).abs() < 1e-5, "capped={capped}");
        let hate = apply_biome_love_temperature(0.2, 0.2, -1.0, 1.0, 0.1);
        assert!((hate - 0.2).abs() < 1e-6);
        let cool_hot = apply_biome_love_temperature(0.8, 0.8, 1.0, 1.0, 0.1);
        assert!((cool_hot - 0.7).abs() < 1e-5);
        let mixed = apply_biome_love_temperature(0.8, 0.2, 1.0, 1.0, 0.1);
        assert!((mixed - 0.8).abs() < 1e-6);
    }

    #[test]
    fn stored_water_cools_hot_and_drains_reserve() {
        let (h, w) = apply_stored_water_cool(0.8, 1.0, 1.0);
        // reduction = 1 * 1 * 0.2 * 0.05 = 0.01
        assert!((h - 0.79).abs() < 1e-5, "h={h}");
        assert!((w - 0.99).abs() < 1e-5, "w={w}");
        let (h2, w2) = apply_stored_water_cool(0.5, 1.0, 1.0);
        assert!((h2 - 0.5).abs() < 1e-6);
        assert!((w2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn held_by_overrides_ambient_to_holder_heat() {
        let mut world = World::new(16, 16, false);
        world.set_biome(2, 2, 5); // desert
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        let extras = TempAmbientExtras {
            person_color: 0,
            biome_love_factor: 0.0,
            held_by_heat: Some(0.5),
            warm_place: None,
            cold_place: None,
        };
        let (heat, amb, _) = update_player_temperature_ex(
            &world,
            &content,
            &mut map_time,
            2,
            2,
            0.0,
            5.0,
            0.5,
            &[],
            TEMPERATURE_IMPACT_PER_SEC,
            TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            TEMPERATURE_IN_WATER_FACTOR,
            HOT_SEASON_TEMPERATURE_FACTOR,
            COLD_SEASON_TEMPERATURE_FACTOR,
            0,
            ClothingTempKnobs::default(),
            extras,
        );
        assert!((amb - 0.5).abs() < 1e-5, "amb={amb}");
        assert!((heat - 0.5).abs() < 1e-4, "heat={heat}");
    }

    #[test]
    fn ginger_color_shift_warms_snow_ambient_vs_black() {
        let mut world = World::new(16, 16, false);
        world.set_biome(3, 3, 4); // snow
        let content = ContentDb::default();
        let mut black_map = WorldMapTimeState::default();
        let mut ginger_map = WorldMapTimeState::default();
        let black = TempAmbientExtras {
            person_color: PERSON_COLOR_BLACK,
            ..TempAmbientExtras::default()
        };
        let ginger = TempAmbientExtras {
            person_color: PERSON_COLOR_GINGER,
            ..TempAmbientExtras::default()
        };
        let (_, amb_b, _) = update_player_temperature_ex(
            &world,
            &content,
            &mut black_map,
            3,
            3,
            0.0,
            1.0,
            0.5,
            &[],
            TEMPERATURE_IMPACT_PER_SEC,
            TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            TEMPERATURE_IN_WATER_FACTOR,
            HOT_SEASON_TEMPERATURE_FACTOR,
            COLD_SEASON_TEMPERATURE_FACTOR,
            0,
            ClothingTempKnobs::default(),
            black,
        );
        let (_, amb_g, _) = update_player_temperature_ex(
            &world,
            &content,
            &mut ginger_map,
            3,
            3,
            0.0,
            1.0,
            0.5,
            &[],
            TEMPERATURE_IMPACT_PER_SEC,
            TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            TEMPERATURE_IN_WATER_FACTOR,
            HOT_SEASON_TEMPERATURE_FACTOR,
            COLD_SEASON_TEMPERATURE_FACTOR,
            0,
            ClothingTempKnobs::default(),
            ginger,
        );
        // Ginger shift -0.1 → subtract negative = warmer ambient than Black +0.1.
        assert!(amb_g > amb_b, "ginger={amb_g} black={amb_b}");
        assert!((amb_g - amb_b - 0.2).abs() < 1e-3, "g={amb_g} b={amb_b}");
    }

    #[test]
    fn warm_place_seeds_and_prefers_closer_desert() {
        // First hot tile seeds anywhere.
        let seeded = consider_warm_place(None, 3, 4, DESERT, 0.7, 0.0, 32, 32, false);
        assert_eq!(seeded, Some((3, 4)));
        // Grass does not replace.
        let keep = consider_warm_place(Some((0, 0)), 5, 5, 0, 0.8, 1.0, 32, 32, false);
        assert_eq!(keep, Some((0, 0)));
        // Closer desert with same heat wins (lower dist → higher old fitness? new /25 vs old/(quad+25)).
        // old at far 10,10 quad=200: (1.0-0.5)/(200+25)=0.5/225; new at 1,0 felt 0.8: 0.3/25=0.012 < old 0.0022 wait
        // old_place_temp 1.0, quad from (1,0) to (10,10) = 9^2+10^2=181; old_fit=0.5/206≈0.0024
        // new felt 0.8: 0.3/25=0.012 → replace
        let replaced = consider_warm_place(Some((10, 10)), 1, 0, DESERT, 0.8, 1.0, 32, 32, false);
        assert_eq!(replaced, Some((1, 0)));
    }

    #[test]
    fn cold_place_water_fitness_doubles() {
        let seeded = consider_cold_place(None, 2, 2, BIOME_SNOW, 0.2, 0.0, 0, 32, 32, false);
        assert_eq!(seeded, Some((2, 2)));
        // New water vs old snow: water ×2 should replace similar temps.
        let water = consider_cold_place(
            Some((0, 0)),
            1,
            0,
            PASSABLE_RIVER,
            0.1,
            0.0,
            BIOME_SNOW,
            32,
            32,
            false,
        );
        assert_eq!(water, Some((1, 0)));
        assert_eq!(plan_temp_place_goto(0.95, Some((1, 1)), Some((9, 9))), Some((9, 9)));
        assert_eq!(plan_temp_place_goto(0.05, Some((1, 1)), Some((9, 9))), Some((1, 1)));
        assert_eq!(plan_temp_place_goto(0.5, Some((1, 1)), Some((9, 9))), None);
    }

    #[test]
    fn desert_tick_records_warm_place() {
        let mut world = World::new(32, 32, false);
        world.set_biome(2, 2, DESERT);
        let content = ContentDb::default();
        let mut map_time = WorldMapTimeState::default();
        let extras = TempAmbientExtras::default();
        let (_, amb, places) = update_player_temperature_ex(
            &world,
            &content,
            &mut map_time,
            2,
            2,
            0.0,
            1.0,
            0.5,
            &[],
            TEMPERATURE_IMPACT_PER_SEC,
            TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            TEMPERATURE_IN_WATER_FACTOR,
            HOT_SEASON_TEMPERATURE_FACTOR,
            COLD_SEASON_TEMPERATURE_FACTOR,
            0,
            ClothingTempKnobs::default(),
            extras,
        );
        assert!(amb > WARM_PLACE_TEMP, "desert amb={amb}");
        assert_eq!(places.warm, Some((2, 2)));
    }
}
