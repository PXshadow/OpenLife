//! Pure age curves (OHOL / OpenLife life-rate subset).
//!
//! Maps age-in-years to fertility, food capacity, and drain multipliers.
//! No player state — pure functions only.

/// Real seconds per game-year (OpenLife default: 60s ≈ 1 year).
pub const SECONDS_PER_YEAR: f32 = 60.0;

/// Age (years) at which fertility opens.
pub const FERTILE_MIN: f32 = 14.0;
/// Age (years) at which fertility closes.
pub const FERTILE_MAX: f32 = 42.0;
/// Old-age food drain threshold.
pub const OLD_AGE: f32 = 60.0;
/// Hard death age ceiling used for curve clamping / notes.
pub const MAX_AGE: f32 = 120.0;
/// Infant stage upper bound (exclusive).
pub const INFANT_MAX: f32 = 3.0;
/// Child stage upper bound (exclusive).
pub const CHILD_MAX: f32 = 14.0;

/// Convert age years → real seconds lived.
#[inline]
pub fn age_to_seconds(age_years: f32) -> f32 {
    if !age_years.is_finite() || age_years < 0.0 {
        return 0.0;
    }
    age_years * SECONDS_PER_YEAR
}

/// Convert real seconds lived → age years.
#[inline]
pub fn seconds_to_age(secs: f32) -> f32 {
    if !secs.is_finite() || secs < 0.0 {
        return 0.0;
    }
    secs / SECONDS_PER_YEAR
}

/// Advance age by `dt` real seconds at rate `years_per_sec` (default 1/60).
pub fn advance_age(age: f32, dt: f32, years_per_sec: f32) -> f32 {
    if !age.is_finite() || age < 0.0 {
        return 0.0;
    }
    if !dt.is_finite() || dt <= 0.0 || !years_per_sec.is_finite() || years_per_sec <= 0.0 {
        return age.min(MAX_AGE);
    }
    (age + dt * years_per_sec).clamp(0.0, MAX_AGE)
}

/// Food capacity curve vs age (OHOL-ish: grows through youth, plateaus, dips late).
///
/// Returns a value in roughly `[5, 20]`:
/// - infant: 5 → 10
/// - child/teen: 10 → 20
/// - adult: 20
/// - elder: slowly down toward 12
pub fn food_max_for_age(age: f32) -> f32 {
    let a = if age.is_finite() { age.max(0.0) } else { 0.0 };
    if a < INFANT_MAX {
        // 0→3: 5 → 10
        return 5.0 + (a / INFANT_MAX) * 5.0;
    }
    if a < CHILD_MAX {
        // 3→14: 10 → 20
        let t = (a - INFANT_MAX) / (CHILD_MAX - INFANT_MAX);
        return 10.0 + t * 10.0;
    }
    if a < OLD_AGE {
        return 20.0;
    }
    // 60→120: 20 → 12
    let t = ((a - OLD_AGE) / (MAX_AGE - OLD_AGE)).clamp(0.0, 1.0);
    20.0 - t * 8.0
}

/// Food drain multiplier from age alone (1.0 baseline adult).
///
/// Infants burn slightly faster; elders burn more.
pub fn food_drain_mult_for_age(age: f32) -> f32 {
    let a = if age.is_finite() { age.max(0.0) } else { 0.0 };
    if a < INFANT_MAX {
        1.15
    } else if a < CHILD_MAX {
        1.05
    } else if a < OLD_AGE {
        1.0
    } else if a < 80.0 {
        1.5
    } else {
        1.75
    }
}

/// True if age is in the fertile window `[FERTILE_MIN, FERTILE_MAX]` (Haxe MaxAgeFertile **inclusive**).
pub fn is_fertile_age(age: f32) -> bool {
    age.is_finite() && age >= FERTILE_MIN && age <= FERTILE_MAX
}

/// Fraction through fertile window in `[0, 1]` (0 outside).
pub fn fertility_curve(age: f32) -> f32 {
    if !is_fertile_age(age) {
        return 0.0;
    }
    let span = FERTILE_MAX - FERTILE_MIN;
    let t = (age - FERTILE_MIN) / span;
    // Peak mid-window (simple smooth bump).
    let m = 4.0 * t * (1.0 - t); // 0 at ends, 1 at mid
    m.clamp(0.0, 1.0)
}

/// Move-speed age factor (infants slow, peak adult, elders slower).
pub fn move_speed_mult_for_age(age: f32) -> f32 {
    let a = if age.is_finite() { age.max(0.0) } else { 0.0 };
    if a < INFANT_MAX {
        0.55 + 0.15 * (a / INFANT_MAX)
    } else if a < CHILD_MAX {
        0.7 + 0.3 * ((a - INFANT_MAX) / (CHILD_MAX - INFANT_MAX))
    } else if a < OLD_AGE {
        1.0
    } else {
        // 60→100: 1.0 → 0.65
        let t = ((a - OLD_AGE) / 40.0).clamp(0.0, 1.0);
        1.0 - t * 0.35
    }
}

/// Years remaining until old-age threshold (0 if already old).
pub fn years_until_old(age: f32) -> f32 {
    if !age.is_finite() {
        return OLD_AGE;
    }
    (OLD_AGE - age).max(0.0)
}

/// `AGECURVE age=N food_max=M drain=D fertile=0|1` query body.
pub fn format_age_curve_query(age: f32) -> String {
    let a = if age.is_finite() { age.max(0.0) } else { 0.0 };
    let fm = food_max_for_age(a);
    let dm = food_drain_mult_for_age(a);
    let fert = if is_fertile_age(a) { 1 } else { 0 };
    format!(
        "AGECURVE age={a:.2} food_max={fm:.2} drain={dm:.2} fertile={fert}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_age_roundtrip() {
        assert!((age_to_seconds(1.0) - 60.0).abs() < 1e-5);
        assert!((seconds_to_age(60.0) - 1.0).abs() < 1e-5);
        assert!((seconds_to_age(age_to_seconds(14.5)) - 14.5).abs() < 1e-4);
        assert_eq!(age_to_seconds(-1.0), 0.0);
        assert_eq!(seconds_to_age(-5.0), 0.0);
        assert_eq!(age_to_seconds(f32::NAN), 0.0);
        assert_eq!(seconds_to_age(f32::INFINITY), 0.0);
    }

    #[test]
    fn advance_age_clamps() {
        let a = advance_age(10.0, 60.0, 1.0 / 60.0);
        assert!((a - 11.0).abs() < 1e-4);
        assert_eq!(advance_age(119.0, 600.0, 1.0 / 60.0), MAX_AGE);
        assert_eq!(advance_age(5.0, -1.0, 1.0 / 60.0), 5.0);
        assert_eq!(advance_age(f32::NAN, 1.0, 1.0 / 60.0), 0.0);
    }

    #[test]
    fn food_max_curve_shape() {
        assert!((food_max_for_age(0.0) - 5.0).abs() < 1e-4);
        assert!((food_max_for_age(3.0) - 10.0).abs() < 1e-4);
        assert!((food_max_for_age(14.0) - 20.0).abs() < 1e-4);
        assert!((food_max_for_age(30.0) - 20.0).abs() < 1e-4);
        assert!(food_max_for_age(90.0) < 20.0);
        assert!(food_max_for_age(90.0) > 12.0);
        assert!((food_max_for_age(120.0) - 12.0).abs() < 1e-3);
        // Monotone non-decreasing youth
        assert!(food_max_for_age(1.0) < food_max_for_age(2.0));
        assert!(food_max_for_age(5.0) < food_max_for_age(10.0));
    }

    #[test]
    fn drain_mult_brackets() {
        assert!((food_drain_mult_for_age(1.0) - 1.15).abs() < 1e-5);
        assert!((food_drain_mult_for_age(10.0) - 1.05).abs() < 1e-5);
        assert!((food_drain_mult_for_age(25.0) - 1.0).abs() < 1e-5);
        assert!((food_drain_mult_for_age(65.0) - 1.5).abs() < 1e-5);
        assert!((food_drain_mult_for_age(90.0) - 1.75).abs() < 1e-5);
    }

    #[test]
    fn fertility_window_and_curve() {
        assert!(!is_fertile_age(10.0));
        assert!(is_fertile_age(14.0));
        assert!(is_fertile_age(28.0));
        assert!(is_fertile_age(42.0), "MaxAgeFertile 42 inclusive (Haxe)");
        assert!(!is_fertile_age(42.01));
        assert!(!is_fertile_age(50.0));
        assert_eq!(fertility_curve(10.0), 0.0);
        assert_eq!(fertility_curve(14.0), 0.0); // t=0 → bump 0
        // Mid window ~28
        let mid = fertility_curve(28.0);
        assert!(mid > 0.9);
        assert!(fertility_curve(20.0) > 0.0);
        assert!(fertility_curve(20.0) < mid + 0.01);
    }

    #[test]
    fn move_speed_mult_shape() {
        assert!(move_speed_mult_for_age(1.0) < move_speed_mult_for_age(10.0));
        assert!((move_speed_mult_for_age(20.0) - 1.0).abs() < 1e-5);
        assert!(move_speed_mult_for_age(80.0) < 1.0);
        assert!(move_speed_mult_for_age(80.0) >= 0.65);
    }

    #[test]
    fn years_until_old_and_format() {
        assert!((years_until_old(40.0) - 20.0).abs() < 1e-5);
        assert_eq!(years_until_old(70.0), 0.0);
        let s = format_age_curve_query(28.0);
        assert!(s.starts_with("AGECURVE age=28.00"));
        assert!(s.contains("food_max=20.00"));
        assert!(s.contains("drain=1.00"));
        assert!(s.contains("fertile=1"));
        let s2 = format_age_curve_query(5.0);
        assert!(s2.contains("fertile=0"));
        assert!(s2.contains("food_max="));
    }

    #[test]
    fn nan_age_safe() {
        assert_eq!(food_max_for_age(f32::NAN), 5.0);
        assert_eq!(food_drain_mult_for_age(f32::NAN), 1.15);
        assert!(!is_fertile_age(f32::NAN));
        assert_eq!(fertility_curve(f32::NAN), 0.0);
    }
}
