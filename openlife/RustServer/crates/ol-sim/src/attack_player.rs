//! Haxe `AiBase.attackPlayer` / `getWeapon` (**AI-ATTACK-PLAYER**).
//!
//! Pure planner: callers supply target + held/clothing + nearby weapon tiles.
//! Live tick maps [`AttackPlayerAction`] to USE / DROP / SELF / MOVE / KILL.

use crate::weapons::is_bloody_weapon;
use ol_move_rules::{
    calculate_exact_quad_distance_f, KILL_DEADLY_RANGE_SLACK, RANGED_DEADLY_DISTANCE_THRESHOLD,
    RANGED_MIN_USE_DISTANCE,
};

/// Knife 560.
// Haxe: AiBase.getWeapon weapons 560
pub const KNIFE: i32 = 560;
/// War Sword 3047.
pub const WAR_SWORD: i32 = 3047;
/// Bow and Arrow 152.
pub const BOW_AND_ARROW: i32 = 152;
/// Arrow 148.
pub const ARROW: i32 = 148;
/// Yew Bow 151.
pub const YEW_BOW: i32 = 151;
/// Arrow Quiver 3948.
pub const ARROW_QUIVER: i32 = 3948;
/// Empty Arrow Quiver with Bow 4149.
pub const EMPTY_ARROW_QUIVER_WITH_BOW: i32 = 4149;
/// Arrow Quiver with Bow 4151.
pub const ARROW_QUIVER_WITH_BOW: i32 = 4151;
/// Clothing slot for quiver SELF (Haxe `self(0, 0, 5)`).
pub const QUIVER_CLOTHING_SLOT: i32 = 5;

/// Haxe `ServerSettings.MinAiAgeForCombat`.
// Haxe: ServerSettings.MinAiAgeForCombat = 8
pub const MIN_AI_AGE_FOR_COMBAT: f32 = 8.0;
/// `food_store < -2` refuse.
// Haxe: AiBase.attackPlayer ~5825
pub const ATTACK_FOOD_STORE_MIN: f32 = -2.0;
/// `GetClosestObjectToPositionByIds(..., 40)`.
// Haxe: AiBase.getWeapon ~5810
pub const WEAPON_SEARCH_DIST: i32 = 40;
/// Haxe `hasWeaponClose` GetClosestObjectToPosition r=20 for Bow 152 / Arrow 148.
// Haxe: AiBase.hasWeaponClose L5727 / L5734
pub const HAS_WEAPON_CLOSE_SEARCH: i32 = 20;
/// `isMovingToHome(4)` when held is bloody (`getWeapon` hunt path).
// Haxe: AiBase.getWeapon ~5752
pub const BLOODY_GO_HOME_TILES: i32 = 4;
/// `dropHeldObject(5)` when wearing quiver-with-bow and not holding bow.
// Haxe: AiBase.getWeapon ~5801
pub const GET_WEAPON_DROP_HOME: f32 = 5.0;

/// Nearby ground weapon tile (`parent_id`, `x`, `y`, `is_permanent`).
pub type WeaponTile = (i32, i32, i32, bool);

/// Combat target snapshot (Haxe `GlobalPlayerInstance` fields used by attackPlayer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackPlayerTarget {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub exact_x: f64,
    pub exact_y: f64,
    pub wounded: bool,
}

/// Clothing flags for `getWeapon` quiver branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttackPlayerClothing {
    /// Arrow Quiver 3948.
    pub arrow_quiver: bool,
    /// Arrow Quiver with Bow 4151.
    pub quiver_with_bow: bool,
    /// Empty Arrow Quiver with Bow 4149.
    pub empty_quiver_with_bow: bool,
    /// Haxe `quiver.canAddToQuiver()`.
    pub can_add_to_quiver: bool,
}

impl AttackPlayerClothing {
    pub fn from_ids(ids: &[i32]) -> Self {
        Self::from_ids_can_add(ids, true)
    }

    pub fn from_ids_can_add(ids: &[i32], can_add: bool) -> Self {
        let mut c = Self::default();
        for &id in ids {
            match id {
                ARROW_QUIVER => c.arrow_quiver = true,
                ARROW_QUIVER_WITH_BOW => c.quiver_with_bow = true,
                EMPTY_ARROW_QUIVER_WITH_BOW => c.empty_quiver_with_bow = true,
                _ => {}
            }
        }
        c.can_add_to_quiver = can_add
            && (c.arrow_quiver || c.quiver_with_bow || c.empty_quiver_with_bow);
        c
    }
}

/// Inputs for [`attack_player`] / [`get_weapon`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackPlayerInput<'a> {
    pub target: Option<AttackPlayerTarget>,
    pub food_store: f32,
    pub self_wounded: bool,
    pub age: f32,
    pub min_ai_age_for_combat: f32,
    pub holding_weapon: bool,
    pub held_id: i32,
    /// 0 → treat as [`Self::held_id`].
    pub held_parent_id: i32,
    pub is_moving: bool,
    pub player_x: i32,
    pub player_y: i32,
    pub exact_x: f64,
    pub exact_y: f64,
    pub home_x: i32,
    pub home_y: i32,
    pub deadly_distance: f32,
    pub clothing: AttackPlayerClothing,
    pub weapon_tiles: &'a [WeaponTile],
}

impl<'a> AttackPlayerInput<'a> {
    fn held_parent(self) -> i32 {
        if self.held_parent_id > 0 {
            self.held_parent_id
        } else {
            self.held_id
        }
    }
}

/// Haxe `getWeapon` next step (`true` → consume tick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetWeaponAction {
    None,
    /// Bloody held + far from home — `isMovingToHome(4)`.
    GoHome { x: i32, y: i32 },
    /// Consume tick without a wire action (bloody near home / GetOrCraft while moving).
    Wait,
    /// `myPlayer.self(0, 0, 5)`.
    SelfClothing { slot: i32 },
    /// `dropHeldObject(5)`.
    DropHeld,
    /// `PickupObj` closest weapon.
    Pickup { x: i32, y: i32, id: i32 },
    /// `GetOrCraftItem(148|152)`.
    SeekOrCraft { actor: i32 },
}

impl GetWeaponAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Haxe `attackPlayer` next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackPlayerAction {
    None,
    GetWeapon(GetWeaponAction),
    /// Stand-off `gotoObj`.
    Goto { x: i32, y: i32 },
    /// `myPlayer.kill(tx-gx, ty-gy, target.id)`.
    Kill {
        target_p_id: i32,
        tx: i32,
        ty: i32,
    },
}

impl AttackPlayerAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Patched `ObjectData.deadlyDistance` when content is missing.
// Haxe: ServerSettings.PatchObjectData weapon deadlyDistance
pub fn deadly_distance_for_held(held_id: i32, from_content: f32) -> f32 {
    if from_content.is_finite() && from_content > 0.0 {
        return from_content;
    }
    match held_id {
        BOW_AND_ARROW | 1624 | 749 => 4.0,
        KNIFE | WAR_SWORD | 750 | 3048 => 1.5,
        _ => 0.0,
    }
}

/// Stand-off tile (Haxe `Math.floor(range)` then ± range∓1 per axis).
// Haxe: AiBase.attackPlayer ~5856–5859
pub fn stand_off_tile(
    player_x: i32,
    player_y: i32,
    target_x: i32,
    target_y: i32,
    deadly_distance: f32,
) -> (i32, i32) {
    let range = if deadly_distance.is_finite() && deadly_distance > 0.0 {
        deadly_distance.floor() as i32
    } else {
        0
    };
    let tx = if target_x > player_x {
        target_x - range + 1
    } else {
        target_x + range - 1
    };
    let ty = if target_y > player_y {
        target_y - range + 1
    } else {
        target_y + range - 1
    };
    (tx, ty)
}

/// True when too far or bow too close (Haxe exact-quad vs deadlyDistance).
// Haxe: AiBase.attackPlayer ~5853
pub fn attack_needs_stand_off(exact_quad: f64, deadly_distance: f32) -> bool {
    let range = if deadly_distance.is_finite() {
        deadly_distance as f64
    } else {
        0.0
    };
    let slack = KILL_DEADLY_RANGE_SLACK as f64;
    exact_quad > range * range + slack
        || (range > RANGED_DEADLY_DISTANCE_THRESHOLD as f64
            && exact_quad < RANGED_MIN_USE_DISTANCE as f64)
}

fn quad_i(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn should_path_to_home(quad_dist: f32, max_tiles: i32) -> bool {
    let mt = max_tiles.max(0);
    quad_dist >= (mt * mt) as f32
}

/// Closest weapon in the Haxe half-open square, ranked by integer quad.
/// Includes permanent tiles so [`PickupObj`] can refuse them and fall through
/// to GetOrCraft (Haxe does not skip permanent in the search).
// Haxe: AiHelper.GetClosestObjectToPositionByIds L355–388; PickupObj L6138–6143
fn closest_weapon(
    tiles: &[WeaponTile],
    ids: &[i32],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<(i32, i32, i32, bool)> {
    let mut best: Option<(i32, i32, i32, i32, bool)> = None; // quad, y, x, id, permanent
    for &(id, x, y, permanent) in tiles {
        if !ids.contains(&id) {
            continue;
        }
        if !in_has_weapon_search(from_x, from_y, x, y, max_r) {
            continue;
        }
        let q = quad_i(from_x, from_y, x, y);
        match best {
            None => best = Some((q, y, x, id, permanent)),
            Some((bq, by, bx, _, _)) => {
                // Haxe: `if quadDistance > bestDistance continue` — equal replaces
                // (scan ty then tx ascending → last win = higher y, then higher x).
                if q < bq || (q == bq && (y > by || (y == by && x > bx))) {
                    best = Some((q, y, x, id, permanent));
                }
            }
        }
    }
    best.map(|(_, y, x, id, permanent)| (id, x, y, permanent))
}

/// Haxe `GetClosestObjectToPosition` half-open square `[c-r, c+r)`.
// Haxe: AiHelper.GetClosestObjectToPositionHelper `base±searchDistance` exclusive end
fn in_has_weapon_search(px: i32, py: i32, x: i32, y: i32, r: i32) -> bool {
    x >= px - r && x < px + r && y >= py - r && y < py + r
}

/// Haxe `AiBase.hasWeaponClose(bow=true)`.
///
/// When `bow` is false the bow block is skipped and the function returns false
/// after wound/age/bloody gates (Haxe has no melee branch).
// Haxe: AiBase.hasWeaponClose L5716–5742
pub fn has_weapon_close(
    bow: bool,
    wounded: bool,
    age: f32,
    min_ai_age_for_combat: f32,
    held_bloody: bool,
    held_parent_id: i32,
    clothing: AttackPlayerClothing,
    tiles: &[WeaponTile],
    player_x: i32,
    player_y: i32,
) -> bool {
    if wounded {
        return false;
    }
    let min_age = if min_ai_age_for_combat.is_finite() && min_ai_age_for_combat > 0.0 {
        min_ai_age_for_combat
    } else {
        MIN_AI_AGE_FOR_COMBAT
    };
    if age < min_age {
        return false;
    }
    if held_bloody {
        return false;
    }
    if !bow {
        return false;
    }
    if held_parent_id == BOW_AND_ARROW {
        return true;
    }
    let r = HAS_WEAPON_CLOSE_SEARCH;
    if tiles.iter().any(|&(id, x, y, _)| {
        id == BOW_AND_ARROW && in_has_weapon_search(player_x, player_y, x, y, r)
    }) {
        return true;
    }
    if held_parent_id == YEW_BOW && clothing.arrow_quiver {
        return true;
    }
    if held_parent_id == YEW_BOW
        && tiles.iter().any(|&(id, x, y, _)| {
            id == ARROW && in_has_weapon_search(player_x, player_y, x, y, r)
        })
    {
        return true;
    }
    clothing.quiver_with_bow
}

/// Haxe `AiBase.getWeapon(onlyBowAndArrow)`.
// Haxe: AiBase.getWeapon ~5745–5818
pub fn get_weapon(inp: &AttackPlayerInput<'_>, only_bow_and_arrow: bool) -> GetWeaponAction {
    if !only_bow_and_arrow && inp.holding_weapon {
        return GetWeaponAction::None;
    }

    if is_bloody_weapon(inp.held_id) {
        let qd = quad_i(inp.player_x, inp.player_y, inp.home_x, inp.home_y) as f32;
        if should_path_to_home(qd, BLOODY_GO_HOME_TILES) {
            return GetWeaponAction::GoHome {
                x: inp.home_x,
                y: inp.home_y,
            };
        }
        return GetWeaponAction::Wait;
    }

    let parent = inp.held_parent();
    // 151 Yew Bow → Arrow Quiver 3948 SELF slot 5
    if parent == YEW_BOW && inp.clothing.arrow_quiver {
        return GetWeaponAction::SelfClothing {
            slot: QUIVER_CLOTHING_SLOT,
        };
    }
    // Empty hand → Arrow Quiver with Bow 4151
    if inp.held_id == 0 && inp.clothing.quiver_with_bow {
        return GetWeaponAction::SelfClothing {
            slot: QUIVER_CLOTHING_SLOT,
        };
    }
    // Arrow 148 into 4149 / 4151 when canAddToQuiver
    if inp.held_id == ARROW
        && (inp.clothing.empty_quiver_with_bow || inp.clothing.quiver_with_bow)
        && inp.clothing.can_add_to_quiver
    {
        return GetWeaponAction::SelfClothing {
            slot: QUIVER_CLOTHING_SLOT,
        };
    }

    if parent != BOW_AND_ARROW {
        if inp.clothing.quiver_with_bow {
            return GetWeaponAction::DropHeld;
        }
        if !only_bow_and_arrow {
            let weapons: &[i32] = if inp.clothing.empty_quiver_with_bow {
                &[KNIFE, WAR_SWORD, BOW_AND_ARROW, ARROW]
            } else {
                &[KNIFE, WAR_SWORD, BOW_AND_ARROW]
            };
            if let Some((id, x, y, permanent)) = closest_weapon(
                inp.weapon_tiles,
                weapons,
                inp.player_x,
                inp.player_y,
                WEAPON_SEARCH_DIST,
            ) {
                // Haxe PickupObj: permanent → false, then GetOrCraft (do not try next tile)
                if !permanent {
                    return GetWeaponAction::Pickup { x, y, id };
                }
            }
        }
        if inp.is_moving {
            return GetWeaponAction::Wait;
        }
        if inp.clothing.empty_quiver_with_bow {
            return GetWeaponAction::SeekOrCraft { actor: ARROW };
        }
        return GetWeaponAction::SeekOrCraft {
            actor: BOW_AND_ARROW,
        };
    }
    GetWeaponAction::None
}

/// Haxe `AiBase.attackPlayer`.
// Haxe: AiBase.attackPlayer ~5822–5876
pub fn attack_player(inp: &AttackPlayerInput<'_>) -> AttackPlayerAction {
    let Some(target) = inp.target else {
        return AttackPlayerAction::None;
    };
    if target.wounded {
        return AttackPlayerAction::None;
    }
    if inp.food_store < ATTACK_FOOD_STORE_MIN {
        return AttackPlayerAction::None;
    }
    if inp.self_wounded {
        return AttackPlayerAction::None;
    }
    let min_age = if inp.min_ai_age_for_combat.is_finite() && inp.min_ai_age_for_combat > 0.0 {
        inp.min_ai_age_for_combat
    } else {
        MIN_AI_AGE_FOR_COMBAT
    };
    if inp.age < min_age {
        return AttackPlayerAction::None;
    }

    let gw = get_weapon(inp, false);
    if gw.is_some() {
        return AttackPlayerAction::GetWeapon(gw);
    }
    if !inp.holding_weapon {
        return AttackPlayerAction::None;
    }

    let exact_quad = calculate_exact_quad_distance_f(
        inp.exact_x,
        inp.exact_y,
        target.exact_x,
        target.exact_y,
        0,
        0,
        false,
    );
    let range = inp.deadly_distance;
    if attack_needs_stand_off(exact_quad, range) {
        let (x, y) = stand_off_tile(
            inp.player_x,
            inp.player_y,
            target.x,
            target.y,
            range,
        );
        return AttackPlayerAction::Goto { x, y };
    }
    AttackPlayerAction::Kill {
        target_p_id: target.p_id,
        tx: target.x,
        ty: target.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animal_damage::is_holding_weapon;

    fn base<'a>(tiles: &'a [WeaponTile]) -> AttackPlayerInput<'a> {
        AttackPlayerInput {
            target: Some(AttackPlayerTarget {
                p_id: 2,
                x: 3,
                y: 0,
                exact_x: 3.0,
                exact_y: 0.0,
                wounded: false,
            }),
            food_store: 10.0,
            self_wounded: false,
            age: 20.0,
            min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
            holding_weapon: true,
            held_id: KNIFE,
            held_parent_id: KNIFE,
            is_moving: false,
            player_x: 0,
            player_y: 0,
            exact_x: 0.0,
            exact_y: 0.0,
            home_x: 0,
            home_y: 0,
            deadly_distance: 1.5,
            clothing: AttackPlayerClothing::default(),
            weapon_tiles: tiles,
        }
    }

    #[test]
    fn null_wounded_food_age_gates() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.target = None;
        assert_eq!(attack_player(&inp), AttackPlayerAction::None);

        inp = base(&tiles);
        inp.target.as_mut().unwrap().wounded = true;
        assert_eq!(attack_player(&inp), AttackPlayerAction::None);

        inp = base(&tiles);
        inp.food_store = -2.5;
        assert_eq!(attack_player(&inp), AttackPlayerAction::None);

        inp = base(&tiles);
        inp.self_wounded = true;
        assert_eq!(attack_player(&inp), AttackPlayerAction::None);

        inp = base(&tiles);
        inp.age = 7.9;
        assert_eq!(attack_player(&inp), AttackPlayerAction::None);
    }

    #[test]
    fn get_weapon_skipped_when_already_holding() {
        let tiles = [(KNIFE, 2, 0, false)];
        let inp = base(&tiles);
        assert_eq!(get_weapon(&inp, false), GetWeaponAction::None);
        assert!(matches!(
            attack_player(&inp),
            AttackPlayerAction::Goto { .. } | AttackPlayerAction::Kill { .. }
        ));
    }

    #[test]
    fn empty_hand_picks_up_closest_knife() {
        let tiles = [(KNIFE, 4, 0, false), (WAR_SWORD, 8, 0, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        inp.deadly_distance = 0.0;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::GetWeapon(GetWeaponAction::Pickup {
                x: 4,
                y: 0,
                id: KNIFE
            })
        );
    }

    #[test]
    fn no_ground_weapon_seeks_bow() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::GetWeapon(GetWeaponAction::SeekOrCraft {
                actor: BOW_AND_ARROW
            })
        );
        inp.clothing.empty_quiver_with_bow = true;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::GetWeapon(GetWeaponAction::SeekOrCraft { actor: ARROW })
        );
    }

    #[test]
    fn knife_in_range_kills_adjacent() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.target.as_mut().unwrap().x = 1;
        inp.target.as_mut().unwrap().exact_x = 1.0;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::Kill {
                target_p_id: 2,
                tx: 1,
                ty: 0
            }
        );
    }

    #[test]
    fn knife_too_far_gotos_standoff_on_target_tile() {
        let tiles = [];
        let inp = base(&tiles); // target at (3,0), range 1.5 → too far (quad 9)
        let (sx, sy) = stand_off_tile(0, 0, 3, 0, 1.5);
        assert_eq!((sx, sy), (3, 0));
        assert_eq!(attack_player(&inp), AttackPlayerAction::Goto { x: 3, y: 0 });
    }

    #[test]
    fn bow_too_close_gotos_standoff() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.held_id = BOW_AND_ARROW;
        inp.held_parent_id = BOW_AND_ARROW;
        inp.deadly_distance = 4.0;
        inp.target.as_mut().unwrap().x = 1;
        inp.target.as_mut().unwrap().exact_x = 1.0;
        let (sx, sy) = stand_off_tile(0, 0, 1, 0, 4.0);
        // Equal-axis uses Haxe else: target + floor(range) - 1
        assert_eq!((sx, sy), (-2, 3));
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::Goto { x: sx, y: sy }
        );
    }

    #[test]
    fn bow_in_range_kills() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.held_id = BOW_AND_ARROW;
        inp.held_parent_id = BOW_AND_ARROW;
        inp.deadly_distance = 4.0;
        inp.target.as_mut().unwrap().x = 3;
        inp.target.as_mut().unwrap().exact_x = 3.0;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::Kill {
                target_p_id: 2,
                tx: 3,
                ty: 0
            }
        );
    }

    #[test]
    fn empty_hand_quiver_with_bow_self() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        inp.clothing.quiver_with_bow = true;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            }
        );
    }

    #[test]
    fn arrow_into_quiver_with_bow_self() {
        // Haxe L5783–5794: held.id 148 + 4149 then 4151 + canAddToQuiver → self(0,0,5)
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = ARROW;
        inp.held_parent_id = ARROW;
        inp.clothing.empty_quiver_with_bow = true;
        inp.clothing.can_add_to_quiver = true;
        assert_eq!(
            get_weapon(&inp, true),
            GetWeaponAction::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            }
        );
        inp.clothing.can_add_to_quiver = false;
        assert_ne!(
            get_weapon(&inp, true),
            GetWeaponAction::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            }
        );
    }

    #[test]
    fn has_weapon_close_bow_gates_and_search() {
        let clothes = AttackPlayerClothing::default();
        assert!(!has_weapon_close(
            true, true, 20.0, 8.0, false, BOW_AND_ARROW, clothes, &[], 0, 0
        ));
        assert!(!has_weapon_close(
            true, false, 7.9, 8.0, false, BOW_AND_ARROW, clothes, &[], 0, 0
        ));
        assert!(!has_weapon_close(
            true, false, 20.0, 8.0, true, BOW_AND_ARROW, clothes, &[], 0, 0
        ));
        assert!(has_weapon_close(
            true, false, 20.0, 8.0, false, BOW_AND_ARROW, clothes, &[], 0, 0
        ));
        // bow=false: no melee branch
        assert!(!has_weapon_close(
            false, false, 20.0, 8.0, false, KNIFE, clothes, &[], 0, 0
        ));
        let tiles = [(BOW_AND_ARROW, 19, 0, false)];
        assert!(has_weapon_close(
            true, false, 20.0, 8.0, false, 0, clothes, &tiles, 0, 0
        ));
        let edge = [(BOW_AND_ARROW, 20, 0, false)]; // half-open: x < 20 excluded
        assert!(!has_weapon_close(
            true, false, 20.0, 8.0, false, 0, clothes, &edge, 0, 0
        ));
        let yew_q = AttackPlayerClothing {
            arrow_quiver: true,
            ..Default::default()
        };
        assert!(has_weapon_close(
            true, false, 20.0, 8.0, false, YEW_BOW, yew_q, &[], 0, 0
        ));
        let arrows = [(ARROW, 5, 0, false)];
        assert!(has_weapon_close(
            true, false, 20.0, 8.0, false, YEW_BOW, clothes, &arrows, 0, 0
        ));
        let qbow = AttackPlayerClothing {
            quiver_with_bow: true,
            ..Default::default()
        };
        assert!(has_weapon_close(
            true, false, 20.0, 8.0, false, 0, qbow, &[], 0, 0
        ));
    }

    #[test]
    fn yew_bow_arrow_quiver_self() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = YEW_BOW;
        inp.held_parent_id = YEW_BOW;
        inp.clothing.arrow_quiver = true;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            }
        );
    }

    #[test]
    fn non_bow_with_quiver_bow_drops_held() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 33;
        inp.held_parent_id = 33;
        inp.clothing.quiver_with_bow = true;
        assert_eq!(get_weapon(&inp, false), GetWeaponAction::DropHeld);
    }

    #[test]
    fn bloody_far_from_home_goes_home_on_bow_path() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.held_id = 750;
        inp.held_parent_id = 750;
        inp.holding_weapon = false;
        inp.home_x = 20;
        inp.home_y = 0;
        assert_eq!(
            get_weapon(&inp, true),
            GetWeaponAction::GoHome { x: 20, y: 0 }
        );
    }

    #[test]
    fn is_holding_weapon_matches_knife() {
        assert!(is_holding_weapon(KNIFE, "Knife"));
        assert!(is_holding_weapon(BOW_AND_ARROW, "Bow and Arrow"));
        assert!(!is_holding_weapon(0, ""));
    }

    #[test]
    fn deadly_distance_falls_back_to_patches() {
        assert!((deadly_distance_for_held(KNIFE, 0.0) - 1.5).abs() < 1e-6);
        assert!((deadly_distance_for_held(BOW_AND_ARROW, 0.0) - 4.0).abs() < 1e-6);
        assert!((deadly_distance_for_held(KNIFE, 2.0) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn empty_quiver_with_bow_includes_arrow_in_pickup() {
        // Haxe L5807–5811: 4149 → weapons [560, 3047, 152, 148]
        let tiles = [(ARROW, 2, 0, false), (KNIFE, 8, 0, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        inp.clothing.empty_quiver_with_bow = true;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::Pickup {
                x: 2,
                y: 0,
                id: ARROW
            }
        );
    }

    #[test]
    fn without_4149_pickup_skips_arrow() {
        // Haxe L5809: no 4149 → [560, 3047, 152] (no 148)
        let tiles = [(ARROW, 2, 0, false), (KNIFE, 8, 0, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::Pickup {
                x: 8,
                y: 0,
                id: KNIFE
            }
        );
    }

    #[test]
    fn only_bow_skips_ground_weapon_pickup() {
        // Haxe L5807: onlyBowAndArrow == false is the pickup gate
        let tiles = [(KNIFE, 2, 0, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, true),
            GetWeaponAction::SeekOrCraft {
                actor: BOW_AND_ARROW
            }
        );
    }

    #[test]
    fn weapon_search_half_open_excludes_r40() {
        // Haxe GetClosestObjectToPositionByIds r=40: x < player+40
        let edge = [(KNIFE, 40, 0, false)];
        let mut inp = base(&edge);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::SeekOrCraft {
                actor: BOW_AND_ARROW
            }
        );
        let inside = [(KNIFE, 39, 0, false)];
        let mut inp = base(&inside);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::Pickup {
                x: 39,
                y: 0,
                id: KNIFE
            }
        );
    }

    #[test]
    fn permanent_closest_weapon_falls_through_to_craft() {
        // Haxe PickupObj permanent → false; do not pick a farther knife
        let tiles = [(BOW_AND_ARROW, 2, 0, true), (KNIFE, 8, 0, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::SeekOrCraft {
                actor: BOW_AND_ARROW
            }
        );
    }

    #[test]
    fn food_store_eq_minus_two_still_attacks() {
        // Haxe L5825: food_store < -2 refuses; -2 is allowed
        let tiles = [];
        let mut inp = base(&tiles);
        inp.food_store = -2.0;
        inp.target.as_mut().unwrap().x = 1;
        inp.target.as_mut().unwrap().exact_x = 1.0;
        assert_eq!(
            attack_player(&inp),
            AttackPlayerAction::Kill {
                target_p_id: 2,
                tx: 1,
                ty: 0
            }
        );
    }

    #[test]
    fn weapon_search_equal_quad_prefers_higher_y() {
        // Haxe scan ty outer, tx inner; equal quad last-wins
        let tiles = [(KNIFE, 3, 0, false), (WAR_SWORD, 0, 3, false)];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = 0;
        inp.held_parent_id = 0;
        assert_eq!(
            get_weapon(&inp, false),
            GetWeaponAction::Pickup {
                x: 0,
                y: 3,
                id: WAR_SWORD
            }
        );
    }

    #[test]
    fn holding_bow_and_arrow_returns_none() {
        let tiles = [];
        let mut inp = base(&tiles);
        inp.holding_weapon = false;
        inp.held_id = BOW_AND_ARROW;
        inp.held_parent_id = BOW_AND_ARROW;
        assert_eq!(get_weapon(&inp, true), GetWeaponAction::None);
        assert_eq!(get_weapon(&inp, false), GetWeaponAction::None);
    }
}
