//! Haxe `AiBase.killAnimal` (**L5878–5965**).
//!
//! Prefix (L5878–5900): when both `animal` and `animalTarget` are null, every 10s
//! look for Wolf 418 at home r=20. No wolf → `profession['HUNTER']` 0/1 from quiver.
//! Wolf found → `hasOrBecomeProfession('HUNTER')` (fail keeps `animalTarget`).
//!
//! Body (L5902–5964): `food_store < 0`; rattlesnake `shortCraft(560,764,10)`;
//! bow `minPickupAge`; `isKillableByBow` (GetTransition 152+id); quad > 400
//! refuse without clearing; `getWeapon(true)`; stand-off `useDistance` (no 0.1
//! slack); `use` always consumes.

use crate::attack_player::{
    get_weapon, stand_off_tile, AttackPlayerInput, GetWeaponAction, ARROW_QUIVER,
    ARROW_QUIVER_WITH_BOW, BOW_AND_ARROW, EMPTY_ARROW_QUIVER_WITH_BOW,
};
use crate::farmer_profession::{is_ignored_floor, AI_IGNORED_FLOOR_IDS};
use crate::hunting::{has_or_become_hunter, HunterProfessionRuntime, HUNT_KNIFE, RATTLE_SNAKE};

/// Wolf 418.
// Haxe: AiBase.killAnimal L5884
pub const WOLF: i32 = 418;
/// Empty Arrow Quiver 874.
// Haxe: AiBase.killAnimal L5890
pub const EMPTY_ARROW_QUIVER: i32 = 874;
/// `GetClosestObjectToPosition(home, 418, 20)`.
// Haxe: AiBase.killAnimal L5884
pub const KILL_ANIMAL_WOLF_SEARCH: i32 = 20;
/// `passedTime > 10` (seconds).
// Haxe: AiBase.killAnimal L5881
pub const KILL_ANIMAL_HOME_LOOK_SEC: f32 = 10.0;
/// `TimeHelper.tickTime = 1/20`.
// Haxe: TimeHelper.tickTime
pub const TIME_HELPER_TICK_TIME: f32 = 1.0 / 20.0;
/// `timeLookedForDeadlyAnimalAtHome` initial.
// Haxe: AiBase L49
pub const TIME_LOOKED_NEVER: f32 = -1.0;
/// `hasOrBecomeProfession('HUNTER')` default maxPeople.
// Haxe: AiBase.hasOrBecomeProfession max = 1; killAnimal L5897
pub const KILL_ANIMAL_HUNTER_MAX: i32 = 1;
/// `food_store < 0` refuse.
// Haxe: AiBase.killAnimal L5902
pub const KILL_ANIMAL_FOOD_MIN: f32 = 0.0;
/// `shortCraft(560, 764, 10)`.
// Haxe: AiBase.killAnimal L5907
pub const KILL_ANIMAL_SNAKE_RADIUS: i32 = 10;
/// `CalculateQuadDistanceToObject > 400` refuse (does not clear target).
// Haxe: AiBase.killAnimal L5927
pub const KILL_ANIMAL_MAX_QUAD: i32 = 400;
/// Goto fail count before clearing `animalTarget`.
// Haxe: AiBase.killAnimal L5950
pub const KILL_ANIMAL_GOTO_FAIL_CLEAR: i32 = 5;

/// Map tile for the home wolf search (`parent_id`, `x`, `y`).
pub type KillAnimalTile = (i32, i32, i32);

/// Inputs for [`kill_animal_prefix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KillAnimalPrefixInput<'a> {
    /// Haxe `animal` (`deadlyAnimal`).
    pub animal: Option<(i32, i32, i32)>,
    /// Haxe `this.animalTarget`.
    pub animal_target: Option<(i32, i32, i32)>,
    /// Haxe `timeLookedForDeadlyAnimalAtHome` (tick units).
    pub time_looked_tick: f32,
    pub now_tick: f32,
    /// Haxe `TimeHelper.tickTime` (20 Hz → 0.05; NPC scheduler may pass 0.2).
    pub tick_time: f32,
    pub clothing_ids: &'a [i32],
    /// Tiles already scanned around home (any parent; helper filters Wolf 418).
    pub home_tiles: &'a [KillAnimalTile],
    pub home_x: i32,
    pub home_y: i32,
    /// `countProfession('HUNTER')` excluding self.
    pub hunter_peer_count: f32,
    pub was_idle: f32,
}

/// Haxe `killAnimal` L5878–5900 outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillAnimalPrefixKind {
    /// `animal != null || animalTarget != null` — skip the look block.
    Skip,
    /// `return false`.
    Stop,
    /// Fall through to the rest of `killAnimal` (L5902+).
    Continue,
}

/// Prefix result (sticky `animalTarget` / look tick always written back).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KillAnimalPrefixResult {
    pub kind: KillAnimalPrefixKind,
    pub animal_target: Option<(i32, i32, i32)>,
    pub time_looked_tick: f32,
}

/// Haxe `TimeHelper.CalculateTimeSinceTicksInSec`.
// Haxe: TimeHelper.CalculateTimeSinceTicksInSec L55
pub fn time_since_ticks_in_sec(now_tick: f32, stored_tick: f32, tick_time: f32) -> f32 {
    let tt = if tick_time.is_finite() && tick_time > 0.0 {
        tick_time
    } else {
        TIME_HELPER_TICK_TIME
    };
    (now_tick - stored_tick) * tt
}

/// True when the home wolf search should run (`passedTime > 10`).
// Haxe: AiBase.killAnimal L5881
pub fn should_look_for_wolf_at_home(now_tick: f32, last_tick: f32, tick_time: f32) -> bool {
    time_since_ticks_in_sec(now_tick, last_tick, tick_time) > KILL_ANIMAL_HOME_LOOK_SEC
}

/// Haxe quiver chain 3948 → 4151 → 874 → 4149.
// Haxe: AiBase.killAnimal L5888–5891
pub fn kill_animal_has_any_quiver(clothing_ids: &[i32]) -> bool {
    clothing_ids.iter().any(|&id| {
        id == ARROW_QUIVER
            || id == ARROW_QUIVER_WITH_BOW
            || id == EMPTY_ARROW_QUIVER
            || id == EMPTY_ARROW_QUIVER_WITH_BOW
    })
}

/// Whether a map tile is skipped by `IsIgnoredFloor`.
// Haxe: AiHelper.GetClosestObjectToPositionHelper L203–204
pub fn wolf_tile_allowed(floor_id: i32, is_food: bool, is_permanent: bool) -> bool {
    !is_ignored_floor(floor_id, is_food, is_permanent, &AI_IGNORED_FLOOR_IDS)
}

/// Haxe `GetClosestObjectToPosition` half-open square `[c-r, c+r)`.
// Haxe: AiHelper.GetClosestObjectToPositionHelper `base±searchDistance` exclusive end
fn in_wolf_search(px: i32, py: i32, x: i32, y: i32, r: i32) -> bool {
    x >= px - r && x < px + r && y >= py - r && y < py + r
}

fn quad_i(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Closest Wolf 418 from home, half-open r=20, ranked by integer quad.
// Haxe: AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 418, 20)
pub fn closest_wolf_at_home(
    tiles: &[KillAnimalTile],
    home_x: i32,
    home_y: i32,
) -> Option<(i32, i32, i32)> {
    let r = KILL_ANIMAL_WOLF_SEARCH;
    let mut best: Option<(i32, i32, i32, i32)> = None; // quad, y, x, id
    for &(id, x, y) in tiles {
        if id != WOLF {
            continue;
        }
        if !in_wolf_search(home_x, home_y, x, y, r) {
            continue;
        }
        let q = quad_i(home_x, home_y, x, y);
        match best {
            None => best = Some((q, y, x, id)),
            Some((bq, by, bx, _)) => {
                // Haxe: equal quad replaces (later scan = higher y, then higher x).
                if q < bq || (q == bq && (y > by || (y == by && x > bx))) {
                    best = Some((q, y, x, id));
                }
            }
        }
    }
    best.map(|(_, y, x, id)| (id, x, y))
}

/// Haxe wolves are map objects (`GetClosestObjectToPosition` 418). Live movers live
/// in `AnimalWorld` — include them in the same home r=20 square.
// Haxe: AiBase.killAnimal L5884 GetClosestObjectToPosition(home, 418, 20)
pub fn wolf_in_home_search(home_x: i32, home_y: i32, x: i32, y: i32) -> bool {
    in_wolf_search(home_x, home_y, x, y, KILL_ANIMAL_WOLF_SEARCH)
}

/// Haxe `killAnimal` L5878–5900.
// Haxe: AiBase.killAnimal L5878–5900
pub fn kill_animal_prefix(
    inp: &KillAnimalPrefixInput<'_>,
    hunter: &mut HunterProfessionRuntime,
) -> KillAnimalPrefixResult {
    if inp.animal.is_some() || inp.animal_target.is_some() {
        return KillAnimalPrefixResult {
            kind: KillAnimalPrefixKind::Skip,
            animal_target: inp.animal_target,
            time_looked_tick: inp.time_looked_tick,
        };
    }

    let mut time_looked = inp.time_looked_tick;
    let mut animal_target = None;
    if should_look_for_wolf_at_home(inp.now_tick, time_looked, inp.tick_time) {
        time_looked = inp.now_tick;
        animal_target = closest_wolf_at_home(inp.home_tiles, inp.home_x, inp.home_y);
    }

    if animal_target.is_none() {
        hunter.weight = if kill_animal_has_any_quiver(inp.clothing_ids) {
            1.0
        } else {
            0.0
        };
        return KillAnimalPrefixResult {
            kind: KillAnimalPrefixKind::Stop,
            animal_target: None,
            time_looked_tick: time_looked,
        };
    }

    if !has_or_become_hunter(
        hunter,
        KILL_ANIMAL_HUNTER_MAX,
        inp.hunter_peer_count,
        inp.was_idle,
    ) {
        return KillAnimalPrefixResult {
            kind: KillAnimalPrefixKind::Stop,
            animal_target,
            time_looked_tick: time_looked,
        };
    }

    KillAnimalPrefixResult {
        kind: KillAnimalPrefixKind::Continue,
        animal_target,
        time_looked_tick: time_looked,
    }
}

/// Haxe `killAnimal` L5902–5964 next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillAnimalAction {
    None,
    /// `shortCraft(560, 764, 10)` — live tries; miss continues to bow hunt.
    ShortCraftSnake,
    GetWeapon(GetWeaponAction),
    /// Stand-off `gotoObj` — Haxe **always** returns true after.
    Goto { x: i32, y: i32 },
    /// `myPlayer.use` — Haxe **always** returns true after.
    Use { x: i32, y: i32 },
}

impl KillAnimalAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Inputs for [`kill_animal_body`] (after prefix Skip/Continue).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KillAnimalBodyInput<'a> {
    pub food_store: f32,
    pub age: f32,
    pub bow_min_pickup_age: f32,
    /// Passed `deadlyAnimal` `(parent_id, x, y)`.
    pub animal: Option<(i32, i32, i32)>,
    /// Sticky `animalTarget` `(parent_id, x, y)`.
    pub animal_target: Option<(i32, i32, i32)>,
    /// `GetTransition(152, animal.id) != null`.
    pub animal_killable_by_bow: bool,
    /// `GetTransition(152, animalTarget.id) != null`.
    pub target_killable_by_bow: bool,
    pub player_x: i32,
    pub player_y: i32,
    pub held_parent_id: i32,
    /// `ObjectData.getObjectData(152).parentId`.
    pub bow_parent_id: i32,
    /// `ObjectData.getObjectData(152).useDistance` (raw, not clamped).
    pub bow_use_distance: f32,
    pub weapon: AttackPlayerInput<'a>,
}

/// Body result (sticky `animalTarget` written back).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KillAnimalBodyResult {
    pub action: KillAnimalAction,
    pub animal_target: Option<(i32, i32, i32)>,
}

/// Haxe `isKillableByBow` = `GetTransition(152, id) != null`.
// Haxe: ObjectHelper.isKillableByBow L748–750
pub fn is_killable_by_bow(has_bow_transition: bool) -> bool {
    has_bow_transition
}

/// Haxe killAnimal range check (quad vs `useDistance`, **no** +0.1 slack).
// Haxe: AiBase.killAnimal L5938
pub fn kill_animal_needs_stand_off(quad: i32, use_distance: f32) -> bool {
    let range = if use_distance.is_finite() {
        use_distance as f64
    } else {
        0.0
    };
    let d = quad as f64;
    d > range * range || (range > 1.9 && d < 1.5)
}

fn kill_animal_quad(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Haxe `killAnimal` L5902–5964 after the L5878–5900 prefix.
// Haxe: AiBase.killAnimal L5902–5964
pub fn kill_animal_body(inp: &KillAnimalBodyInput<'_>) -> KillAnimalBodyResult {
    if inp.food_store < KILL_ANIMAL_FOOD_MIN {
        return KillAnimalBodyResult {
            action: KillAnimalAction::None,
            animal_target: inp.animal_target,
        };
    }

    if let Some((parent, _, _)) = inp.animal {
        if parent == RATTLE_SNAKE {
            return KillAnimalBodyResult {
                action: KillAnimalAction::ShortCraftSnake,
                animal_target: inp.animal_target,
            };
        }
    }

    kill_animal_bow_hunt(inp)
}

/// Bow hunt after snake shortCraft miss (Haxe continues on shortCraft false).
// Haxe: AiBase.killAnimal L5910–5964
pub fn kill_animal_bow_hunt(inp: &KillAnimalBodyInput<'_>) -> KillAnimalBodyResult {
    let mut animal_target = inp.animal_target;
    if inp.age < inp.bow_min_pickup_age {
        return KillAnimalBodyResult {
            action: KillAnimalAction::None,
            animal_target,
        };
    }

    if animal_target.is_some() && !inp.target_killable_by_bow {
        animal_target = None;
    }
    if animal_target.is_none() {
        if let Some(a) = inp.animal {
            if inp.animal_killable_by_bow {
                animal_target = Some(a);
            }
        }
    }
    let Some((tid, tx, ty)) = animal_target else {
        return KillAnimalBodyResult {
            action: KillAnimalAction::None,
            animal_target: None,
        };
    };

    let dist = kill_animal_quad(inp.player_x, inp.player_y, tx, ty);
    if dist > KILL_ANIMAL_MAX_QUAD {
        return KillAnimalBodyResult {
            action: KillAnimalAction::None,
            animal_target: Some((tid, tx, ty)),
        };
    }

    let gw = get_weapon(&inp.weapon, true);
    if gw.is_some() {
        return KillAnimalBodyResult {
            action: KillAnimalAction::GetWeapon(gw),
            animal_target: Some((tid, tx, ty)),
        };
    }

    let bow_parent = if inp.bow_parent_id > 0 {
        inp.bow_parent_id
    } else {
        BOW_AND_ARROW
    };
    if inp.held_parent_id != bow_parent {
        return KillAnimalBodyResult {
            action: KillAnimalAction::None,
            animal_target: Some((tid, tx, ty)),
        };
    }

    if kill_animal_needs_stand_off(dist, inp.bow_use_distance) {
        let (x, y) = stand_off_tile(
            inp.player_x,
            inp.player_y,
            tx,
            ty,
            inp.bow_use_distance,
        );
        return KillAnimalBodyResult {
            action: KillAnimalAction::Goto { x, y },
            animal_target: Some((tid, tx, ty)),
        };
    }

    KillAnimalBodyResult {
        action: KillAnimalAction::Use { x: tx, y: ty },
        animal_target: Some((tid, tx, ty)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inp<'a>(
        tiles: &'a [KillAnimalTile],
        clothes: &'a [i32],
    ) -> KillAnimalPrefixInput<'a> {
        KillAnimalPrefixInput {
            animal: None,
            animal_target: None,
            time_looked_tick: TIME_LOOKED_NEVER,
            now_tick: 200.0,
            tick_time: TIME_HELPER_TICK_TIME,
            clothing_ids: clothes,
            home_tiles: tiles,
            home_x: 0,
            home_y: 0,
            hunter_peer_count: 0.0,
            was_idle: 0.0,
        }
    }

    #[test]
    fn first_look_waits_ten_seconds_from_never() {
        // (0 - (-1)) * 0.05 = 0.05, not > 10
        assert!(!should_look_for_wolf_at_home(
            0.0,
            TIME_LOOKED_NEVER,
            TIME_HELPER_TICK_TIME
        ));
        // (200 - (-1)) * 0.05 = 10.05 > 10
        assert!(should_look_for_wolf_at_home(
            200.0,
            TIME_LOOKED_NEVER,
            TIME_HELPER_TICK_TIME
        ));
    }

    #[test]
    fn no_look_yet_sets_hunter_weight_from_quiver() {
        let tiles = [(WOLF, 3, 0)];
        let clothes = [EMPTY_ARROW_QUIVER];
        let mut inp = inp(&tiles, &clothes);
        inp.now_tick = 0.0;
        let mut hunter = HunterProfessionRuntime::default();
        let r = kill_animal_prefix(&inp, &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Stop);
        assert!(r.animal_target.is_none());
        assert_eq!(hunter.weight, 1.0);
        assert!(!hunter.is_last_hunter);
        let empty: [i32; 0] = [];
        inp.clothing_ids = &empty;
        let mut hunter = HunterProfessionRuntime::default();
        let r = kill_animal_prefix(&inp, &mut hunter);
        assert_eq!(hunter.weight, 0.0);
        assert_eq!(r.kind, KillAnimalPrefixKind::Stop);
    }

    #[test]
    fn look_miss_sets_weight_and_updates_tick() {
        let tiles = [];
        let clothes = [ARROW_QUIVER];
        let mut hunter = HunterProfessionRuntime::default();
        let r = kill_animal_prefix(&inp(&tiles, &clothes), &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Stop);
        assert!(r.animal_target.is_none());
        assert_eq!(r.time_looked_tick, 200.0);
        assert_eq!(hunter.weight, 1.0);
    }

    #[test]
    fn wolf_found_becomes_hunter_and_continues() {
        let tiles = [(WOLF, 4, 1)];
        let clothes: [i32; 0] = [];
        let mut hunter = HunterProfessionRuntime::default();
        let r = kill_animal_prefix(&inp(&tiles, &clothes), &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Continue);
        assert_eq!(r.animal_target, Some((WOLF, 4, 1)));
        assert!(hunter.is_last_hunter);
        assert_eq!(hunter.weight, 1.0);
    }

    #[test]
    fn wolf_found_peer_cap_stops_but_keeps_target() {
        let tiles = [(WOLF, 2, 0)];
        let clothes: [i32; 0] = [];
        let mut hunter = HunterProfessionRuntime::default();
        let mut inp = inp(&tiles, &clothes);
        inp.hunter_peer_count = 1.0;
        let r = kill_animal_prefix(&inp, &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Stop);
        assert_eq!(r.animal_target, Some((WOLF, 2, 0)));
        assert!(!hunter.is_last_hunter);
    }

    #[test]
    fn existing_animal_or_target_skips_look() {
        let tiles = [(WOLF, 1, 0)];
        let clothes: [i32; 0] = [];
        let mut hunter = HunterProfessionRuntime::default();
        let mut inp = inp(&tiles, &clothes);
        inp.animal = Some((418, 9, 9));
        let r = kill_animal_prefix(&inp, &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Skip);
        assert!(r.animal_target.is_none());
        assert_eq!(r.time_looked_tick, TIME_LOOKED_NEVER);
        assert_eq!(hunter.weight, 0.0);

        inp.animal = None;
        inp.animal_target = Some((WOLF, 5, 5));
        let r = kill_animal_prefix(&inp, &mut hunter);
        assert_eq!(r.kind, KillAnimalPrefixKind::Skip);
        assert_eq!(r.animal_target, Some((WOLF, 5, 5)));
    }

    #[test]
    fn wolf_search_half_open_excludes_home_plus_20() {
        let edge = [(WOLF, 20, 0)];
        assert!(closest_wolf_at_home(&edge, 0, 0).is_none());
        let inside = [(WOLF, 19, 0)];
        assert_eq!(closest_wolf_at_home(&inside, 0, 0), Some((WOLF, 19, 0)));
    }

    #[test]
    fn quiver_ids_match_haxe_chain() {
        assert!(kill_animal_has_any_quiver(&[ARROW_QUIVER]));
        assert!(kill_animal_has_any_quiver(&[ARROW_QUIVER_WITH_BOW]));
        assert!(kill_animal_has_any_quiver(&[EMPTY_ARROW_QUIVER]));
        assert!(kill_animal_has_any_quiver(&[EMPTY_ARROW_QUIVER_WITH_BOW]));
        assert!(!kill_animal_has_any_quiver(&[560]));
        assert!(!kill_animal_has_any_quiver(&[]));
    }

    #[test]
    fn wolf_equal_quad_prefers_higher_y() {
        let tiles = [(WOLF, 3, 0), (WOLF, 0, 3)];
        assert_eq!(closest_wolf_at_home(&tiles, 0, 0), Some((WOLF, 0, 3)));
    }

    #[test]
    fn ignored_floor_skips_non_permanent_non_food() {
        assert!(!wolf_tile_allowed(656, false, false));
        assert!(wolf_tile_allowed(656, false, true));
        assert!(wolf_tile_allowed(0, false, false));
    }

    fn weapon_inp<'a>(tiles: &'a [(i32, i32, i32, bool)]) -> AttackPlayerInput<'a> {
        AttackPlayerInput {
            target: None,
            food_store: 10.0,
            self_wounded: false,
            age: 20.0,
            min_ai_age_for_combat: 8.0,
            holding_weapon: false,
            held_id: BOW_AND_ARROW,
            held_parent_id: BOW_AND_ARROW,
            is_moving: false,
            player_x: 0,
            player_y: 0,
            exact_x: 0.0,
            exact_y: 0.0,
            home_x: 0,
            home_y: 0,
            deadly_distance: 0.0,
            clothing: Default::default(),
            weapon_tiles: tiles,
        }
    }

    fn body_base<'a>(weapon: AttackPlayerInput<'a>) -> KillAnimalBodyInput<'a> {
        KillAnimalBodyInput {
            food_store: 10.0,
            age: 20.0,
            bow_min_pickup_age: 10.0,
            animal: Some((WOLF, 3, 0)),
            animal_target: Some((WOLF, 3, 0)),
            animal_killable_by_bow: true,
            target_killable_by_bow: true,
            player_x: 0,
            player_y: 0,
            held_parent_id: BOW_AND_ARROW,
            bow_parent_id: BOW_AND_ARROW,
            bow_use_distance: 4.0,
            weapon,
        }
    }

    #[test]
    fn food_store_below_zero_stops() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.food_store = -0.1;
        let r = kill_animal_body(&inp);
        assert_eq!(r.action, KillAnimalAction::None);
        inp.food_store = 0.0;
        let r = kill_animal_body(&inp);
        assert_ne!(r.action, KillAnimalAction::None);
    }

    #[test]
    fn rattlesnake_short_craft_before_bow() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.animal = Some((RATTLE_SNAKE, 2, 0));
        let r = kill_animal_body(&inp);
        assert_eq!(r.action, KillAnimalAction::ShortCraftSnake);
        assert_eq!(HUNT_KNIFE, 560);
        assert_eq!(KILL_ANIMAL_SNAKE_RADIUS, 10);
    }

    #[test]
    fn snake_miss_continues_to_bow_hunt() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.animal = Some((RATTLE_SNAKE, 2, 0));
        let r = kill_animal_bow_hunt(&inp);
        assert!(matches!(
            r.action,
            KillAnimalAction::Use { .. } | KillAnimalAction::Goto { .. }
        ));
    }

    #[test]
    fn age_below_bow_min_pickup_stops() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.age = 9.9;
        inp.bow_min_pickup_age = 10.0;
        let r = kill_animal_bow_hunt(&inp);
        assert_eq!(r.action, KillAnimalAction::None);
    }

    #[test]
    fn unkillable_target_cleared_without_killable_animal() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.target_killable_by_bow = false;
        inp.animal = None;
        inp.animal_killable_by_bow = false;
        let r = kill_animal_bow_hunt(&inp);
        assert_eq!(r.action, KillAnimalAction::None);
        assert!(r.animal_target.is_none());
    }

    #[test]
    fn unkillable_target_replaced_by_killable_animal() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.animal_target = Some((WOLF, 8, 0));
        inp.target_killable_by_bow = false;
        inp.animal = Some((WOLF, 3, 0));
        inp.animal_killable_by_bow = true;
        let r = kill_animal_bow_hunt(&inp);
        assert_eq!(r.animal_target, Some((WOLF, 3, 0)));
        assert!(r.action.is_some());
    }

    #[test]
    fn quad_over_400_refuses_without_clearing_target() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.animal_target = Some((WOLF, 21, 0)); // 441 > 400
        let r = kill_animal_bow_hunt(&inp);
        assert_eq!(r.action, KillAnimalAction::None);
        assert_eq!(r.animal_target, Some((WOLF, 21, 0)));
        inp.animal_target = Some((WOLF, 20, 0)); // 400 not >
        let r = kill_animal_bow_hunt(&inp);
        assert_ne!(r.action, KillAnimalAction::None);
    }

    #[test]
    fn get_weapon_true_consumes_before_range() {
        let tiles = [];
        let mut w = weapon_inp(&tiles);
        w.held_id = 0;
        w.held_parent_id = 0;
        let mut inp = body_base(w);
        inp.held_parent_id = 0;
        let r = kill_animal_bow_hunt(&inp);
        assert!(matches!(r.action, KillAnimalAction::GetWeapon(_)));
    }

    #[test]
    fn not_holding_bow_after_get_weapon_stops() {
        let tiles = [];
        let mut w = weapon_inp(&tiles);
        w.held_id = 560;
        w.held_parent_id = 560;
        w.holding_weapon = true;
        let mut inp = body_base(w);
        inp.held_parent_id = 560;
        let r = kill_animal_bow_hunt(&inp);
        // onlyBow=true still tries getWeapon; bloody/empty-hand branches may SeekOrCraft
        assert!(matches!(
            r.action,
            KillAnimalAction::None | KillAnimalAction::GetWeapon(_)
        ));
        if matches!(r.action, KillAnimalAction::None) {
            assert_eq!(r.animal_target, Some((WOLF, 3, 0)));
        }
    }

    #[test]
    fn bow_too_far_or_too_close_gotos_standoff() {
        let tiles = [];
        let w = weapon_inp(&tiles);
        let mut inp = body_base(w);
        inp.bow_use_distance = 4.0;
        inp.animal_target = Some((WOLF, 1, 0)); // quad 1 < 1.5 and range > 1.9
        let r = kill_animal_bow_hunt(&inp);
        let (sx, sy) = stand_off_tile(0, 0, 1, 0, 4.0);
        assert_eq!(r.action, KillAnimalAction::Goto { x: sx, y: sy });
        inp.animal_target = Some((WOLF, 3, 0)); // quad 9, range 4 → 16, 9 < 16 in range
        let r = kill_animal_bow_hunt(&inp);
        assert_eq!(r.action, KillAnimalAction::Use { x: 3, y: 0 });
        inp.animal_target = Some((WOLF, 5, 0)); // quad 25 > 16
        let r = kill_animal_bow_hunt(&inp);
        let (sx, sy) = stand_off_tile(0, 0, 5, 0, 4.0);
        assert_eq!(r.action, KillAnimalAction::Goto { x: sx, y: sy });
    }

    #[test]
    fn stand_off_has_no_point_one_slack() {
        // Haxe killAnimal: `distance > range*range` (no +0.1)
        assert!(!kill_animal_needs_stand_off(16, 4.0));
        assert!(kill_animal_needs_stand_off(17, 4.0));
        assert!(kill_animal_needs_stand_off(1, 4.0)); // too close
        assert!(!is_killable_by_bow(false));
        assert!(is_killable_by_bow(true));
    }
}
