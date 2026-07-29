//! AI-JOB-SMITH: wire `smith_profession` (and baker if present) into `lib.rs` (idempotent).

use std::path::PathBuf;

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

fn text_smith_wired(t: &str) -> bool {
    t.contains("mod smith_profession;")
        && t.contains("pub use smith_profession::{")
        && t.contains("pick_smith_profession_goal")
        && t.contains("do_smithing")
        && t.contains("has_or_become_smith")
}

/// True when smith profession symbols are re-exported from lib.rs.
pub fn smith_job_wired(lib_path: &PathBuf) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| text_smith_wired(&t))
        .unwrap_or(false)
}

// Note: omit KNIFE (collides with baker_profession::KNIFE if both exported).
const SMITH_USE: &str = r#"// Haxe: AiBase smith profession (AI-JOB-SMITH / AI-JOB-SMITH-WIRE)
pub use smith_profession::{
    assign_smith_from_speech, chebyshev, count_smith_peers_filtered, critical_smith_shortcrafts,
    decide_smith_job, decide_smith_job_for_slot, do_smithing, do_smithing_products,
    fill_smith_counts_from_map, forge_id_priority, has_or_become_smith, infer_smith_stage_from_have,
    is_forge_id, is_steel_crucible_count_id, parse_smith_profession_speech, pick_forge_near_home,
    pick_forge_parent, pick_smith_profession_goal, prepare_smithing_tools,
    resolve_smith_assigned_job, smith_action_to_goal, smith_goal_from_counts_and_rung,
    smith_goal_from_map_and_rung, smith_job_rung_label, smith_job_slot_priority,
    smith_pipeline_targets, smith_slot_for_rung, try_decide_smith_from_rung, wipe_smith_on_eat,
    ForgeCandidate, MapObj, SmithAction, SmithCounts, SmithJobSlot, SmithPeerSnapshot,
    SmithProfessionRuntime, BASKET_OF_CHARCOAL, FIRING_FORGE, FIRING_KILN, FLAT_ROCK,
    FLAT_STONE_COUNT_RADIUS, FORGE, FORGE_SEARCH_RADIUS, FORGE_WITH_CHARCOAL, IRON_ORE,
    IRON_ORE_COUNT_RADIUS, SHEARS, SMITHING_HAMMER, SMITH_PROFESSION_KEY, STEEL_AXE,
    STEEL_CHISEL_FAMILY, STEEL_COUNT_RADIUS, STEEL_HOE, STEEL_INGOT, STEEL_MINING_PICK, STONE,
    WROUGHT_IRON,
};
"#;

const BAKER_USE: &str = r#"// Haxe: AiBase baker profession (AI-JOB-BAKER)
pub use baker_profession::{
    bake_action_to_goal, count_bread_family, count_raw_stuff_to_bake, decide_baker_job, do_baking,
    has_or_become_baker, hot_oven_bake, is_oven_id, knife_bread_stage, make_raw_pies,
    max_dough_in_bowl, needed_raw_to_fire_oven, parse_baker_profession_speech, pick_baker_goal,
    pick_oven_parent, pre_profession_dough, resolve_baker_assigned_job, BakeAction, BakeCounts,
    BakerProfessionRuntime, BakerTaskState, OvenState, ADOBE_OVEN, BAKER_DEFAULT_MAX_PEOPLE,
    BAKER_PROFESSION_KEY, BURNING_OVEN, CLAY_PLATE, COOKED_PIES, HOT_OVEN, LEAVENED_DOUGH_PLATE,
    OVEN_SEARCH_RADIUS, RAW_PIES, SLICED_BREAD, WOOD_FILLED_OVEN,
};
"#;

/// Expand lib.rs for smith (and baker if source present) profession modules.
pub fn patch_lib_ai_job_smith(lib_path: &PathBuf) -> bool {
    let src_dir = lib_path.parent().map(|p| p.to_path_buf());
    let baker_rs_exists = src_dir
        .as_ref()
        .map(|d| d.join("baker_profession.rs").exists())
        .unwrap_or(false);

    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    // baker mod first (ai_goals references crate::baker_profession)
    if baker_rs_exists && !text.contains("mod baker_profession;") {
        if text.contains("mod farmer_profession;") {
            text = text.replacen(
                "mod farmer_profession;\n",
                "mod farmer_profession;\n// Haxe: AiBase baker profession family (AI-JOB-BAKER)\nmod baker_profession;\n",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("mod smith_profession;") {
        if text.contains("mod baker_profession;") {
            text = text.replacen(
                "mod baker_profession;\n",
                "mod baker_profession;\n// Haxe: AiBase smith profession family (AI-JOB-SMITH)\nmod smith_profession;\n",
                1,
            );
            changed = true;
        } else if text.contains("mod farmer_profession;") {
            text = text.replacen(
                "mod farmer_profession;\n",
                "mod farmer_profession;\n// Haxe: AiBase smith profession family (AI-JOB-SMITH)\nmod smith_profession;\n",
                1,
            );
            changed = true;
        } else {
            return false;
        }
    }

    let farm_end = "    RIPE_WHEAT, THRESHED_WHEAT, WHEAT_SHEAF, WET_PLANTED_CARROTS,\n};\n";

    if baker_rs_exists && !text.contains("pub use baker_profession::{") {
        if text.contains(farm_end) {
            text = text.replacen(farm_end, &format!("{farm_end}{BAKER_USE}"), 1);
            changed = true;
        }
    }

    if !text.contains("pub use smith_profession::{") {
        let baker_end =
            "    OVEN_SEARCH_RADIUS, RAW_PIES, SLICED_BREAD, WOOD_FILLED_OVEN,\n};\n";
        if text.contains("pub use baker_profession::{") && text.contains(baker_end) {
            text = text.replacen(baker_end, &format!("{baker_end}{SMITH_USE}"), 1);
            changed = true;
        } else if text.contains(farm_end) {
            text = text.replacen(farm_end, &format!("{farm_end}{SMITH_USE}"), 1);
            changed = true;
        } else if let Some(idx) = text.find("pub use farmer_profession::{") {
            if let Some(rel) = text[idx..].find("};\n") {
                let insert_at = idx + rel + 3;
                text = format!("{}{}{}", &text[..insert_at], SMITH_USE, &text[insert_at..]);
                changed = true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    if baker_rs_exists && !text.contains("BAKER_TARGET_ID") {
        let old_ai = "ESCAPE_HUNT_MIN_AGE, ESCAPE_PLAYER_DIST_MAX, FARMER_TARGET_ID, HUNGRY_FOOD, HUNGRY_ENTER_FLOOR,";
        let new_ai = "ESCAPE_HUNT_MIN_AGE, ESCAPE_PLAYER_DIST_MAX, BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD, HUNGRY_ENTER_FLOOR,";
        if text.contains(old_ai) {
            text = text.replacen(old_ai, new_ai, 1);
            changed = true;
        }
    }

    // STEEL_HOE / SHOVEL may collide with farmer — farmer exports STEEL_HOE!
    // farmer: STEEL_HOE is not in the public export list (checks) — farmer exports don't include STEEL_HOE
    // farmer has STEEL_HOE as pub const but may not re-export at crate root. Looking at farmer pub use —
    // it doesn't export STEEL_HOE. Good.

    if !changed {
        return text_smith_wired(&text);
    }

    let out = restore_nl(&text, crlf);
    if std::fs::write(lib_path, out).is_err() {
        return false;
    }
    smith_job_wired(lib_path)
}

/// Patch ol-server selfplay to use stage-aware smith pipeline (idempotent).
pub fn patch_selfplay_smith(selfplay_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(selfplay_path) else {
        return false;
    };
    if raw.contains("pick_smith_profession_goal(&craft_graph") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);

    if !text.contains("pick_smith_profession_goal") {
        let old = "pick_farmer_goal, pick_goal_ext, pick_goal_smith_craft,\n    pick_smith_goal,";
        let new = "pick_farmer_goal, pick_goal_ext, pick_goal_smith_craft,\n    pick_smith_goal, pick_smith_profession_goal,";
        if text.contains(old) {
            text = text.replacen(old, new, 1);
        } else {
            let old2 =
                "pick_smith_goal, AnimalWorld, Goal, PlayerSnapshot, Profession, ReverseCraftGraph,";
            let new2 = "pick_smith_goal, pick_smith_profession_goal, AnimalWorld, Goal, PlayerSnapshot, Profession, ReverseCraftGraph,";
            if text.contains(old2) {
                text = text.replacen(old2, new2, 1);
            }
        }
    }

    let old_plan = "if profession == Profession::Smith {\n                    goal = pick_smith_goal(&craft_graph, &have, SMITH_IRON_ID);";
    let new_plan = "if profession == Profession::Smith {\n                    // AI-JOB-SMITH: stage-aware pipeline (stage 0 thin selfplay)\n                    goal = pick_smith_profession_goal(&craft_graph, &have, 0.0);";
    if text.contains(old_plan) {
        text = text.replacen(old_plan, new_plan, 1);
    }

    let ok = text.contains("pick_smith_profession_goal(&craft_graph");
    if ok {
        let out = restore_nl(&text, crlf);
        let _ = std::fs::write(selfplay_path, out);
    }
    ok
}
