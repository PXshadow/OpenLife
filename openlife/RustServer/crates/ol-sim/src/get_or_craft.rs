//! Pure **GetOrCraftItem** / **GetItem** world I/O (AI-CRAFT-GRAPH-IO).
//!
//! Ports Haxe `AiBase.GetOrCraftItem` / `GetItem` search + staging.
//! Craft-miss path uses reverse-graph leaf seek; multi-step
//! `craftItem` / `craftItemHelper` lives in [`craft_item`] (**AI-CRAFT-MULTI**).
//!
//! Haxe anchors:
//! - `AiBase.GetOrCraftItem` ~6150–6219
//! - `AiBase.GetItem` ~6146
//! - `AiHelper.GetClosestObjectById` / `GetClosestObjectToTarget` / `GetClosestObjectToPosition`
//! - `ObjectData.getPileObjId` (caller supplies `pile_id`)
//! - Multi-step: `AiBase.craftItem` / `craftItemHelper` / `searchBestObjectForCrafting`

use std::collections::HashSet;

use crate::craft_graph::ReverseCraftGraph;
use crate::short_craft_intent::{
    apply_short_craft_live_intent, ShortCraftLiveApplyResult, ShortCraftLiveIntent,
};
use crate::SimState;
use ol_net::OutboundHub;

// ── Multi-step craftItem (AI-CRAFT-MULTI) ────────────────────────────────────
// Nested path so the module compiles without a separate lib.rs wire.
// Haxe: AiBase.craftItem / craftItemHelper / searchBestObjectForCrafting
#[path = "craft_item.rs"]
pub mod craft_item;

pub use craft_item::{
    closest_craft_obj, craft_chebyshev, craft_have_set, craft_have_set_ex_filtered, craft_item,
    craft_item_decision_to_live_intent, craft_item_helper, craft_item_helper_ex,
    craft_item_max_needed, craft_item_with_runtime, craft_item_with_runtime_scan,
    craft_world_from_get_or_craft, craft_world_objs_from_ids, first_missing_ingredient,
    reanchor_craft_actor_near_target_filtered, resolve_craft_item_live,
    search_best_object_for_crafting, search_best_object_for_crafting_ex,
    search_best_object_for_crafting_topdown, second_closest_craft_obj,
    closest_craft_obj_dual_center_filtered, closest_craft_obj_filtered,
    should_skip_transition_top_down, CraftAiRuntime, CraftItemDecision, CraftItemInput,
    CraftLiveExpandOpts, CraftObjectIndex, CraftScanFilters, CraftTopDownOpts, CraftTransMeta,
    CraftTransPair, CraftWorldObj, FailedCraftings, ItemToCraftState, TransSkipReason,
    AI_CRAFT_MIN_RADIUS, AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN, AI_MAX_SEARCH_INCREMENT,
    AI_MAX_SEARCH_RADIUS, AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC, DEFAULT_WATER_SOURCE_IDS,
    FORGE_IDS, HARDENED_ROW,
};

// ── Constants (Haxe literals) ───────────────────────────────────────────────

/// Close pile search radius when object has a pile form.
// Haxe: searchDistance = hasPile ? 5 : maxSearchDistance
pub const GET_OR_CRAFT_PILE_CLOSE_R: i32 = 5;

/// Target-relative search radius (stones near craft target, not home piles).
// Haxe: GetClosestObjectToTarget(..., 10, minDistance)
pub const GET_OR_CRAFT_TARGET_R: i32 = 10;

/// Default max search (Haxe `maxSearchDistance = 40`).
pub const GET_OR_CRAFT_DEFAULT_MAX_SEARCH: i32 = 40;

// ── World object snapshot ───────────────────────────────────────────────────

/// One ground object for GetOrCraft search (from scan / map snapshot).
// Haxe: ObjectHelper parentId / tx / ty / objectData.numSlots
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetOrCraftWorldObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    /// Haxe `objectData.numSlots` (containers force empty-hand before pickup).
    pub num_slots: i32,
}

impl GetOrCraftWorldObj {
    pub fn simple(parent_id: i32, x: i32, y: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            num_slots: 0,
        }
    }

    pub fn with_slots(mut self, num_slots: i32) -> Self {
        self.num_slots = num_slots.max(0);
        self
    }

    /// Convert to multi-step craft world obj.
    pub fn to_craft_world(self) -> CraftWorldObj {
        CraftWorldObj::simple(self.parent_id, self.x, self.y).with_slots(self.num_slots)
    }
}

// ── Input ───────────────────────────────────────────────────────────────────

/// Inputs for pure [`get_or_craft_item`].
// Haxe: GetOrCraftItem(objId, craft, minDistance, maxSearchDistance, target)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetOrCraftInput {
    pub obj_id: i32,
    /// When false → Haxe `GetItem` / `craftActorIfNeeded=false` (no craft fallback).
    pub craft: bool,
    /// Min Chebyshev from player (or search base); 0 = any distance in range.
    pub min_distance: i32,
    pub max_search_distance: i32,
    pub player_x: i32,
    pub player_y: i32,
    /// Held parent id; `0` = empty hands.
    pub held_id: i32,
    /// Optional craft-target anchor for target-relative search.
    pub target_x: Option<i32>,
    pub target_y: Option<i32>,
    /// `getPileObjId()` result; `<= 0` = no pile form.
    pub pile_id: i32,
    /// Haxe early-out when `myPlayer.isMoving()`.
    pub is_moving: bool,
}

impl GetOrCraftInput {
    /// Minimal GetItem-style input (no craft, no target, no pile).
    pub fn get_item(obj_id: i32, player_x: i32, player_y: i32) -> Self {
        Self {
            obj_id,
            craft: false,
            min_distance: 0,
            max_search_distance: GET_OR_CRAFT_DEFAULT_MAX_SEARCH,
            player_x,
            player_y,
            held_id: 0,
            target_x: None,
            target_y: None,
            pile_id: -1,
            is_moving: false,
        }
    }

    /// GetOrCraft with craft=true, optional pile and target.
    pub fn get_or_craft(obj_id: i32, player_x: i32, player_y: i32) -> Self {
        Self {
            craft: true,
            ..Self::get_item(obj_id, player_x, player_y)
        }
    }

    pub fn with_pile(mut self, pile_id: i32) -> Self {
        self.pile_id = pile_id;
        self
    }

    pub fn with_target(mut self, tx: i32, ty: i32) -> Self {
        self.target_x = Some(tx);
        self.target_y = Some(ty);
        self
    }

    pub fn with_held(mut self, held_id: i32) -> Self {
        self.held_id = held_id;
        self
    }

    pub fn with_min_distance(mut self, min_distance: i32) -> Self {
        self.min_distance = min_distance;
        self
    }

    pub fn with_max_search(mut self, max_search_distance: i32) -> Self {
        self.max_search_distance = max_search_distance;
        self
    }

    pub fn with_craft(mut self, craft: bool) -> Self {
        self.craft = craft;
        self
    }
}

// ── Result ──────────────────────────────────────────────────────────────────

/// Staging decision from pure GetOrCraft (before live USE/DROP apply).
// Haxe: dropIsAUse / dropTarget / useTarget / craftItem return
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetOrCraftResult {
    /// Player still moving — Haxe returns true without re-staging.
    BusyMoving,
    /// Loose object → Haxe `dropTarget = obj` (pickup via DROP on tile).
    PickupLoose {
        x: i32,
        y: i32,
        object_id: i32,
    },
    /// Pile only → Haxe `dropIsAUse` / `useTarget = pile` (USE empty-handed).
    UseOnPile {
        x: i32,
        y: i32,
        pile_id: i32,
    },
    /// Pile or container while holding — must drop held first then re-enter.
    // Haxe: (usePile || numSlots>0) && dropHeldObject()
    NeedEmptyHand {
        x: i32,
        y: i32,
        object_id: i32,
        is_pile: bool,
    },
    /// craft=true and missing → reverse-graph leaf ingredient to seek.
    SeekIngredient {
        ingredient_id: i32,
        for_product: i32,
    },
    /// craft=true and missing (no leaf or no graph) → multi-step craftItem staging.
    // Haxe: craftItem(objId) — expand via AI-CRAFT-MULTI `craft_item_helper`
    CraftItem {
        object_id: i32,
    },
    /// Not found and craft=false, or obj_id invalid.
    None,
}

impl GetOrCraftResult {
    /// True when this tick already has a concrete tile action (or empty-hand drop).
    pub fn is_action(self) -> bool {
        matches!(
            self,
            Self::PickupLoose { .. } | Self::UseOnPile { .. } | Self::NeedEmptyHand { .. }
        )
    }

    /// True when craft expansion / seek residual.
    pub fn is_craft_staging(self) -> bool {
        matches!(self, Self::SeekIngredient { .. } | Self::CraftItem { .. })
    }
}

// ── Spatial helpers ─────────────────────────────────────────────────────────

#[inline]
pub fn get_or_craft_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Haxe `ObjectData.getPileObjId` — pure transition lookup form.
///
/// `get_trans(actor, target) → Option<(new_actor, new_target)>` mirrors
/// `TransitionImporter.GetTransition`. Returns `new_target` of the self+self
/// pile transition when the empty-hand undo yields the original id; else `-1`.
// Haxe: ObjectData.getPileObjId ~1531–1538
pub fn get_pile_obj_id(
    obj_id: i32,
    get_trans: &dyn Fn(i32, i32) -> Option<(i32, i32)>,
) -> i32 {
    if obj_id <= 0 {
        return -1;
    }
    // GetTransition(this.id, this.id) → pile as newTarget
    let Some((_new_actor, new_target)) = get_trans(obj_id, obj_id) else {
        return -1;
    };
    if new_target <= 0 {
        return -1;
    }
    // GetTransition(0, pile) undo must yield original as newActor
    let Some((undo_actor, _undo_tgt)) = get_trans(0, new_target) else {
        return -1;
    };
    if undo_actor != obj_id {
        return -1;
    }
    new_target
}

/// Pile id from a flat `(actor,target) → (new_actor,new_target)` map.
// Haxe: ObjectData.getPileObjId via TransitionImporter tables
#[inline]
pub fn get_pile_obj_id_from_map(
    obj_id: i32,
    transitions: &std::collections::HashMap<(i32, i32), (i32, i32)>,
) -> i32 {
    get_pile_obj_id(obj_id, &|a, t| transitions.get(&(a, t)).copied())
}

/// Live ContentDb pile lookup (npc / profession GetOrCraft wire).
// Haxe: ObjectData.getObjectData(objId).getPileObjId()
#[inline]
pub fn pile_obj_id_from_content(content: &ol_content::ContentDb, obj_id: i32) -> i32 {
    get_pile_obj_id(obj_id, &|a, t| {
        content
            .find_transition(a, t)
            .map(|tr| (tr.new_actor_id, tr.new_target_id))
    })
}

/// Closest matching `parent_id` within `[min_r, max_r]` of `(from_x, from_y)`.
///
/// Tie-break: lower y, then lower x (stable with profession_scan).
// Haxe: GetClosestObjectToPositionHelper with minDistance / searchDistance
pub fn closest_obj_by_id(
    objs: &[GetOrCraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_r: i32,
) -> Option<GetOrCraftWorldObj> {
    closest_obj_by_id_filtered(objs, parent_id, from_x, from_y, max_r, min_r, None)
}

/// Closest matching parent with optional path-blocked tile skip.
// Haxe: GetClosestObject* + isObjectNotReachable / isObjectWithHostilePath
pub fn closest_obj_by_id_filtered(
    objs: &[GetOrCraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_r: i32,
    blocked: Option<&HashSet<(i32, i32)>>,
) -> Option<GetOrCraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    let max_r = max_r.max(0);
    let min_r = min_r.max(0);
    let mut best: Option<(i32, GetOrCraftWorldObj)> = None;
    for o in objs {
        if o.parent_id != parent_id {
            continue;
        }
        if let Some(b) = blocked {
            if b.contains(&(o.x, o.y)) {
                continue;
            }
        }
        let d = get_or_craft_chebyshev(from_x, from_y, o.x, o.y);
        if d > max_r || d < min_r {
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

// ── Core GetOrCraft ─────────────────────────────────────────────────────────

/// Pure GetOrCraftItem search + staging (no world mutation).
///
/// `graph` / `have` only used when object missing and `inp.craft` is true.
// Haxe: AiBase.GetOrCraftItem ~6150–6219
pub fn get_or_craft_item(
    objs: &[GetOrCraftWorldObj],
    inp: &GetOrCraftInput,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
) -> GetOrCraftResult {
    get_or_craft_item_ex(objs, inp, graph, have, None)
}

/// GetOrCraftItem with optional path-blocked tiles (notReachable / hostile / blockedByAI).
// Haxe: GetClosestObject* isObjectNotReachable / isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
pub fn get_or_craft_item_ex(
    objs: &[GetOrCraftWorldObj],
    inp: &GetOrCraftInput,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
    blocked: Option<&HashSet<(i32, i32)>>,
) -> GetOrCraftResult {
    if inp.is_moving {
        return GetOrCraftResult::BusyMoving;
    }
    if inp.obj_id <= 0 {
        return GetOrCraftResult::None;
    }

    let has_pile = inp.pile_id > 0;
    let max_search = inp.max_search_distance.max(0);
    // Haxe: searchDistance = hasPile ? 5 : maxSearchDistance
    let close_r = if has_pile {
        GET_OR_CRAFT_PILE_CLOSE_R.min(max_search)
    } else {
        max_search
    };
    let min_d = inp.min_distance.max(0);

    let mut obj: Option<GetOrCraftWorldObj> = None;
    let mut pile: Option<GetOrCraftWorldObj> = None;

    // 1) first search close to target (e.g. stones near craft site not home)
    // Haxe: if (target != null) GetClosestObjectToTarget(..., 10, minDistance)
    if let (Some(tx), Some(ty)) = (inp.target_x, inp.target_y) {
        obj = closest_obj_by_id_filtered(
            objs,
            inp.obj_id,
            tx,
            ty,
            GET_OR_CRAFT_TARGET_R,
            min_d,
            blocked,
        );
        if has_pile {
            pile = closest_obj_by_id_filtered(
                objs,
                inp.pile_id,
                tx,
                ty,
                GET_OR_CRAFT_TARGET_R,
                min_d,
                blocked,
            );
        }
    }

    // 2) search close (from player)
    if obj.is_none() {
        obj = closest_obj_by_id_filtered(
            objs,
            inp.obj_id,
            inp.player_x,
            inp.player_y,
            close_r,
            min_d,
            blocked,
        );
    }
    if pile.is_none() && has_pile {
        pile = closest_obj_by_id_filtered(
            objs,
            inp.pile_id,
            inp.player_x,
            inp.player_y,
            close_r,
            min_d,
            blocked,
        );
    }

    // Prefer loose over pile when both found; pile only if no loose.
    // Haxe: usePile = pile != null && obj == null; if (usePile) obj = pile;
    let mut use_pile = pile.is_some() && obj.is_none();
    if use_pile {
        obj = pile;
    }

    // 3) search farther when still missing
    if obj.is_none() && close_r < max_search {
        obj = closest_obj_by_id_filtered(
            objs,
            inp.obj_id,
            inp.player_x,
            inp.player_y,
            max_search,
            min_d,
            blocked,
        );
        if obj.is_none() && has_pile {
            obj = closest_obj_by_id_filtered(
                objs,
                inp.pile_id,
                inp.player_x,
                inp.player_y,
                max_search,
                min_d,
                blocked,
            );
            use_pile = obj.is_some();
        }
    }

    let Some(found) = obj else {
        // Haxe: if (obj == null && craft == false) return false;
        //       if (obj == null) return craftItem(objId);
        if !inp.craft {
            return GetOrCraftResult::None;
        }
        return craft_item_fallback(inp.obj_id, graph, have);
    };

    // Empty-hand gate for pile or container (numSlots > 0).
    // Haxe: if ((usePile || obj.objectData.numSlots > 0) && dropHeldObject()) return true;
    let needs_empty = use_pile || found.num_slots > 0;
    if needs_empty && inp.held_id != 0 {
        return GetOrCraftResult::NeedEmptyHand {
            x: found.x,
            y: found.y,
            object_id: found.parent_id,
            is_pile: use_pile,
        };
    }

    if use_pile {
        GetOrCraftResult::UseOnPile {
            x: found.x,
            y: found.y,
            pile_id: found.parent_id,
        }
    } else {
        GetOrCraftResult::PickupLoose {
            x: found.x,
            y: found.y,
            object_id: found.parent_id,
        }
    }
}

/// Haxe `GetItem` — GetOrCraft with craft=false.
// Haxe: AiBase.GetItem ~6146
pub fn get_item(
    objs: &[GetOrCraftWorldObj],
    obj_id: i32,
    player_x: i32,
    player_y: i32,
    max_search: i32,
    target: Option<(i32, i32)>,
    pile_id: i32,
    held_id: i32,
) -> GetOrCraftResult {
    let mut inp = GetOrCraftInput::get_item(obj_id, player_x, player_y)
        .with_max_search(max_search)
        .with_pile(pile_id)
        .with_held(held_id);
    if let Some((tx, ty)) = target {
        inp = inp.with_target(tx, ty);
    }
    get_or_craft_item(objs, &inp, None, None)
}

/// Craft miss: prefer reverse-graph leaf seek, else `CraftItem` staging.
// Haxe: craftItem(objId) — multi-step via craft_item_helper when resolved live
fn craft_item_fallback(
    obj_id: i32,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
) -> GetOrCraftResult {
    if let Some(g) = graph {
        let empty;
        let have_set = match have {
            Some(h) => h,
            None => {
                empty = HashSet::new();
                &empty
            }
        };
        if let Some(ing) = g.seek_ingredient_for(obj_id, have_set) {
            if ing != obj_id {
                return GetOrCraftResult::SeekIngredient {
                    ingredient_id: ing,
                    for_product: obj_id,
                };
            }
        }
    }
    GetOrCraftResult::CraftItem { object_id: obj_id }
}

// ── Live intent mapping ─────────────────────────────────────────────────────

/// Map pure GetOrCraft staging → [`ShortCraftLiveIntent`].
///
/// `empty_drop` is used when [`GetOrCraftResult::NeedEmptyHand`] (drop held first).
/// Loose pickup uses DropAt on the object tile (Haxe `dropTarget`); piles use
/// empty-handed UseAt (Haxe `dropIsAUse` / `useTarget`).
// Haxe: GetOrCraftItem staging → dropTarget / useTarget / craftItem
pub fn get_or_craft_result_to_live_intent(
    result: GetOrCraftResult,
    empty_drop: Option<(i32, i32)>,
) -> ShortCraftLiveIntent {
    match result {
        // Haxe: GetOrCraftItem isMoving return true → hold tick (PREFER-SHORT-WAIT)
        GetOrCraftResult::BusyMoving => ShortCraftLiveIntent::Wait,
        GetOrCraftResult::None => ShortCraftLiveIntent::None,
        // Haxe dropTarget = obj → DROP on object tile (pickup / swap)
        GetOrCraftResult::PickupLoose { x, y, .. } => ShortCraftLiveIntent::DropAt { x, y },
        // Haxe useTarget = pile, dropIsAUse → USE empty hands on pile
        GetOrCraftResult::UseOnPile { x, y, pile_id } => ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id: pile_id,
            actor_id: 0,
        },
        GetOrCraftResult::NeedEmptyHand { .. } => match empty_drop {
            Some((x, y)) => ShortCraftLiveIntent::DropAt { x, y },
            None => ShortCraftLiveIntent::None,
        },
        GetOrCraftResult::SeekIngredient {
            ingredient_id, ..
        } => ShortCraftLiveIntent::SeekOrCraft {
            actor: ingredient_id,
            craft_if_needed: true,
        },
        GetOrCraftResult::CraftItem { object_id } => {
            ShortCraftLiveIntent::CraftItem { object_id }
        }
    }
}

/// Alias for [`get_or_craft_result_to_live_intent`].
#[inline]
pub fn get_or_craft_to_live_intent(
    result: GetOrCraftResult,
    empty_drop: Option<(i32, i32)>,
) -> ShortCraftLiveIntent {
    get_or_craft_result_to_live_intent(result, empty_drop)
}

/// Convert GetOrCraft world snapshot → craft multi-step objs.
pub fn to_craft_world_objs(objs: &[GetOrCraftWorldObj]) -> Vec<CraftWorldObj> {
    objs.iter().map(|o| o.to_craft_world()).collect()
}

/// Expand CraftItem staging with multi-step craftItemHelper when a graph is present.
///
/// Ephemeral sticky state (no multi-tick cooldown). Prefer
/// [`expand_craft_item_live_sticky`] / [`craft_item_with_runtime`] for AI/NPC
/// with persistent [`CraftAiRuntime`].
// Haxe: craftItem → craftItemHelper → useTarget / dropTarget
pub fn expand_craft_item_live(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    home: Option<(i32, i32)>,
    is_or_can_smith: bool,
    now_sec: f64,
) -> ShortCraftLiveIntent {
    let opts = CraftLiveExpandOpts {
        home,
        is_or_can_smith,
        now_sec,
        use_default_water_sources: true,
    };
    expand_craft_item_live_opts(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        &opts,
        None,
    )
}

/// Expand CraftItem with full [`CraftLiveExpandOpts`] and optional sticky runtime.
// Haxe: craftItem + failedCraftings / itemToCraft / home / SMITH / TimeHelper
pub fn expand_craft_item_live_opts(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
) -> ShortCraftLiveIntent {
    expand_craft_item_live_opts_scan(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        opts,
        runtime,
        CraftScanFilters::default(),
    )
}

/// Expand CraftItem with path-reach / hostile scan filters on world objects.
// Haxe: craftItem + isObjectNotReachable / isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
pub fn expand_craft_item_live_opts_scan(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
    scan: CraftScanFilters<'_>,
) -> ShortCraftLiveIntent {
    let craft_objs = to_craft_world_objs(objs);
    let pile = |id: i32| pile_id_for(id);
    let decision = if let Some(rt) = runtime {
        craft_item_with_runtime_scan(
            &craft_objs,
            object_id,
            player_x,
            player_y,
            held_id,
            opts,
            rt,
            graph,
            Some(&pile),
            scan,
        )
    } else {
        let mut state = ItemToCraftState::new(object_id);
        let mut failed = FailedCraftings::new();
        let mut inp = CraftItemInput::basic(object_id, player_x, player_y)
            .with_held(held_id)
            .with_now(opts.now_sec);
        inp.is_or_can_smith = opts.is_or_can_smith;
        if let Some((hx, hy)) = opts.home {
            inp = inp.with_home(hx, hy);
        }
        craft_item_helper_ex(
            &craft_objs,
            &inp,
            &mut state,
            &mut failed,
            graph,
            Some(&pile),
            None,
            scan,
        )
    };
    craft_item_decision_to_live_intent(decision, empty_drop)
}

/// Sticky multi-tick expand (alias for opts + runtime).
#[inline]
pub fn expand_craft_item_live_sticky(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    runtime: &mut CraftAiRuntime,
) -> ShortCraftLiveIntent {
    expand_craft_item_live_opts(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        opts,
        Some(runtime),
    )
}

/// Sticky multi-tick expand with path-reach scan filters.
// Haxe: craftItem sticky + isObjectNotReachable (AI-CRAFT-LIVE-RESID)
#[inline]
pub fn expand_craft_item_live_sticky_scan(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    runtime: &mut CraftAiRuntime,
    scan: CraftScanFilters<'_>,
) -> ShortCraftLiveIntent {
    expand_craft_item_live_opts_scan(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        opts,
        Some(runtime),
        scan,
    )
}

// Depth-1 multi-step CraftItem resolve + shallow GetOrCraft (AI-CRAFT-MULTI).
include!("get_or_craft_resolve.inc.rs");

// world_objs_from_ids + apply_resolved_seek_or_craft + tests
include!("get_or_craft_tail.inc.rs");
