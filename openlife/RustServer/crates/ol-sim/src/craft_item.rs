//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN** + **AI-CRAFT-DUAL**).
//!
//! Ports Haxe `AiBase.craftItem` / `craftItemHelper` / `searchBestObjectForCrafting`
//! against reverse-graph + world object snapshot, with top-down `DoTransitionSearch`
//! filters and hostile/unreachable scan gates ([`craft_topdown`]).
//!
//! Includes craftItemHelper specials first cut: water/soil retarget, berry-pie gate,
//! bowl fill anti-loops, forge flat-rock / clay-bowl bias, TIME actor wait, sticky
//! [`CraftAiRuntime`] for multi-tick fail+itemToCraft state.
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

use std::collections::{HashMap, HashSet};

use crate::craft_graph::ReverseCraftGraph;
use crate::short_craft_intent::ShortCraftLiveIntent;

// Haxe: searchBestTransitionTopDown / DoTransitionSearch (AI-CRAFT-TOPDOWN)
#[path = "craft_topdown.rs"]
mod craft_topdown;
// Re-export top-down surface for callers; allow unused within this module.
#[allow(unused_imports)]
pub use craft_topdown::{
    auto_decay_time_base_seconds, closest_craft_obj_filtered, craft_obj_passes_scan_filters,
    craft_trans_meta_map_from_content, do_transition_search_skip_reason, effective_ai_should_ignore,
    hardened_row_forces_hoe_soil_ignore, search_best_object_for_crafting_topdown,
    should_skip_craft_edge, should_skip_transition_top_down, time_transition_exceeds_ai_ignore,
    CraftObjectIndex, CraftScanFilters, CraftTopDownOpts, CraftTransMeta, TransSkipReason,
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

/// Default water sources when `ServerSettings.WaterSourceIds` not yet ported.
/// Wells from profession_scan `WELL_IDS` (Deep 663 / Shallow 662) — callers can
/// pass fuller lists via [`CraftLiveExpandOpts::water_source_ids`].
// Haxe: ServerSettings.WaterSourceIds (transition-derived)
pub const DEFAULT_WATER_SOURCE_IDS: [i32; 2] = [663, 662];

/// Haxe forge-adjacent quadDist threshold (~sqrt(10) tiles → Chebyshev ≤ 3).
pub const FORGE_NEAR_CHEBYSHEV: i32 = 3;
/// Min distance from forge when retargeting Flat Rock / Clay Bowl (Haxe minDistance 3).
pub const FORGE_BIAS_MIN_DIST: i32 = 3;
/// Search radius for forge-bias retargets (Haxe GetClosestObjectToTarget r=30).
pub const FORGE_BIAS_SEARCH_R: i32 = 30;
/// Soil retarget radius (Haxe GetClosestObjectToPositionByIds r=30).
pub const SOIL_RETARGET_R: i32 = 30;

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

    /// Set ObjectData-style max uses for multi-use / reverse-use filters.
    // Haxe: ObjectData.numUses
    pub fn with_max_uses(mut self, max_uses: i32) -> Self {
        self.max_uses = max_uses.max(0);
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
        match self.last_fail_sec.get(&product_id) {
            Some(&t) => {
                let passed = now_sec - t;
                (AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC - passed).max(0.0)
            }
            None => 0.0,
        }
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
// Haxe: AiBase.itemToCraft + failedCraftings + lastActorId + calledCraftItem
#[derive(Debug, Clone, Default)]
pub struct CraftAiRuntime {
    pub item: ItemToCraftState,
    pub failed: FailedCraftings,
    /// Haxe `lastActorId` — bowl fill anti-loop.
    pub last_actor_id: i32,
    /// Haxe `calledCraftItem` recursion guard for GetCraftAndDrop specials.
    pub called_craft_item: bool,
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
}

/// Live expand options for multi-step craft on the tick path.
// Haxe: home / hasOrBecomeProfession('SMITH') / TimeHelper ticks / WaterSourceIds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftLiveExpandOpts {
    pub home: Option<(i32, i32)>,
    pub is_or_can_smith: bool,
    pub now_sec: f64,
    /// Optional override; when empty, [`DEFAULT_WATER_SOURCE_IDS`] is used.
    /// Not stored as slice (Copy) — use `water_sources_override` via sticky path.
    pub use_default_water_sources: bool,
}

impl Default for CraftLiveExpandOpts {
    fn default() -> Self {
        Self {
            home: None,
            is_or_can_smith: true,
            now_sec: 0.0,
            use_default_water_sources: true,
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
}

// ── Inputs ──────────────────────────────────────────────────────────────────

/// Live context for one craftItem tick.
// Haxe: craftItemHelper(objId, maxDistance, onlyHome) + player/home
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftItemInput {
    pub product_id: i32,
    /// Haxe `maxDistance` override for `maxSearchRadius` when > 0.
    pub max_distance: i32,
    /// Haxe `onlyHome` → forces searchCurrentPosition=false.
    pub only_home: bool,
    pub player_x: i32,
    pub player_y: i32,
    pub held_id: i32,
    /// Home tile if known and close (Haxe home within 60).
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
    PickupActor {
        object_id: i32,
        x: i32,
        y: i32,
    },
    /// Empty-hand USE on pile to get actor.
    UsePileForActor {
        pile_id: i32,
        x: i32,
        y: i32,
    },
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
// Haxe: GetClosestObject* / secondObject
pub fn closest_craft_obj(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    exclude: Option<(i32, i32)>,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let max_r = max_r.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        if let Some((ex, ey)) = exclude {
            if o.x == ex && o.y == ey {
                continue;
            }
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

/// Second-closest of `parent_id` (Haxe sheep/cow deadly-actor special).
pub fn second_closest_craft_obj(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<CraftWorldObj> {
    let first = closest_craft_obj(objs, parent_id, from_x, from_y, max_r, None)?;
    closest_craft_obj(objs, parent_id, from_x, from_y, max_r, Some((first.x, first.y)))
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
    if ids.is_empty() {
        return None;
    }
    let max_r = max_r.max(0);
    let mut best: Option<(i32, CraftWorldObj)> = None;
    for o in objs {
        if !ids.contains(&o.parent_id) {
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
    // Prefer firing → charcoal → cold (lower index in FORGE_IDS is higher priority).
    let mut best: Option<(usize, i32, CraftWorldObj)> = None;
    for o in objs {
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

// ── craftItemHelper specials (pure retarget / gates) ─────────────────────────

/// Water-source retarget for Clay Bowl / Empty Water Pouch onto closest water.
// Haxe: craftItemHelper ~6905–6962 WaterSourceIds retarget
pub fn retarget_water_source(
    objs: &[CraftWorldObj],
    actor_id: i32,
    target_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    water_source_ids: &[i32],
) -> Option<CraftWorldObj> {
    if actor_id != CLAY_BOWL && actor_id != EMPTY_WATER_POUCH {
        return None;
    }
    if water_source_ids.is_empty() || !water_source_ids.contains(&target_id) {
        return None;
    }
    closest_craft_obj_by_ids(objs, water_source_ids, from_x, from_y, max_r)
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
    if actor_id != CLAY_BOWL || !SOIL_TARGET_IDS.contains(&target_id) {
        return None;
    }
    closest_craft_obj_by_ids(objs, &SOIL_TARGET_IDS, from_x, from_y, SOIL_RETARGET_R)
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
// Haxe: craftItemHelper ~6837–6881 (assumes non-full bowl — no multi-use here)
pub fn bowl_fill_pickup_blocked(
    held_id: i32,
    actor_id: i32,
    target_id: i32,
    last_actor_id: i32,
    objs: &[CraftWorldObj],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> bool {
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
    if held_id != BOWL_OF_DRY_BEANS
        && actor_id == BOWL_OF_DRY_BEANS
        && target_id != DRY_BEAN_PLANTS
    {
        if actor_id == last_actor_id {
            return true;
        }
        let plants =
            count_craft_objs_near(objs, &[DRY_BEAN_PLANTS], from_x, from_y, max_r);
        if plants < 1 {
            return true;
        }
    }
    false
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
    if target_id != FLAT_ROCK || ALLOWED_ON_FLAT_ROCK_NEAR_FORGE.contains(&actor_id) {
        return None;
    }
    let forge = closest_forge_craft(objs, player_x, player_y, FORGE_BIAS_SEARCH_R)?;
    if craft_chebyshev(player_x, player_y, forge.x, forge.y) > FORGE_NEAR_CHEBYSHEV {
        // Haxe uses dist to forge from player via CalculateQuadDistance; only acts when close.
        // When forge far, leave target alone.
        return None;
    }
    // Also only when the current target is near the forge.
    // (Haxe: if forge close to player, retarget flat rock away from forge.)
    match closest_craft_obj_min_anchor_dist(
        objs,
        FLAT_ROCK,
        player_x,
        player_y,
        FORGE_BIAS_SEARCH_R,
        forge.x,
        forge.y,
        FORGE_BIAS_MIN_DIST,
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
    if actor_id != CLAY_BOWL || actor_held {
        return None;
    }
    let forge = closest_forge_craft(objs, player_x, player_y, FORGE_BIAS_SEARCH_R)?;
    if craft_chebyshev(player_x, player_y, forge.x, forge.y) > FORGE_NEAR_CHEBYSHEV {
        return None;
    }
    // If current actor already far enough from forge, keep it.
    if craft_chebyshev(actor_x, actor_y, forge.x, forge.y) >= FORGE_BIAS_MIN_DIST {
        return None;
    }
    match closest_craft_obj_min_anchor_dist(
        objs,
        CLAY_BOWL,
        player_x,
        player_y,
        FORGE_BIAS_SEARCH_R,
        forge.x,
        forge.y,
        FORGE_BIAS_MIN_DIST,
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
    if called_craft_item || actor_id != FIRE_BOW_DRILL || target_id != LONG_STRAIGHT_SHAFT {
        return false;
    }
    let kindling = closest_craft_obj(objs, KINDLING, target_x, target_y, 10, None);
    let tinder = closest_craft_obj(objs, JUNIPER_TINDER, target_x, target_y, 10, None);
    kindling.is_none() && tinder.is_none()
}

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
        radius = (radius + AI_MAX_SEARCH_INCREMENT).max(AI_CRAFT_MIN_RADIUS);
        if radius > max_r {
            radius = max_r;
        }

        let have = craft_have_set(objs, held_id, player_x, player_y, home, radius);

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
            radius,
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
    let (ax, ay, actor_held, actor_from_pile, pile_id, actor_ok) =
        resolve_side(actor_id, objs, held_id, player_x, player_y, home, radius, pile_id_for, None);

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
    let out_actor = if actor_id < 0 { actor_id } else { actor_id.max(0) };
    let out_target = if target_id < 0 { target_id } else { target_id.max(0) };

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
// Haxe: AiBase.craftItemMax ~6604
pub fn craft_item_max_needed(count: i32, max: i32) -> bool {
    count < max
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
    )
}

/// Full craftItemHelper: meta + live path-reach / hostile / full-pile scan filters.
// Haxe: craftItemHelper + addObjectsForCrafting isObjectNotReachable
// Haxe: GetClosestObject* isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
pub fn craft_item_helper_ex(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    meta_by_edge: Option<&HashMap<(i32, i32), CraftTransMeta>>,
    scan: CraftScanFilters<'_>,
) -> CraftItemDecision {
    let product_id = inp.product_id;
    if product_id <= 0 {
        return CraftItemDecision::Failed;
    }

    // Failed crafting cooldown.
    if failed.is_cooling_down(product_id, inp.now_sec) {
        return CraftItemDecision::Cooldown;
    }

    // maxDistance / onlyHome overlay (Haxe craftItem saves/restores).
    let mut max_r = state.max_search_radius;
    if inp.max_distance > 0 {
        max_r = inp.max_distance;
    }
    if max_r < 1 {
        max_r = AI_MAX_SEARCH_RADIUS;
    }

    // Product change → reset sticky (Haxe re-init IntemToCraft fields).
    if state.product_id != product_id {
        state.reset_for_product(product_id);
    }
    state.max_search_radius = max_r;
    if inp.only_home {
        state.search_current_position = false;
    }

    // Sticky: held already is transActor → go USE target.
    // Haxe: if (transActor != null && held == transActor) { useTarget = …; return true; }
    if let (Some(aid), Some(tid), Some(tx), Some(ty)) = (
        state.trans_actor_id,
        state.trans_target_id,
        state.trans_target_x,
        state.trans_target_y,
    ) {
        if inp.held_id == aid && aid > 0 {
            // Target still expected: tile still has target id (or accept sticky).
            // Skip sticky USE when target tile is path-blocked (notReachable/hostile).
            let tile_blocked = !craft_obj_passes_scan_filters(
                &CraftWorldObj::simple(tid.max(0), tx, ty),
                &scan,
            );
            let still = !tile_blocked
                && (objs
                    .iter()
                    .any(|o| o.x == tx && o.y == ty && o.parent_id == tid)
                    || tid == 0);
            if still {
                state.trans_actor_id = None; // actor in hand
                return CraftItemDecision::UseOnTarget {
                    actor_id: aid,
                    target_id: tid,
                    target_x: tx,
                    target_y: ty,
                };
            }
        }
    }

    let home = match (inp.home_x, inp.home_y) {
        (Some(hx), Some(hy)) => {
            // Haxe: use home as start if IsCloseToObject(home, 60)
            if craft_chebyshev(inp.player_x, inp.player_y, hx, hy) <= 60 {
                Some((hx, hy))
            } else {
                None
            }
        }
        _ => None,
    };

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
    let craft_index = CraftObjectIndex::from_objs(objs, None);
    let exists_row = objs.iter().any(|o| {
        o.parent_id == HARDENED_ROW && craft_obj_passes_scan_filters(o, &scan)
    });
    let product_pile = pile_id_for.map(|f| f(product_id)).unwrap_or(-1);
    let mut topdown_opts = CraftTopDownOpts::default()
        .with_last(state.last_actor_id, state.last_target_id)
        .with_hardened_row(exists_row)
        .with_pile(product_pile)
        .with_index(&craft_index)
        .with_search_current(state.search_current_position)
        .with_scan(scan);
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

    // startLocation: home if close else transTarget.
    // Haxe: if (startLocation == null && transTarget != null)
    if state.start_location.is_none() {
        state.start_location = home.or(Some((target_x, target_y)));
    }

    // ── craftItemHelper specials (Haxe ~6740–7037) ──────────────────────────

    // Adobe + Firing Adobe Kiln → pottery residual.
    if actor_id == ADOBE && target_id == FIRING_ADOBE_KILN {
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
    if berry_pie_crust_blocked(
        actor_id,
        target_id,
        objs,
        inp.player_x,
        inp.player_y,
        max_r,
    ) {
        return CraftItemDecision::Failed;
    }

    // Fire bow + shaft → need kindling/tinder near shaft first (residual of GetCraftAndDrop).
    // Haxe: craftItemHelper ~6890–6902
    if fire_bow_needs_kindling(
        objs,
        actor_id,
        target_id,
        target_x,
        target_y,
        inp.called_craft_item,
    ) {
        return CraftItemDecision::SeekIngredient {
            ingredient_id: KINDLING,
            for_product: product_id,
        };
    }

    // Soil retarget for Clay Bowl.
    // Haxe: craftItemHelper ~6784–6793
    if let Some(soil) =
        retarget_soil_for_clay_bowl(objs, actor_id, target_id, inp.player_x, inp.player_y)
    {
        target_id = soil.parent_id;
        target_x = soil.x;
        target_y = soil.y;
    }

    // Flat Rock near forge: forbidden actor retarget or fail.
    // Haxe: craftItemHelper ~6795–6816
    match retarget_flat_rock_near_forge(objs, actor_id, target_id, inp.player_x, inp.player_y) {
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
    match retarget_clay_bowl_away_from_forge(
        objs,
        actor_id,
        actor_x,
        actor_y,
        actor_held,
        inp.player_x,
        inp.player_y,
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
    if bowl_fill_pickup_blocked(
        inp.held_id,
        actor_id,
        target_id,
        inp.last_actor_id,
        objs,
        inp.player_x,
        inp.player_y,
        max_r,
    ) {
        return CraftItemDecision::Failed;
    }

    // Water-source retarget (Clay Bowl / Empty Water Pouch → closest well/water).
    // Haxe: craftItemHelper ~6905–6962
    let water_ids = &DEFAULT_WATER_SOURCE_IDS;
    if let Some(w) = retarget_water_source(
        objs,
        actor_id,
        target_id,
        inp.player_x,
        inp.player_y,
        max_r,
        water_ids,
    ) {
        target_id = w.parent_id;
        target_x = w.x;
        target_y = w.y;
    }

    // TIME actor → WaitTime (non-animal assumed; animal TIME residual fail).
    // Haxe: craftItemHelper ~7019–7037
    if actor_id == -1 {
        state.clear_trans();
        return CraftItemDecision::WaitTime;
    }
    // PLAYER actor not supported.
    // Haxe: craftItemHelper ~7040–7047
    if actor_id == -2 {
        state.clear_trans();
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

    // Deadly actor + sheep/cow → prefer second closest target.
    // Knife 560, War Sword 3047, Mango Leaf 1878 vs Sheep 575/576 / Cow 1458.
    let deadly = [560, 3047, 1878];
    let second_close = [575, 576, 1458];
    if deadly.contains(&actor_id) && second_close.contains(&target_id) {
        let base = home.unwrap_or((inp.player_x, inp.player_y));
        if let Some(sec) =
            second_closest_craft_obj(objs, target_id, base.0, base.1, 30.max(max_r.min(30)))
        {
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
        if actor_id == 0 && inp.held_id != 0 {
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
    if actor_from_pile && pile_id > 0 {
        if inp.held_id != 0 {
            return CraftItemDecision::DropHeldThenPickup {
                actor_id,
                actor_x,
                actor_y,
            };
        }
        return CraftItemDecision::UsePileForActor {
            pile_id,
            x: actor_x,
            y: actor_y,
        };
    }

    if inp.held_id != 0 {
        return CraftItemDecision::DropHeldThenPickup {
            actor_id,
            actor_x,
            actor_y,
        };
    }

    CraftItemDecision::PickupActor {
        object_id: actor_id,
        x: actor_x,
        y: actor_y,
    }
}

/// Haxe `craftItem` wrapper: helper + forge smith gate already inside helper.
// Haxe: craftItem ~6611
pub fn craft_item(
    objs: &[CraftWorldObj],
    inp: &CraftItemInput,
    state: &mut ItemToCraftState,
    failed: &mut FailedCraftings,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftItemDecision {
    craft_item_helper(objs, inp, state, failed, graph, pile_id_for)
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
    let inp = CraftItemInput::from_runtime(
        product_id,
        player_x,
        player_y,
        held_id,
        opts,
        runtime,
    );
    let decision = craft_item_helper_ex(
        objs,
        &inp,
        &mut runtime.item,
        &mut runtime.failed,
        graph,
        pile_id_for,
        None,
        scan,
    );
    runtime.note_craft_done(decision);
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

        CraftItemDecision::SeekIngredient {
            ingredient_id, ..
        } => ShortCraftLiveIntent::SeekOrCraft {
            actor: ingredient_id,
            craft_if_needed: true,
        },

        CraftItemDecision::ShortCraftOnGround { object_id } => {
            ShortCraftLiveIntent::SeekGroundActor { target: object_id }
        }
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
    let mut inp = *inp;
    inp.product_id = object_id;
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
    }

    #[test]
    fn craft_item_max_needed_threshold() {
        assert!(craft_item_max_needed(0, 1));
        assert!(!craft_item_max_needed(1, 1));
        assert!(craft_item_max_needed(1, 2));
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
            CraftItemDecision::UseOnTarget { actor_id, target_id, .. } => {
                // if somehow empty held and actor 0 — shouldn't for 1+2
                assert!(actor_id == 1 || actor_id == 2);
                assert!(target_id == 1 || target_id == 2);
            }
            other => panic!("expected pickup or use, got {other:?}"),
        }
        assert_eq!(state.product_id, 5);
        assert!(state.trans_target_id.is_some() || matches!(d, CraftItemDecision::PickupActor { .. }));
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
                assert!(ingredient_id == 1 || ingredient_id == 2 || ingredient_id == 3 || ingredient_id == 4);
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
        let pair = search_best_object_for_crafting(
            3,
            &objs,
            0,
            0,
            0,
            None,
            60,
            &g,
            None,
        );
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_eq!(p.actor_id, 1);
        assert_eq!(p.target_id, 2);
    }

    #[test]
    fn water_retarget_prefers_closest_well() {
        // Distant pond 999 as initial water target; well 663 closer.
        let objs = vec![
            CraftWorldObj::simple(999, 40, 0),
            CraftWorldObj::simple(663, 5, 0),
        ];
        let ret = retarget_water_source(
            &objs,
            CLAY_BOWL,
            999,
            0,
            0,
            60,
            &[663, 662, 999],
        );
        assert_eq!(ret.map(|o| o.parent_id), Some(663));
        assert_eq!(ret.map(|o| o.x), Some(5));
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
            30
        ));
        assert!(bowl_fill_pickup_blocked(
            0,
            BOWL_OF_GOOSEBERRIES,
            99,
            BOWL_OF_GOOSEBERRIES, // last_actor match
            &objs,
            0,
            0,
            30
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
            30
        ));
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
        let with_k = vec![
            shaft,
            CraftWorldObj::simple(KINDLING, 6, 5),
        ];
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
        let d1 = craft_item_with_runtime(
            &[],
            5,
            0,
            0,
            0,
            &opts,
            &mut runtime,
            &g,
            None,
        );
        assert!(matches!(
            d1,
            CraftItemDecision::Failed | CraftItemDecision::SeekIngredient { .. }
        ));
        // Within 15s → Cooldown
        let opts2 = CraftLiveExpandOpts::default().with_now(110.0);
        let d2 = craft_item_with_runtime(
            &[],
            5,
            0,
            0,
            0,
            &opts2,
            &mut runtime,
            &g,
            None,
        );
        // SeekIngredient does not record fail; Failed does. Ensure fail path recorded.
        if matches!(d1, CraftItemDecision::Failed) {
            assert_eq!(d2, CraftItemDecision::Cooldown);
        } else {
            // If seek, force-record fail to prove sticky map
            runtime.failed.record_fail(5, 100.0);
            let d3 = craft_item_with_runtime(
                &[],
                5,
                0,
                0,
                0,
                &opts2,
                &mut runtime,
                &g,
                None,
            );
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
        let d = craft_item_with_runtime(
            &objs,
            9000,
            0,
            0,
            308,
            &opts,
            &mut runtime,
            &g,
            None,
        );
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
        );
        assert!(
            !matches!(d, CraftItemDecision::AlreadyHave { .. }),
            "blocked product must not AlreadyHave, got {d:?}"
        );
    }
}
