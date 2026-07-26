//! Theft / crime prestige subset (Haxe illegal take / ownership violation).
//!
//! Taking an owned ground object without being the owner applies a prestige hit
//! and increments a personal crime counter. Pure logic — world mutations stay
//! in the sim apply path.

use std::collections::HashMap;

/// Prestige lost on a successful illegal take of an owned object.
pub const THEFT_PRESTIGE_PENALTY: f32 = 1.0;

/// Per-player crime bookkeeping.
#[derive(Debug, Clone, Default)]
pub struct CrimeRecord {
    pub thefts: u32,
    pub prestige_lost: f32,
}

/// Session crime state (no SQL).
#[derive(Debug, Default, Clone)]
pub struct CrimeState {
    pub records: HashMap<i32, CrimeRecord>,
}

/// Outcome of checking a ground take against ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeLegality {
    /// Tile has no owner, or taker is owner.
    Legal,
    /// Object is owned by someone else.
    Theft,
}

/// Decide whether picking up `owner_id` object by `taker_p_id` is theft.
///
/// `owner_id == 0` means unowned → always legal.
pub fn classify_take(taker_p_id: i32, owner_id: i32) -> TakeLegality {
    if owner_id == 0 || owner_id == taker_p_id {
        TakeLegality::Legal
    } else {
        TakeLegality::Theft
    }
}

impl CrimeState {
    pub fn record_mut(&mut self, p_id: i32) -> &mut CrimeRecord {
        self.records.entry(p_id).or_default()
    }

    /// Record a theft: increment counter and return prestige penalty to apply.
    pub fn record_theft(&mut self, p_id: i32) -> f32 {
        let r = self.record_mut(p_id);
        r.thefts = r.thefts.saturating_add(1);
        r.prestige_lost += THEFT_PRESTIGE_PENALTY;
        THEFT_PRESTIGE_PENALTY
    }

    pub fn thefts_of(&self, p_id: i32) -> u32 {
        self.records.get(&p_id).map(|r| r.thefts).unwrap_or(0)
    }

    /// Chat body for `SAY ?CRIME` (without leading p_id).
    pub fn format_crime_query(&self, p_id: i32) -> String {
        match self.records.get(&p_id) {
            Some(r) => format!(
                "CRIME thefts={} prestige_lost={:.1}",
                r.thefts, r.prestige_lost
            ),
            None => "CRIME thefts=0 prestige_lost=0.0".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unowned_is_legal() {
        assert_eq!(classify_take(1, 0), TakeLegality::Legal);
    }

    #[test]
    fn owner_take_is_legal() {
        assert_eq!(classify_take(7, 7), TakeLegality::Legal);
    }

    #[test]
    fn other_owner_is_theft() {
        assert_eq!(classify_take(1, 2), TakeLegality::Theft);
    }

    #[test]
    fn record_theft_accumulates() {
        let mut c = CrimeState::default();
        assert_eq!(c.record_theft(9), THEFT_PRESTIGE_PENALTY);
        assert_eq!(c.record_theft(9), THEFT_PRESTIGE_PENALTY);
        assert_eq!(c.thefts_of(9), 2);
        let q = c.format_crime_query(9);
        assert!(q.contains("thefts=2"));
        assert!(q.contains("prestige_lost=2.0"));
    }

    #[test]
    fn clean_record_query() {
        let c = CrimeState::default();
        assert_eq!(
            c.format_crime_query(1),
            "CRIME thefts=0 prestige_lost=0.0"
        );
    }
}
