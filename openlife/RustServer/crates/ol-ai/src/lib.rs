//! Open Life AI — pure decisions (no `SimState` mutation).
//!
//! ## Interfaces ([`ol_ai_api`])
//!
//! - **Write:** [`PlayerWriteInterface`] → [`ol_net::NetIntent`] (same as human clients)
//! - **Read:** [`PlayerReadInterface`] / [`WorldView`] / [`PlayerView`] / [`FoodSearch`]
//!   (default best-food radius **30**)
//!
//! This crate holds pure craft/goals/profession modules. Traits live in **`ol-ai-api`**
//! so later MainAI / helpers can depend on a tiny API without pulling pure AI bulk.
//!
//! Live world scans and `apply_intent` stay in `ol-sim` / `ol-server`.

#![forbid(unsafe_code)]

// ── Interfaces re-exported from ol-ai-api (Phase A) ─────────────────────────
pub use ol_ai_api::{
    chebyshev, BestFoodHit, BestFoodQuery, CommandSink, FoodSearch, IntentSink, PlayerCommands,
    PlayerReadHandles, PlayerReadInterface, PlayerView, PlayerWriteInterface, WorldView,
    DEFAULT_FOOD_SEARCH_RADIUS,
};

// ── Pure AI decision modules (moved from ol-sim) ───────────────────────────
pub mod craft_graph;
pub mod craft_plan;
pub mod craft_value;
pub mod ai_path_reach;
pub mod professions;
pub mod ai_goals;
// Profession pure SMs (depend on ai_goals + craft_graph only)
pub mod smith_profession;
pub mod farmer_profession;
pub mod baker_profession;
pub mod fire_food_profession;
pub mod pottery_profession;
pub mod shepherd_profession;

// Convenient re-exports (stable API for ol-sim / ol-server)
pub use ai_goals::{
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
    CloseDeadlyPlayer, ClosePlayerTarget, DeadlyPlayerCandidate, EscapeContext, EscapeSideEffects,
    EscapeThreat, Goal, HungryEatEffects, LiveSensorBundle, LiveSensorExtras, LiveSensorInput,
    PlayerTargetCandidate, PriorityBand, PriorityRung, PrioritySensors, Profession,
    BLUE_MASK_HOME_QUAD_MAX, DEADLY_PLAYER_ANGRY_ACTIVE, DEADLY_PLAYER_SEARCH_DIST,
    DEADLY_PLAYER_SEARCH_DIST_AI, DEVIL_MASK_ID, ESCAPE_ANGRY_TIME_IGNORE,
    ESCAPE_DID_NOT_REACH_FOOD_MAX, ESCAPE_DIST, ESCAPE_FOOD_CRIT_SKIP, ESCAPE_HUNT_MIN_AGE,
    ESCAPE_PLAYER_DIST_MAX, EXILE_HOME_QUAD_DANGER, GOBLIN_MASK_ID, HUNGRY_ENTER_FLOOR,
    HUNGRY_ENTER_FRAC, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED, MIN_AGE_TO_EAT,
    PLAYER_TARGET_SEARCH_DIST, SMITHING_HAMMER_ID,
};
pub use ai_path_reach::{
    AiPathReachMaps, AiStickyBlockTargets, StickyFoodTarget, BLOCKED_BY_AI_DEFAULT_SECS,
    HOSTILE_PATH_DEFAULT_SECS, NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS,
};
pub use craft_graph::ReverseCraftGraph;
pub use craft_value::{
    CraftOption, CraftProfession, NearbyObj, ABUNDANCE_SOFT_CAP, DEFAULT_CRAFT_RADIUS,
    DEFAULT_WALK_SPEED, INTERACTION_SEC,
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
    fn reexport_write_interface() {
        let mut sink = VecSink(Vec::new());
        sink.use_at(7, 1, 2, Some(100), None);
        assert_eq!(sink.0.len(), 1);
    }

    #[test]
    fn food_query_default_radius_30() {
        let q = BestFoodQuery::new(1);
        assert_eq!(q.max_dist, DEFAULT_FOOD_SEARCH_RADIUS);
        assert_eq!(q.max_dist, 30);
    }

    #[test]
    fn craft_graph_roundtrip_in_ol_ai() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        assert!(g.ingredients_for(3).is_some());
    }
}
