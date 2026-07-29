//! Eve/Adam wild spawn location near food plants (Haxe `spawnAsEve` + jungle banana).
//!
//! Haxe references:
//! - `GlobalPlayerInstance.spawnAsEve` L1107–1261
//! - `ClearStartLocations` L1263–1273 (foodArray filter)
//! - TimeHelper plant indexes: berry 30, banana 2142, carrot 36@snow,
//!   cactus 761@desert, garlic 4251@grey
//! - TODO L1122 "spawn eve in jungle with bananaplants" — implemented as
//!   stronger banana pool pick + jungle biome fitness bonus
//! - `getCloseSpecialBiomePersonColor` L1387–1439
//!
//! Pure rules + thin world scan helpers (`collect_eve_food_sites` / `find_eve_spawn`).

use crate::player::multi_use::{
    BIOME_DESERT, BIOME_GREY, BIOME_JUNGLE, BIOME_SNOW, PERSON_BLACK, PERSON_BROWN, PERSON_GINGER,
    PERSON_WHITE,
};
use ol_world::World;

/// Wild Gooseberry Bush — primary berry start pool.
pub const EVE_BERRY_BUSH: i32 = 30;
/// Banana Plant — jungle loved food / Eve start.
pub const EVE_BANANA_PLANT: i32 = 2142;
/// Seeding Wild Carrot (snow index).
pub const EVE_WILD_CARROT_SEED: i32 = 36;
/// Barrel Cactus (desert index).
pub const EVE_BARREL_CACTUS: i32 = 761;
/// Flowering Barrel Cactus (ClearStartLocations foodArray).
pub const EVE_FLOWERING_CACTUS: i32 = 762;
/// Fruiting Barrel Cactus.
pub const EVE_FRUITING_CACTUS: i32 = 763;
/// Wild Garlic (grey index).
pub const EVE_WILD_GARLIC: i32 = 4251;
/// Dug / alt wild carrot id in foodArray.
pub const EVE_WILD_CARROT_ALT: i32 = 404;

/// Haxe `ClearStartLocations` foodArray.
pub const EVE_START_FOOD_IDS: &[i32] = &[
    EVE_BERRY_BUSH,
    EVE_BANANA_PLANT,
    EVE_WILD_CARROT_SEED,
    EVE_BARREL_CACTUS,
    EVE_WILD_GARLIC,
    EVE_FLOWERING_CACTUS,
    EVE_FRUITING_CACTUS,
    EVE_WILD_CARROT_ALT,
];

/// Min banana+berry count before food-plant spawn is allowed (else startingGx/Gy).
/// Haxe: `bananaPlantsTmp.length + berryBushesTmp.length < 10`
pub const EVE_MIN_FOOD_SITES: usize = 10;

/// Samples tried per Eve spawn (Haxe `for (i in 0...20)`).
pub const EVE_LOCATION_SAMPLES: usize = 20;

/// Default Chebyshev scan radius around prefer when collecting food plants.
pub const EVE_FOOD_SCAN_RADIUS: i32 = 120;

/// Cap candidates collected (full maps can have thousands of bushes).
pub const EVE_FOOD_SITE_CAP: usize = 400;

/// Jungle / border jungle biome ids for banana preference.
pub const JUNGLE_BIOME: u8 = 6;
pub const BORDER_JUNGLE_BIOME: u8 = 15;

/// Haxe start-location food pool kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EveFoodPool {
    Berry,
    Banana,
    WildCarrot,
    Cactus,
    WildGarlic,
}

/// One candidate food-plant tile for Eve spawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EveFoodSite {
    pub x: i32,
    pub y: i32,
    /// Object parent/base id.
    pub object_id: i32,
    /// Haxe `numberOfUses` (0 when bare).
    pub uses: i32,
    pub biome: u8,
    pub pool: EveFoodPool,
}

/// Counts of cleared pool candidates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EveFoodPoolCounts {
    pub berry: usize,
    pub banana: usize,
    pub wild_carrot: usize,
    pub cactus: usize,
    pub wild_garlic: usize,
}

impl EveFoodPoolCounts {
    pub fn from_sites(sites: &[EveFoodSite]) -> Self {
        let mut c = Self::default();
        for s in sites {
            match s.pool {
                EveFoodPool::Berry => c.berry += 1,
                EveFoodPool::Banana => c.banana += 1,
                EveFoodPool::WildCarrot => c.wild_carrot += 1,
                EveFoodPool::Cactus => c.cactus += 1,
                EveFoodPool::WildGarlic => c.wild_garlic += 1,
            }
        }
        c
    }

    pub fn banana_plus_berry(&self) -> usize {
        self.banana + self.berry
    }

    pub fn count(&self, pool: EveFoodPool) -> usize {
        match pool {
            EveFoodPool::Berry => self.berry,
            EveFoodPool::Banana => self.banana,
            EveFoodPool::WildCarrot => self.wild_carrot,
            EveFoodPool::Cactus => self.cactus,
            EveFoodPool::WildGarlic => self.wild_garlic,
        }
    }
}

/// True when id is in Haxe `ClearStartLocations` foodArray.
#[inline]
pub fn is_eve_start_food_id(object_id: i32) -> bool {
    EVE_START_FOOD_IDS.contains(&object_id)
}

/// Classify a map object into an Eve food pool (None if not a start food).
///
/// Biome gates match TimeHelper index rules for carrot/cactus/garlic.
// Haxe: TimeHelper berryBushes/bananaPlants/… indexes + ClearStartLocations
pub fn classify_eve_food_site(object_id: i32, biome: u8) -> Option<EveFoodPool> {
    if !is_eve_start_food_id(object_id) {
        return None;
    }
    match object_id {
        EVE_BERRY_BUSH => Some(EveFoodPool::Berry),
        EVE_BANANA_PLANT => Some(EveFoodPool::Banana),
        EVE_WILD_CARROT_SEED => {
            if biome == BIOME_SNOW as u8 {
                Some(EveFoodPool::WildCarrot)
            } else {
                None
            }
        }
        EVE_WILD_CARROT_ALT => Some(EveFoodPool::WildCarrot),
        // 761 desert-only index; 762/763 always in foodArray
        EVE_BARREL_CACTUS => {
            if biome == BIOME_DESERT as u8 {
                Some(EveFoodPool::Cactus)
            } else {
                None
            }
        }
        EVE_FLOWERING_CACTUS | EVE_FRUITING_CACTUS => Some(EveFoodPool::Cactus),
        EVE_WILD_GARLIC => {
            if biome == BIOME_GREY as u8 {
                Some(EveFoodPool::WildGarlic)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Haxe `PersonColor.getPersonColorByBiome` — special biomes only; else 0.
// Haxe: ObjectData.PersonColor.getPersonColorByBiome
#[inline]
pub fn person_color_by_biome(biome: i32) -> i32 {
    match biome {
        b if b == BIOME_DESERT => PERSON_BLACK,
        b if b == BIOME_JUNGLE => PERSON_BROWN,
        b if b == BIOME_GREY => PERSON_WHITE,
        b if b == BIOME_SNOW => PERSON_GINGER,
        // Border jungle still maps to brown (loved jungle).
        15 => PERSON_BROWN,
        _ => 0,
    }
}

/// Diagonal + cross ring search for nearest special biome person color.
///
/// Returns 0 when none within `max_search` (Haxe returns −1; Rust uses 0 = none).
// Haxe: GlobalPlayerInstance.getCloseSpecialBiomePersonColor
pub fn get_close_special_biome_person_color(
    get_biome: impl Fn(i32, i32) -> u8,
    x: i32,
    y: i32,
    max_search: i32,
) -> i32 {
    let max_search = max_search.max(0);
    for ii in 0..=max_search {
        let samples = [
            (x + ii, y + ii),
            (x - ii, y + ii),
            (x + ii, y - ii),
            (x - ii, y - ii),
            (x + ii, y),
            (x - ii, y),
            (x, y + ii),
            (x, y - ii),
        ];
        for (sx, sy) in samples {
            let pc = person_color_by_biome(get_biome(sx, sy) as i32);
            if pc > 0 {
                return pc;
            }
        }
    }
    0
}

/// Pick which food pool Eve uses for this spawn.
///
/// Haxe base: default berry; `rand==1 && banana>5` → banana; 2→carrot; 3→cactus; 4→garlic.
/// Fallback when chosen empty: banana if any, else berry.
///
/// **Jungle banana preference (Haxe TODO L1122 + commented L1156):**
/// when `prefer_jungle_banana` and `banana > 10`, treat `rand_0_to_4 ∈ {0,1}` as banana
/// (≈40% vs Haxe 20% at rand==1 only); when `banana > 5`, also allow rand==0.
// Haxe: spawnAsEve startLocations selection + TODO spawn in jungle with bananaplants
pub fn select_eve_food_pool(
    counts: &EveFoodPoolCounts,
    rand_0_to_4: u32,
    prefer_jungle_banana: bool,
) -> EveFoodPool {
    let rand = rand_0_to_4.min(4);
    let mut pool = EveFoodPool::Berry;

    // Stronger banana preference when abundant (implements Haxe TODO + commented line).
    let banana_boost = prefer_jungle_banana
        && ((counts.banana > 10 && rand <= 1) || (counts.banana > 5 && rand == 0));

    if (rand == 1 || banana_boost) && counts.banana > 5 {
        pool = EveFoodPool::Banana;
    } else if rand == 2 && counts.wild_carrot > 5 {
        pool = EveFoodPool::WildCarrot;
    } else if rand == 3 && counts.cactus > 5 {
        pool = EveFoodPool::Cactus;
    } else if rand == 4 && counts.wild_garlic > 5 {
        pool = EveFoodPool::WildGarlic;
    } else if counts.berry > 0 {
        pool = EveFoodPool::Berry;
    }

    // Haxe: if startLocations null → banana then berry
    if counts.count(pool) == 0 {
        if counts.banana > 0 {
            return EveFoodPool::Banana;
        }
        if counts.berry > 0 {
            return EveFoodPool::Berry;
        }
        if counts.wild_carrot > 0 {
            return EveFoodPool::WildCarrot;
        }
        if counts.cactus > 0 {
            return EveFoodPool::Cactus;
        }
        if counts.wild_garlic > 0 {
            return EveFoodPool::WildGarlic;
        }
    }
    pool
}

/// True when SpwanAtLastDead or too few banana+berry sites → use startingGx/Gy.
// Haxe: SpwanAtLastDead || banana+berry < 10
pub fn use_fixed_starting_spawn(spawn_at_last_dead: bool, counts: &EveFoodPoolCounts) -> bool {
    spawn_at_last_dead || counts.banana_plus_berry() < EVE_MIN_FOOD_SITES
}

/// Location fitness for one Eve candidate (higher = better).
///
/// Haxe:
/// ```text
/// fitness = 1 + numberOfUses
///   + (hasCloseNonBlockingGrave ? 1 : 0)
/// sumDistHumans = 1 + (blockingGrave ? 1 : 0) + Σ 10000/quadDist
/// total = fitness / sumDistHumans
/// ```
/// Jungle banana bonus (TODO preference): +0.5 fitness when banana on jungle tile.
// Haxe: spawnAsEve location scoring loop
pub fn eve_location_fitness(
    site: &EveFoodSite,
    player_xy: &[(i32, i32)],
    has_close_blocking_grave: bool,
    has_close_nonblocking_grave: bool,
) -> f32 {
    let mut fitness = 1.0 + site.uses.max(0) as f32;
    if has_close_nonblocking_grave {
        fitness += 1.0;
    }
    // Jungle banana preference: rank banana plants in jungle higher.
    if site.pool == EveFoodPool::Banana
        && (site.biome == JUNGLE_BIOME || site.biome == BORDER_JUNGLE_BIOME)
    {
        fitness += 0.5;
    }

    let mut sum_dist_humans = 1.0f32;
    if has_close_blocking_grave {
        sum_dist_humans += 1.0;
    }
    for &(px, py) in player_xy {
        let dx = (px - site.x) as f32;
        let dy = (py - site.y) as f32;
        // Haxe AiHelper.CalculateQuadDistanceToObject → dx²+dy² style; +1 then 10000/quad
        let quad = 1.0 + dx * dx + dy * dy;
        sum_dist_humans += 10000.0 / quad;
    }

    fitness / sum_dist_humans.max(1e-6)
}

/// Pick best site among up to `samples` random draws from `pool_sites`.
///
/// `index_fn(n)` returns `0..n` (Haxe `calculateRandomInt(n-1)` → 0..=n-1).
// Haxe: spawnAsEve bestLocation loop
pub fn pick_best_eve_site(
    pool_sites: &[EveFoodSite],
    player_xy: &[(i32, i32)],
    samples: usize,
    mut index_fn: impl FnMut(usize) -> usize,
    grave_blocking: impl Fn(i32, i32) -> bool,
    grave_nonblocking: impl Fn(i32, i32) -> bool,
) -> Option<EveFoodSite> {
    if pool_sites.is_empty() {
        return None;
    }
    let n = pool_sites.len();
    let trials = samples.max(1);
    let mut best: Option<(EveFoodSite, f32)> = None;
    for _ in 0..trials {
        let idx = index_fn(n) % n;
        let site = pool_sites[idx];
        let block = grave_blocking(site.x, site.y);
        let nonblock = grave_nonblocking(site.x, site.y);
        let fit = eve_location_fitness(&site, player_xy, block, nonblock);
        match best {
            Some((_, b)) if b >= fit => {}
            _ => best = Some((site, fit)),
        }
    }
    best.map(|(s, _)| s)
}

/// Filter sites belonging to `pool`.
pub fn sites_for_pool(sites: &[EveFoodSite], pool: EveFoodPool) -> Vec<EveFoodSite> {
    sites.iter().copied().filter(|s| s.pool == pool).collect()
}

/// Full pure decision: fixed start or best food site.
///
/// Returns `None` when caller should use `starting_gx/gy` (too few plants / last-dead /
/// empty best).
// Haxe: spawnAsEve gx/gy selection
pub fn resolve_eve_spawn_site(
    sites: &[EveFoodSite],
    player_xy: &[(i32, i32)],
    spawn_at_last_dead: bool,
    prefer_jungle_banana: bool,
    rand_0_to_4: u32,
    samples: usize,
    index_fn: impl FnMut(usize) -> usize,
    grave_blocking: impl Fn(i32, i32) -> bool,
    grave_nonblocking: impl Fn(i32, i32) -> bool,
) -> Option<EveFoodSite> {
    let counts = EveFoodPoolCounts::from_sites(sites);
    if use_fixed_starting_spawn(spawn_at_last_dead, &counts) {
        return None;
    }
    let pool = select_eve_food_pool(&counts, rand_0_to_4, prefer_jungle_banana);
    let pool_sites = sites_for_pool(sites, pool);
    if pool_sites.is_empty() {
        return None;
    }
    pick_best_eve_site(
        &pool_sites,
        player_xy,
        samples,
        index_fn,
        grave_blocking,
        grave_nonblocking,
    )
}

// ── World scan + live pick ──────────────────────────────────────────────────

fn push_site_if_food(
    out: &mut Vec<EveFoodSite>,
    seen: &mut std::collections::HashSet<(i32, i32)>,
    world: &World,
    x: i32,
    y: i32,
    object_id: i32,
    uses: i32,
) {
    if out.len() >= EVE_FOOD_SITE_CAP {
        return;
    }
    let (x, y) = world.wrap_tile(x, y);
    if !seen.insert((x, y)) {
        return;
    }
    let biome = world.get_biome(x, y);
    let Some(pool) = classify_eve_food_site(object_id, biome) else {
        return;
    };
    out.push(EveFoodSite {
        x,
        y,
        object_id,
        uses,
        biome,
        pool,
    });
}

/// Collect Eve food-plant candidates near `prefer` (helpers + ring scan).
///
/// Haxe keeps global maps; Rust scans resident tiles + complex helpers.
// Haxe: WorldMap.world.bananaPlants / berryBushes / …
pub fn collect_eve_food_sites(
    world: &World,
    prefer: (i32, i32),
    scan_radius: i32,
) -> Vec<EveFoodSite> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let r = scan_radius.max(0);

    // Complex helpers first (multi-use plants often live here).
    for (&(tx, ty), h) in &world.helpers {
        if out.len() >= EVE_FOOD_SITE_CAP {
            break;
        }
        if !is_eve_start_food_id(h.base_id) {
            continue;
        }
        // Prefer plants near prefer point when over cap pressure — still take all until cap.
        let _ = (tx, ty);
        push_site_if_food(
            &mut out,
            &mut seen,
            world,
            tx,
            ty,
            h.base_id,
            h.uses_remaining,
        );
    }

    // Dense ring scan for bare object ids (berry/banana without helper).
    let (px, py) = prefer;
    for dy in -r..=r {
        if out.len() >= EVE_FOOD_SITE_CAP {
            break;
        }
        for dx in -r..=r {
            if out.len() >= EVE_FOOD_SITE_CAP {
                break;
            }
            let x = px + dx;
            let y = py + dy;
            if seen.contains(&world.wrap_tile(x, y)) {
                continue;
            }
            let id = world.get_object(x, y);
            if !is_eve_start_food_id(id) {
                continue;
            }
            let uses = world
                .get_helper(x, y)
                .map(|h| h.uses_remaining)
                .unwrap_or(0);
            push_site_if_food(&mut out, &mut seen, world, x, y, id, uses);
        }
    }

    out
}

/// Options for [`find_eve_spawn`].
#[derive(Debug, Clone, Copy)]
pub struct EveSpawnOpts {
    pub spawn_at_last_dead: bool,
    /// Implement Haxe TODO: prefer jungle banana plants.
    pub prefer_jungle_banana: bool,
    pub scan_radius: i32,
    pub samples: usize,
}

impl Default for EveSpawnOpts {
    fn default() -> Self {
        Self {
            spawn_at_last_dead: false,
            prefer_jungle_banana: true,
            scan_radius: EVE_FOOD_SCAN_RADIUS,
            samples: EVE_LOCATION_SAMPLES,
        }
    }
}

// ── Last Eve/Adam pairing (Haxe lastAiEveOrAdam / lastHumanEveOrAdam) ─────────

/// Snapshot of the unpaired Eve/Adam waiting for a spirit-mate.
// Haxe: GlobalPlayerInstance.lastAiEveOrAdam / lastHumanEveOrAdam
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastEveSlot {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// Haxe `getColor()` / `ObjectData.person` race (0 = unset).
    pub person_color: i32,
    pub is_female: bool,
}

/// Result of resolving the pair partner for a new Eve spawn.
// Haxe: spawnAsEve L1107–1117
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvePairResolve {
    /// Living unpaired Eve/Adam to co-spawn with (None ⇒ founder).
    pub partner: Option<LastEveSlot>,
    /// Haxe clears both AI+human last slots when cross-spawning.
    pub clear_both_pools: bool,
}

/// Haxe `MaxPlayersBeforeStartingAsChild` default 0 → cross only when ≤0 living.
pub const MAX_PLAYERS_BEFORE_STARTING_AS_CHILD: usize = 0;

/// True when living population allows AI↔human Eve pair cross-spawn.
// Haxe: GetNumberLifingPlayers() <= MaxPlayersBeforeStartingAsChild
#[inline]
pub fn allow_human_ai_eve_cross(living_players: usize) -> bool {
    living_players <= MAX_PLAYERS_BEFORE_STARTING_AS_CHILD
}

/// Resolve last Eve/Adam partner for this birth (own pool, else optional cross).
// Haxe: GlobalPlayerInstance.spawnAsEve L1107–1117
pub fn resolve_eve_pair_partner(
    is_ai: bool,
    last_ai: Option<LastEveSlot>,
    last_human: Option<LastEveSlot>,
    allow_cross: bool,
) -> EvePairResolve {
    let own = if is_ai { last_ai } else { last_human };
    if own.is_some() {
        return EvePairResolve {
            partner: own,
            clear_both_pools: false,
        };
    }
    if allow_cross {
        let other = if is_ai { last_human } else { last_ai };
        if other.is_some() {
            return EvePairResolve {
                partner: other,
                clear_both_pools: true,
            };
        }
    }
    EvePairResolve {
        partner: None,
        clear_both_pools: false,
    }
}

/// After spawnAsEve: write back last AI/human slots.
///
/// Haxe stores local `lastEveOrAdam` which is `this` (founder) or `null` (after pair).
// Haxe: spawnAsEve L1220–1260
pub fn apply_eve_pair_slot_update(
    is_ai: bool,
    prev_ai: Option<LastEveSlot>,
    prev_human: Option<LastEveSlot>,
    resolve: EvePairResolve,
    self_slot: LastEveSlot,
) -> (Option<LastEveSlot>, Option<LastEveSlot>) {
    let mut ai = prev_ai;
    let mut human = prev_human;
    if resolve.clear_both_pools {
        ai = None;
        human = None;
    }
    // Founder: lastEveOrAdam = this; pair: lastEveOrAdam = null after co-spawn.
    let store = if resolve.partner.is_some() {
        None
    } else {
        Some(self_slot)
    };
    if is_ai {
        ai = store;
    } else {
        human = store;
    }
    (ai, human)
}

/// Drop last-slot when the living player is gone (Haxe birth: deleted → null).
// Haxe: birth L974–975
#[inline]
pub fn clear_deleted_last_eve(slot: Option<LastEveSlot>, still_alive: impl FnOnce(i32) -> bool) -> Option<LastEveSlot> {
    match slot {
        Some(s) if still_alive(s.p_id) => Some(s),
        _ => None,
    }
}

// ── Race person object + Eve identity ────────────────────────────────────────

/// Haxe `ServerSettings.StartingEveAge`.
pub const STARTING_EVE_AGE: f32 = 14.0;
/// Haxe `ServerSettings.ChanceForFemaleChild` (founder uses `>= 0.5` ⇒ female).
pub const CHANCE_FOR_FEMALE_CHILD: f32 = 0.6;
/// Buried grave — not a gravestone for non-blocking fitness.
pub const BURIED_GRAVE_ID: i32 = 1011;

/// Haxe founder sex: `ChanceForFemaleChild >= 0.5` → female.
// Haxe: spawnAsEve L1228
#[inline]
pub fn founder_eve_is_female(chance_for_female: f32) -> bool {
    chance_for_female >= 0.5
}

/// Haxe pairmate sex is opposite of last Eve/Adam.
// Haxe: spawnAsEve L1247 `female = lastEveOrAdam.isFemale() == false`
#[inline]
pub fn pairmate_eve_is_female(partner_is_female: bool) -> bool {
    !partner_is_female
}

/// Display first name for Eve/Adam wild birth.
// Haxe: spawnAsEve L1257 `name = isFemale() ? "EVE" : "ADAM"`
#[inline]
pub fn eve_adam_first_name(is_female: bool) -> &'static str {
    if is_female {
        "EVE"
    } else {
        "ADAM"
    }
}

/// Pick one person object id from a race gender list (Haxe `persons[rand]`).
// Haxe: ObjectData.femaleByRaceObjectData / maleByRaceObjectData
#[inline]
pub fn pick_person_object_from_list(ids: &[i32], index: usize) -> Option<i32> {
    if ids.is_empty() {
        None
    } else {
        Some(ids[index % ids.len()])
    }
}

/// Collect person object ids for a race color + sex from content tables.
///
/// Skips descriptions containing "Jason" (Haxe CreatePersonArray filter).
/// Sex from name/description heuristic (ObjectData.male not loaded in content yet).
// Haxe: ObjectData.CreatePersonArray + femaleByRaceObjectData / maleByRaceObjectData
pub fn collect_person_ids_for_race(
    person_race: &std::collections::HashMap<i32, i32>,
    object_name_desc: impl Fn(i32) -> (String, String),
    race_color: i32,
    want_female: bool,
) -> Vec<i32> {
    if race_color <= 0 {
        return Vec::new();
    }
    let mut ids: Vec<i32> = person_race
        .iter()
        .filter_map(|(&id, &race)| {
            if race != race_color {
                return None;
            }
            let (name, desc) = object_name_desc(id);
            if name.contains("Jason") || desc.contains("Jason") {
                return None;
            }
            let female = crate::person_looks_female(id, &name, &desc);
            if female == want_female {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    ids.sort_unstable();
    ids
}

/// Convenience: pick race person object or `None` if table empty / race 0.
// Haxe: setObjectId(persons[rand].id)
pub fn pick_eve_race_person_object(
    person_race: &std::collections::HashMap<i32, i32>,
    object_name_desc: impl Fn(i32) -> (String, String),
    race_color: i32,
    want_female: bool,
    rand_index: usize,
) -> Option<i32> {
    let ids = collect_person_ids_for_race(person_race, object_name_desc, race_color, want_female);
    pick_person_object_from_list(&ids, rand_index)
}

// ── Account grave fitness filters ────────────────────────────────────────────

/// Split account grave tiles into bone (blocking) vs gravestone (non-blocking).
///
/// Haxe: `isBoneGrave` vs `isGraveWithGraveStone` (!bone && id != Buried 1011).
// Haxe: PlayerAccount.hasCloseBlockingGrave / hasCloseNonBlockingGrave
pub fn split_account_graves_for_eve(
    graves: &[(i32, i32)],
    object_at: impl Fn(i32, i32) -> i32,
) -> (Vec<(i32, i32)>, Vec<(i32, i32)>) {
    let mut bone = Vec::new();
    let mut stone = Vec::new();
    for &(gx, gy) in graves {
        let id = object_at(gx, gy);
        if id <= 0 {
            // Unknown object: treat as blocking (conservative; matches bone-curse path).
            bone.push((gx, gy));
            continue;
        }
        if crate::animal_move::is_bone_grave(id) {
            bone.push((gx, gy));
        } else if id != BURIED_GRAVE_ID {
            stone.push((gx, gy));
        }
    }
    (bone, stone)
}

/// True when account has a close bone grave (Haxe fitness threshold > 1).
// Haxe: PlayerAccount.hasCloseBlockingGrave
pub fn account_has_close_blocking_grave(
    tx: i32,
    ty: i32,
    bone_graves: &[(i32, i32)],
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    crate::move_live_gates::has_close_blocking_grave(
        tx,
        ty,
        bone_graves,
        crate::move_live_gates::GRAVE_BLOCKING_DISTANCE,
        map_w,
        map_h,
        wrap,
    )
}

/// True when account has a close gravestone (boosts Eve location fitness).
// Haxe: PlayerAccount.hasCloseNonBlockingGrave
pub fn account_has_close_nonblocking_grave(
    tx: i32,
    ty: i32,
    stone_graves: &[(i32, i32)],
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    // Same distance/fitness formula as blocking; different grave set.
    crate::move_live_gates::has_close_blocking_grave(
        tx,
        ty,
        stone_graves,
        crate::move_live_gates::GRAVE_BLOCKING_DISTANCE,
        map_w,
        map_h,
        wrap,
    )
}

/// Pick Eve/Adam spawn tile near food plants; fall back to `fallback` when sparse/empty.
///
/// `index_fn(n)` / `rand_0_to_4` supplied by caller (tests inject determinism).
/// `account_graves` enables Haxe account bone/stone fitness (empty = ignore graves).
// Haxe: GlobalPlayerInstance.spawnAsEve gx/gy block
pub fn find_eve_spawn_with_rng(
    world: &World,
    prefer: (i32, i32),
    player_xy: &[(i32, i32)],
    fallback: (i32, i32),
    opts: EveSpawnOpts,
    rand_0_to_4: u32,
    index_fn: impl FnMut(usize) -> usize,
) -> (i32, i32) {
    find_eve_spawn_with_rng_graves(
        world,
        prefer,
        player_xy,
        fallback,
        opts,
        rand_0_to_4,
        index_fn,
        &[],
    )
}

/// Like [`find_eve_spawn_with_rng`] with account grave fitness callbacks.
// Haxe: spawnAsEve hasCloseBlockingGrave / hasCloseNonBlockingGrave
pub fn find_eve_spawn_with_rng_graves(
    world: &World,
    prefer: (i32, i32),
    player_xy: &[(i32, i32)],
    fallback: (i32, i32),
    opts: EveSpawnOpts,
    rand_0_to_4: u32,
    mut index_fn: impl FnMut(usize) -> usize,
    account_graves: &[(i32, i32)],
) -> (i32, i32) {
    let sites = collect_eve_food_sites(world, prefer, opts.scan_radius);
    let (bone, stone) = split_account_graves_for_eve(account_graves, |x, y| world.get_object(x, y));
    let mw = world.width_tiles;
    let mh = world.height_tiles;
    let wrap = world.wrap;
    if let Some(site) = resolve_eve_spawn_site(
        &sites,
        player_xy,
        opts.spawn_at_last_dead,
        opts.prefer_jungle_banana,
        rand_0_to_4,
        opts.samples,
        &mut index_fn,
        |x, y| account_has_close_blocking_grave(x, y, &bone, mw, mh, wrap),
        |x, y| account_has_close_nonblocking_grave(x, y, &stone, mw, mh, wrap),
    ) {
        return (site.x, site.y);
    }
    fallback
}

/// Live Eve spawn: thread_rng + food-plant preference, else `fallback`.
pub fn find_eve_spawn(
    world: &World,
    prefer: (i32, i32),
    player_xy: &[(i32, i32)],
    fallback: (i32, i32),
) -> (i32, i32) {
    find_eve_spawn_for_account(world, prefer, player_xy, fallback, EveSpawnOpts::default(), &[])
}

/// Live Eve spawn with opts + account graves (SpwanAtLastDead / grave fitness).
// Haxe: ServerSettings.SpwanAtLastDead + account graves
pub fn find_eve_spawn_for_account(
    world: &World,
    prefer: (i32, i32),
    player_xy: &[(i32, i32)],
    fallback: (i32, i32),
    opts: EveSpawnOpts,
    account_graves: &[(i32, i32)],
) -> (i32, i32) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let rand_pool = rng.gen_range(0u32..=4);
    find_eve_spawn_with_rng_graves(
        world,
        prefer,
        player_xy,
        fallback,
        opts,
        rand_pool,
        |n| {
            if n == 0 {
                0
            } else {
                rng.gen_range(0..n)
            }
        },
        account_graves,
    )
}

/// Person color for a new Eve at (x,y) from nearest special **live** biome.
pub fn eve_person_color_at(world: &World, x: i32, y: i32) -> i32 {
    get_close_special_biome_person_color(|tx, ty| world.get_biome(tx, ty), x, y, 200)
}

/// Person color via caller biome source (Haxe `originalBiome=true` → getOriginalBiomeId).
// Haxe: getCloseSpecialBiomePersonColor(x, y, originalBiome=true)
pub fn eve_person_color_with_biome(
    get_biome: impl Fn(i32, i32) -> u8,
    x: i32,
    y: i32,
) -> i32 {
    get_close_special_biome_person_color(get_biome, x, y, 200)
}

/// Prefer original biome when stored, else live world biome (Eve founder race).
// Haxe: WorldMap.getOriginalBiomeId fallback getBiomeId
pub fn eve_person_color_prefer_original(
    world: &World,
    original_biomes: &std::collections::HashMap<(i32, i32), u8>,
    x: i32,
    y: i32,
) -> i32 {
    eve_person_color_with_biome(
        |tx, ty| {
            original_biomes
                .get(&(tx, ty))
                .copied()
                .unwrap_or_else(|| world.get_biome(tx, ty))
        },
        x,
        y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_world::World;

    fn site(x: i32, y: i32, id: i32, uses: i32, biome: u8, pool: EveFoodPool) -> EveFoodSite {
        EveFoodSite {
            x,
            y,
            object_id: id,
            uses,
            biome,
            pool,
        }
    }

    #[test]
    fn clear_start_food_ids() {
        assert!(is_eve_start_food_id(30));
        assert!(is_eve_start_food_id(2142));
        assert!(is_eve_start_food_id(763));
        assert!(!is_eve_start_food_id(33));
    }

    #[test]
    fn classify_biome_gates() {
        assert_eq!(
            classify_eve_food_site(2142, JUNGLE_BIOME),
            Some(EveFoodPool::Banana)
        );
        assert_eq!(classify_eve_food_site(30, 0), Some(EveFoodPool::Berry));
        assert_eq!(
            classify_eve_food_site(36, BIOME_SNOW as u8),
            Some(EveFoodPool::WildCarrot)
        );
        assert_eq!(classify_eve_food_site(36, 0), None);
        assert_eq!(
            classify_eve_food_site(761, BIOME_DESERT as u8),
            Some(EveFoodPool::Cactus)
        );
        assert_eq!(classify_eve_food_site(761, 0), None);
        assert_eq!(
            classify_eve_food_site(4251, BIOME_GREY as u8),
            Some(EveFoodPool::WildGarlic)
        );
    }

    #[test]
    fn pool_select_haxe_rand_banana() {
        let counts = EveFoodPoolCounts {
            berry: 20,
            banana: 8,
            wild_carrot: 0,
            cactus: 0,
            wild_garlic: 0,
        };
        assert_eq!(
            select_eve_food_pool(&counts, 1, false),
            EveFoodPool::Banana
        );
        assert_eq!(select_eve_food_pool(&counts, 0, false), EveFoodPool::Berry);
    }

    #[test]
    fn pool_select_jungle_banana_todo_boost() {
        let counts = EveFoodPoolCounts {
            berry: 20,
            banana: 12,
            ..Default::default()
        };
        // prefer + banana>10 → rand 0 also banana
        assert_eq!(
            select_eve_food_pool(&counts, 0, true),
            EveFoodPool::Banana
        );
        // without prefer, rand 0 stays berry
        assert_eq!(
            select_eve_food_pool(&counts, 0, false),
            EveFoodPool::Berry
        );
    }

    #[test]
    fn pool_fallback_to_banana() {
        let counts = EveFoodPoolCounts {
            berry: 0,
            banana: 3,
            ..Default::default()
        };
        assert_eq!(
            select_eve_food_pool(&counts, 0, false),
            EveFoodPool::Banana
        );
    }

    #[test]
    fn fixed_spawn_when_sparse() {
        let sparse = EveFoodPoolCounts {
            berry: 3,
            banana: 2,
            ..Default::default()
        };
        assert!(use_fixed_starting_spawn(false, &sparse));
        let enough = EveFoodPoolCounts {
            berry: 6,
            banana: 5,
            ..Default::default()
        };
        assert!(!use_fixed_starting_spawn(false, &enough));
        assert!(use_fixed_starting_spawn(true, &enough));
    }

    #[test]
    fn fitness_prefers_far_from_humans_and_jungle_banana() {
        let jungle_banana = site(50, 50, 2142, 3, JUNGLE_BIOME, EveFoodPool::Banana);
        let plain_banana = site(50, 50, 2142, 3, 0, EveFoodPool::Banana);
        let near_human = site(1, 1, 30, 3, 0, EveFoodPool::Berry);
        let far = eve_location_fitness(&jungle_banana, &[(0, 0)], false, false);
        let plain = eve_location_fitness(&plain_banana, &[(0, 0)], false, false);
        let crowded = eve_location_fitness(&near_human, &[(0, 0)], false, false);
        assert!(far > plain, "jungle bonus far={far} plain={plain}");
        assert!(far > crowded, "far={far} crowded={crowded}");
    }

    #[test]
    fn pick_best_prefers_high_uses_when_alone() {
        let sites = vec![
            site(10, 10, 2142, 0, JUNGLE_BIOME, EveFoodPool::Banana),
            site(20, 20, 2142, 5, JUNGLE_BIOME, EveFoodPool::Banana),
        ];
        let mut i = 0usize;
        let best = pick_best_eve_site(
            &sites,
            &[],
            4,
            |_| {
                let v = i % 2;
                i += 1;
                v
            },
            |_, _| false,
            |_, _| false,
        )
        .unwrap();
        assert_eq!(best.uses, 5);
        assert_eq!((best.x, best.y), (20, 20));
    }

    #[test]
    fn resolve_none_when_sparse() {
        let sites = vec![site(1, 1, 30, 0, 0, EveFoodPool::Berry)];
        assert!(resolve_eve_spawn_site(
            &sites,
            &[],
            false,
            true,
            1,
            5,
            |_| 0,
            |_, _| false,
            |_, _| false,
        )
        .is_none());
    }

    #[test]
    fn resolve_banana_pool_when_abundant() {
        let mut sites = Vec::new();
        for i in 0..8 {
            sites.push(site(
                100 + i,
                100,
                2142,
                1,
                JUNGLE_BIOME,
                EveFoodPool::Banana,
            ));
        }
        for i in 0..8 {
            sites.push(site(i, 0, 30, 0, 0, EveFoodPool::Berry));
        }
        let best = resolve_eve_spawn_site(
            &sites,
            &[],
            false,
            true,
            1, // banana pool
            10,
            |_| 0,
            |_, _| false,
            |_, _| false,
        )
        .expect("site");
        assert_eq!(best.pool, EveFoodPool::Banana);
        assert_eq!(best.object_id, 2142);
    }

    #[test]
    fn person_color_special_biomes() {
        assert_eq!(person_color_by_biome(BIOME_JUNGLE), PERSON_BROWN);
        assert_eq!(person_color_by_biome(BIOME_DESERT), PERSON_BLACK);
        assert_eq!(person_color_by_biome(BIOME_SNOW), PERSON_GINGER);
        assert_eq!(person_color_by_biome(BIOME_GREY), PERSON_WHITE);
        assert_eq!(person_color_by_biome(0), 0);
    }

    #[test]
    fn close_special_biome_diagonal() {
        let color = get_close_special_biome_person_color(
            |x, y| {
                if x == 5 && y == 5 {
                    JUNGLE_BIOME
                } else {
                    0
                }
            },
            0,
            0,
            10,
        );
        assert_eq!(color, PERSON_BROWN);
    }

    #[test]
    fn collect_and_find_eve_near_bananas() {
        let mut w = World::new(64, 64, false);
        // Place 12 bananas on jungle + 12 berries so pool is eligible.
        for i in 0..12 {
            let x = 20 + i;
            w.set_biome(x, 20, JUNGLE_BIOME);
            w.set_object(x, 20, EVE_BANANA_PLANT);
            w.set_object(i, 5, EVE_BERRY_BUSH);
        }
        let sites = collect_eve_food_sites(&w, (20, 20), 40);
        let counts = EveFoodPoolCounts::from_sites(&sites);
        assert!(counts.banana >= 10, "banana={}", counts.banana);
        assert!(counts.berry >= 10, "berry={}", counts.berry);

        let (sx, sy) = find_eve_spawn_with_rng(
            &w,
            (20, 20),
            &[],
            (0, 0),
            EveSpawnOpts {
                prefer_jungle_banana: true,
                ..Default::default()
            },
            1, // force banana pool
            |_| 0,
        );
        assert_ne!((sx, sy), (0, 0), "should pick a food plant not fallback");
        assert_eq!(w.get_object(sx, sy), EVE_BANANA_PLANT);
        assert_eq!(eve_person_color_at(&w, sx, sy), PERSON_BROWN);
    }

    #[test]
    fn find_eve_falls_back_when_empty_world() {
        let w = World::new(32, 32, false);
        let (sx, sy) =
            find_eve_spawn_with_rng(&w, (5, 5), &[], (7, 8), EveSpawnOpts::default(), 1, |_| 0);
        assert_eq!((sx, sy), (7, 8));
    }

    #[test]
    fn last_eve_pair_founder_then_pairmate_clears() {
        let a = LastEveSlot {
            p_id: 10,
            x: 3,
            y: 4,
            person_color: PERSON_BROWN,
            is_female: true,
        };
        let r0 = resolve_eve_pair_partner(true, None, None, false);
        assert!(r0.partner.is_none());
        let (ai, human) = apply_eve_pair_slot_update(true, None, None, r0, a);
        assert_eq!(ai, Some(a));
        assert!(human.is_none());

        let r1 = resolve_eve_pair_partner(true, ai, human, false);
        assert_eq!(r1.partner, Some(a));
        let b = LastEveSlot {
            p_id: 11,
            x: 3,
            y: 4,
            person_color: PERSON_BROWN,
            is_female: false,
        };
        let (ai2, _) = apply_eve_pair_slot_update(true, ai, human, r1, b);
        assert!(ai2.is_none(), "pair clears last AI slot");
        // Pairmate sex is opposite of partner.
        assert!(!pairmate_eve_is_female(true), "partner female → pairmate male");
        assert!(pairmate_eve_is_female(false), "partner male → pairmate female");
        assert_eq!(eve_adam_first_name(true), "EVE");
        assert_eq!(eve_adam_first_name(false), "ADAM");
    }

    #[test]
    fn last_eve_cross_clears_both_pools() {
        let human = LastEveSlot {
            p_id: 1,
            x: 0,
            y: 0,
            person_color: PERSON_BLACK,
            is_female: false,
        };
        let r = resolve_eve_pair_partner(true, None, Some(human), true);
        assert_eq!(r.partner, Some(human));
        assert!(r.clear_both_pools);
        let self_slot = LastEveSlot {
            p_id: 2,
            x: 0,
            y: 0,
            person_color: PERSON_BLACK,
            is_female: true,
        };
        let (ai, hum) =
            apply_eve_pair_slot_update(true, None, Some(human), r, self_slot);
        assert!(ai.is_none());
        assert!(hum.is_none());
    }

    #[test]
    fn clear_deleted_last_eve_filters() {
        let s = LastEveSlot {
            p_id: 5,
            x: 1,
            y: 1,
            person_color: 0,
            is_female: true,
        };
        assert!(clear_deleted_last_eve(Some(s), |id| id == 5).is_some());
        assert!(clear_deleted_last_eve(Some(s), |_| false).is_none());
    }

    #[test]
    fn fitness_blocking_grave_lowers_score() {
        let site = site(0, 0, 30, 2, 0, EveFoodPool::Berry);
        let good = eve_location_fitness(&site, &[], false, false);
        let bad = eve_location_fitness(&site, &[], true, false);
        let boost = eve_location_fitness(&site, &[], false, true);
        assert!(bad < good, "blocking grave lowers fitness");
        assert!(boost > good, "nonblocking grave raises fitness");
    }

    #[test]
    fn grave_split_bone_vs_stone() {
        let graves = [(0, 0), (1, 0), (2, 0), (3, 0)];
        let (bone, stone) = split_account_graves_for_eve(&graves, |x, _| match x {
            0 => 87,   // bone
            1 => 418,  // non-bone grave-like
            2 => 1011, // buried — neither
            _ => 0,    // unknown → bone
        });
        assert!(bone.contains(&(0, 0)));
        assert!(bone.contains(&(3, 0)));
        assert!(stone.contains(&(1, 0)));
        assert!(!bone.contains(&(1, 0)));
        assert!(!stone.contains(&(2, 0)));
        assert!(!bone.contains(&(2, 0)));
    }

    #[test]
    fn pick_best_avoids_blocking_grave_tile() {
        let sites = vec![
            site(0, 0, 30, 3, 0, EveFoodPool::Berry),
            site(50, 50, 30, 3, 0, EveFoodPool::Berry),
        ];
        let mut step = 0usize;
        let best = pick_best_eve_site(
            &sites,
            &[],
            4,
            |_| {
                let i = step % 2;
                step += 1;
                i
            },
            |x, y| x == 0 && y == 0, // blocking at first site
            |_, _| false,
        )
        .unwrap();
        assert_eq!((best.x, best.y), (50, 50));
    }

    #[test]
    fn race_person_collect_and_pick() {
        let mut race = std::collections::HashMap::new();
        race.insert(100, PERSON_BROWN);
        race.insert(101, PERSON_BROWN);
        race.insert(102, PERSON_BROWN);
        race.insert(200, PERSON_BLACK);
        let name_desc = |id: i32| match id {
            100 => ("FemaleBrown".into(), "Female Brown".into()),
            101 => ("MaleBrown".into(), "Male Brown".into()),
            102 => ("Jason".into(), "Jason Brown".into()),
            200 => ("FemaleBlack".into(), "Female Black".into()),
            _ => (String::new(), String::new()),
        };
        let females = collect_person_ids_for_race(&race, name_desc, PERSON_BROWN, true);
        assert_eq!(females, vec![100]);
        let males = collect_person_ids_for_race(&race, name_desc, PERSON_BROWN, false);
        assert_eq!(males, vec![101]);
        assert_eq!(
            pick_eve_race_person_object(&race, name_desc, PERSON_BROWN, true, 0),
            Some(100)
        );
        assert_eq!(
            pick_eve_race_person_object(&race, name_desc, 0, true, 0),
            None
        );
    }

    #[test]
    fn original_biome_person_color() {
        let mut w = World::new(32, 32, false);
        w.set_biome(5, 5, 0); // live grass
        let mut orig = std::collections::HashMap::new();
        orig.insert((5, 5), JUNGLE_BIOME);
        assert_eq!(
            eve_person_color_prefer_original(&w, &orig, 5, 5),
            PERSON_BROWN
        );
        assert_eq!(eve_person_color_at(&w, 5, 5), 0);
    }

    #[test]
    fn founder_female_from_chance() {
        assert!(founder_eve_is_female(0.6));
        assert!(founder_eve_is_female(0.5));
        assert!(!founder_eve_is_female(0.4));
        assert!((STARTING_EVE_AGE - 14.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_at_last_dead_forces_fixed() {
        let mut sites = Vec::new();
        for i in 0..12 {
            sites.push(site(i, 0, 30, 1, 0, EveFoodPool::Berry));
            sites.push(site(i, 1, 2142, 1, JUNGLE_BIOME, EveFoodPool::Banana));
        }
        assert!(resolve_eve_spawn_site(
            &sites,
            &[],
            true, // SpwanAtLastDead
            true,
            1,
            5,
            |_| 0,
            |_, _| false,
            |_, _| false,
        )
        .is_none());
    }
}
