//! Hungry → SearchBestFood plan (shared FoodSearch interface).

use crate::plan::{ThinkPlan, ThinkSensors};
use ol_ai_api::{FoodSearch, PlayerReadInterface, DEFAULT_FOOD_SEARCH_RADIUS};
use ol_ai_helper::HUNGRY_ENTER_FRAC;

/// Haxe-style hungry enter: `food_store < food_store_max * HUNGRY_ENTER_FRAC`
/// (or absolute floor for tiny max).
#[inline]
pub fn is_hungry_for_food_seek(food_store: f32, food_store_max: f32) -> bool {
    if food_store_max <= 0.0 {
        return food_store < 5.0;
    }
    food_store < food_store_max * HUNGRY_ENTER_FRAC
}

/// Plan food seek/use from a [`FoodSearch`] handle (AI or player adapter).
///
/// Uses default radius [`DEFAULT_FOOD_SEARCH_RADIUS`] (40) unless the
/// implementor overrides via query — we call `best_food_default`.
pub fn plan_hungry_food(
    food: &dyn FoodSearch,
    sensors: &ThinkSensors,
) -> ThinkPlan {
    if sensors.moving {
        return ThinkPlan::Idle;
    }
    if !is_hungry_for_food_seek(sensors.food_store, sensors.food_store_max) {
        return ThinkPlan::Idle;
    }
    let Some(hit) = food.best_food_default(sensors.conn_id) else {
        return ThinkPlan::Idle;
    };
    let dist = (hit.x - sensors.x)
        .abs()
        .max((hit.y - sensors.y).abs());
    if dist <= 1 {
        ThinkPlan::UseFoodTile {
            tx: hit.x,
            ty: hit.y,
            food_id: hit.food_id,
        }
    } else {
        ThinkPlan::SeekFood {
            tx: hit.x,
            ty: hit.y,
            food_id: hit.food_id,
        }
    }
}

/// Same as [`plan_hungry_food`] from a full [`PlayerReadInterface`].
pub fn plan_hungry_food_from_read(
    read: &dyn PlayerReadInterface,
    sensors: &ThinkSensors,
) -> ThinkPlan {
    plan_hungry_food(read.food_search(), sensors)
}

/// Documented default radius for callers building scans.
#[inline]
pub fn food_search_radius() -> i32 {
    DEFAULT_FOOD_SEARCH_RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_ai_api::{BestFoodHit, BestFoodQuery, FoodSearch};

    struct FixedFood(Option<BestFoodHit>);
    impl FoodSearch for FixedFood {
        fn best_food(&self, _q: BestFoodQuery) -> Option<BestFoodHit> {
            self.0
        }
    }

    #[test]
    fn not_hungry_is_idle() {
        let s = ThinkSensors {
            conn_id: 1,
            x: 0,
            y: 0,
            food_store: 15.0,
            food_store_max: 20.0,
            held_id: 0,
            moving: false,
        };
        assert_eq!(
            plan_hungry_food(
                &FixedFood(Some(BestFoodHit {
                    x: 3,
                    y: 0,
                    food_id: 31,
                    score: 1.0,
                    is_yum: true
                })),
                &s
            ),
            ThinkPlan::Idle
        );
    }

    #[test]
    fn hungry_far_seeks() {
        let s = ThinkSensors {
            conn_id: 1,
            x: 0,
            y: 0,
            food_store: 2.0,
            food_store_max: 20.0,
            held_id: 0,
            moving: false,
        };
        assert_eq!(
            plan_hungry_food(
                &FixedFood(Some(BestFoodHit {
                    x: 5,
                    y: 0,
                    food_id: 31,
                    score: 2.0,
                    is_yum: true
                })),
                &s
            ),
            ThinkPlan::SeekFood {
                tx: 5,
                ty: 0,
                food_id: 31
            }
        );
    }

    #[test]
    fn hungry_adjacent_uses() {
        let s = ThinkSensors {
            conn_id: 1,
            x: 0,
            y: 0,
            food_store: 2.0,
            food_store_max: 20.0,
            held_id: 0,
            moving: false,
        };
        assert_eq!(
            plan_hungry_food(
                &FixedFood(Some(BestFoodHit {
                    x: 1,
                    y: 0,
                    food_id: 31,
                    score: 2.0,
                    is_yum: true
                })),
                &s
            ),
            ThinkPlan::UseFoodTile {
                tx: 1,
                ty: 0,
                food_id: 31
            }
        );
    }

    #[test]
    fn default_radius_is_40() {
        assert_eq!(food_search_radius(), 40);
    }
}
