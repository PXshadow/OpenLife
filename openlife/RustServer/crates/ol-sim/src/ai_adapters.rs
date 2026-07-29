//! Adapters: `ol-ai` traits ← live sim/world (OL-AI-SPLIT Phase 2).
//!
//! AI decisions stay in `ol-ai` (or npc_ai using traits). This module only
//! implements read/search surfaces so AI never needs a second mutation path.

use crate::player::PlayerSnapshot;
use crate::search_best_food::AiFoodSearchFlags;
use crate::{search_best_food_full, Player, SimState};
use ol_ai::{
    BestFoodHit as AiBestFoodHit, BestFoodQuery, FoodSearch, PlayerView, WorldView,
    DEFAULT_FOOD_SEARCH_RADIUS,
};
use ol_world::World;

// ── World ───────────────────────────────────────────────────────────────────

/// [`WorldView`] over a borrowed [`World`].
pub struct WorldViewRef<'a>(pub &'a World);

impl WorldView for WorldViewRef<'_> {
    fn width_height(&self) -> (i32, i32) {
        (self.0.width_tiles, self.0.height_tiles)
    }

    fn wrap(&self) -> bool {
        self.0.wrap
    }

    fn object_at(&self, x: i32, y: i32) -> i32 {
        self.0.get_object(x, y)
    }

    fn biome_at(&self, x: i32, y: i32) -> u8 {
        self.0.get_biome(x, y)
    }

    fn floor_at(&self, x: i32, y: i32) -> i32 {
        self.0.get_floor(x, y) as i32
    }

    fn for_each_object_in_rect(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        f: &mut dyn FnMut(i32, i32, i32),
    ) {
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        // Cap scan area to avoid AI think blowups ( ~200² max ).
        let max_side = 200;
        let x1 = x1.min(x0.saturating_add(max_side));
        let y1 = y1.min(y0.saturating_add(max_side));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let id = self.0.get_object(x, y);
                if id != 0 {
                    f(x, y, id);
                }
            }
        }
    }
}

// ── Player ──────────────────────────────────────────────────────────────────

/// [`PlayerView`] from a live [`Player`].
pub struct PlayerRef<'a>(pub &'a Player);

impl PlayerView for PlayerRef<'_> {
    fn conn_id(&self) -> u64 {
        self.0.conn_id
    }
    fn p_id(&self) -> i32 {
        self.0.p_id
    }
    fn pos(&self) -> (i32, i32) {
        (self.0.x, self.0.y)
    }
    fn age(&self) -> f32 {
        self.0.age
    }
    fn food(&self) -> (f32, f32) {
        (self.0.food, self.0.food_max)
    }
    fn held_id(&self) -> i32 {
        self.0.held_id
    }
    fn home(&self) -> (i32, i32) {
        (self.0.home_x, self.0.home_y)
    }
    fn clothing(&self) -> [i32; 6] {
        self.0.clothing_parent_ids()
    }
    fn deleted(&self) -> bool {
        self.0.deleted
    }
    fn is_moving(&self) -> bool {
        self.0.moving || self.0.move_path.is_some()
    }
}

/// [`PlayerView`] from a published [`PlayerSnapshot`] (NPC scheduler thread).
pub struct PlayerSnapshotView<'a>(pub &'a PlayerSnapshot);

impl PlayerView for PlayerSnapshotView<'_> {
    fn conn_id(&self) -> u64 {
        self.0.conn_id
    }
    fn p_id(&self) -> i32 {
        self.0.p_id
    }
    fn pos(&self) -> (i32, i32) {
        (self.0.x, self.0.y)
    }
    fn age(&self) -> f32 {
        self.0.age
    }
    fn food(&self) -> (f32, f32) {
        (self.0.food, self.0.food_max)
    }
    fn held_id(&self) -> i32 {
        self.0.held_id
    }
    fn home(&self) -> (i32, i32) {
        // home_x/y from AI-JOB-SMITH-RESID snapshot; (0,0) is a valid home — always use fields.
        (self.0.home_x, self.0.home_y)
    }
    fn clothing(&self) -> [i32; 6] {
        self.0.clothing
    }
    fn deleted(&self) -> bool {
        self.0.deleted
    }
    fn is_moving(&self) -> bool {
        self.0.moving
    }
}

// ── Food search ─────────────────────────────────────────────────────────────

/// Live Haxe `SearchBestFood` via [`search_best_food_full`].
///
/// Default radius is [`DEFAULT_FOOD_SEARCH_RADIUS`] (30) when query uses defaults.
pub struct SimFoodSearch<'a> {
    pub state: &'a SimState,
    /// When true, use AI seed/danger gates; when false, human/DisplayBestFood style.
    pub ai: bool,
}

impl FoodSearch for SimFoodSearch<'_> {
    fn best_food(&self, q: BestFoodQuery) -> Option<AiBestFoodHit> {
        let radius = if q.max_dist > 0 {
            q.max_dist
        } else {
            DEFAULT_FOOD_SEARCH_RADIUS
        };
        let flags = if self.ai {
            Some(AiFoodSearchFlags::default())
        } else {
            None
        };
        let hit = search_best_food_full(self.state, q.conn_id, radius, None, flags, false)?;
        Some(AiBestFoodHit {
            x: hit.tx,
            y: hit.ty,
            food_id: hit.food_id,
            score: hit.scored_food_value,
            // Positive scored food value is a soft yum/meh signal for AI policy.
            is_yum: hit.scored_food_value > hit.food_value as f32,
        })
    }
}

/// Convenience: AI food search with default radius 30.
pub fn best_food_for_ai(state: &SimState, conn_id: u64) -> Option<AiBestFoodHit> {
    SimFoodSearch { state, ai: true }.best_food_default(conn_id)
}

/// Convenience: AI food search with explicit Chebyshev radius.
pub fn best_food_for_ai_radius(
    state: &SimState,
    conn_id: u64,
    max_dist: i32,
) -> Option<AiBestFoodHit> {
    SimFoodSearch { state, ai: true }.best_food(BestFoodQuery::with_radius(conn_id, max_dist))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_ai::FoodSearch;

    #[test]
    fn food_query_radius_default_constant() {
        assert_eq!(DEFAULT_FOOD_SEARCH_RADIUS, 30);
        let q = BestFoodQuery::new(1);
        assert_eq!(q.max_dist, 30);
    }
}
