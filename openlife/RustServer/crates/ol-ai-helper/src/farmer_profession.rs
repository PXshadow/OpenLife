//! Haxe: `AiBase` farmer profession family (chunk **AI-JOB-FARM** / **AI-JOB-FARM-LIVE**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession` / sticky `lastProfession` + [`Player`](crate::Player) fields
//! - Speech `FARMER!` / `WHEAT!` / `CARROT!` â†’ [`assign_farm_from_speech`]
//! - `keepBushesAlive` / `doPlant` / harvest / water / soil / rows / compost hysteresis
//! - Job sequences: basic / carrot / berry / advanced farming
//! - Pure [`short_craft_apply`] edges for ShortCraft USE/drop/seek
//!
//! No world I/O: callers supply counts and apply returned [`FarmAction`]s
//! via craft/shortCraft (AI-CRAFT) and spatial helpers (AiHelper port).

use std::collections::HashMap;

use crate::ai_goals::priority_ladder::age_job_index;
use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use crate::ai_goals::{Goal, FARMER_TARGET_ID};
use std::collections::HashSet;

// â”€â”€ Object ids (OHOL / OpenLife content; Haxe comments) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Bowl of Water (watering actor).
pub const BOWL_OF_WATER: i32 = 382;
/// Dry Planted Carrots.
pub const DRY_PLANTED_CARROTS: i32 = 396;
/// Wet Planted Carrots.
pub const WET_PLANTED_CARROTS: i32 = 399;
/// Carrot (pulled).
pub const CARROT: i32 = 402;
/// Dry Planted Wheat.
pub const DRY_PLANTED_WHEAT: i32 = 228;
/// Wet Planted Wheat.
pub const WET_PLANTED_WHEAT: i32 = 229;
/// Wheat Sprouts.
pub const WHEAT_SPROUTS: i32 = 230;
/// Unripe Wheat.
pub const UNRIPE_WHEAT: i32 = 243;
/// Ripe Wheat (self-play default farmer target).
pub const RIPE_WHEAT: i32 = 242;
/// Harvested Wheat.
pub const HARVESTED_WHEAT: i32 = 224;
/// Wheat Sheaf.
pub const WHEAT_SHEAF: i32 = 225;
/// Threshed Wheat.
pub const THRESHED_WHEAT: i32 = 226;
/// Threshed Wheat (on ground).
pub const THRESHED_WHEAT_GROUND: i32 = 4069;
/// Dry Planted Corn Seed.
pub const DRY_PLANTED_CORN: i32 = 1109;
/// Wet Planted Corn Seed.
pub const WET_PLANTED_CORN: i32 = 1110;
/// Corn Sprout.
pub const CORN_SPROUT: i32 = 1111;
/// Corn Plant.
pub const CORN_PLANT: i32 = 1112;
/// Ear of Corn.
pub const EAR_OF_CORN: i32 = 1113;
/// Shucked Ear of Corn.
pub const SHUCKED_CORN: i32 = 1114;
/// Dried Ear of Corn.
pub const DRIED_CORN: i32 = 1115;
/// Pile of Dried Corn.
pub const PILE_DRIED_CORN: i32 = 3902;
/// Sharp Stone (shuck corn / makeSharpieFood).
pub const SHARP_STONE: i32 = 34;
/// Seeding Wild Carrot (makeSharpieFood).
// Haxe: AiBase.makeSharpieFood ~4108
pub const SEEDING_WILD_CARROT: i32 = 36;
/// Dug Wild Carrot product.
// Haxe: AiBase.makeSharpieFood ~4111
pub const DUG_WILD_CARROT: i32 = 39;
/// Burdock plant.
// Haxe: AiBase.makeSharpieFood ~4113
pub const BURDOCK: i32 = 804;
/// Dug Burdock product.
// Haxe: AiBase.makeSharpieFood ~4116
pub const DUG_BURDOCK: i32 = 806;
/// Dry Planted Gooseberry Seed.
pub const DRY_PLANTED_GOOSEBERRY: i32 = 216;
/// Wet Planted Gooseberry Seed.
pub const WET_PLANTED_GOOSEBERRY: i32 = 217;
/// Gooseberry Sprout.
pub const GOOSEBERRY_SPROUT: i32 = 219;
/// Domestic Gooseberry Bush.
pub const DOMESTIC_BUSH: i32 = 391;
/// Dry Domestic Gooseberry Bush.
pub const DRY_DOMESTIC_BUSH: i32 = 393;
/// Empty Domestic Gooseberry Bush.
pub const EMPTY_DOMESTIC_BUSH: i32 = 1135;
/// Vigorous Domestic Gooseberry Bush.
pub const VIGOROUS_DOMESTIC_BUSH: i32 = 1134;
/// Dying Gooseberry Bush.
pub const DYING_BUSH: i32 = 389;
/// Languishing Domestic Gooseberry Bush.
pub const LANGUISHING_BUSH: i32 = 392;
/// Bowl of Soil.
pub const BOWL_OF_SOIL: i32 = 1137;
/// Fertile Soil Pile.
pub const FERTILE_SOIL_PILE: i32 = 1101;
/// Fertile Soil.
pub const FERTILE_SOIL: i32 = 1138;
/// Deep Tilled Row.
pub const DEEP_TILLED_ROW: i32 = 213;
/// Shallow Tilled Row.
pub const SHALLOW_TILLED_ROW: i32 = 1136;
/// Hardened Row.
pub const HARDENED_ROW: i32 = 848;
/// Basket of Soil.
pub const BASKET_OF_SOIL: i32 = 336;
/// Basket (empty).
pub const BASKET: i32 = 292;
/// Composting Compost Pile.
pub const COMPOSTING_PILE: i32 = 790;
/// Composted Soil.
pub const COMPOSTED_SOIL: i32 = 624;
/// Wet Compost Pile.
pub const WET_COMPOST: i32 = 625;
/// Shovel of Dung.
pub const SHOVEL_OF_DUNG: i32 = 900;
/// Clay Bowl.
pub const CLAY_BOWL: i32 = 235;
/// Steel Hoe.
pub const STEEL_HOE: i32 = 857;
/// Stone Hoe.
pub const STONE_HOE: i32 = 850;
/// Skewer.
pub const SKEWER: i32 = 139;
/// Weak Skewer (Haxe shortCraft prefers this when actor is Skewer 139).
// Haxe: AiBase.shortCraftOnTarget ~2730
pub const WEAK_SKEWER: i32 = 852;
/// Carrot Row / seeding carrots pull target.
// Haxe: AiBase shortCraft carrot-seed guard target 400
pub const CARROT_ROW: i32 = 400;
/// Tomato Sprout.
pub const TOMATO_SPROUT: i32 = 2832;
/// Cucumber Sprout.
pub const CUCUMBER_SPROUT: i32 = 4228;
/// Hardened Row with Stake.
pub const HARDENED_ROW_STAKE: i32 = 2837;
/// Shovel.
pub const SHOVEL: i32 = 502;
/// Mature Potato Plants.
pub const MATURE_POTATO: i32 = 1146;
/// Potato Plants.
pub const POTATO_PLANTS: i32 = 1143;
/// Dry Planted Potatoes.
pub const DRY_PLANTED_POTATO: i32 = 1145;
/// Wet Planted Potatoes.
pub const WET_PLANTED_POTATO: i32 = 1142;
/// Mounded Potato Plants.
pub const MOUNDED_POTATO: i32 = 1144;
/// Dug Potatoes.
pub const DUG_POTATO: i32 = 4144;
/// Dry Planted Beans.
pub const DRY_PLANTED_BEANS: i32 = 1161;
/// Wet Planted Beans.
pub const WET_PLANTED_BEANS: i32 = 1162;
/// Green Bean Plants.
pub const GREEN_BEAN_PLANTS: i32 = 1173;
/// Dry Bean Plants.
pub const DRY_BEAN_PLANTS: i32 = 1172;
/// Dry Planted Tomato Seed.
pub const DRY_PLANTED_TOMATO: i32 = 2829;
/// Tomato Plant.
pub const TOMATO_PLANT: i32 = 2834;
/// Fruiting Tomato Plant.
pub const FRUITING_TOMATO: i32 = 2835;
/// Dry Planted Cucumber Seeds.
pub const DRY_PLANTED_CUCUMBER: i32 = 4225;
/// Wet Planted Cucumber Seeds.
pub const WET_PLANTED_CUCUMBER: i32 = 4226;
/// Ripe Cucumber Plant.
pub const RIPE_CUCUMBER: i32 = 4232;
/// Dry Planted Pepper Seed.
pub const DRY_PLANTED_PEPPER: i32 = 2839;
/// Wet Planted Pepper Seed.
pub const WET_PLANTED_PEPPER: i32 = 2840;
/// Pepper Plant.
pub const PEPPER_PLANT: i32 = 2842;
/// Fruiting Pepper Plant.
pub const FRUITING_PEPPER: i32 = 2843;
/// Dry Planted Onions.
pub const DRY_PLANTED_ONIONS: i32 = 2851;
/// Wet Planted Onions.
pub const WET_PLANTED_ONIONS: i32 = 2852;
/// Ripe Onions.
pub const RIPE_ONIONS: i32 = 2854;
/// Dry Planted Squash Seeds.
pub const DRY_PLANTED_SQUASH: i32 = 1192;
/// Snow biome id (Haxe `BiomeTag.SNOW`).
pub const SNOW_BIOME: u8 = 4;
/// Ocean biome id (Haxe `BiomeTag.OCEAN`).
pub const OCEAN_BIOME: u8 = 9;

/// Home-radius used by most farm count helpers (Haxe 30).
pub const FARM_HOME_RADIUS: i32 = 30;

// â”€â”€ Profession keys â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `profession` / `assignedProfession` / `lastProfession` farm keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FarmProfession {
    BasicFarmer,
    CarrotFarmer,
    BerryFarmer,
    AdvancedFarmer,
    SoilMaker,
    RowMaker,
    WaterBringer,
}

impl FarmProfession {
    /// Canonical Haxe string key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BasicFarmer => "BASICFARMER",
            Self::CarrotFarmer => "CARROTFARMER",
            // Haxe uses mixed-case `BerryFarmer` for this key.
            Self::BerryFarmer => "BerryFarmer",
            Self::AdvancedFarmer => "ADVANCEDFARMER",
            Self::SoilMaker => "SOILMAKER",
            Self::RowMaker => "ROWMAKER",
            Self::WaterBringer => "WATERBRINGER",
        }
    }

    /// Parse Haxe profession key (case-sensitive for BerryFarmer; others upper).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "BASICFARMER" => Some(Self::BasicFarmer),
            "CARROTFARMER" => Some(Self::CarrotFarmer),
            "BerryFarmer" | "BERRYFARMER" => Some(Self::BerryFarmer),
            "ADVANCEDFARMER" => Some(Self::AdvancedFarmer),
            "SOILMAKER" => Some(Self::SoilMaker),
            "ROWMAKER" => Some(Self::RowMaker),
            "WATERBRINGER" => Some(Self::WaterBringer),
            _ => None,
        }
    }
}

/// Haxe speech `PROF!` aliases â†’ assigned farm profession.
///
/// - `FARMER!` / `WHEAT!` â†’ BASICFARMER
/// - `CARROT!` â†’ CARROTFARMER
/// - raw keys accepted if they match a farm profession
// Haxe: AiBase speech endsWith("!") ~4950
pub fn parse_farm_profession_speech(text: &str) -> Option<FarmProfession> {
    let t = text.trim();
    let prof = if let Some(stripped) = t.strip_suffix('!') {
        stripped.trim()
    } else {
        t
    };
    let upper = prof.to_ascii_uppercase();
    match upper.as_str() {
        "FARMER" | "WHEAT" | "BASICFARMER" => Some(FarmProfession::BasicFarmer),
        "CARROT" | "CARROTFARMER" => Some(FarmProfession::CarrotFarmer),
        "BERRY" | "BERRYFARMER" => Some(FarmProfession::BerryFarmer),
        "ADVANCED" | "ADVANCEDFARMER" => Some(FarmProfession::AdvancedFarmer),
        "SOIL" | "SOILMAKER" => Some(FarmProfession::SoilMaker),
        "ROW" | "ROWMAKER" => Some(FarmProfession::RowMaker),
        "WATER" | "WATERBRINGER" => Some(FarmProfession::WaterBringer),
        _ => FarmProfession::parse(prof),
    }
}

// â”€â”€ Task state (Haxe `taskState` map subset) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Mutable farm hysteresis flags (Haxe `this.taskState[...]`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FarmTaskState {
    /// SoilMaker: enter â‰¤3 soil-units, exit â‰¥10.
    pub soil_maker: f32,
    /// Composting: enter when stock &lt;1, exit when &gt;3.
    pub composting: f32,
    /// RowMaker stages: 0 idle, 1 shallow, 2 deep prep, 3 done deep.
    pub row_maker: f32,
    /// CarrotPlanter min/max hysteresis.
    pub carrot_planter: f32,
    /// Shared planter flag used by all `doPlant` crops (Haxe key always `CornPlanter`).
    /// Haxe quirk: every crop shares this single taskState key.
    pub corn_planter: f32,
    /// Corn harvest: 0 off, 1 picking, 2 shucking.
    pub harvest_corn: f32,
    /// WheatHarvester: 0 active, 1 stopped at max.
    pub wheat_harvester: f32,
    /// Per dry-plant id watering latch (`doWateringOn{id}`).
    pub watering_on: HashMap<i32, f32>,
}

impl FarmTaskState {
    pub fn watering_flag(&self, dry_id: i32) -> f32 {
        *self.watering_on.get(&dry_id).unwrap_or(&0.0)
    }

    pub fn set_watering_flag(&mut self, dry_id: i32, v: f32) {
        if v <= 0.0 {
            self.watering_on.remove(&dry_id);
        } else {
            self.watering_on.insert(dry_id, v);
        }
    }
}

// â”€â”€ Profession assignment / caps â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Sticky last + assigned + per-key weight (Haxe `profession` map).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FarmProfessionRuntime {
    pub last_profession: Option<FarmProfession>,
    pub assigned_profession: Option<FarmProfession>,
    /// Haxe `this.profession[key]` weights (usually 0/1).
    pub weights: HashMap<FarmProfession, f32>,
}

/// Apply speech `FARMER!` / `WHEAT!` / `CARROT!` / `ROW!` / `SOIL!` / `WATER!` / â€¦
/// onto sticky runtime (assigned + last + weight=1).
// Haxe: AiBase speech endsWith("!") â†’ assignedProfession farm keys
pub fn assign_farm_from_speech(runtime: &mut FarmProfessionRuntime, text: &str) -> bool {
    let Some(job) = parse_farm_profession_speech(text) else {
        return false;
    };
    runtime.assigned_profession = Some(job);
    runtime.last_profession = Some(job);
    runtime.weights.insert(job, 1.0);
    true
}

/// Count peers already sticky on `profession` (Haxe `countProfession`).
///
/// Caller supplies how many *other* home AIs already have `lastProfession == profession`.
// Haxe: AiBase.countProfession ~1284
pub fn count_profession_ok(
    peer_count_with_last: f32,
    // reserved for future gravekeeper age exceptions
) -> f32 {
    peer_count_with_last.max(0.0)
}

/// Haxe `hasOrBecomeProfession(profession, max)`.
///
/// - Sticky: if `last_profession == want`, keep and return true.
/// - `max < 0`: high priority â€” do job without assigning (always true).
/// - Else: if `peer_count >= max + was_idle` refuse; else assign weight=1 and sticky last.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_profession(
    runtime: &mut FarmProfessionRuntime,
    want: FarmProfession,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        // High priority: do job but do not assign profession.
        return true;
    }
    if runtime.last_profession == Some(want) {
        runtime.last_profession = Some(want);
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    let count = count_profession_ok(peer_count_with_last);
    if count >= cap {
        return false;
    }
    runtime.weights.insert(want, 1.0);
    runtime.last_profession = Some(want);
    true
}

/// Map assigned/last profession strings into a farm job for AssignedJob dispatch.
// Haxe: AiBase.doTimeStuffHelper assignedProfession block ~696â€“714
pub fn assigned_job_farm_profession(
    assigned: Option<&str>,
    last: Option<&str>,
) -> Option<FarmProfession> {
    let key = assigned.or(last)?;
    FarmProfession::parse(key).or_else(|| {
        // Accept speech aliases without '!'
        parse_farm_profession_speech(key)
    })
}

/// Prefer assigned over last when both set (matches Haxe if/else-if chain order).
pub fn resolve_farm_assigned_job(runtime: &FarmProfessionRuntime) -> Option<FarmProfession> {
    runtime
        .assigned_profession
        .or(runtime.last_profession)
}

/// Age-rotated farm slots only: 0 â†’ BerryFarmer, 1 â†’ BasicFarmer (others non-farm).
// Haxe: jobByAge % 5 â†’ berry / basic / bake / pottery / sheep ~793â€“801
pub fn age_rotated_farm_profession(age: f32) -> Option<FarmProfession> {
    match age_job_index(age) {
        0 => Some(FarmProfession::BerryFarmer),
        1 => Some(FarmProfession::BasicFarmer),
        _ => None,
    }
}

// â”€â”€ World counts snapshot â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Close-object counts near home (Haxe `CountCloseObjects` / `countCurrentObject`).
#[derive(Debug, Clone, Default)]
pub struct FarmCounts {
    /// Object parent id â†’ count (piles already expanded by caller if needed).
    pub by_id: HashMap<i32, i32>,
    /// Held object parent id (0 empty).
    pub held_id: i32,
    /// Standing / target biome for hardened-row refuse.
    pub hardened_row_biome: Option<u8>,
    /// Player is hungry (skips deep-row work in doPrepareRows).
    pub is_hungry: bool,
    /// BASICFARMER profession weight (bush max 3 vs 9).
    pub basic_farmer_weight: f32,
}

impl FarmCounts {
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
}

// â”€â”€ Actions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Pure decision output â€” execution is AI-CRAFT / shortCraft wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FarmAction {
    /// Nothing to do in this step.
    None,
    /// Haxe `shortCraft(actor, target)`.
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `craftItem(objectId)` â€” produce / obtain object.
    CraftItem { object_id: i32 },
    /// Refuse job (e.g. carrots cap, snow row).
    Abort,
    /// Haxe `doBasicFarming` mid: sticky BASICFARMER=1 then `isSheepHerding(1)`.
    /// `max_profession` is the outer `doBasicFarming(max)` arg for late
    /// `doAdvancedFarming(max)` (not the sheep peer-cap, which is always 1).
    /// Caller should set `profession['BASICFARMER']=1` (see
    /// [`apply_basic_farmer_weight_side_effect`]).
    // Haxe: AiBase.doBasicFarming ~2400â€“2413 (AI-SHEPHERD-MID / AI-FARM-STICKY)
    DeferSheepHerding { max_profession: i32 },
    /// Haxe `doAdvancedFarming(max)` after late plants / makeSharpieFood.
    // Haxe: AiBase.doBasicFarming ~2413 (AI-SHEPHERD-MID)
    DeferAdvancedFarming { max_profession: i32 },
    /// Haxe `this.profession['BASICFARMER'] = 0` when basic farm fully idle.
    // Haxe: AiBase.doBasicFarming ~2415 (AI-SHEPHERD-MID)
    ClearBasicFarmerWeight,
}

impl FarmAction {
    pub fn is_some(self) -> bool {
        !matches!(
            self,
            Self::None | Self::Abort | Self::ClearBasicFarmerWeight
        )
    }

    /// Haxe BASICFARMER weight write implied by this action, if any.
    // Haxe: AiBase.doBasicFarming ~2400 / ~2415
    pub fn basic_farmer_weight_side_effect(self) -> Option<f32> {
        match self {
            Self::DeferSheepHerding { .. } => Some(1.0),
            Self::ClearBasicFarmerWeight => Some(0.0),
            _ => None,
        }
    }
}

/// Apply Haxe `profession['BASICFARMER']` side-effects for mid/after-sheep actions.
// Haxe: AiBase.doBasicFarming ~2400 / ~2415
pub fn apply_basic_farmer_weight_side_effect(
    runtime: &mut FarmProfessionRuntime,
    action: FarmAction,
) {
    if let Some(w) = action.basic_farmer_weight_side_effect() {
        runtime.weights.insert(FarmProfession::BasicFarmer, w);
    }
}

/// Read Haxe `profession['BASICFARMER']` sticky weight (default 1.0 when unset).
// Haxe: profession map lookup in doPlantBushes / doBasicFarming
// AI-FARM-STICKY: live scan reads this into ProfessionScanInput.basic_farmer_weight
pub fn basic_farmer_weight_from_runtime(runtime: &FarmProfessionRuntime) -> f32 {
    runtime
        .weights
        .get(&FarmProfession::BasicFarmer)
        .copied()
        .unwrap_or(1.0)
}

// â”€â”€ shortCraft pure apply edges (AI-JOB-FARM-LIVE / CRAFT-LIVE-IO) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Pure shortCraft next step (Haxe `shortCraftOnTarget` without world I/O).
// Haxe: AiBase.shortCraftOnTarget ~2721
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortCraftApply {
    /// Held matches actor â†’ `useHeldObjOnTarget`.
    UseOnTarget { actor: i32, target: i32 },
    /// `actorId == 0` and hands not empty â†’ drop first.
    DropHeld,
    /// Need actor object â€” seek / `GetOrCraftItem` (`craft_if_needed` = Haxe flag).
    SeekOrCraftActor {
        actor: i32,
        /// Haxe `craftActorIfNeeded` â€” false â†’ seek only, do not craft graph.
        craft_if_needed: bool,
    },
    /// Prefer weak skewer 852 when actor is skewer 139 (caller may re-enter).
    /// Prefer [`short_craft_apply_resolved`] for automatic re-entry like Haxe.
    PreferWeakSkewer,
    /// Biome / carrot-seed / maxNewActor refuse.
    Refuse,
    /// Haxe `checkHungryWorkCostById` refused (food_store < cost + 1).
    // Haxe: AiBase.checkHungryWorkCostById ~1412
    RefuseHungry,
}

/// Inputs for [`short_craft_apply`] (caller fills from world / inventory).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShortCraftInput {
    pub held_id: i32,
    pub actor_id: i32,
    pub target_id: i32,
    /// Haxe `target.numberOfUses` (carrot-row seed guard uses `< 4`).
    pub target_uses: i32,
    /// Biome under target tile (snow/ocean refuse for soil/hoe).
    pub target_biome: Option<u8>,
    /// Haxe `hasCarrotSeeds` â€” when false, refuse low-use carrot row 400.
    pub has_carrot_seeds: bool,
    /// Nearby count of transition `newActor` (only if `max_new_actor > 0`).
    /// Include held when held == newActorID via [`new_actor_count_with_held`].
    pub new_actor_count: i32,
    /// Haxe `maxNewActor`; `<= 0` = unlimited.
    pub max_new_actor: i32,
    /// When true and actor is skewer, emit [`ShortCraftApply::PreferWeakSkewer`] first.
    pub try_weak_skewer_first: bool,
    /// Haxe `craftActorIfNeeded` passed to GetOrCraftItem.
    pub craft_actor_if_needed: bool,
    /// Player food store for hungry work cost gate (always-on in Haxe shortCraftOnTarget).
    pub food_store: f32,
    /// Transition `totalHungryWorkCost` (0 = free / unknown allow when no extra flags).
    pub transition_hungry_cost: f32,
}

impl ShortCraftInput {
    /// Minimal input (no biome/seed/max constraints; hungry free; craft allowed).
    pub fn basic(held_id: i32, actor_id: i32, target_id: i32) -> Self {
        Self {
            held_id,
            actor_id,
            target_id,
            target_uses: 1,
            target_biome: None,
            has_carrot_seeds: true,
            new_actor_count: 0,
            max_new_actor: -1,
            try_weak_skewer_first: true,
            craft_actor_if_needed: true,
            food_store: 20.0,
            transition_hungry_cost: 0.0,
        }
    }
}

/// Haxe maxNewActor count: `CountCloseObjects(newActor) + (held == newActor ? 1 : 0)`.
// Haxe: AiBase.shortCraftOnTarget ~2755â€“2756
pub fn new_actor_count_with_held(near_count: i32, held_id: i32, new_actor_id: i32) -> i32 {
    if new_actor_id != 0 && held_id == new_actor_id {
        near_count.saturating_add(1)
    } else {
        near_count
    }
}

/// Pure `shortCraftOnTarget` decision edges (single pass; no weak-skewer re-entry).
///
/// Order matches Haxe: hungry cost â†’ weak-skewer prefer â†’ snow/ocean â†’ carrot-row
/// seed â†’ maxNewActor â†’ held match USE â†’ actor0 DROP â†’ seek/craft actor.
// Haxe: AiBase.shortCraftOnTarget ~2721
pub fn short_craft_apply(inp: ShortCraftInput) -> ShortCraftApply {
    let actor = inp.actor_id;
    let target = inp.target_id;

    // Haxe always runs checkHungryWorkCostById before other shortCraftOnTarget gates.
    // Haxe: AiBase.shortCraftOnTarget ~2728
    if !crate::smith_profession::check_hungry_work_cost_by_id(
        inp.food_store,
        inp.transition_hungry_cost,
    ) {
        return ShortCraftApply::RefuseHungry;
    }

    // Skewer 139 â†’ Weak Skewer 852 FIX (Haxe tries weak first with craftActor=false).
    if inp.try_weak_skewer_first && actor == SKEWER {
        return ShortCraftApply::PreferWeakSkewer;
    }

    // Bowl of Soil 1137 + Hardened Row 848 â€” refuse snow/ocean.
    if actor == BOWL_OF_SOIL && target == HARDENED_ROW {
        if let Some(b) = inp.target_biome {
            if hardened_row_biome_refused(b) {
                return ShortCraftApply::Refuse;
            }
        }
    }
    // Stone Hoe 850 / Steel Hoe 857 + Fertile Soil 1138 â€” refuse snow/ocean.
    if (actor == STONE_HOE || actor == STEEL_HOE) && target == FERTILE_SOIL {
        if let Some(b) = inp.target_biome {
            if hardened_row_biome_refused(b) {
                return ShortCraftApply::Refuse;
            }
        }
    }

    // Dont use carrots if seed is needed // 400 Carrot Row
    if target == CARROT_ROW && !inp.has_carrot_seeds && inp.target_uses < 4 {
        return ShortCraftApply::Refuse;
    }

    // maxNewActor: refuse when nearby newActor count already at cap.
    // Haxe: TODO count maxNewActor at home vs current pos â€” port uses caller count.
    if inp.max_new_actor > 0 && inp.new_actor_count >= inp.max_new_actor {
        return ShortCraftApply::Refuse;
    }

    if inp.held_id == actor {
        return ShortCraftApply::UseOnTarget { actor, target };
    }
    if actor == 0 {
        return ShortCraftApply::DropHeld;
    }
    ShortCraftApply::SeekOrCraftActor {
        actor,
        craft_if_needed: inp.craft_actor_if_needed,
    }
}

/// Haxe shortCraftOnTarget with automatic weak-skewer 852 re-entry.
///
/// When actor is Skewer 139, tries Weak Skewer 852 first (`craftActor=false`);
/// on success returns that result, else continues with 139.
// Haxe: AiBase.shortCraftOnTarget ~2730â€“2731
pub fn short_craft_apply_resolved(inp: ShortCraftInput) -> ShortCraftApply {
    let first = short_craft_apply(inp);
    if first != ShortCraftApply::PreferWeakSkewer {
        return first;
    }
    // Try weak skewer 852 first (craftActorIfNeeded = false).
    let mut weak_inp = inp;
    weak_inp.actor_id = WEAK_SKEWER;
    weak_inp.try_weak_skewer_first = false;
    weak_inp.craft_actor_if_needed = false;
    let weak = short_craft_apply(weak_inp);
    // Haxe returns true only when weak shortCraftOnTarget succeeds.
    // Seek with craft=false still counts as a started action (GetItem path).
    match weak {
        ShortCraftApply::Refuse | ShortCraftApply::RefuseHungry | ShortCraftApply::PreferWeakSkewer => {
            // Fall through to original skewer 139 without prefer signal.
            let mut skewer_inp = inp;
            skewer_inp.try_weak_skewer_first = false;
            short_craft_apply(skewer_inp)
        }
        other => other,
    }
}

/// Map a [`FarmAction::ShortCraft`] through [`short_craft_apply`].
///
/// Returns `None` for non-ShortCraft actions. Hungry cost defaults free (0).
pub fn farm_action_short_craft_apply(
    action: FarmAction,
    held_id: i32,
    target_uses: i32,
    target_biome: Option<u8>,
    has_carrot_seeds: bool,
    new_actor_count: i32,
    max_new_actor: i32,
) -> Option<ShortCraftApply> {
    farm_action_short_craft_apply_ex(
        action,
        held_id,
        target_uses,
        target_biome,
        has_carrot_seeds,
        new_actor_count,
        max_new_actor,
        20.0,
        0.0,
    )
}

/// Farm shortCraft with hungry work cost (Haxe always-on gate).
// Haxe: AiBase.shortCraftOnTarget ~2728
pub fn farm_action_short_craft_apply_ex(
    action: FarmAction,
    held_id: i32,
    target_uses: i32,
    target_biome: Option<u8>,
    has_carrot_seeds: bool,
    new_actor_count: i32,
    max_new_actor: i32,
    food_store: f32,
    transition_hungry_cost: f32,
) -> Option<ShortCraftApply> {
    match action {
        FarmAction::ShortCraft { actor, target } => Some(short_craft_apply(ShortCraftInput {
            held_id,
            actor_id: actor,
            target_id: target,
            target_uses,
            target_biome,
            has_carrot_seeds,
            new_actor_count,
            max_new_actor,
            try_weak_skewer_first: actor == SKEWER,
            craft_actor_if_needed: true,
            food_store,
            transition_hungry_cost,
        })),
        _ => None,
    }
}

// â”€â”€ Watering transition table (Bowl of Water 382 + dry) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Default wet product for Bowl of Water + dry planted id.
///
/// Full table lives in content transitions; this covers farm pipeline ids.
// Haxe: TransitionImporter.GetTransition(382, dryId) in doWateringOn
pub fn default_wet_from_bowl(dry_id: i32) -> Option<i32> {
    Some(match dry_id {
        DRY_PLANTED_CARROTS => WET_PLANTED_CARROTS,
        DRY_PLANTED_WHEAT => WET_PLANTED_WHEAT,
        DRY_PLANTED_CORN => WET_PLANTED_CORN,
        DRY_PLANTED_GOOSEBERRY => WET_PLANTED_GOOSEBERRY,
        DRY_PLANTED_BEANS => WET_PLANTED_BEANS,
        DRY_PLANTED_POTATO => WET_PLANTED_POTATO,
        DRY_PLANTED_TOMATO => 2831, // Wet Planted Tomato Seed
        DRY_PLANTED_CUCUMBER => WET_PLANTED_CUCUMBER,
        DRY_PLANTED_PEPPER => WET_PLANTED_PEPPER,
        DRY_PLANTED_ONIONS => WET_PLANTED_ONIONS,
        DRY_DOMESTIC_BUSH => DOMESTIC_BUSH, // approximate wet bush family
        _ => return None,
    })
}

/// Haxe `doWateringOn(itemToWaterId, min)`.
// Haxe: AiBase.doWateringOn ~2587
pub fn do_watering_on(
    item_to_water_id: i32,
    min: i32,
    dry_count: i32,
    wet_product: Option<i32>,
    task: &mut FarmTaskState,
) -> FarmAction {
    let flag = task.watering_flag(item_to_water_id);
    if dry_count < 1 {
        task.set_watering_flag(item_to_water_id, 0.0);
        return FarmAction::None;
    }
    // When below min and not already latched, skip (batch watering).
    if dry_count < min && flag < 1.0 {
        return FarmAction::None;
    }
    task.set_watering_flag(item_to_water_id, 1.0);
    let wet = wet_product.or_else(|| default_wet_from_bowl(item_to_water_id));
    match wet {
        Some(id) => FarmAction::CraftItem { object_id: id },
        None => FarmAction::None,
    }
}

// â”€â”€ Plant hysteresis â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `doPlant(min, max, toPlantId, toCountIds)` â€” shared `CornPlanter` taskState.
// Haxe: AiBase.doPlant ~2558
pub fn do_plant(
    min_planted: i32,
    max_planted: i32,
    to_plant_id: i32,
    stage_count: i32,
    dry_count_for_water: i32,
    wet_product: Option<i32>,
    task: &mut FarmTaskState,
    // When true, skip nested prepare-rows (caller handles or tests isolation).
    allow_prepare_rows: bool,
    counts: &FarmCounts,
) -> FarmAction {
    let count = stage_count; // already includes to_plant_id + stages
    if count >= max_planted {
        task.corn_planter = 0.0;
        return FarmAction::None;
    }
    if count < min_planted {
        task.corn_planter = 1.0;
    }
    if task.corn_planter < 1.0 {
        return FarmAction::None;
    }
    // Water dry of this crop first.
    let water = do_watering_on(to_plant_id, 3, dry_count_for_water, wet_product, task);
    if water.is_some() {
        return water;
    }
    if allow_prepare_rows {
        let rows = do_prepare_rows(counts, task, /*has_profession*/ true, true);
        if rows.is_some() {
            return rows;
        }
    }
    FarmAction::CraftItem {
        object_id: to_plant_id,
    }
}

/// Wheat stages count helper for `doPlantWheat`.
pub fn wheat_stage_count(counts: &FarmCounts) -> i32 {
    counts.get(DRY_PLANTED_WHEAT)
        + counts.sum(&[RIPE_WHEAT, WET_PLANTED_WHEAT, WHEAT_SPROUTS, UNRIPE_WHEAT])
}

/// Corn stages.
pub fn corn_stage_count(counts: &FarmCounts) -> i32 {
    counts.get(DRY_PLANTED_CORN)
        + counts.sum(&[WET_PLANTED_CORN, CORN_SPROUT, CORN_PLANT])
}

/// Carrot effective stock (carrots + 4Ã— planted).
// Haxe: doPlantCarrots count = carrots + 4 * planted
pub fn carrot_stock_units(counts: &FarmCounts) -> i32 {
    let planted = counts.get(WET_PLANTED_CARROTS) + counts.get(DRY_PLANTED_CARROTS);
    counts.get(CARROT) + 4 * planted
}

/// Haxe `doPlantCarrots(min, max)`.
// Haxe: AiBase.doPlantCarrots ~2220
pub fn do_plant_carrots(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
) -> FarmAction {
    let count = carrot_stock_units(counts);
    if count >= max {
        task.carrot_planter = 0.0;
        return FarmAction::None;
    }
    if count <= min {
        task.carrot_planter = 1.0;
    }
    if task.carrot_planter < 1.0 {
        return FarmAction::None;
    }
    FarmAction::CraftItem {
        object_id: DRY_PLANTED_CARROTS,
    }
}

/// Crop wrapper ids for tests / advanced loop.
pub fn do_plant_wheat(min: i32, max: i32, counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_WHEAT,
        wheat_stage_count(counts),
        counts.get(DRY_PLANTED_WHEAT),
        Some(WET_PLANTED_WHEAT),
        task,
        false,
        counts,
    )
}

pub fn do_plant_corn(min: i32, max: i32, counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_CORN,
        corn_stage_count(counts),
        counts.get(DRY_PLANTED_CORN),
        Some(WET_PLANTED_CORN),
        task,
        false,
        counts,
    )
}

// â”€â”€ Harvest â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `doHarvestCorn(min, max)`.
// Haxe: AiBase.doHarvestCorn ~2422
pub fn do_harvest_corn(
    min_harvest: i32,
    max_harvest: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
) -> FarmAction {
    let dry_piles = counts.get(PILE_DRIED_CORN);
    let count_dry = counts.get(DRIED_CORN) + 2 * dry_piles;
    let count_ear = counts.get(EAR_OF_CORN);
    let count_shucked = counts.get(SHUCKED_CORN);
    let stock = count_dry + count_shucked;

    if stock >= max_harvest {
        task.harvest_corn = 0.0;
        return FarmAction::None;
    }
    if stock < min_harvest {
        task.harvest_corn = 1.0;
    }
    if task.harvest_corn < 1.0 {
        return FarmAction::None;
    }
    // Pick ears while state < 2 and ear stock low.
    if task.harvest_corn < 2.0 && count_ear < 4 {
        // shortCraft(0, Corn Plant 1112)
        if counts.get(CORN_PLANT) > 0 {
            return FarmAction::ShortCraft {
                actor: 0,
                target: CORN_PLANT,
            };
        }
    }
    task.harvest_corn = 2.0;
    // Sharp Stone + Ear of Corn
    if count_ear > 0 {
        // After shuck attempt Haxe sets task to 0.
        task.harvest_corn = 0.0;
        return FarmAction::ShortCraft {
            actor: SHARP_STONE,
            target: EAR_OF_CORN,
        };
    }
    task.harvest_corn = 0.0;
    FarmAction::None
}

/// Haxe `doHarvestWheat(min, max)`.
// Haxe: AiBase.doHarvestWheat ~2449
pub fn do_harvest_wheat(
    min_harvest: i32,
    max_harvest: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
) -> FarmAction {
    let threshed = counts.get(THRESHED_WHEAT) + counts.get(THRESHED_WHEAT_GROUND);
    let all_harvested = threshed + counts.get(HARVESTED_WHEAT) + counts.get(WHEAT_SHEAF);
    let planted_ripe = counts.get(RIPE_WHEAT);

    if threshed >= max_harvest {
        task.wheat_harvester = 1.0;
        return FarmAction::None;
    }
    if threshed < min_harvest {
        task.wheat_harvester = 0.0;
    }
    if task.wheat_harvester > 0.0 {
        return FarmAction::None;
    }
    if planted_ripe > 0 && all_harvested < max_harvest && counts.get(RIPE_WHEAT) > 0 {
        return FarmAction::CraftItem {
            object_id: HARVESTED_WHEAT,
        };
    }
    if counts.get(HARVESTED_WHEAT) > 0 {
        return FarmAction::CraftItem {
            object_id: WHEAT_SHEAF,
        };
    }
    if counts.get(WHEAT_SHEAF) > 0 {
        return FarmAction::CraftItem {
            object_id: THRESHED_WHEAT,
        };
    }
    task.wheat_harvester = 1.0;
    FarmAction::None
}

// â”€â”€ Soil / compost / rows â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Soil-unit metric: 2Ã— pile + fertile + deep rows.
// Haxe: doPrepareSoil count ~2003â€“2008
pub fn soil_unit_count(counts: &FarmCounts) -> i32 {
    2 * counts.get(FERTILE_SOIL_PILE) + counts.get(FERTILE_SOIL) + counts.get(DEEP_TILLED_ROW)
}

/// Haxe `doPrepareSoil` body after shortCrafts (hysteresis + craft 336).
// Haxe: AiBase.doPrepareSoil ~1991
pub fn do_prepare_soil(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> FarmAction {
    // Dung â†’ wet compost first.
    if counts.get(SHOVEL_OF_DUNG) > 0 || true {
        // Prefer shortCraft when either side may exist; pure path: always offer if not blocked.
        // Callers with zero of both still may craft later; match Haxe order:
        // shortCraft(900,625) then shortCraftOnGround(336)
    }
    // Offer dungâ†’compost when wet compost present or always attempt (execution no-ops if missing).
    // For pure tests we only emit when useful signals exist.
    if counts.get(WET_COMPOST) > 0 {
        return FarmAction::ShortCraft {
            actor: SHOVEL_OF_DUNG,
            target: WET_COMPOST,
        };
    }
    if counts.get(BASKET_OF_SOIL) > 0 {
        // shortCraftOnGround(336) â€” model as craft use of held basket of soil.
        return FarmAction::CraftItem {
            object_id: BASKET_OF_SOIL,
        };
    }

    let count = soil_unit_count(counts);
    if count <= 3 {
        task.soil_maker = 1.0;
    }
    if count >= 10 {
        task.soil_maker = 0.0;
    }
    if task.soil_maker < 1.0 {
        return FarmAction::None;
    }
    if !has_profession {
        return FarmAction::None;
    }
    if counts.held_id == BASKET {
        // Basket + soil source â†’ Basket of Soil (multi-target craft).
        return FarmAction::CraftItem {
            object_id: BASKET_OF_SOIL,
        };
    }
    FarmAction::CraftItem {
        object_id: BASKET_OF_SOIL,
    }
}

/// Update SoilMaker hysteresis only (for tests).
pub fn update_soil_maker_hysteresis(count: i32, task: &mut FarmTaskState) {
    if count <= 3 {
        task.soil_maker = 1.0;
    }
    if count >= 10 {
        task.soil_maker = 0.0;
    }
}

/// Haxe `doComposting` + wet-compost 625 recount (AI-SHEPHERD-MID residual).
// Haxe: AiBase.doComposting ~2056â€“2083
pub fn do_composting(counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    // Haxe: countCompost += Math.ceil(fertilePile / 2)
    let pile = counts.get(FERTILE_SOIL_PILE);
    let mut stock = counts.get(COMPOSTING_PILE) + counts.get(COMPOSTED_SOIL);
    stock += (pile + 1) / 2; // integer ceil(n/2) for n >= 0

    if stock < 1 {
        task.composting = 1.0;
    }
    if stock > 3 {
        task.composting = 0.0;
    }
    if task.composting == 0.0 && stock > 0 {
        return FarmAction::None;
    }
    // Haxe: if (craftItem(790)) return true;
    if counts.get(COMPOSTING_PILE) == 0 {
        return FarmAction::CraftItem {
            object_id: COMPOSTING_PILE,
        };
    }
    // Haxe wet-compost recount: countCurrentObject(625) + CountCloseObjects(player,625,30)
    let wet_map = counts.get(WET_COMPOST);
    let wet_held = if counts.held_id == WET_COMPOST { 1 } else { 0 };
    let stock_with_wet = stock + wet_map + wet_held + wet_map;
    if stock_with_wet < 2 {
        return FarmAction::CraftItem {
            object_id: WET_COMPOST,
        };
    }
    FarmAction::CraftItem {
        object_id: COMPOSTING_PILE,
    }
}

/// True if hardened row biome forbids soil (snow/ocean).
// Haxe: doPrepareRows biomeId == SNOW || OCEAN
pub fn hardened_row_biome_refused(biome: u8) -> bool {
    biome == SNOW_BIOME || biome == OCEAN_BIOME
}

/// Haxe `doPrepareRows` core priority: soil â†’ shallow â†’ deep hoe.
// Haxe: AiBase.doPrepareRows ~2086
pub fn do_prepare_rows(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    // When false, skip nested soil (avoid recursion from do_prepare_soil callers).
    allow_soil: bool,
) -> FarmAction {
    if allow_soil {
        let soil = do_prepare_soil(counts, task, has_profession);
        if soil.is_some() {
            return soil;
        }
    }
    // Haxe: if (keepBushesAlive()) return true;
    // shortCraft no-ops without target 389 â€” pure only interrupts when dying present.
    let bushes = keep_bushes_alive(counts);
    if bushes.is_some() && counts.get(DYING_BUSH) > 0 {
        return bushes;
    }
    if !has_profession {
        return FarmAction::None;
    }
    // Skewer sprouts / clear stake
    if counts.get(TOMATO_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: TOMATO_SPROUT,
        };
    }
    if counts.get(CUCUMBER_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: CUCUMBER_SPROUT,
        };
    }
    if counts.get(HARDENED_ROW_STAKE) > 0 {
        return FarmAction::ShortCraft {
            actor: 0,
            target: HARDENED_ROW_STAKE,
        };
    }
    if counts.held_id == BOWL_OF_SOIL && counts.get(POTATO_PLANTS) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: POTATO_PLANTS,
        };
    }

    let deep = counts.get(DEEP_TILLED_ROW);
    let shallow = counts.get(SHALLOW_TILLED_ROW);
    let count_rows = shallow + deep;

    if count_rows < 1 {
        task.row_maker = 1.0;
    }

    // Stage < 2: build shallow rows
    if task.row_maker < 2.0 {
        if count_rows < 9 {
            if let Some(b) = counts.hardened_row_biome {
                if hardened_row_biome_refused(b) {
                    return FarmAction::Abort;
                }
            }
            // Bowl of Soil + Hardened Row
            if counts.get(HARDENED_ROW) > 0 {
                return FarmAction::ShortCraft {
                    actor: BOWL_OF_SOIL,
                    target: HARDENED_ROW,
                };
            }
            // craft shallow row
            return FarmAction::CraftItem {
                object_id: SHALLOW_TILLED_ROW,
            };
        } else {
            task.row_maker = 2.0;
        }
    }

    if counts.is_hungry {
        return FarmAction::None;
    }

    if deep < 5 {
        task.row_maker = 1.0;
    }

    // Stage < 3: deepen rows
    if task.row_maker < 3.0 {
        if deep < 10 {
            // Steel hoe preferred, then stone hoe
            if shallow > 0 {
                return FarmAction::ShortCraft {
                    actor: STEEL_HOE,
                    target: SHALLOW_TILLED_ROW,
                };
            }
            let hard = counts.get(HARDENED_ROW);
            if hard > 0 {
                task.row_maker = 1.0;
                return FarmAction::ShortCraft {
                    actor: BOWL_OF_SOIL,
                    target: HARDENED_ROW,
                };
            }
            if counts.get(FERTILE_SOIL) > 0 {
                return FarmAction::ShortCraft {
                    actor: STEEL_HOE,
                    target: FERTILE_SOIL,
                };
            }
            return FarmAction::ShortCraft {
                actor: STONE_HOE,
                target: FERTILE_SOIL,
            };
        } else {
            task.row_maker = 3.0;
        }
    }

    // Low bowls â†’ pottery deferred (returns None here)
    let mut bowls = counts.get(CLAY_BOWL);
    if counts.held_id == CLAY_BOWL {
        bowls += 1;
    }
    if deep < 6 && bowls < 1 {
        // Signal pottery need via Abort? Prefer None â€” doPottery is separate profession.
        return FarmAction::None;
    }
    FarmAction::None
}

// â”€â”€ Job sequences â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Haxe `doCarrotFarming`.
// Haxe: AiBase.doCarrotFarming ~1944
pub fn do_carrot_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> FarmAction {
    if !has_profession {
        return FarmAction::None;
    }
    // Pull carrots
    if counts.get(400) > 0 {
        // Seeding Carrots / planted pull target 400
        return FarmAction::ShortCraft {
            actor: 0,
            target: 400,
        };
    }
    if counts.get(TOMATO_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: TOMATO_SPROUT,
        };
    }
    if counts.get(CUCUMBER_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: CUCUMBER_SPROUT,
        };
    }
    let rows = do_prepare_rows(counts, task, true, true);
    if rows.is_some() {
        return rows;
    }
    if counts.get(CARROT) > 10 {
        return FarmAction::Abort;
    }
    let water = do_watering_on(
        DRY_PLANTED_CARROTS,
        3,
        counts.get(DRY_PLANTED_CARROTS),
        Some(WET_PLANTED_CARROTS),
        task,
    );
    if water.is_some() {
        return water;
    }
    let plant = do_plant_carrots(2, 5, counts, task);
    if plant.is_some() {
        return plant;
    }
    let plant = do_plant_carrots(6, 40, counts, task);
    if plant.is_some() {
        return plant;
    }
    let water = do_watering_on(
        DRY_PLANTED_CARROTS,
        1,
        counts.get(DRY_PLANTED_CARROTS),
        Some(WET_PLANTED_CARROTS),
        task,
    );
    if water.is_some() {
        return water;
    }
    let compost = do_composting(counts, task);
    if compost.is_some() {
        return compost;
    }
    if counts.get(DYING_BUSH) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        };
    }
    FarmAction::None
}

/// Max bushes for berry planting (Haxe profession['BASICFARMER'] < 7 ? 3 : 9).
pub fn max_bushes(basic_farmer_weight: f32) -> i32 {
    if basic_farmer_weight < 7.0 {
        3
    } else {
        9
    }
}

/// Living domestic bush ids counted by Haxe `keepBushesAlive`
/// (`countCurrentObjects([391, 393, 1134, 1135])`).
// Haxe: AiBase.keepBushesAlive ~6052
pub const KEEP_BUSHES_ALIVE_IDS: [i32; 4] = [
    DOMESTIC_BUSH,
    DRY_DOMESTIC_BUSH,
    VIGOROUS_DOMESTIC_BUSH,
    EMPTY_DOMESTIC_BUSH,
];

/// Threshold under which `keepBushesAlive` tries Bowl of Soil + Dying Bush.
pub const KEEP_BUSHES_ALIVE_MIN: i32 = 20;

/// Sum of living domestic bushes for keepBushesAlive.
// Haxe: AiBase.keepBushesAlive countCurrentObjects ~6060
pub fn keep_bushes_alive_count(counts: &FarmCounts) -> i32 {
    counts.sum(&KEEP_BUSHES_ALIVE_IDS)
}

/// Haxe `keepBushesAlive` pure decision.
///
/// When living bush sum &lt; 20, emit `ShortCraft(1137, 389)` (Bowl of Soil + Dying).
/// Haxe always *attempts* shortCraft (no-ops if no dying target); pure path emits the
/// intent whenever the count gate fires so ladder/selfplay can seek dying bushes.
// Haxe: AiBase.keepBushesAlive ~6052
pub fn keep_bushes_alive(counts: &FarmCounts) -> FarmAction {
    if keep_bushes_alive_count(counts) < KEEP_BUSHES_ALIVE_MIN {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        };
    }
    FarmAction::None
}

/// Haxe `doCriticalStuff` farm-related slice (age-gated bushes + basic + carrot).
///
/// Floors / cleanup / watering(1) / bake / pottery are out of scope here â€” callers
/// chain those professions separately. `basic_ok` / `carrot_ok` are results of
/// `hasOrBecomeProfession(..., max=1)` for the critical path.
// Haxe: AiBase.doCriticalStuff ~6072 farm tails (bushes / basic / carrot)
pub fn do_critical_farm_slice(
    age: f32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    basic_ok: bool,
    carrot_ok: bool,
) -> FarmAction {
    // Haxe: (Math.round(myPlayer.age / 5)) % 2 == 0 && keepBushesAlive()
    let age_slot = (age / 5.0).round() as i32;
    if age_slot.rem_euclid(2) == 0 {
        let a = keep_bushes_alive(counts);
        // Only act when dying target present (Haxe shortCraft fails if none).
        if a.is_some() && counts.get(DYING_BUSH) > 0 {
            return a;
        }
    }
    if basic_ok {
        // Haxe mid-priority doBasicFarming() default maxProfession=2
        let a = do_basic_farming(counts, task, true, BASIC_FARM_DEFAULT_MAX_PROFESSION);
        if a.is_some() {
            return a;
        }
    }
    if carrot_ok {
        let a = do_carrot_farming(counts, task, true);
        if a.is_some() {
            return a;
        }
    }
    FarmAction::None
}

/// Sum of bush-family stages near home.
pub fn bush_stage_count(counts: &FarmCounts) -> i32 {
    counts.sum(&[
        DOMESTIC_BUSH,
        DRY_DOMESTIC_BUSH,
        EMPTY_DOMESTIC_BUSH,
        VIGOROUS_DOMESTIC_BUSH,
        GOOSEBERRY_SPROUT,
        WET_PLANTED_GOOSEBERRY,
        DRY_PLANTED_GOOSEBERRY,
        DYING_BUSH,
    ])
}

/// Haxe `doPlantBushes`.
// Haxe: AiBase.doPlantBushes ~2294
pub fn do_plant_bushes(counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    let water = do_watering_on(
        DRY_DOMESTIC_BUSH,
        3,
        counts.get(DRY_DOMESTIC_BUSH),
        default_wet_from_bowl(DRY_DOMESTIC_BUSH),
        task,
    );
    if water.is_some() {
        return water;
    }
    let water = do_watering_on(
        DRY_PLANTED_GOOSEBERRY,
        3,
        counts.get(DRY_PLANTED_GOOSEBERRY),
        Some(WET_PLANTED_GOOSEBERRY),
        task,
    );
    if water.is_some() {
        return water;
    }
    let bushes = bush_stage_count(counts);
    if bushes >= max_bushes(counts.basic_farmer_weight) {
        return FarmAction::None;
    }
    FarmAction::CraftItem {
        object_id: DRY_PLANTED_GOOSEBERRY,
    }
}

/// Haxe `doBerryFarming`.
// Haxe: AiBase.doBerryFarming ~2259
pub fn do_berry_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> FarmAction {
    if !has_profession {
        return FarmAction::None;
    }
    if counts.get(DYING_BUSH) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        };
    }
    if counts.get(LANGUISHING_BUSH) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: LANGUISHING_BUSH,
        };
    }
    let rows = do_prepare_rows(counts, task, true, true);
    if rows.is_some() {
        return rows;
    }
    let water = do_watering_on(
        DRY_PLANTED_GOOSEBERRY,
        3,
        counts.get(DRY_PLANTED_GOOSEBERRY),
        Some(WET_PLANTED_GOOSEBERRY),
        task,
    );
    if water.is_some() {
        return water;
    }
    let plant = do_plant_bushes(counts, task);
    if plant.is_some() {
        return plant;
    }
    let water = do_watering_on(
        DRY_PLANTED_GOOSEBERRY,
        1,
        counts.get(DRY_PLANTED_GOOSEBERRY),
        Some(WET_PLANTED_GOOSEBERRY),
        task,
    );
    if water.is_some() {
        return water;
    }
    do_composting(counts, task)
}

/// Advanced plant rotation table (Haxe `advancedPlants`).
pub const ADVANCED_PLANTS: [i32; 11] = [
    DRY_PLANTED_POTATO,
    DRY_PLANTED_BEANS,
    DRY_PLANTED_PEPPER,
    DRY_PLANTED_ONIONS,
    DRY_PLANTED_POTATO,
    DRY_PLANTED_CUCUMBER,
    DRY_PLANTED_TOMATO,
    DRY_PLANTED_SQUASH,
    DRY_PLANTED_POTATO,
    WET_PLANTED_ONIONS,
    DRY_PLANTED_POTATO,
];

/// Pick advanced plant id from rotation index (Haxe toPlant + age).
pub fn advanced_plant_at(to_plant: usize, age_years: f32, i: usize) -> i32 {
    let next = to_plant.wrapping_add(age_years.round() as usize);
    let index = (next + i) % ADVANCED_PLANTS.len();
    ADVANCED_PLANTS[index]
}

/// Haxe `doAdvancedFarming` decision for one rotation step.
// Haxe: AiBase.doAdvancedFarming ~3909
pub fn do_advanced_farming_step(
    plant_id: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    bowl_count: i32,
) -> FarmAction {
    // Potato care first (callers should try before rotation; included here for completeness)
    if counts.get(MATURE_POTATO) > 0 {
        return FarmAction::ShortCraft {
            actor: SHOVEL,
            target: MATURE_POTATO,
        };
    }
    if counts.get(POTATO_PLANTS) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: POTATO_PLANTS,
        };
    }
    if bowl_count < 1 {
        // Pottery gate â€” signal via None (doPottery external)
        return FarmAction::None;
    }
    match plant_id {
        DRY_PLANTED_BEANS | WET_PLANTED_BEANS => {
            let stages = counts.get(DRY_PLANTED_BEANS)
                + counts.sum(&[WET_PLANTED_BEANS, GREEN_BEAN_PLANTS, DRY_BEAN_PLANTS]);
            do_plant(
                2,
                4,
                DRY_PLANTED_BEANS,
                stages,
                counts.get(DRY_PLANTED_BEANS),
                Some(WET_PLANTED_BEANS),
                task,
                false,
                counts,
            )
        }
        DRY_PLANTED_POTATO | WET_PLANTED_POTATO => {
            if counts.get(SHOVEL) < 1 {
                return FarmAction::None;
            }
            let stages = counts.get(DRY_PLANTED_POTATO)
                + counts.sum(&[
                    WET_PLANTED_POTATO,
                    POTATO_PLANTS,
                    MOUNDED_POTATO,
                    MATURE_POTATO,
                    DUG_POTATO,
                ]);
            do_plant(
                2,
                8,
                DRY_PLANTED_POTATO,
                stages,
                counts.get(DRY_PLANTED_POTATO),
                Some(WET_PLANTED_POTATO),
                task,
                false,
                counts,
            )
        }
        DRY_PLANTED_CUCUMBER => {
            let stages = counts.get(DRY_PLANTED_CUCUMBER)
                + counts.sum(&[WET_PLANTED_CUCUMBER, CUCUMBER_SPROUT, RIPE_CUCUMBER]);
            do_plant(
                2,
                8,
                DRY_PLANTED_CUCUMBER,
                stages,
                counts.get(DRY_PLANTED_CUCUMBER),
                Some(WET_PLANTED_CUCUMBER),
                task,
                false,
                counts,
            )
        }
        DRY_PLANTED_PEPPER => {
            let stages = counts.get(DRY_PLANTED_PEPPER)
                + counts.sum(&[WET_PLANTED_PEPPER, PEPPER_PLANT, FRUITING_PEPPER]);
            do_plant(
                2,
                5,
                DRY_PLANTED_PEPPER,
                stages,
                counts.get(DRY_PLANTED_PEPPER),
                Some(WET_PLANTED_PEPPER),
                task,
                false,
                counts,
            )
        }
        DRY_PLANTED_TOMATO => {
            let stages = counts.get(DRY_PLANTED_TOMATO)
                + counts.sum(&[TOMATO_PLANT, FRUITING_TOMATO]);
            do_plant(
                1,
                8,
                DRY_PLANTED_TOMATO,
                stages,
                counts.get(DRY_PLANTED_TOMATO),
                default_wet_from_bowl(DRY_PLANTED_TOMATO),
                task,
                false,
                counts,
            )
        }
        DRY_PLANTED_SQUASH => {
            // Haxe: doPlanSquash commented out â€” skip
            FarmAction::None
        }
        DRY_PLANTED_ONIONS | WET_PLANTED_ONIONS => {
            let onion_count = counts.get(RIPE_ONIONS) + counts.get(DRY_PLANTED_ONIONS);
            if onion_count > 6 {
                return FarmAction::None;
            }
            FarmAction::CraftItem {
                object_id: plant_id,
            }
        }
        other => FarmAction::CraftItem { object_id: other },
    }
}

/// Default `maxProfession` for Haxe `doBasicFarming()` / `doBasicFarming(2)`.
// Haxe: AiBase.doBasicFarming maxProfession = 2
pub const BASIC_FARM_DEFAULT_MAX_PROFESSION: i32 = 2;
/// Assigned BASICFARMER job: Haxe `doBasicFarming(100)`.
// Haxe: AiBase.doTimeStuffHelper ~710
pub const BASIC_FARM_ASSIGNED_MAX_PROFESSION: i32 = 100;

/// Haxe `doBasicFarming` sequence (first applicable action).
///
/// `max_profession` is the Haxe `maxProfession` peer-cap for `hasOrBecomeProfession`
/// and late `doAdvancedFarming(maxProfession)` (carried on [`FarmAction::DeferSheepHerding`]).
// Haxe: AiBase.doBasicFarming ~2343
pub fn do_basic_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    max_profession: i32,
) -> FarmAction {
    if !has_profession {
        return FarmAction::None;
    }
    // Pull carrots / dung / skewer / potato care
    if counts.get(400) > 0 {
        return FarmAction::ShortCraft {
            actor: 0,
            target: 400,
        };
    }
    if counts.get(WET_COMPOST) > 0 {
        return FarmAction::ShortCraft {
            actor: SHOVEL_OF_DUNG,
            target: WET_COMPOST,
        };
    }
    if counts.get(TOMATO_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: TOMATO_SPROUT,
        };
    }
    if counts.get(CUCUMBER_SPROUT) > 0 {
        return FarmAction::ShortCraft {
            actor: SKEWER,
            target: CUCUMBER_SPROUT,
        };
    }
    if counts.get(HARDENED_ROW_STAKE) > 0 {
        return FarmAction::ShortCraft {
            actor: 0,
            target: HARDENED_ROW_STAKE,
        };
    }
    if counts.get(MATURE_POTATO) > 0 {
        return FarmAction::ShortCraft {
            actor: SHOVEL,
            target: MATURE_POTATO,
        };
    }
    if counts.get(POTATO_PLANTS) > 0 {
        return FarmAction::ShortCraft {
            actor: BOWL_OF_SOIL,
            target: POTATO_PLANTS,
        };
    }

    let h = do_harvest_corn(1, 5, counts, task);
    if h.is_some() {
        return h;
    }
    let h = do_harvest_wheat(1, 4, counts, task);
    if h.is_some() {
        return h;
    }
    let p = do_plant_corn(1, 3, counts, task);
    if p.is_some() {
        return p;
    }
    let p = do_plant_wheat(2, 5, counts, task);
    if p.is_some() {
        return p;
    }
    // tomato / beans / cucumber / pepper via generic plant
    for (min, max, plant_id, stages) in [
        (
            2,
            5,
            DRY_PLANTED_TOMATO,
            counts.get(DRY_PLANTED_TOMATO) + counts.sum(&[TOMATO_PLANT, FRUITING_TOMATO]),
        ),
        (
            2,
            4,
            DRY_PLANTED_BEANS,
            counts.get(DRY_PLANTED_BEANS)
                + counts.sum(&[WET_PLANTED_BEANS, GREEN_BEAN_PLANTS, DRY_BEAN_PLANTS]),
        ),
        (
            2,
            4,
            DRY_PLANTED_CUCUMBER,
            counts.get(DRY_PLANTED_CUCUMBER)
                + counts.sum(&[WET_PLANTED_CUCUMBER, CUCUMBER_SPROUT, RIPE_CUCUMBER]),
        ),
        (
            2,
            5,
            DRY_PLANTED_PEPPER,
            counts.get(DRY_PLANTED_PEPPER)
                + counts.sum(&[WET_PLANTED_PEPPER, PEPPER_PLANT, FRUITING_PEPPER]),
        ),
    ] {
        let p = do_plant(
            min,
            max,
            plant_id,
            stages,
            counts.get(plant_id),
            default_wet_from_bowl(plant_id),
            task,
            false,
            counts,
        );
        if p.is_some() {
            return p;
        }
    }
    // potatoes
    if counts.get(SHOVEL) >= 1 {
        let stages = counts.get(DRY_PLANTED_POTATO)
            + counts.sum(&[
                WET_PLANTED_POTATO,
                POTATO_PLANTS,
                MOUNDED_POTATO,
                MATURE_POTATO,
                DUG_POTATO,
            ]);
        let p = do_plant(
            2,
            5,
            DRY_PLANTED_POTATO,
            stages,
            counts.get(DRY_PLANTED_POTATO),
            Some(WET_PLANTED_POTATO),
            task,
            false,
            counts,
        );
        if p.is_some() {
            return p;
        }
    }
    let c = do_composting(counts, task);
    if c.is_some() {
        return c;
    }
    // doWatering(3) deferred to WATERBRINGER body
    let p = do_plant_wheat(6, 12, counts, task);
    if p.is_some() {
        return p;
    }
    let p = do_plant_corn(4, 8, counts, task);
    if p.is_some() {
        return p;
    }
    // Haxe: this.profession['BASICFARMER'] = 1; isSheepHerding(1);
    // then late plants â†’ doAdvancedFarming(maxProfession).
    // AI-SHEPHERD-MID mid call site; max_profession carried for advanced expand.
    // Haxe: AiBase.doBasicFarming ~2400â€“2413
    FarmAction::DeferSheepHerding { max_profession }
}

/// Pure Haxe `makeSharpieFood` (wild carrot / burdock + sharp stone).
///
/// Returns `CraftItem` for GetOrCraft sharp stone or dug product when source plant present.
// Haxe: AiBase.makeSharpieFood ~4096â€“4118
pub fn make_sharpie_food(counts: &FarmCounts) -> FarmAction {
    let holding_sharp = counts.held_id == SHARP_STONE;
    // Seeding Wild Carrot 36 â†’ sharp stone 34 / Dug Wild Carrot 39
    if counts.get(SEEDING_WILD_CARROT) > 0 {
        if !holding_sharp {
            // Haxe: GetOrCraftItem(34)
            return FarmAction::CraftItem {
                object_id: SHARP_STONE,
            };
        }
        // Haxe: craftItem(39)
        return FarmAction::CraftItem {
            object_id: DUG_WILD_CARROT,
        };
    }
    // Burdock 804 â†’ sharp stone / Dug Burdock 806
    if counts.get(BURDOCK) > 0 {
        if !holding_sharp {
            return FarmAction::CraftItem {
                object_id: SHARP_STONE,
            };
        }
        return FarmAction::CraftItem {
            object_id: DUG_BURDOCK,
        };
    }
    FarmAction::None
}

/// Haxe `doAdvancedFarming` body after `hasOrBecomeProfession` succeeded.
// Haxe: AiBase.doAdvancedFarming ~3909â€“3958 (partial: rows + potato + rotation plant)
pub fn do_advanced_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    age: f32,
    has_profession: bool,
) -> FarmAction {
    if !has_profession {
        return FarmAction::None;
    }
    // Haxe: if (doPrepareRows(maxPeople)) return true;
    let rows = do_prepare_rows(counts, task, true, true);
    if rows.is_some() {
        return rows;
    }
    let plant = advanced_plant_at(0, age, 0);
    let bowls = counts.get(CLAY_BOWL);
    do_advanced_farming_step(plant, counts, task, bowls)
}

/// Haxe `doBasicFarming` tail after mid `isSheepHerding(1)` fallthrough.
///
/// Order: late wheat(15,30) â†’ late corn(8,12) â†’ age&lt;20 makeSharpieFood â†’
/// defer advanced farming (caller expands / clears BASICFARMER).
// Haxe: AiBase.doBasicFarming ~2408â€“2419 (AI-SHEPHERD-MID)
pub fn do_basic_farming_after_sheep(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    age: f32,
    max_profession: i32,
) -> FarmAction {
    let p = do_plant_wheat(15, 30, counts, task);
    if p.is_some() {
        return p;
    }
    let p = do_plant_corn(8, 12, counts, task);
    if p.is_some() {
        return p;
    }
    // Haxe: if (myPlayer.age < 20 && makeSharpieFood()) return true;
    if age < 20.0 {
        let s = make_sharpie_food(counts);
        if s.is_some() {
            return s;
        }
    }
    // Haxe: Macro.exception(if (doAdvancedFarming(maxProfession)) return true);
    // Then profession['BASICFARMER']=0 when advanced also idle (expanded by caller).
    FarmAction::DeferAdvancedFarming { max_profession }
}

/// Expand [`FarmAction::DeferAdvancedFarming`]: try advanced body, else clear BASICFARMER.
// Haxe: AiBase.doBasicFarming ~2413â€“2415
pub fn expand_advanced_farming_or_clear(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    age: f32,
    has_advanced_profession: bool,
) -> FarmAction {
    let adv = do_advanced_farming(counts, task, age, has_advanced_profession);
    if adv.is_some() {
        return adv;
    }
    FarmAction::ClearBasicFarmerWeight
}

/// Dispatch one farm job step for AssignedJob / age-rotated / mid-prio.
///
/// `max_profession` applies to BasicFarmer (`doBasicFarming(max)`); other jobs ignore it.
// Haxe: assignedProfession dispatch + age job + mid doCarrotFarming
pub fn decide_farm_job(
    job: FarmProfession,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    max_profession: i32,
) -> FarmAction {
    match job {
        FarmProfession::BasicFarmer => {
            do_basic_farming(counts, task, has_profession, max_profession)
        }
        FarmProfession::CarrotFarmer => do_carrot_farming(counts, task, has_profession),
        FarmProfession::BerryFarmer => do_berry_farming(counts, task, has_profession),
        FarmProfession::SoilMaker => do_prepare_soil(counts, task, has_profession),
        FarmProfession::RowMaker => do_prepare_rows(counts, task, has_profession, true),
        FarmProfession::AdvancedFarmer => {
            let plant = advanced_plant_at(0, 20.0, 0);
            let bowls = counts.get(CLAY_BOWL);
            do_advanced_farming_step(plant, counts, task, bowls)
        }
        FarmProfession::WaterBringer => {
            // Prefer first dry crop with count > 0
            for dry in [
                DRY_PLANTED_CARROTS,
                DRY_PLANTED_WHEAT,
                DRY_PLANTED_CORN,
                DRY_PLANTED_GOOSEBERRY,
                DRY_PLANTED_BEANS,
                DRY_PLANTED_POTATO,
            ] {
                let a = do_watering_on(dry, 1, counts.get(dry), default_wet_from_bowl(dry), task);
                if a.is_some() {
                    return a;
                }
            }
            FarmAction::None
        }
    }
}

// â”€â”€ Self-play / craft graph farmer pipeline â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Intermediate + product ids the farmer pipeline cares about (seek order).
pub fn farmer_pipeline_targets() -> &'static [i32] {
    &[
        RIPE_WHEAT,
        THRESHED_WHEAT,
        WHEAT_SHEAF,
        HARVESTED_WHEAT,
        DRY_PLANTED_WHEAT,
        DRY_PLANTED_CORN,
        CORN_PLANT,
        EAR_OF_CORN,
        SHUCKED_CORN,
        DRY_PLANTED_CARROTS,
        CARROT,
        DRY_PLANTED_GOOSEBERRY,
        BASKET_OF_SOIL,
        COMPOSTING_PILE,
        DEEP_TILLED_ROW,
        SHALLOW_TILLED_ROW,
    ]
}

/// Reverse-craft goal for farmer (like smith iron expansion).
// Haxe: thin path was SeekObject(242); expand intermediates via craft graph
pub fn pick_farmer_goal(graph: &ReverseCraftGraph, have: &HashSet<i32>) -> Goal {
    for &want in farmer_pipeline_targets() {
        if have.contains(&want) {
            continue;
        }
        if let Some(ing) = graph.seek_ingredient_for(want, have) {
            return Goal::SeekObject(ing);
        }
        return Goal::SeekObject(want);
    }
    Goal::SeekObject(FARMER_TARGET_ID)
}

// Haxe: AiHelper.CountCloseObjects farm spatial (AI-JOB-FARM-WIRE / farm_spatial)
include!("farm_spatial_inc.rs");

// â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_goals::priority_ladder::{
        goal_from_rung, PriorityRung,
    };
    use crate::ai_goals::Profession;

    fn counts_with(pairs: &[(i32, i32)]) -> FarmCounts {
        let mut c = FarmCounts::default();
        for &(id, n) in pairs {
            c.set(id, n);
        }
        c
    }

    #[test]
    fn has_or_become_profession_respects_max_and_sticky_last_profession() {
        let mut rt = FarmProfessionRuntime::default();
        // Cap 1, peer already has job â†’ refuse
        assert!(!has_or_become_profession(
            &mut rt,
            FarmProfession::BasicFarmer,
            1,
            1.0,
            0.0
        ));
        // No peers â†’ become
        assert!(has_or_become_profession(
            &mut rt,
            FarmProfession::BasicFarmer,
            1,
            0.0,
            0.0
        ));
        assert_eq!(rt.last_profession, Some(FarmProfession::BasicFarmer));
        // Sticky even if peers full
        assert!(has_or_become_profession(
            &mut rt,
            FarmProfession::BasicFarmer,
            1,
            5.0,
            0.0
        ));
        // Different profession refused when max filled by peers
        assert!(!has_or_become_profession(
            &mut rt,
            FarmProfession::CarrotFarmer,
            1,
            1.0,
            0.0
        ));
        // max < 0 high priority always true without changing last for... Haxe still returns true
        assert!(has_or_become_profession(
            &mut rt,
            FarmProfession::SoilMaker,
            -2,
            99.0,
            0.0
        ));
        // was_idle expands cap
        let mut rt2 = FarmProfessionRuntime::default();
        assert!(!has_or_become_profession(
            &mut rt2,
            FarmProfession::BerryFarmer,
            1,
            1.0,
            0.0
        ));
        assert!(has_or_become_profession(
            &mut rt2,
            FarmProfession::BerryFarmer,
            1,
            1.0,
            1.0
        ));
    }

    #[test]
    fn assigned_job_basicfarmer_dispatches_before_age_rotated() {
        let rt = FarmProfessionRuntime {
            assigned_profession: Some(FarmProfession::BasicFarmer),
            last_profession: Some(FarmProfession::BerryFarmer),
            ..Default::default()
        };
        assert_eq!(
            resolve_farm_assigned_job(&rt),
            Some(FarmProfession::BasicFarmer)
        );
        // age would prefer berry at age 0, but assigned wins in AssignedJob rung
        assert_eq!(
            age_rotated_farm_profession(0.0),
            Some(FarmProfession::BerryFarmer)
        );
        let job = resolve_farm_assigned_job(&rt).unwrap();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(RIPE_WHEAT, 2)]);
        let a = decide_farm_job(
            job,
            &counts,
            &mut task,
            true,
            BASIC_FARM_ASSIGNED_MAX_PROFESSION,
        );
        // basic farming with ripe wheat â†’ harvest chain craft 224
        assert_eq!(
            a,
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );
    }

    #[test]
    fn do_plant_hysteresis_min_max_task_state_for_wheat_ids() {
        let mut task = FarmTaskState::default();
        // Below min â†’ enter planter, craft dry wheat
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 0)]);
        let a = do_plant_wheat(2, 5, &counts, &mut task);
        assert!(task.corn_planter >= 1.0);
        assert_eq!(
            a,
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_WHEAT
            }
        );
        // At/above max â†’ exit and none (shared CornPlanter quirk)
        let counts = counts_with(&[
            (DRY_PLANTED_WHEAT, 2),
            (RIPE_WHEAT, 3), // total stage 5
        ]);
        let a = do_plant_wheat(2, 5, &counts, &mut task);
        assert_eq!(task.corn_planter, 0.0);
        assert_eq!(a, FarmAction::None);
        // Between min and max with flag off â†’ none
        task.corn_planter = 0.0;
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 3)]);
        let a = do_plant_wheat(2, 5, &counts, &mut task);
        assert_eq!(a, FarmAction::None);
    }

    #[test]
    fn do_watering_on_requires_dry_count_and_bowl_water_transition() {
        let mut task = FarmTaskState::default();
        // No dry â†’ none, flag cleared
        assert_eq!(
            do_watering_on(DRY_PLANTED_CARROTS, 3, 0, Some(WET_PLANTED_CARROTS), &mut task),
            FarmAction::None
        );
        // dry=2 < min=3 and not latched â†’ none
        assert_eq!(
            do_watering_on(DRY_PLANTED_CARROTS, 3, 2, Some(WET_PLANTED_CARROTS), &mut task),
            FarmAction::None
        );
        // dry >= min â†’ craft wet
        assert_eq!(
            do_watering_on(DRY_PLANTED_CARROTS, 3, 3, Some(WET_PLANTED_CARROTS), &mut task),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_CARROTS
            }
        );
        assert!(task.watering_flag(DRY_PLANTED_CARROTS) >= 1.0);
        // No transition â†’ none
        let mut task2 = FarmTaskState::default();
        assert_eq!(
            do_watering_on(99999, 1, 5, None, &mut task2),
            FarmAction::None
        );
        // Default table works for wheat
        assert_eq!(
            default_wet_from_bowl(DRY_PLANTED_WHEAT),
            Some(WET_PLANTED_WHEAT)
        );
        assert_eq!(BOWL_OF_WATER, 382);
    }

    #[test]
    fn do_harvest_wheat_chain_224_225_226_stops_at_max() {
        let mut task = FarmTaskState::default();
        // Ripe â†’ craft 224
        let c = counts_with(&[(RIPE_WHEAT, 1)]);
        assert_eq!(
            do_harvest_wheat(1, 4, &c, &mut task),
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );
        // Harvested â†’ sheaf
        let c = counts_with(&[(HARVESTED_WHEAT, 1)]);
        assert_eq!(
            do_harvest_wheat(1, 4, &c, &mut task),
            FarmAction::CraftItem {
                object_id: WHEAT_SHEAF
            }
        );
        // Sheaf â†’ threshed
        let c = counts_with(&[(WHEAT_SHEAF, 1)]);
        assert_eq!(
            do_harvest_wheat(1, 4, &c, &mut task),
            FarmAction::CraftItem {
                object_id: THRESHED_WHEAT
            }
        );
        // At max threshed â†’ stop (flag 1)
        let c = counts_with(&[(THRESHED_WHEAT, 4)]);
        assert_eq!(do_harvest_wheat(1, 4, &c, &mut task), FarmAction::None);
        assert!(task.wheat_harvester > 0.0);
    }

    #[test]
    fn do_harvest_corn_pick_then_shuck_with_task_state() {
        let mut task = FarmTaskState::default();
        // Enter harvest when stock below min
        let c = counts_with(&[(CORN_PLANT, 2)]);
        assert_eq!(
            do_harvest_corn(1, 5, &c, &mut task),
            FarmAction::ShortCraft {
                actor: 0,
                target: CORN_PLANT
            }
        );
        assert!(task.harvest_corn >= 1.0);
        // Shuck when ears present
        let mut task = FarmTaskState {
            harvest_corn: 1.0,
            ..Default::default()
        };
        let c = counts_with(&[(EAR_OF_CORN, 2)]);
        assert_eq!(
            do_harvest_corn(1, 5, &c, &mut task),
            FarmAction::ShortCraft {
                actor: SHARP_STONE,
                target: EAR_OF_CORN
            }
        );
        // Max stock stops
        let mut task = FarmTaskState {
            harvest_corn: 1.0,
            ..Default::default()
        };
        let c = counts_with(&[(SHUCKED_CORN, 5)]);
        assert_eq!(do_harvest_corn(1, 5, &c, &mut task), FarmAction::None);
        assert_eq!(task.harvest_corn, 0.0);
    }

    #[test]
    fn do_prepare_rows_shallow_then_deep_hoe_priority() {
        let mut task = FarmTaskState {
            row_maker: 1.0,
            ..Default::default()
        };
        // No rows â†’ craft shallow (or soil on hard)
        let c = counts_with(&[(HARDENED_ROW, 2)]);
        let a = do_prepare_rows(&c, &mut task, true, false);
        assert_eq!(
            a,
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: HARDENED_ROW
            }
        );
        // Enough shallow rows, stage deep: hoe shallow
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(SHALLOW_TILLED_ROW, 9), (DEEP_TILLED_ROW, 0)]);
        let a = do_prepare_rows(&c, &mut task, true, false);
        assert_eq!(
            a,
            FarmAction::ShortCraft {
                actor: STEEL_HOE,
                target: SHALLOW_TILLED_ROW
            }
        );
        // Snow biome refuse
        let mut task = FarmTaskState {
            row_maker: 1.0,
            ..Default::default()
        };
        let mut c = counts_with(&[(HARDENED_ROW, 1)]);
        c.hardened_row_biome = Some(SNOW_BIOME);
        assert_eq!(
            do_prepare_rows(&c, &mut task, true, false),
            FarmAction::Abort
        );
    }

    #[test]
    fn do_prepare_soil_soil_maker_enters_below_3_exits_above_10() {
        let mut task = FarmTaskState::default();
        update_soil_maker_hysteresis(2, &mut task);
        assert!(task.soil_maker >= 1.0);
        update_soil_maker_hysteresis(10, &mut task);
        assert_eq!(task.soil_maker, 0.0);
        // Active soil maker crafts basket of soil
        task.soil_maker = 1.0;
        let c = counts_with(&[(FERTILE_SOIL, 1)]);
        assert_eq!(
            do_prepare_soil(&c, &mut task, true),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL
            }
        );
        // Inactive â†’ none
        task.soil_maker = 0.0;
        let c = counts_with(&[(FERTILE_SOIL_PILE, 6)]); // 2*6=12 units
        assert_eq!(do_prepare_soil(&c, &mut task, true), FarmAction::None);
    }

    #[test]
    fn do_composting_crafts_790_when_stock_low() {
        let mut task = FarmTaskState::default();
        let c = FarmCounts::default(); // stock 0 â†’ enter composting
        assert_eq!(
            do_composting(&c, &mut task),
            FarmAction::CraftItem {
                object_id: COMPOSTING_PILE
            }
        );
        assert!(task.composting >= 1.0);
        // High stock exits
        let c = counts_with(&[(COMPOSTING_PILE, 2), (COMPOSTED_SOIL, 2)]);
        let a = do_composting(&c, &mut task);
        assert_eq!(task.composting, 0.0);
        assert_eq!(a, FarmAction::None);
    }

    #[test]
    fn do_composting_wet_compost_625_recount_when_piles_present() {
        let mut task = FarmTaskState {
            composting: 1.0,
            ..Default::default()
        };
        let c = counts_with(&[(COMPOSTING_PILE, 1)]);
        assert_eq!(
            do_composting(&c, &mut task),
            FarmAction::CraftItem {
                object_id: WET_COMPOST
            }
        );
        let c2 = counts_with(&[(COMPOSTING_PILE, 1), (WET_COMPOST, 1)]);
        assert_eq!(
            do_composting(&c2, &mut task),
            FarmAction::CraftItem {
                object_id: COMPOSTING_PILE
            }
        );
    }

    /// Farm counts past early/mid plant+harvest+compost so doBasicFarming hits mid sheep.
    fn counts_past_basic_mid_wave() -> FarmCounts {
        counts_with(&[
            (DRIED_CORN, 5),
            (THRESHED_WHEAT, 4),
            (DRY_PLANTED_CORN, 10),
            (DRY_PLANTED_WHEAT, 15),
            (DRY_PLANTED_TOMATO, 5),
            (DRY_PLANTED_BEANS, 4),
            (DRY_PLANTED_CUCUMBER, 4),
            (DRY_PLANTED_PEPPER, 5),
            (COMPOSTING_PILE, 2),
            (COMPOSTED_SOIL, 2),
        ])
    }

    #[test]
    fn do_basic_farming_mid_defers_sheep_herding() {
        let mut task = FarmTaskState {
            composting: 0.0,
            corn_planter: 0.0,
            harvest_corn: 0.0,
            wheat_harvester: 1.0,
            ..Default::default()
        };
        let c = counts_past_basic_mid_wave();
        // Default maxProfession=2 carried for late doAdvancedFarming(2)
        assert_eq!(
            do_basic_farming(&c, &mut task, true, BASIC_FARM_DEFAULT_MAX_PROFESSION),
            FarmAction::DeferSheepHerding {
                max_profession: 2
            }
        );
        // Assigned BASICFARMER: doBasicFarming(100) â†’ advanced max 100
        assert_eq!(
            do_basic_farming(&c, &mut task, true, BASIC_FARM_ASSIGNED_MAX_PROFESSION),
            FarmAction::DeferSheepHerding {
                max_profession: 100
            }
        );
        // Haxe: profession['BASICFARMER']=1 before isSheepHerding
        assert_eq!(
            FarmAction::DeferSheepHerding {
                max_profession: 2
            }
            .basic_farmer_weight_side_effect(),
            Some(1.0)
        );
        let mut rt = FarmProfessionRuntime::default();
        apply_basic_farmer_weight_side_effect(
            &mut rt,
            FarmAction::DeferSheepHerding {
                max_profession: 2,
            },
        );
        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&1.0));
    }

    #[test]
    fn basic_farmer_weight_from_runtime_default_and_sticky() {
        let rt = FarmProfessionRuntime::default();
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 1.0);
        let mut rt = FarmProfessionRuntime::default();
        apply_basic_farmer_weight_side_effect(
            &mut rt,
            FarmAction::DeferSheepHerding {
                max_profession: 2,
            },
        );
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 1.0);
        apply_basic_farmer_weight_side_effect(&mut rt, FarmAction::ClearBasicFarmerWeight);
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 0.0);
        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&0.0));
    }

    #[test]
    fn do_basic_farming_after_sheep_late_plants_sharpie_advanced() {
        let mut task = FarmTaskState {
            corn_planter: 0.0,
            ..Default::default()
        };
        // Late wheat/corn caps already met â†’ skip plants â†’ sharpie none â†’ advanced
        let c = counts_with(&[(DRY_PLANTED_WHEAT, 30), (DRY_PLANTED_CORN, 12)]);
        assert_eq!(
            do_basic_farming_after_sheep(&c, &mut task, 15.0, 2),
            FarmAction::DeferAdvancedFarming {
                max_profession: 2
            }
        );
        // Age â‰¥20 skips sharpie even when burdock present (still past late plant caps)
        let c_burdock = counts_with(&[
            (DRY_PLANTED_WHEAT, 30),
            (DRY_PLANTED_CORN, 12),
            (BURDOCK, 1),
        ]);
        assert_eq!(
            do_basic_farming_after_sheep(&c_burdock, &mut task, 20.0, 2),
            FarmAction::DeferAdvancedFarming {
                max_profession: 2
            }
        );
        // Age <20 + burdock â†’ makeSharpieFood (need sharp stone)
        assert_eq!(
            do_basic_farming_after_sheep(&c_burdock, &mut task, 15.0, 2),
            FarmAction::CraftItem {
                object_id: SHARP_STONE
            }
        );
        // Holding sharp + burdock â†’ dug burdock
        let mut c_hold = c_burdock.clone();
        c_hold.held_id = SHARP_STONE;
        assert_eq!(
            do_basic_farming_after_sheep(&c_hold, &mut task, 15.0, 2),
            FarmAction::CraftItem {
                object_id: DUG_BURDOCK
            }
        );
        // Expand advanced with soil/rows already idle â†’ clear BASICFARMER
        // High soil units + deep rows so prepare_rows/soil hysteresis stay off.
        let c_adv = counts_with(&[
            (FERTILE_SOIL_PILE, 6),
            (DEEP_TILLED_ROW, 12),
            (CLAY_BOWL, 0), // bowl gate â†’ advanced step None â†’ clear
        ]);
        task.soil_maker = 0.0;
        task.row_maker = 3.0;
        assert_eq!(
            expand_advanced_farming_or_clear(&c_adv, &mut task, 25.0, true),
            FarmAction::ClearBasicFarmerWeight
        );
        assert_eq!(
            FarmAction::ClearBasicFarmerWeight.basic_farmer_weight_side_effect(),
            Some(0.0)
        );
    }

    #[test]
    fn make_sharpie_food_wild_carrot_and_burdock() {
        let mut c = counts_with(&[(SEEDING_WILD_CARROT, 1)]);
        assert_eq!(
            make_sharpie_food(&c),
            FarmAction::CraftItem {
                object_id: SHARP_STONE
            }
        );
        c.held_id = SHARP_STONE;
        assert_eq!(
            make_sharpie_food(&c),
            FarmAction::CraftItem {
                object_id: DUG_WILD_CARROT
            }
        );
        assert_eq!(make_sharpie_food(&FarmCounts::default()), FarmAction::None);
    }

    #[test]
    fn do_carrot_farming_aborts_when_carrots_gt_10() {
        let mut task = FarmTaskState::default();
        // Enough soil/rows so prepare_rows/soil short-circuit (Haxe checks cap after rows).
        let c = counts_with(&[
            (CARROT, 11),
            (FERTILE_SOIL_PILE, 6), // 2*6=12 soil units â†’ SoilMaker off
            (DEEP_TILLED_ROW, 12),  // RowMaker deep done
            (SHALLOW_TILLED_ROW, 0),
        ]);
        task.soil_maker = 0.0;
        task.row_maker = 3.0;
        assert_eq!(
            do_carrot_farming(&c, &mut task, true),
            FarmAction::Abort
        );
    }

    #[test]
    fn do_berry_farming_waters_216_and_plants_to_max_bushes() {
        let mut task = FarmTaskState::default();
        // Water dry seed when enough dry; soil/rows idle so watering runs.
        let c = counts_with(&[
            (DRY_PLANTED_GOOSEBERRY, 4),
            (FERTILE_SOIL_PILE, 6),
            (DEEP_TILLED_ROW, 12),
        ]);
        task.soil_maker = 0.0;
        task.row_maker = 3.0;
        let a = do_berry_farming(&c, &mut task, true);
        assert_eq!(
            a,
            FarmAction::CraftItem {
                object_id: WET_PLANTED_GOOSEBERRY
            }
        );
        // Plant when below max bushes
        let mut task = FarmTaskState::default();
        let mut c = counts_with(&[(DRY_PLANTED_GOOSEBERRY, 0)]);
        c.basic_farmer_weight = 1.0;
        let a = do_plant_bushes(&c, &mut task);
        assert_eq!(
            a,
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_GOOSEBERRY
            }
        );
        // At max (3) stop
        let c = counts_with(&[(DOMESTIC_BUSH, 3)]);
        assert_eq!(do_plant_bushes(&c, &mut task), FarmAction::None);
    }

    #[test]
    fn age_job_index_0_berry_1_basic_farm_mapping() {
        // age_job_index: round(age/5) % 5
        // age 0 â†’ 0 berry; age 5 â†’ 1 basic
        assert_eq!(
            age_rotated_farm_profession(0.0),
            Some(FarmProfession::BerryFarmer)
        );
        assert_eq!(
            age_rotated_farm_profession(5.0),
            Some(FarmProfession::BasicFarmer)
        );
        assert_eq!(age_rotated_farm_profession(10.0), None); // bake
        assert_eq!(age_job_index(0.0), 0);
        assert_eq!(age_job_index(5.0), 1);
    }

    #[test]
    fn speech_farmer_and_wheat_assign_basicfarmer() {
        assert_eq!(
            parse_farm_profession_speech("FARMER!"),
            Some(FarmProfession::BasicFarmer)
        );
        assert_eq!(
            parse_farm_profession_speech("WHEAT!"),
            Some(FarmProfession::BasicFarmer)
        );
        assert_eq!(
            parse_farm_profession_speech("CARROT!"),
            Some(FarmProfession::CarrotFarmer)
        );
        assert_eq!(
            parse_farm_profession_speech("BASICFARMER!"),
            Some(FarmProfession::BasicFarmer)
        );
    }

    #[test]
    fn selfplay_farmer_seeks_intermediate_ingredient_for_242() {
        let mut g = ReverseCraftGraph::new();
        // 228 + 0 â†’ 242 (fake path for test)
        g.insert(228, 0, 242, 0);
        // 1138 + 850 â†’ 228
        g.insert(1138, 850, 228, 0);
        let have = HashSet::new();
        let goal = pick_farmer_goal(&g, &have);
        // Should seek an ingredient toward ripe wheat pipeline
        match goal {
            Goal::SeekObject(id) => {
                assert!(id == 228 || id == 1138 || id == 850 || id == 242, "got {id}");
            }
            other => panic!("expected SeekObject, got {other:?}"),
        }
        // If we have 228, still may seek 242 or other missing pipeline head
        let mut have = HashSet::new();
        have.insert(228);
        let goal = pick_farmer_goal(&g, &have);
        assert!(matches!(goal, Goal::SeekObject(_)));
    }

    #[test]
    fn goal_from_rung_assigned_job_returns_job_not_seek_object_alone() {
        let g = goal_from_rung(
            PriorityRung::AssignedJob,
            Profession::Farmer,
            0,
            false,
            false,
            false,
        );
        // Ladder maps Job band â†’ SeekObject(farmer wheat) until Goal::Job lands.
        assert_eq!(g, Goal::SeekObject(FARMER_TARGET_ID));
        let g = goal_from_rung(
            PriorityRung::AgeRotatedJob,
            Profession::Farmer,
            0,
            false,
            false,
            false,
        );
        assert_eq!(g, Goal::SeekObject(FARMER_TARGET_ID));
        // Idle fallthrough still seeks wheat for farmer
        let g = goal_from_rung(
            PriorityRung::Idle,
            Profession::Farmer,
            0,
            false,
            false,
            false,
        );
        assert_eq!(g, Goal::SeekObject(FARMER_TARGET_ID));
    }

    #[test]
    fn farm_profession_as_str_matches_haxe_keys() {
        assert_eq!(FarmProfession::BasicFarmer.as_str(), "BASICFARMER");
        assert_eq!(FarmProfession::BerryFarmer.as_str(), "BerryFarmer");
        assert_eq!(
            assigned_job_farm_profession(Some("BASICFARMER"), None),
            Some(FarmProfession::BasicFarmer)
        );
        assert_eq!(
            assigned_job_farm_profession(None, Some("CARROTFARMER")),
            Some(FarmProfession::CarrotFarmer)
        );
    }

    // â”€â”€ AI-JOB-FARM-WIRE / farm_spatial â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn fill_farm_counts_map_radius_and_piles() {
        let objs = [
            FarmMapObj::simple(RIPE_WHEAT, 1, 0),
            FarmMapObj::simple(RIPE_WHEAT, 2, 0),
            FarmMapObj::simple(DRY_PLANTED_CARROTS, 5, 0),
            FarmMapObj::pile(FERTILE_SOIL_PILE, 3, 0, 4),
            FarmMapObj::simple(RIPE_WHEAT, 50, 0), // outside r=30
        ];
        let c = fill_farm_counts_from_map(0, 0, CARROT, &objs, FARM_COUNT_RADIUS);
        assert_eq!(c.get(RIPE_WHEAT), 2);
        assert_eq!(c.get(DRY_PLANTED_CARROTS), 1);
        assert_eq!(c.get(FERTILE_SOIL_PILE), 4);
        assert_eq!(c.held_id, CARROT);
        let c_near = fill_farm_counts_from_map(0, 0, 0, &objs, 4);
        assert_eq!(c_near.get(RIPE_WHEAT), 2);
        assert_eq!(c_near.get(DRY_PLANTED_CARROTS), 0); // dist 5 > 4
    }

    #[test]
    fn count_close_objects_at_and_corn_seeds() {
        let objs = [
            FarmMapObj::simple(1115, 1, 0),
            FarmMapObj::simple(1247, 2, 0),
            FarmMapObj::simple(1115, 25, 0), // outside corn r=20 exclusive square
            FarmMapObj::simple(CARROT, 1, 1),
        ];
        assert_eq!(
            count_close_objects_at(0, 0, 1115, CORN_SEED_COUNT_RADIUS, &objs),
            1
        );
        assert_eq!(count_close_objects_at(0, 0, CARROT, 30, &objs), 1);
        assert_eq!(count_corn_seeds_near(0, 0, 1115, &objs), 3);
        assert_eq!(count_corn_seeds_near(0, 0, 0, &objs), 2);
        // Haxe countCorn: held only for 1115/1120/1247 â€” not 4106/4107
        assert_eq!(count_corn_seeds_near(0, 0, 4106, &objs), 2);
        assert_eq!(count_corn_seeds_near(0, 0, 4107, &objs), 2);
        assert_eq!(count_with_held(2, CARROT, CARROT), 3);
    }

    #[test]
    fn count_close_objects_pile_parent_and_specials() {
        // Pile parent â‰  obj_id: contributes numberOfUses only when count_piles
        let objs = [
            FarmMapObj::simple(CARROT, 1, 0),
            FarmMapObj::pile(9999, 2, 0, 5), // pile form of carrot (table)
            FarmMapObj::simple(WET_CLAY_BOWL_ID, 0, 1),
            FarmMapObj::pile(240, 1, 1, 3), // pretend crock pile for bowl 233
            FarmMapObj::simple(BIG_CHARCOAL_PILE_ID, 0, 2),
            FarmMapObj::pile(HUGE_CHARCOAL_PILE_ID, 1, 2, 7),
        ];
        let table = &[(CARROT, 9999)];
        assert_eq!(
            count_close_objects_with_piles(0, 0, CARROT, 30, &objs, table),
            1 + 5
        );
        // Without pile table â†’ only direct parent matches
        assert_eq!(count_close_objects_at(0, 0, CARROT, 30, &objs), 1);

        // obj 233: count_piles forced false even if pile table provided
        let bowl_table = &[(WET_CLAY_BOWL_ID, 240)];
        assert_eq!(
            count_close_objects_with_piles(0, 0, WET_CLAY_BOWL_ID, 30, &objs, bowl_table),
            1
        );
        // obj 300: pile remaps to 4102 (ignores table)
        assert_eq!(
            count_close_objects_at(0, 0, BIG_CHARCOAL_PILE_ID, 30, &objs),
            1 + 7
        );
        let (cp, pid) = count_close_pile_specials(WET_CLAY_BOWL_ID, true, 240);
        assert!(!cp);
        assert_eq!(pid, 240);
        let (cp2, pid2) = count_close_pile_specials(BIG_CHARCOAL_PILE_ID, true, -1);
        assert!(cp2);
        assert_eq!(pid2, HUGE_CHARCOAL_PILE_ID);
    }

    #[test]
    fn count_close_objects_haxe_square_vs_chebyshev() {
        // Haxe exclusive end: [tx-r, tx+r) â€” high edge excluded; low edge included
        let at_high = [FarmMapObj::simple(CARROT, 10, 0)];
        let at_low = [FarmMapObj::simple(CARROT, -10, 0)];
        let at_corner = [FarmMapObj::simple(CARROT, 10, 10)];
        assert!(!in_count_close_square(0, 0, 10, 0, 10));
        assert!(in_count_close_square(0, 0, -10, 0, 10));
        assert_eq!(farm_chebyshev(0, 0, 10, 0), 10); // chebyshev would include
        assert_eq!(count_close_objects_at(0, 0, CARROT, 10, &at_high), 0);
        assert_eq!(count_close_objects_at(0, 0, CARROT, 10, &at_low), 1);
        assert_eq!(count_close_objects_at(0, 0, CARROT, 10, &at_corner), 0);
        // Inside exclusive square
        let inside = [FarmMapObj::simple(CARROT, 9, 0)];
        assert_eq!(count_close_objects_at(0, 0, CARROT, 10, &inside), 1);
    }

    #[test]
    fn count_close_objects_ignored_floor_skip() {
        // Origin on bear skin rug 656: non-food non-permanent skipped
        let objs = [
            FarmMapObj::simple(CARROT, 1, 0), // non-food default â†’ skip
            FarmMapObj::simple(RIPE_WHEAT, 2, 0).food(), // food kept
            FarmMapObj::simple(FERTILE_SOIL, 3, 0).permanent(), // permanent kept
        ];
        let opts = CountCloseOpts {
            origin_floor_id: 656,
            ..CountCloseOpts::default()
        };
        assert_eq!(
            count_close_objects_ex(0, 0, CARROT, 30, &objs, opts),
            0
        );
        assert_eq!(
            count_close_objects_ex(0, 0, RIPE_WHEAT, 30, &objs, opts),
            1
        );
        assert_eq!(
            count_close_objects_ex(0, 0, FERTILE_SOIL, 30, &objs, opts),
            1
        );
        // Bulk fill with origin floor
        let c = fill_farm_counts_from_map_with_floor(0, 0, 0, &objs, 30, 656);
        assert_eq!(c.get(CARROT), 0);
        assert_eq!(c.get(RIPE_WHEAT), 1);
        assert_eq!(c.get(FERTILE_SOIL), 1);
        assert!(is_ignored_floor(656, false, false, &AI_IGNORED_FLOOR_IDS));
        assert!(!is_ignored_floor(656, true, false, &AI_IGNORED_FLOOR_IDS));
        assert!(!is_ignored_floor(0, false, false, &AI_IGNORED_FLOOR_IDS));
    }

    #[test]
    fn farm_counts_from_nearby_and_ex() {
        let c = farm_counts_from_nearby(
            &[(RIPE_WHEAT, 3), (CARROT, 2)],
            CARROT,
            true,
            1.0,
            Some(4),
        );
        assert_eq!(c.get(RIPE_WHEAT), 3);
        assert_eq!(c.get(CARROT), 2);
        assert!(c.is_hungry);
        assert_eq!(c.basic_farmer_weight, 1.0);
        assert_eq!(c.hardened_row_biome, Some(4));

        let objs = [FarmMapObj::simple(RIPE_WHEAT, 1, 0)];
        let c2 = fill_farm_counts_from_map_ex(0, 0, 0, &objs, 30, true, 7.0, None);
        assert_eq!(c2.get(RIPE_WHEAT), 1);
        assert!(c2.is_hungry);
        assert_eq!(c2.basic_farmer_weight, 7.0);
    }

    #[test]
    fn soil_units_from_map_doubles_pile() {
        let objs = [
            FarmMapObj::pile(FERTILE_SOIL_PILE, 0, 0, 2),
            FarmMapObj::simple(FERTILE_SOIL, 1, 0),
            FarmMapObj::simple(DEEP_TILLED_ROW, 2, 0),
        ];
        // fill piles as uses=2 on one tile â†’ get(pile)=2; soil = 2*2 + 1 + 1 = 6
        assert_eq!(soil_units_from_map(0, 0, &objs), 6);
    }

    #[test]
    fn farm_action_to_goal_and_try_decide_from_rung() {
        assert_eq!(
            farm_action_to_goal(FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }),
            Goal::SeekObject(HARVESTED_WHEAT)
        );
        assert_eq!(
            farm_action_to_goal(FarmAction::ShortCraft {
                actor: 0,
                target: 400
            }),
            Goal::SeekObject(400)
        );
        assert_eq!(
            farm_action_to_goal(FarmAction::None),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
        assert_eq!(
            farm_action_to_goal(FarmAction::Abort),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
        assert!(farm_job_rung_label("ASSIGNED_JOB"));
        assert!(farm_job_rung_label("AGE_ROTATED_JOB"));
        assert!(!farm_job_rung_label("ESCAPE"));
        assert_eq!(farm_max_people_for_dispatch(true, 2), 100);
        assert_eq!(farm_max_people_for_dispatch(false, 2), 2);
        assert_eq!(
            farm_job_for_age_label("BERRY"),
            Some(FarmProfession::BerryFarmer)
        );
        assert_eq!(
            farm_job_for_age_label("BASIC"),
            Some(FarmProfession::BasicFarmer)
        );

        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(RIPE_WHEAT, 2)]);
        assert!(try_decide_farm_from_rung(
            Some(FarmProfession::BasicFarmer),
            "ESCAPE",
            &counts,
            &mut task,
            true,
        )
        .is_none());
        assert!(try_decide_farm_from_rung(None, "ASSIGNED_JOB", &counts, &mut task, true).is_none());
        let a = try_decide_farm_from_rung(
            Some(FarmProfession::BasicFarmer),
            "ASSIGNED_JOB",
            &counts,
            &mut task,
            true,
        )
        .unwrap();
        assert_eq!(
            a,
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );

        // AGE_ROTATED_JOB + BerryFarmer vs ESCAPE None
        let berry_counts = counts_with(&[]);
        let mut task_b = FarmTaskState::default();
        assert!(try_decide_farm_from_rung(
            Some(FarmProfession::BerryFarmer),
            "ESCAPE",
            &berry_counts,
            &mut task_b,
            true,
        )
        .is_none());
        assert!(try_decide_farm_from_rung(
            Some(FarmProfession::BerryFarmer),
            "AGE_ROTATED_JOB",
            &berry_counts,
            &mut task_b,
            true,
        )
        .is_some());

        let objs = [FarmMapObj::simple(RIPE_WHEAT, 1, 0)];
        let c2 = fill_farm_counts_from_map(0, 0, 0, &objs, 30);
        let mut task2 = FarmTaskState::default();
        let a2 = decide_farm_job(
            FarmProfession::BasicFarmer,
            &c2,
            &mut task2,
            true,
            BASIC_FARM_DEFAULT_MAX_PROFESSION,
        );
        assert_eq!(
            a2,
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );
        assert_eq!(farm_radius_table()[0].0, FARM_COUNT_RADIUS);

        // Map fill â†’ try_decide â†’ farm_action_to_goal (ladder composition)
        let ripe = [FarmMapObj::simple(RIPE_WHEAT, 1, 0)];
        let mut task3 = FarmTaskState::default();
        let g = farm_goal_from_map_and_rung(
            Some(FarmProfession::BasicFarmer),
            "ASSIGNED_JOB",
            0,
            0,
            0,
            &ripe,
            30,
            &mut task3,
            true,
            false,
            0.0,
            None,
        )
        .unwrap();
        assert_eq!(g, Goal::SeekObject(HARVESTED_WHEAT));

        // Dry planted wheat â‰¥3 with planter latched â†’ watering CraftItem(wet)
        let mut task_w = FarmTaskState::default();
        task_w.corn_planter = 1.0;
        let water_counts = counts_with(&[(DRY_PLANTED_WHEAT, 3)]);
        let aw = do_plant_wheat(2, 5, &water_counts, &mut task_w);
        assert_eq!(
            aw,
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_eq!(
            farm_action_to_goal(aw),
            Goal::SeekObject(WET_PLANTED_WHEAT)
        );

        // ESCAPE band â†’ None composition
        let mut task4 = FarmTaskState::default();
        assert!(farm_goal_from_map_and_rung(
            Some(FarmProfession::BasicFarmer),
            "ESCAPE",
            0,
            0,
            0,
            &ripe,
            30,
            &mut task4,
            true,
            false,
            0.0,
            None,
        )
        .is_none());
        let mut task5 = FarmTaskState::default();
        let g5 = farm_goal_from_counts_and_rung(
            Some(FarmProfession::BasicFarmer),
            "ASSIGNED_JOB",
            &counts,
            &mut task5,
            true,
        )
        .unwrap();
        assert_eq!(g5, Goal::SeekObject(HARVESTED_WHEAT));
    }

    // â”€â”€ AI-JOB-FARM-LIVE â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn keep_bushes_alive_emits_short_craft_when_below_min() {
        // Haxe: keepBushesAlive â€” bush sum <20 â†’ shortCraft(1137,389); >=20 None
        let low = counts_with(&[(DOMESTIC_BUSH, 5), (DRY_DOMESTIC_BUSH, 2)]);
        assert_eq!(keep_bushes_alive_count(&low), 7);
        assert_eq!(
            keep_bushes_alive(&low),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        let empty = counts_with(&[]);
        assert_eq!(
            keep_bushes_alive(&empty),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        let high = counts_with(&[
            (DOMESTIC_BUSH, 10),
            (DRY_DOMESTIC_BUSH, 5),
            (VIGOROUS_DOMESTIC_BUSH, 3),
            (EMPTY_DOMESTIC_BUSH, 2),
        ]);
        assert_eq!(keep_bushes_alive_count(&high), 20);
        assert_eq!(keep_bushes_alive(&high), FarmAction::None);

        // do_prepare_rows only interrupts when dying present (shortCraft target)
        let mut task = FarmTaskState::default();
        let low_no_dying = counts_with(&[(DOMESTIC_BUSH, 1)]);
        let a_rows = do_prepare_rows(&low_no_dying, &mut task, true, false);
        // No dying â†’ keepBushes does not block; may craft shallow row etc.
        assert_ne!(
            a_rows,
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        let mut task2 = FarmTaskState::default();
        let low_dying = counts_with(&[(DOMESTIC_BUSH, 1), (DYING_BUSH, 1)]);
        assert_eq!(
            do_prepare_rows(&low_dying, &mut task2, true, false),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
    }

    #[test]
    fn assign_farm_from_speech_sets_assigned_and_last() {
        let mut rt = FarmProfessionRuntime::default();
        assert!(!assign_farm_from_speech(&mut rt, "BAKER!"));
        assert!(assign_farm_from_speech(&mut rt, "FARMER!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::BasicFarmer));
        assert_eq!(rt.last_profession, Some(FarmProfession::BasicFarmer));
        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&1.0));
        assert!(assign_farm_from_speech(&mut rt, "WHEAT!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::BasicFarmer));
        assert!(assign_farm_from_speech(&mut rt, "CARROT!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::CarrotFarmer));
        assert!(assign_farm_from_speech(&mut rt, "ROW!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::RowMaker));
        assert!(assign_farm_from_speech(&mut rt, "SOIL!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::SoilMaker));
        assert!(assign_farm_from_speech(&mut rt, "WATER!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::WaterBringer));
        assert!(assign_farm_from_speech(&mut rt, "BERRY!"));
        assert_eq!(rt.assigned_profession, Some(FarmProfession::BerryFarmer));
    }

    #[test]
    fn short_craft_apply_edges_snow_skewer_carrot_max() {
        // Held matches â†’ UseOnTarget
        let on_target = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        });
        assert_eq!(
            on_target,
            ShortCraftApply::UseOnTarget {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        // Missing actor â†’ seek
        let seek = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(0, BOWL_OF_SOIL, DYING_BUSH)
        });
        assert_eq!(
            seek,
            ShortCraftApply::SeekOrCraftActor {
                actor: BOWL_OF_SOIL,
                craft_if_needed: true,
            }
        );
        // actor 0, hands full â†’ drop
        let drop = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(CARROT, 0, CARROT_ROW)
        });
        assert_eq!(drop, ShortCraftApply::DropHeld);
        // Skewer prefer weak
        assert_eq!(
            short_craft_apply(ShortCraftInput::basic(0, SKEWER, TOMATO_SPROUT)),
            ShortCraftApply::PreferWeakSkewer
        );
        // Snow refuse soil on hardened row
        let snow = short_craft_apply(ShortCraftInput {
            target_biome: Some(SNOW_BIOME),
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, HARDENED_ROW)
        });
        assert_eq!(snow, ShortCraftApply::Refuse);
        // Ocean refuse hoe on fertile soil
        let ocean = short_craft_apply(ShortCraftInput {
            target_biome: Some(OCEAN_BIOME),
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(STEEL_HOE, STEEL_HOE, FERTILE_SOIL)
        });
        assert_eq!(ocean, ShortCraftApply::Refuse);
        // Carrot row seed guard
        let seed = short_craft_apply(ShortCraftInput {
            target_uses: 3,
            has_carrot_seeds: false,
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(0, 0, CARROT_ROW)
        });
        assert_eq!(seed, ShortCraftApply::Refuse);
        // maxNewActor
        let maxed = short_craft_apply(ShortCraftInput {
            new_actor_count: 5,
            max_new_actor: 5,
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        });
        assert_eq!(maxed, ShortCraftApply::Refuse);
        // Hungry refuse (farm path always-on gate)
        let hungry = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            food_store: 1.0,
            transition_hungry_cost: 2.0,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        });
        assert_eq!(hungry, ShortCraftApply::RefuseHungry);
        // maxNewActor includes held newActor
        assert_eq!(new_actor_count_with_held(3, 999, 999), 4);
        assert_eq!(new_actor_count_with_held(3, 1, 999), 3);

        // farm_action bridge
        let step = farm_action_short_craft_apply(
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            },
            BOWL_OF_SOIL,
            1,
            None,
            true,
            0,
            -1,
        );
        assert_eq!(
            step,
            Some(ShortCraftApply::UseOnTarget {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            })
        );
        assert!(farm_action_short_craft_apply(
            FarmAction::None,
            0,
            1,
            None,
            true,
            0,
            -1
        )
        .is_none());
    }

    #[test]
    fn do_critical_farm_slice_age_gated_bushes_and_basic() {
        // age 10 â†’ round(10/5)=2 even â†’ keepBushes when dying present
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(DOMESTIC_BUSH, 1), (DYING_BUSH, 1)]);
        assert_eq!(
            do_critical_farm_slice(10.0, &c, &mut task, false, false),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        // age 5 â†’ round(5/5)=1 odd â†’ skip keepBushes; basic with ripe wheat
        let mut task2 = FarmTaskState::default();
        let wheat = counts_with(&[(RIPE_WHEAT, 2)]);
        assert_eq!(
            do_critical_farm_slice(5.0, &wheat, &mut task2, true, false),
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );
        // sticky BasicFarmer job + map fill still works after speech assign
        let mut rt = FarmProfessionRuntime::default();
        assert!(assign_farm_from_speech(&mut rt, "FARMER!"));
        let mut task3 = FarmTaskState::default();
        let objs = [FarmMapObj::simple(RIPE_WHEAT, 1, 0)];
        let g = farm_goal_from_map_and_rung(
            resolve_farm_assigned_job(&rt),
            "ASSIGNED_JOB",
            0,
            0,
            0,
            &objs,
            30,
            &mut task3,
            true,
            false,
            0.0,
            None,
        )
        .unwrap();
        assert_eq!(g, Goal::SeekObject(HARVESTED_WHEAT));
    }
}
