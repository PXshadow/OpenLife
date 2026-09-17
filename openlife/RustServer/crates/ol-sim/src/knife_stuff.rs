//! Haxe `AiBase.doKnifeStuff` (~876) — mid held-knife opportunist.
//!
//! Not assigned TAILOR/BAKER. Baker/smith have other knife uses; this is the
//! mid-ladder shortCraft after `isHandlingFire()` (and hungry `isConsideringMakingFood`).
//!
//! Pure: callers supply held id + nearby `(parent_id, x, y)` tiles.

use crate::farmer_profession::{short_craft_apply, ShortCraftApply, ShortCraftInput};

/// Knife 560.
// Haxe: AiBase.doKnifeStuff ~880
pub const KNIFE: i32 = 560;
/// Baked Bread 1470.
// Haxe: AiBase.doKnifeStuff ~882
pub const BAKED_BREAD: i32 = 1470;
/// Leavened Dough on Clay Plate 1468.
// Haxe: AiBase.doKnifeStuff ~884
pub const LEAVENED_DOUGH_PLATE: i32 = 1468;
/// Dead Wolf 422.
// Haxe: AiBase.doKnifeStuff ~886
pub const DEAD_WOLF: i32 = 422;
/// Dead Grizzly Bear 643.
// Haxe: AiBase.doKnifeStuff ~888
pub const DEAD_GRIZZLY_BEAR: i32 = 643;

/// Haxe `shortCraft(..., dist=20)`.
// Haxe: AiBase.doKnifeStuff ~877
pub const KNIFE_STUFF_DIST: i32 = 20;

/// Mid / hungry ladder rung label (not a profession job).
pub const KNIFE_STUFF_RUNG: &str = "KNIFE_STUFF";

/// Haxe target order: bread, dough plate, wolf, bear.
// Haxe: AiBase.doKnifeStuff ~882–889
pub const KNIFE_STUFF_TARGETS: [i32; 4] = [
    BAKED_BREAD,
    LEAVENED_DOUGH_PLATE,
    DEAD_WOLF,
    DEAD_GRIZZLY_BEAR,
];

/// Nearby scan tile for [`do_knife_stuff`] (`parent_id`, `x`, `y`).
pub type KnifeStuffTile = (i32, i32, i32);

/// Pure `doKnifeStuff` next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnifeStuffAction {
    None,
    /// Held Knife 560 → `useHeldObjOnTarget`.
    UseOnTarget { x: i32, y: i32, target_id: i32 },
}

impl KnifeStuffAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Haxe `shortCraft(560, target, 20, false)` apply (craftActorIfNeeded=false).
// Haxe: AiBase.doKnifeStuff ~883–889; shortCraftOnTarget ~2775
pub fn knife_stuff_short_craft_apply(
    held_id: i32,
    target_id: i32,
    food_store: f32,
    transition_hungry_cost: f32,
) -> ShortCraftApply {
    short_craft_apply(ShortCraftInput {
        held_id,
        actor_id: KNIFE,
        target_id,
        target_uses: 1,
        target_biome: None,
        has_carrot_seeds: true,
        new_actor_count: 0,
        max_new_actor: -1,
        try_weak_skewer_first: false,
        craft_actor_if_needed: false,
        food_store,
        transition_hungry_cost,
    })
}

fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Closest tile with `parent_id` within Chebyshev `max_r` (tie: lower y, then x).
fn closest_target(
    tiles: &[KnifeStuffTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, i32)> = None;
    for &(id, x, y) in tiles {
        if id != parent_id {
            continue;
        }
        let d = chebyshev(from_x, from_y, x, y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, y, x)),
            Some((bd, by, bx)) => {
                if d < bd || (d == bd && (y < by || (y == by && x < bx))) {
                    best = Some((d, y, x));
                }
            }
        }
    }
    best.map(|(_, y, x)| (x, y))
}

/// Haxe `doKnifeStuff`: held Knife 560, else false. First shortCraft in order.
// Haxe: AiBase.doKnifeStuff ~876–891
pub fn do_knife_stuff(held_id: i32, tiles: &[KnifeStuffTile], px: i32, py: i32) -> KnifeStuffAction {
    do_knife_stuff_ex(held_id, tiles, px, py, 20.0, 0.0)
}

/// Same as [`do_knife_stuff`] with hungry work cost (Haxe shortCraftOnTarget gate).
pub fn do_knife_stuff_ex(
    held_id: i32,
    tiles: &[KnifeStuffTile],
    px: i32,
    py: i32,
    food_store: f32,
    transition_hungry_cost: f32,
) -> KnifeStuffAction {
    if held_id != KNIFE {
        return KnifeStuffAction::None;
    }
    do_knife_stuff_with_costs(held_id, tiles, px, py, food_store, |_| transition_hungry_cost)
}

/// Same as [`do_knife_stuff_ex`] with a per-target hungry-work cost.
///
/// Haxe `shortCraft` is a separate call per target: a non-USE result (hungry-cost
/// fail, refuse, seek) is `false` and the next id is tried.
// Haxe: AiBase.doKnifeStuff L882–889
pub fn do_knife_stuff_with_costs(
    held_id: i32,
    tiles: &[KnifeStuffTile],
    px: i32,
    py: i32,
    food_store: f32,
    cost_for_target: impl Fn(i32) -> f32,
) -> KnifeStuffAction {
    if held_id != KNIFE {
        return KnifeStuffAction::None;
    }
    for &target_id in &KNIFE_STUFF_TARGETS {
        let Some((x, y)) = closest_target(tiles, target_id, px, py, KNIFE_STUFF_DIST) else {
            continue;
        };
        match knife_stuff_short_craft_apply(
            held_id,
            target_id,
            food_store,
            cost_for_target(target_id),
        ) {
            ShortCraftApply::UseOnTarget { .. } => {
                return KnifeStuffAction::UseOnTarget { x, y, target_id };
            }
            // Haxe: shortCraft false → fall through to 1468 / 422 / 643
            _ => continue,
        }
    }
    KnifeStuffAction::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_not_knife_returns_none_even_with_bread() {
        let tiles = [(BAKED_BREAD, 2, 0)];
        assert_eq!(do_knife_stuff(0, &tiles, 0, 0), KnifeStuffAction::None);
        assert_eq!(do_knife_stuff(33, &tiles, 0, 0), KnifeStuffAction::None);
        assert!(!do_knife_stuff(KNIFE, &[], 0, 0).is_some());
    }

    #[test]
    fn held_knife_and_bread_in_r20_uses() {
        let tiles = [(BAKED_BREAD, 5, 0)];
        assert_eq!(
            do_knife_stuff(KNIFE, &tiles, 0, 0),
            KnifeStuffAction::UseOnTarget {
                x: 5,
                y: 0,
                target_id: BAKED_BREAD
            }
        );
        let far = [(BAKED_BREAD, 21, 0)];
        assert_eq!(do_knife_stuff(KNIFE, &far, 0, 0), KnifeStuffAction::None);
    }

    #[test]
    fn craft_actor_if_needed_false_does_not_enqueue_craft_knife() {
        let apply = knife_stuff_short_craft_apply(0, BAKED_BREAD, 20.0, 0.0);
        match apply {
            ShortCraftApply::SeekOrCraftActor {
                actor,
                craft_if_needed,
            } => {
                assert_eq!(actor, KNIFE);
                assert!(
                    !craft_if_needed,
                    "Haxe craftActorIfNeeded=false must not craft 560"
                );
            }
            other => panic!("expected SeekOrCraftActor craft=false, got {other:?}"),
        }
        assert!(!matches!(apply, ShortCraftApply::UseOnTarget { .. }));
        // Planner still refuses when not holding the knife (no seek/craft enqueue).
        let tiles = [(BAKED_BREAD, 1, 0)];
        assert_eq!(do_knife_stuff(0, &tiles, 0, 0), KnifeStuffAction::None);
    }

    #[test]
    fn bread_then_dough_then_wolf_then_bear() {
        let all = [
            (DEAD_GRIZZLY_BEAR, 1, 0),
            (DEAD_WOLF, 2, 0),
            (LEAVENED_DOUGH_PLATE, 3, 0),
            (BAKED_BREAD, 4, 0),
        ];
        assert_eq!(
            do_knife_stuff(KNIFE, &all, 0, 0).target_id(),
            Some(BAKED_BREAD)
        );
        let no_bread = [
            (DEAD_GRIZZLY_BEAR, 1, 0),
            (DEAD_WOLF, 2, 0),
            (LEAVENED_DOUGH_PLATE, 3, 0),
        ];
        assert_eq!(
            do_knife_stuff(KNIFE, &no_bread, 0, 0).target_id(),
            Some(LEAVENED_DOUGH_PLATE)
        );
        let animals = [(DEAD_GRIZZLY_BEAR, 1, 0), (DEAD_WOLF, 2, 0)];
        assert_eq!(
            do_knife_stuff(KNIFE, &animals, 0, 0).target_id(),
            Some(DEAD_WOLF)
        );
        let bear = [(DEAD_GRIZZLY_BEAR, 8, 1)];
        assert_eq!(
            do_knife_stuff(KNIFE, &bear, 0, 0).target_id(),
            Some(DEAD_GRIZZLY_BEAR)
        );
        assert_eq!(KNIFE_STUFF_RUNG, "KNIFE_STUFF");
        assert_eq!(KNIFE_STUFF_DIST, 20);
        assert_eq!(KNIFE_STUFF_TARGETS, [1470, 1468, 422, 643]);
    }

    #[test]
    fn bread_hungry_cost_fail_falls_through_to_dough() {
        // Haxe: shortCraft(560, 1470) false still tries 1468 then 422 then 643
        let tiles = [
            (BAKED_BREAD, 1, 0),
            (LEAVENED_DOUGH_PLATE, 2, 0),
            (DEAD_WOLF, 3, 0),
            (DEAD_GRIZZLY_BEAR, 4, 0),
        ];
        let a = do_knife_stuff_with_costs(KNIFE, &tiles, 0, 0, 5.0, |id| {
            if id == BAKED_BREAD {
                10.0
            } else {
                0.0
            }
        });
        assert_eq!(
            a,
            KnifeStuffAction::UseOnTarget {
                x: 2,
                y: 0,
                target_id: LEAVENED_DOUGH_PLATE
            }
        );
        let wolf = do_knife_stuff_with_costs(KNIFE, &tiles, 0, 0, 5.0, |id| {
            if id == BAKED_BREAD || id == LEAVENED_DOUGH_PLATE {
                10.0
            } else {
                0.0
            }
        });
        assert_eq!(
            wolf,
            KnifeStuffAction::UseOnTarget {
                x: 3,
                y: 0,
                target_id: DEAD_WOLF
            }
        );
        let bear = do_knife_stuff_with_costs(KNIFE, &tiles, 0, 0, 5.0, |id| {
            if id == DEAD_GRIZZLY_BEAR {
                0.0
            } else {
                10.0
            }
        });
        assert_eq!(
            bear,
            KnifeStuffAction::UseOnTarget {
                x: 4,
                y: 0,
                target_id: DEAD_GRIZZLY_BEAR
            }
        );
    }
}

impl KnifeStuffAction {
    fn target_id(self) -> Option<i32> {
        match self {
            Self::UseOnTarget { target_id, .. } => Some(target_id),
            Self::None => None,
        }
    }
}
