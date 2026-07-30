//! AI goals + priority ladder — re-exported from **`ol-ai-helper`**.
//!
//! Craft-aware profession expand (`pick_goal_smith_craft*`) from **`ol-ai-professions`**.
//! Prefer editing those crates; this file is a stable `crate::ai_goals` path for sim/server.

// Includes nested `priority_ladder` (`crate::ai_goals::priority_ladder`).
pub use ol_ai_helper::ai_goals::*;

// Was inlined here; moved to professions to avoid helper ↔ professions cycle.
pub use ol_ai_professions::{pick_goal_smith_craft, pick_goal_smith_craft_at_stage};
