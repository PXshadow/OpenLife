//! **AiCraftingHelper** — pure craft planning (no world mutation).
//!
//! Phase C: split from `ol-ai` so craft scoring recompiles without goals/path-reach.

#![forbid(unsafe_code)]

pub mod craft_graph;
pub mod craft_plan;
pub mod craft_value;

pub use craft_graph::ReverseCraftGraph;
pub use craft_value::{
    CraftOption, CraftProfession, NearbyObj, ABUNDANCE_SOFT_CAP, DEFAULT_CRAFT_RADIUS,
    DEFAULT_WALK_SPEED, INTERACTION_SEC,
};
