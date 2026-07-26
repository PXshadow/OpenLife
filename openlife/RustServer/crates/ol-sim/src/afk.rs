//! AFK / idle bookkeeping (pure — no network kick yet).
//!
//! Tracks last activity sim-time per player so a future disconnect path can
//! enforce [`DEFAULT_AFK_SECS`]. Chat, MOVE, USE, DROP, etc. should call
//! [`AfkBook::touch`].

use std::collections::HashMap;

/// Default idle seconds before a player is considered AFK (10 minutes).
pub const DEFAULT_AFK_SECS: f32 = 600.0;

/// Soft warn window: remaining seconds under which `?AFK` reports "warn".
pub const AFK_WARN_REMAINING_SECS: f32 = 60.0;

/// Per-player last activity stamp (sim seconds).
#[derive(Debug, Clone, Default)]
pub struct AfkBook {
    /// `p_id → last activity sim-time`.
    last: HashMap<i32, f32>,
}

impl AfkBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record activity at sim-time `now` for `p_id`.
    pub fn touch(&mut self, p_id: i32, now: f32) {
        self.last.insert(p_id, now);
    }

    /// Remove tracking when a player leaves.
    pub fn remove(&mut self, p_id: i32) {
        self.last.remove(&p_id);
    }

    /// Last activity sim-time, if known.
    pub fn last_activity(&self, p_id: i32) -> Option<f32> {
        self.last.get(&p_id).copied()
    }

    /// Seconds idle at `now` (0 if never tracked or clock went backwards).
    pub fn idle_secs(&self, p_id: i32, now: f32) -> f32 {
        match self.last.get(&p_id) {
            Some(&t) if now >= t => now - t,
            Some(_) => 0.0,
            None => 0.0,
        }
    }

    /// True if idle ≥ `timeout_secs` (and the player has been touched at least once).
    pub fn is_afk(&self, p_id: i32, now: f32, timeout_secs: f32) -> bool {
        if !self.last.contains_key(&p_id) {
            return false;
        }
        self.idle_secs(p_id, now) >= timeout_secs
    }

    /// True using [`DEFAULT_AFK_SECS`].
    pub fn is_afk_default(&self, p_id: i32, now: f32) -> bool {
        self.is_afk(p_id, now, DEFAULT_AFK_SECS)
    }

    /// Seconds remaining until AFK under `timeout_secs` (0 if already AFK / unknown).
    pub fn remaining_secs(&self, p_id: i32, now: f32, timeout_secs: f32) -> f32 {
        if !self.last.contains_key(&p_id) {
            return timeout_secs.max(0.0);
        }
        (timeout_secs - self.idle_secs(p_id, now)).max(0.0)
    }

    /// Collect p_ids that are AFK at `now` under `timeout_secs`.
    pub fn afk_players(&self, now: f32, timeout_secs: f32) -> Vec<i32> {
        let mut out: Vec<i32> = self
            .last
            .keys()
            .copied()
            .filter(|&id| self.is_afk(id, now, timeout_secs))
            .collect();
        out.sort_unstable();
        out
    }

    pub fn len(&self) -> usize {
        self.last.len()
    }

    pub fn is_empty(&self) -> bool {
        self.last.is_empty()
    }
}

/// `AFK idle=N.N remain=N.N status=ok|warn|afk` body (no leading p_id).
pub fn format_afk_query(book: &AfkBook, p_id: i32, now: f32, timeout_secs: f32) -> String {
    let idle = book.idle_secs(p_id, now);
    let remain = book.remaining_secs(p_id, now, timeout_secs);
    let status = if idle >= timeout_secs {
        "afk"
    } else if remain <= AFK_WARN_REMAINING_SECS {
        "warn"
    } else {
        "ok"
    };
    format!("AFK idle={idle:.1} remain={remain:.1} status={status}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_and_idle() {
        let mut book = AfkBook::new();
        book.touch(1, 10.0);
        assert_eq!(book.idle_secs(1, 10.0), 0.0);
        assert_eq!(book.idle_secs(1, 70.0), 60.0);
        assert!(!book.is_afk(1, 70.0, 600.0));
        assert!(book.is_afk(1, 620.0, 600.0));
        assert!(book.is_afk_default(1, 620.0));
    }

    #[test]
    fn unknown_player_not_afk() {
        let book = AfkBook::new();
        assert!(!book.is_afk(99, 1000.0, 10.0));
        assert_eq!(book.idle_secs(99, 1000.0), 0.0);
        assert_eq!(book.remaining_secs(99, 0.0, 600.0), 600.0);
    }

    #[test]
    fn remove_and_list() {
        let mut book = AfkBook::new();
        book.touch(2, 0.0);
        book.touch(3, 0.0);
        book.touch(1, 500.0);
        let afk = book.afk_players(700.0, 600.0);
        assert_eq!(afk, vec![2, 3]);
        book.remove(2);
        assert!(!book.is_afk(2, 700.0, 600.0));
        assert_eq!(book.len(), 2);
    }

    #[test]
    fn format_status_ok_warn_afk() {
        let mut book = AfkBook::new();
        book.touch(1, 0.0);
        let ok = format_afk_query(&book, 1, 10.0, 600.0);
        assert!(ok.contains("status=ok"), "{ok}");
        let warn = format_afk_query(&book, 1, 550.0, 600.0);
        assert!(warn.contains("status=warn"), "{warn}");
        assert!(warn.contains("remain=50.0"), "{warn}");
        let afk = format_afk_query(&book, 1, 700.0, 600.0);
        assert!(afk.contains("status=afk"), "{afk}");
        assert!(afk.contains("remain=0.0"), "{afk}");
    }

    #[test]
    fn backwards_clock_yields_zero_idle() {
        let mut book = AfkBook::new();
        book.touch(1, 100.0);
        assert_eq!(book.idle_secs(1, 50.0), 0.0);
    }
}
