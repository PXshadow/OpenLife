//! Posse join subset (Haxe `POSSE_JOIN` / `PJ` tag).
//!
//! Session-local map of killer → set of target player ids.
//! Protocol (`protocol.txt`):
//! - `PJ\n{killer} {target}\n#` — killer joined posse of target
//! - target `0` means killer left the posse (all targets cleared)
//!
//! No SQL.

use ol_protocol::format_server_message;
use std::collections::{HashMap, HashSet};

/// Session posse state: killer p_id → set of target p_ids.
#[derive(Debug, Default, Clone)]
pub struct PosseState {
    /// Killer → targets they are hunting with / posse-joined against.
    pub by_killer: HashMap<i32, HashSet<i32>>,
}

impl PosseState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Targets currently tracked for `killer` (empty if none).
    pub fn targets(&self, killer: i32) -> impl Iterator<Item = i32> + '_ {
        self.by_killer
            .get(&killer)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }

    /// Sorted list of targets for `killer`.
    pub fn targets_sorted(&self, killer: i32) -> Vec<i32> {
        let mut v: Vec<i32> = self.targets(killer).collect();
        v.sort_unstable();
        v
    }

    /// True when `killer` has `target` in their posse set.
    pub fn has_target(&self, killer: i32, target: i32) -> bool {
        self.by_killer
            .get(&killer)
            .map(|s| s.contains(&target))
            .unwrap_or(false)
    }

    /// Number of targets for `killer`.
    pub fn target_count(&self, killer: i32) -> usize {
        self.by_killer.get(&killer).map(|s| s.len()).unwrap_or(0)
    }

    /// Add `target` to `killer`'s posse set.
    ///
    /// Ignores self-target and non-positive target ids (use [`Self::clear`] to leave).
    /// Returns `true` if the target was newly added.
    pub fn add_posse(&mut self, killer: i32, target: i32) -> bool {
        if target <= 0 || killer == target {
            return false;
        }
        self.by_killer.entry(killer).or_default().insert(target)
    }

    /// Clear all posse targets for `killer`. Returns `true` if anything was removed.
    pub fn clear(&mut self, killer: i32) -> bool {
        match self.by_killer.remove(&killer) {
            Some(s) => !s.is_empty(),
            None => false,
        }
    }

    /// Remove a single target from killer's set. Returns `true` if it was present.
    pub fn remove_target(&mut self, killer: i32, target: i32) -> bool {
        let Some(set) = self.by_killer.get_mut(&killer) else {
            return false;
        };
        let removed = set.remove(&target);
        if set.is_empty() {
            self.by_killer.remove(&killer);
        }
        removed
    }

    /// `?POSSE` chat reply body (without leading player id).
    ///
    /// Lists caller's targets: `POSSE none` or `POSSE 2 5 9`.
    pub fn format_query_text(&self, killer: i32) -> String {
        let targets = self.targets_sorted(killer);
        if targets.is_empty() {
            return "POSSE none".into();
        }
        let parts: Vec<String> = targets.iter().map(|t| t.to_string()).collect();
        format!("POSSE {}", parts.join(" "))
    }

    /// Full `PJ` / `POSSE_JOIN` wire packet for this killer/target pair.
    pub fn join_wire(killer: i32, target: i32) -> String {
        format_posse_join(killer, target)
    }

    /// Leave-posse wire (`target = 0`).
    pub fn leave_wire(killer: i32) -> String {
        format_posse_join(killer, 0)
    }

    /// Drop all edges involving `p_id` as killer **or** target (death cleanup).
    ///
    /// Returns total edges removed (killer-map entries + target removals).
    pub fn prune_player(&mut self, p_id: i32) -> usize {
        let mut removed = 0usize;
        // As killer: whole set goes away.
        if let Some(set) = self.by_killer.remove(&p_id) {
            removed += set.len();
        }
        // As target on other killers.
        let killers: Vec<i32> = self.by_killer.keys().copied().collect();
        for k in killers {
            if self.remove_target(k, p_id) {
                removed += 1;
            }
        }
        removed
    }

    /// Keep only edges where both killer and target are in `alive`.
    /// Returns edges removed.
    pub fn prune_absent(&mut self, alive: &HashSet<i32>) -> usize {
        let mut removed = 0usize;
        let killers: Vec<i32> = self.by_killer.keys().copied().collect();
        for k in killers {
            if !alive.contains(&k) {
                if let Some(set) = self.by_killer.remove(&k) {
                    removed += set.len();
                }
                continue;
            }
            let Some(set) = self.by_killer.get_mut(&k) else {
                continue;
            };
            let before = set.len();
            set.retain(|t| alive.contains(t));
            removed += before.saturating_sub(set.len());
            if set.is_empty() {
                self.by_killer.remove(&k);
            }
        }
        removed
    }
}

/// `PJ` / `POSSE_JOIN` — `PJ\n{killer} {target}\n#`
///
/// `target == 0` means killer left the posse.
pub fn format_posse_join(killer: i32, target: i32) -> String {
    format_server_message("PJ", &[&format!("{killer} {target}")])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_posse_and_has_target() {
        let mut s = PosseState::new();
        assert!(!s.has_target(1, 2));
        assert!(s.add_posse(1, 2));
        assert!(s.has_target(1, 2));
        assert!(!s.add_posse(1, 2)); // already present
        assert!(s.add_posse(1, 3));
        assert_eq!(s.target_count(1), 2);
        assert_eq!(s.targets_sorted(1), vec![2, 3]);
    }

    #[test]
    fn add_posse_rejects_self_and_non_positive() {
        let mut s = PosseState::new();
        assert!(!s.add_posse(5, 5));
        assert!(!s.add_posse(5, 0));
        assert!(!s.add_posse(5, -1));
        assert_eq!(s.target_count(5), 0);
    }

    #[test]
    fn clear_removes_all() {
        let mut s = PosseState::new();
        s.add_posse(1, 2);
        s.add_posse(1, 3);
        assert!(s.clear(1));
        assert_eq!(s.target_count(1), 0);
        assert!(!s.has_target(1, 2));
        assert!(!s.clear(1)); // already empty
    }

    #[test]
    fn remove_single_target() {
        let mut s = PosseState::new();
        s.add_posse(1, 2);
        s.add_posse(1, 3);
        assert!(s.remove_target(1, 2));
        assert!(!s.has_target(1, 2));
        assert!(s.has_target(1, 3));
        assert!(s.remove_target(1, 3));
        assert!(!s.by_killer.contains_key(&1));
        assert!(!s.remove_target(1, 3));
    }

    #[test]
    fn format_query_lists_targets() {
        let mut s = PosseState::new();
        assert_eq!(s.format_query_text(1), "POSSE none");
        s.add_posse(1, 9);
        s.add_posse(1, 3);
        assert_eq!(s.format_query_text(1), "POSSE 3 9");
        assert_eq!(s.format_query_text(99), "POSSE none");
    }

    #[test]
    fn posse_join_wire_shape() {
        assert_eq!(format_posse_join(7, 12), "PJ\n7 12\n#");
        assert_eq!(format_posse_join(7, 0), "PJ\n7 0\n#");
        assert_eq!(PosseState::join_wire(1, 2), "PJ\n1 2\n#");
        assert_eq!(PosseState::leave_wire(1), "PJ\n1 0\n#");
    }

    #[test]
    fn prune_player_as_killer_and_target() {
        let mut s = PosseState::new();
        s.add_posse(1, 2);
        s.add_posse(1, 3);
        s.add_posse(4, 1);
        s.add_posse(4, 5);
        // Removes 2 (as killer) + 1 (as target of 4) = 3
        assert_eq!(s.prune_player(1), 3);
        assert_eq!(s.target_count(1), 0);
        assert!(!s.has_target(4, 1));
        assert!(s.has_target(4, 5));
        assert_eq!(s.prune_player(99), 0);
    }

    #[test]
    fn prune_absent_keeps_living_edges() {
        let mut s = PosseState::new();
        s.add_posse(1, 2);
        s.add_posse(1, 9);
        s.add_posse(9, 2);
        let alive: HashSet<i32> = [1, 2].into_iter().collect();
        let n = s.prune_absent(&alive);
        assert!(n >= 2);
        assert!(s.has_target(1, 2));
        assert!(!s.has_target(1, 9));
        assert!(!s.by_killer.contains_key(&9));
    }
}
