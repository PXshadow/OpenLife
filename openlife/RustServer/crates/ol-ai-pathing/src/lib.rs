//! **AI pathing** — pure path-reach / blocked-by-AI maps (Haxe PATH-REACH / BLOCKED-BY-AI).
//!
//! No world I/O. Live apply lives in `ol-sim` / `ol-server`; this crate is the pure
//! timers, fail-marks, sticky food/goto claims, and `CalculateBlockedByAi` rebuild.
//!
//! Re-exported as `ol_ai::ai_path_reach` and `ol_ai_helper::ai_path_reach` for stable paths.

#![forbid(unsafe_code)]

pub mod ai_path_reach;

pub use ai_path_reach::{
    add_agent_to_blocked_by_ai, add_blocked_by_ai, apply_calculate_blocked_by_ai,
    apply_food_action_fail, apply_food_goto_fail, blocked_by_animal_from_dual_pass,
    blocked_coords_from_live, calculate_blocked_by_ai, cleanup_blocked_by_ai,
    consider_animals_for_goto, food_action_fail_effects, food_goto_fail_effects,
    food_pickup_success_reset_did_not_reach, goto_quad_distance, is_empty_hand_food_use_fail,
    is_food_action_fail_at, mark_food_path_fail, mark_food_pickup_action_fail_on_maps,
    mark_goto_path_fail, mark_not_reachable_on_player, mark_use_or_food_path_fail, mark_use_path_fail,
    merge_path_reach_maps, pending_food_tile_still_actionable, plan_goto_obj,
    preserve_view_path_reach_on_publish, receding_goto_should_abort, resolve_sticky_food,
    settle_pending_food_use_fail, sticky_food_still_valid, sync_path_reach_bidirectional,
    try_add_target_blocked_by_ai, try_mark_food_action_fail_on_maps, would_block_target_by_ai,
    AddTargetBlockResult, AiAgentBlockSource, AiPathReachMaps, AiStickyBlockTargets,
    BlockTargetClaim, FoodActionFailEffects, FoodGotoFailEffects, GotoObjPlan, HumanBlockClaim,
    LastGotoObj, StickyBlockIntentKind, StickyFoodTarget, BLOCKED_BY_AI_DEFAULT_SECS,
    BLOCK_BY_AI_MIN_AGE, BLOCK_TARGET_MAX_AGE_SECS, DONT_BLOCK_BY_AI, HOSTILE_PATH_DEFAULT_SECS,
    NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS, RECEDING_GOTO_DIST_QUAD,
    SMITHING_HAMMER_BLOCK_ID,
};
