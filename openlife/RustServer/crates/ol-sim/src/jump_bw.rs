//! JUMP-BW-FULL — pure helpers for Haxe `GlobalPlayerInstance.jump` + `MoveHelper.JumpToNonBlocked`
//! and MOVE jump exhaustion say.
//!
//! Live wire lives in `lib.rs` (`apply_player_jump`, JUMP client tag, AI JUMP!, MOVE jump gates).
//!
//! # Haxe anchors
//! - `GlobalPlayerInstance.jump` L5098–5120 — not held → PU + BW + FRAME; held → dropPlayer
//! - `MoveHelper.JumpToNonBlocked` L473–519 — blocked standing tile → E/S/W/N try + VOG force
//! - `MoveHelper.moveHelper` L606–626 — rate limit, exhaustion, jumpedTiles, exhausted say
//! - `Connection.sendWiggle` — BABY_WIGGLE (BW) packet
//! - AI `sayHelper` JUMP! → `myPlayer.jump()`

/// Haxe `JumpToNonBlocked` probe order: +x, −y, −x, +y (E, S, W, N).
// Haxe: MoveHelper.JumpToNonBlocked L482–489
pub const JUMP_TO_NON_BLOCKED_OFFSETS: [(i32, i32); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];

/// Haxe exhausted MOVE-jump spoken line.
// Haxe: MoveHelper.moveHelper L626 `p.say('I am too exhausted!', true)`
pub const JUMP_EXHAUSTED_SAY: &str = "I am too exhausted!";

/// Plan for Haxe `GlobalPlayerInstance.jump`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpAction {
    /// Not held: emit PU + BW (wiggle) + FRAME. x/y coords ignored.
    // Haxe: jump L5103–5110
    Wiggle,
    /// Held by `carrier_p_id`: drop out of arms (dropPlayer at carrier feet).
    // Haxe: jump L5116 heldByPlayer.dropPlayer
    DropFromArms { carrier_p_id: i32 },
}

/// Resolve Haxe `jump()` action from held-by link.
// Haxe: GlobalPlayerInstance.jump
#[inline]
pub fn plan_player_jump(held_by: i32) -> JumpAction {
    if held_by == 0 {
        JumpAction::Wiggle
    } else {
        JumpAction::DropFromArms {
            carrier_p_id: held_by,
        }
    }
}

/// Haxe always calls `sendWiggle` on not-held jump (age not checked).
// Haxe: Connection.sendWiggle via jump L5105
#[inline]
pub fn jump_not_held_emits_bw() -> bool {
    true
}

/// Plan `JumpToNonBlocked`.
///
/// - `None` — standing tile not blocked; MOVE continues.
/// - `Some((dx, dy))` — was blocked; first free neighbor offset, or `(0,0)` if all blocked
///   (Haxe still forces VOG/PU and aborts MOVE).
// Haxe: MoveHelper.JumpToNonBlocked L473–519
pub fn plan_jump_to_non_blocked(
    is_blocked: impl Fn(i32, i32) -> bool,
    tx: i32,
    ty: i32,
) -> Option<(i32, i32)> {
    if !is_blocked(tx, ty) {
        return None;
    }
    for &(dx, dy) in &JUMP_TO_NON_BLOCKED_OFFSETS {
        if !is_blocked(tx + dx, ty + dy) {
            return Some((dx, dy));
        }
    }
    // Originally blocked; no free neighbor — stay put but still force-sync.
    Some((0, 0))
}

/// True when post-jump exhaustion should trigger the spoken line.
// Haxe: MoveHelper.moveHelper L623–626
#[inline]
pub fn jump_should_say_exhausted(is_exhausted: bool) -> bool {
    is_exhausted
}

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
        // Center blocked; east free.
        let blocked = |x: i32, y: i32| x == 0 && y == 0;
        assert_eq!(plan_jump_to_non_blocked(blocked, 0, 0), Some((1, 0)));
    }

    #[test]
    fn jump_to_non_blocked_order_eswn() {
        // Center + east blocked → south (0,-1)
        let blocked = |x: i32, y: i32| (x == 0 && y == 0) || (x == 1 && y == 0);
        assert_eq!(plan_jump_to_non_blocked(blocked, 0, 0), Some((0, -1)));
        // Also block south → west
        let blocked2 =
            |x: i32, y: i32| (x == 0 && y == 0) || (x == 1 && y == 0) || (x == 0 && y == -1);
        assert_eq!(plan_jump_to_non_blocked(blocked2, 0, 0), Some((-1, 0)));
        // Block west too → north
        let blocked3 = |x: i32, y: i32| {
            (x == 0 && y == 0)
                || (x == 1 && y == 0)
                || (x == 0 && y == -1)
                || (x == -1 && y == 0)
        };
        assert_eq!(plan_jump_to_non_blocked(blocked3, 0, 0), Some((0, 1)));
    }

    #[test]
    fn jump_to_non_blocked_all_blocked_stays() {
        let blocked = |_x: i32, _y: i32| true;
        assert_eq!(plan_jump_to_non_blocked(blocked, 3, 4), Some((0, 0)));
    }

    #[test]
    fn exhausted_say_gate() {
        assert!(jump_should_say_exhausted(true));
        assert!(!jump_should_say_exhausted(false));
        assert_eq!(JUMP_EXHAUSTED_SAY, "I am too exhausted!");
    }
}
