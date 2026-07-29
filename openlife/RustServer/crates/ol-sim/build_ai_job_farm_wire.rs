//! AI-JOB-FARM-WIRE / farm_spatial: splice CountCloseObjects fill into farmer_profession + lib exports (idempotent).

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

/// True when farm spatial symbols are re-exported from lib.rs.
pub fn farm_wire_wired(lib_path: &PathBuf) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| {
            t.contains("fill_farm_counts_from_map")
                && t.contains("try_decide_farm_from_rung")
                && t.contains("FarmMapObj")
        })
        .unwrap_or(false)
}

const FARM_USE_OLD: &str = r#"// Haxe: AiBase farmer profession (AI-JOB-FARM)
pub use farmer_profession::{
    age_rotated_farm_profession, assigned_job_farm_profession, decide_farm_job,
    default_wet_from_bowl, do_basic_farming, do_berry_farming, do_carrot_farming, do_composting,
    do_harvest_corn, do_harvest_wheat, do_plant, do_plant_bushes, do_plant_carrots, do_plant_corn,
    do_plant_wheat, do_prepare_rows, do_prepare_soil, do_watering_on, has_or_become_profession,
    parse_farm_profession_speech, pick_farmer_goal, resolve_farm_assigned_job, FarmAction,
    FarmCounts, FarmProfession, FarmProfessionRuntime, FarmTaskState, ADVANCED_PLANTS,
    BOWL_OF_WATER, DRY_PLANTED_CARROTS, DRY_PLANTED_WHEAT, FARM_HOME_RADIUS, HARVESTED_WHEAT,
    RIPE_WHEAT, THRESHED_WHEAT, WHEAT_SHEAF, WET_PLANTED_CARROTS,
};
"#;

const FARM_USE_NEW: &str = r#"// Haxe: AiBase farmer profession (AI-JOB-FARM / AI-JOB-FARM-WIRE farm_spatial)
pub use farmer_profession::{
    age_rotated_farm_profession, assigned_job_farm_profession, count_close_objects_at,
    count_close_objects_ex, count_close_objects_with_piles, count_close_pile_specials,
    count_corn_seeds_near, count_with_held, decide_farm_job, default_wet_from_bowl,
    do_basic_farming, do_berry_farming, do_carrot_farming, do_composting, do_harvest_corn,
    do_harvest_wheat, do_plant, do_plant_bushes, do_plant_carrots, do_plant_corn, do_plant_wheat,
    do_prepare_rows, do_prepare_soil, do_watering_on, farm_action_to_goal, farm_chebyshev,
    farm_counts_from_nearby, farm_goal_from_counts_and_rung, farm_goal_from_map_and_rung,
    farm_job_for_age_label, farm_job_rung_label, farm_max_people_for_dispatch, farm_radius_table,
    fill_farm_counts_from_map, fill_farm_counts_from_map_ex, fill_farm_counts_from_map_with_floor,
    has_or_become_profession, in_count_close_square, is_ignored_floor, parse_farm_profession_speech,
    pile_obj_id_from_table, pick_farmer_goal, resolve_farm_assigned_job, soil_units_from_map,
    try_decide_farm_from_rung, CountCloseOpts, FarmAction, FarmCounts, FarmMapObj, FarmProfession,
    FarmProfessionRuntime, FarmTaskState, ADVANCED_PLANTS, AI_IGNORED_FLOOR_IDS,
    BIG_CHARCOAL_PILE_ID, BOWL_OF_WATER, CORN_SEED_COUNT_RADIUS, DRY_PLANTED_CARROTS,
    DRY_PLANTED_WHEAT, FARM_COUNT_RADIUS, FARM_HOME_RADIUS, FARM_ROW_SHORTCRAFT_RADIUS,
    FARM_SHORTCRAFT_RADIUS, HARVESTED_WHEAT, HUGE_CHARCOAL_PILE_ID, RIPE_WHEAT, THRESHED_WHEAT,
    WET_CLAY_BOWL_ID, WHEAT_SHEAF, WET_PLANTED_CARROTS,
};
"#;

const PRIO_OLD: &str = r#"            Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
            Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
            // Thin default; live tick prefers `try_decide_baker_from_rung` + bake_action_to_goal
            // Haxe: AssignedJob BAKER → doBaking(100); AgeRotated Baking → doBaking()
            Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
"#;

const PRIO_NEW: &str = r#"            // Thin default; live tick prefers `try_decide_farm_from_rung` + farm_action_to_goal
            // + fill_farm_counts_from_map (AI-JOB-FARM-WIRE)
            Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
            Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
            // Thin default; live tick prefers `try_decide_baker_from_rung` + bake_action_to_goal
            // Haxe: AssignedJob BAKER → doBaking(100); AgeRotated Baking → doBaking()
            Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
"#;

/// Splice `include!("farm_spatial_inc.rs")` + spatial tests into farmer_profession if missing.
pub fn patch_farmer_spatial_include(src: &PathBuf) -> bool {
    let farm_path = src.join("farmer_profession.rs");
    let inc_path = src.join("farm_spatial_inc.rs");
    let tail_path = src.join("_farmer_tail_wire.rs");
    if !inc_path.exists() {
        return false;
    }
    let Ok(raw) = std::fs::read_to_string(&farm_path) else {
        return false;
    };
    if raw.contains("include!(\"farm_spatial_inc.rs\")")
        || raw.contains("include!(\"farm_spatial_inc.rs\");")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let t = normalize_nl(&raw);
    let marker = "/// Reverse-craft goal for farmer (like smith iron expansion).";
    let Some(idx) = t.find(marker) else {
        return false;
    };
    let tail = if tail_path.exists() {
        normalize_nl(&std::fs::read_to_string(&tail_path).unwrap_or_default())
    } else {
        // Minimal splice: just include before tests
        let head_rest = &t[idx..];
        let tests_marker = "\n// ── Tests ──────────────────────────────────────────────────────────────────\n";
        if let Some(tidx) = head_rest.find(tests_marker) {
            let mut out = String::new();
            out.push_str(&t[..idx]);
            out.push_str(&head_rest[..tidx]);
            out.push_str("\n// Haxe: AiHelper.CountCloseObjects farm spatial (AI-JOB-FARM-WIRE / farm_spatial)\n");
            out.push_str("include!(\"farm_spatial_inc.rs\");\n");
            out.push_str(&head_rest[tidx..]);
            let _ = std::fs::write(&farm_path, restore_nl(&out, crlf));
            return true;
        }
        return false;
    };
    if tail.is_empty() {
        return false;
    }
    let mut out = String::new();
    out.push_str(&t[..idx]);
    out.push_str(&tail);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    let _ = std::fs::write(&farm_path, restore_nl(&out, crlf));
    true
}

/// Update farmer_profession module doc for farm_spatial residual note.
pub fn patch_farmer_doc(farm_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(farm_path) else {
        return false;
    };
    if raw.contains("AI-JOB-FARM-WIRE") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let old = r#"//! Haxe: `AiBase` farmer profession family (chunk **AI-JOB-FARM**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession` / sticky `lastProfession`
//! - Speech `FARMER!` / `WHEAT!` / `CARROT!` → assigned job
//! - `doPlant` / harvest / water / soil / rows / compost hysteresis
//! - Job sequences: basic / carrot / berry / advanced farming
//!
//! No world I/O: callers supply counts and apply returned [`FarmAction`]s
//! via craft/shortCraft (AI-CRAFT) and spatial helpers (AiHelper port).
"#;
    let new = r#"//! Haxe: `AiBase` farmer profession family (chunk **AI-JOB-FARM** / **AI-JOB-FARM-WIRE**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession` / sticky `lastProfession`
//! - Speech `FARMER!` / `WHEAT!` / `CARROT!` → assigned job
//! - `doPlant` / harvest / water / soil / rows / compost hysteresis
//! - Job sequences: basic / carrot / berry / advanced farming
//! - Spatial CountCloseObjects fill (`farm_spatial_inc` / [`fill_farm_counts_from_map`])
//! - Ladder bridge [`try_decide_farm_from_rung`] + [`farm_action_to_goal`]
//!
//! No world I/O: callers supply counts / map snapshots and apply returned [`FarmAction`]s
//! via craft/shortCraft (AI-CRAFT). Residual: keepBushesAlive body, sticky Player FarmTaskState.
"#;
    let t = normalize_nl(&raw);
    if t.contains(old) {
        let t = t.replacen(old, new, 1);
        let _ = std::fs::write(farm_path, restore_nl(&t, crlf));
        return true;
    }
    false
}

/// Idempotent lib.rs export wire for farm spatial symbols via farmer_profession.
pub fn patch_lib_ai_job_farm_wire(lib_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if farm_wire_wired(lib_path) {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if t.contains(FARM_USE_OLD) {
        t = t.replacen(FARM_USE_OLD, FARM_USE_NEW, 1);
        changed = true;
    } else if t.contains("pub use farmer_profession::{") && !t.contains("fill_farm_counts_from_map") {
        // Expand existing export block: inject new symbols after pick_farmer_goal line
        if let Some(idx) = t.find("parse_farm_profession_speech, pick_farmer_goal, resolve_farm_assigned_job, FarmAction,")
        {
            t = t.replacen(
                "parse_farm_profession_speech, pick_farmer_goal, resolve_farm_assigned_job, FarmAction,\n    FarmCounts, FarmProfession, FarmProfessionRuntime, FarmTaskState, ADVANCED_PLANTS,\n    BOWL_OF_WATER, DRY_PLANTED_CARROTS, DRY_PLANTED_WHEAT, FARM_HOME_RADIUS, HARVESTED_WHEAT,\n    RIPE_WHEAT, THRESHED_WHEAT, WHEAT_SHEAF, WET_PLANTED_CARROTS,\n};",
                "parse_farm_profession_speech, pick_farmer_goal, resolve_farm_assigned_job,\n    count_close_objects_at, count_corn_seeds_near, count_with_held, farm_action_to_goal,\n    farm_chebyshev, farm_counts_from_nearby, farm_job_for_age_label, farm_job_rung_label,\n    farm_max_people_for_dispatch, farm_radius_table, fill_farm_counts_from_map,\n    fill_farm_counts_from_map_ex, soil_units_from_map, try_decide_farm_from_rung,\n    FarmAction, FarmCounts, FarmMapObj, FarmProfession, FarmProfessionRuntime, FarmTaskState,\n    ADVANCED_PLANTS, BOWL_OF_WATER, CORN_SEED_COUNT_RADIUS, DRY_PLANTED_CARROTS, DRY_PLANTED_WHEAT,\n    FARM_COUNT_RADIUS, FARM_HOME_RADIUS, FARM_ROW_SHORTCRAFT_RADIUS, FARM_SHORTCRAFT_RADIUS,\n    HARVESTED_WHEAT, RIPE_WHEAT, THRESHED_WHEAT, WHEAT_SHEAF, WET_PLANTED_CARROTS,\n};",
                1,
            );
            let _ = idx;
            changed = t.contains("fill_farm_counts_from_map");
        }
    }

    if changed {
        let _ = std::fs::write(lib_path, restore_nl(&t, crlf));
    }
    farm_wire_wired(lib_path)
}

/// Comment on priority ladder Farmer job band → farm_spatial ladder bridge.
pub fn patch_priority_ladder_farm(prio_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(prio_path) else {
        return false;
    };
    if raw.contains("try_decide_farm_from_rung") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    if t.contains(PRIO_OLD) {
        t = t.replacen(PRIO_OLD, PRIO_NEW, 1);
        let _ = std::fs::write(prio_path, restore_nl(&t, crlf));
        return true;
    }
    false
}

/// Full wire: include + doc + lib exports + ladder comment.
///
/// `prio_path` is derived as `src/priority_ladder.rs` when using the 2-arg form.
pub fn patch_all_farm_wire(src: &PathBuf, lib_path: &PathBuf) -> bool {
    let prio_path = src.join("priority_ladder.rs");
    let a = patch_farmer_spatial_include(src);
    let b = patch_farmer_doc(&src.join("farmer_profession.rs"));
    let c = patch_lib_ai_job_farm_wire(lib_path);
    let d = patch_priority_ladder_farm(&prio_path);
    a || b || c || d || farm_wire_wired(lib_path)
}

/// 3-arg form kept for call sites that pass an explicit ladder path.
pub fn patch_all_farm_wire_with_prio(
    src: &PathBuf,
    lib_path: &PathBuf,
    prio_path: &PathBuf,
) -> bool {
    let a = patch_farmer_spatial_include(src);
    let b = patch_farmer_doc(&src.join("farmer_profession.rs"));
    let c = patch_lib_ai_job_farm_wire(lib_path);
    let d = patch_priority_ladder_farm(prio_path);
    a || b || c || d || farm_wire_wired(lib_path)
}
