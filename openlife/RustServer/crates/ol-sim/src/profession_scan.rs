//! Profession world-scan → shortCraft USE/DROP intent (**CRAFT-LIVE-TICK**).
//!
//! Closes the residual gap after **CRAFT-LIVE-IO**: profession pure SMs + intent
//! mappers exist, but live ticks needed a world → `FarmMapObj` / smith `MapObj` /
//! `BakeMapObj` scan that fills [`ShortCraftIntentCtx`] and enqueues USE/DROP.
//!
//! Haxe anchors:
//! - `AiBase.shortCraft` / `shortCraftOnTarget` / `shortCraftOnGround`
//! - `getClosestObjectById` / `AiHelper.CountCloseObjects`
//! - `getCloseWell` (well ids 663, 662)
//! - `doBasicFarming` / `doSmithing` / `doBaking` profession tick paths
//!
//! Residual (deferred): full dropHeldObject AI body, useHeldObjOnTarget staging
//! fields, GetOrCraftItem multi-step, Defer* baker farm tails, hungry-cost pair.
//! **PATH-REACH**: live `Player.ai_path_reach` + `SimState.blocked_by_ai` filter scan tiles.
//!
//! **NPC-SCAN-FULL / NPC-CRAFT-LADDER / NPC-SCAN-RESID**: farm/smith/baker/pottery/shepherd
//! → USE/DROP; peer wound/follow; clay-in-basket scan; path-filter pure helpers.
//! **AI-FIREFOOD-RUNG**: `ProfessionScanKind::FireFood` assigned/last makeFireFood(100).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).
//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).

use std::collections::{HashMap, HashSet};

use ol_content::ContentDb;
use ol_net::OutboundHub;
use ol_world::World;

use crate::baker_profession::{
    bake_action_short_craft_apply_ex, count_baker_peers_filtered, fill_bake_counts_from_map_ex,
    is_oven_id, pick_oven_near_home, try_decide_baker_from_rung, BakeAction, BakeMapObj,
    BakerPeerSnapshot, BakerProfessionRuntime, BakerTaskState, OvenCandidate,
    BAKING_CRAFT_SEARCH_RADIUS, OVEN_SEARCH_RADIUS,
};
use crate::farmer_profession::{
    apply_basic_farmer_weight_side_effect, basic_farmer_weight_from_runtime,
    expand_advanced_farming_or_clear, fill_farm_counts_from_map_with_floor, make_sharpie_food,
    short_craft_apply_resolved, try_decide_farm_from_rung, FarmAction, FarmCounts, FarmMapObj,
    FarmProfession, FarmProfessionRuntime, FarmTaskState, ShortCraftApply, ShortCraftInput,
    FARM_COUNT_RADIUS, FARM_HOME_RADIUS, FARM_SHORTCRAFT_RADIUS, SKEWER,
};
use crate::short_craft_intent::{
    apply_short_craft_live_intent, short_craft_apply_to_live_intent, smith_apply_to_live_intent,
    ShortCraftIntentCtx, ShortCraftLiveApplyResult, ShortCraftLiveIntent,
};
use crate::pottery_profession::{
    apply_clay_source_to_gather_input, count_potter_peers_filtered, fill_pottery_counts_from_map,
    is_clay_source_id, pottery_action_short_craft_apply, try_decide_potter_from_rung,
    ClaySourceCandidate, GatherClayInput, PotterPeerSnapshot, PotterProfessionRuntime,
    PotteryAction, PotteryCounts, PotteryMapObj, BASKET, CLAY, CLAY_DEPOSIT, CLAY_PIT,
    KILN_SEARCH_RADIUS, POTTERY_CRAFT_SEARCH_RADIUS,
};
use crate::shepherd_profession::{
    count_shepherd_peers_filtered, is_sheep_herding, make_stuff_try_sheep,
    try_decide_shepherd_from_rung, ShepherdAction, ShepherdCounts, ShepherdPeerSnapshot,
    ShepherdProfessionRuntime, SHEPHERD_DEFAULT_MAX_ANIMAL,
    SHEPHERD_SHORTCRAFT_RADIUS,
};
use crate::smith_profession::{
    chisel_family_extras_beyond_static, count_smith_peers_filtered,
    fill_pottery_on_fire_counts_from_map, fill_smith_counts_from_map, is_forge_id,
    pick_forge_near_home, smith_action_apply, steel_chisel_family_from_content,
    try_decide_smith_from_rung, ForgeCandidate, MapObj, SmithAction, SmithApply, SmithApplyInput,
    SmithPeerSnapshot, SmithProfessionRuntime, DEFAULT_MAX_CLAY_BOWLS, DEFAULT_MAX_CLAY_CROCKS,
    DEFAULT_MAX_CLAY_PLATES, FORGE_SEARCH_RADIUS, IRON_ORE_COUNT_RADIUS, STEEL_COUNT_RADIUS,
};
use crate::{
    age_rotated_job_sequence, is_wound_object, AgeRotatedJobKind, LiveSensorInput, PriorityRung,
    SimState, MAX_AGE, MIN_AGE_TO_EAT,
};

// ── Well ids (Haxe getCloseWell) ────────────────────────────────────────────

/// Deep Well 663 (preferred) then Well 662.
// Haxe: AiBase.getCloseWell wellIs = [663, 662]
pub const WELL_IDS: [i32; 2] = [663, 662];

/// Default scan radius when caller does not specify (covers farm home + shortCraft).
pub const DEFAULT_PROFESSION_SCAN_RADIUS: i32 = FARM_HOME_RADIUS;

/// Max smith scan radius (home + iron/steel/forge search).
pub const SMITH_SCAN_RADIUS: i32 = 30;

/// Max baker scan radius (craft maxSearchRadius wrap).
pub const BAKER_SCAN_RADIUS: i32 = BAKING_CRAFT_SEARCH_RADIUS;

/// Max pottery scan radius (Haxe doPottery maxSearchRadius = 30).
// Haxe: AiBase.doPottery maxSearchRadius = 30
pub const POTTERY_SCAN_RADIUS: i32 = POTTERY_CRAFT_SEARCH_RADIUS;

/// Haxe gatherClay clay deposit/pit search radius from player.
// Haxe: AiBase.gatherClay GetClosestObjectById clay deposit/pit r=80
pub const CLAY_DEPOSIT_SEARCH_RADIUS: i32 = 80;

/// Held ids that must drop on non-floored tiles (Haxe `needsNotFlooredPlace`).
// Haxe: AiHelper.needsNotFlooredPlace [356, 336, 1137, 227, 225]
pub const NEEDS_NOT_FLOORED_PLACE: [i32; 5] = [356, 336, 1137, 227, 225];

/// Held ids that refuse empty tiles within 6 of home (Haxe `dontDropCloseHomeIds`).
// Haxe: AiHelper.dontDropCloseHomeIds (132 listed twice in Haxe — once here)
pub const DONT_DROP_CLOSE_HOME_IDS: [i32; 11] =
    [356, 336, 1137, 227, 225, 850, 190, 160, 132, 183, 291];

/// Min Chebyshev distance from home for [`DONT_DROP_CLOSE_HOME_IDS`].
// Haxe: minDistanceToTarget = 6
pub const DONT_DROP_CLOSE_HOME_MIN: i32 = 6;

/// Seeding Carrots 401 — Haxe `countSeeds` carrot family.
// Haxe: AiBase.countSeeds [401, 2745]
pub const SEEDING_CARROTS: i32 = 401;
/// Bowl of Carrot Seeds 2745.
pub const BOWL_OF_CARROT_SEEDS: i32 = 2745;
/// Bowl of Dry Beans 1176 / Dry Bean Plants 1172 — Haxe `hasBeanSeeds`.
pub const BOWL_OF_DRY_BEANS: i32 = 1176;
pub const DRY_BEAN_PLANTS: i32 = 1172;

// ── Tile snapshot ───────────────────────────────────────────────────────────

/// One world tile captured by [`scan_world_radius`].
// Haxe: ObjectHelper snapshot fields used by CountCloseObjects / shortCraft / dropHeldObject
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanTile {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    /// Haxe `numberOfUses` (0 / 1 when unknown).
    pub uses: i32,
    pub floor_id: i32,
    pub is_food: bool,
    pub is_permanent: bool,
    /// Biome id as `u8` for snow/ocean shortCraft gates.
    pub biome: u8,
    /// Haxe `objectData.numSlots` — container capacity (0 = not a container).
    // Haxe: parentObjData.numSlots / useIsDropInContainer
    pub num_slots: i32,
    /// Haxe `objectData.numUses` — multi-use max (0 = unknown/single).
    // Haxe: numberOfUses >= objectData.numUses full-pile filter
    pub num_uses: i32,
    /// First nested parent id (Haxe `contains` / searchContained), 0 if empty/unknown.
    // Haxe: GetClosestObjectToPosition searchContained e.g. clay [126] in basket
    pub contains_id: i32,
    /// Additional nested parent ids (slots 1..) for any-slot `contains` parity.
    // Haxe: ObjectHelper.contains loops all containedObjects (not only first)
    pub contains_extra: [i32; 7],
    /// Contained object count (Haxe `containedObjects.length`).
    pub contained_count: i32,
}

impl ScanTile {
    pub fn empty(x: i32, y: i32, floor_id: i32, biome: u8) -> Self {
        Self {
            parent_id: 0,
            x,
            y,
            uses: 0,
            floor_id,
            is_food: false,
            is_permanent: false,
            biome,
            num_slots: 0,
            num_uses: 0,
            contains_id: 0,
            contains_extra: [0; 7],
            contained_count: 0,
        }
    }

    pub fn simple(parent_id: i32, x: i32, y: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            uses: 1,
            floor_id: 0,
            is_food: false,
            is_permanent: false,
            biome: 0,
            num_slots: 0,
            num_uses: 0,
            contains_id: 0,
            contains_extra: [0; 7],
            contained_count: 0,
        }
    }

    pub fn with_uses(mut self, uses: i32) -> Self {
        self.uses = uses.max(0);
        self
    }

    pub fn with_num_uses(mut self, num_uses: i32) -> Self {
        self.num_uses = num_uses.max(0);
        self
    }

    pub fn with_num_slots(mut self, num_slots: i32) -> Self {
        self.num_slots = num_slots.max(0);
        self
    }

    pub fn with_contains(mut self, contains_id: i32) -> Self {
        self.contains_id = contains_id.max(0);
        if self.contains_id > 0 && self.contained_count < 1 {
            self.contained_count = 1;
        }
        self
    }

    /// Set nested parent ids for multi-slot searchContained tests (any-slot match).
    // Haxe: ObjectHelper.contains — any containedObjects parent
    pub fn with_contains_list(mut self, ids: &[i32]) -> Self {
        self.contains_id = 0;
        self.contains_extra = [0; 7];
        let mut n = 0i32;
        for &raw in ids {
            let id = raw.max(0);
            if id == 0 {
                continue;
            }
            if n == 0 {
                self.contains_id = id;
            } else if (n as usize) <= self.contains_extra.len() {
                self.contains_extra[(n as usize) - 1] = id;
            }
            n += 1;
        }
        if n > self.contained_count {
            self.contained_count = n;
        }
        self
    }

    pub fn with_contained_count(mut self, n: i32) -> Self {
        self.contained_count = n.max(0);
        self
    }

    pub fn with_floor(mut self, floor_id: i32) -> Self {
        self.floor_id = floor_id;
        self
    }

    pub fn with_biome(mut self, biome: u8) -> Self {
        self.biome = biome;
        self
    }

    /// True when multi-use is at capacity (`numberOfUses >= numUses`).
    // Haxe: numberOfUses >= objectData.numUses
    #[inline]
    pub fn is_full_uses(self) -> bool {
        self.num_uses > 1 && self.uses >= self.num_uses
    }

    /// True when container has free slot(s).
    // Haxe: containedObjects.length < numSlots
    #[inline]
    pub fn has_free_slot(self) -> bool {
        self.num_slots > 0 && self.contained_count < self.num_slots
    }

    /// True when any nested slot reports parent id (Haxe `ObjectHelper.contains`).
    // Haxe: ObjectHelper.contains(searchContained) — any containedObjects parent
    #[inline]
    pub fn contains_parent(self, parent: i32) -> bool {
        parent > 0
            && (self.contains_id == parent
                || self.contains_extra.iter().any(|&id| id == parent))
    }
}

// ── World scan ──────────────────────────────────────────────────────────────

/// Inclusive Chebyshev square scan around `(cx, cy)` with radius `r`.
///
/// Includes **empty** tiles (`parent_id == 0`) so callers can pick drop / ground-use
/// anchors. When `content` is `Some`, resolves dummy multi-use ids to parents and
/// fills food/permanent from object defs; uses come from complex helpers.
// Haxe: AiHelper.CountCloseObjects / GetClosestObjectById double loop
pub fn scan_world_radius(
    world: &World,
    content: Option<&ContentDb>,
    cx: i32,
    cy: i32,
    r: i32,
) -> Vec<ScanTile> {
    let r = r.max(0);
    let side = (2 * r + 1) as usize;
    let mut out = Vec::with_capacity(side.saturating_mul(side).min(4096));
    for dy in -r..=r {
        for dx in -r..=r {
            let x = cx + dx;
            let y = cy + dy;
            let raw_id = world.get_object(x, y);
            let floor_id = world.get_floor(x, y) as i32;
            let biome = world.get_biome(x, y) as u8;
            if raw_id == 0 {
                out.push(ScanTile::empty(x, y, floor_id, biome));
                continue;
            }
            let parent_id = content
                .map(|c| c.resolve_base_id(raw_id))
                .unwrap_or(raw_id);
            let uses = world
                .get_helper(x, y)
                .map(|h| {
                    if h.uses_remaining > 0 {
                        h.uses_remaining
                    } else {
                        1
                    }
                })
                .unwrap_or(1);
            let (is_food, is_permanent, num_slots, num_uses) = content
                .and_then(|c| c.get(parent_id))
                .map(|d| {
                    (
                        d.food_value > 0,
                        d.permanent,
                        d.num_slots.max(0),
                        d.num_uses.max(0),
                    )
                })
                .unwrap_or((false, false, 0, 0));
            // Haxe: ObjectHelper.contains — any-slot nested parents + count for free slots
            // Store first 8 resolved parent ids (contains_id + contains_extra[7]).
            let (contains_id, contains_extra, contained_count) = world
                .get_helper(x, y)
                .map(|h| {
                    let mut first = 0i32;
                    let mut extra = [0i32; 7];
                    let mut filled = 0usize;
                    let clay = crate::pottery_profession::CLAY;
                    for &cid in &h.contained {
                        let base = content
                            .map(|c| c.resolve_base_id(cid))
                            .unwrap_or(cid)
                            .max(0);
                        if base <= 0 {
                            continue;
                        }
                        if filled == 0 {
                            first = base;
                            filled = 1;
                        } else if filled <= extra.len() {
                            extra[filled - 1] = base;
                            filled += 1;
                        } else if base == clay
                            && first != clay
                            && !extra.iter().any(|&e| e == clay)
                        {
                            // Overflow: keep clay visible for pottery searchContained
                            // Haxe: ObjectHelper.contains([126]) any slot
                            extra[extra.len() - 1] = base;
                        }
                    }
                    (first, extra, h.contained.len() as i32)
                })
                .unwrap_or((0, [0; 7], 0));
            out.push(ScanTile {
                parent_id,
                x,
                y,
                uses,
                floor_id,
                is_food,
                is_permanent,
                biome,
                num_slots,
                num_uses,
                contains_id,
                contains_extra,
                contained_count,
            });
        }
    }
    out
}

/// Chebyshev distance (shared with smith/baker).
#[inline]
pub fn scan_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Merge two scan snapshots; first list wins on duplicate (x,y).
// Haxe: gatherClay home r=30 + player clay deposit r=80 independent scans
pub fn merge_scan_tiles(primary: &[ScanTile], extra: &[ScanTile]) -> Vec<ScanTile> {
    let mut seen = HashSet::with_capacity(primary.len().saturating_add(extra.len()));
    let mut out = Vec::with_capacity(primary.len().saturating_add(extra.len()));
    for t in primary.iter().chain(extra.iter()) {
        if seen.insert((t.x, t.y)) {
            out.push(*t);
        }
    }
    out
}

/// Pottery live scan: home-centered craft radius + player-centered clay deposit radius.
// Haxe: doPottery maxSearch 30 near home; gatherClay deposit/pit r=80 from player
pub fn pottery_scan_tiles_from_world(
    world: &World,
    content: Option<&ContentDb>,
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
) -> Vec<ScanTile> {
    let cover = scan_chebyshev(home_x, home_y, player_x, player_y);
    // At home: one disc max(craft, deposit) covers kiln + remote pits.
    if cover == 0 {
        return scan_world_radius(
            world,
            content,
            home_x,
            home_y,
            POTTERY_SCAN_RADIUS.max(CLAY_DEPOSIT_SEARCH_RADIUS),
        );
    }
    let home_tiles = scan_world_radius(world, content, home_x, home_y, POTTERY_SCAN_RADIUS);
    let player_tiles =
        scan_world_radius(world, content, player_x, player_y, CLAY_DEPOSIT_SEARCH_RADIUS);
    merge_scan_tiles(&home_tiles, &player_tiles)
}

// ── Convert scan → profession map snapshots ─────────────────────────────────

/// Non-empty tiles → [`FarmMapObj`] for [`fill_farm_counts_from_map`].
pub fn farm_map_from_scan(tiles: &[ScanTile]) -> Vec<FarmMapObj> {
    tiles
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
        .collect()
}

/// Non-empty tiles → [`BakeMapObj`].
pub fn bake_map_from_scan(tiles: &[ScanTile]) -> Vec<BakeMapObj> {
    tiles
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
        .collect()
}

/// Non-empty tiles → smith [`MapObj`].
pub fn smith_map_from_scan(tiles: &[ScanTile]) -> Vec<MapObj> {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| MapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect()
}

/// Non-empty tiles → [`PotteryMapObj`].
// Haxe: CountCloseObjects / GetKiln scan snapshot for doPottery
pub fn pottery_map_from_scan(tiles: &[ScanTile]) -> Vec<PotteryMapObj> {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| PotteryMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect()
}

// ── Spatial queries on scan ─────────────────────────────────────────────────

/// Closest non-empty tile matching `parent_id` within `max_r` of `(from_x, from_y)`.
/// Tie-break: lower y, then lower x (stable).
// Haxe: AiHelper.GetClosestObjectById / getClosestObjectById
pub fn closest_by_parent_id(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<ScanTile> {
    closest_by_parent_id_ex(tiles, parent_id, from_x, from_y, max_r, 0)
}

/// Closest matching `parent_id` with min/max Chebyshev (Haxe `minDistance`).
// Haxe: GetClosestObjectToPositionHelper minDistance / searchDistance
pub fn closest_by_parent_id_ex(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_r: i32,
) -> Option<ScanTile> {
    if parent_id == 0 {
        return None;
    }
    let max_r = max_r.max(0);
    let min_r = min_r.max(0);
    let mut best: Option<(i32, ScanTile)> = None;
    for t in tiles {
        if t.parent_id != parent_id {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d > max_r || d < min_r {
            continue;
        }
        match best {
            None => best = Some((d, *t)),
            Some((bd, bt)) => {
                if d < bd || (d == bd && (t.y < bt.y || (t.y == bt.y && t.x < bt.x))) {
                    best = Some((d, *t));
                }
            }
        }
    }
    best.map(|(_, t)| t)
}

/// Closest matching id relative to a craft target tile (Haxe GetClosestObjectToTarget).
// Haxe: AiHelper.GetClosestObjectToTarget → GetClosestObjectToPosition(target.tx, target.ty, …)
pub fn closest_by_parent_id_to_target(
    tiles: &[ScanTile],
    parent_id: i32,
    target_x: i32,
    target_y: i32,
    max_r: i32,
    min_r: i32,
) -> Option<ScanTile> {
    closest_by_parent_id_ex(tiles, parent_id, target_x, target_y, max_r, min_r)
}

/// Closest of `parent_id` that contains nested `contains_id` (Haxe `searchContained`).
/// When no contain match, falls back to any `parent_id` within radius (Haxe helper order).
// Haxe: AiHelper.GetClosestObjectToPosition(..., searchContained)
pub fn closest_by_parent_contains(
    tiles: &[ScanTile],
    parent_id: i32,
    contains_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<ScanTile> {
    closest_by_parent_contains_ex(
        tiles,
        parent_id,
        contains_id,
        from_x,
        from_y,
        max_r,
        true,
    )
}

/// Closest of `parent_id` with nested `contains_id`; optional fallback to any parent.
// Haxe: GetClosestObjectToPositionHelper searchContained loop
pub fn closest_by_parent_contains_ex(
    tiles: &[ScanTile],
    parent_id: i32,
    contains_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    fallback_any_parent: bool,
) -> Option<ScanTile> {
    if parent_id == 0 {
        return None;
    }
    let max_r = max_r.max(0);
    if contains_id > 0 {
        let mut best: Option<(i32, ScanTile)> = None;
        for t in tiles {
            if t.parent_id != parent_id || !t.contains_parent(contains_id) {
                continue;
            }
            let d = scan_chebyshev(from_x, from_y, t.x, t.y);
            if d > max_r {
                continue;
            }
            match best {
                None => best = Some((d, *t)),
                Some((bd, bt)) => {
                    if d < bd || (d == bd && (t.y < bt.y || (t.y == bt.y && t.x < bt.x))) {
                        best = Some((d, *t));
                    }
                }
            }
        }
        if best.is_some() {
            return best.map(|(_, t)| t);
        }
    }
    if fallback_any_parent {
        closest_by_parent_id(tiles, parent_id, from_x, from_y, max_r)
    } else {
        None
    }
}

// ── Path filters (Haxe isObjectNotReachable / isObjectWithHostilePath) ───────

/// Pure AI path-block set for profession closest picks.
///
/// Haxe keeps `notReachableObjects` + `blockedByAI` + `objectsWithHostilePath` on AiBase.
/// Live maps: [`crate::ai_path_reach::AiPathReachMaps`] on Player (PATH-REACH).
// Haxe: AiBase.isObjectNotReachable / isObjectWithHostilePath
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfessionPathFilters {
    /// World tiles blocked for closest picks (not reachable or hostile path).
    pub blocked: HashSet<(i32, i32)>,
}

impl ProfessionPathFilters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_blocked(coords: impl IntoIterator<Item = (i32, i32)>) -> Self {
        Self {
            blocked: coords.into_iter().collect(),
        }
    }

    /// Haxe `isObjectNotReachable(tx,ty) || isObjectWithHostilePath(tx,ty)`.
    // Haxe: AiBase.isObjectNotReachable ~9273; isObjectWithHostilePath ~9247
    #[inline]
    pub fn is_object_not_reachable(&self, x: i32, y: i32) -> bool {
        self.blocked.contains(&(x, y))
    }

    #[inline]
    pub fn mark_not_reachable(&mut self, x: i32, y: i32) {
        self.blocked.insert((x, y));
    }

    #[inline]
    pub fn mark_hostile_path(&mut self, x: i32, y: i32) {
        self.blocked.insert((x, y));
    }

    /// True when tile may be used as a shortCraft USE target.
    #[inline]
    pub fn target_reachable(&self, x: i32, y: i32) -> bool {
        !self.is_object_not_reachable(x, y)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.blocked.is_empty()
    }
}

/// Build live filters from Player maps + SimState.blocked_by_ai (PATH-REACH).
// Haxe: isObjectNotReachable || isObjectWithHostilePath (+ blockedByAI)
#[inline]
pub fn path_filters_from_player(
    maps: &crate::ai_path_reach::AiPathReachMaps,
    blocked_by_ai: &std::collections::HashMap<(i32, i32), f32>,
) -> ProfessionPathFilters {
    ProfessionPathFilters::with_blocked(maps.blocked_coords(Some(blocked_by_ai)))
}

/// Filter scan tiles when filters non-empty; otherwise return owned copy.
// Haxe: GetClosestObjectToPositionHelper skip not reachable
pub fn apply_path_filters_to_tiles(
    tiles: &[ScanTile],
    filters: &ProfessionPathFilters,
) -> Vec<ScanTile> {
    if filters.is_empty() {
        tiles.to_vec()
    } else {
        filter_scan_tiles_path_owned(tiles, filters)
    }
}

/// Drop tiles marked not-reachable / hostile from a scan slice (pure).
// Haxe: GetClosestObjectToPositionHelper `if (ai.isObjectNotReachable) continue`
pub fn filter_scan_tiles_path<'a>(
    tiles: &'a [ScanTile],
    filters: &ProfessionPathFilters,
) -> Vec<&'a ScanTile> {
    tiles
        .iter()
        .filter(|t| !filters.is_object_not_reachable(t.x, t.y))
        .collect()
}

/// Owned filtered copy of scan tiles (for map fills that take `&[ScanTile]`).
pub fn filter_scan_tiles_path_owned(
    tiles: &[ScanTile],
    filters: &ProfessionPathFilters,
) -> Vec<ScanTile> {
    tiles
        .iter()
        .copied()
        .filter(|t| !filters.is_object_not_reachable(t.x, t.y))
        .collect()
}

/// Closest `parent_id` skipping path-blocked tiles.
// Haxe: GetClosestObjectToPositionHelper isObjectNotReachable skip
pub fn closest_by_parent_id_path(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    filters: Option<&ProfessionPathFilters>,
) -> Option<ScanTile> {
    if parent_id == 0 {
        return None;
    }
    let max_r = max_r.max(0);
    let mut best: Option<(i32, ScanTile)> = None;
    for t in tiles {
        if t.parent_id != parent_id {
            continue;
        }
        if filters
            .map(|f| f.is_object_not_reachable(t.x, t.y))
            .unwrap_or(false)
        {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, *t)),
            Some((bd, bt)) => {
                if d < bd || (d == bd && (t.y < bt.y || (t.y == bt.y && t.x < bt.x))) {
                    best = Some((d, *t));
                }
            }
        }
    }
    best.map(|(_, t)| t)
}

/// Resolve `target_reachable` for a candidate tile under optional path filters.
// Haxe: isObjectNotReachable gate before useHeldObjOnTarget
#[inline]
pub fn target_reachable_for_tile(
    x: i32,
    y: i32,
    filters: Option<&ProfessionPathFilters>,
) -> bool {
    filters
        .map(|f| f.target_reachable(x, y))
        .unwrap_or(true)
}

/// Contained count from held nest (Haxe `heldObject.containedObjects.length`).
// Haxe: heldObject.containedObjects.length (gatherClay basket cargo)
#[inline]
pub fn held_contained_from_helper(held_helper: Option<&ol_world::NestedHelper>) -> i32 {
    held_helper
        .map(|h| h.contained.len() as i32)
        .unwrap_or(0)
}

/// Contained count from a [`crate::Player`] held nest.
#[inline]
pub fn held_contained_from_player(p: &crate::Player) -> i32 {
    held_contained_from_helper(p.held_helper.as_ref())
}

/// True when any held nest slot parent matches `parent` (Haxe `heldObject.contains([id])`).
// Haxe: ObjectHelper.contains — loops containedObjects
#[inline]
pub fn held_nest_contains_parent(
    held_helper: Option<&ol_world::NestedHelper>,
    parent: i32,
) -> bool {
    if parent <= 0 {
        return false;
    }
    held_helper
        .map(|h| h.contained.iter().any(|c| c.id == parent))
        .unwrap_or(false)
}

/// Haxe `heldObject.contains([126])` or held is bare clay — kiln basket-drop / drop staging.
// Haxe: AiBase.dropHeldObject Basket 292 + Clay 126; gatherClay held clay
#[inline]
pub fn held_contains_clay(held_id: i32, held_helper: Option<&ol_world::NestedHelper>) -> bool {
    if held_id == crate::pottery_profession::CLAY {
        return true;
    }
    held_id == crate::pottery_profession::BASKET
        && held_nest_contains_parent(held_helper, crate::pottery_profession::CLAY)
}

/// From player held nest (live wire).
#[inline]
pub fn held_contains_clay_from_player(p: &crate::Player) -> bool {
    held_contains_clay(p.held_id, p.held_helper.as_ref())
}

/// Convert scan tiles → GetOrCraft world objects.
///
/// Prefers live [`ScanTile::num_slots`] filled by [`scan_world_radius`] from
/// ObjectDef; falls back to optional `num_slots_for` lookup when tile slots are 0
/// (partial / synthetic scans).
// Haxe: ObjectHelper snapshot for GetOrCraftItem — objectData.numSlots
pub fn get_or_craft_objs_from_scan(
    tiles: &[ScanTile],
    num_slots_for: Option<&dyn Fn(i32) -> i32>,
) -> Vec<crate::get_or_craft::GetOrCraftWorldObj> {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| {
            // Live scan already carries ObjectDef.numSlots; lookup is residual for
            // synthetic ScanTile::simple without slots filled.
            // Haxe: obj.objectData.numSlots > 0 empty-hand gate
            let slots = if t.num_slots > 0 {
                t.num_slots
            } else {
                num_slots_for.map(|f| f(t.parent_id)).unwrap_or(0)
            };
            crate::get_or_craft::GetOrCraftWorldObj::simple(t.parent_id, t.x, t.y).with_slots(slots)
        })
        .collect()
}

/// Coords of full multi-use tiles (`numberOfUses >= numUses`) for CraftScanFilters.
///
/// Feed into [`crate::CraftScanFilters::with_full_piles`] when Haxe `ignoreFullPiles`
/// is active (pile-drop search / multi-step reverse-use full skip).
// Haxe: ignoreFullPiles + numberOfUses >= objectData.numUses
pub fn full_pile_tiles_from_scan(tiles: &[ScanTile]) -> HashSet<(i32, i32)> {
    tiles
        .iter()
        .filter(|t| t.is_full_uses())
        .map(|t| (t.x, t.y))
        .collect()
}

/// Options for empty-tile search (Haxe dropHeldObject / GetClosestObject empty).
// Haxe: AiHelper.GetClosestObjectToPositionHelper searchEmptyPlace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosestEmptyOpts {
    /// Held parent id for not-floored / home-clearance rules (`-1` = ignore held rules).
    pub held_id: i32,
    pub home_x: i32,
    pub home_y: i32,
    /// Extra min Chebyshev from `(from_x, from_y)` (Haxe `minDistance`).
    pub min_distance: i32,
    /// When true and held ∈ [`DONT_DROP_CLOSE_HOME_IDS`], skip tiles within 6 of home.
    pub respect_home_clearance: bool,
    /// When true and held ∈ [`NEEDS_NOT_FLOORED_PLACE`], skip floored tiles.
    pub respect_not_floored: bool,
}

impl ClosestEmptyOpts {
    pub fn basic() -> Self {
        Self {
            held_id: -1,
            home_x: 0,
            home_y: 0,
            min_distance: 0,
            respect_home_clearance: false,
            respect_not_floored: false,
        }
    }

    /// Live drop path: home clearance + not-floored from held id.
    // Haxe: GetClosestObjectToPositionHelper heldId path
    pub fn for_held(held_id: i32, home_x: i32, home_y: i32) -> Self {
        Self {
            held_id,
            home_x,
            home_y,
            min_distance: 0,
            respect_home_clearance: true,
            respect_not_floored: true,
        }
    }
}

/// Closest empty tile (`parent_id == 0`) within `max_r`, preferring `d > 0`.
// Haxe: getClosestObjectById(0, dist)
pub fn closest_empty_tile(
    tiles: &[ScanTile],
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> Option<(i32, i32)> {
    closest_empty_tile_ex(tiles, from_x, from_y, max_r, ClosestEmptyOpts::basic())
}

/// Empty-tile search with held drop rules (not-floored, home clearance).
// Haxe: AiHelper.GetClosestObjectToPositionHelper objIdToSearch==0
pub fn closest_empty_tile_ex(
    tiles: &[ScanTile],
    from_x: i32,
    from_y: i32,
    max_r: i32,
    opts: ClosestEmptyOpts,
) -> Option<(i32, i32)> {
    let need_bare = opts.respect_not_floored
        && opts.held_id > 0
        && NEEDS_NOT_FLOORED_PLACE.contains(&opts.held_id);
    let home_clear = opts.respect_home_clearance
        && opts.held_id > 0
        && DONT_DROP_CLOSE_HOME_IDS.contains(&opts.held_id);
    let min_d = opts.min_distance.max(0);

    let mut best: Option<(i32, i32, i32)> = None; // dist, x, y
    for t in tiles {
        if t.parent_id != 0 {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d == 0 || d > max_r || d < min_d {
            continue;
        }
        if need_bare && t.floor_id > 0 {
            continue;
        }
        if home_clear {
            let dh = scan_chebyshev(opts.home_x, opts.home_y, t.x, t.y);
            if dh < DONT_DROP_CLOSE_HOME_MIN {
                continue;
            }
        }
        match best {
            None => best = Some((d, t.x, t.y)),
            Some((bd, bx, by)) => {
                if d < bd || (d == bd && (t.y < by || (t.y == by && t.x < bx))) {
                    best = Some((d, t.x, t.y));
                }
            }
        }
    }
    if let Some((_, x, y)) = best {
        return Some((x, y));
    }
    // Fallback: player tile if empty (and rules allow)
    tiles
        .iter()
        .find(|t| {
            t.parent_id == 0
                && t.x == from_x
                && t.y == from_y
                && !(need_bare && t.floor_id > 0)
                && !(home_clear
                    && scan_chebyshev(opts.home_x, opts.home_y, t.x, t.y)
                        < DONT_DROP_CLOSE_HOME_MIN)
        })
        .map(|t| (t.x, t.y))
}

/// Closest empty tile near a well (663 then 662), within search of home.
// Haxe: shortCraftOnGround 336 → GetClosestObjectToTarget(well, 0, …)
pub fn empty_near_well(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    max_r: i32,
) -> Option<(i32, i32)> {
    empty_near_well_ex(tiles, home_x, home_y, max_r, -1)
}

/// Like [`empty_near_well`] with held-aware empty pick (336 soil prefers bare ground).
// Haxe: shortCraftOnGround 336 minDist + needsNotFlooredPlace
pub fn empty_near_well_ex(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    max_r: i32,
    held_id: i32,
) -> Option<(i32, i32)> {
    let well = closest_well(tiles, home_x, home_y, max_r)?;
    let opts = if held_id > 0 {
        ClosestEmptyOpts::for_held(held_id, home_x, home_y)
    } else {
        ClosestEmptyOpts::basic()
    };
    closest_empty_tile_ex(tiles, well.x, well.y, 20, opts)
}

/// Closest well (663 preferred over 662 at equal distance via priority index).
// Haxe: GetClosestObjectToPositionByIds wellIs = [663, 662]
pub fn closest_well(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    max_r: i32,
) -> Option<ScanTile> {
    let mut best: Option<(i32, usize, ScanTile)> = None; // dist, priority index, tile
    for t in tiles {
        let Some(prio) = WELL_IDS.iter().position(|&id| id == t.parent_id) else {
            continue;
        };
        let d = scan_chebyshev(home_x, home_y, t.x, t.y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, prio, *t)),
            Some((bd, bp, _)) if d < bd || (d == bd && prio < bp) => {
                best = Some((d, prio, *t));
            }
            _ => {}
        }
    }
    best.map(|(_, _, t)| t)
}

/// Closest forge tile among forge family ids.
pub fn closest_forge_from_scan(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
) -> Option<(i32, i32, i32)> {
    let cands: Vec<ForgeCandidate> = tiles
        .iter()
        .filter(|t| is_forge_id(t.parent_id))
        .map(|t| ForgeCandidate {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    pick_forge_near_home(home_x, home_y, &cands).map(|f| (f.parent_id, f.x, f.y))
}

/// Closest oven tile among oven family ids.
pub fn closest_oven_from_scan(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
) -> Option<(i32, i32, i32)> {
    let cands: Vec<OvenCandidate> = tiles
        .iter()
        .filter(|t| is_oven_id(t.parent_id))
        .map(|t| OvenCandidate {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    pick_oven_near_home(home_x, home_y, &cands).map(|o| (o.parent_id, o.x, o.y))
}

/// Build [`ShortCraftIntentCtx`] for a known target tile + empty/well/forge anchors.
///
/// `held_id` drives not-floored / home-clearance empty picks (0 = ignore held rules).
// Haxe: useHeldObjOnTarget coords + dropHeldObject empty + shortCraftOnGround anchors
pub fn build_intent_ctx(
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    target: Option<ScanTile>,
    forge_xy: Option<(i32, i32)>,
    target_reachable: bool,
) -> ShortCraftIntentCtx {
    build_intent_ctx_ex(
        tiles,
        player_x,
        player_y,
        home_x,
        home_y,
        target,
        forge_xy,
        target_reachable,
        0,
    )
}

/// Like [`build_intent_ctx`] with held-aware empty / well drop anchors.
// Haxe: dropHeldObject + shortCraftOnGround 336 well path
pub fn build_intent_ctx_ex(
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    target: Option<ScanTile>,
    forge_xy: Option<(i32, i32)>,
    target_reachable: bool,
    held_id: i32,
) -> ShortCraftIntentCtx {
    let (tx, ty) = target
        .map(|t| (t.x, t.y))
        .unwrap_or((player_x, player_y));
    let drop_opts = if held_id > 0 {
        ClosestEmptyOpts::for_held(held_id, home_x, home_y)
    } else {
        ClosestEmptyOpts::basic()
    };
    let empty = closest_empty_tile_ex(tiles, player_x, player_y, 30, drop_opts)
        .or_else(|| closest_empty_tile_ex(tiles, home_x, home_y, 30, drop_opts))
        .or_else(|| closest_empty_tile(tiles, player_x, player_y, 30))
        .unwrap_or((player_x, player_y));
    let well_empty = empty_near_well_ex(tiles, home_x, home_y, 30, held_id);
    let home_empty = closest_empty_tile_ex(tiles, home_x, home_y, 20, drop_opts);
    let (fx, fy) = forge_xy.unwrap_or((home_x, home_y));
    let forge_pickup =
        closest_empty_tile_ex(tiles, fx, fy, 10, ClosestEmptyOpts::basic()).unwrap_or((fx, fy));

    let mut ctx = ShortCraftIntentCtx::at_target(tx, ty);
    ctx.empty_drop_x = empty.0;
    ctx.empty_drop_y = empty.1;
    ctx.forge_x = fx;
    ctx.forge_y = fy;
    ctx.forge_pickup_x = forge_pickup.0;
    ctx.forge_pickup_y = forge_pickup.1;
    ctx.target_reachable = target_reachable;
    if let Some((wx, wy)) = well_empty {
        ctx.empty_near_well_x = Some(wx);
        ctx.empty_near_well_y = Some(wy);
    }
    if let Some((hx, hy)) = home_empty {
        ctx.empty_near_home_x = Some(hx);
        ctx.empty_near_home_y = Some(hy);
    }
    ctx
}

// ── Seed counts from scan (Haxe countSeeds / hasBeanSeeds) ──────────────────

/// Count tiles whose `parent_id` is in `ids` (no radius filter — caller scoped the scan).
// Haxe: AiBase.countCurrentObjects
pub fn count_parent_ids_in_scan(tiles: &[ScanTile], ids: &[i32]) -> i32 {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0 && ids.contains(&t.parent_id))
        .count() as i32
}

/// Haxe `hasCarrotSeeds`: Seeding Carrots 401 + Bowl of Carrot Seeds 2745, count > 1.
// Haxe: AiBase.countSeeds ~1362–1364
pub fn has_carrot_seeds_from_scan(tiles: &[ScanTile]) -> bool {
    count_parent_ids_in_scan(tiles, &[SEEDING_CARROTS, BOWL_OF_CARROT_SEEDS]) > 1
}

/// Haxe `hasBeanSeeds`: Bowl of Dry Beans 1176 + Dry Bean Plants 1172, count > 1.
// Haxe: AiBase.hasBeanSeeds ~1370–1373
pub fn has_bean_seeds_from_scan(tiles: &[ScanTile]) -> bool {
    count_parent_ids_in_scan(tiles, &[BOWL_OF_DRY_BEANS, DRY_BEAN_PLANTS]) > 1
}

/// Count non-empty tiles matching `parent_id` within Chebyshev `max_r` of `(from_x, from_y)`.
pub fn count_parent_id_near(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
) -> i32 {
    if parent_id == 0 {
        return 0;
    }
    tiles
        .iter()
        .filter(|t| t.parent_id == parent_id)
        .filter(|t| scan_chebyshev(from_x, from_y, t.x, t.y) <= max_r)
        .count() as i32
}

/// Floor id at `(x,y)` from scan tiles (0 if missing).
pub fn floor_at_scan(tiles: &[ScanTile], x: i32, y: i32) -> i32 {
    tiles
        .iter()
        .find(|t| t.x == x && t.y == y)
        .map(|t| t.floor_id)
        .unwrap_or(0)
}

// ── Profession kind + tick inputs ───────────────────────────────────────────

/// Which profession SM to run on a scan tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfessionScanKind {
    Farm,
    Smith,
    Baker,
    /// Haxe `doPottery` / POTTER sticky (AI-POTTER + NPC-SCAN-FULL).
    Pottery,
    /// Haxe `isSheepHerding` / SHEPHERD sticky (AI-SHEPHERD).
    Shepherd,
    /// Haxe `makeFireFood` / FIREFOODMAKER assigned|last (AI-FIREFOOD-RUNG).
    // Haxe: assignedProfession == 'FIREFOODMAKER' || lastProfession == 'FIREFOODMAKER' ~754
    FireFood,
    /// Haxe `isHandlingFire` / FIREKEEPER (AI-HANDLING-FIRE).
    HandlingFire,
}

/// Shared player/world inputs for a profession scan tick (no SimState borrow).
/// Not `Copy` — `chisel_family_extra` is a content-scan `Vec` (AI-JOB-SMITH-RESID).
#[derive(Debug, Clone)]
pub struct ProfessionScanInput {
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub held_id: i32,
    /// Haxe `heldObject.numberOfUses` (0 → treat as 1 for bake dough max).
    pub held_uses: i32,
    /// Haxe `heldObject.containedObjects.length` (basket clay cargo).
    // Haxe: heldObject.containedObjects.length gatherClay
    pub held_contained: i32,
    /// Haxe `heldObject.contains([126])` or held clay — kiln drop / DropHeld clay flag.
    // Haxe: dropHeldObject Basket+Clay 126; not merely held_contained > 0
    pub held_contains_clay: bool,
    pub food_store: f32,
    /// Haxe transition hungry work cost for the pair (0 = free).
    pub transition_hungry_cost: f32,
    pub has_carrot_seeds: bool,
    /// Haxe `hasBeanSeeds` for baker bean paths.
    pub has_bean_seeds: bool,
    pub is_hungry: bool,
    pub basic_farmer_weight: f32,
    /// Optional biome override for hardened-row gate (snow/ocean).
    pub hardened_row_biome: Option<u8>,
    /// When false, shortCraft UseAt is suppressed (path filters / not reachable).
    // Haxe: isObjectNotReachable / isObjectWithHostilePath
    pub target_reachable: bool,
    pub peer_count: f32,
    pub was_idle: f32,
    pub age: f32,
    /// True when sticky last/assigned matches this profession.
    pub profession_is_sticky: bool,
    /// Baker assigned-job dispatch (maxPeople 100 vs 1).
    pub is_assigned_job: bool,
    /// Haxe `myPlayer.isMoving()` — dropHeld BusyMoving → Wait hold tick.
    // Haxe: dropHeldObject dropOnStart isMoving (PREFER-SHORT-WAIT)
    pub is_moving: bool,
    /// Haxe `TimeHelper.Season == Winter` — Fire 82 kindling first.
    // Haxe: AiBase.isHandlingFire ~1172 (AI-HANDLING-FIRE)
    pub is_winter: bool,
    /// Content-scan extras for Haxe `objectIdArrays[455]` (ids beyond static seed).
    /// Empty → static [`crate::STEEL_CHISEL_FAMILY`] only.
    // Haxe: ServerSettings.objectIdArrays[455] after PatchObjectData Chisel scan
    // AI-JOB-SMITH-RESID / chisel content scan
    pub chisel_family_extra: Vec<i32>,
}

impl ProfessionScanInput {
    pub fn basic(px: i32, py: i32, held_id: i32) -> Self {
        Self {
            player_x: px,
            player_y: py,
            home_x: px,
            home_y: py,
            held_id,
            held_uses: 1,
            held_contained: 0,
            held_contains_clay: held_id == crate::pottery_profession::CLAY,
            food_store: 20.0,
            transition_hungry_cost: 0.0,
            has_carrot_seeds: true,
            has_bean_seeds: true,
            is_hungry: false,
            basic_farmer_weight: 1.0,
            hardened_row_biome: None,
            target_reachable: true,
            peer_count: 0.0,
            was_idle: 0.0,
            age: 20.0,
            profession_is_sticky: true,
            is_assigned_job: true,
            is_moving: false,
            is_winter: false,
            chisel_family_extra: Vec::new(),
        }
    }
}

/// Result of a profession scan tick (intent + optional debug action labels).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfessionScanTickResult {
    pub intent: ShortCraftLiveIntent,
    /// True when a ShortCraft / SmithAction step was produced (including defer/seek).
    pub had_action: bool,
}

impl ProfessionScanTickResult {
    pub fn none() -> Self {
        Self {
            intent: ShortCraftLiveIntent::None,
            had_action: false,
        }
    }
}

// ── Farm scan tick ──────────────────────────────────────────────────────────

/// Fill farm counts from scan tiles (pure).
///
/// Origin floor at home drives IsIgnoredFloor (Haxe CountCloseObjects origin quirk).
pub fn farm_counts_from_scan(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    held_id: i32,
    home_r: i32,
    is_hungry: bool,
    basic_farmer_weight: f32,
    hardened_row_biome: Option<u8>,
) -> crate::farmer_profession::FarmCounts {
    let map = farm_map_from_scan(tiles);
    let origin_floor = floor_at_scan(tiles, home_x, home_y);
    let mut c = fill_farm_counts_from_map_with_floor(
        home_x,
        home_y,
        held_id,
        &map,
        home_r,
        origin_floor,
    );
    c.is_hungry = is_hungry;
    c.basic_farmer_weight = basic_farmer_weight;
    c.hardened_row_biome = hardened_row_biome;
    c
}

/// Farm profession: decide job → shortCraft → live USE/DROP intent.
// Haxe: doBasicFarming / doCarrotFarming + shortCraft → useHeldObjOnTarget
pub fn farm_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    job: Option<FarmProfession>,
    rung_label: &str,
    task: &mut FarmTaskState,
    has_profession: bool,
    farm_rt: &mut FarmProfessionRuntime,
) -> ProfessionScanTickResult {
    let counts = farm_counts_from_scan(
        tiles,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        FARM_COUNT_RADIUS,
        inp.is_hungry,
        inp.basic_farmer_weight,
        inp.hardened_row_biome,
    );
    let Some(action) =
        try_decide_farm_from_rung(job, rung_label, &counts, task, has_profession)
    else {
        return ProfessionScanTickResult::none();
    };
    farm_action_to_live_intent(tiles, inp, action, farm_rt)
}

/// Build FarmCounts from profession scan tiles (AI-SHEPHERD-MID after-sheep tail).
fn farm_counts_from_scan_for_after_sheep(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
) -> FarmCounts {
    let mut map: HashMap<i32, i32> = HashMap::new();
    for t in tiles {
        *map.entry(t.parent_id).or_insert(0) += 1;
    }
    let pairs: Vec<(i32, i32)> = map.into_iter().collect();
    // Haxe: CountCloseObjects bulk for doBasicFarming after sheep mid
    crate::farm_counts_from_nearby(
        &pairs,
        inp.held_id,
        inp.is_hungry || inp.food_store <= crate::HUNGRY_FOOD,
        inp.basic_farmer_weight,
        inp.hardened_row_biome,
    )
}

/// Map a decided [`FarmAction`] through shortCraft apply + spatial ctx → intent.
///
/// AI-FARM-STICKY: applies Haxe `profession['BASICFARMER']` writes onto `farm_rt`.
// Haxe: AiBase.doBasicFarming ~2400 / ~2415
pub fn farm_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: FarmAction,
    farm_rt: &mut FarmProfessionRuntime,
) -> ProfessionScanTickResult {
    // AI-FARM-STICKY: live Player.farm_profession.weights[BASICFARMER]
    apply_basic_farmer_weight_side_effect(farm_rt, action);
    match action {
        FarmAction::None | FarmAction::Abort | FarmAction::ClearBasicFarmerWeight => {
            ProfessionScanTickResult::none()
        }
        FarmAction::CraftItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        // AI-SHEPHERD-MID: doBasicFarming mid isSheepHerding(max) then late plants
        // + age-gated makeSharpieFood + doAdvancedFarming / clear BASICFARMER.
        FarmAction::DeferSheepHerding { max_profession } => {
            // Haxe: this.profession['BASICFARMER']=1 immediately before isSheepHerding
            // (applied above via apply_basic_farmer_weight_side_effect).
            let counts = shepherd_counts_from_scan(
                tiles,
                inp.home_x,
                inp.home_y,
                inp.held_id,
                true,
                inp.age,
                SHEPHERD_SHORTCRAFT_RADIUS,
            );
            let mut rt = ShepherdProfessionRuntime {
                is_last_shepherd: true,
                ..Default::default()
            };
            let mut farm_task = FarmTaskState::default();
            // Haxe: isSheepHerding(1) hard-coded at mid (not maxProfession)
            // Haxe: AiBase.doBasicFarming ~2402
            let r = is_sheep_herding(
                &mut rt,
                &counts,
                &mut farm_task,
                1,
                SHEPHERD_DEFAULT_MAX_ANIMAL,
                inp.peer_count,
                inp.was_idle,
            );
            if r.action.is_some() {
                return shepherd_action_to_live_intent(tiles, inp, r.action);
            }
            let farm_counts = farm_counts_from_scan_for_after_sheep(tiles, inp);
            // Haxe after mid: late plants → age<20 sharpie → doAdvancedFarming(maxProfession)
            // AI-FARM-STICKY: pass outer doBasicFarming max (2 default / 100 assigned)
            // Haxe: AiBase.doBasicFarming ~2413
            let late = crate::do_basic_farming_after_sheep(
                &farm_counts,
                &mut farm_task,
                inp.age,
                max_profession,
            );
            match late {
                FarmAction::DeferSheepHerding { .. } => ProfessionScanTickResult::none(),
                other => farm_action_to_live_intent(tiles, inp, other, farm_rt),
            }
        }
        // Haxe: doAdvancedFarming(max) then profession['BASICFARMER']=0
        FarmAction::DeferAdvancedFarming { max_profession: _ } => {
            let farm_counts = farm_counts_from_scan_for_after_sheep(tiles, inp);
            let mut farm_task = FarmTaskState::default();
            // Haxe hasOrBecomeProfession('ADVANCEDFARMER', max) — peer-cap residual;
            // attempt body (true) so late plants/sharpie→advanced still yields work.
            let next = expand_advanced_farming_or_clear(
                &farm_counts,
                &mut farm_task,
                inp.age,
                true,
            );
            match next {
                FarmAction::ClearBasicFarmerWeight | FarmAction::None | FarmAction::Abort => {
                    apply_basic_farmer_weight_side_effect(farm_rt, next);
                    ProfessionScanTickResult::none()
                }
                FarmAction::DeferAdvancedFarming { .. } | FarmAction::DeferSheepHerding { .. } => {
                    ProfessionScanTickResult::none()
                }
                other => farm_action_to_live_intent(tiles, inp, other, farm_rt),
            }
        }
        FarmAction::ShortCraft { actor, target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                FARM_SHORTCRAFT_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(tiles, target, inp.home_x, inp.home_y, FARM_SHORTCRAFT_RADIUS)
            });
            let (target_uses, target_biome) = target_tile
                .map(|t| (t.uses, Some(t.biome)))
                .unwrap_or((1, inp.hardened_row_biome));
            let new_actor_count = tiles
                .iter()
                .filter(|t| t.parent_id == actor)
                .filter(|t| {
                    scan_chebyshev(inp.player_x, inp.player_y, t.x, t.y) <= FARM_COUNT_RADIUS
                })
                .count() as i32;
            let sc_inp = ShortCraftInput {
                held_id: inp.held_id,
                actor_id: actor,
                target_id: target,
                target_uses,
                target_biome,
                has_carrot_seeds: inp.has_carrot_seeds,
                new_actor_count,
                max_new_actor: -1,
                try_weak_skewer_first: actor == SKEWER,
                craft_actor_if_needed: true,
                food_store: inp.food_store,
                transition_hungry_cost: inp.transition_hungry_cost,
            };
            let apply = short_craft_apply_resolved(sc_inp);
            if matches!(apply, ShortCraftApply::UseOnTarget { .. }) && target_tile.is_none() {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: target,
                        craft_if_needed: false,
                    },
                    had_action: true,
                };
            }
            // Haxe: shortCraft DropHeld → dropHeldObject smart (DROP-HELD-LIVE / PREFER-SHORT-WAIT)
            if matches!(apply, ShortCraftApply::DropHeld) {
                let intent = super::smart_drop_held_profession_ex(
                    tiles,
                    inp.held_id,
                    inp.held_uses,
                    inp.player_x,
                    inp.player_y,
                    inp.home_x,
                    inp.home_y,
                    inp.food_store,
                    false,
                    40.0,
                    false,
                    inp.is_moving,
                );
                return ProfessionScanTickResult {
                    had_action: super::drop_held_live_intent_actionable(intent)
                        || !matches!(intent, ShortCraftLiveIntent::None),
                    intent,
                };
            }
            let forge =
                closest_forge_from_scan(tiles, inp.home_x, inp.home_y).map(|(_, x, y)| (x, y));
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                forge,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}

// ── Smith scan tick ─────────────────────────────────────────────────────────

/// Smith profession: fill counts → decide → smith_action_apply → live intent.
// Haxe: doSmithing + critical shortCrafts + shortCraftOnGround
// AI-JOB-SMITH-RESID: chisel content scan via ProfessionScanInput.chisel_family_extra
pub fn smith_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    runtime: &mut SmithProfessionRuntime,
) -> ProfessionScanTickResult {
    let map = smith_map_from_scan(tiles);
    let home_r = FORGE_SEARCH_RADIUS
        .max(IRON_ORE_COUNT_RADIUS)
        .max(STEEL_COUNT_RADIUS);
    // Haxe: objectIdArrays[455] — static seed + content extras on counts
    let mut counts =
        fill_smith_counts_from_map(inp.home_x, inp.home_y, inp.held_id, &map, home_r);
    if !inp.chisel_family_extra.is_empty() {
        counts.attach_chisel_family_extra(&inp.chisel_family_extra);
    }
    let Some(action) = try_decide_smith_from_rung(
        inp.profession_is_sticky,
        rung_label,
        &counts,
        runtime,
        inp.peer_count,
        inp.was_idle,
        inp.age,
    ) else {
        return ProfessionScanTickResult::none();
    };
    smith_action_to_live_intent(tiles, inp, action)
}

/// Map a decided [`SmithAction`] through apply + spatial ctx → intent.
pub fn smith_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: SmithAction,
) -> ProfessionScanTickResult {
    if matches!(action, SmithAction::None | SmithAction::Abort) {
        return ProfessionScanTickResult::none();
    }
    // Haxe: shortCraftOnTarget maxNewActor — count existing actor parent near player.
    let actor_for_count = match action {
        SmithAction::ShortCraft { actor, .. } => actor,
        // Ground path: count target as "actor" for nearby existence (no held actor).
        SmithAction::ShortCraftOnGround { target } => target,
        _ => 0,
    };
    let new_actor_count = if actor_for_count > 0 {
        count_parent_id_near(
            tiles,
            actor_for_count,
            inp.player_x,
            inp.player_y,
            SMITH_SCAN_RADIUS,
        )
    } else {
        0
    };
    let forge_xy = closest_forge_from_scan(tiles, inp.home_x, inp.home_y).map(|(_, x, y)| (x, y));
    let quad_dist_to_forge = forge_xy
        .map(|(fx, fy)| {
            let dx = inp.player_x - fx;
            let dy = inp.player_y - fy;
            dx * dx + dy * dy
        })
        .unwrap_or(0);
    // AI-POTTER-L2946: live smith DeferPottery expands via do_pottery_on_fire counts
    // filled from the same scan map (Haxe prepareSmithingTools → doPotteryOnFire).
    // Haxe: AiBase.prepareSmithingTools ~3680 countFirekiln && doPotteryOnFire()
    let pottery_counts = if matches!(action, SmithAction::DeferPottery) {
        let map = smith_map_from_scan(tiles);
        Some(fill_pottery_on_fire_counts_from_map(
            inp.home_x,
            inp.home_y,
            inp.player_x,
            inp.player_y,
            &map,
            20, // Haxe CountCloseObjects r=20
            DEFAULT_MAX_CLAY_BOWLS,
            DEFAULT_MAX_CLAY_PLATES,
            DEFAULT_MAX_CLAY_CROCKS,
        ))
    } else {
        None
    };
    let apply_inp = SmithApplyInput {
        held_id: inp.held_id,
        food_store: inp.food_store,
        short_craft_work_cost: inp.transition_hungry_cost,
        new_actor_count,
        max_new_actor: -1,
        craft_drop_near_count: 0,
        quad_dist_to_forge,
        craft_drop_nearby_exists: false,
        pottery: pottery_counts,
    };
    let apply = smith_action_apply(action, &apply_inp);
    if matches!(
        apply,
        SmithApply::None | SmithApply::Abort | SmithApply::Refuse
    ) {
        return ProfessionScanTickResult::none();
    }
    if matches!(apply, SmithApply::RefuseHungryCost) {
        return ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::RefuseHungry,
            had_action: true,
        };
    }

    let target_id = match apply {
        SmithApply::UseOnTarget { target, .. } => Some(target),
        SmithApply::SeekOrGetGroundActor { target } => Some(target),
        _ => None,
    };
    let target_tile = target_id.and_then(|tid| {
        closest_by_parent_id(tiles, tid, inp.player_x, inp.player_y, SMITH_SCAN_RADIUS).or_else(
            || closest_by_parent_id(tiles, tid, inp.home_x, inp.home_y, SMITH_SCAN_RADIUS),
        )
    });
    // Haxe: shortCraft DropHeld → dropHeldObject smart near forge (DROP-HELD-LIVE / PREFER-SHORT-WAIT)
    if matches!(apply, SmithApply::DropHeld) {
        let intent = super::smart_drop_held_profession_ex(
            tiles,
            inp.held_id,
            inp.held_uses,
            inp.player_x,
            inp.player_y,
            inp.home_x,
            inp.home_y,
            inp.food_store,
            false,
            40.0,
            false,
            inp.is_moving,
        );
        return ProfessionScanTickResult {
            had_action: super::drop_held_live_intent_actionable(intent)
                || !matches!(intent, ShortCraftLiveIntent::None),
            intent,
        };
    }
    let ctx = build_intent_ctx_ex(
        tiles,
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        target_tile,
        forge_xy,
        inp.target_reachable,
        inp.held_id,
    );
    let intent = smith_apply_to_live_intent(apply, &ctx);
    ProfessionScanTickResult {
        had_action: !matches!(intent, ShortCraftLiveIntent::None),
        intent,
    }
}

// ── Baker scan tick ─────────────────────────────────────────────────────────

/// Baker profession: fill counts → decide → shortCraft → live intent.
// Haxe: doBaking + shortCraftOnTarget(raw, hotOven)
pub fn baker_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
) -> ProfessionScanTickResult {
    let map = bake_map_from_scan(tiles);
    let held_uses = if inp.held_uses > 0 { inp.held_uses } else { 1 };
    let origin_floor = floor_at_scan(tiles, inp.home_x, inp.home_y);
    let counts = fill_bake_counts_from_map_ex(
        inp.home_x,
        inp.home_y,
        inp.held_id,
        held_uses,
        &map,
        OVEN_SEARCH_RADIUS,
        inp.is_hungry,
        inp.has_carrot_seeds,
        inp.has_bean_seeds,
        origin_floor,
    );
    let Some(action) = try_decide_baker_from_rung(
        inp.profession_is_sticky,
        rung_label,
        inp.is_assigned_job,
        &counts,
        runtime,
        task,
        inp.peer_count,
        inp.was_idle,
        0, // rng_pie_index
    ) else {
        return ProfessionScanTickResult::none();
    };
    bake_action_to_live_intent(tiles, inp, action)
}

// ── Shepherd scan tick ──────────────────────────────────────────────────────

/// Home-radius object ids used by sheep/cow/goose herding pure SM.
// Haxe: isSheepHerding shortCraft targets + countCorn family
const SHEPHERD_SCAN_IDS: &[i32] = &[
    crate::baker_profession::DOMESTIC_SHEEP,
    crate::baker_profession::DOMESTIC_LAMB,
    crate::baker_profession::HUNGRY_DOMESTIC_LAMB,
    crate::shepherd_profession::HUNGRY_MOUFLON_LAMB,
    crate::shepherd_profession::SHORN_DOMESTIC_SHEEP,
    crate::shepherd_profession::HUNGRY_DOMESTIC_CALF,
    crate::shepherd_profession::DOMESTIC_CALF,
    crate::shepherd_profession::MILK_COW,
    crate::shepherd_profession::DOMESTIC_COW,
    crate::shepherd_profession::DEAD_COW,
    crate::shepherd_profession::DOMESTIC_GOOSE,
    crate::shepherd_profession::COLD_GOOSE_EGG,
    crate::shepherd_profession::BOWL_CORN_KERNELS,
    crate::shepherd_profession::DRIED_EAR_OF_CORN,
    crate::shepherd_profession::BOWL_CORN_COB,
    crate::shepherd_profession::DUMPED_CORN_KERNELS,
    crate::shepherd_profession::PILE_CORN_KERNELS,
    crate::baker_profession::BOWL_BERRIES_CARROT,
    crate::baker_profession::BOWL_GOOSEBERRIES,
    crate::baker_profession::DOMESTIC_BUSH,
    crate::baker_profession::WILD_BUSH,
    crate::baker_profession::MILK_POUCH,
    crate::baker_profession::WHIPPED_CREAM,
    crate::baker_profession::BOWL_OF_CREAM,
    crate::baker_profession::BUTTERED_BREAD,
    crate::baker_profession::BOWL_OF_BUTTER,
    crate::baker_profession::KNIFE,
    crate::shepherd_profession::EMPTY_BUCKET,
    crate::farmer_profession::DYING_BUSH,
    crate::farmer_profession::COMPOSTING_PILE,
    crate::farmer_profession::COMPOSTED_SOIL,
    crate::farmer_profession::FERTILE_SOIL_PILE,
    crate::farmer_profession::DRY_PLANTED_CARROTS,
    crate::farmer_profession::WET_PLANTED_CARROTS,
    crate::farmer_profession::CARROT,
    crate::farmer_profession::DRY_PLANTED_CORN,
    crate::farmer_profession::WET_PLANTED_CORN,
    crate::farmer_profession::CORN_SPROUT,
    crate::farmer_profession::CORN_PLANT,
    crate::farmer_profession::BOWL_OF_SOIL,
];

/// Fill [`ShepherdCounts`] from scan tiles (pure).
// Haxe: CountCloseObjects home radius for isSheepHerding
pub fn shepherd_counts_from_scan(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    held_id: i32,
    has_corn_seeds: bool,
    age: f32,
    radius: i32,
) -> ShepherdCounts {
    let mut c = ShepherdCounts {
        held_id,
        has_corn_seeds,
        age,
        ..Default::default()
    };
    for &id in SHEPHERD_SCAN_IDS {
        let n = tiles
            .iter()
            .filter(|t| t.parent_id == id)
            .filter(|t| scan_chebyshev(home_x, home_y, t.x, t.y) <= radius)
            .count() as i32;
        if n > 0 {
            c.set(id, n);
        }
    }
    c
}

/// Shepherd profession: fill counts → is_sheep_herding → live intent.
// Haxe: isSheepHerding / doFeedLambsAndCalfs → shortCraft
pub fn shepherd_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    runtime: &mut ShepherdProfessionRuntime,
    farm_task: &mut FarmTaskState,
) -> ProfessionScanTickResult {
    let counts = shepherd_counts_from_scan(
        tiles,
        inp.home_x,
        inp.home_y,
        inp.held_id,
        // Corn kernels / dried ear near home act as hasCornSeeds for pure gate.
        inp.has_carrot_seeds
            || count_parent_id_near(
                tiles,
                crate::shepherd_profession::BOWL_CORN_KERNELS,
                inp.home_x,
                inp.home_y,
                SHEPHERD_SHORTCRAFT_RADIUS,
            ) > 0
            || count_parent_id_near(
                tiles,
                crate::shepherd_profession::DRIED_EAR_OF_CORN,
                inp.home_x,
                inp.home_y,
                SHEPHERD_SHORTCRAFT_RADIUS,
            ) > 0,
        inp.age,
        SHEPHERD_SHORTCRAFT_RADIUS,
    );
    // Prefer full is_sheep_herding; maxAnimal 10 age / 100 assigned uses default animal.
    let Some(action) = try_decide_shepherd_from_rung(
        inp.profession_is_sticky,
        rung_label,
        inp.is_assigned_job,
        &counts,
        runtime,
        farm_task,
        inp.peer_count,
        inp.was_idle,
        SHEPHERD_DEFAULT_MAX_ANIMAL,
    ) else {
        return ProfessionScanTickResult::none();
    };
    shepherd_action_to_live_intent(tiles, inp, action)
}

/// Map a decided [`ShepherdAction`] through shortCraft apply → live intent.
pub fn shepherd_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: ShepherdAction,
) -> ProfessionScanTickResult {
    match action {
        ShepherdAction::None | ShepherdAction::Abort => ProfessionScanTickResult::none(),
        ShepherdAction::CraftItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        ShepherdAction::ShortCraft { actor, target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                SHEPHERD_SHORTCRAFT_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    target,
                    inp.home_x,
                    inp.home_y,
                    SHEPHERD_SHORTCRAFT_RADIUS,
                )
            });
            let (target_uses, target_biome) = target_tile
                .map(|t| (t.uses, Some(t.biome)))
                .unwrap_or((1, None));
            let new_actor_count = tiles
                .iter()
                .filter(|t| t.parent_id == actor)
                .filter(|t| {
                    scan_chebyshev(inp.player_x, inp.player_y, t.x, t.y) <= SHEPHERD_SHORTCRAFT_RADIUS
                })
                .count() as i32;
            let sc_inp = ShortCraftInput {
                held_id: inp.held_id,
                actor_id: actor,
                target_id: target,
                target_uses,
                target_biome,
                has_carrot_seeds: inp.has_carrot_seeds,
                new_actor_count,
                max_new_actor: -1,
                try_weak_skewer_first: actor == SKEWER,
                craft_actor_if_needed: true,
                food_store: inp.food_store,
                transition_hungry_cost: inp.transition_hungry_cost,
            };
            let apply = short_craft_apply_resolved(sc_inp);
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                None,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None)
                    || matches!(apply, ShortCraftApply::SeekOrCraftActor { .. }),
                intent,
            }
        }
    }
}

// ── Pottery scan tick (NPC-SCAN-FULL / AI-POTTER live USE I/O) ───────────────

/// Fill pottery counts from scan tiles (pure).
// Haxe: CountCloseObjects + GetKiln for doPottery
pub fn pottery_counts_from_scan(
    tiles: &[ScanTile],
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    held_contained: i32,
) -> PotteryCounts {
    let map = pottery_map_from_scan(tiles);
    fill_pottery_counts_from_map(
        home_x,
        home_y,
        player_x,
        player_y,
        held_id,
        held_contained,
        &map,
        KILN_SEARCH_RADIUS,
        0,
        0,
        0,
    )
}

/// Build gatherClay spatial input from scan tiles (clay-in-basket via ScanTile contain).
// Haxe: AiBase.gatherClay deposit/basket/loose clay scans ~2956–3097
pub fn gather_clay_input_from_scan(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    held_contained: i32,
) -> GatherClayInput {
    let clay_cands: Vec<ClaySourceCandidate> = tiles
        .iter()
        .filter(|t| is_clay_source_id(t.parent_id))
        .map(|t| ClaySourceCandidate {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    let source = crate::pottery_profession::pick_closest_clay_source(
        inp.player_x,
        inp.player_y,
        &clay_cands,
    );

    // Haxe: if (kiln != null) home = kiln — dist + basket-near-home r=10 use kiln xy
    // Haxe: AiBase.gatherClay L2962
    let (home_x, home_y) = closest_by_parent_id(
        tiles,
        crate::pottery_profession::WOOD_FILLED_KILN,
        inp.home_x,
        inp.home_y,
        KILN_SEARCH_RADIUS,
    )
    .or_else(|| {
        closest_by_parent_id(
            tiles,
            crate::pottery_profession::ADOBE_KILN,
            inp.home_x,
            inp.home_y,
            KILN_SEARCH_RADIUS,
        )
    })
    .or_else(|| {
        closest_by_parent_id(
            tiles,
            crate::pottery_profession::FIRING_ADOBE_KILN,
            inp.home_x,
            inp.home_y,
            KILN_SEARCH_RADIUS,
        )
    })
    .map(|t| (t.x, t.y))
    .unwrap_or((inp.home_x, inp.home_y));

    let mut g = GatherClayInput {
        player_x: inp.player_x,
        player_y: inp.player_y,
        home_x,
        home_y,
        held_id: inp.held_id,
        held_contained,
        ..Default::default()
    };
    apply_clay_source_to_gather_input(&mut g, source);

    // Haxe: GetClosestObjectToPosition(home, 292, 10, …, [126]) — basket with clay near home/kiln
    // Prefer contain match only (no empty-basket fallback for "with clay" flags).
    // Haxe: searchContained [126] Clay — any-slot via ScanTile.contains_parent
    let basket_clay_home = closest_by_parent_contains_ex(
        tiles,
        BASKET,
        CLAY,
        home_x,
        home_y,
        10,
        false,
    );
    // Haxe: GetClosestObjectToPosition(player, 292, 20, …, [126])
    let basket_clay_player = closest_by_parent_contains_ex(
        tiles,
        BASKET,
        CLAY,
        inp.player_x,
        inp.player_y,
        20,
        false,
    );
    g.basket_with_clay_near_home = basket_clay_home.is_some();
    g.basket_with_clay_near_player = basket_clay_player.is_some();

    // Haxe: GetClosestObjectToPosition(deposit, 292, 5) — any basket near deposit (no contain)
    let basket_near_dep = source.and_then(|dep| {
        closest_by_parent_id(tiles, BASKET, dep.x, dep.y, 5)
    });
    // Used as "has basket near deposit" for fill path (contained ≤2).
    g.empty_basket_near_deposit = basket_near_dep
        .map(|t| t.contained_count <= 2)
        .unwrap_or(false);
    // Full basket only at deposit (player may be far) — still enables PickupBasket.
    // Haxe: basket = deposit search; if contained > 2 → pickup
    g.full_basket_near_deposit = basket_near_dep
        .map(|t| t.contained_count > 2)
        .unwrap_or(false);

    // Haxe: basket.containedObjects.length > 2 among found baskets; also held cargo
    g.basket_full = held_contained > 2
        || basket_clay_home
            .map(|t| t.contained_count > 2)
            .unwrap_or(false)
        || basket_clay_player
            .map(|t| t.contained_count > 2)
            .unwrap_or(false)
        || g.full_basket_near_deposit;

    // Haxe: GetClosestObjectToPosition(player, 126, 5) loose clay when far from home
    g.loose_clay_near_player = closest_by_parent_id(
        tiles,
        CLAY,
        inp.player_x,
        inp.player_y,
        5,
    )
    .is_some();
    g
}

/// Fire-food maker profession: fill counts → makeFireFood → live intent.
// Haxe: makeFireFood / FIREFOODMAKER assigned|last → makeFireFood(100) ~754–756
pub fn fire_food_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    runtime: &mut crate::FireFoodProfessionRuntime,
) -> ProfessionScanTickResult {
    let map: Vec<crate::FireFoodMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::FireFoodMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
        })
        .collect();
    let mut counts = crate::fill_fire_food_counts_from_map(
        inp.home_x,
        inp.home_y,
        inp.held_id,
        &map,
        crate::FIRE_FOOD_HOME_RADIUS,
        inp.is_hungry,
        false,
        inp.has_bean_seeds,
    );
    let has_corn = tiles.iter().any(|t| {
        matches!(
            t.parent_id,
            crate::BOWL_CORN_KERNELS
                | crate::shepherd_profession::DRIED_EAR_OF_CORN
                | crate::shepherd_profession::BOWL_CORN_COB
        )
    });
    counts.has_corn_seeds = has_corn;
    let Some(action) = crate::try_decide_fire_food_from_rung(
        inp.profession_is_sticky,
        rung_label,
        inp.is_assigned_job,
        &counts,
        runtime,
        inp.peer_count,
        inp.was_idle,
    ) else {
        return ProfessionScanTickResult::none();
    };
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    fire_food_action_to_live_intent(tiles, inp, action)
}

/// Pottery profession: fill counts → doPottery → live USE/DROP intent.
// Haxe: doPottery / doPotteryHelper / gatherClay → shortCraft
pub fn pottery_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    runtime: &mut PotterProfessionRuntime,
) -> ProfessionScanTickResult {
    // Haxe: heldObject.containedObjects.length (basket nest via held_helper)
    let held_contained = inp.held_contained;
    let counts = pottery_counts_from_scan(
        tiles,
        inp.home_x,
        inp.home_y,
        inp.player_x,
        inp.player_y,
        inp.held_id,
        held_contained,
    );
    let gather = gather_clay_input_from_scan(tiles, inp, held_contained);
    let Some(action) = try_decide_potter_from_rung(
        &counts,
        runtime,
        rung_label,
        inp.peer_count,
        inp.was_idle,
        inp.is_assigned_job,
        Some(&gather),
    ) else {
        return ProfessionScanTickResult::none();
    };
    if matches!(action, PotteryAction::None | PotteryAction::Abort) {
        return ProfessionScanTickResult::none();
    }
    pottery_action_to_live_intent(tiles, inp, action)
}

/// Map a decided [`PotteryAction`] through smith-style shortCraft apply → live intent.
// Haxe: shortCraft / shortCraftOnGround / gatherClay spatial USE/DROP
pub fn pottery_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: PotteryAction,
) -> ProfessionScanTickResult {
    match action {
        PotteryAction::None | PotteryAction::Abort => ProfessionScanTickResult::none(),
        PotteryAction::CraftItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        PotteryAction::SeekOrCraft { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekOrCraft {
                actor: object_id,
                craft_if_needed: true,
            },
            had_action: true,
        },
        PotteryAction::GotoHome => {
            if let Some(t) = closest_by_parent_id(
                tiles,
                crate::pottery_profession::WOOD_FILLED_KILN,
                inp.home_x,
                inp.home_y,
                POTTERY_SCAN_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    crate::pottery_profession::ADOBE_KILN,
                    inp.home_x,
                    inp.home_y,
                    POTTERY_SCAN_RADIUS,
                )
            }) {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: t.parent_id,
                        craft_if_needed: false,
                    },
                    had_action: true,
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: CLAY,
                        craft_if_needed: false,
                    },
                    had_action: true,
                }
            }
        }
        PotteryAction::GotoClayDeposit | PotteryAction::UseOnClayDeposit => {
            let t = closest_by_parent_id(
                tiles,
                CLAY_DEPOSIT,
                inp.player_x,
                inp.player_y,
                POTTERY_SCAN_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    CLAY_PIT,
                    inp.player_x,
                    inp.player_y,
                    POTTERY_SCAN_RADIUS,
                )
            });
            if let Some(tile) = t {
                if matches!(action, PotteryAction::UseOnClayDeposit) || inp.held_id != 0 {
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::UseAt {
                            x: tile.x,
                            y: tile.y,
                            target_id: tile.parent_id,
                            actor_id: inp.held_id,
                        },
                        had_action: true,
                    }
                } else {
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::SeekOrCraft {
                            actor: tile.parent_id,
                            craft_if_needed: false,
                        },
                        had_action: true,
                    }
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: CLAY_DEPOSIT,
                        craft_if_needed: true,
                    },
                    had_action: true,
                }
            }
        }
        PotteryAction::PickupLooseClay => {
            if let Some(tile) =
                closest_by_parent_id(tiles, CLAY, inp.player_x, inp.player_y, POTTERY_SCAN_RADIUS)
            {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::UseAt {
                        x: tile.x,
                        y: tile.y,
                        target_id: CLAY,
                        actor_id: 0,
                    },
                    had_action: true,
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: CLAY,
                        craft_if_needed: false,
                    },
                    had_action: true,
                }
            }
        }
        // AI-POTTER-RESID: EmptyBasketAtHome = empty-hand DROP extract (dropIsAUse=false).
        // Haxe: gatherClay L3013–3014 dropTarget=basket; isDropingItem → myPlayer.drop
        PotteryAction::EmptyBasketAtHome => {
            use crate::pottery_profession::{
                empty_basket_at_home_is_drop_extract, EMPTY_BASKET_HOME_SEARCH_RADIUS,
            };
            let from = (inp.home_x, inp.home_y);
            // Haxe: GetClosestObjectToPosition(home, 292, 10, …, [126]) clay-in-basket first
            let tile = closest_by_parent_contains(
                tiles,
                BASKET,
                CLAY,
                from.0,
                from.1,
                EMPTY_BASKET_HOME_SEARCH_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_contains(
                    tiles,
                    BASKET,
                    CLAY,
                    from.0,
                    from.1,
                    POTTERY_SCAN_RADIUS,
                )
            });
            if let Some(tile) = tile {
                if empty_basket_at_home_is_drop_extract(inp.held_id) {
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::DropAt {
                            x: tile.x,
                            y: tile.y,
                        },
                        had_action: true,
                    }
                } else {
                    // SM should DropHeld first; USE would fill/pick wrong — still stage DropAt path
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::DropAt {
                            x: tile.x,
                            y: tile.y,
                        },
                        had_action: true,
                    }
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: BASKET,
                        craft_if_needed: true,
                    },
                    had_action: true,
                }
            }
        }
        PotteryAction::UseOnBasket | PotteryAction::PickupBasket => {
            let from = (inp.player_x, inp.player_y);
            // PickupBasket prefer clay-in-basket (searchContained 126).
            // Haxe: GetClosestObjectToPosition(..., [126]) then any basket
            // UseOnBasket: reuse deposit-adjacent / clay basket (Haxe `basket` variable)
            // Haxe: gatherClay basket near player [126] then deposit r=5 any
            let tile = match action {
                PotteryAction::PickupBasket => {
                    closest_by_parent_contains(
                        tiles,
                        BASKET,
                        CLAY,
                        from.0,
                        from.1,
                        POTTERY_SCAN_RADIUS,
                    )
                    .or_else(|| {
                        // Pickup full basket near deposit even without clay contain flag
                        closest_by_parent_id(tiles, BASKET, from.0, from.1, POTTERY_SCAN_RADIUS)
                    })
                }
                PotteryAction::UseOnBasket => {
                    // Prefer basket with clay near player, then empty/any near deposit, then any
                    closest_by_parent_contains_ex(
                        tiles,
                        BASKET,
                        CLAY,
                        inp.player_x,
                        inp.player_y,
                        20,
                        false,
                    )
                    .or_else(|| {
                        closest_by_parent_id(
                            tiles,
                            CLAY_DEPOSIT,
                            inp.player_x,
                            inp.player_y,
                            POTTERY_SCAN_RADIUS,
                        )
                        .or_else(|| {
                            closest_by_parent_id(
                                tiles,
                                CLAY_PIT,
                                inp.player_x,
                                inp.player_y,
                                POTTERY_SCAN_RADIUS,
                            )
                        })
                        .and_then(|dep| closest_by_parent_id(tiles, BASKET, dep.x, dep.y, 5))
                    })
                    .or_else(|| {
                        closest_by_parent_id(tiles, BASKET, from.0, from.1, POTTERY_SCAN_RADIUS)
                    })
                }
                _ => closest_by_parent_id(tiles, BASKET, from.0, from.1, POTTERY_SCAN_RADIUS),
            };
            if let Some(tile) = tile {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::UseAt {
                        x: tile.x,
                        y: tile.y,
                        target_id: BASKET,
                        actor_id: inp.held_id,
                    },
                    had_action: true,
                }
            } else {
                ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: BASKET,
                        craft_if_needed: true,
                    },
                    had_action: true,
                }
            }
        }
        // Haxe: dropHeldObject(maxDistanceToHome, allowAllPiles) pottery clay/basket
        PotteryAction::DropHeld {
            allow_piles,
            max_distance_to_home,
        } => {
            // Haxe: heldObject.contains([126]) — not merely held_contained > 0
            let held_contains_clay = inp.held_contains_clay || inp.held_id == CLAY;
            // Haxe: empty basket deposit staging → maxDist 0 (empty_basket_drop_is_deposit_staging)
            let deposit_adj = closest_by_parent_id(
                tiles,
                CLAY_DEPOSIT,
                inp.player_x,
                inp.player_y,
                1,
            )
            .or_else(|| {
                closest_by_parent_id(tiles, CLAY_PIT, inp.player_x, inp.player_y, 1)
            })
            .is_some();
            let staging = crate::pottery_profession::empty_basket_drop_is_deposit_staging(
                inp.held_id,
                inp.held_contained,
                deposit_adj,
            );
            let max_dist = if staging {
                0.0
            } else {
                max_distance_to_home.max(0) as f32
            };
            let intent = super::smart_drop_held_profession_ex(
                tiles,
                inp.held_id,
                inp.held_uses,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                inp.food_store,
                allow_piles,
                max_dist,
                held_contains_clay,
                inp.is_moving,
            );
            if super::drop_held_live_intent_actionable(intent)
                || !matches!(intent, ShortCraftLiveIntent::None)
            {
                ProfessionScanTickResult {
                    intent,
                    had_action: true,
                }
            } else {
                // Fallback feet empty when planner yields None (empty hands residual).
                let empty = closest_empty_tile_ex(
                    tiles,
                    inp.player_x,
                    inp.player_y,
                    FARM_SHORTCRAFT_RADIUS.max(10),
                    ClosestEmptyOpts::for_held(inp.held_id, inp.home_x, inp.home_y),
                )
                .or_else(|| closest_empty_tile(tiles, inp.player_x, inp.player_y, 10));
                if let Some((x, y)) = empty {
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::DropAt { x, y },
                        had_action: true,
                    }
                } else {
                    ProfessionScanTickResult {
                        intent: ShortCraftLiveIntent::None,
                        had_action: true,
                    }
                }
            }
        }
        PotteryAction::ShortCraft { .. } | PotteryAction::ShortCraftOnGround { .. } => {
            let apply = pottery_action_short_craft_apply(action, inp.held_id);
            if matches!(
                apply,
                crate::smith_profession::SmithApply::None
                    | crate::smith_profession::SmithApply::Abort
                    | crate::smith_profession::SmithApply::Refuse
            ) {
                return ProfessionScanTickResult::none();
            }
            let target_id = match apply {
                crate::smith_profession::SmithApply::UseOnTarget { target, .. } => Some(target),
                crate::smith_profession::SmithApply::SeekOrGetGroundActor { target } => Some(target),
                _ => match action {
                    PotteryAction::ShortCraft { target, .. }
                    | PotteryAction::ShortCraftOnGround { target } => Some(target),
                    _ => None,
                },
            };
            let target_tile = target_id.and_then(|tid| {
                closest_by_parent_id(tiles, tid, inp.player_x, inp.player_y, POTTERY_SCAN_RADIUS)
                    .or_else(|| {
                        closest_by_parent_id(
                            tiles,
                            tid,
                            inp.home_x,
                            inp.home_y,
                            POTTERY_SCAN_RADIUS,
                        )
                    })
            });
            let kiln_xy = closest_by_parent_id(
                tiles,
                crate::pottery_profession::ADOBE_KILN,
                inp.home_x,
                inp.home_y,
                POTTERY_SCAN_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(
                    tiles,
                    crate::pottery_profession::WOOD_FILLED_KILN,
                    inp.home_x,
                    inp.home_y,
                    POTTERY_SCAN_RADIUS,
                )
            })
            .map(|t| (t.x, t.y));
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                kiln_xy,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = smith_apply_to_live_intent(apply, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}


/// Map a decided [`BakeAction`] through shortCraft apply + spatial ctx → intent.
pub fn bake_action_to_live_intent(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    action: BakeAction,
) -> ProfessionScanTickResult {
    match action {
        BakeAction::None | BakeAction::Abort => ProfessionScanTickResult::none(),
        BakeAction::CraftItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::CraftItem { object_id },
            had_action: true,
        },
        // Haxe: baker doPottery(1) mid chain → expand pure pottery (NPC-SCAN-FULL)
        BakeAction::DeferPottery => {
            let mut rt = PotterProfessionRuntime {
                is_last_potter: true,
                ..Default::default()
            };
            let r = pottery_profession_scan_tick(tiles, inp, "AGE_ROTATED_JOB", &mut rt);
            if r.had_action && !matches!(r.intent, ShortCraftLiveIntent::None) {
                return r;
            }
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::DeferPottery,
                had_action: true,
            }
        },
        // Haxe: baker mid isSheepHerding(2,5) → expand pure shepherd shortCrafts
        BakeAction::DeferSheepHerding => {
            let counts = shepherd_counts_from_scan(
                tiles,
                inp.home_x,
                inp.home_y,
                inp.held_id,
                true,
                inp.age,
                SHEPHERD_SHORTCRAFT_RADIUS,
            );
            let mut rt = ShepherdProfessionRuntime {
                is_last_shepherd: true,
                ..Default::default()
            };
            let mut farm_task = FarmTaskState::default();
            let r = is_sheep_herding(
                &mut rt,
                &counts,
                &mut farm_task,
                2, // baker mid maxProfession
                5, // baker mid maxAnimal
                inp.peer_count,
                inp.was_idle,
            );
            if r.action.is_some() {
                return shepherd_action_to_live_intent(tiles, inp, r.action);
            }
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::None,
                had_action: true,
            }
        }
        BakeAction::DeferFarm
        | BakeAction::DeferPlantCarrots
        | BakeAction::DeferHarvestWheat
        | BakeAction::DeferPlantWheat
        | BakeAction::DeferPlantBeans
        | BakeAction::DeferBerryBowl
        | BakeAction::DeferSeatsCleanup
        | BakeAction::DeferCleanup => {
            // Residual: Defer* bodies not expanded this chunk — surface as staging
            // had_action so callers can chain farm later.
            ProfessionScanTickResult {
                intent: ShortCraftLiveIntent::None,
                had_action: true,
            }
        }
        BakeAction::ShortCraft { actor, target } => {
            let target_tile = closest_by_parent_id(
                tiles,
                target,
                inp.player_x,
                inp.player_y,
                BAKER_SCAN_RADIUS,
            )
            .or_else(|| {
                closest_by_parent_id(tiles, target, inp.home_x, inp.home_y, BAKER_SCAN_RADIUS)
            });
            // Prefer oven family if target is oven id but exact missing
            let target_tile = target_tile.or_else(|| {
                if is_oven_id(target) {
                    closest_oven_from_scan(tiles, inp.home_x, inp.home_y).and_then(|(oid, x, y)| {
                        tiles
                            .iter()
                            .copied()
                            .find(|t| t.parent_id == oid && t.x == x && t.y == y)
                    })
                } else {
                    None
                }
            });
            let new_actor_count = tiles
                .iter()
                .filter(|t| t.parent_id == actor)
                .filter(|t| {
                    scan_chebyshev(inp.player_x, inp.player_y, t.x, t.y) <= BAKER_SCAN_RADIUS
                })
                .count() as i32;
            let apply = bake_action_short_craft_apply_ex(
                BakeAction::ShortCraft { actor, target },
                inp.held_id,
                new_actor_count,
                -1,
                inp.food_store,
                inp.transition_hungry_cost,
            )
            .unwrap_or(ShortCraftApply::Refuse);
            if matches!(apply, ShortCraftApply::RefuseHungry) {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::RefuseHungry,
                    had_action: true,
                };
            }
            if matches!(apply, ShortCraftApply::UseOnTarget { .. }) && target_tile.is_none() {
                return ProfessionScanTickResult {
                    intent: ShortCraftLiveIntent::SeekOrCraft {
                        actor: target,
                        craft_if_needed: false,
                    },
                    had_action: true,
                };
            }
            // Haxe: baker shortCraft DropHeld → dropHeldObject near oven (DROP-HELD-LIVE / PREFER-SHORT-WAIT)
            if matches!(apply, ShortCraftApply::DropHeld) {
                let intent = super::smart_drop_held_profession_ex(
                    tiles,
                    inp.held_id,
                    inp.held_uses,
                    inp.player_x,
                    inp.player_y,
                    inp.home_x,
                    inp.home_y,
                    inp.food_store,
                    false,
                    40.0,
                    false,
                    inp.is_moving,
                );
                return ProfessionScanTickResult {
                    had_action: super::drop_held_live_intent_actionable(intent)
                        || !matches!(intent, ShortCraftLiveIntent::None),
                    intent,
                };
            }
            let forge =
                closest_forge_from_scan(tiles, inp.home_x, inp.home_y).map(|(_, x, y)| (x, y));
            let ctx = build_intent_ctx_ex(
                tiles,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                target_tile,
                forge,
                inp.target_reachable,
                inp.held_id,
            );
            let intent = short_craft_apply_to_live_intent(apply, &ctx);
            ProfessionScanTickResult {
                had_action: !matches!(intent, ShortCraftLiveIntent::None),
                intent,
            }
        }
    }
}

// ── Peer caps from live sim roster ──────────────────────────────────────────

/// Peer wound + follow flags for countProfession filters.
// Haxe: AiBase.countProfession isWounded + playerToFollow
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerRosterFlags {
    /// Haxe `p.isWounded()` — combat wound or heavy held wound.
    pub is_wounded: bool,
    /// Haxe `ai.playerToFollow != null` — social following map membership.
    pub has_player_to_follow: bool,
}

/// Resolve wound/follow for one player from live sim (content + combat + social).
// Haxe: GlobalPlayerInstance.isWounded; AiBase.playerToFollow
pub fn peer_roster_flags_for_player(
    p: &crate::Player,
    content: &ContentDb,
    combat_wound: bool,
    following: Option<&HashMap<i32, i32>>,
) -> PeerRosterFlags {
    let held_is_wound = is_wound_object(content, p.held_id);
    PeerRosterFlags {
        // Haxe: heldObject.isWound() && heldObject != hiddenWound
        is_wounded: combat_wound || p.is_wounded_held(held_is_wound),
        // Haxe: ai.playerToFollow != null — port: social.following has follower p_id
        has_player_to_follow: following
            .map(|f| f.contains_key(&p.p_id))
            .unwrap_or(false),
    }
}

/// Pure peer flags when content/combat are unavailable (tests / pure helpers).
// Haxe: isWounded via is_wounded_held; playerToFollow via following map
pub fn peer_roster_flags_pure(
    p: &crate::Player,
    following: Option<&HashMap<i32, i32>>,
    held_is_wound_object: bool,
) -> PeerRosterFlags {
    PeerRosterFlags {
        is_wounded: p.is_wounded_held(held_is_wound_object),
        has_player_to_follow: following
            .map(|f| f.contains_key(&p.p_id))
            .unwrap_or(false),
    }
}

/// Build smith peer snapshots from other players (same home, sticky last smith).
// Haxe: AiBase.countProfession('SMITH') over Connection.getAis
pub fn smith_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<SmithPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    smith_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Smith peers with wound/follow from social + optional pure wound predicate.
// Haxe: countProfession isWounded / playerToFollow
pub fn smith_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<SmithPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            SmithPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_is_smith: p.smith_profession.is_last_smith,
            }
        })
        .collect()
}

/// Build baker peer snapshots from other players.
// Haxe: AiBase.countProfession('BAKER')
pub fn baker_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<BakerPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    baker_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Baker peers with wound/follow filters.
// Haxe: countProfession isWounded / playerToFollow
pub fn baker_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<BakerPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            BakerPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_is_baker: p.baker_profession.is_last_baker,
            }
        })
        .collect()
}

/// One peer sticky on a farm profession (Haxe `countProfession` farm keys).
// Haxe: AiBase.countProfession over lastProfession farm keys
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FarmPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub has_player_to_follow: bool,
    pub same_home: bool,
    pub last_farm: Option<FarmProfession>,
}

impl FarmPeerSnapshot {
    /// Eligible for farm profession count (Haxe countProfession filters).
    // Haxe: AiBase.countProfession ~1284–1308
    pub fn eligible_for_count(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        // Non-gravekeeper: age > MaxAge - 2 excluded
        if self.age > max_age - 2.0 {
            return false;
        }
        if self.is_wounded {
            return false;
        }
        if self.food_store < 0.0 {
            return false;
        }
        if self.has_player_to_follow {
            return false;
        }
        if !self.same_home {
            return false;
        }
        true
    }

    pub fn counts_as_farm(self, want: FarmProfession, min_age: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age, max_age) && self.last_farm == Some(want)
    }
}

/// Build farm peer snapshots from other players at same home.
// Haxe: AiBase.countProfession for BASICFARMER / BERRY / …
pub fn farm_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<FarmPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    farm_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Farm peers with wound/follow/food_store filters.
// Haxe: countProfession isWounded / food_store / playerToFollow
pub fn farm_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<FarmPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            FarmPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_farm: p.farm_profession.last_profession,
            }
        })
        .collect()
}

/// Count farm peers sticky on `want` (Haxe MaxAge-2, wound, food, follow).
// Haxe: AiBase.countProfession farm key
pub fn count_farm_peers_for_job(
    peers: &[FarmPeerSnapshot],
    want: FarmProfession,
    min_age: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_farm(want, min_age, max_age))
        .count() as f32
}

/// Lightweight multi-profession sticky row for NPC peer roster (no full [`crate::Player`]).
///
/// Built from `PlayerSnapshot` home/last sticky + optional npc-local profession_state OR.
// Haxe: AiBase.countProfession over Connection.getAis
// AI-JOB-SMITH-RESID / multi-prof npc peer
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NpcProfessionPeerRow {
    pub conn_id: u64,
    pub home_x: i32,
    pub home_y: i32,
    pub age: f32,
    pub food_store: f32,
    pub deleted: bool,
    pub has_player_to_follow: bool,
    pub is_wounded: bool,
    pub last_is_smith: bool,
    pub last_is_baker: bool,
    pub last_is_potter: bool,
    pub last_is_shepherd: bool,
    pub last_is_farm: bool,
    pub last_is_fire_food: bool,
}

impl NpcProfessionPeerRow {
    /// Eligible filters shared by all profession peer counts (before last-profession match).
    // Haxe: AiBase.countProfession ~1284–1308
    pub fn eligible(self, self_conn_id: u64, home_x: i32, home_y: i32, min_age: f32, max_age: f32) -> bool {
        if self.conn_id == self_conn_id {
            return false;
        }
        if self.deleted {
            return false;
        }
        if self.age < min_age {
            return false;
        }
        if self.age > max_age - 2.0 {
            return false;
        }
        if self.is_wounded {
            return false;
        }
        if self.food_store < 0.0 {
            return false;
        }
        if self.has_player_to_follow {
            return false;
        }
        if self.home_x != home_x || self.home_y != home_y {
            return false;
        }
        true
    }
}

/// Pure `countProfession` for a scan kind from lightweight NPC/snapshot rows.
// Haxe: AiBase.countProfession(profession)
// AI-JOB-SMITH-RESID / multi-prof npc peer_count
pub fn npc_peer_count_for_kind(
    kind: ProfessionScanKind,
    rows: &[NpcProfessionPeerRow],
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    match kind {
        ProfessionScanKind::Smith => {
            let peers: Vec<SmithPeerSnapshot> = rows
                .iter()
                .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
                .map(|r| SmithPeerSnapshot {
                    deleted: r.deleted,
                    age: r.age,
                    is_wounded: r.is_wounded,
                    food_store: r.food_store,
                    has_player_to_follow: r.has_player_to_follow,
                    same_home: true,
                    last_is_smith: r.last_is_smith,
                })
                .collect();
            count_smith_peers_filtered(&peers, min_age_to_eat, max_age)
        }
        ProfessionScanKind::Baker => {
            let peers: Vec<BakerPeerSnapshot> = rows
                .iter()
                .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
                .map(|r| BakerPeerSnapshot {
                    deleted: r.deleted,
                    age: r.age,
                    is_wounded: r.is_wounded,
                    food_store: r.food_store,
                    has_player_to_follow: r.has_player_to_follow,
                    same_home: true,
                    last_is_baker: r.last_is_baker,
                })
                .collect();
            count_baker_peers_filtered(&peers, min_age_to_eat, max_age)
        }
        ProfessionScanKind::Pottery => {
            let peers: Vec<PotterPeerSnapshot> = rows
                .iter()
                .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
                .map(|r| PotterPeerSnapshot {
                    deleted: r.deleted,
                    age: r.age,
                    is_wounded: r.is_wounded,
                    food_store: r.food_store,
                    has_player_to_follow: r.has_player_to_follow,
                    same_home: true,
                    last_is_potter: r.last_is_potter,
                })
                .collect();
            count_potter_peers_filtered(&peers, min_age_to_eat, max_age)
        }
        ProfessionScanKind::Shepherd => {
            let peers: Vec<ShepherdPeerSnapshot> = rows
                .iter()
                .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
                .map(|r| ShepherdPeerSnapshot {
                    deleted: r.deleted,
                    age: r.age,
                    is_wounded: r.is_wounded,
                    food_store: r.food_store,
                    has_player_to_follow: r.has_player_to_follow,
                    same_home: true,
                    last_is_shepherd: r.last_is_shepherd,
                })
                .collect();
            count_shepherd_peers_filtered(&peers, min_age_to_eat, max_age)
        }
        ProfessionScanKind::Farm => rows
            .iter()
            .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
            .filter(|r| r.last_is_farm)
            .count() as f32,
        ProfessionScanKind::FireFood | ProfessionScanKind::HandlingFire => rows
            .iter()
            .filter(|r| r.eligible(self_conn_id, home_x, home_y, min_age_to_eat, max_age))
            .filter(|r| r.last_is_fire_food)
            .count() as f32,
    }
}

/// Count sticky profession peers for a kind (farm / smith / baker / shepherd / pottery).
// Haxe: AiBase.countProfession — live roster + wound/follow
pub fn peer_count_for_kind(
    kind: ProfessionScanKind,
    state: &SimState,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> f32 {
    let following = &state.social.following;
    let is_wounded = |p: &crate::Player| {
        let combat = state.combat.wound_of(p.p_id) > 0;
        peer_roster_flags_for_player(p, &state.content, combat, Some(following)).is_wounded
    };
    let is_wounded_ref: &dyn Fn(&crate::Player) -> bool = &is_wounded;
    // C-SS-MIN-AGE-AI: live MinAgeToEat for profession peer counts
    let min_age = if state.gameplay.min_age_to_eat.is_finite() && state.gameplay.min_age_to_eat >= 0.0 {
        state.gameplay.min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };

    match kind {
        ProfessionScanKind::Farm => {
            let peers = farm_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            // Any sticky farm last counts as a farm peer (job-specific via count_farm_peers_for_job).
            peers
                .iter()
                .filter(|p| p.eligible_for_count(min_age, MAX_AGE))
                .filter(|p| p.last_farm.is_some())
                .count() as f32
        }
        ProfessionScanKind::Smith => {
            let peers = smith_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            count_smith_peers_filtered(&peers, min_age, MAX_AGE)
        }
        ProfessionScanKind::Baker => {
            let peers = baker_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            count_baker_peers_filtered(&peers, min_age, MAX_AGE)
        }
        ProfessionScanKind::Shepherd => {
            let peers = shepherd_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            count_shepherd_peers_filtered(&peers, min_age, MAX_AGE)
        }
        ProfessionScanKind::Pottery => {
            let peers = potter_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            count_potter_peers_filtered(&peers, min_age, MAX_AGE)
        }
        // AI-FIREFOOD-RUNG: FIREFOODMAKER peer filter
        ProfessionScanKind::FireFood => {
            let peers = fire_food_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            crate::count_fire_food_peers_filtered(&peers, min_age, MAX_AGE)
        }
        // AI-HANDLING-FIRE: FIREKEEPER peers reuse fire-food sticky filter (same home radius)
        ProfessionScanKind::HandlingFire => {
            let peers = fire_food_peers_from_players_ex(
                state.players.values(),
                self_conn_id,
                home_x,
                home_y,
                Some(following),
                Some(is_wounded_ref),
            );
            crate::count_fire_food_peers_filtered(&peers, min_age, MAX_AGE)
        }
    }
}

/// Build fire-food peer snapshots from other players at same home.
// Haxe: AiBase.countProfession('FIREFOODMAKER')
pub fn fire_food_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<crate::FireFoodPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    fire_food_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Fire-food peers with wound/follow filters.
// Haxe: countProfession isWounded / playerToFollow
pub fn fire_food_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<crate::FireFoodPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            crate::FireFoodPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_is_fire_food: p.fire_food_profession.is_last_fire_food,
            }
        })
        .collect()
}

/// Build potter peer snapshots from other players at same home.
// Haxe: AiBase.countProfession('POTTER')
pub fn potter_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<PotterPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    potter_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Potter peers with wound/follow filters.
// Haxe: countProfession isWounded / playerToFollow
pub fn potter_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<PotterPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            PotterPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_is_potter: p.pottery_profession.is_last_potter,
            }
        })
        .collect()
}

/// Build shepherd peer snapshots from other players at same home.
// Haxe: AiBase.countProfession('SHEPHERD')
pub fn shepherd_peers_from_players<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
) -> Vec<ShepherdPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    shepherd_peers_from_players_ex(players, self_conn_id, home_x, home_y, None, None)
}

/// Shepherd peers with wound/follow filters.
// Haxe: countProfession isWounded / playerToFollow
pub fn shepherd_peers_from_players_ex<'a, I>(
    players: I,
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    following: Option<&HashMap<i32, i32>>,
    is_wounded: Option<&dyn Fn(&crate::Player) -> bool>,
) -> Vec<ShepherdPeerSnapshot>
where
    I: IntoIterator<Item = &'a crate::Player>,
{
    players
        .into_iter()
        .filter(|p| p.conn_id != self_conn_id)
        .map(|p| {
            let same_home = p.home_x == home_x && p.home_y == home_y;
            let flags = peer_roster_flags_pure(p, following, false);
            let wounded = is_wounded.map(|f| f(p)).unwrap_or(flags.is_wounded);
            ShepherdPeerSnapshot {
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                has_player_to_follow: flags.has_player_to_follow,
                same_home,
                last_is_shepherd: p.shepherd_profession.is_last_shepherd,
            }
        })
        .collect()
}

// ── Priority ladder → profession scan (NPC-CRAFT-LADDER) ────────────────────

/// Sticky farm/smith/baker/shepherd flags used to plan scan steps and live sensor job bits.
// Haxe: AiBase assignedProfession / lastProfession / isLastSmith / isLastBaker / SHEPHERD
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProfessionStickySnapshot {
    pub farm_assigned: Option<FarmProfession>,
    pub farm_last: Option<FarmProfession>,
    pub smith_assigned: bool,
    pub smith_last: bool,
    pub baker_assigned: bool,
    pub baker_last: bool,
    pub pottery_assigned: bool,
    pub pottery_last: bool,
    pub shepherd_assigned: bool,
    pub shepherd_last: bool,
    /// Haxe assignedProfession == 'FIREFOODMAKER' (AI-FIREFOOD-RUNG).
    pub fire_food_assigned: bool,
    /// Haxe lastProfession == 'FIREFOODMAKER'.
    pub fire_food_last: bool,
    pub fire_keeper_assigned: bool,
    pub fire_keeper_last: bool,
    pub age: f32,
}

impl ProfessionStickySnapshot {
    /// Build from live player sticky runtimes.
    // Haxe: GlobalPlayerInstance + AiBase profession sticky fields
    pub fn from_runtimes(
        farm: &FarmProfessionRuntime,
        smith: &SmithProfessionRuntime,
        baker: &BakerProfessionRuntime,
        age: f32,
    ) -> Self {
        Self::from_runtimes_ex(farm, smith, baker, None, None, None, None, age)
    }

    /// Build including optional shepherd + pottery + fire-food sticky.
    pub fn from_runtimes_ex(
        farm: &FarmProfessionRuntime,
        smith: &SmithProfessionRuntime,
        baker: &BakerProfessionRuntime,
        shepherd: Option<&ShepherdProfessionRuntime>,
        pottery: Option<&PotterProfessionRuntime>,
        fire_food: Option<&crate::FireFoodProfessionRuntime>,
        fire_keeper: Option<&crate::FireKeeperProfessionRuntime>,
        age: f32,
    ) -> Self {
        let (shepherd_assigned, shepherd_last) = shepherd
            .map(|s| (s.is_assigned_shepherd, s.is_last_shepherd))
            .unwrap_or((false, false));
        let (pottery_assigned, pottery_last) = pottery
            .map(|p| (p.is_assigned_potter, p.is_last_potter))
            .unwrap_or((false, false));
        let (fire_food_assigned, fire_food_last) = fire_food
            .map(|f| (f.is_assigned_fire_food, f.is_last_fire_food))
            .unwrap_or((false, false));
        let (fire_keeper_assigned, fire_keeper_last) = fire_keeper
            .map(|f| (f.is_assigned_fire_keeper, f.is_last_fire_keeper))
            .unwrap_or((false, false));
        Self {
            farm_assigned: farm.assigned_profession,
            farm_last: farm.last_profession,
            smith_assigned: smith.is_assigned_smith,
            smith_last: smith.is_last_smith,
            baker_assigned: baker.is_assigned_baker,
            baker_last: baker.is_last_baker,
            pottery_assigned,
            pottery_last,
            shepherd_assigned,
            shepherd_last,
            fire_food_assigned,
            fire_food_last,
            fire_keeper_assigned,
            fire_keeper_last,
            age,
        }
    }

    pub fn has_assigned_job(self) -> bool {
        self.farm_assigned.is_some()
            || self.smith_assigned
            || self.baker_assigned
            || self.pottery_assigned
            || self.shepherd_assigned
            || self.fire_food_assigned
            || self.fire_keeper_assigned
    }

    pub fn has_sticky_profession(self) -> bool {
        self.has_assigned_job()
            || self.farm_last.is_some()
            || self.smith_last
            || self.baker_last
            || self.pottery_last
            || self.shepherd_last
            || self.fire_food_last
            || self.fire_keeper_last
    }

    /// Adults always try the age-rotated job cycle when the ladder reaches it.
    // Haxe: doTimeStuffHelper for (i in 0...5) jobByAge — always attempted past food/combat
    pub fn age_job_pending(self) -> bool {
        self.age_job_pending_ex(MIN_AGE_TO_EAT)
    }

    /// Same as [`Self::age_job_pending`] with live MinAgeToEat.
    // Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
    pub fn age_job_pending_ex(self, min_age_to_eat: f32) -> bool {
        let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
            min_age_to_eat
        } else {
            MIN_AGE_TO_EAT
        };
        self.age >= min_age
    }
}

/// Job-band sensor flags derived from sticky profession runtime.
// Haxe: AssignedJob / AgeRotatedJob sensor gates in doTimeStuffHelper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProfessionJobSensorFlags {
    pub has_assigned_job: bool,
    pub age_job_pending: bool,
    /// Early sticky smith critical shortCrafts (hammer/bloom) when last/assigned smith.
    // Haxe: early doSmithing sticky before mid craft queue
    pub critical_craft_pending: bool,
}

/// Pure: sticky runtimes → ladder job sensors (no world I/O).
// Haxe: AiBase.doTimeStuffHelper assignedProfession / lastProfession / jobByAge gates
pub fn job_sensor_flags_from_sticky(sticky: &ProfessionStickySnapshot) -> ProfessionJobSensorFlags {
    job_sensor_flags_from_sticky_ex(sticky, MIN_AGE_TO_EAT)
}

/// Same as [`job_sensor_flags_from_sticky`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn job_sensor_flags_from_sticky_ex(
    sticky: &ProfessionStickySnapshot,
    min_age_to_eat: f32,
) -> ProfessionJobSensorFlags {
    ProfessionJobSensorFlags {
        has_assigned_job: sticky.has_assigned_job(),
        age_job_pending: sticky.age_job_pending_ex(min_age_to_eat),
        // Early sticky smith critical crafts when already a smith.
        critical_craft_pending: sticky.smith_assigned || sticky.smith_last,
    }
}

/// Write sticky job flags into a [`LiveSensorInput`] (in-place).
// Haxe: sensors for AssignedJob / AgeRotatedJob / CriticalCraft
pub fn apply_job_flags_to_live_input(input: &mut LiveSensorInput, sticky: &ProfessionStickySnapshot) {
    apply_job_flags_to_live_input_ex(input, sticky, MIN_AGE_TO_EAT);
}

/// Same as [`apply_job_flags_to_live_input`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn apply_job_flags_to_live_input_ex(
    input: &mut LiveSensorInput,
    sticky: &ProfessionStickySnapshot,
    min_age_to_eat: f32,
) {
    let f = job_sensor_flags_from_sticky_ex(sticky, min_age_to_eat);
    input.has_assigned_job = f.has_assigned_job;
    input.age_job_pending = f.age_job_pending;
    input.critical_craft_pending = f.critical_craft_pending;
    input.min_age_to_eat = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
}

/// One profession scan step planned from a ladder rung.
// Haxe: AssignedJob block / jobByAge i%5 branch → doBasicFarming / doSmithing / doBaking
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfessionLadderStep {
    pub kind: ProfessionScanKind,
    /// Label passed to try_decide_*_from_rung (`ASSIGNED_JOB` / `AGE_ROTATED_JOB` / `EARLY_STICKY_SMITH`).
    pub rung_label: &'static str,
    pub farm_job: Option<FarmProfession>,
    pub farm_has_profession: bool,
    pub is_assigned_job: bool,
    pub profession_is_sticky: bool,
}

/// Map age-rotated job kind → scan kind + optional farm job (pottery residual None).
// Haxe: jobByAge 0 berry / 1 basic / 2 baking / 3 pottery / 4 sheep
pub fn age_rotated_to_scan_step(kind: AgeRotatedJobKind) -> Option<ProfessionLadderStep> {
    match kind {
        AgeRotatedJobKind::BerryFarming => Some(ProfessionLadderStep {
            kind: ProfessionScanKind::Farm,
            rung_label: "AGE_ROTATED_JOB",
            farm_job: Some(FarmProfession::BerryFarmer),
            farm_has_profession: true,
            is_assigned_job: false,
            profession_is_sticky: true,
        }),
        AgeRotatedJobKind::BasicFarming => Some(ProfessionLadderStep {
            kind: ProfessionScanKind::Farm,
            rung_label: "AGE_ROTATED_JOB",
            farm_job: Some(FarmProfession::BasicFarmer),
            farm_has_profession: true,
            is_assigned_job: false,
            profession_is_sticky: true,
        }),
        AgeRotatedJobKind::Baking => Some(ProfessionLadderStep {
            kind: ProfessionScanKind::Baker,
            rung_label: "AGE_ROTATED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: false,
            profession_is_sticky: true,
        }),
        // Haxe: jobByAge == 3 → doPottery() (NPC-SCAN-FULL)
        AgeRotatedJobKind::Pottery => Some(ProfessionLadderStep {
            kind: ProfessionScanKind::Pottery,
            rung_label: "AGE_ROTATED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: false,
            profession_is_sticky: true,
        }),
        // Haxe: jobByAge == 4 → isSheepHerding()
        AgeRotatedJobKind::SheepHerding => Some(ProfessionLadderStep {
            kind: ProfessionScanKind::Shepherd,
            rung_label: "AGE_ROTATED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: false,
            profession_is_sticky: true,
        }),
    }
}

/// Plan scan steps for an AssignedJob rung from sticky assignment.
// Haxe: assignedProfession BASICFARMER/SMITH/BAKER/SHEPHERD → do*(100)
pub fn plan_assigned_job_steps(sticky: &ProfessionStickySnapshot) -> Vec<ProfessionLadderStep> {
    let mut out = Vec::with_capacity(4);
    if let Some(job) = sticky.farm_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Farm,
            rung_label: "ASSIGNED_JOB",
            farm_job: Some(job),
            farm_has_profession: true,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    if sticky.smith_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Smith,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    if sticky.baker_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Baker,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    if sticky.pottery_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Pottery,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    if sticky.shepherd_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Shepherd,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    // Haxe: assignedProfession == 'FIREFOODMAKER' → makeFireFood(100)
    if sticky.fire_food_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::FireFood,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    // Haxe: assignedProfession == 'FIREKEEPER' → isHandlingFire(100)
    if sticky.fire_keeper_assigned {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::HandlingFire,
            rung_label: "ASSIGNED_JOB",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: true,
            profession_is_sticky: true,
        });
    }
    // Sticky last without explicit assigned still works assigned-weight via last.
    if out.is_empty() {
        if let Some(job) = sticky.farm_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::Farm,
                rung_label: "ASSIGNED_JOB",
                farm_job: Some(job),
                farm_has_profession: true,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        if sticky.smith_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::Smith,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        if sticky.baker_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::Baker,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        if sticky.pottery_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::Pottery,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        if sticky.shepherd_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::Shepherd,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        // Haxe: lastProfession == 'FIREFOODMAKER' → makeFireFood(100)
        if sticky.fire_food_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::FireFood,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
        // Haxe: lastProfession == 'FIREKEEPER' → isHandlingFire(100)
        if sticky.fire_keeper_last {
            out.push(ProfessionLadderStep {
                kind: ProfessionScanKind::HandlingFire,
                rung_label: "ASSIGNED_JOB",
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: true,
                profession_is_sticky: true,
            });
        }
    }
    out
}

/// Plan age-rotated scan steps (Haxe for-loop over 5 job slots, pottery/sheep skipped).
// Haxe: for (i in 0...5) (jobByAge+i)%5
pub fn plan_age_rotated_steps(age: f32) -> Vec<ProfessionLadderStep> {
    age_rotated_job_sequence(age)
        .into_iter()
        .filter_map(age_rotated_to_scan_step)
        .collect()
}

/// Plan steps for CriticalCraft: early sticky smith + critical shortCraft tails.
// Haxe: lastProfession==SMITH doSmithing ~609; critical shortCrafts ~617–621 (all AIs)
// AI-JOB-SMITH-RESID: CriticalCraft tails (CRITICAL_CRAFT slot without profession)
pub fn plan_critical_craft_steps(sticky: &ProfessionStickySnapshot) -> Vec<ProfessionLadderStep> {
    let mut out = Vec::new();
    // Haxe: early sticky doSmithing before bloom/tongs shortCrafts
    if sticky.smith_assigned || sticky.smith_last {
        out.push(ProfessionLadderStep {
            kind: ProfessionScanKind::Smith,
            rung_label: "EARLY_STICKY_SMITH",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: sticky.smith_assigned,
            profession_is_sticky: true,
        });
    }
    // Haxe: critical smith shortCrafts (441+309 / 33+309 / 320+304) — no profession gate
    out.push(ProfessionLadderStep {
        kind: ProfessionScanKind::Smith,
        rung_label: "CRITICAL_CRAFT",
        farm_job: None,
        farm_has_profession: false,
        is_assigned_job: false,
        profession_is_sticky: sticky.smith_assigned || sticky.smith_last,
    });
    out
}

/// Plan profession scan steps for a resolved priority rung.
// Haxe: doTimeStuffHelper AssignedJob / CriticalCraft / AgeRotatedJob branches
pub fn plan_profession_ladder_steps(
    rung: PriorityRung,
    sticky: &ProfessionStickySnapshot,
) -> Vec<ProfessionLadderStep> {
    match rung {
        PriorityRung::AssignedJob => plan_assigned_job_steps(sticky),
        PriorityRung::AgeRotatedJob | PriorityRung::LowPriorityWork => {
            plan_age_rotated_steps(sticky.age)
        }
        PriorityRung::CriticalCraft => plan_critical_craft_steps(sticky),
        // Mid/misc job bands: prefer sticky assigned, else age-rotated fallthrough.
        // AI-HANDLING-FIRE: Haxe mid isHandlingFire() before other mid work (~634)
        PriorityRung::MidPriorityTasks | PriorityRung::CriticalMisc => {
            let mut steps = vec![ProfessionLadderStep {
                kind: ProfessionScanKind::HandlingFire,
                rung_label: match rung {
                    PriorityRung::CriticalMisc => "CRITICAL_MISC",
                    _ => "MID_PRIORITY_TASKS",
                },
                farm_job: None,
                farm_has_profession: false,
                is_assigned_job: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
                profession_is_sticky: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
            }];
            steps.extend(plan_assigned_job_steps(sticky));
            if steps.len() == 1 {
                steps.extend(plan_age_rotated_steps(sticky.age));
            }
            steps
        }
        // Haxe: handleTemperature cold fail → isHandlingFire(2) ~1740
        PriorityRung::Temperature => vec![ProfessionLadderStep {
            kind: ProfessionScanKind::HandlingFire,
            rung_label: "TEMPERATURE",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
            profession_is_sticky: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
        }],
        // Haxe: isConsideringMakingFood early isHandlingFire() ~8540; makeFireFood(1) residual via late tick
        PriorityRung::ConsiderMakeFood => vec![ProfessionLadderStep {
            kind: ProfessionScanKind::HandlingFire,
            rung_label: "CONSIDER_MAKE_FOOD",
            farm_job: None,
            farm_has_profession: false,
            is_assigned_job: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
            profession_is_sticky: sticky.fire_keeper_assigned || sticky.fire_keeper_last,
        }],
        _ => Vec::new(),
    }
}

/// True when live intent is a tile USE/DROP (ready for NetIntent / apply).
#[inline]
pub fn live_intent_is_wire(intent: ShortCraftLiveIntent) -> bool {
    intent.is_wire_action()
}

// AI-MAKE-STUFF: make_stuff_scan_tick + fire_food_action_to_live_intent
include!("make_stuff_live.inc.rs");
// AI-HANDLING-FIRE
include!("handling_fire_live.inc.rs");
// AI-HANDLING-FIRE
// AI-HANDLING-FIRE: handling_fire_profession_scan_tick + late_make_fire_food_scan_tick

/// Run planned ladder steps until one yields a non-None intent (prefer wire USE/DROP).
///
/// Pure — mutates profession task/runtime sticky state like Haxe do* bodies.
// Haxe: AssignedJob/AgeRotated → doBasicFarming/doSmithing/doBaking → shortCraft
pub fn ladder_profession_scan_tick(
    rung: PriorityRung,
    tiles: &[ScanTile],
    base_inp: &ProfessionScanInput,
    sticky: &ProfessionStickySnapshot,
    farm_task: &mut FarmTaskState,
    farm_rt: &mut FarmProfessionRuntime,
    smith_rt: &mut SmithProfessionRuntime,
    baker_rt: &mut BakerProfessionRuntime,
    baker_task: &mut BakerTaskState,
    shepherd_rt: &mut ShepherdProfessionRuntime,
    pottery_rt: &mut PotterProfessionRuntime,
    fire_rt: &mut crate::FireFoodProfessionRuntime,
    fire_keeper_rt: &mut crate::FireKeeperProfessionRuntime,
) -> ProfessionScanTickResult {
    let steps = plan_profession_ladder_steps(rung, sticky);
    let mut staging = ProfessionScanTickResult::none();
    for step in steps {
        // ProfessionScanInput not Copy (chisel_family_extra Vec) — clone per step
        let mut inp = base_inp.clone();
        inp.is_assigned_job = step.is_assigned_job;
        inp.profession_is_sticky = step.profession_is_sticky;
        // AI-FARM-STICKY: refresh weight from sticky runtime each step
        inp.basic_farmer_weight = basic_farmer_weight_from_runtime(farm_rt);
        let r = profession_scan_tick(
            step.kind,
            tiles,
            &inp,
            step.rung_label,
            step.farm_job,
            farm_task,
            step.farm_has_profession,
            farm_rt,
            smith_rt,
            baker_rt,
            baker_task,
            shepherd_rt,
            pottery_rt,
            fire_rt,
            fire_keeper_rt,
        );
        if !r.had_action {
            continue;
        }
        if live_intent_is_wire(r.intent) {
            return r;
        }
        // Haxe: dropHeld isMoving return true — hold tick, do not fall through (PREFER-SHORT-WAIT)
        if matches!(r.intent, ShortCraftLiveIntent::Wait) {
            return r;
        }
        // Keep first staging intent (SeekOrCraft / CraftItem / Defer*) if no wire yet.
        if matches!(staging.intent, ShortCraftLiveIntent::None) {
            staging = r;
        }
    }
    // Haxe: late makeFireFood(1) before makeStuff (~833); also critical ~6107 / hungry residual.
    // AI-HANDLING-FIRE: maxPeople=1 peer-capped residual after empty ladder steps.
    if matches!(staging.intent, ShortCraftLiveIntent::None)
        && matches!(
            rung,
            PriorityRung::LowPriorityWork
                | PriorityRung::AgeRotatedJob
                | PriorityRung::MidPriorityTasks
                | PriorityRung::CriticalMisc
                | PriorityRung::ConsiderMakeFood
        )
    {
        let mut inp = base_inp.clone();
        inp.basic_farmer_weight = basic_farmer_weight_from_runtime(farm_rt);
        let r = late_make_fire_food_scan_tick(tiles, &inp, fire_rt);
        if r.had_action {
            return r;
        }
    }
    // Haxe: late makeStuff() after age-rotated / low-priority job fallthrough.
    // AI-SHEPHERD-MID + AI-MAKE-STUFF live wire for makeStuff (bake+fire bodies).
    if matches!(staging.intent, ShortCraftLiveIntent::None)
        && matches!(
            rung,
            PriorityRung::LowPriorityWork
                | PriorityRung::AgeRotatedJob
                | PriorityRung::MidPriorityTasks
        )
    {
        let mut inp = base_inp.clone();
        inp.basic_farmer_weight = basic_farmer_weight_from_runtime(farm_rt);
        let r = make_stuff_scan_tick(
            tiles,
            &inp,
            farm_task,
            farm_rt,
            shepherd_rt,
            baker_rt,
            baker_task,
            fire_rt,
        );
        if r.had_action {
            return r;
        }
    }
    staging
}

/// Scan world + run ladder profession steps + apply USE/DROP for one AI player.
// Haxe: ServerAi.doTimeStuff → AiBase.doTimeStuffHelper job/craft profession branches
pub fn apply_profession_ladder_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    rung: PriorityRung,
) -> ShortCraftLiveApplyResult {
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        px
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        py
    };
    let held_id = p.held_id;
    let held_uses = p.held_uses;
    let food_store = p.food;
    let age = p.age;
    let sticky = ProfessionStickySnapshot::from_runtimes_ex(
        &p.farm_profession,
        &p.smith_profession,
        &p.baker_profession,
        Some(&p.shepherd_profession),
        Some(&p.pottery_profession),
        Some(&p.fire_food_profession),
        Some(&p.fire_keeper_profession),
        age,
    );
    let steps = plan_profession_ladder_steps(rung, &sticky);
    if steps.is_empty() {
        return ShortCraftLiveApplyResult::Failed;
    }
    // Scan radius covers all planned kinds.
    let scan_r = steps
        .iter()
        .map(|s| match s.kind {
            ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,
            ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,
            ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,
            ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,
            ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,
            ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,
            ProfessionScanKind::HandlingFire => crate::FIRE_FOOD_HOME_RADIUS,
        })
        .max()
        .unwrap_or(DEFAULT_PROFESSION_SCAN_RADIUS);

    let mut tiles = {
        let world = state.world.read().unwrap();
        // Haxe: pottery dual home craft + player clay r=80 when ladder includes Pottery
        if steps.iter().any(|s| s.kind == ProfessionScanKind::Pottery) {
            pottery_scan_tiles_from_world(
                &world,
                Some(&state.content),
                home_x,
                home_y,
                px,
                py,
            )
        } else {
            scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)
        }
    };

    // PATH-REACH: filter notReachableObjects / hostile / blockedByAI before profession picks.
    // Haxe: GetClosestObjectToPositionHelper isObjectNotReachable skip
    {
        let p = state.players.get(&conn_id).expect("player checked above");
        let filters = path_filters_from_player(&p.ai_path_reach, &state.blocked_by_ai);
        tiles = apply_path_filters_to_tiles(&tiles, &filters);
    }

    let has_carrot_seeds = has_carrot_seeds_from_scan(&tiles);
    let has_bean_seeds = has_bean_seeds_from_scan(&tiles);
    // Peer count for primary step kind (assigned first, else first planned).
    let primary_kind = steps[0].kind;
    let peer_count = peer_count_for_kind(primary_kind, state, conn_id, home_x, home_y);
    let profession_is_sticky = sticky.has_sticky_profession();
    let was_idle = if profession_is_sticky { 0.0 } else { 1.0 };

    // Haxe: heldObject.containedObjects.length + contains([126]) via held_helper
    // PREFER-SHORT-WAIT: isMoving for dropHeld BusyMoving → Wait
    let (held_contained, held_contains_clay, is_moving) = state
        .players
        .get(&conn_id)
        .map(|p| {
            (
                held_contained_from_player(p),
                held_contains_clay_from_player(p),
                p.moving || p.move_path.is_some(),
            )
        })
        .unwrap_or((0, held_id == crate::pottery_profession::CLAY, false));

    // Haxe: TimeHelper.Season == Winter (isHandlingFire Fire 82 kindling first)
    let is_winter = matches!(
        state.environment.season,
        crate::environment::Season::Winter
    );
    let base_inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        transition_hungry_cost: 0.0, // residual: content-pair hungry cost
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job: sticky.has_assigned_job(),
        // PREFER-SHORT-WAIT: dropHeld isMoving → Wait
        is_moving,
        is_winter,
        // AI-JOB-SMITH-RESID: objectIdArrays[455] content Chisel scan
        chisel_family_extra: chisel_family_extras_beyond_static(&steel_chisel_family_from_content(
            &state.content,
        )),
    };

    let Some(p) = state.players.get_mut(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let mut farm_task = p.farm_task.clone();
    let mut farm_rt = p.farm_profession.clone();
    let mut smith_rt = p.smith_profession.clone();
    let mut baker_rt = p.baker_profession.clone();
    let mut baker_task = p.baker_task.clone();
    let mut shepherd_rt = p.shepherd_profession.clone();
    let mut pottery_rt = p.pottery_profession.clone();
    let mut fire_rt = p.fire_food_profession.clone();
    let mut fire_keeper_rt = p.fire_keeper_profession.clone();

    // AI-FARM-STICKY: seed ProfessionScanInput from sticky BASICFARMER weight
    let mut base_inp = base_inp;
    base_inp.basic_farmer_weight = basic_farmer_weight_from_runtime(&farm_rt);

    let result = ladder_profession_scan_tick(
        rung,
        &tiles,
        &base_inp,
        &sticky,
        &mut farm_task,
        &mut farm_rt,
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut fire_keeper_rt,
    );

    if let Some(p) = state.players.get_mut(&conn_id) {
        p.farm_task = farm_task;
        p.farm_profession = farm_rt;
        p.smith_profession = smith_rt;
        p.baker_profession = baker_rt;
        p.baker_task = baker_task;
        p.shepherd_profession = shepherd_rt;
        p.pottery_profession = pottery_rt;
        p.fire_food_profession = fire_rt;
        p.fire_keeper_profession = fire_keeper_rt;
    }

    // PATH-REACH / AI-FOOD-FAIL-MARK: failed USE → food 30s or age-gated (Haxe ~8698 / ~9133).
    let intent = result.intent;
    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
    if matches!(apply_r, ShortCraftLiveApplyResult::Failed)
        || matches!(&apply_r, ShortCraftLiveApplyResult::Used(r) if !r.applied)
    {
        if let ShortCraftLiveIntent::UseAt { x, y, .. }
        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } = intent
        {
            crate::mark_path_fail_after_use_live(state, conn_id, x, y);
        }
    }
    apply_r
}

/// Convenience: resolve rung from sticky job flags + vitals, then apply ladder scan.
///
/// Food/threat rungs outrank jobs — when Escape/PickupFood/Eating win, returns Failed
/// without profession work (caller keeps higher-band behavior).
// Haxe: doTimeStuffHelper full order; profession only when AssignedJob/AgeRotated/…
// Haxe: calledCraftItem=false each entry; sticky itemToCraftId + craftingTasks → CraftQueue
pub fn apply_profession_scan_from_sensors(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    threat_near: bool,
    nearby_food: bool,
) -> (PriorityRung, ShortCraftLiveApplyResult) {
    use crate::resolve_priority_rung;
    use crate::sensors_from_ext_ex;
    use crate::LiveSensorExtras;

    // AI-CRAFT-STICKY: per-tick calledCraftItem guard (Haxe doTimeStuffHelper entry)
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.craft_ai_begin_tick();
    }

    let Some(p) = state.players.get(&conn_id) else {
        return (PriorityRung::Idle, ShortCraftLiveApplyResult::Failed);
    };
    let sticky = ProfessionStickySnapshot::from_runtimes_ex(
        &p.farm_profession,
        &p.smith_profession,
        &p.baker_profession,
        Some(&p.shepherd_profession),
        Some(&p.pottery_profession),
        Some(&p.fire_food_profession),
        Some(&p.fire_keeper_profession),
        p.age,
    );
    // C-SS-MIN-AGE-AI: live MinAgeToEat for age_job_pending + food/child sensors
    let min_age = state.gameplay.min_age_to_eat;
    let job = job_sensor_flags_from_sticky_ex(&sticky, min_age);
    let craft_flags = p.craft_ai.sticky_craft_sensor_flags();
    let mut critical_craft_pending = job.critical_craft_pending;
    let mut has_craft_queue = false;
    crate::apply_sticky_flags_to_craft_sensors(
        craft_flags.unfinished_sticky,
        craft_flags.has_craft_queue,
        &mut critical_craft_pending,
        &mut has_craft_queue,
    );
    let extras = LiveSensorExtras {
        has_assigned_job: job.has_assigned_job,
        age_job_pending: job.age_job_pending,
        critical_craft_pending,
        has_craft_queue,
        ..Default::default()
    };
    let sensors = sensors_from_ext_ex(
        p.held_id,
        p.food,
        threat_near,
        nearby_food,
        p.age,
        false,
        p.food_max,
        false,
        &extras,
        min_age,
    );
    let rung = resolve_priority_rung(&sensors);
    match rung {
        // Haxe: itemToCraftId continue + craftingTasks round-robin before clothing/job
        PriorityRung::CraftQueue => {
            let r = apply_sticky_craft_queue_tick(state, outbound, conn_id);
            (rung, r)
        }
        PriorityRung::AssignedJob
        | PriorityRung::AgeRotatedJob
        | PriorityRung::CriticalCraft
        | PriorityRung::LowPriorityWork
        | PriorityRung::MidPriorityTasks
        | PriorityRung::CriticalMisc
        // AI-HANDLING-FIRE: temperature isHandlingFire(2); hungry early isHandlingFire()
        | PriorityRung::Temperature
        | PriorityRung::ConsiderMakeFood => {
            let r = apply_profession_ladder_tick(state, outbound, conn_id, rung);
            (rung, r)
        }
        other => (other, ShortCraftLiveApplyResult::Failed),
    }
}

/// Drive sticky multi-tick craft for CraftQueue rung (Haxe ~667–680).
// Haxe: craftItem(itemToCraftId) continue; craftingTasks.shift/push on fail
pub fn apply_sticky_craft_queue_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    use crate::get_or_craft::world_objs_from_ids;
    use crate::resolve_smith_assigned_job;
    use crate::short_craft_intent::{apply_short_craft_live_intent, ShortCraftLiveIntent};
    use crate::CraftLiveExpandOpts;
    use crate::CraftScanFilters;
    use crate::{
        expand_craft_item_player_sticky_scan, select_sticky_craft_for_tick, StickyCraftTickChoice,
    };

    let Some(p) = state.players.get_mut(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    // begin_tick already at sensor entry; select again is idempotent for guard
    let choice = select_sticky_craft_for_tick(&mut p.craft_ai);
    let Some(product_id) = choice.product_id() else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let held_id = p.held_id;
    let home = if p.home_x != 0 || p.home_y != 0 {
        Some((p.home_x, p.home_y))
    } else {
        None
    };
    let is_smith = resolve_smith_assigned_job(&p.smith_profession);
    let now_sec = state.sim_time as f64;
    // PATH-REACH / AI-CRAFT-LIVE-RESID: notReachable + hostile + blockedByAI into craft scan.
    // Haxe: addObjectsForCrafting isObjectNotReachable; GetClosestObject* hostile
    let blocked = p
        .ai_path_reach
        .blocked_coords(Some(&state.blocked_by_ai));

    // Local scan around player for craft staging (Haxe maxSearchRadius ~40–60).
    let radius = 40i32;
    let mut items: Vec<(i32, i32, i32)> = Vec::new();
    {
        let w = state.world.read().unwrap();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = px + dx;
                let y = py + dy;
                let id = w.get_object(x, y);
                if id != 0 {
                    items.push((id, x, y));
                }
            }
        }
    }
    let objs = world_objs_from_ids(&items, None);
    let graph = state.craft_graph.clone();
    let opts = CraftLiveExpandOpts {
        home,
        is_or_can_smith: is_smith,
        now_sec,
        use_default_water_sources: true,
    };
    // Pile map residual (AI-CRAFT-MULTI); 0 = no pile prefer.
    let pile_id_for = |_id: i32| 0i32;
    let scan = CraftScanFilters::new().with_blocked(&blocked);

    let intent = {
        let Some(p) = state.players.get_mut(&conn_id) else {
            return ShortCraftLiveApplyResult::Failed;
        };
        expand_craft_item_player_sticky_scan(
            product_id,
            &objs,
            px,
            py,
            held_id,
            &pile_id_for,
            Some((px, py)),
            &graph,
            &opts,
            &mut p.craft_ai,
            scan,
        )
    };

    if matches!(intent, ShortCraftLiveIntent::None) {
        if let Some(p) = state.players.get_mut(&conn_id) {
            let _ = p.craft_ai.on_craft_fail_from_choice(choice);
        }
        return ShortCraftLiveApplyResult::Failed;
    }

    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
    if matches!(apply_r, ShortCraftLiveApplyResult::Failed) {
        if let Some(p) = state.players.get_mut(&conn_id) {
            // Haxe: failed craftItem from queue → push back
            if matches!(choice, StickyCraftTickChoice::FromQueue { .. }) {
                p.craft_ai.requeue_current_task();
            }
        }
    }
    apply_r
}

// ── Unified dispatch ────────────────────────────────────────────────────────

/// Run one profession scan tick from pre-scanned tiles.
pub fn profession_scan_tick(
    kind: ProfessionScanKind,
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    farm_job: Option<FarmProfession>,
    farm_task: &mut FarmTaskState,
    farm_has_profession: bool,
    farm_rt: &mut FarmProfessionRuntime,
    smith_rt: &mut SmithProfessionRuntime,
    baker_rt: &mut BakerProfessionRuntime,
    baker_task: &mut BakerTaskState,
    shepherd_rt: &mut ShepherdProfessionRuntime,
    pottery_rt: &mut PotterProfessionRuntime,
    fire_rt: &mut crate::FireFoodProfessionRuntime,
    fire_keeper_rt: &mut crate::FireKeeperProfessionRuntime,
) -> ProfessionScanTickResult {
    match kind {
        ProfessionScanKind::Farm => farm_profession_scan_tick(
            tiles,
            inp,
            farm_job,
            rung_label,
            farm_task,
            farm_has_profession,
            farm_rt,
        ),
        ProfessionScanKind::Smith => smith_profession_scan_tick(tiles, inp, rung_label, smith_rt),
        ProfessionScanKind::Baker => {
            baker_profession_scan_tick(tiles, inp, rung_label, baker_rt, baker_task)
        }
        // NPC-SCAN-FULL: live shortCraft via pottery_profession_scan_tick
        ProfessionScanKind::Pottery => {
            pottery_profession_scan_tick(tiles, inp, rung_label, pottery_rt)
        }
        // AI-SHEPHERD: live shortCraft via shepherd_profession_scan_tick
        ProfessionScanKind::Shepherd => {
            shepherd_profession_scan_tick(tiles, inp, rung_label, shepherd_rt, farm_task)
        }
        // AI-FIREFOOD-RUNG: assigned/last makeFireFood(100)
        ProfessionScanKind::FireFood => {
            fire_food_profession_scan_tick(tiles, inp, rung_label, fire_rt)
        }
        ProfessionScanKind::HandlingFire => handling_fire_profession_scan_tick(
            tiles,
            inp,
            rung_label,
            fire_keeper_rt,
            fire_rt,
            baker_rt,
            baker_task,
        ),
    }
}

/// Scan live world around player's home (or position), decide profession action,
/// map to live intent, and apply USE/DROP when wire-capable.
// Haxe: doTimeStuffHelper profession branch → shortCraft → client USE/DROP
pub fn apply_profession_scan_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    kind: ProfessionScanKind,
    rung_label: &str,
) -> ShortCraftLiveApplyResult {
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        px
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        py
    };
    let held_id = p.held_id;
    let held_uses = p.held_uses;
    // Haxe: heldObject.containedObjects.length + contains([126]) via held_helper
    let held_contained = held_contained_from_player(p);
    let held_contains_clay = held_contains_clay_from_player(p);
    let food_store = p.food;
    let age = p.age;
    let farm_job = crate::farmer_profession::resolve_farm_assigned_job(&p.farm_profession);
    let farm_has =
        farm_job.is_some() || p.farm_profession.last_profession.is_some();
    let smith_sticky = p.smith_profession.is_last_smith || p.smith_profession.is_assigned_smith;
    let baker_sticky = p.baker_profession.is_last_baker || p.baker_profession.is_assigned_baker;
    let shepherd_sticky =
        p.shepherd_profession.is_last_shepherd || p.shepherd_profession.is_assigned_shepherd;
    let pottery_sticky =
        p.pottery_profession.is_last_potter || p.pottery_profession.is_assigned_potter;
    let fire_food_sticky = p.fire_food_profession.is_last_fire_food
        || p.fire_food_profession.is_assigned_fire_food;
    let is_assigned_job = match kind {
        ProfessionScanKind::Farm => p.farm_profession.assigned_profession.is_some(),
        ProfessionScanKind::Smith => p.smith_profession.is_assigned_smith,
        ProfessionScanKind::Baker => p.baker_profession.is_assigned_baker,
        ProfessionScanKind::Pottery => p.pottery_profession.is_assigned_potter,
        ProfessionScanKind::Shepherd => p.shepherd_profession.is_assigned_shepherd,
        // Haxe: assigned/last FIREFOODMAKER both use makeFireFood(100)
        ProfessionScanKind::FireFood => {
            p.fire_food_profession.is_assigned_fire_food
                || p.fire_food_profession.is_last_fire_food
        }
        ProfessionScanKind::HandlingFire => {
            p.fire_keeper_profession.is_assigned_fire_keeper
                || p.fire_keeper_profession.is_last_fire_keeper
        }
    };
    let profession_is_sticky = match kind {
        ProfessionScanKind::Farm => farm_has,
        ProfessionScanKind::Smith => smith_sticky,
        ProfessionScanKind::Baker => baker_sticky,
        ProfessionScanKind::Pottery => pottery_sticky,
        ProfessionScanKind::Shepherd => shepherd_sticky,
        ProfessionScanKind::FireFood => fire_food_sticky,
        ProfessionScanKind::HandlingFire => {
            p.fire_keeper_profession.is_last_fire_keeper
                || p.fire_keeper_profession.is_assigned_fire_keeper
        }
    };

    let mut tiles = {
        let world = state.world.read().unwrap();
        match kind {
            // Haxe: gatherClay deposit/pit r=80 player + doPottery home craft r=30
            ProfessionScanKind::Pottery => pottery_scan_tiles_from_world(
                &world,
                Some(&state.content),
                home_x,
                home_y,
                px,
                py,
            ),
            other => {
                let scan_r = match other {
                    ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,
                    ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,
                    ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,
                    ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,
                    ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,
                    ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,
                    ProfessionScanKind::HandlingFire => crate::FIRE_FOOD_HOME_RADIUS,
                };
                scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)
            }
        }
    };

    // PATH-REACH: filter notReachableObjects / hostile / blockedByAI before profession picks.
    // Haxe: GetClosestObjectToPositionHelper isObjectNotReachable skip
    {
        let p = state.players.get(&conn_id).expect("player checked above");
        let filters = path_filters_from_player(&p.ai_path_reach, &state.blocked_by_ai);
        tiles = apply_path_filters_to_tiles(&tiles, &filters);
    }

    // Haxe: countSeeds / hasBeanSeeds from current objects near home scan.
    let has_carrot_seeds = has_carrot_seeds_from_scan(&tiles);
    let has_bean_seeds = has_bean_seeds_from_scan(&tiles);
    // Haxe: countProfession over peer AIs at same home.
    let peer_count = peer_count_for_kind(kind, state, conn_id, home_x, home_y);
    // Residual: wasIdle float decay from AiBase; use 1 when not sticky so caps expand once.
    let was_idle = if profession_is_sticky { 0.0 } else { 1.0 };

    let is_moving = state
        .players
        .get(&conn_id)
        .map(|p| p.moving || p.move_path.is_some())
        .unwrap_or(false);
    // Haxe: TimeHelper.Season == Winter (isHandlingFire Fire 82 kindling first)
    let is_winter = matches!(
        state.environment.season,
        crate::environment::Season::Winter
    );
    let inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        // Residual: pair hungry cost needs transition table lookup (CRAFT-LIVE-IO).
        transition_hungry_cost: 0.0,
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job,
        // PREFER-SHORT-WAIT: dropHeld isMoving → Wait
        is_moving,
        is_winter,
        // AI-JOB-SMITH-RESID: objectIdArrays[455] content Chisel scan
        chisel_family_extra: chisel_family_extras_beyond_static(&steel_chisel_family_from_content(
            &state.content,
        )),
    };

    let Some(p) = state.players.get_mut(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let mut farm_task = p.farm_task.clone();
    let mut farm_rt = p.farm_profession.clone();
    let mut smith_rt = p.smith_profession.clone();
    let mut baker_rt = p.baker_profession.clone();
    let mut baker_task = p.baker_task.clone();
    let mut shepherd_rt = p.shepherd_profession.clone();
    let mut pottery_rt = p.pottery_profession.clone();
    let mut fire_rt = p.fire_food_profession.clone();
    let mut fire_keeper_rt = p.fire_keeper_profession.clone();

    // AI-FARM-STICKY: seed + write back BASICFARMER weight on Player.farm_profession
    let mut inp = inp;
    inp.basic_farmer_weight = basic_farmer_weight_from_runtime(&farm_rt);

    let result = profession_scan_tick(
        kind,
        &tiles,
        &inp,
        rung_label,
        farm_job,
        &mut farm_task,
        farm_has,
        &mut farm_rt,
        &mut smith_rt,
        &mut baker_rt,
        &mut baker_task,
        &mut shepherd_rt,
        &mut pottery_rt,
        &mut fire_rt,
        &mut fire_keeper_rt,
    );

    if let Some(p) = state.players.get_mut(&conn_id) {
        p.farm_task = farm_task;
        p.farm_profession = farm_rt;
        p.smith_profession = smith_rt;
        p.baker_profession = baker_rt;
        p.baker_task = baker_task;
        p.shepherd_profession = shepherd_rt;
        p.pottery_profession = pottery_rt;
        p.fire_food_profession = fire_rt;
        p.fire_keeper_profession = fire_keeper_rt;
    }

    // PATH-REACH / AI-FOOD-FAIL-MARK: failed USE → food 30s or age-gated (Haxe ~8698 / ~9133).
    let intent = result.intent;
    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
    if matches!(apply_r, ShortCraftLiveApplyResult::Failed)
        || matches!(&apply_r, ShortCraftLiveApplyResult::Used(r) if !r.applied)
    {
        if let ShortCraftLiveIntent::UseAt { x, y, .. }
        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } = intent
        {
            crate::mark_path_fail_after_use_live(state, conn_id, x, y);
        }
    }
    apply_r
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    include!("profession_scan_tests.inc.rs");
}
