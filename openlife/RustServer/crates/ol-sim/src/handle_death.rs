//! Haxe `AiBase.handleDeath` (chunk **AI-HANDLE-DEATH**).
//!
//! Age ≥ MaxAge−2 → wipe professions, last GRAVEKEEPER, then graves if near home,
//! else `isMovingToHome(5)`, else drop held + time+=2.
//!
//! No world I/O: live apply maps [`HandleDeathAction`] to scan/Goto/DROP.

use crate::ai_goals::priority_ladder::{
    should_handle_death, HANDLE_DEATH_AGE_OFFSET, HANDLE_DEATH_MAX_AGE_DEFAULT,
};
use crate::GRAVE_KEEPER_PROFESSION_KEY;

/// Quad dist to home for nested `isHandlingGraves`.
// Haxe: AiBase.handleDeath ~1632
pub const HANDLE_DEATH_HOME_GRAVE_QUAD: i32 = 400;
/// Haxe `isMovingToHome(5)` tile cap (then squared).
// Haxe: AiBase.handleDeath ~1633
pub const HANDLE_DEATH_GO_HOME_TILES: i32 = 5;
/// Haxe `this.time += 2` after graves/home fail.
pub const HANDLE_DEATH_TIME_BUMP: f32 = 2.0;
pub const HANDLE_DEATH_SAY_GOODBYE_P: f32 = 0.05;
pub const HANDLE_DEATH_SAY_JASONIAH_P: f32 = 0.10;
pub const HANDLE_DEATH_SAY_GOODBYE: &str = "Good bye!";
pub const HANDLE_DEATH_SAY_JASONIAH: &str = "Jasoniah is calling me. Take care!";

/// Pure planner output after the age gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleDeathAction {
    /// Age below MaxAge−2 — Haxe returns false (other rungs run).
    NotOld,
    /// Removing / using / moving — tick consumed, professions already wiped.
    Busy,
    /// `quadDist(home) < 400` → try `isHandlingGraves`; on miss, go home or drop.
    GravesThenRest,
    /// Far from fire/home — `isMovingToHome(5)`.
    GoHome,
    /// Already close; `time += 2` then `dropHeldObject(0)`.
    DropHeld,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleDeathPlan {
    pub action: HandleDeathAction,
    pub say: Option<&'static str>,
    pub time_bump: f32,
}

impl HandleDeathPlan {
    pub fn none() -> Self {
        Self {
            action: HandleDeathAction::NotOld,
            say: None,
            time_bump: 0.0,
        }
    }
}

/// Haxe `WorldMap.calculateRandomFloat` say roll.
// Haxe: handleDeath ~1622–1624
pub fn handle_death_say(rand: f32) -> Option<&'static str> {
    if rand < HANDLE_DEATH_SAY_GOODBYE_P {
        Some(HANDLE_DEATH_SAY_GOODBYE)
    } else if rand < HANDLE_DEATH_SAY_JASONIAH_P {
        Some(HANDLE_DEATH_SAY_JASONIAH)
    } else {
        None
    }
}

/// Haxe `isMovingToHome(5)`: `maxDistance = 5*5`; path when quad ≥ 25.
#[inline]
pub fn handle_death_should_go_home(move_target_quad: i32) -> bool {
    let mt = HANDLE_DEATH_GO_HOME_TILES.max(0);
    move_target_quad >= mt * mt
}

/// Wipe job runtimes and assign last GRAVEKEEPER (keep lastGrave).
// Haxe: profession = new Map; profession['GRAVEKEEPER']=1; lastProfession
pub fn wipe_jobs_assign_grave_keeper(
    farm: &mut crate::FarmProfessionRuntime,
    smith: &mut crate::SmithProfessionRuntime,
    baker: &mut crate::BakerProfessionRuntime,
    shepherd: &mut crate::ShepherdProfessionRuntime,
    pottery: &mut crate::PotterProfessionRuntime,
    fire_food: &mut crate::FireFoodProfessionRuntime,
    fire_keeper: &mut crate::FireKeeperProfessionRuntime,
    grave_keeper: &mut crate::GraveKeeperProfessionRuntime,
    hunter: &mut crate::HunterProfessionRuntime,
    lumberjack: &mut crate::LumberjackProfessionRuntime,
    collector: &mut crate::CollectorProfessionRuntime,
    foodserver: &mut crate::FoodServerProfessionRuntime,
) {
    let last_id = grave_keeper.last_grave_id;
    let last_x = grave_keeper.last_grave_x;
    let last_y = grave_keeper.last_grave_y;
    *farm = Default::default();
    *smith = Default::default();
    *baker = Default::default();
    *shepherd = Default::default();
    *pottery = Default::default();
    *fire_food = Default::default();
    *fire_keeper = Default::default();
    *grave_keeper = Default::default();
    *hunter = Default::default();
    *lumberjack = Default::default();
    *collector = Default::default();
    *foodserver = Default::default();
    grave_keeper.is_last_grave_keeper = true;
    grave_keeper.weight = 1.0;
    if last_id != 0 {
        grave_keeper.set_last_grave(last_id, last_x, last_y);
    }
}

/// Full pure `handleDeath` body after age gate (wipe is a live side effect).
// Haxe: AiBase.handleDeath ~1611–1642
pub fn plan_handle_death(
    age: f32,
    max_age: f32,
    is_removing_container: bool,
    is_using_item: bool,
    is_moving: bool,
    home_quad: i32,
    move_target_quad: i32,
    rand: f32,
) -> HandleDeathPlan {
    if !should_handle_death(age, max_age) {
        return HandleDeathPlan::none();
    }
    if is_removing_container || is_using_item {
        return HandleDeathPlan {
            action: HandleDeathAction::Busy,
            say: None,
            time_bump: 0.0,
        };
    }
    let say = handle_death_say(rand);
    if is_moving {
        return HandleDeathPlan {
            action: HandleDeathAction::Busy,
            say,
            time_bump: 0.0,
        };
    }
    if home_quad < HANDLE_DEATH_HOME_GRAVE_QUAD {
        return HandleDeathPlan {
            action: HandleDeathAction::GravesThenRest,
            say,
            time_bump: 0.0,
        };
    }
    if handle_death_should_go_home(move_target_quad) {
        return HandleDeathPlan {
            action: HandleDeathAction::GoHome,
            say,
            time_bump: 0.0,
        };
    }
    HandleDeathPlan {
        action: HandleDeathAction::DropHeld,
        say,
        time_bump: HANDLE_DEATH_TIME_BUMP,
    }
}

/// After graves miss: go home if far, else drop.
pub fn handle_death_after_graves_miss(move_target_quad: i32) -> HandleDeathAction {
    if handle_death_should_go_home(move_target_quad) {
        HandleDeathAction::GoHome
    } else {
        HandleDeathAction::DropHeld
    }
}

pub fn handle_death_last_profession() -> &'static str {
    GRAVE_KEEPER_PROFESSION_KEY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_gate_uses_settings_max_age_not_vitals_120() {
        assert!(!should_handle_death(57.9, HANDLE_DEATH_MAX_AGE_DEFAULT));
        assert!(should_handle_death(58.0, HANDLE_DEATH_MAX_AGE_DEFAULT));
        assert_eq!(HANDLE_DEATH_AGE_OFFSET, 2.0);
        assert_eq!(handle_death_last_profession(), "GRAVEKEEPER");
    }

    #[test]
    fn young_skips() {
        let p = plan_handle_death(20.0, 60.0, false, false, false, 0, 0, 0.5);
        assert_eq!(p.action, HandleDeathAction::NotOld);
        assert!(p.say.is_none());
    }

    #[test]
    fn using_item_busy_before_say() {
        let p = plan_handle_death(58.0, 60.0, false, true, false, 0, 0, 0.01);
        assert_eq!(p.action, HandleDeathAction::Busy);
        assert!(p.say.is_none());
    }

    #[test]
    fn moving_busy_after_say() {
        let p = plan_handle_death(58.0, 60.0, false, false, true, 0, 0, 0.01);
        assert_eq!(p.action, HandleDeathAction::Busy);
        assert_eq!(p.say, Some(HANDLE_DEATH_SAY_GOODBYE));
        let p2 = plan_handle_death(58.0, 60.0, false, false, true, 0, 0, 0.07);
        assert_eq!(p2.say, Some(HANDLE_DEATH_SAY_JASONIAH));
        let p3 = plan_handle_death(58.0, 60.0, false, false, true, 0, 0, 0.5);
        assert!(p3.say.is_none());
    }

    #[test]
    fn near_home_tries_graves() {
        let p = plan_handle_death(58.0, 60.0, false, false, false, 100, 100, 0.5);
        assert_eq!(p.action, HandleDeathAction::GravesThenRest);
        assert_eq!(
            handle_death_after_graves_miss(100),
            HandleDeathAction::GoHome
        );
        assert_eq!(
            handle_death_after_graves_miss(10),
            HandleDeathAction::DropHeld
        );
    }

    #[test]
    fn far_from_home_goes_home() {
        let p = plan_handle_death(58.0, 60.0, false, false, false, 500, 500, 0.5);
        assert_eq!(p.action, HandleDeathAction::GoHome);
        assert!(handle_death_should_go_home(25));
        assert!(!handle_death_should_go_home(24));
    }

    #[test]
    fn already_home_no_graves_band_drops() {
        // home_quad >= 400 skips graves; move_target_quad < 25 → drop
        let p = plan_handle_death(58.0, 60.0, false, false, false, 400, 10, 0.5);
        assert_eq!(p.action, HandleDeathAction::DropHeld);
        assert_eq!(p.time_bump, HANDLE_DEATH_TIME_BUMP);
    }

    #[test]
    fn wipe_jobs_sets_last_gravekeeper_keeps_last_grave() {
        let mut farm = crate::FarmProfessionRuntime::default();
        farm.last_profession = Some(crate::FarmProfession::BasicFarmer);
        let mut smith = crate::SmithProfessionRuntime::default();
        smith.is_last_smith = true;
        smith.stage = 1.0;
        let mut baker = crate::BakerProfessionRuntime::default();
        let mut shepherd = crate::ShepherdProfessionRuntime::default();
        let mut pottery = crate::PotterProfessionRuntime::default();
        let mut fire_food = crate::FireFoodProfessionRuntime::default();
        let mut fire_keeper = crate::FireKeeperProfessionRuntime::default();
        fire_keeper.is_last_fire_keeper = true;
        let mut grave = crate::GraveKeeperProfessionRuntime::default();
        grave.set_last_grave(88, 3, 4);
        let mut hunter = crate::HunterProfessionRuntime::default();
        hunter.is_last_hunter = true;
        let mut lumberjack = crate::LumberjackProfessionRuntime::default();
        lumberjack.is_last_lumberjack = true;
        let mut collector = crate::CollectorProfessionRuntime::default();
        collector.is_last_collector = true;
        let mut foodserver = crate::FoodServerProfessionRuntime::default();
        foodserver.is_last_foodserver = true;
        wipe_jobs_assign_grave_keeper(
            &mut farm,
            &mut smith,
            &mut baker,
            &mut shepherd,
            &mut pottery,
            &mut fire_food,
            &mut fire_keeper,
            &mut grave,
            &mut hunter,
            &mut lumberjack,
            &mut collector,
            &mut foodserver,
        );
        assert!(!hunter.is_last_hunter);
        assert!(!lumberjack.is_last_lumberjack);
        assert!(!collector.is_last_collector);
        assert!(!foodserver.is_last_foodserver);
        assert!(farm.last_profession.is_none());
        assert!(!smith.is_last_smith);
        assert_eq!(smith.stage, 0.0);
        assert!(!fire_keeper.is_last_fire_keeper);
        assert!(grave.is_last_grave_keeper);
        assert_eq!(grave.weight, 1.0);
        assert!(!grave.is_assigned_grave_keeper);
        assert_eq!(grave.last_grave_id, 88);
        assert_eq!((grave.last_grave_x, grave.last_grave_y), (3, 4));
    }
}
