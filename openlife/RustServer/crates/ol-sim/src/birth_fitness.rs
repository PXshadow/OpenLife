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
/// Live path: `GameplayKnobs.eve_or_adam_birth_chance` (this const is the fallback).
pub const EVE_OR_ADAM_BIRTH_CHANCE: f32 = 0.025;

/// Haxe: `(SpawnAiAsEve || isHuman) && EveOrAdamBirthChance > rand`
///
/// `rand01` is 0..1. `no_mother` forces Eve (Rust `spawn_player`).
/// Chance is sanitized like live knobs (finite && >= 0, else compiled 0.025).
pub fn eve_or_adam_birth(
    rand01: f32,
    chance: f32,
    spawn_ai_as_eve: bool,
    is_human: bool,
    no_mother: bool,
) -> bool {
    if no_mother {
        return true;
    }
    if !(spawn_ai_as_eve || is_human) {
        return false;
    }
    let chance = if chance.is_finite() && chance >= 0.0 {
        chance
    } else {
        EVE_OR_ADAM_BIRTH_CHANCE
    };
    chance > rand01
}

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
    /// Haxe `CalculateParentChildFitness` result (added into mother score).
    pub parent_child_fitness: f32,
    /// Haxe `considerFamily` → `child.account.familyPrestige[founderId] / 20`.
    pub consider_family: bool,
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
    /// Squared Euclidean distance to mother (Haxe `CalculateQuadDistanceHelper`).
    pub dist_to_mother: f32,
    /// `p == mother.partner`
    pub is_partner: bool,
    /// `p.partner == mother`
    pub mother_is_partner: bool,
    pub little_kids_count: u32,
    pub is_female: bool,
    pub is_close_relative: bool,
    /// Mother already has a different partner.
    pub mother_has_other_partner: bool,
    /// `mother.father == p`
    pub is_mothers_father: bool,
    pub children_birth_mali: f32,
    pub family_prestige_for_child: f32,
    pub consider_family: bool,
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

/// Live spawn birth knobs (LittleKidsPerMother + AI/human mali).
// Haxe: ServerSettings.LittleKidsPerMother / AiMotherBirthMaliForHumanChild / HumanMotherBirthMaliForAiChild
// SETTINGS-KNOB-TAIL
#[derive(Debug, Clone, Copy)]
pub struct BirthSpawnKnobs {
    pub little_kids_per_mother: i32,
    pub ai_mother_birth_mali_for_human_child: f32,
    pub human_mother_birth_mali_for_ai_child: f32,
}

impl Default for BirthSpawnKnobs {
    fn default() -> Self {
        Self {
            little_kids_per_mother: 3,
            ai_mother_birth_mali_for_human_child: 3.0,
            human_mother_birth_mali_for_ai_child: 1.0,
        }
    }
}

/// Live Min/MaxAgeFertile variant of [`mother_fitness`].
// Haxe: CalculateMotherFitness + ServerSettings.MinAgeFertile / MaxAgeFertile
// C-SS-MORE-BATCH4
pub fn mother_fitness_ex(m: &MotherView, c: &ChildView, min_age: f32, max_age: f32) -> f32 {
    mother_fitness_with_birth_knobs(m, c, min_age, max_age, &BirthSpawnKnobs::default())
}

/// Haxe ineligible sentinel (`return -1000`).
pub const FITNESS_INELIGIBLE: f32 = -1000.0;
/// Haxe GetFittestMother: `if (tmpFitness < -100) continue`.
pub const FITNESS_SKIP_BELOW: f32 = -100.0;

#[inline]
pub fn fitness_is_eligible(fit: f32) -> bool {
    fit >= FITNESS_SKIP_BELOW
}

/// One living child of a parent for Haxe `CalculateParentChildFitness`.
#[derive(Debug, Clone, Copy)]
pub struct ParentChildKid {
    pub is_human: bool,
    pub age: f32,
}

/// Haxe `CalculateParentChildFitness`.
// Haxe: GlobalPlayerInstance.CalculateParentChildFitness L1558–1581
pub fn parent_child_fitness(
    parent_is_human: bool,
    child_is_human: bool,
    kids: &[ParentChildKid],
    min_age_to_eat: f32,
    little_kids_per_mother: i32,
) -> f32 {
    let mut fitness = 0.0f32;
    let mut count_little = 0i32;
    for k in kids {
        // Mixed incoming vs existing same-kind on opposite-kind parent costs extra.
        let mut factor = 1i32;
        if !child_is_human && !k.is_human && parent_is_human {
            factor = 2;
        }
        if child_is_human && k.is_human && !parent_is_human {
            factor = 2;
        }
        fitness -= factor as f32;
        if k.age > min_age_to_eat {
            continue;
        }
        fitness -= factor as f32;
        count_little += factor;
    }
    if little_kids_per_mother > 0 && count_little >= little_kids_per_mother {
        return FITNESS_INELIGIBLE;
    }
    fitness
}

/// [`mother_fitness_ex`] plus LittleKidsPerMother reject and AI/human birth mali.
// Haxe: CalculateMotherFitness (additive) + CalculateParentChildFitness
// SETTINGS-KNOB-TAIL
pub fn mother_fitness_with_birth_knobs(
    m: &MotherView,
    c: &ChildView,
    min_age: f32,
    max_age: f32,
    knobs: &BirthSpawnKnobs,
) -> f32 {
    // Haxe hard rejects → -1000
    if m.deleted || !m.is_female {
        return FITNESS_INELIGIBLE;
    }
    if !is_mother_age_fertile_ex(m.age, min_age, max_age) {
        return FITNESS_INELIGIBLE;
    }
    if m.wounded {
        return FITNESS_INELIGIBLE;
    }
    if m.food < 0.0 {
        return FITNESS_INELIGIBLE;
    }
    let max_exhaustion = if m.is_human == c.is_human { 10.0 } else { 5.0 };
    if m.exhaustion > max_exhaustion {
        return FITNESS_INELIGIBLE;
    }

    let mut fit = 0.0f32;
    fit += m.food / 10.0;
    fit += m.food_max / 10.0;
    let mother_class = PrestigeClass::from_i32(m.prestige_class as i32);
    let child_class = PrestigeClass::from_i32(c.prestige_class as i32);
    fit += calculate_class_boni(mother_class, child_class);
    if m.has_close_nonblocking_grave {
        fit += 3.0;
    }
    fit += m.prestige_from_eating.max(0.0) / 20.0;
    if m.consider_family {
        fit += m.family_prestige_for_child.max(0.0) / 20.0;
    }

    let temperature_mail = ((m.heat - 0.5) * 10.0).powi(2) / 10.0;
    fit -= temperature_mail;
    fit -= m.exhaustion / 5.0;
    fit -= m.children_birth_mali;
    if m.has_close_blocking_grave {
        fit -= 10.0;
    }
    if m.held_speed_mult > 1.1 {
        fit -= 1.0;
    }
    if m.held_id != 0 {
        fit -= 1.0;
    }
    if m.is_human && !c.is_human {
        let mali = if knobs.human_mother_birth_mali_for_ai_child.is_finite() {
            knobs.human_mother_birth_mali_for_ai_child.max(0.0)
        } else {
            1.0
        };
        fit -= mali;
    }
    if !m.is_human && c.is_human {
        let mali = if knobs.ai_mother_birth_mali_for_human_child.is_finite() {
            knobs.ai_mother_birth_mali_for_human_child.max(0.0)
        } else {
            3.0
        };
        fit -= mali;
    }
    fit += m.parent_child_fitness;
    fit
}

/// Father min age — Haxe `MaxAgeForAllowingClothAndPrickupFromOthers` (10).
pub const FATHER_MIN_AGE: f32 = 10.0;
/// Haxe `quadDist > 10000` reject (squared tiles).
pub const FATHER_MAX_QUAD_DIST: f32 = 10_000.0;

/// Father fitness. Ineligible → [`FITNESS_INELIGIBLE`].
// Haxe: GlobalPlayerInstance.CalculateFatherFitness
pub fn father_fitness(f: &FatherView, c: &ChildView, mother: &MotherView) -> f32 {
    if f.deleted || f.is_female {
        return FITNESS_INELIGIBLE;
    }
    if f.age < FATHER_MIN_AGE || f.age > FATHER_MAX_AGE {
        return FITNESS_INELIGIBLE;
    }
    if f.is_mothers_father {
        return FITNESS_INELIGIBLE;
    }
    if f.dist_to_mother > FATHER_MAX_QUAD_DIST {
        return FITNESS_INELIGIBLE;
    }

    let mut fit = 0.0f32;
    if f.is_partner {
        fit += 2.0;
    }
    if f.mother_is_partner {
        fit += 2.0;
    }
    fit += f.food_max / 10.0;
    let father_class = PrestigeClass::from_i32(f.prestige_class as i32);
    let mother_class = PrestigeClass::from_i32(mother.prestige_class as i32);
    fit += calculate_class_boni(father_class, mother_class);
    fit += f.prestige_from_eating.max(0.0) / 20.0;
    if f.consider_family {
        fit += f.family_prestige_for_child.max(0.0) / 20.0;
    }
    if f.age < 16.0 {
        fit -= 2.0;
    }
    if f.is_close_relative {
        fit -= 5.0;
    }
    if !f.is_partner && f.mother_has_other_partner {
        fit -= 2.0;
    }
    if f.wounded {
        fit -= 2.0;
    }
    fit -= f.dist_to_mother / 400.0;
    fit -= f.exhaustion / 5.0;
    fit -= f.children_birth_mali;
    if f.is_human && !c.is_human {
        fit -= 1.0; // HumanMotherBirthMaliForAiChild default; live knobs applied in pick
    }
    if !f.is_human && c.is_human {
        fit -= 3.0;
    }
    let _ = mother;
    fit
}

/// Father fitness with live AI/human mali knobs.
pub fn father_fitness_with_birth_knobs(
    f: &FatherView,
    c: &ChildView,
    mother: &MotherView,
    knobs: &BirthSpawnKnobs,
) -> f32 {
    let mut fit = father_fitness(f, c, mother);
    if fit <= FITNESS_INELIGIBLE + 0.5 {
        return fit;
    }
    // Re-apply mali from knobs (father_fitness used compiled defaults).
    if f.is_human && !c.is_human {
        fit += 1.0;
        fit -= if knobs.human_mother_birth_mali_for_ai_child.is_finite() {
            knobs.human_mother_birth_mali_for_ai_child.max(0.0)
        } else {
            1.0
        };
    }
    if !f.is_human && c.is_human {
        fit += 3.0;
        fit -= if knobs.ai_mother_birth_mali_for_human_child.is_finite() {
            knobs.ai_mother_birth_mali_for_human_child.max(0.0)
        } else {
            3.0
        };
    }
    fit
}

/// Haxe `mother.childrenBirthMali += 1` (unbounded).
pub fn next_children_birth_mali(current: f32) -> f32 {
    current + 1.0
}

/// Haxe grandmother / father `childrenBirthMali += 0.5`.
pub fn next_children_birth_mali_half(current: f32) -> f32 {
    current + 0.5
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
            parent_child_fitness: 0.0,
            consider_family: false,
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
        assert_eq!(mother_fitness(&m, &c), FITNESS_INELIGIBLE);
        m.age = 50.0;
        assert_eq!(mother_fitness(&m, &c), FITNESS_INELIGIBLE);
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
        assert_eq!(mother_fitness(&m, &c), FITNESS_INELIGIBLE);
        assert!(mother_fitness_ex(&m, &c, 12.0, 50.0) > 0.0);
        m.age = 12.0;
        assert_eq!(mother_fitness(&m, &c), FITNESS_INELIGIBLE);
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
        // Haxe: food/10 + food_max/10 + same-class +2 → 1.8 + 2.0 + 2.0 = 5.8
        assert!(
            (healthy - (1.8 + 2.0 + CLASS_BONI_SAME)).abs() < 0.05,
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
        assert!(fit_same > fit_common, "same={fit_same} common={fit_common}");
        assert!(
            fit_common > fit_noble,
            "common={fit_common} noble={fit_noble}"
        );
        // Noble↔Serf −3 on additive base ~3.8 → still eligible (~0.8).
        assert!(
            fit_noble > FITNESS_SKIP_BELOW,
            "noble-serf remains eligible under Haxe additive score {fit_noble}"
        );
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
        assert!(mother_fitness(&serf_m, &child_serf) > mother_fitness(&noble_m, &child_serf));
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
    fn mother_mixed_kind_mali_still_eligible() {
        let knobs = BirthSpawnKnobs::default();
        let mut ai_mom = healthy_mother();
        ai_mom.is_human = false;
        let human = human_child();
        let ai_child = ChildView {
            is_human: false,
            prestige_class: PrestigeClass::Commoner as u8,
        };
        let mixed = mother_fitness_with_birth_knobs(
            &ai_mom,
            &human,
            MOTHER_FERTILE_MIN,
            MOTHER_FERTILE_MAX,
            &knobs,
        );
        let same = mother_fitness_with_birth_knobs(
            &ai_mom,
            &ai_child,
            MOTHER_FERTILE_MIN,
            MOTHER_FERTILE_MAX,
            &knobs,
        );
        assert!(fitness_is_eligible(mixed), "AI mother + human child mali=3 still eligible {mixed}");
        assert!(same > mixed, "same-kind ranks above mixed mali");
        let human_mom = healthy_mother();
        let mixed_h = mother_fitness_with_birth_knobs(
            &human_mom,
            &ai_child,
            MOTHER_FERTILE_MIN,
            MOTHER_FERTILE_MAX,
            &knobs,
        );
        assert!(
            fitness_is_eligible(mixed_h),
            "human mother + AI child mali=1 still eligible {mixed_h}"
        );
    }

    #[test]
    fn little_kids_per_mother_hard_rejects() {
        let c = human_child();
        let mut m = healthy_mother();
        m.parent_child_fitness = FITNESS_INELIGIBLE;
        let knobs = BirthSpawnKnobs::default();
        assert!(
            mother_fitness_with_birth_knobs(&m, &c, MOTHER_FERTILE_MIN, MOTHER_FERTILE_MAX, &knobs)
                < FITNESS_SKIP_BELOW
        );
        m.parent_child_fitness = 0.0;
        assert!(
            mother_fitness_with_birth_knobs(&m, &c, MOTHER_FERTILE_MIN, MOTHER_FERTILE_MAX, &knobs)
                > 0.0
        );
    }

    #[test]
    fn mother_blocking_grave_zero() {
        let c = human_child();
        let mut m = healthy_mother();
        m.has_close_blocking_grave = true;
        let with_grave = mother_fitness(&m, &c);
        let without = mother_fitness(&healthy_mother(), &c);
        assert!((without - with_grave - 10.0).abs() < 1e-4, "blocking grave is −10 mali not a hard reject");
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
            mother_is_partner: true,
            little_kids_count: 0,
            is_female: false,
            is_close_relative: false,
            mother_has_other_partner: false,
            is_mothers_father: false,
            children_birth_mali: 0.0,
            family_prestige_for_child: 0.0,
            consider_family: false,
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
        assert_eq!(father_fitness(&old, &c, &mother), FITNESS_INELIGIBLE);
        let ai_dad = FatherView {
            is_human: false,
            ..near.clone()
        };
        let mixed = father_fitness(&ai_dad, &c, &mother);
        assert!(
            fitness_is_eligible(mixed),
            "AI father + human child is allowed with mali {mixed}"
        );
        assert!(father_fitness(&near, &c, &mother) > mixed);
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
            mother_is_partner: false,
            little_kids_count: 0,
            is_female: false,
            is_close_relative: false,
            mother_has_other_partner: false,
            is_mothers_father: false,
            children_birth_mali: 0.0,
            family_prestige_for_child: 0.0,
            consider_family: false,
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
    fn eve_or_adam_birth_no_mother_forces_eve() {
        assert!(eve_or_adam_birth(0.99, 0.0, false, false, true));
        assert!(eve_or_adam_birth(0.0, 0.0, false, false, true));
    }

    #[test]
    fn eve_or_adam_birth_ai_default_skips_roll() {
        // Default SpawnAiAsEve=false: AI/NPC does not take the Eve roll.
        assert!(!eve_or_adam_birth(0.0, 1.0, false, false, false));
    }

    #[test]
    fn eve_or_adam_birth_ai_spawn_ai_as_eve_roll() {
        assert!(eve_or_adam_birth(0.0, 1.0, true, false, false));
        assert!(!eve_or_adam_birth(0.5, 0.025, true, false, false));
    }

    #[test]
    fn eve_or_adam_birth_human_takes_roll_without_spawn_ai() {
        assert!(eve_or_adam_birth(0.0, 1.0, false, true, false));
    }

    #[test]
    fn mali_steps() {
        assert!((next_children_birth_mali(0.0) - 1.0).abs() < 1e-5);
        assert!((next_children_birth_mali(2.0) - 3.0).abs() < 1e-5);
        assert!((next_children_birth_mali_half(1.0) - 1.5).abs() < 1e-5);
    }

    #[test]
    fn parent_child_mixed_kind_factor() {
        let kids = [
            ParentChildKid {
                is_human: true,
                age: 1.0,
            },
            ParentChildKid {
                is_human: true,
                age: 1.0,
            },
        ];
        let fit = parent_child_fitness(false, true, &kids, 3.0, 3);
        assert_eq!(fit, FITNESS_INELIGIBLE);
        let one = [ParentChildKid {
            is_human: true,
            age: 1.0,
        }];
        let ok = parent_child_fitness(false, true, &one, 3.0, 3);
        assert_eq!(ok, -4.0);
    }
}
