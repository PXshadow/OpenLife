//! Open Life AI façade — re-exports helper / crafting / write-read interfaces.
//!
//! ## Phase C layout
//!
//! | Crate | Role |
//! |-------|------|
//! | [`ol_ai_api`] | `PlayerWriteInterface` / `PlayerReadInterface` |
//! | [`ol_ai_helper`] | Goals, ladder, path-reach, profession pure SMs |
//! | [`ol_ai_crafting`] | Craft graph / plan / value |
//! | **this crate** | Stable `ol_ai::*` path for server + sim |
//!
//! Live world I/O stays in `ol-sim` / `ol-server`.

#![forbid(unsafe_code)]

// ── Interfaces ──────────────────────────────────────────────────────────────
pub use ol_ai_api::{
    chebyshev, BestFoodHit, BestFoodQuery, CommandSink, FoodSearch, IntentSink, PlayerCommands,
    PlayerReadHandles, PlayerReadInterface, PlayerView, PlayerWriteInterface, WorldView,
    DEFAULT_FOOD_SEARCH_RADIUS,
};

// ── AiHelper ────────────────────────────────────────────────────────────────
pub use ol_ai_helper::ai_goals;
pub use ol_ai_helper::ai_path_reach;
pub use ol_ai_helper::professions;
pub use ol_ai_helper::{
    baker_profession, farmer_profession, fire_food_profession, fire_food_rung, pottery_profession,
    shepherd_profession, smith_profession,
};
pub use ol_ai_helper::{
    age_job_index, age_rotated_job_kind, age_rotated_job_sequence, apply_escape_to_sensors,
    baby_hungry_follow_tiles, check_is_hungry_and_eat_effects, child_with_mother_follow_tiles,
    compute_do_stuff, effective_do_stuff, escape_context_from_threats, escape_side_effects,
    escape_target_xy, fill_live_sensors, get_close_deadly_player, get_close_player_target,
    goal_from_rung, is_child_and_has_mother, is_child_and_has_mother_ex, is_deadly_player_candidate,
    is_hungry_simple, is_moving_to_player_needed, is_superbad_temp, ordered_follow_max_tiles,
    pick_goal_from_ladder, pick_goal_from_live_sensors, pick_goal_with_sensors, player_quad_dist,
    resolve_escape_threat, resolve_priority_rung, sensors_from_ext, sensors_from_ext_ex,
    sensors_from_simple, should_attempt_escape, skip_escape_for_hunt, threat_is_far_for_temp,
    threat_quad_from_deadly, update_is_hungry, wounded_follow_tiles, AgeRotatedJobKind,
    AiPathReachMaps, AiStickyBlockTargets, CloseDeadlyPlayer, ClosePlayerTarget,
    DeadlyPlayerCandidate, EscapeContext, EscapeSideEffects, EscapeThreat, Goal, HungryEatEffects,
    LiveSensorBundle, LiveSensorExtras, LiveSensorInput, PlayerTargetCandidate, PriorityBand,
    PriorityRung, PrioritySensors, Profession, StickyFoodTarget, BLUE_MASK_HOME_QUAD_MAX,
    BLOCKED_BY_AI_DEFAULT_SECS, DEADLY_PLAYER_ANGRY_ACTIVE, DEADLY_PLAYER_SEARCH_DIST,
    DEADLY_PLAYER_SEARCH_DIST_AI, DEVIL_MASK_ID, ESCAPE_ANGRY_TIME_IGNORE,
    ESCAPE_DID_NOT_REACH_FOOD_MAX, ESCAPE_DIST, ESCAPE_FOOD_CRIT_SKIP, ESCAPE_HUNT_MIN_AGE,
    ESCAPE_PLAYER_DIST_MAX, EXILE_HOME_QUAD_DANGER, GOBLIN_MASK_ID, HOSTILE_PATH_DEFAULT_SECS,
    HUNGRY_ENTER_FLOOR, HUNGRY_ENTER_FRAC, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED,
    MIN_AGE_TO_EAT, NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS, PLAYER_TARGET_SEARCH_DIST,
    SMITHING_HAMMER_ID,
};

// ── AiCraftingHelper ────────────────────────────────────────────────────────
pub use ol_ai_crafting::craft_graph;
pub use ol_ai_crafting::craft_plan;
pub use ol_ai_crafting::craft_value;
pub use ol_ai_crafting::{
    CraftOption, CraftProfession, NearbyObj, ReverseCraftGraph, ABUNDANCE_SOFT_CAP,
    DEFAULT_CRAFT_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC,
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
}
