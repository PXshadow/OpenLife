//! Pure movement / range / wrap / speed rules for Open Life Rust server.
//!
//! **Compile unit:** editing these helpers should not recompile all of `ol-sim`
//! once sim only re-exports them. Live path/tick still owns `SimState` mutation.
//!
//! Haxe map: `MoveHelper.calculateSpeed` / `isCloseUseExact` / wrap math / jump plans.
//!
//! Not here: temperature (`temperature_handler`), food eating (`food_eating`), world decay.

#![forbid(unsafe_code)]

pub mod action_range;
pub mod close_exact;
pub mod distance;
pub mod jump_pure;
pub mod math_wrap;
pub mod path_steps;
pub mod speed;

pub use action_range::{
    check_if_not_moving_and_close_enough, effective_use_distance, in_use_range, in_use_range_ex,
    is_close,
};
pub use close_exact::{
    is_close_use_exact, is_close_use_exact_f, is_close_use_exact_f_wrap, is_close_use_exact_wrap,
    kill_in_deadly_range, refuse_ranged_kill_too_close, refuse_ranged_use_too_close,
    KILL_DEADLY_RANGE_SLACK, RANGED_DEADLY_DISTANCE_THRESHOLD, RANGED_MIN_USE_DISTANCE,
};
pub use distance::{calculate_distance_sq, calculate_exact_quad_distance_f};
pub use jump_pure::{
    jump_not_held_emits_bw, jump_should_say_exhausted, plan_jump_to_non_blocked, plan_player_jump,
    JumpAction, JUMP_EXHAUSTED_SAY, JUMP_TO_NON_BLOCKED_OFFSETS,
};
pub use math_wrap::{
    chebyshev_wrap, euclidean_wrap, format_wrap_query, haxe_transform_axis, haxe_transform_xy,
    manhattan_wrap, step_wrap, wrap_axis, wrap_delta, wrap_delta_1d, wrap_tile,
};
pub use path_steps::{
    client_path_deltas_to_steps, step_len, steps_to_client_path_deltas, MAX_CLIENT_PATH_STEPS,
};
pub use speed::{
    adjust_contained_speed_mult, adjust_held_speed_mult, ai_class_speed_factor,
    ai_class_speed_factor_ex, apply_calculate_speed_full, apply_calculate_speed_full_bloody,
    apply_calculate_speed_full_live, apply_calculate_speed_full_live_bloody,
    apply_floor_road_to_speed, apply_held_floor_speed, apply_held_floor_speed_at,
    apply_held_floor_speed_at_ex, apply_held_floor_speed_at_ex_bloody, apply_held_floor_speed_ex,
    apply_vitals_speed_polish, apply_vitals_speed_polish_live, backpack_nest_speed_product,
    backpack_speed_product, backpack_speed_product_ex, clamp_held_speed_bad_biome,
    close_enemy_speed_factor, close_enemy_speed_factor_ex, combine_backpack_and_held_nest,
    contained_obj_speed_mult, contained_obj_speed_mult_ex,
    effective_biome_speed, effective_biome_speed_ex, floor_counts_as_road, floor_road_biome_factor,
    floor_road_factor_at, floor_speed_mult, grave_curse_speed_factor, grave_curse_speed_factor_ex,
    half_penalty_for_strong,
    has_both_shoes, heat_is_super_cold, heat_is_super_hot, held_nest_speed_product,
    held_nest_speed_product_ex, held_object_speed_mult, held_object_speed_mult_ex,
    hitpoints_speed_factor, is_horse_or_car, is_water_biome, object_is_boat, path_length,
    resolve_backpack_speed_product, resolve_backpack_speed_product_ex,
    scan_path_road_and_biome, shoe_pair_ids, shoes_soften_backpack_product, shoes_speed_factor,
    shoes_speed_factor_ex,
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
