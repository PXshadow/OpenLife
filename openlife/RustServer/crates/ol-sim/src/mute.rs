//! Per-player chat mute list (pure — filter only; no wire yet).
//!
//! Each listener has a set of muted speaker `p_id`s. When delivering SAY /
//! WHISPER / SHOUT, callers should skip delivery if
//! [`MuteBook::is_muted`]`(listener, speaker)` is true.

use std::collections::{HashMap, HashSet};

/// Session-local mute graph: listener → set of muted speakers.
#[derive(Debug, Clone, Default)]
pub struct MuteBook {
    /// `listener_p_id → { muted_speaker_p_id, … }`
    muted: HashMap<i32, HashSet<i32>>,
}

impl MuteBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mute `speaker` for `listener`. Returns true if newly muted.
    ///
    /// Self-mute is a no-op (`false`).
    pub fn mute(&mut self, listener: i32, speaker: i32) -> bool {
        if listener == speaker {
            return false;
        }
        self.muted.entry(listener).or_default().insert(speaker)
    }

    /// Unmute `speaker` for `listener`. Returns true if was muted.
    pub fn unmute(&mut self, listener: i32, speaker: i32) -> bool {
        let Some(set) = self.muted.get_mut(&listener) else {
            return false;
        };
        let removed = set.remove(&speaker);
        if set.is_empty() {
            self.muted.remove(&listener);
        }
        removed
    }

    /// True if `listener` has muted `speaker`.
    pub fn is_muted(&self, listener: i32, speaker: i32) -> bool {
        self.muted
            .get(&listener)
            .map(|s| s.contains(&speaker))
            .unwrap_or(false)
    }

    /// Whether chat from `speaker` should be delivered to `listener`.
    ///
    /// Always true for self; false when muted.
    pub fn should_deliver(&self, listener: i32, speaker: i32) -> bool {
        listener == speaker || !self.is_muted(listener, speaker)
    }

    /// Sorted list of speakers muted by `listener`.
    pub fn list(&self, listener: i32) -> Vec<i32> {
        let mut v: Vec<i32> = self
            .muted
            .get(&listener)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        v.sort_unstable();
        v
    }

    /// Number of muted speakers for `listener`.
    pub fn count(&self, listener: i32) -> usize {
        self.muted.get(&listener).map(|s| s.len()).unwrap_or(0)
    }

    /// Drop all mute state for a player (as listener and as speaker).
    pub fn clear_player(&mut self, p_id: i32) {
        self.muted.remove(&p_id);
        for set in self.muted.values_mut() {
            set.remove(&p_id);
        }
        self.muted.retain(|_, s| !s.is_empty());
    }

    /// Total listeners with a non-empty mute set.
    pub fn listener_count(&self) -> usize {
        self.muted.len()
    }

    pub fn is_empty(&self) -> bool {
        self.muted.is_empty()
    }
}

/// Whether a listener should receive a chat volume under DEAF mode.
///
/// Deaf listeners drop normal / shout / mumble PS; whispers always deliver
/// (caller must set `is_whisper = true` only for the private WHISPER path).
pub fn should_hear(deaf: bool, is_whisper: bool) -> bool {
    is_whisper || !deaf
}

/// `MUTE none` or `MUTE 2 5 9` (sorted p_ids). Body without leading listener id.
pub fn format_mute_query(book: &MuteBook, listener: i32) -> String {
    let list = book.list(listener);
    if list.is_empty() {
        return "MUTE none".into();
    }
    let ids = list
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format!("MUTE {ids}")
}

/// Parse `MUTE <p_id>` / `UNMUTE <p_id>` / `MUTE LIST` tokens (case-insensitive cmd).
///
/// Returns `("mute"|"unmute"|"list", Option<p_id>)` or `None` if not a mute command.
pub fn parse_mute_command(text: &str) -> Option<(&'static str, Option<i32>)> {
    let t = text.trim();
    let mut parts = t.split_whitespace();
    let cmd = parts.next()?.to_ascii_uppercase();
    match cmd.as_str() {
        "MUTE" => {
            let rest = parts.next()?;
            if rest.eq_ignore_ascii_case("LIST") || rest.eq_ignore_ascii_case("?") {
                return Some(("list", None));
            }
            let id: i32 = rest.parse().ok()?;
            Some(("mute", Some(id)))
        }
        "UNMUTE" => {
            let id: i32 = parts.next()?.parse().ok()?;
            Some(("unmute", Some(id)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mute_unmute_and_deliver() {
        let mut book = MuteBook::new();
        assert!(book.should_deliver(1, 2));
        assert!(book.mute(1, 2));
        assert!(!book.mute(1, 2)); // already muted
        assert!(book.is_muted(1, 2));
        assert!(!book.should_deliver(1, 2));
        // reverse direction unaffected
        assert!(book.should_deliver(2, 1));
        assert!(book.unmute(1, 2));
        assert!(!book.is_muted(1, 2));
        assert!(book.should_deliver(1, 2));
    }

    #[test]
    fn no_self_mute() {
        let mut book = MuteBook::new();
        assert!(!book.mute(1, 1));
        assert!(book.should_deliver(1, 1));
        assert!(book.is_empty());
    }

    #[test]
    fn list_sorted_and_format() {
        let mut book = MuteBook::new();
        book.mute(1, 9);
        book.mute(1, 3);
        book.mute(1, 5);
        assert_eq!(book.list(1), vec![3, 5, 9]);
        assert_eq!(book.count(1), 3);
        let q = format_mute_query(&book, 1);
        assert_eq!(q, "MUTE 3 5 9");
        assert_eq!(format_mute_query(&book, 99), "MUTE none");
    }

    #[test]
    fn clear_player_both_sides() {
        let mut book = MuteBook::new();
        book.mute(1, 2);
        book.mute(3, 1);
        book.mute(3, 2);
        book.clear_player(1);
        assert!(!book.is_muted(1, 2));
        assert!(!book.is_muted(3, 1));
        assert!(book.is_muted(3, 2));
    }

    #[test]
    fn parse_commands() {
        assert_eq!(parse_mute_command("MUTE 12"), Some(("mute", Some(12))));
        assert_eq!(parse_mute_command("unmute 7"), Some(("unmute", Some(7))));
        assert_eq!(parse_mute_command("MUTE LIST"), Some(("list", None)));
        assert_eq!(parse_mute_command("mute ?"), Some(("list", None)));
        assert_eq!(parse_mute_command("hello"), None);
        assert_eq!(parse_mute_command("MUTE"), None);
        assert_eq!(parse_mute_command("MUTE abc"), None);
    }

    #[test]
    fn deaf_hears_whisper_only() {
        assert!(should_hear(false, false));
        assert!(should_hear(false, true));
        assert!(!should_hear(true, false));
        assert!(should_hear(true, true));
    }
}
