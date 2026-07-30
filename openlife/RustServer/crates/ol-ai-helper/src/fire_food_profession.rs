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
//! Residual: full `makePopcornIfNeeded` BowlFiller peer pick (pure stock craft only);
//! late hungry/isHandlingFire makeFireFood(1/2/3) outside assigned/makeStuff.
//!
//! **AI-FIREFOOD-RUNG**: assigned/last FIREFOODMAKER â†’ `makeFireFood(100)` via
//! `ProfessionScanKind::FireFood` + `try_decide_fire_food_from_rung`.

use std::collections::HashMap;

use crate::ai_goals::Goal;
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

/// Raw rabbit family 181+185.
pub fn count_raw_rabbit(counts: &FireFoodCounts) -> i32 {
    counts.sum(&[SKINNED_RABBIT, SKEWERED_RABBIT])
}

/// Raw goose family 514+515+516.
pub fn count_raw_goose(counts: &FireFoodCounts) -> i32 {
    counts.sum(&[DEAD_GOOSE, PLUCKED_GOOSE, SKEWERED_GOOSE])
}

/// Done goose family 517+518.
pub fn count_done_goose(counts: &FireFoodCounts) -> i32 {
    counts.sum(&[COOKED_GOOSE_SKEWERED, COOKED_GOOSE])
}

/// Haxe countOmelette uses `countCurrentObject(236)` (plates) â€” intentional bug port.
// Haxe: AiBase.makeFireFood ~4378 `countOmelette = countCurrentObject(236)`
pub fn count_omelette_haxe_bug(counts: &FireFoodCounts) -> i32 {
    counts.get(CLAY_PLATE)
}

/// True if pure makePopcornIfNeeded would craft popping corn.
// Haxe: AiBase.makePopcornIfNeeded ~4281
pub fn make_popcorn_if_needed(counts: &FireFoodCounts) -> FireFoodAction {
    if !counts.has_corn_seeds {
        return FireFoodAction::None;
    }
    let mut count = counts.get_with_held(POPCORN);
    count += counts.get_with_held(POPPING_CORN);
    // Haxe also counts near player tile â€” pure uses home+held only
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

    let count_done_mutton = counts.get(COOKED_MUTTON);
    let count_done_rabbit = counts.get(COOKED_RABBIT);
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

    // 0 + Cool Flat Rock â†’ ashes
    if counts.get(COOL_FLAT_ROCK) > 0 {
        return FireFoodAction::ShortCraft {
            actor: 0,
            target: COOL_FLAT_ROCK,
        };
    }

    let count_eggs = counts.get(COLD_GOOSE_EGG);
    let count_plates = counts.get(CLAY_PLATE);
    // Haxe bug: countOmelette = countCurrentObject(236) (plates)
    let count_omelette = count_omelette_haxe_bug(counts);

    if count_plates > 0 && count_eggs > 0 && count_omelette < 4 {
        return FireFoodAction::CraftItem {
            object_id: OMELETTE,
        };
    }

    let mut count_raw_fire_food = count_raw_rabbit + count_eggs;
    count_raw_fire_food += counts.get(RAW_MUTTON);
    count_raw_fire_food += counts.get(RAW_PORK);
    count_raw_fire_food += counts.get(RAW_STEW_POT);

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

    // Raw Stew Pot craftItemMax when corn seeds
    // Haxe: ~4404
    if counts.has_corn_seeds && counts.get(RAW_STEW_POT) < 2 {
        return FireFoodAction::CraftItem {
            object_id: RAW_STEW_POT,
        };
    }

    // Raw Mutton if mutton family < 2
    // Haxe: ~4407â€“4408
    let count_mutton = counts.sum(&[COOKED_MUTTON, RAW_MUTTON]);
    if count_mutton < 2 {
        return FireFoodAction::CraftItem {
            object_id: RAW_MUTTON,
        };
    }

    // Raw Pork if pork food < 2
    // Haxe: ~4411â€“4412
    let count_pork_food = counts.sum(&[RAW_PORK, BOWL_CARNITAS]);
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
    let count_bean_food = counts.sum(&[SOAKING_BEANS, COOKED_BEANS]);
    if count_bean_food < 2 && counts.has_bean_seeds {
        return FireFoodAction::CraftItem {
            object_id: SOAKING_BEANS,
        };
    }

    // Fallthrough clear weight
    // Haxe: profession['FIREFOODMAKER'] = 0 ~4423
    runtime.clear_weight();
    FireFoodAction::None
}

/// Map action â†’ self-play goal.
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
    }
}
