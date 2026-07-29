//! Live server settings apply (CONFIG-SETTINGS / Haxe ServerSettings hot-reload).
//!
//! Haxe: `ServerSettings.readFromFile` every ~200 ticks when
//! `TimeHelper.ReadServerSettings` is true. Boot-only knobs stay out of scope.
//!
//! File I/O lives in `ol-config::HotReloadTracker`; this module only mutates
//! [`crate::SimState`] from a [`ol_config::LiveSettings`] snapshot.
//!
//! SETTINGS-FIELD-MAP: critical gameplay knobs (`FoodUsePerSecond`, move speed,
//! yum, animal chances, …) land on [`GameplayKnobs`] via [`apply_live_settings`].

use crate::environment::Season;
use crate::war_posse_persist::WarPosseShare;
use crate::world_food_stats::WorldFoodShare;
use crate::object_counts_share::ObjectCountsShare;
use crate::SimState;
use ol_config::{gameplay_defaults, HotReloadTracker, LiveSettings, HAXE_YEAR_SECS};
use std::sync::{Arc, RwLock};

/// Live-applied Haxe `ServerSettings` gameplay statics (SETTINGS-FIELD-MAP batch).
///
/// Defaults match Haxe; hot-reload overwrites from `server.toml` via LiveSettings.
// Haxe: ServerSettings.FoodUsePerSecond / HealingPerSecond / InitialPlayerMoveSpeed / …
#[derive(Debug, Clone, PartialEq)]
pub struct GameplayKnobs {
    pub food_use_per_second: f32,
    pub healing_per_second: f32,
    pub ageing_seconds_per_year: f32,
    pub initial_player_move_speed: f32,
    pub speed_factor: f32,
    pub yum_bonus: f32,
    pub chance_for_offspring: f32,
    pub chance_for_animal_dying: f32,
    pub hungry_work_cost: f32,
    pub birth_prestige_factor: f32,
    /// Haxe `AllyStrenghTooLowForPickup` (0 = gate off).
    pub ally_strength_too_low_for_pickup: f32,
    /// Haxe `TimeConfirmNewFollower` — delayed I FOLLOW confirm seconds.
    // Haxe: ServerSettings.TimeConfirmNewFollower
    // FOLLOW-HIRE-DELAY
    pub time_confirm_new_follower: f32,
    /// Haxe `HireCost` base coins (I HIRE immediate; not delayed).
    // Haxe: ServerSettings.HireCost
    pub hire_cost: f32,
    /// Haxe `HireCostIncreasePerPerson`.
    // Haxe: ServerSettings.HireCostIncreasePerPerson
    pub hire_cost_increase_per_person: f32,
    /// Haxe `AutoFollowPlayer` — AI acquire closest human when sticky empty.
    // Haxe: ServerSettings.AutoFollowPlayer = false
    // AI-FOLLOW-ACQUIRE
    pub auto_follow_player: bool,
    /// Haxe `PrestigeCostPerDamageForAlly` (illegal ally hit category cost).
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly
    // PRESTIGE-ALLY-COST
    pub prestige_cost_per_damage_for_ally: f32,
    /// Haxe `PrestigeCostPerDamageForChild`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForChild
    // C-SS-MORE
    pub prestige_cost_per_damage_for_child: f32,
    /// Haxe `PrestigeCostPerDamageForElderly`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForElderly
    // C-SS-MORE
    pub prestige_cost_per_damage_for_elderly: f32,
    /// Haxe `PrestigeCostPerDamageForCloseRelatives`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForCloseRelatives
    // C-SS-MORE
    pub prestige_cost_per_damage_for_close_relatives: f32,
    /// Haxe `PrestigeCostPerDamageForWomenWithoutWeapon`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForWomenWithoutWeapon
    // C-SS-MORE
    pub prestige_cost_per_damage_for_women_without_weapon: f32,
    // --- C-SS-FULL-TABLE / settings_long_tail ---
    /// Haxe `FoodFactor` — global fill scale.
    // Haxe: ServerSettings.FoodFactor
    pub food_factor: f32,
    /// Haxe `FoodFactorEatenMoreThanEightPercent`.
    pub food_factor_eaten_more_than_eight_percent: f32,
    /// Haxe `FoodFactorEatenMoreThanTenPercent`.
    pub food_factor_eaten_more_than_ten_percent: f32,
    /// Haxe `FoodFactorEatenLessThanFivePercent`.
    pub food_factor_eaten_less_than_five_percent: f32,
    /// Haxe `FoodFactorEatenLessThanThreePercent`.
    pub food_factor_eaten_less_than_three_percent: f32,
    /// Haxe `FoodFactorEatenLessThanOnePercent`.
    pub food_factor_eaten_less_than_one_percent: f32,
    /// Haxe `YumFoodRestore`.
    // Haxe: ServerSettings.YumFoodRestore
    pub yum_food_restore: f32,
    /// Haxe `LovedFoodRestore`.
    // Haxe: ServerSettings.LovedFoodRestore
    pub loved_food_restore: f32,
    /// Haxe `YumNewCravingChance`.
    // Haxe: ServerSettings.YumNewCravingChance
    pub yum_new_craving_chance: f32,
    /// Haxe `FoodReductionPerEating`.
    // Haxe: ServerSettings.FoodReductionPerEating
    pub food_reduction_per_eating: f32,
    /// Haxe `FoodReductionFaktorForEatingMeh`.
    // Haxe: ServerSettings.FoodReductionFaktorForEatingMeh
    pub food_reduction_faktor_for_eating_meh: f32,
    /// Haxe `HealthLostWhenEatingMeh`.
    // Haxe: ServerSettings.HealthLostWhenEatingMeh
    pub health_lost_when_eating_meh: f32,
    /// Haxe `HealthLostWhenEatingSuperMeh`.
    // Haxe: ServerSettings.HealthLostWhenEatingSuperMeh
    pub health_lost_when_eating_super_meh: f32,
    // --- C-SS-TAIL-KNOBS / settings_knobs ---
    /// Haxe `FoodReductionFaktorForEatingHighQuailitFood` (Haxe typo Quailit).
    // Haxe: ServerSettings.FoodReductionFaktorForEatingHighQuailitFood
    // C-SS-TAIL-KNOBS
    pub food_reduction_faktor_for_eating_high_quality: f32,
    /// Haxe `GrownUpFoodStoreMax`.
    // Haxe: ServerSettings.GrownUpFoodStoreMax
    // C-SS-TAIL-KNOBS
    pub grown_up_food_store_max: f32,
    /// Haxe `NewBornFoodStoreMax`.
    // Haxe: ServerSettings.NewBornFoodStoreMax
    // C-SS-AGE-FOOD
    pub new_born_food_store_max: f32,
    /// Haxe `OldAgeFoodStoreMax`.
    // Haxe: ServerSettings.OldAgeFoodStoreMax
    // C-SS-AGE-FOOD
    pub old_age_food_store_max: f32,
    /// Haxe `MinBiomeSpeedFactor`.
    // Haxe: ServerSettings.MinBiomeSpeedFactor
    // C-SS-TAIL-KNOBS
    pub min_biome_speed_factor: f32,
    /// Haxe `HitpointsSpeedFactor` (0 = disable).
    // Haxe: ServerSettings.HitpointsSpeedFactor
    // C-SS-TAIL-KNOBS
    pub hitpoints_speed_factor: f32,
    /// Haxe `CombatReputationRestorePerYear`.
    // Haxe: ServerSettings.CombatReputationRestorePerYear
    // C-SS-TAIL-KNOBS
    pub combat_reputation_restore_per_year: f32,
    // --- C-SS-MORE-KNOBS / settings_batch2 ---
    /// Haxe `ExhaustionHealingFactor`.
    // Haxe: ServerSettings.ExhaustionHealingFactor
    // C-SS-MORE-KNOBS
    pub exhaustion_healing_factor: f32,
    /// Haxe `WoundDamageFactor`.
    // Haxe: ServerSettings.WoundDamageFactor
    // C-SS-MORE-KNOBS
    pub wound_damage_factor: f32,
    /// Haxe `WoundHealingFactor`.
    // Haxe: ServerSettings.WoundHealingFactor
    // C-SS-WOUND-HEAL
    pub wound_healing_factor: f32,
    /// Haxe `ExhaustionHealingForMaleFaktor` (Haxe typo Faktor) — male exhaustion recovery mult only.
    // Haxe: ServerSettings.ExhaustionHealingForMaleFaktor
    // C-SS-MALE-HEAL
    pub exhaustion_healing_for_male_factor: f32,
    /// Haxe `TemperatureHitsDamageFactor` — super-hot/cold hits mult.
    // Haxe: ServerSettings.TemperatureHitsDamageFactor
    // C-SS-TEMP-HEAL
    pub temperature_hits_damage_factor: f32,
    /// Haxe `TemperatureExhaustionDamageFactor` — super-hot/cold exhaustion mult.
    // Haxe: ServerSettings.TemperatureExhaustionDamageFactor
    // C-SS-TEMP-HEAL
    pub temperature_exhaustion_damage_factor: f32,
    /// Haxe `MaxMovementQuadJumpDistanceBeforeForce` (squared distance).
    // Haxe: ServerSettings.MaxMovementQuadJumpDistanceBeforeForce
    // C-SS-MORE-KNOBS
    pub max_movement_quad_jump_distance_before_force: f32,
    /// Haxe `FoodRestoreFactorWhileFeeding`.
    // Haxe: ServerSettings.FoodRestoreFactorWhileFeeding
    // C-SS-MORE-KNOBS
    pub food_restore_factor_while_feeding: f32,
    /// Haxe `MaxHasEatenForNextGeneration`.
    // Haxe: ServerSettings.MaxHasEatenForNextGeneration
    // C-SS-MORE-KNOBS
    pub max_has_eaten_for_next_generation: f32,
    /// Haxe `HasEatenReductionForNextGeneration`.
    // Haxe: ServerSettings.HasEatenReductionForNextGeneration
    // C-SS-MORE-KNOBS
    pub has_eaten_reduction_for_next_generation: f32,
    /// Haxe `CoinsOnWoundingFactor` — fraction of target coins stolen on wound/kill (+1 floor).
    // Haxe: ServerSettings.CoinsOnWoundingFactor
    // WALLET-COINS
    pub coins_on_wounding_factor: f32,
    // --- C-SS-MORE-BATCH3 / settings_batch3 ---
    /// Haxe `CombatExhaustionCostPerAttack`.
    // Haxe: ServerSettings.CombatExhaustionCostPerAttack
    // C-SS-MORE-BATCH3
    pub combat_exhaustion_cost_per_attack: f32,
    /// Haxe `MinAgeToEat` (years).
    // Haxe: ServerSettings.MinAgeToEat
    // C-SS-MORE-BATCH3
    pub min_age_to_eat: f32,
    /// Haxe `MaxChildAgeForBreastFeeding` (years).
    // Haxe: ServerSettings.MaxChildAgeForBreastFeeding
    // C-SS-MORE-BATCH3
    pub max_child_age_for_breast_feeding: f32,
    /// Haxe `AllyConsideredClose` (tile radius).
    // Haxe: ServerSettings.AllyConsideredClose
    // C-SS-MORE-BATCH3
    pub ally_considered_close: f32,
    /// Haxe `MinMovementAgeInSec`.
    // Haxe: ServerSettings.MinMovementAgeInSec
    // C-SS-MORE-BATCH3
    pub min_movement_age_in_sec: f32,
    // --- C-SS-MORE-BATCH4 / settings_batch4 ---
    /// Haxe `CursedReceiveDamageFactor` — cursed target takes more damage.
    // Haxe: ServerSettings.CursedReceiveDamageFactor
    // C-SS-MORE-BATCH4
    pub cursed_receive_damage_factor: f32,
    /// Haxe `CursedMakeDamageFactor` — cursed attacker deals less damage.
    // Haxe: ServerSettings.CursedMakeDamageFactor
    // C-SS-MORE-BATCH4
    pub cursed_make_damage_factor: f32,
    /// Haxe `PickupBabyMaxDistance` — euclidean max for doBaby/BABY.
    // Haxe: ServerSettings.PickupBabyMaxDistance
    // C-SS-MORE-BATCH4
    pub pickup_baby_max_distance: f32,
    /// Haxe `InheritCoinsFactor` — fraction of wallet credited as coinsInherited.
    // Haxe: ServerSettings.InheritCoinsFactor
    // C-SS-MORE-BATCH4
    pub inherit_coins_factor: f32,
    /// Haxe `MinAgeFertile` (years). Client risk if min &lt; 14.
    // Haxe: ServerSettings.MinAgeFertile
    // C-SS-MORE-BATCH4
    pub min_age_fertile: f32,
    /// Haxe `MaxAgeFertile` (years, inclusive).
    // Haxe: ServerSettings.MaxAgeFertile
    // C-SS-MORE-BATCH4
    pub max_age_fertile: f32,
    // --- C-SS-MORE-BATCH5 / settings_batch5 ---
    /// Haxe `WeaponCoolDownFactor` — normal bloody cool-down mult.
    // Haxe: ServerSettings.WeaponCoolDownFactor
    // C-SS-MORE-BATCH5
    pub weapon_cooldown_factor: f32,
    /// Haxe `WeaponCoolDownFactorIfWounding`.
    // Haxe: ServerSettings.WeaponCoolDownFactorIfWounding
    // C-SS-MORE-BATCH5
    pub weapon_cooldown_factor_if_wounding: f32,
    /// Haxe `CloseEnemyWithWeaponSpeedFactor`.
    // Haxe: ServerSettings.CloseEnemyWithWeaponSpeedFactor
    // C-SS-MORE-BATCH5
    pub close_enemy_with_weapon_speed_factor: f32,
    /// Haxe `ExhaustionOnJump`.
    // Haxe: ServerSettings.ExhaustionOnJump
    // C-SS-MORE-BATCH5
    pub exhaustion_on_jump: f32,
    /// Haxe `HungryWorkHeat` — heat per food when transition temperature < 0.
    // Haxe: ServerSettings.HungryWorkHeat
    // C-SS-MORE-BATCH5
    pub hungry_work_heat: f32,
    /// Haxe `AISpeedFactorSerf`.
    // Haxe: ServerSettings.AISpeedFactorSerf
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_serf: f32,
    /// Haxe `AISpeedFactorCommoner`.
    // Haxe: ServerSettings.AISpeedFactorCommoner
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_commoner: f32,
    /// Haxe `AISpeedFactorNoble`.
    // Haxe: ServerSettings.AISpeedFactorNoble
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_noble: f32,
}

impl Default for GameplayKnobs {
    fn default() -> Self {
        Self {
            food_use_per_second: gameplay_defaults::FOOD_USE_PER_SECOND,
            healing_per_second: gameplay_defaults::HEALING_PER_SECOND,
            ageing_seconds_per_year: gameplay_defaults::AGEING_SECONDS_PER_YEAR,
            initial_player_move_speed: gameplay_defaults::INITIAL_PLAYER_MOVE_SPEED,
            speed_factor: gameplay_defaults::SPEED_FACTOR,
            yum_bonus: gameplay_defaults::YUM_BONUS,
            chance_for_offspring: gameplay_defaults::CHANCE_FOR_OFFSPRING,
            chance_for_animal_dying: gameplay_defaults::CHANCE_FOR_ANIMAL_DYING,
            hungry_work_cost: gameplay_defaults::HUNGRY_WORK_COST,
            birth_prestige_factor: gameplay_defaults::BIRTH_PRESTIGE_FACTOR,
            ally_strength_too_low_for_pickup: gameplay_defaults::ALLY_STRENGTH_TOO_LOW_FOR_PICKUP,
            time_confirm_new_follower: gameplay_defaults::TIME_CONFIRM_NEW_FOLLOWER,
            hire_cost: gameplay_defaults::HIRE_COST,
            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,
            auto_follow_player: gameplay_defaults::AUTO_FOLLOW_PLAYER,
            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,
            prestige_cost_per_damage_for_child: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CHILD,
            prestige_cost_per_damage_for_elderly:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ELDERLY,
            prestige_cost_per_damage_for_close_relatives:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CLOSE_RELATIVES,
            prestige_cost_per_damage_for_women_without_weapon:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_WOMEN_WITHOUT_WEAPON,
            food_factor: gameplay_defaults::FOOD_FACTOR,
            food_factor_eaten_more_than_eight_percent: gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT,
            food_factor_eaten_more_than_ten_percent: gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT,
            food_factor_eaten_less_than_five_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT,
            food_factor_eaten_less_than_three_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT,
            food_factor_eaten_less_than_one_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT,
            yum_food_restore: gameplay_defaults::YUM_FOOD_RESTORE,
            loved_food_restore: gameplay_defaults::LOVED_FOOD_RESTORE,
            yum_new_craving_chance: gameplay_defaults::YUM_NEW_CRAVING_CHANCE,
            food_reduction_per_eating: gameplay_defaults::FOOD_REDUCTION_PER_EATING,
            food_reduction_faktor_for_eating_meh: gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_MEH,
            health_lost_when_eating_meh: gameplay_defaults::HEALTH_LOST_WHEN_EATING_MEH,
            health_lost_when_eating_super_meh: gameplay_defaults::HEALTH_LOST_WHEN_EATING_SUPER_MEH,
            food_reduction_faktor_for_eating_high_quality:
                gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_HIGH_QUALITY,
            grown_up_food_store_max: gameplay_defaults::GROWN_UP_FOOD_STORE_MAX,
            new_born_food_store_max: gameplay_defaults::NEW_BORN_FOOD_STORE_MAX,
            old_age_food_store_max: gameplay_defaults::OLD_AGE_FOOD_STORE_MAX,
            min_biome_speed_factor: gameplay_defaults::MIN_BIOME_SPEED_FACTOR,
            hitpoints_speed_factor: gameplay_defaults::HITPOINTS_SPEED_FACTOR,
            combat_reputation_restore_per_year:
                gameplay_defaults::COMBAT_REPUTATION_RESTORE_PER_YEAR,
            // C-SS-MORE-KNOBS
            exhaustion_healing_factor: gameplay_defaults::EXHAUSTION_HEALING_FACTOR,
            wound_damage_factor: gameplay_defaults::WOUND_DAMAGE_FACTOR,
            wound_healing_factor: gameplay_defaults::WOUND_HEALING_FACTOR,
            // C-SS-MALE-HEAL
            exhaustion_healing_for_male_factor:
                gameplay_defaults::EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            // C-SS-TEMP-HEAL
            temperature_hits_damage_factor: gameplay_defaults::TEMPERATURE_HITS_DAMAGE_FACTOR,
            temperature_exhaustion_damage_factor:
                gameplay_defaults::TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
            max_movement_quad_jump_distance_before_force:
                gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE,
            food_restore_factor_while_feeding:
                gameplay_defaults::FOOD_RESTORE_FACTOR_WHILE_FEEDING,
            max_has_eaten_for_next_generation:
                gameplay_defaults::MAX_HAS_EATEN_FOR_NEXT_GENERATION,
            has_eaten_reduction_for_next_generation:
                gameplay_defaults::HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION,
            // WALLET-COINS
            coins_on_wounding_factor: gameplay_defaults::COINS_ON_WOUNDING_FACTOR,
            // C-SS-MORE-BATCH3 (male factor already under C-SS-MALE-HEAL above)
            combat_exhaustion_cost_per_attack:
                gameplay_defaults::COMBAT_EXHAUSTION_COST_PER_ATTACK,
            min_age_to_eat: gameplay_defaults::MIN_AGE_TO_EAT,
            max_child_age_for_breast_feeding:
                gameplay_defaults::MAX_CHILD_AGE_FOR_BREAST_FEEDING,
            ally_considered_close: gameplay_defaults::ALLY_CONSIDERED_CLOSE,
            min_movement_age_in_sec: gameplay_defaults::MIN_MOVEMENT_AGE_IN_SEC,
            // C-SS-MORE-BATCH4
            cursed_receive_damage_factor: gameplay_defaults::CURSED_RECEIVE_DAMAGE_FACTOR,
            cursed_make_damage_factor: gameplay_defaults::CURSED_MAKE_DAMAGE_FACTOR,
            pickup_baby_max_distance: gameplay_defaults::PICKUP_BABY_MAX_DISTANCE,
            inherit_coins_factor: gameplay_defaults::INHERIT_COINS_FACTOR,
            min_age_fertile: gameplay_defaults::MIN_AGE_FERTILE,
            max_age_fertile: gameplay_defaults::MAX_AGE_FERTILE,
            // C-SS-MORE-BATCH5
            weapon_cooldown_factor: gameplay_defaults::WEAPON_COOLDOWN_FACTOR,
            weapon_cooldown_factor_if_wounding:
                gameplay_defaults::WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
            close_enemy_with_weapon_speed_factor:
                gameplay_defaults::CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,
            exhaustion_on_jump: gameplay_defaults::EXHAUSTION_ON_JUMP,
            hungry_work_heat: gameplay_defaults::HUNGRY_WORK_HEAT,
            ai_speed_factor_serf: gameplay_defaults::AI_SPEED_FACTOR_SERF,
            ai_speed_factor_commoner: gameplay_defaults::AI_SPEED_FACTOR_COMMONER,
            ai_speed_factor_noble: gameplay_defaults::AI_SPEED_FACTOR_NOBLE,
        }
    }
}

impl GameplayKnobs {
    /// Snapshot from live config (sanitized by `ServerConfig::live_settings`).
    // Haxe: ServerSettings.readFromFile Reflect.setField on statics
    pub fn from_live(live: &LiveSettings) -> Self {
        Self {
            food_use_per_second: live.food_use_per_second,
            healing_per_second: live.healing_per_second,
            ageing_seconds_per_year: live.ageing_seconds_per_year,
            initial_player_move_speed: live.initial_player_move_speed,
            speed_factor: live.speed_factor,
            yum_bonus: live.yum_bonus,
            chance_for_offspring: live.chance_for_offspring,
            chance_for_animal_dying: live.chance_for_animal_dying,
            hungry_work_cost: live.hungry_work_cost,
            birth_prestige_factor: live.birth_prestige_factor,
            ally_strength_too_low_for_pickup: live.ally_strength_too_low_for_pickup,
            time_confirm_new_follower: live.time_confirm_new_follower,
            hire_cost: live.hire_cost,
            hire_cost_increase_per_person: live.hire_cost_increase_per_person,
            auto_follow_player: live.auto_follow_player,
            prestige_cost_per_damage_for_ally: live.prestige_cost_per_damage_for_ally,
            prestige_cost_per_damage_for_child: live.prestige_cost_per_damage_for_child,
            prestige_cost_per_damage_for_elderly: live.prestige_cost_per_damage_for_elderly,
            prestige_cost_per_damage_for_close_relatives: live
                .prestige_cost_per_damage_for_close_relatives,
            prestige_cost_per_damage_for_women_without_weapon: live
                .prestige_cost_per_damage_for_women_without_weapon,
            food_factor: live.food_factor,
            food_factor_eaten_more_than_eight_percent: live.food_factor_eaten_more_than_eight_percent,
            food_factor_eaten_more_than_ten_percent: live.food_factor_eaten_more_than_ten_percent,
            food_factor_eaten_less_than_five_percent: live.food_factor_eaten_less_than_five_percent,
            food_factor_eaten_less_than_three_percent: live.food_factor_eaten_less_than_three_percent,
            food_factor_eaten_less_than_one_percent: live.food_factor_eaten_less_than_one_percent,
            yum_food_restore: live.yum_food_restore,
            loved_food_restore: live.loved_food_restore,
            yum_new_craving_chance: live.yum_new_craving_chance,
            food_reduction_per_eating: live.food_reduction_per_eating,
            food_reduction_faktor_for_eating_meh: live.food_reduction_faktor_for_eating_meh,
            health_lost_when_eating_meh: live.health_lost_when_eating_meh,
            health_lost_when_eating_super_meh: live.health_lost_when_eating_super_meh,
            food_reduction_faktor_for_eating_high_quality: live
                .food_reduction_faktor_for_eating_high_quality,
            grown_up_food_store_max: live.grown_up_food_store_max,
            new_born_food_store_max: live.new_born_food_store_max,
            old_age_food_store_max: live.old_age_food_store_max,
            min_biome_speed_factor: live.min_biome_speed_factor,
            hitpoints_speed_factor: live.hitpoints_speed_factor,
            combat_reputation_restore_per_year: live.combat_reputation_restore_per_year,
            // C-SS-MORE-KNOBS
            exhaustion_healing_factor: live.exhaustion_healing_factor,
            wound_damage_factor: live.wound_damage_factor,
            wound_healing_factor: live.wound_healing_factor,
            // C-SS-MALE-HEAL
            exhaustion_healing_for_male_factor: live.exhaustion_healing_for_male_factor,
            // C-SS-TEMP-HEAL
            temperature_hits_damage_factor: live.temperature_hits_damage_factor,
            temperature_exhaustion_damage_factor: live.temperature_exhaustion_damage_factor,
            max_movement_quad_jump_distance_before_force: live
                .max_movement_quad_jump_distance_before_force,
            food_restore_factor_while_feeding: live.food_restore_factor_while_feeding,
            max_has_eaten_for_next_generation: live.max_has_eaten_for_next_generation,
            has_eaten_reduction_for_next_generation: live
                .has_eaten_reduction_for_next_generation,
            // WALLET-COINS
            coins_on_wounding_factor: live.coins_on_wounding_factor,
            // C-SS-MORE-BATCH3 (male factor already under C-SS-MALE-HEAL above)
            combat_exhaustion_cost_per_attack: live.combat_exhaustion_cost_per_attack,
            min_age_to_eat: live.min_age_to_eat,
            max_child_age_for_breast_feeding: live.max_child_age_for_breast_feeding,
            ally_considered_close: live.ally_considered_close,
            min_movement_age_in_sec: live.min_movement_age_in_sec,
            // C-SS-MORE-BATCH4
            cursed_receive_damage_factor: live.cursed_receive_damage_factor,
            cursed_make_damage_factor: live.cursed_make_damage_factor,
            pickup_baby_max_distance: live.pickup_baby_max_distance,
            inherit_coins_factor: live.inherit_coins_factor,
            min_age_fertile: live.min_age_fertile,
            max_age_fertile: live.max_age_fertile,
            // C-SS-MORE-BATCH5
            weapon_cooldown_factor: live.weapon_cooldown_factor,
            weapon_cooldown_factor_if_wounding: live.weapon_cooldown_factor_if_wounding,
            close_enemy_with_weapon_speed_factor: live.close_enemy_with_weapon_speed_factor,
            exhaustion_on_jump: live.exhaustion_on_jump,
            hungry_work_heat: live.hungry_work_heat,
            ai_speed_factor_serf: live.ai_speed_factor_serf,
            ai_speed_factor_commoner: live.ai_speed_factor_commoner,
            ai_speed_factor_noble: live.ai_speed_factor_noble,
        }
    }

    /// Haxe capacity age-band knobs for `calculateFoodStoreMax`.
    // Haxe: ServerSettings.GrownUp/NewBorn/OldAgeFoodStoreMax
    // C-SS-AGE-FOOD
    #[inline]
    pub fn food_store_max_knobs(&self) -> crate::food_store_max::FoodStoreMaxKnobs {
        crate::food_store_max::FoodStoreMaxKnobs {
            grown_up: self.grown_up_food_store_max,
            newborn: self.new_born_food_store_max,
            old_age: self.old_age_food_store_max,
        }
    }

    /// Haxe `WorldMap.getFoodFactor` band table from live knobs.
    // Haxe: ServerSettings.FoodFactorEaten*
    // C-SS-FULL-TABLE
    pub fn food_factor_eaten_bands(&self) -> crate::search_best_food::FoodFactorEatenBands {
        crate::search_best_food::FoodFactorEatenBands {
            less_than_one_percent: self.food_factor_eaten_less_than_one_percent,
            less_than_three_percent: self.food_factor_eaten_less_than_three_percent,
            less_than_five_percent: self.food_factor_eaten_less_than_five_percent,
            more_than_eight_percent: self.food_factor_eaten_more_than_eight_percent,
            more_than_ten_percent: self.food_factor_eaten_more_than_ten_percent,
        }
    }

    /// Live eat-path knobs for [`crate::compute_eat_full`] / [`crate::YumState::eat_full`].
    // Haxe: ServerSettings.YumBonus / FoodFactor / FoodReduction* / HealthLost*
    // C-SS-FULL-TABLE
    pub fn eat_live_knobs(&self) -> crate::yum::EatLiveKnobs {
        crate::yum::EatLiveKnobs {
            yum_bonus: self.yum_bonus,
            food_factor: self.food_factor,
            food_reduction_per_eating: self.food_reduction_per_eating,
            food_reduction_faktor_meh: self.food_reduction_faktor_for_eating_meh,
            health_lost_meh: self.health_lost_when_eating_meh,
            health_lost_super_meh: self.health_lost_when_eating_super_meh,
        }
    }

    /// Live craving-restore knobs for [`crate::YumState::do_increase_food_value_ex`].
    // Haxe: ServerSettings.YumFoodRestore / LovedFoodRestore / YumNewCravingChance
    // C-SS-FULL-TABLE
    pub fn yum_restore_knobs(&self) -> crate::yum::YumRestoreKnobs {
        crate::yum::YumRestoreKnobs {
            yum_food_restore: self.yum_food_restore,
            loved_food_restore: self.loved_food_restore,
            yum_new_craving_chance: self.yum_new_craving_chance,
        }
    }

    /// Live category prestige-cost multipliers for HIT illegal-unarmed costs.
    // Haxe: ServerSettings.PrestigeCostPerDamageFor*
    // C-SS-MORE / PRESTIGE-ALLY-COST
    pub fn prestige_cost_factors(&self) -> crate::reputation::PrestigeCostFactors {
        crate::reputation::PrestigeCostFactors {
            child: self.prestige_cost_per_damage_for_child,
            elderly: self.prestige_cost_per_damage_for_elderly,
            ally: self.prestige_cost_per_damage_for_ally,
            close_relative: self.prestige_cost_per_damage_for_close_relatives,
            woman_unarmed: self.prestige_cost_per_damage_for_women_without_weapon,
            // C-SS-MORE-BATCH3
            min_age_to_eat: self.min_age_to_eat,
        }
    }

    /// Live vitals speed knobs for [`crate::move_speed::vitals_speed_product_ex`].
    // Haxe: ServerSettings.HitpointsSpeedFactor / GrownUpFoodStoreMax
    // C-SS-TAIL-KNOBS
    pub fn vitals_speed_knobs(&self) -> (f32, f32) {
        (self.grown_up_food_store_max, self.hitpoints_speed_factor)
    }

    /// Live inherit-eaten knobs for [`crate::yum::inherit_eaten_food_counts`].
    // Haxe: ServerSettings.MaxHasEatenForNextGeneration / HasEatenReductionForNextGeneration
    // C-SS-MORE-KNOBS
    #[inline]
    pub fn inherit_eaten_knobs(&self) -> (f32, f32) {
        (
            self.has_eaten_reduction_for_next_generation,
            self.max_has_eaten_for_next_generation,
        )
    }

    /// Timed MOVE jump gate as f64 (Haxe MaxMovementQuadJumpDistanceBeforeForce).
    // Haxe: ServerSettings.MaxMovementQuadJumpDistanceBeforeForce
    // C-SS-MORE-KNOBS
    #[inline]
    pub fn max_move_quad_jump_before_force(&self) -> f64 {
        let v = self.max_movement_quad_jump_distance_before_force;
        if v.is_finite() && v > 0.0 {
            v as f64
        } else {
            gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE as f64
        }
    }

    /// Live weapon cool-down factors (normal, if-wounding).
    // Haxe: ServerSettings.WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding
    // C-SS-MORE-BATCH5
    #[inline]
    pub fn weapon_cooldown_knobs(&self) -> (f32, f32) {
        (self.weapon_cooldown_factor, self.weapon_cooldown_factor_if_wounding)
    }

    /// Live AI prestige-class speed factors (serf, commoner, noble).
    // Haxe: ServerSettings.AISpeedFactor*
    // C-SS-MORE-BATCH5
    #[inline]
    pub fn ai_speed_knobs(&self) -> (f32, f32, f32) {
        (
            self.ai_speed_factor_serf,
            self.ai_speed_factor_commoner,
            self.ai_speed_factor_noble,
        )
    }

    /// Live vitals speed knobs including close-enemy + AI class factors.
    // Haxe: HitpointsSpeedFactor / GrownUpFoodStoreMax / CloseEnemy* / AISpeedFactor*
    // C-SS-TAIL-KNOBS + C-SS-MORE-BATCH5
    #[inline]
    pub fn vitals_speed_live_knobs(&self) -> crate::VitalsSpeedLiveKnobs {
        crate::VitalsSpeedLiveKnobs {
            grown_up_food_store_max: self.grown_up_food_store_max,
            hitpoints_speed_factor: self.hitpoints_speed_factor,
            close_enemy_with_weapon_speed_factor: self.close_enemy_with_weapon_speed_factor,
            ai_speed_factor_serf: self.ai_speed_factor_serf,
            ai_speed_factor_commoner: self.ai_speed_factor_commoner,
            ai_speed_factor_noble: self.ai_speed_factor_noble,
        }
    }
}

/// What changed when applying a live settings snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveApplyReport {
    pub keys: Vec<&'static str>,
}

impl LiveApplyReport {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Optional boot package for live settings + hot-reload wiring into the sim loop.
///
/// Haxe: `ServerSettings.readFromFile` at boot + `TimeHelper` every 200 ticks.
#[derive(Debug)]
pub struct SimBootLive {
    /// When present, re-reads `server.toml` on the Haxe cadence.
    pub hot_reload: Option<HotReloadTracker>,
    /// Shared mirror of live knobs for NPC scheduler / outer tasks.
    pub live_share: Option<Arc<RwLock<LiveSettings>>>,
    /// Initial season length in seconds (`season_duration_years * 60`).
    pub season_length_secs: f32,
    /// Haxe `EternalWinter` at boot.
    pub eternal_winter: bool,
    /// SOCIAL-WAR-PERSIST: shared war/posse snapshot for autosave (WPS1).
    pub war_posse_share: Option<WarPosseShare>,
    /// PLAYERS-BIN: sticky living roster for autosave (PLB1).
    pub players_share: Option<crate::PlayersShare>,
    /// FOODSTATS-DISK: world eaten-food stats for FoodStats.txt autosave dump.
    pub world_food_share: Option<WorldFoodShare>,
    /// OBJECTCOUNTS-LIVE: world object census for ObjectCounts.txt autosave dump.
    pub object_counts_share: Option<ObjectCountsShare>,
    /// AI-LLM-HTTP-DRAIN: job/result bridge for ol-server `call_ai_async` worker.
    pub llm_speech_share: Option<crate::LlmSpeechIoShare>,
}

impl Default for SimBootLive {
    fn default() -> Self {
        Self {
            hot_reload: None,
            live_share: None,
            season_length_secs: 450.0,
            eternal_winter: false,
            war_posse_share: None,
            players_share: None,
            world_food_share: None,
            object_counts_share: None,
            llm_speech_share: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Haxe DoSeason season-duration helpers
// ---------------------------------------------------------------------------

/// Haxe `DoSeason`: `TimeToNextSeasonInYears = SeasonDuration/2 + random * SeasonDuration`.
///
/// `unit_random` is in \[0, 1\]; result range is `[0.5, 1.5] * SeasonDuration` years.
/// // Haxe: TimeHelper.DoSeason
pub fn haxe_next_season_duration_years(season_duration_years: f32, unit_random: f32) -> f32 {
    let base = if season_duration_years.is_finite() && season_duration_years > 0.0 {
        season_duration_years
    } else {
        7.5
    };
    let u = if unit_random.is_finite() {
        unit_random.clamp(0.0, 1.0)
    } else {
        0.5
    };
    base * 0.5 + u * base
}

/// Haxe `SeasonHardness = randomFloat() + 0.5`; hard seasons square it.
///
/// // Haxe: TimeHelper.DoSeason SeasonHardness
pub fn haxe_season_hardness(unit_random: f32, hard_season: bool) -> f32 {
    let u = if unit_random.is_finite() {
        unit_random.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let h = u + 0.5; // [0.5, 1.5]
    if hard_season {
        h * h
    } else {
        h
    }
}

/// Whether the newly entered season is a Haxe "hard" season (Winter or Summer).
///
/// // Haxe: TimeHelper.DoSeason hardSeason
#[inline]
pub fn is_hard_season(season: Season) -> bool {
    matches!(season, Season::Winter | Season::Summer)
}

/// Full next-season length in seconds after a roll:
/// `(SeasonDuration/2 + r·SeasonDuration) * hardness * 60`.
///
/// // Haxe: TimeHelper.DoSeason TimeToNextSeasonInYears *= SeasonHardness
pub fn haxe_next_season_length_secs(
    season_duration_years: f32,
    unit_random_duration: f32,
    unit_random_hardness: f32,
    hard_season: bool,
) -> f32 {
    let years = haxe_next_season_duration_years(season_duration_years, unit_random_duration);
    let hard = haxe_season_hardness(unit_random_hardness, hard_season);
    (years * hard * HAXE_YEAR_SECS).max(1.0)
}

/// Re-seed `environment.season_length` after a season rollover (Haxe DoSeason).
///
/// Also refreshes `season_hardness` + `season_text` (Haxe `SeasonHardness` /
/// `SeasonText`) for AI soul prompts.
///
/// Uses [`SimState::season_duration_base_secs`] as the config base (Haxe
/// `ServerSettings.SeasonDuration` converted to seconds).
pub fn reseed_season_length_after_roll(
    state: &mut SimState,
    unit_random_duration: f32,
    unit_random_hardness: f32,
) {
    let base_years =
        if state.season_duration_base_secs.is_finite() && state.season_duration_base_secs > 0.0 {
            state.season_duration_base_secs / HAXE_YEAR_SECS
        } else {
            7.5
        };
    let hard = is_hard_season(state.environment.season);
    let sl = haxe_next_season_length_secs(
        base_years,
        unit_random_duration,
        unit_random_hardness,
        hard,
    );
    state.environment.season_length = sl;
    // Haxe: SeasonHardness + SeasonText on rollover (AI-SOUL-WIRE).
    let (text, op_hard) = crate::player_soul::haxe_season_roll_text_and_hardness(
        state.environment.season.as_str(),
        unit_random_hardness,
    );
    state.environment.season_text = text;
    state.environment.season_hardness = op_hard;
}

/// Apply runtime-safe knobs onto [`SimState`].
///
/// Maps:
/// - `sim_speed` → `SimState::sim_speed`
/// - `timed_movement` / `move_jump_max_chebyshev` / `broadcast_all_updates`
/// - `client_version_strict`
/// - `shutdown_*` countdown lengths
/// - `eternal_winter` → force Winter + sticky flag
/// - `season_length_secs` → base + current `Environment::season_length`
///   (ops-friendly mid-season boundary update; next roll re-samples via
///   [`reseed_season_length_after_roll`])
/// - `lockpick_*` → `SimState::lockpick_settings` (Haxe LockpickSucessChance etc.)
///
/// NPC knobs are returned to the caller (scheduler owns them); this function
/// does not touch NPC tasks.
pub fn apply_live_settings(state: &mut SimState, live: &LiveSettings) -> LiveApplyReport {
    let mut keys = Vec::new();

    let speed = if live.sim_speed.is_finite() && live.sim_speed >= 0.0 {
        live.sim_speed
    } else {
        1.0
    };
    if (state.sim_speed - speed).abs() > f32::EPSILON {
        state.sim_speed = speed;
        keys.push("sim_speed");
    }

    if state.timed_movement != live.timed_movement {
        state.timed_movement = live.timed_movement;
        keys.push("timed_movement");
    }

    let jump = live.move_jump_max_chebyshev.max(0);
    if state.move_jump_max_chebyshev != jump {
        state.move_jump_max_chebyshev = jump;
        keys.push("move_jump_max_chebyshev");
    }

    if state.broadcast_all_updates != live.broadcast_all_updates {
        state.broadcast_all_updates = live.broadcast_all_updates;
        keys.push("broadcast_all_updates");
    }

    if state.client_version_strict != live.client_version_strict {
        state.client_version_strict = live.client_version_strict;
        keys.push("client_version_strict");
    }

    let scd = (live.shutdown_countdown_secs.max(1) as f32).max(1.0);
    if (state.shutdown_countdown_secs - scd).abs() > f32::EPSILON {
        state.shutdown_countdown_secs = scd;
        keys.push("shutdown_countdown_secs");
    }
    let sap = (live.shutdown_apocalypse_secs.max(1) as f32).max(0.5);
    if (state.shutdown_apocalypse_secs - sap).abs() > f32::EPSILON {
        state.shutdown_apocalypse_secs = sap;
        keys.push("shutdown_apocalypse_secs");
    }

    if state.eternal_winter != live.eternal_winter {
        state.eternal_winter = live.eternal_winter;
        keys.push("eternal_winter");
    }
    if live.eternal_winter {
        // Haxe: `if (ServerSettings.EternalWinter) Season = Seasons.Winter;`
        if state.environment.season != Season::Winter {
            state.environment.set_season(Season::Winter);
            if !keys.contains(&"eternal_winter") {
                keys.push("eternal_winter");
            }
        }
    }

    let sl = if live.season_length_secs.is_finite() && live.season_length_secs > 0.0 {
        live.season_length_secs
    } else {
        450.0
    };
    // Always keep Haxe SeasonDuration base for next-roll re-sample.
    if (state.season_duration_base_secs - sl).abs() > 0.01 {
        state.season_duration_base_secs = sl;
    }
    // Mid-session: apply as current boundary so ops can shorten/lengthen the active season.
    // Haxe keeps `TimeToNextSeasonInYears` until the next roll; Rust chooses immediate
    // boundary update for operator feedback (documented intentional delta).
    if (state.environment.season_length - sl).abs() > 0.01 {
        state.environment.season_length = sl;
        // Avoid multi-roll if elapsed already past the new length.
        if state.environment.season_elapsed >= sl {
            state.environment.season_elapsed = (sl * 0.999).max(0.0);
        }
        keys.push("season_length_secs");
    }

    // LOCKPICK-SETTINGS / Haxe ServerSettings.Lockpick*
    let lp = crate::locks::LockpickSettings::from_live(
        live.lockpick_success_chance,
        live.lockpick_fail_chance,
        live.lockpick_exhaustion_cost,
        live.lockpick_coin_cost,
    );
    if state.lockpick_settings != lp {
        if (state.lockpick_settings.success_chance - lp.success_chance).abs() > f32::EPSILON {
            keys.push("lockpick_success_chance");
        }
        if (state.lockpick_settings.fail_chance - lp.fail_chance).abs() > f32::EPSILON {
            keys.push("lockpick_fail_chance");
        }
        if (state.lockpick_settings.exhaustion_cost - lp.exhaustion_cost).abs() > f32::EPSILON {
            keys.push("lockpick_exhaustion_cost");
        }
        if (state.lockpick_settings.coin_cost - lp.coin_cost).abs() > f32::EPSILON {
            keys.push("lockpick_coin_cost");
        }
        state.lockpick_settings = lp;
    }

    // SETTINGS-FIELD-MAP gameplay batch → SimState.gameplay
    // Haxe: ServerSettings.readFromFile Reflect.setField (FoodUsePerSecond, …)
    let gp = GameplayKnobs::from_live(live);
    let old = &state.gameplay;
    let mut push_gp = |name: &'static str, changed: bool| {
        if changed {
            keys.push(name);
        }
    };
    push_gp(
        "food_use_per_second",
        (old.food_use_per_second - gp.food_use_per_second).abs() > f32::EPSILON,
    );
    push_gp(
        "healing_per_second",
        (old.healing_per_second - gp.healing_per_second).abs() > f32::EPSILON,
    );
    push_gp(
        "ageing_seconds_per_year",
        (old.ageing_seconds_per_year - gp.ageing_seconds_per_year).abs() > f32::EPSILON,
    );
    push_gp(
        "initial_player_move_speed",
        (old.initial_player_move_speed - gp.initial_player_move_speed).abs() > f32::EPSILON,
    );
    push_gp(
        "speed_factor",
        (old.speed_factor - gp.speed_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "yum_bonus",
        (old.yum_bonus - gp.yum_bonus).abs() > f32::EPSILON,
    );
    push_gp(
        "chance_for_offspring",
        (old.chance_for_offspring - gp.chance_for_offspring).abs() > 1e-12,
    );
    push_gp(
        "chance_for_animal_dying",
        (old.chance_for_animal_dying - gp.chance_for_animal_dying).abs() > 1e-12,
    );
    push_gp(
        "hungry_work_cost",
        (old.hungry_work_cost - gp.hungry_work_cost).abs() > f32::EPSILON,
    );
    push_gp(
        "birth_prestige_factor",
        (old.birth_prestige_factor - gp.birth_prestige_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "ally_strength_too_low_for_pickup",
        (old.ally_strength_too_low_for_pickup - gp.ally_strength_too_low_for_pickup).abs()
            > f32::EPSILON,
    );
    push_gp(
        "time_confirm_new_follower",
        (old.time_confirm_new_follower - gp.time_confirm_new_follower).abs() > f32::EPSILON,
    );
    push_gp(
        "hire_cost",
        (old.hire_cost - gp.hire_cost).abs() > f32::EPSILON,
    );
    push_gp(
        "hire_cost_increase_per_person",
        (old.hire_cost_increase_per_person - gp.hire_cost_increase_per_person).abs()
            > f32::EPSILON,
    );
    push_gp(
        "auto_follow_player",
        old.auto_follow_player != gp.auto_follow_player,
    );
    push_gp(
        "prestige_cost_per_damage_for_ally",
        (old.prestige_cost_per_damage_for_ally - gp.prestige_cost_per_damage_for_ally).abs()
            > f32::EPSILON,
    );
    // C-SS-MORE PrestigeCost* non-ally
    push_gp(
        "prestige_cost_per_damage_for_child",
        (old.prestige_cost_per_damage_for_child - gp.prestige_cost_per_damage_for_child).abs()
            > f32::EPSILON,
    );
    push_gp(
        "prestige_cost_per_damage_for_elderly",
        (old.prestige_cost_per_damage_for_elderly - gp.prestige_cost_per_damage_for_elderly).abs()
            > f32::EPSILON,
    );
    push_gp(
        "prestige_cost_per_damage_for_close_relatives",
        (old.prestige_cost_per_damage_for_close_relatives
            - gp.prestige_cost_per_damage_for_close_relatives)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "prestige_cost_per_damage_for_women_without_weapon",
        (old.prestige_cost_per_damage_for_women_without_weapon
            - gp.prestige_cost_per_damage_for_women_without_weapon)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_factor",
        (old.food_factor - gp.food_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "food_factor_eaten_more_than_eight_percent",
        (old.food_factor_eaten_more_than_eight_percent
            - gp.food_factor_eaten_more_than_eight_percent)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_factor_eaten_more_than_ten_percent",
        (old.food_factor_eaten_more_than_ten_percent - gp.food_factor_eaten_more_than_ten_percent)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_factor_eaten_less_than_five_percent",
        (old.food_factor_eaten_less_than_five_percent - gp.food_factor_eaten_less_than_five_percent)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_factor_eaten_less_than_three_percent",
        (old.food_factor_eaten_less_than_three_percent
            - gp.food_factor_eaten_less_than_three_percent)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_factor_eaten_less_than_one_percent",
        (old.food_factor_eaten_less_than_one_percent - gp.food_factor_eaten_less_than_one_percent)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "yum_food_restore",
        (old.yum_food_restore - gp.yum_food_restore).abs() > f32::EPSILON,
    );
    push_gp(
        "loved_food_restore",
        (old.loved_food_restore - gp.loved_food_restore).abs() > f32::EPSILON,
    );
    push_gp(
        "yum_new_craving_chance",
        (old.yum_new_craving_chance - gp.yum_new_craving_chance).abs() > f32::EPSILON,
    );
    push_gp(
        "food_reduction_per_eating",
        (old.food_reduction_per_eating - gp.food_reduction_per_eating).abs() > f32::EPSILON,
    );
    push_gp(
        "food_reduction_faktor_for_eating_meh",
        (old.food_reduction_faktor_for_eating_meh - gp.food_reduction_faktor_for_eating_meh)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "health_lost_when_eating_meh",
        (old.health_lost_when_eating_meh - gp.health_lost_when_eating_meh).abs() > f32::EPSILON,
    );
    push_gp(
        "health_lost_when_eating_super_meh",
        (old.health_lost_when_eating_super_meh - gp.health_lost_when_eating_super_meh).abs()
            > f32::EPSILON,
    );
    // C-SS-TAIL-KNOBS
    push_gp(
        "food_reduction_faktor_for_eating_high_quality",
        (old.food_reduction_faktor_for_eating_high_quality
            - gp.food_reduction_faktor_for_eating_high_quality)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "grown_up_food_store_max",
        (old.grown_up_food_store_max - gp.grown_up_food_store_max).abs() > f32::EPSILON,
    );
    // C-SS-AGE-FOOD
    push_gp(
        "new_born_food_store_max",
        (old.new_born_food_store_max - gp.new_born_food_store_max).abs() > f32::EPSILON,
    );
    push_gp(
        "old_age_food_store_max",
        (old.old_age_food_store_max - gp.old_age_food_store_max).abs() > f32::EPSILON,
    );
    push_gp(
        "min_biome_speed_factor",
        (old.min_biome_speed_factor - gp.min_biome_speed_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "hitpoints_speed_factor",
        (old.hitpoints_speed_factor - gp.hitpoints_speed_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "combat_reputation_restore_per_year",
        (old.combat_reputation_restore_per_year - gp.combat_reputation_restore_per_year).abs()
            > f32::EPSILON,
    );
    // C-SS-MORE-KNOBS
    push_gp(
        "exhaustion_healing_factor",
        (old.exhaustion_healing_factor - gp.exhaustion_healing_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "wound_damage_factor",
        (old.wound_damage_factor - gp.wound_damage_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "wound_healing_factor",
        (old.wound_healing_factor - gp.wound_healing_factor).abs() > f32::EPSILON,
    );
    // C-SS-MALE-HEAL
    push_gp(
        "exhaustion_healing_for_male_factor",
        (old.exhaustion_healing_for_male_factor - gp.exhaustion_healing_for_male_factor).abs()
            > f32::EPSILON,
    );
    // C-SS-TEMP-HEAL
    push_gp(
        "temperature_hits_damage_factor",
        (old.temperature_hits_damage_factor - gp.temperature_hits_damage_factor).abs()
            > f32::EPSILON,
    );
    push_gp(
        "temperature_exhaustion_damage_factor",
        (old.temperature_exhaustion_damage_factor - gp.temperature_exhaustion_damage_factor)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "max_movement_quad_jump_distance_before_force",
        (old.max_movement_quad_jump_distance_before_force
            - gp.max_movement_quad_jump_distance_before_force)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "food_restore_factor_while_feeding",
        (old.food_restore_factor_while_feeding - gp.food_restore_factor_while_feeding).abs()
            > f32::EPSILON,
    );
    push_gp(
        "max_has_eaten_for_next_generation",
        (old.max_has_eaten_for_next_generation - gp.max_has_eaten_for_next_generation).abs()
            > f32::EPSILON,
    );
    push_gp(
        "has_eaten_reduction_for_next_generation",
        (old.has_eaten_reduction_for_next_generation
            - gp.has_eaten_reduction_for_next_generation)
            .abs()
            > f32::EPSILON,
    );
    // WALLET-COINS
    push_gp(
        "coins_on_wounding_factor",
        (old.coins_on_wounding_factor - gp.coins_on_wounding_factor).abs() > f32::EPSILON,
    );
    // C-SS-MORE-BATCH3 (male factor already pushed under C-SS-MALE-HEAL above)
    push_gp(
        "combat_exhaustion_cost_per_attack",
        (old.combat_exhaustion_cost_per_attack - gp.combat_exhaustion_cost_per_attack)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "min_age_to_eat",
        (old.min_age_to_eat - gp.min_age_to_eat).abs() > f32::EPSILON,
    );
    push_gp(
        "max_child_age_for_breast_feeding",
        (old.max_child_age_for_breast_feeding - gp.max_child_age_for_breast_feeding)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "ally_considered_close",
        (old.ally_considered_close - gp.ally_considered_close).abs() > f32::EPSILON,
    );
    push_gp(
        "min_movement_age_in_sec",
        (old.min_movement_age_in_sec - gp.min_movement_age_in_sec).abs() > f32::EPSILON,
    );
    // C-SS-MORE-BATCH4
    push_gp(
        "cursed_receive_damage_factor",
        (old.cursed_receive_damage_factor - gp.cursed_receive_damage_factor).abs()
            > f32::EPSILON,
    );
    push_gp(
        "cursed_make_damage_factor",
        (old.cursed_make_damage_factor - gp.cursed_make_damage_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "pickup_baby_max_distance",
        (old.pickup_baby_max_distance - gp.pickup_baby_max_distance).abs() > f32::EPSILON,
    );
    push_gp(
        "inherit_coins_factor",
        (old.inherit_coins_factor - gp.inherit_coins_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "min_age_fertile",
        (old.min_age_fertile - gp.min_age_fertile).abs() > f32::EPSILON,
    );
    push_gp(
        "max_age_fertile",
        (old.max_age_fertile - gp.max_age_fertile).abs() > f32::EPSILON,
    );
    // C-SS-MORE-BATCH5
    push_gp(
        "weapon_cooldown_factor",
        (old.weapon_cooldown_factor - gp.weapon_cooldown_factor).abs() > f32::EPSILON,
    );
    push_gp(
        "weapon_cooldown_factor_if_wounding",
        (old.weapon_cooldown_factor_if_wounding - gp.weapon_cooldown_factor_if_wounding)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "close_enemy_with_weapon_speed_factor",
        (old.close_enemy_with_weapon_speed_factor
            - gp.close_enemy_with_weapon_speed_factor)
            .abs()
            > f32::EPSILON,
    );
    push_gp(
        "exhaustion_on_jump",
        (old.exhaustion_on_jump - gp.exhaustion_on_jump).abs() > f32::EPSILON,
    );
    push_gp(
        "hungry_work_heat",
        (old.hungry_work_heat - gp.hungry_work_heat).abs() > f32::EPSILON,
    );
    push_gp(
        "ai_speed_factor_serf",
        (old.ai_speed_factor_serf - gp.ai_speed_factor_serf).abs() > f32::EPSILON,
    );
    push_gp(
        "ai_speed_factor_commoner",
        (old.ai_speed_factor_commoner - gp.ai_speed_factor_commoner).abs() > f32::EPSILON,
    );
    push_gp(
        "ai_speed_factor_noble",
        (old.ai_speed_factor_noble - gp.ai_speed_factor_noble).abs() > f32::EPSILON,
    );
    state.gameplay = gp;

    // TWIN-MULTI-SERVER: re-sync peer list from live twin_peers (preserve last_pong).
    // Haxe: Connection.loginHelper TODO twins — multi-server peers product registry
    let changed_twins = state.twins.sync_endpoints(
        live.twin_peers
            .iter()
            .map(|p| (p.host.as_str(), p.port)),
    );
    if changed_twins {
        keys.push("twin_peers");
    }

    LiveApplyReport { keys }
}

/// Enforce eternal winter on each season tick (Haxe DoSeason guard).
#[inline]
pub fn enforce_eternal_winter(state: &mut SimState) {
    if state.eternal_winter && state.environment.season != Season::Winter {
        state.environment.set_season(Season::Winter);
    }
}

/// Intent drain budget from live settings (at least 1).
#[inline]
pub fn intent_budget_from_live(live: &LiveSettings) -> usize {
    live.intent_drain_budget.max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Season;
    use ol_config::ServerConfig;
    use ol_content::ContentDb;
    use ol_world::World;
    use std::sync::{Arc, RwLock};

    fn empty_state() -> SimState {
        let world = Arc::new(RwLock::new(World::new(8, 8, false)));
        let content = Arc::new(ContentDb::default());
        SimState::new(world, content)
    }

    #[test]
    fn apply_live_settings_speed_and_season() {
        let mut state = empty_state();
        assert!((state.sim_speed - 1.0).abs() < f32::EPSILON);
        assert!(!state.eternal_winter);

        let live = ServerConfig {
            sim_speed: 2.0,
            eternal_winter: true,
            season_duration_years: 1.0, // → 60 s
            timed_movement: false,
            broadcast_all_updates: true,
            ..Default::default()
        }
        .live_settings();

        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"sim_speed"));
        assert!(report.keys.contains(&"eternal_winter"));
        assert!(report.keys.contains(&"season_length_secs"));
        assert!(report.keys.contains(&"timed_movement"));
        assert!((state.sim_speed - 2.0).abs() < f32::EPSILON);
        assert!(state.eternal_winter);
        assert_eq!(state.environment.season, Season::Winter);
        assert!((state.environment.season_length - 60.0).abs() < 0.01);
        assert!((state.season_duration_base_secs - 60.0).abs() < 0.01);
        assert!(!state.timed_movement);
        assert!(state.broadcast_all_updates);

        // Idempotent second apply
        let report2 = apply_live_settings(&mut state, &live);
        assert!(report2.is_empty());
    }

    #[test]
    fn enforce_eternal_winter_resets_season() {
        let mut state = empty_state();
        state.eternal_winter = true;
        state.environment.set_season(Season::Summer);
        enforce_eternal_winter(&mut state);
        assert_eq!(state.environment.season, Season::Winter);
    }

    #[test]
    fn intent_budget_from_live_min_one() {
        let live = ServerConfig {
            intent_drain_budget: 0,
            ..Default::default()
        }
        .live_settings();
        assert_eq!(intent_budget_from_live(&live), 1);
    }

    /// TWIN-MULTI-SERVER: live twin_peers re-sync preserves pong on matching endpoint.
    #[test]
    fn apply_live_settings_twin_peers_preserves_pong() {
        use ol_config::TwinPeerConfig;
        let mut state = empty_state();
        state.twins = crate::TwinRegistry::from_endpoints([("127.0.0.1", 8006u16)]);
        assert!(state.twins.record_pong("127.0.0.1", 8006, 11.0));

        let live = ServerConfig {
            twin_peers: vec![
                TwinPeerConfig {
                    host: "127.0.0.1".into(),
                    port: 8006,
                },
                TwinPeerConfig {
                    host: "10.0.0.2".into(),
                    port: 8007,
                },
            ],
            ..Default::default()
        }
        .live_settings();
        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"twin_peers"));
        assert_eq!(state.twins.peers()[0].last_pong, Some(11.0));
        assert_eq!(state.twins.len(), 2);
    }

    #[test]
    fn haxe_next_season_duration_years_range() {
        // 7.5 years → sample in [3.75, 11.25]
        let lo = haxe_next_season_duration_years(7.5, 0.0);
        let mid = haxe_next_season_duration_years(7.5, 0.5);
        let hi = haxe_next_season_duration_years(7.5, 1.0);
        assert!((lo - 3.75).abs() < 0.01);
        assert!((mid - 7.5).abs() < 0.01);
        assert!((hi - 11.25).abs() < 0.01);
        // bad years → default 7.5 base
        let bad = haxe_next_season_duration_years(-1.0, 0.0);
        assert!((bad - 3.75).abs() < 0.01);
    }

    #[test]
    fn haxe_season_hardness_hard_squares() {
        // unit 0.5 → hardness 1.0; hard → 1.0
        assert!((haxe_season_hardness(0.5, false) - 1.0).abs() < f32::EPSILON);
        assert!((haxe_season_hardness(0.5, true) - 1.0).abs() < f32::EPSILON);
        // unit 1.0 → 1.5; hard → 2.25
        assert!((haxe_season_hardness(1.0, false) - 1.5).abs() < f32::EPSILON);
        assert!((haxe_season_hardness(1.0, true) - 2.25).abs() < f32::EPSILON);
        // unit 0.0 → 0.5; hard → 0.25
        assert!((haxe_season_hardness(0.0, false) - 0.5).abs() < f32::EPSILON);
        assert!((haxe_season_hardness(0.0, true) - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn haxe_next_season_length_secs_formula() {
        // years sample: 1.0/2 + 0.5*1.0 = 1.0; hardness 1.0 → 60 s
        let s = haxe_next_season_length_secs(1.0, 0.5, 0.5, false);
        assert!((s - 60.0).abs() < 0.01);
        // years sample: 7.5; hardness 1.5 (unit 1, soft) → 7.5*1.5*60 = 675
        let s2 = haxe_next_season_length_secs(7.5, 0.5, 1.0, false);
        assert!((s2 - 675.0).abs() < 0.1);
        // hard season squares hardness: 1.5²=2.25 → 7.5*2.25*60 = 1012.5
        let s3 = haxe_next_season_length_secs(7.5, 0.5, 1.0, true);
        assert!((s3 - 1012.5).abs() < 0.1);
    }

    #[test]
    fn reseed_season_length_after_roll_uses_base() {
        let mut state = empty_state();
        state.season_duration_base_secs = 60.0; // 1 year
        state.environment.set_season(Season::Autumn); // soft
        reseed_season_length_after_roll(&mut state, 0.5, 0.5);
        // 1.0 years * hardness 1.0 * 60 = 60
        assert!((state.environment.season_length - 60.0).abs() < 0.01);
        assert_eq!(state.environment.season_text, "Autumn");
        assert!((state.environment.season_hardness - 1.0).abs() < 1e-4);

        state.environment.set_season(Season::Winter); // hard
        reseed_season_length_after_roll(&mut state, 0.5, 1.0);
        // Length: 1.0 years * hardness 2.25 (always-square helper) * 60 = 135
        assert!((state.environment.season_length - 135.0).abs() < 0.1);
        // SeasonText from pre-square 1.5 → very hard; operational hardness 1.6² after +0.1
        assert_eq!(state.environment.season_text, "A very hard  Winter");
        assert!((state.environment.season_hardness - 1.6 * 1.6).abs() < 1e-3);
    }

    #[test]
    fn mid_season_duration_change_shortens_boundary() {
        let mut state = empty_state();
        state.environment.season_length = 450.0;
        state.environment.season_elapsed = 400.0;
        state.season_duration_base_secs = 450.0;

        let live = ServerConfig {
            season_duration_years: 1.0, // → 60 s
            ..Default::default()
        }
        .live_settings();
        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"season_length_secs"));
        assert!((state.environment.season_length - 60.0).abs() < 0.01);
        assert!((state.season_duration_base_secs - 60.0).abs() < 0.01);
        // elapsed clamped below new length so next tick does not multi-roll
        assert!(state.environment.season_elapsed < state.environment.season_length);
        assert!(state.environment.season_elapsed > 50.0);
    }

    #[test]
    fn is_hard_season_winter_summer() {
        assert!(is_hard_season(Season::Winter));
        assert!(is_hard_season(Season::Summer));
        assert!(!is_hard_season(Season::Spring));
        assert!(!is_hard_season(Season::Autumn));
    }

    #[test]
    fn apply_live_settings_lockpick_knobs() {
        let mut state = empty_state();
        assert_eq!(
            state.lockpick_settings,
            crate::locks::LockpickSettings::default()
        );

        let live = ServerConfig {
            lockpick_success_chance: 40.0,
            lockpick_fail_chance: 25.0,
            lockpick_exhaustion_cost: 0.5,
            lockpick_coin_cost: 3.0,
            ..Default::default()
        }
        .live_settings();

        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"lockpick_success_chance"));
        assert!(report.keys.contains(&"lockpick_fail_chance"));
        assert!(report.keys.contains(&"lockpick_exhaustion_cost"));
        assert!(report.keys.contains(&"lockpick_coin_cost"));
        assert!((state.lockpick_settings.success_chance - 40.0).abs() < f32::EPSILON);
        assert!((state.lockpick_settings.fail_chance - 25.0).abs() < f32::EPSILON);
        assert!((state.lockpick_settings.exhaustion_cost - 0.5).abs() < f32::EPSILON);
        assert!((state.lockpick_settings.coin_cost - 3.0).abs() < f32::EPSILON);

        // Idempotent
        let report2 = apply_live_settings(&mut state, &live);
        assert!(!report2.keys.iter().any(|k| k.starts_with("lockpick_")));
    }

    /// Hot-reload lockpick knobs (all four differ from defaults) + female half exh.
    // Haxe: ServerSettings.Lockpick* after ReadServerSettings + LockPick isFemale
    #[test]
    fn apply_live_settings_lockpick_exhaustion_female_half() {
        use crate::locks::{lockpick_settings_for_player, try_lockpick, LockpickOutcome};

        let mut state = empty_state();
        // All four differ from LockpickSettings::default() (5/10/3/1) so report lists them.
        let live = ServerConfig {
            lockpick_success_chance: 7.0,
            lockpick_fail_chance: 12.0,
            lockpick_exhaustion_cost: 6.0,
            lockpick_coin_cost: 2.0,
            ..Default::default()
        }
        .live_settings();
        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"lockpick_success_chance"));
        assert!(report.keys.contains(&"lockpick_fail_chance"));
        assert!(report.keys.contains(&"lockpick_exhaustion_cost"));
        assert!(report.keys.contains(&"lockpick_coin_cost"));
        assert!((state.lockpick_settings.exhaustion_cost - 6.0).abs() < 1e-5);

        let for_f = lockpick_settings_for_player(&state.lockpick_settings, true);
        assert!((for_f.exhaustion_cost - 3.0).abs() < 1e-5);
        match try_lockpick(5.0, 0.0, 20.0, true, &state.lockpick_settings, 862, 0.5) {
            LockpickOutcome::Failed {
                exhaustion_after, ..
            } => assert!((exhaustion_after - 3.0).abs() < 1e-5),
            o => panic!("expected female Failed mid-roll, got {o:?}"),
        }
    }

    /// SETTINGS-FIELD-MAP: gameplay knobs apply + idempotent second pass.
    // Haxe: ServerSettings.FoodUsePerSecond etc. after readFromFile
    #[test]
    fn apply_live_settings_gameplay_knobs() {
        let mut state = empty_state();
        assert!((state.gameplay.food_use_per_second - 0.10).abs() < f32::EPSILON);
        assert!((state.gameplay.yum_bonus - 5.0).abs() < f32::EPSILON);

        let live = ServerConfig {
            food_use_per_second: 0.25,
            healing_per_second: 0.3,
            ageing_seconds_per_year: 30.0,
            initial_player_move_speed: 5.0,
            speed_factor: 1.25,
            yum_bonus: 7.0,
            chance_for_offspring: 0.001,
            chance_for_animal_dying: 0.002,
            hungry_work_cost: 12.0,
            birth_prestige_factor: 0.2,
            ally_strength_too_low_for_pickup: 0.8,
            time_confirm_new_follower: 5.0,
            hire_cost: 25.0,
            hire_cost_increase_per_person: 12.0,
            prestige_cost_per_damage_for_ally: 2.0,
            prestige_cost_per_damage_for_child: 2.0,
            prestige_cost_per_damage_for_elderly: 3.0,
            prestige_cost_per_damage_for_close_relatives: 1.5,
            prestige_cost_per_damage_for_women_without_weapon: 1.25,
            food_factor: 0.6,
            food_factor_eaten_less_than_one_percent: 3.0,
            yum_food_restore: 0.5,
            loved_food_restore: 0.25,
            yum_new_craving_chance: 0.5,
            food_reduction_per_eating: 2.0,
            food_reduction_faktor_for_eating_meh: 0.4,
            health_lost_when_eating_meh: 1.0,
            health_lost_when_eating_super_meh: 3.0,
            // C-SS-TAIL-KNOBS
            food_reduction_faktor_for_eating_high_quality: 0.55,
            grown_up_food_store_max: 25.0,
            // C-SS-AGE-FOOD
            new_born_food_store_max: 6.0,
            old_age_food_store_max: 12.0,
            min_biome_speed_factor: 0.15,
            hitpoints_speed_factor: 4.0,
            combat_reputation_restore_per_year: 3.5,
            // C-SS-MORE-KNOBS
            exhaustion_healing_factor: 2.0,
            wound_damage_factor: 1.5,
            wound_healing_factor: 2.5,
            // C-SS-MALE-HEAL
            exhaustion_healing_for_male_factor: 2.4,
            // C-SS-TEMP-HEAL
            temperature_hits_damage_factor: 0.75,
            temperature_exhaustion_damage_factor: 0.4,
            max_movement_quad_jump_distance_before_force: 9.0,
            food_restore_factor_while_feeding: 12.0,
            max_has_eaten_for_next_generation: 6.0,
            has_eaten_reduction_for_next_generation: 0.5,
            // WALLET-COINS
            coins_on_wounding_factor: 0.75,
            // C-SS-MORE-BATCH3
            combat_exhaustion_cost_per_attack: 0.25,
            min_age_to_eat: 4.0,
            max_child_age_for_breast_feeding: 8.0,
            ally_considered_close: 3.0,
            min_movement_age_in_sec: 20.0,
            // C-SS-MORE-BATCH4
            cursed_receive_damage_factor: 1.5,
            cursed_make_damage_factor: 0.25,
            pickup_baby_max_distance: 2.5,
            inherit_coins_factor: 0.5,
            min_age_fertile: 12.0,
            max_age_fertile: 50.0,
            // C-SS-MORE-BATCH5
            weapon_cooldown_factor: 0.25,
            weapon_cooldown_factor_if_wounding: 4.0,
            close_enemy_with_weapon_speed_factor: 0.5,
            exhaustion_on_jump: 0.1,
            hungry_work_heat: 0.004,
            ai_speed_factor_serf: 0.7,
            ai_speed_factor_commoner: 0.85,
            ai_speed_factor_noble: 1.1,
            ..Default::default()
        }
        .live_settings();

        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"food_use_per_second"));
        assert!(report.keys.contains(&"healing_per_second"));
        assert!(report.keys.contains(&"initial_player_move_speed"));
        assert!(report.keys.contains(&"yum_bonus"));
        assert!(report.keys.contains(&"chance_for_offspring"));
        assert!(report.keys.contains(&"hungry_work_cost"));
        assert!(report.keys.contains(&"birth_prestige_factor"));
        assert!((state.gameplay.food_use_per_second - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.initial_player_move_speed - 5.0).abs() < f32::EPSILON);
        assert!((state.gameplay.yum_bonus - 7.0).abs() < f32::EPSILON);
        assert!((state.gameplay.chance_for_animal_dying - 0.002).abs() < 1e-9);
        // FOLLOW-HIRE-DELAY
        assert!((state.gameplay.time_confirm_new_follower - 5.0).abs() < f32::EPSILON);
        assert!((state.gameplay.hire_cost - 25.0).abs() < f32::EPSILON);
        assert!(report.keys.contains(&"time_confirm_new_follower"));
        assert!(report.keys.contains(&"hire_cost"));
        // C-SS-MORE PrestigeCost* (ally + non-ally)
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_ally"));
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_child"));
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_elderly"));
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_close_relatives"));
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_women_without_weapon"));
        assert!((state.gameplay.prestige_cost_per_damage_for_child - 2.0).abs() < f32::EPSILON);
        assert!((state.gameplay.prestige_cost_per_damage_for_elderly - 3.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.prestige_cost_per_damage_for_close_relatives - 1.5).abs() < f32::EPSILON
        );
        assert!(
            (state.gameplay.prestige_cost_per_damage_for_women_without_weapon - 1.25).abs()
                < f32::EPSILON
        );
        let pcf = state.gameplay.prestige_cost_factors();
        assert!((pcf.child - 2.0).abs() < f32::EPSILON);
        assert!((pcf.ally - 2.0).abs() < f32::EPSILON);
        // C-SS-FULL-TABLE FoodFactor + restore + reduction family
        assert!(report.keys.contains(&"food_factor"));
        assert!(report.keys.contains(&"yum_food_restore"));
        assert!(report.keys.contains(&"loved_food_restore"));
        assert!(report.keys.contains(&"yum_new_craving_chance"));
        assert!(report.keys.contains(&"food_reduction_per_eating"));
        assert!(report.keys.contains(&"health_lost_when_eating_super_meh"));
        assert!((state.gameplay.food_factor - 0.6).abs() < f32::EPSILON);
        assert!((state.gameplay.yum_food_restore - 0.5).abs() < f32::EPSILON);
        assert!((state.gameplay.food_factor_eaten_less_than_one_percent - 3.0).abs() < f32::EPSILON);
        assert!((state.gameplay.loved_food_restore - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.yum_new_craving_chance - 0.5).abs() < f32::EPSILON);
        assert!((state.gameplay.food_reduction_per_eating - 2.0).abs() < f32::EPSILON);
        assert!((state.gameplay.food_reduction_faktor_for_eating_meh - 0.4).abs() < f32::EPSILON);
        assert!((state.gameplay.health_lost_when_eating_meh - 1.0).abs() < f32::EPSILON);
        assert!((state.gameplay.health_lost_when_eating_super_meh - 3.0).abs() < f32::EPSILON);
        // C-SS-TAIL-KNOBS
        assert!(report.keys.contains(&"grown_up_food_store_max"));
        assert!(report.keys.contains(&"min_biome_speed_factor"));
        assert!(report.keys.contains(&"hitpoints_speed_factor"));
        assert!(report.keys.contains(&"food_reduction_faktor_for_eating_high_quality"));
        assert!(report.keys.contains(&"combat_reputation_restore_per_year"));
        assert!((state.gameplay.grown_up_food_store_max - 25.0).abs() < f32::EPSILON);
        assert!((state.gameplay.min_biome_speed_factor - 0.15).abs() < f32::EPSILON);
        assert!((state.gameplay.hitpoints_speed_factor - 4.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.food_reduction_faktor_for_eating_high_quality - 0.55).abs()
                < f32::EPSILON
        );
        assert!(
            (state.gameplay.combat_reputation_restore_per_year - 3.5).abs() < f32::EPSILON
        );
        // C-SS-AGE-FOOD
        assert!(report.keys.contains(&"new_born_food_store_max"));
        assert!(report.keys.contains(&"old_age_food_store_max"));
        assert!((state.gameplay.new_born_food_store_max - 6.0).abs() < f32::EPSILON);
        assert!((state.gameplay.old_age_food_store_max - 12.0).abs() < f32::EPSILON);
        let cap = state.gameplay.food_store_max_knobs();
        assert!((cap.newborn - 6.0).abs() < f32::EPSILON);
        assert!((cap.old_age - 12.0).abs() < f32::EPSILON);
        assert!((cap.grown_up - 25.0).abs() < f32::EPSILON);
        let bands = state.gameplay.food_factor_eaten_bands();
        assert!((bands.less_than_one_percent - 3.0).abs() < f32::EPSILON);
        let eat = state.gameplay.eat_live_knobs();
        assert!((eat.food_reduction_per_eating - 2.0).abs() < f32::EPSILON);
        let restore = state.gameplay.yum_restore_knobs();
        assert!((restore.loved_food_restore - 0.25).abs() < f32::EPSILON);
        // C-SS-MORE-KNOBS
        assert!(report.keys.contains(&"exhaustion_healing_factor"));
        assert!(report.keys.contains(&"wound_damage_factor"));
        assert!(report.keys.contains(&"wound_healing_factor"));
        // C-SS-MALE-HEAL
        assert!(report.keys.contains(&"exhaustion_healing_for_male_factor"));
        // C-SS-TEMP-HEAL
        assert!(report.keys.contains(&"temperature_hits_damage_factor"));
        assert!(report.keys.contains(&"temperature_exhaustion_damage_factor"));
        assert!(report.keys.contains(&"max_movement_quad_jump_distance_before_force"));
        assert!(report.keys.contains(&"food_restore_factor_while_feeding"));
        assert!(report.keys.contains(&"max_has_eaten_for_next_generation"));
        assert!(report.keys.contains(&"has_eaten_reduction_for_next_generation"));
        assert!((state.gameplay.exhaustion_healing_factor - 2.0).abs() < f32::EPSILON);
        assert!((state.gameplay.wound_damage_factor - 1.5).abs() < f32::EPSILON);
        assert!((state.gameplay.wound_healing_factor - 2.5).abs() < f32::EPSILON);
        assert!((state.gameplay.exhaustion_healing_for_male_factor - 2.4).abs() < f32::EPSILON);
        assert!((state.gameplay.temperature_hits_damage_factor - 0.75).abs() < f32::EPSILON);
        assert!((state.gameplay.temperature_exhaustion_damage_factor - 0.4).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.max_movement_quad_jump_distance_before_force - 9.0).abs()
                < f32::EPSILON
        );
        assert!((state.gameplay.food_restore_factor_while_feeding - 12.0).abs() < f32::EPSILON);
        assert!((state.gameplay.max_has_eaten_for_next_generation - 6.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.has_eaten_reduction_for_next_generation - 0.5).abs() < f32::EPSILON
        );
        assert!((state.gameplay.max_move_quad_jump_before_force() - 9.0).abs() < 1e-9);
        let (red, max_he) = state.gameplay.inherit_eaten_knobs();
        assert!((red - 0.5).abs() < f32::EPSILON);
        assert!((max_he - 6.0).abs() < f32::EPSILON);
        // WALLET-COINS
        assert!(report.keys.contains(&"coins_on_wounding_factor"));
        assert!((state.gameplay.coins_on_wounding_factor - 0.75).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH3
        assert!(report.keys.contains(&"combat_exhaustion_cost_per_attack"));
        assert!(report.keys.contains(&"min_age_to_eat"));
        assert!(report.keys.contains(&"max_child_age_for_breast_feeding"));
        assert!(report.keys.contains(&"ally_considered_close"));
        assert!(report.keys.contains(&"min_movement_age_in_sec"));
        assert!((state.gameplay.combat_exhaustion_cost_per_attack - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.min_age_to_eat - 4.0).abs() < f32::EPSILON);
        assert!((state.gameplay.max_child_age_for_breast_feeding - 8.0).abs() < f32::EPSILON);
        assert!((state.gameplay.ally_considered_close - 3.0).abs() < f32::EPSILON);
        assert!((state.gameplay.min_movement_age_in_sec - 20.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH4
        assert!(report.keys.contains(&"cursed_receive_damage_factor"));
        assert!(report.keys.contains(&"cursed_make_damage_factor"));
        assert!(report.keys.contains(&"pickup_baby_max_distance"));
        assert!(report.keys.contains(&"inherit_coins_factor"));
        assert!(report.keys.contains(&"min_age_fertile"));
        assert!(report.keys.contains(&"max_age_fertile"));
        assert!((state.gameplay.cursed_receive_damage_factor - 1.5).abs() < f32::EPSILON);
        assert!((state.gameplay.cursed_make_damage_factor - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.pickup_baby_max_distance - 2.5).abs() < f32::EPSILON);
        assert!((state.gameplay.inherit_coins_factor - 0.5).abs() < f32::EPSILON);
        assert!((state.gameplay.min_age_fertile - 12.0).abs() < f32::EPSILON);
        assert!((state.gameplay.max_age_fertile - 50.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH5
        assert!(report.keys.contains(&"weapon_cooldown_factor"));
        assert!(report.keys.contains(&"weapon_cooldown_factor_if_wounding"));
        assert!(report.keys.contains(&"close_enemy_with_weapon_speed_factor"));
        assert!(report.keys.contains(&"exhaustion_on_jump"));
        assert!(report.keys.contains(&"hungry_work_heat"));
        assert!(report.keys.contains(&"ai_speed_factor_serf"));
        assert!(report.keys.contains(&"ai_speed_factor_commoner"));
        assert!(report.keys.contains(&"ai_speed_factor_noble"));
        assert!((state.gameplay.weapon_cooldown_factor - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.weapon_cooldown_factor_if_wounding - 4.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.close_enemy_with_weapon_speed_factor - 0.5).abs() < f32::EPSILON
        );
        assert!((state.gameplay.exhaustion_on_jump - 0.1).abs() < f32::EPSILON);
        assert!((state.gameplay.hungry_work_heat - 0.004).abs() < f32::EPSILON);
        assert!((state.gameplay.ai_speed_factor_serf - 0.7).abs() < f32::EPSILON);
        assert!((state.gameplay.ai_speed_factor_commoner - 0.85).abs() < f32::EPSILON);
        assert!((state.gameplay.ai_speed_factor_noble - 1.1).abs() < f32::EPSILON);
        let (wcd, wcd_w) = state.gameplay.weapon_cooldown_knobs();
        assert!((wcd - 0.25).abs() < f32::EPSILON);
        assert!((wcd_w - 4.0).abs() < f32::EPSILON);

        let report2 = apply_live_settings(&mut state, &live);
        assert!(!report2.keys.iter().any(|k| {
            matches!(
                *k,
                "food_use_per_second"
                    | "yum_bonus"
                    | "hungry_work_cost"
                    | "chance_for_offspring"
                    | "initial_player_move_speed"
                    | "grown_up_food_store_max"
                    | "new_born_food_store_max"
                    | "old_age_food_store_max"
                    | "combat_reputation_restore_per_year"
                    | "exhaustion_healing_factor"
                    | "wound_damage_factor"
                    | "wound_healing_factor"
                    | "exhaustion_healing_for_male_factor"
                    | "temperature_hits_damage_factor"
                    | "temperature_exhaustion_damage_factor"
                    | "max_movement_quad_jump_distance_before_force"
                    | "food_restore_factor_while_feeding"
                    | "max_has_eaten_for_next_generation"
                    | "has_eaten_reduction_for_next_generation"
                    | "coins_on_wounding_factor"
                    | "combat_exhaustion_cost_per_attack"
                    | "min_age_to_eat"
                    | "max_child_age_for_breast_feeding"
                    | "ally_considered_close"
                    | "min_movement_age_in_sec"
                    | "cursed_receive_damage_factor"
                    | "cursed_make_damage_factor"
                    | "pickup_baby_max_distance"
                    | "inherit_coins_factor"
                    | "min_age_fertile"
                    | "max_age_fertile"
                    | "weapon_cooldown_factor"
                    | "weapon_cooldown_factor_if_wounding"
                    | "close_enemy_with_weapon_speed_factor"
                    | "exhaustion_on_jump"
                    | "hungry_work_heat"
                    | "ai_speed_factor_serf"
                    | "ai_speed_factor_commoner"
                    | "ai_speed_factor_noble"
            )
        }));
    }

    /// C-SS-TAIL-KNOBS + C-SS-AGE-FOOD: capacity / speed / restore knobs Live.
    #[test]
    fn intentional_non_live_module_consts_documented() {
        use ol_config::{find_critical, SettingsHome};
        let food = find_critical("FoodUsePerSecond").expect("food live");
        assert_eq!(food.home, SettingsHome::Live);
        // C-SS-FULL-TABLE: FoodFactor + Yum*Restore + FoodReduction*/HealthLost* Live
        let ff = find_critical("FoodFactor").expect("FoodFactor");
        assert_eq!(ff.home, SettingsHome::Live);
        let yfr = find_critical("YumFoodRestore").expect("YumFoodRestore");
        assert_eq!(yfr.home, SettingsHome::Live);
        let loved = find_critical("LovedFoodRestore").expect("LovedFoodRestore");
        assert_eq!(loved.home, SettingsHome::Live);
        let ync = find_critical("YumNewCravingChance").expect("YumNewCravingChance");
        assert_eq!(ync.home, SettingsHome::Live);
        let fr = find_critical("FoodReductionPerEating").expect("FoodReductionPerEating");
        assert_eq!(fr.home, SettingsHome::Live);
        let hl = find_critical("HealthLostWhenEatingMeh").expect("HealthLostWhenEatingMeh");
        assert_eq!(hl.home, SettingsHome::Live);
        // C-SS-MORE: all PrestigeCost* Live
        for name in [
            "PrestigeCostPerDamageForAlly",
            "PrestigeCostPerDamageForChild",
            "PrestigeCostPerDamageForElderly",
            "PrestigeCostPerDamageForCloseRelatives",
            "PrestigeCostPerDamageForWomenWithoutWeapon",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
        // C-SS-TAIL-KNOBS + C-SS-AGE-FOOD
        for name in [
            "GrownUpFoodStoreMax",
            "NewBornFoodStoreMax",
            "OldAgeFoodStoreMax",
            "MinBiomeSpeedFactor",
            "HitpointsSpeedFactor",
            "FoodReductionFaktorForEatingHighQuailitFood",
            "CombatReputationRestorePerYear",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
        // C-SS-MORE-KNOBS + C-SS-MALE-HEAL + C-SS-TEMP-HEAL
        for name in [
            "ExhaustionHealingFactor",
            "WoundDamageFactor",
            "WoundHealingFactor",
            "ExhaustionHealingForMaleFaktor",
            "TemperatureHitsDamageFactor",
            "TemperatureExhaustionDamageFactor",
            "MaxMovementQuadJumpDistanceBeforeForce",
            "FoodRestoreFactorWhileFeeding",
            "MaxHasEatenForNextGeneration",
            "HasEatenReductionForNextGeneration",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
        // C-SS-MORE-BATCH4
        for name in [
            "CursedReceiveDamageFactor",
            "CursedMakeDamageFactor",
            "PickupBabyMaxDistance",
            "InheritCoinsFactor",
            "MinAgeFertile",
            "MaxAgeFertile",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
        // C-SS-MORE-BATCH5
        for name in [
            "WeaponCoolDownFactor",
            "WeaponCoolDownFactorIfWounding",
            "CloseEnemyWithWeaponSpeedFactor",
            "ExhaustionOnJump",
            "HungryWorkHeat",
            "AISpeedFactorSerf",
            "AISpeedFactorCommoner",
            "AISpeedFactorNoble",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
    }
}
