//! Pure eat / feed / yum classification used by both AI food search and player eat paths.
//!
//! Extracted from `ol-sim` `yum.rs` free functions so [`crate::food_search`] does not
//! depend on the full sim crate.

/// Haxe `ServerSettings.YumBonus`.
pub const YUM_BONUS: f32 = 5.0;

/// Refuse super-meh when food_store is above this (Haxe canEatObj).
pub const SUPER_MEH_REFUSE_FOOD_STORE: f32 = 4.0;

/// Refuse meh feed when food_store is above this (Haxe canFeedToMeObj).
pub const MEH_FEED_REFUSE_FOOD_STORE: f32 = 2.0;

/// Psilocybe Mushroom — only feed when eater has yellow fever.
// Haxe: GlobalPlayerInstance.canFeedToMeObj parentId == 837
pub const PSILOCYBE_MUSHROOM_ID: i32 = 837;

/// Sanitize live YumBonus (NaN / negative → default).
#[inline]
pub fn resolve_yum_bonus(yum_bonus: f32) -> f32 {
    if yum_bonus.is_finite() && yum_bonus >= 0.0 {
        yum_bonus
    } else {
        YUM_BONUS
    }
}

/// Haxe `isObjYum` with live `ServerSettings.YumBonus`.
pub fn is_obj_yum_ex(food_value: i32, count_eaten: f32, yum_bonus: f32) -> bool {
    if food_value < 1 {
        return false;
    }
    count_eaten < resolve_yum_bonus(yum_bonus)
}

/// Haxe `isObjYum` at default [`YUM_BONUS`].
pub fn is_obj_yum(food_value: i32, count_eaten: f32) -> bool {
    is_obj_yum_ex(food_value, count_eaten, YUM_BONUS)
}

/// Haxe `isObjMeh` with live YumBonus.
pub fn is_obj_meh_ex(food_value: i32, count_eaten: f32, yum_bonus: f32) -> bool {
    !is_obj_yum_ex(food_value, count_eaten, yum_bonus)
}

/// Haxe `isObjMeh`.
pub fn is_obj_meh(food_value: i32, count_eaten: f32) -> bool {
    is_obj_meh_ex(food_value, count_eaten, YUM_BONUS)
}

/// Haxe `isObjSuperMeh` with live YumBonus.
pub fn is_obj_super_meh_ex(food_value: i32, count_eaten: f32, yum_bonus: f32) -> bool {
    if food_value < 1 {
        return false;
    }
    let yb = resolve_yum_bonus(yum_bonus);
    let base = food_value as f32;
    let count = if count_eaten < 0.0 { 0.0 } else { count_eaten };
    let adjusted = base + yb - count;
    adjusted < base / 2.0
}

/// Haxe `isObjSuperMeh` at default [`YUM_BONUS`].
pub fn is_obj_super_meh(food_value: i32, count_eaten: f32) -> bool {
    is_obj_super_meh_ex(food_value, count_eaten, YUM_BONUS)
}

/// Haxe `canEatObj` with live YumBonus.
pub fn can_eat_obj_ex(
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
    yum_bonus: f32,
) -> bool {
    if food_value <= 0 {
        return false;
    }
    if is_obj_super_meh_ex(food_value, count_eaten, yum_bonus)
        && food_store > SUPER_MEH_REFUSE_FOOD_STORE
    {
        return false;
    }
    let room = food_store_max - food_store;
    let need = ((food_value as f32) / 4.0).ceil();
    room >= need
}

/// Haxe `canEatObj` at default [`YUM_BONUS`].
pub fn can_eat_obj(
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
) -> bool {
    can_eat_obj_ex(
        food_value,
        count_eaten,
        food_store,
        food_store_max,
        YUM_BONUS,
    )
}

/// Haxe `canFeedToMeObj` with live YumBonus (meh feed only if starving ≤ 2).
pub fn can_feed_to_me_obj_with_yum(
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
    yum_bonus: f32,
) -> bool {
    if is_obj_meh_ex(food_value, count_eaten, yum_bonus)
        && food_store > MEH_FEED_REFUSE_FOOD_STORE
    {
        return false;
    }
    can_eat_obj_ex(
        food_value,
        count_eaten,
        food_store,
        food_store_max,
        yum_bonus,
    )
}

/// Haxe `canFeedToMeObj` at default YumBonus (no 837 gate).
pub fn can_feed_to_me_obj(
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
) -> bool {
    can_feed_to_me_obj_with_yum(
        food_value,
        count_eaten,
        food_store,
        food_store_max,
        YUM_BONUS,
    )
}

/// Haxe `canFeedToMeObj` with Psilocybe (837) + yellow-fever gate + live YumBonus.
pub fn can_feed_to_me_obj_ex_yum(
    food_parent_id: i32,
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
    has_yellow_fever: bool,
    yum_bonus: f32,
) -> bool {
    if food_parent_id == PSILOCYBE_MUSHROOM_ID && !has_yellow_fever {
        return false;
    }
    can_feed_to_me_obj_with_yum(
        food_value,
        count_eaten,
        food_store,
        food_store_max,
        yum_bonus,
    )
}

/// Haxe `canFeedToMeObj` with 837 gate at default YumBonus.
pub fn can_feed_to_me_obj_ex(
    food_parent_id: i32,
    food_value: i32,
    count_eaten: f32,
    food_store: f32,
    food_store_max: f32,
    has_yellow_fever: bool,
) -> bool {
    can_feed_to_me_obj_ex_yum(
        food_parent_id,
        food_value,
        count_eaten,
        food_store,
        food_store_max,
        has_yellow_fever,
        YUM_BONUS,
    )
}

/// Bowl of Gooseberries — skip eating a full bowl when not hungry.
// Haxe: AiBase.isEating L8804–8805
pub const BOWL_GOOSEBERRIES_EAT: i32 = 253;
/// Cooked Goose — keep one near home for knife craft.
// Haxe: AiBase.isEating L8808–8812
pub const COOKED_GOOSE_EAT: i32 = 518;
/// CountClose home r=20 for cooked goose.
// Haxe: AiBase.isEating L8810
pub const COOKED_GOOSE_KEEP_RADIUS: i32 = 20;
/// Wild Onion 808.
// Haxe: AiBase.isEating L8816
pub const WILD_ONION_EAT: i32 = 808;
/// Onion 2855.
// Haxe: AiBase.isEating L8816
pub const ONION_EAT: i32 = 2855;
/// Hot Pepper 2844.
// Haxe: AiBase.isEating L8822
pub const HOT_PEPPER_EAT: i32 = 2844;
/// Haxe `dropHeldObject(10)` after eating a peel leftover.
// Haxe: AiBase.isEating L8837
pub const EAT_PEEL_DROP_DIST: f32 = 10.0;

/// Haxe `isEating` conservation skips (full berry bowl / last goose / onion / pepper).
// Haxe: AiBase.isEating L8804–8822
#[inline]
pub fn is_eating_conservation_skip(
    is_hungry: bool,
    held_id: i32,
    held_uses: i32,
    held_num_uses: i32,
    cooked_goose_close: i32,
    has_onion_seeds: bool,
    has_pepper_seeds: bool,
) -> bool {
    if !is_hungry && held_id == BOWL_GOOSEBERRIES_EAT && held_uses >= held_num_uses.max(1) {
        return true;
    }
    if held_id == COOKED_GOOSE_EAT && cooked_goose_close < 1 {
        return true;
    }
    if (held_id == WILD_ONION_EAT || held_id == ONION_EAT) && !has_onion_seeds {
        return true;
    }
    if held_id == HOT_PEPPER_EAT && !has_pepper_seeds {
        return true;
    }
    false
}

/// After `self()` eat: drop leftover if remaining held `foodValue <= 0` (banana peel).
// Haxe: AiBase.isEating L8837
#[inline]
pub fn is_eating_drop_peel(food_value: i32) -> bool {
    food_value <= 0
}

/// Haxe `emptyContainer`: empty cargo → false; else dropHeld(0) or remove first slot.
// Haxe: AiBase.emptyContainer L8876–8892
#[inline]
pub fn empty_container_needs_drop(contained_len: i32, is_holding_object: bool) -> bool {
    contained_len >= 1 && is_holding_object
}

/// Haxe `emptyContainer` proceeds to `removeItemFromContainer` when cargo exists and drop missed.
// Haxe: AiBase.emptyContainer L8891
#[inline]
pub fn empty_container_should_remove(contained_len: i32) -> bool {
    contained_len >= 1
}

/// Haxe `isEating` enter gates (age, canEat, hungry-or-yum). Body after L8800 is the next hop.
// Haxe: AiBase.isEating L8796–8798
#[inline]
pub fn is_eating_head(
    age: f32,
    min_age_to_eat: f32,
    can_eat: bool,
    is_hungry: bool,
    is_holding_yum: bool,
) -> bool {
    if age < min_age_to_eat {
        return false;
    }
    if !can_eat {
        return false;
    }
    if !is_hungry && !is_holding_yum {
        return false;
    }
    true
}

/// Haxe starving cascade multiplier for food scoring.
// Haxe: processFood starving factor from food store
pub fn starving_factor(food_store: f32) -> f32 {
    if food_store < -1.5 {
        1.2
    } else if food_store < -1.0 {
        1.5
    } else if food_store < 0.5 {
        2.0
    } else if food_store < 3.0 {
        4.0
    } else {
        16.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starving_factor_cascade() {
        assert!((starving_factor(10.0) - 16.0).abs() < 1e-5);
        assert!((starving_factor(1.0) - 4.0).abs() < 1e-5);
        assert!((starving_factor(-2.0) - 1.2).abs() < 1e-5);
    }

    #[test]
    fn can_eat_refuses_zero_food() {
        assert!(!can_eat_obj(0, 0.0, 5.0, 20.0));
    }

    #[test]
    fn is_eating_head_age_caneat_hungry_or_yum() {
        // Haxe: AiBase.isEating L8796–8798
        assert!(!is_eating_head(2.0, 3.0, true, true, true));
        assert!(!is_eating_head(20.0, 3.0, false, true, true));
        assert!(!is_eating_head(20.0, 3.0, true, false, false));
        assert!(is_eating_head(20.0, 3.0, true, true, false));
        assert!(is_eating_head(20.0, 3.0, true, false, true));
    }

    #[test]
    fn is_eating_conservation_bowl_goose_onion_pepper() {
        // Full berry bowl, not hungry
        assert!(is_eating_conservation_skip(
            false, BOWL_GOOSEBERRIES_EAT, 5, 5, 2, true, true
        ));
        assert!(!is_eating_conservation_skip(
            true, BOWL_GOOSEBERRIES_EAT, 5, 5, 2, true, true
        ));
        // Last cooked goose (none on ground)
        assert!(is_eating_conservation_skip(
            true, COOKED_GOOSE_EAT, 1, 1, 0, true, true
        ));
        assert!(!is_eating_conservation_skip(
            true, COOKED_GOOSE_EAT, 1, 1, 1, true, true
        ));
        assert!(is_eating_conservation_skip(
            true, WILD_ONION_EAT, 1, 1, 0, false, true
        ));
        assert!(is_eating_conservation_skip(
            true, ONION_EAT, 1, 1, 0, false, true
        ));
        assert!(!is_eating_conservation_skip(
            true, WILD_ONION_EAT, 1, 1, 0, true, true
        ));
        assert!(is_eating_conservation_skip(
            true, HOT_PEPPER_EAT, 1, 1, 0, true, false
        ));
        assert!(!is_eating_conservation_skip(
            true, HOT_PEPPER_EAT, 1, 1, 0, true, true
        ));
        assert!(is_eating_drop_peel(0));
        assert!(!is_eating_drop_peel(3));
        assert_eq!(EAT_PEEL_DROP_DIST, 10.0);
        assert!(empty_container_needs_drop(1, true));
        assert!(!empty_container_needs_drop(1, false));
        assert!(!empty_container_needs_drop(0, true));
        assert!(empty_container_should_remove(2));
        assert!(!empty_container_should_remove(0));
    }
}
