//! Best-food query for a player (Haxe `SearchBestFood` surface).
//! Part of [`crate::PlayerReadInterface`].

/// Default Chebyshev search radius in tiles (Haxe SearchBestFood default **40**).
pub const DEFAULT_FOOD_SEARCH_RADIUS: i32 = 40;

/// Parameters for [`FoodSearch::best_food`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestFoodQuery {
    pub conn_id: u64,
    /// Chebyshev radius; **default 40** ([`DEFAULT_FOOD_SEARCH_RADIUS`]).
    pub max_dist: i32,
}

impl BestFoodQuery {
    pub fn new(conn_id: u64) -> Self {
        Self {
            conn_id,
            max_dist: DEFAULT_FOOD_SEARCH_RADIUS,
        }
    }

    pub fn with_radius(conn_id: u64, max_dist: i32) -> Self {
        Self {
            conn_id,
            max_dist: max_dist.max(0),
        }
    }
}

impl Default for BestFoodQuery {
    fn default() -> Self {
        Self {
            conn_id: 0,
            max_dist: DEFAULT_FOOD_SEARCH_RADIUS,
        }
    }
}

/// One candidate food tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BestFoodHit {
    pub x: i32,
    pub y: i32,
    pub food_id: i32,
    pub score: f32,
    pub is_yum: bool,
}

/// Find the best edible for a player within a radius.
///
/// Implementors wrap live `search_best_food*` / content / yum state in `ol-sim`,
/// or a lighter nearby scan on the NPC thread (default r=40).
pub trait FoodSearch {
    fn best_food(&self, q: BestFoodQuery) -> Option<BestFoodHit>;

    /// Shorthand: default radius [`DEFAULT_FOOD_SEARCH_RADIUS`] (40).
    fn best_food_default(&self, conn_id: u64) -> Option<BestFoodHit> {
        self.best_food(BestFoodQuery::new(conn_id))
    }
}
