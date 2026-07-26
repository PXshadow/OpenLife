//! Clothing equip / strip helpers — thin wrappers over [`Player`] methods.
//!
//! Prefer calling [`Player::wear_held`] / [`Player::strip_slot`] directly from
//! sim paths; this module keeps a pure-function style surface for tests and
//! callers that want `Result` without the swap tuple.

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
