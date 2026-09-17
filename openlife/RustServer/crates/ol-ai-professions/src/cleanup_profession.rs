//! General AI `cleanUp` / `pileUp` pure helpers (Haxe `AiBase.cleanUp` / `pileUp`).
//!
//! Wet-nozzle slice already lives in [`crate::pottery_profession::wet_nozzle_cleanup_action`].
//! This module covers pile-up (stone/straw/dried corn) and the ordered cleanUp gate.

use crate::pottery_profession::{wet_nozzle_cleanup_action, PotteryAction, CLAY_WITH_NOZZLE};
use crate::smith_profession::WET_CLAY_NOZZLE;

/// Stone (Haxe pileUp(33, 20)).
pub const STONE: i32 = 33;
/// Stone Pile (content / Haxe getPileObjId for stone).
pub const STONE_PILE: i32 = 661;
/// Straw (Haxe pileUp(227, 30) — loose straw, not threshed 297).
pub const STRAW_LOOSE: i32 = 227;
/// Straw pile (common OHOL pile companion; 0 = unknown → pickup-only path).
pub const STRAW_PILE: i32 = 0;
/// Dried Ear of Corn (Haxe pileUp(1115, 30)).
pub const DRIED_EAR_OF_CORN: i32 = 1115;
/// Pile of Dried Corn.
pub const PILE_DRIED_CORN: i32 = 4107;
/// Long Straight Shaft (cleanUp hatchet when count > 5).
pub const LONG_STRAIGHT_SHAFT: i32 = 67;
/// Stone Hatchet.
pub const STONE_HATCHET: i32 = 71;
/// Weak Skewer.
pub const WEAK_SKEWER: i32 = 852;
/// Weak Skewer Pile.
pub const WEAK_SKEWER_PILE: i32 = 4060;
/// Cut Stones (Haxe BCut Stones).
// Haxe: AiBase.placeFloorUnder L1069
pub const CUT_STONES: i32 = 881;
/// Boards.
// Haxe: AiBase.placeFloorUnder L1071
pub const BOARDS: i32 = 470;
/// Pine Needles.
// Haxe: AiBase.placeFloorUnder L1073
pub const PINE_NEEDLES: i32 = 96;
/// `shortCraft(96, parentId, 40)` search distance.
// Haxe: AiBase.placeFloorUnder L1074
pub const PLACE_FLOOR_UNDER_PINE_DISTANCE: i32 = 40;
/// `doCriticalStuff` `if (cleanUp()) return true`.
// Haxe: AiBase.doCriticalStuff L6101
pub const CRITICAL_CLEANUP_RUNG: &str = "CRITICAL_CLEANUP";
/// Basket of Charcoal.
pub const BASKET_OF_CHARCOAL: i32 = 298;
/// Stack of Flat Rocks.
pub const STACK_OF_FLAT_ROCKS: i32 = 1836;
/// Flat Rock.
pub const FLAT_ROCK: i32 = 291;
/// Small Lump of Clay.
pub const SMALL_LUMP_OF_CLAY: i32 = 3891;
/// Gooseberry (Haxe `cleanUpBowls(253)` remaps bowlId → 31).
pub const GOOSEBERRY: i32 = 31;
/// Bowl of Gooseberries.
pub const BOWL_GOOSEBERRIES: i32 = 253;
/// Clay Bowl.
pub const CLAY_BOWL: i32 = 235;
/// Dry Bean Pod (held filler for Bowl of Dry Beans).
pub const DRY_BEAN_POD: i32 = 1160;
/// Bowl of Dry Beans.
pub const BOWL_DRY_BEANS: i32 = 1176;
/// Home-radius for `cleanUpBowls` CountCloseObjects.
// Haxe: AiBase.cleanUpBowls CountCloseObjects r=30
pub const CLEANUP_BOWL_RADIUS: i32 = 30;

/// Known (obj_id, pile_id, search_dist) for Haxe `pileUp` call sites in `cleanUp`.
// Haxe: AiBase.cleanUp ~1027–1029
pub const CLEANUP_PILE_TARGETS: &[(i32, i32, i32)] = &[
    (STONE, STONE_PILE, 20),
    (STRAW_LOOSE, STRAW_PILE, 30),
    (DRIED_EAR_OF_CORN, PILE_DRIED_CORN, 30),
];

/// Pure `pileUp(objId, dist)` decision.
///
/// - `pile_id < 1` → None (cannot pile)
/// - `held_id == use_actor_parent_id` → None (Haxe: held is last transition actor)
/// - count (near + held-as-obj) < 2 → None
/// - holding obj → ShortCraft onto pile if present, else onto another obj
/// - empty / other held → CraftItem(obj) as PickupItem staging
// Haxe: AiBase.pileUp ~943–971
pub fn pile_up_action(
    held_id: i32,
    obj_id: i32,
    pile_id: i32,
    count_near: i32,
    use_actor_parent_id: i32,
) -> Option<PotteryAction> {
    if pile_id < 1 {
        return None;
    }
    if held_id == use_actor_parent_id && use_actor_parent_id != 0 {
        return None;
    }
    let mut count = count_near;
    if held_id == obj_id {
        count += 1;
    }
    if count < 2 {
        return None;
    }
    if held_id == obj_id {
        // Prefer incomplete pile, else another loose obj (Haxe GetClosestObjectToTarget)
        Some(PotteryAction::ShortCraft {
            actor: obj_id,
            target: pile_id,
        })
    } else {
        // PickupItem(objId)
        Some(PotteryAction::CraftItem { object_id: obj_id })
    }
}

/// Counts snapshot for general cleanUp (caller fills from scan).
#[derive(Debug, Clone, Default)]
pub struct CleanupCounts {
    pub held_id: i32,
    /// Last transition actor parent (Haxe `useActor.parentId`); 0 if none.
    pub use_actor_parent_id: i32,
    /// Age for `% 3` pile gate (Haxe `myPlayer.age % 3 != 0` → skip pileUp).
    pub age: f32,
    pub count_basket_charcoal: i32,
    pub count_stack_flat_rocks: i32,
    pub count_flat_rock: i32,
    pub count_long_shaft: i32,
    pub count_weak_skewer: i32,
    /// Same-tick: Haxe `shortCraft(71, 852)` returned false.
    // Haxe: AiBase.cleanUp L1018
    pub weak_skewer_hatchet_miss: bool,
    pub count_wet_nozzle: i32,
    pub has_clay_with_nozzle: bool,
    pub count_small_clay: i32,
    pub count_stone: i32,
    pub count_straw: i32,
    pub count_dried_corn: i32,
    pub has_stone_pile: bool,
    pub has_straw_pile: bool,
    pub has_dried_corn_pile: bool,
    /// Gooseberry 31 snap (Haxe `cleanUpBowls(253)` remaps to 31).
    pub bowls_gooseberry: CleanupBowlSnap,
    /// Bowl of Dry Beans 1176 snap.
    pub bowls_dry_beans: CleanupBowlSnap,
}

/// Per-bowl-id snapshot for Haxe `cleanUpBowls`.
// Haxe: AiBase.cleanUpBowls ~4191
#[derive(Debug, Clone, Default)]
pub struct CleanupBowlSnap {
    pub held_id: i32,
    /// Count of search id near home (31 gooseberry / 1176 dry-bean bowl).
    pub count_search: i32,
    pub has_closest: bool,
    pub closest_uses: i32,
    /// `objectData.numUses` (0 = unknown / treat as not incomplete).
    pub closest_num_uses: i32,
    /// Second closest search-id is incomplete (Haxe GetClosestObjectById skip first).
    pub second_incomplete: bool,
    pub count_clay_bowl: i32,
    pub has_clay_bowl: bool,
}

/// One `placeFloorUnder` shortCraft attempt.
// Haxe: AiBase.placeFloorUnder L1069–1074
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceFloorUnderStep {
    pub actor: i32,
    pub target: i32,
    /// Haxe `shortCraft` 3rd arg: `false` → craftActorIfNeeded=false (default dist 20); 40 for 96.
    pub max_distance: i32,
    pub craft_actor: bool,
}

impl PlaceFloorUnderStep {
    pub fn to_pottery_action(self) -> PotteryAction {
        PotteryAction::ShortCraft {
            actor: self.actor,
            target: self.target,
        }
    }
}

/// Haxe `placeFloorUnder(obj)`: null / !allowFloorPlacement / floor>0 → None; else 881 then 470 then 96@40.
// Haxe: AiBase.placeFloorUnder L1058–1076
pub fn place_floor_under(
    obj_is_some: bool,
    allow_floor_placement: bool,
    floor_id: i32,
    parent_id: i32,
) -> Option<[PlaceFloorUnderStep; 3]> {
    // Haxe: AiBase L1059 if (obj == null) return false
    if !obj_is_some {
        return None;
    }
    // Haxe: AiBase L1065 if (objData.allowFloorPlacement == false) return false
    if !allow_floor_placement {
        return None;
    }
    // Haxe: AiBase L1066–1067 floor = getFloorId; if (floor > 0) return false
    if floor_id > 0 {
        return None;
    }
    Some([
        // Haxe: AiBase L1070 shortCraft(881, parentId, false)
        PlaceFloorUnderStep {
            actor: CUT_STONES,
            target: parent_id,
            max_distance: 20,
            craft_actor: false,
        },
        // Haxe: AiBase L1072 shortCraft(470, parentId, false)
        PlaceFloorUnderStep {
            actor: BOARDS,
            target: parent_id,
            max_distance: 20,
            craft_actor: false,
        },
        // Haxe: AiBase L1074 shortCraft(96, parentId, 40)
        PlaceFloorUnderStep {
            actor: PINE_NEEDLES,
            target: parent_id,
            max_distance: PLACE_FLOOR_UNDER_PINE_DISTANCE,
            craft_actor: true,
        },
    ])
}

/// First `placeFloorUnder` shortCraft (Cut Stones 881 on `parent_id`).
// Haxe: AiBase.placeFloorUnder L1070
pub fn place_floor_under_action(
    obj_is_some: bool,
    allow_floor_placement: bool,
    floor_id: i32,
    parent_id: i32,
) -> Option<PotteryAction> {
    place_floor_under(obj_is_some, allow_floor_placement, floor_id, parent_id)
        .map(|steps| steps[0].to_pottery_action())
}

/// Next `placeFloorUnder` shortCraft after a failed actor (881 → 470 → 96@40).
// Haxe: AiBase.placeFloorUnder L1070–1074
pub fn place_floor_under_next(failed_actor: i32, parent_id: i32) -> Option<PlaceFloorUnderStep> {
    match failed_actor {
        CUT_STONES => Some(PlaceFloorUnderStep {
            actor: BOARDS,
            target: parent_id,
            max_distance: 20,
            craft_actor: false,
        }),
        BOARDS => Some(PlaceFloorUnderStep {
            actor: PINE_NEEDLES,
            target: parent_id,
            max_distance: PLACE_FLOOR_UNDER_PINE_DISTANCE,
            craft_actor: true,
        }),
        _ => None,
    }
}

/// Haxe `cleanUp()` pure sequence (first hit wins).
///
/// Age `% 3 != 0` returns false for pileUp + nozzle + 2110 + clay + bowls
/// (earlier charcoal/rocks/shaft/skewer shortCrafts still run).
// Haxe: AiBase.cleanUp ~974–1055
pub fn clean_up_action(c: &CleanupCounts) -> Option<PotteryAction> {
    // Basket of Charcoal 298 — empty basket
    if c.count_basket_charcoal > 0 {
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: BASKET_OF_CHARCOAL,
        });
    }
    // Stack of Flat Rocks near forge/home
    if c.count_stack_flat_rocks > 0 {
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: STACK_OF_FLAT_ROCKS,
        });
    }
    // Excess Flat Rock (>3) → pickup (CraftItem staging)
    if c.count_flat_rock > 3 {
        return Some(PotteryAction::CraftItem {
            object_id: FLAT_ROCK,
        });
    }
    // Long Straight Shaft count > 5 → hatchet + shaft
    if c.count_long_shaft > 5 {
        return Some(PotteryAction::ShortCraft {
            actor: STONE_HATCHET,
            target: LONG_STRAIGHT_SHAFT,
        });
    }
    // Weak Skewer count > 5: hatchet first, then dropHeldObject(0), then pile 4060.
    // Haxe: AiBase.cleanUp L1014–1022
    if c.count_weak_skewer > 5 {
        if !c.weak_skewer_hatchet_miss {
            return Some(PotteryAction::ShortCraft {
                actor: STONE_HATCHET,
                target: WEAK_SKEWER,
            });
        }
        if c.held_id == WEAK_SKEWER {
            // Haxe: AiBase L1019 return dropHeldObject(0)
            return Some(PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 0,
            });
        }
        // Haxe: AiBase L1022 shortCraft(0, 4060, 10)
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: WEAK_SKEWER_PILE,
        });
    }

    // Haxe: AiBase.cleanUp L1025 if (age % 3 != 0) return false
    let age_i = c.age as i32;
    if age_i % 3 != 0 {
        return None;
    }

    for &(obj, pile, _dist) in CLEANUP_PILE_TARGETS {
        let (count, has_pile) = match obj {
            STONE => (c.count_stone, c.has_stone_pile),
            STRAW_LOOSE => (c.count_straw, c.has_straw_pile),
            DRIED_EAR_OF_CORN => (c.count_dried_corn, c.has_dried_corn_pile),
            _ => (0, false),
        };
        let pile_id = if has_pile || pile > 0 { pile } else { 0 };
        // Straw pile id unknown (0): still allow pickup when count>=2 and not holding
        if pile_id < 1 {
            let mut n = count;
            if c.held_id == obj {
                n += 1;
            }
            if n >= 2 && c.held_id != obj {
                return Some(PotteryAction::CraftItem { object_id: obj });
            }
            continue;
        }
        if let Some(a) = pile_up_action(
            c.held_id,
            obj,
            pile_id,
            count,
            c.use_actor_parent_id,
        ) {
            return Some(a);
        }
    }

    if let Some(a) = wet_nozzle_cleanup_action(
        c.count_wet_nozzle,
        c.held_id,
        c.has_clay_with_nozzle,
    ) {
        return Some(a);
    }

    // Small Lump of Clay ≥2 → shortCraft merge
    if c.count_small_clay > 1 {
        return Some(PotteryAction::ShortCraft {
            actor: SMALL_LUMP_OF_CLAY,
            target: SMALL_LUMP_OF_CLAY,
        });
    }

    // Haxe: cleanUpBowls(253) then cleanUpBowls(1176)
    if let Some(a) = clean_up_bowls_action(BOWL_GOOSEBERRIES, &c.bowls_gooseberry) {
        return Some(a);
    }
    if let Some(a) = clean_up_bowls_action(BOWL_DRY_BEANS, &c.bowls_dry_beans) {
        return Some(a);
    }

    None
}

/// Haxe `cleanUpBowls(bowlId)` — gooseberry 253 remaps to berry 31; dry beans 1176 fill from pod 1160.
// Haxe: AiBase.cleanUpBowls ~4191–4219
pub fn clean_up_bowls_action(bowl_id: i32, s: &CleanupBowlSnap) -> Option<PotteryAction> {
    let filled_with_id = if bowl_id == BOWL_DRY_BEANS {
        DRY_BEAN_POD
    } else {
        -1
    };
    let search_id = if bowl_id == BOWL_GOOSEBERRIES {
        GOOSEBERRY
    } else {
        bowl_id
    };

    let incomplete = s.has_closest
        && s.closest_num_uses > 0
        && s.closest_uses < s.closest_num_uses;

    // Holding filler (dry bean pod): fill incomplete bowl, else clay bowl if few bowls.
    if s.held_id == filled_with_id && filled_with_id > 0 {
        if incomplete {
            return Some(PotteryAction::ShortCraft {
                actor: filled_with_id,
                target: search_id,
            });
        }
        if s.count_search > 1 && s.second_incomplete {
            return Some(PotteryAction::ShortCraft {
                actor: filled_with_id,
                target: search_id,
            });
        }
        if s.count_search < 3 && s.has_clay_bowl {
            return Some(PotteryAction::ShortCraft {
                actor: filled_with_id,
                target: CLAY_BOWL,
            });
        }
    }

    // Pickup filler from ground when extra clay bowls or incomplete close bowl.
    // Gooseberry path: filledWithID = -1 → Haxe shortCraft(0, -1) no-ops; skip.
    if filled_with_id > 0 && (s.count_clay_bowl > 1 || incomplete) {
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: filled_with_id,
        });
    }

    if s.count_search < 2 {
        return None;
    }
    // Empty only bowls / berries with one use (Haxe numberOfUses > 1 skip).
    if s.has_closest && s.closest_uses > 1 {
        return None;
    }
    if s.has_closest {
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: search_id,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pile_up_needs_two_and_pile_id() {
        assert!(pile_up_action(0, STONE, STONE_PILE, 1, 0).is_none());
        assert_eq!(
            pile_up_action(0, STONE, STONE_PILE, 2, 0),
            Some(PotteryAction::CraftItem { object_id: STONE })
        );
        assert_eq!(
            pile_up_action(STONE, STONE, STONE_PILE, 1, 0),
            Some(PotteryAction::ShortCraft {
                actor: STONE,
                target: STONE_PILE
            })
        );
        assert!(pile_up_action(STONE, STONE, 0, 5, 0).is_none());
    }

    #[test]
    fn clean_up_age_gate_skips_pile_but_keeps_shaft() {
        let mut c = CleanupCounts {
            age: 1.0, // 1 % 3 != 0 → skip pile
            count_stone: 5,
            has_stone_pile: true,
            count_long_shaft: 6,
            ..Default::default()
        };
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: STONE_HATCHET,
                target: LONG_STRAIGHT_SHAFT
            })
        );
        c.count_long_shaft = 0;
        c.age = 3.0; // pile allowed
        // Empty hands → PickupItem(stone) staging (Haxe pileUp)
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::CraftItem { object_id: STONE })
        );
        c.held_id = STONE;
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: STONE,
                target: STONE_PILE
            })
        );
    }

    #[test]
    fn clean_up_wet_nozzle_after_piles() {
        // Haxe: AiBase L1025 age%3!=0 skips nozzle/clay/bowls; L1031–1037 when age%3==0
        let mut c = CleanupCounts {
            age: 1.0,
            count_wet_nozzle: 2,
            count_small_clay: 3,
            bowls_gooseberry: CleanupBowlSnap {
                count_search: 2,
                has_closest: true,
                closest_uses: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(clean_up_action(&c), None);
        c.age = 3.0;
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: WET_CLAY_NOZZLE,
                target: WET_CLAY_NOZZLE
            })
        );
        let _ = CLAY_WITH_NOZZLE;
    }

    #[test]
    fn clean_up_weak_skewer_hatchet_then_drop_then_pile() {
        // Haxe: AiBase L1014–1022
        let mut c = CleanupCounts {
            count_weak_skewer: 6,
            held_id: WEAK_SKEWER,
            ..Default::default()
        };
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: STONE_HATCHET,
                target: WEAK_SKEWER
            })
        );
        c.weak_skewer_hatchet_miss = true;
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 0
            })
        );
        c.held_id = 0;
        c.weak_skewer_hatchet_miss = false;
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: STONE_HATCHET,
                target: WEAK_SKEWER
            })
        );
        c.weak_skewer_hatchet_miss = true;
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: WEAK_SKEWER_PILE
            })
        );
    }

    #[test]
    fn place_floor_under_gates_then_881_470_96_at_40() {
        // Haxe: AiBase.placeFloorUnder L1058–1076
        assert!(place_floor_under(false, true, 0, 237).is_none());
        assert!(place_floor_under(true, false, 0, 237).is_none());
        assert!(place_floor_under(true, true, 1, 237).is_none());
        let steps = place_floor_under(true, true, 0, 237).unwrap();
        assert_eq!(steps[0].actor, CUT_STONES);
        assert_eq!(steps[0].target, 237);
        assert!(!steps[0].craft_actor);
        assert_eq!(steps[1].actor, BOARDS);
        assert_eq!(steps[1].target, 237);
        assert!(!steps[1].craft_actor);
        assert_eq!(steps[2].actor, PINE_NEEDLES);
        assert_eq!(steps[2].target, 237);
        assert_eq!(steps[2].max_distance, PLACE_FLOOR_UNDER_PINE_DISTANCE);
        assert_eq!(
            place_floor_under_action(true, true, 0, 237),
            Some(PotteryAction::ShortCraft {
                actor: CUT_STONES,
                target: 237
            })
        );
        let boards = place_floor_under_next(CUT_STONES, 237).unwrap();
        assert_eq!(boards.actor, BOARDS);
        let pine = place_floor_under_next(BOARDS, 237).unwrap();
        assert_eq!(pine.actor, PINE_NEEDLES);
        assert_eq!(pine.max_distance, 40);
        assert!(place_floor_under_next(PINE_NEEDLES, 237).is_none());
    }

    #[test]
    fn clean_up_bowls_gooseberry_empties_two_single_use() {
        let s = CleanupBowlSnap {
            count_search: 2,
            has_closest: true,
            closest_uses: 1,
            closest_num_uses: 1,
            ..Default::default()
        };
        assert_eq!(
            clean_up_bowls_action(BOWL_GOOSEBERRIES, &s),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: GOOSEBERRY
            })
        );
        let one = CleanupBowlSnap {
            count_search: 1,
            has_closest: true,
            closest_uses: 1,
            ..Default::default()
        };
        assert!(clean_up_bowls_action(BOWL_GOOSEBERRIES, &one).is_none());
        let multi = CleanupBowlSnap {
            count_search: 3,
            has_closest: true,
            closest_uses: 2,
            closest_num_uses: 3,
            ..Default::default()
        };
        assert!(clean_up_bowls_action(BOWL_GOOSEBERRIES, &multi).is_none());
        assert_eq!(CLEANUP_BOWL_RADIUS, 30);
        // Haxe L4211–4212: gooseberry filledWithID=-1 → shortCraft(0,-1) skipped
        let extra_clay_berry = CleanupBowlSnap {
            count_search: 1,
            has_closest: true,
            closest_uses: 1,
            closest_num_uses: 3,
            count_clay_bowl: 2,
            has_clay_bowl: true,
            ..Default::default()
        };
        assert!(
            clean_up_bowls_action(BOWL_GOOSEBERRIES, &extra_clay_berry).is_none(),
            "gooseberry must not shortCraft(0, -1) when extra clay bowls"
        );
    }

    #[test]
    fn clean_up_bowls_dry_beans_fill_then_empty() {
        let fill = CleanupBowlSnap {
            held_id: DRY_BEAN_POD,
            count_search: 1,
            has_closest: true,
            closest_uses: 1,
            closest_num_uses: 3,
            ..Default::default()
        };
        assert_eq!(
            clean_up_bowls_action(BOWL_DRY_BEANS, &fill),
            Some(PotteryAction::ShortCraft {
                actor: DRY_BEAN_POD,
                target: BOWL_DRY_BEANS
            })
        );
        let extra_clay = CleanupBowlSnap {
            count_search: 1,
            has_closest: true,
            closest_uses: 1,
            closest_num_uses: 1,
            count_clay_bowl: 2,
            has_clay_bowl: true,
            ..Default::default()
        };
        assert_eq!(
            clean_up_bowls_action(BOWL_DRY_BEANS, &extra_clay),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: DRY_BEAN_POD
            })
        );
        let empty = CleanupBowlSnap {
            count_search: 2,
            has_closest: true,
            closest_uses: 1,
            closest_num_uses: 1,
            ..Default::default()
        };
        assert_eq!(
            clean_up_bowls_action(BOWL_DRY_BEANS, &empty),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: BOWL_DRY_BEANS
            })
        );
    }

    #[test]
    fn clean_up_action_bowls_after_clay_merge() {
        let c = CleanupCounts {
            age: 3.0,
            bowls_gooseberry: CleanupBowlSnap {
                count_search: 2,
                has_closest: true,
                closest_uses: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            clean_up_action(&c),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: GOOSEBERRY
            })
        );
    }
}
