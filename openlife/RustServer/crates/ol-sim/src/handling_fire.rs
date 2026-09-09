//! Haxe: `AiBase.isHandlingFire` pure body (chunk **AI-HANDLING-FIRE** / `is_handling_fire`).
//!
//! Closes residual after **AI-MAKE-STUFF** / **AI-FIREFOOD-RUNG**:
//! - early mid-band `isHandlingFire()` (default maxProfession=1)
//! - assigned/last `FIREKEEPER` → `isHandlingFire(100)`
//! - nested `makeFireFood(2)` near coals / `makeFireFood(3)` on hot-coals firePlace
//! - late / hungry / critical `makeFireFood(1)` path max dispatch
//! - temperature cold path `isHandlingFire(2)`
//!
//! No world I/O: callers supply fire sensors + peer flags and apply returned
//! [`HandlingFireAction`]s (or expand nested makeFireFood via [`expand_handling_fire_action`]).
//!
//! Residual: kiln doPotteryOnFire commented Haxe; straw-on-ashes commented.
//! **FIRE-PLACE-STICKY** + **FIRE-BEST-AI** + **FIRE-CRAFT-R30** live.

use crate::ai_goals::Goal;
use crate::baker_profession::KINDLING;
use crate::fire_food_profession::{
    make_fire_food, FireFoodAction, FireFoodCounts, FireFoodProfessionRuntime, FIRE, HOT_COALS,
    LARGE_FAST_FIRE, LARGE_SLOW_FIRE,
};
use crate::get_or_craft::craft_item::LONG_STRAIGHT_SHAFT;

// ── Object ids (OHOL / OpenLife; Haxe comments in AiBase.isHandlingFire) ─────

/// Hot Adobe Oven 250 (near-player bake gate).
// Haxe: AiBase.isHandlingFire ~1091
pub const HOT_ADOBE_OVEN: i32 = 250;
/// Flash Fire 3029 (large fire idle).
// Haxe: AiBase.isHandlingFire ~1122
pub const FLASH_FIRE: i32 = 3029;
/// Firewood 344.
// Haxe: AiBase.isHandlingFire ~1195
pub const FIREWOOD: i32 = 344;
/// Weak Skewer 852.
// Haxe: AiBase.isHandlingFire ~1183
pub const WEAK_SKEWER: i32 = 852;
/// Skewer 139.
// Haxe: AiBase.isHandlingFire ~1177
pub const SKEWER_FOR_FIRE: i32 = 139;
/// Big Charcoal Pile 300.
// Haxe: AiBase.isHandlingFire ~1189
pub const BIG_CHARCOAL_PILE: i32 = 300;
/// Basket of Charcoal 298.
// Haxe: AiBase.isHandlingFire ~1192
pub const BASKET_OF_CHARCOAL: i32 = 298;
/// Butt Log 345.
// Haxe: AiBase.isHandlingFire ~1197
pub const BUTT_LOG: i32 = 345;
/// Chopped Tree 339.
// Haxe: AiBase.isHandlingFire ~1200
pub const CHOPPED_TREE: i32 = 339;

/// Near-player hot coals / oven radius (Haxe GetClosestObjectToPosition r=8).
// Haxe: AiBase.isHandlingFire ~1088 / ~1092
pub const HANDLING_FIRE_NEAR_RADIUS: i32 = 8;
/// CountCloseObjects radius around firePlace for skewer/charcoal/log (Haxe 30).
// Haxe: AiBase.isHandlingFire ~1178
pub const HANDLING_FIRE_COUNT_RADIUS: i32 = 30;
/// GetCloseFire home maxdist default (Haxe 20).
// Haxe: AiHelper.GetCloseFire maxdist=20
pub const GET_CLOSE_FIRE_MAXDIST: i32 = 20;
/// Shaft search near home (Haxe 20).
// Haxe: AiBase.isHandlingFire ~1104
pub const SHAFT_HOME_RADIUS: i32 = 20;
/// Shaft search near player (Haxe 40).
// Haxe: AiBase.isHandlingFire ~1105
pub const SHAFT_PLAYER_RADIUS: i32 = 40;

/// Default maxProfession for mid-band isHandlingFire().
// Haxe: isHandlingFire(maxProfession = 1)
pub const HANDLING_FIRE_DEFAULT_MAX: i32 = 1;
/// Temperature cold path isHandlingFire(2).
// Haxe: AiBase.handleTemperature ~1740
pub const HANDLING_FIRE_TEMP_MAX: i32 = 2;
/// Assigned/last FIREKEEPER isHandlingFire(100).
// Haxe: doTimeStuffHelper ~730–731
pub const HANDLING_FIRE_ASSIGNED_MAX: i32 = 100;
/// Urgent hasOrBecomeProfession('FIREKEEPER', 3) on hot coals.
// Haxe: AiBase.isHandlingFire ~1133
pub const FIRE_KEEPER_URGENT_MAX: i32 = 3;

/// Nested makeFireFood max when coals near player.
// Haxe: isHandlingFire makeFireFood(2) ~1089
pub const MAKE_FIRE_FOOD_NEAR_COALS_MAX: i32 = 2;
/// Nested makeFireFood max when firePlace is Hot Coals.
// Haxe: isHandlingFire makeFireFood(3) ~1151
pub const MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX: i32 = 3;
/// Haxe `itemToCraft.maxSearchRadius=30` around nested makeFireFood(3).
// Haxe: AiBase.isHandlingFire ~1149–1152
pub const FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS: i32 = 30;
/// Late doTimeStuffHelper makeFireFood(1).
// Haxe: ~833
pub const MAKE_FIRE_FOOD_LATE_MAX: i32 = 1;
/// Hungry isConsideringMakingFood path makeFireFood(1).
// Haxe: ~8594 / ~8603
pub const MAKE_FIRE_FOOD_HUNGRY_MAX: i32 = 1;
/// doCriticalStuff makeFireFood(1).
// Haxe: ~6107
pub const MAKE_FIRE_FOOD_CRITICAL_MAX: i32 = 1;

/// Canonical Haxe profession string.
pub const FIRE_KEEPER_PROFESSION_KEY: &str = "FIREKEEPER";

// ── Dispatch path for residual makeFireFood max people ─────────────────────

/// Context for residual / nested makeFireFood maxPeople selection.
// Haxe: makeFireFood(1|2|3|100) call sites
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireFoodDispatchPath {
    /// Assigned/last FIREFOODMAKER → 100.
    Assigned,
    /// makeStuff → 2.
    MakeStuff,
    /// isHandlingFire near-player coals → 2.
    HandlingNearCoals,
    /// isHandlingFire firePlace is Hot Coals → 3.
    HandlingHotCoalsPlace,
    /// Late doTimeStuffHelper → 1.
    Late,
    /// Hungry make-food path → 1.
    Hungry,
    /// doCriticalStuff → 1.
    Critical,
    /// Default age / generic → 1.
    Default,
}

/// Max people for a residual/nested makeFireFood path.
// Haxe: makeFireFood(max) at late/hungry/isHandlingFire/assigned/makeStuff
pub fn fire_food_max_people_for_path(path: FireFoodDispatchPath) -> i32 {
    match path {
        FireFoodDispatchPath::Assigned => crate::FIRE_FOOD_ASSIGNED_MAX_PEOPLE,
        FireFoodDispatchPath::MakeStuff | FireFoodDispatchPath::HandlingNearCoals => {
            MAKE_FIRE_FOOD_NEAR_COALS_MAX
        }
        FireFoodDispatchPath::HandlingHotCoalsPlace => MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX,
        FireFoodDispatchPath::Late
        | FireFoodDispatchPath::Hungry
        | FireFoodDispatchPath::Critical
        | FireFoodDispatchPath::Default => MAKE_FIRE_FOOD_LATE_MAX,
    }
}

// ── Profession sticky ──────────────────────────────────────────────────────

/// Sticky last + assigned + weight for FIREKEEPER.
// Haxe: AiBase.profession['FIREKEEPER'] + lastProfession / assignedProfession
#[derive(Debug, Clone, PartialEq)]
pub struct FireKeeperProfessionRuntime {
    pub is_last_fire_keeper: bool,
    pub is_assigned_fire_keeper: bool,
    /// Haxe `this.profession['FIREKEEPER']` weight (0 idle / 1 active).
    pub weight: f32,
    /// This tick ran GetCloseFire / isHandlingFire (write `Player.ai_fire_place_*`).
    // Haxe: myPlayer.firePlace = GetCloseFire
    pub fire_place_touched: bool,
    /// Haxe isHandlingFire fallthrough `myPlayer.firePlace = null`.
    // Haxe: AiBase.isHandlingFire ~1222
    pub give_up_fire_place: bool,
    /// This tick's nested makeFireFood(3) craft wrap (`Some(30)`). Copied to
    /// `itemToCraft.maxSearchRadius` then taken.
    // Haxe: isHandlingFire ~1149–1152; restore only if makeFireFood returns false
    pub craft_search_radius_override: Option<i32>,
}

impl Default for FireKeeperProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_fire_keeper: false,
            is_assigned_fire_keeper: false,
            weight: 0.0,
            fire_place_touched: false,
            give_up_fire_place: false,
            craft_search_radius_override: None,
        }
    }
}

impl FireKeeperProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    /// Haxe checkIsHungryAndEat always clears isCaringForFire; profession weight soft-clear.
    // Haxe: checkIsHungryAndEat isCaringForFire=false; getBestAi may zero profession
    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        if !last_was_foodserver {
            self.is_last_fire_keeper = false;
        }
    }
}

/// Parse speech / assigned tokens for fire keeper.
// Haxe: assignedProfession / lastProfession == 'FIREKEEPER'
pub fn parse_fire_keeper_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("FIREKEEPER") || prof.eq_ignore_ascii_case("FIREKEEP")
}

/// Assign from speech `FIREKEEPER!`.
pub fn assign_fire_keeper_from_speech(
    runtime: &mut FireKeeperProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_fire_keeper_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_fire_keeper = true;
    runtime.is_last_fire_keeper = true;
    runtime.weight = 1.0;
    true
}

/// Assigned or last FIREKEEPER job dispatch.
// Haxe: assignedProfession == 'FIREKEEPER' || lastProfession == 'FIREKEEPER'
pub fn resolve_fire_keeper_assigned_job(runtime: &FireKeeperProfessionRuntime) -> bool {
    runtime.is_assigned_fire_keeper || runtime.is_last_fire_keeper
}

/// Haxe `hasOrBecomeProfession('FIREKEEPER', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466 (same shape as FIREFOODMAKER)
pub fn has_or_become_fire_keeper(
    runtime: &mut FireKeeperProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_fire_keeper {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count_with_last.max(0.0) >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_fire_keeper = true;
    true
}

// ── GetCloseFire pure ──────────────────────────────────────────────────────

/// One map object for fire search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlingFireMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Chebyshev distance (Haxe CountCloseObjects square).
#[inline]
fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Haxe `AiHelper.GetCloseFire`: home-priority 83 → 346 → 82 → 85 within maxdist.
// Haxe: AiHelper.GetCloseFire ~2127–2133
pub fn get_close_fire(
    map: &[HandlingFireMapObj],
    home_x: i32,
    home_y: i32,
    maxdist: i32,
) -> Option<(i32, i32, i32)> {
    for &want in &[LARGE_FAST_FIRE, LARGE_SLOW_FIRE, FIRE, HOT_COALS] {
        let mut best: Option<(i32, i32, i32)> = None; // dist, x, y
        for o in map {
            if o.parent_id != want {
                continue;
            }
            let d = chebyshev(home_x, home_y, o.x, o.y);
            if d > maxdist {
                continue;
            }
            match best {
                None => best = Some((d, o.x, o.y)),
                Some((bd, _, _)) if d < bd => best = Some((d, o.x, o.y)),
                _ => {}
            }
        }
        if let Some((_, x, y)) = best {
            return Some((want, x, y));
        }
    }
    None
}

/// Sticky fire tile if still occupied, else [`get_close_fire`] + tile refresh.
///
/// Haxe `isHandlingFire` always GetCloseFire then `getObjectHelper` at that tile.
/// Persisted `Player.firePlace` coords are reused when the tile still has an object
/// so a tended fire is not swapped for a different home fire mid-job.
// Haxe: GetCloseFire + getObjectHelper(firePlace.tx, ty) ~1084 / ~1118
pub fn resolve_fire_place(
    map: &[HandlingFireMapObj],
    home_x: i32,
    home_y: i32,
    maxdist: i32,
    sticky_xy: Option<(i32, i32)>,
) -> Option<(i32, i32, i32)> {
    if let Some((sx, sy)) = sticky_xy {
        if let Some(o) = map.iter().find(|o| o.x == sx && o.y == sy && o.parent_id > 0) {
            return Some((o.parent_id, sx, sy));
        }
    }
    let (id, x, y) = get_close_fire(map, home_x, home_y, maxdist)?;
    let obj_id = map
        .iter()
        .find(|o| o.x == x && o.y == y)
        .map(|o| o.parent_id)
        .unwrap_or(id);
    Some((obj_id, x, y))
}

/// Haxe firePlace write: GetCloseFire result, or null on isHandlingFire give-up.
// Haxe: myPlayer.firePlace = firePlace ~1085; = null ~1222
pub fn commit_fire_place(resolved: Option<(i32, i32, i32)>, give_up: bool) -> (i32, i32, i32) {
    if give_up {
        (0, 0, 0)
    } else {
        resolved.unwrap_or((0, 0, 0))
    }
}

/// GPI `firePlace` stick (HOME! / isHandlingFire / makeFireFood).
// Haxe: GlobalPlayerInstance.firePlace
pub fn write_player_fire_place(p: &mut crate::Player, id: i32, x: i32, y: i32) {
    p.ai_fire_place_id = id;
    p.ai_fire_place_x = x;
    p.ai_fire_place_y = y;
}

/// Closest object id near (ox,oy) within radius (player-relative for near coals).
// Haxe: GetClosestObjectToPosition
pub fn closest_object_near(
    map: &[HandlingFireMapObj],
    ox: i32,
    oy: i32,
    want_id: i32,
    radius: i32,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, i32)> = None;
    for o in map {
        if o.parent_id != want_id {
            continue;
        }
        let d = chebyshev(ox, oy, o.x, o.y);
        if d > radius {
            continue;
        }
        match best {
            None => best = Some((d, o.x, o.y)),
            Some((bd, _, _)) if d < bd => best = Some((d, o.x, o.y)),
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

/// Count objects near (ox,oy) with held bump.
// Haxe: CountCloseObjects + heldId == id → count+1
pub fn count_close_with_held(
    map: &[HandlingFireMapObj],
    ox: i32,
    oy: i32,
    want_id: i32,
    radius: i32,
    held_id: i32,
) -> i32 {
    let mut n = 0i32;
    for o in map {
        if o.parent_id == want_id && chebyshev(ox, oy, o.x, o.y) <= radius {
            n += 1;
        }
    }
    if held_id == want_id {
        n += 1;
    }
    n
}

/// True if id is a "large fire" that isHandlingFire leaves alone.
// Haxe: objId == 83 || 346 || 3029 → return false
pub fn is_large_fire_idle(obj_id: i32) -> bool {
    matches!(obj_id, LARGE_FAST_FIRE | LARGE_SLOW_FIRE | FLASH_FIRE)
}

// ── Sensors / actions ──────────────────────────────────────────────────────

/// World sensors for pure isHandlingFire (caller-filled).
// Haxe: AiBase.isHandlingFire inputs
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingFireSensors {
    pub held_id: i32,
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    /// Current GetCloseFire result parent id (0 = none).
    pub fire_place_id: i32,
    pub fire_place_x: i32,
    pub fire_place_y: i32,
    /// Object currently at fire place tile (refreshed like getObjectHelper).
    pub obj_at_place_id: i32,
    /// Coals within r=8 of player.
    pub coals_near_player: bool,
    /// Hot Adobe Oven 250 within r=8 of player.
    pub hot_oven_near_player: bool,
    /// Path gates (Haxe isObjectNotReachable / hostile).
    pub fire_reachable: bool,
    pub fire_hostile_path: bool,
    /// Self is best FIREKEEPER for home (no fire place).
    // Haxe: getBestAiForObjByProfession('FIREKEEPER', home)
    pub is_best_fire_keeper_at_home: bool,
    /// Self is best FIREKEEPER for fire place (non-urgent path).
    // Haxe: getBestAiForObjByProfession('FIREKEEPER', firePlace)
    pub is_best_fire_keeper_at_fire: bool,
    /// Season winter → kindling on Fire 82 first.
    // Haxe: TimeHelper.Season == Winter
    pub is_winter: bool,
    /// Shaft 67 near home (r=20) or player (r=40).
    pub has_shaft_near: bool,
    /// Counts around fire place (skewer / weak / charcoal / log / firewood).
    pub count_skewer: i32,
    pub count_weak_skewer: i32,
    pub count_charcoal_pile: i32,
    pub count_butt_log_and_chopped: i32,
    /// Firewood 344 near fire place (held bumps count).
    // Haxe: shortCraftOnTarget(344) then fallthrough ~1195–1206
    pub count_firewood: i32,
    /// FIREKEEPER peer count for hasOrBecome max=3 urgent.
    pub fire_keeper_peer_count: f32,
    pub was_idle: f32,
}

impl Default for HandlingFireSensors {
    fn default() -> Self {
        Self {
            held_id: 0,
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            fire_place_id: 0,
            fire_place_x: 0,
            fire_place_y: 0,
            obj_at_place_id: 0,
            coals_near_player: false,
            hot_oven_near_player: false,
            fire_reachable: true,
            fire_hostile_path: false,
            is_best_fire_keeper_at_home: true,
            is_best_fire_keeper_at_fire: true,
            is_winter: false,
            has_shaft_near: true,
            count_skewer: 0,
            count_weak_skewer: 0,
            count_charcoal_pile: 0,
            count_butt_log_and_chopped: 0,
            count_firewood: 0,
            fire_keeper_peer_count: 0.0,
            was_idle: 0.0,
        }
    }
}

/// Pure decision output for isHandlingFire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlingFireAction {
    None,
    /// Nested makeFireFood(max) — expand via [`expand_handling_fire_action`].
    MakeFireFood {
        max_people: i32,
    },
    /// Nested doBaking(2) when hot oven near player.
    // Haxe: doBaking(2) ~1093
    DoBaking {
        max_people: i32,
    },
    /// CraftItem(id).
    CraftItem {
        object_id: i32,
    },
    /// shortCraftOnTarget(actor, firePlace) — target is fire place object id.
    ShortCraftOnFire {
        actor: i32,
        fire_object_id: i32,
    },
    /// Held object use on fire place (kindling on hot coals).
    // Haxe: useHeldObjOnTarget(firePlace) ~1155
    UseHeldOnFire {
        fire_object_id: i32,
    },
    /// GetOrCraftItem(id) — kindling / etc.
    GetOrCraft {
        object_id: i32,
    },
}

impl HandlingFireAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Fill sensors from map scan (pure helper for tests / profession_scan).
// Haxe: GetCloseFire + near coals/oven + count around fire
pub fn handling_fire_sensors_from_map(
    map: &[HandlingFireMapObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    is_winter: bool,
    fire_reachable: bool,
    fire_hostile_path: bool,
    is_best_at_home: bool,
    is_best_at_fire: bool,
    fire_keeper_peer_count: f32,
    was_idle: f32,
) -> HandlingFireSensors {
    handling_fire_sensors_from_map_ex(
        map,
        held_id,
        player_x,
        player_y,
        home_x,
        home_y,
        is_winter,
        fire_reachable,
        fire_hostile_path,
        is_best_at_home,
        is_best_at_fire,
        fire_keeper_peer_count,
        was_idle,
        None,
    )
}

/// Like [`handling_fire_sensors_from_map`] with sticky `Player.firePlace` tile.
// Haxe: FIRE-PLACE-STICKY / GetCloseFire then getObjectHelper
pub fn handling_fire_sensors_from_map_ex(
    map: &[HandlingFireMapObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    is_winter: bool,
    fire_reachable: bool,
    fire_hostile_path: bool,
    is_best_at_home: bool,
    is_best_at_fire: bool,
    fire_keeper_peer_count: f32,
    was_idle: f32,
    sticky_xy: Option<(i32, i32)>,
) -> HandlingFireSensors {
    let close = resolve_fire_place(map, home_x, home_y, GET_CLOSE_FIRE_MAXDIST, sticky_xy);
    let (fire_place_id, fire_place_x, fire_place_y) = close.unwrap_or((0, home_x, home_y));
    // Haxe: objAtPlace = getObjectHelper(firePlace)
    let obj_at_place_id = if fire_place_id != 0 {
        map.iter()
            .find(|o| o.x == fire_place_x && o.y == fire_place_y)
            .map(|o| o.parent_id)
            .unwrap_or(fire_place_id)
    } else {
        0
    };
    let coals_near_player = closest_object_near(
        map,
        player_x,
        player_y,
        HOT_COALS,
        HANDLING_FIRE_NEAR_RADIUS,
    )
    .is_some();
    let hot_oven_near_player = closest_object_near(
        map,
        player_x,
        player_y,
        HOT_ADOBE_OVEN,
        HANDLING_FIRE_NEAR_RADIUS,
    )
    .is_some();
    let shaft_home =
        closest_object_near(map, home_x, home_y, LONG_STRAIGHT_SHAFT, SHAFT_HOME_RADIUS).is_some();
    let shaft_player = closest_object_near(
        map,
        player_x,
        player_y,
        LONG_STRAIGHT_SHAFT,
        SHAFT_PLAYER_RADIUS,
    )
    .is_some();
    let fx = fire_place_x;
    let fy = fire_place_y;
    let count_skewer = count_close_with_held(
        map,
        fx,
        fy,
        SKEWER_FOR_FIRE,
        HANDLING_FIRE_COUNT_RADIUS,
        held_id,
    );
    let count_weak_skewer = count_close_with_held(
        map,
        fx,
        fy,
        WEAK_SKEWER,
        HANDLING_FIRE_COUNT_RADIUS,
        held_id,
    );
    // Haxe: count pile 300; held 298 bumps count for basket shortCraft gate
    let mut count_charcoal_pile = count_close_with_held(
        map,
        fx,
        fy,
        BIG_CHARCOAL_PILE,
        HANDLING_FIRE_COUNT_RADIUS,
        0,
    );
    if held_id == BASKET_OF_CHARCOAL {
        count_charcoal_pile += 1;
    }
    let count_butt =
        count_close_with_held(map, fx, fy, BUTT_LOG, HANDLING_FIRE_COUNT_RADIUS, held_id);
    let count_chopped =
        count_close_with_held(map, fx, fy, CHOPPED_TREE, HANDLING_FIRE_COUNT_RADIUS, 0);
    let count_firewood =
        count_close_with_held(map, fx, fy, FIREWOOD, HANDLING_FIRE_COUNT_RADIUS, held_id);
    HandlingFireSensors {
        held_id,
        player_x,
        player_y,
        home_x,
        home_y,
        fire_place_id,
        fire_place_x,
        fire_place_y,
        obj_at_place_id,
        coals_near_player,
        hot_oven_near_player,
        fire_reachable,
        fire_hostile_path,
        is_best_fire_keeper_at_home: is_best_at_home,
        is_best_fire_keeper_at_fire: is_best_at_fire,
        is_winter,
        has_shaft_near: shaft_home || shaft_player || held_id == LONG_STRAIGHT_SHAFT,
        count_skewer,
        count_weak_skewer,
        count_charcoal_pile,
        count_butt_log_and_chopped: count_butt + count_chopped,
        count_firewood,
        fire_keeper_peer_count,
        was_idle,
    }
}

/// Full pure `isHandlingFire(maxProfession)` body (without expanding nested makeFireFood/bake).
// Haxe: AiBase.isHandlingFire ~1079–1224
pub fn is_handling_fire(
    sensors: &HandlingFireSensors,
    fire_keeper: &mut FireKeeperProfessionRuntime,
    max_profession: i32,
) -> HandlingFireAction {
    let _ = max_profession; // Haxe param reserved; urgent uses fixed max=3
    fire_keeper.give_up_fire_place = false;
    fire_keeper.craft_search_radius_override = None;

    // Hot Coals near player → makeFireFood(2)
    // Haxe: ~1088–1089
    if sensors.coals_near_player {
        return HandlingFireAction::MakeFireFood {
            max_people: MAKE_FIRE_FOOD_NEAR_COALS_MAX,
        };
    }

    // Hot Adobe Oven near player → doBaking(2)
    // Haxe: ~1091–1093
    if sensors.hot_oven_near_player {
        return HandlingFireAction::DoBaking { max_people: 2 };
    }

    // No fire place → best FIREKEEPER crafts Fire (shaft first)
    // Haxe: ~1100–1111
    if sensors.fire_place_id == 0 {
        if sensors.is_best_fire_keeper_at_home {
            fire_keeper.weight = 1.0;
            fire_keeper.is_last_fire_keeper = true;
            if !sensors.has_shaft_near {
                return HandlingFireAction::CraftItem {
                    object_id: LONG_STRAIGHT_SHAFT,
                };
            }
            return HandlingFireAction::CraftItem { object_id: FIRE };
        }
        // Haxe getBestAiForObjByProfession: this.profession['FIREKEEPER']=0 when another wins
        fire_keeper.weight = 0.0;
        return HandlingFireAction::None;
    }

    if !sensors.fire_reachable || sensors.fire_hostile_path {
        return HandlingFireAction::None;
    }

    let obj_id = if sensors.obj_at_place_id != 0 {
        sensors.obj_at_place_id
    } else {
        sensors.fire_place_id
    };

    // Large fires: leave alone
    // Haxe: ~1122–1130
    if is_large_fire_idle(obj_id) {
        return HandlingFireAction::None;
    }

    // Urgent hot coals: hasOrBecome FIREKEEPER(3) → self is always "best"
    // Haxe: ~1133–1135
    let is_urgent = obj_id == HOT_COALS
        && has_or_become_fire_keeper(
            fire_keeper,
            FIRE_KEEPER_URGENT_MAX,
            sensors.fire_keeper_peer_count,
            sensors.was_idle,
        );
    if !is_urgent && !sensors.is_best_fire_keeper_at_fire {
        // Haxe getBestAiForObjByProfession: this.profession['FIREKEEPER']=0 when another wins
        fire_keeper.weight = 0.0;
        return HandlingFireAction::None;
    }
    if is_urgent || sensors.is_best_fire_keeper_at_fire {
        fire_keeper.weight = 1.0;
        fire_keeper.is_last_fire_keeper = true;
    }

    // Hot Coals 85
    // Haxe: ~1147–1166
    if obj_id == HOT_COALS {
        // Haxe: tmpSearchRadius; itemToCraft.maxSearchRadius=30; makeFireFood(3)
        fire_keeper.craft_search_radius_override = Some(FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS);
        // Prefer nested makeFireFood(3) first (caller expands; if empty, continue below)
        // Returned as action so expand_handling_fire_action can fall through.
        return HandlingFireAction::MakeFireFood {
            max_people: MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX,
        };
    }

    // Fire 82 fuel ladder
    // Haxe: ~1170–1206
    if obj_id == FIRE {
        if sensors.is_winter {
            return HandlingFireAction::ShortCraftOnFire {
                actor: KINDLING,
                fire_object_id: FIRE,
            };
        }
        if sensors.count_skewer > 10 {
            return HandlingFireAction::ShortCraftOnFire {
                actor: SKEWER_FOR_FIRE,
                fire_object_id: FIRE,
            };
        }
        if sensors.count_weak_skewer > 5 {
            return HandlingFireAction::ShortCraftOnFire {
                actor: WEAK_SKEWER,
                fire_object_id: FIRE,
            };
        }
        if sensors.count_charcoal_pile > 10 {
            return HandlingFireAction::ShortCraftOnFire {
                actor: BASKET_OF_CHARCOAL,
                fire_object_id: FIRE,
            };
        }
        // Firewood 344: Haxe shortCraftOnTarget(344) then same-tick fallthrough
        // butt log (count>10) / kindling. Pure mirrors success-when-stocked cascade.
        // Haxe: ~1195–1206
        if sensors.count_firewood > 0 {
            return HandlingFireAction::ShortCraftOnFire {
                actor: FIREWOOD,
                fire_object_id: FIRE,
            };
        }
        return is_handling_fire_fire_fuel_tail(sensors);
    }

    // Fallthrough: clear caring + null firePlace
    // Haxe: myPlayer.firePlace = null; return false ~1222–1224
    fire_keeper.clear_weight();
    fire_keeper.give_up_fire_place = true;
    HandlingFireAction::None
}

/// After hot-coals MakeFireFood(3) expands to empty, resume kindling path.
// Haxe: ~1151 then held kindling / GetOrCraft(72) ~1154–1165
pub fn is_handling_fire_hot_coals_kindling(sensors: &HandlingFireSensors) -> HandlingFireAction {
    let obj_id = if sensors.obj_at_place_id != 0 {
        sensors.obj_at_place_id
    } else {
        sensors.fire_place_id
    };
    if obj_id != HOT_COALS {
        return HandlingFireAction::None;
    }
    if sensors.held_id == KINDLING {
        return HandlingFireAction::UseHeldOnFire {
            fire_object_id: HOT_COALS,
        };
    }
    HandlingFireAction::GetOrCraft {
        object_id: KINDLING,
    }
}

/// Fire 82 fuel tail after firewood shortCraft would fail (butt log / kindling).
// Haxe: ~1197–1206 after firewood
pub fn is_handling_fire_fire_fuel_tail(sensors: &HandlingFireSensors) -> HandlingFireAction {
    let obj_id = if sensors.obj_at_place_id != 0 {
        sensors.obj_at_place_id
    } else {
        sensors.fire_place_id
    };
    if obj_id != FIRE {
        return HandlingFireAction::None;
    }
    if sensors.count_butt_log_and_chopped > 10 {
        return HandlingFireAction::ShortCraftOnFire {
            actor: BUTT_LOG,
            fire_object_id: FIRE,
        };
    }
    HandlingFireAction::ShortCraftOnFire {
        actor: KINDLING,
        fire_object_id: FIRE,
    }
}

/// Expand nested MakeFireFood via pure `make_fire_food`; if empty on hot-coals path,
/// continue with kindling. Fire-82 firewood action with no stock falls to fuel tail.
// Haxe: makeFireFood then kindling fallthrough on objId==85; firewood→butt→kindling ~1195
pub fn expand_handling_fire_action(
    action: HandlingFireAction,
    sensors: &HandlingFireSensors,
    fire_food_counts: &FireFoodCounts,
    fire_food_rt: &mut FireFoodProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
) -> HandlingFireAction {
    match action {
        HandlingFireAction::MakeFireFood { max_people } => {
            let a = make_fire_food(
                fire_food_counts,
                fire_food_rt,
                max_people,
                peer_count,
                was_idle,
            );
            if a.is_some() {
                return fire_food_action_to_handling(a);
            }
            // Hot coals place: kindling after empty makeFireFood(3)
            if max_people == MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX {
                return is_handling_fire_hot_coals_kindling(sensors);
            }
            // Near coals makeFireFood(2) empty → no further work from that branch
            HandlingFireAction::None
        }
        // Defensive: if live/caller still has firewood ShortCraft with empty stock,
        // re-apply fuel tail (same-tick Haxe fallthrough after shortCraft fail).
        HandlingFireAction::ShortCraftOnFire {
            actor,
            fire_object_id,
        } if actor == FIREWOOD && fire_object_id == FIRE && sensors.count_firewood == 0 => {
            is_handling_fire_fire_fuel_tail(sensors)
        }
        other => other,
    }
}

/// Map FireFoodAction into HandlingFireAction concrete form.
fn fire_food_action_to_handling(a: FireFoodAction) -> HandlingFireAction {
    match a {
        FireFoodAction::None | FireFoodAction::Abort => HandlingFireAction::None,
        FireFoodAction::ShortCraft { actor, target } => {
            // Prefer ShortCraftOnFire when target is coals/fire
            if target == HOT_COALS || target == FIRE || target == LARGE_FAST_FIRE {
                HandlingFireAction::ShortCraftOnFire {
                    actor,
                    fire_object_id: target,
                }
            } else {
                // Represent generic short craft as craft seek of actor (live maps shortCraft)
                HandlingFireAction::ShortCraftOnFire {
                    actor,
                    fire_object_id: target,
                }
            }
        }
        FireFoodAction::ShortCraftOnGround { target } => HandlingFireAction::ShortCraftOnFire {
            actor: 0,
            fire_object_id: target,
        },
        FireFoodAction::CraftItem { object_id } => HandlingFireAction::CraftItem { object_id },
    }
}

/// Copy nested makeFireFood(3) wrap onto `itemToCraft.maxSearchRadius`, then take.
///
/// Haxe keeps 30 when makeFireFood returns true (think tick ends). Restore already
/// cleared the flag when makeFireFood returned false (kindling GetOrCraft).
// Haxe: isHandlingFire ~1149–1152
pub fn apply_fire_craft_search_radius_override(
    fire_keeper: &mut FireKeeperProfessionRuntime,
    max_search_radius: &mut i32,
) {
    if let Some(r) = fire_keeper.craft_search_radius_override.take() {
        *max_search_radius = r;
    }
}

/// Haxe restore `tmpSearchRadius` when nested makeFireFood(3) returns false.
// Haxe: isHandlingFire ~1151–1152
fn restore_fire_craft_radius_if_make_fire_food_empty(
    fire_keeper: &mut FireKeeperProfessionRuntime,
    raw: HandlingFireAction,
    expanded: HandlingFireAction,
) {
    let was_hot_coals_wrap = matches!(
        raw,
        HandlingFireAction::MakeFireFood {
            max_people: MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX
        }
    );
    if !was_hot_coals_wrap {
        return;
    }
    // Success maps to CraftItem / ShortCraftOnFire (keep 30). Empty → kindling
    // GetOrCraft / UseHeld / None (restore).
    if matches!(
        expanded,
        HandlingFireAction::GetOrCraft { .. }
            | HandlingFireAction::UseHeldOnFire { .. }
            | HandlingFireAction::None
    ) {
        fire_keeper.craft_search_radius_override = None;
    }
}

/// Full pure decision: is_handling_fire + expand nested makeFireFood.
// Haxe: isHandlingFire with makeFireFood nested returns
pub fn is_handling_fire_full(
    sensors: &HandlingFireSensors,
    fire_keeper: &mut FireKeeperProfessionRuntime,
    fire_food_counts: &FireFoodCounts,
    fire_food_rt: &mut FireFoodProfessionRuntime,
    max_profession: i32,
    peer_count: f32,
    was_idle: f32,
) -> HandlingFireAction {
    let raw = is_handling_fire(sensors, fire_keeper, max_profession);
    let expanded = expand_handling_fire_action(
        raw,
        sensors,
        fire_food_counts,
        fire_food_rt,
        peer_count,
        was_idle,
    );
    restore_fire_craft_radius_if_make_fire_food_empty(fire_keeper, raw, expanded);
    expanded
}

/// Late / hungry / critical residual makeFireFood(1).
// Haxe: makeFireFood(1) ~833 / ~6107 / ~8594 / ~8603
pub fn make_fire_food_late_or_hungry(
    counts: &FireFoodCounts,
    runtime: &mut FireFoodProfessionRuntime,
    path: FireFoodDispatchPath,
    peer_count: f32,
    was_idle: f32,
) -> FireFoodAction {
    let max = fire_food_max_people_for_path(path);
    make_fire_food(counts, runtime, max, peer_count, was_idle)
}

/// Ladder / mid-band rung labels that run isHandlingFire early.
// Haxe: mid critical band ~634; assigned FIREKEEPER ~730; temperature ~1740
pub fn handling_fire_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CRITICAL_CRAFT"
            | "TEMPERATURE"
            | "LOW_PRIORITY_WORK"
            | "AGE_ROTATED_JOB"
            | "CONSIDER_MAKE_FOOD"
    )
}

/// Max profession for isHandlingFire from rung / assigned flag.
// Haxe: isHandlingFire() / (2) / (100)
pub fn handling_fire_max_for_dispatch(is_assigned_fire_keeper: bool, rung_label: &str) -> i32 {
    if is_assigned_fire_keeper || rung_label == "ASSIGNED_JOB" {
        HANDLING_FIRE_ASSIGNED_MAX
    } else if rung_label == "TEMPERATURE" {
        HANDLING_FIRE_TEMP_MAX
    } else {
        HANDLING_FIRE_DEFAULT_MAX
    }
}

/// Thin ladder bridge: FIREKEEPER assigned/last or mid early handling.
// Haxe: assigned FIREKEEPER → isHandlingFire(100); mid → isHandlingFire()
pub fn try_decide_handling_fire_from_rung(
    profession_is_sticky: bool,
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &HandlingFireSensors,
    fire_keeper: &mut FireKeeperProfessionRuntime,
    fire_food_counts: &FireFoodCounts,
    fire_food_rt: &mut FireFoodProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
) -> Option<HandlingFireAction> {
    if !handling_fire_job_rung_label(rung_label) {
        return None;
    }
    let _ = profession_is_sticky;
    let assigned = is_assigned_job
        || fire_keeper.is_assigned_fire_keeper
        || fire_keeper.is_last_fire_keeper
        || rung_label == "ASSIGNED_JOB";
    // Mid-band always tries isHandlingFire regardless of sticky FIREKEEPER
    // (Haxe early isHandlingFire() is unconditional).
    let max = handling_fire_max_for_dispatch(assigned, rung_label);
    Some(is_handling_fire_full(
        sensors,
        fire_keeper,
        fire_food_counts,
        fire_food_rt,
        max,
        peer_count,
        was_idle,
    ))
}

/// Map action → self-play goal.
pub fn handling_fire_action_to_goal(action: HandlingFireAction) -> Goal {
    match action {
        HandlingFireAction::None => Goal::SeekObject(FIRE),
        HandlingFireAction::MakeFireFood { .. } => Goal::SeekObject(HOT_COALS),
        HandlingFireAction::DoBaking { .. } => Goal::SeekObject(HOT_ADOBE_OVEN),
        HandlingFireAction::CraftItem { object_id } => Goal::SeekObject(object_id),
        HandlingFireAction::ShortCraftOnFire { fire_object_id, .. }
        | HandlingFireAction::UseHeldOnFire { fire_object_id } => Goal::SeekObject(fire_object_id),
        HandlingFireAction::GetOrCraft { object_id } => Goal::SeekObject(object_id),
    }
}

/// Pure best-AI gate: self wins when no peer has FIREKEEPER weight, or self has weight
/// and is closest (caller supplies self_is_closest among eligible).
///
/// Live handling-fire uses [`is_self_best_fire_keeper_for_obj`] (Haxe distance pick).
// Haxe: getBestAiForObjByProfession('FIREKEEPER', obj) ~1311
pub fn is_self_best_fire_keeper(
    self_weight: f32,
    self_is_closest_among_eligible: bool,
    any_peer_with_profession: bool,
) -> bool {
    if self_weight > 0.0 && self_is_closest_among_eligible {
        return true;
    }
    if !any_peer_with_profession && self_is_closest_among_eligible {
        return true;
    }
    false
}

/// One AI for Haxe `getBestAiForObjByProfession('FIREKEEPER', home|firePlace)`.
// Haxe: AiBase.getBestAiForObjByProfession ~1311
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireKeeperPeer {
    pub p_id: i32,
    /// Squared Euclidean distance to home or firePlace (no torus wrap).
    pub quad_dist_to_obj: f32,
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub same_home: bool,
    /// Haxe `profession['FIREKEEPER'] > 0` (weight, not lastProfession).
    pub has_fire_keeper: bool,
}

impl FireKeeperPeer {
    fn eligible(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        if self.age > max_age - 2.0 {
            return false;
        }
        if self.is_wounded {
            return false;
        }
        if self.food_store < 2.0 {
            return false;
        }
        if !self.same_home {
            return false;
        }
        true
    }
}

/// Haxe `getBestAiForObjByProfession('FIREKEEPER', obj)` — true when `self` wins.
///
/// Others without FIREKEEPER weight are skipped. Self without weight is included
/// with +100 quad. Closest remaining candidate is assigned.
// Haxe: AiBase.getBestAiForObjByProfession ~1311; isHandlingFire ~1100 / ~1134
pub fn is_self_best_fire_keeper_for_obj(
    self_p_id: i32,
    peers: &[FireKeeperPeer],
    min_age_to_eat: f32,
    max_age: f32,
) -> bool {
    let mut best_id: Option<i32> = None;
    let mut best_dist = f32::MAX;
    for p in peers {
        if !p.eligible(min_age_to_eat, max_age) {
            continue;
        }
        if !p.has_fire_keeper && p.p_id != self_p_id {
            continue;
        }
        let mut dist = p.quad_dist_to_obj;
        if !p.has_fire_keeper {
            dist += 100.0;
        }
        if best_id.is_some() && dist >= best_dist {
            continue;
        }
        best_dist = dist;
        best_id = Some(p.p_id);
    }
    best_id == Some(self_p_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baker_profession::RAW_MUTTON;
    use crate::fire_food_profession::fire_food_counts_from_nearby;

    fn rt() -> FireKeeperProfessionRuntime {
        FireKeeperProfessionRuntime::default()
    }

    fn food_rt() -> FireFoodProfessionRuntime {
        FireFoodProfessionRuntime::default()
    }

    #[test]
    fn speech_and_assign_fire_keeper() {
        assert!(parse_fire_keeper_profession_speech("FIREKEEPER!"));
        assert!(parse_fire_keeper_profession_speech("firekeep"));
        assert!(!parse_fire_keeper_profession_speech("BAKER!"));
        let mut r = rt();
        assert!(assign_fire_keeper_from_speech(&mut r, "FIREKEEPER!"));
        assert!(resolve_fire_keeper_assigned_job(&r));
        assert_eq!(r.weight, 1.0);
    }

    #[test]
    fn max_people_paths() {
        assert_eq!(
            fire_food_max_people_for_path(FireFoodDispatchPath::Assigned),
            100
        );
        assert_eq!(
            fire_food_max_people_for_path(FireFoodDispatchPath::HandlingNearCoals),
            2
        );
        assert_eq!(
            fire_food_max_people_for_path(FireFoodDispatchPath::HandlingHotCoalsPlace),
            3
        );
        assert_eq!(fire_food_max_people_for_path(FireFoodDispatchPath::Late), 1);
        assert_eq!(
            fire_food_max_people_for_path(FireFoodDispatchPath::Hungry),
            1
        );
        assert_eq!(
            fire_food_max_people_for_path(FireFoodDispatchPath::Critical),
            1
        );
        assert_eq!(MAKE_FIRE_FOOD_NEAR_COALS_MAX, 2);
        assert_eq!(MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX, 3);
    }

    #[test]
    fn get_close_fire_priority() {
        let map = [
            HandlingFireMapObj {
                parent_id: HOT_COALS,
                x: 0,
                y: 0,
            },
            HandlingFireMapObj {
                parent_id: FIRE,
                x: 1,
                y: 0,
            },
            HandlingFireMapObj {
                parent_id: LARGE_FAST_FIRE,
                x: 2,
                y: 0,
            },
        ];
        let g = get_close_fire(&map, 0, 0, 20).unwrap();
        assert_eq!(g.0, LARGE_FAST_FIRE);
        assert_eq!(g.1, 2);
    }

    #[test]
    fn resolve_fire_place_prefers_sticky_tile() {
        // FIRE-PLACE-STICKY: keep tended Fire 82 even if Large Fast Fire exists
        let map = [
            HandlingFireMapObj {
                parent_id: LARGE_FAST_FIRE,
                x: 0,
                y: 0,
            },
            HandlingFireMapObj {
                parent_id: FIRE,
                x: 5,
                y: 5,
            },
        ];
        let g = resolve_fire_place(&map, 0, 0, 20, Some((5, 5))).unwrap();
        assert_eq!(g, (FIRE, 5, 5));
        let search = resolve_fire_place(&map, 0, 0, 20, None).unwrap();
        assert_eq!(search.0, LARGE_FAST_FIRE);
        let gone = resolve_fire_place(&map, 0, 0, 20, Some((9, 9))).unwrap();
        assert_eq!(gone.0, LARGE_FAST_FIRE);
        assert_eq!(commit_fire_place(Some((FIRE, 5, 5)), true), (0, 0, 0));
        assert_eq!(commit_fire_place(Some((FIRE, 5, 5)), false), (FIRE, 5, 5));
        assert_eq!(commit_fire_place(None, false), (0, 0, 0));
    }

    #[test]
    fn is_handling_fire_fallthrough_gives_up_place() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: 86, // Ashes
            obj_at_place_id: 86,
            is_best_fire_keeper_at_fire: true,
            ..Default::default()
        };
        assert_eq!(is_handling_fire(&s, &mut fk, 1), HandlingFireAction::None);
        assert!(fk.give_up_fire_place);
        let s2 = HandlingFireSensors {
            fire_place_id: LARGE_FAST_FIRE,
            obj_at_place_id: LARGE_FAST_FIRE,
            is_best_fire_keeper_at_fire: true,
            ..Default::default()
        };
        assert_eq!(is_handling_fire(&s2, &mut fk, 1), HandlingFireAction::None);
        assert!(!fk.give_up_fire_place);
    }

    #[test]
    fn near_coals_returns_make_fire_food_2() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            coals_near_player: true,
            ..Default::default()
        };
        let a = is_handling_fire(&s, &mut fk, 1);
        assert_eq!(
            a,
            HandlingFireAction::MakeFireFood {
                max_people: MAKE_FIRE_FOOD_NEAR_COALS_MAX
            }
        );
    }

    #[test]
    fn near_coals_expands_to_cook_with_max_2() {
        let mut fk = rt();
        let mut fr = food_rt();
        let s = HandlingFireSensors {
            coals_near_player: true,
            ..Default::default()
        };
        let mut c = fire_food_counts_from_nearby(
            &[(HOT_COALS, 1), (RAW_MUTTON, 1)],
            0,
            false,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        c.has_hot_coals = true;
        c.has_fire_place = true;
        let a = is_handling_fire_full(&s, &mut fk, &c, &mut fr, 1, 0.0, 0.0);
        assert!(a.is_some());
        assert!(fr.is_last_fire_food);
    }

    #[test]
    fn hot_oven_defers_bake() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            hot_oven_near_player: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_fire(&s, &mut fk, 1),
            HandlingFireAction::DoBaking { max_people: 2 }
        );
    }

    #[test]
    fn no_fire_best_crafts_fire_or_shaft() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: 0,
            is_best_fire_keeper_at_home: true,
            has_shaft_near: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_fire(&s, &mut fk, 1),
            HandlingFireAction::CraftItem { object_id: FIRE }
        );
        let s2 = HandlingFireSensors {
            fire_place_id: 0,
            is_best_fire_keeper_at_home: true,
            has_shaft_near: false,
            ..Default::default()
        };
        assert_eq!(
            is_handling_fire(&s2, &mut fk, 1),
            HandlingFireAction::CraftItem {
                object_id: LONG_STRAIGHT_SHAFT
            }
        );
        let s3 = HandlingFireSensors {
            fire_place_id: 0,
            is_best_fire_keeper_at_home: false,
            ..Default::default()
        };
        assert_eq!(is_handling_fire(&s3, &mut fk, 1), HandlingFireAction::None);
    }

    #[test]
    fn large_fire_idle() {
        let mut fk = rt();
        for id in [LARGE_FAST_FIRE, LARGE_SLOW_FIRE, FLASH_FIRE] {
            let s = HandlingFireSensors {
                fire_place_id: id,
                obj_at_place_id: id,
                is_best_fire_keeper_at_fire: true,
                ..Default::default()
            };
            assert_eq!(is_handling_fire(&s, &mut fk, 1), HandlingFireAction::None);
        }
    }

    #[test]
    fn hot_coals_place_make_fire_food_3_then_kindling() {
        let mut fk = rt();
        let mut fr = food_rt();
        let s = HandlingFireSensors {
            fire_place_id: HOT_COALS,
            obj_at_place_id: HOT_COALS,
            is_best_fire_keeper_at_fire: true,
            held_id: 0,
            ..Default::default()
        };
        // Stock fire-food thresholds so makeFireFood(3) returns None → kindling fallthrough.
        // Haxe: makeFireFood empty then GetOrCraft(72) / useHeld kindling ~1151–1165
        // Cooked mutton >=3 skips cook ladder; no raws; stock crafts already met.
        let c = fire_food_counts_from_nearby(
            &[
                (crate::baker_profession::COOKED_MUTTON, 3),
                (crate::fire_food_profession::COOKED_RABBIT, 5),
                (crate::fire_food_profession::COOKED_GOOSE, 5),
                (crate::fire_food_profession::RAW_PORK, 0),
                (crate::fire_food_profession::BOWL_CARNITAS, 2),
                (crate::fire_food_profession::PLUCKED_GOOSE, 0),
            ],
            0,
            false,
            false,
            false,
            true,
            true,
            true,
            false,
        );
        let a = is_handling_fire_full(&s, &mut fk, &c, &mut fr, 1, 0.0, 0.0);
        assert_eq!(
            a,
            HandlingFireAction::GetOrCraft {
                object_id: KINDLING
            }
        );
        // Haxe restore tmpSearchRadius when makeFireFood(3) returns false
        assert_eq!(fk.craft_search_radius_override, None);
        // Held kindling → use on fire
        let s2 = HandlingFireSensors {
            held_id: KINDLING,
            ..s.clone()
        };
        let a2 = is_handling_fire_full(&s2, &mut fk, &c, &mut fr, 1, 0.0, 0.0);
        assert_eq!(
            a2,
            HandlingFireAction::UseHeldOnFire {
                fire_object_id: HOT_COALS
            }
        );
        assert_eq!(fk.craft_search_radius_override, None);
    }

    #[test]
    fn hot_coals_make_fire_food_3_sets_craft_r30() {
        // FIRE-CRAFT-R30: wrap only around nested makeFireFood(3), not near-coals (2)
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: HOT_COALS,
            obj_at_place_id: HOT_COALS,
            is_best_fire_keeper_at_fire: true,
            coals_near_player: false,
            ..Default::default()
        };
        let a = is_handling_fire(&s, &mut fk, 1);
        assert_eq!(
            a,
            HandlingFireAction::MakeFireFood {
                max_people: MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX
            }
        );
        assert_eq!(
            fk.craft_search_radius_override,
            Some(FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS)
        );
        let mut fk2 = rt();
        let near = HandlingFireSensors {
            coals_near_player: true,
            ..Default::default()
        };
        let a2 = is_handling_fire(&near, &mut fk2, 1);
        assert_eq!(
            a2,
            HandlingFireAction::MakeFireFood {
                max_people: MAKE_FIRE_FOOD_NEAR_COALS_MAX
            }
        );
        assert_eq!(fk2.craft_search_radius_override, None);
    }

    #[test]
    fn hot_coals_make_fire_food_3_success_keeps_r30() {
        // Haxe: makeFireFood(3) true → keep maxSearchRadius=30 (no restore)
        let mut fk = rt();
        let mut fr = food_rt();
        let s = HandlingFireSensors {
            fire_place_id: HOT_COALS,
            obj_at_place_id: HOT_COALS,
            is_best_fire_keeper_at_fire: true,
            coals_near_player: false,
            ..Default::default()
        };
        // has_hot_coals + has_fire_place; no raws → CraftItem RAW_MUTTON
        let c = fire_food_counts_from_nearby(
            &[],
            0,
            false,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        let a = is_handling_fire_full(&s, &mut fk, &c, &mut fr, 1, 0.0, 0.0);
        assert!(
            matches!(
                a,
                HandlingFireAction::CraftItem { .. } | HandlingFireAction::ShortCraftOnFire { .. }
            ),
            "makeFireFood(3) should act {:?}",
            a
        );
        assert_eq!(
            fk.craft_search_radius_override,
            Some(FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS)
        );
        let mut r = 60i32;
        apply_fire_craft_search_radius_override(&mut fk, &mut r);
        assert_eq!(r, FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS);
        assert_eq!(fk.craft_search_radius_override, None);
    }

    #[test]
    fn fire_82_winter_kindling_and_fuel() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: FIRE,
            obj_at_place_id: FIRE,
            is_best_fire_keeper_at_fire: true,
            is_winter: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_fire(&s, &mut fk, 1),
            HandlingFireAction::ShortCraftOnFire {
                actor: KINDLING,
                fire_object_id: FIRE
            }
        );
        let s2 = HandlingFireSensors {
            is_winter: false,
            count_skewer: 11,
            ..s
        };
        assert_eq!(
            is_handling_fire(&s2, &mut fk, 1),
            HandlingFireAction::ShortCraftOnFire {
                actor: SKEWER_FOR_FIRE,
                fire_object_id: FIRE
            }
        );
    }

    #[test]
    fn not_best_skips_non_urgent() {
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: FIRE,
            obj_at_place_id: FIRE,
            is_best_fire_keeper_at_fire: false,
            fire_keeper_peer_count: 5.0,
            ..Default::default()
        };
        assert_eq!(is_handling_fire(&s, &mut fk, 1), HandlingFireAction::None);
        assert_eq!(fk.weight, 0.0);
    }

    #[test]
    fn late_hungry_make_fire_food_max_1() {
        let mut fr = food_rt();
        let mut c = fire_food_counts_from_nearby(
            &[(HOT_COALS, 1), (RAW_MUTTON, 1)],
            0,
            true,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        c.has_hot_coals = true;
        c.has_fire_place = true;
        let a = make_fire_food_late_or_hungry(&c, &mut fr, FireFoodDispatchPath::Hungry, 0.0, 0.0);
        assert!(a.is_some());
        // peer cap at max=1: second peer blocks new
        let mut fr2 = food_rt();
        let a2 = make_fire_food_late_or_hungry(&c, &mut fr2, FireFoodDispatchPath::Late, 1.0, 0.0);
        assert!(matches!(a2, FireFoodAction::Abort));
    }

    #[test]
    fn try_decide_from_rung_mid() {
        assert!(handling_fire_job_rung_label("MID_PRIORITY_TASKS"));
        assert!(!handling_fire_job_rung_label("ESCAPE"));
        let mut fk = rt();
        let mut fr = food_rt();
        let s = HandlingFireSensors {
            coals_near_player: true,
            ..Default::default()
        };
        let c =
            fire_food_counts_from_nearby(&[], 0, false, false, false, false, false, false, false);
        let a = try_decide_handling_fire_from_rung(
            false,
            "MID_PRIORITY_TASKS",
            false,
            &s,
            &mut fk,
            &c,
            &mut fr,
            0.0,
            0.0,
        );
        assert!(a.is_some());
    }

    #[test]
    fn sensors_from_map_near_coals() {
        let map = [HandlingFireMapObj {
            parent_id: HOT_COALS,
            x: 1,
            y: 0,
        }];
        let s = handling_fire_sensors_from_map(
            &map, 0, 0, 0, 0, 0, false, true, false, true, true, 0.0, 0.0,
        );
        assert!(s.coals_near_player);
        assert_eq!(s.fire_place_id, HOT_COALS);
    }

    #[test]
    fn best_fire_keeper_gate() {
        assert!(is_self_best_fire_keeper(1.0, true, true));
        assert!(is_self_best_fire_keeper(0.0, true, false));
        assert!(!is_self_best_fire_keeper(0.0, true, true));
        assert!(!is_self_best_fire_keeper(1.0, false, true));
    }

    fn fire_peer(p_id: i32, dist: f32, has: bool, same_home: bool) -> FireKeeperPeer {
        FireKeeperPeer {
            p_id,
            quad_dist_to_obj: dist,
            deleted: false,
            age: 20.0,
            is_wounded: false,
            food_store: 10.0,
            same_home,
            has_fire_keeper: has,
        }
    }

    #[test]
    fn self_is_best_fire_keeper_when_no_peer_has_weight() {
        // Haxe: others without FIREKEEPER skipped; self +100 still only candidate
        let self_p = fire_peer(1, 4.0, false, true);
        let other = fire_peer(2, 1.0, false, true);
        assert!(is_self_best_fire_keeper_for_obj(1, &[self_p, other], 3.0, 60.0));
    }

    #[test]
    fn closer_fire_keeper_peer_wins_distance_pick() {
        let self_p = fire_peer(1, 16.0, false, true);
        let other = fire_peer(2, 1.0, true, true);
        assert!(!is_self_best_fire_keeper_for_obj(
            1,
            &[self_p, other],
            3.0,
            60.0
        ));
        assert!(is_self_best_fire_keeper_for_obj(
            2,
            &[self_p, other],
            3.0,
            60.0
        ));
    }

    #[test]
    fn fire_keeper_distance_pick_skips_other_home_and_ineligible() {
        let self_p = fire_peer(1, 25.0, false, true);
        let mut other = fire_peer(2, 1.0, true, false);
        assert!(is_self_best_fire_keeper_for_obj(
            1,
            &[self_p, other],
            3.0,
            60.0
        ));
        other.same_home = true;
        other.is_wounded = true;
        assert!(is_self_best_fire_keeper_for_obj(
            1,
            &[self_p, other],
            3.0,
            60.0
        ));
        other.is_wounded = false;
        other.food_store = 1.0;
        assert!(is_self_best_fire_keeper_for_obj(
            1,
            &[self_p, other],
            3.0,
            60.0
        ));
        other.food_store = 10.0;
        other.age = 59.0; // MaxAge-2 = 58
        assert!(is_self_best_fire_keeper_for_obj(
            1,
            &[self_p, other],
            3.0,
            60.0
        ));
    }

    #[test]
    fn not_best_at_home_zeros_fire_keeper_weight() {
        let mut fk = rt();
        fk.weight = 1.0;
        let s = HandlingFireSensors {
            fire_place_id: 0,
            is_best_fire_keeper_at_home: false,
            ..Default::default()
        };
        assert_eq!(is_handling_fire(&s, &mut fk, 1), HandlingFireAction::None);
        assert_eq!(fk.weight, 0.0);
    }

    #[test]
    fn fire_82_fuel_tail_butt_log_then_kindling() {
        // Haxe: firewood shortCraft fail → butt log when count>10 else kindling
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: FIRE,
            obj_at_place_id: FIRE,
            is_best_fire_keeper_at_fire: true,
            count_firewood: 0,
            count_butt_log_and_chopped: 11,
            ..Default::default()
        };
        assert_eq!(
            is_handling_fire(&s, &mut fk, 1),
            HandlingFireAction::ShortCraftOnFire {
                actor: BUTT_LOG,
                fire_object_id: FIRE
            }
        );
        let s2 = HandlingFireSensors {
            count_butt_log_and_chopped: 0,
            ..s.clone()
        };
        assert_eq!(
            is_handling_fire(&s2, &mut fk, 1),
            HandlingFireAction::ShortCraftOnFire {
                actor: KINDLING,
                fire_object_id: FIRE
            }
        );
        // Stocked firewood still preferred over fuel tail
        let s3 = HandlingFireSensors {
            count_firewood: 1,
            count_butt_log_and_chopped: 11,
            ..s
        };
        assert_eq!(
            is_handling_fire(&s3, &mut fk, 1),
            HandlingFireAction::ShortCraftOnFire {
                actor: FIREWOOD,
                fire_object_id: FIRE
            }
        );
    }

    #[test]
    fn temp_and_consider_max_dispatch() {
        assert_eq!(
            handling_fire_max_for_dispatch(false, "TEMPERATURE"),
            HANDLING_FIRE_TEMP_MAX
        );
        assert_eq!(
            handling_fire_max_for_dispatch(false, "CONSIDER_MAKE_FOOD"),
            HANDLING_FIRE_DEFAULT_MAX
        );
        assert_eq!(
            handling_fire_max_for_dispatch(true, "TEMPERATURE"),
            HANDLING_FIRE_ASSIGNED_MAX
        );
        assert!(handling_fire_job_rung_label("TEMPERATURE"));
        assert!(handling_fire_job_rung_label("CONSIDER_MAKE_FOOD"));
        assert!(handling_fire_job_rung_label("CRITICAL_MISC"));
    }

    #[test]
    fn urgent_hot_coals_peer_cap() {
        // Haxe: hasOrBecomeProfession('FIREKEEPER', 3) on Hot Coals
        let mut fk = rt();
        let s = HandlingFireSensors {
            fire_place_id: HOT_COALS,
            obj_at_place_id: HOT_COALS,
            is_best_fire_keeper_at_fire: false,
            fire_keeper_peer_count: 5.0,
            was_idle: 0.0,
            ..Default::default()
        };
        // peer >= 3 blocks non-last
        assert_eq!(is_handling_fire(&s, &mut fk, 1), HandlingFireAction::None);
        let mut fk2 = rt();
        let s2 = HandlingFireSensors {
            fire_keeper_peer_count: 1.0,
            ..s
        };
        // peer under cap → urgent becomes FIREKEEPER and returns MakeFireFood(3)
        assert_eq!(
            is_handling_fire(&s2, &mut fk2, 1),
            HandlingFireAction::MakeFireFood {
                max_people: MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX
            }
        );
        assert!(fk2.is_last_fire_keeper);
        assert_eq!(
            fk2.craft_search_radius_override,
            Some(FIRE_CRAFT_HOT_COALS_SEARCH_RADIUS)
        );
    }
}
