//! Haxe `AiBase.isCuttingWood` (chunk **AI-JOB-LUMBER**).
//!
//! `firePlace != null` then `hasOrBecomeProfession('LUMBERJACK', max)`:
//! Firewood 344 near fire (count r=15, GetCraftAndDrop max 10 dist 8);
//! Butt Log 345 near home (count r=15, GetCraftAndDrop max 5 dist 8).
//! Assigned/last `isCuttingWood(100)`; low-priority `isCuttingWood()`.
//!
//! After stage 3 Haxe `if (cleanUp()) return true` (same `clean_up_action` SM).

use crate::get_or_craft::{
    get_craft_and_drop_items_close_to_obj, CraftAndDropApply, CraftWorldObj,
};
use crate::get_or_craft::craft_item::count_craft_objs_near;

/// Firewood 344.
// Haxe: AiBase.isCuttingWood ~924
pub const FIREWOOD: i32 = 344;
/// Butt Log 345.
pub const LUMBER_BUTT_LOG: i32 = 345;

/// Outer `CountCloseObjects` radius.
// Haxe: CountCloseObjects(..., 15)
pub const CUTTING_WOOD_COUNT_RADIUS: i32 = 15;
/// Reset stage when count &lt; 2.
pub const CUTTING_WOOD_LOW_STOCK: i32 = 2;
/// Skip GetCraftAndDrop when outer count ≥ 5.
pub const CUTTING_WOOD_STOCK_SKIP: i32 = 5;
/// Firewood GetCraftAndDrop `maxCount`.
// Haxe: GetCraftAndDropItemsCloseToObj(firePlace, 344, 10)
pub const FIREWOOD_CRAFT_DROP_MAX: i32 = 10;
/// Butt log GetCraftAndDrop `maxCount`.
// Haxe: GetCraftAndDropItemsCloseToObj(home, 345, 5)
pub const BUTT_LOG_CRAFT_DROP_MAX: i32 = 5;
/// GetCraftAndDrop default `dist`.
pub const CUTTING_WOOD_CRAFT_DROP_DIST: i32 = 8;
/// GetClosest search around fire/home.
pub const CUTTING_WOOD_SCAN_RADIUS: i32 = 30;
/// Assigned/last `isCuttingWood(100)`.
pub const CUTTING_WOOD_ASSIGNED_MAX: i32 = 100;
/// Low-priority `isCuttingWood()` default maxPeople.
pub const CUTTING_WOOD_DEFAULT_MAX: i32 = 1;

/// Canonical Haxe profession string.
pub const LUMBERJACK_PROFESSION_KEY: &str = "LUMBERJACK";

/// Sticky last + assigned + Haxe `profession['LUMBERJACK']` stage 1/2/3.
// Haxe: AiBase.profession['LUMBERJACK'] + lastProfession / assignedProfession
#[derive(Debug, Clone, PartialEq)]
pub struct LumberjackProfessionRuntime {
    pub is_last_lumberjack: bool,
    pub is_assigned_lumberjack: bool,
    /// Haxe `this.profession['LUMBERJACK']` (1 firewood / 2 logs / 3 done).
    pub stage: f32,
    /// hasOrBecome weight (0 idle / 1 active).
    pub weight: f32,
}

impl Default for LumberjackProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_lumberjack: false,
            is_assigned_lumberjack: false,
            stage: 0.0,
            weight: 0.0,
        }
    }
}

impl LumberjackProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        self.stage = 0.0;
        if !last_was_foodserver {
            self.is_last_lumberjack = false;
        }
    }
}

/// Parse speech / assigned tokens for lumberjack.
// Haxe: assignedProfession / lastProfession == 'LUMBERJACK'
pub fn parse_lumberjack_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("LUMBERJACK") || prof.eq_ignore_ascii_case("LUMBER")
}

/// Assign from speech `LUMBERJACK!`.
pub fn assign_lumberjack_from_speech(
    runtime: &mut LumberjackProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_lumberjack_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_lumberjack = true;
    runtime.is_last_lumberjack = true;
    runtime.weight = 1.0;
    true
}

/// Assigned or last LUMBERJACK job dispatch.
// Haxe: assignedProfession == 'LUMBERJACK' || lastProfession == 'LUMBERJACK'
pub fn resolve_lumberjack_assigned_job(runtime: &LumberjackProfessionRuntime) -> bool {
    runtime.is_assigned_lumberjack || runtime.is_last_lumberjack
}

/// Haxe `hasOrBecomeProfession('LUMBERJACK', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_lumberjack(
    runtime: &mut LumberjackProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_lumberjack {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count_with_last.max(0.0) >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_lumberjack = true;
    if runtime.stage < 1.0 {
        runtime.stage = 1.0;
    }
    true
}

/// World sensors for pure isCuttingWood.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CuttingWoodSensors {
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub held_id: i32,
    pub age: f32,
    /// Haxe `myPlayer.firePlace != null`.
    pub has_fire: bool,
    pub fire_x: i32,
    pub fire_y: i32,
}

impl Default for CuttingWoodSensors {
    fn default() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: 0,
            age: 20.0,
            has_fire: true,
            fire_x: 0,
            fire_y: 0,
        }
    }
}

/// Pure decision output for isCuttingWood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuttingWoodAction {
    None,
    /// Haxe `GetCraftAndDropItemsCloseToObj` (not AlreadyEnough).
    CraftAndDrop {
        which_id: i32,
        apply: CraftAndDropApply,
    },
    /// Haxe `if (cleanUp()) return true` after lumber stage 3.
    // Haxe: AiBase.isCuttingWood L938
    TryCleanup,
}

impl CuttingWoodAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn not_enough(apply: CraftAndDropApply) -> Option<CraftAndDropApply> {
    match apply {
        CraftAndDropApply::AlreadyEnough => None,
        other => Some(other),
    }
}

/// Pure `isCuttingWood` body after the firePlace gate.
// Haxe: AiBase.isCuttingWood ~919–940
pub fn is_cutting_wood(
    sensors: &CuttingWoodSensors,
    lumber: &mut LumberjackProfessionRuntime,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
    objs: &[CraftWorldObj],
) -> CuttingWoodAction {
    if !sensors.has_fire {
        return CuttingWoodAction::None;
    }
    if !has_or_become_lumberjack(lumber, max_people, peer_count, was_idle) {
        return CuttingWoodAction::None;
    }

    let wood_count = count_craft_objs_near(
        objs,
        &[FIREWOOD],
        sensors.fire_x,
        sensors.fire_y,
        CUTTING_WOOD_COUNT_RADIUS,
    );
    if wood_count < CUTTING_WOOD_LOW_STOCK {
        lumber.stage = 1.0;
    }
    if lumber.stage < 2.0 && wood_count < CUTTING_WOOD_STOCK_SKIP {
        let apply = get_craft_and_drop_items_close_to_obj(
            objs,
            sensors.fire_x,
            sensors.fire_y,
            FIREWOOD,
            FIREWOOD_CRAFT_DROP_MAX,
            CUTTING_WOOD_CRAFT_DROP_DIST,
            sensors.held_id,
            sensors.player_x,
            sensors.player_y,
        );
        if let Some(apply) = not_enough(apply) {
            return CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                apply,
            };
        }
    }
    lumber.stage = 2.0;

    let log_count = count_craft_objs_near(
        objs,
        &[LUMBER_BUTT_LOG],
        sensors.home_x,
        sensors.home_y,
        CUTTING_WOOD_COUNT_RADIUS,
    );
    if log_count < CUTTING_WOOD_LOW_STOCK {
        lumber.stage = 2.0;
    }
    if lumber.stage < 3.0 && log_count < CUTTING_WOOD_STOCK_SKIP {
        let apply = get_craft_and_drop_items_close_to_obj(
            objs,
            sensors.home_x,
            sensors.home_y,
            LUMBER_BUTT_LOG,
            BUTT_LOG_CRAFT_DROP_MAX,
            CUTTING_WOOD_CRAFT_DROP_DIST,
            sensors.held_id,
            sensors.player_x,
            sensors.player_y,
        );
        if let Some(apply) = not_enough(apply) {
            return CuttingWoodAction::CraftAndDrop {
                which_id: LUMBER_BUTT_LOG,
                apply,
            };
        }
    }
    lumber.stage = 3.0;
    CuttingWoodAction::TryCleanup
}

/// Max people for isCuttingWood from rung / assigned flag.
// Haxe: isCuttingWood() / isCuttingWood(100)
pub fn cutting_wood_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        CUTTING_WOOD_ASSIGNED_MAX
    } else {
        CUTTING_WOOD_DEFAULT_MAX
    }
}

#[inline]
pub fn cutting_wood_job_rung_label(rung_label: &str) -> bool {
    matches!(rung_label, "ASSIGNED_JOB" | "LOW_PRIORITY_WORK")
}

/// Thin ladder bridge.
pub fn try_decide_cutting_wood_from_rung(
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &CuttingWoodSensors,
    lumber: &mut LumberjackProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
    objs: &[CraftWorldObj],
) -> Option<CuttingWoodAction> {
    if !cutting_wood_job_rung_label(rung_label) {
        return None;
    }
    let assigned = is_assigned_job
        || resolve_lumberjack_assigned_job(lumber)
        || rung_label == "ASSIGNED_JOB";
    let max = cutting_wood_max_for_dispatch(assigned, rung_label);
    Some(is_cutting_wood(
        sensors, lumber, max, peer_count, was_idle, objs,
    ))
}

/// Fill sensors from firePlace sticky + player/home.
pub fn cutting_wood_sensors(
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    age: f32,
    fire_place_id: i32,
    fire_x: i32,
    fire_y: i32,
) -> CuttingWoodSensors {
    CuttingWoodSensors {
        player_x,
        player_y,
        home_x,
        home_y,
        held_id,
        age,
        has_fire: fire_place_id != 0,
        fire_x,
        fire_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire_home() -> CuttingWoodSensors {
        CuttingWoodSensors {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: 0,
            age: 20.0,
            has_fire: true,
            fire_x: 0,
            fire_y: 0,
        }
    }

    #[test]
    fn no_fireplace_skips() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let mut s = fire_home();
        s.has_fire = false;
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &[]);
        assert_eq!(a, CuttingWoodAction::None);
    }

    #[test]
    fn peer_cap_blocks_new_lumberjack() {
        let mut rt = LumberjackProfessionRuntime::default();
        let s = fire_home();
        assert_eq!(
            is_cutting_wood(&s, &mut rt, 1, 1.0, 0.0, &[]),
            CuttingWoodAction::None
        );
        assert!(!rt.is_last_lumberjack);
    }

    #[test]
    fn missing_firewood_crafts() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let s = fire_home();
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &[]);
        assert_eq!(rt.stage, 1.0);
        assert_eq!(
            a,
            CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                apply: CraftAndDropApply::CraftItem {
                    object_id: FIREWOOD
                },
            }
        );
    }

    #[test]
    fn held_firewood_far_gotos_fire() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let mut s = fire_home();
        s.held_id = FIREWOOD;
        s.player_x = 10;
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &[]);
        assert_eq!(
            a,
            CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                apply: CraftAndDropApply::GotoDropTarget {
                    target_x: 0,
                    target_y: 0,
                },
            }
        );
    }

    #[test]
    fn held_firewood_close_drops() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let mut s = fire_home();
        s.held_id = FIREWOOD;
        s.player_x = 1;
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &[]);
        assert_eq!(
            a,
            CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                apply: CraftAndDropApply::DropNearTarget {
                    target_x: 0,
                    target_y: 0,
                },
            }
        );
    }

    #[test]
    fn pickup_firewood_outside_drop_band() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let s = fire_home();
        let objs = vec![CraftWorldObj::simple(FIREWOOD, 12, 0)];
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                apply: CraftAndDropApply::Pickup {
                    object_id: FIREWOOD,
                    x: 12,
                    y: 0,
                },
            }
        );
    }

    #[test]
    fn enough_firewood_then_craft_logs() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let s = fire_home();
        // 5 firewood within r=15 skips GetCraftAndDrop wood; then logs.
        let objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(FIREWOOD, i, 0))
            .collect();
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(rt.stage, 2.0);
        assert_eq!(
            a,
            CuttingWoodAction::CraftAndDrop {
                which_id: LUMBER_BUTT_LOG,
                apply: CraftAndDropApply::CraftItem {
                    object_id: LUMBER_BUTT_LOG
                },
            }
        );
    }

    #[test]
    fn stocked_wood_and_logs_runs_cleanup() {
        // Haxe L938: after stage 3, if (cleanUp()) return true
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let s = fire_home();
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(FIREWOOD, i, 0))
            .collect();
        objs.extend((0..5).map(|i| CraftWorldObj::simple(LUMBER_BUTT_LOG, i, 1)));
        let a = is_cutting_wood(&s, &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(a, CuttingWoodAction::TryCleanup);
        assert_eq!(rt.stage, 3.0);
    }

    #[test]
    fn assigned_uses_max_100() {
        assert_eq!(
            cutting_wood_max_for_dispatch(true, "ASSIGNED_JOB"),
            CUTTING_WOOD_ASSIGNED_MAX
        );
        assert_eq!(
            cutting_wood_max_for_dispatch(false, "LOW_PRIORITY_WORK"),
            CUTTING_WOOD_DEFAULT_MAX
        );
    }

    #[test]
    fn mid_rung_skipped() {
        let mut rt = LumberjackProfessionRuntime::default();
        rt.is_last_lumberjack = true;
        let s = fire_home();
        assert!(try_decide_cutting_wood_from_rung(
            "MID_PRIORITY_TASKS",
            false,
            &s,
            &mut rt,
            0.0,
            0.0,
            &[]
        )
        .is_none());
        let a = try_decide_cutting_wood_from_rung(
            "ASSIGNED_JOB",
            true,
            &s,
            &mut rt,
            0.0,
            0.0,
            &[],
        );
        assert!(matches!(
            a,
            Some(CuttingWoodAction::CraftAndDrop {
                which_id: FIREWOOD,
                ..
            })
        ));
    }

    #[test]
    fn speech_assign_lumberjack() {
        let mut r = LumberjackProfessionRuntime::default();
        assert!(parse_lumberjack_profession_speech("LUMBERJACK!"));
        assert!(parse_lumberjack_profession_speech("lumber"));
        assert!(!parse_lumberjack_profession_speech("HUNTER!"));
        assert!(assign_lumberjack_from_speech(&mut r, "LUMBERJACK!"));
        assert!(r.is_assigned_lumberjack);
        assert!(r.is_last_lumberjack);
        assert_eq!(r.weight, 1.0);
        assert!(resolve_lumberjack_assigned_job(&r));
    }
}
