//! Feed-other pure logic (Haxe FeedOther / SAY FEED / breast-feeding).
//!
//! **BREASTFEED-EDGES / nurse_edges** — Haxe parity for:
//! - `isHoldingChildInBreastFeedingAgeAndCanFeed`
//! - TimeHelper continuous breast-feed (`FoodRestoreFactorWhileFeeding`)
//! - `doBabyHelper` pickup restore + exhaustion + mother-follow + happy emote gate
//! - `getMaxChildFeeding` cap
//!
//! Range check + food transfer math only — no world I/O.

/// Max Chebyshev distance to feed another player (adjacent or same tile).
pub const FEED_RANGE: i32 = 1;

/// Haxe `MaxChildAgeForBreastFeeding` — child **older** than this cannot nurse
/// (`age > Max` fails continuous; `age == Max` still OK).
// Haxe: ServerSettings.MaxChildAgeForBreastFeeding = 6
pub const MAX_CHILD_AGE_BREAST_FEEDING: f32 = 6.0;

/// Haxe `MaxAgeForAllowingClothAndPrickupFromOthers` — cannot HOLD/BABY if
/// `target.age >=` this (knocked-out pickup TODO in Haxe too).
// Haxe: ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers = 10
pub const MAX_AGE_FOR_PICKUP_FROM_OTHERS: f32 = 10.0;

/// Haxe `PickupFeedingFoodRestore` — food granted to baby on HOLD (half from mother).
// Haxe: ServerSettings.PickupFeedingFoodRestore = 1.5
pub const PICKUP_FEEDING_FOOD_RESTORE: f32 = 1.5;

/// Haxe `FoodRestoreFactorWhileFeeding` — multiplier on FoodUsePerSecond while nursing.
// Haxe: ServerSettings.FoodRestoreFactorWhileFeeding = 10
pub const FOOD_RESTORE_FACTOR_WHILE_FEEDING: f32 = 10.0;

/// Haxe `PickupExhaustionGain` on successful doBaby/HOLD.
// Haxe: ServerSettings.PickupExhaustionGain = 0.2
pub const PICKUP_EXHAUSTION_GAIN: f32 = 0.2;

/// Haxe `PickupBabyMaxDistance` — euclidean max for doBaby/BABY (squared compare).
// Haxe: ServerSettings.PickupBabyMaxDistance = 1.9
pub const PICKUP_BABY_MAX_DISTANCE: f32 = 1.9;

/// Haxe TimeHelper hits heal while nursing: `hits -= time * 0.2`.
// Haxe: TimeHelper.doPlayerTimeStuff breast-feed block
pub const NURSE_HITS_HEAL_PER_SEC: f32 = 0.2;

/// Floor used by Haxe `getMaxChildFeeding` when `food_store_max` is tiny.
// Haxe: GlobalPlayerInstance.getMaxChildFeeding → Math.max(4, food_store_max)
pub const MIN_MAX_CHILD_FEEDING: f32 = 4.0;

/// Haxe `getMaxChildFeeding` — baby food cap while nursing / pickup restore.
// Haxe: GlobalPlayerInstance.getMaxChildFeeding L6290
#[inline]
pub fn get_max_child_feeding(food_store_max: f32) -> f32 {
    food_store_max.max(MIN_MAX_CHILD_FEEDING)
}

/// Continuous nurse age gate: Haxe `heldPlayer.age > MaxChildAgeForBreastFeeding`.
/// Age exactly equal to max is still allowed.
// Haxe: isHoldingChildInBreastFeedingAgeAndCanFeed L5907-5911
#[inline]
pub fn can_nurse_age(baby_age: f32) -> bool {
    can_nurse_age_ex(baby_age, MAX_CHILD_AGE_BREAST_FEEDING)
}

/// Live max-age variant of [`can_nurse_age`] (`age <= max` continuous).
// Haxe: ServerSettings.MaxChildAgeForBreastFeeding
// C-SS-MORE-BATCH3
#[inline]
pub fn can_nurse_age_ex(baby_age: f32, max_child_age: f32) -> bool {
    let max = if max_child_age.is_finite() && max_child_age >= 0.0 {
        max_child_age
    } else {
        MAX_CHILD_AGE_BREAST_FEEDING
    };
    baby_age.is_finite() && baby_age <= max
}

/// Pickup one-shot breast-feed age: Haxe `targetPlayer.age < MaxChildAgeForBreastFeeding`
/// (strict less — age == max does **not** get pickup restore).
// Haxe: doBabyHelper L4992
#[inline]
pub fn can_pickup_breastfeed_age(baby_age: f32) -> bool {
    can_pickup_breastfeed_age_ex(baby_age, MAX_CHILD_AGE_BREAST_FEEDING)
}

/// Live max-age variant of [`can_pickup_breastfeed_age`] (`age < max` strict).
// Haxe: ServerSettings.MaxChildAgeForBreastFeeding
// C-SS-MORE-BATCH3
#[inline]
pub fn can_pickup_breastfeed_age_ex(baby_age: f32, max_child_age: f32) -> bool {
    let max = if max_child_age.is_finite() && max_child_age >= 0.0 {
        max_child_age
    } else {
        MAX_CHILD_AGE_BREAST_FEEDING
    };
    baby_age.is_finite() && baby_age < max
}

/// Haxe doBaby age gates for HOLD/BABY pickup (not breast-feed-only).
/// Target must be younger than [`MAX_AGE_FOR_PICKUP_FROM_OTHERS`]; carrier must
/// be at least one year older than target. **No hard age≥14 on carrier.**
// Haxe: doBabyHelper L4956-4964
#[inline]
pub fn can_pickup_player_ages(carrier_age: f32, target_age: f32) -> bool {
    if !carrier_age.is_finite() || !target_age.is_finite() {
        return false;
    }
    if target_age >= MAX_AGE_FOR_PICKUP_FROM_OTHERS {
        return false;
    }
    carrier_age >= target_age + 1.0
}

/// Haxe `isCloseToPlayerUseExact(..., PickupBabyMaxDistance)` for doBaby.
/// Uses euclidean distance on float positions (tile coords as f32 when no exact).
// Haxe: MoveHelper.isCloseUseExact + ServerSettings.PickupBabyMaxDistance
#[inline]
pub fn can_pickup_baby_distance(ax: f32, ay: f32, bx: f32, by: f32) -> bool {
    can_pickup_baby_distance_ex(ax, ay, bx, by, PICKUP_BABY_MAX_DISTANCE)
}

/// Live max-distance variant of [`can_pickup_baby_distance`].
// Haxe: ServerSettings.PickupBabyMaxDistance
// C-SS-MORE-BATCH4
#[inline]
pub fn can_pickup_baby_distance_ex(
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    max_distance: f32,
) -> bool {
    if !ax.is_finite() || !ay.is_finite() || !bx.is_finite() || !by.is_finite() {
        return false;
    }
    let max = if max_distance.is_finite() && max_distance > 0.0 {
        max_distance as f64
    } else {
        PICKUP_BABY_MAX_DISTANCE as f64
    };
    let dx = (ax - bx) as f64;
    let dy = (ay - by) as f64;
    dx * dx + dy * dy <= max * max
}

/// Hands-free gate for HOLD/BABY (age gates are separate via [`can_pickup_player_ages`]).
/// Haxe: o_id empty (or only hiddenWound, not modeled here).
// Haxe: doBabyHelper L4947-4954
#[inline]
pub fn can_hold_baby_hands(deleted: bool, holding_player_id: i32, held_id: i32) -> bool {
    !deleted && holding_player_id == 0 && held_id == 0
}

/// Haxe `ObjectHelper.isDroppable` — drop target's held object on pickup when true.
// Haxe: ObjectHelper.isDroppable → id != 0 && !isWound
#[inline]
pub fn is_droppable_on_baby_pickup(held_id: i32, is_wound: bool) -> bool {
    held_id != 0 && !is_wound
}

/// Target is holding another player → must force-drop before pickup.
// Haxe: doBabyHelper L4972-4976 targetPlayer.dropPlayer
#[inline]
pub fn needs_force_drop_nested_hold(target_holding_player_id: i32) -> bool {
    target_holding_player_id != 0
}

/// Continuous nurse mother cost: Haxe folds half-transfer into `foodDecay`, then
/// drains **yum_bonus first** (full amount, may go negative) else `food_store`.
///
/// Returns `(new_yum_bonus, new_food)`.
// Haxe: TimeHelper.doPlayerTimeStuff L978-983 (after foodDecay += food/2)
#[inline]
pub fn drain_mother_nurse_cost(yum_bonus: f32, food: f32, amount: f32) -> (f32, f32) {
    if amount <= 0.0 || !amount.is_finite() {
        return (yum_bonus, food);
    }
    // Port-as-is: if yum_bonus > 0, subtract full amount from yum only (can go negative).
    if yum_bonus > 0.0 {
        (yum_bonus - amount, food)
    } else {
        (yum_bonus, food - amount)
    }
}

/// Whether baby food `ceil` changed (Haxe sendFoodUpdate when ceil differs).
// Haxe: TimeHelper L955-966 Math.ceil before/after
#[inline]
pub fn baby_food_ceil_changed(food_before: f32, food_after: f32) -> bool {
    food_before.ceil() as i32 != food_after.ceil() as i32
}

/// True when mother can continuously breast-feed held child (Haxe
/// `isHoldingChildInBreastFeedingAgeAndCanFeed`).
// Haxe: GlobalPlayerInstance.isHoldingChildInBreastFeedingAgeAndCanFeed L5907
pub fn can_breastfeed(
    mother_age: f32,
    mother_food: f32,
    mother_fertile: bool,
    baby_age: f32,
    holding_baby: bool,
) -> bool {
    can_breastfeed_ex(
        mother_age,
        mother_food,
        mother_fertile,
        baby_age,
        holding_baby,
        MAX_CHILD_AGE_BREAST_FEEDING,
    )
}

/// Live max-child-age variant of [`can_breastfeed`].
// Haxe: ServerSettings.MaxChildAgeForBreastFeeding
// C-SS-MORE-BATCH3
pub fn can_breastfeed_ex(
    mother_age: f32,
    mother_food: f32,
    mother_fertile: bool,
    baby_age: f32,
    holding_baby: bool,
    max_child_age: f32,
) -> bool {
    if !holding_baby || !mother_fertile {
        return false;
    }
    // Haxe: food_store < 0 → cannot feed (exactly 0 still can).
    if mother_food < 0.0 {
        return false;
    }
    if !can_nurse_age_ex(baby_age, max_child_age) {
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
/// Cap is [`get_max_child_feeding`] (not a fraction of food_max).
// Haxe: TimeHelper.doPlayerTimeStuff L950-969
pub fn breastfeed_tick(
    dt: f32,
    food_use_per_sec: f32,
    baby_food: f32,
    baby_food_max: f32,
) -> (f32, f32) {
    breastfeed_tick_ex(
        dt,
        food_use_per_sec,
        baby_food,
        baby_food_max,
        FOOD_RESTORE_FACTOR_WHILE_FEEDING,
    )
}

/// Live-factor variant of [`breastfeed_tick`].
// Haxe: ServerSettings.FoodRestoreFactorWhileFeeding
// C-SS-MORE-KNOBS
pub fn breastfeed_tick_ex(
    dt: f32,
    food_use_per_sec: f32,
    baby_food: f32,
    baby_food_max: f32,
    food_restore_factor_while_feeding: f32,
) -> (f32, f32) {
    if dt <= 0.0 {
        return (0.0, 0.0);
    }
    let cap = get_max_child_feeding(baby_food_max);
    if baby_food >= cap {
        return (0.0, 0.0);
    }
    let factor = if food_restore_factor_while_feeding.is_finite()
        && food_restore_factor_while_feeding >= 0.0
    {
        food_restore_factor_while_feeding
    } else {
        FOOD_RESTORE_FACTOR_WHILE_FEEDING
    };
    let food = factor * dt * food_use_per_sec;
    let room = (cap - baby_food).max(0.0);
    let to_baby = food.min(room);
    let from_mother = to_baby * 0.5;
    (to_baby, from_mother)
}

/// Hits reduced while nursing for `dt` (Haxe always under can-feed gate,
/// even when baby food is already at cap).
// Haxe: TimeHelper L971-973
#[inline]
pub fn nurse_hits_heal(hits: f32, dt: f32) -> f32 {
    if hits <= 0.0 || dt <= 0.0 {
        return hits.max(0.0);
    }
    (hits - dt * NURSE_HITS_HEAL_PER_SEC).max(0.0)
}

/// One-shot food on HOLD pickup (Haxe doBabyHelper PickupFeedingFoodRestore).
/// Returns `(to_baby, from_mother)`. Cap = [`get_max_child_feeding`].
// Haxe: doBabyHelper L4992-5000
pub fn pickup_feed_amounts(baby_food: f32, baby_food_max: f32) -> (f32, f32) {
    let cap = get_max_child_feeding(baby_food_max);
    if baby_food >= cap {
        return (0.0, 0.0);
    }
    let food = PICKUP_FEEDING_FOOD_RESTORE;
    let room = (cap - baby_food).max(0.0);
    let to_baby = food.min(room);
    (to_baby, to_baby * 0.5)
}

/// Haxe doBaby follow reassignment: set follow to picker when target has no
/// follow, or follow is non-fertile and picker is fertile.
// Haxe: doBabyHelper L5009-5013
#[inline]
pub fn should_set_follow_on_hold(
    has_follow: bool,
    follow_is_fertile: bool,
    picker_is_fertile: bool,
) -> bool {
    if !has_follow {
        return true;
    }
    !follow_is_fertile && picker_is_fertile
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

    /// C-SS-MORE-BATCH3: live MaxChildAge ≤ continuous vs < pickup.
    // Haxe: ServerSettings.MaxChildAgeForBreastFeeding
    #[test]
    fn max_child_age_live_boundary() {
        // Continuous: age == max still OK
        assert!(can_nurse_age_ex(8.0, 8.0));
        assert!(!can_nurse_age_ex(8.01, 8.0));
        // Pickup: strict less
        assert!(!can_pickup_breastfeed_age_ex(8.0, 8.0));
        assert!(can_pickup_breastfeed_age_ex(7.99, 8.0));
        assert!(can_breastfeed_ex(20.0, 10.0, true, 8.0, true, 8.0));
        assert!(!can_breastfeed_ex(20.0, 10.0, true, 8.01, true, 8.0));
    }

    #[test]
    fn breastfeed_gates() {
        assert!(can_breastfeed(20.0, 10.0, true, 2.0, true));
        assert!(can_breastfeed(20.0, 10.0, true, 6.0, true)); // age == max OK continuous
        assert!(!can_breastfeed(20.0, 10.0, true, 6.01, true)); // too old
        assert!(!can_breastfeed(20.0, 10.0, true, 7.0, true));
        assert!(!can_breastfeed(20.0, -0.1, true, 2.0, true)); // mother starving
        assert!(can_breastfeed(20.0, 0.0, true, 2.0, true)); // food == 0 still can
        assert!(!can_breastfeed(20.0, 10.0, false, 2.0, true)); // not fertile
        assert!(!can_breastfeed(20.0, 10.0, true, 2.0, false)); // not holding
    }

    #[test]
    fn nurse_vs_pickup_age_edge() {
        // Continuous: age == 6 OK; pickup restore: age == 6 blocked (Haxe < vs >).
        assert!(can_nurse_age(6.0));
        assert!(!can_pickup_breastfeed_age(6.0));
        assert!(can_pickup_breastfeed_age(5.99));
        assert!(!can_nurse_age(6.01));
    }

    #[test]
    fn get_max_child_feeding_matches_haxe() {
        assert!((get_max_child_feeding(20.0) - 20.0).abs() < 1e-5);
        assert!((get_max_child_feeding(2.0) - 4.0).abs() < 1e-5);
        assert!((get_max_child_feeding(4.0) - 4.0).abs() < 1e-5);
    }

    #[test]
    fn breastfeed_tick_factor_ten() {
        // food = 10 * 1 * 0.1 = 1.0 for default FoodUsePerSecond
        let (to, from) = breastfeed_tick(1.0, 0.10, 5.0, 20.0);
        assert!((to - 1.0).abs() < 1e-5, "got to={to}");
        assert!((from - 0.5).abs() < 1e-5);
        // Cap = food_max (20), not 0.85*20
        let (to2, from2) = breastfeed_tick(1.0, 0.10, 19.5, 20.0);
        assert!((to2 - 0.5).abs() < 1e-5, "room to full max, got {to2}");
        assert!((from2 - 0.25).abs() < 1e-5);
        let (to3, from3) = breastfeed_tick(1.0, 0.10, 20.0, 20.0);
        assert_eq!(to3, 0.0);
        assert_eq!(from3, 0.0);
    }

    /// C-SS-MORE-KNOBS: FoodRestoreFactorWhileFeeding live scales baby gain + mother half-cost.
    // Haxe: ServerSettings.FoodRestoreFactorWhileFeeding
    #[test]
    fn breastfeed_tick_live_factor_override() {
        let (to, from) = breastfeed_tick_ex(1.0, 0.10, 5.0, 20.0, 20.0);
        assert!((to - 2.0).abs() < 1e-5, "got to={to}");
        assert!((from - 1.0).abs() < 1e-5);
        let (to0, from0) = breastfeed_tick_ex(1.0, 0.10, 5.0, 20.0, 0.0);
        assert_eq!(to0, 0.0);
        assert_eq!(from0, 0.0);
    }

    #[test]
    fn breastfeed_tick_tiny_food_max_floor() {
        // food_max 2 → cap 4; baby at 3 still has room
        let (to, _) = breastfeed_tick(1.0, 0.10, 3.0, 2.0);
        assert!((to - 1.0).abs() < 1e-5);
    }

    #[test]
    fn nurse_hits_heal_even_when_full() {
        assert!((nurse_hits_heal(1.0, 1.0) - 0.8).abs() < 1e-5);
        assert_eq!(nurse_hits_heal(0.0, 1.0), 0.0);
        assert_eq!(nurse_hits_heal(0.1, 1.0), 0.0);
    }

    #[test]
    fn pickup_feed_half_from_mother() {
        let (to, from) = pickup_feed_amounts(0.0, 20.0);
        assert!((to - PICKUP_FEEDING_FOOD_RESTORE).abs() < 1e-5);
        assert!((from - to * 0.5).abs() < 1e-5);
    }

    #[test]
    fn pickup_feed_respects_max_child_feeding() {
        let (to, _) = pickup_feed_amounts(20.0, 20.0);
        assert_eq!(to, 0.0);
        let (to2, _) = pickup_feed_amounts(3.5, 2.0); // cap 4
        assert!((to2 - 0.5).abs() < 1e-5);
    }

    #[test]
    fn can_pickup_player_ages_haxe() {
        assert!(can_pickup_player_ages(20.0, 2.0));
        assert!(can_pickup_player_ages(11.0, 9.5));
        assert!(!can_pickup_player_ages(20.0, 10.0)); // too old
        assert!(!can_pickup_player_ages(10.5, 10.0));
        assert!(!can_pickup_player_ages(5.0, 5.0)); // need +1 year
        assert!(can_pickup_player_ages(6.0, 5.0));
        // No hard carrier age≥14 — age-12 may hold age-5
        assert!(can_pickup_player_ages(12.0, 5.0));
        assert!(!can_pickup_player_ages(12.0, 11.5)); // need +1
    }

    #[test]
    fn can_pickup_baby_distance_euclidean_1_9() {
        // Same tile
        assert!(can_pickup_baby_distance(0.0, 0.0, 0.0, 0.0));
        // Adjacent tile (dist 1) OK
        assert!(can_pickup_baby_distance(0.0, 0.0, 1.0, 0.0));
        // Diagonal tile (√2 ≈ 1.41) OK
        assert!(can_pickup_baby_distance(0.0, 0.0, 1.0, 1.0));
        // Dist 1.9 on axis OK
        assert!(can_pickup_baby_distance(0.0, 0.0, 1.9, 0.0));
        // Dist 2.0 blocked (Chebyshev-1 would still allow if same-row gap 2 — euclid fails)
        assert!(!can_pickup_baby_distance(0.0, 0.0, 2.0, 0.0));
        // Far diagonal blocked
        assert!(!can_pickup_baby_distance(0.0, 0.0, 2.0, 2.0));
    }

    /// C-SS-MORE-BATCH4: live PickupBabyMaxDistance boundary.
    #[test]
    fn can_pickup_baby_distance_ex_live_max() {
        // Default 1.9: dist 2.0 blocked
        assert!(!can_pickup_baby_distance_ex(0.0, 0.0, 2.0, 0.0, 1.9));
        // Live 2.5: dist 2.0 allowed, dist 2.5 allowed, 2.6 blocked
        assert!(can_pickup_baby_distance_ex(0.0, 0.0, 2.0, 0.0, 2.5));
        assert!(can_pickup_baby_distance_ex(0.0, 0.0, 2.5, 0.0, 2.5));
        assert!(!can_pickup_baby_distance_ex(0.0, 0.0, 2.6, 0.0, 2.5));
        // Invalid max falls back to 1.9
        assert!(!can_pickup_baby_distance_ex(0.0, 0.0, 2.0, 0.0, f32::NAN));
        assert!(can_pickup_baby_distance_ex(0.0, 0.0, 1.9, 0.0, 0.0));
    }

    #[test]
    fn can_hold_baby_hands_free_only() {
        assert!(can_hold_baby_hands(false, 0, 0));
        assert!(!can_hold_baby_hands(true, 0, 0));
        assert!(!can_hold_baby_hands(false, 5, 0));
        assert!(!can_hold_baby_hands(false, 0, 33));
    }

    #[test]
    fn is_droppable_on_baby_pickup_haxe() {
        assert!(is_droppable_on_baby_pickup(33, false));
        assert!(!is_droppable_on_baby_pickup(0, false));
        assert!(!is_droppable_on_baby_pickup(798, true)); // wound
    }

    #[test]
    fn drain_mother_nurse_prefers_yum_bonus() {
        // yum > 0 → full amount from yum (may go negative), food untouched
        let (y, f) = drain_mother_nurse_cost(2.0, 10.0, 0.5);
        assert!((y - 1.5).abs() < 1e-5);
        assert!((f - 10.0).abs() < 1e-5);
        let (y2, f2) = drain_mother_nurse_cost(0.2, 10.0, 0.5);
        assert!((y2 - (-0.3)).abs() < 1e-5, "yum overshoot, got {y2}");
        assert!((f2 - 10.0).abs() < 1e-5);
        // yum == 0 → drain food
        let (y3, f3) = drain_mother_nurse_cost(0.0, 10.0, 0.5);
        assert_eq!(y3, 0.0);
        assert!((f3 - 9.5).abs() < 1e-5);
    }

    #[test]
    fn baby_food_ceil_changed_detects_unit_cross() {
        // ceil(4.1)=5, ceil(5.0)=5 → no change; ceil(3.9)=4, ceil(5.0)=5 → change
        assert!(!baby_food_ceil_changed(4.1, 5.0));
        assert!(!baby_food_ceil_changed(4.1, 4.9));
        assert!(baby_food_ceil_changed(3.9, 5.0));
        assert!(baby_food_ceil_changed(4.0, 5.0)); // ceil 4 → 5
    }

    #[test]
    fn should_set_follow_on_hold_logic() {
        assert!(should_set_follow_on_hold(false, false, true));
        assert!(should_set_follow_on_hold(true, false, true)); // infertile follow → fertile picker
        assert!(!should_set_follow_on_hold(true, true, true)); // already fertile follow
        assert!(!should_set_follow_on_hold(true, false, false)); // picker also infertile
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
        assert_eq!(can_feed(0, 0, 0, 0, 0, false, true), Err("not food"));
        assert_eq!(can_feed(0, 0, 0, 0, 33, false, false), Err("not food"));
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
