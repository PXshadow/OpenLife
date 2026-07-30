//! YUM / MEH / superMeh food history + display (Haxe GlobalPlayerInstance yum slice).
//!
//! Haxe anchors:
//! - `GlobalPlayerInstance.isObjYum` / `isObjMeh` / `isObjSuperMeh`
//! - `isHoldingYum` / `isHoldingMeh`
//! - `displayFood` / `DisplayBestFood`
//! - `canEatObj` / eat path (`hasEatenMap` + `ServerSettings.YumBonus`)
//! - `doIncreaseFoodValue` / `restoreFoodCount` / CRAVING wire (CRAVING-WIRE)
//! - `PlayerAccount.displayYum`
//!
//! Pure classifiers and fill math live here; world scan for `SearchBestFood`
//! is optional input via candidate lists.


// Phase B: pure free functions live in ol-player-helper (shared with AI).
pub use ol_player_helper::{
    can_eat_obj, can_eat_obj_ex, can_feed_to_me_obj, can_feed_to_me_obj_ex,
    can_feed_to_me_obj_ex_yum, can_feed_to_me_obj_with_yum, is_obj_meh, is_obj_meh_ex,
    is_obj_super_meh, is_obj_super_meh_ex, is_obj_yum, is_obj_yum_ex, resolve_yum_bonus,
    starving_factor, MEH_FEED_REFUSE_FOOD_STORE, PSILOCYBE_MUSHROOM_ID,
    SUPER_MEH_REFUSE_FOOD_STORE, YUM_BONUS,
};

use std::collections::{HashMap, VecDeque};

// ---------------------------------------------------------------------------
// ServerSettings constants (Haxe openlife.settings.ServerSettings)
/// Haxe `ServerSettings.FoodReductionPerEating` — hasEatenMap increment per eat.
pub const FOOD_REDUCTION_PER_EATING: f32 = 1.0;

/// Haxe `ServerSettings.FoodReductionFaktorForEatingMeh`.
pub const FOOD_REDUCTION_FAKTOR_MEH: f32 = 0.2;

/// Haxe `ServerSettings.FoodReductionFaktorForEatingHighQuailitFood`.
pub const FOOD_REDUCTION_FAKTOR_HIGH_QUALITY: f32 = 0.8;

/// Haxe `ServerSettings.HealthLostWhenEatingMeh`.
pub const HEALTH_LOST_MEH: f32 = 0.5;

/// Haxe `ServerSettings.HealthLostWhenEatingSuperMeh`.
pub const HEALTH_LOST_SUPER_MEH: f32 = 2.0;

/// Haxe `ServerSettings.FoodFactor` (global fill scale; live via GameplayKnobs).
// C-SS-FULL-TABLE: default constant; live path uses `state.gameplay.food_factor`
pub const FOOD_FACTOR: f32 = 1.0;
/// Sanitize global FoodFactor for fill scale.
// Haxe: ServerSettings.FoodFactor
#[inline]
pub fn resolve_food_factor(food_factor: f32) -> f32 {
    if food_factor.is_finite() && food_factor >= 0.0 {
        food_factor
    } else {
        FOOD_FACTOR
    }
}

/// Sanitize non-negative live eat/restore knobs (NaN/neg → default).
#[inline]
pub fn resolve_nonneg_knob(v: f32, default: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else {
        default
    }
}

/// Live eat-path knobs (YumBonus + FoodFactor + hasEaten reduction + meh health).
// Haxe: ServerSettings.YumBonus / FoodFactor / FoodReduction* / HealthLostWhenEating*
// C-SS-FULL-TABLE
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EatLiveKnobs {
    pub yum_bonus: f32,
    pub food_factor: f32,
    pub food_reduction_per_eating: f32,
    pub food_reduction_faktor_meh: f32,
    pub health_lost_meh: f32,
    pub health_lost_super_meh: f32,
}

impl Default for EatLiveKnobs {
    fn default() -> Self {
        Self {
            yum_bonus: YUM_BONUS,
            food_factor: FOOD_FACTOR,
            food_reduction_per_eating: FOOD_REDUCTION_PER_EATING,
            food_reduction_faktor_meh: FOOD_REDUCTION_FAKTOR_MEH,
            health_lost_meh: HEALTH_LOST_MEH,
            health_lost_super_meh: HEALTH_LOST_SUPER_MEH,
        }
    }
}

impl EatLiveKnobs {
    /// Sanitize each field (NaN/neg → Haxe default).
    pub fn resolved(self) -> Self {
        Self {
            yum_bonus: resolve_yum_bonus(self.yum_bonus),
            food_factor: resolve_food_factor(self.food_factor),
            food_reduction_per_eating: resolve_nonneg_knob(
                self.food_reduction_per_eating,
                FOOD_REDUCTION_PER_EATING,
            ),
            food_reduction_faktor_meh: resolve_nonneg_knob(
                self.food_reduction_faktor_meh,
                FOOD_REDUCTION_FAKTOR_MEH,
            ),
            health_lost_meh: resolve_nonneg_knob(self.health_lost_meh, HEALTH_LOST_MEH),
            health_lost_super_meh: resolve_nonneg_knob(
                self.health_lost_super_meh,
                HEALTH_LOST_SUPER_MEH,
            ),
        }
    }
}

/// Live craving-restore knobs for [`YumState::do_increase_food_value_ex`].
// Haxe: ServerSettings.YumFoodRestore / LovedFoodRestore / YumNewCravingChance
// C-SS-FULL-TABLE
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YumRestoreKnobs {
    pub yum_food_restore: f32,
    pub loved_food_restore: f32,
    pub yum_new_craving_chance: f32,
}

impl Default for YumRestoreKnobs {
    fn default() -> Self {
        Self {
            yum_food_restore: YUM_FOOD_RESTORE,
            loved_food_restore: LOVED_FOOD_RESTORE,
            yum_new_craving_chance: YUM_NEW_CRAVING_CHANCE,
        }
    }
}

impl YumRestoreKnobs {
    pub fn resolved(self) -> Self {
        Self {
            yum_food_restore: resolve_yum_food_restore(self.yum_food_restore),
            loved_food_restore: resolve_nonneg_knob(
                self.loved_food_restore,
                LOVED_FOOD_RESTORE,
            ),
            yum_new_craving_chance: resolve_nonneg_knob(
                self.yum_new_craving_chance,
                YUM_NEW_CRAVING_CHANCE,
            ),
        }
    }
}
/// Super-meh self-eat refuse in full eat path when `food_store > 5`.
pub const SUPER_MEH_EAT_REFUSE_FOOD_STORE: f32 = 5.0;

/// Haxe `ServerSettings.YumFoodRestore` — random other-food hasEaten restore per eat.
/// Live path uses `state.gameplay.yum_food_restore` via [`YumState::do_increase_food_value_ex`].
// C-SS-FULL-TABLE
pub const YUM_FOOD_RESTORE: f32 = 0.8;

/// Sanitize live YumFoodRestore (Haxe static ≥ 0; NaN/neg → default).
// Haxe: ServerSettings.YumFoodRestore
#[inline]
pub fn resolve_yum_food_restore(yum_food_restore: f32) -> f32 {
    if yum_food_restore.is_finite() && yum_food_restore >= 0.0 {
        yum_food_restore
    } else {
        YUM_FOOD_RESTORE
    }
}

/// Haxe `ServerSettings.LovedFoodRestore` — loved-biome food restore per eat.
/// Live path uses `state.gameplay.loved_food_restore` via [`YumRestoreKnobs`].
// C-SS-FULL-TABLE
pub const LOVED_FOOD_RESTORE: f32 = 0.1;

/// Haxe `ServerSettings.YumNewCravingChance` — chance to pick random foodObjects craving.
/// Live path uses `state.gameplay.yum_new_craving_chance` via [`YumRestoreKnobs`].
// C-SS-FULL-TABLE
pub const YUM_NEW_CRAVING_CHANCE: f32 = 0.2;

/// Haxe `dontChangeCraving = playerFrom != playerTo || isFoodYum == false`.
///
/// - Self-eat yum → may pick a new craving (`false`)
/// - Self-eat meh → keep current craving (`true`)
/// - Feed-other (any yum/meh) → always keep eater's craving (`true`)
// Haxe: GlobalPlayerInstance eat path L3135
#[inline]
pub fn dont_change_craving(is_self_eat: bool, is_yum: bool) -> bool {
    !is_self_eat || !is_yum
}

// ---------------------------------------------------------------------------
// Loved food ids (Haxe Biome.getLovedFoodIds) — CRAVING-WIRE
// ---------------------------------------------------------------------------

/// Haxe biome ids (BiomeTag).
pub const BIOME_GREY: i32 = 3;
pub const BIOME_SNOW: i32 = 4;
pub const BIOME_DESERT: i32 = 5;
pub const BIOME_JUNGLE: i32 = 6;

/// Haxe `PersonColor` race ids.
pub const PERSON_BLACK: i32 = 1;
pub const PERSON_BROWN: i32 = 3;
pub const PERSON_WHITE: i32 = 4;
pub const PERSON_GINGER: i32 = 6;

/// Haxe `Biome.getLovedFoodIds(biomeTag)`.
#[inline]
pub fn loved_food_ids_for_biome(biome_tag: i32) -> &'static [i32] {
    match biome_tag {
        BIOME_DESERT => &[768, 197],   // Cactus Fruit, Cooked Rabbit
        BIOME_JUNGLE => &[2143, 1880], // banana, Mango Slices
        BIOME_GREY => &[4252, 1242],   // Wild Garlic, Bowl of Sauerkraut
        BIOME_SNOW => &[40, 2106],     // Wild Carrot, Cooked Fish
        _ => &[],
    }
}

/// Haxe `Biome.GetLovedBiomeByPlayer` via person race color.
#[inline]
pub fn loved_biome_for_person_color(person_color: i32) -> Option<i32> {
    match person_color {
        PERSON_GINGER => Some(BIOME_SNOW),
        PERSON_WHITE => Some(BIOME_GREY),
        PERSON_BROWN => Some(BIOME_JUNGLE),
        PERSON_BLACK => Some(BIOME_DESERT),
        _ => None,
    }
}

/// Haxe `player.getLovedFoodIds()` from person race color.
#[inline]
pub fn loved_food_ids_for_person_color(person_color: i32) -> &'static [i32] {
    match loved_biome_for_person_color(person_color) {
        Some(b) => loved_food_ids_for_biome(b),
        None => &[],
    }
}

// ---------------------------------------------------------------------------
// Food classification (pure)
// ---------------------------------------------------------------------------

/// Resolve content food id (Haxe `ObjectData.getFoodId` simplified).
///
/// When `dummy_parent` / `food_from_target` are known, pass the resolved base id
/// as `food_id`. Without content tables, callers pass the held/object id.
#[inline]
#[allow(dead_code)] // public helper for AI / content callers
pub fn resolve_food_id(object_id: i32, dummy_parent: Option<i32>) -> i32 {
    dummy_parent.unwrap_or(object_id)
}
/// Haxe `isHoldingYum` with live YumBonus.
// Haxe: GlobalPlayerInstance.isHoldingYum
pub fn is_holding_yum_ex(
    held_id: i32,
    food_value: i32,
    count_eaten: f32,
    yum_bonus: f32,
) -> bool {
    if held_id <= 0 {
        return false;
    }
    is_obj_yum_ex(food_value, count_eaten, yum_bonus)
}

/// Haxe `isHoldingYum` without player-hold check: food + yum.
pub fn is_holding_yum(held_id: i32, food_value: i32, count_eaten: f32) -> bool {
    is_holding_yum_ex(held_id, food_value, count_eaten, YUM_BONUS)
}

/// Haxe `isHoldingMeh` with live YumBonus: foodValue ≥ 1 and `countEaten > YumBonus`.
// Haxe: GlobalPlayerInstance.isHoldingMeh
pub fn is_holding_meh_ex(
    held_id: i32,
    food_value: i32,
    count_eaten: f32,
    yum_bonus: f32,
) -> bool {
    if held_id <= 0 {
        return false;
    }
    if food_value < 1 {
        return false;
    }
    count_eaten > resolve_yum_bonus(yum_bonus)
}

/// Haxe `isHoldingMeh` at default [`YUM_BONUS`].
pub fn is_holding_meh(held_id: i32, food_value: i32, count_eaten: f32) -> bool {
    is_holding_meh_ex(held_id, food_value, count_eaten, YUM_BONUS)
}

/// Emote name when picking up food with live YumBonus.
// Haxe: GlobalPlayerInstance.setHeldObject food branch
pub fn hold_food_emote_ex(
    held_id: i32,
    food_value: i32,
    count_eaten: f32,
    yum_bonus: f32,
) -> Option<&'static str> {
    if held_id <= 0 || food_value < 1 {
        return None;
    }
    if is_holding_yum_ex(held_id, food_value, count_eaten, yum_bonus) {
        Some("JOY")
    } else if is_obj_super_meh_ex(food_value, count_eaten, yum_bonus) {
        Some("SAD")
    } else {
        Some("HMPH")
    }
}

/// Emote name when picking up food (Haxe `setHeldObject` food branch).
///
/// Returns `"JOY"` / `"SAD"` / `"HMPH"` for food; `None` for non-food.
pub fn hold_food_emote(
    held_id: i32,
    food_value: i32,
    count_eaten: f32,
) -> Option<&'static str> {
    hold_food_emote_ex(held_id, food_value, count_eaten, YUM_BONUS)
}

// ---------------------------------------------------------------------------
// canEatObj gates (pure)
// ---------------------------------------------------------------------------
// Eat fill formula (pure)
// ---------------------------------------------------------------------------

/// Result of computing one eat (Haxe self/feed eat path fill + classification).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EatCompute {
    /// Fill units before world food factors (Haxe foodValue after YumBonus adjust).
    pub fill: f32,
    pub is_yum: bool,
    pub is_super_meh: bool,
    pub is_craving: bool,
    /// Amount to add to hasEatenMap (0 when superMeh — Haxe skips reduce).
    pub has_eaten_delta: f32,
    /// Prestige / health delta sign magnitude (positive yum, negative meh).
    pub health_delta: f32,
}

/// Haxe eat-path food value + hasEatenMap delta with live YumBonus / FoodFactor.
///
/// `count_eaten` is current map value (negative = craving longing).
/// Does **not** apply WorldMap FoodFactor / starving (see [`crate::apply_world_food_factors`]).
// Haxe: GlobalPlayerInstance doEating YumBonus / FoodFactor (~L3087–3186)
/// Full eat math with live reduction / health knobs.
// Haxe: GlobalPlayerInstance doEating + ServerSettings.FoodReduction* / HealthLost*
// C-SS-FULL-TABLE
pub fn compute_eat_full(
    base_food_value: i32,
    count_eaten: f32,
    knobs: EatLiveKnobs,
) -> EatCompute {
    let k = knobs.resolved();
    let yb = k.yum_bonus;
    let ff = k.food_factor;
    let original = base_food_value as f32;
    if original < 1.0 {
        return EatCompute {
            fill: 0.0,
            is_yum: false,
            is_super_meh: false,
            is_craving: false,
            has_eaten_delta: 0.0,
            health_delta: 0.0,
        };
    }

    let is_craving = count_eaten < 0.0;
    let food_boni = if is_craving {
        ((-count_eaten) / 2.0).min(20.0)
    } else {
        0.0
    };

    let count = if count_eaten < 0.0 { 0.0 } else { count_eaten };
    let mut food_value = original;
    if is_craving {
        food_value += 1.0 + food_boni;
    }
    food_value += yb;
    food_value -= count;

    let is_yum = count_eaten < yb;
    let is_super_meh = is_obj_super_meh_ex(base_food_value, count_eaten, yb);
    if is_super_meh {
        food_value = original / 2.0;
    }

    // hasEatenMap update (skipped for superMeh in Haxe)
    let has_eaten_delta = if is_super_meh {
        0.0
    } else if is_craving {
        let mut fe = ((-count_eaten) / 2.0).ceil().max(1.0);
        if fe > 10.0 {
            fe = 10.0;
        }
        fe
    } else {
        let mut fe = k.food_reduction_per_eating;
        if !is_yum {
            fe *= k.food_reduction_faktor_meh;
        }
        fe
    };

    let health_delta = if is_yum {
        // Haxe: addHealthAndPrestige(foodEaten) — use delta amount
        has_eaten_delta.max(k.food_reduction_per_eating)
    } else if is_super_meh {
        -k.health_lost_super_meh
    } else {
        -k.health_lost_meh
    };

    EatCompute {
        fill: (food_value * ff).max(0.0),
        is_yum,
        is_super_meh,
        is_craving,
        has_eaten_delta,
        health_delta,
    }
}

/// Haxe eat-path food value + hasEatenMap delta with live YumBonus / FoodFactor.
///
/// Reduction / health use default ModuleConst values; prefer [`compute_eat_full`]
/// when live FoodReduction* / HealthLost* knobs matter.
pub fn compute_eat_ex(
    base_food_value: i32,
    count_eaten: f32,
    yum_bonus: f32,
    food_factor: f32,
) -> EatCompute {
    compute_eat_full(
        base_food_value,
        count_eaten,
        EatLiveKnobs {
            yum_bonus,
            food_factor,
            ..EatLiveKnobs::default()
        },
    )
}

/// Haxe eat-path food value + hasEatenMap delta at default YumBonus / FoodFactor.
///
/// `count_eaten` is current map value (negative = craving longing).
pub fn compute_eat(base_food_value: i32, count_eaten: f32) -> EatCompute {
    compute_eat_ex(base_food_value, count_eaten, YUM_BONUS, FOOD_FACTOR)
}

/// Whether self-eat should refuse (Haxe superMeh + food_store > 5).
pub fn refuse_self_eat_super_meh(is_super_meh: bool, food_store: f32) -> bool {
    is_super_meh && food_store > SUPER_MEH_EAT_REFUSE_FOOD_STORE
}

/// Haxe feed-other feeder yum prestige share (`gainedPrestige * 0.2`).
// Haxe: GlobalPlayerInstance.doEating L3151–3152
// FEED-OTHER-YUM / feed_full_eat
pub const FEED_OTHER_FEEDER_PRESTIGE_SHARE: f32 = 0.2;

/// Prestige delta for feeder when feed-other is yum (0 if meh/superMeh or non-yum).
///
/// Haxe: `playerFrom.addHealthAndPrestige(gainedPrestige * 0.2)` only on yum path.
// Haxe: GlobalPlayerInstance.doEating L3151–3152
// FEED-OTHER-YUM
pub fn feed_other_feeder_prestige_delta(eater_health_delta: f32, is_yum: bool) -> f32 {
    if is_yum && eater_health_delta.is_finite() && eater_health_delta > 0.0 {
        eater_health_delta * FEED_OTHER_FEEDER_PRESTIGE_SHARE
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// displayFood text (pure)
// ---------------------------------------------------------------------------

/// Haxe `displayFood` LS label text with live YumBonus (without distance suffix).
///
/// Yum: `Y` or `YY` (craving) + up to 5 `U` + `M!`  
/// Meh: `M` + `E`×count + `H!`
// Haxe: GlobalPlayerInstance.displayFood
pub fn format_display_food_text_ex(
    food_id: i32,
    food_value: i32,
    count_eaten: f32,
    currently_craving: i32,
    yum_bonus: f32,
) -> String {
    let yb = resolve_yum_bonus(yum_bonus);
    let is_yum = count_eaten < yb;
    if is_yum {
        let mut text = if food_id == currently_craving && currently_craving != 0 {
            String::from("YY")
        } else {
            String::from("Y")
        };
        let mut count = (yb - count_eaten).ceil() as i32;
        count = count.clamp(0, 5);
        for _ in 0..count {
            text.push('U');
        }
        text.push_str("M!");
        text
    } else {
        let mut text = String::from("M");
        let denom = (food_value as f32 / 4.0).max(1.0);
        let count = (1.0 + (count_eaten - yb) / denom).floor() as i32;
        let count = count.max(1);
        for _ in 0..count {
            text.push('E');
        }
        text.push_str("H!");
        text
    }
}

/// Haxe `displayFood` LS label text at default [`YUM_BONUS`].
pub fn format_display_food_text(
    food_id: i32,
    food_value: i32,
    count_eaten: f32,
    currently_craving: i32,
) -> String {
    format_display_food_text_ex(
        food_id,
        food_value,
        count_eaten,
        currently_craving,
        YUM_BONUS,
    )
}

/// Append `_NM` distance suffix when dist > 9 (Haxe displayFood).
pub fn append_distance_suffix(text: &str, dist: i32) -> String {
    if dist > 9 {
        format!("{text}_{dist}M")
    } else {
        text.to_string()
    }
}

/// Full displayFood label with live YumBonus including optional distance.
// Haxe: GlobalPlayerInstance.displayFood
pub fn format_display_food_label_ex(
    food_id: i32,
    food_value: i32,
    count_eaten: f32,
    currently_craving: i32,
    dist: i32,
    yum_bonus: f32,
) -> String {
    let base = format_display_food_text_ex(
        food_id,
        food_value,
        count_eaten,
        currently_craving,
        yum_bonus,
    );
    append_distance_suffix(&base, dist)
}

/// Full displayFood label including optional distance.
pub fn format_display_food_label(
    food_id: i32,
    food_value: i32,
    count_eaten: f32,
    currently_craving: i32,
    dist: i32,
) -> String {
    format_display_food_label_ex(
        food_id,
        food_value,
        count_eaten,
        currently_craving,
        dist,
        YUM_BONUS,
    )
}

// ---------------------------------------------------------------------------
// DisplayBestFood gates (pure)
// ---------------------------------------------------------------------------

/// Haxe `DisplayBestFood` display decision (without running SearchBestFood).
///
/// Shows when:
/// - best food exists
/// - not holding yum OR best matches craving
/// - `food_store < food_store_max * food_store_factor`
/// - quad distance to best > 10
pub fn should_display_best_food(
    has_best: bool,
    holding_yum: bool,
    best_food_id: i32,
    currently_craving: i32,
    food_store: f32,
    food_store_max: f32,
    food_store_factor: f32,
    best_quad_dist: f32,
) -> bool {
    if !has_best {
        return false;
    }
    let holding_ok = !holding_yum || (best_food_id == currently_craving && currently_craving != 0);
    if !holding_ok {
        return false;
    }
    if food_store >= food_store_max * food_store_factor {
        return false;
    }
    best_quad_dist > 10.0
}

// ---------------------------------------------------------------------------
// SearchBestFood scoring (pure candidate pick)
// ---------------------------------------------------------------------------

/// One food candidate for AI / display search.
#[derive(Debug, Clone, Copy)]
pub struct FoodCandidate {
    pub food_id: i32,
    pub food_value: i32,
    pub tx: i32,
    pub ty: i32,
    /// hasEatenMap count for this food id.
    pub count_eaten: f32,
}

/// Score one food for SearchBestFood-style pick with live YumBonus (higher is better).
///
/// Simplified from Haxe `processFood` without goose/carrot special cases.
// Haxe: AiHelper.processFood isYum = countEaten < ServerSettings.YumBonus
pub fn score_food_candidate_ex(
    c: &FoodCandidate,
    base_x: i32,
    base_y: i32,
    food_store: f32,
    food_store_max: f32,
    currently_craving: i32,
    starving_factor: f32,
    yum_bonus: f32,
) -> Option<f32> {
    if c.food_value <= 0 {
        return None;
    }
    if !can_eat_obj_ex(
        c.food_value,
        c.count_eaten,
        food_store,
        food_store_max,
        yum_bonus,
    ) {
        return None;
    }
    let dx = (c.tx - base_x) as f32;
    let dy = (c.ty - base_y) as f32;
    let mut quad = 16.0 + dx * dx + dy * dy;
    if quad < 1.0 {
        quad = 1.0;
    }

    let yb = resolve_yum_bonus(yum_bonus);
    let original = c.food_value as f32;
    let mut food_value = original - c.count_eaten;
    let is_yum = c.count_eaten < yb;
    let is_super_meh = food_value < original / 2.0;
    if is_yum {
        food_value *= starving_factor;
    }
    if is_super_meh {
        food_value = original / starving_factor;
    }
    if is_super_meh && food_store > 3.0 {
        food_value = 0.0;
    }
    if currently_craving != 0 && c.food_id == currently_craving {
        food_value *= starving_factor;
    }
    Some(food_value / quad)
}

/// Score one food at default [`YUM_BONUS`].
pub fn score_food_candidate(
    c: &FoodCandidate,
    base_x: i32,
    base_y: i32,
    food_store: f32,
    food_store_max: f32,
    currently_craving: i32,
    starving_factor: f32,
) -> Option<f32> {
    score_food_candidate_ex(
        c,
        base_x,
        base_y,
        food_store,
        food_store_max,
        currently_craving,
        starving_factor,
        YUM_BONUS,
    )
}

/// Pick best candidate by score with live YumBonus; returns index into `cands` or None.
pub fn pick_best_food_ex(
    cands: &[FoodCandidate],
    base_x: i32,
    base_y: i32,
    food_store: f32,
    food_store_max: f32,
    currently_craving: i32,
    yum_bonus: f32,
) -> Option<usize> {
    // Full Haxe cascade via starving_factor (includes < -1 / < -1.5)
    let starving = starving_factor(food_store);
    let mut best_i: Option<usize> = None;
    let mut best_score = f32::NEG_INFINITY;
    for (i, c) in cands.iter().enumerate() {
        if let Some(s) = score_food_candidate_ex(
            c,
            base_x,
            base_y,
            food_store,
            food_store_max,
            currently_craving,
            starving,
            yum_bonus,
        ) {
            if s > best_score {
                best_score = s;
                best_i = Some(i);
            }
        }
    }
    best_i
}

/// Pick best candidate by score; returns index into `cands` or None.
pub fn pick_best_food(
    cands: &[FoodCandidate],
    base_x: i32,
    base_y: i32,
    food_store: f32,
    food_store_max: f32,
    currently_craving: i32,
) -> Option<usize> {
    pick_best_food_ex(
        cands,
        base_x,
        base_y,
        food_store,
        food_store_max,
        currently_craving,
        YUM_BONUS,
    )
}
// ---------------------------------------------------------------------------
// YumState — live player yum book
// ---------------------------------------------------------------------------

/// CRAVING wire payload (Haxe `ClientTag.CRAVING` / CR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CravingWire {
    pub food_id: i32,
    /// Haxe `${- count}` longing amount after display adjust (positive).
    pub bonus: i32,
}

/// Nearby best food for craving pick + optional `displayFood` LS.
///
/// `(food_id, count_eaten, world_tx, world_ty)` from SearchBestFood lite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearbyBestFood {
    pub food_id: i32,
    pub count_eaten: f32,
    pub tx: i32,
    pub ty: i32,
}

/// Pending Haxe `displayFood(bestfood)` after new-craving nearby branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingDisplayFood {
    pub food_id: i32,
    pub tx: i32,
    pub ty: i32,
}

// ---------------------------------------------------------------------------
// InheritEatenFoodCounts (C-SS-MORE-KNOBS / birth hasEaten inheritance)
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.MaxHasEatenForNextGeneration` default.
// Haxe: ServerSettings.MaxHasEatenForNextGeneration = 4
pub const MAX_HAS_EATEN_FOR_NEXT_GENERATION: f32 = 4.0;

/// Haxe `ServerSettings.HasEatenReductionForNextGeneration` default.
// Haxe: ServerSettings.HasEatenReductionForNextGeneration = 1
pub const HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION: f32 = 1.0;

/// One hasEatenMap entry for the next generation.
///
/// When `count > 0`: subtract `reduction` and floor at 0.
/// Then clamp to `max_for_next` (positives only clamp; negatives preserved).
// Haxe: GlobalPlayerInstance.InheritEatenFoodCounts L1355-1362
#[inline]
pub fn inherit_eaten_food_count(count: f32, reduction: f32, max_for_next: f32) -> f32 {
    let mut food_count = if count.is_finite() { count } else { 0.0 };
    let red = if reduction.is_finite() && reduction >= 0.0 {
        reduction
    } else {
        HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION
    };
    let max_v = if max_for_next.is_finite() && max_for_next >= 0.0 {
        max_for_next
    } else {
        MAX_HAS_EATEN_FOR_NEXT_GENERATION
    };
    // Haxe: if (foodCount > 0) { foodCount -= reduction; if < 0 → 0 }
    if food_count > 0.0 {
        food_count -= red;
        if food_count < 0.0 {
            food_count = 0.0;
        }
    }
    // Haxe: if (foodCount > MaxHasEaten) foodCount = MaxHasEaten
    if food_count > max_v {
        food_count = max_v;
    }
    food_count
}

/// Copy + reduce + clamp an entire hasEatenMap for a newborn.
// Haxe: GlobalPlayerInstance.InheritEatenFoodCounts L1355-1362
pub fn inherit_eaten_food_counts(
    source: &HashMap<i32, f32>,
    reduction: f32,
    max_for_next: f32,
) -> HashMap<i32, f32> {
    source
        .iter()
        .map(|(&food_id, &count)| {
            (
                food_id,
                inherit_eaten_food_count(count, reduction, max_for_next),
            )
        })
        .collect()
}

/// Resolve which ancestor's hasEatenMap to copy.
///
/// Haxe: girls inherit from father line; boys from mother line.
/// Prefer grandparent when present (`mother.mother` / `father.father`).
// Haxe: GlobalPlayerInstance.InheritEatenFoodCounts L1350-1353
#[inline]
pub fn inherit_eaten_source_parent_id(
    child_is_female: bool,
    mother_id: i32,
    mother_mother_id: Option<i32>,
    father_id: Option<i32>,
    father_father_id: Option<i32>,
) -> i32 {
    // Haxe: if child.father != null && child.isFemale() → father.father ?? father
    if child_is_female {
        if let Some(fid) = father_id {
            return father_father_id.unwrap_or(fid);
        }
    }
    // Default: mother.mother ?? mother (also used when female has no father)
    mother_mother_id.unwrap_or(mother_id)
}

/// Per-player yum / meh state (Haxe `hasEatenMap` + wire just_ate fields).
#[derive(Debug, Clone)]
pub struct YumState {
    /// Haxe `hasEatenMap` — food id → times-eaten (negative = craving longing).
    pub has_eaten: HashMap<i32, f32>,
    /// Recent eat order (query / legacy variety display).
    pub history: VecDeque<i32>,
    pub capacity: usize,
    /// Wire / FX `yum_bonus` (stored bonus food or remaining yum signal).
    pub yum_bonus: f32,
    /// Last food object id eaten (`last_ate_id` on FX / PU).
    pub just_ate_id: i32,
    /// Haxe `just_ate` flag: 1 while the eat PU is in flight, then cleared.
    pub just_ate: bool,
    /// Food store (ceil) before the last eat — FX `last_ate_fill_max`.
    pub last_ate_fill_max: i32,
    /// Haxe `currentlyCraving` object id (0 = none).
    pub currently_craving: i32,
    /// Haxe `cravings` — food ids with hasEaten ≤ 0 (known craving pool).
    pub cravings: Vec<i32>,
    /// Haxe `lastCravingIndex` into foodObjects when picking random craving.
    pub last_craving_index: i32,
    /// Pending CRAVING wire after last `do_increase_food_value` (taken by net).
    pub pending_craving_wire: Option<CravingWire>,
    /// When set, caller should `displayFood` this nearby best food (Haxe branch).
    pub pending_display_food: Option<PendingDisplayFood>,
}

impl Default for YumState {
    fn default() -> Self {
        Self {
            has_eaten: HashMap::new(),
            history: VecDeque::with_capacity(8),
            capacity: 8,
            yum_bonus: 0.0,
            just_ate_id: 0,
            just_ate: false,
            last_ate_fill_max: 0,
            currently_craving: 0,
            cravings: Vec::new(),
            last_craving_index: 0,
            pending_craving_wire: None,
            pending_display_food: None,
        }
    }
}

impl YumState {
    /// Haxe `getCountEaten`.
    pub fn get_count_eaten(&self, food_id: i32) -> f32 {
        self.has_eaten.get(&food_id).copied().unwrap_or(0.0)
    }

    /// Haxe `isObjYum` using internal map + live YumBonus.
    // Haxe: GlobalPlayerInstance.isObjYum
    pub fn is_obj_yum_ex(&self, food_id: i32, food_value: i32, yum_bonus: f32) -> bool {
        is_obj_yum_ex(food_value, self.get_count_eaten(food_id), yum_bonus)
    }

    /// Haxe `isObjYum` using internal map at default [`YUM_BONUS`].
    pub fn is_obj_yum(&self, food_id: i32, food_value: i32) -> bool {
        self.is_obj_yum_ex(food_id, food_value, YUM_BONUS)
    }

    /// Haxe `isObjMeh` with live YumBonus.
    pub fn is_obj_meh_ex(&self, food_id: i32, food_value: i32, yum_bonus: f32) -> bool {
        is_obj_meh_ex(food_value, self.get_count_eaten(food_id), yum_bonus)
    }

    /// Haxe `isObjMeh`.
    pub fn is_obj_meh(&self, food_id: i32, food_value: i32) -> bool {
        self.is_obj_meh_ex(food_id, food_value, YUM_BONUS)
    }

    /// Haxe `isObjSuperMeh` with live YumBonus.
    pub fn is_obj_super_meh_ex(&self, food_id: i32, food_value: i32, yum_bonus: f32) -> bool {
        is_obj_super_meh_ex(food_value, self.get_count_eaten(food_id), yum_bonus)
    }

    /// Haxe `isObjSuperMeh`.
    pub fn is_obj_super_meh(&self, food_id: i32, food_value: i32) -> bool {
        self.is_obj_super_meh_ex(food_id, food_value, YUM_BONUS)
    }

    /// Haxe `isHoldingYum` with live YumBonus.
    pub fn is_holding_yum_ex(&self, held_id: i32, food_value: i32, yum_bonus: f32) -> bool {
        is_holding_yum_ex(
            held_id,
            food_value,
            self.get_count_eaten(held_id),
            yum_bonus,
        )
    }

    /// Haxe `isHoldingYum`.
    pub fn is_holding_yum(&self, held_id: i32, food_value: i32) -> bool {
        self.is_holding_yum_ex(held_id, food_value, YUM_BONUS)
    }

    /// Haxe `isHoldingMeh` with live YumBonus.
    pub fn is_holding_meh_ex(&self, held_id: i32, food_value: i32, yum_bonus: f32) -> bool {
        is_holding_meh_ex(
            held_id,
            food_value,
            self.get_count_eaten(held_id),
            yum_bonus,
        )
    }

    /// Haxe `isHoldingMeh`.
    pub fn is_holding_meh(&self, held_id: i32, food_value: i32) -> bool {
        self.is_holding_meh_ex(held_id, food_value, YUM_BONUS)
    }

    /// Haxe `canEatObj` with this map + live YumBonus.
    pub fn can_eat_obj_ex(
        &self,
        food_id: i32,
        food_value: i32,
        food_store: f32,
        food_max: f32,
        yum_bonus: f32,
    ) -> bool {
        can_eat_obj_ex(
            food_value,
            self.get_count_eaten(food_id),
            food_store,
            food_max,
            yum_bonus,
        )
    }

    /// Haxe `canEatObj` with this map.
    pub fn can_eat_obj(&self, food_id: i32, food_value: i32, food_store: f32, food_max: f32) -> bool {
        self.can_eat_obj_ex(food_id, food_value, food_store, food_max, YUM_BONUS)
    }

    /// Haxe `reduceFoodValue` without `reducesLongingFor` chain (base case).
    pub fn reduce_food_value(&mut self, food_id: i32, food_eaten: f32) {
        if food_id == 0 || food_eaten == 0.0 {
            return;
        }
        let e = self.has_eaten.entry(food_id).or_insert(0.0);
        *e += food_eaten;
    }

    /// Haxe `reduceFoodValue` with optional high-quality longing target.
    ///
    /// When `reduces_longing_for > 0`, apply scaled reduction to both ids
    /// (Haxe Berry Pie → Bowl of Berries style).
    pub fn reduce_food_value_chain(
        &mut self,
        food_id: i32,
        food_eaten: f32,
        reduces_longing_for: i32,
    ) {
        self.reduce_food_value_chain_ex(
            food_id,
            food_eaten,
            reduces_longing_for,
            FOOD_REDUCTION_FAKTOR_HIGH_QUALITY,
        );
    }

    /// Live-knob variant of [`Self::reduce_food_value_chain`].
    ///
    /// `high_quality_factor` = Haxe `FoodReductionFaktorForEatingHighQuailitFood`.
    // Haxe: GlobalPlayerInstance.reduceFoodValue + ServerSettings.FoodReductionFaktorForEatingHighQuailitFood
    // C-SS-TAIL-KNOBS
    pub fn reduce_food_value_chain_ex(
        &mut self,
        food_id: i32,
        food_eaten: f32,
        reduces_longing_for: i32,
        high_quality_factor: f32,
    ) {
        if reduces_longing_for < 1 {
            self.reduce_food_value(food_id, food_eaten);
            return;
        }
        let hq = if high_quality_factor.is_finite() && high_quality_factor >= 0.0 {
            high_quality_factor
        } else {
            FOOD_REDUCTION_FAKTOR_HIGH_QUALITY
        };
        let fe = food_eaten * hq * 0.5;
        self.reduce_food_value(food_id, fe);
        // recurse one level for longing target (Haxe may recurse; port one-level)
        self.reduce_food_value(reduces_longing_for, fe);
    }

    /// Haxe `restoreFoodCount` (subtract amount; push to `cravings` when ≤ 0).
    pub fn restore_food_count(&mut self, food_id: i32, amount: f32) {
        if food_id == 0 || amount == 0.0 {
            return;
        }
        let e = self.has_eaten.entry(food_id).or_insert(0.0);
        *e -= amount;
        let count = *e;
        if count <= 0.0 && !self.cravings.contains(&food_id) {
            self.cravings.push(food_id);
        }
    }

    /// Remove food id from craving list (Haxe `cravings.remove`).
    pub fn remove_craving(&mut self, food_id: i32) {
        self.cravings.retain(|&id| id != food_id);
    }

    /// Take pending CRAVING wire for outbound send.
    pub fn take_craving_wire(&mut self) -> Option<CravingWire> {
        self.pending_craving_wire.take()
    }

    /// Take pending displayFood (food id + world tile) for LS send.
    pub fn take_pending_display_food(&mut self) -> Option<PendingDisplayFood> {
        self.pending_display_food.take()
    }

    /// Record eating food with full live eat knobs (YumBonus / FoodFactor / reduction).
    ///
    /// `fill_before` is ceil(food_store) before the gain is applied.
    /// Returns fill to add (clamped ≥ 0). Does **not** apply refuse gates —
    /// caller should check [`can_eat_obj_ex`] / [`refuse_self_eat_super_meh`].
    // Haxe: GlobalPlayerInstance doEating (C-SS-FULL-TABLE / YUM-LIVE-SETTINGS)
    pub fn eat_full(
        &mut self,
        food_id: i32,
        base_value: f32,
        fill_before: i32,
        knobs: EatLiveKnobs,
    ) -> f32 {
        let k = knobs.resolved();
        let yb = k.yum_bonus;
        let base_i = base_value.round() as i32;
        let count = self.get_count_eaten(food_id);
        let computed = compute_eat_full(base_i, count, k);

        self.just_ate_id = food_id;
        self.just_ate = true;
        self.last_ate_fill_max = fill_before;

        if computed.has_eaten_delta != 0.0 {
            self.reduce_food_value(food_id, computed.has_eaten_delta);
        }

        self.history.push_back(food_id);
        while self.history.len() > self.capacity {
            self.history.pop_front();
        }

        // Wire yum_bonus field: remaining yum “charge” for this food after the eat.
        // (Haxe FX yum_bonus uses remaining band relative to live YumBonus setting.)
        let after = self.get_count_eaten(food_id);
        if computed.is_yum {
            self.yum_bonus = (yb - after.max(0.0)).max(0.0);
        } else {
            self.yum_bonus = 0.0;
        }
        // Small legacy bump so FX ceil stays visible on first yum eats.
        if computed.is_yum && self.yum_bonus < 0.1 {
            self.yum_bonus = 0.1;
        }

        computed.fill
    }

    /// Record eating food with live Haxe `ServerSettings.YumBonus` / FoodFactor.
    ///
    /// Reduction / health knobs stay at defaults; prefer [`eat_full`] for live
    /// FoodReduction* / HealthLost*.
    // Haxe: GlobalPlayerInstance doEating YumBonus (YUM-LIVE-SETTINGS)
    pub fn eat_ex(
        &mut self,
        food_id: i32,
        base_value: f32,
        fill_before: i32,
        yum_bonus: f32,
        food_factor: f32,
    ) -> f32 {
        self.eat_full(
            food_id,
            base_value,
            fill_before,
            EatLiveKnobs {
                yum_bonus,
                food_factor,
                ..EatLiveKnobs::default()
            },
        )
    }

    /// Record eating food with default [`YUM_BONUS`] / [`FOOD_FACTOR`].
    pub fn eat(&mut self, food_id: i32, base_value: f32, fill_before: i32) -> f32 {
        self.eat_ex(food_id, base_value, fill_before, YUM_BONUS, FOOD_FACTOR)
    }

    /// Eat with full gate checks + live YumBonus; returns None if refused.
    // Haxe: GlobalPlayerInstance canEatObj + doEating (YUM-LIVE-SETTINGS)
    pub fn try_eat_ex(
        &mut self,
        food_id: i32,
        base_value: f32,
        fill_before: i32,
        food_store: f32,
        food_store_max: f32,
        yum_bonus: f32,
        food_factor: f32,
    ) -> Option<EatCompute> {
        let yb = resolve_yum_bonus(yum_bonus);
        let base_i = base_value.round() as i32;
        let count = self.get_count_eaten(food_id);
        if !can_eat_obj_ex(base_i, count, food_store, food_store_max, yb) {
            return None;
        }
        let computed = compute_eat_ex(base_i, count, yb, food_factor);
        if refuse_self_eat_super_meh(computed.is_super_meh, food_store) {
            return None;
        }
        let _ = self.eat_ex(food_id, base_value, fill_before, yb, food_factor);
        Some(computed)
    }

    /// Eat with full gate checks; returns None if refused.
    pub fn try_eat(
        &mut self,
        food_id: i32,
        base_value: f32,
        fill_before: i32,
        food_store: f32,
        food_store_max: f32,
    ) -> Option<EatCompute> {
        self.try_eat_ex(
            food_id,
            base_value,
            fill_before,
            food_store,
            food_store_max,
            YUM_BONUS,
            FOOD_FACTOR,
        )
    }

    /// Clear the transient `just_ate` flag after PU/FX fan-out (Haxe post-PU).
    pub fn clear_just_ate_flag(&mut self) {
        self.just_ate = false;
    }

    /// Wire ints for FX / PU.
    pub fn just_ate_flag(&self) -> i32 {
        if self.just_ate {
            1
        } else {
            0
        }
    }

    pub fn yum_bonus_ceil(&self) -> i32 {
        self.yum_bonus.ceil() as i32
    }

    /// Human-readable `?YUM` reply body (without player id).
    pub fn query_text(&self) -> String {
        format!(
            "YUM bonus={} history={} eaten={} craving={}",
            self.yum_bonus,
            self.history.len(),
            self.has_eaten.len(),
            self.currently_craving
        )
    }

    /// Clear YUM history / hasEaten / bonus / just-ate / cravings (SAY CLEAR YUM / RESET YUM).
    pub fn clear(&mut self) {
        self.has_eaten.clear();
        self.history.clear();
        self.yum_bonus = 0.0;
        self.just_ate_id = 0;
        self.just_ate = false;
        self.last_ate_fill_max = 0;
        self.currently_craving = 0;
        self.cravings.clear();
        self.last_craving_index = 0;
        self.pending_craving_wire = None;
        self.pending_display_food = None;
    }

    /// Haxe `doIncreaseFoodValue` — restore random + loved foods; pick/send CRAVING.
    ///
    /// Call **after** hasEaten reduce for the eaten food (not on superMeh).
    /// `food_objects` is Haxe `ObjectData.foodObjects` order (food_value ≥ 1).
    /// `nearby_best` is optional SearchBestFood lite hit (id, count, tx, ty).
    /// RNG: `rand_int(max)` → 0..=max inclusive; `rand_f01()` → [0,1).
    // Haxe: GlobalPlayerInstance.doIncreaseFoodValue L3350-3480
    pub fn do_increase_food_value(
        &mut self,
        eaten_food_id: i32,
        amount_eaten: f32,
        dont_change_craving: bool,
        loved_food_ids: &[i32],
        food_objects: &[i32],
        nearby_best: Option<NearbyBestFood>,
        rand_int: impl FnMut(i32) -> i32,
        rand_f01: impl FnMut() -> f32,
    ) -> Option<CravingWire> {
        self.do_increase_food_value_ex(
            eaten_food_id,
            amount_eaten,
            dont_change_craving,
            loved_food_ids,
            food_objects,
            nearby_best,
            YumRestoreKnobs::default(),
            rand_int,
            rand_f01,
        )
    }

    /// `do_increase_food_value` with live restore knobs (Yum / Loved / NewCraving).
    // Haxe: ServerSettings.YumFoodRestore / LovedFoodRestore / YumNewCravingChance
    // C-SS-FULL-TABLE
    pub fn do_increase_food_value_ex(
        &mut self,
        eaten_food_id: i32,
        amount_eaten: f32,
        dont_change_craving: bool,
        loved_food_ids: &[i32],
        food_objects: &[i32],
        nearby_best: Option<NearbyBestFood>,
        restore: YumRestoreKnobs,
        mut rand_int: impl FnMut(i32) -> i32,
        mut rand_f01: impl FnMut() -> f32,
    ) -> Option<CravingWire> {
        let r = restore.resolved();
        let yfr = r.yum_food_restore;
        let lfr = r.loved_food_restore;
        let new_craving_chance = r.yum_new_craving_chance;
        self.pending_craving_wire = None;
        self.pending_display_food = None;

        // Haxe: if (hasEatenMap[eatenFoodId] > 0) cravings.remove(eatenFoodId);
        if self.get_count_eaten(eaten_food_id) > 0.0 {
            self.remove_craving(eaten_food_id);
        }

        let has_eaten_keys: Vec<i32> = self.has_eaten.keys().copied().collect();
        if has_eaten_keys.is_empty() {
            return None;
        }

        let max_key = (has_eaten_keys.len() as i32) - 1;
        let random = rand_int(max_key.max(0)).clamp(0, max_key.max(0)) as usize;
        let random = random.min(has_eaten_keys.len().saturating_sub(1));
        let key = has_eaten_keys[random];

        // Snapshot craving count *before* restore (Haxe port-as-is).
        let craving_has_eaten_count = self.get_count_eaten(self.currently_craving);

        if key != eaten_food_id {
            self.restore_food_count(key, amount_eaten * yfr);
        }

        for &food_id in loved_food_ids {
            self.restore_food_count(food_id, amount_eaten * lfr);
        }

        // Display adjust: full YUM shows as +1 craving on wire.
        let craving_display = craving_has_eaten_count - 1.0;

        let keep_craving = craving_has_eaten_count < 0.0
            && self.currently_craving != 0
            && (dont_change_craving || self.currently_craving == eaten_food_id);

        let wire = if keep_craving {
            // Haxe: send CRAVING currentlyCraving ${- cravingHasEatenCount} (after --)
            CravingWire {
                food_id: self.currently_craving,
                bonus: (-craving_display).round() as i32,
            }
        } else if self.cravings.is_empty() || rand_f01() < new_craving_chance {
            // Random new craving from foodObjects
            self.currently_craving = 0;
            let mut index = 0i32;
            let mut found = false;
            for i in 0..31 {
                let mut idx = self.last_craving_index + rand_int(6 + i) - 3;
                if idx == self.last_craving_index {
                    idx += 1;
                }
                if idx < 0 {
                    continue;
                }
                if (idx as usize) >= food_objects.len() {
                    continue;
                }
                let new_id = food_objects[idx as usize];
                if self.get_count_eaten(new_id) > 0.0 {
                    continue;
                }
                index = idx;
                found = true;
                break;
            }
            if !found {
                let w = CravingWire {
                    food_id: self.currently_craving,
                    bonus: 0,
                };
                self.pending_craving_wire = Some(w);
                return Some(w);
            }
            let new_id = food_objects[index as usize];
            if !self.has_eaten.contains_key(&new_id) {
                self.has_eaten.insert(new_id, -1.0);
            }
            let mut nh = self.get_count_eaten(new_id);
            nh -= 1.0;
            self.last_craving_index = index;
            self.currently_craving = new_id;
            CravingWire {
                food_id: new_id,
                bonus: (-nh).round() as i32,
            }
        } else {
            // Nearby best or known cravings list
            self.currently_craving = 0; // ignore while scoring nearby
            let mut food_id = 0i32;
            let mut nh = 1.0f32;
            let mut best_tx = 0i32;
            let mut best_ty = 0i32;
            if let Some(nb) = nearby_best {
                food_id = nb.food_id;
                nh = nb.count_eaten;
                best_tx = nb.tx;
                best_ty = nb.ty;
            }
            if nh > 0.0 {
                // Pick from known craving list
                if !self.cravings.is_empty() {
                    let max_c = (self.cravings.len() as i32) - 1;
                    let r = rand_int(max_c.max(0)).clamp(0, max_c.max(0)) as usize;
                    let r = r.min(self.cravings.len() - 1);
                    food_id = self.cravings[r];
                    nh = self.get_count_eaten(food_id);
                }
            } else if food_id != 0 {
                // Haxe: displayFood(bestfood) — LS after CR send
                self.pending_display_food = Some(PendingDisplayFood {
                    food_id,
                    tx: best_tx,
                    ty: best_ty,
                });
            }
            nh -= 1.0;
            // Haxe wire uses foodId then sets currentlyCraving = key (bug L3475).
            // Intentional delta: currently_craving = food_id (matches CR wire + YY).
            let _haxe_key_bug = key;
            self.currently_craving = food_id;
            CravingWire {
                food_id,
                bonus: (-nh).round() as i32,
            }
        };

        self.pending_craving_wire = Some(wire);
        Some(wire)
    }

    /// Format displayFood label for a world food object with live YumBonus.
    // Haxe: GlobalPlayerInstance.displayFood
    pub fn display_food_label_ex(
        &self,
        food_id: i32,
        food_value: i32,
        dist: i32,
        yum_bonus: f32,
    ) -> String {
        format_display_food_label_ex(
            food_id,
            food_value,
            self.get_count_eaten(food_id),
            self.currently_craving,
            dist,
            yum_bonus,
        )
    }

    /// Format displayFood label for a world food object.
    pub fn display_food_label(
        &self,
        food_id: i32,
        food_value: i32,
        dist: i32,
    ) -> String {
        self.display_food_label_ex(food_id, food_value, dist, YUM_BONUS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C-SS-MORE-KNOBS: InheritEatenFoodCounts reduction + clamp + negatives preserved.
    // Haxe: GlobalPlayerInstance.InheritEatenFoodCounts
    #[test]
    fn inherit_eaten_food_counts_reduction_clamp_negatives() {
        // count > 0: subtract reduction, floor 0
        assert!((inherit_eaten_food_count(3.0, 1.0, 4.0) - 2.0).abs() < 1e-5);
        assert!((inherit_eaten_food_count(0.5, 1.0, 4.0) - 0.0).abs() < 1e-5);
        // clamp to max
        assert!((inherit_eaten_food_count(10.0, 1.0, 4.0) - 4.0).abs() < 1e-5);
        // zero stays zero (not reduced)
        assert!((inherit_eaten_food_count(0.0, 1.0, 4.0) - 0.0).abs() < 1e-5);
        // negatives preserved (not reduced, not clamped upward)
        assert!((inherit_eaten_food_count(-2.0, 1.0, 4.0) - (-2.0)).abs() < 1e-5);
        // custom knobs
        assert!((inherit_eaten_food_count(5.0, 0.5, 6.0) - 4.5).abs() < 1e-5);

        let mut src = HashMap::new();
        src.insert(33, 5.0); // → 4 after red 1, then clamp max 4
        src.insert(40, -1.5); // keep
        src.insert(77, 0.2); // → 0
        let out = inherit_eaten_food_counts(&src, 1.0, 4.0);
        assert!((out[&33] - 4.0).abs() < 1e-5);
        assert!((out[&40] - (-1.5)).abs() < 1e-5);
        assert!((out[&77] - 0.0).abs() < 1e-5);
    }

    /// C-SS-MORE-KNOBS: gender source — girls father line, boys mother line; prefer grandparent.
    // Haxe: InheritEatenFoodCounts L1350-1353
    #[test]
    fn inherit_eaten_source_parent_gender_and_grandparent() {
        // Boy: mother.mother if present else mother
        assert_eq!(
            inherit_eaten_source_parent_id(false, 10, Some(11), Some(20), Some(21)),
            11
        );
        assert_eq!(
            inherit_eaten_source_parent_id(false, 10, None, Some(20), Some(21)),
            10
        );
        // Girl with father: father.father ?? father
        assert_eq!(
            inherit_eaten_source_parent_id(true, 10, Some(11), Some(20), Some(21)),
            21
        );
        assert_eq!(
            inherit_eaten_source_parent_id(true, 10, Some(11), Some(20), None),
            20
        );
        // Girl without father: fall back to mother line
        assert_eq!(
            inherit_eaten_source_parent_id(true, 10, Some(11), None, None),
            11
        );
    }

    #[test]
    fn is_obj_yum_true_when_fresh() {
        assert!(is_obj_yum(5, 0.0));
        assert!(is_obj_yum(5, 4.9));
        assert!(!is_obj_yum(5, 5.0));
        assert!(!is_obj_yum(5, 6.0));
        assert!(!is_obj_yum(0, 0.0));
        assert!(!is_obj_yum(-1, 0.0));
        // craving (negative count) is still yum
        assert!(is_obj_yum(5, -2.0));
    }

    #[test]
    fn is_obj_super_meh_after_many_eats() {
        // fresh: 5 + 5 - 0 = 10 >= 5/2 → not super
        assert!(!is_obj_super_meh(5, 0.0));
        // after 5 yum eats: 5+5-5 = 5 >= 2.5 → not super yet
        assert!(!is_obj_super_meh(5, 5.0));
        // when adjusted < food/2: 5+5-count < 2.5 → count > 7.5
        assert!(is_obj_super_meh(5, 8.0));
        assert!(!is_obj_super_meh(5, 7.0));
    }

    #[test]
    fn is_obj_meh_is_not_yum() {
        assert!(!is_obj_meh(5, 0.0));
        assert!(is_obj_meh(5, 5.0));
        assert!(is_obj_meh(0, 0.0));
    }

    #[test]
    fn holding_yum_and_meh() {
        assert!(is_holding_yum(33, 5, 0.0));
        assert!(!is_holding_yum(0, 5, 0.0));
        assert!(!is_holding_meh(33, 5, 0.0));
        assert!(!is_holding_meh(33, 5, 5.0)); // strict >
        assert!(is_holding_meh(33, 5, 5.1));
    }

    #[test]
    fn hold_food_emote_joy_sad_hmph() {
        assert_eq!(hold_food_emote(33, 5, 0.0), Some("JOY"));
        assert_eq!(hold_food_emote(33, 5, 8.0), Some("SAD")); // super meh
        assert_eq!(hold_food_emote(33, 5, 5.5), Some("HMPH")); // meh not super
        assert_eq!(hold_food_emote(0, 5, 0.0), None);
        assert_eq!(hold_food_emote(33, 0, 0.0), None);
    }

    #[test]
    fn can_eat_obj_gates() {
        // fresh food, room ok
        assert!(can_eat_obj(5, 0.0, 10.0, 20.0));
        // full — need ceil(5/4)=2 room
        assert!(!can_eat_obj(5, 0.0, 19.0, 20.0));
        assert!(can_eat_obj(5, 0.0, 18.0, 20.0));
        // super meh + food_store > 4 refuse
        assert!(!can_eat_obj(5, 8.0, 10.0, 20.0));
        // super meh when starving allowed
        assert!(can_eat_obj(5, 8.0, 3.0, 20.0));
        assert!(!can_eat_obj(0, 0.0, 0.0, 20.0));
    }

    #[test]
    fn can_feed_meh_only_when_starving() {
        // meh (count >= 5) and food_store > 2
        assert!(!can_feed_to_me_obj(5, 5.0, 10.0, 20.0));
        assert!(can_feed_to_me_obj(5, 5.0, 2.0, 20.0));
        // yum always ok if room
        assert!(can_feed_to_me_obj(5, 0.0, 10.0, 20.0));
        // Haxe: 837 only when hasYellowFever
        assert!(!can_feed_to_me_obj_ex(
            PSILOCYBE_MUSHROOM_ID,
            3,
            0.0,
            10.0,
            20.0,
            false
        ));
        assert!(can_feed_to_me_obj_ex(
            PSILOCYBE_MUSHROOM_ID,
            3,
            0.0,
            10.0,
            20.0,
            true
        ));
    }

    #[test]
    fn compute_eat_first_yum_gains_bonus() {
        let e = compute_eat(5, 0.0);
        assert!(e.is_yum);
        assert!(!e.is_super_meh);
        // 5 + 5 - 0 = 10
        assert!((e.fill - 10.0).abs() < 1e-4);
        assert!((e.has_eaten_delta - 1.0).abs() < 1e-4);
        assert!(e.health_delta > 0.0);
    }

    /// YUM-LIVE-SETTINGS: live yum_bonus changes first-eat fill and band edges.
    // Haxe: ServerSettings.YumBonus hot-reload
    #[test]
    fn compute_eat_ex_uses_live_yum_bonus() {
        // default 5 → fill 5+5=10
        let d = compute_eat_ex(5, 0.0, 5.0, 1.0);
        assert!((d.fill - 10.0).abs() < 1e-4);
        assert!(d.is_yum);
        // live 7 → fill 5+7=12
        let e = compute_eat_ex(5, 0.0, 7.0, 1.0);
        assert!((e.fill - 12.0).abs() < 1e-4);
        assert!(e.is_yum);
        // count=5 still yum when band is 7
        assert!(is_obj_yum_ex(5, 5.0, 7.0));
        assert!(!is_obj_yum_ex(5, 5.0, 5.0));
        // live 3: count=3 is meh; first fill 5+3=8
        let f = compute_eat_ex(5, 0.0, 3.0, 1.0);
        assert!((f.fill - 8.0).abs() < 1e-4);
        assert!(!is_obj_yum_ex(5, 3.0, 3.0));
        // food_factor scales fill
        let g = compute_eat_ex(5, 0.0, 5.0, 0.5);
        assert!((g.fill - 5.0).abs() < 1e-4);
    }

    /// C-SS-FULL-TABLE: live FoodReduction* / HealthLost* via compute_eat_full.
    // Haxe: ServerSettings.FoodReductionPerEating / HealthLostWhenEating*
    #[test]
    fn compute_eat_full_live_reduction_and_health() {
        let mut k = EatLiveKnobs::default();
        k.food_reduction_per_eating = 2.0;
        k.food_reduction_faktor_meh = 0.5;
        k.health_lost_meh = 1.25;
        k.health_lost_super_meh = 4.0;
        // Fresh yum: hasEaten delta = 2.0
        let yum = compute_eat_full(5, 0.0, k);
        assert!(yum.is_yum);
        assert!((yum.has_eaten_delta - 2.0).abs() < 1e-4);
        assert!((yum.health_delta - 2.0).abs() < 1e-4);
        // Meh: count >= yum_bonus → reduction *= 0.5; health = -1.25
        let meh = compute_eat_full(5, 5.0, k);
        assert!(!meh.is_yum && !meh.is_super_meh);
        assert!((meh.has_eaten_delta - 1.0).abs() < 1e-4); // 2.0 * 0.5
        assert!((meh.health_delta + 1.25).abs() < 1e-4);
        // Super meh: health = -4
        let sm = compute_eat_full(5, 20.0, k);
        assert!(sm.is_super_meh);
        assert_eq!(sm.has_eaten_delta, 0.0);
        assert!((sm.health_delta + 4.0).abs() < 1e-4);
    }

    /// C-SS-FULL-TABLE: sanitize YumRestoreKnobs.
    #[test]
    fn yum_restore_knobs_resolved_sanitize() {
        let bad = YumRestoreKnobs {
            yum_food_restore: -1.0,
            loved_food_restore: f32::NAN,
            yum_new_craving_chance: -0.5,
        }
        .resolved();
        assert!((bad.yum_food_restore - YUM_FOOD_RESTORE).abs() < 1e-4);
        assert!((bad.loved_food_restore - LOVED_FOOD_RESTORE).abs() < 1e-4);
        assert!((bad.yum_new_craving_chance - YUM_NEW_CRAVING_CHANCE).abs() < 1e-4);
    }

    #[test]
    fn is_obj_super_meh_ex_flips_with_live_band() {
        // default: super when count > 7.5 for food_value 5
        assert!(is_obj_super_meh_ex(5, 8.0, 5.0));
        assert!(!is_obj_super_meh_ex(5, 7.0, 5.0));
        // yum_bonus=3: adjusted = 5+3-count < 2.5 → count > 5.5
        assert!(is_obj_super_meh_ex(5, 6.0, 3.0));
        assert!(!is_obj_super_meh_ex(5, 5.0, 3.0));
        // yum_bonus=7: need count > 9.5
        assert!(!is_obj_super_meh_ex(5, 8.0, 7.0));
        assert!(is_obj_super_meh_ex(5, 10.0, 7.0));
    }

    #[test]
    fn format_display_food_text_ex_u_count_uses_live_band() {
        // yum_bonus=3, fresh → 3 U
        assert_eq!(
            format_display_food_text_ex(33, 5, 0.0, 0, 3.0),
            "YUUUM!"
        );
        // yum_bonus=7, count=0 → clamp U to 5
        assert_eq!(
            format_display_food_text_ex(33, 5, 0.0, 0, 7.0),
            "YUUUUUM!"
        );
        // yum_bonus=7, count=4 still yum → 3 U
        assert_eq!(
            format_display_food_text_ex(33, 5, 4.0, 0, 7.0),
            "YUUUM!"
        );
        // count=5 with yum_bonus=3 is meh
        let meh = format_display_food_text_ex(33, 5, 5.0, 0, 3.0);
        assert!(meh.starts_with('M') && meh.ends_with("H!"), "got {meh}");
    }

    #[test]
    fn eat_ex_and_try_eat_ex_use_live_yum_bonus() {
        let mut y = YumState::default();
        let fill = y.eat_ex(33, 5.0, 5, 7.0, 1.0);
        assert!((fill - 12.0).abs() < 1e-4); // 5+7
        // remaining wire charge: 7 - 1 = 6
        assert!((y.yum_bonus - 6.0).abs() < 1e-4);

        let mut y2 = YumState::default();
        // count_eaten=5 is still yum under band 7 → allowed when store ok
        y2.reduce_food_value(33, 5.0);
        assert!(y2.is_obj_yum_ex(33, 5, 7.0));
        let r = y2.try_eat_ex(33, 5.0, 5, 10.0, 20.0, 7.0, 1.0);
        assert!(r.is_some());
        assert!(r.unwrap().is_yum);
        // under default band, count=5 is meh not yum
        let mut y3 = YumState::default();
        y3.reduce_food_value(33, 5.0);
        assert!(!y3.is_obj_yum(33, 5));
    }

    #[test]
    fn resolve_yum_bonus_sanitizes() {
        assert!((resolve_yum_bonus(7.0) - 7.0).abs() < 1e-6);
        assert!((resolve_yum_bonus(-1.0) - YUM_BONUS).abs() < 1e-6);
        assert!((resolve_yum_bonus(f32::NAN) - YUM_BONUS).abs() < 1e-6);
        assert!((resolve_yum_bonus(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn compute_eat_after_many_is_super_meh() {
        let e = compute_eat(5, 8.0);
        assert!(!e.is_yum);
        assert!(e.is_super_meh);
        assert!((e.fill - 2.5).abs() < 1e-4); // base/2
        assert_eq!(e.has_eaten_delta, 0.0);
        assert!((e.health_delta + HEALTH_LOST_SUPER_MEH).abs() < 1e-4);
    }

    #[test]
    fn compute_eat_meh_not_super() {
        // count=5 → adjusted = 5+5-5=5 >= 2.5 → not super; not yum
        let e = compute_eat(5, 5.0);
        assert!(!e.is_yum);
        assert!(!e.is_super_meh);
        assert!((e.fill - 5.0).abs() < 1e-4);
        assert!((e.has_eaten_delta - FOOD_REDUCTION_FAKTOR_MEH).abs() < 1e-4);
    }

    #[test]
    fn display_food_yum_and_meh_text() {
        // fresh: Y + 5 U + M!
        assert_eq!(
            format_display_food_text(33, 5, 0.0, 0),
            "YUUUUUM!"
        );
        // craving match
        assert_eq!(
            format_display_food_text(33, 5, 0.0, 33),
            "YYUUUUUM!"
        );
        // partial yum count=3 → ceil(5-3)=2 U
        assert_eq!(
            format_display_food_text(33, 5, 3.0, 0),
            "YUUM!"
        );
        // meh count=5: floor(1 + 0/(5/4)) = 1 E
        let meh = format_display_food_text(33, 5, 5.0, 0);
        assert!(meh.starts_with("M") && meh.ends_with("H!"), "got {meh}");
        assert!(meh.contains('E'));
    }

    #[test]
    fn display_food_distance_suffix() {
        assert_eq!(append_distance_suffix("YUM!", 5), "YUM!");
        assert_eq!(append_distance_suffix("YUM!", 10), "YUM!_10M");
        let full = format_display_food_label(33, 5, 0.0, 0, 12);
        assert!(full.ends_with("_12M"));
    }

    #[test]
    fn display_best_food_gates() {
        assert!(!should_display_best_food(
            false, false, 0, 0, 5.0, 20.0, 0.5, 100.0
        ));
        // holding yum blocks unless craving
        assert!(!should_display_best_food(
            true, true, 40, 0, 5.0, 20.0, 0.5, 100.0
        ));
        assert!(should_display_best_food(
            true, true, 40, 40, 5.0, 20.0, 0.5, 100.0
        ));
        // full (store >= max * factor)
        assert!(!should_display_best_food(
            true, false, 40, 0, 10.0, 20.0, 0.5, 100.0
        ));
        // close (quad <= 10)
        assert!(!should_display_best_food(
            true, false, 40, 0, 5.0, 20.0, 0.5, 10.0
        ));
        // ok
        assert!(should_display_best_food(
            true, false, 40, 0, 5.0, 20.0, 0.5, 11.0
        ));
    }

    #[test]
    fn has_eaten_map_restore_reduce() {
        let mut y = YumState::default();
        assert_eq!(y.get_count_eaten(33), 0.0);
        y.reduce_food_value(33, 1.0);
        assert!((y.get_count_eaten(33) - 1.0).abs() < 1e-5);
        y.restore_food_count(33, 0.5);
        assert!((y.get_count_eaten(33) - 0.5).abs() < 1e-5);
        y.reduce_food_value_chain(100, 2.0, 50);
        // default HQ 0.8 → fe = 2 * 0.8 * 0.5 = 0.8
        assert!((y.get_count_eaten(100) - 0.8).abs() < 1e-5);
        assert!((y.get_count_eaten(50) - 0.8).abs() < 1e-5);
        // C-SS-TAIL-KNOBS live high-quality factor
        let mut y2 = YumState::default();
        y2.reduce_food_value_chain_ex(100, 2.0, 50, 0.5);
        // fe = 2 * 0.5 * 0.5 = 0.5
        assert!((y2.get_count_eaten(100) - 0.5).abs() < 1e-5);
        assert!((y2.get_count_eaten(50) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn eat_updates_has_eaten_and_fill() {
        let mut y = YumState::default();
        let a = y.eat(33, 5.0, 5);
        assert!((a - 10.0).abs() < 1e-4); // 5+5
        assert!((y.get_count_eaten(33) - 1.0).abs() < 1e-4);
        assert_eq!(y.just_ate_id, 33);
        assert!(y.just_ate);
        assert_eq!(y.last_ate_fill_max, 5);
        assert_eq!(y.history.len(), 1);

        // second eat: 5+5-1 = 9
        let b = y.eat(33, 5.0, 8);
        assert!((b - 9.0).abs() < 1e-4);
        assert!((y.get_count_eaten(33) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn try_eat_refuses_super_meh_when_full() {
        let mut y = YumState::default();
        for _ in 0..10 {
            y.reduce_food_value(33, 1.0);
        }
        assert!(y.is_obj_super_meh(33, 5));
        assert!(y
            .try_eat(33, 5.0, 10, 10.0, 20.0)
            .is_none());
        // starving: can_eat allows super meh when store <= 4
        let r = y.try_eat(33, 5.0, 2, 2.0, 20.0);
        assert!(r.is_some());
        assert!(r.unwrap().is_super_meh);
    }

    #[test]
    fn query_text_includes_bonus_and_history_len() {
        let mut y = YumState::default();
        let _ = y.eat(33, 3.0, 0);
        let t = y.query_text();
        assert!(t.contains("bonus="));
        assert!(t.contains("history=1"));
        assert!(t.contains("eaten="));
        assert!(t.starts_with("YUM "));
    }

    #[test]
    fn clear_resets_history_bonus_and_just_ate() {
        let mut y = YumState::default();
        let _ = y.eat(33, 3.0, 5);
        let _ = y.eat(40, 3.0, 8);
        y.currently_craving = 40;
        y.cravings.push(40);
        assert!(!y.history.is_empty());
        assert!(!y.has_eaten.is_empty());
        assert!(y.just_ate);
        y.clear();
        assert!(y.history.is_empty());
        assert!(y.has_eaten.is_empty());
        assert_eq!(y.yum_bonus, 0.0);
        assert_eq!(y.just_ate_id, 0);
        assert!(!y.just_ate);
        assert_eq!(y.last_ate_fill_max, 0);
        assert_eq!(y.currently_craving, 0);
        assert!(y.cravings.is_empty());
    }

    #[test]
    fn pick_best_prefers_yum_nearby() {
        let cands = [
            FoodCandidate {
                food_id: 1,
                food_value: 3,
                tx: 50,
                ty: 0,
                count_eaten: 0.0,
            },
            FoodCandidate {
                food_id: 2,
                food_value: 3,
                tx: 2,
                ty: 0,
                count_eaten: 0.0,
            },
            FoodCandidate {
                food_id: 3,
                food_value: 3,
                tx: 2,
                ty: 0,
                count_eaten: 10.0, // super meh-ish
            },
        ];
        let i = pick_best_food(&cands, 0, 0, 5.0, 20.0, 0).unwrap();
        assert_eq!(i, 1); // closer yum
    }

    #[test]
    fn yum_bonus_on_variety_legacy_shape() {
        // First eats still improve fill vs depleted hasEaten
        let mut y = YumState::default();
        let a = y.eat(33, 3.0, 5);
        let b = y.eat(33, 3.0, 8);
        assert!(a > b);
        let c = y.eat(40, 3.0, 10);
        // fresh id gets full YumBonus again
        assert!(c >= b);
        assert_eq!(y.just_ate_id, 40);
        assert!(y.just_ate);
        assert_eq!(y.last_ate_fill_max, 10);
        assert_eq!(y.history.len(), 3);
    }

    // -----------------------------------------------------------------------
    // CRAVING-WIRE
    // -----------------------------------------------------------------------

    #[test]
    fn restore_food_count_pushes_cravings() {
        let mut y = YumState::default();
        y.reduce_food_value(33, 2.0);
        y.restore_food_count(33, 2.5);
        assert!((y.get_count_eaten(33) + 0.5).abs() < 1e-4);
        assert!(y.cravings.contains(&33));
        y.restore_food_count(33, 0.1);
        assert_eq!(y.cravings.iter().filter(|&&id| id == 33).count(), 1);
    }

    #[test]
    fn loved_food_ids_by_person_color() {
        assert_eq!(loved_food_ids_for_person_color(PERSON_BROWN), &[2143, 1880]);
        assert_eq!(loved_food_ids_for_person_color(PERSON_BLACK), &[768, 197]);
        assert_eq!(loved_food_ids_for_person_color(PERSON_WHITE), &[4252, 1242]);
        assert_eq!(loved_food_ids_for_person_color(PERSON_GINGER), &[40, 2106]);
        assert!(loved_food_ids_for_person_color(0).is_empty());
    }

    #[test]
    fn do_increase_keeps_current_craving_when_eating_it() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0);
        y.has_eaten.insert(40, -2.0); // after reduce already applied
        y.currently_craving = 40;
        y.cravings.push(40);
        let foods = [10, 20, 30, 40, 50];
        let wire = y
            .do_increase_food_value(
                40,
                1.0,
                false,
                &[],
                &foods,
                None,
                |_| 0,
                || 0.99,
            )
            .expect("wire");
        assert_eq!(wire.food_id, 40);
        // craving_has = -2, display = -3, bonus = 3
        assert_eq!(wire.bonus, 3);
        assert_eq!(y.currently_craving, 40);
        assert_eq!(y.take_craving_wire(), Some(wire));
    }

    #[test]
    fn do_increase_dont_change_keeps_craving_on_meh() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 6.0); // meh eaten
        y.has_eaten.insert(40, -1.0);
        y.currently_craving = 40;
        let foods = [40, 50];
        let wire = y
            .do_increase_food_value(
                33,
                0.2,
                true, // dont_change_craving (meh / feed-other)
                &[],
                &foods,
                None,
                |_| 0,
                || 0.0,
            )
            .expect("wire");
        assert_eq!(wire.food_id, 40);
        assert_eq!(y.currently_craving, 40);
    }

    #[test]
    fn do_increase_loved_food_restore() {
        let mut y = YumState::default();
        // Single hasEaten key (= eaten) so random YumFoodRestore never hits banana.
        y.has_eaten.insert(33, 1.0);
        y.has_eaten.insert(2143, 2.0); // banana
        // Force random key pick to index of 33 regardless of HashMap order.
        let foods = [33, 2143];
        let loved = loved_food_ids_for_person_color(PERSON_BROWN);
        assert!(loved.contains(&2143));
        let keys_snapshot: Vec<i32> = y.has_eaten.keys().copied().collect();
        let idx_33 = keys_snapshot.iter().position(|&k| k == 33).unwrap() as i32;
        let _ = y.do_increase_food_value(
            33,
            1.0,
            true,
            loved,
            &foods,
            None,
            |_| idx_33,
            || 0.5,
        );
        assert!((y.get_count_eaten(2143) - (2.0 - LOVED_FOOD_RESTORE)).abs() < 1e-4);
    }

    /// C-SS-FULL-TABLE: live LovedFoodRestore override.
    // Haxe: ServerSettings.LovedFoodRestore
    #[test]
    fn do_increase_ex_live_loved_food_restore() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0);
        y.has_eaten.insert(2143, 2.0);
        let foods = [33, 2143];
        let loved = loved_food_ids_for_person_color(PERSON_BROWN);
        let keys_snapshot: Vec<i32> = y.has_eaten.keys().copied().collect();
        let idx_33 = keys_snapshot.iter().position(|&k| k == 33).unwrap() as i32;
        let restore = YumRestoreKnobs {
            yum_food_restore: 0.0, // no random restore noise
            loved_food_restore: 0.5,
            yum_new_craving_chance: 0.0,
        };
        let _ = y.do_increase_food_value_ex(
            33,
            1.0,
            true,
            loved,
            &foods,
            None,
            restore,
            |_| idx_33,
            || 0.5,
        );
        assert!((y.get_count_eaten(2143) - (2.0 - 0.5)).abs() < 1e-4);
    }

    #[test]
    fn do_increase_new_random_craving_from_food_objects() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0);
        y.currently_craving = 0;
        y.last_craving_index = 0;
        // foodObjects with uneaten ids.
        let foods = [100, 101, 102];
        let wire = y
            .do_increase_food_value(
                33,
                1.0,
                false,
                &[],
                &foods,
                None,
                |_| {
                    // key pick: max_key=0 → clamp 0
                    // index walk: last(0) + r - 3; r=3 → idx=0 == last → idx++
                    // → foods[1] = 101
                    3
                },
                || 0.0, // force new craving branch
            )
            .expect("wire");
        assert_eq!(wire.food_id, 101);
        assert_eq!(y.currently_craving, 101);
        // seeded -1 then display -- → bonus 2
        assert_eq!(wire.bonus, 2);
        assert!((y.get_count_eaten(101) + 1.0).abs() < 1e-4);
        assert_eq!(y.last_craving_index, 1);
    }

    #[test]
    fn do_increase_picks_from_cravings_list() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0);
        y.has_eaten.insert(77, -1.0);
        y.cravings.push(77);
        y.currently_craving = 0;
        let foods = [33, 77, 99];
        // amount 0 → no random YumFoodRestore noise on 77's count.
        let wire = y
            .do_increase_food_value(
                33,
                0.0,
                false,
                &[],
                &foods,
                Some(NearbyBestFood {
                    food_id: 50,
                    count_eaten: 2.0, // positive → use cravings list
                    tx: 5,
                    ty: 6,
                }),
                |_| 0,
                || 0.99, // > YumNewCravingChance → list branch
            )
            .expect("wire");
        assert_eq!(wire.food_id, 77);
        assert_eq!(y.currently_craving, 77);
        // count -1, display -2, bonus 2
        assert_eq!(wire.bonus, 2);
        assert!(y.take_pending_display_food().is_none());
    }

    #[test]
    fn do_increase_nearby_best_sets_pending_display() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0);
        y.has_eaten.insert(88, 0.0);
        y.cravings.push(88); // non-empty so we can take nearby branch
        y.currently_craving = 0;
        let foods = [33, 88, 99];
        let wire = y
            .do_increase_food_value(
                33,
                1.0,
                false,
                &[],
                &foods,
                Some(NearbyBestFood {
                    food_id: 88,
                    count_eaten: 0.0, // ≤0 → keep nearby + displayFood
                    tx: 12,
                    ty: -3,
                }),
                |_| 0,
                || 0.99, // skip random foodObjects branch
            )
            .expect("wire");
        assert_eq!(wire.food_id, 88);
        assert_eq!(y.currently_craving, 88);
        let pend = y.take_pending_display_food().expect("pending display");
        assert_eq!(pend.food_id, 88);
        assert_eq!(pend.tx, 12);
        assert_eq!(pend.ty, -3);
    }

    #[test]
    fn dont_change_craving_feed_other_always_true() {
        // Feed-other (not self): always keep craving even on yum.
        assert!(dont_change_craving(false, true));
        assert!(dont_change_craving(false, false));
        // Self yum → may change; self meh → keep.
        assert!(!dont_change_craving(true, true));
        assert!(dont_change_craving(true, false));
    }

    #[test]
    fn do_increase_feed_other_keeps_craving_even_on_yum() {
        let mut y = YumState::default();
        y.has_eaten.insert(33, 1.0); // yum food being fed
        y.has_eaten.insert(40, -2.0);
        y.currently_craving = 40;
        let foods = [10, 20, 33, 40];
        let is_yum = true;
        let dont = dont_change_craving(/*is_self_eat=*/ false, is_yum);
        assert!(dont);
        let wire = y
            .do_increase_food_value(33, 1.0, dont, &[], &foods, None, |_| 0, || 0.0)
            .expect("wire");
        assert_eq!(wire.food_id, 40);
        assert_eq!(y.currently_craving, 40);
        assert_eq!(wire.bonus, 3); // -2 display → -3 → bonus 3
    }

    #[test]
    fn do_increase_empty_map_returns_none() {
        let mut y = YumState::default();
        assert!(y
            .do_increase_food_value(1, 1.0, false, &[], &[1], None, |_| 0, || 0.0)
            .is_none());
    }

    /// FEED-OTHER-YUM: feeder gets 20% of eater yum prestige; meh/superMeh → 0.
    // Haxe: GlobalPlayerInstance.doEating L3151–3156
    #[test]
    fn feed_other_feeder_prestige_delta_yum_share() {
        assert!((feed_other_feeder_prestige_delta(1.0, true) - 0.2).abs() < 1e-5);
        assert!((feed_other_feeder_prestige_delta(5.0, true) - 1.0).abs() < 1e-5);
        assert_eq!(feed_other_feeder_prestige_delta(1.0, false), 0.0);
        assert_eq!(feed_other_feeder_prestige_delta(-2.0, true), 0.0);
        assert_eq!(feed_other_feeder_prestige_delta(0.0, true), 0.0);
    }
}
