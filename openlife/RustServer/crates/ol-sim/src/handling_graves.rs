//! Haxe: `AiBase.isHandlingGraves` pure body (chunk **AI-JOB-GRAVE** / `is_handling_graves`).
//!
//! Assigned/last `GRAVEKEEPER` → `isHandlingGraves(100)`; mid/hungry default max=1.
//! Bones path uses Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)` distance pick
//! unless lastProfession is GRAVEKEEPER or age ≥ 50.
//!
//! No world I/O: callers supply grave sensors + best-AI flag.

use crate::ai_goals::Goal;

/// Fresh Grave 87 (Haxe array; comment says 97 — port array as-is).
// Haxe: AiBase.isHandlingGraves graveIdsToDigIn ~1485
pub const GRAVE_FRESH: i32 = 87;
/// Grave 88.
pub const GRAVE_88: i32 = 88;
/// Old Grave 89.
pub const GRAVE_OLD: i32 = 89;
/// Bone Pile 357.
pub const BONE_PILE: i32 = 357;
/// Ids AI may dig / empty (Haxe `[87, 88, 89, 357]`).
pub const GRAVE_DIG_IDS: [i32; 4] = [GRAVE_FRESH, GRAVE_88, GRAVE_OLD, BONE_PILE];
/// Basket of Bones 356.
pub const BASKET_OF_BONES: i32 = 356;
/// Basket 292.
pub const BASKET: i32 = 292;
/// Shovel 502.
pub const SHOVEL: i32 = 502;
/// Stone Hoe 850.
pub const STONE_HOE: i32 = 850;
/// Marked Grave 1012 (GetGraveyard first).
// Haxe: AiBase.GetGraveyard ~3637
pub const MARKED_GRAVE: i32 = 1012;
/// Buried Grave 1011.
pub const BURIED_GRAVE: i32 = 1011;

/// Default search radius (Haxe `age < 50 && age > 59` is dead → always 30).
// Haxe: isHandlingGraves searchDistance ~1469
pub const GRAVE_SEARCH_RADIUS: i32 = 30;
/// GetGraveyard home max dist.
pub const GRAVEYARD_HOME_RADIUS: i32 = 25;
/// GetGraveyard min dist from home.
pub const GRAVEYARD_HOME_MIN: i32 = 8;
/// Age at which lastProfession becomes GRAVEKEEPER and best-AI is skipped.
// Haxe: age > 50 lastProfession; age < 50 best-AI ~1526–1534
pub const GRAVE_KEEPER_OLD_AGE: f32 = 50.0;
/// Move bones if quad dist to home < 30.
pub const GRAVE_HOME_CLOSE_QUAD: i32 = 30;
/// Bone pile already moved: pickup if quad to graveyard > 100.
pub const GRAVE_YARD_BONE_PILE_QUAD: i32 = 100;
/// Other graves: pickup if quad to graveyard > 25.
pub const GRAVE_YARD_OTHER_QUAD: i32 = 25;
/// Do not haul if graveyard farther than this quad.
pub const GRAVE_YARD_MAX_QUAD: i32 = 1600;
/// GetOrCraft vs GetItem when home quad < 900.
pub const GRAVE_HOME_CRAFT_QUAD: i32 = 900;
/// Haxe `GetItem(502, 10, grave)` — shovel search from grave then player.
// Haxe: isHandlingGraves GetItem(502, 10, grave) ~1600
pub const GRAVE_SHOVEL_NEAR_RADIUS: i32 = 10;
/// Haxe `PickupItem(356)`: GetClosestObjectToTarget(home, 356, 20).
// Haxe: AiBase.PickupItem L6130–6135; isHandlingGraves L1538
pub const PICKUP_BONES_HOME_RADIUS: i32 = 20;

/// Canonical Haxe profession string.
pub const GRAVE_KEEPER_PROFESSION_KEY: &str = "GRAVEKEEPER";

/// Sticky last + assigned + weight for GRAVEKEEPER.
// Haxe: AiBase.profession['GRAVEKEEPER'] + lastProfession / assignedProfession / lastGrave
#[derive(Debug, Clone, PartialEq)]
pub struct GraveKeeperProfessionRuntime {
    pub is_last_grave_keeper: bool,
    pub is_assigned_grave_keeper: bool,
    /// Haxe `this.profession['GRAVEKEEPER']` weight (0 idle / 1 active).
    pub weight: f32,
    /// Haxe `lastGrave` sticky tile (0 id = none).
    pub last_grave_id: i32,
    pub last_grave_x: i32,
    pub last_grave_y: i32,
}

impl Default for GraveKeeperProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_grave_keeper: false,
            is_assigned_grave_keeper: false,
            weight: 0.0,
            last_grave_id: 0,
            last_grave_x: 0,
            last_grave_y: 0,
        }
    }
}

impl GraveKeeperProfessionRuntime {
    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        if !last_was_foodserver {
            self.is_last_grave_keeper = false;
        }
    }

    pub fn set_last_grave(&mut self, id: i32, x: i32, y: i32) {
        self.last_grave_id = id;
        self.last_grave_x = x;
        self.last_grave_y = y;
    }

    pub fn clear_last_grave(&mut self) {
        self.last_grave_id = 0;
        self.last_grave_x = 0;
        self.last_grave_y = 0;
    }
}

/// Parse speech / assigned tokens for grave keeper.
// Haxe: assignedProfession / lastProfession == 'GRAVEKEEPER'
pub fn parse_grave_keeper_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("GRAVEKEEPER") || prof.eq_ignore_ascii_case("GRAVEKEEP")
}

/// Assign from speech `GRAVEKEEPER!`.
pub fn assign_grave_keeper_from_speech(
    runtime: &mut GraveKeeperProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_grave_keeper_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_grave_keeper = true;
    runtime.is_last_grave_keeper = true;
    runtime.weight = 1.0;
    true
}

/// True when id is in Haxe `graveIdsToDigIn`.
#[inline]
pub fn is_grave_dig_id(id: i32) -> bool {
    GRAVE_DIG_IDS.contains(&id)
}

/// Haxe searchDistance: `isGravekeeper && age < 50 && age > 59` is never true → 30.
// Haxe: ~1469
pub fn grave_search_radius(_is_last: bool, _age: f32) -> i32 {
    GRAVE_SEARCH_RADIUS
}

/// Skip best-AI when last GRAVEKEEPER or age ≥ 50.
// Haxe: lastProfession != 'GRAVEKEEPER' && age < 50 ~1526
pub fn grave_keeper_needs_best_ai(is_last: bool, age: f32) -> bool {
    !is_last && age < GRAVE_KEEPER_OLD_AGE
}

/// Map object for grave / graveyard search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlingGravesMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    pub floor_id: i32,
    pub contained_count: i32,
    /// Haxe `ownersByPlayerAccount[0]` (0 = none).
    // Haxe: ObjectHelper.getOwnerAccount L721–724
    pub owner_account: i32,
}

#[inline]
fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

#[inline]
fn quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Closest grave-dig id within radius of player; sticky lastGrave if still valid.
// Haxe: lastGrave if still in graveIds else GetClosestObjectToPositionByIds ~1487
pub fn pick_grave(
    map: &[HandlingGravesMapObj],
    player_x: i32,
    player_y: i32,
    radius: i32,
    sticky: Option<(i32, i32, i32)>,
) -> Option<(i32, i32, i32, i32, i32)> {
    if let Some((id, x, y)) = sticky {
        if is_grave_dig_id(id) {
            if let Some(o) = map.iter().find(|o| o.x == x && o.y == y && o.parent_id == id) {
                return Some((o.parent_id, o.x, o.y, o.floor_id, o.contained_count));
            }
        }
    }
    let mut best: Option<(i32, i32, i32, i32, i32, i32)> = None; // dist, id, x, y, floor, cargo
    for o in map {
        if !is_grave_dig_id(o.parent_id) {
            continue;
        }
        let d = chebyshev(player_x, player_y, o.x, o.y);
        if d > radius {
            continue;
        }
        match best {
            None => best = Some((d, o.parent_id, o.x, o.y, o.floor_id, o.contained_count)),
            Some((bd, ..)) if d < bd => {
                best = Some((d, o.parent_id, o.x, o.y, o.floor_id, o.contained_count))
            }
            _ => {}
        }
    }
    best.map(|(_, id, x, y, f, c)| (id, x, y, f, c))
}

/// Haxe `grave.getOwnerAccount().id == myPlayer.account.id` (first account owner only).
// Haxe: AiBase.isHandlingGraves L1503–1508; ObjectHelper.getOwnerAccount
pub fn is_own_grave_account(owners_by_account: &[i32], self_account_id: i32) -> bool {
    match owners_by_account.first() {
        Some(&a) if a != 0 && a == self_account_id => true,
        _ => false,
    }
}

/// Haxe `GetGraveyard`: Marked 1012 then Buried 1011, home r=25 min 8.
// Haxe: AiBase.GetGraveyard ~3634
pub fn get_graveyard(
    map: &[HandlingGravesMapObj],
    home_x: i32,
    home_y: i32,
) -> Option<(i32, i32, i32)> {
    for &want in &[MARKED_GRAVE, BURIED_GRAVE] {
        let mut best: Option<(i32, i32, i32, i32)> = None;
        for o in map {
            if o.parent_id != want {
                continue;
            }
            let d = chebyshev(home_x, home_y, o.x, o.y);
            if d > GRAVEYARD_HOME_RADIUS {
                continue;
            }
            // Haxe GetClosest minDistance: quadDistance < min² skip (8²=64)
            let dx = o.x - home_x;
            let dy = o.y - home_y;
            if dx * dx + dy * dy < GRAVEYARD_HOME_MIN * GRAVEYARD_HOME_MIN {
                continue;
            }
            match best {
                None => best = Some((d, o.parent_id, o.x, o.y)),
                Some((bd, ..)) if d < bd => best = Some((d, o.parent_id, o.x, o.y)),
                _ => {}
            }
        }
        if let Some((_, id, x, y)) = best {
            return Some((id, x, y));
        }
    }
    None
}

/// World sensors for pure isHandlingGraves.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingGravesSensors {
    pub held_id: i32,
    pub held_contained: i32,
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub age: f32,
    pub is_hungry: bool,
    pub grave_reachable: bool,
    pub grave_hostile_path: bool,
    /// Self wins Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)`.
    pub is_best_grave_keeper: bool,
    pub grave_id: i32,
    pub grave_x: i32,
    pub grave_y: i32,
    pub grave_floor_id: i32,
    pub grave_contained: i32,
    pub is_own_grave: bool,
    /// Basket of Bones 356 on the scan (PickupItem).
    pub has_basket_of_bones: bool,
    /// Shovel 502 within r=10 of grave or player (Haxe GetItem 502,10).
    pub has_shovel_near_grave: bool,
    pub graveyard_id: i32,
    pub graveyard_x: i32,
    pub graveyard_y: i32,
}

impl Default for HandlingGravesSensors {
    fn default() -> Self {
        Self {
            held_id: 0,
            held_contained: 0,
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            age: 20.0,
            is_hungry: false,
            grave_reachable: true,
            grave_hostile_path: false,
            is_best_grave_keeper: true,
            grave_id: 0,
            grave_x: 0,
            grave_y: 0,
            grave_floor_id: 0,
            grave_contained: 0,
            is_own_grave: false,
            has_basket_of_bones: false,
            has_shovel_near_grave: false,
            graveyard_id: 0,
            graveyard_x: 0,
            graveyard_y: 0,
        }
    }
}

/// Pure decision output for isHandlingGraves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlingGravesAction {
    None,
    DropHeld,
    RemoveFromGrave { x: i32, y: i32, object_id: i32 },
    UseHeldOnGrave { x: i32, y: i32 },
    PickupItem { object_id: i32 },
    GetOrCraft { object_id: i32 },
    GetItem { object_id: i32 },
}

impl HandlingGravesAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Fill sensors from a map snapshot + sticky lastGrave.
pub fn handling_graves_sensors_from_map(
    map: &[HandlingGravesMapObj],
    held_id: i32,
    held_contained: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    age: f32,
    is_hungry: bool,
    grave_reachable: bool,
    grave_hostile_path: bool,
    is_best: bool,
    self_account_id: i32,
    sticky: Option<(i32, i32, i32)>,
) -> HandlingGravesSensors {
    // Haxe: PickupItem(356) GetClosestObjectToTarget(home, 356, 20)
    let has_basket_of_bones = map.iter().any(|o| {
        o.parent_id == BASKET_OF_BONES
            && chebyshev(o.x, o.y, home_x, home_y) <= PICKUP_BONES_HOME_RADIUS
    });
    let radius = grave_search_radius(false, age);
    let grave = pick_grave(map, player_x, player_y, radius, sticky);
    let yard = get_graveyard(map, home_x, home_y);
    let (grave_id, grave_x, grave_y, grave_floor_id, grave_contained) =
        grave.unwrap_or((0, 0, 0, 0, 0));
    let grave_owner = map
        .iter()
        .find(|o| o.x == grave_x && o.y == grave_y && o.parent_id == grave_id)
        .map(|o| o.owner_account)
        .unwrap_or(0);
    let is_own_grave = grave_owner != 0 && grave_owner == self_account_id;
    let (graveyard_id, graveyard_x, graveyard_y) = yard.unwrap_or((0, 0, 0));
    let has_shovel_near_grave = grave_id != 0
        && map.iter().any(|o| {
            o.parent_id == SHOVEL
                && (chebyshev(o.x, o.y, grave_x, grave_y) <= GRAVE_SHOVEL_NEAR_RADIUS
                    || chebyshev(o.x, o.y, player_x, player_y) <= GRAVE_SHOVEL_NEAR_RADIUS)
        });
    HandlingGravesSensors {
        held_id,
        held_contained,
        player_x,
        player_y,
        home_x,
        home_y,
        age,
        is_hungry,
        grave_reachable,
        grave_hostile_path,
        is_best_grave_keeper: is_best,
        grave_id,
        grave_x,
        grave_y,
        grave_floor_id,
        grave_contained,
        is_own_grave,
        has_basket_of_bones,
        has_shovel_near_grave,
        graveyard_id,
        graveyard_x,
        graveyard_y,
    }
}

/// Full pure `isHandlingGraves(maxPlayer)` body.
// Haxe: AiBase.isHandlingGraves ~1466–1608
pub fn is_handling_graves(
    sensors: &HandlingGravesSensors,
    grave_keeper: &mut GraveKeeperProfessionRuntime,
    _max_player: i32,
) -> HandlingGravesAction {
    let _ = _max_player; // Haxe hasOrBecome GRAVEKEEPER is commented out
    // Basket of Bones 356 → drop
    // Haxe: ~1472
    if sensors.held_id == BASKET_OF_BONES {
        return HandlingGravesAction::DropHeld;
    }

    if sensors.grave_id == 0 {
        grave_keeper.clear_last_grave();
        return HandlingGravesAction::None;
    }

    if !sensors.grave_reachable || sensors.grave_hostile_path {
        grave_keeper.clear_last_grave();
        return HandlingGravesAction::None;
    }
    grave_keeper.set_last_grave(sensors.grave_id, sensors.grave_x, sensors.grave_y);

    // cannot touch own grave
    // Haxe: ~1504–1508
    if sensors.is_own_grave {
        return HandlingGravesAction::None;
    }

    if sensors.grave_contained > 0 {
        if sensors.held_id != 0 {
            return HandlingGravesAction::DropHeld;
        }
        return HandlingGravesAction::RemoveFromGrave {
            x: sensors.grave_x,
            y: sensors.grave_y,
            object_id: sensors.grave_id,
        };
    }

    // Only take care of bones if not hungry
    // Haxe: ~1524
    if sensors.is_hungry {
        return HandlingGravesAction::None;
    }

    if grave_keeper_needs_best_ai(grave_keeper.is_last_grave_keeper, sensors.age)
        && !sensors.is_best_grave_keeper
    {
        // Haxe getBestAi: this.profession['GRAVEKEEPER']=0 when another wins
        grave_keeper.weight = 0.0;
        return HandlingGravesAction::None;
    }

    grave_keeper.weight = 1.0;
    if sensors.age > GRAVE_KEEPER_OLD_AGE {
        grave_keeper.is_last_grave_keeper = true;
    }

    // Basket of Bones 356
    // Haxe: PickupItem(356) ~1538
    if sensors.held_id == 0 && sensors.has_basket_of_bones {
        return HandlingGravesAction::PickupItem {
            object_id: BASKET_OF_BONES,
        };
    }

    let home_quad = quad_dist(
        sensors.player_x,
        sensors.player_y,
        sensors.home_x,
        sensors.home_y,
    );
    let mut pickup = sensors.grave_floor_id > 0;
    if !pickup {
        let q = quad_dist(
            sensors.home_x,
            sensors.home_y,
            sensors.grave_x,
            sensors.grave_y,
        );
        if q < GRAVE_HOME_CLOSE_QUAD {
            pickup = true;
        }
    }
    if !pickup && sensors.graveyard_id != 0 {
        let q = quad_dist(
            sensors.grave_x,
            sensors.grave_y,
            sensors.graveyard_x,
            sensors.graveyard_y,
        );
        let min_q = if sensors.grave_id == BONE_PILE {
            GRAVE_YARD_BONE_PILE_QUAD
        } else {
            GRAVE_YARD_OTHER_QUAD
        };
        if q > min_q && q < GRAVE_YARD_MAX_QUAD {
            pickup = true;
        }
    }

    if pickup {
        if sensors.held_id == BASKET {
            if sensors.held_contained > 0 {
                return HandlingGravesAction::DropHeld;
            }
            return HandlingGravesAction::UseHeldOnGrave {
                x: sensors.grave_x,
                y: sensors.grave_y,
            };
        }
        if home_quad < GRAVE_HOME_CRAFT_QUAD {
            return HandlingGravesAction::GetOrCraft { object_id: BASKET };
        }
        return HandlingGravesAction::GetItem { object_id: BASKET };
    }

    if sensors.held_id == SHOVEL || sensors.held_id == STONE_HOE {
        return HandlingGravesAction::UseHeldOnGrave {
            x: sensors.grave_x,
            y: sensors.grave_y,
        };
    }

    // Shovel GetItem(502, 10, grave) then hoe GetOrCraft/GetItem(850)
    // Haxe: ~1600–1606
    if sensors.has_shovel_near_grave {
        return HandlingGravesAction::GetItem { object_id: SHOVEL };
    }
    if home_quad < GRAVE_HOME_CRAFT_QUAD {
        return HandlingGravesAction::GetOrCraft {
            object_id: STONE_HOE,
        };
    }
    HandlingGravesAction::GetItem { object_id: STONE_HOE }
}

/// Ladder / mid-band rungs that run isHandlingGraves.
// Haxe: mid ~657; assigned ~742; hungry ~8538
pub fn handling_graves_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CONSIDER_MAKE_FOOD"
            | "LOW_PRIORITY_WORK"
            | "AGE_ROTATED_JOB"
            | "HANDLE_DEATH"
    )
}

/// Thin ladder bridge.
pub fn try_decide_handling_graves_from_rung(
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &HandlingGravesSensors,
    grave_keeper: &mut GraveKeeperProfessionRuntime,
) -> Option<HandlingGravesAction> {
    if !handling_graves_job_rung_label(rung_label) {
        return None;
    }
    let assigned = is_assigned_job
        || grave_keeper.is_assigned_grave_keeper
        || grave_keeper.is_last_grave_keeper
        || rung_label == "ASSIGNED_JOB";
    let _ = assigned;
    Some(is_handling_graves(sensors, grave_keeper, 1))
}

/// Map action → self-play goal.
pub fn handling_graves_action_to_goal(action: HandlingGravesAction) -> Goal {
    match action {
        HandlingGravesAction::None => Goal::Idle,
        HandlingGravesAction::DropHeld => Goal::Idle,
        HandlingGravesAction::RemoveFromGrave { .. }
        | HandlingGravesAction::UseHeldOnGrave { .. } => Goal::SeekObject(GRAVE_88),
        HandlingGravesAction::PickupItem { object_id }
        | HandlingGravesAction::GetOrCraft { object_id }
        | HandlingGravesAction::GetItem { object_id } => Goal::SeekObject(object_id),
    }
}

/// One AI for Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)`.
///
/// GRAVEKEEPER does **not** skip age > MaxAge−2 (Haxe exception).
// Haxe: getBestAiForObjByProfession ~1322 profession != 'GRAVEKEEPER'
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraveKeeperPeer {
    pub p_id: i32,
    pub quad_dist_to_obj: f32,
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub same_home: bool,
    /// Haxe `profession['GRAVEKEEPER'] > 0` (weight).
    pub has_grave_keeper: bool,
}

impl GraveKeeperPeer {
    fn eligible(self, min_age_to_eat: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        // GRAVEKEEPER includes old AIs (Haxe skip only when profession != GRAVEKEEPER)
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

/// True when `self` wins Haxe GRAVEKEEPER distance pick vs the grave.
// Haxe: AiBase.getBestAiForObjByProfession ~1311; isHandlingGraves ~1527
pub fn is_self_best_grave_keeper_for_obj(
    self_p_id: i32,
    peers: &[GraveKeeperPeer],
    min_age_to_eat: f32,
) -> bool {
    let mut best_id: Option<i32> = None;
    let mut best_dist = f32::MAX;
    for p in peers {
        if !p.eligible(min_age_to_eat) {
            continue;
        }
        if !p.has_grave_keeper && p.p_id != self_p_id {
            continue;
        }
        let mut dist = p.quad_dist_to_obj;
        if !p.has_grave_keeper {
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

    fn rt() -> GraveKeeperProfessionRuntime {
        GraveKeeperProfessionRuntime::default()
    }

    #[test]
    fn speech_and_assign_grave_keeper() {
        assert!(parse_grave_keeper_profession_speech("GRAVEKEEPER!"));
        assert!(parse_grave_keeper_profession_speech("gravekeep"));
        assert!(!parse_grave_keeper_profession_speech("FIREKEEPER!"));
        let mut r = rt();
        assert!(assign_grave_keeper_from_speech(&mut r, "GRAVEKEEPER!"));
        assert!(r.is_assigned_grave_keeper);
        assert_eq!(r.weight, 1.0);
    }

    #[test]
    fn search_radius_always_30_haxe_dead_condition() {
        assert_eq!(grave_search_radius(true, 55.0), 30);
        assert_eq!(grave_search_radius(false, 20.0), 30);
        assert!(grave_keeper_needs_best_ai(false, 20.0));
        assert!(!grave_keeper_needs_best_ai(true, 20.0));
        assert!(!grave_keeper_needs_best_ai(false, 50.0));
    }

    #[test]
    fn drop_basket_of_bones_first() {
        let mut gk = rt();
        let s = HandlingGravesSensors {
            held_id: BASKET_OF_BONES,
            grave_id: GRAVE_88,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::DropHeld
        );
    }

    #[test]
    fn own_grave_skips() {
        let mut gk = rt();
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 1,
            grave_y: 0,
            is_own_grave: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::None
        );
    }

    #[test]
    fn cargo_drops_then_removes() {
        let mut gk = rt();
        let s = HandlingGravesSensors {
            held_id: 33,
            grave_id: GRAVE_88,
            grave_x: 2,
            grave_y: 0,
            grave_contained: 1,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::DropHeld
        );
        let s2 = HandlingGravesSensors {
            held_id: 0,
            ..s
        };
        assert_eq!(
            is_handling_graves(&s2, &mut gk, 1),
            HandlingGravesAction::RemoveFromGrave {
                x: 2,
                y: 0,
                object_id: GRAVE_88,
            }
        );
    }

    #[test]
    fn hungry_skips_bones() {
        let mut gk = rt();
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            is_hungry: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::None
        );
    }

    #[test]
    fn not_best_zeros_weight_when_young() {
        let mut gk = rt();
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            age: 20.0,
            is_best_grave_keeper: false,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::None
        );
        assert_eq!(gk.weight, 0.0);
    }

    #[test]
    fn last_or_old_skips_best_ai() {
        let mut gk = rt();
        gk.is_last_grave_keeper = true;
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 20,
            grave_y: 0,
            home_x: 0,
            home_y: 0,
            age: 20.0,
            is_best_grave_keeper: false,
            held_id: SHOVEL,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::UseHeldOnGrave { x: 20, y: 0 }
        );
        let mut gk2 = rt();
        let s2 = HandlingGravesSensors {
            age: 55.0,
            is_best_grave_keeper: false,
            held_id: SHOVEL,
            grave_id: GRAVE_88,
            grave_x: 20,
            grave_y: 0,
            home_x: 0,
            home_y: 0,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s2, &mut gk2, 1),
            HandlingGravesAction::UseHeldOnGrave { x: 20, y: 0 }
        );
        assert!(gk2.is_last_grave_keeper);
    }

    #[test]
    fn close_to_home_gets_basket() {
        let mut gk = rt();
        gk.is_last_grave_keeper = true;
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 2,
            grave_y: 0,
            home_x: 0,
            home_y: 0,
            player_x: 0,
            player_y: 0,
            held_id: 0,
            ..Default::default()
        };
        // home-grave quad = 4 < 30 → pickup → GetOrCraft basket (home player quad 0 < 900)
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::GetOrCraft { object_id: BASKET }
        );
    }

    #[test]
    fn held_basket_uses_on_grave() {
        let mut gk = rt();
        gk.is_last_grave_keeper = true;
        let s = HandlingGravesSensors {
            held_id: BASKET,
            grave_id: GRAVE_88,
            grave_x: 2,
            grave_y: 0,
            home_x: 0,
            home_y: 0,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::UseHeldOnGrave { x: 2, y: 0 }
        );
    }

    #[test]
    fn pick_grave_prefers_sticky() {
        let map = [
            HandlingGravesMapObj {
                parent_id: GRAVE_OLD,
                x: 0,
                y: 0,
                floor_id: 0,
                contained_count: 0,
                owner_account: 0,
            },
            HandlingGravesMapObj {
                parent_id: GRAVE_88,
                x: 5,
                y: 0,
                floor_id: 0,
                contained_count: 0,
                owner_account: 0,
            },
        ];
        let g = pick_grave(&map, 0, 0, 30, Some((GRAVE_88, 5, 0))).unwrap();
        assert_eq!(g.0, GRAVE_88);
        assert_eq!((g.1, g.2), (5, 0));
        let near = pick_grave(&map, 0, 0, 30, None).unwrap();
        assert_eq!(near.0, GRAVE_OLD);
        // Haxe: AiBase L1487 lastGrave.parentId not in graveIdsToDigIn → search
        let skip_sticky = pick_grave(&map, 0, 0, 30, Some((MARKED_GRAVE, 5, 0))).unwrap();
        assert_eq!(skip_sticky.0, GRAVE_OLD);
    }

    #[test]
    fn get_graveyard_marked_then_buried_quad_min() {
        // Haxe L3634–3641: 1012 then 1011, home r=25, minDistance quad ≥ 8²
        let close = [HandlingGravesMapObj {
            parent_id: MARKED_GRAVE,
            x: 5,
            y: 0,
            floor_id: 0,
            contained_count: 0,
            owner_account: 0,
        }];
        assert!(get_graveyard(&close, 0, 0).is_none(), "quad 25 < 64");
        let diag = [HandlingGravesMapObj {
            parent_id: MARKED_GRAVE,
            x: 6,
            y: 6,
            floor_id: 0,
            contained_count: 0,
            owner_account: 0,
        }];
        let g = get_graveyard(&diag, 0, 0).unwrap();
        assert_eq!(g, (MARKED_GRAVE, 6, 6), "quad 72 ≥ 64 even if cheb 6 < 8");
        let buried = [HandlingGravesMapObj {
            parent_id: BURIED_GRAVE,
            x: 10,
            y: 0,
            floor_id: 0,
            contained_count: 0,
            owner_account: 0,
        }];
        assert_eq!(get_graveyard(&buried, 0, 0), Some((BURIED_GRAVE, 10, 0)));
        assert_eq!(GRAVEYARD_HOME_RADIUS, 25);
        assert_eq!(GRAVEYARD_HOME_MIN, 8);
    }

    #[test]
    fn unreachable_or_missing_grave_clears_last_grave() {
        // Haxe: AiBase L1491–1499 lastGrave=null if missing/unreachable; else lastGrave=grave
        let mut gk = rt();
        gk.set_last_grave(GRAVE_88, 3, 0);
        let s = HandlingGravesSensors {
            grave_id: 0,
            ..Default::default()
        };
        assert_eq!(is_handling_graves(&s, &mut gk, 1), HandlingGravesAction::None);
        assert_eq!(gk.last_grave_id, 0);
        gk.set_last_grave(GRAVE_88, 3, 0);
        let s2 = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 3,
            grave_y: 0,
            grave_reachable: false,
            ..Default::default()
        };
        assert_eq!(is_handling_graves(&s2, &mut gk, 1), HandlingGravesAction::None);
        assert_eq!(gk.last_grave_id, 0);
        let s3 = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 3,
            grave_y: 0,
            grave_reachable: true,
            grave_hostile_path: false,
            ..Default::default()
        };
        let _ = is_handling_graves(&s3, &mut gk, 1);
        assert_eq!(gk.last_grave_id, GRAVE_88);
        assert_eq!((gk.last_grave_x, gk.last_grave_y), (3, 0));
    }

    #[test]
    fn own_grave_from_first_account_owner() {
        // Haxe: AiBase L1503–1508 getOwnerAccount()[0] == myPlayer.account.id
        assert!(is_own_grave_account(&[7, 9], 7));
        assert!(!is_own_grave_account(&[9, 7], 7));
        assert!(!is_own_grave_account(&[], 7));
        assert!(!is_own_grave_account(&[0], 7));
        let map = [HandlingGravesMapObj {
            parent_id: GRAVE_88,
            x: 1,
            y: 0,
            floor_id: 0,
            contained_count: 0,
            owner_account: 42,
        }];
        let s = handling_graves_sensors_from_map(
            &map, 0, 0, 0, 0, 0, 0, 20.0, false, true, false, true, 42, None,
        );
        assert!(s.is_own_grave);
        let s2 = handling_graves_sensors_from_map(
            &map, 0, 0, 0, 0, 0, 0, 20.0, false, true, false, true, 1, None,
        );
        assert!(!s2.is_own_grave);
    }

    #[test]
    fn pickup_bones_only_within_20_of_home() {
        // Haxe: PickupItem(356) r=20 from home
        let near = [HandlingGravesMapObj {
            parent_id: BASKET_OF_BONES,
            x: 20,
            y: 0,
            floor_id: 0,
            contained_count: 0,
            owner_account: 0,
        }];
        let far = [HandlingGravesMapObj {
            parent_id: BASKET_OF_BONES,
            x: 21,
            y: 0,
            floor_id: 0,
            contained_count: 0,
            owner_account: 0,
        }];
        let s_near = handling_graves_sensors_from_map(
            &near, 0, 0, 0, 0, 0, 0, 20.0, false, true, false, true, 0, None,
        );
        let s_far = handling_graves_sensors_from_map(
            &far, 0, 0, 0, 0, 0, 0, 20.0, false, true, false, true, 0, None,
        );
        assert!(s_near.has_basket_of_bones);
        assert!(!s_far.has_basket_of_bones);
    }

    #[test]
    fn old_ai_included_in_grave_keeper_pick() {
        let peers = [
            GraveKeeperPeer {
                p_id: 1,
                quad_dist_to_obj: 4.0,
                deleted: false,
                age: 119.0,
                is_wounded: false,
                food_store: 5.0,
                same_home: true,
                has_grave_keeper: true,
            },
            GraveKeeperPeer {
                p_id: 2,
                quad_dist_to_obj: 1.0,
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                same_home: true,
                has_grave_keeper: true,
            },
        ];
        assert!(is_self_best_grave_keeper_for_obj(2, &peers, 3.0));
        assert!(!is_self_best_grave_keeper_for_obj(1, &peers, 3.0));
        // old self with weight still eligible (FIREKEEPER would skip age>max-2)
        let only_old = [peers[0]];
        assert!(is_self_best_grave_keeper_for_obj(1, &only_old, 3.0));
    }

    #[test]
    fn closer_weight_wins_grave_keeper_pick() {
        let peers = [
            GraveKeeperPeer {
                p_id: 1,
                quad_dist_to_obj: 25.0,
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                same_home: true,
                has_grave_keeper: false,
            },
            GraveKeeperPeer {
                p_id: 2,
                quad_dist_to_obj: 4.0,
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                same_home: true,
                has_grave_keeper: true,
            },
        ];
        assert!(!is_self_best_grave_keeper_for_obj(1, &peers, 3.0));
        assert!(is_self_best_grave_keeper_for_obj(2, &peers, 3.0));
    }

    #[test]
    fn shovel_near_gets_item_else_hoe() {
        let mut gk = rt();
        gk.is_last_grave_keeper = true;
        // grave-home quad 400 >= 30 → not pickup; empty hands
        let s = HandlingGravesSensors {
            grave_id: GRAVE_88,
            grave_x: 20,
            grave_y: 0,
            home_x: 0,
            home_y: 0,
            player_x: 0,
            player_y: 0,
            held_id: 0,
            has_shovel_near_grave: true,
            ..Default::default()
        };
        assert_eq!(
            is_handling_graves(&s, &mut gk, 1),
            HandlingGravesAction::GetItem { object_id: SHOVEL }
        );
        let s2 = HandlingGravesSensors {
            has_shovel_near_grave: false,
            ..s
        };
        assert_eq!(
            is_handling_graves(&s2, &mut gk, 1),
            HandlingGravesAction::GetOrCraft {
                object_id: STONE_HOE
            }
        );
        let s3 = HandlingGravesSensors {
            player_x: 40,
            player_y: 0,
            has_shovel_near_grave: false,
            ..s
        };
        assert_eq!(
            is_handling_graves(&s3, &mut gk, 1),
            HandlingGravesAction::GetItem {
                object_id: STONE_HOE
            }
        );
        assert_eq!(
            handling_graves_action_to_goal(HandlingGravesAction::GetItem { object_id: SHOVEL }),
            Goal::SeekObject(SHOVEL)
        );
        assert_eq!(GRAVE_KEEPER_PROFESSION_KEY, "GRAVEKEEPER");
    }
}
