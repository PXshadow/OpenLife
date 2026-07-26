//! Newbie tutorial / life tips (Haxe tutorial flags subset).
//!
//! Tracks which tip ids a player has already seen so SAY HELP tips can advance.

use std::collections::{HashMap, HashSet};

/// Ordered tip texts for first-life guidance.
pub const TIPS: &[&str] = &[
    "TIP0 Eat berries when food is low (USE on food or SAY FEED).",
    "TIP1 DROP places items; REMV takes from baskets.",
    "TIP2 SAY HOME sets home; GOHOME walks toward it.",
    "TIP3 FOLLOW a leader; EXILE marks enemies for legal kills.",
    "TIP4 CRAFT with empty tile USE recipes; STORE backpacks items.",
    "TIP5 SAY ?HELP lists commands; WHISPER and SHOUT change range.",
];

/// Per-player tutorial progress.
#[derive(Debug, Clone, Default)]
pub struct TutorialProgress {
    pub seen: HashSet<u8>,
    pub next_index: u8,
}

impl TutorialProgress {
    /// Mark tip index seen and advance.
    pub fn advance(&mut self) -> Option<&'static str> {
        let i = self.next_index as usize;
        if i >= TIPS.len() {
            return None;
        }
        self.seen.insert(self.next_index);
        let tip = TIPS[i];
        self.next_index = self.next_index.saturating_add(1);
        Some(tip)
    }

    pub fn peek(&self) -> Option<&'static str> {
        let i = self.next_index as usize;
        if i >= TIPS.len() {
            None
        } else {
            Some(TIPS[i])
        }
    }

    pub fn done(&self) -> bool {
        self.next_index as usize >= TIPS.len()
    }
}

/// Session map p_id → tutorial.
#[derive(Debug, Default, Clone)]
pub struct TutorialState {
    pub by_player: HashMap<i32, TutorialProgress>,
}

impl TutorialState {
    pub fn progress_mut(&mut self, p_id: i32) -> &mut TutorialProgress {
        self.by_player.entry(p_id).or_default()
    }

    /// Next tip for player (does not advance).
    pub fn format_tip_query(&self, p_id: i32) -> String {
        match self.by_player.get(&p_id) {
            Some(p) if p.done() => "TIP done".into(),
            Some(p) => p.peek().unwrap_or("TIP done").to_string(),
            None => TIPS[0].to_string(),
        }
    }

    /// Advance and return tip body.
    pub fn take_tip(&mut self, p_id: i32) -> String {
        match self.progress_mut(p_id).advance() {
            Some(t) => t.to_string(),
            None => "TIP done".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_through_tips() {
        let mut t = TutorialState::default();
        let a = t.take_tip(1);
        assert!(a.starts_with("TIP0"));
        let b = t.take_tip(1);
        assert!(b.starts_with("TIP1"));
        for _ in 0..10 {
            let _ = t.take_tip(1);
        }
        assert_eq!(t.take_tip(1), "TIP done");
        assert_eq!(t.format_tip_query(1), "TIP done");
    }

    #[test]
    fn peek_without_advance() {
        let t = TutorialState::default();
        assert!(t.format_tip_query(9).starts_with("TIP0"));
    }
}
