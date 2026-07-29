//! Pure Haxe-style birth fitness (mother / father / parent-child).
//!
//! Ported as pure functions + fixture tables. Missing Rust fields
//! (`prestige_from_eating`, graves) default to 0 / false — do not invent weights.
//!
//! Haxe references:
//! - `CalculateMotherFitness` / `CalculateFatherFitness` / `CalculateParentChildFitness`
//! - `calculateClassBoni` (CLASS-BONI / prestige_class_table)
//! - `EveOrAdamBirthChance = 0.025`
//! - Mother fertile ages 14–42 (`MaxAgeFertile`); father fitness rejects age > 55.

use crate::prestige::{calculate_class_boni, PrestigeClass};

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
    is_mother_age_fertile_ex(age, MOTHER_FERTILE_MIN, MOTHER_FERTILE_MAX)
}

/// Live Min/MaxAgeFertile mother band (inclusive both ends).
// Haxe: ServerSettings.MinAgeFertile / MaxAgeFertile
// C-SS-MORE-BATCH4
#[inline]
pub fn is_mother_age_fertile_ex(age: f32, min_age: f32, max_age: f32) -> bool {
    crate::fertility::age_fertile_ex(age, min_age, max_age)
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
    /// Haxe `lineage.prestigeClass` int tag (see [`PrestigeClass`]).
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
    /// Haxe child `lineage.prestigeClass` (set at spawn before mother pick).
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
    /// Haxe father `lineage.prestigeClass`.
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
/// little-kids pressure, prestige soft factors, **class boni** (`calculateClassBoni`),
/// grave soft mali.
// Haxe: GlobalPlayerInstance.CalculateMotherFitness + calculateClassBoni
pub fn mother_fitness(m: &MotherView, c: &ChildView) -> f32 {
    mother_fitness_ex(m, c, MOTHER_FERTILE_MIN, MOTHER_FERTILE_MAX)
}

/// Live Min/MaxAgeFertile variant of [`mother_fitness`].
// Haxe: CalculateMotherFitness + ServerSettings.MinAgeFertile / MaxAgeFertile
// C-SS-MORE-BATCH4
pub fn mother_fitness_ex(m: &MotherView, c: &ChildView, min_age: f32, max_age: f32) -> f32 {
    if m.deleted || !m.is_female || !m.is_human {
        return 0.0;
    }
    if !is_mother_age_fertile_ex(m.age, min_age, max_age) {
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
    fit *= parent_child_factor(m.little_kids_count);

    // Soft prestige-from-eating / family prestige (Haxe /20 additive → soft mult).
    fit *= 1.0 + 0.001 * m.prestige_from_eating.max(0.0);
    fit *= 1.0 + 0.001 * m.family_prestige_for_child.max(0.0);

    // CLASS-BONI: Haxe `tmpFitness += p.calculateClassBoni(child)`
    // Additive on multiplicative base so same-class (+2) strongly ranks above Noble↔Serf (−3).
    let mother_class = PrestigeClass::from_i32(m.prestige_class as i32);
    let child_class = PrestigeClass::from_i32(c.prestige_class as i32);
    fit += calculate_class_boni(mother_class, child_class);

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
// Haxe: GlobalPlayerInstance.CalculateFatherFitness + calculateClassBoni(mother)
pub fn father_fitness(f: &FatherView, c: &ChildView, mother: &MotherView) -> f32 {
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

    fit *= parent_child_factor(f.little_kids_count);
    fit *= 1.0 + 0.001 * f.prestige_from_eating.max(0.0);

    // CLASS-BONI: Haxe `tmpFitness += p.calculateClassBoni(mother)`
    // Father compares himself to mother (not the child).
    let father_class = PrestigeClass::from_i32(f.prestige_class as i32);
    let mother_class = PrestigeClass::from_i32(mother.prestige_class as i32);
    fit += calculate_class_boni(father_class, mother_class);

    // Child human-ness soft pen (father path does not use class boni on child).
    let _ = c;
    if !c.is_human {
        fit *= 0.5;
    }

    fit.max(0.0)
}

fn parent_child_factor(little_kids: u32) -> f32 {
    // More little kids → lower fitness (care burden).
    // Haxe CalculateParentChildFitness does not use prestige class.
    1.0 / (1.0 + 0.15 * little_kids as f32)
}

/// Increment children_birth_mali after a successful birth (Haxe-shaped step).
pub fn next_children_birth_mali(current: f32) -> f32 {
    (current + 0.1).min(0.9)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prestige::{CLASS_BONI_NOBLE_SERF, CLASS_BONI_SAME};

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
            prestige_class: PrestigeClass::Commoner as u8,
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
            // Match Commoner mother → same-class boni +2 by default in healthy fixtures.
            prestige_class: PrestigeClass::Commoner as u8,
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

    /// C-SS-MORE-BATCH4: live fertile band allows age 50 when max=50.
    // Haxe: ServerSettings.MinAgeFertile / MaxAgeFertile
    #[test]
    fn mother_fitness_ex_live_age_band() {
        let c = human_child();
        let mut m = healthy_mother();
        m.age = 50.0;
        assert_eq!(mother_fitness(&m, &c), 0.0);
        assert!(mother_fitness_ex(&m, &c, 12.0, 50.0) > 0.0);
        m.age = 12.0;
        assert_eq!(mother_fitness(&m, &c), 0.0);
        assert!(mother_fitness_ex(&m, &c, 12.0, 50.0) > 0.0);
        assert!(is_mother_age_fertile_ex(45.0, 14.0, 50.0));
        assert!(!is_mother_age_fertile_ex(45.0, 14.0, 42.0));
    }

    #[test]
    fn mother_fixture_healthy_above_starving() {
        let c = human_child();
        let healthy = mother_fitness(&healthy_mother(), &c);
        let mut starving = healthy_mother();
        starving.food = 1.0;
        let low = mother_fitness(&starving, &c);
        assert!(healthy > low, "healthy={healthy} low={low}");
        // food_ratio 0.9 → *0.95; same-class +2 → ~2.95
        assert!(
            (healthy - (0.95 + CLASS_BONI_SAME)).abs() < 0.05,
            "healthy fitness fixture {healthy}"
        );
    }

    #[test]
    fn mother_class_boni_same_beats_noble_serf() {
        let child_serf = ChildView {
            is_human: true,
            prestige_class: PrestigeClass::Serf as u8,
        };
        let mut same = healthy_mother();
        same.prestige_class = PrestigeClass::Serf as u8;
        let mut noble = healthy_mother();
        noble.prestige_class = PrestigeClass::Noble as u8;
        let mut commoner = healthy_mother();
        commoner.prestige_class = PrestigeClass::Commoner as u8;

        let fit_same = mother_fitness(&same, &child_serf);
        let fit_noble = mother_fitness(&noble, &child_serf);
        let fit_common = mother_fitness(&commoner, &child_serf);

        // same +2, noble−serf −3, commoner–serf 0
        assert!(
            fit_same > fit_common,
            "same={fit_same} common={fit_common}"
        );
        assert!(
            fit_common > fit_noble,
            "common={fit_common} noble={fit_noble}"
        );
        // Noble↔Serf still eligible if base mult keeps score > 0 (base ~0.95 − 3 < 0 → 0).
        assert_eq!(fit_noble, 0.0, "noble-serf mali zeros multiplicative~1 base");
        assert!((fit_same - fit_common - CLASS_BONI_SAME).abs() < 0.05);
        let _ = CLASS_BONI_NOBLE_SERF;
    }

    /// Child Serf vs Commoner changes mother rank under pure mother_fitness.
    #[test]
    fn mother_fitness_child_class_changes_rank() {
        let mut serf_m = healthy_mother();
        serf_m.prestige_class = PrestigeClass::Serf as u8;
        let mut noble_m = healthy_mother();
        noble_m.prestige_class = PrestigeClass::Noble as u8;

        let child_serf = ChildView {
            is_human: true,
            prestige_class: PrestigeClass::Serf as u8,
        };
        let child_common = ChildView {
            is_human: true,
            prestige_class: PrestigeClass::Commoner as u8,
        };

        // Serf child: same-class Serf mother beats Noble (Noble−Serf −3 → 0).
        assert!(
            mother_fitness(&serf_m, &child_serf) > mother_fitness(&noble_m, &child_serf)
        );
        // Commoner child: Noble and Serf both get 0 boni; base equal → equal fitness.
        let ns = mother_fitness(&serf_m, &child_common);
        let nn = mother_fitness(&noble_m, &child_common);
        assert!((ns - nn).abs() < 1e-4, "serf={ns} noble={nn}");
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
            prestige_class: PrestigeClass::Commoner as u8,
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
    fn father_class_boni_vs_mother() {
        let c = human_child();
        let mut mother = healthy_mother();
        mother.prestige_class = PrestigeClass::Noble as u8;
        let base = FatherView {
            deleted: false,
            age: 30.0,
            food: 20.0,
            food_max: 20.0,
            exhaustion: 0.0,
            heat: 0.5,
            wounded: false,
            held_id: 0,
            held_speed_mult: 1.0,
            prestige_class: PrestigeClass::Noble as u8,
            prestige_from_eating: 0.0,
            is_human: true,
            dist_to_mother: 1.0,
            is_partner: false,
            little_kids_count: 0,
        };
        let serf = FatherView {
            prestige_class: PrestigeClass::Serf as u8,
            ..base.clone()
        };
        let fit_same = father_fitness(&base, &c, &mother);
        let fit_serf = father_fitness(&serf, &c, &mother);
        assert!(fit_same > fit_serf, "same={fit_same} serf={fit_serf}");
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
