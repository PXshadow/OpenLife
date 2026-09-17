//! Haxe `AiBase.isRemovingFromContainer` / `removeItemFromContainer` (**AI-REMOVE-CONTAINER**).
//!
//! Sticky `removeFromContainerTarget` + `expectedContainer`. First tick only stages;
//! later ticks drop / goto / REMV. No world I/O.

/// Staged Haxe `removeFromContainerTarget` + `expectedContainer`.
// Haxe: AiBase.removeItemFromContainer ~1455; isRemovingFromContainer ~9140
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveFromContainerStaging {
    pub tx: i32,
    pub ty: i32,
    /// Haxe `expectedContainer.parentId` (set from `container.id` at stage).
    pub expected_parent: i32,
}

impl RemoveFromContainerStaging {
    pub fn new(tx: i32, ty: i32, expected_parent: i32) -> Self {
        Self {
            tx,
            ty,
            expected_parent,
        }
    }
}

/// Next step for a staged remove (Haxe `isRemovingFromContainer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveFromContainerAdvance {
    /// No sticky target — Haxe returns false.
    Idle,
    /// Expected parent gone/changed or cargo empty — clear, return false.
    Cancel,
    /// Hands full (not hidden wound) — `dropHeldObject()`, keep sticky.
    DropHeld,
    /// Path in progress.
    Wait,
    /// Quad dist > 1 — `gotoObj`; keep sticky.
    Goto { x: i32, y: i32 },
    /// `gotoObj` failed — clear, return false.
    GotoFailed { x: i32, y: i32 },
    /// Holding a player — `dropPlayer` at feet, keep sticky.
    DropPlayer,
    /// Adjacent empty hands — `myPlayer.remove` then always clear.
    RemvNow { x: i32, y: i32 },
}

impl RemoveFromContainerAdvance {
    pub fn clears_sticky(self) -> bool {
        matches!(
            self,
            Self::Cancel | Self::GotoFailed { .. } | Self::RemvNow { .. } | Self::Idle
        )
    }
}

/// Haxe `removeItemFromContainer`: skip notReachable / hostile; else stage.
// Haxe: AiBase.removeItemFromContainer ~1455–1463
pub fn stage_remove_item_from_container(
    tx: i32,
    ty: i32,
    expected_parent: i32,
    reachable: bool,
    hostile_path: bool,
) -> Option<RemoveFromContainerStaging> {
    if !reachable || hostile_path || expected_parent == 0 {
        return None;
    }
    Some(RemoveFromContainerStaging::new(tx, ty, expected_parent))
}

/// Haxe `AiHelper.isStillExpectedItem` — world parent still matches expected.
// Haxe: AiHelper.isStillExpectedItem ~589
#[inline]
pub fn is_still_expected_item(world_parent: i32, expected_parent: i32) -> bool {
    world_parent != 0 && world_parent == expected_parent
}

/// Pure `isRemovingFromContainer` body.
// Haxe: AiBase.isRemovingFromContainer ~9140–9225
pub fn advance_remove_from_container(
    staging: Option<RemoveFromContainerStaging>,
    world_parent: i32,
    contained_count: i32,
    held_id: i32,
    is_hidden_wound: bool,
    holding_player: bool,
    player_x: i32,
    player_y: i32,
    is_moving: bool,
    goto_ok: bool,
) -> RemoveFromContainerAdvance {
    let Some(st) = staging else {
        return RemoveFromContainerAdvance::Idle;
    };
    if !is_still_expected_item(world_parent, st.expected_parent) {
        return RemoveFromContainerAdvance::Cancel;
    }
    if contained_count < 1 {
        return RemoveFromContainerAdvance::Cancel;
    }
    // Haxe: heldObject.id != 0 && heldObject != hiddenWound → dropHeldObject()
    if held_id != 0 && !is_hidden_wound {
        return RemoveFromContainerAdvance::DropHeld;
    }
    if is_moving {
        return RemoveFromContainerAdvance::Wait;
    }
    let dx = st.tx - player_x;
    let dy = st.ty - player_y;
    if dx * dx + dy * dy > 1 {
        if goto_ok {
            return RemoveFromContainerAdvance::Goto { x: st.tx, y: st.ty };
        }
        return RemoveFromContainerAdvance::GotoFailed { x: st.tx, y: st.ty };
    }
    if holding_player {
        return RemoveFromContainerAdvance::DropPlayer;
    }
    RemoveFromContainerAdvance::RemvNow { x: st.tx, y: st.ty }
}

/// Haxe `myPlayer.remove(target.tx - gx, target.ty - gy)` (birth-relative REMV).
// Haxe: AiBase.isRemovingFromContainer L9211
#[inline]
pub fn remove_command_xy(target_tx: i32, target_ty: i32, gx: i32, gy: i32) -> (i32, i32) {
    (target_tx - gx, target_ty - gy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st() -> RemoveFromContainerStaging {
        RemoveFromContainerStaging::new(4, 0, 88)
    }

    #[test]
    fn stage_skips_blocked() {
        assert!(stage_remove_item_from_container(1, 1, 88, true, false).is_some());
        assert!(stage_remove_item_from_container(1, 1, 88, false, false).is_none());
        assert!(stage_remove_item_from_container(1, 1, 88, true, true).is_none());
        assert!(stage_remove_item_from_container(1, 1, 0, true, false).is_none());
    }

    #[test]
    fn idle_without_sticky() {
        assert_eq!(
            advance_remove_from_container(None, 88, 1, 0, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::Idle
        );
    }

    #[test]
    fn cancel_when_expected_gone_or_empty() {
        assert_eq!(
            advance_remove_from_container(Some(st()), 0, 1, 0, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::Cancel
        );
        assert_eq!(
            advance_remove_from_container(Some(st()), 89, 1, 0, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::Cancel
        );
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 0, 0, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::Cancel
        );
    }

    #[test]
    fn drop_held_keeps_sticky_except_hidden_wound() {
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 33, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::DropHeld
        );
        assert!(!RemoveFromContainerAdvance::DropHeld.clears_sticky());
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 33, true, false, 4, 0, false, true),
            RemoveFromContainerAdvance::RemvNow { x: 4, y: 0 }
        );
    }

    #[test]
    fn moving_waits_goto_when_far() {
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 4, 0, true, true),
            RemoveFromContainerAdvance::Wait
        );
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 0, 0, false, true),
            RemoveFromContainerAdvance::Goto { x: 4, y: 0 }
        );
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 0, 0, false, false),
            RemoveFromContainerAdvance::GotoFailed { x: 4, y: 0 }
        );
        assert!(RemoveFromContainerAdvance::GotoFailed { x: 4, y: 0 }.clears_sticky());
    }

    #[test]
    fn drop_player_then_remv() {
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, true, 4, 0, false, true),
            RemoveFromContainerAdvance::DropPlayer
        );
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 4, 0, false, true),
            RemoveFromContainerAdvance::RemvNow { x: 4, y: 0 }
        );
        // cardinal adjacent quad=1 is close enough
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 3, 0, false, true),
            RemoveFromContainerAdvance::RemvNow { x: 4, y: 0 }
        );
        // diagonal quad=2 → goto
        assert_eq!(
            advance_remove_from_container(Some(st()), 88, 1, 0, false, false, 3, 1, false, true),
            RemoveFromContainerAdvance::Goto { x: 4, y: 0 }
        );
        assert!(RemoveFromContainerAdvance::RemvNow { x: 4, y: 0 }.clears_sticky());
    }

    #[test]
    fn remove_command_xy_is_birth_relative() {
        // Haxe: L9211 target.tx - gx
        assert_eq!(remove_command_xy(110, 220, 100, 200), (10, 20));
        assert_eq!(remove_command_xy(5, 5, 0, 0), (5, 5));
    }
}
