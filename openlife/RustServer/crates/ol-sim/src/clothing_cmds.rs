//! Clothing equip / strip helpers — thin wrappers over [`Player`] methods.
//!
//! Prefer calling [`Player::wear_held`] / [`Player::strip_slot`] directly from
//! sim paths; this module keeps a pure-function style surface for tests and
//! callers that want `Result` without the swap tuple.
//!
//! Also re-exports **TH-CLOTHING-MATRIX** transition helpers (see
//! [`clothing_transitions`]).

// Haxe: GPI clothing transition matrix (TH-CLOTHING-MATRIX)
#[path = "clothing_transitions.rs"]
pub mod clothing_transitions;

// Re-export matrix API (root `clothing_transitions` also pub-used from lib).
#[allow(unused_imports)]
pub use clothing_transitions::{
    allow_reset_uses_on_target, apply_drink_self, apply_place_obj_in_clothing, apply_self_clothing,
    apply_sremv_from_clothing, apply_sremv_from_clothing_with_content, apply_switch_cloths,
    apply_switch_cloths_on_other, apply_transition_on_clothing, can_put_into_clothing,
    clothing_slot_from_def, crown_say_line, empty_hand_container_take_index,
    format_clothing_helper_string, format_clothing_set, get_clothing_slot_index, is_clothing_string,
    other_player_accepts_cloth, put_into_clothing_nest, refuse_take_permanent_contained,
    resolve_switch_slot, sremv_resolved_index, switch_clothing_index_full,
    take_from_clothing_nest, take_from_clothing_nest_checked, try_drink_water_pure,
    try_transition_on_clothing_pure, try_transition_on_clothing_with_content, ClothingSlotIds,
    ClothingTransitionIn, ClothingTransitionOut, DrinkWaterIn, DrinkWaterOut, SelfClothingPath,
    CLOTHING_INDEX_LABELS, EMPTY_BOWL_ID, EMPTY_POUCH_ID, MAX_AGE_CLOTH_OTHERS, MAX_STORED_WATER,
    TEMP_REDUCTION_PER_DRINK, WATER_BOWL_ID, WATER_POUCH_ID,
};

use crate::player::{ClothingSlot, Player};

/// Equip held into `slot` (swap previous into hands). Returns equipped id.
pub fn try_wear_held(
    player: &mut Player,
    slot: ClothingSlot,
) -> Result<i32, &'static str> {
    player.wear_held(slot).map(|(id, _)| id)
}

/// Strip slot into empty hands. Returns stripped id.
pub fn try_strip(player: &mut Player, slot: ClothingSlot) -> Result<i32, &'static str> {
    player.strip_slot(slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wear_and_strip() {
        let mut p = Player::new(1, 1, "a@b.c");
        p.held_id = 99;
        assert_eq!(try_wear_held(&mut p, ClothingSlot::Hat).unwrap(), 99);
        assert_eq!(p.hat, 99);
        assert_eq!(p.held_id, 0);
        assert_eq!(try_strip(&mut p, ClothingSlot::Hat).unwrap(), 99);
        assert_eq!(p.held_id, 99);
        assert_eq!(p.hat, 0);
    }

    #[test]
    fn strip_needs_empty_hands() {
        let mut p = Player::new(1, 1, "a@b.c");
        p.hat = 5;
        p.held_id = 1;
        assert_eq!(try_strip(&mut p, ClothingSlot::Hat), Err("HANDS"));
    }

    #[test]
    fn wear_swaps_previous_into_hands() {
        let mut p = Player::new(1, 1, "a@b.c");
        p.held_id = 10;
        assert_eq!(try_wear_held(&mut p, ClothingSlot::Chest).unwrap(), 10);
        p.held_id = 11;
        assert_eq!(try_wear_held(&mut p, ClothingSlot::Chest).unwrap(), 11);
        assert_eq!(p.chest, 11);
        assert_eq!(p.held_id, 10);
    }
}
