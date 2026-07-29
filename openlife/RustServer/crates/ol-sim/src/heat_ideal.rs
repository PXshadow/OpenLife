//! Pure heat / body-temperature ideal helpers (OHOL heat meter subset).
//!
//! Heat is a continuous value in `[0, 1]` where **0.5 is ideal** comfort.
//! Extremes drive extra food drain and movement penalties (callers apply).
//!
//! Chunk: **MAP-TEMP-PLAYER** also hosts Haxe `updateTemperature` body-heat step.

/// Ideal body heat (perfect comfort).
pub const IDEAL_HEAT: f32 = 0.5;
/// Comfort half-width: `|heat - ideal| <= COMFORT_RADIUS` → comfortable.
pub const COMFORT_RADIUS: f32 = 0.10;
/// Beyond this deviation from ideal → "extreme" (super hot / super cold).
pub const EXTREME_RADIUS: f32 = 0.35;
/// Extra food drain scale per unit |heat − ideal| (matches sim TEMP_FOOD_EXTRA spirit).
pub const HEAT_FOOD_EXTRA_SCALE: f32 = 0.10;
/// Cap on heat-driven extra food drain.
pub const HEAT_FOOD_EXTRA_CAP: f32 = 0.08;

/// Haxe `ServerSettings.TemperatureImpactPerSec`.
pub const TEMPERATURE_IMPACT_PER_SEC: f32 = 0.03;
/// Haxe `ServerSettings.TemperatureImpactPerSecIfGood`.
pub const TEMPERATURE_IMPACT_PER_SEC_IF_GOOD: f32 = 0.06;
/// Haxe `ServerSettings.TemperatureImpactReduction` (0 = disabled; Haxe TODO).
pub const TEMPERATURE_IMPACT_REDUCTION: f32 = 0.0;
/// Haxe `ServerSettings.TemperatureInWaterFactor`.
pub const TEMPERATURE_IN_WATER_FACTOR: f32 = 1.5;

/// Coarse comfort label for UI / SAY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatLabel {
    SuperCold,
    Cold,
    Cool,
    Ideal,
    Warm,
    Hot,
    SuperHot,
}

impl HeatLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SuperCold => "super_cold",
            Self::Cold => "cold",
            Self::Cool => "cool",
            Self::Ideal => "ideal",
            Self::Warm => "warm",
            Self::Hot => "hot",
            Self::SuperHot => "super_hot",
        }
    }
}

/// Clamp heat into `[0, 1]`; non-finite → ideal.
#[inline]
pub fn clamp_heat(heat: f32) -> f32 {
    if !heat.is_finite() {
        return IDEAL_HEAT;
    }
    heat.clamp(0.0, 1.0)
}

/// Absolute deviation from ideal.
#[inline]
pub fn heat_error(heat: f32) -> f32 {
    (clamp_heat(heat) - IDEAL_HEAT).abs()
}

/// Signed error: positive = too hot, negative = too cold.
#[inline]
pub fn heat_signed_error(heat: f32) -> f32 {
    clamp_heat(heat) - IDEAL_HEAT
}

/// True when within comfort band around ideal.
pub fn is_comfortable(heat: f32) -> bool {
    heat_error(heat) <= COMFORT_RADIUS
}

/// True when extremely hot (`heat >= ideal + EXTREME_RADIUS`).
pub fn is_super_hot(heat: f32) -> bool {
    clamp_heat(heat) >= IDEAL_HEAT + EXTREME_RADIUS
}

/// True when extremely cold (`heat <= ideal - EXTREME_RADIUS`).
pub fn is_super_cold(heat: f32) -> bool {
    clamp_heat(heat) <= IDEAL_HEAT - EXTREME_RADIUS
}

/// Label from heat value.
pub fn label_for_heat(heat: f32) -> HeatLabel {
    let h = clamp_heat(heat);
    let e = h - IDEAL_HEAT;
    if e <= -EXTREME_RADIUS {
        HeatLabel::SuperCold
    } else if e <= -COMFORT_RADIUS * 2.0 {
        HeatLabel::Cold
    } else if e < -COMFORT_RADIUS {
        HeatLabel::Cool
    } else if e <= COMFORT_RADIUS {
        HeatLabel::Ideal
    } else if e < COMFORT_RADIUS * 2.0 {
        HeatLabel::Warm
    } else if e < EXTREME_RADIUS {
        HeatLabel::Hot
    } else {
        HeatLabel::SuperHot
    }
}

/// Extra food drain from heat discomfort (0 when comfortable).
pub fn heat_food_extra(heat: f32) -> f32 {
    let err = heat_error(heat);
    if err <= COMFORT_RADIUS {
        return 0.0;
    }
    let over = err - COMFORT_RADIUS;
    (over * HEAT_FOOD_EXTRA_SCALE).min(HEAT_FOOD_EXTRA_CAP)
}

/// Move-speed multiplier from heat (1.0 ideal; down to ~0.7 at extremes).
pub fn heat_move_mult(heat: f32) -> f32 {
    let err = heat_error(heat);
    if err <= COMFORT_RADIUS {
        return 1.0;
    }
    // Linear to 0.7 at err=0.5
    let t = ((err - COMFORT_RADIUS) / (0.5 - COMFORT_RADIUS)).clamp(0.0, 1.0);
    1.0 - t * 0.30
}

/// Blend environmental temperature toward ideal by clothing warmth bonus.
///
/// `warmth` in `[0, 1.5]` typical; pulls heat toward ideal.
pub fn apply_clothing_warmth(heat: f32, warmth: f32) -> f32 {
    let h = clamp_heat(heat);
    let w = if warmth.is_finite() {
        warmth.clamp(0.0, 2.0)
    } else {
        0.0
    };
    // Pull fraction: up to 50% toward ideal at warmth=1.5
    let pull = (w / 1.5).clamp(0.0, 1.0) * 0.5;
    clamp_heat(h + (IDEAL_HEAT - h) * pull)
}

/// Combine biome temperature sample with optional indoor relief.
///
/// Indoor (`true`) pulls 30% toward ideal (floor / roof stub).
pub fn env_heat(biome_temp: f32, indoor: bool) -> f32 {
    let mut h = clamp_heat(biome_temp);
    if indoor {
        h = h + (IDEAL_HEAT - h) * 0.30;
    }
    clamp_heat(h)
}

/// Haxe `updateTemperature` body-heat integration (clothing/water subset).
///
/// ```text
/// heatchange = ambient - (0.5 * impactReduction + heat * (1 - impactReduction))
/// newTempPositive = (heat>0.5 && ambient<0.5) || (heat<0.5 && ambient>0.5)
/// rate = (good||water ? waterFactor : clothingFactor) * timeFactor * dt * heatchange
/// heat += rate; clamp 0..1
/// ```
///
/// `clothing_factor` is Haxe insulation/heat-protection factor (`1` = bare).
/// Full closest-heat-object / loved-biome / held-object branches are deferred.
pub fn body_heat_step(
    heat: f32,
    ambient: f32,
    dt: f32,
    clothing_factor: f32,
    in_water: bool,
) -> f32 {
    let heat = clamp_heat(heat);
    let ambient = if ambient.is_finite() { ambient } else { IDEAL_HEAT };
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let clothing = if clothing_factor.is_finite() && clothing_factor > 0.0 {
        clothing_factor
    } else {
        1.0
    };

    let impact_reduction = TEMPERATURE_IMPACT_REDUCTION.clamp(0.0, 1.0);
    let heatchange = ambient - (IDEAL_HEAT * impact_reduction + heat * (1.0 - impact_reduction));

    let new_temp_is_positive =
        (heat > IDEAL_HEAT && ambient < IDEAL_HEAT) || (heat < IDEAL_HEAT && ambient > IDEAL_HEAT);
    let time_factor = if new_temp_is_positive {
        TEMPERATURE_IMPACT_PER_SEC_IF_GOOD
    } else {
        TEMPERATURE_IMPACT_PER_SEC
    };
    let water_factor = if in_water {
        TEMPERATURE_IN_WATER_FACTOR
    } else {
        1.0
    };

    // Haxe: ignore clothing if heat change is positive or if in water
    let rate = if new_temp_is_positive || in_water {
        water_factor * time_factor * dt * heatchange
    } else {
        clothing * time_factor * dt * heatchange
    };
    clamp_heat(heat + rate)
}

/// `HEAT heat=H ideal=0.50 err=E label=L extra=X` query body.
pub fn format_heat_ideal_query(heat: f32) -> String {
    let h = clamp_heat(heat);
    let err = heat_error(h);
    let label = label_for_heat(h);
    let extra = heat_food_extra(h);
    format!(
        "HEAT heat={h:.2} ideal={IDEAL_HEAT:.2} err={err:.2} label={} extra={extra:.3}",
        label.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideal_and_clamp() {
        assert_eq!(clamp_heat(0.5), 0.5);
        assert_eq!(clamp_heat(-1.0), 0.0);
        assert_eq!(clamp_heat(2.0), 1.0);
        assert_eq!(clamp_heat(f32::NAN), IDEAL_HEAT);
        assert!((heat_error(0.5) - 0.0).abs() < 1e-6);
        assert!((heat_error(0.8) - 0.3).abs() < 1e-5);
        assert!((heat_signed_error(0.8) - 0.3).abs() < 1e-5);
        assert!((heat_signed_error(0.2) + 0.3).abs() < 1e-5);
    }

    #[test]
    fn comfort_and_extremes() {
        assert!(is_comfortable(0.5));
        assert!(is_comfortable(0.55));
        assert!(is_comfortable(0.45));
        assert!(!is_comfortable(0.7));
        assert!(!is_comfortable(0.3));
        assert!(is_super_hot(0.9));
        assert!(is_super_hot(0.85));
        assert!(!is_super_hot(0.7));
        assert!(is_super_cold(0.1));
        assert!(is_super_cold(0.15));
        assert!(!is_super_cold(0.3));
    }

    #[test]
    fn labels_cover_range() {
        assert_eq!(label_for_heat(0.5), HeatLabel::Ideal);
        assert_eq!(label_for_heat(0.55), HeatLabel::Ideal);
        assert_eq!(label_for_heat(0.65), HeatLabel::Warm);
        assert_eq!(label_for_heat(0.75), HeatLabel::Hot);
        assert_eq!(label_for_heat(0.9), HeatLabel::SuperHot);
        assert_eq!(label_for_heat(0.35), HeatLabel::Cool);
        assert_eq!(label_for_heat(0.25), HeatLabel::Cold);
        assert_eq!(label_for_heat(0.1), HeatLabel::SuperCold);
        assert_eq!(HeatLabel::Ideal.as_str(), "ideal");
        assert_eq!(HeatLabel::SuperHot.as_str(), "super_hot");
    }

    #[test]
    fn food_extra_and_move() {
        assert_eq!(heat_food_extra(0.5), 0.0);
        assert_eq!(heat_food_extra(0.55), 0.0);
        assert!(heat_food_extra(0.8) > 0.0);
        assert!(heat_food_extra(0.0) <= HEAT_FOOD_EXTRA_CAP);
        assert!((heat_move_mult(0.5) - 1.0).abs() < 1e-5);
        assert!(heat_move_mult(0.0) < 1.0);
        assert!(heat_move_mult(0.0) >= 0.7);
        assert!(heat_move_mult(1.0) >= 0.7);
    }

    #[test]
    fn clothing_and_indoor() {
        let cold = 0.2;
        let warmed = apply_clothing_warmth(cold, 1.5);
        assert!(warmed > cold);
        assert!(warmed < IDEAL_HEAT + 0.01);
        let hot = 0.9;
        let cooled = apply_clothing_warmth(hot, 1.5);
        assert!(cooled < hot);
        let indoor = env_heat(0.2, true);
        let outdoor = env_heat(0.2, false);
        assert!(indoor > outdoor);
        assert!((outdoor - 0.2).abs() < 1e-5);
    }

    #[test]
    fn format_query() {
        let s = format_heat_ideal_query(0.5);
        assert!(s.starts_with("HEAT heat=0.50"));
        assert!(s.contains("ideal=0.50"));
        assert!(s.contains("err=0.00"));
        assert!(s.contains("label=ideal"));
        assert!(s.contains("extra=0.000"));
        let s2 = format_heat_ideal_query(0.9);
        assert!(s2.contains("label=super_hot"));
        assert!(s2.contains("extra="));
    }

    #[test]
    fn constants_ordered() {
        assert!(COMFORT_RADIUS < EXTREME_RADIUS);
        assert!(IDEAL_HEAT + EXTREME_RADIUS <= 1.0 + 1e-6);
        assert!(IDEAL_HEAT - EXTREME_RADIUS >= 0.0 - 1e-6);
    }

    #[test]
    fn body_heat_step_warms_toward_hot_ambient() {
        let h0 = 0.5;
        let h1 = body_heat_step(h0, 1.0, 5.0, 1.0, false);
        assert!(h1 > h0, "heat rises toward hot ambient: {h1}");
        assert!(h1 <= 1.0);
    }

    #[test]
    fn body_heat_step_cools_toward_cold_ambient() {
        let h0 = 0.5;
        let h1 = body_heat_step(h0, 0.0, 5.0, 1.0, false);
        assert!(h1 < h0, "heat falls toward cold ambient: {h1}");
        assert!(h1 >= 0.0);
    }

    #[test]
    fn body_heat_step_good_temp_is_faster() {
        // Cold body, warm ambient → "good" (toward ideal) uses higher impact rate.
        let cold = 0.2;
        let good = body_heat_step(cold, 0.6, 1.0, 1.0, false);
        // Hot body, warmer ambient → "bad" (away from ideal) uses slower rate.
        let hot = 0.7;
        let bad = body_heat_step(hot, 0.9, 1.0, 1.0, false);
        let good_delta = (good - cold).abs();
        let bad_delta = (bad - hot).abs();
        assert!(
            good_delta > bad_delta,
            "good={good_delta} bad={bad_delta}"
        );
    }

    #[test]
    fn body_heat_step_clothing_slows_bad_drift() {
        let heat = 0.5;
        let bare = body_heat_step(heat, 0.0, 2.0, 1.0, false);
        let clothed = body_heat_step(heat, 0.0, 2.0, 0.2, false);
        // Clothing factor < 1 slows cooling away from ideal when cold ambient.
        assert!(
            clothed > bare,
            "clothed={clothed} should cool less than bare={bare}"
        );
    }
}
