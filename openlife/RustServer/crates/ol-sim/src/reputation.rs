//! Combat reputation score (float), **separate from** prestige / PrestigeClass.
//!
//! Haxe maps this via `GlobalPlayerInstance.lostCombatPrestige` (positive = bad)
//! and stores lineage `reputation = lostCombatPrestige * (-1)` (higher = better).
//! Labels match `PlayerSoul.getCombatPrestigeLabel`.
//!
//! Pure bookkeeping — not yet wired into every combat path.

use std::collections::HashMap;

/// Seven reputation levels from combat prestige (Haxe label table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReputationLabel {
    SuperGood,
    Good,
    FairlyGood,
    Neutral,
    FairlyBad,
    Bad,
    SuperBad,
}

impl ReputationLabel {
    /// Wire / chat token (`super_good`, `neutral`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SuperGood => "super_good",
            Self::Good => "good",
            Self::FairlyGood => "fairly_good",
            Self::Neutral => "neutral",
            Self::FairlyBad => "fairly_bad",
            Self::Bad => "bad",
            Self::SuperBad => "super_bad",
        }
    }

    /// Haxe `getCombatPrestigeLabel` human phrase.
    pub fn display(self) -> &'static str {
        match self {
            Self::SuperGood => "super good",
            Self::Good => "good",
            Self::FairlyGood => "fairly good",
            Self::Neutral => "neutral",
            Self::FairlyBad => "fairly bad",
            Self::Bad => "bad",
            Self::SuperBad => "super bad",
        }
    }
}

/// Map Haxe `lostCombatPrestige` (positive = bad) to a label.
///
/// Thresholds (from `PlayerSoul.getCombatPrestigeLabel`):
/// - `> 50` → super bad
/// - `≥ 20` → bad
/// - `> 4` → fairly bad
/// - `> -4` → neutral
/// - `≥ -20` → fairly good
/// - `≥ -50` → good
/// - else → super good
pub fn label_from_lost_combat(lost_combat: f32) -> ReputationLabel {
    if !lost_combat.is_finite() {
        return ReputationLabel::Neutral;
    }
    if lost_combat > 50.0 {
        ReputationLabel::SuperBad
    } else if lost_combat >= 20.0 {
        ReputationLabel::Bad
    } else if lost_combat > 4.0 {
        ReputationLabel::FairlyBad
    } else if lost_combat > -4.0 {
        ReputationLabel::Neutral
    } else if lost_combat >= -20.0 {
        ReputationLabel::FairlyGood
    } else if lost_combat >= -50.0 {
        ReputationLabel::Good
    } else {
        ReputationLabel::SuperGood
    }
}

/// Convert lost-combat float to stored lineage reputation (`* -1`).
///
/// Higher reputation is better (inverse of lost combat prestige).
pub fn reputation_from_lost_combat(lost_combat: f32) -> f32 {
    if !lost_combat.is_finite() {
        return 0.0;
    }
    -lost_combat
}

/// Inverse of [`reputation_from_lost_combat`].
pub fn lost_combat_from_reputation(reputation: f32) -> f32 {
    if !reputation.is_finite() {
        return 0.0;
    }
    -reputation
}

/// Label from the **stored reputation** float (higher = better).
pub fn label_from_reputation(reputation: f32) -> ReputationLabel {
    label_from_lost_combat(lost_combat_from_reputation(reputation))
}

/// True when lost combat prestige is high enough that AI treats the player as
/// dangerous (Haxe uses thresholds around 1 / 4 / 5).
pub fn is_dangerous_lost_combat(lost_combat: f32) -> bool {
    lost_combat.is_finite() && lost_combat > 1.0
}

/// Session-local reputation book (p_id → reputation float, higher = better).
///
/// Independent of [`crate::prestige::PrestigeClass`] and economy trade prestige.
#[derive(Debug, Clone, Default)]
pub struct ReputationBook {
    scores: HashMap<i32, f32>,
}

impl ReputationBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current reputation (0.0 if unknown).
    pub fn get(&self, p_id: i32) -> f32 {
        self.scores.get(&p_id).copied().unwrap_or(0.0)
    }

    /// Set absolute reputation score.
    pub fn set(&mut self, p_id: i32, reputation: f32) {
        let v = if reputation.is_finite() {
            reputation
        } else {
            0.0
        };
        self.scores.insert(p_id, v);
    }

    /// Add delta to reputation (positive delta improves reputation).
    pub fn add(&mut self, p_id: i32, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        let cur = self.get(p_id);
        self.set(p_id, cur + delta);
    }

    /// Record lost-combat float (Haxe field); stores inverted reputation.
    pub fn set_from_lost_combat(&mut self, p_id: i32, lost_combat: f32) {
        self.set(p_id, reputation_from_lost_combat(lost_combat));
    }

    /// Lost-combat view of stored score (positive = bad).
    pub fn lost_combat(&self, p_id: i32) -> f32 {
        lost_combat_from_reputation(self.get(p_id))
    }

    /// Label for a player (neutral if unknown / 0).
    pub fn label(&self, p_id: i32) -> ReputationLabel {
        label_from_reputation(self.get(p_id))
    }

    /// Apply illegal combat damage: attacker gains bad reputation.
    ///
    /// `damage` is the wound/kill magnitude. Higher prestige class multiplies
    /// guilt by 0.5 in Haxe when attacking down; this pure helper takes a
    /// precomputed `guilt_scale` (default 1.0).
    pub fn apply_illegal_hit(&mut self, attacker: i32, damage: f32, guilt_scale: f32) {
        if !damage.is_finite() || damage <= 0.0 {
            return;
        }
        let scale = if guilt_scale.is_finite() && guilt_scale > 0.0 {
            guilt_scale
        } else {
            1.0
        };
        // lost_combat += damage * scale → reputation -= damage * scale
        self.add(attacker, -(damage * scale));
    }

    /// Legal hit: both sides recover some lost combat prestige (Haxe halves).
    pub fn apply_legal_hit(&mut self, a: i32, b: i32, damage: f32) {
        if !damage.is_finite() || damage <= 0.0 {
            return;
        }
        let recover = damage / 2.0;
        // lost_combat -= damage/2 → reputation += damage/2
        self.add(a, recover);
        self.add(b, recover);
    }

    pub fn remove(&mut self, p_id: i32) {
        self.scores.remove(&p_id);
    }

    pub fn len(&self) -> usize {
        self.scores.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

/// `REP score=N.N label=neutral lost=N.N` body (no leading p_id).
pub fn format_reputation_query(book: &ReputationBook, p_id: i32) -> String {
    let score = book.get(p_id);
    let label = book.label(p_id);
    let lost = book.lost_combat(p_id);
    format!(
        "REP score={score:.1} label={} lost={lost:.1}",
        label.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_haxe_lost_combat_table() {
        assert_eq!(label_from_lost_combat(51.0), ReputationLabel::SuperBad);
        assert_eq!(label_from_lost_combat(20.0), ReputationLabel::Bad);
        assert_eq!(label_from_lost_combat(19.9), ReputationLabel::FairlyBad);
        assert_eq!(label_from_lost_combat(4.1), ReputationLabel::FairlyBad);
        assert_eq!(label_from_lost_combat(4.0), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(0.0), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(-3.9), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(-4.0), ReputationLabel::FairlyGood);
        assert_eq!(label_from_lost_combat(-20.0), ReputationLabel::FairlyGood);
        assert_eq!(label_from_lost_combat(-20.1), ReputationLabel::Good);
        assert_eq!(label_from_lost_combat(-50.0), ReputationLabel::Good);
        assert_eq!(label_from_lost_combat(-50.1), ReputationLabel::SuperGood);
        assert_eq!(label_from_lost_combat(f32::NAN), ReputationLabel::Neutral);
    }

    #[test]
    fn reputation_is_negative_lost_combat() {
        assert_eq!(reputation_from_lost_combat(10.0), -10.0);
        assert_eq!(reputation_from_lost_combat(-3.0), 3.0);
        assert_eq!(lost_combat_from_reputation(7.5), -7.5);
        assert_eq!(
            label_from_reputation(25.0),
            label_from_lost_combat(-25.0)
        );
    }

    #[test]
    fn separate_from_zero_default() {
        let book = ReputationBook::new();
        assert_eq!(book.get(1), 0.0);
        assert_eq!(book.label(1), ReputationLabel::Neutral);
        assert!(!is_dangerous_lost_combat(book.lost_combat(1)));
    }

    #[test]
    fn illegal_hit_worsens_attacker_only() {
        let mut book = ReputationBook::new();
        book.apply_illegal_hit(1, 10.0, 1.0);
        assert_eq!(book.get(1), -10.0);
        assert_eq!(book.lost_combat(1), 10.0);
        assert_eq!(book.label(1), ReputationLabel::FairlyBad);
        assert_eq!(book.get(2), 0.0);
        // Higher-class guilt scale 0.5
        book.apply_illegal_hit(3, 10.0, 0.5);
        assert_eq!(book.get(3), -5.0);
    }

    #[test]
    fn legal_hit_recovers_both() {
        let mut book = ReputationBook::new();
        book.set_from_lost_combat(1, 8.0); // rep = -8
        book.set_from_lost_combat(2, 4.0); // rep = -4
        book.apply_legal_hit(1, 2, 4.0); // +2 each
        assert_eq!(book.get(1), -6.0);
        assert_eq!(book.get(2), -2.0);
    }

    #[test]
    fn format_query_and_remove() {
        let mut book = ReputationBook::new();
        book.set(5, 30.0);
        let q = format_reputation_query(&book, 5);
        assert!(q.contains("score=30.0"), "{q}");
        assert!(q.contains("label=good") || q.contains("label=fairly_good"), "{q}");
        assert!(q.starts_with("REP "), "{q}");
        book.remove(5);
        assert!(book.is_empty());
    }

    #[test]
    fn dangerous_threshold() {
        assert!(!is_dangerous_lost_combat(1.0));
        assert!(is_dangerous_lost_combat(1.01));
        assert!(is_dangerous_lost_combat(5.0));
    }
}
