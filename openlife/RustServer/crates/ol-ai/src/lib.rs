//! Open Life AI façade — re-exports helper / pathing / crafting / professions / interfaces.
//!
//! ## Layout
//!
//! | Crate | Role |
//! |-------|------|
//! | [`ol_ai_api`] | `PlayerWriteInterface` / `PlayerReadInterface` |
//! | [`ol_ai_helper`] | Goals, priority ladder |
//! | [`ol_ai_pathing`] | Path-reach / blockedByAI maps |
//! | [`ol_ai_crafting`] | Craft graph / plan / value |
//! | [`ol_ai_professions`] | Profession pure SMs + craft goal expand |
//! | [`ol_main_ai`] | High-level ThinkPlan (separate crate) |
//! | **this crate** | Stable `ol_ai::*` path for server + sim |

#![forbid(unsafe_code)]

// ── Interfaces ──────────────────────────────────────────────────────────────
pub use ol_ai_api::{
    chebyshev, BestFoodHit, BestFoodQuery, CommandSink, FoodSearch, IntentSink, PlayerCommands,
    PlayerReadHandles, PlayerReadInterface, PlayerView, PlayerWriteInterface, WorldView,
    DEFAULT_FOOD_SEARCH_RADIUS,
};

// ── AiHelper (goals / ladder) ───────────────────────────────────────────────
pub use ol_ai_helper::ai_goals;
pub use ol_ai_helper::{
    age_job_index, age_rotated_job_kind, age_rotated_job_sequence, apply_escape_to_sensors,
    auto_follow_ai_human_early_return, baby_hungry_follow_tiles, check_is_hungry_and_eat_effects,
    child_with_mother_follow_tiles, count_seeds_from_counts, decay_was_idle,
    craving_item_to_craft, handle_temperature_after_pickup, hot_kiln_pottery_gate,
    hungry_infant_always_returns, idle_say_line, idle_should_drop_held,
    search_food_and_eat_debug_say, should_debug_profession, should_debug_say,
    nice_baby_noble_wants_weapon, roll_is_nice_baby, should_escape_on_moved_one_tile,
    should_switch_cloth, should_zero_profession_weight, skip_home_and_jobs_while_moving,
    switch_cloth_resolve_parent, in_get_close_clothings_square, is_get_close_clothing_object,
    tick_waiting_time, waiting_time_blocks_think,
    compute_do_stuff, effective_do_stuff, escape_candidate_xy, escape_context_from_threats,
    escape_debug_say, escape_dir_flip, escape_maybe_assign_animal_target, escape_side_effects,
    escape_target_xy, fill_live_sensors, get_close_deadly_player, get_close_player_target,
    goal_from_rung, is_child_and_has_mother, is_child_and_has_mother_ex,
    is_child_and_has_mother_from_follow, is_deadly_player_candidate,
    is_hungry_simple, is_moving_to_player_needed, is_superbad_temp, ordered_follow_max_tiles,
    pick_escape_tile, pick_goal, pick_goal_ext, pick_goal_with_biome, pick_smith_goal,
    player_quad_dist, resolve_escape_threat, resolve_priority_rung, sensors_from_ext,
    sensors_from_ext_ex,
    sensors_from_simple, should_attempt_escape, skip_escape_for_hunt, smith_product_targets,
    threat_is_far_for_temp, threat_quad_from_deadly, update_is_hungry, wounded_follow_tiles,
    AgeRotatedJobKind, CloseDeadlyPlayer, ClosePlayerTarget, CountSeedsFlags,
    DeadlyPlayerCandidate, EscapeContext, EscapePick, EscapeRand,
    EscapeSideEffects, EscapeThreat, Goal, HungryEatEffects, LiveSensorBundle, LiveSensorExtras,
    LiveSensorInput, PlayerTargetCandidate, PriorityBand, PriorityRung, PrioritySensors,
    Profession, BACKPACK_CLOTH_ID, BLUE_MASK_HOME_QUAD_MAX, COUNT_SEEDS_CARROT_IDS,
    COUNT_SEEDS_CORN_IDS, DEADLY_PLAYER_ANGRY_ACTIVE, DEADLY_PLAYER_SEARCH_DIST,
    DEADLY_PLAYER_SEARCH_DIST_AI, DEVIL_MASK_ID, ESCAPE_ANGRY_TIME_IGNORE,
    ESCAPE_DID_NOT_REACH_FOOD_MAX, ESCAPE_DIR_RAND_LOWER, ESCAPE_DIR_RAND_UPPER, ESCAPE_DIST,
    ESCAPE_FOOD_CRIT_SKIP, ESCAPE_HUNT_MIN_AGE, ESCAPE_IS_DANGEROUS_RADIUS, ESCAPE_PLAYER_DIST_MAX,
    ESCAPE_RETRY_INNER, ESCAPE_RETRY_OUTER, EXILE_HOME_QUAD_DANGER, GOBLIN_MASK_ID,
    HUNGRY_ENTER_FLOOR,
    HUNGRY_ENTER_FRAC, HUNGRY_FOOD, HUNGRY_LEAVE_FRAC, KNIFE_ID, MAX_CHILD_AGE_BREASTFEED,
    CLOTHS_WITH_PILES_IDS, GET_CLOSE_CLOTHINGS_RADIUS, MIN_AGE_TO_EAT, MOUFLON_HIDE_ID,
    PILE_MOUFLON_HIDES_ID, PILE_SHEEP_SKINS_ID,
    PLAYER_TARGET_SEARCH_DIST, SHEEP_SKIN_CLOTH_ID, SMITHING_HAMMER_ID, SMITH_IRON_ID,
    SMITH_TARGET_ID, WAR_SWORD_ID,
};

// ── Pathing (path-reach / blockedByAI / PathfinderNew) ──────────────────────
pub use ol_ai_pathing::ai_path_reach;
pub use ol_ai_pathing::pathfinder_new;
pub use ol_ai_pathing::{
    create_path as pathfinder_new_create_path,
    create_path_with_budget as pathfinder_new_create_path_with_budget,
    path_to_steps as pathfinder_new_path_to_steps, AiPathReachMaps, AiStickyBlockTargets,
    PathBudget, StickyFoodTarget, BLOCKED_BY_AI_DEFAULT_SECS, HOSTILE_PATH_DEFAULT_SECS,
    NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS, PATHFINDER_NEW_DEFAULT_RADIUS,
    PATHFINDER_NEW_TIMEOUT_MS,
};

// ── Crafting ────────────────────────────────────────────────────────────────
pub use ol_ai_crafting::craft_graph;
pub use ol_ai_crafting::craft_plan;
pub use ol_ai_crafting::craft_value;
pub use ol_ai_crafting::{
    CraftOption, CraftProfession, NearbyObj, ReverseCraftGraph, ABUNDANCE_SOFT_CAP,
    DEFAULT_CRAFT_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC,
};

// ── Professions ─────────────────────────────────────────────────────────────
pub use ol_ai_professions::baker_profession;
pub use ol_ai_professions::farmer_profession;
pub use ol_ai_professions::fire_food_profession;
pub use ol_ai_professions::fire_food_rung;
pub use ol_ai_professions::pottery_profession;
pub use ol_ai_professions::cleanup_profession;
pub use ol_ai_professions::professions;
pub use ol_ai_professions::shepherd_profession;
pub use ol_ai_professions::smith_profession;
pub use ol_ai_professions::{
    is_fishing_biome, is_grassland, pick_goal_smith_craft, pick_goal_smith_craft_at_stage,
};

#[cfg(test)]
mod tests {
    use super::*;
    use ol_net::NetIntent;

    struct VecSink(Vec<NetIntent>);
    impl CommandSink for VecSink {
        fn push(&mut self, intent: NetIntent) -> bool {
            self.0.push(intent);
            true
        }
    }

    #[test]
    fn facade_write_and_craft_graph() {
        let mut sink = VecSink(Vec::new());
        sink.use_at(1, 0, 0, None, None);
        assert_eq!(sink.0.len(), 1);
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        assert!(g.ingredients_for(3).is_some());
    }

    #[test]
    fn food_radius_40() {
        assert_eq!(DEFAULT_FOOD_SEARCH_RADIUS, 40);
    }

    #[test]
    fn path_reach_from_pathing_crate() {
        let mut m = AiPathReachMaps::new();
        m.add_not_reachable(1, 2, 0.0);
        assert!(!m.is_empty());
    }
}
