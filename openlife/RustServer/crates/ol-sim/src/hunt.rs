//! Hunt / animal damage pure helpers.

use crate::animals::{AnimalKind, AnimalWorld};

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
}
