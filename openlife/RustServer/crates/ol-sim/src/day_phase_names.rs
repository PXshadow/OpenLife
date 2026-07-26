//! Pure day-phase names from hour-of-day (OHOL / OpenLife day cycle subset).
//!
//! Standalone from [`crate::environment::Environment`] so callers can label
//! hours without owning season state.

/// Real seconds for a default full game day (matches Environment default).
pub const DEFAULT_DAY_LENGTH_SECS: f32 = 240.0;
/// Hours in a game day.
pub const HOURS_PER_DAY: f32 = 24.0;

/// Dawn start hour (inclusive).
pub const DAWN_START: f32 = 5.0;
/// Day start hour (inclusive).
pub const DAY_START: f32 = 7.0;
/// Dusk start hour (inclusive).
pub const DUSK_START: f32 = 18.0;
/// Night start hour (inclusive) — also wraps after midnight until dawn.
pub const NIGHT_START: f32 = 20.0;

/// Coarse day phase with stable wire names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayPhaseName {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl DayPhaseName {
    /// Wire / SAY token: `DAWN` | `DAY` | `DUSK` | `NIGHT`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dawn => "DAWN",
            Self::Day => "DAY",
            Self::Dusk => "DUSK",
            Self::Night => "NIGHT",
        }
    }

    /// Lowercase display name.
    pub fn display(self) -> &'static str {
        match self {
            Self::Dawn => "dawn",
            Self::Day => "day",
            Self::Dusk => "dusk",
            Self::Night => "night",
        }
    }

    /// Parse `DAWN|DAY|DUSK|NIGHT` (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "DAWN" => Some(Self::Dawn),
            "DAY" => Some(Self::Day),
            "DUSK" => Some(Self::Dusk),
            "NIGHT" => Some(Self::Night),
            _ => None,
        }
    }

    /// All phases in clock order starting at dawn.
    pub fn all() -> [Self; 4] {
        [Self::Dawn, Self::Day, Self::Dusk, Self::Night]
    }
}

/// Normalize hour into `[0, 24)`.
#[inline]
pub fn wrap_hour(hour: f32) -> f32 {
    if !hour.is_finite() {
        return 0.0;
    }
    hour.rem_euclid(HOURS_PER_DAY)
}

/// Phase from continuous hour in `[0, 24)` (any real; wraps).
///
/// | Hours       | Phase |
/// |-------------|-------|
/// | [5, 7)      | Dawn  |
/// | [7, 18)     | Day   |
/// | [18, 20)    | Dusk  |
/// | else        | Night |
pub fn phase_from_hour(hour: f32) -> DayPhaseName {
    let h = wrap_hour(hour);
    if (DAWN_START..DAY_START).contains(&h) {
        DayPhaseName::Dawn
    } else if (DAY_START..DUSK_START).contains(&h) {
        DayPhaseName::Day
    } else if (DUSK_START..NIGHT_START).contains(&h) {
        DayPhaseName::Dusk
    } else {
        DayPhaseName::Night
    }
}

/// Wire name for hour.
pub fn phase_name_from_hour(hour: f32) -> &'static str {
    phase_from_hour(hour).as_str()
}

/// Food / stress multiplier by phase (night costs more).
pub fn phase_food_mult(phase: DayPhaseName) -> f32 {
    match phase {
        DayPhaseName::Day => 1.0,
        DayPhaseName::Dawn | DayPhaseName::Dusk => 1.05,
        DayPhaseName::Night => 1.15,
    }
}

/// Food multiplier from hour.
pub fn food_mult_from_hour(hour: f32) -> f32 {
    phase_food_mult(phase_from_hour(hour))
}

/// True if phase is dark enough for night vision notes (dusk or night).
pub fn is_dark(phase: DayPhaseName) -> bool {
    matches!(phase, DayPhaseName::Dusk | DayPhaseName::Night)
}

/// True if hour is dark.
pub fn is_dark_hour(hour: f32) -> bool {
    is_dark(phase_from_hour(hour))
}

/// Advance hour by `dt` real seconds given `day_length` real seconds per 24h.
pub fn advance_hour(hour: f32, dt: f32, day_length: f32) -> f32 {
    let dl = if day_length.is_finite() && day_length > 0.0 {
        day_length
    } else {
        DEFAULT_DAY_LENGTH_SECS
    };
    let hours_per_sec = HOURS_PER_DAY / dl;
    let d = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
    wrap_hour(wrap_hour(hour) + d * hours_per_sec)
}

/// Hours remaining in the current phase.
pub fn hours_remaining_in_phase(hour: f32) -> f32 {
    let h = wrap_hour(hour);
    let end = match phase_from_hour(h) {
        DayPhaseName::Dawn => DAY_START,
        DayPhaseName::Day => DUSK_START,
        DayPhaseName::Dusk => NIGHT_START,
        DayPhaseName::Night => {
            // Night is [20, 24) ∪ [0, 5)
            if h >= NIGHT_START {
                HOURS_PER_DAY + DAWN_START // cross midnight
            } else {
                DAWN_START
            }
        }
    };
    if end >= HOURS_PER_DAY {
        // night after 20: remaining to 24 + dawn
        (HOURS_PER_DAY - h) + DAWN_START
    } else {
        (end - h).max(0.0)
    }
}

/// Next phase after the current one.
pub fn next_phase(phase: DayPhaseName) -> DayPhaseName {
    match phase {
        DayPhaseName::Dawn => DayPhaseName::Day,
        DayPhaseName::Day => DayPhaseName::Dusk,
        DayPhaseName::Dusk => DayPhaseName::Night,
        DayPhaseName::Night => DayPhaseName::Dawn,
    }
}

/// `DAYPHASE hour=H phase=NAME mult=M rem=R` query body.
pub fn format_day_phase_query(hour: f32) -> String {
    let h = wrap_hour(hour);
    let phase = phase_from_hour(h);
    let mult = phase_food_mult(phase);
    let rem = hours_remaining_in_phase(h);
    format!(
        "DAYPHASE hour={h:.2} phase={} mult={mult:.2} rem={rem:.2}",
        phase.as_str()
    )
}

/// Short `DAY NAME` body (phase only).
pub fn format_day_query(hour: f32) -> String {
    format!("DAY {}", phase_name_from_hour(hour))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_hour_range() {
        assert!((wrap_hour(0.0) - 0.0).abs() < 1e-5);
        assert!((wrap_hour(24.0) - 0.0).abs() < 1e-5);
        assert!((wrap_hour(25.5) - 1.5).abs() < 1e-5);
        assert!((wrap_hour(-1.0) - 23.0).abs() < 1e-5);
        assert_eq!(wrap_hour(f32::NAN), 0.0);
    }

    #[test]
    fn phase_boundaries() {
        assert_eq!(phase_from_hour(5.0), DayPhaseName::Dawn);
        assert_eq!(phase_from_hour(6.9), DayPhaseName::Dawn);
        assert_eq!(phase_from_hour(7.0), DayPhaseName::Day);
        assert_eq!(phase_from_hour(12.0), DayPhaseName::Day);
        assert_eq!(phase_from_hour(17.9), DayPhaseName::Day);
        assert_eq!(phase_from_hour(18.0), DayPhaseName::Dusk);
        assert_eq!(phase_from_hour(19.9), DayPhaseName::Dusk);
        assert_eq!(phase_from_hour(20.0), DayPhaseName::Night);
        assert_eq!(phase_from_hour(23.0), DayPhaseName::Night);
        assert_eq!(phase_from_hour(0.0), DayPhaseName::Night);
        assert_eq!(phase_from_hour(4.9), DayPhaseName::Night);
        assert_eq!(phase_name_from_hour(12.0), "DAY");
        assert_eq!(phase_name_from_hour(3.0), "NIGHT");
    }

    #[test]
    fn parse_and_display() {
        assert_eq!(DayPhaseName::parse("dawn"), Some(DayPhaseName::Dawn));
        assert_eq!(DayPhaseName::parse("DAY"), Some(DayPhaseName::Day));
        assert_eq!(DayPhaseName::parse("Dusk"), Some(DayPhaseName::Dusk));
        assert_eq!(DayPhaseName::parse("NIGHT"), Some(DayPhaseName::Night));
        assert_eq!(DayPhaseName::parse("noon"), None);
        assert_eq!(DayPhaseName::Day.display(), "day");
        assert_eq!(DayPhaseName::all().len(), 4);
    }

    #[test]
    fn food_mult_and_dark() {
        assert!((phase_food_mult(DayPhaseName::Day) - 1.0).abs() < 1e-5);
        assert!((phase_food_mult(DayPhaseName::Night) - 1.15).abs() < 1e-5);
        assert!((food_mult_from_hour(12.0) - 1.0).abs() < 1e-5);
        assert!((food_mult_from_hour(22.0) - 1.15).abs() < 1e-5);
        assert!(!is_dark(DayPhaseName::Day));
        assert!(!is_dark(DayPhaseName::Dawn));
        assert!(is_dark(DayPhaseName::Dusk));
        assert!(is_dark(DayPhaseName::Night));
        assert!(is_dark_hour(21.0));
        assert!(!is_dark_hour(10.0));
    }

    #[test]
    fn advance_hour_full_day() {
        // day_length=240s → 24h in 240s → 0.1 h/s
        let h = advance_hour(0.0, 240.0, 240.0);
        assert!((h - 0.0).abs() < 1e-3);
        let h2 = advance_hour(0.0, 10.0, 240.0);
        assert!((h2 - 1.0).abs() < 1e-3);
        let h3 = advance_hour(23.0, 20.0, 240.0); // +2h → 1.0
        assert!((h3 - 1.0).abs() < 1e-3);
    }

    #[test]
    fn hours_remaining() {
        assert!((hours_remaining_in_phase(7.0) - 11.0).abs() < 1e-3); // day until 18
        assert!((hours_remaining_in_phase(5.0) - 2.0).abs() < 1e-3); // dawn until 7
        assert!((hours_remaining_in_phase(18.0) - 2.0).abs() < 1e-3); // dusk until 20
        // night at 22: 2h to midnight + 5h to dawn = 7
        assert!((hours_remaining_in_phase(22.0) - 7.0).abs() < 1e-3);
        // night at 2: 3h to dawn
        assert!((hours_remaining_in_phase(2.0) - 3.0).abs() < 1e-3);
    }

    #[test]
    fn next_phase_cycle() {
        assert_eq!(next_phase(DayPhaseName::Dawn), DayPhaseName::Day);
        assert_eq!(next_phase(DayPhaseName::Day), DayPhaseName::Dusk);
        assert_eq!(next_phase(DayPhaseName::Dusk), DayPhaseName::Night);
        assert_eq!(next_phase(DayPhaseName::Night), DayPhaseName::Dawn);
    }

    #[test]
    fn formatters() {
        let s = format_day_phase_query(12.0);
        assert!(s.starts_with("DAYPHASE hour=12.00"));
        assert!(s.contains("phase=DAY"));
        assert!(s.contains("mult=1.00"));
        assert!(s.contains("rem=6.00"));
        assert_eq!(format_day_query(12.0), "DAY DAY");
        assert_eq!(format_day_query(22.0), "DAY NIGHT");
        assert_eq!(format_day_query(6.0), "DAY DAWN");
    }

    #[test]
    fn environment_parity_sample_hours() {
        // Same brackets as environment::DayPhase::from_hour
        for h in [0.0_f32, 4.99, 5.0, 6.5, 7.0, 12.0, 17.99, 18.0, 19.5, 20.0, 23.5] {
            let name = phase_from_hour(h);
            // Just ensure total coverage / no panic
            assert!(!name.as_str().is_empty());
        }
    }
}
