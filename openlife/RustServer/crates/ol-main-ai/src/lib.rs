//! **MainAI** — high-level AI decisions over shared player interfaces.
//!
//! ## Rules
//!
//! - **Reads:** [`ol_ai_api::FoodSearch`] / [`PlayerReadInterface`] only
//! - **Writes:** [`ol_ai_api::PlayerWriteInterface`] → `NetIntent` (same as humans)
//! - No direct `World` / `SimState` mutation
//!
//! Phase D extracts seek-food / hungry planning first; profession ladders stay
//! in the server NPC loop until a later rung.

#![forbid(unsafe_code)]

mod plan;
mod seek_food;

pub use plan::{apply_plan, ThinkPlan, ThinkSensors};
pub use seek_food::{is_hungry_for_food_seek, plan_hungry_food, plan_hungry_food_from_read};

/// Package version of the default food search radius (re-export for callers).
pub use ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS;
