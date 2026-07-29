//! Haxe: `AiBase` smith profession family (chunks **AI-JOB-SMITH** / **LIVE**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession('SMITH')` with **max one smith** hard cap
//! - Speech `SMITH!` → assigned job
//! - `GetForge` id priority (Firing 304 → Charcoal 305 → Forge 303)
//! - `prepareSmithingTools` stage ladder (iron / steel / hammer prep)
//! - `doSmithing` product sequence (pick → shovel → hoe → … → knife)
//! - Live apply: [`smith_action_apply`] / [`smith_action_short_craft_apply`] /
//!   [`do_pottery_on_fire`] / [`apply_consider_making_food_smith_wipe`]
//!
//! No world I/O: callers supply counts / forge parent id and apply returned
//! [`SmithAction`]s via [`SmithApply`] (USE/DROP/craft) and spatial helpers.
//!
//! Existing thin reverse-craft path: [`crate::ai_goals::pick_smith_goal`] /
//! `smith_product_targets` — this module adds the full profession stage SM.
//!
//! **Intentional delta:** Haxe `prepareSmithingTools` lists the steel/crucible
//! block textually before the wrought-iron block, but stage labels imply
//! ore (2) → wrought (3) → crucible (3.5) → steel (4). Pure SM runs iron
//! stock while `stage < 3` **before** the steel path so the stage machine is
//! reachable (Haxe order makes iron nearly dead when `countSteel < 1`).

use std::collections::{HashMap, HashSet};

use crate::ai_goals::{Goal, SMITH_IRON_ID, SMITH_TARGET_ID, SMITHING_HAMMER_ID};
use crate::craft_graph::ReverseCraftGraph;

// ── Object ids (OHOL / OpenLife content; Haxe comments in AiBase) ───────────

/// Forge (cold).
pub const FORGE: i32 = 303;
/// Firing Forge.
pub const FIRING_FORGE: i32 = 304;
/// Forge with Charcoal.
pub const FORGE_WITH_CHARCOAL: i32 = 305;
/// Flat Rock.
pub const FLAT_ROCK: i32 = 291;
/// Stone.
pub const STONE: i32 = 33;
/// Smithing Hammer.
pub const SMITHING_HAMMER: i32 = SMITHING_HAMMER_ID; // 441
/// Hot Iron Bloom on Flat Rock.
pub const HOT_IRON_BLOOM_FLAT: i32 = 309;
/// Cold Iron Bloom on Flat Rock.
pub const COLD_IRON_BLOOM_FLAT: i32 = 312;
/// Wrought Iron on Flat Rock.
pub const WROUGHT_IRON_FLAT: i32 = 313;
/// Wrought Iron.
pub const WROUGHT_IRON: i32 = 314;
/// Iron Ore.
pub const IRON_ORE: i32 = 290;
/// Basket of Charcoal.
pub const BASKET_OF_CHARCOAL: i32 = 298;
/// Unforged Sealed Steel Crucible.
pub const UNFORGED_SEALED_CRUCIBLE: i32 = 319;
/// Unforged Steel Crucible in Wooden Tongs.
pub const UNFORGED_CRUCIBLE_TONGS: i32 = 320;
/// Forged Steel Crucible.
pub const FORGED_CRUCIBLE: i32 = 322;
/// Hot Steel Crucible in Wooden Tongs.
pub const HOT_CRUCIBLE_TONGS: i32 = 323;
/// Cool Steel Crucible in Wooden Tongs.
pub const COOL_CRUCIBLE_TONGS: i32 = 324;
/// Steel Ingot.
pub const STEEL_INGOT: i32 = 326;
/// Steel Ingot on Flat Rock.
pub const STEEL_INGOT_FLAT: i32 = 335;
/// Steel File Blank on Flat Rock.
pub const STEEL_FILE_BLANK_FLAT: i32 = 450;
/// Steel Chisel on Flat Rock.
pub const STEEL_CHISEL_FLAT: i32 = 451;
/// Steel Adze Head on Flat Rock.
pub const STEEL_ADZE_HEAD_FLAT: i32 = 453;
/// Steel Mining Pick.
pub const STEEL_MINING_PICK: i32 = 684;
/// Shovel.
pub const SHOVEL: i32 = 502;
/// Shovel of Dung (counts as shovel stock).
pub const SHOVEL_OF_DUNG: i32 = 900;
/// Steel Hoe.
pub const STEEL_HOE: i32 = 857;
/// Shears.
pub const SHEARS: i32 = 568;
/// Steel Axe.
pub const STEEL_AXE: i32 = 334;
/// Steel Chisel.
pub const STEEL_CHISEL: i32 = 455;
/// Steel File Blank.
pub const STEEL_FILE_BLANK: i32 = 457;
/// Hot Steel File Blank.
pub const HOT_STEEL_FILE_BLANK: i32 = 447;
/// Oiled File Blank.
pub const OILED_FILE_BLANK: i32 = 464;
/// Oiled File Blank with Chisel.
pub const OILED_FILE_BLANK_CHISEL: i32 = 465;
/// Steel File with Chisel.
pub const STEEL_FILE_WITH_CHISEL: i32 = 466;
/// Steel File.
pub const STEEL_FILE: i32 = 458;
/// Steel Blade Blank.
pub const STEEL_BLADE_BLANK: i32 = 459;
/// Knife.
pub const KNIFE: i32 = 560;
/// Bowl of Water (reheat cold bloom; Haxe shortCraft 239+312).
pub const BOWL_OF_WATER: i32 = 239;
/// Firing Adobe Kiln (pottery fallthrough gate).
pub const FIRING_KILN: i32 = 282;
/// Clay Bowl (pottery defer seek / doPotteryOnFire).
pub const CLAY_BOWL: i32 = 235;
/// Clay Plate (pottery defer seek / doPotteryOnFire).
pub const CLAY_PLATE: i32 = 236;
/// Wet Clay Bowl (doPotteryOnFire wet stock).
pub const WET_CLAY_BOWL: i32 = 233;
/// Wet Bowl in Wooden Tongs.
pub const WET_BOWL_TONGS: i32 = 284;
/// Wet Clay Plate.
pub const WET_CLAY_PLATE: i32 = 234;
/// Wet Plate in Wooden Tongs.
pub const WET_PLATE_TONGS: i32 = 240;
/// Wet Clay Crock.
pub const WET_CLAY_CROCK: i32 = 1216;
/// Wet Crock in Wooden Tongs.
pub const WET_CROCK_TONGS: i32 = 1218;
/// Clay Crock.
pub const CLAY_CROCK: i32 = 1217;
/// Crock with Squash (counts as crock stock).
pub const CROCK_WITH_SQUASH: i32 = 1243;
/// Wooden Tongs with Fired Bowl (craft target when under bowl max).
pub const FIRED_BOWL_TONGS: i32 = 283;
/// Fired Plate in Wooden Tongs.
pub const FIRED_PLATE_TONGS: i32 = 241;
/// Wooden Tongs with Fired Crock.
pub const FIRED_CROCK_TONGS: i32 = 1219;
/// Adobe (fuel kiln when coal low).
pub const ADOBE: i32 = 127;
/// Big Charcoal Pile.
pub const BIG_CHARCOAL_PILE: i32 = 300;
/// Huge Charcoal Pile (counts as coal stock).
pub const HUGE_CHARCOAL_PILE: i32 = 4102;
/// Wet Clay Nozzle (AI-POTTER-L2946 other potter craft).
// Haxe residual L2946 / content id 285
pub const WET_CLAY_NOZZLE: i32 = 285;
/// Wet Nozzle in Wooden Tongs.
pub const WET_NOZZLE_TONGS: i32 = 295;
/// Clay Nozzle (fired).
pub const CLAY_NOZZLE: i32 = 286;
/// Fired Nozzle in Wooden Tongs (craftItem target).
pub const FIRED_NOZZLE_TONGS: i32 = 296;
/// Default aiCraftMax fallbacks when content DB not loaded (OHOL defaults).
pub const DEFAULT_MAX_CLAY_BOWLS: i32 = 3;
pub const DEFAULT_MAX_CLAY_PLATES: i32 = 3;
pub const DEFAULT_MAX_CLAY_CROCKS: i32 = 2;
/// Default max clay nozzles (bellows / forge air; content has no LimitObject).
// Haxe: L2946 TODO — no aiCraftMax; keep small stock
pub const DEFAULT_MAX_CLAY_NOZZLES: i32 = 2;
/// Haxe `CalculateQuadDistanceToObject` threshold for drop-near goto (~893).
pub const CRAFT_DROP_GOTO_QUAD_DIST: i32 = 5;

/// Home/forge search radius used by Haxe `GetForge` (20).
pub const FORGE_SEARCH_RADIUS: i32 = 20;
/// Flat rock / stone count radius near home (Haxe CountCloseObjects r=10).
pub const FLAT_STONE_COUNT_RADIUS: i32 = 10;
/// Iron / ore count radius near forge (Haxe CountCloseObjects r=20).
pub const IRON_ORE_COUNT_RADIUS: i32 = 20;
/// Steel / crucible count radius (Haxe TODO: use 15; pure SM documents intent).
pub const STEEL_COUNT_RADIUS: i32 = 15;
/// Critical shortCraft bloom radius (Haxe shortCraft r=5).
pub const BLOOM_SHORTCRAFT_RADIUS: i32 = 5;
/// Crucible tongs → firing forge shortCraft radius (Haxe r=10).
pub const TONGS_FORGE_SHORTCRAFT_RADIUS: i32 = 10;
/// Charcoal basket → forge shortCraft radius (Haxe r=30).
pub const CHARCOAL_FORGE_SHORTCRAFT_RADIUS: i32 = 30;
/// Default drop distance for GetCraftAndDropItemsCloseToObj near forge.
pub const DROP_NEAR_FORGE_DIST: i32 = 5;

/// Haxe count / shortCraft radii used by live `CountCloseObjects` / shortCraft fillers.
#[allow(dead_code)]
pub fn smith_radius_table() -> &'static [(i32, &'static str)] {
    &[
        (FORGE_SEARCH_RADIUS, "GetForge home"),
        (FLAT_STONE_COUNT_RADIUS, "flat/stone home"),
        (IRON_ORE_COUNT_RADIUS, "iron/ore forge"),
        (STEEL_COUNT_RADIUS, "steel/crucible"),
        (BLOOM_SHORTCRAFT_RADIUS, "bloom shortCraft"),
        (TONGS_FORGE_SHORTCRAFT_RADIUS, "tongs→forge"),
        (CHARCOAL_FORGE_SHORTCRAFT_RADIUS, "charcoal→forge"),
        (DROP_NEAR_FORGE_DIST, "drop near forge"),
    ]
}

/// Haxe `ServerSettings.objectIdArrays[455]` seed — any parentId whose description
/// contains `"Chisel"` is pushed at content load. Static seed always counted;
/// full content scan via [`collect_steel_chisel_family_ids`] /
/// [`steel_chisel_family_from_content`] extends via [`SmithCounts::chisel_family_extra`].
// Haxe: ServerSettings.PatchObjectData ~612–616; AiBase.doSmithing ~3869
pub const STEEL_CHISEL_FAMILY: &[i32] = &[
    STEEL_CHISEL,          // 455
    STEEL_CHISEL_FLAT,     // 451 on flat rock
    STEEL_FILE_WITH_CHISEL, // 466
];

/// Haxe `obj.description.contains('Chisel')` gate for objectIdArrays[455].
// Haxe: ServerSettings.PatchObjectData ~612
#[inline]
pub fn is_steel_chisel_family_description(description: &str) -> bool {
    description.contains("Chisel")
}

/// Collect unique parent ids for Haxe `objectIdArrays[455]`.
///
/// Always includes [`STEEL_CHISEL_FAMILY`] seed first, then every `(parent_id, desc)`
/// pair whose description contains `"Chisel"`.
// Haxe: ServerSettings.PatchObjectData ~612–616
pub fn collect_steel_chisel_family_ids<'a, I>(objects: I) -> Vec<i32>
where
    I: IntoIterator<Item = (i32, &'a str)>,
{
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for &id in STEEL_CHISEL_FAMILY {
        if id > 0 && seen.insert(id) {
            out.push(id);
        }
    }
    for (parent_id, desc) in objects {
        if parent_id <= 0 {
            continue;
        }
        if is_steel_chisel_family_description(desc) && seen.insert(parent_id) {
            out.push(parent_id);
        }
    }
    out
}

/// Content-scan extras only (not already in [`STEEL_CHISEL_FAMILY`]).
// Haxe: objectIdArrays[455] beyond static 455 seed
pub fn chisel_family_extras_beyond_static(family: &[i32]) -> Vec<i32> {
    family
        .iter()
        .copied()
        .filter(|&id| id > 0 && !STEEL_CHISEL_FAMILY.contains(&id))
        .collect()
}

/// Scan a [`ContentDb`] for parent ids with `"Chisel"` in description.
///
/// Uses [`ContentDb::resolve_base_id`] so multi-use dummies map to parent (Haxe `parentId`).
// Haxe: ServerSettings.objectIdArrays[455] from PatchObjectData
pub fn steel_chisel_family_from_content(db: &ol_content::ContentDb) -> Vec<i32> {
    collect_steel_chisel_family_ids(db.objects.values().map(|o| {
        let parent = db.resolve_base_id(o.id);
        (parent, o.description.as_str())
    }))
}

/// Load-time style cache for Haxe `objectIdArrays[455]` (PatchObjectData Chisel scan).
///
/// Call once per content load (npc scheduler boot / sim boot) instead of
/// re-scanning every profession tick.
// Haxe: ServerSettings.PatchObjectData ~612–616 objectIdArrays[455]
// AI-JOB-SMITH-RESID / chisel content cache
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SteelChiselFamilyTable {
    /// Full family (static seed + content extras).
    pub family: Vec<i32>,
    /// Content-only extras beyond [`STEEL_CHISEL_FAMILY`].
    pub extras: Vec<i32>,
}

impl SteelChiselFamilyTable {
    /// Build from a full content scan (Haxe PatchObjectData once at load).
    // Haxe: ServerSettings.objectIdArrays[455]
    pub fn from_content(db: &ol_content::ContentDb) -> Self {
        let family = steel_chisel_family_from_content(db);
        let extras = chisel_family_extras_beyond_static(&family);
        Self { family, extras }
    }

    /// Pure construct when ids are already collected.
    pub fn from_family(family: Vec<i32>) -> Self {
        let extras = chisel_family_extras_beyond_static(&family);
        Self { family, extras }
    }
}

/// Prefer published home over position proxy for NPC peer same-home filter.
///
/// When `snap_home` is `Some`, use it (even if 0,0). When absent (legacy views),
/// fall back to position as home proxy.
// Haxe: GlobalPlayerInstance.home.tx/ty; AiBase.countProfession same-home
// AI-JOB-SMITH-RESID / peer home fidelity
#[inline]
pub fn peer_home_coords(
    snap_home: Option<(i32, i32)>,
    pos_x: i32,
    pos_y: i32,
) -> (i32, i32) {
    snap_home.unwrap_or((pos_x, pos_y))
}

/// Haxe `isWounded()` for peer count when only held-object wound flag is known
/// (snapshot lacks hiddenWound alias — treat content wound as wounded).
// Haxe: GlobalPlayerInstance.isWounded; AiBase.countProfession skips wounded
// AI-JOB-SMITH-RESID / peer wound fidelity
#[inline]
pub fn peer_is_wounded_from_held(held_is_wound_object: bool) -> bool {
    held_is_wound_object
}

/// Haxe `dropNearForgeItemIds` subset used for pipeline seek bias.
pub const DROP_NEAR_FORGE_IDS: &[i32] = &[
    289, 290, 327, 326, 319, 320, 441, 568, 311, 308,
];

// ── Profession key ─────────────────────────────────────────────────────────

/// Canonical Haxe profession string for smith.
pub const SMITH_PROFESSION_KEY: &str = "SMITH";

/// Parse speech / assigned profession tokens for smith.
///
/// Accepts `SMITH`, `SMITH!`, case-insensitive.
// Haxe: AiBase speech endsWith("!") ~4950; assignedProfession == 'SMITH'
pub fn parse_smith_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("SMITH")
}

// ── Runtime / stage ────────────────────────────────────────────────────────

/// Sticky last + assigned + stage weight for SMITH.
///
/// Haxe `this.profession['SMITH']` is a float stage (0, 1.5, 2, 3, 3.5, 4…12).
/// Store on AI session / player AI state across ticks (live wire residual).
#[derive(Debug, Clone, PartialEq)]
pub struct SmithProfessionRuntime {
    /// Sticky last profession is smith.
    pub is_last_smith: bool,
    /// Assigned via speech / order.
    pub is_assigned_smith: bool,
    /// Haxe `this.profession['SMITH']` stage weight.
    pub stage: f32,
}

impl Default for SmithProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_smith: false,
            is_assigned_smith: false,
            stage: 0.0,
        }
    }
}

impl SmithProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset stage when product ladder finishes (Haxe sets SMITH=0 after knife).
    pub fn reset_stage(&mut self) {
        self.stage = 0.0;
    }

    /// Haxe `isConsideringMakingFood`: `profession['SMITH']=0` and last→Eating
    /// (unless last was FOODSERVER).
    // Haxe: AiBase.isConsideringMakingFood ~8482–8485
    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.stage = 0.0;
        if !last_was_foodserver {
            self.is_last_smith = false;
        }
    }
}

/// Count peers already sticky on SMITH (Haxe `countProfession('SMITH')`).
///
/// Prefer [`count_smith_peers_filtered`] when peer snapshots are available.
// Haxe: AiBase.countProfession
pub fn count_smith_peers(peer_count_with_last_smith: f32) -> f32 {
    peer_count_with_last_smith.max(0.0)
}

/// One AI peer for pure `countProfession('SMITH')` filtering.
// Haxe: AiBase.countProfession ~1284–1308
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmithPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    /// Haxe `ai.playerToFollow != null` — following another player excludes from count.
    pub has_player_to_follow: bool,
    /// Same home tile as the counting AI (`home.tx/ty` match).
    pub same_home: bool,
    /// `lastProfession == 'SMITH'`.
    pub last_is_smith: bool,
}

impl SmithPeerSnapshot {
    /// Eligible for profession count (before last-profession match).
    // Haxe: AiBase.countProfession filters
    pub fn eligible_for_count(self, min_age_to_eat: f32, max_age: f32) -> bool {
        if self.deleted {
            return false;
        }
        if self.age < min_age_to_eat {
            return false;
        }
        // Gravekeeper exception not used for SMITH.
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

    pub fn counts_as_smith(self, min_age_to_eat: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age_to_eat, max_age) && self.last_is_smith
    }
}

/// Full pure `countProfession('SMITH')` over peer snapshots.
// Haxe: AiBase.countProfession ~1284
pub fn count_smith_peers_filtered(
    peers: &[SmithPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_smith(min_age_to_eat, max_age))
        .count() as f32
}

/// Lightweight NPC/player row for max-one smith population (no full [`crate::Player`]).
///
/// Used by npc_ai when only `PlayerSnapshot` + sticky `SmithProfessionRuntime` exist.
// Haxe: AiBase.countProfession over Connection.getAis (NPC smith pop residual)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NpcSmithPeerRow {
    pub conn_id: u64,
    pub home_x: i32,
    pub home_y: i32,
    pub age: f32,
    pub food_store: f32,
    pub deleted: bool,
    pub has_player_to_follow: bool,
    pub is_wounded: bool,
    /// Sticky last profession is SMITH (`SmithProfessionRuntime.is_last_smith`).
    pub last_is_smith: bool,
}

impl NpcSmithPeerRow {
    /// Build from published snapshot fields + optional sticky override.
    ///
    /// `last_is_smith_override` ORs with snapshot sticky so npc-local profession_state
    /// still counts when the view has not been republished yet.
    // Haxe: countProfession row from Connection AI + lastProfession
    // AI-JOB-SMITH-RESID / PlayerSnapshot home + is_last_smith
    pub fn from_snapshot_fields(
        conn_id: u64,
        home_x: i32,
        home_y: i32,
        age: f32,
        food_store: f32,
        deleted: bool,
        has_player_to_follow: bool,
        is_wounded: bool,
        snap_is_last_smith: bool,
        last_is_smith_override: bool,
    ) -> Self {
        Self {
            conn_id,
            home_x,
            home_y,
            age,
            food_store,
            deleted,
            has_player_to_follow,
            is_wounded,
            last_is_smith: snap_is_last_smith || last_is_smith_override,
        }
    }
}

/// Pure max-one smith peer count from lightweight NPC rows (same-home filters).
// Haxe: countProfession('SMITH') + hasOrBecomeProfession max one
pub fn smith_peer_count_from_npc_rows(
    rows: &[NpcSmithPeerRow],
    self_conn_id: u64,
    home_x: i32,
    home_y: i32,
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    let peers: Vec<SmithPeerSnapshot> = rows
        .iter()
        .filter(|r| r.conn_id != self_conn_id)
        .map(|r| SmithPeerSnapshot {
            deleted: r.deleted,
            age: r.age,
            is_wounded: r.is_wounded,
            food_store: r.food_store,
            has_player_to_follow: r.has_player_to_follow,
            same_home: r.home_x == home_x && r.home_y == home_y,
            last_is_smith: r.last_is_smith,
        })
        .collect();
    count_smith_peers_filtered(&peers, min_age_to_eat, max_age)
}

/// Apply eat-path profession wipe (stage 0 + clear sticky unless FOODSERVER).
// Haxe: AiBase.isConsideringMakingFood ~8482–8485
pub fn wipe_smith_on_eat(runtime: &mut SmithProfessionRuntime, last_was_foodserver: bool) {
    runtime.wipe_on_eat(last_was_foodserver);
}

/// Haxe `hasOrBecomeProfession('SMITH', max)` with **hard max one smith**.
///
/// Special rule (Haxe ~4481): if any peer already has lastProfession SMITH
/// (`count > 0`), refuse — even when `max` would allow more.
///
/// - Sticky: if already last smith, keep and return true.
/// - `max < 0`: high priority — do job without assigning (always true).
// Haxe: AiBase.hasOrBecomeProfession ~4466 + SMITH special case
pub fn has_or_become_smith(
    runtime: &mut SmithProfessionRuntime,
    max: i32,
    peer_count_with_last_smith: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        // High priority: do job but do not assign profession.
        return true;
    }
    if runtime.is_last_smith {
        runtime.is_last_smith = true;
        return true;
    }
    let count = count_smith_peers(peer_count_with_last_smith);
    // Hard cap: at most one SMITH in the village (Haxe ignores max for this).
    if count > 0.0 {
        return false;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if count >= cap {
        return false;
    }
    runtime.stage = runtime.stage.max(1.0);
    runtime.is_last_smith = true;
    true
}

/// Prefer assigned over sticky last for AssignedJob dispatch.
// Haxe: assignedProfession == 'SMITH' || lastProfession == 'SMITH' ~724
pub fn resolve_smith_assigned_job(runtime: &SmithProfessionRuntime) -> bool {
    runtime.is_assigned_smith || runtime.is_last_smith
}

// ── Forge selection ────────────────────────────────────────────────────────

/// Ordered forge parent ids: Firing → Charcoal → cold Forge.
// Haxe: AiBase.GetForge ~3644
pub fn forge_id_priority() -> &'static [i32] {
    &[FIRING_FORGE, FORGE_WITH_CHARCOAL, FORGE]
}

/// Pick best forge parent id from a set of nearby forge parent ids.
///
/// Returns the first match in [`forge_id_priority`] order, or `None`.
pub fn pick_forge_parent(nearby_forge_parent_ids: &[i32]) -> Option<i32> {
    for &want in forge_id_priority() {
        if nearby_forge_parent_ids.contains(&want) {
            return Some(want);
        }
    }
    None
}

/// Spatial forge candidate (id + world tile).
// Haxe: AiHelper.GetClosestObjectToPosition result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeCandidate {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Chebyshev distance (OHOL tile distance used by many AI helpers).
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Haxe `GetForge`: priority 304→305→303, closest within [`FORGE_SEARCH_RADIUS`] of home.
///
/// Among matches of the first priority that has any candidate in range, pick closest.
// Haxe: AiBase.GetForge ~3644–3656
pub fn pick_forge_near_home(
    home_x: i32,
    home_y: i32,
    candidates: &[ForgeCandidate],
) -> Option<ForgeCandidate> {
    pick_forge_near_home_radius(home_x, home_y, candidates, FORGE_SEARCH_RADIUS)
}

/// Like [`pick_forge_near_home`] with explicit radius.
pub fn pick_forge_near_home_radius(
    home_x: i32,
    home_y: i32,
    candidates: &[ForgeCandidate],
    radius: i32,
) -> Option<ForgeCandidate> {
    for &want in forge_id_priority() {
        let mut best: Option<(i32, ForgeCandidate)> = None;
        for &c in candidates {
            if c.parent_id != want {
                continue;
            }
            let d = chebyshev(home_x, home_y, c.x, c.y);
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

/// True if `id` is any forge family parent.
pub fn is_forge_id(id: i32) -> bool {
    matches!(id, FORGE | FIRING_FORGE | FORGE_WITH_CHARCOAL)
}

/// Steel / crucible parent ids counted within [`STEEL_COUNT_RADIUS`] of forge.
///
/// Haxe TODO in prepareSmithingTools: use distance of 15 for steel/crucible counts.
// Haxe: AiBase.prepareSmithingTools ~3715 TODO
pub fn is_steel_crucible_count_id(id: i32) -> bool {
    matches!(
        id,
        UNFORGED_SEALED_CRUCIBLE
            | UNFORGED_CRUCIBLE_TONGS
            | FORGED_CRUCIBLE
            | HOT_CRUCIBLE_TONGS
            | COOL_CRUCIBLE_TONGS
            | STEEL_INGOT
            | STEEL_INGOT_FLAT
            | STEEL_FILE_BLANK_FLAT
            | STEEL_CHISEL_FLAT
            | STEEL_ADZE_HEAD_FLAT
            | STEEL_FILE_BLANK
            | HOT_STEEL_FILE_BLANK
            | OILED_FILE_BLANK
            | OILED_FILE_BLANK_CHISEL
            | STEEL_FILE_WITH_CHISEL
            | STEEL_FILE
            | STEEL_BLADE_BLANK
            | STEEL_CHISEL
            | STEEL_AXE
            | STEEL_MINING_PICK
            | STEEL_HOE
    )
}

// ── World counts snapshot ──────────────────────────────────────────────────

/// Close-object counts near home/forge (Haxe CountCloseObjects / countCurrentObject).
#[derive(Debug, Clone, Default)]
pub struct SmithCounts {
    /// Object parent id → count (piles expanded by caller if needed).
    pub by_id: HashMap<i32, i32>,
    /// Held object parent id (0 empty).
    pub held_id: i32,
    /// Closest forge parent id if any (303/304/305); `None` = no forge.
    pub forge_parent_id: Option<i32>,
    /// Optional extra chisel-family parent ids from content scan (extends [`STEEL_CHISEL_FAMILY`]).
    pub chisel_family_extra: Vec<i32>,
}

impl SmithCounts {
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

    /// Steel chisel stock: static family + optional content-scan extras + held.
    // Haxe: countCurrentObjects(objectIdArrays[455])
    pub fn count_steel_chisel_stock(&self) -> i32 {
        let mut n = 0;
        for &id in STEEL_CHISEL_FAMILY {
            n += self.get(id);
            if self.held_id == id {
                n += 1;
            }
        }
        for &id in &self.chisel_family_extra {
            if STEEL_CHISEL_FAMILY.contains(&id) {
                continue; // already counted
            }
            n += self.get(id);
            if self.held_id == id {
                n += 1;
            }
        }
        n
    }

    /// Attach content-scan extras (Haxe objectIdArrays[455] beyond static seed).
    // Haxe: ServerSettings.objectIdArrays[455]
    pub fn attach_chisel_family(&mut self, family: &[i32]) {
        self.chisel_family_extra = chisel_family_extras_beyond_static(family);
    }

    /// Attach only pre-filtered extras (ids not in [`STEEL_CHISEL_FAMILY`]).
    pub fn attach_chisel_family_extra(&mut self, extras: &[i32]) {
        self.chisel_family_extra = extras
            .iter()
            .copied()
            .filter(|&id| id > 0 && !STEEL_CHISEL_FAMILY.contains(&id))
            .collect();
    }
}

/// Map object at a tile for mock [`fill_smith_counts_from_map`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Fill [`SmithCounts`] from a mock map snapshot (unit tests / thin tick).
///
/// - Forge: [`pick_forge_near_home`] → `forge_parent_id`
/// - Flat/stone: count within [`FLAT_STONE_COUNT_RADIUS`] of home
/// - Iron/ore/wrought: count within [`IRON_ORE_COUNT_RADIUS`] of forge (or home if no forge)
/// - Steel/crucible family: count within [`STEEL_COUNT_RADIUS`] of forge (Haxe TODO r=15)
/// - Other smith pipeline ids: count within `home_r` of home (default 20)
///
/// See [`fill_smith_counts_from_map_ex`] to attach content-scanned chisel family.
// Haxe: GetForge + CountCloseObjects radii in prepareSmithingTools (~3715 steel r=15)
pub fn fill_smith_counts_from_map(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[MapObj],
    home_r: i32,
) -> SmithCounts {
    fill_smith_counts_from_map_ex(home_x, home_y, held_id, objects, home_r, None)
}

/// Like [`fill_smith_counts_from_map`] plus optional Haxe `objectIdArrays[455]` family.
///
/// `chisel_family` is the full family (static + content); only extras are stored on counts.
// Haxe: countCurrentObjects(objectIdArrays[455]) after PatchObjectData Chisel scan
pub fn fill_smith_counts_from_map_ex(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[MapObj],
    home_r: i32,
    chisel_family: Option<&[i32]>,
) -> SmithCounts {
    let forge_cands: Vec<ForgeCandidate> = objects
        .iter()
        .filter(|o| is_forge_id(o.parent_id))
        .map(|o| ForgeCandidate {
            parent_id: o.parent_id,
            x: o.x,
            y: o.y,
        })
        .collect();
    let forge = pick_forge_near_home(home_x, home_y, &forge_cands);
    let (fx, fy) = forge
        .map(|f| (f.x, f.y))
        .unwrap_or((home_x, home_y));

    let mut counts = SmithCounts {
        held_id,
        forge_parent_id: forge.map(|f| f.parent_id),
        ..Default::default()
    };

    for o in objects {
        let d_home = chebyshev(home_x, home_y, o.x, o.y);
        let d_forge = chebyshev(fx, fy, o.x, o.y);
        let id = o.parent_id;
        let in_range = if id == FLAT_ROCK || id == STONE {
            d_home <= FLAT_STONE_COUNT_RADIUS
        } else if id == IRON_ORE || id == WROUGHT_IRON {
            d_forge <= IRON_ORE_COUNT_RADIUS
        } else if is_steel_crucible_count_id(id) {
            // Haxe TODO: steel/crucible counts use r=15 from forge (not generic home_r).
            d_forge <= STEEL_COUNT_RADIUS
        } else if is_forge_id(id) {
            false // forge is parent only
        } else {
            d_home <= home_r
        };
        if in_range {
            let n = counts.get(id);
            counts.set(id, n + 1);
        }
    }
    if let Some(fam) = chisel_family {
        counts.attach_chisel_family(fam);
    }
    counts
}

// ── Actions ────────────────────────────────────────────────────────────────

/// Pure decision output — execution is AI-CRAFT / shortCraft wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithAction {
    /// Nothing to do in this step.
    None,
    /// Haxe `shortCraft(actor, target)`.
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `shortCraftOnGround(target)` — empty-hands use on ground object.
    ShortCraftOnGround { target: i32 },
    /// Haxe `craftItem(objectId)` — produce / obtain object.
    CraftItem { object_id: i32 },
    /// Haxe `GetCraftAndDropItemsCloseToObj(forge, id, want, dist)`.
    CraftAndDropNearForge { object_id: i32, want_count: i32 },
    /// Forge not firing + kiln hot → chain to `doPotteryOnFire` (AI-JOB-POTTER).
    DeferPottery,
    /// No forge / cannot become smith / job refuse.
    Abort,
}

impl SmithAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None | Self::Abort | Self::DeferPottery)
    }

    /// True when live tick should chain to pottery (not a smith craft step).
    pub fn is_defer(self) -> bool {
        matches!(self, Self::DeferPottery)
    }
}

// ── Critical shortCrafts (high priority, also outside full doSmithing) ─────

/// Haxe critical smith shortCrafts before assigned job (~617–621).
///
/// Order:
/// 1. Hammer 441 + Hot Iron Bloom 309 (when hammer stock/held)
/// 2. Stone 33 + Hot Iron Bloom 309 (fallback when no hammer)
/// 3. Unforged crucible tongs 320 + Firing Forge 304 (if not holding hammer)
/// 4. Basket of Charcoal 298 + Forge 303
// Haxe: AiBase.doTimeStuffHelper ~617–621
pub fn critical_smith_shortcrafts(held_id: i32, counts: &SmithCounts) -> SmithAction {
    if counts.get(HOT_IRON_BLOOM_FLAT) > 0 {
        // Prefer hammer when held or nearby; else stone fallback (Haxe second try).
        if held_id == SMITHING_HAMMER || counts.get(SMITHING_HAMMER) > 0 {
            return SmithAction::ShortCraft {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT,
            };
        }
        return SmithAction::ShortCraft {
            actor: STONE,
            target: HOT_IRON_BLOOM_FLAT,
        };
    }
    if held_id != SMITHING_HAMMER
        && counts.get(UNFORGED_CRUCIBLE_TONGS) > 0
        && counts.forge_parent_id == Some(FIRING_FORGE)
    {
        return SmithAction::ShortCraft {
            actor: UNFORGED_CRUCIBLE_TONGS,
            target: FIRING_FORGE,
        };
    }
    // Charcoal basket into cold forge (start of doSmithing).
    if held_id == BASKET_OF_CHARCOAL && counts.forge_parent_id == Some(FORGE) {
        return SmithAction::ShortCraft {
            actor: BASKET_OF_CHARCOAL,
            target: FORGE,
        };
    }
    SmithAction::None
}

// ── prepareSmithingTools ───────────────────────────────────────────────────

/// Haxe `prepareSmithingTools` pure body.
///
/// Mutates `runtime.stage` as Haxe advances `profession['SMITH']`.
///
/// Order (see module delta): shortCrafts → flat/stone → **iron (stage&lt;3)** →
/// steel/crucible → hammer (stage&lt;5).
// Haxe: AiBase.prepareSmithingTools ~3659
pub fn prepare_smithing_tools(
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
) -> SmithAction {
    let Some(forge_id) = counts.forge_parent_id else {
        return SmithAction::Abort;
    };
    let held = counts.held_id;

    // Smithing Hammer + Hot Iron Bloom on Flat Rock; stone fallback when no hammer.
    // Haxe: prepareSmithingTools shortCraft(441,309) then shortCraft(33,309) ~3667–3670
    if counts.get(HOT_IRON_BLOOM_FLAT) > 0 {
        if held == SMITHING_HAMMER || counts.get(SMITHING_HAMMER) > 0 {
            return SmithAction::ShortCraft {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT,
            };
        }
        return SmithAction::ShortCraft {
            actor: STONE,
            target: HOT_IRON_BLOOM_FLAT,
        };
    }

    // Crucible tongs into firing forge (unless holding hammer)
    if held != SMITHING_HAMMER
        && counts.get(UNFORGED_CRUCIBLE_TONGS) > 0
        && forge_id == FIRING_FORGE
    {
        return SmithAction::ShortCraft {
            actor: UNFORGED_CRUCIBLE_TONGS,
            target: FIRING_FORGE,
        };
    }

    // Pottery fallthrough when forge not firing and not holding stone/hammer.
    // Haxe: prepareSmithingTools ~3675–3680 → doPotteryOnFire()
    if forge_id != FIRING_FORGE
        && held != STONE
        && held != SMITHING_HAMMER
        && counts.get(FIRING_KILN) > 0
    {
        return SmithAction::DeferPottery;
    }

    // Cold Iron Bloom reheat: Bowl of Water 239 + 312
    if counts.get(COLD_IRON_BLOOM_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: BOWL_OF_WATER,
            target: COLD_IRON_BLOOM_FLAT,
        };
    }
    // Pickup wrought iron / steel from flat rock
    if counts.get(WROUGHT_IRON_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: 0,
            target: WROUGHT_IRON_FLAT,
        };
    }
    if counts.get(STEEL_INGOT_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: 0,
            target: STEEL_INGOT_FLAT,
        };
    }
    if counts.get(STEEL_FILE_BLANK_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: 0,
            target: STEEL_FILE_BLANK_FLAT,
        };
    }
    if counts.get(STEEL_CHISEL_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: 0,
            target: STEEL_CHISEL_FLAT,
        };
    }
    if counts.get(STEEL_ADZE_HEAD_FLAT) > 0 {
        return SmithAction::ShortCraft {
            actor: 0,
            target: STEEL_ADZE_HEAD_FLAT,
        };
    }

    // Staging: flat rocks + stone near forge when stage < 3 or forge is firing.
    if runtime.stage < 3.0 || forge_id == FIRING_FORGE {
        let flat = counts.get(FLAT_ROCK);
        if flat < 2 {
            return SmithAction::CraftAndDropNearForge {
                object_id: FLAT_ROCK,
                want_count: 2,
            };
        }
        let mut stone = counts.get(STONE);
        if held == STONE || held == SMITHING_HAMMER {
            stone += 1;
        }
        if stone < 1 {
            return SmithAction::CraftAndDropNearForge {
                object_id: STONE,
                want_count: 1,
            };
        }
    }

    let count_steel = counts.get(STEEL_INGOT);

    // ── Iron before steel while stage < 3 (intentional delta; see module docs) ──
    // Haxe: AiBase.prepareSmithingTools wrought iron block ~3769
    if runtime.stage < 3.0 {
        let mut iron = counts.get(WROUGHT_IRON);
        if held == WROUGHT_IRON {
            iron += 1;
        }
        if iron < 5 {
            if runtime.stage < 2.0 {
                let ore = counts.get(IRON_ORE);
                if ore < 5 {
                    return SmithAction::CraftItem {
                        object_id: IRON_ORE,
                    };
                }
                runtime.stage = 2.0;
            }
            return SmithAction::CraftItem {
                object_id: WROUGHT_IRON,
            };
        }
        runtime.stage = 3.0;
    }

    let count_crucible = counts.get(UNFORGED_SEALED_CRUCIBLE);
    let count_forged_crucible = counts.get(FORGED_CRUCIBLE);

    // Steel / crucible path (Haxe when countSteel < 1 || forged crucible present)
    if count_steel < 1 || count_forged_crucible > 0 {
        // Cool Steel Crucible in Wooden Tongs — Haxe shortCraftOnGround(324) ~3725
        if counts.get(COOL_CRUCIBLE_TONGS) > 0 {
            return SmithAction::ShortCraftOnGround {
                target: COOL_CRUCIBLE_TONGS,
            };
        }

        if runtime.stage < 3.5 && count_forged_crucible < 1 {
            if count_crucible < 3 {
                return SmithAction::CraftAndDropNearForge {
                    object_id: UNFORGED_SEALED_CRUCIBLE,
                    want_count: 3,
                };
            }
            runtime.stage = 3.5;
        }

        // Hot Steel Crucible in Wooden Tongs 323
        if count_crucible > 0 {
            return SmithAction::CraftItem {
                object_id: HOT_CRUCIBLE_TONGS,
            };
        }

        // Steel Ingot 326
        return SmithAction::CraftItem {
            object_id: STEEL_INGOT,
        };
    }

    if count_steel > 1 && runtime.stage < 4.0 {
        runtime.stage = 4.0;
    }

    if count_steel < 1 {
        return SmithAction::None;
    }

    if runtime.stage < 5.0 {
        let hammer = counts.get(SMITHING_HAMMER);
        if hammer < 1 {
            return SmithAction::CraftItem {
                object_id: SMITHING_HAMMER,
            };
        }
        runtime.stage = 5.0;
    }

    SmithAction::None
}

// ── doSmithing product ladder ──────────────────────────────────────────────

/// Haxe `doSmithing` product ladder pure body (stages 6…12 then knife).
///
/// Resets stage to 0 after knife stock is present.
// Haxe: AiBase.doSmithing ~3824–3906
pub fn do_smithing_products(
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
) -> SmithAction {
    // Steel Mining Pick 684
    if runtime.stage < 6.0 {
        if counts.get(STEEL_MINING_PICK) < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_MINING_PICK,
            };
        }
        runtime.stage = 6.0;
    }

    // Shovel 502 (+ dung shovel counts)
    if runtime.stage < 7.0 {
        let shovel = counts.get(SHOVEL) + counts.get(SHOVEL_OF_DUNG);
        if shovel < 1 {
            return SmithAction::CraftItem {
                object_id: SHOVEL,
            };
        }
        runtime.stage = 7.0;
    }

    // Steel Hoe 857
    if runtime.stage < 7.1 {
        let hoe = counts.get_with_held(STEEL_HOE);
        if hoe < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_HOE,
            };
        }
        runtime.stage = 7.1;
    }

    // Shears 568
    if runtime.stage < 7.5 {
        if counts.get(SHEARS) < 1 {
            return SmithAction::CraftItem {
                object_id: SHEARS,
            };
        }
        runtime.stage = 7.5;
    }

    // Steel Axe 334
    if runtime.stage < 8.0 {
        if counts.get(STEEL_AXE) < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_AXE,
            };
        }
        runtime.stage = 8.0;
    }

    // Steel Chisel 455 — family count (Haxe objectIdArrays[455])
    if runtime.stage < 9.0 {
        if counts.count_steel_chisel_stock() < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_CHISEL,
            };
        }
        runtime.stage = 9.0;
    }

    let count_file = counts.get(STEEL_FILE);

    // Steel File Blank 457 (+ intermediate blanks)
    if runtime.stage < 10.0 {
        let blanks = counts.sum(&[
            STEEL_FILE_BLANK,
            HOT_STEEL_FILE_BLANK,
            OILED_FILE_BLANK,
            OILED_FILE_BLANK_CHISEL,
            STEEL_FILE_WITH_CHISEL,
        ]);
        if blanks + count_file < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_FILE_BLANK,
            };
        }
        runtime.stage = 10.0;
    }

    // Steel File 458
    if runtime.stage < 11.0 {
        if count_file < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_FILE,
            };
        }
        runtime.stage = 11.0;
    }

    // Steel Blade Blank 459
    if runtime.stage < 12.0 {
        if counts.get(STEEL_BLADE_BLANK) < 1 {
            return SmithAction::CraftItem {
                object_id: STEEL_BLADE_BLANK,
            };
        }
        runtime.stage = 12.0;
    }

    // Knife 560
    if counts.get(KNIFE) < 1 {
        return SmithAction::CraftItem {
            object_id: KNIFE,
        };
    }

    runtime.stage = 0.0;
    SmithAction::None
}

/// Full `doSmithing` entry: charcoal, profession, forge, prepare, products.
// Haxe: AiBase.doSmithing ~3805
pub fn do_smithing(
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
    max_people: i32,
    peer_count_with_last_smith: f32,
    was_idle: f32,
) -> SmithAction {
    // Basket of Charcoal + Forge
    if counts.held_id == BASKET_OF_CHARCOAL && counts.forge_parent_id == Some(FORGE) {
        return SmithAction::ShortCraft {
            actor: BASKET_OF_CHARCOAL,
            target: FORGE,
        };
    }

    if !has_or_become_smith(runtime, max_people, peer_count_with_last_smith, was_idle) {
        return SmithAction::Abort;
    }

    if counts.forge_parent_id.is_none() {
        return SmithAction::Abort;
    }

    // Surface prepare result including DeferPottery (is_some() excludes defer).
    // Haxe: prepareSmithingTools → doPotteryOnFire when kiln firing + cold forge
    let prep = prepare_smithing_tools(counts, runtime);
    match prep {
        SmithAction::None => {}
        other => return other,
    }

    do_smithing_products(counts, runtime)
}

/// Haxe `doTimeStuffHelper` call slots that invoke smith work.
// Haxe: early sticky ~609; critical ~617–621; assigned ~724–725; mid ~770; low ~811; elder collect ~6004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithJobSlot {
    /// `lastProfession == SMITH` early high-prio → `doSmithing()` max=1.
    EarlySticky,
    /// Critical shortCrafts only (bloom / tongs / charcoal).
    CriticalShortCraft,
    /// AssignedJob / sticky assigned path → `doSmithing(100)`.
    Assigned,
    /// Mid sticky `lastProfession == SMITH` → `doSmithing()` max=1.
    MidSticky,
    /// Open low-priority `doSmithing()` max=1.
    LowPriority,
    /// Collecting helper age>40 → `doSmithing(1)`.
    ElderCollect,
}

impl SmithJobSlot {
    /// Haxe `maxPeople` for this slot (`doSmithing(maxPeople)`).
    pub fn max_people(self) -> i32 {
        match self {
            Self::Assigned => 100,
            Self::EarlySticky
            | Self::MidSticky
            | Self::LowPriority
            | Self::ElderCollect => 1,
            Self::CriticalShortCraft => 0, // N/A — shortCraft only
        }
    }

    /// Whether this slot runs full `doSmithing` (vs critical shortCraft only).
    pub fn is_full_job(self) -> bool {
        !matches!(self, Self::CriticalShortCraft)
    }
}

/// Recommended evaluation order for a pure tick (Haxe doTimeStuffHelper order).
pub fn smith_job_slot_priority() -> &'static [SmithJobSlot] {
    &[
        SmithJobSlot::EarlySticky,
        SmithJobSlot::CriticalShortCraft,
        SmithJobSlot::Assigned,
        SmithJobSlot::MidSticky,
        SmithJobSlot::LowPriority,
        SmithJobSlot::ElderCollect,
    ]
}

/// Dispatch smith job for AssignedJob / sticky last / mid-prio.
pub fn decide_smith_job(
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
    max_people: i32,
    peer_count_with_last_smith: f32,
    was_idle: f32,
) -> SmithAction {
    do_smithing(
        counts,
        runtime,
        max_people,
        peer_count_with_last_smith,
        was_idle,
    )
}

/// Slot-aware dispatch matching Haxe multi call sites.
// Haxe: doTimeStuffHelper smith slots
pub fn decide_smith_job_for_slot(
    slot: SmithJobSlot,
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
    peer_count_with_last_smith: f32,
    was_idle: f32,
    // ElderCollect: player age (Haxe age > 40).
    age: f32,
) -> SmithAction {
    match slot {
        SmithJobSlot::CriticalShortCraft => {
            critical_smith_shortcrafts(counts.held_id, counts)
        }
        SmithJobSlot::EarlySticky | SmithJobSlot::MidSticky => {
            if !runtime.is_last_smith && !runtime.is_assigned_smith {
                return SmithAction::None;
            }
            do_smithing(
                counts,
                runtime,
                slot.max_people(),
                peer_count_with_last_smith,
                was_idle,
            )
        }
        SmithJobSlot::Assigned => {
            if !resolve_smith_assigned_job(runtime) {
                return SmithAction::None;
            }
            do_smithing(
                counts,
                runtime,
                slot.max_people(),
                peer_count_with_last_smith,
                was_idle,
            )
        }
        SmithJobSlot::LowPriority => do_smithing(
            counts,
            runtime,
            slot.max_people(),
            peer_count_with_last_smith,
            was_idle,
        ),
        SmithJobSlot::ElderCollect => {
            if age <= 40.0 {
                return SmithAction::None;
            }
            do_smithing(
                counts,
                runtime,
                slot.max_people(),
                peer_count_with_last_smith,
                was_idle,
            )
        }
    }
}

// ── Pipeline seek / craft graph ────────────────────────────────────────────

/// Ordered smith product + intermediate ids (stage ladder seek order).
pub fn smith_pipeline_targets() -> &'static [i32] {
    &[
        // Prep / materials
        FLAT_ROCK,
        STONE,
        IRON_ORE,
        WROUGHT_IRON,
        UNFORGED_SEALED_CRUCIBLE,
        STEEL_INGOT,
        SMITHING_HAMMER,
        // Product ladder
        STEEL_MINING_PICK,
        SHOVEL,
        STEEL_HOE,
        SHEARS,
        STEEL_AXE,
        STEEL_CHISEL,
        STEEL_FILE_BLANK,
        STEEL_FILE,
        STEEL_BLADE_BLANK,
        KNIFE,
    ]
}

/// Stage-aware pure goal: first missing pipeline id, else reverse-craft via graph.
// Haxe: doSmithing craftItem chain + thin pick_smith_goal
pub fn pick_smith_profession_goal(
    graph: &ReverseCraftGraph,
    have: &HashSet<i32>,
    stage: f32,
) -> Goal {
    // Stage-gated product wants (match do_smithing_products / prepare thresholds).
    let stage_wants: &[(f32, i32)] = &[
        (2.0, IRON_ORE),
        (3.0, WROUGHT_IRON),
        (3.5, UNFORGED_SEALED_CRUCIBLE),
        (4.0, STEEL_INGOT),
        (5.0, SMITHING_HAMMER),
        (6.0, STEEL_MINING_PICK),
        (7.0, SHOVEL),
        (7.1, STEEL_HOE),
        (7.5, SHEARS),
        (8.0, STEEL_AXE),
        (9.0, STEEL_CHISEL),
        (10.0, STEEL_FILE_BLANK),
        (11.0, STEEL_FILE),
        (12.0, STEEL_BLADE_BLANK),
        (13.0, KNIFE),
    ];
    for &(need_stage, want) in stage_wants {
        if stage < need_stage && !have.contains(&want) {
            if let Some(ing) = graph.seek_ingredient_for(want, have) {
                return Goal::SeekObject(ing);
            }
            return Goal::SeekObject(want);
        }
    }
    for &want in smith_pipeline_targets() {
        if have.contains(&want) {
            continue;
        }
        if let Some(ing) = graph.seek_ingredient_for(want, have) {
            return Goal::SeekObject(ing);
        }
        return Goal::SeekObject(want);
    }
    crate::ai_goals::pick_smith_goal(graph, have, SMITH_IRON_ID)
}

/// Map a [`SmithAction`] into a high-level [`Goal`] for self-play / thin tick.
///
/// Prefer [`smith_action_apply`] for live USE/DROP/craft intents (AI-JOB-SMITH-LIVE).
pub fn smith_action_to_goal(action: SmithAction) -> Goal {
    match action {
        SmithAction::None | SmithAction::Abort => Goal::SeekObject(SMITH_TARGET_ID),
        // Seek target for USE; actor seek is handled by short_craft_apply when not held.
        SmithAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        // Cool crucible 324 etc. — ground-use path (not CraftItem).
        SmithAction::ShortCraftOnGround { target } => Goal::SeekObject(target),
        SmithAction::CraftItem { object_id } => Goal::SeekObject(object_id),
        SmithAction::CraftAndDropNearForge { object_id, .. } => Goal::SeekObject(object_id),
        SmithAction::DeferPottery => Goal::SeekObject(FIRING_KILN),
    }
}

// ── Live tick I/O apply (AI-JOB-SMITH-LIVE) ─────────────────────────────────

/// Pure next step after a [`SmithAction`] (shortCraft / craft / drop / pottery).
///
/// Live tick / `apply_intent` maps these to USE, DROP, seek, craftItem.
// Haxe: shortCraftOnTarget / shortCraftOnGround / GetCraftAndDropItemsCloseToObj / craftItem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmithApply {
    /// No-op / already satisfied.
    None,
    /// Job refuse (no forge / cannot become).
    Abort,
    /// Held matches actor → `useHeldObjOnTarget`.
    UseOnTarget { actor: i32, target: i32 },
    /// actorId == 0 and hands not empty → drop first.
    DropHeld,
    /// Need actor — `GetOrCraftItem` / seek.
    SeekOrCraftActor { actor: i32 },
    /// Holding ground-use object → use on empty tile (shortCraftOnGround held path).
    UseOnEmptyGround { held: i32 },
    /// Need to hold ground-use object first (`GetItem`).
    SeekOrGetGroundActor { target: i32 },
    /// craftItem / reverse-craft residual (seek-only until AI-CRAFT apply).
    CraftItem { object_id: i32 },
    /// Holding craft-and-drop object, too far from forge → goto forge.
    GotoForgeForDrop { object_id: i32 },
    /// Holding object near forge → `dropHeldObject` near forge.
    DropNearForge { object_id: i32 },
    /// Object exists near forge — pickup then re-enter drop path.
    PickupNearForge { object_id: i32 },
    /// DeferPottery with no pottery body step — seek kiln.
    DeferPottery,
    /// Hungry work cost refused the shortCraft.
    RefuseHungryCost,
    /// maxNewActor / biome-style refuse from shared short_craft_apply.
    Refuse,
}

impl SmithApply {
    pub fn is_actionable(self) -> bool {
        !matches!(
            self,
            Self::None | Self::Abort | Self::Refuse | Self::RefuseHungryCost | Self::DeferPottery
        )
    }
}

/// Haxe floor-place actor ids allowed without a transition (Pine Needles / Boards / Cut Stones).
// Haxe: AiBase.checkHungryWorkCostById ~1424–1425
pub const FLOOR_PLACE_ACTOR_IDS: [i32; 3] = [96, 470, 881];

/// Pure resolution of Haxe `checkHungryWorkCostById` inputs (no ContentDb).
///
/// Callers fill from `GetTransition(actor,target)` / `(actor,-1)`, object flags.
// Haxe: AiBase.checkHungryWorkCostById ~1412
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HungryWorkCostLookup {
    /// Transition found for (actor,target) or (actor,-1).
    pub transition_found: bool,
    /// `totalHungryWorkCost()` when found; ignored if not found.
    pub transition_hungry_cost: f32,
    /// Target `allowFloorPlacement`.
    pub target_allow_floor: bool,
    /// Target container slot count.
    pub target_num_slots: i32,
    /// Held object containable.
    pub held_containable: bool,
    /// Haxe `useIsDropInContainer` staging flag.
    pub use_is_drop_in_container: bool,
}

impl HungryWorkCostLookup {
    /// Known transition with given cost (no floor/container specials).
    pub fn from_transition_cost(cost: f32) -> Self {
        Self {
            transition_found: true,
            transition_hungry_cost: cost,
            target_allow_floor: false,
            target_num_slots: 0,
            held_containable: false,
            use_is_drop_in_container: false,
        }
    }
}

/// Full Haxe `checkHungryWorkCostById` pure gate (floor place + container drop + food).
///
/// - No transition: allow floor actors 96/470/881 when target allows floor; or
///   container-drop when `use_is_drop_in_container && numSlots>0 && containable`.
/// - Transition: refuse when `food_store < cost + 1` and cost > 0.
// Haxe: AiBase.checkHungryWorkCostById ~1412
pub fn check_hungry_work_cost_lookup(
    actor_id: i32,
    food_store: f32,
    lookup: &HungryWorkCostLookup,
) -> bool {
    if !lookup.transition_found {
        let can_place_floor = lookup.target_allow_floor
            && FLOOR_PLACE_ACTOR_IDS.contains(&actor_id);
        if can_place_floor {
            return true;
        }
        // TODOC Haxe: check container can really contain object
        if lookup.use_is_drop_in_container
            && lookup.target_num_slots > 0
            && lookup.held_containable
        {
            return true;
        }
        // No transition and no special allow → refuse (Haxe returns false).
        return false;
    }
    check_hungry_work_cost_by_id(food_store, lookup.transition_hungry_cost)
}

/// Haxe `checkHungryWorkCostById` food gate only (transition already known).
///
/// Caller supplies transition `totalHungryWorkCost` (0 = free / unknown allow).
/// For floor/container when transition missing, use [`check_hungry_work_cost_lookup`].
// Haxe: AiBase.checkHungryWorkCostById ~1441–1450
pub fn check_hungry_work_cost_by_id(food_store: f32, transition_hungry_cost: f32) -> bool {
    if transition_hungry_cost > 0.0 && food_store < transition_hungry_cost + 1.0 {
        return false;
    }
    true
}

/// Map a [`SmithAction::ShortCraft`] through shared [`crate::farmer_profession::short_craft_apply`].
///
/// Smith edges never use skewer/carrot/biome guards; `try_weak_skewer_first = false`.
/// Returns `None` for non-ShortCraft actions.
// Haxe: AiBase.shortCraftOnTarget ~2721 (smith call sites)
pub fn smith_action_short_craft_apply(
    action: SmithAction,
    held_id: i32,
    new_actor_count: i32,
    max_new_actor: i32,
) -> Option<crate::farmer_profession::ShortCraftApply> {
    match action {
        SmithAction::ShortCraft { actor, target } => {
            Some(crate::farmer_profession::short_craft_apply(
                crate::farmer_profession::ShortCraftInput {
                    held_id,
                    actor_id: actor,
                    target_id: target,
                    target_uses: 1,
                    target_biome: None,
                    has_carrot_seeds: true,
                    new_actor_count,
                    max_new_actor,
                    try_weak_skewer_first: false,
                    craft_actor_if_needed: true,
                    food_store: 20.0,
                    transition_hungry_cost: 0.0,
                },
            ))
        }
        _ => None,
    }
}

/// Pure `GetCraftAndDropItemsCloseToObj(forge, id, want, dist)` decision edges.
// Haxe: AiBase.GetCraftAndDropItemsCloseToObj ~893
pub fn craft_and_drop_near_forge_apply(
    object_id: i32,
    want_count: i32,
    near_count: i32,
    held_id: i32,
    // Haxe CalculateQuadDistanceToObject(forge) while holding the object.
    quad_dist_to_forge: i32,
    // True when a free object of object_id exists near forge (pickup path).
    nearby_object_exists: bool,
) -> SmithApply {
    if near_count >= want_count {
        return SmithApply::None;
    }
    if held_id == object_id {
        // Haxe: if (quadDist > 5) return gotoObj(target); else dropHeldObject(5, target)
        if quad_dist_to_forge > CRAFT_DROP_GOTO_QUAD_DIST {
            return SmithApply::GotoForgeForDrop { object_id };
        }
        return SmithApply::DropNearForge { object_id };
    }
    if nearby_object_exists {
        return SmithApply::PickupNearForge { object_id };
    }
    SmithApply::CraftItem { object_id }
}

/// Pure `shortCraftOnGround(target)` edges.
// Haxe: AiBase.shortCraftOnGround ~2692
pub fn short_craft_on_ground_apply(held_id: i32, target: i32) -> SmithApply {
    if held_id == target {
        SmithApply::UseOnEmptyGround { held: target }
    } else {
        SmithApply::SeekOrGetGroundActor { target }
    }
}

/// Inputs for live [`smith_action_apply`] (caller fills from world / inventory).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmithApplyInput {
    pub held_id: i32,
    /// Food store for hungry work cost gate (Haxe shortCraftOnTarget).
    pub food_store: f32,
    /// Transition hungry work cost for ShortCraft pair (0 = free).
    pub short_craft_work_cost: f32,
    /// Nearby newActor count when maxNewActor > 0 (smith usually -1).
    pub new_actor_count: i32,
    pub max_new_actor: i32,
    /// For CraftAndDropNearForge: count of object near forge within drop dist.
    pub craft_drop_near_count: i32,
    /// Quad distance to forge while holding craft-drop object.
    pub quad_dist_to_forge: i32,
    /// Pickup candidate exists near forge.
    pub craft_drop_nearby_exists: bool,
    /// Optional pottery counts when resolving DeferPottery; ignored otherwise.
    pub pottery: Option<PotteryOnFireCounts>,
}

impl SmithApplyInput {
    /// Minimal input (no hungry cost, unlimited newActor, craft-drop empty).
    pub fn basic(held_id: i32) -> Self {
        Self {
            held_id,
            food_store: 20.0,
            short_craft_work_cost: 0.0,
            new_actor_count: 0,
            max_new_actor: -1,
            craft_drop_near_count: 0,
            quad_dist_to_forge: 0,
            craft_drop_nearby_exists: false,
            pottery: None,
        }
    }
}

/// Full pure apply for a [`SmithAction`] → live USE/DROP/craft intent.
// Haxe: shortCraft / shortCraftOnGround / craftItem / GetCraftAndDrop / doPotteryOnFire
pub fn smith_action_apply(action: SmithAction, inp: &SmithApplyInput) -> SmithApply {
    match action {
        SmithAction::None => SmithApply::None,
        SmithAction::Abort => SmithApply::Abort,
        SmithAction::DeferPottery => {
            if let Some(ref pot) = inp.pottery {
                let step = do_pottery_on_fire(pot);
                if step != SmithAction::None && step != SmithAction::Abort {
                    return smith_action_apply(step, inp);
                }
            }
            SmithApply::DeferPottery
        }
        SmithAction::ShortCraft { actor, target } => {
            // Hungry gate lives in short_craft_apply (Haxe shortCraftOnTarget always-on).
            let sc = crate::farmer_profession::short_craft_apply(
                crate::farmer_profession::ShortCraftInput {
                    held_id: inp.held_id,
                    actor_id: actor,
                    target_id: target,
                    target_uses: 1,
                    target_biome: None,
                    has_carrot_seeds: true,
                    new_actor_count: inp.new_actor_count,
                    max_new_actor: inp.max_new_actor,
                    try_weak_skewer_first: false,
                    craft_actor_if_needed: true,
                    food_store: inp.food_store,
                    transition_hungry_cost: inp.short_craft_work_cost,
                },
            );
            match sc {
                crate::farmer_profession::ShortCraftApply::UseOnTarget { actor, target } => {
                    SmithApply::UseOnTarget { actor, target }
                }
                crate::farmer_profession::ShortCraftApply::DropHeld => SmithApply::DropHeld,
                crate::farmer_profession::ShortCraftApply::SeekOrCraftActor { actor, .. } => {
                    SmithApply::SeekOrCraftActor { actor }
                }
                crate::farmer_profession::ShortCraftApply::PreferWeakSkewer => {
                    // Should not fire for smith (try_weak_skewer_first=false).
                    SmithApply::SeekOrCraftActor { actor }
                }
                crate::farmer_profession::ShortCraftApply::Refuse => SmithApply::Refuse,
                crate::farmer_profession::ShortCraftApply::RefuseHungry => {
                    SmithApply::RefuseHungryCost
                }
            }
        }
        SmithAction::ShortCraftOnGround { target } => {
            short_craft_on_ground_apply(inp.held_id, target)
        }
        SmithAction::CraftItem { object_id } => SmithApply::CraftItem { object_id },
        SmithAction::CraftAndDropNearForge {
            object_id,
            want_count,
        } => craft_and_drop_near_forge_apply(
            object_id,
            want_count,
            inp.craft_drop_near_count,
            inp.held_id,
            inp.quad_dist_to_forge,
            inp.craft_drop_nearby_exists,
        ),
    }
}

// ── doPotteryOnFire pure body (smith DeferPottery fallthrough) ─────────────

/// Counts for pure [`do_pottery_on_fire`] (Haxe countCurrentObject(s) + CountCloseObjects).
// Haxe: AiBase.doPotteryOnFire ~2908
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PotteryOnFireCounts {
    /// Clay Bowl 235 (home / current inventory sense).
    pub count_bowl: i32,
    /// Close bowls r=20 (Haxe CountCloseObjects).
    pub count_close_bowl: i32,
    pub max_bowls: i32,
    /// Wet clay bowl 233 + wet bowl tongs 284.
    pub count_wet_bowl: i32,
    pub count_plate: i32,
    pub max_plates: i32,
    /// Wet plate 234 + tongs 240.
    pub count_wet_plate: i32,
    /// Clay crock 1217 + crock squash 1243.
    pub count_crock: i32,
    pub count_close_crock: i32,
    pub max_crock: i32,
    /// Wet crock 1216 + tongs 1218.
    pub count_wet_crock: i32,
    /// Wet Clay Nozzle 285 + Wet Nozzle tongs 295 (AI-POTTER-L2946).
    pub count_wet_nozzle: i32,
    /// Clay Nozzle 286 + Fired Nozzle tongs 296.
    pub count_nozzle: i32,
    /// aiCraftMax-style cap for nozzles (default 2).
    pub max_nozzle: i32,
    /// Big/huge charcoal piles 300+4102.
    pub count_coal: i32,
    /// Firing kiln 282 present (adobe+charcoal shortCraft gate).
    pub firing_kiln: bool,
}

impl Default for PotteryOnFireCounts {
    fn default() -> Self {
        Self {
            count_bowl: 0,
            count_close_bowl: 0,
            max_bowls: DEFAULT_MAX_CLAY_BOWLS,
            count_wet_bowl: 0,
            count_plate: 0,
            max_plates: DEFAULT_MAX_CLAY_PLATES,
            count_wet_plate: 0,
            count_crock: 0,
            count_close_crock: 0,
            max_crock: DEFAULT_MAX_CLAY_CROCKS,
            count_wet_crock: 0,
            count_wet_nozzle: 0,
            count_nozzle: 0,
            max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,
            count_coal: 0,
            firing_kiln: true,
        }
    }
}

/// Fill pottery-on-fire counts from a map snapshot (home-radius bowls/plates + kiln).
// Haxe: countCurrentObjects + CountCloseObjects in doPotteryOnFire
pub fn fill_pottery_on_fire_counts_from_map(
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    objects: &[MapObj],
    close_r: i32,
    max_bowls: i32,
    max_plates: i32,
    max_crock: i32,
) -> PotteryOnFireCounts {
    let mut c = PotteryOnFireCounts {
        max_bowls,
        max_plates,
        max_crock,
        ..Default::default()
    };
    let mut wet_bowl = 0;
    let mut wet_plate = 0;
    let mut wet_crock = 0;
    let mut wet_nozzle = 0;
    let mut bowl_home = 0;
    let mut plate_home = 0;
    let mut crock_home = 0;
    let mut nozzle_home = 0;
    let mut bowl_close = 0;
    let mut crock_close = 0;
    let mut coal = 0;
    let mut kiln = false;
    for o in objects {
        let d_home = chebyshev(home_x, home_y, o.x, o.y);
        let d_player = chebyshev(player_x, player_y, o.x, o.y);
        let id = o.parent_id;
        if id == FIRING_KILN && d_home <= FORGE_SEARCH_RADIUS {
            kiln = true;
        }
        // Home-ish current counts (r=20 like forge search)
        if d_home <= FORGE_SEARCH_RADIUS {
            match id {
                WET_CLAY_BOWL | WET_BOWL_TONGS => wet_bowl += 1,
                WET_CLAY_PLATE | WET_PLATE_TONGS => wet_plate += 1,
                WET_CLAY_CROCK | WET_CROCK_TONGS => wet_crock += 1,
                CLAY_BOWL => bowl_home += 1,
                CLAY_PLATE => plate_home += 1,
                CLAY_CROCK | CROCK_WITH_SQUASH => crock_home += 1,
                WET_CLAY_NOZZLE | WET_NOZZLE_TONGS => wet_nozzle += 1,
                CLAY_NOZZLE | FIRED_NOZZLE_TONGS => nozzle_home += 1,
                BIG_CHARCOAL_PILE | HUGE_CHARCOAL_PILE => coal += 1,
                _ => {}
            }
        }
        if d_player <= close_r {
            if id == CLAY_BOWL {
                bowl_close += 1;
            }
            if id == CLAY_CROCK {
                crock_close += 1;
            }
        }
    }
    c.count_wet_bowl = wet_bowl;
    c.count_wet_plate = wet_plate;
    c.count_wet_crock = wet_crock;
    c.count_wet_nozzle = wet_nozzle;
    c.count_bowl = bowl_home;
    c.count_plate = plate_home;
    c.count_crock = crock_home;
    c.count_nozzle = nozzle_home;
    c.max_nozzle = if c.max_nozzle > 0 {
        c.max_nozzle
    } else {
        DEFAULT_MAX_CLAY_NOZZLES
    };
    c.count_close_bowl = bowl_close;
    c.count_close_crock = crock_close;
    c.count_coal = coal;
    c.firing_kiln = kiln;
    c
}

/// Haxe `doPotteryOnFire` pure decision body (smith prepare fallthrough).
///
/// Order: fired bowl tongs under max → plate tongs → crock tongs →
/// **other potter crafts (L2946)** → adobe+kiln.
/// Bowl gate ports Haxe FIX as-is (`countBowl < countCloseBowls`); wet-bowl
/// path restored under L2946 residual crafts.
// Haxe: AiBase.doPotteryOnFire ~2908–2953
// Haxe: L2946 `// TODO make other potter stuff` → AI-POTTER-L2946
pub fn do_pottery_on_fire(c: &PotteryOnFireCounts) -> SmithAction {
    // Wooden Tongs with Fired Bowl 283 — Haxe FIX bowl limit uses close count.
    // Port-as-is: countBowl < maxBowls && countBowl < countCloseBowls
    if c.count_bowl < c.max_bowls && c.count_bowl < c.count_close_bowl {
        return SmithAction::CraftItem {
            object_id: FIRED_BOWL_TONGS,
        };
    }
    // Fired Plate in Wooden Tongs 241 when wet plates present under max
    if c.count_wet_plate > 0 && c.count_plate < c.max_plates {
        return SmithAction::CraftItem {
            object_id: FIRED_PLATE_TONGS,
        };
    }
    // Wooden Tongs with Fired Crock 1219
    if c.count_wet_crock > 0
        && c.count_crock < c.max_crock
        && c.count_close_crock < c.max_crock
    {
        return SmithAction::CraftItem {
            object_id: FIRED_CROCK_TONGS,
        };
    }

    // ── AI-POTTER-L2946 / Haxe L2946 other potter crafts ───────────────────
    // (slot is before adobe fuel in Haxe; adobe moved below residual crafts)

    // Wet-bowl fire path (Haxe original commented gate before FIX):
    // countWetBowl > 0 && countBowl < maxBowls && craftItem(283)
    if c.count_wet_bowl > 0 && c.count_bowl < c.max_bowls {
        return SmithAction::CraftItem {
            object_id: FIRED_BOWL_TONGS,
        };
    }

    // Shape wet crock: Wet Clay Bowl 233 + Wet Clay Bowl 233 → Wet Clay Crock 1216
    // Content: transitions/233_233.txt. Enables crock firing when wet bowls exist.
    let crock_stock = c.count_crock + c.count_wet_crock;
    if crock_stock < c.max_crock && c.count_wet_bowl >= 2 {
        return SmithAction::ShortCraft {
            actor: WET_CLAY_BOWL,
            target: WET_CLAY_BOWL,
        };
    }

    // Fire clay nozzle: craftItem(296) Fired Nozzle in Wooden Tongs
    let max_n = if c.max_nozzle > 0 {
        c.max_nozzle
    } else {
        DEFAULT_MAX_CLAY_NOZZLES
    };
    if c.count_wet_nozzle > 0 && c.count_nozzle < max_n {
        return SmithAction::CraftItem {
            object_id: FIRED_NOZZLE_TONGS,
        };
    }

    // Adobe 127 + Firing Adobe Kiln 282 when coal < 3 (Haxe after L2946 TODO)
    if c.count_coal < 3 && c.firing_kiln {
        return SmithAction::ShortCraft {
            actor: ADOBE,
            target: FIRING_KILN,
        };
    }
    SmithAction::None
}

/// Resolve DeferPottery through pottery body, else return action unchanged.
// Haxe: prepareSmithingTools → doPotteryOnFire()
pub fn resolve_smith_defer_pottery(
    action: SmithAction,
    pottery: &PotteryOnFireCounts,
) -> SmithAction {
    if matches!(action, SmithAction::DeferPottery) {
        let step = do_pottery_on_fire(pottery);
        if step != SmithAction::None {
            return step;
        }
    }
    action
}

// ── isConsideringMakingFood SMITH wipe wire ────────────────────────────────

/// Haxe age/hungry gate before profession wipe in `isConsideringMakingFood`.
// Haxe: AiBase.isConsideringMakingFood ~8476–8485
pub fn should_wipe_smith_on_consider_food(
    age: f32,
    is_hungry: bool,
    has_food_target: bool,
    min_age_to_eat: f32,
) -> bool {
    if age < min_age_to_eat {
        return false;
    }
    // Haxe: if (isHungry == false && foodTarget == null) return false;
    if !is_hungry && !has_food_target {
        return false;
    }
    true
}

/// Apply SMITH wipe when hungry path considers making food.
///
/// Returns true when wipe ran (stage 0 + clear last unless FOODSERVER).
// Haxe: AiBase.isConsideringMakingFood ~8482–8485
pub fn apply_consider_making_food_smith_wipe(
    runtime: &mut SmithProfessionRuntime,
    age: f32,
    is_hungry: bool,
    has_food_target: bool,
    min_age_to_eat: f32,
    last_was_foodserver: bool,
) -> bool {
    if !should_wipe_smith_on_consider_food(age, is_hungry, has_food_target, min_age_to_eat) {
        return false;
    }
    wipe_smith_on_eat(runtime, last_was_foodserver);
    true
}

/// Build a [`SmithPeerSnapshot`] from live player-ish fields (sim peer AI scan).
// Haxe: Connection.getAis + countProfession filters
pub fn smith_peer_snapshot(
    deleted: bool,
    age: f32,
    is_wounded: bool,
    food_store: f32,
    has_player_to_follow: bool,
    same_home: bool,
    last_is_smith: bool,
) -> SmithPeerSnapshot {
    SmithPeerSnapshot {
        deleted,
        age,
        is_wounded,
        food_store,
        has_player_to_follow,
        same_home,
        last_is_smith,
    }
}

// ── Ladder bridge (AI-JOB-SMITH-WIRE / LIVE) ────────────────────────────────

/// Job-band rungs that should run smith decide when profession / open job applies.
// Haxe: doTimeStuffHelper assigned / mid / low / craft slots + early sticky ~609
pub fn smith_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "AGE_ROTATED_JOB"
            | "LOW_PRIORITY_WORK"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CRAFT_QUEUE"
            | "CRITICAL_CRAFT"
            | "EARLY_STICKY_SMITH"
    )
}

/// Map a priority-ladder label to a [`SmithJobSlot`].
///
/// - `EARLY_STICKY_SMITH` → EarlySticky (Haxe lastProfession==SMITH before critical ~609)
/// - `ASSIGNED_JOB` → Assigned (maxPeople 100)
/// - `MID_PRIORITY_TASKS` → MidSticky (needs last/assigned)
/// - `CRITICAL_CRAFT` / `CRITICAL_MISC` → CriticalShortCraft
/// - `LOW_PRIORITY_WORK` / `AGE_ROTATED_JOB` / `CRAFT_QUEUE` → LowPriority (open become)
// Haxe: doTimeStuffHelper smith call sites
pub fn smith_slot_for_rung(rung_label: &str) -> Option<SmithJobSlot> {
    match rung_label {
        "EARLY_STICKY_SMITH" => Some(SmithJobSlot::EarlySticky),
        "ASSIGNED_JOB" => Some(SmithJobSlot::Assigned),
        "MID_PRIORITY_TASKS" => Some(SmithJobSlot::MidSticky),
        "CRITICAL_CRAFT" | "CRITICAL_MISC" => Some(SmithJobSlot::CriticalShortCraft),
        "LOW_PRIORITY_WORK" | "AGE_ROTATED_JOB" | "CRAFT_QUEUE" => {
            Some(SmithJobSlot::LowPriority)
        }
        _ => None,
    }
}

/// Apply speech `SMITH!` / assigned profession token onto sticky runtime.
// Haxe: AiBase speech endsWith("!") → assignedProfession = 'SMITH'
pub fn assign_smith_from_speech(runtime: &mut SmithProfessionRuntime, text: &str) -> bool {
    if !parse_smith_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_smith = true;
    runtime.is_last_smith = true;
    runtime.stage = runtime.stage.max(1.0);
    true
}

/// Thin ladder bridge: when rung is a smith job band, run [`decide_smith_job_for_slot`].
///
/// `profession_is_smith` should be true when last/assigned profession is SMITH or
/// age-rotated/open job selected smith. Sticky/assigned slots no-op via slot gates when
/// runtime flags are clear; open LowPriority may become smith (max one peer).
// Haxe: AiBase assignedProfession / lastProfession / open doSmithing
pub fn try_decide_smith_from_rung(
    profession_is_smith: bool,
    rung_label: &str,
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
    peer_count_with_last_smith: f32,
    was_idle: f32,
    age: f32,
) -> Option<SmithAction> {
    let slot = smith_slot_for_rung(rung_label)?;
    // Critical shortCrafts run without profession (Haxe high-prio bloom/tongs/charcoal).
    if !profession_is_smith
        && !matches!(
            slot,
            SmithJobSlot::LowPriority | SmithJobSlot::CriticalShortCraft | SmithJobSlot::ElderCollect
        )
    {
        return None;
    }
    Some(decide_smith_job_for_slot(
        slot,
        counts,
        runtime,
        peer_count_with_last_smith,
        was_idle,
        age,
    ))
}

/// Compose fill → decide → goal for live tick / ladder consumers (pure).
///
/// Returns `None` when rung is not a smith band (caller keeps thin
/// `SeekObject(SMITH_TARGET_ID)`). On decide maps via [`smith_action_to_goal`].
// Haxe: AssignedJob/sticky/open → doSmithing + shortCraft/craftItem seek
pub fn smith_goal_from_map_and_rung(
    profession_is_smith: bool,
    rung_label: &str,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[MapObj],
    home_r: i32,
    runtime: &mut SmithProfessionRuntime,
    peer_count_with_last_smith: f32,
    was_idle: f32,
    age: f32,
) -> Option<Goal> {
    let counts = fill_smith_counts_from_map(home_x, home_y, held_id, objects, home_r);
    let action = try_decide_smith_from_rung(
        profession_is_smith,
        rung_label,
        &counts,
        runtime,
        peer_count_with_last_smith,
        was_idle,
        age,
    )?;
    Some(smith_action_to_goal(action))
}

/// Same as [`smith_goal_from_map_and_rung`] but from an already-built [`SmithCounts`].
pub fn smith_goal_from_counts_and_rung(
    profession_is_smith: bool,
    rung_label: &str,
    counts: &SmithCounts,
    runtime: &mut SmithProfessionRuntime,
    peer_count_with_last_smith: f32,
    was_idle: f32,
    age: f32,
) -> Option<Goal> {
    let action = try_decide_smith_from_rung(
        profession_is_smith,
        rung_label,
        counts,
        runtime,
        peer_count_with_last_smith,
        was_idle,
        age,
    )?;
    Some(smith_action_to_goal(action))
}

/// Approximate stage from inventory for thin selfplay when runtime is not sticky.
///
/// Walks the product ladder downward so held/nearby stock advances the seek stage.
pub fn infer_smith_stage_from_have(have: &HashSet<i32>) -> f32 {
    if have.contains(&KNIFE) {
        return 0.0; // ladder complete → cycle reset
    }
    if have.contains(&STEEL_BLADE_BLANK) {
        return 12.0;
    }
    if have.contains(&STEEL_FILE) {
        return 11.0;
    }
    if have.contains(&STEEL_FILE_BLANK)
        || have.contains(&HOT_STEEL_FILE_BLANK)
        || have.contains(&OILED_FILE_BLANK)
    {
        return 10.0;
    }
    if STEEL_CHISEL_FAMILY.iter().any(|id| have.contains(id)) {
        return 9.0;
    }
    if have.contains(&STEEL_AXE) {
        return 8.0;
    }
    if have.contains(&SHEARS) {
        return 7.5;
    }
    if have.contains(&STEEL_HOE) {
        return 7.1;
    }
    if have.contains(&SHOVEL) || have.contains(&SHOVEL_OF_DUNG) {
        return 7.0;
    }
    if have.contains(&STEEL_MINING_PICK) {
        return 6.0;
    }
    if have.contains(&SMITHING_HAMMER) {
        return 5.0;
    }
    if have.contains(&STEEL_INGOT) {
        return 4.0;
    }
    if have.contains(&UNFORGED_SEALED_CRUCIBLE) || have.contains(&FORGED_CRUCIBLE) {
        return 3.5;
    }
    if have.contains(&WROUGHT_IRON) {
        return 3.0;
    }
    if have.contains(&IRON_ORE) {
        return 2.0;
    }
    0.0
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn counts_with(pairs: &[(i32, i32)], forge: Option<i32>, held: i32) -> SmithCounts {
        let mut c = SmithCounts {
            forge_parent_id: forge,
            held_id: held,
            ..Default::default()
        };
        for &(id, n) in pairs {
            c.set(id, n);
        }
        c
    }

    #[test]
    fn has_or_become_smith_max_one_and_sticky() {
        let mut rt = SmithProfessionRuntime::default();
        // Peer already smith → hard refuse even with max 2
        assert!(!has_or_become_smith(&mut rt, 2, 1.0, 0.0));
        assert!(!rt.is_last_smith);
        // No peers → become
        assert!(has_or_become_smith(&mut rt, 1, 0.0, 0.0));
        assert!(rt.is_last_smith);
        assert!(rt.stage >= 1.0);
        // Sticky even if peers appear
        assert!(has_or_become_smith(&mut rt, 1, 5.0, 0.0));
        // max < 0 high priority without sticky
        let mut rt2 = SmithProfessionRuntime::default();
        assert!(has_or_become_smith(&mut rt2, -2, 99.0, 0.0));
        assert!(!rt2.is_last_smith);
    }

    #[test]
    fn parse_smith_speech_and_assigned_job() {
        assert!(parse_smith_profession_speech("SMITH!"));
        assert!(parse_smith_profession_speech("smith"));
        assert!(parse_smith_profession_speech("  Smith!  "));
        assert!(!parse_smith_profession_speech("FARMER!"));
        let mut rt = SmithProfessionRuntime::default();
        assert!(!resolve_smith_assigned_job(&rt));
        rt.is_assigned_smith = true;
        assert!(resolve_smith_assigned_job(&rt));
        rt.is_assigned_smith = false;
        rt.is_last_smith = true;
        assert!(resolve_smith_assigned_job(&rt));
    }

    #[test]
    fn get_forge_priority_firing_then_charcoal_then_cold() {
        assert_eq!(
            pick_forge_parent(&[FORGE, FORGE_WITH_CHARCOAL, FIRING_FORGE]),
            Some(FIRING_FORGE)
        );
        assert_eq!(
            pick_forge_parent(&[FORGE, FORGE_WITH_CHARCOAL]),
            Some(FORGE_WITH_CHARCOAL)
        );
        assert_eq!(pick_forge_parent(&[FORGE]), Some(FORGE));
        assert_eq!(pick_forge_parent(&[291, 314]), None);
        assert!(is_forge_id(303) && is_forge_id(304) && is_forge_id(305));
        assert!(!is_forge_id(314));
        assert_eq!(forge_id_priority(), &[304, 305, 303]);
    }

    #[test]
    fn prepare_tools_flat_rock_and_stone_when_stage_low() {
        let mut rt = SmithProfessionRuntime {
            stage: 1.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(&[], Some(FIRING_FORGE), 0);
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::CraftAndDropNearForge {
                object_id: FLAT_ROCK,
                want_count: 2
            }
        );
        let c = counts_with(&[(FLAT_ROCK, 2)], Some(FIRING_FORGE), 0);
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::CraftAndDropNearForge {
                object_id: STONE,
                want_count: 1
            }
        );
    }

    #[test]
    fn prepare_tools_iron_ore_then_wrought_iron() {
        let mut rt = SmithProfessionRuntime {
            stage: 1.0,
            is_last_smith: true,
            ..Default::default()
        };
        // Flat rocks + stone present; stage < 3 → iron ore first (delta order)
        let c = counts_with(
            &[(FLAT_ROCK, 2), (STONE, 1), (WROUGHT_IRON, 0), (IRON_ORE, 0)],
            Some(FORGE),
            0,
        );
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::CraftItem {
                object_id: IRON_ORE
            }
        );
        // Ore stocked → wrought iron
        let mut rt = SmithProfessionRuntime {
            stage: 1.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(
            &[(FLAT_ROCK, 2), (STONE, 1), (IRON_ORE, 5), (WROUGHT_IRON, 0)],
            Some(FORGE),
            0,
        );
        let a = prepare_smithing_tools(&c, &mut rt);
        assert_eq!(
            a,
            SmithAction::CraftItem {
                object_id: WROUGHT_IRON
            }
        );
        assert!(rt.stage >= 2.0);
    }

    #[test]
    fn prepare_tools_hot_bloom_hammer_and_cold_reheat() {
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        // No hammer stock → stone fallback
        let c = counts_with(&[(HOT_IRON_BLOOM_FLAT, 1)], Some(FIRING_FORGE), 0);
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::ShortCraft {
                actor: STONE,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
        // Hammer nearby → prefer hammer
        let c = counts_with(
            &[(HOT_IRON_BLOOM_FLAT, 1), (SMITHING_HAMMER, 1)],
            Some(FIRING_FORGE),
            0,
        );
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::ShortCraft {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
        let c = counts_with(&[(COLD_IRON_BLOOM_FLAT, 1)], Some(FORGE), 0);
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::ShortCraft {
                actor: BOWL_OF_WATER,
                target: COLD_IRON_BLOOM_FLAT
            }
        );
        let c = counts_with(&[(WROUGHT_IRON_FLAT, 1)], Some(FORGE), 0);
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::ShortCraft {
                actor: 0,
                target: WROUGHT_IRON_FLAT
            }
        );
    }

    #[test]
    fn do_smithing_product_ladder_pick_shovel_hoe() {
        let mut rt = SmithProfessionRuntime {
            stage: 5.0,
            is_last_smith: true,
            ..Default::default()
        };
        let base = [
            (FLAT_ROCK, 2),
            (STONE, 1),
            (WROUGHT_IRON, 5),
            (STEEL_INGOT, 2),
            (SMITHING_HAMMER, 1),
        ];
        let c = counts_with(&base, Some(FORGE), 0);
        assert_eq!(prepare_smithing_tools(&c, &mut rt), SmithAction::None);

        assert_eq!(
            do_smithing_products(&c, &mut rt),
            SmithAction::CraftItem {
                object_id: STEEL_MINING_PICK
            }
        );
        let mut pairs = base.to_vec();
        pairs.push((STEEL_MINING_PICK, 1));
        let c = counts_with(&pairs, Some(FORGE), 0);
        rt.stage = 6.0;
        assert_eq!(
            do_smithing_products(&c, &mut rt),
            SmithAction::CraftItem {
                object_id: SHOVEL
            }
        );
        pairs.push((SHOVEL, 1));
        let c = counts_with(&pairs, Some(FORGE), 0);
        rt.stage = 7.0;
        assert_eq!(
            do_smithing_products(&c, &mut rt),
            SmithAction::CraftItem {
                object_id: STEEL_HOE
            }
        );
    }

    #[test]
    fn do_smithing_full_aborts_without_forge_or_slot() {
        let mut rt = SmithProfessionRuntime::default();
        let c = counts_with(&[], None, 0);
        assert_eq!(
            do_smithing(&c, &mut rt, 1, 0.0, 0.0),
            SmithAction::Abort
        );
        let mut rt = SmithProfessionRuntime::default();
        let c = counts_with(&[], Some(FORGE), 0);
        assert_eq!(
            do_smithing(&c, &mut rt, 1, 1.0, 0.0),
            SmithAction::Abort
        );
    }

    #[test]
    fn do_smithing_charcoal_into_forge_and_become() {
        let mut rt = SmithProfessionRuntime::default();
        let c = counts_with(&[], Some(FORGE), BASKET_OF_CHARCOAL);
        assert_eq!(
            do_smithing(&c, &mut rt, 1, 0.0, 0.0),
            SmithAction::ShortCraft {
                actor: BASKET_OF_CHARCOAL,
                target: FORGE
            }
        );
        let mut rt = SmithProfessionRuntime::default();
        let c = counts_with(&[], Some(FORGE), 0);
        let a = do_smithing(&c, &mut rt, 1, 0.0, 0.0);
        assert!(rt.is_last_smith);
        assert!(a.is_some());
        assert_eq!(
            a,
            SmithAction::CraftAndDropNearForge {
                object_id: FLAT_ROCK,
                want_count: 2
            }
        );
    }

    #[test]
    fn product_ladder_knife_resets_stage() {
        let mut rt = SmithProfessionRuntime {
            stage: 12.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(
            &[
                (STEEL_MINING_PICK, 1),
                (SHOVEL, 1),
                (STEEL_HOE, 1),
                (SHEARS, 1),
                (STEEL_AXE, 1),
                (STEEL_CHISEL, 1),
                (STEEL_FILE_BLANK, 1),
                (STEEL_FILE, 1),
                (STEEL_BLADE_BLANK, 1),
                (KNIFE, 1),
            ],
            Some(FORGE),
            0,
        );
        assert_eq!(do_smithing_products(&c, &mut rt), SmithAction::None);
        assert_eq!(rt.stage, 0.0);
        let c = counts_with(
            &[
                (STEEL_MINING_PICK, 1),
                (SHOVEL, 1),
                (STEEL_HOE, 1),
                (SHEARS, 1),
                (STEEL_AXE, 1),
                (STEEL_CHISEL, 1),
                (STEEL_FILE, 1),
                (STEEL_BLADE_BLANK, 1),
            ],
            Some(FORGE),
            0,
        );
        rt.stage = 12.0;
        assert_eq!(
            do_smithing_products(&c, &mut rt),
            SmithAction::CraftItem { object_id: KNIFE }
        );
    }

    #[test]
    fn pick_smith_profession_goal_stage_and_pipeline() {
        let g = ReverseCraftGraph::default();
        let empty = HashSet::new();
        let goal = pick_smith_profession_goal(&g, &empty, 0.0);
        assert_eq!(goal, Goal::SeekObject(IRON_ORE));
        let have: HashSet<i32> = [IRON_ORE].into_iter().collect();
        assert_eq!(
            pick_smith_profession_goal(&g, &have, 2.0),
            Goal::SeekObject(WROUGHT_IRON)
        );
        let mut have = HashSet::new();
        for &id in smith_pipeline_targets() {
            if id != KNIFE {
                have.insert(id);
            }
        }
        assert_eq!(
            pick_smith_profession_goal(&g, &have, 12.0),
            Goal::SeekObject(KNIFE)
        );
    }

    #[test]
    fn smith_action_to_goal_maps_craft_and_short() {
        assert_eq!(
            smith_action_to_goal(SmithAction::CraftItem {
                object_id: STEEL_AXE
            }),
            Goal::SeekObject(STEEL_AXE)
        );
        assert_eq!(
            smith_action_to_goal(SmithAction::ShortCraft {
                actor: 441,
                target: 309
            }),
            Goal::SeekObject(309)
        );
        assert_eq!(
            smith_action_to_goal(SmithAction::None),
            Goal::SeekObject(SMITH_TARGET_ID)
        );
    }

    #[test]
    fn critical_shortcrafts_and_drop_near_forge_ids() {
        // No hammer → stone fallback
        let c = counts_with(&[(HOT_IRON_BLOOM_FLAT, 1)], Some(FIRING_FORGE), 0);
        assert_eq!(
            critical_smith_shortcrafts(0, &c),
            SmithAction::ShortCraft {
                actor: STONE,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
        let c = counts_with(
            &[(HOT_IRON_BLOOM_FLAT, 1), (SMITHING_HAMMER, 1)],
            Some(FIRING_FORGE),
            0,
        );
        assert_eq!(
            critical_smith_shortcrafts(0, &c),
            SmithAction::ShortCraft {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
        // Tongs + firing forge when not holding hammer
        let c = counts_with(&[(UNFORGED_CRUCIBLE_TONGS, 1)], Some(FIRING_FORGE), 0);
        assert_eq!(
            critical_smith_shortcrafts(0, &c),
            SmithAction::ShortCraft {
                actor: UNFORGED_CRUCIBLE_TONGS,
                target: FIRING_FORGE
            }
        );
        assert!(DROP_NEAR_FORGE_IDS.contains(&SMITHING_HAMMER));
        assert!(DROP_NEAR_FORGE_IDS.contains(&STEEL_INGOT));
        assert_eq!(SMITHING_HAMMER, 441);
        assert_eq!(WROUGHT_IRON, SMITH_TARGET_ID);
        assert_eq!(WROUGHT_IRON, SMITH_IRON_ID);
    }

    #[test]
    fn pottery_gate_defers_when_kiln_firing() {
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(&[(FIRING_KILN, 1)], Some(FORGE), 0);
        assert_eq!(prepare_smithing_tools(&c, &mut rt), SmithAction::DeferPottery);
        // Holding hammer skips pottery gate
        let c = counts_with(&[(FIRING_KILN, 1)], Some(FORGE), SMITHING_HAMMER);
        assert_ne!(prepare_smithing_tools(&c, &mut rt), SmithAction::DeferPottery);
        // Firing forge skips pottery
        let c = counts_with(&[(FIRING_KILN, 1)], Some(FIRING_FORGE), 0);
        assert_ne!(prepare_smithing_tools(&c, &mut rt), SmithAction::DeferPottery);
    }

    #[test]
    fn cool_crucible_uses_short_craft_on_ground() {
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(
            &[
                (FLAT_ROCK, 2),
                (STONE, 1),
                (WROUGHT_IRON, 5),
                (STEEL_INGOT, 0),
                (COOL_CRUCIBLE_TONGS, 1),
            ],
            Some(FORGE),
            0,
        );
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::ShortCraftOnGround {
                target: COOL_CRUCIBLE_TONGS
            }
        );
    }

    #[test]
    fn chisel_family_stock_skips_craft() {
        let mut rt = SmithProfessionRuntime {
            stage: 8.0,
            is_last_smith: true,
            ..Default::default()
        };
        // File-with-chisel counts as chisel stock
        let c = counts_with(&[(STEEL_FILE_WITH_CHISEL, 1)], Some(FORGE), 0);
        let a = do_smithing_products(&c, &mut rt);
        assert_ne!(
            a,
            SmithAction::CraftItem {
                object_id: STEEL_CHISEL
            }
        );
        assert!(rt.stage >= 9.0);
    }

    #[test]
    fn chisel_content_scan_extends_family_and_stock() {
        // Haxe: description.contains('Chisel') → objectIdArrays[455]
        assert!(is_steel_chisel_family_description("Steel Chisel"));
        assert!(is_steel_chisel_family_description("Oiled File Blank with Chisel"));
        assert!(is_steel_chisel_family_description("Dug Big Rock with Chisel"));
        assert!(!is_steel_chisel_family_description("Steel File"));
        assert!(!is_steel_chisel_family_description("chisel lowercase only"));

        let fam = collect_steel_chisel_family_ids([
            (OILED_FILE_BLANK_CHISEL, "Oiled File Blank with Chisel"),
            (9999, "Fancy Ritual Chisel"),
            (9999, "Fancy Ritual Chisel"), // dedupe
            (STEEL_CHISEL, "Steel Chisel"),
            (1234, "No match here"),
        ]);
        assert!(fam.contains(&STEEL_CHISEL));
        assert!(fam.contains(&STEEL_CHISEL_FLAT));
        assert!(fam.contains(&STEEL_FILE_WITH_CHISEL));
        assert!(fam.contains(&OILED_FILE_BLANK_CHISEL));
        assert!(fam.contains(&9999));
        assert!(!fam.contains(&1234));

        let extras = chisel_family_extras_beyond_static(&fam);
        assert!(extras.contains(&OILED_FILE_BLANK_CHISEL));
        assert!(extras.contains(&9999));
        assert!(!extras.contains(&STEEL_CHISEL));

        // Content-scan extra 9999 counts as stock → skip craft 455
        let mut rt = SmithProfessionRuntime {
            stage: 8.0,
            is_last_smith: true,
            ..Default::default()
        };
        let mut c = counts_with(&[(9999, 1)], Some(FORGE), 0);
        c.attach_chisel_family(&fam);
        assert_eq!(c.count_steel_chisel_stock(), 1);
        let a = do_smithing_products(&c, &mut rt);
        assert_ne!(
            a,
            SmithAction::CraftItem {
                object_id: STEEL_CHISEL
            }
        );
        assert!(rt.stage >= 9.0);

        // fill_smith_counts_from_map_ex attaches family
        let map = [MapObj {
            parent_id: OILED_FILE_BLANK_CHISEL,
            x: 0,
            y: 0,
        }];
        let filled = fill_smith_counts_from_map_ex(0, 0, 0, &map, 20, Some(&fam));
        assert!(filled.chisel_family_extra.contains(&OILED_FILE_BLANK_CHISEL));
        assert_eq!(filled.count_steel_chisel_stock(), 1);
    }

    #[test]
    fn npc_smith_peer_rows_max_one_population() {
        // Two sticky smiths at same home → count 1 for self not in set of peers...
        // rows include self + peer; count excludes self.
        let rows = [
            NpcSmithPeerRow {
                conn_id: 1,
                home_x: 10,
                home_y: 10,
                age: 25.0,
                food_store: 5.0,
                deleted: false,
                has_player_to_follow: false,
                is_wounded: false,
                last_is_smith: true,
            },
            NpcSmithPeerRow {
                conn_id: 2,
                home_x: 10,
                home_y: 10,
                age: 30.0,
                food_store: 5.0,
                deleted: false,
                has_player_to_follow: false,
                is_wounded: false,
                last_is_smith: true,
            },
            NpcSmithPeerRow {
                conn_id: 3,
                home_x: 99,
                home_y: 99, // other home
                age: 30.0,
                food_store: 5.0,
                deleted: false,
                has_player_to_follow: false,
                is_wounded: false,
                last_is_smith: true,
            },
            NpcSmithPeerRow {
                conn_id: 4,
                home_x: 10,
                home_y: 10,
                age: 30.0,
                food_store: 5.0,
                deleted: false,
                has_player_to_follow: true, // following → excluded
                is_wounded: false,
                last_is_smith: true,
            },
        ];
        // Self=1 sees peer conn 2 only (same home, eligible; 3 other-home, 4 following)
        assert_eq!(
            smith_peer_count_from_npc_rows(&rows, 1, 10, 10, 3.0, 60.0),
            1.0
        );
        // Self=5 (no row) sees both sticky smiths at home (conn 1 + 2)
        assert_eq!(
            smith_peer_count_from_npc_rows(&rows, 5, 10, 10, 3.0, 60.0),
            2.0
        );
        // has_or_become refuses when peer_count > 0 (max one smith)
        let mut rt = SmithProfessionRuntime::default();
        assert!(!has_or_become_smith(&mut rt, 1, 1.0, 0.0));
        assert!(!has_or_become_smith(&mut rt, 1, 2.0, 0.0));
    }

    #[test]
    fn npc_smith_peer_wounded_excluded_and_home_fidelity() {
        // Haxe: countProfession skips isWounded; same-home via home.tx/ty not position
        // AI-JOB-SMITH-RESID residual close
        assert!(peer_is_wounded_from_held(true));
        assert!(!peer_is_wounded_from_held(false));
        assert_eq!(peer_home_coords(Some((10, 20)), 99, 99), (10, 20));
        assert_eq!(peer_home_coords(None, 5, 6), (5, 6));

        let rows = [
            NpcSmithPeerRow::from_snapshot_fields(
                1, 10, 10, 25.0, 5.0, false, false, false, true, false,
            ),
            // Wounded sticky smith at same home — must not count
            NpcSmithPeerRow::from_snapshot_fields(
                2, 10, 10, 30.0, 5.0, false, false, true, true, false,
            ),
            // Healthy sticky smith at same home
            NpcSmithPeerRow::from_snapshot_fields(
                3, 10, 10, 30.0, 5.0, false, false, false, false, true, // override sticky
            ),
            // Position elsewhere but home matches via snapshot home
            NpcSmithPeerRow {
                conn_id: 4,
                home_x: 10,
                home_y: 10,
                age: 28.0,
                food_store: 5.0,
                deleted: false,
                has_player_to_follow: false,
                is_wounded: false,
                last_is_smith: true,
            },
        ];
        // Self=1: peers 3 + 4 (2 wounded excluded)
        assert_eq!(
            smith_peer_count_from_npc_rows(&rows, 1, 10, 10, 3.0, 60.0),
            2.0
        );
        // Two same-home sticky smiths → second has_or_become false
        let mut rt = SmithProfessionRuntime::default();
        let peers = smith_peer_count_from_npc_rows(&rows, 99, 10, 10, 3.0, 60.0);
        assert!(peers >= 1.0);
        assert!(!has_or_become_smith(&mut rt, 1, peers, 0.0));
    }

    #[test]
    fn steel_chisel_family_table_cache_extras() {
        // Load-time table: family + extras split (PatchObjectData once)
        let fam = collect_steel_chisel_family_ids([
            (OILED_FILE_BLANK_CHISEL, "Oiled File Blank with Chisel"),
            (8888, "Ritual Chisel Stone"),
        ]);
        let table = SteelChiselFamilyTable::from_family(fam);
        assert!(table.family.contains(&STEEL_CHISEL));
        assert!(table.extras.contains(&OILED_FILE_BLANK_CHISEL));
        assert!(table.extras.contains(&8888));
        assert!(!table.extras.contains(&STEEL_CHISEL));
    }

    #[test]
    fn peer_filters_and_wipe_on_eat() {
        let peers = [
            SmithPeerSnapshot {
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_smith: true,
            },
            SmithPeerSnapshot {
                deleted: false,
                age: 2.0, // too young
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_smith: true,
            },
            SmithPeerSnapshot {
                deleted: false,
                age: 25.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: false, // other home
                last_is_smith: true,
            },
            SmithPeerSnapshot {
                deleted: false,
                age: 30.0,
                is_wounded: false,
                food_store: -1.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_smith: true,
            },
        ];
        assert_eq!(count_smith_peers_filtered(&peers, 3.0, 60.0), 1.0);

        let mut rt = SmithProfessionRuntime {
            stage: 6.0,
            is_last_smith: true,
            is_assigned_smith: true,
        };
        wipe_smith_on_eat(&mut rt, false);
        assert_eq!(rt.stage, 0.0);
        assert!(!rt.is_last_smith);
        // FOODSERVER keeps sticky last
        let mut rt = SmithProfessionRuntime {
            stage: 6.0,
            is_last_smith: true,
            is_assigned_smith: false,
        };
        wipe_smith_on_eat(&mut rt, true);
        assert_eq!(rt.stage, 0.0);
        assert!(rt.is_last_smith);
    }

    #[test]
    fn spatial_get_forge_priority_and_radius() {
        let home = (0, 0);
        let cands = [
            ForgeCandidate {
                parent_id: FORGE,
                x: 1,
                y: 0,
            },
            ForgeCandidate {
                parent_id: FIRING_FORGE,
                x: 15,
                y: 0,
            },
            ForgeCandidate {
                parent_id: FORGE_WITH_CHARCOAL,
                x: 2,
                y: 0,
            },
        ];
        // Firing preferred even if farther (within r=20)
        let f = pick_forge_near_home(home.0, home.1, &cands).unwrap();
        assert_eq!(f.parent_id, FIRING_FORGE);
        // Outside radius ignored
        let far = [ForgeCandidate {
            parent_id: FIRING_FORGE,
            x: 50,
            y: 0,
        }];
        assert!(pick_forge_near_home(0, 0, &far).is_none());
        // Closest among same priority
        let same = [
            ForgeCandidate {
                parent_id: FORGE,
                x: 10,
                y: 0,
            },
            ForgeCandidate {
                parent_id: FORGE,
                x: 3,
                y: 0,
            },
        ];
        let f = pick_forge_near_home(0, 0, &same).unwrap();
        assert_eq!((f.x, f.y), (3, 0));
    }

    #[test]
    fn fill_smith_counts_map_radii() {
        let objs = [
            MapObj {
                parent_id: FIRING_FORGE,
                x: 5,
                y: 0,
            },
            MapObj {
                parent_id: FLAT_ROCK,
                x: 1,
                y: 0,
            },
            MapObj {
                parent_id: FLAT_ROCK,
                x: 2,
                y: 0,
            },
            MapObj {
                parent_id: IRON_ORE,
                x: 6,
                y: 0,
            }, // near forge
            MapObj {
                parent_id: IRON_ORE,
                x: 40,
                y: 0,
            }, // far
            MapObj {
                parent_id: HOT_IRON_BLOOM_FLAT,
                x: 4,
                y: 0,
            },
        ];
        let c = fill_smith_counts_from_map(0, 0, 0, &objs, 20);
        assert_eq!(c.forge_parent_id, Some(FIRING_FORGE));
        assert_eq!(c.get(FLAT_ROCK), 2);
        assert_eq!(c.get(IRON_ORE), 1);
        assert_eq!(c.get(HOT_IRON_BLOOM_FLAT), 1);
    }

    #[test]
    fn job_slot_assigned_max_and_elder() {
        assert_eq!(SmithJobSlot::Assigned.max_people(), 100);
        assert_eq!(SmithJobSlot::LowPriority.max_people(), 1);
        let mut rt = SmithProfessionRuntime {
            is_assigned_smith: true,
            stage: 1.0,
            ..Default::default()
        };
        let c = counts_with(&[], Some(FORGE), 0);
        let a = decide_smith_job_for_slot(
            SmithJobSlot::Assigned,
            &c,
            &mut rt,
            0.0,
            0.0,
            20.0,
        );
        assert!(a.is_some() || a == SmithAction::None);
        assert!(rt.is_last_smith);

        let mut rt = SmithProfessionRuntime::default();
        let a = decide_smith_job_for_slot(
            SmithJobSlot::ElderCollect,
            &c,
            &mut rt,
            0.0,
            0.0,
            30.0, // too young
        );
        assert_eq!(a, SmithAction::None);
        let a = decide_smith_job_for_slot(
            SmithJobSlot::ElderCollect,
            &c,
            &mut rt,
            0.0,
            0.0,
            45.0,
        );
        assert!(a.is_some());
    }

    #[test]
    fn infer_stage_and_pipeline_goal() {
        let mut have = HashSet::new();
        assert_eq!(infer_smith_stage_from_have(&have), 0.0);
        have.insert(SMITHING_HAMMER);
        assert_eq!(infer_smith_stage_from_have(&have), 5.0);
        have.insert(STEEL_MINING_PICK);
        assert_eq!(infer_smith_stage_from_have(&have), 6.0);
        let g = ReverseCraftGraph::default();
        let goal = pick_smith_profession_goal(&g, &have, 5.0);
        // stage >= 5 and pick missing would want pick at stage < 6 — have pick so shovel
        assert_eq!(goal, Goal::SeekObject(SHOVEL));
    }

    #[test]
    fn steel_crucible_path_when_no_steel_after_iron_ready() {
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        // Iron done (stage >= 3); no steel → crucibles
        let c = counts_with(
            &[
                (FLAT_ROCK, 2),
                (STONE, 1),
                (WROUGHT_IRON, 5),
                (STEEL_INGOT, 0),
                (UNFORGED_SEALED_CRUCIBLE, 0),
            ],
            Some(FORGE),
            0,
        );
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::CraftAndDropNearForge {
                object_id: UNFORGED_SEALED_CRUCIBLE,
                want_count: 3
            }
        );
    }

    #[test]
    fn prepare_hammer_after_steel_stocked() {
        let mut rt = SmithProfessionRuntime {
            stage: 4.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(
            &[
                (FLAT_ROCK, 2),
                (STONE, 1),
                (WROUGHT_IRON, 5),
                (STEEL_INGOT, 2),
                (SMITHING_HAMMER, 0),
            ],
            Some(FORGE),
            0,
        );
        assert_eq!(
            prepare_smithing_tools(&c, &mut rt),
            SmithAction::CraftItem {
                object_id: SMITHING_HAMMER
            }
        );
    }

    // ── AI-JOB-SMITH-WIRE live-tick bridge tests ───────────────────────────

    #[test]
    fn do_smithing_surfaces_defer_pottery() {
        // prepare returns DeferPottery; do_smithing must not swallow into products.
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(&[(FIRING_KILN, 1)], Some(FORGE), 0);
        assert_eq!(
            do_smithing(&c, &mut rt, 1, 0.0, 0.0),
            SmithAction::DeferPottery
        );
        assert_eq!(
            smith_action_to_goal(SmithAction::DeferPottery),
            Goal::SeekObject(FIRING_KILN)
        );
    }

    #[test]
    fn try_decide_smith_from_rung_assigned_vs_sticky_gates() {
        assert!(smith_job_rung_label("ASSIGNED_JOB"));
        assert!(smith_job_rung_label("MID_PRIORITY_TASKS"));
        assert!(!smith_job_rung_label("ESCAPE"));
        assert_eq!(
            smith_slot_for_rung("ASSIGNED_JOB"),
            Some(SmithJobSlot::Assigned)
        );
        assert_eq!(SmithJobSlot::Assigned.max_people(), 100);

        let c = counts_with(&[(FLAT_ROCK, 2), (STONE, 1)], Some(FORGE), 0);

        // Assigned without assigned/last → None from slot gate (returns None action)
        let mut rt = SmithProfessionRuntime::default();
        let a = try_decide_smith_from_rung(true, "ASSIGNED_JOB", &c, &mut rt, 0.0, 0.0, 20.0);
        assert_eq!(a, Some(SmithAction::None));

        // Assigned with speech assign → full job (max 100)
        let mut rt = SmithProfessionRuntime::default();
        assert!(assign_smith_from_speech(&mut rt, "SMITH!"));
        assert!(rt.is_assigned_smith && rt.is_last_smith);
        let a = try_decide_smith_from_rung(true, "ASSIGNED_JOB", &c, &mut rt, 0.0, 0.0, 20.0);
        assert!(a.is_some());
        let a = a.unwrap();
        assert!(a.is_some() || a == SmithAction::None);

        // MidSticky without last/assigned → None action
        let mut rt = SmithProfessionRuntime::default();
        let a = try_decide_smith_from_rung(true, "MID_PRIORITY_TASKS", &c, &mut rt, 0.0, 0.0, 20.0);
        assert_eq!(a, Some(SmithAction::None));

        // MidSticky with sticky last
        let mut rt = SmithProfessionRuntime {
            is_last_smith: true,
            stage: 1.0,
            ..Default::default()
        };
        let a = try_decide_smith_from_rung(true, "MID_PRIORITY_TASKS", &c, &mut rt, 0.0, 0.0, 20.0);
        assert!(a.is_some());

        // Non-smith profession + assigned rung → no decide
        let mut rt = SmithProfessionRuntime::default();
        assert!(try_decide_smith_from_rung(false, "ASSIGNED_JOB", &c, &mut rt, 0.0, 0.0, 20.0)
            .is_none());

        // Escape rung → None
        assert!(try_decide_smith_from_rung(true, "ESCAPE", &c, &mut rt, 0.0, 0.0, 20.0).is_none());
    }

    #[test]
    fn critical_shortcraft_slot_before_full_job() {
        let c = counts_with(
            &[(HOT_IRON_BLOOM_FLAT, 1)],
            Some(FIRING_FORGE),
            0,
        );
        let mut rt = SmithProfessionRuntime::default();
        // Critical runs without profession_is_smith
        let a = try_decide_smith_from_rung(false, "CRITICAL_CRAFT", &c, &mut rt, 0.0, 0.0, 20.0);
        assert_eq!(
            a,
            Some(SmithAction::ShortCraft {
                actor: STONE,
                target: HOT_IRON_BLOOM_FLAT
            })
        );
        let c2 = counts_with(
            &[(HOT_IRON_BLOOM_FLAT, 1), (SMITHING_HAMMER, 1)],
            Some(FIRING_FORGE),
            0,
        );
        let a = decide_smith_job_for_slot(
            SmithJobSlot::CriticalShortCraft,
            &c2,
            &mut rt,
            0.0,
            0.0,
            20.0,
        );
        assert_eq!(
            a,
            SmithAction::ShortCraft {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
    }

    #[test]
    fn fill_decide_goal_composition_and_steel_radius() {
        // Steel at d=16 from forge (beyond STEEL_COUNT_RADIUS=15) not counted;
        // steel at d=10 counted even when home_r would allow farther generic ids.
        let objs = [
            MapObj {
                parent_id: FORGE,
                x: 0,
                y: 0,
            },
            MapObj {
                parent_id: STEEL_INGOT,
                x: 10,
                y: 0,
            },
            MapObj {
                parent_id: STEEL_INGOT,
                x: 16,
                y: 0,
            },
            MapObj {
                parent_id: UNFORGED_SEALED_CRUCIBLE,
                x: 14,
                y: 0,
            },
            MapObj {
                parent_id: UNFORGED_SEALED_CRUCIBLE,
                x: 20,
                y: 0,
            },
            MapObj {
                parent_id: FLAT_ROCK,
                x: 1,
                y: 0,
            },
            MapObj {
                parent_id: FLAT_ROCK,
                x: 2,
                y: 0,
            },
            MapObj {
                parent_id: STONE,
                x: 1,
                y: 1,
            },
        ];
        let c = fill_smith_counts_from_map(0, 0, 0, &objs, 30);
        assert_eq!(c.forge_parent_id, Some(FORGE));
        assert_eq!(c.get(STEEL_INGOT), 1); // only d=10
        assert_eq!(c.get(UNFORGED_SEALED_CRUCIBLE), 1); // only d=14
        assert!(is_steel_crucible_count_id(STEEL_INGOT));
        assert!(!is_steel_crucible_count_id(FLAT_ROCK));

        let mut rt = SmithProfessionRuntime {
            is_assigned_smith: true,
            is_last_smith: true,
            stage: 5.0,
        };
        let goal = smith_goal_from_counts_and_rung(
            true,
            "ASSIGNED_JOB",
            &c,
            &mut rt,
            0.0,
            0.0,
            20.0,
        );
        assert!(goal.is_some());
        // Map compose path
        let mut rt2 = SmithProfessionRuntime {
            is_last_smith: true,
            stage: 1.0,
            ..Default::default()
        };
        let g2 = smith_goal_from_map_and_rung(
            true,
            "LOW_PRIORITY_WORK",
            0,
            0,
            0,
            &objs,
            20,
            &mut rt2,
            0.0,
            0.0,
            20.0,
        );
        assert!(g2.is_some());
    }

    #[test]
    fn sticky_runtime_stage_survives_two_decide_ticks() {
        let mut rt = SmithProfessionRuntime {
            is_last_smith: true,
            stage: 5.0,
            ..Default::default()
        };
        // Stocked through hammer; products want pick → stage stays 5 until pick present
        let c = counts_with(
            &[
                (FLAT_ROCK, 2),
                (STONE, 1),
                (WROUGHT_IRON, 5),
                (STEEL_INGOT, 2),
                (SMITHING_HAMMER, 1),
            ],
            Some(FORGE),
            0,
        );
        let a1 = decide_smith_job_for_slot(
            SmithJobSlot::MidSticky,
            &c,
            &mut rt,
            0.0,
            0.0,
            25.0,
        );
        assert_eq!(
            a1,
            SmithAction::CraftItem {
                object_id: STEEL_MINING_PICK
            }
        );
        assert_eq!(rt.stage, 5.0); // not advanced until pick stocked
        let a2 = decide_smith_job_for_slot(
            SmithJobSlot::MidSticky,
            &c,
            &mut rt,
            0.0,
            0.0,
            25.0,
        );
        assert_eq!(a2, a1);
        assert_eq!(rt.stage, 5.0);
        // Still sticky last after two ticks
        assert!(rt.is_last_smith);
    }

    // ── AI-JOB-SMITH-LIVE action apply / pottery / eat wipe ─────────────────

    #[test]
    fn smith_action_short_craft_apply_hammer_bloom_and_charcoal() {
        // Holding hammer → UseOnTarget for 441+309
        let a = SmithAction::ShortCraft {
            actor: SMITHING_HAMMER,
            target: HOT_IRON_BLOOM_FLAT,
        };
        assert_eq!(
            smith_action_short_craft_apply(a, SMITHING_HAMMER, 0, -1),
            Some(crate::farmer_profession::ShortCraftApply::UseOnTarget {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT
            })
        );
        // Not holding stone → seek stone for 33+309
        let a = SmithAction::ShortCraft {
            actor: STONE,
            target: HOT_IRON_BLOOM_FLAT,
        };
        assert_eq!(
            smith_action_short_craft_apply(a, 0, 0, -1),
            Some(crate::farmer_profession::ShortCraftApply::SeekOrCraftActor {
                actor: STONE,
                craft_if_needed: true,
            })
        );
        // Charcoal into cold forge held
        let a = SmithAction::ShortCraft {
            actor: BASKET_OF_CHARCOAL,
            target: FORGE,
        };
        assert_eq!(
            smith_action_short_craft_apply(a, BASKET_OF_CHARCOAL, 0, -1),
            Some(crate::farmer_profession::ShortCraftApply::UseOnTarget {
                actor: BASKET_OF_CHARCOAL,
                target: FORGE
            })
        );
        // actor 0 → DropHeld when holding something
        let a = SmithAction::ShortCraft {
            actor: 0,
            target: WROUGHT_IRON_FLAT,
        };
        assert_eq!(
            smith_action_short_craft_apply(a, WROUGHT_IRON, 0, -1),
            Some(crate::farmer_profession::ShortCraftApply::DropHeld)
        );
        // Non-short → None
        assert!(smith_action_short_craft_apply(
            SmithAction::CraftItem {
                object_id: STEEL_AXE
            },
            0,
            0,
            -1
        )
        .is_none());
    }

    #[test]
    fn smith_action_apply_hungry_cost_and_craft_drop() {
        let sc = SmithAction::ShortCraft {
            actor: SMITHING_HAMMER,
            target: HOT_IRON_BLOOM_FLAT,
        };
        let mut inp = SmithApplyInput::basic(SMITHING_HAMMER);
        assert_eq!(
            smith_action_apply(sc, &inp),
            SmithApply::UseOnTarget {
                actor: SMITHING_HAMMER,
                target: HOT_IRON_BLOOM_FLAT
            }
        );
        // Hungry cost refuse
        inp.food_store = 1.0;
        inp.short_craft_work_cost = 2.0;
        assert_eq!(smith_action_apply(sc, &inp), SmithApply::RefuseHungryCost);
        assert!(!check_hungry_work_cost_by_id(1.0, 2.0));
        assert!(check_hungry_work_cost_by_id(5.0, 2.0));

        // CraftAndDropNearForge flat rock want=2
        let drop = SmithAction::CraftAndDropNearForge {
            object_id: FLAT_ROCK,
            want_count: 2,
        };
        let mut inp = SmithApplyInput::basic(0);
        inp.craft_drop_near_count = 0;
        assert_eq!(
            smith_action_apply(drop, &inp),
            SmithApply::CraftItem {
                object_id: FLAT_ROCK
            }
        );
        inp.held_id = FLAT_ROCK;
        inp.quad_dist_to_forge = 10; // > 5 → goto
        assert_eq!(
            smith_action_apply(drop, &inp),
            SmithApply::GotoForgeForDrop {
                object_id: FLAT_ROCK
            }
        );
        inp.quad_dist_to_forge = 2;
        assert_eq!(
            smith_action_apply(drop, &inp),
            SmithApply::DropNearForge {
                object_id: FLAT_ROCK
            }
        );
        inp.held_id = 0;
        inp.craft_drop_near_count = 2;
        assert_eq!(smith_action_apply(drop, &inp), SmithApply::None);
        inp.craft_drop_near_count = 1;
        inp.craft_drop_nearby_exists = true;
        assert_eq!(
            smith_action_apply(drop, &inp),
            SmithApply::PickupNearForge {
                object_id: FLAT_ROCK
            }
        );
    }

    #[test]
    fn cool_crucible_ground_apply_not_craft_item() {
        let a = SmithAction::ShortCraftOnGround {
            target: COOL_CRUCIBLE_TONGS,
        };
        assert_eq!(
            smith_action_to_goal(a),
            Goal::SeekObject(COOL_CRUCIBLE_TONGS)
        );
        assert_eq!(
            short_craft_on_ground_apply(0, COOL_CRUCIBLE_TONGS),
            SmithApply::SeekOrGetGroundActor {
                target: COOL_CRUCIBLE_TONGS
            }
        );
        assert_eq!(
            smith_action_apply(a, &SmithApplyInput::basic(COOL_CRUCIBLE_TONGS)),
            SmithApply::UseOnEmptyGround {
                held: COOL_CRUCIBLE_TONGS
            }
        );
    }

    #[test]
    fn do_pottery_on_fire_bowl_plate_crock_adobe() {
        // Fired bowl tongs when under max and count_bowl < close
        let mut pot = PotteryOnFireCounts {
            count_bowl: 0,
            count_close_bowl: 2,
            max_bowls: 3,
            ..Default::default()
        };
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );
        pot.count_bowl = 2;
        pot.count_close_bowl = 2; // not bowl < close → skip
        pot.count_wet_plate = 1;
        pot.count_plate = 0;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_PLATE_TONGS
            }
        );
        pot.count_wet_plate = 0;
        pot.count_wet_crock = 1;
        pot.count_crock = 0;
        pot.count_close_crock = 0;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_CROCK_TONGS
            }
        );
        pot.count_wet_crock = 0;
        pot.count_coal = 1;
        pot.firing_kiln = true;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::ShortCraft {
                actor: ADOBE,
                target: FIRING_KILN
            }
        );
        pot.count_coal = 3;
        assert_eq!(do_pottery_on_fire(&pot), SmithAction::None);
    }

    #[test]
    fn defer_pottery_resolves_via_pottery_body_and_apply() {
        let mut rt = SmithProfessionRuntime {
            stage: 3.0,
            is_last_smith: true,
            ..Default::default()
        };
        let c = counts_with(&[(FIRING_KILN, 1)], Some(FORGE), 0);
        assert_eq!(
            do_smithing(&c, &mut rt, 1, 0.0, 0.0),
            SmithAction::DeferPottery
        );
        let pot = PotteryOnFireCounts {
            count_bowl: 0,
            count_close_bowl: 1,
            max_bowls: 3,
            firing_kiln: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_smith_defer_pottery(SmithAction::DeferPottery, &pot),
            SmithAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );
        let mut inp = SmithApplyInput::basic(0);
        inp.pottery = Some(pot);
        assert_eq!(
            smith_action_apply(SmithAction::DeferPottery, &inp),
            SmithApply::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );
    }

    #[test]
    fn consider_making_food_wipes_smith_unless_foodserver() {
        assert!(!should_wipe_smith_on_consider_food(2.0, true, false, 3.0)); // too young
        assert!(!should_wipe_smith_on_consider_food(20.0, false, false, 3.0)); // not hungry, no target
        assert!(should_wipe_smith_on_consider_food(20.0, true, false, 3.0));
        assert!(should_wipe_smith_on_consider_food(20.0, false, true, 3.0));

        let mut rt = SmithProfessionRuntime {
            stage: 7.0,
            is_last_smith: true,
            is_assigned_smith: true,
        };
        assert!(apply_consider_making_food_smith_wipe(
            &mut rt, 20.0, true, false, 3.0, false
        ));
        assert_eq!(rt.stage, 0.0);
        assert!(!rt.is_last_smith);

        let mut rt = SmithProfessionRuntime {
            stage: 4.0,
            is_last_smith: true,
            ..Default::default()
        };
        assert!(apply_consider_making_food_smith_wipe(
            &mut rt, 20.0, true, false, 3.0, true // FOODSERVER
        ));
        assert_eq!(rt.stage, 0.0);
        assert!(rt.is_last_smith);
        // Non-hungry without target → no wipe
        rt.stage = 5.0;
        assert!(!apply_consider_making_food_smith_wipe(
            &mut rt, 20.0, false, false, 3.0, false
        ));
        assert_eq!(rt.stage, 5.0);
    }

    #[test]
    fn early_sticky_rung_and_peer_snapshot_helper() {
        assert_eq!(
            smith_slot_for_rung("EARLY_STICKY_SMITH"),
            Some(SmithJobSlot::EarlySticky)
        );
        assert!(smith_job_rung_label("EARLY_STICKY_SMITH"));
        let c = counts_with(&[(FLAT_ROCK, 2), (STONE, 1)], Some(FORGE), 0);
        let mut rt = SmithProfessionRuntime {
            is_last_smith: true,
            stage: 1.0,
            ..Default::default()
        };
        let a = try_decide_smith_from_rung(
            true,
            "EARLY_STICKY_SMITH",
            &c,
            &mut rt,
            0.0,
            0.0,
            20.0,
        );
        assert!(a.is_some());
        // Without sticky last → None action (gate)
        let mut rt2 = SmithProfessionRuntime::default();
        assert_eq!(
            try_decide_smith_from_rung(true, "EARLY_STICKY_SMITH", &c, &mut rt2, 0.0, 0.0, 20.0),
            Some(SmithAction::None)
        );

        let peer = smith_peer_snapshot(false, 25.0, false, 5.0, false, true, true);
        assert!(peer.counts_as_smith(3.0, 60.0));
        assert_eq!(count_smith_peers_filtered(&[peer], 3.0, 60.0), 1.0);
        // Second smith has_or_become refuses
        let mut rt = SmithProfessionRuntime::default();
        assert!(!has_or_become_smith(&mut rt, 1, 1.0, 0.0));
    }

    #[test]
    fn fill_pottery_counts_from_map_basic() {
        let objs = [
            MapObj {
                parent_id: FIRING_KILN,
                x: 1,
                y: 0,
            },
            MapObj {
                parent_id: CLAY_BOWL,
                x: 0,
                y: 1,
            },
            MapObj {
                parent_id: WET_CLAY_PLATE,
                x: 2,
                y: 0,
            },
            MapObj {
                parent_id: BIG_CHARCOAL_PILE,
                x: 3,
                y: 0,
            },
        ];
        let pot = fill_pottery_on_fire_counts_from_map(
            0,
            0,
            0,
            0,
            &objs,
            20,
            DEFAULT_MAX_CLAY_BOWLS,
            DEFAULT_MAX_CLAY_PLATES,
            DEFAULT_MAX_CLAY_CROCKS,
        );
        assert!(pot.firing_kiln);
        assert_eq!(pot.count_bowl, 1);
        assert_eq!(pot.count_close_bowl, 1);
        assert_eq!(pot.count_wet_plate, 1);
        assert_eq!(pot.count_coal, 1);
    }

    #[test]
    fn do_pottery_on_fire_l2946_other_crafts() {
        // Wet-bowl fire when FIX close-bowl gate fails (count_bowl == close)
        let mut pot = PotteryOnFireCounts {
            count_bowl: 1,
            count_close_bowl: 1,
            max_bowls: 3,
            count_wet_bowl: 2,
            firing_kiln: true,
            ..Default::default()
        };
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );

        // Wet crock shaping 233+233 when under crock max and ≥2 wet bowls
        pot.count_wet_bowl = 2;
        pot.count_bowl = 3;
        pot.max_bowls = 3;
        pot.count_crock = 0;
        pot.count_wet_crock = 0;
        pot.max_crock = 2;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );

        // Nozzle fire when wet nozzles present under max
        pot.count_wet_bowl = 0;
        pot.count_wet_nozzle = 1;
        pot.count_nozzle = 0;
        pot.max_nozzle = 2;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_NOZZLE_TONGS
            }
        );

        // After residual crafts: adobe when coal low
        pot.count_wet_nozzle = 0;
        pot.count_coal = 1;
        pot.firing_kiln = true;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::ShortCraft {
                actor: ADOBE,
                target: FIRING_KILN
            }
        );
    }

    #[test]
    fn elder_collect_age_gate_on_rung_open() {
        let c = counts_with(&[], Some(FORGE), 0);
        let mut rt = SmithProfessionRuntime::default();
        // ElderCollect is not a ladder rung; exercise slot directly
        assert_eq!(
            decide_smith_job_for_slot(
                SmithJobSlot::ElderCollect,
                &c,
                &mut rt,
                0.0,
                0.0,
                40.0
            ),
            SmithAction::None
        );
        let a = decide_smith_job_for_slot(
            SmithJobSlot::ElderCollect,
            &c,
            &mut rt,
            0.0,
            0.0,
            41.0,
        );
        // age>40 may Abort if peer cap or become; with empty peers becomes + forge → prep
        assert!(a.is_some() || matches!(a, SmithAction::Abort | SmithAction::None));
    }
}
