//! MOVE-NEST-SPEED: wire held_nest_product into lib.rs at build time.

use std::path::PathBuf;

pub fn patch_lib_move_nest_speed(lib_path: &PathBuf) -> bool {
    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let orig = text.clone();

    // Re-exports
    let old_exp = "    apply_vitals_speed_polish, backpack_speed_product, ballast_speed_mult,\n\
    clamp_held_speed_bad_biome, close_enemy_speed_factor, compose_move_speed,\n\
    compose_move_speed_with_floor, contained_obj_speed_mult, effective_biome_speed,\n\
    floor_counts_as_road, floor_road_biome_factor, floor_road_factor_at, floor_speed_mult,\n\
    format_speed_query, format_weight_query, grave_curse_speed_factor,\n\
    half_penalty_for_strong, has_both_shoes, heat_is_super_cold, heat_is_super_hot,\n\
    held_object_speed_mult, hitpoints_speed_factor, is_horse_or_car, is_water_biome,\n";
    let new_exp = "    apply_vitals_speed_polish, backpack_speed_product, ballast_speed_mult,\n\
    clamp_held_speed_bad_biome, close_enemy_speed_factor, combine_backpack_and_held_nest,\n\
    compose_move_speed, compose_move_speed_with_floor, contained_obj_speed_mult,\n\
    effective_biome_speed, floor_counts_as_road, floor_road_biome_factor,\n\
    floor_road_factor_at, floor_speed_mult, format_speed_query, format_weight_query,\n\
    grave_curse_speed_factor, half_penalty_for_strong, has_both_shoes,\n\
    heat_is_super_cold, heat_is_super_hot, held_nest_speed_product,\n\
    held_object_speed_mult, hitpoints_speed_factor, is_horse_or_car, is_water_biome,\n";
    if text.contains(old_exp) {
        text = text.replacen(old_exp, new_exp, 1);
    }

    let old1 = "        let vitals = VitalsSpeedInput {\n\
            has_both_shoes: has_both_shoes(left_shoe, right_shoe),\n\
            on_horse_or_car: on_horse,\n\
            current_food_store_max: p.food_max,\n\
            heat,\n\
            curse_active,\n\
            close_hostile_with_weapon: close_hostile,\n\
            is_ai,\n\
            prestige_class: class,\n\
            is_strong,\n\
        };\n\
        let speed = apply_calculate_speed_full(";
    let new1 = "        let vitals = VitalsSpeedInput {\n\
            has_both_shoes: has_both_shoes(left_shoe, right_shoe),\n\
            on_horse_or_car: on_horse,\n\
            current_food_store_max: p.food_max,\n\
            heat,\n\
            curse_active,\n\
            close_hostile_with_weapon: close_hostile,\n\
            is_ai,\n\
            prestige_class: class,\n\
            is_strong,\n\
            // MOVE-NEST-SPEED: heldObject.containedObjects (+1 nest)\n\
            held_nest_product: held_nest_speed_product(&state.content, p.held_helper.as_ref()),\n\
        };\n\
        let speed = apply_calculate_speed_full(";
    if text.contains(old1) {
        text = text.replacen(old1, new1, 1);
    }

    let old2 = "    let vitals = VitalsSpeedInput {\n\
        has_both_shoes: has_both_shoes(left_shoe, right_shoe),\n\
        on_horse_or_car: on_horse,\n\
        current_food_store_max: p.food_max,\n\
        heat,\n\
        // S-MOVE-LIVE-GATES: live grave curse + close enemy weapon.\n\
        curse_active,\n\
        close_hostile_with_weapon: close_hostile,\n\
        is_ai,\n\
        prestige_class: class,\n\
        is_strong,\n\
    };\n\
    let composed = apply_calculate_speed_full(";
    let new2 = "    let vitals = VitalsSpeedInput {\n\
        has_both_shoes: has_both_shoes(left_shoe, right_shoe),\n\
        on_horse_or_car: on_horse,\n\
        current_food_store_max: p.food_max,\n\
        heat,\n\
        // S-MOVE-LIVE-GATES: live grave curse + close enemy weapon.\n\
        curse_active,\n\
        close_hostile_with_weapon: close_hostile,\n\
        is_ai,\n\
        prestige_class: class,\n\
        is_strong,\n\
        // MOVE-NEST-SPEED: heldObject.containedObjects (+1 nest)\n\
        held_nest_product: held_nest_speed_product(&state.content, p.held_helper.as_ref()),\n\
    };\n\
    let composed = apply_calculate_speed_full(";
    if text.contains(old2) {
        text = text.replacen(old2, new2, 1);
    }

    if text == orig {
        // already wired?
        return text.contains("held_nest_product: held_nest_speed_product");
    }
    if std::fs::write(lib_path, text).is_err() {
        return false;
    }
    true
}
