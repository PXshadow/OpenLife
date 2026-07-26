//! Pregnancy / fertility timer subset (Haxe birth cooldown + gestation).
//!
//! Full baby holding / nursing lives elsewhere; this module only tracks
//! who can birth and a simple gestation countdown.

use std::collections::HashMap;

/// Minimum age to initiate birth (Haxe adult threshold subset).
pub const FERTILE_MIN_AGE: f32 = 14.0;

/// Maximum age still fertile for **mothers** (Haxe `MaxAgeFertile` = 42).
/// Father fitness uses a separate 55 gate in [`crate::birth_fitness`].
pub const FERTILE_MAX_AGE: f32 = 42.0;

/// Sim-seconds of cooldown after a successful BIRTH.
pub const BIRTH_COOLDOWN_SECS: f32 = 120.0;

/// Optional gestation before baby appears (0 = instant birth as today).
pub const GESTATION_SECS: f32 = 30.0;

/// Per-mother fertility bookkeeping.
#[derive(Debug, Clone, Default)]
pub struct FertilityRecord {
    /// Earliest sim_time when next birth is allowed.
    pub next_birth_ready: f32,
    /// If Some, baby arrives at this sim_time (gestation in progress).
    pub gestating_until: Option<f32>,
    pub births: u32,
    /// Haxe `childrenBirthMali` — accumulates per birth, lowers mother fitness.
    pub children_birth_mali: f32,
}

/// Session fertility map.
#[derive(Debug, Default, Clone)]
pub struct FertilityState {
    pub by_mother: HashMap<i32, FertilityRecord>,
}

impl FertilityState {
    pub fn record_mut(&mut self, mother_id: i32) -> &mut FertilityRecord {
        self.by_mother.entry(mother_id).or_default()
    }

    /// True when age is in fertile band.
    pub fn age_fertile(age: f32) -> bool {
        age >= FERTILE_MIN_AGE && age <= FERTILE_MAX_AGE
    }

    /// Can mother start a birth/gestation at `sim_time`?
    pub fn can_birth(&self, mother_id: i32, age: f32, sim_time: f32) -> Result<(), &'static str> {
        if !Self::age_fertile(age) {
            return Err("AGE");
        }
        if let Some(r) = self.by_mother.get(&mother_id) {
            if r.gestating_until.is_some() {
                return Err("GESTATING");
            }
            if sim_time < r.next_birth_ready {
                return Err("COOLDOWN");
            }
        }
        Ok(())
    }

    /// Start instant birth path: set cooldown, increment count.
    pub fn complete_birth(&mut self, mother_id: i32, sim_time: f32) {
        let r = self.record_mut(mother_id);
        r.births = r.births.saturating_add(1);
        r.gestating_until = None;
        r.next_birth_ready = sim_time + BIRTH_COOLDOWN_SECS;
        r.children_birth_mali = crate::birth_fitness::next_children_birth_mali(r.children_birth_mali);
    }

    /// Start timed gestation; returns due sim_time.
    pub fn start_gestation(&mut self, mother_id: i32, sim_time: f32) -> f32 {
        let due = sim_time + GESTATION_SECS;
        let r = self.record_mut(mother_id);
        r.gestating_until = Some(due);
        due
    }

    /// Mothers whose gestation completed at/before sim_time.
    pub fn poll_due(&mut self, sim_time: f32) -> Vec<i32> {
        let mut due = Vec::new();
        for (id, r) in self.by_mother.iter_mut() {
            if let Some(t) = r.gestating_until {
                if sim_time >= t {
                    r.gestating_until = None;
                    r.births = r.births.saturating_add(1);
                    r.next_birth_ready = sim_time + BIRTH_COOLDOWN_SECS;
                    r.children_birth_mali =
                        crate::birth_fitness::next_children_birth_mali(r.children_birth_mali);
                    due.push(*id);
                }
            }
        }
        due
    }

    /// Chat body for `SAY ?FERTILE` / `?BIRTH` status.
    pub fn format_query(&self, mother_id: i32, age: f32, sim_time: f32) -> String {
        let fertile = Self::age_fertile(age);
        match self.by_mother.get(&mother_id) {
            Some(r) => {
                let status = if r.gestating_until.is_some() {
                    "gestating"
                } else if sim_time < r.next_birth_ready {
                    "cooldown"
                } else if fertile {
                    "ready"
                } else {
                    "infertile"
                };
                format!(
                    "FERTILE {status} births={} next_ready={:.0}",
                    r.births, r.next_birth_ready
                )
            }
            None => {
                let status = if fertile { "ready" } else { "infertile" };
                format!("FERTILE {status} births=0 next_ready=0")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_band() {
        assert!(!FertilityState::age_fertile(10.0));
        assert!(FertilityState::age_fertile(20.0));
        assert!(FertilityState::age_fertile(42.0));
        assert!(!FertilityState::age_fertile(43.0));
        assert!(!FertilityState::age_fertile(55.0));
        assert_eq!(FERTILE_MAX_AGE, 42.0);
    }

    #[test]
    fn cooldown_blocks_second_birth() {
        let mut f = FertilityState::default();
        assert!(f.can_birth(1, 20.0, 0.0).is_ok());
        f.complete_birth(1, 0.0);
        assert_eq!(f.can_birth(1, 20.0, 10.0), Err("COOLDOWN"));
        assert!(f.can_birth(1, 20.0, BIRTH_COOLDOWN_SECS + 1.0).is_ok());
    }

    #[test]
    fn gestation_poll() {
        let mut f = FertilityState::default();
        let due = f.start_gestation(3, 0.0);
        assert_eq!(due, GESTATION_SECS);
        assert_eq!(f.can_birth(3, 20.0, 1.0), Err("GESTATING"));
        assert!(f.poll_due(GESTATION_SECS - 1.0).is_empty());
        let done = f.poll_due(GESTATION_SECS);
        assert_eq!(done, vec![3]);
        assert_eq!(f.by_mother.get(&3).unwrap().births, 1);
    }

    #[test]
    fn query_ready() {
        let f = FertilityState::default();
        let q = f.format_query(1, 25.0, 0.0);
        assert!(q.contains("ready"));
        let q2 = f.format_query(1, 5.0, 0.0);
        assert!(q2.contains("infertile"));
    }
}
