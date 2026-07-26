//! Pure Haxe-style birth fitness (mother / father / parent-child).
//!
//! Ported as pure functions + fixture tables. Missing Rust fields
//! (`prestige_from_eating`, graves) default to 0 / false — do not invent weights.
//!
//! Haxe references:
//! - `CalculateMotherFitness` / `CalculateFatherFitness` / `CalculateParentChildFitness`
//! - `EveOrAdamBirthChance = 0.025`
//! - Mother fertile ages 14–42 (`MaxAgeFertile`); father fitness rejects age > 55.

/// Eve/Adam wild birth chance when no suitable mother (Haxe).
pub const EVE_OR_ADAM_BIRTH_CHANCE: f32 = 0.025;

/// Mother fertile max (years) — matches `age_curves::FERTILE_MAX` (Haxe inclusive).
pub const MOTHER_FERTILE_MAX: f32 = 42.0;
/// Mother fertile min.
pub const MOTHER_FERTILE_MIN: f32 = 14.0;
/// Father fitness age reject above this.
pub const FATHER_MAX_AGE: f32 = 55.0;

/// Shared mother age band (inclusive 14–42).
#[inline]
pub fn is_mother_age_fertile(age: f32) -> bool {
    age.is_finite() && age >= MOTHER_FERTILE_MIN && age <= MOTHER_FERTILE_MAX
}

#[derive(Debug, Clone)]
pub struct MotherView {
    pub deleted: bool,
    pub is_female: bool,
    pub age: f32,
    pub food: f32,
    pub food_max: f32,
    pub exhaustion: f32,
    /// 0..1 temperature / heat.
    pub heat: f32,
    pub wounded: bool,
    pub held_id: i32,
    /// >1.1 mali
    pub held_speed_mult: f32,
    pub children_birth_mali: f32,
    pub prestige_class: u8,
    /// 0 if field missing in Rust yet.
    pub prestige_from_eating: f32,
    pub family_prestige_for_child: f32,
    pub has_close_nonblocking_grave: bool,
    pub has_close_blocking_grave: bool,
    pub is_human: bool,
    /// Input to parent-child fitness.
    pub little_kids_count: u32,
}

#[derive(Debug, Clone)]
pub struct ChildView {
    pub is_human: bool,
    pub prestige_class: u8,
}

#[derive(Debug, Clone)]
pub struct FatherView {
    pub deleted: bool,
    pub age: f32,
    pub food: f32,
    pub food_max: f32,
    pub exhaustion: f32,
    pub heat: f32,
    pub wounded: bool,
    pub held_id: i32,
    pub held_speed_mult: f32,
    pub prestige_class: u8,
    pub prestige_from_eating: f32,
    pub is_human: bool,
    /// Chebyshev (or world) distance to mother.
    pub dist_to_mother: f32,
    pub is_partner: bool,
    pub little_kids_count: u32,
}

/// Mother fitness score (higher = more likely selected). Returns 0 if ineligible.
///
/// Simplified Haxe-shaped formula for parity tests (not bit-identical float path):
/// base 1.0, age band, food ratio, heat comfort, wounds, held load, birth mali,
/// little-kids pressure, prestige soft factors, grave soft mali.
pub fn mother_fitness(m: &MotherView, c: &ChildView) -> f32 {
    if m.deleted || !m.is_female || !m.is_human {
        return 0.0;
    }
    if !is_mother_age_fertile(m.age) {
        return 0.0;
    }
    if m.has_close_blocking_grave {
        return 0.0;
    }

    let mut fit = 1.0f32;

    // Food fullness favors mothers who can nurse.
    let food_ratio = if m.food_max > 0.0 {
        (m.food / m.food_max).clamp(0.0, 1.5)
    } else {
        0.0
    };
    fit *= 0.5 + 0.5 * food_ratio;

    // Heat comfort around 0.5.
    let heat_err = (m.heat - 0.5).abs();
    fit *= (1.0 - heat_err).clamp(0.2, 1.0);

    // Exhaustion / wounds.
    fit *= (1.0 - m.exhaustion.clamp(0.0, 0.9)).max(0.1);
    if m.wounded {
        fit *= 0.5;
    }

    // Held object slows / blocks.
    if m.held_id != 0 {
        fit *= 0.85;
    }
    if m.held_speed_mult > 1.1 {
        fit *= 0.7;
    }

    // Accumulated birth mali (more children → lower fitness).
    fit *= (1.0 - m.children_birth_mali.clamp(0.0, 0.9)).max(0.05);

    // Little kids nearby pressure (parent-child).
    fit *= parent_child_factor(m.little_kids_count, c);

    // Prestige soft bonus (class 0..N).
    fit *= 1.0 + 0.02 * m.prestige_class as f32;
    fit *= 1.0 + 0.001 * m.prestige_from_eating.max(0.0);
    fit *= 1.0 + 0.001 * m.family_prestige_for_child.max(0.0);

    if m.has_close_nonblocking_grave {
        fit *= 0.9;
    }

    // Cross-species soft pen.
    if !c.is_human {
        fit *= 0.5;
    }

    fit.max(0.0)
}

/// Father fitness; 0 if age > 55 or deleted.
pub fn father_fitness(f: &FatherView, c: &ChildView, _mother: &MotherView) -> f32 {
    if f.deleted || !f.is_human {
        return 0.0;
    }
    if f.age > FATHER_MAX_AGE || f.age < 14.0 {
        return 0.0;
    }

    let mut fit = 1.0f32;

    let food_ratio = if f.food_max > 0.0 {
        (f.food / f.food_max).clamp(0.0, 1.5)
    } else {
        0.0
    };
    fit *= 0.5 + 0.5 * food_ratio;
    let heat_err = (f.heat - 0.5).abs();
    fit *= (1.0 - heat_err).clamp(0.2, 1.0);
    fit *= (1.0 - f.exhaustion.clamp(0.0, 0.9)).max(0.1);
    if f.wounded {
        fit *= 0.5;
    }
    if f.held_id != 0 {
        fit *= 0.9;
    }
    if f.held_speed_mult > 1.1 {
        fit *= 0.8;
    }

    // Distance: closer fathers preferred; beyond 40 tiles soft floor.
    let dist = f.dist_to_mother.max(0.0);
    fit *= (1.0 / (1.0 + dist / 20.0)).clamp(0.05, 1.0);

    if f.is_partner {
        fit *= 1.5;
    }

    fit *= parent_child_factor(f.little_kids_count, c);
    fit *= 1.0 + 0.02 * f.prestige_class as f32;
    fit *= 1.0 + 0.001 * f.prestige_from_eating.max(0.0);

    if !c.is_human {
        fit *= 0.5;
    }

    fit.max(0.0)
}

fn parent_child_factor(little_kids: u32, c: &ChildView) -> f32 {
    // More little kids → lower fitness (care burden).
    let base = 1.0 / (1.0 + 0.15 * little_kids as f32);
    // Prestige class of child slightly modulates (Haxe subset).
    base * (1.0 + 0.01 * c.prestige_class as f32)
}

/// Increment children_birth_mali after a successful birth (Haxe-shaped step).
pub fn next_children_birth_mali(current: f32) -> f32 {
    (current + 0.1).min(0.9)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_mother() -> MotherView {
        MotherView {
            deleted: false,
            is_female: true,
            age: 25.0,
            food: 18.0,
            food_max: 20.0,
            exhaustion: 0.0,
            heat: 0.5,
            wounded: false,
            held_id: 0,
            held_speed_mult: 1.0,
            children_birth_mali: 0.0,
            prestige_class: 2,
            prestige_from_eating: 0.0,
            family_prestige_for_child: 0.0,
            has_close_nonblocking_grave: false,
            has_close_blocking_grave: false,
            is_human: true,
            little_kids_count: 0,
        }
    }

    fn human_child() -> ChildView {
        ChildView {
            is_human: true,
            prestige_class: 0,
        }
    }

    #[test]
    fn mother_ineligible_outside_age() {
        let c = human_child();
        let mut m = healthy_mother();
        m.age = 10.0;
        assert_eq!(mother_fitness(&m, &c), 0.0);
        m.age = 50.0;
        assert_eq!(mother_fitness(&m, &c), 0.0);
        m.age = 42.0;
        assert!(mother_fitness(&m, &c) > 0.0);
    }

    #[test]
    fn mother_fixture_healthy_above_starving() {
        let c = human_child();
        let healthy = mother_fitness(&healthy_mother(), &c);
        let mut starving = healthy_mother();
        starving.food = 1.0;
        let low = mother_fitness(&starving, &c);
        assert!(healthy > low, "healthy={healthy} low={low}");
        // Hand-calc band: food_ratio 0.9 → fit *= 0.95; prestige 2 → *1.04
        // Expected roughly 0.95 * 1.04 ≈ 0.988
        assert!(
            (healthy - 0.988).abs() < 0.05,
            "healthy fitness fixture {healthy}"
        );
    }

    #[test]
    fn mother_birth_mali_reduces() {
        let c = human_child();
        let a = mother_fitness(&healthy_mother(), &c);
        let mut m = healthy_mother();
        m.children_birth_mali = 0.5;
        let b = mother_fitness(&m, &c);
        assert!(b < a);
    }

    #[test]
    fn mother_blocking_grave_zero() {
        let c = human_child();
        let mut m = healthy_mother();
        m.has_close_blocking_grave = true;
        assert_eq!(mother_fitness(&m, &c), 0.0);
    }

    #[test]
    fn father_age_and_distance() {
        let c = human_child();
        let mother = healthy_mother();
        let near = FatherView {
            deleted: false,
            age: 30.0,
            food: 15.0,
            food_max: 20.0,
            exhaustion: 0.0,
            heat: 0.5,
            wounded: false,
            held_id: 0,
            held_speed_mult: 1.0,
            prestige_class: 1,
            prestige_from_eating: 0.0,
            is_human: true,
            dist_to_mother: 2.0,
            is_partner: true,
            little_kids_count: 0,
        };
        let far = FatherView {
            dist_to_mother: 80.0,
            is_partner: false,
            ..near.clone()
        };
        let old = FatherView {
            age: 60.0,
            ..near.clone()
        };
        assert!(father_fitness(&near, &c, &mother) > father_fitness(&far, &c, &mother));
        assert_eq!(father_fitness(&old, &c, &mother), 0.0);
    }

    #[test]
    fn eve_chance_constant() {
        assert!((EVE_OR_ADAM_BIRTH_CHANCE - 0.025).abs() < 1e-6);
    }

    #[test]
    fn mali_steps() {
        assert!((next_children_birth_mali(0.0) - 0.1).abs() < 1e-5);
        assert!((next_children_birth_mali(0.85) - 0.9).abs() < 1e-5);
    }
}
