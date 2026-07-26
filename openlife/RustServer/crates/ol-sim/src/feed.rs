//! Feed-other pure logic (Haxe FeedOther / SAY FEED / breast-feeding subset).
//!
//! Range check + food transfer math only — no world I/O.

/// Max Chebyshev distance to feed another player (adjacent or same tile).
pub const FEED_RANGE: i32 = 1;

/// Haxe `MaxChildAgeForBreastFeeding` — child older than this cannot nurse.
pub const MAX_CHILD_AGE_BREAST_FEEDING: f32 = 6.0;
/// Haxe `PickupFeedingFoodRestore` — food granted to baby on HOLD (half from mother).
pub const PICKUP_FEEDING_FOOD_RESTORE: f32 = 1.5;
/// Haxe `FoodRestoreFactorWhileFeeding` — multiplier on FoodUsePerSecond while nursing.
pub const FOOD_RESTORE_FACTOR_WHILE_FEEDING: f32 = 2.0;
/// Cap baby food while nursing: fraction of baby's food_max (Haxe getMaxChildFeeding ~ half+).
pub const MAX_CHILD_FEEDING_FRAC: f32 = 0.85;

/// True when mother can continuously breast-feed held child (Haxe
/// `isHoldingChildInBreastFeedingAgeAndCanFeed`).
pub fn can_breastfeed(
    mother_age: f32,
    mother_food: f32,
    mother_fertile: bool,
    baby_age: f32,
    holding_baby: bool,
) -> bool {
    if !holding_baby || !mother_fertile {
        return false;
    }
    if mother_food < 0.0 {
        return false;
    }
    if baby_age > MAX_CHILD_AGE_BREAST_FEEDING {
        return false;
    }
    // Fertile band already checked by caller; still require adult-ish mother.
    mother_age >= 14.0
}

/// Continuous nursing transfer for `dt` seconds.
///
/// Returns `(food_to_baby, drain_from_mother)` matching Haxe TimeHelper:
/// baby gains `FoodRestoreFactorWhileFeeding * dt * food_use_per_sec`,
/// mother foodDecay += food/2.
pub fn breastfeed_tick(
    dt: f32,
    food_use_per_sec: f32,
    baby_food: f32,
    baby_food_max: f32,
) -> (f32, f32) {
    if dt <= 0.0 {
        return (0.0, 0.0);
    }
    let cap = (baby_food_max * MAX_CHILD_FEEDING_FRAC).max(0.0);
    if baby_food >= cap {
        return (0.0, 0.0);
    }
    let food = FOOD_RESTORE_FACTOR_WHILE_FEEDING * dt * food_use_per_sec;
    let room = (cap - baby_food).max(0.0);
    let to_baby = food.min(room);
    let from_mother = to_baby * 0.5;
    (to_baby, from_mother)
}

/// One-shot food on HOLD pickup (Haxe doBabyHelper PickupFeedingFoodRestore).
/// Returns `(to_baby, from_mother)`.
pub fn pickup_feed_amounts(baby_food: f32, baby_food_max: f32) -> (f32, f32) {
    let cap = (baby_food_max * MAX_CHILD_FEEDING_FRAC).max(0.0);
    if baby_food >= cap {
        return (0.0, 0.0);
    }
    let food = PICKUP_FEEDING_FOOD_RESTORE;
    let room = (cap - baby_food).max(0.0);
    let to_baby = food.min(room);
    (to_baby, to_baby * 0.5)
}

/// Whether the feeder may transfer held food to the target.
///
/// Errors:
/// - `"not food"` — held is empty or not food
/// - `"target deleted"` — target is deleted
/// - `"out of range"` — Chebyshev distance &gt; [`FEED_RANGE`]
pub fn can_feed(
    feeder_x: i32,
    feeder_y: i32,
    target_x: i32,
    target_y: i32,
    held_id: i32,
    target_deleted: bool,
    held_is_food: bool,
) -> Result<(), &'static str> {
    if held_id == 0 || !held_is_food {
        return Err("not food");
    }
    if target_deleted {
        return Err("target deleted");
    }
    let dist = (feeder_x - target_x).abs().max((feeder_y - target_y).abs());
    if dist > FEED_RANGE {
        return Err("out of range");
    }
    Ok(())
}

/// Apply feed amounts: fill target up to `target_max` from `held_food_value`.
///
/// Returns `(new_target_food, leftover)`.
/// - If the whole held value is consumed (target room ≥ held), leftover is `0.0`.
/// - If target is already full, leftover equals the full held value (nothing transferred).
pub fn apply_feed_amounts(
    held_food_value: f32,
    target_food: f32,
    target_max: f32,
) -> (f32, f32) {
    if held_food_value <= 0.0 {
        return (target_food.min(target_max), held_food_value.max(0.0));
    }
    let room = (target_max - target_food).max(0.0);
    let transferred = held_food_value.min(room);
    let new_target = (target_food + transferred).min(target_max);
    let leftover = held_food_value - transferred;
    (new_target, leftover)
}

/// Heuristic: object name looks like common edible food when content food_value
/// is unavailable. Case-insensitive substring match.
pub fn name_looks_like_food(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("berry")
        || lower.contains("fruit")
        || lower.contains("meat")
        || lower.contains("bread")
        || lower.contains("pie")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breastfeed_gates() {
        assert!(can_breastfeed(20.0, 10.0, true, 2.0, true));
        assert!(!can_breastfeed(20.0, 10.0, true, 7.0, true)); // too old
        assert!(!can_breastfeed(20.0, -0.1, true, 2.0, true)); // mother starving
        assert!(!can_breastfeed(20.0, 10.0, false, 2.0, true)); // not fertile
        assert!(!can_breastfeed(20.0, 10.0, true, 2.0, false)); // not holding
    }

    #[test]
    fn breastfeed_tick_transfers() {
        let (to, from) = breastfeed_tick(1.0, 0.2, 5.0, 20.0);
        assert!(to > 0.0);
        assert!((from - to * 0.5).abs() < 1e-5);
        let (to2, from2) = breastfeed_tick(1.0, 0.2, 19.0, 20.0);
        assert_eq!(to2, 0.0);
        assert_eq!(from2, 0.0);
    }

    #[test]
    fn pickup_feed_half_from_mother() {
        let (to, from) = pickup_feed_amounts(0.0, 20.0);
        assert!((to - PICKUP_FEEDING_FOOD_RESTORE).abs() < 1e-5);
        assert!((from - to * 0.5).abs() < 1e-5);
    }

    #[test]
    fn can_feed_requires_food_and_range() {
        assert!(can_feed(0, 0, 0, 0, 33, false, true).is_ok());
        assert!(can_feed(0, 0, 1, 0, 33, false, true).is_ok());
        assert!(can_feed(0, 0, 1, 1, 33, false, true).is_ok());
        assert_eq!(
            can_feed(0, 0, 2, 0, 33, false, true),
            Err("out of range")
        );
        assert_eq!(
            can_feed(0, 0, 0, 0, 0, false, true),
            Err("not food")
        );
        assert_eq!(
            can_feed(0, 0, 0, 0, 33, false, false),
            Err("not food")
        );
        assert_eq!(
            can_feed(0, 0, 0, 0, 33, true, true),
            Err("target deleted")
        );
    }

    #[test]
    fn apply_feed_full_consume() {
        let (food, left) = apply_feed_amounts(5.0, 10.0, 20.0);
        assert!((food - 15.0).abs() < 1e-5);
        assert!((left - 0.0).abs() < 1e-5);
    }

    #[test]
    fn apply_feed_partial_when_near_max() {
        let (food, left) = apply_feed_amounts(5.0, 18.0, 20.0);
        assert!((food - 20.0).abs() < 1e-5);
        assert!((left - 3.0).abs() < 1e-5);
    }

    #[test]
    fn apply_feed_nothing_when_full() {
        let (food, left) = apply_feed_amounts(4.0, 20.0, 20.0);
        assert!((food - 20.0).abs() < 1e-5);
        assert!((left - 4.0).abs() < 1e-5);
    }

    #[test]
    fn name_food_heuristic() {
        assert!(name_looks_like_food("Wild Gooseberry"));
        assert!(name_looks_like_food("Cooked Meat"));
        assert!(name_looks_like_food("Berry Pie"));
        assert!(!name_looks_like_food("Stone Axe"));
    }
}
