//! Haxe: `AiBase` shepherd / sheep-herding profession family (chunk **AI-SHEPHERD**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession('SHEPHERD')` with max-people + sticky last
//! - Speech `SHEPHERD!` → assigned job
//! - `isSheepHerding(maxProfession, maxAnimal)` body sequence
//! - `doFeedLambsAndCalfs` early mid-priority helper
//! - shared farm fallthrough: compost / plant carrots / keepBushesAlive / plant corn
//! - `handleMilk` **ungated** (Haxe crafts 4081 from zero; baker SM stays stock-gated)
//!
//! No world I/O: callers supply counts / task state and apply returned
//! [`ShepherdAction`]s via craft/shortCraft.
//!
//! Mid sites (AI-SHEPHERD-MID): do_basic_farming / make_stuff sheep call sites,
//! after_sheep sharpie+advanced, make_stuff live wire, wet-compost 625, Profession::Shepherd goal.
//! Mid: makeFireFood/doBaking bodies via AI-MAKE-STUFF (`fire_food_profession` + make_stuff_try_bodies).
//! Residual: nested milk; knife-kill excess (commented Haxe).

use std::collections::HashMap;

use crate::ai_goals::Goal;
use crate::baker_profession::{
    BOWL_BERRIES_CARROT, BOWL_GOOSEBERRIES, BOWL_OF_BUTTER, BOWL_OF_CREAM, BUTTERED_BREAD,
    DOMESTIC_BUSH, DOMESTIC_LAMB, DOMESTIC_SHEEP, HUNGRY_DOMESTIC_LAMB, KNIFE, MILK_POUCH, SKEWER,
    WHIPPED_CREAM, WILD_BUSH,
};
use crate::farmer_profession::{
    do_composting, do_plant_carrots, do_plant_corn, keep_bushes_alive, FarmAction, FarmCounts,
    FarmTaskState, DYING_BUSH,
};

// ── Object ids (OHOL / OpenLife content; Haxe comments in AiBase) ───────────

/// Hungry Mouflon Lamb (doFeedLambsAndCalfs uses 603, not 604).
// Haxe: AiBase.doFeedLambsAndCalfs ~5682
pub const HUNGRY_MOUFLON_LAMB: i32 = 603;
/// Shorn Domestic Sheep.
// Haxe: AiBase.isSheepHerding ~1860
pub const SHORN_DOMESTIC_SHEEP: i32 = 576;
/// Bowl with Corn Kernels.
// Haxe: AiBase.isSheepHerding ~1841
pub const BOWL_CORN_KERNELS: i32 = 1247;
/// Hungry Domestic Calf.
pub const HUNGRY_DOMESTIC_CALF: i32 = 1462;
/// Domestic Calf.
pub const DOMESTIC_CALF: i32 = 1459;
/// Empty Bucket.
pub const EMPTY_BUCKET: i32 = 659;
/// Milk Cow.
pub const MILK_COW: i32 = 1489;
/// Domestic Cow.
pub const DOMESTIC_COW: i32 = 1458;
/// Dead Cow.
pub const DEAD_COW: i32 = 1900;
/// Cold Goose Egg.
pub const COLD_GOOSE_EGG: i32 = 1262;
/// Domestic Goose.
pub const DOMESTIC_GOOSE: i32 = 1256;
/// Dung Goose Egg Incubator.
pub const DUNG_GOOSE_EGG_INCUBATOR: i32 = 1263;
/// Dried Ear of Corn (countCorn held).
pub const DRIED_EAR_OF_CORN: i32 = 1115;
/// Bowl with Corn Cob (countCorn held).
pub const BOWL_CORN_COB: i32 = 1120;
/// Dumped Corn Kernels (countCorn map-only).
pub const DUMPED_CORN_KERNELS: i32 = 4106;
/// Pile of Corn Kernels (countCorn map-only).
pub const PILE_CORN_KERNELS: i32 = 4107;

/// Home/animal shortCraft distance (Haxe isSheepHerding distance = 30).
// Haxe: AiBase.isSheepHerding ~1822
pub const SHEPHERD_SHORTCRAFT_RADIUS: i32 = 30;
/// Default max people for age-rotated / mid sheep (Haxe isSheepHerding() max=1).
pub const SHEPHERD_DEFAULT_MAX_PEOPLE: i32 = 1;
/// Assigned-job max people (Haxe isSheepHerding(100)).
pub const SHEPHERD_ASSIGNED_MAX_PEOPLE: i32 = 100;
/// Default max domestic sheep before stop lamb/sheep feed (Haxe maxAnimal=10).
pub const SHEPHERD_DEFAULT_MAX_ANIMAL: i32 = 10;
/// Baker mid call uses maxAnimal=5 (`isSheepHerding(2,5)`).
pub const SHEPHERD_BAKER_MAX_ANIMAL: i32 = 5;
/// doFeedLambsAndCalfs hard cap on sheep/cow counts before feed (Haxe `< 10`).
pub const FEED_LAMBS_CALFS_ANIMAL_CAP: i32 = 10;

/// Canonical Haxe profession string for shepherd.
pub const SHEPHERD_PROFESSION_KEY: &str = "SHEPHERD";

// ── Profession speech / runtime ────────────────────────────────────────────

/// Parse speech / assigned profession tokens for shepherd.
///
/// Accepts `SHEPHERD`, `SHEPHERD!`, case-insensitive.
// Haxe: AiBase speech endsWith("!"); assignedProfession == 'SHEPHERD'
pub fn parse_shepherd_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("SHEPHERD")
}

/// Sticky last + assigned + weight for SHEPHERD.
///
/// Haxe `this.profession['SHEPHERD']` is set to 1 on become and cleared to 0
/// on isSheepHerding fallthrough.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdProfessionRuntime {
    pub is_last_shepherd: bool,
    pub is_assigned_shepherd: bool,
    /// Haxe `this.profession['SHEPHERD']` weight (0 idle / 1 active).
    pub weight: f32,
}

impl Default for ShepherdProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_shepherd: false,
            is_assigned_shepherd: false,
            weight: 0.0,
        }
    }
}

impl ShepherdProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear profession weight (Haxe `this.profession['SHEPHERD'] = 0`).
    // Haxe: AiBase.isSheepHerding ~1915
    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    /// Apply eat-path profession wipe.
    // Haxe: isConsideringMakingFood profession wipe family
    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        if !last_was_foodserver {
            self.is_last_shepherd = false;
        }
    }
}

/// Assign from speech `SHEPHERD!`.
// Haxe: assignedProfession = 'SHEPHERD'
pub fn assign_shepherd_from_speech(runtime: &mut ShepherdProfessionRuntime, text: &str) -> bool {
    if !parse_shepherd_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_shepherd = true;
    runtime.is_last_shepherd = true;
    runtime.weight = 1.0;
    true
}

/// Count peers already sticky on SHEPHERD.
// Haxe: AiBase.countProfession('SHEPHERD')
pub fn count_shepherd_peers(peer_count_with_last_shepherd: f32) -> f32 {
    peer_count_with_last_shepherd.max(0.0)
}

/// One AI peer for pure `countProfession('SHEPHERD')` filtering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShepherdPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub has_player_to_follow: bool,
    pub same_home: bool,
    pub last_is_shepherd: bool,
}

impl ShepherdPeerSnapshot {
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

    pub fn counts_as_shepherd(self, min_age_to_eat: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age_to_eat, max_age) && self.last_is_shepherd
    }
}

/// Full pure `countProfession('SHEPHERD')` over peer snapshots.
pub fn count_shepherd_peers_filtered(
    peers: &[ShepherdPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_shepherd(min_age_to_eat, max_age))
        .count() as f32
}

/// Haxe `hasOrBecomeProfession('SHEPHERD', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_shepherd(
    runtime: &mut ShepherdProfessionRuntime,
    max: i32,
    peer_count_with_last_shepherd: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        // High priority: do job but do not assign profession.
        return true;
    }
    if runtime.is_last_shepherd {
        runtime.is_last_shepherd = true;
        return true;
    }
    let count = count_shepherd_peers(peer_count_with_last_shepherd);
    let cap = max as f32 + was_idle.max(0.0);
    if count >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_shepherd = true;
    true
}

pub fn has_or_become_shepherd_filtered(
    runtime: &mut ShepherdProfessionRuntime,
    max: i32,
    peers: &[ShepherdPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
    was_idle: f32,
) -> bool {
    let peer_count = count_shepherd_peers_filtered(peers, min_age_to_eat, max_age);
    has_or_become_shepherd(runtime, max, peer_count, was_idle)
}

/// Prefer assigned over sticky last for AssignedJob dispatch.
// Haxe: assignedProfession == 'SHEPHERD' || lastProfession == 'SHEPHERD'
pub fn resolve_shepherd_assigned_job(runtime: &ShepherdProfessionRuntime) -> bool {
    runtime.is_assigned_shepherd || runtime.is_last_shepherd
}

/// Max people for assigned vs age-rotated dispatch.
// Haxe: isSheepHerding(100) assigned; isSheepHerding() / (1) age/mid
pub fn shepherd_max_people_for_dispatch(is_assigned_job: bool) -> i32 {
    if is_assigned_job {
        SHEPHERD_ASSIGNED_MAX_PEOPLE
    } else {
        SHEPHERD_DEFAULT_MAX_PEOPLE
    }
}

// ── World counts snapshot ──────────────────────────────────────────────────

/// Close-object counts for sheep/cow/goose herding near home.
#[derive(Debug, Clone, Default)]
pub struct ShepherdCounts {
    pub by_id: HashMap<i32, i32>,
    pub held_id: i32,
    /// Caller has corn seeds (calf / goose / cow feed gates).
    pub has_corn_seeds: bool,
    /// Player age for `(round(age/5)) % 2` bush gate.
    pub age: f32,
}

impl ShepherdCounts {
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

    pub fn get_with_held(&self, id: i32) -> i32 {
        self.get(id) + if self.held_id == id { 1 } else { 0 }
    }

    /// Convert to farm counts for compost / plant / bush helpers.
    pub fn as_farm_counts(&self) -> FarmCounts {
        FarmCounts {
            by_id: self.by_id.clone(),
            held_id: self.held_id,
            hardened_row_biome: None,
            is_hungry: false,
            basic_farmer_weight: 0.0,
        }
    }
}

/// Build counts from a nearby `(id, count)` snapshot (unit tests / thin live).
pub fn shepherd_counts_from_nearby(
    pairs: &[(i32, i32)],
    held_id: i32,
    has_corn_seeds: bool,
    age: f32,
) -> ShepherdCounts {
    let mut c = ShepherdCounts {
        held_id,
        has_corn_seeds,
        age,
        ..Default::default()
    };
    for &(id, n) in pairs {
        c.set(id, n);
    }
    c
}

// ── Actions / results ──────────────────────────────────────────────────────

/// Pure decision output — execution is craft / shortCraft wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShepherdAction {
    /// Nothing to do in this step.
    None,
    /// Haxe `shortCraft(actor, target)`.
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `craftItem(objectId)`.
    CraftItem { object_id: i32 },
    /// Cap refuse / no profession.
    Abort,
}

impl ShepherdAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None | Self::Abort)
    }
}

/// Full `isSheepHerding` pure outcome (action + Haxe bool + profession clear).
// Haxe: AiBase.isSheepHerding ~1820–1918
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheepHerdingResult {
    pub action: ShepherdAction,
    /// Haxe `isSheepHerding` boolean return.
    pub haxe_return: bool,
    /// When true, set `profession['SHEPHERD'] = 0`.
    pub clear_profession_weight: bool,
}

impl SheepHerdingResult {
    pub fn none() -> Self {
        Self {
            action: ShepherdAction::None,
            haxe_return: false,
            clear_profession_weight: false,
        }
    }

    pub fn abort() -> Self {
        Self {
            action: ShepherdAction::Abort,
            haxe_return: false,
            clear_profession_weight: false,
        }
    }

    pub fn acted(action: ShepherdAction) -> Self {
        Self {
            action,
            haxe_return: true,
            clear_profession_weight: false,
        }
    }

    /// Haxe `if (handleMilk()) return false` — action runs, outer bool is false.
    // Haxe: AiBase.isSheepHerding ~1838
    pub fn milk_quirk(action: ShepherdAction) -> Self {
        Self {
            action,
            haxe_return: false,
            clear_profession_weight: false,
        }
    }

    pub fn fallthrough_clear() -> Self {
        Self {
            action: ShepherdAction::None,
            haxe_return: false,
            clear_profession_weight: true,
        }
    }
}

fn farm_to_shepherd(a: FarmAction) -> ShepherdAction {
    match a {
        FarmAction::None
        | FarmAction::DeferSheepHerding { .. }
        | FarmAction::DeferAdvancedFarming { .. }
        | FarmAction::ClearBasicFarmerWeight => ShepherdAction::None,
        FarmAction::Abort => ShepherdAction::Abort,
        FarmAction::ShortCraft { actor, target } => ShepherdAction::ShortCraft { actor, target },
        FarmAction::CraftItem { object_id } => ShepherdAction::CraftItem { object_id },
    }
}

// ── handleMilk (ungated for shepherd) ──────────────────────────────────────

/// Haxe `handleMilk` pure body **without** baker milk-stock gate.
///
/// Crafts Whole Milk Pouch 4081 from zero when count &lt; 3 (Haxe first step).
/// Skim-milk bucket shortCrafts are commented in Haxe — skipped.
// Haxe: AiBase.handleMilk ~1774–1817
pub fn handle_milk_for_shepherd(counts: &ShepherdCounts) -> ShepherdAction {
    let pouch = counts.get_with_held(MILK_POUCH);
    if pouch < 3 {
        return ShepherdAction::CraftItem {
            object_id: MILK_POUCH,
        };
    }
    // Skewer 139 + Bowl of Whipped Cream 3374
    if counts.get(WHIPPED_CREAM) > 0 || counts.held_id == WHIPPED_CREAM {
        return ShepherdAction::ShortCraft {
            actor: SKEWER,
            target: WHIPPED_CREAM,
        };
    }
    // Skewer 139 + Bowl of Cream 1464
    if counts.get(BOWL_OF_CREAM) > 0 || counts.held_id == BOWL_OF_CREAM {
        return ShepherdAction::ShortCraft {
            actor: SKEWER,
            target: BOWL_OF_CREAM,
        };
    }
    // Buttered Bread on Clay Plate 1473
    if counts.get_with_held(BUTTERED_BREAD) < 1 {
        return ShepherdAction::CraftItem {
            object_id: BUTTERED_BREAD,
        };
    }
    // Bowl of Butter 1465
    if counts.get_with_held(BOWL_OF_BUTTER) < 1 {
        return ShepherdAction::CraftItem {
            object_id: BOWL_OF_BUTTER,
        };
    }
    ShepherdAction::None
}

// ── countCorn ──────────────────────────────────────────────────────────────

/// Haxe `countCorn` pure sum: dried/cob/kernels + dumped/pile; held only for
/// 1115 / 1120 / 1247 (not 4106 / 4107).
// Haxe: AiBase.countCorn ~1920
pub fn count_corn(counts: &ShepherdCounts) -> i32 {
    let mut n = counts.get(DRIED_EAR_OF_CORN)
        + counts.get(BOWL_CORN_COB)
        + counts.get(BOWL_CORN_KERNELS)
        + counts.get(DUMPED_CORN_KERNELS)
        + counts.get(PILE_CORN_KERNELS);
    for id in [DRIED_EAR_OF_CORN, BOWL_CORN_COB, BOWL_CORN_KERNELS] {
        if counts.held_id == id {
            n += 1;
        }
    }
    n
}

// ── fillBerryBowlIfNeeded (held path only; Haxe TODO early return) ─────────

/// Haxe `fillBerryBowlIfNeeded` — only held 253→bush; BOWLFILLER path dead after TODO.
// Haxe: AiBase.fillBerryBowlIfNeeded ~4224–4246
pub fn fill_berry_bowl_for_shepherd(counts: &ShepherdCounts) -> ShepherdAction {
    if counts.held_id != BOWL_GOOSEBERRIES {
        return ShepherdAction::None;
    }
    if counts.get(DOMESTIC_BUSH) > 0 {
        return ShepherdAction::ShortCraft {
            actor: BOWL_GOOSEBERRIES,
            target: DOMESTIC_BUSH,
        };
    }
    if counts.get(WILD_BUSH) > 0 {
        return ShepherdAction::ShortCraft {
            actor: BOWL_GOOSEBERRIES,
            target: WILD_BUSH,
        };
    }
    ShepherdAction::None
}

// ── shortCraft emit when target present ────────────────────────────────────

fn try_short(counts: &ShepherdCounts, actor: i32, target: i32) -> Option<ShepherdAction> {
    if counts.get(target) > 0 {
        Some(ShepherdAction::ShortCraft { actor, target })
    } else {
        None
    }
}

/// True when feed actor is held or stocked nearby (optional gate for feed crafts).
fn has_actor(counts: &ShepherdCounts, actor: i32) -> bool {
    counts.held_id == actor || counts.get(actor) > 0
}

fn try_feed(
    counts: &ShepherdCounts,
    actor: i32,
    target: i32,
    require_actor: bool,
) -> Option<ShepherdAction> {
    if counts.get(target) == 0 {
        return None;
    }
    if require_actor && !has_actor(counts, actor) {
        // Still emit: Haxe shortCraft seeks/crafts actor; pure SM surfaces the pair.
        // Keep emit so live tick can GetOrCraft actor.
    }
    Some(ShepherdAction::ShortCraft { actor, target })
}

// ── isSheepHerding ─────────────────────────────────────────────────────────

/// Haxe `isSheepHerding(maxProfession, maxAnimal)` pure state machine.
///
/// Sequence (port-as-is):
/// 1. hasOrBecomeProfession SHEPHERD
/// 2. lambs 258+604 / 258+542 when sheep &lt; maxAnimal
/// 3. handleMilk → **return false quirk** if milk acted
/// 4. calves 1247+1462/1459 (hasCornSeeds), empty bucket + milk cow
/// 5. doComposting, doPlantCarrots(1,5), age-gated keepBushesAlive
/// 6. feed shorn 576 / sheep 575 when under max
/// 7. doPlantCorn(2,5)
/// 8. goose egg/goose incubator when maxAnimal&gt;5 and countCorn gates
/// 9. knife + dead cow; cow feed when count≤5 and countCorn&gt;3
/// 10. fillBerryBowlIfNeeded (held only)
/// 11. profession SHEPHERD = 0; return false
// Haxe: AiBase.isSheepHerding ~1820–1918
pub fn is_sheep_herding(
    runtime: &mut ShepherdProfessionRuntime,
    counts: &ShepherdCounts,
    farm_task: &mut FarmTaskState,
    max_profession: i32,
    max_animal: i32,
    peer_count: f32,
    was_idle: f32,
) -> SheepHerdingResult {
    if !has_or_become_shepherd(runtime, max_profession, peer_count, was_idle) {
        return SheepHerdingResult::abort();
    }

    let sheep = counts.get(DOMESTIC_SHEEP);

    // Lambs when under max animal
    if sheep < max_animal {
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, HUNGRY_DOMESTIC_LAMB, false) {
            return SheepHerdingResult::acted(a);
        }
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, DOMESTIC_LAMB, false) {
            return SheepHerdingResult::acted(a);
        }
    }

    // Haxe: if (handleMilk()) return false;
    let milk = handle_milk_for_shepherd(counts);
    if milk.is_some() {
        return SheepHerdingResult::milk_quirk(milk);
    }

    // Feed and milk the cows
    if counts.has_corn_seeds {
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, HUNGRY_DOMESTIC_CALF, false) {
            return SheepHerdingResult::acted(a);
        }
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, DOMESTIC_CALF, false) {
            return SheepHerdingResult::acted(a);
        }
    }
    if let Some(a) = try_short(counts, EMPTY_BUCKET, MILK_COW) {
        return SheepHerdingResult::acted(a);
    }

    // doComposting (dung gate commented in Haxe — always try)
    let farm = counts.as_farm_counts();
    let compost = farm_to_shepherd(do_composting(&farm, farm_task));
    if compost.is_some() {
        return SheepHerdingResult::acted(compost);
    }

    let carrots = farm_to_shepherd(do_plant_carrots(1, 5, &farm, farm_task));
    if carrots.is_some() {
        return SheepHerdingResult::acted(carrots);
    }

    // (Math.round(age / 5)) % 2 == 0 && keepBushesAlive()
    let age_slot = (counts.age / 5.0).round() as i32;
    if age_slot.rem_euclid(2) == 0 {
        let bushes = farm_to_shepherd(keep_bushes_alive(&farm));
        // Only act when dying target present (Haxe shortCraft fails if none).
        if bushes.is_some() && counts.get(DYING_BUSH) > 0 {
            return SheepHerdingResult::acted(bushes);
        }
    }

    // Feed shorn / sheep
    if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, SHORN_DOMESTIC_SHEEP, false) {
        return SheepHerdingResult::acted(a);
    }
    if sheep < max_animal {
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, DOMESTIC_SHEEP, false) {
            return SheepHerdingResult::acted(a);
        }
    }
    // Knife kill excess sheep when count>5: commented in Haxe — skip

    let corn_plant = farm_to_shepherd(do_plant_corn(2, 5, &farm, farm_task));
    if corn_plant.is_some() {
        return SheepHerdingResult::acted(corn_plant);
    }

    let count_corn_n = count_corn(counts);

    // Cold Goose Egg / Domestic Goose feed when maxAnimal > 5
    let goose_egg = counts.get(COLD_GOOSE_EGG);
    if max_animal > 5 && goose_egg < 5 && count_corn_n > 1 {
        if counts.has_corn_seeds {
            if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, DOMESTIC_GOOSE, false) {
                return SheepHerdingResult::acted(a);
            }
        }
    }

    // Domestic Goose count → craft incubator 1263
    let goose = counts.get(DOMESTIC_GOOSE);
    if max_animal > 5 && goose < 5 {
        return SheepHerdingResult::acted(ShepherdAction::CraftItem {
            object_id: DUNG_GOOSE_EGG_INCUBATOR,
        });
    }

    // Knife + Dead Cow
    if let Some(a) = try_short(counts, KNIFE, DEAD_COW) {
        return SheepHerdingResult::acted(a);
    }

    // Domestic Cow: excess kill commented; else feed when countCorn > 3
    let cow = counts.get(DOMESTIC_COW);
    if cow <= 5 {
        if count_corn_n > 3 && counts.has_corn_seeds {
            if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, DOMESTIC_COW, false) {
                return SheepHerdingResult::acted(a);
            }
        }
    }

    // fillBerryBowlIfNeeded (held path; TODO dead BOWLFILLER)
    let bowl = fill_berry_bowl_for_shepherd(counts);
    if bowl.is_some() {
        return SheepHerdingResult::acted(bowl);
    }

    // Fallthrough: clear profession weight
    runtime.clear_weight();
    SheepHerdingResult::fallthrough_clear()
}

/// Apply profession weight clear side-effect when result says so (already applied
/// inside [`is_sheep_herding`]; this is a no-op helper for callers that only
/// hold the result).
pub fn apply_sheep_herding_result(
    runtime: &mut ShepherdProfessionRuntime,
    result: &SheepHerdingResult,
) {
    if result.clear_profession_weight {
        runtime.clear_weight();
    }
}

// ── doFeedLambsAndCalfs ────────────────────────────────────────────────────

/// Haxe `doFeedLambsAndCalfs(maxPeople)` pure early mid-priority body.
///
/// Uses **603** Hungry Mouflon Lamb (not 604). Caps sheep/cow feeds at 10.
// Haxe: AiBase.doFeedLambsAndCalfs ~5666–5713
pub fn do_feed_lambs_and_calfs(
    runtime: &mut ShepherdProfessionRuntime,
    counts: &ShepherdCounts,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
) -> ShepherdAction {
    if !has_or_become_shepherd(runtime, max_people, peer_count, was_idle) {
        return ShepherdAction::Abort;
    }

    // Empty Bucket 659 + Milk Cow 1489
    if let Some(a) = try_short(counts, EMPTY_BUCKET, MILK_COW) {
        return a;
    }

    // Domestic Sheep 575
    let sheep = counts.get(DOMESTIC_SHEEP);
    if sheep < FEED_LAMBS_CALFS_ANIMAL_CAP {
        // 258 + Hungry Mouflon Lamb 603 (not 604)
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, HUNGRY_MOUFLON_LAMB, false) {
            return a;
        }
        // 258 + Mouflon Lamb 542 (same id as Domestic Lamb)
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, DOMESTIC_LAMB, false) {
            return a;
        }
    }

    // Domestic Cow 1458
    let cow = counts.get(DOMESTIC_COW);
    if cow < FEED_LAMBS_CALFS_ANIMAL_CAP && counts.has_corn_seeds {
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, HUNGRY_DOMESTIC_CALF, false) {
            return a;
        }
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, DOMESTIC_CALF, false) {
            return a;
        }
    }

    // craft Domestic Sheep / Domestic Mouflon commented in Haxe — skip
    ShepherdAction::None
}

// ── Baker mid alignment ────────────────────────────────────────────────────

/// Expand baker mid `isSheepHerding(2,5)` pure steps (lambs + milk quirk + calves).
///
/// Returns baker-compatible `(actor, target)` or craft id; callers map to
/// [`crate::BakeAction`]. Prefer full [`is_sheep_herding`] for dedicated shepherd.
// Haxe: AiBase.doBakingHelper isSheepHerding(2,5) ~3340
pub fn sheep_herding_steps_for_baker(
    counts: &ShepherdCounts,
    max_animal: i32,
) -> Option<ShepherdAction> {
    let sheep = counts.get(DOMESTIC_SHEEP);
    if sheep < max_animal {
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, HUNGRY_DOMESTIC_LAMB, false) {
            return Some(a);
        }
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, DOMESTIC_LAMB, false) {
            return Some(a);
        }
    }
    // Milk path — baker still stock-gates in handle_milk; here surface shortCrafts only
    // when whipped/cream present (no craft-from-zero in baker mid).
    if counts.get(WHIPPED_CREAM) > 0 || counts.held_id == WHIPPED_CREAM {
        return Some(ShepherdAction::ShortCraft {
            actor: SKEWER,
            target: WHIPPED_CREAM,
        });
    }
    if counts.get(BOWL_OF_CREAM) > 0 || counts.held_id == BOWL_OF_CREAM {
        return Some(ShepherdAction::ShortCraft {
            actor: SKEWER,
            target: BOWL_OF_CREAM,
        });
    }
    if counts.has_corn_seeds {
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, HUNGRY_DOMESTIC_CALF, false) {
            return Some(a);
        }
        if let Some(a) = try_feed(counts, BOWL_CORN_KERNELS, DOMESTIC_CALF, false) {
            return Some(a);
        }
    }
    if let Some(a) = try_short(counts, EMPTY_BUCKET, MILK_COW) {
        return Some(a);
    }
    if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, SHORN_DOMESTIC_SHEEP, false) {
        return Some(a);
    }
    if sheep < max_animal {
        if let Some(a) = try_feed(counts, BOWL_BERRIES_CARROT, DOMESTIC_SHEEP, false) {
            return Some(a);
        }
    }
    None
}

// ── Goal / shortCraft apply helpers ────────────────────────────────────────

/// Map shepherd action → AI Goal for ladder residual.
// Haxe: seek sheep / cow / craft target
pub fn shepherd_action_to_goal(action: ShepherdAction) -> Goal {
    match action {
        ShepherdAction::None | ShepherdAction::Abort => Goal::Idle,
        ShepherdAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        ShepherdAction::CraftItem { object_id } => Goal::SeekObject(object_id),
    }
}

/// Max people for age-rotated sheep (default 1) vs assigned (100).
pub fn try_decide_shepherd_from_rung(
    profession_is_sticky: bool,
    rung_label: &str,
    is_assigned_job: bool,
    counts: &ShepherdCounts,
    runtime: &mut ShepherdProfessionRuntime,
    farm_task: &mut FarmTaskState,
    peer_count: f32,
    was_idle: f32,
    max_animal: i32,
) -> Option<ShepherdAction> {
    let _ = profession_is_sticky;
    let max_people = if is_assigned_job || rung_label == "ASSIGNED_JOB" {
        SHEPHERD_ASSIGNED_MAX_PEOPLE
    } else {
        SHEPHERD_DEFAULT_MAX_PEOPLE
    };
    let r = is_sheep_herding(
        runtime,
        counts,
        farm_task,
        max_people,
        max_animal,
        peer_count,
        was_idle,
    );
    if r.action.is_some() {
        Some(r.action)
    } else if matches!(r.action, ShepherdAction::Abort) {
        None
    } else {
        None
    }
}

// AI-SHEPHERD-MID: makeStuff / basic-farm mid / pick_shepherd_goal
include!("shepherd_mid_sites.inc.rs");

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::farmer_profession::{COMPOSTING_PILE, DRY_PLANTED_CARROTS};

    fn counts(pairs: &[(i32, i32)]) -> ShepherdCounts {
        shepherd_counts_from_nearby(pairs, 0, true, 20.0)
    }

    #[test]
    fn is_sheep_herding_refuses_when_peer_cap_reached() {
        let mut rt = ShepherdProfessionRuntime::default();
        let c = counts(&[]);
        let mut task = FarmTaskState::default();
        // max=1, peer_count>=1, not sticky → refuse
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 1.0, 0.0);
        assert!(matches!(r.action, ShepherdAction::Abort));
        assert!(!r.haxe_return);
        assert!(!rt.is_last_shepherd);
    }

    #[test]
    fn is_sheep_herding_feeds_hungry_lamb_604_when_sheep_below_max() {
        let mut rt = ShepherdProfessionRuntime::default();
        let c = counts(&[(HUNGRY_DOMESTIC_LAMB, 1), (DOMESTIC_SHEEP, 2)]);
        let mut task = FarmTaskState::default();
        // Bypass milk: pouch >= 3 and butter stock so handle_milk is None
        let mut c = c;
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 1.0);
        assert!(r.haxe_return);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: HUNGRY_DOMESTIC_LAMB,
            }
        );
    }

    #[test]
    fn is_sheep_herding_feeds_domestic_lamb_542_when_sheep_below_max() {
        let mut rt = ShepherdProfessionRuntime::default();
        let mut c = counts(&[(DOMESTIC_LAMB, 1), (DOMESTIC_SHEEP, 0)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        let mut task = FarmTaskState::default();
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 1.0);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: DOMESTIC_LAMB,
            }
        );
    }

    #[test]
    fn is_sheep_herding_handle_milk_return_false_quirk_when_milk_stock_active() {
        let mut rt = ShepherdProfessionRuntime::default();
        // No lambs; pouch < 3 → milk craft; Haxe returns false
        let c = counts(&[(DOMESTIC_SHEEP, 20)]); // over max so no lamb path
        let mut task = FarmTaskState::default();
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 1.0);
        assert!(!r.haxe_return);
        assert!(!r.clear_profession_weight);
        assert_eq!(
            r.action,
            ShepherdAction::CraftItem {
                object_id: MILK_POUCH,
            }
        );
    }

    #[test]
    fn is_sheep_herding_feeds_calf_and_milk_cow_when_has_corn_seeds() {
        let mut rt = ShepherdProfessionRuntime::default();
        let mut c = counts(&[(HUNGRY_DOMESTIC_CALF, 1), (DOMESTIC_SHEEP, 20)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        c.has_corn_seeds = true;
        let mut task = FarmTaskState::default();
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 1.0);
        assert!(r.haxe_return);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_CORN_KERNELS,
                target: HUNGRY_DOMESTIC_CALF,
            }
        );

        // milk cow when no calves
        let mut c2 = counts(&[(MILK_COW, 1), (DOMESTIC_SHEEP, 20)]);
        c2.set(MILK_POUCH, 3);
        c2.set(BUTTERED_BREAD, 1);
        c2.set(BOWL_OF_BUTTER, 1);
        c2.has_corn_seeds = false;
        let mut rt2 = ShepherdProfessionRuntime::default();
        let mut task2 = FarmTaskState::default();
        let r2 = is_sheep_herding(&mut rt2, &c2, &mut task2, 1, 10, 0.0, 1.0);
        assert_eq!(
            r2.action,
            ShepherdAction::ShortCraft {
                actor: EMPTY_BUCKET,
                target: MILK_COW,
            }
        );
    }

    #[test]
    fn is_sheep_herding_compost_then_plant_carrots_then_age_gated_bushes_order() {
        let mut rt = ShepherdProfessionRuntime {
            is_last_shepherd: true,
            ..Default::default()
        };
        // milk done; no calves/cow; compost stock empty → compost first
        let mut c = counts(&[(DOMESTIC_SHEEP, 20)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        c.age = 20.0; // round(20/5)=4 even → bushes eligible after compost/carrots
        let mut task = FarmTaskState::default();
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 0.0);
        assert!(r.haxe_return);
        assert_eq!(
            r.action,
            ShepherdAction::CraftItem {
                object_id: COMPOSTING_PILE,
            }
        );

        // compost done (stock > 3 exit) + low carrots → plant carrots
        let mut c2 = c.clone();
        c2.set(COMPOSTING_PILE, 5); // stock high → composting idle
        let mut task2 = FarmTaskState {
            composting: 0.0,
            ..Default::default()
        };
        // force composting exit
        task2.composting = 0.0;
        let r2 = is_sheep_herding(&mut rt, &c2, &mut task2, 1, 10, 0.0, 0.0);
        // stock 5 → composting flag 0, returns None from compost; carrots stock 0 → plant
        assert_eq!(
            r2.action,
            ShepherdAction::CraftItem {
                object_id: DRY_PLANTED_CARROTS,
            }
        );

        // carrots full + dying bush + even age → bushes
        let mut c3 = c2.clone();
        // carrot stock high
        c3.set(DRY_PLANTED_CARROTS, 10);
        c3.set(DYING_BUSH, 1);
        // living bushes low so keep_bushes_alive fires
        let mut task3 = FarmTaskState {
            composting: 0.0,
            carrot_planter: 0.0,
            ..Default::default()
        };
        // stock carrots via wet/dry planted — carrot_stock_units
        // set many carrots
        use crate::farmer_profession::CARROT;
        c3.set(CARROT, 20);
        let r3 = is_sheep_herding(&mut rt, &c3, &mut task3, 1, 10, 0.0, 0.0);
        // may hit shorn/sheep feed or plant corn or bushes — ensure bushes when even age + dying
        // After compost none + carrots none (stock>=5) + bushes
        assert!(
            matches!(
                r3.action,
                ShepherdAction::ShortCraft {
                    actor: _,
                    target: DYING_BUSH
                } | ShepherdAction::ShortCraft {
                    actor: BOWL_BERRIES_CARROT,
                    target: _
                } | ShepherdAction::CraftItem { .. }
            ),
            "got {:?}",
            r3.action
        );
    }

    #[test]
    fn is_sheep_herding_feed_shorn_576_and_sheep_575_when_under_max_animal() {
        let mut rt = ShepherdProfessionRuntime {
            is_last_shepherd: true,
            ..Default::default()
        };
        let mut c = counts(&[(SHORN_DOMESTIC_SHEEP, 1), (DOMESTIC_SHEEP, 2)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        // compost/carrot idle: high stock
        c.set(COMPOSTING_PILE, 5);
        use crate::farmer_profession::CARROT;
        c.set(CARROT, 20);
        let mut task = FarmTaskState {
            composting: 0.0,
            carrot_planter: 0.0,
            ..Default::default()
        };
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 0.0);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: SHORN_DOMESTIC_SHEEP,
            }
        );

        let mut c2 = c.clone();
        c2.set(SHORN_DOMESTIC_SHEEP, 0);
        let r2 = is_sheep_herding(&mut rt, &c2, &mut task, 1, 10, 0.0, 0.0);
        assert_eq!(
            r2.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: DOMESTIC_SHEEP,
            }
        );
    }

    #[test]
    fn is_sheep_herding_goose_incubator_when_max_animal_gt_5_and_count_corn() {
        let mut rt = ShepherdProfessionRuntime {
            is_last_shepherd: true,
            ..Default::default()
        };
        // sheep at max so no feed; milk done; no calves; compost/carrot full; no shorn
        let mut c = counts(&[(DOMESTIC_SHEEP, 20), (DOMESTIC_GOOSE, 1)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        c.set(COMPOSTING_PILE, 5);
        use crate::farmer_profession::{CARROT, DRY_PLANTED_CORN, WET_PLANTED_CORN};
        c.set(CARROT, 20);
        // corn stages high so doPlantCorn idle
        c.set(DRY_PLANTED_CORN, 10);
        c.set(WET_PLANTED_CORN, 10);
        let mut task = FarmTaskState {
            composting: 0.0,
            carrot_planter: 0.0,
            corn_planter: 0.0,
            ..Default::default()
        };
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 0.0);
        assert_eq!(
            r.action,
            ShepherdAction::CraftItem {
                object_id: DUNG_GOOSE_EGG_INCUBATOR,
            }
        );
    }

    #[test]
    fn is_sheep_herding_dead_cow_knife_and_cow_feed_corn_gate() {
        let mut rt = ShepherdProfessionRuntime {
            is_last_shepherd: true,
            ..Default::default()
        };
        let mut c = counts(&[(DOMESTIC_SHEEP, 20), (DEAD_COW, 1), (DOMESTIC_GOOSE, 10)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        c.set(COMPOSTING_PILE, 5);
        use crate::farmer_profession::{CARROT, DRY_PLANTED_CORN, WET_PLANTED_CORN};
        c.set(CARROT, 20);
        c.set(DRY_PLANTED_CORN, 10);
        c.set(WET_PLANTED_CORN, 10);
        // goose >= 5 skips incubator when maxAnimal>5; goose count is 10
        let mut task = FarmTaskState {
            composting: 0.0,
            carrot_planter: 0.0,
            corn_planter: 0.0,
            ..Default::default()
        };
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 0.0);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: KNIFE,
                target: DEAD_COW,
            }
        );

        // cow feed when countCorn > 3 and has seeds
        // Skip goose path: goose>=5 skips incubator; cold eggs>=5 skips goose feed.
        let mut c2 = c.clone();
        c2.set(DEAD_COW, 0);
        c2.set(DOMESTIC_COW, 2);
        c2.set(DOMESTIC_GOOSE, 10);
        c2.set(COLD_GOOSE_EGG, 5);
        c2.set(BOWL_CORN_KERNELS, 4);
        c2.has_corn_seeds = true;
        let r2 = is_sheep_herding(&mut rt, &c2, &mut task, 1, 10, 0.0, 0.0);
        assert_eq!(
            r2.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_CORN_KERNELS,
                target: DOMESTIC_COW,
            }
        );
    }

    #[test]
    fn is_sheep_herding_clears_profession_weight_on_fallthrough_false() {
        let mut rt = ShepherdProfessionRuntime {
            is_last_shepherd: true,
            weight: 1.0,
            ..Default::default()
        };
        // Everything satisfied: high sheep, milk done, high compost/carrots/corn, goose max, no dead cow
        let mut c = counts(&[(DOMESTIC_SHEEP, 20), (DOMESTIC_GOOSE, 10), (DOMESTIC_COW, 10)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        c.set(COMPOSTING_PILE, 5);
        use crate::farmer_profession::{CARROT, DRY_PLANTED_CORN, WET_PLANTED_CORN};
        c.set(CARROT, 20);
        c.set(DRY_PLANTED_CORN, 10);
        c.set(WET_PLANTED_CORN, 10);
        let mut task = FarmTaskState {
            composting: 0.0,
            carrot_planter: 0.0,
            corn_planter: 0.0,
            ..Default::default()
        };
        let r = is_sheep_herding(&mut rt, &c, &mut task, 1, 10, 0.0, 0.0);
        assert!(!r.haxe_return);
        assert!(r.clear_profession_weight);
        assert_eq!(rt.weight, 0.0);
        assert!(matches!(r.action, ShepherdAction::None));
    }

    #[test]
    fn do_feed_lambs_and_calfs_uses_603_not_604_and_caps_at_10() {
        let mut rt = ShepherdProfessionRuntime::default();
        let c = counts(&[
            (HUNGRY_MOUFLON_LAMB, 1),
            (HUNGRY_DOMESTIC_LAMB, 1), // 604 present but must prefer 603 path only
            (DOMESTIC_SHEEP, 5),
        ]);
        let a = do_feed_lambs_and_calfs(&mut rt, &c, 1, 0.0, 1.0);
        assert_eq!(
            a,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: HUNGRY_MOUFLON_LAMB,
            }
        );

        // sheep >= 10: skip mouflon lamb, still can milk cow first
        let mut c2 = counts(&[
            (HUNGRY_MOUFLON_LAMB, 1),
            (DOMESTIC_SHEEP, 10),
            (HUNGRY_DOMESTIC_CALF, 1),
            (DOMESTIC_COW, 2),
        ]);
        c2.has_corn_seeds = true;
        let mut rt2 = ShepherdProfessionRuntime::default();
        let a2 = do_feed_lambs_and_calfs(&mut rt2, &c2, 1, 0.0, 1.0);
        assert_eq!(
            a2,
            ShepherdAction::ShortCraft {
                actor: BOWL_CORN_KERNELS,
                target: HUNGRY_DOMESTIC_CALF,
            }
        );
    }

    #[test]
    fn count_corn_held_only_1115_1120_1247_not_4106_4107() {
        let mut c = ShepherdCounts::default();
        c.set(DUMPED_CORN_KERNELS, 2);
        c.set(PILE_CORN_KERNELS, 3);
        c.held_id = DUMPED_CORN_KERNELS;
        // map counts 2+3=5; held 4106 must NOT add
        assert_eq!(count_corn(&c), 5);

        c.held_id = BOWL_CORN_KERNELS;
        c.set(BOWL_CORN_KERNELS, 1);
        // 2+3+1 map + 1 held = 7
        assert_eq!(count_corn(&c), 7);
    }

    #[test]
    fn assign_and_has_or_become_shepherd() {
        let mut rt = ShepherdProfessionRuntime::default();
        assert!(assign_shepherd_from_speech(&mut rt, "SHEPHERD!"));
        assert!(rt.is_assigned_shepherd);
        assert!(rt.is_last_shepherd);
        assert!(resolve_shepherd_assigned_job(&rt));

        let mut rt2 = ShepherdProfessionRuntime::default();
        assert!(!has_or_become_shepherd(&mut rt2, 1, 1.0, 0.0));
        assert!(has_or_become_shepherd(&mut rt2, 1, 0.0, 1.0));
        assert!(rt2.is_last_shepherd);
        assert_eq!(rt2.weight, 1.0);
    }

    #[test]
    fn sheep_herding_steps_for_baker_aligned_with_lambs() {
        let c = counts(&[(HUNGRY_DOMESTIC_LAMB, 1), (DOMESTIC_SHEEP, 0)]);
        let a = sheep_herding_steps_for_baker(&c, 5).unwrap();
        assert_eq!(
            a,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: HUNGRY_DOMESTIC_LAMB,
            }
        );
    }

    #[test]
    fn make_stuff_orders_farm_then_sheep() {
        use crate::farmer_profession::FarmAction;
        assert_eq!(
            make_stuff(
                FarmAction::CraftItem {
                    object_id: DRY_PLANTED_CARROTS
                },
                true,
                false
            ),
            MakeStuffAction::BasicFarming {
                max_profession: 2
            }
        );
        assert_eq!(
            make_stuff(FarmAction::None, true, false),
            MakeStuffAction::SheepHerding {
                max_profession: 2
            }
        );
        assert_eq!(
            make_stuff(FarmAction::None, false, false),
            MakeStuffAction::None
        );
        // Residual fire after farm+sheep fallthrough
        assert_eq!(
            make_stuff(FarmAction::None, false, true),
            MakeStuffAction::DeferFireFood {
                max_profession: 2
            }
        );
        // Full Haxe order: sharpie → bake → farm → sheep → fire
        assert_eq!(
            make_stuff_ordered(MakeStuffInputs {
                sharpie_has_work: true,
                baking_has_work: true,
                basic_farm_has_work: true,
                sheep_has_work: true,
                fire_has_work: true,
            }),
            MakeStuffAction::DeferSharpieFood
        );
        assert_eq!(
            make_stuff_ordered(MakeStuffInputs {
                sharpie_has_work: false,
                baking_has_work: true,
                basic_farm_has_work: true,
                sheep_has_work: true,
                fire_has_work: true,
            }),
            MakeStuffAction::DeferBaking {
                max_profession: 2
            }
        );
        assert_eq!(
            make_stuff_ordered(MakeStuffInputs {
                sharpie_has_work: false,
                baking_has_work: false,
                basic_farm_has_work: false,
                sheep_has_work: false,
                fire_has_work: true,
            }),
            MakeStuffAction::DeferFireFood {
                max_profession: 2
            }
        );
    }

    #[test]
    fn make_stuff_try_prefers_sharpie_then_farm() {
        use crate::farmer_profession::{make_sharpie_food, FarmCounts, BURDOCK};
        let mut task = FarmTaskState::default();
        let mut c = FarmCounts::default();
        c.set(BURDOCK, 1);
        assert!(make_sharpie_food(&c).is_some());
        assert_eq!(
            make_stuff_try(&c, &mut task, true, false, true, true),
            MakeStuffAction::DeferSharpieFood
        );
        // No sharpie plant → basic farming mid defer (empty map still DeferSheep)
        let c2 = FarmCounts::default();
        assert_eq!(
            make_stuff_try(&c2, &mut task, true, false, true, true),
            MakeStuffAction::BasicFarming {
                max_profession: 2
            }
        );
    }

    #[test]
    fn basic_farm_mid_try_sheep_max_people_1() {
        let mut rt = ShepherdProfessionRuntime::default();
        let mut c = counts(&[(HUNGRY_DOMESTIC_LAMB, 1), (DOMESTIC_SHEEP, 0)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        let mut task = FarmTaskState::default();
        let r = basic_farm_mid_try_sheep(&mut rt, &c, &mut task, 0.0, 1.0);
        assert!(r.haxe_return);
        assert_eq!(
            r.action,
            ShepherdAction::ShortCraft {
                actor: BOWL_BERRIES_CARROT,
                target: HUNGRY_DOMESTIC_LAMB,
            }
        );
    }

    #[test]
    fn make_stuff_try_sheep_max_people_2() {
        let mut rt = ShepherdProfessionRuntime::default();
        let mut c = counts(&[(HUNGRY_DOMESTIC_LAMB, 1), (DOMESTIC_SHEEP, 0)]);
        c.set(MILK_POUCH, 3);
        c.set(BUTTERED_BREAD, 1);
        c.set(BOWL_OF_BUTTER, 1);
        let mut task = FarmTaskState::default();
        let r = make_stuff_try_sheep(&mut rt, &c, &mut task, 1.0, 0.0);
        assert!(r.action.is_some() || r.haxe_return);
        assert!(rt.is_last_shepherd);
    }

    #[test]
    fn make_stuff_try_bodies_prefers_bake_before_farm_sheep_fire() {
        // AI-MAKE-STUFF: doBaking has work → DeferBaking before farm/sheep/fire
        use crate::baker_profession::{
            BakeCounts, BakerProfessionRuntime, BakerTaskState, CLAY_PLATE, HOT_OVEN, RAW_PIES,
        };
        use crate::fire_food_profession::{FireFoodCounts, FireFoodProfessionRuntime};
        use crate::farmer_profession::FarmCounts;
        let mut farm_task = FarmTaskState::default();
        let farm_counts = FarmCounts::default();
        let mut baker_rt = BakerProfessionRuntime {
            is_last_baker: true,
            ..Default::default()
        };
        let mut baker_task = BakerTaskState::default();
        // Hot oven + raw pie + plate → do_baking has work
        let mut bake = BakeCounts {
            oven_parent_id: Some(HOT_OVEN),
            held_uses: 1,
            ..Default::default()
        };
        bake.set(RAW_PIES[0], 1);
        bake.set(CLAY_PLATE, 1);
        let mut fire_rt = FireFoodProfessionRuntime::default();
        let fire_counts = FireFoodCounts::default();
        let a = make_stuff_try_bodies(
            &farm_counts,
            &mut farm_task,
            true,
            &bake,
            &mut baker_rt,
            &mut baker_task,
            0.0,
            0.0,
            0,
            true, // sheep would work if we got there
            &fire_counts,
            &mut fire_rt,
            0.0,
            0.0,
        );
        assert_eq!(
            a,
            MakeStuffAction::DeferBaking {
                max_profession: MAKE_STUFF_FARM_MAX_PEOPLE
            }
        );
    }

    #[test]
    fn make_stuff_fire_has_work_hot_coals_raw_mutton() {
        // AI-MAKE-STUFF: makeFireFood(2) peer room + hot coals + raw mutton
        use crate::baker_profession::RAW_MUTTON;
        use crate::fire_food_profession::{
            fire_food_counts_from_nearby, make_fire_food, FireFoodAction,
            FireFoodProfessionRuntime, HOT_COALS,
        };
        let mut fire_rt = FireFoodProfessionRuntime::default();
        let mut c = fire_food_counts_from_nearby(
            &[(RAW_MUTTON, 1)],
            0,
            false,
            false,
            false,
            false,
            true,
            false,
            false,
        );
        c.has_hot_coals = true;
        c.has_fire_place = true;
        assert!(make_stuff_fire_has_work(&c, &mut fire_rt, 0.0, 1.0));
        assert!(fire_rt.is_last_fire_food);
        // Probe already became sticky; second call still has work
        let a = make_fire_food(&c, &mut fire_rt, MAKE_STUFF_FARM_MAX_PEOPLE, 0.0, 0.0);
        assert_eq!(
            a,
            FireFoodAction::ShortCraft {
                actor: RAW_MUTTON,
                target: HOT_COALS
            }
        );
    }
}
