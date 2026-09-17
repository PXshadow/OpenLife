//! Haxe `AiBase.isUsingItem` preflight (L8910–8998).
//!
//! Walk/use after this hop lives in `short_craft_intent::advance_use_held_staging`.

/// Milkweed 50 / Flowering 51 / Fruiting 52 — expected parent may change.
// Haxe: AiBase.isUsingItem L8924–8925
pub const MILKWEED_ID: i32 = 50;
pub const FLOWERING_MILKWEED_ID: i32 = 51;
pub const FRUITING_MILKWEED_ID: i32 = 52;
/// Bowl of Gooseberries 253.
// Haxe: AiBase.isUsingItem L8970
pub const USE_BOWL_GOOSEBERRIES: i32 = 253;
/// Wild Gooseberry Bush 30.
// Haxe: AiBase.isUsingItem L8976
pub const USE_WILD_BUSH: i32 = 30;
/// Domestic Gooseberry Bush 391.
// Haxe: AiBase.isUsingItem L8976
pub const USE_DOMESTIC_BUSH: i32 = 391;
/// Bowl of Dry Beans 1176.
// Haxe: AiBase.isUsingItem L8985
pub const USE_BOWL_DRY_BEANS: i32 = 1176;
/// Dry Bean Plants 1172.
// Haxe: AiBase.isUsingItem L8991
pub const USE_DRY_BEAN_PLANTS: i32 = 1172;
/// Bow and Arrow 152 — `killAnimal` when useTarget is an animal.
// Haxe: AiBase.isUsingItem L9016
pub const USE_BOW_AND_ARROW: i32 = 152;
/// Goose On Stump 1268 — `time -= 1` after successful use.
// Haxe: AiBase.isUsingItem L9101
pub const GOOSE_ON_STUMP: i32 = 1268;

/// Haxe milkweed family may change 50→51→52 without CancleUse.
// Haxe: AiBase.isUsingItem L8924–8925
#[inline]
pub fn is_milkweed_use_family(parent_id: i32) -> bool {
    matches!(
        parent_id,
        MILKWEED_ID | FLOWERING_MILKWEED_ID | FRUITING_MILKWEED_ID
    )
}

/// Haxe expectedUseTarget / isStillExpectedItem with milkweed exception.
// Haxe: AiBase.isUsingItem L8922–8936
#[inline]
pub fn is_using_item_target_still_expected(expected_parent: i32, world_parent: i32) -> bool {
    if world_parent == 0 {
        return false;
    }
    if expected_parent == 0 || expected_parent == world_parent {
        return true;
    }
    is_milkweed_use_family(expected_parent) && is_milkweed_use_family(world_parent)
}

/// Haxe `useIsDropInContainer == false && containedObjects.length > 0`.
// Haxe: AiBase.isUsingItem L8910
#[inline]
pub fn is_using_item_abort_contained(use_is_drop_in_container: bool, contained_n: i32) -> bool {
    !use_is_drop_in_container && contained_n > 0
}

/// Held 253 with remaining uses, target not a gooseberry bush.
// Haxe: AiBase.isUsingItem L8970–8976
#[inline]
pub fn is_using_item_need_fill_berry(
    held_id: i32,
    held_uses: i32,
    held_num_uses: i32,
    use_target_parent: i32,
) -> bool {
    if held_id != USE_BOWL_GOOSEBERRIES {
        return false;
    }
    if held_num_uses > 0 && held_uses >= held_num_uses {
        return false;
    }
    use_target_parent != USE_WILD_BUSH && use_target_parent != USE_DOMESTIC_BUSH
}

/// Held 1176 with remaining uses, target not dry bean plants.
// Haxe: AiBase.isUsingItem L8985–8991
#[inline]
pub fn is_using_item_need_fill_beans(
    held_id: i32,
    held_uses: i32,
    held_num_uses: i32,
    use_target_parent: i32,
) -> bool {
    if held_id != USE_BOWL_DRY_BEANS {
        return false;
    }
    if held_num_uses > 0 && held_uses >= held_num_uses {
        return false;
    }
    use_target_parent != USE_DRY_BEAN_PLANTS
}

/// Outcome of Haxe `isUsingItem` L8910–8998 (before goto/use).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsingItemPreflight {
    /// Continue walk/use.
    Continue,
    /// Nonempty container, not drop-in: mark notReachable, CancleUse, busy.
    AbortContainedBusy,
    /// Target changed / gone: CancleUse, not busy.
    CancelTargetChanged,
    /// Empty useActor, holding, considerDropHeldObject: busy drop.
    ConsiderDropHeld,
    /// Wrong non-empty actor: CancleUse + dropHeld, not busy.
    CancelWrongActor,
    /// Fill berry bowl (onlyFillHeld) then retry.
    NeedFillBerry,
    /// Fill dry bean bowl then retry.
    NeedFillBean,
}

/// Haxe `isUsingItem` L8910–8998.
pub fn using_item_preflight(
    use_is_drop_in_container: bool,
    contained_n: i32,
    expected_parent: i32,
    world_parent: i32,
    held_id: i32,
    use_actor_parent: i32,
    consider_drop: bool,
    held_uses: i32,
    held_num_uses: i32,
) -> UsingItemPreflight {
    if is_using_item_abort_contained(use_is_drop_in_container, contained_n) {
        return UsingItemPreflight::AbortContainedBusy;
    }
    if !is_using_item_target_still_expected(expected_parent, world_parent) {
        return UsingItemPreflight::CancelTargetChanged;
    }
    if held_id != use_actor_parent {
        if use_actor_parent == 0 {
            if held_id != 0 && consider_drop {
                return UsingItemPreflight::ConsiderDropHeld;
            }
        } else {
            return UsingItemPreflight::CancelWrongActor;
        }
    }
    if is_using_item_need_fill_berry(held_id, held_uses, held_num_uses, world_parent) {
        return UsingItemPreflight::NeedFillBerry;
    }
    if is_using_item_need_fill_beans(held_id, held_uses, held_num_uses, world_parent) {
        return UsingItemPreflight::NeedFillBean;
    }
    UsingItemPreflight::Continue
}

/// Haxe `heldObject.id == 152 && useTarget.isAnimal()`.
// Haxe: AiBase.isUsingItem L9016
#[inline]
pub fn is_using_item_bow_on_animal(held_id: i32, target_is_animal: bool) -> bool {
    held_id == USE_BOW_AND_ARROW && target_is_animal
}

/// Haxe `CalculateQuadDistanceToObject > 1` → gotoObj.
// Haxe: AiBase.isUsingItem L9020–9024
#[inline]
pub fn is_using_item_needs_goto(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    let dx = px - tx;
    let dy = py - ty;
    dx * dx + dy * dy > 1
}

/// After successful `use()`, `dropIsAUse` → CancleUse (no itemToCraft bump).
// Haxe: AiBase.isUsingItem L9061–9072
#[inline]
pub fn is_using_item_drop_is_a_use_done(drop_is_a_use: bool) -> bool {
    drop_is_a_use
}

/// Haxe `taregtObjectId == 1268` → `time -= 1`.
// Haxe: AiBase.isUsingItem L9101
#[inline]
pub fn is_using_item_goose_stump_speedup(ground_parent_after: i32) -> bool {
    ground_parent_after == GOOSE_ON_STUMP
}

/// Bookkeeping after a successful non-dropIsAUse `use()`.
///
/// Returns true when the sticky product appeared held or on the ground (`countDone++`).
// Haxe: AiBase.isUsingItem L9077–9089
pub fn note_using_item_craft_progress(
    product_id: i32,
    count_done: &mut i32,
    count_transitions_done: &mut i32,
    last_actor_id: &mut i32,
    last_target_id: &mut i32,
    last_new_actor_id: &mut i32,
    last_new_target_id: &mut i32,
    use_actor_id: i32,
    use_target_id: i32,
    held_parent_after: i32,
    ground_parent_after: i32,
) -> bool {
    if product_id <= 0 {
        return false;
    }
    *count_transitions_done = count_transitions_done.saturating_add(1);
    *last_actor_id = use_actor_id;
    *last_target_id = use_target_id;
    *last_new_actor_id = held_parent_after;
    *last_new_target_id = ground_parent_after;
    if held_parent_after == product_id || ground_parent_after == product_id {
        *count_done = count_done.saturating_add(1);
        true
    } else {
        false
    }
}

/// Haxe USE-fail: CancleUse + clear trans; Too hot / food short-circuit; else age mark.
// Haxe: AiBase.isUsingItem L9107–9137
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsingItemFailPlan {
    /// `message == 'Too hot!'` → `isHandlingTemperature = true; handleTemperature()`.
    pub handle_temperature: bool,
    /// `message.contains('food')` → `isHungry = true; return true`.
    pub set_hungry: bool,
    /// `age > 3` → `addNotReachableObject` (90s).
    pub mark_not_reachable: bool,
    /// `age <= 3` → `addObjectWithHostilePath`.
    pub mark_hostile: bool,
}

/// Parse Haxe `myPlayer.message` after `use()` returned false.
// Haxe: AiBase.isUsingItem L9125–9134
pub fn is_using_item_use_fail(age: f32, message: &str) -> UsingItemFailPlan {
    if message.eq_ignore_ascii_case("Too hot!") {
        return UsingItemFailPlan {
            handle_temperature: true,
            set_hungry: false,
            mark_not_reachable: false,
            mark_hostile: false,
        };
    }
    if message.to_ascii_lowercase().contains("food") {
        return UsingItemFailPlan {
            handle_temperature: false,
            set_hungry: true,
            mark_not_reachable: false,
            mark_hostile: false,
        };
    }
    let adult = age > 3.0;
    UsingItemFailPlan {
        handle_temperature: false,
        set_hungry: false,
        mark_not_reachable: adult,
        mark_hostile: !adult,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contained_abort_unless_drop_in_container() {
        assert!(is_using_item_abort_contained(false, 1));
        assert!(!is_using_item_abort_contained(true, 2));
        assert!(!is_using_item_abort_contained(false, 0));
        assert_eq!(
            using_item_preflight(false, 1, 292, 292, 33, 33, false, 1, 1),
            UsingItemPreflight::AbortContainedBusy
        );
        assert_eq!(
            using_item_preflight(true, 2, 292, 292, 126, 126, false, 1, 1),
            UsingItemPreflight::Continue
        );
    }

    #[test]
    fn milkweed_family_keeps_expected() {
        assert!(is_using_item_target_still_expected(50, 51));
        assert!(is_using_item_target_still_expected(51, 52));
        assert!(!is_using_item_target_still_expected(82, 85));
        assert!(!is_using_item_target_still_expected(82, 0));
        assert_eq!(
            using_item_preflight(false, 0, 50, 52, 0, 0, false, 0, 0),
            UsingItemPreflight::Continue
        );
        assert_eq!(
            using_item_preflight(false, 0, 82, 85, 0, 0, false, 0, 0),
            UsingItemPreflight::CancelTargetChanged
        );
    }

    #[test]
    fn wrong_actor_consider_drop_or_cancel() {
        assert_eq!(
            using_item_preflight(false, 0, 391, 391, 2144, 0, true, 1, 1),
            UsingItemPreflight::ConsiderDropHeld
        );
        assert_eq!(
            using_item_preflight(false, 0, 391, 391, 2144, 0, false, 1, 1),
            UsingItemPreflight::Continue
        );
        assert_eq!(
            using_item_preflight(false, 0, 82, 82, 33, 72, false, 1, 1),
            UsingItemPreflight::CancelWrongActor
        );
    }

    #[test]
    fn berry_and_bean_fill_gates() {
        assert!(is_using_item_need_fill_berry(253, 2, 5, 82));
        assert!(!is_using_item_need_fill_berry(253, 5, 5, 82));
        assert!(!is_using_item_need_fill_berry(253, 2, 5, 30));
        assert!(!is_using_item_need_fill_berry(253, 2, 5, 391));
        assert!(is_using_item_need_fill_beans(1176, 1, 3, 82));
        assert!(!is_using_item_need_fill_beans(1176, 1, 3, 1172));
        assert_eq!(
            using_item_preflight(false, 0, 82, 82, 253, 253, false, 1, 5),
            UsingItemPreflight::NeedFillBerry
        );
        assert_eq!(
            using_item_preflight(false, 0, 82, 82, 1176, 1176, false, 1, 3),
            UsingItemPreflight::NeedFillBean
        );
    }

    #[test]
    fn using_item_goto_bow_drop_use_and_craft_progress() {
        assert!(is_using_item_needs_goto(0, 0, 2, 0));
        assert!(!is_using_item_needs_goto(0, 0, 1, 0));
        assert!(is_using_item_needs_goto(0, 0, 1, 1)); // quad 2
        assert!(is_using_item_bow_on_animal(152, true));
        assert!(!is_using_item_bow_on_animal(152, false));
        assert!(!is_using_item_bow_on_animal(33, true));
        assert!(is_using_item_drop_is_a_use_done(true));
        assert!(!is_using_item_drop_is_a_use_done(false));
        assert!(is_using_item_goose_stump_speedup(1268));
        assert!(!is_using_item_goose_stump_speedup(82));
        let mut done = 0;
        let mut trans = 0;
        let mut la = -1;
        let mut lt = -1;
        let mut lna = -1;
        let mut lnt = -1;
        assert!(note_using_item_craft_progress(
            83, &mut done, &mut trans, &mut la, &mut lt, &mut lna, &mut lnt, 71, 72, 83, 0
        ));
        assert_eq!(done, 1);
        assert_eq!(trans, 1);
        assert_eq!(la, 71);
        assert!(!note_using_item_craft_progress(
            83, &mut done, &mut trans, &mut la, &mut lt, &mut lna, &mut lnt, 71, 72, 80, 78
        ));
        assert_eq!(done, 1);
        assert_eq!(trans, 2);
        assert!(!note_using_item_craft_progress(
            0, &mut done, &mut trans, &mut la, &mut lt, &mut lna, &mut lnt, 1, 2, 3, 4
        ));
    }

    #[test]
    fn using_item_use_fail_too_hot_food_and_age() {
        let hot = is_using_item_use_fail(20.0, "Too hot!");
        assert!(hot.handle_temperature);
        assert!(!hot.mark_not_reachable);
        let food = is_using_item_use_fail(20.0, "Need 2 more food!");
        assert!(food.set_hungry);
        assert!(!food.mark_not_reachable);
        let adult = is_using_item_use_fail(4.0, "");
        assert!(adult.mark_not_reachable);
        assert!(!adult.mark_hostile);
        let infant = is_using_item_use_fail(2.0, "");
        assert!(infant.mark_hostile);
        assert!(!infant.mark_not_reachable);
        assert!(!is_using_item_use_fail(20.0, "need more food").handle_temperature);
    }
}
