//! Ownership / gate lock permissions (Haxe ObjectHelper.owner subset).

use crate::ally::AllyState;

/// Whether `actor` may use/pass a locked owned object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Allowed,
    Denied,
}

/// Decide access for an owned gate/object.
///
/// - owner_id 0 → always allowed (unowned)
/// - actor == owner → allowed
/// - ally of owner (either direction) → allowed when `allies_pass`
/// - else denied when `locked`
pub fn check_owned_access(
    actor: i32,
    owner_id: i32,
    locked: bool,
    allies: &AllyState,
    allies_pass: bool,
) -> Access {
    if owner_id == 0 || !locked {
        return Access::Allowed;
    }
    if actor == owner_id {
        return Access::Allowed;
    }
    if allies_pass && allies.is_mutual_or_either(actor, owner_id) {
        return Access::Allowed;
    }
    Access::Denied
}

/// Format lock status chat body.
pub fn format_lock_query(owner_id: i32, locked: bool) -> String {
    format!(
        "LOCK owner={} locked={}",
        owner_id,
        if locked { 1 } else { 0 }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unowned_open() {
        let a = AllyState::default();
        assert_eq!(
            check_owned_access(1, 0, true, &a, true),
            Access::Allowed
        );
    }

    #[test]
    fn owner_passes_lock() {
        let a = AllyState::default();
        assert_eq!(
            check_owned_access(7, 7, true, &a, true),
            Access::Allowed
        );
    }

    #[test]
    fn stranger_denied_when_locked() {
        let a = AllyState::default();
        assert_eq!(
            check_owned_access(1, 2, true, &a, true),
            Access::Denied
        );
        assert_eq!(
            check_owned_access(1, 2, false, &a, true),
            Access::Allowed
        );
    }

    #[test]
    fn ally_passes() {
        let mut a = AllyState::default();
        a.add(1, 2).unwrap();
        assert_eq!(
            check_owned_access(1, 2, true, &a, true),
            Access::Allowed
        );
    }
}
