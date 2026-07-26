//! Curse tokens / curse score subset (Haxe Connection + OneLife `curses.cpp`).
//!
//! Wire tags (from `protocol.txt` / client `ClientTag`):
//! - `CX` — `CURSE_TOKEN_CHANGE` — player’s remaining curse token count
//! - `CS` — `CURSE_SCORE_CHANGE` — excess curse points above threshold
//!
//! No SQL / remote curse server: session-local map keyed by `p_id`.

// CX / CS wire builders live in ol-protocol for shared consistency.
pub use ol_protocol::{format_curse_score_change, format_curse_token_change};
use std::collections::HashMap;

/// OneLife default `curseThreshold` (score ≥ this → cursed with excess points).
pub const CURSE_THRESHOLD: i32 = 10;
/// New players start with one curse token (OneLife `findCurseRecord`).
pub const DEFAULT_CURSE_TOKENS: i32 = 1;

/// Per-player curse inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursePlayer {
    /// Spendable curse tokens (typically 0 or 1 in vanilla).
    pub tokens: i32,
    /// Raw curse points received (incremented when others curse this player).
    pub score: i32,
}

impl Default for CursePlayer {
    fn default() -> Self {
        Self {
            tokens: DEFAULT_CURSE_TOKENS,
            score: 0,
        }
    }
}

impl CursePlayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Excess curse points above threshold (OneLife `getCurseLevel` excessPoints).
    ///
    /// `0` when score &lt; threshold; else `(score - threshold) + 1`.
    pub fn excess(&self) -> i32 {
        compute_excess(self.score, CURSE_THRESHOLD)
    }

    /// Curse level floors: `score / threshold` when cursed, else `0`.
    pub fn curse_level(&self) -> i32 {
        if self.score < CURSE_THRESHOLD {
            0
        } else {
            self.score / CURSE_THRESHOLD
        }
    }

    pub fn is_cursed(&self) -> bool {
        self.curse_level() > 0
    }
}

/// `excess = 0` if `score < threshold`, else `(score - threshold) + 1`.
pub fn compute_excess(score: i32, threshold: i32) -> i32 {
    if score < threshold {
        0
    } else {
        (score - threshold) + 1
    }
}

/// Session curse state keyed by player id.
#[derive(Debug, Default, Clone)]
pub struct CurseState {
    pub entries: HashMap<i32, CursePlayer>,
}

impl CurseState {
    pub fn ensure(&mut self, p_id: i32) -> &mut CursePlayer {
        self.entries.entry(p_id).or_default()
    }

    pub fn get(&self, p_id: i32) -> Option<&CursePlayer> {
        self.entries.get(&p_id)
    }

    pub fn tokens(&self, p_id: i32) -> i32 {
        self.entries
            .get(&p_id)
            .map(|e| e.tokens)
            .unwrap_or(DEFAULT_CURSE_TOKENS)
    }

    pub fn score(&self, p_id: i32) -> i32 {
        self.entries.get(&p_id).map(|e| e.score).unwrap_or(0)
    }

    pub fn excess(&self, p_id: i32) -> i32 {
        self.entries
            .get(&p_id)
            .map(|e| e.excess())
            .unwrap_or(0)
    }

    /// Grant one token (regen path). Returns new token count.
    pub fn add_token(&mut self, p_id: i32) -> i32 {
        let e = self.ensure(p_id);
        e.tokens = e.tokens.saturating_add(1);
        e.tokens
    }

    /// Spend one token. Returns `false` if none available.
    pub fn spend_token(&mut self, p_id: i32) -> bool {
        let e = self.ensure(p_id);
        if e.tokens < 1 {
            return false;
        }
        e.tokens -= 1;
        true
    }

    /// Add raw curse score points to a player (receiver of a curse).
    pub fn add_score(&mut self, p_id: i32, amount: i32) -> i32 {
        if amount <= 0 {
            return self.score(p_id);
        }
        let e = self.ensure(p_id);
        e.score = e.score.saturating_add(amount);
        e.score
    }

    /// Spend giver’s token and +1 score on target. Fails if self-curse or no token.
    pub fn curse_player(&mut self, from: i32, to: i32) -> bool {
        if from == to {
            return false;
        }
        if !self.spend_token(from) {
            return false;
        }
        self.add_score(to, 1);
        true
    }

    /// `?CURSE` chat reply body (without leading p_id).
    pub fn format_query_text(&self, p_id: i32) -> String {
        let tokens = self.tokens(p_id);
        let score = self.score(p_id);
        let excess = compute_excess(score, CURSE_THRESHOLD);
        let level = if score < CURSE_THRESHOLD {
            0
        } else {
            score / CURSE_THRESHOLD
        };
        format!("CURSE tokens={tokens} score={score} excess={excess} level={level}")
    }

    /// Full `CX` wire packet for current token count.
    pub fn token_wire(&self, p_id: i32) -> String {
        format_curse_token_change(self.tokens(p_id))
    }

    /// Full `CS` wire packet for current excess points.
    pub fn score_wire(&self, p_id: i32) -> String {
        format_curse_score_change(self.excess(p_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_starts_with_one_token() {
        let mut s = CurseState::default();
        let e = s.ensure(1);
        assert_eq!(e.tokens, DEFAULT_CURSE_TOKENS);
        assert_eq!(e.score, 0);
        assert_eq!(e.excess(), 0);
        assert!(!e.is_cursed());
    }

    #[test]
    fn add_and_spend_token() {
        let mut s = CurseState::default();
        s.ensure(1);
        assert!(s.spend_token(1));
        assert_eq!(s.tokens(1), 0);
        assert!(!s.spend_token(1));
        assert_eq!(s.add_token(1), 1);
        assert_eq!(s.tokens(1), 1);
        assert_eq!(s.add_token(1), 2);
    }

    #[test]
    fn excess_above_threshold() {
        assert_eq!(compute_excess(0, CURSE_THRESHOLD), 0);
        assert_eq!(compute_excess(9, CURSE_THRESHOLD), 0);
        assert_eq!(compute_excess(10, CURSE_THRESHOLD), 1);
        assert_eq!(compute_excess(11, CURSE_THRESHOLD), 2);
        assert_eq!(compute_excess(20, CURSE_THRESHOLD), 11);

        let mut s = CurseState::default();
        s.ensure(2);
        for _ in 0..10 {
            s.add_score(2, 1);
        }
        assert_eq!(s.score(2), 10);
        assert_eq!(s.excess(2), 1);
        assert_eq!(s.get(2).unwrap().curse_level(), 1);
        assert!(s.get(2).unwrap().is_cursed());
    }

    #[test]
    fn curse_player_spends_and_scores() {
        let mut s = CurseState::default();
        s.ensure(1);
        s.ensure(2);
        assert!(s.curse_player(1, 2));
        assert_eq!(s.tokens(1), 0);
        assert_eq!(s.score(2), 1);
        assert!(!s.curse_player(1, 2)); // no tokens left
        assert!(!s.curse_player(1, 1)); // self
    }

    #[test]
    fn wire_cx_cs_shape() {
        let cx = format_curse_token_change(1);
        assert_eq!(cx, "CX\n1\n#");
        let cs = format_curse_score_change(3);
        assert_eq!(cs, "CS\n3\n#");

        let mut s = CurseState::default();
        s.ensure(5);
        s.add_score(5, 12);
        assert_eq!(s.token_wire(5), "CX\n1\n#");
        assert_eq!(s.score_wire(5), "CS\n3\n#"); // (12-10)+1 = 3
    }

    #[test]
    fn query_text() {
        let mut s = CurseState::default();
        s.ensure(1);
        s.spend_token(1);
        s.add_score(1, 15);
        let q = s.format_query_text(1);
        assert!(q.starts_with("CURSE "));
        assert!(q.contains("tokens=0"));
        assert!(q.contains("score=15"));
        assert!(q.contains("excess=6"));
        assert!(q.contains("level=1"));
    }
}
