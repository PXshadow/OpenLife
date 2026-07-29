//! World-wide eaten-food statistics and FoodFactor (Haxe WorldMap food stats).
//!
//! Chunks:
//! - **WORLD-FOOD-FACTOR** / `food_factor` — live map + eat/search factors
//! - **EATEN-FOOD-PCT** / `world_food_map` — live `eatenFoodPercentage` on eat + horse path
//! - **FOODSTATS-DISK** / `foodstats_txt` — `FoodStats.txt` dump on autosave
//! - **FOODSTATS-WEB** / `food_stats_html` — ol-web `/stats/food` via [`format_food_statistics_html`]
//! - **LINEAGE-24H** / `starving_window` — `reasonKilledLastDay` 24h window for starving factor
//!
//! Haxe anchors:
//! - `WorldMap.addFoodStatistic` / `writeFoodStatistics` / `eatenFoodPercentage`
//! - `WorldMap.getFoodFactor` / `getEatenFoodPercentage` / `getStarvingFoodFactor`
//! - `WebServer.generateFoodStatistics` HTML Food/Eaten/Related table
//! - `Lineage.GenerateLineageStatistics` / `reasonKilledLastDay` (yearsSinceDeath < 1440)
//! - Eat path multiplies `ServerSettings.FoodFactor` × world FoodFactor × starving
//!   then records the final fill (`GlobalPlayerInstance` ~L3186–3215)
//! - Save path: `writeFoodStatistics(dir + "FoodStats" + tmpDataNumber + ".txt")`
//!
//! Pure helpers + [`WorldFoodStats`] session map. Live wire: `try_eat_held` +
//! FEED/NURSE + **horse mount-eat** (`try_horse_eat`) +
//! `search_best_food_full` cand `food_factor`. Disk: [`write_food_statistics`] via
//! outer autosave share ([`WorldFoodShare`]). Web: same share → ol-web `/stats/food`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::age_curves::SECONDS_PER_YEAR;
use crate::search_best_food::{food_factor_for_id, food_factor_for_id_ex, FoodFactorEatenBands};

// ---------------------------------------------------------------------------
// higherQaulityFood edges (Haxe ServerSettings patches)
// ---------------------------------------------------------------------------

/// Haxe `ObjectData.higherQaulityFood` patches used by getEatenFoodPercentage rollup.
///
/// Format: `(food_id, higher_quality_food_id)`.
// Haxe: ServerSettings higherQaulityFood L1321–1353
pub const HIGHER_QUALITY_FOOD_EDGES: &[(i32, i32)] = &[
    (31, 253),     // Gooseberry → Bowl of Gooseberries
    (253, 272),    // Bowl of Gooseberries → Cooked Berry Pie
    (4895, 1121),  // Popcorn → Bowl of Popcorn
    (40, 402),     // Wild Carrot → Carrot
    (402, 273),    // Carrot → Cooked Carrot Pie
    (808, 2855),   // Wild Onion → Onion
    (2855, 2860),  // Onion → Chopped Onion on Plate
    (2836, 2861),  // Tomato → Chopped Tomato on Plate
    (570, 803),    // Cooked Mutton → Cooked Mutton Pie
    // Milk chains (ServerSettings L1345–1353)
    (1463, 4081),  // Bowl of Whole Milk → Whole Milk Pouch
    (4081, 3593),  // Whole Milk Pouch → Whole Milk Bottle
    (1481, 4082),  // Bowl of Skim Milk → Skim Milk Pouch
    (4082, 3596),  // Skim Milk Pouch → Skim Milk Bottle
];

/// Default FoodStats dump filename under save directory.
///
/// Haxe: `FoodStats{tmpDataNumber}.txt` rotated with save slots.
/// Rust: single latest `FoodStats.txt` (same diagnostic text; no slot rotation).
// Haxe: WorldMap.write → writeFoodStatistics
pub const DEFAULT_FOOD_STATS_FILE: &str = "FoodStats.txt";

/// Haxe save-slot FoodStats filename (`FoodStats{tmpDataNumber}.txt`).
///
/// Rust autosave uses [`DEFAULT_FOOD_STATS_FILE`] (fixed latest). This helper
/// exists for parity tests / optional multi-slot dumps.
// Haxe: WorldMap.write L789 `FoodStats` + tmpDataNumber + `.txt`
pub fn haxe_food_stats_slot_filename(tmp_data_number: u32) -> String {
    format!("FoodStats{tmp_data_number}.txt")
}

/// Outer autosave / shutdown share of live [`WorldFoodStats`] (FOODSTATS-DISK).
// Haxe: WorldMap.eatenFood* maps written on save
pub type WorldFoodShare = Arc<RwLock<WorldFoodStats>>;

/// Look up higher-quality chain edges as a slice for pure rollups.
#[inline]
pub fn higher_quality_edges() -> &'static [(i32, i32)] {
    HIGHER_QUALITY_FOOD_EDGES
}

// ---------------------------------------------------------------------------
// LINEAGE-24H — last-day death reason window (starving_window)
// ---------------------------------------------------------------------------

/// Haxe `yearsSinceDeath < 1440` → last 24 real hours.
///
/// Haxe `CalculateTimeSinceTicksInYears` = real_seconds / 60 (`AgeingSecondsPerYear`).
/// 1440 game-years × 60 s/year = 86400 s = 24 h.
// Haxe: Lineage.GenerateLineageStatistics L353
pub const LAST_DAY_YEARS: f32 = 1440.0;

/// Haxe `yearsSinceDeath < 60` → last real hour (web table only; not starving factor).
// Haxe: Lineage.GenerateLineageStatistics L354
pub const LAST_HOUR_YEARS: f32 = 60.0;

/// Haxe throttle: regenerate lineage stats at most once per 60 real seconds.
// Haxe: Lineage.GenerateLineageStatistics L333–334
pub const LINEAGE_STATS_THROTTLE_SECS: f32 = 60.0;

/// Real seconds in the last-day window (24 h).
// Haxe: 1440 years × SECONDS_PER_YEAR
pub const LAST_DAY_SECS: f32 = LAST_DAY_YEARS * SECONDS_PER_YEAR;

/// Real seconds in the last-hour window.
pub const LAST_HOUR_SECS: f32 = LAST_HOUR_YEARS * SECONDS_PER_YEAR;

/// One session death stamp for lineage stats / starving window.
///
/// Haxe rebuilds maps by scanning all lineages; Rust stamps deaths at wire time
/// (session ring). Same window math: death_sim_time + years_since < 1440.
// Haxe: Lineage deathTime + deathReason in GenerateLineageStatistics
#[derive(Debug, Clone, PartialEq)]
pub struct LineageDeathStamp {
    /// Sim seconds when the player died (`SimState.sim_time`).
    pub death_sim_time: f32,
    /// Stats key after kid remap (`reason_age`, `reason_hunger`, `reason_hunger_kid`, …).
    pub reason_key: String,
    /// Age years at death (for diagnostics; kid remap applied at stamp time).
    pub age_years: f32,
}

/// Haxe `CalculateTimeSinceTicksInYears` from real elapsed seconds.
// Haxe: TimeHelper.CalculateTimeSinceTicksInYears
#[inline]
pub fn years_since_from_secs(elapsed_secs: f32) -> f32 {
    if !elapsed_secs.is_finite() || elapsed_secs <= 0.0 {
        0.0
    } else {
        elapsed_secs / SECONDS_PER_YEAR
    }
}

/// True when death is inside the last-day window (Haxe `yearsSinceDeath < 1440`).
// Haxe: Lineage L353 isLastDay
#[inline]
pub fn is_last_day_years(years_since_death: f32) -> bool {
    years_since_death.is_finite() && years_since_death < LAST_DAY_YEARS
}

/// True when death is inside the last-hour window (Haxe `yearsSinceDeath < 60`).
// Haxe: Lineage L354 isLastHour
#[inline]
pub fn is_last_hour_years(years_since_death: f32) -> bool {
    years_since_death.is_finite() && years_since_death < LAST_HOUR_YEARS
}

/// True when `now_sim - death_sim` is within 24 real hours.
///
/// Session [`LineageDeathStamp`]s always represent real deaths (including
/// `death_sim_time == 0` at server start). For Haxe lineage *fields* where
/// `deathTime == 0` means never died, use [`is_lineage_death_time_in_last_day`].
// Haxe: Lineage L353 with CalculateTimeSinceTicksInYears
#[inline]
pub fn is_death_in_last_day(death_sim_time: f32, now_sim: f32) -> bool {
    if !death_sim_time.is_finite() || !now_sim.is_finite() {
        return false;
    }
    let elapsed = (now_sim - death_sim_time).max(0.0);
    is_last_day_years(years_since_from_secs(elapsed))
}

/// Haxe lineage field gate: `deathTime > 0 && yearsSinceDeath < 1440`.
// Haxe: Lineage L353 isLastDay (`lineage.deathTime > 0`)
#[inline]
pub fn is_lineage_death_time_in_last_day(death_time: f32, now_sim: f32) -> bool {
    death_time > 0.0 && is_death_in_last_day(death_time, now_sim)
}

/// Parse `reason_killed_<id>` object id (Haxe `Std.parseInt` after strip).
///
/// Returns `None` for non-kill wires or unparseable ids.
// Haxe: Lineage.GenerateLineageStatistics L356–359
pub fn parse_reason_killed_object_id(reason: &str) -> Option<i32> {
    let r = reason.trim();
    let rest = r.strip_prefix("reason_killed_")?;
    if rest.is_empty() {
        return None;
    }
    // Haxe parseInt stops at non-digits; accept leading digits only.
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i32>().ok().filter(|&id| id > 0)
}

/// Haxe stats key for a death reason + age (content-free).
///
/// - `age < 5 && reason_hunger` → `reason_hunger_kid` (excluded from starving factor)
/// - `reason_killed_<id>` stays as raw wire (use [`normalize_death_reason_for_stats_ex`] for name)
// Haxe: Lineage.GenerateLineageStatistics L356–378
pub fn normalize_death_reason_for_stats(reason: &str, age_years: f32) -> String {
    normalize_death_reason_for_stats_ex(reason, age_years, None)
}

/// Haxe stats key with optional `reason_killed_<id>` → object name remap.
///
/// When `object_name` is `Some(name)` and reason is `reason_killed_*`, key becomes
/// that name (Haxe `ObjectData.getObjectData(id).name`). Kid hunger remap runs after.
// Haxe: Lineage.GenerateLineageStatistics L356–378
pub fn normalize_death_reason_for_stats_ex(
    reason: &str,
    age_years: f32,
    object_name: Option<&str>,
) -> String {
    let r = reason.trim();
    if r.is_empty() {
        return String::new();
    }
    // Haxe: if deathReason.startsWith('reason_killed_') → ObjectData.name
    let mut killed_by = if r.starts_with("reason_killed_") {
        match object_name.map(str::trim).filter(|n| !n.is_empty()) {
            Some(name) => name.to_string(),
            None => r.to_string(),
        }
    } else {
        r.to_string()
    };
    // Haxe: if (age < 5 && killedBy == 'reason_hunger') killedBy = 'reason_hunger_kid';
    if killed_by == "reason_hunger" && age_years.is_finite() && age_years < 5.0 {
        killed_by = "reason_hunger_kid".into();
    }
    killed_by
}

/// Resolve kill-name then normalize (content-aware path).
///
/// `name_of(id)` returns object display name; empty/`None` keeps raw `reason_killed_<id>`.
// Haxe: Lineage.GenerateLineageStatistics L356–361 + L378
pub fn normalize_death_reason_for_stats_with_resolver<F>(
    reason: &str,
    age_years: f32,
    mut name_of: F,
) -> String
where
    F: FnMut(i32) -> Option<String>,
{
    let obj_name = parse_reason_killed_object_id(reason).and_then(|id| name_of(id));
    normalize_death_reason_for_stats_ex(reason, age_years, obj_name.as_deref())
}

/// One lineage row for [`generate_lineage_statistics`] / boot seed (content-free).
///
/// Mirrors Haxe `AllLineages` scan fields used by `GenerateLineageStatistics`.
// Haxe: Lineage.GenerateLineageStatistics L347–386
#[derive(Debug, Clone, PartialEq)]
pub struct LineageStatRow {
    /// Haxe `deathTime` (sim seconds); `0` = never died (excluded from last-day/hour).
    pub death_sim_time: f32,
    /// Raw wire reason (`reason_hunger`, `reason_killed_33`, …); empty if none.
    pub death_reason: String,
    /// Age years at death (or −1 for never-died / unknown — Haxe clamps `< -1` → −1).
    pub age_years: f32,
    /// Haxe `lineage.generation`.
    pub generation: i32,
}

impl LineageStatRow {
    /// Build a row from lineage death session fields.
    pub fn from_death_fields(
        death_sim_time: f32,
        death_reason: impl Into<String>,
        age_years: f32,
        generation: i32,
    ) -> Self {
        Self {
            death_sim_time: if death_sim_time.is_finite() {
                death_sim_time.max(0.0)
            } else {
                0.0
            },
            death_reason: death_reason.into(),
            age_years,
            generation,
        }
    }

    /// True when this lineage has a recorded death (Haxe `deathTime > 0` or non-empty reason).
    #[inline]
    pub fn has_death(&self) -> bool {
        self.death_sim_time > 0.0 || !self.death_reason.trim().is_empty()
    }
}

/// Full Haxe `GenerateLineageStatistics` maps (web / diagnostics).
// Haxe: Lineage.reasonKilled* / ages* / generations
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineageStatistics {
    /// All-time reason histogram (after kill-name + kid remap).
    pub reason_killed: HashMap<String, i32>,
    /// Last-day reason histogram (`yearsSinceDeath < 1440`).
    pub reason_killed_last_day: HashMap<String, i32>,
    /// Last-hour reason histogram (`yearsSinceDeath < 60`).
    pub reason_killed_last_hour: HashMap<String, i32>,
    /// All-time age histogram (integer years; living/unknown → −1).
    pub ages: HashMap<i32, i32>,
    pub ages_last_day: HashMap<i32, i32>,
    pub ages_last_hour: HashMap<i32, i32>,
    /// Generation histogram.
    pub generations: HashMap<i32, i32>,
    /// Count of lineages with death in last day (`countNew` in Haxe).
    pub count_last_day: i32,
    /// Count of lineages not in last-day death window (`countOld`).
    pub count_old: i32,
}

impl LineageStatistics {
    /// Last-day exact `reason_age` / `reason_hunger` counts for starving factor.
    pub fn last_day_age_hunger(&self) -> (i32, i32) {
        let age = self
            .reason_killed_last_day
            .get("reason_age")
            .copied()
            .unwrap_or(0);
        let hunger = self
            .reason_killed_last_day
            .get("reason_hunger")
            .copied()
            .unwrap_or(0);
        (age, hunger)
    }
}

/// Round age for histograms (Haxe `Math.round`; clamp `< -1` → −1).
// Haxe: Lineage.GenerateLineageStatistics L350, L374
#[inline]
pub fn lineage_stats_age_key(age_years: f32) -> i32 {
    if !age_years.is_finite() {
        return -1;
    }
    let rounded = age_years.round() as i32;
    if rounded < -1 {
        -1
    } else {
        rounded
    }
}

/// Haxe `GenerateLineageStatistics` pure scan (optional kill-name resolver).
///
/// `name_of(object_id)` supplies `ObjectData.name` for `reason_killed_<id>` keys.
/// Pass `|_| None` for content-free keys (raw wire tags).
// Haxe: Lineage.GenerateLineageStatistics L332–402
pub fn generate_lineage_statistics<I, F>(rows: I, now_sim: f32, mut name_of: F) -> LineageStatistics
where
    I: IntoIterator<Item = LineageStatRow>,
    F: FnMut(i32) -> Option<String>,
{
    let now = if now_sim.is_finite() { now_sim } else { 0.0 };
    let mut stats = LineageStatistics::default();
    for row in rows {
        let years_since = if row.death_sim_time > 0.0 {
            years_since_from_secs((now - row.death_sim_time).max(0.0))
        } else {
            // Never died: not last-day/hour; still counted in all-time ages/gens.
            f32::INFINITY
        };
        let is_last_day = row.death_sim_time > 0.0 && is_last_day_years(years_since);
        let is_last_hour = row.death_sim_time > 0.0 && is_last_hour_years(years_since);

        if is_last_day {
            stats.count_last_day = stats.count_last_day.saturating_add(1);
        } else {
            stats.count_old = stats.count_old.saturating_add(1);
        }

        // Haxe age for never-died often lands at −1 after clamp.
        let age_key = if row.has_death() {
            lineage_stats_age_key(row.age_years)
        } else {
            -1
        };
        *stats.ages.entry(age_key).or_insert(0) += 1;
        if is_last_day {
            *stats.ages_last_day.entry(age_key).or_insert(0) += 1;
        }
        if is_last_hour {
            *stats.ages_last_hour.entry(age_key).or_insert(0) += 1;
        }

        let killed_by = normalize_death_reason_for_stats_with_resolver(
            &row.death_reason,
            row.age_years,
            &mut name_of,
        );
        if !killed_by.is_empty() {
            *stats.reason_killed.entry(killed_by.clone()).or_insert(0) += 1;
            if is_last_day {
                *stats
                    .reason_killed_last_day
                    .entry(killed_by.clone())
                    .or_insert(0) += 1;
            }
            if is_last_hour {
                *stats.reason_killed_last_hour.entry(killed_by).or_insert(0) += 1;
            }
        }

        *stats.generations.entry(row.generation).or_insert(0) += 1;
    }
    stats
}

/// Content-free [`generate_lineage_statistics`] (raw `reason_killed_<id>` keys).
// Haxe: Lineage.GenerateLineageStatistics without ObjectData resolve
pub fn generate_lineage_statistics_raw(rows: impl IntoIterator<Item = LineageStatRow>, now_sim: f32) -> LineageStatistics {
    generate_lineage_statistics(rows, now_sim, |_| None)
}

/// Build session death stamps from lineage rows that have a death record.
///
/// Used to rehydrate [`WorldFoodStats::death_stamps`] after load / boot
/// (Haxe rescans `AllLineages` each `GenerateLineageStatistics`).
// Haxe: Lineage AllLineages deathTime/deathReason scan
pub fn death_stamps_from_lineage_rows(
    rows: impl IntoIterator<Item = LineageStatRow>,
) -> Vec<LineageDeathStamp> {
    let mut out = Vec::new();
    for row in rows {
        if !row.has_death() {
            continue;
        }
        let key = normalize_death_reason_for_stats(&row.death_reason, row.age_years);
        if key.is_empty() {
            continue;
        }
        // Prefer recorded death_sim_time; if only reason set (edge), stamp at 0.
        let t = if row.death_sim_time.is_finite() && row.death_sim_time > 0.0 {
            row.death_sim_time
        } else if row.death_sim_time.is_finite() {
            row.death_sim_time.max(0.0)
        } else {
            0.0
        };
        out.push(LineageDeathStamp {
            death_sim_time: t,
            reason_key: key,
            age_years: if row.age_years.is_finite() {
                row.age_years.max(0.0)
            } else {
                0.0
            },
        });
    }
    out
}

/// Count last-day `reason_age` / `reason_hunger` from stamps (starving factor inputs).
// Haxe: Lineage.reasonKilledLastDay['reason_age'|'reason_hunger']
pub fn count_last_day_age_hunger(
    stamps: &[LineageDeathStamp],
    now_sim: f32,
) -> (i32, i32) {
    let mut age = 0_i32;
    let mut hunger = 0_i32;
    for s in stamps {
        if !is_death_in_last_day(s.death_sim_time, now_sim) {
            continue;
        }
        if s.reason_key == "reason_age" {
            age = age.saturating_add(1);
        } else if s.reason_key == "reason_hunger" {
            // reason_hunger_kid already remapped at stamp — excluded here
            hunger = hunger.saturating_add(1);
        }
    }
    (age, hunger)
}

/// Build full last-day reason histogram (web / diagnostics).
// Haxe: Lineage.reasonKilledLastDay
pub fn count_reason_killed_last_day(
    stamps: &[LineageDeathStamp],
    now_sim: f32,
) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for s in stamps {
        if s.reason_key.is_empty() {
            continue;
        }
        if !is_death_in_last_day(s.death_sim_time, now_sim) {
            continue;
        }
        *map.entry(s.reason_key.clone()).or_insert(0) += 1;
    }
    map
}

/// Build last-hour reason histogram (web table column).
// Haxe: Lineage.reasonKilledLastHour
pub fn count_reason_killed_last_hour(
    stamps: &[LineageDeathStamp],
    now_sim: f32,
) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for s in stamps {
        if s.reason_key.is_empty() {
            continue;
        }
        if !s.death_sim_time.is_finite() || !now_sim.is_finite() {
            continue;
        }
        let years = years_since_from_secs((now_sim - s.death_sim_time).max(0.0));
        if !is_last_hour_years(years) {
            continue;
        }
        *map.entry(s.reason_key.clone()).or_insert(0) += 1;
    }
    map
}

/// Haxe `WorldMap.getStarvingFoodFactor` from last-day death reason counts.
///
/// ```text
/// killedAge    = 10 + reason_age
/// killedHunger = 4 * (10 + reason_hunger)
/// factor = (2*killedHunger + killedAge) / (killedHunger + 2*killedAge)
/// ```
///
/// Empty stats → 1.5. Clamped implicitly by formula (~0.5–2 for normal ratios).
// Haxe: WorldMap.getStarvingFoodFactor
pub fn starving_food_factor_from_deaths(reason_age_last_day: i32, reason_hunger_last_day: i32) -> f32 {
    let age = reason_age_last_day.max(0) as f32;
    let hunger = reason_hunger_last_day.max(0) as f32;
    let killed_age = 10.0 + age;
    let killed_hunger = 4.0 * (10.0 + hunger);
    let num = killed_hunger + killed_hunger + killed_age;
    let den = killed_hunger + killed_age + killed_age;
    if den <= 0.0 {
        1.0
    } else {
        num / den
    }
}

/// Human-readable death-reason label for web tables (Haxe `generateLineageStatistics`).
// Haxe: WebServer.generateLineageStatistics L367–372
pub fn lineage_death_reason_display_label(reason: &str) -> String {
    match reason.trim() {
        "null" => "N/A".into(),
        "reason_age" => "OLD AGE".into(),
        "reason_hunger" => "STARVATION".into(),
        "reason_hunger_kid" => "STARVATION KID".into(),
        other => other.to_string(),
    }
}

/// Haxe `WebServer.generateLineageStatistics` death-reason HTML table body (pure).
///
/// Columns: Reason | Total | Last Day | Last Hour. Sorted by reason key.
/// Appends "Extra food because of Starving: N%" from last-day age/hunger counts.
// Haxe: WebServer.generateLineageStatistics L364–383
pub fn format_lineage_death_reason_html(stats: &LineageStatistics) -> String {
    let mut keys: Vec<&String> = stats.reason_killed.keys().collect();
    keys.sort();
    let mut out = String::from(
        "<br><br>\n<center><table>\n<tr><td><b>Reason killed</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n",
    );
    for reason in keys {
        if reason.is_empty() {
            continue;
        }
        let reason_text = lineage_death_reason_display_label(reason);
        let total = stats.reason_killed.get(reason).copied().unwrap_or(0);
        let day = stats
            .reason_killed_last_day
            .get(reason)
            .copied()
            .unwrap_or(0);
        let hour = stats
            .reason_killed_last_hour
            .get(reason)
            .copied()
            .unwrap_or(0);
        out.push_str(&format!(
            "<tr><td>{reason_text}</td><td>{total}</td><td>{day}</td><td>{hour}</td></tr>\n"
        ));
    }
    let (age, hunger) = stats.last_day_age_hunger();
    let factor = starving_food_factor_from_deaths(age, hunger);
    // Haxe: Math.ceil(factor * 100) - 100
    let food_factor_percent = (factor * 100.0).ceil() as i32 - 100;
    out.push_str(&format!(
        "</table>Extra food because of Starving: {food_factor_percent}%</center>\n"
    ));
    out
}

/// Haxe `WebServer.generateLineageStatistics` ages HTML table (pure).
///
/// Columns: Age | Total | Last Day | Last Hour. Keys sorted ascending; `< 0` → `N/A`.
// Haxe: WebServer.generateLineageStatistics L384–399
pub fn format_lineage_ages_html(stats: &LineageStatistics) -> String {
    let mut keys: Vec<i32> = stats.ages.keys().copied().collect();
    keys.sort_unstable();
    let mut out = String::from(
        "<br><br>\n<center><table>\n<tr><td><b>Age</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n",
    );
    for age in keys {
        let age_text = if age < 0 {
            "N/A".to_string()
        } else {
            age.to_string()
        };
        let total = stats.ages.get(&age).copied().unwrap_or(0);
        let day = stats.ages_last_day.get(&age).copied().unwrap_or(0);
        let hour = stats.ages_last_hour.get(&age).copied().unwrap_or(0);
        out.push_str(&format!(
            "<tr><td>{age_text}</td><td>{total}</td><td>{day}</td><td>{hour}</td></tr>\n"
        ));
    }
    out.push_str("</table></center>\n");
    out
}

/// Full Haxe `generateLineageStatistics` HTML fragment (reason table + starving % + ages).
// Haxe: WebServer.generateLineageStatistics L364–399
pub fn format_lineage_statistics_html(stats: &LineageStatistics) -> String {
    let mut out = format_lineage_death_reason_html(stats);
    out.push_str(&format_lineage_ages_html(stats));
    out
}

// ---------------------------------------------------------------------------
// Apply factors to eat fill
// ---------------------------------------------------------------------------

/// Haxe eat path: `foodValue *= FoodFactor; *= getFoodFactor; *= getStarvingFoodFactor`.
///
/// `base_fill` is already post-yum and post-global `ServerSettings.FoodFactor`
/// (Rust [`crate::yum::compute_eat`] multiplies `FOOD_FACTOR`).
// Haxe: GlobalPlayerInstance eat L3186–3192
#[inline]
pub fn apply_world_food_factors(base_fill: f32, food_factor: f32, starving_food_factor: f32) -> f32 {
    (base_fill * food_factor * starving_food_factor).max(0.0)
}

/// Super-meh post-factor trade: +1 food (Haxe L3195–3206 side-effects separate).
// Haxe: GlobalPlayerInstance eat isSuperMeh L3195
#[inline]
pub fn super_meh_extra_food_value(is_super_meh: bool) -> f32 {
    if is_super_meh {
        1.0
    } else {
        0.0
    }
}

/// Pure superMeh health-for-food trade after fill factors (Haxe L3195–3206).
///
/// Always: `age += 0.2`, `foodValue += 1` (fill is via [`super_meh_extra_food_value`]).
/// If `prestige > 0`: `prestige -= 1`.
/// Else: `hits += 1`, `age += 0.8`, set `woundedBy = food_parent_id`, recompute
/// food_store_max; death when food_store_max &lt; 1.
// Haxe: GlobalPlayerInstance doEating isSuperMeh L3195–3206
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuperMehTrade {
    pub age_delta: f32,
    pub prestige_delta: f32,
    pub hits_delta: f32,
    /// Food parent id for `woundedBy` when hits path; 0 otherwise.
    pub wounded_by_food_id: i32,
    /// Caller must recompute food_max after hits; kill if [`super_meh_food_max_is_deadly`].
    pub needs_food_max_recompute: bool,
}

/// Compute superMeh age/prestige/hits deltas (no fill — use [`super_meh_extra_food_value`]).
// Haxe: GlobalPlayerInstance doEating L3195–3206
#[inline]
pub fn super_meh_trade(is_super_meh: bool, prestige: f32, food_parent_id: i32) -> SuperMehTrade {
    if !is_super_meh {
        return SuperMehTrade {
            age_delta: 0.0,
            prestige_delta: 0.0,
            hits_delta: 0.0,
            wounded_by_food_id: 0,
            needs_food_max_recompute: false,
        };
    }
    if prestige > 0.0 {
        SuperMehTrade {
            age_delta: 0.2,
            prestige_delta: -1.0,
            hits_delta: 0.0,
            wounded_by_food_id: 0,
            needs_food_max_recompute: false,
        }
    } else {
        SuperMehTrade {
            // age += 0.2 then age += 0.8 on hits path
            age_delta: 1.0,
            prestige_delta: 0.0,
            hits_delta: 1.0,
            wounded_by_food_id: food_parent_id,
            needs_food_max_recompute: true,
        }
    }
}

/// Haxe superMeh death gate after hits: `food_store_max < 1`.
// Haxe: GlobalPlayerInstance doEating L3205
#[inline]
pub fn super_meh_food_max_is_deadly(food_store_max: f32) -> bool {
    food_store_max < 1.0
}

// ---------------------------------------------------------------------------
// WorldFoodStats — live session map
// ---------------------------------------------------------------------------

/// Per-food accumulated eat statistics (Haxe WorldMap.eatenFood* maps).
#[derive(Debug, Clone, Default)]
pub struct WorldFoodStats {
    /// Sum of final fill values eaten for each food parent id.
    pub eaten_values: HashMap<i32, f32>,
    /// Fill counted as yum (final > base food_value).
    pub eaten_yum: HashMap<i32, f32>,
    /// Fill counted as meh (final ≤ base).
    pub eaten_meh: HashMap<i32, f32>,
    /// Yum bonus sum (final − base when > 0).
    pub yum_boni: HashMap<i32, f32>,
    /// Meh mali sum (−(final − base) when ≤ 0).
    pub meh_mali: HashMap<i32, f32>,
    /// Percentage of total eaten (0–100). Recomputed on add for live map.
    pub eaten_percentage: HashMap<i32, f32>,
    /// Session death stamps (LINEAGE-24H). Windowed for starving factor.
    // Haxe: Lineage AllLineages deathTime/deathReason scan
    pub death_stamps: Vec<LineageDeathStamp>,
    /// Cached last-day `reason_age` count (after [`Self::refresh_starving_window`]).
    ///
    /// Haxe `Lineage.reasonKilledLastDay['reason_age']`.
    pub reason_age_deaths: i32,
    /// Cached last-day exact `reason_hunger` count (kid/nursing excluded).
    ///
    /// Haxe `getStarvingFoodFactor` uses `Lineage.reasonKilledLastDay['reason_hunger']`
    /// after remapping age&lt;5 hunger → `reason_hunger_kid` (excluded). Nursing hunger
    /// is a distinct wire and is also excluded.
    pub reason_hunger_deaths: i32,
    /// Full last-day reason histogram (Haxe `reasonKilledLastDay`).
    pub reason_killed_last_day: HashMap<String, i32>,
    /// Last-hour reason histogram (Haxe `reasonKilledLastHour`; web column).
    pub reason_killed_last_hour: HashMap<String, i32>,
    /// Sim time of last window rebuild (`-1` = never). Haxe `lastStatisticGenerated`.
    pub last_statistic_sim: f32,
    /// Last `now_sim` used for window (for zero-arg get after refresh).
    pub window_now_sim: f32,
}

impl WorldFoodStats {
    pub fn new() -> Self {
        Self {
            last_statistic_sim: -1.0,
            ..Self::default()
        }
    }

    /// Haxe `WorldMap.addFoodStatistic` + immediate percentage recompute (live map).
    ///
    /// `food_id` = parent id; `base_food_value` = content `foodValue`;
    /// `final_food_value` = fill after world factors (what was added to store).
    // Haxe: WorldMap.addFoodStatistic
    pub fn add_food_statistic(&mut self, food_id: i32, base_food_value: f32, final_food_value: f32) {
        if food_id <= 0 {
            return;
        }
        let yum = final_food_value - base_food_value;
        *self.eaten_values.entry(food_id).or_insert(0.0) += final_food_value;
        if yum > 0.0 {
            *self.eaten_yum.entry(food_id).or_insert(0.0) += final_food_value;
            *self.yum_boni.entry(food_id).or_insert(0.0) += yum;
        } else {
            *self.eaten_meh.entry(food_id).or_insert(0.0) += final_food_value;
            *self.meh_mali.entry(food_id).or_insert(0.0) -= yum;
        }
        // Live map (EATEN-FOOD-PCT intentional delta): Haxe only recomputes in
        // writeFoodStatistics (on save). Rust recomputes on each add so getFoodFactor
        // and SearchBestFood see session percentages without waiting for autosave.
        self.recompute_percentages();
    }

    /// Haxe `writeFoodStatistics` percentage pass:
    /// `round(value / total * 100)`.
    // Haxe: WorldMap.writeFoodStatistics
    pub fn recompute_percentages(&mut self) {
        let total: f32 = self.eaten_values.values().sum();
        self.eaten_percentage.clear();
        if total <= 0.0 {
            return;
        }
        for (&food_id, &val) in &self.eaten_values {
            // Haxe: Math.round(eatenFoodValues[foodId] / total * 100) / 1
            let pct = ((val / total) * 100.0).round();
            self.eaten_percentage.insert(food_id, pct);
        }
    }

    /// Sparse `(food_id, percentage)` for pure rollups.
    pub fn percentage_pairs(&self) -> Vec<(i32, f32)> {
        self.eaten_percentage
            .iter()
            .map(|(&id, &p)| (id, p))
            .collect()
    }

    /// Haxe `getEatenFoodPercentage` with higher-quality rollup.
    // Haxe: WorldMap.getEatenFoodPercentage
    pub fn get_eaten_food_percentage(&self, food_id: i32) -> f32 {
        let pairs = self.percentage_pairs();
        eaten_percentage_with_hq(food_id, &pairs, higher_quality_edges(), 0)
    }

    /// Haxe `WorldMap.getFoodFactor(foodId)` at default ServerSettings bands.
    // Haxe: WorldMap.getFoodFactor
    pub fn get_food_factor(&self, food_id: i32) -> f32 {
        self.get_food_factor_ex(food_id, &FoodFactorEatenBands::default())
    }

    /// Haxe `WorldMap.getFoodFactor` with live `ServerSettings.FoodFactorEaten*` bands.
    // Haxe: WorldMap.getFoodFactor + ServerSettings.FoodFactorEaten*
    // C-SS-FULL-TABLE
    pub fn get_food_factor_ex(&self, food_id: i32, bands: &FoodFactorEatenBands) -> f32 {
        let pairs = self.percentage_pairs();
        food_factor_for_id_ex(food_id, &pairs, higher_quality_edges(), bands)
    }

    /// Rebuild last-day counters from stamps (Haxe `GenerateLineageStatistics` core).
    ///
    /// Throttled to once per [`LINEAGE_STATS_THROTTLE_SECS`] unless `force`.
    /// Returns true when the window was rebuilt.
    // Haxe: Lineage.GenerateLineageStatistics
    pub fn refresh_starving_window(&mut self, now_sim: f32, force: bool) -> bool {
        let now = if now_sim.is_finite() { now_sim } else { 0.0 };
        if !force && self.last_statistic_sim >= 0.0 {
            let since = now - self.last_statistic_sim;
            if since.is_finite() && since < LINEAGE_STATS_THROTTLE_SECS {
                return false;
            }
        }
        let (age, hunger) = count_last_day_age_hunger(&self.death_stamps, now);
        self.reason_age_deaths = age;
        self.reason_hunger_deaths = hunger;
        self.reason_killed_last_day = count_reason_killed_last_day(&self.death_stamps, now);
        self.reason_killed_last_hour = count_reason_killed_last_hour(&self.death_stamps, now);
        self.last_statistic_sim = now;
        self.window_now_sim = now;
        true
    }

    /// Replace death stamps from lineage death fields and rebuild the window.
    ///
    /// Haxe keeps deathTime/deathReason on each lineage and rescans on stats;
    /// Rust rehydrates the session stamp ring after OLN2 load / process boot.
    /// Returns number of stamps seeded.
    // Haxe: GenerateLineageStatistics scans AllLineages
    pub fn seed_death_stamps_from_lineage_rows(
        &mut self,
        rows: impl IntoIterator<Item = LineageStatRow>,
        now_sim: f32,
    ) -> usize {
        self.death_stamps = death_stamps_from_lineage_rows(rows);
        let n = self.death_stamps.len();
        self.refresh_starving_window(now_sim, true);
        n
    }

    /// Append a pre-normalized stamp without kid re-map (used by tests / bulk seed).
    pub fn push_death_stamp(&mut self, stamp: LineageDeathStamp) {
        if stamp.reason_key.is_empty() {
            return;
        }
        self.death_stamps.push(stamp);
    }

    /// Haxe `WorldMap.getStarvingFoodFactor` using cached last-day counts.
    ///
    /// Prefer [`Self::get_starving_food_factor_at`] when sim time is available so the
    /// 24h window stays accurate. This uses the last refresh (or live count at
    /// `window_now_sim` / stamp times when never refreshed).
    // Haxe: WorldMap.getStarvingFoodFactor
    pub fn get_starving_food_factor(&self) -> f32 {
        // If stamps exist but window never refreshed, count at max(death, window_now).
        if self.last_statistic_sim < 0.0 && !self.death_stamps.is_empty() {
            let now = self
                .death_stamps
                .iter()
                .map(|s| s.death_sim_time)
                .fold(self.window_now_sim, f32::max);
            let (age, hunger) = count_last_day_age_hunger(&self.death_stamps, now);
            return starving_food_factor_from_deaths(age, hunger);
        }
        starving_food_factor_from_deaths(self.reason_age_deaths, self.reason_hunger_deaths)
    }

    /// Haxe `getStarvingFoodFactor` with live 24h window at `now_sim`.
    ///
    /// Does **not** mutate throttle state — pure count from stamps.
    // Haxe: WorldMap.getStarvingFoodFactor + GenerateLineageStatistics
    pub fn get_starving_food_factor_at(&self, now_sim: f32) -> f32 {
        let (age, hunger) = count_last_day_age_hunger(&self.death_stamps, now_sim);
        starving_food_factor_from_deaths(age, hunger)
    }

    /// Ensure window is fresh (throttle) then return starving factor.
    // Haxe: getStarvingFoodFactor → GenerateLineageStatistics
    pub fn get_starving_food_factor_refreshed(&mut self, now_sim: f32) -> f32 {
        self.refresh_starving_window(now_sim, false);
        starving_food_factor_from_deaths(self.reason_age_deaths, self.reason_hunger_deaths)
    }

    /// Note a death for the last-day starving window (LINEAGE-24H).
    ///
    /// `death_sim_time` = `SimState.sim_time` at death; `age_years` drives kid remap.
    /// Immediately rebuilds last-day cache so eat-soon-after sees the death.
    // Haxe: Lineage deathTime/deathReason + GenerateLineageStatistics L378–383
    pub fn note_death_reason_at(
        &mut self,
        death_sim_time: f32,
        reason: &str,
        age_years: f32,
    ) {
        let key = normalize_death_reason_for_stats(reason, age_years);
        if key.is_empty() {
            return;
        }
        let t = if death_sim_time.is_finite() {
            death_sim_time.max(0.0)
        } else {
            0.0
        };
        self.death_stamps.push(LineageDeathStamp {
            death_sim_time: t,
            reason_key: key,
            age_years,
        });
        // Force rebuild so a death immediately affects fill factors (Haxe next get
        // regenerates within 60s throttle; we force at stamp for snappier parity).
        self.refresh_starving_window(t, true);
    }

    /// Convenience: note death at current window time / adult age (tests + legacy).
    ///
    /// Prefer [`Self::note_death_reason_at`] on the live death path.
    // Haxe: WorldMap.getStarvingFoodFactor + Lineage L378 reason_hunger_kid remap
    pub fn note_death_reason(&mut self, reason: &str) {
        // Adult age so reason_hunger is not remapped to kid.
        let t = self.window_now_sim.max(0.0);
        self.note_death_reason_at(t, reason, 30.0);
    }

    /// Last-day count for an arbitrary reason key (web / diagnostics).
    // Haxe: Lineage.reasonKilledLastDay[reason]
    pub fn reason_killed_last_day_count(&self, reason: &str) -> i32 {
        self.reason_killed_last_day
            .get(reason)
            .copied()
            .unwrap_or(0)
    }

    /// Last-hour count for an arbitrary reason key (web / diagnostics).
    // Haxe: Lineage.reasonKilledLastHour[reason]
    pub fn reason_killed_last_hour_count(&self, reason: &str) -> i32 {
        self.reason_killed_last_hour
            .get(reason)
            .copied()
            .unwrap_or(0)
    }

    /// One Haxe `writeFoodStatistics` line for `food_id`.
    ///
    /// Format:
    /// `{pct}% t: {totalPct}% pipes: {val} {name}[{id}] yum: {yum} meh: {meh} boni: {boni} mali: {mali}`
    // Haxe: WorldMap.writeFoodStatistics L828–839
    pub fn format_stats_line(&self, food_id: i32, name: &str) -> String {
        // Haxe: Math.round(x * 1) / 1 → nearest integer
        let round1 = |v: f32| v.round();
        let pct = self.eaten_percentage.get(&food_id).copied().unwrap_or(0.0);
        let total_pct = self.get_eaten_food_percentage(food_id);
        let val = round1(self.eaten_values.get(&food_id).copied().unwrap_or(0.0));
        let yum = round1(self.eaten_yum.get(&food_id).copied().unwrap_or(0.0));
        let meh = round1(self.eaten_meh.get(&food_id).copied().unwrap_or(0.0));
        let boni = round1(self.yum_boni.get(&food_id).copied().unwrap_or(0.0));
        let mali = round1(self.meh_mali.get(&food_id).copied().unwrap_or(0.0));
        let display = if name.is_empty() { "food" } else { name };
        format!(
            "{pct}% t: {total_pct}% pipes: {val} {display}[{food_id}] yum: {yum} meh: {meh} boni: {boni} mali: {mali}"
        )
    }

    /// Format FoodStats dump lines with a name resolver (sorted by food id).
    // Haxe: WorldMap.writeFoodStatistics
    pub fn format_stats_lines_with_names<F>(&self, mut name_of: F) -> Vec<String>
    where
        F: FnMut(i32) -> String,
    {
        let mut ids: Vec<i32> = self.eaten_values.keys().copied().collect();
        ids.sort_unstable();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let name = name_of(id);
            out.push(self.format_stats_line(id, &name));
        }
        out
    }

    /// Format a simple FoodStats dump line list (fallback name `"food"`).
    // Haxe: WorldMap.writeFoodStatistics
    pub fn format_stats_lines(&self) -> Vec<String> {
        self.format_stats_lines_with_names(|_| String::new())
    }

    /// Full dump body (joined with `\n`, trailing newline when non-empty).
    // Haxe: WorldMap.writeFoodStatistics
    pub fn format_stats_text_with_names<F>(&self, name_of: F) -> String
    where
        F: FnMut(i32) -> String,
    {
        let lines = self.format_stats_lines_with_names(name_of);
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }

    /// Full dump body with fallback names.
    pub fn format_stats_text(&self) -> String {
        self.format_stats_text_with_names(|_| String::new())
    }
}

/// Haxe `WorldMap.writeFoodStatistics` — recompute percentages then write text dump.
///
/// Does **not** load back (Haxe dump is write-only diagnostic / web source).
/// Path is typically `save_directory/FoodStats.txt` (Rust fixed name; Haxe used
/// `FoodStats{N}.txt` with rotating save slots).
// Haxe: WorldMap.writeFoodStatistics
pub fn write_food_statistics(
    stats: &WorldFoodStats,
    path: impl AsRef<Path>,
    name_of: impl FnMut(i32) -> String,
) -> Result<(), String> {
    let mut clone = stats.clone();
    clone.recompute_percentages();
    let text = clone.format_stats_text_with_names(name_of);
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("food stats mkdir: {e}"))?;
        }
    }
    fs::write(path, text.as_bytes()).map_err(|e| format!("food stats write: {e}"))
}

/// Write FoodStats with fallback names (`food[id]`).
// Haxe: WorldMap.writeFoodStatistics
pub fn write_food_statistics_ids_only(
    stats: &WorldFoodStats,
    path: impl AsRef<Path>,
) -> Result<(), String> {
    write_food_statistics(stats, path, |_| String::new())
}

/// Haxe `WebServer.generateFoodStatistics` HTML table body (pure).
///
/// Columns: Food | Eaten (%) | Related (HQ rollup %). Sorted by food id.
/// Wired by ol-web `/stats/food` from the live [`WorldFoodShare`] snapshot
/// (FOODSTATS-WEB). Not loaded back from disk.
// Haxe: WebServer.generateFoodStatistics L402–424
pub fn format_food_statistics_html<F>(stats: &WorldFoodStats, mut name_of: F) -> String
where
    F: FnMut(i32) -> String,
{
    let mut clone = stats.clone();
    clone.recompute_percentages();
    let mut ids: Vec<i32> = clone.eaten_percentage.keys().copied().collect();
    ids.sort_unstable();
    let mut out = String::from(
        "<br><br>\n<center><table>\n<tr><td><b>Food</b></td><td><b>Eaten</b></td><td><b>Related</b></td></tr>\n",
    );
    for id in ids {
        let name = name_of(id);
        let display = if name.is_empty() {
            format!("food[{id}]")
        } else {
            name
        };
        let eaten = clone.eaten_percentage.get(&id).copied().unwrap_or(0.0);
        // Haxe: Math.round(getEatenFoodPercentage(foodId))
        let related = clone.get_eaten_food_percentage(id).round() as i32;
        let eaten_i = eaten.round() as i32;
        out.push_str(&format!(
            "<tr><td>{display}</td><td>{eaten_i}%</td><td>{related}%</td></tr>\n"
        ));
    }
    out.push_str("</table></center>\n");
    out
}

fn eaten_percentage_with_hq(
    food_id: i32,
    eaten_pct: &[(i32, f32)],
    higher_quality: &[(i32, i32)],
    depth: u8,
) -> f32 {
    if depth > 16 {
        return 0.0;
    }
    let mut pct = 0.0_f32;
    for &(id, p) in eaten_pct {
        if id == food_id {
            pct = p;
            break;
        }
    }
    let mut hq = 0;
    for &(id, next) in higher_quality {
        if id == food_id {
            hq = next;
            break;
        }
    }
    if hq > 0 {
        pct += eaten_percentage_with_hq(hq, eaten_pct, higher_quality, depth + 1);
    }
    pct
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_best_food::{
        food_factor_from_eaten_percentage, FOOD_FACTOR_EATEN_GE_10, FOOD_FACTOR_EATEN_GE_8,
        FOOD_FACTOR_EATEN_LT_1, FOOD_FACTOR_EATEN_LT_5,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_map_food_factor_is_rare_band() {
        let s = WorldFoodStats::new();
        // No entries → percentage 0 → <1% band 2.5
        assert!((s.get_food_factor(31) - FOOD_FACTOR_EATEN_LT_1).abs() < 1e-5);
    }

    #[test]
    fn add_statistic_recomputes_percentage() {
        let mut s = WorldFoodStats::new();
        // base 5, final 10 (yum)
        s.add_food_statistic(100, 5.0, 10.0);
        s.add_food_statistic(200, 5.0, 10.0);
        assert_eq!(s.eaten_percentage.get(&100).copied(), Some(50.0));
        assert_eq!(s.eaten_percentage.get(&200).copied(), Some(50.0));
        // 50% → ≥10% band (Haxe getFoodFactor)
        assert!((s.get_food_factor(100) - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5);
        // Mid band 5..8 exclusive of 8: use 6%
        let mut s2 = WorldFoodStats::new();
        s2.add_food_statistic(100, 5.0, 6.0);
        s2.add_food_statistic(200, 5.0, 94.0);
        assert!((s2.get_food_factor(100) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dominant_food_gets_penalized_factor() {
        let mut s = WorldFoodStats::new();
        // 96% vs 4% — clear ≥10% penalty vs <5% boost (avoid exact 5% mid-band 1.0).
        // Haxe getFoodFactor: ≥10 → 0.5; <5 (and ≥3) → 1.5
        s.add_food_statistic(100, 5.0, 96.0);
        s.add_food_statistic(200, 5.0, 4.0);
        let f100 = s.get_food_factor(100);
        let f200 = s.get_food_factor(200);
        assert_eq!(s.eaten_percentage.get(&100).copied(), Some(96.0));
        assert_eq!(s.eaten_percentage.get(&200).copied(), Some(4.0));
        assert!((f100 - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5, "f100={f100}");
        assert!((f200 - FOOD_FACTOR_EATEN_LT_5).abs() < 1e-5, "f200={f200}");
    }

    #[test]
    fn yum_meh_split_tracked() {
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(1, 5.0, 8.0); // yum +3
        s.add_food_statistic(2, 5.0, 3.0); // meh -2
        assert!((s.yum_boni.get(&1).copied().unwrap_or(0.0) - 3.0).abs() < 1e-5);
        assert!((s.meh_mali.get(&2).copied().unwrap_or(0.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn higher_quality_rollup_affects_factor() {
        let mut s = WorldFoodStats::new();
        // Only cooked berry pie eaten → gooseberry rolls up HQ chain 31→253→272
        s.add_food_statistic(272, 10.0, 4.0);
        s.add_food_statistic(999, 10.0, 96.0);
        // 272 is 4% → LT_5 band 1.5; 31 rolls up to same
        assert!((s.get_food_factor(272) - FOOD_FACTOR_EATEN_LT_5).abs() < 1e-5);
        assert!((s.get_food_factor(31) - FOOD_FACTOR_EATEN_LT_5).abs() < 1e-5);
    }

    #[test]
    fn starving_empty_is_1_5() {
        let s = WorldFoodStats::new();
        assert!((s.get_starving_food_factor() - 1.5).abs() < 1e-5);
        assert!((s.get_starving_food_factor_at(0.0) - 1.5).abs() < 1e-5);
    }

    #[test]
    fn note_death_reason_only_age_and_hunger() {
        let mut s = WorldFoodStats::new();
        s.note_death_reason("reason_age");
        s.note_death_reason("reason_hunger");
        s.note_death_reason("reason_hunger_kid");
        s.note_death_reason("reason_nursing_hunger");
        s.note_death_reason("reason_killed_33");
        assert_eq!(s.reason_age_deaths, 1);
        assert_eq!(s.reason_hunger_deaths, 1);
        assert_eq!(s.reason_killed_last_day_count("reason_age"), 1);
        assert_eq!(s.reason_killed_last_day_count("reason_hunger"), 1);
        assert_eq!(s.reason_killed_last_day_count("reason_hunger_kid"), 1);
        assert_eq!(s.reason_killed_last_day_count("reason_nursing_hunger"), 1);
        assert_eq!(s.reason_killed_last_day_count("reason_killed_33"), 1);
    }

    #[test]
    fn kid_hunger_remaps_out_of_starving() {
        let mut s = WorldFoodStats::new();
        s.note_death_reason_at(100.0, "reason_hunger", 3.0); // age < 5 → kid
        s.note_death_reason_at(100.0, "reason_hunger", 20.0); // adult
        assert_eq!(s.reason_hunger_deaths, 1);
        assert_eq!(s.reason_killed_last_day_count("reason_hunger_kid"), 1);
        assert!(
            (s.get_starving_food_factor_at(100.0) - starving_food_factor_from_deaths(0, 1)).abs()
                < 1e-5
        );
    }

    #[test]
    fn last_day_window_drops_old_deaths() {
        let mut s = WorldFoodStats::new();
        // Death at t=0; after 24h+1s it leaves the window.
        s.note_death_reason_at(0.0, "reason_hunger", 30.0);
        s.note_death_reason_at(0.0, "reason_age", 80.0);
        assert_eq!(s.reason_hunger_deaths, 1);
        assert_eq!(s.reason_age_deaths, 1);
        // Still inside day at t = LAST_DAY_SECS - 1
        let inside = LAST_DAY_SECS - 1.0;
        assert!(
            (s.get_starving_food_factor_at(inside) - starving_food_factor_from_deaths(1, 1)).abs()
                < 1e-5
        );
        // Outside: years_since >= 1440
        let outside = LAST_DAY_SECS + 1.0;
        assert!((s.get_starving_food_factor_at(outside) - 1.5).abs() < 1e-5);
        // Force refresh at outside time → cached counters drop
        s.refresh_starving_window(outside, true);
        assert_eq!(s.reason_age_deaths, 0);
        assert_eq!(s.reason_hunger_deaths, 0);
        assert!((s.get_starving_food_factor() - 1.5).abs() < 1e-5);
    }

    #[test]
    fn years_since_and_window_helpers() {
        assert!((years_since_from_secs(60.0) - 1.0).abs() < 1e-5);
        assert!((years_since_from_secs(LAST_DAY_SECS) - LAST_DAY_YEARS).abs() < 1e-3);
        assert!(is_last_day_years(1439.0));
        assert!(!is_last_day_years(1440.0));
        assert!(is_last_hour_years(59.0));
        assert!(!is_last_hour_years(60.0));
        assert!(is_death_in_last_day(10.0, 10.0 + LAST_DAY_SECS - 1.0));
        assert!(!is_death_in_last_day(10.0, 10.0 + LAST_DAY_SECS));
        // Session stamps allow death_sim_time == 0 (server-start deaths).
        assert!(is_death_in_last_day(0.0, 100.0));
        // Lineage field: deathTime 0 means never died.
        assert!(!is_lineage_death_time_in_last_day(0.0, 100.0));
        assert!(is_lineage_death_time_in_last_day(1.0, 100.0));
    }

    #[test]
    fn stats_throttle_skips_rebuild() {
        let mut s = WorldFoodStats::new();
        s.note_death_reason_at(0.0, "reason_age", 90.0);
        // After note at 0, last_statistic_sim = 0. At t=30 < 60 → skip.
        s.last_statistic_sim = 0.0;
        assert!(!s.refresh_starving_window(30.0, false));
        assert!(s.refresh_starving_window(30.0, true)); // force
        assert!(s.refresh_starving_window(100.0, false)); // past throttle
    }

    #[test]
    fn milk_hq_chain_rollup() {
        let mut s = WorldFoodStats::new();
        for _ in 0..9 {
            s.add_food_statistic(3593, 5.0, 10.0);
        }
        s.add_food_statistic(200, 5.0, 10.0);
        let f_bowl = s.get_food_factor(1463);
        let f_pouch = s.get_food_factor(4081);
        let f_bottle = s.get_food_factor(3593);
        assert!(
            (f_bottle - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5,
            "bottle={f_bottle}"
        );
        assert!(
            (f_bowl - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5,
            "bowl rolls up HQ={f_bowl}"
        );
        assert!(
            (f_pouch - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5,
            "pouch rolls up HQ={f_pouch}"
        );
    }

    #[test]
    fn pure_band_helpers() {
        assert!((food_factor_from_eaten_percentage(0.0) - FOOD_FACTOR_EATEN_LT_1).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(8.5) - FOOD_FACTOR_EATEN_GE_8).abs() < 1e-5);
        assert!((food_factor_from_eaten_percentage(12.0) - FOOD_FACTOR_EATEN_GE_10).abs() < 1e-5);
    }

    #[test]
    fn format_stats_line_shape() {
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(40, 3.0, 6.0);
        let line = s.format_stats_line(40, "Wild Carrot");
        assert!(line.contains("% t:"), "{line}");
        assert!(line.contains("pipes:"), "{line}");
        assert!(line.contains("Wild Carrot[40]"), "{line}");
        assert!(line.contains("yum:"), "{line}");
    }

    #[test]
    fn write_food_statistics_roundtrip_file() {
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(40, 3.0, 6.0);
        s.add_food_statistic(31, 2.0, 2.0); // meh path (final == base)
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ol_foodstats_{nanos}.txt"));
        write_food_statistics(&s, &path, |id| format!("name{id}")).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert!(text.contains("name40[40]"), "{text}");
        assert!(text.contains("name31[31]"), "{text}");
    }

    #[test]
    fn format_stats_hq_total_percent_line() {
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(40, 3.0, 6.0);
        s.add_food_statistic(31, 2.0, 4.0);
        // HQ: 40→402 so related may exceed plain %
        let line = s.format_stats_line(40, "carrot");
        assert!(line.contains("% t:"), "{line}");
    }

    #[test]
    fn html_table_shape() {
        let s = WorldFoodStats::new();
        let html = format_food_statistics_html(&s, |_| String::new());
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("Eaten"), "{html}");
    }

    #[test]
    fn html_with_foods_sorted() {
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(272, 10.0, 40.0); // cooked berry pie 40%
        s.add_food_statistic(31, 2.0, 10.0); // gooseberry 10%
        s.add_food_statistic(999, 5.0, 50.0); // filler
        let html = format_food_statistics_html(&s, |id| format!("F{id}"));
        assert!(html.contains("F31"), "{html}");
        assert!(html.contains("F272"), "{html}");
        // ids appear in sorted order in table rows
        let i31 = html.find("F31").unwrap();
        let i272 = html.find("F272").unwrap();
        let i999 = html.find("F999").unwrap();
        assert!(i31 < i272 && i272 < i999, "sorted by id");
    }

    #[test]
    fn haxe_slot_filename() {
        assert_eq!(haxe_food_stats_slot_filename(3), "FoodStats3.txt");
    }

    #[test]
    fn apply_world_food_factors_multiplies() {
        assert!((apply_world_food_factors(10.0, 2.5, 1.5) - 37.5).abs() < 1e-4);
        assert!((apply_world_food_factors(-1.0, 2.0, 1.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn super_meh_helpers() {
        assert!((super_meh_extra_food_value(true) - 1.0).abs() < 1e-5);
        assert!((super_meh_extra_food_value(false) - 0.0).abs() < 1e-5);
        assert!(super_meh_food_max_is_deadly(0.5));
        assert!(!super_meh_food_max_is_deadly(1.0));
        let mut s = WorldFoodStats::new();
        s.add_food_statistic(40, 3.0, 6.0);
        s.add_food_statistic(31, 2.0, 2.0);
        let _ = s.format_stats_text();
    }

    #[test]
    fn starving_food_factor_from_deaths_extremes() {
        // Empty → 1.5
        assert!((starving_food_factor_from_deaths(0, 0) - 1.5).abs() < 1e-5);
        // Age-only heavy: hunger=0, age large → approaches ~0.5
        let age_only = starving_food_factor_from_deaths(1000, 0);
        assert!(age_only < 0.55 && age_only > 0.49, "age_only={age_only}");
        // Hunger-heavy → approaches ~2.0
        let hunger_heavy = starving_food_factor_from_deaths(0, 1000);
        assert!(
            hunger_heavy > 1.95 && hunger_heavy <= 2.0,
            "hunger_heavy={hunger_heavy}"
        );
    }

    #[test]
    fn kid_hunger_and_kill_name_normalize() {
        assert_eq!(
            normalize_death_reason_for_stats("reason_hunger", 3.0),
            "reason_hunger_kid"
        );
        assert_eq!(
            normalize_death_reason_for_stats("reason_hunger", 20.0),
            "reason_hunger"
        );
        assert_eq!(parse_reason_killed_object_id("reason_killed_33"), Some(33));
        assert_eq!(parse_reason_killed_object_id("reason_hunger"), None);
        assert_eq!(
            normalize_death_reason_for_stats_ex("reason_killed_33", 40.0, Some("Knife")),
            "Knife"
        );
        assert_eq!(
            normalize_death_reason_for_stats_with_resolver("reason_killed_99", 40.0, |id| {
                if id == 99 {
                    Some("Wolf".into())
                } else {
                    None
                }
            }),
            "Wolf"
        );
        // No resolver → raw wire key
        assert_eq!(
            normalize_death_reason_for_stats("reason_killed_33", 40.0),
            "reason_killed_33"
        );
    }

    #[test]
    fn seed_death_stamps_from_lineage_rows_rebuilds_window() {
        let rows = vec![
            LineageStatRow::from_death_fields(100.0, "reason_hunger", 30.0, 1),
            LineageStatRow::from_death_fields(100.0, "reason_age", 90.0, 0),
            LineageStatRow::from_death_fields(0.0, "", 0.0, 2), // living — skipped
            LineageStatRow::from_death_fields(100.0, "reason_hunger", 2.0, 1), // kid
        ];
        let mut s = WorldFoodStats::new();
        let n = s.seed_death_stamps_from_lineage_rows(rows, 100.0);
        assert_eq!(n, 3);
        assert_eq!(s.reason_age_deaths, 1);
        assert_eq!(s.reason_hunger_deaths, 1); // kid remapped out
        assert_eq!(s.reason_killed_last_day_count("reason_hunger_kid"), 1);
        assert!(
            (s.get_starving_food_factor_at(100.0) - starving_food_factor_from_deaths(1, 1)).abs()
                < 1e-5
        );
    }

    #[test]
    fn generate_lineage_statistics_maps_and_windows() {
        let now = 10_000.0;
        let rows = vec![
            LineageStatRow::from_death_fields(now - 10.0, "reason_hunger", 25.0, 3),
            LineageStatRow::from_death_fields(now - LAST_HOUR_SECS - 5.0, "reason_age", 80.0, 1),
            LineageStatRow::from_death_fields(
                now - LAST_DAY_SECS - 10.0,
                "reason_killed_33",
                40.0,
                2,
            ),
            LineageStatRow::from_death_fields(0.0, "", 0.0, 0), // living
        ];
        let stats = generate_lineage_statistics(rows, now, |id| {
            if id == 33 {
                Some("Arrow".into())
            } else {
                None
            }
        });
        assert_eq!(stats.reason_killed.get("reason_hunger"), Some(&1));
        assert_eq!(stats.reason_killed.get("reason_age"), Some(&1));
        assert_eq!(stats.reason_killed.get("Arrow"), Some(&1));
        // Last hour: only the hunger death at now-10
        assert_eq!(stats.reason_killed_last_hour.get("reason_hunger"), Some(&1));
        assert!(!stats.reason_killed_last_hour.contains_key("reason_age"));
        // Last day: hunger + age (kill is older than 24h)
        assert_eq!(stats.reason_killed_last_day.get("reason_hunger"), Some(&1));
        assert_eq!(stats.reason_killed_last_day.get("reason_age"), Some(&1));
        assert!(!stats.reason_killed_last_day.contains_key("Arrow"));
        assert!(stats.ages.contains_key(&25) || stats.ages.contains_key(&-1));
        assert!(stats.generations.get(&3).copied().unwrap_or(0) >= 1);
        assert_eq!(stats.count_last_day, 2);
        assert_eq!(stats.count_old, 2); // old kill + living

        let html = format_lineage_statistics_html(&stats);
        assert!(html.contains("Reason killed"));
        assert!(html.contains("<b>Age</b>"));
        let _ = format_lineage_death_reason_html(&stats);
        assert!(html.contains("STARVATION"), "{html}");
        assert!(html.contains("OLD AGE"), "{html}");
        assert!(html.contains("Arrow"), "{html}");
        assert!(html.contains("Extra food because of Starving"), "{html}");
    }

    #[test]
    fn last_hour_stamp_histogram() {
        let mut s = WorldFoodStats::new();
        // now must be > LAST_HOUR_SECS so older death stays non-negative.
        let now = LAST_HOUR_SECS + 500.0;
        s.note_death_reason_at(now, "reason_age", 90.0);
        s.note_death_reason_at(now - LAST_HOUR_SECS - 1.0, "reason_hunger", 30.0);
        s.refresh_starving_window(now, true);
        assert_eq!(s.reason_killed_last_hour_count("reason_age"), 1);
        assert_eq!(s.reason_killed_last_hour_count("reason_hunger"), 0);
        // Still in last day
        assert_eq!(s.reason_killed_last_day_count("reason_hunger"), 1);
    }
}
