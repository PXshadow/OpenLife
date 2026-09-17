//! Haxe `AiBase.isCollecting` (chunk **AI-JOB-COLLECT**).
//!
//! `hasOrBecomeProfession('COLLECTOR', max)` then wrap `maxSearchRadius=60`
//! and `isCollectingHelper`: kindling `makeOrCollect`, `keepBushesAlive`,
//! age>40 `doSmithing(1)`, rabbits, mutton shortCraft, pork, thread, more kindling.
//! Assigned/last `isCollecting(100)`; after critical `isCollecting(1)`.

use crate::get_or_craft::{
    get_craft_and_drop_items_close_to_obj, CraftAndDropApply, CraftWorldObj,
};
use crate::get_or_craft::craft_item::count_craft_objs_near;

/// Kindling 72.
pub const COLLECT_KINDLING: i32 = 72;
/// Dead Rabbit 180.
pub const DEAD_RABBIT: i32 = 180;
/// Rabbit Fur 183.
pub const RABBIT_FUR: i32 = 183;
/// Raw Mutton 569.
pub const RAW_MUTTON: i32 = 569;
/// Hot Adobe Oven 250.
pub const COLLECT_HOT_OVEN: i32 = 250;
/// Hot Coals 85.
pub const COLLECT_HOT_COALS: i32 = 85;
/// Raw Pork 1342.
pub const RAW_PORK: i32 = 1342;
/// Thread 58.
pub const THREAD: i32 = 58;
/// Bowl of Soil 1137.
pub const COLLECT_BOWL_OF_SOIL: i32 = 1137;
/// Dying Gooseberry Bush 389.
pub const COLLECT_DYING_BUSH: i32 = 389;
/// Domestic Gooseberry Bush 391.
pub const DOMESTIC_BUSH: i32 = 391;
/// Dry Domestic Gooseberry Bush 393.
pub const DRY_DOMESTIC_BUSH: i32 = 393;
/// Vigorous Domestic Gooseberry Bush 1134.
pub const VIGOROUS_DOMESTIC_BUSH: i32 = 1134;
/// Empty Domestic Gooseberry Bush 1135.
pub const EMPTY_DOMESTIC_BUSH: i32 = 1135;

pub const KEEP_BUSHES_ALIVE_IDS: [i32; 4] = [
    DOMESTIC_BUSH,
    DRY_DOMESTIC_BUSH,
    VIGOROUS_DOMESTIC_BUSH,
    EMPTY_DOMESTIC_BUSH,
];
pub const KEEP_BUSHES_ALIVE_MIN: i32 = 20;
/// Haxe `countCurrentObject(183) < 15`.
pub const RABBIT_FUR_CAP: i32 = 15;

/// `makeOrCollect` CountCloseObjects radius.
pub const COLLECT_COUNT_RADIUS: i32 = 15;
/// GetCraftAndDrop `maxCount` (always 10).
pub const COLLECT_CRAFT_DROP_MAX: i32 = 10;
/// GetCraftAndDrop default dist.
pub const COLLECT_CRAFT_DROP_DIST: i32 = 8;
/// Haxe `itemToCraft.maxSearchRadius = 60` wrap.
pub const COLLECTING_SCAN_RADIUS: i32 = 60;
/// Mutton shortCraft distance.
pub const COLLECT_MUTTON_SHORTCRAFT_R: i32 = 10;
/// Mutton shortCraft `maxNewActor`.
pub const COLLECT_MUTTON_MAX_NEW_ACTOR: i32 = 5;
/// Nested `doSmithing(1)` age gate.
pub const COLLECT_SMITH_MIN_AGE: f32 = 40.0;

pub const COLLECTING_ASSIGNED_MAX: i32 = 100;
pub const COLLECTING_DEFAULT_MAX: i32 = 1;
pub const COLLECTOR_PROFESSION_KEY: &str = "COLLECTOR";

/// Sticky last + assigned + makeOrCollect taskState flags.
// Haxe: lastProfession / assignedProfession COLLECTOR + taskState['$id']
#[derive(Debug, Clone, PartialEq)]
pub struct CollectorProfessionRuntime {
    pub is_last_collector: bool,
    pub is_assigned_collector: bool,
    pub weight: f32,
    /// Haxe `taskState['72']` (both kindling makeOrCollect calls share this).
    pub task_kindling: f32,
    /// Haxe `taskState['180']`.
    pub task_rabbit: f32,
    /// Haxe `taskState['569']`.
    pub task_mutton: f32,
    /// Haxe `taskState['1342']`.
    pub task_pork: f32,
    /// Haxe `taskState['58']`.
    pub task_thread: f32,
}

impl Default for CollectorProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_collector: false,
            is_assigned_collector: false,
            weight: 0.0,
            task_kindling: 0.0,
            task_rabbit: 0.0,
            task_mutton: 0.0,
            task_pork: 0.0,
            task_thread: 0.0,
        }
    }
}

impl CollectorProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        self.task_kindling = 0.0;
        self.task_rabbit = 0.0;
        self.task_mutton = 0.0;
        self.task_pork = 0.0;
        self.task_thread = 0.0;
        if !last_was_foodserver {
            self.is_last_collector = false;
        }
    }
}

pub fn parse_collector_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    // Haxe: assignedProfession 'COLLECT' remaps to COLLECTOR ~4957
    prof.eq_ignore_ascii_case("COLLECTOR") || prof.eq_ignore_ascii_case("COLLECT")
}

pub fn assign_collector_from_speech(
    runtime: &mut CollectorProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_collector_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_collector = true;
    runtime.is_last_collector = true;
    runtime.weight = 1.0;
    true
}

pub fn resolve_collector_assigned_job(runtime: &CollectorProfessionRuntime) -> bool {
    runtime.is_assigned_collector || runtime.is_last_collector
}

pub fn has_or_become_collector(
    runtime: &mut CollectorProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_collector {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count_with_last.max(0.0) >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_collector = true;
    true
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollectingSensors {
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub held_id: i32,
    pub age: f32,
}

impl Default for CollectingSensors {
    fn default() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: 0,
            age: 20.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectingAction {
    None,
    CraftAndDrop {
        which_id: i32,
        apply: CraftAndDropApply,
    },
    ShortCraft {
        actor: i32,
        target: i32,
        radius: i32,
        max_new_actor: i32,
        craft_actor_if_needed: bool,
    },
    /// Haxe `age > 40 && doSmithing(1)`.
    DeferSmithing,
}

impl CollectingAction {
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

/// Haxe `makeOrCollect(id, min, max)` toward home.
// Haxe: AiBase.makeOrCollect ~6034
pub fn make_or_collect(
    objs: &[CraftWorldObj],
    target_x: i32,
    target_y: i32,
    held_id: i32,
    player_x: i32,
    player_y: i32,
    id: i32,
    min: i32,
    max: i32,
    task_flag: &mut f32,
) -> Option<CraftAndDropApply> {
    let count = count_craft_objs_near(
        objs,
        &[id],
        target_x,
        target_y,
        COLLECT_COUNT_RADIUS,
    );
    if count <= min {
        *task_flag = 1.0;
    }
    if count >= max {
        *task_flag = 0.0;
    }
    if *task_flag <= 0.0 {
        return None;
    }
    let apply = get_craft_and_drop_items_close_to_obj(
        objs,
        target_x,
        target_y,
        id,
        COLLECT_CRAFT_DROP_MAX,
        COLLECT_CRAFT_DROP_DIST,
        held_id,
        player_x,
        player_y,
    );
    not_enough(apply)
}

fn count_ids(objs: &[CraftWorldObj], ids: &[i32], held_id: i32) -> i32 {
    let mut n = if ids.contains(&held_id) { 1 } else { 0 };
    for o in objs {
        if ids.contains(&o.parent_id) {
            n += 1;
        }
    }
    n
}

/// Pure `isCollectingHelper` after the profession gate.
// Haxe: AiBase.isCollectingHelper ~5995–6031
pub fn is_collecting(
    sensors: &CollectingSensors,
    collector: &mut CollectorProfessionRuntime,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
    objs: &[CraftWorldObj],
) -> CollectingAction {
    if !has_or_become_collector(collector, max_people, peer_count, was_idle) {
        return CollectingAction::None;
    }
    let hx = sensors.home_x;
    let hy = sensors.home_y;
    let px = sensors.player_x;
    let py = sensors.player_y;
    let held = sensors.held_id;

    if let Some(apply) = make_or_collect(
        objs,
        hx,
        hy,
        held,
        px,
        py,
        COLLECT_KINDLING,
        2,
        5,
        &mut collector.task_kindling,
    ) {
        return CollectingAction::CraftAndDrop {
            which_id: COLLECT_KINDLING,
            apply,
        };
    }

    let bushes = count_ids(objs, &KEEP_BUSHES_ALIVE_IDS, held);
    // Haxe keepBushesAlive: countCurrentObjects < 20 then shortCraft(1137,389,30)
    // (shortCraft no-ops if no dying target within r=30 of the player).
    if bushes < KEEP_BUSHES_ALIVE_MIN
        && count_craft_objs_near(objs, &[COLLECT_DYING_BUSH], px, py, 30) > 0
    {
        return CollectingAction::ShortCraft {
            actor: COLLECT_BOWL_OF_SOIL,
            target: COLLECT_DYING_BUSH,
            radius: 30,
            max_new_actor: -1,
            craft_actor_if_needed: true,
        };
    }

    // Haxe: age > 40 && doSmithing(1) — live tries smith, then continues on false.
    if sensors.age > COLLECT_SMITH_MIN_AGE {
        return CollectingAction::DeferSmithing;
    }

    collecting_after_smith(sensors, collector, objs)
}

/// Haxe `isCollectingHelper` after `keepBushesAlive` / failed `doSmithing(1)`.
// Haxe: AiBase.isCollectingHelper L6006–6031
pub fn collecting_after_smith(
    sensors: &CollectingSensors,
    collector: &mut CollectorProfessionRuntime,
    objs: &[CraftWorldObj],
) -> CollectingAction {
    let hx = sensors.home_x;
    let hy = sensors.home_y;
    let px = sensors.player_x;
    let py = sensors.player_y;
    let held = sensors.held_id;

    let fur = count_ids(objs, &[RABBIT_FUR], held);
    if fur < RABBIT_FUR_CAP {
        if let Some(apply) = make_or_collect(
            objs,
            hx,
            hy,
            held,
            px,
            py,
            DEAD_RABBIT,
            2,
            4,
            &mut collector.task_rabbit,
        ) {
            return CollectingAction::CraftAndDrop {
                which_id: DEAD_RABBIT,
                apply,
            };
        }
    }

    CollectingAction::None
        .pipe_mutton_short_then_collect(sensors, collector, objs)
}

trait CollectTail {
    fn pipe_mutton_short_then_collect(
        self,
        sensors: &CollectingSensors,
        collector: &mut CollectorProfessionRuntime,
        objs: &[CraftWorldObj],
    ) -> CollectingAction;
}

impl CollectTail for CollectingAction {
    fn pipe_mutton_short_then_collect(
        self,
        sensors: &CollectingSensors,
        collector: &mut CollectorProfessionRuntime,
        objs: &[CraftWorldObj],
    ) -> CollectingAction {
        if self.is_some() {
            return self;
        }
        collecting_helper_tail(sensors, collector, objs)
    }
}

fn collecting_helper_tail(
    sensors: &CollectingSensors,
    collector: &mut CollectorProfessionRuntime,
    objs: &[CraftWorldObj],
) -> CollectingAction {
    let hx = sensors.home_x;
    let hy = sensors.home_y;
    let px = sensors.player_x;
    let py = sensors.player_y;
    let held = sensors.held_id;

    // Haxe: shortCraft(569, 250, 10, false, 5) then shortCraft(569, 85, ...)
    // getClosestObjectById is from the player, not home.
    if count_craft_objs_near(objs, &[COLLECT_HOT_OVEN], px, py, COLLECT_MUTTON_SHORTCRAFT_R) > 0
    {
        return CollectingAction::ShortCraft {
            actor: RAW_MUTTON,
            target: COLLECT_HOT_OVEN,
            radius: COLLECT_MUTTON_SHORTCRAFT_R,
            max_new_actor: COLLECT_MUTTON_MAX_NEW_ACTOR,
            craft_actor_if_needed: false,
        };
    }
    if count_craft_objs_near(objs, &[COLLECT_HOT_COALS], px, py, COLLECT_MUTTON_SHORTCRAFT_R) > 0
    {
        return CollectingAction::ShortCraft {
            actor: RAW_MUTTON,
            target: COLLECT_HOT_COALS,
            radius: COLLECT_MUTTON_SHORTCRAFT_R,
            max_new_actor: COLLECT_MUTTON_MAX_NEW_ACTOR,
            craft_actor_if_needed: false,
        };
    }

    if let Some(apply) = make_or_collect(
        objs, hx, hy, held, px, py, RAW_MUTTON, 1, 5, &mut collector.task_mutton,
    ) {
        return CollectingAction::CraftAndDrop {
            which_id: RAW_MUTTON,
            apply,
        };
    }
    if let Some(apply) = make_or_collect(
        objs, hx, hy, held, px, py, RAW_PORK, 1, 5, &mut collector.task_pork,
    ) {
        return CollectingAction::CraftAndDrop {
            which_id: RAW_PORK,
            apply,
        };
    }
    if let Some(apply) = make_or_collect(
        objs, hx, hy, held, px, py, THREAD, 1, 3, &mut collector.task_thread,
    ) {
        return CollectingAction::CraftAndDrop {
            which_id: THREAD,
            apply,
        };
    }
    if let Some(apply) = make_or_collect(
        objs,
        hx,
        hy,
        held,
        px,
        py,
        COLLECT_KINDLING,
        6,
        10,
        &mut collector.task_kindling,
    ) {
        return CollectingAction::CraftAndDrop {
            which_id: COLLECT_KINDLING,
            apply,
        };
    }
    CollectingAction::None
}

pub fn collecting_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        COLLECTING_ASSIGNED_MAX
    } else {
        COLLECTING_DEFAULT_MAX
    }
}

#[inline]
pub fn collecting_job_rung_label(rung_label: &str) -> bool {
    matches!(rung_label, "ASSIGNED_JOB" | "LOW_PRIORITY_WORK")
}

pub fn try_decide_collecting_from_rung(
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &CollectingSensors,
    collector: &mut CollectorProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
    objs: &[CraftWorldObj],
) -> Option<CollectingAction> {
    if !collecting_job_rung_label(rung_label) {
        return None;
    }
    let assigned = is_assigned_job
        || resolve_collector_assigned_job(collector)
        || rung_label == "ASSIGNED_JOB";
    let max = collecting_max_for_dispatch(assigned, rung_label);
    Some(is_collecting(
        sensors, collector, max, peer_count, was_idle, objs,
    ))
}

pub fn collecting_sensors(
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    age: f32,
) -> CollectingSensors {
    CollectingSensors {
        player_x,
        player_y,
        home_x,
        home_y,
        held_id,
        age,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> CollectingSensors {
        CollectingSensors::default()
    }

    #[test]
    fn peer_cap_blocks_new_collector() {
        let mut rt = CollectorProfessionRuntime::default();
        assert_eq!(
            is_collecting(&home(), &mut rt, 1, 1.0, 0.0, &[]),
            CollectingAction::None
        );
        assert!(!rt.is_last_collector);
    }

    #[test]
    fn missing_kindling_crafts() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &[]);
        assert_eq!(rt.task_kindling, 1.0);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: COLLECT_KINDLING,
                apply: CraftAndDropApply::CraftItem {
                    object_id: COLLECT_KINDLING
                },
            }
        );
    }

    #[test]
    fn kindling_stocked_then_bushes() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.push(CraftWorldObj::simple(COLLECT_DYING_BUSH, 2, 1));
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(rt.task_kindling, 0.0);
        assert_eq!(
            a,
            CollectingAction::ShortCraft {
                actor: COLLECT_BOWL_OF_SOIL,
                target: COLLECT_DYING_BUSH,
                radius: 30,
                max_new_actor: -1,
                craft_actor_if_needed: true,
            }
        );
    }

    #[test]
    fn low_bushes_without_dying_skips_to_rabbits() {
        // Haxe shortCraft(1137,389,30) returns false if no dying bush
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: DEAD_RABBIT,
                apply: CraftAndDropApply::CraftItem {
                    object_id: DEAD_RABBIT
                },
            }
        );
    }

    #[test]
    fn mutton_oven_only_from_player_not_home() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut s = home();
        s.player_x = 0;
        s.player_y = 0;
        s.home_x = 40;
        s.home_y = 0;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, 40 + i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, 40 + i, 2)));
        objs.extend((0..15).map(|i| CraftWorldObj::simple(RABBIT_FUR, 40 + i, 3)));
        objs.push(CraftWorldObj::simple(COLLECT_HOT_OVEN, 40, 0));
        let a = is_collecting(&s, &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: RAW_MUTTON,
                apply: CraftAndDropApply::CraftItem {
                    object_id: RAW_MUTTON
                },
            }
        );
    }

    #[test]
    fn bushes_alive_then_rabbits() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, i, 2)));
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: DEAD_RABBIT,
                apply: CraftAndDropApply::CraftItem {
                    object_id: DEAD_RABBIT
                },
            }
        );
    }

    #[test]
    fn fur_cap_skips_rabbits_then_mutton_oven() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, i, 2)));
        objs.extend((0..15).map(|i| CraftWorldObj::simple(RABBIT_FUR, i, 3)));
        objs.push(CraftWorldObj::simple(COLLECT_HOT_OVEN, 1, 1));
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CollectingAction::ShortCraft {
                actor: RAW_MUTTON,
                target: COLLECT_HOT_OVEN,
                radius: 10,
                max_new_actor: 5,
                craft_actor_if_needed: false,
            }
        );
    }

    #[test]
    fn smith_miss_continues_to_rabbits() {
        // Haxe: age>40 && doSmithing(1) false → makeOrCollect(180,2,4)
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, i, 2)));
        let a = collecting_after_smith(&home(), &mut rt, &objs);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: DEAD_RABBIT,
                apply: CraftAndDropApply::CraftItem {
                    object_id: DEAD_RABBIT
                },
            }
        );
    }

    #[test]
    fn age_over_40_defers_smith_after_bushes() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        let mut s = home();
        s.age = 41.0;
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, i, 2)));
        let a = is_collecting(&s, &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(a, CollectingAction::DeferSmithing);
    }

    #[test]
    fn assigned_uses_max_100() {
        assert_eq!(
            collecting_max_for_dispatch(true, "ASSIGNED_JOB"),
            COLLECTING_ASSIGNED_MAX
        );
        assert_eq!(
            collecting_max_for_dispatch(false, "LOW_PRIORITY_WORK"),
            COLLECTING_DEFAULT_MAX
        );
    }

    #[test]
    fn mid_rung_skipped() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        assert!(try_decide_collecting_from_rung(
            "MID_PRIORITY_TASKS",
            false,
            &home(),
            &mut rt,
            0.0,
            0.0,
            &[]
        )
        .is_none());
        let a = try_decide_collecting_from_rung(
            "ASSIGNED_JOB",
            true,
            &home(),
            &mut rt,
            0.0,
            0.0,
            &[],
        );
        assert!(matches!(
            a,
            Some(CollectingAction::CraftAndDrop {
                which_id: COLLECT_KINDLING,
                ..
            })
        ));
    }

    #[test]
    fn speech_assign_collector() {
        let mut r = CollectorProfessionRuntime::default();
        assert!(parse_collector_profession_speech("COLLECTOR!"));
        assert!(parse_collector_profession_speech("collect"));
        assert!(!parse_collector_profession_speech("LUMBERJACK!"));
        assert!(assign_collector_from_speech(&mut r, "COLLECT!"));
        assert!(r.is_assigned_collector);
        assert!(r.is_last_collector);
        assert!(resolve_collector_assigned_job(&r));
    }

    #[test]
    fn extra_kindling_after_other_stocks() {
        let mut rt = CollectorProfessionRuntime::default();
        rt.is_last_collector = true;
        // 5 kindling skips first (max 5); 20 bushes; 15 fur; no oven/coals;
        // mutton/pork/thread empty → first of those crafts mutton (min 1).
        let mut objs: Vec<_> = (0..5)
            .map(|i| CraftWorldObj::simple(COLLECT_KINDLING, i, 0))
            .collect();
        objs.extend((0..20).map(|i| CraftWorldObj::simple(DOMESTIC_BUSH, i, 2)));
        objs.extend((0..15).map(|i| CraftWorldObj::simple(RABBIT_FUR, i, 3)));
        let a = is_collecting(&home(), &mut rt, 1, 0.0, 0.0, &objs);
        assert_eq!(
            a,
            CollectingAction::CraftAndDrop {
                which_id: RAW_MUTTON,
                apply: CraftAndDropApply::CraftItem {
                    object_id: RAW_MUTTON
                },
            }
        );
    }
}
