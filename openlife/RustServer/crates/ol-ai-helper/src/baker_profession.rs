//! Haxe: `AiBase` baker profession family (chunk **AI-JOB-BAKER** / **AI-JOB-BAKER-WIRE**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession('BAKER')` with max-people + sticky last
//! - Speech `BAKER!` â†’ assigned job ([`assign_baker_from_speech`])
//! - Pre-profession dough / pie-crust handling (`UseUpDough` family)
//! - Oven state ladder (Hot 250 â†’ Burning 249 â†’ Adobe 237 / Wood-filled 247)
//! - Hot-oven bake shortCrafts (raw pies, bread loaf, mutton, potato, beans, turkey)
//! - `handleMilk` (milk pouch / cream skewer / buttered bread / butter)
//! - Knife bread/slicing stage (`profession['BAKER']` 0/1/2/3)
//! - `makeRawPies` hysteresis + `count_pies` rotation + pie index via `last_pie`
//! - Mid pipeline: turkey slice, plant carrots, kindling `makeOrCollect`, harvest wheat,
//!   mutton, sheep defer, wheat gate, raw pies, plant wheat/beans, stew, berry/pottery/cleanup
//! - Pipeline seek for self-play / reverse-craft; ladder `try_decide_baker_from_rung`
//! - Live-tick bridge (**AI-JOB-BAKER-WIRE**): [`fill_bake_counts_from_map`],
//!   [`pick_oven_near_home`], [`baker_goal_from_map_and_rung`] / counts variant
//!
//! No world I/O: callers supply map snapshots / counts / oven parent id and apply returned
//! [`BakeAction`]s via craft/shortCraft (AI-CRAFT) and spatial helpers.
//!
//! Residual gaps: live world tile scan into selfplay/npc tick, Defer* live job bodies,
//! hot_oven always-attempt empty shortCraft no-ops (pure SM still stock-gates).

use std::collections::{HashMap, HashSet};

use crate::ai_goals::{Goal, BAKER_TARGET_ID};
use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use crate::farmer_profession::{
    in_count_close_square, is_ignored_floor, short_craft_apply, AI_IGNORED_FLOOR_IDS,
    ShortCraftApply, ShortCraftInput,
};

// â”€â”€ Object ids (OHOL / OpenLife content; Haxe comments in AiBase) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Adobe Oven (cold).
pub const ADOBE_OVEN: i32 = 237;
/// Wood-filled Adobe Oven.
pub const WOOD_FILLED_OVEN: i32 = 247;
/// Burning Adobe Oven.
pub const BURNING_OVEN: i32 = 249;
/// Hot Adobe Oven.
pub const HOT_OVEN: i32 = 250;
/// Knife.
pub const KNIFE: i32 = 560;
/// Bowl of Dough.
pub const BOWL_OF_DOUGH: i32 = 252;
/// Clay Plate.
pub const CLAY_PLATE: i32 = 236;
/// Clay Bowl.
pub const CLAY_BOWL: i32 = 235;
/// Raw Pie Crust.
pub const RAW_PIE_CRUST: i32 = 264;
/// Leavened Dough on Clay Plate.
pub const LEAVENED_DOUGH_PLATE: i32 = 1468;
/// Bowl of Leavened Dough.
pub const BOWL_LEAVENED_DOUGH: i32 = 1466;
/// Raw Bread Loaf.
pub const RAW_BREAD_LOAF: i32 = 1469;
/// Baked Bread.
pub const BAKED_BREAD: i32 = 1470;
/// Sliced Bread.
pub const SLICED_BREAD: i32 = 1471;
/// Raw Mutton.
pub const RAW_MUTTON: i32 = 569;
/// Cooked Mutton.
pub const COOKED_MUTTON: i32 = 570;
/// Raw Potato.
pub const RAW_POTATO: i32 = 1147;
/// Baked Potato.
pub const BAKED_POTATO: i32 = 1148;
/// Bowl of Soaking Beans.
pub const SOAKING_BEANS: i32 = 1180;
/// Plucked Turkey on Plate.
pub const PLUCKED_TURKEY_PLATE: i32 = 2183;
/// Cooked Turkey.
pub const COOKED_TURKEY: i32 = 2185;
/// Cooked Turkey on Plate.
pub const COOKED_TURKEY_PLATE: i32 = 2186;
/// Bowl of Turkey Broth.
pub const TURKEY_BROTH: i32 = 2198;
/// Turkey Slice on Plate.
pub const TURKEY_SLICE_PLATE: i32 = 2190;
/// Open Fermented Sauerkraut.
pub const SAUERKRAUT: i32 = 1241;
/// Chopped Tomato on Plate.
pub const CHOPPED_TOMATO_PLATE: i32 = 2861;
/// Mango Slices.
pub const MANGO_SLICES: i32 = 1880;
/// Kindling.
pub const KINDLING: i32 = 72;
/// Dug Potatoes.
pub const DUG_POTATOES: i32 = 4144;
/// Raw Mutton Pie.
pub const RAW_MUTTON_PIE: i32 = 802;
/// Cooked Mutton Pie.
pub const COOKED_MUTTON_PIE: i32 = 803;
/// Raw Carrot Pie.
pub const RAW_CARROT_PIE: i32 = 268;
/// Cooked Carrot Pie.
pub const COOKED_CARROT_PIE: i32 = 273;
/// Raw Berry Pie.
pub const RAW_BERRY_PIE: i32 = 265;
/// Cooked Berry Pie.
pub const COOKED_BERRY_PIE: i32 = 272;
/// Bowl of Wheat.
pub const BOWL_OF_WHEAT: i32 = 245;
/// Deep Tilled Row.
pub const DEEP_TILLED_ROW: i32 = 213;
/// Ripe Wheat.
pub const RIPE_WHEAT: i32 = 242;
/// Dry Planted Wheat.
pub const DRY_PLANTED_WHEAT: i32 = 228;
/// Threshed Wheat.
pub const THRESHED_WHEAT: i32 = 226;
/// Straw (counts as wheat stock with threshed family).
pub const STRAW: i32 = 297;
/// Pile of Threshed Wheat.
pub const PILE_THRESHED_WHEAT: i32 = 4070;
/// Raw Stew Pot.
pub const RAW_STEW_POT: i32 = 1246;
/// Wild Gooseberry Bush.
pub const WILD_BUSH: i32 = 30;
/// Domestic Gooseberry Bush.
pub const DOMESTIC_BUSH: i32 = 391;
/// Bowl of Gooseberries.
pub const BOWL_GOOSEBERRIES: i32 = 253;
/// Bowl of Gooseberries and Carrot.
pub const BOWL_BERRIES_CARROT: i32 = 258;
/// Whole Milk Pouch.
// Haxe: handleMilk 4081
pub const MILK_POUCH: i32 = 4081;
/// Skewer (cream whip / butter path).
pub const SKEWER: i32 = 139;
/// Bowl of Whipped Cream.
pub const WHIPPED_CREAM: i32 = 3374;
/// Bowl of Cream.
pub const BOWL_OF_CREAM: i32 = 1464;
/// Buttered Bread on Clay Plate.
pub const BUTTERED_BREAD: i32 = 1473;
/// Bowl of Butter.
pub const BOWL_OF_BUTTER: i32 = 1465;
/// Dry Planted Carrots (baker mid-pipeline plant gate).
pub const DRY_PLANTED_CARROTS: i32 = 396;
/// Wet Planted Carrots.
pub const WET_PLANTED_CARROTS: i32 = 399;
/// Carrot (pulled).
pub const CARROT: i32 = 402;
/// Dry Planted Beans (plant-beans fallthrough craft target).
pub const DRY_PLANTED_BEANS: i32 = 1177;
/// Domestic Sheep.
pub const DOMESTIC_SHEEP: i32 = 575;
/// Domestic Lamb.
pub const DOMESTIC_LAMB: i32 = 542;
/// Hungry Domestic Lamb.
pub const HUNGRY_DOMESTIC_LAMB: i32 = 604;
/// Bowl of Tomato Seeds (makeSeatsAndCleanUp craft).
pub const BOWL_TOMATO_SEEDS: i32 = 2828;
/// Harvested Wheat (harvest wheat craft step).
pub const HARVESTED_WHEAT: i32 = 224;
/// Wheat Sheaf.
pub const WHEAT_SHEAF: i32 = 225;

/// Home/oven search radius used by Haxe `doBaking` (20).
pub const OVEN_SEARCH_RADIUS: i32 = 20;
/// Pie crust home-radius count (Haxe 30).
pub const PIE_CRUST_COUNT_RADIUS: i32 = 30;
/// Haxe `doBaking` temporary `itemToCraft.maxSearchRadius` wrapper.
// Haxe: AiBase.doBaking ~3121â€“3124
pub const BAKING_CRAFT_SEARCH_RADIUS: i32 = 30;
/// Default shortCraft distance / maxNewActor count radius (Haxe shortCraft r=20â€“30).
// Haxe: AiBase.shortCraft distance default 20; maxNewActor CountCloseObjects r=30
pub const BAKING_SHORTCRAFT_RADIUS: i32 = 30;
/// Haxe `shortCraftOnTarget(569, hotOven, false, 4)` maxNewActor for raw mutton.
// Haxe: AiBase.doBakingHelper ~3219
pub const RAW_MUTTON_HOT_OVEN_MAX_NEW_ACTOR: i32 = 4;
/// Default max people for baker profession.
pub const BAKER_DEFAULT_MAX_PEOPLE: i32 = 1;
/// Haxe assigned BAKER job uses `doBaking(100)`.
// Haxe: AiBase assignedProfession BAKER ~722
pub const BAKER_ASSIGNED_MAX_PEOPLE: i32 = 100;

/// Haxe `AiBase.rawPies` â€” raw pie ids matched 1:1 with [`COOKED_PIES`].
// Haxe: AiBase.rawPies = [265, 802, 268, 270, 266, 271, 269, 267]
pub const RAW_PIES: &[i32] = &[265, 802, 268, 270, 266, 271, 269, 267];

/// Haxe `AiHelper.pies` â€” cooked pie ids parallel to [`RAW_PIES`].
// Haxe: AiHelper.pies = [272, 803, 273, 274, 275, 276, 277, 278]
pub const COOKED_PIES: &[i32] = &[272, 803, 273, 274, 275, 276, 277, 278];

/// Extra raw bake targets (not in rawPies) counted toward firing the oven.
pub const EXTRA_RAW_BAKE_IDS: &[i32] = &[
    RAW_POTATO,      // 1147
    RAW_BREAD_LOAF,  // 1469
    RAW_MUTTON,      // 569
    SOAKING_BEANS,   // 1180
];

/// Haxe `dropNearOvenItemIds` subset (bakery staging bias).
pub const DROP_NEAR_OVEN_IDS: &[i32] = &[
    235, 1603, 236, 1602, 252, 1470, 1471, 1285, 253, 518, 547, 548, 260, 4057, 502, 569, 1354,
    245, 258,
];

// â”€â”€ Profession key â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Canonical Haxe profession string for baker.
pub const BAKER_PROFESSION_KEY: &str = "BAKER";

/// Parse speech / assigned profession tokens for baker.
///
/// Accepts `BAKER`, `BAKER!`, case-insensitive.
// Haxe: AiBase speech endsWith("!") ~4950; assignedProfession == 'BAKER'
pub fn parse_baker_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("BAKER")
}

// â”€â”€ Runtime / stage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Sticky last + assigned + stage weight for BAKER.
///
/// Haxe `this.profession['BAKER']` is a float stage:
/// - `0` â€” idle / no oven / finished fallthrough
/// - `1` â€” just became baker (`hasOrBecomeProfession`)
/// - `2` â€” hot oven bake path active
/// - `3` â€” post knife-bread; mid pipeline (pies/wheat/â€¦)
#[derive(Debug, Clone, PartialEq)]
pub struct BakerProfessionRuntime {
    /// Sticky last profession is baker.
    pub is_last_baker: bool,
    /// Assigned via speech / order.
    pub is_assigned_baker: bool,
    /// Haxe `this.profession['BAKER']` stage weight.
    pub stage: f32,
    /// Haxe `lastPie` index into raw/cooked pie arrays (`-1` = unset â†’ random).
    pub last_pie: i32,
    /// Haxe `countPies` â€” incremented when a raw pie craft finishes; drives `extraPies % 4`.
    // Haxe: AiBase.countPies ~112; bumped ~9092â€“9093
    pub count_pies: i32,
}

impl Default for BakerProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_baker: false,
            is_assigned_baker: false,
            stage: 0.0,
            last_pie: -1,
            count_pies: 0,
        }
    }
}

impl BakerProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset stage (Haxe sets BAKER=0 when no oven / end of doBaking).
    pub fn reset_stage(&mut self) {
        self.stage = 0.0;
    }
}

/// Haxe `taskState` subset used by baking.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BakerTaskState {
    /// `makeRawPies` hysteresis: 0 idle, 1 active.
    pub make_raw_pies: f32,
    /// Kindling `makeOrCollect(72,1,5)` hysteresis (`taskState['72']`).
    // Haxe: AiBase.makeOrCollect ~6034
    pub kindling_collect: f32,
    /// Carrot planter hysteresis (baker mid calls `doPlantCarrots(2,10)`).
    pub carrot_planter: f32,
    /// Wheat harvester hysteresis (`doHarvestWheat(1,4)`).
    pub wheat_harvester: f32,
    /// Wheat planter hysteresis (`doPlantWheat(2,5)`).
    pub wheat_planter: f32,
    /// Bean planter hysteresis (`doPlantBeans(2,4)`).
    pub bean_planter: f32,
}

/// Count peers already sticky on BAKER (Haxe `countProfession('BAKER')`).
///
/// Prefer [`count_baker_peers_filtered`] when peer snapshots are available.
// Haxe: AiBase.countProfession
pub fn count_baker_peers(peer_count_with_last_baker: f32) -> f32 {
    peer_count_with_last_baker.max(0.0)
}

/// One AI peer for pure `countProfession('BAKER')` filtering.
// Haxe: AiBase.countProfession ~1284â€“1308
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BakerPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    /// Haxe `ai.playerToFollow != null` â€” following another player excludes from count.
    pub has_player_to_follow: bool,
    /// Same home tile as the counting AI (`home.tx/ty` match).
    pub same_home: bool,
    /// `lastProfession == 'BAKER'`.
    pub last_is_baker: bool,
}

impl BakerPeerSnapshot {
    /// Eligible for profession count (before last-profession match).
    // Haxe: AiBase.countProfession filters
    pub fn eligible_for_count(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        // Gravekeeper exception not used for BAKER.
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

    pub fn counts_as_baker(self, min_age_to_eat: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age_to_eat, max_age) && self.last_is_baker
    }
}

/// Haxe `countProfession('BAKER')` with full peer filters.
// Haxe: AiBase.countProfession ~1284
pub fn count_baker_peers_filtered(
    peers: &[BakerPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_baker(min_age_to_eat, max_age))
        .count() as f32
}

/// Haxe `hasOrBecomeProfession('BAKER', max)`.
///
/// - Sticky: if already last baker, keep and return true.
/// - `max < 0`: high priority â€” do job without assigning (always true).
/// - Else: if `peer_count >= max + was_idle` refuse; else stage=max(stage,1) and sticky.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_baker(
    runtime: &mut BakerProfessionRuntime,
    max: i32,
    peer_count_with_last_baker: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_baker {
        runtime.is_last_baker = true;
        return true;
    }
    let count = count_baker_peers(peer_count_with_last_baker);
    let cap = max as f32 + was_idle.max(0.0);
    if count >= cap {
        return false;
    }
    runtime.stage = runtime.stage.max(1.0);
    runtime.is_last_baker = true;
    true
}

/// [`has_or_become_baker`] with filtered peer snapshots (preferred live path).
pub fn has_or_become_baker_filtered(
    runtime: &mut BakerProfessionRuntime,
    max: i32,
    peers: &[BakerPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
    was_idle: f32,
) -> bool {
    let peer_count = count_baker_peers_filtered(peers, min_age_to_eat, max_age);
    has_or_become_baker(runtime, max, peer_count, was_idle)
}

/// Prefer assigned over sticky last for AssignedJob dispatch.
// Haxe: assignedProfession == 'BAKER' || lastProfession == 'BAKER' ~721
pub fn resolve_baker_assigned_job(runtime: &BakerProfessionRuntime) -> bool {
    runtime.is_assigned_baker || runtime.is_last_baker
}

// â”€â”€ Oven selection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Ordered oven parent ids for â€œany oven presentâ€ checks (not hot-first).
// Haxe: doBaking cold oven lookup 237 then 247
pub fn cold_oven_id_priority() -> &'static [i32] {
    &[ADOBE_OVEN, WOOD_FILLED_OVEN]
}

/// Hot / fire / cold priority for snapshot classification.
pub fn oven_id_priority() -> &'static [i32] {
    &[HOT_OVEN, BURNING_OVEN, ADOBE_OVEN, WOOD_FILLED_OVEN]
}

/// Pick best oven parent id from nearby oven parents (hot first).
pub fn pick_oven_parent(nearby_oven_parent_ids: &[i32]) -> Option<i32> {
    for &want in oven_id_priority() {
        if nearby_oven_parent_ids.contains(&want) {
            return Some(want);
        }
    }
    None
}

/// True if `id` is any adobe-oven family parent used by baking.
pub fn is_oven_id(id: i32) -> bool {
    matches!(
        id,
        ADOBE_OVEN | WOOD_FILLED_OVEN | BURNING_OVEN | HOT_OVEN
    )
}

/// Coarse oven state for pure decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OvenState {
    Hot,
    Burning,
    Cold,
    None,
}

impl OvenState {
    pub fn from_parent(id: Option<i32>) -> Self {
        match id {
            Some(HOT_OVEN) => Self::Hot,
            Some(BURNING_OVEN) => Self::Burning,
            Some(ADOBE_OVEN) | Some(WOOD_FILLED_OVEN) => Self::Cold,
            _ => Self::None,
        }
    }
}

// â”€â”€ World counts snapshot â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Close-object counts near home/oven (Haxe CountCloseObjects / countCurrentObject).
#[derive(Debug, Clone, Default)]
pub struct BakeCounts {
    /// Object parent id â†’ count (piles expanded by caller if needed).
    pub by_id: HashMap<i32, i32>,
    /// Held object parent id (0 empty).
    pub held_id: i32,
    /// Held object numberOfUses (for dough last-use reserve).
    pub held_uses: i32,
    /// Closest oven parent id if any (237/247/249/250); `None` = no oven.
    pub oven_parent_id: Option<i32>,
    /// Player is hungry (affects neededRaw + kindling skip).
    pub is_hungry: bool,
    /// Caller has corn seeds (stew pot gate).
    pub has_corn_seeds: bool,
    /// Caller has bean seeds (soaking beans gate).
    pub has_bean_seeds: bool,
}

impl BakeCounts {
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

    /// Effective count including held object of that id (+1).
    pub fn get_with_held(&self, id: i32) -> i32 {
        self.get(id) + if self.held_id == id { 1 } else { 0 }
    }

    pub fn oven_state(&self) -> OvenState {
        OvenState::from_parent(self.oven_parent_id)
    }

    pub fn has_knife(&self) -> bool {
        self.get_with_held(KNIFE) > 0
    }

    pub fn has_close_plate(&self) -> bool {
        self.get(CLAY_PLATE) > 0
    }
}

// â”€â”€ Actions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Pure decision output â€” execution is AI-CRAFT / shortCraft wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakeAction {
    /// Nothing to do in this step (or deferred fallthrough with no pure target).
    None,
    /// Haxe `shortCraft(actor, target)` / `shortCraftOnTarget(actor, oven)`.
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `craftItem(objectId)` / `craftItemMax`.
    CraftItem { object_id: i32 },
    /// Generic farm handoff when no finer defer applies â€” live tick.
    DeferFarm,
    /// Haxe `doPlantCarrots` â€” live farm / pure craft when stock present.
    DeferPlantCarrots,
    /// Haxe `doHarvestWheat`.
    DeferHarvestWheat,
    /// Haxe `isSheepHerding`.
    DeferSheepHerding,
    /// Haxe `doPlantWheat`.
    DeferPlantWheat,
    /// Haxe `doPlantBeans`.
    DeferPlantBeans,
    /// Haxe `fillBerryBowlIfNeeded`.
    DeferBerryBowl,
    /// Haxe `makeSeatsAndCleanUp`.
    DeferSeatsCleanup,
    /// Haxe end-of-job `cleanUp`.
    DeferCleanup,
    /// Chain to `doPottery` â€” live tick / AI-JOB-POTTER.
    DeferPottery,
    /// No oven / cannot become baker / job refuse.
    Abort,
}

impl BakeAction {
    pub fn is_some(self) -> bool {
        !matches!(
            self,
            Self::None
                | Self::Abort
                | Self::DeferFarm
                | Self::DeferPlantCarrots
                | Self::DeferHarvestWheat
                | Self::DeferSheepHerding
                | Self::DeferPlantWheat
                | Self::DeferPlantBeans
                | Self::DeferBerryBowl
                | Self::DeferSeatsCleanup
                | Self::DeferCleanup
                | Self::DeferPottery
        )
    }

    /// True if caller should hand off to another profession / cleanup body.
    pub fn is_defer(self) -> bool {
        matches!(
            self,
            Self::DeferFarm
                | Self::DeferPlantCarrots
                | Self::DeferHarvestWheat
                | Self::DeferSheepHerding
                | Self::DeferPlantWheat
                | Self::DeferPlantBeans
                | Self::DeferBerryBowl
                | Self::DeferSeatsCleanup
                | Self::DeferCleanup
                | Self::DeferPottery
        )
    }
}

/// Haxe `craftItemMax(objId, max)` â€” craft when stock below max.
// Haxe: AiBase.craftItemMax ~6604
pub fn craft_item_max(counts: &BakeCounts, object_id: i32, max: i32) -> BakeAction {
    if counts.get_with_held(object_id) < max {
        BakeAction::CraftItem { object_id }
    } else {
        BakeAction::None
    }
}

/// Haxe `makeOrCollect(id, min, max)` pure hysteresis.
///
/// When task flag active, emits [`BakeAction::CraftItem`] for `id` (live tick
/// expands to GetCraftAndDropItemsCloseToObj / collect path).
// Haxe: AiBase.makeOrCollect ~6034â€“6049
pub fn make_or_collect(
    id: i32,
    min: i32,
    max: i32,
    count: i32,
    task_flag: &mut f32,
) -> BakeAction {
    if count <= min {
        *task_flag = 1.0;
    }
    if count >= max {
        *task_flag = 0.0;
    }
    if *task_flag > 0.0 {
        return BakeAction::CraftItem { object_id: id };
    }
    BakeAction::None
}

/// Note a finished raw-pie craft (bumps `count_pies` for `extraPies % 4`).
// Haxe: rawPies.contains(taregtObjectId) â†’ countPies += 1 ~9092
pub fn note_raw_pie_crafted(runtime: &mut BakerProfessionRuntime, crafted_parent_id: i32) {
    if RAW_PIES.contains(&crafted_parent_id) {
        runtime.count_pies = runtime.count_pies.saturating_add(1);
    }
}

/// True when held id should stage-drop near oven (bakery bias).
// Haxe: considerDropHeldObject dropNearOvenItemIds / pies / rawPies ~5216
pub fn should_drop_near_oven(held_id: i32) -> bool {
    if held_id <= 0 {
        return false;
    }
    DROP_NEAR_OVEN_IDS.contains(&held_id)
        || RAW_PIES.contains(&held_id)
        || COOKED_PIES.contains(&held_id)
}

/// Anchor tile for bakery drop staging: oven if known, else home.
// Haxe: considerDropHeldObject dropTarget = home (oven bias residual; dropNearOven body empty)
pub fn drop_near_oven_anchor(
    home_x: i32,
    home_y: i32,
    oven_xy: Option<(i32, i32)>,
) -> (i32, i32) {
    oven_xy.unwrap_or((home_x, home_y))
}

/// When held is a bakery staging id, return drop-anchor tile (home/oven).
///
/// Haxe immediately `dropHeldObject()` for these ids; live tick uses the anchor
/// for spatial drop-near-home/oven placement.
// Haxe: considerDropHeldObject ~5216â€“5218
pub fn consider_drop_near_oven(
    held_id: i32,
    home_x: i32,
    home_y: i32,
    oven_xy: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    if !should_drop_near_oven(held_id) {
        return None;
    }
    Some(drop_near_oven_anchor(home_x, home_y, oven_xy))
}

/// Haxe `shortCraft` / `shortCraftOnTarget` limits for a baker ShortCraft pair.
///
/// Returns `(max_new_actor, craft_actor_if_needed)`.
/// - Raw mutton â†’ hot oven: maxNewActor=4, craftActor=false
/// - Other raw bake â†’ hot oven: craftActor=false, unlimited newActor
/// - Knife bread shortCrafts: craftActor=false
/// - Default: craftActor=true, unlimited
// Haxe: AiBase.doBakingHelper shortCraftOnTarget flags ~3212â€“3226; knife ~3273+
pub fn baker_short_craft_limits(actor: i32, target: i32) -> (i32, bool) {
    if target == HOT_OVEN {
        if actor == RAW_MUTTON {
            return (RAW_MUTTON_HOT_OVEN_MAX_NEW_ACTOR, false);
        }
        if RAW_PIES.contains(&actor)
            || actor == RAW_BREAD_LOAF
            || actor == RAW_POTATO
            || actor == SOAKING_BEANS
        {
            return (-1, false);
        }
        // Plucked turkey on plate uses default craftActorIfNeeded=true
        if actor == PLUCKED_TURKEY_PLATE {
            return (-1, true);
        }
    }
    if actor == KNIFE {
        return (-1, false);
    }
    // Clay bowl + sauerkraut: Haxe shortCraft(235, 1241, 20, 1) â†’ craftActor=true
    (-1, true)
}

/// Map a [`BakeAction::ShortCraft`] through [`short_craft_apply`].
///
/// Fills maxNewActor / craftActor from [`baker_short_craft_limits`] unless
/// `max_new_actor_override >= 0` (caller-supplied). When craftActor is false
/// and held â‰  actor, [`ShortCraftApply::SeekOrCraftActor`] carries
/// `craft_if_needed=false` so live tick only seeks (does not craft).
///
/// Hungry cost defaults free; use [`bake_action_short_craft_apply_ex`] for food gate.
///
/// Returns `None` for non-ShortCraft actions.
// Haxe: AiBase.shortCraftOnTarget ~2721
pub fn bake_action_short_craft_apply(
    action: BakeAction,
    held_id: i32,
    new_actor_count: i32,
    max_new_actor_override: i32,
) -> Option<ShortCraftApply> {
    bake_action_short_craft_apply_ex(action, held_id, new_actor_count, max_new_actor_override, 20.0, 0.0)
}

/// Baker shortCraft with hungry work cost (Haxe always-on shortCraftOnTarget gate).
// Haxe: AiBase.shortCraftOnTarget ~2728
pub fn bake_action_short_craft_apply_ex(
    action: BakeAction,
    held_id: i32,
    new_actor_count: i32,
    max_new_actor_override: i32,
    food_store: f32,
    transition_hungry_cost: f32,
) -> Option<ShortCraftApply> {
    match action {
        BakeAction::ShortCraft { actor, target } => {
            let (default_max, craft_actor) = baker_short_craft_limits(actor, target);
            let max_new = if max_new_actor_override >= 0 {
                max_new_actor_override
            } else {
                default_max
            };
            Some(short_craft_apply(ShortCraftInput {
                held_id,
                actor_id: actor,
                target_id: target,
                target_uses: 1,
                target_biome: None,
                has_carrot_seeds: true,
                new_actor_count,
                max_new_actor: max_new,
                try_weak_skewer_first: false, // baker path: no farm skewer prefer
                craft_actor_if_needed: craft_actor,
                food_store,
                transition_hungry_cost,
            }))
        }
        _ => None,
    }
}

/// Direct shortCraft apply for baker actor/target (same limits as action path).
pub fn baker_short_craft_apply(
    held_id: i32,
    actor: i32,
    target: i32,
    new_actor_count: i32,
) -> ShortCraftApply {
    bake_action_short_craft_apply(
        BakeAction::ShortCraft { actor, target },
        held_id,
        new_actor_count,
        -1,
    )
    .expect("ShortCraft action")
}

/// Build [`BakeCounts`] from a nearby (id, count) snapshot (mock home radius).
///
/// Oven parent is [`pick_oven_parent`] over ids present with count > 0.
// Haxe: CountCloseObjects / GetClosestObjectToPosition oven family
pub fn bake_counts_from_nearby(
    nearby: &[(i32, i32)],
    held_id: i32,
    held_uses: i32,
    is_hungry: bool,
    has_corn_seeds: bool,
    has_bean_seeds: bool,
) -> BakeCounts {
    let mut c = BakeCounts {
        held_id,
        held_uses,
        is_hungry,
        has_corn_seeds,
        has_bean_seeds,
        ..Default::default()
    };
    let mut oven_ids: Vec<i32> = Vec::new();
    for &(id, n) in nearby {
        if n <= 0 {
            continue;
        }
        c.set(id, c.get(id) + n);
        if is_oven_id(id) {
            oven_ids.push(id);
        }
    }
    c.oven_parent_id = pick_oven_parent(&oven_ids);
    c
}

/// Haxe assigned vs age-rotated maxPeople for `doBaking`.
// Haxe: doBaking(100) assigned; doBaking() / doBaking(1|2) elsewhere
pub fn baker_max_people_for_dispatch(is_assigned_job: bool, hot_oven_urgent: bool) -> i32 {
    if is_assigned_job {
        BAKER_ASSIGNED_MAX_PEOPLE
    } else if hot_oven_urgent {
        2
    } else {
        BAKER_DEFAULT_MAX_PEOPLE
    }
}

/// Infer pipeline stage from inventory for self-play (no runtime sticky stage).
pub fn infer_baker_pipeline_stage(have: &HashSet<i32>) -> f32 {
    if have.contains(&COOKED_CARROT_PIE) || have.contains(&COOKED_BERRY_PIE) {
        return 5.0;
    }
    if have.contains(&RAW_CARROT_PIE) || have.contains(&RAW_MUTTON_PIE) {
        return 4.5;
    }
    if have.contains(&SLICED_BREAD) {
        return 3.5;
    }
    if have.contains(&LEAVENED_DOUGH_PLATE) || have.contains(&BAKED_BREAD) {
        return 3.0;
    }
    if have.contains(&BURNING_OVEN) || have.contains(&HOT_OVEN) {
        return 2.5;
    }
    if have.contains(&RAW_PIE_CRUST) || have.contains(&BOWL_OF_DOUGH) {
        return 2.0;
    }
    if have.contains(&CLAY_PLATE) {
        return 1.5;
    }
    0.0
}

/// Haxe `handleMilk` pure body (post hot-oven in `doBakingHelper`).
///
/// Pure SM is **stock-gated**: emits only when milk-family stock/held is present so
/// empty-world `do_baking` can still abort / fire oven. Live tick may still call
/// `craftItem(4081)` from zero via craft graph when desired.
// Haxe: AiBase.handleMilk ~1774â€“1817
pub fn handle_milk(counts: &BakeCounts) -> BakeAction {
    let pouch = counts.get_with_held(MILK_POUCH);
    let has_whipped = counts.get(WHIPPED_CREAM) > 0 || counts.held_id == WHIPPED_CREAM;
    let has_cream = counts.get(BOWL_OF_CREAM) > 0 || counts.held_id == BOWL_OF_CREAM;
    let has_butter = counts.get(BOWL_OF_BUTTER) > 0 || counts.held_id == BOWL_OF_BUTTER;
    let has_buttered = counts.get(BUTTERED_BREAD) > 0 || counts.held_id == BUTTERED_BREAD;
    let milk_active = pouch > 0 || has_whipped || has_cream || has_butter || has_buttered;
    if !milk_active {
        return BakeAction::None;
    }

    // Whole Milk Pouch 4081 â€” keep at least 3 (Haxe first step when path active)
    if pouch < 3 {
        return BakeAction::CraftItem {
            object_id: MILK_POUCH,
        };
    }
    // Skewer + Bowl of Whipped Cream
    if has_whipped {
        return BakeAction::ShortCraft {
            actor: SKEWER,
            target: WHIPPED_CREAM,
        };
    }
    // Skewer + Bowl of Cream
    if has_cream {
        return BakeAction::ShortCraft {
            actor: SKEWER,
            target: BOWL_OF_CREAM,
        };
    }
    // Buttered Bread on Clay Plate 1473
    if counts.get_with_held(BUTTERED_BREAD) < 1 {
        return BakeAction::CraftItem {
            object_id: BUTTERED_BREAD,
        };
    }
    // Bowl of Butter 1465
    if counts.get_with_held(BOWL_OF_BUTTER) < 1 {
        return BakeAction::CraftItem {
            object_id: BOWL_OF_BUTTER,
        };
    }
    BakeAction::None
}

/// Carrot stock units (same weighting as farmer `carrot_stock_units`).
pub fn baker_carrot_stock(counts: &BakeCounts) -> i32 {
    let planted = counts.get(WET_PLANTED_CARROTS) + counts.get(DRY_PLANTED_CARROTS);
    counts.get(CARROT) + 4 * planted
}

/// Pure `doPlantCarrots(min,max)` for baker mid-pipeline.
// Haxe: AiBase.doPlantCarrots via doBakingHelper ~3314
pub fn plant_carrots_for_baker(
    min: i32,
    max: i32,
    counts: &BakeCounts,
    task: &mut BakerTaskState,
) -> BakeAction {
    let stock = baker_carrot_stock(counts);
    if stock >= max {
        task.carrot_planter = 0.0;
        return BakeAction::None;
    }
    if stock <= min {
        task.carrot_planter = 1.0;
    }
    if task.carrot_planter < 1.0 {
        return BakeAction::None;
    }
    BakeAction::CraftItem {
        object_id: DRY_PLANTED_CARROTS,
    }
}

/// Pure harvest-wheat gate used mid-bake (`doHarvestWheat(1,4)`).
// Haxe: AiBase.doHarvestWheat ~2449; baker call ~3322
pub fn harvest_wheat_for_baker(
    min_harvest: i32,
    max_harvest: i32,
    counts: &BakeCounts,
    task: &mut BakerTaskState,
) -> BakeAction {
    let threshed = counts.get(THRESHED_WHEAT) + counts.get(PILE_THRESHED_WHEAT);
    let all_harvested = threshed + counts.get(HARVESTED_WHEAT) + counts.get(WHEAT_SHEAF);
    let planted_ripe = counts.get(RIPE_WHEAT);

    if threshed >= max_harvest {
        task.wheat_harvester = 1.0;
        return BakeAction::None;
    }
    if threshed < min_harvest {
        task.wheat_harvester = 0.0;
    }
    if task.wheat_harvester > 0.0 {
        return BakeAction::None;
    }
    if planted_ripe > 0 && all_harvested < max_harvest {
        return BakeAction::CraftItem {
            object_id: HARVESTED_WHEAT,
        };
    }
    if counts.get(HARVESTED_WHEAT) > 0 {
        return BakeAction::CraftItem {
            object_id: WHEAT_SHEAF,
        };
    }
    if counts.get(WHEAT_SHEAF) > 0 {
        return BakeAction::CraftItem {
            object_id: THRESHED_WHEAT,
        };
    }
    // Ripe present but no craft path stocked â†’ live harvest body.
    if planted_ripe > 0 {
        return BakeAction::DeferHarvestWheat;
    }
    task.wheat_harvester = 1.0;
    BakeAction::None
}

/// Sheep-herding steps that baker mid tries (`isSheepHerding(2,5)`).
///
/// Emits lamb shortCrafts when feed stock is local; otherwise
/// [`BakeAction::DeferSheepHerding`] so profession_scan can expand via full
/// [`crate::is_sheep_herding`] (calves / milk cow / shorn / sheep).
// Haxe: AiBase.isSheepHerding ~1820; baker ~3340
pub fn sheep_herding_for_baker(counts: &BakeCounts, max_animal: i32) -> BakeAction {
    let sheep = counts.get(DOMESTIC_SHEEP);
    if sheep < max_animal {
        // Bowl of Gooseberries and Carrot + Hungry Domestic Lamb / Domestic Lamb
        if counts.get(HUNGRY_DOMESTIC_LAMB) > 0
            && (counts.get(BOWL_BERRIES_CARROT) > 0 || counts.held_id == BOWL_BERRIES_CARROT)
        {
            return BakeAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: HUNGRY_DOMESTIC_LAMB,
            };
        }
        if counts.get(DOMESTIC_LAMB) > 0
            && (counts.get(BOWL_BERRIES_CARROT) > 0 || counts.held_id == BOWL_BERRIES_CARROT)
        {
            return BakeAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: DOMESTIC_LAMB,
            };
        }
        // Lambs present without feed stock â†’ live full shepherd SM
        if counts.get(HUNGRY_DOMESTIC_LAMB) > 0 || counts.get(DOMESTIC_LAMB) > 0 {
            return BakeAction::DeferSheepHerding;
        }
    }
    // Shorn / sheep feed / calves / milk cow without local feed â†’ defer full body
    // Haxe: isSheepHerding continues past lambs into handleMilk/calves/shorn
    if counts.get(crate::shepherd_profession::SHORN_DOMESTIC_SHEEP) > 0
        || counts.get(crate::shepherd_profession::HUNGRY_DOMESTIC_CALF) > 0
        || counts.get(crate::shepherd_profession::DOMESTIC_CALF) > 0
        || counts.get(crate::shepherd_profession::MILK_COW) > 0
    {
        return BakeAction::DeferSheepHerding;
    }
    BakeAction::None
}

/// Fill berry bowl when holding bowl of gooseberries near a bush.
// Haxe: AiBase.fillBerryBowlIfNeeded ~4224
pub fn fill_berry_bowl_if_needed(counts: &BakeCounts) -> BakeAction {
    if counts.held_id != BOWL_GOOSEBERRIES {
        return BakeAction::None;
    }
    if counts.get(DOMESTIC_BUSH) > 0 {
        return BakeAction::ShortCraft {
            actor: BOWL_GOOSEBERRIES,
            target: DOMESTIC_BUSH,
        };
    }
    if counts.get(WILD_BUSH) > 0 {
        return BakeAction::ShortCraft {
            actor: BOWL_GOOSEBERRIES,
            target: WILD_BUSH,
        };
    }
    BakeAction::DeferBerryBowl
}

/// makeSeatsAndCleanUp pure gate.
///
/// - Hungry â†’ no-op
/// - Tomato seeds already present â†’ early exit (Haxe `countTomatoSeeds > 0`)
/// - `bowl_filler_allowed` â†’ [`BakeAction::CraftItem`] 2828 (Haxe craft after BOWLFILLER)
/// - `force` â†’ [`BakeAction::DeferSeatsCleanup`] when no seeds / no BOWLFILLER
/// - Else None so bakery pipeline continues
// Haxe: AiBase.makeSeatsAndCleanUp ~3485
pub fn make_seats_and_cleanup(counts: &BakeCounts) -> BakeAction {
    make_seats_and_cleanup_ex(counts, false, false)
}

/// Extended seats helper.
///
/// `bowl_filler_allowed` mirrors successful `hasOrBecomeProfession('BOWLFILLER')`.
// Haxe: AiBase.makeSeatsAndCleanUp ~3485â€“3512
pub fn make_seats_and_cleanup_ex(
    counts: &BakeCounts,
    force: bool,
    bowl_filler_allowed: bool,
) -> BakeAction {
    if counts.is_hungry {
        return BakeAction::None;
    }
    let tomato = counts.get_with_held(BOWL_TOMATO_SEEDS);
    if tomato > 0 {
        return BakeAction::None;
    }
    if bowl_filler_allowed {
        return BakeAction::CraftItem {
            object_id: BOWL_TOMATO_SEEDS,
        };
    }
    if force {
        return BakeAction::DeferSeatsCleanup;
    }
    BakeAction::None
}

// â”€â”€ Dough / bread helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe max dough uses kept in bowl when knife present and bread stock low.
///
/// - no knife â†’ 0 (use up all dough to crust)
/// - knife + countBread â‰¤ 1 â†’ 1 (keep last use for bread plate)
/// - countBread > 1 â†’ 0
// Haxe: doBakingHelper maxDoughInBowl ~3133â€“3152
pub fn max_dough_in_bowl(has_knife: bool, count_bread_family: i32) -> i32 {
    let mut max = if has_knife { 1 } else { 0 };
    if count_bread_family > 1 {
        max = 0;
    }
    max
}

/// Sliced + leavened-on-plate bread stock (Haxe countBread += countSlicedBread).
pub fn count_bread_family(counts: &BakeCounts) -> i32 {
    counts.get(LEAVENED_DOUGH_PLATE) + counts.get(SLICED_BREAD)
}

/// Pre-profession dough handling (runs even before hasOrBecomeProfession).
///
/// 1. Held Bowl of Dough 252 + plate when uses > maxDoughInBowl
/// 2. Craft Raw Pie Crust 264 when dough present, crust < 5, maxDoughInBowl == 0
// Haxe: doBakingHelper ~3154â€“3163
pub fn pre_profession_dough(counts: &BakeCounts) -> BakeAction {
    let has_knife = counts.has_knife();
    let bread = count_bread_family(counts);
    let max_d = max_dough_in_bowl(has_knife, bread);

    if counts.held_id == BOWL_OF_DOUGH && counts.held_uses > max_d {
        return BakeAction::ShortCraft {
            actor: BOWL_OF_DOUGH,
            target: CLAY_PLATE,
        };
    }

    let count_dough = counts.get_with_held(BOWL_OF_DOUGH);
    let count_crust = counts.get(RAW_PIE_CRUST);
    if count_dough > 0 && count_crust < 5 && max_d == 0 {
        return BakeAction::CraftItem {
            object_id: RAW_PIE_CRUST,
        };
    }
    BakeAction::None
}

/// Count raw pies + extra raw bake items (for lighting oven).
// Haxe: countRawPies when hotOven==null && fireOven==null ~3173â€“3187
pub fn count_raw_stuff_to_bake(counts: &BakeCounts) -> i32 {
    counts.sum(RAW_PIES) + counts.sum(EXTRA_RAW_BAKE_IDS)
}

/// Haxe `neededRaw = isHungry ? 1 : 4; if no plate â†’ 1`.
pub fn needed_raw_to_fire_oven(is_hungry: bool, has_close_plate: bool) -> i32 {
    if !has_close_plate {
        return 1;
    }
    if is_hungry {
        1
    } else {
        4
    }
}

// â”€â”€ Hot oven bake â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Resolve next pie index (Haxe lastPie / random fallback).
///
/// `rng_index` is used only when `last_pie < 0` (caller supplies random in 0..len-1).
pub fn next_pie_start(last_pie: i32, rng_index: usize) -> usize {
    if last_pie >= 0 {
        (last_pie as usize) % RAW_PIES.len()
    } else {
        rng_index % RAW_PIES.len()
    }
}

/// Hot oven shortCraft chain: raw pies (rotated), bread loaf, mutton, potato, beans, turkey.
///
/// Mutates `runtime.stage = 2` and advances `last_pie` when a raw pie slot is chosen.
// Haxe: doBakingHelper hotOven block ~3206â€“3227
pub fn hot_oven_bake(
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    rng_pie_index: usize,
) -> BakeAction {
    runtime.stage = 2.0;
    let start = next_pie_start(runtime.last_pie, rng_pie_index);

    for i in 0..RAW_PIES.len() {
        let index = (start + i) % RAW_PIES.len();
        let raw = RAW_PIES[index];
        if counts.get(raw) > 0 || counts.held_id == raw {
            runtime.last_pie = index as i32;
            return BakeAction::ShortCraft {
                actor: raw,
                target: HOT_OVEN,
            };
        }
        // Haxe always *attempts* shortCraftOnTarget even if missing (no-ops).
        // Pure SM only emits when stock/held present to avoid infinite no-ops.
    }

    // Still rotate last_pie like Haxe for empty attempt path.
    runtime.last_pie = start as i32;

    // Raw Bread Loaf if sliced bread low
    if counts.get(SLICED_BREAD) < 3
        && (counts.get(RAW_BREAD_LOAF) > 0 || counts.held_id == RAW_BREAD_LOAF)
    {
        return BakeAction::ShortCraft {
            actor: RAW_BREAD_LOAF,
            target: HOT_OVEN,
        };
    }
    // Raw Mutton 569 â†’ hot oven, maxNewActor=4 (Haxe ~3219).
    // Pure SM: skip when cooked/newActor stock already at cap so potato/beans can run.
    // Haxe shortCraftOnTarget refuses when CountCloseObjects(newActor) >= 4.
    if counts.get(RAW_MUTTON) > 0 || counts.held_id == RAW_MUTTON {
        let done = counts.get_with_held(COOKED_MUTTON);
        if done < RAW_MUTTON_HOT_OVEN_MAX_NEW_ACTOR {
            return BakeAction::ShortCraft {
                actor: RAW_MUTTON,
                target: HOT_OVEN,
            };
        }
        // maxNewActor refused â€” fall through to potato
    }
    if counts.get(RAW_POTATO) > 0 || counts.held_id == RAW_POTATO {
        return BakeAction::ShortCraft {
            actor: RAW_POTATO,
            target: HOT_OVEN,
        };
    }
    if counts.get(SOAKING_BEANS) > 0 || counts.held_id == SOAKING_BEANS {
        return BakeAction::ShortCraft {
            actor: SOAKING_BEANS,
            target: HOT_OVEN,
        };
    }
    if counts.get(PLUCKED_TURKEY_PLATE) > 0 || counts.held_id == PLUCKED_TURKEY_PLATE {
        return BakeAction::ShortCraft {
            actor: PLUCKED_TURKEY_PLATE,
            target: HOT_OVEN,
        };
    }
    BakeAction::None
}

// â”€â”€ Knife bread stage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Knife-assisted bread / tomato / mango when `stage < 3` and knife present.
// Haxe: doBakingHelper hasKnife && profession['BAKER'] < 3 ~3271â€“3305
pub fn knife_bread_stage(counts: &BakeCounts, runtime: &BakerProfessionRuntime) -> BakeAction {
    if !counts.has_knife() || runtime.stage >= 3.0 {
        return BakeAction::None;
    }

    // Cooked Turkey on Plate
    if counts.get(COOKED_TURKEY_PLATE) > 0 {
        return BakeAction::ShortCraft {
            actor: KNIFE,
            target: COOKED_TURKEY_PLATE,
        };
    }
    // Chopped Tomato on Plate (craftItemMax)
    if counts.get(CHOPPED_TOMATO_PLATE) < 1 {
        return BakeAction::CraftItem {
            object_id: CHOPPED_TOMATO_PLATE,
        };
    }
    // Knife + Baked Bread â†’ sliced
    if counts.get(BAKED_BREAD) > 0 {
        return BakeAction::ShortCraft {
            actor: KNIFE,
            target: BAKED_BREAD,
        };
    }

    if counts.get(SLICED_BREAD) < 2 {
        // Knife + Leavened Dough on Clay Plate
        if counts.get(LEAVENED_DOUGH_PLATE) > 0 {
            return BakeAction::ShortCraft {
                actor: KNIFE,
                target: LEAVENED_DOUGH_PLATE,
            };
        }
        // countBread < 2 â†’ craft leavened dough on plate
        let bread = count_bread_family(counts);
        if bread < 2 {
            return BakeAction::CraftItem {
                object_id: LEAVENED_DOUGH_PLATE,
            };
        }
    }

    // Mango Slices
    if counts.get(MANGO_SLICES) < 1 {
        return BakeAction::CraftItem {
            object_id: MANGO_SLICES,
        };
    }

    BakeAction::None
}

// â”€â”€ makeRawPies â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `makeRawPies(min=2, max=5)` pure body.
///
/// Mutates `task.make_raw_pies` hysteresis and `runtime.last_pie`.
// Haxe: AiBase.makeRawPies ~3381
pub fn make_raw_pies(
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    min: i32,
    max: i32,
    rng_pie_index: usize,
) -> BakeAction {
    let cooked = counts.sum(COOKED_PIES);
    if cooked >= max {
        task.make_raw_pies = 0.0;
        return BakeAction::None;
    }
    if cooked < min {
        task.make_raw_pies = 1.0;
    }
    if task.make_raw_pies < 1.0 {
        return BakeAction::None;
    }

    let count_carrot_pies = counts.sum(&[COOKED_CARROT_PIE, RAW_CARROT_PIE]);
    let count_mutton_pies = counts.sum(&[COOKED_MUTTON_PIE, RAW_MUTTON_PIE]);
    let count_berry = counts.sum(&[
        WILD_BUSH,
        DOMESTIC_BUSH,
        BOWL_GOOSEBERRIES,
        BOWL_BERRIES_CARROT,
    ]);
    // Haxe: extraPies = countPies % 4 (craft-success counter, not cooked sum).
    // Haxe: AiBase.makeRawPies ~3406
    let extra_pies = runtime.count_pies.rem_euclid(4);

    if extra_pies == 0 && count_mutton_pies < 2 {
        return BakeAction::CraftItem {
            object_id: RAW_MUTTON_PIE,
        };
    }
    if extra_pies == 2 && count_carrot_pies < 2 {
        return BakeAction::CraftItem {
            object_id: RAW_CARROT_PIE,
        };
    }

    let start = next_pie_start(runtime.last_pie, rng_pie_index);
    for i in 0..RAW_PIES.len() {
        let index = (start + i) % RAW_PIES.len();
        let raw = RAW_PIES[index];
        let cooked_id = COOKED_PIES[index];
        if raw == RAW_BERRY_PIE && count_berry < 2 {
            continue;
        }
        let count = counts.get(raw) + counts.get(cooked_id);
        if count > 1 {
            continue;
        }
        runtime.last_pie = index as i32;
        return BakeAction::CraftItem { object_id: raw };
    }
    BakeAction::None
}

// â”€â”€ Mid / low pipeline after knife stage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Turkey slice, carrots, kindling, harvest wheat, potatoes, mutton, sheep, wheat gates,
/// raw pies, plant wheat/beans, stew, berry bowl, pottery, cleanup.
///
/// Sequenced farm fallthroughs emit specific `Defer*` actions when pure craft cannot act.
// Haxe: doBakingHelper after stage=3 ~3307â€“3377
pub fn mid_bake_pipeline(
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    rng_pie_index: usize,
) -> BakeAction {
    runtime.stage = 3.0;

    // Turkey Slice on Plate â€” craftItemMax(2190)
    let turkey_slice = craft_item_max(counts, TURKEY_SLICE_PLATE, 1);
    if turkey_slice.is_some() {
        return turkey_slice;
    }

    // Baker needs Carrots â€” doPlantCarrots(2, 10)
    let carrots = plant_carrots_for_baker(2, 10, counts, task);
    if carrots.is_some() {
        return carrots;
    }

    // Kindling makeOrCollect(72, 1, 5) when not hungry
    if !counts.is_hungry {
        let kindling = counts.get_with_held(KINDLING);
        let k = make_or_collect(KINDLING, 1, 5, kindling, &mut task.kindling_collect);
        if k.is_some() {
            return k;
        }
    }

    // doHarvestWheat(1, 4)
    let harvest = harvest_wheat_for_baker(1, 4, counts, task);
    if harvest.is_some() || harvest.is_defer() {
        if !matches!(harvest, BakeAction::None) {
            return harvest;
        }
    }

    // Potatoes: Raw + Baked < 5 && Dug Potatoes
    let count_potatos = counts.sum(&[RAW_POTATO, BAKED_POTATO]);
    if count_potatos < 5 && counts.get(DUG_POTATOES) > 0 {
        return BakeAction::ShortCraft {
            actor: 0,
            target: DUG_POTATOES,
        };
    }

    let count_mutton_pies = counts.sum(&[COOKED_MUTTON_PIE, RAW_MUTTON_PIE]);
    if count_mutton_pies < 2 {
        return BakeAction::CraftItem {
            object_id: RAW_MUTTON_PIE,
        };
    }

    let count_mutton = counts.sum(&[COOKED_MUTTON, RAW_MUTTON]);
    if count_mutton < 2 {
        return BakeAction::CraftItem {
            object_id: RAW_MUTTON,
        };
    }

    // isSheepHerding(2, 5)
    let sheep = sheep_herding_for_baker(counts, 5);
    if sheep.is_some() || matches!(sheep, BakeAction::DeferSheepHerding) {
        return sheep;
    }

    // Wheat stock for pie gate
    let count_wheat = counts.sum(&[
        RIPE_WHEAT,
        DRY_PLANTED_WHEAT,
        THRESHED_WHEAT,
        STRAW,
        PILE_THRESHED_WHEAT,
    ]);

    if count_wheat < 10 && counts.held_id == BOWL_OF_WHEAT {
        return BakeAction::ShortCraft {
            actor: BOWL_OF_WHEAT,
            target: DEEP_TILLED_ROW,
        };
    }

    if count_wheat < 1 {
        return BakeAction::CraftItem {
            object_id: DRY_PLANTED_WHEAT,
        };
    }

    let pies = make_raw_pies(counts, runtime, task, 2, 5, rng_pie_index);
    if pies.is_some() {
        return pies;
    }

    // doPlantWheat(2, 5) after pies
    {
        let planted = counts.get(DRY_PLANTED_WHEAT);
        let stage = count_wheat;
        if stage >= 5 {
            task.wheat_planter = 0.0;
        } else if stage <= 2 {
            task.wheat_planter = 1.0;
        }
        if task.wheat_planter >= 1.0 && planted < 5 {
            return BakeAction::CraftItem {
                object_id: DRY_PLANTED_WHEAT,
            };
        }
        if task.wheat_planter >= 1.0 {
            return BakeAction::DeferPlantWheat;
        }
    }

    // doPlantBeans(2, 4)
    {
        let beans = counts.get(DRY_PLANTED_BEANS) + counts.get(SOAKING_BEANS);
        if beans >= 4 {
            task.bean_planter = 0.0;
        } else if beans <= 2 {
            task.bean_planter = 1.0;
        }
        if task.bean_planter >= 1.0 {
            if beans < 4 {
                return BakeAction::CraftItem {
                    object_id: DRY_PLANTED_BEANS,
                };
            }
            return BakeAction::DeferPlantBeans;
        }
    }

    // Soaking beans / stew when seeds available â€” craftItemMax(..., 2)
    if counts.has_bean_seeds {
        let beans = craft_item_max(counts, SOAKING_BEANS, 2);
        if beans.is_some() {
            return beans;
        }
    }
    if counts.has_corn_seeds {
        let stew = craft_item_max(counts, RAW_STEW_POT, 2);
        if stew.is_some() {
            return stew;
        }
    }

    // End of pure baker pipeline â€” Haxe sets BAKER=0 then berry/pottery/cleanup
    runtime.stage = 0.0;

    let berry = fill_berry_bowl_if_needed(counts);
    if berry.is_some() || matches!(berry, BakeAction::DeferBerryBowl) {
        // Only defer berry when holding bowl without bush; skip empty-hands DeferBerryBowl
        if berry.is_some() {
            return berry;
        }
        if counts.held_id == BOWL_GOOSEBERRIES {
            return berry;
        }
    }

    // doPottery(1)
    if !counts.has_close_plate() || counts.get(RAW_PIE_CRUST) < 1 {
        // Late pottery after profession reset â€” always try once when plates thin
        return BakeAction::DeferPottery;
    }

    if !counts.is_hungry {
        return BakeAction::DeferCleanup;
    }

    BakeAction::None
}

// â”€â”€ Full doBaking â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Full `doBaking` / `doBakingHelper` pure body.
///
/// `rng_pie_index`: random in `0..RAW_PIES.len()` when `last_pie` unset.
// Haxe: AiBase.doBaking / doBakingHelper ~3120â€“3378
pub fn do_baking(
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    max_people: i32,
    peer_count_with_last_baker: f32,
    was_idle: f32,
    rng_pie_index: usize,
) -> BakeAction {
    // Pre-profession dough (before hasOrBecome)
    let pre = pre_profession_dough(counts);
    if pre.is_some() {
        return pre;
    }

    if !has_or_become_baker(
        runtime,
        max_people,
        peer_count_with_last_baker,
        was_idle,
    ) {
        return BakeAction::Abort;
    }

    let oven = counts.oven_state();

    // Fire oven when raw stock ready and no hot/burning
    if matches!(oven, OvenState::None | OvenState::Cold) {
        // Cold oven still "has oven" â€” only fire when truly no hot/burning.
        if !matches!(oven, OvenState::Hot | OvenState::Burning) {
            // Re-check: Cold means adobe/wood-filled present, hot/burning absent.
        }
    }

    match oven {
        OvenState::Hot => {
            let a = hot_oven_bake(counts, runtime, rng_pie_index);
            if a.is_some() {
                return a;
            }
        }
        OvenState::Burning => {
            // Wait for hot â€” no pure action (Haxe continues past fire without craft)
        }
        OvenState::None | OvenState::Cold => {
            // Only count raw + fire when no hot and no burning
            if !matches!(oven, OvenState::Hot) {
                let raw = count_raw_stuff_to_bake(counts);
                let need = needed_raw_to_fire_oven(counts.is_hungry, counts.has_close_plate());
                if matches!(oven, OvenState::None | OvenState::Cold) && raw >= need {
                    // Cold with raw still fires burning oven craft
                    if matches!(oven, OvenState::None)
                        || matches!(oven, OvenState::Cold)
                    {
                        // Haxe: hot==null && fire==null && raw >= needed â†’ craft 249
                        // When cold oven exists, fireOven is still null if not burning â€” yes fire.
                        // Actually Haxe only sets fireOven when hot is null; cold oven is separate later.
                        // Fire check is only hotOven==null && fireOven==null.
                        // Cold adobe does NOT prevent firing â€” fireOven is burning only.
                        // So cold oven + enough raw â†’ craft burning.
                    }
                }
            }
        }
    }

    // Explicit fire path: no hot, no burning, enough raw
    if !matches!(oven, OvenState::Hot | OvenState::Burning) {
        let raw = count_raw_stuff_to_bake(counts);
        let need = needed_raw_to_fire_oven(counts.is_hungry, counts.has_close_plate());
        if raw >= need {
            return BakeAction::CraftItem {
                object_id: BURNING_OVEN,
            };
        }
    }

    // Hot already handled; burning continues to post-oven work (milk / turkey / â€¦)

    // Haxe: handleMilk() after hot-oven block ~3229
    let milk = handle_milk(counts);
    if milk.is_some() {
        return milk;
    }

    // Cooked Turkey 2185
    if counts.get(COOKED_TURKEY) > 0 {
        return BakeAction::ShortCraft {
            actor: 0,
            target: COOKED_TURKEY,
        };
    }
    // Bowl of Turkey Broth 2198 â€” craftItemMax (stock-gated pure: turkey family present)
    // Haxe: craftItemMax(2198) ~3235
    let turkey_family = counts.get(PLUCKED_TURKEY_PLATE)
        + counts.get(COOKED_TURKEY_PLATE)
        + counts.get(COOKED_TURKEY)
        + counts.get(TURKEY_SLICE_PLATE)
        + counts.get(TURKEY_BROTH);
    if turkey_family > 0 {
        let broth = craft_item_max(counts, TURKEY_BROTH, 1);
        if broth.is_some() {
            return broth;
        }
    }

    // Clay Bowl + Open Fermented Sauerkraut
    if counts.get(SAUERKRAUT) > 0 {
        return BakeAction::ShortCraft {
            actor: CLAY_BOWL,
            target: SAUERKRAUT,
        };
    }

    // makeSeatsAndCleanUp when not hungry â€” pure emits craft only if tomato seeds already tracked
    // Haxe: ~3244; DeferSeatsCleanup is not is_some so empty world continues
    let seats = make_seats_and_cleanup(counts);
    if seats.is_some() {
        return seats;
    }

    // No hot/burning: need a cold oven present, else abort stage 0
    if !matches!(oven, OvenState::Hot | OvenState::Burning) {
        if matches!(oven, OvenState::None) {
            runtime.stage = 0.0;
            return BakeAction::Abort;
        }
        // Cold oven present â€” continue prep without baking into oven
    }

    // No close plates + no pie crust â†’ pottery
    if !counts.has_close_plate() && counts.get(RAW_PIE_CRUST) < 1 {
        return BakeAction::DeferPottery;
    }

    let knife_a = knife_bread_stage(counts, runtime);
    if knife_a.is_some() {
        return knife_a;
    }

    // After knife block Haxe sets stage=3
    mid_bake_pipeline(counts, runtime, task, rng_pie_index)
}

/// Dispatch baker job for AssignedJob / sticky last / age-rotated baking.
// Haxe: assigned BAKER â†’ doBaking(100); age job 2 â†’ doBaking()
pub fn decide_baker_job(
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    max_people: i32,
    peer_count_with_last_baker: f32,
    was_idle: f32,
    rng_pie_index: usize,
) -> BakeAction {
    do_baking(
        counts,
        runtime,
        task,
        max_people,
        peer_count_with_last_baker,
        was_idle,
        rng_pie_index,
    )
}

// â”€â”€ Pipeline seek / craft graph â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Ordered bakery product + intermediate ids (seek order for self-play).
pub fn baker_pipeline_targets() -> &'static [i32] {
    &[
        // Oven / fire
        BURNING_OVEN,
        HOT_OVEN,
        ADOBE_OVEN,
        // Dough / bread
        BOWL_OF_DOUGH,
        RAW_PIE_CRUST,
        LEAVENED_DOUGH_PLATE,
        RAW_BREAD_LOAF,
        BAKED_BREAD,
        SLICED_BREAD,
        CLAY_PLATE,
        // Pies
        RAW_BERRY_PIE,
        RAW_CARROT_PIE,
        RAW_MUTTON_PIE,
        COOKED_BERRY_PIE,
        COOKED_CARROT_PIE,
        COOKED_MUTTON_PIE,
        // Sides
        RAW_MUTTON,
        RAW_POTATO,
        SOAKING_BEANS,
        KINDLING,
        KNIFE,
        // Default cooked pie target for thin goals
        COOKED_CARROT_PIE,
    ]
}

/// Stage-aware pure goal: first missing pipeline id, else reverse-craft via graph.
// Haxe: doBaking craftItem chain + thin BAKER_TARGET_ID
pub fn pick_baker_goal(graph: &ReverseCraftGraph, have: &HashSet<i32>, stage: f32) -> Goal {
    // Stage-biased wants
    let stage_wants: &[(f32, i32)] = &[
        (1.5, CLAY_PLATE),
        (2.0, RAW_PIE_CRUST),
        (2.5, BURNING_OVEN),
        (3.0, LEAVENED_DOUGH_PLATE),
        (3.5, SLICED_BREAD),
        (4.0, RAW_CARROT_PIE),
        (4.5, RAW_MUTTON_PIE),
        (5.0, COOKED_CARROT_PIE),
    ];
    for &(need_stage, want) in stage_wants {
        if stage < need_stage && !have.contains(&want) {
            if let Some(ing) = graph.seek_ingredient_for(want, have) {
                return Goal::SeekObject(ing);
            }
            return Goal::SeekObject(want);
        }
    }
    for &want in baker_pipeline_targets() {
        if have.contains(&want) {
            continue;
        }
        if let Some(ing) = graph.seek_ingredient_for(want, have) {
            return Goal::SeekObject(ing);
        }
        return Goal::SeekObject(want);
    }
    Goal::SeekObject(BAKER_TARGET_ID)
}

/// Map a [`BakeAction`] into a high-level [`Goal`] for self-play / thin tick.
pub fn bake_action_to_goal(action: BakeAction) -> Goal {
    match action {
        BakeAction::None | BakeAction::Abort => Goal::SeekObject(BAKER_TARGET_ID),
        BakeAction::DeferFarm
        | BakeAction::DeferPlantCarrots
        | BakeAction::DeferHarvestWheat
        | BakeAction::DeferPlantWheat
        | BakeAction::DeferPlantBeans => Goal::SeekObject(crate::ai_goals::FARMER_TARGET_ID),
        BakeAction::DeferSheepHerding => Goal::SeekObject(DOMESTIC_SHEEP),
        BakeAction::DeferBerryBowl => Goal::SeekObject(BOWL_GOOSEBERRIES),
        BakeAction::DeferSeatsCleanup => Goal::SeekObject(BOWL_TOMATO_SEEDS),
        BakeAction::DeferCleanup => Goal::SeekObject(BAKER_TARGET_ID),
        BakeAction::DeferPottery => Goal::SeekObject(CLAY_PLATE),
        BakeAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        BakeAction::CraftItem { object_id } => Goal::SeekObject(object_id),
    }
}

/// Job-band rungs that should run `decide_baker_job` when profession is baker.
// Haxe: AssignedJob BAKER â†’ doBaking(100); AgeRotated baking â†’ doBaking()
pub fn baker_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "AGE_ROTATED_JOB"
            | "LOW_PRIORITY_WORK"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CRAFT_QUEUE"
            | "CRITICAL_CRAFT"
    )
}

/// Thin ladder bridge: when rung is a baker job band, run pure `decide_baker_job`.
///
/// `is_assigned_job` selects maxPeople 100 vs default 1 (Haxe assigned vs age-rotated).
// Haxe: AiBase assignedProfession / jobByAge==2
pub fn try_decide_baker_from_rung(
    profession_is_baker: bool,
    rung_label: &str,
    is_assigned_job: bool,
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    peer_count_with_last_baker: f32,
    was_idle: f32,
    rng_pie_index: usize,
) -> Option<BakeAction> {
    if !profession_is_baker || !baker_job_rung_label(rung_label) {
        return None;
    }
    let max_people = baker_max_people_for_dispatch(is_assigned_job, false);
    Some(decide_baker_job(
        counts,
        runtime,
        task,
        max_people,
        peer_count_with_last_baker,
        was_idle,
        rng_pie_index,
    ))
}

// â”€â”€ Live-tick spatial fill / compose (AI-JOB-BAKER-WIRE / AI-JOB-BAKER-LIVE) â”€

/// Map object at a tile for mock [`fill_bake_counts_from_map`].
///
/// `uses` is Haxe `numberOfUses` on pile tiles (expanded under `parent_id` in fill).
/// Floor / food / permanent drive [`is_ignored_floor`] skip (origin-floor quirk).
/// Unit tests and thin tick supply a snapshot; live world scan remains residual.
// Haxe: CountCloseObjects / GetClosestObjectToPosition inputs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BakeMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    /// Pile uses; `<= 1` â†’ count as 1 when treated as a pile contribution.
    pub uses: i32,
    /// Floor under this tile (optional; bulk fill uses origin floor param instead).
    pub floor_id: i32,
    /// Haxe `objData.foodValue > 0` â€” never skipped by IsIgnoredFloor.
    pub is_food: bool,
    /// Haxe `objData.isPermanent()` â€” never skipped by IsIgnoredFloor.
    pub is_permanent: bool,
}

impl BakeMapObj {
    pub fn simple(parent_id: i32, x: i32, y: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            uses: 1,
            floor_id: 0,
            is_food: false,
            is_permanent: false,
        }
    }

    pub fn pile(parent_id: i32, x: i32, y: i32, uses: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            uses: uses.max(1),
            floor_id: 0,
            is_food: false,
            is_permanent: false,
        }
    }

    pub fn with_floor(mut self, floor_id: i32) -> Self {
        self.floor_id = floor_id;
        self
    }

    pub fn food(mut self) -> Self {
        self.is_food = true;
        self
    }

    pub fn permanent(mut self) -> Self {
        self.is_permanent = true;
        self
    }

    fn fill_contrib(self) -> i32 {
        if self.uses <= 1 {
            1
        } else {
            self.uses
        }
    }
}

/// Spatial oven candidate (id + world tile).
// Haxe: AiHelper.GetClosestObjectToPosition result (oven family)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OvenCandidate {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Chebyshev distance (OHOL tile distance used by AI helpers).
// Haxe: AiHelper distance / GetClosestObjectToPosition
pub fn baker_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Haxe count / search radii used by live oven fill.
// Haxe: AiBase.doBakingHelper home.tx/ty r=20 oven; pie crust r=30; craft maxSearchRadius=30
pub fn baker_radius_table() -> &'static [(i32, &'static str)] {
    &[
        (OVEN_SEARCH_RADIUS, "oven GetClosestObjectToPosition home"),
        (PIE_CRUST_COUNT_RADIUS, "CountCloseObjects pie crust 264"),
        (BAKING_CRAFT_SEARCH_RADIUS, "doBaking itemToCraft.maxSearchRadius wrap"),
    ]
}

/// Haxe sequential oven lookup: Hot 250 â†’ Burning 249 â†’ Adobe 237 â†’ Wood-filled 247.
///
/// Among matches of the first priority that has any candidate in range, pick closest.
// Haxe: doBakingHelper GetClosestObjectToPosition home r=20 (~3169â€“3176, ~3247â€“3250)
pub fn pick_oven_near_home(
    home_x: i32,
    home_y: i32,
    candidates: &[OvenCandidate],
) -> Option<OvenCandidate> {
    pick_oven_near_home_radius(home_x, home_y, candidates, OVEN_SEARCH_RADIUS)
}

/// Like [`pick_oven_near_home`] with explicit radius.
pub fn pick_oven_near_home_radius(
    home_x: i32,
    home_y: i32,
    candidates: &[OvenCandidate],
    radius: i32,
) -> Option<OvenCandidate> {
    for &want in oven_id_priority() {
        let mut best: Option<(i32, OvenCandidate)> = None;
        for &c in candidates {
            if c.parent_id != want {
                continue;
            }
            let d = baker_chebyshev(home_x, home_y, c.x, c.y);
            if d > radius {
                continue;
            }
            match best {
                None => best = Some((d, c)),
                Some((bd, _)) if d < bd => best = Some((d, c)),
                _ => {}
            }
        }
        if let Some((_, c)) = best {
            return Some(c);
        }
    }
    None
}

/// Fill [`BakeCounts`] from a mock map snapshot (unit tests / thin tick).
///
/// - Oven: [`pick_oven_near_home`] within [`OVEN_SEARCH_RADIUS`] of home (chebyshev)
/// - Pie crust 264: Haxe half-open square [`PIE_CRUST_COUNT_RADIUS`] of home
/// - Other bakery ids: half-open square `home_r` of home (default 20)
/// - Pile `uses` expanded under parent_id; IsIgnoredFloor via origin floor (0 here)
/// - Held uses / hunger / seed flags default off â€” use [`fill_bake_counts_from_map_ex`]
// Haxe: GetClosestObjectToPosition oven + CountCloseObjects pie crust r=30 + countCurrentObject
pub fn fill_bake_counts_from_map(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[BakeMapObj],
    home_r: i32,
) -> BakeCounts {
    fill_bake_counts_from_map_ex(
        home_x, home_y, held_id, 0, objects, home_r, false, false, false, 0,
    )
}

/// Like [`fill_bake_counts_from_map`] with origin floor for IsIgnoredFloor.
// Haxe: CountCloseObjects + IsIgnoredFloor(getFloorId(home))
pub fn fill_bake_counts_from_map_with_floor(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[BakeMapObj],
    home_r: i32,
    origin_floor_id: i32,
) -> BakeCounts {
    fill_bake_counts_from_map_ex(
        home_x,
        home_y,
        held_id,
        0,
        objects,
        home_r,
        false,
        false,
        false,
        origin_floor_id,
    )
}

/// Like [`fill_bake_counts_from_map`] with held uses / hunger / seed flags / origin floor.
pub fn fill_bake_counts_from_map_ex(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    held_uses: i32,
    objects: &[BakeMapObj],
    home_r: i32,
    is_hungry: bool,
    has_corn_seeds: bool,
    has_bean_seeds: bool,
    origin_floor_id: i32,
) -> BakeCounts {
    let oven_cands: Vec<OvenCandidate> = objects
        .iter()
        .filter(|o| is_oven_id(o.parent_id))
        .map(|o| OvenCandidate {
            parent_id: o.parent_id,
            x: o.x,
            y: o.y,
        })
        .collect();
    let oven = pick_oven_near_home(home_x, home_y, &oven_cands);

    let mut counts = BakeCounts {
        held_id,
        held_uses,
        is_hungry,
        has_corn_seeds,
        has_bean_seeds,
        oven_parent_id: oven.map(|o| o.parent_id),
        ..Default::default()
    };

    for o in objects {
        let id = o.parent_id;
        if is_oven_id(id) {
            // Oven is parent only (not inventory stock).
            continue;
        }
        // Haxe CountCloseObjects: half-open square, not chebyshev â‰¤ r.
        let r = if id == RAW_PIE_CRUST {
            PIE_CRUST_COUNT_RADIUS
        } else {
            home_r
        };
        if !in_count_close_square(home_x, home_y, o.x, o.y, r) {
            continue;
        }
        // Haxe: floorID = getFloorId(tx, ty) â€” origin, not each tile
        if is_ignored_floor(
            origin_floor_id,
            o.is_food,
            o.is_permanent,
            &AI_IGNORED_FLOOR_IDS,
        ) {
            continue;
        }
        let n = counts.get(id) + o.fill_contrib();
        counts.set(id, n);
    }
    counts
}

/// Apply speech `BAKER!` / assigned profession token onto sticky runtime.
// Haxe: AiBase speech endsWith("!") â†’ assignedProfession = 'BAKER'
pub fn assign_baker_from_speech(runtime: &mut BakerProfessionRuntime, text: &str) -> bool {
    if !parse_baker_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_baker = true;
    runtime.is_last_baker = true;
    runtime.stage = runtime.stage.max(1.0);
    true
}

/// Compose fill â†’ decide â†’ goal for live tick / ladder consumers (pure).
///
/// Returns `None` when rung is not a baker band (caller keeps thin
/// `SeekObject(BAKER_TARGET_ID)`). On decide maps via [`bake_action_to_goal`].
// Haxe: AssignedJob/sticky/age â†’ doBaking + shortCraft/craftItem seek
pub fn baker_goal_from_map_and_rung(
    profession_is_baker: bool,
    rung_label: &str,
    is_assigned_job: bool,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    held_uses: i32,
    objects: &[BakeMapObj],
    home_r: i32,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    peer_count_with_last_baker: f32,
    was_idle: f32,
    rng_pie_index: usize,
    is_hungry: bool,
    has_corn_seeds: bool,
    has_bean_seeds: bool,
) -> Option<Goal> {
    let counts = fill_bake_counts_from_map_ex(
        home_x,
        home_y,
        held_id,
        held_uses,
        objects,
        home_r,
        is_hungry,
        has_corn_seeds,
        has_bean_seeds,
        0,
    );
    baker_goal_from_counts_and_rung(
        profession_is_baker,
        rung_label,
        is_assigned_job,
        &counts,
        runtime,
        task,
        peer_count_with_last_baker,
        was_idle,
        rng_pie_index,
    )
}

/// Same as [`baker_goal_from_map_and_rung`] but from an already-built [`BakeCounts`].
pub fn baker_goal_from_counts_and_rung(
    profession_is_baker: bool,
    rung_label: &str,
    is_assigned_job: bool,
    counts: &BakeCounts,
    runtime: &mut BakerProfessionRuntime,
    task: &mut BakerTaskState,
    peer_count_with_last_baker: f32,
    was_idle: f32,
    rng_pie_index: usize,
) -> Option<Goal> {
    let action = try_decide_baker_from_rung(
        profession_is_baker,
        rung_label,
        is_assigned_job,
        counts,
        runtime,
        task,
        peer_count_with_last_baker,
        was_idle,
        rng_pie_index,
    )?;
    Some(bake_action_to_goal(action))
}

// â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;

    fn counts_with(pairs: &[(i32, i32)], oven: Option<i32>, held: i32) -> BakeCounts {
        let mut c = BakeCounts {
            oven_parent_id: oven,
            held_id: held,
            held_uses: 0,
            ..Default::default()
        };
        for &(id, n) in pairs {
            c.set(id, n);
        }
        c
    }

    #[test]
    fn has_or_become_baker_max_and_sticky() {
        let mut rt = BakerProfessionRuntime::default();
        assert!(!has_or_become_baker(&mut rt, 1, 1.0, 0.0));
        assert!(!rt.is_last_baker);
        assert!(has_or_become_baker(&mut rt, 1, 0.0, 0.0));
        assert!(rt.is_last_baker);
        assert!(rt.stage >= 1.0);
        assert!(has_or_become_baker(&mut rt, 1, 5.0, 0.0));
        let mut rt2 = BakerProfessionRuntime::default();
        assert!(has_or_become_baker(&mut rt2, -2, 99.0, 0.0));
        assert!(!rt2.is_last_baker);
        // was_idle expands cap
        let mut rt3 = BakerProfessionRuntime::default();
        assert!(has_or_become_baker(&mut rt3, 1, 1.0, 1.0));
    }

    #[test]
    fn parse_baker_speech_and_assigned_job() {
        assert!(parse_baker_profession_speech("BAKER!"));
        assert!(parse_baker_profession_speech("baker"));
        assert!(parse_baker_profession_speech("  Baker!  "));
        assert!(!parse_baker_profession_speech("FARMER!"));
        let mut rt = BakerProfessionRuntime::default();
        assert!(!resolve_baker_assigned_job(&rt));
        rt.is_assigned_baker = true;
        assert!(resolve_baker_assigned_job(&rt));
        rt.is_assigned_baker = false;
        rt.is_last_baker = true;
        assert!(resolve_baker_assigned_job(&rt));
    }

    #[test]
    fn oven_priority_hot_then_burning_then_cold() {
        assert_eq!(
            pick_oven_parent(&[ADOBE_OVEN, BURNING_OVEN, HOT_OVEN]),
            Some(HOT_OVEN)
        );
        assert_eq!(
            pick_oven_parent(&[ADOBE_OVEN, BURNING_OVEN]),
            Some(BURNING_OVEN)
        );
        assert_eq!(pick_oven_parent(&[WOOD_FILLED_OVEN]), Some(WOOD_FILLED_OVEN));
        assert_eq!(pick_oven_parent(&[314, 303]), None);
        assert!(is_oven_id(237) && is_oven_id(250));
        assert!(!is_oven_id(314));
        assert_eq!(OvenState::from_parent(Some(HOT_OVEN)), OvenState::Hot);
        assert_eq!(OvenState::from_parent(Some(BURNING_OVEN)), OvenState::Burning);
        assert_eq!(OvenState::from_parent(Some(ADOBE_OVEN)), OvenState::Cold);
        assert_eq!(OvenState::from_parent(None), OvenState::None);
    }

    #[test]
    fn max_dough_and_pre_profession_dough() {
        assert_eq!(max_dough_in_bowl(false, 0), 0);
        assert_eq!(max_dough_in_bowl(true, 0), 1);
        assert_eq!(max_dough_in_bowl(true, 2), 0);

        let mut c = counts_with(&[(CLAY_PLATE, 1)], Some(ADOBE_OVEN), BOWL_OF_DOUGH);
        c.held_uses = 2;
        c.set(KNIFE, 1);
        // bread family 0 â†’ max 1; uses 2 > 1 â†’ plate dough
        assert_eq!(
            pre_profession_dough(&c),
            BakeAction::ShortCraft {
                actor: BOWL_OF_DOUGH,
                target: CLAY_PLATE
            }
        );

        // no knife, dough present, crust low â†’ pie crust
        let c = counts_with(&[(BOWL_OF_DOUGH, 1), (RAW_PIE_CRUST, 0)], Some(ADOBE_OVEN), 0);
        assert_eq!(
            pre_profession_dough(&c),
            BakeAction::CraftItem {
                object_id: RAW_PIE_CRUST
            }
        );
    }

    #[test]
    fn needed_raw_and_fire_oven() {
        assert_eq!(needed_raw_to_fire_oven(false, true), 4);
        assert_eq!(needed_raw_to_fire_oven(true, true), 1);
        assert_eq!(needed_raw_to_fire_oven(false, false), 1);

        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        // enough raw pies, no oven â†’ fire burning after become
        let c = counts_with(&[(RAW_BERRY_PIE, 4)], None, 0);
        assert_eq!(
            do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0),
            BakeAction::CraftItem {
                object_id: BURNING_OVEN
            }
        );
        assert!(rt.is_last_baker);
    }

    #[test]
    fn hot_oven_bakes_rotated_raw_pie() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 1.0,
            last_pie: 0,
            ..Default::default()
        };
        let c = counts_with(&[(RAW_CARROT_PIE, 1)], Some(HOT_OVEN), 0);
        // last_pie=0 â†’ start at 0; berry missing, mutton missing, carrot at index 2
        let a = hot_oven_bake(&c, &mut rt, 0);
        assert_eq!(
            a,
            BakeAction::ShortCraft {
                actor: RAW_CARROT_PIE,
                target: HOT_OVEN
            }
        );
        assert_eq!(rt.stage, 2.0);
        assert_eq!(rt.last_pie, 2); // rawPies[2] == 268
    }

    #[test]
    fn knife_bread_stage_crafts_leavened_when_low() {
        let rt = BakerProfessionRuntime {
            stage: 2.0,
            is_last_baker: true,
            ..Default::default()
        };
        let c = counts_with(
            &[(KNIFE, 1), (SLICED_BREAD, 0), (LEAVENED_DOUGH_PLATE, 0)],
            Some(ADOBE_OVEN),
            0,
        );
        // tomato first when stage < 3 (chopped tomato craft before bread path)
        let a = knife_bread_stage(&c, &rt);
        assert_eq!(
            a,
            BakeAction::CraftItem {
                object_id: CHOPPED_TOMATO_PLATE
            }
        );
        // with tomato stocked â†’ leavened dough plate
        let c = counts_with(
            &[
                (KNIFE, 1),
                (CHOPPED_TOMATO_PLATE, 1),
                (SLICED_BREAD, 0),
                (LEAVENED_DOUGH_PLATE, 0),
            ],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            knife_bread_stage(&c, &rt),
            BakeAction::CraftItem {
                object_id: LEAVENED_DOUGH_PLATE
            }
        );
    }

    #[test]
    fn make_raw_pies_hysteresis_and_rotation() {
        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        // cooked < min â†’ enter
        let c = counts_with(&[(COOKED_BERRY_PIE, 0)], Some(ADOBE_OVEN), 0);
        let a = make_raw_pies(&c, &mut rt, &mut task, 2, 5, 0);
        assert!(task.make_raw_pies >= 1.0);
        assert!(a.is_some());
        // cooked >= max â†’ exit
        let c = counts_with(
            &[
                (COOKED_BERRY_PIE, 2),
                (COOKED_MUTTON_PIE, 2),
                (COOKED_CARROT_PIE, 2),
            ],
            Some(ADOBE_OVEN),
            0,
        );
        task.make_raw_pies = 1.0;
        assert_eq!(
            make_raw_pies(&c, &mut rt, &mut task, 2, 5, 0),
            BakeAction::None
        );
        assert_eq!(task.make_raw_pies, 0.0);
    }

    #[test]
    fn do_baking_aborts_without_oven_when_no_raw_fire() {
        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        let c = counts_with(&[], None, 0);
        // Become baker then abort no oven
        assert_eq!(
            do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0),
            BakeAction::Abort
        );
        assert!(rt.is_last_baker);
        assert_eq!(rt.stage, 0.0);
    }

    #[test]
    fn do_baking_defers_pottery_without_plates() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 1.0,
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        let c = counts_with(&[], Some(ADOBE_OVEN), 0);
        assert_eq!(
            do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0),
            BakeAction::DeferPottery
        );
    }

    #[test]
    fn do_baking_hot_path_and_peer_cap() {
        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        // Peer baker blocks
        let c = counts_with(&[(RAW_BERRY_PIE, 1), (CLAY_PLATE, 1)], Some(HOT_OVEN), 0);
        assert_eq!(
            do_baking(&c, &mut rt, &mut task, 1, 1.0, 0.0, 0),
            BakeAction::Abort
        );
        // Become + bake pie into hot oven
        let mut rt = BakerProfessionRuntime::default();
        let a = do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0);
        assert_eq!(
            a,
            BakeAction::ShortCraft {
                actor: RAW_BERRY_PIE,
                target: HOT_OVEN
            }
        );
        assert_eq!(rt.stage, 2.0);
    }

    #[test]
    fn pick_baker_goal_stage_and_pipeline() {
        let g = ReverseCraftGraph::default();
        let empty = HashSet::new();
        let goal = pick_baker_goal(&g, &empty, 0.0);
        assert_eq!(goal, Goal::SeekObject(CLAY_PLATE));
        let have: HashSet<i32> = [CLAY_PLATE].into_iter().collect();
        assert_eq!(
            pick_baker_goal(&g, &have, 1.5),
            Goal::SeekObject(RAW_PIE_CRUST)
        );
        let mut have = HashSet::new();
        for &id in baker_pipeline_targets() {
            if id != COOKED_CARROT_PIE {
                have.insert(id);
            }
        }
        // All pipeline except default target â€” may still seek early missing
        // With everything in pipeline present except we removed cooked carrot â€”
        // many targets still in list after cooked carrot. Insert all:
        have.insert(COOKED_CARROT_PIE);
        assert_eq!(
            pick_baker_goal(&g, &have, 99.0),
            Goal::SeekObject(BAKER_TARGET_ID)
        );
    }

    #[test]
    fn bake_action_to_goal_maps() {
        assert_eq!(
            bake_action_to_goal(BakeAction::CraftItem {
                object_id: RAW_BERRY_PIE
            }),
            Goal::SeekObject(RAW_BERRY_PIE)
        );
        assert_eq!(
            bake_action_to_goal(BakeAction::ShortCraft {
                actor: 265,
                target: HOT_OVEN
            }),
            Goal::SeekObject(HOT_OVEN)
        );
        assert_eq!(
            bake_action_to_goal(BakeAction::DeferPottery),
            Goal::SeekObject(CLAY_PLATE)
        );
        assert_eq!(
            bake_action_to_goal(BakeAction::Abort),
            Goal::SeekObject(BAKER_TARGET_ID)
        );
    }

    #[test]
    fn raw_cooked_pies_parallel_length() {
        assert_eq!(RAW_PIES.len(), COOKED_PIES.len());
        assert_eq!(RAW_PIES[0], RAW_BERRY_PIE);
        assert_eq!(COOKED_PIES[0], COOKED_BERRY_PIE);
        assert_eq!(RAW_PIES[1], RAW_MUTTON_PIE);
        assert_eq!(COOKED_PIES[1], COOKED_MUTTON_PIE);
        assert_eq!(RAW_PIES[2], RAW_CARROT_PIE);
        assert_eq!(COOKED_PIES[2], COOKED_CARROT_PIE);
    }

    #[test]
    fn mid_pipeline_mutton_and_wheat_gate() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 3.0,
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        // turkey slice missing first
        let c = counts_with(&[(CLAY_PLATE, 1)], Some(ADOBE_OVEN), 0);
        assert_eq!(
            mid_bake_pipeline(&c, &mut rt, &mut task, 0),
            BakeAction::CraftItem {
                object_id: TURKEY_SLICE_PLATE
            }
        );
        // stocked turkey + carrots + kindling + no mutton pies â†’ raw mutton pie
        // (carrots stocked high so plant_carrots hysteresis stays off)
        let c = counts_with(
            &[
                (CLAY_PLATE, 1),
                (TURKEY_SLICE_PLATE, 1),
                (CARROT, 12),
                (KINDLING, 5),
                (RAW_POTATO, 5),
            ],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            mid_bake_pipeline(&c, &mut rt, &mut task, 0),
            BakeAction::CraftItem {
                object_id: RAW_MUTTON_PIE
            }
        );
    }

    #[test]
    fn handle_milk_order_when_stock_present() {
        // Empty â†’ none (stock-gated)
        let c = counts_with(&[], Some(HOT_OVEN), 0);
        assert_eq!(handle_milk(&c), BakeAction::None);
        // Cream present â†’ still fill milk pouch first when under 3
        let c = counts_with(&[(BOWL_OF_CREAM, 1)], Some(HOT_OVEN), 0);
        assert_eq!(
            handle_milk(&c),
            BakeAction::CraftItem {
                object_id: MILK_POUCH
            }
        );
        // Pouch stocked â†’ skewer cream
        let c = counts_with(&[(MILK_POUCH, 3), (BOWL_OF_CREAM, 1)], Some(HOT_OVEN), 0);
        assert_eq!(
            handle_milk(&c),
            BakeAction::ShortCraft {
                actor: SKEWER,
                target: BOWL_OF_CREAM
            }
        );
        // Butter path â†’ buttered bread
        let c = counts_with(&[(MILK_POUCH, 3), (BOWL_OF_BUTTER, 1)], Some(HOT_OVEN), 0);
        assert_eq!(
            handle_milk(&c),
            BakeAction::CraftItem {
                object_id: BUTTERED_BREAD
            }
        );
    }

    #[test]
    fn make_or_collect_kindling_hysteresis() {
        let mut flag = 0.0;
        assert_eq!(make_or_collect(KINDLING, 1, 5, 0, &mut flag), BakeAction::CraftItem { object_id: KINDLING });
        assert!(flag >= 1.0);
        assert_eq!(make_or_collect(KINDLING, 1, 5, 5, &mut flag), BakeAction::None);
        assert_eq!(flag, 0.0);
        // between min and max with flag off â†’ none
        assert_eq!(make_or_collect(KINDLING, 1, 5, 3, &mut flag), BakeAction::None);
        // drop to min â†’ re-enter
        assert_eq!(make_or_collect(KINDLING, 1, 5, 1, &mut flag), BakeAction::CraftItem { object_id: KINDLING });
    }

    #[test]
    fn extra_pies_uses_count_pies_not_cooked_sum() {
        let mut rt = BakerProfessionRuntime {
            count_pies: 0, // %4==0 â†’ mutton pie bias
            ..Default::default()
        };
        let mut task = BakerTaskState {
            make_raw_pies: 1.0,
            ..Default::default()
        };
        // cooked pies in hysteresis band (2..5)
        let c = counts_with(
            &[(COOKED_BERRY_PIE, 2), (COOKED_CARROT_PIE, 0), (COOKED_MUTTON_PIE, 0)],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            make_raw_pies(&c, &mut rt, &mut task, 2, 5, 0),
            BakeAction::CraftItem {
                object_id: RAW_MUTTON_PIE
            }
        );
        note_raw_pie_crafted(&mut rt, RAW_MUTTON_PIE);
        assert_eq!(rt.count_pies, 1);
        // count_pies=2 â†’ %4==2 â†’ carrot bias
        rt.count_pies = 2;
        let c = counts_with(
            &[(COOKED_BERRY_PIE, 2), (COOKED_CARROT_PIE, 0), (COOKED_MUTTON_PIE, 2)],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            make_raw_pies(&c, &mut rt, &mut task, 2, 5, 0),
            BakeAction::CraftItem {
                object_id: RAW_CARROT_PIE
            }
        );
    }

    #[test]
    fn bake_counts_from_nearby_picks_hot_oven() {
        let c = bake_counts_from_nearby(
            &[(ADOBE_OVEN, 1), (HOT_OVEN, 1), (RAW_BERRY_PIE, 2), (CLAY_PLATE, 1)],
            0,
            0,
            false,
            false,
            false,
        );
        assert_eq!(c.oven_parent_id, Some(HOT_OVEN));
        assert_eq!(c.get(RAW_BERRY_PIE), 2);
        assert!(c.has_close_plate());
    }

    #[test]
    fn should_drop_near_oven_ids() {
        assert!(should_drop_near_oven(CLAY_PLATE));
        assert!(should_drop_near_oven(RAW_BERRY_PIE));
        assert!(should_drop_near_oven(COOKED_CARROT_PIE));
        assert!(!should_drop_near_oven(KNIFE));
        assert!(!should_drop_near_oven(0));
    }

    #[test]
    fn craft_turkey_broth_when_turkey_family() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 2.0,
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        // Hot oven empty bake path falls through â†’ turkey broth when family present
        let c = counts_with(
            &[(COOKED_TURKEY_PLATE, 1), (CLAY_PLATE, 1)],
            Some(HOT_OVEN),
            0,
        );
        let a = do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0);
        assert_eq!(
            a,
            BakeAction::CraftItem {
                object_id: TURKEY_BROTH
            }
        );
    }

    #[test]
    fn plant_carrots_and_kindling_in_mid() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 3.0,
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        let c = counts_with(
            &[(TURKEY_SLICE_PLATE, 1), (CLAY_PLATE, 1)],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            mid_bake_pipeline(&c, &mut rt, &mut task, 0),
            BakeAction::CraftItem {
                object_id: DRY_PLANTED_CARROTS
            }
        );
        // carrots stocked, kindling low
        let c = counts_with(
            &[
                (TURKEY_SLICE_PLATE, 1),
                (CLAY_PLATE, 1),
                (CARROT, 12),
                (KINDLING, 0),
            ],
            Some(ADOBE_OVEN),
            0,
        );
        assert_eq!(
            mid_bake_pipeline(&c, &mut rt, &mut task, 0),
            BakeAction::CraftItem {
                object_id: KINDLING
            }
        );
    }

    #[test]
    fn try_decide_baker_from_rung_assigned_max() {
        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        let c = counts_with(&[(RAW_BERRY_PIE, 4)], None, 0);
        let a = try_decide_baker_from_rung(
            true,
            "ASSIGNED_JOB",
            true,
            &c,
            &mut rt,
            &mut task,
            0.0,
            0.0,
            0,
        );
        assert_eq!(
            a,
            Some(BakeAction::CraftItem {
                object_id: BURNING_OVEN
            })
        );
        assert_eq!(baker_max_people_for_dispatch(true, false), BAKER_ASSIGNED_MAX_PEOPLE);
        assert_eq!(baker_max_people_for_dispatch(false, true), 2);
        assert!(!baker_job_rung_label("ESCAPE"));
    }

    #[test]
    fn infer_stage_and_needed_raw_plate_gate() {
        let mut have = HashSet::new();
        assert_eq!(infer_baker_pipeline_stage(&have), 0.0);
        have.insert(CLAY_PLATE);
        assert_eq!(infer_baker_pipeline_stage(&have), 1.5);
        have.insert(RAW_PIE_CRUST);
        assert_eq!(infer_baker_pipeline_stage(&have), 2.0);
        // plate-hungry still fires with need=1 when cold adobe present
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        let c = counts_with(&[(RAW_BERRY_PIE, 1)], Some(ADOBE_OVEN), 0);
        // no plate â†’ neededRaw=1, raw>=1 â†’ burning
        assert_eq!(
            do_baking(&c, &mut rt, &mut task, 1, 0.0, 0.0, 0),
            BakeAction::CraftItem {
                object_id: BURNING_OVEN
            }
        );
    }

    // â”€â”€ AI-JOB-BAKER-WIRE â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn fill_bake_counts_hot_oven_beats_burning_and_radii() {
        // Hot within r=20 wins over closer burning; far hot ignored.
        // Haxe: GetClosestObjectToPosition priority 250 then 249; pie crust r=30
        let objs = [
            BakeMapObj::simple(BURNING_OVEN, 2, 0),
            BakeMapObj::simple(HOT_OVEN, 10, 0),
            BakeMapObj::simple(HOT_OVEN, 40, 0), // far
            BakeMapObj::simple(RAW_PIE_CRUST, 25, 0), // within pie r=30, outside home_r=20
            BakeMapObj::simple(RAW_PIE_CRUST, 35, 0), // outside pie r=30
            BakeMapObj::simple(CLAY_PLATE, 5, 0),
            BakeMapObj::simple(CLAY_PLATE, 25, 0), // outside home_r=20
            BakeMapObj::simple(RAW_BERRY_PIE, 3, 0),
        ];
        let c = fill_bake_counts_from_map(0, 0, BOWL_OF_DOUGH, &objs, OVEN_SEARCH_RADIUS);
        assert_eq!(c.oven_parent_id, Some(HOT_OVEN));
        assert_eq!(c.held_id, BOWL_OF_DOUGH);
        assert_eq!(c.get(RAW_PIE_CRUST), 1); // only x=25 in pie radius
        assert_eq!(c.get(CLAY_PLATE), 1); // only x=5 in home_r
        assert_eq!(c.get(RAW_BERRY_PIE), 1);
        // adobe only when no hot/burning in range
        let cold = [
            BakeMapObj::simple(ADOBE_OVEN, 4, 0),
            BakeMapObj::simple(WOOD_FILLED_OVEN, 1, 0),
        ];
        let c2 = fill_bake_counts_from_map(0, 0, 0, &cold, 20);
        assert_eq!(c2.oven_parent_id, Some(ADOBE_OVEN));
    }

    #[test]
    fn pick_oven_near_home_radius_and_priority() {
        let cands = [
            OvenCandidate {
                parent_id: ADOBE_OVEN,
                x: 1,
                y: 0,
            },
            OvenCandidate {
                parent_id: BURNING_OVEN,
                x: 15,
                y: 0,
            },
            OvenCandidate {
                parent_id: HOT_OVEN,
                x: 25,
                y: 0,
            }, // beyond r=20
        ];
        assert_eq!(
            pick_oven_near_home(0, 0, &cands).map(|c| c.parent_id),
            Some(BURNING_OVEN)
        );
        assert_eq!(
            pick_oven_near_home_radius(0, 0, &cands, 30)
                .map(|c| c.parent_id),
            Some(HOT_OVEN)
        );
        assert_eq!(baker_radius_table()[0].0, OVEN_SEARCH_RADIUS);
    }

    #[test]
    fn assign_baker_from_speech_sticky() {
        let mut rt = BakerProfessionRuntime::default();
        assert!(!assign_baker_from_speech(&mut rt, "FARMER!"));
        assert!(assign_baker_from_speech(&mut rt, "BAKER!"));
        assert!(rt.is_assigned_baker);
        assert!(rt.is_last_baker);
        assert!(rt.stage >= 1.0);
    }

    #[test]
    fn baker_goal_from_map_and_rung_assigned_and_escape() {
        let objs = [
            BakeMapObj::simple(RAW_BERRY_PIE, 2, 0),
            BakeMapObj::simple(RAW_BERRY_PIE, 3, 0),
            BakeMapObj::simple(RAW_BERRY_PIE, 4, 0),
            BakeMapObj::simple(RAW_BERRY_PIE, 5, 0),
        ];
        let mut rt = BakerProfessionRuntime::default();
        let mut task = BakerTaskState::default();
        // ASSIGNED_JOB maxPeople=100 â†’ fire burning oven when raw stocked
        let g = baker_goal_from_map_and_rung(
            true,
            "ASSIGNED_JOB",
            true,
            0,
            0,
            0,
            0,
            &objs,
            20,
            &mut rt,
            &mut task,
            0.0,
            0.0,
            0,
            false,
            false,
            false,
        );
        assert_eq!(g, Some(Goal::SeekObject(BURNING_OVEN)));
        assert!(rt.is_last_baker);

        // ESCAPE not a baker rung â†’ None (thin SeekObject kept by caller)
        let mut rt2 = BakerProfessionRuntime {
            is_last_baker: true,
            ..Default::default()
        };
        let mut task2 = BakerTaskState::default();
        assert!(baker_goal_from_map_and_rung(
            true,
            "ESCAPE",
            false,
            0,
            0,
            0,
            0,
            &objs,
            20,
            &mut rt2,
            &mut task2,
            0.0,
            0.0,
            0,
            false,
            false,
            false,
        )
        .is_none());

        // peer cap aborts assigned path when peers >= max (default age-rotated max=1)
        let mut rt3 = BakerProfessionRuntime::default();
        let mut task3 = BakerTaskState::default();
        let g3 = baker_goal_from_counts_and_rung(
            true,
            "AGE_ROTATED_JOB",
            false,
            &fill_bake_counts_from_map(0, 0, 0, &objs, 20),
            &mut rt3,
            &mut task3,
            1.0, // peers already 1 â‰¥ max 1
            0.0,
            0,
        );
        assert_eq!(g3, Some(Goal::SeekObject(BAKER_TARGET_ID))); // Abort â†’ thin target
        assert!(!rt3.is_last_baker);
    }

    #[test]
    fn baker_goal_hot_path_short_craft_seek() {
        let objs = [
            BakeMapObj::simple(HOT_OVEN, 1, 0),
            BakeMapObj::simple(RAW_CARROT_PIE, 2, 0),
        ];
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 1.0,
            last_pie: 2, // start at carrot pie
            ..Default::default()
        };
        let mut task = BakerTaskState::default();
        let g = baker_goal_from_map_and_rung(
            true,
            "AGE_ROTATED_JOB",
            false,
            0,
            0,
            0,
            0,
            &objs,
            20,
            &mut rt,
            &mut task,
            0.0,
            0.0,
            2,
            false,
            false,
            false,
        );
        // ShortCraft(raw, 250) â†’ SeekObject(HOT_OVEN)
        assert_eq!(g, Some(Goal::SeekObject(HOT_OVEN)));
        assert_eq!(rt.stage, 2.0);
    }

    #[test]
    fn fill_bake_counts_ex_hungry_and_held_uses() {
        let objs = [BakeMapObj::simple(CLAY_PLATE, 0, 0)];
        let c = fill_bake_counts_from_map_ex(
            0,
            0,
            BOWL_OF_DOUGH,
            2,
            &objs,
            20,
            true,
            true,
            true,
            0,
        );
        assert_eq!(c.held_id, BOWL_OF_DOUGH);
        assert_eq!(c.held_uses, 2);
        assert!(c.is_hungry);
        assert!(c.has_corn_seeds);
        assert!(c.has_bean_seeds);
        assert_eq!(c.get(CLAY_PLATE), 1);
    }

    // â”€â”€ AI-JOB-BAKER-LIVE â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn fill_bake_counts_pile_uses_and_ignored_floor() {
        // Pile of plates (uses=5) counts as 5; ignored floor skips non-food non-permanent.
        // Haxe: CountCloseObjects pile +uses; IsIgnoredFloor origin floor
        let objs = [
            BakeMapObj::pile(CLAY_PLATE, 1, 0, 5),
            BakeMapObj::simple(RAW_BERRY_PIE, 2, 0),
            BakeMapObj::simple(RAW_MUTTON, 3, 0).with_floor(AI_IGNORED_FLOOR_IDS[0]),
            BakeMapObj::simple(COOKED_CARROT_PIE, 4, 0)
                .with_floor(AI_IGNORED_FLOOR_IDS[0])
                .food(),
        ];
        // No origin floor â†’ all counted (tile floor not used in bulk fill)
        let c = fill_bake_counts_from_map(0, 0, 0, &objs, 20);
        assert_eq!(c.get(CLAY_PLATE), 5);
        assert_eq!(c.get(RAW_BERRY_PIE), 1);
        assert_eq!(c.get(RAW_MUTTON), 1);
        assert_eq!(c.get(COOKED_CARROT_PIE), 1);

        // Origin on ignored floor â†’ skip non-food non-permanent
        let c2 = fill_bake_counts_from_map_with_floor(0, 0, 0, &objs, 20, AI_IGNORED_FLOOR_IDS[0]);
        assert_eq!(c2.get(CLAY_PLATE), 0);
        assert_eq!(c2.get(RAW_BERRY_PIE), 0);
        assert_eq!(c2.get(RAW_MUTTON), 0);
        assert_eq!(c2.get(COOKED_CARROT_PIE), 1); // food survives
    }

    #[test]
    fn fill_bake_counts_half_open_square_edge() {
        // Haxe half-open: tile at home+radius excluded; home-radius included.
        let at_high = [BakeMapObj::simple(CLAY_PLATE, 20, 0)];
        let at_low = [BakeMapObj::simple(CLAY_PLATE, -20, 0)];
        assert_eq!(
            fill_bake_counts_from_map(0, 0, 0, &at_high, 20).get(CLAY_PLATE),
            0
        );
        assert_eq!(
            fill_bake_counts_from_map(0, 0, 0, &at_low, 20).get(CLAY_PLATE),
            1
        );
    }

    #[test]
    fn hot_oven_raw_mutton_max_new_actor_four() {
        let mut rt = BakerProfessionRuntime {
            is_last_baker: true,
            stage: 2.0,
            last_pie: 0,
            ..Default::default()
        };
        // raw mutton present, cooked stock under cap â†’ bake mutton
        let c = counts_with(&[(RAW_MUTTON, 1), (COOKED_MUTTON, 2)], Some(HOT_OVEN), 0);
        assert_eq!(
            hot_oven_bake(&c, &mut rt, 0),
            BakeAction::ShortCraft {
                actor: RAW_MUTTON,
                target: HOT_OVEN
            }
        );
        // cooked/newActor at cap â†’ skip mutton, fall through to potato if present
        let c2 = counts_with(
            &[(RAW_MUTTON, 1), (COOKED_MUTTON, 4), (RAW_POTATO, 1)],
            Some(HOT_OVEN),
            0,
        );
        assert_eq!(
            hot_oven_bake(&c2, &mut rt, 0),
            BakeAction::ShortCraft {
                actor: RAW_POTATO,
                target: HOT_OVEN
            }
        );
        // limits table
        assert_eq!(
            baker_short_craft_limits(RAW_MUTTON, HOT_OVEN),
            (RAW_MUTTON_HOT_OVEN_MAX_NEW_ACTOR, false)
        );
    }

    #[test]
    fn bake_action_short_craft_apply_edges() {
        // held match â†’ USE
        let on_target = bake_action_short_craft_apply(
            BakeAction::ShortCraft {
                actor: RAW_CARROT_PIE,
                target: HOT_OVEN,
            },
            RAW_CARROT_PIE,
            0,
            -1,
        );
        assert_eq!(
            on_target,
            Some(ShortCraftApply::UseOnTarget {
                actor: RAW_CARROT_PIE,
                target: HOT_OVEN
            })
        );
        // actor 0 â†’ DROP
        let drop = baker_short_craft_apply(CLAY_PLATE, 0, COOKED_TURKEY, 0);
        assert_eq!(drop, ShortCraftApply::DropHeld);
        // held mismatch â†’ seek/craft actor
        let seek = baker_short_craft_apply(0, RAW_MUTTON, HOT_OVEN, 0);
        assert_eq!(
            seek,
            ShortCraftApply::SeekOrCraftActor {
                actor: RAW_MUTTON,
                craft_if_needed: false, // muttonâ†’hot oven: craftActor=false
            }
        );
        // maxNewActor refuse when stock high
        let refused = baker_short_craft_apply(RAW_MUTTON, RAW_MUTTON, HOT_OVEN, 4);
        assert_eq!(refused, ShortCraftApply::Refuse);
        // under cap still USE
        let ok = baker_short_craft_apply(RAW_MUTTON, RAW_MUTTON, HOT_OVEN, 3);
        assert_eq!(
            ok,
            ShortCraftApply::UseOnTarget {
                actor: RAW_MUTTON,
                target: HOT_OVEN
            }
        );
        // non-ShortCraft â†’ None
        assert!(bake_action_short_craft_apply(
            BakeAction::CraftItem {
                object_id: RAW_PIE_CRUST
            },
            0,
            0,
            -1
        )
        .is_none());
    }

    #[test]
    fn count_baker_peers_filtered_age_wound_home_follow() {
        let peers = [
            BakerPeerSnapshot {
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_baker: true,
            },
            BakerPeerSnapshot {
                deleted: false,
                age: 2.0, // too young
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_baker: true,
            },
            BakerPeerSnapshot {
                deleted: false,
                age: 25.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: false,
                last_is_baker: true,
            },
            BakerPeerSnapshot {
                deleted: false,
                age: 30.0,
                is_wounded: true,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_baker: true,
            },
            BakerPeerSnapshot {
                deleted: false,
                age: 22.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: true,
                same_home: true,
                last_is_baker: true,
            },
            BakerPeerSnapshot {
                deleted: false,
                age: 28.0,
                is_wounded: false,
                food_store: -1.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_baker: true,
            },
        ];
        assert_eq!(count_baker_peers_filtered(&peers, 3.0, 60.0), 1.0);

        let mut rt = BakerProfessionRuntime::default();
        assert!(!has_or_become_baker_filtered(
            &mut rt, 1, &peers, 3.0, 60.0, 0.0
        )); // 1 peer already
        assert!(!rt.is_last_baker);
        let empty: [BakerPeerSnapshot; 0] = [];
        assert!(has_or_become_baker_filtered(
            &mut rt, 1, &empty, 3.0, 60.0, 0.0
        ));
        assert!(rt.is_last_baker);
    }

    #[test]
    fn make_seats_and_cleanup_ex_tomato_and_bowl_filler() {
        // tomato present â†’ early exit
        let c = counts_with(&[(BOWL_TOMATO_SEEDS, 1)], Some(ADOBE_OVEN), 0);
        assert_eq!(make_seats_and_cleanup_ex(&c, true, true), BakeAction::None);
        // force without bowl filler â†’ defer
        let empty = counts_with(&[], Some(ADOBE_OVEN), 0);
        assert_eq!(
            make_seats_and_cleanup_ex(&empty, true, false),
            BakeAction::DeferSeatsCleanup
        );
        // bowl filler allowed â†’ craft 2828
        assert_eq!(
            make_seats_and_cleanup_ex(&empty, false, true),
            BakeAction::CraftItem {
                object_id: BOWL_TOMATO_SEEDS
            }
        );
        // hungry â†’ none even with force
        let mut hungry = empty;
        hungry.is_hungry = true;
        assert_eq!(
            make_seats_and_cleanup_ex(&hungry, true, true),
            BakeAction::None
        );
    }

    #[test]
    fn consider_drop_near_oven_anchor() {
        assert_eq!(
            consider_drop_near_oven(CLAY_PLATE, 10, 20, Some((3, 4))),
            Some((3, 4))
        );
        assert_eq!(
            consider_drop_near_oven(RAW_BERRY_PIE, 10, 20, None),
            Some((10, 20))
        );
        assert_eq!(consider_drop_near_oven(KNIFE, 10, 20, Some((1, 1))), None);
        assert_eq!(drop_near_oven_anchor(0, 0, Some((5, 6))), (5, 6));
        assert_eq!(BAKING_SHORTCRAFT_RADIUS, BAKING_CRAFT_SEARCH_RADIUS);
    }
}
