//! Estimate current food drain/sec for status queries (`SAY ?DRAIN`).
//!
//! Mirrors the multiplicative factors in `tick_vitals` (biome / weather /
//! old-age / sleep / sit / sick) plus additive bleed / fire / snow, minus
//! clothing warmth relief. Temperature-extra / desert-extra are folded into
//! `base` by the caller when known.

use crate::environment::clothing_temp_bonus;
use crate::{
    OLD_AGE_FOOD_DRAIN_MULT, OLD_AGE_THRESHOLD, SICK_FOOD_DRAIN_MULT, SLEEP_FOOD_DRAIN_MULT,
    SIT_FOOD_DRAIN_MULT,
};

/// Bundle of drain factors for `?DRAIN` text.
#[derive(Debug, Clone, Copy)]
pub struct DrainEstimate {
    pub base: f32,
    pub biome_mult: f32,
    pub weather_mult: f32,
    pub old_age_mult: f32,
    pub sleep_mult: f32,
    pub sick_mult: f32,
    pub sit_mult: f32,
    pub bleed: f32,
    pub fire: f32,
    pub snow: f32,
    pub clothing_relief: f32,
}

impl DrainEstimate {
    /// Approximate total food/sec (same composition as vitals, floor 0.01).
    pub fn total(&self) -> f32 {
        let mut d = self.base * self.biome_mult * self.weather_mult;
        d *= self.old_age_mult * self.sleep_mult * self.sick_mult * self.sit_mult;
        d += self.bleed + self.fire + self.snow;
        d = (d - self.clothing_relief).max(0.01);
        d
    }

    /// `SAY ?DRAIN` body without leading p_id.
    pub fn format_query(&self) -> String {
        format!(
            "DRAIN total={:.3} base={:.3} biome={:.2} weather={:.2} age={:.2} sleep={:.2} sick={:.2} sit={:.2} bleed={:.3} fire={:.3} snow={:.3} warm={:.3}",
            self.total(),
            self.base,
            self.biome_mult,
            self.weather_mult,
            self.old_age_mult,
            self.sleep_mult,
            self.sick_mult,
            self.sit_mult,
            self.bleed,
            self.fire,
            self.snow,
            self.clothing_relief
        )
    }
}

/// Build a drain estimate from live player / world factors.
///
/// `base` should be `FOOD_USE_PER_SEC * day_night * apoc` (and any additive
/// temp extras the caller wants folded in). Clothing relief is `bonus * 0.02`
/// matching vitals.
pub fn estimate_food_drain(
    base: f32,
    biome_mult: f32,
    weather_mult: f32,
    age: f32,
    sleeping: bool,
    sitting: bool,
    sick: bool,
    bleed: f32,
    fire: f32,
    snow: f32,
    hat: i32,
    chest: i32,
    shoes: i32,
) -> DrainEstimate {
    let warm = clothing_temp_bonus(hat, chest, shoes);
    DrainEstimate {
        base,
        biome_mult,
        weather_mult,
        old_age_mult: if age > OLD_AGE_THRESHOLD {
            OLD_AGE_FOOD_DRAIN_MULT
        } else {
            1.0
        },
        sleep_mult: if sleeping {
            SLEEP_FOOD_DRAIN_MULT
        } else {
            1.0
        },
        sick_mult: if sick {
            SICK_FOOD_DRAIN_MULT
        } else {
            1.0
        },
        sit_mult: if sitting {
            SIT_FOOD_DRAIN_MULT
        } else {
            1.0
        },
        bleed,
        fire,
        snow,
        clothing_relief: warm * 0.02,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_and_format() {
        let e = DrainEstimate {
            base: 0.1,
            biome_mult: 1.0,
            weather_mult: 1.0,
            old_age_mult: 1.0,
            sleep_mult: 1.0,
            sick_mult: 1.0,
            sit_mult: 1.0,
            bleed: 0.0,
            fire: 0.0,
            snow: 0.0,
            clothing_relief: 0.0,
        };
        assert!((e.total() - 0.1).abs() < 1e-4);
        assert!(e.format_query().starts_with("DRAIN total="));
    }

    #[test]
    fn estimate_applies_clothing_and_state_mults() {
        let e = estimate_food_drain(
            0.1, 1.0, 1.0, 70.0, true, false, false, 0.0, 0.0, 0.0, 1, 1, 1,
        );
        assert!((e.old_age_mult - OLD_AGE_FOOD_DRAIN_MULT).abs() < 1e-6);
        assert!((e.sleep_mult - SLEEP_FOOD_DRAIN_MULT).abs() < 1e-6);
        assert!((e.clothing_relief - 1.5 * 0.02).abs() < 1e-6);
        // (0.1 * 1.5 * 0.5) - 0.03 = 0.045
        assert!((e.total() - 0.045).abs() < 1e-4);
    }
}
