//! Top-down craft transition search filters (**AI-CRAFT-TOPDOWN** / craft_topdown).
//!
//! Ports Haxe `AiBase.searchBestTransitionTopDown` / `DoTransitionSearch` skip gates
//! and object-scan filters (`isObjectNotReachable`, hostile path, `ignoreFullPiles`,
//! non-empty containers) used by `searchBestObjectForCrafting`.
//!
//! Nested under [`super`] (`craft_item`) via `#[path]`, or usable as crate module.
//!
//! Haxe anchors:
//! - `AiBase.searchBestObjectForCrafting` ~7132–7186
//! - `AiBase.searchBestTransitionTopDown` ~7696–7799
//! - `AiBase.DoTransitionSearch` ~7801–8039
//! - `AiBase.addObjectsForCrafting` ~7251–7361 (unreachable / container / full)
//! - `ServerSettings.AiIgnoreTimeTransitionsLongerThen` = 120
//! - Hardened Row 848 ↔ Stone Hoe 850 / Steel Hoe 857 + Fertile Soil 1138

use std::collections::{HashMap, HashSet};

use crate::craft_graph::ReverseCraftGraph;

use super::{
    closest_craft_obj_dual_center, craft_chebyshev, craft_have_set_ex, craft_obj_in_dual_center,
    CraftTransPair, CraftWorldObj, FERTILE_SOIL, AI_CRAFT_MIN_RADIUS, AI_MAX_SEARCH_INCREMENT,
    AI_MAX_SEARCH_RADIUS,
};

// ── Constants ───────────────────────────────────────────────────────────────

/// Haxe `ServerSettings.AiIgnoreTimeTransitionsLongerThen`.
pub const AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN: f32 = 120.0;

/// Haxe: min-count gate only when `searchRadius < 40`.
// Haxe: DoTransitionSearch ~7869
pub const AI_CRAFT_MIN_COUNT_RADIUS_CAP: i32 = 40;

/// Hardened Row (when present, hoe+soil transitions are ignored).
// Haxe: searchBestTransitionTopDown ~7721–7736
pub const HARDENED_ROW: i32 = 848;
/// Stone Hoe.
pub const STONE_HOE: i32 = 850;
/// Steel Hoe.
pub const STEEL_HOE: i32 = 857;

// ── Transition metadata (Haxe TransitionData craft-AI fields) ────────────────

/// Subset of Haxe `TransitionData` used by `DoTransitionSearch`.
// Haxe: TransitionData.aiShouldIgnore / reverseUseTarget / targetMinUseFraction / …
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftTransMeta {
    pub actor_id: i32,
    pub target_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub ai_should_ignore: bool,
    /// Haxe `autoDecaySeconds` (negative = hours).
    pub auto_decay_seconds: f32,
    pub reverse_use_target: bool,
    /// 1.0 = target must be full (`targetMinUseFraction`).
    pub target_min_use_fraction: f32,
    /// Haxe `ignoreIfMaxIsReachedObjectId` (−1 unset).
    pub ignore_if_max_is_reached_object_id: i32,
    /// Haxe `igmoreIfMinIsNotReachedObjectId` (typo in Haxe; −1 unset).
    pub ignore_if_min_is_not_reached_object_id: i32,
}

impl CraftTransMeta {
    /// Minimal meta for reverse-graph pair `(actor, target) → product`.
    pub fn pair(actor_id: i32, target_id: i32, new_actor_id: i32, new_target_id: i32) -> Self {
        Self {
            actor_id,
            target_id,
            new_actor_id,
            new_target_id,
            ai_should_ignore: false,
            auto_decay_seconds: 0.0,
            reverse_use_target: false,
            target_min_use_fraction: 0.0,
            ignore_if_max_is_reached_object_id: -1,
            ignore_if_min_is_not_reached_object_id: -1,
        }
    }

    pub fn with_ai_should_ignore(mut self, v: bool) -> Self {
        self.ai_should_ignore = v;
        self
    }

    pub fn with_auto_decay(mut self, secs: f32) -> Self {
        self.auto_decay_seconds = secs;
        self
    }

    pub fn with_reverse_use_target(mut self, v: bool) -> Self {
        self.reverse_use_target = v;
        self
    }

    pub fn with_target_min_use_fraction(mut self, f: f32) -> Self {
        self.target_min_use_fraction = f;
        self
    }

    pub fn with_ignore_if_max(mut self, id: i32) -> Self {
        self.ignore_if_max_is_reached_object_id = id;
        self
    }

    pub fn with_ignore_if_min(mut self, id: i32) -> Self {
        self.ignore_if_min_is_not_reached_object_id = id;
        self
    }
}

/// Build [`CraftTransMeta`] map from content transitions + ai_should_ignore side-tables.
///
/// Keys are `(actor_id, target_id)`. Prefer primary transitions; last-use fills gaps.
/// Primary `ai_should_ignore` marks every matching edge. Last-use-only ignores
/// (pond LT) mark only when the edge is filled from `transitions_last_use` or
/// when no primary row exists.
// Haxe: TransitionData fields + ServerSettings.PatchTransitions aiShouldIgnore (C-SS-AI-IGNORE)
pub fn craft_trans_meta_map_from_content(
    content: &ol_content::ContentDb,
) -> HashMap<(i32, i32), CraftTransMeta> {
    let mut map = HashMap::new();
    for t in content.transitions.values() {
        let mut m = CraftTransMeta::pair(
            t.actor_id,
            t.target_id,
            t.new_actor_id,
            t.new_target_id,
        )
        .with_auto_decay(t.auto_decay_seconds)
        .with_reverse_use_target(t.reverse_use_target)
        .with_target_min_use_fraction(t.target_min_use_fraction);
        // Primary table only — last-use-only pond keys stay craftable on primary.
        if content.ai_should_ignore.contains(&(t.actor_id, t.target_id)) {
            m = m.with_ai_should_ignore(true);
        }
        map.insert((t.actor_id, t.target_id), m);
    }
    for t in content.transitions_last_use.values() {
        let key = (t.actor_id, t.target_id);
        // Primary already owns this edge key — do not collapse last-use-only
        // ignore (pond LT) onto primary meta (Haxe LA/LT maps are separate).
        if map.contains_key(&key) {
            continue;
        }
        let last_use_ignore = content.ai_should_ignore.contains(&key)
            || content.ai_should_ignore_last_use.contains(&key);
        let mut m = CraftTransMeta::pair(
            t.actor_id,
            t.target_id,
            t.new_actor_id,
            t.new_target_id,
        )
        .with_auto_decay(t.auto_decay_seconds)
        .with_reverse_use_target(t.reverse_use_target)
        .with_target_min_use_fraction(t.target_min_use_fraction);
        if last_use_ignore {
            m = m.with_ai_should_ignore(true);
        }
        map.insert(key, m);
    }
    // Orphan primary ignore keys (no transition body) still force skip via meta.
    for &(a, t) in &content.ai_should_ignore {
        map.entry((a, t))
            .and_modify(|m| m.ai_should_ignore = true)
            .or_insert_with(|| CraftTransMeta::pair(a, t, 0, 0).with_ai_should_ignore(true));
    }
    // Last-use-only orphans only when no primary edge meta exists.
    for &(a, t) in &content.ai_should_ignore_last_use {
        if !map.contains_key(&(a, t)) {
            map.insert(
                (a, t),
                CraftTransMeta::pair(a, t, 0, 0).with_ai_should_ignore(true),
            );
        }
    }
    map
}

/// Why `DoTransitionSearch` skipped a transition (tests / debug).
// Haxe: DoTransitionSearch continue reasons ~7809–7878
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransSkipReason {
    ActorIsWantedOrProduct,
    TargetIsWantedOrProduct,
    TargetIsProductPile,
    AiShouldIgnore,
    UndoesLastTransition,
    TargetIsPlayerMinusOne,
    TimeTransitionTooLong,
    ReverseUseTargetFull,
    TargetNotFull,
    MaxCountReached,
    MinCountNotReached,
}

// ── Object index for max/min / full-use gates ────────────────────────────────

/// Counts + closest multi-use state for max/min and reverse-use filters.
// Haxe: transitionsByObjectId[id].count / closestObject.numberOfUses
#[derive(Debug, Clone, Default)]
pub struct CraftObjectIndex {
    /// parent_id → object count (piles may add numberOfUses in Haxe).
    pub counts: HashMap<i32, i32>,
    /// parent_id → (closest num_uses, ObjectData.numUses max).
    pub closest_uses: HashMap<i32, (i32, i32)>,
    /// parent_id → `aiCraftMax` (default 1 when missing).
    pub ai_craft_max: HashMap<i32, i32>,
    /// parent_id → `aiCraftMin` (default 0 when missing).
    pub ai_craft_min: HashMap<i32, i32>,
}

impl CraftObjectIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_count(&mut self, id: i32, count: i32) {
        self.counts.insert(id, count);
    }

    pub fn set_closest_uses(&mut self, id: i32, num_uses: i32, max_uses: i32) {
        self.closest_uses.insert(id, (num_uses, max_uses));
    }

    pub fn set_ai_craft_max(&mut self, id: i32, max: i32) {
        self.ai_craft_max.insert(id, max);
    }

    pub fn set_ai_craft_min(&mut self, id: i32, min: i32) {
        self.ai_craft_min.insert(id, min);
    }

    /// Build index from a world snapshot (count + closest uses per parent_id).
    // Haxe: addObjectsForCrafting doCountObjects
    pub fn from_objs(objs: &[CraftWorldObj], blocked: Option<&HashSet<(i32, i32)>>) -> Self {
        let mut idx = Self::new();
        for o in objs {
            if o.parent_id <= 0 {
                continue;
            }
            if let Some(b) = blocked {
                if b.contains(&(o.x, o.y)) {
                    continue;
                }
            }
            // Haxe piles may add numberOfUses; count all non-blocked ground objs.
            *idx.counts.entry(o.parent_id).or_insert(0) += 1.max(o.num_uses);
            let max_u = o.max_uses;
            idx.closest_uses
                .entry(o.parent_id)
                .and_modify(|(nu, mu)| {
                    // Prefer highest num_uses among closest candidates (Haxe closestObject).
                    if o.num_uses > *nu {
                        *nu = o.num_uses;
                    }
                    if max_u > *mu {
                        *mu = max_u;
                    }
                })
                .or_insert((o.num_uses, max_u));
        }
        idx
    }

    /// Merge ObjectData-style `aiCraftMax` / `aiCraftMin` maps (caller supplies content).
    // Haxe: ObjectData.aiCraftMax / aiCraftMin
    pub fn with_ai_craft_limits(
        mut self,
        max_map: &HashMap<i32, i32>,
        min_map: &HashMap<i32, i32>,
    ) -> Self {
        for (&id, &m) in max_map {
            self.ai_craft_max.insert(id, m);
        }
        for (&id, &m) in min_map {
            self.ai_craft_min.insert(id, m);
        }
        self
    }
}

// ── Time transition helpers ─────────────────────────────────────────────────

/// Absolute seconds from Haxe `autoDecaySeconds` (negative hours → ×3600).
// Haxe: TransitionData.calculateTimeToChange base before random
pub fn auto_decay_time_base_seconds(auto_decay_seconds: f32) -> f32 {
    if auto_decay_seconds < 0.0 {
        (-3600.0) * auto_decay_seconds
    } else {
        auto_decay_seconds.max(0.0)
    }
}

/// True when time transition should be ignored by AI craft search.
// Haxe: calculateTimeToChange() > AiIgnoreTimeTransitionsLongerThen
///
/// Haxe multiplies by random ∈ [0.5, 1.5]; we use the deterministic base (mean).
pub fn time_transition_exceeds_ai_ignore(auto_decay_seconds: f32) -> bool {
    auto_decay_time_base_seconds(auto_decay_seconds) > AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN
}

// ── Hardened-row hoe+soil dynamic ignore ────────────────────────────────────

/// True when hoe+soil should be treated as `aiShouldIgnore` due to Hardened Row presence.
// Haxe: searchBestTransitionTopDown ~7721–7736 (850/857 + 1138)
pub fn hardened_row_forces_hoe_soil_ignore(
    exists_hardened_row: bool,
    actor_id: i32,
    target_id: i32,
) -> bool {
    if !exists_hardened_row || target_id != FERTILE_SOIL {
        return false;
    }
    actor_id == STONE_HOE || actor_id == STEEL_HOE
}

/// Effective `aiShouldIgnore` after hardened-row patch.
pub fn effective_ai_should_ignore(meta: &CraftTransMeta, exists_hardened_row: bool) -> bool {
    if hardened_row_forces_hoe_soil_ignore(exists_hardened_row, meta.actor_id, meta.target_id)
    {
        return true;
    }
    // When no hardened row, hoe+soil ignore is cleared in Haxe.
    if !exists_hardened_row
        && meta.target_id == FERTILE_SOIL
        && (meta.actor_id == STONE_HOE || meta.actor_id == STEEL_HOE)
    {
        return false;
    }
    meta.ai_should_ignore
}

// ── DoTransitionSearch skip ─────────────────────────────────────────────────

/// Return skip reason for one transition (None = allowed).
// Haxe: DoTransitionSearch ~7808–7878
pub fn do_transition_search_skip_reason(
    trans: &CraftTransMeta,
    wanted_id: i32,
    obj_to_craft_id: i32,
    obj_to_craft_pile_id: i32,
    last_actor_id: i32,
    last_target_id: i32,
    search_radius: i32,
    exists_hardened_row: bool,
    index: Option<&CraftObjectIndex>,
) -> Option<TransSkipReason> {
    if trans.actor_id == wanted_id || trans.actor_id == obj_to_craft_id {
        return Some(TransSkipReason::ActorIsWantedOrProduct);
    }
    if trans.target_id == wanted_id || trans.target_id == obj_to_craft_id {
        return Some(TransSkipReason::TargetIsWantedOrProduct);
    }
    if obj_to_craft_pile_id > 0 && trans.target_id == obj_to_craft_pile_id {
        return Some(TransSkipReason::TargetIsProductPile);
    }
    if effective_ai_should_ignore(trans, exists_hardened_row) {
        return Some(TransSkipReason::AiShouldIgnore);
    }
    if last_actor_id >= 0
        && last_target_id >= 0
        && trans.new_actor_id == last_actor_id
        && trans.new_target_id == last_target_id
    {
        return Some(TransSkipReason::UndoesLastTransition);
    }
    if trans.target_id == -1 {
        return Some(TransSkipReason::TargetIsPlayerMinusOne);
    }
    if time_transition_exceeds_ai_ignore(trans.auto_decay_seconds) {
        return Some(TransSkipReason::TimeTransitionTooLong);
    }

    if let Some(idx) = index {
        if trans.reverse_use_target {
            let nid = trans.new_target_id;
            if let Some(&(num_uses, max_uses)) = idx.closest_uses.get(&nid) {
                if max_uses > 1 && num_uses >= max_uses {
                    return Some(TransSkipReason::ReverseUseTargetFull);
                }
            }
        }

        if (trans.target_min_use_fraction - 1.0).abs() < 1e-5 && !trans.reverse_use_target {
            let tid = trans.target_id;
            if let Some(&(num_uses, max_uses)) = idx.closest_uses.get(&tid) {
                if max_uses > 1 && num_uses < max_uses {
                    return Some(TransSkipReason::TargetNotFull);
                }
            }
        }

        if trans.ignore_if_max_is_reached_object_id > 0 {
            let mid = trans.ignore_if_max_is_reached_object_id;
            let count = idx.counts.get(&mid).copied().unwrap_or(0);
            let max = idx.ai_craft_max.get(&mid).copied().unwrap_or(1);
            if count >= max {
                return Some(TransSkipReason::MaxCountReached);
            }
        }

        if trans.ignore_if_min_is_not_reached_object_id > 0 {
            // Haxe: if (searchRadius >= 40) continue; — skip min-gated transitions at large radius
            if search_radius >= AI_CRAFT_MIN_COUNT_RADIUS_CAP {
                return Some(TransSkipReason::MinCountNotReached);
            }
            let mid = trans.ignore_if_min_is_not_reached_object_id;
            let count = idx.counts.get(&mid).copied().unwrap_or(0);
            let min = idx.ai_craft_min.get(&mid).copied().unwrap_or(0);
            if count <= min {
                return Some(TransSkipReason::MinCountNotReached);
            }
        }
    } else if trans.ignore_if_min_is_not_reached_object_id > 0
        && search_radius >= AI_CRAFT_MIN_COUNT_RADIUS_CAP
    {
        return Some(TransSkipReason::MinCountNotReached);
    }

    None
}

/// True when transition should be skipped by top-down craft search.
pub fn should_skip_transition_top_down(
    trans: &CraftTransMeta,
    wanted_id: i32,
    obj_to_craft_id: i32,
    obj_to_craft_pile_id: i32,
    last_actor_id: i32,
    last_target_id: i32,
    search_radius: i32,
    exists_hardened_row: bool,
    index: Option<&CraftObjectIndex>,
) -> bool {
    do_transition_search_skip_reason(
        trans,
        wanted_id,
        obj_to_craft_id,
        obj_to_craft_pile_id,
        last_actor_id,
        last_target_id,
        search_radius,
        exists_hardened_row,
        index,
    )
    .is_some()
}

// ── Object scan filters (hostile / unreachable / full piles) ────────────────

/// Filters applied when picking closest craft objects.
// Haxe: isObjectNotReachable / isObjectWithHostilePath / ignoreFullPiles / container
#[derive(Debug, Clone, Copy, Default)]
pub struct CraftScanFilters<'a> {
    /// Tiles blocked by notReachable / hostile / blockedByAI.
    pub blocked: Option<&'a HashSet<(i32, i32)>>,
    /// Tiles with full multi-use objects (ignoreFullPiles).
    pub full_pile_tiles: Option<&'a HashSet<(i32, i32)>>,
    /// Tiles with non-empty containers.
    pub nonempty_container_tiles: Option<&'a HashSet<(i32, i32)>>,
    /// When true, skip tiles in `full_pile_tiles`.
    pub ignore_full_piles: bool,
    /// When true, skip tiles in `nonempty_container_tiles`.
    pub skip_nonempty_containers: bool,
}

impl<'a> CraftScanFilters<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_blocked(mut self, blocked: &'a HashSet<(i32, i32)>) -> Self {
        self.blocked = Some(blocked);
        self
    }

    pub fn with_full_piles(mut self, tiles: &'a HashSet<(i32, i32)>) -> Self {
        self.full_pile_tiles = Some(tiles);
        self.ignore_full_piles = true;
        self
    }

    pub fn with_nonempty_containers(mut self, tiles: &'a HashSet<(i32, i32)>) -> Self {
        self.nonempty_container_tiles = Some(tiles);
        self.skip_nonempty_containers = true;
        self
    }

    pub fn with_ignore_full_piles(mut self, v: bool) -> Self {
        self.ignore_full_piles = v;
        self
    }

    pub fn with_skip_nonempty_containers(mut self, v: bool) -> Self {
        self.skip_nonempty_containers = v;
        self
    }
}

/// True when object passes craft scan filters.
// Haxe: addObjectsForCrafting continues
pub fn craft_obj_passes_scan_filters(o: &CraftWorldObj, filters: &CraftScanFilters<'_>) -> bool {
    if let Some(b) = filters.blocked {
        if b.contains(&(o.x, o.y)) {
            return false;
        }
    }
    if filters.skip_nonempty_containers {
        if let Some(c) = filters.nonempty_container_tiles {
            if c.contains(&(o.x, o.y)) {
                return false;
            }
        }
    }
    if filters.ignore_full_piles {
        if let Some(f) = filters.full_pile_tiles {
            if f.contains(&(o.x, o.y)) {
                return false;
            }
        }
    }
    true
}

/// Closest matching parent_id with scan filters.
// Haxe: GetClosestObject* + isObjectNotReachable / ignoreFullPiles
pub fn closest_craft_obj_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    exclude: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
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
        if !craft_obj_passes_scan_filters(o, filters) {
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

// ── Top-down search options ─────────────────────────────────────────────────

/// Options for filtered top-down craft pair search.
// Haxe: itemToCraft lastActor/Target + transitionsByObjectId + path maps
#[derive(Debug, Clone, Copy)]
pub struct CraftTopDownOpts<'a> {
    pub last_actor_id: i32,
    pub last_target_id: i32,
    pub obj_to_craft_pile_id: i32,
    pub exists_hardened_row: bool,
    pub scan: CraftScanFilters<'a>,
    pub index: Option<&'a CraftObjectIndex>,
    /// Optional (actor, target) → [`CraftTransMeta`] for reverse-graph edges.
    /// When present, reverseUse / time / aiShouldIgnore / max-min apply on real edges.
    // Haxe: TransitionImporter transitions with TransitionData fields
    pub meta_by_edge: Option<&'a HashMap<(i32, i32), CraftTransMeta>>,
    /// Haxe `itemToCraft.searchCurrentPosition` — dual home+player scan when true.
    // Haxe: IntemToCraft.searchCurrentPosition (AI-CRAFT-DUAL)
    pub search_current_position: bool,
}

impl<'a> Default for CraftTopDownOpts<'a> {
    fn default() -> Self {
        Self {
            last_actor_id: -1,
            last_target_id: -1,
            obj_to_craft_pile_id: -1,
            exists_hardened_row: false,
            scan: CraftScanFilters::default(),
            index: None,
            meta_by_edge: None,
            search_current_position: true,
        }
    }
}

impl<'a> CraftTopDownOpts<'a> {
    pub fn with_last(mut self, actor: i32, target: i32) -> Self {
        self.last_actor_id = actor;
        self.last_target_id = target;
        self
    }

    pub fn with_pile(mut self, pile_id: i32) -> Self {
        self.obj_to_craft_pile_id = pile_id;
        self
    }

    pub fn with_hardened_row(mut self, exists: bool) -> Self {
        self.exists_hardened_row = exists;
        self
    }

    pub fn with_scan(mut self, scan: CraftScanFilters<'a>) -> Self {
        self.scan = scan;
        self
    }

    pub fn with_index(mut self, index: &'a CraftObjectIndex) -> Self {
        self.index = Some(index);
        self
    }

    /// Supply TransitionData-like meta keyed by reverse-graph (actor, target).
    // Haxe: TransitionData on edges from TransitionImporter
    pub fn with_meta_map(mut self, map: &'a HashMap<(i32, i32), CraftTransMeta>) -> Self {
        self.meta_by_edge = Some(map);
        self
    }

    /// Dual-center: when true, objects near player count even if far from home.
    // Haxe: itemToCraft.searchCurrentPosition (AI-CRAFT-DUAL)
    pub fn with_search_current(mut self, search_current: bool) -> Self {
        self.search_current_position = search_current;
        self
    }

    /// Look up meta for an edge, if provided.
    pub fn meta_for(&self, actor_id: i32, target_id: i32) -> Option<&'a CraftTransMeta> {
        self.meta_by_edge
            .and_then(|m| m.get(&(actor_id, target_id)))
    }
}

/// True when reverse-graph edge (actor, target) producing product should be skipped.
pub fn should_skip_craft_edge(
    actor_id: i32,
    target_id: i32,
    product_id: i32,
    search_radius: i32,
    opts: &CraftTopDownOpts<'_>,
    meta: Option<&CraftTransMeta>,
) -> bool {
    let owned;
    let m = if let Some(meta) = meta {
        meta
    } else {
        owned = CraftTransMeta::pair(actor_id, target_id, product_id, 0);
        &owned
    };
    should_skip_transition_top_down(
        m,
        product_id,
        product_id,
        opts.obj_to_craft_pile_id,
        opts.last_actor_id,
        opts.last_target_id,
        search_radius,
        opts.exists_hardened_row,
        opts.index,
    )
}

// ── Filtered resolve / search ───────────────────────────────────────────────

/// Closest `parent_id` in dual-center radius, scan-filtered, ranked by **player** dist.
// Haxe: addObjectsForCrafting + path filters + player rank (AI-CRAFT-DUAL)
fn closest_craft_obj_dual_filtered(
    objs: &[CraftWorldObj],
    parent_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    search_current: bool,
    exclude: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
) -> Option<CraftWorldObj> {
    if parent_id <= 0 {
        return None;
    }
    // Fast path: no scan filters → pure dual-center helper.
    if filters.blocked.is_none()
        && filters.full_pile_tiles.is_none()
        && filters.nonempty_container_tiles.is_none()
    {
        return closest_craft_obj_dual_center(
            objs,
            parent_id,
            player_x,
            player_y,
            home,
            radius,
            search_current,
            exclude,
        );
    }
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
        if !craft_obj_in_dual_center(
            o.x,
            o.y,
            player_x,
            player_y,
            home,
            radius,
            search_current,
        ) {
            continue;
        }
        if !craft_obj_passes_scan_filters(o, filters) {
            continue;
        }
        let d = craft_chebyshev(player_x, player_y, o.x, o.y);
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

fn resolve_side_filtered(
    id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    exclude: Option<(i32, i32)>,
    filters: &CraftScanFilters<'_>,
    search_current: bool,
) -> (i32, i32, bool, bool, i32, bool) {
    if id == 0 {
        return (player_x, player_y, held_id == 0, false, -1, true);
    }
    if id == -1 {
        return (player_x, player_y, true, false, -1, true);
    }
    if id == -2 || id < 0 {
        return (0, 0, false, false, -1, false);
    }
    if held_id == id {
        return (player_x, player_y, true, false, -1, true);
    }

    // Dual-center + scan filters, player-ranked (AI-CRAFT-DUAL).
    // Haxe: addObjectsForCrafting home + optional current; closest by player
    if let Some(o) = closest_craft_obj_dual_filtered(
        objs,
        id,
        player_x,
        player_y,
        home,
        radius,
        search_current,
        exclude,
        filters,
    ) {
        return (o.x, o.y, false, false, -1, true);
    }

    if let Some(f) = pile_id_for {
        let pile = f(id);
        if pile > 0 {
            if let Some(o) = closest_craft_obj_dual_filtered(
                objs,
                pile,
                player_x,
                player_y,
                home,
                radius,
                search_current,
                exclude,
                filters,
            ) {
                return (o.x, o.y, false, true, pile, true);
            }
        }
    }
    (0, 0, false, false, -1, false)
}

fn resolve_pair_filtered(
    actor_id: i32,
    target_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    radius: i32,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    filters: &CraftScanFilters<'_>,
    search_current: bool,
) -> Option<CraftTransPair> {
    let (ax, ay, actor_held, actor_from_pile, pile_id, actor_ok) = resolve_side_filtered(
        actor_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        radius,
        pile_id_for,
        None,
        filters,
        search_current,
    );
    if !actor_ok {
        return None;
    }
    let exclude = if actor_id == target_id && actor_id > 0 && !actor_held {
        Some((ax, ay))
    } else {
        None
    };
    let (tx, ty, _t_held, _t_pile, _t_pile_id, target_ok) = resolve_side_filtered(
        target_id,
        objs,
        if actor_held { 0 } else { held_id },
        player_x,
        player_y,
        home,
        radius,
        None,
        exclude,
        filters,
        search_current,
    );
    if !target_ok {
        return None;
    }
    let dist_player_actor = if actor_held || actor_id == -1 {
        0
    } else {
        craft_chebyshev(player_x, player_y, ax, ay)
    };
    let distance = dist_player_actor + craft_chebyshev(ax, ay, tx, ty);
    let out_actor = if actor_id < 0 { actor_id } else { actor_id.max(0) };
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

fn find_best_pair_topdown(
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
    opts: &CraftTopDownOpts<'_>,
) -> Option<CraftTransPair> {
    if let Some(path) = graph.find_path_to_product(product_id, have, 8) {
        if path.is_empty() {
            return None;
        }
        for &(actor, target) in &path {
            // C-SS-AI-IGNORE: content PatchTransitions aiShouldIgnore
            if graph.ai_should_ignore_edge(actor, target) {
                continue;
            }
            let meta = opts.meta_for(actor, target);
            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {
                continue;
            }
            if let Some(pair) = resolve_pair_filtered(
                actor,
                target,
                objs,
                held_id,
                player_x,
                player_y,
                home,
                radius,
                pile_id_for,
                &opts.scan,
                opts.search_current_position,
            ) {
                return Some(pair);
            }
        }
    }

    if let Some(pairs) = graph.ingredients_for(product_id) {
        let mut best: Option<CraftTransPair> = None;
        for &(actor, target) in pairs {
            // C-SS-AI-IGNORE: content PatchTransitions aiShouldIgnore
            if graph.ai_should_ignore_edge(actor, target) {
                continue;
            }
            let meta = opts.meta_for(actor, target);
            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {
                continue;
            }
            if let Some(pair) = resolve_pair_filtered(
                actor,
                target,
                objs,
                held_id,
                player_x,
                player_y,
                home,
                radius,
                pile_id_for,
                &opts.scan,
                opts.search_current_position,
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

/// Filtered `searchBestObjectForCrafting` with DoTransitionSearch + scan gates.
// Haxe: searchBestObjectForCrafting + searchBestTransitionTopDown + DoTransitionSearch
pub fn search_best_object_for_crafting_topdown(
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
    if product_id <= 0 {
        return None;
    }
    let max_r = if max_search_radius < 1 {
        AI_MAX_SEARCH_RADIUS
    } else {
        max_search_radius
    };

    let mut opts_local = *opts;
    if !opts_local.exists_hardened_row {
        opts_local.exists_hardened_row = objs.iter().any(|o| {
            o.parent_id == HARDENED_ROW && craft_obj_passes_scan_filters(o, &opts.scan)
        });
    }

    let mut radius = 0;
    while radius < max_r {
        radius = (radius + AI_MAX_SEARCH_INCREMENT).max(AI_CRAFT_MIN_RADIUS);
        if radius > max_r {
            radius = max_r;
        }

        let mut have = HashSet::new();
        if held_id > 0 {
            have.insert(held_id);
        }
        have.insert(0);
        for o in objs {
            if o.parent_id <= 0 {
                continue;
            }
            if !craft_obj_passes_scan_filters(o, &opts_local.scan) {
                continue;
            }
            if craft_obj_in_dual_center(
                o.x,
                o.y,
                player_x,
                player_y,
                home,
                radius,
                opts_local.search_current_position,
            ) {
                have.insert(o.parent_id);
            }
        }
        // When no scan filters, fall back to craft_have_set_ex for parity.
        if opts_local.scan.blocked.is_none()
            && opts_local.scan.full_pile_tiles.is_none()
            && opts_local.scan.nonempty_container_tiles.is_none()
        {
            have = craft_have_set_ex(
                objs,
                held_id,
                player_x,
                player_y,
                home,
                radius,
                opts_local.search_current_position,
            );
        }

        if have.contains(&product_id) {
            return None;
        }

        if let Some(pair) = find_best_pair_topdown(
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
            &opts_local,
        ) {
            return Some(pair);
        }

        if radius >= max_r {
            break;
        }
    }
    None
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_meta(a: i32, t: i32, na: i32, nt: i32) -> CraftTransMeta {
        CraftTransMeta::pair(a, t, na, nt)
    }

    #[test]
    fn ai_should_ignore_skips() {
        let m = basic_meta(1, 2, 3, 0).with_ai_should_ignore(true);
        assert_eq!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, None),
            Some(TransSkipReason::AiShouldIgnore)
        );
    }

    #[test]
    fn undo_last_transition_skips() {
        let m = basic_meta(1, 2, 10, 20);
        assert_eq!(
            do_transition_search_skip_reason(&m, 99, 99, -1, 10, 20, 15, false, None),
            Some(TransSkipReason::UndoesLastTransition)
        );
        assert!(
            do_transition_search_skip_reason(&m, 99, 99, -1, 10, 21, 15, false, None).is_none()
        );
    }

    #[test]
    fn time_transition_long_skips() {
        let m = basic_meta(1, 2, 3, 0).with_auto_decay(200.0);
        assert_eq!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, None),
            Some(TransSkipReason::TimeTransitionTooLong)
        );
        let short = basic_meta(1, 2, 3, 0).with_auto_decay(30.0);
        assert!(
            do_transition_search_skip_reason(&short, 3, 3, -1, -1, -1, 15, false, None).is_none()
        );
        let hour = basic_meta(1, 2, 3, 0).with_auto_decay(-1.0);
        assert!(time_transition_exceeds_ai_ignore(hour.auto_decay_seconds));
    }

    #[test]
    fn target_minus_one_and_product_as_input_skip() {
        let m = basic_meta(1, -1, 3, 0);
        assert_eq!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, None),
            Some(TransSkipReason::TargetIsPlayerMinusOne)
        );
        // Actor is the product/wanted id → skip (DoTransitionSearch product-as-input).
        let m2 = basic_meta(5, 2, 8, 0);
        assert_eq!(
            do_transition_search_skip_reason(&m2, 5, 5, -1, -1, -1, 15, false, None),
            Some(TransSkipReason::ActorIsWantedOrProduct)
        );
        // Target is the product → skip.
        let m3 = basic_meta(1, 5, 8, 0);
        assert_eq!(
            do_transition_search_skip_reason(&m3, 5, 5, -1, -1, -1, 15, false, None),
            Some(TransSkipReason::TargetIsWantedOrProduct)
        );
    }

    #[test]
    fn reverse_use_target_full_skips() {
        let m = basic_meta(1, 2, 0, 50).with_reverse_use_target(true);
        let mut idx = CraftObjectIndex::new();
        idx.set_closest_uses(50, 5, 5);
        assert_eq!(
            do_transition_search_skip_reason(&m, 99, 99, -1, -1, -1, 15, false, Some(&idx)),
            Some(TransSkipReason::ReverseUseTargetFull)
        );
        idx.set_closest_uses(50, 3, 5);
        assert!(
            do_transition_search_skip_reason(&m, 99, 99, -1, -1, -1, 15, false, Some(&idx))
                .is_none()
        );
    }

    #[test]
    fn target_must_be_full_when_min_use_fraction_one() {
        let m = basic_meta(1, 40, 3, 0).with_target_min_use_fraction(1.0);
        let mut idx = CraftObjectIndex::new();
        idx.set_closest_uses(40, 2, 5);
        assert_eq!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, Some(&idx)),
            Some(TransSkipReason::TargetNotFull)
        );
        idx.set_closest_uses(40, 5, 5);
        assert!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, Some(&idx))
                .is_none()
        );
    }

    #[test]
    fn max_count_and_min_count_gates() {
        let m = basic_meta(1, 2, 3, 0).with_ignore_if_max(235);
        let mut idx = CraftObjectIndex::new();
        idx.set_count(235, 3);
        idx.set_ai_craft_max(235, 2);
        assert_eq!(
            do_transition_search_skip_reason(&m, 3, 3, -1, -1, -1, 15, false, Some(&idx)),
            Some(TransSkipReason::MaxCountReached)
        );

        let m2 = basic_meta(1, 2, 3, 0).with_ignore_if_min(152);
        assert_eq!(
            do_transition_search_skip_reason(&m2, 3, 3, -1, -1, -1, 40, false, Some(&idx)),
            Some(TransSkipReason::MinCountNotReached)
        );
        idx.set_count(152, 0);
        idx.set_ai_craft_min(152, 1);
        assert_eq!(
            do_transition_search_skip_reason(&m2, 3, 3, -1, -1, -1, 15, false, Some(&idx)),
            Some(TransSkipReason::MinCountNotReached)
        );
        idx.set_count(152, 5);
        assert!(
            do_transition_search_skip_reason(&m2, 3, 3, -1, -1, -1, 15, false, Some(&idx))
                .is_none()
        );
    }

    #[test]
    fn hardened_row_ignores_hoe_soil() {
        assert!(hardened_row_forces_hoe_soil_ignore(
            true,
            STONE_HOE,
            FERTILE_SOIL
        ));
        assert!(hardened_row_forces_hoe_soil_ignore(
            true,
            STEEL_HOE,
            FERTILE_SOIL
        ));
        assert!(!hardened_row_forces_hoe_soil_ignore(
            false,
            STONE_HOE,
            FERTILE_SOIL
        ));

        let m = basic_meta(STONE_HOE, FERTILE_SOIL, 848, 0);
        assert_eq!(
            do_transition_search_skip_reason(&m, 99, 99, -1, -1, -1, 15, true, None),
            Some(TransSkipReason::AiShouldIgnore)
        );
        let m2 = m.with_ai_should_ignore(true);
        assert!(
            do_transition_search_skip_reason(&m2, 99, 99, -1, -1, -1, 15, false, None).is_none()
        );
    }

    #[test]
    fn scan_filters_block_hostile_full_container() {
        let blocked: HashSet<(i32, i32)> = [(5, 5)].into_iter().collect();
        let full: HashSet<(i32, i32)> = [(2, 0)].into_iter().collect();
        let containers: HashSet<(i32, i32)> = [(3, 0)].into_iter().collect();
        let filters = CraftScanFilters::new()
            .with_blocked(&blocked)
            .with_full_piles(&full)
            .with_nonempty_containers(&containers);

        let near = CraftWorldObj::simple(10, 1, 1);
        let hostile = CraftWorldObj::simple(10, 5, 5);
        let full_o = CraftWorldObj::simple(10, 2, 0);
        let container = CraftWorldObj::simple(10, 3, 0).with_slots(4);

        assert!(craft_obj_passes_scan_filters(&near, &filters));
        assert!(!craft_obj_passes_scan_filters(&hostile, &filters));
        assert!(!craft_obj_passes_scan_filters(&full_o, &filters));
        assert!(!craft_obj_passes_scan_filters(&container, &filters));

        let objs = vec![hostile, near, full_o, container];
        let closest = closest_craft_obj_filtered(&objs, 10, 0, 0, 30, None, &filters);
        assert_eq!(closest.map(|o| (o.x, o.y)), Some((1, 1)));
    }

    #[test]
    fn topdown_search_skips_blocked_actor_tile() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let blocked: HashSet<(i32, i32)> = [(5, 0), (6, 0)].into_iter().collect();
        let scan = CraftScanFilters::new().with_blocked(&blocked);
        let opts = CraftTopDownOpts::default().with_scan(scan);
        let pair = search_best_object_for_crafting_topdown(
            3, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        assert!(pair.is_none());

        let opts2 = CraftTopDownOpts::default();
        let pair2 = search_best_object_for_crafting_topdown(
            3, &objs, 0, 0, 0, None, 60, &g, None, &opts2,
        );
        assert!(pair2.is_some());
    }

    #[test]
    fn product_pile_as_target_skipped() {
        let m = basic_meta(1, 99, 10, 0);
        assert_eq!(
            do_transition_search_skip_reason(&m, 10, 10, 99, -1, -1, 15, false, None),
            Some(TransSkipReason::TargetIsProductPile)
        );
    }

    #[test]
    fn ai_should_ignore_meta_skips_edge() {
        let ignored = CraftTransMeta::pair(1, 2, 3, 0).with_ai_should_ignore(true);
        let opts = CraftTopDownOpts::default();
        assert!(should_skip_craft_edge(1, 2, 3, 15, &opts, Some(&ignored)));
        let ok = CraftTransMeta::pair(4, 5, 3, 0);
        assert!(!should_skip_craft_edge(4, 5, 3, 15, &opts, Some(&ok)));
    }

    #[test]
    fn meta_map_skips_ai_ignore_on_graph_edge() {
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
        let opts = CraftTopDownOpts::default().with_meta_map(&meta_map);
        let pair = search_best_object_for_crafting_topdown(
            3, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        assert!(pair.is_some());
        let p = pair.unwrap();
        // First reverse edge ignored; second (4,5) used.
        assert_eq!((p.actor_id, p.target_id), (4, 5));
    }

    /// Primary pond fill is not ignored; last-use-only flag does not collapse.
    // Haxe: ~3531–3547 pond LT-only aiShouldIgnore (C-SS-AI-IGNORE gap-close)
    #[test]
    fn craft_trans_meta_map_pond_last_use_only() {
        let mut db = ol_content::ContentDb::default();
        // Primary water-fill row (craftable).
        db.transitions.insert(
            (235, 141),
            ol_content::Transition {
                actor_id: 235,
                target_id: 141,
                new_actor_id: 382,
                new_target_id: 141,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
            },
        );
        // Last-use only ignore (Haxe getTransition(..., false, true)).
        db.ai_should_ignore_last_use.insert((235, 141));
        // Primary explicit ignore for comparison edge.
        db.ai_should_ignore.insert((58, 235));

        let map = craft_trans_meta_map_from_content(&db);
        // Primary pond: NOT ignored (last_use_only must not collapse onto primary).
        assert!(!map.get(&(235, 141)).unwrap().ai_should_ignore);
        // Primary explicit ignore still applied.
        assert!(map.get(&(58, 235)).unwrap().ai_should_ignore);

        // If only last-use transition exists (no primary), last_use ignore applies.
        let mut db2 = ol_content::ContentDb::default();
        db2.transitions_last_use.insert(
            (209, 142),
            ol_content::Transition {
                actor_id: 209,
                target_id: 142,
                new_actor_id: 210,
                new_target_id: 142,
                last_use_actor: false,
                last_use_target: true,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
            },
        );
        db2.ai_should_ignore_last_use.insert((209, 142));
        let map2 = craft_trans_meta_map_from_content(&db2);
        assert!(map2.get(&(209, 142)).unwrap().ai_should_ignore);
    }

    // C-SS-AI-IGNORE: graph side-table skip (content PatchTransitions → ReverseCraftGraph)
    #[test]
    fn graph_ai_should_ignore_edge_skips_topdown_pair() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        g.insert(4, 5, 3, 0);
        g.mark_ai_should_ignore(1, 2);
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
            CraftWorldObj::simple(4, 3, 0),
            CraftWorldObj::simple(5, 4, 0),
        ];
        let opts = CraftTopDownOpts::default();
        let pair = search_best_object_for_crafting_topdown(
            3, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_eq!((p.actor_id, p.target_id), (4, 5));
        assert!(g.ai_should_ignore_edge(1, 2));
    }

    #[test]
    fn reverse_use_full_meta_skips_on_graph_edge() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 1, 0),
            CraftWorldObj::simple(2, 2, 0),
            // new_target 50 already full multi-use
            CraftWorldObj::simple(50, 0, 1)
                .with_uses(5)
                .with_max_uses(5),
        ];
        let mut meta_map = HashMap::new();
        meta_map.insert(
            (1, 2),
            CraftTransMeta::pair(1, 2, 3, 50).with_reverse_use_target(true),
        );
        let idx = CraftObjectIndex::from_objs(&objs, None);
        assert_eq!(idx.closest_uses.get(&50), Some(&(5, 5)));
        let opts = CraftTopDownOpts::default()
            .with_meta_map(&meta_map)
            .with_index(&idx);
        let pair = search_best_object_for_crafting_topdown(
            3, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        assert!(pair.is_none());
    }

    #[test]
    fn hardened_row_present_skips_hoe_soil_edge() {
        let mut g = ReverseCraftGraph::new();
        g.insert(STONE_HOE, FERTILE_SOIL, 900, 0);
        g.insert(1, 2, 900, 0);
        let objs = vec![
            CraftWorldObj::simple(HARDENED_ROW, 10, 10),
            CraftWorldObj::simple(STONE_HOE, 1, 0),
            CraftWorldObj::simple(FERTILE_SOIL, 2, 0),
            CraftWorldObj::simple(1, 3, 0),
            CraftWorldObj::simple(2, 4, 0),
        ];
        let opts = CraftTopDownOpts::default().with_hardened_row(true);
        let pair = search_best_object_for_crafting_topdown(
            900, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        assert!(pair.is_some());
        let p = pair.unwrap();
        assert_ne!((p.actor_id, p.target_id), (STONE_HOE, FERTILE_SOIL));
        assert_eq!((p.actor_id, p.target_id), (1, 2));
    }

    #[test]
    fn last_actor_target_undo_skipped_via_meta_new_ids() {
        // Edge produces new_actor/new_target matching last — undo skip.
        let mut g = ReverseCraftGraph::new();
        g.insert(10, 20, 99, 0);
        g.insert(30, 40, 99, 0);
        let objs = vec![
            CraftWorldObj::simple(10, 1, 0),
            CraftWorldObj::simple(20, 2, 0),
            CraftWorldObj::simple(30, 3, 0),
            CraftWorldObj::simple(40, 4, 0),
        ];
        let mut meta_map = HashMap::new();
        meta_map.insert((10, 20), CraftTransMeta::pair(10, 20, 7, 8));
        meta_map.insert((30, 40), CraftTransMeta::pair(30, 40, 99, 0));
        let opts = CraftTopDownOpts::default()
            .with_last(7, 8)
            .with_meta_map(&meta_map);
        let pair = search_best_object_for_crafting_topdown(
            99, &objs, 0, 0, 0, None, 60, &g, None, &opts,
        );
        let p = pair.expect("should pick non-undo edge");
        assert_eq!((p.actor_id, p.target_id), (30, 40));
    }

    #[test]
    fn default_opts_match_unfiltered_when_no_meta() {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        let objs = vec![
            CraftWorldObj::simple(1, 5, 0),
            CraftWorldObj::simple(2, 6, 0),
        ];
        let top = search_best_object_for_crafting_topdown(
            3,
            &objs,
            0,
            0,
            0,
            None,
            60,
            &g,
            None,
            &CraftTopDownOpts::default(),
        );
        assert!(top.is_some());
        let p = top.unwrap();
        assert_eq!((p.actor_id, p.target_id), (1, 2));
    }

    #[test]
    fn index_from_objs_loads_max_uses() {
        let objs = vec![
            CraftWorldObj::simple(50, 0, 0)
                .with_uses(3)
                .with_max_uses(5),
            CraftWorldObj::simple(50, 1, 0)
                .with_uses(4)
                .with_max_uses(5),
        ];
        let idx = CraftObjectIndex::from_objs(&objs, None);
        assert_eq!(idx.counts.get(&50), Some(&(7))); // 3+4
        assert_eq!(idx.closest_uses.get(&50), Some(&(4, 5)));
    }

    #[test]
    fn time_auto_decay_negative_hours() {
        assert!((auto_decay_time_base_seconds(-1.0) - 3600.0).abs() < 1e-3);
        assert!(time_transition_exceeds_ai_ignore(-1.0));
        assert!(!time_transition_exceeds_ai_ignore(60.0));
        assert!(time_transition_exceeds_ai_ignore(121.0));
    }
}
