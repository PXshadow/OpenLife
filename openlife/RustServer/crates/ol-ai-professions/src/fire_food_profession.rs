//! Haxe: `AiBase.makeFireFood` pure body (chunk **AI-MAKE-STUFF** / `make_fire_bake`).
//!
//! Closes residual after **AI-SHEPHERD-MID**: `makeStuff` step 5 (`makeFireFood(2)`)
//! and standalone fire-food maker profession sticky.
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession('FIREFOODMAKER')` with max-people + sticky last
//! - Speech `FIREFOOD!` / `FIREFOODMAKER!` â†’ assigned job
//! - Hot-coals cook ladder (mutton, goose, rabbit, pork bowl, beans, kindling/stew)
//! - Fire craft when no fireplace; unskew cooked rabbit/goose on ground
//! - Omelette / second-fire / raw mutton-pork-goose-bean stock gates
//!
//! No world I/O: callers supply counts / fire flags and apply returned
//! [`FireFoodAction`]s via craft/shortCraft.
//!
//! Residual: late hungry/isHandlingFire makeFireFood(1/2/3) outside assigned/makeStuff.
//!
//! **AI-FIREFOOD-RUNG**: assigned/last FIREFOODMAKER â†’ `makeFireFood(100)` via
//! `ProfessionScanKind::FireFood` + `try_decide_fire_food_from_rung`.

use std::collections::HashMap;

use ol_ai_helper::ai_goals::Goal;
use crate::baker_profession::{
    CLAY_PLATE, COOKED_MUTTON, KINDLING, RAW_MUTTON, RAW_STEW_POT, SOAKING_BEANS,
};

// â”€â”€ Object ids (OHOL / OpenLife content; Haxe comments in AiBase.makeFireFood) â”€

/// Hot Coals 85.
// Haxe: AiBase.makeFireFood ~4339
pub const HOT_COALS: i32 = 85;
/// Fire 82.
// Haxe: AiBase.makeFireFood ~4363 / GetCloseFire fallback
pub const FIRE: i32 = 82;
/// Large Fast Fire 83 (GetCloseFire first).
// Haxe: AiHelper.GetCloseFire ~2129
pub const LARGE_FAST_FIRE: i32 = 83;
/// Large Slow Fire 346.
// Haxe: AiHelper.GetCloseFire ~2130
pub const LARGE_SLOW_FIRE: i32 = 346;
/// Cooked Rabbit (skewered) 186 â€” unskew via shortCraftOnGround.
// Haxe: AiBase.makeFireFood ~4324
pub const COOKED_RABBIT_SKEWERED: i32 = 186;
/// Cooked Rabbit 197.
pub const COOKED_RABBIT: i32 = 197;
/// Skinned Rabbit 181.
pub const SKINNED_RABBIT: i32 = 181;
/// Skewered Rabbit 185.
pub const SKEWERED_RABBIT: i32 = 185;
/// Dead Canada Goose 514.
pub const DEAD_GOOSE: i32 = 514;
/// Plucked Goose 515.
pub const PLUCKED_GOOSE: i32 = 515;
/// Skewered Goose 516.
pub const SKEWERED_GOOSE: i32 = 516;
/// Cooked Goose - skewered 517.
// Haxe: AiBase.makeFireFood ~4325
pub const COOKED_GOOSE_SKEWERED: i32 = 517;
/// Cooked Goose 518.
pub const COOKED_GOOSE: i32 = 518;
/// Bowl of Raw Pork 1354.
pub const BOWL_RAW_PORK: i32 = 1354;
/// Bowl of Carnitas 1355.
pub const BOWL_CARNITAS: i32 = 1355;
/// Raw Pork 1342.
pub const RAW_PORK: i32 = 1342;
/// Flint Chip 135.
pub const FLINT_CHIP: i32 = 135;
/// Dead Grizzly Bear 643.
pub const DEAD_GRIZZLY: i32 = 643;
/// Skinned Bear 657.
pub const SKINNED_BEAR: i32 = 657;
/// Cool Flat Rock 1284.
pub const COOL_FLAT_ROCK: i32 = 1284;
/// Firewood.
// Haxe: AiBase.makeFireWood L4429
pub const FIREWOOD: i32 = 344;
/// Stack of Firewood (Haxe searches this if closeWood is null, then discards the result).
// Haxe: AiBase.makeFireWood L4430
pub const STACK_OF_FIREWOOD: i32 = 1316;
/// Kindling Pile (same discarded-search pattern as stack of firewood).
// Haxe: AiBase.makeFireWood L4435
pub const KINDLING_PILE: i32 = 1599;
/// Haxe `GetClosestObjectById` default searchDistance for makeFireWood.
// Haxe: AiHelper.GetClosestObjectById searchDistance = 40
pub const MAKE_FIRE_WOOD_SEARCH_DIST: i32 = 40;
/// Haxe `shortCraft(0, 1284, 20)` searchDistance.
// Haxe: AiBase.makeFireFood L4373
pub const COOL_FLAT_ROCK_SHORTCRAFT_DIST: i32 = 20;
/// Cold Goose Egg 1262.
pub const COLD_GOOSE_EGG: i32 = 1262;
/// Omelette 1285.
// Haxe: AiBase.makeFireFood ~4380 (note: Haxe countOmelette uses plate id 236 â€” ported as-is)
pub const OMELETTE: i32 = 1285;
/// Bowl of Cooked Beans 1292.
pub const COOKED_BEANS: i32 = 1292;
/// Popping Corn 1122.
pub const POPPING_CORN: i32 = 1122;
/// Popcorn 1121.
pub const POPCORN: i32 = 1121;
/// Haxe CountClose popcorn 1121 at home r=40.
// Haxe: AiBase.makePopcornIfNeeded L4293
pub const POPCORN_HOME_COUNT_RADIUS: i32 = 40;
/// Haxe CountClose popcorn 1121 at player r=40 (added to home count; may double).
// Haxe: AiBase.makePopcornIfNeeded L4294
pub const POPCORN_PLAYER_COUNT_RADIUS: i32 = 40;
/// Haxe CountClose popping corn 1122 at player r=30 (not home).
// Haxe: AiBase.makePopcornIfNeeded L4298
pub const POPPING_CORN_PLAYER_COUNT_RADIUS: i32 = 30;

/// Home search radius for hot coals / fire (Haxe 30).
// Haxe: AiBase.makeFireFood GetClosestObjectToHome r=30
pub const FIRE_FOOD_HOME_RADIUS: i32 = 30;
/// Default max people for makeFireFood / age paths.
pub const FIRE_FOOD_DEFAULT_MAX_PEOPLE: i32 = 1;
/// Haxe `makeStuff` uses `makeFireFood(2)`.
// Haxe: AiBase.makeStuff ~4083
pub const FIRE_FOOD_MAKE_STUFF_MAX_PEOPLE: i32 = 2;
/// Assigned FIREFOODMAKER uses large max (parity with other assigned jobs).
pub const FIRE_FOOD_ASSIGNED_MAX_PEOPLE: i32 = 100;

/// Canonical Haxe profession string.
pub const FIRE_FOOD_PROFESSION_KEY: &str = "FIREFOODMAKER";

// â”€â”€ Profession speech / runtime â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Parse speech / assigned profession tokens for fire-food maker.
///
/// Accepts `FIREFOOD`, `FIREFOOD!`, `FIREFOODMAKER`, `FIREFOODMAKER!`.
// Haxe: assignedProfession / lastProfession == 'FIREFOODMAKER'
pub fn parse_fire_food_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("FIREFOODMAKER") || prof.eq_ignore_ascii_case("FIREFOOD")
}

/// Sticky last + assigned + weight for FIREFOODMAKER.
// Haxe: AiBase.profession['FIREFOODMAKER'] + lastProfession
#[derive(Debug, Clone, PartialEq)]
pub struct FireFoodProfessionRuntime {
    pub is_last_fire_food: bool,
    pub is_assigned_fire_food: bool,
    /// Haxe `this.profession['FIREFOODMAKER']` weight (0 idle / 1 active).
    pub weight: f32,
}

impl Default for FireFoodProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_fire_food: false,
            is_assigned_fire_food: false,
            weight: 0.0,
        }
    }
}

impl FireFoodProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear profession weight (Haxe fallthrough `profession['FIREFOODMAKER']=0`).
    // Haxe: AiBase.makeFireFood ~4423
    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    /// Apply eat-path profession wipe.
    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        if !last_was_foodserver {
            self.is_last_fire_food = false;
        }
    }
}

/// Assign from speech `FIREFOOD!` / `FIREFOODMAKER!`.
pub fn assign_fire_food_from_speech(
    runtime: &mut FireFoodProfessionRuntime,
    text: &str,
) -> bool {
    if !parse_fire_food_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_fire_food = true;
    runtime.is_last_fire_food = true;
    runtime.weight = 1.0;
    true
}

/// Count peers already sticky on FIREFOODMAKER.
// Haxe: AiBase.countProfession('FIREFOODMAKER')
pub fn count_fire_food_peers(peer_count_with_last: f32) -> f32 {
    peer_count_with_last.max(0.0)
}

/// One AI peer for pure countProfession filtering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireFoodPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub has_player_to_follow: bool,
    pub same_home: bool,
    pub last_is_fire_food: bool,
}

impl FireFoodPeerSnapshot {
    pub fn eligible_for_count(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
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
        if !self.same_home {
            return false;
        }
        true
    }

    pub fn counts_as_fire_food(self, min_age_to_eat: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age_to_eat, max_age) && self.last_is_fire_food
    }
}

pub fn count_fire_food_peers_filtered(
    peers: &[FireFoodPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_fire_food(min_age_to_eat, max_age))
        .count() as f32
}

/// Haxe `hasOrBecomeProfession('FIREFOODMAKER', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_fire_food(
    runtime: &mut FireFoodProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_fire_food {
        runtime.is_last_fire_food = true;
        return true;
    }
    let count = count_fire_food_peers(peer_count_with_last);
    let cap = max as f32 + was_idle.max(0.0);
    if count >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_fire_food = true;
    true
}

pub fn has_or_become_fire_food_filtered(
    runtime: &mut FireFoodProfessionRuntime,
    max: i32,
    peers: &[FireFoodPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
    was_idle: f32,
) -> bool {
    let peer_count = count_fire_food_peers_filtered(peers, min_age_to_eat, max_age);
    has_or_become_fire_food(runtime, max, peer_count, was_idle)
}

pub fn resolve_fire_food_assigned_job(runtime: &FireFoodProfessionRuntime) -> bool {
    runtime.is_assigned_fire_food || runtime.is_last_fire_food
}

// â”€â”€ Counts / actions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Close-object counts for makeFireFood (home radius + held).
// Haxe: countCurrentObject / CountCloseObjects family
#[derive(Debug, Clone, Default)]
pub struct FireFoodCounts {
    pub by_id: HashMap<i32, i32>,
    pub held_id: i32,
    pub is_hungry: bool,
    pub has_corn_seeds: bool,
    pub has_bean_seeds: bool,
    /// Hot Coals 85 present near home (Haxe GetClosestObjectToHome 85,30).
    pub has_hot_coals: bool,
    /// GetCloseFire non-null (83 / 346 / 82).
    pub has_fire_place: bool,
    /// Haxe `hotCoals == firePlace` (same ObjectHelper). Normally false when ids differ;
    /// kindling-on-coals only when true; stew pot when coals exist and this is false.
    // Haxe: AiBase.makeFireFood ~4356â€“4359
    pub hot_coals_is_fire_place: bool,
    /// Second fire 82 near home excluding firePlace (Haxe exclude firePlace).
    pub has_second_fire: bool,
    /// BowlFiller peer is self (makePopcornIfNeeded best AI gate). Default true for pure unit.
    // Haxe: getBestAiForObjByProfession('BowlFiller') ~4307
    pub is_best_bowl_filler: bool,
    /// Precomputed Haxe popcorn stock (home 1121 r=40 + player 1121 r=40 + player 1122 r=30 + held).
    /// `None` → fall back to `get_with_held(1121)+get_with_held(1122)` for unit tests.
    // Haxe: AiBase.makePopcornIfNeeded L4292–4299
    pub popcorn_stock: Option<i32>,
}

/// One AI for Haxe `getBestAiForObjByProfession('BowlFiller', home)`.
// Haxe: AiBase.getBestAiForObjByProfession ~1311
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BowlFillerPeer {
    pub p_id: i32,
    /// Squared Euclidean distance to the home/object used as the search origin.
    pub quad_dist_to_obj: f32,
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub same_home: bool,
    /// Haxe `profession['BowlFiller'] > 0` (baker last/assigned in live Rust).
    pub has_bowl_filler: bool,
}

impl BowlFillerPeer {
    fn eligible(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        if self.age > max_age - 2.0 {
            return false;
        }
        if self.is_wounded {
            return false;
        }
        if self.food_store < 2.0 {
            return false;
        }
        if !self.same_home {
            return false;
        }
        true
    }
}

/// Haxe `getBestAiForObjByProfession('BowlFiller')` — true when `self` wins.
///
/// Others without BowlFiller profession are skipped. Self without profession is
/// included with +100 quad. Closest remaining candidate is assigned.
// Haxe: AiBase.getBestAiForObjByProfession ~1311; makePopcornIfNeeded ~4307
pub fn is_self_best_bowl_filler(
    self_p_id: i32,
    peers: &[BowlFillerPeer],
    min_age_to_eat: f32,
    max_age: f32,
) -> bool {
    let mut best_id: Option<i32> = None;
    let mut best_dist = f32::MAX;
    for p in peers {
        if !p.eligible(min_age_to_eat, max_age) {
            continue;
        }
        if !p.has_bowl_filler && p.p_id != self_p_id {
            continue;
        }
        let mut dist = p.quad_dist_to_obj;
        if !p.has_bowl_filler {
            dist += 100.0;
        }
        if best_id.is_some() && dist >= best_dist {
            continue;
        }
        best_dist = dist;
        best_id = Some(p.p_id);
    }
    best_id == Some(self_p_id)
}

impl FireFoodCounts {
    pub fn get(&self, id: i32) -> i32 {
        *self.by_id.get(&id).unwrap_or(&0)
    }

    pub fn set(&mut self, id: i32, n: i32) {
        if n <= 0 {
            self.by_id.remove(&id);
        } else {
            self.by_id.insert(id, n);
        }
    }

    pub fn sum(&self, ids: &[i32]) -> i32 {
        ids.iter().map(|&id| self.get(id)).sum()
    }

    pub fn get_with_held(&self, id: i32) -> i32 {
        self.get(id) + if self.held_id == id { 1 } else { 0 }
    }
}

/// Pure decision output for makeFireFood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireFoodAction {
    None,
    /// Cannot become profession / refuse.
    Abort,
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `shortCraftOnGround(id)`.
    ShortCraftOnGround { target: i32 },
    CraftItem { object_id: i32 },
}

impl FireFoodAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None | Self::Abort)
    }
}

/// Needed raw stock to fire second coals path (Haxe isHungry ? 1 : 4).
// Haxe: AiBase.makeFireFood ~4392
pub fn needed_raw_fire_food(is_hungry: bool) -> i32 {
    if is_hungry {
        1
    } else {
        4
    }
}

/// Haxe `countCurrentObject` — map + held.
// Haxe: AiBase.countCurrentObjectHelper L3448
pub fn count_current_object(counts: &FireFoodCounts, id: i32) -> i32 {
    counts.get_with_held(id)
}

/// Haxe `countCurrentObjects(ids)` — each id includes held.
// Haxe: AiBase.countCurrentObjects L3432–3436
pub fn count_current_objects(counts: &FireFoodCounts, ids: &[i32]) -> i32 {
    ids.iter().map(|&id| counts.get_with_held(id)).sum()
}

/// Raw rabbit family 181+185.
pub fn count_raw_rabbit(counts: &FireFoodCounts) -> i32 {
    count_current_objects(counts, &[SKINNED_RABBIT, SKEWERED_RABBIT])
}

/// Raw goose family 514+515+516.
pub fn count_raw_goose(counts: &FireFoodCounts) -> i32 {
    count_current_objects(counts, &[DEAD_GOOSE, PLUCKED_GOOSE, SKEWERED_GOOSE])
}

/// Done goose family 517+518.
pub fn count_done_goose(counts: &FireFoodCounts) -> i32 {
    count_current_objects(counts, &[COOKED_GOOSE_SKEWERED, COOKED_GOOSE])
}

/// Haxe countOmelette uses `countCurrentObject(236)` (plates) — intentional bug port.
// Haxe: AiBase.makeFireFood L4378 `countOmelette = countCurrentObject(236)`
pub fn count_omelette_haxe_bug(counts: &FireFoodCounts) -> i32 {
    count_current_object(counts, CLAY_PLATE)
}

/// True if pure makePopcornIfNeeded would craft popping corn.
///
/// Stock uses [`FireFoodCounts::popcorn_stock`] when set (home 1121 r=40 + player
/// 1121 r=40 + player 1122 r=30 + held). Else `get_with_held` 1121+1122.
/// BowlFiller (`getBestAiForObjByProfession`) + `craftItem(1122)`.
// Haxe: AiBase.makePopcornIfNeeded L4281–4312
pub fn make_popcorn_if_needed(counts: &FireFoodCounts) -> FireFoodAction {
    if !counts.has_corn_seeds {
        return FireFoodAction::None;
    }
    let count = counts.popcorn_stock.unwrap_or_else(|| {
        counts.get_with_held(POPCORN) + counts.get_with_held(POPPING_CORN)
    });
    if count > 0 {
        return FireFoodAction::None;
    }
    if !counts.is_best_bowl_filler {
        return FireFoodAction::None;
    }
    FireFoodAction::CraftItem {
        object_id: POPPING_CORN,
    }
}

/// Full pure `makeFireFood(maxPeople)` body.
// Haxe: AiBase.makeFireFood ~4315â€“4424
pub fn make_fire_food(
    counts: &FireFoodCounts,
    runtime: &mut FireFoodProfessionRuntime,
    max_people: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> FireFoodAction {
    if !has_or_become_fire_food(runtime, max_people, peer_count_with_last, was_idle) {
        return FireFoodAction::Abort;
    }

    // Unskew cooked rabbit / goose on ground
    // Haxe: shortCraftOnGround(186) / (517) ~4324â€“4325
    if counts.get_with_held(COOKED_RABBIT_SKEWERED) > 0 || counts.held_id == COOKED_RABBIT_SKEWERED
    {
        // Prefer on-ground when map has skewered cooked rabbit (held handled as ground too)
        if counts.get(COOKED_RABBIT_SKEWERED) > 0 || counts.held_id == COOKED_RABBIT_SKEWERED {
            return FireFoodAction::ShortCraftOnGround {
                target: COOKED_RABBIT_SKEWERED,
            };
        }
    }
    if counts.get(COOKED_GOOSE_SKEWERED) > 0 || counts.held_id == COOKED_GOOSE_SKEWERED {
        return FireFoodAction::ShortCraftOnGround {
            target: COOKED_GOOSE_SKEWERED,
        };
    }

    // Haxe countCurrentObject(s) include held
    // Haxe: AiBase.makeFireFood L4328–4336
    let count_done_mutton = count_current_object(counts, COOKED_MUTTON);
    let count_done_rabbit = count_current_object(counts, COOKED_RABBIT);
    let count_raw_rabbit = count_raw_rabbit(counts);
    let count_raw_goose = count_raw_goose(counts);
    let count_done_r_goose = count_done_goose(counts);

    // Hot coals cook ladder
    // Haxe: ~4341â€“4359
    if counts.has_hot_coals {
        if count_done_mutton < 3 {
            if counts.get(RAW_MUTTON) > 0 || counts.held_id == RAW_MUTTON {
                return FireFoodAction::ShortCraft {
                    actor: RAW_MUTTON,
                    target: HOT_COALS,
                };
            }
        }
        if count_raw_goose > 0 && count_done_r_goose < 5 {
            if counts.get(SKEWERED_GOOSE) > 0 || counts.held_id == SKEWERED_GOOSE {
                return FireFoodAction::ShortCraft {
                    actor: SKEWERED_GOOSE,
                    target: HOT_COALS,
                };
            }
        }
        if count_raw_rabbit > 0 && count_done_rabbit < 5 {
            if counts.get(SKEWERED_RABBIT) > 0 || counts.held_id == SKEWERED_RABBIT {
                return FireFoodAction::ShortCraft {
                    actor: SKEWERED_RABBIT,
                    target: HOT_COALS,
                };
            }
        }
        if counts.get(BOWL_RAW_PORK) > 0 || counts.held_id == BOWL_RAW_PORK {
            return FireFoodAction::ShortCraft {
                actor: BOWL_RAW_PORK,
                target: HOT_COALS,
            };
        }
        if counts.get(SOAKING_BEANS) > 0 || counts.held_id == SOAKING_BEANS {
            return FireFoodAction::ShortCraft {
                actor: SOAKING_BEANS,
                target: HOT_COALS,
            };
        }
        // Kindling only when coals are the firePlace object
        if counts.hot_coals_is_fire_place
            && (counts.get(KINDLING) > 0 || counts.held_id == KINDLING)
        {
            return FireFoodAction::ShortCraft {
                actor: KINDLING,
                target: HOT_COALS,
            };
        }
        // Stew pot when coals are NOT the firePlace
        if !counts.hot_coals_is_fire_place
            && (counts.get(RAW_STEW_POT) > 0 || counts.held_id == RAW_STEW_POT)
        {
            return FireFoodAction::ShortCraft {
                actor: RAW_STEW_POT,
                target: HOT_COALS,
            };
        }
    }

    // No fire place â†’ craft Fire 82
    // Haxe: ~4362â€“4363
    if !counts.has_fire_place {
        return FireFoodAction::CraftItem { object_id: FIRE };
    }

    // Flint Chip + Dead Grizzly
    // Haxe: shortCraft(135, 643)
    if counts.get(DEAD_GRIZZLY) > 0 {
        return FireFoodAction::ShortCraft {
            actor: FLINT_CHIP,
            target: DEAD_GRIZZLY,
        };
    }
    // 0 + Skinned Bear
    if counts.get(SKINNED_BEAR) > 0 {
        return FireFoodAction::ShortCraft {
            actor: 0,
            target: SKINNED_BEAR,
        };
    }

    // makePopcornIfNeeded
    let popcorn = make_popcorn_if_needed(counts);
    if popcorn.is_some() {
        return popcorn;
    }

    // Haxe: shortCraft(0, 1284, 20)
    // Haxe: AiBase.makeFireFood L4373
    if counts.get(COOL_FLAT_ROCK) > 0 || counts.held_id == COOL_FLAT_ROCK {
        return FireFoodAction::ShortCraft {
            actor: 0,
            target: COOL_FLAT_ROCK,
        };
    }

    let count_eggs = count_current_object(counts, COLD_GOOSE_EGG);
    let count_plates = count_current_object(counts, CLAY_PLATE);
    // Haxe bug: countOmelette = countCurrentObject(236) (plates)
    let count_omelette = count_omelette_haxe_bug(counts);

    if count_plates > 0 && count_eggs > 0 && count_omelette < 4 {
        return FireFoodAction::CraftItem {
            object_id: OMELETTE,
        };
    }

    let mut count_raw_fire_food = count_raw_rabbit + count_eggs;
    count_raw_fire_food += count_current_object(counts, RAW_MUTTON);
    count_raw_fire_food += count_current_object(counts, RAW_PORK);
    count_raw_fire_food += count_current_object(counts, RAW_STEW_POT);

    let needed_raw = needed_raw_fire_food(counts.is_hungry);
    let need_coals = (count_omelette < 1 && count_plates > 0)
        || count_done_rabbit < 1
        || count_done_mutton < 1;

    if count_raw_fire_food >= needed_raw && !counts.has_hot_coals && need_coals {
        // Second fire 82 excluding firePlace; else craft Fire
        // Haxe: ~4395â€“4398
        if !counts.has_second_fire {
            return FireFoodAction::CraftItem { object_id: FIRE };
        }
        // Second fire exists â€” continue to stock crafts (coals will appear from fire)
    }

    // Raw Stew Pot craftItemMax(1246, 2) when corn seeds — countCurrentObject includes held
    // Haxe: AiBase.makeFireFood L4404
    if counts.has_corn_seeds && count_current_object(counts, RAW_STEW_POT) < 2 {
        return FireFoodAction::CraftItem {
            object_id: RAW_STEW_POT,
        };
    }

    // Raw Mutton if mutton family < 2
    // Haxe: ~4407â€“4408
    // Haxe: AiBase.makeFireFood L4407 countCurrentObjects([570, 569])
    let count_mutton = count_current_objects(counts, &[COOKED_MUTTON, RAW_MUTTON]);
    if count_mutton < 2 {
        return FireFoodAction::CraftItem {
            object_id: RAW_MUTTON,
        };
    }

    // Raw Pork if pork food < 2
    // Haxe: ~4411â€“4412
    // Haxe: AiBase.makeFireFood L4411 countCurrentObjects([1342, 1355])
    let count_pork_food = count_current_objects(counts, &[RAW_PORK, BOWL_CARNITAS]);
    if count_pork_food < 2 {
        return FireFoodAction::CraftItem {
            object_id: RAW_PORK,
        };
    }

    // Plucked Goose if goose stock low
    // Haxe: ~4415
    if count_done_r_goose + count_raw_goose < 3 {
        return FireFoodAction::CraftItem {
            object_id: PLUCKED_GOOSE,
        };
    }

    // Soaking beans when bean seeds and bean food < 2
    // Haxe: ~4418â€“4419
    // Haxe: AiBase.makeFireFood L4418–4419 countCurrentObjects + craftItemMax(1180, 2)
    let count_bean_food = count_current_objects(counts, &[SOAKING_BEANS, COOKED_BEANS]);
    if count_bean_food < 2
        && counts.has_bean_seeds
        && count_current_object(counts, SOAKING_BEANS) < 2
    {
        return FireFoodAction::CraftItem {
            object_id: SOAKING_BEANS,
        };
    }

    // Fallthrough clear weight
    // Haxe: profession['FIREFOODMAKER'] = 0 ~4423
    runtime.clear_weight();
    FireFoodAction::None
}

/// Closest firewood/kindling for [`make_fire_wood`] (`num_uses`, `uses`).
// Haxe: AiBase.makeFireWood L4429–4438 GetClosestObjectById
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireWoodSnap {
    /// Closest Firewood 344 (`objectData.numUses`, `numberOfUses`).
    pub close_wood: Option<(i32, i32)>,
    /// Closest Kindling 72.
    pub close_kindling: Option<(i32, i32)>,
}

/// Haxe `makeFireWood`: craft 344 if no close firewood or incomplete pile; else kindling 72.
///
/// Stack 1316 / pile 1599 searches are unused in Haxe (result discarded). Call site is
/// commented (`age < 15 && makeFireWood()`).
// Haxe: AiBase.makeFireWood L4427–4440
pub fn make_fire_wood(snap: FireWoodSnap) -> FireFoodAction {
    let do_craft_wood = match snap.close_wood {
        None => true,
        Some((num_uses, uses)) => num_uses > 1 && uses < num_uses,
    };
    if do_craft_wood {
        return FireFoodAction::CraftItem {
            object_id: FIREWOOD,
        };
    }
    let do_craft_kindling = match snap.close_kindling {
        None => true,
        Some((num_uses, uses)) => num_uses > 1 && uses < num_uses,
    };
    if do_craft_kindling {
        return FireFoodAction::CraftItem {
            object_id: KINDLING,
        };
    }
    FireFoodAction::None
}

/// Map action → self-play goal.
pub fn fire_food_action_to_goal(action: FireFoodAction) -> Goal {
    match action {
        FireFoodAction::None | FireFoodAction::Abort => Goal::SeekObject(FIRE),
        FireFoodAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        FireFoodAction::ShortCraftOnGround { target } => Goal::SeekObject(target),
        FireFoodAction::CraftItem { object_id } => Goal::SeekObject(object_id),
    }
}

/// Fill [`FireFoodCounts`] from (id, count) pairs + fire flags (unit tests / thin tick).
pub fn fire_food_counts_from_nearby(
    pairs: &[(i32, i32)],
    held_id: i32,
    is_hungry: bool,
    has_corn_seeds: bool,
    has_bean_seeds: bool,
    has_hot_coals: bool,
    has_fire_place: bool,
    hot_coals_is_fire_place: bool,
    has_second_fire: bool,
) -> FireFoodCounts {
    let mut c = FireFoodCounts {
        held_id,
        is_hungry,
        has_corn_seeds,
        has_bean_seeds,
        has_hot_coals,
        has_fire_place,
        hot_coals_is_fire_place,
        has_second_fire,
        is_best_bowl_filler: true,
        ..Default::default()
    };
    for &(id, n) in pairs {
        c.set(id, n);
    }
    c
}

/// Map-object snapshot for home-radius fire food counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireFoodMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Haxe popcorn stock: CountClose home 1121 r=40 + player 1121 r=40 + held 1121
/// + CountClose player 1122 r=30 + held 1122. Same 1121 tile in both squares counts twice.
// Haxe: AiBase.makePopcornIfNeeded L4292–4299
pub fn popcorn_stock_from_map(
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    map: &[FireFoodMapObj],
) -> i32 {
    use crate::farmer_profession::in_count_close_square;
    let mut n = 0;
    for o in map {
        if o.parent_id == POPCORN {
            if in_count_close_square(home_x, home_y, o.x, o.y, POPCORN_HOME_COUNT_RADIUS) {
                n += 1;
            }
            if in_count_close_square(
                player_x,
                player_y,
                o.x,
                o.y,
                POPCORN_PLAYER_COUNT_RADIUS,
            ) {
                n += 1;
            }
        } else if o.parent_id == POPPING_CORN
            && in_count_close_square(
                player_x,
                player_y,
                o.x,
                o.y,
                POPPING_CORN_PLAYER_COUNT_RADIUS,
            )
        {
            n += 1;
        }
    }
    if held_id == POPCORN {
        n += 1;
    }
    if held_id == POPPING_CORN {
        n += 1;
    }
    n
}

impl FireFoodCounts {
    /// Fill [`FireFoodCounts::popcorn_stock`] from a map snapshot.
    // Haxe: AiBase.makePopcornIfNeeded L4292–4299
    pub fn apply_popcorn_stock_from_map(
        &mut self,
        home_x: i32,
        home_y: i32,
        player_x: i32,
        player_y: i32,
        map: &[FireFoodMapObj],
    ) {
        self.popcorn_stock = Some(popcorn_stock_from_map(
            home_x, home_y, player_x, player_y, self.held_id, map,
        ));
    }
}

/// Exclusive-square home radius fill (same convention as farm/bake).
// Haxe: CountCloseObjects home radius
pub fn fill_fire_food_counts_from_map(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    map: &[FireFoodMapObj],
    radius: i32,
    is_hungry: bool,
    has_corn_seeds: bool,
    has_bean_seeds: bool,
) -> FireFoodCounts {
    use crate::farmer_profession::in_count_close_square;

    let mut c = FireFoodCounts {
        held_id,
        is_hungry,
        has_corn_seeds,
        has_bean_seeds,
        is_best_bowl_filler: true,
        ..Default::default()
    };

    let mut has_coals = false;
    let mut has_fire_place = false;
    let mut has_fire_82 = false;
    let mut fire_82_count = 0i32;

    for o in map {
        if o.parent_id == 0 {
            continue;
        }
        if !in_count_close_square(home_x, home_y, o.x, o.y, radius) {
            continue;
        }
        c.set(o.parent_id, c.get(o.parent_id) + 1);
        if o.parent_id == HOT_COALS {
            has_coals = true;
        }
        if o.parent_id == LARGE_FAST_FIRE
            || o.parent_id == LARGE_SLOW_FIRE
            || o.parent_id == FIRE
        {
            has_fire_place = true;
        }
        if o.parent_id == FIRE {
            has_fire_82 = true;
            fire_82_count += 1;
        }
    }

    c.has_hot_coals = has_coals;
    c.has_fire_place = has_fire_place;
    // Coals and fire place are different object ids â†’ never same ObjectHelper
    c.hot_coals_is_fire_place = false;
    // Second fire: another Fire 82 when firePlace is some fire object
    c.has_second_fire = fire_82_count >= 2 || (has_fire_place && has_fire_82 && fire_82_count >= 1 && {
        // If firePlace is large fire (83/346), a single 82 counts as second
        map.iter().any(|o| {
            in_count_close_square(home_x, home_y, o.x, o.y, radius)
                && (o.parent_id == LARGE_FAST_FIRE || o.parent_id == LARGE_SLOW_FIRE)
        }) && has_fire_82
    });
    // Simpler: if fire place exists and there's a Fire 82 distinct from sole firePlace
    // When only one FIRE and fire place is FIRE, second is false.
    // When fire place is large fire + any FIRE 82 â†’ second true.
    // When two FIRE 82 â†’ second true.
    if fire_82_count >= 2 {
        c.has_second_fire = true;
    } else if fire_82_count == 1 {
        let has_large = map.iter().any(|o| {
            in_count_close_square(home_x, home_y, o.x, o.y, radius)
                && (o.parent_id == LARGE_FAST_FIRE || o.parent_id == LARGE_SLOW_FIRE)
        });
        c.has_second_fire = has_large;
    } else {
        c.has_second_fire = false;
    }
    let _ = has_fire_82; // used above
    c
}

/// Max people for makeStuff vs assigned vs default.
// Haxe: makeFireFood(2) in makeStuff; assigned large max
pub fn fire_food_max_people_for_dispatch(is_assigned_job: bool, make_stuff_path: bool) -> i32 {
    if is_assigned_job {
        FIRE_FOOD_ASSIGNED_MAX_PEOPLE
    } else if make_stuff_path {
        FIRE_FOOD_MAKE_STUFF_MAX_PEOPLE
    } else {
        FIRE_FOOD_DEFAULT_MAX_PEOPLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt() -> FireFoodProfessionRuntime {
        FireFoodProfessionRuntime::default()
    }

    fn counts(pairs: &[(i32, i32)]) -> FireFoodCounts {
        fire_food_counts_from_nearby(
            pairs, 0, false, false, false, false, true, false, false,
        )
    }

    #[test]
    fn speech_and_assign() {
        assert!(parse_fire_food_profession_speech("FIREFOOD!"));
        assert!(parse_fire_food_profession_speech("firefoodmaker"));
        assert!(!parse_fire_food_profession_speech("BAKER!"));
        let mut r = rt();
        assert!(assign_fire_food_from_speech(&mut r, "FIREFOOD!"));
        assert!(r.is_assigned_fire_food && r.is_last_fire_food);
        assert_eq!(r.weight, 1.0);
    }

    #[test]
    fn has_or_become_cap() {
        let mut r = rt();
        assert!(!has_or_become_fire_food(&mut r, 1, 1.0, 0.0));
        assert!(!r.is_last_fire_food);
        assert!(has_or_become_fire_food(&mut r, 2, 1.0, 0.0));
        assert!(r.is_last_fire_food);
        // sticky keeps
        assert!(has_or_become_fire_food(&mut r, 1, 5.0, 0.0));
    }

    #[test]
    fn abort_when_cannot_become() {
        let mut r = rt();
        let c = counts(&[]);
        assert_eq!(
            make_fire_food(&c, &mut r, 1, 1.0, 0.0),
            FireFoodAction::Abort
        );
    }

    #[test]
    fn unskew_cooked_rabbit_on_ground() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let c = counts(&[(COOKED_RABBIT_SKEWERED, 1)]);
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::ShortCraftOnGround {
                target: COOKED_RABBIT_SKEWERED
            }
        );
    }

    #[test]
    fn hot_coals_cook_mutton() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[(RAW_MUTTON, 1), (COOKED_MUTTON, 0)]);
        c.has_hot_coals = true;
        c.has_fire_place = true;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::ShortCraft {
                actor: RAW_MUTTON,
                target: HOT_COALS
            }
        );
        // Haxe countCurrentObject(570) includes held — 3 held+map skips cook
        c.set(COOKED_MUTTON, 2);
        c.held_id = COOKED_MUTTON;
        let skip = make_fire_food(&c, &mut r, 2, 0.0, 0.0);
        assert!(!matches!(
            skip,
            FireFoodAction::ShortCraft {
                actor: RAW_MUTTON,
                ..
            }
        ));
    }

    #[test]
    fn hot_coals_skewered_rabbit_cap() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[
            (SKEWERED_RABBIT, 1),
            (SKINNED_RABBIT, 0),
            (COOKED_RABBIT, 2),
        ]);
        c.has_hot_coals = true;
        c.has_fire_place = true;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::ShortCraft {
                actor: SKEWERED_RABBIT,
                target: HOT_COALS
            }
        );
        // At done rabbit >= 5, skip rabbit cook
        c.set(COOKED_RABBIT, 5);
        // no other coal work â†’ fall through past coals
        let a = make_fire_food(&c, &mut r, 2, 0.0, 0.0);
        assert!(!matches!(
            a,
            FireFoodAction::ShortCraft {
                actor: SKEWERED_RABBIT,
                ..
            }
        ));
    }

    #[test]
    fn kindling_only_when_coals_are_fireplace() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[(KINDLING, 1)]);
        c.has_hot_coals = true;
        c.has_fire_place = true;
        c.hot_coals_is_fire_place = false;
        // stew path needs stew; without stew and without kindling match â†’ fallthrough
        let a = make_fire_food(&c, &mut r, 2, 0.0, 0.0);
        assert!(!matches!(
            a,
            FireFoodAction::ShortCraft {
                actor: KINDLING,
                ..
            }
        ));
        c.hot_coals_is_fire_place = true;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::ShortCraft {
                actor: KINDLING,
                target: HOT_COALS
            }
        );
    }

    #[test]
    fn stew_on_coals_when_not_fireplace() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[(RAW_STEW_POT, 1)]);
        c.has_hot_coals = true;
        c.has_fire_place = true;
        c.hot_coals_is_fire_place = false;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::ShortCraft {
                actor: RAW_STEW_POT,
                target: HOT_COALS
            }
        );
    }

    #[test]
    fn craft_fire_when_no_fireplace() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[]);
        c.has_fire_place = false;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem { object_id: FIRE }
        );
    }

    #[test]
    fn omelette_when_plates_and_eggs() {
        let mut r = rt();
        r.is_last_fire_food = true;
        // countOmelette = plates (Haxe bug) so omelette < 4 means plates < 4
        let mut c = counts(&[(CLAY_PLATE, 2), (COLD_GOOSE_EGG, 1)]);
        c.has_fire_place = true;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem {
                object_id: OMELETTE
            }
        );
    }

    #[test]
    fn second_fire_when_raw_stock_and_need_coals() {
        let mut r = rt();
        r.is_last_fire_food = true;
        // raw rabbit + eggs high, no coals, need coals (no done rabbit/mutton)
        let mut c = counts(&[
            (SKINNED_RABBIT, 2),
            (SKEWERED_RABBIT, 2),
            (COLD_GOOSE_EGG, 0),
            (CLAY_PLATE, 0),
        ]);
        c.has_fire_place = true;
        c.has_hot_coals = false;
        c.has_second_fire = false;
        c.is_hungry = false; // need 4 raw
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem { object_id: FIRE }
        );
    }

    #[test]
    fn stock_raw_mutton_when_low() {
        let mut r = rt();
        r.is_last_fire_food = true;
        // fire place + coals path skipped (no coals); enough done food so need_coals false for rabbit/mutton?
        // need_coals = (omelette<1 && plates>0) || done_rabbit<1 || done_mutton<1
        // set done rabbit and mutton high to skip second fire
        let mut c = counts(&[
            (COOKED_RABBIT, 5),
            (COOKED_MUTTON, 5),
            (RAW_MUTTON, 0),
            (CLAY_PLATE, 0),
        ]);
        c.has_fire_place = true;
        c.has_hot_coals = false;
        // mutton family = 5 cooked only â†’ countMutton >= 2, skip mutton craft
        // pork < 2 â†’ craft raw pork
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem {
                object_id: RAW_PORK
            }
        );
        // mutton family low
        c.set(COOKED_MUTTON, 0);
        c.set(COOKED_RABBIT, 5);
        // need_coals true (done mutton < 1) + raw stock?
        // raw_fire_food = raw_rabbit(0)+eggs(0)+raw_mutton(0)+pork(0)+stew(0)=0 < 4
        // so no second fire; stock mutton craft
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem {
                object_id: RAW_MUTTON
            }
        );
    }

    #[test]
    fn fallthrough_clears_weight() {
        let mut r = rt();
        r.is_last_fire_food = true;
        r.weight = 1.0;
        // Stock all high so fallthrough
        let mut c = counts(&[
            (COOKED_RABBIT, 5),
            (COOKED_MUTTON, 5),
            (RAW_MUTTON, 2),
            (RAW_PORK, 2),
            (BOWL_CARNITAS, 0),
            (PLUCKED_GOOSE, 2),
            (COOKED_GOOSE, 2),
            (SOAKING_BEANS, 2),
        ]);
        c.has_fire_place = true;
        c.has_hot_coals = false;
        c.has_bean_seeds = false;
        c.has_corn_seeds = false;
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::None
        );
        assert_eq!(r.weight, 0.0);
    }

    #[test]
    fn popcorn_needs_corn_seeds_and_empty_stock() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[]);
        c.has_fire_place = true;
        c.has_corn_seeds = true;
        c.is_best_bowl_filler = true;
        // Before stock crafts: popcorn after bear/flat rock
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem {
                object_id: POPPING_CORN
            }
        );
        c.set(POPCORN, 1);
        let a = make_fire_food(&c, &mut r, 2, 0.0, 0.0);
        assert!(!matches!(
            a,
            FireFoodAction::CraftItem {
                object_id: POPPING_CORN
            }
        ));
        c.set(POPCORN, 0);
        c.is_best_bowl_filler = false;
        let skipped = make_fire_food(&c, &mut r, 2, 0.0, 0.0);
        assert!(!matches!(
            skipped,
            FireFoodAction::CraftItem {
                object_id: POPPING_CORN
            }
        ));
        assert_eq!(POPCORN_HOME_COUNT_RADIUS, 40);
        assert_eq!(POPCORN_PLAYER_COUNT_RADIUS, 40);
        assert_eq!(POPPING_CORN_PLAYER_COUNT_RADIUS, 30);
        // Home+player same origin double-counts 1121 in both squares
        let map = [FireFoodMapObj {
            parent_id: POPCORN,
            x: 1,
            y: 0,
        }];
        assert_eq!(popcorn_stock_from_map(0, 0, 0, 0, 0, &map), 2);
        // Player-only popping 1122 r=30; not counted at home-only r=40
        let popping = [FireFoodMapObj {
            parent_id: POPPING_CORN,
            x: 25,
            y: 0,
        }];
        assert_eq!(popcorn_stock_from_map(0, 0, 0, 0, 0, &popping), 1);
        assert_eq!(
            popcorn_stock_from_map(100, 100, 0, 0, 0, &popping),
            1,
            "1122 is player r=30 only"
        );
        let far_popping = [FireFoodMapObj {
            parent_id: POPPING_CORN,
            x: 35,
            y: 0,
        }];
        assert_eq!(
            popcorn_stock_from_map(0, 0, 0, 0, 0, &far_popping),
            0,
            "1122 player r=30 half-open excludes x=35"
        );
        let mut c2 = counts(&[]);
        c2.has_corn_seeds = true;
        c2.is_best_bowl_filler = true;
        c2.apply_popcorn_stock_from_map(0, 0, 0, 0, &map);
        assert_eq!(make_popcorn_if_needed(&c2), FireFoodAction::None);
        c2.popcorn_stock = Some(0);
        assert_eq!(
            make_popcorn_if_needed(&c2),
            FireFoodAction::CraftItem {
                object_id: POPPING_CORN
            }
        );
        c2.has_corn_seeds = false;
        assert_eq!(make_popcorn_if_needed(&c2), FireFoodAction::None);
    }

    fn bowl_peer(p_id: i32, dist: f32, has: bool, same_home: bool) -> BowlFillerPeer {
        BowlFillerPeer {
            p_id,
            quad_dist_to_obj: dist,
            deleted: false,
            age: 20.0,
            is_wounded: false,
            food_store: 10.0,
            same_home,
            has_bowl_filler: has,
        }
    }

    #[test]
    fn self_is_best_bowl_filler_when_no_peer_has_profession() {
        // Haxe: others without BowlFiller skipped; self +100 still only candidate
        let self_p = bowl_peer(1, 4.0, false, true);
        let other = bowl_peer(2, 1.0, false, true);
        assert!(is_self_best_bowl_filler(1, &[self_p, other], 3.0, 60.0));
    }

    #[test]
    fn closer_bowl_filler_peer_wins_popcorn_gate() {
        let self_p = bowl_peer(1, 16.0, false, true);
        let other = bowl_peer(2, 1.0, true, true);
        assert!(!is_self_best_bowl_filler(1, &[self_p, other], 3.0, 60.0));
        assert!(is_self_best_bowl_filler(2, &[self_p, other], 3.0, 60.0));
    }

    #[test]
    fn fill_from_map_detects_coals_and_fire() {
        let map = [
            FireFoodMapObj {
                parent_id: HOT_COALS,
                x: 0,
                y: 0,
            },
            FireFoodMapObj {
                parent_id: FIRE,
                x: 1,
                y: 0,
            },
            FireFoodMapObj {
                parent_id: RAW_MUTTON,
                x: 2,
                y: 0,
            },
        ];
        let c = fill_fire_food_counts_from_map(0, 0, 0, &map, 30, false, false, false);
        assert!(c.has_hot_coals);
        assert!(c.has_fire_place);
        assert_eq!(c.get(RAW_MUTTON), 1);
        assert!(!c.hot_coals_is_fire_place);
    }

    #[test]
    fn make_stuff_max_people_constant() {
        assert_eq!(FIRE_FOOD_MAKE_STUFF_MAX_PEOPLE, 2);
        assert_eq!(fire_food_max_people_for_dispatch(false, true), 2);
        assert_eq!(fire_food_max_people_for_dispatch(true, false), 100);
    }

    #[test]
    fn omelette_haxe_bug_uses_plates() {
        let c = counts(&[(CLAY_PLATE, 3), (OMELETTE, 99)]);
        assert_eq!(count_omelette_haxe_bug(&c), 3);
        let mut held = counts(&[(CLAY_PLATE, 3)]);
        held.held_id = CLAY_PLATE;
        assert_eq!(count_omelette_haxe_bug(&held), 4);
        assert_eq!(
            count_current_objects(&held, &[SKINNED_RABBIT, SKEWERED_RABBIT]),
            0
        );
        let mut raw = counts(&[(SKINNED_RABBIT, 1)]);
        raw.held_id = SKEWERED_RABBIT;
        assert_eq!(count_raw_rabbit(&raw), 2);
        assert_eq!(COOL_FLAT_ROCK_SHORTCRAFT_DIST, 20);
    }

    #[test]
    fn fire_food_stock_crafts_count_current_includes_held() {
        let mut r = rt();
        r.is_last_fire_food = true;
        let mut c = counts(&[
            (COOKED_RABBIT, 5),
            (COOKED_MUTTON, 1),
            (RAW_PORK, 2),
            (PLUCKED_GOOSE, 2),
            (COOKED_GOOSE, 2),
            (SOAKING_BEANS, 2),
        ]);
        c.has_fire_place = true;
        c.has_hot_coals = false;
        c.has_corn_seeds = true;
        c.popcorn_stock = Some(1);
        c.is_best_bowl_filler = false;
        c.held_id = RAW_STEW_POT;
        c.set(RAW_STEW_POT, 1);
        // craftItemMax(1246,2): map 1 + held 1 → skip stew; mutton family 1+held stew not mutton
        assert_eq!(
            make_fire_food(&c, &mut r, 2, 0.0, 0.0),
            FireFoodAction::CraftItem {
                object_id: RAW_MUTTON
            }
        );
        c.held_id = COOKED_MUTTON;
        c.set(RAW_STEW_POT, 2);
        // mutton family map 1 + held cooked → 2, skip mutton; pork 2 skip; goose 4 skip
        assert_eq!(make_fire_food(&c, &mut r, 2, 0.0, 0.0), FireFoodAction::None);
        assert_eq!(MAKE_FIRE_WOOD_SEARCH_DIST, 40);
        assert_eq!(STACK_OF_FIREWOOD, 1316);
        assert_eq!(KINDLING_PILE, 1599);
    }

    #[test]
    fn make_fire_wood_crafts_when_missing_or_incomplete_pile() {
        // Haxe L4427–4440: null → craft 344; complete single-use → try kindling
        assert_eq!(
            make_fire_wood(FireWoodSnap {
                close_wood: None,
                close_kindling: None,
            }),
            FireFoodAction::CraftItem {
                object_id: FIREWOOD
            }
        );
        assert_eq!(
            make_fire_wood(FireWoodSnap {
                close_wood: Some((1, 1)),
                close_kindling: None,
            }),
            FireFoodAction::CraftItem {
                object_id: KINDLING
            }
        );
        assert_eq!(
            make_fire_wood(FireWoodSnap {
                close_wood: Some((3, 1)),
                close_kindling: Some((1, 1)),
            }),
            FireFoodAction::CraftItem {
                object_id: FIREWOOD
            }
        );
        assert_eq!(
            make_fire_wood(FireWoodSnap {
                close_wood: Some((1, 1)),
                close_kindling: Some((1, 1)),
            }),
            FireFoodAction::None
        );
    }
}
