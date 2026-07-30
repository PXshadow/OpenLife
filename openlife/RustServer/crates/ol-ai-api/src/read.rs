//! **PlayerReadInterface** — fast read-only access for AI / tools.
//!
//! Not the client TCP path. Humans still learn the world via PU/MX from the sim.
//! AI (and diagnostics) use this for low-latency map/body/food queries.

use crate::food_search::{BestFoodHit, BestFoodQuery, FoodSearch, DEFAULT_FOOD_SEARCH_RADIUS};
use crate::player_view::PlayerView;
use crate::world_view::WorldView;

/// Bundle of read handles for one think tick (or tool query).
///
/// Prefer constructing this at the edge (ol-server NPC tick / sim adapter)
/// rather than threading `SimState` into pure AI crates.
#[derive(Clone, Copy)]
pub struct PlayerReadHandles<'a> {
    pub world: &'a dyn WorldView,
    pub player: &'a dyn PlayerView,
    pub food: &'a dyn FoodSearch,
}

impl<'a> PlayerReadHandles<'a> {
    pub fn new(
        world: &'a dyn WorldView,
        player: &'a dyn PlayerView,
        food: &'a dyn FoodSearch,
    ) -> Self {
        Self {
            world,
            player,
            food,
        }
    }

    /// Best food for the focal player at default radius **40**.
    pub fn best_food_default(&self) -> Option<BestFoodHit> {
        self.food.best_food_default(self.player.conn_id())
    }

    /// Best food for the focal player at an explicit Chebyshev radius.
    pub fn best_food_radius(&self, max_dist: i32) -> Option<BestFoodHit> {
        self.food
            .best_food(BestFoodQuery::with_radius(self.player.conn_id(), max_dist))
    }

    /// Explicit query (may target another conn if adapter allows).
    pub fn best_food(&self, q: BestFoodQuery) -> Option<BestFoodHit> {
        self.food.best_food(q)
    }

    pub fn default_food_radius() -> i32 {
        DEFAULT_FOOD_SEARCH_RADIUS
    }
}

/// Fast read-only façade: world + self body + food search.
///
/// Implement by holding (or borrowing) the three sub-interfaces.
/// Default methods forward to [`FoodSearch`] using the focal player's `conn_id`.
pub trait PlayerReadInterface {
    fn world(&self) -> &dyn WorldView;
    fn self_player(&self) -> &dyn PlayerView;
    fn food_search(&self) -> &dyn FoodSearch;

    fn as_handles(&self) -> PlayerReadHandles<'_> {
        PlayerReadHandles::new(self.world(), self.self_player(), self.food_search())
    }

    /// Best food for this body, default radius **40**.
    fn best_food_default(&self) -> Option<BestFoodHit> {
        self.as_handles().best_food_default()
    }

    /// Best food for this body at `max_dist` (Chebyshev).
    fn best_food_radius(&self, max_dist: i32) -> Option<BestFoodHit> {
        self.as_handles().best_food_radius(max_dist)
    }
}
