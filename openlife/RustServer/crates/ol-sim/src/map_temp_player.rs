//! MAP-TEMP-PLAYER / `vitals_tile_temps`
//!
//! Haxe: `TemperatureHandler.BalanceTemperatureArea` + ambient read in
//! `GlobalPlayerInstance.updateTemperature`.
//!
//! Live wire into vitals:
//! 1. Ensure player tile has a temperature in `WorldMapTimeState.tile_temps`
//! 2. Balance Chebyshev area radius 5 (`doLocalHeat=false`) around the player
//! 3. Sample ambient from tile_temps (water halves)
//! 4. Integrate body `heat` toward ambient (Haxe impact-per-sec subset)

use ol_content::ContentDb;
use ol_world::{World, OCEAN, PASSABLE_RIVER};

use crate::heat_ideal::{body_heat_step, clamp_heat, IDEAL_HEAT};
use crate::world_time::{
    balance_temperature_area_coords, balance_tile_temperature, floor_insulation_from_content,
    initialize_tile_temperature, local_heat_from_value, object_insulation_from_content,
    WorldMapTimeState,
};

/// Haxe `BalanceTemperatureArea(..., d=5, ...)` from `updateTemperature`.
pub const PLAYER_BALANCE_TEMP_RADIUS: i32 = 5;

/// Haxe `if (timePassed > 5) timePassed = 5`.
pub const PLAYER_TEMP_TIME_PASSED_CAP: f32 = 5.0;

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
    let t = initialize_tile_temperature(biome, orig, season_impact_raw, local_heat);
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
            current,
            &neighbors,
            delta_time,
            0.0,
            obj_ins,
            floor_ins,
            false, // player path
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
    let dt = delta_time.clamp(0.0, PLAYER_TEMP_TIME_PASSED_CAP);
    let _ = ensure_tile_temperature(world, content, map_time, px, py, season_impact_raw);
    apply_balance_temperature_area(
        world,
        content,
        map_time,
        px,
        py,
        PLAYER_BALANCE_TEMP_RADIUS,
        dt,
    );
    let t = ensure_tile_temperature(world, content, map_time, px, py, season_impact_raw);
    let biome = world.get_biome(px, py);
    apply_water_ambient(t, biome)
}

/// Lightweight clothing factor stub (full insulation matrix deferred).
///
/// `insulationFactor = 1 / (1 + clothingInsulation * 2)` with mild slot weights.
pub fn clothing_factor_from_slots(hat: i32, chest: i32, shoes: i32) -> f32 {
    let mut ins = 0.0_f32;
    if hat > 0 {
        ins += 0.25;
    }
    if chest > 0 {
        ins += 0.5;
    }
    if shoes > 0 {
        ins += 0.25;
    }
    1.0 / (1.0 + ins * 2.0)
}

/// One player temperature tick: ambient from tile_temps + body heat step.
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
    clothing_factor: f32,
) -> (f32, f32) {
    let ambient = player_ambient_from_tile_temps(
        world,
        content,
        map_time,
        px,
        py,
        season_impact_raw,
        delta_time,
    );
    let biome = world.get_biome(px, py);
    let in_water = is_water_biome_temp(biome);
    let heat = body_heat_step(
        current_heat,
        ambient,
        delta_time.clamp(0.0, PLAYER_TEMP_TIME_PASSED_CAP),
        clothing_factor,
        in_water,
    );
    (heat, ambient)
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
            1.0,
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
}
