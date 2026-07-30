//! **AiHelper** — pure AI decision utilities (goals, ladder).
//!
//! Path-reach maps live in **`ol-ai-pathing`** (re-exported here for stable paths).
//! Profession state machines live in **`ol-ai-professions`** (no reverse dep).

#![forbid(unsafe_code)]

pub mod ai_goals;
/// Path-reach / blocked-by-AI maps (owned by `ol-ai-pathing`).
pub use ol_ai_pathing::ai_path_reach;

pub use ai_goals::{
    age_job_index, age_rotated_job_kind, age_rotated_job_sequence, apply_escape_to_sensors,
    baby_hungry_follow_tiles, check_is_hungry_and_eat_effects, child_with_mother_follow_tiles,
    compute_do_stuff, effective_do_stuff, escape_context_from_threats, escape_side_effects,
    escape_target_xy, fill_live_sensors, get_close_deadly_player, get_close_player_target,
    goal_from_rung, is_child_and_has_mother, is_child_and_has_mother_ex, is_deadly_player_candidate,
    is_hungry_simple, is_moving_to_player_needed, is_superbad_temp, ordered_follow_max_tiles,
    pick_goal, pick_goal_ext, pick_goal_with_biome, pick_smith_goal, player_quad_dist,
    resolve_escape_threat, resolve_priority_rung, sensors_from_ext, sensors_from_ext_ex,
    sensors_from_simple, should_attempt_escape, skip_escape_for_hunt, smith_product_targets,
    threat_is_far_for_temp, threat_quad_from_deadly, update_is_hungry, wounded_follow_tiles,
    AgeRotatedJobKind, CloseDeadlyPlayer, ClosePlayerTarget, DeadlyPlayerCandidate, EscapeContext,
    EscapeSideEffects, EscapeThreat, Goal, HungryEatEffects, LiveSensorBundle, LiveSensorExtras,
    LiveSensorInput, PlayerTargetCandidate, PriorityBand, PriorityRung, PrioritySensors,
    Profession, BLUE_MASK_HOME_QUAD_MAX, DEADLY_PLAYER_ANGRY_ACTIVE, DEADLY_PLAYER_SEARCH_DIST,
    DEADLY_PLAYER_SEARCH_DIST_AI, DEVIL_MASK_ID, ESCAPE_ANGRY_TIME_IGNORE,
    ESCAPE_DID_NOT_REACH_FOOD_MAX, ESCAPE_DIST, ESCAPE_FOOD_CRIT_SKIP, ESCAPE_HUNT_MIN_AGE,
    ESCAPE_PLAYER_DIST_MAX, EXILE_HOME_QUAD_DANGER, GOBLIN_MASK_ID, HUNGRY_ENTER_FLOOR,
    HUNGRY_ENTER_FRAC, HUNGRY_FOOD, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED, MIN_AGE_TO_EAT,
    PLAYER_TARGET_SEARCH_DIST, SMITHING_HAMMER_ID, SMITH_IRON_ID, SMITH_TARGET_ID,
};
pub use ol_ai_pathing::{
    AiPathReachMaps, AiStickyBlockTargets, StickyFoodTarget, BLOCKED_BY_AI_DEFAULT_SECS,
    HOSTILE_PATH_DEFAULT_SECS, NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS,
};
