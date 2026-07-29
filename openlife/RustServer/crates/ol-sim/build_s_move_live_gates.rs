//! Build-time wire for **S-MOVE-LIVE-GATES** / `grave_enemy_live`
//! + **MOVE-NEST-SPEED** / `held_nest_mult` vitals wire.
//!
//! Ensures mod + player fields + live speed gates in player_move_speed/path start.
//! Idempotent. Prefers Python scripts when available.

use std::path::Path;
use std::process::Command;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

pub fn live_gates_wired(lib_text: &str, player_text: &str) -> bool {
    let no_placeholder = !lib_text.contains("placeholder replaced below");
    let pms_live = lib_text
        .contains("let (curse_active, close_hostile) = live_move_speed_gates(state, p);")
        || (lib_text.contains("curse_active: live_move_speed_gates(state, p).0")
            && lib_text.contains("close_hostile_with_weapon: live_move_speed_gates(state, p).1"));
    // MOVE-NEST-SPEED: held nest product on both calculateSpeed live paths
    let nest_live = lib_text.contains("held_nest_product: held_nest_speed_product")
        && lib_text.contains("held_nest_speed_product");
    lib_text.contains("mod move_live_gates;")
        && lib_text.contains("fn live_move_speed_gates(")
        && lib_text.contains("apply_grave_curse_live_gates")
        && player_text.contains("pub is_cursed:")
        && player_text.contains("pub angry_time:")
        && no_placeholder
        && pms_live
        && nest_live
}

fn try_run_python(script: &Path) {
    if !script.exists() {
        return;
    }
    let _ = Command::new("python")
        .arg(script)
        .status()
        .or_else(|_| Command::new("python3").arg(script).status())
        .or_else(|_| Command::new("py").arg("-3").arg(script).status());
}

/// Apply S-MOVE-LIVE-GATES + MOVE-NEST-SPEED. Returns true when ready.
pub fn patch_s_move_live_gates(src: &Path, workspace: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let player_path = src.join("player.rs");

    // Always attempt nest wire (idempotent) before early-out.
    let _ = patch_lib_move_nest(&lib_path);
    // Port kit docs for this chunk (idempotent).
    try_run_python(&workspace.join("docs/port/_patch_move_nest_docs.py"));

    let lib_t = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_t = std::fs::read_to_string(&player_path).unwrap_or_default();
    if live_gates_wired(&lib_t, &player_t) {
        return true;
    }

    for rel in [
        "docs/port/_patch_s_move_live_gates.py",
        "docs/port/_fix_s_move_live_gates_pms.py",
    ] {
        let script = workspace.join(rel);
        if !script.exists() {
            continue;
        }
        let py = Command::new("python")
            .arg(&script)
            .status()
            .or_else(|_| Command::new("python3").arg(&script).status())
            .or_else(|_| Command::new("py").arg("-3").arg(&script).status());
        if let Ok(s) = py {
            if s.success() {
                let _ = patch_lib_move_nest(&lib_path);
                let lib2 = std::fs::read_to_string(&lib_path).unwrap_or_default();
                let p2 = std::fs::read_to_string(&player_path).unwrap_or_default();
                if live_gates_wired(&lib2, &p2) {
                    return true;
                }
            }
        }
    }

    let _ = patch_player_rs(&player_path);
    let _ = patch_lib_rs(&lib_path);
    let _ = patch_lib_move_nest(&lib_path);
    let lib_f = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_f = std::fs::read_to_string(&player_path).unwrap_or_default();
    live_gates_wired(&lib_f, &player_f)
}

/// MOVE-NEST-SPEED: VitalsSpeedInput.held_nest_product + re-exports.
fn patch_lib_move_nest(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let orig = t.clone();

    // Re-exports for held_nest_speed_product
    if !t.contains("held_nest_speed_product") {
        let old = "    apply_vitals_speed_polish, backpack_speed_product, ballast_speed_mult,\n\
    clamp_held_speed_bad_biome, close_enemy_speed_factor, compose_move_speed,\n\
    compose_move_speed_with_floor, contained_obj_speed_mult, effective_biome_speed,\n\
    floor_counts_as_road, floor_road_biome_factor, floor_road_factor_at, floor_speed_mult,\n\
    format_speed_query, format_weight_query, grave_curse_speed_factor,\n\
    half_penalty_for_strong, has_both_shoes, heat_is_super_cold, heat_is_super_hot,\n\
    held_object_speed_mult, hitpoints_speed_factor, is_horse_or_car, is_water_biome,\n";
        let new = "    apply_vitals_speed_polish, backpack_speed_product, ballast_speed_mult,\n\
    clamp_held_speed_bad_biome, close_enemy_speed_factor, combine_backpack_and_held_nest,\n\
    compose_move_speed, compose_move_speed_with_floor, contained_obj_speed_mult,\n\
    effective_biome_speed, floor_counts_as_road, floor_road_biome_factor,\n\
    floor_road_factor_at, floor_speed_mult, format_speed_query, format_weight_query,\n\
    grave_curse_speed_factor, half_penalty_for_strong, has_both_shoes,\n\
    heat_is_super_cold, heat_is_super_hot, held_nest_speed_product,\n\
    held_object_speed_mult, hitpoints_speed_factor, is_horse_or_car, is_water_biome,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        }
    }

    // Path-start vitals + player_move_speed vitals
    if !t.contains("held_nest_product: held_nest_speed_product") {
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
        if t.contains(old1) {
            t = t.replacen(old1, new1, 1);
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
        if t.contains(old2) {
            t = t.replacen(old2, new2, 1);
        }
    }

    if t == orig {
        return t.contains("held_nest_product: held_nest_speed_product");
    }
    let _ = std::fs::write(lib_path, restore_nl(&t, crlf));
    t.contains("held_nest_product: held_nest_speed_product")
}

fn patch_player_rs(player_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(player_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    if t.contains("pub is_cursed:") && t.contains("pub angry_time:") {
        return true;
    }

    if !t.contains("pub is_cursed:") {
        let old = "    pub prestige_class: PrestigeClass,\n";
        let new = "    pub prestige_class: PrestigeClass,\n    /// Haxe `isCursed` (close-grave speed mali + CU).\n    pub is_cursed: bool,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        } else {
            return false;
        }
    }
    if !t.contains("pub angry_time:") {
        let old = "    pub is_cursed: bool,\n";
        let new = "    pub is_cursed: bool,\n    /// Haxe `angryTime` (close-enemy weapon speed when < 0).\n    pub angry_time: f32,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        } else {
            return false;
        }
    }

    let _ = std::fs::write(player_path, restore_nl(&t, crlf));
    true
}

fn patch_lib_rs(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod move_live_gates;") {
        let old = "mod move_path;\n// Haxe: MoveHelper.calculateSpeed — pure helpers live in move_speed.rs via move_notes.\n";
        let new = "mod move_path;\n// Haxe: MoveHelper.calculateSpeed grave/enemy live gates (S-MOVE-LIVE-GATES)\nmod move_live_gates;\n// Haxe: MoveHelper.calculateSpeed — pure helpers live in move_speed.rs via move_notes.\n";
        if !t.contains(old) {
            return false;
        }
        t = t.replacen(old, new, 1);
        changed = true;
    }

    if !t.contains("pub use move_live_gates::") {
        let insert = "pub use move_live_gates::{\n    calculate_close_blocking_grave_fitness, calculate_distance_sq,\n    close_hostile_weapon_speed_active, format_cursed_message,\n    has_close_blocking_grave, has_close_hostile_with_weapon,\n    is_close_use_exact, is_friendly_ally_only, resolve_grave_curse,\n    ClosePlayerCandidate, GraveCurseTransition, BLOCKING_GRAVE_FITNESS_CAP,\n    BLOCKING_GRAVE_FITNESS_THRESHOLD, CLOSE_ENEMY_WEAPON_DISTANCE,\n    COMBAT_ANGRY_TIME_BEFORE_ATTACK, CURSE_CLEAR_EMOTE_INDEX,\n    CURSE_CLEAR_SAY, CURSE_ENTER_EMOTE_INDEX, CURSE_ENTER_SAY,\n    GRAVE_BLOCKING_DISTANCE, GRAVE_CURSE_CLEAR_DISTANCE_MULT,\n    MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE,\n};\n";
        if t.contains("pub use move_notes::{\n") {
            t = t.replacen(
                "pub use move_notes::{\n",
                &format!("{insert}pub use move_notes::{{\n"),
                1,
            );
            changed = true;
        }
    }

    if !t.contains("fn live_move_speed_gates(") {
        let helpers = include_str!("src/_s_move_live_gates_helpers.rs.txt");
        if let Some(idx) = t.find(
            "/// Reported move speed for PU / FX from ride + weather + snow + fire + ballast",
        ) {
            t = format!("{}{}{}", &t[..idx], helpers, &t[idx..]);
            changed = true;
        } else {
            return false;
        }
    }

    let old_ph = "        // S-MOVE-LIVE-GATES: live grave curse + close enemy weapon.\n        curse_active: false, // placeholder replaced below\n        close_hostile_with_weapon: false,";
    let new_ph = "        // S-MOVE-LIVE-GATES: live grave curse + close enemy weapon.\n        curse_active: live_move_speed_gates(state, p).0,\n        close_hostile_with_weapon: live_move_speed_gates(state, p).1,";
    if t.contains(old_ph) {
        t = t.replacen(old_ph, new_ph, 1);
        changed = true;
    }

    let old_path = "        let vitals = VitalsSpeedInput {\n            has_both_shoes: has_both_shoes(p.shoes),\n            on_horse_or_car: on_horse,\n            current_food_store_max: p.food_max,\n            heat,\n            curse_active: false,\n            close_hostile_with_weapon: false,\n            is_ai,\n            prestige_class: class,\n            is_strong,\n        };";
    let new_path = "        let (curse_active, close_hostile) = live_move_speed_gates(state, p);\n        let vitals = VitalsSpeedInput {\n            has_both_shoes: has_both_shoes(p.shoes),\n            on_horse_or_car: on_horse,\n            current_food_store_max: p.food_max,\n            heat,\n            curse_active,\n            close_hostile_with_weapon: close_hostile,\n            is_ai,\n            prestige_class: class,\n            is_strong,\n        };";
    if t.contains(old_path) {
        t = t.replacen(old_path, new_path, 1);
        changed = true;
    }

    let old_pms2 = "    let vitals = VitalsSpeedInput {\n        has_both_shoes: has_both_shoes(p.shoes),\n        on_horse_or_car: on_horse,\n        current_food_store_max: p.food_max,\n        heat,\n        // Grave curse population gate + account grave book remain product polish.\n        curse_active: false,\n        close_hostile_with_weapon: false,\n        is_ai,\n        prestige_class: class,\n        is_strong,\n    };";
    let new_pms2 = "    let (curse_active, close_hostile) = live_move_speed_gates(state, p);\n    let vitals = VitalsSpeedInput {\n        has_both_shoes: has_both_shoes(p.shoes),\n        on_horse_or_car: on_horse,\n        current_food_store_max: p.food_max,\n        heat,\n        // S-MOVE-LIVE-GATES: live grave curse + close enemy weapon.\n        curse_active,\n        close_hostile_with_weapon: close_hostile,\n        is_ai,\n        prestige_class: class,\n        is_strong,\n    };";
    if t.contains(old_pms2) {
        t = t.replacen(old_pms2, new_pms2, 1);
        changed = true;
    }

    if !t.contains("apply_grave_curse_live_gates(state, outbound, conn_id)") {
        let old_ps = "    let p_id = {\n        let p = state.players.get_mut(&conn_id).ok_or(MoveReject::NoPlayer)?;\n        p.move_path = Some(path);\n        p.moving = true;\n        p.p_id\n    };";
        let new_ps = "    let p_id = {\n        let p = state.players.get_mut(&conn_id).ok_or(MoveReject::NoPlayer)?;\n        p.move_path = Some(path);\n        p.moving = true;\n        p.p_id\n    };\n    // S-MOVE-LIVE-GATES: mutate is_cursed + CU/PE/say on enter/clear.\n    apply_grave_curse_live_gates(state, outbound, conn_id);";
        if t.contains(old_ps) {
            t = t.replacen(old_ps, new_ps, 1);
            changed = true;
        }
    }

    if !t.contains("tp.angry_time -= dmg") {
        let old_hit = "                    HitResult::Wound(w) => {\n                        // Reduce food_max (HP) by damage (Haxe food_store_max from hits).\n                        if let Some(tp) =\n                            state.players.values_mut().find(|x| x.p_id == target_id)\n                        {\n                            tp.food_max = (tp.food_max - dmg).max(FOOD_MAX_DEATH);\n                            if tp.food > tp.food_max {\n                                tp.food = tp.food_max;\n                            }\n                        }";
        let new_hit = "                    HitResult::Wound(w) => {\n                        // Reduce food_max (HP) by damage (Haxe food_store_max from hits).\n                        // Haxe DoDamage: angryTime -= damage on both parties.\n                        if let Some(tp) =\n                            state.players.values_mut().find(|x| x.p_id == target_id)\n                        {\n                            tp.food_max = (tp.food_max - dmg).max(FOOD_MAX_DEATH);\n                            if tp.food > tp.food_max {\n                                tp.food = tp.food_max;\n                            }\n                            tp.angry_time -= dmg;\n                        }\n                        if let Some(kp) =\n                            state.players.values_mut().find(|x| x.p_id == killer_id)\n                        {\n                            kp.angry_time -= dmg;\n                        }";
        if t.contains(old_hit) {
            t = t.replacen(old_hit, new_hit, 1);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(lib_path, restore_nl(&t, crlf));
    }
    true
}
