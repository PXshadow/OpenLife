//! Smart AI `dropHeldObject` body (DROP-HELD-AI / drop_held_smart).
//!
//! Ports Haxe `AiBase.dropHeldObject` + `storeInQuiver` + id tables used for
//! oven / forge / fire / well / soil / pile staging. Pure decision + thin map
//! to [`ShortCraftLiveIntent`]; live USE/DROP apply stays in
//! [`super::apply_short_craft_live_intent`] (or crate short_craft_intent).
//!
//! Loaded as `short_craft_intent::drop_held_ai` via `#[path]` (and optionally
//! crate-root re-export).
//!
//! Haxe anchors:
//! - `AiBase.dropHeldObject` ~5267–5649
//! - `AiBase.storeInQuiver` ~5087–5138
//! - `AiBase.UseUpDough` ~5237–5261 (delegates bread stock via baker helpers)
//! - `AiBase.considerDropHeldObject` ~5191–5234 (id tables)
//! - `AiBase.dropNear*ItemIds` ~5164–5189
//! - empty-tile filters already in `profession_scan` (`NEEDS_NOT_FLOORED_PLACE`,
//!   `DONT_DROP_CLOSE_HOME_IDS`)

use crate::animal_damage::{ARROW_QUIVER_ID, BOW_AND_ARROW_ID, EMPTY_ARROW_QUIVER_ID};
use crate::baker_profession::{
    max_dough_in_bowl, should_drop_near_oven, BOWL_OF_DOUGH, CLAY_PLATE, COOKED_MUTTON,
    COOKED_PIES, DROP_NEAR_OVEN_IDS, RAW_MUTTON,
};
use crate::farmer_profession::{in_count_close_square, BOWL_OF_SOIL};
use crate::smith_profession::{is_forge_id, DROP_NEAR_FORGE_IDS};

use super::profession_scan::{
    closest_by_parent_id, closest_empty_tile_ex, closest_well, scan_chebyshev, ClosestEmptyOpts,
    ScanTile, WELL_IDS,
};
use super::ShortCraftLiveIntent;

// ── Object ids (Haxe literals) ──────────────────────────────────────────────

/// Yew Bow.
// Haxe: storeInQuiver held 151
pub const YEW_BOW: i32 = 151;
/// Arrow.
// Haxe: storeInQuiver held 148
pub const ARROW: i32 = 148;
/// Empty Arrow Quiver with Bow.
// Haxe: 4149
pub const EMPTY_ARROW_QUIVER_WITH_BOW: i32 = 4149;
/// Arrow Quiver with Bow.
// Haxe: 4151
pub const ARROW_QUIVER_WITH_BOW: i32 = 4151;

/// Banana Peel.
pub const BANANA_PEEL: i32 = 2144;
/// Sharp Stone.
pub const SHARP_STONE: i32 = 34;
/// Flint Chip.
pub const FLINT_CHIP: i32 = 135;
/// Milkweed Stalk.
pub const MILKWEED_STALK: i32 = 57;
/// Flat Rock with Rabbit Bait.
pub const FLAT_ROCK_RABBIT_BAIT: i32 = 3180;

/// Hot Adobe Oven.
pub const HOT_ADOBE_OVEN: i32 = 250;
/// Hot Coals.
pub const HOT_COALS: i32 = 85;
/// Dying Gooseberry Bush.
pub const DYING_BUSH: i32 = 389;
/// Hardened Row.
pub const HARDENED_ROW: i32 = 848;
/// Stone Hoe.
pub const STONE_HOE: i32 = 850;
/// Steel Hoe.
pub const STEEL_HOE: i32 = 857;
/// Basket.
pub const BASKET: i32 = 292;
/// Omelette — Haxe `AiHelper.dropOnTable`.
// Haxe: dropOnTable = [1285]
pub const OMELETTE: i32 = 1285;
/// Table — preferred container for omelette.
// Haxe: Table 3371
pub const TABLE: i32 = 3371;
/// Wooden Slot Box — most preferred small-food store.
// Haxe: Wooden Slot Box 3065
pub const WOODEN_SLOT_BOX: i32 = 3065;
/// Haxe `AiHelper.dropOnTable` parent ids.
pub const DROP_ON_TABLE_IDS: &[i32] = &[OMELETTE];
/// Haxe `AiHelper.pies` (baked pies for store + same-food prefer).
// Haxe: pies = [272, 803, 273, 274, 275, 276, 277, 278] == baker COOKED_PIES
pub const BAKED_PIE_IDS: &[i32] = COOKED_PIES;
/// Haxe `AiHelper.smallCookedFood`.
// Haxe: smallCookedFood = [570]
pub const SMALL_COOKED_FOOD_IDS: &[i32] = &[COOKED_MUTTON];
/// Shallow Tilled Row.
pub const SHALLOW_TILLED_ROW: i32 = 1136;
/// Fertile Soil.
pub const FERTILE_SOIL: i32 = 1138;
/// Dry Bean Pod.
pub const DRY_BEAN_POD: i32 = 1160;
/// Bowl of Dry Beans.
pub const BOWL_OF_DRY_BEANS: i32 = 1176;
/// Gooseberry.
pub const GOOSEBERRY: i32 = 31;
/// Bowl of Gooseberries.
pub const BOWL_OF_GOOSEBERRIES: i32 = 253;
/// Clay Bowl.
pub const CLAY_BOWL: i32 = 235;
/// Basket of Bones.
pub const BASKET_OF_BONES: i32 = 356;
/// Clay.
pub const CLAY: i32 = 126;
/// Hot Iron Bloom in Wooden Tongs.
pub const HOT_IRON_BLOOM_TONGS: i32 = 308;
/// Flat Rock.
pub const FLAT_ROCK: i32 = 291;
/// Stone.
pub const STONE: i32 = 33;
/// Shovel of Dung.
pub const SHOVEL_OF_DUNG: i32 = 900;
/// Wet Compost Pile.
pub const WET_COMPOST: i32 = 625;
/// Bowl of Wheat.
pub const BOWL_OF_WHEAT: i32 = 245;
/// Ripe Wheat.
pub const RIPE_WHEAT: i32 = 242;
/// Dry Planted Wheat.
pub const DRY_PLANTED_WHEAT: i32 = 228;
/// Deep Tilled Row.
pub const DEEP_TILLED_ROW: i32 = 213;
/// Clay Plate.
pub const CLAY_PLATE_ID: i32 = 236;
/// Shovel.
pub const SHOVEL: i32 = 502;
/// Basket of Soil.
pub const BASKET_OF_SOIL: i32 = 336;
/// Straw.
pub const STRAW: i32 = 227;
/// Bowl of Water.
pub const BOWL_OF_WATER: i32 = 382;
/// Leavened Dough on Clay Plate.
pub const LEAVENED_DOUGH_PLATE: i32 = 1468;
/// Bowl of Leavened Dough.
pub const BOWL_OF_LEAVENED_DOUGH: i32 = 1466;
/// Knife.
pub const KNIFE: i32 = 560;
/// Sliced Bread (Haxe 1471).
pub const SLICED_BREAD_ID: i32 = 1471;

/// Kindling / firewood / rabbit / potato / goose / mouflon-rope — drop near fire.
// Haxe: dropNearFireItemIds
pub const DROP_NEAR_FIRE_IDS: &[i32] = &[72, 344, 180, 181, 185, 1147, 1148, 516, 540];

/// Basket of Soil / Straw / Bowl of Water — drop near well.
// Haxe: dropNearWellItemIds
pub const DROP_NEAR_WELL_IDS: &[i32] = &[336, 227, 382];

/// Stone / Sharp Stone / Banana Peel / Bowl of Water / Full Water Pouch.
/// Haxe declares this table; no call sites (dead in `considerDropHeldObject`).
// Haxe: AiBase.dropAtCurrentPosition L5181
pub const DROP_AT_CURRENT_POSITION_IDS: &[i32] = &[33, 34, 2144, 382, 210];

/// Never pile these unless `allow_all_piles`.
// Haxe: dontUsePile (allowAllPiles ? [] : …)
pub const DONT_USE_PILE_IDS: &[i32] = &[225, 1113, 292, 233, 132, 64, 66];

/// Held ids that force USE-as-drop on empty (never plain DROP).
// Haxe: dontUseDropForItems
pub const DONT_USE_DROP_FOR_ITEMS: &[i32] = &[356, 336, 1137, 186, 283, 241, 324, 323];

/// Default max search while expanding empty/pile rings.
// Haxe: maxSearchDistance = 40
pub const DROP_HELD_MAX_SEARCH: i32 = 40;

/// Haxe `UseUpDough` knife `GetClosestObjectToPosition(home, 560, 30)`.
// Haxe: AiBase.UseUpDough L5246
pub const USE_UP_DOUGH_KNIFE_HOME_RADIUS: i32 = 30;
/// Haxe `CountCloseObjects` sliced/leavened bread family from home.
// Haxe: AiBase.UseUpDough L5250–5254
pub const USE_UP_DOUGH_BREAD_HOME_RADIUS: i32 = 20;
/// Haxe `shortCraft(252, 236, 10, false)`.
// Haxe: AiBase.UseUpDough L5258
pub const USE_UP_DOUGH_PLATE_RADIUS: i32 = 10;

/// Haxe `AiBase.closeUseQuadDistance` — refuse far piles while a food target is set.
/// Distinct from doTimeStuffHelper close-use gate `distance < 25` (L505).
// Haxe: AiBase L33; dropHeldObject L5585–5587
pub const CLOSE_USE_QUAD_DISTANCE: i32 = 400;

/// Quad distance threshold “close enough” to home/target for dropOnStart walk.
// Haxe: quadIsCloseEnoughDistanceToTarget = 400
pub const DROP_CLOSE_ENOUGH_QUAD: i32 = 400;

/// Clothing slot for quiver SELF (Haxe `myPlayer.self(0, 0, 5)`).
// Haxe: self(0,0,5) backpack/quiver slot
pub const QUIVER_CLOTHING_SLOT: i32 = 5;

// ── Decision enum ───────────────────────────────────────────────────────────

/// Pure outcome of smart dropHeldObject (before live USE/DROP/SELF).
// Haxe: dropHeldObject return true + dropTarget / useTarget / self / shortCraft
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropHeldDecision {
    /// Empty hands / wound / nothing to do.
    None,
    /// Held is wound or hidden wound — refuse drop.
    RefuseWound,
    /// Store bow/arrow into quiver clothing (SELF slot 5).
    SelfClothing { slot: i32 },
    /// Prefer a shortCraft USE before dropping (caller resolves target tile).
    PreferShortCraft {
        actor: i32,
        target: i32,
        /// Max search radius hint (Haxe shortCraft dist arg).
        max_search: i32,
        craft_actor: bool,
        /// maxNewActor for shortCraftOnTarget (e.g. mutton oven = 4).
        max_new_actor: i32,
    },
    /// USE held on a known world target (bowl fill, basket, container drop).
    UseAt {
        x: i32,
        y: i32,
        target_id: i32,
        actor_id: i32,
    },
    /// Plain DROP at empty tile (`dropIsAUse = false`).
    DropAt { x: i32, y: i32 },
    /// USE as drop on tile (pile / empty with ground transition / dontUseDrop).
    UseAsDrop {
        x: i32,
        y: i32,
        target_id: i32,
        actor_id: i32,
    },
    /// Walk toward drop anchor (home/oven/forge/…) before placing.
    Goto { x: i32, y: i32 },
    /// Haxe: isMoving while dropOnStart — hold the tick.
    BusyMoving,
}

impl DropHeldDecision {
    pub fn is_action(self) -> bool {
        !matches!(self, Self::None | Self::RefuseWound)
    }

    /// Map wire-capable decisions to [`ShortCraftLiveIntent`].
    ///
    /// Prefer [`resolve_prefer_short_craft`] / [`plan_drop_held_live`] first so
    /// PreferShortCraft becomes UseAt when target is in scan. Unresolved
    /// PreferShortCraft keeps `craft_actor` as SeekOrCraft craft_if_needed.
    /// BusyMoving → Wait (hold tick; Haxe isMoving return true).
    // Haxe: dropTarget → DROP; useTarget → USE; gotoObj → walk; self → SELF clothing
    // Haxe: isMoving return true (PREFER-SHORT-WAIT BusyMoving)
    pub fn to_live_intent(self) -> ShortCraftLiveIntent {
        match self {
            Self::UseAt {
                x,
                y,
                target_id,
                actor_id,
            }
            | Self::UseAsDrop {
                x,
                y,
                target_id,
                actor_id,
            } => ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id,
                actor_id,
            },
            Self::DropAt { x, y } => ShortCraftLiveIntent::DropAt { x, y },
            // Haxe: myPlayer.gotoObj(target) while dropOnStart — walk, not DROP
            Self::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
            // Haxe: myPlayer.self(0, 0, 5) quiver store
            Self::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
            // Haxe: shortCraft(actor, target, …, craftActor) when target not tile-resolved
            Self::PreferShortCraft {
                actor, craft_actor, ..
            } => ShortCraftLiveIntent::SeekOrCraft {
                actor,
                craft_if_needed: craft_actor,
            },
            // Haxe: if (myPlayer.isMoving()) return true — hold tick, no fallthrough
            Self::BusyMoving => ShortCraftLiveIntent::Wait,
            Self::None | Self::RefuseWound => ShortCraftLiveIntent::None,
        }
    }
}

// Free-function alias (prefer_short_busy_to_live.inc) for external call sites.
include!("prefer_short_busy_to_live.inc.rs");

/// Raw `SELF` payload for clothing store (`SELF x y slot`).
// Haxe: myPlayer.self(0, 0, clothingSlot)
#[inline]
pub fn self_clothing_raw_payload(slot: i32) -> String {
    format!("0 0 {slot}")
}

// ── Id tables ───────────────────────────────────────────────────────────────

/// True when held should stage near fire (kindling, wood, rabbits…).
// Haxe: dropNearFireItemIds.contains
#[inline]
pub fn should_drop_near_fire(held_id: i32) -> bool {
    held_id > 0 && DROP_NEAR_FIRE_IDS.contains(&held_id)
}

/// True when held should stage near forge (tongs, ore, hammer…).
// Haxe: dropNearForgeItemIds.contains
#[inline]
pub fn should_drop_near_forge(held_id: i32) -> bool {
    held_id > 0 && DROP_NEAR_FORGE_IDS.contains(&held_id)
}

/// True when held should stage near well (soil basket, straw, water bowl).
// Haxe: dropNearWellItemIds.contains
#[inline]
pub fn should_drop_near_well(held_id: i32) -> bool {
    held_id > 0 && DROP_NEAR_WELL_IDS.contains(&held_id)
}

/// True when held is bakery/oven staging (includes pies via baker helper).
// Haxe: dropNearOvenItemIds || pies || rawPies
#[inline]
pub fn should_drop_near_oven_held(held_id: i32) -> bool {
    should_drop_near_oven(held_id) || DROP_NEAR_OVEN_IDS.contains(&held_id)
}

/// True when held is in Haxe `dropAtCurrentPosition` (table only; unused in Haxe).
// Haxe: AiBase.dropAtCurrentPosition L5181
#[inline]
pub fn should_drop_at_current_position(held_id: i32) -> bool {
    held_id > 0 && DROP_AT_CURRENT_POSITION_IDS.contains(&held_id)
}

/// Pile form blocked for this held unless `allow_all_piles`.
// Haxe: dontUsePile.contains → pileId = 0
#[inline]
pub fn pile_blocked(held_id: i32, allow_all_piles: bool) -> bool {
    !allow_all_piles && DONT_USE_PILE_IDS.contains(&held_id)
}

/// Haxe `pileId == itemToCraft.lastTargetId` → `pileId = -1`.
// Haxe: AiBase.dropHeldObject L5291
#[inline]
pub fn pile_id_after_last_craft_target(pile_id: i32, last_target_id: i32) -> i32 {
    if pile_id > 0 && pile_id == last_target_id {
        -1
    } else {
        pile_id
    }
}

/// Haxe `if (maxDistanceToHome < 1) this.dropTarget = null`.
// Haxe: AiBase.dropHeldObject L5282
#[inline]
pub fn drop_held_clears_drop_target(max_distance_to_home: f32) -> bool {
    max_distance_to_home < 1.0
}

/// Haxe `triedDropCount > 5 ? 0 : 10`.
// Haxe: AiBase.isDropingItem L8361
pub const IS_DROPPING_NEAR_DIST: i32 = 10;
/// After this many tries, drop at feet.
pub const IS_DROPPING_FEET_AFTER_TRIES: i32 = 5;
/// Held + ground object too far (quad) → `dropHeldObject` first.
// Haxe: AiBase.isDropingItem L8381
pub const IS_DROPPING_TOO_FAR_QUAD: i32 = 25;

/// Head of Haxe `isDropingItem` (through container / too-far dropHeld).
///
/// Rest is [`is_dropping_item_goto`] (L8401–8463).
// Haxe: AiBase.isDropingItem L8351–8391
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsDropingItemHead {
    Idle,
    TargetGone,
    DropHeld { max_distance: i32 },
    Continue,
}

/// Haxe `isDropingItem` through L8391: triedDropCount, expected, container, too-far.
// Haxe: AiBase.isDropingItem L8351–8391
pub fn is_dropping_item_head(
    is_moving: bool,
    drop_target: Option<(i32, i32, i32)>,
    world_parent_at_target: Option<i32>,
    held_id: i32,
    drop_target_num_slots: i32,
    player_x: i32,
    player_y: i32,
    tried_drop_count: &mut i32,
) -> IsDropingItemHead {
    if is_moving || drop_target.is_none() {
        *tried_drop_count = 0;
    }
    let Some((tid, tx, ty)) = drop_target else {
        return IsDropingItemHead::Idle;
    };
    let world_p = world_parent_at_target.unwrap_or(-1);
    if tid != world_p {
        return IsDropingItemHead::TargetGone;
    }
    let max_distance = if *tried_drop_count > IS_DROPPING_FEET_AFTER_TRIES {
        0
    } else {
        IS_DROPPING_NEAR_DIST
    };
    *tried_drop_count += 1;
    if held_id != 0 && drop_target_num_slots > 0 {
        return IsDropingItemHead::DropHeld { max_distance };
    }
    let dist = quad_distance_xy(player_x, player_y, tx, ty);
    if held_id != 0 && tid != 0 && dist > IS_DROPPING_TOO_FAR_QUAD as f32 {
        return IsDropingItemHead::DropHeld { max_distance };
    }
    IsDropingItemHead::Continue
}

/// Follow-player drop abort (quad).
// Haxe: AiBase.isDropingItem L8401
pub const IS_DROPPING_FOLLOW_TOO_FAR_QUAD: i32 = 400;
/// Stack of Clay Plates — drop converts to empty-hand USE.
// Haxe: AiBase.isDropingItem L8411
pub const STACK_OF_CLAY_PLATES: i32 = 1602;
/// Stack of Clay Bowls — drop converts to empty-hand USE.
// Haxe: AiBase.isDropingItem L8411
pub const STACK_OF_CLAY_BOWLS: i32 = 1603;
/// Extracted Arrowhead Wound — `use` instead of `drop`.
// Haxe: AiBase.isDropingItem L8451
pub const EXTRACTED_ARROWHEAD_WOUND: i32 = 1367;
/// Haxe `if (distance > 1) gotoObj` (squared Euclidean).
// Haxe: AiBase.isDropingItem L8430
pub const IS_DROPPING_GOTO_QUAD: f32 = 1.0;

/// Rest of Haxe `isDropingItem` after container / too-far (L8401–8463).
// Haxe: AiBase.isDropingItem L8401–8463
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsDropingItemGoto {
    /// L8401–8407: following and drop target too far → `dropHeldObject`.
    DropHeld { max_distance: i32 },
    /// L8411–8422: clay plate/bowl stack → USE empty actor, `return false`.
    ConvertToUse,
    /// L8426: `dropTarget == null`.
    Idle,
    /// L8428: `isMoving()` → stay busy.
    WaitMoving,
    /// L8430–8446: `distance > 1` → `gotoObj`.
    WalkToTarget,
    /// L8451–8454: parent 1367 → `use`.
    UseOnTarget,
    /// L8456–8463: `drop` then clear `dropTarget`.
    DropOnTarget,
}

#[inline]
pub fn drop_target_is_clay_stack(parent_id: i32) -> bool {
    parent_id == STACK_OF_CLAY_PLATES || parent_id == STACK_OF_CLAY_BOWLS
}

#[inline]
pub fn drop_target_uses_use_not_drop(parent_id: i32) -> bool {
    parent_id == EXTRACTED_ARROWHEAD_WOUND
}

/// Haxe `triedDropCount` after `is_dropping_item_head` increment → `dropDistance`.
#[inline]
pub fn is_dropping_drop_distance_after_try(tried_drop_count_after: i32) -> i32 {
    if tried_drop_count_after > IS_DROPPING_FEET_AFTER_TRIES + 1 {
        0
    } else {
        IS_DROPPING_NEAR_DIST
    }
}

/// Haxe `isDropingItem` L8401–8463 (follow abort, clay stacks, goto / use / drop).
///
/// `considerDropHeldObject` at L8424 is commented out. `dropTarget == null` (L8426)
/// is only reachable after that commented call, so live targets fall through.
// Haxe: AiBase.isDropingItem L8401–8463
pub fn is_dropping_item_goto(
    drop_target_id: i32,
    distance_quad: f32,
    has_player_to_follow: bool,
    is_hungry: bool,
    drop_max_distance: i32,
    is_moving: bool,
) -> IsDropingItemGoto {
    if drop_target_id != 0
        && distance_quad > IS_DROPPING_FOLLOW_TOO_FAR_QUAD as f32
        && has_player_to_follow
        && !is_hungry
    {
        return IsDropingItemGoto::DropHeld {
            max_distance: drop_max_distance,
        };
    }
    if drop_target_is_clay_stack(drop_target_id) {
        return IsDropingItemGoto::ConvertToUse;
    }
    if is_moving {
        return IsDropingItemGoto::WaitMoving;
    }
    if distance_quad > IS_DROPPING_GOTO_QUAD {
        return IsDropingItemGoto::WalkToTarget;
    }
    if drop_target_uses_use_not_drop(drop_target_id) {
        return IsDropingItemGoto::UseOnTarget;
    }
    IsDropingItemGoto::DropOnTarget
}

/// Held forces USE-as-drop on empty ground (bones basket, soil, tongs…).
// Haxe: dontUseDropForItems
#[inline]
pub fn must_use_as_drop(held_id: i32) -> bool {
    DONT_USE_DROP_FOR_ITEMS.contains(&held_id)
}

/// `dropOnStart` false for peels / chips that always drop at feet.
// Haxe: Banana Peel / Sharp Stone / Flint Chip / Milkweed / Rabbit Bait
#[inline]
pub fn force_drop_at_feet(held_id: i32) -> bool {
    matches!(
        held_id,
        BANANA_PEEL | SHARP_STONE | FLINT_CHIP | MILKWEED_STALK | FLAT_ROCK_RABBIT_BAIT
    )
}

/// Haxe `shouldDebugSay` when filling a basket with held clay.
// Haxe: AiBase.dropHeldObject L5392
#[inline]
pub fn drop_clay_in_basket_debug_say(debug_say: bool) -> Option<&'static str> {
    if debug_say {
        Some("drop clay in basket")
    } else {
        None
    }
}

/// Haxe `shouldDebugSay` after `gotoObj` while `dropOnStart`.
// Haxe: AiBase.dropHeldObject L5548–5553
pub const DROP_HELD_GOTO_HOME_SAY: &str = "Goto home!";
/// Path failed; Haxe continues search (does not return).
// Haxe: AiBase.dropHeldObject L5553
pub const DROP_HELD_CANNOT_GOTO_HOME_SAY: &str = "Cannot Goto home!";

/// `path_ok` is Haxe `gotoObj` return; live should only emit Goto when true.
#[inline]
pub fn drop_held_goto_home_debug_say(debug_say: bool, path_ok: bool) -> Option<&'static str> {
    if !debug_say {
        return None;
    }
    Some(if path_ok {
        DROP_HELD_GOTO_HOME_SAY
    } else {
        DROP_HELD_CANNOT_GOTO_HOME_SAY
    })
}

// ── Table / small-food container prefer (DROP-HELD-TABLE) ────────────────────

/// Haxe `AiHelper.ShouldDropOnTable` — omelette stages on tables.
// Haxe: AiHelper.ShouldDropOnTable ~30–32
#[inline]
pub fn should_drop_on_table(parent_id: i32) -> bool {
    parent_id > 0 && DROP_ON_TABLE_IDS.contains(&parent_id)
}

/// Haxe `AiHelper.IsBakedPie`.
// Haxe: AiHelper.IsBakedPie ~34–36
#[inline]
pub fn is_baked_pie(parent_id: i32) -> bool {
    parent_id > 0 && BAKED_PIE_IDS.contains(&parent_id)
}

/// Haxe `AiHelper.isSmallFoodToStore` — pies + cooked mutton into boxes/baskets.
// Haxe: AiHelper.isSmallFoodToStore ~38–40
#[inline]
pub fn is_small_food_to_store(parent_id: i32) -> bool {
    is_baked_pie(parent_id) || (parent_id > 0 && SMALL_COOKED_FOOD_IDS.contains(&parent_id))
}

/// True when empty-search may USE-drop into a free-slot container.
// Haxe: allowDropInContainer && (ShouldDropOnTable || isSmallFoodToStore)
#[inline]
pub fn allows_drop_in_container(held_id: i32) -> bool {
    should_drop_on_table(held_id) || is_small_food_to_store(held_id)
}

/// Distance adjust for container prefer (Haxe quadDistance factor math).
// Haxe: if factor<1 quad-=4; then *=factor (or /= if ≤0)
#[inline]
pub fn adjust_container_drop_score(quad_distance: f32, factor: f32) -> f32 {
    let mut d = quad_distance;
    if factor < 1.0 {
        d -= 4.0;
    }
    if d > 0.0 {
        d * factor
    } else if factor != 0.0 {
        d / factor
    } else {
        d
    }
}

/// Haxe container prefer factor for table/small-food held (None = not eligible).
// Haxe: GetClosestObjectToPositionHelper considerContainer factors ~232–272
pub fn container_prefer_factor(held_id: i32, container: &ScanTile) -> Option<f32> {
    if !allows_drop_in_container(held_id) || !container.has_free_slot() {
        return None;
    }
    let parent = container.parent_id;
    if should_drop_on_table(held_id) {
        // All free containers eligible; Table 3371 strongly preferred.
        let factor = if parent == TABLE { 0.25 } else { 1.0 };
        return Some(factor);
    }
    // isSmallFoodToStore
    let mut factor = if parent == WOODEN_SLOT_BOX {
        0.25
    } else if parent == BASKET {
        0.5
    } else {
        0.8 // other containers
    };
    // Prefer container already holding same small food class.
    if is_baked_pie(held_id) {
        if tile_contains_baked_pie(container) {
            factor *= 0.5;
        }
    } else if held_id == COOKED_MUTTON && container.contains_parent(COOKED_MUTTON) {
        factor *= 0.5;
    }
    Some(factor)
}

/// True when any nested slot is a baked pie parent id.
// Haxe: IsBakedPie(contained.parentId)
#[inline]
fn tile_contains_baked_pie(t: &ScanTile) -> bool {
    if is_baked_pie(t.contains_id) {
        return true;
    }
    t.contains_extra.iter().any(|&id| is_baked_pie(id))
}

/// Squared euclidean distance (Haxe quadDistance helper, no world wrap).
#[inline]
fn quad_distance_xy(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (bx - ax) as f32;
    let dy = (by - ay) as f32;
    dx * dx + dy * dy
}

// ── Quiver ──────────────────────────────────────────────────────────────────

/// Clothing snapshot for quiver store (parent ids + multi-use capacity).
// Haxe: getClothingById + ObjectHelper.canAddToQuiver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuiverClothing {
    /// Empty Arrow Quiver 874.
    pub empty_quiver: bool,
    /// Arrow Quiver 3948.
    pub arrow_quiver: bool,
    /// Empty Arrow Quiver with Bow 4149.
    pub empty_quiver_with_bow: bool,
    /// Arrow Quiver with Bow 4151.
    pub quiver_with_bow: bool,
    /// Haxe `quiver.canAddToQuiver()` — room for another arrow/bow-arrow.
    pub can_add: bool,
    /// Active quiver clothing `numberOfUses` (0 = unknown / single-use treat as free).
    pub quiver_uses: i32,
    /// Active quiver clothing `objectData.numUses` (0 = unknown → can_add if present).
    pub quiver_num_uses: i32,
}

impl QuiverClothing {
    /// Build from a flat list of clothing parent ids (any order/length).
    /// Capacity defaults open when quiver present (`numUses < 2` or unknown).
    pub fn from_ids(ids: &[i32]) -> Self {
        Self::from_ids_with_uses(ids, 0, 0)
    }

    /// Build with explicit quiver multi-use state for `canAddToQuiver`.
    // Haxe: canAddToQuiver → numUses < 2 || numberOfUses < numUses
    pub fn from_ids_with_uses(ids: &[i32], quiver_uses: i32, quiver_num_uses: i32) -> Self {
        let mut q = Self {
            quiver_uses: quiver_uses.max(0),
            quiver_num_uses: quiver_num_uses.max(0),
            ..Self::default()
        };
        for &id in ids {
            match id {
                EMPTY_ARROW_QUIVER_ID => q.empty_quiver = true,
                ARROW_QUIVER_ID => q.arrow_quiver = true,
                EMPTY_ARROW_QUIVER_WITH_BOW => q.empty_quiver_with_bow = true,
                ARROW_QUIVER_WITH_BOW => q.quiver_with_bow = true,
                _ => {}
            }
        }
        let has_quiver =
            q.empty_quiver || q.arrow_quiver || q.empty_quiver_with_bow || q.quiver_with_bow;
        q.can_add = has_quiver && can_add_to_quiver(q.quiver_uses, q.quiver_num_uses);
        q
    }

    /// Recompute `can_add` after manually tweaking uses/flags.
    pub fn refresh_can_add(&mut self) {
        let has_quiver = self.empty_quiver
            || self.arrow_quiver
            || self.empty_quiver_with_bow
            || self.quiver_with_bow;
        self.can_add = has_quiver && can_add_to_quiver(self.quiver_uses, self.quiver_num_uses);
    }

    /// Build from Haxe `clothingObjects` parent ids + multi-use (PlayerSnapshot clothing).
    // Haxe: getClothingById + canAddToQuiver uses (DROP-HELD-TABLE quiver snapshot)
    pub fn from_clothing_snapshot(ids: &[i32], uses: &[i32]) -> Self {
        let mut quiver_uses = 0i32;
        for (i, &id) in ids.iter().enumerate() {
            if matches!(
                id,
                EMPTY_ARROW_QUIVER_ID
                    | ARROW_QUIVER_ID
                    | EMPTY_ARROW_QUIVER_WITH_BOW
                    | ARROW_QUIVER_WITH_BOW
            ) {
                quiver_uses = uses.get(i).copied().unwrap_or(0).max(0);
                break;
            }
        }
        // numUses unknown from snapshot alone → 0 → can_add when quiver present (numUses < 2)
        Self::from_ids_with_uses(ids, quiver_uses, 0)
    }
}

/// Collect clothing parent ids from a 6-slot Haxe clothingObjects array (pad/trunc).
// Haxe: clothingObjects[0..5]
#[inline]
pub fn clothing_ids_snapshot(ids: &[i32]) -> [i32; 6] {
    let mut out = [0i32; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ids.get(i).copied().unwrap_or(0).max(0);
    }
    out
}

/// Pure quiver snapshot from clothing parent ids (uses default open capacity).
// Haxe: storeInQuiver clothing scan
#[inline]
pub fn quiver_from_clothing_ids(ids: &[i32]) -> QuiverClothing {
    QuiverClothing::from_ids(ids)
}

/// Pure quiver snapshot with multi-use from clothing uses array.
// Haxe: canAddToQuiver numberOfUses
#[inline]
pub fn quiver_from_clothing_snapshot(ids: &[i32], uses: &[i32]) -> QuiverClothing {
    QuiverClothing::from_clothing_snapshot(ids, uses)
}

/// Haxe `ObjectHelper.canAddToQuiver`.
// Haxe: ObjectHelper.canAddToQuiver ~767
#[inline]
pub fn can_add_to_quiver(number_of_uses: i32, num_uses: i32) -> bool {
    num_uses < 2 || number_of_uses < num_uses
}

/// Haxe `storeInQuiver` — put Yew Bow / Bow+Arrow / Arrow into quiver clothing.
// Haxe: AiBase.storeInQuiver ~5087–5138
pub fn store_in_quiver(held_id: i32, clothing: QuiverClothing) -> Option<DropHeldDecision> {
    match held_id {
        YEW_BOW => {
            if clothing.empty_quiver || clothing.arrow_quiver {
                return Some(DropHeldDecision::SelfClothing {
                    slot: QUIVER_CLOTHING_SLOT,
                });
            }
        }
        BOW_AND_ARROW_ID => {
            if (clothing.empty_quiver || clothing.arrow_quiver) && clothing.can_add {
                return Some(DropHeldDecision::SelfClothing {
                    slot: QUIVER_CLOTHING_SLOT,
                });
            }
        }
        ARROW => {
            if (clothing.empty_quiver
                || clothing.empty_quiver_with_bow
                || clothing.arrow_quiver
                || clothing.quiver_with_bow)
                && clothing.can_add
            {
                return Some(DropHeldDecision::SelfClothing {
                    slot: QUIVER_CLOTHING_SLOT,
                });
            }
        }
        _ => {}
    }
    None
}

/// Haxe `dropNearOven` / `dropNearKiln` / `dropNearForge` / `dropGraveyard` — empty bodies.
// Haxe: AiBase.dropNearOven L5140–5158 (locals only; dropHeldObject uses id tables)

// ── UseUpDough ──────────────────────────────────────────────────────────────

/// Inputs for dough use-up (Haxe `UseUpDough`).
// Haxe: UseUpDough ~5237
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseUpDoughInput {
    pub held_id: i32,
    pub held_uses: i32,
    /// Current AI useTarget is Clay Plate — allow, don't force use-up.
    pub use_target_is_plate: bool,
    pub has_knife_near: bool,
    /// Sliced Bread 1471 + Leavened on plate 1468 + Bowl Leavened 1466 near home.
    pub count_bread_family: i32,
    /// True when a Clay Plate is reachable for shortCraft.
    pub plate_available: bool,
}

/// Bowl of Dough 252 → Clay Plate when uses exceed keep-last budget.
// Haxe: UseUpDough shortCraft(252, 236, 10, false)
pub fn use_up_dough(inp: UseUpDoughInput) -> Option<DropHeldDecision> {
    if inp.held_id != BOWL_OF_DOUGH {
        return None;
    }
    if inp.use_target_is_plate {
        return None;
    }
    let max_d = max_dough_in_bowl(inp.has_knife_near, inp.count_bread_family);
    if inp.held_uses > max_d && inp.plate_available {
        return Some(DropHeldDecision::PreferShortCraft {
            actor: BOWL_OF_DOUGH,
            target: CLAY_PLATE,
            max_search: USE_UP_DOUGH_PLATE_RADIUS,
            craft_actor: false,
            max_new_actor: i32::MAX,
        });
    }
    None
}

// ── DropHeld input / core ───────────────────────────────────────────────────

/// World anchors for smart drop placement.
// Haxe: home / GetForge / GetKiln / getCloseWell / firePlace / GetGraveyard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropHeldAnchors {
    pub home_x: i32,
    pub home_y: i32,
    pub forge_x: Option<i32>,
    pub forge_y: Option<i32>,
    pub kiln_x: Option<i32>,
    pub kiln_y: Option<i32>,
    pub well_x: Option<i32>,
    pub well_y: Option<i32>,
    pub fire_x: Option<i32>,
    pub fire_y: Option<i32>,
    pub graveyard_x: Option<i32>,
    pub graveyard_y: Option<i32>,
    /// Optional oven tile (home often is oven; used for clay-bowl overflow).
    pub oven_x: Option<i32>,
    pub oven_y: Option<i32>,
}

impl DropHeldAnchors {
    pub fn home_only(home_x: i32, home_y: i32) -> Self {
        Self {
            home_x,
            home_y,
            forge_x: None,
            forge_y: None,
            kiln_x: None,
            kiln_y: None,
            well_x: None,
            well_y: None,
            fire_x: None,
            fire_y: None,
            graveyard_x: None,
            graveyard_y: None,
            oven_x: None,
            oven_y: None,
        }
    }

    pub fn forge_xy(self) -> Option<(i32, i32)> {
        match (self.forge_x, self.forge_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    pub fn kiln_xy(self) -> Option<(i32, i32)> {
        match (self.kiln_x, self.kiln_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    pub fn well_xy(self) -> Option<(i32, i32)> {
        match (self.well_x, self.well_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    pub fn fire_xy(self) -> Option<(i32, i32)> {
        match (self.fire_x, self.fire_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }

    pub fn graveyard_xy(self) -> Option<(i32, i32)> {
        match (self.graveyard_x, self.graveyard_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }
}

/// Inputs for pure [`drop_held_object`].
// Haxe: dropHeldObject(maxDistanceToHome, allowAllPiles, target)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropHeldInput {
    pub held_id: i32,
    pub held_uses: i32,
    pub is_wound: bool,
    pub is_hidden_wound: bool,
    /// Haxe `maxDistanceToHome` (default 40). `< 1` clears dropTarget.
    pub max_distance_to_home: f32,
    pub allow_all_piles: bool,
    /// `getPileObjId()`; ≤0 = no pile.
    pub pile_id: i32,
    /// Haxe `itemToCraft.lastTargetId` — `pileId == lastTargetId` → `pileId = -1`.
    // Haxe: AiBase.dropHeldObject L5291
    pub last_target_id: i32,
    /// Haxe `itemToCraft.lastNewTargetId` — skip drop tile with that parent.
    pub last_new_target_id: i32,
    pub player_x: i32,
    pub player_y: i32,
    pub food_store: f32,
    pub is_moving: bool,
    /// When true, food target is active → refuse far piles (closeUseQuadDistance).
    pub has_food_target: bool,
    /// Close-use quad distance cap when hungry (Haxe `closeUseQuadDistance` = 400).
    // Haxe: AiBase L33
    pub close_use_quad_distance: i32,
    /// Held basket contains Clay 126 (nested).
    pub held_contains_clay: bool,
    pub quiver: QuiverClothing,
    pub anchors: DropHeldAnchors,
    /// Explicit drop target; defaults to home when unset.
    pub target_x: Option<i32>,
    pub target_y: Option<i32>,
    /// UseUpDough extras.
    pub use_target_is_plate: bool,
    pub has_knife_near: bool,
    pub count_bread_family: i32,
}

impl DropHeldInput {
    pub fn basic(held_id: i32, player_x: i32, player_y: i32, home_x: i32, home_y: i32) -> Self {
        Self {
            held_id,
            held_uses: 1,
            is_wound: false,
            is_hidden_wound: false,
            max_distance_to_home: 40.0,
            allow_all_piles: false,
            pile_id: -1,
            last_target_id: 0,
            last_new_target_id: 0,
            player_x,
            player_y,
            food_store: 20.0,
            is_moving: false,
            has_food_target: false,
            close_use_quad_distance: CLOSE_USE_QUAD_DISTANCE,
            held_contains_clay: false,
            quiver: QuiverClothing::default(),
            anchors: DropHeldAnchors::home_only(home_x, home_y),
            target_x: None,
            target_y: None,
            use_target_is_plate: false,
            has_knife_near: false,
            count_bread_family: 0,
        }
    }
}

/// Count parent ids in scan within Chebyshev `r` of `(cx, cy)`.
// Haxe: AiHelper.CountCloseObjects
pub fn count_near(tiles: &[ScanTile], cx: i32, cy: i32, parent_id: i32, r: i32) -> i32 {
    if parent_id <= 0 {
        return 0;
    }
    let r = r.max(0);
    tiles
        .iter()
        .filter(|t| t.parent_id == parent_id && scan_chebyshev(cx, cy, t.x, t.y) <= r)
        .count() as i32
}

/// Count including piles of same base (when `pile_id > 0`).
pub fn count_near_with_piles(
    tiles: &[ScanTile],
    cx: i32,
    cy: i32,
    parent_id: i32,
    pile_id: i32,
    r: i32,
    count_piles: bool,
) -> i32 {
    count_close_objects(tiles, cx, cy, parent_id, r, count_piles, pile_id)
}

/// Haxe `AiHelper.CountCloseObjects` — half-open square; piles add `numberOfUses`.
// Haxe: CountCloseObjectsHelper `ty-radius...ty+radius` exclusive end; pile += uses
pub fn count_close_objects(
    tiles: &[ScanTile],
    tx: i32,
    ty: i32,
    obj_id: i32,
    radius: i32,
    count_piles: bool,
    pile_id: i32,
) -> i32 {
    if obj_id <= 0 {
        return 0;
    }
    let mut n = 0i32;
    for t in tiles {
        if !in_count_close_square(tx, ty, t.x, t.y, radius) {
            continue;
        }
        if t.parent_id == obj_id {
            n += 1;
        } else if count_piles && pile_id > 0 && t.parent_id == pile_id {
            n += t.uses.max(1);
        }
    }
    n
}

/// Closest non-full multi-use object of `parent_id`.
///
/// When `num_uses` is known on the tile, skip full piles (`uses >= num_uses`).
/// Otherwise fall back to `max_uses` ceiling when `max_uses > 0`.
// Haxe: numberOfUses < objectData.numUses / ignoreFullPiles
fn closest_partial_uses(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    max_uses: i32,
) -> Option<ScanTile> {
    closest_partial_uses_skip(tiles, parent_id, from_x, from_y, max_r, max_uses, None)
}

/// Like [`closest_partial_uses`] but optionally skips one tile (second-closest scan).
// Haxe: GetClosestObjectById(..., ignoreObj, ...) second pass for full first bowl
fn closest_partial_uses_skip(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    max_uses: i32,
    skip: Option<(i32, i32)>,
) -> Option<ScanTile> {
    let mut best: Option<(i32, ScanTile)> = None;
    for t in tiles {
        if t.parent_id != parent_id {
            continue;
        }
        if let Some((sx, sy)) = skip {
            if t.x == sx && t.y == sy {
                continue;
            }
        }
        // Prefer content-aware full check; else heuristic max_uses
        if t.is_full_uses() {
            continue;
        }
        if t.num_uses <= 1 && max_uses > 0 && t.uses >= max_uses {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, *t)),
            Some((bd, _)) if d < bd => best = Some((d, *t)),
            Some((bd, bo)) if d == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)) => {
                best = Some((d, *t))
            }
            _ => {}
        }
    }
    best.map(|(_, t)| t)
}

/// Closest of `parent_id`, optionally requiring `contains_id` match (prefer first).
// Haxe: GetClosestObjectToPosition(..., searchContained)
fn closest_with_contains(
    tiles: &[ScanTile],
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    prefer_contains: i32,
) -> Option<ScanTile> {
    if prefer_contains > 0 {
        let mut best: Option<(i32, ScanTile)> = None;
        for t in tiles {
            if t.parent_id != parent_id || !t.contains_parent(prefer_contains) {
                continue;
            }
            let d = scan_chebyshev(from_x, from_y, t.x, t.y);
            if d > max_r {
                continue;
            }
            match best {
                None => best = Some((d, *t)),
                Some((bd, _)) if d < bd => best = Some((d, *t)),
                Some((bd, bo)) if d == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)) => {
                    best = Some((d, *t))
                }
                _ => {}
            }
        }
        if best.is_some() {
            return best.map(|(_, t)| t);
        }
    }
    closest_by_parent_id(tiles, parent_id, from_x, from_y, max_r)
}

/// Closest free-slot container (no held-type filter; Chebyshev).
// Haxe: numSlots drop-in generic
fn closest_free_container(
    tiles: &[ScanTile],
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_d: i32,
) -> Option<ScanTile> {
    let mut best: Option<(i32, ScanTile)> = None;
    for t in tiles {
        if !t.has_free_slot() {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d < min_d || d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, *t)),
            Some((bd, _)) if d < bd => best = Some((d, *t)),
            Some((bd, bo)) if d == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)) => {
                best = Some((d, *t))
            }
            _ => {}
        }
    }
    best.map(|(_, t)| t)
}

/// Best free container for table/small-food held using Haxe prefer factors.
// Haxe: ShouldDropOnTable / isSmallFoodToStore container scoring ~232–272
pub fn closest_preferred_container(
    tiles: &[ScanTile],
    held_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_d: i32,
) -> Option<ScanTile> {
    if !allows_drop_in_container(held_id) {
        return None;
    }
    let mut best: Option<(f32, i32, ScanTile)> = None; // score, cheby, tile
    for t in tiles {
        let Some(factor) = container_prefer_factor(held_id, t) else {
            continue;
        };
        let cheb = scan_chebyshev(from_x, from_y, t.x, t.y);
        if cheb < min_d || cheb > max_r {
            continue;
        }
        let score = adjust_container_drop_score(quad_distance_xy(from_x, from_y, t.x, t.y), factor);
        match best {
            None => best = Some((score, cheb, *t)),
            Some((bs, bd, bo)) => {
                if score < bs
                    || (score == bs
                        && (cheb < bd
                            || (cheb == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)))))
                {
                    best = Some((score, cheb, *t));
                }
            }
        }
    }
    best.map(|(_, _, t)| t)
}

/// Joint empty-tile + preferred-container pick (Haxe empty search with factor prefer).
// Haxe: GetClosestObjectToPositionHelper objIdToSearch==0 + container factors
fn best_empty_or_container_drop(
    tiles: &[ScanTile],
    held_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    opts: ClosestEmptyOpts,
) -> Option<(ScanTile, bool)> {
    // bool = drop_in_container
    let min_d = opts.min_distance.max(0);
    let allow_cont = allows_drop_in_container(held_id);
    let mut best: Option<(f32, i32, ScanTile, bool)> = None;

    if allow_cont {
        for t in tiles {
            let Some(factor) = container_prefer_factor(held_id, t) else {
                continue;
            };
            let cheb = scan_chebyshev(from_x, from_y, t.x, t.y);
            if cheb < min_d || cheb > max_r {
                continue;
            }
            let score =
                adjust_container_drop_score(quad_distance_xy(from_x, from_y, t.x, t.y), factor);
            match best {
                None => best = Some((score, cheb, *t, true)),
                Some((bs, bd, bo, _)) => {
                    if score < bs
                        || (score == bs
                            && (cheb < bd
                                || (cheb == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)))))
                    {
                        best = Some((score, cheb, *t, true));
                    }
                }
            }
        }
    }

    if let Some((ex, ey)) = closest_empty_tile_ex(tiles, from_x, from_y, max_r, opts) {
        let cheb = scan_chebyshev(from_x, from_y, ex, ey);
        let score = adjust_container_drop_score(quad_distance_xy(from_x, from_y, ex, ey), 1.0);
        let empty = ScanTile::empty(ex, ey, 0, 0);
        match best {
            None => best = Some((score, cheb, empty, false)),
            Some((bs, bd, bo, _)) => {
                if score < bs
                    || (score == bs
                        && (cheb < bd || (cheb == bd && (ey < bo.y || (ey == bo.y && ex < bo.x)))))
                {
                    best = Some((score, cheb, empty, false));
                }
            }
        }
    }

    best.map(|(_, _, t, is_c)| (t, is_c))
}

/// Haxe `shortCraftOnTarget` CountCloseObjects radius for maxNewActor.
// Haxe: AiBase.shortCraftOnTarget ~2755 CountCloseObjects(..., 30)
pub const SHORT_CRAFT_NEW_ACTOR_COUNT_RADIUS: i32 = 30;

/// Haxe `GetTransition(actor, target).newActorID`.
// Haxe: TransitionImporter.GetTransition(actorId, target.parentId)
pub fn short_craft_transition_new_actor(
    content: &ol_content::ContentDb,
    actor: i32,
    target: i32,
) -> Option<i32> {
    content.find_transition(actor, target).map(|t| t.new_actor_id)
}

/// Nearby `newActor` count + held match (Haxe maxNewActor gate).
// Haxe: CountCloseObjects(newActorID, 30) + (held == newActorID)
pub fn count_short_craft_new_actor(
    tiles: &[ScanTile],
    px: i32,
    py: i32,
    held_id: i32,
    new_actor_id: i32,
) -> i32 {
    let near = count_near(tiles, px, py, new_actor_id, SHORT_CRAFT_NEW_ACTOR_COUNT_RADIUS);
    crate::farmer_profession::new_actor_count_with_held(near, held_id, new_actor_id)
}

/// True when maxNewActor > 0 and nearby newActor is already at cap.
// Haxe: if (countActor >= maxNewActor) return false
pub fn max_new_actor_refuses(max_new_actor: i32, new_actor_count: i32) -> bool {
    max_new_actor > 0 && new_actor_count >= max_new_actor
}

/// Prefer shortCraft if target id exists in scan within `max_search`.
///
/// When `max_new_actor > 0` and `content` has a transition, count
/// `trans.newActorID` (r=30) and skip if at cap (Haxe shortCraftOnTarget).
fn prefer_short_if_present(
    tiles: &[ScanTile],
    actor: i32,
    target: i32,
    from_x: i32,
    from_y: i32,
    max_search: i32,
    craft_actor: bool,
    max_new_actor: i32,
    held_id: i32,
    content: Option<&ol_content::ContentDb>,
) -> Option<DropHeldDecision> {
    if closest_by_parent_id(tiles, target, from_x, from_y, max_search).is_none() {
        return None;
    }
    if max_new_actor > 0 {
        if let Some(db) = content {
            if let Some(new_id) = short_craft_transition_new_actor(db, actor, target) {
                let count = count_short_craft_new_actor(tiles, from_x, from_y, held_id, new_id);
                if max_new_actor_refuses(max_new_actor, count) {
                    return None;
                }
            }
        }
    }
    Some(DropHeldDecision::PreferShortCraft {
        actor,
        target,
        max_search,
        craft_actor,
        max_new_actor,
    })
}

/// Core pure dropHeldObject planner (no content → maxNewActor gate skipped).
// Haxe: AiBase.dropHeldObject ~5267–5649
pub fn drop_held_object(inp: DropHeldInput, tiles: &[ScanTile]) -> DropHeldDecision {
    drop_held_object_ex(inp, tiles, None)
}

/// dropHeldObject with ContentDb for `GetTransition(...).newActorID` maxNewActor.
// Haxe: AiBase.shortCraftOnTarget ~2749 CountCloseObjects(trans.newActorID, 30)
pub fn drop_held_object_ex(
    inp: DropHeldInput,
    tiles: &[ScanTile],
    content: Option<&ol_content::ContentDb>,
) -> DropHeldDecision {
    let held = inp.held_id;
    if held <= 0 {
        return DropHeldDecision::None;
    }
    if inp.is_wound || inp.is_hidden_wound {
        return DropHeldDecision::RefuseWound;
    }

    if let Some(d) = store_in_quiver(held, inp.quiver) {
        return d;
    }

    let plate_available = closest_by_parent_id(
        tiles,
        CLAY_PLATE,
        inp.player_x,
        inp.player_y,
        USE_UP_DOUGH_PLATE_RADIUS,
    )
    .is_some();
    if let Some(d) = use_up_dough(UseUpDoughInput {
        held_id: held,
        held_uses: inp.held_uses,
        use_target_is_plate: inp.use_target_is_plate,
        has_knife_near: inp.has_knife_near,
        count_bread_family: inp.count_bread_family,
        plate_available,
    }) {
        return d;
    }

    if let Some(d) = special_held_actions(inp, tiles, content) {
        return d;
    }

    let mut target_x = inp.target_x.unwrap_or(inp.anchors.home_x);
    let mut target_y = inp.target_y.unwrap_or(inp.anchors.home_y);
    let mut drop_close_to_player = true;
    let mut min_distance: i32 = 0;
    let pile_after_last = pile_id_after_last_craft_target(inp.pile_id, inp.last_target_id);
    let mut pile_id = if pile_blocked(held, inp.allow_all_piles) {
        0
    } else {
        pile_after_last
    };

    if held == BASKET_OF_BONES && inp.max_distance_to_home > 5.0 {
        if let Some((gx, gy)) = inp.anchors.graveyard_xy() {
            target_x = gx;
            target_y = gy;
            drop_close_to_player = false;
        }
    }

    if held == CLAY && inp.max_distance_to_home > 5.0 {
        if let Some((kx, ky)) = inp.anchors.kiln_xy() {
            drop_close_to_player = false;
            target_x = kx;
            target_y = ky;
        }
        // Haxe: distance is to `target` after optional kiln assign (home if no kiln).
        let dist_q = {
            let dx = inp.player_x - target_x;
            let dy = inp.player_y - target_y;
            dx * dx + dy * dy
        };
        if dist_q > 400 {
            // Haxe: basket containing clay [126], else any basket r=10 from player
            if let Some(b) =
                closest_with_contains(tiles, BASKET, inp.player_x, inp.player_y, 10, CLAY)
            {
                return DropHeldDecision::UseAt {
                    x: b.x,
                    y: b.y,
                    target_id: BASKET,
                    actor_id: CLAY,
                };
            }
            if let Some(b) = closest_by_parent_id(tiles, BASKET, inp.player_x, inp.player_y, 10) {
                return DropHeldDecision::UseAt {
                    x: b.x,
                    y: b.y,
                    target_id: BASKET,
                    actor_id: CLAY,
                };
            }
        }
    }

    if (held == FLAT_ROCK || held == STONE) && inp.max_distance_to_home > 5.0 {
        if let Some((fx, fy)) = inp.anchors.forge_xy() {
            let max_items = if held == FLAT_ROCK { 3 } else { 1 };
            let count_piles = held == STONE;
            let count = count_near_with_piles(
                tiles,
                fx,
                fy,
                held,
                if count_piles { pile_id } else { 0 },
                3,
                count_piles,
            );
            if count < max_items {
                if held == FLAT_ROCK {
                    pile_id = 0;
                }
                drop_close_to_player = false;
                target_x = fx;
                target_y = fy;
                min_distance = -1;
            }
        }
    }

    if held == BASKET && inp.held_contains_clay && inp.max_distance_to_home > 5.0 {
        pile_id = 0;
        if let Some((kx, ky)) = inp.anchors.kiln_xy() {
            target_x = kx;
            target_y = ky;
            drop_close_to_player = false;
            if let Some((ex, ey)) =
                closest_empty_tile_ex(tiles, kx, ky, 5, ClosestEmptyOpts::basic())
            {
                return DropHeldDecision::DropAt { x: ex, y: ey };
            }
            if let Some(sw) = closest_switch_tile(tiles, kx, ky, held, 20) {
                return DropHeldDecision::DropAt { x: sw.x, y: sw.y };
            }
        }
    }

    if should_drop_near_oven_held(held) && inp.max_distance_to_home > 5.0 {
        target_x = inp.anchors.home_x;
        target_y = inp.anchors.home_y;
        drop_close_to_player = false;
        let mut count = 0;
        if held == CLAY_BOWL {
            // Haxe: CountCloseObjects home r=15 default countPiles
            count = count_close_objects(
                tiles,
                target_x,
                target_y,
                CLAY_BOWL,
                15,
                true,
                inp.pile_id,
            );
        }
        if count >= 3 {
            if let Some((fx, fy)) = inp.anchors.forge_xy() {
                let fc = count_close_objects(tiles, fx, fy, CLAY_BOWL, 15, true, inp.pile_id);
                if fc < 3 {
                    target_x = fx;
                    target_y = fy;
                    pile_id = 0;
                    count = fc;
                } else {
                    count = 100;
                }
            } else {
                count = 100;
            }
        }
        if count >= 3 {
            drop_close_to_player = true;
        }
        if held == SHOVEL {
            let dx = inp.player_x - target_x;
            let dy = inp.player_y - target_y;
            if dx * dx + dy * dy <= 225 {
                drop_close_to_player = true;
            }
        }
    }

    if should_drop_near_forge(held) && inp.max_distance_to_home > 5.0 {
        if let Some((fx, fy)) = inp.anchors.forge_xy() {
            target_x = fx;
            target_y = fy;
            drop_close_to_player = false;
        }
    }

    if held == CLAY_PLATE_ID && inp.max_distance_to_home > 5.0 {
        // Haxe L5500: CountCloseObjects(target, 236, 10, false)
        let count = count_close_objects(tiles, target_x, target_y, held, 10, false, 0);
        if count < 5 {
            pile_id = 0;
            if let Some((ex, ey)) =
                closest_empty_tile_ex(tiles, target_x, target_y, 5, ClosestEmptyOpts::basic())
            {
                return DropHeldDecision::DropAt { x: ex, y: ey };
            }
            if let Some(sw) = closest_switch_tile(tiles, target_x, target_y, held, 20) {
                return DropHeldDecision::DropAt { x: sw.x, y: sw.y };
            }
        }
    }

    if should_drop_near_fire(held) && inp.max_distance_to_home > 5.0 {
        if let Some((fx, fy)) = inp.anchors.fire_xy() {
            target_x = fx;
            target_y = fy;
        }
        drop_close_to_player = false;
    }

    if should_drop_near_well(held) && inp.max_distance_to_home > 5.0 {
        if let Some((wx, wy)) = inp.anchors.well_xy().or_else(|| {
            // Haxe getCloseWell: GetClosestObjectToPositionByIds home default r=40
            closest_well(tiles, inp.anchors.home_x, inp.anchors.home_y, 40).map(|t| (t.x, t.y))
        }) {
            target_x = wx;
            target_y = wy;
        } else {
            target_x = inp.anchors.home_x;
            target_y = inp.anchors.home_y;
        }
        drop_close_to_player = false;
    }

    let mut drop_on_start = min_distance < 1 && !force_drop_at_feet(held);
    if drop_close_to_player {
        drop_on_start = false;
    }

    if drop_on_start && inp.max_distance_to_home > 0.0 {
        if inp.is_moving {
            return DropHeldDecision::BusyMoving;
        }
        let max_q = (inp.max_distance_to_home * inp.max_distance_to_home) as i32;
        let dx = inp.player_x - target_x;
        let dy = inp.player_y - target_y;
        let quad = dx * dx + dy * dy;
        if quad > DROP_CLOSE_ENOUGH_QUAD && quad < max_q {
            return DropHeldDecision::Goto {
                x: target_x,
                y: target_y,
            };
        }
        if quad > max_q {
            drop_close_to_player = true;
        }
    }

    if drop_close_to_player {
        target_x = inp.player_x;
        target_y = inp.player_y;
    }

    if pile_blocked(held, inp.allow_all_piles) {
        pile_id = 0;
    }

    // Haxe: mindistance + 4*i with mindistance=-1 → rings 3,7,11…; minDist<0 allows d=0
    // Haxe: AiHelper GetClosestObjectToPositionHelper minDistance < 0 → quadMin=0
    let ring_base = min_distance; // may be -1 (flat rock / stone near forge)
    let min_d_filter = min_distance.max(0);
    let mut new_drop: Option<ScanTile> = None;
    let mut drop_in_container = false;
    for i in 1..11 {
        let search_distance = ring_base + 4 * i;
        if search_distance > DROP_HELD_MAX_SEARCH {
            break;
        }
        if search_distance < 0 {
            continue;
        }

        if pile_id > 0 {
            // Haxe: ignoreFullPiles + numberOfUses >= numUses → null
            new_drop = closest_partial_uses(
                tiles,
                pile_id,
                target_x,
                target_y,
                search_distance,
                0, // rely on tile.num_uses when known
            )
            .and_then(|t| if t.is_full_uses() { None } else { Some(t) });
        }

        if new_drop.is_none() && pile_id > 0 {
            new_drop = closest_by_parent_id(tiles, held, target_x, target_y, search_distance)
                .filter(|t| scan_chebyshev(target_x, target_y, t.x, t.y) >= min_d_filter);
        }

        if let Some(t) = new_drop {
            if inp.has_food_target {
                // Haxe: AiBase.dropHeldObject L5585–5587 (closeUseQuadDistance = 400)
                let dx = inp.player_x - t.x;
                let dy = inp.player_y - t.y;
                if dx * dx + dy * dy > inp.close_use_quad_distance {
                    new_drop = None;
                }
            }
        }

        // Haxe: empty search + table/small-food container prefer (DROP-HELD-TABLE)
        // Only ShouldDropOnTable / isSmallFoodToStore may USE-drop into free containers.
        if new_drop.is_none() {
            let opts = ClosestEmptyOpts {
                held_id: held,
                home_x: inp.anchors.home_x,
                home_y: inp.anchors.home_y,
                min_distance: min_d_filter,
                respect_home_clearance: min_distance >= 0,
                respect_not_floored: true,
            };
            if allows_drop_in_container(held) {
                if let Some((t, is_cont)) = best_empty_or_container_drop(
                    tiles,
                    held,
                    target_x,
                    target_y,
                    search_distance,
                    opts,
                ) {
                    new_drop = Some(t);
                    drop_in_container = is_cont;
                }
            } else if let Some((ex, ey)) =
                closest_empty_tile_ex(tiles, target_x, target_y, search_distance, opts)
            {
                new_drop = Some(ScanTile::empty(ex, ey, 0, 0));
                drop_in_container = false;
            }
            // Non-table/small-food: never free-container fallback (Haxe L195 gate).
        }

        if let Some(t) = new_drop {
            if t.parent_id > 0 && t.parent_id == inp.last_new_target_id {
                let opts = ClosestEmptyOpts::for_held(held, inp.anchors.home_x, inp.anchors.home_y);
                new_drop =
                    closest_empty_tile_ex(tiles, target_x, target_y, DROP_HELD_MAX_SEARCH, opts)
                        .map(|(x, y)| ScanTile::empty(x, y, 0, 0));
                drop_in_container = false;
            }
        }

        if new_drop.is_some() {
            break;
        }
    }

    let Some(tile) = new_drop else {
        return DropHeldDecision::DropAt {
            x: inp.player_x,
            y: inp.player_y,
        };
    };

    // Haxe: numSlots > 0 on empty-search hit → USE drop-in container
    if drop_in_container || (tile.num_slots > 0 && tile.parent_id > 0) {
        return DropHeldDecision::UseAsDrop {
            x: tile.x,
            y: tile.y,
            target_id: tile.parent_id,
            actor_id: held,
        };
    }

    if tile.parent_id == 0 && !must_use_as_drop(held) {
        DropHeldDecision::DropAt {
            x: tile.x,
            y: tile.y,
        }
    } else {
        DropHeldDecision::UseAsDrop {
            x: tile.x,
            y: tile.y,
            target_id: tile.parent_id,
            actor_id: held,
        }
    }
}

/// Skewered Rabbit (fire-near item with cook shortCraft).
// Haxe: considerDropHeldObject shortCraft(185, 85, 20, false)
pub const SKEWERED_RABBIT: i32 = 185;

/// Special-case shortCraft / useHeld before spatial drop.
fn special_held_actions(
    inp: DropHeldInput,
    tiles: &[ScanTile],
    content: Option<&ol_content::ContentDb>,
) -> Option<DropHeldDecision> {
    let held = inp.held_id;
    let px = inp.player_x;
    let py = inp.player_y;
    let prefer = |actor, target, max_search, craft_actor, max_new_actor| {
        prefer_short_if_present(
            tiles,
            actor,
            target,
            px,
            py,
            max_search,
            craft_actor,
            max_new_actor,
            held,
            content,
        )
    };

    // Haxe: considerDropHeldObject — Skewered Rabbit 185 + Hot Coals 85
    if held == SKEWERED_RABBIT {
        if let Some(d) = prefer(SKEWERED_RABBIT, HOT_COALS, 20, false, i32::MAX) {
            return Some(d);
        }
    }

    if held == RAW_MUTTON {
        if let Some(d) = prefer(RAW_MUTTON, HOT_ADOBE_OVEN, 10, false, 4) {
            return Some(d);
        }
        if let Some(d) = prefer(RAW_MUTTON, HOT_COALS, 10, false, 4) {
            return Some(d);
        }
    }

    if held == BOWL_OF_SOIL {
        if let Some(d) = prefer(BOWL_OF_SOIL, DYING_BUSH, 15, false, i32::MAX) {
            return Some(d);
        }
        if let Some(d) = prefer(BOWL_OF_SOIL, HARDENED_ROW, 15, false, i32::MAX) {
            return Some(d);
        }
    }

    if held == STONE_HOE && inp.food_store > 3.0 && inp.max_distance_to_home > 5.0 {
        let baskets = count_near(tiles, px, py, BASKET, 30);
        if baskets > 15 {
            if let Some(d) = prefer(STONE_HOE, BASKET, 15, true, i32::MAX) {
                return Some(d);
            }
        }
        if let Some(d) = prefer(STONE_HOE, SHALLOW_TILLED_ROW, 15, true, i32::MAX) {
            return Some(d);
        }
        if let Some(d) = prefer(STONE_HOE, FERTILE_SOIL, 15, true, i32::MAX) {
            return Some(d);
        }
    }

    if held == STEEL_HOE && inp.food_store > 2.0 && inp.max_distance_to_home > 5.0 {
        if let Some(d) = prefer(STEEL_HOE, SHALLOW_TILLED_ROW, 15, false, i32::MAX) {
            return Some(d);
        }
        if let Some(d) = prefer(STEEL_HOE, FERTILE_SOIL, 15, false, i32::MAX) {
            return Some(d);
        }
    }

    // Haxe: heldObjId == 1160 || heldObjId == 31 && maxDistanceToHome > 5 (port-as-is precedence)
    // Dry bean pod always tries bowl fill; gooseberry only when maxDistanceToHome > 5.
    if held == DRY_BEAN_POD || (held == GOOSEBERRY && inp.max_distance_to_home > 5.0) {
        let bowl_id = if held == DRY_BEAN_POD {
            BOWL_OF_DRY_BEANS
        } else {
            BOWL_OF_GOOSEBERRIES
        };
        // Haxe: closest bowl; if full, second-closest; then clay bowl
        if let Some(b) = closest_partial_uses(tiles, bowl_id, px, py, 30, 0) {
            return Some(DropHeldDecision::UseAt {
                x: b.x,
                y: b.y,
                target_id: bowl_id,
                actor_id: held,
            });
        }
        // First hit may be full — Haxe re-scans ignoring first bowl
        if let Some(first) = closest_by_parent_id(tiles, bowl_id, px, py, 30) {
            if first.is_full_uses() {
                if let Some(b) = closest_partial_uses_skip(
                    tiles,
                    bowl_id,
                    px,
                    py,
                    30,
                    0,
                    Some((first.x, first.y)),
                ) {
                    return Some(DropHeldDecision::UseAt {
                        x: b.x,
                        y: b.y,
                        target_id: bowl_id,
                        actor_id: held,
                    });
                }
            }
        }
        if let Some(b) = closest_by_parent_id(tiles, CLAY_BOWL, px, py, 30) {
            return Some(DropHeldDecision::UseAt {
                x: b.x,
                y: b.y,
                target_id: CLAY_BOWL,
                actor_id: held,
            });
        }
    }

    if held == HOT_IRON_BLOOM_TONGS {
        if let Some(d) = prefer(HOT_IRON_BLOOM_TONGS, FLAT_ROCK, 10, false, i32::MAX) {
            return Some(d);
        }
    }

    if held == SHOVEL_OF_DUNG && inp.max_distance_to_home > 5.0 {
        if let Some(d) = prefer(SHOVEL_OF_DUNG, WET_COMPOST, 20, false, i32::MAX) {
            return Some(d);
        }
    }

    if held == BOWL_OF_WHEAT {
        // Haxe: CountCloseObjects 242 r=20 default piles; 228 r=20 countPiles=false
        let ripe_pile = content
            .map(|c| crate::get_or_craft::pile_obj_id_from_content(c, RIPE_WHEAT))
            .filter(|&id| id > 0)
            .unwrap_or(0);
        let count_wheat = count_close_objects(tiles, px, py, RIPE_WHEAT, 20, true, ripe_pile)
            + count_close_objects(tiles, px, py, DRY_PLANTED_WHEAT, 20, false, 0);
        if count_wheat < 10 {
            if let Some(d) = prefer(BOWL_OF_WHEAT, DEEP_TILLED_ROW, 20, false, i32::MAX) {
                return Some(d);
            }
        }
    }

    None
}

/// Closest non-permanent tile that is not the held id (Haxe GetClosestObjectToTarget -10).
fn closest_switch_tile(
    tiles: &[ScanTile],
    from_x: i32,
    from_y: i32,
    held_id: i32,
    max_r: i32,
) -> Option<ScanTile> {
    let mut best: Option<(i32, ScanTile)> = None;
    for t in tiles {
        if t.parent_id == 0 || t.parent_id == held_id || t.is_permanent {
            continue;
        }
        let d = scan_chebyshev(from_x, from_y, t.x, t.y);
        if d == 0 || d > max_r {
            continue;
        }
        match best {
            None => best = Some((d, *t)),
            Some((bd, _)) if d < bd => best = Some((d, *t)),
            Some((bd, bo)) if d == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)) => {
                best = Some((d, *t))
            }
            _ => {}
        }
    }
    best.map(|(_, t)| t)
}

/// Fill forge/kiln/well anchors from a scan around home when not pre-set.
// Haxe: GetForge / GetKiln / getCloseWell
pub fn fill_anchors_from_scan(mut anchors: DropHeldAnchors, tiles: &[ScanTile]) -> DropHeldAnchors {
    if anchors.forge_xy().is_none() {
        if let Some(t) = tiles
            .iter()
            .filter(|t| is_forge_id(t.parent_id))
            .min_by_key(|t| {
                (
                    scan_chebyshev(anchors.home_x, anchors.home_y, t.x, t.y),
                    t.y,
                    t.x,
                )
            })
        {
            anchors.forge_x = Some(t.x);
            anchors.forge_y = Some(t.y);
        }
    }
    if anchors.well_xy().is_none() {
        if let Some(w) = closest_well(tiles, anchors.home_x, anchors.home_y, 40) {
            anchors.well_x = Some(w.x);
            anchors.well_y = Some(w.y);
        }
    }
    if anchors.kiln_xy().is_none() {
        const KILN_IDS: [i32; 3] = [283, 642, 238];
        if let Some(t) = tiles
            .iter()
            .filter(|t| KILN_IDS.contains(&t.parent_id))
            .min_by_key(|t| {
                (
                    scan_chebyshev(anchors.home_x, anchors.home_y, t.x, t.y),
                    t.y,
                    t.x,
                )
            })
        {
            anchors.kiln_x = Some(t.x);
            anchors.kiln_y = Some(t.y);
        }
    }
    let _ = WELL_IDS;
    anchors
}

/// considerDropHeldObject: should we interrupt a goto to drop staging items?
// Haxe: considerDropHeldObject ~5191–5234
pub fn consider_drop_held_object(
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    goto_x: i32,
    goto_y: i32,
) -> bool {
    matches!(
        consider_drop_held_decision(
            held_id,
            player_x,
            player_y,
            home_x,
            home_y,
            goto_x,
            goto_y,
            &[]
        ),
        Some(_)
    )
}

/// Rich consider path: PreferShortCraft (185→85) or flag that dropHeld should run.
// Haxe: considerDropHeldObject — shortCraft(185,85) then dropHeldObject
pub fn consider_drop_held_decision(
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    goto_x: i32,
    goto_y: i32,
    tiles: &[ScanTile],
) -> Option<DropHeldDecision> {
    consider_drop_held_decision_ex(
        held_id, 1, player_x, player_y, home_x, home_y, goto_x, goto_y, tiles, false, 0, false,
    )
}

/// considerDropHeld with dough uses / knife / bread sensors (UseUpDough before fire/oven).
// Haxe: considerDropHeldObject ~5191–5234 UseUpDough ~5203 then 185→85
pub fn consider_drop_held_decision_ex(
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    goto_x: i32,
    goto_y: i32,
    tiles: &[ScanTile],
    has_knife_near: bool,
    count_bread_family: i32,
    use_target_is_plate: bool,
) -> Option<DropHeldDecision> {
    if held_id < 1 {
        return None;
    }
    if goto_x == home_x && goto_y == home_y {
        return None;
    }
    if held_id == BANANA_PEEL || held_id == SHARP_STONE {
        return Some(DropHeldDecision::None); // signal: run dropHeldObject
    }
    // Haxe: UseUpDough() ~5203 — before fire/oven/forge interrupt tables
    let plate_available = closest_by_parent_id(
        tiles,
        CLAY_PLATE,
        player_x,
        player_y,
        USE_UP_DOUGH_PLATE_RADIUS,
    )
    .is_some();
    if let Some(d) = use_up_dough(UseUpDoughInput {
        held_id,
        held_uses,
        use_target_is_plate,
        has_knife_near,
        count_bread_family,
        plate_available,
    }) {
        return Some(d);
    }
    // Haxe: shortCraft(185, 85, 20, false)
    if held_id == SKEWERED_RABBIT {
        if let Some(d) = prefer_short_if_present(
            tiles,
            SKEWERED_RABBIT,
            HOT_COALS,
            player_x,
            player_y,
            20,
            false,
            i32::MAX,
            held_id,
            None,
        ) {
            return Some(d);
        }
    }
    if should_drop_near_fire(held_id)
        || should_drop_near_oven_held(held_id)
        || should_drop_near_forge(held_id)
    {
        return Some(DropHeldDecision::None);
    }
    let dx_h = player_x - home_x;
    let dy_h = player_y - home_y;
    let quad_home = dx_h * dx_h + dy_h * dy_h;
    let dx_t = goto_x - home_x;
    let dy_t = goto_y - home_y;
    let quad_target = dx_t * dx_t + dy_t * dy_t;
    if quad_target + 25 < quad_home {
        return None;
    }
    Some(DropHeldDecision::None)
}

/// Resolve PreferShortCraft against scan → UseAt when target tile found.
///
/// When target is missing, leaves PreferShortCraft so [`DropHeldDecision::to_live_intent`]
/// can stage SeekOrCraft with `craft_actor` (Haxe GetOrCraftItem when not holding actor).
// Haxe: shortCraft → useHeldObjOnTarget (held==actor) / GetOrCraftItem (else)
// Haxe: PREFER-SHORT-WAIT — plan_drop_held_live always runs this before to_live_intent
pub fn resolve_prefer_short_craft(
    decision: DropHeldDecision,
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
) -> DropHeldDecision {
    match decision {
        DropHeldDecision::PreferShortCraft {
            actor,
            target,
            max_search,
            ..
        } => {
            if let Some(t) = closest_by_parent_id(tiles, target, player_x, player_y, max_search) {
                DropHeldDecision::UseAt {
                    x: t.x,
                    y: t.y,
                    target_id: target,
                    actor_id: actor,
                }
            } else {
                decision
            }
        }
        other => other,
    }
}

// ── DROP-HELD-LIVE: pure enqueue bridge ──────────────────────────────────────

/// Optional live sensors for building [`DropHeldInput`] from a world scan.
// Haxe: dropHeldObject sensors (quiver clothing, pileId, food target, knife, bread)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropHeldSensorExtras {
    pub quiver: QuiverClothing,
    pub held_contains_clay: bool,
    pub pile_id: i32,
    /// Haxe `itemToCraft.lastTargetId` (pile cancel).
    // Haxe: AiBase.dropHeldObject L5291
    pub last_target_id: i32,
    pub last_new_target_id: i32,
    pub has_food_target: bool,
    pub is_wound: bool,
    pub is_hidden_wound: bool,
    pub use_target_is_plate: bool,
    /// `None` → knife 560 at home r=30 (Haxe GetClosestObjectToPosition).
    pub has_knife_near: Option<bool>,
    /// `None` → count bread family (1471/1468/1466) near home r=20.
    pub count_bread_family: Option<i32>,
    pub close_use_quad_distance: i32,
    pub target_x: Option<i32>,
    pub target_y: Option<i32>,
}

impl Default for DropHeldSensorExtras {
    fn default() -> Self {
        Self {
            quiver: QuiverClothing::default(),
            held_contains_clay: false,
            pile_id: -1,
            last_target_id: 0,
            last_new_target_id: 0,
            has_food_target: false,
            is_wound: false,
            is_hidden_wound: false,
            use_target_is_plate: false,
            has_knife_near: None,
            count_bread_family: None,
            close_use_quad_distance: CLOSE_USE_QUAD_DISTANCE,
            target_x: None,
            target_y: None,
        }
    }
}

/// Count sliced bread + leavened plate/bowl near anchor (UseUpDough budget).
// Haxe: CountCloseObjects bread family near home for maxDoughInBowl
pub fn count_bread_family_near(tiles: &[ScanTile], cx: i32, cy: i32, r: i32) -> i32 {
    count_near(tiles, cx, cy, SLICED_BREAD_ID, r)
        + count_near(tiles, cx, cy, LEAVENED_DOUGH_PLATE, r)
        + count_near(tiles, cx, cy, BOWL_OF_LEAVENED_DOUGH, r)
}

/// Knife present within Chebyshev `r` of `(px, py)` (UseUpDough uses home).
// Haxe: GetClosestObjectToPosition(home, 560, 30)
pub fn has_knife_near_scan(tiles: &[ScanTile], px: i32, py: i32, r: i32) -> bool {
    closest_by_parent_id(tiles, KNIFE, px, py, r).is_some()
}

/// Build [`DropHeldInput`] from profession / npc sensors + fill anchors from scan.
// Haxe: dropHeldObject(maxDistanceToHome, allowAllPiles) + GetForge/GetKiln/well
pub fn drop_held_input_from_sensors(
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    is_moving: bool,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    tiles: &[ScanTile],
    extras: DropHeldSensorExtras,
) -> DropHeldInput {
    let anchors = fill_anchors_from_scan(DropHeldAnchors::home_only(home_x, home_y), tiles);
    let has_knife = extras.has_knife_near.unwrap_or_else(|| {
        has_knife_near_scan(tiles, home_x, home_y, USE_UP_DOUGH_KNIFE_HOME_RADIUS)
    });
    let bread = extras.count_bread_family.unwrap_or_else(|| {
        count_bread_family_near(tiles, home_x, home_y, USE_UP_DOUGH_BREAD_HOME_RADIUS)
    });
    DropHeldInput {
        held_id,
        held_uses: held_uses.max(1),
        is_wound: extras.is_wound,
        is_hidden_wound: extras.is_hidden_wound,
        max_distance_to_home,
        allow_all_piles,
        pile_id: extras.pile_id,
        last_target_id: extras.last_target_id,
        last_new_target_id: extras.last_new_target_id,
        player_x,
        player_y,
        food_store,
        is_moving,
        has_food_target: extras.has_food_target,
        close_use_quad_distance: extras.close_use_quad_distance,
        held_contains_clay: extras.held_contains_clay,
        quiver: extras.quiver,
        anchors,
        target_x: extras.target_x,
        target_y: extras.target_y,
        use_target_is_plate: extras.use_target_is_plate,
        has_knife_near: has_knife,
        count_bread_family: bread,
    }
}

/// Pure dropHeldObject then resolve PreferShortCraft → UseAt when target in scan.
// Haxe: dropHeldObject + shortCraft resolve before USE
pub fn plan_drop_held_live(inp: DropHeldInput, tiles: &[ScanTile]) -> DropHeldDecision {
    plan_drop_held_live_ex(inp, tiles, None)
}

/// Like [`plan_drop_held_live`] with content for maxNewActor `trans.newActorID`.
// Haxe: shortCraftOnTarget GetTransition + CountCloseObjects(newActorID, 30)
pub fn plan_drop_held_live_ex(
    inp: DropHeldInput,
    tiles: &[ScanTile],
    content: Option<&ol_content::ContentDb>,
) -> DropHeldDecision {
    let d = drop_held_object_ex(inp, tiles, content);
    resolve_prefer_short_craft(d, tiles, inp.player_x, inp.player_y)
}

/// Smart drop → live intent (USE/DROP/Goto/SelfClothing) for profession/npc enqueue.
// Haxe: dropHeldObject → useTarget/dropTarget/gotoObj/self
pub fn smart_drop_held_to_live_intent(
    inp: DropHeldInput,
    tiles: &[ScanTile],
) -> ShortCraftLiveIntent {
    smart_drop_held_to_live_intent_ex(inp, tiles, None)
}

/// Like [`smart_drop_held_to_live_intent`] with content maxNewActor gate.
pub fn smart_drop_held_to_live_intent_ex(
    inp: DropHeldInput,
    tiles: &[ScanTile],
    content: Option<&ol_content::ContentDb>,
) -> ShortCraftLiveIntent {
    plan_drop_held_live_ex(inp, tiles, content).to_live_intent()
}

/// Profession-tick convenience: sensors + allow_piles → live intent.
// Haxe: pottery DropHeld allowAllPiles / farm-smith shortCraft DropHeld
pub fn smart_drop_held_from_sensors(
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    is_moving: bool,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    tiles: &[ScanTile],
    extras: DropHeldSensorExtras,
) -> ShortCraftLiveIntent {
    smart_drop_held_from_sensors_ex(
        held_id,
        held_uses,
        player_x,
        player_y,
        home_x,
        home_y,
        food_store,
        is_moving,
        allow_all_piles,
        max_distance_to_home,
        tiles,
        extras,
        None,
    )
}

/// Like [`smart_drop_held_from_sensors`] with ContentDb maxNewActor count.
// Haxe: shortCraftOnTarget trans.newActorID CountCloseObjects r=30
pub fn smart_drop_held_from_sensors_ex(
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    is_moving: bool,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    tiles: &[ScanTile],
    extras: DropHeldSensorExtras,
    content: Option<&ol_content::ContentDb>,
) -> ShortCraftLiveIntent {
    let inp = drop_held_input_from_sensors(
        held_id,
        held_uses,
        player_x,
        player_y,
        home_x,
        home_y,
        food_store,
        is_moving,
        allow_all_piles,
        max_distance_to_home,
        tiles,
        extras,
    );
    smart_drop_held_to_live_intent_ex(inp, tiles, content)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smith_profession::FORGE;

    fn empty_grid(cx: i32, cy: i32, r: i32) -> Vec<ScanTile> {
        let mut v = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                v.push(ScanTile::empty(cx + dx, cy + dy, 0, 0));
            }
        }
        v
    }

    #[test]
    fn empty_hands_none() {
        let tiles = empty_grid(0, 0, 2);
        let d = drop_held_object(DropHeldInput::basic(0, 0, 0, 0, 0), &tiles);
        assert_eq!(d, DropHeldDecision::None);
    }

    #[test]
    fn wound_refuses() {
        let tiles = empty_grid(0, 0, 2);
        let mut inp = DropHeldInput::basic(33, 0, 0, 0, 0);
        inp.is_wound = true;
        assert_eq!(drop_held_object(inp, &tiles), DropHeldDecision::RefuseWound);
    }

    #[test]
    fn store_yew_bow_in_quiver() {
        let q = QuiverClothing::from_ids(&[EMPTY_ARROW_QUIVER_ID]);
        assert_eq!(
            store_in_quiver(YEW_BOW, q),
            Some(DropHeldDecision::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            })
        );
        assert_eq!(store_in_quiver(YEW_BOW, QuiverClothing::default()), None);
        // Haxe L5091–5099: Yew Bow 151 uses 874 then 3948; no canAddToQuiver
        let mut filled = QuiverClothing::from_ids(&[ARROW_QUIVER_ID]);
        filled.can_add = false;
        assert_eq!(
            store_in_quiver(YEW_BOW, filled),
            Some(DropHeldDecision::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            })
        );
    }

    #[test]
    fn store_arrow_requires_can_add() {
        let mut q = QuiverClothing::from_ids(&[ARROW_QUIVER_ID]);
        q.can_add = false;
        assert_eq!(store_in_quiver(ARROW, q), None);
        q.can_add = true;
        assert!(store_in_quiver(ARROW, q).is_some());
    }

    #[test]
    fn store_bow_and_arrow_requires_can_add() {
        // Haxe L5105–5116: held 152 → 874 then 3948 + canAddToQuiver; not 4149/4151
        let mut empty = QuiverClothing::from_ids(&[EMPTY_ARROW_QUIVER_ID]);
        empty.can_add = true;
        assert_eq!(
            store_in_quiver(BOW_AND_ARROW_ID, empty),
            Some(DropHeldDecision::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            })
        );
        empty.can_add = false;
        assert_eq!(store_in_quiver(BOW_AND_ARROW_ID, empty), None);
        let with_bow = QuiverClothing::from_ids(&[EMPTY_ARROW_QUIVER_WITH_BOW]);
        assert_eq!(store_in_quiver(BOW_AND_ARROW_ID, with_bow), None);
        let arrow_q = QuiverClothing::from_ids(&[ARROW_QUIVER_ID]);
        assert!(store_in_quiver(BOW_AND_ARROW_ID, arrow_q).is_some());
    }

    #[test]
    fn store_arrow_on_quiver_with_bow_ids() {
        // Haxe L5119–5134: Arrow 148 → 874 / 4149 / 3948 / 4151 + canAddToQuiver
        let q4149 = QuiverClothing::from_ids(&[EMPTY_ARROW_QUIVER_WITH_BOW]);
        assert!(store_in_quiver(ARROW, q4149).is_some());
        let q4151 = QuiverClothing::from_ids(&[ARROW_QUIVER_WITH_BOW]);
        assert!(store_in_quiver(ARROW, q4151).is_some());
        assert_eq!(store_in_quiver(ARROW, QuiverClothing::default()), None);
    }

    #[test]
    fn use_up_dough_when_extra_uses() {
        let d = use_up_dough(UseUpDoughInput {
            held_id: BOWL_OF_DOUGH,
            held_uses: 2,
            use_target_is_plate: false,
            has_knife_near: false,
            count_bread_family: 0,
            plate_available: true,
        });
        assert!(matches!(
            d,
            Some(DropHeldDecision::PreferShortCraft {
                actor: BOWL_OF_DOUGH,
                target: CLAY_PLATE,
                ..
            })
        ));
        assert_eq!(
            use_up_dough(UseUpDoughInput {
                held_id: BOWL_OF_DOUGH,
                held_uses: 5,
                use_target_is_plate: true,
                has_knife_near: true,
                count_bread_family: 0,
                plate_available: true,
            }),
            None
        );
    }

    #[test]
    fn mutton_prefers_hot_oven() {
        let mut tiles = empty_grid(5, 5, 8);
        tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 8, 5));
        let inp = DropHeldInput::basic(RAW_MUTTON, 5, 5, 0, 0);
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: RAW_MUTTON,
                    target: HOT_ADOBE_OVEN,
                    max_new_actor: 4,
                    ..
                }
            ),
            "got {d:?}"
        );
        let resolved = resolve_prefer_short_craft(d, &tiles, 5, 5);
        assert_eq!(
            resolved,
            DropHeldDecision::UseAt {
                x: 8,
                y: 5,
                target_id: HOT_ADOBE_OVEN,
                actor_id: RAW_MUTTON,
            }
        );
    }

    #[test]
    fn bowl_of_soil_prefers_dying_bush() {
        let mut tiles = empty_grid(0, 0, 10);
        tiles.push(ScanTile::simple(DYING_BUSH, 3, 0));
        let d = drop_held_object(DropHeldInput::basic(BOWL_OF_SOIL, 0, 0, 0, 0), &tiles);
        assert!(matches!(
            d,
            DropHeldDecision::PreferShortCraft {
                target: DYING_BUSH,
                ..
            }
        ));
    }

    #[test]
    fn forge_item_stages_near_forge() {
        let mut tiles = empty_grid(10, 10, 6);
        tiles.push(ScanTile::simple(FORGE, 10, 10));
        let mut inp = DropHeldInput::basic(289, 0, 0, 0, 0);
        inp.anchors.forge_x = Some(10);
        inp.anchors.forge_y = Some(10);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::Goto { x: 10, y: 10 })
                || matches!(d, DropHeldDecision::DropAt { .. }),
            "got {d:?}"
        );
    }

    #[test]
    fn banana_peel_drops_near_player() {
        let tiles = empty_grid(5, 5, 4);
        let d = drop_held_object(DropHeldInput::basic(BANANA_PEEL, 5, 5, 0, 0), &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x, y } if (x - 5).abs() <= 4 && (y - 5).abs() <= 4),
            "got {d:?}"
        );
    }

    #[test]
    fn dont_use_pile_for_wheat_sheaf() {
        assert!(pile_blocked(225, false));
        assert!(!pile_blocked(225, true));
        assert!(!pile_blocked(33, false));
        // Haxe L5301: Wheat Sheaf / Ear of Corn / Basket / Wet Clay Bowl / Yew / Straight / Curved
        assert_eq!(DONT_USE_PILE_IDS, &[225, 1113, 292, 233, 132, 64, 66]);
        for id in DONT_USE_PILE_IDS {
            assert!(pile_blocked(*id, false));
            assert!(!pile_blocked(*id, true));
        }
    }

    #[test]
    fn drop_on_start_false_for_peel_chip_list() {
        // Haxe L5303–5307
        assert!(force_drop_at_feet(BANANA_PEEL));
        assert!(force_drop_at_feet(SHARP_STONE));
        assert!(force_drop_at_feet(FLINT_CHIP));
        assert!(force_drop_at_feet(MILKWEED_STALK));
        assert!(force_drop_at_feet(FLAT_ROCK_RABBIT_BAIT));
        assert!(!force_drop_at_feet(STONE));
        assert_eq!(drop_clay_in_basket_debug_say(false), None);
        assert_eq!(
            drop_clay_in_basket_debug_say(true),
            Some("drop clay in basket")
        );
        assert_eq!(drop_held_goto_home_debug_say(false, true), None);
        assert_eq!(
            drop_held_goto_home_debug_say(true, true),
            Some("Goto home!")
        );
        assert_eq!(
            drop_held_goto_home_debug_say(true, false),
            Some("Cannot Goto home!")
        );
    }

    #[test]
    fn bones_basket_must_use_as_drop() {
        assert!(must_use_as_drop(BASKET_OF_BONES));
        assert!(must_use_as_drop(BASKET_OF_SOIL));
        assert!(!must_use_as_drop(STONE));
        // Haxe L5626: 356, 336, 1137, 186, 283, 241, 324, 323
        assert_eq!(
            DONT_USE_DROP_FOR_ITEMS,
            &[356, 336, 1137, 186, 283, 241, 324, 323]
        );
        for id in DONT_USE_DROP_FOR_ITEMS {
            assert!(must_use_as_drop(*id));
        }
    }

    #[test]
    fn last_new_target_skips_pile_for_empty() {
        // Haxe L5607–5609: lastNewTargetId == drop tile → empty r=maxSearch
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(33, 2, 0));
        let mut inp = DropHeldInput::basic(33, 0, 0, 0, 0);
        inp.pile_id = 33;
        inp.last_new_target_id = 33;
        inp.max_distance_to_home = 1.0; // drop close, search around player
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x, y } if !(x == 2 && y == 0)),
            "must not pile onto lastNewTarget, got {d:?}"
        );
    }

    #[test]
    fn oven_ids_match_baker_table() {
        assert!(should_drop_near_oven_held(CLAY_BOWL));
        assert!(should_drop_near_oven_held(RAW_MUTTON));
        assert!(should_drop_near_forge(HOT_IRON_BLOOM_TONGS));
        assert!(should_drop_near_fire(72));
        assert!(should_drop_near_well(BASKET_OF_SOIL));
    }

    #[test]
    fn drop_near_id_tables_match_haxe() {
        // Haxe: AiBase L5164 / L5173–5175 / L5181 / L5186 / L5189
        assert_eq!(
            DROP_NEAR_FIRE_IDS,
            &[72, 344, 180, 181, 185, 1147, 1148, 516, 540]
        );
        assert_eq!(
            DROP_NEAR_OVEN_IDS,
            &[
                235, 1603, 236, 1602, 252, 1470, 1471, 1285, 253, 518, 547, 548, 260, 4057, 502,
                569, 1354, 245, 258
            ]
        );
        assert_eq!(
            DROP_NEAR_FORGE_IDS,
            &[289, 290, 327, 326, 319, 320, 441, 568, 311, 308]
        );
        assert_eq!(DROP_NEAR_WELL_IDS, &[336, 227, 382]);
        assert_eq!(DROP_AT_CURRENT_POSITION_IDS, &[33, 34, 2144, 382, 210]);
        assert!(should_drop_at_current_position(BANANA_PEEL));
        assert!(should_drop_at_current_position(SHARP_STONE));
        assert!(should_drop_at_current_position(210));
        assert!(!should_drop_at_current_position(1137)); // Bowl of Soil in comment only
        assert!(!should_drop_at_current_position(0));
    }

    #[test]
    fn consider_drop_when_carrying_away_from_home() {
        assert!(consider_drop_held_object(72, 100, 100, 0, 0, 120, 120));
        assert!(!consider_drop_held_object(33, 100, 0, 0, 0, 10, 0));
        assert!(consider_drop_held_object(BANANA_PEEL, 50, 50, 0, 0, 60, 60));
        // Haxe L5198–5200: empty / going home refuse; Sharp Stone always dropHeld
        assert!(!consider_drop_held_object(34, 0, 0, 0, 0, 0, 0));
        assert!(!consider_drop_held_object(0, 50, 50, 0, 0, 60, 60));
        assert!(consider_drop_held_object(SHARP_STONE, 100, 0, 0, 0, 10, 0));
        // Haxe L5210–5223: fire / oven+pies / forge interrupt dropHeld
        assert!(consider_drop_held_object(BOWL_OF_DOUGH, 100, 100, 0, 0, 120, 120));
        assert!(consider_drop_held_object(289, 100, 100, 0, 0, 120, 120));
        // Haxe L5231: target closer to home than player → keep carrying
        assert!(!consider_drop_held_object(33, 10, 0, 0, 0, 3, 0));
        // 25-buffer: quad_target+25 not < quad_home → drop
        assert!(consider_drop_held_object(33, 6, 0, 0, 0, 5, 0));
    }

    #[test]
    fn use_up_dough_matches_haxe_budget() {
        // Haxe: held != 252; plate skip; knife 0/1; countBread>1 → 0
        assert_eq!(
            use_up_dough(UseUpDoughInput {
                held_id: 33,
                held_uses: 5,
                use_target_is_plate: false,
                has_knife_near: true,
                count_bread_family: 0,
                plate_available: true,
            }),
            None
        );
        let keep_last = use_up_dough(UseUpDoughInput {
            held_id: BOWL_OF_DOUGH,
            held_uses: 1,
            use_target_is_plate: false,
            has_knife_near: true,
            count_bread_family: 0,
            plate_available: true,
        });
        assert_eq!(keep_last, None);
        let no_plate = use_up_dough(UseUpDoughInput {
            held_id: BOWL_OF_DOUGH,
            held_uses: 3,
            use_target_is_plate: false,
            has_knife_near: false,
            count_bread_family: 0,
            plate_available: false,
        });
        assert_eq!(no_plate, None);
        let bread_full = use_up_dough(UseUpDoughInput {
            held_id: BOWL_OF_DOUGH,
            held_uses: 1,
            use_target_is_plate: false,
            has_knife_near: true,
            count_bread_family: 2,
            plate_available: true,
        });
        assert!(bread_full.is_some());
    }

    #[test]
    fn use_up_dough_sensors_home_radii() {
        // Knife at player r=20 does not count; knife at home r=30 does.
        let mut tiles = empty_grid(0, 0, 2);
        tiles.push(ScanTile::simple(KNIFE, 25, 0));
        let extras = DropHeldSensorExtras::default();
        let inp = drop_held_input_from_sensors(
            BOWL_OF_DOUGH,
            3,
            0,
            0,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &tiles,
            extras,
        );
        assert!(inp.has_knife_near);
        let mut far = empty_grid(0, 0, 2);
        far.push(ScanTile::simple(KNIFE, 31, 0));
        let inp2 = drop_held_input_from_sensors(
            BOWL_OF_DOUGH,
            3,
            0,
            0,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &far,
            DropHeldSensorExtras::default(),
        );
        assert!(!inp2.has_knife_near);
        let mut bread = empty_grid(0, 0, 2);
        bread.push(ScanTile::simple(SLICED_BREAD_ID, 20, 0));
        bread.push(ScanTile::simple(LEAVENED_DOUGH_PLATE, 0, 0));
        let inp3 = drop_held_input_from_sensors(
            BOWL_OF_DOUGH,
            3,
            0,
            0,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &bread,
            DropHeldSensorExtras::default(),
        );
        assert_eq!(inp3.count_bread_family, 2);
        let mut bread_far = empty_grid(0, 0, 2);
        bread_far.push(ScanTile::simple(SLICED_BREAD_ID, 21, 0));
        let inp4 = drop_held_input_from_sensors(
            BOWL_OF_DOUGH,
            3,
            0,
            0,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &bread_far,
            DropHeldSensorExtras::default(),
        );
        assert_eq!(inp4.count_bread_family, 0);
    }

    #[test]
    fn pile_id_cancels_when_matches_last_target() {
        // Haxe L5291: pileId == lastTargetId → pileId = -1
        assert_eq!(pile_id_after_last_craft_target(225, 225), -1);
        assert_eq!(pile_id_after_last_craft_target(225, 1113), 225);
        assert_eq!(pile_id_after_last_craft_target(-1, 225), -1);
        assert!(drop_held_clears_drop_target(0.0));
        assert!(drop_held_clears_drop_target(0.9));
        assert!(!drop_held_clears_drop_target(1.0));
        assert!(!drop_held_clears_drop_target(40.0));
    }

    #[test]
    fn is_dropping_item_head_gates() {
        // Haxe L8351–8391
        let mut tries = 3;
        assert_eq!(
            is_dropping_item_head(false, None, None, 33, 0, 0, 0, &mut tries),
            IsDropingItemHead::Idle
        );
        assert_eq!(tries, 0);

        tries = 2;
        assert_eq!(
            is_dropping_item_head(true, Some((292, 1, 0)), Some(292), 33, 0, 0, 0, &mut tries),
            IsDropingItemHead::Continue
        );
        assert_eq!(tries, 1); // reset then +1

        tries = 0;
        assert_eq!(
            is_dropping_item_head(false, Some((292, 1, 0)), Some(0), 33, 0, 0, 0, &mut tries),
            IsDropingItemHead::TargetGone
        );
        assert_eq!(tries, 0); // gone before increment

        tries = 0;
        assert_eq!(
            is_dropping_item_head(false, Some((292, 1, 0)), Some(292), 33, 4, 0, 0, &mut tries),
            IsDropingItemHead::DropHeld { max_distance: 10 }
        );
        assert_eq!(tries, 1);

        tries = 6;
        assert_eq!(
            is_dropping_item_head(false, Some((292, 1, 0)), Some(292), 33, 4, 0, 0, &mut tries),
            IsDropingItemHead::DropHeld { max_distance: 0 }
        );

        tries = 0;
        // (6,0) quad 36 > 25, held+ground
        assert_eq!(
            is_dropping_item_head(false, Some((34, 6, 0)), Some(34), 33, 0, 0, 0, &mut tries),
            IsDropingItemHead::DropHeld { max_distance: 10 }
        );
        tries = 0;
        // (4,0) quad 16 <= 25 → continue
        assert_eq!(
            is_dropping_item_head(false, Some((34, 4, 0)), Some(34), 33, 0, 0, 0, &mut tries),
            IsDropingItemHead::Continue
        );
        tries = 0;
        // empty hand never too-far
        assert_eq!(
            is_dropping_item_head(false, Some((34, 6, 0)), Some(34), 0, 0, 0, 0, &mut tries),
            IsDropingItemHead::Continue
        );
    }

    #[test]
    fn is_dropping_item_goto_follow_stack_wound_and_walk() {
        // Haxe L8401–8463
        assert_eq!(
            is_dropping_item_goto(34, 401.0, true, false, 10, false),
            IsDropingItemGoto::DropHeld { max_distance: 10 }
        );
        // hungry still follows the drop (no abort)
        assert_eq!(
            is_dropping_item_goto(34, 401.0, true, true, 10, false),
            IsDropingItemGoto::WalkToTarget
        );
        // empty ground (id 0) never follow-abort
        assert_eq!(
            is_dropping_item_goto(0, 401.0, true, false, 10, false),
            IsDropingItemGoto::WalkToTarget
        );
        // quad 400 is not > 400
        assert_eq!(
            is_dropping_item_goto(34, 400.0, true, false, 10, false),
            IsDropingItemGoto::WalkToTarget
        );
        // clay stacks convert even while moving / far
        assert_eq!(
            is_dropping_item_goto(STACK_OF_CLAY_PLATES, 100.0, false, false, 10, true),
            IsDropingItemGoto::ConvertToUse
        );
        assert_eq!(
            is_dropping_item_goto(STACK_OF_CLAY_BOWLS, 0.0, false, false, 10, false),
            IsDropingItemGoto::ConvertToUse
        );
        assert_eq!(
            is_dropping_item_goto(34, 4.0, false, false, 10, true),
            IsDropingItemGoto::WaitMoving
        );
        // orthogonal adjacent quad=1 is close (not > 1)
        assert_eq!(
            is_dropping_item_goto(34, 1.0, false, false, 10, false),
            IsDropingItemGoto::DropOnTarget
        );
        // diagonal quad=2 → goto
        assert_eq!(
            is_dropping_item_goto(34, 2.0, false, false, 10, false),
            IsDropingItemGoto::WalkToTarget
        );
        assert_eq!(
            is_dropping_item_goto(EXTRACTED_ARROWHEAD_WOUND, 0.0, false, false, 10, false),
            IsDropingItemGoto::UseOnTarget
        );
        assert_eq!(
            is_dropping_item_goto(0, 0.0, false, false, 10, false),
            IsDropingItemGoto::DropOnTarget
        );
        assert_eq!(is_dropping_drop_distance_after_try(1), 10);
        assert_eq!(is_dropping_drop_distance_after_try(6), 10);
        assert_eq!(is_dropping_drop_distance_after_try(7), 0);
    }

    #[test]
    fn consider_plate_use_target_skips_use_up_dough() {
        // Haxe UseUpDough: useTarget.parentId == 236 → false, then oven table drops
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(CLAY_PLATE, 2, 0));
        let d = consider_drop_held_decision_ex(
            BOWL_OF_DOUGH,
            3,
            0,
            0,
            0,
            0,
            50,
            50,
            &tiles,
            false,
            0,
            true,
        );
        assert_eq!(d, Some(DropHeldDecision::None));
    }

    #[test]
    fn to_live_intent_drop_and_use() {
        let d = DropHeldDecision::DropAt { x: 1, y: 2 };
        assert_eq!(
            d.to_live_intent(),
            ShortCraftLiveIntent::DropAt { x: 1, y: 2 }
        );
        let u = DropHeldDecision::UseAt {
            x: 3,
            y: 4,
            target_id: 250,
            actor_id: 569,
        };
        assert_eq!(
            u.to_live_intent(),
            ShortCraftLiveIntent::UseAt {
                x: 3,
                y: 4,
                target_id: 250,
                actor_id: 569,
            }
        );
        // Haxe: self(0,0,5) — live SelfClothing for Raw SELF enqueue
        assert_eq!(
            DropHeldDecision::SelfClothing { slot: 5 }.to_live_intent(),
            ShortCraftLiveIntent::SelfClothing { slot: 5 }
        );
        assert_eq!(self_clothing_raw_payload(5), "0 0 5");
        // Haxe: gotoObj — not DropAt
        assert_eq!(
            DropHeldDecision::Goto { x: 9, y: 8 }.to_live_intent(),
            ShortCraftLiveIntent::Goto { x: 9, y: 8 }
        );
    }

    #[test]
    fn gooseberry_fills_bowl() {
        let mut tiles = empty_grid(0, 0, 5);
        tiles.push(
            ScanTile::simple(BOWL_OF_GOOSEBERRIES, 2, 0)
                .with_uses(1)
                .with_num_uses(5),
        );
        let mut inp = DropHeldInput::basic(GOOSEBERRY, 0, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 2,
                y: 0,
                target_id: BOWL_OF_GOOSEBERRIES,
                actor_id: GOOSEBERRY,
            }
        );
    }

    #[test]
    fn gooseberry_skips_bowl_when_max_dist_low() {
        // Haxe: 31 && maxDistanceToHome > 5 — near-home gooseberry does not fill
        let mut tiles = empty_grid(0, 0, 5);
        tiles.push(
            ScanTile::simple(BOWL_OF_GOOSEBERRIES, 2, 0)
                .with_uses(1)
                .with_num_uses(5),
        );
        let mut inp = DropHeldInput::basic(GOOSEBERRY, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            !matches!(
                d,
                DropHeldDecision::UseAt {
                    target_id: BOWL_OF_GOOSEBERRIES,
                    ..
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn dry_bean_pod_fills_even_when_near_home() {
        // Haxe precedence: dry bean 1160 always tries bowl; gooseberry needs maxDist>5
        let mut tiles = empty_grid(0, 0, 5);
        tiles.push(
            ScanTile::simple(BOWL_OF_DRY_BEANS, 1, 0)
                .with_uses(1)
                .with_num_uses(5),
        );
        let mut inp = DropHeldInput::basic(DRY_BEAN_POD, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0; // near-home still fills for beans
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 1,
                y: 0,
                target_id: BOWL_OF_DRY_BEANS,
                actor_id: DRY_BEAN_POD,
            }
        );
    }

    #[test]
    fn gooseberry_skips_full_bowl_uses_second() {
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(
            ScanTile::simple(BOWL_OF_GOOSEBERRIES, 1, 0)
                .with_uses(5)
                .with_num_uses(5),
        );
        tiles.push(
            ScanTile::simple(BOWL_OF_GOOSEBERRIES, 4, 0)
                .with_uses(2)
                .with_num_uses(5),
        );
        let mut inp = DropHeldInput::basic(GOOSEBERRY, 0, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 4,
                y: 0,
                target_id: BOWL_OF_GOOSEBERRIES,
                actor_id: GOOSEBERRY,
            }
        );
    }

    #[test]
    fn fill_anchors_finds_forge_well_and_kiln() {
        let mut tiles = empty_grid(0, 0, 5);
        tiles.push(ScanTile::simple(FORGE, 4, 0));
        tiles.push(ScanTile::simple(663, 0, 4));
        tiles.push(ScanTile::simple(283, 3, 3)); // kiln family
        let a = fill_anchors_from_scan(DropHeldAnchors::home_only(0, 0), &tiles);
        assert_eq!(a.forge_xy(), Some((4, 0)));
        assert_eq!(a.well_xy(), Some((0, 4)));
        assert_eq!(a.kiln_xy(), Some((3, 3)));
    }

    #[test]
    fn sliced_bread_const_aligns() {
        assert_eq!(SLICED_BREAD_ID, 1471);
    }

    #[test]
    fn table_food_container_empty_search_yields_use_as_drop() {
        // Haxe: only ShouldDropOnTable / isSmallFoodToStore may drop into free containers.
        let basket = ScanTile::simple(BASKET, 2, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let tiles = vec![basket];
        let mut inp = DropHeldInput::basic(COOKED_MUTTON, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 2,
                    y: 0,
                    target_id: BASKET,
                    actor_id: COOKED_MUTTON,
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn non_table_food_skips_free_container() {
        // Stone must not USE-drop into basket (Haxe L195 gate).
        let basket = ScanTile::simple(BASKET, 2, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let tiles = vec![basket];
        let mut inp = DropHeldInput::basic(STONE, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x: 0, y: 0 }),
            "stone should fall back to feet, got {d:?}"
        );
    }

    #[test]
    fn clay_prefers_basket_containing_clay() {
        let mut tiles = empty_grid(0, 0, 12);
        // Empty basket closer
        tiles.push(ScanTile::simple(BASKET, 2, 0).with_num_slots(4));
        // Clay basket farther
        tiles.push(
            ScanTile::simple(BASKET, 5, 0)
                .with_num_slots(4)
                .with_contains(CLAY)
                .with_contained_count(1),
        );
        let mut inp = DropHeldInput::basic(CLAY, 0, 0, 0, 0);
        inp.anchors.kiln_x = Some(100);
        inp.anchors.kiln_y = Some(100); // far kiln → dist_q > 400
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 5,
                y: 0,
                target_id: BASKET,
                actor_id: CLAY,
            },
            "should prefer clay basket over closer empty"
        );
    }

    #[test]
    fn clay_far_from_home_no_kiln_uses_empty_basket() {
        // Haxe: no kiln → distance is to home; >400 → any basket r=10
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(BASKET, 3, 0).with_num_slots(4));
        let mut inp = DropHeldInput::basic(CLAY, 0, 0, 30, 0); // home 30,0 → dist_q=900
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 3,
                y: 0,
                target_id: BASKET,
                actor_id: CLAY,
            },
            "got {d:?}"
        );
    }

    #[test]
    fn clay_far_kiln_empty_basket_fallback() {
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(BASKET, 2, 0).with_num_slots(4));
        let mut inp = DropHeldInput::basic(CLAY, 0, 0, 0, 0);
        inp.anchors.kiln_x = Some(100);
        inp.anchors.kiln_y = Some(0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 2,
                y: 0,
                target_id: BASKET,
                actor_id: CLAY,
            },
            "got {d:?}"
        );
    }

    #[test]
    fn bones_retarget_graveyard() {
        let tiles = empty_grid(0, 0, 6);
        let mut inp = DropHeldInput::basic(BASKET_OF_BONES, 0, 0, 0, 0);
        inp.anchors.graveyard_x = Some(20);
        inp.anchors.graveyard_y = Some(0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::Goto { x: 20, y: 0 }
                    | DropHeldDecision::DropAt { .. }
                    | DropHeldDecision::UseAsDrop { .. }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn mutton_falls_back_to_hot_coals() {
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(HOT_COALS, 4, 0));
        let d = drop_held_object(DropHeldInput::basic(RAW_MUTTON, 0, 0, 0, 0), &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: RAW_MUTTON,
                    target: HOT_COALS,
                    max_search: 10,
                    craft_actor: false,
                    max_new_actor: 4,
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn steel_hoe_uses_craft_actor_false() {
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(SHALLOW_TILLED_ROW, 3, 0));
        let mut inp = DropHeldInput::basic(STEEL_HOE, 0, 0, 0, 0);
        inp.food_store = 5.0;
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: STEEL_HOE,
                    target: SHALLOW_TILLED_ROW,
                    max_search: 15,
                    craft_actor: false,
                    ..
                }
            ),
            "got {d:?}"
        );
        inp.food_store = 2.0;
        let d2 = drop_held_object(inp, &tiles);
        assert!(
            !matches!(
                d2,
                DropHeldDecision::PreferShortCraft {
                    actor: STEEL_HOE,
                    ..
                }
            ),
            "food_store > 2 required, got {d2:?}"
        );
    }

    #[test]
    fn count_close_objects_half_open_and_pile_uses() {
        // Haxe CountCloseObjects r=3: [tx-3, tx+3) — tile at +3 excluded
        let mut tiles = empty_grid(0, 0, 1);
        tiles.push(ScanTile::simple(STONE, 3, 0));
        assert_eq!(
            count_close_objects(&tiles, 0, 0, STONE, 3, false, 0),
            0
        );
        tiles.push(ScanTile::simple(STONE, 2, 0));
        assert_eq!(
            count_close_objects(&tiles, 0, 0, STONE, 3, false, 0),
            1
        );
        let mut piled = empty_grid(0, 0, 1);
        piled.push(ScanTile::simple(4100, 1, 0).with_uses(5));
        assert_eq!(
            count_close_objects(&piled, 0, 0, STONE, 3, true, 4100),
            5
        );
    }

    #[test]
    fn hot_iron_bloom_tongs_on_flat_rock() {
        // Haxe L5401–5402: shortCraft(308, 291, 10, false)
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(FLAT_ROCK, 4, 0));
        let d = drop_held_object(
            DropHeldInput::basic(HOT_IRON_BLOOM_TONGS, 0, 0, 0, 0),
            &tiles,
        );
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: HOT_IRON_BLOOM_TONGS,
                    target: FLAT_ROCK,
                    max_search: 10,
                    craft_actor: false,
                    ..
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn stone_at_forge_counts_piles_by_uses() {
        // 5-use stone pile at forge fills maxItems=1 → do not retarget forge
        let mut tiles = empty_grid(0, 0, 4);
        tiles.push(ScanTile::simple(FORGE, 0, 0));
        tiles.push(ScanTile::simple(9991, 1, 0).with_uses(5));
        let mut inp = DropHeldInput::basic(STONE, 10, 0, 0, 0);
        inp.anchors.forge_x = Some(0);
        inp.anchors.forge_y = Some(0);
        inp.pile_id = 9991;
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            !matches!(d, DropHeldDecision::Goto { x: 0, y: 0 }),
            "full pile should not stage more stone at forge, got {d:?}"
        );
    }

    #[test]
    fn bowl_of_wheat_plants_when_count_under_10() {
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(DEEP_TILLED_ROW, 3, 0));
        tiles.push(ScanTile::simple(RIPE_WHEAT, 1, 0));
        let d = drop_held_object(DropHeldInput::basic(BOWL_OF_WHEAT, 0, 0, 0, 0), &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: BOWL_OF_WHEAT,
                    target: DEEP_TILLED_ROW,
                    max_search: 20,
                    craft_actor: false,
                    ..
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn shovel_of_dung_on_wet_compost() {
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(WET_COMPOST, 5, 0));
        let mut inp = DropHeldInput::basic(SHOVEL_OF_DUNG, 0, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: SHOVEL_OF_DUNG,
                    target: WET_COMPOST,
                    max_search: 20,
                    craft_actor: false,
                    ..
                }
            ),
            "got {d:?}"
        );
        inp.max_distance_to_home = 1.0;
        let d2 = drop_held_object(inp, &tiles);
        assert!(
            !matches!(
                d2,
                DropHeldDecision::PreferShortCraft {
                    actor: SHOVEL_OF_DUNG,
                    ..
                }
            ),
            "maxDist>5 required, got {d2:?}"
        );
    }

    #[test]
    fn clay_bowl_overflow_stages_at_forge() {
        let mut tiles = empty_grid(0, 0, 6);
        for i in 0..3 {
            tiles.push(ScanTile::simple(CLAY_BOWL, i, 0));
        }
        tiles.push(ScanTile::simple(FORGE, 8, 0));
        let mut inp = DropHeldInput::basic(CLAY_BOWL, 0, 0, 0, 0);
        inp.anchors.forge_x = Some(8);
        inp.anchors.forge_y = Some(0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::Goto { x: 8, y: 0 }
                    | DropHeldDecision::DropAt { .. }
                    | DropHeldDecision::BusyMoving
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn shovel_near_home_drops_close() {
        // Haxe L5482–5485: shovel 502 quadDistance <= 225 → dropCloseToPlayer
        let tiles = empty_grid(10, 0, 4);
        let mut inp = DropHeldInput::basic(SHOVEL, 10, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x, y } if (x - 10).abs() <= 4 && y.abs() <= 4),
            "got {d:?}"
        );
    }

    #[test]
    fn pile_skips_full_num_uses() {
        // Pile at capacity must be skipped (not uses>=10 heuristic).
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(
            ScanTile::simple(9999, 1, 0) // fake pile id
                .with_uses(3)
                .with_num_uses(3),
        );
        tiles.push(ScanTile::simple(9999, 3, 0).with_uses(1).with_num_uses(3));
        let mut inp = DropHeldInput::basic(33, 0, 0, 0, 0);
        inp.pile_id = 9999;
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 3,
                    y: 0,
                    target_id: 9999,
                    ..
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn store_arrow_filled_quiver_refuses() {
        // Haxe canAddToQuiver: numUses>=2 && numberOfUses >= numUses → false
        let q = QuiverClothing::from_ids_with_uses(&[ARROW_QUIVER_ID], 5, 5);
        assert!(!q.can_add);
        assert_eq!(store_in_quiver(ARROW, q), None);
        let open = QuiverClothing::from_ids_with_uses(&[ARROW_QUIVER_ID], 2, 5);
        assert!(open.can_add);
        assert!(store_in_quiver(ARROW, open).is_some());
    }

    #[test]
    fn skewered_rabbit_prefers_hot_coals() {
        let mut tiles = empty_grid(0, 0, 10);
        tiles.push(ScanTile::simple(HOT_COALS, 4, 0));
        let d = drop_held_object(DropHeldInput::basic(SKEWERED_RABBIT, 0, 0, 0, 0), &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: SKEWERED_RABBIT,
                    target: HOT_COALS,
                    max_search: 20,
                    ..
                }
            ),
            "got {d:?}"
        );
        let consider = consider_drop_held_decision(SKEWERED_RABBIT, 0, 0, 0, 0, 50, 50, &tiles);
        assert!(
            matches!(
                consider,
                Some(DropHeldDecision::PreferShortCraft {
                    target: HOT_COALS,
                    ..
                })
            ),
            "got {consider:?}"
        );
    }

    #[test]
    fn flat_rock_near_forge_min_distance_negative() {
        // With mindistance=-1, search rings start at 3 and allow close tiles.
        let mut tiles = empty_grid(10, 10, 4);
        tiles.push(ScanTile::simple(FORGE, 10, 10));
        let mut inp = DropHeldInput::basic(FLAT_ROCK, 10, 10, 0, 0);
        inp.anchors.forge_x = Some(10);
        inp.anchors.forge_y = Some(10);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        // Close enough to forge: drop near forge tile (not far goto only)
        assert!(
            matches!(
                d,
                DropHeldDecision::DropAt { x, y }
                    if (x - 10).abs() <= 4 && (y - 10).abs() <= 4
            ) || matches!(
                d,
                DropHeldDecision::BusyMoving | DropHeldDecision::Goto { .. }
            ),
            "got {d:?}"
        );
        // When player is ON forge and not moving, should place on adjacent empty
        if let DropHeldDecision::DropAt { x, y } = d {
            assert!((x - 10).abs() <= 3 && (y - 10).abs() <= 3);
        }
    }

    #[test]
    fn can_add_to_quiver_matches_haxe() {
        assert!(can_add_to_quiver(0, 0)); // numUses < 2
        assert!(can_add_to_quiver(1, 1)); // numUses < 2
        assert!(can_add_to_quiver(3, 5)); // uses < numUses
        assert!(!can_add_to_quiver(5, 5));
        assert!(!can_add_to_quiver(6, 5));
    }

    #[test]
    fn consider_use_up_dough_before_oven_table() {
        // Dough with plate + extra uses interrupts goto even though 252 is oven-near.
        let mut tiles = empty_grid(0, 0, 8);
        tiles.push(ScanTile::simple(CLAY_PLATE, 2, 0));
        let d =
            consider_drop_held_decision_ex(
                BOWL_OF_DOUGH,
                3,
                0,
                0,
                0,
                0,
                50,
                50,
                &tiles,
                false,
                0,
                false,
            );
        assert!(
            matches!(
                d,
                Some(DropHeldDecision::PreferShortCraft {
                    actor: BOWL_OF_DOUGH,
                    target: CLAY_PLATE,
                    ..
                })
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn plan_drop_held_live_resolves_mutton_oven() {
        let mut tiles = empty_grid(5, 5, 8);
        tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 8, 5));
        let inp = DropHeldInput::basic(RAW_MUTTON, 5, 5, 0, 0);
        let d = plan_drop_held_live(inp, &tiles);
        assert_eq!(
            d,
            DropHeldDecision::UseAt {
                x: 8,
                y: 5,
                target_id: HOT_ADOBE_OVEN,
                actor_id: RAW_MUTTON,
            }
        );
        assert_eq!(
            smart_drop_held_to_live_intent(inp, &tiles),
            ShortCraftLiveIntent::UseAt {
                x: 8,
                y: 5,
                target_id: HOT_ADOBE_OVEN,
                actor_id: RAW_MUTTON,
            }
        );
    }

    #[test]
    fn smart_drop_clay_far_kiln_basket_or_goto() {
        // Clay far from kiln → basket with clay or goto kiln (not feet-only DropAt).
        let mut tiles = empty_grid(0, 0, 12);
        tiles.push(ScanTile::simple(283, 100, 100)); // kiln far
        tiles.push(
            ScanTile::simple(BASKET, 3, 0)
                .with_num_slots(4)
                .with_contains(CLAY)
                .with_contained_count(1),
        );
        let intent = smart_drop_held_from_sensors(
            CLAY,
            1,
            0,
            0,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &tiles,
            DropHeldSensorExtras::default(),
        );
        assert!(
            matches!(
                intent,
                ShortCraftLiveIntent::UseAt {
                    target_id: BASKET,
                    actor_id: CLAY,
                    ..
                } | ShortCraftLiveIntent::Goto { .. }
                    | ShortCraftLiveIntent::DropAt { .. }
            ),
            "got {intent:?}"
        );
        // Prefer basket containing clay when far from kiln
        if let ShortCraftLiveIntent::UseAt {
            target_id,
            actor_id,
            x,
            y,
            ..
        } = intent
        {
            assert_eq!((target_id, actor_id, x, y), (BASKET, CLAY, 3, 0));
        }
    }

    #[test]
    fn self_clothing_to_live_and_payload() {
        let q = QuiverClothing::from_ids(&[EMPTY_ARROW_QUIVER_ID]);
        let mut extras = DropHeldSensorExtras::default();
        extras.quiver = q;
        let tiles = empty_grid(0, 0, 2);
        let intent = smart_drop_held_from_sensors(
            YEW_BOW, 1, 0, 0, 0, 0, 20.0, false, false, 40.0, &tiles, extras,
        );
        assert_eq!(
            intent,
            ShortCraftLiveIntent::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            }
        );
    }

    #[test]
    fn force_drop_at_feet_banana_live() {
        let tiles = empty_grid(5, 5, 4);
        let intent = smart_drop_held_from_sensors(
            BANANA_PEEL,
            1,
            5,
            5,
            0,
            0,
            20.0,
            false,
            false,
            40.0,
            &tiles,
            DropHeldSensorExtras::default(),
        );
        assert!(
            matches!(
                intent,
                ShortCraftLiveIntent::DropAt { x, y }
                    if (x - 5).abs() <= 4 && (y - 5).abs() <= 4
            ),
            "got {intent:?}"
        );
    }

    // ── PREFER-SHORT-WAIT ───────────────────────────────────────────────────

    #[test]
    fn busy_moving_to_wait_live_intent() {
        // Haxe: dropOnStart && isMoving → return true (hold tick)
        assert_eq!(
            DropHeldDecision::BusyMoving.to_live_intent(),
            ShortCraftLiveIntent::Wait
        );
    }

    #[test]
    fn drop_on_start_while_moving_is_busy_wait() {
        // Oven-near held stages dropOnStart; while moving → BusyMoving → Wait
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 0, 0)); // home/oven
        let mut inp = DropHeldInput::basic(CLAY_BOWL, 20, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        inp.is_moving = true;
        // Clay bowl is oven-near → dropClose=false → dropOnStart → BusyMoving
        let d = drop_held_object(inp, &tiles);
        assert_eq!(d, DropHeldDecision::BusyMoving, "got {d:?}");
        assert_eq!(
            plan_drop_held_live(inp, &tiles).to_live_intent(),
            ShortCraftLiveIntent::Wait
        );
        assert_eq!(
            smart_drop_held_to_live_intent(inp, &tiles),
            ShortCraftLiveIntent::Wait
        );
    }

    #[test]
    fn clay_plate_under_five_drops_empty_then_switch() {
        // Haxe L5501–5514: count<5 pileId=0; empty r=5 then switch -10 r=20; dropIsAUse=false
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(CLAY_PLATE_ID, 1, 0));
        let mut inp = DropHeldInput::basic(CLAY_PLATE_ID, 0, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x, y } if x.abs() <= 5 && y.abs() <= 5 && !(x == 0 && y == 0)),
            "got {d:?}"
        );
        // No empties in r=5 → switch non-permanent other id r=20
        let mut only_sw = vec![
            ScanTile::simple(CLAY_PLATE_ID, 0, 0),
            ScanTile::simple(33, 8, 0),
        ];
        only_sw[1].is_permanent = false;
        let d2 = drop_held_object(inp, &only_sw);
        assert_eq!(
            d2,
            DropHeldDecision::DropAt { x: 8, y: 0 },
            "got {d2:?}"
        );
    }

    #[test]
    fn drop_on_start_goto_when_quad_between_400_and_max() {
        // Haxe L5544: quad > 400 && quad < maxDist² → gotoObj(target)
        let tiles = empty_grid(0, 0, 2);
        let mut inp = DropHeldInput::basic(CLAY_BOWL, 21, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(d, DropHeldDecision::Goto { x: 0, y: 0 }, "got {d:?}");
        inp.max_distance_to_home = 20.0; // max_q=400; 441 > 400 → dropCloseToPlayer
        let near_player = empty_grid(21, 0, 2);
        let d2 = drop_held_object(inp, &near_player);
        assert!(
            matches!(d2, DropHeldDecision::DropAt { x, y } if (x - 21).abs() <= 2 && y.abs() <= 2),
            "too far → drop close, got {d2:?}"
        );
    }

    #[test]
    fn fire_item_clears_drop_close() {
        // Kindling 72: dropClose=false → dropOnStart can Goto fire
        let mut tiles = empty_grid(0, 0, 2);
        tiles.push(ScanTile::simple(85, 5, 0)); // coals as firePlace
        let mut inp = DropHeldInput::basic(72, 26, 0, 0, 0);
        inp.anchors.fire_x = Some(5);
        inp.anchors.fire_y = Some(0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(d, DropHeldDecision::Goto { x: 5, y: 0 }, "got {d:?}");
    }

    #[test]
    fn well_item_stages_at_well_or_home() {
        let mut tiles = empty_grid(0, 0, 2);
        tiles.push(ScanTile::simple(663, 8, 0));
        let mut inp = DropHeldInput::basic(BASKET_OF_SOIL, 30, 0, 0, 0);
        inp.anchors.well_x = Some(8);
        inp.anchors.well_y = Some(0);
        inp.max_distance_to_home = 40.0;
        let d = drop_held_object(inp, &tiles);
        assert_eq!(d, DropHeldDecision::Goto { x: 8, y: 0 }, "got {d:?}");
    }

    #[test]
    fn prefer_short_craft_uses_craft_actor_flag() {
        // Unresolved PreferShortCraft keeps craft_actor as SeekOrCraft craft_if_needed
        let d = DropHeldDecision::PreferShortCraft {
            actor: STONE_HOE,
            target: BASKET,
            max_search: 15,
            craft_actor: true,
            max_new_actor: i32::MAX,
        };
        assert_eq!(
            d.to_live_intent(),
            ShortCraftLiveIntent::SeekOrCraft {
                actor: STONE_HOE,
                craft_if_needed: true,
            }
        );
        let d2 = DropHeldDecision::PreferShortCraft {
            actor: RAW_MUTTON,
            target: HOT_ADOBE_OVEN,
            max_search: 10,
            craft_actor: false,
            max_new_actor: 4,
        };
        assert_eq!(
            d2.to_live_intent(),
            ShortCraftLiveIntent::SeekOrCraft {
                actor: RAW_MUTTON,
                craft_if_needed: false,
            }
        );
    }

    #[test]
    fn plan_resolves_prefer_short_before_live() {
        // plan_drop_held_live must not leave PreferShortCraft when target in scan
        let mut tiles = empty_grid(5, 5, 8);
        tiles.push(ScanTile::simple(HOT_COALS, 8, 5));
        let inp = DropHeldInput::basic(SKEWERED_RABBIT, 5, 5, 0, 0);
        let d = plan_drop_held_live(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAt {
                    target_id: HOT_COALS,
                    actor_id: SKEWERED_RABBIT,
                    ..
                }
            ),
            "got {d:?}"
        );
        assert!(matches!(
            d.to_live_intent(),
            ShortCraftLiveIntent::UseAt {
                target_id: HOT_COALS,
                actor_id: SKEWERED_RABBIT,
                ..
            }
        ));
    }

    // ── DROP-HELD-TABLE ─────────────────────────────────────────────────────

    #[test]
    fn should_drop_on_table_and_small_food_helpers() {
        assert!(should_drop_on_table(OMELETTE));
        assert!(!should_drop_on_table(COOKED_MUTTON));
        assert!(is_baked_pie(272));
        assert!(is_baked_pie(803));
        assert!(is_small_food_to_store(COOKED_MUTTON));
        assert!(is_small_food_to_store(272));
        assert!(!is_small_food_to_store(STONE));
        assert!(allows_drop_in_container(OMELETTE));
        assert!(allows_drop_in_container(COOKED_MUTTON));
        assert!(!allows_drop_in_container(STONE));
    }

    #[test]
    fn omelette_prefers_table_over_closer_empty() {
        // Table at (3,0): quad=9, factor=0.25 → (9-4)*0.25=1.25 beats empty at (2,0)=4
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(
            ScanTile::simple(TABLE, 3, 0)
                .with_num_slots(4)
                .with_contained_count(0),
        );
        let mut inp = DropHeldInput::basic(OMELETTE, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 3,
                    y: 0,
                    target_id: TABLE,
                    actor_id: OMELETTE,
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn small_food_prefers_slot_box_over_basket() {
        // Haxe factor: same cheb — box 0.25 vs basket 0.5; score (16-4)*f → box 3 < basket 6.
        // No empty-grid tiles so free-ground factor 1.0 cannot steal the pick.
        let tiles = vec![
            ScanTile::simple(BASKET, 4, 0)
                .with_num_slots(4)
                .with_contained_count(0),
            ScanTile::simple(WOODEN_SLOT_BOX, 0, 4)
                .with_num_slots(8)
                .with_contained_count(0),
        ];
        let mut inp = DropHeldInput::basic(COOKED_MUTTON, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    target_id: WOODEN_SLOT_BOX,
                    actor_id: COOKED_MUTTON,
                    ..
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn pie_prefers_container_already_holding_pie() {
        // Same cheb: empty basket factor 0.5 → score 6; pie-basket ×0.5 → 0.25 → score 3.
        let tiles = vec![
            ScanTile::simple(BASKET, 4, 0)
                .with_num_slots(4)
                .with_contained_count(0),
            ScanTile::simple(BASKET, 0, 4)
                .with_num_slots(4)
                .with_contains(272)
                .with_contained_count(1),
        ];
        let mut inp = DropHeldInput::basic(803, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 0,
                    y: 4,
                    target_id: BASKET,
                    actor_id: 803,
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn container_prefer_factor_table_and_box() {
        let table = ScanTile::simple(TABLE, 0, 0)
            .with_num_slots(2)
            .with_contained_count(0);
        let box_ = ScanTile::simple(WOODEN_SLOT_BOX, 0, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let bask = ScanTile::simple(BASKET, 0, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        assert_eq!(container_prefer_factor(OMELETTE, &table), Some(0.25));
        assert_eq!(container_prefer_factor(OMELETTE, &bask), Some(1.0));
        assert_eq!(container_prefer_factor(COOKED_MUTTON, &box_), Some(0.25));
        assert_eq!(container_prefer_factor(COOKED_MUTTON, &bask), Some(0.5));
        assert_eq!(container_prefer_factor(STONE, &bask), None);
    }

    #[test]
    fn quiver_from_clothing_snapshot_ids() {
        let ids = [0, 0, 0, 0, 0, EMPTY_ARROW_QUIVER_ID];
        let uses = [0, 0, 0, 0, 0, 0];
        let q = quiver_from_clothing_snapshot(&ids, &uses);
        assert!(q.empty_quiver);
        assert!(q.can_add);
        assert_eq!(
            store_in_quiver(YEW_BOW, q),
            Some(DropHeldDecision::SelfClothing {
                slot: QUIVER_CLOTHING_SLOT
            })
        );
        assert_eq!(clothing_ids_snapshot(&[1, 2, 3]), [1, 2, 3, 0, 0, 0]);
    }

    fn mutton_cook_db() -> ol_content::ContentDb {
        use ol_content::{ContentDb, Transition};
        let mut db = ContentDb::default();
        db.transitions.insert(
            (RAW_MUTTON, HOT_ADOBE_OVEN),
            Transition {
                actor_id: RAW_MUTTON,
                target_id: HOT_ADOBE_OVEN,
                new_actor_id: COOKED_MUTTON,
                new_target_id: HOT_ADOBE_OVEN,
                ..Default::default()
            },
        );
        db.transitions.insert(
            (RAW_MUTTON, HOT_COALS),
            Transition {
                actor_id: RAW_MUTTON,
                target_id: HOT_COALS,
                new_actor_id: COOKED_MUTTON,
                new_target_id: HOT_COALS,
                ..Default::default()
            },
        );
        db
    }

    #[test]
    fn max_new_actor_refuses_at_cap() {
        assert!(!max_new_actor_refuses(-1, 99));
        assert!(!max_new_actor_refuses(0, 99));
        assert!(!max_new_actor_refuses(4, 3));
        assert!(max_new_actor_refuses(4, 4));
        assert!(max_new_actor_refuses(4, 5));
    }

    #[test]
    fn count_short_craft_new_actor_radius_and_held() {
        let mut tiles = empty_grid(0, 0, 32);
        tiles.push(ScanTile::simple(COOKED_MUTTON, 1, 0));
        tiles.push(ScanTile::simple(COOKED_MUTTON, 2, 0));
        tiles.push(ScanTile::simple(COOKED_MUTTON, 31, 0)); // Chebyshev 31 > 30
        assert_eq!(
            count_short_craft_new_actor(&tiles, 0, 0, RAW_MUTTON, COOKED_MUTTON),
            2
        );
        assert_eq!(
            count_short_craft_new_actor(&tiles, 0, 0, COOKED_MUTTON, COOKED_MUTTON),
            3
        );
        assert_eq!(count_short_craft_new_actor(&tiles, 0, 0, RAW_MUTTON, 0), 0);
        let db = mutton_cook_db();
        assert_eq!(
            short_craft_transition_new_actor(&db, RAW_MUTTON, HOT_ADOBE_OVEN),
            Some(COOKED_MUTTON)
        );
        assert_eq!(short_craft_transition_new_actor(&db, RAW_MUTTON, 9999), None);
    }

    #[test]
    fn mutton_oven_max_new_actor_counts_transition_new_actor() {
        // Haxe: CountCloseObjects(trans.newActorID=570, 30) >= 4 → shortCraft false
        let db = mutton_cook_db();
        let mut tiles = empty_grid(5, 5, 8);
        tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 8, 5));
        for i in 0..4 {
            tiles.push(ScanTile::simple(COOKED_MUTTON, 5 + i, 6));
        }
        let inp = DropHeldInput::basic(RAW_MUTTON, 5, 5, 0, 0);
        let d = drop_held_object_ex(inp, &tiles, Some(&db));
        assert!(
            !matches!(
                d,
                DropHeldDecision::PreferShortCraft {
                    actor: RAW_MUTTON,
                    target: HOT_ADOBE_OVEN,
                    ..
                }
            ),
            "cap must skip PreferShortCraft, got {d:?}"
        );
        let resolved = plan_drop_held_live_ex(inp, &tiles, Some(&db));
        assert!(
            !matches!(
                resolved,
                DropHeldDecision::UseAt {
                    actor_id: RAW_MUTTON,
                    target_id: HOT_ADOBE_OVEN,
                    ..
                }
            ),
            "cap must not USE oven, got {resolved:?}"
        );

        let mut under = empty_grid(5, 5, 8);
        under.push(ScanTile::simple(HOT_ADOBE_OVEN, 8, 5));
        for i in 0..3 {
            under.push(ScanTile::simple(COOKED_MUTTON, 5 + i, 6));
        }
        let d2 = drop_held_object_ex(inp, &under, Some(&db));
        assert!(
            matches!(
                d2,
                DropHeldDecision::PreferShortCraft {
                    actor: RAW_MUTTON,
                    target: HOT_ADOBE_OVEN,
                    max_new_actor: 4,
                    ..
                }
            ),
            "under cap must PreferShortCraft, got {d2:?}"
        );

        let d3 = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d3,
                DropHeldDecision::PreferShortCraft {
                    target: HOT_ADOBE_OVEN,
                    ..
                }
            ),
            "no content skips gate, got {d3:?}"
        );

        let mut raw_tiles = empty_grid(5, 5, 8);
        raw_tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 8, 5));
        for i in 0..4 {
            raw_tiles.push(ScanTile::simple(RAW_MUTTON, 5 + i, 6));
        }
        let d4 = drop_held_object_ex(inp, &raw_tiles, Some(&db));
        assert!(
            matches!(
                d4,
                DropHeldDecision::PreferShortCraft {
                    target: HOT_ADOBE_OVEN,
                    ..
                }
            ),
            "count trans.newActor (cooked) not actor (raw), got {d4:?}"
        );
    }

    #[test]
    fn close_use_quad_distance_default_is_400_not_25() {
        // Haxe: AiBase L33 closeUseQuadDistance = 400 (L505 distance<25 is a different gate)
        assert_eq!(CLOSE_USE_QUAD_DISTANCE, 400);
        assert_eq!(
            DropHeldInput::basic(33, 0, 0, 0, 0).close_use_quad_distance,
            400
        );
        assert_eq!(
            DropHeldSensorExtras::default().close_use_quad_distance,
            400
        );
    }

    #[test]
    fn hungry_far_pile_refuses_beyond_close_use_quad_distance() {
        // Only the pile tile — no closer empties. (16,0) quad=256 < 400; (21,0) 441 > 400.
        let near = vec![ScanTile::simple(33, 16, 0)];
        let mut inp = DropHeldInput::basic(33, 0, 0, 0, 0);
        inp.has_food_target = true;
        inp.pile_id = 33;
        let d_near = drop_held_object(inp, &near);
        assert!(
            matches!(
                d_near,
                DropHeldDecision::UseAsDrop {
                    x: 16,
                    y: 0,
                    target_id: 33,
                    actor_id: 33,
                }
            ),
            "256 < 400 must keep pile, got {d_near:?}"
        );

        let far = vec![ScanTile::simple(33, 21, 0)];
        let d_far = drop_held_object(inp, &far);
        assert!(
            !matches!(
                d_far,
                DropHeldDecision::UseAsDrop {
                    x: 21,
                    y: 0,
                    ..
                }
            ),
            "441 > 400 must refuse far pile, got {d_far:?}"
        );

        inp.close_use_quad_distance = 25;
        let d_old = drop_held_object(inp, &near);
        assert!(
            !matches!(
                d_old,
                DropHeldDecision::UseAsDrop {
                    x: 16,
                    y: 0,
                    ..
                }
            ),
            "legacy 25 would refuse 256, got {d_old:?}"
        );
    }
}
