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

use ol_ai_helper::ai_goals::priority_ladder::age_job_index;
use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use ol_ai_helper::ai_goals::{Goal, FARMER_TARGET_ID};
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
/// Wet Planted Squash Seeds.
pub const WET_PLANTED_SQUASH: i32 = 1190;
/// Dry Planted Garlic.
// Haxe: AiBase.doAdvancedFarming L3998
pub const DRY_PLANTED_GARLIC: i32 = 4262;
/// Wet Planted Garlic.
// Haxe: AiBase.doAdvancedFarming L3998
pub const WET_PLANTED_GARLIC: i32 = 4263;
/// Mature Garlic (CountClose with dry planted garlic).
// Haxe: AiBase.doAdvancedFarming L4002
pub const MATURE_GARLIC: i32 = 4265;
/// Hubbard Squash.
pub const HUBBARD_SQUASH: i32 = 1199;
/// Ripe Squash Plant.
pub const RIPE_SQUASH_PLANT: i32 = 1196;
/// Crock with Squash.
pub const CROCK_WITH_SQUASH: i32 = 1243;
/// Plate of Squash Chunks.
pub const PLATE_SQUASH_CHUNKS: i32 = 1202;
/// Plate of Squash Chunks with Seeds.
pub const PLATE_SQUASH_CHUNKS_SEEDS: i32 = 1201;
/// Dry Planted Milkweed Seed.
pub const DRY_PLANTED_MILKWEED: i32 = 214;
/// Wet Planted Milkweed Seed.
pub const WET_PLANTED_MILKWEED: i32 = 215;
/// Milkweed Sprout.
pub const MILKWEED_SPROUT: i32 = 218;
/// Milkweed.
pub const MILKWEED: i32 = 50;
/// Flowering Milkweed.
pub const FLOWERING_MILKWEED: i32 = 51;
/// Fruiting Milkweed.
pub const FRUITING_MILKWEED: i32 = 52;
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
    /// Haxe `toPlant` rotation seed; `<= 0` re-rolls (Haxe `toPlant > 0 ? toPlant : rand`).
    // Haxe: AiBase.doAdvancedFarming L3969
    pub to_plant: i32,
    /// Haxe `taskState['EearOfCornMaker']` (typo kept).
    // Haxe: AiBase.isConsideringMakingFood L8570
    pub ear_of_corn_maker: f32,
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
    /// Clay bowls in home half-open square r=15 (`None` → use [`FarmCounts::get`] 235).
    // Haxe: AiBase.doAdvancedFarming L3928 CountCloseObjects r=15
    pub bowl_count_home_15: Option<i32>,
}

impl FarmCounts {
    pub fn get(&self, id: i32) -> i32 {
        *self.by_id.get(&id).unwrap_or(&0)
    }

    /// Haxe `countCurrentObject` — map count plus held parent match.
    // Haxe: AiBase.countCurrentObjectHelper L3448
    pub fn get_with_held(&self, id: i32) -> i32 {
        self.get(id) + if self.held_id == id { 1 } else { 0 }
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
    /// Haxe `this.profession['ADVANCEDFARMER'] = 0` after rotation miss.
    // Haxe: AiBase.doAdvancedFarming L4070
    ClearAdvancedFarmerWeight,
    /// Haxe `doCarrotFarming` tail `if (cleanUp()) return true`.
    // Haxe: AiBase.doCarrotFarming L1986
    DeferCleanup,
    /// Haxe `doPrepareRows` `countBowls < 1 && doPottery(maxProfession)` when deepRows < 6.
    // Haxe: AiBase.doPrepareRows L2208–2214
    DeferPottery { max_profession: i32 },
}

impl FarmAction {
    pub fn is_some(self) -> bool {
        !matches!(
            self,
            Self::None | Self::Abort | Self::ClearBasicFarmerWeight | Self::ClearAdvancedFarmerWeight
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

    /// Haxe `profession['ADVANCEDFARMER']` write, if any.
    // Haxe: AiBase.doAdvancedFarming L4070
    pub fn advanced_farmer_weight_side_effect(self) -> Option<f32> {
        match self {
            Self::ClearAdvancedFarmerWeight => Some(0.0),
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

/// Apply Haxe `profession['ADVANCEDFARMER'] = 0` after rotation miss.
// Haxe: AiBase.doAdvancedFarming L4070
pub fn apply_advanced_farmer_weight_side_effect(
    runtime: &mut FarmProfessionRuntime,
    action: FarmAction,
) {
    if let Some(w) = action.advanced_farmer_weight_side_effect() {
        runtime.weights.insert(FarmProfession::AdvancedFarmer, w);
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

/// Default wet product for Bowl of Water 382 + dry planted id (`trans.newTargetID`).
///
/// Full table lives in content transitions; this covers farm pipeline ids.
// Haxe: AiBase.doWateringOn L2601–2609 TransitionImporter.GetTransition(382, itemToWaterId)
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
        DRY_PLANTED_SQUASH => WET_PLANTED_SQUASH,
        DRY_PLANTED_MILKWEED => WET_PLANTED_MILKWEED,
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
    // Haxe: GetTransition(382, id); trans==null → false; else craftItem(trans.newTargetID)
    // Haxe: AiBase.doWateringOn L2601–2609
    let wet = wet_product.or_else(|| default_wet_from_bowl(item_to_water_id));
    match wet {
        Some(id) => FarmAction::CraftItem { object_id: id },
        None => FarmAction::None,
    }
}

/// Haxe `shortCraftOnGround(actorId)`: held → use empty ground; else `GetItem`.
// Haxe: AiBase.shortCraftOnGround L2692–2711
pub fn short_craft_on_ground(held_id: i32, actor_id: i32) -> FarmAction {
    if held_id == actor_id {
        FarmAction::ShortCraft {
            actor: actor_id,
            target: 0,
        }
    } else {
        FarmAction::CraftItem {
            object_id: actor_id,
        }
    }
}

/// Default dry-plant ids for Haxe `ServerSettings.WateringTargetsIds` (farm subset).
///
/// Order matches the historical commented list in `doWateringHelper` and the
/// WaterBringer job body (carrots first).
// Haxe: AiBase.doWateringHelper ~3558 / commented ~3583–3593
pub const WATERING_TARGET_DRY_IDS: &[i32] = &[
    DRY_PLANTED_CARROTS,   // 396
    DRY_PLANTED_WHEAT,     // 228
    DRY_PLANTED_CORN,      // 1109
    DRY_PLANTED_TOMATO,    // 2829
    DRY_PLANTED_CUCUMBER,  // 4225
    DRY_DOMESTIC_BUSH,     // 393
    DRY_PLANTED_GOOSEBERRY,// 216
    DRY_PLANTED_ONIONS,    // 2851
    DRY_PLANTED_BEANS,     // 1161
    DRY_PLANTED_POTATO,    // 1145
];

/// Dry ids excluding carrots (Haxe `WateringTargetsIdsWithoutCarrots` when carrot stock high).
// Haxe: AiBase.doWateringHelper ~3567–3569
pub fn watering_targets_without_carrots() -> impl Iterator<Item = i32> {
    WATERING_TARGET_DRY_IDS
        .iter()
        .copied()
        .filter(|&id| id != DRY_PLANTED_CARROTS)
}

/// Haxe `doWatering` / `doWateringHelper` player-relative search distance.
// Haxe: AiBase.doWatering ~3548 `distance = 30`
pub const WATERING_SEARCH_DIST: i32 = 30;
/// Full Water Pouch — actor used to discover watering targets.
// Haxe: ServerSettings.InitWateringTargets L4039
pub const FULL_WATER_POUCH: i32 = 210;
/// Haxe `IgnoreToWaterNewTargets` (pouch pile / adobe / bowl / buckets / watered bush).
// Haxe: ServerSettings.IgnoreToWaterNewTargets L4036
pub const IGNORE_TO_WATER_NEW_TARGETS: [i32; 7] = [210, 4094, 127, 382, 660, 1099, 3946];

/// Pure Haxe `ServerSettings.InitWateringTargets`.
///
/// Edges are `(actor_id, target_id, new_target_id)` from pouch-210 transitions.
/// Skips `targetID < 1` (TIME) and ignored new targets; first-seen target wins.
/// `WithoutCarrots` omits edges whose **new** target is Dry Planted Carrots 396.
// Haxe: ServerSettings.InitWateringTargets L4038–4054
pub fn init_watering_target_ids(
    transitions: impl IntoIterator<Item = (i32, i32, i32)>,
) -> (Vec<i32>, Vec<i32>) {
    let mut all: Vec<i32> = Vec::new();
    let mut without_carrots: Vec<i32> = Vec::new();
    for (actor_id, target_id, new_target_id) in transitions {
        if actor_id != FULL_WATER_POUCH {
            continue;
        }
        if target_id < 1 {
            continue;
        }
        if IGNORE_TO_WATER_NEW_TARGETS.contains(&new_target_id) {
            continue;
        }
        if all.contains(&target_id) {
            continue;
        }
        all.push(target_id);
        if new_target_id != DRY_PLANTED_CARROTS {
            without_carrots.push(target_id);
        }
    }
    (all, without_carrots)
}

/// Assigned/last WATERBRINGER: Haxe `doWatering(100)`.
// Haxe: AiBase.doTimeStuffHelper ~736
pub const WATER_BRINGER_ASSIGNED_MAX_PEOPLE: i32 = 100;
/// Low-priority `doWatering(1)` (after assigned WATERBRINGER / mid farm watering).
// Haxe: AiBase.doTimeStuffHelper ~779; doCriticalStuff ~6103
pub const WATER_BRINGER_LOW_MAX_PEOPLE: i32 = 1;
/// Ladder label for low `doWatering(1)` (not assigned WATERBRINGER).
// Haxe: doWatering(1) ~779 before jobByAge
pub const DO_WATERING_LOW_RUNG: &str = "DO_WATERING_LOW";

/// Haxe `doWatering(maxPeople)` peer-cap: assigned/last **100**, else **1**.
// Haxe: assigned ~736 doWatering(100); low ~779 doWatering(1)
pub fn watering_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        WATER_BRINGER_ASSIGNED_MAX_PEOPLE
    } else {
        WATER_BRINGER_LOW_MAX_PEOPLE
    }
}

/// Assigned/last CARROTFARMER: Haxe `doCarrotFarming(100)`.
// Haxe: AiBase.doTimeStuffHelper ~707
pub const CARROT_FARMER_ASSIGNED_MAX_PEOPLE: i32 = 100;
/// Low-priority `doCarrotFarming(1)` (after low `doWatering(1)`).
// Haxe: AiBase.doTimeStuffHelper ~780; doCriticalStuff ~6117
pub const CARROT_FARMER_LOW_MAX_PEOPLE: i32 = 1;
/// Ladder label for low `doCarrotFarming(1)` (not assigned CARROTFARMER).
// Haxe: doCarrotFarming(1) ~780 before jobByAge
pub const DO_CARROT_LOW_RUNG: &str = "DO_CARROT_LOW";

/// Haxe `doCarrotFarming(maxProfession)` peer-cap: assigned/last **100**, else **1**.
// Haxe: assigned ~707 doCarrotFarming(100); low ~780 doCarrotFarming(1)
pub fn carrot_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        CARROT_FARMER_ASSIGNED_MAX_PEOPLE
    } else {
        CARROT_FARMER_LOW_MAX_PEOPLE
    }
}

/// Bowl of Green Beans 1175.
// Haxe: AiBase.fillBeanBowlIfNeeded beanBowlId green
pub const BOWL_OF_GREEN_BEANS: i32 = 1175;
/// Bowl of Dry Beans 1176 (`countCurrentObjects` green-fill gate).
// Haxe: AiBase.fillBeanBowlIfNeeded countDryBeans [1176, 1172]
pub const BOWL_OF_DRY_BEANS: i32 = 1176;
/// Haxe `GetClosestObjectById` default searchDistance for plant/bowl.
// Haxe: AiHelper.GetClosestObjectById searchDistance = 40
pub const FILL_BEAN_BOWL_SEARCH_DIST: i32 = 40;
/// Ladder label for low `fillBeanBowlIfNeeded()` (green beans, after doCarrotFarming(1)).
// Haxe: AiBase.doTimeStuffHelper ~786
pub const FILL_BEAN_BOWL_RUNG: &str = "FILL_BEAN_BOWL";
/// Ladder label for mid `fillBeanBowlIfNeeded(*, true)` onlyFillHeld (before isHandlingFire).
// Haxe: AiBase.doTimeStuffHelper ~628–629
pub const FILL_BEAN_HELD_RUNG: &str = "FILL_BEAN_HELD";
/// Mid `shortCraft(0, 400, 10)` searchDistance (pull carrot row).
// Haxe: AiBase.doTimeStuffHelper ~609
pub const PULL_CARROT_ROW_SEARCH_DIST: i32 = 10;
/// Ladder label for mid `shortCraft(0, 400, 10)` (before fillBerryBowlIfNeeded(true)).
// Haxe: AiBase.doTimeStuffHelper ~609
pub const PULL_CARROT_ROW_RUNG: &str = "PULL_CARROT_ROW";
/// Clay Plate 236 + Cooked Omelette 1281 (don't let omelette burn).
// Haxe: AiBase.doTimeStuffHelper L624
pub const COOKED_OMELETTE: i32 = 1281;
pub const COOKED_OMELETTE_RUNG: &str = "COOKED_OMELETTE";
/// `doCriticalStuff` farm slice (bushes / basic / carrot).
// Haxe: AiBase.doCriticalStuff L6072–6127
pub const CRITICAL_STUFF_RUNG: &str = "CRITICAL_STUFF";
/// `doCriticalStuff` `placeFloorUnder(home/GetKiln/GetForge)` before bushes.
// Haxe: AiBase.doCriticalStuff L6091–6095
pub const PLACE_FLOOR_UNDER_RUNG: &str = "PLACE_FLOOR_UNDER";

/// Haxe `numberOfUses >= objectData.numUses`. Missing `numUses` (0) is not full.
fn bean_bowl_is_full(uses: i32, num_uses: i32) -> bool {
    num_uses > 0 && uses >= num_uses
}

/// Bowl / plant parent ids for `fillBeanBowlIfNeeded(greenBeans)`.
// Haxe: AiBase.fillBeanBowlIfNeeded ~4145–4148
pub fn bean_bowl_ids(green_beans: bool) -> (i32, i32) {
    if green_beans {
        (BOWL_OF_GREEN_BEANS, GREEN_BEAN_PLANTS)
    } else {
        (BOWL_OF_DRY_BEANS, DRY_BEAN_PLANTS)
    }
}

/// Sensors for Haxe `fillBeanBowlIfNeeded`.
// Haxe: AiBase.fillBeanBowlIfNeeded ~4143
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillBeanBowlInput {
    pub held_id: i32,
    pub held_uses: i32,
    pub held_num_uses: i32,
    pub green_beans: bool,
    pub only_fill_held: bool,
    pub count_dry_beans: i32,
    pub plant_xy: Option<(i32, i32)>,
    pub bowl_xy: Option<(i32, i32)>,
    pub bowl_uses: i32,
    pub bowl_num_uses: i32,
    pub is_best_bowl_filler: bool,
}

/// Haxe `fillBeanBowlIfNeeded` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillBeanBowlAction {
    None,
    /// `useHeldObjOnTarget(closeBeans)`.
    UseHeldOnPlant { x: i32, y: i32, plant_id: i32 },
    /// `dropTarget = closeBowl` pickup (empty-hand USE).
    PickupBowl { x: i32, y: i32, bowl_id: i32 },
    /// `GetItem(235)` clay bowl (no craft).
    GetClayBowl,
}

impl FillBeanBowlAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Haxe `fillBeanBowlIfNeeded(greenBeans, onlyFillHeld)`.
///
/// Low `fillBeanBowlIfNeeded()` is green + `onlyFillHeld=false`. Mid calls use
/// `onlyFillHeld=true` (held path only).
// Haxe: AiBase.fillBeanBowlIfNeeded ~4143
pub fn fill_bean_bowl_if_needed(inp: &FillBeanBowlInput) -> FillBeanBowlAction {
    let (bowl_id, plant_id) = bean_bowl_ids(inp.green_beans);
    if inp.held_id == bowl_id && bean_bowl_is_full(inp.held_uses, inp.held_num_uses) {
        return FillBeanBowlAction::None;
    }
    if inp.green_beans && inp.count_dry_beans < 1 {
        return FillBeanBowlAction::None;
    }
    let Some((px, py)) = inp.plant_xy else {
        return FillBeanBowlAction::None;
    };
    let holding_bowl = inp.held_id == bowl_id;
    let holding_clay_no_close_bowl = inp.held_id == CLAY_BOWL && inp.bowl_xy.is_none();
    if holding_bowl || holding_clay_no_close_bowl {
        return FillBeanBowlAction::UseHeldOnPlant {
            x: px,
            y: py,
            plant_id,
        };
    }
    if inp.only_fill_held {
        return FillBeanBowlAction::None;
    }
    if let Some((bx, by)) = inp.bowl_xy {
        if bean_bowl_is_full(inp.bowl_uses, inp.bowl_num_uses) {
            return FillBeanBowlAction::None;
        }
        if !inp.is_best_bowl_filler {
            return FillBeanBowlAction::None;
        }
        return FillBeanBowlAction::PickupBowl {
            x: bx,
            y: by,
            bowl_id,
        };
    }
    if !inp.is_best_bowl_filler {
        return FillBeanBowlAction::None;
    }
    FillBeanBowlAction::GetClayBowl
}

/// Mid `fillBeanBowlIfNeeded(true, true)` then `(false, true)` — held path only.
// Haxe: AiBase.doTimeStuffHelper ~628–629
pub fn fill_bean_bowl_held_if_needed(
    green: &FillBeanBowlInput,
    dry: &FillBeanBowlInput,
) -> FillBeanBowlAction {
    let a = fill_bean_bowl_if_needed(green);
    if a.is_some() {
        return a;
    }
    fill_bean_bowl_if_needed(dry)
}

/// Sensors for mid `shortCraft(0, 400, 10)`.
// Haxe: AiBase.doTimeStuffHelper ~609; shortCraftOnTarget ~2684
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PullCarrotRowInput {
    pub held_id: i32,
    pub food_store: f32,
    pub transition_hungry_cost: f32,
    pub has_carrot_seeds: bool,
    /// Closest carrot row 400 `(x, y, numberOfUses)`.
    pub row: Option<(i32, i32, i32)>,
}

/// Mid pull-carrot-row action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullCarrotRowAction {
    None,
    /// Empty hands: `useHeldObjOnTarget` carrot row 400.
    UseEmptyOnRow { x: i32, y: i32 },
    /// Holding something: Haxe `actorId == 0` → `dropHeldObject`.
    DropHeld,
}

impl PullCarrotRowAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Haxe mid `shortCraft(0, 400, 10)` — empty-hand USE on closest carrot row r=10.
///
/// No `hasOrBecomeProfession`. Missing row, hungry cost, and carrot-seed guard
/// (`!hasCarrotSeeds && uses < 4`) are no-ops. Hands full → drop first.
// Haxe: AiBase.doTimeStuffHelper ~609; shortCraftOnTarget ~2710 / ~2724 / ~2732
pub fn pull_carrot_row_if_needed(inp: &PullCarrotRowInput) -> PullCarrotRowAction {
    let Some((x, y, uses)) = inp.row else {
        return PullCarrotRowAction::None;
    };
    let sc = ShortCraftInput {
        held_id: inp.held_id,
        actor_id: 0,
        target_id: CARROT_ROW,
        target_uses: uses,
        target_biome: None,
        has_carrot_seeds: inp.has_carrot_seeds,
        new_actor_count: 0,
        max_new_actor: -1,
        try_weak_skewer_first: false,
        craft_actor_if_needed: true,
        food_store: inp.food_store,
        transition_hungry_cost: inp.transition_hungry_cost,
    };
    match short_craft_apply(sc) {
        ShortCraftApply::UseOnTarget { .. } => PullCarrotRowAction::UseEmptyOnRow { x, y },
        ShortCraftApply::DropHeld => PullCarrotRowAction::DropHeld,
        _ => PullCarrotRowAction::None,
    }
}

/// Squared Euclidean (Haxe `CalculateQuadDistanceHelper`).
fn watering_quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = (ax - bx) as i64;
    let dy = (ay - by) as i64;
    dx * dx + dy * dy
}

/// Closest watering-target parent id near the player.
///
/// Search box is Chebyshev ≤ `distance` (Haxe for-loop around player). Among
/// matches, pick smallest squared Euclidean (Haxe quad). Equal quad keeps the
/// first object (tests use unique distances).
// Haxe: AiHelper.GetClosestObjectToPositionByIds ~3558
pub fn closest_watering_parent_id(
    player_x: i32,
    player_y: i32,
    objects: &[FarmMapObj],
    ids: &[i32],
    distance: i32,
) -> Option<i32> {
    if ids.is_empty() {
        return None;
    }
    let dist = distance.max(0);
    let mut best: Option<(i64, i32)> = None;
    for o in objects {
        if o.parent_id == 0 || !ids.contains(&o.parent_id) {
            continue;
        }
        let cheb = (o.x - player_x).abs().max((o.y - player_y).abs());
        if cheb > dist {
            continue;
        }
        let q = watering_quad_dist(player_x, player_y, o.x, o.y);
        match best {
            None => best = Some((q, o.parent_id)),
            Some((bq, _)) if q < bq => best = Some((q, o.parent_id)),
            _ => {}
        }
    }
    best.map(|(_, id)| id)
}

/// Haxe `doWateringHelper` — first dry target with a watering action (list order).
///
/// Mid basic/shepherd `doWatering(3)` keeps this list-order walk. Assigned
/// WATERBRINGER uses [`do_watering_helper_closest`].
///
/// When carrot stock (`CARROT` 402) ≥ 20, skip dry planted carrots (Haxe).
// Haxe: AiBase.doWateringHelper commented list ~3583; mid farm reuse
pub fn do_watering_helper(counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    let skip_carrots = counts.get(CARROT) >= 20;
    if skip_carrots {
        for dry in watering_targets_without_carrots() {
            let a = do_watering_on(dry, 1, counts.get(dry), default_wet_from_bowl(dry), task);
            if a.is_some() {
                return a;
            }
        }
    } else {
        for &dry in WATERING_TARGET_DRY_IDS {
            let a = do_watering_on(dry, 1, counts.get(dry), default_wet_from_bowl(dry), task);
            if a.is_some() {
                return a;
            }
        }
    }
    FarmAction::None
}

/// Haxe `doWatering(maxPeople)` — WaterBringer peer-cap then [`do_watering_helper`].
// Haxe: AiBase.doWatering ~3548
pub fn do_watering(
    runtime: &mut FarmProfessionRuntime,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    max_people: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> FarmAction {
    if !has_or_become_profession(
        runtime,
        FarmProfession::WaterBringer,
        max_people,
        peer_count_with_last,
        was_idle,
    ) {
        return FarmAction::None;
    }
    do_watering_helper(counts, task)
}

/// Haxe `doWateringHelper` — closest dry target then `doWateringOn`.
///
/// When carrot stock ≥ 20 and the closest target is dry planted carrots, retarget
/// without carrots. On a found target whose watering step fails, zero WATERBRINGER
/// weight (Haxe `this.profession['WATERBRINGER']=0`). No target → None, weight unchanged.
// Haxe: AiBase.doWateringHelper ~3553
pub fn do_watering_helper_closest(
    runtime: &mut FarmProfessionRuntime,
    player_x: i32,
    player_y: i32,
    objects: &[FarmMapObj],
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    distance: i32,
) -> FarmAction {
    let mut target = closest_watering_parent_id(
        player_x,
        player_y,
        objects,
        WATERING_TARGET_DRY_IDS,
        distance,
    );
    if target.is_none() {
        return FarmAction::None;
    }
    if target == Some(DRY_PLANTED_CARROTS) && counts.get(CARROT) >= 20 {
        let skip: Vec<i32> = watering_targets_without_carrots().collect();
        target = closest_watering_parent_id(player_x, player_y, objects, &skip, distance);
    }
    let Some(dry) = target else {
        return FarmAction::None;
    };
    let a = do_watering_on(dry, 1, counts.get(dry), default_wet_from_bowl(dry), task);
    if a.is_some() {
        return a;
    }
    // Haxe L3629 this.profession['WATERBRINGER'] = 0 after doWateringOn fallthrough
    runtime.weights.insert(FarmProfession::WaterBringer, 0.0);
    FarmAction::None
}

/// Assigned WATERBRINGER `doWatering(maxPeople)` with closest-target helper.
// Haxe: AiBase.doWatering ~3548 then doWateringHelper
pub fn do_watering_closest(
    runtime: &mut FarmProfessionRuntime,
    player_x: i32,
    player_y: i32,
    objects: &[FarmMapObj],
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    max_people: i32,
    peer_count_with_last: f32,
    was_idle: f32,
    distance: i32,
) -> FarmAction {
    if !has_or_become_profession(
        runtime,
        FarmProfession::WaterBringer,
        max_people,
        peer_count_with_last,
        was_idle,
    ) {
        return FarmAction::None;
    }
    do_watering_helper_closest(
        runtime, player_x, player_y, objects, counts, task, distance,
    )
}

// â”€â”€ Plant hysteresis â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Sum of `countCurrentObject` for ids (map + held).
// Haxe: AiBase.countCurrentObjectHelper L3448
pub fn plant_stage_count(counts: &FarmCounts, ids: &[i32]) -> i32 {
    ids.iter().map(|&id| counts.get_with_held(id)).sum()
}

/// Haxe `doPlant(min, max, toPlantId, toCountIds)` — shared `CornPlanter` taskState.
// Haxe: AiBase.doPlant L2558–2584
pub fn do_plant(
    min_planted: i32,
    max_planted: i32,
    to_plant_id: i32,
    stage_count: i32,
    dry_count_for_water: i32,
    wet_product: Option<i32>,
    task: &mut FarmTaskState,
    // When true, nested `doPrepareRows()` (Haxe `doPlant`).
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
/// Haxe `countCurrentObject` / `countCurrentObjects` includes held.
// Haxe: AiBase.doPlantWheat L2496–2499; countCurrentObjectHelper L3448
pub fn wheat_stage_count(counts: &FarmCounts) -> i32 {
    counts.get_with_held(DRY_PLANTED_WHEAT)
        + counts.get_with_held(RIPE_WHEAT)
        + counts.get_with_held(WET_PLANTED_WHEAT)
        + counts.get_with_held(WHEAT_SPROUTS)
        + counts.get_with_held(UNRIPE_WHEAT)
}

/// Corn stages.
// Haxe: AiBase.doPlantCorn L2508–2511
pub fn corn_stage_count(counts: &FarmCounts) -> i32 {
    counts.get_with_held(DRY_PLANTED_CORN)
        + counts.get_with_held(WET_PLANTED_CORN)
        + counts.get_with_held(CORN_SPROUT)
        + counts.get_with_held(CORN_PLANT)
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

/// Haxe `doPlantPepper`.
// Haxe: AiBase.doPlantPepper L2490–2493
pub fn do_plant_pepper(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant_pepper_ex(min, max, counts, task, allow_prepare_rows)
}

fn do_plant_pepper_ex(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_PEPPER,
        plant_stage_count(
            counts,
            &[
                DRY_PLANTED_PEPPER,
                WET_PLANTED_PEPPER,
                PEPPER_PLANT,
                FRUITING_PEPPER,
            ],
        ),
        counts.get(DRY_PLANTED_PEPPER),
        Some(WET_PLANTED_PEPPER),
        task,
        allow_prepare_rows,
        counts,
    )
}

/// Haxe `doPlantBeans`.
// Haxe: AiBase.doPlantBeans L2502–2505
pub fn do_plant_beans(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_BEANS,
        plant_stage_count(
            counts,
            &[
                DRY_PLANTED_BEANS,
                WET_PLANTED_BEANS,
                GREEN_BEAN_PLANTS,
                DRY_BEAN_PLANTS,
            ],
        ),
        counts.get(DRY_PLANTED_BEANS),
        Some(WET_PLANTED_BEANS),
        task,
        allow_prepare_rows,
        counts,
    )
}

/// Haxe `doPlantTomato` — stages are dry seed + plant + fruiting (not wet 2831 / sprout 2832).
// Haxe: AiBase.doPlantTomato L2525–2528
pub fn do_plant_tomato(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_TOMATO,
        plant_stage_count(
            counts,
            &[DRY_PLANTED_TOMATO, TOMATO_PLANT, FRUITING_TOMATO],
        ),
        counts.get(DRY_PLANTED_TOMATO),
        default_wet_from_bowl(DRY_PLANTED_TOMATO),
        task,
        allow_prepare_rows,
        counts,
    )
}

/// Haxe `doPlantCucumber`.
// Haxe: AiBase.doPlantCucumber L2531–2534
pub fn do_plant_cucumber(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_CUCUMBER,
        plant_stage_count(
            counts,
            &[
                DRY_PLANTED_CUCUMBER,
                WET_PLANTED_CUCUMBER,
                CUCUMBER_SPROUT,
                RIPE_CUCUMBER,
            ],
        ),
        counts.get(DRY_PLANTED_CUCUMBER),
        Some(WET_PLANTED_CUCUMBER),
        task,
        allow_prepare_rows,
        counts,
    )
}

/// Haxe `doPlanSquash` (typo in Haxe name).
// Haxe: AiBase.doPlanSquash L2537–2545
pub fn do_plant_squash(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_SQUASH,
        plant_stage_count(
            counts,
            &[
                DRY_PLANTED_SQUASH,
                WET_PLANTED_SQUASH,
                HUBBARD_SQUASH,
                RIPE_SQUASH_PLANT,
                CROCK_WITH_SQUASH,
                PLATE_SQUASH_CHUNKS,
                PLATE_SQUASH_CHUNKS_SEEDS,
            ],
        ),
        counts.get(DRY_PLANTED_SQUASH),
        Some(WET_PLANTED_SQUASH),
        task,
        allow_prepare_rows,
        counts,
    )
}

/// Haxe `doPlantMilkWeed`.
// Haxe: AiBase.doPlantMilkWeed L2548–2555
pub fn do_plant_milkweed(
    min: i32,
    max: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    do_plant(
        min,
        max,
        DRY_PLANTED_MILKWEED,
        plant_stage_count(
            counts,
            &[
                DRY_PLANTED_MILKWEED,
                WET_PLANTED_MILKWEED,
                MILKWEED_SPROUT,
                MILKWEED,
                FLOWERING_MILKWEED,
                FRUITING_MILKWEED,
            ],
        ),
        counts.get(DRY_PLANTED_MILKWEED),
        Some(WET_PLANTED_MILKWEED),
        task,
        allow_prepare_rows,
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
    // Haxe countCurrentObject includes held; pile 3902 counts as 2 dried ears.
    // Haxe: AiBase.doHarvestCorn L2423–2426
    let dry_piles = counts.get_with_held(PILE_DRIED_CORN);
    let count_dry = counts.get_with_held(DRIED_CORN) + 2 * dry_piles;
    let count_ear = counts.get_with_held(EAR_OF_CORN);
    let count_shucked = counts.get_with_held(SHUCKED_CORN);
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
    // 0 + Corn Plant 1112; craftActorIfNeeded=false; maxNewActor=5 (ear 1113).
    // Ear < 4 (countCurrentObject) is stricter than maxNewActor 5 on the same snapshot.
    // Haxe: AiBase.doHarvestCorn L2438
    if task.harvest_corn < 2.0 && count_ear < 4 {
        if counts.get(CORN_PLANT) > 0 {
            return FarmAction::ShortCraft {
                actor: 0,
                target: CORN_PLANT,
            };
        }
    }
    task.harvest_corn = 2.0;
    // Sharp Stone + Ear of Corn. Haxe sets task=0 only if shortCraft returns false.
    // Haxe: AiBase.doHarvestCorn L2441–2443
    if count_ear > 0 {
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
    // Haxe countCurrentObjects includes held.
    // Haxe: AiBase.doHarvestWheat L2453–2461
    let threshed = counts.get_with_held(THRESHED_WHEAT)
        + counts.get_with_held(THRESHED_WHEAT_GROUND);
    let all_harvested = threshed
        + counts.get_with_held(HARVESTED_WHEAT)
        + counts.get_with_held(WHEAT_SHEAF);
    let planted_ripe = counts.get_with_held(RIPE_WHEAT);

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
    if planted_ripe > 0 && all_harvested < max_harvest {
        return FarmAction::CraftItem {
            object_id: HARVESTED_WHEAT,
        };
    }
    if counts.get_with_held(HARVESTED_WHEAT) > 0 {
        return FarmAction::CraftItem {
            object_id: WHEAT_SHEAF,
        };
    }
    if counts.get_with_held(WHEAT_SHEAF) > 0 {
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
    // Haxe: countCurrentObject includes held (2× pile 1101 + 1138 + 213)
    // Haxe: AiBase.doPrepareSoil L2003–2008
    2 * counts.get_with_held(FERTILE_SOIL_PILE)
        + counts.get_with_held(FERTILE_SOIL)
        + counts.get_with_held(DEEP_TILLED_ROW)
}

/// Haxe `doPrepareSoil` body after shortCrafts (hysteresis + craft 336).
// Haxe: AiBase.doPrepareSoil ~1991
pub fn do_prepare_soil(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> FarmAction {
    // Dung â†’ wet compost first.
    // Haxe: shortCraft(900, 625, distance, false) — GetItem shovel, do not craft it
    // Haxe: AiBase.doPrepareSoil L1998
    let has_shovel = counts.held_id == SHOVEL_OF_DUNG || counts.get(SHOVEL_OF_DUNG) > 0;
    // Offer dungâ†’compost when wet compost present or always attempt (execution no-ops if missing).
    // For pure tests we only emit when useful signals exist.
    if counts.get(WET_COMPOST) > 0 && has_shovel {
        return FarmAction::ShortCraft {
            actor: SHOVEL_OF_DUNG,
            target: WET_COMPOST,
        };
    }
    // Haxe: shortCraftOnGround(336) — held → drop near well; else GetItem(336)
    // Haxe: AiBase.doPrepareSoil L2001; shortCraftOnGround L2692–2711
    if counts.held_id == BASKET_OF_SOIL {
        return FarmAction::ShortCraft {
            actor: BASKET_OF_SOIL,
            target: 0,
        };
    }
    if counts.get(BASKET_OF_SOIL) > 0 {
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
    // Haxe: held Basket 292 → closest GetTransitionByNewActor(336) target
    // Haxe: AiBase.doPrepareSoil L2028–2046
    if counts.held_id == BASKET {
        if counts.get(FERTILE_SOIL) > 0 {
            return FarmAction::ShortCraft {
                actor: BASKET,
                target: FERTILE_SOIL,
            };
        }
        if counts.get(FERTILE_SOIL_PILE) > 0 {
            return FarmAction::ShortCraft {
                actor: BASKET,
                target: FERTILE_SOIL_PILE,
            };
        }
    }
    // Basket of Soil 336
    // Haxe: AiBase.doPrepareSoil L2051
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
    // Haxe: countCurrentObject(625) includes held; player-local CountClose is extra
    // when the player is away from home (same snapshot → once).
    // Haxe: AiBase.doComposting L2077–2078
    let stock_with_wet = stock + counts.get_with_held(WET_COMPOST);
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
/// Nested pottery uses default `doPrepareRows()` maxProfession=2.
// Haxe: AiBase.doPrepareRows ~2086 maxProfession = 2
pub fn do_prepare_rows(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    // When false, skip nested soil (avoid recursion from do_prepare_soil callers).
    allow_soil: bool,
) -> FarmAction {
    do_prepare_rows_ex(
        counts,
        task,
        has_profession,
        allow_soil,
        BASIC_FARM_DEFAULT_MAX_PROFESSION,
    )
}

/// Haxe `doPrepareRows(maxProfession)` — assigned ROWMAKER uses 100.
// Haxe: AiBase.doPrepareRows L2086; assigned L701 doPrepareRows(100)
pub fn do_prepare_rows_ex(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    allow_soil: bool,
    max_profession: i32,
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
    // Haxe: AiBase.doPrepareRows L2176–2205
    if task.row_maker < 3.0 {
        if deep < 10 {
            let has_steel_hoe =
                counts.held_id == STEEL_HOE || counts.get(STEEL_HOE) > 0;
            // Steel Hoe 857 + Shallow 1136, craftActorIfNeeded=false
            // Haxe: L2181
            if shallow > 0 {
                if has_steel_hoe {
                    return FarmAction::ShortCraft {
                        actor: STEEL_HOE,
                        target: SHALLOW_TILLED_ROW,
                    };
                }
                // Stone Hoe 850 + Shallow 1136 (may craft; search r=60)
                // Haxe: L2184–2187
                return FarmAction::ShortCraft {
                    actor: STONE_HOE,
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
            // Steel Hoe 857 + Fertile Soil 1138, craftActorIfNeeded=false
            // Haxe: L2198–2199
            // Stone Hoe 850 + Fertile Soil 1138 (may craft; default craftActorIfNeeded)
            // Haxe: AiBase.doPrepareRows L2200–2201
            if counts.get(FERTILE_SOIL) > 0 {
                if has_steel_hoe {
                    return FarmAction::ShortCraft {
                        actor: STEEL_HOE,
                        target: FERTILE_SOIL,
                    };
                }
                return FarmAction::ShortCraft {
                    actor: STONE_HOE,
                    target: FERTILE_SOIL,
                };
            }
        } else {
            task.row_maker = 3.0;
        }
    }

    // Haxe: countBowls includes held Clay Bowl 235
    // Haxe: AiBase.doPrepareRows L2167–2168
    let mut bowls = counts.get(CLAY_BOWL);
    if counts.held_id == CLAY_BOWL {
        bowls += 1;
    }
    if deep < 6 && bowls < 1 {
        // Haxe: if (countBowls < 1 && doPottery(maxProfession)) return true;
        // Haxe: AiBase.doPrepareRows L2208–2214
        return FarmAction::DeferPottery { max_profession };
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
    // Haxe: if (cleanUp()) return true
    // Haxe: AiBase.doCarrotFarming L1986
    FarmAction::DeferCleanup
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
// Haxe: AiBase.doPlantBushes L2294–2340
pub fn do_plant_bushes(counts: &FarmCounts, task: &mut FarmTaskState) -> FarmAction {
    // Dry Domestic Gooseberry Bush 393
    // Haxe: AiBase.doPlantBushes L2300
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
    // Dry Planted Gooseberry Seed 216
    // Haxe: AiBase.doPlantBushes L2302
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
    // 391+393+1135+1134+219+217+216+389
    // Haxe: AiBase.doPlantBushes L2307–2322
    let bushes = bush_stage_count(counts);
    // Haxe: profession['BASICFARMER'] < 7 ? 3 : 9
    // Haxe: AiBase.doPlantBushes L2326–2328
    if bushes >= max_bushes(counts.basic_farmer_weight) {
        return FarmAction::None;
    }
    // Wet/Dry Planted Gooseberry Seed — craft 216
    // Haxe: AiBase.doPlantBushes L2330–2332
    // L2335–2338 doWateringOn(393)/ (216) min=1 run only if craftItem(216) returns false.
    FarmAction::CraftItem {
        object_id: DRY_PLANTED_GOOSEBERRY,
    }
}

/// Haxe `doPlantPotatos` — requires shovel (`countCurrentObject(502)` includes held).
// Haxe: AiBase.doPlantPotatos L2514–2522
pub fn do_plant_potatos(
    min_planted: i32,
    max_planted: i32,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    allow_prepare_rows: bool,
) -> FarmAction {
    if counts.get_with_held(SHOVEL) < 1 {
        return FarmAction::None;
    }
    let stages = plant_stage_count(
        counts,
        &[
            DRY_PLANTED_POTATO,
            WET_PLANTED_POTATO,
            POTATO_PLANTS,
            MOUNDED_POTATO,
            MATURE_POTATO,
            DUG_POTATO,
        ],
    );
    do_plant(
        min_planted,
        max_planted,
        DRY_PLANTED_POTATO,
        stages,
        counts.get(DRY_PLANTED_POTATO),
        Some(WET_PLANTED_POTATO),
        task,
        allow_prepare_rows,
        counts,
    )
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
        DRY_PLANTED_SQUASH | WET_PLANTED_SQUASH => {
            // Haxe: doPlanSquash commented out — skip
            // Haxe: AiBase.doAdvancedFarming L4018–4022
            FarmAction::None
        }
        DRY_PLANTED_GARLIC | WET_PLANTED_GARLIC => advanced_garlic_craft(plant_id, counts),
        // Haxe `if (toPlant == 2851)` CountClose 2854+2851 > 6 skip; 2852 has no gate.
        // Haxe: AiBase.doAdvancedFarming L4032–4044
        DRY_PLANTED_ONIONS => advanced_dry_onion_craft(counts),
        other => FarmAction::CraftItem { object_id: other },
    }
}

/// Default `maxProfession` for Haxe `doBasicFarming()` / `doBasicFarming(2)`.
// Haxe: AiBase.doBasicFarming maxProfession = 2
pub const BASIC_FARM_DEFAULT_MAX_PROFESSION: i32 = 2;
/// Assigned BASICFARMER job: Haxe `doBasicFarming(100)`.
// Haxe: AiBase.doTimeStuffHelper ~710
pub const BASIC_FARM_ASSIGNED_MAX_PROFESSION: i32 = 100;
/// Haxe `doAdvancedFarming` default maxPeople.
// Haxe: AiBase.doAdvancedFarming L3909
pub const ADVANCED_FARM_DEFAULT_MAX_PEOPLE: i32 = 2;
/// Haxe `return doPottery(3)` when no clay bowls near home r=15.
// Haxe: AiBase.doAdvancedFarming L3929
pub const ADVANCED_FARM_POTTERY_MAX_PEOPLE: i32 = 3;
/// Haxe CountCloseObjects clay bowl home r=15 in doAdvancedFarming.
// Haxe: AiBase.doAdvancedFarming L3928
pub const ADVANCED_BOWL_COUNT_RADIUS: i32 = 15;
/// Haxe garlic CountClose home r=30 (4262 + 4265).
// Haxe: AiBase.doAdvancedFarming L4001–4003
pub const ADVANCED_GARLIC_COUNT_RADIUS: i32 = 30;
/// Haxe skip garlic when CountClose 4262+4265 > 2.
// Haxe: AiBase.doAdvancedFarming L4005
pub const ADVANCED_GARLIC_SKIP_ABOVE: i32 = 2;
/// Haxe onion CountClose home r=30 (2854 + 2851).
// Haxe: AiBase.doAdvancedFarming L4035–4037
pub const ADVANCED_ONION_COUNT_RADIUS: i32 = 30;
/// Haxe skip dry onions when CountClose 2854+2851 > 6.
// Haxe: AiBase.doAdvancedFarming L4038
pub const ADVANCED_ONION_SKIP_ABOVE: i32 = 6;
/// Haxe `makeSharpieFood(maxDistance = 40)`.
// Haxe: AiBase.makeSharpieFood L4096
pub const MAKE_SHARPIE_FOOD_DEFAULT_MAX_DISTANCE: i32 = 40;
/// Haxe `maxDistance <= 10` uses `GetClosestObjectById` from the player (half-open square).
// Haxe: AiBase.makeSharpieFood L4108 / L4113
pub const MAKE_SHARPIE_FOOD_CLOSE_SEARCH_MAX: i32 = 10;
/// Haxe `makeSharpieFood(5)` hungry / doStuff close call.
// Haxe: AiBase.doTimeStuffHelper L656; isConsideringMakingFood L8541
pub const MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE: i32 = 5;
/// Haxe `if (quadDistance < 900) return false` after meh/superMeh scaling.
// Haxe: AiBase.isConsideringMakingFood L8497
pub const CONSIDER_MAKE_FOOD_NEAR_QUAD: f32 = 900.0;
/// Haxe `if (myPlayer.food_store < -1) return false` when `foodTarget != null`.
// Haxe: AiBase.isConsideringMakingFood L8490
pub const CONSIDER_MAKE_FOOD_STARVING_STORE: f32 = -1.0;
/// Haxe `isMeh` / `isSuperMeh` each multiply food quad by 4.
// Haxe: AiBase.isConsideringMakingFood L8493–8494
pub const CONSIDER_MAKE_FOOD_MEH_QUAD_MUL: f32 = 4.0;

/// Scaled food-target quad used by `isConsideringMakingFood` (meh ×4, superMeh ×4).
// Haxe: AiBase.isConsideringMakingFood L8491–8494
pub fn consider_making_food_scaled_food_quad(
    raw_quad: f32,
    is_meh: bool,
    is_super_meh: bool,
) -> f32 {
    let mut q = raw_quad;
    if is_meh {
        q *= CONSIDER_MAKE_FOOD_MEH_QUAD_MUL;
    }
    if is_super_meh {
        q *= CONSIDER_MAKE_FOOD_MEH_QUAD_MUL;
    }
    q
}

/// After age/hungry enter + SMITH wipe: skip making food (starving with a target, or near).
///
/// `food_target` is `(raw_quad, isMeh, isSuperMeh)`. `None` means no `foodTarget`
/// (hungry path continues). Home-distance L8501 is the next hop.
// Haxe: AiBase.isConsideringMakingFood L8487–8497
pub fn consider_making_food_skip_after_enter(
    food_target: Option<(f32, bool, bool)>,
    food_store: f32,
) -> bool {
    let Some((raw_quad, is_meh, is_super_meh)) = food_target else {
        return false;
    };
    if food_store < CONSIDER_MAKE_FOOD_STARVING_STORE {
        return true;
    }
    consider_making_food_scaled_food_quad(raw_quad, is_meh, is_super_meh)
        < CONSIDER_MAKE_FOOD_NEAR_QUAD
}

/// Haxe `quadDistance = -1` when `foodTarget == null`.
// Haxe: AiBase.isConsideringMakingFood L8487
pub const CONSIDER_MAKE_FOOD_NO_TARGET_QUAD: f32 = -1.0;
/// Haxe `lastCheckedTimes['considerFood']` refresh (`passedTime > 15`).
// Haxe: AiBase.isConsideringMakingFood L8509–8510
pub const CONSIDER_FOOD_RECHECK_SEC: f32 = 15.0;
/// Haxe `deadlyPlayer == null ? 10000`.
// Haxe: AiBase.isConsideringMakingFood L8527
pub const CONSIDER_MAKE_FOOD_NO_THREAT_DIST: f32 = 10000.0;
/// Haxe `if (dist > 9000)` swap in animal quad.
// Haxe: AiBase.isConsideringMakingFood L8528
pub const CONSIDER_MAKE_FOOD_USE_ANIMAL_DIST: f32 = 9000.0;
/// Haxe `if (dist < 100) doStuff = true`.
// Haxe: AiBase.isConsideringMakingFood L8531
pub const CONSIDER_MAKE_FOOD_DO_STUFF_FIGHT_DIST: f32 = 100.0;
/// Haxe `if (dist > 400) doStuff = false`.
// Haxe: AiBase.isConsideringMakingFood L8532
pub const CONSIDER_MAKE_FOOD_DO_STUFF_FAR_DIST: f32 = 400.0;
/// Haxe `if (quadDistanceToHome > 900) doStuff = false`.
// Haxe: AiBase.isConsideringMakingFood L8533
pub const CONSIDER_MAKE_FOOD_DO_STUFF_HOME_QUAD: f32 = 900.0;
/// Haxe `heat < 0.1 || heat > 0.9`.
// Haxe: AiBase.isConsideringMakingFood L8526
pub const CONSIDER_MAKE_FOOD_HEAT_LOW: f32 = 0.1;
pub const CONSIDER_MAKE_FOOD_HEAT_HIGH: f32 = 0.9;
/// Haxe `makeSharpieFood(20)` after popcorn / shucked corn.
// Haxe: AiBase.isConsideringMakingFood L8583
pub const MAKE_SHARPIE_FOOD_FAR_CALL_DISTANCE: i32 = 20;
/// Turkey Slice on Plate — `craftItemMax(2190)`.
// Haxe: AiBase.isConsideringMakingFood L8546
pub const TURKEY_SLICE_ON_PLATE: i32 = 2190;
/// CountClose home r=30 for corn family.
// Haxe: AiBase.isConsideringMakingFood L8565–8568
pub const CONSIDER_FOOD_CORN_COUNT_RADIUS: i32 = 30;
/// Skinned Rabbit 181.
// Haxe: AiBase.isConsideringMakingFood L8588
pub const SKINNED_RABBIT: i32 = 181;
/// Skewered Rabbit 185.
// Haxe: AiBase.isConsideringMakingFood L8591
pub const SKEWERED_RABBIT: i32 = 185;
/// CountClose home r=25 for raw rabbit.
// Haxe: AiBase.isConsideringMakingFood L8588
pub const CONSIDER_FOOD_RABBIT_COUNT_RADIUS: i32 = 25;
/// Three Sisters Stew 1249.
// Haxe: AiBase.isConsideringMakingFood L8552
pub const THREE_SISTERS_STEW: i32 = 1249;
/// Partial Bucket of Skim Milk 1483.
// Haxe: AiBase.isConsideringMakingFood L8555
pub const PARTIAL_SKIM_MILK: i32 = 1483;
/// Full Bucket of Skim Milk 2124.
// Haxe: AiBase.isConsideringMakingFood L8558
pub const FULL_SKIM_MILK: i32 = 2124;
/// Open Fermented Sauerkraut 1241.
// Haxe: AiBase.isConsideringMakingFood L8561
pub const OPEN_SAUERKRAUT: i32 = 1241;

/// Scaled food quad, or `-1` when there is no `foodTarget`.
// Haxe: AiBase.isConsideringMakingFood L8487 / L8491–8494
pub fn consider_making_food_food_quad(food_target: Option<(f32, bool, bool)>) -> f32 {
    food_target
        .map(|(raw, meh, super_meh)| {
            consider_making_food_scaled_food_quad(raw, meh, super_meh)
        })
        .unwrap_or(CONSIDER_MAKE_FOOD_NO_TARGET_QUAD)
}

/// Haxe `if (quadDistanceToHome > quadDistance) return false`.
// Haxe: AiBase.isConsideringMakingFood L8504
pub fn consider_making_food_skip_too_far_from_home(home_quad: f32, food_quad: f32) -> bool {
    home_quad > food_quad
}

/// L8487–8504: starving / near food / farther from home than from food.
// Haxe: AiBase.isConsideringMakingFood L8487–8504
pub fn consider_making_food_skip_after_enter_with_home(
    food_target: Option<(f32, bool, bool)>,
    food_store: f32,
    home_quad: f32,
) -> bool {
    if consider_making_food_skip_after_enter(food_target, food_store) {
        return true;
    }
    consider_making_food_skip_too_far_from_home(
        home_quad,
        consider_making_food_food_quad(food_target),
    )
}

/// Haxe `passedTime > 15` → `searchFoodAndEat`.
// Haxe: AiBase.isConsideringMakingFood L8509–8510
pub fn consider_making_food_should_research(passed_sec: f32) -> bool {
    passed_sec > CONSIDER_FOOD_RECHECK_SEC
}

/// Haxe consider-food `doStuff` (heat + deadly dist + home).
///
/// `deadly_player_quad` / `deadly_animal_quad` are Haxe `CalculateDistanceToPlayer`
/// / `CalculateQuadDistanceToObject` (both squared). `None` → 10000.
// Haxe: AiBase.isConsideringMakingFood L8526–8533
pub fn consider_making_food_do_stuff(
    heat: f32,
    deadly_player_quad: Option<f32>,
    deadly_animal_quad: Option<f32>,
    home_quad: f32,
) -> bool {
    let mut dist = deadly_player_quad.unwrap_or(CONSIDER_MAKE_FOOD_NO_THREAT_DIST);
    if dist > CONSIDER_MAKE_FOOD_USE_ANIMAL_DIST {
        dist = deadly_animal_quad.unwrap_or(CONSIDER_MAKE_FOOD_NO_THREAT_DIST);
    }
    let superbad_temp =
        heat < CONSIDER_MAKE_FOOD_HEAT_LOW || heat > CONSIDER_MAKE_FOOD_HEAT_HIGH;
    let mut do_stuff = !superbad_temp;
    if dist < CONSIDER_MAKE_FOOD_DO_STUFF_FIGHT_DIST {
        do_stuff = true;
    }
    if dist > CONSIDER_MAKE_FOOD_DO_STUFF_FAR_DIST {
        do_stuff = false;
    }
    if home_quad > CONSIDER_MAKE_FOOD_DO_STUFF_HOME_QUAD {
        do_stuff = false;
    }
    do_stuff
}

/// Haxe `countDryCorn += 2 * countCurrentObject(3902)` plus CountClose 1115.
// Haxe: AiBase.isConsideringMakingFood L8565–8566
pub fn consider_making_food_count_dry_corn(count_dried_ear: i32, count_pile: i32) -> i32 {
    count_dried_ear + 2 * count_pile
}

/// Yum-1114 ear-of-corn hysteresis (`EearOfCornMaker`).
// Haxe: AiBase.isConsideringMakingFood L8563–8577
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarOfCornMakerPlan {
    /// `shortCraft(0, 1112, 30, false, 4)` while the maker flag is on.
    pub pick_ear: bool,
    /// `countCorn > 0 && countShuckedCorn < 2` → `craftItem(1114)`.
    pub shuck: bool,
}

pub fn consider_making_food_ear_of_corn_maker(
    is_yum_shucked: bool,
    count_dry_corn: i32,
    count_corn: i32,
    count_shucked: i32,
    maker_flag: &mut f32,
) -> EarOfCornMakerPlan {
    if !is_yum_shucked {
        return EarOfCornMakerPlan {
            pick_ear: false,
            shuck: false,
        };
    }
    if count_corn < 1 {
        *maker_flag = 1.0;
    }
    if count_corn > 1 || count_dry_corn > 5 {
        *maker_flag = 0.0;
    }
    EarOfCornMakerPlan {
        pick_ear: *maker_flag > 0.0,
        shuck: count_corn > 0 && count_shucked < 2,
    }
}

/// Held bumps CountClose 181 / 185.
// Haxe: AiBase.isConsideringMakingFood L8588–8592
pub fn consider_making_food_raw_rabbit_count(
    count_skinned: i32,
    held_skinned: bool,
    count_skewered: i32,
    held_skewered: bool,
) -> i32 {
    count_skinned
        + i32::from(held_skinned)
        + count_skewered
        + i32::from(held_skewered)
}

/// Haxe `if (countRawRabbit > 1 && makeFireFood(1))`.
// Haxe: AiBase.isConsideringMakingFood L8594
pub fn consider_making_food_fire_food_on_extra_rabbit(raw_rabbit: i32) -> bool {
    raw_rabbit > 1
}

/// Haxe `if (countRawRabbit <= 1 && makeFireFood(1))` after baking/watering.
// Haxe: AiBase.isConsideringMakingFood L8603
pub fn consider_making_food_fire_food_on_few_rabbit(raw_rabbit: i32) -> bool {
    raw_rabbit <= 1
}

/// Hungry consider-food shortCraft table (actor, target, dist, maxNewActor).
/// `craftActorIfNeeded` default true except corn-plant (false).
// Haxe: AiBase.isConsideringMakingFood L8548–8561
pub fn consider_making_food_short_crafts() -> &'static [(i32, i32, i32, i32)] {
    &[
        (0, CARROT_ROW, 10, -1),
        (CLAY_BOWL, THREE_SISTERS_STEW, 20, 1),
        (CLAY_BOWL, PARTIAL_SKIM_MILK, 30, 1),
        (CLAY_BOWL, FULL_SKIM_MILK, 30, 1),
        (CLAY_BOWL, OPEN_SAUERKRAUT, 20, 1),
    ]
}
/// Haxe `doWatering(3)` mid basic farming (before wheat 6/12 + sheep).
// Haxe: AiBase.doBasicFarming ~2395
pub const BASIC_FARM_MID_WATER_MAX_PEOPLE: i32 = 3;

/// Haxe `doBasicFarming` sequence (first applicable action).
///
/// `max_profession` is the Haxe `maxProfession` peer-cap for `hasOrBecomeProfession`
/// and late `doAdvancedFarming(maxProfession)` (carried on [`FarmAction::DeferSheepHerding`]).
///
/// Mid `doWatering(3)` uses [`do_watering_helper`] only (no WaterBringer peer-cap).
/// Prefer [`do_basic_farming_ex`] on live scan so peer-cap applies.
// Haxe: AiBase.doBasicFarming ~2343
pub fn do_basic_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    max_profession: i32,
) -> FarmAction {
    do_basic_farming_ex(counts, task, has_profession, max_profession, None)
}

/// Like [`do_basic_farming`], with optional WaterBringer peer context for mid `doWatering(3)`.
///
/// When `watering` is `Some((rt, peer_count, was_idle))`, mid watering uses
/// [`do_watering`] (`hasOrBecomeProfession('WATERBRINGER', 3)`). When `None`,
/// mid watering uses [`do_watering_helper`] only (pure probes / legacy callers).
// Haxe: AiBase.doBasicFarming ~2395 doWatering(3)
pub fn do_basic_farming_ex(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
    max_profession: i32,
    watering: Option<(&mut FarmProfessionRuntime, f32, f32)>,
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
    // Haxe doPlant* → doPlant → doPrepareRows() (default maxProfession=2)
    // Haxe: AiBase.doPlant L2578; doPlantCorn L2376 / doPlantWheat L2380
    let p = do_plant(
        1,
        3,
        DRY_PLANTED_CORN,
        corn_stage_count(counts),
        counts.get(DRY_PLANTED_CORN),
        Some(WET_PLANTED_CORN),
        task,
        true,
        counts,
    );
    if p.is_some() {
        return p;
    }
    let p = do_plant(
        2,
        5,
        DRY_PLANTED_WHEAT,
        wheat_stage_count(counts),
        counts.get(DRY_PLANTED_WHEAT),
        Some(WET_PLANTED_WHEAT),
        task,
        true,
        counts,
    );
    if p.is_some() {
        return p;
    }
    // Haxe: doPlantTomato(2,5); doPlantBeans(2,4); doPlantCucumber(2,4); doPlantPepper(2,5)
    // Haxe: AiBase.doBasicFarming L2382–2388
    let p = do_plant_tomato(2, 5, counts, task, true);
    if p.is_some() {
        return p;
    }
    let p = do_plant_beans(2, 4, counts, task, true);
    if p.is_some() {
        return p;
    }
    let p = do_plant_cucumber(2, 4, counts, task, true);
    if p.is_some() {
        return p;
    }
    let p = do_plant_pepper_ex(2, 5, counts, task, true);
    if p.is_some() {
        return p;
    }
    // Haxe L2390–2392: 1137+1143 / 502+1146 already at head; doPlantPotatos(2,5)
    // Haxe: AiBase.doPlantPotatos L2514 countCurrentObject(502) includes held
    let p = do_plant_potatos(2, 5, counts, task, true);
    if p.is_some() {
        return p;
    }
    let c = do_composting(counts, task);
    if c.is_some() {
        return c;
    }
    // Haxe: if (doWatering(3)) return true;
    // Haxe: AiBase.doBasicFarming ~2395
    let w = match watering {
        Some((rt, peer, idle)) => do_watering(
            rt,
            counts,
            task,
            BASIC_FARM_MID_WATER_MAX_PEOPLE,
            peer,
            idle,
        ),
        None => do_watering_helper(counts, task),
    };
    if w.is_some() {
        return w;
    }
    // Haxe: doPlantWheat(6, 12); doPlantCorn(4, 8)
    // Haxe: AiBase.doBasicFarming L2397–2398
    let p = do_plant(
        6,
        12,
        DRY_PLANTED_WHEAT,
        wheat_stage_count(counts),
        counts.get(DRY_PLANTED_WHEAT),
        Some(WET_PLANTED_WHEAT),
        task,
        true,
        counts,
    );
    if p.is_some() {
        return p;
    }
    let p = do_plant(
        4,
        8,
        DRY_PLANTED_CORN,
        corn_stage_count(counts),
        counts.get(DRY_PLANTED_CORN),
        Some(WET_PLANTED_CORN),
        task,
        true,
        counts,
    );
    if p.is_some() {
        return p;
    }
    // Haxe: this.profession['BASICFARMER'] = 1; isSheepHerding(1);
    // then late plants → doAdvancedFarming(maxProfession).
    // AI-SHEPHERD-MID mid call site; max_profession carried for advanced expand.
    // Haxe: AiBase.doBasicFarming ~2400–2413
    FarmAction::DeferSheepHerding { max_profession }
}

/// Haxe `maxDistance <= 10` → `AiHelper.GetClosestObjectById` from player.
// Haxe: AiBase.makeSharpieFood L4108
pub fn make_sharpie_food_uses_player_search(max_distance: i32) -> bool {
    max_distance <= MAKE_SHARPIE_FOOD_CLOSE_SEARCH_MAX
}

/// Plant in range for `makeSharpieFood(maxDistance)`.
///
/// - `maxDistance <= 10`: half-open square from player (`GetClosestObjectById`).
/// - else: player quad `dx²+dy² <= maxDistance²` (`getClosestObjectById` cache gate).
// Haxe: AiBase.makeSharpieFood L4108–4114; getClosestObjectById L3478–3479
pub fn make_sharpie_food_plant_in_range(
    max_distance: i32,
    player_x: i32,
    player_y: i32,
    plant_x: i32,
    plant_y: i32,
) -> bool {
    if make_sharpie_food_uses_player_search(max_distance) {
        in_count_close_square(player_x, player_y, plant_x, plant_y, max_distance)
    } else {
        let dx = plant_x - player_x;
        let dy = plant_y - player_y;
        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
            <= max_distance.saturating_mul(max_distance)
    }
}

/// Fill wild-carrot / burdock counts using [`make_sharpie_food_plant_in_range`].
// Haxe: AiBase.makeSharpieFood L4108–4114
pub fn fill_sharpie_counts_from_xy(
    player_x: i32,
    player_y: i32,
    held_id: i32,
    objects: impl IntoIterator<Item = (i32, i32, i32)>,
    max_distance: i32,
) -> FarmCounts {
    let mut c = FarmCounts {
        held_id,
        ..Default::default()
    };
    let mut n_carrot = 0;
    let mut n_burdock = 0;
    for (id, x, y) in objects {
        if !make_sharpie_food_plant_in_range(max_distance, player_x, player_y, x, y) {
            continue;
        }
        if id == SEEDING_WILD_CARROT {
            n_carrot += 1;
        } else if id == BURDOCK {
            n_burdock += 1;
        }
    }
    c.set(SEEDING_WILD_CARROT, n_carrot);
    c.set(BURDOCK, n_burdock);
    c
}

/// `makeSharpieFood(maxDistance)` from player-relative map objects.
// Haxe: AiBase.makeSharpieFood L4108–4118
pub fn make_sharpie_food_from_xy(
    player_x: i32,
    player_y: i32,
    held_id: i32,
    objects: impl IntoIterator<Item = (i32, i32, i32)>,
    max_distance: i32,
) -> FarmAction {
    let c = fill_sharpie_counts_from_xy(player_x, player_y, held_id, objects, max_distance);
    make_sharpie_food(&c)
}

/// Pure Haxe `makeSharpieFood` (wild carrot / burdock + sharp stone).
///
/// Returns `CraftItem` for GetOrCraft sharp stone or dug product when source plant present.
/// Callers must fill counts with plants already in the Haxe search (see
/// [`make_sharpie_food_from_xy`]). Default search is
/// [`MAKE_SHARPIE_FOOD_DEFAULT_MAX_DISTANCE`] (Haxe `maxDistance = 40`).
// Haxe: AiBase.makeSharpieFood L4096–4118
pub fn make_sharpie_food(counts: &FarmCounts) -> FarmAction {
    // Haxe L4101: isHoldingSharpStone = held == 34 Sharp Stone
    let holding_sharp = counts.held_id == SHARP_STONE;
    // Seeding Wild Carrot 36 → sharp stone 34 / Dug Wild Carrot 39
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
    // Burdock 804 → sharp stone / Dug Burdock 806
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

/// Clay bowls for the advanced-farming pottery gate (CountClose home r=15).
// Haxe: AiBase.doAdvancedFarming L3928
pub fn advanced_farming_bowl_count(counts: &FarmCounts) -> i32 {
    counts
        .bowl_count_home_15
        .unwrap_or_else(|| counts.get(CLAY_BOWL))
}

/// Haxe `doAdvancedFarming` body after `hasOrBecomeProfession` succeeded.
// Haxe: AiBase.doAdvancedFarming L3909–4072
pub fn do_advanced_farming(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    age: f32,
    has_profession: bool,
) -> FarmAction {
    do_advanced_farming_ex(
        counts,
        task,
        age,
        has_profession,
        ADVANCED_FARM_DEFAULT_MAX_PEOPLE,
        0,
    )
}

/// Like [`do_advanced_farming`] with `maxPeople` and rotation RNG.
// Haxe: doAdvancedFarming(maxPeople); toPlant = toPlant > 0 ? toPlant : randomInt(len-1)
pub fn do_advanced_farming_ex(
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    age: f32,
    has_profession: bool,
    max_people: i32,
    rng_plant: i32,
) -> FarmAction {
    if !has_profession {
        return FarmAction::None;
    }
    // Haxe: if (doPrepareRows(maxPeople)) return true;
    let rows = do_prepare_rows_ex(counts, task, true, true, max_people);
    if rows.is_some() {
        return rows;
    }
    // Haxe L3917–3918 shortCraft(502, 1146, 30); shortCraft(1137, 1143, 30)
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
    let bowls = advanced_farming_bowl_count(counts);
    if bowls < 1 {
        return FarmAction::DeferPottery {
            max_profession: ADVANCED_FARM_POTTERY_MAX_PEOPLE,
        };
    }
    if task.to_plant <= 0 {
        task.to_plant = rng_plant;
    }
    let start = if task.to_plant > 0 {
        task.to_plant as usize
    } else {
        0
    };
    for i in 0..ADVANCED_PLANTS.len() {
        let id = advanced_plant_at(start, age, i);
        let a = match id {
            DRY_PLANTED_BEANS | WET_PLANTED_BEANS => {
                do_plant_beans(2, 4, counts, task, true)
            }
            DRY_PLANTED_POTATO | WET_PLANTED_POTATO => {
                do_plant_potatos(2, 8, counts, task, true)
            }
            DRY_PLANTED_CUCUMBER => do_plant_cucumber(2, 8, counts, task, true),
            DRY_PLANTED_PEPPER => do_plant_pepper(2, 5, counts, task, true),
            DRY_PLANTED_TOMATO => do_plant_tomato(1, 8, counts, task, true),
            // Haxe: doPlanSquash commented — skip
            // Haxe: AiBase.doAdvancedFarming L4018–4023
            DRY_PLANTED_SQUASH | WET_PLANTED_SQUASH => FarmAction::None,
            DRY_PLANTED_GARLIC | WET_PLANTED_GARLIC => advanced_garlic_craft(id, counts),
            DRY_PLANTED_ONIONS => advanced_dry_onion_craft(counts),
            // Haxe `if (craftItem(toPlant))` including Wet Planted Onions 2852
            // Haxe: AiBase.doAdvancedFarming L4044
            _other => FarmAction::CraftItem { object_id: id },
        };
        if a.is_some() {
            return a;
        }
    }
    // Haxe: this.profession['ADVANCEDFARMER'] = 0; return false
    // Haxe: AiBase.doAdvancedFarming L4070–4071
    FarmAction::ClearAdvancedFarmerWeight
}

/// Haxe CountClose 4262+4265 home r=30; count > 2 skip else craftItem(toPlant).
// Haxe: AiBase.doAdvancedFarming L3998–4008
fn advanced_garlic_craft(plant_id: i32, counts: &FarmCounts) -> FarmAction {
    let count = counts.get(DRY_PLANTED_GARLIC) + counts.get(MATURE_GARLIC);
    if count > ADVANCED_GARLIC_SKIP_ABOVE {
        FarmAction::None
    } else {
        FarmAction::CraftItem {
            object_id: plant_id,
        }
    }
}

/// Haxe CountClose 2854+2851 home r=30; count > 6 skip else craftItem(2851).
// Haxe: AiBase.doAdvancedFarming L4032–4044
fn advanced_dry_onion_craft(counts: &FarmCounts) -> FarmAction {
    let count = counts.get(RIPE_ONIONS) + counts.get(DRY_PLANTED_ONIONS);
    if count > ADVANCED_ONION_SKIP_ABOVE {
        FarmAction::None
    } else {
        FarmAction::CraftItem {
            object_id: DRY_PLANTED_ONIONS,
        }
    }
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
    // Haxe doPlantWheat/Corn → doPlant → doPrepareRows()
    // Haxe: AiBase.doBasicFarming L2408–2409; doPlant L2578
    let p = do_plant(
        15,
        30,
        DRY_PLANTED_WHEAT,
        wheat_stage_count(counts),
        counts.get(DRY_PLANTED_WHEAT),
        Some(WET_PLANTED_WHEAT),
        task,
        true,
        counts,
    );
    if p.is_some() {
        return p;
    }
    let p = do_plant(
        8,
        12,
        DRY_PLANTED_CORN,
        corn_stage_count(counts),
        counts.get(DRY_PLANTED_CORN),
        Some(WET_PLANTED_CORN),
        task,
        true,
        counts,
    );
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
    // Haxe doAdvancedFarming zeros ADVANCEDFARMER then returns false; caller zeros BASICFARMER.
    if matches!(adv, FarmAction::ClearAdvancedFarmerWeight) {
        return adv;
    }
    FarmAction::ClearBasicFarmerWeight
}

/// Dispatch one farm job step for AssignedJob / age-rotated / mid-prio.
///
/// `max_profession` applies to BasicFarmer (`doBasicFarming(max)`) and RowMaker
/// nested `doPottery(maxProfession)` (assigned ROWMAKER uses 100).
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
        FarmProfession::RowMaker => {
            do_prepare_rows_ex(counts, task, has_profession, true, max_profession)
        }
        FarmProfession::AdvancedFarmer => do_advanced_farming_ex(
            counts,
            task,
            20.0,
            has_profession,
            max_profession,
            0,
        ),
        FarmProfession::WaterBringer => {
            // List-order fallback (mid farm). Assigned live uses closest helper.
            // Haxe: assigned WATERBRINGER → doWatering(100) GetClosest r=30
            do_watering_helper(counts, task)
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
    use ol_ai_helper::ai_goals::priority_ladder::{
        goal_from_rung, PriorityRung,
    };
    use ol_ai_helper::ai_goals::Profession;

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
        // Haxe L2601–2609 GetTransition(382, dry).newTargetID
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_CARROTS), Some(WET_PLANTED_CARROTS));
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_CORN), Some(WET_PLANTED_CORN));
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_BEANS), Some(WET_PLANTED_BEANS));
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_TOMATO), Some(2831));
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_SQUASH), Some(WET_PLANTED_SQUASH));
        assert_eq!(default_wet_from_bowl(DRY_PLANTED_MILKWEED), Some(WET_PLANTED_MILKWEED));
        assert_eq!(default_wet_from_bowl(99999), None);
        let mut task3 = FarmTaskState::default();
        assert_eq!(
            do_watering_on(DRY_PLANTED_PEPPER, 1, 1, None, &mut task3),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_PEPPER
            }
        );
        // Haxe L2692–2711 shortCraftOnGround: held → empty ground; else GetItem
        assert_eq!(
            short_craft_on_ground(BASKET_OF_SOIL, BASKET_OF_SOIL),
            FarmAction::ShortCraft {
                actor: BASKET_OF_SOIL,
                target: 0
            }
        );
        assert_eq!(
            short_craft_on_ground(0, BASKET_OF_SOIL),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL
            }
        );
        // Haxe L2769–2773: actor 0 + not holding 0 → dropHeldObject
        let drop = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(CARROT, 0, CARROT_ROW)
        });
        assert_eq!(drop, ShortCraftApply::DropHeld);
        // Haxe L2775 GetOrCraftItem(actor, craftActorIfNeeded, target)
        let seek = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            craft_actor_if_needed: false,
            ..ShortCraftInput::basic(0, STONE_HOE, FERTILE_SOIL)
        });
        assert_eq!(
            seek,
            ShortCraftApply::SeekOrCraftActor {
                actor: STONE_HOE,
                craft_if_needed: false,
            }
        );
    }

    #[test]
    fn init_watering_target_ids_from_pouch_edges() {
        // Haxe L4038–4054: actor 210, skip TIME/ignored newTarget, dedupe, 396 not in without-carrots
        let edges = [
            (FULL_WATER_POUCH, DRY_PLANTED_CARROTS, DRY_PLANTED_CARROTS),
            (FULL_WATER_POUCH, DRY_PLANTED_WHEAT, WET_PLANTED_WHEAT),
            (FULL_WATER_POUCH, -1, DRY_PLANTED_WHEAT), // TIME
            (FULL_WATER_POUCH, 382, BOWL_OF_WATER),    // ignored new
            (99, DRY_PLANTED_BEANS, WET_PLANTED_BEANS), // wrong actor
            (FULL_WATER_POUCH, DRY_PLANTED_WHEAT, WET_PLANTED_WHEAT), // dedupe
            (FULL_WATER_POUCH, DRY_PLANTED_BEANS, WET_PLANTED_BEANS),
        ];
        let (all, no_carrot) = init_watering_target_ids(edges);
        assert_eq!(all, vec![DRY_PLANTED_CARROTS, DRY_PLANTED_WHEAT, DRY_PLANTED_BEANS]);
        assert_eq!(no_carrot, vec![DRY_PLANTED_WHEAT, DRY_PLANTED_BEANS]);
        assert_eq!(WATERING_SEARCH_DIST, 30);
    }

    #[test]
    fn do_watering_helper_prefers_dry_carrots_then_skips_when_stock_high() {
        let mut task = FarmTaskState::default();
        let dry = counts_with(&[(DRY_PLANTED_CARROTS, 2), (DRY_PLANTED_WHEAT, 2)]);
        assert_eq!(
            do_watering_helper(&dry, &mut task),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_CARROTS
            }
        );
        let mut task2 = FarmTaskState::default();
        let high_carrot = counts_with(&[
            (CARROT, 20),
            (DRY_PLANTED_CARROTS, 5),
            (DRY_PLANTED_WHEAT, 2),
        ]);
        assert_eq!(
            do_watering_helper(&high_carrot, &mut task2),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
    }

    #[test]
    fn do_watering_respects_waterbringer_peer_cap() {
        let mut rt = FarmProfessionRuntime::default();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 2)]);
        assert_eq!(
            do_watering(&mut rt, &counts, &mut task, 3, 3.0, 0.0),
            FarmAction::None
        );
        assert_ne!(rt.last_profession, Some(FarmProfession::WaterBringer));
        let mut rt2 = FarmProfessionRuntime::default();
        let mut task2 = FarmTaskState::default();
        assert_eq!(
            do_watering(&mut rt2, &counts, &mut task2, 3, 0.0, 0.0),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_eq!(rt2.last_profession, Some(FarmProfession::WaterBringer));
    }

    #[test]
    fn closest_watering_parent_id_prefers_near_wheat_over_far_carrots() {
        let objs = [
            FarmMapObj::simple(DRY_PLANTED_CARROTS, 10, 0),
            FarmMapObj::simple(DRY_PLANTED_WHEAT, 1, 0),
        ];
        assert_eq!(
            closest_watering_parent_id(0, 0, &objs, WATERING_TARGET_DRY_IDS, WATERING_SEARCH_DIST),
            Some(DRY_PLANTED_WHEAT)
        );
        assert_eq!(
            closest_watering_parent_id(0, 0, &objs, WATERING_TARGET_DRY_IDS, 0),
            None
        );
    }

    #[test]
    fn do_watering_helper_closest_waters_near_wheat_not_list_order_carrots() {
        let mut rt = FarmProfessionRuntime::default();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(DRY_PLANTED_CARROTS, 2), (DRY_PLANTED_WHEAT, 2)]);
        let objs = [
            FarmMapObj::simple(DRY_PLANTED_CARROTS, 10, 0),
            FarmMapObj::simple(DRY_PLANTED_WHEAT, 1, 0),
        ];
        assert_eq!(
            do_watering_helper_closest(
                &mut rt,
                0,
                0,
                &objs,
                &counts,
                &mut task,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_ne!(
            rt.weights.get(&FarmProfession::WaterBringer).copied(),
            Some(0.0)
        );
    }

    #[test]
    fn do_watering_helper_closest_skips_carrots_when_stock_high() {
        let mut rt = FarmProfessionRuntime::default();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[
            (CARROT, 20),
            (DRY_PLANTED_CARROTS, 5),
            (DRY_PLANTED_WHEAT, 2),
        ]);
        // Closest is carrots; stock ≥20 retargets to wheat.
        let objs = [
            FarmMapObj::simple(DRY_PLANTED_CARROTS, 1, 0),
            FarmMapObj::simple(DRY_PLANTED_WHEAT, 8, 0),
        ];
        assert_eq!(
            do_watering_helper_closest(
                &mut rt,
                0,
                0,
                &objs,
                &counts,
                &mut task,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
    }

    #[test]
    fn do_watering_helper_closest_zeros_weight_when_target_watering_fails() {
        let mut rt = FarmProfessionRuntime::default();
        rt.weights.insert(FarmProfession::WaterBringer, 1.0);
        let mut task = FarmTaskState::default();
        // Closest wheat is on the map, but home counts have none → doWateringOn fails.
        let counts = FarmCounts::default();
        let objs = [FarmMapObj::simple(DRY_PLANTED_WHEAT, 2, 0)];
        assert_eq!(
            do_watering_helper_closest(
                &mut rt,
                0,
                0,
                &objs,
                &counts,
                &mut task,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::None
        );
        assert_eq!(rt.weights.get(&FarmProfession::WaterBringer), Some(&0.0));
    }

    #[test]
    fn do_watering_helper_closest_no_target_leaves_weight() {
        let mut rt = FarmProfessionRuntime::default();
        rt.weights.insert(FarmProfession::WaterBringer, 1.0);
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 2)]);
        assert_eq!(
            do_watering_helper_closest(
                &mut rt,
                0,
                0,
                &[],
                &counts,
                &mut task,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::None
        );
        assert_eq!(rt.weights.get(&FarmProfession::WaterBringer), Some(&1.0));
    }

    #[test]
    fn do_watering_closest_assigned_max_ignores_small_peer_count() {
        let mut rt = FarmProfessionRuntime::default();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 2)]);
        let objs = [FarmMapObj::simple(DRY_PLANTED_WHEAT, 1, 0)];
        assert_eq!(
            do_watering_closest(
                &mut rt,
                0,
                0,
                &objs,
                &counts,
                &mut task,
                WATER_BRINGER_ASSIGNED_MAX_PEOPLE,
                3.0,
                0.0,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_eq!(rt.last_profession, Some(FarmProfession::WaterBringer));
        // Default max=2 with 3 peers still refuses (mid path).
        let mut rt2 = FarmProfessionRuntime::default();
        let mut task2 = FarmTaskState::default();
        assert_eq!(
            do_watering_closest(
                &mut rt2,
                0,
                0,
                &objs,
                &counts,
                &mut task2,
                2,
                3.0,
                0.0,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::None
        );
        assert_ne!(rt2.last_profession, Some(FarmProfession::WaterBringer));
    }

    #[test]
    fn do_watering_closest_low_max_refuses_one_peer() {
        let mut rt = FarmProfessionRuntime::default();
        let mut task = FarmTaskState::default();
        let counts = counts_with(&[(DRY_PLANTED_WHEAT, 2)]);
        let objs = [FarmMapObj::simple(DRY_PLANTED_WHEAT, 1, 0)];
        assert_eq!(
            watering_max_for_dispatch(false, DO_WATERING_LOW_RUNG),
            WATER_BRINGER_LOW_MAX_PEOPLE
        );
        assert_eq!(
            watering_max_for_dispatch(true, "ASSIGNED_JOB"),
            WATER_BRINGER_ASSIGNED_MAX_PEOPLE
        );
        assert_eq!(
            do_watering_closest(
                &mut rt,
                0,
                0,
                &objs,
                &counts,
                &mut task,
                WATER_BRINGER_LOW_MAX_PEOPLE,
                1.0,
                0.0,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::None
        );
        assert_ne!(rt.last_profession, Some(FarmProfession::WaterBringer));
        let mut rt2 = FarmProfessionRuntime::default();
        let mut task2 = FarmTaskState::default();
        assert_eq!(
            do_watering_closest(
                &mut rt2,
                0,
                0,
                &objs,
                &counts,
                &mut task2,
                WATER_BRINGER_LOW_MAX_PEOPLE,
                0.0,
                0.0,
                WATERING_SEARCH_DIST,
            ),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_eq!(rt2.last_profession, Some(FarmProfession::WaterBringer));
    }

    #[test]
    fn do_watering_helper_is_invoked_after_compost_in_basic_farm_order() {
        // Spot-check: helper returns wet wheat for dry wheat targets.
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(DRY_PLANTED_WHEAT, 2)]);
        assert_eq!(
            do_watering_helper(&c, &mut task),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        // Full basic body past early plant caps: mid watering before sheep.
        let mut task2 = FarmTaskState {
            wheat_harvester: 1.0,
            corn_planter: 0.0,
            harvest_corn: 0.0,
            composting: 0.0,
            ..Default::default()
        };
        let past = counts_with(&[
            (DRIED_CORN, 5),
            (THRESHED_WHEAT, 4),
            (DRY_PLANTED_WHEAT, 2),
            (WET_PLANTED_WHEAT, 15),
            (WET_PLANTED_CORN, 10),
            (TOMATO_PLANT, 5),
            (WET_PLANTED_BEANS, 4),
            (WET_PLANTED_CUCUMBER, 4),
            (WET_PLANTED_PEPPER, 5),
            (COMPOSTING_PILE, 2),
            (COMPOSTED_SOIL, 2),
        ]);
        assert_eq!(
            do_basic_farming(&past, &mut task2, true, BASIC_FARM_DEFAULT_MAX_PROFESSION),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
    }

    #[test]
    fn do_basic_farming_ex_mid_watering_peer_cap_falls_through_to_sheep() {
        // Past early plant caps + dry wheat for mid water; peer-cap full → sheep defer.
        let mut task = FarmTaskState {
            composting: 0.0,
            corn_planter: 0.0,
            harvest_corn: 0.0,
            wheat_harvester: 1.0,
            ..Default::default()
        };
        let c = counts_with(&[
            (DRIED_CORN, 5),
            (THRESHED_WHEAT, 4),
            (DRY_PLANTED_WHEAT, 2),
            (WET_PLANTED_WHEAT, 15),
            (WET_PLANTED_CORN, 10),
            (TOMATO_PLANT, 5),
            (WET_PLANTED_BEANS, 4),
            (WET_PLANTED_CUCUMBER, 4),
            (WET_PLANTED_PEPPER, 5),
            (COMPOSTING_PILE, 2),
            (COMPOSTED_SOIL, 2),
        ]);
        let mut rt = FarmProfessionRuntime::default();
        assert_eq!(
            do_basic_farming_ex(
                &c,
                &mut task,
                true,
                BASIC_FARM_DEFAULT_MAX_PROFESSION,
                Some((&mut rt, 3.0, 0.0)),
            ),
            FarmAction::DeferSheepHerding {
                max_profession: 2
            }
        );
        assert_ne!(rt.last_profession, Some(FarmProfession::WaterBringer));
        // Peer room → mid watering succeeds and sticky WaterBringer.
        let mut task2 = FarmTaskState {
            composting: 0.0,
            corn_planter: 0.0,
            harvest_corn: 0.0,
            wheat_harvester: 1.0,
            ..Default::default()
        };
        let mut rt2 = FarmProfessionRuntime::default();
        assert_eq!(
            do_basic_farming_ex(
                &c,
                &mut task2,
                true,
                BASIC_FARM_DEFAULT_MAX_PROFESSION,
                Some((&mut rt2, 0.0, 0.0)),
            ),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_WHEAT
            }
        );
        assert_eq!(rt2.last_profession, Some(FarmProfession::WaterBringer));
        assert_eq!(BASIC_FARM_MID_WATER_MAX_PEOPLE, 3);
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
        // Haxe L2424–2426 countCurrentObject includes held; pile 3902 ×2
        let mut held_dry = FarmCounts::default();
        held_dry.held_id = DRIED_CORN;
        let mut task_h = FarmTaskState {
            harvest_corn: 1.0,
            ..Default::default()
        };
        assert_eq!(
            do_harvest_corn(1, 1, &held_dry, &mut task_h),
            FarmAction::None,
            "held dried ear counts as stock"
        );
        assert_eq!(task_h.harvest_corn, 0.0);
        // Haxe L2441–2443: shuck success does not clear task (only fail does)
        let mut task_s = FarmTaskState {
            harvest_corn: 2.0,
            ..Default::default()
        };
        let ears = counts_with(&[(EAR_OF_CORN, 1)]);
        assert_eq!(
            do_harvest_corn(1, 5, &ears, &mut task_s),
            FarmAction::ShortCraft {
                actor: SHARP_STONE,
                target: EAR_OF_CORN
            }
        );
        assert_eq!(task_s.harvest_corn, 2.0);
        // Held pile 3902 = 2 dried
        let mut held_pile = FarmCounts::default();
        held_pile.held_id = PILE_DRIED_CORN;
        let mut task_p = FarmTaskState::default();
        assert_eq!(
            do_harvest_corn(1, 2, &held_pile, &mut task_p),
            FarmAction::None
        );
    }

    #[test]
    fn do_harvest_wheat_held_counts_and_chain() {
        // Haxe L2453–2483 countCurrentObject includes held
        let mut task = FarmTaskState::default();
        let mut held = FarmCounts::default();
        held.held_id = HARVESTED_WHEAT;
        assert_eq!(
            do_harvest_wheat(1, 4, &held, &mut task),
            FarmAction::CraftItem {
                object_id: WHEAT_SHEAF
            }
        );
        let mut sheaf = FarmCounts::default();
        sheaf.held_id = WHEAT_SHEAF;
        assert_eq!(
            do_harvest_wheat(1, 4, &sheaf, &mut task),
            FarmAction::CraftItem {
                object_id: THRESHED_WHEAT
            }
        );
        let mut ripe = FarmCounts::default();
        ripe.held_id = RIPE_WHEAT;
        assert_eq!(
            do_harvest_wheat(1, 4, &ripe, &mut task),
            FarmAction::CraftItem {
                object_id: HARVESTED_WHEAT
            }
        );
        // Ground 4069 counts as threshed
        let ground = counts_with(&[(THRESHED_WHEAT_GROUND, 4)]);
        assert_eq!(do_harvest_wheat(1, 4, &ground, &mut task), FarmAction::None);
        assert!(task.wheat_harvester > 0.0);
    }

    #[test]
    fn do_plant_pepper_and_wheat_wrapper_ids() {
        // Haxe L2490–2499 wrappers
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_pepper(2, 5, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_PEPPER
            }
        );
        assert_eq!(
            wheat_stage_count(&counts_with(&[
                (DRY_PLANTED_WHEAT, 1),
                (RIPE_WHEAT, 1),
                (WET_PLANTED_WHEAT, 1),
                (WHEAT_SPROUTS, 1),
                (UNRIPE_WHEAT, 1),
            ])),
            5
        );
    }

    #[test]
    fn do_plant_beans_tomato_cucumber_squash_milkweed_ids() {
        // Haxe L2502–2555 wrappers; tomato omits wet 2831 and sprout 2832
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_beans(2, 4, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_BEANS
            }
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_tomato(2, 5, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_TOMATO
            }
        );
        let wet_only = counts_with(&[(2831, 5), (TOMATO_SPROUT, 5)]);
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_tomato(2, 5, &wet_only, &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_TOMATO
            },
            "wet 2831 / sprout 2832 are not tomato stages"
        );
        let at_max = counts_with(&[(TOMATO_PLANT, 5)]);
        assert_eq!(
            do_plant_tomato(2, 5, &at_max, &mut FarmTaskState::default(), false),
            FarmAction::None
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_cucumber(2, 4, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_CUCUMBER
            }
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_squash(2, 5, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_SQUASH
            }
        );
        let squash_full = counts_with(&[
            (DRY_PLANTED_SQUASH, 1),
            (WET_PLANTED_SQUASH, 1),
            (HUBBARD_SQUASH, 1),
            (RIPE_SQUASH_PLANT, 1),
            (CROCK_WITH_SQUASH, 1),
            (PLATE_SQUASH_CHUNKS, 1),
            (PLATE_SQUASH_CHUNKS_SEEDS, 1),
        ]);
        assert_eq!(
            plant_stage_count(
                &squash_full,
                &[
                    DRY_PLANTED_SQUASH,
                    WET_PLANTED_SQUASH,
                    HUBBARD_SQUASH,
                    RIPE_SQUASH_PLANT,
                    CROCK_WITH_SQUASH,
                    PLATE_SQUASH_CHUNKS,
                    PLATE_SQUASH_CHUNKS_SEEDS,
                ]
            ),
            7
        );
        assert_eq!(
            do_plant_squash(2, 5, &squash_full, &mut FarmTaskState::default(), false),
            FarmAction::None
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_milkweed(2, 7, &FarmCounts::default(), &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_MILKWEED
            }
        );
        // Held dry seed counts (countCurrentObject)
        let mut held = FarmCounts::default();
        held.held_id = DRY_PLANTED_BEANS;
        assert_eq!(
            plant_stage_count(&held, &[DRY_PLANTED_BEANS, WET_PLANTED_BEANS]),
            1
        );
    }

    #[test]
    fn do_plant_watering_then_rows_then_craft() {
        // Haxe L2572–2582: CornPlanter latch then water min 3, doPrepareRows, craftItem
        let mut task = FarmTaskState {
            corn_planter: 1.0,
            ..Default::default()
        };
        let dry = counts_with(&[(DRY_PLANTED_BEANS, 3)]);
        assert_eq!(
            do_plant_beans(2, 4, &dry, &mut task, false),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_BEANS
            }
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_plant_beans(2, 4, &FarmCounts::default(), &mut task, true),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL
            }
        );
        // Haxe L2587–2599 CountCloseObjects (map, not held): count<1 clears flag
        let mut task = FarmTaskState::default();
        task.set_watering_flag(DRY_PLANTED_BEANS, 1.0);
        assert_eq!(
            do_watering_on(DRY_PLANTED_BEANS, 3, 0, Some(WET_PLANTED_BEANS), &mut task),
            FarmAction::None
        );
        assert_eq!(task.watering_flag(DRY_PLANTED_BEANS), 0.0);
        assert_eq!(
            do_watering_on(DRY_PLANTED_BEANS, 3, 2, Some(WET_PLANTED_BEANS), &mut task),
            FarmAction::None
        );
        assert_eq!(
            do_watering_on(DRY_PLANTED_BEANS, 3, 3, Some(WET_PLANTED_BEANS), &mut task),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_BEANS
            }
        );
        assert!(task.watering_flag(DRY_PLANTED_BEANS) >= 1.0);
    }

    #[test]
    fn do_basic_farming_after_sheep_nests_prepare_rows() {
        // Haxe L2408 doPlantWheat(15,30) → doPlant → doPrepareRows
        let mut task = FarmTaskState::default();
        let c = FarmCounts::default();
        assert_eq!(
            do_basic_farming_after_sheep(&c, &mut task, 25.0, 2),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL
            }
        );
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
        // Enough shallow rows, stage deep: steel hoe if present, else stone hoe
        // Haxe L2181 craftActor=false then L2184–2187 stone hoe
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(SHALLOW_TILLED_ROW, 9), (DEEP_TILLED_ROW, 0)]);
        let a = do_prepare_rows(&c, &mut task, true, false);
        assert_eq!(
            a,
            FarmAction::ShortCraft {
                actor: STONE_HOE,
                target: SHALLOW_TILLED_ROW
            }
        );
        let c_steel = counts_with(&[(SHALLOW_TILLED_ROW, 9), (DEEP_TILLED_ROW, 0), (STEEL_HOE, 1)]);
        assert_eq!(
            do_prepare_rows(&c_steel, &mut task, true, false),
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
    fn do_prepare_rows_deep_hard_row_then_steel_hoe_on_soil() {
        // Haxe L2189–2199: countHardRows>0 → RowMaker=1, 1137+848; steel hoe+soil if hoe present
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(DEEP_TILLED_ROW, 2), (HARDENED_ROW, 1)]);
        let a = do_prepare_rows(&c, &mut task, true, false);
        assert_eq!(
            a,
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: HARDENED_ROW,
            }
        );
        assert_eq!(task.row_maker, 1.0);
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(DEEP_TILLED_ROW, 2), (FERTILE_SOIL, 1), (STEEL_HOE, 1)]);
        assert_eq!(
            do_prepare_rows(&c, &mut task, true, false),
            FarmAction::ShortCraft {
                actor: STEEL_HOE,
                target: FERTILE_SOIL,
            }
        );
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(DEEP_TILLED_ROW, 2), (FERTILE_SOIL, 1)]);
        assert_eq!(
            do_prepare_rows(&c, &mut task, true, false),
            FarmAction::ShortCraft {
                actor: STONE_HOE,
                target: FERTILE_SOIL,
            }
        );
    }

    #[test]
    fn do_prepare_rows_stone_hoe_soil_then_pottery_when_deep_lt_6() {
        // Haxe L2200–2214: 850+1138; else doPottery(max) when deepRows<6 && bowls<1
        let mut task = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let c = counts_with(&[(DEEP_TILLED_ROW, 2)]);
        assert_eq!(
            do_prepare_rows(&c, &mut task, true, false),
            FarmAction::DeferPottery {
                max_profession: BASIC_FARM_DEFAULT_MAX_PROFESSION
            }
        );
        let mut task100 = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        assert_eq!(
            do_prepare_rows_ex(&c, &mut task100, true, false, 100),
            FarmAction::DeferPottery {
                max_profession: 100
            }
        );
        let mut task_bowl = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let with_bowl = counts_with(&[(DEEP_TILLED_ROW, 2), (CLAY_BOWL, 1)]);
        assert_eq!(
            do_prepare_rows(&with_bowl, &mut task_bowl, true, false),
            FarmAction::None
        );
        let mut task_held = FarmTaskState {
            row_maker: 2.0,
            ..Default::default()
        };
        let mut held = counts_with(&[(DEEP_TILLED_ROW, 2)]);
        held.held_id = CLAY_BOWL;
        assert_eq!(
            do_prepare_rows(&held, &mut task_held, true, false),
            FarmAction::None
        );
        // deepRows >= 6 skips pottery even with no bowls
        let deep_done = counts_with(&[(DEEP_TILLED_ROW, 6)]);
        let mut task3 = FarmTaskState {
            row_maker: 3.0,
            ..Default::default()
        };
        assert_eq!(
            do_prepare_rows(&deep_done, &mut task3, true, false),
            FarmAction::None
        );
    }

    #[test]
    fn do_plant_carrots_hysteresis_crafts_396() {
        // Haxe L2220–2254: count = carrots + 4*planted; >=max off; <=min on; craft 396
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(CARROT, 2)]);
        assert_eq!(carrot_stock_units(&c), 2);
        assert_eq!(
            do_plant_carrots(2, 5, &c, &mut task),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_CARROTS
            }
        );
        assert!(task.carrot_planter >= 1.0);
        let high = counts_with(&[(CARROT, 5)]);
        assert_eq!(do_plant_carrots(2, 5, &high, &mut task), FarmAction::None);
        assert_eq!(task.carrot_planter, 0.0);
        // planted 1 counts as 4 units + 0 carrots = 4; between min 2 and max 5, latch off → none
        let planted = counts_with(&[(DRY_PLANTED_CARROTS, 1)]);
        assert_eq!(carrot_stock_units(&planted), 4);
        assert_eq!(
            do_plant_carrots(2, 5, &planted, &mut task),
            FarmAction::None
        );
        task.carrot_planter = 1.0;
        assert_eq!(
            do_plant_carrots(2, 5, &planted, &mut task),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_CARROTS
            }
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
    fn do_prepare_soil_short_craft_on_ground_336_held_then_getitem() {
        // Haxe L2001 shortCraftOnGround(336): held → drop (target 0); else GetItem
        let mut task = FarmTaskState::default();
        let mut held = FarmCounts::default();
        held.held_id = BASKET_OF_SOIL;
        assert_eq!(
            do_prepare_soil(&held, &mut task, true),
            FarmAction::ShortCraft {
                actor: BASKET_OF_SOIL,
                target: 0,
            }
        );
        let on_map = counts_with(&[(BASKET_OF_SOIL, 1)]);
        assert_eq!(
            do_prepare_soil(&on_map, &mut task, true),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL,
            }
        );
    }

    #[test]
    fn do_prepare_soil_held_basket_uses_soil_source_then_craft_336() {
        // Haxe L2028–2051: held 292 + closest 336-transition target, else craftItem(336)
        let mut task = FarmTaskState::default();
        task.soil_maker = 1.0;
        let mut c = counts_with(&[(FERTILE_SOIL, 1)]);
        c.held_id = BASKET;
        assert_eq!(
            do_prepare_soil(&c, &mut task, true),
            FarmAction::ShortCraft {
                actor: BASKET,
                target: FERTILE_SOIL,
            }
        );
        let mut pile = counts_with(&[(FERTILE_SOIL_PILE, 1)]);
        pile.held_id = BASKET;
        task.soil_maker = 1.0;
        assert_eq!(
            do_prepare_soil(&pile, &mut task, true),
            FarmAction::ShortCraft {
                actor: BASKET,
                target: FERTILE_SOIL_PILE,
            }
        );
        let mut empty = FarmCounts::default();
        empty.held_id = BASKET;
        task.soil_maker = 1.0;
        assert_eq!(
            do_prepare_soil(&empty, &mut task, true),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL,
            }
        );
    }

    #[test]
    fn soil_unit_count_includes_held() {
        let mut c = counts_with(&[(FERTILE_SOIL_PILE, 1), (FERTILE_SOIL, 1)]);
        assert_eq!(soil_unit_count(&c), 3);
        c.held_id = FERTILE_SOIL_PILE;
        assert_eq!(soil_unit_count(&c), 5);
        c.held_id = DEEP_TILLED_ROW;
        assert_eq!(soil_unit_count(&c), 4);
    }

    #[test]
    fn do_prepare_soil_dung_on_wet_compost_needs_shovel() {
        // Haxe L1998 shortCraft(900,625,distance,false) — no craft missing shovel
        let mut task = FarmTaskState::default();
        let wet = counts_with(&[(WET_COMPOST, 1)]);
        let a = do_prepare_soil(&wet, &mut task, true);
        assert_ne!(
            a,
            FarmAction::ShortCraft {
                actor: SHOVEL_OF_DUNG,
                target: WET_COMPOST,
            },
            "without shovel should not emit dung+compost, got {:?}",
            a
        );
        let with_shovel = counts_with(&[(WET_COMPOST, 1), (SHOVEL_OF_DUNG, 1)]);
        assert_eq!(
            do_prepare_soil(&with_shovel, &mut task, true),
            FarmAction::ShortCraft {
                actor: SHOVEL_OF_DUNG,
                target: WET_COMPOST,
            }
        );
        let mut held = counts_with(&[(WET_COMPOST, 1)]);
        held.held_id = SHOVEL_OF_DUNG;
        assert_eq!(
            do_prepare_soil(&held, &mut task, true),
            FarmAction::ShortCraft {
                actor: SHOVEL_OF_DUNG,
                target: WET_COMPOST,
            }
        );
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
    ///
    /// Uses **wet** planted caps (not dry) so mid `doWatering(3)` has no targets and
    /// falls through to `DeferSheepHerding` (Haxe waters dry before sheep).
    fn counts_past_basic_mid_wave() -> FarmCounts {
        counts_with(&[
            (DRIED_CORN, 5),
            (THRESHED_WHEAT, 4),
            (WET_PLANTED_CORN, 10),
            (WET_PLANTED_WHEAT, 15),
            (TOMATO_PLANT, 5),
            (WET_PLANTED_BEANS, 4),
            (WET_PLANTED_CUCUMBER, 4),
            (WET_PLANTED_PEPPER, 5),
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
        // Assigned BASICFARMER: doBasicFarming(100) → advanced max 100
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
            (CLAY_BOWL, 0), // Haxe L3929 return doPottery(3)
        ]);
        task.soil_maker = 0.0;
        task.row_maker = 3.0;
        assert_eq!(
            expand_advanced_farming_or_clear(&c_adv, &mut task, 25.0, true),
            FarmAction::DeferPottery {
                max_profession: ADVANCED_FARM_POTTERY_MAX_PEOPLE
            }
        );
        assert_eq!(
            FarmAction::ClearBasicFarmerWeight.basic_farmer_weight_side_effect(),
            Some(0.0)
        );
    }

    #[test]
    fn wet_onion_in_rotation_crafts_item() {
        // Haxe L4044 craftItem(toPlant) for 2852 (no count>6 gate — that's only 2851)
        assert_eq!(
            FarmAction::ClearAdvancedFarmerWeight.advanced_farmer_weight_side_effect(),
            Some(0.0)
        );
        let mut task = FarmTaskState::default();
        assert_eq!(
            do_advanced_farming_step(WET_PLANTED_ONIONS, &FarmCounts::default(), &mut task, 4),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_ONIONS
            }
        );
    }

    #[test]
    fn garlic_over_two_skips_craft() {
        // Haxe L4001–4008 CountClose 4262+4265 > 2 → continue
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(DRY_PLANTED_GARLIC, 2), (MATURE_GARLIC, 1)]);
        assert_eq!(
            do_advanced_farming_step(DRY_PLANTED_GARLIC, &c, &mut task, 4),
            FarmAction::None
        );
        let c2 = counts_with(&[(DRY_PLANTED_GARLIC, 1)]);
        assert_eq!(
            do_advanced_farming_step(DRY_PLANTED_GARLIC, &c2, &mut task, 4),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_GARLIC
            }
        );
        assert_eq!(ADVANCED_GARLIC_COUNT_RADIUS, 30);
        assert_eq!(ADVANCED_GARLIC_SKIP_ABOVE, 2);
    }

    fn idle_advanced_task() -> FarmTaskState {
        FarmTaskState {
            row_maker: 3.0,
            soil_maker: 0.0,
            corn_planter: 0.0,
            ..Default::default()
        }
    }

    fn idle_advanced_counts(extra: &[(i32, i32)]) -> FarmCounts {
        let mut pairs = vec![
            (DEEP_TILLED_ROW, 12),
            (FERTILE_SOIL_PILE, 6),
            (CLAY_BOWL, 2),
        ];
        pairs.extend_from_slice(extra);
        let mut c = counts_with(&pairs);
        c.bowl_count_home_15 = Some(2);
        c
    }

    #[test]
    fn do_advanced_farming_pepper_squash_tomato_onion_tail() {
        // Haxe L4011–4044: doPlantPepper(2,5); squash skip; doPlantTomato(1,8);
        // onions CountClose 2854+2851 > 6 skip else craftItem
        let mut task = idle_advanced_task();
        task.to_plant = 2;
        let c = idle_advanced_counts(&[]);
        assert_eq!(
            do_advanced_farming_ex(&c, &mut task, 0.0, true, 2, 2),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_PEPPER
            }
        );

        let mut task = idle_advanced_task();
        task.to_plant = 6;
        assert_eq!(
            do_advanced_farming_ex(&c, &mut task, 0.0, true, 2, 6),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_TOMATO
            }
        );

        let mut task = idle_advanced_task();
        assert_eq!(
            do_advanced_farming_step(DRY_PLANTED_SQUASH, &c, &mut task, 2),
            FarmAction::None
        );
        // Squash skip in the loop → next array slot is potato 1145 (needs shovel)
        let with_shovel = idle_advanced_counts(&[(SHOVEL, 1)]);
        let mut task = idle_advanced_task();
        task.to_plant = 7;
        assert_eq!(
            do_advanced_farming_ex(&with_shovel, &mut task, 0.0, true, 2, 7),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_POTATO
            }
        );

        let onions_high = idle_advanced_counts(&[(DRY_PLANTED_ONIONS, 4), (RIPE_ONIONS, 3)]);
        let mut task = idle_advanced_task();
        assert_eq!(
            do_advanced_farming_step(DRY_PLANTED_ONIONS, &onions_high, &mut task, 2),
            FarmAction::None
        );
        let onions_low = idle_advanced_counts(&[(DRY_PLANTED_ONIONS, 2), (RIPE_ONIONS, 1)]);
        let mut task = idle_advanced_task();
        task.to_plant = 3;
        assert_eq!(
            do_advanced_farming_ex(&onions_low, &mut task, 0.0, true, 2, 3),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_ONIONS
            }
        );
        // 2852 has no count gate (Haxe if is only 2851)
        let mut task = idle_advanced_task();
        task.to_plant = 9;
        assert_eq!(
            do_advanced_farming_ex(&onions_high, &mut task, 0.0, true, 2, 9),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_ONIONS
            }
        );
        assert_eq!(ADVANCED_ONION_COUNT_RADIUS, 30);
        assert_eq!(ADVANCED_ONION_SKIP_ABOVE, 6);
    }

    #[test]
    fn do_advanced_farming_potato_shortcraft_then_pottery_r15() {
        // Haxe L3917–3929: 502+1146 then 1137+1143 then bowls r=15 → doPottery(3)
        let mut task = FarmTaskState {
            row_maker: 3.0,
            soil_maker: 0.0,
            ..Default::default()
        };
        let mut c = counts_with(&[(MATURE_POTATO, 1), (DEEP_TILLED_ROW, 12), (FERTILE_SOIL_PILE, 6)]);
        assert_eq!(
            do_advanced_farming(&c, &mut task, 20.0, true),
            FarmAction::ShortCraft {
                actor: SHOVEL,
                target: MATURE_POTATO
            }
        );
        c = counts_with(&[(POTATO_PLANTS, 1), (DEEP_TILLED_ROW, 12), (FERTILE_SOIL_PILE, 6)]);
        assert_eq!(
            do_advanced_farming(&c, &mut task, 20.0, true),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: POTATO_PLANTS
            }
        );
        let empty = counts_with(&[(DEEP_TILLED_ROW, 12), (FERTILE_SOIL_PILE, 6)]);
        assert_eq!(
            do_advanced_farming(&empty, &mut task, 20.0, true),
            FarmAction::DeferPottery {
                max_profession: ADVANCED_FARM_POTTERY_MAX_PEOPLE
            }
        );
        assert_eq!(ADVANCED_BOWL_COUNT_RADIUS, 15);
        assert_eq!(ADVANCED_FARM_DEFAULT_MAX_PEOPLE, 2);
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
        assert_eq!(MAKE_SHARPIE_FOOD_DEFAULT_MAX_DISTANCE, 40);
        assert_eq!(MAKE_SHARPIE_FOOD_CLOSE_SEARCH_MAX, 10);
        assert_eq!(MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE, 5);
        assert!(make_sharpie_food_uses_player_search(5));
        assert!(make_sharpie_food_uses_player_search(10));
        assert!(!make_sharpie_food_uses_player_search(20));
        assert!(!make_sharpie_food_uses_player_search(40));
        // L4108 GetClosest square: (5,0) is on the exclusive high edge of r=5
        assert!(make_sharpie_food_plant_in_range(5, 0, 0, 4, 0));
        assert!(!make_sharpie_food_plant_in_range(5, 0, 0, 5, 0));
        assert!(make_sharpie_food_plant_in_range(5, 0, 0, 4, 4));
        // L4109 getClosestObjectById quad gate
        assert!(make_sharpie_food_plant_in_range(20, 0, 0, 15, 0));
        assert!(!make_sharpie_food_plant_in_range(20, 0, 0, 21, 0));
        assert_eq!(
            make_sharpie_food_from_xy(
                0,
                0,
                0,
                [(SEEDING_WILD_CARROT, 5, 0)],
                MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE,
            ),
            FarmAction::None
        );
        assert_eq!(
            make_sharpie_food_from_xy(
                0,
                0,
                0,
                [(SEEDING_WILD_CARROT, 4, 0)],
                MAKE_SHARPIE_FOOD_CLOSE_CALL_DISTANCE,
            ),
            FarmAction::CraftItem {
                object_id: SHARP_STONE
            }
        );
        assert_eq!(
            make_sharpie_food_from_xy(
                0,
                0,
                SHARP_STONE,
                [(BURDOCK, 12, 0)],
                20,
            ),
            FarmAction::CraftItem {
                object_id: DUG_BURDOCK
            }
        );
    }

    #[test]
    fn consider_making_food_skip_starving_near_and_meh_scale() {
        // Haxe L8487–8497: no target → do not skip after enter
        assert!(!consider_making_food_skip_after_enter(None, 0.0));
        assert!(!consider_making_food_skip_after_enter(None, -2.0));
        // starving with a target → skip
        assert!(consider_making_food_skip_after_enter(
            Some((10_000.0, false, false)),
            -1.1
        ));
        assert!(!consider_making_food_skip_after_enter(
            Some((10_000.0, false, false)),
            -1.0
        ));
        // raw quad 899 near; 900 not < 900
        assert!(consider_making_food_skip_after_enter(
            Some((899.0, false, false)),
            2.0
        ));
        assert!(!consider_making_food_skip_after_enter(
            Some((900.0, false, false)),
            2.0
        ));
        // meh ×4: 300 → 1200 not near; 224 → 896 near
        assert!(!consider_making_food_skip_after_enter(
            Some((300.0, true, false)),
            2.0
        ));
        assert!(consider_making_food_skip_after_enter(
            Some((224.0, true, false)),
            2.0
        ));
        // superMeh stacks on meh: 60 ×4 ×4 = 960 not near; 56 ×16 = 896 near
        assert!(!consider_making_food_skip_after_enter(
            Some((60.0, true, true)),
            2.0
        ));
        assert!(consider_making_food_skip_after_enter(
            Some((56.0, true, true)),
            2.0
        ));
        assert_eq!(
            consider_making_food_scaled_food_quad(10.0, true, true),
            160.0
        );
        assert_eq!(CONSIDER_MAKE_FOOD_NEAR_QUAD, 900.0);
    }

    #[test]
    fn consider_making_food_home_do_stuff_corn_rabbit() {
        // L8504: no target food_quad=-1 → any home_quad ≥ 0 skips
        assert!(consider_making_food_skip_too_far_from_home(0.0, -1.0));
        assert!(consider_making_food_skip_after_enter_with_home(None, 2.0, 0.0));
        // far food 10000, home 200 → make food
        assert!(!consider_making_food_skip_after_enter_with_home(
            Some((10_000.0, false, false)),
            2.0,
            200.0
        ));
        // home farther than food → skip
        assert!(consider_making_food_skip_after_enter_with_home(
            Some((10_000.0, false, false)),
            2.0,
            10_001.0
        ));
        assert!(consider_making_food_should_research(15.1));
        assert!(!consider_making_food_should_research(15.0));
        // no threat, mild heat, home close → doStuff true
        assert!(consider_making_food_do_stuff(0.5, None, None, 0.0) == false); // dist 10000 > 400
        assert!(consider_making_food_do_stuff(0.5, Some(50.0), None, 0.0));
        assert!(!consider_making_food_do_stuff(0.05, Some(50.0), None, 1000.0)); // home > 900 wins
        assert!(consider_making_food_do_stuff(0.05, Some(50.0), None, 0.0)); // close attacker overrides temp
        assert!(!consider_making_food_do_stuff(0.5, Some(50.0), None, 901.0));
        let mut flag = 0.0;
        let p = consider_making_food_ear_of_corn_maker(true, 0, 0, 0, &mut flag);
        assert_eq!(flag, 1.0);
        assert!(p.pick_ear);
        assert!(!p.shuck);
        let p = consider_making_food_ear_of_corn_maker(true, 6, 2, 0, &mut flag);
        assert_eq!(flag, 0.0);
        assert!(!p.pick_ear);
        let p = consider_making_food_ear_of_corn_maker(true, 0, 1, 1, &mut flag);
        assert!(p.shuck);
        assert!(!consider_making_food_ear_of_corn_maker(false, 0, 0, 0, &mut flag).pick_ear);
        assert_eq!(
            consider_making_food_raw_rabbit_count(1, true, 0, true),
            3
        );
        assert!(consider_making_food_fire_food_on_extra_rabbit(2));
        assert!(!consider_making_food_fire_food_on_extra_rabbit(1));
        assert!(consider_making_food_fire_food_on_few_rabbit(1));
        assert!(consider_making_food_fire_food_on_few_rabbit(0));
        assert!(!consider_making_food_fire_food_on_few_rabbit(2));
        assert_eq!(consider_making_food_short_crafts()[0], (0, 400, 10, -1));
        assert_eq!(MAKE_SHARPIE_FOOD_FAR_CALL_DISTANCE, 20);
        assert_eq!(TURKEY_SLICE_ON_PLATE, 2190);
    }

    #[test]
    fn do_carrot_farming_low_max_refuses_one_peer() {
        let mut rt = FarmProfessionRuntime::default();
        assert!(!has_or_become_profession(
            &mut rt,
            FarmProfession::CarrotFarmer,
            CARROT_FARMER_LOW_MAX_PEOPLE,
            1.0,
            0.0,
        ));
        assert_ne!(rt.last_profession, Some(FarmProfession::CarrotFarmer));
        let mut rt2 = FarmProfessionRuntime::default();
        assert!(has_or_become_profession(
            &mut rt2,
            FarmProfession::CarrotFarmer,
            CARROT_FARMER_LOW_MAX_PEOPLE,
            0.0,
            0.0,
        ));
        assert_eq!(rt2.last_profession, Some(FarmProfession::CarrotFarmer));
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(CARROT_ROW, 1)]);
        assert_eq!(
            do_carrot_farming(&c, &mut task, true),
            FarmAction::ShortCraft {
                actor: 0,
                target: CARROT_ROW
            }
        );
    }

    fn fill_bean_green_base() -> FillBeanBowlInput {
        FillBeanBowlInput {
            held_id: 0,
            held_uses: 1,
            held_num_uses: 5,
            green_beans: true,
            only_fill_held: false,
            count_dry_beans: 1,
            plant_xy: Some((2, 0)),
            bowl_xy: None,
            bowl_uses: 1,
            bowl_num_uses: 5,
            is_best_bowl_filler: true,
        }
    }

    #[test]
    fn fill_bean_bowl_if_needed_green_held_and_pickup() {
        // Haxe fillBeanBowlIfNeeded(): green + plant + dry stock.
        let mut inp = fill_bean_green_base();
        inp.held_id = BOWL_OF_GREEN_BEANS;
        assert_eq!(
            fill_bean_bowl_if_needed(&inp),
            FillBeanBowlAction::UseHeldOnPlant {
                x: 2,
                y: 0,
                plant_id: GREEN_BEAN_PLANTS
            }
        );
        inp.held_uses = 5;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.held_id = 0;
        inp.held_uses = 1;
        inp.count_dry_beans = 0;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.count_dry_beans = 1;
        inp.plant_xy = None;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.plant_xy = Some((2, 0));
        assert_eq!(
            fill_bean_bowl_if_needed(&inp),
            FillBeanBowlAction::GetClayBowl
        );
        inp.bowl_xy = Some((1, 0));
        assert_eq!(
            fill_bean_bowl_if_needed(&inp),
            FillBeanBowlAction::PickupBowl {
                x: 1,
                y: 0,
                bowl_id: BOWL_OF_GREEN_BEANS
            }
        );
        inp.bowl_uses = 5;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.bowl_uses = 1;
        inp.is_best_bowl_filler = false;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.bowl_xy = None;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        inp.held_id = CLAY_BOWL;
        inp.is_best_bowl_filler = true;
        assert_eq!(
            fill_bean_bowl_if_needed(&inp),
            FillBeanBowlAction::UseHeldOnPlant {
                x: 2,
                y: 0,
                plant_id: GREEN_BEAN_PLANTS
            }
        );
        inp.held_id = 0;
        inp.only_fill_held = true;
        assert_eq!(fill_bean_bowl_if_needed(&inp), FillBeanBowlAction::None);
        assert_eq!(FILL_BEAN_BOWL_RUNG, "FILL_BEAN_BOWL");
        assert_eq!(FILL_BEAN_HELD_RUNG, "FILL_BEAN_HELD");
        assert_eq!(
            bean_bowl_ids(true),
            (BOWL_OF_GREEN_BEANS, GREEN_BEAN_PLANTS)
        );
        assert_eq!(bean_bowl_ids(false), (BOWL_OF_DRY_BEANS, DRY_BEAN_PLANTS));
    }

    #[test]
    fn fill_bean_bowl_held_if_needed_green_then_dry() {
        // Haxe fillBeanBowlIfNeeded(true, true) then (false, true): held only.
        let mut green = fill_bean_green_base();
        green.only_fill_held = true;
        green.held_id = BOWL_OF_GREEN_BEANS;
        let mut dry = fill_bean_green_base();
        dry.green_beans = false;
        dry.only_fill_held = true;
        dry.held_id = BOWL_OF_DRY_BEANS;
        dry.plant_xy = Some((3, 0));
        assert_eq!(
            fill_bean_bowl_held_if_needed(&green, &dry),
            FillBeanBowlAction::UseHeldOnPlant {
                x: 2,
                y: 0,
                plant_id: GREEN_BEAN_PLANTS
            }
        );
        green.held_id = 0;
        assert_eq!(
            fill_bean_bowl_held_if_needed(&green, &dry),
            FillBeanBowlAction::UseHeldOnPlant {
                x: 3,
                y: 0,
                plant_id: DRY_BEAN_PLANTS
            }
        );
        dry.held_id = 0;
        dry.bowl_xy = Some((1, 0));
        assert_eq!(
            fill_bean_bowl_held_if_needed(&green, &dry),
            FillBeanBowlAction::None
        );
    }

    #[test]
    fn pull_carrot_row_if_needed_empty_hand_drop_and_seed_guard() {
        // Haxe shortCraft(0, 400, 10): empty USE; seed guard uses<4; held → drop.
        let mut inp = PullCarrotRowInput {
            held_id: 0,
            food_store: 20.0,
            transition_hungry_cost: 0.0,
            has_carrot_seeds: true,
            row: Some((2, 0, 1)),
        };
        assert_eq!(
            pull_carrot_row_if_needed(&inp),
            PullCarrotRowAction::UseEmptyOnRow { x: 2, y: 0 }
        );
        inp.row = None;
        assert_eq!(pull_carrot_row_if_needed(&inp), PullCarrotRowAction::None);
        inp.row = Some((2, 0, 1));
        inp.has_carrot_seeds = false;
        assert_eq!(pull_carrot_row_if_needed(&inp), PullCarrotRowAction::None);
        inp.row = Some((2, 0, 4));
        assert_eq!(
            pull_carrot_row_if_needed(&inp),
            PullCarrotRowAction::UseEmptyOnRow { x: 2, y: 0 }
        );
        inp.has_carrot_seeds = true;
        inp.row = Some((2, 0, 1));
        inp.held_id = CARROT;
        assert_eq!(pull_carrot_row_if_needed(&inp), PullCarrotRowAction::DropHeld);
        inp.food_store = 0.0;
        inp.transition_hungry_cost = 5.0;
        assert_eq!(pull_carrot_row_if_needed(&inp), PullCarrotRowAction::None);
        assert_eq!(PULL_CARROT_ROW_RUNG, "PULL_CARROT_ROW");
        assert_eq!(PULL_CARROT_ROW_SEARCH_DIST, 10);
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
    fn do_carrot_farming_skewer_sprouts_then_cleanup_tail() {
        // Haxe L1952–1953 shortCraft 139+2832 / 139+4228 before rows
        let mut task = FarmTaskState::default();
        let c = counts_with(&[(TOMATO_SPROUT, 1)]);
        assert_eq!(
            do_carrot_farming(&c, &mut task, true),
            FarmAction::ShortCraft {
                actor: SKEWER,
                target: TOMATO_SPROUT,
            }
        );
        let c = counts_with(&[(CUCUMBER_SPROUT, 1)]);
        assert_eq!(
            do_carrot_farming(&c, &mut task, true),
            FarmAction::ShortCraft {
                actor: SKEWER,
                target: CUCUMBER_SPROUT,
            }
        );
        // Haxe L1975–1976 Bowl of Soil + Dying Bush; L1986 cleanUp
        let mut task = FarmTaskState::default();
        let c = counts_with(&[
            (DYING_BUSH, 1),
            (FERTILE_SOIL_PILE, 6),
            (DEEP_TILLED_ROW, 12),
            (CARROT, 8),
            (COMPOSTING_PILE, 5),
        ]);
        task.soil_maker = 0.0;
        task.row_maker = 3.0;
        task.composting = 0.0;
        task.carrot_planter = 0.0;
        assert_eq!(
            do_carrot_farming(&c, &mut task, true),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        let mut c2 = c.clone();
        c2.set(DYING_BUSH, 0);
        assert_eq!(
            do_carrot_farming(&c2, &mut task, true),
            FarmAction::DeferCleanup
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
        // Haxe L2267–2270 Bowl of Soil + Dying/Languishing before rows
        let mut task = FarmTaskState::default();
        let dying = counts_with(&[(DYING_BUSH, 1)]);
        assert_eq!(
            do_berry_farming(&dying, &mut task, true),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
        let lang = counts_with(&[(LANGUISHING_BUSH, 1)]);
        assert_eq!(
            do_berry_farming(&lang, &mut task, true),
            FarmAction::ShortCraft {
                actor: BOWL_OF_SOIL,
                target: LANGUISHING_BUSH,
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
        // Haxe L2300 doWateringOn(393, 3) first in doPlantBushes
        let dry_bush = counts_with(&[(DRY_DOMESTIC_BUSH, 3)]);
        assert_eq!(
            do_plant_bushes(&dry_bush, &mut task),
            FarmAction::CraftItem {
                object_id: DOMESTIC_BUSH
            }
        );
        // Haxe L2302 doWateringOn(216, 3)
        let mut task216 = FarmTaskState::default();
        let dry_seed = counts_with(&[(DRY_PLANTED_GOOSEBERRY, 3)]);
        assert_eq!(
            do_plant_bushes(&dry_seed, &mut task216),
            FarmAction::CraftItem {
                object_id: WET_PLANTED_GOOSEBERRY
            }
        );
        // At max (3) stop
        let c = counts_with(&[(DOMESTIC_BUSH, 3)]);
        assert_eq!(do_plant_bushes(&c, &mut task), FarmAction::None);
        // Haxe L2326 BASICFARMER >= 7 → maxBushes 9; 3 domestic still plants
        let mut heavy = counts_with(&[(DOMESTIC_BUSH, 3)]);
        heavy.basic_farmer_weight = 7.0;
        assert_eq!(max_bushes(7.0), 9);
        assert_eq!(
            do_plant_bushes(&heavy, &mut FarmTaskState::default()),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_GOOSEBERRY
            }
        );
        // Haxe L2307–2322 all eight bush-family ids
        let all_ids = counts_with(&[
            (DOMESTIC_BUSH, 1),
            (DRY_DOMESTIC_BUSH, 1),
            (EMPTY_DOMESTIC_BUSH, 1),
            (VIGOROUS_DOMESTIC_BUSH, 1),
            (GOOSEBERRY_SPROUT, 1),
            (WET_PLANTED_GOOSEBERRY, 1),
            (DRY_PLANTED_GOOSEBERRY, 1),
            (DYING_BUSH, 1),
        ]);
        assert_eq!(bush_stage_count(&all_ids), 8);
        assert_eq!(
            do_plant_bushes(&all_ids, &mut FarmTaskState::default()),
            FarmAction::None,
            "8 >= max 3"
        );
    }

    #[test]
    fn do_plant_potatos_requires_shovel_including_held() {
        // Haxe L2516–2517 countCurrentObject(502) includes held
        let mut task = FarmTaskState::default();
        let empty = FarmCounts::default();
        assert_eq!(
            do_plant_potatos(2, 5, &empty, &mut task, false),
            FarmAction::None
        );
        let on_map = counts_with(&[(SHOVEL, 1)]);
        assert_eq!(
            do_plant_potatos(2, 5, &on_map, &mut task, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_POTATO
            }
        );
        let mut held = FarmCounts::default();
        held.held_id = SHOVEL;
        let mut task2 = FarmTaskState::default();
        assert_eq!(
            do_plant_potatos(2, 5, &held, &mut task2, false),
            FarmAction::CraftItem {
                object_id: DRY_PLANTED_POTATO
            }
        );
    }

    #[test]
    fn do_basic_farming_do_plant_nests_prepare_rows() {
        // Haxe L2376 doPlantCorn(1,3) → doPlant → doPrepareRows → doPrepareSoil
        let mut task = FarmTaskState::default();
        let c = FarmCounts::default();
        assert_eq!(
            do_basic_farming(&c, &mut task, true, BASIC_FARM_DEFAULT_MAX_PROFESSION),
            FarmAction::CraftItem {
                object_id: BASKET_OF_SOIL
            }
        );
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
        assert_eq!(
            farm_action_to_goal(FarmAction::DeferPottery { max_profession: 2 }),
            Goal::SeekObject(CLAY_BOWL)
        );
        assert!(farm_job_rung_label("ASSIGNED_JOB"));
        assert!(farm_job_rung_label("AGE_ROTATED_JOB"));
        assert!(farm_job_rung_label(DO_WATERING_LOW_RUNG));
        assert!(farm_job_rung_label(DO_CARROT_LOW_RUNG));
        assert!(!farm_job_rung_label("ESCAPE"));
        assert_eq!(
            carrot_max_for_dispatch(false, DO_CARROT_LOW_RUNG),
            CARROT_FARMER_LOW_MAX_PEOPLE
        );
        assert_eq!(
            carrot_max_for_dispatch(true, "ASSIGNED_JOB"),
            CARROT_FARMER_ASSIGNED_MAX_PEOPLE
        );
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
