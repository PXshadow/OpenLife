//! JUMP-BW-FULL — pure helpers re-exported from `ol-move-rules`.
//!
//! Live wire lives in `lib.rs` (`apply_player_jump`, JUMP client tag, AI JUMP!, MOVE jump gates).
//!
//! # Haxe anchors
//! - `GlobalPlayerInstance.jump` L5098–5120 — not held → PU + BW + FRAME; held → dropPlayer
//! - `MoveHelper.JumpToNonBlocked` L473–519 — blocked standing tile → E/S/W/N try + VOG force
//! - `MoveHelper.moveHelper` L606–626 — rate limit, exhaustion, jumpedTiles, exhausted say

pub use ol_move_rules::{
    jump_not_held_emits_bw, jump_should_say_exhausted, plan_jump_to_non_blocked, plan_player_jump,
    JumpAction, JUMP_EXHAUSTED_SAY, JUMP_TO_NON_BLOCKED_OFFSETS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_jump_wiggle_when_not_held() {
        assert_eq!(plan_player_jump(0), JumpAction::Wiggle);
        assert!(jump_not_held_emits_bw());
    }

    #[test]
    fn plan_jump_drop_when_held() {
        assert_eq!(
            plan_player_jump(42),
            JumpAction::DropFromArms { carrier_p_id: 42 }
        );
    }

    #[test]
    fn jump_to_non_blocked_not_needed() {
        let blocked = |x: i32, y: i32| x == 5 && y == 5;
        assert_eq!(plan_jump_to_non_blocked(blocked, 0, 0), None);
    }

    #[test]
    fn jump_to_non_blocked_prefers_east() {
        let blocked = |x: i32, y: i32| x == 0 && y == 0;
        assert_eq!(plan_jump_to_non_blocked(blocked, 0, 0), Some((1, 0)));
    }

    #[test]
    fn jump_exhausted_say_gate() {
        assert!(jump_should_say_exhausted(true));
        assert!(!jump_should_say_exhausted(false));
        assert_eq!(JUMP_EXHAUSTED_SAY, "I am too exhausted!");
    }
}
