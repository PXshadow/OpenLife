//! Sit / stand posture (Haxe emote-adjacent posture subset).

/// Food drain multiplier while sitting.
pub const SIT_FOOD_DRAIN_MULT: f32 = 0.75;

/// Whether sitting blocks MOVE (true = block).
pub const SIT_BLOCKS_MOVE: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sit_slows_hunger() {
        assert!(SIT_FOOD_DRAIN_MULT < 1.0);
        assert!(SIT_BLOCKS_MOVE);
    }
}
