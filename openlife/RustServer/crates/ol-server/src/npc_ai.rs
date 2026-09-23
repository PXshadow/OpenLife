//! Basic AI NPC scheduler (Haxe `AiBase.RunAi` shape — single thread).
//!
//! Think order matches `AiBase.doTimeStuffHelper`: escape → drop/close-use → baby →
//! eat/feed/clothes → consider-food → pickup food → attack/kill/feed → L599 jobs
//! (CriticalCraft, fire, knife, clothes, lambs, hunting, sharpie, graves, bucket,
//! skewer, craft queue, clothing, assigned, critical, low) → home / idle.
//! Activity logged in RAM and flushed every 30s ([`npc_activity`]).
//!
//! // Haxe: ServerAi.doTimeStuff → AiBase.doTimeStuffHelper

use crate::npc_activity::{
    NpcActivityEvent, NpcActivityKind, NpcActivityLog, NpcStuckTracker,
};
use ol_config::{gameplay_defaults, LiveSettings, AI_IGNORED_FLOOR_IDS};
use ol_content::ContentDb;
use ol_metrics::{Counters, ScopeTimer};
use ol_ai::{
    BestFoodHit, BestFoodQuery, CommandSink, DeadlyPlayerCandidate, EscapeThreat, FoodSearch,
    LiveSensorInput, PlayerWriteInterface, DEFAULT_FOOD_SEARCH_RADIUS, DEADLY_PLAYER_SEARCH_DIST_AI,
    ESCAPE_ANGRY_TIME_IGNORE, ESCAPE_DIST, ESCAPE_FOOD_CRIT_SKIP, ESCAPE_IS_DANGEROUS_RADIUS,
    auto_follow_ai_human_early_return, baby_hungry_follow_tiles, check_is_hungry_and_eat_effects,
    child_with_mother_follow_tiles, compute_do_stuff, decay_was_idle, escape_debug_say,
    escape_maybe_assign_animal_target, escape_side_effects, fill_live_sensors,
    get_close_deadly_player, get_close_player_target, handle_temperature_after_pickup, hungry_infant_always_returns,
    idle_say_line, idle_should_drop_held, pick_escape_tile, search_food_and_eat_debug_say,
    is_child_and_has_mother_from_follow, is_moving_to_player_needed, is_superbad_temp,
    nice_baby_noble_wants_weapon, should_escape_on_moved_one_tile, should_switch_cloth,
    should_zero_profession_weight, skip_escape_for_hunt, skip_home_and_jobs_while_moving,
    switch_cloth_resolve_parent, in_get_close_clothings_square, is_get_close_clothing_object,
    tick_waiting_time, waiting_time_blocks_think,
    GET_CLOSE_CLOTHINGS_RADIUS, DEVIL_MASK_ID, GOBLIN_MASK_ID, PLAYER_TARGET_SEARCH_DIST,
    EscapeRand, KNIFE_ID, WAR_SWORD_ID, PlayerTargetCandidate,
};
use ol_net::NetIntent;
use ol_main_ai::{plan_hungry_food, ThinkPlan, ThinkSensors};
use ol_player_helper::{
    can_eat_obj, is_dangerous_near, is_eating_conservation_skip, is_eating_drop_peel,
    is_eating_head, is_obj_yum, pick_best_search_food, to_best_hit, AiFoodSearchFlags,
    ProcessFoodOpts, SearchFoodCand, COOKED_GOOSE_EAT, COOKED_GOOSE_KEEP_RADIUS,
    EAT_PEEL_DROP_DIST,
};
use ol_sim::{
    ai_rebirth_wait_secs,
    apply_fire_craft_search_radius_override, apply_food_goto_fail, apply_job_flags_to_live_input,
    attack_player, attack_player_action_to_live_intent, deadly_distance_for_held, get_weapon,
    has_weapon_close, is_bloody_weapon, AttackPlayerClothing, AttackPlayerInput,
    AttackPlayerTarget, GetWeaponAction, BOW_AND_ARROW, HAS_WEAPON_CLOSE_SEARCH,
    MIN_AI_AGE_FOR_COMBAT, RATTLE_SNAKE, WEAPON_SEARCH_DIST,
    kill_animal_body, kill_animal_bow_hunt, kill_animal_prefix, wolf_in_home_search,
    wolf_tile_allowed,
    KillAnimalAction, KillAnimalBodyInput, KillAnimalPrefixInput, KillAnimalPrefixKind,
    KILL_ANIMAL_GOTO_FAIL_CLEAR, KILL_ANIMAL_SNAKE_RADIUS, KILL_ANIMAL_WOLF_SEARCH,
    TIME_HELPER_TICK_TIME, TIME_LOOKED_NEVER, WOLF, AnimalKind,
    pick_close_hungry_child, pick_most_distant_own_child, HungryChildCand, is_fertile,
    is_eve_or_adam_name, STARTING_NAME, FEMALE_FIRST_NAMES, MALE_FIRST_NAMES,
    get_max_child_feeding, can_pickup_baby_distance,
    MAX_CHILD_AGE_BREAST_FEEDING, HUNGRY_CHILD_SEARCH_DIST, DISTANT_OWN_CHILD_MIN_DIST,
    DISTANT_OWN_CHILD_SEARCH_DIST,
    go_home_goal_xy, go_home_move_target, should_path_to_home, handle_death_after_graves_miss, plan_handle_death,
    should_handle_death, wipe_jobs_assign_grave_keeper, HandleDeathAction, HANDLE_DEATH_TIME_BUMP,
    fail_warm_clear_place, get_close_biome, handling_fire_profession_scan_tick,
    is_angry_or_terrified, is_super_cold_for_person,
    is_super_hot_for_person, person_looks_female, plan_handle_temperature,
    HandleTemperatureAction, HandleTemperatureInput, HandleTemperaturePlan,
    COOL_BIOMES, GET_CLOSE_BIOME_DIST, HANDLE_TEMP_KINDLING, HANDLE_TEMP_LARGE_FAST_FIRE,
    HANDLE_TEMP_RELAX_TIME, HANDLE_TEMP_SAY_DRINK, WARM_BIOMES,
    advance_remove_from_container, remove_command_xy, stage_remove_item_from_container,
    RemoveFromContainerAdvance, RemoveFromContainerStaging, NOT_REACHABLE_DEFAULT_SECS,
    apply_path_filters_to_tiles,
    filter_scan_tiles_in_radius,
    basic_farmer_weight_from_runtime, blocked_by_ai_with_peer_progress,
    AiAgentBlockSource, BlockTargetClaim, GOTO_APPROACH_RAD, TRY_MOVE_NEAREST_TILE_FIRST_DEFAULT,
    goto_approach_tweaks, remove_agent_blocked_by_ai,
    collect_deadly_animal_blocked_around_for_player,
    AnimalPathPlayerCtx, BowlFillerPeer, AnimalWorld, DEADLY_ANIMAL_SEARCH_DIST,
    is_holding_weapon, is_self_best_bowl_filler, is_self_best_fire_keeper_for_obj,
    is_self_best_grave_keeper_for_obj, FireKeeperPeer, GraveKeeperPeer, pick_grave,
    consider_animals_for_goto,
    consider_drop_held_object, eatable_original_food_value, food_pickup_action_success_reset,
    food_pickup_in_container, fill_bean_bowl_if_needed, fill_berry_bowl_held_if_needed,
    FillBeanBowlAction, FillBeanBowlInput, FillBerryBowlHeldAction, FillBerryBowlHeldInput,
    FILL_BEAN_BOWL_SEARCH_DIST, FILL_BERRY_HELD_SEARCH_DIST, BOWL_OF_DRY_BEANS, DRY_BEAN_PLANTS,
    using_item_preflight, UsingItemPreflight, is_using_item_bow_on_animal,
    is_using_item_drop_is_a_use_done, is_using_item_goose_stump_speedup,
    is_using_item_use_fail, note_using_item_craft_progress, USE_BOW_AND_ARROW,
    note_raw_pie_crafted, kill_animal_needs_stand_off, mark_use_path_fail,
    is_eatable_check_again, full_pile_tiles_from_scan, nonempty_container_tiles_from_scan,
    get_or_craft_objs_from_scan, goto_path_outcome, has_bean_seeds_from_scan,
    has_onion_seeds_from_scan, has_pepper_seeds_from_scan,
    count_seeds_from_scan, has_carrot_seeds_from_scan, init_water_source_ids_from_content,
    AI_MAX_SEARCH_RADIUS,
    is_walkable, npc_think_job_rungs,
    is_walkable_with_animals, is_wound_object, ladder_profession_scan_tick,
    profession_scan_tick, feed_lambs_ladder_step, ProfessionLadderStep,
    plan_ally_up, plan_found_family, AllyUpBest, smith_blocks_mid_feed,
    StarvingCand, SKEWER, TOMATO_SPROUT, FOODSERVER_MIN_FOOD,
    hunting_profession_scan_tick, knife_stuff_profession_scan_tick,
    pull_carrot_row_profession_scan_tick, HUNTING_MID_MIN_AGE, mark_food_path_fail,
    mark_goto_path_fail, merge_path_reach_maps, next_step, next_step_consider_animals_for_player,
    npc_enqueue_get_or_craft_ex, npc_peer_count_for_kind, npc_peer_counts_by_kind,
    farm_peer_lasts_from_npc_rows, path_filters_from_player, peer_home_coords,
    peer_is_wounded_from_held_ex, pending_food_tile_still_actionable, pile_obj_id_from_content,
    plan_goto_obj, plan_is_picking_up_food, plan_profession_ladder_steps,
    ProfessionScanTickResult,
    quiver_from_clothing_snapshot, snapshot_blocked_by_ai_share,
    commit_fire_place, count_tailor_profession_from_rows, fill_up_quiver_search_radius,
    has_or_become_tailor, resolve_fire_place,
    home_cloth_stock_from_world, home_has_loom_from_world, is_fill_up_quiver_plan,
    is_old_enough_for_bow, HOME_CLOTH_COUNT_RADIUS,
    plan_clothing_craft_tick, plan_high_priority_clothing, plan_quiver_arrow_precursors,
    make_sharpie_food_from_xy, FarmAction, MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE,
    do_knife_stuff, KnifeStuffAction,
    do_baking, do_watering, fill_bake_counts_from_map_ex, fill_farm_counts_from_map_ex,
    fill_fire_food_counts_from_map, make_fire_food, BakeAction, BakeMapObj, FarmMapObj,
    FireFoodAction, FireFoodMapObj,
    person_color_from_race,
    quiver_can_add_from_slots, requeue_runtime_task_on_fail,
    resolve_sticky_food, scan_held_hungry_work_cost, scan_world_radius,
    select_runtime_sticky_craft_for_tick, self_clothing_raw_payload,
    drop_held_clears_drop_target, drop_target_uses_use_not_drop, is_dropping_drop_distance_after_try,
    is_dropping_item_goto, is_dropping_item_head, settle_pending_food_use_fail,
    smart_drop_held_from_sensors_ex, IsDropingItemGoto, IsDropingItemHead,
    apply_consider_making_food_smith_wipe, consider_making_food_do_stuff,
    consider_making_food_ear_of_corn_maker, consider_making_food_fire_food_on_extra_rabbit,
    consider_making_food_fire_food_on_few_rabbit,
    consider_making_food_raw_rabbit_count, consider_making_food_should_research,
    consider_making_food_short_crafts, consider_making_food_skip_after_enter_with_home,
    should_wipe_smith_on_consider_food, time_since_ticks_in_sec,
    MAKE_SHARPIE_FOOD_FAR_CALL_DISTANCE, TURKEY_SLICE_ON_PLATE,
    pull_carrot_row_if_needed, PullCarrotRowAction, PullCarrotRowInput,
    CORN_PLANT, DRIED_CORN, EAR_OF_CORN, PILE_DRIED_CORN, SHUCKED_CORN,
    SKINNED_RABBIT, SKEWERED_RABBIT,
    AiPathReachMaps,
    BakerProfessionRuntime, BakerTaskState, CraftAiRuntime, CraftLiveExpandOpts, CraftProfession,
    BlockedByAiShare, DropHeldSensorExtras, EnvView, FarmProfession, FarmProfessionRuntime,
    FarmTaskState,
    FireFoodProfessionRuntime, FireKeeperProfessionRuntime, GraveKeeperProfessionRuntime,
    HunterProfessionRuntime, LumberjackProfessionRuntime, CollectorProfessionRuntime,
    FoodServerProfessionRuntime,
    GotoObjPlan, GotoPathOutcome,
    IsPickingupFoodInput, IsPickingupFoodPlan, LastGotoObj, NearbyObj, NpcProfessionPeerRow,
    PlayerSnapshot, PotterProfessionRuntime, PrestigeClass, PriorityRung, ProfessionScanInput,
    ClothingCraftInput, ClothingCraftPlan, HandlingFireMapObj, HandlingGravesMapObj, ScanTile,
    StickyCraftTickChoice,
    GET_CLOSE_FIRE_MAXDIST, HOME_LOOM_RADIUS,
    ProfessionScanKind, ProfessionStickySnapshot, ReverseCraftGraph, ShepherdProfessionRuntime,
    ShortCraftLiveIntent, SmithProfessionRuntime, SteelChiselFamilyTable, StickyFoodTarget,
    BAKER_SCAN_RADIUS, DEFAULT_CRAFT_RADIUS, DEFAULT_PROFESSION_SCAN_RADIUS,
    FIRE_FOOD_HOME_RADIUS, GOTO_COLLISION_RAD, GRAVE_SEARCH_RADIUS, HANDLING_FIRE_SCAN_RADIUS,
    HUNTING_SHORTCRAFT_RADIUS, CUTTING_WOOD_SCAN_RADIUS, COLLECTING_SCAN_RADIUS,
    STARVING_SEARCH_DIST, TAILOR_SCAN_RADIUS,
    MAX_AGE,
    MIN_AGE_TO_EAT, POTTERY_SCAN_RADIUS, SHEPHERD_SHORTCRAFT_RADIUS, SMITH_SCAN_RADIUS,
};
use ol_world::{is_biome_blocking, World, DESERT, OCEAN, PASSABLE_RIVER, RIVER};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Command sink for NPC â†’ same channel as human clients ([`PlayerWriteInterface`]).
struct NpcWriteTx<'a>(&'a tokio::sync::mpsc::Sender<NetIntent>);

impl CommandSink for NpcWriteTx<'_> {
    fn push(&mut self, intent: NetIntent) -> bool {
        self.0.try_send(intent).is_ok()
    }
}

/// Haxe `itemToCraft.maxSearchRadius = 60` at `doTimeStuffHelper` L433, used by
/// `GetOrCraftItem` → `craftItem` (killAnimal `getWeapon` L5815).
// Haxe: AiBase.doTimeStuffHelper L433; GetOrCraftItem L6198
const NPC_GET_OR_CRAFT_SEARCH_RADIUS: i32 = AI_MAX_SEARCH_RADIUS;

/// Haxe `isClose` d=1 (squared Euclidean). Diagonal is not close enough to USE/DROP.
#[inline]
fn npc_is_close_action(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    let dx = px - tx;
    let dy = py - ty;
    dx * dx + dy * dy <= 1
}

/// Where to stand before a DROP/USE. A diagonal neighbor is chebyshev-1 but
/// not Haxe `isClose`, so an immediate DROP is rejected and the held rope stays.
fn npc_action_stand_tile(px: i32, py: i32, tx: i32, ty: i32) -> (i32, i32) {
    if npc_is_close_action(px, py, tx, ty) {
        return (tx, ty);
    }
    let dx = tx - px;
    let dy = ty - py;
    if dx.abs() == 1 && dy.abs() == 1 {
        return (tx, py);
    }
    (tx, ty)
}

/// Haxe `WorldMap.transformX/Y` — nearest wrapped image of `(gx,gy)` next to the body.
///
/// Scan tiles may store `y=2` while the player is at `y=484` on a 500-tall wrap
/// map; walking that raw goal goes the long way (~482 tiles) and never `isClose`.
// Haxe: WorldMap.transformX/Y L1582–1602; AiHelper.gotoAdv uses rx,ry
fn npc_wrap_goal(world: &World, px: i32, py: i32, gx: i32, gy: i32) -> (i32, i32) {
    if !world.wrap || world.width_tiles <= 0 || world.height_tiles <= 0 {
        return (gx, gy);
    }
    let w = world.width_tiles;
    let h = world.height_tiles;
    let shift = |from: i32, to: i32, size: i32| -> i32 {
        let mut d = to - from;
        let half = size / 2;
        if d > half {
            d -= size;
        } else if d < -half {
            d += size;
        }
        from + d
    };
    (shift(px, gx, w), shift(py, gy, h))
}

/// Arrival action staged on a sticky MOVE (Haxe `useTarget` / `dropTarget`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StickyArrive {
    /// Pure walk (explore / Goto). Hold the path; no USE/DROP on arrival.
    None,
    /// Haxe `useHeldObjOnTarget` / `isUsingItem`.
    Use,
    /// Haxe `dropTarget` / `isDropingItem` (includes empty-hand pickup).
    Drop,
}

impl StickyArrive {
    fn from_flags(pending_use: bool, pending_drop: bool) -> Self {
        if pending_use {
            Self::Use
        } else if pending_drop {
            Self::Drop
        } else {
            Self::None
        }
    }
}

/// After hunger/escape, Haxe `isUsingItem`/`isDropingItem` return true while
/// still moving so craft does not send a replacement MOVE or USE (USE cancels path).
// Haxe: AiBase.isUsingItem L9013; isDropingItem L8428
fn npc_skip_craft_while_sticky_moving(
    moving: bool,
    sticky_valid: bool,
    sticky_arrive: StickyArrive,
) -> bool {
    moving && sticky_valid && sticky_arrive != StickyArrive::None
}

/// Haxe `CalculateQuadDistanceToObject` vs 25 — close use runs before feeding.
// Haxe: AiBase.doTimeStuffHelper L500–510
#[inline]
fn npc_haxe_close_use_prio(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    let dx = px - tx;
    let dy = py - ty;
    dx * dx + dy * dy < 25
}

#[inline]
fn npc_holding_player(held_id: i32, holding_player_id: i32) -> bool {
    holding_player_id != 0 || held_id < 0
}

/// Haxe `dropHeldObject`: empty ground near the player, not the tile underfoot.
/// DROP on the player's own tile is in range but does not clear the hand
/// (live 0.3.37: held 31 stayed through `craft @` the feet tile).
/// Search grows by 1 up to 40. A blocked feet tile is not a drop target.
// Haxe: AiBase.dropHeldObject L5566–5592; isDropingItem L8456
fn npc_empty_drop_xy(world: &World, px: i32, py: i32) -> (i32, i32) {
    fn open_tile(world: &World, x: i32, y: i32) -> bool {
        if world.get_object(x, y) != 0 {
            return false;
        }
        !is_biome_blocking(world.get_biome(x, y), world.get_floor(x, y) as i32)
    }
    for r in 1i32..=40 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let x = px + dx;
                let y = py + dy;
                if open_tile(world, x, y) {
                    return (x, y);
                }
            }
        }
    }
    (px, py)
}

/// Haxe `craftItemHelper` drops a held berry before `dropTarget =` the rope.
/// A direct 59+124 / 59+131 DropAt was swapping the food onto the rope, so the
/// hand stayed full and `switchCloths` never saw 128.
// Haxe: AiBase.craftItemHelper L7114; considerDropHeldObject L5198; dropHeldObject L5566
fn npc_drop_held_before_loose_pickup(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    intent: ShortCraftLiveIntent,
) -> ShortCraftLiveIntent {
    let ShortCraftLiveIntent::DropAt { x, y } = intent else {
        return intent;
    };
    let held_base = content.resolve_base_id(held_id);
    if held_base <= 0 {
        return intent;
    }
    // Rope and reed are the skirt halves. Empty-dropping them leaves 124 on
    // the ground and the hand empty, so 59+124 never runs.
    if held_base == 59 || held_base == 124 {
        return intent;
    }
    let tile = content.resolve_base_id(world.get_object(x, y));
    if tile == held_base || npc_clothing_slot(content, tile).is_some() {
        return intent;
    }
    if !consider_drop_held_object(held_base, px, py, home_x, home_y, x, y) {
        return intent;
    }
    let (dx, dy) = npc_empty_drop_xy(world, px, py);
    ShortCraftLiveIntent::DropAt { x: dx, y: dy }
}

/// Park the rope/reed DROP while food is put on an empty tile.
///
/// The hand-clear DROP must be labeled `drop_held_clear` so arrival restores
/// this sticky. Otherwise the next think picks the berry back up and
/// `switchCloths` never sees rope or 128.
// Haxe: craftItemHelper L7114 considerDropHeldObject; isDropingItem L497 before food
fn npc_park_actor_behind_food_drop(
    st: &mut NpcProfessionState,
    content: &ContentDb,
    world: &World,
    original: ShortCraftLiveIntent,
    rewritten: ShortCraftLiveIntent,
) -> ShortCraftLiveIntent {
    let (
        ShortCraftLiveIntent::DropAt { x: ox, y: oy },
        ShortCraftLiveIntent::DropAt { x, y },
    ) = (original, rewritten)
    else {
        return rewritten;
    };
    if ox == x && oy == y {
        return rewritten;
    }
    let expected = sticky_parent_id(content, world.get_object(ox, oy));
    st.resume_drop = Some(NpcStickyMove {
        gx: ox,
        gy: oy,
        expected_parent_id: expected,
        use_actor_parent: 0,
        pending_use: false,
        pending_drop: true,
        label: npc_drop_arrive_label(content, expected, ox, oy, true),
        move_from: None,
    });
    st.hand_clear_pending = true;
    rewritten
}

/// Haxe `isDropingItem` is always before `isFeedingChild`; close `isUsingItem` (quad < 25) too.
// Haxe: AiBase.doTimeStuffHelper L497 / L500–510 then L557 isFeedingChild
fn npc_skip_feed_for_sticky(
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    sticky_valid: bool,
    arrive: StickyArrive,
) -> bool {
    if !sticky_valid {
        return false;
    }
    match arrive {
        StickyArrive::Drop => true,
        StickyArrive::Use => npc_haxe_close_use_prio(px, py, gx, gy),
        StickyArrive::None => false,
    }
}

/// Early sticky return: `isDropingItem` always; `isUsingItem` only when quad < 25.
// Haxe: AiBase.doTimeStuffHelper L497 / L500–510
fn npc_sticky_early_return(arrive: StickyArrive, px: i32, py: i32, gx: i32, gy: i32) -> bool {
    match arrive {
        StickyArrive::Drop => true,
        StickyArrive::Use => npc_haxe_close_use_prio(px, py, gx, gy),
        StickyArrive::None => false,
    }
}

/// What to do on sticky arrival (Haxe `isUsingItem` / `isDropingItem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StickyActPlan {
    /// Path still running — Haxe `if (isMoving()) return true`.
    WaitUntilStopped,
    /// Walk toward the tile (`distance > 1` → `gotoObj`).
    Walk,
    /// Holding a baby: `dropPlayer(myPlayer.x, myPlayer.y)` then next think USE/DROP.
    // Haxe: AiBase.isUsingItem L9040–9045; isPickingupFood L8657–8663
    DropHeldPlayerAtFeet,
    /// Holding an object while `useActor.id == 0`: drop to empty the hand, then USE.
    // Haxe: AiBase.isUsingItem L9048–9051
    DropHeldForEmptyHand,
    DropTarget,
    UseTarget,
}

/// Haxe `isUsingItem` L8910–8998 before goto/use.
fn npc_using_item_head_live(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    held_id: i32,
    held_uses: i32,
    home_x: i32,
    home_y: i32,
    sticky: &NpcStickyMove,
) -> Option<(NpcActivityKind, String, u32)> {
    let world_id = world.get_object(sticky.gx, sticky.gy);
    let world_parent = sticky_parent_id(content, world_id);
    let contained_n = world
        .get_helper(sticky.gx, sticky.gy)
        .map(|h| h.contained.len() as i32)
        .unwrap_or(0);
    let consider = consider_drop_held_object(
        held_id,
        px,
        py,
        home_x,
        home_y,
        sticky.gx,
        sticky.gy,
    );
    let num_uses = content
        .get(content.resolve_base_id(held_id))
        .map(|d| d.num_uses)
        .unwrap_or(0);
    match using_item_preflight(
        st.use_is_drop_in_container,
        contained_n,
        sticky.expected_parent_id,
        world_parent,
        content.resolve_base_id(held_id),
        sticky.use_actor_parent,
        consider,
        held_uses,
        num_uses,
    ) {
        UsingItemPreflight::Continue => None,
        UsingItemPreflight::AbortContainedBusy => {
            st.path_reach.add_not_reachable(sticky.gx, sticky.gy, 90.0);
            npc_cancle_use(st);
            Some((
                NpcActivityKind::Think,
                format!("use_contained_abort @{},{}", sticky.gx, sticky.gy),
                100,
            ))
        }
        UsingItemPreflight::CancelTargetChanged => {
            npc_cancle_use(st);
            None
        }
        UsingItemPreflight::ConsiderDropHeld => {
            let (dx, dy) = npc_empty_drop_xy(world, px, py);
            if npc_drop_at(intent_tx, conn_id, dx, dy, None) {
                Some((
                    NpcActivityKind::Craft,
                    format!("use_consider_drop held={}", held_id),
                    400,
                ))
            } else {
                Some((NpcActivityKind::Think, "use_consider_drop".into(), 100))
            }
        }
        UsingItemPreflight::CancelWrongActor => {
            npc_cancle_use(st);
            let (dx, dy) = npc_empty_drop_xy(world, px, py);
            let _ = npc_drop_at(intent_tx, conn_id, dx, dy, None);
            None
        }
        UsingItemPreflight::NeedFillBerry => {
            let bush = npc_closest_parent_cheb(
                &npc_scan_fill_once(world, content, px, py, FILL_BERRY_HELD_SEARCH_DIST),
                px,
                py,
                30,
                FILL_BERRY_HELD_SEARCH_DIST,
            )
            .or_else(|| {
                npc_closest_parent_cheb(
                    &npc_scan_fill_once(world, content, px, py, FILL_BERRY_HELD_SEARCH_DIST),
                    px,
                    py,
                    391,
                    FILL_BERRY_HELD_SEARCH_DIST,
                )
            });
            let act = fill_berry_bowl_held_if_needed(&FillBerryBowlHeldInput {
                held_id,
                held_uses,
                held_num_uses: num_uses,
                bush,
            });
            match act {
                FillBerryBowlHeldAction::UseHeldOnBush { x, y, bush_id } => {
                    if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                        Some((
                            NpcActivityKind::Craft,
                            format!("use_fill_berry {bush_id} @{},{}", x, y),
                            400,
                        ))
                    } else {
                        None
                    }
                }
                FillBerryBowlHeldAction::None => {
                    npc_cancle_use(st);
                    let (dx, dy) = npc_empty_drop_xy(world, px, py);
                    let _ = npc_drop_at(intent_tx, conn_id, dx, dy, None);
                    None
                }
            }
        }
        UsingItemPreflight::NeedFillBean => {
            let tiles = npc_scan_fill_once(world, content, px, py, FILL_BEAN_BOWL_SEARCH_DIST);
            let plant = npc_closest_parent_cheb(
                &tiles,
                px,
                py,
                DRY_BEAN_PLANTS,
                FILL_BEAN_BOWL_SEARCH_DIST,
            )
            .map(|(x, y, _)| (x, y));
            let bowl = npc_closest_parent_cheb(
                &tiles,
                px,
                py,
                BOWL_OF_DRY_BEANS,
                FILL_BEAN_BOWL_SEARCH_DIST,
            );
            let act = fill_bean_bowl_if_needed(&FillBeanBowlInput {
                held_id,
                held_uses,
                held_num_uses: num_uses,
                green_beans: false,
                only_fill_held: false,
                count_dry_beans: npc_count_parent_cheb(
                    &tiles,
                    px,
                    py,
                    BOWL_OF_DRY_BEANS,
                    FILL_BEAN_BOWL_SEARCH_DIST,
                ) + npc_count_parent_cheb(
                    &tiles,
                    px,
                    py,
                    DRY_BEAN_PLANTS,
                    FILL_BEAN_BOWL_SEARCH_DIST,
                ),
                plant_xy: plant,
                bowl_xy: bowl.map(|(x, y, _)| (x, y)),
                bowl_uses: 1,
                bowl_num_uses: 1,
                is_best_bowl_filler: true,
            });
            match act {
                FillBeanBowlAction::UseHeldOnPlant { x, y, plant_id } => {
                    if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                        Some((
                            NpcActivityKind::Craft,
                            format!("use_fill_bean {plant_id} @{},{}", x, y),
                            400,
                        ))
                    } else {
                        npc_cancle_use(st);
                        let (dx, dy) = npc_empty_drop_xy(world, px, py);
                        let _ = npc_drop_at(intent_tx, conn_id, dx, dy, None);
                        Some((NpcActivityKind::Think, "use_fill_bean_fail".into(), 100))
                    }
                }
                FillBeanBowlAction::None
                | FillBeanBowlAction::PickupBowl { .. }
                | FillBeanBowlAction::GetClayBowl => {
                    npc_cancle_use(st);
                    let (dx, dy) = npc_empty_drop_xy(world, px, py);
                    let _ = npc_drop_at(intent_tx, conn_id, dx, dy, None);
                    Some((NpcActivityKind::Think, "use_fill_bean_drop".into(), 100))
                }
            }
        }
    }
}

fn npc_scan_fill_once(
    world: &World,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    r: i32,
) -> Vec<ScanTile> {
    scan_world_radius(world, Some(content), cx, cy, r)
}

fn npc_plan_sticky_arrive(
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    moving: bool,
    holding_player: bool,
    held_id: i32,
    use_actor_parent: i32,
    arrive: StickyArrive,
) -> StickyActPlan {
    if arrive == StickyArrive::None {
        return if moving || npc_is_close_action(px, py, gx, gy) {
            StickyActPlan::WaitUntilStopped
        } else {
            StickyActPlan::Walk
        };
    }
    if npc_is_close_action(px, py, gx, gy) {
        if moving {
            return StickyActPlan::WaitUntilStopped;
        }
        if holding_player {
            return StickyActPlan::DropHeldPlayerAtFeet;
        }
        // Haxe: isHoldingObject && useActor.id == 0 → dropHeldObject(0)
        if arrive == StickyArrive::Use && use_actor_parent == 0 && held_id > 0 {
            return StickyActPlan::DropHeldForEmptyHand;
        }
        return match arrive {
            StickyArrive::Drop => StickyActPlan::DropTarget,
            StickyArrive::Use => StickyActPlan::UseTarget,
            StickyArrive::None => StickyActPlan::WaitUntilStopped,
        };
    }
    StickyActPlan::Walk
}

fn npc_held_name(content: &ContentDb, held_id: i32) -> String {
    content
        .get(held_id)
        .map(|o| o.name.clone())
        .unwrap_or_default()
}

fn npc_holding_weapon(content: &ContentDb, held_id: i32) -> bool {
    is_holding_weapon(held_id, &npc_held_name(content, held_id))
}

fn npc_is_wounded(content: &ContentDb, held_id: i32) -> bool {
    is_wound_object(content, held_id)
}

/// Haxe CreateCollisionChunk `isAnimalDeadlyForMe` player context.
// Haxe: AiHelper.CreateCollisionChunkHelper ~1508
fn npc_animal_path_ctx(content: &ContentDb, p: &PlayerSnapshot) -> AnimalPathPlayerCtx {
    AnimalPathPlayerCtx {
        holding_weapon: npc_holding_weapon(content, p.held_id),
        person_color: content.person_color(p.display_object_id),
    }
}

/// Haxe `getBestAiForObjByProfession('BowlFiller', home)` from published views.
// Haxe: AiBase.makePopcornIfNeeded ~4307
fn npc_is_best_bowl_filler(
    self_p: &PlayerSnapshot,
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    profession_state: &HashMap<u64, NpcProfessionState>,
    content: &ContentDb,
) -> bool {
    let Ok(views) = player_views.read() else {
        return true;
    };
    let (hx, hy) = peer_home_coords(Some((self_p.home_x, self_p.home_y)), self_p.x, self_p.y);
    let mut peers = Vec::new();
    for snap in views.values() {
        let (ohx, ohy) = peer_home_coords(Some((snap.home_x, snap.home_y)), snap.x, snap.y);
        let pst = profession_state.get(&snap.conn_id);
        let has = snap.is_last_baker
            || pst
                .map(|s| s.baker_rt.is_last_baker || s.baker_rt.is_assigned_baker)
                .unwrap_or(false);
        let dx = (snap.x - hx) as f32;
        let dy = (snap.y - hy) as f32;
        peers.push(BowlFillerPeer {
            p_id: snap.p_id,
            quad_dist_to_obj: dx * dx + dy * dy,
            deleted: snap.deleted,
            age: snap.age,
            is_wounded: npc_is_wounded(content, snap.held_id),
            food_store: snap.food,
            same_home: ohx == hx && ohy == hy,
            has_bowl_filler: has,
        });
    }
    is_self_best_bowl_filler(self_p.p_id, &peers, MIN_AGE_TO_EAT, MAX_AGE)
}

/// Haxe `getBestAiForObjByProfession('FIREKEEPER', home|firePlace)` from published views.
// Haxe: AiBase.isHandlingFire ~1100 / ~1134; profession['FIREKEEPER'] > 0
fn npc_is_best_fire_keeper(
    self_p: &PlayerSnapshot,
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    profession_state: &HashMap<u64, NpcProfessionState>,
    content: &ContentDb,
    obj_x: i32,
    obj_y: i32,
) -> bool {
    let Ok(views) = player_views.read() else {
        return true;
    };
    let (hx, hy) = peer_home_coords(Some((self_p.home_x, self_p.home_y)), self_p.x, self_p.y);
    let mut peers = Vec::new();
    for snap in views.values() {
        let (ohx, ohy) = peer_home_coords(Some((snap.home_x, snap.home_y)), snap.x, snap.y);
        let pst = profession_state.get(&snap.conn_id);
        let has = pst
            .map(|s| s.fire_keeper_rt.weight > 0.0)
            .unwrap_or(false);
        let dx = (snap.x - obj_x) as f32;
        let dy = (snap.y - obj_y) as f32;
        peers.push(FireKeeperPeer {
            p_id: snap.p_id,
            quad_dist_to_obj: dx * dx + dy * dy,
            deleted: snap.deleted,
            age: snap.age,
            is_wounded: npc_is_wounded(content, snap.held_id),
            food_store: snap.food,
            same_home: ohx == hx && ohy == hy,
            has_fire_keeper: has,
        });
    }
    is_self_best_fire_keeper_for_obj(self_p.p_id, &peers, MIN_AGE_TO_EAT, MAX_AGE)
}

/// Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)` from published views.
// Haxe: AiBase.isHandlingGraves ~1527; profession['GRAVEKEEPER'] > 0
fn npc_is_best_grave_keeper(
    self_p: &PlayerSnapshot,
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    profession_state: &HashMap<u64, NpcProfessionState>,
    content: &ContentDb,
    obj_x: i32,
    obj_y: i32,
) -> bool {
    let Ok(views) = player_views.read() else {
        return true;
    };
    let (hx, hy) = peer_home_coords(Some((self_p.home_x, self_p.home_y)), self_p.x, self_p.y);
    let mut peers = Vec::new();
    for snap in views.values() {
        let (ohx, ohy) = peer_home_coords(Some((snap.home_x, snap.home_y)), snap.x, snap.y);
        let pst = profession_state.get(&snap.conn_id);
        let has = pst
            .map(|s| s.grave_keeper_rt.weight > 0.0)
            .unwrap_or(false);
        let dx = (snap.x - obj_x) as f32;
        let dy = (snap.y - obj_y) as f32;
        peers.push(GraveKeeperPeer {
            p_id: snap.p_id,
            quad_dist_to_obj: dx * dx + dy * dy,
            deleted: snap.deleted,
            age: snap.age,
            is_wounded: npc_is_wounded(content, snap.held_id),
            food_store: snap.food,
            same_home: ohx == hx && ohy == hy,
            has_grave_keeper: has,
        });
    }
    is_self_best_grave_keeper_for_obj(self_p.p_id, &peers, MIN_AGE_TO_EAT)
}

fn npc_pick_grave_xy(
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
    last_grave: Option<(i32, i32, i32)>,
) -> Option<(i32, i32)> {
    let map: Vec<HandlingGravesMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| HandlingGravesMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
            floor_id: t.floor_id,
            contained_count: t.contained_count,
            owner_account: t.owner_account,
        })
        .collect();
    pick_grave(&map, player_x, player_y, GRAVE_SEARCH_RADIUS, last_grave)
        .map(|(_, x, y, _, _)| (x, y))
}

/// Haxe `GetCloseDeadlyPlayerHelper` candidates from published snapshots.
// Haxe: AiHelper.GetCloseDeadlyPlayerHelper
fn npc_deadly_player_candidates(
    self_p: &PlayerSnapshot,
    views: &HashMap<u64, PlayerSnapshot>,
    content: &ContentDb,
) -> Vec<DeadlyPlayerCandidate> {
    views
        .values()
        .filter(|o| o.p_id != self_p.p_id)
        .map(|o| {
            let holding = npc_holding_weapon(content, o.held_id);
            let name = npc_held_name(content, o.held_id);
            let attacked = self_p.last_attacked_player_id == o.p_id
                || self_p.last_player_attacked_me_id == o.p_id
                || o.last_attacked_player_id == self_p.p_id
                || o.last_player_attacked_me_id == self_p.p_id;
            DeadlyPlayerCandidate {
                p_id: o.p_id,
                x: o.x,
                y: o.y,
                deleted: o.deleted,
                age: o.age,
                angry_time: o.angry_time,
                lost_combat_prestige: o.lost_combat_prestige,
                is_cursed: o.is_cursed,
                is_ai: o.ai_controlled || o.email.contains("npc"),
                holding_weapon: holding,
                held_is_bloody: name.to_ascii_lowercase().contains("bloody"),
                exiled_by_observer_leaders: false,
                is_friendly: !attacked,
            }
        })
        .collect()
}

/// Haxe `doTimeStuffHelper` GetCloseDeadly* + fill_live_sensors.
// Haxe: AiBase.doTimeStuffHelper L486–492
fn npc_fill_live_sensor_input(
    p: &PlayerSnapshot,
    content: &ContentDb,
    views: &HashMap<u64, PlayerSnapshot>,
    animals: Option<&AnimalWorld>,
    st: &NpcProfessionState,
    nearby_food: bool,
) -> LiveSensorInput {
    npc_fill_live_sensor_input_ex(p, content, views, animals, st, nearby_food, &[])
}

/// Same as [`npc_fill_live_sensor_input`] with nearby tiles for `hasWeaponClose` ground search.
// Haxe: AiBase.hasWeaponClose L5716–5742
fn npc_fill_live_sensor_input_ex(
    p: &PlayerSnapshot,
    content: &ContentDb,
    views: &HashMap<u64, PlayerSnapshot>,
    animals: Option<&AnimalWorld>,
    st: &NpcProfessionState,
    nearby_food: bool,
    nearby: &[NearbyObj],
) -> LiveSensorInput {
    let deadly_animal = animals.and_then(|aw| {
        aw.get_close_deadly_animal(p.x, p.y, DEADLY_ANIMAL_SEARCH_DIST)
            .map(|d| (d.x, d.y, d.dist_quad))
    });
    let cands = npc_deadly_player_candidates(p, views, content);
    let deadly_player = get_close_deadly_player(
        p.x,
        p.y,
        p.angry_time,
        p.home_x,
        p.home_y,
        DEADLY_PLAYER_SEARCH_DIST_AI,
        &cands,
    )
    .map(|d| (d.x, d.y, d.dist, d.angry_time));
    let has_mother = p.held_by > 0;
    let mut input = LiveSensorInput {
        held_id: p.held_id,
        food: p.food,
        food_max: p.food_max,
        was_hungry: st.was_hungry,
        age: p.age,
        heat: p.heat,
        has_mother,
        follow_player: has_mother || p.ai_follow_p_id > 0,
        ordered_follow: p.ai_follow_p_id > 0,
        deadly_animal,
        deadly_player,
        nearby_food,
        holding_weapon: npc_holding_weapon(content, p.held_id),
        is_wounded: npc_is_wounded(content, p.held_id) && !p.is_hidden_wound,
        did_not_reach_food: st.food_goto.did_not_reach_food,
        held_by_other: p.held_by > 0 && p.age < MIN_AGE_TO_EAT,
        has_craft_queue: st.craft_rt.should_continue_unfinished()
            || !st.craft_rt.crafting_tasks.is_empty(),
        handling_temperature: st.handling_temperature
            || is_super_hot_for_person(p.heat, content.person_color(p.display_object_id))
            || is_super_cold_for_person(p.heat, content.person_color(p.display_object_id)),
        removing_container: st.remove_from_container.is_some(),
        combat_target: deadly_player.is_some(),
        ..Default::default()
    };
    let weapon_tiles: Vec<(i32, i32, i32, bool)> = nearby
        .iter()
        .map(|o| (o.id, o.x, o.y, false))
        .collect();
    input.has_weapon_close = has_weapon_close(
        true,
        input.is_wounded,
        p.age,
        MIN_AI_AGE_FOR_COMBAT,
        is_bloody_weapon(p.held_id),
        p.held_id,
        AttackPlayerClothing::from_ids(&p.clothing),
        &weapon_tiles,
        p.x,
        p.y,
    );
    let mut sticky = ProfessionStickySnapshot::from_runtimes_ex(
        &st.farm_rt,
        &st.smith_rt,
        &st.baker_rt,
        Some(&st.shepherd_rt),
        Some(&st.pottery_rt),
        Some(&st.fire_rt),
        Some(&st.fire_keeper_rt),
        Some(&st.grave_keeper_rt),
        Some(&st.hunter_rt),
        Some(&st.lumberjack_rt),
        Some(&st.collector_rt),
        Some(&st.foodserver_rt),
        p.age,
    );
    sticky.tailor_assigned = p.is_assigned_tailor;
    sticky.tailor_last = p.is_last_tailor || st.last_is_tailor;
    apply_job_flags_to_live_input(&mut input, &sticky);
    input
}

/// Haxe `WorldMap.calculateRandomFloat` / `calculateRandomInt` for escape jitter.
struct NpcEscapeRng;

impl EscapeRand for NpcEscapeRng {
    fn random_float(&mut self) -> f32 {
        rand::random::<f32>()
    }
    fn random_int(&mut self, max_inclusive: i32) -> i32 {
        if max_inclusive < 0 {
            return 0;
        }
        rand::thread_rng().gen_range(0..=max_inclusive)
    }
}

/// Haxe `GlobalPlayerInstance.isBlocked`.
// Haxe: GlobalPlayerInstance.isBlocked L6201–6212
fn npc_is_blocked(world: &World, content: &ContentDb, held_id: i32, tx: i32, ty: i32) -> bool {
    let id = world.get_object(tx, ty);
    if id != 0 {
        if let Some(def) = content.get(id) {
            if def.blocks_walking {
                return true;
            }
        }
    }
    let biome = world.get_biome(tx, ty);
    let held_parent = content
        .dummy_parent
        .get(&held_id)
        .copied()
        .unwrap_or(held_id);
    if content.is_boat.contains(&held_id) || content.is_boat.contains(&held_parent) {
        if matches!(biome, OCEAN | PASSABLE_RIVER | RIVER) {
            return false;
        }
    }
    is_biome_blocking(biome, world.get_floor(tx, ty) as i32)
}

/// Haxe `AiHelper.IsDangerous` around an escape candidate (map animals + hostile path).
// Haxe: AiHelper.IsDangerousHelper L1058–1074; AiBase.escape L6561
fn npc_escape_tile_is_dangerous(
    world: &World,
    content: &ContentDb,
    animal_tiles: &[(i32, i32)],
    hostile: &[(i32, i32)],
    tx: i32,
    ty: i32,
) -> bool {
    if is_dangerous_near(tx, ty, ESCAPE_IS_DANGEROUS_RADIUS, animal_tiles, hostile) {
        return true;
    }
    let r = ESCAPE_IS_DANGEROUS_RADIUS;
    for y in (ty - r)..(ty + r) {
        for x in (tx - r)..(tx + r) {
            let id = world.get_object(x, y);
            if id == 0 {
                continue;
            }
            if let Some(def) = content.get(id) {
                if def.is_animal() && def.deadly_distance > 0.0 {
                    return true;
                }
            }
        }
    }
    false
}

/// Haxe `escape(animal, deadlyPlayer)` — L6493–6599 (caller already gated `didNotReachFood < 5`).
// Haxe: AiBase.doTimeStuff L401–404; doTimeStuffHelper L492; escape L6501–6599
fn npc_try_escape_now(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &std::sync::RwLock<World>,
    content: &ContentDb,
    animals: &std::sync::RwLock<AnimalWorld>,
    player_views: &std::sync::RwLock<HashMap<u64, PlayerSnapshot>>,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
) -> Option<(NpcActivityKind, String, u32)> {
    if !ol_ai::should_attempt_escape(st.food_goto.did_not_reach_food) {
        return None;
    }
    let animals_g = animals.read().ok();
    let views_g = player_views.read().ok()?;
    let nearby = world
        .read()
        .ok()
        .map(|w| collect_nearby(&w, p.x, p.y, HAS_WEAPON_CLOSE_SEARCH))
        .unwrap_or_default();
    let input = npc_fill_live_sensor_input_ex(
        p,
        content,
        &views_g,
        animals_g.as_deref(),
        st,
        false,
        &nearby,
    );
    let bundle = fill_live_sensors(&input);
    st.was_hungry = bundle.is_hungry;

    let close_animal = animals_g
        .as_deref()
        .and_then(|aw| aw.get_close_deadly_animal(p.x, p.y, DEADLY_ANIMAL_SEARCH_DIST));
    let animal_xy_id = close_animal.map(|d| {
        let oid = animals_g
            .as_deref()
            .and_then(|aw| {
                aw.animals
                    .iter()
                    .find(|a| a.id == d.id)
                    .map(|a| a.map_object_id())
            })
            .unwrap_or_else(|| d.kind.object_id());
        (d.x, d.y, oid)
    });
    let player_active = input
        .deadly_player
        .is_some_and(|(_, _, _, angry)| angry <= ESCAPE_ANGRY_TIME_IGNORE);
    // Haxe: no animal and (null/angry) player → return false *before* animalTarget.
    if animal_xy_id.is_none() && !player_active {
        return None;
    }
    // Haxe: `food_store < -1` return false before animalTarget.
    if input.food < ESCAPE_FOOD_CRIT_SKIP {
        return None;
    }
    let killable = animal_xy_id
        .map(|(_, _, id)| content.find_transition(BOW_AND_ARROW, id).is_some())
        .unwrap_or(false);
    // Haxe L6503: assign even when hunt-skip / hasWeaponClose later return false.
    st.animal_target =
        escape_maybe_assign_animal_target(st.animal_target, animal_xy_id, killable);
    if skip_escape_for_hunt(input.holding_weapon, input.is_wounded, input.age) {
        return None;
    }
    if bundle.escape_threat == EscapeThreat::None {
        return None;
    }

    let (threat_tx, threat_ty) = match bundle.escape_threat {
        EscapeThreat::Animal => input
            .deadly_animal
            .map(|(x, y, _)| (x, y))
            .unwrap_or((p.x, p.y)),
        EscapeThreat::Player => input
            .deadly_player
            .map(|(x, y, _, _)| (x, y))
            .unwrap_or((p.x, p.y)),
        EscapeThreat::None => (p.x, p.y),
    };
    let description = match bundle.escape_threat {
        EscapeThreat::Player => input
            .deadly_player
            .and_then(|(x, y, _, _)| {
                views_g.values().find(|v| v.x == x && v.y == y).map(|v| {
                    v.display_name
                        .split_whitespace()
                        .next()
                        .unwrap_or("player")
                        .to_string()
                })
            })
            .unwrap_or_else(|| "player".into()),
        EscapeThreat::Animal => animal_xy_id
            .and_then(|(_, _, id)| {
                content.get(id).map(|d| {
                    if !d.description.is_empty() {
                        d.description.clone()
                    } else {
                        d.name.clone()
                    }
                })
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "animal".into()),
        EscapeThreat::None => "threat".into(),
    };
    if let Some(line) = escape_debug_say(
        p.ai_debug_say,
        &description,
        st.food_goto.did_not_reach_food,
    ) {
        let _ = npc_say_raw(intent_tx, conn_id, "SAY", &line);
    }

    let animal_tiles: Vec<(i32, i32)> = animals_g
        .as_deref()
        .map(|aw| {
            aw.animals
                .iter()
                .filter(|a| a.kind.is_deadly_for_ai())
                .map(|a| (a.x, a.y))
                .collect()
        })
        .unwrap_or_default();
    let hostile: Vec<(i32, i32)> = st.path_reach.hostile_path.keys().copied().collect();
    let had_use = st.sticky_move.as_ref().is_some_and(|s| s.pending_use);
    let use_xy = st.sticky_move.as_ref().map(|s| (s.gx, s.gy));
    let food_xy = st.food_goto.sticky_food.as_ref().map(|f| (f.x, f.y));
    let prev_escape = st.escape_target;
    let seq = npc_client_move_seq(p.done_moving_seq);
    let try_nearest = st.try_move_nearest_tile_first;
    let did_not_reach_food = st.food_goto.did_not_reach_food;
    let w = world.read().unwrap();
    let mut rng = NpcEscapeRng;
    let pick = pick_escape_tile(
        p.x,
        p.y,
        threat_tx,
        threat_ty,
        ESCAPE_DIST,
        &mut rng,
        |tx, ty| npc_is_blocked(&w, content, p.held_id, tx, ty),
        |tx, ty| npc_escape_tile_is_dangerous(&w, content, &animal_tiles, &hostile, tx, ty),
        |tx, ty| {
            npc_try_walk_to_ex(
                intent_tx,
                &w,
                content,
                conn_id,
                p.x,
                p.y,
                tx,
                ty,
                p.food,
                did_not_reach_food,
                None,
                false,
                try_nearest,
                false,
                seq,
            )
        },
    );
    let effects = escape_side_effects(had_use, food_xy.is_some(), prev_escape.is_some());
    if effects.increment_did_not_reach_food {
        st.food_goto.did_not_reach_food += 1.0;
    }
    if let Some((x, y)) = use_xy {
        if had_use {
            st.path_reach.add_object_with_hostile_path(x, y);
        }
    }
    if let Some((x, y)) = food_xy {
        st.path_reach.add_object_with_hostile_path(x, y);
    }
    if let Some((x, y)) = prev_escape {
        st.path_reach.add_object_with_hostile_path(x, y);
    }
    if effects.cancel_use {
        npc_cancle_use(st);
    }
    if effects.clear_food_target {
        st.food_goto.sticky_food = None;
    }
    if effects.clear_craft_trans {
        st.craft_rt.item.clear_trans();
    }
    // Haxe: always `escapeTarget = newEscapetarget`; always `return true`.
    st.escape_target = Some((pick.tx, pick.ty));
    Some((
        NpcActivityKind::Combat,
        format!(
            "escape_{:?} @{},{} done={}",
            bundle.escape_threat, pick.tx, pick.ty, pick.goto_done
        ),
        250,
    ))
}

/// Haxe `doStuff && attackPlayer(playerTarget)` — getWeapon / stand-off / KILL.
// Haxe: AiBase.doTimeStuffHelper ~591
fn npc_run_attack_player(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    tick: u64,
    views: &HashMap<u64, PlayerSnapshot>,
    tiles: &[ScanTile],
) -> Option<(NpcActivityKind, String, u32)> {
    let cands = npc_deadly_player_candidates(p, views, content);
    let deadly = get_close_deadly_player(
        p.x,
        p.y,
        p.angry_time,
        p.home_x,
        p.home_y,
        DEADLY_PLAYER_SEARCH_DIST_AI,
        &cands,
    );
    let has_red = p.clothing.iter().any(|&id| id == DEVIL_MASK_ID);
    let has_blue = p.clothing.iter().any(|&id| id == GOBLIN_MASK_ID);
    let pt_cands: Vec<PlayerTargetCandidate> = views
        .values()
        .map(|o| {
            let dx = o.x - p.home_x;
            let dy = o.y - p.home_y;
            PlayerTargetCandidate {
                p_id: o.p_id,
                x: o.x,
                y: o.y,
                deleted: o.deleted,
                age: o.age,
                is_same_family: (p.home_x != 0 || p.home_y != 0)
                    && o.home_x == p.home_x
                    && o.home_y == p.home_y,
                is_ally: o.ai_follow_p_id == p.p_id || p.ai_follow_p_id == o.p_id,
                is_top_leader: o.ai_follow_p_id == 0,
                lost_combat_prestige: o.lost_combat_prestige,
                is_cursed: o.is_cursed,
                target_home_quad: (dx * dx + dy * dy) as f32,
            }
        })
        .collect();
    let mask_t = get_close_player_target(
        has_red,
        has_blue,
        0.0,
        p.age,
        p.x,
        p.y,
        PLAYER_TARGET_SEARCH_DIST,
        &pt_cands,
    );
    let target_p_id = deadly
        .map(|d| d.p_id)
        .or_else(|| mask_t.map(|t| t.p_id))?;
    let target = views.values().find(|o| o.p_id == target_p_id && !o.deleted).map(|o| {
        AttackPlayerTarget {
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            exact_x: o.x as f64,
            exact_y: o.y as f64,
            wounded: npc_is_wounded(content, o.held_id) && !o.is_hidden_wound,
        }
    })?;
    let nearby: Vec<(i32, i32, i32, bool)> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| (t.parent_id, t.x, t.y, t.is_permanent))
        .collect();
    let held_name = npc_held_name(content, p.held_id);
    let parent = content
        .dummy_parent
        .get(&p.held_id)
        .copied()
        .unwrap_or(p.held_id);
    let content_dd = content
        .get(p.held_id)
        .map(|d| d.deadly_distance)
        .unwrap_or(0.0);
    let inp = AttackPlayerInput {
        target: Some(target),
        food_store: p.food,
        self_wounded: npc_is_wounded(content, p.held_id) && !p.is_hidden_wound,
        age: p.age,
        min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
        holding_weapon: npc_holding_weapon(content, p.held_id),
        held_id: p.held_id,
        held_parent_id: parent,
        is_moving: p.moving,
        player_x: p.x,
        player_y: p.y,
        exact_x: p.x as f64,
        exact_y: p.y as f64,
        home_x: if p.home_x != 0 || p.home_y != 0 {
            p.home_x
        } else {
            p.x
        },
        home_y: if p.home_x != 0 || p.home_y != 0 {
            p.home_y
        } else {
            p.y
        },
        deadly_distance: deadly_distance_for_held(p.held_id, content_dd),
        clothing: AttackPlayerClothing::from_ids(&p.clothing),
        weapon_tiles: &nearby,
        };
    let _ = held_name;
    let action = attack_player(&inp);
    if !action.is_some() {
        return None;
    }
    let intent = attack_player_action_to_live_intent(action, p.held_id);
    match intent {
        ShortCraftLiveIntent::Kill {
            target_p_id,
            x,
            y,
        } => {
            if npc_say_raw(intent_tx, conn_id, "KILL", &format!("{x} {y} {target_p_id}")) {
                st.food_goto.did_not_reach_food = 0.0;
                Some((
                    NpcActivityKind::Combat,
                    format!("attack_kill target={target_p_id} @{},{}", x, y),
                    400,
                ))
            } else {
                None
            }
        }
        ShortCraftLiveIntent::Goto { x, y } => {
            // Haxe gotoObj always called; MOVE only if arrived or stand-off tile changed.
            if npc_try_walk_to_sticky(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                x,
                y,
                p.food,
                0,
                false,
                p.moving,
                format!("attack_goto @{},{}", x, y),
                npc_client_move_seq(p.done_moving_seq),
            ) {
                // Haxe L5862: if (done) didNotReachAnimalTarget = 0
                st.did_not_reach_animal_target = 0;
                Some((
                    NpcActivityKind::Combat,
                    format!("attack_goto @{},{}", x, y),
                    250,
                ))
            } else {
                None
            }
        }
        ShortCraftLiveIntent::UseAt { x, y, .. } => {
            if npc_is_close_action(p.x, p.y, x, y) {
                if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                    Some((
                        NpcActivityKind::Combat,
                        format!("attack_pickup @{},{}", x, y),
                        400,
                    ))
                } else {
                    None
                }
            } else if npc_try_walk_to(
                intent_tx,
                world,
                content,
                conn_id,
                p.x,
                p.y,
                x,
                y,
                p.food,
                st.food_goto.did_not_reach_food,
                st.animal_path,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((
                    NpcActivityKind::Combat,
                    format!("attack_walk_weapon @{},{}", x, y),
                    250,
                ))
            } else {
                None
            }
        }
        ShortCraftLiveIntent::DropAt { x, y } => {
            let (dx, dy) = if x == 0 && y == 0 { (p.x, p.y) } else { (x, y) };
            if npc_is_close_action(p.x, p.y, dx, dy) {
                if npc_drop_at(intent_tx, conn_id, dx, dy, None) {
                    Some((
                        NpcActivityKind::Combat,
                        format!("attack_drop @{},{}", dx, dy),
                        400,
                    ))
                } else {
                    None
                }
            } else if npc_try_walk_to(
                intent_tx,
                world,
                content,
                conn_id,
                p.x,
                p.y,
                dx,
                dy,
                p.food,
                st.food_goto.did_not_reach_food,
                st.animal_path,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((
                    NpcActivityKind::Combat,
                    format!("attack_walk_drop @{},{}", dx, dy),
                    250,
                ))
            } else {
                None
            }
        }
        ShortCraftLiveIntent::SelfClothing { slot } => {
            let payload = self_clothing_raw_payload(slot);
            if npc_say_raw(intent_tx, conn_id, "SELF", &payload) {
                Some((
                    NpcActivityKind::Combat,
                    format!("attack_self slot={slot}"),
                    400,
                ))
            } else {
                None
            }
        }
        ShortCraftLiveIntent::SeekOrCraft { actor, .. } => {
            // Haxe attackPlayer L5836 getWeapon(false) → GetOrCraftItem(148/152)
            npc_emit_seek_or_craft(
                intent_tx,
                world,
                content,
                craft_graph,
                st,
                conn_id,
                p,
                tick,
                actor,
                NpcActivityKind::Combat,
                "attack_get_weapon",
            )
        }
        ShortCraftLiveIntent::Wait => Some((NpcActivityKind::Combat, "attack_wait".into(), 200)),
        _ => None,
    }
}

/// Enqueue USE via [`PlayerWriteInterface`] (identical to human client command).
#[inline]
fn npc_use_at(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    x: i32,
    y: i32,
    id: Option<i32>,
    index: Option<i32>,
) -> bool {
    NpcWriteTx(intent_tx).use_at(conn_id, x, y, id, index)
}

/// Enqueue MOVE via [`PlayerWriteInterface`].
#[inline]
fn npc_move_path(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    xs: i32,
    ys: i32,
    deltas: &[(i32, i32)],
    seq: Option<i32>,
) -> bool {
    NpcWriteTx(intent_tx).move_path(conn_id, xs, ys, deltas, seq)
}

/// MOVE `@seq` — same player field humans use. Not an AI-only counter.
///
/// Haxe human: `Program.hx` `++player.done_moving_seqNum` on the MOVE line.
/// Haxe AI: `AiHelper.Goto` L1469 `playerInterface.move(..., ai.seqNum++, data)` —
/// that seq is the MOVE command seq; `MoveHelper` writes it to `p.done_moving_seqNum`.
/// Rust NPC is not a second protocol: send `done_moving_seq + 1` like the client.
fn npc_client_move_seq(done_moving_seq: i32) -> i32 {
    done_moving_seq.saturating_add(1).max(1)
}

/// Enqueue DROP via [`PlayerWriteInterface`].
#[inline]
fn npc_drop_at(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    x: i32,
    y: i32,
    clothing_slot: Option<i32>,
) -> bool {
    NpcWriteTx(intent_tx).drop_at(conn_id, x, y, clothing_slot)
}

/// Haxe `isRemovingFromContainer` one npc think.
// Haxe: AiBase.isRemovingFromContainer ~9140
fn npc_run_remove_from_container(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
) -> Option<(NpcActivityKind, String, u32)> {
    let staging = st.remove_from_container?;
    let (world_parent, contained_n) = {
        let id = world.get_object(staging.tx, staging.ty);
        let parent = if id != 0 {
            content.resolve_base_id(id)
        } else {
            0
        };
        let n = world
            .get_helper(staging.tx, staging.ty)
            .map(|h| h.contained.len() as i32)
            .unwrap_or(0);
        (parent, n)
    };
    let adv = advance_remove_from_container(
        Some(staging),
        world_parent,
        contained_n,
        p.held_id,
        p.is_hidden_wound,
        p.holding_player_id != 0,
        p.x,
        p.y,
        p.moving,
        true,
    );
    match adv {
        RemoveFromContainerAdvance::Idle | RemoveFromContainerAdvance::Cancel => {
            st.remove_from_container = None;
            None
        }
        RemoveFromContainerAdvance::GotoFailed { x: _, y: _ } => {
            // Haxe L9191–9195: clear sticky, return false (no notReachable).
            st.remove_from_container = None;
            None
        }
        RemoveFromContainerAdvance::Wait => Some((
            NpcActivityKind::Think,
            "remove_wait".into(),
            200,
        )),
        RemoveFromContainerAdvance::DropHeld => {
            // Haxe L9161: dropHeldObject() default maxDistanceToHome=40.
            let (dx, dy) = npc_empty_drop_xy(world, p.x, p.y);
            if npc_drop_at(intent_tx, conn_id, dx, dy, None) {
                Some((
                    NpcActivityKind::Craft,
                    format!("remove_drop_held @{},{}", dx, dy),
                    400,
                ))
            } else {
                Some((NpcActivityKind::Think, "remove_drop_busy".into(), 200))
            }
        }
        RemoveFromContainerAdvance::DropPlayer => {
            // Haxe L9201–9205: dropPlayer(x,y) — PUTDOWN/DROPBABY; always return true.
            let sent = npc_say_raw(intent_tx, conn_id, "SAY", "DROPBABY");
            Some((
                NpcActivityKind::Baby,
                "remove_drop_player".into(),
                if sent { 400 } else { 100 },
            ))
        }
        RemoveFromContainerAdvance::Goto { x, y } => {
            if npc_try_walk_to(
                intent_tx,
                world,
                content,
                conn_id,
                p.x,
                p.y,
                x,
                y,
                p.food,
                st.food_goto.did_not_reach_food,
                st.animal_path,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((
                    NpcActivityKind::Craft,
                    format!("remove_goto @{},{}", x, y),
                    250,
                ))
            } else {
                st.remove_from_container = None;
                None
            }
        }
        RemoveFromContainerAdvance::RemvNow { x, y } => {
            // Haxe L9211: remove(tx - gx, ty - gy); fail → addNotReachable; always clear + return true.
            let (rx, ry) = remove_command_xy(x, y, p.birth_x, p.birth_y);
            let payload = format!("{rx} {ry}");
            if npc_say_raw(intent_tx, conn_id, "REMV", &payload) {
                st.remove_from_container = None;
                Some((
                    NpcActivityKind::Craft,
                    format!("remove_remv @{},{}", x, y),
                    500,
                ))
            } else {
                st.remove_from_container = None;
                st.path_reach.add_not_reachable_object(x, y, NOT_REACHABLE_DEFAULT_SECS);
                Some((NpcActivityKind::Think, "remove_remv_fail".into(), 100))
            }
        }
    }
}

fn npc_commit_temp_plan(
    st: &mut NpcProfessionState,
    plan: HandleTemperaturePlan,
    snap_cold: Option<(i32, i32)>,
    snap_warm: Option<(i32, i32)>,
) {
    st.handling_temperature = plan.is_handling;
    st.temp_just_arrived = plan.just_arrived;
    st.last_heat = plan.last_heat;
    if plan.clear_cold_place {
        st.rejected_cold_place = snap_cold.or(st.rejected_cold_place);
    }
    if plan.clear_warm_place {
        st.rejected_warm_place = snap_warm.or(st.rejected_warm_place);
    }
}

/// Nested Haxe `isHandlingFire(2)` on fail-warm (rung label TEMPERATURE).
// Haxe: AiBase.handleTemperature L1740
fn npc_temp_try_handling_fire(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    env_winter: bool,
    fire_id: i32,
    fire_x: i32,
    fire_y: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, HANDLING_FIRE_SCAN_RADIUS);
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        p.x
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        p.y
    };
    let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
    inp.home_x = home_x;
    inp.home_y = home_y;
    inp.held_uses = p.held_uses.max(1);
    inp.food_store = p.food;
    inp.age = p.age;
    inp.is_moving = p.moving;
    inp.is_winter = env_winter;
    inp.fire_place_id = fire_id;
    inp.fire_place_x = fire_x;
    inp.fire_place_y = fire_y;
    inp.profession_is_sticky =
        st.fire_keeper_rt.is_last_fire_keeper || st.fire_keeper_rt.is_assigned_fire_keeper;
    inp.is_assigned_job = st.fire_keeper_rt.is_assigned_fire_keeper;
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "TEMPERATURE",
        &mut st.fire_keeper_rt,
        &mut st.fire_rt,
        &mut st.baker_rt,
        &mut st.baker_task,
    );
    if !r.had_action {
        return None;
    }
    let mut kind = NpcActivityKind::Think;
    let mut detail = String::new();
    let mut game_ms = 200u32;
    if npc_commit_craft_live(
        &r.intent,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        npc_client_move_seq(p.done_moving_seq),
    ) {
        Some((kind, format!("temp_handling_fire {detail}"), game_ms))
    } else {
        None
    }
}

/// Haxe `isHandlingFire()` at doTimeStuffHelper L634 (before makeSharpieFood L656).
/// No fire → best FIREKEEPER crafts shaft 67 then Fire 82.
// Haxe: AiBase.isHandlingFire L1079–1111
fn npc_run_is_handling_fire_mid(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    env_winter: bool,
    is_best_home: bool,
    is_best_fire: bool,
) -> Option<(NpcActivityKind, String, u32)> {
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, HANDLING_FIRE_SCAN_RADIUS);
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        p.x
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        p.y
    };
    let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
    inp.home_x = home_x;
    inp.home_y = home_y;
    inp.held_uses = p.held_uses.max(1);
    inp.food_store = p.food;
    inp.age = p.age;
    inp.is_moving = p.moving;
    inp.is_winter = env_winter;
    inp.is_best_fire_keeper_at_home = is_best_home;
    inp.is_best_fire_keeper_at_fire = is_best_fire;
    inp.profession_is_sticky =
        st.fire_keeper_rt.is_last_fire_keeper || st.fire_keeper_rt.is_assigned_fire_keeper;
    inp.is_assigned_job = st.fire_keeper_rt.is_assigned_fire_keeper;
    let r = handling_fire_profession_scan_tick(
        &tiles,
        &inp,
        "MID_PRIORITY_TASKS",
        &mut st.fire_keeper_rt,
        &mut st.fire_rt,
        &mut st.baker_rt,
        &mut st.baker_task,
    );
    if !r.had_action {
        return None;
    }
    let mut kind = NpcActivityKind::Craft;
    let mut detail = String::new();
    let mut game_ms = 200u32;
    if npc_commit_craft_live(
        &r.intent,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        npc_client_move_seq(p.done_moving_seq),
    ) {
        Some((kind, format!("isHandlingFire {detail}"), game_ms))
    } else {
        None
    }
}

/// Commit a profession-scan intent (knife / carrot row / mid isHunting).
fn npc_apply_scan_tick(
    r: ProfessionScanTickResult,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    label: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    if !r.had_action {
        return None;
    }
    let mut kind = NpcActivityKind::Craft;
    let mut detail = String::new();
    let mut game_ms = 200u32;
    if npc_commit_craft_live(
        &r.intent,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        npc_client_move_seq(p.done_moving_seq),
    ) {
        Some((kind, format!("{label} {detail}"), game_ms))
    } else {
        None
    }
}

fn npc_sticky_from_state(st: &NpcProfessionState, p: &PlayerSnapshot) -> ProfessionStickySnapshot {
    let mut sticky = ProfessionStickySnapshot::from_runtimes_ex(
        &st.farm_rt,
        &st.smith_rt,
        &st.baker_rt,
        Some(&st.shepherd_rt),
        Some(&st.pottery_rt),
        Some(&st.fire_rt),
        Some(&st.fire_keeper_rt),
        Some(&st.grave_keeper_rt),
        Some(&st.hunter_rt),
        Some(&st.lumberjack_rt),
        Some(&st.collector_rt),
        Some(&st.foodserver_rt),
        p.age,
    );
    sticky.smith_last |= p.is_last_smith;
    sticky.baker_last |= p.is_last_baker;
    sticky.pottery_last |= p.is_last_potter;
    sticky.shepherd_last |= p.is_last_shepherd;
    sticky.hunter_last |= p.is_last_hunter || st.hunter_rt.is_last_hunter;
    sticky.tailor_last |= p.is_last_tailor || st.last_is_tailor;
    sticky.tailor_assigned |= p.is_assigned_tailor;
    sticky.grave_keeper_last |= st.grave_keeper_rt.is_last_grave_keeper;
    sticky.grave_keeper_assigned |= st.grave_keeper_rt.is_assigned_grave_keeper;
    sticky.foodserver_last |= p.is_last_foodserver || st.foodserver_rt.is_last_foodserver;
    sticky.foodserver_assigned |= st.foodserver_rt.is_assigned_foodserver;
    sticky
}

fn npc_scan_input_for_jobs(
    p: &PlayerSnapshot,
    content: &ContentDb,
    st: &NpcProfessionState,
    tiles: &[ScanTile],
    env_winter: bool,
) -> ProfessionScanInput {
    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
    let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
    inp.home_x = hx;
    inp.home_y = hy;
    inp.held_uses = p.held_uses.max(1);
    inp.held_contained = p.held_contained;
    inp.held_contains_clay = p.held_contains_clay;
    inp.food_store = p.food;
    inp.is_moving = p.moving;
    inp.age = p.age;
    inp.was_idle = st.was_idle;
    inp.has_carrot_seeds = has_carrot_seeds_from_scan(tiles);
    inp.has_bean_seeds = has_bean_seeds_from_scan(tiles);
    inp.target_reachable = true;
    inp.content = Some(std::sync::Arc::new(content.clone()));
    inp.clothing = p.clothing;
    inp.clothing_uses = p.clothing_uses;
    inp.is_winter = env_winter;
    inp.bucket_water_source_ids = init_water_source_ids_from_content(content).1;
    inp.currently_craving = p.currently_craving;
    inp
}

fn npc_run_ladder_rung(
    rung: PriorityRung,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    env_winter: bool,
    label: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
    let inp = npc_scan_input_for_jobs(p, content, st, &tiles, env_winter);
    let sticky = npc_sticky_from_state(st, p);
    let r = ladder_profession_scan_tick(
        rung,
        &tiles,
        &inp,
        &sticky,
        &mut st.farm_task,
        &mut st.farm_rt,
        &mut st.smith_rt,
        &mut st.baker_rt,
        &mut st.baker_task,
        &mut st.shepherd_rt,
        &mut st.pottery_rt,
        &mut st.fire_rt,
        &mut st.fire_keeper_rt,
        &mut st.grave_keeper_rt,
        &mut st.hunter_rt,
        &mut st.lumberjack_rt,
        &mut st.collector_rt,
        &mut st.foodserver_rt,
    );
    npc_apply_scan_tick(r, intent_tx, world, content, st, conn_id, p, label)
}

fn npc_run_ladder_step(
    step: ProfessionLadderStep,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    env_winter: bool,
) -> Option<(NpcActivityKind, String, u32)> {
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
    let mut inp = npc_scan_input_for_jobs(p, content, st, &tiles, env_winter);
    inp.is_assigned_job = step.is_assigned_job;
    inp.profession_is_sticky = step.profession_is_sticky;
    let r = profession_scan_tick(
        step.kind,
        &tiles,
        &inp,
        step.rung_label,
        step.farm_job,
        &mut st.farm_task,
        step.farm_has_profession,
        &mut st.farm_rt,
        &mut st.smith_rt,
        &mut st.baker_rt,
        &mut st.baker_task,
        &mut st.shepherd_rt,
        &mut st.pottery_rt,
        &mut st.fire_rt,
        &mut st.fire_keeper_rt,
        &mut st.grave_keeper_rt,
        &mut st.hunter_rt,
        &mut st.lumberjack_rt,
        &mut st.collector_rt,
        &mut st.foodserver_rt,
    );
    npc_apply_scan_tick(
        r,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p,
        step.rung_label,
    )
}

fn npc_starving_cands_from_views(
    p: &PlayerSnapshot,
    views: &HashMap<u64, PlayerSnapshot>,
    content: &ContentDb,
) -> Vec<StarvingCand> {
    let held_food = content
        .get(content.resolve_base_id(p.held_id))
        .map(|d| d.food_value)
        .unwrap_or(0);
    views
        .values()
        .filter(|o| o.conn_id != p.conn_id && !o.deleted && o.p_id != 0)
        .map(|o| StarvingCand {
            conn_id: o.conn_id,
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            age: o.age,
            food: o.food,
            food_max: o.food_max,
            deleted: o.deleted,
            held_by: o.held_by > 0,
            is_ai: o.is_ai || o.ai_controlled,
            is_wounded: npc_is_wounded(content, o.held_id) && !o.is_hidden_wound,
            has_yellow_fever: o.sick,
            is_smith: o.is_last_smith,
            prestige_class: 0,
            is_ally: o.ai_follow_p_id == p.p_id || p.ai_follow_p_id == o.p_id,
            is_close_relative: false,
            is_follow_target: o.p_id == p.ai_follow_p_id,
            angry_time: o.angry_time,
            lost_combat_prestige: o.lost_combat_prestige,
            can_feed_held: held_food > 0,
            is_starting_name: o.display_name.to_ascii_uppercase().contains("EVE")
                || o.display_name.is_empty(),
            is_female: content
                .get(o.display_object_id)
                .map(|d| !d.male)
                .unwrap_or(true),
        })
        .collect()
}

fn npc_run_feed_player_in_need(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    views: &HashMap<u64, PlayerSnapshot>,
    env_winter: bool,
) -> Option<(NpcActivityKind, String, u32)> {
    if p.age < MIN_AGE_TO_EAT || p.food < FOODSERVER_MIN_FOOD {
        return None;
    }
    if smith_blocks_mid_feed(st.smith_rt.stage) {
        return None;
    }
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, STARVING_SEARCH_DIST);
    let mut inp = npc_scan_input_for_jobs(p, content, st, &tiles, env_winter);
    inp.feeding_cands = npc_starving_cands_from_views(p, views, content);
    inp.held_food_value = content
        .get(content.resolve_base_id(p.held_id))
        .map(|d| d.food_value)
        .unwrap_or(0);
    inp.feeder_p_id = p.p_id;
    inp.feeder_is_smith = st.smith_rt.is_last_smith || p.is_last_smith;
    let r = profession_scan_tick(
        ProfessionScanKind::FoodServer,
        &tiles,
        &inp,
        "FEED_PLAYER_IN_NEED",
        None,
        &mut st.farm_task,
        false,
        &mut st.farm_rt,
        &mut st.smith_rt,
        &mut st.baker_rt,
        &mut st.baker_task,
        &mut st.shepherd_rt,
        &mut st.pottery_rt,
        &mut st.fire_rt,
        &mut st.fire_keeper_rt,
        &mut st.grave_keeper_rt,
        &mut st.hunter_rt,
        &mut st.lumberjack_rt,
        &mut st.collector_rt,
        &mut st.foodserver_rt,
    );
    npc_apply_scan_tick(
        r,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p,
        "isFeedingPlayerInNeed",
    )
}

fn npc_run_skewer_tomato_sprout(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
) -> Option<(NpcActivityKind, String, u32)> {
    // Haxe L659: shortCraft(139, 2832, 20)
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, 20);
    let Some((x, y)) = closest_parent_in_tiles(&tiles, TOMATO_SPROUT, p.x, p.y, 20) else {
        return None;
    };
    let held = content.resolve_base_id(p.held_id);
    if held == SKEWER {
        if npc_is_close_action(p.x, p.y, x, y) {
            if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                return Some((
                    NpcActivityKind::Craft,
                    format!("shortCraft 139+2832 @{x},{y}"),
                    400,
                ));
            }
        } else if npc_try_walk_to_sticky(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            p.x,
            p.y,
            x,
            y,
            p.food,
            TOMATO_SPROUT,
            true,
            p.moving,
            "skewer_tomato_sprout",
            npc_client_move_seq(p.done_moving_seq),
        ) {
            return Some((
                NpcActivityKind::Craft,
                "skewer_tomato_sprout_walk".into(),
                250,
            ));
        }
        return None;
    }
    npc_emit_seek_or_craft(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        SKEWER,
        NpcActivityKind::Craft,
        "shortCraft_skewer_139",
    )
}

/// Haxe `handleTemperature` drink / GetOrCraft / biome / fire / arrive.
// Haxe: AiBase.handleTemperature ~1645 (AI-HANDLE-TEMP)
fn npc_run_handle_temperature(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    env_winter: bool,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    tick: u64,
) -> Option<(NpcActivityKind, String, u32)> {
    let held_id = if p.held_id != 0 {
        content.resolve_base_id(p.held_id)
    } else {
        0
    };
    let color = content.person_color(p.display_object_id);
    let _female = content
        .get(p.display_object_id)
        .map(|o| !o.male)
        .unwrap_or_else(|| person_looks_female(p.display_object_id, "", ""));
    let fire_id = if st.fire_place_id != 0 {
        st.fire_place_id
    } else {
        p.ai_fire_place_id
    };
    let fire_x = if st.fire_place_id != 0 {
        st.fire_place_x
    } else {
        p.ai_fire_place_x
    };
    let fire_y = if st.fire_place_id != 0 {
        st.fire_place_y
    } else {
        p.ai_fire_place_y
    };
    let fire_parent = if fire_id != 0 {
        content.resolve_base_id(fire_id)
    } else {
        0
    };
    let fire_heat_value = if fire_parent != 0 {
        content.get(fire_parent).map(|o| o.heat_value).unwrap_or(0.0)
    } else {
        0.0
    };
    let blocked = st.path_reach.blocked_coords(None);
    let mut cool_tiles = Vec::new();
    let mut warm_tiles = Vec::new();
    let start_x = p.x - GET_CLOSE_BIOME_DIST;
    let end_x = p.x + GET_CLOSE_BIOME_DIST;
    let start_y = p.y - GET_CLOSE_BIOME_DIST;
    let end_y = p.y + GET_CLOSE_BIOME_DIST;
    for ty in start_y..end_y {
        for tx in start_x..end_x {
            let b = world.get_biome(tx, ty);
            if b == 4 || b == PASSABLE_RIVER {
                cool_tiles.push((tx, ty, b));
            } else if b == DESERT || b == 6 {
                warm_tiles.push((tx, ty, b));
            }
        }
    }
    let close_cool = get_close_biome(
        p.x,
        p.y,
        &COOL_BIOMES,
        &cool_tiles,
        |x, y| blocked.contains(&(x, y)),
        world.width_tiles,
        world.height_tiles,
        world.wrap,
    );
    let close_warm = get_close_biome(
        p.x,
        p.y,
        &WARM_BIOMES,
        &warm_tiles,
        |x, y| blocked.contains(&(x, y)),
        world.width_tiles,
        world.height_tiles,
        world.wrap,
    );
    let mut inp = HandleTemperatureInput {
        heat: p.heat,
        last_heat: st.last_heat,
        player_last_temperature: p.last_temperature,
        is_handling: st.handling_temperature,
        just_arrived: st.temp_just_arrived,
        is_super_hot: is_super_hot_for_person(p.heat, color),
        is_super_cold: is_super_cold_for_person(p.heat, color),
        winter_female_with_kids: false,
        held_id,
        item_to_craft_id: st.craft_rt.item.product_id,
        fire_x,
        fire_y,
        fire_parent,
        fire_heat_value,
        has_fire_place: fire_id != 0,
        close_cool,
        close_warm,
        cold_place: p.cold_place.filter(|c| st.rejected_cold_place != Some(*c)),
        warm_place: p.warm_place.filter(|c| st.rejected_warm_place != Some(*c)),
        px: p.x,
        py: p.y,
        age: p.age,
        skip_water: false,
    };
    let mut plan = plan_handle_temperature(inp);
    if matches!(plan.action, HandleTemperatureAction::GetOrCraftWater) {
        let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
        let blocked_set = st.path_reach.blocked_coords(None);
        let intent = npc_expand_craft_product(
            world,
            &tiles,
            p.x,
            p.y,
            p.held_id,
            p.moving,
            p.home_x,
            p.home_y,
            content,
            craft_graph,
            &mut st.craft_rt,
            &blocked_set,
            false,
            tick,
            210,
        );
        if !matches!(intent, ShortCraftLiveIntent::None) {
            st.handling_temperature = plan.is_handling;
            st.temp_just_arrived = plan.just_arrived;
            st.last_heat = plan.last_heat;
            let mut kind = NpcActivityKind::Think;
            let mut detail = String::new();
            let mut game_ms = 200u32;
            if npc_commit_craft_live(
                &intent,
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                return Some((kind, format!("temp_craft_water {detail}"), game_ms));
            }
        }
        inp.skip_water = true;
        plan = plan_handle_temperature(inp);
    }
    npc_commit_temp_plan(st, plan, p.cold_place, p.warm_place);
    match plan.action {
        HandleTemperatureAction::Idle => None,
        HandleTemperatureAction::DrinkSelf => {
            let payload = self_clothing_raw_payload(-1);
            if intent_tx
                .try_send(NetIntent::Raw {
                    conn_id,
                    tag: "SELF".into(),
                    payload,
                })
                .is_ok()
            {
                let _ = npc_say_raw(intent_tx, conn_id, "SAY", HANDLE_TEMP_SAY_DRINK);
                Some((NpcActivityKind::Think, "temp_drink".into(), 400))
            } else {
                Some((NpcActivityKind::Think, "temp_drink_busy".into(), 200))
            }
        }
        HandleTemperatureAction::CraftLargeFastFire => {
            let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
            let blocked_set = st.path_reach.blocked_coords(None);
            let intent = npc_expand_craft_product(
                world,
                &tiles,
                p.x,
                p.y,
                p.held_id,
                p.moving,
                p.home_x,
                p.home_y,
                content,
                craft_graph,
                &mut st.craft_rt,
                &blocked_set,
                false,
                tick,
                HANDLE_TEMP_LARGE_FAST_FIRE,
            );
            let mut kind = NpcActivityKind::Think;
            let mut detail = String::new();
            let mut game_ms = 200u32;
            if npc_commit_craft_live(
                &intent,
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((kind, format!("temp_craft_83 {detail}"), game_ms))
            } else {
                None
            }
        }
        HandleTemperatureAction::GetOrCraftWater => None,
        HandleTemperatureAction::Relax => Some((
            NpcActivityKind::Think,
            "temp_relax".into(),
            (HANDLE_TEMP_RELAX_TIME * 1000.0) as u32,
        )),
        HandleTemperatureAction::Goto { x, y } => {
            // Haxe L1757–1759: tryMoveNearestTileFirst = false unless goodPlace is firePlace
            if npc_try_walk_to_ex(
                intent_tx,
                world,
                content,
                conn_id,
                p.x,
                p.y,
                x,
                y,
                p.food,
                st.food_goto.did_not_reach_food,
                st.animal_path,
                false,
                plan.try_move_nearest_tile_first,
                true,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((
                    NpcActivityKind::Think,
                    format!("temp_place @{},{}", x, y),
                    250,
                ))
            } else {
                None
            }
        }
        HandleTemperatureAction::KindlingOnFire { x, y } => {
            if held_id == HANDLE_TEMP_KINDLING {
                if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                    return Some((
                        NpcActivityKind::Craft,
                        format!("temp_kindling @{},{}", x, y),
                        500,
                    ));
                }
            }
            let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
            let blocked_set = st.path_reach.blocked_coords(None);
            let intent = npc_expand_craft_product(
                world,
                &tiles,
                p.x,
                p.y,
                p.held_id,
                p.moving,
                p.home_x,
                p.home_y,
                content,
                craft_graph,
                &mut st.craft_rt,
                &blocked_set,
                false,
                tick,
                HANDLE_TEMP_KINDLING,
            );
            let mut kind = NpcActivityKind::Think;
            let mut detail = String::new();
            let mut game_ms = 200u32;
            if npc_commit_craft_live(
                &intent,
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                Some((kind, format!("temp_kindling_seek {detail}"), game_ms))
            } else if let Some(fire) = npc_temp_try_handling_fire(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p,
                env_winter,
                fire_id,
                fire_x,
                fire_y,
            ) {
                Some(fire)
            } else {
                let cleared = fail_warm_clear_place(plan);
                npc_commit_temp_plan(st, cleared, p.cold_place, p.warm_place);
                None
            }
        }
        HandleTemperatureAction::HandlingFire => {
            if let Some(fire) = npc_temp_try_handling_fire(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p,
                env_winter,
                fire_id,
                fire_x,
                fire_y,
            ) {
                Some(fire)
            } else {
                let cleared = fail_warm_clear_place(plan);
                npc_commit_temp_plan(st, cleared, p.cold_place, p.warm_place);
                None
            }
        }
    }
}

/// Enqueue raw SAY/JUMP/â€¦ via [`PlayerWriteInterface`].
#[inline]
fn npc_say_raw(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    tag: &str,
    payload: &str,
) -> bool {
    NpcWriteTx(intent_tx).say_raw(conn_id, tag, payload)
}

/// Haxe `searchFoodAndEat` shouldDebugSay after `SearchBestFood` (foodTarget was null).
// Haxe: AiBase.searchFoodAndEat L5076–5078
fn npc_search_food_and_eat_debug_say(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    content: &ContentDb,
    conn_id: u64,
    debug_say: bool,
    food: Option<&StickyFoodTarget>,
) {
    let name_owned = food.map(|f| {
        content
            .get(f.parent_id)
            .map(|d| d.name.clone())
            .unwrap_or_default()
    });
    if let Some(line) = search_food_and_eat_debug_say(debug_say, name_owned.as_deref()) {
        let _ = npc_say_raw(intent_tx, conn_id, "SAY", &line);
    }
}

fn npc_hungry_child_cands(
    views: &std::collections::HashMap<u64, PlayerSnapshot>,
    mother_p_id: i32,
) -> Vec<HungryChildCand> {
    views
        .values()
        .filter(|o| !o.deleted && o.p_id != mother_p_id)
        .map(|o| HungryChildCand {
            conn_id: o.conn_id,
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            age: o.age,
            food: o.food,
            held_by: o.held_by,
            is_own: o.ai_follow_p_id == mother_p_id,
        })
        .collect()
}

/// Haxe `AiBase.isFeedingChild` — pickup / hold / drop hungry infants (before food seek).
// Haxe: AiBase.isFeedingChild L6412–6490
fn npc_run_is_feeding_child(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &ol_world::World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    views: &std::collections::HashMap<u64, PlayerSnapshot>,
) -> Option<(NpcActivityKind, String, u32)> {
    let looks_female = person_looks_female(p.display_object_id, "", "");
    let fertile = is_fertile(p.deleted, p.age, looks_female);
    if p.food < 2.0 {
        return None;
    }
    if !fertile || p.food < 1.0 {
        if p.holding_player_id != 0 {
            // Haxe: dropPlayer then say('I cannot feed you!')
            // Haxe: AiBase.isFeedingChild L6418–6421
            let _ = npc_say_raw(intent_tx, conn_id, "SAY", "DROPBABY");
            let _ = npc_say_raw(intent_tx, conn_id, "SAY", "I cannot feed you!");
            return Some((
                NpcActivityKind::Baby,
                format!("drop_cannot_feed held={}", p.holding_player_id),
                200,
            ));
        }
        return None;
    }
    if p.holding_player_id != 0 {
        let baby = views.values().find(|o| o.p_id == p.holding_player_id);
        if let Some(b) = baby {
            // Haxe isFeedingChild: YOU ARE while holding StartingName (SPOON).
            // Haxe: AiBase.isFeedingChild L6428–6434
            let baby_first = b
                .display_name
                .split_whitespace()
                .next()
                .unwrap_or("");
            let mother_first = p
                .display_name
                .split_whitespace()
                .next()
                .unwrap_or("");
            if baby_first.eq_ignore_ascii_case(STARTING_NAME)
                && (b.ai_follow_p_id == p.p_id || b.age > 1.5)
            {
                let baby_female = person_looks_female(b.display_object_id, "", "");
                let names = if baby_female {
                    FEMALE_FIRST_NAMES
                } else {
                    MALE_FIRST_NAMES
                };
                let random = names
                    .get(rand::random::<usize>() % names.len().max(1))
                    .copied()
                    .unwrap_or("ALICE");
                let new_name = if is_eve_or_adam_name(mother_first)
                    || rand::random::<f32>() < 0.2
                {
                    random
                } else if !mother_first.is_empty() {
                    mother_first
                } else {
                    random
                };
                let payload = format!("YOU ARE {new_name}");
                if npc_say_raw(intent_tx, conn_id, "SAY", &payload) {
                    return Some((
                        NpcActivityKind::Baby,
                        format!("you_are child={} {payload}", b.p_id),
                        400,
                    ));
                }
            }
            let cap = get_max_child_feeding(b.food_max);
            if b.food > cap - 0.2 {
                let cands = npc_hungry_child_cands(views, p.p_id);
                let another = pick_close_hungry_child(
                    p.x,
                    p.y,
                    &cands,
                    HUNGRY_CHILD_SEARCH_DIST,
                    MAX_CHILD_AGE_BREAST_FEEDING,
                    3.0,
                )
                .filter(|c| c.p_id != b.p_id);
                // Haxe: age*60 > MinMovementAgeInSec && hits < 1 && !isIll
                // Haxe: AiBase.isFeedingChild L6436–6441
                let min_move = gameplay_defaults::MIN_MOVEMENT_AGE_IN_SEC;
                let can_walk = b.age * 60.0 > min_move && b.hits < 1.0 && !b.sick;
                if another.is_some() || can_walk {
                    let _ = npc_say_raw(intent_tx, conn_id, "SAY", "DROPBABY");
                    return Some((
                        NpcActivityKind::Baby,
                        format!("drop_full child={} food={:.1}", b.p_id, b.food),
                        200,
                    ));
                }
            }
        }
        // Haxe: holding baby → handleTemperature + stay (TimeHelper nurses).
        return Some((
            NpcActivityKind::Baby,
            format!("hold_nurse child={}", p.holding_player_id),
            400,
        ));
    }
    let cands = npc_hungry_child_cands(views, p.p_id);
    let child = pick_close_hungry_child(
        p.x,
        p.y,
        &cands,
        HUNGRY_CHILD_SEARCH_DIST,
        MAX_CHILD_AGE_BREAST_FEEDING,
        3.0,
    )?;
    let close = can_pickup_baby_distance(
        p.x as f32,
        p.y as f32,
        child.x as f32,
        child.y as f32,
    );
    // isFeedingChild returns before switchCloths (Haxe L557 then L558). A skirt
    // just DROPped into the hand (isDropingItem L8456) would be carried to the
    // child and then dropHeldObject(0)'d, so SELF (L8709) never sees held 128.
    // Wear first when the held object is clothing; rope 59 is not clothing and
    // still falls through to the reed USE below.
    if p.held_id > 0 && !p.is_hidden_wound {
        let class_u8 = npc_prestige_class_u8(st.prestige_class);
        if let Some(out) = npc_run_switch_cloths(intent_tx, content, conn_id, p, class_u8) {
            return Some(out);
        }
    }
    // Finish 59+124 or 59+131 before walking to the baby. Doing it only when
    // already close ping-ponged: the USE walked away, the next think goto_feed
    // walked back, then dropHeldObject(0) put the rope on the ground.
    // Haxe: isFeedingChild L6476 dropHeldObject(0) only after the child is in reach;
    // switchCloths L8709 must still see held 128 from isDropingItem L8456.
    if p.held_id > 0 && !p.is_hidden_wound {
        if let Some(out) = npc_try_pair_before_baby(intent_tx, world, content, st, conn_id, p) {
            return Some(out);
        }
    }
    if !close {
        let walked = npc_try_walk_to(
            intent_tx,
            world,
            content,
            conn_id,
            p.x,
            p.y,
            child.x,
            child.y,
            p.food,
            st.food_goto.did_not_reach_food,
            st.animal_path,
            npc_client_move_seq(p.done_moving_seq),
        );
        if walked {
            return Some((
                NpcActivityKind::Baby,
                format!("goto_feed child={} @{},{}", child.p_id, child.x, child.y),
                250,
            ));
        }
        return None;
    }
    if p.held_id != 0 && !p.is_hidden_wound {
        // Haxe dropHeldObject(0): empty tile, walk there, DROP only when adjacent.
        if let Some(out) = npc_send_or_walk_empty_drop(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            p.x,
            p.y,
            p.food,
            p.moving,
            p.held_id,
            npc_client_move_seq(p.done_moving_seq),
            false,
        ) {
            let detail = if out.1.starts_with("drop_held_clear") {
                format!("drop_obj_for_baby {}", out.1)
            } else {
                format!("drop_obj_for_baby held={}", p.held_id)
            };
            return Some((out.0, detail, out.2));
        }
        return None;
    }
    let payload = format!("{} {} {}", child.x, child.y, child.p_id);
    if npc_say_raw(intent_tx, conn_id, "BABY", &payload) {
        return Some((
            NpcActivityKind::Baby,
            format!("pickup child={} @{},{}", child.p_id, child.x, child.y),
            500,
        ));
    }
    None
}

/// Haxe `age < MinAgeToEat && isHungry` always returns after follow attempt.
// Haxe: AiBase.doTimeStuffHelper L523–532
fn npc_hungry_infant_skips_craft(age: f32, hungry: bool, _has_living_mother: bool) -> bool {
    hungry_infant_always_returns(age, hungry, MIN_AGE_TO_EAT)
}

/// Haxe `isMovingToPlayer(maxDistance)` when `playerToFollow` is already set.
/// Returns Some only when a MOVE was sent (`gotoAdv` done). Close / goto-fail → None.
// Haxe: AiBase.isMovingToPlayer L8284–8323
fn npc_try_follow_player_within(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &ol_world::World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    views: &std::collections::HashMap<u64, PlayerSnapshot>,
    max_tiles: i32,
    label: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    let follow = p.ai_follow_p_id;
    if follow <= 0 {
        return None;
    }
    let target = views.values().find(|o| o.p_id == follow && !o.deleted)?;
    let dx = (target.x - p.x) as f32;
    let dy = (target.y - p.y) as f32;
    if !is_moving_to_player_needed(dx * dx + dy * dy, max_tiles) {
        return None;
    }
    // Haxe L8307–8310: still moving → time += 1, then gotoAdv.
    if p.moving {
        st.think_time_sec += 1.0;
    }
    let walked = npc_try_walk_to(
        intent_tx,
        world,
        content,
        conn_id,
        p.x,
        p.y,
        target.x,
        target.y,
        p.food,
        st.food_goto.did_not_reach_food,
        st.animal_path,
        npc_client_move_seq(p.done_moving_seq),
    );
    if walked {
        Some((
            NpcActivityKind::Baby,
            format!("{label} follow={follow} @{},{}", target.x, target.y),
            250,
        ))
    } else {
        None
    }
}

/// Haxe L538–546: nice baby after mother follow miss + handleTemperature miss.
/// Always consumes the tick (`time += 2; return`).
// Haxe: AiBase.doTimeStuffHelper L538–546
fn npc_run_nice_baby_after_mother(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &ol_world::World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
) -> Option<(NpcActivityKind, String, u32)> {
    if !p.ai_is_nice_baby {
        return None;
    }
    if nice_baby_noble_wants_weapon(
        p.ai_is_nice_baby,
        st.prestige_class.is_noble_or_more(),
        content.resolve_base_id(p.held_id),
    ) {
        let want = [WAR_SWORD_ID, KNIFE_ID];
        let tiles = npc_scan_cached(st, world, content, p.x, p.y, 40);
        for id in want {
            if let Some((x, y)) = closest_parent_in_tiles(&tiles, id, p.x, p.y, 40) {
                if npc_is_close_action(p.x, p.y, x, y) {
                    if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                        return Some((
                            NpcActivityKind::Baby,
                            format!("nice_baby_get {id}"),
                            400,
                        ));
                    }
                } else if npc_try_walk_to(
                    intent_tx,
                    world,
                    content,
                    conn_id,
                    p.x,
                    p.y,
                    x,
                    y,
                    p.food,
                    st.food_goto.did_not_reach_food,
                    st.animal_path,
                    npc_client_move_seq(p.done_moving_seq),
                ) {
                    return Some((
                        NpcActivityKind::Baby,
                        format!("nice_baby_walk {id}"),
                        250,
                    ));
                }
            }
        }
    }
    st.think_time_sec += 2.0;
    Some((
        NpcActivityKind::Baby,
        format!("nice_baby_wait_mother={}", p.ai_follow_p_id),
        2000,
    ))
}

/// Haxe `isStayingCloseToChild` — fertile mother walks to a far own infant.
// Haxe: AiBase.isStayingCloseToChild L6399–6409
fn npc_run_stay_close_to_child(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &ol_world::World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    views: &std::collections::HashMap<u64, PlayerSnapshot>,
) -> Option<(NpcActivityKind, String, u32)> {
    let looks_female = person_looks_female(p.display_object_id, "", "");
    if !is_fertile(p.deleted, p.age, looks_female) {
        return None;
    }
    let cands = npc_hungry_child_cands(views, p.p_id);
    let child = pick_most_distant_own_child(
        p.x,
        p.y,
        &cands,
        DISTANT_OWN_CHILD_MIN_DIST,
        DISTANT_OWN_CHILD_SEARCH_DIST,
        3.0,
    )?;
    let walked = npc_try_walk_to(
        intent_tx,
        world,
        content,
        conn_id,
        p.x,
        p.y,
        child.x,
        child.y,
        p.food,
        st.food_goto.did_not_reach_food,
        st.animal_path,
        npc_client_move_seq(p.done_moving_seq),
    );
    if walked {
        Some((
            NpcActivityKind::Baby,
            format!("guard_child={} @{},{}", child.p_id, child.x, child.y),
            250,
        ))
    } else {
        None
    }
}

/// Max tiles per NPC MOVE commit.
///
/// Walk speed â‰ˆ 3.75 tiles/s â†’ 16 steps â‰ˆ 4.3s, spanning â‰¥1 think skip when
/// `ai_think_period_ticks` is 15â€“20 (3â€“4s). While `PlayerSnapshot.moving` is
/// true the scheduler skips that NPC â€” fewer accepts, less log spam.
const NPC_PATH_MAX_STEPS: usize = 16;

/// Animal-aware first step toward goal (Haxe Goto + CreateCollisionChunk animals).
// Haxe: AiHelper.gotoAdv considerAnimals; GotoHelper CreateCollisionChunk
fn npc_next_step_to(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
    animal: Option<AnimalPathPlayerCtx>,
    check_if_dangerous: bool,
) -> Option<(i32, i32)> {
    // Haxe: considerAnimals = checkIfDangerous && didNotReachFood < 5 && food_store > -1
    let consider = consider_animals_for_goto(check_if_dangerous, did_not_reach_food, food_store);
    next_step_consider_animals_for_player(world, content, sx, sy, gx, gy, consider, animal)
}

/// Multi-step relative path toward `(gx,gy)` (animal-aware), capped at `max_steps`.
/// Prefer this over a single `npc_next_step_to` so timed movement commits a real path.
///
/// `stop_when_close`: Haxe USE/DROP `isClose` — stop on orthogonal adjacency so a
/// 16-step greedy path does not walk *past* an unwalkable craft tile.
fn npc_path_toward(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
    max_steps: usize,
    animal: Option<AnimalPathPlayerCtx>,
    stop_when_close: bool,
) -> Vec<(i32, i32)> {
    npc_path_toward_ex(
        world,
        content,
        sx,
        sy,
        gx,
        gy,
        food_store,
        did_not_reach_food,
        max_steps,
        animal,
        stop_when_close,
        TRY_MOVE_NEAREST_TILE_FIRST_DEFAULT,
        true,
    )
}

/// First walkable GotoHelper approach tile (nearest-first by default).
// Haxe: AiHelper.GotoHelper L1365–1397; AiBase.tryMoveNearestTileFirst L113
fn npc_remap_goto_goal(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    stop_when_close: bool,
    try_nearest: bool,
) -> (i32, i32, bool) {
    let px = gx - sx;
    let py = gy - sy;
    for (tweak_x, tweak_y) in goto_approach_tweaks(px, py, GOTO_APPROACH_RAD, try_nearest) {
        let dx = gx + tweak_x;
        let dy = gy + tweak_y;
        if dx == sx && dy == sy {
            continue;
        }
        if is_walkable(world, content, dx, dy) {
            // Walk onto the approach tile (Haxe path end is that tile).
            return (dx, dy, false);
        }
    }
    (gx, gy, stop_when_close)
}

fn npc_path_toward_ex(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
    max_steps: usize,
    animal: Option<AnimalPathPlayerCtx>,
    stop_when_close: bool,
    try_nearest: bool,
    check_if_dangerous: bool,
) -> Vec<(i32, i32)> {
    let mut deltas = Vec::new();
    if max_steps == 0 || (sx == gx && sy == gy) {
        return deltas;
    }
    if stop_when_close && npc_is_close_action(sx, sy, gx, gy) {
        return deltas;
    }
    let (gx, gy, stop_when_close) = npc_remap_goto_goal(
        world,
        content,
        sx,
        sy,
        gx,
        gy,
        stop_when_close,
        try_nearest,
    );
    let mut cx = sx;
    let mut cy = sy;
    for _ in 0..max_steps {
        if cx == gx && cy == gy {
            break;
        }
        if stop_when_close && npc_is_close_action(cx, cy, gx, gy) {
            break;
        }
        let step = npc_next_step_to(
            world,
            content,
            cx,
            cy,
            gx,
            gy,
            food_store,
            did_not_reach_food,
            animal,
            check_if_dangerous,
        );
        let Some((dx, dy)) = step else {
            break;
        };
        if dx == 0 && dy == 0 {
            break;
        }
        let nx = cx + dx;
        let ny = cy + dy;
        deltas.push((dx, dy));
        cx = nx;
        cy = ny;
        if stop_when_close && npc_is_close_action(cx, cy, gx, gy) {
            break;
        }
    }
    // Greedy one-tile fallback when A* finds nothing (edge/blocked).
    // Action walks only step orthogonally so the next tile is `isClose`.
    if deltas.is_empty() && !(stop_when_close && npc_is_close_action(sx, sy, gx, gy)) {
        let sdx = (gx - sx).signum();
        let sdy = (gy - sy).signum();
        let try_steps: Vec<(i32, i32)> = if stop_when_close {
            vec![(sdx, 0), (0, sdy)]
        } else {
            vec![(sdx, 0), (0, sdy), (sdx, sdy)]
        };
        for (dx, dy) in try_steps {
            if dx == 0 && dy == 0 {
                continue;
            }
            // Same gate as sim truncate_walkable. A step into snow-grey was
            // accepted by object walkability, then rejected, and the NPC
            // retried that MOVE forever (rope 59 left on the far side).
            if !npc_is_blocked(world, content, 0, sx + dx, sy + dy) {
                deltas.push((dx, dy));
                break;
            }
        }
    }
    deltas
}

/// Enqueue multi-step MOVE toward world goal; returns true if intent accepted.
fn npc_try_walk_to(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    conn_id: u64,
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
    animal: Option<AnimalPathPlayerCtx>,
    seq: i32,
) -> bool {
    npc_try_walk_to_ex(
        intent_tx,
        world,
        content,
        conn_id,
        px,
        py,
        gx,
        gy,
        food_store,
        did_not_reach_food,
        animal,
        false,
        TRY_MOVE_NEAREST_TILE_FIRST_DEFAULT,
        true,
        seq,
    )
}

fn npc_try_walk_to_ex(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    conn_id: u64,
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
    animal: Option<AnimalPathPlayerCtx>,
    stop_when_close: bool,
    try_nearest: bool,
    check_if_dangerous: bool,
    seq: i32,
) -> bool {
    let (gx, gy) = npc_wrap_goal(world, px, py, gx, gy);
    let deltas = npc_path_toward_ex(
        world,
        content,
        px,
        py,
        gx,
        gy,
        food_store,
        did_not_reach_food,
        NPC_PATH_MAX_STEPS,
        animal,
        stop_when_close,
        try_nearest,
        check_if_dangerous,
    );
    if deltas.is_empty() {
        return false;
    }
    npc_move_path(intent_tx, conn_id, px, py, &deltas, Some(seq))
}

/// Clothing `craftItem(128)`: home plus the player’s current tile.
/// `searchBestObjectForCrafting` steps radius by 30 up to 60 and stops
/// on the first ring that finds a transition.
// Haxe: IntemToCraft.searchCurrentPosition; searchBestObjectForCrafting L7145
fn npc_prepare_clothing_l672_search(craft_rt: &mut CraftAiRuntime, opts: &mut CraftLiveExpandOpts) {
    craft_rt.item.search_current_position = true;
    craft_rt.item.max_search_radius = NPC_GET_OR_CRAFT_SEARCH_RADIUS;
    opts.ai_max_search_radius = NPC_GET_OR_CRAFT_SEARCH_RADIUS;
    opts.ai_max_search_increment = 30;
}

fn npc_craft_live_opts(
    content: &ContentDb,
    home_x: i32,
    home_y: i32,
    is_smith: bool,
    tick: u64,
) -> CraftLiveExpandOpts {
    let (water_ids, bucket_ids) = init_water_source_ids_from_content(content);
    CraftLiveExpandOpts {
        home: Some((home_x, home_y)),
        is_or_can_smith: is_smith,
        now_sec: tick as f64 * 0.2,
        water_source_ids: water_ids,
        bucket_water_source_ids: bucket_ids,
        is_hidden_wound: false,
        ..Default::default()
    }
    .with_content_craft_gates(content)
}

/// Expand one queued / sticky product via GetOrCraft (Haxe `craftItem` from doTimeStuffHelper).
// Haxe: AiBase.doTimeStuffHelper ~667–680 craftItem
fn npc_expand_craft_intent(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    is_smith: bool,
    tick: u64,
    intent: ShortCraftLiveIntent,
) -> ShortCraftLiveIntent {
    let opts = npc_craft_live_opts(content, home_x, home_y, is_smith, tick);
    npc_expand_craft_intent_opts(
        world, tiles, px, py, held_id, moving, content, craft_graph, craft_rt, blocked, opts,
        intent,
    )
}

/// Clothing `craftItem` after Haxe L672 (home-only, first pass r=60).
fn npc_expand_clothing_craft_intent(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    is_smith: bool,
    tick: u64,
    intent: ShortCraftLiveIntent,
) -> ShortCraftLiveIntent {
    let mut opts = npc_craft_live_opts(content, home_x, home_y, is_smith, tick);
    npc_prepare_clothing_l672_search(craft_rt, &mut opts);
    npc_expand_craft_intent_opts(
        world, tiles, px, py, held_id, moving, content, craft_graph, craft_rt, blocked, opts,
        intent,
    )
}

fn npc_expand_craft_intent_opts(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    opts: CraftLiveExpandOpts,
    intent: ShortCraftLiveIntent,
) -> ShortCraftLiveIntent {
    let goc_objs = get_or_craft_objs_from_scan(tiles, None);
    let full_piles = full_pile_tiles_from_scan(tiles);
    let nonempty_boxes = nonempty_container_tiles_from_scan(tiles);
    let pile_id_for = |id: i32| {
        let p = pile_obj_id_from_content(content, id);
        if p > 0 {
            p
        } else {
            0
        }
    };
    // Haxe dropHeldObject(0): empty feet tile, else an orthogonal neighbor.
    // Dropping on the occupied player tile swaps or no-ops, so held food never leaves the hand.
    let (dx, dy) = npc_empty_drop_xy(world, px, py);
    npc_enqueue_get_or_craft_ex(
        intent,
        &goc_objs,
        px,
        py,
        held_id,
        moving,
        Some((dx, dy)),
        Some(craft_graph),
        &opts,
        Some(craft_rt),
        &pile_id_for,
        Some(blocked),
        Some(&full_piles),
        Some(&nonempty_boxes),
    )
}

fn npc_expand_craft_product(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    is_smith: bool,
    tick: u64,
    product_id: i32,
) -> ShortCraftLiveIntent {
    npc_expand_craft_intent(
        world,
        tiles,
        px,
        py,
        held_id,
        moving,
        home_x,
        home_y,
        content,
        craft_graph,
        craft_rt,
        blocked,
        is_smith,
        tick,
        ShortCraftLiveIntent::CraftItem {
            object_id: product_id,
        },
    )
}

fn npc_clothing_craft_item_fallback(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    is_smith: bool,
    tick: u64,
    object_id: i32,
) -> ShortCraftLiveIntent {
    let pickup = npc_expand_clothing_craft_intent(
        world,
        tiles,
        px,
        py,
        held_id,
        moving,
        home_x,
        home_y,
        content,
        craft_graph,
        craft_rt,
        blocked,
        is_smith,
        tick,
        ShortCraftLiveIntent::SeekOrCraft {
            actor: object_id,
            craft_if_needed: false,
        },
    );
    if npc_craft_expand_progress(&pickup, moving) && !matches!(pickup, ShortCraftLiveIntent::Wait)
    {
        pickup
    } else {
        npc_expand_clothing_craft_product(
            world,
            tiles,
            px,
            py,
            held_id,
            moving,
            home_x,
            home_y,
            content,
            craft_graph,
            craft_rt,
            blocked,
            is_smith,
            tick,
            object_id,
        )
    }
}

fn npc_expand_clothing_craft_product(
    world: &World,
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    moving: bool,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    craft_rt: &mut CraftAiRuntime,
    blocked: &HashSet<(i32, i32)>,
    is_smith: bool,
    tick: u64,
    product_id: i32,
) -> ShortCraftLiveIntent {
    npc_expand_clothing_craft_intent(
        world,
        tiles,
        px,
        py,
        held_id,
        moving,
        home_x,
        home_y,
        content,
        craft_graph,
        craft_rt,
        blocked,
        is_smith,
        tick,
        ShortCraftLiveIntent::CraftItem {
            object_id: product_id,
        },
    )
}

/// Haxe `isConsideringMakingFood` body L8538–8607 (after home skip / food refresh).
/// Tail L8603: `countRawRabbit <= 1 && makeFireFood(1)` after baking/watering.
// Haxe: AiBase.isConsideringMakingFood L8538–8607
fn npc_run_considering_making_food(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
) -> Option<(NpcActivityKind, String, u32)> {
    let seq = npc_client_move_seq(p.done_moving_seq);
    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
    let tiles = npc_scan_cached_rw(st, world, content, p.x, p.y, 40);
    if let Some(out) = npc_try_sharpie_food(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE,
        &tiles,
        seq,
    ) {
        return Some(out);
    }
    // Haxe L8543: after makeSharpieFood(5) miss, still moving → busy
    if p.moving {
        return Some((
            NpcActivityKind::Move,
            "consider_food_moving".into(),
            250,
        ));
    }
    let blocked = st.path_reach.blocked_coords(None);
    if npc_count_parent_cheb(&tiles, hx, hy, TURKEY_SLICE_ON_PLATE, 30)
        + i32::from(p.held_id == TURKEY_SLICE_ON_PLATE)
        < 1
    {
        if let Some(out) = npc_try_craft_product_commit(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            TURKEY_SLICE_ON_PLATE,
            &tiles,
            &blocked,
            seq,
            "consider_food_turkey",
        ) {
            return Some(out);
        }
    }
    let row = npc_closest_parent_cheb(&tiles, p.x, p.y, 400, 10);
    match pull_carrot_row_if_needed(&PullCarrotRowInput {
        held_id: p.held_id,
        food_store: p.food,
        transition_hungry_cost: 0.0,
        has_carrot_seeds: st.has_carrot_seeds,
        row: row.map(|(x, y, uses)| (x, y, uses)),
    }) {
        PullCarrotRowAction::UseEmptyOnRow { x, y } => {
            let w = world.read().ok()?;
            let mut kind = NpcActivityKind::Craft;
            let mut detail = String::new();
            let mut game_ms = 200u32;
            if npc_commit_craft_live(
                &ShortCraftLiveIntent::UseAt {
                    x,
                    y,
                    target_id: 400,
                    actor_id: 0,
                },
                intent_tx,
                &w,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                seq,
            ) {
                return Some((kind, "consider_food_pull_carrot".into(), game_ms));
            }
        }
        PullCarrotRowAction::DropHeld => {
            let w = world.read().ok()?;
            let (dx, dy) = npc_empty_drop_xy(&w, p.x, p.y);
            if npc_drop_at(intent_tx, conn_id, dx, dy, None) {
                return Some((
                    NpcActivityKind::Craft,
                    "consider_food_drop_for_carrot".into(),
                    400,
                ));
            }
        }
        PullCarrotRowAction::None => {}
    }
    let knife_tiles: Vec<(i32, i32, i32)> = tiles
        .iter()
        .map(|t| (t.parent_id, t.x, t.y))
        .collect();
    if let KnifeStuffAction::UseOnTarget { x, y, target_id } =
        do_knife_stuff(p.held_id, &knife_tiles, p.x, p.y)
    {
        let w = world.read().ok()?;
        let mut kind = NpcActivityKind::Craft;
        let mut detail = String::new();
        let mut game_ms = 200u32;
        if npc_commit_craft_live(
            &ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id,
                actor_id: p.held_id,
            },
            intent_tx,
            &w,
            content,
            st,
            conn_id,
            p.x,
            p.y,
            p.food,
            p.moving,
            &mut kind,
            &mut detail,
            &mut game_ms,
            seq,
        ) {
            return Some((kind, format!("consider_food_knife {target_id}"), game_ms));
        }
    }
    for &(actor, target, dist, _max_new) in consider_making_food_short_crafts() {
        if actor == 0 {
            continue; // carrot row already handled
        }
        let Some((x, y, _)) = npc_closest_parent_cheb(&tiles, p.x, p.y, target, dist) else {
            continue;
        };
        if p.held_id != actor && p.held_id != 0 {
            continue;
        }
        if p.held_id == actor {
            let w = world.read().ok()?;
            let mut kind = NpcActivityKind::Craft;
            let mut detail = String::new();
            let mut game_ms = 200u32;
            if npc_commit_craft_live(
                &ShortCraftLiveIntent::UseAt {
                    x,
                    y,
                    target_id: target,
                    actor_id: actor,
                },
                intent_tx,
                &w,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                seq,
            ) {
                return Some((kind, format!("consider_food_short {actor}+{target}"), game_ms));
            }
        } else if let Some(out) = npc_try_craft_product_commit(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            actor,
            &tiles,
            &blocked,
            seq,
            "consider_food_short_actor",
        ) {
            return Some(out);
        }
    }
    let count_dry = consider_making_food_count_dry_from_tiles(&tiles, hx, hy, p.held_id);
    let count_corn = npc_count_parent_cheb(&tiles, hx, hy, EAR_OF_CORN, 30)
        + i32::from(p.held_id == EAR_OF_CORN);
    let count_shucked = npc_count_parent_cheb(&tiles, hx, hy, SHUCKED_CORN, 30)
        + i32::from(p.held_id == SHUCKED_CORN);
    let yum_1114 = content
        .get(SHUCKED_CORN)
        .map(|o| o.food_value >= 1)
        .unwrap_or(false);
    let corn_plan = consider_making_food_ear_of_corn_maker(
        yum_1114,
        count_dry,
        count_corn,
        count_shucked,
        &mut st.farm_task.ear_of_corn_maker,
    );
    if corn_plan.pick_ear {
        if let Some((x, y, _)) = npc_closest_parent_cheb(&tiles, p.x, p.y, CORN_PLANT, 30) {
            if p.held_id == 0 {
                let w = world.read().ok()?;
                let mut kind = NpcActivityKind::Craft;
                let mut detail = String::new();
                let mut game_ms = 200u32;
                if npc_commit_craft_live(
                    &ShortCraftLiveIntent::UseAt {
                        x,
                        y,
                        target_id: CORN_PLANT,
                        actor_id: 0,
                    },
                    intent_tx,
                    &w,
                    content,
                    st,
                    conn_id,
                    p.x,
                    p.y,
                    p.food,
                    p.moving,
                    &mut kind,
                    &mut detail,
                    &mut game_ms,
                    seq,
                ) {
                    return Some((kind, "consider_food_pick_ear".into(), game_ms));
                }
            }
        }
    }
    if corn_plan.shuck {
        if let Some(out) = npc_try_craft_product_commit(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            SHUCKED_CORN,
            &tiles,
            &blocked,
            seq,
            "consider_food_shuck",
        ) {
            return Some(out);
        }
    }
    if let Some(out) = npc_try_craft_product_commit(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        SHUCKED_CORN,
        &tiles,
        &blocked,
        seq,
        "consider_food_1114",
    ) {
        return Some(out);
    }
    if let Some(out) = npc_try_sharpie_food(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        MAKE_SHARPIE_FOOD_FAR_CALL_DISTANCE,
        &tiles,
        seq,
    ) {
        return Some(out);
    }
    let raw_rabbit = consider_making_food_raw_rabbit_count(
        npc_count_parent_cheb(&tiles, hx, hy, SKINNED_RABBIT, 25),
        p.held_id == SKINNED_RABBIT,
        npc_count_parent_cheb(&tiles, hx, hy, SKEWERED_RABBIT, 25),
        p.held_id == SKEWERED_RABBIT,
    );
    if consider_making_food_fire_food_on_extra_rabbit(raw_rabbit) {
        if let Some(out) = npc_try_hungry_make_fire_food(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            hx,
            hy,
            &tiles,
            &blocked,
            seq,
        ) {
            return Some(out);
        }
    }
    if let Some(out) = npc_try_hungry_do_baking(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        hx,
        hy,
        &tiles,
        &blocked,
        seq,
    ) {
        return Some(out);
    }
    if let Some(out) = npc_try_hungry_do_watering(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        hx,
        hy,
        &tiles,
        &blocked,
        seq,
    ) {
        return Some(out);
    }
    // Haxe L8601–8602 fillUpBerryBowl / cleanUpBowls / fillBeanBowl commented skip
    // Haxe L8603: countRawRabbit <= 1 && makeFireFood(1)
    if consider_making_food_fire_food_on_few_rabbit(raw_rabbit) {
        if let Some(out) = npc_try_hungry_make_fire_food(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            hx,
            hy,
            &tiles,
            &blocked,
            seq,
        ) {
            return Some(out);
        }
    }
    None
}

fn npc_try_hungry_make_fire_food(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    hx: i32,
    hy: i32,
    tiles: &[ScanTile],
    blocked: &std::collections::HashSet<(i32, i32)>,
    seq: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let map: Vec<FireFoodMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| FireFoodMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    let counts = fill_fire_food_counts_from_map(
        hx,
        hy,
        p.held_id,
        &map,
        FIRE_FOOD_HOME_RADIUS,
        true,
        st.has_corn_seeds,
        has_bean_seeds_from_scan(tiles),
    );
    let action = make_fire_food(&counts, &mut st.fire_rt, 1, 0.0, st.was_idle);
    npc_commit_hungry_food_action(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        tiles,
        blocked,
        seq,
        match action {
            FireFoodAction::CraftItem { object_id } => HungryFoodAct::Craft(object_id),
            FireFoodAction::ShortCraft { actor, target } => HungryFoodAct::Pair { actor, target },
            FireFoodAction::ShortCraftOnGround { target } => HungryFoodAct::OnGround { target },
            FireFoodAction::None | FireFoodAction::Abort => HungryFoodAct::None,
        },
        "consider_food_firefood",
    )
}

fn npc_try_hungry_do_baking(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    hx: i32,
    hy: i32,
    tiles: &[ScanTile],
    blocked: &std::collections::HashSet<(i32, i32)>,
    seq: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let map: Vec<BakeMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| BakeMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
            uses: t.uses.max(1),
            floor_id: t.floor_id,
            is_food: t.is_food,
            is_permanent: t.is_permanent,
        })
        .collect();
    let origin_floor = tiles
        .iter()
        .find(|t| t.x == hx && t.y == hy)
        .map(|t| t.floor_id)
        .unwrap_or(0);
    let counts = fill_bake_counts_from_map_ex(
        hx,
        hy,
        p.held_id,
        p.held_uses.max(1),
        &map,
        30,
        true,
        st.has_corn_seeds,
        has_bean_seeds_from_scan(tiles),
        origin_floor,
    );
    let action = do_baking(
        &counts,
        &mut st.baker_rt,
        &mut st.baker_task,
        1,
        0.0,
        st.was_idle,
        0,
    );
    npc_commit_hungry_food_action(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        tiles,
        blocked,
        seq,
        match action {
            BakeAction::CraftItem { object_id } => HungryFoodAct::Craft(object_id),
            BakeAction::ShortCraft { actor, target } => HungryFoodAct::Pair { actor, target },
            _ => HungryFoodAct::None,
        },
        "consider_food_bake",
    )
}

fn npc_try_hungry_do_watering(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    hx: i32,
    hy: i32,
    tiles: &[ScanTile],
    blocked: &std::collections::HashSet<(i32, i32)>,
    seq: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let map: Vec<FarmMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| FarmMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
            uses: t.uses.max(1),
            floor_id: t.floor_id,
            is_food: t.is_food,
            is_permanent: t.is_permanent,
        })
        .collect();
    let counts = fill_farm_counts_from_map_ex(
        hx,
        hy,
        p.held_id,
        &map,
        30,
        true,
        basic_farmer_weight_from_runtime(&st.farm_rt),
        None,
    );
    let action = do_watering(
        &mut st.farm_rt,
        &counts,
        &mut st.farm_task,
        1,
        0.0,
        st.was_idle,
    );
    npc_commit_hungry_food_action(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        tiles,
        blocked,
        seq,
        match action {
            FarmAction::CraftItem { object_id } => HungryFoodAct::Craft(object_id),
            FarmAction::ShortCraft { actor, target } => HungryFoodAct::Pair { actor, target },
            _ => HungryFoodAct::None,
        },
        "consider_food_water",
    )
}

#[derive(Clone, Copy)]
enum HungryFoodAct {
    None,
    Craft(i32),
    Pair { actor: i32, target: i32 },
    OnGround { target: i32 },
}

fn npc_commit_hungry_food_action(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    tiles: &[ScanTile],
    blocked: &std::collections::HashSet<(i32, i32)>,
    seq: i32,
    act: HungryFoodAct,
    tag: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    match act {
        HungryFoodAct::None => None,
        HungryFoodAct::Craft(object_id) => npc_try_craft_product_commit(
            intent_tx,
            world,
            content,
            craft_graph,
            st,
            conn_id,
            p,
            tick,
            is_smith,
            object_id,
            tiles,
            blocked,
            seq,
            tag,
        ),
        HungryFoodAct::Pair { .. } | HungryFoodAct::OnGround { .. } => {
            let (actor, target) = match act {
                HungryFoodAct::Pair { actor, target } => (actor, target),
                HungryFoodAct::OnGround { target } => (p.held_id.max(0), target),
                HungryFoodAct::None | HungryFoodAct::Craft(_) => return None,
            };
            let Some((x, y, _)) = npc_closest_parent_cheb(tiles, p.x, p.y, target, 30) else {
                if actor > 0 && p.held_id != actor {
                    return npc_try_craft_product_commit(
                        intent_tx,
                        world,
                        content,
                        craft_graph,
                        st,
                        conn_id,
                        p,
                        tick,
                        is_smith,
                        actor,
                        tiles,
                        blocked,
                        seq,
                        tag,
                    );
                }
                return None;
            };
            if p.held_id == actor || (actor == 0 && p.held_id == 0) {
                let w = world.read().ok()?;
                let mut kind = NpcActivityKind::Craft;
                let mut detail = String::new();
                let mut game_ms = 200u32;
                if npc_commit_craft_live(
                    &ShortCraftLiveIntent::UseAt {
                        x,
                        y,
                        target_id: target,
                        actor_id: actor,
                    },
                    intent_tx,
                    &w,
                    content,
                    st,
                    conn_id,
                    p.x,
                    p.y,
                    p.food,
                    p.moving,
                    &mut kind,
                    &mut detail,
                    &mut game_ms,
                    seq,
                ) {
                    return Some((kind, format!("{tag} {actor}+{target}"), game_ms));
                }
                None
            } else if actor > 0 {
                npc_try_craft_product_commit(
                    intent_tx,
                    world,
                    content,
                    craft_graph,
                    st,
                    conn_id,
                    p,
                    tick,
                    is_smith,
                    actor,
                    tiles,
                    blocked,
                    seq,
                    tag,
                )
            } else {
                None
            }
        }
    }
}

fn npc_count_parent_cheb(tiles: &[ScanTile], ox: i32, oy: i32, id: i32, r: i32) -> i32 {
    tiles
        .iter()
        .filter(|t| {
            t.parent_id == id && (t.x - ox).abs().max((t.y - oy).abs()) <= r
        })
        .count() as i32
}

fn npc_closest_parent_cheb(
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    id: i32,
    r: i32,
) -> Option<(i32, i32, i32)> {
    let mut best: Option<(i32, i32, i32, i32)> = None;
    for t in tiles {
        if t.parent_id != id {
            continue;
        }
        let d = (t.x - px).abs().max((t.y - py).abs());
        if d > r {
            continue;
        }
        match best {
            None => best = Some((d, t.y, t.x, t.uses.max(1))),
            Some((bd, by, bx, _)) => {
                if d < bd || (d == bd && (t.y < by || (t.y == by && t.x < bx))) {
                    best = Some((d, t.y, t.x, t.uses.max(1)));
                }
            }
        }
    }
    best.map(|(_, y, x, uses)| (x, y, uses))
}

fn consider_making_food_count_dry_from_tiles(
    tiles: &[ScanTile],
    hx: i32,
    hy: i32,
    held_id: i32,
) -> i32 {
    npc_count_parent_cheb(tiles, hx, hy, DRIED_CORN, 30)
        + i32::from(held_id == DRIED_CORN)
        + 2 * (npc_count_parent_cheb(tiles, hx, hy, PILE_DRIED_CORN, 30)
            + i32::from(held_id == PILE_DRIED_CORN))
}

fn npc_try_sharpie_food(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    max_d: i32,
    tiles: &[ScanTile],
    seq: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let FarmAction::CraftItem { object_id } = make_sharpie_food_from_xy(
        p.x,
        p.y,
        p.held_id,
        tiles.iter().map(|t| (t.parent_id, t.x, t.y)),
        max_d,
    ) else {
        return None;
    };
    tracing::debug!(
        conn_id,
        held = p.held_id,
        object_id,
        max_d,
        x = p.x,
        y = p.y,
        "ai_craft: makeSharpieFood want"
    );
    let blocked = st.path_reach.blocked_coords(None);
    npc_try_craft_product_commit(
        intent_tx,
        world,
        content,
        craft_graph,
        st,
        conn_id,
        p,
        tick,
        is_smith,
        object_id,
        tiles,
        &blocked,
        seq,
        "make_sharpie_food",
    )
}

fn npc_try_craft_product_commit(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &RwLock<World>,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    is_smith: bool,
    object_id: i32,
    tiles: &[ScanTile],
    blocked: &std::collections::HashSet<(i32, i32)>,
    seq: i32,
    tag: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    let intent = npc_expand_craft_product(
        &world.read().unwrap(),
        tiles,
        p.x,
        p.y,
        p.held_id,
        p.moving,
        p.home_x,
        p.home_y,
        content,
        craft_graph,
        &mut st.craft_rt,
        blocked,
        is_smith,
        tick,
        object_id,
    );
    let mut kind = NpcActivityKind::Craft;
    let mut detail = String::new();
    let mut game_ms = 200u32;
    let w = world.read().ok()?;
    if npc_commit_craft_live(
        &intent,
        intent_tx,
        &w,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        seq,
    ) {
        if detail.is_empty() {
            detail = format!("{tag} {object_id}");
        }
        Some((kind, detail, game_ms))
    } else {
        None
    }
}

/// True when expand produced a Haxe `craftItem` success (USE/DROP/MOVE/busy Wait).
/// Idle `Wait` is not progress — FromQueue callers re-push the task.
// Haxe: craftItem isMoving return true; else fail → craftingTasks.push
fn npc_craft_expand_progress(intent: &ShortCraftLiveIntent, moving: bool) -> bool {
    match intent {
        ShortCraftLiveIntent::Wait => moving,
        ShortCraftLiveIntent::UseAt { .. }
        | ShortCraftLiveIntent::UseOnEmptyGround { .. }
        | ShortCraftLiveIntent::DropAt { .. }
        | ShortCraftLiveIntent::Goto { .. }
        | ShortCraftLiveIntent::PickupNearForge { .. }
        | ShortCraftLiveIntent::GotoForge { .. } => true,
        _ => false,
    }
}

/// Commit USE/DROP/MOVE/Wait from a craft expand. False → caller may requeue the task.
fn npc_commit_craft_live(
    intent: &ShortCraftLiveIntent,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    kind: &mut NpcActivityKind,
    detail: &mut String,
    game_ms: &mut u32,
    seq: i32,
) -> bool {
    match *intent {
        ShortCraftLiveIntent::Wait => {
            if !moving {
                return false;
            }
            *kind = NpcActivityKind::Craft;
            *detail = "craft_queue_wait".into();
            *game_ms = 200;
            true
        }
        ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id,
            actor_id,
        } => {
            let ground_id = world.get_object(x, y);
            if npc_reject_bare_craft_use(content, st, x, y, actor_id, ground_id, target_id) {
                return false;
            }
            let expected = if target_id != 0 {
                sticky_parent_id(content, target_id)
            } else {
                sticky_parent_id(content, world.get_object(x, y))
            };
            if npc_do_arrive_or_walk(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                x,
                y,
                expected,
                actor_id,
                StickyArrive::Use,
                format!("use_held @{x},{y}"),
                seq,
            ) {
                *kind = NpcActivityKind::Craft;
                *detail = if npc_is_close_action(px, py, x, y) && !moving {
                    format!("craft_queue_use @{x},{y}")
                } else if npc_is_close_action(px, py, x, y) {
                    format!("craft_queue_wait_use @{x},{y}")
                } else {
                    format!("craft_queue_walk_use @{x},{y}")
                };
                *game_ms = if npc_is_close_action(px, py, x, y) && !moving {
                    500
                } else {
                    250
                };
                return true;
            }
            false
        }
        ShortCraftLiveIntent::UseOnEmptyGround { x, y, held: actor_id } => {
            let expected = sticky_parent_id(content, world.get_object(x, y));
            if npc_do_arrive_or_walk(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                x,
                y,
                expected,
                actor_id,
                StickyArrive::Use,
                format!("use_held @{x},{y}"),
                seq,
            ) {
                *kind = NpcActivityKind::Craft;
                *detail = if npc_is_close_action(px, py, x, y) && !moving {
                    format!("craft_queue_use @{x},{y}")
                } else {
                    format!("craft_queue_walk_use @{x},{y}")
                };
                *game_ms = 250;
                return true;
            }
            false
        }
        ShortCraftLiveIntent::DropAt { x, y }
        | ShortCraftLiveIntent::Goto { x, y }
        | ShortCraftLiveIntent::PickupNearForge { x, y, .. } => {
            let is_drop = matches!(
                intent,
                ShortCraftLiveIntent::DropAt { .. }
                    | ShortCraftLiveIntent::PickupNearForge { .. }
            );
            if npc_is_close_action(px, py, x, y) && !is_drop {
                // Goto arrived — hold the tick so the next expand can USE/DROP.
                // Returning false used to fall through to explore and never craft.
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_arrived @{x},{y}");
                *game_ms = 200;
                clear_sticky_move(st);
                return true;
            }
            let arrive = if is_drop {
                StickyArrive::Drop
            } else {
                StickyArrive::None
            };
            let expected = sticky_parent_id(content, world.get_object(x, y));
            // Haxe GetOrCraftItem L6215 dropTarget = clothing obj; isDropingItem
            // then pickup_cloth so 128 on the ground is DROPped, not recrafted.
            // Food-clear drops must keep that label so the parked rope/reed
            // sticky is restored before isPickingupFood grabs the berry.
            let drop_label = if is_drop && st.hand_clear_pending {
                st.hand_clear_pending = false;
                format!("drop_held_clear @{x},{y}")
            } else {
                npc_drop_arrive_label(content, expected, x, y, is_drop)
            };
            if npc_do_arrive_or_walk(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                x,
                y,
                expected,
                0,
                arrive,
                drop_label.clone(),
                seq,
            ) {
                *kind = NpcActivityKind::Craft;
                *detail = if npc_is_close_action(px, py, x, y) && !moving && is_drop {
                    drop_label
                } else if is_drop && npc_clothing_slot(content, expected).is_some() {
                    drop_label
                } else {
                    format!("craft_queue_walk @{x},{y}")
                };
                *game_ms = if npc_is_close_action(px, py, x, y) && !moving && is_drop {
                    400
                } else {
                    250
                };
                return true;
            }
            false
        }
        ShortCraftLiveIntent::GotoForge {
            forge_x, forge_y, ..
        } => {
            if npc_is_close_action(px, py, forge_x, forge_y) {
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_forge_arrived @{forge_x},{forge_y}");
                *game_ms = 200;
                clear_sticky_move(st);
                return true;
            }
            if npc_try_walk_to_arrive(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                forge_x,
                forge_y,
                food,
                0,
                0,
                StickyArrive::None,
                moving,
                format!("forge @{forge_x},{forge_y}"),
                seq,
            ) {
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_forge @{forge_x},{forge_y}");
                *game_ms = 250;
                true
            } else {
                false
            }
        }
        ShortCraftLiveIntent::SelfClothing { slot } => {
            let payload = self_clothing_raw_payload(slot);
            if intent_tx
                .try_send(NetIntent::Raw {
                    conn_id,
                    tag: "SELF".into(),
                    payload,
                })
                .is_ok()
            {
                *kind = NpcActivityKind::Craft;
                *detail = format!("clothing_self slot={slot}");
                *game_ms = 400;
                true
            } else {
                false
            }
        }
        ShortCraftLiveIntent::Kill {
            target_p_id,
            x,
            y,
        } => {
            if npc_say_raw(intent_tx, conn_id, "KILL", &format!("{x} {y} {target_p_id}")) {
                st.food_goto.did_not_reach_food = 0.0;
                *kind = NpcActivityKind::Combat;
                *detail = format!("attack_kill target={target_p_id} @{},{}", x, y);
                *game_ms = 400;
                true
            } else {
                false
            }
        }
        ShortCraftLiveIntent::SeekOrCraft { .. } => {
            // Staging leftover: expand already ran. Pretending success caused the
            // clothing CraftItem(128) no-op loop (no walk / no USE).
            false
        }
        _ => false,
    }
}

/// Walk + record sticky goal so mid-path thinks do not replan unless target invalid.
fn npc_try_walk_to_sticky(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    expected_parent_id: i32,
    pending_use: bool,
    moving: bool,
    label: impl Into<String>,
    seq: i32,
) -> bool {
    npc_try_walk_to_arrive(
        intent_tx,
        world,
        content,
        st,
        conn_id,
        px,
        py,
        gx,
        gy,
        food_store,
        expected_parent_id,
        0,
        if pending_use {
            StickyArrive::Use
        } else {
            StickyArrive::None
        },
        moving,
        label,
        seq,
    )
}

/// Haxe `gotoObj` receding abort before a new MOVE.
///
/// Same goal, quad distance not improved, and quad > 100 → hostile path and
/// false (caller clears `dropTarget`). Snapshot lag of ≤0.5 does not abort.
/// Not called while `isMoving` on that same goal.
// Haxe: AiHelper.gotoObj L1085–1091; isDropingItem L8428 `if (isMoving()) return`
fn npc_goto_obj_receding_abort(
    st: &mut NpcProfessionState,
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    parent_id: i32,
) -> bool {
    let tgt = LastGotoObj::new(gx, gy, parent_id);
    match plan_goto_obj(
        st.food_goto.last_goto,
        st.food_goto.last_goto_dist,
        tgt,
        px,
        py,
    ) {
        GotoObjPlan::AbortReceding { dist_quad }
            if dist_quad > st.food_goto.last_goto_dist + 0.5 =>
        {
            st.path_reach.add_object_with_hostile_path(gx, gy);
            st.food_goto.last_goto = None;
            st.food_goto.last_goto_dist = -1.0;
            clear_sticky_move(st);
            true
        }
        GotoObjPlan::AbortReceding { dist_quad } | GotoObjPlan::Proceed { dist_quad } => {
            st.food_goto.last_goto = Some(tgt);
            st.food_goto.last_goto_dist = dist_quad;
            false
        }
    }
}

/// Haxe `isUsingItem` L8923: flowering / fruiting milkweed may replace plain milkweed.
fn npc_ground_matches_use_target(content: &ContentDb, target_id: i32, ground_id: i32) -> bool {
    if target_id == 0 {
        return true;
    }
    let expect = content.resolve_base_id(target_id);
    let ground = content.resolve_base_id(ground_id);
    if expect == ground {
        return true;
    }
    matches!(expect, 50 | 51 | 52) && matches!(ground, 50 | 51 | 52)
}

/// USE that is not a transition.
///
/// Empty-hand on a loose object is Haxe pickup (`TransitionHelper.use` L804).
/// Holding an object and USEing the tile it just left puts it back down.
/// That loop (Flint Chip 135) never reaches rope + reed or `switchCloths`.
/// Haxe `use()` false clears `transActor` / `transTarget` and `addNotReachable`
/// (isUsingItem L9107–L9133).
fn npc_craft_use_is_bare_swap(content: &ContentDb, actor_id: i32, ground_id: i32) -> bool {
    if content
        .find_transition_prefer(actor_id, ground_id, false)
        .is_some()
        || content.find_transition_max_use(actor_id, ground_id).is_some()
    {
        return false;
    }
    let actor = content.resolve_base_id(actor_id);
    let ground = content.resolve_base_id(ground_id);
    if actor == 0 && ground != 0 {
        let permanent = content.get(ground).map(|d| d.permanent).unwrap_or(false);
        if !permanent {
            return false;
        }
    }
    true
}

/// Reject a craft USE Haxe would fail. True → caller must not send USE.
fn npc_reject_bare_craft_use(
    content: &ContentDb,
    st: &mut NpcProfessionState,
    x: i32,
    y: i32,
    actor_id: i32,
    ground_id: i32,
    target_id: i32,
) -> bool {
    let bare = npc_craft_use_is_bare_swap(content, actor_id, ground_id);
    let mismatch = !npc_ground_matches_use_target(content, target_id, ground_id);
    if !bare && !mismatch {
        return false;
    }
    st.path_reach.add_not_reachable(x, y, 90.0);
    st.craft_rt.item.clear_trans();
    clear_sticky_move(st);
    true
}

/// Walk toward `(gx,gy)` and stage USE/DROP on arrival.
///
/// If already moving to the same tile, do **not** enqueue a replacement MOVE
/// (Haxe `isMoving()` return — a new MOVE truncates the remaining path).
fn npc_try_walk_to_arrive(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    expected_parent_id: i32,
    use_actor_parent: i32,
    arrive: StickyArrive,
    moving: bool,
    label: impl Into<String>,
    seq: i32,
) -> bool {
    let label = label.into();
    let (gx, gy) = npc_wrap_goal(world, px, py, gx, gy);
    // Haxe: if (isMoving()) return true — never goto the same dest while newMoves != null.
    // New dest (follow / danger) may send. Haxe: AiBase.isUsingItem L9013; MoveHelper.isMoveing L65
    if let Some(s) = st.sticky_move.as_ref() {
        let same = s.gx == gx && s.gy == gy;
        // Sim rejected the last MOVE (EmptyPath / blocked). Resending from this
        // tile cancels the path, so isDropingItem never reaches DROP L8456.
        if same && npc_prior_move_never_started(s.move_from, px, py, moving) {
            return false;
        }
        if !npc_should_send_move(moving, same) {
            set_sticky_arrive(
                st,
                gx,
                gy,
                expected_parent_id,
                use_actor_parent,
                arrive,
                label,
            );
            return true;
        }
    }
    // Haxe gotoObj: a drop/use walk that is not getting closer (quad > 100)
    // is a hostile path. Live 0.3.38 kept resending drop_held_walk while the
    // body stepped away and starved, so switchCloths never ran again.
    if npc_goto_obj_receding_abort(st, px, py, gx, gy, expected_parent_id) {
        return false;
    }
    let ok = npc_try_walk_to_ex(
        intent_tx,
        world,
        content,
        conn_id,
        px,
        py,
        gx,
        gy,
        food_store,
        st.food_goto.did_not_reach_food,
        st.animal_path,
        arrive != StickyArrive::None,
        st.try_move_nearest_tile_first,
        true,
        seq,
    );
    if ok {
        set_sticky_arrive(
            st,
            gx,
            gy,
            expected_parent_id,
            use_actor_parent,
            arrive,
            label,
        );
        if let Some(s) = st.sticky_move.as_mut() {
            s.move_from = Some((px, py));
        }
        // Haxe move() sets newMoves this tick → isMoving() true immediately.
        st.move_sent_tick = st.now_tick;
    }
    ok
}

/// Haxe `isMoving()`: `MoveHelper.newMoves != null`.
/// Snapshot `moving` is `move_path.is_some()`. Same sim tick as `move()` also
/// counts — Haxe sets newMoves in `MoveHelper.Move` before the next think.
// Haxe: MoveHelper.isMoveing L65; GlobalPlayerInstance.isMoving L1632–1634
fn npc_haxe_is_moving(snapshot_moving: bool, move_sent_sim_tick: u64, sim_tick: u64) -> bool {
    snapshot_moving || (move_sent_sim_tick > 0 && move_sent_sim_tick == sim_tick)
}

/// When to enqueue a MOVE (Haxe `gotoObj` / `gotoAdv` / `isUsingItem`).
///
/// Send if dest changed (follow / danger). If dest is unchanged, send only when
/// `!isMoving()`. No wall-clock hold.
// Haxe: AiBase.isUsingItem L9013 `if (myPlayer.isMoving()) return true;` then goto if dist>1
fn npc_should_send_move(moving: bool, same_target: bool) -> bool {
    if !same_target {
        return true;
    }
    !moving
}

/// Last MOVE toward this goal was sent from the tile the body is still on, and
/// `isMoving` never became true. Haxe `gotoObj` then returns false.
// Haxe: AiBase.isDropingItem L8442–8446
fn npc_prior_move_never_started(
    move_from: Option<(i32, i32)>,
    px: i32,
    py: i32,
    moving: bool,
) -> bool {
    matches!(move_from, Some((fx, fy)) if fx == px && fy == py && !moving)
}

/// Haxe isUsingItem / isDropingItem: USE or DROP when orthogonally close and stopped;
/// otherwise walk with sticky. USE while moving is refused and **cancels** the path.
fn npc_do_arrive_or_walk(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    x: i32,
    y: i32,
    expected_parent: i32,
    actor_parent: i32,
    arrive: StickyArrive,
    label: impl Into<String>,
    seq: i32,
) -> bool {
    let label = label.into();
    let (x, y) = npc_wrap_goal(world, px, py, x, y);
    if npc_is_close_action(px, py, x, y) {
        if moving {
            // Haxe isUsingItem L9013: if isMoving() return true — wait, then USE/DROP.
            set_sticky_arrive(
                st,
                x,
                y,
                expected_parent,
                actor_parent,
                arrive,
                label,
            );
            return true;
        }
        let ok = match arrive {
            StickyArrive::Drop => {
                if drop_target_uses_use_not_drop(expected_parent) {
                    npc_use_at(intent_tx, conn_id, x, y, None, None)
                } else {
                    npc_drop_at(intent_tx, conn_id, x, y, None)
                }
            }
            StickyArrive::Use => npc_use_at(intent_tx, conn_id, x, y, None, None),
            StickyArrive::None => {
                clear_sticky_move(st);
                return true;
            }
        };
        if ok {
            clear_sticky_move(st);
            if arrive == StickyArrive::Use {
                npc_stage_clothing_pickup_if_use_makes_cloth(
                    content,
                    st,
                    actor_parent,
                    expected_parent,
                    x,
                    y,
                );
            }
        }
        return ok;
    }
    npc_try_walk_to_arrive(
        intent_tx,
        world,
        content,
        st,
        conn_id,
        px,
        py,
        x,
        y,
        food,
        expected_parent,
        actor_parent,
        arrive,
        moving,
        label,
        seq,
    )
}

/// Apply Haxe `isUsingItem` / `isDropingItem` for a staged sticky goal.
/// Holding a player: drop at feet (keep sticky) so the next think can USE/DROP the tile.
fn npc_apply_sticky_arrive(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    held_id: i32,
    holding_player_id: i32,
    sticky: &NpcStickyMove,
    seq: i32,
    held_uses: i32,
    home_x: i32,
    home_y: i32,
    age: f32,
) -> Option<(NpcActivityKind, String, u32)> {
    let arrive = StickyArrive::from_flags(sticky.pending_use, sticky.pending_drop);
    if arrive == StickyArrive::None {
        return None;
    }
    if arrive == StickyArrive::Use {
        if let Some(out) = npc_using_item_head_live(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            px,
            py,
            held_id,
            held_uses,
            home_x,
            home_y,
            sticky,
        ) {
            return Some(out);
        }
        if !st
            .sticky_move
            .as_ref()
            .is_some_and(|s| s.pending_use)
        {
            return None;
        }
        if !moving && food >= 0.0 {
            let world_id = world.get_object(sticky.gx, sticky.gy);
            let world_parent = sticky_parent_id(content, world_id);
            let is_animal = content
                .get(world_parent)
                .map(|d| d.is_animal())
                .unwrap_or(false);
            if is_using_item_bow_on_animal(held_id, is_animal) {
                let dx = px - sticky.gx;
                let dy = py - sticky.gy;
                let quad = dx * dx + dy * dy;
                let ud = content
                    .get(USE_BOW_AND_ARROW)
                    .map(|d| d.use_distance as f32)
                    .unwrap_or(1.0);
                if !kill_animal_needs_stand_off(quad, ud)
                    && npc_use_at(intent_tx, conn_id, sticky.gx, sticky.gy, None, None)
                {
                    st.animal_target = Some((sticky.gx, sticky.gy, world_parent));
                    return Some((
                        NpcActivityKind::Combat,
                        format!("use_bow_animal {world_parent} @{},{}", sticky.gx, sticky.gy),
                        400,
                    ));
                }
            }
        }
    }
    let holding = npc_holding_player(held_id, holding_player_id);
    let plan = npc_plan_sticky_arrive(
        px,
        py,
        sticky.gx,
        sticky.gy,
        moving,
        holding,
        held_id,
        sticky.use_actor_parent,
        arrive,
    );
    match plan {
        StickyActPlan::WaitUntilStopped => Some((
            NpcActivityKind::Move,
            format!("walk_use {}", sticky.label),
            250,
        )),
        StickyActPlan::DropHeldPlayerAtFeet => {
            if npc_drop_at(intent_tx, conn_id, px, py, None) {
                Some((
                    NpcActivityKind::Baby,
                    format!(
                        "drop_baby_for_use child={} @{},{}",
                        holding_player_id, sticky.gx, sticky.gy
                    ),
                    400,
                ))
            } else {
                None
            }
        }
        StickyActPlan::DropHeldForEmptyHand => {
            // Haxe dropHeldObject(0): drop on empty ground next to the player, not SWAP on a full tile.
            let (dx, dy) = npc_empty_drop_xy(world, px, py);
            if npc_drop_at(intent_tx, conn_id, dx, dy, None) {
                Some((
                    NpcActivityKind::Craft,
                    format!(
                        "drop_held_for_empty_use held={} @{},{}",
                        held_id, sticky.gx, sticky.gy
                    ),
                    400,
                ))
            } else {
                None
            }
        }
        StickyActPlan::DropTarget => {
            // Haxe L8451: Extracted Arrowhead Wound uses `use` not `drop`.
            if drop_target_uses_use_not_drop(sticky.expected_parent_id) {
                if npc_use_at(intent_tx, conn_id, sticky.gx, sticky.gy, None, None) {
                    clear_sticky_move(st);
                    return Some((
                        NpcActivityKind::Craft,
                        format!("drop_wound_use @{},{}", sticky.gx, sticky.gy),
                        500,
                    ));
                }
                return None;
            }
            if npc_drop_at(intent_tx, conn_id, sticky.gx, sticky.gy, None) {
                let resume = if sticky.label.starts_with("drop_held_clear") {
                    st.resume_drop.take()
                } else {
                    st.resume_drop = None;
                    None
                };
                clear_sticky_move(st);
                if let Some(prev) = resume {
                    st.sticky_move = Some(prev);
                }
                let detail = if sticky.label.starts_with("pickup_cloth") {
                    format!("{} @{},{}", sticky.label, sticky.gx, sticky.gy)
                } else {
                    format!("drop_held_arrive @{},{}", sticky.gx, sticky.gy)
                };
                Some((NpcActivityKind::Craft, detail, 500))
            } else {
                None
            }
        }
        StickyActPlan::UseTarget => {
            let held_ok = sticky.use_actor_parent == 0
                || sticky_parent_id(content, held_id) == sticky.use_actor_parent;
            if !held_ok {
                return None;
            }
            if npc_use_at(intent_tx, conn_id, sticky.gx, sticky.gy, None, None) {
                if is_using_item_drop_is_a_use_done(st.drop_is_a_use) {
                    npc_cancle_use(st);
                    return Some((
                        NpcActivityKind::Craft,
                        format!("drop_as_use @{},{}", sticky.gx, sticky.gy),
                        500,
                    ));
                }
                let world_id = world.get_object(sticky.gx, sticky.gy);
                let ground = sticky_parent_id(content, world_id);
                let actor = sticky_parent_id(content, held_id);
                let target = sticky.expected_parent_id;
                let product = st.craft_rt.item.product_id;
                let _ = note_using_item_craft_progress(
                    product,
                    &mut st.craft_rt.item.count_done,
                    &mut st.craft_rt.item.count_transitions_done,
                    &mut st.craft_rt.item.last_actor_id,
                    &mut st.craft_rt.item.last_target_id,
                    &mut st.craft_rt.item.last_new_actor_id,
                    &mut st.craft_rt.item.last_new_target_id,
                    actor,
                    target,
                    actor,
                    ground,
                );
                st.craft_rt.item.clear_trans();
                note_raw_pie_crafted(&mut st.baker_rt, ground);
                if is_using_item_goose_stump_speedup(ground) {
                    st.think_time_sec -= 1.0;
                }
                clear_sticky_move(st);
                npc_stage_clothing_pickup_if_use_makes_cloth(
                    content,
                    st,
                    actor,
                    target,
                    sticky.gx,
                    sticky.gy,
                );
                Some((
                    NpcActivityKind::Craft,
                    format!("use_held_arrive @{},{}", sticky.gx, sticky.gy),
                    500,
                ))
            } else {
                // Haxe L9107–9137: CancleUse, clear trans, Too hot / food / age mark, return true.
                npc_cancle_use(st);
                st.craft_rt.item.clear_trans();
                let fail = is_using_item_use_fail(age, "");
                if fail.handle_temperature {
                    st.handling_temperature = true;
                } else if fail.set_hungry {
                    st.was_hungry = true;
                } else {
                    mark_use_path_fail(&mut st.path_reach, sticky.gx, sticky.gy, age);
                }
                Some((
                    NpcActivityKind::Think,
                    format!("use_fail_mark @{},{}", sticky.gx, sticky.gy),
                    100,
                ))
            }
        }
        StickyActPlan::Walk => {
            let sent_before = st.move_sent_tick;
            if npc_try_walk_to_arrive(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                sticky.gx,
                sticky.gy,
                food,
                sticky.expected_parent_id,
                sticky.use_actor_parent,
                arrive,
                moving,
                sticky.label.clone(),
                seq,
            ) {
                // In-flight MOVE (Haxe isMoving return) — do not log a new craft walk.
                if moving || st.move_sent_tick == sent_before {
                    return Some((
                        NpcActivityKind::Move,
                        format!("walk_use {}", sticky.label),
                        250,
                    ));
                }
                let tag = if arrive == StickyArrive::Drop {
                    "drop_held_walk"
                } else {
                    "use_held_walk"
                };
                Some((
                    NpcActivityKind::Craft,
                    format!("{tag} @{},{}", sticky.gx, sticky.gy),
                    250,
                ))
            } else {
                // Haxe L9033–9038: goto fail → CancleUse; return done (false).
                if arrive == StickyArrive::Use {
                    npc_cancle_use(st);
                }
                None
            }
        }
    }
}

/// Haxe `isDropingItem` L8351–8463: head gates then follow/stack/goto/use/drop.
// Haxe: AiBase.isDropingItem L8351–8463
fn npc_apply_dropping_item(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    held_id: i32,
    holding_player_id: i32,
    hungry: bool,
    has_player_to_follow: bool,
    sticky: &NpcStickyMove,
    seq: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let world_p = sticky_parent_id(content, world.get_object(sticky.gx, sticky.gy));
    let pickup_cloth = sticky.label.starts_with("pickup_cloth");
    // Haxe isStillExpectedItem uses the live tile parent. 0.3.11 kept the sticky
    // when 124 became 128, but is_dropping_item_head still TargetGone (124 != 128).
    let live_parent = match npc_clothing_drop_live_parent(
        content,
        sticky.expected_parent_id,
        world_p,
        pickup_cloth,
    ) {
        ClothingDropLive::WaitForProduct => {
            // Haxe triedDropCount only changes dropDistance, never aborts pickup.
            // Sharing it here cleared the sticky after any prior craft_queue_drop.
            st.clothing_wait_apply = st.clothing_wait_apply.saturating_add(1);
            if npc_clothing_wait_apply_give_up(st.clothing_wait_apply) {
                clear_sticky_move(st);
                return None;
            }
            return Some((
                NpcActivityKind::Craft,
                format!(
                    "pickup_cloth wait_apply {}",
                    sticky.expected_parent_id
                ),
                100,
            ));
        }
        ClothingDropLive::Parent(live) => live,
    };
    if let Some(s) = st.sticky_move.as_mut() {
        s.expected_parent_id = live_parent;
    }
    let mut sticky = sticky.clone();
    sticky.expected_parent_id = live_parent;
    let num_slots = content.get(live_parent).map(|o| o.num_slots).unwrap_or(0);
    let head = is_dropping_item_head(
        moving,
        Some((live_parent, sticky.gx, sticky.gy)),
        Some(world_p),
        held_id,
        num_slots,
        px,
        py,
        &mut st.tried_drop_count,
    );
    match head {
        IsDropingItemHead::Idle | IsDropingItemHead::TargetGone => {
            let resume = if sticky.label.starts_with("drop_held_clear") {
                st.resume_drop.take()
            } else {
                None
            };
            clear_sticky_move(st);
            if let Some(prev) = resume {
                st.sticky_move = Some(prev);
            }
            return None;
        }
        IsDropingItemHead::DropHeld { max_distance } => {
            return npc_drop_held_for_dropping_item(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                held_id,
                seq,
                max_distance,
            );
        }
        IsDropingItemHead::Continue => {}
    }
    let max_d = is_dropping_drop_distance_after_try(st.tried_drop_count);
    let dx = px - sticky.gx;
    let dy = py - sticky.gy;
    let dist = (dx * dx + dy * dy) as f32;
    match is_dropping_item_goto(
        sticky.expected_parent_id,
        dist,
        has_player_to_follow,
        hungry,
        max_d,
        moving,
    ) {
        IsDropingItemGoto::DropHeld { max_distance } => npc_drop_held_for_dropping_item(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            px,
            py,
            food,
            moving,
            held_id,
            seq,
            max_distance,
        ),
        IsDropingItemGoto::ConvertToUse => {
            st.drop_is_a_use = true;
            set_sticky_arrive(
                st,
                sticky.gx,
                sticky.gy,
                sticky.expected_parent_id,
                0,
                StickyArrive::Use,
                "drop_as_use_stack",
            );
            if npc_haxe_close_use_prio(px, py, sticky.gx, sticky.gy) {
                let use_sticky = st.sticky_move.clone()?;
                npc_apply_sticky_arrive(
                    intent_tx,
                    world,
                    content,
                    st,
                    conn_id,
                    px,
                    py,
                    food,
                    moving,
                    held_id,
                    holding_player_id,
                    &use_sticky,
                    seq,
                    0,
                    px,
                    py,
                    20.0,
                )
            } else {
                None
            }
        }
        IsDropingItemGoto::Idle => {
            clear_sticky_move(st);
            None
        }
        IsDropingItemGoto::UseOnTarget => {
            let mut use_sticky = sticky.clone();
            use_sticky.pending_use = true;
            use_sticky.pending_drop = false;
            use_sticky.use_actor_parent = if held_id > 0 {
                sticky_parent_id(content, held_id)
            } else {
                0
            };
            npc_apply_sticky_arrive(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                held_id,
                holding_player_id,
                &use_sticky,
                seq,
                0,
                px,
                py,
                20.0,
            )
        }
        IsDropingItemGoto::WaitMoving
        | IsDropingItemGoto::WalkToTarget
        | IsDropingItemGoto::DropOnTarget => {
            let applied = npc_apply_sticky_arrive(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                px,
                py,
                food,
                moving,
                held_id,
                holding_player_id,
                &sticky,
                seq,
                0,
                px,
                py,
                20.0,
            );
            if applied.is_none() && matches!(
                is_dropping_item_goto(
                    sticky.expected_parent_id,
                    dist,
                    has_player_to_follow,
                    hungry,
                    max_d,
                    moving,
                ),
                IsDropingItemGoto::WalkToTarget
            ) {
                // Haxe L8442–8446: goto fail clears dropTarget but still returns true.
                // Also skip this tile next craft search (addNotReachable 90s) so a
                // rope behind snow-grey is not chosen again over same-side thread.
                st.path_reach
                    .add_not_reachable(sticky.gx, sticky.gy, 90.0);
                clear_sticky_move(st);
                return Some((
                    NpcActivityKind::Move,
                    format!("drop_goto_fail @{},{}", sticky.gx, sticky.gy),
                    250,
                ));
            }
            applied
        }
    }
}

fn npc_held_drop_is_open(world: &World, x: i32, y: i32) -> bool {
    world.get_object(x, y) == 0
        && !is_biome_blocking(world.get_biome(x, y), world.get_floor(x, y) as i32)
}

/// Held rope or yew shaft plus the other half of a skirt or bow, before a baby drop.
///
/// `isFeedingChild` used to `goto` the child while holding rope 59, then
/// `dropHeldObject(0)` on arrival. The skirt USE never landed, so `switchCloths`
/// never saw held 128 and `getWeapon` never got a 59+131 USE.
// Haxe: AiBase.isFeedingChild L6476; getWeapon L5815; switchCloths L8709
fn npc_try_pair_before_baby(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
) -> Option<(NpcActivityKind, String, u32)> {
    let held_base = content.resolve_base_id(p.held_id);
    if held_base != 59 && held_base != 131 {
        return None;
    }
    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
    let blocked = st.path_reach.blocked_coords(None);
    let reed = if held_base == 59 {
        npc_reed_skirt_direct(
            world,
            content,
            p.x,
            p.y,
            hx,
            hy,
            held_base,
            NPC_GET_OR_CRAFT_SEARCH_RADIUS,
            &blocked,
        )
    } else {
        None
    };
    let intent = reed.or_else(|| {
        npc_yew_bow_direct(
            world,
            content,
            p.x,
            p.y,
            hx,
            hy,
            held_base,
            NPC_GET_OR_CRAFT_SEARCH_RADIUS,
            &blocked,
        )
    })?;
    let tag = match intent {
        ShortCraftLiveIntent::UseAt { target_id: 124, .. } => "reed_skirt_before_baby",
        ShortCraftLiveIntent::UseAt {
            target_id: 131,
            actor_id: 59,
            ..
        }
        | ShortCraftLiveIntent::UseAt {
            target_id: 59,
            actor_id: 131,
            ..
        } => "yew_bow_before_baby",
        _ => "pair_before_baby",
    };
    let mut kind = NpcActivityKind::Craft;
    let mut detail = String::new();
    let mut game_ms = 200u32;
    if !npc_commit_craft_live(
        &intent,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        npc_client_move_seq(p.done_moving_seq),
    ) {
        return None;
    }
    if let Some(s) = st.sticky_move.as_mut() {
        if s.pending_use && !s.label.starts_with(tag) {
            s.label = format!("{tag} {}", s.label);
        }
    }
    Some((kind, format!("{tag} {detail}"), game_ms))
}

/// Haxe `dropHeldObject` does not drop in place. It aims at an empty tile and
/// `isDropingItem` DROPs only when stopped and orthogonally adjacent (L8456).
/// A far tile or the occupied feet tile is rejected or swaps the same berry.
// Haxe: AiBase.dropHeldObject L5284 / L5566–5592; isDropingItem L8456
fn npc_send_or_walk_empty_drop(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    held_id: i32,
    seq: i32,
    suspend_craft: bool,
) -> Option<(NpcActivityKind, String, u32)> {
    // Haxe L5284: nothing in hand → false, think continues.
    if held_id == 0 {
        return None;
    }
    let (dx, dy) = npc_empty_drop_xy(world, px, py);
    if !npc_held_drop_is_open(world, dx, dy) {
        return None;
    }
    if !moving && npc_is_close_action(px, py, dx, dy) {
        if !npc_drop_at(intent_tx, conn_id, dx, dy, None) {
            return None;
        }
        return Some((
            NpcActivityKind::Craft,
            format!("droping_drop_held placed @{dx},{dy}"),
            400,
        ));
    }
    if suspend_craft {
        let cur_is_clear = st
            .sticky_move
            .as_ref()
            .is_some_and(|s| s.label.starts_with("drop_held_clear"));
        if !cur_is_clear {
            if let Some(cur) = st.sticky_move.clone() {
                st.resume_drop = Some(cur);
            }
        }
    } else {
        st.resume_drop = None;
    }
    set_sticky_arrive(
        st,
        dx,
        dy,
        0,
        0,
        StickyArrive::Drop,
        "drop_held_clear",
    );
    if !moving {
        let _ = npc_try_walk_to_arrive(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            px,
            py,
            dx,
            dy,
            food,
            0,
            0,
            StickyArrive::Drop,
            false,
            "drop_held_clear",
            seq,
        );
    }
    Some((
        NpcActivityKind::Craft,
        format!("drop_held_clear @{dx},{dy}"),
        250,
    ))
}

/// Follow-too-far / container / too-far: drop the held object on empty ground.
/// The craft pickup stays in `resume_drop` so Reed Skirt 128 is not forgotten.
// Haxe: AiBase.isDropingItem L8401–8407 / L8371 / L8381 `return dropHeldObject`
fn npc_drop_held_for_dropping_item(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    px: i32,
    py: i32,
    food: f32,
    moving: bool,
    held_id: i32,
    seq: i32,
    max_distance: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    let out = npc_send_or_walk_empty_drop(
        intent_tx,
        world,
        content,
        st,
        conn_id,
        px,
        py,
        food,
        moving,
        held_id,
        seq,
        true,
    )?;
    if out.1.starts_with("drop_held_clear") {
        return Some(out);
    }
    // Adjacent DROP landed. Keep the rope / skirt goal (clearing it on the
    // 6th try forgot 128 while the berry was still in the hand).
    let _ = max_distance;
    Some(out)
}

/// Dual-pass Goto fail mark: animal-only block â†’ hostile_path 20s; else not_reachable 90s.
// Haxe: AiHelper.gotoAdv ~1116â€“1141
fn npc_mark_goto_path_fail(
    path_reach: &mut AiPathReachMaps,
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
    food_store: f32,
    did_not_reach_food: f32,
) {
    let consider = consider_animals_for_goto(true, did_not_reach_food, food_store);
    let outcome = goto_path_outcome(world, content, sx, sy, gx, gy, consider);
    let blocked_by_animal = matches!(outcome, GotoPathOutcome::BlockedByAnimal);
    mark_goto_path_fail(path_reach, gx, gy, blocked_by_animal);
}


/// PATH-REACH-MERGE: pull Player.ai_path_reach into NPC maps (max timers).
// Haxe: single AiBase maps â€” Rust dual ownership â†’ merge each think
// PATH-REACH-MERGE / dual_map_merge
fn pull_player_path_reach(st: &mut NpcProfessionState, snap: &PlayerSnapshot) {
    merge_path_reach_maps(&mut st.path_reach, &snap.ai_path_reach);
    if snap.ai_did_not_reach_food > st.food_goto.did_not_reach_food {
        st.food_goto.did_not_reach_food = snap.ai_did_not_reach_food;
    }
    if snap.ai_last_goto_obj_id != 0 {
        st.food_goto.last_goto = Some(LastGotoObj::new(
            snap.ai_last_goto_obj_x,
            snap.ai_last_goto_obj_y,
            snap.ai_last_goto_obj_id,
        ));
        st.food_goto.last_goto_dist = snap.ai_last_goto_obj_distance;
    }
    if snap.ai_sticky_food_id != 0 {
        st.food_goto.sticky_food = Some(StickyFoodTarget::new(
            snap.ai_sticky_food_x,
            snap.ai_sticky_food_y,
            snap.ai_sticky_food_id,
        ));
    }
}

/// PATH-REACH-MERGE: push NPC maps into player_views for tick_vitals absorb.
// Haxe: AiBase L85â€“86 single maps
fn push_npc_path_reach_to_views(
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    conn_id: u64,
    path_reach: &AiPathReachMaps,
) {
    if path_reach.is_empty() {
        return;
    }
    if let Ok(mut g) = player_views.write() {
        if let Some(s) = g.get_mut(&conn_id) {
            merge_path_reach_maps(&mut s.ai_path_reach, path_reach);
        }
    }
}

fn push_npc_food_goto_to_views(
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    conn_id: u64,
    food_goto: &NpcFoodGotoState,
) {
    if let Ok(mut g) = player_views.write() {
        if let Some(s) = g.get_mut(&conn_id) {
            s.ai_did_not_reach_food = food_goto.did_not_reach_food;
            if let Some(lg) = food_goto.last_goto {
                s.ai_last_goto_obj_id = lg.parent_id;
                s.ai_last_goto_obj_x = lg.x;
                s.ai_last_goto_obj_y = lg.y;
                s.ai_last_goto_obj_distance = food_goto.last_goto_dist;
            }
            if let Some(sf) = food_goto.sticky_food {
                s.ai_sticky_food_id = sf.parent_id;
                s.ai_sticky_food_x = sf.x;
                s.ai_sticky_food_y = sf.y;
            } else {
                s.ai_sticky_food_id = 0;
            }
        }
    }
}

/// Sticky isPickingupFood / gotoObj bookkeeping (AI-GOTO-FOOD).
// Haxe: AiBase.foodTarget / lastGotoObj / didNotReachFood
#[derive(Debug, Clone)]
struct NpcFoodGotoState {
    sticky_food: Option<StickyFoodTarget>,
    last_goto: Option<LastGotoObj>,
    last_goto_dist: f32,
    did_not_reach_food: f32,
    /// Async food USE/DROP/REMV tile awaiting apply result (AI-FOOD-FAIL-MARK).
    // Haxe: isPickingupFood use/remove/drop returns false sync â†’ mark 30s
    pending_food_xy: Option<(i32, i32)>,
    /// Pending was container REMV (basket food_value often 0 on ground id).
    // Haxe: isInContainer remove path ~8684
    pending_food_container: bool,
    /// Pending was isUse (permanent / foodValue&lt;1). Success does not require holding.
    // Haxe: AiBase.isPickingupFood L8703–8706 after use() true
    pending_food_is_use: bool,
}

impl Default for NpcFoodGotoState {
    fn default() -> Self {
        Self {
            sticky_food: None,
            last_goto: None,
            last_goto_dist: -1.0,
            did_not_reach_food: 0.0,
            pending_food_xy: None,
            pending_food_container: false,
            pending_food_is_use: false,
        }
    }
}

/// Sticky MOVE intent while path is in progress (Haxe useTarget / expectedUseTarget).
///
/// While moving, AI does **not** replan unless this goal becomes invalid
/// (object parent id changed / gone). Pure walks (`expected_parent_id == 0`)
/// stay valid until the path finishes.
// Haxe: AiBase.useTarget + expectedUseTarget; isUsingItem target-changed â†’ CancleUse
#[derive(Debug, Clone)]
struct NpcStickyMove {
    gx: i32,
    gy: i32,
    /// World parent id expected at goal; `0` = walk-only (no object check).
    expected_parent_id: i32,
    /// Haxe `useActor.parentId` when this sticky is a staged use (0 = walk-only).
    use_actor_parent: i32,
    /// Haxe `useHeldObjOnTarget` — after walk, issue USE instead of replanning.
    pending_use: bool,
    /// Haxe `dropTarget` / `isDropingItem` — after walk, issue DROP (pickup/swap).
    pending_drop: bool,
    /// Short label for activity log.
    label: String,
    /// Tile a MOVE was already sent from. Resending from the same tile cancels
    /// the in-flight path (NPC-thread snapshot lags `isMoving()`).
    // Haxe: AiBase.isUsingItem L9013 `if (isMoving()) return true`
    move_from: Option<(i32, i32)>,
}

/// Per-NPC sticky profession task state for ladder scan (Haxe AiBase profession fields).
#[derive(Debug)]
struct NpcProfessionState {
    farm_task: FarmTaskState,
    farm_rt: FarmProfessionRuntime,
    smith_rt: SmithProfessionRuntime,
    baker_rt: BakerProfessionRuntime,
    baker_task: BakerTaskState,
    shepherd_rt: ShepherdProfessionRuntime,
    pottery_rt: PotterProfessionRuntime,
    fire_rt: FireFoodProfessionRuntime,
    /// AI-HANDLING-FIRE: FIREKEEPER sticky (isHandlingFire).
    fire_keeper_rt: FireKeeperProfessionRuntime,
    /// AI-JOB-GRAVE: GRAVEKEEPER sticky (isHandlingGraves).
    grave_keeper_rt: GraveKeeperProfessionRuntime,
    /// AI-JOB-HUNT: HUNTER sticky (isHunting).
    hunter_rt: HunterProfessionRuntime,
    /// AI-JOB-LUMBER: LUMBERJACK sticky (isCuttingWood).
    lumberjack_rt: LumberjackProfessionRuntime,
    /// AI-JOB-COLLECT: COLLECTOR sticky (isCollecting).
    collector_rt: CollectorProfessionRuntime,
    /// AI-JOB-FOODSERVER: FOODSERVER sticky (isFeedingPlayerInNeed).
    foodserver_rt: FoodServerProfessionRuntime,
    /// PATH-REACH: Haxe AiBase notReachableObjects / objectsWithHostilePath (npc-local).
    // Haxe: AiBase L85â€“86; AiHelper.gotoAdv fail â†’ addNotReachable / addHostilePath
    path_reach: AiPathReachMaps,
    /// AI-GOTO-FOOD: sticky foodTarget + lastGotoObj + didNotReachFood.
    // Haxe: AiBase.foodTarget / lastGotoObj / didNotReachFood
    food_goto: NpcFoodGotoState,
    /// AI-CRAFT-NPC-ENQUEUE: sticky multi-step craftItem state (failedCraftings / itemToCraft).
    // Haxe: AiBase.itemToCraft + failedCraftings across GetOrCraftItem / craftItem ticks
    craft_rt: CraftAiRuntime,
    /// Haxe `AiBase.time` â€” reaction cooldown (seconds). Think only when â‰¤ 0.
    // Haxe: AiBase.time / doTimeStuff
    think_time_sec: f32,
    /// Haxe `lineage.prestigeClass` for reaction time selection.
    // Haxe: PrestigeClass Serf / Commoner / Noble
    prestige_class: PrestigeClass,
    /// True once role prestige class has been assigned for this body.
    class_assigned: bool,
    /// Active MOVE goal â€” validated while `PlayerSnapshot.moving`.
    sticky_move: Option<NpcStickyMove>,
    /// Craft pickup parked while `dropHeldObject` walks to an empty tile.
    /// Restored after the DROP so a far rope or skirt goal is not forgotten.
    // Haxe: AiBase.dropHeldObject L5566; isDropingItem L8456
    resume_drop: Option<NpcStickyMove>,
    /// Previous-tick hungry hysteresis for fill_live_sensors.
    // Haxe: AiBase.isHungry
    was_hungry: bool,
    /// Haxe CreateCollisionChunk player context for Goto animal footprints.
    animal_path: Option<AnimalPathPlayerCtx>,
    /// Haxe `lastProfession == 'TAILOR'` (hasOrBecomeProfession clothing).
    last_is_tailor: bool,
    /// Haxe `GlobalPlayerInstance.firePlace` sticky (npc-local).
    fire_place_id: i32,
    fire_place_x: i32,
    fire_place_y: i32,
    /// Haxe `removeFromContainerTarget` (AI-REMOVE-CONTAINER).
    remove_from_container: Option<RemoveFromContainerStaging>,
    /// Haxe `AiBase.isHandlingTemperature` / `justArrived` / `lastTemperature`.
    handling_temperature: bool,
    temp_just_arrived: bool,
    last_heat: f32,
    /// Haxe `coldPlace = null` after fail-cool (ignore snapshot until re-seeded).
    // Haxe: AiBase.handleTemperature L1720
    rejected_cold_place: Option<(i32, i32)>,
    /// Haxe `warmPlace = null` after fail-warm.
    // Haxe: AiBase.handleTemperature L1743
    rejected_warm_place: Option<(i32, i32)>,
    /// Reused world scan for this think (same center + radius or inner radius).
    scan_cache: Option<NpcScanCache>,
    /// Scan fill time this think (µs), excluding cache hits.
    scan_us_acc: u64,
    scan_calls: u32,
    scan_hits: u32,
    /// Held id we already tried to eat this hunger bout (Haxe refuseFood → drop).
    eat_fail_held: i32,
    /// After `self()` eat, drop leftover if next tick held `foodValue <= 0`.
    // Haxe: AiBase.isEating L8837 dropHeldObject(10)
    eat_pending_peel: bool,
    /// Haxe `isCaringForFire` — cleared in checkIsHungryAndEat (food has priority).
    // Haxe: AiBase.checkIsHungryAndEat L8864
    is_caring_for_fire: bool,
    /// Haxe `useIsDropInContainer` (put clay in basket). CancleUse clears.
    // Haxe: AiBase.useIsDropInContainer L71 / isUsingItem L8910
    use_is_drop_in_container: bool,
    /// Haxe `dropIsAUse` — successful USE is a pile drop, not craft progress.
    // Haxe: AiBase.dropIsAUse L70 / isUsingItem L9061
    drop_is_a_use: bool,
    /// Last tile we thought on. Haxe `movedOneTile` — replan after a tile even if still pathing.
    last_think_xy: Option<(i32, i32)>,
    /// Haxe `TimeHelper.tick` for this think.
    now_tick: u64,
    /// Sim tick when `move()` was sent — that tick `newMoves != null` (isMoving).
    move_sent_tick: u64,
    /// Haxe `escapeTarget` (resetTargets clears).
    // Haxe: AiBase L55 / resetTargets L320
    escape_target: Option<(i32, i32)>,
    /// Haxe `tryMoveNearestTileFirst` (GotoHelper i=0/1 swap).
    // Haxe: AiBase L113
    try_move_nearest_tile_first: bool,
    /// Haxe `AiBase.waitingTime` (STOP / speech wait countdown).
    // Haxe: AiBase L46 / doTimeStuffHelper L514–518
    waiting_time: f32,
    /// Haxe `AiBase.wasIdle` (decays by reactionTime/10).
    // Haxe: AiBase L105 / L424
    was_idle: f32,
    /// Haxe `hasCornSeeds` / `hasCarrotSeeds` from `countSeeds`.
    // Haxe: AiBase.countSeeds L1358–1364
    has_corn_seeds: bool,
    has_carrot_seeds: bool,
    /// Haxe `hasPepperSeeds` / `hasOnionSeeds`.
    // Haxe: AiBase.hasPepperSeeds L1376–1380; hasOnionSeeds L1383–1386
    has_pepper_seeds: bool,
    has_onion_seeds: bool,
    /// Haxe `myPlayer.lastProfession` key for `cleanUpProfessions`.
    last_profession_key: Option<String>,
    /// Haxe `animalTarget` (killAnimal) as `(x, y, id)`.
    // Haxe: AiBase L54 / killAnimal L5878
    animal_target: Option<(i32, i32, i32)>,
    did_not_reach_animal_target: i32,
    /// Haxe `timeLookedForDeadlyAnimalAtHome` (scheduler tick units, −1 = never).
    // Haxe: AiBase L49 / killAnimal L5880
    time_looked_for_deadly_animal_at_home: f32,
    /// Haxe `triedDropCount` for `isDropingItem` dropDistance 10/0.
    // Haxe: AiBase.isDropingItem L8361
    tried_drop_count: i32,
    /// This DROP is only to empty the hand; label it `drop_held_clear`.
    hand_clear_pending: bool,
    /// Thinks spent waiting for 59+124→128 before `isDropingItem` DROP.
    /// Not `triedDropCount` (that is dropDistance; Haxe never aborts pickup on it).
    clothing_wait_apply: i32,
    /// Haxe `lastCheckedTimes['considerFood']` (sim tick).
    // Haxe: AiBase.isConsideringMakingFood L8509
    last_consider_food_tick: f32,
    /// Haxe `timeLastLeaderCheck` for `allyUp` 10s cooldown.
    // Haxe: AiBase.allyUp L8244
    last_leader_check_tick: f32,
}

#[derive(Debug)]
struct NpcScanCache {
    cx: i32,
    cy: i32,
    r: i32,
    tiles: Vec<ScanTile>,
}

impl Default for NpcProfessionState {
    fn default() -> Self {
        Self {
            farm_task: FarmTaskState::default(),
            farm_rt: FarmProfessionRuntime::default(),
            smith_rt: SmithProfessionRuntime::default(),
            baker_rt: BakerProfessionRuntime::default(),
            baker_task: BakerTaskState::default(),
            shepherd_rt: ShepherdProfessionRuntime::default(),
            pottery_rt: PotterProfessionRuntime::default(),
            fire_rt: FireFoodProfessionRuntime::default(),
            fire_keeper_rt: FireKeeperProfessionRuntime::default(),
            grave_keeper_rt: GraveKeeperProfessionRuntime::default(),
            hunter_rt: HunterProfessionRuntime::default(),
            lumberjack_rt: LumberjackProfessionRuntime::default(),
            collector_rt: CollectorProfessionRuntime::default(),
            foodserver_rt: FoodServerProfessionRuntime::default(),
            path_reach: AiPathReachMaps::default(),
            food_goto: NpcFoodGotoState::default(),
            craft_rt: CraftAiRuntime::default(),
            think_time_sec: 0.0,
            prestige_class: PrestigeClass::Commoner,
            class_assigned: false,
            sticky_move: None,
            resume_drop: None,
            was_hungry: false,
            animal_path: None,
            last_is_tailor: false,
            fire_place_id: 0,
            fire_place_x: 0,
            fire_place_y: 0,
            remove_from_container: None,
            handling_temperature: false,
            temp_just_arrived: false,
            last_heat: 0.5,
            rejected_cold_place: None,
            rejected_warm_place: None,
            scan_cache: None,
            scan_us_acc: 0,
            scan_calls: 0,
            scan_hits: 0,
            eat_fail_held: 0,
            eat_pending_peel: false,
            is_caring_for_fire: false,
            use_is_drop_in_container: false,
            drop_is_a_use: false,
            last_think_xy: None,
            now_tick: 0,
            move_sent_tick: 0,
            escape_target: None,
            try_move_nearest_tile_first: true,
            waiting_time: 0.0,
            was_idle: 0.0,
            has_corn_seeds: false,
            has_carrot_seeds: false,
            has_pepper_seeds: false,
            has_onion_seeds: false,
            last_profession_key: None,
            animal_target: None,
            did_not_reach_animal_target: 0,
            time_looked_for_deadly_animal_at_home: TIME_LOOKED_NEVER,
            tried_drop_count: 0,
            hand_clear_pending: false,
            clothing_wait_apply: 0,
            last_consider_food_tick: 0.0,
            last_leader_check_tick: 0.0,
        }
    }
}

/// Haxe `doTimeStuffHelper`: skip only while moving *and* we have not arrived on a new tile.
// Haxe: AiBase.doTimeStuffHelper L428 `if (movedOneTileTmp == false && myPlayer.isMoving()) return;`
fn haxe_skip_mid_path_think(moving: bool, moved_one_tile: bool) -> bool {
    moving && !moved_one_tile
}

fn npc_prestige_class_u8(class: PrestigeClass) -> u8 {
    match class {
        PrestigeClass::Serf => 1,
        PrestigeClass::Commoner | PrestigeClass::NotSet => 2,
        PrestigeClass::Noble | PrestigeClass::King | PrestigeClass::Emperor => 3,
    }
}

fn npc_last_profession_key(st: &NpcProfessionState, p: &PlayerSnapshot) -> Option<&'static str> {
    if let Some(ref k) = st.last_profession_key {
        return match k.as_str() {
            "SMITH" => Some("SMITH"),
            "BAKER" => Some("BAKER"),
            "POTTER" => Some("POTTER"),
            "SHEPHERD" => Some("SHEPHERD"),
            "FIREKEEPER" => Some("FIREKEEPER"),
            "GRAVEKEEPER" => Some("GRAVEKEEPER"),
            "FOODSERVER" => Some("FOODSERVER"),
            "Eating" => Some("Eating"),
            "HUNTER" => Some("HUNTER"),
            "LUMBERJACK" => Some("LUMBERJACK"),
            "COLLECTOR" => Some("COLLECTOR"),
            "TAILOR" => Some("TAILOR"),
            "FIREFOODMAKER" => Some("FIREFOODMAKER"),
            "BowlFiller" => Some("BowlFiller"),
            _ => Some("BASICFARMER"),
        };
    }
    if st.smith_rt.is_last_smith || p.is_last_smith {
        return Some("SMITH");
    }
    if st.baker_rt.is_last_baker || p.is_last_baker {
        return Some("BAKER");
    }
    if st.pottery_rt.is_last_potter || p.is_last_potter {
        return Some("POTTER");
    }
    if st.shepherd_rt.is_last_shepherd || p.is_last_shepherd {
        return Some("SHEPHERD");
    }
    if st.fire_keeper_rt.is_last_fire_keeper {
        return Some("FIREKEEPER");
    }
    if st.grave_keeper_rt.is_last_grave_keeper {
        return Some("GRAVEKEEPER");
    }
    if st.foodserver_rt.is_last_foodserver || p.is_last_foodserver {
        return Some("FOODSERVER");
    }
    if st.hunter_rt.is_last_hunter || p.is_last_hunter {
        return Some("HUNTER");
    }
    if st.lumberjack_rt.is_last_lumberjack || p.is_last_lumberjack {
        return Some("LUMBERJACK");
    }
    if st.collector_rt.is_last_collector || p.is_last_collector {
        return Some("COLLECTOR");
    }
    if st.last_is_tailor || p.is_last_tailor {
        return Some("TAILOR");
    }
    if st.fire_rt.is_last_fire_food || p.is_last_fire_food {
        return Some("FIREFOODMAKER");
    }
    if st.farm_rt.last_profession.is_some() || p.is_last_farm {
        return Some("BASICFARMER");
    }
    None
}

/// Haxe `cleanUpProfessions` — zero non-last profession stage weights.
// Haxe: AiBase.cleanUpProfessions L4443–4461
fn npc_clean_up_professions(st: &mut NpcProfessionState, p: &PlayerSnapshot) {
    let last = npc_last_profession_key(st, p);
    if last.is_none() {
        return;
    }
    if should_zero_profession_weight("POTTER", last) {
        st.pottery_rt.stage = 0.0;
    }
    if should_zero_profession_weight("SMITH", last) {
        // smith stage is in smith_rt; leave last flags
    }
}

/// How `isDropingItem` should treat a clothing-pickup sticky after 59+124→128.
///
/// Haxe `dropTarget` is a live ObjectHelper (`isStillExpectedItem` L589 compares
/// stored parentId to world at tx,ty). Rust snapshots expected=124 so the think
/// before sim-apply stays valid; once the tile is clothing, use the live parent.
/// Never DROP on the pre-product tile (that would pick up Reed Bundle 124).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClothingDropLive {
    WaitForProduct,
    Parent(i32),
}

fn npc_clothing_drop_live_parent(
    content: &ContentDb,
    expected: i32,
    world_p: i32,
    pickup_cloth: bool,
) -> ClothingDropLive {
    if npc_clothing_slot(content, world_p).is_some() {
        return ClothingDropLive::Parent(world_p);
    }
    // Wait only while the USE tile is still the pre-product (Reed Bundle 124).
    // Empty (0) or a different object is TargetGone — not a wait.
    if pickup_cloth && expected > 0 && world_p == expected {
        return ClothingDropLive::WaitForProduct;
    }
    ClothingDropLive::Parent(expected)
}

/// Haxe `triedDropCount` never aborts `isDropingItem`; it only shortens dropDistance.
/// Rust waits for async 59+124→128. Give up after this many thinks so a failed USE
/// does not pin the NPC. Independent of `tried_drop_count` (prior craft drops).
// Haxe: AiBase.isDropingItem L8361
const CLOTHING_WAIT_APPLY_MAX: i32 = 20;

fn npc_clothing_wait_apply_give_up(wait_ticks: i32) -> bool {
    wait_ticks > CLOTHING_WAIT_APPLY_MAX
}

/// After a USE whose newTarget is clothing (Reed Skirt 128 = Rope 59 + Reed Bundle 124),
/// stage Haxe `dropTarget` / pile `useTarget` so the next think's `isDropingItem`
/// (before food) picks it up, then `switchCloths` `self()` wears it.
// Haxe: AiBase.craftClothIfNeeded L4709 places the item; isPickingupCloths L8723–8748;
// isDropingItem L497; switchCloths L8709
fn npc_stage_clothing_pickup_if_use_makes_cloth(
    content: &ContentDb,
    st: &mut NpcProfessionState,
    actor_id: i32,
    target_id: i32,
    x: i32,
    y: i32,
) {
    let Some(tr) = content
        .find_transition(actor_id, target_id)
        .or_else(|| content.find_transition_last_use(actor_id, target_id))
    else {
        // Already standing on clothing (GetOrCraft / isPickingupCloths re-enter).
        let base = content.resolve_base_id(target_id);
        if npc_clothing_slot(content, base).is_some() {
            npc_stage_clothing_drop_target(content, st, x, y, target_id, base);
        }
        return;
    };
    // Ground product (newTarget). newActor clothing is held → switchCloths next think.
    let pid = tr.new_target_id;
    if pid <= 0 {
        return;
    }
    let base = content.resolve_base_id(pid);
    if npc_clothing_slot(content, base).is_none() {
        return;
    }
    // Haxe dropTarget is the same tile helper; USE 59+124 then id becomes 128.
    // Expect the *current* tile (124) so the next think before sim-apply stays
    // valid; after apply, sticky_move_still_valid keeps clothing on that tile.
    let tile_expect = content.resolve_base_id(if target_id > 0 { target_id } else { base });
    npc_stage_clothing_drop_target(content, st, x, y, tile_expect, base);
}

fn npc_stage_clothing_drop_target(
    content: &ContentDb,
    st: &mut NpcProfessionState,
    x: i32,
    y: i32,
    tile_expect: i32,
    cloth_base: i32,
) {
    let permanent = content
        .get(content.resolve_base_id(cloth_base))
        .map(|d| d.permanent)
        .unwrap_or(false);
    let arrive = if permanent {
        StickyArrive::Use
    } else {
        StickyArrive::Drop
    };
    set_sticky_arrive(
        st,
        x,
        y,
        tile_expect,
        0,
        arrive,
        format!("pickup_cloth {cloth_base}"),
    );
}

/// Haxe GetOrCraftItem L6215 `dropTarget = obj` when the tile is clothing.
fn npc_drop_arrive_label(
    content: &ContentDb,
    expected: i32,
    x: i32,
    y: i32,
    is_drop: bool,
) -> String {
    if is_drop && npc_clothing_slot(content, expected).is_some() {
        format!("pickup_cloth {expected}")
    } else {
        format!("craft @{x},{y}")
    }
}

/// Haxe `GetCloseClothings` r=8 around the player (half-open).
// Haxe: AiHelper.GetCloseClothings L541; isPickingupCloths L8726
fn npc_clothing_pickup_in_range(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    in_get_close_clothings_square(px, py, tx, ty, GET_CLOSE_CLOTHINGS_RADIUS)
}

fn npc_clothing_slot(content: &ContentDb, id: i32) -> Option<i32> {
    let def = content.get(id)?;
    let c = def
        .clothing
        .trim()
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase())
        .unwrap_or('n');
    match c {
        'h' => Some(0),
        't' => Some(1),
        's' => Some(2),
        'b' => Some(4),
        'p' => Some(5),
        _ => None,
    }
}

fn npc_should_switch_held_or_obj(
    content: &ContentDb,
    obj_id: i32,
    clothing: [i32; 6],
    prestige_class: u8,
) -> bool {
    let resolved = switch_cloth_resolve_parent(content.resolve_base_id(obj_id));
    let Some(def) = content.get(resolved) else {
        return false;
    };
    let slot = npc_clothing_slot(content, resolved);
    let worn_id = slot
        .and_then(|s| clothing.get(s as usize).copied())
        .unwrap_or(0);
    let shoe_other = clothing.get(3).copied().unwrap_or(0);
    let worn_def = if worn_id != 0 {
        content.get(content.resolve_base_id(worn_id))
    } else {
        None
    };
    should_switch_cloth(
        resolved,
        def.extra_prestige_factor,
        def.prestige_factor,
        &def.name,
        slot,
        worn_id,
        shoe_other,
        worn_def.map(|d| d.extra_prestige_factor).unwrap_or(0.0),
        worn_def.map(|d| d.prestige_factor).unwrap_or(0.0),
        worn_def.map(|d| d.name.as_str()).unwrap_or(""),
        prestige_class,
    )
}

/// Haxe `switchCloths` — SELF when held clothing should replace worn.
// Haxe: AiBase.switchCloths L8709–8720
fn npc_run_switch_cloths(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    prestige_class: u8,
) -> Option<(NpcActivityKind, String, u32)> {
    if p.age < MIN_AGE_TO_EAT {
        return None;
    }
    if p.held_id == 0 {
        return None;
    }
    if !npc_should_switch_held_or_obj(content, p.held_id, p.clothing, prestige_class) {
        return None;
    }
    let payload = self_clothing_raw_payload(-1);
    if intent_tx
        .try_send(NetIntent::Raw {
            conn_id,
            tag: "SELF".into(),
            payload,
        })
        .is_ok()
    {
        Some((NpcActivityKind::Craft, format!("switch_cloths held={}", p.held_id), 400))
    } else {
        None
    }
}

fn npc_is_switchable_ground_cloth(
    content: &ContentDb,
    obj_id: i32,
    clothing: [i32; 6],
    prestige_class: u8,
) -> bool {
    if obj_id <= 0 {
        return false;
    }
    let base = content.resolve_base_id(obj_id);
    let def = content.get(base);
    let clothing_field = def.map(|d| d.clothing.as_str()).unwrap_or("n");
    if !is_get_close_clothing_object(clothing_field, obj_id)
        && !is_get_close_clothing_object(clothing_field, base)
    {
        return false;
    }
    npc_should_switch_held_or_obj(content, obj_id, clothing, prestige_class)
}

fn npc_stage_cloth_pickup_at(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    x: i32,
    y: i32,
    parent_id: i32,
) -> Option<(NpcActivityKind, String, u32)> {
    if st.path_reach.blocks_target(x, y, None) {
        return None;
    }
    let base = content.resolve_base_id(parent_id);
    let permanent = content.get(base).map(|d| d.permanent).unwrap_or(false);
    // Haxe: pile → useTarget (isUsingItem next); loose → dropTarget (isDropingItem next).
    let arrive = if permanent {
        StickyArrive::Use
    } else {
        StickyArrive::Drop
    };
    let label = if permanent {
        format!("pickup_cloth_pile {parent_id}")
    } else {
        format!("pickup_cloth {parent_id}")
    };
    if npc_do_arrive_or_walk(
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        x,
        y,
        parent_id,
        0,
        arrive,
        label.clone(),
        npc_client_move_seq(p.done_moving_seq),
    ) {
        return Some((NpcActivityKind::Craft, label, 250));
    }
    // Receding gotoObj already marked the tile hostile and cleared the sticky.
    if st.path_reach.blocks_target(x, y, None) {
        return None;
    }
    set_sticky_arrive(st, x, y, parent_id, 0, arrive, label.clone());
    Some((NpcActivityKind::Craft, label, 100))
}

/// Closest `id` within wrap-aware Chebyshev `max_r` (Haxe GetOrCraftItem
/// `maxSearchDistance`, default 40, clothing pass 60).
// Haxe: AiBase.GetOrCraftItem L6150–6198
fn npc_existing_object_xy_within(
    world: &World,
    px: i32,
    py: i32,
    id: i32,
    max_r: i32,
) -> Option<(i32, i32)> {
    if id <= 0 || max_r < 0 {
        return None;
    }
    let (x, y) = world.find_closest_object_id(id, px, py)?;
    if world.chebyshev(px, py, x, y) <= max_r {
        Some((x, y))
    } else {
        None
    }
}

/// True when a walk can reach `(gx,gy)` without crossing a blocking biome.
///
/// Haxe `isBlocked` / `gotoObj`: snow-grey stops the step. A rope whose
/// Chebyshev distance is inside 60 but whose walk hits that band is not a
/// drop target — the NPC would stand on the near edge and retry the MOVE.
fn npc_clothing_goal_reachable(
    world: &World,
    content: &ContentDb,
    sx: i32,
    sy: i32,
    gx: i32,
    gy: i32,
) -> bool {
    if npc_is_close_action(sx, sy, gx, gy) {
        return true;
    }
    // Haxe gotoObj: walk while a step is not isBlocked and gets closer.
    // A flood that gave up after 250 side steps was skipping ropes and
    // yew shafts that are a normal walk away, so 59+124 and 59+131 never ran.
    let start_d = world.chebyshev(sx, sy, gx, gy);
    if start_d > 80 {
        return false;
    }
    for (dx, dy) in [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ] {
        let nx = sx + dx;
        let ny = sy + dy;
        if npc_is_blocked(world, content, 0, nx, ny) {
            continue;
        }
        if world.chebyshev(nx, ny, gx, gy) < start_d {
            return true;
        }
    }
    false
}

fn npc_craft_intent_goal(intent: &ShortCraftLiveIntent) -> Option<(i32, i32)> {
    match *intent {
        ShortCraftLiveIntent::UseAt { x, y, .. }
        | ShortCraftLiveIntent::DropAt { x, y }
        | ShortCraftLiveIntent::Goto { x, y }
        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. }
        | ShortCraftLiveIntent::PickupNearForge { x, y, .. } => Some((x, y)),
        _ => None,
    }
}

/// Closest `id` inside `max_r` of the player or home that a walk can reach.
fn npc_approachable_cloth_xy(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    id: i32,
    max_r: i32,
    blocked: &HashSet<(i32, i32)>,
) -> Option<(i32, i32)> {
    if id <= 0 || max_r < 0 {
        return None;
    }
    let min_x = px.min(home_x) - max_r;
    let max_x = px.max(home_x) + max_r;
    let min_y = py.min(home_y) - max_r;
    let max_y = py.max(home_y) + max_r;
    let mut cands = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if world.get_object(x, y) != id {
                continue;
            }
            // Haxe isObjectNotReachable after isDropingItem goto fail (L8442–8446).
            if blocked.contains(&(x, y)) {
                continue;
            }
            let dp = world.chebyshev(px, py, x, y);
            let dh = world.chebyshev(home_x, home_y, x, y);
            if dp > max_r && dh > max_r {
                continue;
            }
            cands.push((dp, x, y));
        }
    }
    cands.sort_by_key(|c| c.0);
    cands.into_iter().find_map(|(_d, x, y)| {
        if npc_clothing_goal_reachable(world, content, px, py, x, y) {
            Some((x, y))
        } else {
            None
        }
    })
}

/// Rope 59 + Yew Shaft 131 inside `max_r`. Held rope USEs the shaft;
/// otherwise DROP-pick the rope. Haxe `craftItem(152)` when `59_131` is ready
/// (newTarget Yew Bow 151), before seeking Arrow 148.
// Haxe: AiBase.craftItemHelper; getWeapon GetOrCraftItem(152) L5815
fn npc_yew_bow_direct(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    held_base: i32,
    max_r: i32,
    blocked: &HashSet<(i32, i32)>,
) -> Option<ShortCraftLiveIntent> {
    // Held rope or shaft is one half. Do not also require that half on the ground.
    if held_base == 59 {
        let shaft =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 131, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::UseAt {
            x: shaft.0,
            y: shaft.1,
            target_id: 131,
            actor_id: 59,
        });
    }
    if held_base == 131 {
        let rope =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 59, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::UseAt {
            x: rope.0,
            y: rope.1,
            target_id: 59,
            actor_id: 131,
        });
    }
    // Holding Yew Bow 151 or Arrow 148: combine those, never drop them for rope.
    if held_base == 151 {
        let arrow =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 148, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::UseAt {
            x: arrow.0,
            y: arrow.1,
            target_id: 148,
            actor_id: 151,
        });
    }
    if held_base == 148 {
        let bow =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 151, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::UseAt {
            x: bow.0,
            y: bow.1,
            target_id: 151,
            actor_id: 148,
        });
    }
    // Haxe GetOrCraftItem(152) picks up a finished bow-and-arrow. A Yew Bow
    // already on the ground is the actor of 148+151, so pick that up before
    // making another bow from rope and shaft. The next think, holding 151,
    // seeks the missing arrow instead of 59+131 again.
    if let Some(done) =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 152, max_r, blocked)
    {
        return Some(ShortCraftLiveIntent::DropAt {
            x: done.0,
            y: done.1,
        });
    }
    if let Some(bow) =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 151, max_r, blocked)
    {
        return Some(ShortCraftLiveIntent::DropAt {
            x: bow.0,
            y: bow.1,
        });
    }
    let rope =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 59, max_r, blocked)?;
    let _shaft =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 131, max_r, blocked)?;
    Some(ShortCraftLiveIntent::DropAt {
        x: rope.0,
        y: rope.1,
    })
}

/// Rope 59 + Reed Bundle 124 inside `max_r` of the player or home.
/// Held rope USEs a walkable bundle. It does not need a second rope on the ground.
/// Otherwise DROP-pick a walkable rope, and only when a walkable bundle exists too.
/// Both tiles must be walkable (Haxe `gotoObj` / `isBlocked`).
fn npc_reed_skirt_direct(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    held_base: i32,
    max_r: i32,
    blocked: &HashSet<(i32, i32)>,
) -> Option<ShortCraftLiveIntent> {
    // Live 0.3.29: held 59 made this None (no ground rope). The generic fallback
    // then DROP-placed 59 on the feet tile and the next think picked it up.
    if held_base == 59 {
        let reed =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 124, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::UseAt {
            x: reed.0,
            y: reed.1,
            target_id: 124,
            actor_id: 59,
        });
    }
    // The only bundle is in hand. DROP onto the rope swaps (rope in hand,
    // bundle on that tile). Requiring a second ground 124 returned None and
    // the held bundle was empty-dropped. Transition is 59+124, not 124+59.
    if held_base == 124 {
        let rope =
            npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 59, max_r, blocked)?;
        return Some(ShortCraftLiveIntent::DropAt {
            x: rope.0,
            y: rope.1,
        });
    }
    let rope =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 59, max_r, blocked)?;
    let _reed =
        npc_approachable_cloth_xy(world, content, px, py, home_x, home_y, 124, max_r, blocked)?;
    Some(ShortCraftLiveIntent::DropAt {
        x: rope.0,
        y: rope.1,
    })
}

/// Existing clothing tile to DROP-pick (Haxe GetOrCraftItem L6196 dropTarget=obj).
fn npc_existing_switch_cloth_xy(
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    clothing: [i32; 6],
    prestige_class: u8,
) -> Option<(i32, i32, i32)> {
    world.find_closest_object_pred(px, py, |oid| {
        npc_is_switchable_ground_cloth(content, oid, clothing, prestige_class)
    })
}

/// Haxe `isPickingupCloths` — drop/use nearby better clothing (r=8 + home r=60).
// Haxe: AiBase.isPickingupCloths L8723–8752; AiHelper.GetCloseClothings L541
fn npc_run_is_pickingup_cloths(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    prestige_class: u8,
) -> Option<(NpcActivityKind, String, u32)> {
    if p.age < MIN_AGE_TO_EAT {
        return None;
    }
    // Haxe GetCloseClothings default r=8 half-open around the player.
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, GET_CLOSE_CLOTHINGS_RADIUS);
    for t in &tiles {
        if t.parent_id == 0 {
            continue;
        }
        if !npc_clothing_pickup_in_range(p.x, p.y, t.x, t.y) {
            continue;
        }
        if !npc_is_switchable_ground_cloth(content, t.parent_id, p.clothing, prestige_class) {
            continue;
        }
        if let Some(hit) = npc_stage_cloth_pickup_at(
            intent_tx,
            world,
            content,
            st,
            conn_id,
            p,
            t.x,
            t.y,
            t.parent_id,
        ) {
            return Some(hit);
        }
    }
    // Haxe isPickingupCloths L8723 only GetCloseClothings r=8. Do not
    // world-search: live 0.3.21 sent adults to orphaned 128 at 228,275
    // (cheb 100+) every L652 tick, so craftHighPriorityClothing L684 never
    // recrafted at home and worn stayed 0.
    None
}

/// Haxe `countProfession('HUNTER')` excluding self (same-home, age/wound/food/follow).
// Haxe: AiBase.countProfession L1288–1308
fn npc_count_hunter_peers(
    views: &HashMap<u64, PlayerSnapshot>,
    conn_id: u64,
    home_x: i32,
    home_y: i32,
    content: &ContentDb,
) -> f32 {
    views
        .values()
        .filter(|o| {
            if o.conn_id == conn_id || o.deleted || !o.is_last_hunter {
                return false;
            }
            if o.home_x != home_x || o.home_y != home_y {
                return false;
            }
            if o.age < MIN_AGE_TO_EAT || o.age > MAX_AGE - 2.0 {
                return false;
            }
            if o.food < 0.0 {
                return false;
            }
            if o.ai_follow_p_id > 0 {
                return false;
            }
            if npc_is_wounded(content, o.held_id) && !o.is_hidden_wound {
                return false;
            }
            true
        })
        .count() as f32
}

/// Haxe `killAnimal` — wolf-at-home prefix, snake knife shortCraft, else bow hunt.
// Haxe: AiBase.killAnimal L5878–5964
/// Haxe `GetOrCraftItem(id)` from `getWeapon` — expand then walk/USE.
// Haxe: AiBase.getWeapon L5814–5815
fn npc_emit_seek_or_craft(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    actor: i32,
    kind_if_ok: NpcActivityKind,
    label: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    // Haxe craftItem(152): when rope and yew shaft are both inside the 60
    // search, USE 59+131 (Yew Bow 151) instead of walking a deeper leaf.
    // Skip tiles isDropingItem already marked not reachable (L8442–8446).
    let blocked = st.path_reach.blocked_coords(None);
    if actor == BOW_AND_ARROW {
        let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
        if let Some(raw) = npc_yew_bow_direct(
            world,
            content,
            p.x,
            p.y,
            hx,
            hy,
            content.resolve_base_id(p.held_id),
            NPC_GET_OR_CRAFT_SEARCH_RADIUS,
            &blocked,
        ) {
            // Haxe L7114: drop held food before dropTarget = rope, else the
            // berry stays in hand and getWeapon never reaches 59+131.
            let rewritten = npc_drop_held_before_loose_pickup(
                world,
                content,
                p.x,
                p.y,
                hx,
                hy,
                p.held_id,
                raw,
            );
            let intent = npc_park_actor_behind_food_drop(st, content, world, raw, rewritten);
            let mut kind = kind_if_ok;
            let mut detail = format!("{label} {actor}");
            let mut game_ms = 250;
            if npc_commit_craft_live(
                &intent,
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                p.food,
                p.moving,
                &mut kind,
                &mut detail,
                &mut game_ms,
                npc_client_move_seq(p.done_moving_seq),
            ) {
                if let Some(s) = st.sticky_move.as_mut() {
                    if s.pending_use && !s.label.starts_with("yew_bow_before_baby") {
                        s.label = format!("yew_bow_before_baby {}", s.label);
                    }
                }
                if !detail.starts_with(label) {
                    detail = format!("{label} {detail}");
                }
                return Some((kind, detail, game_ms));
            }
        }
    }
    let tiles = npc_scan_for_craft(
        st,
        world,
        content,
        p.x,
        p.y,
        p.home_x,
        p.home_y,
        NPC_GET_OR_CRAFT_SEARCH_RADIUS,
    );
    let staging = ShortCraftLiveIntent::SeekOrCraft {
        actor,
        craft_if_needed: true,
    };
    let mut resolved = npc_expand_craft_intent(
        world,
        &tiles,
        p.x,
        p.y,
        p.held_id,
        p.moving,
        p.home_x,
        p.home_y,
        content,
        craft_graph,
        &mut st.craft_rt,
        &blocked,
        false,
        tick,
        staging,
    );
    // Haxe GetOrCraftItem miss → craftItem(objId) with maxSearchRadius 60.
    // Haxe: AiBase.GetOrCraftItem L6198
    if !p.moving && !npc_craft_expand_progress(&resolved, false) {
        resolved = npc_expand_craft_intent(
            world,
            &tiles,
            p.x,
            p.y,
            p.held_id,
            p.moving,
            p.home_x,
            p.home_y,
            content,
            craft_graph,
            &mut st.craft_rt,
            &blocked,
            false,
            tick,
            ShortCraftLiveIntent::CraftItem { object_id: actor },
        );
    }
    let mut kind = kind_if_ok;
    let mut detail = format!("{label} {actor}");
    let mut game_ms = 250;
    if npc_commit_craft_live(
        &resolved,
        intent_tx,
        world,
        content,
        st,
        conn_id,
        p.x,
        p.y,
        p.food,
        p.moving,
        &mut kind,
        &mut detail,
        &mut game_ms,
        npc_client_move_seq(p.done_moving_seq),
    ) {
        // Keep GetOrCraft label as first token so objects_created sees seek_weapon.
        if !detail.starts_with(label) {
            detail = format!("{label} {detail}");
        }
        Some((kind, detail, game_ms))
    } else {
        None
    }
}

fn npc_run_kill_animal(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    st: &mut NpcProfessionState,
    conn_id: u64,
    p: &PlayerSnapshot,
    tick: u64,
    deadly: Option<(i32, i32, i32)>,
    hunter_peer_count: f32,
    animals: Option<&AnimalWorld>,
) -> Option<(NpcActivityKind, String, u32)> {
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        p.x
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        p.y
    };
    let home_scan = npc_scan_cached(st, world, content, home_x, home_y, KILL_ANIMAL_WOLF_SEARCH);
    let mut wolf_tiles: Vec<(i32, i32, i32)> = home_scan
        .iter()
        .filter(|t| t.parent_id == WOLF)
        .filter(|t| wolf_tile_allowed(t.floor_id, t.is_food, t.is_permanent))
        .map(|t| (t.parent_id, t.x, t.y))
        .collect();
    // Haxe wolves are map objects; live movers are AnimalWorld. Same home r=20 square.
    if let Some(aw) = animals {
        for a in &aw.animals {
            if a.kind != AnimalKind::Wolf {
                continue;
            }
            if wolf_in_home_search(home_x, home_y, a.x, a.y) {
                wolf_tiles.push((WOLF, a.x, a.y));
            }
        }
    }
    let prefix_target = st.animal_target.map(|(x, y, id)| (id, x, y));
    let prefix_animal = deadly.map(|(x, y, id)| (id, x, y));
    let prefix_inp = KillAnimalPrefixInput {
        animal: prefix_animal,
        animal_target: prefix_target,
        time_looked_tick: st.time_looked_for_deadly_animal_at_home,
        now_tick: st.now_tick as f32,
        tick_time: TIME_HELPER_TICK_TIME,
        clothing_ids: &p.clothing,
        home_tiles: &wolf_tiles,
        home_x,
        home_y,
        hunter_peer_count,
        was_idle: st.was_idle,
    };
    let prefix = kill_animal_prefix(&prefix_inp, &mut st.hunter_rt);
    st.time_looked_for_deadly_animal_at_home = prefix.time_looked_tick;
    st.animal_target = prefix.animal_target.map(|(id, x, y)| (x, y, id));
    if st.hunter_rt.is_last_hunter {
        st.last_profession_key = Some("HUNTER".into());
    }
    if prefix.kind == KillAnimalPrefixKind::Stop {
        return None;
    }
    // Haxe L5902–5964
    let tiles = npc_scan_cached(st, world, content, p.x, p.y, WEAPON_SEARCH_DIST);
    let nearby: Vec<(i32, i32, i32, bool)> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| (t.parent_id, t.x, t.y, t.is_permanent))
        .collect();
    let held_parent = content
        .dummy_parent
        .get(&p.held_id)
        .copied()
        .unwrap_or(p.held_id);
    let bow_parent = content
        .dummy_parent
        .get(&BOW_AND_ARROW)
        .copied()
        .unwrap_or(BOW_AND_ARROW);
    // Haxe objects/152.txt minPickupAge=10. A cache value of 0 let infants
    // run GetOrCraftItem(152) (kill_animal_seek_weapon at age < 1).
    let bow_min = content
        .get(BOW_AND_ARROW)
        .map(|d| d.min_pickup_age as f32)
        .filter(|age| *age >= 1.0)
        .unwrap_or(10.0);
    let bow_use = content
        .get(BOW_AND_ARROW)
        .map(|d| d.use_distance as f32)
        .unwrap_or(0.0);
    let animal = deadly.map(|(x, y, id)| (id, x, y)).or_else(|| {
        closest_parent_in_tiles(&tiles, RATTLE_SNAKE, p.x, p.y, DEADLY_ANIMAL_SEARCH_DIST)
            .map(|(x, y)| (RATTLE_SNAKE, x, y))
    });
    let animal_target = st.animal_target.map(|(x, y, id)| (id, x, y));
    let killable = |id: i32| content.find_transition(BOW_AND_ARROW, id).is_some();
    let weapon = AttackPlayerInput {
        target: None,
        food_store: p.food,
        self_wounded: npc_is_wounded(content, p.held_id) && !p.is_hidden_wound,
        age: p.age,
        min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
        holding_weapon: npc_holding_weapon(content, p.held_id),
        held_id: p.held_id,
        held_parent_id: held_parent,
        is_moving: p.moving,
        player_x: p.x,
        player_y: p.y,
        exact_x: p.x as f64,
        exact_y: p.y as f64,
        home_x: p.home_x,
        home_y: p.home_y,
        deadly_distance: 0.0,
        clothing: AttackPlayerClothing::from_ids(&p.clothing),
        weapon_tiles: &nearby,
    };
    let body_inp = KillAnimalBodyInput {
        food_store: p.food,
        age: p.age,
        bow_min_pickup_age: bow_min,
        animal,
        animal_target,
        animal_killable_by_bow: animal.map(|(id, _, _)| killable(id)).unwrap_or(false),
        target_killable_by_bow: animal_target
            .map(|(id, _, _)| killable(id))
            .unwrap_or(false),
        player_x: p.x,
        player_y: p.y,
        held_parent_id: held_parent,
        bow_parent_id: bow_parent,
        bow_use_distance: bow_use,
        weapon,
    };
    let mut result = kill_animal_body(&body_inp);
    if matches!(result.action, KillAnimalAction::ShortCraftSnake) {
        if let Some((x, y)) =
            closest_parent_in_tiles(&tiles, RATTLE_SNAKE, p.x, p.y, KILL_ANIMAL_SNAKE_RADIUS)
        {
            if p.held_id == KNIFE_ID {
                if npc_is_close_action(p.x, p.y, x, y) {
                    if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                        return Some((NpcActivityKind::Combat, "kill_snake_knife".into(), 400));
                    }
                } else if npc_try_walk_to_sticky(
                    intent_tx,
                    world,
                    content,
                    st,
                    conn_id,
                    p.x,
                    p.y,
                    x,
                    y,
                    p.food,
                    RATTLE_SNAKE,
                    true,
                    p.moving,
                    "kill_snake_walk",
                    npc_client_move_seq(p.done_moving_seq),
                ) {
                    return Some((NpcActivityKind::Combat, "kill_snake_walk".into(), 250));
                }
            } else if let Some(out) = npc_emit_seek_or_craft(
                intent_tx,
                world,
                content,
                craft_graph,
                st,
                conn_id,
                p,
                tick,
                KNIFE_ID,
                NpcActivityKind::Combat,
                "kill_snake_get_knife",
            ) {
                // Haxe shortCraft(560, 764, 10) → GetOrCraftItem(560)
                return Some(out);
            }
        }
        // Haxe shortCraft false → continue bow hunt (wolf / getWeapon 152).
        result = kill_animal_bow_hunt(&body_inp);
    }
    st.animal_target = result.animal_target.map(|(id, x, y)| (x, y, id));
    match result.action {
        KillAnimalAction::None | KillAnimalAction::ShortCraftSnake => None,
        KillAnimalAction::GetWeapon(gw) => match gw {
            GetWeaponAction::None => None,
            GetWeaponAction::Wait => {
                // Haxe GetOrCraftItem L6152: isMoving → return true.
                // killAnimal then returns true (L593) so later bands do not run.
                Some((NpcActivityKind::Combat, "kill_animal_wait_weapon".into(), 200))
            }
            GetWeaponAction::GoHome { x, y } => {
                if npc_try_walk_to_sticky(
                    intent_tx,
                    world,
                    content,
                    st,
                    conn_id,
                    p.x,
                    p.y,
                    x,
                    y,
                    p.food,
                    0,
                    false,
                    p.moving,
                    "kill_animal_bloody_home",
                    npc_client_move_seq(p.done_moving_seq),
                ) {
                    Some((NpcActivityKind::Combat, "kill_animal_bloody_home".into(), 250))
                } else {
                    None
                }
            }
            GetWeaponAction::SelfClothing { slot } => {
                let payload = self_clothing_raw_payload(slot);
                if intent_tx
                    .try_send(NetIntent::Raw {
                        conn_id,
                        tag: "SELF".into(),
                        payload,
                    })
                    .is_ok()
                {
                    Some((NpcActivityKind::Combat, "kill_animal_quiver".into(), 400))
                } else {
                    None
                }
            }
            GetWeaponAction::DropHeld => {
                if npc_drop_at(intent_tx, conn_id, p.x, p.y, None) {
                    Some((NpcActivityKind::Combat, "kill_animal_drop".into(), 400))
                } else {
                    None
                }
            }
            GetWeaponAction::Pickup { x, y, id } => {
                if npc_is_close_action(p.x, p.y, x, y) {
                    if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                        Some((
                            NpcActivityKind::Combat,
                            format!("kill_animal_pickup {id}"),
                            400,
                        ))
                    } else {
                        None
                    }
                } else if npc_try_walk_to_sticky(
                    intent_tx,
                    world,
                    content,
                    st,
                    conn_id,
                    p.x,
                    p.y,
                    x,
                    y,
                    p.food,
                    id,
                    true,
                    p.moving,
                    "kill_animal_walk_weapon",
                    npc_client_move_seq(p.done_moving_seq),
                ) {
                    Some((NpcActivityKind::Combat, "kill_animal_walk_weapon".into(), 250))
                } else {
                    None
                }
            }
            GetWeaponAction::SeekOrCraft { actor } => {
                // Haxe getWeapon: GetOrCraftItem(148) / GetOrCraftItem(152) — not a log-only busy.
                // Haxe: AiBase.getWeapon L5814–5815
                let out = npc_emit_seek_or_craft(
                    intent_tx,
                    world,
                    content,
                    craft_graph,
                    st,
                    conn_id,
                    p,
                    tick,
                    actor,
                    NpcActivityKind::Combat,
                    "kill_animal_seek_weapon",
                );
                if out.is_none() {
                    tracing::info!(
                        conn_id,
                        actor,
                        held = p.held_id,
                        "ai_craft: getWeapon SeekOrCraft miss"
                    );
                }
                out
            }
        },
        KillAnimalAction::Goto { x, y } => {
            if npc_try_walk_to_sticky(
                intent_tx,
                world,
                content,
                st,
                conn_id,
                p.x,
                p.y,
                x,
                y,
                p.food,
                0,
                false,
                p.moving,
                "kill_animal_range",
                npc_client_move_seq(p.done_moving_seq),
            ) {
                st.did_not_reach_animal_target = 0;
                Some((NpcActivityKind::Combat, "kill_animal_range".into(), 250))
            } else {
                st.did_not_reach_animal_target += 1;
                if st.did_not_reach_animal_target >= KILL_ANIMAL_GOTO_FAIL_CLEAR {
                    st.animal_target = None;
                }
                Some((NpcActivityKind::Combat, "kill_animal_range_fail".into(), 200))
            }
        }
        KillAnimalAction::Use { x, y } => {
            let _ = npc_use_at(intent_tx, conn_id, x, y, None, None);
            st.food_goto.did_not_reach_food = 0.0;
            Some((NpcActivityKind::Combat, "kill_animal_use".into(), 400))
        }
    }
}

fn closest_parent_in_tiles(
    tiles: &[ScanTile],
    parent: i32,
    px: i32,
    py: i32,
    max_r: i32,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, i32)> = None;
    for t in tiles {
        if t.parent_id != parent {
            continue;
        }
        let d = (t.x - px).abs().max((t.y - py).abs());
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, t.x, t.y)),
            Some((bd, _, _)) if d < bd => best = Some((d, t.x, t.y)),
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

fn npc_scan_try_cache(
    st: &mut NpcProfessionState,
    cx: i32,
    cy: i32,
    r: i32,
) -> Option<Vec<ScanTile>> {
    let c = st.scan_cache.as_ref()?;
    if c.cx != cx || c.cy != cy {
        return None;
    }
    if c.r == r {
        st.scan_hits = st.scan_hits.saturating_add(1);
        return Some(c.tiles.clone());
    }
    if c.r > r {
        st.scan_hits = st.scan_hits.saturating_add(1);
        return Some(filter_scan_tiles_in_radius(&c.tiles, cx, cy, r));
    }
    None
}

fn npc_scan_fill(
    st: &mut NpcProfessionState,
    world: &World,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    r: i32,
) -> Vec<ScanTile> {
    let t0 = Instant::now();
    let tiles = scan_world_radius(world, Some(content), cx, cy, r);
    st.scan_us_acc = st.scan_us_acc.saturating_add(t0.elapsed().as_micros() as u64);
    st.scan_cache = Some(NpcScanCache {
        cx,
        cy,
        r,
        tiles: tiles.clone(),
    });
    tiles
}

fn npc_scan_cached(
    st: &mut NpcProfessionState,
    world: &World,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    r: i32,
) -> Vec<ScanTile> {
    st.scan_calls = st.scan_calls.saturating_add(1);
    if let Some(hit) = npc_scan_try_cache(st, cx, cy, r) {
        return hit;
    }
    npc_scan_fill(st, world, content, cx, cy, r)
}

fn npc_merge_scan_tiles(mut a: Vec<ScanTile>, b: Vec<ScanTile>) -> Vec<ScanTile> {
    let mut seen: HashSet<(i32, i32)> = a.iter().map(|t| (t.x, t.y)).collect();
    for t in b {
        if seen.insert((t.x, t.y)) {
            a.push(t);
        }
    }
    a
}

/// True only when the player Chebyshev square *is* the home square.
/// Covering the home *point* is not enough: Haxe `addAllObjectsForCrafting`
/// L7240 scans a full `radius` around home, so ingredients just outside the
/// player square still count (live 0.3.16: Thread 58 at 444,18 with player
/// 465,64 home 477,29 r=40).
// Haxe: AiBase.addAllObjectsForCrafting L7240
fn npc_player_scan_covers_home_square(px: i32, py: i32, home_x: i32, home_y: i32) -> bool {
    home_x == px && home_y == py
}

/// Haxe `addAllObjectsForCrafting` always scans home; player scan is extra.
/// Merge a full home square unless the player is standing on home.
// Haxe: AiBase.addAllObjectsForCrafting L7240
fn npc_scan_for_craft(
    st: &mut NpcProfessionState,
    world: &World,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    r: i32,
) -> Vec<ScanTile> {
    let player = npc_scan_cached(st, world, content, px, py, r);
    if npc_player_scan_covers_home_square(px, py, home_x, home_y) {
        return player;
    }
    let home = scan_world_radius(world, Some(content), home_x, home_y, r);
    npc_merge_scan_tiles(player, home)
}

fn npc_scan_for_craft_rw(
    st: &mut NpcProfessionState,
    world: &RwLock<World>,
    content: &ContentDb,
    px: i32,
    py: i32,
    home_x: i32,
    home_y: i32,
    r: i32,
) -> Vec<ScanTile> {
    let player = npc_scan_cached_rw(st, world, content, px, py, r);
    if npc_player_scan_covers_home_square(px, py, home_x, home_y) {
        return player;
    }
    match world.try_read() {
        Ok(w) => {
            let home = scan_world_radius(&w, Some(content), home_x, home_y, r);
            npc_merge_scan_tiles(player, home)
        }
        Err(_) => player,
    }
}

/// Same as [`npc_scan_cached`] but takes the world lock only on a cache miss
/// so human USE/DROP is not blocked by a reused snapshot.
fn npc_scan_cached_rw(
    st: &mut NpcProfessionState,
    world: &RwLock<World>,
    content: &ContentDb,
    cx: i32,
    cy: i32,
    r: i32,
) -> Vec<ScanTile> {
    st.scan_calls = st.scan_calls.saturating_add(1);
    if let Some(hit) = npc_scan_try_cache(st, cx, cy, r) {
        return hit;
    }
    match world.try_read() {
        Ok(w) => npc_scan_fill(st, &w, content, cx, cy, r),
        Err(_) => {
            // Don't block the sim: reuse last snapshot or skip. Target
            // validity is checked again before USE (Haxe expected parent).
            if let Some(c) = &st.scan_cache {
                if c.r >= r {
                    return filter_scan_tiles_in_radius(&c.tiles, c.cx, c.cy, r.min(c.r));
                }
                return c.tiles.clone();
            }
            Vec::new()
        }
    }
}

/// Reserved NPC conn id base (above self-play).
pub const NPC_CONN_BASE: u64 = 9_100_000;

#[derive(Debug, Clone)]
pub struct NpcConfig {
    pub enabled: bool,
    pub min: u32,
    pub max: u32,
    pub think_period_ticks: u32,
    /// Haxe `AiReactionTime` (Commoner seconds).
    pub reaction_time: f32,
    /// Haxe `AiReactionTimeSerf`.
    pub reaction_time_serf: f32,
    /// Haxe `AiReactionTimeNoble`.
    pub reaction_time_noble: f32,
    /// Haxe `AiReactionTimeFactorIfAngry`.
    pub reaction_time_factor_if_angry: f32,
    pub observe_radius: i32,
    pub craft_radius: i32,
    /// Haxe `ServerSettings.HungryWorkCost` (default 5).
    pub hungry_work_cost: f32,
    /// Haxe `ServerSettings.AiIgnoredFloorIds` (Bear Skin Rug 656/888).
    pub ignored_floor_ids: Vec<i32>,
}

impl Default for NpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min: 3,
            max: 40,
            think_period_ticks: 10,
            reaction_time: gameplay_defaults::AI_REACTION_TIME,
            reaction_time_serf: gameplay_defaults::AI_REACTION_TIME_SERF,
            reaction_time_noble: gameplay_defaults::AI_REACTION_TIME_NOBLE,
            reaction_time_factor_if_angry: gameplay_defaults::AI_REACTION_TIME_FACTOR_IF_ANGRY,
            observe_radius: 16,
            craft_radius: DEFAULT_CRAFT_RADIUS,
            hungry_work_cost: gameplay_defaults::HUNGRY_WORK_COST,
            ignored_floor_ids: AI_IGNORED_FLOOR_IDS.to_vec(),
        }
    }
}

impl NpcConfig {
    /// Map live hot-reload knobs â†’ NPC scheduler config.
    ///
    /// // Haxe: ServerSettings.NumberOfAis / MinNumberOfAis (static Reflect update same-tick)
    pub fn from_live(live: &LiveSettings) -> Self {
        Self {
            enabled: live.npc_enabled,
            min: live.npc_min,
            max: live.npc_max.max(live.npc_min),
            think_period_ticks: live.ai_think_period_ticks.max(1),
            reaction_time: live.ai_reaction_time.max(0.05),
            reaction_time_serf: live.ai_reaction_time_serf.max(0.05),
            reaction_time_noble: live.ai_reaction_time_noble.max(0.05),
            reaction_time_factor_if_angry: live.ai_reaction_time_factor_if_angry.max(0.05),
            observe_radius: live.ai_observe_radius.max(4),
            craft_radius: live.ai_craft_radius.max(8),
            hungry_work_cost: if live.hungry_work_cost.is_finite() && live.hungry_work_cost >= 0.0 {
                live.hungry_work_cost
            } else {
                gameplay_defaults::HUNGRY_WORK_COST
            },
            ignored_floor_ids: if live.ai_ignored_floor_ids.is_empty() {
                AI_IGNORED_FLOOR_IDS.to_vec()
            } else {
                live.ai_ignored_floor_ids.clone()
            },
        }
    }

    /// Haxe class-based reaction delay (seconds).
    // Haxe: AiBase.doTimeStuffHelper reactionTime by prestigeClass
    pub fn reaction_for_class(&self, class: PrestigeClass, angry: bool) -> f32 {
        let mut t = if class.is_noble_or_more() {
            self.reaction_time_noble
        } else if matches!(class, PrestigeClass::Serf) {
            self.reaction_time_serf
        } else {
            // Commoner / NotSet
            self.reaction_time
        };
        if angry {
            t *= self.reaction_time_factor_if_angry;
        }
        t.max(0.05)
    }
}

fn profession_for_index(i: u32) -> CraftProfession {
    match i % 3 {
        0 => CraftProfession::Forager,
        1 => CraftProfession::Farmer,
        _ => CraftProfession::Hunter,
    }
}

/// Assign prestige class for permanent NPCs (demo diversity + Haxe parity testing).
/// Foragerâ†’Serf, Farmerâ†’Commoner, Hunterâ†’Noble.
// Haxe: lineage.prestigeClass at birth (account score); NPCs get role-mapped class
fn prestige_class_for_npc_index(i: u32) -> PrestigeClass {
    match i % 3 {
        0 => PrestigeClass::Serf,
        1 => PrestigeClass::Commoner,
        _ => PrestigeClass::Noble,
    }
}

/// Haxe milkweed family may change 50â†’51â†’52 without cancelling use.
// Haxe: AiBase.isUsingItem milkweed exception
fn is_milkweed_family(parent_id: i32) -> bool {
    matches!(parent_id, 50 | 51 | 52)
}

/// Resolve base parent id for sticky object checks.
fn sticky_parent_id(content: &ContentDb, id: i32) -> i32 {
    if id == 0 {
        0
    } else {
        content.resolve_base_id(id)
    }
}

/// True if sticky MOVE goal is still valid (Haxe isStillExpectedItem / expectedUseTarget).
// Haxe: AiHelper.isStillExpectedItem; AiBase expectedUseTarget.parentId
fn sticky_move_still_valid(
    world: &World,
    content: &ContentDb,
    sticky: &NpcStickyMove,
) -> bool {
    if sticky.expected_parent_id == 0 {
        // Pure walk: valid until path completes (moving flag clears).
        return true;
    }
    let id = world.get_object(sticky.gx, sticky.gy);
    if id == 0 {
        return false;
    }
    let parent = sticky_parent_id(content, id);
    if parent == sticky.expected_parent_id {
        return true;
    }
    // Milkweed may flower/fruit mid-walk without invalidating.
    if is_milkweed_family(parent) && is_milkweed_family(sticky.expected_parent_id) {
        return true;
    }
    // Craft 59+124 → 128 on the same tile: dropTarget parent changes to clothing.
    // Haxe isPickingupCloths then sees clothing=b; keep the pickup sticky.
    if (sticky.pending_drop || sticky.pending_use)
        && npc_clothing_slot(content, parent).is_some()
    {
        return true;
    }
    false
}

fn set_sticky_move(
    st: &mut NpcProfessionState,
    gx: i32,
    gy: i32,
    expected_parent_id: i32,
    label: impl Into<String>,
) {
    set_sticky_arrive(
        st,
        gx,
        gy,
        expected_parent_id,
        0,
        StickyArrive::None,
        label,
    );
}



fn set_sticky_arrive(
    st: &mut NpcProfessionState,
    gx: i32,
    gy: i32,
    expected_parent_id: i32,
    use_actor_parent: i32,
    arrive: StickyArrive,
    label: impl Into<String>,
) {
    let move_from = st.sticky_move.as_ref().and_then(|s| {
        if s.gx == gx && s.gy == gy {
            s.move_from
        } else {
            None
        }
    });
    let label = label.into();
    if label.starts_with("pickup_cloth") {
        st.clothing_wait_apply = 0;
        st.tried_drop_count = 0;
    }
    st.sticky_move = Some(NpcStickyMove {
        gx,
        gy,
        expected_parent_id,
        use_actor_parent,
        pending_use: matches!(arrive, StickyArrive::Use),
        pending_drop: matches!(arrive, StickyArrive::Drop),
        label,
        move_from,
    });
}

fn clear_sticky_move(st: &mut NpcProfessionState) {
    st.sticky_move = None;
}

/// Haxe `AiBase.newBorn` — wipe reused NPC sticky state on rebirth.
// Haxe: AiBase.newBorn L327–346
fn npc_wipe_on_newborn(st: &mut NpcProfessionState) {
    *st = NpcProfessionState::default();
}

/// Haxe `CancleUse` — clear useTarget/useActor/expectedUseTarget; dropIsAUse false.
/// Does not clear dropTarget.
// Haxe: AiBase.CancleUse L8868–8874
fn npc_cancle_use(st: &mut NpcProfessionState) {
    st.use_is_drop_in_container = false;
    st.drop_is_a_use = false;
    let Some(s) = st.sticky_move.as_mut() else {
        return;
    };
    s.pending_use = false;
    s.use_actor_parent = 0;
    if !s.pending_drop {
        st.sticky_move = None;
    }
}

/// Haxe `resetTargets`: escape/food/CancleUse/trans* (not drop/remove).
// Haxe: AiBase.resetTargets L319–325; CancleUse L8868–8873
fn npc_reset_targets(st: &mut NpcProfessionState) {
    st.escape_target = None;
    st.food_goto.sticky_food = None;
    npc_cancle_use(st);
    st.craft_rt.item.clear_trans();
}

/// Sticky snapshot for NPC craft profession roles (multi-profession scan).
// Haxe: assignedProfession / lastProfession / jobByAge farm/smith/baker/potter/shepherd
fn npc_sticky_for_craft_profession(
    profession: CraftProfession,
    age: f32,
) -> Option<ProfessionStickySnapshot> {
    match profession {
        CraftProfession::Farmer => Some(ProfessionStickySnapshot {
            farm_assigned: Some(FarmProfession::BasicFarmer),
            farm_last: Some(FarmProfession::BasicFarmer),
            age,
            ..Default::default()
        }),
        CraftProfession::Smith => Some(ProfessionStickySnapshot {
            smith_assigned: true,
            smith_last: true,
            age,
            ..Default::default()
        }),
        CraftProfession::Hunter => Some(ProfessionStickySnapshot {
            hunter_assigned: true,
            hunter_last: true,
            age,
            ..Default::default()
        }),
        // Forager/Generic: age-rotated multi-profession (NPC-SCAN-FULL).
        CraftProfession::Forager
        | CraftProfession::Explorer
        | CraftProfession::Generic => Some(ProfessionStickySnapshot {
            age,
            ..Default::default()
        }),
    }
}

fn collect_nearby(world: &World, px: i32, py: i32, radius: i32) -> Vec<NearbyObj> {
    let mut out = Vec::new();
    let r = radius.max(1).min(60);
    for dy in -r..=r {
        for dx in -r..=r {
            let x = px + dx;
            let y = py + dy;
            let id = world.get_object(x, y);
            if id != 0 {
                out.push(NearbyObj { id, x, y });
            }
        }
    }
    out
}

fn food_at(content: &ContentDb, id: i32) -> i32 {
    content.get(id).map(|d| d.food_value).unwrap_or(0)
}

/// Haxe `isEatable` food value: dummyParent then foodFromTarget then own `foodValue`.
// Haxe: AiHelper.isEatable L605–610; SearchBestFoodHelperNew L918–920
fn eatable_food_value_of(content: &ContentDb, id: i32) -> i32 {
    if id == 0 {
        return 0;
    }
    content.search_food_id_and_value(id).1
}

/// Haxe `canEatObj` without yum tables: foodValue, age, and stomach room.
// Haxe: GPI.canEatObj L6264–6272
fn npc_can_eat_held(content: &ContentDb, held_id: i32, food: f32, food_max: f32, age: f32) -> bool {
    if age < MIN_AGE_TO_EAT {
        return false;
    }
    let fv = eatable_food_value_of(content, held_id);
    can_eat_obj(fv, 0.0, food, food_max)
}

/// Haxe `isEating` → `myPlayer.self()` → `doSelf(..., clothingSlot=-1)` → `doEating`.
/// Not a ground `USE` (that is refused while moving).
// Haxe: AiBase.isEating L8829; GlobalPlayerInstance.self/doSelf L2693–2735
fn npc_eat_self_intent(conn_id: u64) -> NetIntent {
    NetIntent::Raw {
        conn_id,
        tag: "SELF".into(),
        payload: self_clothing_raw_payload(-1),
    }
}

/// NPC-thread [`FoodSearch`]: scores a pre-scanned nearby list with the **same**
/// pure SearchBestFood scorer as players (`ol_player_helper::pick_best_search_food`).
///
/// Candidates must already be gathered within radius (SeekFood uses
/// [`DEFAULT_FOOD_SEARCH_RADIUS`] = 40). Full world+container scan on the sim
/// writer uses [`ol_sim::search_best_food_full`] / [`ol_sim::best_food_for_ai`].
// Haxe: SearchBestFood processFood scoring; isObjectNotReachable skip
struct NpcNearbyFoodSearch<'a> {
    content: &'a ContentDb,
    nearby: &'a [NearbyObj],
    px: i32,
    py: i32,
    food_store: f32,
    food_store_max: f32,
    path_reach: Option<&'a AiPathReachMaps>,
}

impl FoodSearch for NpcNearbyFoodSearch<'_> {
    fn best_food(&self, q: BestFoodQuery) -> Option<BestFoodHit> {
        let r = if q.max_dist > 0 {
            q.max_dist
        } else {
            DEFAULT_FOOD_SEARCH_RADIUS
        };
        let mut cands: Vec<SearchFoodCand> = Vec::new();
        let mut stock_tiles: Vec<(i32, i32, i32, i32)> = Vec::new();
        for o in self.nearby {
            let d = (o.x - self.px).abs().max((o.y - self.py).abs());
            if d > r {
                continue;
            }
            let base = self.content.resolve_base_id(o.id);
            let Some(def) = self.content.get(base) else {
                continue;
            };
            let uses = if def.num_uses > 0 { def.num_uses } else { 1 };
            stock_tiles.push((o.x, o.y, base, uses));
            let (food_id, fv) = self.content.search_food_id_and_value(o.id);
            if fv <= 0 {
                continue;
            }
            let not_reachable = self
                .path_reach
                .map(|m| m.blocks_target(o.x, o.y, None))
                .unwrap_or(false);
            cands.push(SearchFoodCand {
                parent_id: base,
                food_id,
                food_value: fv,
                tx: o.x,
                ty: o.y,
                count_eaten: 0.0, // snapshot lacks full hasEatenMap; pure gates still apply
                number_of_uses: uses,
                index_in_container: -1,
                is_dangerous: false,
                not_reachable,
                food_factor: 1.0,
            });
        }
        let mut opts = ProcessFoodOpts::human(
            self.px,
            self.py,
            self.food_store,
            self.food_store_max,
            0,
        );
        // Haxe: SearchBestFood hasPepperSeeds / hasOnionSeeds / countSeeds on current objects
        opts.ai = Some(AiFoodSearchFlags::from_parent_ids(
            stock_tiles.iter().map(|t| t.2),
        ));
        let (idx, score) = pick_best_search_food(&cands, &opts, &stock_tiles)?;
        let cand = &cands[idx];
        let hit = to_best_hit(cand, &score, self.px, self.py);
        Some(BestFoodHit {
            x: hit.tx,
            y: hit.ty,
            // Haxe foodTarget is the ground/bush object, not foodFromTarget (31).
            food_id: cand.parent_id,
            score: hit.scored_food_value,
            is_yum: hit.scored_food_value > hit.food_value as f32,
        })
    }
}

/// Find best edible ground object via shared SearchBestFood pure scoring (r=40).
fn nearest_food(
    content: &ContentDb,
    nearby: &[NearbyObj],
    px: i32,
    py: i32,
    food_store: f32,
    food_store_max: f32,
    path_reach: Option<&AiPathReachMaps>,
) -> Option<NearbyObj> {
    let search = NpcNearbyFoodSearch {
        content,
        nearby,
        px,
        py,
        food_store,
        food_store_max,
        path_reach,
    };
    let hit = search.best_food_default(0)?;
    nearby
        .iter()
        .find(|o| o.x == hit.x && o.y == hit.y)
        .copied()
        .or(Some(NearbyObj {
            id: hit.food_id,
            x: hit.x,
            y: hit.y,
        }))
}


/// After prior food USE/DROP/REMV was sent, mark 30s if still empty-handed and tile food.
// Haxe: isPickingupFood done==false â†’ addNotReachableObject(food, 30) (AI-FOOD-FAIL-MARK)
fn settle_npc_pending_food_action(
    content: &ContentDb,
    nearby: &[NearbyObj],
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
) {
    let pending = st.food_goto.pending_food_xy.take();
    let pending_container = std::mem::take(&mut st.food_goto.pending_food_container);
    let pending_is_use = std::mem::take(&mut st.food_goto.pending_food_is_use);
    let Some((x, y)) = pending else {
        return;
    };
    // Ground food_value OR container/basket still present (REMV residual).
    // Haxe: container food often foodValue on contained, ground basket foodValue 0
    let (tile_id, ground_fv) = nearby
        .iter()
        .find(|o| o.x == x && o.y == y)
        .map(|o| (o.id, food_at(content, o.id)))
        .unwrap_or((0, 0));
    let tile_still_food =
        pending_food_tile_still_actionable(ground_fv, pending_container, tile_id);
    if p.held_id != 0 || pending_is_use {
        // Haxe L8703–8706: didNotReachFood=0; foodTarget=null; return true.
        // isUse (bush) succeeds without picking up; do not 30s-mark leftover food.
        st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
        st.food_goto.sticky_food = None;
        st.food_goto.last_goto = None;
        st.food_goto.last_goto_dist = -1.0;
        return;
    }
    if settle_pending_food_use_fail(
        &mut st.path_reach,
        &mut st.food_goto.sticky_food,
        Some((x, y)),
        p.held_id,
        tile_still_food,
    ) {
        st.food_goto.last_goto = None;
        st.food_goto.last_goto_dist = -1.0;
    } else if !tile_still_food {
        // Tile gone (someone ate it) â€” clear sticky without mark.
        st.food_goto.sticky_food = None;
        st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
    }
}

/// Resolve sticky foodTarget or adopt nearest edible (Haxe isPickingupFood + SearchBestFood).
// Haxe: AiBase.foodTarget sticky until pickup / path fail
fn resolve_npc_food_target(
    content: &ContentDb,
    nearby: &[NearbyObj],
    px: i32,
    py: i32,
    food_store: f32,
    food_store_max: f32,
    path_reach: &AiPathReachMaps,
    food_goto: &mut NpcFoodGotoState,
) -> Option<StickyFoodTarget> {
    // Container sticky: Haxe isEatableCheckAgain returns true when indexInContainer > -1.
    // Ground: re-fetch tile + dummyParent foodValue (`isEatable`).
    // Haxe: AiHelper.isEatableCheckAgain L594–602
    let sticky_tile = food_goto.sticky_food.map(|s| {
        if food_pickup_in_container(s.index_in_container) {
            (s.parent_id, eatable_food_value_of(content, s.parent_id))
        } else {
            let id = nearby
                .iter()
                .find(|o| o.x == s.x && o.y == s.y)
                .map(|o| o.id)
                .unwrap_or(0);
            let fv = eatable_food_value_of(content, id);
            (id, fv)
        }
    });
    let (sticky_id, sticky_fv) = sticky_tile.unwrap_or((0, 0));
    if let Some(s) = food_goto.sticky_food {
        if is_eatable_check_again(s.index_in_container, sticky_id, sticky_fv) {
            food_goto.sticky_food = Some(s);
            return Some(s);
        }
    }
    // MainAI + shared SearchBestFood pure scoring (same default r=40 as players).
    let search = NpcNearbyFoodSearch {
        content,
        nearby,
        px,
        py,
        food_store,
        food_store_max,
        path_reach: Some(path_reach),
    };
    let sensors = ThinkSensors {
        conn_id: 0,
        x: px,
        y: py,
        food_store,
        food_store_max,
        held_id: 0,
        moving: false,
    };
    let cand = match plan_hungry_food(&search, &sensors) {
        ThinkPlan::SeekFood { tx, ty, food_id }
        | ThinkPlan::UseFoodTile { tx, ty, food_id } => {
            Some(StickyFoodTarget::new(tx, ty, food_id))
        }
        ThinkPlan::Idle => nearest_food(
            content,
            nearby,
            px,
            py,
            food_store,
            food_store_max,
            Some(path_reach),
        )
        .map(|f| StickyFoodTarget::new(f.x, f.y, f.id)),
    };
    let resolved = resolve_sticky_food(food_goto.sticky_food, sticky_id, sticky_fv, cand);
    food_goto.sticky_food = resolved;
    resolved
}

/// Content permanent + food_value for food pickup SM (AI-PICKUP-FOOD).
// Haxe: foodTarget.isPermanent() / objectData.foodValue
fn food_meta(content: &ContentDb, id: i32) -> (bool, i32) {
    content
        .get(id)
        .map(|d| (d.permanent, d.food_value))
        .unwrap_or((false, 0))
}

/// Emit dropHeld / USE / DROP / REMV / walk for full isPickingupFood SM.
/// Returns true when the tick is consumed (Haxe return true).
// Haxe: AiBase.isPickingupFood ~8610â€“8706 (AI-PICKUP-FOOD)
fn npc_run_is_picking_up_food(
    content: &ContentDb,
    world: &Arc<RwLock<World>>,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    p: &PlayerSnapshot,
    food: StickyFoodTarget,
    st: &mut NpcProfessionState,
    nearby: &[NearbyObj],
) -> Option<(NpcActivityKind, String, u32)> {
    let (is_perm, fv) = food_meta(content, food.parent_id);
    // Haxe isEatableCheckAgain: container index > -1 always true; else re-fetch + dummyParent.
    // Haxe: AiBase.isPickingupFood L8618–8621; AiHelper L594–614
    let tile_still = if food_pickup_in_container(food.index_in_container) {
        is_eatable_check_again(food.index_in_container, food.parent_id, fv)
    } else {
        let tid = nearby
            .iter()
            .find(|o| o.x == food.x && o.y == food.y)
            .map(|o| o.id)
            .unwrap_or(0);
        let tfv = eatable_food_value_of(content, tid);
        is_eatable_check_again(-1, tid, tfv)
    };
    let is_holding = p.held_id > 0;
    // First plan pass assumes dropHeld would act when holding (Haxe dropHeldObject often true).
    let mut drop_would = is_holding;
    let mut plan = plan_is_picking_up_food(&IsPickingupFoodInput::from_sticky(
        food,
        is_perm,
        fv,
        tile_still,
        p.held_id,
        is_holding,
        p.moving,
        p.x,
        p.y,
        p.food,
        p.holding_player_id != 0,
        drop_would,
    ));

    // DropHeldBeforeMove: try smart drop; if no actionable intent, replan without drop_would.
    if let IsPickingupFoodPlan::DropHeldBeforeMove {
        max_distance_to_home,
    } = plan
    {
        let tiles = npc_scan_cached_rw(
            st,
            world.as_ref(),
            content,
            p.x,
            p.y,
            DEFAULT_FOOD_SEARCH_RADIUS,
        );
        let mut drop_extras = DropHeldSensorExtras::default();
        drop_extras.quiver = quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
        drop_extras.held_contains_clay = p.held_contains_clay;
        drop_extras.has_food_target = true;
        drop_extras.last_target_id = st.craft_rt.item.last_target_id;
        drop_extras.last_new_target_id = st.craft_rt.item.last_new_target_id;
        if drop_held_clears_drop_target(max_distance_to_home) {
            if let Some(s) = st.sticky_move.as_mut() {
                s.pending_drop = false;
            }
        }
        let intent = smart_drop_held_from_sensors_ex(
            p.held_id,
            p.held_uses.max(1),
            p.x,
            p.y,
            p.x,
            p.y,
            p.food,
            p.moving,
            false,
            max_distance_to_home,
            &tiles,
            drop_extras,
            Some(content),
        );
        if let Some(out) = npc_emit_drop_or_walk(
            intent_tx,
            conn_id,
            p,
            st,
            world,
            content,
            intent,
            "pickup_drop_before",
        ) {
            return Some(out);
        }
        // dropHeld returned none â€” continue SM with held still in hand
        drop_would = false;
        plan = plan_is_picking_up_food(&IsPickingupFoodInput::from_sticky(
            food,
            is_perm,
            fv,
            tile_still,
            p.held_id,
            is_holding,
            p.moving,
            p.x,
            p.y,
            p.food,
            p.holding_player_id != 0,
            drop_would,
        ));
    }

    match plan {
        IsPickingupFoodPlan::Inactive => None,
        IsPickingupFoodPlan::ClearedAlreadyHeld => {
            st.food_goto.sticky_food = None;
            st.food_goto.last_goto = None;
            st.food_goto.last_goto_dist = -1.0;
            None
        }
        IsPickingupFoodPlan::ClearedUneatable => {
            // Haxe L8618–8621 clears foodTarget. Returning busy here replayed
            // the same tile every think and never reached switchCloths.
            st.path_reach
                .add_not_reachable(food.x, food.y, 90.0);
            st.food_goto.sticky_food = None;
            st.food_goto.last_goto = None;
            st.food_goto.last_goto_dist = -1.0;
            None
        }
        IsPickingupFoodPlan::DropHeldBeforeMove { .. } => {
            // Unreachable after replan path above
            Some((NpcActivityKind::SeekFood, "drop_held_retry".into(), 100))
        }
        IsPickingupFoodPlan::BusyMoving => Some((
            NpcActivityKind::SeekFood,
            "pickup_busy_moving".into(),
            100,
        )),
        IsPickingupFoodPlan::GotoFood => {
            let tgt = LastGotoObj::new(food.x, food.y, food.parent_id);
            match plan_goto_obj(
                st.food_goto.last_goto,
                st.food_goto.last_goto_dist,
                tgt,
                p.x,
                p.y,
            ) {
                GotoObjPlan::AbortReceding { dist_quad } => {
                    // Equal dist is snapshot lag (Haxe never re-gotoObj while isMoving).
                    // Only abort when the goal actually got farther.
                    if dist_quad <= st.food_goto.last_goto_dist + 0.5 {
                        st.food_goto.last_goto = Some(tgt);
                        st.food_goto.last_goto_dist = dist_quad;
                        let walked = {
                            let w = world.read().unwrap();
                            npc_try_walk_to_sticky(
                                intent_tx,
                                &w,
                                content,
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                food.x,
                                food.y,
                                p.food,
                                food.parent_id,
                                true,
                                p.moving,
                                format!("walk_food id={}", food.parent_id),
                            npc_client_move_seq(p.done_moving_seq),
                            )
                        };
                        if walked {
                            return Some((
                                NpcActivityKind::SeekFood,
                                format!(
                                    "walk_food id={} @{},{}",
                                    food.parent_id, food.x, food.y
                                ),
                                250,
                            ));
                        }
                    }
                    st.path_reach.add_object_with_hostile_path(food.x, food.y);
                    apply_food_goto_fail(
                        &mut st.food_goto.did_not_reach_food,
                        &mut st.food_goto.sticky_food,
                        &mut st.food_goto.last_goto,
                        &mut st.food_goto.last_goto_dist,
                        None,
                    );
                    Some((
                        NpcActivityKind::SeekFood,
                        format!("food_receding @{},{}", food.x, food.y),
                        100,
                    ))
                }
                GotoObjPlan::Proceed { dist_quad } => {
                    st.food_goto.last_goto = Some(tgt);
                    st.food_goto.last_goto_dist = dist_quad;
                    let walked = {
                        let w = world.read().unwrap();
                        npc_try_walk_to_sticky(
                            intent_tx,
                            &w,
                            content,
                            st,
                            conn_id,
                            p.x,
                            p.y,
                            food.x,
                            food.y,
                            p.food,
                            food.parent_id,
                            true,
                            p.moving,
                            format!("walk_food id={}", food.parent_id),
                            npc_client_move_seq(p.done_moving_seq),
                        )
                    };
                    if walked {
                        return Some((
                            NpcActivityKind::SeekFood,
                            format!(
                                "walk_food id={} @{},{}",
                                food.parent_id, food.x, food.y
                            ),
                            250,
                        ));
                    }
                    let w = world.read().unwrap();
                    npc_mark_goto_path_fail(
                        &mut st.path_reach,
                        &w,
                        content,
                        p.x,
                        p.y,
                        food.x,
                        food.y,
                        p.food,
                        st.food_goto.did_not_reach_food,
                    );
                    apply_food_goto_fail(
                        &mut st.food_goto.did_not_reach_food,
                        &mut st.food_goto.sticky_food,
                        &mut st.food_goto.last_goto,
                        &mut st.food_goto.last_goto_dist,
                        None,
                    );
                    Some((
                        NpcActivityKind::SeekFood,
                        format!("food_goto_fail @{},{}", food.x, food.y),
                        100,
                    ))
                }
            }
        }
        IsPickingupFoodPlan::DropHeldPlayer => {
            // Haxe: dropPlayer(myPlayer.x, myPlayer.y) — live PUTDOWN/DROPBABY at feet
            let sent = npc_say_raw(intent_tx, conn_id, "SAY", "DROPBABY");
            Some((
                NpcActivityKind::SeekFood,
                "food_drop_held_player".into(),
                if sent { 400 } else { 100 },
            ))
        }
        IsPickingupFoodPlan::DropHeldForPickup => {
            let tiles = npc_scan_cached_rw(
                st,
                world.as_ref(),
                content,
                p.x,
                p.y,
                DEFAULT_FOOD_SEARCH_RADIUS,
            );
            let mut drop_extras = DropHeldSensorExtras::default();
            drop_extras.quiver = quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
            drop_extras.held_contains_clay = p.held_contains_clay;
            drop_extras.has_food_target = true;
            drop_extras.last_target_id = st.craft_rt.item.last_target_id;
            drop_extras.last_new_target_id = st.craft_rt.item.last_new_target_id;
            // Haxe: maxDistanceToHome < 1 → dropTarget = null
            if drop_held_clears_drop_target(0.0) {
                if let Some(s) = st.sticky_move.as_mut() {
                    s.pending_drop = false;
                }
            }
            let intent = smart_drop_held_from_sensors_ex(
                p.held_id,
                p.held_uses.max(1),
                p.x,
                p.y,
                p.x,
                p.y,
                p.food,
                p.moving,
                false,
                0.0, // Haxe dropHeldObject(0)
                &tiles,
                drop_extras,
                Some(content),
            );
            if let Some(out) = npc_emit_drop_or_walk(
                intent_tx,
                conn_id,
                p,
                st,
                world,
                content,
                intent,
                "pickup_drop_for",
            ) {
                return Some(out);
            }
            // Haxe L8671–8674: dropHeldObject(0) result ignored; always return true.
            Some((
                NpcActivityKind::SeekFood,
                "pickup_drop_held_0".into(),
                100,
            ))
        }
        IsPickingupFoodPlan::Remv { x, y, index } => {
            let payload = format!("{x} {y} {index}");
            // PlayerWriteInterface: same Raw path as human clients
            if npc_say_raw(&intent_tx, conn_id, "REMV", &payload) {
                // Haxe L8703–8706 after remove() true.
                st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
                st.food_goto.sticky_food = None;
                st.food_goto.last_goto = None;
                st.food_goto.last_goto_dist = -1.0;
                st.food_goto.pending_food_xy = Some((x, y));
                st.food_goto.pending_food_container = true;
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("remv_food id={} idx={} @{},{}", food.parent_id, index, x, y),
                    500,
                ));
            }
            // try_send fail â†’ 30s
            mark_food_path_fail(&mut st.path_reach, x, y);
            st.food_goto.sticky_food = None;
            Some((
                NpcActivityKind::SeekFood,
                format!("remv_food_fail mark30 @{},{}", x, y),
                100,
            ))
        }
        IsPickingupFoodPlan::Use { x, y } => {
            if npc_use_at(&intent_tx, conn_id, x, y, None, None)
            {
                // Haxe L8703–8706 after use() true: reset didNotReachFood + clear foodTarget.
                st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
                st.food_goto.sticky_food = None;
                st.food_goto.last_goto = None;
                st.food_goto.last_goto_dist = -1.0;
                st.food_goto.pending_food_xy = Some((x, y));
                st.food_goto.pending_food_container = food.in_container();
                st.food_goto.pending_food_is_use = true;
                return Some((
                    NpcActivityKind::SeekFood,
                    format!(
                        "use_food id={} fv={} perm={}",
                        food.parent_id, fv, is_perm
                    ),
                    500,
                ));
            }
            mark_food_path_fail(&mut st.path_reach, x, y);
            st.food_goto.sticky_food = None;
            Some((
                NpcActivityKind::SeekFood,
                format!("use_food_fail mark30 @{},{}", x, y),
                100,
            ))
        }
        IsPickingupFoodPlan::DropOnFood { x, y } => {
            // PlayerWriteInterface: same Drop command as human clients
            if npc_drop_at(&intent_tx, conn_id, x, y, None) {
                // Haxe L8703–8706 after drop() true.
                st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
                st.food_goto.sticky_food = None;
                st.food_goto.last_goto = None;
                st.food_goto.last_goto_dist = -1.0;
                st.food_goto.pending_food_xy = Some((x, y));
                st.food_goto.pending_food_container = false;
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("drop_pickup_food id={} @{},{}", food.parent_id, x, y),
                    500,
                ));
            }
            mark_food_path_fail(&mut st.path_reach, x, y);
            st.food_goto.sticky_food = None;
            Some((
                NpcActivityKind::SeekFood,
                format!("drop_pickup_fail mark30 @{},{}", x, y),
                100,
            ))
        }
    }
}

/// Enqueue smart-drop DropAt / UseAt / Goto / Wait for food-pickup dropHeld path.
fn npc_emit_drop_or_walk(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    intent: ShortCraftLiveIntent,
    tag: &str,
) -> Option<(NpcActivityKind, String, u32)> {
    match intent {
        ShortCraftLiveIntent::DropAt { x, y } => {
            // Haxe isClose is orthogonal only. Chebyshev-1 used to DROP on a
            // diagonal, the sim rejected it, and rope 59 never left the hand.
            if npc_is_close_action(p.x, p.y, x, y) {
                if p.moving {
                    // Haxe isDropingItem L8428: already in range, wait until stopped.
                    // Another goto refreshed move_path, so DROP L8456 never ran.
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_wait_drop @{},{}", x, y),
                        100,
                    ));
                }
                if npc_drop_at(&intent_tx, conn_id, x, y, None) {
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_drop @{},{}", x, y),
                        400,
                    ));
                }
                return None;
            }
            let (sx, sy) = npc_action_stand_tile(p.x, p.y, x, y);
            let walked = {
                let w = world.read().unwrap();
                npc_try_walk_to(
                    intent_tx,
                    &w,
                    content,
                    conn_id,
                    p.x,
                    p.y,
                    sx,
                    sy,
                    p.food,
                    st.food_goto.did_not_reach_food,
                    st.animal_path,
                    npc_client_move_seq(p.done_moving_seq),
                )
            };
            if walked {
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("{tag}_walk_drop @{},{}", sx, sy),
                    250,
                ));
            }
            None
        }
        ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id,
            ..
        } => {
            if npc_is_close_action(p.x, p.y, x, y) {
                if p.moving {
                    // Haxe isDropingItem L8428 / isUsingItem L9013: wait, do not goto.
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_wait_use @{},{}", x, y),
                        100,
                    ));
                }
                if npc_use_at(&intent_tx, conn_id, x, y, None, None) {
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_use tid={} @{},{}", target_id, x, y),
                        400,
                    ));
                }
                return None;
            } else if {
                let (sx, sy) = npc_action_stand_tile(p.x, p.y, x, y);
                let w = world.read().unwrap();
                npc_try_walk_to(
                    intent_tx,
                    &w,
                    content,
                    conn_id,
                    p.x,
                    p.y,
                    sx,
                    sy,
                    p.food,
                    st.food_goto.did_not_reach_food,
                    st.animal_path,
                    npc_client_move_seq(p.done_moving_seq),
                )
            } {
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("{tag}_walk_use @{},{}", x, y),
                    250,
                ));
            }
            None
        }
        ShortCraftLiveIntent::Goto { x, y } => {
            if {
                let w = world.read().unwrap();
                npc_try_walk_to(
                    intent_tx,
                    &w,
                    content,
                    conn_id,
                    p.x,
                    p.y,
                    x,
                    y,
                    p.food,
                    st.food_goto.did_not_reach_food,
                    st.animal_path,
                    npc_client_move_seq(p.done_moving_seq),
                )
            } {
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("{tag}_goto @{},{}", x, y),
                    250,
                ));
            }
            None
        }
        ShortCraftLiveIntent::Wait => Some((
            NpcActivityKind::SeekFood,
            format!("{tag}_wait"),
            100,
        )),
        ShortCraftLiveIntent::SelfClothing { slot } => {
            let payload = self_clothing_raw_payload(slot);
            if intent_tx
                .try_send(NetIntent::Raw {
                    conn_id,
                    tag: "SELF".into(),
                    payload,
                })
                .is_ok()
            {
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("{tag}_self slot={}", slot),
                    400,
                ));
            }
            None
        }
        _ => None,
    }
}


/// Live `blockedByAI` plus other NPCs' in-flight craft tiles (not yet sticky).
// Haxe: isObjectNotReachable ORs blockedByAI; craft_progress = extra in-flight claims
fn npc_merged_blocked_by_ai(
    share: &BlockedByAiShare,
    craft_progress: &HashMap<u64, ((i32, i32), i32)>,
    self_conn: u64,
) -> HashMap<(i32, i32), f32> {
    let global = snapshot_blocked_by_ai_share(share);
    blocked_by_ai_with_peer_progress(
        &global,
        craft_progress
            .iter()
            .filter(|(cid, _)| **cid != self_conn)
            .map(|(_, ((tx, ty), _))| (*tx, *ty)),
    )
}

/// Own food/drop/use/remove/block claims for `RemoveBlockedByAi` during think.
// Haxe: AiBase.RemoveBlockedByAi L260–267
fn npc_own_block_source(p: &PlayerSnapshot, st: &NpcProfessionState) -> AiAgentBlockSource {
    let mut sticky = p.ai_block_targets.clone();
    if sticky.food_target.is_none() {
        if let Some(f) = st.food_goto.sticky_food {
            sticky.set_food(BlockTargetClaim::simple(f.x, f.y, f.parent_id));
        } else if p.ai_sticky_food_id != 0 {
            sticky.set_food(BlockTargetClaim::simple(
                p.ai_sticky_food_x,
                p.ai_sticky_food_y,
                p.ai_sticky_food_id,
            ));
        }
    }
    if sticky.use_target.is_none() {
        if let Some(s) = st.sticky_move.as_ref() {
            if s.pending_use {
                sticky.set_use(BlockTargetClaim::simple(
                    s.gx,
                    s.gy,
                    s.expected_parent_id.max(1),
                ));
            }
        }
    }
    if sticky.drop_target.is_none() {
        if let Some(s) = st.sticky_move.as_ref() {
            if s.pending_drop {
                sticky.set_drop(BlockTargetClaim::simple(
                    s.gx,
                    s.gy,
                    s.expected_parent_id.max(1),
                ));
            }
        }
    }
    if sticky.remove_from_container_target.is_none() {
        if let Some(r) = st.remove_from_container.as_ref() {
            sticky.set_remove_from_container(BlockTargetClaim::simple(
                r.tx,
                r.ty,
                r.expected_parent.max(1),
            ));
        }
    }
    sticky.to_agent_block_source(p.age, false, false, sticky.player_block_sim_time)
}

/// Live `blockedByAI` for this AI's think: peers stay, own claims are unclaimed.
// Haxe: AiBase.RunAi L199 RemoveBlockedByAi then doTimeStuff
fn npc_blocked_by_ai_for_think(
    share: &BlockedByAiShare,
    craft_progress: &HashMap<u64, ((i32, i32), i32)>,
    self_conn: u64,
    p: &PlayerSnapshot,
    st: &NpcProfessionState,
) -> HashMap<(i32, i32), f32> {
    let mut merged = npc_merged_blocked_by_ai(share, craft_progress, self_conn);
    remove_agent_blocked_by_ai(&mut merged, &npc_own_block_source(p, st));
    merged
}

fn log_ev(
    log: &NpcActivityLog,
    conn_id: u64,
    p: &PlayerSnapshot,
    kind: NpcActivityKind,
    cpu_us: u32,
    game_ms: u32,
    detail: impl Into<String>,
) {
    let detail = detail.into();
    // Haxe DebugAi is off: in-flight MOVE / infant wait flood the log and stall sim ticks.
    if kind == NpcActivityKind::Move
        && (detail.starts_with("walk_use") || detail.starts_with("walk_target"))
    {
        return;
    }
    if kind == NpcActivityKind::Baby
        && (detail.starts_with("baby_wait") || detail.starts_with("baby_hungry"))
    {
        return;
    }
    if matches!(kind, NpcActivityKind::Stuck | NpcActivityKind::StuckCycle)
        && detail.contains("baby_wait")
    {
        return;
    }
    log.push(NpcActivityEvent {
        wall_unix_ms: 0,
        conn_id,
        p_id: p.p_id,
        kind,
        cpu_us,
        game_ms,
        age: p.age,
        food: p.food,
        x: p.x,
        y: p.y,
        held_id: p.held_id,
        detail,
    });
}

/// Run single AI scheduler thread loop (async task).
///
/// `live_share` is re-read each wake (~200 ms) so `server.toml` hot-reload
/// adjusts `npc_enabled` / min / max / observe / craft radius on the same
/// Haxe `MaxAiSkipedTicksBeforeReducingAIs`.
pub const MAX_AI_SKIPPED_TICKS_BEFORE_REDUCING: u64 = 10;

/// Haxe `currentMaxAIs` step every 200 ticks from skipped-tick window.
// Haxe: AiBase.RunAi L172–176
pub fn adjust_ai_current_max(
    current_max: u32,
    min: u32,
    max: u32,
    last_skipped_ticks: u64,
    reduce_threshold: u64,
) -> u32 {
    let min = min.max(1);
    let max = max.max(min);
    let mut m = current_max.clamp(min, max);
    if m < max && last_skipped_ticks < reduce_threshold {
        m += 1;
    }
    if m > min && last_skipped_ticks > reduce_threshold {
        m = m.saturating_sub(1);
    }
    m.clamp(min, max)
}

/// Haxe spawn gate: `tick % 20 != 0 && count < currentMax && (lastSkiped < Max || count < Min)`.
/// Empty server still fills AIs (Eve/Adam when no human is left; not a spawn ban).
// Haxe: AiBase.RunAi L153–155
pub fn should_spawn_new_ai(
    tick: u64,
    living: u32,
    current_max: u32,
    min: u32,
    last_skipped_ticks: u64,
    reduce_threshold: u64,
) -> bool {
    if living >= current_max {
        return false;
    }
    if tick % 20 == 0 {
        return false;
    }
    last_skipped_ticks < reduce_threshold || living < min
}

/// Haxe `ServerAi.doRebirth`: deleted (or pruned from player_views) and under cap.
///
/// Dead NPCs are removed from [`SimState::publish_player_view`], so a missing
/// view is the death signal — waiting for `p.deleted` never fires.
// Haxe: AiBase.RunAi L192 `if (ai.player.deleted && aiCountAlive < currentMaxAIs)`
pub fn npc_slot_should_rebirth(view_alive: bool, living: u32, current_max: u32) -> bool {
    !view_alive && living < current_max.max(1)
}

/// Haxe `Server.main` loads players (each with `new ServerAi`) **before** `DoTimeLoop`.
/// Skip Eve LOGIN until the sim has published at least one tick of that roster.
// Haxe: Server.main L76 `loaded $aiCount Ais` then L81 TimeHelper.DoTimeLoop
pub fn npc_scheduler_sim_ready(sim_tick: u64) -> bool {
    sim_tick > 0
}

/// Slot index for `NPC_CONN_BASE + i` logins (`i` is ServerAi.number, tens).
/// Loaded PLB1 bodies are `LOADED_PLAYER_CONN_BASE + p_id` (~11e6) and must
/// not look like a slot — that conn minus `NPC_CONN_BASE` is millions.
// Haxe: ServerAi.number vs GlobalPlayerInstance.ReadPlayers L573–577
pub fn npc_slot_index(conn_id: u64) -> Option<u32> {
    let d = conn_id.checked_sub(NPC_CONN_BASE)?;
    // NumberOfAis is 40; 10_000 leaves room without matching loaded p_id conns.
    if d >= 10_000 {
        return None;
    }
    u32::try_from(d).ok()
}

/// Haxe `Connection.getAis()` living: loaded `ServerAi`, NPC slots, takeover.
// Haxe: GlobalPlayerInstance.ReadPlayers L577 `connection.serverAi = new ServerAi(obj)`
pub fn npc_counts_as_living_ai(
    deleted: bool,
    ai_controlled: bool,
    is_ai: bool,
    conn_id: u64,
) -> bool {
    !deleted && (ai_controlled || is_ai || conn_id >= NPC_CONN_BASE)
}

fn npc_view_is_living_ai(p: &PlayerSnapshot) -> bool {
    npc_counts_as_living_ai(p.deleted, p.ai_controlled, p.is_ai, p.conn_id)
}

/// Retry delay after a rebirth LOGIN if the view is still empty (not `f32::MAX`).
pub const NPC_REBIRTH_RETRY_SECS: f32 = 2.0;

fn count_living_npcs(player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>) -> u32 {
    player_views
        .read()
        .ok()
        .map(|g| g.values().filter(|p| npc_view_is_living_ai(p)).count() as u32)
        .unwrap_or(0)
}

/// Full `doTimeStuff` conn ids: living ServerAi bodies + in-flight NPC slots +
/// previously alive slots (death / rebirth). Haxe iterates `Connection.getAis()`.
// Haxe: AiBase.RunAi L191 `for (ai in Connection.getAis())`
fn npc_think_conn_ids(
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    active: u32,
    stuck_map: &HashMap<u64, NpcStuckTracker>,
) -> Vec<u64> {
    let mut ids: Vec<u64> = player_views
        .read()
        .ok()
        .map(|g| {
            g.values()
                .filter(|p| npc_view_is_living_ai(p))
                .map(|p| p.conn_id)
                .collect()
        })
        .unwrap_or_default();
    for i in 0..active {
        ids.push(NPC_CONN_BASE + i as u64);
    }
    for (&cid, t) in stuck_map {
        if t.ever_alive {
            ids.push(cid);
        }
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// wake as the sim `live_share` write (CONFIG-SETTINGS; no 2 s lag).
///
/// // Haxe: ServerSettings.NumberOfAis statics update mid-session via readFromFile
pub async fn run_npc_scheduler(
    live_share: Arc<RwLock<LiveSettings>>,
    intent_tx: tokio::sync::mpsc::Sender<NetIntent>,
    world: Arc<RwLock<World>>,
    content: Arc<ContentDb>,
    player_views: Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    counters: Arc<Counters>,
    activity: Arc<NpcActivityLog>,
    // Reverse craft graph for multi-step GetOrCraft / craftItem expand (AI-CRAFT-NPC-ENQUEUE).
    craft_graph: Arc<ReverseCraftGraph>,
    // NPC-SCAN-FULL: live SimState.blocked_by_ai mirror (Haxe blockedByAI OR)
    blocked_by_ai: BlockedByAiShare,
    // DROP-HELD-LIVE: season for HandlingFire winter kindling (Haxe TimeHelper.Season)
    env_view: EnvView,
    // AI-PRIO-LIVE: GetCloseDeadlyAnimal from sim-published AnimalWorld
    animals: Arc<RwLock<AnimalWorld>>,
) {
    let labels = ["npc-forager", "npc-farmer", "npc-hunter"];
    let mut tick: u64 = 0;
    let mut active: u32 = 0;
    // Haxe `currentMaxAIs` starts at MinNumberOfAis; lastSkipedTicks starts at 100
    // so the first window only fills the min until skip-tick balance runs.
    let mut current_max_ais: u32 = 0;
    let mut last_skipped_ticks: u64 = 100;
    let mut skip_at_window_start: u64 = 0;
    let mut last_balance_sim_tick: u64 = 0;
    let mut stuck_map: HashMap<u64, NpcStuckTracker> = HashMap::new();
    /// conn â†’ (craft_key, remaining_cooldown_thinks)
    let mut craft_blacklist: HashMap<u64, HashMap<String, u32>> = HashMap::new();
    /// conn â†’ (target_xy, best_dist_seen) for progress tracking
    let craft_progress: HashMap<u64, ((i32, i32), i32)> = HashMap::new();
    /// conn â†’ sticky farm/smith/baker task state for profession ladder scan.
    let mut profession_state: HashMap<u64, NpcProfessionState> = HashMap::new();
    let mut announced = false;
    // AI-JOB-SMITH-RESID: load-time objectIdArrays[455] Chisel cache (PatchObjectData once)
    // Haxe: ServerSettings.PatchObjectData ~612â€“616
    let chisel_table = SteelChiselFamilyTable::from_content(content.as_ref());
    let chisel_family_extra = chisel_table.extras.clone();

    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
        tick = tick.wrapping_add(1);
        activity.try_flush();
        // Haxe cleanupBlockedObjects(reactionTime) / blockedByAI time decay.
        if let Ok(mut g) = blocked_by_ai.write() {
            g.retain(|_, t| {
                *t -= 0.2;
                *t > 0.0
            });
        }

        // Same-wake as sim hot-reload: LiveSettings â†’ NpcConfig (no outer 2 s copy).
        let cfg = live_share
            .read()
            .map(|g| NpcConfig::from_live(&g))
            .unwrap_or_else(|_| NpcConfig::default());
        if !cfg.enabled {
            if tick % 50 == 1 {
                debug!("npc scheduler idle (npc_enabled=false; hot-reload can re-enable)");
            }
            continue;
        }

        let min = cfg.min.max(1);
        let max = cfg.max.max(min);
        let think_period = cfg.think_period_ticks.max(1);
        let radius = cfg.observe_radius.max(4);
        let craft_radius = cfg.craft_radius.max(8).min(80);
        // Scheduler wake dt (matches sleep above). Haxe doTimeStuff(timePassedInSeconds).
        const SCHED_DT_SEC: f32 = 0.2;

        if !announced {
            announced = true;
            current_max_ais = min;
            skip_at_window_start = counters.skip_ticks.load(Ordering::Relaxed);
            info!(
                min,
                max, think_period, radius, craft_radius, "npc scheduler started (Haxe min/max + skip-tick balance)"
            );
        }

        // Haxe AiBase.RunAi every 200 ticks: grow/shrink currentMaxAIs from skipped ticks.
        let sim_tick = counters.ticks.load(Ordering::Relaxed);
        let skips_now = counters.skip_ticks.load(Ordering::Relaxed);
        if sim_tick >= last_balance_sim_tick.saturating_add(200) {
            last_skipped_ticks = skips_now.saturating_sub(skip_at_window_start);
            skip_at_window_start = skips_now;
            last_balance_sim_tick = sim_tick;
            let grown = adjust_ai_current_max(
                current_max_ais,
                min,
                max,
                last_skipped_ticks,
                MAX_AI_SKIPPED_TICKS_BEFORE_REDUCING,
            );
            if grown != current_max_ais {
                info!(
                    from = current_max_ais,
                    to = grown,
                    last_skipped_ticks,
                    "npc: Haxe skip-tick currentMaxAIs"
                );
            }
            current_max_ais = grown;
        }

        let living = count_living_npcs(&player_views);
        // Haxe: tick % 20 != 0 && aiCount < currentMaxAIs && (lastSkiped < Max || count < Min)
        // Do not LOGIN extra Eves before PLB1 ServerAi bodies are in player_views.
        // Haxe: Server.main L76 loaded Ais then L81 DoTimeLoop
        if npc_scheduler_sim_ready(sim_tick)
            && should_spawn_new_ai(
                tick,
                living,
                current_max_ais,
                min,
                last_skipped_ticks,
                MAX_AI_SKIPPED_TICKS_BEFORE_REDUCING,
            )
            && active < current_max_ais
        {
            let conn_id = NPC_CONN_BASE + active as u64;
            let email = format!("{}@local", labels[active as usize % labels.len()]);
            let _ = intent_tx
                .send(NetIntent::Login {
                    conn_id,
                    reconnect: false,
                    email,
                    client_tag: "client_npc".into(),
                    client_ip: String::new(),
                })
                .await;
            info!(conn_id, living, current_max_ais, "npc: login requested");
            active += 1;
        }

        // Full think list = Haxe Connection.getAis() (loaded ServerAi + NPC slots).
        // Thin takeover only for ai_controlled bodies not already on that list.
        // Haxe: AiBase.RunAi L191 for (ai in Connection.getAis()) doTimeStuff
        let think_conns = npc_think_conn_ids(&player_views, active, &stuck_map);
        let think_set: HashSet<u64> = think_conns.iter().copied().collect();
        {
            let takeover: Vec<u64> = {
                let views = player_views.read().unwrap();
                let mut ids: Vec<u64> = views
                    .values()
                    .filter(|o| {
                        o.ai_controlled && !o.deleted && !think_set.contains(&o.conn_id)
                    })
                    .map(|o| o.conn_id)
                    .collect();
                ids.sort_unstable();
                ids
            };
            for (ti, conn_id) in takeover.into_iter().enumerate() {
                if (tick as u32 + ti as u32) % think_period != 0 {
                    continue;
                }
                let snap = player_views
                    .read()
                    .ok()
                    .and_then(|g| g.get(&conn_id).cloned());
                let Some(p) = snap else { continue };
                if p.deleted || p.moving {
                    continue;
                }
                let hungry = {
                    let st = profession_state.entry(conn_id).or_default();
                    let e = check_is_hungry_and_eat_effects(
                        st.was_hungry,
                        p.food,
                        p.food_max,
                        p.held_id,
                        p.age,
                        st.food_goto.sticky_food.is_some(),
                    );
                    st.was_hungry = e.is_hungry;
                    if e.clear_caring_for_fire {
                        st.is_caring_for_fire = false;
                    }
                    e.is_hungry
                };
                if hungry && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                    let _ = intent_tx.try_send(npc_eat_self_intent(conn_id));
                    continue;
                }
                let nearby = {
                    let w = world.read().unwrap();
                    collect_nearby(&w, p.x, p.y, radius)
                };
                // PATH-REACH-MERGE: pull once per takeover think (food + explore arms).
                // Haxe: AiBase single maps always current; dual ownership â†’ pull each think
                // PATH-REACH-MERGE / dual_map_merge
                {
                    let st = profession_state.entry(conn_id).or_default();
                    pull_player_path_reach(st, &p);
                    st.path_reach.cleanup(0.2 * think_period as f32);
                    st.animal_path = Some(npc_animal_path_ctx(content.as_ref(), &p));
                }
                let mut did_food = false;
                {
                    // Haxe isPickingupFood L8611: sticky foodTarget even if not hungry.
                    let st = profession_state.entry(conn_id).or_default();
                    settle_npc_pending_food_action(&content, &nearby, &p, st);
                    let had_food_target = st.food_goto.sticky_food.is_some();
                    let food = if had_food_target {
                        st.food_goto.sticky_food
                    } else if hungry {
                        resolve_npc_food_target(
                            &content,
                            &nearby,
                            p.x,
                            p.y,
                            p.food,
                            p.food_max,
                            &st.path_reach,
                            &mut st.food_goto,
                        )
                    } else {
                        None
                    };
                    if let Some(food) = food {
                        let _ = npc_run_is_picking_up_food(
                            &content,
                            &world,
                            &intent_tx,
                            conn_id,
                            &p,
                            food,
                            st,
                            &nearby,
                        );
                        did_food = true;
                    }
                }
                if !did_food {
                    // Explore one step in a rotating cardinal (animal footprint avoid when gate open).
                    // Haxe: CreateCollisionChunk considerAnimal on explore Goto
                    let dirs = [(1i32, 0), (0, 1), (-1, 0), (0, -1)];
                    let (dx, dy) = dirs[(tick as usize + ti) % 4];
                    let nx = p.x + dx;
                    let ny = p.y + dy;
                    let walkable = {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        let consider = consider_animals_for_goto(
                            true,
                            st.food_goto.did_not_reach_food,
                            p.food,
                        );
                        if consider {
                            let ab = collect_deadly_animal_blocked_around_for_player(
                                &w,
                                &content,
                                p.x,
                                p.y,
                                GOTO_COLLISION_RAD,
                                Some(npc_animal_path_ctx(content.as_ref(), &p)),
                            );
                            is_walkable_with_animals(&w, &content, nx, ny, Some(&ab))
                        } else {
                            is_walkable(&w, &content, nx, ny)
                        }
                    };
                    if walkable {
                        let _ = intent_tx.try_send(NetIntent::Move {
                            conn_id,
                            xs: p.x,
                            ys: p.y,
                            deltas: vec![(dx, dy)],
                            seq: Some(npc_client_move_seq(p.done_moving_seq)),
                        });
                    }
                }
                // PATH-REACH-MERGE: push takeover food/walk marks for tick_vitals absorb
                // (was missing â€” AI-TAKEOVER never wrote into player_views before)
                // Haxe: AiBase L85â€“86 single maps
                if let Some(st) = profession_state.get(&conn_id) {
                    push_npc_path_reach_to_views(&player_views, conn_id, &st.path_reach);
                    push_npc_food_goto_to_views(&player_views, conn_id, &st.food_goto);
                }
            }
        }

        for (think_idx, conn_id) in think_conns.into_iter().enumerate() {
            let i = npc_slot_index(conn_id).unwrap_or(think_idx as u32);
            // Ensure prestige class for reaction timing (Forager=Serf, Farmer=Commoner, Hunter=Noble).
            {
                let st = profession_state.entry(conn_id).or_default();
                // Re-assert role class if still default Commoner and never assigned.
                if !st.class_assigned {
                    if npc_slot_index(conn_id).is_some() {
                        st.prestige_class = prestige_class_for_npc_index(i);
                    }
                    st.class_assigned = true;
                }
            }

            let snap = player_views
                .read()
                .ok()
                .and_then(|g| g.get(&conn_id).cloned());
            // Dead NPCs are pruned from player_views; missing == deleted.
            let alive = snap.as_ref().map(|p| !p.deleted).unwrap_or(false);
            let tracker = stuck_map.entry(conn_id).or_default();
            if !alive {
                // First LOGIN is in-flight: view is empty until sim applies it.
                if !tracker.ever_alive {
                    continue;
                }
                // Haxe ServerAi.doRebirth — must run even when the view was dropped.
                if !tracker.was_deleted {
                    tracker.was_deleted = true;
                    let age = snap.as_ref().map(|p| p.age).unwrap_or(14.0);
                    tracker.rebirth_wait_sec =
                        ai_rebirth_wait_secs(age, rand::random::<f32>());
                    if let Some(p) = snap.as_ref() {
                        log_ev(
                            &activity,
                            conn_id,
                            p,
                            NpcActivityKind::Death,
                            0,
                            0,
                            format!(
                                "age={:.1} food={:.1} reason=deleted_or_starved held={} wait={:.1}s",
                                p.age, p.food, p.held_id, tracker.rebirth_wait_sec
                            ),
                        );
                    } else {
                        info!(
                            conn_id,
                            wait = tracker.rebirth_wait_sec,
                            "npc: slot empty (pruned death), rebirth wait"
                        );
                    }
                    continue;
                }
                tracker.rebirth_wait_sec -= SCHED_DT_SEC;
                if tracker.rebirth_wait_sec > 0.0 {
                    continue;
                }
                if npc_slot_index(conn_id).is_none() {
                    // Loaded ServerAi: Haxe doRebirth on same Connection. Cap refill
                    // is createNewServerAiWithNewPlayer when living < currentMax.
                    tracker.ever_alive = false;
                    continue;
                }
                if !npc_slot_should_rebirth(false, living, current_max_ais) {
                    tracker.rebirth_wait_sec = NPC_REBIRTH_RETRY_SECS;
                    continue;
                }
                // Retry soon if LOGIN does not restore a living view (do not use MAX).
                tracker.rebirth_wait_sec = NPC_REBIRTH_RETRY_SECS;
                let email = format!("npc-slot-{}@local", i);
                let _ = intent_tx.try_send(NetIntent::Login {
                    conn_id,
                    reconnect: false,
                    email,
                    client_tag: "client_npc".into(),
                    client_ip: String::new(),
                });
                info!(conn_id, living, current_max_ais, "npc: rebirth login");
                if let Some(st) = profession_state.get_mut(&conn_id) {
                    // Haxe: ServerAi.doRebirth → ai.newBorn() L327–351
                    npc_wipe_on_newborn(st);
                }
                continue;
            }
            let mut p = snap.expect("alive NPC has a view");
            tracker.was_deleted = false;
            tracker.ever_alive = true;
            tracker.rebirth_wait_sec = 0.0;
            tracker.note_position(p.x, p.y);

            // Haxe: AiBase.doTimeStuff L393–409 — movedOneTile escape BEFORE time>0 wait
            let moved_one_tile = {
                let st = profession_state.entry(conn_id).or_default();
                let moved = st
                    .last_think_xy
                    .map(|(x, y)| x != p.x || y != p.y)
                    .unwrap_or(true);
                st.last_think_xy = Some((p.x, p.y));
                moved
            };
            {
                let st = profession_state.entry(conn_id).or_default();
                st.think_time_sec -= SCHED_DT_SEC;
                if should_escape_on_moved_one_tile(
                    moved_one_tile,
                    st.food_goto.did_not_reach_food,
                ) {
                    if let Some((k, d, ms)) = npc_try_escape_now(
                        &intent_tx,
                        world.as_ref(),
                        content.as_ref(),
                        animals.as_ref(),
                        player_views.as_ref(),
                        st,
                        conn_id,
                        &p,
                    ) {
                        log_ev(&activity, conn_id, &p, k, ms, 0, d);
                        continue;
                    }
                }
                if st.think_time_sec > 1.0 {
                    st.think_time_sec = 1.0;
                }
                if st.think_time_sec > 0.0 {
                    continue;
                }
            }

            let timer = ScopeTimer::start();
            {
                let st = profession_state.entry(conn_id).or_default();
                st.scan_cache = None;
                st.scan_us_acc = 0;
                st.scan_calls = 0;
                st.scan_hits = 0;
            }

            // Haxe: time += reactionTime; isAngryOrTerrified * AiReactionTimeFactorIfAngry
            // Haxe: AiBase.doTimeStuffHelper L418–424
            {
                let st = profession_state.entry(conn_id).or_default();
                let angry = is_angry_or_terrified(p.angry_time);
                let reaction = cfg.reaction_for_class(st.prestige_class, angry);
                st.think_time_sec += reaction;
                st.was_idle = decay_was_idle(st.was_idle, reaction);
                // Haxe TimeHelper.tick — same tick as MoveHelper.newMoves.
                st.now_tick = sim_tick;
                p.moving = npc_haxe_is_moving(p.moving, st.move_sent_tick, sim_tick);
            }

            // Haxe checkIsHungryAndEat hysteresis + baby F + fire-care wipe.
            // Haxe: AiBase.checkIsHungryAndEat L8841–8866
            let hungry = {
                let st = profession_state.entry(conn_id).or_default();
                let e = check_is_hungry_and_eat_effects(
                    st.was_hungry,
                    p.food,
                    p.food_max,
                    p.held_id,
                    p.age,
                    st.food_goto.sticky_food.is_some(),
                );
                st.was_hungry = e.is_hungry;
                if e.clear_caring_for_fire {
                    st.is_caring_for_fire = false;
                }
                if p.ai_debug_say && e.is_hungry {
                    let _ = npc_say_raw(
                        &intent_tx,
                        conn_id,
                        "SAY",
                        &format!("F {}", p.food.round()),
                    );
                }
                if e.baby_say_f {
                    let _ = npc_say_raw(&intent_tx, conn_id, "SAY", "F");
                }
                e.is_hungry
            };
            let starving = p.food < -1.0;
            let has_living_mother = p.held_by > 0
                || (p.ai_follow_p_id > 0
                    && player_views.read().ok().is_some_and(|g| {
                        g.values()
                            .any(|o| o.p_id == p.ai_follow_p_id && !o.deleted)
                    }));

            // Haxe: if (movedOneTileTmp == false && isMoving()) return
            // Replan after each arrived tile (feeding / eat / escape can retarget a long craft walk).
            // Haxe: AiBase.doTimeStuffHelper L428
            if p.moving {
                let (still_valid, pending_use, pending_drop, sticky_label) = {
                    let st = profession_state.entry(conn_id).or_default();
                    match st.sticky_move.clone() {
                        None => (true, false, false, String::new()),
                        Some(ref sticky) => {
                            let w = world.read().unwrap();
                            let ok = sticky_move_still_valid(&w, &content, sticky);
                            (ok, sticky.pending_use, sticky.pending_drop, sticky.label.clone())
                        }
                    }
                };
                // Haxe L428: skip the whole think only mid-tile. After a committed
                // tile, hunger/escape/baby run (checkIsHungryAndEat before isUsingItem).
                if haxe_skip_mid_path_think(true, moved_one_tile) {
                    let detail = if pending_use || pending_drop {
                        format!("walk_use {sticky_label}")
                    } else if sticky_label.is_empty() {
                        "walk_target moving".into()
                    } else {
                        format!("walk_target {sticky_label}")
                    };
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::Move,
                        0,
                        250,
                        detail,
                    );
                    continue;
                }
                if !still_valid {
                    if let Some(st) = profession_state.get_mut(&conn_id) {
                        clear_sticky_move(st);
                    }
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::StuckCycle,
                        0,
                        0,
                        format!("sticky_invalid interrupt was={sticky_label}"),
                    );
                }
            }

            let profession = profession_for_index(i);

            let nearby = {
                let w = world.read().unwrap();
                collect_nearby(&w, p.x, p.y, craft_radius)
            };

            // PATH-REACH-MERGE: pull Player marks once per think (all arms incl. explore/craft).
            // Haxe: single AiBase maps always current; dual ownership â†’ pull each think
            // PATH-REACH-MERGE / dual_map_merge
            {
                let st = profession_state.entry(conn_id).or_default();
                pull_player_path_reach(st, &p);
                st.path_reach.cleanup(0.2 * think_period as f32);
                st.animal_path = Some(npc_animal_path_ctx(content.as_ref(), &p));
            }

            let mut acted = false;
            let mut detail = String::new();
            let mut kind = NpcActivityKind::Think;
            let mut game_ms = 200u32;

            // Haxe: countSeeds L440 then cleanUpProfessions L454
            {
                let st = profession_state.entry(conn_id).or_default();
                let w = world.read().unwrap();
                let tiles = npc_scan_cached(st, &w, content.as_ref(), p.x, p.y, 60);
                let flags = count_seeds_from_scan(&tiles);
                st.has_corn_seeds = flags.has_corn_seeds;
                st.has_carrot_seeds = flags.has_carrot_seeds;
                st.has_pepper_seeds = has_pepper_seeds_from_scan(&tiles);
                st.has_onion_seeds = has_onion_seeds_from_scan(&tiles);
                if flags.early_return {
                    continue;
                }
                npc_clean_up_professions(st, &p);
            }
            // Haxe: AutoFollowAi && isHuman → time=0.2 isMovingToPlayer(2,false) L460–464
            if auto_follow_ai_human_early_return(false, !p.is_ai && p.connected && !p.ai_controlled)
            {
                let st = profession_state.entry(conn_id).or_default();
                st.think_time_sec = 0.2;
                continue;
            }

            // Haxe: getHeldByPlayer() != null → return (carried baby does not think)
            // Haxe: AiBase.doTimeStuffHelper L472–475
            if p.held_by > 0 {
                continue;
            }

            // --- 0. Escape deadly animal/player (AI-PRIO-LIVE) ---
            // Haxe: doTimeStuffHelper GetCloseDeadly* + escape before hungry L486–492
            if !acted {
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_try_escape_now(
                    &intent_tx,
                    world.as_ref(),
                    content.as_ref(),
                    animals.as_ref(),
                    player_views.as_ref(),
                    st,
                    conn_id,
                    &p,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L497 isDropingItem; L500–510 close isUsingItem (quad < 25).
            // After escape + checkIsHungryAndEat; before waitingTime / baby.
            if !acted {
                let pending = profession_state
                    .get(&conn_id)
                    .and_then(|st| st.sticky_move.clone());
                if pending.is_none() {
                    // Haxe L8353: dropTarget == null → triedDropCount = 0
                    if let Some(st) = profession_state.get_mut(&conn_id) {
                        st.tried_drop_count = 0;
                    }
                }
                if let Some(sticky) = pending {
                    let arrive = StickyArrive::from_flags(sticky.pending_use, sticky.pending_drop);
                    let valid = {
                        let w = world.read().unwrap();
                        sticky_move_still_valid(&w, &content, &sticky)
                    };
                    if !valid || arrive == StickyArrive::None {
                        if let Some(st) = profession_state.get_mut(&conn_id) {
                            clear_sticky_move(st);
                        }
                    } else if arrive == StickyArrive::Drop {
                        let applied = {
                            let w = world.read().unwrap();
                            let st = profession_state.entry(conn_id).or_default();
                            npc_apply_dropping_item(
                                &intent_tx,
                                &w,
                                &content,
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                p.food,
                                p.moving,
                                p.held_id,
                                p.holding_player_id,
                                hungry,
                                p.ai_follow_p_id != 0,
                                &sticky,
                                npc_client_move_seq(p.done_moving_seq),
                            )
                        };
                        if let Some((k, d, ms)) = applied {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    } else if sticky.label.starts_with("reed_skirt_before_baby")
                        || sticky.label.starts_with("yew_bow_before_baby")
                        || npc_sticky_early_return(arrive, p.x, p.y, sticky.gx, sticky.gy)
                    {
                        let applied = {
                            let w = world.read().unwrap();
                            let st = profession_state.entry(conn_id).or_default();
                            npc_apply_sticky_arrive(
                                &intent_tx,
                                &w,
                                &content,
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                p.food,
                                false,
                                p.held_id,
                                p.holding_player_id,
                                &sticky,
                                npc_client_move_seq(p.done_moving_seq),
                                p.held_uses,
                                p.home_x,
                                p.home_y,
                                p.age,
                            )
                        };
                        if let Some((k, d, ms)) = applied {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        } else if let Some(st) = profession_state.get_mut(&conn_id) {
                            clear_sticky_move(st);
                        }
                    }
                }
            }

            // Haxe L514–518: deadlyPlayer==null && waitingTime>1 skip
            if !acted {
                let deadly_player = {
                    let views = player_views.read().ok();
                    views.as_ref().and_then(|vs| {
                        let cands = npc_deadly_player_candidates(&p, vs, content.as_ref());
                        get_close_deadly_player(
                            p.x,
                            p.y,
                            p.angry_time,
                            p.home_x,
                            p.home_y,
                            DEADLY_PLAYER_SEARCH_DIST_AI,
                            &cands,
                        )
                    })
                };
                let st = profession_state.entry(conn_id).or_default();
                if waiting_time_blocks_think(deadly_player.is_some(), st.waiting_time) {
                    let (add, w) = tick_waiting_time(st.waiting_time);
                    st.think_time_sec += add;
                    st.waiting_time = w;
                    kind = NpcActivityKind::Think;
                    detail = "waiting_time".into();
                    game_ms = 1000;
                    acted = true;
                }
            }

            // Haxe L523–532: hungry infant BEFORE isChildAndHasMother; always returns.
            if !acted && npc_hungry_infant_skips_craft(p.age, hungry, has_living_mother) {
                if let Some(st) = profession_state.get_mut(&conn_id) {
                    clear_sticky_move(st);
                }
                let views_g = player_views.read().ok();
                let walked5 = if let Some(views) = views_g.as_ref() {
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
                    npc_try_follow_player_within(
                        &intent_tx,
                        &w,
                        content.as_ref(),
                        conn_id,
                        &p,
                        st,
                        views,
                        baby_hungry_follow_tiles(),
                        "baby_hungry_5",
                    )
                } else {
                    None
                };
                if let Some((k, d, ms)) = walked5 {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                } else {
                    let walked3 = if let Some(views) = views_g.as_ref() {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        npc_try_follow_player_within(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            conn_id,
                            &p,
                            st,
                            views,
                            3,
                            "baby_hungry_3",
                        )
                    } else {
                        None
                    };
                    if let Some(st) = profession_state.get_mut(&conn_id) {
                        st.think_time_sec += 2.5;
                    }
                    if let Some((k, d, ms)) = walked3 {
                        kind = k;
                        detail = d;
                        game_ms = ms;
                    } else {
                        kind = NpcActivityKind::Baby;
                        detail = "baby_hungry_stay".into();
                        game_ms = 2500;
                    }
                    acted = true;
                }
            }

            // Haxe L533–548: isChildAndHasMother — walk, then handleTemperature, then nice baby.
            // Non-nice baby that is already close falls through.
            if !acted {
                let views_g = player_views.read().ok();
                let child_with_mother = views_g.as_ref().is_some_and(|views| {
                    let mother_deleted = !views
                        .values()
                        .any(|o| o.p_id == p.ai_follow_p_id && !o.deleted);
                    is_child_and_has_mother_from_follow(
                        p.age,
                        p.ai_follow_p_id > 0,
                        mother_deleted,
                        MIN_AGE_TO_EAT,
                    )
                });
                if child_with_mother {
                    if let Some(views) = views_g.as_ref() {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        if let Some((k, d, ms)) = npc_try_follow_player_within(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            conn_id,
                            &p,
                            st,
                            views,
                            child_with_mother_follow_tiles(p.ai_is_nice_baby),
                            "baby_follow_mother",
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                    if !acted {
                        let env_winter = env_view
                            .read()
                            .ok()
                            .map(|e| e.is_winter())
                            .unwrap_or(false);
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        if let Some((k, d, ms)) = npc_run_handle_temperature(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            craft_graph.as_ref(),
                            env_winter,
                            conn_id,
                            &p,
                            st,
                            tick,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                    if !acted && p.ai_is_nice_baby {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        if let Some((k, d, ms)) = npc_run_nice_baby_after_mother(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            conn_id,
                            &p,
                            st,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                }
            }
            // Haxe L549–552: isWounded || hasYellowFever → isMovingToPlayer(2); return
            if !acted {
                let wounded = npc_is_wounded(content.as_ref(), p.held_id) && !p.is_hidden_wound;
                if wounded || p.sick {
                    let views_g = player_views.read().ok();
                    if let Some(views) = views_g.as_ref() {
                        let follow = p.ai_follow_p_id;
                        if follow > 0 {
                            if let Some(t) = views.values().find(|o| o.p_id == follow && !o.deleted)
                            {
                                let max_q = 2 * 2;
                                let dx = t.x - p.x;
                                let dy = t.y - p.y;
                                if dx * dx + dy * dy >= max_q {
                                    let w = world.read().unwrap();
                                    let st = profession_state.entry(conn_id).or_default();
                                    let _ = npc_try_walk_to(
                                        &intent_tx,
                                        &w,
                                        content.as_ref(),
                                        conn_id,
                                        p.x,
                                        p.y,
                                        t.x,
                                        t.y,
                                        p.food,
                                        st.food_goto.did_not_reach_food,
                                        st.animal_path,
                                        npc_client_move_seq(p.done_moving_seq),
                                    );
                                }
                            }
                        }
                    }
                    kind = NpcActivityKind::Think;
                    detail = "wounded_seek_help".into();
                    game_ms = 250;
                    acted = true;
                }
            }

            // --- 0b. handleDeath (age ≥ MaxAge−2) after baby/wounded ---
            // Haxe: deadlyPlayer == null && handleDeath() L554
            if !acted {
                let max_age = live_share
                    .read()
                    .map(|g| {
                        if g.max_age.is_finite() && g.max_age > 2.0 {
                            g.max_age
                        } else {
                            gameplay_defaults::MAX_AGE
                        }
                    })
                    .unwrap_or(gameplay_defaults::MAX_AGE);
                if should_handle_death(p.age, max_age) {
                    let deadly_player = {
                        let views = player_views.read().ok();
                        views.as_ref().and_then(|vs| {
                            let cands = npc_deadly_player_candidates(&p, vs, content.as_ref());
                            get_close_deadly_player(
                                p.x,
                                p.y,
                                p.angry_time,
                                p.home_x,
                                p.home_y,
                                DEADLY_PLAYER_SEARCH_DIST_AI,
                                &cands,
                            )
                        })
                    };
                    if deadly_player.is_none() {
                        let st = profession_state.entry(conn_id).or_default();
                        wipe_jobs_assign_grave_keeper(
                            &mut st.farm_rt,
                            &mut st.smith_rt,
                            &mut st.baker_rt,
                            &mut st.shepherd_rt,
                            &mut st.pottery_rt,
                            &mut st.fire_rt,
                            &mut st.fire_keeper_rt,
                            &mut st.grave_keeper_rt,
                            &mut st.hunter_rt,
                            &mut st.lumberjack_rt,
                            &mut st.collector_rt,
                            &mut st.foodserver_rt,
                        );
                        if st.remove_from_container.is_some() {
                            let w = world.read().unwrap();
                            if let Some((k, d, ms)) = npc_run_remove_from_container(
                                &intent_tx,
                                &w,
                                content.as_ref(),
                                conn_id,
                                &p,
                                st,
                            ) {
                                kind = k;
                                detail = format!("handle_death_{d}");
                                game_ms = ms;
                                acted = true;
                            }
                        }
                        if acted {
                            // Haxe handleDeath returns after isRemovingFromContainer
                        } else {
                        let (home_x, home_y) =
                            peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                        let fire = if st.fire_place_id != 0 {
                            Some((st.fire_place_x, st.fire_place_y))
                        } else if p.ai_fire_place_id != 0 {
                            Some((p.ai_fire_place_x, p.ai_fire_place_y))
                        } else {
                            None
                        };
                        let (mtx, mty) = go_home_move_target(home_x, home_y, fire);
                        let home_quad =
                            (p.x - home_x) * (p.x - home_x) + (p.y - home_y) * (p.y - home_y);
                        let move_quad =
                            (p.x - mtx) * (p.x - mtx) + (p.y - mty) * (p.y - mty);
                        let seed = (p.p_id as u32)
                            .wrapping_mul(1103515245)
                            .wrapping_add(p.x as u32);
                        let rand = (seed as f32) / (u32::MAX as f32);
                        let plan = plan_handle_death(
                            p.age,
                            max_age,
                            false,
                            false,
                            p.moving,
                            home_quad,
                            move_quad,
                            rand,
                        );
                        if let Some(say) = plan.say {
                            let _ = npc_say_raw(&intent_tx, conn_id, "SAY", say);
                        }
                        let mut action = plan.action;
                        if action == HandleDeathAction::Busy {
                            kind = NpcActivityKind::Think;
                            detail = "handle_death_busy".into();
                            game_ms = 200;
                            acted = true;
                        } else if action == HandleDeathAction::GravesThenRest {
                            let tiles = npc_scan_cached_rw(
                                st,
                                world.as_ref(),
                                content.as_ref(),
                                home_x,
                                home_y,
                                GRAVE_SEARCH_RADIUS,
                            );
                            let sticky = ProfessionStickySnapshot::from_runtimes_ex(
                                &st.farm_rt,
                                &st.smith_rt,
                                &st.baker_rt,
                                Some(&st.shepherd_rt),
                                Some(&st.pottery_rt),
                                Some(&st.fire_rt),
                                Some(&st.fire_keeper_rt),
                                Some(&st.grave_keeper_rt),
                                Some(&st.hunter_rt),
                                Some(&st.lumberjack_rt),
                                Some(&st.collector_rt),
                                Some(&st.foodserver_rt),
                                p.age,
                            );
                            let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
                            inp.home_x = home_x;
                            inp.home_y = home_y;
                            inp.age = p.age;
                            inp.food_store = p.food;
                            inp.is_hungry = p.food < 5.0;
                            inp.is_assigned_job = true;
                            inp.profession_is_sticky = true;
                            let result = ladder_profession_scan_tick(
                                PriorityRung::HandleDeath,
                                &tiles,
                                &inp,
                                &sticky,
                                &mut st.farm_task,
                                &mut st.farm_rt,
                                &mut st.smith_rt,
                                &mut st.baker_rt,
                                &mut st.baker_task,
                                &mut st.shepherd_rt,
                                &mut st.pottery_rt,
                                &mut st.fire_rt,
                                &mut st.fire_keeper_rt,
                                &mut st.grave_keeper_rt,
                                &mut st.hunter_rt,
                                &mut st.lumberjack_rt,
                                &mut st.collector_rt,
                                &mut st.foodserver_rt,
                            );
                            if result.had_action {
                                match result.intent {
                                    ShortCraftLiveIntent::UseAt { x, y, .. } => {
                                        let dist = (x - p.x).abs().max((y - p.y).abs());
                                        if dist <= 1 {
                                            if npc_use_at(&intent_tx, conn_id, x, y, None, None) {
                                                kind = NpcActivityKind::Craft;
                                                detail = format!("handle_death_grave_use @{},{}", x, y);
                                                game_ms = 500;
                                                acted = true;
                                            }
                                        } else {
                                            let w = world.read().unwrap();
                                            if npc_try_walk_to(
                                                &intent_tx,
                                                &w,
                                                content.as_ref(),
                                                conn_id,
                                                p.x,
                                                p.y,
                                                x,
                                                y,
                                                p.food,
                                                st.food_goto.did_not_reach_food,
                                                st.animal_path,
                                                npc_client_move_seq(p.done_moving_seq),
                                            ) {
                                                kind = NpcActivityKind::Craft;
                                                detail = format!("handle_death_grave_walk @{},{}", x, y);
                                                game_ms = 250;
                                                acted = true;
                                            }
                                        }
                                    }
                                    ShortCraftLiveIntent::DropAt { x, y } => {
                                        if npc_drop_at(&intent_tx, conn_id, x, y, None) {
                                            kind = NpcActivityKind::Craft;
                                            detail = format!("handle_death_grave_drop @{},{}", x, y);
                                            game_ms = 400;
                                            acted = true;
                                        }
                                    }
                                    ShortCraftLiveIntent::Goto { x, y } => {
                                        let (gx, gy) = (x, y);
                                        let w = world.read().unwrap();
                                        if npc_try_walk_to(
                                            &intent_tx,
                                            &w,
                                            content.as_ref(),
                                            conn_id,
                                            p.x,
                                            p.y,
                                            gx,
                                            gy,
                                            p.food,
                                            st.food_goto.did_not_reach_food,
                                            st.animal_path,
                                            npc_client_move_seq(p.done_moving_seq),
                                        ) {
                                            kind = NpcActivityKind::Craft;
                                            detail = format!("handle_death_grave_seek @{},{}", gx, gy);
                                            game_ms = 250;
                                            acted = true;
                                        }
                                    }
                                    ShortCraftLiveIntent::SeekOrCraft { .. }
                                    | ShortCraftLiveIntent::CraftItem { .. }
                                    | ShortCraftLiveIntent::Wait => {
                                        kind = NpcActivityKind::Think;
                                        detail = "handle_death_grave_seek".into();
                                        game_ms = 200;
                                        acted = true;
                                    }
                                    _ => {}
                                }
                            }
                            if !acted {
                                action = handle_death_after_graves_miss(move_quad);
                            }
                        }
                        if !acted && action == HandleDeathAction::GoHome {
                            let (gx, gy) = go_home_goal_xy(mtx, mty, seed);
                            let w = world.read().unwrap();
                            if npc_try_walk_to(
                                &intent_tx,
                                &w,
                                content.as_ref(),
                                conn_id,
                                p.x,
                                p.y,
                                gx,
                                gy,
                                p.food,
                                st.food_goto.did_not_reach_food,
                                st.animal_path,
                                npc_client_move_seq(p.done_moving_seq),
                            ) {
                                kind = NpcActivityKind::Think;
                                detail = format!("handle_death_home @{},{}", gx, gy);
                                game_ms = 250;
                                acted = true;
                            }
                        }
                        if !acted && action == HandleDeathAction::DropHeld {
                            st.think_time_sec += HANDLE_DEATH_TIME_BUMP;
                            if p.held_id != 0 {
                                if npc_drop_at(&intent_tx, conn_id, p.x, p.y, None) {
                                    kind = NpcActivityKind::Craft;
                                    detail = "handle_death_drop".into();
                                    game_ms = 400;
                                    acted = true;
                                }
                            } else {
                                kind = NpcActivityKind::Think;
                                detail = "handle_death_wait".into();
                                game_ms = 200;
                                acted = true;
                            }
                        }
                        if !acted && action == HandleDeathAction::GravesThenRest {
                            kind = NpcActivityKind::Think;
                            detail = "handle_death_graves_idle".into();
                            game_ms = 200;
                            acted = true;
                        }
                        } // else not already removing
                    }
                }
            }

            // --- 1. isEating (Haxe L8796–8838): head + conservation + self() + peel drop.
            // Haxe isEating has no caller isMoving gate.
            if !acted && p.held_id > 0 {
                let st = profession_state.entry(conn_id).or_default();
                if st.eat_pending_peel {
                    st.eat_pending_peel = false;
                    let fv = eatable_food_value_of(&content, p.held_id);
                    if is_eating_drop_peel(fv) {
                        let tiles = npc_scan_cached_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
                            DEFAULT_FOOD_SEARCH_RADIUS,
                        );
                        let mut drop_extras = DropHeldSensorExtras::default();
                        drop_extras.quiver =
                            quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
                        drop_extras.held_contains_clay = p.held_contains_clay;
                        let intent = smart_drop_held_from_sensors_ex(
                            p.held_id,
                            p.held_uses.max(1),
                            p.x,
                            p.y,
                            p.home_x,
                            p.home_y,
                            p.food,
                            p.moving,
                            false,
                            EAT_PEEL_DROP_DIST,
                            &tiles,
                            drop_extras,
                            Some(content.as_ref()),
                        );
                        if let Some(out) = npc_emit_drop_or_walk(
                            &intent_tx,
                            conn_id,
                            &p,
                            st,
                            &world,
                            content.as_ref(),
                            intent,
                            "eat_peel",
                        ) {
                            kind = out.0;
                            detail = out.1;
                            game_ms = out.2;
                            acted = true;
                        }
                    }
                }
                if !acted {
                    let fv = eatable_food_value_of(&content, p.held_id);
                    let can_eat = npc_can_eat_held(
                        &content,
                        p.held_id,
                        p.food,
                        p.food_max,
                        p.age,
                    );
                    let holding_yum = is_obj_yum(fv, 0.0);
                    let num_uses = content
                        .get(content.resolve_base_id(p.held_id))
                        .map(|d| d.num_uses)
                        .unwrap_or(1);
                    let goose_close = {
                        let w = world.read().unwrap();
                        let tiles = npc_scan_cached(
                            st,
                            &w,
                            content.as_ref(),
                            p.home_x,
                            p.home_y,
                            COOKED_GOOSE_KEEP_RADIUS,
                        );
                        npc_count_parent_cheb(
                            &tiles,
                            p.home_x,
                            p.home_y,
                            COOKED_GOOSE_EAT,
                            COOKED_GOOSE_KEEP_RADIUS,
                        )
                    };
                    let skip = is_eating_conservation_skip(
                        hungry,
                        p.held_id,
                        p.held_uses,
                        num_uses,
                        goose_close,
                        st.has_onion_seeds,
                        st.has_pepper_seeds,
                    );
                    if is_eating_head(p.age, MIN_AGE_TO_EAT, can_eat, hungry, holding_yum)
                        && !skip
                    {
                        if st.eat_fail_held == p.held_id {
                            st.eat_fail_held = 0;
                        } else if intent_tx.try_send(npc_eat_self_intent(conn_id)).is_ok()
                        {
                            st.eat_fail_held = p.held_id;
                            st.eat_pending_peel = true;
                            st.food_goto.did_not_reach_food = food_pickup_action_success_reset();
                            st.food_goto.sticky_food = None;
                            kind = NpcActivityKind::Eat;
                            detail = format!("eat_held={}", p.held_id);
                            game_ms = 500;
                            acted = true;
                        }
                    }
                }
            } else if let Some(st) = profession_state.get_mut(&conn_id) {
                st.eat_fail_held = 0;
            }

            // --- 1a. isFeedingChild (Haxe after isEating, before pickup food) ---
            // Haxe: AiBase.isFeedingChild L6412 — BABY pickup + hold while TimeHelper nurses
            // Skip while isDropingItem / close isUsingItem (those return before feeding).
            if !acted {
                let skip_feed = {
                    let st = profession_state.get(&conn_id);
                    match st.and_then(|s| s.sticky_move.as_ref()) {
                        Some(sticky) => {
                            let arrive =
                                StickyArrive::from_flags(sticky.pending_use, sticky.pending_drop);
                            let valid = {
                                let w = world.read().unwrap();
                                sticky_move_still_valid(&w, &content, sticky)
                            };
                            npc_skip_feed_for_sticky(
                                p.x, p.y, sticky.gx, sticky.gy, valid, arrive,
                            )
                        }
                        None => false,
                    }
                };
                if !skip_feed {
                    let views_g = player_views.read().ok();
                    if let Some(views) = views_g.as_ref() {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        if let Some((k, d, ms)) = npc_run_is_feeding_child(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            conn_id,
                            &p,
                            st,
                            views,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                }
            }

            // --- 1b. switchCloths after isFeedingChild ---
            // Haxe: AiBase.doTimeStuffHelper L558
            if !acted {
                let class_u8 = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_prestige_class_u8(st.prestige_class)
                };
                if let Some((k, d, ms)) = npc_run_switch_cloths(
                    &intent_tx,
                    content.as_ref(),
                    conn_id,
                    &p,
                    class_u8,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // --- 1c. isConsideringMakingFood (Haxe before isPickingupFood) ---
            // Age/hungry enter → SMITH wipe + Eating, skip near/starving/too-far-from-home,
            // then 15s searchFoodAndEat + nested make-food body (sharpie / turkey / corn…).
            // Haxe: AiBase.isConsideringMakingFood L8466–8598; caller L573 deadlyPlayer==null
            if !acted && p.age >= MIN_AGE_TO_EAT {
                let consider_food_is_best_fire = {
                    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    npc_is_best_fire_keeper(
                        &p,
                        &player_views,
                        &profession_state,
                        content.as_ref(),
                        hx,
                        hy,
                    )
                };
                let has_deadly_player = {
                    let views_g = player_views.read().ok();
                    views_g.as_ref().and_then(|views| {
                        let cands = npc_deadly_player_candidates(&p, views, content.as_ref());
                        get_close_deadly_player(
                            p.x,
                            p.y,
                            p.angry_time,
                            p.home_x,
                            p.home_y,
                            DEADLY_PLAYER_SEARCH_DIST_AI,
                            &cands,
                        )
                    })
                    .is_some()
                };
                if has_deadly_player {
                    // Haxe L573: skip consider-food while a deadly player is present.
                } else {
                let st = profession_state.entry(conn_id).or_default();
                settle_npc_pending_food_action(&content, &nearby, &p, st);
                let had_sticky = st.food_goto.sticky_food.is_some();
                // Haxe L8480: if (!isHungry && foodTarget == null) return false
                // — do not SearchBestFood when full (that pinned harvest and blocked crafts).
                let mut food = if hungry {
                    resolve_npc_food_target(
                        &content,
                        &nearby,
                        p.x,
                        p.y,
                        p.food,
                        p.food_max,
                        &st.path_reach,
                        &mut st.food_goto,
                    )
                } else if had_sticky {
                    st.food_goto.sticky_food
                } else {
                    tracing::debug!(
                        conn_id,
                        p_id = p.p_id,
                        "ai_craft: skip consider_food not_hungry no_target"
                    );
                    None
                };
                let has_food = food.is_some();
                if should_wipe_smith_on_consider_food(
                    p.age,
                    hungry,
                    has_food,
                    MIN_AGE_TO_EAT,
                ) {
                    let last_fs =
                        npc_last_profession_key(st, &p) == Some("FOODSERVER");
                    apply_consider_making_food_smith_wipe(
                        &mut st.smith_rt,
                        p.age,
                        hungry,
                        has_food,
                        MIN_AGE_TO_EAT,
                        last_fs,
                    );
                    if !last_fs {
                        st.last_profession_key = Some("Eating".into());
                    }
                    let food_tgt = food.as_ref().map(|f| {
                        let dx = f.x - p.x;
                        let dy = f.y - p.y;
                        ((dx * dx + dy * dy) as f32, false, false)
                    });
                    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    let home_quad = {
                        let dx = hx - p.x;
                        let dy = hy - p.y;
                        (dx * dx + dy * dy) as f32
                    };
                    if !consider_making_food_skip_after_enter_with_home(
                        food_tgt, p.food, home_quad,
                    ) {
                        let passed = time_since_ticks_in_sec(
                            tick as f32,
                            st.last_consider_food_tick,
                            TIME_HELPER_TICK_TIME,
                        );
                        if consider_making_food_should_research(passed) {
                            st.last_consider_food_tick = tick as f32;
                            food = resolve_npc_food_target(
                                &content,
                                &nearby,
                                p.x,
                                p.y,
                                p.food,
                                p.food_max,
                                &st.path_reach,
                                &mut st.food_goto,
                            );
                            let _ = food;
                        }
                        let sticky_use = st
                            .sticky_move
                            .as_ref()
                            .map(|s| s.pending_use)
                            .unwrap_or(false);
                        if sticky_use {
                            // Haxe L8522 isUsingItem — handled by the far-use block below
                            // if this body returns None.
                        } else {
                            let animal_quad = animals.read().ok().and_then(|aw| {
                                aw.get_close_deadly_animal(p.x, p.y, DEADLY_ANIMAL_SEARCH_DIST)
                                    .map(|d| {
                                        let dx = d.x - p.x;
                                        let dy = d.y - p.y;
                                        (dx * dx + dy * dy) as f32
                                    })
                            });
                            let do_stuff = consider_making_food_do_stuff(
                                p.heat,
                                None,
                                animal_quad,
                                home_quad,
                            );
                            if do_stuff {
                                let views_g = player_views.read().ok();
                                let animals_g = animals.read().ok();
                                if let Some(views) = views_g.as_ref() {
                                    let tiles = npc_scan_cached_rw(
                                        st,
                                        world.as_ref(),
                                        content.as_ref(),
                                        p.x,
                                        p.y,
                                        WEAPON_SEARCH_DIST,
                                    );
                                    if let Some((k, d, ms)) = npc_run_attack_player(
                                        &intent_tx,
                                        &world.read().unwrap(),
                                        content.as_ref(),
                                        craft_graph.as_ref(),
                                        conn_id,
                                        &p,
                                        st,
                                        tick,
                                        views,
                                        &tiles,
                                    ) {
                                        kind = k;
                                        detail = d;
                                        game_ms = ms;
                                        acted = true;
                                    }
                                    let deadly = animals_g.as_ref().and_then(|aw| {
                                        aw.get_close_deadly_animal(
                                            p.x,
                                            p.y,
                                            DEADLY_ANIMAL_SEARCH_DIST,
                                        )
                                        .map(|d| (d.x, d.y, d.kind.object_id()))
                                    });
                                    let hunter_peers = npc_count_hunter_peers(
                                        views, conn_id, hx, hy, content.as_ref(),
                                    );
                                    let w = world.read().unwrap();
                                    if !acted {
                                    if let Some((k, d, ms)) = npc_run_kill_animal(
                                        &intent_tx,
                                        &w,
                                        content.as_ref(),
                                        craft_graph.as_ref(),
                                        st,
                                        conn_id,
                                        &p,
                                        tick,
                                        deadly,
                                        hunter_peers,
                                        animals_g.as_deref(),
                                    ) {
                                        kind = k;
                                        detail = d;
                                        game_ms = ms;
                                        acted = true;
                                    }
                                    }
                                }
                            }
                            if !acted {
                                let env_winter = env_view
                                    .read()
                                    .ok()
                                    .map(|e| e.is_winter())
                                    .unwrap_or(false);
                                let w = world.read().unwrap();
                                let graves = ProfessionLadderStep {
                                    kind: ProfessionScanKind::HandlingGraves,
                                    rung_label: "CONSIDER_MAKE_FOOD",
                                    farm_job: None,
                                    farm_has_profession: false,
                                    is_assigned_job: false,
                                    profession_is_sticky: false,
                                };
                                if let Some((k, d, ms)) = npc_run_ladder_step(
                                    graves,
                                    &intent_tx,
                                    &w,
                                    content.as_ref(),
                                    st,
                                    conn_id,
                                    &p,
                                    env_winter,
                                ) {
                                    kind = k;
                                    detail = d;
                                    game_ms = ms;
                                    acted = true;
                                }
                            }
                            if !acted {
                                let class_u8 = npc_prestige_class_u8(st.prestige_class);
                                let w = world.read().unwrap();
                                if let Some((k, d, ms)) = npc_run_is_pickingup_cloths(
                                    &intent_tx,
                                    &w,
                                    content.as_ref(),
                                    st,
                                    conn_id,
                                    &p,
                                    class_u8,
                                ) {
                                    kind = k;
                                    detail = d;
                                    game_ms = ms;
                                    acted = true;
                                }
                            }
                            if !acted {
                                let env_winter = env_view
                                    .read()
                                    .ok()
                                    .map(|e| e.is_winter())
                                    .unwrap_or(false);
                                let w = world.read().unwrap();
                                if let Some((k, d, ms)) = npc_run_is_handling_fire_mid(
                                    &intent_tx,
                                    &w,
                                    content.as_ref(),
                                    st,
                                    conn_id,
                                    &p,
                                    env_winter,
                                    consider_food_is_best_fire,
                                    consider_food_is_best_fire,
                                ) {
                                    kind = k;
                                    detail = d;
                                    game_ms = ms;
                                    acted = true;
                                }
                            }
                            if !acted {
                                let is_smith =
                                    matches!(profession, CraftProfession::Smith);
                                if let Some((k, d, ms)) = npc_run_considering_making_food(
                                    &intent_tx,
                                    world.as_ref(),
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    st,
                                    conn_id,
                                    &p,
                                    tick,
                                    is_smith,
                                ) {
                                    kind = k;
                                    detail = d;
                                    game_ms = ms;
                                    acted = true;
                                }
                            }
                        }
                    }
                }
                }
            }

            // Haxe later isUsingItem (far use, after feeding / isConsideringMakingFood).
            // Haxe: AiBase.doTimeStuffHelper L574 before isPickingupFood L576
            if !acted {
                let sticky = profession_state
                    .get(&conn_id)
                    .and_then(|st| st.sticky_move.clone());
                if let Some(sticky) = sticky {
                    let arrive = StickyArrive::from_flags(sticky.pending_use, sticky.pending_drop);
                    let valid = {
                        let w = world.read().unwrap();
                        sticky_move_still_valid(&w, &content, &sticky)
                    };
                    if valid && arrive == StickyArrive::Use {
                        if p.moving {
                            kind = NpcActivityKind::Move;
                            detail = format!("walk_use {}", sticky.label);
                            game_ms = 250;
                            acted = true;
                        } else {
                            let applied = {
                                let w = world.read().unwrap();
                                let st = profession_state.entry(conn_id).or_default();
                                npc_apply_sticky_arrive(
                                    &intent_tx,
                                    &w,
                                    &content,
                                    st,
                                    conn_id,
                                    p.x,
                                    p.y,
                                    p.food,
                                    false,
                                    p.held_id,
                                    p.holding_player_id,
                                    &sticky,
                                    npc_client_move_seq(p.done_moving_seq),
                                    p.held_uses,
                                    p.home_x,
                                    p.home_y,
                                    p.age,
                                )
                            };
                            if let Some((k, d, ms)) = applied {
                                kind = k;
                                detail = d;
                                game_ms = ms;
                                acted = true;
                            }
                        }
                    }
                }
            }

            // Haxe L575: isRemovingFromContainer after far isUsingItem, before isPickingupFood
            if !acted {
                let st = profession_state.entry(conn_id).or_default();
                if st.remove_from_container.is_some() {
                    let w = world.read().unwrap();
                    if let Some((k, d, ms)) = npc_run_remove_from_container(
                        &intent_tx,
                        &w,
                        content.as_ref(),
                        conn_id,
                        &p,
                        st,
                    ) {
                        kind = k;
                        detail = d;
                        game_ms = ms;
                        acted = true;
                    }
                }
            }

            // Haxe isUsingItem L574 before isPickingupFood. A held rope or reed
            // must reach 59+124 (or 59+131) before food dropHeldObject drops it.
            if !acted {
                let held_base = content.resolve_base_id(p.held_id);
                if held_base == 59 || held_base == 124 || held_base == 131 {
                    let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    let st = profession_state.entry(conn_id).or_default();
                    let blocked = st.path_reach.blocked_coords(None);
                    let intent = {
                        let w = world.read().unwrap();
                        npc_reed_skirt_direct(
                            &w,
                            content.as_ref(),
                            p.x,
                            p.y,
                            hx,
                            hy,
                            held_base,
                            NPC_GET_OR_CRAFT_SEARCH_RADIUS,
                            &blocked,
                        )
                        .filter(|_| held_base == 59 || held_base == 124)
                        .or_else(|| {
                            if held_base == 59 || held_base == 131 {
                                npc_yew_bow_direct(
                                    &w,
                                    content.as_ref(),
                                    p.x,
                                    p.y,
                                    hx,
                                    hy,
                                    held_base,
                                    NPC_GET_OR_CRAFT_SEARCH_RADIUS,
                                    &blocked,
                                )
                            } else {
                                None
                            }
                        })
                    };
                    if let Some(intent) = intent {
                        let committed = {
                            let w = world.read().unwrap();
                            npc_commit_craft_live(
                                &intent,
                                &intent_tx,
                                &w,
                                content.as_ref(),
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                p.food,
                                p.moving,
                                &mut kind,
                                &mut detail,
                                &mut game_ms,
                                npc_client_move_seq(p.done_moving_seq),
                            )
                        };
                        if committed {
                            acted = true;
                            detail = format!("pair_before_food {detail}");
                        }
                    }
                }
            }

            // --- 2. isPickingupFood (Haxe L576 unconditional; L8611 foodTarget==null → false)
            // Sticky: run SM (isEatableCheckAgain / dropHeld / USE / DROP / REMV).
            // No sticky + hungry: checkIsHungryAndEat searchFoodAndEat then pickup.
            // Haxe: AiBase.isPickingupFood L8610–8700
            if !acted {
                let st = profession_state.entry(conn_id).or_default();
                // PATH-REACH-MERGE: maps already pulled at think start
                settle_npc_pending_food_action(&content, &nearby, &p, st);
                // Haxe L8610: leftover foodTarget is still picked while full.
                let food = if st.food_goto.sticky_food.is_some() {
                    st.food_goto.sticky_food
                } else if hungry || starving {
                    let found = resolve_npc_food_target(
                        &content,
                        &nearby,
                        p.x,
                        p.y,
                        p.food,
                        p.food_max,
                        &st.path_reach,
                        &mut st.food_goto,
                    );
                    npc_search_food_and_eat_debug_say(
                        &intent_tx,
                        &content,
                        conn_id,
                        p.ai_debug_say,
                        found.as_ref(),
                    );
                    found
                } else {
                    None
                };
                if let Some(food) = food {
                    if let Some((k, d, ms)) = npc_run_is_picking_up_food(
                        &content,
                        &world,
                        &intent_tx,
                        conn_id,
                        &p,
                        food,
                        st,
                        &nearby,
                    ) {
                        kind = k;
                        detail = d;
                        game_ms = ms;
                        // Helper Some ⇒ Haxe isPickingupFood return true (tick consumed)
                        acted = true;
                    }
                }
            }

            // Haxe isUsingItem/isDropingItem after hunger: if still moving, keep path
            // (do not replan craft / USE). Food may have already retargeted above.
            if !acted && p.moving {
                let st = profession_state.entry(conn_id).or_default();
                if let Some(sticky) = st.sticky_move.clone() {
                    let arrive = StickyArrive::from_flags(sticky.pending_use, sticky.pending_drop);
                    let valid = {
                        let w = world.read().unwrap();
                        sticky_move_still_valid(&w, &content, &sticky)
                    };
                    if npc_skip_craft_while_sticky_moving(true, valid, arrive) {
                        kind = NpcActivityKind::Move;
                        detail = format!("walk_use {}", sticky.label);
                        game_ms = 250;
                        acted = true;
                    }
                }
            }

            // Decay craft blacklists.
            if let Some(bl) = craft_blacklist.get_mut(&conn_id) {
                bl.retain(|_, n| {
                    *n = n.saturating_sub(1);
                    *n > 0
                });
            }

            // Haxe L586: isHandlingTemperature && dist > 100 after pickup
            if !acted {
                let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                let animals_g = animals.read().ok();
                let views_g = player_views.read().ok();
                let threat_quad = if let Some(views) = views_g.as_ref() {
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input_ex(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                        &nearby,
                    );
                    fill_live_sensors(&input).threat_quad_dist
                } else {
                    10000.0
                };
                let handling = profession_state
                    .get(&conn_id)
                    .map(|s| s.handling_temperature)
                    .unwrap_or(false);
                if handle_temperature_after_pickup(handling, threat_quad) {
                    let env_winter = env_view
                        .read()
                        .ok()
                        .map(|e| e.is_winter())
                        .unwrap_or(false);
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
                    if let Some((k, d, ms)) = npc_run_handle_temperature(
                        &intent_tx,
                        &w,
                        content.as_ref(),
                        craft_graph.as_ref(),
                        env_winter,
                        conn_id,
                        &p,
                        st,
                        tick,
                    ) {
                        kind = k;
                        detail = d;
                        game_ms = ms;
                        acted = true;
                    }
                }
            }

            // --- 1c. attackPlayer (AI-ATTACK-PLAYER) ---
            // Haxe: doStuff && attackPlayer(playerTarget) ~591 after pickup food / temp
            if !acted {
                let views_g = player_views.read().ok();
                let animals_g = animals.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input_ex(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                        &nearby,
                    );
                    let bundle = fill_live_sensors(&input);
                    if bundle.sensors.do_stuff {
                        let tiles = npc_scan_cached_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
                            WEAPON_SEARCH_DIST,
                        );
                        if let Some((k, d, ms)) = npc_run_attack_player(
                            &intent_tx,
                            &world.read().unwrap(),
                            content.as_ref(),
                            craft_graph.as_ref(),
                            conn_id,
                            &p,
                            st,
                            tick,
                            views,
                            &tiles,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                }
            }

            // --- 1d. isStayingCloseToChild (Haxe after attackPlayer) ---
            // Haxe L592: doStuff && isStayingCloseToChild()
            if !acted {
                let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                let animals_g = animals.read().ok();
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input_ex(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                        &nearby,
                    );
                    let do_stuff = fill_live_sensors(&input).sensors.do_stuff;
                    if do_stuff {
                        let w = world.read().unwrap();
                        if let Some((k, d, ms)) = npc_run_stay_close_to_child(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            conn_id,
                            &p,
                            st,
                            views,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                }
            }

            // --- 1e. killAnimal (Haxe doStuff && killAnimal(deadlyAnimal) L593) ---
            if !acted {
                let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                let animals_g = animals.read().ok();
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input_ex(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                        &nearby,
                    );
                    let bundle = fill_live_sensors(&input);
                    if bundle.sensors.do_stuff {
                        let deadly = animals_g.as_ref().and_then(|aw| {
                            aw.get_close_deadly_animal(p.x, p.y, DEADLY_ANIMAL_SEARCH_DIST)
                                .map(|d| (d.x, d.y, d.kind.object_id()))
                        });
                        let home_x = if p.home_x != 0 || p.home_y != 0 {
                            p.home_x
                        } else {
                            p.x
                        };
                        let home_y = if p.home_x != 0 || p.home_y != 0 {
                            p.home_y
                        } else {
                            p.y
                        };
                        let hunter_peers = npc_count_hunter_peers(
                            views,
                            conn_id,
                            home_x,
                            home_y,
                            content.as_ref(),
                        );
                        let w = world.read().unwrap();
                        if let Some((k, d, ms)) = npc_run_kill_animal(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            craft_graph.as_ref(),
                            st,
                            conn_id,
                            &p,
                            tick,
                            deadly,
                            hunter_peers,
                            animals_g.as_deref(),
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                    // Haxe L594: doStuff && profession['SMITH'] < 1 && isFeedingPlayerInNeed()
                    if !acted && bundle.sensors.do_stuff {
                        let env_winter = env_view
                            .read()
                            .ok()
                            .map(|e| e.is_winter())
                            .unwrap_or(false);
                        let w = world.read().unwrap();
                        if let Some((k, d, ms)) = npc_run_feed_player_in_need(
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            st,
                            conn_id,
                            &p,
                            views,
                            env_winter,
                        ) {
                            kind = k;
                            detail = d;
                            game_ms = ms;
                            acted = true;
                        }
                    }
                }
            }

            // --- 2a. Continuous follow walk (AI-FOLLOW-WALK) ---
            // Haxe: AiBase.isMovingToPlayer after sticky playerToFollow / LLM follow
            if !acted {
                let follow_p = p.ai_follow_p_id;
                if follow_p > 0 {
                    let target = {
                        let views = player_views.read().unwrap();
                        views
                            .values()
                            .find(|o| o.p_id == follow_p && !o.deleted)
                            .cloned()
                    };
                    if let Some(t) = target {
                        let max_tiles = if p.ai_auto_stop_follow { 10 } else { 5 };
                        let max_q = max_tiles * max_tiles;
                        let dx = t.x - p.x;
                        let dy = t.y - p.y;
                        let qd = dx * dx + dy * dy;
                        if qd >= max_q {
                            let goal_x = t.x + 1;
                            let goal_y = t.y;
                            let dist = (goal_x - p.x).abs().max((goal_y - p.y).abs());
                            if dist > 1 && !p.moving {
                                if let Some((sdx, sdy)) = {
                                    let w = world.read().unwrap();
                                    next_step(&w, p.x, p.y, goal_x, goal_y, &|nx, ny| {
                                        is_walkable(&w, &content, nx, ny)
                                    })
                                } {
                                    if intent_tx
                                        .try_send(NetIntent::Move {
                                            conn_id,
                                            xs: p.x,
                                            ys: p.y,
                                            deltas: vec![(sdx, sdy)],
                                            seq: Some(npc_client_move_seq(p.done_moving_seq)),
                                        })
                                        .is_ok()
                                    {
                                        kind = NpcActivityKind::Think;
                                        detail = format!(
                                            "follow_walk target={} @{},{}",
                                            follow_p, t.x, t.y
                                        );
                                        game_ms = 250;
                                        acted = true;
                                    }
                                }
                            } else if p.moving {
                                kind = NpcActivityKind::Think;
                                detail = format!("follow_busy_moving target={follow_p}");
                                game_ms = 200;
                                acted = true;
                            }
                        }
                    }
                }
            }

            // Haxe L599: if (myPlayer.isMoving()) return — skip home/jobs/craft
            if !acted && skip_home_and_jobs_while_moving(p.moving) {
                kind = NpcActivityKind::Think;
                detail = "busy_moving_skip_jobs".into();
                game_ms = 200;
                acted = true;
            }

            // Haxe L601–603: searchNewHomeIfNeeded (always false after home reassign),
            // foundFamily + allyUp (SAY, no return).
            if !acted {
                let st = profession_state.entry(conn_id).or_default();
                let now = st.now_tick as f32;
                if let Some(plan) = plan_found_family(
                    p.p_id,
                    false,
                    false,
                    p.ai_follow_p_id,
                    0,
                    None,
                    0.0,
                    0,
                    0.0,
                    "",
                    &[],
                    1.0,
                ) {
                    let _ = npc_say_raw(&intent_tx, conn_id, "SAY", &plan.iam_say);
                    if plan.follow_me {
                        let _ = npc_say_raw(&intent_tx, conn_id, "SAY", "I FOLLOW ME");
                    }
                }
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let best = views
                        .values()
                        .filter(|o| {
                            !o.deleted
                                && o.p_id != p.p_id
                                && o.home_x == p.home_x
                                && o.home_y == p.home_y
                        })
                        .max_by(|a, b| {
                            a.lost_combat_prestige
                                .partial_cmp(&b.lost_combat_prestige)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|o| AllyUpBest {
                            p_id: o.p_id,
                            name: o.display_name.clone(),
                            quad_dist: {
                                let dx = o.x - p.x;
                                let dy = o.y - p.y;
                                dx * dx + dy * dy
                            },
                            pending_new_follower: false,
                            already_ally: o.ai_follow_p_id == p.p_id,
                        });
                    if let Some(say) = plan_ally_up(
                        st.last_leader_check_tick,
                        now,
                        p.home_x != 0 || p.home_y != 0,
                        0,
                        p.age,
                        p.ai_follow_p_id,
                        p.ai_follow_p_id > 0,
                        false,
                        p.p_id,
                        best.as_ref(),
                    ) {
                        st.last_leader_check_tick = now;
                        let _ = npc_say_raw(&intent_tx, conn_id, "SAY", &say);
                    }
                }
            }

            // Haxe L609–629 CriticalCraft prefix before isHandlingFire L634.
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_run_ladder_rung(
                    PriorityRung::CriticalCraft,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    env_winter,
                    "CriticalCraft",
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L634 isHandlingFire() before makeSharpieFood L656 (shaft 67 then Fire 82).
            // Haxe: AiBase.doTimeStuffHelper L634; isHandlingFire L1079–1111
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let (hx, hy) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                let is_best_home = npc_is_best_fire_keeper(
                    &p,
                    &player_views,
                    &profession_state,
                    content.as_ref(),
                    hx,
                    hy,
                );
                let is_best_fire = is_best_home;
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_run_is_handling_fire_mid(
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    env_winter,
                    is_best_home,
                    is_best_fire,
                ) {
                    tracing::info!(
                        conn_id,
                        p_id = p.p_id,
                        held = p.held_id,
                        detail = %d,
                        "ai_craft: isHandlingFire"
                    );
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L635 doKnifeStuff() after isHandlingFire, before pull-carrot / clothes.
            // Haxe: AiBase.doTimeStuffHelper L635; doKnifeStuff L876
            if !acted {
                let tiles = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_scan_cached_rw(st, world.as_ref(), content.as_ref(), p.x, p.y, 40)
                };
                let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
                inp.held_uses = p.held_uses.max(1);
                inp.food_store = p.food;
                inp.is_moving = p.moving;
                inp.target_reachable = true;
                inp.content = Some(content.clone());
                let r = knife_stuff_profession_scan_tick(&tiles, &inp);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_apply_scan_tick(
                    r,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    "doKnifeStuff",
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L651 shortCraft(0, 400, 10) pull carrots before isPickingupCloths.
            // Haxe: AiBase.doTimeStuffHelper L651
            if !acted {
                let tiles = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_scan_cached_rw(st, world.as_ref(), content.as_ref(), p.x, p.y, 40)
                };
                let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
                inp.held_uses = p.held_uses.max(1);
                inp.food_store = p.food;
                inp.is_moving = p.moving;
                inp.has_carrot_seeds = has_carrot_seeds_from_scan(&tiles);
                inp.target_reachable = true;
                inp.content = Some(content.clone());
                let r = pull_carrot_row_profession_scan_tick(&tiles, &inp);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_apply_scan_tick(
                    r,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    "pull_carrot_row",
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L652 isPickingupCloths() before makeSharpieFood L656.
            // Haxe: AiBase.doTimeStuffHelper L652; isPickingupCloths L8723–8752
            if !acted {
                let class_u8 = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_prestige_class_u8(st.prestige_class)
                };
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_run_is_pickingup_cloths(
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    class_u8,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L653 handleTemperature() after isPickingupCloths, before isHunting.
            // Haxe: AiBase.doTimeStuffHelper L653
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_run_handle_temperature(
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    craft_graph.as_ref(),
                    env_winter,
                    conn_id,
                    &p,
                    st,
                    tick,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L654: doFeedLambsAndCalfs(1) after handleTemperature, before isHunting.
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                let step = feed_lambs_ladder_step(&npc_sticky_from_state(st, &p));
                if let Some((k, d, ms)) = npc_run_ladder_step(
                    step,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    env_winter,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L655: doStuff && age > 14 && isHunting() before makeSharpieFood.
            // Haxe: AiBase.doTimeStuffHelper L655; isHunting L5967
            if !acted && p.age > HUNTING_MID_MIN_AGE {
                let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                let animals_g = animals.read().ok();
                let views_g = player_views.read().ok();
                let do_stuff = if let Some(views) = views_g.as_ref() {
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input_ex(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                        &nearby,
                    );
                    fill_live_sensors(&input).sensors.do_stuff
                } else {
                    compute_do_stuff(is_superbad_temp(p.heat), 10000.0)
                };
                if do_stuff {
                    let tiles = {
                        let st = profession_state.entry(conn_id).or_default();
                        npc_scan_cached_rw(st, world.as_ref(), content.as_ref(), p.x, p.y, 40)
                    };
                    let (home_x, home_y) =
                        peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    let mut inp = ProfessionScanInput::basic(p.x, p.y, p.held_id);
                    inp.home_x = home_x;
                    inp.home_y = home_y;
                    inp.held_uses = p.held_uses.max(1);
                    inp.food_store = p.food;
                    inp.age = p.age;
                    inp.is_moving = p.moving;
                    inp.target_reachable = true;
                    inp.content = Some(content.clone());
                    {
                        let st = profession_state.entry(conn_id).or_default();
                        inp.profession_is_sticky =
                            st.hunter_rt.is_last_hunter || st.hunter_rt.is_assigned_hunter;
                    }
                    let r = {
                        let st = profession_state.entry(conn_id).or_default();
                        hunting_profession_scan_tick(
                            &tiles,
                            &inp,
                            "MID_PRIORITY_TASKS",
                            &mut st.hunter_rt,
                        )
                    };
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
                    if let Some((k, d, ms)) = npc_apply_scan_tick(
                        r,
                        &intent_tx,
                        &w,
                        content.as_ref(),
                        st,
                        conn_id,
                        &p,
                        "isHunting",
                    ) {
                        kind = k;
                        detail = d;
                        game_ms = ms;
                        acted = true;
                    }
                }
            }

            // Haxe L656: makeSharpieFood(5) after isMoving return (GetOrCraft 34 / craft 39).
            // Haxe: AiBase.doTimeStuffHelper L656; makeSharpieFood L4096–4118
            if !acted {
                let is_smith = matches!(profession, CraftProfession::Smith);
                let tiles = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_scan_cached_rw(st, world.as_ref(), content.as_ref(), p.x, p.y, 40)
                };
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_try_sharpie_food(
                    &intent_tx,
                    world.as_ref(),
                    content.as_ref(),
                    craft_graph.as_ref(),
                    st,
                    conn_id,
                    &p,
                    tick,
                    is_smith,
                    MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE,
                    &tiles,
                    npc_client_move_seq(p.done_moving_seq),
                ) {
                    tracing::info!(
                        conn_id,
                        p_id = p.p_id,
                        held = p.held_id,
                        x = p.x,
                        y = p.y,
                        detail = %d,
                        "ai_craft: makeSharpieFood(5)"
                    );
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // Haxe L657–659: isHandlingGraves, fillBucketIfNeeded, shortCraft(139, 2832, 20)
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                let sticky = npc_sticky_from_state(st, &p);
                let graves = ProfessionLadderStep {
                    kind: ProfessionScanKind::HandlingGraves,
                    rung_label: "MID_PRIORITY_TASKS",
                    farm_job: None,
                    farm_has_profession: false,
                    is_assigned_job: sticky.grave_keeper_assigned || sticky.grave_keeper_last,
                    profession_is_sticky: sticky.grave_keeper_assigned || sticky.grave_keeper_last,
                };
                if let Some((k, d, ms)) = npc_run_ladder_step(
                    graves,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    env_winter,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }
            if !acted {
                let env_winter = env_view
                    .read()
                    .ok()
                    .map(|e| e.is_winter())
                    .unwrap_or(false);
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                let sticky = npc_sticky_from_state(st, &p);
                let bucket = ProfessionLadderStep {
                    kind: ProfessionScanKind::Farm,
                    rung_label: "FILL_BUCKET",
                    farm_job: Some(FarmProfession::WaterBringer),
                    farm_has_profession: true,
                    is_assigned_job: false,
                    profession_is_sticky: sticky.farm_assigned
                        == Some(FarmProfession::WaterBringer)
                        || sticky.farm_last == Some(FarmProfession::WaterBringer),
                };
                if let Some((k, d, ms)) = npc_run_ladder_step(
                    bucket,
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    st,
                    conn_id,
                    &p,
                    env_winter,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }
            if !acted {
                let w = world.read().unwrap();
                let st = profession_state.entry(conn_id).or_default();
                if let Some((k, d, ms)) = npc_run_skewer_tomato_sprout(
                    &intent_tx,
                    &w,
                    content.as_ref(),
                    craft_graph.as_ref(),
                    st,
                    conn_id,
                    &p,
                    tick,
                ) {
                    kind = k;
                    detail = d;
                    game_ms = ms;
                    acted = true;
                }
            }

            // --- 2a2. Sticky craft queue drain (AI-JOB-DEFER) ---
            // Haxe: doTimeStuffHelper ~667–680 itemToCraft continue then craftingTasks.shift
            // before clothing / assigned job. Player path: apply_sticky_craft_queue_tick.
            if !acted {
                let has_queue = {
                    let st = profession_state.entry(conn_id).or_default();
                    st.craft_rt.should_continue_unfinished()
                        || !st.craft_rt.crafting_tasks.is_empty()
                };
                if has_queue {
                    let (home_x, home_y) =
                        peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    let scan_r = craft_radius.min(60).max(8);
                    let peer_blocked_by_ai = {
                        let st = profession_state.entry(conn_id).or_default();
                        npc_blocked_by_ai_for_think(
                            &blocked_by_ai,
                            &craft_progress,
                            conn_id,
                            &p,
                            st,
                        )
                    };
                    let tiles = {
                        let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                        let raw = npc_scan_for_craft_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
                            home_x,
                            home_y,
                            scan_r,
                        );
                        let filters =
                            path_filters_from_player(&st.path_reach, &peer_blocked_by_ai);
                        apply_path_filters_to_tiles(&raw, &filters)
                    };
                    let blocked = {
                        let st = profession_state.entry(conn_id).or_default();
                        st.path_reach.blocked_coords(Some(&peer_blocked_by_ai))
                    };
                    let is_smith = matches!(profession, CraftProfession::Smith);
                    let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                    let mut choice = select_runtime_sticky_craft_for_tick(&mut st.craft_rt);
                    // Remaining queue after first shift (Haxe for i in 0...length).
                    let mut extra_left = st.craft_rt.crafting_tasks.len();
                    while !matches!(choice, StickyCraftTickChoice::None) {
                        let Some(product_id) = choice.product_id() else {
                            break;
                        };
                        // Reed Skirt is the clothing block (reachable reed USE, then
                        // isDropingItem DROP, switchCloths SELF). Generic continue
                        // DropAt on the feet tile while holding rope 59 places it;
                        // the next think picks it up, so 128 never stays in hand.
                        // Haxe: craftClothIfNeeded L4502; isDropingItem L8456; switchCloths L8709
                        if product_id == 128 {
                            if matches!(choice, StickyCraftTickChoice::Continue { .. }) {
                                st.craft_rt.item.product_id = 0;
                                st.craft_rt.item.clear_trans();
                            }
                            if extra_left == 0 {
                                break;
                            }
                            extra_left -= 1;
                            choice = match st.craft_rt.take_next_crafting_task() {
                                Some(id) => StickyCraftTickChoice::FromQueue { product_id: id },
                                None => StickyCraftTickChoice::None,
                            };
                            continue;
                        }
                        let intent = npc_expand_craft_product(
                            &world.read().unwrap(),
                            &tiles,
                            p.x,
                            p.y,
                            p.held_id,
                            p.moving,
                            home_x,
                            home_y,
                            content.as_ref(),
                            craft_graph.as_ref(),
                            &mut st.craft_rt,
                            &blocked,
                            is_smith,
                            tick,
                            product_id,
                        );
                        let committed = if !npc_craft_expand_progress(&intent, p.moving) {
                            false
                        } else {
                            let w = world.read().unwrap();
                            npc_commit_craft_live(
                                &intent,
                                &intent_tx,
                                &w,
                                content.as_ref(),
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                p.food,
                                p.moving,
                                &mut kind,
                                &mut detail,
                                &mut game_ms,
                                npc_client_move_seq(p.done_moving_seq),
                            )
                        };
                        if committed {
                            acted = true;
                            break;
                        }
                        requeue_runtime_task_on_fail(&mut st.craft_rt, choice);
                        if extra_left == 0 {
                            break;
                        }
                        extra_left -= 1;
                        choice = match st.craft_rt.take_next_crafting_task() {
                            Some(id) => StickyCraftTickChoice::FromQueue { product_id: id },
                            None => StickyCraftTickChoice::None,
                        };
                    }
                }
            }

            // --- 2a3. Clothing craft bands (AI-CLOTHING-CRAFT) ---
            // Haxe: high then medium if age>10 then low if age>30 / assigned TAILOR
            // Runs after food pickup/make; hungry with nearby food already `acted`.
            // Haxe switchCloths / isPickingupCloths L8709–8723 skip age < MinAgeToEat.
            if !acted && p.age >= MIN_AGE_TO_EAT {
                let mut rag = [false; 6];
                for i in 0..6 {
                    let id = p.clothing[i];
                    if id > 0 {
                        if let Some(d) = content.get(id) {
                            let n = d.name.to_ascii_uppercase();
                            let desc = d.description.to_ascii_uppercase();
                            rag[i] = n.contains("RAG ") || desc.contains("RAG ");
                        }
                    }
                }
                // Haxe `myPlayer.getColor()` from person object race (not hardcoded 0/Brown).
                let color = person_color_from_race(
                    content.person_color(content.resolve_base_id(p.display_object_id)),
                );
                let (home_stock, has_loom) = {
                    let w = world.read().unwrap();
                    (
                        home_cloth_stock_from_world(
                            &w,
                            p.home_x,
                            p.home_y,
                            HOME_CLOTH_COUNT_RADIUS,
                        ),
                        home_has_loom_from_world(&w, p.home_x, p.home_y, HOME_LOOM_RADIUS),
                    )
                };
                let (home_x, home_y) =
                    peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                let npc_last_tailor = p.is_last_tailor
                    || profession_state
                        .get(&conn_id)
                        .map(|s| s.last_is_tailor)
                        .unwrap_or(false);
                let tailor_count = {
                    let views = player_views.read().unwrap();
                    let rows: Vec<NpcProfessionPeerRow> = views
                        .values()
                        .map(|snap| {
                            let (hx, hy) = peer_home_coords(
                                Some((snap.home_x, snap.home_y)),
                                snap.x,
                                snap.y,
                            );
                            let pst = profession_state.get(&snap.conn_id);
                            NpcProfessionPeerRow {
                                conn_id: snap.conn_id,
                                home_x: hx,
                                home_y: hy,
                                age: snap.age,
                                food_store: snap.food,
                                deleted: snap.deleted,
                                has_player_to_follow: snap.ai_follow_p_id != 0,
                                is_wounded: peer_is_wounded_from_held_ex(
                                    is_wound_object(content.as_ref(), snap.held_id),
                                    snap.is_hidden_wound,
                                ),
                                last_is_tailor: snap.is_last_tailor
                                    || pst.map(|s| s.last_is_tailor).unwrap_or(false),
                                ..NpcProfessionPeerRow::default()
                            }
                        })
                        .collect();
                    count_tailor_profession_from_rows(
                        &rows,
                        home_x,
                        home_y,
                        MIN_AGE_TO_EAT,
                        MAX_AGE,
                    )
                };
                let was_idle = if npc_last_tailor
                    || p.is_last_smith
                    || p.is_last_baker
                    || p.is_last_potter
                    || p.is_last_shepherd
                    || p.is_last_farm
                    || p.is_last_fire_food
                {
                    0.0
                } else {
                    1.0
                };
                let assigned_tailor = p.is_assigned_tailor || npc_last_tailor;
                let has_tailor = has_or_become_tailor(
                    npc_last_tailor,
                    1,
                    tailor_count as f32,
                    was_idle,
                ) || (assigned_tailor
                    && has_or_become_tailor(
                        npc_last_tailor,
                        100,
                        tailor_count as f32,
                        was_idle,
                    ));
                let cinp = ClothingCraftInput {
                    color,
                    clothing_ids: &p.clothing,
                    rag: &rag,
                    age: p.age,
                    has_tailor,
                    bow_old_enough: is_old_enough_for_bow(
                        p.age,
                        content
                            .get(BOW_AND_ARROW)
                            .map(|d| d.min_pickup_age as f32)
                            .unwrap_or(0.0),
                    ),
                    held_id: p.held_id,
                    quiver_can_add: quiver_can_add_from_slots(&p.clothing, &p.clothing_uses),
                    home_stock,
                    // Haxe ObjectData.male from displayed person id (Female001=19).
                    female: content
                        .get(p.display_object_id)
                        .map(|o| !o.male)
                        .unwrap_or(true),
                    has_loom,
                    assigned_tailor,
                };
                if let Some(plan) = plan_clothing_craft_tick(&cinp) {
                    if has_tailor
                        && plan_high_priority_clothing(&cinp).is_none()
                        && !is_fill_up_quiver_plan(plan)
                    {
                        profession_state
                            .entry(conn_id)
                            .or_default()
                            .last_is_tailor = true;
                    }
                    let scan_r = if is_fill_up_quiver_plan(plan) {
                        fill_up_quiver_search_radius(
                            p.age,
                            tailor_count,
                            npc_last_tailor,
                        )
                        .max(8)
                    } else {
                        // Haxe L672 itemToCraft.maxSearchRadius = 60 before
                        // craftHighPriorityClothing. craft_radius (40) missed
                        // home-square 58+58 / 124 just outside the player scan.
                        NPC_GET_OR_CRAFT_SEARCH_RADIUS
                    };
                    let peer_blocked_by_ai = {
                        let st = profession_state.entry(conn_id).or_default();
                        npc_blocked_by_ai_for_think(
                            &blocked_by_ai,
                            &craft_progress,
                            conn_id,
                            &p,
                            st,
                        )
                    };
                    let tiles = {
                        let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                        let raw = npc_scan_for_craft_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
                            home_x,
                            home_y,
                            scan_r,
                        );
                        let filters =
                            path_filters_from_player(&st.path_reach, &peer_blocked_by_ai);
                        apply_path_filters_to_tiles(&raw, &filters)
                    };
                    let blocked = {
                        let st = profession_state.entry(conn_id).or_default();
                        st.path_reach.blocked_coords(Some(&peer_blocked_by_ai))
                    };
                    let is_smith = matches!(profession, CraftProfession::Smith);
                    let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                    let old_max_r = if is_fill_up_quiver_plan(plan) {
                        let old = st.craft_rt.item.max_search_radius;
                        st.craft_rt.item.max_search_radius = scan_r;
                        Some(old)
                    } else {
                        None
                    };
                    let intent = match plan {
                        ClothingCraftPlan::SelfClothing { slot } => {
                            ShortCraftLiveIntent::SelfClothing { slot }
                        }
                        ClothingCraftPlan::GetItemThenGetOrCraft {
                            get_id,
                            fallback_id,
                        } => {
                            if tiles.iter().any(|t| t.parent_id == get_id) {
                                npc_expand_clothing_craft_intent(
                                    &world.read().unwrap(),
                                    &tiles,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    home_x,
                                    home_y,
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    &mut st.craft_rt,
                                    &blocked,
                                    is_smith,
                                    tick,
                                    ShortCraftLiveIntent::SeekOrCraft {
                                        actor: get_id,
                                        craft_if_needed: false,
                                    },
                                )
                            } else {
                                npc_expand_clothing_craft_product(
                                    &world.read().unwrap(),
                                    &tiles,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    home_x,
                                    home_y,
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    &mut st.craft_rt,
                                    &blocked,
                                    is_smith,
                                    tick,
                                    fallback_id,
                                )
                            }
                        }
                        ClothingCraftPlan::GetOrCraft {
                            object_id,
                            craft_if_needed,
                        } => {
                            if craft_if_needed {
                                npc_expand_clothing_craft_product(
                                    &world.read().unwrap(),
                                    &tiles,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    home_x,
                                    home_y,
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    &mut st.craft_rt,
                                    &blocked,
                                    is_smith,
                                    tick,
                                    object_id,
                                )
                            } else {
                                npc_expand_clothing_craft_intent(
                                    &world.read().unwrap(),
                                    &tiles,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    home_x,
                                    home_y,
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    &mut st.craft_rt,
                                    &blocked,
                                    is_smith,
                                    tick,
                                    ShortCraftLiveIntent::SeekOrCraft {
                                        actor: object_id,
                                        craft_if_needed: false,
                                    },
                                )
                            }
                        }
                        ClothingCraftPlan::CraftItem(object_id) => {
                            // Existing cloth within 60 of the player or home: dropTarget,
                            // then isDropingItem DROP, then switchCloths SELF.
                            // Skip a tile a walk cannot reach (snow-grey between).
                            // Haxe: GetOrCraftItem L6196; isDropingItem L8456; switchCloths L8709
                            let existing = world.read().ok().and_then(|w| {
                                npc_approachable_cloth_xy(
                                    &w,
                                    content.as_ref(),
                                    p.x,
                                    p.y,
                                    home_x,
                                    home_y,
                                    object_id,
                                    NPC_GET_OR_CRAFT_SEARCH_RADIUS,
                                    &blocked,
                                )
                            });
                            if let Some((x, y)) = existing {
                                ShortCraftLiveIntent::DropAt { x, y }
                            } else if object_id == 128 {
                                let mut local_tiles = tiles.clone();
                                let mut local_blocked = blocked.clone();
                                let mut chosen = ShortCraftLiveIntent::None;
                                let held_rope = content.resolve_base_id(p.held_id) == 59;
                                for _ in 0..5 {
                                    chosen = world
                                        .read()
                                        .ok()
                                        .and_then(|w| {
                                            npc_reed_skirt_direct(
                                                &w,
                                                content.as_ref(),
                                                p.x,
                                                p.y,
                                                home_x,
                                                home_y,
                                                content.resolve_base_id(p.held_id),
                                                NPC_GET_OR_CRAFT_SEARCH_RADIUS,
                                                &local_blocked,
                                            )
                                        })
                                        .unwrap_or_else(|| {
                                            // Held rope is the actor. Fallback DropAt(feet)
                                            // places it; the next think picks it up again.
                                            if held_rope {
                                                return ShortCraftLiveIntent::None;
                                            }
                                            npc_clothing_craft_item_fallback(
                                                &world.read().unwrap(),
                                                &local_tiles,
                                                p.x,
                                                p.y,
                                                p.held_id,
                                                p.moving,
                                                home_x,
                                                home_y,
                                                content.as_ref(),
                                                craft_graph.as_ref(),
                                                &mut st.craft_rt,
                                                &local_blocked,
                                                is_smith,
                                                tick,
                                                object_id,
                                            )
                                        });
                                    let Some((tx, ty)) = npc_craft_intent_goal(&chosen) else {
                                        break;
                                    };
                                    let reachable = world.read().ok().is_some_and(|w| {
                                        npc_clothing_goal_reachable(
                                            &w,
                                            content.as_ref(),
                                            p.x,
                                            p.y,
                                            tx,
                                            ty,
                                        )
                                    });
                                    if reachable {
                                        break;
                                    }
                                    st.path_reach.add_not_reachable(tx, ty, 90.0);
                                    local_blocked.insert((tx, ty));
                                    local_tiles.retain(|t| t.x != tx || t.y != ty);
                                    chosen = ShortCraftLiveIntent::None;
                                }
                                // Haxe L7114: food in hand must leave before the rope DROP.
                                // Park the rope so the berry drop does not get picked
                                // back up before isDropingItem reaches it.
                                let w = world.read().unwrap();
                                let rewritten = npc_drop_held_before_loose_pickup(
                                    &w,
                                    content.as_ref(),
                                    p.x,
                                    p.y,
                                    home_x,
                                    home_y,
                                    p.held_id,
                                    chosen,
                                );
                                npc_park_actor_behind_food_drop(
                                    st,
                                    content.as_ref(),
                                    &w,
                                    chosen,
                                    rewritten,
                                )
                            } else {
                                npc_clothing_craft_item_fallback(
                                    &world.read().unwrap(),
                                    &tiles,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    home_x,
                                    home_y,
                                    content.as_ref(),
                                    craft_graph.as_ref(),
                                    &mut st.craft_rt,
                                    &blocked,
                                    is_smith,
                                    tick,
                                    object_id,
                                )
                            }
                        }
                    };
                    let intent = if matches!(
                        plan,
                        ClothingCraftPlan::GetOrCraft {
                            object_id: 148,
                            craft_if_needed: true
                        }
                    ) && !npc_craft_expand_progress(&intent, p.moving)
                    {
                        if let Some(ClothingCraftPlan::CraftItem(id)) =
                            plan_quiver_arrow_precursors(p.held_id, &home_stock)
                        {
                            npc_expand_clothing_craft_product(
                                &world.read().unwrap(),
                                &tiles,
                                p.x,
                                p.y,
                                p.held_id,
                                p.moving,
                                home_x,
                                home_y,
                                content.as_ref(),
                                craft_graph.as_ref(),
                                &mut st.craft_rt,
                                &blocked,
                                is_smith,
                                tick,
                                id,
                            )
                        } else {
                            intent
                        }
                    } else {
                        intent
                    };
                    let committed = {
                        let w = world.read().unwrap();
                        npc_commit_craft_live(
                            &intent,
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            st,
                            conn_id,
                            p.x,
                            p.y,
                            p.food,
                            p.moving,
                            &mut kind,
                            &mut detail,
                            &mut game_ms,
                            npc_client_move_seq(p.done_moving_seq),
                        )
                    };
                    if let Some(old) = old_max_r {
                        st.craft_rt.item.max_search_radius = old;
                    }
                    if committed {
                        acted = true;
                        detail = format!("clothing_craft {plan:?} {detail}");
                        tracing::info!(
                            conn_id,
                            p_id = p.p_id,
                            held = p.held_id,
                            detail = %detail,
                            "ai_craft: clothing"
                        );
                    } else if matches!(plan, ClothingCraftPlan::CraftItem(200)) {
                        // Haxe: craftClothIfNeeded(200) false → try Reed Skirt 128 same tick.
                        let skirt = npc_expand_clothing_craft_product(
                            &world.read().unwrap(),
                            &tiles,
                            p.x,
                            p.y,
                            p.held_id,
                            p.moving,
                            home_x,
                            home_y,
                            content.as_ref(),
                            craft_graph.as_ref(),
                            &mut st.craft_rt,
                            &blocked,
                            is_smith,
                            tick,
                            128,
                        );
                        let w = world.read().unwrap();
                        if npc_commit_craft_live(
                            &skirt,
                            &intent_tx,
                            &w,
                            content.as_ref(),
                            st,
                            conn_id,
                            p.x,
                            p.y,
                            p.food,
                            p.moving,
                            &mut kind,
                            &mut detail,
                            &mut game_ms,
                            npc_client_move_seq(p.done_moving_seq),
                        ) {
                            acted = true;
                            if detail.is_empty() || detail == "idle" {
                                detail = "clothing_craft CraftItem(128)".into();
                            }
                            tracing::info!(
                                conn_id,
                                p_id = p.p_id,
                                "ai_craft: clothing reed_skirt"
                            );
                        }
                    }
                }
            }

            // --- 2b. Profession ladder scan (NPC-CRAFT-LADDER) ---
            // Haxe: AssignedJob / AgeRotatedJob â†’ doBasicFarming/doSmithing/doBaking â†’ USE/DROP
            // Escape/food bands already handled above; only when not hungry-starving.
            // AI-FOLLOW-WALK: follow holds tick above when far from sticky target
            if !acted {
                if let Some(mut sticky) = npc_sticky_for_craft_profession(profession, p.age) {
                    // AI-JOB-SMITH-RESID: true home from PlayerSnapshot (Haxe home.tx/ty)
                    let (home_x, home_y) =
                        peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                    sticky.tailor_assigned = p.is_assigned_tailor;
                    sticky.tailor_last = p.is_last_tailor
                        || profession_state
                            .get(&conn_id)
                            .map(|s| s.last_is_tailor)
                            .unwrap_or(false);
                    // Merge live last/assigned from runtimes (Haxe lastProfession || assigned)
                    {
                        let st = profession_state.entry(conn_id).or_default();
                        let live = ProfessionStickySnapshot::from_runtimes_ex(
                            &st.farm_rt,
                            &st.smith_rt,
                            &st.baker_rt,
                            Some(&st.shepherd_rt),
                            Some(&st.pottery_rt),
                            Some(&st.fire_rt),
                            Some(&st.fire_keeper_rt),
                            Some(&st.grave_keeper_rt),
                            Some(&st.hunter_rt),
                            Some(&st.lumberjack_rt),
                            Some(&st.collector_rt),
                            Some(&st.foodserver_rt),
                            p.age,
                        );
                        if sticky.farm_assigned.is_none() {
                            sticky.farm_assigned = live.farm_assigned;
                        }
                        if sticky.farm_last.is_none() {
                            sticky.farm_last = live.farm_last;
                        }
                        sticky.smith_assigned |= live.smith_assigned;
                        sticky.smith_last |= live.smith_last;
                        sticky.baker_assigned |= live.baker_assigned;
                        sticky.baker_last |= live.baker_last;
                        sticky.pottery_assigned |= live.pottery_assigned;
                        sticky.pottery_last |= live.pottery_last;
                        sticky.shepherd_assigned |= live.shepherd_assigned;
                        sticky.shepherd_last |= live.shepherd_last;
                        sticky.fire_food_assigned |= live.fire_food_assigned;
                        sticky.fire_food_last |= live.fire_food_last;
                        sticky.fire_keeper_assigned |= live.fire_keeper_assigned;
                        sticky.fire_keeper_last |= live.fire_keeper_last;
                        sticky.hunter_assigned |= live.hunter_assigned;
                        sticky.hunter_last |= live.hunter_last;
                        sticky.lumberjack_assigned |= live.lumberjack_assigned;
                        sticky.lumberjack_last |= live.lumberjack_last;
                        sticky.collector_assigned |= live.collector_assigned;
                        sticky.collector_last |= live.collector_last;
                        sticky.foodserver_assigned |= live.foodserver_assigned;
                        sticky.foodserver_last |= live.foodserver_last;
                    }
                    let rungs = npc_think_job_rungs(&sticky);
                    let steps: Vec<_> = rungs
                        .iter()
                        .flat_map(|r| plan_profession_ladder_steps(*r, &sticky))
                        .collect();
                    let rung = rungs.first().copied().unwrap_or(PriorityRung::AgeRotatedJob);
                    let scan_r = steps
                        .iter()
                        .map(|s| match s.kind {
                            ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,
                            ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,
                            ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,
                            ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,
                            ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,
                            ProfessionScanKind::FireFood => FIRE_FOOD_HOME_RADIUS,
                            ProfessionScanKind::HandlingFire => HANDLING_FIRE_SCAN_RADIUS,
                            ProfessionScanKind::HandlingGraves => GRAVE_SEARCH_RADIUS,
                            ProfessionScanKind::Hunting => HUNTING_SHORTCRAFT_RADIUS,
                            ProfessionScanKind::CuttingWood => CUTTING_WOOD_SCAN_RADIUS,
                            ProfessionScanKind::Collecting => COLLECTING_SCAN_RADIUS,
                            ProfessionScanKind::FoodServer => STARVING_SEARCH_DIST,
                            ProfessionScanKind::Tailor => TAILOR_SCAN_RADIUS,
                        })
                        .max()
                        .unwrap_or(DEFAULT_PROFESSION_SCAN_RADIUS)
                        .min(craft_radius)
                        .max(8);
                    // Ensure sticky entry exists before peer roster (no long-lived mut borrow).
                    profession_state.entry(conn_id).or_default();
                    // PATH-REACH: filter notReachable / hostile / live blockedByAI before picks.
                    // Haxe: cleanupBlockedObjects + isObjectNotReachable (OR blockedByAI)
                    // Haxe: AiBase.RemoveBlockedByAi L199 before think
                    let peer_blocked_by_ai = {
                        let st = profession_state.entry(conn_id).or_default();
                        npc_blocked_by_ai_for_think(
                            &blocked_by_ai,
                            &craft_progress,
                            conn_id,
                            &p,
                            st,
                        )
                    };
                    let tiles = {
                        let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                        let raw = npc_scan_cached_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            home_x,
                            home_y,
                            scan_r,
                        );
                        let filters =
                            path_filters_from_player(&st.path_reach, &peer_blocked_by_ai);
                        apply_path_filters_to_tiles(&raw, &filters)
                    };
                    // AI-JOB-SMITH-RESID: multi-prof peer pop from snapshots + npc sticky
                    // Haxe: countProfession over Connection.getAis (home / wound / last)
                    let primary_kind = steps
                        .first()
                        .map(|s| s.kind)
                        .unwrap_or(ProfessionScanKind::Farm);
                    let (peer_count, farm_peer_lasts, peer_count_by_kind) = {
                        let views = player_views.read().unwrap();
                        // Union of published views + local profession_state (npc sticky lag).
                        let mut seen: HashMap<u64, ()> = HashMap::new();
                        let mut rows: Vec<NpcProfessionPeerRow> = Vec::new();
                        for (cid, snap) in views.iter() {
                            seen.insert(*cid, ());
                            let pst = profession_state.get(cid);
                            let (hx, hy) =
                                peer_home_coords(Some((snap.home_x, snap.home_y)), snap.x, snap.y);
                            let held_wound = is_wound_object(content.as_ref(), snap.held_id);
                            rows.push(NpcProfessionPeerRow {
                                conn_id: *cid,
                                home_x: hx,
                                home_y: hy,
                                age: snap.age,
                                food_store: snap.food,
                                deleted: snap.deleted,
                                has_player_to_follow: snap.ai_follow_p_id != 0,
                                is_wounded: peer_is_wounded_from_held_ex(
                                    held_wound,
                                    snap.is_hidden_wound,
                                ),
                                last_is_smith: snap.is_last_smith
                                    || pst.map(|s| s.smith_rt.is_last_smith).unwrap_or(false),
                                last_is_baker: snap.is_last_baker
                                    || pst.map(|s| s.baker_rt.is_last_baker).unwrap_or(false),
                                last_is_potter: snap.is_last_potter
                                    || pst.map(|s| s.pottery_rt.is_last_potter).unwrap_or(false),
                                last_is_shepherd: snap.is_last_shepherd
                                    || pst
                                        .map(|s| s.shepherd_rt.is_last_shepherd)
                                        .unwrap_or(false),
                                last_is_farm: snap.is_last_farm
                                    || snap.last_farm.is_some()
                                    || pst
                                        .map(|s| s.farm_rt.last_profession.is_some())
                                        .unwrap_or(false),
                                last_farm: pst
                                    .and_then(|s| s.farm_rt.last_profession)
                                    .or(snap.last_farm),
                                last_is_fire_food: snap.is_last_fire_food
                                    || pst
                                        .map(|s| s.fire_rt.is_last_fire_food)
                                        .unwrap_or(false),
                                last_is_fire_keeper: snap.is_last_fire_keeper
                                    || pst
                                        .map(|s| s.fire_keeper_rt.is_last_fire_keeper)
                                        .unwrap_or(false),
                                last_is_hunter: snap.is_last_hunter
                                    || pst.map(|s| s.hunter_rt.is_last_hunter).unwrap_or(false),
                                last_is_lumberjack: snap.is_last_lumberjack
                                    || pst
                                        .map(|s| s.lumberjack_rt.is_last_lumberjack)
                                        .unwrap_or(false),
                                last_is_collector: snap.is_last_collector
                                    || pst
                                        .map(|s| s.collector_rt.is_last_collector)
                                        .unwrap_or(false),
                                last_is_foodserver: snap.is_last_foodserver
                                    || pst
                                        .map(|s| s.foodserver_rt.is_last_foodserver)
                                        .unwrap_or(false),
                                last_is_tailor: snap.is_last_tailor
                                    || pst.map(|s| s.last_is_tailor).unwrap_or(false),
                            });
                        }
                        // Profession-state-only peers (no published view yet)
                        for (cid, pst) in profession_state.iter() {
                            if seen.contains_key(cid) {
                                continue;
                            }
                            rows.push(NpcProfessionPeerRow {
                                conn_id: *cid,
                                home_x,
                                home_y,
                                age: 20.0,
                                food_store: 5.0,
                                deleted: false,
                                has_player_to_follow: false,
                                is_wounded: false,
                                last_is_smith: pst.smith_rt.is_last_smith,
                                last_is_baker: pst.baker_rt.is_last_baker,
                                last_is_potter: pst.pottery_rt.is_last_potter,
                                last_is_shepherd: pst.shepherd_rt.is_last_shepherd,
                                last_is_farm: pst.farm_rt.last_profession.is_some(),
                                last_farm: pst.farm_rt.last_profession,
                                last_is_fire_food: pst.fire_rt.is_last_fire_food,
                                last_is_fire_keeper: pst.fire_keeper_rt.is_last_fire_keeper,
                                last_is_hunter: pst.hunter_rt.is_last_hunter,
                                last_is_lumberjack: pst.lumberjack_rt.is_last_lumberjack,
                                last_is_collector: pst.collector_rt.is_last_collector,
                                last_is_foodserver: pst.foodserver_rt.is_last_foodserver,
                                last_is_tailor: pst.last_is_tailor,
                            });
                        }
                        let peer_count = npc_peer_count_for_kind(
                            primary_kind,
                            &rows,
                            conn_id,
                            home_x,
                            home_y,
                            MIN_AGE_TO_EAT,
                            MAX_AGE,
                        );
                        let farm_peer_lasts = farm_peer_lasts_from_npc_rows(
                            &rows,
                            conn_id,
                            home_x,
                            home_y,
                            MIN_AGE_TO_EAT,
                            MAX_AGE,
                        );
                        let peer_count_by_kind = npc_peer_counts_by_kind(
                            &rows,
                            conn_id,
                            home_x,
                            home_y,
                            MIN_AGE_TO_EAT,
                            MAX_AGE,
                        );
                        (peer_count, farm_peer_lasts, peer_count_by_kind)
                    };
                    let basic_farmer_weight = basic_farmer_weight_from_runtime(
                        &profession_state
                            .get(&conn_id)
                            .expect("npc profession entry")
                            .farm_rt,
                    );
                    let fire_place_id = profession_state
                        .get(&conn_id)
                        .map(|s| s.fire_place_id)
                        .filter(|&id| id != 0)
                        .unwrap_or(p.ai_fire_place_id);
                    let fire_place_x = profession_state
                        .get(&conn_id)
                        .map(|s| s.fire_place_x)
                        .unwrap_or(p.ai_fire_place_x);
                    let fire_place_y = profession_state
                        .get(&conn_id)
                        .map(|s| s.fire_place_y)
                        .unwrap_or(p.ai_fire_place_y);
                    let (fire_obj_x, fire_obj_y) = if fire_place_id != 0 {
                        (fire_place_x, fire_place_y)
                    } else {
                        (home_x, home_y)
                    };
                    let is_best_fire_keeper_at_home = npc_is_best_fire_keeper(
                        &p,
                        &player_views,
                        &profession_state,
                        content.as_ref(),
                        home_x,
                        home_y,
                    );
                    let is_best_fire_keeper_at_fire = npc_is_best_fire_keeper(
                        &p,
                        &player_views,
                        &profession_state,
                        content.as_ref(),
                        fire_obj_x,
                        fire_obj_y,
                    );
                    let last_grave = profession_state.get(&conn_id).and_then(|s| {
                        if s.grave_keeper_rt.last_grave_id != 0 {
                            Some((
                                s.grave_keeper_rt.last_grave_id,
                                s.grave_keeper_rt.last_grave_x,
                                s.grave_keeper_rt.last_grave_y,
                            ))
                        } else {
                            None
                        }
                    });
                    let is_best_grave_keeper =
                        if let Some((gx, gy)) = npc_pick_grave_xy(&tiles, p.x, p.y, last_grave)
                        {
                            npc_is_best_grave_keeper(
                                &p,
                                &player_views,
                                &profession_state,
                                content.as_ref(),
                                gx,
                                gy,
                            )
                        } else {
                            true
                        };
                    let inp = ProfessionScanInput {
                        player_x: p.x,
                        player_y: p.y,
                        home_x,
                        home_y,
                        held_id: p.held_id,
                        held_uses: p.held_uses.max(1),
                        held_contained: p.held_contained,
                        held_contains_clay: p.held_contains_clay,
                        food_store: p.food,
                        transition_hungry_cost: scan_held_hungry_work_cost(
                            content.as_ref(),
                            p.held_id,
                            cfg.hungry_work_cost,
                        ),
                        hungry_work_cost_knob: cfg.hungry_work_cost,
                        content: Some(content.clone()),
                        has_carrot_seeds: has_carrot_seeds_from_scan(&tiles),
                        has_bean_seeds: has_bean_seeds_from_scan(&tiles),
                        is_hungry: false,
                        basic_farmer_weight,
                        hardened_row_biome: None,
                        // PATH-REACH: tiles already filtered via path_reach maps
                        target_reachable: true,
                        peer_count,
                        farm_peer_lasts,
                        was_idle: profession_state
                            .get(&conn_id)
                            .map(|s| s.was_idle)
                            .unwrap_or(0.0),
                        age: p.age,
                        profession_is_sticky: sticky.has_sticky_profession(),
                        is_assigned_job: sticky.has_assigned_job(),
                        // PREFER-SHORT-WAIT (npc usually skips when p.moving; keep field)
                        is_moving: p.moving,
                        // AI-HANDLING-FIRE: Season==Winter → Fire 82 kindling first
                        is_winter: env_view
                            .read()
                            .map(|g| g.is_winter())
                            .unwrap_or(false),
                        // Load-time objectIdArrays[455] cache (not re-scanned each tick)
                        chisel_family_extra: chisel_family_extra.clone(),
                        // Haxe AiIgnoredFloorIds — live table (empty → compiled 656/888)
                        ignored_floor_ids: cfg.ignored_floor_ids.clone(),
                        is_best_bowl_filler: npc_is_best_bowl_filler(
                            &p,
                            &player_views,
                            &profession_state,
                            content.as_ref(),
                        ),
                        is_best_fire_keeper_at_home,
                        is_best_fire_keeper_at_fire,
                        is_best_grave_keeper,
                        self_account_id: 0,
                        peer_count_by_kind: Some(peer_count_by_kind),
                        // Haxe: storeInQuiver clothingObjects (DROP-HELD-QUIVER)
                        clothing: p.clothing,
                        clothing_uses: p.clothing_uses,
                        fire_place_id,
                        fire_place_x,
                        fire_place_y,
                        feeding_cands: {
                            let views_g = player_views.read().ok();
                            views_g
                                .as_ref()
                                .map(|v| npc_starving_cands_from_views(&p, v, content.as_ref()))
                                .unwrap_or_default()
                        },
                        held_food_value: 0,
                        feeder_is_fertile: false,
                        feeder_is_smith: false,
                        feeder_p_id: p.p_id,
                        person_color: person_color_from_race(
                            content.person_color(content.resolve_base_id(p.display_object_id)),
                        ),
                        looks_female: content
                            .get(p.display_object_id)
                            .map(|o| !o.male)
                            .unwrap_or(true),
                        assigned_tailor: p.is_assigned_tailor,
                        last_is_tailor: p.is_last_tailor
                            || profession_state
                                .get(&conn_id)
                                .map(|s| s.last_is_tailor)
                                .unwrap_or(false),
                        bucket_water_source_ids: init_water_source_ids_from_content(
                            content.as_ref(),
                        )
                        .1,
                        currently_craving: p.currently_craving,
                    };
                    let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                    let mut sticky = sticky;
                    sticky.grave_keeper_assigned = st.grave_keeper_rt.is_assigned_grave_keeper;
                    sticky.grave_keeper_last = st.grave_keeper_rt.is_last_grave_keeper;
                    sticky.hunter_assigned = st.hunter_rt.is_assigned_hunter;
                    sticky.hunter_last = st.hunter_rt.is_last_hunter;
                    sticky.lumberjack_assigned = st.lumberjack_rt.is_assigned_lumberjack;
                    sticky.lumberjack_last = st.lumberjack_rt.is_last_lumberjack;
                    sticky.collector_assigned = st.collector_rt.is_assigned_collector;
                    sticky.collector_last = st.collector_rt.is_last_collector;
                    sticky.foodserver_assigned = st.foodserver_rt.is_assigned_foodserver;
                    sticky.foodserver_last = st.foodserver_rt.is_last_foodserver;
                    sticky.tailor_assigned = p.is_assigned_tailor;
                    sticky.tailor_last = st.last_is_tailor || p.is_last_tailor;
                    let rungs = npc_think_job_rungs(&sticky);
                    let mut result = ProfessionScanTickResult::none();
                    let mut rung = rung;
                    for r in rungs {
                        let rtick = ladder_profession_scan_tick(
                            r,
                            &tiles,
                            &inp,
                            &sticky,
                            &mut st.farm_task,
                            &mut st.farm_rt,
                            &mut st.smith_rt,
                            &mut st.baker_rt,
                            &mut st.baker_task,
                            &mut st.shepherd_rt,
                            &mut st.pottery_rt,
                            &mut st.fire_rt,
                            &mut st.fire_keeper_rt,
                            &mut st.grave_keeper_rt,
                            &mut st.hunter_rt,
                            &mut st.lumberjack_rt,
                            &mut st.collector_rt,
                            &mut st.foodserver_rt,
                        );
                        if rtick.had_action {
                            result = rtick;
                            rung = r;
                            break;
                        }
                    }
                    // FIRE-CRAFT-R30: itemToCraft.maxSearchRadius=30 around makeFireFood(3)
                    apply_fire_craft_search_radius_override(
                        &mut st.fire_keeper_rt,
                        &mut st.craft_rt.item.max_search_radius,
                    );
                    if st.fire_keeper_rt.fire_place_touched {
                        let map: Vec<HandlingFireMapObj> = tiles
                            .iter()
                            .filter(|t| t.parent_id != 0)
                            .map(|t| HandlingFireMapObj {
                                parent_id: t.parent_id,
                                x: t.x,
                                y: t.y,
                            })
                            .collect();
                        let resolved = resolve_fire_place(
                            &map,
                            home_x,
                            home_y,
                            GET_CLOSE_FIRE_MAXDIST,
                            None,
                        );
                        let (id, x, y) = commit_fire_place(
                            resolved,
                            st.fire_keeper_rt.give_up_fire_place,
                        );
                        st.fire_place_id = id;
                        st.fire_place_x = x;
                        st.fire_place_y = y;
                        st.fire_keeper_rt.fire_place_touched = false;
                        st.fire_keeper_rt.give_up_fire_place = false;
                    }
                    if result.had_action
                        && steps.first().map(|s| s.kind) == Some(ProfessionScanKind::Tailor)
                    {
                        st.last_is_tailor = true;
                    }
                    if result.had_action {
                        match result.intent {
                            ShortCraftLiveIntent::UseAt { x, y, .. }
                            | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. }
                            | ShortCraftLiveIntent::DropAt { x, y } => {
                                let committed = {
                                    let w = world.read().unwrap();
                                    npc_commit_craft_live(
                                        &result.intent,
                                        &intent_tx,
                                        &w,
                                        &content,
                                        st,
                                        conn_id,
                                        p.x,
                                        p.y,
                                        p.food,
                                        p.moving,
                                        &mut kind,
                                        &mut detail,
                                        &mut game_ms,
                                        npc_client_move_seq(p.done_moving_seq),
                                    )
                                };
                                if committed {
                                    acted = true;
                                } else {
                                    let w = world.read().unwrap();
                                    npc_mark_goto_path_fail(
                                        &mut st.path_reach,
                                        &w,
                                        &content,
                                        p.x,
                                        p.y,
                                        x,
                                        y,
                                        p.food,
                                        st.food_goto.did_not_reach_food,
                                    );
                                }
                            }
                            ShortCraftLiveIntent::StageRemoveFromContainer {
                                x,
                                y,
                                expected_parent,
                            } => {
                                st.remove_from_container = stage_remove_item_from_container(
                                    x,
                                    y,
                                    expected_parent,
                                    true,
                                    false,
                                );
                                kind = NpcActivityKind::Craft;
                                detail = format!(
                                    "stage_remove parent={expected_parent} @{},{} rung={}",
                                    x,
                                    y,
                                    rung.as_label()
                                );
                                game_ms = 200;
                                acted = true;
                            }
                            ShortCraftLiveIntent::Remv { x, y } => {
                                let payload = format!("{x} {y}");
                                if npc_say_raw(&intent_tx, conn_id, "REMV", &payload) {
                                    st.remove_from_container = None;
                                    kind = NpcActivityKind::Craft;
                                    detail = format!("prof_remv @{},{} rung={}", x, y, rung.as_label());
                                    game_ms = 500;
                                    acted = true;
                                }
                            }
                            ShortCraftLiveIntent::SeekOrCraft { .. }
                            | ShortCraftLiveIntent::SeekGroundActor { .. }
                            | ShortCraftLiveIntent::CraftItem { .. } => {
                                // AI-CRAFT-NPC-ENQUEUE: multi-step GetOrCraft + craftItem expand
                                // with path-reach CraftScanFilters (hostile/unreachable/blockedByAI).
                                // Haxe: AiBase.GetOrCraftItem â†’ craftItem â†’ useTarget / dropTarget
                                // Residuals closed: live pile_id (getPileObjId), ScanTile.num_slots,
                                // ignoreFullPiles full multi-use tiles, peer blockedByAI merge.
                                let staging = result.intent;
                                // ScanTile.num_slots from ObjectDef at scan_world_radius
                                // Haxe: objectData.numSlots empty-hand container gate
                                let goc_objs = get_or_craft_objs_from_scan(&tiles, None);
                                // Haxe: ignoreFullPiles + numberOfUses >= numUses
                                let full_piles = full_pile_tiles_from_scan(&tiles);
                                // Haxe: addObjectsForCrafting skip nonempty containers
                                let nonempty_boxes = nonempty_container_tiles_from_scan(&tiles);
                                // Haxe: isObjectNotReachable ORs blockedByAI
                                // Haxe: AiBase.RemoveBlockedByAi L199 before think
                                let peer_blocked_by_ai = npc_blocked_by_ai_for_think(
                                    &blocked_by_ai,
                                    &craft_progress,
                                    conn_id,
                                    &p,
                                    st,
                                );
                                let blocked =
                                    st.path_reach.blocked_coords(Some(&peer_blocked_by_ai));
                                let empty_drop = {
                                    let w = world.read().unwrap();
                                    Some(npc_empty_drop_xy(&w, p.x, p.y))
                                };
                                let is_smith = matches!(profession, CraftProfession::Smith);
                                // Haxe: ObjectData.getPileObjId via self+self transition
                                let content_ref = content.as_ref();
                                // Haxe: ServerSettings.WaterSourceIds / BucketWaterSourceIds
                                let (water_ids, bucket_ids) =
                                    init_water_source_ids_from_content(content_ref);
                                let opts = CraftLiveExpandOpts {
                                    home: Some((home_x, home_y)),
                                    is_or_can_smith: is_smith,
                                    now_sec: tick as f64 * 0.2,
                                    water_source_ids: water_ids,
                                    bucket_water_source_ids: bucket_ids,
                                    is_hidden_wound: p.is_hidden_wound,
                                    ..Default::default()
                                }
                                .with_content_craft_gates(content_ref);
                                let pile_id_for = |id: i32| {
                                    let p = pile_obj_id_from_content(content_ref, id);
                                    if p > 0 {
                                        p
                                    } else {
                                        0
                                    }
                                };
                                let resolved = npc_enqueue_get_or_craft_ex(
                                    staging,
                                    &goc_objs,
                                    p.x,
                                    p.y,
                                    p.held_id,
                                    p.moving,
                                    empty_drop,
                                    Some(craft_graph.as_ref()),
                                    &opts,
                                    Some(&mut st.craft_rt),
                                    &pile_id_for,
                                    Some(&blocked),
                                    Some(&full_piles),
                                    Some(&nonempty_boxes),
                                );
                                match resolved {
                                    ShortCraftLiveIntent::Wait
                                    | ShortCraftLiveIntent::UseAt { .. }
                                    | ShortCraftLiveIntent::UseOnEmptyGround { .. }
                                    | ShortCraftLiveIntent::DropAt { .. }
                                    | ShortCraftLiveIntent::Goto { .. }
                                    | ShortCraftLiveIntent::PickupNearForge { .. }
                                    | ShortCraftLiveIntent::GotoForge { .. }
                                    | ShortCraftLiveIntent::SelfClothing { .. }
                                    | ShortCraftLiveIntent::Kill { .. } => {
                                        let fail_xy = match resolved {
                                            ShortCraftLiveIntent::UseAt { x, y, .. }
                                            | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. }
                                            | ShortCraftLiveIntent::DropAt { x, y }
                                            | ShortCraftLiveIntent::Goto { x, y }
                                            | ShortCraftLiveIntent::PickupNearForge { x, y, .. } => {
                                                Some((x, y))
                                            }
                                            ShortCraftLiveIntent::GotoForge {
                                                forge_x,
                                                forge_y,
                                                ..
                                            } => Some((forge_x, forge_y)),
                                            _ => None,
                                        };
                                        let committed = {
                                            let w = world.read().unwrap();
                                            npc_commit_craft_live(
                                                &resolved,
                                                &intent_tx,
                                                &w,
                                                &content,
                                                st,
                                                conn_id,
                                                p.x,
                                                p.y,
                                                p.food,
                                                p.moving,
                                                &mut kind,
                                                &mut detail,
                                                &mut game_ms,
                                                npc_client_move_seq(p.done_moving_seq),
                                            )
                                        };
                                        if committed {
                                            acted = true;
                                        } else if let Some((x, y)) = fail_xy {
                                            let w = world.read().unwrap();
                                            npc_mark_goto_path_fail(
                                                &mut st.path_reach,
                                                &w,
                                                &content,
                                                p.x,
                                                p.y,
                                                x,
                                                y,
                                                p.food,
                                                st.food_goto.did_not_reach_food,
                                            );
                                        }
                                    }
                                    // Residual SeekOrCraft / CraftItem / None â†’ fall through
                                    // to craft_value / explore
                                    _ => {}
                                }
                            }
                            ShortCraftLiveIntent::GotoForge {
                                forge_x,
                                forge_y,
                                ..
                            } => {
                                let dist =
                                    (forge_x - p.x).abs().max((forge_y - p.y).abs());
                                if dist > 1 {
                                    let walked = {
                                        let w = world.read().unwrap();
                                        let st = profession_state.entry(conn_id).or_default();
                                        npc_try_walk_to(
                                            &intent_tx,
                                            &w,
                                            &content,
                                            conn_id,
                                            p.x,
                                            p.y,
                                            forge_x,
                                            forge_y,
                                            p.food,
                                            st.food_goto.did_not_reach_food,
                                            st.animal_path,
                                            npc_client_move_seq(p.done_moving_seq),
                                        )
                                    };
                                    if walked {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_forge_walk @{},{}",
                                            forge_x, forge_y
                                        );
                                        game_ms = 250;
                                        acted = true;
                                    }
                                }
                            }
                            // Haxe: dropHeldObject gotoObj dropOnStart (DROP-HELD-LIVE)
                            ShortCraftLiveIntent::Goto { x, y } => {
                                let dist = (x - p.x).abs().max((y - p.y).abs());
                                if dist > 1 {
                                    let walked = {
                                        let w = world.read().unwrap();
                                        let st = profession_state.entry(conn_id).or_default();
                                        npc_try_walk_to(
                                            &intent_tx,
                                            &w,
                                            &content,
                                            conn_id,
                                            p.x,
                                            p.y,
                                            x,
                                            y,
                                            p.food,
                                            st.food_goto.did_not_reach_food,
                                            st.animal_path,
                                            npc_client_move_seq(p.done_moving_seq),
                                        )
                                    };
                                    if walked {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_drop_goto @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 250;
                                        acted = true;
                                    }
                                }
                            }
                            ShortCraftLiveIntent::Kill {
                                target_p_id,
                                x,
                                y,
                            } => {
                                if npc_say_raw(
                                    &intent_tx,
                                    conn_id,
                                    "KILL",
                                    &format!("{x} {y} {target_p_id}"),
                                ) {
                                    st.food_goto.did_not_reach_food = 0.0;
                                    kind = NpcActivityKind::Combat;
                                    detail = format!(
                                        "prof_kill target={target_p_id} @{},{} rung={}",
                                        x,
                                        y,
                                        rung.as_label()
                                    );
                                    game_ms = 400;
                                    acted = true;
                                }
                            }
                            // Haxe: storeInQuiver â†’ self(0,0,5) (DROP-HELD-LIVE)
                            ShortCraftLiveIntent::SelfClothing { slot } => {
                                if intent_tx
                                    .try_send(NetIntent::Raw {
                                        conn_id,
                                        tag: "SELF".into(),
                                        payload: self_clothing_raw_payload(slot),
                                    })
                                    .is_ok()
                                {
                                    kind = NpcActivityKind::Craft;
                                    detail = format!(
                                        "prof_self_clothing slot={slot} rung={}",
                                        rung.as_label()
                                    );
                                    game_ms = 400;
                                    acted = true;
                                }
                            }
                            // Haxe: isMoving / dropHeld return true â€” hold tick (PREFER-SHORT-WAIT)
                            ShortCraftLiveIntent::Wait => {
                                kind = NpcActivityKind::Craft;
                                detail = format!(
                                    "prof_wait_busy_moving rung={}",
                                    rung.as_label()
                                );
                                game_ms = 200;
                                acted = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Haxe doTimeStuffHelper L840–873: go home, then drop held, then idle wait + say.
            if !acted {
                let (home_x, home_y) = peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
                let fire = {
                    let st = profession_state.get(&conn_id);
                    st.and_then(|s| {
                        if s.fire_place_id != 0 {
                            Some((s.fire_place_x, s.fire_place_y))
                        } else {
                            None
                        }
                    })
                };
                let (tx, ty) = go_home_move_target(home_x, home_y, fire);
                let dx = p.x - tx;
                let dy = p.y - ty;
                let quad = (dx * dx + dy * dy) as f32;
                if should_path_to_home(quad, 4) {
                    let (gx, gy) = go_home_goal_xy(tx, ty, (tick as u32) ^ (conn_id as u32));
                    let walked = {
                        let w = world.read().unwrap();
                        let st = profession_state.entry(conn_id).or_default();
                        npc_try_walk_to(
                            &intent_tx,
                            &w,
                            &content,
                            conn_id,
                            p.x,
                            p.y,
                            gx,
                            gy,
                            p.food,
                            st.food_goto.did_not_reach_food,
                            st.animal_path,
                            npc_client_move_seq(p.done_moving_seq),
                        )
                    };
                    if walked || p.moving {
                        kind = NpcActivityKind::Move;
                        detail = format!("idle_home @{gx},{gy}");
                        game_ms = 250;
                        acted = true;
                    }
                }
            }
            if !acted && idle_should_drop_held(p.held_id, p.is_hidden_wound) {
                if npc_drop_at(&intent_tx, conn_id, p.x, p.y, None) {
                    kind = NpcActivityKind::Craft;
                    detail = format!("idle_drop_held={}", p.held_id);
                    game_ms = 400;
                    acted = true;
                }
            }
            if !acted {
                let st = profession_state.entry(conn_id).or_default();
                st.think_time_sec += 2.5;
                st.was_idle += 1.0;
                let rand = ((tick.wrapping_mul(1103515245).wrapping_add(conn_id)) % 1000)
                    as f32
                    / 1000.0;
                if let Some(line) = idle_say_line(p.age, MIN_AGE_TO_EAT, rand) {
                    let _ = npc_say_raw(&intent_tx, conn_id, "SAY", line);
                }
                kind = NpcActivityKind::Explore;
                detail = "idle_wait".into();
                game_ms = 2500;
                acted = true;
            }



            if !acted {
                kind = NpcActivityKind::Error;
                detail = "no_action".into();
            }

            tracker.note_action(&detail);
            // Stuck nudge only when this think did **not** already commit a MOVE.
            // Double-MOVE in one think (walk + stuck) was inflating accepts ~2Ã—
            // and cancelling multi-step paths mid-commit.
            let already_moved = detail.contains("walk")
                || detail.starts_with("explore")
                || detail.contains("_walk")
                || detail.starts_with("prof_goc_walk")
                || detail.starts_with("prof_walk")
                || detail.starts_with("smart_drop_walk")
                || detail.starts_with("follow");
            if tracker.is_stuck() {
                let why = if tracker.position_cycle() {
                    "pos_cycle"
                } else if tracker.craft_loop() {
                    "craft_loop"
                } else if tracker.same_pos_count >= 12 {
                    "pos_stuck"
                } else {
                    "action_spam"
                };
                log_ev(
                    &activity,
                    conn_id,
                    &p,
                    if tracker.position_cycle() || tracker.craft_loop() {
                        NpcActivityKind::StuckCycle
                    } else {
                        NpcActivityKind::Stuck
                    },
                    timer.elapsed().as_micros() as u32,
                    0,
                    format!(
                        "{} detail={} crafts={:?}",
                        why, detail, tracker.craft_ring
                    ),
                );
                if !already_moved {
                    // Nudge only when idle/stuck without a walk this think.
                    let _ = intent_tx.try_send(NetIntent::Move {
                        conn_id,
                        xs: p.x,
                        ys: p.y,
                        deltas: vec![(1, 0), (0, 1)],
                        seq: Some(npc_client_move_seq(p.done_moving_seq)),
                    });
                }
                tracker.same_pos_count = 0;
                tracker.same_action_count = 0;
            }

            let cpu = timer.elapsed().as_micros() as u32;
            log_ev(&activity, conn_id, &p, kind, cpu, game_ms, detail);
            debug!(conn_id, ?kind, "npc think");

            let (scan_us, scan_calls, scan_hits) = profession_state
                .get(&conn_id)
                .map(|st| (st.scan_us_acc, st.scan_calls as u64, st.scan_hits as u64))
                .unwrap_or((0, 0, 0));
            counters.record_ai_think_parts(cpu as u64, scan_us, scan_calls, scan_hits);

            // PATH-REACH-MERGE: push NPC path maps into player_views for tick_vitals absorb
            if let Some(st) = profession_state.get(&conn_id) {
                push_npc_path_reach_to_views(&player_views, conn_id, &st.path_reach);
                push_npc_food_goto_to_views(&player_views, conn_id, &st.food_goto);
            }
            let dt_ms = 200u64.saturating_mul(active as u64).max(200);
            counters
                .ai_sim_time_ms
                .fetch_add(dt_ms / active.max(1) as u64, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_config::ServerConfig;

    #[test]
    fn npc_config_from_live_maps_knobs() {
        let live = ServerConfig {
            npc_enabled: false,
            npc_min: 2,
            npc_max: 8,
            ai_think_period_ticks: 5,
            ai_observe_radius: 12,
            ai_craft_radius: 40,
            ai_reaction_time: 0.5,
            ai_reaction_time_serf: 0.7,
            ai_reaction_time_noble: 0.2,
            ai_ignored_floor_ids: vec![656],
            ..Default::default()
        }
        .live_settings();
        let cfg = NpcConfig::from_live(&live);
        assert!(!cfg.enabled);
        assert_eq!(cfg.min, 2);
        assert_eq!(cfg.max, 8);
        assert_eq!(cfg.think_period_ticks, 5);
        assert_eq!(cfg.observe_radius, 12);
        assert_eq!(cfg.craft_radius, 40);
        assert!((cfg.reaction_time - 0.5).abs() < f32::EPSILON);
        assert!((cfg.reaction_time_serf - 0.7).abs() < f32::EPSILON);
        assert!((cfg.reaction_time_noble - 0.2).abs() < f32::EPSILON);
        assert_eq!(cfg.ignored_floor_ids, vec![656]);

        // Empty live table → compiled AiIgnoredFloorIds (Bear Skin Rug 656/888).
        let mut empty_live = ServerConfig::default().live_settings();
        empty_live.ai_ignored_floor_ids.clear();
        let cfg = NpcConfig::from_live(&empty_live);
        assert_eq!(cfg.ignored_floor_ids, AI_IGNORED_FLOOR_IDS);
    }

    #[test]
    fn craft_use_rejects_flint_putdown_and_keeps_milkweed() {
        let mut content = ContentDb::default();
        let mut loose = ol_content::ObjectDef::empty(135);
        loose.permanent = false;
        content.objects.insert(135, loose);
        let mut weed = ol_content::ObjectDef::empty(50);
        weed.permanent = true;
        content.objects.insert(50, weed);
        let mut tree = ol_content::ObjectDef::empty(49);
        tree.permanent = true;
        content.objects.insert(49, tree);
        let mut trans = ol_content::Transition::default();
        trans.actor_id = 0;
        trans.target_id = 50;
        trans.new_actor_id = 57;
        content.transitions.insert((0, 50), trans);

        // Held flint on the tile it just left: no 135+0 transition.
        assert!(npc_craft_use_is_bare_swap(&content, 135, 0));
        // Empty hand may pick up a loose flint chip (Haxe L804).
        assert!(!npc_craft_use_is_bare_swap(&content, 0, 135));
        // Permanent tree with no transition is not a pickup.
        assert!(npc_craft_use_is_bare_swap(&content, 0, 49));
        // Milkweed harvest is a real transition.
        assert!(!npc_craft_use_is_bare_swap(&content, 0, 50));
        assert!(npc_ground_matches_use_target(&content, 50, 51));
        assert!(!npc_ground_matches_use_target(&content, 135, 0));
    }

    #[test]
    fn haxe_skip_mid_path_only_when_not_yet_a_new_tile() {
        // Haxe: skip iff still moving AND this think has not arrived on a new tile.
        assert!(haxe_skip_mid_path_think(true, false));
        assert!(!haxe_skip_mid_path_think(true, true));
        assert!(!haxe_skip_mid_path_think(false, false));
        assert!(!haxe_skip_mid_path_think(false, true));
    }

    #[test]
    fn sticky_use_skips_craft_while_moving_not_hunger() {
        // Haxe isUsingItem: if isMoving() return true — after hunger, skip craft only.
        assert!(npc_skip_craft_while_sticky_moving(
            true,
            true,
            StickyArrive::Use
        ));
        assert!(npc_skip_craft_while_sticky_moving(
            true,
            true,
            StickyArrive::Drop
        ));
        assert!(!npc_skip_craft_while_sticky_moving(
            true,
            true,
            StickyArrive::None
        ));
        assert!(!npc_skip_craft_while_sticky_moving(
            false,
            true,
            StickyArrive::Use
        ));
        // Mid-tile still skips the whole think (Haxe L428).
        assert!(haxe_skip_mid_path_think(true, false));
        assert!(!haxe_skip_mid_path_think(true, true));
    }

    #[test]
    fn close_use_prio_is_haxe_quad_lt_25() {
        // Haxe L500: distance < 25 (squared Euclidean). 4 tiles orthogonal = 16; 5 = 25 not close.
        assert!(npc_haxe_close_use_prio(10, 10, 10, 10));
        assert!(npc_haxe_close_use_prio(10, 10, 14, 10));
        assert!(!npc_haxe_close_use_prio(10, 10, 15, 10));
        assert!(npc_haxe_close_use_prio(10, 10, 13, 13)); // 9+9=18
        assert!(!npc_haxe_close_use_prio(10, 10, 14, 14)); // 16+16=32
    }

    #[test]
    fn skip_feed_matches_haxe_drop_and_close_use() {
        // isDropingItem always before isFeedingChild.
        assert!(npc_skip_feed_for_sticky(
            0,
            0,
            40,
            40,
            true,
            StickyArrive::Drop
        ));
        // Close use (quad 1) skips feed; far use does not.
        assert!(npc_skip_feed_for_sticky(
            0,
            0,
            1,
            0,
            true,
            StickyArrive::Use
        ));
        assert!(!npc_skip_feed_for_sticky(
            0,
            0,
            20,
            0,
            true,
            StickyArrive::Use
        ));
        assert!(!npc_skip_feed_for_sticky(
            0,
            0,
            1,
            0,
            true,
            StickyArrive::None
        ));
        assert!(!npc_skip_feed_for_sticky(
            0,
            0,
            1,
            0,
            false,
            StickyArrive::Drop
        ));
    }

    #[test]
    fn sticky_arrive_drops_held_player_at_feet_before_use_or_drop() {
        // Haxe isUsingItem L9040: close + heldPlayer → dropPlayer at feet, not USE/DROP on the tile.
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, true, 0, 0, StickyArrive::Use),
            StickyActPlan::DropHeldPlayerAtFeet
        );
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, true, 0, 0, StickyArrive::Drop),
            StickyActPlan::DropHeldPlayerAtFeet
        );
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, false, 0, 0, StickyArrive::Use),
            StickyActPlan::UseTarget
        );
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, false, 0, 0, StickyArrive::Drop),
            StickyActPlan::DropTarget
        );
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, true, true, 0, 0, StickyArrive::Use),
            StickyActPlan::WaitUntilStopped
        );
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 20, 20, false, true, 0, 0, StickyArrive::Drop),
            StickyActPlan::Walk
        );
    }

    #[test]
    fn sticky_use_drops_held_object_when_actor_must_be_empty() {
        // Haxe isUsingItem L9048: holding banana + useActor 0 → dropHeldObject, not USE.
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, false, 2143, 0, StickyArrive::Use),
            StickyActPlan::DropHeldForEmptyHand
        );
        // Actor is the held object — USE it.
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, false, 33, 33, StickyArrive::Use),
            StickyActPlan::UseTarget
        );
        // Empty hand + empty actor — USE.
        assert_eq!(
            npc_plan_sticky_arrive(5, 5, 5, 6, false, false, 0, 0, StickyArrive::Use),
            StickyActPlan::UseTarget
        );
    }

    #[test]
    fn sticky_early_return_drop_always_close_use_only() {
        assert!(npc_sticky_early_return(StickyArrive::Drop, 0, 0, 50, 50));
        assert!(npc_sticky_early_return(StickyArrive::Use, 0, 0, 3, 0));
        assert!(!npc_sticky_early_return(StickyArrive::Use, 0, 0, 20, 0));
        assert!(!npc_sticky_early_return(StickyArrive::None, 0, 0, 0, 0));
    }

    #[test]
    fn last_profession_eating_is_not_farmer() {
        // Haxe L8485: lastProfession = 'Eating' unless FOODSERVER
        let mut st = NpcProfessionState::default();
        st.last_profession_key = Some("Eating".into());
        let p = ol_sim::Player::new(1, 1, "eat@test").snapshot();
        assert_eq!(npc_last_profession_key(&st, &p), Some("Eating"));
        st.last_profession_key = Some("FOODSERVER".into());
        assert_eq!(npc_last_profession_key(&st, &p), Some("FOODSERVER"));
    }

    #[test]
    fn holding_player_from_held_id_or_holding_player_id() {
        assert!(npc_holding_player(-9103334, 0));
        assert!(npc_holding_player(0, 9103334));
        assert!(!npc_holding_player(0, 0));
        assert!(!npc_holding_player(33, 0));
    }

    #[test]
    fn eatable_food_value_uses_dummy_parent() {
        // Haxe isEatable: dummyParent.foodValue when the tile id is a multi-use dummy.
        let mut db = ContentDb::default();
        db.objects.insert(
            31,
            ol_content::ObjectDef {
                id: 31,
                food_value: 5,
                ..ol_content::ObjectDef::empty(31)
            },
        );
        db.objects.insert(
            5001,
            ol_content::ObjectDef {
                id: 5001,
                food_value: 0,
                ..ol_content::ObjectDef::empty(5001)
            },
        );
        db.dummy_parent.insert(5001, 31);
        assert_eq!(eatable_food_value_of(&db, 31), 5);
        assert_eq!(eatable_food_value_of(&db, 5001), 5);
        assert_eq!(eatable_food_value_of(&db, 0), 0);
        assert!(is_eatable_check_again(-1, 5001, eatable_food_value_of(&db, 5001)));
        assert!(is_eatable_check_again(0, 0, 0));
    }

    #[test]
    fn hungry_infant_never_reaches_craft_item() {
        // Haxe MinAgeToEat = 3; hungry infant always returns before craftItem.
        assert!(npc_hungry_infant_skips_craft(0.0, true, true));
        assert!(npc_hungry_infant_skips_craft(2.9, true, true));
        assert!(!npc_hungry_infant_skips_craft(3.0, true, true));
        assert!(!npc_hungry_infant_skips_craft(14.0, true, true));
        assert!(!npc_hungry_infant_skips_craft(0.0, false, true));
        // Haxe L523–532: no mother required — still return after follow + time+=2.5
        assert!(npc_hungry_infant_skips_craft(0.0, true, false));
    }

    #[test]
    fn npc_move_seq_is_player_done_moving_plus_one() {
        // Same as resolve_move_seq(None): human ++done_moving_seqNum / AI move() seq.
        assert_eq!(npc_client_move_seq(0), 1);
        assert_eq!(npc_client_move_seq(1), 2);
        assert_eq!(npc_client_move_seq(7), 8);
    }

    #[test]
    fn send_move_only_if_arrived_or_target_changed() {
        // Haxe isUsingItem L9013: isMoving → no goto. Same dest, moving → do not send.
        assert!(!npc_should_send_move(true, true));
        // Arrived (!isMoving) → send next hop.
        assert!(npc_should_send_move(false, true));
        // Dest changed (follow / danger) while moving → send.
        assert!(npc_should_send_move(true, false));
        assert!(npc_should_send_move(false, false));
    }

    #[test]
    fn is_moving_is_newmoves_or_same_sim_tick_as_move() {
        // Haxe isMoving: newMoves != null. Snapshot moving, or move() this sim tick.
        assert!(npc_haxe_is_moving(true, 0, 10));
        assert!(npc_haxe_is_moving(false, 7, 7));
        assert!(!npc_haxe_is_moving(false, 7, 8));
        assert!(!npc_haxe_is_moving(false, 0, 0));
    }

    #[test]
    fn npc_is_close_action_is_haxe_quad_not_chebyshev() {
        // Orthogonal adjacent is close; diagonal (Chebyshev 1, quad 2) is not.
        assert!(npc_is_close_action(5, 5, 5, 5));
        assert!(npc_is_close_action(5, 5, 6, 5));
        assert!(npc_is_close_action(5, 5, 5, 6));
        assert!(!npc_is_close_action(5, 5, 6, 6));
        assert!(!npc_is_close_action(5, 5, 7, 5));
        // Live 0.3.32: rope holder at 449,239 DROPped at diagonal 450,238 forever.
        assert_eq!(npc_action_stand_tile(449, 239, 450, 238), (450, 239));
        assert_eq!(npc_action_stand_tile(5, 5, 6, 5), (6, 5));
    }

    #[test]
    fn npc_wrap_goal_picks_nearest_torus_image() {
        let w = World::new(500, 500, true);
        // Player at y=484, tile stored as y=2 → walk to 502 (18 tiles), not 2 (482).
        assert_eq!(npc_wrap_goal(&w, 488, 484, 481, 2), (481, 502));
        assert_eq!(npc_wrap_goal(&w, 488, 484, 481, 502), (481, 502));
        let plane = World::new(500, 500, false);
        assert_eq!(npc_wrap_goal(&plane, 488, 484, 481, 2), (481, 2));
    }

    #[test]
    fn action_path_stops_when_orthogonally_close() {
        // Greedy 16-step walks must not continue past an adjacent USE/DROP tile.
        let w = World::new(32, 32, false);
        let c = ContentDb::default();
        let deltas = npc_path_toward(&w, &c, 10, 11, 10, 10, 10.0, 0.0, 16, None, true);
        assert!(
            deltas.is_empty(),
            "already close — no extra steps: {deltas:?}"
        );
        let deltas = npc_path_toward(&w, &c, 10, 12, 10, 10, 10.0, 0.0, 16, None, true);
        if !deltas.is_empty() {
            let (mut x, mut y) = (10, 12);
            for (dx, dy) in &deltas {
                x += *dx;
                y += *dy;
            }
            assert!(
                npc_is_close_action(x, y, 10, 10),
                "stop-when-close path must end adjacent, got {x},{y} steps={deltas:?}"
            );
            assert!(deltas.len() <= 2, "overshot: {deltas:?}");
        }
    }

    #[test]
    fn npc_config_from_live_ignored_floor_ids() {
        let live = ServerConfig {
            ai_ignored_floor_ids: vec![656],
            ..Default::default()
        }
        .live_settings();
        let cfg = NpcConfig::from_live(&live);
        assert_eq!(cfg.ignored_floor_ids, vec![656]);

        let empty = ServerConfig {
            ai_ignored_floor_ids: Vec::new(),
            ..Default::default()
        }
        .live_settings();
        let cfg = NpcConfig::from_live(&empty);
        assert_eq!(cfg.ignored_floor_ids, AI_IGNORED_FLOOR_IDS);

        // from_live itself, not only ServerConfig.live_settings() fill.
        let mut live_empty = ServerConfig::default().live_settings();
        live_empty.ai_ignored_floor_ids.clear();
        let cfg = NpcConfig::from_live(&live_empty);
        assert_eq!(cfg.ignored_floor_ids, AI_IGNORED_FLOOR_IDS);
    }

    #[test]
    fn npc_config_from_live_max_ge_min() {
        let live = ServerConfig {
            npc_min: 10,
            npc_max: 3, // inverted
            ..Default::default()
        }
        .live_settings();
        // LiveSettings already clamps max >= min; from_live also guards.
        let cfg = NpcConfig::from_live(&live);
        assert!(cfg.max >= cfg.min);
    }

    #[test]
    fn reaction_time_by_prestige_class_haxe() {
        let cfg = NpcConfig::default();
        // Haxe: Serf 0.7, Commoner 0.5, Noble 0.2
        assert!((cfg.reaction_for_class(PrestigeClass::Serf, false) - 0.7).abs() < 0.01);
        assert!((cfg.reaction_for_class(PrestigeClass::Commoner, false) - 0.5).abs() < 0.01);
        assert!((cfg.reaction_for_class(PrestigeClass::Noble, false) - 0.2).abs() < 0.01);
        assert!((cfg.reaction_for_class(PrestigeClass::King, false) - 0.2).abs() < 0.01);
        // Angry multiplies by 0.2
        assert!((cfg.reaction_for_class(PrestigeClass::Commoner, true) - 0.1).abs() < 0.01);
    }

    #[test]
    fn prestige_class_for_npc_roles() {
        assert_eq!(prestige_class_for_npc_index(0), PrestigeClass::Serf);
        assert_eq!(prestige_class_for_npc_index(1), PrestigeClass::Commoner);
        assert_eq!(prestige_class_for_npc_index(2), PrestigeClass::Noble);
    }

    #[test]
    fn npc_merged_blocked_by_ai_uses_share_and_peer_craft() {
        let share = ol_sim::new_blocked_by_ai_share();
        {
            let mut g = share.write().unwrap();
            g.insert((1, 1), 2.0);
        }
        let mut progress = HashMap::new();
        progress.insert(2, ((1, 1), 3));
        progress.insert(3, ((5, 5), 1));
        progress.insert(1, ((9, 9), 1));
        let merged = npc_merged_blocked_by_ai(&share, &progress, 1);
        assert!((merged[&(1, 1)] - 5.0).abs() < 0.01);
        assert!((merged[&(5, 5)] - 5.0).abs() < 0.01);
        assert!(!merged.contains_key(&(9, 9)));
    }

    #[test]
    fn remove_blocked_by_ai_during_think_drops_own_food() {
        // Haxe: AiBase.RemoveBlockedByAi L199 / L260–276
        let share = ol_sim::new_blocked_by_ai_share();
        {
            let mut g = share.write().unwrap();
            g.insert((4, 5), 5.0);
            g.insert((1, 1), 5.0);
        }
        let mut p = ol_sim::Player::new(1, 1, "npc@test");
        p.ai_block_targets
            .set_food(BlockTargetClaim::simple(4, 5, 31));
        let snap = p.snapshot();
        let st = NpcProfessionState::default();
        let merged = npc_blocked_by_ai_for_think(&share, &HashMap::new(), 1, &snap, &st);
        assert!(!merged.contains_key(&(4, 5)));
        assert!(merged.contains_key(&(1, 1)));
    }

    #[test]
    fn reset_targets_clears_escape_food_use_and_trans() {
        // Haxe: AiBase.resetTargets L319–325
        let mut st = NpcProfessionState::default();
        st.escape_target = Some((9, 9));
        st.food_goto.sticky_food = Some(StickyFoodTarget::new(4, 5, 31));
        st.sticky_move = Some(NpcStickyMove {
            gx: 2,
            gy: 3,
            expected_parent_id: 33,
            use_actor_parent: 33,
            pending_use: true,
            pending_drop: false,
            label: "use".into(),
            move_from: None,
        });
        st.craft_rt.item.trans_actor_id = Some(33);
        st.craft_rt.item.trans_target_id = Some(32);
        npc_reset_targets(&mut st);
        assert!(st.escape_target.is_none());
        assert!(st.food_goto.sticky_food.is_none());
        assert!(st.sticky_move.is_none());
        assert!(st.craft_rt.item.trans_actor_id.is_none());
        assert!(st.craft_rt.item.trans_target_id.is_none());
    }

    #[test]
    fn reset_targets_keeps_pending_drop() {
        let mut st = NpcProfessionState::default();
        st.sticky_move = Some(NpcStickyMove {
            gx: 2,
            gy: 3,
            expected_parent_id: 0,
            use_actor_parent: 0,
            pending_use: false,
            pending_drop: true,
            label: "drop".into(),
            move_from: None,
        });
        npc_reset_targets(&mut st);
        assert!(st.sticky_move.is_some());
    }

    #[test]
    fn cancle_use_clears_use_keeps_drop() {
        // Haxe: AiBase.CancleUse L8868–8874
        let mut st = NpcProfessionState::default();
        st.sticky_move = Some(NpcStickyMove {
            gx: 1,
            gy: 2,
            expected_parent_id: 33,
            use_actor_parent: 0,
            pending_use: true,
            pending_drop: true,
            label: "use+drop".into(),
            move_from: None,
        });
        npc_cancle_use(&mut st);
        let s = st.sticky_move.expect("dropTarget remains");
        assert!(!s.pending_use);
        assert!(s.pending_drop);
        st.sticky_move = Some(NpcStickyMove {
            gx: 1,
            gy: 2,
            expected_parent_id: 33,
            use_actor_parent: 0,
            pending_use: true,
            pending_drop: false,
            label: "use".into(),
            move_from: None,
        });
        npc_cancle_use(&mut st);
        assert!(st.sticky_move.is_none());
    }

    #[test]
    fn newborn_wipes_profession_state() {
        // Haxe: AiBase.newBorn L327–346
        let mut st = NpcProfessionState::default();
        st.path_reach.add_not_reachable(1, 1, 90.0);
        st.food_goto.did_not_reach_food = 4.0;
        st.food_goto.sticky_food = Some(StickyFoodTarget::new(1, 2, 31));
        st.craft_rt.item.trans_actor_id = Some(33);
        st.was_hungry = true;
        st.try_move_nearest_tile_first = false;
        st.escape_target = Some((0, 0));
        npc_wipe_on_newborn(&mut st);
        assert!(st.path_reach.is_empty());
        assert_eq!(st.food_goto.did_not_reach_food, 0.0);
        assert!(st.food_goto.sticky_food.is_none());
        assert!(st.craft_rt.item.trans_actor_id.is_none());
        assert!(!st.was_hungry);
        assert!(st.try_move_nearest_tile_first);
        assert!(st.escape_target.is_none());
    }

    #[test]
    fn try_move_nearest_tile_first_default_true() {
        assert!(TRY_MOVE_NEAREST_TILE_FIRST_DEFAULT);
        assert!(NpcProfessionState::default().try_move_nearest_tile_first);
        assert_eq!(
            ol_sim::goto_approach_ii_order(true, false),
            [1, 0, 2, 3, 4]
        );
        assert_eq!(
            ol_sim::goto_approach_ii_order(false, false),
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn craft_queue_wait_without_progress_is_fail() {
        // Haxe: isMoving → craftItem true; idle Wait / None → push craftingTasks.
        assert!(!npc_craft_expand_progress(&ShortCraftLiveIntent::Wait, false));
        assert!(npc_craft_expand_progress(&ShortCraftLiveIntent::Wait, true));
        assert!(!npc_craft_expand_progress(&ShortCraftLiveIntent::None, false));
        assert!(npc_craft_expand_progress(
            &ShortCraftLiveIntent::UseAt {
                x: 1,
                y: 2,
                target_id: 3,
                actor_id: 0,
            },
            false
        ));
        assert!(npc_craft_expand_progress(
            &ShortCraftLiveIntent::Goto { x: 4, y: 5 },
            false
        ));
        assert!(!npc_craft_expand_progress(
            &ShortCraftLiveIntent::CraftItem { object_id: 71 },
            false
        ));
    }

    #[test]
    fn sticky_walk_only_always_valid() {
        // expected_parent_id == 0 â†’ pure walk, valid regardless of world contents.
        let sticky = NpcStickyMove {
            gx: 10,
            gy: 10,
            expected_parent_id: 0,
            use_actor_parent: 0,
            pending_use: false,
            pending_drop: false,
            label: "walk".into(),
            move_from: None,
        };
        assert_eq!(sticky.expected_parent_id, 0);
        // milkweed family helper
        assert!(is_milkweed_family(50));
        assert!(is_milkweed_family(51));
        assert!(!is_milkweed_family(36));
    }

    #[test]
    fn live_sensors_escape_wolf_near_npc() {
        // AI-PRIO-LIVE: GetCloseDeadlyAnimal → fill_live_sensors → EscapeThreat::Animal
        let mut aw = AnimalWorld::new();
        aw.spawn(ol_sim::AnimalKind::Wolf, 1, 0);
        let mut player = ol_sim::Player::new(1, 1, "npc@test");
        player.x = 0;
        player.y = 0;
        player.age = 20.0;
        player.food = 15.0;
        player.food_max = 20.0;
        let snap = player.snapshot();
        let st = NpcProfessionState::default();
        let views = HashMap::new();
        let content = ContentDb::default();
        let input = npc_fill_live_sensor_input(&snap, &content, &views, Some(&aw), &st, false);
        assert!(input.deadly_animal.is_some());
        let bundle = fill_live_sensors(&input);
        assert_eq!(bundle.escape_threat, EscapeThreat::Animal);
    }

    #[test]
    fn deadly_player_candidate_marks_attacker_unfriendly() {
        let mut self_p = ol_sim::Player::new(1, 1, "npc@test");
        self_p.p_id = 1;
        self_p.last_player_attacked_me_id = 2;
        let mut other = ol_sim::Player::new(2, 2, "human@test");
        other.p_id = 2;
        other.x = 1;
        other.y = 0;
        other.angry_time = 0.0;
        other.held_id = 560; // known knife
        let mut views = HashMap::new();
        views.insert(2, other.snapshot());
        let content = ContentDb::default();
        let cands = npc_deadly_player_candidates(&self_p.snapshot(), &views, &content);
        assert_eq!(cands.len(), 1);
        assert!(!cands[0].is_friendly);
        assert!(cands[0].holding_weapon);
    }

    #[test]
    fn hidden_wound_is_not_peer_wounded() {
        assert!(!peer_is_wounded_from_held_ex(true, true));
        assert!(peer_is_wounded_from_held_ex(true, false));
    }

    #[test]
    fn lost_combat_prestige_makes_deadly_player_candidate() {
        let mut self_pl = ol_sim::Player::new(1, 1, "npc@test");
        self_pl.last_attacked_player_id = 2;
        let self_p = self_pl.snapshot();
        let mut other = ol_sim::Player::new(2, 2, "human@test");
        other.p_id = 2;
        other.x = 1;
        other.y = 0;
        let mut snap = other.snapshot();
        snap.lost_combat_prestige = 6.0;
        let mut views = HashMap::new();
        views.insert(2, snap);
        let content = ContentDb::default();
        let cands = npc_deadly_player_candidates(&self_p, &views, &content);
        assert_eq!(cands.len(), 1);
        assert!(cands[0].lost_combat_prestige > 5.0);
        assert!(!cands[0].is_friendly);
        assert!(ol_ai::is_deadly_player_candidate(&cands[0], 10.0, 0, 0));
    }

    #[test]
    fn haxe_ai_skip_tick_grows_and_shrinks_current_max() {
        assert_eq!(adjust_ai_current_max(20, 20, 40, 0, 10), 21);
        assert_eq!(adjust_ai_current_max(40, 20, 40, 0, 10), 40);
        assert_eq!(adjust_ai_current_max(21, 20, 40, 11, 10), 20);
        assert_eq!(adjust_ai_current_max(20, 20, 40, 11, 10), 20);
        assert_eq!(adjust_ai_current_max(25, 20, 40, 10, 10), 25);
    }

    #[test]
    fn haxe_ai_spawn_gate_min_and_skip_window() {
        assert!(should_spawn_new_ai(1, 5, 20, 20, 100, 10));
        assert!(!should_spawn_new_ai(20, 5, 20, 20, 100, 10));
        assert!(!should_spawn_new_ai(1, 20, 20, 20, 100, 10));
        assert!(should_spawn_new_ai(1, 21, 25, 20, 5, 10));
        assert!(!should_spawn_new_ai(1, 21, 25, 20, 100, 10));
        assert!(
            should_spawn_new_ai(1, 0, 20, 20, 0, 10),
            "empty server still fills AIs as Eve/Adam"
        );
    }

    #[test]
    fn haxe_readplayers_server_ai_counts_before_new_eve_login() {
        // Haxe Server.main: loaded Ais already in Connection.getAis() before DoTimeLoop.
        assert!(!npc_scheduler_sim_ready(0));
        assert!(npc_scheduler_sim_ready(1));
        let loaded_conn = 2_000_000u64 + 9_103_423;
        assert!(npc_counts_as_living_ai(false, true, true, loaded_conn));
        assert!(!npc_counts_as_living_ai(true, true, true, loaded_conn));
        assert!(npc_counts_as_living_ai(
            false,
            false,
            true,
            NPC_CONN_BASE
        ));
        assert_eq!(npc_slot_index(NPC_CONN_BASE), Some(0));
        assert_eq!(npc_slot_index(NPC_CONN_BASE + 3), Some(3));
        assert_eq!(npc_slot_index(loaded_conn), None);
        // 56 loaded AIs already at/over currentMax → no extra Eve LOGIN.
        assert!(!should_spawn_new_ai(1, 56, 20, 20, 100, 10));
    }

    #[test]
    fn haxe_iseating_sends_self_not_ground_use() {
        // Haxe isEating L8829 myPlayer.self() → SELF 0 0 -1 (doSelf clothingSlot<0).
        let intent = npc_eat_self_intent(11_103_636);
        match intent {
            NetIntent::Raw {
                conn_id,
                tag,
                payload,
            } => {
                assert_eq!(conn_id, 11_103_636);
                assert_eq!(tag, "SELF");
                assert_eq!(payload, self_clothing_raw_payload(-1));
                assert_eq!(payload, "0 0 -1");
            }
            other => panic!("eat must be SELF, got {other:?}"),
        }
    }

    #[test]
    fn pruned_dead_npc_slot_reborns_under_cap() {
        assert!(
            npc_slot_should_rebirth(false, 0, 20),
            "missing view with empty living must rebirth"
        );
        assert!(npc_slot_should_rebirth(false, 19, 20));
        assert!(
            !npc_slot_should_rebirth(true, 0, 20),
            "living view is not a rebirth"
        );
        assert!(
            !npc_slot_should_rebirth(false, 20, 20),
            "at cap Haxe skips doRebirth"
        );
    }

    #[test]
    fn pending_first_login_is_not_a_death() {
        let t = NpcStuckTracker::default();
        assert!(!t.ever_alive, "new slot waits for first living view");
        assert!(!t.was_deleted);
    }

    #[test]
    fn aibase_l401_800_control_flow_helpers() {
        // Haxe: doTimeStuff L397–409 / L523–532 / L514 / L599 / L533 / L8284
        assert!(should_escape_on_moved_one_tile(true, 0.0));
        assert!(!should_escape_on_moved_one_tile(false, 0.0));
        assert!(npc_hungry_infant_skips_craft(2.0, true, false));
        assert!(!npc_hungry_infant_skips_craft(3.0, true, true));
        assert!(waiting_time_blocks_think(false, 2.0));
        assert!(skip_home_and_jobs_while_moving(true));
        assert!(!skip_home_and_jobs_while_moving(false));
        assert!(nice_baby_noble_wants_weapon(true, true, 0));
        assert!(is_child_and_has_mother_from_follow(2.0, true, false, MIN_AGE_TO_EAT));
        assert!(!is_child_and_has_mother_from_follow(3.0, true, false, MIN_AGE_TO_EAT));
        assert!(!is_child_and_has_mother_from_follow(2.0, true, true, MIN_AGE_TO_EAT));
        assert_eq!(child_with_mother_follow_tiles(true), 2);
        assert_eq!(child_with_mother_follow_tiles(false), 4);
        assert_eq!(baby_hungry_follow_tiles(), 5);
        assert!(is_moving_to_player_needed(25.0, 5));
        assert!(!is_moving_to_player_needed(24.0, 5));
        assert!(compute_do_stuff(true, 50.0));
        assert!(!compute_do_stuff(true, 10000.0));
        assert!(compute_do_stuff(false, 10000.0));
        assert!(npc_sticky_early_return(StickyArrive::Drop, 0, 0, 50, 50));
        assert!(npc_sticky_early_return(StickyArrive::Use, 0, 0, 3, 0));
        assert!(!npc_sticky_early_return(StickyArrive::Use, 0, 0, 20, 0));
        let mut st = NpcProfessionState::default();
        st.was_idle = 1.0;
        st.was_idle = decay_was_idle(st.was_idle, 0.5);
        assert!((st.was_idle - 0.95).abs() < 1e-5);
        let rungs = npc_think_job_rungs(&ProfessionStickySnapshot {
            age: 20.0,
            ..Default::default()
        });
        assert_eq!(rungs[0], PriorityRung::CriticalMisc);
        assert_eq!(rungs[1], PriorityRung::LowPriorityWork);
        assert!(!rungs.contains(&PriorityRung::CriticalCraft));
        assert!(!rungs.contains(&PriorityRung::MidPriorityTasks));
    }

    #[test]
    fn get_or_craft_item_uses_haxe_max_search_radius() {
        // Haxe: doTimeStuffHelper L433 itemToCraft.maxSearchRadius = 60;
        // GetOrCraftItem L6198 craftItem(objId) uses that radius.
        assert_eq!(NPC_GET_OR_CRAFT_SEARCH_RADIUS, 60);
        assert_eq!(AI_MAX_SEARCH_RADIUS, 60);
        assert_eq!(GET_CLOSE_CLOTHINGS_RADIUS, 8);
    }

    #[test]
    fn clothing_pickup_sticky_stays_when_tile_becomes_skirt() {
        // Haxe dropTarget is the USE tile: 59+124 then newTarget 128 on the same spot.
        let mut db = ContentDb::default();
        db.objects.insert(
            124,
            ol_content::ObjectDef {
                id: 124,
                clothing: "n".into(),
                ..ol_content::ObjectDef::empty(124)
            },
        );
        let mut skirt = ol_content::ObjectDef::empty(128);
        skirt.id = 128;
        skirt.clothing = "b".into();
        db.objects.insert(128, skirt);
        let mut w = World::new(32, 32, false);
        w.set_object(5, 5, 128);
        let sticky = NpcStickyMove {
            gx: 5,
            gy: 5,
            expected_parent_id: 124,
            use_actor_parent: 0,
            pending_use: false,
            pending_drop: true,
            label: "pickup_cloth 128".into(),
            move_from: None,
        };
        assert!(sticky_move_still_valid(&w, &db, &sticky));
    }

    #[test]
    fn clothing_drop_live_parent_waits_then_uses_skirt() {
        // After USE, expected is still 124 until sim apply; then live parent is 128.
        let mut db = ContentDb::default();
        db.objects.insert(
            124,
            ol_content::ObjectDef {
                id: 124,
                clothing: "n".into(),
                ..ol_content::ObjectDef::empty(124)
            },
        );
        let mut skirt = ol_content::ObjectDef::empty(128);
        skirt.id = 128;
        skirt.clothing = "b".into();
        db.objects.insert(128, skirt);
        assert_eq!(
            npc_clothing_drop_live_parent(&db, 124, 124, true),
            ClothingDropLive::WaitForProduct,
            "do not DROP on Reed Bundle before 128 exists"
        );
        assert_eq!(
            npc_clothing_drop_live_parent(&db, 124, 128, true),
            ClothingDropLive::Parent(128),
            "Haxe live dropTarget parentId follows 128"
        );
        assert_eq!(
            npc_clothing_drop_live_parent(&db, 124, 0, false),
            ClothingDropLive::Parent(124),
            "non-clothing drop still uses snapshot expected (head TargetGone if mismatch)"
        );
        assert_eq!(
            npc_clothing_drop_live_parent(&db, 124, 0, true),
            ClothingDropLive::Parent(124),
            "empty tile after failed USE is TargetGone, not wait-forever"
        );
    }

    #[test]
    fn clothing_wait_apply_ignores_tried_drop_count() {
        // Live 0.3.12: craft_queue_drop left triedDropCount > 3, so the first
        // pickup_cloth wait_apply aborted and never DROPped Reed Skirt 128.
        assert!(!npc_clothing_wait_apply_give_up(3));
        assert!(!npc_clothing_wait_apply_give_up(20));
        assert!(npc_clothing_wait_apply_give_up(21));
        assert_eq!(CLOTHING_WAIT_APPLY_MAX, 20);
    }

    #[test]
    fn clothing_slot_chars_match_haxe_get_clothing_slot() {
        // Haxe: ObjectData.getClothingSlot L1548–1560
        let slot = |c: char| match c {
            'h' => Some(0),
            't' => Some(1),
            's' => Some(2),
            'b' => Some(4),
            'p' => Some(5),
            _ => None,
        };
        assert_eq!(slot('b'), Some(4), "Reed Skirt clothing=b is bottom slot 4");
        assert_eq!(slot('n'), None);
    }

    #[test]
    fn clothing_pickup_range_is_get_close_clothings_r8() {
        // Haxe isPickingupCloths L8726: GetCloseClothings r=8 around the player.
        // Home r=60 is craftItem L672, not this band.
        assert!(npc_clothing_pickup_in_range(0, 0, 5, 0));
        assert!(!npc_clothing_pickup_in_range(0, 0, 8, 0));
        assert!(!npc_clothing_pickup_in_range(100, 100, 20, 0));
        assert!(!npc_clothing_pickup_in_range(100, 100, 0, 0));
    }

    #[test]
    fn food_drop_parks_rope_for_restore() {
        // Live 0.3.39: clothing_craft dropped berry 31 on an empty tile, then
        // the next think empty-hand DROPped that same berry back into the hand.
        // The rope goal must stay parked behind drop_held_clear.
        let mut w = World::new(40, 40, false);
        let db = ContentDb::default();
        w.set_object(12, 0, 59);
        let original = ShortCraftLiveIntent::DropAt { x: 12, y: 0 };
        let rewritten = ShortCraftLiveIntent::DropAt { x: 6, y: 1 };
        let mut st = NpcProfessionState::default();
        let out = npc_park_actor_behind_food_drop(&mut st, &db, &w, original, rewritten);
        assert!(matches!(out, ShortCraftLiveIntent::DropAt { x: 6, y: 1 }));
        assert!(st.hand_clear_pending);
        let parked = st.resume_drop.as_ref().expect("rope parked");
        assert_eq!((parked.gx, parked.gy), (12, 0));
        assert!(parked.pending_drop);
        assert_eq!(parked.expected_parent_id, 59);
        let same = npc_park_actor_behind_food_drop(&mut st, &db, &w, original, original);
        assert!(matches!(same, ShortCraftLiveIntent::DropAt { x: 12, y: 0 }));
    }

    #[test]
    fn receding_drop_walk_aborts_like_haxe_goto_obj() {
        // Live 0.3.38: drop_held_walk @471,127 from ~450,118 stepped west
        // (farther) for minutes while food fell below 0. Haxe gotoObj L1085
        // aborts when the same goal's quad distance does not improve and > 100.
        let mut st = NpcProfessionState::default();
        assert!(
            !npc_goto_obj_receding_abort(&mut st, 450, 118, 471, 127, 0),
            "first visit records lastGotoObj and still walks"
        );
        assert!(st.food_goto.last_goto.is_some());
        st.sticky_move = Some(NpcStickyMove {
            gx: 471,
            gy: 127,
            expected_parent_id: 0,
            use_actor_parent: 0,
            pending_use: false,
            pending_drop: true,
            label: "drop_held_walk".into(),
            move_from: Some((450, 118)),
        });
        assert!(
            npc_goto_obj_receding_abort(&mut st, 449, 118, 471, 127, 0),
            "stepping away from a far drop target is a hostile path"
        );
        assert!(st.path_reach.is_object_with_hostile_path(471, 127));
        assert!(st.sticky_move.is_none());
        assert!(st.food_goto.last_goto.is_none());
        let mut closer = NpcProfessionState::default();
        assert!(!npc_goto_obj_receding_abort(
            &mut closer, 450, 118, 471, 127, 0
        ));
        assert!(
            !npc_goto_obj_receding_abort(&mut closer, 451, 118, 471, 127, 0),
            "a step toward the goal is not receding"
        );
    }

    #[test]
    fn rejected_move_is_goto_fail_not_another_send() {
        // Live 0.3.28: CraftItem(128) drop_held_walk @462,471 stuck at 466,467
        // while food fell below 0. Each think resent MOVE; sim never set
        // isMoving. Haxe L8442 clears dropTarget when gotoObj returns false.
        assert!(npc_prior_move_never_started(Some((466, 467)), 466, 467, false));
        assert!(!npc_prior_move_never_started(Some((466, 467)), 466, 467, true));
        assert!(!npc_prior_move_never_started(Some((467, 466)), 466, 467, false));
        assert!(!npc_prior_move_never_started(None, 466, 467, false));
    }

    #[test]
    fn too_far_drop_keeps_clothing_drop_target() {
        // Live 0.3.27: isDropingItem quad>25 called dropHeldObject(10) and Rust
        // cleared the sticky, so DROP L8456 never put 128 in hand and
        // switchCloths L8709 never ran. Haxe L5281 clears only when max < 1.
        assert!(!drop_held_clears_drop_target(10.0));
        assert!(drop_held_clears_drop_target(0.0));
    }

    #[test]
    fn drop_arrive_label_pickup_cloth_when_tile_is_skirt() {
        let mut db = ContentDb::default();
        let mut skirt = ol_content::ObjectDef::empty(128);
        skirt.id = 128;
        skirt.clothing = "b".into();
        db.objects.insert(128, skirt);
        db.objects.insert(
            33,
            ol_content::ObjectDef {
                id: 33,
                clothing: "n".into(),
                ..ol_content::ObjectDef::empty(33)
            },
        );
        assert_eq!(
            npc_drop_arrive_label(&db, 128, 4, 5, true),
            "pickup_cloth 128"
        );
        assert_eq!(
            npc_drop_arrive_label(&db, 33, 4, 5, true),
            "craft @4,5"
        );
        assert_eq!(
            npc_drop_arrive_label(&db, 128, 4, 5, false),
            "craft @4,5"
        );
    }

    #[test]
    fn craft_scan_merges_home_square_when_home_point_is_inside_player_r() {
        // Live 0.3.16: player 465,64 home 477,29 r=40. Home *point* is inside
        // player Chebyshev r, so the old skip dropped the home square.
        // Thread 444,18 is in home r=40, not player r=40 (Haxe L7240).
        let mut w = World::new(64, 64, false);
        let db = ContentDb::default();
        w.set_object(10, 0, 58);
        let mut st = NpcProfessionState::default();
        // player (10,10) home (10,5) r=6: home point in player square (dy=5<=6);
        // object (10,0) from home dy=5<=6, from player dy=10>6.
        let tiles = npc_scan_for_craft(&mut st, &w, &db, 10, 10, 10, 5, 6);
        assert!(
            tiles.iter().any(|t| t.parent_id == 58 && t.x == 10 && t.y == 0),
            "Haxe addAllObjectsForCrafting scans the full home square, not just the home tile"
        );
        assert!(npc_player_scan_covers_home_square(5, 5, 5, 5));
        assert!(!npc_player_scan_covers_home_square(10, 10, 10, 5));
    }

    #[test]
    fn existing_switch_cloth_finds_orphaned_skirt_beyond_home_r60() {
        // Live 0.3.20/0.3.21: 128 at 228,275, home 433,245, player 448,239.
        // Unbounded world-search finds it, but Haxe GetOrCraft maxSearch 60
        // and isPickingupCloths r=8/home r=60 do not — recraft at home instead.
        let mut db = ContentDb::default();
        let mut skirt = ol_content::ObjectDef::empty(128);
        skirt.id = 128;
        skirt.clothing = "b".into();
        db.objects.insert(128, skirt);
        let mut w = World::new(512, 512, false);
        w.set_object(228, 275, 128);
        w.set_object(449, 233, 50);
        let hit = npc_existing_switch_cloth_xy(&w, &db, 448, 239, [0; 6], 0);
        assert_eq!(hit, Some((228, 275, 128)));
        assert!(!npc_clothing_pickup_in_range(448, 239, 228, 275));
        assert_eq!(
            w.find_closest_object_id(128, 448, 239),
            Some((228, 275))
        );
        assert!(w.chebyshev(448, 239, 228, 275) > NPC_GET_OR_CRAFT_SEARCH_RADIUS);
        assert_eq!(
            npc_existing_object_xy_within(
                &w,
                448,
                239,
                128,
                NPC_GET_OR_CRAFT_SEARCH_RADIUS
            ),
            None
        );
    }

    #[test]
    fn clothing_existing_pickup_uses_get_or_craft_radius_60() {
        let mut w = World::new(128, 128, false);
        w.set_object(20, 0, 128);
        w.set_object(80, 0, 128);
        assert_eq!(
            npc_existing_object_xy_within(&w, 0, 0, 128, NPC_GET_OR_CRAFT_SEARCH_RADIUS),
            Some((20, 0))
        );
        w.set_object(20, 0, 0);
        assert_eq!(
            npc_existing_object_xy_within(&w, 0, 0, 128, NPC_GET_OR_CRAFT_SEARCH_RADIUS),
            None,
            "80 tiles is beyond Haxe clothing maxSearchRadius 60; recraft at home"
        );
    }

    #[test]
    fn reed_skirt_direct_uses_bundle_when_holding_rope() {
        // Held 59 + reed 124 inside 60 → USE, not a closer milkweed step.
        let mut w = World::new(128, 128, false);
        w.set_object(10, 0, 59);
        w.set_object(4, 0, 124);
        w.set_object(1, 0, 50);
        let db = ContentDb::default();
        let use_reed = npc_reed_skirt_direct(&w, &db, 0, 0, 0, 0, 59, 60, &HashSet::new());
        assert!(matches!(
            use_reed,
            Some(ShortCraftLiveIntent::UseAt {
                target_id: 124,
                x: 4,
                y: 0,
                ..
            })
        ));
        let pick_rope = npc_reed_skirt_direct(&w, &db, 0, 0, 0, 0, 0, 60, &HashSet::new());
        assert!(matches!(
            pick_rope,
            Some(ShortCraftLiveIntent::DropAt { x: 10, y: 0 })
        ));
        // Live 0.3.40: held 124 was the only bundle, so the ground-124 check
        // returned None and the bundle was dropped on an empty tile.
        w.set_object(4, 0, 0);
        let held_reed = npc_reed_skirt_direct(&w, &db, 0, 0, 0, 0, 124, 60, &HashSet::new());
        assert!(matches!(
            held_reed,
            Some(ShortCraftLiveIntent::DropAt { x: 10, y: 0 })
        ));
        let keep = npc_drop_held_before_loose_pickup(&w, &db, 0, 0, 0, 0, 124, held_reed.unwrap());
        assert!(matches!(
            keep,
            ShortCraftLiveIntent::DropAt { x: 10, y: 0 }
        ));
    }

    #[test]
    fn reed_skirt_direct_skips_rope_behind_blocking_biome() {
        // Live 0.3.28: Chebyshev picked rope 59 across snow-grey. The walk
        // stopped on the near edge and retried forever, so DROP never put 128
        // in hand and switchCloths never ran.
        let mut w = World::new(80, 80, false);
        let db = ContentDb::default();
        for x in 0..80 {
            for y in 12..16 {
                w.set_biome(x, y, 21);
            }
        }
        w.set_object(10, 20, 59);
        w.set_object(28, 11, 59);
        w.set_object(8, 11, 124);
        // Standing on the last walkable tile: Haxe isBlocked, no closer step.
        assert!(
            !npc_clothing_goal_reachable(&w, &db, 10, 11, 10, 20),
            "no closer step toward a rope behind the mountain"
        );
        assert!(npc_clothing_goal_reachable(&w, &db, 10, 11, 28, 11));
        // Open ground still walks toward a rope inside the 60 search.
        assert!(npc_clothing_goal_reachable(&w, &db, 10, 11, 50, 11));
        let pick = npc_reed_skirt_direct(&w, &db, 10, 11, 10, 11, 0, 60, &HashSet::new());
        assert!(matches!(
            pick,
            Some(ShortCraftLiveIntent::DropAt { x: 28, y: 11 })
        ));
    }

    #[test]
    fn reed_skirt_direct_uses_held_rope_without_ground_rope() {
        // Live 0.3.29: rope already in hand, reed on the near side. Requiring a
        // ground rope returned None and the fallback DROP-placed 59 on the feet.
        let mut w = World::new(64, 64, false);
        let db = ContentDb::default();
        w.set_object(6, 0, 124);
        let use_reed = npc_reed_skirt_direct(&w, &db, 0, 0, 0, 0, 59, 60, &HashSet::new());
        assert!(matches!(
            use_reed,
            Some(ShortCraftLiveIntent::UseAt {
                target_id: 124,
                actor_id: 59,
                x: 6,
                y: 0,
            })
        ));
        for x in 0..64 {
            for y in 2..6 {
                w.set_biome(x, y, 21);
            }
        }
        w.set_object(6, 0, 0);
        w.set_object(6, 20, 124);
        assert_eq!(
            npc_reed_skirt_direct(&w, &db, 6, 1, 6, 1, 59, 60, &HashSet::new()),
            None,
            "held rope must not walk into snow-grey, and must not drop on the feet"
        );
    }

    #[test]
    fn yew_bow_direct_uses_shaft_when_holding_rope() {
        // Haxe craftItem(152) with 59 and 131 in range USEs them (newTarget 151).
        let mut w = World::new(80, 80, false);
        let db = ContentDb::default();
        w.set_object(6, 0, 131);
        w.set_object(12, 0, 59);
        let use_shaft = npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 59, 60, &HashSet::new());
        assert!(matches!(
            use_shaft,
            Some(ShortCraftLiveIntent::UseAt {
                target_id: 131,
                x: 6,
                y: 0,
                ..
            })
        ));
        let pick_rope = npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 0, 60, &HashSet::new());
        assert!(matches!(
            pick_rope,
            Some(ShortCraftLiveIntent::DropAt { x: 12, y: 0 })
        ));
        // Live 0.3.41: the only rope was in hand, so the ground-rope check
        // returned None and food dropHeldObject put the rope back down.
        w.set_object(12, 0, 0);
        let held_only = npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 59, 60, &HashSet::new());
        assert!(matches!(
            held_only,
            Some(ShortCraftLiveIntent::UseAt {
                target_id: 131,
                x: 6,
                y: 0,
                ..
            })
        ));
        // Holding the bow must not drop it to fetch another rope.
        assert!(npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 151, 60, &HashSet::new()).is_none());
        // A finished yew bow in range is picked up instead of making another one.
        w.set_object(12, 0, 59);
        w.set_object(4, 0, 151);
        assert!(matches!(
            npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 0, 60, &HashSet::new()),
            Some(ShortCraftLiveIntent::DropAt { x: 4, y: 0 })
        ));
    }

    #[test]
    fn yew_bow_direct_skips_not_reachable_rope() {
        // isDropingItem goto fail marks the tile. The next getWeapon must not
        // DROP-pick that same rope (live loop: drop_goto_fail @431,238).
        let mut w = World::new(80, 80, false);
        let db = ContentDb::default();
        w.set_object(12, 0, 59);
        w.set_object(6, 0, 131);
        let mut blocked = HashSet::new();
        blocked.insert((12, 0));
        assert!(npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 0, 60, &blocked).is_none());
        w.set_object(8, 0, 59);
        assert!(matches!(
            npc_yew_bow_direct(&w, &db, 0, 0, 0, 0, 0, 60, &blocked),
            Some(ShortCraftLiveIntent::DropAt { x: 8, y: 0 })
        ));
    }

    #[test]
    fn held_food_drops_before_rope_pickup() {
        // Live: clothing_craft DropAt on rope while held stayed 31, so 128
        // never reached switchCloths. Haxe drops the berry first (L7114).
        let mut w = World::new(40, 40, false);
        let db = ContentDb::default();
        w.set_object(12, 0, 59);
        w.set_object(6, 0, 124);
        let pick = npc_reed_skirt_direct(&w, &db, 5, 8, 0, 0, 31, 60, &HashSet::new());
        let Some(pick) = pick else {
            panic!("rope+reed in range");
        };
        let rewritten = npc_drop_held_before_loose_pickup(&w, &db, 5, 8, 0, 0, 31, pick);
        assert!(
            matches!(rewritten, ShortCraftLiveIntent::DropAt { x, y } if (x, y) != (12, 0) && (x, y) != (5, 8)),
            "food must leave the hand on a neighbor, not the rope or the feet tile"
        );
        // Target much closer to home than the player: keep the actor pickup.
        w.set_object(12, 0, 0);
        w.set_object(1, 0, 59);
        let near_home = npc_reed_skirt_direct(&w, &db, 30, 0, 0, 0, 31, 60, &HashSet::new());
        let Some(near_home) = near_home else {
            panic!("near-home rope");
        };
        let kept = npc_drop_held_before_loose_pickup(&w, &db, 30, 0, 0, 0, 31, near_home);
        assert!(matches!(
            kept,
            ShortCraftLiveIntent::DropAt { x: 1, y: 0 }
        ));
    }

    #[test]
    fn empty_drop_skips_occupied_feet_and_mountain() {
        // Live 0.3.30: four neighbors full, DROP swapped the gooseberry on the
        // feet tile and the rope walk was cleared. Mountain is not a drop tile.
        let mut w = World::new(20, 20, false);
        w.set_object(5, 5, 31);
        w.set_object(4, 5, 31);
        w.set_object(6, 5, 31);
        w.set_object(5, 4, 31);
        w.set_object(5, 6, 31);
        w.set_biome(4, 4, 21);
        let (x, y) = npc_empty_drop_xy(&w, 5, 5);
        assert_ne!((x, y), (5, 5));
        w.set_object(5, 5, 0);
        let (x2, y2) = npc_empty_drop_xy(&w, 5, 5);
        assert_ne!((x2, y2), (5, 5), "open feet tile is still not an in-place drop");
        assert_eq!(w.get_object(x, y), 0);
        assert!(
            !is_biome_blocking(w.get_biome(x, y), w.get_floor(x, y) as i32),
            "drop tile {x},{y} must not be mountain"
        );
        assert_ne!((x, y), (4, 4));
    }

    #[test]
    fn clothing_high_priority_uses_haxe_max_search_radius_60() {
        // Haxe: doTimeStuffHelper L672–673 before craftHighPriorityClothing L684.
        assert_eq!(NPC_GET_OR_CRAFT_SEARCH_RADIUS, 60);
        let mut st = NpcProfessionState::default();
        let mut opts = CraftLiveExpandOpts::default();
        npc_prepare_clothing_l672_search(&mut st.craft_rt, &mut opts);
        assert!(
            st.craft_rt.item.search_current_position,
            "searchCurrentPosition includes the player, not only home"
        );
        assert_eq!(st.craft_rt.item.max_search_radius, 60);
        assert_eq!(opts.ai_max_search_radius, 60);
        assert_eq!(opts.ai_max_search_increment, 30);
    }
}
