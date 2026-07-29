//! AI-PRIO priority_ladder build-time patch (included from build.rs).
//!
//! Expands `pub use ai_goals::{...}` so ladder symbols are crate-root exports.
//! Idempotent. Handles CRLF sources.

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

const OLD_EXPORT: &str = r#"pub use ai_goals::{
    format_seeking_query, parse_profession_token, pick_goal, pick_goal_ext, pick_goal_smith_craft,
    pick_smith_goal, smith_product_targets, Goal, Profession, FARMER_TARGET_ID, HUNGRY_FOOD,
    SMITH_IRON_ID, SMITH_TARGET_ID,
};"#;

const NEW_EXPORT: &str = r#"pub use ai_goals::{
    age_job_index, format_seeking_query, goal_from_rung, is_child_and_has_mother,
    is_hungry_simple, parse_profession_token, pick_goal, pick_goal_ext, pick_goal_from_ladder,
    pick_goal_smith_craft, pick_smith_goal, resolve_priority_rung, sensors_from_simple,
    smith_product_targets, update_is_hungry, Goal, PriorityBand, PriorityRung, PrioritySensors,
    Profession, FARMER_TARGET_ID, HUNGRY_FOOD, HUNGRY_ENTER_FLOOR, HUNGRY_ENTER_FRAC,
    HUNGRY_LEAVE_FRAC, MIN_AGE_TO_EAT, SMITH_IRON_ID, SMITH_TARGET_ID, SMITHING_HAMMER_ID,
};"#;

/// True when AI-PRIO ladder symbols are already re-exported from lib.rs.
pub fn ai_prio_wired(lib_path: &PathBuf) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| {
            t.contains("resolve_priority_rung")
                && t.contains("PriorityRung")
                && t.contains("pick_goal_from_ladder")
                && t.contains("MIN_AGE_TO_EAT")
        })
        .unwrap_or(false)
}

/// Expand ai_goals pub use for priority ladder (idempotent).
pub fn patch_lib_ai_prio(lib_path: &PathBuf) -> bool {
    if ai_prio_wired(lib_path) {
        return true;
    }
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    if !text.contains(OLD_EXPORT) {
        // Already expanded under different formatting, or partial.
        return ai_prio_wired(lib_path);
    }
    text = text.replacen(OLD_EXPORT, NEW_EXPORT, 1);
    let out = restore_nl(&text, crlf);
    std::fs::write(lib_path, out).is_ok() && ai_prio_wired(lib_path)
}
