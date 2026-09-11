//! Basic AI NPC scheduler (Haxe `AiBase.RunAi` shape â€” single thread).
//!
//! Priority: eat if hungry â†’ seek food â†’ **profession ladder scan** (farm/smith/baker/
//! pottery/shepherd shortCraft USE/DROP) â†’ craft (bottom-up valuation) â†’ explore.
//! Activity logged in RAM and flushed every 30s ([`npc_activity`]).
//!
//! // Haxe: ServerAi.doTimeStuff â†’ AiBase.doTimeStuffHelper AssignedJob / AgeRotatedJob

use crate::npc_activity::{
    NpcActivityEvent, NpcActivityKind, NpcActivityLog, NpcStuckTracker,
};
use ol_config::{gameplay_defaults, LiveSettings, AI_IGNORED_FLOOR_IDS};
use ol_content::ContentDb;
use ol_metrics::{Counters, ScopeTimer};
use ol_ai::{
    BestFoodHit, BestFoodQuery, CommandSink, DeadlyPlayerCandidate, EscapeThreat, FoodSearch,
    LiveSensorInput, PlayerWriteInterface, DEFAULT_FOOD_SEARCH_RADIUS, DEADLY_PLAYER_SEARCH_DIST_AI,
    ESCAPE_DIST, escape_target_xy, fill_live_sensors, get_close_deadly_player,
    update_is_hungry,
};
use ol_net::NetIntent;
use ol_main_ai::{plan_hungry_food, ThinkPlan, ThinkSensors};
use ol_player_helper::{
    pick_best_search_food, to_best_hit, AiFoodSearchFlags, ProcessFoodOpts, SearchFoodCand,
};
use ol_sim::{
    ai_rebirth_wait_secs,
    apply_fire_craft_search_radius_override, apply_food_goto_fail, apply_job_flags_to_live_input,
    attack_player, attack_player_action_to_live_intent, deadly_distance_for_held,
    AttackPlayerClothing, AttackPlayerInput, AttackPlayerTarget,
    MIN_AI_AGE_FOR_COMBAT, WEAPON_SEARCH_DIST,
    pick_close_hungry_child, pick_most_distant_own_child, HungryChildCand, is_fertile,
    is_eve_or_adam_name, STARTING_NAME, FEMALE_FIRST_NAMES, MALE_FIRST_NAMES,
    get_max_child_feeding, can_pickup_baby_distance,
    MAX_CHILD_AGE_BREAST_FEEDING, HUNGRY_CHILD_SEARCH_DIST, DISTANT_OWN_CHILD_MIN_DIST,
    DISTANT_OWN_CHILD_SEARCH_DIST,
    go_home_goal_xy, go_home_move_target, handle_death_after_graves_miss, plan_handle_death,
    should_handle_death, wipe_jobs_assign_grave_keeper, HandleDeathAction, HANDLE_DEATH_TIME_BUMP,
    fail_warm_clear_place, get_close_biome, is_super_cold_for_person, is_super_hot_for_person,
    person_looks_female, plan_handle_temperature, HandleTemperatureAction, HandleTemperatureInput,
    COOL_BIOMES, GET_CLOSE_BIOME_DIST, HANDLE_TEMP_KINDLING, HANDLE_TEMP_LARGE_FAST_FIRE,
    HANDLE_TEMP_RELAX_TIME, HANDLE_TEMP_SAY_DRINK, WARM_BIOMES,
    advance_remove_from_container, stage_remove_item_from_container, RemoveFromContainerAdvance,
    RemoveFromContainerStaging,
    apply_path_filters_to_tiles,
    filter_scan_tiles_in_radius,
    basic_farmer_weight_from_runtime, blocked_by_ai_with_peer_progress,
    collect_deadly_animal_blocked_around_for_player,
    AnimalPathPlayerCtx, BowlFillerPeer, AnimalWorld, DEADLY_ANIMAL_SEARCH_DIST,
    is_holding_weapon, is_self_best_bowl_filler, is_self_best_fire_keeper_for_obj,
    is_self_best_grave_keeper_for_obj, FireKeeperPeer, GraveKeeperPeer, pick_grave,
    consider_animals_for_goto, evaluate_nearby_crafts, force_drop_at_feet,
    food_pickup_action_success_reset, full_pile_tiles_from_scan, nonempty_container_tiles_from_scan,
    get_or_craft_objs_from_scan, goto_path_outcome, has_bean_seeds_from_scan,
    has_carrot_seeds_from_scan, init_water_source_ids_from_content, is_walkable,
    is_walkable_with_animals, is_wound_object, ladder_profession_scan_tick, mark_food_path_fail,
    mark_goto_path_fail, merge_path_reach_maps, next_step, next_step_consider_animals_for_player,
    npc_enqueue_get_or_craft_ex, npc_peer_count_for_kind, npc_peer_counts_by_kind,
    farm_peer_lasts_from_npc_rows, path_filters_from_player, peer_home_coords,
    peer_is_wounded_from_held_ex, pending_food_tile_still_actionable, pile_obj_id_from_content,
    plan_goto_obj, plan_is_picking_up_food, plan_profession_ladder_steps,
    quiver_from_clothing_snapshot, snapshot_blocked_by_ai_share,
    commit_fire_place, count_tailor_profession_from_rows, fill_up_quiver_search_radius,
    has_or_become_tailor, resolve_fire_place,
    home_cloth_stock_from_world, home_has_loom_from_world, is_fill_up_quiver_plan,
    plan_clothing_craft_tick, plan_high_priority_clothing, plan_quiver_arrow_precursors,
    make_sharpie_food, FarmAction, FarmCounts, BURDOCK, SEEDING_WILD_CARROT,
    person_color_from_race,
    quiver_can_add_from_slots, requeue_runtime_task_on_fail,
    resolve_sticky_food, scan_held_hungry_work_cost, scan_world_radius,
    select_runtime_sticky_craft_for_tick, self_clothing_raw_payload,
    settle_pending_food_use_fail, smart_drop_held_from_sensors_ex, AiPathReachMaps,
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
    BAKER_SCAN_RADIUS, DEFAULT_CRAFT_RADIUS, DEFAULT_PROFESSION_SCAN_RADIUS, DEFAULT_WALK_SPEED,
    FIRE, FIRE_FOOD_HOME_RADIUS, GOTO_COLLISION_RAD, GRAVE_SEARCH_RADIUS, HANDLING_FIRE_COUNT_RADIUS,
    HOT_COALS, HUNTING_SHORTCRAFT_RADIUS, CUTTING_WOOD_SCAN_RADIUS, COLLECTING_SCAN_RADIUS,
    STARVING_SEARCH_DIST, TAILOR_SCAN_RADIUS,
    INTERACTION_SEC, MAX_AGE,
    MIN_AGE_TO_EAT, POTTERY_SCAN_RADIUS, SHEPHERD_SHORTCRAFT_RADIUS, SMITH_SCAN_RADIUS,
};
use ol_world::{World, DESERT, PASSABLE_RIVER};
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

/// Haxe `isClose` d=1 (squared Euclidean). Diagonal is not close enough to USE/DROP.
#[inline]
fn npc_is_close_action(px: i32, py: i32, tx: i32, ty: i32) -> bool {
    let dx = px - tx;
    let dy = py - ty;
    dx * dx + dy * dy <= 1
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

/// Haxe `doStuff && attackPlayer(playerTarget)` — getWeapon / stand-off / KILL.
// Haxe: AiBase.doTimeStuffHelper ~591
fn npc_run_attack_player(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
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
    let target = deadly.and_then(|d| {
        views.values().find(|o| o.p_id == d.p_id && !o.deleted).map(|o| {
            AttackPlayerTarget {
                p_id: o.p_id,
                x: o.x,
                y: o.y,
                exact_x: o.x as f64,
                exact_y: o.y as f64,
                wounded: npc_is_wounded(content, o.held_id) && !o.is_hidden_wound,
            }
        })
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
            ) {
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
        ShortCraftLiveIntent::SeekOrCraft { actor, .. } => Some((
            NpcActivityKind::Combat,
            format!("attack_get_weapon {actor}"),
            200,
        )),
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
        RemoveFromContainerAdvance::GotoFailed { x, y } => {
            st.remove_from_container = None;
            st.path_reach.add_not_reachable(x, y, 90.0);
            None
        }
        RemoveFromContainerAdvance::Wait => Some((
            NpcActivityKind::Think,
            "remove_wait".into(),
            200,
        )),
        RemoveFromContainerAdvance::DropHeld | RemoveFromContainerAdvance::DropPlayer => {
            if npc_drop_at(intent_tx, conn_id, p.x, p.y, None) {
                Some((
                    NpcActivityKind::Craft,
                    format!("remove_drop @{},{}", p.x, p.y),
                    400,
                ))
            } else {
                Some((NpcActivityKind::Think, "remove_drop_busy".into(), 200))
            }
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
            let payload = format!("{x} {y}");
            if npc_say_raw(intent_tx, conn_id, "REMV", &payload) {
                st.remove_from_container = None;
                Some((
                    NpcActivityKind::Craft,
                    format!("remove_remv @{},{}", x, y),
                    500,
                ))
            } else {
                st.remove_from_container = None;
                st.path_reach.add_not_reachable(x, y, 90.0);
                Some((NpcActivityKind::Think, "remove_remv_fail".into(), 100))
            }
        }
    }
}

/// Haxe `handleTemperature` drink / GetOrCraft / biome / fire / arrive.
// Haxe: AiBase.handleTemperature ~1645 (AI-HANDLE-TEMP)
fn npc_run_handle_temperature(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &World,
    content: &ContentDb,
    craft_graph: &ReverseCraftGraph,
    _env_winter: bool,
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
        cold_place: p.cold_place,
        warm_place: p.warm_place,
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
            ) {
                return Some((kind, format!("temp_craft_water {detail}"), game_ms));
            }
        }
        inp.skip_water = true;
        plan = plan_handle_temperature(inp);
    }
    st.handling_temperature = plan.is_handling;
    st.temp_just_arrived = plan.just_arrived;
    st.last_heat = plan.last_heat;
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
            ) {
                Some((kind, format!("temp_kindling_seek {detail}"), game_ms))
            } else {
                let cleared = fail_warm_clear_place(plan);
                st.handling_temperature = cleared.is_handling;
                st.temp_just_arrived = cleared.just_arrived;
                None
            }
        }
        HandleTemperatureAction::HandlingFire => {
            let _ = (FIRE, HOT_COALS);
            let cleared = fail_warm_clear_place(plan);
            st.handling_temperature = cleared.is_handling;
            st.temp_just_arrived = cleared.just_arrived;
            None
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
            let _ = npc_say_raw(intent_tx, conn_id, "SAY", "DROPBABY");
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
        if npc_drop_at(intent_tx, conn_id, p.x, p.y, None) {
            return Some((
                NpcActivityKind::Baby,
                format!("drop_obj_for_baby held={}", p.held_id),
                400,
            ));
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

/// Haxe `age < MinAgeToEat && isHungry` / `isChildAndHasMother` — stay with mother.
// Haxe: AiBase.doTimeStuffHelper L523–548; isChildAndHasMother L5651–5654
fn npc_run_is_child_with_mother(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &ol_world::World,
    content: &ContentDb,
    conn_id: u64,
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
    views: &std::collections::HashMap<u64, PlayerSnapshot>,
) -> Option<(NpcActivityKind, String, u32)> {
    if p.age >= MIN_AGE_TO_EAT {
        return None;
    }
    if p.held_by > 0 {
        return Some((
            NpcActivityKind::Baby,
            format!("baby_held_by={}", p.held_by),
            400,
        ));
    }
    let follow = p.ai_follow_p_id;
    if follow <= 0 {
        return None;
    }
    let mother = views.values().find(|o| o.p_id == follow && !o.deleted)?;
    // Haxe isMovingToPlayer: hungry infant 5 then 3; else ~4.
    let max_tiles = if p.food < 2.0 { 3 } else { 4 };
    let dx = mother.x - p.x;
    let dy = mother.y - p.y;
    if dx * dx + dy * dy <= max_tiles * max_tiles {
        return Some((
            NpcActivityKind::Baby,
            format!("baby_wait_mother={}", follow),
            400,
        ));
    }
    let walked = npc_try_walk_to(
        intent_tx,
        world,
        content,
        conn_id,
        p.x,
        p.y,
        mother.x,
        mother.y,
        p.food,
        st.food_goto.did_not_reach_food,
        st.animal_path,
    );
    if walked {
        return Some((
            NpcActivityKind::Baby,
            format!(
                "baby_follow_mother={} @{},{}",
                follow, mother.x, mother.y
            ),
            250,
        ));
    }
    Some((
        NpcActivityKind::Baby,
        format!("baby_wait_mother={}", follow),
        400,
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
) -> Option<(i32, i32)> {
    // Haxe: considerAnimals = checkIfDangerous && didNotReachFood < 5 && food_store > -1
    let consider = consider_animals_for_goto(true, did_not_reach_food, food_store);
    next_step_consider_animals_for_player(world, content, sx, sy, gx, gy, consider, animal)
}

/// Multi-step relative path toward `(gx,gy)` (animal-aware), capped at `max_steps`.
/// Prefer this over a single `npc_next_step_to` so timed movement commits a real path.
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
) -> Vec<(i32, i32)> {
    let mut deltas = Vec::new();
    if max_steps == 0 || (sx == gx && sy == gy) {
        return deltas;
    }
    let mut cx = sx;
    let mut cy = sy;
    for _ in 0..max_steps {
        if cx == gx && cy == gy {
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
        );
        let Some((dx, dy)) = step else {
            break;
        };
        if dx == 0 && dy == 0 {
            break;
        }
        deltas.push((dx, dy));
        cx += dx;
        cy += dy;
    }
    // Greedy one-tile fallback when A* finds nothing (edge/blocked).
    if deltas.is_empty() {
        let sdx = (gx - sx).signum();
        let sdy = (gy - sy).signum();
        for (dx, dy) in [(sdx, 0), (0, sdy), (sdx, sdy)] {
            if dx == 0 && dy == 0 {
                continue;
            }
            if is_walkable(world, content, sx + dx, sy + dy) {
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
) -> bool {
    let deltas = npc_path_toward(
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
    );
    if deltas.is_empty() {
        return false;
    }
    intent_tx
        .try_send(NetIntent::Move {
            conn_id,
            xs: px,
            ys: py,
            deltas,
            seq: None,
        })
        .is_ok()
}

/// Expand one queued / sticky product via GetOrCraft (Haxe `craftItem` from doTimeStuffHelper).
// Haxe: AiBase.doTimeStuffHelper ~667–680 craftItem
fn npc_expand_craft_intent(
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
    let goc_objs = get_or_craft_objs_from_scan(tiles, None);
    let full_piles = full_pile_tiles_from_scan(tiles);
    let nonempty_boxes = nonempty_container_tiles_from_scan(tiles);
    let (water_ids, bucket_ids) = init_water_source_ids_from_content(content);
    let opts = CraftLiveExpandOpts {
        home: Some((home_x, home_y)),
        is_or_can_smith: is_smith,
        now_sec: tick as f64 * 0.2,
        water_source_ids: water_ids,
        bucket_water_source_ids: bucket_ids,
        ..Default::default()
    }
    .with_content_craft_gates(content);
    let pile_id_for = |id: i32| {
        let p = pile_obj_id_from_content(content, id);
        if p > 0 {
            p
        } else {
            0
        }
    };
    npc_enqueue_get_or_craft_ex(
        intent,
        &goc_objs,
        px,
        py,
        held_id,
        moving,
        Some((px, py)),
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

/// Haxe `isConsideringMakingFood` → `makeSharpieFood` (wild carrot / burdock).
// Haxe: AiBase.makeSharpieFood L4096–4118
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
    const R: i32 = 40;
    let tiles = npc_scan_cached_rw(st, world, content, p.x, p.y, R);
    let mut counts = FarmCounts::default();
    counts.held_id = p.held_id;
    let mut n_carrot = 0i32;
    let mut n_burdock = 0i32;
    for t in &tiles {
        if t.parent_id == SEEDING_WILD_CARROT {
            n_carrot += 1;
        } else if t.parent_id == BURDOCK {
            n_burdock += 1;
        }
    }
    counts.set(SEEDING_WILD_CARROT, n_carrot);
    counts.set(BURDOCK, n_burdock);
    let FarmAction::CraftItem { object_id } = make_sharpie_food(&counts) else {
        return None;
    };
    let blocked = st.path_reach.blocked_coords(None);
    let intent = npc_expand_craft_product(
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
    ) {
        if detail.is_empty() {
            detail = format!("make_sharpie_food {object_id}");
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
) -> bool {
    let did_not_reach_food = st.food_goto.did_not_reach_food;
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
            if npc_is_close_action(px, py, x, y) {
                if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                    clear_sticky_move(st);
                    *kind = NpcActivityKind::Craft;
                    *detail = format!("craft_queue_use @{x},{y}");
                    *game_ms = 500;
                    return true;
                }
            } else if npc_try_walk_to(
                intent_tx,
                world,
                content,
                conn_id,
                px,
                py,
                x,
                y,
                food,
                did_not_reach_food,
                st.animal_path,
            ) {
                let expected = if target_id != 0 {
                    sticky_parent_id(content, target_id)
                } else {
                    sticky_parent_id(content, world.get_object(x, y))
                };
                set_sticky_move_use(
                    st,
                    x,
                    y,
                    expected,
                    actor_id,
                    true,
                    format!("use_held @{x},{y}"),
                );
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_walk_use @{x},{y}");
                *game_ms = 250;
                return true;
            }
            false
        }
        ShortCraftLiveIntent::UseOnEmptyGround { x, y, held: actor_id } => {
            if npc_is_close_action(px, py, x, y) {
                if npc_use_at(intent_tx, conn_id, x, y, None, None) {
                    clear_sticky_move(st);
                    *kind = NpcActivityKind::Craft;
                    *detail = format!("craft_queue_use @{x},{y}");
                    *game_ms = 500;
                    return true;
                }
            } else if npc_try_walk_to(
                intent_tx,
                world,
                content,
                conn_id,
                px,
                py,
                x,
                y,
                food,
                did_not_reach_food,
                st.animal_path,
            ) {
                let expected = sticky_parent_id(content, world.get_object(x, y));
                set_sticky_move_use(
                    st,
                    x,
                    y,
                    expected,
                    actor_id,
                    true,
                    format!("use_held @{x},{y}"),
                );
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_walk_use @{x},{y}");
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
            if npc_is_close_action(px, py, x, y) && is_drop {
                if npc_drop_at(intent_tx, conn_id, x, y, None) {
                    *kind = NpcActivityKind::Craft;
                    *detail = format!("craft_queue_drop @{x},{y}");
                    *game_ms = 400;
                    return true;
                }
            } else if !npc_is_close_action(px, py, x, y)
                && npc_try_walk_to(
                    intent_tx,
                    world,
                    content,
                    conn_id,
                    px,
                    py,
                    x,
                    y,
                    food,
                    did_not_reach_food,
                    st.animal_path,
                )
            {
                *kind = NpcActivityKind::Craft;
                *detail = format!("craft_queue_walk @{x},{y}");
                *game_ms = 250;
                return true;
            }
            false
        }
        ShortCraftLiveIntent::GotoForge {
            forge_x, forge_y, ..
        } => {
            if !npc_is_close_action(px, py, forge_x, forge_y)
                && npc_try_walk_to(
                    intent_tx,
                    world,
                    content,
                    conn_id,
                    px,
                    py,
                    forge_x,
                    forge_y,
                    food,
                    did_not_reach_food,
                    st.animal_path,
                )
            {
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
    label: impl Into<String>,
) -> bool {
    let ok = npc_try_walk_to(
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
    );
    if ok {
        set_sticky_move(st, gx, gy, expected_parent_id, label);
    }
    ok
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
    /// Short label for activity log.
    label: String,
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
    /// Reused world scan for this think (same center + radius or inner radius).
    scan_cache: Option<NpcScanCache>,
    /// Scan fill time this think (µs), excluding cache hits.
    scan_us_acc: u64,
    scan_calls: u32,
    scan_hits: u32,
    /// Held id we already tried to eat this hunger bout (Haxe refuseFood → drop).
    eat_fail_held: i32,
    /// Last tile we thought on. Haxe `movedOneTile` — replan after a tile even if still pathing.
    last_think_xy: Option<(i32, i32)>,
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
            scan_cache: None,
            scan_us_acc: 0,
            scan_calls: 0,
            scan_hits: 0,
            eat_fail_held: 0,
            last_think_xy: None,
        }
    }
}

/// Haxe `doTimeStuffHelper`: skip only while moving *and* we have not arrived on a new tile.
// Haxe: AiBase.doTimeStuffHelper L428 `if (movedOneTileTmp == false && myPlayer.isMoving()) return;`
fn haxe_skip_mid_path_think(moving: bool, moved_one_tile: bool) -> bool {
    moving && !moved_one_tile
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
    false
}

fn set_sticky_move(
    st: &mut NpcProfessionState,
    gx: i32,
    gy: i32,
    expected_parent_id: i32,
    label: impl Into<String>,
) {
    set_sticky_move_use(st, gx, gy, expected_parent_id, 0, false, label);
}

fn set_sticky_move_use(
    st: &mut NpcProfessionState,
    gx: i32,
    gy: i32,
    expected_parent_id: i32,
    use_actor_parent: i32,
    pending_use: bool,
    label: impl Into<String>,
) {
    st.sticky_move = Some(NpcStickyMove {
        gx,
        gy,
        expected_parent_id,
        use_actor_parent,
        pending_use,
        label: label.into(),
    });
}

fn clear_sticky_move(st: &mut NpcProfessionState) {
    st.sticky_move = None;
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

/// Haxe `canEatObj` without yum tables: foodValue, age, and stomach room.
// Haxe: GPI.canEatObj L6264–6272
fn npc_can_eat_held(content: &ContentDb, held_id: i32, food: f32, food_max: f32, age: f32) -> bool {
    if age < MIN_AGE_TO_EAT {
        return false;
    }
    let fv = food_at(content, held_id);
    if fv < 1 {
        return false;
    }
    let need = (fv as f32 / 4.0).ceil();
    food_max - food >= need
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
            let fv = food_at(self.content, o.id);
            if fv <= 0 {
                continue;
            }
            let not_reachable = self
                .path_reach
                .map(|m| m.blocks_target(o.x, o.y, None))
                .unwrap_or(false);
            cands.push(SearchFoodCand {
                parent_id: base,
                food_id: base,
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
        // AI hungry seek: seed/danger gates like SimFoodSearch(ai=true)
        opts.ai = Some(AiFoodSearchFlags::default());
        let (idx, score) = pick_best_search_food(&cands, &opts, &stock_tiles)?;
        let cand = &cands[idx];
        let hit = to_best_hit(cand, &score, self.px, self.py);
        Some(BestFoodHit {
            x: hit.tx,
            y: hit.ty,
            food_id: hit.food_id,
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
    if p.held_id != 0 {
        // Async success: picked something up â€” clear sticky + reset didNotReachFood.
        // Haxe: ~8703â€“8704 (only after done==true; Rust settles next tick)
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
    // Container sticky: ground tile is basket/etc (food_value often 0) — validate via sticky parent.
    // Haxe: isEatableCheckAgain; container indexInContainer > -1 still often true (TODO in Haxe)
    let sticky_tile = food_goto.sticky_food.map(|s| {
        if s.in_container() {
            let fv = food_at(content, s.parent_id);
            (s.parent_id, fv)
        } else {
            let id = nearby
                .iter()
                .find(|o| o.x == s.x && o.y == s.y)
                .map(|o| o.id)
                .unwrap_or(0);
            let fv = food_at(content, id);
            (id, fv)
        }
    });
    let (sticky_id, sticky_fv) = sticky_tile.unwrap_or((0, 0));
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
    // Container sticky: tile still eatable if parent food still has food_value;
    // ground: tile id food_value > 0.
    let tile_still = if food.in_container() {
        fv > 0
    } else {
        let tid = nearby
            .iter()
            .find(|o| o.x == food.x && o.y == food.y)
            .map(|o| o.id)
            .unwrap_or(0);
        let tfv = food_at(content, tid);
        tid != 0 && tfv > 0
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
            st.food_goto.sticky_food = None;
            st.food_goto.last_goto = None;
            st.food_goto.last_goto_dist = -1.0;
            Some((
                NpcActivityKind::SeekFood,
                format!("food_uneatable id={}", food.parent_id),
                100,
            ))
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
                                format!("walk_food id={}", food.parent_id),
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
                            format!("walk_food id={}", food.parent_id),
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
            // Can't drop â€” mark food fail 30s (can't USE/REMV with hands full)
            mark_food_path_fail(&mut st.path_reach, food.x, food.y);
            st.food_goto.sticky_food = None;
            Some((
                NpcActivityKind::SeekFood,
                format!("pickup_drop_fail mark30 @{},{}", food.x, food.y),
                100,
            ))
        }
        IsPickingupFoodPlan::Remv { x, y, index } => {
            let payload = format!("{x} {y} {index}");
            // PlayerWriteInterface: same Raw path as human clients
            if npc_say_raw(&intent_tx, conn_id, "REMV", &payload) {
                // Keep sticky until next-tick settle (Haxe clears only after known done).
                // Haxe: isPickingupFood ~8694â€“8704
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
                // Async apply; settle next tick marks 30s if still empty + tile food.
                // Haxe: isPickingupFood ~8694â€“8704 (no optimistic clear)
                st.food_goto.pending_food_xy = Some((x, y));
                st.food_goto.pending_food_container = food.in_container();
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
                // Keep sticky until settle confirms success/fail.
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
            let dist = (x - p.x).abs().max((y - p.y).abs());
            if dist <= 1 {
                if npc_drop_at(&intent_tx, conn_id, x, y, None)
                {
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_drop @{},{}", x, y),
                        400,
                    ));
                }
            } else if {
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
                )
            } {
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("{tag}_walk_drop @{},{}", x, y),
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
            let dist = (x - p.x).abs().max((y - p.y).abs());
            if dist <= 1 {
                if npc_use_at(&intent_tx, conn_id, x, y, None, None)
                {
                    return Some((
                        NpcActivityKind::SeekFood,
                        format!("{tag}_use tid={} @{},{}", target_id, x, y),
                        400,
                    ));
                }
            } else if {
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
/// Haxe `AddObjBlockedByAi` default 5s — reserve a target so other AIs skip it.
fn npc_claim_tile(share: &BlockedByAiShare, x: i32, y: i32) {
    if let Ok(mut g) = share.write() {
        g.insert((x, y), 5.0);
    }
}

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

fn log_ev(
    log: &NpcActivityLog,
    conn_id: u64,
    p: &PlayerSnapshot,
    kind: NpcActivityKind,
    cpu_us: u32,
    game_ms: u32,
    detail: impl Into<String>,
) {
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
        detail: detail.into(),
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

fn count_living_npcs(player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>) -> u32 {
    player_views
        .read()
        .ok()
        .map(|g| {
            g.values()
                .filter(|p| p.conn_id >= NPC_CONN_BASE && !p.deleted)
                .count() as u32
        })
        .unwrap_or(0)
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
    let mut craft_progress: HashMap<u64, ((i32, i32), i32)> = HashMap::new();
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
        if should_spawn_new_ai(
            tick,
            living,
            current_max_ais,
            min,
            last_skipped_ticks,
            MAX_AI_SKIPPED_TICKS_BEFORE_REDUCING,
        ) && active < max
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

        // AI-TAKEOVER: drive disconnected human bodies with thin eat/explore AI.
        // Haxe: ServerAi.doTimeStuff on Connection.ais (human-replacement AIs).
        {
            let takeover: Vec<u64> = {
                let views = player_views.read().unwrap();
                let mut ids: Vec<u64> = views
                    .values()
                    .filter(|o| o.ai_controlled && !o.deleted)
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
                    let h = update_is_hungry(st.was_hungry, p.food, p.food_max, p.held_id);
                    st.was_hungry = h;
                    h
                };
                if hungry && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                    let _ = intent_tx.try_send(NetIntent::Use {
                        conn_id,
                        x: p.x,
                        y: p.y,
                        id: None,
                        index: None,
                    });
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
                if hungry {
                    // AI-PICKUP-FOOD: full isPickingupFood SM (same as NPC SeekFood)
                    // Haxe: isPickingupFood drop held / USE / DROP / REMV
                    let st = profession_state.entry(conn_id).or_default();
                    settle_npc_pending_food_action(&content, &nearby, &p, st);
                    if let Some(food) = resolve_npc_food_target(
                        &content,
                        &nearby,
                        p.x,
                        p.y,
                        p.food,
                        p.food_max,
                        &st.path_reach,
                        &mut st.food_goto,
                    ) {
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
                            seq: None,
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

        for i in 0..active {
            let conn_id = NPC_CONN_BASE + i as u64;
            // Ensure prestige class for reaction timing (Forager=Serf, Farmer=Commoner, Hunter=Noble).
            {
                let st = profession_state.entry(conn_id).or_default();
                // Re-assert role class if still default Commoner and never assigned.
                if !st.class_assigned {
                    st.prestige_class = prestige_class_for_npc_index(i);
                    st.class_assigned = true;
                }
            }

            // Haxe: AiBase.time -= timePassedInSeconds; if (time > 0) return
            {
                let st = profession_state.entry(conn_id).or_default();
                st.think_time_sec -= SCHED_DT_SEC;
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
            let snap = player_views
                .read()
                .ok()
                .and_then(|g| g.get(&conn_id).cloned());
            let Some(p) = snap else {
                continue;
            };

            let tracker = stuck_map.entry(conn_id).or_default();

            // Death detection. Haxe ServerAi.doRebirth waits before CreateNewAiPlayer.
            if p.deleted {
                if !tracker.was_deleted {
                    tracker.was_deleted = true;
                    tracker.rebirth_wait_sec = ai_rebirth_wait_secs(p.age, rand::random::<f32>());
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::Death,
                        0,
                        0,
                        format!(
                            "age={:.1} food={:.1} reason=deleted_or_starved held={} wait={:.1}s",
                            p.age, p.food, p.held_id, tracker.rebirth_wait_sec
                        ),
                    );
                    continue;
                }
                // Scheduler wake is 200 ms.
                tracker.rebirth_wait_sec -= 0.2;
                if tracker.rebirth_wait_sec > 0.0 {
                    continue;
                }
                tracker.rebirth_wait_sec = f32::MAX;
                // Haxe: `if (this.number > ServerSettings.NumberOfAis) removeAi`
                if (i as u32) >= max {
                    continue;
                }
                let email = format!("npc-re-{}@local", i);
                let _ = intent_tx.try_send(NetIntent::Login {
                    conn_id,
                    reconnect: false,
                    email,
                    client_tag: "client_npc".into(),
                    client_ip: String::new(),
                });
                if let Some(st) = profession_state.get_mut(&conn_id) {
                    clear_sticky_move(st);
                    st.think_time_sec = 0.0;
                    st.class_assigned = false;
                    st.last_think_xy = None;
                }
                continue;
            }
            tracker.was_deleted = false;
            tracker.rebirth_wait_sec = 0.0;
            tracker.note_position(p.x, p.y);

            // Haxe: time += reactionTime (class-based Serf/Commoner/Noble)
            {
                let st = profession_state.entry(conn_id).or_default();
                let angry = false; // residual: isAngryOrTerrified
                st.think_time_sec += cfg.reaction_for_class(st.prestige_class, angry);
            }

            // Haxe checkIsHungryAndEat hysteresis: enter at max(3, 30% max), leave at 80%.
            // Haxe: AiBase.checkIsHungryAndEat L8841–8856
            let hungry = {
                let st = profession_state.entry(conn_id).or_default();
                let h = update_is_hungry(st.was_hungry, p.food, p.food_max, p.held_id);
                st.was_hungry = h;
                h
            };
            let starving = p.food < -1.0;

            // Haxe: if (movedOneTileTmp == false && isMoving()) return
            // Replan after each arrived tile (feeding / eat / escape can retarget a long craft walk).
            // Haxe: AiBase.doTimeStuffHelper L428
            let moved_one_tile = {
                let st = profession_state.entry(conn_id).or_default();
                let moved = st
                    .last_think_xy
                    .map(|(x, y)| x != p.x || y != p.y)
                    .unwrap_or(true);
                st.last_think_xy = Some((p.x, p.y));
                moved
            };
            if p.moving {
                let (still_valid, pending_use, sticky_label) = {
                    let st = profession_state.entry(conn_id).or_default();
                    match st.sticky_move.clone() {
                        None => (true, false, String::new()),
                        Some(ref sticky) => {
                            let w = world.read().unwrap();
                            let ok = sticky_move_still_valid(&w, &content, sticky);
                            (ok, sticky.pending_use, sticky.label.clone())
                        }
                    }
                };
                if still_valid && haxe_skip_mid_path_think(true, moved_one_tile) {
                    let detail = if pending_use {
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
            } else {
                // Path finished: Haxe isUsingItem — USE staged target instead of replanning.
                let pending = profession_state
                    .get(&conn_id)
                    .and_then(|st| st.sticky_move.clone());
                let mut finished_use = false;
                if let Some(sticky) = pending {
                    // Haxe isUsingItem: close USE still runs when hungry.
                    if sticky.pending_use {
                        let w = world.read().unwrap();
                        if sticky_move_still_valid(&w, &content, &sticky) {
                            let held_ok = sticky.use_actor_parent == 0
                                || sticky_parent_id(&content, p.held_id) == sticky.use_actor_parent;
                            if held_ok && npc_is_close_action(p.x, p.y, sticky.gx, sticky.gy) {
                                if npc_use_at(&intent_tx, conn_id, sticky.gx, sticky.gy, None, None)
                                {
                                    if let Some(st) = profession_state.get_mut(&conn_id) {
                                        clear_sticky_move(st);
                                    }
                                    log_ev(
                                        &activity,
                                        conn_id,
                                        &p,
                                        NpcActivityKind::Craft,
                                        500,
                                        0,
                                        format!("use_held_arrive @{},{}", sticky.gx, sticky.gy),
                                    );
                                    finished_use = true;
                                }
                            } else if held_ok && !npc_is_close_action(p.x, p.y, sticky.gx, sticky.gy) {
                                let walked = {
                                    let st = profession_state.entry(conn_id).or_default();
                                    npc_try_walk_to_sticky(
                                        &intent_tx,
                                        &w,
                                        &content,
                                        st,
                                        conn_id,
                                        p.x,
                                        p.y,
                                        sticky.gx,
                                        sticky.gy,
                                        p.food,
                                        sticky.expected_parent_id,
                                        sticky.label.clone(),
                                    )
                                };
                                if walked {
                                    if let Some(st) = profession_state.get_mut(&conn_id) {
                                        if let Some(ref mut sm) = st.sticky_move {
                                            sm.pending_use = true;
                                            sm.use_actor_parent = sticky.use_actor_parent;
                                        }
                                    }
                                    log_ev(
                                        &activity,
                                        conn_id,
                                        &p,
                                        NpcActivityKind::Craft,
                                        250,
                                        0,
                                        format!("use_held_walk @{},{}", sticky.gx, sticky.gy),
                                    );
                                    finished_use = true;
                                }
                            }
                        }
                    }
                }
                if finished_use {
                    continue;
                }
                if let Some(st) = profession_state.get_mut(&conn_id) {
                    clear_sticky_move(st);
                }
            }

            let profession = profession_for_index(i);
            let food_need = if p.food_max > 0.1 {
                ((p.food_max - p.food) / p.food_max).clamp(0.0, 2.0)
            } else {
                0.0
            };

            let ally_count = {
                let views = player_views.read().unwrap();
                views
                    .values()
                    .filter(|o| {
                        !o.deleted
                            && o.conn_id != conn_id
                            && (o.x - p.x).abs().max((o.y - p.y).abs()) <= craft_radius
                    })
                    .count() as u32
            };

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

            // Haxe: getHeldByPlayer() != null → return (carried baby does not think)
            // Haxe: AiBase.doTimeStuffHelper L472–475
            if p.held_by > 0 {
                continue;
            }

            // --- 0. Escape deadly animal/player (AI-PRIO-LIVE) ---
            // Haxe: doTimeStuffHelper GetCloseDeadly* + escape before hungry
            if !acted {
                let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                let animals_g = animals.read().ok();
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                    );
                    let bundle = fill_live_sensors(&input);
                    st.was_hungry = bundle.is_hungry;
                    if bundle.escape_threat != EscapeThreat::None {
                        let (tx, ty) = match bundle.escape_threat {
                            EscapeThreat::Animal => input
                                .deadly_animal
                                .map(|(x, y, _)| escape_target_xy(p.x, p.y, x, y, ESCAPE_DIST))
                                .unwrap_or((p.x + ESCAPE_DIST, p.y)),
                            EscapeThreat::Player => input
                                .deadly_player
                                .map(|(x, y, _, _)| {
                                    escape_target_xy(p.x, p.y, x, y, ESCAPE_DIST)
                                })
                                .unwrap_or((p.x + ESCAPE_DIST, p.y)),
                            EscapeThreat::None => (p.x, p.y),
                        };
                        let walked = {
                            let w = world.read().unwrap();
                            npc_try_walk_to(
                                &intent_tx,
                                &w,
                                &content,
                                conn_id,
                                p.x,
                                p.y,
                                tx,
                                ty,
                                p.food,
                                st.food_goto.did_not_reach_food,
                                st.animal_path,
                            )
                        };
                        if walked {
                            kind = NpcActivityKind::Combat;
                            detail = format!("escape_{:?} @{},{}", bundle.escape_threat, tx, ty);
                            game_ms = 250;
                            acted = true;
                        }
                    }
                }
            }

            // --- 0b. handleDeath (age ≥ MaxAge−2) before eat ---
            // Haxe: deadlyPlayer == null && handleDeath() ~554
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

            // --- 0c. isRemovingFromContainer (after use/handleDeath, before eat) ---
            // Haxe: doTimeStuffHelper isUsingItem then isRemovingFromContainer ~573–575
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

            // --- 0d. Hungry infant / isChildAndHasMother stay with mother ---
            // Haxe: doTimeStuffHelper L523–548 before isEating / isFeedingChild
            if !acted {
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
                    if let Some((k, d, ms)) = npc_run_is_child_with_mother(
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

            // --- 1. Eat held food if hungry ---
            // Haxe isEating: canEatObj (room + not superMeh); else dropHeldObject.
            if !acted && hungry && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                let st = profession_state.entry(conn_id).or_default();
                let can_eat = npc_can_eat_held(
                    &content,
                    p.held_id,
                    p.food,
                    p.food_max,
                    p.age,
                );
                // Haxe doEating refuseFood (superMeh + food>4 / no room): drop and seek better.
                if !can_eat || st.eat_fail_held == p.held_id {
                    st.eat_fail_held = 0;
                    if npc_drop_at(&intent_tx, conn_id, p.x, p.y, None) {
                        kind = NpcActivityKind::Eat;
                        detail = format!("eat_refuse_drop held={}", p.held_id);
                        game_ms = 400;
                        acted = true;
                    }
                } else if intent_tx
                    .try_send(NetIntent::Use {
                        conn_id,
                        x: p.x,
                        y: p.y,
                        id: None,
                        index: None,
                    })
                    .is_ok()
                {
                    st.eat_fail_held = p.held_id;
                    kind = NpcActivityKind::Eat;
                    detail = format!("eat_held={}", p.held_id);
                    game_ms = 500;
                    acted = true;
                }
            } else if let Some(st) = profession_state.get_mut(&conn_id) {
                st.eat_fail_held = 0;
            }

            // --- 1a. isFeedingChild (Haxe after isEating, before pickup food) ---
            // Haxe: AiBase.isFeedingChild L6412 — BABY pickup + hold while TimeHelper nurses
            if !acted {
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

            // --- 1b. handleTemperature drink/craft/biome (Haxe ~1645 / ~653) ---
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

            // --- 1c. isConsideringMakingFood (Haxe before isPickingupFood) ---
            // Hungry + no nearby food → makeSharpieFood. Starving with a food
            // target, or food within ~30 tiles, skip make and pick instead.
            // Haxe: AiBase.isConsideringMakingFood L8466–8607
            if !acted && hungry && p.age >= MIN_AGE_TO_EAT {
                let st = profession_state.entry(conn_id).or_default();
                settle_npc_pending_food_action(&content, &nearby, &p, st);
                let food = resolve_npc_food_target(
                    &content,
                    &nearby,
                    p.x,
                    p.y,
                    p.food,
                    p.food_max,
                    &st.path_reach,
                    &mut st.food_goto,
                );
                let food_near = food.as_ref().map(|f| {
                    let dx = f.x - p.x;
                    let dy = f.y - p.y;
                    dx * dx + dy * dy < 900
                }).unwrap_or(false);
                let skip_make = (starving && food.is_some()) || food_near;
                if !skip_make {
                    let is_smith = matches!(profession, CraftProfession::Smith);
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

            // --- 2. Seek / pick food if hungry (AI-PICKUP-FOOD full isPickingupFood SM) ---
            // Haxe: AiBase.isPickingupFood drop held / permanent USE / DROP / container REMV
            if !acted && (hungry || starving) {
                let st = profession_state.entry(conn_id).or_default();
                // PATH-REACH-MERGE: maps already pulled at think start
                settle_npc_pending_food_action(&content, &nearby, &p, st);
                if let Some(food) = resolve_npc_food_target(
                    &content,
                    &nearby,
                    p.x,
                    p.y,
                    p.food,
                    p.food_max,
                    &st.path_reach,
                    &mut st.food_goto,
                ) {
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
                        // Helper Some â‡’ Haxe isPickingupFood return true (tick consumed)
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

            // --- 1c. attackPlayer (AI-ATTACK-PLAYER) ---
            // Haxe: doStuff && attackPlayer(playerTarget) ~591 after pickup food / temp
            if !acted {
                let views_g = player_views.read().ok();
                let animals_g = animals.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let nearby_food = nearby.iter().any(|o| food_at(&content, o.id) > 0);
                    let st = profession_state.entry(conn_id).or_default();
                    let input = npc_fill_live_sensor_input(
                        &p,
                        content.as_ref(),
                        views,
                        animals_g.as_deref(),
                        st,
                        nearby_food,
                    );
                    let bundle = fill_live_sensors(&input);
                    if bundle.sensors.do_stuff && bundle.sensors.combat_target {
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
                            conn_id,
                            &p,
                            st,
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
            if !acted {
                let views_g = player_views.read().ok();
                if let Some(views) = views_g.as_ref() {
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
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
                                            seq: None,
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
                    let peer_blocked_by_ai =
                        npc_merged_blocked_by_ai(&blocked_by_ai, &craft_progress, conn_id);
                    let tiles = {
                        let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                        let raw = npc_scan_cached_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
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
                        let intent = npc_expand_craft_product(
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
            if !acted {
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
                let color = person_color_from_race(0);
                let (home_stock, has_loom) = {
                    let w = world.read().unwrap();
                    (
                        home_cloth_stock_from_world(&w, p.home_x, p.home_y, 60),
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
                    bow_old_enough: p.age >= 3.0,
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
                        craft_radius.min(60).max(8)
                    };
                    let peer_blocked_by_ai =
                        npc_merged_blocked_by_ai(&blocked_by_ai, &craft_progress, conn_id);
                    let tiles = {
                        let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
                        let raw = npc_scan_cached_rw(
                            st,
                            world.as_ref(),
                            content.as_ref(),
                            p.x,
                            p.y,
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
                                npc_expand_craft_intent(
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
                                npc_expand_craft_product(
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
                                npc_expand_craft_product(
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
                                npc_expand_craft_intent(
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
                        ClothingCraftPlan::CraftItem(object_id) => npc_expand_craft_product(
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
                        ),
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
                            npc_expand_craft_product(
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
                        )
                    };
                    if let Some(old) = old_max_r {
                        st.craft_rt.item.max_search_radius = old;
                    }
                    if committed {
                        acted = true;
                        if detail.is_empty() || detail == "idle" {
                            detail = format!("clothing_craft {plan:?}");
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
                    let rung = if sticky.has_assigned_job() {
                        PriorityRung::AssignedJob
                    } else {
                        PriorityRung::AgeRotatedJob
                    };
                    let steps = plan_profession_ladder_steps(rung, &sticky);
                    let scan_r = steps
                        .iter()
                        .map(|s| match s.kind {
                            ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,
                            ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,
                            ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,
                            ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,
                            ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,
                            ProfessionScanKind::FireFood => FIRE_FOOD_HOME_RADIUS,
                            ProfessionScanKind::HandlingFire => HANDLING_FIRE_COUNT_RADIUS,
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
                    let peer_blocked_by_ai =
                        npc_merged_blocked_by_ai(&blocked_by_ai, &craft_progress, conn_id);
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
                        was_idle: if sticky.has_sticky_profession() { 0.0 } else { 1.0 },
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
                        peer_count_by_kind: Some(peer_count_by_kind),
                        // Haxe: storeInQuiver clothingObjects (DROP-HELD-QUIVER)
                        clothing: p.clothing,
                        clothing_uses: p.clothing_uses,
                        fire_place_id,
                        fire_place_x,
                        fire_place_y,
                        feeding_cands: Vec::new(),
                        held_food_value: 0,
                        feeder_is_fertile: false,
                        feeder_is_smith: false,
                        feeder_p_id: p.p_id,
                        person_color: person_color_from_race(0),
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
                    let result = ladder_profession_scan_tick(
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
                            ShortCraftLiveIntent::UseAt {
                                x,
                                y,
                                target_id,
                                actor_id,
                            } => {
                                let dist = (x - p.x).abs().max((y - p.y).abs());
                                if dist <= 1 {
                                    if npc_use_at(&intent_tx, conn_id, x, y, None, None)
                                    {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_use actor={actor_id} target={target_id} @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 500;
                                        acted = true;
                                    }
                                } else {
                                    // Walk toward shortCraft target (same as craft walk).
                                    // AI-ANIMAL-GOTO: animal footprints + dual-pass fail mark
                                    let walked = {
                        let w = world.read().unwrap();
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
                        )
                    };
                    if walked {
                                            kind = NpcActivityKind::Craft;
                                            detail = format!(
                                                "prof_walk target={target_id} @{},{} dist={} rung={}",
                                                x,
                                                y,
                                                dist,
                                                rung.as_label()
                                            );
                                            game_ms = 250;
                                            acted = true;
                    } else {
                                        // PATH-REACH / AI-ANIMAL-GOTO: dual-pass hostile vs notReachable
                                        // Haxe: AiHelper.gotoAdv ~1116â€“1141
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
                            }
                            ShortCraftLiveIntent::UseOnEmptyGround { x, y, held } => {
                                let dist = (x - p.x).abs().max((y - p.y).abs());
                                if dist <= 1 {
                                    if npc_use_at(&intent_tx, conn_id, x, y, None, None)
                                    {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_use_ground held={held} @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 500;
                                        acted = true;
                                    }
                                } else if {
                let w = world.read().unwrap();
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
                )
            } {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_walk_ground @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 250;
                                        acted = true;
                    } else {
                                    // PATH-REACH / AI-ANIMAL-GOTO dual-pass
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
                            ShortCraftLiveIntent::DropAt { x, y } => {
                                let dist = (x - p.x).abs().max((y - p.y).abs());
                                if dist <= 1 {
                                    if npc_drop_at(&intent_tx, conn_id, x, y, None)
                                    {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_drop @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 400;
                                        acted = true;
                                    }
                                } else if {
                let w = world.read().unwrap();
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
                )
            } {
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_walk_drop @{},{} rung={}",
                                            x,
                                            y,
                                            rung.as_label()
                                        );
                                        game_ms = 250;
                                        acted = true;
                    } else {
                                    // PATH-REACH / AI-ANIMAL-GOTO dual-pass
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
                                let peer_blocked_by_ai = npc_merged_blocked_by_ai(
                                    &blocked_by_ai,
                                    &craft_progress,
                                    conn_id,
                                );
                                let blocked =
                                    st.path_reach.blocked_coords(Some(&peer_blocked_by_ai));
                                let empty_drop = Some((p.x, p.y));
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
                                    ShortCraftLiveIntent::Wait => {
                                        // PREFER-SHORT-WAIT: hold tick while moving
                                        kind = NpcActivityKind::Craft;
                                        detail = format!(
                                            "prof_goc_wait rung={}",
                                            rung.as_label()
                                        );
                                        game_ms = 200;
                                        acted = true;
                                    }
                                    ShortCraftLiveIntent::UseAt { x, y, .. }
                                    | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } => {
                                        let (log_actor, log_target) = match resolved {
                                            ShortCraftLiveIntent::UseAt {
                                                actor_id,
                                                target_id,
                                                ..
                                            } => (actor_id, target_id),
                                            ShortCraftLiveIntent::UseOnEmptyGround {
                                                held, ..
                                            } => (held, 0),
                                            _ => (0, 0),
                                        };
                                        let dist =
                                            (x - p.x).abs().max((y - p.y).abs());
                                        if dist <= 1 {
                                            if npc_use_at(&intent_tx, conn_id, x, y, None, None)
                                            {
                                                kind = NpcActivityKind::Craft;
                                                detail = format!(
                                                    "prof_goc_use actor={log_actor} target={log_target} @{},{} rung={}",
                                                    x,
                                                    y,
                                                    rung.as_label()
                                                );
                                                game_ms = 500;
                                                acted = true;
                                            }
                                        } else if {
                let w = world.read().unwrap();
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
                )
            } {
                                                kind = NpcActivityKind::Craft;
                                                detail = format!(
                                                    "prof_goc_walk_use @{},{} rung={}",
                                                    x,
                                                    y,
                                                    rung.as_label()
                                                );
                                                game_ms = 250;
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
                                    ShortCraftLiveIntent::DropAt { x, y }
                                    | ShortCraftLiveIntent::Goto { x, y }
                                    | ShortCraftLiveIntent::PickupNearForge {
                                        x,
                                        y,
                                        ..
                                    } => {
                                        let dist =
                                            (x - p.x).abs().max((y - p.y).abs());
                                        let is_drop = matches!(
                                            resolved,
                                            ShortCraftLiveIntent::DropAt { .. }
                                                | ShortCraftLiveIntent::PickupNearForge {
                                                    ..
                                                }
                                        );
                                        if dist <= 1 && is_drop {
                                            // PickupLoose maps to DropAt on object tile
                                            // (swap/pickup). Empty-hand USE when DropAt
                                            // is pile residual is rare here â€” Prefer DROP.
                                            if npc_drop_at(&intent_tx, conn_id, x, y, None)
                                            {
                                                kind = NpcActivityKind::Craft;
                                                detail = format!(
                                                    "prof_goc_drop @{},{} rung={}",
                                                    x,
                                                    y,
                                                    rung.as_label()
                                                );
                                                game_ms = 400;
                                                acted = true;
                                            }
                                        } else if dist > 1 {
                                            if {
                let w = world.read().unwrap();
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
                )
            } {
                                                    kind = NpcActivityKind::Craft;
                                                    detail = format!(
                                                        "prof_goc_walk @{},{} rung={}",
                                                        x,
                                                        y,
                                                        rung.as_label()
                                                    );
                                                    game_ms = 250;
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
                                    }
                                    ShortCraftLiveIntent::GotoForge {
                                        forge_x,
                                        forge_y,
                                        ..
                                    } => {
                                        let dist = (forge_x - p.x)
                                            .abs()
                                            .max((forge_y - p.y).abs());
                                        if dist > 1 {
                                            if {
                let w = world.read().unwrap();
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
                )
            } {
                                                    kind = NpcActivityKind::Craft;
                                                    detail = format!(
                                                        "prof_goc_forge @{},{}",
                                                        forge_x, forge_y
                                                    );
                                                    game_ms = 250;
                                                    acted = true;
                                                }
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

            // --- 2c. Smart dropHeld for peels/chips at feet (DROP-HELD-LIVE residual) ---
            // Haxe: force dropOnStart=false for Banana Peel / Sharp Stone / Flint Chipâ€¦
            if !acted && !starving && p.held_id != 0 && force_drop_at_feet(p.held_id) {
                let tiles = {
                    let st = profession_state.entry(conn_id).or_default();
                    npc_scan_cached_rw(st, world.as_ref(), content.as_ref(), p.x, p.y, 8)
                };
                // Haxe: storeInQuiver clothingObjects scan (DROP-HELD-TABLE snapshot)
                let mut drop_extras = DropHeldSensorExtras::default();
                drop_extras.quiver =
                    quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
                drop_extras.held_contains_clay = p.held_contains_clay;
                let intent = smart_drop_held_from_sensors_ex(
                    p.held_id,
                    p.held_uses.max(1),
                    p.x,
                    p.y,
                    p.x,
                    p.y,
                    p.food,
                    p.moving, // PREFER-SHORT-WAIT: isMoving â†’ BusyMoving â†’ Wait
                    false,
                    1.0, // drop close to player
                    &tiles,
                    drop_extras,
                    Some(content.as_ref()),
                );
                match intent {
                    ShortCraftLiveIntent::DropAt { x, y } => {
                        let dist = (x - p.x).abs().max((y - p.y).abs());
                        if dist <= 1 {
                            if npc_drop_at(&intent_tx, conn_id, x, y, None)
                            {
                                kind = NpcActivityKind::Craft;
                                detail = format!("smart_drop_feet held={} @{},{}", p.held_id, x, y);
                                game_ms = 400;
                                acted = true;
                            }
                        } else {
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
                                )
                            };
                            if walked {
                                kind = NpcActivityKind::Craft;
                                detail = format!("smart_drop_walk @{},{}", x, y);
                                game_ms = 250;
                                acted = true;
                            }
                        }
                    }
                    ShortCraftLiveIntent::UseAt { x, y, .. }
                    | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } => {
                        if npc_use_at(&intent_tx, conn_id, x, y, None, None)
                        {
                            kind = NpcActivityKind::Craft;
                            detail = format!("smart_drop_use held={} @{},{}", p.held_id, x, y);
                            game_ms = 400;
                            acted = true;
                        }
                    }
                    // Haxe: isMoving return true â€” hold tick (PREFER-SHORT-WAIT)
                    ShortCraftLiveIntent::Wait => {
                        kind = NpcActivityKind::Craft;
                        detail = format!("smart_drop_wait_busy held={}", p.held_id);
                        game_ms = 200;
                        acted = true;
                    }
                    _ => {}
                }
            }

            // --- 3. Bottom-up craft valuation (tools/food priority in craft_value) ---
            // Haxe: after isPickingupFood / isConsideringMakingFood; clothing/makeStuff
            // still run when hungry if those returned false (no nearby food).
            if !acted {
                let max_craft_dist = if hungry {
                    12
                } else if p.food < p.food_max * 0.6 {
                    24
                } else {
                    craft_radius
                };
                let options = evaluate_nearby_crafts(
                    &content,
                    p.x,
                    p.y,
                    p.held_id,
                    &nearby,
                    profession,
                    hungry,
                    food_need,
                    ally_count,
                    DEFAULT_WALK_SPEED,
                    INTERACTION_SEC,
                    max_craft_dist.min(craft_radius),
                );
                let blocked = craft_blacklist.get(&conn_id).cloned().unwrap_or_default();
                let best = options.iter().find(|o| {
                    if o.net_score <= 0.1 {
                        return false;
                    }
                    let key = format!("{}+{}", o.actor_id, o.target_id);
                    if blocked.contains_key(&key) {
                        return false;
                    }
                    let (gx, gy) = if o.actor_id != 0
                        && o.actor_id != p.held_id
                        && (o.actor_x != p.x || o.actor_y != p.y)
                    {
                        (o.actor_x, o.actor_y)
                    } else {
                        (o.target_x, o.target_y)
                    };
                    let dist = (gx - p.x).abs().max((gy - p.y).abs());
                    dist <= max_craft_dist
                });
                if let Some(best) = best {
                    let key = format!("{}+{}", best.actor_id, best.target_id);
                    tracker.note_craft_key(key.clone());
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::CraftPlan,
                        0,
                        (best.time_cost_sec * 1000.0) as u32,
                        format!(
                            "plan {} score={:.1} time={:.1}s prod={}/{} in={:.1}",
                            key,
                            best.net_score,
                            best.time_cost_sec,
                            best.new_actor_id,
                            best.new_target_id,
                            best.input_value
                        ),
                    );

                    let (gx, gy) = if best.actor_id != 0
                        && best.actor_id != p.held_id
                        && (best.actor_x != p.x || best.actor_y != p.y)
                    {
                        (best.actor_x, best.actor_y)
                    } else {
                        (best.target_x, best.target_y)
                    };
                    let dist = (gx - p.x).abs().max((gy - p.y).abs());

                    // Abandon only if distance gets *worse*, or stuck long without improvement.
                    let abandon = if let Some(((tx, ty), best_d)) = craft_progress.get(&conn_id).copied()
                    {
                        if tx == gx && ty == gy {
                            if dist < best_d {
                                craft_progress.insert(conn_id, ((gx, gy), dist));
                                npc_claim_tile(&blocked_by_ai, gx, gy);
                                false
                            } else if dist > best_d + 2 {
                                true // wandered away
                            } else {
                                // Same or slight stall: allow more multi-step walks.
                                tracker.same_action_count >= 15
                            }
                        } else {
                            craft_progress.insert(conn_id, ((gx, gy), dist));
                            npc_claim_tile(&blocked_by_ai, gx, gy);
                            false
                        }
                    } else {
                        craft_progress.insert(conn_id, ((gx, gy), dist));
                        npc_claim_tile(&blocked_by_ai, gx, gy);
                        false
                    };
                    // Prefer USE when adjacent even if craft_loop flagged (arrival after walk spam).
                    if dist <= 1 {
                        if intent_tx
                            .try_send(NetIntent::Use {
                                conn_id,
                                x: best.target_x,
                                y: best.target_y,
                                id: None,
                                index: None,
                            })
                            .is_ok()
                        {
                            kind = NpcActivityKind::Craft;
                            detail = format!(
                                "use craft {}->{}/{} score={:.1}",
                                key, best.new_actor_id, best.new_target_id, best.net_score
                            );
                            game_ms = (best.time_cost_sec * 1000.0) as u32;
                            acted = true;
                            craft_progress.remove(&conn_id);
                            tracker.craft_ring.clear();
                            tracker.same_action_count = 0;
                        }
                    } else if abandon || tracker.craft_loop() {
                        craft_blacklist
                            .entry(conn_id)
                            .or_default()
                            .insert(key.clone(), 25);
                        craft_progress.remove(&conn_id);
                        tracker.craft_ring.clear();
                        log_ev(
                            &activity,
                            conn_id,
                            &p,
                            NpcActivityKind::StuckCycle,
                            0,
                            0,
                            format!("blacklist craft {key} dist={dist}"),
                        );
                    } else {
                        // Multi-step path + sticky goal (Haxe useTarget while moving).
                        let walked = {
                            let w = world.read().unwrap();
                            let st = profession_state.entry(conn_id).or_default();
                            let goal_obj = if gx == best.actor_x && gy == best.actor_y {
                                best.actor_id
                            } else {
                                best.target_id
                            };
                            let expect = sticky_parent_id(&content, goal_obj);
                            npc_try_walk_to_sticky(
                                &intent_tx,
                                &w,
                                &content,
                                st,
                                conn_id,
                                p.x,
                                p.y,
                                gx,
                                gy,
                                p.food,
                                expect,
                                format!("walk_craft {key}"),
                            )
                        };
                        if walked {
                            kind = NpcActivityKind::Craft;
                            detail = format!("walk_craft {} @{},{} dist={}", key, gx, gy, dist);
                            game_ms = 250;
                            acted = true;
                        }
                    }
                }
            }

            // --- 4. Baby nurse: TimeHelper breast-feeds while holding; pickup is 1a. ---

            // --- 5. Combat: HIT nearby non-allied low-food adults when hunter ---
            if !acted && matches!(profession, CraftProfession::Hunter) && !hungry {
                let prey = {
                    let views = player_views.read().unwrap();
                    views
                        .values()
                        .filter(|o| {
                            !o.deleted
                                && o.conn_id != conn_id
                                && o.age >= 14.0
                                && (o.x - p.x).abs().max((o.y - p.y).abs()) <= 2
                                && !o.email.contains("npc-forager")
                        })
                        .min_by_key(|o| (o.x - p.x).abs().max((o.y - p.y).abs()))
                        .map(|o| o.p_id)
                };
                if let Some(tid) = prey {
                    if intent_tx
                        .try_send(NetIntent::Raw {
                            conn_id,
                            tag: "SAY".into(),
                            payload: format!("HIT {tid}"),
                        })
                        .is_ok()
                    {
                        kind = NpcActivityKind::Combat;
                        detail = format!("hit p_id={tid}");
                        game_ms = 400;
                        acted = true;
                    }
                }
            }

            // --- 6. Explore (multi-step wander; animal footprint when gate open) ---
            // Haxe: Goto / CreateCollisionChunk considerAnimal on wander
            if !acted {
                let dirs = [(6i32, 0), (0, 6), (-6, 0), (0, -6), (4, 4), (-4, 4)];
                let (odx, ody) = dirs[(tick as usize + i as usize) % dirs.len()];
                let gx = p.x + odx;
                let gy = p.y + ody;
                let walked = {
                    let w = world.read().unwrap();
                    let st = profession_state.entry(conn_id).or_default();
                    npc_try_walk_to_sticky(
                        &intent_tx,
                        &w,
                        &content,
                        st,
                        conn_id,
                        p.x,
                        p.y,
                        gx,
                        gy,
                        p.food,
                        0, // pure walk â€” no object invalidation mid-path
                        format!("explore {gx},{gy}"),
                    )
                };
                if walked {
                    kind = NpcActivityKind::Explore;
                    detail = format!("explore toward {},{}", gx, gy);
                    game_ms = 250;
                    acted = true;
                }
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
                        seq: None,
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
    fn haxe_skip_mid_path_only_when_not_yet_a_new_tile() {
        // Haxe: skip iff still moving AND this think has not arrived on a new tile.
        assert!(haxe_skip_mid_path_think(true, false));
        assert!(!haxe_skip_mid_path_think(true, true));
        assert!(!haxe_skip_mid_path_think(false, false));
        assert!(!haxe_skip_mid_path_think(false, true));
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
            label: "walk".into(),
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
}
