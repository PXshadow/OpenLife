//! Live AI path-block maps (**PATH-REACH** / **BLOCKED-BY-AI**).
//!
//! Haxe `AiBase` keeps three timed maps:
//! - `notReachableObjects` — per-AI tiles that failed path/use (default **90s**)
//! - `objectsWithHostilePath` — animal-blocked paths (default **20s**)
//! - static `blockedByAI` — other AIs' current targets (default **5s** / rebuild)
//!
//! Sticky [`AiStickyBlockTargets`] on Player → live rebuild of global map each tick.
//! Pure profession scans use `ProfessionPathFilters` (coordinate set) in
//! `profession_scan`. Live apply builds that snapshot via [`AiPathReachMaps::blocked_coords`]:
//! `ProfessionPathFilters::with_blocked(maps.blocked_coords(Some(&blocked_by_ai)))`.
//!
//! // Haxe: AiBase.notReachableObjects / objectsWithHostilePath / blockedByAI
//! // Haxe: cleanupBlockedObjects ~6258; addNotReachable ~9265; addHostilePath ~9234
//! // Haxe: isObjectNotReachable ~9273; isObjectWithHostilePath ~9245; AddObjBlockedByAi ~9253
//! // Haxe: CalculateBlockedByAi / AddToBlockedByAi / AddTargetBlockedByAi ~222–302

use std::collections::{HashMap, HashSet};

/// Haxe `addNotReachable` / `addNotReachableObject` default time.
pub const NOT_REACHABLE_DEFAULT_SECS: f32 = 90.0;
/// Haxe `addHostilePath` default time.
pub const HOSTILE_PATH_DEFAULT_SECS: f32 = 20.0;
/// Haxe `AddObjBlockedByAi` default time.
pub const BLOCKED_BY_AI_DEFAULT_SECS: f32 = 5.0;
/// Haxe food-target not-reachable override (`addNotReachableObject(food, 30)`).
pub const NOT_REACHABLE_FOOD_SECS: f32 = 30.0;
/// Haxe human/AI `blockTargetForAi` age gate (`timePassedInSeconds > 20` skip).
// Haxe: AiBase.CalculateBlockedByAi ~231; AddToBlockedByAi ~250–252
pub const BLOCK_TARGET_MAX_AGE_SECS: f32 = 20.0;
/// Haxe `AddToBlockedByAi`: skip AI younger than this.
// Haxe: AiBase.AddToBlockedByAi ~244
pub const BLOCK_BY_AI_MIN_AGE: f32 = 3.0;
/// Smithing Hammer 441 — always may set `blockTargetForAi` after USE.
// Haxe: TransitionHelper.use isHoldingSmithingHammer 441
pub const SMITHING_HAMMER_BLOCK_ID: i32 = 441;

/// Haxe `AiBase.DontBlockByAi` — fires / ovens / kilns / forges (shared multi-AI).
// Haxe: AiBase.DontBlockByAi L283
/// Fire 82, Large Fast Fire 83, Hot Coals 85, Large Slow Fire 346, Flash Fire 3029,
/// Adobe Oven 237, Hot Adobe Oven 250, Adobe Kiln 238, Firing Adobe Kiln 282,
/// Forge 303, Firing Forge 304, Firing Newcomen Hammer 2238.
pub const DONT_BLOCK_BY_AI: &[i32] = &[
    82, 83, 85, 346, 3029, 237, 250, 238, 282, 303, 304, 2238,
];

/// Live per-AI timed path maps (Haxe `AiBase.notReachableObjects` + `objectsWithHostilePath`).
// Haxe: AiBase L85–86
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiPathReachMaps {
    /// Absolute world tile → remaining block seconds.
    // Haxe: notReachableObjects Map index → time
    pub not_reachable: HashMap<(i32, i32), f32>,
    /// Absolute world tile → remaining hostile-path block seconds.
    // Haxe: objectsWithHostilePath Map index → time
    pub hostile_path: HashMap<(i32, i32), f32>,
}

impl AiPathReachMaps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Haxe `addNotReachable(tx, ty, time=90)`.
    // Haxe: AiBase.addNotReachable ~9265
    pub fn add_not_reachable(&mut self, x: i32, y: i32, time: f32) {
        let t = if time > 0.0 {
            time
        } else {
            NOT_REACHABLE_DEFAULT_SECS
        };
        self.not_reachable.insert((x, y), t);
    }

    /// Haxe `addNotReachableObject(obj, time=90)`.
    // Haxe: AiBase.addNotReachableObject ~9260
    #[inline]
    pub fn add_not_reachable_object(&mut self, x: i32, y: i32, time: f32) {
        self.add_not_reachable(x, y, time);
    }

    /// Haxe `addHostilePath(tx, ty, time=20)`.
    // Haxe: AiBase.addHostilePath ~9234
    pub fn add_hostile_path(&mut self, x: i32, y: i32, time: f32) {
        let t = if time > 0.0 {
            time
        } else {
            HOSTILE_PATH_DEFAULT_SECS
        };
        self.hostile_path.insert((x, y), t);
    }

    /// Haxe `addObjectWithHostilePath(obj)`.
    // Haxe: AiBase.addObjectWithHostilePath ~9229
    #[inline]
    pub fn add_object_with_hostile_path(&mut self, x: i32, y: i32) {
        self.add_hostile_path(x, y, HOSTILE_PATH_DEFAULT_SECS);
    }

    /// Haxe `isObjectNotReachable` personal map only (caller ORs `blockedByAI`).
    // Haxe: AiBase.isObjectNotReachable ~9273 (notReachableObjects.exists)
    #[inline]
    pub fn is_personal_not_reachable(&self, x: i32, y: i32) -> bool {
        self.not_reachable.contains_key(&(x, y))
    }

    /// Haxe `isObjectWithHostilePath`.
    // Haxe: AiBase.isObjectWithHostilePath ~9245
    #[inline]
    pub fn is_object_with_hostile_path(&self, x: i32, y: i32) -> bool {
        self.hostile_path.contains_key(&(x, y))
    }

    /// Full Haxe `isObjectNotReachable` including optional global `blockedByAI`.
    // Haxe: notReachableObjects || blockedByAI
    pub fn is_object_not_reachable(
        &self,
        x: i32,
        y: i32,
        blocked_by_ai: Option<&HashMap<(i32, i32), f32>>,
    ) -> bool {
        if self.is_personal_not_reachable(x, y) {
            return true;
        }
        blocked_by_ai
            .map(|m| m.contains_key(&(x, y)))
            .unwrap_or(false)
    }

    /// Profession / closest-pick skip: personal not-reachable, hostile path, or blockedByAI.
    pub fn blocks_target(
        &self,
        x: i32,
        y: i32,
        blocked_by_ai: Option<&HashMap<(i32, i32), f32>>,
    ) -> bool {
        self.is_object_not_reachable(x, y, blocked_by_ai)
            || self.is_object_with_hostile_path(x, y)
    }

    /// Haxe `cleanupBlockedObjectsHelper` — decay both maps by `timePassed`.
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264
    pub fn cleanup(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        decay_timed_map(&mut self.not_reachable, dt);
        decay_timed_map(&mut self.hostile_path, dt);
    }

    /// Collect all blocked coords (not reachable + hostile + optional blockedByAI).
    ///
    /// Feed into `ProfessionPathFilters::with_blocked` for pure scan filters.
    pub fn blocked_coords(
        &self,
        blocked_by_ai: Option<&HashMap<(i32, i32), f32>>,
    ) -> HashSet<(i32, i32)> {
        let mut s = HashSet::new();
        for &xy in self.not_reachable.keys() {
            s.insert(xy);
        }
        for &xy in self.hostile_path.keys() {
            s.insert(xy);
        }
        if let Some(b) = blocked_by_ai {
            for &xy in b.keys() {
                s.insert(xy);
            }
        }
        s
    }

    /// Keys of personal not-reachable map (for live food search skip).
    #[inline]
    pub fn not_reachable_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.not_reachable.keys().copied()
    }

    /// Keys of hostile-path map (for live food danger scan).
    #[inline]
    pub fn hostile_path_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.hostile_path.keys().copied()
    }

    /// True when either timed map has entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.not_reachable.is_empty() && self.hostile_path.is_empty()
    }
}

/// Haxe `AddObjBlockedByAi` — static map of tiles claimed by other AIs.
// Haxe: AiBase.AddObjBlockedByAi ~9253
pub fn add_blocked_by_ai(map: &mut HashMap<(i32, i32), f32>, x: i32, y: i32, time: f32) {
    let t = if time > 0.0 {
        time
    } else {
        BLOCKED_BY_AI_DEFAULT_SECS
    };
    map.insert((x, y), t);
}

/// Decay global `blockedByAI` map.
pub fn cleanup_blocked_by_ai(map: &mut HashMap<(i32, i32), f32>, dt: f32) {
    if dt <= 0.0 {
        return;
    }
    decay_timed_map(map, dt);
}

/// Build blocked coordinate set from live player maps + global blockedByAI.
// Haxe: isObjectNotReachable || isObjectWithHostilePath
#[inline]
pub fn blocked_coords_from_live(
    maps: &AiPathReachMaps,
    blocked_by_ai: &HashMap<(i32, i32), f32>,
) -> HashSet<(i32, i32)> {
    maps.blocked_coords(Some(blocked_by_ai))
}

/// Mark target not-reachable after failed USE (Haxe addNotReachableObject on use fail).
// Haxe: AiBase.addNotReachableObject ~9260 / failed use ~8916
#[inline]
pub fn mark_not_reachable_on_player(maps: &mut AiPathReachMaps, x: i32, y: i32, time: f32) {
    maps.add_not_reachable(x, y, time);
}

// ── Fail-mark helpers (Haxe USE / food / Goto fail paths) ───────────────────

/// Haxe USE fail: `age > 3` → notReachable (90s); else hostile path (20s).
// Haxe: AiBase isUsingObject fail ~9133–9134
pub fn mark_use_path_fail(maps: &mut AiPathReachMaps, x: i32, y: i32, age: f32) {
    if age > BLOCK_BY_AI_MIN_AGE {
        maps.add_not_reachable(x, y, NOT_REACHABLE_DEFAULT_SECS);
    } else {
        maps.add_object_with_hostile_path(x, y);
    }
}

/// Haxe food pickup/USE fail → `addNotReachableObject(food, 30)`.
// Haxe: AiBase pickup food fail ~8698
pub fn mark_food_path_fail(maps: &mut AiPathReachMaps, x: i32, y: i32) {
    maps.add_not_reachable(x, y, NOT_REACHABLE_FOOD_SECS);
}

/// Effects after food REMV/USE/DROP fails (Haxe `isPickingupFood` ~8694–8700).
///
/// Unlike path/goto fail: **no** `didNotReachFood++` — only 30s notReachable + clear sticky.
// Haxe: AiBase.isPickingupFood done==false → addNotReachableObject(food, 30); foodTarget=null
// AI-FOOD-FAIL-MARK / food_use_fail_30s
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodActionFailEffects {
    /// Haxe `addNotReachableObject(food, 30)`.
    pub not_reachable_secs: f32,
    /// Clear sticky `foodTarget`.
    pub clear_food_target: bool,
}

/// Pure effects for food USE/DROP/REMV fail.
// Haxe: AiBase.isPickingupFood ~8694–8700
#[inline]
pub fn food_action_fail_effects() -> FoodActionFailEffects {
    FoodActionFailEffects {
        not_reachable_secs: NOT_REACHABLE_FOOD_SECS,
        clear_food_target: true,
    }
}

/// True when failed action coords match sticky food claim (`foodTarget` tile).
// Haxe: isPickingupFood fail on foodTarget.tx / foodTarget.ty
#[inline]
pub fn is_food_action_fail_at(food_xy: Option<(i32, i32)>, x: i32, y: i32) -> bool {
    food_xy
        .map(|(fx, fy)| fx == x && fy == y)
        .unwrap_or(false)
}

/// Empty-hand USE on edible tile — treat as food-pickup fail when no sticky claim.
///
/// Haxe `isPickingupFood` USE path runs with empty hands after dropHeld; berry bush
/// harvest has `foodValue > 0`. Profession USE usually holds a tool (`held != 0`).
// Haxe: isPickingupFood isUse empty-hand use ~8688 / fail ~8698
#[inline]
pub fn is_empty_hand_food_use_fail(held_id: i32, target_food_value: i32) -> bool {
    held_id == 0 && target_food_value > 0
}

/// Mark 30s not_reachable + clear sticky foodTarget / action food claim.
// Haxe: AiBase.isPickingupFood ~8694–8700
// AI-FOOD-FAIL-MARK
pub fn apply_food_action_fail(
    maps: &mut AiPathReachMaps,
    sticky_food: &mut Option<StickyFoodTarget>,
    action_targets: Option<&mut AiStickyBlockTargets>,
    x: i32,
    y: i32,
) {
    mark_food_path_fail(maps, x, y);
    let e = food_action_fail_effects();
    if e.clear_food_target {
        *sticky_food = None;
        if let Some(t) = action_targets {
            if t.food_target
                .map(|c| c.x == x && c.y == y)
                .unwrap_or(false)
            {
                t.food_target = None;
            }
        }
    }
}

/// Live maps helper: empty-hand food sticky claim at `(x,y)` → 30s mark + clear claim.
///
/// Held tool on a food tile is **not** isPickingupFood (shortCraft may note Food for
/// blockedByAI) — those fail via age-gated USE mark instead.
///
/// Returns `true` when the food fail path applied (caller skips general USE mark).
// Haxe: AiBase.isPickingupFood ~8698 (empty hands after dropHeld)
// AI-FOOD-FAIL-MARK
pub fn try_mark_food_action_fail_on_maps(
    maps: &mut AiPathReachMaps,
    action_targets: &mut AiStickyBlockTargets,
    x: i32,
    y: i32,
    held_id: i32,
) -> bool {
    if held_id != 0 {
        return false;
    }
    let food_xy = action_targets.food_target.map(|c| (c.x, c.y));
    if !is_food_action_fail_at(food_xy, x, y) {
        return false;
    }
    mark_food_path_fail(maps, x, y);
    action_targets.food_target = None;
    true
}

/// Prefer food 30s over age-gated USE fail for isPickingupFood-style fails.
///
/// Food 30s when: empty hands + (sticky food claim at tile **or** tile foodValue > 0).
/// Else age-gated general USE fail (90s / hostile).
///
/// Returns `true` when food 30s was applied.
// Haxe: isPickingupFood fail 30s vs isUsingObject fail age-gate ~9133
// AI-FOOD-FAIL-MARK
pub fn mark_use_or_food_path_fail(
    maps: &mut AiPathReachMaps,
    action_targets: &mut AiStickyBlockTargets,
    x: i32,
    y: i32,
    age: f32,
    held_id: i32,
    target_food_value: i32,
) -> bool {
    if try_mark_food_action_fail_on_maps(maps, action_targets, x, y, held_id) {
        return true;
    }
    if is_empty_hand_food_use_fail(held_id, target_food_value) {
        mark_food_path_fail(maps, x, y);
        // Clear food sticky if present even when food_value was the trigger
        if action_targets
            .food_target
            .map(|c| c.x == x && c.y == y)
            .unwrap_or(false)
        {
            action_targets.food_target = None;
        }
        return true;
    }
    mark_use_path_fail(maps, x, y, age);
    false
}


/// Whether a pending food USE/DROP/REMV tile still looks actionable for settle.
///
/// Ground edible: `ground_food_value > 0`. Container/basket REMV: ground food_value
/// is often 0 (basket itself is not food) — treat as still-food when
/// `pending_is_container && tile_id != 0`.
// Haxe: isPickingupFood isInContainer REMV fail still marks 30s (~8684–8698)
// AI-FOOD-FAIL-MARK
#[inline]
pub fn pending_food_tile_still_actionable(
    ground_food_value: i32,
    pending_is_container: bool,
    tile_id: i32,
) -> bool {
    if ground_food_value > 0 {
        return true;
    }
    // Container present at tile (food still in basket / REMV failed).
    pending_is_container && tile_id != 0
}

/// Settle async food USE/DROP/REMV after send (Rust net is async; Haxe is sync).
///
/// If hands still empty and tile still food → `mark_food_path_fail` 30s + clear sticky.
/// If held anything → treat as success (no mark).
// Haxe: AiBase.isPickingupFood done==false ~8694–8700 (sync use/remove/drop)
// AI-FOOD-FAIL-MARK / food_use_fail_30s
pub fn settle_pending_food_use_fail(
    maps: &mut AiPathReachMaps,
    sticky_food: &mut Option<StickyFoodTarget>,
    pending_xy: Option<(i32, i32)>,
    held_id: i32,
    tile_still_food: bool,
) -> bool {
    let Some((x, y)) = pending_xy else {
        return false;
    };
    // Picked something up (food or other) — do not mark the prior food tile.
    if held_id != 0 {
        return false;
    }
    if !tile_still_food {
        return false;
    }
    apply_food_action_fail(maps, sticky_food, None, x, y);
    true
}

/// DROP/REMV fail on sticky food or remove-from-container claim → 30s mark.
///
/// Unlike USE path, held may still be non-zero (failed swap-DROP on berry bowl).
/// Does **not** age-gate general USE (callers use this only for food-pickup actions).
///
/// Returns `true` when 30s food mark applied.
// Haxe: AiBase.isPickingupFood drop/remove done==false ~8684–8699
// AI-FOOD-FAIL-MARK
pub fn mark_food_pickup_action_fail_on_maps(
    maps: &mut AiPathReachMaps,
    action_targets: &mut AiStickyBlockTargets,
    sticky_food: Option<&mut Option<StickyFoodTarget>>,
    x: i32,
    y: i32,
) -> bool {
    let food_claim = action_targets
        .food_target
        .map(|c| c.x == x && c.y == y)
        .unwrap_or(false);
    let remv_claim = action_targets
        .remove_from_container_target
        .map(|c| c.x == x && c.y == y)
        .unwrap_or(false);
    let sticky_claim = sticky_food
        .as_ref()
        .and_then(|s| s.as_ref())
        .map(|s| s.x == x && s.y == y)
        .unwrap_or(false);
    if !food_claim && !remv_claim && !sticky_claim {
        return false;
    }
    mark_food_path_fail(maps, x, y);
    if food_claim {
        action_targets.food_target = None;
    }
    if remv_claim {
        action_targets.remove_from_container_target = None;
    }
    if let Some(sf) = sticky_food {
        if sf.map(|s| s.x == x && s.y == y).unwrap_or(false) {
            *sf = None;
        }
    }
    true
}

/// Merge `src` not_reachable (+ hostile) timers into `dst` (max remaining secs).
///
/// Dual-map ownership: live NetIntent marks `Player.ai_path_reach`; NPC SeekFood
/// uses `NpcProfessionState.path_reach`. Callers pull Player → NPC each tick.
// AI-FOOD-FAIL-MARK / PATH-REACH dual-map
pub fn merge_path_reach_maps(dst: &mut AiPathReachMaps, src: &AiPathReachMaps) {
    for (&xy, &t) in &src.not_reachable {
        dst.not_reachable
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
    for (&xy, &t) in &src.hostile_path {
        dst.hostile_path
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
}


/// PATH-REACH-MERGE: max-merge both ways so dual ownership matches Haxe single maps.
///
/// After this, `a` and `b` hold identical max timers for every key.
// Haxe: AiBase L85–86 single notReachableObjects / objectsWithHostilePath
// PATH-REACH-MERGE / dual_map_merge
pub fn sync_path_reach_bidirectional(a: &mut AiPathReachMaps, b: &mut AiPathReachMaps) {
    merge_path_reach_maps(a, b);
    merge_path_reach_maps(b, a);
}

/// PATH-REACH-MERGE: when overwriting a published `PlayerSnapshot`, keep unabsorbed
/// NPC path marks already written into `player_views` (max timers).
///
/// Without this, `publish_player_view` / `publish_all_player_views` replace the view
/// from `Player` only and can drop marks pushed after the last
/// `merge_npc_path_reach_from_views` absorb.
// Haxe: AiBase L85–86 single maps (no dual-view publish race)
// PATH-REACH-MERGE / dual_map_merge
pub fn preserve_view_path_reach_on_publish(
    new_maps: &mut AiPathReachMaps,
    prev_view_maps: Option<&AiPathReachMaps>,
) {
    if let Some(prev) = prev_view_maps {
        if !prev.is_empty() {
            merge_path_reach_maps(new_maps, prev);
        }
    }
}

/// Haxe `gotoAdv` fail: animal-only block → hostile; else not-reachable.
// Haxe: AiHelper.gotoAdv ~1140–1141
pub fn mark_goto_path_fail(maps: &mut AiPathReachMaps, x: i32, y: i32, blocked_by_animal: bool) {
    if blocked_by_animal {
        maps.add_hostile_path(x, y, HOSTILE_PATH_DEFAULT_SECS);
    } else {
        maps.add_not_reachable(x, y, NOT_REACHABLE_DEFAULT_SECS);
    }
}

// ── Animal-aware Goto pure gates (AI-ANIMAL-GOTO) ───────────────────────────

/// Haxe `gotoAdv` `considerAnimals` gate.
///
/// `checkIfDangerous && didNotReachFood < 5 && food_store > -1`.
// Haxe: AiHelper.gotoAdv ~1116
#[inline]
pub fn consider_animals_for_goto(
    check_if_dangerous: bool,
    did_not_reach_food: f32,
    food_store: f32,
) -> bool {
    check_if_dangerous && did_not_reach_food < 5.0 && food_store > -1.0
}

/// After path-with-animals failed: true when a recheck **without** animals succeeds.
///
/// That means only deadly animals blocked the path → `hostile_path` (not `not_reachable`).
// Haxe: AiHelper.gotoAdv ~1121–1141
#[inline]
pub fn blocked_by_animal_from_dual_pass(
    consider_animals: bool,
    path_ok_without_animals: bool,
) -> bool {
    consider_animals && path_ok_without_animals
}

/// Haxe `gotoObj` receding-target abort (quad distance, same object).
///
/// `lastGotoObj == obj && lastGotoObjDistance <= distance && distance > 100`
/// → `addObjectWithHostilePath` and abort.
// Haxe: AiHelper.gotoObj ~1086–1091
pub const RECEDING_GOTO_DIST_QUAD: f32 = 100.0;

#[inline]
pub fn receding_goto_should_abort(
    same_target: bool,
    last_dist_quad: f32,
    current_dist_quad: f32,
) -> bool {
    same_target
        && last_dist_quad <= current_dist_quad
        && current_dist_quad > RECEDING_GOTO_DIST_QUAD
}

// ── Food / explore sticky Goto (AI-GOTO-FOOD) ───────────────────────────────

/// Quad distance used by Haxe `gotoObj` / `CalculateQuadDistanceToObject`.
// Haxe: AiHelper.CalculateQuadDistanceToObject / gotoObj ~1085
#[inline]
pub fn goto_quad_distance(px: i32, py: i32, tx: i32, ty: i32) -> f32 {
    let dx = (tx - px) as f32;
    let dy = (ty - py) as f32;
    dx * dx + dy * dy
}

/// Sticky `lastGotoObj` identity (tile + parent id).
// Haxe: AiBase.lastGotoObj + lastGotoObjDistance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LastGotoObj {
    pub x: i32,
    pub y: i32,
    /// Parent object id; `None` means no sticky last target.
    pub parent_id: i32,
}

impl LastGotoObj {
    pub fn new(x: i32, y: i32, parent_id: i32) -> Self {
        Self { x, y, parent_id }
    }

    #[inline]
    pub fn matches(&self, x: i32, y: i32, parent_id: i32) -> bool {
        self.x == x && self.y == y && self.parent_id == parent_id
    }
}

/// Sticky `foodTarget` lite (world tile + parent id + optional container slot).
// Haxe: AiBase.foodTarget / ObjectHelper.indexInContainer (AI-PICKUP-FOOD)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyFoodTarget {
    pub x: i32,
    pub y: i32,
    pub parent_id: i32,
    /// Haxe `indexInContainer`; `-1` = ground object.
    // Haxe: ObjectHelper.indexInContainer
    pub index_in_container: i32,
}

impl StickyFoodTarget {
    pub fn new(x: i32, y: i32, parent_id: i32) -> Self {
        Self {
            x,
            y,
            parent_id,
            index_in_container: -1,
        }
    }

    /// Sticky food inside a container slot.
    // Haxe: foodTarget.indexInContainer >= 0
    pub fn with_container(x: i32, y: i32, parent_id: i32, index_in_container: i32) -> Self {
        Self {
            x,
            y,
            parent_id,
            index_in_container,
        }
    }

    #[inline]
    pub fn in_container(self) -> bool {
        self.index_in_container >= 0
    }
}

/// Plan for Haxe `gotoObj` receding check + lastGoto bookkeeping.
// Haxe: AiHelper.gotoObj ~1081–1099
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GotoObjPlan {
    /// Same target receding past threshold → `addObjectWithHostilePath`, clear last.
    AbortReceding { dist_quad: f32 },
    /// Update lastGotoObj + distance, then `gotoAdv`.
    Proceed { dist_quad: f32 },
}

/// Haxe `gotoObj` pure plan (receding abort vs proceed).
///
/// `last` / `last_dist_quad` are prior sticky lastGotoObj state (`None` = first visit).
// Haxe: AiHelper.gotoObj ~1085–1099
pub fn plan_goto_obj(
    last: Option<LastGotoObj>,
    last_dist_quad: f32,
    target: LastGotoObj,
    player_x: i32,
    player_y: i32,
) -> GotoObjPlan {
    let dist = goto_quad_distance(player_x, player_y, target.x, target.y);
    let same = last
        .map(|l| l.matches(target.x, target.y, target.parent_id))
        .unwrap_or(false);
    if receding_goto_should_abort(same, last_dist_quad, dist) {
        GotoObjPlan::AbortReceding { dist_quad: dist }
    } else {
        GotoObjPlan::Proceed { dist_quad: dist }
    }
}

/// Side effects when food path/goto fails (clear sticky + count miss).
// Haxe: isPickingupFood done==false → foodTarget=null; gotoAdv → resetTargets
// Haxe: escape with foodTarget → didNotReachFood++
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodGotoFailEffects {
    /// Clear sticky `foodTarget`.
    pub clear_food_target: bool,
    /// Clear `lastGotoObj`.
    pub clear_last_goto: bool,
    /// Increment `didNotReachFood` (had a food target on fail/abort).
    pub increment_did_not_reach_food: bool,
    /// Call `resetTargets` (clear food/use/drop sticky claims).
    pub reset_action_targets: bool,
}

/// Effects after food `gotoObj`/`gotoAdv` returns false.
// Haxe: AiBase.isPickingupFood ~8652; AiHelper.gotoAdv ~1148
pub fn food_goto_fail_effects(had_food_target: bool) -> FoodGotoFailEffects {
    FoodGotoFailEffects {
        clear_food_target: true,
        clear_last_goto: true,
        increment_did_not_reach_food: had_food_target,
        reset_action_targets: true,
    }
}

/// Apply [`food_goto_fail_effects`] onto counters + sticky options.
pub fn apply_food_goto_fail(
    did_not_reach_food: &mut f32,
    sticky_food: &mut Option<StickyFoodTarget>,
    last_goto: &mut Option<LastGotoObj>,
    last_goto_dist: &mut f32,
    action_targets: Option<&mut AiStickyBlockTargets>,
) {
    let had = sticky_food.is_some();
    let e = food_goto_fail_effects(had);
    if e.increment_did_not_reach_food {
        *did_not_reach_food += 1.0;
    }
    if e.clear_food_target {
        *sticky_food = None;
    }
    if e.clear_last_goto {
        *last_goto = None;
        *last_goto_dist = -1.0;
    }
    if e.reset_action_targets {
        if let Some(t) = action_targets {
            t.clear_action_targets();
        }
    }
}

/// Successful food pickup resets `didNotReachFood` (Haxe isPickingupFood after USE).
// Haxe: AiBase.isPickingupFood ~8703
#[inline]
pub fn food_pickup_success_reset_did_not_reach() -> f32 {
    0.0
}

/// Sticky food still valid when claimed tile still holds edible food.
///
/// Multi-use id change on the same tile still counts while `food_value > 0`.
// Haxe: AiBase.isPickingupFood isEatableCheckAgain ~8618–8621
#[inline]
pub fn sticky_food_still_valid(
    _sticky: StickyFoodTarget,
    tile_id: i32,
    tile_food_value: i32,
) -> bool {
    tile_id != 0 && tile_food_value > 0
}

/// Keep sticky food when still valid; else adopt `candidate` as new sticky.
// Haxe: SearchBestFood only when foodTarget == null
pub fn resolve_sticky_food(
    sticky: Option<StickyFoodTarget>,
    sticky_tile_id: i32,
    sticky_tile_food_value: i32,
    candidate: Option<StickyFoodTarget>,
) -> Option<StickyFoodTarget> {
    if let Some(s) = sticky {
        if sticky_food_still_valid(s, sticky_tile_id, sticky_tile_food_value) {
            return Some(s);
        }
    }
    candidate
}

// ── CalculateBlockedByAi pure (Haxe ~222–302) ──────────────────────────────

/// One tile claim that may enter `blockedByAI` (Haxe `ObjectHelper` target).
// Haxe: AiBase.AddTargetBlockedByAi target fields
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockTargetClaim {
    pub x: i32,
    pub y: i32,
    /// Haxe `target.parentId`.
    pub parent_id: i32,
    /// Haxe `target.numberOfUses` — multi-use (`>1`) skips block but stops agent chain.
    pub number_of_uses: i32,
    /// Haxe `target.objectData.isAnimal()`.
    pub is_animal: bool,
    /// When `Some(new_target_id)` for held+target transition: if equal to `parent_id`,
    /// do **not** block and continue the agent target chain (`return false` in Haxe).
    /// Caller resolves `TransitionImporter.GetTransition(held, target).newTargetID`.
    // Haxe: AddTargetBlockedByAi heldObj branch ~294–297
    pub held_new_target_id: Option<i32>,
}

impl BlockTargetClaim {
    pub fn simple(x: i32, y: i32, parent_id: i32) -> Self {
        Self {
            x,
            y,
            parent_id,
            number_of_uses: 1,
            is_animal: false,
            held_new_target_id: None,
        }
    }
}

/// Outcome of pure [`try_add_target_blocked_by_ai`].
// Haxe: AddTargetBlockedByAi Bool — true = stop agent chain; false = try next target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddTargetBlockResult {
    /// No target / null — keep chain (`return false`).
    SkipContinue,
    /// Filtered (multi-use / animal / DontBlock) without inserting — stop chain (`return true`).
    FilteredStop,
    /// Held→target leaves object unchanged — keep chain without insert (`return false`).
    SameTargetContinue,
    /// Tile inserted into blockedByAI — stop chain (`return true`).
    AddedStop,
}

/// Pure Haxe `AddTargetBlockedByAi` filters (no mutex).
///
/// Returns whether the agent target chain should stop and whether the tile was added.
// Haxe: AiBase.AddTargetBlockedByAi ~286–302
pub fn try_add_target_blocked_by_ai(
    map: &mut HashMap<(i32, i32), f32>,
    claim: Option<&BlockTargetClaim>,
) -> AddTargetBlockResult {
    let Some(c) = claim else {
        return AddTargetBlockResult::SkipContinue;
    };
    // Haxe: numberOfUses > 1 → return true (stop, no add)
    if c.number_of_uses > 1 {
        return AddTargetBlockResult::FilteredStop;
    }
    // Haxe: isAnimal → return true
    if c.is_animal {
        return AddTargetBlockResult::FilteredStop;
    }
    // Haxe: DontBlockByAi.contains(parentId) → return true
    if DONT_BLOCK_BY_AI.contains(&c.parent_id) {
        return AddTargetBlockResult::FilteredStop;
    }
    // Haxe: held + transition same newTarget → return false (continue, no add)
    // allows multi-AI to share Hot Adobe Oven / kiln bowl uses
    if let Some(new_tid) = c.held_new_target_id {
        if new_tid == c.parent_id {
            return AddTargetBlockResult::SameTargetContinue;
        }
    }
    add_blocked_by_ai(map, c.x, c.y, BLOCKED_BY_AI_DEFAULT_SECS);
    AddTargetBlockResult::AddedStop
}

/// True when a claim would be inserted (ignores chain-stop semantics).
pub fn would_block_target_by_ai(claim: &BlockTargetClaim) -> bool {
    if claim.number_of_uses > 1 {
        return false;
    }
    if claim.is_animal {
        return false;
    }
    if DONT_BLOCK_BY_AI.contains(&claim.parent_id) {
        return false;
    }
    if let Some(new_tid) = claim.held_new_target_id {
        if new_tid == claim.parent_id {
            return false;
        }
    }
    true
}

/// One living AI's targets for `AddToBlockedByAi` (order matches Haxe).
// Haxe: AiBase.AddToBlockedByAi ~242–258
#[derive(Debug, Clone, Default)]
pub struct AiAgentBlockSource {
    pub age: f32,
    pub is_wounded: bool,
    pub deleted: bool,
    /// Haxe `player.blockTargetForAi` if `timePassed < 20`.
    pub player_block_target: Option<BlockTargetClaim>,
    /// Haxe `ai.myPlayer.blockTargetForAi`.
    pub ai_block_target: Option<BlockTargetClaim>,
    /// Haxe `ai.foodTarget`.
    pub food_target: Option<BlockTargetClaim>,
    /// Haxe `ai.dropTarget`.
    pub drop_target: Option<BlockTargetClaim>,
    /// Haxe `ai.useTarget` (+ held for transition exception).
    pub use_target: Option<BlockTargetClaim>,
    /// Haxe `ai.removeFromContainerTarget`.
    pub remove_from_container_target: Option<BlockTargetClaim>,
}

/// Haxe human `blockTargetForAi` when claim age ≤ 20s.
// Haxe: CalculateBlockedByAi living humans ~225–233
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HumanBlockClaim {
    pub claim: BlockTargetClaim,
    /// Seconds since `blockTargetTime` (Haxe `CalculateTimeSinceTicksInSec`).
    pub age_secs: f32,
}

/// Pure Haxe `AddToBlockedByAi` for one agent into `map`.
// Haxe: AiBase.AddToBlockedByAi ~242–258
pub fn add_agent_to_blocked_by_ai(
    map: &mut HashMap<(i32, i32), f32>,
    agent: &AiAgentBlockSource,
) {
    if agent.deleted {
        return;
    }
    if agent.age < BLOCK_BY_AI_MIN_AGE {
        return;
    }
    if agent.is_wounded {
        return;
    }
    // player.blockTargetForAi when timePassed < 20 (caller pre-filters Option)
    let _ = try_add_target_blocked_by_ai(map, agent.player_block_target.as_ref());
    // ordered early-return chain
    if matches!(
        try_add_target_blocked_by_ai(map, agent.ai_block_target.as_ref()),
        AddTargetBlockResult::FilteredStop | AddTargetBlockResult::AddedStop
    ) {
        return;
    }
    if matches!(
        try_add_target_blocked_by_ai(map, agent.food_target.as_ref()),
        AddTargetBlockResult::FilteredStop | AddTargetBlockResult::AddedStop
    ) {
        return;
    }
    if matches!(
        try_add_target_blocked_by_ai(map, agent.drop_target.as_ref()),
        AddTargetBlockResult::FilteredStop | AddTargetBlockResult::AddedStop
    ) {
        return;
    }
    if matches!(
        try_add_target_blocked_by_ai(map, agent.use_target.as_ref()),
        AddTargetBlockResult::FilteredStop | AddTargetBlockResult::AddedStop
    ) {
        return;
    }
    let _ = try_add_target_blocked_by_ai(map, agent.remove_from_container_target.as_ref());
}

/// Pure Haxe `CalculateBlockedByAi` — wipe + rebuild from humans + living AIs.
// Haxe: AiBase.CalculateBlockedByAi ~222–239
pub fn calculate_blocked_by_ai(
    humans: &[HumanBlockClaim],
    agents: &[AiAgentBlockSource],
) -> HashMap<(i32, i32), f32> {
    let mut map = HashMap::new();
    for h in humans {
        if h.age_secs > BLOCK_TARGET_MAX_AGE_SECS {
            continue;
        }
        let _ = try_add_target_blocked_by_ai(&mut map, Some(&h.claim));
    }
    for a in agents {
        add_agent_to_blocked_by_ai(&mut map, a);
    }
    map
}

/// Replace `SimState.blocked_by_ai` from pure rebuild (Haxe wipe-each-AI-frame).
// Haxe: CalculateBlockedByAi assigns `blockedByAI = new Map` then fills
pub fn apply_calculate_blocked_by_ai(
    dest: &mut HashMap<(i32, i32), f32>,
    humans: &[HumanBlockClaim],
    agents: &[AiAgentBlockSource],
) {
    *dest = calculate_blocked_by_ai(humans, agents);
}

// ── Sticky targets → live CalculateBlockedByAi (BLOCKED-BY-AI) ──────────────

/// Kind of sticky action target for live AI intent notes.
// Haxe: AiBase.foodTarget / dropTarget / useTarget / removeFromContainerTarget
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickyBlockIntentKind {
    Food,
    Drop,
    Use,
    RemoveFromContainer,
}

/// Sticky AI block claims that feed live [`calculate_blocked_by_ai`] each tick.
///
/// Haxe keeps these on `AiBase` / `GlobalPlayerInstance`; Rust stores them on
/// Player and rebuilds `SimState.blocked_by_ai` via
/// [`rebuild_blocked_by_ai_from_sticky`].
// Haxe: AiBase.foodTarget / dropTarget / useTarget / removeFromContainerTarget
// Haxe: GlobalPlayerInstance.blockTargetForAi + blockTargetTime
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiStickyBlockTargets {
    /// Haxe `ai.foodTarget`.
    pub food_target: Option<BlockTargetClaim>,
    /// Haxe `ai.dropTarget`.
    pub drop_target: Option<BlockTargetClaim>,
    /// Haxe `ai.useTarget` (+ held transition exception via claim field).
    pub use_target: Option<BlockTargetClaim>,
    /// Haxe `ai.removeFromContainerTarget`.
    pub remove_from_container_target: Option<BlockTargetClaim>,
    /// Haxe `ai.myPlayer.blockTargetForAi`.
    pub ai_block_target: Option<BlockTargetClaim>,
    /// Haxe `player.blockTargetForAi` (human USE / smith hammer).
    pub player_block: Option<BlockTargetClaim>,
    /// `SimState.sim_time` when [`Self::player_block`] was set.
    // Haxe: GlobalPlayerInstance.blockTargetTime
    pub player_block_sim_time: f32,
}

impl AiStickyBlockTargets {
    pub fn clear_action_targets(&mut self) {
        self.food_target = None;
        self.drop_target = None;
        self.use_target = None;
        self.remove_from_container_target = None;
        self.ai_block_target = None;
    }

    pub fn clear_all(&mut self) {
        *self = Self::default();
    }

    pub fn set_food(&mut self, c: BlockTargetClaim) {
        self.food_target = Some(c);
    }

    pub fn set_drop(&mut self, c: BlockTargetClaim) {
        self.drop_target = Some(c);
    }

    pub fn set_use(&mut self, c: BlockTargetClaim) {
        self.use_target = Some(c);
    }

    pub fn set_remove_from_container(&mut self, c: BlockTargetClaim) {
        self.remove_from_container_target = Some(c);
    }

    pub fn set_ai_block(&mut self, c: BlockTargetClaim) {
        self.ai_block_target = Some(c);
    }

    /// Haxe: `player.blockTargetForAi = target; player.blockTargetTime = tick`.
    pub fn set_player_block(&mut self, c: BlockTargetClaim, sim_time: f32) {
        self.player_block = Some(c);
        self.player_block_sim_time = sim_time;
    }

    pub fn clear_player_block(&mut self) {
        self.player_block = None;
        self.player_block_sim_time = 0.0;
    }

    /// Seconds since `player_block` was set (`sim_time - stamp`).
    // Haxe: CalculateTimeSinceTicksInSec(player.blockTargetTime)
    pub fn player_block_age_secs(&self, sim_time: f32) -> f32 {
        if self.player_block.is_none() {
            return f32::MAX;
        }
        (sim_time - self.player_block_sim_time).max(0.0)
    }

    /// Record sticky action claim (food / drop / use / remove).
    pub fn note_action_claim(&mut self, kind: StickyBlockIntentKind, claim: BlockTargetClaim) {
        match kind {
            StickyBlockIntentKind::Food => self.food_target = Some(claim),
            StickyBlockIntentKind::Drop => self.drop_target = Some(claim),
            StickyBlockIntentKind::Use => self.use_target = Some(claim),
            StickyBlockIntentKind::RemoveFromContainer => {
                self.remove_from_container_target = Some(claim);
            }
        }
    }

    /// Build pure [`AiAgentBlockSource`] for one living AI body.
    // Haxe: AddToBlockedByAi ~242–258
    pub fn to_agent_block_source(
        &self,
        age: f32,
        is_wounded: bool,
        deleted: bool,
        sim_time: f32,
    ) -> AiAgentBlockSource {
        // Haxe: first `player.blockTargetForAi` only when timePassed < 20
        let player_block_target =
            if self.player_block_age_secs(sim_time) < BLOCK_TARGET_MAX_AGE_SECS {
                self.player_block
            } else {
                None
            };
        // Haxe: `ai.myPlayer.blockTargetForAi` has no second age gate and early-stops
        // the food/drop/use chain. Same GPI field as player.blockTargetForAi in Haxe —
        // mirror by falling back to sticky player_block (even when age > 20).
        let ai_block_target = self.ai_block_target.or(self.player_block);
        AiAgentBlockSource {
            age,
            is_wounded,
            deleted,
            player_block_target,
            ai_block_target,
            food_target: self.food_target,
            drop_target: self.drop_target,
            use_target: self.use_target,
            remove_from_container_target: self.remove_from_container_target,
        }
    }

    /// Human living-body claim for the CalculateBlockedByAi human loop.
    // Haxe: CalculateBlockedByAi living humans ~225–233
    pub fn to_human_block_claim(&self, sim_time: f32) -> Option<HumanBlockClaim> {
        let claim = self.player_block?;
        Some(HumanBlockClaim {
            claim,
            age_secs: self.player_block_age_secs(sim_time),
        })
    }
}

/// Instance `numberOfUses` for [`BlockTargetClaim`] / AddTarget filters.
///
/// Prefer live world helper `uses_remaining`. When unknown (0 / no helper), default
/// to **1** — do **not** fall back to `ObjectData.num_uses` (would mark full
/// multi-use templates as FilteredStop vs Haxe instance uses).
// Haxe: ObjectHelper.numberOfUses in AddTargetBlockedByAi ~288
#[inline]
pub fn block_claim_number_of_uses(instance_uses_remaining: i32) -> i32 {
    if instance_uses_remaining > 0 {
        instance_uses_remaining
    } else {
        1
    }
}

/// Haxe `TransitionHelper.use` gates for setting `blockTargetForAi` after USE.
///
/// Blocks last target for AI when human (or smith hammer 441), unless permanent
/// (except hammer), weapon, animal, food, or clothing.
// Haxe: TransitionHelper.use ~397–414
pub fn should_set_block_target_for_ai(
    is_human: bool,
    held_id: i32,
    target_parent_id: i32,
    target_permanent: bool,
    target_is_weapon: bool,
    target_is_animal: bool,
    target_food_value: i32,
    target_is_clothing: bool,
) -> bool {
    if target_parent_id == 0 {
        return false;
    }
    let is_hammer = held_id == SMITHING_HAMMER_BLOCK_ID;
    let mut block = is_human || is_hammer;
    if !block {
        return false;
    }
    if target_permanent && !is_hammer {
        block = false;
    }
    if target_is_weapon {
        block = false;
    }
    if target_is_animal {
        block = false;
    }
    if target_food_value > 0 {
        block = false;
    }
    if target_is_clothing {
        block = false;
    }
    block
}

/// One living body's sticky row for rebuild (AI agent or human claim).
#[derive(Debug, Clone)]
pub struct StickyBlockBodyRow {
    pub is_ai: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub deleted: bool,
    pub sticky: AiStickyBlockTargets,
}

/// Pure rebuild of `blockedByAI` from sticky body rows (no SimState).
// Haxe: CalculateBlockedByAi ~222–239
pub fn rebuild_blocked_by_ai_from_sticky(
    sim_time: f32,
    bodies: &[StickyBlockBodyRow],
) -> HashMap<(i32, i32), f32> {
    let mut humans = Vec::new();
    let mut agents = Vec::new();
    for b in bodies {
        if b.deleted {
            continue;
        }
        if b.is_ai {
            agents.push(b.sticky.to_agent_block_source(
                b.age,
                b.is_wounded,
                false,
                sim_time,
            ));
        } else if let Some(h) = b.sticky.to_human_block_claim(sim_time) {
            humans.push(h);
        }
    }
    calculate_blocked_by_ai(&humans, &agents)
}

/// Apply pure sticky rebuild into `dest` (Haxe wipe-each-AI-frame).
// Haxe: blockedByAI = new Map then fill
pub fn apply_rebuild_blocked_by_ai_from_sticky(
    dest: &mut HashMap<(i32, i32), f32>,
    sim_time: f32,
    bodies: &[StickyBlockBodyRow],
) {
    *dest = rebuild_blocked_by_ai_from_sticky(sim_time, bodies);
}

fn decay_timed_map(map: &mut HashMap<(i32, i32), f32>, dt: f32) {
    map.retain(|_, t| {
        *t -= dt;
        *t > 0.0
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_not_reachable_default_and_query() {
        let mut m = AiPathReachMaps::new();
        m.add_not_reachable(3, 4, 0.0); // default 90
        assert!(m.is_personal_not_reachable(3, 4));
        assert!((m.not_reachable[&(3, 4)] - NOT_REACHABLE_DEFAULT_SECS).abs() < 0.01);
        assert!(!m.is_object_with_hostile_path(3, 4));
        assert!(m.blocks_target(3, 4, None));
    }

    #[test]
    fn hostile_path_and_blocked_by_ai() {
        let mut m = AiPathReachMaps::new();
        m.add_hostile_path(1, 1, HOSTILE_PATH_DEFAULT_SECS);
        assert!(m.is_object_with_hostile_path(1, 1));
        assert!(!m.is_personal_not_reachable(1, 1));
        assert!(m.blocks_target(1, 1, None));

        let mut global = HashMap::new();
        add_blocked_by_ai(&mut global, 9, 9, BLOCKED_BY_AI_DEFAULT_SECS);
        assert!(m.is_object_not_reachable(9, 9, Some(&global)));
        assert!(!m.is_object_not_reachable(9, 9, None));
    }

    #[test]
    fn cleanup_decays_and_removes() {
        let mut m = AiPathReachMaps::new();
        m.add_not_reachable(0, 0, 10.0);
        m.add_hostile_path(1, 0, 5.0);
        m.cleanup(4.0);
        assert!((m.not_reachable[&(0, 0)] - 6.0).abs() < 0.01);
        assert!((m.hostile_path[&(1, 0)] - 1.0).abs() < 0.01);
        m.cleanup(2.0);
        assert!(!m.hostile_path.contains_key(&(1, 0)));
        assert!(m.not_reachable.contains_key(&(0, 0)));
        m.cleanup(10.0);
        assert!(m.is_empty());
    }

    #[test]
    fn blocked_coords_merges_all_sources() {
        let mut m = AiPathReachMaps::new();
        m.add_not_reachable(1, 0, 90.0);
        m.add_hostile_path(2, 0, 20.0);
        let mut global = HashMap::new();
        add_blocked_by_ai(&mut global, 3, 0, 5.0);
        let s = m.blocked_coords(Some(&global));
        assert!(s.contains(&(1, 0)));
        assert!(s.contains(&(2, 0)));
        assert!(s.contains(&(3, 0)));
        assert!(!s.contains(&(4, 0)));
        let s2 = blocked_coords_from_live(&m, &global);
        assert_eq!(s, s2);
    }

    #[test]
    fn ttl_boundaries_90_20_5() {
        let mut m = AiPathReachMaps::new();
        m.add_not_reachable(0, 0, NOT_REACHABLE_DEFAULT_SECS);
        m.add_hostile_path(1, 0, HOSTILE_PATH_DEFAULT_SECS);
        let mut global = HashMap::new();
        add_blocked_by_ai(&mut global, 2, 0, BLOCKED_BY_AI_DEFAULT_SECS);

        m.cleanup(NOT_REACHABLE_DEFAULT_SECS - 0.01);
        assert!(m.is_personal_not_reachable(0, 0));
        m.cleanup(0.02);
        assert!(!m.is_personal_not_reachable(0, 0));

        // hostile already partially decayed with first cleanup (~90s) — use fresh
        let mut m2 = AiPathReachMaps::new();
        m2.add_hostile_path(1, 0, HOSTILE_PATH_DEFAULT_SECS);
        m2.cleanup(HOSTILE_PATH_DEFAULT_SECS - 0.01);
        assert!(m2.is_object_with_hostile_path(1, 0));
        m2.cleanup(0.02);
        assert!(!m2.is_object_with_hostile_path(1, 0));

        cleanup_blocked_by_ai(&mut global, BLOCKED_BY_AI_DEFAULT_SECS - 0.01);
        assert!(global.contains_key(&(2, 0)));
        cleanup_blocked_by_ai(&mut global, 0.02);
        assert!(!global.contains_key(&(2, 0)));
    }

    #[test]
    fn mark_use_path_fail_age_gate() {
        let mut m = AiPathReachMaps::new();
        mark_use_path_fail(&mut m, 5, 5, 14.0);
        assert!(m.is_personal_not_reachable(5, 5));
        assert!(!m.is_object_with_hostile_path(5, 5));

        let mut m2 = AiPathReachMaps::new();
        mark_use_path_fail(&mut m2, 5, 5, 2.0);
        assert!(!m2.is_personal_not_reachable(5, 5));
        assert!(m2.is_object_with_hostile_path(5, 5));
    }

    #[test]
    fn mark_food_path_fail_30s() {
        let mut m = AiPathReachMaps::new();
        mark_food_path_fail(&mut m, 1, 2);
        assert!((m.not_reachable[&(1, 2)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
    }

    // Haxe: AiBase.isPickingupFood use fail ~8694–8700 (AI-FOOD-FAIL-MARK)
    #[test]
    fn food_action_fail_30s_clears_sticky() {
        let e = food_action_fail_effects();
        assert!((e.not_reachable_secs - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(e.clear_food_target);

        assert!(is_food_action_fail_at(Some((3, 4)), 3, 4));
        assert!(!is_food_action_fail_at(Some((3, 4)), 0, 0));
        assert!(!is_food_action_fail_at(None, 3, 4));

        assert!(is_empty_hand_food_use_fail(0, 5));
        assert!(!is_empty_hand_food_use_fail(34, 5));
        assert!(!is_empty_hand_food_use_fail(0, 0));

        let mut maps = AiPathReachMaps::new();
        let mut sticky = Some(StickyFoodTarget::new(7, 8, 31));
        let mut targets = AiStickyBlockTargets::default();
        targets.set_food(BlockTargetClaim::simple(7, 8, 31));
        apply_food_action_fail(&mut maps, &mut sticky, Some(&mut targets), 7, 8);
        assert!((maps.not_reachable[&(7, 8)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(sticky.is_none());
        assert!(targets.food_target.is_none());
    }

    #[test]
    fn mark_use_or_food_path_fail_prefers_food_30s() {
        // Empty-hand + sticky food claim → 30s not age-gate 90s
        let mut maps = AiPathReachMaps::new();
        let mut targets = AiStickyBlockTargets::default();
        targets.set_food(BlockTargetClaim::simple(1, 1, 31));
        assert!(mark_use_or_food_path_fail(
            &mut maps, &mut targets, 1, 1, 20.0, 0, 0
        ));
        assert!((maps.not_reachable[&(1, 1)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(targets.food_target.is_none());

        // Empty-hand edible, no sticky → still 30s food mark
        let mut maps2 = AiPathReachMaps::new();
        let mut t2 = AiStickyBlockTargets::default();
        assert!(mark_use_or_food_path_fail(
            &mut maps2, &mut t2, 2, 2, 20.0, 0, 4
        ));
        assert!((maps2.not_reachable[&(2, 2)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);

        // Held tool even on food sticky → general USE fail (not isPickingupFood)
        let mut maps3 = AiPathReachMaps::new();
        let mut t3 = AiStickyBlockTargets::default();
        t3.set_food(BlockTargetClaim::simple(3, 3, 31));
        assert!(!mark_use_or_food_path_fail(
            &mut maps3, &mut t3, 3, 3, 20.0, 334, 5
        ));
        assert!((maps3.not_reachable[&(3, 3)] - NOT_REACHABLE_DEFAULT_SECS).abs() < 0.01);

        // Held tool, no food → age-gate 90s
        let mut maps4 = AiPathReachMaps::new();
        let mut t4 = AiStickyBlockTargets::default();
        assert!(!mark_use_or_food_path_fail(
            &mut maps4, &mut t4, 4, 4, 20.0, 334, 0
        ));
        assert!((maps4.not_reachable[&(4, 4)] - NOT_REACHABLE_DEFAULT_SECS).abs() < 0.01);
    }


    #[test]
    fn settle_pending_food_use_fail_marks_30s() {
        // Still empty-handed + tile food → 30s mark (async USE fail residual)
        let mut maps = AiPathReachMaps::new();
        let mut sticky = Some(StickyFoodTarget::new(5, 6, 31));
        assert!(settle_pending_food_use_fail(
            &mut maps,
            &mut sticky,
            Some((5, 6)),
            0,
            true,
        ));
        assert!((maps.not_reachable[&(5, 6)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(sticky.is_none());

        // Held something → success path, no mark
        let mut maps2 = AiPathReachMaps::new();
        let mut sticky2 = Some(StickyFoodTarget::new(1, 1, 31));
        assert!(!settle_pending_food_use_fail(
            &mut maps2,
            &mut sticky2,
            Some((1, 1)),
            34,
            true,
        ));
        assert!(maps2.not_reachable.is_empty());
        assert!(sticky2.is_some());

        // Tile gone → success, no mark
        let mut maps3 = AiPathReachMaps::new();
        let mut sticky3 = None;
        assert!(!settle_pending_food_use_fail(
            &mut maps3,
            &mut sticky3,
            Some((2, 2)),
            0,
            false,
        ));
        assert!(maps3.not_reachable.is_empty());
    }

    // Haxe: container REMV fail — basket ground food_value 0 still marks 30s
    // AI-FOOD-FAIL-MARK
    #[test]
    fn pending_food_tile_still_actionable_container() {
        assert!(pending_food_tile_still_actionable(5, false, 31));
        assert!(!pending_food_tile_still_actionable(0, false, 31));
        // Basket on ground: food_value 0, container pending, tile present
        assert!(pending_food_tile_still_actionable(0, true, 1001));
        // Empty ground after successful REMV
        assert!(!pending_food_tile_still_actionable(0, true, 0));
        assert!(!pending_food_tile_still_actionable(0, false, 0));
    }

    #[test]
    fn mark_food_pickup_action_fail_on_maps_drop_remv() {
        // Sticky food claim (DROP fail on berry bowl tile)
        let mut maps = AiPathReachMaps::new();
        let mut targets = AiStickyBlockTargets::default();
        targets.set_food(BlockTargetClaim::simple(3, 4, 31));
        let mut sticky = Some(StickyFoodTarget::new(3, 4, 31));
        assert!(mark_food_pickup_action_fail_on_maps(
            &mut maps,
            &mut targets,
            Some(&mut sticky),
            3,
            4,
        ));
        assert!((maps.not_reachable[&(3, 4)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(targets.food_target.is_none());
        assert!(sticky.is_none());

        // REMV claim only (container take fail)
        let mut maps2 = AiPathReachMaps::new();
        let mut t2 = AiStickyBlockTargets::default();
        t2.set_remove_from_container(BlockTargetClaim::simple(1, 2, 1001));
        assert!(mark_food_pickup_action_fail_on_maps(
            &mut maps2, &mut t2, None, 1, 2
        ));
        assert!((maps2.not_reachable[&(1, 2)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(t2.remove_from_container_target.is_none());

        // No claim → no mark
        let mut maps3 = AiPathReachMaps::new();
        let mut t3 = AiStickyBlockTargets::default();
        assert!(!mark_food_pickup_action_fail_on_maps(
            &mut maps3, &mut t3, None, 9, 9
        ));
        assert!(maps3.not_reachable.is_empty());
    }

    #[test]
    fn merge_path_reach_maps_max_timers() {
        let mut dst = AiPathReachMaps::new();
        dst.add_not_reachable(1, 1, 10.0);
        dst.add_hostile_path(2, 2, 5.0);
        let mut src = AiPathReachMaps::new();
        src.add_not_reachable(1, 1, 30.0); // max wins
        src.add_not_reachable(3, 3, 30.0); // new
        src.add_hostile_path(2, 2, 20.0);
        merge_path_reach_maps(&mut dst, &src);
        assert!((dst.not_reachable[&(1, 1)] - 30.0).abs() < 0.01);
        assert!((dst.not_reachable[&(3, 3)] - 30.0).abs() < 0.01);
        assert!((dst.hostile_path[&(2, 2)] - 20.0).abs() < 0.01);
    }

    #[test]
    fn sync_path_reach_bidirectional_equalizes() {
        // PATH-REACH-MERGE: both maps end with max of either
        let mut a = AiPathReachMaps::new();
        a.add_not_reachable(1, 1, 10.0);
        a.add_hostile_path(2, 2, 5.0);
        let mut b = AiPathReachMaps::new();
        b.add_not_reachable(1, 1, 30.0);
        b.add_not_reachable(3, 3, 40.0);
        b.add_hostile_path(2, 2, 20.0);
        sync_path_reach_bidirectional(&mut a, &mut b);
        assert!((a.not_reachable[&(1, 1)] - 30.0).abs() < 0.01);
        assert!((b.not_reachable[&(1, 1)] - 30.0).abs() < 0.01);
        assert!((a.not_reachable[&(3, 3)] - 40.0).abs() < 0.01);
        assert!((b.not_reachable[&(3, 3)] - 40.0).abs() < 0.01);
        assert!((a.hostile_path[&(2, 2)] - 20.0).abs() < 0.01);
        assert!((b.hostile_path[&(2, 2)] - 20.0).abs() < 0.01);
    }

    #[test]
    fn preserve_view_path_reach_on_publish_keeps_npc_marks() {
        // PATH-REACH-MERGE: publish must not drop unabsorbed NPC-pushed maps
        let mut from_player = AiPathReachMaps::new();
        from_player.add_not_reachable(1, 1, 10.0); // Player USE-fail
        let mut in_view = AiPathReachMaps::new();
        in_view.add_not_reachable(2, 2, 90.0); // NPC walk-fail not yet absorbed
        in_view.add_hostile_path(3, 3, 20.0);
        preserve_view_path_reach_on_publish(&mut from_player, Some(&in_view));
        assert!((from_player.not_reachable[&(1, 1)] - 10.0).abs() < 0.01);
        assert!((from_player.not_reachable[&(2, 2)] - 90.0).abs() < 0.01);
        assert!(from_player.is_object_with_hostile_path(3, 3));
        // max wins on overlap
        let mut p2 = AiPathReachMaps::new();
        p2.add_not_reachable(2, 2, 5.0);
        preserve_view_path_reach_on_publish(&mut p2, Some(&in_view));
        assert!((p2.not_reachable[&(2, 2)] - 90.0).abs() < 0.01);
        // None / empty prev is no-op
        let mut p3 = AiPathReachMaps::new();
        p3.add_not_reachable(9, 9, 1.0);
        preserve_view_path_reach_on_publish(&mut p3, None);
        assert!((p3.not_reachable[&(9, 9)] - 1.0).abs() < 0.01);
        preserve_view_path_reach_on_publish(&mut p3, Some(&AiPathReachMaps::new()));
        assert!((p3.not_reachable[&(9, 9)] - 1.0).abs() < 0.01);
    }


    #[test]
    fn mark_goto_path_fail_animal_vs_block() {
        let mut m = AiPathReachMaps::new();
        mark_goto_path_fail(&mut m, 3, 3, true);
        assert!(m.is_object_with_hostile_path(3, 3));
        assert!(!m.is_personal_not_reachable(3, 3));

        let mut m2 = AiPathReachMaps::new();
        mark_goto_path_fail(&mut m2, 3, 3, false);
        assert!(m2.is_personal_not_reachable(3, 3));
        assert!(!m2.is_object_with_hostile_path(3, 3));
    }

    // Haxe: AiHelper.gotoAdv ~1116 / dual-pass / gotoObj receding
    #[test]
    fn consider_animals_gate_and_dual_pass() {
        assert!(consider_animals_for_goto(true, 0.0, 5.0));
        assert!(consider_animals_for_goto(true, 4.9, 0.0));
        assert!(!consider_animals_for_goto(false, 0.0, 5.0));
        assert!(!consider_animals_for_goto(true, 5.0, 5.0));
        assert!(!consider_animals_for_goto(true, 0.0, -1.0));
        assert!(!consider_animals_for_goto(true, 0.0, -1.1));

        assert!(blocked_by_animal_from_dual_pass(true, true));
        assert!(!blocked_by_animal_from_dual_pass(true, false));
        assert!(!blocked_by_animal_from_dual_pass(false, true));
    }

    #[test]
    fn receding_goto_abort_threshold() {
        assert!(receding_goto_should_abort(true, 90.0, 101.0));
        assert!(receding_goto_should_abort(true, 100.0, 100.1));
        // distance not increasing
        assert!(!receding_goto_should_abort(true, 120.0, 110.0));
        // under threshold
        assert!(!receding_goto_should_abort(true, 50.0, 80.0));
        // different target
        assert!(!receding_goto_should_abort(false, 90.0, 101.0));
        // equal last==current and >100
        assert!(receding_goto_should_abort(true, 150.0, 150.0));
    }

    // Haxe: AiHelper.gotoObj + isPickingupFood sticky food (AI-GOTO-FOOD)
    #[test]
    fn plan_goto_obj_receding_vs_proceed() {
        let tgt = LastGotoObj::new(20, 0, 31);
        // First visit: no last → Proceed
        match plan_goto_obj(None, -1.0, tgt, 0, 0) {
            GotoObjPlan::Proceed { dist_quad } => {
                assert!((dist_quad - 400.0).abs() < 0.01);
            }
            other => panic!("expected Proceed, got {other:?}"),
        }
        // Same target, distance increasing past 100 → AbortReceding
        let last = Some(LastGotoObj::new(20, 0, 31));
        match plan_goto_obj(last, 90.0, tgt, 0, 0) {
            GotoObjPlan::AbortReceding { dist_quad } => {
                assert!((dist_quad - 400.0).abs() < 0.01);
            }
            other => panic!("expected AbortReceding, got {other:?}"),
        }
        // Same target but closer → Proceed (not receding)
        match plan_goto_obj(last, 500.0, tgt, 10, 0) {
            GotoObjPlan::Proceed { dist_quad } => {
                assert!((dist_quad - 100.0).abs() < 0.01);
            }
            other => panic!("expected Proceed, got {other:?}"),
        }
    }

    #[test]
    fn sticky_food_resolve_and_fail_effects() {
        let s = StickyFoodTarget::new(3, 4, 31);
        assert!(sticky_food_still_valid(s, 31, 5));
        assert!(!sticky_food_still_valid(s, 0, 5));
        assert!(!sticky_food_still_valid(s, 31, 0));
        // Keep sticky when tile still edible
        let kept = resolve_sticky_food(Some(s), 31, 5, Some(StickyFoodTarget::new(9, 9, 99)));
        assert_eq!(kept, Some(s));
        // Replace when sticky dead
        let next = StickyFoodTarget::new(1, 2, 40);
        let adopted = resolve_sticky_food(Some(s), 0, 0, Some(next));
        assert_eq!(adopted, Some(next));

        let e = food_goto_fail_effects(true);
        assert!(e.clear_food_target);
        assert!(e.increment_did_not_reach_food);
        assert!(e.reset_action_targets);
        assert!(!food_goto_fail_effects(false).increment_did_not_reach_food);

        let mut dnrf = 0.0;
        let mut sticky = Some(s);
        let mut last = Some(LastGotoObj::new(3, 4, 31));
        let mut last_d = 12.0;
        let mut targets = AiStickyBlockTargets::default();
        targets.set_food(BlockTargetClaim::simple(3, 4, 31));
        apply_food_goto_fail(
            &mut dnrf,
            &mut sticky,
            &mut last,
            &mut last_d,
            Some(&mut targets),
        );
        assert!((dnrf - 1.0).abs() < 0.01);
        assert!(sticky.is_none());
        assert!(last.is_none());
        assert!((last_d - -1.0).abs() < 0.01);
        assert!(targets.food_target.is_none());
        assert_eq!(food_pickup_success_reset_did_not_reach(), 0.0);
    }

    #[test]
    fn add_target_filters_multi_use_animal_dont_block() {
        let mut map = HashMap::new();
        // multi-use
        let multi = BlockTargetClaim {
            x: 1,
            y: 0,
            parent_id: 400,
            number_of_uses: 3,
            is_animal: false,
            held_new_target_id: None,
        };
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&multi)),
            AddTargetBlockResult::FilteredStop
        );
        assert!(map.is_empty());
        assert!(!would_block_target_by_ai(&multi));

        // animal
        let animal = BlockTargetClaim {
            x: 2,
            y: 0,
            parent_id: 518,
            number_of_uses: 1,
            is_animal: true,
            held_new_target_id: None,
        };
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&animal)),
            AddTargetBlockResult::FilteredStop
        );
        assert!(map.is_empty());

        // DontBlockByAi fire
        let fire = BlockTargetClaim::simple(3, 0, 82);
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&fire)),
            AddTargetBlockResult::FilteredStop
        );
        assert!(map.is_empty());

        // normal claim adds
        let ok = BlockTargetClaim::simple(4, 0, 33);
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&ok)),
            AddTargetBlockResult::AddedStop
        );
        assert!(map.contains_key(&(4, 0)));
    }

    #[test]
    fn add_target_held_same_new_target_continues() {
        let mut map = HashMap::new();
        // Hot Adobe Oven style: held + target leaves same parent
        let kiln = BlockTargetClaim {
            x: 10,
            y: 10,
            parent_id: 250,
            number_of_uses: 1,
            is_animal: false,
            held_new_target_id: Some(250),
        };
        // 250 is also DontBlockByAi — that hits first
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&kiln)),
            AddTargetBlockResult::FilteredStop
        );

        // Non-DontBlock parent with same newTarget
        let bowl_use = BlockTargetClaim {
            x: 11,
            y: 10,
            parent_id: 999,
            number_of_uses: 1,
            is_animal: false,
            held_new_target_id: Some(999),
        };
        assert_eq!(
            try_add_target_blocked_by_ai(&mut map, Some(&bowl_use)),
            AddTargetBlockResult::SameTargetContinue
        );
        assert!(map.is_empty());
        assert!(!would_block_target_by_ai(&bowl_use));
    }

    #[test]
    fn calculate_blocked_by_ai_multi_agent_food_drop_use() {
        let agents = vec![
            AiAgentBlockSource {
                age: 20.0,
                food_target: Some(BlockTargetClaim::simple(1, 0, 31)),
                ..Default::default()
            },
            AiAgentBlockSource {
                age: 15.0,
                drop_target: Some(BlockTargetClaim::simple(2, 0, 32)),
                ..Default::default()
            },
            AiAgentBlockSource {
                age: 18.0,
                use_target: Some(BlockTargetClaim::simple(3, 0, 33)),
                ..Default::default()
            },
            // age < 3 skipped
            AiAgentBlockSource {
                age: 2.0,
                food_target: Some(BlockTargetClaim::simple(4, 0, 34)),
                ..Default::default()
            },
            // wounded skipped
            AiAgentBlockSource {
                age: 20.0,
                is_wounded: true,
                food_target: Some(BlockTargetClaim::simple(5, 0, 35)),
                ..Default::default()
            },
            // DontBlock fire filtered
            AiAgentBlockSource {
                age: 20.0,
                use_target: Some(BlockTargetClaim::simple(6, 0, 82)),
                ..Default::default()
            },
        ];
        let map = calculate_blocked_by_ai(&[], &agents);
        assert!(map.contains_key(&(1, 0)));
        assert!(map.contains_key(&(2, 0)));
        assert!(map.contains_key(&(3, 0)));
        assert!(!map.contains_key(&(4, 0)));
        assert!(!map.contains_key(&(5, 0)));
        assert!(!map.contains_key(&(6, 0)));
    }

    #[test]
    fn calculate_blocked_by_ai_agent_chain_stops_at_first_add() {
        // food added → drop/use not considered
        let agent = AiAgentBlockSource {
            age: 20.0,
            food_target: Some(BlockTargetClaim::simple(1, 0, 31)),
            drop_target: Some(BlockTargetClaim::simple(2, 0, 32)),
            use_target: Some(BlockTargetClaim::simple(3, 0, 33)),
            ..Default::default()
        };
        let map = calculate_blocked_by_ai(&[], &[agent]);
        assert!(map.contains_key(&(1, 0)));
        assert!(!map.contains_key(&(2, 0)));
        assert!(!map.contains_key(&(3, 0)));
    }

    #[test]
    fn calculate_blocked_by_ai_human_age_gate() {
        let humans = vec![
            HumanBlockClaim {
                claim: BlockTargetClaim::simple(7, 7, 100),
                age_secs: 10.0,
            },
            HumanBlockClaim {
                claim: BlockTargetClaim::simple(8, 8, 101),
                age_secs: 25.0, // > 20 skip
            },
        ];
        let map = calculate_blocked_by_ai(&humans, &[]);
        assert!(map.contains_key(&(7, 7)));
        assert!(!map.contains_key(&(8, 8)));
    }

    #[test]
    fn apply_calculate_replaces_dest() {
        let mut dest = HashMap::new();
        add_blocked_by_ai(&mut dest, 99, 99, 5.0);
        let agents = [AiAgentBlockSource {
            age: 20.0,
            food_target: Some(BlockTargetClaim::simple(1, 1, 40)),
            ..Default::default()
        }];
        apply_calculate_blocked_by_ai(&mut dest, &[], &agents);
        assert!(!dest.contains_key(&(99, 99)));
        assert!(dest.contains_key(&(1, 1)));
    }

    #[test]
    fn sticky_to_agent_respects_player_block_age() {
        let mut s = AiStickyBlockTargets::default();
        s.set_player_block(BlockTargetClaim::simple(5, 5, 100), 10.0);
        s.set_food(BlockTargetClaim::simple(1, 0, 31));
        // age 5s < 20 → player block included; ai_block mirrors for chain-stop
        let a = s.to_agent_block_source(20.0, false, false, 15.0);
        assert!(a.player_block_target.is_some());
        assert!(a.ai_block_target.is_some());
        assert!(a.food_target.is_some());
        // age 25s > 20 → age-gated player_block dropped; Haxe myPlayer.block still set
        let a2 = s.to_agent_block_source(20.0, false, false, 35.0);
        assert!(a2.player_block_target.is_none());
        assert!(a2.ai_block_target.is_some(), "no second age gate on myPlayer.block");
        assert!(a2.food_target.is_some());
        // chain-stop: food not both claimed when player_block present
        let map = calculate_blocked_by_ai(&[], &[a2]);
        assert!(map.contains_key(&(5, 5)));
        assert!(
            !map.contains_key(&(1, 0)),
            "ai_block_target early-stops food chain"
        );
    }

    #[test]
    fn block_claim_number_of_uses_no_template_fallback() {
        assert_eq!(block_claim_number_of_uses(3), 3);
        assert_eq!(block_claim_number_of_uses(0), 1);
        assert_eq!(block_claim_number_of_uses(-1), 1);
    }

    #[test]
    fn player_block_chain_stops_before_food_on_rebuild() {
        let mut sticky = AiStickyBlockTargets::default();
        sticky.set_player_block(BlockTargetClaim::simple(4, 4, 50), 0.0);
        sticky.set_food(BlockTargetClaim::simple(8, 8, 31));
        sticky.set_drop(BlockTargetClaim::simple(9, 9, 32));
        let bodies = [StickyBlockBodyRow {
            is_ai: true,
            age: 20.0,
            is_wounded: false,
            deleted: false,
            sticky,
        }];
        let map = rebuild_blocked_by_ai_from_sticky(1.0, &bodies);
        assert!(map.contains_key(&(4, 4)));
        assert!(!map.contains_key(&(8, 8)));
        assert!(!map.contains_key(&(9, 9)));
    }

    #[test]
    fn clear_action_targets_then_rebuild_empties_when_no_claims() {
        let mut sticky = AiStickyBlockTargets::default();
        sticky.set_food(BlockTargetClaim::simple(1, 1, 31));
        sticky.clear_action_targets();
        let bodies = [StickyBlockBodyRow {
            is_ai: true,
            age: 20.0,
            is_wounded: false,
            deleted: false,
            sticky,
        }];
        let map = rebuild_blocked_by_ai_from_sticky(1.0, &bodies);
        assert!(map.is_empty());
    }

    #[test]
    fn wounded_agent_skipped_on_rebuild() {
        let mut sticky = AiStickyBlockTargets::default();
        sticky.set_food(BlockTargetClaim::simple(2, 2, 31));
        let bodies = [StickyBlockBodyRow {
            is_ai: true,
            age: 20.0,
            is_wounded: true,
            deleted: false,
            sticky,
        }];
        let map = rebuild_blocked_by_ai_from_sticky(1.0, &bodies);
        assert!(!map.contains_key(&(2, 2)));
    }

    #[test]
    fn rebuild_from_sticky_ai_food_and_human_block() {
        let mut ai = AiStickyBlockTargets::default();
        ai.set_food(BlockTargetClaim::simple(3, 0, 31));
        let mut human = AiStickyBlockTargets::default();
        human.set_player_block(BlockTargetClaim::simple(7, 7, 200), 0.0);
        let bodies = [
            StickyBlockBodyRow {
                is_ai: true,
                age: 20.0,
                is_wounded: false,
                deleted: false,
                sticky: ai,
            },
            StickyBlockBodyRow {
                is_ai: false,
                age: 30.0,
                is_wounded: false,
                deleted: false,
                sticky: human,
            },
            // young AI skipped
            StickyBlockBodyRow {
                is_ai: true,
                age: 1.0,
                is_wounded: false,
                deleted: false,
                sticky: {
                    let mut s = AiStickyBlockTargets::default();
                    s.set_use(BlockTargetClaim::simple(9, 9, 50));
                    s
                },
            },
        ];
        let map = rebuild_blocked_by_ai_from_sticky(5.0, &bodies);
        assert!(map.contains_key(&(3, 0)), "AI food claim");
        assert!(map.contains_key(&(7, 7)), "human block claim");
        assert!(!map.contains_key(&(9, 9)), "young AI skipped");
    }

    #[test]
    fn should_set_block_target_for_ai_gates() {
        // human + normal object
        assert!(should_set_block_target_for_ai(
            true, 0, 100, false, false, false, 0, false
        ));
        // AI without hammer
        assert!(!should_set_block_target_for_ai(
            false, 0, 100, false, false, false, 0, false
        ));
        // AI with smith hammer
        assert!(should_set_block_target_for_ai(
            false, SMITHING_HAMMER_BLOCK_ID, 100, false, false, false, 0, false
        ));
        // permanent blocks human unless hammer
        assert!(!should_set_block_target_for_ai(
            true, 0, 100, true, false, false, 0, false
        ));
        assert!(should_set_block_target_for_ai(
            true, SMITHING_HAMMER_BLOCK_ID, 100, true, false, false, 0, false
        ));
        // food / weapon / animal / clothing / parent0
        assert!(!should_set_block_target_for_ai(
            true, 0, 100, false, false, false, 5, false
        ));
        assert!(!should_set_block_target_for_ai(
            true, 0, 100, false, true, false, 0, false
        ));
        assert!(!should_set_block_target_for_ai(
            true, 0, 100, false, false, true, 0, false
        ));
        assert!(!should_set_block_target_for_ai(
            true, 0, 100, false, false, false, 0, true
        ));
        assert!(!should_set_block_target_for_ai(
            true, 0, 0, false, false, false, 0, false
        ));
    }

    #[test]
    fn apply_rebuild_from_sticky_wipes_old() {
        let mut dest = HashMap::new();
        add_blocked_by_ai(&mut dest, 99, 99, 5.0);
        let mut sticky = AiStickyBlockTargets::default();
        sticky.set_drop(BlockTargetClaim::simple(2, 2, 32));
        let bodies = [StickyBlockBodyRow {
            is_ai: true,
            age: 18.0,
            is_wounded: false,
            deleted: false,
            sticky,
        }];
        apply_rebuild_blocked_by_ai_from_sticky(&mut dest, 1.0, &bodies);
        assert!(!dest.contains_key(&(99, 99)));
        assert!(dest.contains_key(&(2, 2)));
    }
}
