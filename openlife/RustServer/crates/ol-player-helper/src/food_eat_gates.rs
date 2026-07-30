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
}
