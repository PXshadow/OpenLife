//! Pure food fill tables (OHOL eat / yum subset).
//!
//! Given food-value of an object, current store, capacity, and yum bonus,
//! compute how much food is restored and the resulting store.

/// Default adult food capacity (matches sim `MAX_FOOD` when not age-curved).
pub const DEFAULT_FOOD_MAX: f32 = 20.0;
/// Starting food on spawn.
pub const DEFAULT_START_FOOD: f32 = 10.0;

/// Cap for yum multiplier applied on top of base food value.
pub const MAX_YUM_MULT: f32 = 4.0;

/// Look-up style table entry: content `food_value` → base fill units.
///
/// OHOL stores food_value on objects (often small ints). This table documents
/// common bands used by the pure helpers; unknown values pass through as-is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillBand {
    pub min_value: i32,
    pub max_value: i32,
    /// Multiplier applied to food_value inside this band (usually 1.0).
    pub mult: f32,
}

/// Static bands: snack / meal / feast scaling (identity mults; reserved for tuning).
pub const FILL_BANDS: &[FillBand] = &[
    FillBand {
        min_value: 1,
        max_value: 3,
        mult: 1.0,
    },
    FillBand {
        min_value: 4,
        max_value: 8,
        mult: 1.0,
    },
    FillBand {
        min_value: 9,
        max_value: 20,
        mult: 1.0,
    },
    FillBand {
        min_value: 21,
        max_value: i32::MAX,
        mult: 1.0,
    },
];

/// Band label for diagnostics.
pub fn band_name(food_value: i32) -> &'static str {
    if food_value <= 0 {
        return "none";
    }
    if food_value <= 3 {
        "snack"
    } else if food_value <= 8 {
        "meal"
    } else if food_value <= 20 {
        "feast"
    } else {
        "mega"
    }
}

/// Band multiplier for `food_value` (1.0 if none).
pub fn band_mult(food_value: i32) -> f32 {
    if food_value <= 0 {
        return 0.0;
    }
    for b in FILL_BANDS {
        if food_value >= b.min_value && food_value <= b.max_value {
            return b.mult;
        }
    }
    1.0
}

/// Base fill from content food_value (before yum).
pub fn base_fill(food_value: i32) -> f32 {
    if food_value <= 0 {
        return 0.0;
    }
    (food_value as f32) * band_mult(food_value)
}

/// Effective fill with yum multiplier clamped to [`MAX_YUM_MULT`].
pub fn fill_with_yum(food_value: i32, yum_mult: f32) -> f32 {
    let base = base_fill(food_value);
    if base <= 0.0 {
        return 0.0;
    }
    let ym = if yum_mult.is_finite() {
        yum_mult.clamp(1.0, MAX_YUM_MULT)
    } else {
        1.0
    };
    base * ym
}

/// Apply fill to current food, clamping to `[0, food_max]`.
///
/// Returns `(new_food, actual_gained)`.
pub fn apply_fill(food: f32, food_max: f32, fill: f32) -> (f32, f32) {
    let max = if food_max.is_finite() && food_max > 0.0 {
        food_max
    } else {
        DEFAULT_FOOD_MAX
    };
    let cur = if food.is_finite() { food.max(0.0) } else { 0.0 };
    let add = if fill.is_finite() && fill > 0.0 {
        fill
    } else {
        0.0
    };
    let new = (cur + add).clamp(0.0, max);
    let gained = (new - cur).max(0.0);
    (new, gained)
}

/// Eat helper: food_value + yum → new store.
pub fn eat(food: f32, food_max: f32, food_value: i32, yum_mult: f32) -> (f32, f32) {
    apply_fill(food, food_max, fill_with_yum(food_value, yum_mult))
}

/// Room left before capacity.
pub fn room_left(food: f32, food_max: f32) -> f32 {
    let max = if food_max.is_finite() && food_max > 0.0 {
        food_max
    } else {
        DEFAULT_FOOD_MAX
    };
    let cur = if food.is_finite() { food.max(0.0) } else { 0.0 };
    (max - cur).max(0.0)
}

/// True if eating `food_value` would add nothing (full or non-food).
pub fn would_waste(food: f32, food_max: f32, food_value: i32, yum_mult: f32) -> bool {
    if food_value <= 0 {
        return true;
    }
    let fill = fill_with_yum(food_value, yum_mult);
    let (_, gained) = apply_fill(food, food_max, fill);
    gained <= 0.0
}

/// Fraction full in `[0, 1]`.
pub fn fill_ratio(food: f32, food_max: f32) -> f32 {
    let max = if food_max.is_finite() && food_max > 0.0 {
        food_max
    } else {
        DEFAULT_FOOD_MAX
    };
    let cur = if food.is_finite() { food.clamp(0.0, max) } else { 0.0 };
    (cur / max).clamp(0.0, 1.0)
}

/// `FILLTABLE value=N base=B yum=Y effective=E band=name` query body.
pub fn format_fill_table_query(food_value: i32, yum_mult: f32) -> String {
    let base = base_fill(food_value);
    let ym = if yum_mult.is_finite() {
        yum_mult.clamp(1.0, MAX_YUM_MULT)
    } else {
        1.0
    };
    let eff = fill_with_yum(food_value, ym);
    format!(
        "FILLTABLE value={food_value} base={base:.2} yum={ym:.2} effective={eff:.2} band={}",
        band_name(food_value)
    )
}

/// `FOODFILL food=F max=M room=R ratio=P` query body for current vitals.
pub fn format_food_fill_status(food: f32, food_max: f32) -> String {
    let room = room_left(food, food_max);
    let ratio = fill_ratio(food, food_max);
    let f = if food.is_finite() { food.max(0.0) } else { 0.0 };
    let m = if food_max.is_finite() && food_max > 0.0 {
        food_max
    } else {
        DEFAULT_FOOD_MAX
    };
    format!("FOODFILL food={f:.2} max={m:.2} room={room:.2} ratio={ratio:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_and_names() {
        assert_eq!(band_name(0), "none");
        assert_eq!(band_name(-1), "none");
        assert_eq!(band_name(2), "snack");
        assert_eq!(band_name(5), "meal");
        assert_eq!(band_name(12), "feast");
        assert_eq!(band_name(50), "mega");
        assert_eq!(band_mult(0), 0.0);
        assert_eq!(band_mult(2), 1.0);
        assert_eq!(band_mult(100), 1.0);
    }

    #[test]
    fn base_and_yum_fill() {
        assert_eq!(base_fill(0), 0.0);
        assert_eq!(base_fill(5), 5.0);
        assert_eq!(fill_with_yum(5, 1.0), 5.0);
        assert_eq!(fill_with_yum(5, 2.0), 10.0);
        assert_eq!(fill_with_yum(5, 10.0), 20.0); // clamped to MAX_YUM_MULT=4
        assert_eq!(fill_with_yum(5, 0.5), 5.0); // yum floor 1.0
        assert_eq!(fill_with_yum(5, f32::NAN), 5.0);
    }

    #[test]
    fn apply_fill_clamps() {
        let (n, g) = apply_fill(18.0, 20.0, 5.0);
        assert!((n - 20.0).abs() < 1e-5);
        assert!((g - 2.0).abs() < 1e-5);
        let (n2, g2) = apply_fill(10.0, 20.0, 3.0);
        assert!((n2 - 13.0).abs() < 1e-5);
        assert!((g2 - 3.0).abs() < 1e-5);
        let (n3, g3) = apply_fill(20.0, 20.0, 5.0);
        assert!((n3 - 20.0).abs() < 1e-5);
        assert_eq!(g3, 0.0);
        let (n4, g4) = apply_fill(5.0, 20.0, -1.0);
        assert!((n4 - 5.0).abs() < 1e-5);
        assert_eq!(g4, 0.0);
    }

    #[test]
    fn eat_composites() {
        let (n, g) = eat(10.0, 20.0, 5, 2.0);
        assert!((n - 20.0).abs() < 1e-5);
        assert!((g - 10.0).abs() < 1e-5);
        let (n2, g2) = eat(19.0, 20.0, 5, 1.0);
        assert!((n2 - 20.0).abs() < 1e-5);
        assert!((g2 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn room_ratio_waste() {
        assert!((room_left(5.0, 20.0) - 15.0).abs() < 1e-5);
        assert_eq!(room_left(20.0, 20.0), 0.0);
        assert!((fill_ratio(10.0, 20.0) - 0.5).abs() < 1e-5);
        assert!((fill_ratio(0.0, 20.0) - 0.0).abs() < 1e-5);
        assert!((fill_ratio(20.0, 20.0) - 1.0).abs() < 1e-5);
        assert!(would_waste(20.0, 20.0, 5, 1.0));
        assert!(!would_waste(10.0, 20.0, 5, 1.0));
        assert!(would_waste(10.0, 20.0, 0, 1.0));
    }

    #[test]
    fn formatters() {
        let t = format_fill_table_query(5, 2.0);
        assert!(t.contains("value=5"));
        assert!(t.contains("base=5.00"));
        assert!(t.contains("yum=2.00"));
        assert!(t.contains("effective=10.00"));
        assert!(t.contains("band=meal"));
        let s = format_food_fill_status(10.0, 20.0);
        assert!(s.contains("food=10.00"));
        assert!(s.contains("max=20.00"));
        assert!(s.contains("room=10.00"));
        assert!(s.contains("ratio=0.50"));
    }

    #[test]
    fn nan_and_defaults() {
        let (n, _) = apply_fill(f32::NAN, f32::NAN, 5.0);
        assert!((n - 5.0).abs() < 1e-5); // max defaults 20, cur 0
        assert!((room_left(f32::NAN, 0.0) - DEFAULT_FOOD_MAX).abs() < 1e-5);
        assert!((fill_ratio(f32::NAN, f32::NAN) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn fill_bands_cover_positive_ints() {
        for v in [1, 3, 4, 8, 9, 20, 21, 100] {
            assert!(band_mult(v) > 0.0, "v={v}");
            assert_ne!(band_name(v), "none");
        }
    }
}
