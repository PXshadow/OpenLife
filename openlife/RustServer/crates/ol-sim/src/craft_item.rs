//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN** + **AI-CRAFT-DUAL** + **AI-CRAFT-LIVE-MORE** + **AI-CRAFT-MULTI-SPECIALS**).
//!
//! Ports Haxe `AiBase.craftItem` / `craftItemHelper` / `searchBestObjectForCrafting`
//! against reverse-graph + world object snapshot, with top-down `DoTransitionSearch`
//! filters and hostile/unreachable scan gates ([`craft_topdown`]).
//!
//! Includes craftItemHelper specials: water/soil retarget, berry-pie gate,
//! bowl fill anti-loops, forge flat-rock / clay-bowl bias, TIME actor wait, sticky
//! [`CraftAiRuntime`] for multi-tick fail+itemToCraft state. Specials GetClosest
//! and GetCraftAndDrop pickup honor [`CraftScanFilters`]; interrupted
//! `countDone < count` re-queues onto [`CraftAiRuntime::crafting_tasks`]
//! (**AI-CRAFT-MULTI-RESID**).
//!
//! Haxe anchors:
//! - `AiBase.craftItem` ~6611–6644
//! - `AiBase.craftItemHelper` ~6646–7130
//! - `AiBase.craftItemMax` ~6604
//! - `AiBase.searchBestObjectForCrafting` ~7132–7186
//! - `AiBase.searchBestTransitionTopDown` / `DoTransitionSearch` ~7696–8039
//! - `ServerSettings.AiTimeToWaitIfCraftingFailed` / `AiMaxSearchRadius` / `AiMaxSearchIncrement`
//! - forge SMITH gate ids 304/305/303
//! - craftItemHelper specials ~6750–7037 (water, soil, forge, bowls, TIME)
//! - dual-center searchCurrentPosition + pile*1.5 / r=6 re-anchor ~7050–7242 (AI-CRAFT-DUAL)
//! - GetCraftAndDropItemsCloseToObj adze/froe/goose/kindling + fillBucket residual (AI-CRAFT-LIVE-MORE)
//! - `ServerSettings.InitWaterSourceIds` → WaterSourceIds / BucketWaterSourceIds (AI-CRAFT-MULTI-SPECIALS)
//! - specials GetClosest + CraftAiRuntime.craftingTasks (AI-CRAFT-MULTI-RESID)

use std::collections::{HashMap, HashSet, VecDeque};

use crate::craft_graph::ReverseCraftGraph;
use crate::short_craft_intent::drop_held_ai::consider_drop_held_object;
use crate::short_craft_intent::ShortCraftLiveIntent;

// Haxe: searchBestTransitionTopDown / DoTransitionSearch (AI-CRAFT-TOPDOWN)
#[path = "craft_topdown.rs"]
mod craft_topdown;
// Re-export top-down surface for callers; allow unused within this module.
#[allow(unused_imports)]
pub use craft_topdown::{
    auto_decay_time_base_seconds, closest_craft_obj_filtered, craft_obj_passes_scan_filters,
    craft_trans_meta_map_from_content, do_transition_search_skip_reason,
    effective_ai_should_ignore, hardened_row_forces_hoe_soil_ignore,
    search_best_object_for_crafting_topdown, should_skip_craft_edge,
    should_skip_transition_top_down, time_transition_exceeds_ai_ignore, CraftObjectIndex,
    CraftScanFilters, CraftTopDownOpts, CraftTransMeta, TransSkipReason,
    AI_CRAFT_MIN_COUNT_RADIUS_CAP, AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN, HARDENED_ROW, STEEL_HOE,
    STONE_HOE,
};

// Haxe: searchCurrentPosition dual-center + pile*1.5 / r=6 re-anchor (AI-CRAFT-DUAL)
include!("craft_dual_center.inc.rs");

// ── Constants (Haxe ServerSettings / AiBase literals) ───────────────────────

/// Haxe `ServerSettings.AiTimeToWaitIfCraftingFailed` (seconds).
pub const AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC: f64 = 15.0;

/// Haxe `ServerSettings.AiMaxSearchRadius`.
pub const AI_MAX_SEARCH_RADIUS: i32 = 60;

/// Haxe `ServerSettings.AiMaxSearchIncrement`.
pub const AI_MAX_SEARCH_INCREMENT: i32 = 30;

/// Minimum craft scan radius (Haxe `intitObjectsForCraftigHelper` floor 15).
pub const AI_CRAFT_MIN_RADIUS: i32 = 15;

/// Haxe forge family for SMITH profession gate: Firing Forge 304, Forge+Charcoal 305, Forge 303.
pub const FORGE_IDS: [i32; 3] = [304, 305, 303];
/// Haxe `hasOrBecomeProfession('SMITH', 2)` maxPeople on the craftItem forge gate.
// Haxe: AiBase.craftItem L6638
pub const CRAFT_ITEM_SMITH_MAX: i32 = 2;
/// Haxe `craftItemMax` default `max = 1`.
// Haxe: AiBase.craftItemMax L6604
pub const CRAFT_ITEM_MAX_DEFAULT: i32 = 1;
/// Haxe `AiHelper.GetClosestObject` default `searchDistance` (held-transActor retarget).
// Haxe: AiHelper.GetClosestObject L391; AiBase.craftItemHelper L6669
pub const GET_CLOSEST_OBJECT_DEFAULT_R: i32 = 40;
/// Haxe `IsCloseToObject(home, 60)` for `itemToCraft.startLocation`.
// Haxe: AiBase.craftItemHelper L6711
pub const CRAFT_START_HOME_DIST: i32 = 60;

/// Basket of Soil (Haxe shortCraftOnGround special in craftItemHelper).
pub const BASKET_OF_SOIL: i32 = 336;

/// Adobe / Firing Adobe Kiln special (defer residual pottery path).
pub const ADOBE: i32 = 127;
pub const FIRING_ADOBE_KILN: i32 = 282;

/// Clay Bowl (water fill / soil scoop / forge-bias specials).
// Haxe: craftItemHelper Clay Bowl 235
pub const CLAY_BOWL: i32 = 235;
/// Empty Water Pouch.
// Haxe: Empty Water Pouch 209
pub const EMPTY_WATER_POUCH: i32 = 209;
/// Bowl of Gooseberries (fill anti-loop + berry pie crust gate).
// Haxe: Bowl of Gooseberries 253
pub const BOWL_OF_GOOSEBERRIES: i32 = 253;
/// Raw Pie Crust / Raw Berry Pie / Cooked Berry Pie.
// Haxe: 264 / 265 / 272 berry pie crust gate
pub const RAW_PIE_CRUST: i32 = 264;
pub const RAW_BERRY_PIE: i32 = 265;
pub const COOKED_BERRY_PIE: i32 = 272;
/// Bowl of Dry Beans + Dry Bean Plants fill sources.
// Haxe: Bowl of Dry Beans 1176 / Dry Bean Plants 1172
pub const BOWL_OF_DRY_BEANS: i32 = 1176;
pub const DRY_BEAN_PLANTS: i32 = 1172;
/// Fertile Soil Pile / Fertile Soil (Clay Bowl soil retarget).
// Haxe: 1101 / 1138
pub const FERTILE_SOIL_PILE: i32 = 1101;
pub const FERTILE_SOIL: i32 = 1138;
/// Flat Rock (forge-adjacent retarget).
// Haxe: Flat Rock 291
pub const FLAT_ROCK: i32 = 291;
/// Fire Bow Drill + Long Straight Shaft (kindling residual).
// Haxe: 74 + 67 → kindling 72 / tinder 61
pub const FIRE_BOW_DRILL: i32 = 74;
pub const LONG_STRAIGHT_SHAFT: i32 = 67;
pub const KINDLING: i32 = 72;
pub const JUNIPER_TINDER: i32 = 61;
/// Steel Adze / Froe + Butt Log (GetCraftAndDrop residual flag).
// Haxe: 462/463 + 345
pub const STEEL_ADZE: i32 = 462;
pub const STEEL_FROE: i32 = 463;
pub const BUTT_LOG: i32 = 345;

/// Haxe `berryBushesIds` for Bowl of Gooseberries fill check.
// Haxe: AiBase.berryBushesIds = [30, 391]
pub const BERRY_BUSH_IDS: [i32; 2] = [30, 391];

/// Soil targets for Clay Bowl retarget within 30.
// Haxe: soilTargets = [1101, 1138]
pub const SOIL_TARGET_IDS: [i32; 2] = [FERTILE_SOIL_PILE, FERTILE_SOIL];

/// Actors allowed on Flat Rock near a forge (tongs with hot metal).
// Haxe: allowedOnFlatRockIds
pub const ALLOWED_ON_FLAT_ROCK_NEAR_FORGE: [i32; 5] = [308, 2217, 329, 1525, 2293];

/// Default water sources when `ServerSettings.WaterSourceIds` is empty / not loaded.
/// Wells from profession_scan `WELL_IDS` (Deep 663 / Shallow 662) — callers can
/// pass fuller lists via [`CraftLiveExpandOpts::water_source_ids`] from
/// [`init_water_source_ids`] / [`init_water_source_ids_from_content`].
// Haxe: ServerSettings.WaterSourceIds (transition-derived)
pub const DEFAULT_WATER_SOURCE_IDS: [i32; 2] = [663, 662];

/// Haxe `CalculateQuadDistanceToObject(player, forge) < 10`.
// Haxe: AiBase.craftItemHelper L6804 / L6825
pub const FORGE_NEAR_QUAD: i32 = 10;
/// Haxe `GetForge` `GetClosestObjectToPosition(home, 304/305/303, 20)`.
// Haxe: AiBase.GetForge L3644–3654
pub const GET_FORGE_SEARCH_R: i32 = 20;
/// Min distance from forge when retargeting Flat Rock / Clay Bowl (Haxe minDistance 3).
pub const FORGE_BIAS_MIN_DIST: i32 = 3;
/// Search radius for forge-bias retargets (Haxe GetClosestObjectToTarget r=30).
pub const FORGE_BIAS_SEARCH_R: i32 = 30;
/// Soil retarget radius (Haxe GetClosestObjectToPositionByIds r=30).
pub const SOIL_RETARGET_R: i32 = 30;
/// Haxe `GetClosestObjectToPositionByIds(..., waterSourceIds, myPlayer)` default r=40.
// Haxe: AiHelper.GetClosestObjectToPositionByIds L332; AiBase.craftItemHelper L6943
pub const WATER_SOURCE_RETARGET_R: i32 = 40;
/// Haxe `GetClosestObjectToHome(..., 30)` for knife/sword/mango vs sheep/cow.
// Haxe: AiBase.craftItemHelper L6981
pub const DEADLY_SECOND_CLOSE_R: i32 = 30;
/// Haxe TIME-actor wait only if `secondsUntillChange < 10`.
// Haxe: AiBase.craftItemHelper L7024
pub const CRAFT_TIME_WAIT_MAX_SEC: f32 = 10.0;
/// Haxe `ServerSettings.AiIgnoredFloorIds` default (Bear Skin 656 / … 888).
// Haxe: AiHelper.IsIgnoredFloor L46
pub const CRAFT_AI_IGNORED_FLOOR_IDS: [i32; 2] = [656, 888];
/// Haxe `BiomeTag.SNOW` / `BiomeTag.OCEAN` for soil/row skip.
// Haxe: AiBase.addObjectsForCrafting L7287–7289; Biome.SNOW=4 OCEAN=9
pub const CRAFT_BIOME_SNOW: i32 = 4;
pub const CRAFT_BIOME_OCEAN: i32 = 9;
/// Haxe Carrot Row — skip low uses when `!hasCarrotSeeds`.
// Haxe: AiBase.addObjectsForCrafting L7327
pub const CARROT_ROW: i32 = 400;
/// Haxe `objQuadDistance > 4 && IsDangerous` skip for closest/second.
// Haxe: AiBase.addObjectsForCrafting L7341 / L7353
pub const CRAFT_DANGEROUS_QUAD_MIN: i32 = 4;
/// Haxe `IsDangerous` default radius (half-open box).
// Haxe: AiHelper.IsDangerous L1046
pub const CRAFT_IS_DANGEROUS_RADIUS: i32 = 4;

// ── World object (scan snapshot; same shape as GetOrCraftWorldObj) ──────────

/// Ground / held-adjacent object for multi-step craft search.
// Haxe: ObjectHelper parentId / tx / ty / numberOfUses / objectData.numUses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftWorldObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    /// Haxe `objectData.numSlots` (containers).
    pub num_slots: i32,
    /// Haxe `numberOfUses` (pile size / multi-use remaining).
    pub num_uses: i32,
    /// Haxe `objectData.numUses` max (0 = unknown; used by reverseUse / minUseFraction).
    // Haxe: ObjectData.numUses for DoTransitionSearch reverseUseTarget / targetMinUseFraction
    pub max_uses: i32,
    /// Haxe `world.getFloorId(tx, ty)` for IsIgnoredFloor.
    pub floor_id: i32,
    /// Haxe `world.getBiomeId(tx, ty)` (SNOW/OCEAN skip soil/row).
    pub biome_id: i32,
    /// Haxe `container.containedObjects.length`.
    pub contained_count: i32,
    /// Haxe `objData.foodValue > 0`.
    pub is_food: bool,
    /// Haxe `objData.isPermanent()`.
    pub is_permanent: bool,
}

impl CraftWorldObj {
    pub fn simple(parent_id: i32, x: i32, y: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            num_slots: 0,
            num_uses: 1,
            max_uses: 0,
            floor_id: 0,
            biome_id: 0,
            contained_count: 0,
            is_food: false,
            is_permanent: false,
        }
    }

    pub fn with_slots(mut self, num_slots: i32) -> Self {
        self.num_slots = num_slots.max(0);
        self
    }

    pub fn with_uses(mut self, num_uses: i32) -> Self {
        self.num_uses = num_uses.max(0);
        self
    }

    /// Haxe `ObjectHelper.isFull`: `numberOfUses >= objectData.numUses`.
    /// Unknown max (`max_uses == 0`) is treated as not full.
    // Haxe: ObjectHelper.isFull L868–869
    pub fn is_full(self) -> bool {
        self.max_uses > 0 && self.num_uses >= self.max_uses
    }

    /// Set ObjectData-style max uses for multi-use / reverse-use filters.
    // Haxe: ObjectData.numUses
    pub fn with_max_uses(mut self, max_uses: i32) -> Self {
        self.max_uses = max_uses.max(0);
        self
    }

    pub fn with_floor(mut self, floor_id: i32) -> Self {
        self.floor_id = floor_id;
        self
    }

    pub fn with_biome(mut self, biome_id: i32) -> Self {
        self.biome_id = biome_id;
        self
    }

    pub fn with_contained(mut self, n: i32) -> Self {
        self.contained_count = n.max(0);
        self
    }

    pub fn with_food(mut self, is_food: bool) -> Self {
        self.is_food = is_food;
        self
    }

    pub fn with_permanent(mut self, is_permanent: bool) -> Self {
        self.is_permanent = is_permanent;
        self
    }
}

// ── Sticky craft state (Haxe IntemToCraft) ───────────────────────────────────

/// Sticky multi-step craft state across AI ticks.
// Haxe: AiHelper.IntemToCraft
#[derive(Debug, Clone, PartialEq)]
pub struct ItemToCraftState {
    /// Product parent id being crafted (`itemToCraft.itemToCraft.parentId`).
    pub product_id: i32,
    pub max_search_radius: i32,
    /// Haxe `searchCurrentPosition` (home-centric when false).
    pub search_current_position: bool,
    pub count: i32,
    pub count_done: i32,
    pub count_transitions_done: i32,
    pub last_actor_id: i32,
    pub last_target_id: i32,
    pub last_new_actor_id: i32,
    pub last_new_target_id: i32,
    /// Last chosen transition actor/target (sticky between ticks).
    pub trans_actor_id: Option<i32>,
    pub trans_target_id: Option<i32>,
    pub trans_actor_x: Option<i32>,
    pub trans_actor_y: Option<i32>,
    pub trans_target_x: Option<i32>,
    pub trans_target_y: Option<i32>,
    /// Craft drop anchor (home or first target).
    // Haxe: startLocation
    pub start_location: Option<(i32, i32)>,
    pub best_distance: i32,
    /// Haxe `craftingList` (CalculateSteps craftFrom chain).
    pub crafting_list: Vec<i32>,
    /// Haxe `craftingTransitions` (craftTransFrom, may be shorter than list).
    pub crafting_transitions: Vec<CraftTransMeta>,
}

impl Default for ItemToCraftState {
    fn default() -> Self {
        Self {
            product_id: 0,
            max_search_radius: AI_MAX_SEARCH_RADIUS,
            search_current_position: true, // Haxe IntemToCraft default true (AI-CRAFT-DUAL)
            count: 0,
            count_done: 0,
            count_transitions_done: 0,
            last_actor_id: -1,
            last_target_id: -1,
            last_new_actor_id: -1,
            last_new_target_id: -1,
            trans_actor_id: None,
            trans_target_id: None,
            trans_actor_x: None,
            trans_actor_y: None,
            trans_target_x: None,
            trans_target_y: None,
            start_location: None,
            best_distance: i32::MAX / 4,
            crafting_list: Vec::new(),
            crafting_transitions: Vec::new(),
        }
    }
}

impl ItemToCraftState {
    pub fn new(product_id: i32) -> Self {
        Self {
            product_id,
            count: 1,
            ..Self::default()
        }
    }

    pub fn with_max_search(mut self, r: i32) -> Self {
        self.max_search_radius = r.max(0);
        self
    }

    /// Reset when product id changes (Haxe `itemToCraft.itemToCraft.parentId != objId`).
    pub fn reset_for_product(&mut self, product_id: i32) {
        self.product_id = product_id;
        self.count = 1;
        self.count_done = 0;
        self.count_transitions_done = 0;
        self.last_actor_id = -1;
        self.last_target_id = -1;
        self.last_new_actor_id = -1;
        self.last_new_target_id = -1;
        self.trans_actor_id = None;
        self.trans_target_id = None;
        self.trans_actor_x = None;
        self.trans_actor_y = None;
        self.trans_target_x = None;
        self.trans_target_y = None;
        self.start_location = None;
        self.best_distance = i32::MAX / 4;
        self.crafting_list.clear();
        self.crafting_transitions.clear();
    }

    pub fn clear_trans(&mut self) {
        self.trans_actor_id = None;
        self.trans_target_id = None;
        self.trans_actor_x = None;
        self.trans_actor_y = None;
        self.trans_target_x = None;
        self.trans_target_y = None;
    }

    pub fn set_trans_pair(
        &mut self,
        actor_id: i32,
        ax: i32,
        ay: i32,
        target_id: i32,
        tx: i32,
        ty: i32,
        dist: i32,
    ) {
        self.trans_actor_id = Some(actor_id);
        self.trans_actor_x = Some(ax);
        self.trans_actor_y = Some(ay);
        self.trans_target_id = Some(target_id);
        self.trans_target_x = Some(tx);
        self.trans_target_y = Some(ty);
        self.best_distance = dist;
    }
}

// ── Failed craft cooldown (Haxe failedCraftings) ─────────────────────────────

/// Map product_id → sim time (seconds or tick-as-sec) when craft last failed.
// Haxe: failedCraftings Map<Int, Float> + AiTimeToWaitIfCraftingFailed
#[derive(Debug, Clone, Default)]
pub struct FailedCraftings {
    pub last_fail_sec: HashMap<i32, f64>,
}

impl FailedCraftings {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when still within cooldown for `product_id`.
    // Haxe: waitTime = AiTimeToWaitIfCraftingFailed - passedTimeSinceFailed; if (waitTime > 0) return false
    pub fn is_cooling_down(&self, product_id: i32, now_sec: f64) -> bool {
        self.remaining_wait_sec(product_id, now_sec) > 0.0
    }

    pub fn remaining_wait_sec(&self, product_id: i32, now_sec: f64) -> f64 {
        self.remaining_wait_sec_ex(product_id, now_sec, AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC)
    }

    /// Live wait override (Haxe `AiTimeToWaitIfCraftingFailed`).
    // SETTINGS-LONG-TAIL
    pub fn remaining_wait_sec_ex(&self, product_id: i32, now_sec: f64, wait_sec: f64) -> f64 {
        let wait = if wait_sec.is_finite() && wait_sec >= 0.0 {
            wait_sec
        } else {
            AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC
        };
        match self.last_fail_sec.get(&product_id) {
            Some(&t) => {
                let passed = now_sec - t;
                (wait - passed).max(0.0)
            }
            None => 0.0,
        }
    }

    pub fn is_cooling_down_ex(&self, product_id: i32, now_sec: f64, wait_sec: f64) -> bool {
        self.remaining_wait_sec_ex(product_id, now_sec, wait_sec) > 0.0
    }

    pub fn record_fail(&mut self, product_id: i32, now_sec: f64) {
        self.last_fail_sec.insert(product_id, now_sec);
    }

    pub fn clear(&mut self) {
        self.last_fail_sec.clear();
    }
}

// ── Sticky multi-tick craft runtime (Haxe Player.itemToCraft + failedCraftings) ─

/// Persistent craft state for an AI/NPC across ticks.
// Haxe: AiBase.itemToCraft + failedCraftings + lastActorId + calledCraftItem + craftingTasks
#[derive(Debug, Clone, Default)]
pub struct CraftAiRuntime {
    pub item: ItemToCraftState,
    pub failed: FailedCraftings,
    /// Haxe `lastActorId` — bowl fill anti-loop.
    pub last_actor_id: i32,
    /// Haxe `calledCraftItem` recursion guard for GetCraftAndDrop specials.
    pub called_craft_item: bool,
    /// Haxe `craftingTasks` — interrupted / queued product ids (NPC sticky shell).
    // Haxe: AiBase.craftingTasks
    pub crafting_tasks: Vec<i32>,
}

impl CraftAiRuntime {
    pub fn new() -> Self {
        Self {
            last_actor_id: -1,
            ..Self::default()
        }
    }

    /// Haxe `craftItem` post-success: `lastActorId = -1` then set from transActor.
    // Haxe: if (done) lastActorId = -1; if (done && itemToCraft.transActor != null) lastActorId = …
    pub fn note_craft_done(&mut self, decision: CraftItemDecision) {
        if !decision.is_action() {
            return;
        }
        self.last_actor_id = -1;
        if let Some(aid) = self.item.trans_actor_id {
            if aid > 0 {
                self.last_actor_id = aid;
            }
        } else if let CraftItemDecision::UseOnTarget { actor_id, .. } = decision {
            if actor_id > 0 {
                self.last_actor_id = actor_id;
            }
        } else if let CraftItemDecision::PickupActor { object_id, .. } = decision {
            if object_id > 0 {
                self.last_actor_id = object_id;
            }
        }
        self.called_craft_item = false;
    }

    /// Clear per-tick recursion guard (Haxe `calledCraftItem = false` each doTime).
    pub fn clear_tick_guard(&mut self) {
        self.called_craft_item = false;
    }

    /// Mark recursion guard after GetCraftAndDrop specials (Haxe `calledCraftItem = true`).
    // Haxe: craftItemHelper calledCraftItem = true before GetCraftAndDrop
    pub fn note_called_craft_item_from_decision(&mut self, decision: CraftItemDecision) {
        if matches!(
            decision,
            CraftItemDecision::GotoDropAnchor { .. }
                | CraftItemDecision::DropNearAnchor { .. }
                | CraftItemDecision::SeekIngredient { .. }
                | CraftItemDecision::PickupActor { .. }
        ) {
            self.called_craft_item = true;
        }
    }

    /// Haxe `addTask(taskId, atEnd)` — skip `<1` and duplicates.
    // Haxe: AiBase.addTask
    pub fn add_task(&mut self, task_id: i32, at_end: bool) {
        if task_id < 1 {
            return;
        }
        if self.crafting_tasks.contains(&task_id) {
            return;
        }
        if at_end {
            self.crafting_tasks.push(task_id);
        } else {
            self.crafting_tasks.insert(0, task_id);
        }
    }

    /// Prepare sticky for a new `product_id` (re-queue interrupted prior product).
    ///
    /// No-ops when `item.product_id` already matches (PlayerCraftAi prepare already
    /// reset) so player + runtime queues are not double-filled.
    // Haxe: craftItemHelper when itemToCraft.itemToCraft.parentId != objId ~6677–6690
    pub fn prepare_for_product(&mut self, product_id: i32) {
        let prev = self.item.product_id;
        if prev > 0 && prev != product_id {
            // Interrupted unfinished craft → re-queue prior product.
            if self.item.count_done < self.item.count {
                self.add_task(prev, true);
            }
        }
        if product_id > 0 && self.item.product_id != product_id {
            self.item.reset_for_product(product_id);
        }
    }

    /// Pop next queued craft task and bind [`ItemToCraftState`] via reset.
    // Haxe: craftingTasks.shift + craftItem
    pub fn take_next_crafting_task(&mut self) -> Option<i32> {
        if self.crafting_tasks.is_empty() {
            return None;
        }
        let id = self.crafting_tasks.remove(0);
        self.item.reset_for_product(id);
        Some(id)
    }

    /// Unfinished sticky product (`product_id > 0 && (count <= 0 || count_done < count)`).
    // Haxe: itemToCraftId > 0 && itemToCraft.countDone < itemToCraft.count
    pub fn should_continue_unfinished(&self) -> bool {
        if self.item.product_id <= 0 {
            return false;
        }
        if self.item.count <= 0 {
            return true;
        }
        self.item.count_done < self.item.count
    }
}

/// Live expand options for multi-step craft on the tick path.
// Haxe: home / hasOrBecomeProfession('SMITH') / TimeHelper ticks / WaterSourceIds
#[derive(Debug, Clone, PartialEq)]
pub struct CraftLiveExpandOpts {
    pub home: Option<(i32, i32)>,
    pub is_or_can_smith: bool,
    pub now_sec: f64,
    /// Haxe `ServerSettings.WaterSourceIds` from [`init_water_source_ids`].
    /// Empty → [`DEFAULT_WATER_SOURCE_IDS`] via [`Self::effective_water_source_ids`].
    pub water_source_ids: Vec<i32>,
    /// Haxe `ServerSettings.BucketWaterSourceIds` from [`init_water_source_ids`].
    /// Empty → [`DEFAULT_BUCKET_WATER_SOURCE_IDS`] via [`Self::effective_bucket_water_source_ids`].
    pub bucket_water_source_ids: Vec<i32>,
    /// Haxe `AiTimeToWaitIfCraftingFailed`.
    // SETTINGS-LONG-TAIL
    pub ai_time_to_wait_if_crafting_failed_sec: f64,
    /// Haxe `AiMaxSearchRadius`.
    // SETTINGS-LONG-TAIL
    pub ai_max_search_radius: i32,
    /// Haxe `AiMaxSearchIncrement`.
    // SETTINGS-LONG-TAIL
    pub ai_max_search_increment: i32,
    /// Haxe `AiIgnoreTimeTransitionsLongerThen`.
    // SETTINGS-LONG-TAIL
    pub ai_ignore_time_transitions_longer_then: f32,
    /// From `ContentDb` (`craft_trans_meta_map_from_content`).
    pub trans_meta: Option<HashMap<(i32, i32), CraftTransMeta>>,
    /// Haxe `ObjectData.aiCraftMax`.
    pub ai_craft_max: HashMap<i32, i32>,
    /// Haxe `ObjectData.aiCraftMin`.
    pub ai_craft_min: HashMap<i32, i32>,
    /// Haxe `heldObject != hiddenWound`.
    // Haxe: AiBase.craftItemHelper L7003
    pub is_hidden_wound: bool,
}

impl Default for CraftLiveExpandOpts {
    fn default() -> Self {
        Self {
            home: None,
            is_or_can_smith: true,
            now_sec: 0.0,
            water_source_ids: Vec::new(),
            bucket_water_source_ids: Vec::new(),
            ai_time_to_wait_if_crafting_failed_sec: AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC,
            ai_max_search_radius: AI_MAX_SEARCH_RADIUS,
            ai_max_search_increment: AI_MAX_SEARCH_INCREMENT,
            ai_ignore_time_transitions_longer_then: AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN,
            trans_meta: None,
            ai_craft_max: HashMap::new(),
            ai_craft_min: HashMap::new(),
            is_hidden_wound: false,
        }
    }
}

impl CraftLiveExpandOpts {
    pub fn with_home(mut self, x: i32, y: i32) -> Self {
        self.home = Some((x, y));
        self
    }

    pub fn with_smith(mut self, is_or_can_smith: bool) -> Self {
        self.is_or_can_smith = is_or_can_smith;
        self
    }

    pub fn with_now(mut self, now_sec: f64) -> Self {
        self.now_sec = now_sec;
        self
    }

    /// Set transition-derived water + bucket source ids (Haxe InitWaterSourceIds result).
    pub fn with_water_source_ids(mut self, water: Vec<i32>, bucket: Vec<i32>) -> Self {
        self.water_source_ids = water;
        self.bucket_water_source_ids = bucket;
        self
    }

    /// Copy LimitTransitions / PatchTransitions AI gates from content.
    pub fn with_content_craft_gates(mut self, content: &ol_content::ContentDb) -> Self {
        self.trans_meta = Some(craft_trans_meta_map_from_content(content));
        self.ai_craft_max = content.ai_craft_max.clone();
        self.ai_craft_min = content.ai_craft_min.clone();
        self
    }

    /// Effective bowl/pouch water sources for [`retarget_water_source`].
    #[inline]
    pub fn effective_water_source_ids(&self) -> &[i32] {
        effective_water_source_ids(&self.water_source_ids)
    }

    /// Effective bucket water sources for [`fill_bucket_if_needed_apply`].
    #[inline]
    pub fn effective_bucket_water_source_ids(&self) -> &[i32] {
        effective_bucket_water_source_ids(&self.bucket_water_source_ids)
    }
}

// ── Inputs ──────────────────────────────────────────────────────────────────

/// Live context for one craftItem tick.
// Haxe: craftItemHelper(objId, maxDistance, onlyHome) + player/home
#[derive(Debug, Clone, PartialEq)]
pub struct CraftItemInput {
    pub product_id: i32,
    /// Haxe `maxDistance` override for `maxSearchRadius` when > 0.
    pub max_distance: i32,
    /// Haxe `onlyHome` → forces searchCurrentPosition=false.
    pub only_home: bool,
    pub player_x: i32,
    pub player_y: i32,
    pub held_id: i32,
    /// Home tile if known (search dual-center always; startLocation uses quad ≤ 60²).
    pub home_x: Option<i32>,
    pub home_y: Option<i32>,
    /// Sim time in seconds (for failedCraftings cooldown).
    pub now_sec: f64,
    /// Haxe `myPlayer.isMoving()` early path uses sticky held actor.
    pub is_moving: bool,
    /// True when AI already has SMITH profession (or can become).
    // Haxe: hasOrBecomeProfession('SMITH', 2)
    pub is_or_can_smith: bool,
    /// Last actor id used (Haxe `lastActorId` anti-loop for berry bowl etc.).
    pub last_actor_id: i32,
    /// Haxe `calledCraftItem` — skip recursive GetCraftAndDrop specials when true.
    pub called_craft_item: bool,
    /// Haxe `AiTimeToWaitIfCraftingFailed`.
    // SETTINGS-LONG-TAIL
    pub ai_wait_failed_sec: f64,
    /// Haxe `AiMaxSearchRadius` when `max_distance` is unset.
    // SETTINGS-LONG-TAIL
    pub ai_max_search_radius: i32,
    /// Haxe `AiMaxSearchIncrement`.
    // SETTINGS-LONG-TAIL
    pub ai_search_increment: i32,
    /// Haxe `AiIgnoreTimeTransitionsLongerThen`.
    // SETTINGS-LONG-TAIL
    pub ai_ignore_time_transitions_longer_then: f32,
    /// Haxe `ObjectData.aiCraftMax`.
    pub ai_craft_max: HashMap<i32, i32>,
    /// Haxe `ObjectData.aiCraftMin`.
    pub ai_craft_min: HashMap<i32, i32>,
    /// Haxe `doPotteryOnFire()` would consume the adobe+kiln tick.
    // Haxe: AiBase.craftItemHelper L6746
    pub pottery_on_fire: bool,
    /// Haxe `hasCarrotSeeds` — Carrot Row 400 uses < 4 skipped when false.
    // Haxe: AiBase.addObjectsForCrafting L7327
    pub has_carrot_seeds: bool,
    /// Haxe `heldObject != hiddenWound` when dropping for empty actor.
    // Haxe: AiBase.craftItemHelper L7003
    pub is_hidden_wound: bool,
    /// Haxe `transTarget.isAnimal()` on TIME-actor wait.
    // Haxe: AiBase.craftItemHelper L7024
    pub target_is_animal: bool,
    /// Haxe `transTarget.timeUntillChange()` seconds.
    // Haxe: AiBase.craftItemHelper L7021
    pub target_seconds_until_change: f32,
}

impl CraftItemInput {
    pub fn basic(product_id: i32, player_x: i32, player_y: i32) -> Self {
        Self {
            product_id,
            max_distance: -1,
            only_home: false,
            player_x,
            player_y,
            held_id: 0,
            home_x: None,
            home_y: None,
            now_sec: 0.0,
            is_moving: false,
            is_or_can_smith: true,
            last_actor_id: -1,
            called_craft_item: false,
            ai_wait_failed_sec: AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC,
            ai_max_search_radius: AI_MAX_SEARCH_RADIUS,
            ai_search_increment: AI_MAX_SEARCH_INCREMENT,
            ai_ignore_time_transitions_longer_then: AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN,
            ai_craft_max: HashMap::new(),
            ai_craft_min: HashMap::new(),
            pottery_on_fire: false,
            has_carrot_seeds: true,
            is_hidden_wound: false,
            target_is_animal: false,
            target_seconds_until_change: 0.0,
        }
    }

    pub fn with_held(mut self, held_id: i32) -> Self {
        self.held_id = held_id;
        self
    }

    pub fn with_home(mut self, x: i32, y: i32) -> Self {
        self.home_x = Some(x);
        self.home_y = Some(y);
        self
    }

    pub fn with_now(mut self, now_sec: f64) -> Self {
        self.now_sec = now_sec;
        self
    }

    pub fn with_max_distance(mut self, max_distance: i32) -> Self {
        self.max_distance = max_distance;
        self
    }

    pub fn with_last_actor(mut self, last_actor_id: i32) -> Self {
        self.last_actor_id = last_actor_id;
        self
    }

    pub fn from_runtime(
        product_id: i32,
        player_x: i32,
        player_y: i32,
        held_id: i32,
        opts: &CraftLiveExpandOpts,
        runtime: &CraftAiRuntime,
    ) -> Self {
        let mut inp = Self::basic(product_id, player_x, player_y)
            .with_held(held_id)
            .with_now(opts.now_sec)
            .with_last_actor(runtime.last_actor_id);
        inp.is_or_can_smith = opts.is_or_can_smith;
        inp.called_craft_item = runtime.called_craft_item;
        inp.ai_wait_failed_sec = opts.ai_time_to_wait_if_crafting_failed_sec;
        inp.ai_max_search_radius = opts.ai_max_search_radius;
        inp.ai_search_increment = opts.ai_max_search_increment;
        inp.ai_ignore_time_transitions_longer_then = opts.ai_ignore_time_transitions_longer_then;
        inp.ai_craft_max = opts.ai_craft_max.clone();
        inp.ai_craft_min = opts.ai_craft_min.clone();
        inp.is_hidden_wound = opts.is_hidden_wound;
        if let Some((hx, hy)) = opts.home {
            inp = inp.with_home(hx, hy);
        }
        inp
    }
}

// ── Search result pair ──────────────────────────────────────────────────────

/// Best (actor, target) pair found for one craft step.
// Haxe: itemToCraft.transActor / transTarget
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftTransPair {
    pub actor_id: i32,
    pub actor_x: i32,
    pub actor_y: i32,
    /// Actor is currently held (distance 0).
    pub actor_held: bool,
    /// Actor comes from a pile (empty-hand USE on pile).
    pub actor_from_pile: bool,
    pub pile_id: i32,
    pub target_id: i32,
    pub target_x: i32,
    pub target_y: i32,
    /// Chebyshev player→actor + actor→target (Haxe bestDistance approx).
    pub distance: i32,
    pub search_radius: i32,
}

// ── Decision enum ───────────────────────────────────────────────────────────

/// Pure craftItemHelper outcome for one tick.
// Haxe: craftItemHelper return + useTarget / dropTarget / useActor staging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftItemDecision {
    /// Still in failedCraftings cooldown.
    Cooldown,
    /// No actor/target found → fail recorded by caller.
    Failed,
    /// Product already held or present on ground (no craft step needed).
    AlreadyHave {
        object_id: i32,
        x: i32,
        y: i32,
        held: bool,
    },
    /// Held is actor (or empty actor) → USE on target.
    // Haxe: useTarget = transTarget; useActor = transActor; return true
    UseOnTarget {
        actor_id: i32,
        target_id: i32,
        target_x: i32,
        target_y: i32,
    },
    /// Need empty hands (actor id 0) while holding something.
    // Haxe: dropHeldObject when Empty is needed
    DropHeldForEmpty,
    /// Pickup loose actor (Haxe dropTarget = transActor).
    PickupActor { object_id: i32, x: i32, y: i32 },
    /// Empty-hand USE on pile to get actor.
    UsePileForActor { pile_id: i32, x: i32, y: i32 },
    /// Holding something; must drop before pickup (Haxe considerDropHeldObject).
    DropHeldThenPickup {
        actor_id: i32,
        actor_x: i32,
        actor_y: i32,
    },
    /// Missing leaf ingredient → seek/craft that first.
    SeekIngredient {
        ingredient_id: i32,
        for_product: i32,
    },
    /// Forge in use and AI cannot become smith.
    // Haxe: hasOrBecomeProfession('SMITH', 2) == false → return false
    NeedSmithProfession,
    /// Special: Basket of Soil shortCraftOnGround residual staging.
    ShortCraftOnGround { object_id: i32 },
    /// Special: adobe on firing kiln → pottery residual.
    DeferPottery,
    /// Wait for time-transition target (actor id -1).
    WaitTime,
    /// GetCraftAndDrop: walk toward drop anchor while holding whichObj (quadDist > 5).
    // Haxe: GetCraftAndDropItemsCloseToObj gotoObj(target)
    GotoDropAnchor { target_x: i32, target_y: i32 },
    /// GetCraftAndDrop: drop held whichObj near anchor.
    // Haxe: dropHeldObject(5, target)
    DropNearAnchor { target_x: i32, target_y: i32 },
}

impl CraftItemDecision {
    pub fn is_action(self) -> bool {
        matches!(
            self,
            Self::UseOnTarget { .. }
                | Self::PickupActor { .. }
                | Self::UsePileForActor { .. }
                | Self::DropHeldForEmpty
                | Self::DropHeldThenPickup { .. }
                | Self::AlreadyHave { .. }
                | Self::SeekIngredient { .. }
                | Self::ShortCraftOnGround { .. }
                | Self::GotoDropAnchor { .. }
                | Self::DropNearAnchor { .. }
        )
    }
}

// ── Spatial helpers ─────────────────────────────────────────────────────────

#[inline]
pub fn craft_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Closest matching parent_id within `max_r` of `(from_x, from_y)`.
///
/// When `exclude` is set, skip that tile (for second-closest sheep/cow).
/// Unfiltered wrapper — prefer [`closest_craft_obj_filtered`] on live AI paths.
// Haxe: GetClosestObject* / secondObject
pub fn closest_craft_obj(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    exclude: Option<(i32, i32)>,
) -> Option<CraftWorldObj> {
    closest_craft_obj_filtered(
        objs,
        parent_id,
        from_x,
        from_y,
        max_r,
        exclude,
        &CraftScanFilters::default(),
    )
}

/// Second-closest of `parent_id` (Haxe sheep/cow deadly-actor special).
pub fn second_closest_craft_obj(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<CraftWorldObj> {
    second_closest_craft_obj_filtered(
        objs,
        parent_id,
        from_x,
        from_y,
        max_r,
        &CraftScanFilters::default(),
    )
}

/// Second-closest of `parent_id` skipping scan-blocked tiles.
// Haxe: GetClosestObject* secondObject + isObjectNotReachable
pub fn second_closest_craft_obj_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    let first = closest_craft_obj_filtered(objs, parent_id, from_x, from_y, max_r, None, filters)?;
    closest_craft_obj_filtered(
        objs,
        parent_id,
        from_x,
        from_y,
        max_r,
        Some((first.x, first.y)),
        filters,
    )
}

/// Closest object whose parent_id is in `ids` within `max_r` of `(from_x, from_y)`.
// Haxe: AiHelper.GetClosestObjectToPositionByIds
pub fn closest_craft_obj_by_ids(
    objs: &[CraftWorldObj],
    ids: &[i32],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<CraftWorldObj> {
    closest_craft_obj_by_ids_filtered(
        objs,
        ids,
        from_x,
        from_y,
        max_r,
        &CraftScanFilters::default(),
    )
}

/// Closest of `ids` with hostile / notReachable / full-pile scan filters.
// Haxe: GetClosestObjectToPositionByIdsHelper isObjectNotReachable / hostile
pub fn closest_craft_obj_by_ids_filtered(
    objs: &[CraftWorldObj],
    ids: &[i32],
    from_x: i32,
    from_y: i32,
    max_r: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if ids.is_empty() {
        return None;
    }
    let max_r = max_r.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if !ids.contains(&o.parent_id) {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        let d = craft_chebyshev(from_x, from_y, o.x, o.y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, *o)),
            Some((bd, bo)) => {
                if d < bd || (d == bd && (o.y < bo.y || (o.y == bo.y && o.x < bo.x))) {
                    best = Some((d, *o));
                }
            }
        }
    }
    best.map(|(_, o)| o)
}

/// Closest of `parent_id` with Chebyshev distance to `anchor` **≥ min_dist**.
// Haxe: GetClosestObjectToTarget(player, forge, id, 30, minDistance=3)
pub fn closest_craft_obj_min_anchor_dist(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    anchor_x: i32,
    anchor_y: i32,
    min_anchor_dist: i32,
) -> Option<CraftWorldObj> {
    closest_craft_obj_min_anchor_dist_filtered(
        objs,
        parent_id,
        from_x,
        from_y,
        max_r,
        anchor_x,
        anchor_y,
        min_anchor_dist,
        &CraftScanFilters::default(),
    )
}

/// [`closest_craft_obj_min_anchor_dist`] skipping scan-blocked tiles.
// Haxe: GetClosestObjectToTarget + isObjectNotReachable
pub fn closest_craft_obj_min_anchor_dist_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    anchor_x: i32,
    anchor_y: i32,
    min_anchor_dist: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let max_r = max_r.max(0);
    let min_anchor_dist = min_anchor_dist.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        let d_from = craft_chebyshev(from_x, from_y, o.x, o.y);
        if d_from > max_r {
            continue;
        }
        let d_anchor = craft_chebyshev(anchor_x, anchor_y, o.x, o.y);
        if d_anchor < min_anchor_dist {
            continue;
        }
        match best {
            None => best = Some((d_from, *o)),
            Some((bd, bo)) => {
                if d_from < bd || (d_from == bd && (o.y < bo.y || (o.y == bo.y && o.x < bo.x))) {
                    best = Some((d_from, *o));
                }
            }
        }
    }
    best.map(|(_, o)| o)
}

/// Haxe `GetClosestObjectToTarget(player, target, id, searchDistance, minDistance)`.
///
/// Search is **centered on the target** (Chebyshev box `search_distance`), skip
/// `quad < minDistance²`, pick min quad to the target.
// Haxe: AiHelper.GetClosestObjectToTarget L116–119; GetClosestObjectToPositionHelper L182
pub fn closest_craft_obj_to_target_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    target_x: i32,
    target_y: i32,
    search_distance: i32,
    min_distance: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let max_r = search_distance.max(0);
    let min_d = min_distance.max(0);
    let min_quad = min_d * min_d;
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        if craft_chebyshev(target_x, target_y, o.x, o.y) > max_r {
            continue;
        }
        let q = craft_quad_distance(target_x, target_y, o.x, o.y);
        if q < min_quad {
            continue;
        }
        match best {
            None => best = Some((q, *o)),
            Some((bq, bo)) => {
                if q < bq || (q == bq && (o.y < bo.y || (o.y == bo.y && o.x < bo.x))) {
                    best = Some((q, *o));
                }
            }
        }
    }
    best.map(|(_, o)| o)
}

/// Haxe `GetClosestObjectToPositionByIds`: Chebyshev box, min **quad** to `(from_x,from_y)`.
// Haxe: AiHelper.GetClosestObjectToPositionByIdsHelper L346; GetClosestObjectToPositionHelper L182/290
pub fn closest_craft_obj_by_ids_quad_filtered(
    objs: &[CraftWorldObj],
    ids: &[i32],
    from_x: i32,
    from_y: i32,
    search_distance: i32,
    exclude: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if ids.is_empty() {
        return None;
    }
    let max_r = search_distance.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if !ids.contains(&o.parent_id) {
            continue;
        }
        if exclude == Some((o.x, o.y)) {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        if craft_chebyshev(from_x, from_y, o.x, o.y) > max_r {
            continue;
        }
        let q = craft_quad_distance(from_x, from_y, o.x, o.y);
        match best {
            None => best = Some((q, *o)),
            Some((bq, bo)) => {
                if q < bq || (q == bq && (o.y < bo.y || (o.y == bo.y && o.x < bo.x))) {
                    best = Some((q, *o));
                }
            }
        }
    }
    best.map(|(_, o)| o)
}

/// Second-closest by quad (Haxe GetClosestObjectToHome then ignore first).
// Haxe: AiBase.craftItemHelper L6981–6987
pub fn second_closest_craft_obj_quad_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    search_distance: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    let first = closest_craft_obj_by_ids_quad_filtered(
        objs,
        &[parent_id],
        from_x,
        from_y,
        search_distance,
        None,
        filters,
    )?;
    closest_craft_obj_by_ids_quad_filtered(
        objs,
        &[parent_id],
        from_x,
        from_y,
        search_distance,
        Some((first.x, first.y)),
        filters,
    )
}

/// Count objects with parent_id in `ids` within `max_r` of player.
// Haxe: countCurrentObjects / CountCloseObjects
pub fn count_craft_objs_near(
    objs: &[CraftWorldObj],
    ids: &[i32],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> i32 {
    let max_r = max_r.max(0);
    let mut n = 0;
    for o in objs {
        if ids.contains(&o.parent_id) && craft_chebyshev(from_x, from_y, o.x, o.y) <= max_r {
            n += 1;
        }
    }
    n
}

/// Closest forge in [`FORGE_IDS`] (priority 304 → 305 → 303 like GetForge).
// Haxe: GetForge
pub fn closest_forge_craft(
    objs: &[CraftWorldObj],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<CraftWorldObj> {
    closest_forge_craft_filtered(objs, from_x, from_y, max_r, &CraftScanFilters::default())
}

/// [`closest_forge_craft`] skipping scan-blocked tiles.
// Haxe: GetForge + isObjectNotReachable
pub fn closest_forge_craft_filtered(
    objs: &[CraftWorldObj],
    from_x: i32,
    from_y: i32,
    max_r: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    // Prefer firing → charcoal → cold (lower index in FORGE_IDS is higher priority).
    let mut best: Option<(usize, i32, CraftWorldObj)> = None;
    for o in objs {
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        let Some(prio) = FORGE_IDS.iter().position(|&id| id == o.parent_id) else {
            continue;
        };
        let d = craft_chebyshev(from_x, from_y, o.x, o.y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((prio, d, *o)),
            Some((bp, bd, _)) => {
                if prio < bp || (prio == bp && d < bd) {
                    best = Some((prio, d, *o));
                }
            }
        }
    }
    best.map(|(_, _, o)| o)
}

/// Haxe `GetForge`: 304 then 305 then 303 from **home** (else player), r=20.
// Haxe: AiBase.GetForge L3644–3654
pub fn get_forge(
    objs: &[CraftWorldObj],
    home: Option<(i32, i32)>,
    player_x: i32,
    player_y: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    let (bx, by) = home.unwrap_or((player_x, player_y));
    closest_forge_craft_filtered(objs, bx, by, GET_FORGE_SEARCH_R, filters)
}

// ── craftItemHelper specials (pure retarget / gates) ─────────────────────────

/// Water-source retarget for Clay Bowl / Empty Water Pouch onto closest water.
// Haxe: craftItemHelper ~6905–6962 WaterSourceIds retarget
pub fn retarget_water_source(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    from_x: i32,
    from_y: i32,
    _max_r: i32,
    water_source_ids: &[i32],
) -> Option<CraftWorldObj> {
    retarget_water_source_ex(
        objs,
        actor_id,
        target_id,
        from_x,
        from_y,
        _max_r,
        water_source_ids,
        &CraftScanFilters::default(),
    )
}

/// [`retarget_water_source`] skipping notReachable / hostile / full-pile tiles.
// Haxe: GetClosestObjectToPositionByIds(myPlayer, waterSourceIds)
pub fn retarget_water_source_ex(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    from_x: i32,
    from_y: i32,
    _max_r: i32,
    water_source_ids: &[i32],
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if actor_id != CLAY_BOWL && actor_id != EMPTY_WATER_POUCH {
        return None;
    }
    if water_source_ids.is_empty() || !water_source_ids.contains(&target_id) {
        return None;
    }
    closest_craft_obj_by_ids_quad_filtered(
        objs,
        water_source_ids,
        from_x,
        from_y,
        WATER_SOURCE_RETARGET_R,
        None,
        filters,
    )
}

/// Soil retarget: Clay Bowl prefers closest Fertile Soil Pile / Fertile Soil within 30.
// Haxe: craftItemHelper ~6784–6793
pub fn retarget_soil_for_clay_bowl(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    from_x: i32,
    from_y: i32,
) -> Option<CraftWorldObj> {
    retarget_soil_for_clay_bowl_ex(
        objs,
        actor_id,
        target_id,
        from_x,
        from_y,
        &CraftScanFilters::default(),
    )
}

/// [`retarget_soil_for_clay_bowl`] skipping scan-blocked tiles.
// Haxe: GetClosestObjectToPositionByIds(myPlayer, soilTargets, 30)
pub fn retarget_soil_for_clay_bowl_ex(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    from_x: i32,
    from_y: i32,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if actor_id != CLAY_BOWL || !SOIL_TARGET_IDS.contains(&target_id) {
        return None;
    }
    closest_craft_obj_by_ids_filtered(
        objs,
        &SOIL_TARGET_IDS,
        from_x,
        from_y,
        SOIL_RETARGET_R,
        filters,
    )
}

/// Berry pie crust gate: block 253+264 when raw/cooked berry pie count > 1.
// Haxe: craftItemHelper ~6751–6756
pub fn berry_pie_crust_blocked(
    actor_id: i32,
    target_id: i32,
    objs: &[CraftWorldObj],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> bool {
    if actor_id != BOWL_OF_GOOSEBERRIES || target_id != RAW_PIE_CRUST {
        return false;
    }
    count_craft_objs_near(
        objs,
        &[RAW_BERRY_PIE, COOKED_BERRY_PIE],
        from_x,
        from_y,
        max_r,
    ) > 1
}

/// Bowl-fill anti-loop: incomplete berry/bean bowl without fill sources / lastActor match.
// Haxe: craftItemHelper L6837–6881
pub fn bowl_fill_pickup_blocked(
    held_id: i32,
    actor_id: i32,
    target_id: i32,
    last_actor_id: i32,
    objs: &[CraftWorldObj],
    from_x: i32,
    from_y: i32,
    max_r: i32,
    actor_full: bool,
) -> bool {
    if actor_full {
        return false;
    }
    // Bowl of Gooseberries 253
    if held_id != BOWL_OF_GOOSEBERRIES
        && actor_id == BOWL_OF_GOOSEBERRIES
        && !BERRY_BUSH_IDS.contains(&target_id)
    {
        if actor_id == last_actor_id {
            return true;
        }
        let bushes = count_craft_objs_near(objs, &BERRY_BUSH_IDS, from_x, from_y, max_r);
        if bushes < 1 {
            return true;
        }
    }
    // Bowl of Dry Beans 1176
    if held_id != BOWL_OF_DRY_BEANS && actor_id == BOWL_OF_DRY_BEANS && target_id != DRY_BEAN_PLANTS
    {
        if actor_id == last_actor_id {
            return true;
        }
        let plants = count_craft_objs_near(objs, &[DRY_BEAN_PLANTS], from_x, from_y, max_r);
        if plants < 1 {
            return true;
        }
    }
    false
}

/// Haxe `shouldDebugSay` while counting bushes to fill a gooseberry bowl.
// Haxe: AiBase.craftItemHelper L6855
pub fn bowl_gooseberry_fill_debug_say(debug_say: bool, bush_count: i32) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!(
        "Bushed to fill Bowl of Gooseberries: {bush_count}"
    ))
}

/// Haxe `shouldDebugSay` while counting dry-bean fill targets (`Yargets` typo kept).
// Haxe: AiBase.craftItemHelper L6878
pub fn bowl_beans_fill_debug_say(
    debug_say: bool,
    actor_name: &str,
    count: i32,
) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!("Yargets to fill {actor_name} {count}"))
}

/// Flat Rock near forge: forbidden actor → retarget rock ≥ min dist from forge, or fail.
// Haxe: craftItemHelper ~6795–6816
/// Returns `Some(Ok(new_target))` retarget, `Some(Err(()))` fail, `None` no change.
pub fn retarget_flat_rock_near_forge(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    player_x: i32,
    player_y: i32,
) -> Option<Result<CraftWorldObj, ()>> {
    retarget_flat_rock_near_forge_ex(
        objs,
        actor_id,
        target_id,
        player_x,
        player_y,
        None,
        &CraftScanFilters::default(),
    )
}

/// [`retarget_flat_rock_near_forge`] skipping scan-blocked forge/rock tiles.
// Haxe: GetForge + GetClosestObjectToTarget(player, forge, 291, 30, 3)
pub fn retarget_flat_rock_near_forge_ex(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
) -> Option<Result<CraftWorldObj, ()>> {
    if target_id != FLAT_ROCK || ALLOWED_ON_FLAT_ROCK_NEAR_FORGE.contains(&actor_id) {
        return None;
    }
    let forge = get_forge(objs, home, player_x, player_y, filters)?;
    if craft_quad_distance(player_x, player_y, forge.x, forge.y) >= FORGE_NEAR_QUAD {
        return None;
    }
    match closest_craft_obj_to_target_filtered(
        objs,
        FLAT_ROCK,
        forge.x,
        forge.y,
        FORGE_BIAS_SEARCH_R,
        FORGE_BIAS_MIN_DIST,
        filters,
    ) {
        Some(o) => Some(Ok(o)),
        None => Some(Err(())),
    }
}

/// Clay Bowl actor near forge: pick bowl ≥ min dist from forge, or fail.
// Haxe: craftItemHelper ~6818–6835
pub fn retarget_clay_bowl_away_from_forge(
    objs: &[CraftWorldObj],
    actor_id: i32,
    actor_x: i32,
    actor_y: i32,
    actor_held: bool,
    player_x: i32,
    player_y: i32,
) -> Option<Result<CraftWorldObj, ()>> {
    retarget_clay_bowl_away_from_forge_ex(
        objs,
        actor_id,
        actor_x,
        actor_y,
        actor_held,
        player_x,
        player_y,
        None,
        &CraftScanFilters::default(),
    )
}

/// [`retarget_clay_bowl_away_from_forge`] skipping scan-blocked bowls/forges.
// Haxe: GetClosestObjectToTarget(player, forge, 235, 30, 3)
pub fn retarget_clay_bowl_away_from_forge_ex(
    objs: &[CraftWorldObj],
    actor_id: i32,
    _actor_x: i32,
    _actor_y: i32,
    _actor_held: bool,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
) -> Option<Result<CraftWorldObj, ()>> {
    if actor_id != CLAY_BOWL {
        return None;
    }
    let forge = get_forge(objs, home, player_x, player_y, filters)?;
    if craft_quad_distance(player_x, player_y, forge.x, forge.y) >= FORGE_NEAR_QUAD {
        return None;
    }
    match closest_craft_obj_to_target_filtered(
        objs,
        CLAY_BOWL,
        forge.x,
        forge.y,
        FORGE_BIAS_SEARCH_R,
        FORGE_BIAS_MIN_DIST,
        filters,
    ) {
        Some(o) => Some(Ok(o)),
        None => Some(Err(())),
    }
}

/// Fire bow + shaft: when no kindling/tinder near shaft, seek kindling first.
// Haxe: craftItemHelper ~6890–6902 GetCraftAndDrop kindling/tinder residual
pub fn fire_bow_needs_kindling(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    target_x: i32,
    target_y: i32,
    called_craft_item: bool,
) -> bool {
    fire_bow_needs_kindling_ex(
        objs,
        actor_id,
        target_id,
        target_x,
        target_y,
        called_craft_item,
        &CraftScanFilters::default(),
    )
}

/// [`fire_bow_needs_kindling`] ignoring scan-blocked kindling/tinder.
// Haxe: GetCraftAndDropItemsCloseToObj + GetClosestObjectToTarget
pub fn fire_bow_needs_kindling_ex(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    target_x: i32,
    target_y: i32,
    called_craft_item: bool,
    filters: &CraftScanFilters<'_>,
) -> bool {
    if called_craft_item || actor_id != FIRE_BOW_DRILL || target_id != LONG_STRAIGHT_SHAFT {
        return false;
    }
    let kindling =
        closest_craft_obj_filtered(objs, KINDLING, target_x, target_y, 10, None, filters);
    let tinder =
        closest_craft_obj_filtered(objs, JUNIPER_TINDER, target_x, target_y, 10, None, filters);
    kindling.is_none() && tinder.is_none()
}

// Haxe: GetCraftAndDropItemsCloseToObj + craftItemHelper specials (AI-CRAFT-LIVE-MORE)
// Haxe: ServerSettings.InitWaterSourceIds (AI-CRAFT-MULTI-SPECIALS)
include!("craft_and_drop.inc.rs");

// ── Have-set builder ────────────────────────────────────────────────────────

/// Object ids present: held + ground under dual-center (home **or** player).
///
/// Defaults to `search_current_position=true` (Haxe IntemToCraft). Prefer
/// [`craft_have_set_ex`] when `onlyHome` / sticky flag matters.
// Haxe: addAllObjectsForCraftig + searchCurrentPosition (AI-CRAFT-DUAL)
pub fn craft_have_set(
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
) -> HashSet<i32> {
    craft_have_set_ex(objs, held_id, player_x, player_y, home, radius, true)
}

// ── searchBestObjectForCrafting (AI-CRAFT-TOPDOWN + reverse-graph) ───────────

/// Find best actionable (actor, target) craft step for `product_id`.
///
/// Expands search radius by [`AI_MAX_SEARCH_INCREMENT`] (floor [`AI_CRAFT_MIN_RADIUS`])
/// up to `max_search_radius`. Uses reverse-graph path leaf→root with top-down
/// `DoTransitionSearch` skip gates (default opts — no last/meta/scan filters).
// Haxe: searchBestObjectForCrafting + searchBestTransitionTopDown (+ AI-CRAFT-TOPDOWN filters)
pub fn search_best_object_for_crafting(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    max_search_radius: i32,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> Option<CraftTransPair> {
    search_best_object_for_crafting_topdown(
        product_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        max_search_radius,
        graph,
        pile_id_for,
        &CraftTopDownOpts::default(),
    )
}

/// Filtered craft search with last-transition / scan / index / meta opts.
// Haxe: searchBestObjectForCrafting + DoTransitionSearch filters
pub fn search_best_object_for_crafting_ex(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    max_search_radius: i32,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    opts: &CraftTopDownOpts<'_>,
) -> Option<CraftTransPair> {
    search_best_object_for_crafting_topdown(
        product_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        max_search_radius,
        graph,
        pile_id_for,
        opts,
    )
}

/// Unfiltered reverse-graph search (regression / parity with pre-topdown path).
// Haxe: searchBestObjectForCrafting simplified (no DoTransitionSearch filters)
#[allow(dead_code)]
pub(crate) fn search_best_object_for_crafting_unfiltered(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    max_search_radius: i32,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> Option<CraftTransPair> {
    if product_id <= 0 {
        return None;
    }
    let max_r = if max_search_radius < 1 {
        AI_MAX_SEARCH_RADIUS
    } else {
        max_search_radius
    };

    let mut radius = 0;
    let mut best: Option<CraftTransPair> = None;

    while radius < max_r {
        radius += AI_MAX_SEARCH_INCREMENT;
        if radius > max_r {
            radius = max_r;
        }
        let scan_r = craft_init_objects_search_radius(radius);

        let have = craft_have_set(objs, held_id, player_x, player_y, home, scan_r);

        if have.contains(&product_id) {
            return None;
        }

        if let Some(pair) = find_best_pair_in_radius(
            product_id,
            objs,
            held_id,
            player_x,
            player_y,
            home,
            scan_r,
            graph,
            &have,
            pile_id_for,
        ) {
            match best {
                None => best = Some(pair),
                Some(b) if pair.distance < b.distance => best = Some(pair),
                _ => {}
            }
            if best.is_some() {
                return best;
            }
        }

        if radius >= max_r {
            break;
        }
    }
    best
}

fn find_best_pair_in_radius(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    graph: &ReverseCraftGraph,
    have: &HashSet<i32>,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> Option<CraftTransPair> {
    // Path leaf→root: first fully-present step is the multi-step action.
    if let Some(path) = graph.find_path_to_product(product_id, have, 8) {
        if path.is_empty() {
            return None;
        }
        for &(actor, target) in &path {
            if let Some(pair) = resolve_pair(
                actor,
                target,
                objs,
                held_id,
                player_x,
                player_y,
                home,
                radius,
                pile_id_for,
            ) {
                return Some(pair);
            }
        }
    }

    // Fallback: direct reverse edges for product (any ingredients both present).
    if let Some(pairs) = graph.ingredients_for(product_id) {
        let mut best: Option<CraftTransPair> = None;
        for &(actor, target) in pairs {
            if let Some(pair) = resolve_pair(
                actor,
                target,
                objs,
                held_id,
                player_x,
                player_y,
                home,
                radius,
                pile_id_for,
            ) {
                match best {
                    None => best = Some(pair),
                    Some(b) if pair.distance < b.distance => best = Some(pair),
                    _ => {}
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

fn resolve_pair(
    actor_id: i32,
    target_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> Option<CraftTransPair> {
    // Actor: empty hands / TIME / held / ground / pile
    let (ax, ay, actor_held, actor_from_pile, pile_id, actor_ok) = resolve_side(
        actor_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        radius,
        pile_id_for,
        None,
    );

    if !actor_ok {
        return None;
    }

    // Target: same id as actor uses second closest when both needed on ground.
    let exclude = if actor_id == target_id && actor_id > 0 && !actor_held {
        Some((ax, ay))
    } else {
        None
    };
    let (tx, ty, _t_held, _t_pile, _t_pile_id, target_ok) = resolve_side(
        target_id,
        objs,
        // Don't count held as target if it's already used as actor.
        if actor_held { 0 } else { held_id },
        player_x,
        player_y,
        home,
        radius,
        None, // piles mainly for actors
        exclude,
    );
    if !target_ok {
        return None;
    }

    let dist_player_actor = if actor_held || actor_id == -1 {
        0
    } else {
        craft_chebyshev(player_x, player_y, ax, ay)
    };
    let dist_actor_target = craft_chebyshev(ax, ay, tx, ty);
    let distance = dist_player_actor + dist_actor_target;

    // Preserve TIME (-1) / PLAYER (-2); clamp only loose ground ids.
    let out_actor = if actor_id < 0 {
        actor_id
    } else {
        actor_id.max(0)
    };
    let out_target = if target_id < 0 {
        target_id
    } else {
        target_id.max(0)
    };

    Some(CraftTransPair {
        actor_id: out_actor,
        actor_x: ax,
        actor_y: ay,
        actor_held,
        actor_from_pile,
        pile_id,
        target_id: out_target,
        target_x: tx,
        target_y: ty,
        distance,
        search_radius: radius,
    })
}

/// Resolve one side of a transition to a world position.
/// Returns (x, y, held, from_pile, pile_id, ok).
fn resolve_side(
    id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    exclude: Option<(i32, i32)>,
) -> (i32, i32, bool, bool, i32, bool) {
    // Empty hands ok as actor 0.
    if id == 0 {
        return (player_x, player_y, held_id == 0, false, -1, true);
    }
    // TIME (-1): actor is virtual; target side resolves separately → WaitTime path.
    // Haxe: craftItemHelper ~7019–7037
    if id == -1 {
        return (player_x, player_y, true, false, -1, true);
    }
    // PLAYER (-2): not supported (match Haxe false).
    // Haxe: craftItemHelper ~7040–7047
    if id == -2 {
        return (0, 0, false, false, -1, false);
    }
    if id < 0 {
        return (0, 0, false, false, -1, false);
    }
    if held_id == id {
        return (player_x, player_y, true, false, -1, true);
    }

    // Dual-center closest ranked by player distance (AI-CRAFT-DUAL).
    // Haxe: addObjectsForCrafting home + optional current position; rank by player quad.
    let search_current = true;
    if let Some(o) = closest_craft_obj_dual_center(
        objs,
        id,
        player_x,
        player_y,
        home,
        radius,
        search_current,
        exclude,
    ) {
        return (o.x, o.y, false, false, -1, true);
    }

    // Pile form for actor pickup (dual-center).
    if let Some(f) = pile_id_for {
        let pile = f(id);
        if pile > 0 {
            if let Some(o) = closest_craft_obj_dual_center(
                objs,
                pile,
                player_x,
                player_y,
                home,
                radius,
                search_current,
                exclude,
            ) {
                return (o.x, o.y, false, true, pile, true);
            }
        }
    }
    (0, 0, false, false, -1, false)
}

/// First missing positive ingredient on the reverse-graph path (for SeekIngredient).
pub fn first_missing_ingredient(
    product_id: i32,
    graph: &ReverseCraftGraph,
    have: &HashSet<i32>,
) -> Option<i32> {
    graph.seek_ingredient_for(product_id, have)
}

// ── craftItemMax ────────────────────────────────────────────────────────────

/// Haxe `craftItemMax(objId, max)` — true when count < max (caller then craftItem).
// Haxe: AiBase.craftItemMax L6604–6606 `countCurrentObject(objId) < max && craftItem`
pub fn craft_item_max_needed(count: i32, max: i32) -> bool {
    count < max
}

/// Haxe `AiHelper.IsCloseToObject`: `quadDistance <= distance²`.
// Haxe: AiHelper.IsCloseToObject L60–62
pub fn is_close_to_object(px: i32, py: i32, ox: i32, oy: i32, distance: i32) -> bool {
    let d = distance.max(0);
    craft_quad_distance(px, py, ox, oy) <= d * d
}

/// Haxe fail say when `itemToCraftName != null` after no transActor.
// Haxe: AiBase.craftItemHelper L6732–6734
pub fn craft_item_fail_say(item_to_craft_name: Option<&str>) -> Option<String> {
    item_to_craft_name.map(|n| format!("Failed to craft {n}"))
}

/// Haxe `shouldDebugSay` `Goto target ` + transTarget.name.
// Haxe: AiBase.craftItemHelper L7001
pub fn craft_goto_target_debug_say(debug_say: bool, target_name: &str) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!("Goto target {target_name}"))
}

/// Haxe empty actor + held not hiddenWound → dropHeldObject.
// Haxe: AiBase.craftItemHelper L7003
pub fn craft_empty_actor_should_drop_held(held_id: i32, is_hidden_wound: bool) -> bool {
    held_id != 0 && !is_hidden_wound
}

/// Haxe TIME wait: not animal and `secondsUntillChange < 10`.
// Haxe: AiBase.craftItemHelper L7024
pub fn craft_time_actor_should_wait(target_is_animal: bool, seconds_until_change: f32) -> bool {
    !target_is_animal && seconds_until_change < CRAFT_TIME_WAIT_MAX_SEC
}

/// Haxe `this.time += secondsUntillChange / 4`.
// Haxe: AiBase.craftItemHelper L7026
pub fn craft_time_actor_wait_add_sec(seconds_until_change: f32) -> f32 {
    seconds_until_change / 4.0
}

/// Haxe `Wait for ${transTarget.name}...`.
// Haxe: AiBase.craftItemHelper L7028
pub fn craft_wait_for_target_debug_say(debug_say: bool, target_name: &str) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!("Wait for {target_name}..."))
}

/// Haxe `Actor is player!?!`.
// Haxe: AiBase.craftItemHelper L7045
pub fn craft_actor_is_player_debug_say(debug_say: bool) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some("Actor is player!?!".to_string())
}

/// Haxe `Goto actor ` + transActor.name.
// Haxe: AiBase.craftItemHelper L7088
pub fn craft_goto_actor_debug_say(debug_say: bool, actor_name: &str) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!("Goto actor {actor_name}"))
}

/// Haxe `Goto piled actor ` + transActor.name.
// Haxe: AiBase.craftItemHelper L7101
pub fn craft_goto_piled_actor_debug_say(debug_say: bool, actor_name: &str) -> Option<String> {
    if !debug_say {
        return None;
    }
    Some(format!("Goto piled actor {actor_name}"))
}

/// Haxe `intitObjectsForCraftigHelper` floor: `if (radius < 15) radius = 15`.
/// May exceed `maxSearchRadius` for the object scan on that increment.
// Haxe: AiBase.intitObjectsForCraftigHelper L7195–7198
pub fn craft_init_objects_search_radius(loop_radius: i32) -> i32 {
    loop_radius.max(AI_CRAFT_MIN_RADIUS)
}

/// Haxe `cachedObjectLists[radius]` miss or `objectZero.count < 0` → refill.
// Haxe: AiBase.intitObjectsForCraftigHelper L7202–7214
pub fn craft_cached_object_list_needs_fill(sentinel_count: Option<i32>) -> bool {
    match sentinel_count {
        None => true,
        Some(c) => c < 0,
    }
}

/// Haxe `new TransitionForObject(0,0,0,null); count = -1` then addAll sets `count = 0`.
// Haxe: AiBase.intitObjectsForCraftigHelper L7206; addAllObjectsForCraftig L7227
pub fn craft_new_object_list_sentinel() -> i32 {
    -1
}

/// Haxe `for (ty in baseY-radius...baseY+radius)` half-open Chebyshev box.
// Haxe: AiBase.addObjectsForCrafting L7260–7261
pub fn craft_in_add_objects_box(
    ox: i32,
    oy: i32,
    base_x: i32,
    base_y: i32,
    radius: i32,
) -> bool {
    let r = radius.max(0);
    ox >= base_x - r && ox < base_x + r && oy >= base_y - r && oy < base_y + r
}

/// Haxe `AiHelper.IsIgnoredFloor`.
// Haxe: AiHelper.IsIgnoredFloor L42–46
pub fn craft_is_ignored_floor(
    floor_id: i32,
    is_food: bool,
    is_permanent: bool,
    ignored_floor_ids: &[i32],
) -> bool {
    if floor_id < 1 {
        return false;
    }
    if is_food || is_permanent {
        return false;
    }
    ignored_floor_ids.contains(&floor_id)
}

/// Haxe addObjectsForCrafting tile skips (dummy parent already `parent_id`).
// Haxe: AiBase.addObjectsForCrafting L7262–7290
pub fn craft_add_objects_tile_allowed(o: &CraftWorldObj, ignored_floor_ids: &[i32]) -> bool {
    if o.parent_id == 0 {
        return false;
    }
    if o.num_slots > 0 && o.contained_count > 0 {
        return false;
    }
    if craft_is_ignored_floor(o.floor_id, o.is_food, o.is_permanent, ignored_floor_ids) {
        return false;
    }
    if (o.parent_id == FERTILE_SOIL || o.parent_id == HARDENED_ROW)
        && (o.biome_id == CRAFT_BIOME_SNOW || o.biome_id == CRAFT_BIOME_OCEAN)
    {
        return false;
    }
    true
}

/// Haxe `addObjectsForCrafting(base, radius, doCount, onlyRelevant=false)` id set.
// Haxe: AiBase.addObjectsForCrafting L7251–7290 (notReachable via scan filters)
pub fn add_objects_for_crafting(
    objs: &[CraftWorldObj],
    base_x: i32,
    base_y: i32,
    radius: i32,
    ignored_floor_ids: &[i32],
    filters: &CraftScanFilters<'_>,
) -> HashSet<i32> {
    let mut have = HashSet::new();
    have.insert(0);
    for o in objs {
        if !craft_in_add_objects_box(o.x, o.y, base_x, base_y, radius) {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        if !craft_add_objects_tile_allowed(o, ignored_floor_ids) {
            continue;
        }
        have.insert(o.parent_id);
    }
    have
}

/// Haxe `addAllObjectsForCraftig`: held + home scan (`searchCurrentPosition` player scan is commented out).
// Haxe: AiBase.addAllObjectsForCraftig L7227–7242
pub fn add_all_objects_for_crafting(
    objs: &[CraftWorldObj],
    held_id: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    ignored_floor_ids: &[i32],
    filters: &CraftScanFilters<'_>,
) -> HashSet<i32> {
    let mut have = HashSet::new();
    have.insert(0);
    if held_id > 0 {
        have.insert(held_id);
    }
    if let Some((hx, hy)) = home {
        have.extend(add_objects_for_crafting(
            objs,
            hx,
            hy,
            radius,
            ignored_floor_ids,
            filters,
        ));
    }
    have
}

/// Haxe `TransitionForObject` entry in `transitionsByObjectId`.
// Haxe: AiHelper.TransitionForObject L2141
#[derive(Debug, Clone, PartialEq)]
pub struct CraftTransForObject {
    pub obj_id: i32,
    pub count: i32,
    /// Closest ground instance (x, y, player-quad-dist).
    pub closest: Option<(i32, i32, i32)>,
    pub closest_uses: i32,
    /// Haxe `closestObject.objectData.numUses` for reverse-use / full gates.
    pub closest_max_uses: i32,
    /// Second-closest (same-id actor+target).
    pub second: Option<(i32, i32, i32)>,
    pub is_done: bool,
    pub best_craft_distance: i32,
    pub best_craft_steps: i32,
    /// Haxe `craftActor` ObjectHelper (`parentId`, tx, ty) from a prior step.
    pub craft_actor: Option<(i32, i32, i32)>,
    /// Haxe `craftTarget` ObjectHelper (`parentId`, tx, ty) from a prior step.
    pub craft_target: Option<(i32, i32, i32)>,
    /// Haxe `craftTransFrom` (same assignment as `bestTransition` in L7535–7546).
    pub craft_trans_from: Option<CraftTransMeta>,
    /// Haxe `bestTransition` on the product `TransitionForObject`.
    pub best_transition: Option<CraftTransMeta>,
    /// Haxe `usePile` (actor created from pile closest).
    pub use_pile: bool,
    /// Pile parent id when `use_pile` (Haxe pile `closestObject.id`).
    pub pile_obj_id: i32,
    /// Haxe `wantedObjId` set when enqueueing a missing ingredient.
    pub wanted_obj_id: i32,
    /// Haxe `wantedObjs` (ids of TransitionForObject that need this ingredient).
    pub wanted_objs: Vec<i32>,
    /// Haxe `craftFrom` (id of the TransitionForObject that filled craftActor).
    pub craft_from: Option<i32>,
}

impl CraftTransForObject {
    pub fn new(obj_id: i32) -> Self {
        Self {
            obj_id,
            count: 0,
            closest: None,
            closest_uses: 0,
            closest_max_uses: 0,
            second: None,
            is_done: false,
            best_craft_distance: -1,
            best_craft_steps: -1,
            craft_actor: None,
            craft_target: None,
            craft_trans_from: None,
            best_transition: None,
            use_pile: false,
            pile_obj_id: 0,
            wanted_obj_id: 0,
            wanted_objs: Vec::new(),
            craft_from: None,
        }
    }
}

/// Haxe `IsDangerousHelper` half-open r=4 around the object.
// Haxe: AiHelper.IsDangerousHelper L1058–1074
pub fn craft_object_is_dangerous(
    ox: i32,
    oy: i32,
    deadly_or_hostile: &HashSet<(i32, i32)>,
) -> bool {
    let r = CRAFT_IS_DANGEROUS_RADIUS;
    for ty in (oy - r)..(oy + r) {
        for tx in (ox - r)..(ox + r) {
            if deadly_or_hostile.contains(&(tx, ty)) {
                return true;
            }
        }
    }
    false
}

fn craft_skip_dangerous_far(
    player_quad: i32,
    ox: i32,
    oy: i32,
    deadly_or_hostile: &HashSet<(i32, i32)>,
) -> bool {
    player_quad > CRAFT_DANGEROUS_QUAD_MIN
        && craft_object_is_dangerous(ox, oy, deadly_or_hostile)
}

/// Haxe addObjectsForCrafting map fill: onlyRelevant, count, carrot skip, closest/second.
// Haxe: AiBase.addObjectsForCrafting L7302–7359
pub fn add_objects_for_crafting_into(
    map: &mut HashMap<i32, CraftTransForObject>,
    objs: &[CraftWorldObj],
    base_x: i32,
    base_y: i32,
    radius: i32,
    player_x: i32,
    player_y: i32,
    only_relevant: bool,
    do_count: bool,
    has_carrot_seeds: bool,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    ignored_floor_ids: &[i32],
    filters: &CraftScanFilters<'_>,
    deadly_or_hostile: &HashSet<(i32, i32)>,
) {
    for o in objs {
        if !craft_in_add_objects_box(o.x, o.y, base_x, base_y, radius) {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        if !craft_add_objects_tile_allowed(o, ignored_floor_ids) {
            continue;
        }
        let trans = if let Some(t) = map.get_mut(&o.parent_id) {
            t
        } else if only_relevant {
            continue;
        } else {
            map.insert(o.parent_id, CraftTransForObject::new(o.parent_id));
            map.get_mut(&o.parent_id).expect("just inserted")
        };
        if do_count {
            trans.count += 1;
            let pile_id = pile_id_for.map(|f| f(o.parent_id)).unwrap_or(-1);
            if pile_id == o.parent_id {
                trans.count += o.num_uses;
            }
        }
        if o.parent_id == CARROT_ROW && !has_carrot_seeds && o.num_uses < 4 {
            continue;
        }
        if trans
            .closest
            .is_some_and(|(x, y, _)| x == o.x && y == o.y)
            || trans.second.is_some_and(|(x, y, _)| x == o.x && y == o.y)
        {
            continue;
        }
        let q = craft_quad_distance(player_x, player_y, o.x, o.y);
        if trans.closest.map(|(_, _, d)| d).unwrap_or(i32::MAX) > q {
            if craft_skip_dangerous_far(q, o.x, o.y, deadly_or_hostile) {
                continue;
            }
            trans.second = trans.closest;
            trans.closest = Some((o.x, o.y, q));
            trans.closest_uses = o.num_uses;
            trans.closest_max_uses = o.max_uses;
            continue;
        }
        if trans.second.map(|(_, _, d)| d).unwrap_or(i32::MAX) > q {
            if craft_skip_dangerous_far(q, o.x, o.y, deadly_or_hostile) {
                continue;
            }
            trans.second = Some((o.x, o.y, q));
        }
    }
}

/// Haxe bottom-up seed: `transitionsByObjectId[0]` empty and `[-1]` TIME.
// Haxe: AiBase.searchBestTransitionBottomUp L7378–7388
pub fn craft_bottom_up_seed_empty_and_time(map: &mut HashMap<i32, CraftTransForObject>) {
    for id in [0, -1] {
        let mut e = CraftTransForObject::new(id);
        e.closest = Some((0, 0, 0));
        e.is_done = true;
        e.best_craft_distance = 0;
        e.best_craft_steps = 0;
        map.insert(id, e);
    }
}

/// Haxe `for (obj in transitionsByObjectId) { obj.isDone = true; todo.push(obj.objId); }`
// Haxe: AiBase.searchBestTransitionBottomUp L7396–7399
pub fn craft_bottom_up_todo_from_map(map: &mut HashMap<i32, CraftTransForObject>) -> Vec<i32> {
    let mut todo = Vec::new();
    for e in map.values_mut() {
        e.is_done = true;
        todo.push(e.obj_id);
    }
    todo
}

/// Haxe `count > 30000` break in the bottom-up todo loop.
// Haxe: AiBase.searchBestTransitionBottomUp L7402
pub const CRAFT_BOTTOM_UP_TODO_CAP: i32 = 30000;

/// Haxe `getTransitionByActor` / `getTransitionByTarget` lists from `TransitionData`.
// Haxe: TransitionImporter.getTransitionByActor L421 / getTransitionByTarget L401
pub fn craft_bottom_up_trans_index(
    metas: &[CraftTransMeta],
) -> (
    HashMap<i32, Vec<CraftTransMeta>>,
    HashMap<i32, Vec<CraftTransMeta>>,
) {
    let mut by_actor: HashMap<i32, Vec<CraftTransMeta>> = HashMap::new();
    let mut by_target: HashMap<i32, Vec<CraftTransMeta>> = HashMap::new();
    for &m in metas {
        by_actor.entry(m.actor_id).or_default().push(m);
        by_target.entry(m.target_id).or_default().push(m);
    }
    (by_actor, by_target)
}

/// Haxe `world.getTransitionByNewActor` / `getTransitionByNewTarget` (top-down).
// Haxe: TransitionImporter.getTransitionByNewActor L430 / getTransitionByNewTarget L411
// Haxe: AiBase.searchBestTransitionTopDown L7696–7700
pub fn craft_top_down_trans_index(
    metas: &[CraftTransMeta],
) -> (
    HashMap<i32, Vec<CraftTransMeta>>,
    HashMap<i32, Vec<CraftTransMeta>>,
) {
    let mut by_new_actor: HashMap<i32, Vec<CraftTransMeta>> = HashMap::new();
    let mut by_new_target: HashMap<i32, Vec<CraftTransMeta>> = HashMap::new();
    for &m in metas {
        by_new_actor.entry(m.new_actor_id).or_default().push(m);
        by_new_target.entry(m.new_target_id).or_default().push(m);
    }
    (by_new_actor, by_new_target)
}

/// Haxe `count > 30000` break in the top-down `objectsToSearch` loop.
// Haxe: AiBase.searchBestTransitionTopDown L7761
pub const CRAFT_TOP_DOWN_TODO_CAP: i32 = 30000;

/// Haxe `if (itemToCraft.bestDistance < 100) break`.
// Haxe: AiBase.searchBestTransitionTopDown L7783
pub const CRAFT_TOP_DOWN_BEST_DIST_BREAK: i32 = 100;

/// Haxe top-down seed: empty 0, TIME -1, and wipe `objToCraftId` entry.
// Haxe: AiBase.searchBestTransitionTopDown L7710–7716
pub fn craft_top_down_seed(map: &mut HashMap<i32, CraftTransForObject>, obj_to_craft_id: i32) {
    craft_bottom_up_seed_empty_and_time(map);
    if obj_to_craft_id != 0 && obj_to_craft_id != -1 {
        map.insert(obj_to_craft_id, CraftTransForObject::new(obj_to_craft_id));
    }
}

/// Haxe `existsHardenedRow = transitionsByObjectId[848] != null && …closestObject != null`.
// Haxe: AiBase.searchBestTransitionTopDown L7721
pub fn craft_top_down_exists_hardened_row(map: &HashMap<i32, CraftTransForObject>) -> bool {
    map.get(&HARDENED_ROW)
        .and_then(|e| e.closest)
        .is_some()
}

/// Haxe `transitionsByObjectId` → `CraftObjectIndex` for DoTransitionSearch skip gates.
// Haxe: AiBase.DoTransitionSearch L7828–7878 (count / closest.numberOfUses)
pub fn craft_object_index_from_trans_map(
    map: &HashMap<i32, CraftTransForObject>,
) -> CraftObjectIndex {
    let mut idx = CraftObjectIndex::new();
    for (&id, e) in map {
        idx.set_count(id, e.count);
        if e.closest.is_some() {
            idx.set_closest_uses(id, e.closest_uses, e.closest_max_uses);
        }
    }
    idx
}

fn craft_top_down_ensure_actor(
    map: &mut HashMap<i32, CraftTransForObject>,
    actor_id: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) {
    if map.contains_key(&actor_id) {
        return;
    }
    let mut e = CraftTransForObject::new(actor_id);
    if let Some(f) = pile_id_for {
        let pile_id = f(actor_id);
        if pile_id > 0 {
            if let Some(pile) = map.get(&pile_id) {
                e.use_pile = true;
                e.pile_obj_id = pile_id;
                // Haxe copies closestObject only; closestObjectDistance stays 0.
                if let Some((x, y, _)) = pile.closest {
                    e.closest = Some((x, y, 0));
                    e.closest_uses = pile.closest_uses;
                    e.closest_max_uses = pile.closest_max_uses;
                }
            }
        }
    }
    map.insert(actor_id, e);
}

/// Haxe `DoTransitionSearch` skip gates + create-missing + enqueue + craft* sub.
///
/// `usePile` empty-hand transActor is L8007+ (next hop).
// Haxe: AiBase.DoTransitionSearch L7801–8000
pub fn do_transition_search(
    item: &mut ItemToCraftState,
    map: &mut HashMap<i32, CraftTransForObject>,
    objects_to_search: &mut VecDeque<i32>,
    wanted_id: i32,
    transitions: &[CraftTransMeta],
    exists_hardened_row: bool,
    obj_to_craft_pile_id: i32,
    search_radius: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> bool {
    let obj_to_craft_id = item.product_id;
    map.entry(wanted_id)
        .or_insert_with(|| CraftTransForObject::new(wanted_id));
    let idx = craft_object_index_from_trans_map(map);
    let mut found = false;
    for trans in transitions {
        if do_transition_search_skip_reason(
            trans,
            wanted_id,
            obj_to_craft_id,
            obj_to_craft_pile_id,
            item.last_actor_id,
            item.last_target_id,
            search_radius,
            exists_hardened_row,
            Some(&idx),
        )
        .is_some()
        {
            continue;
        }
        craft_top_down_ensure_actor(map, trans.actor_id, pile_id_for);
        map.entry(trans.target_id)
            .or_insert_with(|| CraftTransForObject::new(trans.target_id));

        let actor_id = trans.actor_id;
        let target_id = trans.target_id;
        let same = actor_id == target_id;
        let actor_closest = map.get(&actor_id).and_then(|e| e.closest);
        let actor_second = map.get(&actor_id).and_then(|e| e.second);
        let actor_craft_a = map.get(&actor_id).and_then(|e| e.craft_actor);
        let actor_craft_t = map.get(&actor_id).and_then(|e| e.craft_target);
        let actor_dist = actor_closest.map(|(_, _, d)| d).unwrap_or(0);
        let target_closest = if same {
            actor_second
        } else {
            map.get(&target_id).and_then(|e| e.closest)
        };
        let target_craft_a = map.get(&target_id).and_then(|e| e.craft_actor);
        let target_craft_t = map.get(&target_id).and_then(|e| e.craft_target);

        let mut actor_obj = actor_closest.map(|(x, y, _)| (actor_id, x, y));
        let mut target_obj = target_closest.map(|(x, y, _)| {
            (
                if same { actor_id } else { target_id },
                x,
                y,
            )
        });

        if actor_obj.is_none() {
            if let Some(a) = map.get_mut(&actor_id) {
                if !a.wanted_objs.contains(&wanted_id) {
                    a.wanted_objs.push(wanted_id);
                }
            }
        }
        if target_obj.is_none() {
            if let Some(t) = map.get_mut(&target_id) {
                if !t.wanted_objs.contains(&wanted_id) {
                    t.wanted_objs.push(wanted_id);
                }
            }
        }

        if actor_obj.is_none()
            && !map.get(&actor_id).map(|e| e.is_done).unwrap_or(true)
        {
            if let Some(a) = map.get_mut(&actor_id) {
                a.wanted_obj_id = wanted_id;
                a.is_done = true;
            }
            objects_to_search.push_back(actor_id);
        }
        if target_obj.is_none()
            && !map.get(&target_id).map(|e| e.is_done).unwrap_or(true)
        {
            if let Some(t) = map.get_mut(&target_id) {
                t.wanted_obj_id = wanted_id;
                t.is_done = true;
            }
            objects_to_search.push_back(target_id);
        }

        if actor_obj.is_none() && actor_craft_a.is_none() {
            continue;
        }
        if target_obj.is_none() && target_craft_a.is_none() {
            continue;
        }

        found = true;

        if actor_obj.is_none() {
            actor_obj = actor_craft_a;
            target_obj = actor_craft_t;
        } else if target_obj.is_none() {
            actor_obj = target_craft_a;
            target_obj = target_craft_t;
        }

        let wanted_craft_none = map
            .get(&wanted_id)
            .and_then(|w| w.craft_actor)
            .is_none();
        if wanted_craft_none {
            if let Some(w) = map.get_mut(&wanted_id) {
                w.craft_actor = actor_obj;
                w.craft_target = target_obj;
                w.craft_trans_from = Some(*trans);
            }
            let deps = map
                .get(&wanted_id)
                .map(|w| w.wanted_objs.clone())
                .unwrap_or_default();
            let mut keep = Vec::new();
            for dep in deps {
                if map.get(&dep).and_then(|e| e.craft_actor).is_some() {
                    continue;
                }
                if let Some(e) = map.get_mut(&dep) {
                    e.craft_from = Some(wanted_id);
                }
                if !objects_to_search.iter().any(|&id| id == dep) {
                    objects_to_search.push_front(dep);
                }
                keep.push(dep);
            }
            if let Some(w) = map.get_mut(&wanted_id) {
                w.wanted_objs = keep;
            }
        }

        if wanted_id != obj_to_craft_id {
            continue;
        }

        let extra = match (
            map.get(&wanted_id).and_then(|w| w.craft_actor),
            map.get(&wanted_id).and_then(|w| w.craft_target),
        ) {
            (Some((_, ax, ay)), Some((_, tx, ty))) => craft_quad_distance(ax, ay, tx, ty),
            _ => 0,
        };
        let dist = actor_dist + extra;
        if dist < item.best_distance {
            let use_pile = map.get(&actor_id).map(|e| e.use_pile).unwrap_or(false);
            if use_pile {
                // Haxe L8007–8010: transActor empty hand; transTarget = pile actorObj.
                if let Some((_, ax, ay)) = actor_obj {
                    let pile_id = map
                        .get(&actor_id)
                        .map(|e| e.pile_obj_id)
                        .filter(|&id| id > 0)
                        .unwrap_or(actor_id);
                    item.set_trans_pair(0, 0, 0, pile_id, ax, ay, dist);
                }
            } else if let (Some((aid, ax, ay)), Some((tid, tx, ty))) = (actor_obj, target_obj)
            {
                item.set_trans_pair(aid, ax, ay, tid, tx, ty, dist);
            }
        }
        calculate_craft_steps(item, map, None);
        return true;
    }
    found
}

/// Haxe `CalculateSteps` craftFrom walk + TIME-not-wanted `altActor` override.
///
/// Debug text / nozzle 2110·3891 traces skipped.
// Haxe: AiBase.CalculateSteps L8060–8150
pub fn calculate_craft_steps(
    item: &mut ItemToCraftState,
    map: &HashMap<i32, CraftTransForObject>,
    time_trans_for: Option<&dyn Fn(i32) -> Option<CraftTransMeta>>,
) -> Option<(usize, i32)> {
    item.crafting_list.clear();
    item.crafting_transitions.clear();
    let mut obj_id = item.product_id;
    for _ in 0..100 {
        let Some(obj) = map.get(&obj_id) else {
            break;
        };
        let Some(from_id) = obj.craft_from else {
            break;
        };
        if item.crafting_list.contains(&from_id) {
            break;
        }
        item.crafting_list.insert(0, from_id);
        if let Some(tr) = obj.craft_trans_from {
            item.crafting_transitions.insert(0, tr);
        }
        obj_id = from_id;
    }
    let mut first_non_time = None;
    let mut alt_actor: Option<(i32, i32, i32)> = None;
    let mut alt_target: Option<(i32, i32, i32)> = None;
    for (index, &wanted_id) in item.crafting_list.iter().enumerate() {
        let Some(time_tr) = time_trans_for.and_then(|f| f(wanted_id)) else {
            continue;
        };
        let is_time_wanted = item.crafting_list.contains(&time_tr.new_target_id);
        if is_time_wanted {
            continue;
        }
        let Some(ct) = item.crafting_transitions.get(index) else {
            continue;
        };
        let mut do_first = if ct.actor_id == wanted_id {
            ct.target_id
        } else {
            ct.actor_id
        };
        if let Some(obj) = map.get(&do_first) {
            if obj.closest.is_some() {
                // Haxe L8102–8104: on-ground → doFirst=0, do not set alt*.
                do_first = 0;
            } else {
                alt_actor = obj.craft_actor;
                alt_target = obj.craft_target;
            }
        }
        if first_non_time.is_none() {
            first_non_time = Some((index, do_first));
        }
    }
    if let Some((aid, ax, ay)) = alt_actor {
        item.trans_actor_id = Some(aid);
        item.trans_actor_x = Some(ax);
        item.trans_actor_y = Some(ay);
        if let Some((tid, tx, ty)) = alt_target {
            item.trans_target_id = Some(tid);
            item.trans_target_x = Some(tx);
            item.trans_target_y = Some(ty);
        } else {
            item.trans_target_id = None;
            item.trans_target_x = None;
            item.trans_target_y = None;
        }
    }
    first_non_time
}

/// Haxe `searchBestTransitionTopDown` seed + hardened-row + `objectsToSearch` loop.
///
/// Commented bread/knife L7742–7756 and DebugAiCrafting traces skipped.
// Haxe: AiBase.searchBestTransitionTopDown L7701–7784
pub fn search_best_transition_top_down(
    item: &mut ItemToCraftState,
    map: &mut HashMap<i32, CraftTransForObject>,
    trans_by_new_actor: &HashMap<i32, Vec<CraftTransMeta>>,
    trans_by_new_target: &HashMap<i32, Vec<CraftTransMeta>>,
    search_radius: i32,
    crafting_steps_for: Option<&dyn Fn(i32) -> i32>,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) {
    let obj_to_craft_id = item.product_id;
    item.best_distance = i32::MAX / 4;
    let mut objects_to_search = VecDeque::new();
    objects_to_search.push_back(obj_to_craft_id);
    craft_top_down_seed(map, obj_to_craft_id);
    let exists_hardened = craft_top_down_exists_hardened_row(map);
    let pile_id = pile_id_for.map(|f| f(obj_to_craft_id)).unwrap_or(-1);
    let mut count = 1;
    while !objects_to_search.is_empty() {
        if count > CRAFT_TOP_DOWN_TODO_CAP {
            break;
        }
        let wanted_id = objects_to_search.pop_front().expect("nonempty");
        let steps = crafting_steps_for.map(|f| f(wanted_id)).unwrap_or(0);
        if steps < 0 {
            continue;
        }
        count += 1;
        // Haxe: found = found || DoTransitionSearch(newActor); then newTarget (|| short-circuit).
        let mut found = false;
        if let Some(list) = trans_by_new_actor.get(&wanted_id) {
            found = do_transition_search(
                item,
                map,
                &mut objects_to_search,
                wanted_id,
                list,
                exists_hardened,
                pile_id,
                search_radius,
                pile_id_for,
            );
        }
        if !found {
            if let Some(list) = trans_by_new_target.get(&wanted_id) {
                do_transition_search(
                    item,
                    map,
                    &mut objects_to_search,
                    wanted_id,
                    list,
                    exists_hardened,
                    pile_id,
                    search_radius,
                    pile_id_for,
                );
            }
        }
        if item.trans_actor_id.is_some() {
            break;
        }
        if item.best_distance < CRAFT_TOP_DOWN_BEST_DIST_BREAK {
            break;
        }
    }
}

#[derive(Clone, Copy)]
struct BottomUpObj {
    id: i32,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
struct BottomUpEntry {
    obj_id: i32,
    closest: Option<(i32, i32, i32)>,
    second: Option<(i32, i32, i32)>,
    best_craft_distance: i32,
    craft_actor: Option<(i32, i32, i32)>,
    craft_target: Option<(i32, i32, i32)>,
}

impl BottomUpEntry {
    fn from_map(map: &HashMap<i32, CraftTransForObject>, id: i32) -> Option<Self> {
        let e = map.get(&id)?;
        Some(Self {
            obj_id: e.obj_id,
            closest: e.closest,
            second: e.second,
            best_craft_distance: e.best_craft_distance,
            craft_actor: e.craft_actor,
            craft_target: e.craft_target,
        })
    }

    fn closest_obj(self) -> Option<BottomUpObj> {
        self.closest
            .map(|(x, y, _)| BottomUpObj { id: self.obj_id, x, y })
    }

    fn second_obj(self) -> Option<BottomUpObj> {
        self.second
            .map(|(x, y, _)| BottomUpObj { id: self.obj_id, x, y })
    }
}

fn bottom_up_craft_obj(triple: Option<(i32, i32, i32)>) -> Option<BottomUpObj> {
    triple.map(|(id, x, y)| BottomUpObj { id, x, y })
}

/// Haxe `searchBestTransitionBottomUp` todo while + `DoTransitionSearchBottomUp`.
///
/// Production `searchBestObjectForCrafting` still uses top-down (`test = false`).
// Haxe: AiBase.searchBestTransitionBottomUp L7364–7438
pub fn search_best_transition_bottom_up(
    item: &mut ItemToCraftState,
    map: &mut HashMap<i32, CraftTransForObject>,
    trans_by_actor: &HashMap<i32, Vec<CraftTransMeta>>,
    trans_by_target: &HashMap<i32, Vec<CraftTransMeta>>,
) {
    item.best_distance = i32::MAX / 4;
    craft_bottom_up_seed_empty_and_time(map);
    let mut todo = craft_bottom_up_todo_from_map(map);
    craft_bottom_up_process_todo(item, map, &mut todo, trans_by_actor, trans_by_target);
}

/// Haxe todo FIFO: `shift`, skip `objId < 1`, actor then target transition lists.
// Haxe: AiBase.searchBestTransitionBottomUp L7401–7422
pub fn craft_bottom_up_process_todo(
    item: &mut ItemToCraftState,
    map: &mut HashMap<i32, CraftTransForObject>,
    todo: &mut Vec<i32>,
    trans_by_actor: &HashMap<i32, Vec<CraftTransMeta>>,
    trans_by_target: &HashMap<i32, Vec<CraftTransMeta>>,
) {
    let mut count = 1;
    let mut i = 0usize;
    while i < todo.len() {
        if count > CRAFT_BOTTOM_UP_TODO_CAP {
            break;
        }
        count += 1;
        let obj_id = todo[i];
        i += 1;
        if obj_id < 1 {
            continue;
        }
        if let Some(list) = trans_by_actor.get(&obj_id) {
            do_transition_search_bottom_up(item, map, todo, list);
        }
        if let Some(list) = trans_by_target.get(&obj_id) {
            do_transition_search_bottom_up(item, map, todo, list);
        }
    }
}

fn bottom_up_obj_triple(o: BottomUpObj) -> (i32, i32, i32) {
    (o.id, o.x, o.y)
}

/// Haxe L7518–7547: create product map entries, `todo.push` if `!isDone`, `isBest` craft*.
// Haxe: AiBase.DoTransitionSearchBottomUp L7503–7547
fn bottom_up_record_product(
    map: &mut HashMap<i32, CraftTransForObject>,
    todo: &mut Vec<i32>,
    product_id: i32,
    input_id: i32,
    distance: i32,
    actor_obj: Option<BottomUpObj>,
    target_obj: Option<BottomUpObj>,
    trans: CraftTransMeta,
) {
    let e = map
        .entry(product_id)
        .or_insert_with(|| CraftTransForObject::new(product_id));
    if !e.is_done {
        e.is_done = true;
        todo.push(product_id);
    }
    let is_best = e.best_craft_distance < 0 || distance < e.best_craft_distance;
    if input_id != product_id && is_best {
        e.best_craft_distance = distance;
        e.craft_actor = actor_obj.map(bottom_up_obj_triple);
        e.craft_target = target_obj.map(bottom_up_obj_triple);
        e.craft_trans_from = Some(trans);
        e.best_transition = Some(trans);
    }
}

/// Haxe `DoTransitionSearchBottomUp` through product enqueue / `isBest` craft*.
///
/// Commented block L7549–7690 is skipped (`/* … */`).
// Haxe: AiBase.DoTransitionSearchBottomUp L7440–7547
pub fn do_transition_search_bottom_up(
    item: &mut ItemToCraftState,
    map: &mut HashMap<i32, CraftTransForObject>,
    todo: &mut Vec<i32>,
    transitions: &[CraftTransMeta],
) {
    let wanted_id = item.product_id;
    for trans in transitions {
        if trans.ai_should_ignore {
            continue;
        }
        let Some(actor) = BottomUpEntry::from_map(map, trans.actor_id) else {
            continue;
        };
        let Some(target) = BottomUpEntry::from_map(map, trans.target_id) else {
            continue;
        };

        let mut actor_obj = actor.closest_obj();
        let mut target_obj = if trans.actor_id == trans.target_id {
            actor.second_obj()
        } else {
            target.closest_obj()
        };

        if actor_obj.is_none() && actor.craft_actor.is_none() {
            continue;
        }
        if target_obj.is_none() && target.craft_actor.is_none() {
            continue;
        }

        let mut distance = if actor_obj.is_none() {
            actor.best_craft_distance
        } else {
            actor.closest.map(|(_, _, d)| d).unwrap_or(0)
        };

        match (actor_obj, target_obj) {
            (Some(ao), Some(to)) => {
                distance += craft_quad_distance(ao.x, ao.y, to.x, to.y);
            }
            (None, Some(to)) => {
                let Some(ct) = actor.craft_target else {
                    continue;
                };
                distance += craft_quad_distance(ct.1, ct.2, to.x, to.y);
            }
            (Some(ao), None) => {
                let Some(ca) = target.craft_actor else {
                    continue;
                };
                distance += craft_quad_distance(ao.x, ao.y, ca.1, ca.2);
            }
            (None, None) => {
                let (Some(ct), Some(ca)) = (actor.craft_target, target.craft_actor) else {
                    continue;
                };
                distance += craft_quad_distance(ct.1, ct.2, ca.1, ca.2);
            }
        }

        if actor_obj.is_none() {
            actor_obj = bottom_up_craft_obj(actor.craft_actor);
            target_obj = bottom_up_craft_obj(actor.craft_target);
        } else if target_obj.is_none() {
            actor_obj = bottom_up_craft_obj(target.craft_actor);
            target_obj = bottom_up_craft_obj(target.craft_target);
        }

        let Some(actor_obj_now) = actor_obj else {
            continue;
        };
        let target_for_dist = match target_obj {
            Some(t) => t,
            None => match bottom_up_craft_obj(target.craft_target) {
                Some(t) => t,
                None => continue,
            },
        };
        distance += craft_quad_distance(
            actor_obj_now.x,
            actor_obj_now.y,
            target_for_dist.x,
            target_for_dist.y,
        );

        if (wanted_id == trans.new_actor_id || wanted_id == trans.new_target_id)
            && distance < item.best_distance
        {
            item.trans_actor_id = Some(actor_obj_now.id);
            item.trans_actor_x = Some(actor_obj_now.x);
            item.trans_actor_y = Some(actor_obj_now.y);
            if let Some(to) = target_obj {
                item.trans_target_id = Some(to.id);
                item.trans_target_x = Some(to.x);
                item.trans_target_y = Some(to.y);
            } else {
                item.trans_target_id = None;
                item.trans_target_x = None;
                item.trans_target_y = None;
            }
            item.best_distance = distance;
        }

        // Haxe L7503–7547: newActor/newTarget map insert, todo if !isDone, isBest craft*.
        bottom_up_record_product(
            map,
            todo,
            trans.new_actor_id,
            trans.actor_id,
            distance,
            actor_obj,
            target_obj,
            *trans,
        );
        bottom_up_record_product(
            map,
            todo,
            trans.new_target_id,
            trans.target_id,
            distance,
            actor_obj,
            target_obj,
            *trans,
        );
    }
}

/// Haxe `isHoldingObject && considerDropHeldObject(transTarget)` after pile/loose staging.
/// No home → drop (Haxe `dropHeldObject` at end of consider). Home==goto → keep held.
// Haxe: AiBase.craftItemHelper L7114; considerDropHeldObject L5198 / L5231
pub fn craft_helper_should_drop_held(
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    goto_x: i32,
    goto_y: i32,
) -> bool {
    if held_id < 1 {
        return false;
    }
    let Some((hx, hy)) = home else {
        return true;
    };
    consider_drop_held_object(held_id, player_x, player_y, hx, hy, goto_x, goto_y)
}

/// Haxe `craftItem` post-helper forge gate: `useTarget` or `transTarget` in 304/305/303.
///
/// Runs even when the helper returned cooldown/fail (Haxe still calls
/// `hasOrBecomeProfession('SMITH', 2)`). Overrides a successful USE with
/// [`CraftItemDecision::NeedSmithProfession`] when the AI cannot become smith.
// Haxe: AiBase.craftItem L6633–6639
pub fn craft_item_post_forge_gate(
    decision: CraftItemDecision,
    trans_target_id: Option<i32>,
    is_or_can_smith: bool,
) -> CraftItemDecision {
    let use_id = match decision {
        CraftItemDecision::UseOnTarget { target_id, .. } => Some(target_id),
        _ => None,
    };
    let use_forge = use_id.is_some_and(|id| FORGE_IDS.contains(&id))
        || trans_target_id.is_some_and(|id| FORGE_IDS.contains(&id));
    if use_forge && !is_or_can_smith {
        CraftItemDecision::NeedSmithProfession
    } else {
        decision
    }
}

/// Haxe `isStillExpectedItem`: world parent at tile equals expected parent (0==0 counts).
// Haxe: AiHelper.isStillExpectedItem L589–591
pub fn craft_use_target_still_expected(expected_parent: i32, world_parent: i32) -> bool {
    expected_parent == world_parent
}

/// Haxe craftItemHelper `held == transActor` block (before product-change reset).
///
/// Always clears `transActor`. Re-aims `useTarget` via `GetClosestObject` (r=40)
/// of `transTarget.objectData`, else the sticky tile. Returns `UseOnTarget` when
/// `isStillExpectedItem`; otherwise fall through (`None`).
// Haxe: AiBase.craftItemHelper L6666–6675
pub fn try_craft_held_trans_actor_use(
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    state: &mut ItemToCraftState,
    scan: &CraftScanFilters<'_>,
) -> Option<CraftItemDecision> {
    let (Some(aid), Some(tid), Some(stx), Some(sty)) = (
        state.trans_actor_id,
        state.trans_target_id,
        state.trans_target_x,
        state.trans_target_y,
    ) else {
        return None;
    };
    if held_id != aid || aid <= 0 {
        return None;
    }
    // Haxe: useActor = transActor; itemToCraft.transActor = null
    state.trans_actor_id = None;
    let (tid, tx, ty) = closest_craft_obj_filtered(
        objs,
        tid,
        player_x,
        player_y,
        GET_CLOSEST_OBJECT_DEFAULT_R,
        None,
        scan,
    )
    .map(|o| (o.parent_id, o.x, o.y))
    .unwrap_or((tid, stx, sty));
    let world_parent = objs
        .iter()
        .find(|o| o.x == tx && o.y == ty)
        .map(|o| o.parent_id)
        .unwrap_or(0);
    if craft_use_target_still_expected(tid, world_parent) {
        Some(CraftItemDecision::UseOnTarget {
            actor_id: aid,
            target_id: tid,
            target_x: tx,
            target_y: ty,
        })
    } else {
        None
    }
}

// ── craftItemHelper core ────────────────────────────────────────────────────

/// Pure multi-step craftItemHelper decision for one tick.
///
/// Mutates `state` sticky fields and may record fail into `failed`.
/// Graph edge ignore covers default path; pass meta via
/// [`craft_item_helper_with_meta`] for full DoTransitionSearch content meta.
// Haxe: craftItemHelper ~6646–7130
pub fn craft_item_helper(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftItemDecision {
    craft_item_helper_with_meta(objs, inp, state, failed, graph, pile_id_for, None)
}

/// Like [`craft_item_helper`] with optional full [`CraftTransMeta`] map
/// (time / reverseUse / minUseFraction / aiShouldIgnore beyond graph seed).
// Haxe: craftItemHelper + DoTransitionSearch TransitionData meta
// C-SS-AI-IGNORE / AI-CRAFT-TOPDOWN: meta_by_edge ubiquity gap-close
pub fn craft_item_helper_with_meta(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    meta_by_edge: Option<&HashMap<(i32, i32), CraftTransMeta>>,
) -> CraftItemDecision {
    craft_item_helper_ex(
        objs,
        inp,
        state,
        failed,
        graph,
        pile_id_for,
        meta_by_edge,
        CraftScanFilters::default(),
        &DEFAULT_WATER_SOURCE_IDS,
    )
}

/// Full craftItemHelper: meta + live path-reach / hostile / full-pile scan filters.
///
/// `water_source_ids` is Haxe `ServerSettings.WaterSourceIds` (empty →
/// [`DEFAULT_WATER_SOURCE_IDS`] via [`effective_water_source_ids`]).
// Haxe: craftItemHelper + addObjectsForCrafting isObjectNotReachable
// Haxe: GetClosestObject* isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
// Haxe: ServerSettings.WaterSourceIds retarget (AI-CRAFT-MULTI-SPECIALS)
pub fn craft_item_helper_ex(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    meta_by_edge: Option<&HashMap<(i32, i32), CraftTransMeta>>,
    scan: CraftScanFilters<'_>,
    water_source_ids: &[i32],
) -> CraftItemDecision {
    let product_id = inp.product_id;
    if product_id <= 0 {
        return CraftItemDecision::Failed;
    }

    // Failed crafting cooldown.
    if failed.is_cooling_down_ex(product_id, inp.now_sec, inp.ai_wait_failed_sec) {
        return CraftItemDecision::Cooldown;
    }

    // Haxe L6666–6675: held==transActor GetClosest retarget BEFORE product reset.
    if let Some(d) = try_craft_held_trans_actor_use(
        objs,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        state,
        &scan,
    ) {
        return craft_item_post_forge_gate(d, state.trans_target_id, inp.is_or_can_smith);
    }

    // maxDistance / onlyHome overlay (Haxe craftItem saves/restores around helper).
    let mut max_r = state.max_search_radius;
    if inp.max_distance > 0 {
        max_r = inp.max_distance;
    }
    if max_r < 1 {
        max_r = if inp.ai_max_search_radius >= 1 {
            inp.ai_max_search_radius
        } else {
            AI_MAX_SEARCH_RADIUS
        };
    }

    // Product change → reset sticky (Haxe re-init IntemToCraft fields L6677–6690).
    if state.product_id != product_id {
        state.reset_for_product(product_id);
    }
    state.max_search_radius = max_r;
    if inp.only_home {
        state.search_current_position = false;
    }

    // Dual-center search always uses home if set (Haxe addObjectsForCrafting home).
    // startLocation uses home only when IsCloseToObject(home, 60) (quad).
    // Haxe: AiBase.craftItemHelper L6705–6722; searchBestObjectForCrafting home scan
    let home = match (inp.home_x, inp.home_y) {
        (Some(hx), Some(hy)) => Some((hx, hy)),
        _ => None,
    };
    let home_for_start = home.filter(|&(hx, hy)| {
        is_close_to_object(
            inp.player_x,
            inp.player_y,
            hx,
            hy,
            CRAFT_START_HOME_DIST,
        )
    });

    // Already have product?
    if inp.held_id == product_id {
        return CraftItemDecision::AlreadyHave {
            object_id: product_id,
            x: inp.player_x,
            y: inp.player_y,
            held: true,
        };
    }
    if let Some(o) = closest_craft_obj_filtered(
        objs,
        product_id,
        inp.player_x,
        inp.player_y,
        max_r,
        None,
        &scan,
    ) {
        return CraftItemDecision::AlreadyHave {
            object_id: product_id,
            x: o.x,
            y: o.y,
            held: false,
        };
    }

    // Search best multi-step pair (AI-CRAFT-TOPDOWN + path-reach scan filters).
    // Haxe: searchBestObjectForCrafting + DoTransitionSearch lastActor/Target undo
    // Haxe: addObjectsForCrafting isObjectNotReachable / GetClosest hostile
    let craft_index = CraftObjectIndex::from_objs(objs, None)
        .with_ai_craft_limits(&inp.ai_craft_max, &inp.ai_craft_min);
    let exists_row = objs
        .iter()
        .any(|o| o.parent_id == HARDENED_ROW && craft_obj_passes_scan_filters(o, &scan));
    let product_pile = pile_id_for.map(|f| f(product_id)).unwrap_or(-1);
    let mut topdown_opts = CraftTopDownOpts::default()
        .with_last(state.last_actor_id, state.last_target_id)
        .with_hardened_row(exists_row)
        .with_pile(product_pile)
        .with_index(&craft_index)
        .with_search_current(state.search_current_position)
        .with_scan(scan)
        .with_search_increment(inp.ai_search_increment)
        .with_ignore_time_longer_then(inp.ai_ignore_time_transitions_longer_then);
    if let Some(map) = meta_by_edge {
        topdown_opts = topdown_opts.with_meta_map(map);
    }
    let pair = search_best_object_for_crafting_ex(
        product_id,
        objs,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        home,
        max_r,
        graph,
        pile_id_for,
        &topdown_opts,
    );

    let Some(pair) = pair else {
        // No full step — try seek leaf ingredient (scan-filtered have-set).
        let have = craft_have_set_ex_filtered(
            objs,
            inp.held_id,
            inp.player_x,
            inp.player_y,
            home,
            max_r,
            state.search_current_position,
            &scan,
        );
        if let Some(ing) = first_missing_ingredient(product_id, graph, &have) {
            if ing != product_id {
                return CraftItemDecision::SeekIngredient {
                    ingredient_id: ing,
                    for_product: product_id,
                };
            }
        }
        failed.record_fail(product_id, inp.now_sec);
        state.clear_trans();
        return CraftItemDecision::Failed;
    };

    // Mutable pair fields for specials retarget.
    let actor_id = pair.actor_id;
    let mut actor_x = pair.actor_x;
    let mut actor_y = pair.actor_y;
    let actor_held = pair.actor_held;
    let mut actor_from_pile = pair.actor_from_pile;
    let mut pile_id = pair.pile_id;
    let mut target_id = pair.target_id;
    let mut target_x = pair.target_x;
    let mut target_y = pair.target_y;

    // startLocation: home if IsCloseToObject(home, 60) else transTarget.
    // Haxe: if (startLocation == null && transTarget != null) L6705–6722
    if state.start_location.is_none() {
        state.start_location = home_for_start.or(Some((target_x, target_y)));
    }

    // ── craftItemHelper specials (Haxe ~6740–7037) ──────────────────────────

    // Adobe + Firing Adobe Kiln: only when doPotteryOnFire() would consume.
    // Haxe: AiBase.craftItemHelper L6746 `if (… && doPotteryOnFire()) return true`
    if actor_id == ADOBE && target_id == FIRING_ADOBE_KILN && inp.pottery_on_fire {
        return CraftItemDecision::DeferPottery;
    }
    // Basket of Soil shortCraftOnGround.
    if actor_id == BASKET_OF_SOIL {
        return CraftItemDecision::ShortCraftOnGround {
            object_id: BASKET_OF_SOIL,
        };
    }

    // Berry pie crust gate (253 + 264 blocked when pie count > 1).
    // Haxe: craftItemHelper ~6751–6756
    if berry_pie_crust_blocked(actor_id, target_id, objs, inp.player_x, inp.player_y, max_r) {
        return CraftItemDecision::Failed;
    }

    // Bring targets to tool: Steel Adze/Froe + Butt Log (GetCraftAndDrop).
    // Haxe: craftItemHelper ~6761–6771
    if let Some(d) = adze_froe_butt_log_craft_and_drop_ex(
        objs,
        actor_id,
        target_id,
        actor_x,
        actor_y,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.called_craft_item,
        product_id,
        &scan,
    ) {
        return d;
    }

    // Domestic Goose empty-hand: Steel Axe near stump (GetCraftAndDrop).
    // Haxe: craftItemHelper ~6773–6781
    if actor_id == 0 && target_id == DOMESTIC_GOOSE && !inp.called_craft_item {
        if closest_craft_obj_filtered(
            objs,
            STUMP,
            inp.player_x,
            inp.player_y,
            GOOSE_STUMP_SEARCH_R,
            None,
            &scan,
        )
        .is_none()
        {
            return CraftItemDecision::Failed;
        }
        if let Some(d) = goose_axe_near_stump_craft_and_drop_ex(
            objs,
            actor_id,
            target_id,
            inp.held_id,
            inp.player_x,
            inp.player_y,
            inp.called_craft_item,
            product_id,
            &scan,
        ) {
            return d;
        }
    }

    // Fire bow + shaft → GetCraftAndDrop kindling then tinder near shaft.
    // Haxe: craftItemHelper ~6890–6902
    if let Some(d) = fire_bow_kindling_craft_and_drop_ex(
        objs,
        actor_id,
        target_id,
        target_x,
        target_y,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.called_craft_item,
        product_id,
        &scan,
    ) {
        return d;
    }

    // Soil retarget for Clay Bowl.
    // Haxe: craftItemHelper ~6784–6793
    if let Some(soil) =
        retarget_soil_for_clay_bowl_ex(objs, actor_id, target_id, inp.player_x, inp.player_y, &scan)
    {
        target_id = soil.parent_id;
        target_x = soil.x;
        target_y = soil.y;
    }

    // Flat Rock near forge: forbidden actor retarget or fail.
    // Haxe: craftItemHelper ~6795–6816
    match retarget_flat_rock_near_forge_ex(
        objs,
        actor_id,
        target_id,
        inp.player_x,
        inp.player_y,
        home,
        &scan,
    ) {
        Some(Ok(rock)) => {
            target_id = rock.parent_id;
            target_x = rock.x;
            target_y = rock.y;
        }
        Some(Err(())) => return CraftItemDecision::Failed,
        None => {}
    }

    // Clay Bowl not taken from next to forge.
    // Haxe: craftItemHelper ~6818–6835
    match retarget_clay_bowl_away_from_forge_ex(
        objs,
        actor_id,
        actor_x,
        actor_y,
        actor_held,
        inp.player_x,
        inp.player_y,
        home,
        &scan,
    ) {
        Some(Ok(bowl)) => {
            actor_x = bowl.x;
            actor_y = bowl.y;
            actor_from_pile = false;
            pile_id = -1;
        }
        Some(Err(())) => return CraftItemDecision::Failed,
        None => {}
    }

    // Bowl fill anti-loops (gooseberries / dry beans).
    // Haxe: craftItemHelper ~6837–6881
    let actor_full = objs
        .iter()
        .find(|o| o.x == actor_x && o.y == actor_y && o.parent_id == actor_id)
        .map(|o| o.is_full())
        .unwrap_or(false);
    if bowl_fill_pickup_blocked(
        inp.held_id,
        actor_id,
        target_id,
        inp.last_actor_id,
        objs,
        inp.player_x,
        inp.player_y,
        max_r,
        actor_full,
    ) {
        return CraftItemDecision::Failed;
    }

    // Water-source retarget (Clay Bowl / Empty Water Pouch → closest well/water).
    // Haxe: craftItemHelper ~6905–6962 ServerSettings.WaterSourceIds
    let water_ids = effective_water_source_ids(water_source_ids);
    if let Some(w) = retarget_water_source_ex(
        objs,
        actor_id,
        target_id,
        inp.player_x,
        inp.player_y,
        max_r,
        water_ids,
        &scan,
    ) {
        target_id = w.parent_id;
        target_x = w.x;
        target_y = w.y;
    }

    // TIME actor: wait only if not animal and secondsUntilChange < 10.
    // Haxe: AiBase.craftItemHelper L7019–7037
    if actor_id == -1 {
        if craft_time_actor_should_wait(inp.target_is_animal, inp.target_seconds_until_change) {
            state.trans_actor_id = None;
            return CraftItemDecision::WaitTime;
        }
        state.clear_trans();
        return CraftItemDecision::Failed;
    }
    // PLAYER actor not supported.
    // Haxe: AiBase.craftItemHelper L7040–7047
    if actor_id == -2 {
        state.trans_actor_id = None;
        return CraftItemDecision::Failed;
    }

    state.set_trans_pair(
        actor_id,
        actor_x,
        actor_y,
        target_id,
        target_x,
        target_y,
        pair.distance,
    );

    // Forge SMITH gate (after pair chosen; Haxe craftItem post-helper).
    let uses_forge = FORGE_IDS.contains(&target_id);
    if uses_forge && !inp.is_or_can_smith {
        return CraftItemDecision::NeedSmithProfession;
    }

    // Deadly actor + sheep/cow → second closest from **home** r=30 (quad).
    // Knife 560, War Sword 3047, Mango Leaf 1878 vs Sheep 575/576 / Cow 1458.
    // Haxe: AiBase.craftItemHelper L6971–6992 GetClosestObjectToHome(..., 30, ignoreFirst)
    let deadly = [560, 3047, 1878];
    let second_close = [575, 576, 1458];
    if deadly.contains(&actor_id) && second_close.contains(&target_id) {
        let base = home.unwrap_or((inp.player_x, inp.player_y));
        if let Some(sec) = second_closest_craft_obj_quad_filtered(
            objs,
            target_id,
            base.0,
            base.1,
            DEADLY_SECOND_CLOSE_R,
            &scan,
        ) {
            target_x = sec.x;
            target_y = sec.y;
            target_id = sec.parent_id;
            state.trans_target_x = Some(target_x);
            state.trans_target_y = Some(target_y);
            state.trans_target_id = Some(target_id);
        }
    }

    // Actor already held or empty actor.
    // Haxe: held == transActor || transActor.id == 0
    if actor_held || actor_id == 0 {
        if actor_id == 0
            && craft_empty_actor_should_drop_held(inp.held_id, inp.is_hidden_wound)
        {
            return CraftItemDecision::DropHeldForEmpty;
        }
        state.trans_actor_id = None;
        return CraftItemDecision::UseOnTarget {
            actor_id,
            target_id,
            target_x,
            target_y,
        };
    }

    // Dual-center residual: pile-vs-loose *1.5 + r=6 re-anchor near craft target.
    // Haxe: craftItemHelper ~7050–7083 (AI-CRAFT-DUAL) + path-reach filters
    {
        let re = reanchor_craft_actor_near_target_filtered(
            objs,
            actor_id,
            actor_x,
            actor_y,
            actor_from_pile,
            pile_id,
            target_x,
            target_y,
            inp.player_x,
            inp.player_y,
            pile_id_for,
            &scan,
        );
        actor_x = re.actor_x;
        actor_y = re.actor_y;
        actor_from_pile = re.from_pile;
        pile_id = re.pile_id;
        state.trans_actor_x = Some(actor_x);
        state.trans_actor_y = Some(actor_y);
    }

    // Need to acquire actor — pile or loose.
    // Haxe L7108–7129: if holding && considerDropHeldObject(transTarget) return true (drop);
    // else still return true with pile USE / dropTarget=actor.
    let home_xy = match (inp.home_x, inp.home_y) {
        (Some(hx), Some(hy)) => Some((hx, hy)),
        _ => None,
    };
    if craft_helper_should_drop_held(
        inp.held_id,
        inp.player_x,
        inp.player_y,
        home_xy,
        target_x,
        target_y,
    ) {
        return CraftItemDecision::DropHeldThenPickup {
            actor_id,
            actor_x,
            actor_y,
        };
    }
    if actor_from_pile && pile_id > 0 {
        return CraftItemDecision::UsePileForActor {
            pile_id,
            x: actor_x,
            y: actor_y,
        };
    }

    CraftItemDecision::PickupActor {
        object_id: actor_id,
        x: actor_x,
        y: actor_y,
    }
}

/// Haxe `craftItem` wrapper: save/restore radius + helper + forge SMITH gate.
// Haxe: AiBase.craftItem L6611–6643
pub fn craft_item(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftItemDecision {
    let tmp_r = state.max_search_radius;
    let tmp_search = state.search_current_position;
    if inp.max_distance > 0 {
        state.max_search_radius = inp.max_distance;
    }
    if inp.only_home {
        state.search_current_position = false;
    }
    let d = craft_item_helper(objs, inp, state, failed, graph, pile_id_for);
    state.max_search_radius = tmp_r;
    state.search_current_position = tmp_search;
    craft_item_post_forge_gate(d, state.trans_target_id, inp.is_or_can_smith)
}

/// Sticky multi-tick craftItem using [`CraftAiRuntime`].
// Haxe: craftItem + Player.itemToCraft / failedCraftings / lastActorId
pub fn craft_item_with_runtime(
    objs: &[CraftWorldObj],
    product_id: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    opts: &CraftLiveExpandOpts,
    runtime: &mut CraftAiRuntime,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftItemDecision {
    craft_item_with_runtime_scan(
        objs,
        product_id,
        player_x,
        player_y,
        held_id,
        opts,
        runtime,
        graph,
        pile_id_for,
        CraftScanFilters::default(),
    )
}

/// Sticky multi-tick craftItem with path-reach / hostile scan filters.
// Haxe: craftItem + isObjectNotReachable / isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
pub fn craft_item_with_runtime_scan(
    objs: &[CraftWorldObj],
    product_id: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    opts: &CraftLiveExpandOpts,
    runtime: &mut CraftAiRuntime,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    scan: CraftScanFilters<'_>,
) -> CraftItemDecision {
    // Haxe L6666–6675: held==transActor before product-change addTask/reset.
    if let Some(d) = try_craft_held_trans_actor_use(
        objs,
        held_id,
        player_x,
        player_y,
        &mut runtime.item,
        &scan,
    ) {
        let d = craft_item_post_forge_gate(d, runtime.item.trans_target_id, opts.is_or_can_smith);
        runtime.note_craft_done(d);
        runtime.note_called_craft_item_from_decision(d);
        return d;
    }
    // Haxe: craftItemHelper product-change addTask when countDone < count ~6677
    runtime.prepare_for_product(product_id);
    let tmp_r = runtime.item.max_search_radius;
    let tmp_search = runtime.item.search_current_position;
    let inp = CraftItemInput::from_runtime(product_id, player_x, player_y, held_id, opts, runtime);
    let mut decision = craft_item_helper_ex(
        objs,
        &inp,
        &mut runtime.item,
        &mut runtime.failed,
        graph,
        pile_id_for,
        opts.trans_meta.as_ref(),
        scan,
        opts.effective_water_source_ids(),
    );
    // Haxe L6625–6626 restore maxSearchRadius / searchCurrentPosition
    runtime.item.max_search_radius = tmp_r;
    runtime.item.search_current_position = tmp_search;
    // Haxe L6628–6629 lastActorId then L6633–6639 forge gate (may still return false).
    runtime.note_craft_done(decision);
    decision = craft_item_post_forge_gate(decision, runtime.item.trans_target_id, opts.is_or_can_smith);
    runtime.note_called_craft_item_from_decision(decision);
    decision
}

// ── Live intent mapping ─────────────────────────────────────────────────────

/// Map pure [`CraftItemDecision`] → [`ShortCraftLiveIntent`].
///
/// `empty_drop` used when dropping held before empty-hand / pickup.
// Haxe: useTarget / dropTarget / dropHeldObject / craftItem staging
pub fn craft_item_decision_to_live_intent(
    decision: CraftItemDecision,
    empty_drop: Option<(i32, i32)>,
) -> ShortCraftLiveIntent {
    match decision {
        CraftItemDecision::Cooldown
        | CraftItemDecision::Failed
        | CraftItemDecision::NeedSmithProfession
        | CraftItemDecision::WaitTime
        | CraftItemDecision::DeferPottery => ShortCraftLiveIntent::None,

        CraftItemDecision::AlreadyHave {
            object_id,
            x,
            y,
            held,
        } => {
            if held {
                ShortCraftLiveIntent::None
            } else {
                // Pickup already-made product (loose).
                ShortCraftLiveIntent::DropAt { x, y }
            }
        }

        CraftItemDecision::UseOnTarget {
            actor_id,
            target_id,
            target_x,
            target_y,
        } => ShortCraftLiveIntent::UseAt {
            x: target_x,
            y: target_y,
            target_id,
            actor_id,
        },

        CraftItemDecision::DropHeldForEmpty | CraftItemDecision::DropHeldThenPickup { .. } => {
            match empty_drop {
                Some((x, y)) => ShortCraftLiveIntent::DropAt { x, y },
                None => ShortCraftLiveIntent::None,
            }
        }

        CraftItemDecision::PickupActor { x, y, .. } => ShortCraftLiveIntent::DropAt { x, y },

        CraftItemDecision::UsePileForActor { pile_id, x, y } => ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id: pile_id,
            actor_id: 0,
        },

        CraftItemDecision::SeekIngredient { ingredient_id, .. } => {
            ShortCraftLiveIntent::SeekOrCraft {
                actor: ingredient_id,
                craft_if_needed: true,
            }
        }

        CraftItemDecision::ShortCraftOnGround { object_id } => {
            ShortCraftLiveIntent::SeekGroundActor { target: object_id }
        }

        CraftItemDecision::GotoDropAnchor { target_x, target_y } => ShortCraftLiveIntent::Goto {
            x: target_x,
            y: target_y,
        },
        CraftItemDecision::DropNearAnchor { target_x, target_y } => ShortCraftLiveIntent::DropAt {
            x: target_x,
            y: target_y,
        },
    }
}

/// Expand `ShortCraftLiveIntent::CraftItem` via multi-step helper.
// Haxe: craftItem(objId) residual from GetOrCraft / shortCraft
pub fn resolve_craft_item_live(
    intent: ShortCraftLiveIntent,
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    empty_drop: Option<(i32, i32)>,
) -> ShortCraftLiveIntent {
    let object_id = match intent {
        ShortCraftLiveIntent::CraftItem { object_id } => object_id,
        other => return other,
    };
    let mut inp = inp.clone();
    inp.product_id = object_id;
    // Haxe: prepareSmithingTools craftItem(290, 60)
    if inp.max_distance <= 0 {
        let d = crate::smith_profession::smith_craft_item_max_distance(object_id);
        if d > 0 {
            inp.max_distance = d;
        }
    }
    let decision = craft_item(objs, &inp, state, failed, graph, pile_id_for);
    craft_item_decision_to_live_intent(decision, empty_drop)
}

/// Convert GetOrCraft-style `(parent_id, x, y)` list into [`CraftWorldObj`].
pub fn craft_world_objs_from_ids(items: &[(i32, i32, i32)]) -> Vec<CraftWorldObj> {
    items
        .iter()
        .map(|&(id, x, y)| CraftWorldObj::simple(id, x, y))
        .collect()
}

/// Convert from get_or_craft world objs (same fields, num_uses default 1).
pub fn craft_world_from_get_or_craft(
    parent_id: i32,
    x: i32,
    y: i32,
    num_slots: i32,
) -> CraftWorldObj {
    CraftWorldObj::simple(parent_id, x, y).with_slots(num_slots)
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    fn sample_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        // 1+2 → 3, 3+4 → 5
        g.insert(1, 2, 3, 0);
        g.insert(3, 4, 5, 0);
        g
    }

    #[test]
    fn cooldown_blocks_retry() {
        let mut failed = FailedCraftings::new();
        failed.record_fail(99, 100.0);
        assert!(failed.is_cooling_down(99, 110.0));
        assert!(!failed.is_cooling_down(99, 116.0));
        assert!(!failed.is_cooling_down(1, 110.0));
        assert!(failed.is_cooling_down_ex(99, 110.0, 15.0));
        assert!(!failed.is_cooling_down_ex(99, 110.0, 5.0));
    }

    #[test]
    fn craft_item_max_needed_threshold() {
        assert!(craft_item_max_needed(0, CRAFT_ITEM_MAX_DEFAULT));
        assert!(!craft_item_max_needed(1, CRAFT_ITEM_MAX_DEFAULT));
        assert!(craft_item_max_needed(1, 2));
        assert_eq!(CRAFT_ITEM_SMITH_MAX, 2);
    }

    #[test]
    fn craft_item_restores_search_radius_and_current_position() {
        // Haxe L6612–6626: save/restore maxSearchRadius + searchCurrentPosition
        let g = sample_graph();
        let mut state = ItemToCraftState::new(5);
        state.max_search_radius = 60;
        state.search_current_position = true;
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(5, 0, 0).with_max_distance(20);
        inp.only_home = true;
        let _ = craft_item(&[], &inp, &mut state, &mut failed, &g, None);
        assert_eq!(state.max_search_radius, 60);
        assert!(state.search_current_position);
    }

    #[test]
    fn held_trans_actor_retargets_closest_same_id() {
        // Haxe L6669: GetClosestObject(transTarget.objectData) r=40, else sticky
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(2, 3, 0),
            CraftWorldObj::simple(2, 50, 50),
        ];
        let mut state = ItemToCraftState::new(3);
        state.set_trans_pair(1, 0, 0, 2, 50, 50, 10);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0).with_held(1);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                target_x,
                target_y,
            } => {
                assert_eq!((actor_id, target_id), (1, 2));
                assert_eq!((target_x, target_y), (3, 0));
            }
            other => panic!("expected UseOnTarget on closer 2, got {other:?}"),
        }
        assert!(state.trans_actor_id.is_none());
    }

    #[test]
    fn held_trans_actor_missing_tile_falls_through() {
        // Haxe L6674: isStillExpectedItem false → do not return; transActor already null
        let g = sample_graph();
        let mut state = ItemToCraftState::new(5);
        state.set_trans_pair(1, 0, 0, 2, 50, 50, 10);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(5, 0, 0).with_held(1);
        let d = craft_item_helper(&[], &inp, &mut state, &mut failed, &g, None);
        assert!(state.trans_actor_id.is_none());
        assert!(!matches!(d, CraftItemDecision::UseOnTarget { .. }));
    }

    #[test]
    fn held_trans_actor_forge_without_smith_is_need_smith() {
        // Haxe L6633–6639 after helper sets useTarget to a forge
        let g = sample_graph();
        let objs = vec![CraftWorldObj::simple(303, 2, 2)];
        let mut state = ItemToCraftState::new(9000);
        state.set_trans_pair(308, 0, 0, 303, 2, 2, 4);
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(9000, 0, 0).with_held(308);
        inp.is_or_can_smith = false;
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(d, CraftItemDecision::NeedSmithProfession);
    }

    #[test]
    fn craft_use_target_still_expected_matches_haxe_parent_eq() {
        assert!(craft_use_target_still_expected(303, 303));
        assert!(craft_use_target_still_expected(0, 0));
        assert!(!craft_use_target_still_expected(303, 0));
        assert!(!craft_use_target_still_expected(303, 304));
    }

    #[test]
    fn multi_step_uses_leaf_pair_when_both_present() {
        let g = sample_graph();
        // Have 1 and 2 on ground — first step toward 5 is 1+2 → 3
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
            CraftWorldObj::simple(4, 20, 0),
        ];
        let mut state = ItemToCraftState::new(5);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(5, 0, 0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::PickupActor { object_id, .. } => {
                assert!(object_id == 1 || object_id == 2);
            }
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                ..
            } => {
                // if somehow empty held and actor 0 — shouldn't for 1+2
                assert!(actor_id == 1 || actor_id == 2);
                assert!(target_id == 1 || target_id == 2);
            }
            other => panic!("expected pickup or use, got {other:?}"),
        }
        assert_eq!(state.product_id, 5);
        assert!(
            state.trans_target_id.is_some() || matches!(d, CraftItemDecision::PickupActor { .. })
        );
    }

    #[test]
    fn held_actor_uses_target() {
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(2, 3, 3),
            CraftWorldObj::simple(4, 10, 10),
        ];
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0).with_held(1);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(
            d,
            CraftItemDecision::UseOnTarget {
                actor_id: 1,
                target_id: 2,
                target_x: 3,
                target_y: 3,
            }
        );
        let intent = craft_item_decision_to_live_intent(d, None);
        assert_eq!(
            intent,
            ShortCraftLiveIntent::UseAt {
                x: 3,
                y: 3,
                target_id: 2,
                actor_id: 1,
            }
        );
    }

    #[test]
    fn missing_both_seeks_or_fails() {
        let g = sample_graph();
        let objs: Vec<CraftWorldObj> = vec![];
        let mut state = ItemToCraftState::new(5);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(5, 0, 0).with_now(50.0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::SeekIngredient {
                ingredient_id,
                for_product,
            } => {
                assert_eq!(for_product, 5);
                assert!(
                    ingredient_id == 1
                        || ingredient_id == 2
                        || ingredient_id == 3
                        || ingredient_id == 4
                );
            }
            CraftItemDecision::Failed => {
                assert!(failed.is_cooling_down(5, 50.0));
            }
            other => panic!("expected seek or fail, got {other:?}"),
        }
    }

    #[test]
    fn failed_cooldown_returns_cooldown() {
        let g = sample_graph();
        let objs: Vec<CraftWorldObj> = vec![];
        let mut state = ItemToCraftState::new(5);
        let mut failed = FailedCraftings::new();
        failed.record_fail(5, 0.0);
        let inp = CraftItemInput::basic(5, 0, 0).with_now(5.0);
        assert_eq!(
            craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None),
            CraftItemDecision::Cooldown
        );
    }

    #[test]
    fn forge_needs_smith() {
        let mut g = ReverseCraftGraph::new();
        // tongs + forge → product
        g.insert(308, 303, 9000, 0);
        let objs = vec![
            CraftWorldObj::simple(308, 1, 1),
            CraftWorldObj::simple(303, 2, 2),
        ];
        let mut state = ItemToCraftState::new(9000);
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(9000, 0, 0).with_held(308);
        inp.is_or_can_smith = false;
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(d, CraftItemDecision::NeedSmithProfession);
    }

    #[test]
    fn forge_ok_when_smith() {
        let mut g = ReverseCraftGraph::new();
        g.insert(308, 303, 9000, 0);
        let objs = vec![
            CraftWorldObj::simple(308, 1, 1),
            CraftWorldObj::simple(303, 2, 2),
        ];
        let mut state = ItemToCraftState::new(9000);
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(9000, 0, 0).with_held(308);
        inp.is_or_can_smith = true;
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(
            d,
            CraftItemDecision::UseOnTarget {
                actor_id: 308,
                target_id: 303,
                target_x: 2,
                target_y: 2,
            }
        );
    }

    #[test]
    fn drop_held_before_pickup_actor() {
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        // Holding junk 99, need to pick actor for 1+2
        let inp = CraftItemInput::basic(3, 0, 0).with_held(99);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::DropHeldThenPickup { actor_id, .. } => {
                assert!(actor_id == 1 || actor_id == 2);
            }
            other => panic!("expected drop then pickup, got {other:?}"),
        }
        let intent = craft_item_decision_to_live_intent(d, Some((0, 1)));
        assert_eq!(intent, ShortCraftLiveIntent::DropAt { x: 0, y: 1 });
    }

    #[test]
    fn consider_drop_skips_when_target_closer_to_home() {
        // Haxe L5231: quad(home,target)+25 < quad(player,home) → keep held
        assert!(!craft_helper_should_drop_held(99, 20, 0, Some((0, 0)), 1, 0));
        assert!(craft_helper_should_drop_held(99, 0, 0, None, 6, 0));
        assert!(!craft_helper_should_drop_held(99, 5, 0, Some((0, 0)), 0, 0)); // goto==home
        assert_eq!(
            craft_goto_piled_actor_debug_say(true, "Stone").as_deref(),
            Some("Goto piled actor Stone")
        );
        assert_eq!(craft_init_objects_search_radius(10), AI_CRAFT_MIN_RADIUS);
        assert_eq!(craft_init_objects_search_radius(30), 30);

        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
        ];
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 20, 0)
            .with_held(99)
            .with_home(0, 0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::PickupActor { object_id, .. } => {
                assert!(object_id == 1 || object_id == 2);
            }
            other => panic!("expected pickup without drop, got {other:?}"),
        }
    }

    #[test]
    fn pile_actor_empty_hands_use_pile() {
        let mut g = ReverseCraftGraph::new();
        g.insert(10, 20, 30, 0);
        let objs = vec![
            CraftWorldObj::simple(11, 4, 4), // pile of 10
            CraftWorldObj::simple(20, 5, 5),
        ];
        let pile_fn = |id: i32| if id == 10 { 11 } else { -1 };
        let mut state = ItemToCraftState::new(30);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(30, 0, 0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, Some(&pile_fn));
        assert_eq!(
            d,
            CraftItemDecision::UsePileForActor {
                pile_id: 11,
                x: 4,
                y: 4,
            }
        );
    }

    #[test]
    fn home_sets_start_location() {
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0).with_home(1, 1);
        let _ = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(state.start_location, Some((1, 1)));
    }

    #[test]
    fn start_location_home_uses_quad_not_chebyshev() {
        // Haxe IsCloseToObject: quad <= 60². (60,60) chebyshev=60 but quad=7200 > 3600.
        assert!(is_close_to_object(0, 0, 60, 0, CRAFT_START_HOME_DIST));
        assert!(!is_close_to_object(0, 0, 60, 60, CRAFT_START_HOME_DIST));
        assert_eq!(
            craft_item_fail_say(Some("Bow")),
            Some("Failed to craft Bow".into())
        );
        assert_eq!(craft_item_fail_say(None), None);

        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0).with_home(60, 60);
        let _ = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(state.start_location, Some((6, 0))); // transTarget, not far home
    }

    #[test]
    fn resolve_craft_item_live_expands_staging() {
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 2, 2),
            CraftWorldObj::simple(2, 3, 3),
        ];
        let mut state = ItemToCraftState::default();
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(0, 0, 0); // product filled from intent
        let intent = ShortCraftLiveIntent::CraftItem { object_id: 3 };
        let resolved = resolve_craft_item_live(
            intent,
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            Some((0, 0)),
        );
        assert!(
            matches!(
                resolved,
                ShortCraftLiveIntent::DropAt { .. } | ShortCraftLiveIntent::UseAt { .. }
            ),
            "got {resolved:?}"
        );
    }

    #[test]
    fn product_change_resets_state() {
        let mut state = ItemToCraftState::new(3);
        state.count_done = 5;
        state.set_trans_pair(1, 0, 0, 2, 1, 1, 3);
        state.reset_for_product(5);
        assert_eq!(state.product_id, 5);
        assert_eq!(state.count_done, 0);
        assert!(state.trans_actor_id.is_none());
    }

    #[test]
    fn already_have_product_on_ground() {
        let g = sample_graph();
        let objs = vec![CraftWorldObj::simple(5, 8, 8)];
        let mut state = ItemToCraftState::new(5);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(5, 0, 0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(
            d,
            CraftItemDecision::AlreadyHave {
                object_id: 5,
                x: 8,
                y: 8,
                held: false,
            }
        );
    }

    #[test]
    fn search_radius_expand_finds_far_pair() {
        let g = sample_graph();
        // Beyond min 15? put at distance 20 — still within first expand to 30
        let objs = vec![
            CraftWorldObj::simple(1, 20, 0),
            CraftWorldObj::simple(2, 21, 0),
        ];
        let pair = search_best_object_for_crafting(3, &objs, 0, 0, 0, None, 60, &g, None);
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_eq!(p.actor_id, 1);
        assert_eq!(p.target_id, 2);

        // Haxe intitObjectsForCraftigHelper floors scan to 15 even if maxSearchRadius is 10.
        let near_floor = vec![
            CraftWorldObj::simple(1, 12, 0),
            CraftWorldObj::simple(2, 12, 1),
        ];
        assert!(search_best_object_for_crafting(3, &near_floor, 0, 0, 0, None, 10, &g, None).is_some());
    }

    #[test]
    fn add_objects_for_crafting_filters_and_cache() {
        assert!(craft_cached_object_list_needs_fill(None));
        assert!(craft_cached_object_list_needs_fill(Some(-1)));
        assert!(!craft_cached_object_list_needs_fill(Some(0)));
        assert_eq!(craft_new_object_list_sentinel(), -1);

        // Half-open box: includes base-r, excludes base+r.
        assert!(craft_in_add_objects_box(0, 0, 15, 15, 15));
        assert!(craft_in_add_objects_box(0, 0, 0, 0, 15));
        assert!(!craft_in_add_objects_box(15, 0, 0, 0, 15));
        assert!(craft_in_add_objects_box(-15, 0, 0, 0, 15));

        assert!(craft_is_ignored_floor(656, false, false, &CRAFT_AI_IGNORED_FLOOR_IDS));
        assert!(!craft_is_ignored_floor(656, true, false, &CRAFT_AI_IGNORED_FLOOR_IDS));
        assert!(!craft_is_ignored_floor(0, false, false, &CRAFT_AI_IGNORED_FLOOR_IDS));

        let soil_snow = CraftWorldObj::simple(FERTILE_SOIL, 1, 0).with_biome(CRAFT_BIOME_SNOW);
        assert!(!craft_add_objects_tile_allowed(&soil_snow, &[]));
        let row_ocean = CraftWorldObj::simple(HARDENED_ROW, 1, 0).with_biome(CRAFT_BIOME_OCEAN);
        assert!(!craft_add_objects_tile_allowed(&row_ocean, &[]));
        let soil_green = CraftWorldObj::simple(FERTILE_SOIL, 1, 0).with_biome(1);
        assert!(craft_add_objects_tile_allowed(&soil_green, &[]));

        let box_obj = CraftWorldObj::simple(10, 1, 0).with_slots(2).with_contained(1);
        assert!(!craft_add_objects_tile_allowed(&box_obj, &[]));
        let floor_skip = CraftWorldObj::simple(33, 1, 0).with_floor(656);
        assert!(!craft_add_objects_tile_allowed(
            &floor_skip,
            &CRAFT_AI_IGNORED_FLOOR_IDS
        ));
        let floor_food = CraftWorldObj::simple(33, 1, 0)
            .with_floor(656)
            .with_food(true);
        assert!(craft_add_objects_tile_allowed(
            &floor_food,
            &CRAFT_AI_IGNORED_FLOOR_IDS
        ));

        let objs = vec![
            CraftWorldObj::simple(10, 1, 0),
            CraftWorldObj::simple(20, 40, 0),
        ];
        let scan = CraftScanFilters::default();
        let home_ids = add_all_objects_for_crafting(
            &objs,
            99,
            Some((0, 0)),
            15,
            &[],
            &scan,
        );
        assert!(home_ids.contains(&0));
        assert!(home_ids.contains(&99));
        assert!(home_ids.contains(&10));
        assert!(!home_ids.contains(&20)); // player scan commented out; 40 is outside home box
    }

    #[test]
    fn add_objects_map_only_relevant_closest_second_carrot_danger() {
        let scan = CraftScanFilters::default();
        let empty_h = HashSet::new();
        let objs = vec![
            CraftWorldObj::simple(10, 3, 0),
            CraftWorldObj::simple(10, 8, 0),
            CraftWorldObj::simple(10, 1, 0),
            CraftWorldObj::simple(99, 2, 0),
        ];
        let mut map = HashMap::new();
        map.insert(10, CraftTransForObject::new(10));
        add_objects_for_crafting_into(
            &mut map,
            &objs,
            0,
            0,
            15,
            0,
            0,
            true,
            true,
            true,
            None,
            &[],
            &scan,
            &empty_h,
        );
        assert!(!map.contains_key(&99)); // onlyRelevant
        let t = map.get(&10).unwrap();
        assert_eq!(t.count, 3);
        assert_eq!(t.closest, Some((1, 0, 1)));
        assert_eq!(t.second, Some((3, 0, 9)));

        // onlyRelevant=false creates 99
        add_objects_for_crafting_into(
            &mut map,
            &objs,
            0,
            0,
            15,
            0,
            0,
            false,
            true,
            true,
            None,
            &[],
            &scan,
            &empty_h,
        );
        assert!(map.contains_key(&99));

        // pile extra uses when parentId == pileObjId
        let pile_fn = |id: i32| if id == 50 { 50 } else { -1 };
        let piles = vec![CraftWorldObj::simple(50, 1, 0).with_uses(5)];
        let mut pm = HashMap::new();
        add_objects_for_crafting_into(
            &mut pm,
            &piles,
            0,
            0,
            15,
            0,
            0,
            false,
            true,
            true,
            Some(&pile_fn),
            &[],
            &scan,
            &empty_h,
        );
        assert_eq!(pm.get(&50).unwrap().count, 6); // 1 + 5 uses

        // carrot row uses<4 without seeds: counted but no closest
        let carrots = vec![CraftWorldObj::simple(CARROT_ROW, 2, 0).with_uses(2)];
        let mut cm = HashMap::new();
        add_objects_for_crafting_into(
            &mut cm,
            &carrots,
            0,
            0,
            15,
            0,
            0,
            false,
            true,
            false,
            None,
            &[],
            &scan,
            &empty_h,
        );
        let c = cm.get(&CARROT_ROW).unwrap();
        assert_eq!(c.count, 1);
        assert!(c.closest.is_none());

        // danger skip when quad > 4
        let far = vec![CraftWorldObj::simple(7, 5, 0)]; // quad=25 > 4
        let mut danger = HashSet::new();
        danger.insert((5, 0));
        let mut dm = HashMap::new();
        add_objects_for_crafting_into(
            &mut dm,
            &far,
            0,
            0,
            15,
            0,
            0,
            false,
            true,
            true,
            None,
            &[],
            &scan,
            &danger,
        );
        assert!(dm.get(&7).unwrap().closest.is_none());

        craft_bottom_up_seed_empty_and_time(&mut dm);
        assert!(dm.get(&0).unwrap().is_done);
        assert_eq!(dm.get(&-1).unwrap().best_craft_distance, 0);
        let todo = craft_bottom_up_todo_from_map(&mut dm);
        assert!(todo.contains(&0));
        assert!(todo.contains(&-1));
        assert!(todo.contains(&7));
        assert!(dm.values().all(|e| e.is_done));
    }

    fn bu_entry_at(id: i32, x: i32, y: i32, player_x: i32, player_y: i32) -> CraftTransForObject {
        let mut e = CraftTransForObject::new(id);
        e.closest = Some((x, y, craft_quad_distance(player_x, player_y, x, y)));
        e
    }

    fn dts(
        item: &mut ItemToCraftState,
        map: &mut HashMap<i32, CraftTransForObject>,
        wanted: i32,
        trans: &[CraftTransMeta],
        hardened: bool,
        pile: i32,
        radius: i32,
    ) -> bool {
        do_transition_search(
            item,
            map,
            &mut VecDeque::new(),
            wanted,
            trans,
            hardened,
            pile,
            radius,
            None,
        )
    }

    #[test]
    fn bottom_up_todo_direct_wanted_pair_and_filters() {
        // Haxe L7401–7498: FIFO skip objId<1; aiShouldIgnore; both map entries;
        // distance = actorClosestQuad + pairwise + pairwise again; strict bestDistance.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0)); // closest quad 9
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0)); // closest quad 16
        let pair = CraftTransMeta::pair(1, 2, 3, 0);
        let ignored = CraftTransMeta::pair(1, 2, 3, 0).with_ai_should_ignore(true);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[ignored]);
        let mut item = ItemToCraftState::new(3);
        let mut todo = vec![0, -1, 1];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        assert!(item.trans_actor_id.is_none());

        let (by_a, by_t) = craft_bottom_up_trans_index(&[pair]);
        // Missing target in map → skip.
        let mut only_actor = HashMap::new();
        only_actor.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        let mut item = ItemToCraftState::new(3);
        let mut todo = vec![1];
        craft_bottom_up_process_todo(&mut item, &mut only_actor, &mut todo, &by_a, &by_t);
        assert!(item.trans_actor_id.is_none());

        let mut item = ItemToCraftState::new(3);
        let mut todo = vec![0, -1, 1];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        // pairwise (3,0)-(0,4) = 25; start 9; +25 +25 = 59
        assert_eq!(item.trans_actor_id, Some(1));
        assert_eq!(item.trans_target_id, Some(2));
        assert_eq!((item.trans_actor_x, item.trans_actor_y), (Some(3), Some(0)));
        assert_eq!((item.trans_target_x, item.trans_target_y), (Some(0), Some(4)));
        assert_eq!(item.best_distance, 9 + 25 + 25);

        // Same distance does not replace (strict <).
        let prev = item.best_distance;
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![1], &by_a, &by_t);
        assert_eq!(item.best_distance, prev);

        // Wanted as newTargetId also matches.
        let as_new_target = CraftTransMeta::pair(1, 2, 0, 9);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[as_new_target]);
        let mut item = ItemToCraftState::new(9);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![2], &by_a, &by_t);
        assert_eq!(item.trans_actor_id, Some(1));
        assert_eq!(item.trans_target_id, Some(2));
        assert_eq!(CRAFT_BOTTOM_UP_TODO_CAP, 30000);
    }

    #[test]
    fn bottom_up_same_id_uses_second_and_empty_hand() {
        // Haxe L7459: actor==target → secondObject.
        let mut map = HashMap::new();
        let mut e = CraftTransForObject::new(10);
        e.closest = Some((1, 0, 1));
        e.second = Some((4, 0, 16));
        map.insert(10, e);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[CraftTransMeta::pair(10, 10, 20, 0)]);
        let mut item = ItemToCraftState::new(20);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![10], &by_a, &by_t);
        assert_eq!(item.trans_actor_id, Some(10));
        assert_eq!(item.trans_target_id, Some(10));
        assert_eq!((item.trans_actor_x, item.trans_actor_y), (Some(1), Some(0)));
        assert_eq!((item.trans_target_x, item.trans_target_y), (Some(4), Some(0)));
        // start 1 + pairwise 9 + pairwise 9
        assert_eq!(item.best_distance, 1 + 9 + 9);

        // Empty-hand target 0 from seed at (0,0).
        let mut map = HashMap::new();
        map.insert(5, bu_entry_at(5, 3, 0, 0, 0));
        craft_bottom_up_seed_empty_and_time(&mut map);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[CraftTransMeta::pair(5, 0, 6, 0)]);
        let mut item = ItemToCraftState::new(6);
        search_best_transition_bottom_up(&mut item, &mut map, &by_a, &by_t);
        assert_eq!(item.trans_actor_id, Some(5));
        assert_eq!(item.trans_target_id, Some(0));
        assert_eq!((item.trans_target_x, item.trans_target_y), (Some(0), Some(0)));
        // closest 9 + pairwise to (0,0) 9 + 9
        assert_eq!(item.best_distance, 9 + 9 + 9);
    }

    #[test]
    fn bottom_up_craft_actor_substitution_distance() {
        // Haxe L7461–7489: missing closest uses craftActor/craftTarget; then replace objs.
        let mut map = HashMap::new();
        let mut actor = CraftTransForObject::new(1);
        actor.best_craft_distance = 7;
        actor.craft_actor = Some((50, 1, 2));
        actor.craft_target = Some((51, 5, 2));
        map.insert(1, actor);
        map.insert(2, bu_entry_at(2, 0, 6, 0, 0));
        let mut item = ItemToCraftState::new(3);
        do_transition_search_bottom_up(
            &mut item,
            &mut map,
            &mut Vec::new(),
            &[CraftTransMeta::pair(1, 2, 3, 0)],
        );
        // start 7 + dist(craftTarget 5,2 → target 0,6)=25+16=41 → 48;
        // replace with actor.craft*; extra dist(1,2)-(5,2)=16 → 64
        assert_eq!(item.trans_actor_id, Some(50));
        assert_eq!(item.trans_target_id, Some(51));
        assert_eq!((item.trans_actor_x, item.trans_actor_y), (Some(1), Some(2)));
        assert_eq!(item.best_distance, 7 + 41 + 16);

        // Actor on ground, target missing → replace BOTH with target.craft*.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 2, 0, 0, 0)); // closest quad 4
        let mut target = CraftTransForObject::new(2);
        target.craft_actor = Some((80, 8, 0));
        target.craft_target = Some((81, 10, 0));
        map.insert(2, target);
        let mut item = ItemToCraftState::new(3);
        do_transition_search_bottom_up(
            &mut item,
            &mut map,
            &mut Vec::new(),
            &[CraftTransMeta::pair(1, 2, 3, 0)],
        );
        // start 4 + dist((2,0),(8,0))=36 → 40; replace; extra dist((8,0),(10,0))=4 → 44
        assert_eq!(item.trans_actor_id, Some(80));
        assert_eq!(item.trans_target_id, Some(81));
        assert_eq!(item.best_distance, 4 + 36 + 4);

        // Both missing: pairwise craftTarget→craftActor then replace with actor.craft*.
        let mut map = HashMap::new();
        let mut actor = CraftTransForObject::new(1);
        actor.best_craft_distance = 3;
        actor.craft_actor = Some((50, 1, 0));
        actor.craft_target = Some((51, 4, 0));
        map.insert(1, actor);
        let mut target = CraftTransForObject::new(2);
        target.craft_actor = Some((80, 8, 0));
        target.craft_target = Some((81, 10, 0));
        map.insert(2, target);
        let mut item = ItemToCraftState::new(3);
        do_transition_search_bottom_up(
            &mut item,
            &mut map,
            &mut Vec::new(),
            &[CraftTransMeta::pair(1, 2, 3, 0)],
        );
        // 3 + dist((4,0),(8,0))=16 → 19; extra dist((1,0),(4,0))=9 → 28
        assert_eq!(item.trans_actor_id, Some(50));
        assert_eq!(item.trans_target_id, Some(51));
        assert_eq!(item.best_distance, 3 + 16 + 9);
    }

    #[test]
    fn bottom_up_strict_better_distance_replaces_and_skips_dead_ends() {
        // Haxe L7461 / L7494: no closest+no craftActor skip; only wanted newActor/newTarget;
        // later pair with smaller distance replaces (strict <).
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 10, 0, 0, 0)); // closest quad 100
        map.insert(2, bu_entry_at(2, 0, 0, 0, 0));
        map.insert(4, bu_entry_at(4, 1, 0, 0, 0)); // closest quad 1
        let mut dead = CraftTransForObject::new(8);
        dead.craft_actor = None;
        map.insert(8, dead);
        let far = CraftTransMeta::pair(1, 2, 3, 0);
        let close = CraftTransMeta::pair(4, 2, 3, 0);
        let not_wanted = CraftTransMeta::pair(1, 2, 99, 0);
        let missing_craft = CraftTransMeta::pair(8, 2, 3, 0);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[far, close, not_wanted, missing_craft]);

        let mut item = ItemToCraftState::new(3);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![8], &by_a, &by_t);
        assert!(item.trans_actor_id.is_none());

        let mut item = ItemToCraftState::new(3);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![1], &by_a, &by_t);
        // far pair: 100 + 100 + 100; not_wanted (99) ignored
        assert_eq!(item.trans_actor_id, Some(1));
        assert_eq!(item.best_distance, 300);

        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![4], &by_a, &by_t);
        // close pair: 1 + 1 + 1 = 3 replaces 300
        assert_eq!(item.trans_actor_id, Some(4));
        assert_eq!(item.trans_target_id, Some(2));
        assert_eq!(item.best_distance, 3);

        let mut item = ItemToCraftState::new(7);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![1, 4], &by_a, &by_t);
        assert!(item.trans_actor_id.is_none());
    }

    #[test]
    fn bottom_up_enqueues_new_products_and_is_best() {
        // Haxe L7503–7547: create missing newActor; !isDone → todo.push; isBest craft*.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        let pair = CraftTransMeta::pair(1, 2, 3, 0);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[pair]);
        let mut item = ItemToCraftState::new(99);
        let mut todo = vec![1];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        assert!(todo.contains(&3));
        let p = map.get(&3).unwrap();
        assert!(p.is_done);
        assert_eq!(p.best_craft_distance, 9 + 25 + 25);
        assert_eq!(p.craft_actor, Some((1, 3, 0)));
        assert_eq!(p.craft_target, Some((2, 0, 4)));
        assert_eq!(p.best_transition, Some(pair));
        assert_eq!(p.craft_trans_from, Some(pair));
        assert!(item.trans_actor_id.is_none());

        // Already isDone → do not enqueue again; same distance does not replace.
        let mut todo2 = vec![1];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo2, &by_a, &by_t);
        assert!(!todo2.contains(&3));
        assert_eq!(map.get(&3).unwrap().best_craft_distance, 59);

        // Closer recipe replaces craft* (strict <).
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        map.insert(4, bu_entry_at(4, 1, 0, 0, 0));
        let far = CraftTransMeta::pair(1, 2, 3, 0);
        let close = CraftTransMeta::pair(4, 2, 3, 0);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[far, close]);
        let mut item = ItemToCraftState::new(99);
        craft_bottom_up_process_todo(&mut item, &mut map, &mut vec![1], &by_a, &by_t);
        assert_eq!(map.get(&3).unwrap().craft_actor, Some((1, 3, 0)));
        let mut todo = vec![4];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        assert!(!todo.contains(&3));
        let p = map.get(&3).unwrap();
        // 4@(1,0) + 2@(0,4): closest 1 + pairwise 17 + 17
        assert_eq!(p.best_craft_distance, 1 + 17 + 17);
        assert_eq!(p.craft_actor, Some((4, 1, 0)));
        assert_eq!(p.best_transition, Some(close));
    }

    #[test]
    fn bottom_up_identity_skip_and_multistep_via_craft_actor() {
        // Haxe L7528: actorID==newActorID does not write craft* on that product.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 0, 0, 0));
        let ident = CraftTransMeta::pair(1, 2, 1, 5);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[ident]);
        let mut item = ItemToCraftState::new(99);
        let mut todo = vec![1];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        assert!(map.get(&1).unwrap().craft_actor.is_none());
        assert!(map.get(&1).unwrap().is_done);
        let five = map.get(&5).unwrap();
        assert!(five.is_done);
        assert_eq!(five.craft_actor, Some((1, 3, 0)));
        assert_eq!(five.craft_target, Some((2, 0, 0)));
        assert!(todo.contains(&5));

        // 1+2→3 then 3+4→7: 3 not on ground; wanted uses actor.craft* after sub.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        map.insert(4, bu_entry_at(4, 6, 0, 0, 0));
        let step1 = CraftTransMeta::pair(1, 2, 3, 0);
        let step2 = CraftTransMeta::pair(3, 4, 7, 0);
        let (by_a, by_t) = craft_bottom_up_trans_index(&[step1, step2]);
        let mut item = ItemToCraftState::new(7);
        let mut todo = vec![1, 4];
        craft_bottom_up_process_todo(&mut item, &mut map, &mut todo, &by_a, &by_t);
        assert!(map.get(&3).unwrap().is_done);
        assert_eq!(map.get(&3).unwrap().craft_actor, Some((1, 3, 0)));
        assert_eq!(item.trans_actor_id, Some(1));
        assert_eq!(item.trans_target_id, Some(2));
    }

    #[test]
    fn top_down_world_index_by_new_actor_and_new_target() {
        // Haxe L7696–7700: searchBestTransitionTopDown binds itemToCraft + getWorld()
        // (getTransitionByNewActor / getTransitionByNewTarget).
        let item = ItemToCraftState::new(7);
        assert_eq!(item.product_id, 7);
        let a = CraftTransMeta::pair(1, 2, 7, 0);
        let b = CraftTransMeta::pair(3, 4, 0, 7);
        let c = CraftTransMeta::pair(5, 6, 8, 9);
        let (by_na, by_nt) = craft_top_down_trans_index(&[a, b, c]);
        assert_eq!(by_na.get(&7).map(|v| v.as_slice()), Some(&[a][..]));
        assert_eq!(by_nt.get(&7).map(|v| v.as_slice()), Some(&[b][..]));
        assert_eq!(by_na.get(&8).map(|v| v.as_slice()), Some(&[c][..]));
        assert_eq!(by_nt.get(&9).map(|v| v.as_slice()), Some(&[c][..]));
        assert!(by_na.get(&1).is_none());
        assert!(by_nt.get(&2).is_none());
    }

    #[test]
    fn top_down_seed_wipes_product_and_dummy_empty_time() {
        // Haxe L7710–7716: overwrite 0/-1 with dummy closest; wipe objToCraftId.
        let mut map = HashMap::new();
        map.insert(7, bu_entry_at(7, 4, 0, 0, 0));
        craft_top_down_seed(&mut map, 7);
        assert!(map.get(&0).unwrap().is_done);
        assert_eq!(map.get(&0).unwrap().closest, Some((0, 0, 0)));
        assert!(map.get(&-1).unwrap().is_done);
        assert_eq!(map.get(&-1).unwrap().closest, Some((0, 0, 0)));
        let p = map.get(&7).unwrap();
        assert!(!p.is_done);
        assert!(p.closest.is_none());
        assert_eq!(CRAFT_TOP_DOWN_TODO_CAP, 30000);
        assert_eq!(CRAFT_TOP_DOWN_BEST_DIST_BREAK, 100);
    }

    #[test]
    fn top_down_hardened_row_and_direct_wanted_pair() {
        // Haxe L7721: 848 with closest → existsHardenedRow.
        let mut map = HashMap::new();
        assert!(!craft_top_down_exists_hardened_row(&map));
        map.insert(HARDENED_ROW, bu_entry_at(HARDENED_ROW, 2, 0, 0, 0));
        assert!(craft_top_down_exists_hardened_row(&map));

        // Direct 1+2→3 with both on ground.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0)); // closest quad 9
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0)); // closest quad 16
        let pair = CraftTransMeta::pair(1, 2, 3, 0);
        let (by_na, by_nt) = craft_top_down_trans_index(&[pair]);
        let mut item = ItemToCraftState::new(3);
        search_best_transition_top_down(
            &mut item, &mut map, &by_na, &by_nt, 15, None, None,
        );
        assert_eq!(item.trans_actor_id, Some(1));
        assert_eq!(item.trans_target_id, Some(2));
        assert_eq!((item.trans_actor_x, item.trans_actor_y), (Some(3), Some(0)));
        assert_eq!((item.trans_target_x, item.trans_target_y), (Some(0), Some(4)));
        // actor closest 9 + pairwise 25
        assert_eq!(item.best_distance, 9 + 25);

        // carftingSteps < 0 skips wanted (Haxe L7769).
        let mut item = ItemToCraftState::new(3);
        let steps = |id: i32| if id == 3 { -1 } else { 0 };
        search_best_transition_top_down(
            &mut item,
            &mut map,
            &by_na,
            &by_nt,
            15,
            Some(&steps),
            None,
        );
        assert!(item.trans_actor_id.is_none());
    }

    #[test]
    fn top_down_hardened_row_skips_hoe_soil() {
        // Haxe L7723–7729: 848 present → 850/857+1138 aiShouldIgnore.
        let mut map = HashMap::new();
        map.insert(HARDENED_ROW, bu_entry_at(HARDENED_ROW, 1, 0, 0, 0));
        map.insert(STONE_HOE, bu_entry_at(STONE_HOE, 2, 0, 0, 0));
        map.insert(FERTILE_SOIL, bu_entry_at(FERTILE_SOIL, 3, 0, 0, 0));
        let hoe = CraftTransMeta::pair(STONE_HOE, FERTILE_SOIL, 99, 0);
        let (by_na, by_nt) = craft_top_down_trans_index(&[hoe]);
        let mut item = ItemToCraftState::new(99);
        search_best_transition_top_down(
            &mut item, &mut map, &by_na, &by_nt, 15, None, None,
        );
        assert!(item.trans_actor_id.is_none());

        map.remove(&HARDENED_ROW);
        let mut item = ItemToCraftState::new(99);
        search_best_transition_top_down(
            &mut item, &mut map, &by_na, &by_nt, 15, None, None,
        );
        assert_eq!(item.trans_actor_id, Some(STONE_HOE));
        assert_eq!(item.trans_target_id, Some(FERTILE_SOIL));
    }

    #[test]
    fn do_transition_search_skip_gates_use_trans_map() {
        // Haxe L7808–7878: skip gates read transitionsByObjectId count / closest uses.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        let pair = CraftTransMeta::pair(1, 2, 3, 0);

        // reverseUseTarget full on newTarget 50
        let mut map_ru = map.clone();
        let mut fifty = CraftTransForObject::new(50);
        fifty.closest = Some((5, 0, 25));
        fifty.closest_uses = 5;
        fifty.closest_max_uses = 5;
        map_ru.insert(50, fifty);
        let ru = CraftTransMeta::pair(1, 2, 3, 50).with_reverse_use_target(true);
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_ru, 3, &[ru], false, -1, 15));
        assert!(item.trans_actor_id.is_none());
        map_ru.get_mut(&50).unwrap().closest_uses = 3;
        let mut item = ItemToCraftState::new(3);
        assert!(dts(&mut item, &mut map_ru, 3, &[ru], false, -1, 15));
        assert_eq!(item.trans_actor_id, Some(1));

        // targetMinUseFraction==1 and not full (maxUses>1)
        let mut t2 = bu_entry_at(2, 0, 4, 0, 0);
        t2.closest_uses = 2;
        t2.closest_max_uses = 5;
        let mut map_nf = map.clone();
        map_nf.insert(2, t2);
        let nf = pair.with_target_min_use_fraction(1.0);
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_nf, 3, &[nf], false, -1, 15));

        // ignoreIfMaxIsReached: default aiCraftMax=1, count>=1 skips
        let mut map_max = map.clone();
        let mut bowls = CraftTransForObject::new(235);
        bowls.count = 1;
        map_max.insert(235, bowls);
        let mx = pair.with_ignore_if_max(235);
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_max, 3, &[mx], false, -1, 15));
        map_max.get_mut(&235).unwrap().count = 0;
        let mut item = ItemToCraftState::new(3);
        assert!(dts(&mut item, &mut map_max, 3, &[mx], false, -1, 15));

        // ignoreIfMin: missing map entry does not skip; count 0 does (aiCraftMin default 0)
        let mn = pair.with_ignore_if_min(152);
        let mut item = ItemToCraftState::new(3);
        let mut map_mn = map.clone();
        assert!(dts(&mut item, &mut map_mn, 3, &[mn], false, -1, 15));
        let mut map_min = map.clone();
        map_min.insert(152, CraftTransForObject::new(152)); // count 0
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_min, 3, &[mn], false, -1, 15));
        // searchRadius>=40 always skips min-gated
        let mut map_r40 = map.clone();
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_r40, 3, &[mn], false, -1, 40));

        // undo last transition
        let mut map_undo = map.clone();
        let mut item = ItemToCraftState::new(3);
        item.last_actor_id = 3;
        item.last_target_id = 0;
        assert!(!dts(&mut item, &mut map_undo, 3, &[pair], false, -1, 15));

        // TIME too long
        let long = pair.with_auto_decay(200.0);
        let mut map_time = map.clone();
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_time, 3, &[long], false, -1, 15));

        // product pile as target
        let mut map_pile = map.clone();
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_pile, 3, &[pair], false, 2, 15));

        // actor is wanted/product
        let as_actor = CraftTransMeta::pair(3, 2, 4, 0);
        let mut map_aw = map.clone();
        let mut item = ItemToCraftState::new(3);
        assert!(!dts(&mut item, &mut map_aw, 3, &[as_actor], false, -1, 15));
    }

    #[test]
    fn do_transition_search_creates_missing_pile_and_enqueues() {
        // Haxe L7913–7957: missing actor from pile closest; !isDone → objectsToSearch.push.
        let mut map = HashMap::new();
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        let mut pile = bu_entry_at(11, 5, 0, 0, 0);
        pile.closest_uses = 3;
        pile.closest_max_uses = 4;
        map.insert(11, pile);
        let pair = CraftTransMeta::pair(10, 2, 3, 0);
        let pile_fn = |id: i32| if id == 10 { 11 } else { -1 };
        let mut item = ItemToCraftState::new(3);
        let mut q = VecDeque::new();
        assert!(do_transition_search(
            &mut item,
            &mut map,
            &mut q,
            3,
            &[pair],
            false,
            -1,
            15,
            Some(&pile_fn),
        ));
        let a = map.get(&10).unwrap();
        assert!(a.use_pile);
        assert_eq!(a.pile_obj_id, 11);
        assert_eq!(a.closest, Some((5, 0, 0))); // distance not copied
        // Haxe L8007–8010: empty-hand actor + pile as target
        assert_eq!(item.trans_actor_id, Some(0));
        assert_eq!(item.trans_target_id, Some(11));
        assert_eq!((item.trans_target_x, item.trans_target_y), (Some(5), Some(0)));
        assert!(q.is_empty()); // actorObj from pile, not enqueued

        // Missing actor, no pile: enqueue, no trans.
        let mut map = HashMap::new();
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        let mut item = ItemToCraftState::new(3);
        let mut q = VecDeque::new();
        assert!(!do_transition_search(
            &mut item,
            &mut map,
            &mut q,
            3,
            &[pair],
            false,
            -1,
            15,
            None,
        ));
        assert!(item.trans_actor_id.is_none());
        assert_eq!(q, VecDeque::from([10]));
        let a = map.get(&10).unwrap();
        assert!(a.is_done);
        assert_eq!(a.wanted_obj_id, 3);
        assert_eq!(a.wanted_objs, vec![3]);
        assert!(!a.use_pile);
        assert!(map.get(&2).unwrap().wanted_objs.is_empty()); // target on ground
    }

    #[test]
    fn do_transition_search_craft_actor_sub_and_wanted_unshift() {
        // Haxe L7959–7993: craftActor fallback; first wanted.craftActor fills + unshift deps.
        let mut map = HashMap::new();
        map.insert(6, bu_entry_at(6, 4, 0, 0, 0));
        let mut five = CraftTransForObject::new(5);
        five.craft_actor = Some((50, 1, 0));
        five.craft_target = Some((51, 2, 0));
        map.insert(5, five);
        let mut seven = CraftTransForObject::new(7);
        seven.wanted_objs.push(9);
        map.insert(7, seven);
        map.insert(9, CraftTransForObject::new(9));
        let pair = CraftTransMeta::pair(5, 6, 7, 0);
        let mut item = ItemToCraftState::new(7);
        let mut q = VecDeque::new();
        assert!(do_transition_search(
            &mut item,
            &mut map,
            &mut q,
            7,
            &[pair],
            false,
            -1,
            15,
            None,
        ));
        // actor closest none → dist 0 + pairwise craft pair (1,0)-(2,0)=1
        assert_eq!(item.trans_actor_id, Some(50));
        assert_eq!(item.trans_target_id, Some(51));
        assert_eq!(item.best_distance, 1);
        assert_eq!(map.get(&7).unwrap().craft_actor, Some((50, 1, 0)));
        assert_eq!(map.get(&9).unwrap().craft_from, Some(7));
        assert_eq!(q.front(), Some(&9));

        // wantedId != product: found, no trans on item.
        let mut map = HashMap::new();
        map.insert(1, bu_entry_at(1, 3, 0, 0, 0));
        map.insert(2, bu_entry_at(2, 0, 4, 0, 0));
        let mut item = ItemToCraftState::new(99);
        let mut q = VecDeque::new();
        assert!(do_transition_search(
            &mut item,
            &mut map,
            &mut q,
            3,
            &[CraftTransMeta::pair(1, 2, 3, 0)],
            false,
            -1,
            15,
            None,
        ));
        assert!(item.trans_actor_id.is_none());
        assert_eq!(map.get(&3).unwrap().craft_actor, Some((1, 3, 0)));
    }

    #[test]
    fn calculate_craft_steps_walks_craft_from_chain() {
        // Haxe L8060–8078: unshift craftFrom ids; break on null or cycle.
        let mut map = HashMap::new();
        let mut seven = CraftTransForObject::new(7);
        seven.craft_from = Some(5);
        seven.craft_trans_from = Some(CraftTransMeta::pair(5, 6, 7, 0));
        map.insert(7, seven);
        let mut five = CraftTransForObject::new(5);
        five.craft_from = Some(1);
        five.craft_trans_from = Some(CraftTransMeta::pair(1, 2, 5, 0));
        map.insert(5, five);
        map.insert(1, CraftTransForObject::new(1));
        let mut item = ItemToCraftState::new(7);
        let non_time = calculate_craft_steps(&mut item, &map, None);
        assert_eq!(item.crafting_list, vec![1, 5]);
        assert_eq!(item.crafting_transitions.len(), 2);
        assert_eq!(item.crafting_transitions[0].actor_id, 1);
        assert_eq!(item.crafting_transitions[1].actor_id, 5);
        assert!(non_time.is_none());

        // cycle: 7→5→5
        map.get_mut(&5).unwrap().craft_from = Some(5);
        let mut item = ItemToCraftState::new(7);
        calculate_craft_steps(&mut item, &map, None);
        assert_eq!(item.crafting_list, vec![5]);

        // TIME trans whose newTarget is not in the list → first doFirst
        map.get_mut(&5).unwrap().craft_from = Some(1);
        let mut item = ItemToCraftState::new(7);
        let time_fn = |id: i32| {
            if id == 5 {
                Some(CraftTransMeta::pair(-1, 5, 0, 99))
            } else {
                None
            }
        };
        let hit = calculate_craft_steps(&mut item, &map, Some(&time_fn));
        // craftingTransitions[1] is 5+6→7; actor==wanted 5 → doFirst target 6
        assert_eq!(hit, Some((1, 6)));

        // Haxe L8102–8104: doFirst on ground → doFirst=0, no altActor override.
        map.insert(6, bu_entry_at(6, 8, 0, 0, 0));
        let mut item = ItemToCraftState::new(7);
        item.set_trans_pair(1, 3, 0, 2, 0, 4, 34);
        let hit = calculate_craft_steps(&mut item, &map, Some(&time_fn));
        assert_eq!(hit, Some((1, 0)));
        assert_eq!(item.trans_actor_id, Some(1));

        // Haxe L8108–8124: no closest → altActor/Target replace trans.
        let mut six = CraftTransForObject::new(6);
        six.craft_actor = Some((50, 1, 0));
        six.craft_target = Some((51, 2, 0));
        map.insert(6, six);
        let mut item = ItemToCraftState::new(7);
        item.set_trans_pair(1, 3, 0, 2, 0, 4, 34);
        calculate_craft_steps(&mut item, &map, Some(&time_fn));
        assert_eq!(item.trans_actor_id, Some(50));
        assert_eq!(item.trans_target_id, Some(51));
        assert_eq!((item.trans_actor_x, item.trans_actor_y), (Some(1), Some(0)));
    }

    #[test]
    fn water_retarget_prefers_closest_well() {
        // Distant pond 999 as initial water target; well 663 closer.
        let objs = vec![
            CraftWorldObj::simple(999, 40, 0),
            CraftWorldObj::simple(663, 5, 0),
        ];
        let ret = retarget_water_source(&objs, CLAY_BOWL, 999, 0, 0, 60, &[663, 662, 999]);
        assert_eq!(ret.map(|o| o.parent_id), Some(663));
        assert_eq!(ret.map(|o| o.x), Some(5));
    }

    #[test]
    fn water_retarget_uses_quad_rank_and_default_r40() {
        // (5,5) chebyshev 5 but quad 50; (6,0) chebyshev 6 quad 36 → Haxe picks (6,0).
        let objs = vec![
            CraftWorldObj::simple(663, 5, 5),
            CraftWorldObj::simple(663, 6, 0),
        ];
        let ret = retarget_water_source(&objs, CLAY_BOWL, 663, 0, 0, 60, &[663]);
        assert_eq!(ret.map(|o| (o.x, o.y)), Some((6, 0)));
        // r=40 Chebyshev box: (50,0) excluded even if caller passed max_r=60.
        let far = vec![
            CraftWorldObj::simple(663, 50, 0),
            CraftWorldObj::simple(662, 3, 0),
        ];
        let ret = retarget_water_source(&far, CLAY_BOWL, 663, 0, 0, 60, &[663, 662]);
        assert_eq!(ret.map(|o| o.parent_id), Some(662));
    }

    #[test]
    fn deadly_actor_second_closest_from_home_quad() {
        let sheep = 575;
        let objs = vec![
            CraftWorldObj::simple(sheep, 1, 0),
            CraftWorldObj::simple(sheep, 4, 0),
        ];
        let sec = second_closest_craft_obj_quad_filtered(
            &objs,
            sheep,
            0,
            0,
            DEADLY_SECOND_CLOSE_R,
            &CraftScanFilters::default(),
        );
        assert_eq!(sec.map(|o| o.x), Some(4));
        assert!(second_closest_craft_obj_quad_filtered(
            &objs[..1],
            sheep,
            0,
            0,
            DEADLY_SECOND_CLOSE_R,
            &CraftScanFilters::default(),
        )
        .is_none());
    }

    #[test]
    fn soil_retarget_closest_of_pile_or_loose() {
        let objs = vec![
            CraftWorldObj::simple(FERTILE_SOIL, 20, 0),
            CraftWorldObj::simple(FERTILE_SOIL_PILE, 4, 0),
        ];
        let ret = retarget_soil_for_clay_bowl(&objs, CLAY_BOWL, FERTILE_SOIL, 0, 0);
        assert_eq!(ret.map(|o| o.parent_id), Some(FERTILE_SOIL_PILE));
    }

    #[test]
    fn berry_pie_crust_gate_blocks_when_pies_gt_one() {
        let objs = vec![
            CraftWorldObj::simple(RAW_BERRY_PIE, 1, 0),
            CraftWorldObj::simple(COOKED_BERRY_PIE, 2, 0),
            CraftWorldObj::simple(RAW_PIE_CRUST, 3, 0),
        ];
        assert!(berry_pie_crust_blocked(
            BOWL_OF_GOOSEBERRIES,
            RAW_PIE_CRUST,
            &objs,
            0,
            0,
            30
        ));
        assert!(!berry_pie_crust_blocked(
            BOWL_OF_GOOSEBERRIES,
            RAW_PIE_CRUST,
            &objs[..1],
            0,
            0,
            30
        ));
    }

    #[test]
    fn bowl_fill_blocks_without_bushes_or_last_actor() {
        let objs = vec![CraftWorldObj::simple(BOWL_OF_GOOSEBERRIES, 2, 0)];
        assert!(bowl_fill_pickup_blocked(
            0,
            BOWL_OF_GOOSEBERRIES,
            99, // not a bush
            -1,
            &objs,
            0,
            0,
            30,
            false
        ));
        assert!(bowl_fill_pickup_blocked(
            0,
            BOWL_OF_GOOSEBERRIES,
            99,
            BOWL_OF_GOOSEBERRIES, // last_actor match
            &objs,
            0,
            0,
            30,
            false
        ));
        let with_bush = vec![
            CraftWorldObj::simple(BOWL_OF_GOOSEBERRIES, 2, 0),
            CraftWorldObj::simple(30, 3, 0),
        ];
        assert!(!bowl_fill_pickup_blocked(
            0,
            BOWL_OF_GOOSEBERRIES,
            99,
            -1,
            &with_bush,
            0,
            0,
            30,
            false
        ));
        assert!(!bowl_fill_pickup_blocked(
            0,
            BOWL_OF_GOOSEBERRIES,
            99,
            -1,
            &objs,
            0,
            0,
            30,
            true
        ));
        assert_eq!(
            bowl_gooseberry_fill_debug_say(true, 2).as_deref(),
            Some("Bushed to fill Bowl of Gooseberries: 2")
        );
        assert_eq!(
            bowl_beans_fill_debug_say(true, "Bowl of Dry Beans", 0).as_deref(),
            Some("Yargets to fill Bowl of Dry Beans 0")
        );
        assert_eq!(bowl_gooseberry_fill_debug_say(false, 2), None);
    }

    #[test]
    fn flat_rock_near_forge_retargets_or_fails() {
        // Forge at (1,0), flat rock at (1,1) too close; far rock at (10,0).
        let objs = vec![
            CraftWorldObj::simple(303, 1, 0),
            CraftWorldObj::simple(FLAT_ROCK, 1, 1),
            CraftWorldObj::simple(FLAT_ROCK, 10, 0),
        ];
        // Forbidden actor (knife 560) on flat rock
        match retarget_flat_rock_near_forge(&objs, 560, FLAT_ROCK, 0, 0) {
            Some(Ok(o)) => {
                assert_eq!(o.x, 10);
            }
            other => panic!("expected retarget, got {other:?}"),
        }
        // Only close rock → fail
        let only_close = vec![
            CraftWorldObj::simple(303, 1, 0),
            CraftWorldObj::simple(FLAT_ROCK, 1, 1),
        ];
        assert!(matches!(
            retarget_flat_rock_near_forge(&only_close, 560, FLAT_ROCK, 0, 0),
            Some(Err(()))
        ));
        // Allowed tongs actor → no change
        assert!(retarget_flat_rock_near_forge(&objs, 308, FLAT_ROCK, 0, 0).is_none());
        // Haxe quad < 10: player (0,0) forge (3,1) = 10 → do not retarget
        let far_enough = vec![
            CraftWorldObj::simple(303, 3, 1),
            CraftWorldObj::simple(FLAT_ROCK, 3, 1),
            CraftWorldObj::simple(FLAT_ROCK, 10, 0),
        ];
        assert!(retarget_flat_rock_near_forge(&far_enough, 560, FLAT_ROCK, 0, 0).is_none());
    }

    #[test]
    fn clay_bowl_away_from_forge() {
        let objs = vec![
            CraftWorldObj::simple(303, 1, 0),
            CraftWorldObj::simple(CLAY_BOWL, 1, 1), // near forge
            CraftWorldObj::simple(CLAY_BOWL, 12, 0),
        ];
        match retarget_clay_bowl_away_from_forge(&objs, CLAY_BOWL, 1, 1, false, 0, 0) {
            Some(Ok(o)) => assert_eq!(o.x, 12),
            other => panic!("expected far bowl, got {other:?}"),
        }
    }

    #[test]
    fn fire_bow_needs_kindling_when_none_near() {
        let shaft = CraftWorldObj::simple(LONG_STRAIGHT_SHAFT, 5, 5);
        let objs = vec![shaft];
        assert!(fire_bow_needs_kindling(
            &objs,
            FIRE_BOW_DRILL,
            LONG_STRAIGHT_SHAFT,
            5,
            5,
            false
        ));
        let with_k = vec![shaft, CraftWorldObj::simple(KINDLING, 6, 5)];
        assert!(!fire_bow_needs_kindling(
            &with_k,
            FIRE_BOW_DRILL,
            LONG_STRAIGHT_SHAFT,
            5,
            5,
            false
        ));
    }

    #[test]
    fn sticky_failed_craftings_across_runtime_calls() {
        let g = sample_graph();
        let mut runtime = CraftAiRuntime::new();
        let opts = CraftLiveExpandOpts::default().with_now(100.0);
        // No ingredients → fail records cooldown
        let d1 = craft_item_with_runtime(&[], 5, 0, 0, 0, &opts, &mut runtime, &g, None);
        assert!(matches!(
            d1,
            CraftItemDecision::Failed | CraftItemDecision::SeekIngredient { .. }
        ));
        // Within 15s → Cooldown
        let opts2 = CraftLiveExpandOpts::default().with_now(110.0);
        let d2 = craft_item_with_runtime(&[], 5, 0, 0, 0, &opts2, &mut runtime, &g, None);
        // SeekIngredient does not record fail; Failed does. Ensure fail path recorded.
        if matches!(d1, CraftItemDecision::Failed) {
            assert_eq!(d2, CraftItemDecision::Cooldown);
        } else {
            // If seek, force-record fail to prove sticky map
            runtime.failed.record_fail(5, 100.0);
            let d3 = craft_item_with_runtime(&[], 5, 0, 0, 0, &opts2, &mut runtime, &g, None);
            assert_eq!(d3, CraftItemDecision::Cooldown);
        }
    }

    #[test]
    fn sticky_held_trans_actor_continues_use() {
        let g = sample_graph();
        let mut runtime = CraftAiRuntime::new();
        runtime.item = ItemToCraftState::new(3);
        runtime.item.set_trans_pair(1, 0, 0, 2, 4, 4, 4);
        let opts = CraftLiveExpandOpts::default();
        let objs = vec![CraftWorldObj::simple(2, 4, 4)];
        let d = craft_item_with_runtime(
            &objs,
            3,
            0,
            0,
            1, // holding actor
            &opts,
            &mut runtime,
            &g,
            None,
        );
        assert_eq!(
            d,
            CraftItemDecision::UseOnTarget {
                actor_id: 1,
                target_id: 2,
                target_x: 4,
                target_y: 4,
            }
        );
    }

    #[test]
    fn time_actor_pair_yields_wait_time() {
        let mut g = ReverseCraftGraph::new();
        // TIME (-1) + target 50 → product 51
        g.insert(-1, 50, 51, 0);
        let objs = vec![CraftWorldObj::simple(50, 2, 2)];
        let mut state = ItemToCraftState::new(51);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(51, 0, 0);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(d, CraftItemDecision::WaitTime);
    }

    #[test]
    fn time_actor_animal_or_long_wait_fails() {
        assert!(craft_time_actor_should_wait(false, 0.0));
        assert!(craft_time_actor_should_wait(false, 9.9));
        assert!(!craft_time_actor_should_wait(false, 10.0));
        assert!(!craft_time_actor_should_wait(true, 1.0));
        assert_eq!(craft_time_actor_wait_add_sec(8.0), 2.0);
        assert!(!craft_empty_actor_should_drop_held(33, true));
        assert!(craft_empty_actor_should_drop_held(33, false));
        assert!(!craft_empty_actor_should_drop_held(0, false));
        assert_eq!(
            craft_goto_target_debug_say(true, "Well").as_deref(),
            Some("Goto target Well")
        );
        assert_eq!(
            craft_wait_for_target_debug_say(true, "Oven").as_deref(),
            Some("Wait for Oven...")
        );
        assert_eq!(
            craft_actor_is_player_debug_say(true).as_deref(),
            Some("Actor is player!?!")
        );
        assert_eq!(
            craft_goto_actor_debug_say(true, "Clay Bowl").as_deref(),
            Some("Goto actor Clay Bowl")
        );
        assert_eq!(craft_goto_target_debug_say(false, "Well"), None);

        let mut g = ReverseCraftGraph::new();
        g.insert(-1, 50, 51, 0);
        let objs = vec![CraftWorldObj::simple(50, 2, 2)];
        let mut state = ItemToCraftState::new(51);
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(51, 0, 0);
        inp.target_is_animal = true;
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(d, CraftItemDecision::Failed);
    }

    /// Full meta_by_edge path into craft_item_helper (C-SS-AI-IGNORE / topdown gap-close).
    // Haxe: DoTransitionSearch aiShouldIgnore via TransitionData meta
    #[test]
    fn craft_item_helper_with_meta_skips_ignored_edge() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        g.insert(4, 5, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
            CraftWorldObj::simple(4, 3, 0),
            CraftWorldObj::simple(5, 4, 0),
        ];
        let mut meta_map = HashMap::new();
        meta_map.insert(
            (1, 2),
            CraftTransMeta::pair(1, 2, 3, 0).with_ai_should_ignore(true),
        );
        meta_map.insert((4, 5), CraftTransMeta::pair(4, 5, 3, 0));
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0);
        let d = craft_item_helper_with_meta(
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            Some(&meta_map),
        );
        // Should pick non-ignored (4,5) path → go get actor or use.
        match d {
            CraftItemDecision::PickupActor { object_id, .. } => assert_eq!(object_id, 4),
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                ..
            } => assert_eq!((actor_id, target_id), (4, 5)),
            CraftItemDecision::DropHeldThenPickup { actor_id, .. } => assert_eq!(actor_id, 4),
            CraftItemDecision::UsePileForActor { .. } => {}
            other => panic!("expected craft step via (4,5), got {other:?}"),
        }
    }

    #[test]
    fn water_bowl_helper_retargets_in_craft() {
        let mut g = ReverseCraftGraph::new();
        // Clay Bowl + Deep Well 663 → Bowl of Water 382
        g.insert(CLAY_BOWL, 663, 382, 0);
        // Far well first in scan order; close well at x=3 — retarget picks closest.
        let objs = vec![
            CraftWorldObj::simple(663, 40, 0),
            CraftWorldObj::simple(663, 3, 0),
        ];
        let mut state = ItemToCraftState::new(382);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(382, 0, 0).with_held(CLAY_BOWL);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        match d {
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                target_x,
                ..
            } => {
                assert_eq!(actor_id, CLAY_BOWL);
                assert_eq!(target_id, 663);
                // Closest water among DEFAULT_WATER_SOURCE_IDS
                assert_eq!(target_x, 3);
            }
            other => panic!("expected UseOnTarget, got {other:?}"),
        }
    }

    #[test]
    fn craft_live_opts_water_ids_retarget_pond() {
        // InitWaterSourceIds-derived list includes pond 511; defaults would miss it.
        let mut g = ReverseCraftGraph::new();
        g.insert(CLAY_BOWL, 511, BOWL_OF_WATER, 0);
        g.insert(CLAY_BOWL, 663, BOWL_OF_WATER, 0);
        let objs = vec![
            CraftWorldObj::simple(511, 2, 0),
            CraftWorldObj::simple(663, 30, 0),
        ];
        let mut runtime = CraftAiRuntime::new();
        let (water, bucket) = init_water_source_ids([
            (CLAY_BOWL, 511, BOWL_OF_WATER),
            (CLAY_BOWL, 663, BOWL_OF_WATER),
            (EMPTY_BUCKET, 511, FULL_BUCKET_WATER),
        ]);
        let opts = CraftLiveExpandOpts::default().with_water_source_ids(water, bucket);
        assert_eq!(opts.effective_water_source_ids(), &[511, 663]);
        assert_eq!(opts.effective_bucket_water_source_ids(), &[511]);
        let d = craft_item_with_runtime(
            &objs,
            BOWL_OF_WATER,
            0,
            0,
            CLAY_BOWL,
            &opts,
            &mut runtime,
            &g,
            None,
        );
        match d {
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                target_x,
                ..
            } => {
                assert_eq!(actor_id, CLAY_BOWL);
                assert_eq!(target_id, 511);
                assert_eq!(target_x, 2);
            }
            other => panic!("expected UseOnTarget on pond 511, got {other:?}"),
        }
    }

    #[test]
    fn home_opts_set_start_location_via_runtime() {
        let g = sample_graph();
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let mut runtime = CraftAiRuntime::new();
        let opts = CraftLiveExpandOpts::default().with_home(2, 2);
        let _ = craft_item_with_runtime(&objs, 3, 0, 0, 0, &opts, &mut runtime, &g, None);
        assert_eq!(runtime.item.start_location, Some((2, 2)));
    }

    #[test]
    fn smith_false_with_forge_returns_need_smith() {
        let mut g = ReverseCraftGraph::new();
        g.insert(308, 303, 9000, 0);
        let objs = vec![
            CraftWorldObj::simple(308, 1, 1),
            CraftWorldObj::simple(303, 2, 2),
        ];
        let mut runtime = CraftAiRuntime::new();
        let opts = CraftLiveExpandOpts::default().with_smith(false);
        let d = craft_item_with_runtime(&objs, 9000, 0, 0, 308, &opts, &mut runtime, &g, None);
        assert_eq!(d, CraftItemDecision::NeedSmithProfession);
    }

    /// Blocked target tile forces alternate craft object (AI-CRAFT-LIVE-RESID).
    // Haxe: addObjectsForCrafting isObjectNotReachable skip
    #[test]
    fn scan_filters_skip_blocked_target_picks_alt() {
        let mut g = ReverseCraftGraph::new();
        // 1+2→3; two targets at (2,0) blocked and (8,0) free
        g.insert(1, 2, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
            CraftWorldObj::simple(2, 8, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((2, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0).with_held(1);
        let d = craft_item_helper_ex(
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            None,
            scan,
            &DEFAULT_WATER_SOURCE_IDS,
        );
        match d {
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                target_x,
                target_y,
            } => {
                assert_eq!((actor_id, target_id), (1, 2));
                assert_eq!((target_x, target_y), (8, 0));
            }
            other => panic!("expected UseOnTarget on free target, got {other:?}"),
        }
    }

    /// All actor tiles blocked → no pair / fail (or seek), not pickup on blocked.
    // Haxe: isObjectNotReachable continues until no usable object
    #[test]
    fn scan_filters_block_all_actors_fails_or_seeks() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((1, 0));
        blocked.insert((2, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let mut state = ItemToCraftState::new(3);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(3, 0, 0);
        let d = craft_item_helper_ex(
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            None,
            scan,
            &DEFAULT_WATER_SOURCE_IDS,
        );
        assert!(
            matches!(
                d,
                CraftItemDecision::Failed
                    | CraftItemDecision::SeekIngredient { .. }
                    | CraftItemDecision::Cooldown
            ),
            "expected fail/seek when all tiles blocked, got {d:?}"
        );
    }

    /// Product on blocked tile is not treated as AlreadyHave.
    #[test]
    fn scan_filters_skip_blocked_product_already_have() {
        let g = ReverseCraftGraph::new();
        let objs = vec![CraftWorldObj::simple(99, 3, 3)];
        let mut blocked = HashSet::new();
        blocked.insert((3, 3));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let mut state = ItemToCraftState::new(99);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(99, 0, 0);
        let d = craft_item_helper_ex(
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            None,
            scan,
            &DEFAULT_WATER_SOURCE_IDS,
        );
        assert!(
            !matches!(d, CraftItemDecision::AlreadyHave { .. }),
            "blocked product must not AlreadyHave, got {d:?}"
        );
    }

    /// Water retarget skips a closer blocked well (AI-CRAFT-MULTI-RESID).
    // Haxe: GetClosestObjectToPositionByIds isObjectNotReachable
    #[test]
    fn water_retarget_skips_blocked_well_picks_alt() {
        let objs = vec![
            CraftWorldObj::simple(663, 2, 0),
            CraftWorldObj::simple(663, 8, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((2, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let ret = retarget_water_source_ex(
            &objs,
            CLAY_BOWL,
            663,
            0,
            0,
            60,
            &DEFAULT_WATER_SOURCE_IDS,
            &scan,
        );
        assert_eq!(ret.map(|o| (o.parent_id, o.x)), Some((663, 8)));
        // Unfiltered still prefers the blocked closer well.
        let unf = retarget_water_source(&objs, CLAY_BOWL, 663, 0, 0, 60, &DEFAULT_WATER_SOURCE_IDS);
        assert_eq!(unf.map(|o| o.x), Some(2));
    }

    /// Soil retarget skips blocked pile, picks loose soil.
    // Haxe: GetClosestObjectToPositionByIds(soilTargets, 30, myPlayer)
    #[test]
    fn soil_retarget_skips_blocked_pile() {
        let objs = vec![
            CraftWorldObj::simple(FERTILE_SOIL_PILE, 4, 0),
            CraftWorldObj::simple(FERTILE_SOIL, 20, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((4, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let ret = retarget_soil_for_clay_bowl_ex(&objs, CLAY_BOWL, FERTILE_SOIL, 0, 0, &scan);
        assert_eq!(ret.map(|o| o.parent_id), Some(FERTILE_SOIL));
        let mut all = HashSet::new();
        all.insert((4, 0));
        all.insert((20, 0));
        let scan_all = CraftScanFilters::new().with_blocked(&all);
        assert!(retarget_soil_for_clay_bowl_ex(
            &objs,
            CLAY_BOWL,
            FERTILE_SOIL_PILE,
            0,
            0,
            &scan_all
        )
        .is_none());
    }

    /// Helper water specials must not undo path-filtered pair search.
    // Haxe: craftItemHelper water retarget uses GetClosest with myPlayer
    #[test]
    fn helper_water_retarget_skips_blocked_well() {
        let mut g = ReverseCraftGraph::new();
        g.insert(CLAY_BOWL, 663, BOWL_OF_WATER, 0);
        let objs = vec![
            CraftWorldObj::simple(663, 2, 0),
            CraftWorldObj::simple(663, 8, 0),
        ];
        let mut blocked = HashSet::new();
        blocked.insert((2, 0));
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let mut state = ItemToCraftState::new(BOWL_OF_WATER);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(BOWL_OF_WATER, 0, 0).with_held(CLAY_BOWL);
        let d = craft_item_helper_ex(
            &objs,
            &inp,
            &mut state,
            &mut failed,
            &g,
            None,
            None,
            scan,
            &DEFAULT_WATER_SOURCE_IDS,
        );
        match d {
            CraftItemDecision::UseOnTarget {
                actor_id,
                target_id,
                target_x,
                target_y,
            } => {
                assert_eq!((actor_id, target_id), (CLAY_BOWL, 663));
                assert_eq!((target_x, target_y), (8, 0));
            }
            other => panic!("expected UseOnTarget on free well, got {other:?}"),
        }
    }

    /// NPC runtime re-queues unfinished countDone on product switch.
    // Haxe: craftItemHelper ~6677–6679 addTask
    #[test]
    fn runtime_prepare_requeues_unfinished_count_done() {
        let mut rt = CraftAiRuntime::new();
        rt.item = ItemToCraftState::new(7);
        rt.item.count = 2;
        rt.item.count_done = 1;
        rt.prepare_for_product(11);
        assert!(rt.crafting_tasks.contains(&7));
        assert_eq!(rt.item.product_id, 11);
        assert_eq!(rt.item.count_done, 0);
        assert_eq!(rt.take_next_crafting_task(), Some(7));
        assert_eq!(rt.item.product_id, 7);
        assert!(rt.crafting_tasks.is_empty());
    }

    #[test]
    fn runtime_prepare_skips_requeue_when_finished() {
        let mut rt = CraftAiRuntime::new();
        rt.item = ItemToCraftState::new(7);
        rt.item.count = 1;
        rt.item.count_done = 1;
        rt.prepare_for_product(11);
        assert!(!rt.crafting_tasks.contains(&7));
        assert_eq!(rt.item.product_id, 11);
    }

    #[test]
    fn runtime_prepare_no_requeue_when_count_zero_uninit() {
        let mut rt = CraftAiRuntime::new();
        rt.item.product_id = 7;
        rt.item.count = 0;
        rt.item.count_done = 0;
        rt.prepare_for_product(11);
        // 0 < 0 is false — uninitialized IntemToCraft does not queue.
        assert!(rt.crafting_tasks.is_empty());
        assert_eq!(rt.item.product_id, 11);
    }

    #[test]
    fn craft_item_with_runtime_scan_product_switch_requeues() {
        let g = ReverseCraftGraph::new();
        let mut runtime = CraftAiRuntime::new();
        runtime.item = ItemToCraftState::new(7);
        runtime.item.count = 2;
        runtime.item.count_done = 1;
        let opts = CraftLiveExpandOpts::default();
        let _d = craft_item_with_runtime_scan(
            &[],
            11,
            0,
            0,
            0,
            &opts,
            &mut runtime,
            &g,
            None,
            CraftScanFilters::default(),
        );
        assert!(runtime.crafting_tasks.contains(&7));
        assert_eq!(runtime.item.product_id, 11);
        assert_eq!(runtime.item.count_done, 0);
        // New product just reset: count=1, count_done=0 → unfinished for the NEW id.
        assert_eq!(runtime.item.count, 1);
        assert!(runtime.should_continue_unfinished());
    }
}
