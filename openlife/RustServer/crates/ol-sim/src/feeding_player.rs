//! Haxe `AiBase.isFeedingPlayerInNeed` (chunk **AI-JOB-FOODSERVER**).
//!
//! Assigned/last `FOODSERVER` → `isFeedingPlayerInNeed(100)`. Sticky
//! `feedingPlayerTarget` + `GetCloseStarvingPlayer` (r=30), then feed held food
//! or `SearchBestFood(target, self)`. Mid `isFeedingPlayerInNeed()` max=1
//! (`doStuff && profession['SMITH'] < 1`) is **AI-FEED-MID**.

use crate::feed::MAX_CHILD_AGE_BREAST_FEEDING;
use crate::move_live_gates::COMBAT_ANGRY_TIME_BEFORE_ATTACK;
use crate::MIN_AGE_TO_EAT;
use ol_player_helper::food_eat_gates::PSILOCYBE_MUSHROOM_ID;

/// Canonical Haxe profession string.
pub const FOODSERVER_PROFESSION_KEY: &str = "FOODSERVER";
/// Assigned/last `isFeedingPlayerInNeed(100)`.
pub const FOODSERVER_ASSIGNED_MAX: i32 = 100;
/// Default `isFeedingPlayerInNeed()` maxPlayer.
pub const FOODSERVER_DEFAULT_MAX: i32 = 1;
/// Feeder `food_store < 2` refuse.
pub const FOODSERVER_MIN_FOOD: f32 = 2.0;
/// `GetCloseStarvingPlayer` default searchDistance.
pub const STARVING_SEARCH_DIST: i32 = 30;
/// Haxe `quadDist > 10` → feed to 40% else 80%.
pub const FEED_FULL_FAR_QUAD: f32 = 10.0;
pub const FEED_FULL_FAR_FRAC: f32 = 0.4;
pub const FEED_FULL_NEAR_FRAC: f32 = 0.8;
/// Goto when player-quad > 1 (same as feed isClose).
pub const FEED_GOTO_QUAD: i32 = 1;
/// Keep moving when already moving and quad > 10.
pub const FEED_KEEP_MOVING_QUAD: i32 = 10;
/// `GetCloseStarvingPlayer` minQuadHungry.
pub const MIN_QUAD_HUNGRY: f32 = 0.01;
/// SearchBestFood default radius when held is not feedable.
pub const FOODSERVER_FOOD_SEARCH_RADIUS: i32 = 40;
/// `targetPlayer.age > 1.5` for StartingName `You are`.
// Haxe: AiBase.isFeedingPlayerInNeed L6385
pub const FEED_NAME_MIN_AGE: f32 = 1.5;
/// `Math.random() < 0.2` random name vs feeder name.
// Haxe: AiBase.isFeedingPlayerInNeed L6387
pub const FEED_NAME_RANDOM_CHANCE: f32 = 0.2;
/// `this.time += 2` after `doOnOther`.
// Haxe: AiBase.isFeedingPlayerInNeed L6394
pub const FEED_WAIT_SECS: f32 = 2.0;

/// Sticky last + assigned + weight + feedingPlayerTarget.
// Haxe: profession['FOODSERVER'] + feedingPlayerTarget
#[derive(Debug, Clone, PartialEq)]
pub struct FoodServerProfessionRuntime {
    pub is_last_foodserver: bool,
    pub is_assigned_foodserver: bool,
    pub weight: f32,
    /// Sticky `feedingPlayerTarget` p_id (0 = none).
    pub feeding_target_p_id: i32,
}

impl Default for FoodServerProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_foodserver: false,
            is_assigned_foodserver: false,
            weight: 0.0,
            feeding_target_p_id: 0,
        }
    }
}

impl FoodServerProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        self.feeding_target_p_id = 0;
        if !last_was_foodserver {
            self.is_last_foodserver = false;
        }
    }

    pub fn clear_target(&mut self) {
        self.feeding_target_p_id = 0;
    }
}

pub fn parse_foodserver_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("FOODSERVER") || prof.eq_ignore_ascii_case("FOOD")
}

pub fn assign_foodserver_from_speech(
    runtime: &mut FoodServerProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_foodserver_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_foodserver = true;
    runtime.is_last_foodserver = true;
    runtime.weight = 1.0;
    true
}

pub fn resolve_foodserver_assigned_job(runtime: &FoodServerProfessionRuntime) -> bool {
    runtime.is_assigned_foodserver || runtime.is_last_foodserver
}

pub fn has_or_become_foodserver(
    runtime: &mut FoodServerProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_foodserver {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count_with_last.max(0.0) >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_foodserver = true;
    true
}

/// Nearby player snapshot for `GetCloseStarvingPlayer` / feed gates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarvingCand {
    pub conn_id: u64,
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub age: f32,
    pub food: f32,
    pub food_max: f32,
    pub deleted: bool,
    pub held_by: bool,
    pub is_ai: bool,
    pub is_wounded: bool,
    pub has_yellow_fever: bool,
    pub is_smith: bool,
    pub prestige_class: i32,
    pub is_ally: bool,
    pub is_close_relative: bool,
    pub is_follow_target: bool,
    pub angry_time: f32,
    pub lost_combat_prestige: f32,
    /// `target.canFeedToMe(held)` for the feeder's current held object.
    pub can_feed_held: bool,
    /// First name is `ServerSettings.StartingName` (YOU ARE gate).
    // Haxe: AiBase.isFeedingPlayerInNeed L6385
    pub is_starting_name: bool,
    pub is_female: bool,
}

impl Default for StarvingCand {
    fn default() -> Self {
        Self {
            conn_id: 0,
            p_id: 0,
            x: 0,
            y: 0,
            age: 20.0,
            food: 0.0,
            food_max: 20.0,
            deleted: false,
            held_by: false,
            is_ai: false,
            is_wounded: false,
            has_yellow_fever: false,
            is_smith: false,
            prestige_class: 2,
            is_ally: true,
            is_close_relative: false,
            is_follow_target: false,
            angry_time: COMBAT_ANGRY_TIME_BEFORE_ATTACK,
            lost_combat_prestige: 0.0,
            can_feed_held: true,
            is_starting_name: false,
            is_female: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeedingPlayerSensors {
    pub player_x: i32,
    pub player_y: i32,
    pub age: f32,
    pub food: f32,
    pub min_age_to_eat: f32,
    pub is_moving: bool,
    pub is_fertile: bool,
    pub is_smith: bool,
    pub held_id: i32,
    pub held_food_value: i32,
    pub self_p_id: i32,
}

impl FeedingPlayerSensors {
    pub fn basic(x: i32, y: i32, held_id: i32, held_food_value: i32) -> Self {
        Self {
            player_x: x,
            player_y: y,
            age: 20.0,
            food: 10.0,
            min_age_to_eat: MIN_AGE_TO_EAT,
            is_moving: false,
            is_fertile: false,
            is_smith: false,
            held_id,
            held_food_value,
            self_p_id: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedingPlayerAction {
    None,
    /// Held not feedable — live `SearchBestFood(target, self)`.
    SeekFood { target_p_id: i32, target_conn: u64 },
    Goto { x: i32, y: i32 },
    Wait,
    ForceStopWait,
    Feed { target_p_id: i32, target_conn: u64 },
}

impl FeedingPlayerAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn player_quad(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Haxe `You are $newName` when target is StartingName and age > 1.5.
/// Eve/Adam or 20% roll uses `random_name`; else feeder first name.
// Haxe: AiBase.isFeedingPlayerInNeed L6385–6389
pub fn feed_you_are_say(
    target_is_starting: bool,
    target_age: f32,
    feeder_is_eve_or_adam: bool,
    feeder_first_name: &str,
    random_name: &str,
    roll_random: bool,
) -> Option<String> {
    if !target_is_starting || !(target_age > FEED_NAME_MIN_AGE) {
        return None;
    }
    let name = if feeder_is_eve_or_adam || roll_random {
        random_name
    } else if !feeder_first_name.is_empty() {
        feeder_first_name
    } else {
        random_name
    };
    Some(format!("You are {name}"))
}

fn is_noble_or_more(class: i32) -> bool {
    class >= 3
}

/// Haxe `GetCloseStarvingPlayerHelper`.
// Haxe: AiHelper.GetCloseStarvingPlayerHelper ~1952
pub fn pick_close_starving_player(
    sensors: &FeedingPlayerSensors,
    cands: &[StarvingCand],
    search_dist: i32,
) -> Option<StarvingCand> {
    let max_dist = (search_dist.max(0) * search_dist.max(0)) as f32;
    let angry_half = COMBAT_ANGRY_TIME_BEFORE_ATTACK / 2.0;
    let mut best: Option<(f32, StarvingCand)> = None;
    for &p in cands {
        if p.p_id == sensors.self_p_id {
            continue;
        }
        if p.deleted || p.held_by {
            continue;
        }
        if sensors.is_fertile && p.age < MAX_CHILD_AGE_BREAST_FEEDING {
            continue;
        }
        if p.is_ai
            && p.age > sensors.min_age_to_eat
            && !p.is_wounded
            && !p.has_yellow_fever
            && !p.is_smith
        {
            continue;
        }
        let class_food = if is_noble_or_more(p.prestige_class) {
            p.prestige_class as f32 * 4.0
        } else {
            p.prestige_class as f32 * 2.0
        };
        let consider = class_food.min(p.food_max * 0.6);
        let mut hungry = consider - p.food;
        if !p.is_ally && p.angry_time < angry_half {
            continue;
        }
        if !p.is_ally && p.lost_combat_prestige > 4.0 {
            continue;
        }
        if !p.is_close_relative || p.is_follow_target {
            hungry = hungry / 2.0 - 0.25;
        }
        if !p.is_ally {
            hungry = hungry / 2.0 - 0.2;
        }
        if sensors.is_smith {
            hungry = hungry / 2.0 - 0.2;
        }
        if hungry < 0.0 {
            continue;
        }
        let dist = player_quad(sensors.player_x, sensors.player_y, p.x, p.y) as f32 + 1.0;
        if dist > max_dist {
            continue;
        }
        let quad_hungry = (hungry * hungry) / dist;
        if quad_hungry < MIN_QUAD_HUNGRY {
            continue;
        }
        match best {
            None => best = Some((quad_hungry, p)),
            Some((bq, _)) if quad_hungry > bq => best = Some((quad_hungry, p)),
            _ => {}
        }
    }
    best.map(|(_, p)| p)
}

fn cand_by_pid(cands: &[StarvingCand], p_id: i32) -> Option<StarvingCand> {
    cands.iter().copied().find(|c| c.p_id == p_id)
}

fn held_not_feedable(sensors: &FeedingPlayerSensors, target: &StarvingCand) -> bool {
    sensors.held_food_value < 1
        || sensors.held_id == PSILOCYBE_MUSHROOM_ID
        || !target.can_feed_held
}

/// Pure `isFeedingPlayerInNeed(maxPlayer)` after age/food gates.
// Haxe: AiBase.isFeedingPlayerInNeed ~6296
pub fn is_feeding_player_in_need(
    sensors: &FeedingPlayerSensors,
    rt: &mut FoodServerProfessionRuntime,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
    cands: &[StarvingCand],
) -> FeedingPlayerAction {
    if sensors.age < sensors.min_age_to_eat {
        return FeedingPlayerAction::None;
    }
    if sensors.food < FOODSERVER_MIN_FOOD {
        return FeedingPlayerAction::None;
    }

    if rt.feeding_target_p_id == 0 {
        rt.weight = 0.0;
        if let Some(p) = pick_close_starving_player(sensors, cands, STARVING_SEARCH_DIST) {
            rt.feeding_target_p_id = p.p_id;
        }
    }
    if rt.feeding_target_p_id == 0 {
        return FeedingPlayerAction::None;
    }

    let Some(target) = cand_by_pid(cands, rt.feeding_target_p_id) else {
        rt.clear_target();
        return FeedingPlayerAction::None;
    };
    let quad = player_quad(sensors.player_x, sensors.player_y, target.x, target.y);
    let full_frac = if quad as f32 > FEED_FULL_FAR_QUAD {
        FEED_FULL_FAR_FRAC
    } else {
        FEED_FULL_NEAR_FRAC
    };
    if target.food > target.food_max * full_frac {
        rt.clear_target();
        return FeedingPlayerAction::None;
    }
    if target.deleted || target.held_by {
        rt.clear_target();
        return FeedingPlayerAction::None;
    }

    if !has_or_become_foodserver(rt, max_people, peer_count, was_idle) {
        return FeedingPlayerAction::None;
    }

    if held_not_feedable(sensors, &target) {
        return FeedingPlayerAction::SeekFood {
            target_p_id: target.p_id,
            target_conn: target.conn_id,
        };
    }

    if quad > FEED_KEEP_MOVING_QUAD && sensors.is_moving {
        return FeedingPlayerAction::Wait;
    }
    if quad > FEED_GOTO_QUAD {
        if sensors.is_moving {
            return FeedingPlayerAction::ForceStopWait;
        }
        return FeedingPlayerAction::Goto {
            x: target.x,
            y: target.y,
        };
    }
    FeedingPlayerAction::Feed {
        target_p_id: target.p_id,
        target_conn: target.conn_id,
    }
}

/// Haxe `profession['SMITH'] < 1` gate on mid `isFeedingPlayerInNeed()`.
// Haxe: AiBase.doTimeStuffHelper ~594
#[inline]
pub fn smith_blocks_mid_feed(smith_stage: f32) -> bool {
    smith_stage >= 1.0
}

/// True when mid feed would have a starving target (age/food already gated).
pub fn mid_feed_has_starving_target(
    sensors: &FeedingPlayerSensors,
    rt: &FoodServerProfessionRuntime,
    cands: &[StarvingCand],
) -> bool {
    if sensors.age < sensors.min_age_to_eat || sensors.food < FOODSERVER_MIN_FOOD {
        return false;
    }
    if rt.feeding_target_p_id != 0 && cand_by_pid(cands, rt.feeding_target_p_id).is_some() {
        return true;
    }
    pick_close_starving_player(sensors, cands, STARVING_SEARCH_DIST).is_some()
}

/// Sensor pair for [`PriorityRung::FeedPlayerInNeed`].
// Haxe: doStuff && profession['SMITH'] < 1 && isFeedingPlayerInNeed()
pub fn mid_feed_sensor_flags(
    sensors: &FeedingPlayerSensors,
    rt: &FoodServerProfessionRuntime,
    smith_stage: f32,
    cands: &[StarvingCand],
) -> (bool, bool) {
    let smith_blocks = smith_blocks_mid_feed(smith_stage);
    let need = mid_feed_has_starving_target(sensors, rt, cands);
    (need, smith_blocks)
}

/// Assigned `isFeedingPlayerInNeed(100)`; mid `FEED_PLAYER_IN_NEED` always max=1.
// Haxe: assigned ~739 max=100; mid ~594 default maxPlayer=1
pub fn foodserver_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if rung_label == "FEED_PLAYER_IN_NEED" {
        FOODSERVER_DEFAULT_MAX
    } else if is_assigned || rung_label == "ASSIGNED_JOB" {
        FOODSERVER_ASSIGNED_MAX
    } else {
        FOODSERVER_DEFAULT_MAX
    }
}

#[inline]
pub fn foodserver_job_rung_label(rung_label: &str) -> bool {
    matches!(rung_label, "ASSIGNED_JOB" | "FEED_PLAYER_IN_NEED")
}

pub fn try_decide_feeding_player_from_rung(
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &FeedingPlayerSensors,
    rt: &mut FoodServerProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
    cands: &[StarvingCand],
) -> Option<FeedingPlayerAction> {
    if !foodserver_job_rung_label(rung_label) {
        return None;
    }
    // Mid slot is always max=1 even if last/assigned FOODSERVER (Haxe ~594).
    let assigned_job = is_assigned_job || rung_label == "ASSIGNED_JOB";
    let max = foodserver_max_for_dispatch(assigned_job, rung_label);
    Some(is_feeding_player_in_need(
        sensors, rt, max, peer_count, was_idle, cands,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hungry_human_near() -> StarvingCand {
        StarvingCand {
            conn_id: 2,
            p_id: 2,
            x: 1,
            y: 0,
            food: 0.0,
            food_max: 20.0,
            prestige_class: 2,
            is_ai: false,
            is_ally: true,
            can_feed_held: true,
            ..Default::default()
        }
    }

    #[test]
    fn feed_you_are_starting_name_age_gate() {
        assert!(feed_you_are_say(false, 20.0, false, "LISA", "ALICE", false).is_none());
        assert!(feed_you_are_say(true, 1.5, false, "LISA", "ALICE", false).is_none());
        assert_eq!(
            feed_you_are_say(true, 1.51, false, "LISA", "ALICE", false).as_deref(),
            Some("You are LISA")
        );
        assert_eq!(
            feed_you_are_say(true, 20.0, true, "LISA", "ALICE", false).as_deref(),
            Some("You are ALICE")
        );
        assert_eq!(
            feed_you_are_say(true, 20.0, false, "LISA", "ALICE", true).as_deref(),
            Some("You are ALICE")
        );
    }

    #[test]
    fn age_and_food_gates() {
        let mut rt = FoodServerProfessionRuntime::default();
        let mut s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        s.age = 2.0;
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]),
            FeedingPlayerAction::None
        );
        s.age = 20.0;
        s.food = 1.0;
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]),
            FeedingPlayerAction::None
        );
        // Haxe L6298: food_store < 2 refuses; 2.0 is allowed
        s.food = 2.0;
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]);
        assert!(a.is_some());
    }

    #[test]
    fn assigned_feeds_adjacent_when_holding_food() {
        let mut rt = FoodServerProfessionRuntime {
            is_assigned_foodserver: true,
            is_last_foodserver: true,
            weight: 1.0,
            feeding_target_p_id: 0,
        };
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]);
        assert_eq!(
            a,
            FeedingPlayerAction::Feed {
                target_p_id: 2,
                target_conn: 2
            }
        );
        assert_eq!(rt.feeding_target_p_id, 2);
        assert!(rt.is_last_foodserver);
    }

    #[test]
    fn goto_when_farther_than_quad_one() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let mut t = hungry_human_near();
        t.x = 3;
        t.y = 0;
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[t]);
        assert_eq!(a, FeedingPlayerAction::Goto { x: 3, y: 0 });
    }

    #[test]
    fn seek_food_when_held_not_food() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, 0, 0);
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]);
        assert_eq!(
            a,
            FeedingPlayerAction::SeekFood {
                target_p_id: 2,
                target_conn: 2
            }
        );
    }

    #[test]
    fn psilocybe_held_seeks_other_food() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, PSILOCYBE_MUSHROOM_ID, 5);
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[hungry_human_near()]);
        assert!(matches!(a, FeedingPlayerAction::SeekFood { .. }));
    }

    #[test]
    fn skips_full_target_and_clears_sticky() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            feeding_target_p_id: 2,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let mut t = hungry_human_near();
        t.food = 19.0; // > 20 * 0.8
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[t]),
            FeedingPlayerAction::None
        );
        assert_eq!(rt.feeding_target_p_id, 0);
    }

    #[test]
    fn pick_skips_healthy_ai_that_can_eat() {
        let mut rt = FoodServerProfessionRuntime::default();
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let ai = StarvingCand {
            conn_id: 3,
            p_id: 3,
            x: 1,
            y: 0,
            is_ai: true,
            age: 20.0,
            food: 0.0,
            food_max: 20.0,
            ..Default::default()
        };
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[ai]),
            FeedingPlayerAction::None
        );
    }

    #[test]
    fn pick_includes_wounded_ai() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let ai = StarvingCand {
            conn_id: 3,
            p_id: 3,
            x: 1,
            y: 0,
            is_ai: true,
            is_wounded: true,
            age: 20.0,
            food: 0.0,
            food_max: 20.0,
            can_feed_held: true,
            ..Default::default()
        };
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[ai]),
            FeedingPlayerAction::Feed {
                target_p_id: 3,
                target_conn: 3
            }
        );
    }

    #[test]
    fn assigned_max_ignores_small_peer_count() {
        let mut rt = FoodServerProfessionRuntime::default();
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let a = is_feeding_player_in_need(&s, &mut rt, 100, 3.0, 0.0, &[hungry_human_near()]);
        assert!(a.is_some());
        let mut rt2 = FoodServerProfessionRuntime::default();
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt2, 1, 1.0, 0.0, &[hungry_human_near()]),
            FeedingPlayerAction::None
        );
    }

    #[test]
    fn no_target_zeros_weight() {
        let mut rt = FoodServerProfessionRuntime {
            weight: 1.0,
            ..Default::default()
        };
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[]),
            FeedingPlayerAction::None
        );
        assert_eq!(rt.weight, 0.0);
        assert_eq!(rt.feeding_target_p_id, 0);
    }

    #[test]
    fn speech_and_max_dispatch() {
        let mut r = FoodServerProfessionRuntime::default();
        assert!(parse_foodserver_profession_speech("FOODSERVER!"));
        assert!(parse_foodserver_profession_speech("food"));
        assert!(!parse_foodserver_profession_speech("BAKER!"));
        assert!(assign_foodserver_from_speech(&mut r, "FOODSERVER!"));
        assert!(resolve_foodserver_assigned_job(&r));
        assert_eq!(foodserver_max_for_dispatch(true, "ASSIGNED_JOB"), 100);
        assert_eq!(foodserver_max_for_dispatch(false, "LOW_PRIORITY_WORK"), 1);
        assert_eq!(foodserver_max_for_dispatch(true, "FEED_PLAYER_IN_NEED"), 1);
        assert_eq!(foodserver_max_for_dispatch(false, "FEED_PLAYER_IN_NEED"), 1);
        assert!(foodserver_job_rung_label("FEED_PLAYER_IN_NEED"));
        assert!(try_decide_feeding_player_from_rung(
            "ESCAPE",
            true,
            &FeedingPlayerSensors::basic(0, 0, 31, 5),
            &mut r,
            0.0,
            0.0,
            &[],
        )
        .is_none());
    }

    #[test]
    fn mid_feed_max1_peer_cap_and_smith_gate() {
        // Haxe mid isFeedingPlayerInNeed() max=1; profession['SMITH'] < 1
        let mut rt = FoodServerProfessionRuntime::default();
        let s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        let cands = [hungry_human_near()];
        let a = try_decide_feeding_player_from_rung(
            "FEED_PLAYER_IN_NEED",
            false,
            &s,
            &mut rt,
            0.0,
            1.0,
            &cands,
        );
        assert!(matches!(
            a,
            Some(FeedingPlayerAction::Feed { target_p_id: 2, .. })
        ));
        assert!(rt.is_last_foodserver);

        let mut rt2 = FoodServerProfessionRuntime::default();
        let blocked = try_decide_feeding_player_from_rung(
            "FEED_PLAYER_IN_NEED",
            false,
            &s,
            &mut rt2,
            1.0,
            0.0,
            &cands,
        );
        assert_eq!(blocked, Some(FeedingPlayerAction::None));
        assert!(!rt2.is_last_foodserver);

        let mut last = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            ..Default::default()
        };
        let last_ok = try_decide_feeding_player_from_rung(
            "FEED_PLAYER_IN_NEED",
            true,
            &s,
            &mut last,
            5.0,
            0.0,
            &cands,
        );
        assert!(matches!(last_ok, Some(FeedingPlayerAction::Feed { .. })));

        assert!(!smith_blocks_mid_feed(0.0));
        assert!(smith_blocks_mid_feed(1.0));
        assert!(smith_blocks_mid_feed(1.5));
        let (need, block) = mid_feed_sensor_flags(&s, &FoodServerProfessionRuntime::default(), 0.0, &cands);
        assert!(need && !block);
        let (need2, block2) =
            mid_feed_sensor_flags(&s, &FoodServerProfessionRuntime::default(), 2.0, &cands);
        assert!(need2 && block2);
        let empty = mid_feed_sensor_flags(
            &s,
            &FoodServerProfessionRuntime::default(),
            0.0,
            &[],
        );
        assert!(!empty.0);
    }

    #[test]
    fn wait_when_moving_and_far() {
        let mut rt = FoodServerProfessionRuntime {
            is_last_foodserver: true,
            feeding_target_p_id: 2,
            ..Default::default()
        };
        let mut s = FeedingPlayerSensors::basic(0, 0, 31, 5);
        s.is_moving = true;
        let mut t = hungry_human_near();
        t.x = 5;
        t.y = 0; // quad 25 > 10
        assert_eq!(
            is_feeding_player_in_need(&s, &mut rt, 100, 0.0, 0.0, &[t]),
            FeedingPlayerAction::Wait
        );
    }
}
