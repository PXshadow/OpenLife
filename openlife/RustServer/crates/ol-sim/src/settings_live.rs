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

use crate::ai_path_reach::BlockedByAiShare;
use crate::environment::Season;
use crate::object_counts_share::ObjectCountsShare;
use crate::war_posse_persist::WarPosseShare;
use crate::world_food_stats::WorldFoodShare;
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
    /// Haxe `BiomeAnimalHitChance` (DoDamage biome-animal miss gate).
    // MOSQUITO-MAPCHANCE
    pub biome_animal_hit_chance: f32,
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
    /// Haxe `StartingEveAge` — Eve/Adam spawn `age` / `trueAge` (years).
    // Haxe: ServerSettings.StartingEveAge = 14
    // SETTINGS-LONG-TAIL
    pub starting_eve_age: f32,
    /// Haxe `EveOrAdamBirthChance` — synthetic/NPC Eve vs mother spawn roll.
    // Haxe: ServerSettings.EveOrAdamBirthChance = 0.025
    // SETTINGS-LONG-TAIL
    pub eve_or_adam_birth_chance: f32,
    /// Haxe `SpawnAiAsEve` — AIs/NPCs may take that Eve roll even if a mother exists.
    // Haxe: ServerSettings.SpawnAiAsEve = false
    // SETTINGS-LONG-TAIL
    pub spawn_ai_as_eve: bool,
    /// Humans may spawn as children of AI mothers. Default false.
    pub allow_humans_born_to_ais: bool,
    /// Haxe `MaxPlayersBeforeStartingAsChild` — living count allowing AI↔human Eve cross.
    // Haxe: ServerSettings.MaxPlayersBeforeStartingAsChild = 0
    // SETTINGS-LONG-TAIL
    pub max_players_before_starting_as_child: i32,
    /// Haxe `ObjDecayChance` — long-term object decay roll.
    // Haxe: ServerSettings.ObjDecayChance = 0.00005
    // SETTINGS-LONG-TAIL
    pub obj_decay_chance: f32,
    /// Haxe `FloorDecayChance` — long-term floor decay roll.
    // Haxe: ServerSettings.FloorDecayChance = 0.00001
    // SETTINGS-LONG-TAIL
    pub floor_decay_chance: f32,
    /// Haxe `ObjRespawnChance` — RespawnObjects per empty original tile.
    // Haxe: ServerSettings.ObjRespawnChance = 0.00006
    // SETTINGS-LONG-TAIL
    pub obj_respawn_chance: f32,
    /// Haxe `GrowBackPlantsIncreaseIfLowPopulation` — spring original-plant boost if current < original/2.
    // Haxe: ServerSettings.GrowBackPlantsIncreaseIfLowPopulation = 2
    // SETTINGS-LONG-TAIL
    pub grow_back_plants_increase_if_low_population: f32,
    /// Haxe `GrowBackOriginalPlantsFactor` — spring original-plant roll multiplier.
    // Haxe: ServerSettings.GrowBackOriginalPlantsFactor = 0.02
    // SETTINGS-LONG-TAIL
    pub grow_back_original_plants_factor: f32,
    /// Haxe `GrowNewPlantsFromExistingFactor` — offspring per season per living plant.
    // Haxe: ServerSettings.GrowNewPlantsFromExistingFactor = 0.05
    // SETTINGS-LONG-TAIL
    pub grow_new_plants_from_existing_factor: f32,
    /// Haxe `SpringWildFoodRegrowChance` — per-season spring chance recompute.
    // Haxe: ServerSettings.SpringWildFoodRegrowChance = 1
    // SETTINGS-LONG-TAIL
    pub spring_wild_food_regrow_chance: f32,
    /// Haxe `WinterWildFoodDecayChance` — per-season winter chance recompute.
    // Haxe: ServerSettings.WinterWildFoodDecayChance = 1.5
    // SETTINGS-LONG-TAIL
    pub winter_wild_food_decay_chance: f32,
    /// Haxe `HotSeasonTemperatureFactor` — scale positive season impact on tile temp.
    // Haxe: ServerSettings.HotSeasonTemperatureFactor = 0.75
    // SETTINGS-LONG-TAIL
    pub hot_season_temperature_factor: f32,
    /// Haxe `ColdSeasonTemperatureFactor` — scale negative season impact on tile temp.
    // Haxe: ServerSettings.ColdSeasonTemperatureFactor = 0.75
    // SETTINGS-LONG-TAIL
    pub cold_season_temperature_factor: f32,
    /// Haxe `CursedGraveTime` — hours extra decay per overflowing sharp stone.
    // Haxe: ServerSettings.CursedGraveTime = 12
    // SETTINGS-LONG-TAIL
    pub cursed_grave_time: f32,
    /// Haxe `AnimalDecayFactor` — long-term decayFactor for horse-cart / domestic / wolf ids.
    // Haxe: ServerSettings.AnimalDecayFactor = 0.05
    // SETTINGS-LONG-TAIL
    pub animal_decay_factor: f32,
    /// Haxe `ObjDecayFactorForPermanentObjs`.
    // Haxe: ServerSettings.ObjDecayFactorForPermanentObjs = 0.2
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_permanent: f32,
    /// Haxe `ObjDecayFactorForFood`.
    // Haxe: ServerSettings.ObjDecayFactorForFood = 2
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_food: f32,
    /// Haxe `ObjDecayFactorForClothing`.
    // Haxe: ServerSettings.ObjDecayFactorForClothing = 2
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_clothing: f32,
    /// Haxe `ObjDecayFactorForWalls`.
    // Haxe: ServerSettings.ObjDecayFactorForWalls = 0.2
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_walls: f32,
    /// Haxe `ObjDecayFactorPerTechLevel`.
    // Haxe: ServerSettings.ObjDecayFactorPerTechLevel = 10
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_per_tech_level: f32,
    /// Haxe `DecayFactorInDeepWater`.
    // Haxe: ServerSettings.DecayFactorInDeepWater = 5
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_deep_water: f32,
    /// Haxe `DecayFactorInMountain`.
    // Haxe: ServerSettings.DecayFactorInMountain = 3
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_mountain: f32,
    /// Haxe `DecayFactorInWalkableWater`.
    // Haxe: ServerSettings.DecayFactorInWalkableWater = 2
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_walkable_water: f32,
    /// Haxe `DecayFactorInJungle`.
    // Haxe: ServerSettings.DecayFactorInJungle = 2
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_jungle: f32,
    /// Haxe `DecayFactorInSwamp`.
    // Haxe: ServerSettings.DecayFactorInSwamp = 2
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_swamp: f32,
    /// Haxe `ScoreFactor` — EMA weight of this life prestige into account score.
    // Haxe: ServerSettings.ScoreFactor = 0.2
    // SETTINGS-LONG-TAIL
    pub score_factor: f32,
    /// Haxe `AncestorPrestigeFactor`.
    pub ancestor_prestige_factor: f32,
    /// Haxe `DisplayScoreFactor`.
    pub display_score_factor: f32,
    /// Haxe `DisplayScoreOn`.
    // SETTINGS-KNOB-TAIL
    pub display_score_on: bool,
    /// Haxe `MaxCoinsPerChest`.
    // SETTINGS-KNOB-TAIL
    pub max_coins_per_chest: i32,
    /// Haxe `MaxCoinsPerPouch`.
    // SETTINGS-KNOB-TAIL
    pub max_coins_per_pouch: i32,
    /// Haxe `ChanceForFemaleChild`.
    // SETTINGS-KNOB-TAIL
    pub chance_for_female_child: f32,
    /// Haxe `ChanceForOtherChildColor`.
    // SETTINGS-KNOB-TAIL
    pub chance_for_other_child_color: f32,
    /// Haxe `ChanceForOtherChildColorIfCloseToWrongSpecialBiome`.
    // SETTINGS-KNOB-TAIL
    pub chance_for_other_child_color_if_close_to_wrong_special_biome: f32,
    /// Haxe `LittleKidsPerMother`.
    // SETTINGS-KNOB-TAIL
    pub little_kids_per_mother: i32,
    /// Haxe `NewChildExhaustionForMother`.
    // SETTINGS-KNOB-TAIL
    pub new_child_exhaustion_for_mother: f32,
    /// Haxe `AiMotherBirthMaliForHumanChild`.
    // SETTINGS-KNOB-TAIL
    pub ai_mother_birth_mali_for_human_child: f32,
    /// Haxe `HumanMotherBirthMaliForAiChild`.
    // SETTINGS-KNOB-TAIL
    pub human_mother_birth_mali_for_ai_child: f32,
    /// Haxe `SpwanAtLastDead` (typo Spwan).
    // SETTINGS-KNOB-TAIL
    pub spawn_at_last_dead: bool,
    /// Haxe `TemperatureOwnTileRate`.
    // SETTINGS-KNOB-TAIL
    pub temperature_own_tile_rate: f32,
    /// Haxe `TemperatureBalanceRate`.
    // SETTINGS-KNOB-TAIL
    pub temperature_balance_rate: f32,
    /// Haxe `TemperatureLocalHeatFactor`.
    // SETTINGS-KNOB-TAIL
    pub temperature_local_heat_factor: f32,
    /// Haxe `AverageSeasonTemperatureImpact`.
    // SETTINGS-KNOB-TAIL
    pub average_season_temperature_impact: f32,
    /// Haxe `AiTotalScoreFactor`.
    pub ai_total_score_factor: f32,
    /// Haxe `OldGraveDecayMali`.
    pub old_grave_decay_mali: f32,
    /// Haxe `CursedGraveMali`.
    pub cursed_grave_mali: f32,
    /// Haxe `MaxDistanceToBeConsideredAsClose`.
    pub max_distance_close: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForMapChanges`.
    pub max_distance_map_changes: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSay`.
    pub max_distance_say: i32,
    /// Haxe `MaxDistanceToAutoExileAttacker` (quad compare).
    pub max_distance_auto_exile_attacker: i32,
    /// Haxe `SendMoveEveryXTicks` (`-1` = disabled).
    // Haxe: ServerSettings.SendMoveEveryXTicks = -1
    // SETTINGS-LONG-TAIL
    pub send_move_every_x_ticks: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCoseForMovement` (typo Cose) — PM fan.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCoseForMovement = 30
    // SETTINGS-LONG-TAIL
    pub max_distance_cose_for_movement: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSayAi` — AI sayHelper hear radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi = 20
    // SETTINGS-LONG-TAIL
    pub max_distance_say_ai: f32,
    /// Haxe `SpeedWithBothShoes`.
    // Haxe: ServerSettings.SpeedWithBothShoes = 1.1
    // SETTINGS-LONG-TAIL
    pub speed_with_both_shoes: f32,
    /// Haxe `AgingFactorWhileStarvingToDeath`.
    // Haxe: ServerSettings.AgingFactorWhileStarvingToDeath = 0.5
    // SETTINGS-LONG-TAIL
    pub aging_factor_while_starving: f32,
    /// Haxe `GrownUpAge`.
    // Haxe: ServerSettings.GrownUpAge = 14
    // SETTINGS-LONG-TAIL
    pub grown_up_age: f32,
    /// Haxe `FoodUseChildFaktor`.
    // Haxe: ServerSettings.FoodUseChildFaktor = 1
    // SETTINGS-LONG-TAIL
    pub food_use_child_faktor: f32,
    /// Haxe `AIFoodUseFactorSerf`.
    // Haxe: ServerSettings.AIFoodUseFactorSerf = 0.8
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_serf: f32,
    /// Haxe `AIFoodUseFactorCommoner`.
    // Haxe: ServerSettings.AIFoodUseFactorCommoner = 0.9
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_commoner: f32,
    /// Haxe `AIFoodUseFactorNoble` (Noble only; King/Emperor stay 1.0).
    // Haxe: ServerSettings.AIFoodUseFactorNoble = 1
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_noble: f32,
    /// Haxe `EveFoodUseFactor`.
    // Haxe: ServerSettings.EveFoodUseFactor = 1
    // SETTINGS-LONG-TAIL
    pub eve_food_use_factor: f32,
    /// Haxe `AgingFactorHumanBornToAi`.
    pub aging_factor_human_born_to_ai: f32,
    /// Haxe `AgingFactorAiBornToHuman`.
    pub aging_factor_ai_born_to_human: f32,
    /// Haxe `EveDamageFactor`.
    // Haxe: ServerSettings.EveDamageFactor = 1
    // SETTINGS-LONG-TAIL
    pub eve_damage_factor: f32,
    /// Haxe `TargetWoundedDamageFactor`.
    // Haxe: ServerSettings.TargetWoundedDamageFactor = 0.2
    // SETTINGS-LONG-TAIL
    pub target_wounded_damage_factor: f32,
    /// Haxe `MaleDamageFactor`.
    // Haxe: ServerSettings.MaleDamageFactor = 1.2
    // SETTINGS-LONG-TAIL
    pub male_damage_factor: f32,
    /// Haxe `AnimalDamageFactor`.
    // Haxe: ServerSettings.AnimalDamageFactor = 1.5
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor: f32,
    /// Haxe `AnimalDamageFactorInWinter`.
    // Haxe: ServerSettings.AnimalDamageFactorInWinter = 2
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor_in_winter: f32,
    /// Haxe `AnimalDamageFactorIfAttacked`.
    // Haxe: ServerSettings.AnimalDamageFactorIfAttacked = 1.5
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor_if_attacked: f32,
    /// Haxe `WeaponDamageFactor`.
    // Haxe: ServerSettings.WeaponDamageFactor = 1
    // SETTINGS-LONG-TAIL
    pub weapon_damage_factor: f32,
    /// Haxe `GraveBlockingDistance`.
    // Haxe: ServerSettings.GraveBlockingDistance = 40
    // SETTINGS-LONG-TAIL
    pub grave_blocking_distance: f32,
    /// Haxe `MaxPlayersBeforeActivatingGraveCurse`.
    // Haxe: ServerSettings.MaxPlayersBeforeActivatingGraveCurse = 0
    // SETTINGS-LONG-TAIL
    pub max_players_before_activating_grave_curse: i32,
    /// Haxe `MaxPlayersBeforeForbidTouchGrave`.
    // Haxe: ServerSettings.MaxPlayersBeforeForbidTouchGrave = 9999
    // GRAVE-TOUCH-PLAYERS
    pub max_players_before_forbid_touch_grave: i32,
    /// Haxe `CombatAngryTimeBeforeAttack`.
    // Haxe: ServerSettings.CombatAngryTimeBeforeAttack = 5
    // SETTINGS-LONG-TAIL
    pub combat_angry_time_before_attack: f32,
    /// Haxe `CombatAngryTimeMinimum` (angry recovery floor while killMode).
    pub combat_angry_time_minimum: f32,
    /// Haxe `ChanceForDomesticAnimalDyingFactor` (applied; Haxe line was unassigned).
    pub chance_for_domestic_animal_dying_factor: f32,
    /// Haxe `DoorIds` live table (empty → compiled default).
    pub door_ids: Vec<i32>,
    /// Haxe `AiIgnoredFloorIds` live table (empty → compiled default).
    pub ai_ignored_floor_ids: Vec<i32>,
    /// Haxe `Secret` for SAY `!S` (do not log the value).
    pub secret: String,
    /// Haxe `AllowDebugCommmands` — gates `DoDebugCommands` including `!S`.
    pub allow_debug_commands: bool,
    /// Haxe `DebugSayPlayerPosition` — LS world coords at feet on MOVE (debug).
    // Haxe: ServerSettings.DebugSayPlayerPosition = false
    pub debug_say_player_position: bool,
    /// Haxe `AiTimeToWaitIfCraftingFailed`.
    // Haxe: ServerSettings.AiTimeToWaitIfCraftingFailed = 15
    // SETTINGS-LONG-TAIL
    pub ai_time_to_wait_if_crafting_failed: f32,
    /// Haxe `AiMaxSearchRadius`.
    // Haxe: ServerSettings.AiMaxSearchRadius = 60
    // SETTINGS-LONG-TAIL
    pub ai_max_search_radius: i32,
    /// Haxe `AiMaxSearchIncrement`.
    // Haxe: ServerSettings.AiMaxSearchIncrement = 30
    // SETTINGS-LONG-TAIL
    pub ai_max_search_increment: i32,
    /// Haxe `AiIgnoreTimeTransitionsLongerThen`.
    // Haxe: ServerSettings.AiIgnoreTimeTransitionsLongerThen = 120
    // SETTINGS-LONG-TAIL
    pub ai_ignore_time_transitions_longer_then: f32,
    /// Haxe `AiMemoryMaxEntries`.
    // Haxe: ServerSettings.AiMemoryMaxEntries = 20
    // SOUL-LIVE-CAPS
    pub ai_memory_max_entries: i32,
    /// Haxe `AiChatMemoryMaxEntries`.
    // Haxe: ServerSettings.AiChatMemoryMaxEntries = 100
    // SOUL-LIVE-CAPS
    pub ai_chat_memory_max_entries: i32,
    /// Haxe `AlternativeOutcomePercentIncreasePerHit`.
    // Haxe: ServerSettings.AlternativeOutcomePercentIncreasePerHit = 10
    // SETTINGS-LONG-TAIL
    pub alternative_outcome_percent_increase_per_hit: f32,
    /// Haxe `AlternativeOutcomeHitsDecreaseOnSucess`.
    // Haxe: ServerSettings.AlternativeOutcomeHitsDecreaseOnSucess = 5
    // SETTINGS-LONG-TAIL
    pub alternative_outcome_hits_decrease_on_success: f32,
    /// Haxe `FortificationCosePerHit`.
    // Haxe: ServerSettings.FortificationCosePerHit = 1
    // TH-ALT-LIVE-KNOBS
    pub fortification_cost_per_hit: f32,
    /// Haxe `ReduceAgeNeededToPickupObjects`.
    // Haxe: ServerSettings.ReduceAgeNeededToPickupObjects = 10
    // MIN-PICKUP-AGE
    pub reduce_age_needed_to_pickup_objects: f32,
    /// Haxe `ChanceThatAnimalsCanPassBlockingBiome`.
    // Haxe: ServerSettings.ChanceThatAnimalsCanPassBlockingBiome = 0.03
    // SETTINGS-LONG-TAIL
    pub chance_animals_pass_blocking_biome: f32,
    /// Haxe `chancePreferredBiome`.
    // Haxe: ServerSettings.chancePreferredBiome = 0.8
    // SETTINGS-LONG-TAIL
    pub chance_preferred_biome: f32,
    /// Haxe `CloseGraveSpeedMali`.
    // Haxe: ServerSettings.CloseGraveSpeedMali = 0.9
    // SETTINGS-LONG-TAIL
    pub close_grave_speed_mali: f32,
    /// Haxe `TemperatureSpeedImpact`.
    // Haxe: ServerSettings.TemperatureSpeedImpact = 1
    // SETTINGS-LONG-TAIL
    pub temperature_speed_impact: f32,
    /// Haxe `MinSpeedReductionPerContainedObj`.
    // Haxe: ServerSettings.MinSpeedReductionPerContainedObj = 0.98
    // SETTINGS-LONG-TAIL
    pub min_speed_reduction_per_contained_obj: f32,
    /// Haxe `LovedFoodUseChance`.
    // Haxe: ServerSettings.LovedFoodUseChance = 0.5
    // SETTINGS-LONG-TAIL
    pub loved_food_use_chance: f32,
    /// Haxe `MaxAgeForAllowingClothAndPrickupFromOthers`.
    // Haxe: ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers = 10
    // SETTINGS-LONG-TAIL
    pub max_age_for_allowing_cloth_and_pickup_from_others: f32,
    /// Haxe `MaxAgeForAllowingDie`.
    // Haxe: ServerSettings.MaxAgeForAllowingDie = 2
    // SETTINGS-LONG-TAIL
    pub max_age_for_allowing_die: f32,
    /// Haxe `PrestigeCostForDie`.
    // Haxe: ServerSettings.PrestigeCostForDie = 0
    // SETTINGS-LONG-TAIL
    pub prestige_cost_for_die: f32,
    /// Haxe `StartingFamilyName` — DoNaming I AM found-new-family gate.
    // Haxe: ServerSettings.StartingFamilyName = "SNOW"
    // SETTINGS-LONG-TAIL
    pub starting_family_name: String,
    /// Haxe `StartingName` — DoNaming YOU ARE only if target first name is this.
    // Haxe: ServerSettings.StartingName = "SPOON"
    // SETTINGS-LONG-TAIL
    pub starting_name: String,
    /// Haxe `FoundFamilyNeededPrestige` — DoNaming I AM found-new prestige gate.
    // Haxe: ServerSettings.FoundFamilyNeededPrestige = 50
    // SETTINGS-LONG-TAIL
    pub found_family_needed_prestige: f32,
    /// Haxe `FoundFamilyCost` — coins required and subtracted on found-new family.
    // Haxe: ServerSettings.FoundFamilyCost = 10
    // SETTINGS-LONG-TAIL
    pub found_family_cost: f32,
    /// Haxe `FoundFamilyNeededFollowers` — DoNaming I AM found-new same-family follower gate.
    // Haxe: ServerSettings.FoundFamilyNeededFollowers = 4
    // SETTINGS-LONG-TAIL
    pub found_family_needed_followers: i32,
    /// Haxe `FoundFamilyBreakAllianceChance` — AI foundFamily I FOLLOW ME roll.
    // Haxe: ServerSettings.FoundFamilyBreakAllianceChance = 0.5
    // SETTINGS-LONG-TAIL
    pub found_family_break_alliance_chance: f32,
    /// Haxe `PickupExhaustionGain`.
    // Haxe: ServerSettings.PickupExhaustionGain = 0.2
    // SETTINGS-LONG-TAIL
    pub pickup_exhaustion_gain: f32,
    /// Haxe `PickupFeedingFoodRestore`.
    // Haxe: ServerSettings.PickupFeedingFoodRestore = 1.5
    // SETTINGS-LONG-TAIL
    pub pickup_feeding_food_restore: f32,
    /// Haxe `DeathWithFoodStoreMax`.
    // Haxe: ServerSettings.DeathWithFoodStoreMax = -0.1
    // SETTINGS-LONG-TAIL
    pub death_with_food_store_max: f32,
    /// Haxe `FoodStoreMaxReductionWhileStarvingToDeath`.
    // Haxe: ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath = 5
    // SETTINGS-LONG-TAIL
    pub food_store_max_reduction_while_starving: f32,
    /// Haxe `TemperatureReductionPerDrinking`.
    // Haxe: ServerSettings.TemperatureReductionPerDrinking = 0.5
    // SETTINGS-LONG-TAIL
    pub temperature_reduction_per_drinking: f32,
    /// Haxe `MaxStoredWater`.
    // Haxe: ServerSettings.MaxStoredWater = 1
    // SETTINGS-LONG-TAIL
    pub max_stored_water: f32,
    /// Haxe `MaxJumpsPerTenSec`.
    // Haxe: ServerSettings.MaxJumpsPerTenSec = 10
    // SETTINGS-LONG-TAIL
    pub max_jumps_per_ten_sec: f32,
    /// Haxe `TemperatureImpactPerSec`.
    // Haxe: ServerSettings.TemperatureImpactPerSec = 0.03
    // SETTINGS-LONG-TAIL
    pub temperature_impact_per_sec: f32,
    /// Haxe `TemperatureImpactPerSecIfGood`.
    // Haxe: ServerSettings.TemperatureImpactPerSecIfGood = 0.06
    // SETTINGS-LONG-TAIL
    pub temperature_impact_per_sec_if_good: f32,
    /// Haxe `TemperatureInWaterFactor`.
    // Haxe: ServerSettings.TemperatureInWaterFactor = 1.5
    // SETTINGS-LONG-TAIL
    pub temperature_in_water_factor: f32,
    /// Haxe `TemperatureImpactBelow`.
    // Haxe: ServerSettings.TemperatureImpactBelow = 0.6
    // SETTINGS-LONG-TAIL
    pub temperature_impact_below: f32,
    /// Haxe `TemperatureImpactColorFactor`.
    // Haxe: ServerSettings.TemperatureImpactColorFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub temperature_impact_color_factor: f32,
    /// Haxe `AllowEatingOrFeedingIfIll`.
    // Haxe: ServerSettings.AllowEatingOrFeedingIfIll = false
    // SETTINGS-LONG-TAIL
    pub allow_eating_or_feeding_if_ill: bool,
    /// Haxe `ResistanceAgainstFeverForEatingMushrooms`.
    // Haxe: ServerSettings.ResistanceAgainstFeverForEatingMushrooms = 0.2
    // SETTINGS-LONG-TAIL
    pub resistance_against_fever_for_eating_mushrooms: f32,
    /// Haxe `ExhaustionYellowFeverPerSec`.
    // Haxe: ServerSettings.ExhaustionYellowFeverPerSec = 0.1
    // SETTINGS-LONG-TAIL
    pub exhaustion_yellow_fever_per_sec: f32,
    /// Haxe `MinHealthFoodStoreMaxFactor`.
    // Haxe: ServerSettings.MinHealthFoodStoreMaxFactor = 0.8
    // SETTINGS-LONG-TAIL
    pub min_health_food_store_max_factor: f32,
    /// Haxe `MaxHealthFoodStoreMaxFactor`.
    // Haxe: ServerSettings.MaxHealthFoodStoreMaxFactor = 1.2
    // SETTINGS-LONG-TAIL
    pub max_health_food_store_max_factor: f32,
    /// Haxe `MinHealthAgingFactor`.
    // Haxe: ServerSettings.MinHealthAgingFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub min_health_aging_factor: f32,
    /// Haxe `MaxHealthAgingFactor`.
    // Haxe: ServerSettings.MaxHealthAgingFactor = 2
    // SETTINGS-LONG-TAIL
    pub max_health_aging_factor: f32,
    /// Haxe `MinHealthPerYear`.
    // Haxe: ServerSettings.MinHealthPerYear = 1
    // SETTINGS-LONG-TAIL
    pub min_health_per_year: f32,
    /// Haxe `MaxAge`.
    // Haxe: ServerSettings.MaxAge = 60
    // SETTINGS-LONG-TAIL
    pub max_age: f32,
    /// Haxe `AnimalDeadlyDistanceFactor`.
    // Haxe: ServerSettings.AnimalDeadlyDistanceFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub animal_deadly_distance_factor: f32,
    /// Haxe `ChanceForAnimalDyingFactorIfInLovedBiome`.
    // Haxe: ServerSettings.ChanceForAnimalDyingFactorIfInLovedBiome = 0.1
    // SETTINGS-LONG-TAIL
    pub chance_for_animal_dying_factor_if_in_loved_biome: f32,
    /// Haxe `OffspringFactorIfAnimalPopIsLow`.
    // Haxe: ServerSettings.OffspringFactorIfAnimalPopIsLow = 10
    // SETTINGS-LONG-TAIL
    pub offspring_factor_if_animal_pop_is_low: f32,
    /// Haxe `MaxOffspringFactor`.
    // Haxe: ServerSettings.MaxOffspringFactor = 1
    // SETTINGS-LONG-TAIL
    pub max_offspring_factor: f32,
    /// Haxe `OffspringFactorLowAnimalPopulationBelow`.
    // Haxe: ServerSettings.OffspringFactorLowAnimalPopulationBelow = 0.2
    // SETTINGS-LONG-TAIL
    pub offspring_factor_low_animal_population_below: f32,
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
            biome_animal_hit_chance: gameplay_defaults::BIOME_ANIMAL_HIT_CHANCE,
            hungry_work_cost: gameplay_defaults::HUNGRY_WORK_COST,
            birth_prestige_factor: gameplay_defaults::BIRTH_PRESTIGE_FACTOR,
            ally_strength_too_low_for_pickup: gameplay_defaults::ALLY_STRENGTH_TOO_LOW_FOR_PICKUP,
            time_confirm_new_follower: gameplay_defaults::TIME_CONFIRM_NEW_FOLLOWER,
            hire_cost: gameplay_defaults::HIRE_COST,
            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,
            auto_follow_player: gameplay_defaults::AUTO_FOLLOW_PLAYER,
            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,
            prestige_cost_per_damage_for_child:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CHILD,
            prestige_cost_per_damage_for_elderly:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ELDERLY,
            prestige_cost_per_damage_for_close_relatives:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CLOSE_RELATIVES,
            prestige_cost_per_damage_for_women_without_weapon:
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_WOMEN_WITHOUT_WEAPON,
            food_factor: gameplay_defaults::FOOD_FACTOR,
            food_factor_eaten_more_than_eight_percent:
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT,
            food_factor_eaten_more_than_ten_percent:
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT,
            food_factor_eaten_less_than_five_percent:
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT,
            food_factor_eaten_less_than_three_percent:
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT,
            food_factor_eaten_less_than_one_percent:
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT,
            yum_food_restore: gameplay_defaults::YUM_FOOD_RESTORE,
            loved_food_restore: gameplay_defaults::LOVED_FOOD_RESTORE,
            yum_new_craving_chance: gameplay_defaults::YUM_NEW_CRAVING_CHANCE,
            food_reduction_per_eating: gameplay_defaults::FOOD_REDUCTION_PER_EATING,
            food_reduction_faktor_for_eating_meh:
                gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_MEH,
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
            food_restore_factor_while_feeding: gameplay_defaults::FOOD_RESTORE_FACTOR_WHILE_FEEDING,
            max_has_eaten_for_next_generation: gameplay_defaults::MAX_HAS_EATEN_FOR_NEXT_GENERATION,
            has_eaten_reduction_for_next_generation:
                gameplay_defaults::HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION,
            // WALLET-COINS
            coins_on_wounding_factor: gameplay_defaults::COINS_ON_WOUNDING_FACTOR,
            // C-SS-MORE-BATCH3 (male factor already under C-SS-MALE-HEAL above)
            combat_exhaustion_cost_per_attack: gameplay_defaults::COMBAT_EXHAUSTION_COST_PER_ATTACK,
            min_age_to_eat: gameplay_defaults::MIN_AGE_TO_EAT,
            max_child_age_for_breast_feeding: gameplay_defaults::MAX_CHILD_AGE_FOR_BREAST_FEEDING,
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
            // SETTINGS-LONG-TAIL
            starting_eve_age: gameplay_defaults::STARTING_EVE_AGE,
            eve_or_adam_birth_chance: gameplay_defaults::EVE_OR_ADAM_BIRTH_CHANCE,
            spawn_ai_as_eve: gameplay_defaults::SPAWN_AI_AS_EVE,
            allow_humans_born_to_ais: gameplay_defaults::ALLOW_HUMANS_BORN_TO_AIS,
            max_players_before_starting_as_child:
                gameplay_defaults::MAX_PLAYERS_BEFORE_STARTING_AS_CHILD,
            obj_decay_chance: gameplay_defaults::OBJ_DECAY_CHANCE,
            floor_decay_chance: gameplay_defaults::FLOOR_DECAY_CHANCE,
            obj_respawn_chance: gameplay_defaults::OBJ_RESPAWN_CHANCE,
            grow_back_plants_increase_if_low_population:
                gameplay_defaults::GROW_BACK_PLANTS_INCREASE_IF_LOW_POPULATION,
            grow_back_original_plants_factor: gameplay_defaults::GROW_BACK_ORIGINAL_PLANTS_FACTOR,
            grow_new_plants_from_existing_factor:
                gameplay_defaults::GROW_NEW_PLANTS_FROM_EXISTING_FACTOR,
            spring_wild_food_regrow_chance: gameplay_defaults::SPRING_WILD_FOOD_REGROW_CHANCE,
            winter_wild_food_decay_chance: gameplay_defaults::WINTER_WILD_FOOD_DECAY_CHANCE,
            hot_season_temperature_factor: gameplay_defaults::HOT_SEASON_TEMPERATURE_FACTOR,
            cold_season_temperature_factor: gameplay_defaults::COLD_SEASON_TEMPERATURE_FACTOR,
            cursed_grave_time: gameplay_defaults::CURSED_GRAVE_TIME,
            animal_decay_factor: gameplay_defaults::ANIMAL_DECAY_FACTOR,
            obj_decay_factor_for_permanent: gameplay_defaults::OBJ_DECAY_FACTOR_FOR_PERMANENT_OBJS,
            obj_decay_factor_for_food: gameplay_defaults::OBJ_DECAY_FACTOR_FOR_FOOD,
            obj_decay_factor_for_clothing: gameplay_defaults::OBJ_DECAY_FACTOR_FOR_CLOTHING,
            obj_decay_factor_for_walls: gameplay_defaults::OBJ_DECAY_FACTOR_FOR_WALLS,
            obj_decay_factor_per_tech_level: gameplay_defaults::OBJ_DECAY_FACTOR_PER_TECH_LEVEL,
            decay_factor_in_deep_water: gameplay_defaults::DECAY_FACTOR_IN_DEEP_WATER,
            decay_factor_in_mountain: gameplay_defaults::DECAY_FACTOR_IN_MOUNTAIN,
            decay_factor_in_walkable_water: gameplay_defaults::DECAY_FACTOR_IN_WALKABLE_WATER,
            decay_factor_in_jungle: gameplay_defaults::DECAY_FACTOR_IN_JUNGLE,
            decay_factor_in_swamp: gameplay_defaults::DECAY_FACTOR_IN_SWAMP,
            score_factor: gameplay_defaults::SCORE_FACTOR,
            ancestor_prestige_factor: gameplay_defaults::ANCESTOR_PRESTIGE_FACTOR,
            display_score_factor: gameplay_defaults::DISPLAY_SCORE_FACTOR,
            display_score_on: gameplay_defaults::DISPLAY_SCORE_ON,
            max_coins_per_chest: gameplay_defaults::MAX_COINS_PER_CHEST,
            max_coins_per_pouch: gameplay_defaults::MAX_COINS_PER_POUCH,
            chance_for_female_child: gameplay_defaults::CHANCE_FOR_FEMALE_CHILD,
            chance_for_other_child_color: gameplay_defaults::CHANCE_FOR_OTHER_CHILD_COLOR,
            chance_for_other_child_color_if_close_to_wrong_special_biome:
                gameplay_defaults::CHANCE_FOR_OTHER_CHILD_COLOR_IF_CLOSE_TO_WRONG_SPECIAL_BIOME,
            little_kids_per_mother: gameplay_defaults::LITTLE_KIDS_PER_MOTHER,
            new_child_exhaustion_for_mother: gameplay_defaults::NEW_CHILD_EXHAUSTION_FOR_MOTHER,
            ai_mother_birth_mali_for_human_child:
                gameplay_defaults::AI_MOTHER_BIRTH_MALI_FOR_HUMAN_CHILD,
            human_mother_birth_mali_for_ai_child:
                gameplay_defaults::HUMAN_MOTHER_BIRTH_MALI_FOR_AI_CHILD,
            spawn_at_last_dead: gameplay_defaults::SPAWN_AT_LAST_DEAD,
            temperature_own_tile_rate: gameplay_defaults::TEMPERATURE_OWN_TILE_RATE,
            temperature_balance_rate: gameplay_defaults::TEMPERATURE_BALANCE_RATE,
            temperature_local_heat_factor: gameplay_defaults::TEMPERATURE_LOCAL_HEAT_FACTOR,
            average_season_temperature_impact:
                gameplay_defaults::AVERAGE_SEASON_TEMPERATURE_IMPACT,
            ai_total_score_factor: gameplay_defaults::AI_TOTAL_SCORE_FACTOR,
            old_grave_decay_mali: gameplay_defaults::OLD_GRAVE_DECAY_MALI,
            cursed_grave_mali: gameplay_defaults::CURSED_GRAVE_MALI,
            // Playable cull (24); Haxe compiled default is 2_000_000 via ServerConfig/toml.
            max_distance_close: 24,
            max_distance_map_changes: gameplay_defaults::MAX_DISTANCE_MAP_CHANGES,
            max_distance_say: gameplay_defaults::MAX_DISTANCE_SAY,
            send_move_every_x_ticks: gameplay_defaults::SEND_MOVE_EVERY_X_TICKS,
            max_distance_cose_for_movement: gameplay_defaults::MAX_DISTANCE_COSE_FOR_MOVEMENT,
            max_distance_say_ai: gameplay_defaults::MAX_DISTANCE_SAY_AI,
            max_distance_auto_exile_attacker: gameplay_defaults::MAX_DISTANCE_AUTO_EXILE_ATTACKER,
            speed_with_both_shoes: gameplay_defaults::SPEED_WITH_BOTH_SHOES,
            aging_factor_while_starving: gameplay_defaults::AGING_FACTOR_WHILE_STARVING,
            grown_up_age: gameplay_defaults::GROWN_UP_AGE,
            food_use_child_faktor: gameplay_defaults::FOOD_USE_CHILD_FAKTOR,
            ai_food_use_factor_serf: gameplay_defaults::AI_FOOD_USE_FACTOR_SERF,
            ai_food_use_factor_commoner: gameplay_defaults::AI_FOOD_USE_FACTOR_COMMONER,
            ai_food_use_factor_noble: gameplay_defaults::AI_FOOD_USE_FACTOR_NOBLE,
            eve_food_use_factor: gameplay_defaults::EVE_FOOD_USE_FACTOR,
            aging_factor_human_born_to_ai: gameplay_defaults::AGING_FACTOR_HUMAN_BORN_TO_AI,
            aging_factor_ai_born_to_human: gameplay_defaults::AGING_FACTOR_AI_BORN_TO_HUMAN,
            eve_damage_factor: gameplay_defaults::EVE_DAMAGE_FACTOR,
            target_wounded_damage_factor: gameplay_defaults::TARGET_WOUNDED_DAMAGE_FACTOR,
            male_damage_factor: gameplay_defaults::MALE_DAMAGE_FACTOR,
            animal_damage_factor: gameplay_defaults::ANIMAL_DAMAGE_FACTOR,
            animal_damage_factor_in_winter: gameplay_defaults::ANIMAL_DAMAGE_FACTOR_IN_WINTER,
            animal_damage_factor_if_attacked: gameplay_defaults::ANIMAL_DAMAGE_FACTOR_IF_ATTACKED,
            weapon_damage_factor: gameplay_defaults::WEAPON_DAMAGE_FACTOR,
            grave_blocking_distance: gameplay_defaults::GRAVE_BLOCKING_DISTANCE,
            max_players_before_activating_grave_curse:
                gameplay_defaults::MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE,
            max_players_before_forbid_touch_grave:
                gameplay_defaults::MAX_PLAYERS_BEFORE_FORBID_TOUCH_GRAVE,
            combat_angry_time_before_attack: gameplay_defaults::COMBAT_ANGRY_TIME_BEFORE_ATTACK,
            combat_angry_time_minimum: gameplay_defaults::COMBAT_ANGRY_TIME_MINIMUM,
            chance_for_domestic_animal_dying_factor:
                gameplay_defaults::CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR,
            door_ids: ol_config::DOOR_IDS.to_vec(),
            ai_ignored_floor_ids: ol_config::AI_IGNORED_FLOOR_IDS.to_vec(),
            secret: gameplay_defaults::SECRET.to_string(),
            allow_debug_commands: true,
            debug_say_player_position: false,
            ai_time_to_wait_if_crafting_failed:
                gameplay_defaults::AI_TIME_TO_WAIT_IF_CRAFTING_FAILED,
            ai_max_search_radius: gameplay_defaults::AI_MAX_SEARCH_RADIUS,
            ai_max_search_increment: gameplay_defaults::AI_MAX_SEARCH_INCREMENT,
            ai_ignore_time_transitions_longer_then:
                gameplay_defaults::AI_IGNORE_TIME_TRANSITIONS_LONGER_THEN,
            ai_memory_max_entries: gameplay_defaults::AI_MEMORY_MAX_ENTRIES,
            ai_chat_memory_max_entries: gameplay_defaults::AI_CHAT_MEMORY_MAX_ENTRIES,
            alternative_outcome_percent_increase_per_hit:
                gameplay_defaults::ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT,
            alternative_outcome_hits_decrease_on_success:
                gameplay_defaults::ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS,
            fortification_cost_per_hit: gameplay_defaults::FORTIFICATION_COST_PER_HIT,
            reduce_age_needed_to_pickup_objects:
                gameplay_defaults::REDUCE_AGE_NEEDED_TO_PICKUP_OBJECTS,
            chance_animals_pass_blocking_biome:
                gameplay_defaults::CHANCE_ANIMALS_PASS_BLOCKING_BIOME,
            chance_preferred_biome: gameplay_defaults::CHANCE_PREFERRED_BIOME,
            close_grave_speed_mali: gameplay_defaults::CLOSE_GRAVE_SPEED_MALI,
            temperature_speed_impact: gameplay_defaults::TEMPERATURE_SPEED_IMPACT,
            min_speed_reduction_per_contained_obj:
                gameplay_defaults::MIN_SPEED_REDUCTION_PER_CONTAINED_OBJ,
            loved_food_use_chance: gameplay_defaults::LOVED_FOOD_USE_CHANCE,
            max_age_for_allowing_cloth_and_pickup_from_others:
                gameplay_defaults::MAX_AGE_FOR_ALLOWING_CLOTH_AND_PICKUP_FROM_OTHERS,
            max_age_for_allowing_die: gameplay_defaults::MAX_AGE_FOR_ALLOWING_DIE,
            prestige_cost_for_die: gameplay_defaults::PRESTIGE_COST_FOR_DIE,
            starting_family_name: gameplay_defaults::STARTING_FAMILY_NAME.to_string(),
            starting_name: gameplay_defaults::STARTING_NAME.to_string(),
            found_family_needed_prestige: gameplay_defaults::FOUND_FAMILY_NEEDED_PRESTIGE,
            found_family_cost: gameplay_defaults::FOUND_FAMILY_COST,
            found_family_needed_followers: gameplay_defaults::FOUND_FAMILY_NEEDED_FOLLOWERS,
            found_family_break_alliance_chance: gameplay_defaults::FOUND_FAMILY_BREAK_ALLIANCE_CHANCE,
            pickup_exhaustion_gain: gameplay_defaults::PICKUP_EXHAUSTION_GAIN,
            pickup_feeding_food_restore: gameplay_defaults::PICKUP_FEEDING_FOOD_RESTORE,
            death_with_food_store_max: gameplay_defaults::DEATH_WITH_FOOD_STORE_MAX,
            food_store_max_reduction_while_starving:
                gameplay_defaults::FOOD_STORE_MAX_REDUCTION_WHILE_STARVING,
            temperature_reduction_per_drinking:
                gameplay_defaults::TEMPERATURE_REDUCTION_PER_DRINKING,
            max_stored_water: gameplay_defaults::MAX_STORED_WATER,
            max_jumps_per_ten_sec: gameplay_defaults::MAX_JUMPS_PER_TEN_SEC,
            temperature_impact_per_sec: gameplay_defaults::TEMPERATURE_IMPACT_PER_SEC,
            temperature_impact_per_sec_if_good:
                gameplay_defaults::TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            temperature_in_water_factor: gameplay_defaults::TEMPERATURE_IN_WATER_FACTOR,
            temperature_impact_below: gameplay_defaults::TEMPERATURE_IMPACT_BELOW,
            temperature_impact_color_factor: gameplay_defaults::TEMPERATURE_IMPACT_COLOR_FACTOR,
            allow_eating_or_feeding_if_ill: gameplay_defaults::ALLOW_EATING_OR_FEEDING_IF_ILL,
            resistance_against_fever_for_eating_mushrooms:
                gameplay_defaults::RESISTANCE_AGAINST_FEVER_FOR_EATING_MUSHROOMS,
            exhaustion_yellow_fever_per_sec: gameplay_defaults::EXHAUSTION_YELLOW_FEVER_PER_SEC,
            min_health_food_store_max_factor: gameplay_defaults::MIN_HEALTH_FOOD_STORE_MAX_FACTOR,
            max_health_food_store_max_factor: gameplay_defaults::MAX_HEALTH_FOOD_STORE_MAX_FACTOR,
            min_health_aging_factor: gameplay_defaults::MIN_HEALTH_AGING_FACTOR,
            max_health_aging_factor: gameplay_defaults::MAX_HEALTH_AGING_FACTOR,
            min_health_per_year: gameplay_defaults::MIN_HEALTH_PER_YEAR,
            max_age: gameplay_defaults::MAX_AGE,
            animal_deadly_distance_factor: gameplay_defaults::ANIMAL_DEADLY_DISTANCE_FACTOR,
            chance_for_animal_dying_factor_if_in_loved_biome:
                gameplay_defaults::CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME,
            offspring_factor_if_animal_pop_is_low:
                gameplay_defaults::OFFSPRING_FACTOR_IF_ANIMAL_POP_IS_LOW,
            max_offspring_factor: gameplay_defaults::MAX_OFFSPRING_FACTOR,
            offspring_factor_low_animal_population_below:
                gameplay_defaults::OFFSPRING_FACTOR_LOW_ANIMAL_POPULATION_BELOW,
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
            biome_animal_hit_chance: live.biome_animal_hit_chance,
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
            food_factor_eaten_more_than_eight_percent: live
                .food_factor_eaten_more_than_eight_percent,
            food_factor_eaten_more_than_ten_percent: live.food_factor_eaten_more_than_ten_percent,
            food_factor_eaten_less_than_five_percent: live.food_factor_eaten_less_than_five_percent,
            food_factor_eaten_less_than_three_percent: live
                .food_factor_eaten_less_than_three_percent,
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
            has_eaten_reduction_for_next_generation: live.has_eaten_reduction_for_next_generation,
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
            // SETTINGS-LONG-TAIL
            starting_eve_age: live.starting_eve_age,
            eve_or_adam_birth_chance: live.eve_or_adam_birth_chance,
            spawn_ai_as_eve: live.spawn_ai_as_eve,
            allow_humans_born_to_ais: live.allow_humans_born_to_ais,
            max_players_before_starting_as_child: live.max_players_before_starting_as_child,
            obj_decay_chance: live.obj_decay_chance,
            floor_decay_chance: live.floor_decay_chance,
            obj_respawn_chance: live.obj_respawn_chance,
            grow_back_plants_increase_if_low_population: live
                .grow_back_plants_increase_if_low_population,
            grow_back_original_plants_factor: live.grow_back_original_plants_factor,
            grow_new_plants_from_existing_factor: live.grow_new_plants_from_existing_factor,
            spring_wild_food_regrow_chance: live.spring_wild_food_regrow_chance,
            winter_wild_food_decay_chance: live.winter_wild_food_decay_chance,
            hot_season_temperature_factor: live.hot_season_temperature_factor,
            cold_season_temperature_factor: live.cold_season_temperature_factor,
            cursed_grave_time: live.cursed_grave_time,
            animal_decay_factor: live.animal_decay_factor,
            obj_decay_factor_for_permanent: live.obj_decay_factor_for_permanent,
            obj_decay_factor_for_food: live.obj_decay_factor_for_food,
            obj_decay_factor_for_clothing: live.obj_decay_factor_for_clothing,
            obj_decay_factor_for_walls: live.obj_decay_factor_for_walls,
            obj_decay_factor_per_tech_level: live.obj_decay_factor_per_tech_level,
            decay_factor_in_deep_water: live.decay_factor_in_deep_water,
            decay_factor_in_mountain: live.decay_factor_in_mountain,
            decay_factor_in_walkable_water: live.decay_factor_in_walkable_water,
            decay_factor_in_jungle: live.decay_factor_in_jungle,
            decay_factor_in_swamp: live.decay_factor_in_swamp,
            score_factor: live.score_factor,
            ancestor_prestige_factor: live.ancestor_prestige_factor,
            display_score_factor: live.display_score_factor,
            display_score_on: live.display_score_on,
            max_coins_per_chest: live.max_coins_per_chest,
            max_coins_per_pouch: live.max_coins_per_pouch,
            chance_for_female_child: live.chance_for_female_child,
            chance_for_other_child_color: live.chance_for_other_child_color,
            chance_for_other_child_color_if_close_to_wrong_special_biome: live
                .chance_for_other_child_color_if_close_to_wrong_special_biome,
            little_kids_per_mother: live.little_kids_per_mother,
            new_child_exhaustion_for_mother: live.new_child_exhaustion_for_mother,
            ai_mother_birth_mali_for_human_child: live.ai_mother_birth_mali_for_human_child,
            human_mother_birth_mali_for_ai_child: live.human_mother_birth_mali_for_ai_child,
            spawn_at_last_dead: live.spawn_at_last_dead,
            temperature_own_tile_rate: live.temperature_own_tile_rate,
            temperature_balance_rate: live.temperature_balance_rate,
            temperature_local_heat_factor: live.temperature_local_heat_factor,
            average_season_temperature_impact: live.average_season_temperature_impact,
            ai_total_score_factor: live.ai_total_score_factor,
            old_grave_decay_mali: live.old_grave_decay_mali,
            cursed_grave_mali: live.cursed_grave_mali,
            max_distance_close: live.max_distance_close,
            max_distance_map_changes: live.max_distance_map_changes,
            max_distance_say: live.max_distance_say,
            send_move_every_x_ticks: live.send_move_every_x_ticks,
            max_distance_cose_for_movement: live.max_distance_cose_for_movement,
            max_distance_say_ai: live.max_distance_say_ai,
            max_distance_auto_exile_attacker: live.max_distance_auto_exile_attacker,
            speed_with_both_shoes: live.speed_with_both_shoes,
            aging_factor_while_starving: live.aging_factor_while_starving,
            grown_up_age: live.grown_up_age,
            food_use_child_faktor: live.food_use_child_faktor,
            ai_food_use_factor_serf: live.ai_food_use_factor_serf,
            ai_food_use_factor_commoner: live.ai_food_use_factor_commoner,
            ai_food_use_factor_noble: live.ai_food_use_factor_noble,
            eve_food_use_factor: live.eve_food_use_factor,
            aging_factor_human_born_to_ai: live.aging_factor_human_born_to_ai,
            aging_factor_ai_born_to_human: live.aging_factor_ai_born_to_human,
            eve_damage_factor: live.eve_damage_factor,
            target_wounded_damage_factor: live.target_wounded_damage_factor,
            male_damage_factor: live.male_damage_factor,
            animal_damage_factor: live.animal_damage_factor,
            animal_damage_factor_in_winter: live.animal_damage_factor_in_winter,
            animal_damage_factor_if_attacked: live.animal_damage_factor_if_attacked,
            weapon_damage_factor: live.weapon_damage_factor,
            grave_blocking_distance: live.grave_blocking_distance,
            max_players_before_activating_grave_curse: live
                .max_players_before_activating_grave_curse,
            max_players_before_forbid_touch_grave: live.max_players_before_forbid_touch_grave,
            combat_angry_time_before_attack: live.combat_angry_time_before_attack,
            combat_angry_time_minimum: live.combat_angry_time_minimum,
            chance_for_domestic_animal_dying_factor: live
                .chance_for_domestic_animal_dying_factor,
            door_ids: if live.door_ids.is_empty() {
                ol_config::DOOR_IDS.to_vec()
            } else {
                live.door_ids.clone()
            },
            ai_ignored_floor_ids: if live.ai_ignored_floor_ids.is_empty() {
                ol_config::AI_IGNORED_FLOOR_IDS.to_vec()
            } else {
                live.ai_ignored_floor_ids.clone()
            },
            secret: if live.secret.is_empty() {
                gameplay_defaults::SECRET.to_string()
            } else {
                live.secret.clone()
            },
            allow_debug_commands: live.allow_debug_commands,
            debug_say_player_position: live.debug_say_player_position,
            ai_time_to_wait_if_crafting_failed: live.ai_time_to_wait_if_crafting_failed,
            ai_max_search_radius: live.ai_max_search_radius,
            ai_max_search_increment: live.ai_max_search_increment,
            ai_ignore_time_transitions_longer_then: live.ai_ignore_time_transitions_longer_then,
            ai_memory_max_entries: live.ai_memory_max_entries,
            ai_chat_memory_max_entries: live.ai_chat_memory_max_entries,
            alternative_outcome_percent_increase_per_hit: live
                .alternative_outcome_percent_increase_per_hit,
            alternative_outcome_hits_decrease_on_success: live
                .alternative_outcome_hits_decrease_on_success,
            fortification_cost_per_hit: live.fortification_cost_per_hit,
            reduce_age_needed_to_pickup_objects: live.reduce_age_needed_to_pickup_objects,
            chance_animals_pass_blocking_biome: live.chance_animals_pass_blocking_biome,
            chance_preferred_biome: live.chance_preferred_biome,
            close_grave_speed_mali: live.close_grave_speed_mali,
            temperature_speed_impact: live.temperature_speed_impact,
            min_speed_reduction_per_contained_obj: live.min_speed_reduction_per_contained_obj,
            loved_food_use_chance: live.loved_food_use_chance,
            max_age_for_allowing_cloth_and_pickup_from_others: live
                .max_age_for_allowing_cloth_and_pickup_from_others,
            max_age_for_allowing_die: live.max_age_for_allowing_die,
            prestige_cost_for_die: live.prestige_cost_for_die,
            starting_family_name: {
                let t = live.starting_family_name.trim();
                if t.is_empty() {
                    gameplay_defaults::STARTING_FAMILY_NAME.to_string()
                } else {
                    t.to_string()
                }
            },
            starting_name: {
                let t = live.starting_name.trim();
                if t.is_empty() {
                    gameplay_defaults::STARTING_NAME.to_string()
                } else {
                    t.to_string()
                }
            },
            found_family_needed_prestige: live.found_family_needed_prestige,
            found_family_cost: live.found_family_cost,
            found_family_needed_followers: live.found_family_needed_followers,
            found_family_break_alliance_chance: live.found_family_break_alliance_chance,
            pickup_exhaustion_gain: live.pickup_exhaustion_gain,
            pickup_feeding_food_restore: live.pickup_feeding_food_restore,
            death_with_food_store_max: live.death_with_food_store_max,
            food_store_max_reduction_while_starving: live.food_store_max_reduction_while_starving,
            temperature_reduction_per_drinking: live.temperature_reduction_per_drinking,
            max_stored_water: live.max_stored_water,
            max_jumps_per_ten_sec: live.max_jumps_per_ten_sec,
            temperature_impact_per_sec: live.temperature_impact_per_sec,
            temperature_impact_per_sec_if_good: live.temperature_impact_per_sec_if_good,
            temperature_in_water_factor: live.temperature_in_water_factor,
            temperature_impact_below: live.temperature_impact_below,
            temperature_impact_color_factor: live.temperature_impact_color_factor,
            allow_eating_or_feeding_if_ill: live.allow_eating_or_feeding_if_ill,
            resistance_against_fever_for_eating_mushrooms: live
                .resistance_against_fever_for_eating_mushrooms,
            exhaustion_yellow_fever_per_sec: live.exhaustion_yellow_fever_per_sec,
            min_health_food_store_max_factor: live.min_health_food_store_max_factor,
            max_health_food_store_max_factor: live.max_health_food_store_max_factor,
            min_health_aging_factor: live.min_health_aging_factor,
            max_health_aging_factor: live.max_health_aging_factor,
            min_health_per_year: live.min_health_per_year,
            max_age: live.max_age,
            animal_deadly_distance_factor: live.animal_deadly_distance_factor,
            chance_for_animal_dying_factor_if_in_loved_biome: live
                .chance_for_animal_dying_factor_if_in_loved_biome,
            offspring_factor_if_animal_pop_is_low: live.offspring_factor_if_animal_pop_is_low,
            max_offspring_factor: live.max_offspring_factor,
            offspring_factor_low_animal_population_below: live
                .offspring_factor_low_animal_population_below,
        }
    }

    /// Scale raw season impact by live `AverageSeasonTemperatureImpact` / Haxe 0.2.
    // Haxe: TimeHelper.DoSeason AverageSeasonTemperatureImpact * SeasonHardness
    // SETTINGS-KNOB-TAIL
    #[inline]
    pub fn scale_season_temperature_impact(&self, raw: f32) -> f32 {
        let avg = if self.average_season_temperature_impact.is_finite()
            && self.average_season_temperature_impact >= 0.0
        {
            self.average_season_temperature_impact
        } else {
            gameplay_defaults::AVERAGE_SEASON_TEMPERATURE_IMPACT
        };
        let base = gameplay_defaults::AVERAGE_SEASON_TEMPERATURE_IMPACT;
        if base > 0.0 && raw.is_finite() {
            raw * (avg / base)
        } else {
            raw
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
            starve_reduction: self.food_store_max_reduction_while_starving,
            max_age: self.max_age,
        }
    }

    /// Live Haxe `DeathWithFoodStoreMax` (negative death line).
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn death_with_food_store_max_live(&self) -> f32 {
        if self.death_with_food_store_max.is_finite() {
            self.death_with_food_store_max
        } else {
            crate::food_store_max::DEATH_WITH_FOOD_STORE_MAX
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
        (
            self.weapon_cooldown_factor,
            self.weapon_cooldown_factor_if_wounding,
        )
    }

    /// Live sharp-stone overflow extra seconds (`CursedGraveTime` hours × 3600).
    // Haxe: ServerSettings.CursedGraveTime * 60 * 60
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn cursed_grave_sharp_stone_extra_secs(&self) -> f32 {
        crate::world_time::cursed_grave_sharp_stone_extra_secs(self.cursed_grave_time)
    }

    /// Live long-term decay chance knobs (Haxe ObjDecayChance / FloorDecayChance).
    // Haxe: ServerSettings.ObjDecayChance / FloorDecayChance
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn decay_chance_knobs(&self) -> crate::long_term::DecayChanceKnobs {
        crate::long_term::DecayChanceKnobs {
            obj_decay_chance: self.obj_decay_chance,
            floor_decay_chance: self.floor_decay_chance,
            animal_decay_factor: self.animal_decay_factor,
            obj_decay_factor_for_permanent: self.obj_decay_factor_for_permanent,
            obj_decay_factor_for_food: self.obj_decay_factor_for_food,
            obj_decay_factor_for_clothing: self.obj_decay_factor_for_clothing,
            obj_decay_factor_for_walls: self.obj_decay_factor_for_walls,
            obj_decay_factor_per_tech_level: self.obj_decay_factor_per_tech_level,
            decay_factor_in_deep_water: self.decay_factor_in_deep_water,
            decay_factor_in_mountain: self.decay_factor_in_mountain,
            decay_factor_in_walkable_water: self.decay_factor_in_walkable_water,
            decay_factor_in_jungle: self.decay_factor_in_jungle,
            decay_factor_in_swamp: self.decay_factor_in_swamp,
            obj_respawn_chance: self.obj_respawn_chance,
            grow_back_plants_increase_if_low_population: self
                .grow_back_plants_increase_if_low_population,
            grow_back_original_plants_factor: self.grow_back_original_plants_factor,
            grow_new_from_existing_factor: self.grow_new_plants_from_existing_factor,
        }
    }

    /// Live animal DoDamage factors (base / winter / if-attacked).
    // Haxe: ServerSettings.AnimalDamageFactor*
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn animal_damage_factor_knobs(&self) -> crate::animal_damage::AnimalDamageFactorKnobs {
        crate::animal_damage::AnimalDamageFactorKnobs {
            factor: self.animal_damage_factor,
            winter: self.animal_damage_factor_in_winter,
            if_attacked: self.animal_damage_factor_if_attacked,
        }
    }

    /// Live spawn / UpdateEmotes `CombatAngryTimeBeforeAttack`.
    // Haxe: ServerSettings.CombatAngryTimeBeforeAttack
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn combat_angry_time_before_attack_live(&self) -> f32 {
        crate::fever_pe::combat_angry_before_attack_ex(self.combat_angry_time_before_attack)
    }

    /// Live `CombatAngryTimeMinimum` (finite; compiled −60 fallback).
    // Haxe: ServerSettings.CombatAngryTimeMinimum = -60
    #[inline]
    pub fn combat_angry_time_minimum_live(&self) -> f32 {
        if self.combat_angry_time_minimum.is_finite() {
            self.combat_angry_time_minimum
        } else {
            gameplay_defaults::COMBAT_ANGRY_TIME_MINIMUM
        }
    }

    /// Live `DoorIds` membership (empty table → compiled Haxe list).
    // Haxe: ServerSettings.IsDoor
    #[inline]
    pub fn is_door_id(&self, item_id: i32) -> bool {
        ol_config::is_door_id_in(item_id, &self.door_ids)
    }

    /// Live `AiIgnoredFloorIds` membership (empty table → compiled Haxe list).
    // Haxe: ServerSettings.AiIgnoredFloorIds / AiHelper.IsIgnoredFloor
    #[inline]
    pub fn is_ai_ignored_floor_id(&self, floor_id: i32) -> bool {
        ol_config::is_ai_ignored_floor_id_in(floor_id, &self.ai_ignored_floor_ids)
    }

    /// Slice for farm/bake count-close (compiled default when empty).
    #[inline]
    pub fn ai_ignored_floor_ids_slice(&self) -> &[i32] {
        if self.ai_ignored_floor_ids.is_empty() {
            ol_config::AI_IGNORED_FLOOR_IDS
        } else {
            &self.ai_ignored_floor_ids
        }
    }

    /// Live AI craftItem search knobs (wait / max radius / increment / ignore-time).
    // Haxe: ServerSettings.AiTimeToWaitIfCraftingFailed / AiMaxSearchRadius / AiMaxSearchIncrement / AiIgnoreTimeTransitionsLongerThen
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn apply_craft_ai_search_knobs(&self, opts: &mut crate::CraftLiveExpandOpts) {
        opts.ai_time_to_wait_if_crafting_failed_sec =
            if self.ai_time_to_wait_if_crafting_failed.is_finite()
                && self.ai_time_to_wait_if_crafting_failed >= 0.0
            {
                f64::from(self.ai_time_to_wait_if_crafting_failed)
            } else {
                crate::get_or_craft::AI_TIME_TO_WAIT_IF_CRAFTING_FAILED_SEC
            };
        opts.ai_max_search_radius = if self.ai_max_search_radius >= 1 {
            self.ai_max_search_radius
        } else {
            crate::get_or_craft::AI_MAX_SEARCH_RADIUS
        };
        opts.ai_max_search_increment = if self.ai_max_search_increment >= 1 {
            self.ai_max_search_increment
        } else {
            crate::get_or_craft::AI_MAX_SEARCH_INCREMENT
        };
        opts.ai_ignore_time_transitions_longer_then =
            if self.ai_ignore_time_transitions_longer_then.is_finite()
                && self.ai_ignore_time_transitions_longer_then >= 0.0
            {
                self.ai_ignore_time_transitions_longer_then
            } else {
                crate::get_or_craft::AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN
            };
    }

    /// Live grave-curse radius + population gate (Haxe GraveBlockingDistance / MaxPlayers*).
    // Haxe: ServerSettings.GraveBlockingDistance / MaxPlayersBeforeActivatingGraveCurse
    // SETTINGS-LONG-TAIL
    #[inline]
    pub fn grave_curse_live_knobs(&self) -> (f32, usize) {
        let dist =
            if self.grave_blocking_distance.is_finite() && self.grave_blocking_distance >= 0.0 {
                self.grave_blocking_distance
            } else {
                crate::move_live_gates::GRAVE_BLOCKING_DISTANCE
            };
        let cap = if self.max_players_before_activating_grave_curse >= 0 {
            self.max_players_before_activating_grave_curse as usize
        } else {
            crate::move_live_gates::MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE
        };
        (dist, cap)
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
            speed_with_both_shoes: self.speed_with_both_shoes,
            close_grave_speed_mali: self.close_grave_speed_mali,
            temperature_speed_impact: self.temperature_speed_impact,
            min_speed_reduction_per_contained_obj: self.min_speed_reduction_per_contained_obj,
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
    /// Haxe OriginalObjects.bin — frozen generation census path.
    pub original_census_path: Option<std::path::PathBuf>,
    /// AI-LLM-HTTP-DRAIN: job/result bridge for ol-server `call_ai_async` worker.
    pub llm_speech_share: Option<crate::LlmSpeechIoShare>,
    /// NPC-SCAN-FULL: live `blockedByAI` mirror for the NPC think thread.
    pub blocked_by_ai_share: Option<BlockedByAiShare>,
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
            original_census_path: None,
            llm_speech_share: None,
            blocked_by_ai_share: None,
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
    let sl =
        haxe_next_season_length_secs(base_years, unit_random_duration, unit_random_hardness, hard);
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

    let max_p = live.max_players.max(1);
    if state.spawn_queue.max_players != max_p {
        state.spawn_queue.max_players = max_p;
        keys.push("max_players");
    }
    if state.spawn_queue.npc_min != live.npc_min {
        state.spawn_queue.npc_min = live.npc_min;
        keys.push("npc_min");
    }
    if state.last_vanilla_id != live.last_vanilla_id {
        state.last_vanilla_id = live.last_vanilla_id;
        keys.push("last_vanilla_id");
    }
    let ol_name = if live.open_life_client_name.trim().is_empty() {
        "OpenLife".to_string()
    } else {
        live.open_life_client_name.clone()
    };
    if state.open_life_client_name != ol_name {
        state.open_life_client_name = ol_name;
        keys.push("open_life_client_name");
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
        "biome_animal_hit_chance",
        (old.biome_animal_hit_chance - gp.biome_animal_hit_chance).abs() > 1e-12,
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
        (old.hire_cost_increase_per_person - gp.hire_cost_increase_per_person).abs() > f32::EPSILON,
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
        (old.food_factor_eaten_less_than_five_percent
            - gp.food_factor_eaten_less_than_five_percent)
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
        (old.food_reduction_faktor_for_eating_meh - gp.food_reduction_faktor_for_eating_meh).abs()
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
        (old.temperature_exhaustion_damage_factor - gp.temperature_exhaustion_damage_factor).abs()
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
        (old.has_eaten_reduction_for_next_generation - gp.has_eaten_reduction_for_next_generation)
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
        (old.combat_exhaustion_cost_per_attack - gp.combat_exhaustion_cost_per_attack).abs()
            > f32::EPSILON,
    );
    push_gp(
        "min_age_to_eat",
        (old.min_age_to_eat - gp.min_age_to_eat).abs() > f32::EPSILON,
    );
    push_gp(
        "max_child_age_for_breast_feeding",
        (old.max_child_age_for_breast_feeding - gp.max_child_age_for_breast_feeding).abs()
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
        (old.cursed_receive_damage_factor - gp.cursed_receive_damage_factor).abs() > f32::EPSILON,
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
        (old.weapon_cooldown_factor_if_wounding - gp.weapon_cooldown_factor_if_wounding).abs()
            > f32::EPSILON,
    );
    push_gp(
        "close_enemy_with_weapon_speed_factor",
        (old.close_enemy_with_weapon_speed_factor - gp.close_enemy_with_weapon_speed_factor).abs()
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
    push_gp(
        "starting_eve_age",
        (old.starting_eve_age - gp.starting_eve_age).abs() > f32::EPSILON,
    );
    push_gp(
        "eve_or_adam_birth_chance",
        (old.eve_or_adam_birth_chance - gp.eve_or_adam_birth_chance).abs() > 1e-12,
    );
    push_gp(
        "max_players_before_starting_as_child",
        old.max_players_before_starting_as_child != gp.max_players_before_starting_as_child,
    );
    push_gp(
        "spawn_ai_as_eve",
        old.spawn_ai_as_eve != gp.spawn_ai_as_eve,
    );
    push_gp(
        "allow_humans_born_to_ais",
        old.allow_humans_born_to_ais != gp.allow_humans_born_to_ais,
    );
    push_gp(
        "obj_decay_chance",
        (old.obj_decay_chance - gp.obj_decay_chance).abs() > 1e-12,
    );
    push_gp(
        "floor_decay_chance",
        (old.floor_decay_chance - gp.floor_decay_chance).abs() > 1e-12,
    );
    push_gp(
        "obj_respawn_chance",
        (old.obj_respawn_chance - gp.obj_respawn_chance).abs() > 1e-12,
    );
    push_gp(
        "grow_back_plants_increase_if_low_population",
        (old.grow_back_plants_increase_if_low_population
            - gp.grow_back_plants_increase_if_low_population)
            .abs()
            > 1e-12,
    );
    push_gp(
        "grow_back_original_plants_factor",
        (old.grow_back_original_plants_factor - gp.grow_back_original_plants_factor).abs()
            > 1e-12,
    );
    push_gp(
        "grow_new_plants_from_existing_factor",
        (old.grow_new_plants_from_existing_factor - gp.grow_new_plants_from_existing_factor)
            .abs()
            > 1e-12,
    );
    push_gp(
        "spring_wild_food_regrow_chance",
        (old.spring_wild_food_regrow_chance - gp.spring_wild_food_regrow_chance).abs() > 1e-12,
    );
    push_gp(
        "winter_wild_food_decay_chance",
        (old.winter_wild_food_decay_chance - gp.winter_wild_food_decay_chance).abs() > 1e-12,
    );
    push_gp(
        "hot_season_temperature_factor",
        (old.hot_season_temperature_factor - gp.hot_season_temperature_factor).abs() > 1e-12,
    );
    push_gp(
        "cold_season_temperature_factor",
        (old.cold_season_temperature_factor - gp.cold_season_temperature_factor).abs() > 1e-12,
    );
    push_gp(
        "cursed_grave_time",
        (old.cursed_grave_time - gp.cursed_grave_time).abs() > f32::EPSILON,
    );
    push_gp(
        "animal_decay_factor",
        (old.animal_decay_factor - gp.animal_decay_factor).abs() > 1e-12,
    );
    push_gp(
        "obj_decay_factor_for_permanent",
        (old.obj_decay_factor_for_permanent - gp.obj_decay_factor_for_permanent).abs() > 1e-12,
    );
    push_gp(
        "obj_decay_factor_for_food",
        (old.obj_decay_factor_for_food - gp.obj_decay_factor_for_food).abs() > 1e-12,
    );
    push_gp(
        "obj_decay_factor_for_clothing",
        (old.obj_decay_factor_for_clothing - gp.obj_decay_factor_for_clothing).abs() > 1e-12,
    );
    push_gp(
        "obj_decay_factor_for_walls",
        (old.obj_decay_factor_for_walls - gp.obj_decay_factor_for_walls).abs() > 1e-12,
    );
    push_gp(
        "obj_decay_factor_per_tech_level",
        (old.obj_decay_factor_per_tech_level - gp.obj_decay_factor_per_tech_level).abs() > 1e-12,
    );
    push_gp(
        "decay_factor_in_deep_water",
        (old.decay_factor_in_deep_water - gp.decay_factor_in_deep_water).abs() > 1e-12,
    );
    push_gp(
        "decay_factor_in_mountain",
        (old.decay_factor_in_mountain - gp.decay_factor_in_mountain).abs() > 1e-12,
    );
    push_gp(
        "decay_factor_in_walkable_water",
        (old.decay_factor_in_walkable_water - gp.decay_factor_in_walkable_water).abs() > 1e-12,
    );
    push_gp(
        "decay_factor_in_jungle",
        (old.decay_factor_in_jungle - gp.decay_factor_in_jungle).abs() > 1e-12,
    );
    push_gp(
        "decay_factor_in_swamp",
        (old.decay_factor_in_swamp - gp.decay_factor_in_swamp).abs() > 1e-12,
    );
    push_gp(
        "score_factor",
        (old.score_factor - gp.score_factor).abs() > 1e-12,
    );
    push_gp(
        "ancestor_prestige_factor",
        (old.ancestor_prestige_factor - gp.ancestor_prestige_factor).abs() > 1e-12,
    );
    push_gp(
        "display_score_factor",
        (old.display_score_factor - gp.display_score_factor).abs() > 1e-12,
    );
    push_gp(
        "display_score_on",
        old.display_score_on != gp.display_score_on,
    );
    push_gp(
        "max_coins_per_chest",
        old.max_coins_per_chest != gp.max_coins_per_chest,
    );
    push_gp(
        "max_coins_per_pouch",
        old.max_coins_per_pouch != gp.max_coins_per_pouch,
    );
    push_gp(
        "chance_for_female_child",
        (old.chance_for_female_child - gp.chance_for_female_child).abs() > 1e-12,
    );
    push_gp(
        "chance_for_other_child_color",
        (old.chance_for_other_child_color - gp.chance_for_other_child_color).abs() > 1e-12,
    );
    push_gp(
        "chance_for_other_child_color_if_close_to_wrong_special_biome",
        (old.chance_for_other_child_color_if_close_to_wrong_special_biome
            - gp.chance_for_other_child_color_if_close_to_wrong_special_biome)
            .abs()
            > 1e-12,
    );
    push_gp(
        "little_kids_per_mother",
        old.little_kids_per_mother != gp.little_kids_per_mother,
    );
    push_gp(
        "new_child_exhaustion_for_mother",
        (old.new_child_exhaustion_for_mother - gp.new_child_exhaustion_for_mother).abs() > 1e-12,
    );
    push_gp(
        "ai_mother_birth_mali_for_human_child",
        (old.ai_mother_birth_mali_for_human_child - gp.ai_mother_birth_mali_for_human_child).abs()
            > 1e-12,
    );
    push_gp(
        "human_mother_birth_mali_for_ai_child",
        (old.human_mother_birth_mali_for_ai_child - gp.human_mother_birth_mali_for_ai_child).abs()
            > 1e-12,
    );
    push_gp(
        "spawn_at_last_dead",
        old.spawn_at_last_dead != gp.spawn_at_last_dead,
    );
    push_gp(
        "temperature_own_tile_rate",
        (old.temperature_own_tile_rate - gp.temperature_own_tile_rate).abs() > 1e-12,
    );
    push_gp(
        "temperature_balance_rate",
        (old.temperature_balance_rate - gp.temperature_balance_rate).abs() > 1e-12,
    );
    push_gp(
        "temperature_local_heat_factor",
        (old.temperature_local_heat_factor - gp.temperature_local_heat_factor).abs() > 1e-12,
    );
    push_gp(
        "average_season_temperature_impact",
        (old.average_season_temperature_impact - gp.average_season_temperature_impact).abs()
            > 1e-12,
    );
    push_gp(
        "ai_total_score_factor",
        (old.ai_total_score_factor - gp.ai_total_score_factor).abs() > 1e-12,
    );
    push_gp(
        "old_grave_decay_mali",
        (old.old_grave_decay_mali - gp.old_grave_decay_mali).abs() > 1e-12,
    );
    push_gp(
        "cursed_grave_mali",
        (old.cursed_grave_mali - gp.cursed_grave_mali).abs() > 1e-12,
    );
    push_gp(
        "max_distance_close",
        old.max_distance_close != gp.max_distance_close,
    );
    push_gp(
        "max_distance_map_changes",
        old.max_distance_map_changes != gp.max_distance_map_changes,
    );
    push_gp(
        "max_distance_say",
        old.max_distance_say != gp.max_distance_say,
    );
    push_gp(
        "send_move_every_x_ticks",
        old.send_move_every_x_ticks != gp.send_move_every_x_ticks,
    );
    push_gp(
        "max_distance_cose_for_movement",
        old.max_distance_cose_for_movement != gp.max_distance_cose_for_movement,
    );
    push_gp(
        "max_distance_say_ai",
        (old.max_distance_say_ai - gp.max_distance_say_ai).abs() > 1e-12,
    );
    push_gp(
        "max_distance_auto_exile_attacker",
        old.max_distance_auto_exile_attacker != gp.max_distance_auto_exile_attacker,
    );
    push_gp(
        "speed_with_both_shoes",
        (old.speed_with_both_shoes - gp.speed_with_both_shoes).abs() > 1e-12,
    );
    push_gp(
        "aging_factor_while_starving",
        (old.aging_factor_while_starving - gp.aging_factor_while_starving).abs() > 1e-12,
    );
    push_gp(
        "grown_up_age",
        (old.grown_up_age - gp.grown_up_age).abs() > 1e-12,
    );
    push_gp(
        "food_use_child_faktor",
        (old.food_use_child_faktor - gp.food_use_child_faktor).abs() > 1e-12,
    );
    push_gp(
        "ai_food_use_factor_serf",
        (old.ai_food_use_factor_serf - gp.ai_food_use_factor_serf).abs() > 1e-12,
    );
    push_gp(
        "ai_food_use_factor_commoner",
        (old.ai_food_use_factor_commoner - gp.ai_food_use_factor_commoner).abs() > 1e-12,
    );
    push_gp(
        "ai_food_use_factor_noble",
        (old.ai_food_use_factor_noble - gp.ai_food_use_factor_noble).abs() > 1e-12,
    );
    push_gp(
        "eve_food_use_factor",
        (old.eve_food_use_factor - gp.eve_food_use_factor).abs() > 1e-12,
    );
    push_gp(
        "aging_factor_human_born_to_ai",
        (old.aging_factor_human_born_to_ai - gp.aging_factor_human_born_to_ai).abs() > 1e-12,
    );
    push_gp(
        "aging_factor_ai_born_to_human",
        (old.aging_factor_ai_born_to_human - gp.aging_factor_ai_born_to_human).abs() > 1e-12,
    );
    push_gp(
        "eve_damage_factor",
        (old.eve_damage_factor - gp.eve_damage_factor).abs() > 1e-12,
    );
    push_gp(
        "target_wounded_damage_factor",
        (old.target_wounded_damage_factor - gp.target_wounded_damage_factor).abs() > 1e-12,
    );
    push_gp(
        "male_damage_factor",
        (old.male_damage_factor - gp.male_damage_factor).abs() > 1e-12,
    );
    push_gp(
        "animal_damage_factor",
        (old.animal_damage_factor - gp.animal_damage_factor).abs() > 1e-12,
    );
    push_gp(
        "animal_damage_factor_in_winter",
        (old.animal_damage_factor_in_winter - gp.animal_damage_factor_in_winter).abs() > 1e-12,
    );
    push_gp(
        "animal_damage_factor_if_attacked",
        (old.animal_damage_factor_if_attacked - gp.animal_damage_factor_if_attacked).abs() > 1e-12,
    );
    push_gp(
        "weapon_damage_factor",
        (old.weapon_damage_factor - gp.weapon_damage_factor).abs() > 1e-12,
    );
    push_gp(
        "grave_blocking_distance",
        (old.grave_blocking_distance - gp.grave_blocking_distance).abs() > 1e-12,
    );
    push_gp(
        "max_players_before_activating_grave_curse",
        old.max_players_before_activating_grave_curse
            != gp.max_players_before_activating_grave_curse,
    );
    push_gp(
        "max_players_before_forbid_touch_grave",
        old.max_players_before_forbid_touch_grave != gp.max_players_before_forbid_touch_grave,
    );
    push_gp(
        "combat_angry_time_before_attack",
        (old.combat_angry_time_before_attack - gp.combat_angry_time_before_attack).abs() > 1e-12,
    );
    push_gp(
        "combat_angry_time_minimum",
        (old.combat_angry_time_minimum - gp.combat_angry_time_minimum).abs() > 1e-12,
    );
    push_gp(
        "chance_for_domestic_animal_dying_factor",
        (old.chance_for_domestic_animal_dying_factor
            - gp.chance_for_domestic_animal_dying_factor)
            .abs()
            > 1e-12,
    );
    push_gp("door_ids", old.door_ids != gp.door_ids);
    push_gp(
        "ai_ignored_floor_ids",
        old.ai_ignored_floor_ids != gp.ai_ignored_floor_ids,
    );
    push_gp("secret", old.secret != gp.secret);
    push_gp(
        "allow_debug_commands",
        old.allow_debug_commands != gp.allow_debug_commands,
    );
    push_gp(
        "debug_say_player_position",
        old.debug_say_player_position != gp.debug_say_player_position,
    );
    push_gp(
        "ai_time_to_wait_if_crafting_failed",
        (old.ai_time_to_wait_if_crafting_failed - gp.ai_time_to_wait_if_crafting_failed).abs()
            > 1e-12,
    );
    push_gp(
        "ai_max_search_radius",
        old.ai_max_search_radius != gp.ai_max_search_radius,
    );
    push_gp(
        "ai_memory_max_entries",
        old.ai_memory_max_entries != gp.ai_memory_max_entries,
    );
    push_gp(
        "ai_chat_memory_max_entries",
        old.ai_chat_memory_max_entries != gp.ai_chat_memory_max_entries,
    );
    push_gp(
        "ai_max_search_increment",
        old.ai_max_search_increment != gp.ai_max_search_increment,
    );
    push_gp(
        "ai_ignore_time_transitions_longer_then",
        (old.ai_ignore_time_transitions_longer_then - gp.ai_ignore_time_transitions_longer_then)
            .abs()
            > 1e-12,
    );
    push_gp(
        "alternative_outcome_percent_increase_per_hit",
        (old.alternative_outcome_percent_increase_per_hit
            - gp.alternative_outcome_percent_increase_per_hit)
            .abs()
            > 1e-12,
    );
    push_gp(
        "alternative_outcome_hits_decrease_on_success",
        (old.alternative_outcome_hits_decrease_on_success
            - gp.alternative_outcome_hits_decrease_on_success)
            .abs()
            > 1e-12,
    );
    push_gp(
        "fortification_cost_per_hit",
        (old.fortification_cost_per_hit - gp.fortification_cost_per_hit).abs() > 1e-12,
    );
    push_gp(
        "reduce_age_needed_to_pickup_objects",
        (old.reduce_age_needed_to_pickup_objects - gp.reduce_age_needed_to_pickup_objects).abs()
            > 1e-12,
    );
    push_gp(
        "chance_animals_pass_blocking_biome",
        (old.chance_animals_pass_blocking_biome - gp.chance_animals_pass_blocking_biome).abs()
            > 1e-12,
    );
    push_gp(
        "chance_preferred_biome",
        (old.chance_preferred_biome - gp.chance_preferred_biome).abs() > 1e-12,
    );
    push_gp(
        "close_grave_speed_mali",
        (old.close_grave_speed_mali - gp.close_grave_speed_mali).abs() > 1e-12,
    );
    push_gp(
        "temperature_speed_impact",
        (old.temperature_speed_impact - gp.temperature_speed_impact).abs() > 1e-12,
    );
    push_gp(
        "min_speed_reduction_per_contained_obj",
        (old.min_speed_reduction_per_contained_obj - gp.min_speed_reduction_per_contained_obj)
            .abs()
            > 1e-12,
    );
    push_gp(
        "loved_food_use_chance",
        (old.loved_food_use_chance - gp.loved_food_use_chance).abs() > 1e-12,
    );
    push_gp(
        "max_age_for_allowing_cloth_and_pickup_from_others",
        (old.max_age_for_allowing_cloth_and_pickup_from_others
            - gp.max_age_for_allowing_cloth_and_pickup_from_others)
            .abs()
            > 1e-12,
    );
    push_gp(
        "max_age_for_allowing_die",
        (old.max_age_for_allowing_die - gp.max_age_for_allowing_die).abs() > 1e-12,
    );
    push_gp(
        "prestige_cost_for_die",
        (old.prestige_cost_for_die - gp.prestige_cost_for_die).abs() > 1e-12,
    );
    push_gp(
        "starting_family_name",
        old.starting_family_name != gp.starting_family_name,
    );
    push_gp("starting_name", old.starting_name != gp.starting_name);
    push_gp(
        "found_family_needed_prestige",
        (old.found_family_needed_prestige - gp.found_family_needed_prestige).abs() > 1e-12,
    );
    push_gp(
        "found_family_cost",
        (old.found_family_cost - gp.found_family_cost).abs() > 1e-12,
    );
    push_gp(
        "found_family_needed_followers",
        old.found_family_needed_followers != gp.found_family_needed_followers,
    );
    push_gp(
        "found_family_break_alliance_chance",
        (old.found_family_break_alliance_chance - gp.found_family_break_alliance_chance).abs()
            > 1e-12,
    );
    push_gp(
        "pickup_exhaustion_gain",
        (old.pickup_exhaustion_gain - gp.pickup_exhaustion_gain).abs() > 1e-12,
    );
    push_gp(
        "pickup_feeding_food_restore",
        (old.pickup_feeding_food_restore - gp.pickup_feeding_food_restore).abs() > 1e-12,
    );
    push_gp(
        "death_with_food_store_max",
        (old.death_with_food_store_max - gp.death_with_food_store_max).abs() > 1e-12,
    );
    push_gp(
        "food_store_max_reduction_while_starving",
        (old.food_store_max_reduction_while_starving - gp.food_store_max_reduction_while_starving)
            .abs()
            > 1e-12,
    );
    push_gp(
        "temperature_reduction_per_drinking",
        (old.temperature_reduction_per_drinking - gp.temperature_reduction_per_drinking).abs()
            > 1e-12,
    );
    push_gp(
        "max_stored_water",
        (old.max_stored_water - gp.max_stored_water).abs() > 1e-12,
    );
    push_gp(
        "max_jumps_per_ten_sec",
        (old.max_jumps_per_ten_sec - gp.max_jumps_per_ten_sec).abs() > 1e-12,
    );
    push_gp(
        "temperature_impact_per_sec",
        (old.temperature_impact_per_sec - gp.temperature_impact_per_sec).abs() > 1e-12,
    );
    push_gp(
        "temperature_impact_per_sec_if_good",
        (old.temperature_impact_per_sec_if_good - gp.temperature_impact_per_sec_if_good).abs()
            > 1e-12,
    );
    push_gp(
        "temperature_in_water_factor",
        (old.temperature_in_water_factor - gp.temperature_in_water_factor).abs() > 1e-12,
    );
    push_gp(
        "temperature_impact_below",
        (old.temperature_impact_below - gp.temperature_impact_below).abs() > 1e-12,
    );
    push_gp(
        "temperature_impact_color_factor",
        (old.temperature_impact_color_factor - gp.temperature_impact_color_factor).abs() > 1e-12,
    );
    push_gp(
        "allow_eating_or_feeding_if_ill",
        old.allow_eating_or_feeding_if_ill != gp.allow_eating_or_feeding_if_ill,
    );
    push_gp(
        "resistance_against_fever_for_eating_mushrooms",
        (old.resistance_against_fever_for_eating_mushrooms
            - gp.resistance_against_fever_for_eating_mushrooms)
            .abs()
            > 1e-12,
    );
    push_gp(
        "exhaustion_yellow_fever_per_sec",
        (old.exhaustion_yellow_fever_per_sec - gp.exhaustion_yellow_fever_per_sec).abs() > 1e-12,
    );
    push_gp(
        "min_health_food_store_max_factor",
        (old.min_health_food_store_max_factor - gp.min_health_food_store_max_factor).abs() > 1e-12,
    );
    push_gp(
        "max_health_food_store_max_factor",
        (old.max_health_food_store_max_factor - gp.max_health_food_store_max_factor).abs() > 1e-12,
    );
    push_gp(
        "min_health_aging_factor",
        (old.min_health_aging_factor - gp.min_health_aging_factor).abs() > 1e-12,
    );
    push_gp(
        "max_health_aging_factor",
        (old.max_health_aging_factor - gp.max_health_aging_factor).abs() > 1e-12,
    );
    push_gp(
        "min_health_per_year",
        (old.min_health_per_year - gp.min_health_per_year).abs() > 1e-12,
    );
    push_gp(
        "max_age",
        (old.max_age - gp.max_age).abs() > 1e-12,
    );
    push_gp(
        "animal_deadly_distance_factor",
        (old.animal_deadly_distance_factor - gp.animal_deadly_distance_factor).abs() > 1e-12,
    );
    push_gp(
        "chance_for_animal_dying_factor_if_in_loved_biome",
        (old.chance_for_animal_dying_factor_if_in_loved_biome
            - gp.chance_for_animal_dying_factor_if_in_loved_biome)
            .abs()
            > 1e-12,
    );
    push_gp(
        "offspring_factor_if_animal_pop_is_low",
        (old.offspring_factor_if_animal_pop_is_low - gp.offspring_factor_if_animal_pop_is_low)
            .abs()
            > 1e-12,
    );
    push_gp(
        "max_offspring_factor",
        (old.max_offspring_factor - gp.max_offspring_factor).abs() > 1e-12,
    );
    push_gp(
        "offspring_factor_low_animal_population_below",
        (old.offspring_factor_low_animal_population_below
            - gp.offspring_factor_low_animal_population_below)
            .abs()
            > 1e-12,
    );
    state.gameplay = gp;
    // SOUL-LIVE-CAPS: live FIFO caps used by add_player_soul_* / LLM chat memory.
    state.ai_memory_max_entries = state.gameplay.ai_memory_max_entries.max(1) as usize;
    state.ai_chat_memory_max_entries = state.gameplay.ai_chat_memory_max_entries.max(1) as usize;

    // TWIN-MULTI-SERVER: re-sync peer list from live twin_peers (preserve last_pong).
    // Haxe: Connection.loginHelper TODO twins — multi-server peers product registry
    let changed_twins = state
        .twins
        .sync_endpoints(live.twin_peers.iter().map(|p| (p.host.as_str(), p.port)));
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

/// Haxe `TimeHelper.tick % 200 == 0` → `ServerSettings.readFromFile(false)`.
///
/// Re-reads `server.toml` when the file **mtime changed** (Rust analog of Haxe
/// always-reread: skip work when the file is unchanged). Applies live knobs
/// onto [`SimState`] and mirrors [`LiveSettings`] for NPC / outer tasks.
///
/// Omitted TOML keys keep compiled defaults (`#[serde(default)]`) — same idea as
/// Haxe `**default** name = value` lines that `readFromFile` skips.
// Haxe: TimeHelper.hx ~90–97
pub fn poll_and_apply_live_settings(
    state: &mut SimState,
    tracker: &mut HotReloadTracker,
    live_share: Option<&Arc<RwLock<LiveSettings>>>,
) {
    match tracker.poll(state.tick) {
        Ok(Some(res)) => {
            let report = apply_live_settings(state, &res.live);
            if let Some(share) = live_share {
                if let Ok(mut g) = share.write() {
                    *g = res.live;
                }
            }
            if !report.keys.is_empty() {
                tracing::info!(
                    keys = ?report.keys,
                    path = %tracker.path().display(),
                    "sim: server.toml live settings hot-reload"
                );
            } else if res.reloaded_from_disk {
                tracing::debug!(
                    path = %tracker.path().display(),
                    "sim: server.toml re-read (no live knob change)"
                );
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %tracker.path().display(),
                "sim: settings reload failed (keeping previous knobs)"
            );
        }
    }
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
    #[test]
    fn poll_and_apply_rereads_when_file_mtime_changes() {
        use std::fs;
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "ol_live_poll_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("server.toml");
        let mut cfg = ServerConfig::default();
        cfg.settings_reload_every_ticks = 10;
        cfg.yum_bonus = 5.0;
        fs::write(
            &path,
            "settings_hot_reload = true\nsettings_reload_every_ticks = 10\nyum_bonus = 5.0\n",
        )
        .unwrap();

        let mut tracker = HotReloadTracker::new(&path, cfg);
        let mut state = empty_state();
        state.tick = 10;
        poll_and_apply_live_settings(&mut state, &mut tracker, None);
        assert!((state.gameplay.yum_bonus - 5.0).abs() < f32::EPSILON);

        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut cfg2 = ServerConfig::default();
        cfg2.settings_reload_every_ticks = 10;
        cfg2.yum_bonus = 9.0;
        cfg2.starting_eve_age = 20.0;
        fs::write(
            &path,
            "settings_hot_reload = true\nsettings_reload_every_ticks = 10\nyum_bonus = 9.0\nstarting_eve_age = 20.0\n",
        )
        .unwrap();
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "\n# touch").unwrap();
        drop(f);

        state.tick = 20;
        poll_and_apply_live_settings(&mut state, &mut tracker, None);
        assert!(
            (state.gameplay.yum_bonus - 9.0).abs() < f32::EPSILON,
            "yum_bonus should follow the changed file"
        );
        assert!((state.gameplay.starting_eve_age - 20.0).abs() < f32::EPSILON);
        let _ = fs::remove_dir_all(&dir);
    }

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
    fn apply_live_settings_vanilla_id_map() {
        let mut state = empty_state();
        assert_eq!(state.last_vanilla_id, -1);
        assert_eq!(state.open_life_client_name, "OpenLife");
        let live = ServerConfig {
            last_vanilla_id: 4000,
            open_life_client_name: "OpenLifeClient".into(),
            ..Default::default()
        }
        .live_settings();
        let report = apply_live_settings(&mut state, &live);
        assert!(report.keys.contains(&"last_vanilla_id"));
        assert!(report.keys.contains(&"open_life_client_name"));
        assert_eq!(state.last_vanilla_id, 4000);
        assert_eq!(state.open_life_client_name, "OpenLifeClient");
        let report2 = apply_live_settings(&mut state, &live);
        assert!(!report2.keys.contains(&"last_vanilla_id"));
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
            biome_animal_hit_chance: 0.25,
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
            // SETTINGS-LONG-TAIL
            starting_eve_age: 18.0,
            eve_or_adam_birth_chance: 0.5,
            spawn_ai_as_eve: true,
            allow_humans_born_to_ais: true,
            max_players_before_starting_as_child: 3,
            obj_decay_chance: 0.0001,
            floor_decay_chance: 0.00002,
            obj_respawn_chance: 0.001,
            grow_back_plants_increase_if_low_population: 4.0,
            grow_back_original_plants_factor: 0.04,
            grow_new_plants_from_existing_factor: 0.1,
            spring_wild_food_regrow_chance: 2.0,
            winter_wild_food_decay_chance: 3.0,
            hot_season_temperature_factor: 1.5,
            cold_season_temperature_factor: 0.5,
            cursed_grave_time: 6.0,
            animal_decay_factor: 0.1,
            obj_decay_factor_for_permanent: 0.5,
            obj_decay_factor_for_food: 4.0,
            obj_decay_factor_for_clothing: 4.0,
            obj_decay_factor_for_walls: 0.4,
            obj_decay_factor_per_tech_level: 20.0,
            decay_factor_in_deep_water: 10.0,
            decay_factor_in_mountain: 6.0,
            decay_factor_in_walkable_water: 4.0,
            decay_factor_in_jungle: 4.0,
            decay_factor_in_swamp: 4.0,
            score_factor: 0.5,
            ancestor_prestige_factor: 0.4,
            display_score_factor: 2.0,
            display_score_on: false,
            max_coins_per_chest: 50,
            max_coins_per_pouch: 10,
            chance_for_female_child: 0.4,
            chance_for_other_child_color: 0.5,
            chance_for_other_child_color_if_close_to_wrong_special_biome: 0.8,
            little_kids_per_mother: 1,
            new_child_exhaustion_for_mother: 2.0,
            ai_mother_birth_mali_for_human_child: 5.0,
            human_mother_birth_mali_for_ai_child: 2.0,
            spawn_at_last_dead: true,
            temperature_own_tile_rate: 0.1,
            temperature_balance_rate: 0.5,
            temperature_local_heat_factor: 0.01,
            average_season_temperature_impact: 0.4,
            ai_total_score_factor: 0.5,
            old_grave_decay_mali: 8.0,
            cursed_grave_mali: 5.0,
            max_distance_close: 30,
            max_distance_map_changes: 12,
            max_distance_say: 18,
            send_move_every_x_ticks: 90,
            max_distance_cose_for_movement: 40,
            max_distance_say_ai: 12.0,
            max_distance_auto_exile_attacker: 7,
            speed_with_both_shoes: 1.4,
            aging_factor_while_starving: 0.25,
            grown_up_age: 16.0,
            food_use_child_faktor: 2.0,
            ai_food_use_factor_serf: 0.5,
            ai_food_use_factor_commoner: 0.7,
            ai_food_use_factor_noble: 1.2,
            eve_food_use_factor: 0.4,
            aging_factor_human_born_to_ai: 4.0,
            aging_factor_ai_born_to_human: 2.0,
            eve_damage_factor: 0.5,
            target_wounded_damage_factor: 0.4,
            male_damage_factor: 1.5,
            animal_damage_factor: 2.0,
            animal_damage_factor_in_winter: 3.0,
            animal_damage_factor_if_attacked: 2.5,
            weapon_damage_factor: 1.5,
            grave_blocking_distance: 50.0,
            max_players_before_activating_grave_curse: 3,
            max_players_before_forbid_touch_grave: 2,
            combat_angry_time_before_attack: 8.0,
            combat_angry_time_minimum: -30.0,
            chance_for_domestic_animal_dying_factor: 3.0,
            door_ids: vec![115, 876],
            ai_ignored_floor_ids: vec![656],
            secret: "TESTSECRET".into(),
            allow_debug_commands: false,
            debug_say_player_position: true,
            ai_time_to_wait_if_crafting_failed: 7.0,
            ai_max_search_radius: 40,
            ai_max_search_increment: 20,
            ai_ignore_time_transitions_longer_then: 90.0,
            ai_memory_max_entries: 3,
            ai_chat_memory_max_entries: 7,
            alternative_outcome_percent_increase_per_hit: 20.0,
            alternative_outcome_hits_decrease_on_success: 3.0,
            fortification_cost_per_hit: 2.0,
            reduce_age_needed_to_pickup_objects: 4.0,
            chance_animals_pass_blocking_biome: 0.1,
            chance_preferred_biome: 0.5,
            close_grave_speed_mali: 0.7,
            temperature_speed_impact: 0.5,
            min_speed_reduction_per_contained_obj: 0.9,
            loved_food_use_chance: 0.25,
            max_age_for_allowing_cloth_and_pickup_from_others: 8.0,
            max_age_for_allowing_die: 1.0,
            prestige_cost_for_die: 5.0,
            starting_family_name: "ICE".into(),
            starting_name: "FORK".into(),
            found_family_needed_prestige: 100.0,
            found_family_cost: 20.0,
            found_family_needed_followers: 8,
            found_family_break_alliance_chance: 0.25,
            pickup_exhaustion_gain: 0.4,
            pickup_feeding_food_restore: 2.0,
            death_with_food_store_max: -0.2,
            food_store_max_reduction_while_starving: 3.0,
            temperature_reduction_per_drinking: 0.25,
            max_stored_water: 2.0,
            max_jumps_per_ten_sec: 4.0,
            temperature_impact_per_sec: 0.05,
            temperature_impact_per_sec_if_good: 0.1,
            temperature_in_water_factor: 2.0,
            temperature_impact_below: 0.4,
            temperature_impact_color_factor: 0.25,
            allow_eating_or_feeding_if_ill: true,
            resistance_against_fever_for_eating_mushrooms: 0.5,
            exhaustion_yellow_fever_per_sec: 0.3,
            min_health_food_store_max_factor: 0.5,
            max_health_food_store_max_factor: 1.5,
            min_health_aging_factor: 0.25,
            max_health_aging_factor: 3.0,
            min_health_per_year: 2.0,
            max_age: 50.0,
            animal_deadly_distance_factor: 1.5,
            chance_for_animal_dying_factor_if_in_loved_biome: 0.5,
            offspring_factor_if_animal_pop_is_low: 5.0,
            max_offspring_factor: 2.0,
            offspring_factor_low_animal_population_below: 0.05,
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
        assert!((state.gameplay.biome_animal_hit_chance - 0.25).abs() < 1e-9);
        // FOLLOW-HIRE-DELAY
        assert!((state.gameplay.time_confirm_new_follower - 5.0).abs() < f32::EPSILON);
        assert!((state.gameplay.hire_cost - 25.0).abs() < f32::EPSILON);
        assert!(report.keys.contains(&"time_confirm_new_follower"));
        assert!(report.keys.contains(&"hire_cost"));
        // C-SS-MORE PrestigeCost* (ally + non-ally)
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_ally"));
        assert!(report.keys.contains(&"prestige_cost_per_damage_for_child"));
        assert!(report
            .keys
            .contains(&"prestige_cost_per_damage_for_elderly"));
        assert!(report
            .keys
            .contains(&"prestige_cost_per_damage_for_close_relatives"));
        assert!(report
            .keys
            .contains(&"prestige_cost_per_damage_for_women_without_weapon"));
        assert!((state.gameplay.prestige_cost_per_damage_for_child - 2.0).abs() < f32::EPSILON);
        assert!((state.gameplay.prestige_cost_per_damage_for_elderly - 3.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.prestige_cost_per_damage_for_close_relatives - 1.5).abs()
                < f32::EPSILON
        );
        assert!(
            (state
                .gameplay
                .prestige_cost_per_damage_for_women_without_weapon
                - 1.25)
                .abs()
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
        assert!(
            (state.gameplay.food_factor_eaten_less_than_one_percent - 3.0).abs() < f32::EPSILON
        );
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
        assert!(report
            .keys
            .contains(&"food_reduction_faktor_for_eating_high_quality"));
        assert!(report.keys.contains(&"combat_reputation_restore_per_year"));
        assert!((state.gameplay.grown_up_food_store_max - 25.0).abs() < f32::EPSILON);
        assert!((state.gameplay.min_biome_speed_factor - 0.15).abs() < f32::EPSILON);
        assert!((state.gameplay.hitpoints_speed_factor - 4.0).abs() < f32::EPSILON);
        assert!(
            (state.gameplay.food_reduction_faktor_for_eating_high_quality - 0.55).abs()
                < f32::EPSILON
        );
        assert!((state.gameplay.combat_reputation_restore_per_year - 3.5).abs() < f32::EPSILON);
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
        assert!(report
            .keys
            .contains(&"temperature_exhaustion_damage_factor"));
        assert!(report
            .keys
            .contains(&"max_movement_quad_jump_distance_before_force"));
        assert!(report.keys.contains(&"food_restore_factor_while_feeding"));
        assert!(report.keys.contains(&"max_has_eaten_for_next_generation"));
        assert!(report
            .keys
            .contains(&"has_eaten_reduction_for_next_generation"));
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
        assert!(report
            .keys
            .contains(&"close_enemy_with_weapon_speed_factor"));
        assert!(report.keys.contains(&"exhaustion_on_jump"));
        assert!(report.keys.contains(&"hungry_work_heat"));
        assert!(report.keys.contains(&"ai_speed_factor_serf"));
        assert!(report.keys.contains(&"ai_speed_factor_commoner"));
        assert!(report.keys.contains(&"ai_speed_factor_noble"));
        // SETTINGS-LONG-TAIL
        assert!(report.keys.contains(&"starting_eve_age"));
        assert!((state.gameplay.starting_eve_age - 18.0).abs() < f32::EPSILON);
        assert!(report.keys.contains(&"eve_or_adam_birth_chance"));
        assert!((state.gameplay.eve_or_adam_birth_chance - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"spawn_ai_as_eve"));
        assert!(state.gameplay.spawn_ai_as_eve);
        assert!(report.keys.contains(&"allow_humans_born_to_ais"));
        assert!(state.gameplay.allow_humans_born_to_ais);
        assert!(report.keys.contains(&"max_players_before_starting_as_child"));
        assert_eq!(state.gameplay.max_players_before_starting_as_child, 3);
        assert!(report.keys.contains(&"obj_decay_chance"));
        assert!(report.keys.contains(&"floor_decay_chance"));
        assert!(report.keys.contains(&"obj_respawn_chance"));
        assert!(report.keys.contains(&"grow_back_plants_increase_if_low_population"));
        assert!(report.keys.contains(&"grow_back_original_plants_factor"));
        assert!(report.keys.contains(&"spring_wild_food_regrow_chance"));
        assert!(report.keys.contains(&"winter_wild_food_decay_chance"));
        assert!(report.keys.contains(&"hot_season_temperature_factor"));
        assert!(report.keys.contains(&"cold_season_temperature_factor"));
        assert!((state.gameplay.obj_decay_chance - 0.0001).abs() < 1e-12);
        assert!((state.gameplay.floor_decay_chance - 0.00002).abs() < 1e-12);
        assert!((state.gameplay.obj_respawn_chance - 0.001).abs() < 1e-12);
        assert!(
            (state.gameplay.grow_back_plants_increase_if_low_population - 4.0).abs() < 1e-12
        );
        assert!((state.gameplay.grow_back_original_plants_factor - 0.04).abs() < 1e-12);
        assert!(report.keys.contains(&"grow_new_plants_from_existing_factor"));
        assert!((state.gameplay.grow_new_plants_from_existing_factor - 0.1).abs() < 1e-12);
        assert!((state.gameplay.spring_wild_food_regrow_chance - 2.0).abs() < 1e-12);
        assert!((state.gameplay.winter_wild_food_decay_chance - 3.0).abs() < 1e-12);
        assert!((state.gameplay.hot_season_temperature_factor - 1.5).abs() < 1e-12);
        assert!((state.gameplay.cold_season_temperature_factor - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"cursed_grave_time"));
        assert!((state.gameplay.cursed_grave_time - 6.0).abs() < f32::EPSILON);
        assert!(report.keys.contains(&"animal_decay_factor"));
        assert!((state.gameplay.animal_decay_factor - 0.1).abs() < 1e-12);
        assert!(report.keys.contains(&"obj_decay_factor_for_permanent"));
        assert!((state.gameplay.obj_decay_factor_for_permanent - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"obj_decay_factor_for_food"));
        assert!((state.gameplay.obj_decay_factor_for_food - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"obj_decay_factor_for_clothing"));
        assert!((state.gameplay.obj_decay_factor_for_clothing - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"obj_decay_factor_for_walls"));
        assert!((state.gameplay.obj_decay_factor_for_walls - 0.4).abs() < 1e-12);
        assert!(report.keys.contains(&"obj_decay_factor_per_tech_level"));
        assert!((state.gameplay.obj_decay_factor_per_tech_level - 20.0).abs() < 1e-12);
        assert!(report.keys.contains(&"decay_factor_in_deep_water"));
        assert!((state.gameplay.decay_factor_in_deep_water - 10.0).abs() < 1e-12);
        assert!(report.keys.contains(&"decay_factor_in_mountain"));
        assert!((state.gameplay.decay_factor_in_mountain - 6.0).abs() < 1e-12);
        assert!(report.keys.contains(&"decay_factor_in_walkable_water"));
        assert!((state.gameplay.decay_factor_in_walkable_water - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"decay_factor_in_jungle"));
        assert!((state.gameplay.decay_factor_in_jungle - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"decay_factor_in_swamp"));
        assert!((state.gameplay.decay_factor_in_swamp - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"score_factor"));
        assert!((state.gameplay.score_factor - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"ancestor_prestige_factor"));
        assert!(report.keys.contains(&"display_score_factor"));
        assert!(report.keys.contains(&"display_score_on"));
        assert!(report.keys.contains(&"max_coins_per_chest"));
        assert!(report.keys.contains(&"chance_for_female_child"));
        assert!(report.keys.contains(&"spawn_at_last_dead"));
        assert!(report.keys.contains(&"temperature_own_tile_rate"));
        assert!(report.keys.contains(&"average_season_temperature_impact"));
        assert!(report.keys.contains(&"ai_total_score_factor"));
        assert!((state.gameplay.ancestor_prestige_factor - 0.4).abs() < 1e-12);
        assert!((state.gameplay.display_score_factor - 2.0).abs() < 1e-12);
        assert!(!state.gameplay.display_score_on);
        assert_eq!(state.gameplay.max_coins_per_chest, 50);
        assert_eq!(state.gameplay.max_coins_per_pouch, 10);
        assert!((state.gameplay.chance_for_female_child - 0.4).abs() < 1e-12);
        assert!(state.gameplay.spawn_at_last_dead);
        assert!((state.gameplay.temperature_own_tile_rate - 0.1).abs() < 1e-12);
        assert!((state.gameplay.average_season_temperature_impact - 0.4).abs() < 1e-12);
        assert!((state.gameplay.ai_total_score_factor - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"old_grave_decay_mali"));
        assert!((state.gameplay.old_grave_decay_mali - 8.0).abs() < 1e-12);
        assert!(report.keys.contains(&"cursed_grave_mali"));
        assert!((state.gameplay.cursed_grave_mali - 5.0).abs() < 1e-12);
        assert!(report.keys.contains(&"max_distance_close"));
        assert_eq!(state.gameplay.max_distance_close, 30);
        assert_eq!(state.gameplay.max_distance_map_changes, 12);
        assert_eq!(state.gameplay.max_distance_say, 18);
        assert!(report.keys.contains(&"send_move_every_x_ticks"));
        assert_eq!(state.gameplay.send_move_every_x_ticks, 90);
        assert!(report.keys.contains(&"max_distance_cose_for_movement"));
        assert_eq!(state.gameplay.max_distance_cose_for_movement, 40);
        assert!(report.keys.contains(&"max_distance_say_ai"));
        assert!((state.gameplay.max_distance_say_ai - 12.0).abs() < 1e-12);
        assert!(report.keys.contains(&"max_distance_auto_exile_attacker"));
        assert_eq!(state.gameplay.max_distance_auto_exile_attacker, 7);
        assert!(report.keys.contains(&"speed_with_both_shoes"));
        assert!((state.gameplay.speed_with_both_shoes - 1.4).abs() < 1e-12);
        assert!(report.keys.contains(&"aging_factor_while_starving"));
        assert!((state.gameplay.aging_factor_while_starving - 0.25).abs() < 1e-12);
        assert!(report.keys.contains(&"grown_up_age"));
        assert!((state.gameplay.grown_up_age - 16.0).abs() < 1e-12);
        assert!(report.keys.contains(&"food_use_child_faktor"));
        assert!((state.gameplay.food_use_child_faktor - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"ai_food_use_factor_serf"));
        assert!(report.keys.contains(&"ai_food_use_factor_commoner"));
        assert!(report.keys.contains(&"ai_food_use_factor_noble"));
        assert!((state.gameplay.ai_food_use_factor_serf - 0.5).abs() < 1e-12);
        assert!((state.gameplay.ai_food_use_factor_commoner - 0.7).abs() < 1e-12);
        assert!((state.gameplay.ai_food_use_factor_noble - 1.2).abs() < 1e-12);
        assert!(report.keys.contains(&"eve_food_use_factor"));
        assert!((state.gameplay.eve_food_use_factor - 0.4).abs() < 1e-12);
        assert!(report.keys.contains(&"aging_factor_human_born_to_ai"));
        assert!(report.keys.contains(&"aging_factor_ai_born_to_human"));
        assert!((state.gameplay.aging_factor_human_born_to_ai - 4.0).abs() < 1e-12);
        assert!((state.gameplay.aging_factor_ai_born_to_human - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"eve_damage_factor"));
        assert!((state.gameplay.eve_damage_factor - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"target_wounded_damage_factor"));
        assert!((state.gameplay.target_wounded_damage_factor - 0.4).abs() < 1e-12);
        assert!(report.keys.contains(&"male_damage_factor"));
        assert!((state.gameplay.male_damage_factor - 1.5).abs() < 1e-12);
        assert!(report.keys.contains(&"animal_damage_factor"));
        assert!((state.gameplay.animal_damage_factor - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"animal_damage_factor_in_winter"));
        assert!((state.gameplay.animal_damage_factor_in_winter - 3.0).abs() < 1e-12);
        assert!(report.keys.contains(&"animal_damage_factor_if_attacked"));
        assert!((state.gameplay.animal_damage_factor_if_attacked - 2.5).abs() < 1e-12);
        assert!(report.keys.contains(&"weapon_damage_factor"));
        assert!((state.gameplay.weapon_damage_factor - 1.5).abs() < 1e-12);
        assert!(report.keys.contains(&"grave_blocking_distance"));
        assert!((state.gameplay.grave_blocking_distance - 50.0).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"max_players_before_activating_grave_curse"));
        assert_eq!(state.gameplay.max_players_before_activating_grave_curse, 3);
        assert!(report
            .keys
            .contains(&"max_players_before_forbid_touch_grave"));
        assert_eq!(state.gameplay.max_players_before_forbid_touch_grave, 2);
        assert!(report.keys.contains(&"combat_angry_time_before_attack"));
        assert!((state.gameplay.combat_angry_time_before_attack - 8.0).abs() < 1e-12);
        assert!((state.gameplay.combat_angry_time_before_attack_live() - 8.0).abs() < 1e-12);
        assert!(report.keys.contains(&"combat_angry_time_minimum"));
        assert!((state.gameplay.combat_angry_time_minimum + 30.0).abs() < 1e-12);
        assert!(report.keys.contains(&"chance_for_domestic_animal_dying_factor"));
        assert!((state.gameplay.chance_for_domestic_animal_dying_factor - 3.0).abs() < 1e-12);
        assert!(report.keys.contains(&"door_ids"));
        assert_eq!(state.gameplay.door_ids, vec![115, 876]);
        assert!(report.keys.contains(&"ai_ignored_floor_ids"));
        assert_eq!(state.gameplay.ai_ignored_floor_ids, vec![656]);
        assert!(report.keys.contains(&"secret"));
        assert_eq!(state.gameplay.secret, "TESTSECRET");
        assert!(report.keys.contains(&"allow_debug_commands"));
        assert!(!state.gameplay.allow_debug_commands);
        assert!(report.keys.contains(&"ai_time_to_wait_if_crafting_failed"));
        assert!((state.gameplay.ai_time_to_wait_if_crafting_failed - 7.0).abs() < 1e-12);
        assert!(report.keys.contains(&"ai_max_search_radius"));
        assert_eq!(state.gameplay.ai_max_search_radius, 40);
        assert!(report.keys.contains(&"ai_max_search_increment"));
        assert_eq!(state.gameplay.ai_max_search_increment, 20);
        assert!(report.keys.contains(&"ai_memory_max_entries"));
        assert_eq!(state.gameplay.ai_memory_max_entries, 3);
        assert_eq!(state.ai_memory_max_entries, 3);
        assert!(report.keys.contains(&"ai_chat_memory_max_entries"));
        assert_eq!(state.gameplay.ai_chat_memory_max_entries, 7);
        assert_eq!(state.ai_chat_memory_max_entries, 7);
        assert!(report
            .keys
            .contains(&"ai_ignore_time_transitions_longer_then"));
        assert!((state.gameplay.ai_ignore_time_transitions_longer_then - 90.0).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"alternative_outcome_percent_increase_per_hit"));
        assert!((state.gameplay.alternative_outcome_percent_increase_per_hit - 20.0).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"alternative_outcome_hits_decrease_on_success"));
        assert!((state.gameplay.alternative_outcome_hits_decrease_on_success - 3.0).abs() < 1e-12);
        assert!(report.keys.contains(&"fortification_cost_per_hit"));
        assert!((state.gameplay.fortification_cost_per_hit - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"reduce_age_needed_to_pickup_objects"));
        assert!((state.gameplay.reduce_age_needed_to_pickup_objects - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"chance_animals_pass_blocking_biome"));
        assert!((state.gameplay.chance_animals_pass_blocking_biome - 0.1).abs() < 1e-12);
        assert!(report.keys.contains(&"chance_preferred_biome"));
        assert!((state.gameplay.chance_preferred_biome - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"close_grave_speed_mali"));
        assert!((state.gameplay.close_grave_speed_mali - 0.7).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_speed_impact"));
        assert!((state.gameplay.temperature_speed_impact - 0.5).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"min_speed_reduction_per_contained_obj"));
        assert!((state.gameplay.min_speed_reduction_per_contained_obj - 0.9).abs() < 1e-12);
        assert!(report.keys.contains(&"loved_food_use_chance"));
        assert!((state.gameplay.loved_food_use_chance - 0.25).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"max_age_for_allowing_cloth_and_pickup_from_others"));
        assert!(
            (state
                .gameplay
                .max_age_for_allowing_cloth_and_pickup_from_others
                - 8.0)
                .abs()
                < 1e-12
        );
        assert!(report.keys.contains(&"max_age_for_allowing_die"));
        assert!((state.gameplay.max_age_for_allowing_die - 1.0).abs() < 1e-12);
        assert!(report.keys.contains(&"prestige_cost_for_die"));
        assert!((state.gameplay.prestige_cost_for_die - 5.0).abs() < 1e-12);
        assert!(report.keys.contains(&"starting_family_name"));
        assert_eq!(state.gameplay.starting_family_name, "ICE");
        assert!(report.keys.contains(&"starting_name"));
        assert_eq!(state.gameplay.starting_name, "FORK");
        assert!(report.keys.contains(&"found_family_needed_prestige"));
        assert!((state.gameplay.found_family_needed_prestige - 100.0).abs() < 1e-12);
        assert!(report.keys.contains(&"found_family_cost"));
        assert!((state.gameplay.found_family_cost - 20.0).abs() < 1e-12);
        assert!(report.keys.contains(&"found_family_needed_followers"));
        assert_eq!(state.gameplay.found_family_needed_followers, 8);
        assert!(report.keys.contains(&"found_family_break_alliance_chance"));
        assert!((state.gameplay.found_family_break_alliance_chance - 0.25).abs() < 1e-12);
        assert!(report.keys.contains(&"pickup_exhaustion_gain"));
        assert!((state.gameplay.pickup_exhaustion_gain - 0.4).abs() < 1e-12);
        assert!(report.keys.contains(&"pickup_feeding_food_restore"));
        assert!((state.gameplay.pickup_feeding_food_restore - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"death_with_food_store_max"));
        assert!((state.gameplay.death_with_food_store_max + 0.2).abs() < 1e-12);
        assert!((state.gameplay.death_with_food_store_max_live() + 0.2).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"food_store_max_reduction_while_starving"));
        assert!((state.gameplay.food_store_max_reduction_while_starving - 3.0).abs() < 1e-12);
        assert!((state.gameplay.food_store_max_knobs().starve_reduction - 3.0).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_reduction_per_drinking"));
        assert!((state.gameplay.temperature_reduction_per_drinking - 0.25).abs() < 1e-12);
        assert!(report.keys.contains(&"max_stored_water"));
        assert!((state.gameplay.max_stored_water - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"max_jumps_per_ten_sec"));
        assert!((state.gameplay.max_jumps_per_ten_sec - 4.0).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_impact_per_sec"));
        assert!((state.gameplay.temperature_impact_per_sec - 0.05).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_impact_per_sec_if_good"));
        assert!((state.gameplay.temperature_impact_per_sec_if_good - 0.1).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_in_water_factor"));
        assert!((state.gameplay.temperature_in_water_factor - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_impact_below"));
        assert!((state.gameplay.temperature_impact_below - 0.4).abs() < 1e-12);
        assert!(report.keys.contains(&"temperature_impact_color_factor"));
        assert!((state.gameplay.temperature_impact_color_factor - 0.25).abs() < 1e-12);
        assert!(report.keys.contains(&"allow_eating_or_feeding_if_ill"));
        assert!(state.gameplay.allow_eating_or_feeding_if_ill);
        assert!(report
            .keys
            .contains(&"resistance_against_fever_for_eating_mushrooms"));
        assert!((state.gameplay.resistance_against_fever_for_eating_mushrooms - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"exhaustion_yellow_fever_per_sec"));
        assert!((state.gameplay.exhaustion_yellow_fever_per_sec - 0.3).abs() < 1e-12);
        assert!(report.keys.contains(&"min_health_food_store_max_factor"));
        assert!((state.gameplay.min_health_food_store_max_factor - 0.5).abs() < 1e-12);
        assert!(report.keys.contains(&"max_health_food_store_max_factor"));
        assert!((state.gameplay.max_health_food_store_max_factor - 1.5).abs() < 1e-12);
        assert!(report.keys.contains(&"min_health_aging_factor"));
        assert!((state.gameplay.min_health_aging_factor - 0.25).abs() < 1e-12);
        assert!(report.keys.contains(&"max_health_aging_factor"));
        assert!((state.gameplay.max_health_aging_factor - 3.0).abs() < 1e-12);
        assert!(report.keys.contains(&"min_health_per_year"));
        assert!((state.gameplay.min_health_per_year - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"max_age"));
        assert!((state.gameplay.max_age - 50.0).abs() < 1e-12);
        assert!((state.gameplay.food_store_max_knobs().max_age - 50.0).abs() < 1e-12);
        assert!(report.keys.contains(&"animal_deadly_distance_factor"));
        assert!((state.gameplay.animal_deadly_distance_factor - 1.5).abs() < 1e-12);
        assert!(report
            .keys
            .contains(&"chance_for_animal_dying_factor_if_in_loved_biome"));
        assert!(
            (state.gameplay.chance_for_animal_dying_factor_if_in_loved_biome - 0.5).abs() < 1e-12
        );
        assert!(report.keys.contains(&"offspring_factor_if_animal_pop_is_low"));
        assert!((state.gameplay.offspring_factor_if_animal_pop_is_low - 5.0).abs() < 1e-12);
        assert!(report.keys.contains(&"max_offspring_factor"));
        assert!((state.gameplay.max_offspring_factor - 2.0).abs() < 1e-12);
        assert!(report.keys.contains(&"offspring_factor_low_animal_population_below"));
        assert!(
            (state.gameplay.offspring_factor_low_animal_population_below - 0.05).abs() < 1e-12
        );
        let vsk = state.gameplay.vitals_speed_live_knobs();
        assert!((vsk.close_grave_speed_mali - 0.7).abs() < 1e-12);
        assert!((vsk.temperature_speed_impact - 0.5).abs() < 1e-12);
        assert!((vsk.min_speed_reduction_per_contained_obj - 0.9).abs() < 1e-12);
        let (gdist, gcap) = state.gameplay.grave_curse_live_knobs();
        assert!((gdist - 50.0).abs() < 1e-12);
        assert_eq!(gcap, 3);
        let adk = state.gameplay.animal_damage_factor_knobs();
        assert!((adk.factor - 2.0).abs() < 1e-12);
        assert!((adk.winter - 3.0).abs() < 1e-12);
        assert!((adk.if_attacked - 2.5).abs() < 1e-12);
        let dck = state.gameplay.decay_chance_knobs();
        assert!((dck.obj_decay_chance - 0.0001).abs() < 1e-12);
        assert!((dck.floor_decay_chance - 0.00002).abs() < 1e-12);
        assert!((dck.animal_decay_factor - 0.1).abs() < 1e-12);
        assert!((dck.obj_decay_factor_for_permanent - 0.5).abs() < 1e-12);
        assert!((dck.obj_decay_factor_for_food - 4.0).abs() < 1e-12);
        assert!((dck.obj_decay_factor_for_clothing - 4.0).abs() < 1e-12);
        assert!((dck.obj_decay_factor_for_walls - 0.4).abs() < 1e-12);
        assert!((dck.obj_decay_factor_per_tech_level - 20.0).abs() < 1e-12);
        assert!((dck.decay_factor_in_deep_water - 10.0).abs() < 1e-12);
        assert!((dck.decay_factor_in_mountain - 6.0).abs() < 1e-12);
        assert!((dck.decay_factor_in_walkable_water - 4.0).abs() < 1e-12);
        assert!((dck.decay_factor_in_jungle - 4.0).abs() < 1e-12);
        assert!((dck.decay_factor_in_swamp - 4.0).abs() < 1e-12);
        assert!((dck.obj_respawn_chance - 0.001).abs() < 1e-12);
        assert!((dck.grow_back_plants_increase_if_low_population - 4.0).abs() < 1e-12);
        assert!((dck.grow_back_original_plants_factor - 0.04).abs() < 1e-12);
        assert!((dck.grow_new_from_existing_factor - 0.1).abs() < 1e-12);
        assert!((state.gameplay.weapon_cooldown_factor - 0.25).abs() < f32::EPSILON);
        assert!((state.gameplay.weapon_cooldown_factor_if_wounding - 4.0).abs() < f32::EPSILON);
        assert!((state.gameplay.close_enemy_with_weapon_speed_factor - 0.5).abs() < f32::EPSILON);
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
                    | "starting_eve_age"
                    | "eve_or_adam_birth_chance"
                    | "spawn_ai_as_eve"
                    | "allow_humans_born_to_ais"
                    | "max_players_before_starting_as_child"
                    | "obj_decay_chance"
                    | "floor_decay_chance"
                    | "obj_respawn_chance"
                    | "grow_back_plants_increase_if_low_population"
                    | "grow_back_original_plants_factor"
                    | "grow_new_plants_from_existing_factor"
                    | "spring_wild_food_regrow_chance"
                    | "winter_wild_food_decay_chance"
                    | "hot_season_temperature_factor"
                    | "cold_season_temperature_factor"
                    | "cursed_grave_time"
                    | "animal_decay_factor"
                    | "obj_decay_factor_for_permanent"
                    | "obj_decay_factor_for_food"
                    | "obj_decay_factor_for_clothing"
                    | "obj_decay_factor_for_walls"
                    | "obj_decay_factor_per_tech_level"
                    | "decay_factor_in_deep_water"
                    | "decay_factor_in_mountain"
                    | "decay_factor_in_walkable_water"
                    | "decay_factor_in_jungle"
                    | "decay_factor_in_swamp"
                    | "score_factor"
                    | "ancestor_prestige_factor"
                    | "display_score_factor"
                    | "display_score_on"
                    | "max_coins_per_chest"
                    | "max_coins_per_pouch"
                    | "chance_for_female_child"
                    | "chance_for_other_child_color"
                    | "chance_for_other_child_color_if_close_to_wrong_special_biome"
                    | "little_kids_per_mother"
                    | "new_child_exhaustion_for_mother"
                    | "ai_mother_birth_mali_for_human_child"
                    | "human_mother_birth_mali_for_ai_child"
                    | "spawn_at_last_dead"
                    | "temperature_own_tile_rate"
                    | "temperature_balance_rate"
                    | "temperature_local_heat_factor"
                    | "average_season_temperature_impact"
                    | "ai_total_score_factor"
                    | "old_grave_decay_mali"
                    | "cursed_grave_mali"
                    | "max_distance_close"
                    | "max_distance_map_changes"
                    | "max_distance_say"
                    | "send_move_every_x_ticks"
                    | "max_distance_cose_for_movement"
                    | "max_distance_say_ai"
                    | "max_distance_auto_exile_attacker"
                    | "speed_with_both_shoes"
                    | "aging_factor_while_starving"
                    | "grown_up_age"
                    | "food_use_child_faktor"
                    | "ai_food_use_factor_serf"
                    | "ai_food_use_factor_commoner"
                    | "ai_food_use_factor_noble"
                    | "eve_food_use_factor"
                    | "aging_factor_human_born_to_ai"
                    | "aging_factor_ai_born_to_human"
                    | "eve_damage_factor"
                    | "target_wounded_damage_factor"
                    | "male_damage_factor"
                    | "animal_damage_factor"
                    | "animal_damage_factor_in_winter"
                    | "animal_damage_factor_if_attacked"
                    | "weapon_damage_factor"
                    | "grave_blocking_distance"
                    | "max_players_before_activating_grave_curse"
                    | "max_players_before_forbid_touch_grave"
                    | "combat_angry_time_before_attack"
                    | "combat_angry_time_minimum"
                    | "chance_for_domestic_animal_dying_factor"
                    | "door_ids"
                    | "ai_ignored_floor_ids"
                    | "secret"
                    | "allow_debug_commands"
                    | "ai_time_to_wait_if_crafting_failed"
                    | "ai_max_search_radius"
                    | "ai_max_search_increment"
                    | "ai_ignore_time_transitions_longer_then"
                    | "ai_memory_max_entries"
                    | "ai_chat_memory_max_entries"
                    | "alternative_outcome_percent_increase_per_hit"
                    | "alternative_outcome_hits_decrease_on_success"
                    | "fortification_cost_per_hit"
                    | "reduce_age_needed_to_pickup_objects"
                    | "chance_animals_pass_blocking_biome"
                    | "chance_preferred_biome"
                    | "close_grave_speed_mali"
                    | "temperature_speed_impact"
                    | "min_speed_reduction_per_contained_obj"
                    | "loved_food_use_chance"
                    | "max_age_for_allowing_cloth_and_pickup_from_others"
                    | "max_age_for_allowing_die"
                    | "prestige_cost_for_die"
                    | "starting_family_name"
                    | "starting_name"
                    | "found_family_needed_prestige"
                    | "found_family_cost"
                    | "found_family_needed_followers"
                    | "found_family_break_alliance_chance"
                    | "pickup_exhaustion_gain"
                    | "pickup_feeding_food_restore"
                    | "death_with_food_store_max"
                    | "food_store_max_reduction_while_starving"
                    | "temperature_reduction_per_drinking"
                    | "max_stored_water"
                    | "max_jumps_per_ten_sec"
                    | "temperature_impact_per_sec"
                    | "temperature_impact_per_sec_if_good"
                    | "temperature_in_water_factor"
                    | "temperature_impact_below"
                    | "temperature_impact_color_factor"
                    | "allow_eating_or_feeding_if_ill"
                    | "resistance_against_fever_for_eating_mushrooms"
                    | "exhaustion_yellow_fever_per_sec"
                    | "min_health_food_store_max_factor"
                    | "max_health_food_store_max_factor"
                    | "min_health_aging_factor"
                    | "max_health_aging_factor"
                    | "min_health_per_year"
                    | "max_age"
                    | "animal_deadly_distance_factor"
                    | "chance_for_animal_dying_factor_if_in_loved_biome"
                    | "offspring_factor_if_animal_pop_is_low"
                    | "max_offspring_factor"
                    | "offspring_factor_low_animal_population_below"
            )
        }));
    }

    /// LiveSettings Min/MaxHealthFoodStoreMaxFactor change the live food-max factor.
    // Haxe: GlobalPlayerInstance.CalculateHealthFoodStoreMaxFactor
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_health_food_store_max_factor_overrides_compiled() {
        let mut state = empty_state();
        assert!((state.gameplay.min_health_food_store_max_factor - 0.8).abs() < 1e-12);
        assert!((state.gameplay.max_health_food_store_max_factor - 1.2).abs() < 1e-12);
        // yum 0, median 30, true_age 30 → mali branch (health = -30)
        let compiled = state.player_health_food_store_max_factor(1, 30.0);
        // (-30-30) / ((1/0.8)*(-30)-30) = -60 / -67.5 = 0.888…
        assert!((compiled - 0.888888).abs() < 1e-3);

        let live = ServerConfig {
            min_health_food_store_max_factor: 0.5,
            max_health_food_store_max_factor: 1.5,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        let overridden = state.player_health_food_store_max_factor(1, 30.0);
        // (-30-30) / ((1/0.5)*(-30)-30) = -60 / -90 = 0.666…
        assert!((overridden - 0.666666).abs() < 1e-3);
        assert!((overridden - compiled).abs() > 0.1);
    }

    /// LiveSettings Min/MaxHealthAgingFactor change the live health-age factor.
    // Haxe: GlobalPlayerInstance.CalculateHealthAgeFactor
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_health_aging_factor_overrides_compiled() {
        let mut state = empty_state();
        assert!((state.gameplay.min_health_aging_factor - 0.5).abs() < 1e-12);
        assert!((state.gameplay.max_health_aging_factor - 2.0).abs() < 1e-12);
        // yum 0, median 30, true_age 30 → mali branch (health = -30)
        let compiled = state.player_health_age_factor(1, 30.0);
        // (-30-30) / ((1/0.5)*(-30)-30) = -60 / -90 = 0.666…
        assert!((compiled - 0.666666).abs() < 1e-3);

        let live = ServerConfig {
            min_health_aging_factor: 0.25,
            max_health_aging_factor: 3.0,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        let overridden = state.player_health_age_factor(1, 30.0);
        // (-30-30) / ((1/0.25)*(-30)-30) = -60 / -150 = 0.4
        assert!((overridden - 0.4).abs() < 1e-3);
        assert!((overridden - compiled).abs() > 0.1);
    }

    /// LiveSettings MinHealthPerYear floors medianPrestige; MaxAge scales health-age factor.
    // Haxe: GlobalPlayerInstance.calculatePrestigeClass / CalculateHealthAgeFactor
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_min_health_per_year_and_max_age_overrides_compiled() {
        let mut state = empty_state();
        assert!((state.gameplay.min_health_per_year - 1.0).abs() < 1e-12);
        assert!((state.gameplay.max_age - 60.0).abs() < 1e-12);
        state.recompute_median_prestige();
        assert!((state.median_prestige - 30.0).abs() < 1e-12);
        let compiled = state.player_health_age_factor(1, 30.0);
        // health = 0 - 30*(30/30) = -30 → mali 0.5 → 0.666…
        assert!((compiled - 0.666666).abs() < 1e-3);

        let live = ServerConfig {
            min_health_per_year: 2.0,
            max_age: 30.0,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        state.recompute_median_prestige();
        assert!((state.median_prestige - 60.0).abs() < 1e-12);
        // restore compiled median so MaxAge is the only health-factor change
        state.median_prestige = 30.0;
        let overridden = state.player_health_age_factor(1, 30.0);
        // health = 0 - 30*(30/15) = -60 → mali 0.5 → (-90)/(-150) = 0.6
        assert!((overridden - 0.6).abs() < 1e-3);
        assert!((overridden - compiled).abs() > 0.05);
        let knobs = state.gameplay.food_store_max_knobs();
        assert!((knobs.max_age - 30.0).abs() < 1e-12);
        // old band starts > 20; age 25 → 10 + 5/10 * 10 = 15
        assert!(
            (crate::food_store_max_from_parts_ex(25.0, 10.0, 0.0, 0.0, 1.0, knobs) - 15.0)
                .abs()
                < 1e-4
        );
    }

    /// LiveSettings AnimalDeadlyDistanceFactor changes HIT combat range.
    // Haxe: ServerSettings.AnimalDeadlyDistanceFactor
    // SETTINGS-KNOB-TAIL
    #[test]
    fn average_season_temperature_impact_scales_identity_at_haxe_default() {
        let g = GameplayKnobs::default();
        assert!((g.scale_season_temperature_impact(0.4) - 0.4).abs() < 1e-12);
        let mut g2 = GameplayKnobs::default();
        g2.average_season_temperature_impact = 0.4;
        assert!((g2.scale_season_temperature_impact(0.4) - 0.8).abs() < 1e-12);
    }

    // SETTINGS-LONG-TAIL
    #[test]
    fn live_animal_deadly_distance_factor_overrides_compiled() {
        use crate::animals::AnimalKind;
        let mut state = empty_state();
        assert!((state.gameplay.animal_deadly_distance_factor - 0.5).abs() < 1e-12);
        let compiled =
            AnimalKind::Wolf.combat_profile_ex(state.gameplay.animal_deadly_distance_factor);
        assert!((compiled.deadly_distance - 0.5).abs() < 1e-12);

        let live = ServerConfig {
            animal_deadly_distance_factor: 1.5,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        let overridden =
            AnimalKind::Wolf.combat_profile_ex(state.gameplay.animal_deadly_distance_factor);
        assert!((overridden.deadly_distance - 1.5).abs() < 1e-12);
    }

    /// LiveSettings ChanceForAnimalDyingFactorIfInLovedBiome scales preferred-biome die chance.
    // Haxe: ServerSettings.ChanceForAnimalDyingFactorIfInLovedBiome
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_chance_for_animal_dying_factor_if_in_loved_biome_overrides_compiled() {
        use crate::animal_pop::{
            compute_dying_chance_ex, CHANCE_FOR_ANIMAL_DYING,
            CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME,
        };
        let mut state = empty_state();
        assert!(
            (state.gameplay.chance_for_animal_dying_factor_if_in_loved_biome
                - CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME)
                .abs()
                < 1e-12
        );
        let compiled = compute_dying_chance_ex(
            true,
            false,
            5,
            5,
            CHANCE_FOR_ANIMAL_DYING,
            state.gameplay.chance_for_animal_dying_factor_if_in_loved_biome,
        );
        assert!((compiled - CHANCE_FOR_ANIMAL_DYING * 0.1).abs() < 1e-12);

        let live = ServerConfig {
            chance_for_animal_dying_factor_if_in_loved_biome: 0.5,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        let overridden = compute_dying_chance_ex(
            true,
            false,
            5,
            5,
            CHANCE_FOR_ANIMAL_DYING,
            state.gameplay.chance_for_animal_dying_factor_if_in_loved_biome,
        );
        assert!((overridden - CHANCE_FOR_ANIMAL_DYING * 0.5).abs() < 1e-12);
        assert!((overridden - compiled).abs() > 1e-8);
    }

    /// LiveSettings OffspringFactorIfAnimalPopIsLow / MaxOffspringFactor change pop rolls.
    // Haxe: ServerSettings.OffspringFactorIfAnimalPopIsLow / MaxOffspringFactor
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_offspring_pop_factors_override_compiled() {
        use crate::animal_pop::{
            compute_offspring_chance_ex, offspring_pop_allows, CHANCE_FOR_OFFSPRING,
            OFFSPRING_FACTOR_IF_POP_LOW,
        };
        let mut state = empty_state();
        assert!((state.gameplay.offspring_factor_if_animal_pop_is_low - 10.0).abs() < 1e-12);
        assert!((state.gameplay.max_offspring_factor - 1.0).abs() < 1e-12);
        let compiled = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            state.gameplay.offspring_factor_if_animal_pop_is_low,
            state.gameplay.offspring_factor_low_animal_population_below,
        );
        assert!((compiled - CHANCE_FOR_OFFSPRING * OFFSPRING_FACTOR_IF_POP_LOW).abs() < 1e-12);
        assert!(offspring_pop_allows(9, 10, state.gameplay.max_offspring_factor));
        assert!(!offspring_pop_allows(10, 10, state.gameplay.max_offspring_factor));

        let live = ServerConfig {
            offspring_factor_if_animal_pop_is_low: 5.0,
            max_offspring_factor: 2.0,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        let overridden = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            state.gameplay.offspring_factor_if_animal_pop_is_low,
            state.gameplay.offspring_factor_low_animal_population_below,
        );
        assert!((overridden - CHANCE_FOR_OFFSPRING * 5.0).abs() < 1e-12);
        assert!(offspring_pop_allows(19, 10, state.gameplay.max_offspring_factor));
        assert!(!offspring_pop_allows(20, 10, state.gameplay.max_offspring_factor));
    }

    /// LiveSettings OffspringFactorLowAnimalPopulationBelow changes the low-pop birth gate.
    // Haxe: ServerSettings.OffspringFactorLowAnimalPopulationBelow
    // SETTINGS-LONG-TAIL
    #[test]
    fn live_offspring_factor_low_animal_population_below_overrides_compiled() {
        use crate::animal_pop::{compute_offspring_chance_ex, CHANCE_FOR_OFFSPRING};
        let mut state = empty_state();
        assert!(
            (state.gameplay.offspring_factor_low_animal_population_below - 0.2).abs() < 1e-12
        );
        // current 1, original 10 → 1 < 10*0.2 → boost
        let compiled = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            10.0,
            state.gameplay.offspring_factor_low_animal_population_below,
        );
        assert!((compiled - CHANCE_FOR_OFFSPRING * 10.0).abs() < 1e-12);

        let live = ServerConfig {
            offspring_factor_low_animal_population_below: 0.05,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        // 1 < 10*0.05 is false → no boost
        let overridden = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            10.0,
            state.gameplay.offspring_factor_low_animal_population_below,
        );
        assert!((overridden - CHANCE_FOR_OFFSPRING).abs() < 1e-12);
        assert!((overridden - compiled).abs() > 1e-8);
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
        // SETTINGS-LONG-TAIL
        let eve = find_critical("StartingEveAge").expect("StartingEveAge");
        assert_eq!(eve.home, SettingsHome::Live);
        for name in [
            "EveOrAdamBirthChance",
            "SpawnAiAsEve",
            "ObjDecayChance",
            "FloorDecayChance",
            "CursedGraveTime",
            "AnimalDecayFactor",
            "ObjDecayFactorForFood",
            "ObjDecayFactorForClothing",
            "ObjDecayFactorForWalls",
            "ObjDecayFactorPerTechLevel",
            "DecayFactorInDeepWater",
            "DecayFactorInMountain",
            "DecayFactorInWalkableWater",
            "DecayFactorInJungle",
            "DecayFactorInSwamp",
            "ObjRespawnChance",
            "GrowBackPlantsIncreaseIfLowPopulation",
            "ScoreFactor",
            "SendMoveEveryXTicks",
            "MaxDistanceToBeConsideredAsCoseForMovement",
            "MaxDistanceToBeConsideredAsCloseForSayAi",
            "SpeedWithBothShoes",
            "AgingFactorWhileStarvingToDeath",
            "GrownUpAge",
            "FoodUseChildFaktor",
            "AIFoodUseFactorSerf",
            "AIFoodUseFactorCommoner",
            "AIFoodUseFactorNoble",
            "EveFoodUseFactor",
            "EveDamageFactor",
            "TargetWoundedDamageFactor",
            "MaleDamageFactor",
            "AnimalDamageFactor",
            "AnimalDamageFactorInWinter",
            "AnimalDamageFactorIfAttacked",
            "WeaponDamageFactor",
            "GraveBlockingDistance",
            "MaxPlayersBeforeActivatingGraveCurse",
            "CombatAngryTimeBeforeAttack",
            "AiTimeToWaitIfCraftingFailed",
            "AiMaxSearchRadius",
            "AiMaxSearchIncrement",
            "AiIgnoreTimeTransitionsLongerThen",
            "AlternativeOutcomePercentIncreasePerHit",
            "AlternativeOutcomeHitsDecreaseOnSucess",
            "ChanceThatAnimalsCanPassBlockingBiome",
            "chancePreferredBiome",
            "CloseGraveSpeedMali",
            "TemperatureSpeedImpact",
            "MinSpeedReductionPerContainedObj",
            "LovedFoodUseChance",
            "MaxAgeForAllowingClothAndPrickupFromOthers",
            "MaxAgeForAllowingDie",
            "PrestigeCostForDie",
            "StartingFamilyName",
            "StartingName",
            "FoundFamilyNeededPrestige",
            "FoundFamilyCost",
            "FoundFamilyNeededFollowers",
            "FoundFamilyBreakAllianceChance",
            "PickupExhaustionGain",
            "PickupFeedingFoodRestore",
            "DeathWithFoodStoreMax",
            "FoodStoreMaxReductionWhileStarvingToDeath",
            "TemperatureReductionPerDrinking",
            "MaxStoredWater",
            "MaxJumpsPerTenSec",
            "TemperatureImpactPerSec",
            "TemperatureImpactPerSecIfGood",
            "TemperatureInWaterFactor",
            "TemperatureImpactBelow",
            "TemperatureImpactColorFactor",
            "AllowEatingOrFeedingIfIll",
            "ResistanceAgainstFeverForEatingMushrooms",
            "ExhaustionYellowFeverPerSec",
            "MinHealthFoodStoreMaxFactor",
            "MaxHealthFoodStoreMaxFactor",
            "MinHealthAgingFactor",
            "MaxHealthAgingFactor",
            "MinHealthPerYear",
            "MaxAge",
            "AnimalDeadlyDistanceFactor",
            "ChanceForAnimalDyingFactorIfInLovedBiome",
            "OffspringFactorIfAnimalPopIsLow",
            "MaxOffspringFactor",
            "OffspringFactorLowAnimalPopulationBelow",
            "ObjDecayFactorForFood",
            "ObjDecayFactorForClothing",
            "ObjDecayFactorForWalls",
            "ObjDecayFactorPerTechLevel",
            "DecayFactorInDeepWater",
            "DecayFactorInMountain",
            "DecayFactorInWalkableWater",
            "DecayFactorInJungle",
            "DecayFactorInSwamp",
            "ObjRespawnChance",
            "GrowBackPlantsIncreaseIfLowPopulation",
            "GrowBackOriginalPlantsFactor",
            "SpringWildFoodRegrowChance",
            "WinterWildFoodDecayChance",
            "HotSeasonTemperatureFactor",
            "ColdSeasonTemperatureFactor",
        ] {
            let e = find_critical(name).expect(name);
            assert_eq!(e.home, SettingsHome::Live, "{name}");
        }
    }
}
