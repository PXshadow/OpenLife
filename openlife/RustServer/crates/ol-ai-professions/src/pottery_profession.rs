//! Haxe: `AiBase` potter profession family (chunk **AI-POTTER** / **pottery_job**).
//!
//! Pure decision helpers for:
//! - `hasOrBecomeProfession('POTTER')` with max-people + sticky last
//! - Speech `POTTER!` â†’ assigned job
//! - Kiln selection (`GetKiln` priority + firing-first in `doPottery`)
//! - Stage ladder (`profession['POTTER']` 0/2/3/10)
//! - `doPottery` / `doPotteryHelper` shortCraft sequence
//! - `doPotteryOnFire` via [`crate::do_pottery_on_fire`] (shared with smith fallthrough)
//! - `gatherClay` pure spatial decisions (basket / deposit / drop home)
//!
//! No world I/O: callers supply counts / kiln parent and apply returned
//! [`PotteryAction`]s via craft/shortCraft and spatial helpers.
//!
//! **AI-POTTER-RESID**: EmptyBasketAtHome dropIsAUse=false â†’ empty-hand DROP extract;
//! wet-nozzle cleanup shortCraft(285,285) / shortCraft(0,2110) (Haxe cleanUp ~L1031â€“1041).
//! **AI-POTTER-L2946** other crafts in shared `do_pottery_on_fire` (wet-bowl fire,
//! wet-crock 233+233 shape, clay nozzle 296) + fired nozzle tongs shortCraftOnGround
//! + smith live DeferPottery pottery fill.

use std::collections::HashMap;

use ol_ai_helper::ai_goals::{Goal, POTTER_TARGET_ID};
use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use crate::smith_profession::{
    do_pottery_on_fire, PotteryOnFireCounts, SmithAction, ADOBE, BIG_CHARCOAL_PILE, CLAY_BOWL,
    CLAY_CROCK, CLAY_NOZZLE, CLAY_PLATE, CROCK_WITH_SQUASH, DEFAULT_MAX_CLAY_BOWLS,
    DEFAULT_MAX_CLAY_CROCKS, DEFAULT_MAX_CLAY_NOZZLES, DEFAULT_MAX_CLAY_PLATES, FIRED_BOWL_TONGS,
    FIRED_CROCK_TONGS, FIRED_NOZZLE_TONGS, FIRED_PLATE_TONGS, FIRING_KILN, HUGE_CHARCOAL_PILE,
    STONE, WET_BOWL_TONGS, WET_CLAY_BOWL, WET_CLAY_CROCK, WET_CLAY_NOZZLE, WET_CLAY_PLATE,
    WET_CROCK_TONGS, WET_NOZZLE_TONGS, WET_PLATE_TONGS,
};

// â”€â”€ Object ids (OHOL / OpenLife content; Haxe comments in AiBase) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Adobe Kiln (cold).
// Haxe: AiBase.GetKiln / doPottery ~238
pub const ADOBE_KILN: i32 = 238;
/// Wood-filled Adobe Kiln.
pub const WOOD_FILLED_KILN: i32 = 281;
/// Firing Adobe Kiln (same as smith `FIRING_KILN` = 282).
pub const FIRING_ADOBE_KILN: i32 = FIRING_KILN;
/// Firing Adobe Kiln Sealed.
pub const FIRING_KILN_SEALED: i32 = 293;
/// Sealed Adobe Kiln.
pub const SEALED_ADOBE_KILN: i32 = 294;
/// Adobe Kiln with Charcoal.
pub const KILN_WITH_CHARCOAL: i32 = 299;
/// Basket (empty / clay carrier).
pub const BASKET: i32 = 292;
/// Basket of Charcoal.
pub const BASKET_OF_CHARCOAL: i32 = 298;
/// Clay.
pub const CLAY: i32 = 126;
/// Clay Deposit.
pub const CLAY_DEPOSIT: i32 = 125;
/// Clay Pit.
pub const CLAY_PIT: i32 = 409;
/// Pile of Clay.
pub const PILE_OF_CLAY: i32 = 3905;
/// Clay with Nozzle (empty-hand USE â†’ Wet Clay Nozzle).
// Haxe: AiBase.cleanUp shortCraft(0, 2110) ~L1040â€“1041
pub const CLAY_WITH_NOZZLE: i32 = 2110;
// Clay Bowl/Plate/Fired tongs ids live in smith_profession; use CLAY_BOWL etc. via imports above.

/// Home/kiln search radius (Haxe GetClosestObjectToPosition r=20).
pub const KILN_SEARCH_RADIUS: i32 = 20;
/// Clay on-floor count radius near home (Haxe CountCloseObjects r=20).
pub const CLAY_FLOOR_COUNT_RADIUS: i32 = 20;
/// Default max people for age-rotated pottery (Haxe doPottery() max=1).
pub const POTTER_DEFAULT_MAX_PEOPLE: i32 = 1;
/// Assigned-job max people (Haxe doPottery(100)).
pub const POTTER_ASSIGNED_MAX_PEOPLE: i32 = 100;
/// High-priority pottery (Haxe doPottery(-2) critical / hot kiln).
pub const POTTER_CRITICAL_MAX_PEOPLE: i32 = -2;
/// Craft search radius override in doPottery (Haxe maxSearchRadius = 30).
pub const POTTERY_CRAFT_SEARCH_RADIUS: i32 = 30;
/// gatherClay home quad distance â‰¤100 for "close to home".
// Haxe: distanceToHome <= 100 (quad)
pub const GATHER_CLAY_HOME_QUAD: i32 = 100;
/// gatherClay far-from-home loose clay scan (quad > 225).
pub const GATHER_CLAY_FAR_QUAD: i32 = 225;
/// Max clay units to need in one batch (Haxe neededClay cap 6).
pub const NEEDED_CLAY_CAP: i32 = 6;
/// Wet stock threshold before skipping pile-of-clay pull (Haxe < 4).
pub const WET_STOCK_PILE_GATE: i32 = 4;
/// Clay total threshold to stop gathering (Haxe clay < 4).
pub const GATHER_CLAY_MIN_STOCK: i32 = 4;
/// Empty basket-with-clay search near home (Haxe GetClosestObjectToPosition r=10).
// Haxe: AiBase.gatherClay empty basket home r=10 ~L3009
pub const EMPTY_BASKET_HOME_SEARCH_RADIUS: i32 = 10;
/// Wet nozzle ground count radius in cleanUp (Haxe CountCloseObjects r=20).
// Haxe: AiBase.cleanUp Wet Clay Nozzle count r=20 ~L1032
pub const WET_NOZZLE_CLEANUP_COUNT_RADIUS: i32 = 20;
/// shortCraft(285,285) search radius (Haxe 30).
pub const WET_NOZZLE_MERGE_SEARCH_RADIUS: i32 = 30;
/// shortCraft(0, 2110) search radius (Haxe 20).
pub const CLAY_WITH_NOZZLE_SEARCH_RADIUS: i32 = 20;

/// Canonical Haxe profession string for potter.
pub const POTTER_PROFESSION_KEY: &str = "POTTER";

// â”€â”€ Profession speech / runtime â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Parse speech / assigned profession tokens for potter.
///
/// Accepts `POTTER`, `POTTER!`, case-insensitive.
// Haxe: AiBase speech endsWith("!"); assignedProfession == 'POTTER'
pub fn parse_potter_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("POTTER")
}

/// Sticky last + assigned + stage weight for POTTER.
///
/// Haxe `this.profession['POTTER']` stages used in doPotteryHelper:
/// - `0` â€” idle / stock full / finished fallthrough
/// - `1` â€” just became potter (`hasOrBecomeProfession`)
/// - `2` â€” stop gathering clay; shape pottery first
/// - `3` â€” shaping wet bowls/plates from clay/pile
/// - `10` â€” firing / on-fire path
#[derive(Debug, Clone, PartialEq)]
pub struct PotterProfessionRuntime {
    pub is_last_potter: bool,
    pub is_assigned_potter: bool,
    /// Haxe `this.profession['POTTER']` stage weight.
    pub stage: f32,
}

impl Default for PotterProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_potter: false,
            is_assigned_potter: false,
            stage: 0.0,
        }
    }
}

impl PotterProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_stage(&mut self) {
        self.stage = 0.0;
    }

    /// Apply eat-path profession wipe (stage 0 + clear sticky unless FOODSERVER residual).
    // Haxe: isConsideringMakingFood profession wipe family
    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.stage = 0.0;
        if !last_was_foodserver {
            self.is_last_potter = false;
        }
    }
}

/// Assign from speech `POTTER!`.
// Haxe: assignedProfession = 'POTTER'
pub fn assign_potter_from_speech(runtime: &mut PotterProfessionRuntime, text: &str) -> bool {
    if !parse_potter_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_potter = true;
    runtime.is_last_potter = true;
    runtime.stage = runtime.stage.max(1.0);
    true
}

/// Count peers already sticky on POTTER.
// Haxe: AiBase.countProfession('POTTER')
pub fn count_potter_peers(peer_count_with_last_potter: f32) -> f32 {
    peer_count_with_last_potter.max(0.0)
}

/// One AI peer for pure `countProfession('POTTER')` filtering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PotterPeerSnapshot {
    pub deleted: bool,
    pub age: f32,
    pub is_wounded: bool,
    pub food_store: f32,
    pub has_player_to_follow: bool,
    pub same_home: bool,
    pub last_is_potter: bool,
}

impl PotterPeerSnapshot {
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

    pub fn counts_as_potter(self, min_age_to_eat: f32, max_age: f32) -> bool {
        self.eligible_for_count(min_age_to_eat, max_age) && self.last_is_potter
    }
}

/// Full pure `countProfession('POTTER')` over peer snapshots.
pub fn count_potter_peers_filtered(
    peers: &[PotterPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
) -> f32 {
    peers
        .iter()
        .filter(|p| p.counts_as_potter(min_age_to_eat, max_age))
        .count() as f32
}

/// Haxe `hasOrBecomeProfession('POTTER', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_potter(
    runtime: &mut PotterProfessionRuntime,
    max: i32,
    peer_count_with_last_potter: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        // High priority: do job but do not assign profession.
        return true;
    }
    if runtime.is_last_potter {
        runtime.is_last_potter = true;
        return true;
    }
    let count = count_potter_peers(peer_count_with_last_potter);
    let cap = max as f32 + was_idle.max(0.0);
    if count >= cap {
        return false;
    }
    runtime.stage = runtime.stage.max(1.0);
    runtime.is_last_potter = true;
    true
}

pub fn has_or_become_potter_filtered(
    runtime: &mut PotterProfessionRuntime,
    max: i32,
    peers: &[PotterPeerSnapshot],
    min_age_to_eat: f32,
    max_age: f32,
    was_idle: f32,
) -> bool {
    let peer_count = count_potter_peers_filtered(peers, min_age_to_eat, max_age);
    has_or_become_potter(runtime, max, peer_count, was_idle)
}

/// Prefer assigned over sticky last for AssignedJob dispatch.
// Haxe: assignedProfession == 'POTTER' || lastProfession == 'POTTER' ~727
pub fn resolve_potter_assigned_job(runtime: &PotterProfessionRuntime) -> bool {
    runtime.is_assigned_potter || runtime.is_last_potter
}

// â”€â”€ Kiln selection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Ordered kiln parent ids for `GetKiln` (wood-filled â†’ cold â†’ firing â†’ sealedâ€¦).
// Haxe: AiBase.GetKiln ~2781â€“2795
pub fn kiln_id_priority() -> &'static [i32] {
    &[
        WOOD_FILLED_KILN,
        ADOBE_KILN,
        FIRING_ADOBE_KILN,
        SEALED_ADOBE_KILN,
        FIRING_KILN_SEALED,
    ]
}

/// Kiln parents used when firing-path already checked (doPottery mid: wood/cold only).
// Haxe: doPotteryHelper after firing check ~2831â€“2834
pub fn kiln_id_priority_after_fire_check() -> &'static [i32] {
    &[WOOD_FILLED_KILN, ADOBE_KILN]
}

/// True if `id` is any adobe-kiln family parent used by pottery.
pub fn is_kiln_id(id: i32) -> bool {
    matches!(
        id,
        ADOBE_KILN
            | WOOD_FILLED_KILN
            | FIRING_ADOBE_KILN
            | SEALED_ADOBE_KILN
            | FIRING_KILN_SEALED
            | KILN_WITH_CHARCOAL
    )
}

/// Spatial kiln candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KilnCandidate {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Chebyshev distance (same as smith/farmer helpers).
pub fn potter_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Quad distance (dxÂ²+dyÂ²) matching Haxe `CalculateQuadDistanceToObject`.
pub fn potter_quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Haxe `GetKiln`: priority wood-filledâ†’coldâ†’firingâ†’sealed, closest in radius of home.
// Haxe: AiBase.GetKiln ~2781
pub fn pick_kiln_near_home(
    home_x: i32,
    home_y: i32,
    candidates: &[KilnCandidate],
) -> Option<KilnCandidate> {
    pick_kiln_near_home_radius(home_x, home_y, candidates, KILN_SEARCH_RADIUS)
}

pub fn pick_kiln_near_home_radius(
    home_x: i32,
    home_y: i32,
    candidates: &[KilnCandidate],
    radius: i32,
) -> Option<KilnCandidate> {
    pick_kiln_by_priority(home_x, home_y, candidates, radius, kiln_id_priority())
}

/// Closest kiln matching priority list within radius.
pub fn pick_kiln_by_priority(
    home_x: i32,
    home_y: i32,
    candidates: &[KilnCandidate],
    radius: i32,
    priority: &[i32],
) -> Option<KilnCandidate> {
    for &want in priority {
        let mut best: Option<(i32, KilnCandidate)> = None;
        for &c in candidates {
            if c.parent_id != want {
                continue;
            }
            let d = potter_chebyshev(home_x, home_y, c.x, c.y);
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

/// Closest firing kiln (282) near home â€” early doPottery gate.
// Haxe: GetClosestObjectToPosition(â€¦, 282, 20)
pub fn pick_firing_kiln_near_home(
    home_x: i32,
    home_y: i32,
    candidates: &[KilnCandidate],
) -> Option<KilnCandidate> {
    pick_kiln_by_priority(
        home_x,
        home_y,
        candidates,
        KILN_SEARCH_RADIUS,
        &[FIRING_ADOBE_KILN],
    )
}

/// Clay deposit / pit candidate for gather path.
// Haxe: AiBase.gatherClay L2968 TODO GetClosestObjectByIds clay deposit vs pit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaySourceCandidate {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// True if id is a diggable clay source (deposit or pit).
pub fn is_clay_source_id(id: i32) -> bool {
    matches!(id, CLAY_DEPOSIT | CLAY_PIT)
}

/// Closest clay deposit **or** pit to the player (Haxe GetClosestObjectByIds family).
///
/// Caller passes map objects; pure pick replaces Haxe TODO L2968.
// Haxe: AiBase.gatherClay ~2968 GetClosestObjectByIds([125, 409])
pub fn pick_closest_clay_source(
    player_x: i32,
    player_y: i32,
    candidates: &[ClaySourceCandidate],
) -> Option<ClaySourceCandidate> {
    pick_closest_clay_source_radius(player_x, player_y, candidates, i32::MAX)
}

pub fn pick_closest_clay_source_radius(
    player_x: i32,
    player_y: i32,
    candidates: &[ClaySourceCandidate],
    radius: i32,
) -> Option<ClaySourceCandidate> {
    let mut best: Option<(i32, ClaySourceCandidate)> = None;
    for &c in candidates {
        if !is_clay_source_id(c.parent_id) {
            continue;
        }
        let d = potter_chebyshev(player_x, player_y, c.x, c.y);
        if d > radius {
            continue;
        }
        match best {
            None => best = Some((d, c)),
            Some((bd, _)) if d < bd => best = Some((d, c)),
            // Prefer deposit over pit on equal distance (stable content bias).
            Some((bd, prev)) if d == bd && c.parent_id == CLAY_DEPOSIT && prev.parent_id == CLAY_PIT => {
                best = Some((d, c));
            }
            _ => {}
        }
    }
    best.map(|(_, c)| c)
}

/// Build [`GatherClayInput`] deposit fields from closest clay source pick.
pub fn apply_clay_source_to_gather_input(
    inp: &mut GatherClayInput,
    source: Option<ClaySourceCandidate>,
) {
    match source {
        Some(s) => {
            inp.has_clay_deposit = true;
            inp.deposit_x = s.x;
            inp.deposit_y = s.y;
        }
        None => {
            inp.has_clay_deposit = false;
            inp.deposit_x = 0;
            inp.deposit_y = 0;
        }
    }
}

/// Empty-basket drop near deposit stages the basket as dropTarget (Haxe dropIsAUse path).
///
/// Live layer should treat [`PotteryAction::DropHeld`] of basket adjacent to deposit as
/// â€œplace basket then digâ€ staging, not a free-form drop away from the pit.
// Haxe: AiBase.gatherClay L2992 dropHeldObject(0) near deposit (empty basket staging)
pub fn empty_basket_drop_is_deposit_staging(
    held_id: i32,
    held_contained: i32,
    deposit_adjacent: bool,
) -> bool {
    held_id == BASKET && held_contained <= 2 && deposit_adjacent
}

/// EmptyBasketAtHome with empty hands uses DROP extract (not USE pickup whole basket).
///
/// Haxe sets `dropIsAUse = false` + `dropTarget = basket` so `isDropingItem` calls
/// `myPlayer.drop` â†’ `DoContainerStuffOnObj` takes first contained (clay).
// Haxe: AiBase.gatherClay L3013â€“3014 TODO empty basket ??? (dropIsAUse=false)
pub fn empty_basket_at_home_is_drop_extract(held_id: i32) -> bool {
    held_id == 0
}

/// Haxe `cleanUp` wet-nozzle residual (outside L2946 pottery-on-fire TODO).
///
/// 1. `shortCraft(285, 285)` when â‰¥2 ground wet nozzles, or â‰¥1 ground + holding 285  
/// 2. else `shortCraft(0, 2110)` when Clay with Nozzle is present  
///
/// Age `% 3` gate stays with the general cleanUp caller; pottery SM may call freely.
// Haxe: AiBase.cleanUp ~L1031â€“1041
pub fn wet_nozzle_cleanup_action(
    count_wet_nozzle_ground: i32,
    held_id: i32,
    has_clay_with_nozzle: bool,
) -> Option<PotteryAction> {
    // Wet Clay Nozzle 285 + Wet Clay Nozzle 285 = Clay (content 285_285.txt)
    if count_wet_nozzle_ground > 1
        || (count_wet_nozzle_ground > 0 && held_id == WET_CLAY_NOZZLE)
    {
        return Some(PotteryAction::ShortCraft {
            actor: WET_CLAY_NOZZLE,
            target: WET_CLAY_NOZZLE,
        });
    }
    // 0 + Clay with Nozzle 2110 = Wet Clay Nozzle 285 (content 0_2110.txt)
    if has_clay_with_nozzle {
        return Some(PotteryAction::ShortCraft {
            actor: 0,
            target: CLAY_WITH_NOZZLE,
        });
    }
    None
}

// â”€â”€ World counts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Close-object counts near home/kiln for pottery decisions.
#[derive(Debug, Clone, Default)]
pub struct PotteryCounts {
    pub by_id: HashMap<i32, i32>,
    /// Held object parent id (0 empty).
    pub held_id: i32,
    /// Contained count when holding basket (clay in basket).
    pub held_contained: i32,
    /// Closest kiln parent after GetKiln / mid-pipeline lookup.
    pub kiln_parent_id: Option<i32>,
    /// Firing kiln present near home (early gate).
    pub firing_kiln: bool,
    /// Clay Bowl aiCraftMax (content or default).
    pub max_bowls: i32,
    pub max_plates: i32,
    pub max_crock: i32,
    /// Clay nozzle aiCraftMax-style cap (AI-POTTER-L2946).
    pub max_nozzle: i32,
    /// Close bowls r=20 from player (doPotteryOnFire FIX gate).
    pub count_close_bowl: i32,
    pub count_close_crock: i32,
    /// Clay on floor near home (126, no wet).
    pub count_clay_on_floor: i32,
    /// Home present (required for doPottery).
    pub has_home: bool,
}

impl PotteryCounts {
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

    /// Wet Clay Bowl 233 + Wet Bowl tongs 284.
    pub fn count_wet_bowl(&self) -> i32 {
        self.sum(&[WET_CLAY_BOWL, WET_BOWL_TONGS])
    }

    /// Wet Clay Plate 234 + Wet Plate tongs 240.
    pub fn count_wet_plate(&self) -> i32 {
        self.sum(&[WET_CLAY_PLATE, WET_PLATE_TONGS])
    }

    /// Wet Clay Crock 1216 + tongs 1218 (+ squash crock 1243 for stock clay sum).
    pub fn count_wet_crock(&self) -> i32 {
        self.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS, CROCK_WITH_SQUASH])
    }

    /// Clay 126 + wet bowls + wet plates (Haxe clay stock for gather gate).
    // Haxe: clay += countWetBowl + countWetPlate
    pub fn count_clay_stock(&self) -> i32 {
        self.get(CLAY) + self.count_wet_bowl() + self.count_wet_plate()
    }

    pub fn count_bowl(&self) -> i32 {
        self.get(CLAY_BOWL)
    }

    pub fn count_plate(&self) -> i32 {
        self.get(CLAY_PLATE)
    }

    pub fn count_crock(&self) -> i32 {
        self.sum(&[CLAY_CROCK, CROCK_WITH_SQUASH])
    }

    /// Wet crock for shaping/firing (excludes squash).
    // Haxe: countCurrentObjects([1216, 1218]) in doPotteryOnFire
    pub fn count_wet_crock_raw(&self) -> i32 {
        self.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS])
    }

    /// Wet Clay Nozzle 285 + Wet Nozzle tongs 295.
    pub fn count_wet_nozzle(&self) -> i32 {
        self.sum(&[WET_CLAY_NOZZLE, WET_NOZZLE_TONGS])
    }

    /// Clay Nozzle 286 + Fired Nozzle tongs 296.
    pub fn count_nozzle(&self) -> i32 {
        self.sum(&[CLAY_NOZZLE, FIRED_NOZZLE_TONGS])
    }

    pub fn count_charcoal_basket(&self) -> i32 {
        self.get(BASKET_OF_CHARCOAL)
    }

    pub fn count_coal(&self) -> i32 {
        self.sum(&[BIG_CHARCOAL_PILE, HUGE_CHARCOAL_PILE])
    }
}

/// Map object for mock fill / thin tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PotteryMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
}

/// Fill [`PotteryCounts`] from a map snapshot.
// Haxe: countCurrentObject(s) + CountCloseObjects + kiln lookup
pub fn fill_pottery_counts_from_map(
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    held_contained: i32,
    objects: &[PotteryMapObj],
    home_r: i32,
    max_bowls: i32,
    max_plates: i32,
    max_crock: i32,
) -> PotteryCounts {
    let kiln_cands: Vec<KilnCandidate> = objects
        .iter()
        .filter(|o| is_kiln_id(o.parent_id))
        .map(|o| KilnCandidate {
            parent_id: o.parent_id,
            x: o.x,
            y: o.y,
        })
        .collect();
    let firing = pick_firing_kiln_near_home(home_x, home_y, &kiln_cands);
    let kiln = pick_kiln_near_home(home_x, home_y, &kiln_cands);

    let mut c = PotteryCounts {
        held_id,
        held_contained,
        kiln_parent_id: kiln.map(|k| k.parent_id),
        firing_kiln: firing.is_some(),
        max_bowls,
        max_plates,
        max_crock,
        max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,
        has_home: true,
        ..Default::default()
    };

    for o in objects {
        let d_home = potter_chebyshev(home_x, home_y, o.x, o.y);
        let d_player = potter_chebyshev(player_x, player_y, o.x, o.y);
        let id = o.parent_id;
        if d_home <= home_r {
            if is_kiln_id(id) {
                // kiln is parent only
            } else {
                let n = c.get(id);
                c.set(id, n + 1);
            }
            if id == CLAY {
                c.count_clay_on_floor += 1;
            }
        }
        if d_player <= home_r {
            if id == CLAY_BOWL {
                c.count_close_bowl += 1;
            }
            if id == CLAY_CROCK {
                c.count_close_crock += 1;
            }
        }
    }
    c
}

// â”€â”€ Actions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Pure decision output â€” execution is AI-CRAFT / shortCraft / spatial wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotteryAction {
    /// Nothing to do this step.
    None,
    /// Haxe `shortCraft(actor, target)`.
    ShortCraft { actor: i32, target: i32 },
    /// Haxe `shortCraftOnGround(target)`.
    ShortCraftOnGround { target: i32 },
    /// Haxe `craftItem(objectId)`.
    CraftItem { object_id: i32 },
    /// Haxe `GetOrCraftItem` / seek (basket, clay deposit tools).
    SeekOrCraft { object_id: i32 },
    /// Drop held object (Haxe `dropHeldObject(maxDistanceToHome, allowAllPiles)`).
    ///
    /// `max_distance_to_home`: 0 = feet/staging at deposit; 1 = near-home clear hands;
    /// 10 = clay drop radius; 40 = default smart drop.
    DropHeld {
        allow_piles: bool,
        max_distance_to_home: i32,
    },
    /// Goto home (bring full basket).
    GotoHome,
    /// Goto clay deposit / pit.
    GotoClayDeposit,
    /// Use held on clay deposit (empty hands dig).
    UseOnClayDeposit,
    /// Use held clay on basket.
    UseOnBasket,
    /// Pickup basket (full â†’ home, empty â†’ fill).
    PickupBasket,
    /// Empty basket at home (set dropTarget basket).
    EmptyBasketAtHome,
    /// Pickup loose clay far from home.
    PickupLooseClay,
    /// Chain complete / stock full / cannot potter.
    Abort,
}

impl PotteryAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None | Self::Abort)
    }

    pub fn is_spatial(self) -> bool {
        matches!(
            self,
            Self::GotoHome
                | Self::GotoClayDeposit
                | Self::UseOnClayDeposit
                | Self::UseOnBasket
                | Self::PickupBasket
                | Self::EmptyBasketAtHome
                | Self::PickupLooseClay
                | Self::DropHeld { .. }
        )
    }
}

/// Map smith `do_pottery_on_fire` result into [`PotteryAction`].
pub fn smith_pottery_action_to_pottery(a: SmithAction) -> PotteryAction {
    match a {
        SmithAction::None | SmithAction::Abort | SmithAction::DeferPottery => PotteryAction::None,
        SmithAction::ShortCraft { actor, target } => PotteryAction::ShortCraft { actor, target },
        SmithAction::ShortCraftOnGround { target } => PotteryAction::ShortCraftOnGround { target },
        SmithAction::CraftItem { object_id } => PotteryAction::CraftItem { object_id },
        SmithAction::CraftAndDropNearForge { object_id, .. } => {
            PotteryAction::CraftItem { object_id }
        }
    }
}

/// Build [`PotteryOnFireCounts`] from [`PotteryCounts`] for shared on-fire body.
pub fn pottery_on_fire_counts_from_pottery(c: &PotteryCounts) -> PotteryOnFireCounts {
    PotteryOnFireCounts {
        count_bowl: c.count_bowl(),
        count_close_bowl: c.count_close_bowl,
        max_bowls: if c.max_bowls > 0 {
            c.max_bowls
        } else {
            DEFAULT_MAX_CLAY_BOWLS
        },
        count_wet_bowl: c.count_wet_bowl(),
        count_plate: c.count_plate(),
        max_plates: if c.max_plates > 0 {
            c.max_plates
        } else {
            DEFAULT_MAX_CLAY_PLATES
        },
        count_wet_plate: c.count_wet_plate(),
        count_crock: c.count_crock(),
        count_close_crock: c.count_close_crock,
        max_crock: if c.max_crock > 0 {
            c.max_crock
        } else {
            DEFAULT_MAX_CLAY_CROCKS
        },
        // On-fire wet crock excludes squash (Haxe countCurrentObjects([1216, 1218])).
        count_wet_crock: c.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS]),
        count_wet_nozzle: c.count_wet_nozzle(),
        count_nozzle: c.count_nozzle(),
        max_nozzle: if c.max_nozzle > 0 {
            c.max_nozzle
        } else {
            DEFAULT_MAX_CLAY_NOZZLES
        },
        count_coal: c.count_coal(),
        firing_kiln: c.firing_kiln || c.kiln_parent_id == Some(FIRING_ADOBE_KILN),
    }
}

/// Haxe `doPotteryOnFire` pure body (potter module entry).
// Haxe: AiBase.doPotteryOnFire ~2908â€“2953
pub fn do_pottery_on_fire_action(c: &PotteryCounts) -> PotteryAction {
    smith_pottery_action_to_pottery(do_pottery_on_fire(&pottery_on_fire_counts_from_pottery(c)))
}

// â”€â”€ Needed clay math â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Bowls/plates needed vs max, including wet stock; clay needed capped at 6.
// Haxe: doPotteryHelper neededBowls/Plates/Clay ~2874â€“2880
pub fn needed_pottery_clay(c: &PotteryCounts) -> (i32, i32, i32) {
    let mut count_bowl = c.count_bowl() + c.count_wet_bowl();
    let mut count_plate = c.count_plate() + c.count_wet_plate();
    let max_b = if c.max_bowls > 0 {
        c.max_bowls
    } else {
        DEFAULT_MAX_CLAY_BOWLS
    };
    let max_p = if c.max_plates > 0 {
        c.max_plates
    } else {
        DEFAULT_MAX_CLAY_PLATES
    };
    let needed_bowls = if count_bowl > max_b {
        0
    } else {
        max_b - count_bowl
    };
    let needed_plates = if count_plate > max_p {
        0
    } else {
        max_p - count_plate
    };
    let mut needed_clay = needed_bowls + needed_plates;
    if needed_clay > NEEDED_CLAY_CAP {
        needed_clay = NEEDED_CLAY_CAP;
    }
    // silence unused mut if we don't reassign â€” keep parity with Haxe locals
    let _ = (&mut count_bowl, &mut count_plate);
    (needed_bowls, needed_plates, needed_clay)
}

// â”€â”€ gatherClay pure â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Spatial inputs for pure [`gather_clay`].
// Haxe: AiBase.gatherClay ~2956â€“3097
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherClayInput {
    /// Player world x/y.
    pub player_x: i32,
    pub player_y: i32,
    /// Home (or kiln when kiln != null â€” Haxe reassigns home = kiln).
    pub home_x: i32,
    pub home_y: i32,
    pub held_id: i32,
    /// Contained objects in held basket.
    pub held_contained: i32,
    /// Clay deposit or pit present (closest).
    pub has_clay_deposit: bool,
    pub deposit_x: i32,
    pub deposit_y: i32,
    /// Basket with clay near home (empty at home).
    pub basket_with_clay_near_home: bool,
    /// Basket with clay near player (bring home / fill).
    pub basket_with_clay_near_player: bool,
    /// Empty basket near deposit (fill path).
    pub empty_basket_near_deposit: bool,
    /// Full basket only at deposit r=5 (player may be far) â€” still pickup.
    // Haxe: basket from deposit search with containedObjects.length > 2
    pub full_basket_near_deposit: bool,
    /// Full basket (contained > 2) among basket targets.
    pub basket_full: bool,
    /// Loose clay near player when far from home.
    pub loose_clay_near_player: bool,
}

impl Default for GatherClayInput {
    fn default() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: 0,
            held_contained: 0,
            has_clay_deposit: false,
            deposit_x: 0,
            deposit_y: 0,
            basket_with_clay_near_home: false,
            basket_with_clay_near_player: false,
            empty_basket_near_deposit: false,
            full_basket_near_deposit: false,
            basket_full: false,
            loose_clay_near_player: false,
        }
    }
}

/// Pure `gatherClay` decision body.
// Haxe: AiBase.gatherClay ~2956â€“3097
pub fn gather_clay(inp: &GatherClayInput) -> PotteryAction {
    let dist_home = potter_quad_dist(inp.player_x, inp.player_y, inp.home_x, inp.home_y);
    let _dist_deposit = if inp.has_clay_deposit {
        potter_quad_dist(inp.player_x, inp.player_y, inp.deposit_x, inp.deposit_y)
    } else {
        -1
    };
    // Haxe uses CalculateQuadDistance but compares to 1 for "adjacent" via helper â€”
    // live layer uses tile adjacency; pure uses chebyshev â‰¤1 for deposit.
    let deposit_adj = inp.has_clay_deposit
        && potter_chebyshev(inp.player_x, inp.player_y, inp.deposit_x, inp.deposit_y) <= 1;

    // Holding Basket 292
    if inp.held_id == BASKET {
        if inp.held_contained > 2 {
            if dist_home <= GATHER_CLAY_HOME_QUAD {
                // Haxe: dropHeldObject() default maxDistance
                return PotteryAction::DropHeld {
                    allow_piles: false,
                    max_distance_to_home: 40,
                };
            }
            return PotteryAction::GotoHome;
        }
        // empty basket â†’ drop near deposit
        if !inp.has_clay_deposit {
            return PotteryAction::None;
        }
        if deposit_adj {
            // Haxe: dropHeldObject(0) â€” stage basket at deposit feet
            return PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 0,
            };
        }
        // Haxe returns true even if goto fails (new pit can be tried)
        return PotteryAction::GotoClayDeposit;
    }

    // Close to home: empty basket with clay
    if dist_home <= GATHER_CLAY_HOME_QUAD && inp.basket_with_clay_near_home {
        if inp.held_id != 0 {
            // Haxe: dropHeldObject(1, true)
            return PotteryAction::DropHeld {
                allow_piles: true,
                max_distance_to_home: 1,
            };
        }
        return PotteryAction::EmptyBasketAtHome;
    }

    // Full basket near player or deposit â†’ pickup to bring home
    // Haxe: basket != null && contained > 2 (includes deposit-only full when player far)
    if inp.basket_full
        && (inp.basket_with_clay_near_player
            || inp.empty_basket_near_deposit
            || inp.basket_with_clay_near_home
            || inp.full_basket_near_deposit)
    {
        if inp.held_id != 0 {
            // Haxe: dropHeldObject(1)
            return PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 1,
            };
        }
        return PotteryAction::PickupBasket;
    }

    // Holding Clay 126
    if inp.held_id == CLAY {
        if dist_home <= GATHER_CLAY_HOME_QUAD {
            // Haxe: dropHeldObject(10, true)
            return PotteryAction::DropHeld {
                allow_piles: true,
                max_distance_to_home: 10,
            };
        }
        // need basket to put clay
        let has_basket = inp.basket_with_clay_near_player
            || inp.empty_basket_near_deposit
            || inp.basket_with_clay_near_home;
        if !has_basket {
            // Haxe: dropHeldObject(10)
            return PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 10,
            };
        }
        return PotteryAction::UseOnBasket;
    }

    // Loose clay far from home
    if dist_home > GATHER_CLAY_FAR_QUAD && inp.loose_clay_near_player {
        return PotteryAction::PickupLooseClay;
    }

    if !inp.has_clay_deposit {
        return PotteryAction::None;
    }

    // No basket â†’ GetOrCraft basket 292
    let has_any_basket = inp.basket_with_clay_near_player
        || inp.empty_basket_near_deposit
        || inp.basket_with_clay_near_home
        || inp.basket_full
        || inp.full_basket_near_deposit;
    // Haxe: basket null after searches near deposit / player
    // When holding nothing and no basket nearby â†’ craft basket
    if !has_any_basket {
        return PotteryAction::SeekOrCraft {
            object_id: BASKET,
        };
    }

    // Basket near deposit empty path: Haxe finds empty basket near deposit;
    // if not holding and not adjacent, goto deposit then dig.
    if inp.held_id != 0 {
        // Haxe: dropHeldObject(10)
        return PotteryAction::DropHeld {
            allow_piles: true,
            max_distance_to_home: 10,
        };
    }

    if !deposit_adj {
        return PotteryAction::GotoClayDeposit;
    }

    PotteryAction::UseOnClayDeposit
}

// â”€â”€ doPottery pure body â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Full `doPottery` / `doPotteryHelper` pure body.
///
/// Mutates `runtime.stage` like Haxe `profession['POTTER']`.
/// Optional `gather` input enables clay-gather branch; `None` skips gather
/// (returns seek clay deposit when stage needs clay).
// Haxe: AiBase.doPottery / doPotteryHelper ~2798â€“2905
pub fn do_pottery(
    counts: &PotteryCounts,
    runtime: &mut PotterProfessionRuntime,
    max_people: i32,
    peer_count_with_last_potter: f32,
    was_idle: f32,
    gather: Option<&GatherClayInput>,
) -> PotteryAction {
    if !counts.has_home {
        return PotteryAction::Abort;
    }

    if !has_or_become_potter(
        runtime,
        max_people,
        peer_count_with_last_potter,
        was_idle,
    ) {
        return PotteryAction::Abort;
    }

    // shortCraftOnGround fired tongs products
    if counts.get(FIRED_BOWL_TONGS) > 0 || counts.held_id == FIRED_BOWL_TONGS {
        return PotteryAction::ShortCraftOnGround {
            target: FIRED_BOWL_TONGS,
        };
    }
    if counts.get(FIRED_PLATE_TONGS) > 0 || counts.held_id == FIRED_PLATE_TONGS {
        return PotteryAction::ShortCraftOnGround {
            target: FIRED_PLATE_TONGS,
        };
    }
    if counts.get(FIRED_CROCK_TONGS) > 0 || counts.held_id == FIRED_CROCK_TONGS {
        return PotteryAction::ShortCraftOnGround {
            target: FIRED_CROCK_TONGS,
        };
    }
    // AI-POTTER-L2946: after craftItem(296), place/use Fired Nozzle tongs like bowl/plate/crock.
    // Haxe: shortCraftOnGround pattern for fired tongs products in doPottery
    if counts.get(FIRED_NOZZLE_TONGS) > 0 || counts.held_id == FIRED_NOZZLE_TONGS {
        return PotteryAction::ShortCraftOnGround {
            target: FIRED_NOZZLE_TONGS,
        };
    }

    // Basket of Charcoal 298 when count > 2
    if counts.count_charcoal_basket() > 2 {
        return PotteryAction::ShortCraftOnGround {
            target: BASKET_OF_CHARCOAL,
        };
    }

    // AI-POTTER-RESID: cleanUp wet-nozzle residual (Haxe ~L1031â€“1041, outside L2946)
    // Count ground 285 only (not tongs 295) â€” matches CountCloseObjects(home, 285, 20).
    if let Some(a) = wet_nozzle_cleanup_action(
        counts.get(WET_CLAY_NOZZLE),
        counts.held_id,
        counts.get(CLAY_WITH_NOZZLE) > 0 || counts.held_id == CLAY_WITH_NOZZLE,
    ) {
        return a;
    }

    // Firing kiln â†’ stage 10 + doPotteryOnFire
    if counts.firing_kiln {
        runtime.stage = 10.0;
        let on_fire = do_pottery_on_fire_action(counts);
        if on_fire.is_some() {
            return on_fire;
        }
    }

    // Unseal sealed kiln / empty charcoal kiln
    if counts.get(SEALED_ADOBE_KILN) > 0 {
        return PotteryAction::ShortCraft {
            actor: 0,
            target: SEALED_ADOBE_KILN,
        };
    }
    if counts.get(KILN_WITH_CHARCOAL) > 0 {
        return PotteryAction::ShortCraft {
            actor: BASKET,
            target: KILN_WITH_CHARCOAL,
        };
    }

    // Need kiln (wood-filled or cold adobe) for rest of pipeline
    let has_kiln = counts.kiln_parent_id.is_some()
        || counts.firing_kiln
        || counts.get(WOOD_FILLED_KILN) > 0
        || counts.get(ADOBE_KILN) > 0;

    let clay_stock = counts.count_clay_stock();
    if runtime.stage < 2.0 && clay_stock < GATHER_CLAY_MIN_STOCK {
        if let Some(g) = gather {
            let a = gather_clay(g);
            if a.is_some() {
                return a;
            }
        } else {
            // Without spatial gather input: seek clay deposit
            return PotteryAction::SeekOrCraft {
                object_id: CLAY_DEPOSIT,
            };
        }
    }

    runtime.stage = runtime.stage.max(2.0);

    if !has_kiln {
        return PotteryAction::Abort;
    }

    let max_b = if counts.max_bowls > 0 {
        counts.max_bowls
    } else {
        DEFAULT_MAX_CLAY_BOWLS
    };
    let max_p = if counts.max_plates > 0 {
        counts.max_plates
    } else {
        DEFAULT_MAX_CLAY_PLATES
    };

    // AI-POTTER-L2946: shape wet crock (233+233) before bowl/plate stock-full gate
    // so crocks still form when bowls+plates are at max (Haxe L2946 residual).
    let max_c = if counts.max_crock > 0 {
        counts.max_crock
    } else {
        DEFAULT_MAX_CLAY_CROCKS
    };
    let crock_stock = counts.count_crock() + counts.count_wet_crock_raw();
    if crock_stock < max_c && counts.count_wet_bowl() >= 2 {
        runtime.stage = runtime.stage.max(3.0);
        return PotteryAction::ShortCraft {
            actor: WET_CLAY_BOWL,
            target: WET_CLAY_BOWL,
        };
    }

    // Stock full â†’ clear profession stage
    if counts.count_bowl() >= max_b && counts.count_plate() >= max_p {
        runtime.stage = 0.0;
        return PotteryAction::None;
    }

    let (needed_bowls, needed_plates, needed_clay) = needed_pottery_clay(counts);
    let wet_total = counts.count_wet_bowl() + counts.count_wet_plate();

    if runtime.stage < 3.0
        && counts.count_clay_on_floor < needed_clay
        && wet_total < WET_STOCK_PILE_GATE
    {
        // shortCraft(0, 3905) Pile of Clay
        // Always attempt pile pull when gate says so (Haxe shortCraft searches).
        runtime.stage = runtime.stage.max(2.0);
        return PotteryAction::ShortCraft {
            actor: 0,
            target: PILE_OF_CLAY,
        };
    }

    runtime.stage = 3.0;

    // Stone + wet bowl â†’ wet plate when need plates and more bowls than plates
    let count_bowl_eff = counts.count_bowl() + counts.count_wet_bowl();
    let count_plate_eff = counts.count_plate() + counts.count_wet_plate();
    if needed_plates > 0 && count_bowl_eff > count_plate_eff {
        return PotteryAction::ShortCraft {
            actor: STONE,
            target: WET_CLAY_BOWL,
        };
    }
    if needed_bowls + needed_plates > 0 {
        return PotteryAction::ShortCraft {
            actor: STONE,
            target: CLAY,
        };
    }

    // AI-POTTER-L2946: shape wet crock (233+233) when under crock max and â‰¥2 wet bowls.
    // Content transitions/233_233.txt; feeds doPotteryOnFire crock fire path.
    let max_c = if counts.max_crock > 0 {
        counts.max_crock
    } else {
        DEFAULT_MAX_CLAY_CROCKS
    };
    let crock_stock = counts.count_crock() + counts.count_wet_crock_raw();
    if crock_stock < max_c && counts.count_wet_bowl() >= 2 {
        return PotteryAction::ShortCraft {
            actor: WET_CLAY_BOWL,
            target: WET_CLAY_BOWL,
        };
    }

    runtime.stage = 10.0;
    let on_fire = do_pottery_on_fire_action(counts);
    if on_fire.is_some() {
        return on_fire;
    }

    runtime.stage = 0.0;
    PotteryAction::None
}

/// Decide potter job for age-rotated / assigned dispatch.
// Haxe: AssignedJob POTTER â†’ doPottery(100); jobByAge==3 â†’ doPottery()
pub fn decide_potter_job(
    counts: &PotteryCounts,
    runtime: &mut PotterProfessionRuntime,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
    gather: Option<&GatherClayInput>,
) -> PotteryAction {
    do_pottery(
        counts,
        runtime,
        max_people,
        peer_count,
        was_idle,
        gather,
    )
}

/// Max people for dispatch (assigned â†’ 100, else 1).
pub fn potter_max_people_for_dispatch(is_assigned_job: bool) -> i32 {
    if is_assigned_job {
        POTTER_ASSIGNED_MAX_PEOPLE
    } else {
        POTTER_DEFAULT_MAX_PEOPLE
    }
}

/// Job-band rungs that should run potter decisions.
pub fn potter_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "AGE_ROTATED_JOB"
            | "LOW_PRIORITY_WORK"
            | "CRITICAL_POTTERY"
            | "HOT_KILN"
    )
}

/// Try decide from rung label (ladder hook).
pub fn try_decide_potter_from_rung(
    counts: &PotteryCounts,
    runtime: &mut PotterProfessionRuntime,
    rung_label: &str,
    peer_count: f32,
    was_idle: f32,
    is_assigned_job: bool,
    gather: Option<&GatherClayInput>,
) -> Option<PotteryAction> {
    if !potter_job_rung_label(rung_label) {
        return None;
    }
    if !runtime.is_last_potter && !runtime.is_assigned_potter && rung_label == "ASSIGNED_JOB" {
        // Assigned path still attempts hasOrBecome via do_pottery
    }
    let max = if rung_label == "CRITICAL_POTTERY" || rung_label == "HOT_KILN" {
        POTTER_CRITICAL_MAX_PEOPLE
    } else {
        potter_max_people_for_dispatch(is_assigned_job)
    };
    let a = decide_potter_job(counts, runtime, max, peer_count, was_idle, gather);
    if a.is_some() {
        Some(a)
    } else if matches!(a, PotteryAction::Abort) {
        Some(a)
    } else {
        Some(a) // include None (stock full) so ladder can continue
    }
}

/// Map [`PotteryAction`] â†’ high-level [`Goal`].
pub fn pottery_action_to_goal(action: PotteryAction) -> Goal {
    match action {
        PotteryAction::None | PotteryAction::Abort => Goal::SeekObject(POTTER_TARGET_ID),
        PotteryAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        PotteryAction::ShortCraftOnGround { target } => Goal::SeekObject(target),
        PotteryAction::CraftItem { object_id } | PotteryAction::SeekOrCraft { object_id } => {
            Goal::SeekObject(object_id)
        }
        PotteryAction::GotoHome | PotteryAction::DropHeld { .. } => Goal::SeekObject(CLAY),
        PotteryAction::GotoClayDeposit
        | PotteryAction::UseOnClayDeposit
        | PotteryAction::PickupLooseClay => Goal::SeekObject(CLAY_DEPOSIT),
        PotteryAction::UseOnBasket
        | PotteryAction::PickupBasket
        | PotteryAction::EmptyBasketAtHome => Goal::SeekObject(BASKET),
    }
}

/// Thin reverse-craft / inventory bias toward clay bowls.
pub fn pick_potter_goal(graph: &ReverseCraftGraph, have: &std::collections::HashSet<i32>) -> Goal {
    // Prefer wet/fired intermediates missing from inventory
    for &id in &[
        CLAY_BOWL,
        CLAY_PLATE,
        WET_CLAY_BOWL,
        FIRED_BOWL_TONGS,
        CLAY,
        ADOBE_KILN,
    ] {
        if !have.contains(&id) {
            let products = graph.products_using(id);
            if let Some(&p) = products.first() {
                return Goal::SeekObject(p);
            }
            return Goal::SeekObject(id);
        }
    }
    Goal::SeekObject(POTTER_TARGET_ID)
}

/// Goal from counts + rung (wire helper).
pub fn potter_goal_from_counts_and_rung(
    counts: &PotteryCounts,
    runtime: &mut PotterProfessionRuntime,
    rung_label: &str,
    peer_count: f32,
    was_idle: f32,
    is_assigned_job: bool,
) -> Goal {
    match try_decide_potter_from_rung(
        counts,
        runtime,
        rung_label,
        peer_count,
        was_idle,
        is_assigned_job,
        None,
    ) {
        Some(a) => pottery_action_to_goal(a),
        None => Goal::SeekObject(POTTER_TARGET_ID),
    }
}

/// Goal from map snapshot.
pub fn potter_goal_from_map_and_rung(
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    objects: &[PotteryMapObj],
    runtime: &mut PotterProfessionRuntime,
    rung_label: &str,
    peer_count: f32,
    was_idle: f32,
    is_assigned_job: bool,
) -> Goal {
    let counts = fill_pottery_counts_from_map(
        home_x,
        home_y,
        player_x,
        player_y,
        held_id,
        0,
        objects,
        KILN_SEARCH_RADIUS,
        DEFAULT_MAX_CLAY_BOWLS,
        DEFAULT_MAX_CLAY_PLATES,
        DEFAULT_MAX_CLAY_CROCKS,
    );
    potter_goal_from_counts_and_rung(
        &counts,
        runtime,
        rung_label,
        peer_count,
        was_idle,
        is_assigned_job,
    )
}

/// Radius table for docs/tests.
pub fn potter_radius_table() -> &'static [(i32, &'static str)] {
    &[
        (KILN_SEARCH_RADIUS, "GetKiln home"),
        (CLAY_FLOOR_COUNT_RADIUS, "clay floor home"),
        (POTTERY_CRAFT_SEARCH_RADIUS, "doPottery maxSearch"),
        (GATHER_CLAY_HOME_QUAD, "gatherClay home quad"),
        (GATHER_CLAY_FAR_QUAD, "gatherClay far quad"),
    ]
}

include!("pottery_action_apply.inc.rs");

// â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn counts_basic() -> PotteryCounts {
        PotteryCounts {
            has_home: true,
            max_bowls: DEFAULT_MAX_CLAY_BOWLS,
            max_plates: DEFAULT_MAX_CLAY_PLATES,
            max_crock: DEFAULT_MAX_CLAY_CROCKS,
            max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,
            kiln_parent_id: Some(ADOBE_KILN),
            ..Default::default()
        }
    }

    #[test]
    fn parse_and_assign_potter_speech() {
        assert!(parse_potter_profession_speech("POTTER!"));
        assert!(parse_potter_profession_speech("potter"));
        assert!(!parse_potter_profession_speech("SMITH!"));
        let mut rt = PotterProfessionRuntime::default();
        assert!(assign_potter_from_speech(&mut rt, "POTTER!"));
        assert!(rt.is_assigned_potter);
        assert!(rt.is_last_potter);
        assert!(rt.stage >= 1.0);
    }

    #[test]
    fn has_or_become_potter_max_and_sticky() {
        let mut rt = PotterProfessionRuntime::default();
        assert!(!has_or_become_potter(&mut rt, 1, 1.0, 0.0));
        assert!(has_or_become_potter(&mut rt, 1, 0.0, 0.0));
        assert!(rt.is_last_potter);
        assert!(has_or_become_potter(&mut rt, 1, 5.0, 0.0)); // sticky
        assert!(has_or_become_potter(&mut rt, -2, 99.0, 0.0)); // high prio
        let mut rt2 = PotterProfessionRuntime::default();
        assert!(has_or_become_potter(&mut rt2, 1, 0.5, 1.0)); // was_idle expands cap
    }

    #[test]
    fn kiln_priority_and_pick() {
        assert_eq!(kiln_id_priority()[0], WOOD_FILLED_KILN);
        let cands = [
            KilnCandidate {
                parent_id: ADOBE_KILN,
                x: 5,
                y: 0,
            },
            KilnCandidate {
                parent_id: WOOD_FILLED_KILN,
                x: 10,
                y: 0,
            },
            KilnCandidate {
                parent_id: FIRING_ADOBE_KILN,
                x: 1,
                y: 0,
            },
        ];
        // GetKiln prefers wood-filled before cold before firing
        let k = pick_kiln_near_home(0, 0, &cands).unwrap();
        assert_eq!(k.parent_id, WOOD_FILLED_KILN);
        let f = pick_firing_kiln_near_home(0, 0, &cands).unwrap();
        assert_eq!(f.parent_id, FIRING_ADOBE_KILN);
        assert!(is_kiln_id(ADOBE_KILN));
        assert!(!is_kiln_id(CLAY_BOWL));
    }

    #[test]
    fn needed_clay_caps_at_six() {
        let mut c = counts_basic();
        c.max_bowls = 10;
        c.max_plates = 10;
        let (nb, np, nc) = needed_pottery_clay(&c);
        assert_eq!(nb, 10);
        assert_eq!(np, 10);
        assert_eq!(nc, NEEDED_CLAY_CAP);
    }

    #[test]
    fn do_pottery_fired_tongs_first() {
        let mut c = counts_basic();
        c.set(FIRED_BOWL_TONGS, 1);
        let mut rt = PotterProfessionRuntime::default();
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraftOnGround {
                target: FIRED_BOWL_TONGS
            }
        );
    }

    /// AI-POTTER-L2946: residual craftItem(296) product uses shortCraftOnGround like other tongs.
    #[test]
    fn l2946_do_pottery_short_craft_on_ground_fired_nozzle() {
        let mut c = counts_basic();
        c.set(FIRED_NOZZLE_TONGS, 1);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraftOnGround {
                target: FIRED_NOZZLE_TONGS
            }
        );
        // Held path
        c.set(FIRED_NOZZLE_TONGS, 0);
        c.held_id = FIRED_NOZZLE_TONGS;
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraftOnGround {
                target: FIRED_NOZZLE_TONGS
            }
        );
    }

    #[test]
    fn do_pottery_firing_kiln_sets_stage_10_and_on_fire() {
        let mut c = counts_basic();
        c.firing_kiln = true;
        c.count_close_bowl = 5;
        c.max_bowls = 3;
        // count_bowl 0 < max and < close â†’ craft fired bowl tongs
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 2.0,
            ..Default::default()
        };
        let a = do_pottery(&c, &mut rt, 1, 0.0, 0.0, None);
        assert_eq!(
            a,
            PotteryAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );
        assert_eq!(rt.stage, 10.0);
    }

    #[test]
    fn do_pottery_stock_full_resets_stage() {
        let mut c = counts_basic();
        c.set(CLAY_BOWL, 3);
        c.set(CLAY_PLATE, 3);
        c.set(CLAY, 10); // clay stock high so no gather
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 3.0,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::None
        );
        assert_eq!(rt.stage, 0.0);
    }

    #[test]
    fn do_pottery_shapes_wet_bowl_from_clay() {
        let mut c = counts_basic();
        c.set(CLAY, 5);
        c.count_clay_on_floor = 5;
        c.max_bowls = 3;
        c.max_plates = 3;
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 3.0,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: STONE,
                target: CLAY
            }
        );
    }

    #[test]
    fn do_pottery_prefers_wet_plate_when_more_bowls() {
        let mut c = counts_basic();
        c.set(CLAY, 5);
        c.count_clay_on_floor = 5;
        // One wet bowl only â€” â‰¥2 wet bowls prefer L2946 crock shape (233+233) first.
        c.set(WET_CLAY_BOWL, 1);
        c.set(CLAY_BOWL, 1); // bowl_eff=2 > plate_eff=0 still prefers stoneâ†’wet plate
        c.max_bowls = 3;
        c.max_plates = 3;
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 3.0,
            ..Default::default()
        };
        // countBowl_eff=2, countPlate_eff=0, needed plates > 0, bowls > plates
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: STONE,
                target: WET_CLAY_BOWL
            }
        );
    }

    #[test]
    fn do_pottery_pile_of_clay_when_floor_short() {
        let mut c = counts_basic();
        c.set(CLAY, 5); // clay stock â‰¥4 so stageâ†’2 without gather
        c.count_clay_on_floor = 0;
        c.max_bowls = 3;
        c.max_plates = 3;
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 2.0,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: 0,
                target: PILE_OF_CLAY
            }
        );
    }

    #[test]
    fn do_pottery_no_kiln_aborts_after_gather_stage() {
        let mut c = counts_basic();
        c.kiln_parent_id = None;
        c.firing_kiln = false;
        c.set(CLAY, 5);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 2.0,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::Abort
        );
    }

    #[test]
    fn gather_clay_full_basket_goes_home() {
        let inp = GatherClayInput {
            player_x: 50,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: BASKET,
            held_contained: 3,
            ..Default::default()
        };
        // dist_home = 2500 > 100
        assert_eq!(gather_clay(&inp), PotteryAction::GotoHome);
        let near = GatherClayInput {
            player_x: 5,
            player_y: 0,
            held_id: BASKET,
            held_contained: 3,
            ..inp
        };
        assert_eq!(
            gather_clay(&near),
            PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 40,
            }
        );
    }

    #[test]
    fn gather_clay_seeks_basket_when_none() {
        let inp = GatherClayInput {
            has_clay_deposit: true,
            deposit_x: 10,
            deposit_y: 0,
            ..Default::default()
        };
        assert_eq!(
            gather_clay(&inp),
            PotteryAction::SeekOrCraft {
                object_id: BASKET
            }
        );
    }

    #[test]
    fn gather_clay_digs_when_adjacent_empty_hands() {
        let inp = GatherClayInput {
            player_x: 10,
            player_y: 0,
            has_clay_deposit: true,
            deposit_x: 10,
            deposit_y: 0,
            empty_basket_near_deposit: true,
            ..Default::default()
        };
        assert_eq!(gather_clay(&inp), PotteryAction::UseOnClayDeposit);
    }

    #[test]
    fn gather_clay_empty_basket_deposit_staging_max_dist_0() {
        // Haxe: dropHeldObject(0) when empty basket adjacent deposit
        let inp = GatherClayInput {
            player_x: 10,
            player_y: 0,
            held_id: BASKET,
            held_contained: 0,
            has_clay_deposit: true,
            deposit_x: 10,
            deposit_y: 0,
            ..Default::default()
        };
        assert_eq!(
            gather_clay(&inp),
            PotteryAction::DropHeld {
                allow_piles: false,
                max_distance_to_home: 0,
            }
        );
        assert!(empty_basket_drop_is_deposit_staging(BASKET, 0, true));
    }

    #[test]
    fn gather_clay_full_basket_deposit_only_pickup() {
        // Player far; full basket only at deposit â†’ still PickupBasket
        let inp = GatherClayInput {
            player_x: 0,
            player_y: 0,
            has_clay_deposit: true,
            deposit_x: 50,
            deposit_y: 0,
            basket_full: true,
            full_basket_near_deposit: true,
            ..Default::default()
        };
        assert_eq!(gather_clay(&inp), PotteryAction::PickupBasket);
    }

    #[test]
    fn fill_pottery_counts_from_map_basic() {
        let objs = [
            PotteryMapObj {
                parent_id: ADOBE_KILN,
                x: 2,
                y: 0,
            },
            PotteryMapObj {
                parent_id: CLAY_BOWL,
                x: 1,
                y: 0,
            },
            PotteryMapObj {
                parent_id: CLAY,
                x: 0,
                y: 1,
            },
            PotteryMapObj {
                parent_id: FIRING_ADOBE_KILN,
                x: 3,
                y: 0,
            },
        ];
        let c = fill_pottery_counts_from_map(
            0,
            0,
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
        assert!(c.firing_kiln);
        assert_eq!(c.count_bowl(), 1);
        assert_eq!(c.count_clay_on_floor, 1);
        assert!(c.kiln_parent_id.is_some());
    }

    #[test]
    fn pottery_action_to_goal_maps() {
        assert_eq!(
            pottery_action_to_goal(PotteryAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }),
            Goal::SeekObject(FIRED_BOWL_TONGS)
        );
        assert_eq!(
            pottery_action_to_goal(PotteryAction::ShortCraft {
                actor: STONE,
                target: CLAY
            }),
            Goal::SeekObject(CLAY)
        );
        assert_eq!(
            pottery_action_to_goal(PotteryAction::Abort),
            Goal::SeekObject(POTTER_TARGET_ID)
        );
    }

    #[test]
    fn on_fire_adobe_when_coal_low() {
        let mut c = counts_basic();
        c.firing_kiln = true;
        c.max_bowls = 0; // force skip bowl craft: count_bowl < max fails if max 0?
        // max 0: count_bowl(0) < 0 is false â€” skip bowl
        c.max_bowls = 0;
        c.max_plates = 0;
        c.max_crock = 0;
        // coal 0 < 3 && firing â†’ adobe+kiln
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::ShortCraft {
                actor: ADOBE,
                target: FIRING_ADOBE_KILN
            }
        );
    }

    #[test]
    fn pick_potter_goal_defaults() {
        let g = ReverseCraftGraph::default();
        let have = HashSet::new();
        assert_eq!(
            pick_potter_goal(&g, &have),
            Goal::SeekObject(CLAY_BOWL)
        );
    }

    #[test]
    fn try_decide_and_max_people() {
        assert_eq!(potter_max_people_for_dispatch(true), 100);
        assert_eq!(potter_max_people_for_dispatch(false), 1);
        assert!(potter_job_rung_label("AGE_ROTATED_JOB"));
        assert!(!potter_job_rung_label("ESCAPE"));
        let c = counts_basic();
        let mut rt = PotterProfessionRuntime::default();
        let a = try_decide_potter_from_rung(&c, &mut rt, "AGE_ROTATED_JOB", 0.0, 0.0, false, None);
        assert!(a.is_some());
    }

    #[test]
    fn short_craft_apply_held_actor() {
        use crate::smith_profession::SmithApply;
        let a = PotteryAction::ShortCraft {
            actor: STONE,
            target: CLAY,
        };
        assert_eq!(
            pottery_action_short_craft_apply(a, STONE),
            SmithApply::UseOnTarget {
                actor: STONE,
                target: CLAY
            }
        );
        assert_eq!(
            pottery_action_short_craft_apply(a, 0),
            SmithApply::SeekOrCraftActor { actor: STONE }
        );
    }

    #[test]
    fn peer_filter_and_wipe() {
        let peers = [
            PotterPeerSnapshot {
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: true,
                last_is_potter: true,
            },
            PotterPeerSnapshot {
                deleted: false,
                age: 20.0,
                is_wounded: false,
                food_store: 5.0,
                has_player_to_follow: false,
                same_home: false,
                last_is_potter: true,
            },
        ];
        assert_eq!(count_potter_peers_filtered(&peers, 3.0, 60.0), 1.0);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 10.0,
            ..Default::default()
        };
        rt.wipe_on_eat(false);
        assert_eq!(rt.stage, 0.0);
        assert!(!rt.is_last_potter);
    }

    #[test]
    fn unseal_and_charcoal_kiln_shortcrafts() {
        let mut c = counts_basic();
        c.set(CLAY, 5);
        c.set(SEALED_ADOBE_KILN, 1);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            ..Default::default()
        };
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: 0,
                target: SEALED_ADOBE_KILN
            }
        );
        c.set(SEALED_ADOBE_KILN, 0);
        c.set(KILN_WITH_CHARCOAL, 1);
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: BASKET,
                target: KILN_WITH_CHARCOAL
            }
        );
    }

    #[test]
    fn pick_closest_clay_source_deposit_vs_pit() {
        // Haxe L2968: closest of deposit(125) vs pit(409)
        let cands = [
            ClaySourceCandidate {
                parent_id: CLAY_PIT,
                x: 5,
                y: 0,
            },
            ClaySourceCandidate {
                parent_id: CLAY_DEPOSIT,
                x: 10,
                y: 0,
            },
        ];
        let s = pick_closest_clay_source(0, 0, &cands).unwrap();
        assert_eq!(s.parent_id, CLAY_PIT);
        assert_eq!((s.x, s.y), (5, 0));
        // Equal distance â†’ prefer deposit
        let tie = [
            ClaySourceCandidate {
                parent_id: CLAY_PIT,
                x: 4,
                y: 0,
            },
            ClaySourceCandidate {
                parent_id: CLAY_DEPOSIT,
                x: 4,
                y: 0,
            },
        ];
        let s2 = pick_closest_clay_source(0, 0, &tie).unwrap();
        assert_eq!(s2.parent_id, CLAY_DEPOSIT);
        let mut inp = GatherClayInput::default();
        apply_clay_source_to_gather_input(&mut inp, Some(s));
        assert!(inp.has_clay_deposit);
        assert_eq!((inp.deposit_x, inp.deposit_y), (5, 0));
    }

    #[test]
    fn empty_basket_drop_staging_and_critical_max() {
        assert!(empty_basket_drop_is_deposit_staging(BASKET, 0, true));
        assert!(!empty_basket_drop_is_deposit_staging(BASKET, 0, false));
        assert!(!empty_basket_drop_is_deposit_staging(BASKET, 3, true));
        // Haxe doPottery(-2): high prio does job without assigning profession
        let mut rt = PotterProfessionRuntime::default();
        let c = counts_basic();
        let a = do_pottery(&c, &mut rt, POTTER_CRITICAL_MAX_PEOPLE, 99.0, 0.0, None);
        assert!(a.is_some() || matches!(a, PotteryAction::None | PotteryAction::SeekOrCraft { .. } | PotteryAction::Abort));
        assert!(!rt.is_last_potter); // max < 0 does not assign
        let mut rt2 = PotterProfessionRuntime::default();
        assert!(has_or_become_potter(
            &mut rt2,
            POTTER_CRITICAL_MAX_PEOPLE,
            99.0,
            0.0
        ));
        assert!(!rt2.is_last_potter);
        let a2 = try_decide_potter_from_rung(
            &c,
            &mut rt2,
            "CRITICAL_POTTERY",
            99.0,
            0.0,
            false,
            None,
        );
        assert!(a2.is_some());
    }

    #[test]
    fn empty_basket_at_home_drop_extract_and_gather() {
        // Haxe L3013: empty hands â†’ dropIsAUse=false DROP extract, not USE whole basket
        assert!(empty_basket_at_home_is_drop_extract(0));
        assert!(!empty_basket_at_home_is_drop_extract(CLAY));
        assert!(!empty_basket_at_home_is_drop_extract(BASKET));
        // gatherClay: close home + basket with clay â†’ EmptyBasketAtHome when empty hands
        let inp = GatherClayInput {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            held_id: 0,
            basket_with_clay_near_home: true,
            has_clay_deposit: true,
            deposit_x: 50,
            deposit_y: 0,
            ..Default::default()
        };
        assert_eq!(gather_clay(&inp), PotteryAction::EmptyBasketAtHome);
        // holding something first â†’ DropHeld maxDist 1 allow piles
        let mut held = inp;
        held.held_id = STONE;
        assert_eq!(
            gather_clay(&held),
            PotteryAction::DropHeld {
                allow_piles: true,
                max_distance_to_home: 1,
            }
        );
        assert_eq!(EMPTY_BASKET_HOME_SEARCH_RADIUS, 10);
    }

    #[test]
    fn wet_nozzle_cleanup_merge_and_clay_with_nozzle() {
        // Haxe cleanUp ~L1031â€“1036: merge when count>1
        assert_eq!(
            wet_nozzle_cleanup_action(2, 0, false),
            Some(PotteryAction::ShortCraft {
                actor: WET_CLAY_NOZZLE,
                target: WET_CLAY_NOZZLE,
            })
        );
        // count>0 && held 285
        assert_eq!(
            wet_nozzle_cleanup_action(1, WET_CLAY_NOZZLE, false),
            Some(PotteryAction::ShortCraft {
                actor: WET_CLAY_NOZZLE,
                target: WET_CLAY_NOZZLE,
            })
        );
        // single ground, empty hands â†’ no merge; try 2110
        assert_eq!(
            wet_nozzle_cleanup_action(1, 0, true),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: CLAY_WITH_NOZZLE,
            })
        );
        // no wet nozzles, 2110 present
        assert_eq!(
            wet_nozzle_cleanup_action(0, 0, true),
            Some(PotteryAction::ShortCraft {
                actor: 0,
                target: CLAY_WITH_NOZZLE,
            })
        );
        assert_eq!(wet_nozzle_cleanup_action(0, 0, false), None);
        assert_eq!(wet_nozzle_cleanup_action(1, 0, false), None);
        // do_pottery wires cleanup before fire/gather
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 0.0,
            ..Default::default()
        };
        let mut c = counts_basic();
        c.set(WET_CLAY_NOZZLE, 2);
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: WET_CLAY_NOZZLE,
                target: WET_CLAY_NOZZLE,
            }
        );
        c.set(WET_CLAY_NOZZLE, 0);
        c.set(CLAY_WITH_NOZZLE, 1);
        assert_eq!(
            do_pottery(&c, &mut rt, 1, 0.0, 0.0, None),
            PotteryAction::ShortCraft {
                actor: 0,
                target: CLAY_WITH_NOZZLE,
            }
        );
    }

    #[test]
    fn smith_on_fire_bridge_and_short_craft_drop() {
        // Shared do_pottery_on_fire via smith body maps into PotteryAction
        let mut c = counts_basic();
        c.firing_kiln = true;
        c.count_close_bowl = 5;
        c.max_bowls = 3;
        let a = do_pottery_on_fire_action(&c);
        assert_eq!(
            a,
            PotteryAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );
        use crate::smith_profession::SmithApply;
        assert_eq!(
            pottery_action_short_craft_apply(
                PotteryAction::DropHeld {
                    allow_piles: true,
                    max_distance_to_home: 10,
                },
                126
            ),
            SmithApply::DropHeld
        );
        assert_eq!(
            pottery_action_short_craft_apply(
                PotteryAction::ShortCraftOnGround {
                    target: FIRED_BOWL_TONGS
                },
                0
            ),
            crate::smith_profession::short_craft_on_ground_apply(0, FIRED_BOWL_TONGS)
        );
    }

    /// AI-POTTER-L2946: residual doPotteryOnFire other crafts via potter bridge.
    #[test]
    fn l2946_other_pottery_crafts_via_on_fire_action() {
        let mut c = counts_basic();
        c.firing_kiln = true;
        c.max_bowls = 3;
        c.count_close_bowl = 0;
        c.set(WET_CLAY_BOWL, 2);
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );

        c.set(CLAY_BOWL, 3);
        c.count_close_bowl = 3;
        c.set(WET_CLAY_BOWL, 2);
        c.max_crock = 2;
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );

        c.set(WET_CLAY_BOWL, 0);
        c.set(WET_CLAY_NOZZLE, 1);
        c.max_nozzle = 2;
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::CraftItem {
                object_id: FIRED_NOZZLE_TONGS
            }
        );
    }

    #[test]
    fn l2946_do_pottery_shapes_wet_crock_at_stage3() {
        let mut c = counts_basic();
        c.set(CLAY, 5);
        c.set(CLAY_BOWL, 3);
        c.set(CLAY_PLATE, 2);
        c.set(WET_CLAY_BOWL, 2);
        c.max_bowls = 3;
        c.max_plates = 2;
        c.max_crock = 2;
        c.kiln_parent_id = Some(ADOBE_KILN);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 3.0,
            ..Default::default()
        };
        let a = do_pottery(&c, &mut rt, 1, 0.0, 0.0, None);
        assert_eq!(
            a,
            PotteryAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );
    }
}
