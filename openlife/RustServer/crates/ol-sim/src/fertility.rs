//! Pregnancy / fertility timer subset (Haxe birth cooldown + gestation).
//!
//! Full baby holding / nursing lives elsewhere; this module tracks
//! who can birth and a simple gestation countdown.
//!
//! # Haxe parity
//! - `GlobalPlayerInstance.isFertile` — not deleted, age in
//!   `[MinAgeFertile, MaxAgeFertile]`, **and female**.
//! - Mother band: `ServerSettings.MinAgeFertile=14`, `MaxAgeFertile=42`
//!   (inclusive; Haxe uses `age > MaxAgeFertile` ⇒ false).
//! - Father fitness uses a separate 55 gate in [`crate::birth_fitness`].

use std::collections::HashMap;

/// Minimum age to initiate birth (Haxe `MinAgeFertile`).
pub const FERTILE_MIN_AGE: f32 = 14.0;

/// Maximum age still fertile for **mothers** (Haxe `MaxAgeFertile` = 42).
/// Father fitness uses a separate 55 gate in [`crate::birth_fitness`].
pub const FERTILE_MAX_AGE: f32 = 42.0;

/// Sim-seconds of cooldown after a successful BIRTH.
pub const BIRTH_COOLDOWN_SECS: f32 = 120.0;

/// Optional gestation before baby appears (0 = instant birth as today).
pub const GESTATION_SECS: f32 = 30.0;

/// Age-only fertile band (Haxe `MinAgeFertile`..=`MaxAgeFertile`).
// Haxe: GlobalPlayerInstance.isFertile age check
#[inline]
pub fn age_fertile(age: f32) -> bool {
    age_fertile_ex(age, FERTILE_MIN_AGE, FERTILE_MAX_AGE)
}

/// Live min/max age band for mother fertility (inclusive both ends).
// Haxe: ServerSettings.MinAgeFertile / MaxAgeFertile
// C-SS-MORE-BATCH4
#[inline]
pub fn age_fertile_ex(age: f32, min_age: f32, max_age: f32) -> bool {
    let min = if min_age.is_finite() && min_age >= 0.0 {
        min_age
    } else {
        FERTILE_MIN_AGE
    };
    let max = if max_age.is_finite() && max_age > 0.0 {
        max_age
    } else {
        FERTILE_MAX_AGE
    };
    age.is_finite() && age >= min && age <= max
}

/// Haxe `GlobalPlayerInstance.isFertile`.
///
/// Requires: not deleted, female, age in fertile band.
// Haxe: GlobalPlayerInstance.isFertile L5137-5141
#[inline]
pub fn is_fertile(deleted: bool, age: f32, is_female: bool) -> bool {
    is_fertile_ex(deleted, age, is_female, FERTILE_MIN_AGE, FERTILE_MAX_AGE)
}

/// Live min/max band variant of [`is_fertile`].
// Haxe: GlobalPlayerInstance.isFertile + ServerSettings.Min/MaxAgeFertile
// C-SS-MORE-BATCH4
#[inline]
pub fn is_fertile_ex(
    deleted: bool,
    age: f32,
    is_female: bool,
    min_age: f32,
    max_age: f32,
) -> bool {
    if deleted {
        return false;
    }
    if !is_female {
        return false;
    }
    age_fertile_ex(age, min_age, max_age)
}

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

    /// True when age is in fertile band (age-only; prefer [`is_fertile`] for full gate).
    pub fn age_fertile(age: f32) -> bool {
        age_fertile(age)
    }

    /// Legacy 3-arg gate (age + cooldown only; assumes alive female).
    /// Prefer [`Self::can_birth_full`] (FERTILITY-TWINS).
    pub fn can_birth(
        &self,
        mother_id: i32,
        age: f32,
        sim_time: f32,
    ) -> Result<(), &'static str> {
        self.can_birth_full(mother_id, age, sim_time, false, true)
    }

    /// Can mother start a birth/gestation at `sim_time`?
    ///
    /// Full Haxe gate: [`is_fertile`] + not already gestating + cooldown clear.
    /// Errors: `"DELETED"`, `"MALE"`, `"AGE"`, `"GESTATING"`, `"COOLDOWN"`.
    // Haxe: isFertile + birth cooldown / gestation bookkeeping
    pub fn can_birth_full(
        &self,
        mother_id: i32,
        age: f32,
        sim_time: f32,
        deleted: bool,
        is_female: bool,
    ) -> Result<(), &'static str> {
        self.can_birth_full_ex(
            mother_id,
            age,
            sim_time,
            deleted,
            is_female,
            FERTILE_MIN_AGE,
            FERTILE_MAX_AGE,
        )
    }

    /// Live Min/MaxAgeFertile variant of [`Self::can_birth_full`].
    // Haxe: isFertile + ServerSettings.MinAgeFertile / MaxAgeFertile
    // C-SS-MORE-BATCH4
    pub fn can_birth_full_ex(
        &self,
        mother_id: i32,
        age: f32,
        sim_time: f32,
        deleted: bool,
        is_female: bool,
        min_age: f32,
        max_age: f32,
    ) -> Result<(), &'static str> {
        if deleted {
            return Err("DELETED");
        }
        if !is_female {
            return Err("MALE");
        }
        if !age_fertile_ex(age, min_age, max_age) {
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

    /// Chat body for `SAY ?FERTILE` / `?BIRTH` status (assumes female for legacy).
    pub fn format_query(&self, mother_id: i32, age: f32, sim_time: f32) -> String {
        self.format_query_sex(mother_id, age, sim_time, true)
    }

    /// Chat body with sex-aware status (`male` / `infertile` / `ready` / …).
    pub fn format_query_sex(
        &self,
        mother_id: i32,
        age: f32,
        sim_time: f32,
        is_female: bool,
    ) -> String {
        self.format_query_sex_ex(
            mother_id,
            age,
            sim_time,
            is_female,
            FERTILE_MIN_AGE,
            FERTILE_MAX_AGE,
        )
    }

    /// Live Min/MaxAgeFertile variant of [`Self::format_query_sex`].
    // Haxe: isFertile + ServerSettings.MinAgeFertile / MaxAgeFertile
    // C-SS-MORE-BATCH4
    pub fn format_query_sex_ex(
        &self,
        mother_id: i32,
        age: f32,
        sim_time: f32,
        is_female: bool,
        min_age: f32,
        max_age: f32,
    ) -> String {
        let fertile = is_fertile_ex(false, age, is_female, min_age, max_age);
        match self.by_mother.get(&mother_id) {
            Some(r) => {
                let status = if r.gestating_until.is_some() {
                    "gestating"
                } else if sim_time < r.next_birth_ready {
                    "cooldown"
                } else if fertile {
                    "ready"
                } else if !is_female {
                    "male"
                } else {
                    "infertile"
                };
                format!(
                    "FERTILE {status} births={} next_ready={:.0}",
                    r.births, r.next_birth_ready
                )
            }
            None => {
                let status = if fertile {
                    "ready"
                } else if !is_female {
                    "male"
                } else {
                    "infertile"
                };
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
        assert!(!age_fertile(10.0));
        assert!(age_fertile(20.0));
        assert!(age_fertile(42.0));
        assert!(!age_fertile(43.0));
        assert!(!age_fertile(55.0));
        assert_eq!(FERTILE_MAX_AGE, 42.0);
        assert!(!FertilityState::age_fertile(10.0));
        assert!(FertilityState::age_fertile(20.0));
    }

    #[test]
    fn is_fertile_requires_female_and_alive() {
        assert!(is_fertile(false, 20.0, true));
        assert!(!is_fertile(true, 20.0, true));
        assert!(!is_fertile(false, 20.0, false));
        assert!(!is_fertile(false, 10.0, true));
        assert!(!is_fertile(false, 50.0, true));
        assert!(is_fertile(false, 14.0, true));
        assert!(is_fertile(false, 42.0, true));
    }

    /// C-SS-MORE-BATCH4: live Min/MaxAgeFertile band.
    // Haxe: ServerSettings.MinAgeFertile / MaxAgeFertile
    #[test]
    fn age_fertile_ex_live_band() {
        // Default inclusive 14..=42
        assert!(age_fertile_ex(14.0, 14.0, 42.0));
        assert!(age_fertile_ex(42.0, 14.0, 42.0));
        assert!(!age_fertile_ex(42.01, 14.0, 42.0));
        assert!(!age_fertile_ex(13.9, 14.0, 42.0));
        // Live override 12..=50
        assert!(age_fertile_ex(12.0, 12.0, 50.0));
        assert!(age_fertile_ex(50.0, 12.0, 50.0));
        assert!(!age_fertile_ex(50.01, 12.0, 50.0));
        assert!(!age_fertile_ex(11.9, 12.0, 50.0));
        assert!(is_fertile_ex(false, 12.0, true, 12.0, 50.0));
        assert!(!is_fertile_ex(false, 12.0, false, 12.0, 50.0));
        assert!(!is_fertile_ex(true, 20.0, true, 12.0, 50.0));
    }

    /// C-SS-MORE-BATCH4: can_birth / query honor live fertile band.
    // Haxe: ServerSettings.MinAgeFertile / MaxAgeFertile
    #[test]
    fn can_birth_full_ex_live_age_band() {
        let f = FertilityState::default();
        // Age 45 fails default 14..=42
        assert_eq!(
            f.can_birth_full(1, 45.0, 0.0, false, true),
            Err("AGE")
        );
        // Live max 50 allows 45
        assert!(f
            .can_birth_full_ex(1, 45.0, 0.0, false, true, 12.0, 50.0)
            .is_ok());
        // Live min 16 rejects 15
        assert_eq!(
            f.can_birth_full_ex(1, 15.0, 0.0, false, true, 16.0, 42.0),
            Err("AGE")
        );
        let q = f.format_query_sex_ex(1, 45.0, 0.0, true, 12.0, 50.0);
        assert!(q.contains("ready"), "{q}");
        let q2 = f.format_query_sex_ex(1, 45.0, 0.0, true, 14.0, 42.0);
        assert!(q2.contains("infertile"), "{q2}");
    }

    #[test]
    fn can_birth_male_and_age_gates() {
        let f = FertilityState::default();
        assert_eq!(
            f.can_birth_full(1, 20.0, 0.0, false, false),
            Err("MALE")
        );
        assert_eq!(f.can_birth_full(1, 10.0, 0.0, false, true), Err("AGE"));
        assert_eq!(
            f.can_birth_full(1, 20.0, 0.0, true, true),
            Err("DELETED")
        );
        assert!(f.can_birth_full(1, 20.0, 0.0, false, true).is_ok());
        // legacy can_birth assumes female
        assert!(f.can_birth(1, 20.0, 0.0).is_ok());
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
    fn query_ready_and_male() {
        let f = FertilityState::default();
        let q = f.format_query(1, 25.0, 0.0);
        assert!(q.contains("ready"), "{q}");
        let q2 = f.format_query(1, 5.0, 0.0);
        assert!(q2.contains("infertile"), "{q2}");
        let q3 = f.format_query_sex(1, 25.0, 0.0, false);
        assert!(q3.contains("male"), "{q3}");
    }
}
