// JUMP-BW-FULL pure helpers (included from move_path so unit tests compile without lib mod wire).
// Canonical docs/module: jump_bw.rs (same symbols).

/// Haxe `JumpToNonBlocked` probe order: +x, −y, −x, +y (E, S, W, N).
// Haxe: MoveHelper.JumpToNonBlocked L482–489
pub const JUMP_TO_NON_BLOCKED_OFFSETS: [(i32, i32); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];

/// Haxe exhausted MOVE-jump spoken line.
// Haxe: MoveHelper.moveHelper L626
pub const JUMP_EXHAUSTED_SAY: &str = "I am too exhausted!";

/// Plan for Haxe `GlobalPlayerInstance.jump`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpAction {
    /// Not held: emit PU + BW + FRAME.
    Wiggle,
    /// Held by carrier: drop out of arms.
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

/// Haxe always calls `sendWiggle` on not-held jump.
#[inline]
pub fn jump_not_held_emits_bw() -> bool {
    true
}

/// Plan `JumpToNonBlocked`. `None` = not blocked; `Some((dx,dy))` = was blocked.
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
    Some((0, 0))
}

/// True when post-jump exhaustion should trigger the spoken line.
#[inline]
pub fn jump_should_say_exhausted(is_exhausted: bool) -> bool {
    is_exhausted
}
