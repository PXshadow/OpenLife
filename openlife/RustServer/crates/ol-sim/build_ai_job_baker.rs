//! AI-JOB-BAKER: wire `baker_profession` mod + exports + priority_ladder Baker arms (idempotent).

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

/// True when baker profession symbols are re-exported from lib.rs.
pub fn baker_job_wired(lib_path: &PathBuf) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| {
            t.contains("mod baker_profession;")
                && t.contains("pub use baker_profession::{")
                && t.contains("do_baking")
                && t.contains("has_or_become_baker")
                && t.contains("pick_baker_goal")
                && t.contains("BAKER_TARGET_ID")
        })
        .unwrap_or(false)
}

const MOD_OLD: &str = r#"// Haxe: AiBase farmer profession family (AI-JOB-FARM)
mod farmer_profession;
"#;

const MOD_NEW: &str = r#"// Haxe: AiBase farmer profession family (AI-JOB-FARM)
mod farmer_profession;
// Haxe: AiBase baker profession family (AI-JOB-BAKER)
mod baker_profession;
"#;

// Prefer insert after smith if present, else after farmer export.
const USE_AFTER_SMITH: &str = r#"    SMITH_PROFESSION_KEY, STEEL_AXE, STEEL_HOE, STEEL_INGOT, STEEL_MINING_PICK, STONE,
    WROUGHT_IRON,
};
"#;

const USE_AFTER_FARMER: &str = r#"    RIPE_WHEAT, THRESHED_WHEAT, WHEAT_SHEAF, WET_PLANTED_CARROTS,
};
"#;

const USE_BLOCK: &str = r#"// Haxe: AiBase baker profession (AI-JOB-BAKER)
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

const AI_GOALS_OLD: &str = r#"    ESCAPE_HUNT_MIN_AGE, ESCAPE_PLAYER_DIST_MAX, FARMER_TARGET_ID, HUNGRY_FOOD, HUNGRY_ENTER_FLOOR,
    HUNGRY_ENTER_FRAC, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED, MIN_AGE_TO_EAT, SMITH_IRON_ID,
    SMITH_TARGET_ID, SMITHING_HAMMER_ID,
};"#;

const AI_GOALS_NEW: &str = r#"    ESCAPE_HUNT_MIN_AGE, ESCAPE_PLAYER_DIST_MAX, BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD,
    HUNGRY_ENTER_FLOOR, HUNGRY_ENTER_FRAC, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED,
    MIN_AGE_TO_EAT, SMITH_IRON_ID, SMITH_TARGET_ID, SMITHING_HAMMER_ID,
};"#;

/// Expand lib.rs for baker profession module (idempotent).
pub fn patch_lib_ai_job_baker(lib_path: &PathBuf) -> bool {
    if baker_job_wired(lib_path) {
        return true;
    }
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);

    if !text.contains("mod baker_profession;") {
        if text.contains(MOD_OLD) {
            text = text.replacen(MOD_OLD, MOD_NEW, 1);
        } else if text.contains("mod farmer_profession;")
            && !text.contains("mod baker_profession;")
        {
            text = text.replacen(
                "mod farmer_profession;\n",
                "mod farmer_profession;\n// Haxe: AiBase baker profession family (AI-JOB-BAKER)\nmod baker_profession;\n",
                1,
            );
        } else {
            return false;
        }
    }

    if !text.contains("pub use baker_profession::{") {
        if text.contains(USE_AFTER_SMITH) {
            text = text.replacen(
                USE_AFTER_SMITH,
                &format!("{USE_AFTER_SMITH}{USE_BLOCK}"),
                1,
            );
        } else if text.contains(USE_AFTER_FARMER) {
            text = text.replacen(
                USE_AFTER_FARMER,
                &format!("{USE_AFTER_FARMER}{USE_BLOCK}"),
                1,
            );
        } else {
            return false;
        }
    }

    if !text.contains("BAKER_TARGET_ID") {
        if text.contains(AI_GOALS_OLD) {
            text = text.replacen(AI_GOALS_OLD, AI_GOALS_NEW, 1);
        } else if text.contains("FARMER_TARGET_ID, HUNGRY_FOOD")
            && !text.contains("BAKER_TARGET_ID")
        {
            text = text.replacen(
                "FARMER_TARGET_ID, HUNGRY_FOOD",
                "BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD",
                1,
            );
        }
    }

    let out = restore_nl(&text, crlf);
    std::fs::write(lib_path, out).is_ok() && baker_job_wired(lib_path)
}

/// Patch priority_ladder goal_from_rung for Profession::Baker (idempotent).
pub fn patch_priority_ladder_baker(prio_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(prio_path) else {
        return false;
    };
    if raw.contains("Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID)") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);

    // Import BAKER_TARGET_ID
    let old_use = "use super::{Goal, Profession, FARMER_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID};";
    let new_use =
        "use super::{Goal, Profession, BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID};";
    if text.contains(old_use) {
        text = text.replacen(old_use, new_use, 1);
    } else if !text.contains("BAKER_TARGET_ID") {
        // Already different formatting — try insert after FARMER
        text = text.replacen(
            "FARMER_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID",
            "BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID",
            1,
        );
    }

    // Craft/job match
    let old_craft = r#"            Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
            Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
            Profession::Hunter if prey_adjacent => Goal::Hunt,"#;
    let new_craft = r#"            Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
            Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
            Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
            Profession::Hunter if prey_adjacent => Goal::Hunt,"#;
    if text.contains(old_craft) {
        text = text.replacen(old_craft, new_craft, 1);
    }

    // Idle match
    let old_idle = r#"                Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
                Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
                Profession::Explorer => Goal::Explore,"#;
    let new_idle = r#"                Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
                Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
                Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
                Profession::Explorer => Goal::Explore,"#;
    if text.contains(old_idle) {
        text = text.replacen(old_idle, new_idle, 1);
    }

    let ok = text.contains("Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID)");
    if ok {
        let out = restore_nl(&text, crlf);
        let _ = std::fs::write(prio_path, out);
    }
    ok
}

/// Patch ol-server selfplay to use baker pipeline when Profession::Baker (idempotent).
pub fn patch_selfplay_baker(selfplay_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(selfplay_path) else {
        return false;
    };
    if raw.contains("pick_baker_goal(&craft_graph") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);

    // Import
    if !text.contains("pick_baker_goal") {
        if text.contains("pick_farmer_goal, pick_goal_ext") {
            text = text.replacen(
                "pick_farmer_goal, pick_goal_ext",
                "pick_baker_goal, pick_farmer_goal, pick_goal_ext",
                1,
            );
        }
    }
    if !text.contains("BAKER_TARGET_ID") {
        if text.contains("FARMER_TARGET_ID,") {
            text = text.replacen("FARMER_TARGET_ID,", "BAKER_TARGET_ID, FARMER_TARGET_ID,", 1);
        }
    }

    // craft_want arm
    let old_want = "(Profession::Farmer, _) => Some(FARMER_TARGET_ID),";
    let new_want = "(Profession::Farmer, _) => Some(FARMER_TARGET_ID),\n            (Profession::Baker, _) => Some(BAKER_TARGET_ID),";
    if text.contains(old_want) && !text.contains("(Profession::Baker, _)") {
        text = text.replacen(old_want, new_want, 1);
    }

    // Expand matches for Farmer | Smith to include Baker
    if text.contains("Profession::Farmer | Profession::Smith")
        && !text.contains("Profession::Farmer | Profession::Smith | Profession::Baker")
    {
        text = text.replace(
            "Profession::Farmer | Profession::Smith",
            "Profession::Farmer | Profession::Smith | Profession::Baker",
        );
    }

    // Plan branch after farmer
    let old_plan = r#"                } else if profession == Profession::Farmer {
                    // Haxe: AI-JOB-FARM pipeline + reverse-craft intermediates (not only 242).
                    goal = pick_farmer_goal(&craft_graph, &have);"#;
    let new_plan = r#"                } else if profession == Profession::Farmer {
                    // Haxe: AI-JOB-FARM pipeline + reverse-craft intermediates (not only 242).
                    goal = pick_farmer_goal(&craft_graph, &have);
                } else if profession == Profession::Baker {
                    // Haxe: AI-JOB-BAKER oven/pie/bread pipeline
                    goal = pick_baker_goal(&craft_graph, &have, 0.0);"#;
    if text.contains(old_plan) && !text.contains("pick_baker_goal(&craft_graph") {
        text = text.replacen(old_plan, new_plan, 1);
    }

    // Goal pick path: also use pick_goal_smith_craft for Baker
    let old_goal = "let mut goal = if profession == Profession::Smith {";
    let new_goal =
        "let mut goal = if matches!(profession, Profession::Smith | Profession::Baker) {";
    if text.contains(old_goal) {
        text = text.replacen(old_goal, new_goal, 1);
    }

    let ok = text.contains("pick_baker_goal(&craft_graph");
    if ok {
        let out = restore_nl(&text, crlf);
        let _ = std::fs::write(selfplay_path, out);
    }
    ok
}
