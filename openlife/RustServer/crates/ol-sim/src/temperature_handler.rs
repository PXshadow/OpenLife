//! # Temperature handling (Haxe `TemperatureHandler` + GPI `updateTemperature`)
//!
//! **Canonical home for ambient / body temperature.** New heat/temp logic
//! belongs here (or in the thin helpers this module re-exports) — not in food
//! eating, combat, or world auto-decay.
//!
//! ## Haxe map
//!
//! | Haxe | Rust |
//! |------|------|
//! | `TemperatureHandler.BalanceTemperatureArea` | [`apply_balance_temperature_area`] |
//! | Tile init / local heat on the map | [`ensure_tile_temperature`], `world_time` tile helpers |
//! | `GlobalPlayerInstance.updateTemperature` | [`update_player_temperature`] |
//! | Body heat toward ambient (ideal 0.5) | [`crate::heat_ideal`] |
//!
//! ## What belongs here
//!
//! - Sparse tile temperature for a player neighborhood
//! - Balancing Chebyshev radius around the player
//! - Integrating `Player.heat` toward ambient
//! - Clothing / insulation helpers used by that path (worn rValue matrix)
//!
//! ## What does **not** belong here
//!
//! - Food eat / yum fill → [`crate::food_eating`]
//! - Age / wound heal food pipes → player tick / `food_store_max`
//! - Map auto-decay, animals, snow → [`crate::world_time`] / `long_term`
//!
//! Live call site: the **player** tick slice (`tick_vitals`) calls
//! [`update_player_temperature`] per player. That is player-time orchestration,
//! not world map-slice ownership of body heat.

// Façade re-exports: callers may use `temperature_handler::*` or the leaf modules.
#[allow(unused_imports)]
pub use crate::heat_ideal::{
    apply_clothing_warmth, body_heat_step, clamp_heat, env_heat, format_heat_ideal_query,
    heat_error, heat_food_drain_time, heat_food_extra, heat_move_mult, heat_signed_error,
    is_comfortable, is_super_cold, is_super_hot, label_for_heat, HeatLabel, COMFORT_RADIUS,
    EXTREME_RADIUS, HEAT_DAMAGE_DOUBLE_COLD, HEAT_DAMAGE_DOUBLE_HOT, HEAT_FOOD_EXTRA_CAP,
    HEAT_FOOD_EXTRA_SCALE, HEAT_FOOD_USE_PER_SECOND, HEAT_TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
    HEAT_TEMPERATURE_HITS_DAMAGE_FACTOR, IDEAL_HEAT, TEMPERATURE_IMPACT_PER_SEC,
    TEMPERATURE_IMPACT_PER_SEC_IF_GOOD, TEMPERATURE_IMPACT_REDUCTION, TEMPERATURE_IN_WATER_FACTOR,
};

/// Haxe HEAT `foodUsePerSecond` + `foodDrainTime` with person-color super gates.
///
/// Indoor bonus is always 0 on the wire (`heat foodDrainTime 0`).
// Haxe: GlobalPlayerInstance.updateTemperature L6530–6552
pub fn player_heat_food_drain(
    heat: f32,
    person_color: i32,
    food_use_per_second: f32,
    hits_damage_factor: f32,
    exhaustion_damage_factor: f32,
    impact_below: f32,
    color_factor: f32,
) -> (f32, f32) {
    let hot = crate::player_soul::is_super_hot_for_person_ex(
        heat,
        person_color,
        impact_below,
        color_factor,
    );
    let cold = crate::player_soul::is_super_cold_for_person_ex(
        heat,
        person_color,
        impact_below,
        color_factor,
    );
    heat_food_drain_time(
        heat,
        hot,
        cold,
        food_use_per_second,
        hits_damage_factor,
        exhaustion_damage_factor,
    )
}

#[allow(unused_imports)]
pub use crate::map_temp_player::{
    apply_balance_temperature_area, apply_biome_love_temperature, apply_clothing_to_ambient,
    apply_color_temperature_shift, apply_stored_water_cool, clothing_body_factor,
    clothing_heat_protection_sum, clothing_insulation_sum, consider_cold_place,
    consider_warm_place, ensure_tile_temperature, get_tile_temperature, plan_temp_place_goto,
    temperature_shift_for_color, update_player_temperature, ClothingTempKnobs, TempAmbientExtras,
    TempPlaceState, PLAYER_BALANCE_TEMP_RADIUS, PLAYER_TEMP_TIME_PASSED_CAP,
    TEMPERATURE_CLOTHING_FACTOR, TEMPERATURE_CLOTHING_INSULATION_FACTOR,
    TEMPERATURE_NATURAL_HEAT_INSULATION,
};

/// Documentation anchor: Haxe class this module mirrors.
pub const HAXE_TEMPERATURE_HANDLER: &str = "server/TemperatureHandler.hx";
