//! Hunt / animal damage pure helpers.
//!
//! Also hosts **COMBAT-MOSQUITO-KIND** pure jungle-love helpers used by fever/path
//! moskito damage (live wire in `lib.rs` via `_apply_mosquito_rest.py` residual).

use crate::animals::{AnimalKind, AnimalWorld};
use crate::food_store_max::{biome_love_factor, BIOME_TAG_JUNGLE};
use crate::weapon_wound::moskito_damage_factor;

/// Meat placeholder object id (0 = none until content wired).
pub const HUNT_MEAT_OBJECT_ID: i32 = 0;

/// Damage to apply per successful HUNT.
pub const HUNT_DAMAGE: i32 = 5;

/// Prestige gain on animal kill.
pub const HUNT_KILL_PRESTIGE: f32 = 0.25;

/// Chebyshev range for `SAY HUNT` (adjacent tile, including own tile).
pub const HUNT_RANGE: i32 = 1;

/// Result of a hunt attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HuntResult {
    Miss,
    Hit {
        animal_id: i32,
        kind: AnimalKind,
        hp_left: i32,
    },
    Kill {
        animal_id: i32,
        kind: AnimalKind,
        /// Tile where the animal stood (for clearing the map object).
        x: i32,
        y: i32,
    },
}

/// Damage nearest animal within `range` of `(x, y)` via [`AnimalWorld::damage`].
pub fn hunt_nearest(
    animals: &mut AnimalWorld,
    x: i32,
    y: i32,
    range: i32,
    damage: i32,
) -> HuntResult {
    let Some(id) = animals.nearest_id(x, y, range) else {
        return HuntResult::Miss;
    };
    let (ax, ay) = animals
        .get(id)
        .map(|a| (a.x, a.y))
        .unwrap_or((x, y));
    match animals.damage(id, damage) {
        Some((0, kind, true)) => HuntResult::Kill {
            animal_id: id,
            kind,
            x: ax,
            y: ay,
        },
        Some((hp_left, kind, false)) => HuntResult::Hit {
            animal_id: id,
            kind,
            hp_left,
        },
        // Id vanished between nearest_id and damage (should not happen single-threaded).
        _ => HuntResult::Miss,
    }
}

// ── COMBAT-MOSQUITO-KIND pure ─────────────────────────────────────────────

/// Haxe `biomeLoveFactor(BiomeTag.JUNGLE)` for mosquito damage / fever infect.
// Haxe: GlobalPlayerInstance.DoDamage lovesJungle L4647
// COMBAT-MOSQUITO-KIND
#[inline]
pub fn jungle_biome_love_for_mosquito(
    floor_id: i32,
    self_color: i32,
    mother_color: Option<i32>,
    father_color: Option<i32>,
) -> f32 {
    biome_love_factor(
        BIOME_TAG_JUNGLE,
        floor_id,
        self_color,
        mother_color,
        father_color,
    )
}

/// Haxe `moskitoDamageFactor` from person colors + yellowfever resistance + health.
// Haxe: DoDamage moskitoDamageFactor L4647–4652
// COMBAT-MOSQUITO-KIND
#[inline]
pub fn moskito_damage_factor_from_love(
    floor_id: i32,
    self_color: i32,
    mother_color: Option<i32>,
    father_color: Option<i32>,
    yellowfever_count: f32,
    health_factor: f32,
) -> f32 {
    let loves = jungle_biome_love_for_mosquito(floor_id, self_color, mother_color, father_color);
    moskito_damage_factor(loves, yellowfever_count, health_factor)
}

/// Apply Haxe mosquito damage scale onto rolled path damage.
// Haxe: else damage *= moskitoDamageFactor L4660
#[inline]
pub fn scale_damage_by_moskito_factor(applied_damage: f32, moskito_factor: f32) -> f32 {
    let d = if applied_damage.is_finite() {
        applied_damage.max(0.0)
    } else {
        0.0
    };
    let f = if moskito_factor.is_finite() {
        moskito_factor.max(0.0)
    } else {
        1.0
    };
    d * f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_hit_kill() {
        let mut w = AnimalWorld::default();
        assert_eq!(hunt_nearest(&mut w, 0, 0, 1, 5), HuntResult::Miss);
        w.spawn(AnimalKind::Rabbit, 0, 0);
        // Rabbit default hp 5
        match hunt_nearest(&mut w, 0, 0, 1, 3) {
            HuntResult::Hit {
                hp_left,
                kind,
                animal_id,
            } => {
                assert_eq!(hp_left, 2);
                assert_eq!(kind, AnimalKind::Rabbit);
                assert_eq!(animal_id, 0);
            }
            o => panic!("{o:?}"),
        }
        match hunt_nearest(&mut w, 0, 0, 1, 5) {
            HuntResult::Kill { kind, x, y, .. } => {
                assert_eq!(kind, AnimalKind::Rabbit);
                assert_eq!((x, y), (0, 0));
            }
            o => panic!("{o:?}"),
        }
        assert!(w.animals.is_empty());
    }

    #[test]
    fn adjacent_range_misses_far_animal() {
        let mut w = AnimalWorld::default();
        w.spawn(AnimalKind::Wolf, 5, 5);
        assert_eq!(
            hunt_nearest(&mut w, 0, 0, HUNT_RANGE, HUNT_DAMAGE),
            HuntResult::Miss
        );
        // Adjacent wolf
        w.animals[0].x = 1;
        w.animals[0].y = 0;
        match hunt_nearest(&mut w, 0, 0, HUNT_RANGE, HUNT_DAMAGE) {
            HuntResult::Hit { kind, .. } | HuntResult::Kill { kind, .. } => {
                assert_eq!(kind, AnimalKind::Wolf);
            }
            o => panic!("{o:?}"),
        }
    }

    // COMBAT-MOSQUITO-KIND
    #[test]
    fn jungle_love_and_moskito_factor_for_brown() {
        // PersonColor.Brown = 3 loves JUNGLE → 1.0
        let love = jungle_biome_love_for_mosquito(0, 3, None, None);
        assert!((love - 1.0).abs() < 1e-6);
        // black (1) hates jungle → −0.5
        let hate = jungle_biome_love_for_mosquito(0, 1, None, None);
        assert!((hate - (-0.5)).abs() < 1e-6);
        // brown love → moskito factor 0.5 at hf=1
        let f = moskito_damage_factor_from_love(0, 3, None, None, 0.0, 1.0);
        assert!((f - 0.5).abs() < 1e-5);
        // scale path damage
        assert!((scale_damage_by_moskito_factor(2.0, 0.5) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn mosquito_kind_is_hunt_target() {
        let mut w = AnimalWorld::default();
        w.spawn(AnimalKind::Mosquito, 0, 0);
        match hunt_nearest(&mut w, 0, 0, 1, 1) {
            HuntResult::Hit { kind, hp_left, .. } => {
                assert_eq!(kind, AnimalKind::Mosquito);
                assert_eq!(hp_left, 2); // default_hp 3 − 1
            }
            o => panic!("{o:?}"),
        }
    }
}
