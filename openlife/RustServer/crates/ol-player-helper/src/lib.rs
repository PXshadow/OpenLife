//! **PlayerHelper** — pure logic shared by AI and ordinary player/sim code.
//!
//! Phase B of the AI split:
//! - No `SimState` / `World` mutation
//! - No HTTP / LLM
//! - Used by `ol-sim` (live adapters) and later by MainAI via [`ol_ai_api`] reads
//!
//! ## Modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`geom`] | CountClose square, torus distance |
//! | [`food_eat_gates`] | canEat / yum-meh / starving factor (pure) |
//! | [`food_search`] | SearchBestFood pure scoring (`process_food`, stock, danger) |

#![forbid(unsafe_code)]

pub mod food_eat_gates;
pub mod food_search;
pub mod geom;

// Convenient re-exports for sim / AI call sites
pub use food_eat_gates::{
    can_eat_obj, can_eat_obj_ex, can_feed_to_me_obj, can_feed_to_me_obj_ex,
    can_feed_to_me_obj_ex_yum, can_feed_to_me_obj_with_yum, is_obj_meh, is_obj_meh_ex,
    is_obj_super_meh, is_obj_super_meh_ex, is_obj_yum, is_obj_yum_ex, resolve_yum_bonus,
    starving_factor, MEH_FEED_REFUSE_FOOD_STORE, PSILOCYBE_MUSHROOM_ID,
    SUPER_MEH_REFUSE_FOOD_STORE, YUM_BONUS,
};
pub use food_search::{
    container_blocks_remove, count_parent_in_radius, count_stock_with_piles, food_factor_for_id,
    food_factor_for_id_ex, food_factor_from_eaten_percentage, food_factor_from_eaten_percentage_ex,
    food_stock_pile_id, get_food_id, in_search_food_square, is_dangerous_near,
    pick_best_search_food, process_food, resolve_food_from_target, scoring_is_super_meh,
    scoring_is_super_meh_ex, to_best_hit, to_best_hit_ex, AiFoodSearchFlags, BestFoodHit,
    FoodFactorEatenBands, ProcessFoodOpts, ProcessFoodScore, SearchFoodCand, SearchFoodCounters,
    StockTile, BLOCKS_REMOVE_CONTAINER_IDS, CARROT_ID, CARROT_PILE_ID, CARROT_ROW_ID,
    COOKED_GOOSE_ID, DRIED_CORN_ID, FOOD_DANGER_RADIUS, FOOD_FACTOR_EATEN_GE_10,
    FOOD_FACTOR_EATEN_GE_8, FOOD_FACTOR_EATEN_LT_1, FOOD_FACTOR_EATEN_LT_3, FOOD_FACTOR_EATEN_LT_5,
    FOOD_STOCK_COUNT_RADIUS, FRUITING_PEPPER_ID, HOT_PEPPER_ID, ONION_ID, PILE_DRIED_CORN_ID,
    PILE_SHUCKED_CORN_ID, RIPE_ONIONS_ID, SEARCH_BEST_FOOD_RADIUS, SHUCKED_CORN_ID, WILD_ONION_ID,
};
pub use geom::{calculate_distance_sq, chebyshev, in_count_close_square};

/// Product default for AI interface best-food radius (tiles).
/// Same as [`ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS`].
pub const DEFAULT_AI_FOOD_SEARCH_RADIUS: i32 = ol_ai_api::DEFAULT_FOOD_SEARCH_RADIUS;
