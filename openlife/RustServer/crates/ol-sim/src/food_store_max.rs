//! Pure food capacity recompute (Haxe `calculateFoodStoreMax` / exhaustion–wound pipes).
//!
//! Chunk **EXHAUSTION-WOUND** / `wound_food_pipes` + **HEALTH-AGE-FOOD** / `health_food_max`:
//! - base = GrownUpFoodStoreMax × health factor (age bands overwrite base)
//! - starve reduction, −hits, −exhaustion with half-floor
//! - healing extras for exhaustion / wound hits (TimeHelper.updateFoodAndDoHealing)
//! - live yum_multiplier / medianPrestige → health food-max + aging speed factors

// Haxe: ServerSettings.GrownUpFoodStoreMax / NewBorn / OldAge / MaxAge
pub const GROWN_UP_FOOD_STORE_MAX: f32 = 20.0;
pub const NEWBORN_FOOD_STORE_MAX: f32 = 4.0;
pub const OLD_AGE_FOOD_STORE_MAX: f32 = 10.0;
/// Haxe `ServerSettings.MaxAge` used in the old-age food capacity band.
pub const FOOD_CAPACITY_MAX_AGE: f32 = 60.0;
/// Age below which the newborn→grown-up linear band replaces the base (Haxe hardcodes 20).
pub const FOOD_CAPACITY_YOUTH_AGE: f32 = 20.0;
/// Years from MaxAge where old-age band begins (Haxe: maxAge - 10).
pub const FOOD_CAPACITY_OLD_BAND_YEARS: f32 = 10.0;

// Haxe: ServerSettings.DeathWithFoodStoreMax
pub const DEATH_WITH_FOOD_STORE_MAX: f32 = -0.1;
// Haxe: ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath
pub const FOOD_STORE_MAX_REDUCTION_WHILE_STARVING: f32 = 5.0;
// Haxe: ServerSettings.CombatExhaustionCostPerAttack
pub const COMBAT_EXHAUSTION_COST_PER_ATTACK: f32 = 0.1;
// Haxe: ServerSettings.ExhaustionHealingFactor / ExhaustionHealingForMaleFaktor
pub const EXHAUSTION_HEALING_FACTOR: f32 = 1.5;
pub const EXHAUSTION_HEALING_FOR_MALE_FACTOR: f32 = 1.2;
// Haxe: ServerSettings.WoundHealingFactor / WoundDamageFactor
pub const WOUND_HEALING_FACTOR: f32 = 1.0;
pub const WOUND_DAMAGE_FACTOR: f32 = 1.0;
// Haxe: ServerSettings.ExhaustionYellowFeverPerSec
pub const EXHAUSTION_YELLOW_FEVER_PER_SEC: f32 = 0.1;
/// Haxe yellow-fever heat rise per second (`0.02 * isHeldFaktor`).
// Haxe: TimeHelper.updateFoodAndDoHealing hasYellowFever heat L451
pub const YELLOW_FEVER_HEAT_PER_SEC: f32 = 0.02;
/// Haxe `isHeldFaktor` when `heldByPlayer != null` (cared for → milder heat).
// Haxe: TimeHelper L445 isHeldFaktor 0.2
pub const YELLOW_FEVER_HELD_BY_FACTOR: f32 = 0.2;
// Haxe: ServerSettings.HealingPerSecond
pub const HEALING_PER_SECOND: f32 = 0.10;
// Haxe: ServerSettings.TemperatureHitsDamageFactor / TemperatureExhaustionDamageFactor
// C-SS-TEMP-HEAL
pub const TEMPERATURE_HITS_DAMAGE_FACTOR: f32 = 0.5;
pub const TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR: f32 = 0.2;
/// Absolute heat thresholds for ×2 temperature damage (independent of person-color super gates).
// Haxe: TimeHelper.updateFoodAndDoHealing heat > 0.95 / heat < 0.05
pub const TEMPERATURE_DAMAGE_DOUBLE_HOT: f32 = 0.95;
pub const TEMPERATURE_DAMAGE_DOUBLE_COLD: f32 = 0.05;
/// Age years before temperature hits/exhaustion apply (BB first-year grace).
// Haxe: TimeHelper.updateFoodAndDoHealing if (player.age > 1)
pub const TEMPERATURE_DAMAGE_MIN_AGE: f32 = 1.0;
// Haxe BiomeTag ids used by biomeLoveFactor
// Haxe: Biome.hx BiomeTag
pub const BIOME_TAG_GREEN: i32 = 0;
pub const BIOME_TAG_SWAMP: i32 = 1;
pub const BIOME_TAG_YELLOW: i32 = 2;
pub const BIOME_TAG_GREY: i32 = 3;
pub const BIOME_TAG_SNOW: i32 = 4;
pub const BIOME_TAG_DESERT: i32 = 5;
pub const BIOME_TAG_JUNGLE: i32 = 6;
pub const BIOME_TAG_PASSABLERIVER: i32 = 13;
// Haxe: ServerSettings.Max/MinHealthFoodStoreMaxFactor
pub const MAX_HEALTH_FOOD_STORE_MAX_FACTOR: f32 = 1.2;
pub const MIN_HEALTH_FOOD_STORE_MAX_FACTOR: f32 = 0.8;
// Haxe: ServerSettings.Max/MinHealthAgingFactor
pub const MAX_HEALTH_AGING_FACTOR: f32 = 2.0;
pub const MIN_HEALTH_AGING_FACTOR: f32 = 0.5;
// Haxe: ServerSettings.MinHealthPerYear (expected health per year for normal health)
pub const MIN_HEALTH_PER_YEAR: f32 = 1.0;
/// Floor for Haxe `medianPrestige` = `MinHealthPerYear * 30`.
pub const MIN_HEALTH_MEDIAN_PRESTIGE: f32 = MIN_HEALTH_PER_YEAR * 30.0;
// Haxe: ServerSettings.AgingFactorWhileStarvingToDeath / GrownUpAge / AgeingSecondsPerYear
pub const AGING_FACTOR_WHILE_STARVING: f32 = 0.5;
pub const GROWN_UP_AGE_YEARS: f32 = 14.0;
pub const AGEING_SECONDS_PER_YEAR: f32 = 60.0;
// Haxe: ServerSettings.MinAgeToEat — birth cross-species aging mults only below this
pub const MIN_AGE_TO_EAT_YEARS: f32 = 3.0;
// Haxe: ServerSettings.AgingFactorHumanBornToAi / AgingFactorAiBornToHuman
pub const AGING_FACTOR_HUMAN_BORN_TO_AI: f32 = 3.0;
pub const AGING_FACTOR_AI_BORN_TO_HUMAN: f32 = 1.5;
// Haxe: ServerSettings.BirthPrestigeFactor — spawn yum_multiplier seed from account.totalScore
pub const BIRTH_PRESTIGE_FACTOR: f32 = 0.4;
// Haxe: ServerSettings.MaxAge for health median scaling (same as food capacity max age)
pub const HEALTH_MEDIAN_MAX_AGE: f32 = 60.0;

/// Haxe `GlobalPlayerInstance.calculateNotReducedFoodStoreMax`.
///
/// Currently returns GrownUp only (Haxe TODO: increase with health — port-as-is).
// Haxe: GlobalPlayerInstance.calculateNotReducedFoodStoreMax
#[inline]
pub fn calculate_not_reduced_food_store_max() -> f32 {
    calculate_not_reduced_food_store_max_ex(GROWN_UP_FOOD_STORE_MAX)
}

/// Live-knob variant: `grown_up` = Haxe `GrownUpFoodStoreMax`.
// Haxe: GlobalPlayerInstance.calculateNotReducedFoodStoreMax
// C-SS-TAIL-KNOBS
#[inline]
pub fn calculate_not_reduced_food_store_max_ex(grown_up: f32) -> f32 {
    if grown_up.is_finite() && grown_up > 0.0 {
        grown_up
    } else {
        GROWN_UP_FOOD_STORE_MAX
    }
}

/// Haxe `GlobalPlayerInstance.CalculateHealthFactor(maxBoni, maxMali)`.
///
/// `health` = yum_multiplier − medianPrestige × (trueAge / (MaxAge/2)).
// Haxe: GlobalPlayerInstance.CalculateHealthFactor
pub fn calculate_health_factor(
    yum_multiplier: f32,
    median_prestige: f32,
    true_age: f32,
    max_age: f32,
    max_boni: f32,
    max_mali: f32,
) -> f32 {
    let median = if median_prestige.is_finite() {
        median_prestige
    } else {
        0.0
    };
    let max_age = if max_age.is_finite() && max_age > 0.0 {
        max_age
    } else {
        HEALTH_MEDIAN_MAX_AGE
    };
    let true_age = if true_age.is_finite() {
        true_age.max(0.0)
    } else {
        0.0
    };
    let yum = if yum_multiplier.is_finite() {
        yum_multiplier
    } else {
        0.0
    };
    // Haxe: health -= medianHealth * (this.trueAge / (ServerSettings.MaxAge / 2))
    let health = yum - median * (true_age / (max_age / 2.0));
    if health >= 0.0 {
        // (maxBoni * health + median) / (health + median)
        let denom = health + median;
        if denom.abs() < f32::EPSILON {
            1.0
        } else {
            (max_boni * health + median) / denom
        }
    } else {
        // (health - median) / ((1/maxMali) * health - median)
        let inv_mali = if max_mali.abs() < f32::EPSILON {
            1.0
        } else {
            1.0 / max_mali
        };
        let denom = inv_mali * health - median;
        if denom.abs() < f32::EPSILON {
            1.0
        } else {
            (health - median) / denom
        }
    }
}

/// Haxe `GlobalPlayerInstance.CalculateHealthFoodStoreMaxFactor`.
// Haxe: GlobalPlayerInstance.CalculateHealthFoodStoreMaxFactor
#[inline]
pub fn calculate_health_food_store_max_factor(
    yum_multiplier: f32,
    median_prestige: f32,
    true_age: f32,
) -> f32 {
    calculate_health_factor(
        yum_multiplier,
        median_prestige,
        true_age,
        HEALTH_MEDIAN_MAX_AGE,
        MAX_HEALTH_FOOD_STORE_MAX_FACTOR,
        MIN_HEALTH_FOOD_STORE_MAX_FACTOR,
    )
}

/// Haxe `GlobalPlayerInstance.CalculateHealthAgeFactor`.
// Haxe: GlobalPlayerInstance.CalculateHealthAgeFactor
#[inline]
pub fn calculate_health_age_factor(
    yum_multiplier: f32,
    median_prestige: f32,
    true_age: f32,
) -> f32 {
    calculate_health_factor(
        yum_multiplier,
        median_prestige,
        true_age,
        HEALTH_MEDIAN_MAX_AGE,
        MAX_HEALTH_AGING_FACTOR,
        MIN_HEALTH_AGING_FACTOR,
    )
}

/// Haxe `medianPrestige = Math.max(neededPrestige, MinHealthPerYear * 30)`.
///
/// `needed_commoner` is [`crate::prestige::calculate_needed_prestige`] at 0.4.
// Haxe: GlobalPlayerInstance.calculatePrestigeClass medianPrestige L1078
#[inline]
pub fn median_prestige_for_health(needed_commoner_prestige: f32) -> f32 {
    let needed = if needed_commoner_prestige.is_finite() {
        needed_commoner_prestige
    } else {
        0.0
    };
    needed.max(MIN_HEALTH_MEDIAN_PRESTIGE)
}

/// Result of one Haxe `TimeHelper.updateAge` time step (wall + display age).
// Haxe: TimeHelper.updateAge
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgeStepResult {
    /// Wall-clock years advanced (always `dt / AgeingSecondsPerYear`).
    pub true_age_delta: f32,
    /// Display age years advanced (`true_age_delta * ageing_factor`).
    pub age_delta: f32,
    /// Combined ageing factor after health + birth + starve gates.
    pub ageing_factor: f32,
    /// Haxe `player.age_r = AgeingSecondsPerYear / ageingFactor` (wire rate).
    pub age_r: f32,
}

/// Haxe birth aging mult when `age < MinAgeToEat` and mother is opposite human/AI.
///
/// - Human child + AI mother → [`AGING_FACTOR_HUMAN_BORN_TO_AI`] (3)
/// - AI child + human mother → [`AGING_FACTOR_AI_BORN_TO_HUMAN`] (1.5)
/// - Otherwise 1.0 (`mother_is_ai = None` when no mother)
// Haxe: TimeHelper.updateAge L707-713
pub fn birth_cross_species_aging_mult(
    age: f32,
    child_is_human: bool,
    mother_is_ai: Option<bool>,
) -> f32 {
    birth_cross_species_aging_mult_ex(age, child_is_human, mother_is_ai, MIN_AGE_TO_EAT_YEARS)
}

/// Same as [`birth_cross_species_aging_mult`] with live MinAgeToEat.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn birth_cross_species_aging_mult_ex(
    age: f32,
    child_is_human: bool,
    mother_is_ai: Option<bool>,
    min_age_to_eat: f32,
) -> f32 {
    let age = if age.is_finite() { age.max(0.0) } else { 0.0 };
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT_YEARS
    };
    if age >= min_age {
        return 1.0;
    }
    match mother_is_ai {
        Some(true) if child_is_human => AGING_FACTOR_HUMAN_BORN_TO_AI,
        Some(false) if !child_is_human => AGING_FACTOR_AI_BORN_TO_HUMAN,
        _ => 1.0,
    }
}

/// Haxe `player.age_r = AgeingSecondsPerYear / ageingFactor`.
// Haxe: TimeHelper.updateAge L725
#[inline]
pub fn age_r_from_ageing_factor(ageing_factor: f32) -> f32 {
    age_r_from_ageing_factor_ex(ageing_factor, AGEING_SECONDS_PER_YEAR)
}

/// Like [`age_r_from_ageing_factor`] with live `AgeingSecondsPerYear`.
// Haxe: ServerSettings.AgeingSecondsPerYear
#[inline]
pub fn age_r_from_ageing_factor_ex(ageing_factor: f32, ageing_seconds_per_year: f32) -> f32 {
    let f = if ageing_factor.is_finite() && ageing_factor > 0.0 {
        ageing_factor
    } else {
        1.0
    };
    let secs = if ageing_seconds_per_year.is_finite() && ageing_seconds_per_year > 0.0 {
        ageing_seconds_per_year
    } else {
        AGEING_SECONDS_PER_YEAR
    };
    secs / f
}

/// Haxe GPI init: `yum_multiplier = max(totalScore * BirthPrestigeFactor, (median/30)*trueAge)`.
// Haxe: GlobalPlayerInstance init L1001-1003
pub fn birth_yum_multiplier(
    account_total_score: f32,
    median_prestige: f32,
    true_age: f32,
) -> f32 {
    birth_yum_multiplier_ex(
        account_total_score,
        median_prestige,
        true_age,
        BIRTH_PRESTIGE_FACTOR,
    )
}

/// Like [`birth_yum_multiplier`] with live `BirthPrestigeFactor`.
// Haxe: ServerSettings.BirthPrestigeFactor (SETTINGS-FIELD-MAP live)
pub fn birth_yum_multiplier_ex(
    account_total_score: f32,
    median_prestige: f32,
    true_age: f32,
    birth_prestige_factor: f32,
) -> f32 {
    let score = if account_total_score.is_finite() {
        account_total_score.max(0.0)
    } else {
        0.0
    };
    let median = if median_prestige.is_finite() {
        median_prestige.max(0.0)
    } else {
        0.0
    };
    let true_age = if true_age.is_finite() {
        true_age.max(0.0)
    } else {
        0.0
    };
    let factor = if birth_prestige_factor.is_finite() && birth_prestige_factor >= 0.0 {
        birth_prestige_factor
    } else {
        BIRTH_PRESTIGE_FACTOR
    };
    let from_score = score * factor;
    // Haxe: (medianPrestige / 30) * trueAge
    let floor = (median / 30.0) * true_age;
    from_score.max(floor)
}

/// Haxe `TimeHelper.updateAge` core ageing factor + birth mult + age_r.
///
/// - Youth (`age < GrownUpAge`): health age factor **not** applied (Haxe branch commented out)
/// - Adult: `ageingFactor = 1 / healthAgeFactor`
/// - Birth cross-species mult (pass [`birth_cross_species_aging_mult`] result; `1.0` if N/A)
/// - Starving (`food_store < 0`): ×0.5 if youth, ×2 if adult
// Haxe: TimeHelper.updateAge L691-732
pub fn age_step_from_health(
    dt: f32,
    age: f32,
    food_store: f32,
    health_age_factor: f32,
    birth_aging_mult: f32,
) -> AgeStepResult {
    age_step_from_health_ex(
        dt,
        age,
        food_store,
        health_age_factor,
        birth_aging_mult,
        AGEING_SECONDS_PER_YEAR,
    )
}

/// Like [`age_step_from_health`] with live `AgeingSecondsPerYear`.
// Haxe: ServerSettings.AgeingSecondsPerYear (SETTINGS-FIELD-MAP live)
pub fn age_step_from_health_ex(
    dt: f32,
    age: f32,
    food_store: f32,
    health_age_factor: f32,
    birth_aging_mult: f32,
    ageing_seconds_per_year: f32,
) -> AgeStepResult {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let age = if age.is_finite() { age.max(0.0) } else { 0.0 };
    let food = if food_store.is_finite() {
        food_store
    } else {
        0.0
    };
    let hf = if health_age_factor.is_finite() && health_age_factor > 0.0 {
        health_age_factor
    } else {
        1.0
    };
    let birth_m = if birth_aging_mult.is_finite() && birth_aging_mult > 0.0 {
        birth_aging_mult
    } else {
        1.0
    };
    let secs = if ageing_seconds_per_year.is_finite() && ageing_seconds_per_year > 0.0 {
        ageing_seconds_per_year
    } else {
        AGEING_SECONDS_PER_YEAR
    };

    let mut ageing_factor = 1.0;
    // Haxe: if (age < GrownUpAge) { /* ageingFactor = healthFactor; commented */ }
    // else ageingFactor = 1 / healthFactor
    if age >= GROWN_UP_AGE_YEARS {
        ageing_factor = 1.0 / hf;
    }
    // Haxe: human/AI mother birth mults when age < MinAgeToEat (before starve gate)
    ageing_factor *= birth_m;
    if food < 0.0 {
        if age < GROWN_UP_AGE_YEARS {
            ageing_factor *= AGING_FACTOR_WHILE_STARVING;
        } else {
            ageing_factor *= 1.0 / AGING_FACTOR_WHILE_STARVING;
        }
    }

    let true_age_delta = dt / secs;
    let age_r = age_r_from_ageing_factor_ex(ageing_factor, secs);
    AgeStepResult {
        true_age_delta,
        age_delta: true_age_delta * ageing_factor,
        ageing_factor,
        age_r,
    }
}

/// Inputs for Haxe `calculateFoodStoreMax`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodStoreMaxInput {
    pub age: f32,
    pub food_store: f32,
    pub hits: f32,
    pub exhaustion: f32,
    /// From [`calculate_health_food_store_max_factor`]; use `1.0` when unknown.
    pub health_factor: f32,
}

impl Default for FoodStoreMaxInput {
    fn default() -> Self {
        Self {
            age: 20.0,
            food_store: 10.0,
            hits: 0.0,
            exhaustion: 0.0,
            health_factor: 1.0,
        }
    }
}

/// Live age-band + base capacity knobs (Haxe GrownUp / NewBorn / OldAge FoodStoreMax).
// Haxe: ServerSettings.GrownUpFoodStoreMax / NewBornFoodStoreMax / OldAgeFoodStoreMax
// C-SS-AGE-FOOD
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodStoreMaxKnobs {
    pub grown_up: f32,
    pub newborn: f32,
    pub old_age: f32,
}

impl Default for FoodStoreMaxKnobs {
    fn default() -> Self {
        Self {
            grown_up: GROWN_UP_FOOD_STORE_MAX,
            newborn: NEWBORN_FOOD_STORE_MAX,
            old_age: OLD_AGE_FOOD_STORE_MAX,
        }
    }
}

impl FoodStoreMaxKnobs {
    /// Sanitize non-finite / non-positive knobs to Haxe module defaults.
    // C-SS-AGE-FOOD
    #[inline]
    pub fn sanitize(self) -> Self {
        Self {
            grown_up: calculate_not_reduced_food_store_max_ex(self.grown_up),
            newborn: if self.newborn.is_finite() && self.newborn > 0.0 {
                self.newborn
            } else {
                NEWBORN_FOOD_STORE_MAX
            },
            old_age: if self.old_age.is_finite() && self.old_age > 0.0 {
                self.old_age
            } else {
                OLD_AGE_FOOD_STORE_MAX
            },
        }
    }

    /// Grown-up-only override (module newborn/old_age) — C-SS-TAIL-KNOBS compat.
    #[inline]
    pub fn with_grown_up(grown_up: f32) -> Self {
        Self {
            grown_up,
            ..Self::default()
        }
    }
}

/// Haxe `GlobalPlayerInstance.calculateFoodStoreMax`.
///
/// Order (port-as-is):
/// 1. base = notReduced × healthFactor
/// 2. if age < 20: overwrite with newborn→grown linear (no health factor)
/// 3. if age > MaxAge−10: overwrite with old→grown linear
/// 4. if food_store < 0: += FoodStoreMaxReductionWhileStarving × food_store
/// 5. −= hits
/// 6. if exhaustion > 0: −= exhaustion, floor at half of pre-exhaustion value
// Haxe: GlobalPlayerInstance.calculateFoodStoreMax
pub fn calculate_food_store_max(input: FoodStoreMaxInput) -> f32 {
    calculate_food_store_max_ex(input, FoodStoreMaxKnobs::default())
}

/// Live-knob variant: age bands + base from [`FoodStoreMaxKnobs`].
// Haxe: GlobalPlayerInstance.calculateFoodStoreMax + ServerSettings.*FoodStoreMax
// C-SS-TAIL-KNOBS / C-SS-AGE-FOOD
pub fn calculate_food_store_max_ex(input: FoodStoreMaxInput, knobs: FoodStoreMaxKnobs) -> f32 {
    let knobs = knobs.sanitize();
    let grown = knobs.grown_up;
    let newborn = knobs.newborn;
    let old_age = knobs.old_age;
    let age = if input.age.is_finite() {
        input.age
    } else {
        0.0
    };
    let health = if input.health_factor.is_finite() && input.health_factor > 0.0 {
        input.health_factor
    } else {
        1.0
    };
    let mut max = grown * health;

    if age < FOOD_CAPACITY_YOUTH_AGE {
        // NewBorn + age/20 * (GrownUp - NewBorn)
        max = newborn + age / FOOD_CAPACITY_YOUTH_AGE * (grown - newborn);
    }
    let max_age = FOOD_CAPACITY_MAX_AGE;
    if age > max_age - FOOD_CAPACITY_OLD_BAND_YEARS {
        // OldAge + (maxAge - age)/10 * (GrownUp - OldAge)
        max = old_age + (max_age - age) / FOOD_CAPACITY_OLD_BAND_YEARS * (grown - old_age);
    }

    let food = if input.food_store.is_finite() {
        input.food_store
    } else {
        0.0
    };
    if food < 0.0 {
        max += FOOD_STORE_MAX_REDUCTION_WHILE_STARVING * food;
    }

    let hits = if input.hits.is_finite() {
        input.hits.max(0.0)
    } else {
        0.0
    };
    max -= hits;

    let exhaustion = if input.exhaustion.is_finite() {
        input.exhaustion
    } else {
        0.0
    };
    if exhaustion > 0.0 {
        let pre = max;
        max -= exhaustion;
        let floor = pre / 2.0;
        if max < floor {
            max = floor;
        }
    }

    max
}

/// Convenience wrapper for common call sites.
#[inline]
pub fn food_store_max_from_parts(
    age: f32,
    food_store: f32,
    hits: f32,
    exhaustion: f32,
    health_factor: f32,
) -> f32 {
    food_store_max_from_parts_ex(
        age,
        food_store,
        hits,
        exhaustion,
        health_factor,
        FoodStoreMaxKnobs::default(),
    )
}

/// Live-knob convenience wrapper.
// C-SS-TAIL-KNOBS / C-SS-AGE-FOOD
#[inline]
pub fn food_store_max_from_parts_ex(
    age: f32,
    food_store: f32,
    hits: f32,
    exhaustion: f32,
    health_factor: f32,
    knobs: FoodStoreMaxKnobs,
) -> f32 {
    calculate_food_store_max_ex(
        FoodStoreMaxInput {
            age,
            food_store,
            hits,
            exhaustion,
            health_factor,
        },
        knobs,
    )
}

/// True when recomputed food_store_max is below the starvation/wound death line.
// Haxe: TimeHelper.updateFoodAndDoHealing food_store_max < DeathWithFoodStoreMax
#[inline]
pub fn food_max_is_deadly(food_store_max: f32) -> bool {
    food_store_max < DEATH_WITH_FOOD_STORE_MAX
}

/// Haxe DoDamage combat death: `food_store_max < 0` (stricter than tick threshold −0.1).
// Haxe: GlobalPlayerInstance.DoDamage if (targetPlayer.food_store_max < 0)
#[inline]
pub fn food_max_is_combat_deadly(food_store_max: f32) -> bool {
    food_store_max < 0.0
}

/// Apply one real-damage hit to hits + exhaustion, recompute food_max.
///
/// Haxe DoDamage: `hits += damage` (real damage), always `exhaustion += damage`,
/// then `food_store_max = calculateFoodStoreMax()`.
// Haxe: GlobalPlayerInstance.DoDamage hits/exhaustion/food_store_max
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageFoodPipeResult {
    pub hits_after: f32,
    pub exhaustion_after: f32,
    pub food_store_max: f32,
    pub combat_lethal: bool,
}

pub fn apply_damage_food_pipe(
    age: f32,
    food_store: f32,
    hits_before: f32,
    exhaustion_before: f32,
    damage: f32,
    health_factor: f32,
    real_damage: bool,
) -> DamageFoodPipeResult {
    let dmg = if damage.is_finite() {
        damage.max(0.0)
    } else {
        0.0
    };
    let hits_after = if real_damage {
        (if hits_before.is_finite() {
            hits_before.max(0.0)
        } else {
            0.0
        }) + dmg
    } else if hits_before.is_finite() {
        hits_before.max(0.0)
    } else {
        0.0
    };
    let exhaustion_after = (if exhaustion_before.is_finite() {
        exhaustion_before
    } else {
        0.0
    }) + dmg;
    let food_store_max = food_store_max_from_parts(
        age,
        food_store,
        hits_after,
        exhaustion_after,
        health_factor,
    );
    DamageFoodPipeResult {
        hits_after,
        exhaustion_after,
        food_store_max,
        combat_lethal: food_max_is_combat_deadly(food_store_max),
    }
}

/// Spawn credit: Haxe sets `exhaustion = -food_store_max` after first recompute.
// Haxe: GlobalPlayerInstance init exhaustion = -food_store_max
#[inline]
pub fn spawn_exhaustion_credit(food_store_max: f32) -> f32 {
    if food_store_max.is_finite() {
        -food_store_max
    } else {
        -GROWN_UP_FOOD_STORE_MAX
    }
}

/// Snapshot of one vitals healing step (exhaustion + hits).
// Haxe: TimeHelper.updateFoodAndDoHealing doHealing branch
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealingFoodPipeStep {
    pub exhaustion_after: f32,
    pub hits_after: f32,
    pub extra_food_drain: f32,
    pub food_store_max: f32,
    pub deadly: bool,
}

/// Whether Haxe `doHealing` gates pass (simplified live set).
///
/// Haxe: not starving (food≥0), not super hot/cold (caller), not wounded, no yellow fever, angryTime > 0.
/// Caller should pass `is_super_hot_for_person || is_super_cold_for_person` (person-color thresholds),
/// not the crude 0.05/0.95 absolute extremes.
// Haxe: TimeHelper.updateFoodAndDoHealing doHealing
// C-SS-TEMP-HEAL
#[inline]
pub fn can_do_healing(
    food_store: f32,
    wounded: bool,
    has_yellow_fever: bool,
    angry_time: f32,
    super_heat: bool,
) -> bool {
    food_store >= 0.0
        && !wounded
        && !has_yellow_fever
        && angry_time > 0.0
        && !super_heat
}

/// Temperature hits + exhaustion extras for one vitals step.
///
/// When `age > 1` and super-hot or super-cold: damage = `original_food_decay`
/// (×2 when heat > 0.95 or < 0.05). Hits/exhaustion scaled by live factors.
// Haxe: TimeHelper.updateFoodAndDoHealing L901–910
// C-SS-TEMP-HEAL
#[inline]
pub fn temperature_damage_extras(
    age: f32,
    is_super_hot: bool,
    is_super_cold: bool,
    heat: f32,
    original_food_decay: f32,
) -> (f32, f32) {
    temperature_damage_extras_ex(
        age,
        is_super_hot,
        is_super_cold,
        heat,
        original_food_decay,
        TEMPERATURE_HITS_DAMAGE_FACTOR,
        TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
    )
}

/// Live-factor variant of [`temperature_damage_extras`].
// Haxe: ServerSettings.TemperatureHitsDamageFactor / TemperatureExhaustionDamageFactor
// C-SS-TEMP-HEAL
pub fn temperature_damage_extras_ex(
    age: f32,
    is_super_hot: bool,
    is_super_cold: bool,
    heat: f32,
    original_food_decay: f32,
    hits_damage_factor: f32,
    exhaustion_damage_factor: f32,
) -> (f32, f32) {
    if !(age.is_finite() && age > TEMPERATURE_DAMAGE_MIN_AGE) {
        return (0.0, 0.0);
    }
    let original = if original_food_decay.is_finite() {
        original_food_decay.max(0.0)
    } else {
        0.0
    };
    let heat = if heat.is_finite() { heat } else { 0.5 };
    let mut damage = 0.0;
    if is_super_hot {
        damage = if heat > TEMPERATURE_DAMAGE_DOUBLE_HOT {
            2.0 * original
        } else {
            original
        };
    } else if is_super_cold {
        damage = if heat < TEMPERATURE_DAMAGE_DOUBLE_COLD {
            2.0 * original
        } else {
            original
        };
    }
    if damage <= 0.0 {
        return (0.0, 0.0);
    }
    let hf = if hits_damage_factor.is_finite() && hits_damage_factor >= 0.0 {
        hits_damage_factor
    } else {
        TEMPERATURE_HITS_DAMAGE_FACTOR
    };
    let ef = if exhaustion_damage_factor.is_finite() && exhaustion_damage_factor >= 0.0 {
        exhaustion_damage_factor
    } else {
        TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR
    };
    (damage * hf, damage * ef)
}

/// Biome-love exhaustion recovery (not gated by `doHealing`).
///
/// When `love > 0` and `exhaustion > -food_store_max`: subtract
/// `healing * 0.5 * min(love, 1)`. Wrong-biome exhaustion gain stays off (Haxe commented).
// Haxe: TimeHelper.updateFoodAndDoHealing L915–918
// C-SS-TEMP-HEAL
#[inline]
pub fn biome_love_exhaustion_heal(
    exhaustion: f32,
    food_store_max: f32,
    healing: f32,
    biome_love_factor: f32,
) -> f32 {
    let mut exh = if exhaustion.is_finite() {
        exhaustion
    } else {
        0.0
    };
    let mut love = if biome_love_factor.is_finite() {
        biome_love_factor
    } else {
        0.0
    };
    // Haxe: if (biomeLoveFactor > 1) biomeLoveFactor = 1;
    if love > 1.0 {
        love = 1.0;
    }
    let food_max = if food_store_max.is_finite() {
        food_store_max
    } else {
        GROWN_UP_FOOD_STORE_MAX
    };
    let healing = if healing.is_finite() && healing > 0.0 {
        healing
    } else {
        0.0
    };
    // Haxe wrong-biome gain intentionally commented out — leave off.
    // if(biomeLoveFactor < 0) player.exhaustion -= originalFoodDecay * biomeLoveFactor / 2;
    if love > 0.0 && exh > -food_max {
        exh -= healing * 0.5 * love;
    }
    exh
}

/// Haxe `Biome.GetLovedBiomeByPlayer` / `IsBiomeLovedbyColor`.
// Haxe: Biome.IsBiomeLovedbyColor / GetLovedBiomeByPlayer
// C-SS-TEMP-HEAL
#[inline]
pub fn is_biome_loved_by_color(biome: i32, person_color: i32) -> bool {
    match person_color {
        // PersonColor.Ginger → SNOW
        6 => biome == BIOME_TAG_SNOW,
        // PersonColor.White → GREY
        4 => biome == BIOME_TAG_GREY,
        // PersonColor.Brown → JUNGLE
        3 => biome == BIOME_TAG_JUNGLE,
        // PersonColor.Black → DESERT
        1 => biome == BIOME_TAG_DESERT,
        _ => false,
    }
}

/// Haxe `GlobalPlayerInstance.BiomeLoveFactorForColor` for one person.
///
/// `mother_or_father`: parent contribution is halved after scoring.
// Haxe: GlobalPlayerInstance.BiomeLoveFactorForColor
// C-SS-TEMP-HEAL
pub fn biome_love_factor_for_color(
    biome: i32,
    person_color: i32,
    floor_id: i32,
    mother_or_father: bool,
) -> f32 {
    let mut loved: f32 = 0.0;
    if is_biome_loved_by_color(biome, person_color) {
        loved += 1.0;
    }
    // Self only: unloved non-GREEN/YELLOW → −0.5
    if !mother_or_father
        && loved <= 0.0
        && biome != BIOME_TAG_GREEN
        && biome != BIOME_TAG_YELLOW
    {
        loved -= 0.5;
    }
    // Self only: floor/bridge in swamp or passable river → −2.5 more when still unloved
    // Haxe code uses floorId != 0 (on floor); port-as-is despite top-level comment wording.
    if !mother_or_father
        && loved <= 0.0
        && floor_id != 0
        && (biome == BIOME_TAG_SWAMP || biome == BIOME_TAG_PASSABLERIVER)
    {
        loved -= 2.5;
    }
    if mother_or_father {
        loved *= 0.5;
    }
    loved
}

/// Haxe `GlobalPlayerInstance.biomeLoveFactor` from self + optional parent colors.
// Haxe: GlobalPlayerInstance.biomeLoveFactor
// C-SS-TEMP-HEAL
#[inline]
pub fn biome_love_factor(
    biome: i32,
    floor_id: i32,
    self_color: i32,
    mother_color: Option<i32>,
    father_color: Option<i32>,
) -> f32 {
    let mut loved = biome_love_factor_for_color(biome, self_color, floor_id, false);
    if let Some(mc) = mother_color {
        loved += biome_love_factor_for_color(biome, mc, floor_id, true);
    }
    if let Some(fc) = father_color {
        loved += biome_love_factor_for_color(biome, fc, floor_id, true);
    }
    loved
}

/// Live temperature + biome-love extras for the healing food pipe (C-SS-TEMP-HEAL).
// Haxe: TimeHelper.updateFoodAndDoHealing L901–918
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempBiomeHealKnobs {
    pub is_super_hot: bool,
    pub is_super_cold: bool,
    pub heat: f32,
    pub temperature_hits_damage_factor: f32,
    pub temperature_exhaustion_damage_factor: f32,
    /// Precomputed Haxe `biomeLoveFactor()` (capped to 1 inside heal helper).
    pub biome_love_factor: f32,
}

impl Default for TempBiomeHealKnobs {
    fn default() -> Self {
        Self {
            is_super_hot: false,
            is_super_cold: false,
            heat: 0.5,
            temperature_hits_damage_factor: TEMPERATURE_HITS_DAMAGE_FACTOR,
            temperature_exhaustion_damage_factor: TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
            biome_love_factor: 0.0,
        }
    }
}

/// One step of exhaustion + wound-hit healing with extra food drain.
///
/// - Exhaustion heal when `do_healing && exhaustion > -food_store_max`
/// - Hits heal when `do_healing && hits > 0`
/// - Recomputes food_store_max after deltas
// Haxe: TimeHelper.updateFoodAndDoHealing exhaustion/hits healing
pub fn step_healing_food_pipe(
    age: f32,
    food_store: f32,
    hits: f32,
    exhaustion: f32,
    health_factor: f32,
    dt: f32,
    original_food_decay: f32,
    do_healing: bool,
    is_male: bool,
) -> HealingFoodPipeStep {
    step_healing_food_pipe_ex(
        age,
        food_store,
        hits,
        exhaustion,
        health_factor,
        dt,
        original_food_decay,
        do_healing,
        is_male,
        HEALING_PER_SECOND,
        FoodStoreMaxKnobs::default(),
        EXHAUSTION_HEALING_FACTOR,
        WOUND_HEALING_FACTOR,
        EXHAUSTION_HEALING_FOR_MALE_FACTOR,
        TempBiomeHealKnobs::default(),
    )
}

/// Like [`step_healing_food_pipe`] with live `HealingPerSecond` + age-band capacity knobs
/// + live `ExhaustionHealingFactor` + live `WoundHealingFactor` + live male exhaustion factor
/// + live temperature damage + biome-love exhaustion heal.
// Haxe: ServerSettings.HealingPerSecond / ExhaustionHealingFactor / WoundHealingFactor / ExhaustionHealingForMaleFaktor / Temperature*DamageFactor / *FoodStoreMax
// SETTINGS-FIELD-MAP / C-SS-AGE-FOOD / C-SS-MORE-KNOBS / C-SS-WOUND-HEAL / C-SS-MALE-HEAL / C-SS-TEMP-HEAL
pub fn step_healing_food_pipe_ex(
    age: f32,
    food_store: f32,
    hits: f32,
    exhaustion: f32,
    health_factor: f32,
    dt: f32,
    original_food_decay: f32,
    do_healing: bool,
    is_male: bool,
    healing_per_second: f32,
    capacity: FoodStoreMaxKnobs,
    exhaustion_healing_factor: f32,
    wound_healing_factor: f32,
    exhaustion_healing_for_male_factor: f32,
    temp_biome: TempBiomeHealKnobs,
) -> HealingFoodPipeStep {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let original = if original_food_decay.is_finite() {
        original_food_decay.max(0.0)
    } else {
        0.0
    };
    let hps = if healing_per_second.is_finite() && healing_per_second >= 0.0 {
        healing_per_second
    } else {
        HEALING_PER_SECOND
    };
    let exh_f = if exhaustion_healing_factor.is_finite() && exhaustion_healing_factor >= 0.0 {
        exhaustion_healing_factor
    } else {
        EXHAUSTION_HEALING_FACTOR
    };
    let wh_f = if wound_healing_factor.is_finite() && wound_healing_factor >= 0.0 {
        wound_healing_factor
    } else {
        WOUND_HEALING_FACTOR
    };
    // Haxe: ServerSettings.ExhaustionHealingForMaleFaktor (male-only mult; female = 1)
    // C-SS-MALE-HEAL
    let male_f = if exhaustion_healing_for_male_factor.is_finite()
        && exhaustion_healing_for_male_factor >= 0.0
    {
        exhaustion_healing_for_male_factor
    } else {
        EXHAUSTION_HEALING_FOR_MALE_FACTOR
    };
    let healing = dt * hps;
    let mut hits_after = if hits.is_finite() { hits.max(0.0) } else { 0.0 };
    let mut exhaustion_after = if exhaustion.is_finite() {
        exhaustion
    } else {
        0.0
    };
    let mut extra = 0.0;

    // Starving: hits += originalFoodDecay * 0.5 (Haxe uses full original, not ×dt here —
    // originalFoodDecay is already timePassed * foodUsePerSecond).
    if food_store < 0.0 {
        hits_after += original * 0.5;
    }

    let food_max_probe = food_store_max_from_parts_ex(
        age,
        food_store,
        hits_after,
        exhaustion_after,
        health_factor,
        capacity,
    );

    if do_healing && exhaustion_after > -food_max_probe {
        // Haxe: TimeHelper.updateFoodAndDoHealing healingFaktor = isMale ? maleFactor : 1
        let male = if is_male { male_f } else { 1.0 };
        // Haxe exhaustionFaktor currently always 1
        // food drain uses ExhaustionHealingFactor only (not male mult)
        extra += original * exh_f;
        exhaustion_after -= healing * exh_f * male;
    }

    // C-SS-TEMP-HEAL: temperature hits/exhaustion after exhaustion heal, before hits heal.
    // Haxe: TimeHelper.updateFoodAndDoHealing L901–910
    let (hits_delta, exh_delta) = temperature_damage_extras_ex(
        age,
        temp_biome.is_super_hot,
        temp_biome.is_super_cold,
        temp_biome.heat,
        original,
        temp_biome.temperature_hits_damage_factor,
        temp_biome.temperature_exhaustion_damage_factor,
    );
    hits_after += hits_delta;
    exhaustion_after += exh_delta;

    // C-SS-TEMP-HEAL: biome-love exhaustion recovery (not doHealing-gated).
    // Haxe: TimeHelper.updateFoodAndDoHealing L915–918
    // food_store_max probe after temp mutations for the floor check
    let food_max_after_temp = food_store_max_from_parts_ex(
        age,
        food_store,
        hits_after,
        exhaustion_after,
        health_factor,
        capacity,
    );
    exhaustion_after = biome_love_exhaustion_heal(
        exhaustion_after,
        food_max_after_temp,
        healing,
        temp_biome.biome_love_factor,
    );

    if do_healing && healing > 0.0 && hits_after > 0.0 {
        // Haxe: hits -= healing * WoundHealingFactor; foodDecay += original * WoundHealingFactor
        // C-SS-WOUND-HEAL
        hits_after = (hits_after - healing * wh_f).max(0.0);
        extra += original * wh_f;
    }

    let food_store_max = food_store_max_from_parts_ex(
        age,
        food_store,
        hits_after,
        exhaustion_after,
        health_factor,
        capacity,
    );
    HealingFoodPipeStep {
        exhaustion_after,
        hits_after,
        extra_food_drain: extra,
        food_store_max,
        deadly: food_max_is_deadly(food_store_max),
    }
}

/// Wound bleed extras: hits += bleed, foodDecay += 2×bleed (Haxe).
///
/// `bleed_per_sec` is `wound.objectData.damage` (or combat stack proxy).
/// Factor applied here is [`WOUND_DAMAGE_FACTOR`].
// Haxe: TimeHelper.updateFoodAndDoHealing bleedingDamage
#[inline]
pub fn wound_bleed_food_extras(bleed_per_sec: f32, dt: f32) -> (f32, f32) {
    wound_bleed_food_extras_ex(bleed_per_sec, dt, WOUND_DAMAGE_FACTOR)
}

/// Live-knob variant of [`wound_bleed_food_extras`].
// Haxe: ServerSettings.WoundDamageFactor
// C-SS-MORE-KNOBS
#[inline]
pub fn wound_bleed_food_extras_ex(
    bleed_per_sec: f32,
    dt: f32,
    wound_damage_factor: f32,
) -> (f32, f32) {
    let b = if bleed_per_sec.is_finite() {
        bleed_per_sec.max(0.0)
    } else {
        0.0
    };
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let wdf = if wound_damage_factor.is_finite() && wound_damage_factor >= 0.0 {
        wound_damage_factor
    } else {
        WOUND_DAMAGE_FACTOR
    };
    let bleed = b * dt * wdf;
    (bleed, 2.0 * bleed) // (hits_delta, food_drain_delta)
}

/// Yellow fever food drain: `ExhaustionYellowFeverPerSec * 2 * dt`.
// Haxe: TimeHelper L447 food_store -= time * ExhaustionYellowFeverPerSec * 2
#[inline]
pub fn yellow_fever_food_drain(dt: f32) -> f32 {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    dt * EXHAUSTION_YELLOW_FEVER_PER_SEC * 2.0
}

/// Yellow fever heat rise; held-by halves hardship (`isHeldFaktor` 0.2).
// Haxe: TimeHelper L445–452 heat += time * 0.02 * isHeldFaktor
#[inline]
pub fn yellow_fever_heat_delta(dt: f32, is_held_by: bool) -> f32 {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let held_f = if is_held_by {
        YELLOW_FEVER_HELD_BY_FACTOR
    } else {
        1.0
    };
    dt * YELLOW_FEVER_HEAT_PER_SEC * held_f
}

/// Extra drain/sec from active exhaustion healing (for `?DRAIN` estimates).
// Haxe: updateFoodAndDoHealing exhaustionFoodNeed rate
#[inline]
pub fn exhaustion_heal_drain_rate(original_food_use_per_sec: f32, do_healing: bool) -> f32 {
    exhaustion_heal_drain_rate_ex(
        original_food_use_per_sec,
        do_healing,
        EXHAUSTION_HEALING_FACTOR,
    )
}

/// Live-factor variant of [`exhaustion_heal_drain_rate`].
// Haxe: ServerSettings.ExhaustionHealingFactor
// C-SS-MORE-KNOBS
#[inline]
pub fn exhaustion_heal_drain_rate_ex(
    original_food_use_per_sec: f32,
    do_healing: bool,
    exhaustion_healing_factor: f32,
) -> f32 {
    if do_healing {
        let f = if exhaustion_healing_factor.is_finite() && exhaustion_healing_factor >= 0.0 {
            exhaustion_healing_factor
        } else {
            EXHAUSTION_HEALING_FACTOR
        };
        original_food_use_per_sec.max(0.0) * f
    } else {
        0.0
    }
}

/// Extra drain/sec from active wound-hit healing.
// Haxe: updateFoodAndDoHealing WoundHealingFactor foodDecay
#[inline]
pub fn wound_heal_drain_rate(original_food_use_per_sec: f32, do_healing: bool, hits: f32) -> f32 {
    wound_heal_drain_rate_ex(
        original_food_use_per_sec,
        do_healing,
        hits,
        WOUND_HEALING_FACTOR,
    )
}

/// Live-factor variant of [`wound_heal_drain_rate`].
// Haxe: ServerSettings.WoundHealingFactor
// C-SS-WOUND-HEAL
#[inline]
pub fn wound_heal_drain_rate_ex(
    original_food_use_per_sec: f32,
    do_healing: bool,
    hits: f32,
    wound_healing_factor: f32,
) -> f32 {
    if do_healing && hits > 0.0 {
        let f = if wound_healing_factor.is_finite() && wound_healing_factor >= 0.0 {
            wound_healing_factor
        } else {
            WOUND_HEALING_FACTOR
        };
        original_food_use_per_sec.max(0.0) * f
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_reduced_is_grown_up() {
        assert!((calculate_not_reduced_food_store_max() - 20.0).abs() < 1e-6);
        // C-SS-TAIL-KNOBS live grown-up
        assert!((calculate_not_reduced_food_store_max_ex(25.0) - 25.0).abs() < 1e-6);
    }

    #[test]
    fn adult_base_health_one() {
        let m = food_store_max_from_parts(30.0, 10.0, 0.0, 0.0, 1.0);
        assert!((m - 20.0).abs() < 1e-4);
        // live GrownUpFoodStoreMax 25 → adult base 25
        let m2 = food_store_max_from_parts_ex(
            30.0,
            10.0,
            0.0,
            0.0,
            1.0,
            FoodStoreMaxKnobs::with_grown_up(25.0),
        );
        assert!((m2 - 25.0).abs() < 1e-4);
    }

    #[test]
    fn youth_age_band() {
        // age 0 → NewBorn 4
        assert!((food_store_max_from_parts(0.0, 4.0, 0.0, 0.0, 1.0) - 4.0).abs() < 1e-4);
        // age 10 → 4 + 10/20 * 16 = 12
        assert!((food_store_max_from_parts(10.0, 10.0, 0.0, 0.0, 1.0) - 12.0).abs() < 1e-4);
        // age 20 still adult (condition age < 20)
        assert!((food_store_max_from_parts(20.0, 10.0, 0.0, 0.0, 1.0) - 20.0).abs() < 1e-4);
    }

    #[test]
    fn old_age_band() {
        // age 50 → still adult (old starts > 50)
        assert!((food_store_max_from_parts(50.0, 10.0, 0.0, 0.0, 1.0) - 20.0).abs() < 1e-4);
        // age 55 → 10 + (60-55)/10 * 10 = 15
        assert!((food_store_max_from_parts(55.0, 10.0, 0.0, 0.0, 1.0) - 15.0).abs() < 1e-4);
        // age 60 → 10
        assert!((food_store_max_from_parts(60.0, 10.0, 0.0, 0.0, 1.0) - 10.0).abs() < 1e-4);
    }

    // --- C-SS-AGE-FOOD live NewBorn / OldAge bands ---

    #[test]
    fn live_newborn_band_overrides_module() {
        // newborn 6, grown 20: age 0 → 6; age 10 → 6 + 10/20*(20-6) = 13
        let knobs = FoodStoreMaxKnobs {
            grown_up: 20.0,
            newborn: 6.0,
            old_age: 10.0,
        };
        assert!((food_store_max_from_parts_ex(0.0, 4.0, 0.0, 0.0, 1.0, knobs) - 6.0).abs() < 1e-4);
        assert!((food_store_max_from_parts_ex(10.0, 10.0, 0.0, 0.0, 1.0, knobs) - 13.0).abs() < 1e-4);
    }

    #[test]
    fn live_old_age_band_overrides_module() {
        // old 12, grown 20: age 55 → 12 + 5/10*(20-12) = 16; age 60 → 12
        let knobs = FoodStoreMaxKnobs {
            grown_up: 20.0,
            newborn: 4.0,
            old_age: 12.0,
        };
        assert!((food_store_max_from_parts_ex(55.0, 10.0, 0.0, 0.0, 1.0, knobs) - 16.0).abs() < 1e-4);
        assert!((food_store_max_from_parts_ex(60.0, 10.0, 0.0, 0.0, 1.0, knobs) - 12.0).abs() < 1e-4);
    }

    #[test]
    fn live_combined_grown_newborn_old() {
        // grown 25, newborn 5, old 8
        let knobs = FoodStoreMaxKnobs {
            grown_up: 25.0,
            newborn: 5.0,
            old_age: 8.0,
        };
        // youth mid: 5 + 10/20*(25-5) = 15
        assert!((food_store_max_from_parts_ex(10.0, 10.0, 0.0, 0.0, 1.0, knobs) - 15.0).abs() < 1e-4);
        // adult ignores newborn/old
        assert!((food_store_max_from_parts_ex(30.0, 10.0, 0.0, 0.0, 1.0, knobs) - 25.0).abs() < 1e-4);
        // old mid: 8 + 5/10*(25-8) = 16.5
        assert!((food_store_max_from_parts_ex(55.0, 10.0, 0.0, 0.0, 1.0, knobs) - 16.5).abs() < 1e-4);
    }

    #[test]
    fn adult_ignores_live_newborn_and_old_age() {
        let knobs = FoodStoreMaxKnobs {
            grown_up: 20.0,
            newborn: 1.0,
            old_age: 1.0,
        };
        assert!((food_store_max_from_parts_ex(30.0, 10.0, 0.0, 0.0, 1.0, knobs) - 20.0).abs() < 1e-4);
        assert!((food_store_max_from_parts_ex(50.0, 10.0, 0.0, 0.0, 1.0, knobs) - 20.0).abs() < 1e-4);
    }

    #[test]
    fn health_factor_scales_adult_only() {
        let hi = food_store_max_from_parts(30.0, 10.0, 0.0, 0.0, 1.2);
        assert!((hi - 24.0).abs() < 1e-4);
        // youth overwrites health
        let youth = food_store_max_from_parts(10.0, 10.0, 0.0, 0.0, 1.2);
        assert!((youth - 12.0).abs() < 1e-4);
    }

    #[test]
    fn hits_subtract() {
        let m = food_store_max_from_parts(30.0, 10.0, 5.0, 0.0, 1.0);
        assert!((m - 15.0).abs() < 1e-4);
    }

    #[test]
    fn exhaustion_half_floor() {
        // base 20, hits 0, exhaustion 15 → 20-15=5, floor 10 → 10
        let m = food_store_max_from_parts(30.0, 10.0, 0.0, 15.0, 1.0);
        assert!((m - 10.0).abs() < 1e-4);
        // exhaustion 5 → 15 (no floor)
        let m2 = food_store_max_from_parts(30.0, 10.0, 0.0, 5.0, 1.0);
        assert!((m2 - 15.0).abs() < 1e-4);
        // negative exhaustion does not reduce
        let m3 = food_store_max_from_parts(30.0, 10.0, 0.0, -20.0, 1.0);
        assert!((m3 - 20.0).abs() < 1e-4);
    }

    #[test]
    fn starving_reduces_max() {
        // food_store -2 → max += 5 * -2 = -10 → 10
        let m = food_store_max_from_parts(30.0, -2.0, 0.0, 0.0, 1.0);
        assert!((m - 10.0).abs() < 1e-4);
    }

    #[test]
    fn cap_base_is_not_reduced() {
        // Callers must pass not-reduced (~20) into CombatState::cap_damage
        let not_red = calculate_not_reduced_food_store_max();
        let cap = not_red / 2.0 + 1.0;
        assert!((cap - 11.0).abs() < 1e-6);
    }

    #[test]
    fn apply_damage_raises_hits_and_exhaustion() {
        let r = apply_damage_food_pipe(30.0, 10.0, 0.0, 0.0, 4.0, 1.0, true);
        assert!((r.hits_after - 4.0).abs() < 1e-5);
        assert!((r.exhaustion_after - 4.0).abs() < 1e-5);
        // 20 - 4 hits - 4 exh = 12
        assert!((r.food_store_max - 12.0).abs() < 1e-4);
        assert!(!r.combat_lethal);
    }

    #[test]
    fn heavy_damage_combat_lethal() {
        // Cap-style big hit: 11 dmg → hits 11, exh 11 → pre-exh 9, −11 floor 4.5
        let r = apply_damage_food_pipe(30.0, 10.0, 0.0, 0.0, 11.0, 1.0, true);
        assert!((r.food_store_max - 4.5).abs() < 1e-4);
        assert!(!r.combat_lethal);
        // Stack enough hits+exh to go negative before half-floor on low base
        // Start with hits 15, dmg 10 → hits 25, exh 10; base 20-25=-5, −10 floor -2.5 → lethal
        let r2 = apply_damage_food_pipe(30.0, 10.0, 15.0, 0.0, 10.0, 1.0, true);
        assert!(r2.food_store_max < 0.0);
        assert!(r2.combat_lethal);
    }

    #[test]
    fn heal_exhaustion_increases_drain() {
        let step = step_healing_food_pipe(
            30.0, 10.0, 0.0, 5.0, 1.0, 1.0, 0.10, true, false,
        );
        assert!(step.exhaustion_after < 5.0);
        assert!((step.extra_food_drain - 0.10 * EXHAUSTION_HEALING_FACTOR).abs() < 1e-5);
        // food_max rises as exhaustion falls
        assert!(step.food_store_max > food_store_max_from_parts(30.0, 10.0, 0.0, 5.0, 1.0) - 0.01);
    }

    /// C-SS-MORE-KNOBS: live ExhaustionHealingFactor scales drain + exhaustion delta.
    // Haxe: ServerSettings.ExhaustionHealingFactor
    #[test]
    fn heal_exhaustion_live_factor_override() {
        let base = step_healing_food_pipe(
            30.0, 10.0, 0.0, 5.0, 1.0, 1.0, 0.10, true, false,
        );
        let double = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            3.0, // 2× default 1.5
            WOUND_HEALING_FACTOR,
            EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            TempBiomeHealKnobs::default(),
        );
        assert!((double.extra_food_drain - 0.10 * 3.0).abs() < 1e-5);
        // Larger factor → more exhaustion recovered (lower after)
        assert!(double.exhaustion_after < base.exhaustion_after);
        // Male multiplies exhaustion heal only (not food drain)
        let male = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            true,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            1.5,
            WOUND_HEALING_FACTOR,
            EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            TempBiomeHealKnobs::default(),
        );
        let female = step_healing_food_pipe(
            30.0, 10.0, 0.0, 5.0, 1.0, 1.0, 0.10, true, false,
        );
        assert!((male.extra_food_drain - female.extra_food_drain).abs() < 1e-5);
        assert!(male.exhaustion_after < female.exhaustion_after);
    }

    /// C-SS-MALE-HEAL: live ExhaustionHealingForMaleFaktor scales male recovery only (not food drain).
    // Haxe: ServerSettings.ExhaustionHealingForMaleFaktor
    #[test]
    fn heal_exhaustion_live_male_factor_override() {
        let male_default = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            true,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            1.2, // default male factor
            TempBiomeHealKnobs::default(),
        );
        let male_double = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            true,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            2.4, // 2× default 1.2
            TempBiomeHealKnobs::default(),
        );
        // Food drain ignores male factor
        assert!((male_default.extra_food_drain - male_double.extra_food_drain).abs() < 1e-5);
        assert!((male_default.extra_food_drain - 0.10 * EXHAUSTION_HEALING_FACTOR).abs() < 1e-5);
        // Higher male factor → more exhaustion recovered
        assert!(male_double.exhaustion_after < male_default.exhaustion_after);
        // Female path ignores male factor (healingFaktor = 1)
        let female_1 = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            1.2,
            TempBiomeHealKnobs::default(),
        );
        let female_2 = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            true,
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            2.4,
            TempBiomeHealKnobs::default(),
        );
        assert!((female_1.exhaustion_after - female_2.exhaustion_after).abs() < 1e-5);
        assert!((female_1.extra_food_drain - female_2.extra_food_drain).abs() < 1e-5);
    }

    /// C-SS-MORE-KNOBS: WoundDamageFactor doubles hits + 2× food drain.
    // Haxe: ServerSettings.WoundDamageFactor
    #[test]
    fn wound_bleed_live_factor_override() {
        let (h, f) = wound_bleed_food_extras_ex(0.05, 1.0, 2.0);
        assert!((h - 0.10).abs() < 1e-6);
        assert!((f - 0.20).abs() < 1e-6);
        let (h0, f0) = wound_bleed_food_extras_ex(0.05, 1.0, 0.0);
        assert_eq!(h0, 0.0);
        assert_eq!(f0, 0.0);
    }

    #[test]
    fn heal_hits_restores_max() {
        // exhaustion already at floor (-food_max) so only wound-hit heal extras apply
        let before = food_store_max_from_parts(30.0, 10.0, 4.0, -20.0, 1.0);
        let step = step_healing_food_pipe(
            30.0, 10.0, 4.0, -20.0, 1.0, 1.0, 0.10, true, false,
        );
        assert!(step.hits_after < 4.0);
        assert!(step.food_store_max > before);
        assert!((step.extra_food_drain - 0.10 * WOUND_HEALING_FACTOR).abs() < 1e-5);
    }

    /// C-SS-WOUND-HEAL: live WoundHealingFactor scales hits heal + food drain.
    // Haxe: ServerSettings.WoundHealingFactor
    #[test]
    fn heal_hits_live_wound_healing_factor_override() {
        let base = step_healing_food_pipe(
            30.0, 10.0, 4.0, -20.0, 1.0, 1.0, 0.10, true, false,
        );
        let double = step_healing_food_pipe_ex(
            30.0,
            10.0,
            4.0,
            -20.0,
            1.0,
            1.0,
            0.10,
            true,
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            2.0, // 2× default 1.0
            EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            TempBiomeHealKnobs::default(),
        );
        assert!((double.extra_food_drain - 0.10 * 2.0).abs() < 1e-5);
        // Larger factor → more hits healed (lower after)
        assert!(double.hits_after < base.hits_after);
        // Drain rate helper
        assert!((wound_heal_drain_rate_ex(0.10, true, 2.0, 2.0) - 0.20).abs() < 1e-6);
        assert_eq!(wound_heal_drain_rate_ex(0.10, true, 0.0, 2.0), 0.0);
        assert_eq!(wound_heal_drain_rate_ex(0.10, false, 2.0, 2.0), 0.0);
    }

    #[test]
    fn no_heal_when_wounded_gate() {
        assert!(!can_do_healing(10.0, true, false, 10.0, false));
        assert!(can_do_healing(10.0, false, false, 10.0, false));
        assert!(!can_do_healing(10.0, false, false, 0.0, false));
        assert!(!can_do_healing(-1.0, false, false, 10.0, false));
        // C-SS-TEMP-HEAL: super_heat blocks heal
        assert!(!can_do_healing(10.0, false, false, 10.0, true));
    }

    /// C-SS-TEMP-HEAL: age≤1 no temp damage; super-hot/cold 1× and 2× extremes.
    // Haxe: TimeHelper.updateFoodAndDoHealing L901–910
    #[test]
    fn temperature_damage_extras_age_and_extremes() {
        let orig = 0.10;
        // age ≤ 1 → no damage
        let (h0, e0) = temperature_damage_extras(1.0, true, false, 0.85, orig);
        assert_eq!(h0, 0.0);
        assert_eq!(e0, 0.0);
        // super-hot heat=0.85 → 1×
        let (h1, e1) = temperature_damage_extras(2.0, true, false, 0.85, orig);
        assert!((h1 - orig * TEMPERATURE_HITS_DAMAGE_FACTOR).abs() < 1e-6);
        assert!((e1 - orig * TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR).abs() < 1e-6);
        // heat=0.96 → double damage
        let (h2, e2) = temperature_damage_extras(2.0, true, false, 0.96, orig);
        assert!((h2 - 2.0 * orig * TEMPERATURE_HITS_DAMAGE_FACTOR).abs() < 1e-6);
        assert!((e2 - 2.0 * orig * TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR).abs() < 1e-6);
        // super-cold heat=0.15 → 1×
        let (hc, ec) = temperature_damage_extras(2.0, false, true, 0.15, orig);
        assert!((hc - orig * TEMPERATURE_HITS_DAMAGE_FACTOR).abs() < 1e-6);
        assert!((ec - orig * TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR).abs() < 1e-6);
        // heat=0.04 → 2×
        let (hc2, ec2) = temperature_damage_extras(2.0, false, true, 0.04, orig);
        assert!((hc2 - 2.0 * orig * TEMPERATURE_HITS_DAMAGE_FACTOR).abs() < 1e-6);
        assert!((ec2 - 2.0 * orig * TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR).abs() < 1e-6);
        // factor overrides
        let (ho, eo) = temperature_damage_extras_ex(2.0, true, false, 0.85, orig, 1.0, 0.4);
        assert!((ho - orig * 1.0).abs() < 1e-6);
        assert!((eo - orig * 0.4).abs() < 1e-6);
        // neither hot nor cold
        let (hn, en) = temperature_damage_extras(30.0, false, false, 0.5, orig);
        assert_eq!(hn, 0.0);
        assert_eq!(en, 0.0);
    }

    /// C-SS-TEMP-HEAL: biome-love exhaustion recovery + love cap + floor.
    // Haxe: TimeHelper.updateFoodAndDoHealing L915–918
    #[test]
    fn biome_love_exhaustion_heal_love_and_floor() {
        let healing = 0.10; // 1s * HealingPerSecond
        let food_max = 20.0;
        // love=1 → exh drops healing*0.5
        let e1 = biome_love_exhaustion_heal(5.0, food_max, healing, 1.0);
        assert!((e1 - (5.0 - healing * 0.5)).abs() < 1e-6);
        // love=2 clamps to 1
        let e2 = biome_love_exhaustion_heal(5.0, food_max, healing, 2.0);
        assert!((e2 - e1).abs() < 1e-6);
        // love<=0 no change
        assert!((biome_love_exhaustion_heal(5.0, food_max, healing, 0.0) - 5.0).abs() < 1e-6);
        assert!((biome_love_exhaustion_heal(5.0, food_max, healing, -1.0) - 5.0).abs() < 1e-6);
        // floor: exhaustion already at -food_max → no heal
        let floor = biome_love_exhaustion_heal(-food_max, food_max, healing, 1.0);
        assert!((floor - (-food_max)).abs() < 1e-6);
    }

    /// C-SS-TEMP-HEAL: brown self + jungle parents love scores; swamp floor penalty.
    // Haxe: GlobalPlayerInstance.biomeLoveFactor / BiomeLoveFactorForColor
    #[test]
    fn biome_love_factor_brown_jungle_and_swamp_floor() {
        // PersonColor.Brown = 3, JUNGLE = 6
        let brown = 3;
        // brown self + both jungle parents → 1 + 0.5 + 0.5 = 2
        let love = biome_love_factor(
            BIOME_TAG_JUNGLE,
            0,
            brown,
            Some(brown),
            Some(brown),
        );
        assert!((love - 2.0).abs() < 1e-6);
        // brown self alone in jungle → 1
        assert!((biome_love_factor(BIOME_TAG_JUNGLE, 0, brown, None, None) - 1.0).abs() < 1e-6);
        // unloved non-green/yellow self → −0.5
        assert!((biome_love_factor(BIOME_TAG_GREY, 0, brown, None, None) - (-0.5)).abs() < 1e-6);
        // swamp with floor self → −0.5 − 2.5 = −3.0
        assert!(
            (biome_love_factor(BIOME_TAG_SWAMP, 100, brown, None, None) - (-3.0)).abs() < 1e-6
        );
        // swamp no floor → only −0.5
        assert!((biome_love_factor(BIOME_TAG_SWAMP, 0, brown, None, None) - (-0.5)).abs() < 1e-6);
        // green unloved stays 0
        assert!((biome_love_factor(BIOME_TAG_GREEN, 0, brown, None, None) - 0.0).abs() < 1e-6);
        // black loves desert
        assert!(is_biome_loved_by_color(BIOME_TAG_DESERT, 1));
        assert!(!is_biome_loved_by_color(BIOME_TAG_JUNGLE, 1));
    }

    /// C-SS-TEMP-HEAL: pipe applies temp hits/exh + biome love recovery.
    // Haxe: TimeHelper.updateFoodAndDoHealing L901–918 order
    #[test]
    fn step_healing_applies_temp_damage_and_biome_love() {
        // Super-hot: no do_healing effects if we gate externally; here do_healing=false
        // so only temp + biome love apply.
        let tb = TempBiomeHealKnobs {
            is_super_hot: true,
            is_super_cold: false,
            heat: 0.85,
            temperature_hits_damage_factor: TEMPERATURE_HITS_DAMAGE_FACTOR,
            temperature_exhaustion_damage_factor: TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
            biome_love_factor: 1.0,
        };
        let step = step_healing_food_pipe_ex(
            30.0,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            false, // no do_healing (exhaustion/hits heal off)
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            tb,
        );
        // hits += orig * 0.5
        assert!((step.hits_after - 0.10 * 0.5).abs() < 1e-5);
        // exh: 5 + orig*0.2 − healing*0.5*love = 5 + 0.02 − 0.05 = 4.97
        let expect_exh = 5.0 + 0.10 * TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR
            - HEALING_PER_SECOND * 0.5 * 1.0;
        assert!((step.exhaustion_after - expect_exh).abs() < 1e-5);
        // age ≤ 1 → no temp damage (biome love still applies)
        let tb_bb = TempBiomeHealKnobs {
            is_super_hot: true,
            heat: 0.96,
            biome_love_factor: 0.0,
            ..TempBiomeHealKnobs::default()
        };
        let bb = step_healing_food_pipe_ex(
            0.5,
            10.0,
            0.0,
            5.0,
            1.0,
            1.0,
            0.10,
            false,
            false,
            HEALING_PER_SECOND,
            FoodStoreMaxKnobs::default(),
            EXHAUSTION_HEALING_FACTOR,
            WOUND_HEALING_FACTOR,
            EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            tb_bb,
        );
        assert!((bb.hits_after - 0.0).abs() < 1e-6);
        assert!((bb.exhaustion_after - 5.0).abs() < 1e-6);
    }

    #[test]
    fn spawn_credit_negative_max() {
        assert!((spawn_exhaustion_credit(20.0) - (-20.0)).abs() < 1e-6);
    }

    #[test]
    fn wound_bleed_doubles_food() {
        let (h, f) = wound_bleed_food_extras(0.05, 1.0);
        assert!((h - 0.05).abs() < 1e-6);
        assert!((f - 0.10).abs() < 1e-6);
        // table rates 0.05 / 0.06 / 0.1
        let (h6, f6) = wound_bleed_food_extras(0.06, 1.0);
        assert!((h6 - 0.06).abs() < 1e-6);
        assert!((f6 - 0.12).abs() < 1e-6);
        let (h1, f1) = wound_bleed_food_extras(0.1, 1.0);
        assert!((h1 - 0.1).abs() < 1e-6);
        assert!((f1 - 0.2).abs() < 1e-6);
    }

    #[test]
    fn yellow_fever_food_and_heat_vitals() {
        // 0.1 * 2 = 0.2 food/sec
        assert!((yellow_fever_food_drain(1.0) - 0.2).abs() < 1e-6);
        assert!((yellow_fever_food_drain(0.5) - 0.1).abs() < 1e-6);
        // heat 0.02/s alone; held-by → 0.004/s
        assert!((yellow_fever_heat_delta(1.0, false) - 0.02).abs() < 1e-6);
        assert!((yellow_fever_heat_delta(1.0, true) - 0.004).abs() < 1e-6);
        assert_eq!(yellow_fever_food_drain(0.0), 0.0);
    }

    #[test]
    fn health_factor_neutral_at_zero_yum() {
        // health = 0 - median*(trueAge/(max/2)); median 0 → health 0 → factor 1 via denom edge
        let f = calculate_health_food_store_max_factor(0.0, 0.0, 20.0);
        assert!((f - 1.0).abs() < 1e-4);
    }

    #[test]
    fn death_threshold() {
        assert!(!food_max_is_deadly(0.0));
        assert!(!food_max_is_deadly(-0.1));
        assert!(food_max_is_deadly(-0.11));
        assert!(food_max_is_combat_deadly(-0.01));
        assert!(!food_max_is_combat_deadly(0.0));
    }

    // --- HEALTH-AGE-FOOD / health_food_max ---

    #[test]
    fn median_prestige_floors_at_thirty() {
        assert!((median_prestige_for_health(0.0) - 30.0).abs() < 1e-5);
        assert!((median_prestige_for_health(10.0) - 30.0).abs() < 1e-5);
        assert!((median_prestige_for_health(45.0) - 45.0).abs() < 1e-5);
    }

    #[test]
    fn health_food_max_factor_high_yum_above_one() {
        // health = 60 - 30*(0/30) = 60 → factor = (1.2*60+30)/(60+30) = 102/90 = 1.133…
        let f = calculate_health_food_store_max_factor(60.0, 30.0, 0.0);
        assert!((f - 1.133333).abs() < 1e-3);
        let max = food_store_max_from_parts(30.0, 10.0, 0.0, 0.0, f);
        assert!(max > 22.0 && max < 24.0);
    }

    #[test]
    fn health_age_factor_high_yum_above_one() {
        // same health 60 → (2*60+30)/(60+30) = 150/90 = 1.666…
        let f = calculate_health_age_factor(60.0, 30.0, 0.0);
        assert!((f - 1.666666).abs() < 1e-3);
    }

    #[test]
    fn health_age_factor_low_yum_below_one() {
        // health = 0 - 30*(30/30) = -30 → mali branch with min 0.5
        // (health - median) / ((1/0.5)*health - median) = (-30-30)/(2*(-30)-30) = (-60)/(-90) = 0.666…
        let f = calculate_health_age_factor(0.0, 30.0, 30.0);
        assert!(f > 0.5 && f < 1.0);
        assert!((f - 0.666666).abs() < 1e-3);
    }

    #[test]
    fn age_step_true_age_constant_adult_health() {
        // 60s → 1 true year; adult health_age=2 → ageingFactor=0.5 → display +0.5
        let s = age_step_from_health(60.0, 20.0, 10.0, 2.0, 1.0);
        assert!((s.true_age_delta - 1.0).abs() < 1e-5);
        assert!((s.ageing_factor - 0.5).abs() < 1e-5);
        assert!((s.age_delta - 0.5).abs() < 1e-5);
        // age_r = 60 / 0.5 = 120
        assert!((s.age_r - 120.0).abs() < 1e-4);
    }

    #[test]
    fn age_step_youth_ignores_health_factor() {
        // Haxe youth branch does not apply health (commented out)
        let s = age_step_from_health(60.0, 10.0, 5.0, 2.0, 1.0);
        assert!((s.ageing_factor - 1.0).abs() < 1e-5);
        assert!((s.age_delta - 1.0).abs() < 1e-5);
        assert!((s.age_r - 60.0).abs() < 1e-4);
    }

    #[test]
    fn age_step_starving_youth_slows() {
        let s = age_step_from_health(60.0, 10.0, -1.0, 1.0, 1.0);
        assert!((s.ageing_factor - 0.5).abs() < 1e-5);
        assert!((s.age_delta - 0.5).abs() < 1e-5);
    }

    #[test]
    fn age_step_starving_adult_speeds() {
        // adult health=1 → factor 1; starve → ×2
        let s = age_step_from_health(60.0, 20.0, -1.0, 1.0, 1.0);
        assert!((s.ageing_factor - 2.0).abs() < 1e-5);
        assert!((s.age_delta - 2.0).abs() < 1e-5);
    }

    #[test]
    fn birth_mult_human_infant_ai_mother() {
        // age < 3, human + AI mother → ×3
        assert!(
            (birth_cross_species_aging_mult(1.0, true, Some(true))
                - AGING_FACTOR_HUMAN_BORN_TO_AI)
                .abs()
                < 1e-6
        );
        // age >= 3 → 1
        assert!((birth_cross_species_aging_mult(3.0, true, Some(true)) - 1.0).abs() < 1e-6);
        // human + human mother → 1
        assert!((birth_cross_species_aging_mult(1.0, true, Some(false)) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn birth_mult_ai_infant_human_mother() {
        assert!(
            (birth_cross_species_aging_mult(0.5, false, Some(false))
                - AGING_FACTOR_AI_BORN_TO_HUMAN)
                .abs()
                < 1e-6
        );
        // AI + AI mother → 1
        assert!((birth_cross_species_aging_mult(0.5, false, Some(true)) - 1.0).abs() < 1e-6);
        // no mother
        assert!((birth_cross_species_aging_mult(0.5, false, None) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn age_step_applies_birth_mult_before_starve() {
        // youth human×AI → ×3; not starving
        let s = age_step_from_health(60.0, 1.0, 5.0, 1.0, 3.0);
        assert!((s.ageing_factor - 3.0).abs() < 1e-5);
        assert!((s.age_delta - 3.0).abs() < 1e-5);
        assert!((s.age_r - 20.0).abs() < 1e-4); // 60/3
        // youth ×3 then starve ×0.5 → 1.5
        let s2 = age_step_from_health(60.0, 1.0, -1.0, 1.0, 3.0);
        assert!((s2.ageing_factor - 1.5).abs() < 1e-5);
    }

    #[test]
    fn birth_yum_from_score_and_true_age_floor() {
        // score 100 → 40; floor (30/30)*14 = 14 → 40
        let y = birth_yum_multiplier(100.0, 30.0, 14.0);
        assert!((y - 40.0).abs() < 1e-4);
        // score 0 → floor trueAge when median 30
        let y2 = birth_yum_multiplier(0.0, 30.0, 14.0);
        assert!((y2 - 14.0).abs() < 1e-4);
        // newborn trueAge ~0
        let y3 = birth_yum_multiplier(0.0, 30.0, 0.01);
        assert!((y3 - 0.01).abs() < 1e-5);
    }

    #[test]
    fn birth_floor_cancels_age_health_penalty() {
        // Haxe intent: birth floor ≈ median*(trueAge/30) cancels age term in CalculateHealthFactor
        let yum = birth_yum_multiplier(0.0, 30.0, 14.0);
        let f = calculate_health_food_store_max_factor(yum, 30.0, 14.0);
        assert!((f - 1.0).abs() < 1e-3, "neutral health at birth floor, got {f}");
    }

    #[test]
    fn birth_cross_species_aging_mult_ex_live() {
        // C-SS-MIN-AGE-AI: age 4 with min 5 still in window; with default 3 is adult window
        assert_eq!(
            birth_cross_species_aging_mult_ex(4.0, true, Some(true), 5.0),
            AGING_FACTOR_HUMAN_BORN_TO_AI
        );
        assert_eq!(
            birth_cross_species_aging_mult_ex(4.0, true, Some(true), 3.0),
            1.0
        );
    }
}
