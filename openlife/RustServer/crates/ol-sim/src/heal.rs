//! Wound heal helpers (Haxe bandage / first-aid subset).

use crate::combat::CombatState;

/// Names that count as a healing item when held (case-insensitive contains).
pub const HEAL_NAME_HINTS: &[&str] = &["bandage", "kit", "poultice", "medicine", "herb"];

/// True if object name looks like a healing item.
pub fn name_looks_like_heal(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    HEAL_NAME_HINTS.iter().any(|h| n.contains(h))
}

/// Outcome of attempting a heal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealResult {
    /// No wound to clear.
    Healthy,
    /// Wound cleared (and optional item consumed).
    Healed { previous: u8 },
    /// Not allowed (no item when required).
    Denied,
}

/// Attempt to clear wounds for `p_id`.
///
/// - `require_item`: if true, `held_is_heal` must be true
/// - clears wound via [`CombatState::clear_wound`]
pub fn try_heal(
    combat: &mut CombatState,
    p_id: i32,
    held_is_heal: bool,
    require_item: bool,
) -> HealResult {
    let w = combat.wound_of(p_id);
    if w == 0 {
        return HealResult::Healthy;
    }
    if require_item && !held_is_heal {
        return HealResult::Denied;
    }
    combat.clear_wound(p_id);
    HealResult::Healed { previous: w }
}

/// Chat body for `?WOUND` without leading p_id.
pub fn format_wound_query(combat: &CombatState, p_id: i32) -> String {
    let w = combat.wound_of(p_id);
    let bleed = combat.bleed_drain(p_id);
    format!("WOUND level={w} bleed={bleed:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heal_clears_wound() {
        let mut c = CombatState::default();
        c.apply_wound(1, 2);
        assert_eq!(c.wound_of(1), 2);
        assert_eq!(
            try_heal(&mut c, 1, true, true),
            HealResult::Healed { previous: 2 }
        );
        assert_eq!(c.wound_of(1), 0);
    }

    #[test]
    fn denied_without_item() {
        let mut c = CombatState::default();
        c.apply_wound(1, 1);
        assert_eq!(try_heal(&mut c, 1, false, true), HealResult::Denied);
        assert_eq!(try_heal(&mut c, 1, false, false), HealResult::Healed { previous: 1 });
    }

    #[test]
    fn healthy_noop() {
        let mut c = CombatState::default();
        assert_eq!(try_heal(&mut c, 1, true, true), HealResult::Healthy);
    }

    #[test]
    fn name_hints() {
        assert!(name_looks_like_heal("Sterile Bandage"));
        assert!(!name_looks_like_heal("Stone"));
    }

    #[test]
    fn wound_query() {
        let mut c = CombatState::default();
        c.apply_wound(3, 2);
        let q = format_wound_query(&c, 3);
        assert!(q.contains("level=2"));
    }
}
