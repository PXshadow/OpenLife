//! Basic AI NPC scheduler (Haxe `AiBase.RunAi` shape — single thread).
//!
//! Priority: eat if hungry → seek food → **profession ladder scan** (farm/smith/baker/
//! pottery/shepherd shortCraft USE/DROP) → craft (bottom-up valuation) → explore.
//! Activity logged in RAM and flushed every 30s ([`npc_activity`]).
//!
//! // Haxe: ServerAi.doTimeStuff → AiBase.doTimeStuffHelper AssignedJob / AgeRotatedJob

use crate::npc_activity::{
    NpcActivityEvent, NpcActivityKind, NpcActivityLog, NpcStuckTracker,
};
use ol_config::{gameplay_defaults, LiveSettings};
use ol_content::ContentDb;
use ol_metrics::{Counters, ScopeTimer};
use ol_ai::{IntentSink, PlayerCommands, DEFAULT_FOOD_SEARCH_RADIUS};
use ol_net::NetIntent;
use ol_sim::{
    apply_food_goto_fail, apply_path_filters_to_tiles, basic_farmer_weight_from_runtime,
    collect_deadly_animal_blocked_around, consider_animals_for_goto, evaluate_nearby_crafts,
    force_drop_at_feet, food_pickup_action_success_reset, full_pile_tiles_from_scan,
    get_or_craft_objs_from_scan, goto_path_outcome, has_bean_seeds_from_scan,
    has_carrot_seeds_from_scan, is_walkable, is_walkable_with_animals, is_wound_object,
    ladder_profession_scan_tick, mark_food_path_fail, mark_goto_path_fail, merge_path_reach_maps,
    next_step, next_step_consider_animals, npc_enqueue_get_or_craft_ex, npc_peer_count_for_kind,
    path_filters_from_player, peer_home_coords, peer_is_wounded_from_held,
    pending_food_tile_still_actionable, pile_obj_id_from_content, plan_goto_obj,
    plan_is_picking_up_food, plan_profession_ladder_steps, quiver_from_clothing_snapshot,
    resolve_sticky_food, scan_world_radius, self_clothing_raw_payload,
    settle_pending_food_use_fail, smart_drop_held_from_sensors, AiPathReachMaps,
    BakerProfessionRuntime, BakerTaskState, CraftAiRuntime, CraftLiveExpandOpts, CraftProfession,
    DropHeldSensorExtras, FarmProfession, FarmProfessionRuntime, FarmTaskState,
    FireFoodProfessionRuntime, FireKeeperProfessionRuntime, GotoObjPlan, GotoPathOutcome,
    IsPickingupFoodInput, IsPickingupFoodPlan, LastGotoObj, NearbyObj, NpcProfessionPeerRow,
    PlayerSnapshot, PotterProfessionRuntime, PrestigeClass, PriorityRung, ProfessionScanInput,
    ProfessionScanKind, ProfessionStickySnapshot, ReverseCraftGraph, ShepherdProfessionRuntime,
    ShortCraftLiveIntent, SmithProfessionRuntime, SteelChiselFamilyTable, StickyFoodTarget,
    BAKER_SCAN_RADIUS, DEFAULT_CRAFT_RADIUS, DEFAULT_PROFESSION_SCAN_RADIUS, DEFAULT_WALK_SPEED,
    FIRE_FOOD_HOME_RADIUS, GOTO_COLLISION_RAD, HANDLING_FIRE_COUNT_RADIUS, INTERACTION_SEC, MAX_AGE,
    MIN_AGE_TO_EAT, POTTERY_SCAN_RADIUS, SHEPHERD_SHORTCRAFT_RADIUS, SMITH_SCAN_RADIUS,
};
use ol_world::World;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// `NetIntent` sink for NPC → same channel as human clients (OL-AI-SPLIT).
struct NpcIntentTx<'a>(&'a tokio::sync::mpsc::Sender<NetIntent>);

impl IntentSink for NpcIntentTx<'_> {
    fn push(&mut self, intent: NetIntent) -> bool {
        self.0.try_send(intent).is_ok()
    }
}

/// Enqueue USE via [`PlayerCommands`] (identical to human client intent).
#[inline]
fn npc_use_at(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    x: i32,
    y: i32,
    id: Option<i32>,
    index: Option<i32>,
) -> bool {
    NpcIntentTx(intent_tx).use_at(conn_id, x, y, id, index)
}

/// Enqueue MOVE via [`PlayerCommands`].
#[inline]
fn npc_move_path(
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    conn_id: u64,
    xs: i32,
    ys: i32,
    deltas: &[(i32, i32)],
    seq: Option<i32>,
) -> bool {
    NpcIntentTx(intent_tx).move_path(conn_id, xs, ys, deltas, seq)
}

/// Max tiles per NPC MOVE commit.
///
/// Walk speed ≈ 3.75 tiles/s → 16 steps ≈ 4.3s, spanning ≥1 think skip when
/// `ai_think_period_ticks` is 15–20 (3–4s). While `PlayerSnapshot.moving` is
/// true the scheduler skips that NPC — fewer accepts, less log spam.
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
) -> Option<(i32, i32)> {
    // Haxe: considerAnimals = checkIfDangerous && didNotReachFood < 5 && food_store > -1
    let consider = consider_animals_for_goto(true, did_not_reach_food, food_store);
    next_step_consider_animals(world, content, sx, sy, gx, gy, consider)
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
    );
    if ok {
        set_sticky_move(st, gx, gy, expected_parent_id, label);
    }
    ok
}

/// Dual-pass Goto fail mark: animal-only block → hostile_path 20s; else not_reachable 90s.
// Haxe: AiHelper.gotoAdv ~1116–1141
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
// Haxe: single AiBase maps — Rust dual ownership → merge each think
// PATH-REACH-MERGE / dual_map_merge
fn pull_player_path_reach(st: &mut NpcProfessionState, snap: &PlayerSnapshot) {
    merge_path_reach_maps(&mut st.path_reach, &snap.ai_path_reach);
}

/// PATH-REACH-MERGE: push NPC maps into player_views for tick_vitals absorb.
// Haxe: AiBase L85–86 single maps
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

/// Sticky isPickingupFood / gotoObj bookkeeping (AI-GOTO-FOOD).
// Haxe: AiBase.foodTarget / lastGotoObj / didNotReachFood
#[derive(Debug, Clone)]
struct NpcFoodGotoState {
    sticky_food: Option<StickyFoodTarget>,
    last_goto: Option<LastGotoObj>,
    last_goto_dist: f32,
    did_not_reach_food: f32,
    /// Async food USE/DROP/REMV tile awaiting apply result (AI-FOOD-FAIL-MARK).
    // Haxe: isPickingupFood use/remove/drop returns false sync → mark 30s
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
// Haxe: AiBase.useTarget + expectedUseTarget; isUsingItem target-changed → CancleUse
#[derive(Debug, Clone)]
struct NpcStickyMove {
    gx: i32,
    gy: i32,
    /// World parent id expected at goal; `0` = walk-only (no object check).
    expected_parent_id: i32,
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
    /// PATH-REACH: Haxe AiBase notReachableObjects / objectsWithHostilePath (npc-local).
    // Haxe: AiBase L85–86; AiHelper.gotoAdv fail → addNotReachable / addHostilePath
    path_reach: AiPathReachMaps,
    /// AI-GOTO-FOOD: sticky foodTarget + lastGotoObj + didNotReachFood.
    // Haxe: AiBase.foodTarget / lastGotoObj / didNotReachFood
    food_goto: NpcFoodGotoState,
    /// AI-CRAFT-NPC-ENQUEUE: sticky multi-step craftItem state (failedCraftings / itemToCraft).
    // Haxe: AiBase.itemToCraft + failedCraftings across GetOrCraftItem / craftItem ticks
    craft_rt: CraftAiRuntime,
    /// Haxe `AiBase.time` — reaction cooldown (seconds). Think only when ≤ 0.
    // Haxe: AiBase.time / doTimeStuff
    think_time_sec: f32,
    /// Haxe `lineage.prestigeClass` for reaction time selection.
    // Haxe: PrestigeClass Serf / Commoner / Noble
    prestige_class: PrestigeClass,
    /// True once role prestige class has been assigned for this body.
    class_assigned: bool,
    /// Active MOVE goal — validated while `PlayerSnapshot.moving`.
    sticky_move: Option<NpcStickyMove>,
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
            path_reach: AiPathReachMaps::default(),
            food_goto: NpcFoodGotoState::default(),
            craft_rt: CraftAiRuntime::default(),
            think_time_sec: 0.0,
            prestige_class: PrestigeClass::Commoner,
            class_assigned: false,
            sticky_move: None,
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
        }
    }
}

impl NpcConfig {
    /// Map live hot-reload knobs → NPC scheduler config.
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
/// Forager→Serf, Farmer→Commoner, Hunter→Noble.
// Haxe: lineage.prestigeClass at birth (account score); NPCs get role-mapped class
fn prestige_class_for_npc_index(i: u32) -> PrestigeClass {
    match i % 3 {
        0 => PrestigeClass::Serf,
        1 => PrestigeClass::Commoner,
        _ => PrestigeClass::Noble,
    }
}

/// Haxe milkweed family may change 50→51→52 without cancelling use.
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
    st.sticky_move = Some(NpcStickyMove {
        gx,
        gy,
        expected_parent_id,
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
        // Forager/Hunter/Generic: age-rotated multi-profession (NPC-SCAN-FULL).
        CraftProfession::Forager
        | CraftProfession::Hunter
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

/// Find nearest edible ground object (food_value > 0), skipping path-reach blocks.
///
/// Candidates must already be gathered within [`DEFAULT_FOOD_SEARCH_RADIUS`] (30)
/// tiles (see SeekFood `scan_world_radius`). Full `FoodSearch`/`search_best_food_full`
/// lives on the sim writer path via `ol_sim::best_food_for_ai`.
// Haxe: SearchBestFood skips isObjectNotReachable / isObjectWithHostilePath
// OL-AI-SPLIT: FoodSearch surface (default r=30)
fn nearest_food(
    content: &ContentDb,
    nearby: &[NearbyObj],
    px: i32,
    py: i32,
    path_reach: Option<&AiPathReachMaps>,
) -> Option<NearbyObj> {
    let r = DEFAULT_FOOD_SEARCH_RADIUS;
    nearby
        .iter()
        .filter(|o| food_at(content, o.id) > 0)
        .filter(|o| {
            let d = (o.x - px).abs().max((o.y - py).abs());
            d <= r
        })
        .filter(|o| {
            path_reach
                .map(|m| !m.blocks_target(o.x, o.y, None))
                .unwrap_or(true)
        })
        .min_by_key(|o| (o.x - px).abs().max((o.y - py).abs()))
        .copied()
}


/// After prior food USE/DROP/REMV was sent, mark 30s if still empty-handed and tile food.
// Haxe: isPickingupFood done==false → addNotReachableObject(food, 30) (AI-FOOD-FAIL-MARK)
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
        // Async success: picked something up — clear sticky + reset didNotReachFood.
        // Haxe: ~8703–8704 (only after done==true; Rust settles next tick)
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
        // Tile gone (someone ate it) — clear sticky without mark.
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
    // Ground nearest only (container slots need SearchBestFood live — residual).
    let cand = nearest_food(content, nearby, px, py, Some(path_reach)).map(|f| {
        StickyFoodTarget::new(f.x, f.y, f.id)
    });
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
// Haxe: AiBase.isPickingupFood ~8610–8706 (AI-PICKUP-FOOD)
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
        false, // holding_player residual (snapshot lacks holding_player_id)
        drop_would,
    ));

    // DropHeldBeforeMove: try smart drop; if no actionable intent, replan without drop_would.
    if let IsPickingupFoodPlan::DropHeldBeforeMove {
        max_distance_to_home,
    } = plan
    {
        let tiles = {
            let w = world.read().unwrap();
            // OL-AI-SPLIT: FoodSearch default radius 30 (was 12)
            scan_world_radius(&w, Some(content), p.x, p.y, DEFAULT_FOOD_SEARCH_RADIUS)
        };
        let mut drop_extras = DropHeldSensorExtras::default();
        drop_extras.quiver = quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
        drop_extras.has_food_target = true;
        let intent = smart_drop_held_from_sensors(
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
        // dropHeld returned none — continue SM with held still in hand
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
            false,
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
                GotoObjPlan::AbortReceding { .. } => {
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
                        npc_try_walk_to(
                            intent_tx,
                            &w,
                            content,
                            conn_id,
                            p.x,
                            p.y,
                            food.x,
                            food.y,
                            p.food,
                            st.food_goto.did_not_reach_food,
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
            // Residual: PlayerSnapshot lacks holding_player_id — treat as busy.
            Some((
                NpcActivityKind::SeekFood,
                "drop_held_player_residual".into(),
                100,
            ))
        }
        IsPickingupFoodPlan::DropHeldForPickup => {
            let tiles = {
                let w = world.read().unwrap();
                // OL-AI-SPLIT: FoodSearch default radius 30 (was 12)
            scan_world_radius(&w, Some(content), p.x, p.y, DEFAULT_FOOD_SEARCH_RADIUS)
            };
            let mut drop_extras = DropHeldSensorExtras::default();
            drop_extras.quiver = quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
            drop_extras.has_food_target = true;
            let intent = smart_drop_held_from_sensors(
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
            // Can't drop — mark food fail 30s (can't USE/REMV with hands full)
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
            if intent_tx
                .try_send(NetIntent::Raw {
                    conn_id,
                    tag: "REMV".into(),
                    payload,
                })
                .is_ok()
            {
                // Keep sticky until next-tick settle (Haxe clears only after known done).
                // Haxe: isPickingupFood ~8694–8704
                st.food_goto.pending_food_xy = Some((x, y));
                st.food_goto.pending_food_container = true;
                return Some((
                    NpcActivityKind::SeekFood,
                    format!("remv_food id={} idx={} @{},{}", food.parent_id, index, x, y),
                    500,
                ));
            }
            // try_send fail → 30s
            mark_food_path_fail(&mut st.path_reach, x, y);
            st.food_goto.sticky_food = None;
            Some((
                NpcActivityKind::SeekFood,
                format!("remv_food_fail mark30 @{},{}", x, y),
                100,
            ))
        }
        IsPickingupFoodPlan::Use { x, y } => {
            if intent_tx
                .try_send(NetIntent::Use {
                    conn_id,
                    x,
                    y,
                    id: None,
                    index: None,
                })
                .is_ok()
            {
                // Async apply; settle next tick marks 30s if still empty + tile food.
                // Haxe: isPickingupFood ~8694–8704 (no optimistic clear)
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
            if intent_tx
                .try_send(NetIntent::Drop {
                    conn_id,
                    x,
                    y,
                    c: None,
                })
                .is_ok()
            {
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
                if intent_tx
                    .try_send(NetIntent::Drop {
                        conn_id,
                        x,
                        y,
                        c: None,
                    })
                    .is_ok()
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
                if intent_tx
                    .try_send(NetIntent::Use {
                        conn_id,
                        x,
                        y,
                        id: None,
                        index: None,
                    })
                    .is_ok()
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
) {
    let labels = ["npc-forager", "npc-farmer", "npc-hunter"];
    let mut tick: u64 = 0;
    let mut active: u32 = 0;
    let mut target_pop: u32 = 0;
    let mut stuck_map: HashMap<u64, NpcStuckTracker> = HashMap::new();
    /// conn → (craft_key, remaining_cooldown_thinks)
    let mut craft_blacklist: HashMap<u64, HashMap<String, u32>> = HashMap::new();
    /// conn → (target_xy, best_dist_seen) for progress tracking
    let mut craft_progress: HashMap<u64, ((i32, i32), i32)> = HashMap::new();
    /// conn → sticky farm/smith/baker task state for profession ladder scan.
    let mut profession_state: HashMap<u64, NpcProfessionState> = HashMap::new();
    let mut announced = false;
    // AI-JOB-SMITH-RESID: load-time objectIdArrays[455] Chisel cache (PatchObjectData once)
    // Haxe: ServerSettings.PatchObjectData ~612–616
    let chisel_table = SteelChiselFamilyTable::from_content(content.as_ref());
    let chisel_family_extra = chisel_table.extras.clone();

    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
        tick = tick.wrapping_add(1);
        activity.try_flush();

        // Same-wake as sim hot-reload: LiveSettings → NpcConfig (no outer 2 s copy).
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

        // Floor population (login missing agents up to min).
        while active < min {
            let conn_id = NPC_CONN_BASE + active as u64;
            let email = format!("{}@local", labels[active as usize % labels.len()]);
            let _ = intent_tx
                .send(NetIntent::Login {
                    conn_id,
                    reconnect: false,
                    email,
                    client_tag: "client_npc".into(),
                })
                .await;
            info!(conn_id, "npc: login requested");
            active += 1;
            target_pop = active;
        }
        if !announced {
            announced = true;
            info!(
                min,
                max, think_period, radius, craft_radius, "npc scheduler started (eat+craft+activity log)"
            );
        }

        let intents = counters.intents_applied.load(Ordering::Relaxed);
        let skips = counters.skip_ticks.load(Ordering::Relaxed);
        if skips > 0 && tick % 50 == 0 && active > min {
            active = active.saturating_sub(1).max(min);
            target_pop = active;
        } else if tick % 100 == 0 && active < max && intents < 100_000 {
            if active >= target_pop && active < max {
                active += 1;
                target_pop = active;
                let conn_id = NPC_CONN_BASE + (active - 1) as u64;
                let email = format!("npc-{active}@local");
                let _ = intent_tx
                    .send(NetIntent::Login {
                        conn_id,
                        reconnect: false,
                        email,
                        client_tag: "client_npc".into(),
                    })
                    .await;
                info!(conn_id, active, "npc: grown population");
            }
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
                let hungry = p.food < p.food_max * 0.45;
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
                // Haxe: AiBase single maps always current; dual ownership → pull each think
                // PATH-REACH-MERGE / dual_map_merge
                {
                    let st = profession_state.entry(conn_id).or_default();
                    pull_player_path_reach(st, &p);
                    st.path_reach.cleanup(0.2 * think_period as f32);
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
                            let ab = collect_deadly_animal_blocked_around(
                                &w,
                                &content,
                                p.x,
                                p.y,
                                GOTO_COLLISION_RAD,
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
                // (was missing — AI-TAKEOVER never wrote into player_views before)
                // Haxe: AiBase L85–86 single maps
                if let Some(st) = profession_state.get(&conn_id) {
                    push_npc_path_reach_to_views(&player_views, conn_id, &st.path_reach);
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
            let snap = player_views
                .read()
                .ok()
                .and_then(|g| g.get(&conn_id).cloned());
            let Some(p) = snap else {
                continue;
            };

            let tracker = stuck_map.entry(conn_id).or_default();

            // Death detection.
            if p.deleted {
                if !tracker.was_deleted {
                    tracker.was_deleted = true;
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::Death,
                        0,
                        0,
                        format!(
                            "age={:.1} food={:.1} reason=deleted_or_starved held={}",
                            p.age, p.food, p.held_id
                        ),
                    );
                    // Respawn: request new login same conn (sim may replace body).
                    let email = format!("npc-re-{}@local", i);
                    let _ = intent_tx.try_send(NetIntent::Login {
                        conn_id,
                        reconnect: false,
                        email,
                        client_tag: "client_npc".into(),
                    });
                    if let Some(st) = profession_state.get_mut(&conn_id) {
                        clear_sticky_move(st);
                        st.think_time_sec = 0.0;
                        st.class_assigned = false;
                    }
                }
                continue;
            }
            tracker.was_deleted = false;
            tracker.note_position(p.x, p.y);

            // Haxe: time += reactionTime (class-based Serf/Commoner/Noble)
            {
                let st = profession_state.entry(conn_id).or_default();
                let angry = false; // residual: isAngryOrTerrified
                st.think_time_sec += cfg.reaction_for_class(st.prestige_class, angry);
            }

            // Haxe: if (!movedOneTile && isMoving()) return — don't replan mid-path
            // unless sticky goal invalid (target parent changed / gone).
            if p.moving {
                let (still_valid, sticky_label) = {
                    let st = profession_state.entry(conn_id).or_default();
                    match st.sticky_move.clone() {
                        None => (true, String::new()),
                        Some(ref sticky) => {
                            let w = world.read().unwrap();
                            let ok = sticky_move_still_valid(&w, &content, sticky);
                            (ok, sticky.label.clone())
                        }
                    }
                };
                if still_valid {
                    continue;
                }
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
            } else if let Some(st) = profession_state.get_mut(&conn_id) {
                clear_sticky_move(st);
            }

            let profession = profession_for_index(i);
            let hungry = p.food < p.food_max * 0.45;
            let starving = p.food < p.food_max * 0.25;
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
            // Haxe: single AiBase maps always current; dual ownership → pull each think
            // PATH-REACH-MERGE / dual_map_merge
            {
                let st = profession_state.entry(conn_id).or_default();
                pull_player_path_reach(st, &p);
                st.path_reach.cleanup(0.2 * think_period as f32);
            }

            let mut acted = false;
            let mut detail = String::new();
            let mut kind = NpcActivityKind::Think;
            let mut game_ms = 200u32;

            // --- 1. Eat held food if hungry ---
            if !acted && hungry && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                // USE on self-tile fails transition → sim try_eat_held.
                if intent_tx
                    .try_send(NetIntent::Use {
                        conn_id,
                        x: p.x,
                        y: p.y,
                        id: None,
                        index: None,
                    })
                    .is_ok()
                {
                    kind = NpcActivityKind::Eat;
                    detail = format!("eat_held={}", p.held_id);
                    game_ms = 500;
                    acted = true;
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
                        // Helper Some ⇒ Haxe isPickingupFood return true (tick consumed)
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

            // --- 2a. Continuous follow walk (AI-FOLLOW-WALK) ---
            // Haxe: AiBase.isMovingToPlayer after sticky playerToFollow / LLM follow
            if !acted && !starving {
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

            // --- 2b. Profession ladder scan (NPC-CRAFT-LADDER) ---
            // Haxe: AssignedJob / AgeRotatedJob → doBasicFarming/doSmithing/doBaking → USE/DROP
            // Escape/food bands already handled above; only when not hungry-starving.
            // AI-FOLLOW-WALK: follow holds tick above when far from sticky target
            if !acted && !starving && !hungry {
                if let Some(sticky) = npc_sticky_for_craft_profession(profession, p.age) {
                    // AI-JOB-SMITH-RESID: true home from PlayerSnapshot (Haxe home.tx/ty)
                    let (home_x, home_y) =
                        peer_home_coords(Some((p.home_x, p.home_y)), p.x, p.y);
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
                        })
                        .max()
                        .unwrap_or(DEFAULT_PROFESSION_SCAN_RADIUS)
                        .min(craft_radius)
                        .max(8);
                    // Ensure sticky entry exists before peer roster (no long-lived mut borrow).
                    profession_state.entry(conn_id).or_default();
                    // PATH-REACH: filter notReachable / hostile / peer blockedByAI before picks.
                    // Haxe: cleanupBlockedObjects + isObjectNotReachable (OR blockedByAI)
                    // Peer craft targets approximate SimState.blocked_by_ai for the NPC thread
                    // (no live SimState share); human/sticky rebuild remains on sim tick.
                    let tiles = {
                        let path_reach = &profession_state
                            .get(&conn_id)
                            .expect("npc profession entry")
                            .path_reach;
                        let w = world.read().unwrap();
                        let raw =
                            scan_world_radius(&w, Some(content.as_ref()), home_x, home_y, scan_r);
                        // Haxe: blockedByAI — peer AI craft/use claims
                        let mut peer_blocked_by_ai: HashMap<(i32, i32), f32> = HashMap::new();
                        for (cid, ((tx, ty), _)) in craft_progress.iter() {
                            if *cid != conn_id {
                                peer_blocked_by_ai.insert((*tx, *ty), 5.0);
                            }
                        }
                        let filters =
                            path_filters_from_player(path_reach, &peer_blocked_by_ai);
                        apply_path_filters_to_tiles(&raw, &filters)
                    };
                    // AI-JOB-SMITH-RESID: multi-prof peer pop from snapshots + npc sticky
                    // Haxe: countProfession over Connection.getAis (home / wound / last)
                    let primary_kind = steps
                        .first()
                        .map(|s| s.kind)
                        .unwrap_or(ProfessionScanKind::Farm);
                    let peer_count = {
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
                                is_wounded: peer_is_wounded_from_held(held_wound),
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
                                    || pst
                                        .map(|s| s.farm_rt.last_profession.is_some())
                                        .unwrap_or(false),
                                last_is_fire_food: snap.is_last_fire_food
                                    || pst
                                        .map(|s| s.fire_rt.is_last_fire_food)
                                        .unwrap_or(false),
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
                                last_is_fire_food: pst.fire_rt.is_last_fire_food,
                            });
                        }
                        npc_peer_count_for_kind(
                            primary_kind,
                            &rows,
                            conn_id,
                            home_x,
                            home_y,
                            MIN_AGE_TO_EAT,
                            MAX_AGE,
                        )
                    };
                    let basic_farmer_weight = basic_farmer_weight_from_runtime(
                        &profession_state
                            .get(&conn_id)
                            .expect("npc profession entry")
                            .farm_rt,
                    );
                    let inp = ProfessionScanInput {
                        player_x: p.x,
                        player_y: p.y,
                        home_x,
                        home_y,
                        held_id: p.held_id,
                        held_uses: p.held_uses.max(1),
                        // Snapshot lacks nest; clay bare-held only (AI-POTTER residual).
                        held_contained: 0,
                        held_contains_clay: p.held_id == 126,
                        food_store: p.food,
                        transition_hungry_cost: 0.0,
                        has_carrot_seeds: has_carrot_seeds_from_scan(&tiles),
                        has_bean_seeds: has_bean_seeds_from_scan(&tiles),
                        is_hungry: false,
                        basic_farmer_weight,
                        hardened_row_biome: None,
                        // PATH-REACH: tiles already filtered via path_reach maps
                        target_reachable: true,
                        peer_count,
                        was_idle: if sticky.has_sticky_profession() { 0.0 } else { 1.0 },
                        age: p.age,
                        profession_is_sticky: sticky.has_sticky_profession(),
                        is_assigned_job: sticky.has_assigned_job(),
                        // PREFER-SHORT-WAIT (npc usually skips when p.moving; keep field)
                        is_moving: p.moving,
                        // AI-HANDLING-FIRE: Season==Winter → Fire 82 kindling first (npc residual false)
                        is_winter: false,
                        // Load-time objectIdArrays[455] cache (not re-scanned each tick)
                        chisel_family_extra: chisel_family_extra.clone(),
                    };
                    let st = profession_state.get_mut(&conn_id).expect("npc profession entry");
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
                    );
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
                                    if intent_tx
                                        .try_send(NetIntent::Use {
                                            conn_id,
                                            x,
                                            y,
                                            id: None,
                                            index: None,
                                        })
                                        .is_ok()
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
                                        // Haxe: AiHelper.gotoAdv ~1116–1141
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
                                    if intent_tx
                                        .try_send(NetIntent::Use {
                                            conn_id,
                                            x,
                                            y,
                                            id: None,
                                            index: None,
                                        })
                                        .is_ok()
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
                                    if intent_tx
                                        .try_send(NetIntent::Drop {
                                            conn_id,
                                            x,
                                            y,
                                            c: None,
                                        })
                                        .is_ok()
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
                            ShortCraftLiveIntent::SeekOrCraft { .. }
                            | ShortCraftLiveIntent::SeekGroundActor { .. }
                            | ShortCraftLiveIntent::CraftItem { .. } => {
                                // AI-CRAFT-NPC-ENQUEUE: multi-step GetOrCraft + craftItem expand
                                // with path-reach CraftScanFilters (hostile/unreachable/blockedByAI).
                                // Haxe: AiBase.GetOrCraftItem → craftItem → useTarget / dropTarget
                                // Residuals closed: live pile_id (getPileObjId), ScanTile.num_slots,
                                // ignoreFullPiles full multi-use tiles, peer blockedByAI merge.
                                let staging = result.intent;
                                // ScanTile.num_slots from ObjectDef at scan_world_radius
                                // Haxe: objectData.numSlots empty-hand container gate
                                let goc_objs = get_or_craft_objs_from_scan(&tiles, None);
                                // Haxe: ignoreFullPiles + numberOfUses >= numUses
                                let full_piles = full_pile_tiles_from_scan(&tiles);
                                // Haxe: isObjectNotReachable ORs blockedByAI
                                let mut peer_blocked_by_ai: HashMap<(i32, i32), f32> =
                                    HashMap::new();
                                for (cid, ((tx, ty), _)) in craft_progress.iter() {
                                    if *cid != conn_id {
                                        peer_blocked_by_ai.insert((*tx, *ty), 5.0);
                                    }
                                }
                                let blocked =
                                    st.path_reach.blocked_coords(Some(&peer_blocked_by_ai));
                                let empty_drop = Some((p.x, p.y));
                                let is_smith = matches!(profession, CraftProfession::Smith);
                                let opts = CraftLiveExpandOpts {
                                    home: Some((home_x, home_y)),
                                    is_or_can_smith: is_smith,
                                    now_sec: tick as f64 * 0.2,
                                    use_default_water_sources: true,
                                };
                                // Haxe: ObjectData.getPileObjId via self+self transition
                                let content_ref = content.as_ref();
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
                                            if intent_tx
                                                .try_send(NetIntent::Use {
                                                    conn_id,
                                                    x,
                                                    y,
                                                    id: None,
                                                    index: None,
                                                })
                                                .is_ok()
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
                                            // is pile residual is rare here — Prefer DROP.
                                            if intent_tx
                                                .try_send(NetIntent::Drop {
                                                    conn_id,
                                                    x,
                                                    y,
                                                    c: None,
                                                })
                                                .is_ok()
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
                                    // Residual SeekOrCraft / CraftItem / None → fall through
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
                            // Haxe: storeInQuiver → self(0,0,5) (DROP-HELD-LIVE)
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
                            // Haxe: isMoving / dropHeld return true — hold tick (PREFER-SHORT-WAIT)
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
            // Haxe: force dropOnStart=false for Banana Peel / Sharp Stone / Flint Chip…
            if !acted && !starving && p.held_id != 0 && force_drop_at_feet(p.held_id) {
                let tiles = {
                    let w = world.read().unwrap();
                    scan_world_radius(&w, Some(content.as_ref()), p.x, p.y, 8)
                };
                // Haxe: storeInQuiver clothingObjects scan (DROP-HELD-TABLE snapshot)
                let mut drop_extras = DropHeldSensorExtras::default();
                drop_extras.quiver =
                    quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
                let intent = smart_drop_held_from_sensors(
                    p.held_id,
                    p.held_uses.max(1),
                    p.x,
                    p.y,
                    p.x,
                    p.y,
                    p.food,
                    p.moving, // PREFER-SHORT-WAIT: isMoving → BusyMoving → Wait
                    false,
                    1.0, // drop close to player
                    &tiles,
                    drop_extras,
                );
                match intent {
                    ShortCraftLiveIntent::DropAt { x, y } => {
                        let dist = (x - p.x).abs().max((y - p.y).abs());
                        if dist <= 1 {
                            if intent_tx
                                .try_send(NetIntent::Drop {
                                    conn_id,
                                    x,
                                    y,
                                    c: None,
                                })
                                .is_ok()
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
                        if intent_tx
                            .try_send(NetIntent::Use {
                                conn_id,
                                x,
                                y,
                                id: None,
                                index: None,
                            })
                            .is_ok()
                        {
                            kind = NpcActivityKind::Craft;
                            detail = format!("smart_drop_use held={} @{},{}", p.held_id, x, y);
                            game_ms = 400;
                            acted = true;
                        }
                    }
                    // Haxe: isMoving return true — hold tick (PREFER-SHORT-WAIT)
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
            // When hungry, skip long walks (logs showed dist=36+ then starve/blacklist).
            if !acted && !starving {
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
                                false
                            } else if dist > best_d + 2 {
                                true // wandered away
                            } else {
                                // Same or slight stall: allow more multi-step walks.
                                tracker.same_action_count >= 15
                            }
                        } else {
                            craft_progress.insert(conn_id, ((gx, gy), dist));
                            false
                        }
                    } else {
                        craft_progress.insert(conn_id, ((gx, gy), dist));
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
                                "use craft {}→{}/{} score={:.1}",
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

            // --- 4. Feed kids (NURSE/FEED) when holding a baby + food ---
            // Holding baby is modeled as holding_player_id on sim; snapshot may not expose it.
            // Use SAY NURSE when holding food and another young player is adjacent.
            if !acted && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                let baby_near = {
                    let views = player_views.read().unwrap();
                    views.values().find(|o| {
                        !o.deleted
                            && o.conn_id != conn_id
                            && o.age < 3.0
                            && (o.x - p.x).abs().max((o.y - p.y).abs()) <= 1
                    }).map(|o| o.p_id)
                };
                if baby_near.is_some() {
                    if intent_tx
                        .try_send(NetIntent::Raw {
                            conn_id,
                            tag: "SAY".into(),
                            payload: "NURSE".into(),
                        })
                        .is_ok()
                    {
                        kind = NpcActivityKind::Feed;
                        detail = format!("nurse baby held_food={}", p.held_id);
                        game_ms = 500;
                        acted = true;
                    }
                }
            }

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
                        0, // pure walk — no object invalidation mid-path
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
            // Double-MOVE in one think (walk + stuck) was inflating accepts ~2×
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

            counters
                .ai_cpu_us
                .fetch_add(cpu as u64, Ordering::Relaxed);

            // PATH-REACH-MERGE: push NPC path maps into player_views for tick_vitals absorb
            if let Some(st) = profession_state.get(&conn_id) {
                push_npc_path_reach_to_views(&player_views, conn_id, &st.path_reach);
            }
            counters.ai_thinks.fetch_add(1, Ordering::Relaxed);
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
    fn sticky_walk_only_always_valid() {
        // expected_parent_id == 0 → pure walk, valid regardless of world contents.
        let sticky = NpcStickyMove {
            gx: 10,
            gy: 10,
            expected_parent_id: 0,
            label: "walk".into(),
        };
        assert_eq!(sticky.expected_parent_id, 0);
        // milkweed family helper
        assert!(is_milkweed_family(50));
        assert!(is_milkweed_family(51));
        assert!(!is_milkweed_family(36));
    }
}
