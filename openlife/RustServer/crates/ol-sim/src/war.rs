//! War / alliance status subset (Haxe `WAR_REPORT` / `WR` tag).
//!
//! Session-local map of undirected player pairs → status string:
//! `Peace` / `War` / `Alliance`. No SQL.

use ol_protocol::format_server_message;
use std::collections::HashMap;

/// Default / neutral status (missing pairs are treated as Peace).
pub const STATUS_PEACE: &str = "Peace";
/// Active war between a pair.
pub const STATUS_WAR: &str = "War";
/// Friendly alliance between a pair.
pub const STATUS_ALLIANCE: &str = "Alliance";

/// Canonical undirected pair key: `(min(a,b), max(a,b))`.
#[inline]
pub fn pair_key(a: i32, b: i32) -> (i32, i32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Session war/alliance state.
#[derive(Debug, Default, Clone)]
pub struct WarState {
    /// Undirected `(a, b)` → status (`Peace` / `War` / `Alliance`).
    pub pairs: HashMap<(i32, i32), String>,
}

impl WarState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current status for a pair (defaults to Peace when unset).
    pub fn status(&self, a: i32, b: i32) -> &str {
        if a == b {
            return STATUS_PEACE;
        }
        self.pairs
            .get(&pair_key(a, b))
            .map(|s| s.as_str())
            .unwrap_or(STATUS_PEACE)
    }

    /// Set status string for a pair. Self-pairs are ignored.
    /// Returns the stored status (or Peace for self).
    pub fn set_status(&mut self, a: i32, b: i32, status: impl Into<String>) -> String {
        if a == b {
            return STATUS_PEACE.to_string();
        }
        let s = status.into();
        let key = pair_key(a, b);
        if s == STATUS_PEACE {
            self.pairs.remove(&key);
            STATUS_PEACE.to_string()
        } else {
            self.pairs.insert(key, s.clone());
            s
        }
    }

    /// Declare war between `a` and `b`. Returns false for self-target.
    pub fn declare_war(&mut self, a: i32, b: i32) -> bool {
        if a == b {
            return false;
        }
        self.set_status(a, b, STATUS_WAR);
        true
    }

    /// Make peace between `a` and `b` (status → Peace). Returns false for self.
    pub fn make_peace(&mut self, a: i32, b: i32) -> bool {
        if a == b {
            return false;
        }
        self.set_status(a, b, STATUS_PEACE);
        true
    }

    /// Form an alliance between `a` and `b`. Returns false for self.
    pub fn make_alliance(&mut self, a: i32, b: i32) -> bool {
        if a == b {
            return false;
        }
        self.set_status(a, b, STATUS_ALLIANCE);
        true
    }

    /// True when the pair is currently at War.
    pub fn is_at_war(&self, a: i32, b: i32) -> bool {
        self.status(a, b) == STATUS_WAR
    }

    /// True when the pair is currently Allied.
    pub fn is_allied(&self, a: i32, b: i32) -> bool {
        self.status(a, b) == STATUS_ALLIANCE
    }

    /// Non-Peace pairs involving `p_id` (or all pairs if `p_id` is None).
    pub fn list_relations(&self, p_id: Option<i32>) -> Vec<(i32, i32, String)> {
        let mut out: Vec<(i32, i32, String)> = self
            .pairs
            .iter()
            .filter(|((a, b), status)| {
                if status.as_str() == STATUS_PEACE {
                    return false;
                }
                match p_id {
                    Some(id) => *a == id || *b == id,
                    None => true,
                }
            })
            .map(|((a, b), s)| (*a, *b, s.clone()))
            .collect();
        out.sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)));
        out
    }

    /// Drop every pair involving `p_id` (death / session cleanup).
    ///
    /// Returns the number of pairs removed. Self-only / missing → 0.
    pub fn prune_player(&mut self, p_id: i32) -> usize {
        let before = self.pairs.len();
        self.pairs.retain(|(a, b), _| *a != p_id && *b != p_id);
        before.saturating_sub(self.pairs.len())
    }

    /// Keep only pairs where both ends are in `alive`. Returns pairs removed.
    pub fn prune_absent(&mut self, alive: &std::collections::HashSet<i32>) -> usize {
        let before = self.pairs.len();
        self.pairs
            .retain(|(a, b), _| alive.contains(a) && alive.contains(b));
        before.saturating_sub(self.pairs.len())
    }

    /// `?WAR` chat reply body (without leading player id).
    ///
    /// Lists all non-Peace pairs: `WAR a b War; c d Alliance` or `WAR none`.
    pub fn format_query_text(&self) -> String {
        let rels = self.list_relations(None);
        if rels.is_empty() {
            return "WAR none".into();
        }
        let parts: Vec<String> = rels
            .iter()
            .map(|(a, b, s)| format!("{a} {b} {s}"))
            .collect();
        format!("WAR {}", parts.join("; "))
    }

    /// Full `WR` / `WAR_REPORT` wire packet: `WR\n{a} {b} {status}\n#`.
    pub fn war_report_wire(a: i32, b: i32, status: &str) -> String {
        format_war_report(a, b, status)
    }
}

/// `WR` / `WAR_REPORT` — `WR\n{a} {b} {status}\n#`
pub fn format_war_report(a: i32, b: i32, status: &str) -> String {
    format_server_message("WR", &[&format!("{a} {b} {status}")])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_key_orders() {
        assert_eq!(pair_key(5, 2), (2, 5));
        assert_eq!(pair_key(2, 5), (2, 5));
        assert_eq!(pair_key(3, 3), (3, 3));
    }

    #[test]
    fn declare_war_and_is_at_war() {
        let mut w = WarState::new();
        assert!(!w.is_at_war(1, 2));
        assert_eq!(w.status(1, 2), STATUS_PEACE);
        assert!(w.declare_war(1, 2));
        assert!(w.is_at_war(1, 2));
        assert!(w.is_at_war(2, 1)); // undirected
        assert_eq!(w.status(2, 1), STATUS_WAR);
        assert!(!w.declare_war(1, 1)); // self
        assert!(!w.is_at_war(1, 1));
    }

    #[test]
    fn make_peace() {
        let mut w = WarState::new();
        w.declare_war(10, 20);
        assert!(w.is_at_war(10, 20));
        assert!(w.make_peace(20, 10));
        assert!(!w.is_at_war(10, 20));
        assert_eq!(w.status(10, 20), STATUS_PEACE);
        assert!(!w.pairs.contains_key(&pair_key(10, 20)));
        assert!(!w.make_peace(5, 5));
    }

    #[test]
    fn alliance_status() {
        let mut w = WarState::new();
        assert!(w.make_alliance(1, 3));
        assert_eq!(w.status(1, 3), STATUS_ALLIANCE);
        assert!(w.is_allied(3, 1));
        assert!(!w.is_at_war(1, 3));
        w.declare_war(1, 3);
        assert!(w.is_at_war(1, 3));
        assert!(!w.is_allied(1, 3));
    }

    #[test]
    fn format_query_lists_relations() {
        let mut w = WarState::new();
        assert_eq!(w.format_query_text(), "WAR none");
        w.declare_war(2, 5);
        w.make_alliance(1, 3);
        let q = w.format_query_text();
        assert!(q.starts_with("WAR "));
        assert!(q.contains("1 3 Alliance"));
        assert!(q.contains("2 5 War"));
    }

    #[test]
    fn war_report_wire_shape() {
        let wr = format_war_report(1, 2, STATUS_WAR);
        assert_eq!(wr, "WR\n1 2 War\n#");
        assert_eq!(
            WarState::war_report_wire(3, 4, STATUS_PEACE),
            "WR\n3 4 Peace\n#"
        );
    }

    #[test]
    fn prune_player_drops_pairs() {
        let mut w = WarState::new();
        w.declare_war(1, 2);
        w.make_alliance(1, 3);
        w.declare_war(2, 3);
        assert_eq!(w.prune_player(1), 2);
        assert!(!w.is_at_war(1, 2));
        assert!(!w.is_allied(1, 3));
        assert!(w.is_at_war(2, 3));
        assert_eq!(w.prune_player(99), 0);
    }

    #[test]
    fn prune_absent_keeps_living_pairs() {
        use std::collections::HashSet;
        let mut w = WarState::new();
        w.declare_war(1, 2);
        w.declare_war(3, 4);
        let alive: HashSet<i32> = [1, 2].into_iter().collect();
        assert_eq!(w.prune_absent(&alive), 1);
        assert!(w.is_at_war(1, 2));
        assert!(!w.is_at_war(3, 4));
    }
}
