//! Apocalypse state machine (Haxe `APOCALYPSE` / `APOCALYPSE_DONE` tags subset).
//!
//! Phases: Idle → Warning → Active → Done.
//! During Active, global food drain is slightly increased.

/// Slight global food drain multiplier while [`ApocalypsePhase::Active`].
pub const APOC_FOOD_DRAIN_MULT: f32 = 1.15;

/// Default warning phase length (seconds).
pub const DEFAULT_WARNING_SECS: f32 = 30.0;
/// Default active phase length (seconds).
pub const DEFAULT_ACTIVE_SECS: f32 = 60.0;

/// Apocalypse lifecycle phases (Haxe AP pending → active → AD done).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ApocalypsePhase {
    #[default]
    Idle,
    /// Pending — client visual lead-in (Haxe `APOCALYPSE` / AP).
    Warning,
    /// Active disaster; food drain increased.
    Active,
    /// Over (Haxe `APOCALYPSE_DONE` / AD).
    Done,
}

impl ApocalypsePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Warning => "WARNING",
            Self::Active => "ACTIVE",
            Self::Done => "DONE",
        }
    }
}

/// Server-side apocalypse countdown machine.
#[derive(Debug, Clone)]
pub struct Apocalypse {
    pub phase: ApocalypsePhase,
    /// Seconds remaining in Warning / Active. Unused in Idle / Done.
    pub countdown: f32,
    pub warning_duration: f32,
    pub active_duration: f32,
}

impl Default for Apocalypse {
    fn default() -> Self {
        Self {
            phase: ApocalypsePhase::Idle,
            countdown: 0.0,
            warning_duration: DEFAULT_WARNING_SECS,
            active_duration: DEFAULT_ACTIVE_SECS,
        }
    }
}

impl Apocalypse {
    /// Begin warning countdown (from Idle or Done).
    pub fn trigger(&mut self) {
        self.phase = ApocalypsePhase::Warning;
        self.countdown = self.warning_duration.max(0.0);
    }

    /// Force-stop apocalypse (testing / `SAY ENDAPOC`). Resets to Idle.
    pub fn end(&mut self) {
        self.phase = ApocalypsePhase::Idle;
        self.countdown = 0.0;
    }

    /// Advance phase machine by `dt` seconds.
    ///
    /// `Warning` → `Active` → `Done`. Idle and Done are terminal until
    /// [`Self::trigger`].
    pub fn tick(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        match self.phase {
            ApocalypsePhase::Idle | ApocalypsePhase::Done => {}
            ApocalypsePhase::Warning => {
                self.countdown -= dt;
                if self.countdown <= 0.0 {
                    self.phase = ApocalypsePhase::Active;
                    self.countdown = self.active_duration.max(0.0);
                }
            }
            ApocalypsePhase::Active => {
                self.countdown -= dt;
                if self.countdown <= 0.0 {
                    self.phase = ApocalypsePhase::Done;
                    self.countdown = 0.0;
                }
            }
        }
    }

    /// Global food drain multiplier: [`APOC_FOOD_DRAIN_MULT`] during Active, else 1.0.
    pub fn food_drain_multiplier(&self) -> f32 {
        if self.phase == ApocalypsePhase::Active {
            APOC_FOOD_DRAIN_MULT
        } else {
            1.0
        }
    }

    pub fn is_active(&self) -> bool {
        self.phase == ApocalypsePhase::Active
    }

    /// Text for `SAY ?APOC` (without leading player id).
    pub fn query_text(&self) -> String {
        match self.phase {
            ApocalypsePhase::Idle => "APOC IDLE".into(),
            ApocalypsePhase::Warning => {
                format!("APOC WARNING {:.0}", self.countdown.max(0.0))
            }
            ApocalypsePhase::Active => {
                format!("APOC ACTIVE {:.0}", self.countdown.max(0.0))
            }
            ApocalypsePhase::Done => "APOC DONE".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle() {
        let a = Apocalypse::default();
        assert_eq!(a.phase, ApocalypsePhase::Idle);
        assert_eq!(a.countdown, 0.0);
        assert_eq!(a.food_drain_multiplier(), 1.0);
        assert!(!a.is_active());
        assert_eq!(a.query_text(), "APOC IDLE");
    }

    #[test]
    fn trigger_starts_warning() {
        let mut a = Apocalypse {
            warning_duration: 10.0,
            active_duration: 20.0,
            ..Default::default()
        };
        a.trigger();
        assert_eq!(a.phase, ApocalypsePhase::Warning);
        assert_eq!(a.countdown, 10.0);
        assert_eq!(a.food_drain_multiplier(), 1.0);
        assert!(a.query_text().starts_with("APOC WARNING"));
    }

    #[test]
    fn phase_transitions_warning_active_done() {
        let mut a = Apocalypse {
            warning_duration: 5.0,
            active_duration: 8.0,
            ..Default::default()
        };
        a.trigger();
        assert_eq!(a.phase, ApocalypsePhase::Warning);

        // Partial warning tick — still warning.
        a.tick(2.0);
        assert_eq!(a.phase, ApocalypsePhase::Warning);
        assert!((a.countdown - 3.0).abs() < 1e-4);

        // Finish warning → Active with full active duration.
        a.tick(3.0);
        assert_eq!(a.phase, ApocalypsePhase::Active);
        assert!((a.countdown - 8.0).abs() < 1e-4);
        assert_eq!(a.food_drain_multiplier(), APOC_FOOD_DRAIN_MULT);
        assert!(a.is_active());
        assert!(a.query_text().starts_with("APOC ACTIVE"));

        // Partial active.
        a.tick(3.0);
        assert_eq!(a.phase, ApocalypsePhase::Active);
        assert!((a.countdown - 5.0).abs() < 1e-4);

        // Finish active → Done.
        a.tick(5.0);
        assert_eq!(a.phase, ApocalypsePhase::Done);
        assert_eq!(a.countdown, 0.0);
        assert_eq!(a.food_drain_multiplier(), 1.0);
        assert!(!a.is_active());
        assert_eq!(a.query_text(), "APOC DONE");

        // Done is sticky until re-trigger.
        a.tick(100.0);
        assert_eq!(a.phase, ApocalypsePhase::Done);
    }

    #[test]
    fn overshoot_countdown_still_transitions() {
        let mut a = Apocalypse {
            warning_duration: 1.0,
            active_duration: 1.0,
            ..Default::default()
        };
        a.trigger();
        a.tick(50.0); // huge step through warning
        assert_eq!(a.phase, ApocalypsePhase::Active);
        a.tick(50.0); // huge step through active
        assert_eq!(a.phase, ApocalypsePhase::Done);
    }

    #[test]
    fn idle_tick_noop() {
        let mut a = Apocalypse::default();
        a.tick(10.0);
        assert_eq!(a.phase, ApocalypsePhase::Idle);
    }

    #[test]
    fn retrigger_from_done() {
        let mut a = Apocalypse {
            warning_duration: 2.0,
            active_duration: 2.0,
            phase: ApocalypsePhase::Done,
            countdown: 0.0,
            ..Default::default()
        };
        a.trigger();
        assert_eq!(a.phase, ApocalypsePhase::Warning);
        assert_eq!(a.countdown, 2.0);
    }

    #[test]
    fn end_resets_to_idle_from_any_phase() {
        let mut a = Apocalypse {
            warning_duration: 5.0,
            active_duration: 5.0,
            ..Default::default()
        };
        a.trigger();
        a.tick(5.0); // → Active
        assert_eq!(a.phase, ApocalypsePhase::Active);
        assert!(a.is_active());
        a.end();
        assert_eq!(a.phase, ApocalypsePhase::Idle);
        assert_eq!(a.countdown, 0.0);
        assert_eq!(a.food_drain_multiplier(), 1.0);
        assert!(!a.is_active());
        assert_eq!(a.query_text(), "APOC IDLE");
        // Can re-trigger after end.
        a.trigger();
        assert_eq!(a.phase, ApocalypsePhase::Warning);
    }

    #[test]
    fn phase_as_str() {
        assert_eq!(ApocalypsePhase::Idle.as_str(), "IDLE");
        assert_eq!(ApocalypsePhase::Warning.as_str(), "WARNING");
        assert_eq!(ApocalypsePhase::Active.as_str(), "ACTIVE");
        assert_eq!(ApocalypsePhase::Done.as_str(), "DONE");
    }
}
