//! Floor / road / biome move-speed factors — **canonical** in `ol-move-rules`.
//!
//! COMBAT-BLOODY held override stays here (weapons crate stays in `ol-sim`).
//! Prefer editing `crates/ol-move-rules/src/speed.rs`.

use ol_content::ContentDb;
use ol_world::{NestedHelper, World};

pub use ol_move_rules::speed::{
    adjust_contained_speed_mult, adjust_held_speed_mult, ai_class_speed_factor,
    ai_class_speed_factor_ex, apply_floor_road_to_speed, apply_held_floor_speed,
    apply_held_floor_speed_at, apply_held_floor_speed_at_ex, apply_held_floor_speed_ex,
    apply_vitals_speed_polish, apply_vitals_speed_polish_live, backpack_nest_speed_product,
    backpack_speed_product, backpack_speed_product_ex, clamp_held_speed_bad_biome,
    close_enemy_speed_factor, close_enemy_speed_factor_ex, combine_backpack_and_held_nest,
    contained_obj_speed_mult, contained_obj_speed_mult_ex, effective_biome_speed,
    effective_biome_speed_ex, floor_counts_as_road, floor_road_biome_factor, floor_road_factor_at,
    floor_speed_mult, grave_curse_speed_factor, grave_curse_speed_factor_ex,
    half_penalty_for_strong, has_both_shoes, heat_is_super_cold, heat_is_super_hot,
    held_nest_speed_product, held_nest_speed_product_ex, held_object_speed_mult_ex,
    hitpoints_speed_factor, is_horse_or_car, is_water_biome, object_is_boat, path_length,
    resolve_backpack_speed_product, scan_path_road_and_biome, shoe_pair_ids,
    shoes_soften_backpack_product, shoes_speed_factor, shoes_speed_factor_ex,
    soften_contained_speed_on_floor, soften_held_speed_on_floor, temperature_speed_factor,
    tile_biome_blocks_move, tile_biome_speed, truncate_path_with_road, vitals_speed_product,
    vitals_speed_product_ex, vitals_speed_product_live, PathRoadScan, SpeedPrestigeClass,
    VitalsSpeedInput, VitalsSpeedLiveKnobs, AI_SPEED_FACTOR_COMMONER, AI_SPEED_FACTOR_NOBLE,
    AI_SPEED_FACTOR_SERF, BOAT_ON_LAND_SPEED_FACTOR, CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,
    CLOSE_GRAVE_SPEED_MALI, CONTAINED_SPEED_FLOOR, GROWN_UP_FOOD_STORE_MAX, HITPOINTS_SPEED_FACTOR,
    HORSE_OR_CAR_SPEED_THRESHOLD, INITIAL_PLAYER_MOVE_SPEED, MIN_BIOME_SPEED_FACTOR,
    MIN_SPEED_REDUCTION_PER_CONTAINED, ROAD_SPEED_THRESHOLD, SPEED_FACTOR, SPEED_WITH_BOTH_SHOES,
    TEMPERATURE_SPEED_IMPACT, TRUNC_MOVEMENT_SPEED_DIFF,
};

/// Held object `speedMult` with COMBAT-BLOODY PatchObjectData override.
// Haxe: ServerSettings.PatchObjectData speedMult 750/3048/749
#[inline]
pub fn held_object_speed_mult(content: &ContentDb, held_id: i32) -> f32 {
    held_object_speed_mult_ex(
        content,
        held_id,
        crate::weapons::bloody_weapon_speed_mult(held_id),
    )
}

/// Full Haxe `calculateSpeed` with COMBAT-BLOODY held override from sim weapons.
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
    ol_move_rules::apply_calculate_speed_full_bloody(
        world,
        content,
        tx,
        ty,
        base_speed,
        full_path_has_road,
        held_id,
        backpack,
        clothing_pack,
        vitals,
        crate::weapons::bloody_weapon_speed_mult(held_id),
    )
}

/// Live-knob variant with COMBAT-BLOODY held override.
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
    ol_move_rules::apply_calculate_speed_full_live_bloody(
        world,
        content,
        tx,
        ty,
        base_speed,
        full_path_has_road,
        held_id,
        backpack,
        clothing_pack,
        vitals,
        knobs,
        crate::weapons::bloody_weapon_speed_mult(held_id),
    )
}

/// Convert sim [`crate::prestige::PrestigeClass`] → speed-factor class tag.
#[inline]
pub fn speed_prestige_class(class: crate::prestige::PrestigeClass) -> SpeedPrestigeClass {
    SpeedPrestigeClass::from_u8(class as u8)
}
