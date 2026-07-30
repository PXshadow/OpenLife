//! Adapters: `ol-ai-api` traits ← live sim/world (Phase A).
//!
//! - **Write:** AI uses [`ol_ai_api::PlayerWriteInterface`] → `NetIntent` (not this file).
//! - **Read:** [`WorldView`] / [`PlayerView`] / [`FoodSearch`] / [`PlayerReadInterface`].
//!
//! Full Haxe `SearchBestFood` (`search_best_food_live.inc.rs`) is not always
//! `include!`d in `lib.rs` yet; [`SimFoodSearch`] uses a correct **ground**
//! scan at default radius 30 so the interface is real and NPC-ready. When the
//! live include is wired, upgrade [`SimFoodSearch::best_food`] to call it.

use crate::player::PlayerSnapshot;
use crate::{Player, SimState};
use ol_ai_api::{
    BestFoodHit as ApiBestFoodHit, BestFoodQuery, FoodSearch, PlayerReadInterface, PlayerView,
    WorldView, DEFAULT_FOOD_SEARCH_RADIUS,
};
use ol_world::World;

// ── World ───────────────────────────────────────────────────────────────────

/// [`WorldView`] over a borrowed [`World`] (caller holds the `RwLock` read guard).
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

/// Live best-food via ground scan on [`SimState`] (default radius **30**).
///
/// Phase A: nearest edible ground tile by Chebyshev, preferring higher
/// `food_value`. Container / yum / path-reach full parity lands when
/// `search_best_food_live.inc.rs` is included in `lib.rs`.
pub struct SimFoodSearch<'a> {
    pub state: &'a SimState,
    /// Reserved for AI seed/danger gates when full SearchBestFood is wired.
    pub ai: bool,
}

impl FoodSearch for SimFoodSearch<'_> {
    fn best_food(&self, q: BestFoodQuery) -> Option<ApiBestFoodHit> {
        let _ = self.ai;
        let p = self.state.players.get(&q.conn_id)?;
        if p.deleted {
            return None;
        }
        let radius = if q.max_dist > 0 {
            q.max_dist
        } else {
            DEFAULT_FOOD_SEARCH_RADIUS
        };
        let (px, py) = (p.x, p.y);
        let world = self.state.world.read().ok()?;
        let mut best: Option<(i32, i32, i32, i32, i32)> = None; // dist, -fv, x, y, food_id
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = px + dx;
                let y = py + dy;
                let id = world.get_object(x, y);
                if id == 0 {
                    continue;
                }
                let base = self.state.content.resolve_base_id(id);
                let Some(def) = self.state.content.get(base) else {
                    continue;
                };
                if def.food_value <= 0 {
                    continue;
                }
                let dist = dx.abs().max(dy.abs());
                let key = (dist, -def.food_value, x, y, base);
                if best.map(|b| key < b).unwrap_or(true) {
                    best = Some(key);
                }
            }
        }
        best.map(|(dist, neg_fv, x, y, food_id)| ApiBestFoodHit {
            x,
            y,
            food_id,
            score: (-neg_fv) as f32 / (1.0 + dist as f32),
            is_yum: false,
        })
    }
}

/// Convenience: AI food search with default radius 30.
pub fn best_food_for_ai(state: &SimState, conn_id: u64) -> Option<ApiBestFoodHit> {
    SimFoodSearch { state, ai: true }.best_food_default(conn_id)
}

/// Convenience: AI food search with explicit Chebyshev radius.
pub fn best_food_for_ai_radius(
    state: &SimState,
    conn_id: u64,
    max_dist: i32,
) -> Option<ApiBestFoodHit> {
    SimFoodSearch { state, ai: true }.best_food(BestFoodQuery::with_radius(conn_id, max_dist))
}

// ── Combined PlayerReadInterface (sim-thread) ───────────────────────────────

/// Full [`PlayerReadInterface`] over live [`SimState`] for one connection.
///
/// Implements world/player/food by locking / indexing `state` per call so we
/// never need to hold a `World` guard across the façade.
pub struct SimPlayerRead<'a> {
    pub state: &'a SimState,
    pub conn_id: u64,
}

impl SimPlayerRead<'_> {
    pub fn new(state: &SimState, conn_id: u64) -> Option<SimPlayerRead<'_>> {
        let p = state.players.get(&conn_id)?;
        if p.deleted {
            return None;
        }
        Some(SimPlayerRead { state, conn_id })
    }
}

impl WorldView for SimPlayerRead<'_> {
    fn width_height(&self) -> (i32, i32) {
        self.state
            .world
            .read()
            .map(|w| (w.width_tiles, w.height_tiles))
            .unwrap_or((0, 0))
    }

    fn wrap(&self) -> bool {
        self.state.world.read().map(|w| w.wrap).unwrap_or(false)
    }

    fn object_at(&self, x: i32, y: i32) -> i32 {
        self.state
            .world
            .read()
            .map(|w| w.get_object(x, y))
            .unwrap_or(0)
    }

    fn biome_at(&self, x: i32, y: i32) -> u8 {
        self.state
            .world
            .read()
            .map(|w| w.get_biome(x, y))
            .unwrap_or(0)
    }

    fn floor_at(&self, x: i32, y: i32) -> i32 {
        self.state
            .world
            .read()
            .map(|w| w.get_floor(x, y) as i32)
            .unwrap_or(0)
    }

    fn for_each_object_in_rect(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        f: &mut dyn FnMut(i32, i32, i32),
    ) {
        if let Ok(w) = self.state.world.read() {
            WorldViewRef(&w).for_each_object_in_rect(x0, y0, x1, y1, f);
        }
    }
}

impl PlayerView for SimPlayerRead<'_> {
    fn conn_id(&self) -> u64 {
        self.conn_id
    }
    fn p_id(&self) -> i32 {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.p_id)
            .unwrap_or(0)
    }
    fn pos(&self) -> (i32, i32) {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| (p.x, p.y))
            .unwrap_or((0, 0))
    }
    fn age(&self) -> f32 {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.age)
            .unwrap_or(0.0)
    }
    fn food(&self) -> (f32, f32) {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| (p.food, p.food_max))
            .unwrap_or((0.0, 0.0))
    }
    fn held_id(&self) -> i32 {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.held_id)
            .unwrap_or(0)
    }
    fn home(&self) -> (i32, i32) {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| (p.home_x, p.home_y))
            .unwrap_or((0, 0))
    }
    fn clothing(&self) -> [i32; 6] {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.clothing_parent_ids())
            .unwrap_or([0; 6])
    }
    fn deleted(&self) -> bool {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.deleted)
            .unwrap_or(true)
    }
    fn is_moving(&self) -> bool {
        self.state
            .players
            .get(&self.conn_id)
            .map(|p| p.moving || p.move_path.is_some())
            .unwrap_or(false)
    }
}

impl FoodSearch for SimPlayerRead<'_> {
    fn best_food(&self, q: BestFoodQuery) -> Option<ApiBestFoodHit> {
        SimFoodSearch {
            state: self.state,
            ai: true,
        }
        .best_food(q)
    }
}

impl PlayerReadInterface for SimPlayerRead<'_> {
    fn world(&self) -> &dyn WorldView {
        self
    }
    fn self_player(&self) -> &dyn PlayerView {
        self
    }
    fn food_search(&self) -> &dyn FoodSearch {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_ai_api::FoodSearch;

    #[test]
    fn food_query_radius_default_constant() {
        assert_eq!(DEFAULT_FOOD_SEARCH_RADIUS, 30);
        let q = BestFoodQuery::new(1);
        assert_eq!(q.max_dist, 30);
    }
}
