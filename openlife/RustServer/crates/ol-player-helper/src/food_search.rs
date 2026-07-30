//! Haxe `AiHelper.SearchBestFood` / `SearchBestFoodHelperNew` / `processFood`.
//!
//! Chunk: **SEARCH-BEST-FOOD** / `ai_food_search`
//!
//! Pure candidate scoring + conservation/seed gates (**PlayerHelper**).
//! Live world scan stays in `ol-sim` (`search_best_food_live.inc.rs` / adapters).
//!
//! // Haxe: openlife.auto.AiHelper.SearchBestFood (~658â€“1044)

use crate::food_eat_gates::{
    can_eat_obj_ex, can_feed_to_me_obj_ex_yum, is_obj_super_meh_ex, resolve_yum_bonus,
    starving_factor, YUM_BONUS,
};
use crate::geom::{calculate_distance_sq, in_count_close_square};

// â”€â”€ Object parent ids (Haxe processFood specials) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Cooked Goose â€” skip the first found (keep one for knife craft).
pub const COOKED_GOOSE_ID: i32 = 518;
/// Carrot.
pub const CARROT_ID: i32 = 402;
/// Carrot Pile.
pub const CARROT_PILE_ID: i32 = 2742;
/// Carrot Row (multi-use).
pub const CARROT_ROW_ID: i32 = 400;
/// Shucked Ear of Corn.
pub const SHUCKED_CORN_ID: i32 = 1114;
/// Pile of Shucked Corn.
pub const PILE_SHUCKED_CORN_ID: i32 = 3901;
/// Dried Ear of Corn (counts toward corn stock).
pub const DRIED_CORN_ID: i32 = 1115;
/// Pile of Dried Corn (stock pile form of 1115).
pub const PILE_DRIED_CORN_ID: i32 = 3902;
/// Hot Pepper.
pub const HOT_PEPPER_ID: i32 = 2844;
/// Fruiting Pepper Plant.
pub const FRUITING_PEPPER_ID: i32 = 2843;
/// Wild Onion.
pub const WILD_ONION_ID: i32 = 805;
/// Onion (808).
pub const ONION_ID: i32 = 808;
/// Ripe Onions (multi-use).
pub const RIPE_ONIONS_ID: i32 = 2854;

/// CountCloseObjects radius for carrot/corn stock checks (Haxe r=20).
pub const FOOD_STOCK_COUNT_RADIUS: i32 = 20;

/// Default SearchBestFood radius (same as [`ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS`] = 40).
pub const SEARCH_BEST_FOOD_RADIUS: i32 = ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS;

/// IsDangerous scan radius around food tile (Haxe default 4).
pub const FOOD_DANGER_RADIUS: i32 = 4;

/// Closed Wooden Chest / Locked Wooden Chest â€” Haxe `blocksRemove` patches.
/// Content has no `blocks_remove` field yet; hardcode ServerSettings patches.
pub const BLOCKS_REMOVE_CONTAINER_IDS: &[i32] = &[987, 988];

// WorldMap.getFoodFactor thresholds (ServerSettings defaults; live via FoodFactorEatenBands)
// Haxe: ServerSettings.FoodFactorEaten* â€” C-SS-FULL-TABLE LiveSettings
pub const FOOD_FACTOR_EATEN_LT_1: f32 = 2.5;
pub const FOOD_FACTOR_EATEN_LT_3: f32 = 2.0;
pub const FOOD_FACTOR_EATEN_LT_5: f32 = 1.5;
pub const FOOD_FACTOR_EATEN_GE_8: f32 = 0.8;
pub const FOOD_FACTOR_EATEN_GE_10: f32 = 0.5;

/// Live / override band table for [`food_factor_from_eaten_percentage_ex`].
// Haxe: ServerSettings.FoodFactorEatenLessThanOnePercent â€¦ MoreThanTenPercent
// C-SS-FULL-TABLE
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodFactorEatenBands {
    pub less_than_one_percent: f32,
    pub less_than_three_percent: f32,
    pub less_than_five_percent: f32,
    pub more_than_eight_percent: f32,
    pub more_than_ten_percent: f32,
}

impl Default for FoodFactorEatenBands {
    fn default() -> Self {
        Self {
            less_than_one_percent: FOOD_FACTOR_EATEN_LT_1,
            less_than_three_percent: FOOD_FACTOR_EATEN_LT_3,
            less_than_five_percent: FOOD_FACTOR_EATEN_LT_5,
            more_than_eight_percent: FOOD_FACTOR_EATEN_GE_8,
            more_than_ten_percent: FOOD_FACTOR_EATEN_GE_10,
        }
    }
}

// â”€â”€ AI seed / reach flags (Haxe ServerAi / AiBase) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Optional AI constraints when `player.getAi() != null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiFoodSearchFlags {
    pub has_carrot_seeds: bool,
    pub has_pepper_seeds: bool,
    pub has_onion_seeds: bool,
    pub has_corn_seeds: bool,
}

impl Default for AiFoodSearchFlags {
    fn default() -> Self {
        // Optimistic defaults: act as if seeds exist (human / DisplayBestFood path).
        Self {
            has_carrot_seeds: true,
            has_pepper_seeds: true,
            has_onion_seeds: true,
            has_corn_seeds: true,
        }
    }
}

impl AiFoodSearchFlags {
    /// Strict AI with no seeds â€” refuses seed-critical foods.
    pub fn no_seeds() -> Self {
        Self {
            has_carrot_seeds: false,
            has_pepper_seeds: false,
            has_onion_seeds: false,
            has_corn_seeds: false,
        }
    }
}

// â”€â”€ Pure inputs / hits â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// One ground/container stock tile for CountCloseObjects (tx, ty, parent_id, uses).
///
/// Single objects use `uses = 1`; piles contribute `uses` when matched as pile form.
pub type StockTile = (i32, i32, i32, i32);

/// One food object (ground or container slot) for scoring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchFoodCand {
    /// Ground/container object parent id (conservation/seed gates use this).
    pub parent_id: i32,
    /// Haxe `getFoodId` after foodFromTarget resolve (craving / hasEaten key).
    pub food_id: i32,
    /// foodValue from foodFromTarget or object (original).
    pub food_value: i32,
    pub tx: i32,
    pub ty: i32,
    /// hasEatenMap[foodId].
    pub count_eaten: f32,
    /// ObjectHelper.numberOfUses (multi-use remaining).
    pub number_of_uses: i32,
    /// Container slot index when from container; -1 for ground.
    pub index_in_container: i32,
    /// When true and AI + score-distance > 4 â†’ skip (IsDangerous).
    pub is_dangerous: bool,
    /// Pre-marked unreachable for AI (`isObjectNotReachable`).
    pub not_reachable: bool,
    /// WorldMap.getFoodFactor(foodId) for this candidate.
    pub food_factor: f32,
}

impl SearchFoodCand {
    /// Build a ground cand with food_id = parent_id and neutral food_factor.
    pub fn simple(
        parent_id: i32,
        food_value: i32,
        tx: i32,
        ty: i32,
        count_eaten: f32,
        number_of_uses: i32,
    ) -> Self {
        Self {
            parent_id,
            food_id: parent_id,
            food_value,
            tx,
            ty,
            count_eaten,
            number_of_uses,
            index_in_container: -1,
            is_dangerous: false,
            not_reachable: false,
            food_factor: 1.0,
        }
    }
}

/// Mutable counters shared across one SearchBestFood scan (Haxe `ctx`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchFoodCounters {
    pub goose_found: bool,
    /// -1 = not yet counted.
    pub count_carrots: i32,
    pub count_corn: i32,
}

impl SearchFoodCounters {
    pub fn new() -> Self {
        Self {
            goose_found: false,
            count_carrots: -1,
            count_corn: -1,
        }
    }
}

/// Best hit from SearchBestFood.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BestFoodHit {
    pub food_id: i32,
    pub food_value: i32,
    pub tx: i32,
    pub ty: i32,
    /// Haxe comparison distance (16 + quad + optional feeder).
    pub score_distance: f32,
    /// Scored food value after yum/meh/craving/factor (numerator).
    pub scored_food_value: f32,
    /// Raw quad dist from eater (no +16) for DisplayBestFood gates.
    pub raw_quad_dist: f32,
    pub index_in_container: i32,
}

/// Options for one processFood evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ProcessFoodOpts {
    pub base_x: i32,
    pub base_y: i32,
    pub food_store: f32,
    pub food_store_max: f32,
    pub currently_craving: i32,
    pub starving_factor: f32,
    /// Fallback WorldMap.getFoodFactor when cand.food_factor is 1.0-style neutral;
    /// multiplied with [`SearchFoodCand::food_factor`] (both default 1.0).
    pub food_factor: f32,
    /// Haxe `ServerSettings.YumBonus` for yum/meh band (live via GameplayKnobs).
    // Haxe: ServerSettings.YumBonus (YUM-LIVE-SETTINGS)
    pub yum_bonus: f32,
    pub feed_other: bool,
    pub feeding_tx: Option<i32>,
    pub feeding_ty: Option<i32>,
    /// None = human / no AI seed gates / no danger skip.
    pub ai: Option<AiFoodSearchFlags>,
    /// Map size for Haxe `CalculateDistance` torus wrap (0 = no wrap).
    pub map_w: i32,
    pub map_h: i32,
    pub wrap: bool,
    /// Eater has yellow fever (canFeedToMeObj 837 gate).
    pub has_yellow_fever: bool,
}

impl ProcessFoodOpts {
    pub fn human(
        base_x: i32,
        base_y: i32,
        food_store: f32,
        food_store_max: f32,
        currently_craving: i32,
    ) -> Self {
        Self {
            base_x,
            base_y,
            food_store,
            food_store_max,
            currently_craving,
            starving_factor: starving_factor(food_store),
            food_factor: 1.0,
            yum_bonus: YUM_BONUS,
            feed_other: false,
            feeding_tx: None,
            feeding_ty: None,
            ai: None,
            map_w: 0,
            map_h: 0,
            wrap: false,
            has_yellow_fever: false,
        }
    }
}

// â”€â”€ World food factor (pure) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `WorldMap.getFoodFactor` from eaten percentage (0â€“100 scale) with live bands.
// Haxe: WorldMap.getFoodFactor + ServerSettings.FoodFactorEaten*
// C-SS-FULL-TABLE
pub fn food_factor_from_eaten_percentage_ex(
    food_percentage: f32,
    bands: &FoodFactorEatenBands,
) -> f32 {
    if food_percentage < 1.0 {
        bands.less_than_one_percent
    } else if food_percentage < 3.0 {
        bands.less_than_three_percent
    } else if food_percentage < 5.0 {
        bands.less_than_five_percent
    } else if food_percentage >= 10.0 {
        bands.more_than_ten_percent
    } else if food_percentage >= 8.0 {
        bands.more_than_eight_percent
    } else {
        1.0
    }
}

/// Haxe `WorldMap.getFoodFactor` at default ServerSettings band constants.
// Haxe: WorldMap.getFoodFactor
pub fn food_factor_from_eaten_percentage(food_percentage: f32) -> f32 {
    food_factor_from_eaten_percentage_ex(food_percentage, &FoodFactorEatenBands::default())
}

/// Look up eaten percentage for `food_id`, rolling up `higher_quality` chain.
///
/// `eaten_pct`: `(food_id, percentage)` sparse map.
/// `higher_quality`: `(food_id, higher_quality_food_id)` edges (Haxe higherQaulityFood).
// Haxe: WorldMap.getEatenFoodPercentage + getFoodFactor
pub fn food_factor_for_id_ex(
    food_id: i32,
    eaten_pct: &[(i32, f32)],
    higher_quality: &[(i32, i32)],
    bands: &FoodFactorEatenBands,
) -> f32 {
    let pct = eaten_percentage_rollup(food_id, eaten_pct, higher_quality, 0);
    food_factor_from_eaten_percentage_ex(pct, bands)
}

/// Default-band [`food_factor_for_id_ex`].
pub fn food_factor_for_id(
    food_id: i32,
    eaten_pct: &[(i32, f32)],
    higher_quality: &[(i32, i32)],
) -> f32 {
    food_factor_for_id_ex(
        food_id,
        eaten_pct,
        higher_quality,
        &FoodFactorEatenBands::default(),
    )
}

fn eaten_percentage_rollup(
    food_id: i32,
    eaten_pct: &[(i32, f32)],
    higher_quality: &[(i32, i32)],
    depth: u8,
) -> f32 {
    if depth > 16 {
        return 0.0;
    }
    let mut pct = 0.0_f32;
    for &(id, p) in eaten_pct {
        if id == food_id {
            pct = p;
            break;
        }
    }
    let mut hq = 0;
    for &(id, next) in higher_quality {
        if id == food_id {
            hq = next;
            break;
        }
    }
    if hq > 0 {
        pct += eaten_percentage_rollup(hq, eaten_pct, higher_quality, depth + 1);
    }
    pct
}

/// Haxe `ObjectData.getFoodId` â€” parent, or foodFromTarget parent when set.
// Haxe: ObjectData.getFoodId
#[inline]
pub fn get_food_id(parent_id: i32, food_from_target_parent: Option<i32>) -> i32 {
    food_from_target_parent.unwrap_or(parent_id)
}

/// Resolve food value + food id when `foodFromTarget` is known.
///
/// Returns `(food_id, food_value)`. When target has food_value > 0, uses target;
/// otherwise falls back to the object itself (may be 0 â†’ skip later).
// Haxe: AiHelper.SearchBestFoodHelperNew foodObjData = foodFromTarget ?? objData
#[inline]
pub fn resolve_food_from_target(
    parent_id: i32,
    object_food_value: i32,
    food_from_target: Option<(i32, i32)>,
) -> (i32, i32) {
    match food_from_target {
        Some((tid, tfv)) if tfv > 0 => (tid, tfv),
        _ => (parent_id, object_food_value),
    }
}

/// Known pile parent for CountCloseObjects stock (transition table residual).
// Haxe: ObjectData.getPileObjId (hardcoded food piles used by processFood)
#[inline]
pub fn food_stock_pile_id(obj_id: i32) -> i32 {
    match obj_id {
        CARROT_ID => CARROT_PILE_ID,
        SHUCKED_CORN_ID => PILE_SHUCKED_CORN_ID,
        DRIED_CORN_ID => PILE_DRIED_CORN_ID,
        _ => -1,
    }
}

/// Haxe SearchBestFood / CountCloseObjects half-open square geometry.
// Haxe: for (ty in baseY-radius...baseY+radius)
#[inline]
pub fn in_search_food_square(base_x: i32, base_y: i32, tx: i32, ty: i32, radius: i32) -> bool {
    in_count_close_square(base_x, base_y, tx, ty, radius)
}

/// Haxe `blocksRemove` for known closed chests (content field residual).
// Haxe: ObjectData.blocksRemove + ServerSettings 987/988
#[inline]
pub fn container_blocks_remove(parent_id: i32) -> bool {
    BLOCKS_REMOVE_CONTAINER_IDS.contains(&parent_id)
}

// â”€â”€ Stock counts (pure) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Count tiles with `parent_id` in half-open square (simple +1 per tile).
// Haxe: AiHelper.CountCloseObjects (no piles)
pub fn count_parent_in_radius(
    cx: i32,
    cy: i32,
    parent_id: i32,
    radius: i32,
    tiles: &[StockTile],
) -> i32 {
    let mut n = 0;
    for &(tx, ty, pid, _uses) in tiles {
        if pid == parent_id && in_count_close_square(cx, cy, tx, ty, radius) {
            n += 1;
        }
    }
    n
}

/// CountCloseObjects with pile uses: matching `obj_id` â†’ +1; matching pile â†’ +uses.
// Haxe: AiHelper.CountCloseObjectsHelper countPiles
pub fn count_stock_with_piles(
    cx: i32,
    cy: i32,
    obj_id: i32,
    pile_id: i32,
    radius: i32,
    tiles: &[StockTile],
) -> i32 {
    let mut n = 0;
    for &(tx, ty, pid, uses) in tiles {
        if !in_count_close_square(cx, cy, tx, ty, radius) {
            continue;
        }
        if pid == obj_id {
            n += 1;
        } else if pile_id >= 0 && pid == pile_id {
            n += uses.max(1);
        }
    }
    n
}

// â”€â”€ Danger (pure) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `IsDangerousHelper` animal branch: deadly animal / hostile path near food.
///
/// Geometry matches Haxe `for (ty in baseY-radius...baseY+radius)` half-open square.
// Haxe: AiHelper.IsDangerousHelper
pub fn is_dangerous_near(
    food_tx: i32,
    food_ty: i32,
    radius: i32,
    deadly_animal_tiles: &[(i32, i32)],
    hostile_path_tiles: &[(i32, i32)],
) -> bool {
    for &(tx, ty) in deadly_animal_tiles {
        if tx >= food_tx - radius
            && tx < food_tx + radius
            && ty >= food_ty - radius
            && ty < food_ty + radius
        {
            return true;
        }
    }
    for &(tx, ty) in hostile_path_tiles {
        if tx >= food_tx - radius
            && tx < food_tx + radius
            && ty >= food_ty - radius
            && ty < food_ty + radius
        {
            return true;
        }
    }
    false
}

// â”€â”€ Distance â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Squared distance for scoring (Haxe `AiHelper.CalculateDistance` when wrap).
// Haxe: AiHelper.CalculateDistance / processFood quadDistance
fn score_quad_dist(opts: &ProcessFoodOpts, tx: i32, ty: i32) -> f32 {
    if opts.wrap && opts.map_w > 0 && opts.map_h > 0 {
        calculate_distance_sq(opts.base_x, opts.base_y, tx, ty, opts.map_w, opts.map_h, true)
            as f32
    } else {
        let dx = (tx - opts.base_x) as f32;
        let dy = (ty - opts.base_y) as f32;
        dx * dx + dy * dy
    }
}

fn feeder_quad_dist(opts: &ProcessFoodOpts, tx: i32, ty: i32) -> f32 {
    let (Some(fx), Some(fy)) = (opts.feeding_tx, opts.feeding_ty) else {
        return 0.0;
    };
    if opts.wrap && opts.map_w > 0 && opts.map_h > 0 {
        calculate_distance_sq(fx, fy, tx, ty, opts.map_w, opts.map_h, true) as f32
    } else {
        let fdx = (tx - fx) as f32;
        let fdy = (ty - fy) as f32;
        fdx * fdx + fdy * fdy
    }
}

// â”€â”€ processFood score â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Result of scoring one candidate (None = skip).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessFoodScore {
    pub food_value_scored: f32,
    pub quad_distance: f32,
    pub ratio: f32,
}

/// Haxe `processFood` pure â€” updates counters; returns score if candidate competes.
///
/// `stock_tiles` used when carrot/corn counts still need lazy CountCloseObjects.
// Haxe: AiHelper.processFood
pub fn process_food(
    cand: &SearchFoodCand,
    opts: &ProcessFoodOpts,
    counters: &mut SearchFoodCounters,
    stock_tiles: &[StockTile],
) -> Option<ProcessFoodScore> {
    if cand.not_reachable {
        return None;
    }
    if cand.food_value <= 0 {
        return None;
    }

    let parent_id = cand.parent_id;

    // Cooked Goose: skip first
    if parent_id == COOKED_GOOSE_ID {
        if !counters.goose_found {
            counters.goose_found = true;
            return None;
        }
    }

    // Carrot / pile: need â‰¥4 carrots nearby (pile uses count)
    if parent_id == CARROT_ID || parent_id == CARROT_PILE_ID {
        if counters.count_carrots < 0 {
            counters.count_carrots = count_stock_with_piles(
                opts.base_x,
                opts.base_y,
                CARROT_ID,
                food_stock_pile_id(CARROT_ID),
                FOOD_STOCK_COUNT_RADIUS,
                stock_tiles,
            );
        }
        if counters.count_carrots < 4 {
            return None;
        }
    }

    // Carrot Row: don't eat nearly empty (HelperNew: uses < 2)
    if parent_id == CARROT_ROW_ID && cand.number_of_uses < 2 {
        return None;
    }

    // Corn stock (shucked + dried, each with piles)
    if parent_id == SHUCKED_CORN_ID || parent_id == PILE_SHUCKED_CORN_ID {
        if counters.count_corn < 0 {
            counters.count_corn = count_stock_with_piles(
                opts.base_x,
                opts.base_y,
                SHUCKED_CORN_ID,
                food_stock_pile_id(SHUCKED_CORN_ID),
                FOOD_STOCK_COUNT_RADIUS,
                stock_tiles,
            ) + count_stock_with_piles(
                opts.base_x,
                opts.base_y,
                DRIED_CORN_ID,
                food_stock_pile_id(DRIED_CORN_ID),
                FOOD_STOCK_COUNT_RADIUS,
                stock_tiles,
            );
        }
        if counters.count_corn < 2 {
            return None;
        }
    }

    // AI seed protection
    if let Some(ai) = opts.ai {
        if parent_id == CARROT_ROW_ID && !ai.has_carrot_seeds {
            return None;
        }
        if parent_id == HOT_PEPPER_ID && !ai.has_pepper_seeds {
            return None;
        }
        if parent_id == FRUITING_PEPPER_ID && !ai.has_pepper_seeds && cand.number_of_uses < 2 {
            return None;
        }
        if (parent_id == WILD_ONION_ID || parent_id == ONION_ID) && !ai.has_onion_seeds {
            return None;
        }
        if parent_id == SHUCKED_CORN_ID && !ai.has_corn_seeds {
            return None;
        }
        if parent_id == RIPE_ONIONS_ID && cand.number_of_uses < 3 {
            return None;
        }
    }

    // foodId for craving / hasEaten (caller already keyed count_eaten by this)
    let food_id = if cand.food_id != 0 {
        cand.food_id
    } else {
        parent_id
    };

    // Feed-other + canEat (837 yellow-fever on feed)
    let yum_b = resolve_yum_bonus(opts.yum_bonus);
    if opts.feed_other {
        if !can_feed_to_me_obj_ex_yum(
            food_id,
            cand.food_value,
            cand.count_eaten,
            opts.food_store,
            opts.food_store_max,
            opts.has_yellow_fever,
            yum_b,
        ) {
            return None;
        }
    }
    if !can_eat_obj_ex(
        cand.food_value,
        cand.count_eaten,
        opts.food_store,
        opts.food_store_max,
        yum_b,
    ) {
        return None;
    }

    // Stomach room (Haxe also checks ceil(original/4) even after canEat)
    let original = cand.food_value as f32;
    let room = opts.food_store_max - opts.food_store;
    let need = (original / 4.0).ceil();
    if room < need {
        return None;
    }

    let mut quad = 16.0 + score_quad_dist(opts, cand.tx, cand.ty);
    if opts.feeding_tx.is_some() {
        quad += 1.0 + feeder_quad_dist(opts, cand.tx, cand.ty);
    }
    if quad < 1.0 {
        quad = 1.0;
    }

    let mut food_value = original - cand.count_eaten;
    // Haxe: isYum = countEaten < ServerSettings.YumBonus (YUM-LIVE-SETTINGS)
    let is_yum = cand.count_eaten < yum_b;
    let is_super_meh = food_value < original / 2.0;

    // Global food factor (per-cand Ã— opts fallback)
    food_value *= cand.food_factor * opts.food_factor;

    if is_yum {
        food_value *= opts.starving_factor;
    }
    if is_super_meh {
        food_value = original / opts.starving_factor;
    }
    if is_super_meh && opts.food_store > 3.0 {
        food_value = 0.0;
    }
    if opts.currently_craving != 0 && food_id == opts.currently_craving {
        food_value *= opts.starving_factor;
    }

    // AI danger: only when score-distance > 4
    if opts.ai.is_some() && quad > 4.0 && cand.is_dangerous {
        return None;
    }

    let ratio = food_value / quad;
    Some(ProcessFoodScore {
        food_value_scored: food_value,
        quad_distance: quad,
        ratio,
    })
}

/// Pick best among candidates; returns index into `cands`.
// Haxe: SearchBestFoodHelperNew loop + processFood best compare
pub fn pick_best_search_food(
    cands: &[SearchFoodCand],
    opts: &ProcessFoodOpts,
    stock_tiles: &[StockTile],
) -> Option<(usize, ProcessFoodScore)> {
    let mut counters = SearchFoodCounters::new();
    let mut best_i: Option<usize> = None;
    let mut best_ratio = f32::NEG_INFINITY;
    let mut best_score: Option<ProcessFoodScore> = None;

    for (i, c) in cands.iter().enumerate() {
        if let Some(s) = process_food(c, opts, &mut counters, stock_tiles) {
            if s.ratio > best_ratio {
                best_ratio = s.ratio;
                best_i = Some(i);
                best_score = Some(s);
            }
        }
    }
    Some((best_i?, best_score?))
}

/// Build [`BestFoodHit`] from cand + score.
pub fn to_best_hit(
    cand: &SearchFoodCand,
    score: &ProcessFoodScore,
    base_x: i32,
    base_y: i32,
) -> BestFoodHit {
    to_best_hit_ex(cand, score, base_x, base_y, 0, 0, false)
}

/// [`to_best_hit`] with optional torus wrap for raw_quad_dist.
pub fn to_best_hit_ex(
    cand: &SearchFoodCand,
    score: &ProcessFoodScore,
    base_x: i32,
    base_y: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> BestFoodHit {
    let raw = if wrap && map_w > 0 && map_h > 0 {
        calculate_distance_sq(base_x, base_y, cand.tx, cand.ty, map_w, map_h, true) as f32
    } else {
        let dx = (cand.tx - base_x) as f32;
        let dy = (cand.ty - base_y) as f32;
        dx * dx + dy * dy
    };
    let food_id = if cand.food_id != 0 {
        cand.food_id
    } else {
        cand.parent_id
    };
    BestFoodHit {
        food_id,
        food_value: cand.food_value,
        tx: cand.tx,
        ty: cand.ty,
        score_distance: score.quad_distance,
        scored_food_value: score.food_value_scored,
        raw_quad_dist: raw,
        index_in_container: cand.index_in_container,
    }
}

/// Super-meh refuse classification used in scoring (exposed for tests).
#[inline]
pub fn scoring_is_super_meh(food_value: i32, count_eaten: f32) -> bool {
    scoring_is_super_meh_ex(food_value, count_eaten, YUM_BONUS)
}

/// Super-meh refuse classification with live YumBonus.
// Haxe: ServerSettings.YumBonus via isObjSuperMeh (YUM-LIVE-SETTINGS)
#[inline]
pub fn scoring_is_super_meh_ex(food_value: i32, count_eaten: f32, yum_bonus: f32) -> bool {
    let original = food_value as f32;
    let adjusted = original - count_eaten;
    adjusted < original / 2.0 || is_obj_super_meh_ex(food_value, count_eaten, yum_bonus)
}

// â”€â”€ tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_opts(food: f32) -> ProcessFoodOpts {
        ProcessFoodOpts::human(10, 10, food, 20.0, 0)
    }

    fn cand(id: i32, fv: i32, tx: i32, ty: i32, count: f32, uses: i32) -> SearchFoodCand {
        SearchFoodCand::simple(id, fv, tx, ty, count, uses)
    }

    fn stock3(tiles: &[(i32, i32, i32)]) -> Vec<StockTile> {
        tiles.iter().map(|&(x, y, id)| (x, y, id, 1)).collect()
    }

    #[test]
    fn starving_factor_matches_haxe_cascade() {
        assert!((starving_factor(5.0) - 16.0).abs() < 1e-5);
        assert!((starving_factor(2.0) - 4.0).abs() < 1e-5);
        assert!((starving_factor(0.0) - 2.0).abs() < 1e-5);
        assert!((starving_factor(-1.2) - 1.5).abs() < 1e-5);
        assert!((starving_factor(-2.0) - 1.2).abs() < 1e-5);
    }

    /// YUM-LIVE-SETTINGS: processFood isYum band follows opts.yum_bonus.
    // Haxe: AiHelper.processFood countEaten < ServerSettings.YumBonus
    #[test]
    fn process_food_uses_live_yum_bonus_band() {
        let mut counters = SearchFoodCounters::new();
        // food_value 12, count 5 â†’ scored base 7 (not superMeh: 7 >= 6).
        // Under band 5: meh (count not < 5). Under band 7: yum â†’ Ã—starving_factor.
        let c = cand(33, 12, 12, 10, 5.0, 1);
        // food_store=2 â†’ starving_factor 4 when yum
        let mut opts = basic_opts(2.0);
        opts.yum_bonus = 5.0;
        let s5 = process_food(&c, &opts, &mut counters, &[]);
        assert!(s5.is_some());
        let mut counters2 = SearchFoodCounters::new();
        opts.yum_bonus = 7.0;
        let s7 = process_food(&c, &opts, &mut counters2, &[]);
        assert!(s7.is_some());
        let r5 = s5.unwrap().ratio;
        let r7 = s7.unwrap().ratio;
        assert!(
            r7 > r5,
            "yum band should score higher under live yum_bonus=7; r5={r5} r7={r7}"
        );
    }

    #[test]
    fn food_factor_bands() {
        assert!((food_factor_from_eaten_percentage(0.0) - 2.5).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(2.0) - 2.0).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(4.0) - 1.5).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(6.0) - 1.0).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(8.5) - 0.8).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(12.0) - 0.5).abs() < 1e-5);
    }

    /// C-SS-FULL-TABLE: live band table overrides defaults.
    #[test]
    fn food_factor_bands_live_ex() {
        let mut b = FoodFactorEatenBands::default();
        b.less_than_one_percent = 4.0;
        b.more_than_ten_percent = 0.25;
        assert!((food_factor_from_eaten_percentage_ex(0.0, &b) - 4.0).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage_ex(12.0, &b) - 0.25).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage_ex(6.0, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn food_factor_for_id_rollup_higher_quality() {
        // base 0% â†’ 2.5; with higher quality at 4% â†’ total 4% â†’ 1.5
        let eaten = [(99, 0.0_f32), (100, 4.0)];
        let hq = [(99, 100)];
        assert!((food_factor_for_id(99, &eaten, &hq) - 1.5).abs() < 1e-5);
        assert!((food_factor_for_id(100, &eaten, &[]) - 1.5).abs() < 1e-5);
    }

    #[test]
    fn food_factor_changes_ranking() {
        let opts = basic_opts(5.0);
        // Same distance: higher food_factor must win ranking.
        let mut common = cand(40, 5, 15, 10, 0.0, 1);
        common.food_factor = 0.5; // â‰¥10% eaten band
        let mut rare = cand(41, 5, 15, 10, 0.0, 1);
        rare.food_factor = 2.5; // <1% eaten band
        let (i, _) = pick_best_search_food(&[common, rare], &opts, &[]).unwrap();
        assert_eq!(i, 1, "higher food_factor should rank above equal-distance food");
        // Mild distance disadvantage still beaten by rare food factor.
        let mut near_common = cand(40, 5, 12, 10, 0.0, 1);
        near_common.food_factor = 0.5;
        let mut mid_rare = cand(41, 5, 16, 10, 0.0, 1);
        mid_rare.food_factor = 2.5;
        let (j, _) = pick_best_search_food(&[near_common, mid_rare], &opts, &[]).unwrap();
        assert_eq!(j, 1, "2.5Ã— factor should beat modest distance gap");
    }

    #[test]
    fn goose_skips_first_keeps_second() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        let stock = [];
        let g1 = cand(COOKED_GOOSE_ID, 8, 11, 10, 0.0, 1);
        assert!(process_food(&g1, &opts, &mut counters, &stock).is_none());
        assert!(counters.goose_found);
        let g2 = cand(COOKED_GOOSE_ID, 8, 12, 10, 0.0, 1);
        assert!(process_food(&g2, &opts, &mut counters, &stock).is_some());
    }

    #[test]
    fn carrot_needs_four_nearby() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        let stock = stock3(&[(10, 11, CARROT_ID), (10, 12, CARROT_ID)]);
        let c = cand(CARROT_ID, 5, 11, 10, 0.0, 1);
        assert!(process_food(&c, &opts, &mut counters, &stock).is_none());

        let mut counters2 = SearchFoodCounters::new();
        let stock4: Vec<StockTile> = (0..4).map(|i| (10 + i, 10, CARROT_ID, 1)).collect();
        assert!(process_food(&c, &opts, &mut counters2, &stock4).is_some());
    }

    #[test]
    fn carrot_pile_uses_count_toward_stock() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        // one single carrot + pile with 3 uses = 4 stock
        let stock: Vec<StockTile> = vec![
            (10, 11, CARROT_ID, 1),
            (10, 12, CARROT_PILE_ID, 3),
        ];
        let c = cand(CARROT_ID, 5, 11, 10, 0.0, 1);
        assert!(process_food(&c, &opts, &mut counters, &stock).is_some());
        assert_eq!(counters.count_carrots, 4);

        let mut counters2 = SearchFoodCounters::new();
        let stock_low: Vec<StockTile> = vec![(10, 12, CARROT_PILE_ID, 3)];
        assert!(process_food(&c, &opts, &mut counters2, &stock_low).is_none());
    }

    #[test]
    fn carrot_row_low_uses_skipped() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        let c = cand(CARROT_ROW_ID, 4, 11, 10, 0.0, 1);
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
        let c2 = cand(CARROT_ROW_ID, 4, 11, 10, 0.0, 2);
        // no AI â†’ seed gate off; uses ok
        assert!(process_food(&c2, &opts, &mut counters, &[]).is_some());
    }

    #[test]
    fn ai_seed_gates_carrot_row_and_onion() {
        let mut opts = basic_opts(5.0);
        opts.ai = Some(AiFoodSearchFlags::no_seeds());
        let mut counters = SearchFoodCounters::new();
        let row = cand(CARROT_ROW_ID, 4, 11, 10, 0.0, 4);
        assert!(process_food(&row, &opts, &mut counters, &[]).is_none());
        let onion = cand(WILD_ONION_ID, 3, 11, 10, 0.0, 1);
        assert!(process_food(&onion, &opts, &mut counters, &[]).is_none());
        opts.ai = Some(AiFoodSearchFlags::default());
        let mut counters2 = SearchFoodCounters::new();
        assert!(process_food(&onion, &opts, &mut counters2, &[]).is_some());
    }

    #[test]
    fn prefers_closer_yum_over_far() {
        let opts = basic_opts(5.0);
        let cands = [
            cand(33, 5, 50, 10, 0.0, 1),
            cand(40, 5, 12, 10, 0.0, 1),
        ];
        let (i, _) = pick_best_search_food(&cands, &opts, &[]).unwrap();
        assert_eq!(i, 1);
    }

    #[test]
    fn craving_boosts_score() {
        let mut opts = basic_opts(5.0);
        opts.currently_craving = 99;
        let cands = [
            cand(40, 5, 15, 10, 0.0, 1),
            cand(99, 5, 15, 10, 0.0, 1),
        ];
        let (i, _) = pick_best_search_food(&cands, &opts, &[]).unwrap();
        assert_eq!(i, 1);
    }

    #[test]
    fn super_meh_zero_when_food_store_above_three() {
        let opts = basic_opts(5.0); // food_store 5 > 3
        let mut counters = SearchFoodCounters::new();
        let c = cand(33, 5, 11, 10, 10.0, 1);
        // can_eat_obj refuses super meh when store > 4
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
    }

    #[test]
    fn feed_other_refuses_meh_when_not_starving() {
        let mut opts = basic_opts(10.0);
        opts.feed_other = true;
        let mut counters = SearchFoodCounters::new();
        let c = cand(33, 5, 11, 10, 5.0, 1); // meh
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
        opts.food_store = 1.0;
        opts.starving_factor = starving_factor(1.0);
        let mut counters2 = SearchFoodCounters::new();
        assert!(process_food(&c, &opts, &mut counters2, &[]).is_some());
    }

    #[test]
    fn feed_other_refuses_837_without_yellow_fever() {
        let mut opts = basic_opts(5.0);
        opts.feed_other = true;
        opts.has_yellow_fever = false;
        let mut counters = SearchFoodCounters::new();
        let c = cand(837, 3, 11, 10, 0.0, 1);
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
        opts.has_yellow_fever = true;
        let mut counters2 = SearchFoodCounters::new();
        assert!(process_food(&c, &opts, &mut counters2, &[]).is_some());
    }

    #[test]
    fn danger_skips_for_ai() {
        let mut opts = basic_opts(5.0);
        opts.ai = Some(AiFoodSearchFlags::default());
        let mut counters = SearchFoodCounters::new();
        let mut c = cand(33, 5, 30, 10, 0.0, 1);
        c.is_dangerous = true;
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
        c.is_dangerous = false;
        assert!(process_food(&c, &opts, &mut counters, &[]).is_some());
    }

    #[test]
    fn hostile_path_marks_dangerous() {
        assert!(is_dangerous_near(
            10,
            10,
            FOOD_DANGER_RADIUS,
            &[],
            &[(11, 10)]
        ));
        let mut opts = basic_opts(5.0);
        opts.ai = Some(AiFoodSearchFlags::default());
        let mut counters = SearchFoodCounters::new();
        let mut c = cand(33, 5, 20, 10, 0.0, 1);
        c.is_dangerous = is_dangerous_near(c.tx, c.ty, FOOD_DANGER_RADIUS, &[], &[(21, 10)]);
        assert!(c.is_dangerous);
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
    }

    #[test]
    fn feeder_distance_penalizes() {
        let mut opts = basic_opts(5.0);
        opts.feeding_tx = Some(0);
        opts.feeding_ty = Some(0);
        let near_eater_far_feeder = cand(40, 5, 20, 10, 0.0, 1);
        let near_both = cand(33, 5, 11, 10, 0.0, 1);
        let (i, _) =
            pick_best_search_food(&[near_eater_far_feeder, near_both], &opts, &[]).unwrap();
        assert_eq!(i, 1);
    }

    #[test]
    fn is_dangerous_near_half_open() {
        assert!(is_dangerous_near(10, 10, 4, &[(12, 10)], &[]));
        assert!(!is_dangerous_near(10, 10, 4, &[(14, 10)], &[])); // 10+4=14 excluded
        assert!(is_dangerous_near(10, 10, 4, &[], &[(11, 11)]));
    }

    #[test]
    fn search_radius_half_open_excludes_high_edge() {
        // Haxe: base+radius excluded
        assert!(in_search_food_square(10, 10, 10 + 39, 10, 40));
        assert!(!in_search_food_square(10, 10, 10 + 40, 10, 40));
        assert!(in_search_food_square(10, 10, 10 - 40, 10, 40));
    }

    #[test]
    fn container_blocks_remove_closed_chests() {
        assert!(container_blocks_remove(987));
        assert!(container_blocks_remove(988));
        assert!(!container_blocks_remove(33));
    }

    #[test]
    fn not_reachable_skipped() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        let mut c = cand(33, 5, 11, 10, 0.0, 1);
        c.not_reachable = true;
        assert!(process_food(&c, &opts, &mut counters, &[]).is_none());
    }

    #[test]
    fn corn_needs_two_stock() {
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        let stock = stock3(&[(10, 11, SHUCKED_CORN_ID)]);
        let c = cand(SHUCKED_CORN_ID, 4, 11, 10, 0.0, 1);
        assert!(process_food(&c, &opts, &mut counters, &stock).is_none());
        let stock2 = stock3(&[(10, 11, SHUCKED_CORN_ID), (10, 12, DRIED_CORN_ID)]);
        let mut counters2 = SearchFoodCounters::new();
        assert!(process_food(&c, &opts, &mut counters2, &stock2).is_some());
    }

    #[test]
    fn torus_wrap_prefers_across_map_edge() {
        let mut opts = basic_opts(5.0);
        opts.base_x = 0;
        opts.base_y = 0;
        opts.map_w = 100;
        opts.map_h = 100;
        opts.wrap = true;
        // Without wrap: (99,0) is far (99Â²); with wrap: dx=-1 â†’ dist 1
        let across = cand(40, 5, 99, 0, 0.0, 1);
        let far_plane = cand(41, 5, 30, 0, 0.0, 1);
        let (i, _) = pick_best_search_food(&[across, far_plane], &opts, &[]).unwrap();
        assert_eq!(i, 0, "torus wrap should make edge food closer");
    }

    #[test]
    fn food_from_target_only_candidate_scored() {
        // Bush parent 1001 has no food; berry target 1002 supplies value + food_id
        let (fid, fv) = resolve_food_from_target(1001, 0, Some((1002, 6)));
        assert_eq!((fid, fv), (1002, 6));
        let mut c = SearchFoodCand::simple(1001, fv, 11, 10, 0.0, 1);
        c.food_id = fid;
        let opts = basic_opts(5.0);
        let mut counters = SearchFoodCounters::new();
        assert!(process_food(&c, &opts, &mut counters, &[]).is_some());
        let hit = to_best_hit(
            &c,
            &process_food(&c, &opts, &mut SearchFoodCounters::new(), &[]).unwrap(),
            10,
            10,
        );
        assert_eq!(hit.food_id, 1002);
        assert_eq!(hit.food_value, 6);
    }

    #[test]
    fn get_food_id_prefers_food_from_target() {
        assert_eq!(get_food_id(1001, None), 1001);
        assert_eq!(get_food_id(1001, Some(1002)), 1002);
    }

    #[test]
    fn to_best_hit_raw_quad() {
        let c = cand(33, 5, 13, 10, 0.0, 1);
        let s = ProcessFoodScore {
            food_value_scored: 80.0,
            quad_distance: 16.0 + 9.0,
            ratio: 80.0 / 25.0,
        };
        let h = to_best_hit(&c, &s, 10, 10);
        assert_eq!(h.food_id, 33);
        assert!((h.raw_quad_dist - 9.0).abs() < 1e-5);
        assert_eq!(h.index_in_container, -1);
    }
}
