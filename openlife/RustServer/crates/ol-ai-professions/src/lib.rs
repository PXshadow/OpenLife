//! **AI professions** — pure profession state machines and craft goal expansion.
//!
//! Depends on [`ol_ai_helper`] (Goal / ladder types) and [`ol_ai_crafting`].
//! Does **not** mutate the world; live profession scan remains in `ol-sim` until
//! MainAI absorbs it.

#![forbid(unsafe_code)]

pub mod baker_profession;
pub mod farmer_profession;
pub mod fire_food_profession;
pub mod fire_food_rung;
pub mod goal_expand;
pub mod pottery_profession;
pub mod professions;
pub mod shepherd_profession;
pub mod smith_profession;

pub use goal_expand::{pick_goal_smith_craft, pick_goal_smith_craft_at_stage};
pub use professions::{
    is_chop_biome, is_fishing_biome, is_grassland, is_mountain_biome, is_swamp, ProfActionResult,
    GRASSLAND_BIOME, PROF_ACTION_COOLDOWN_SECS,
};
