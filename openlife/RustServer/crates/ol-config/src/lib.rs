//! Server configuration loaded from TOML.
//!
//! Haxe: `openlife/settings/ServerSettings.hx` (`readFromFile` / `writeToFile`) +
//! `TimeHelper.ReadServerSettings` hot-reload every ~200 ticks.

#![forbid(unsafe_code)]

mod field_map;

pub use field_map::{
    find_critical, gameplay_defaults, is_ai_ignored_floor_id, is_ai_ignored_floor_id_in, is_door_id,
    is_door_id_in, live_critical_names, module_const_critical_names, secret_omit_names, FieldEntry,
    SettingsHome, AI_IGNORED_FLOOR_IDS, CRITICAL_FIELD_MAP, DOOR_IDS,
};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
}

/// Haxe year proxy used by `ServerSettings.SeasonDuration` (1 year ≈ 60 real seconds).
pub const HAXE_YEAR_SECS: f32 = 60.0;

/// Default Haxe `TimeHelper` settings-reload period (ticks @ 20 Hz ≈ 10 s).
pub const DEFAULT_SETTINGS_RELOAD_EVERY_TICKS: u64 = 200;

// Haxe: ServerSettings.LockpickSucessChance / FailChance / ExhaustionCost / CoinCost
/// Haxe `LockpickSucessChance` default (%).
pub const DEFAULT_LOCKPICK_SUCCESS_CHANCE: f32 = 5.0;
/// Haxe `LockpickFailChance` default (%).
pub const DEFAULT_LOCKPICK_FAIL_CHANCE: f32 = 10.0;
/// Haxe `LockpickExhaustionCost` default.
pub const DEFAULT_LOCKPICK_EXHAUSTION_COST: f32 = 3.0;
/// Haxe `LockpickCoinCost` default.
pub const DEFAULT_LOCKPICK_COIN_COST: f32 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub game_port: u16,
    pub web_port: u16,
    pub bind: String,
    pub max_players: u32,
    pub required_version: i32,
    /// When true, LOGIN with a numeric `client_tag` that does not match
    /// [`Self::required_version`] is hard-rejected (PS reject + no spawn).
    /// Default **false** (soft-log only). Normal OHOL tags like `client_official`
    /// are not numeric and are never treated as a version mismatch.
    pub client_version_strict: bool,
    /// Haxe `lastVanillaID` — highest vanilla object id. `< 1` disables mapping.
    // Haxe: ServerSettings.lastVanillaID = -1
    pub last_vanilla_id: i32,
    /// Haxe `OpenLifeClientName` — LOGIN `client_tag` substring that skips remap.
    // Haxe: ServerSettings.OpenLifeClientName = "OpenLife"
    pub open_life_client_name: String,
    pub content_path: PathBuf,
    pub challenge_len: usize,
    pub tick_hz: u32,
    /// Time dilation: multiplies `dt` passed into sim vitals (`1.0` = realtime).
    /// Values `> 1` speed up aging/food/etc.; `0` freezes vitals time (same as pause).
    pub sim_speed: f32,
    pub enable_game_net: bool,
    pub enable_web: bool,

    /// Mirror Haxe ticket-server account check on LOGIN. Default **on**.
    pub verify_ohol_ticket: bool,
    /// Ticket endpoint (Haxe default host path).
    pub ticket_verify_url: String,

    /// PNG biome map (Haxe `MapFileName`).
    pub map_png_path: PathBuf,
    /// Save directory for versioned binary world (Haxe `SaveDirectory`).
    pub save_directory: PathBuf,
    /// If true and no save exists, generate from PNG + natural objects.
    pub generate_map_if_missing: bool,
    /// Force regenerate even if save exists.
    pub force_regenerate_map: bool,
    /// Density factor for natural object placement (Haxe ~0.4 gate).
    pub natural_object_density: f32,

    /// Spawn in-process self-play agents (dev / viewer). Default **on**.
    pub selfplay_enabled: bool,
    /// Number of self-play agents to spawn (clamped to 1–3): Forager, +Farmer, +Hunter.
    pub selfplay_agents: u8,
    /// Cap on transitions seeded into the reverse craft graph at boot (fast restart).
    pub craft_graph_seed_cap: usize,

    /// Multi-server twin peer endpoints (listed in sim; pongs via TWINPONG / future sockets).
    ///
    /// Empty by default. Seeded into `TwinRegistry` at boot and re-synced on LiveSettings
    /// hot-reload (`SAY ?TWINS`). Inter-server TCP/UDP sockets remain residual.
    pub twin_peers: Vec<TwinPeerConfig>,

    /// Timed multi-tile MovePath + PM (Haxe MoveHelper). Default **on** (Haxe-like; not instant).
    pub timed_movement: bool,
    /// AI craft search radius (tiles) for bottom-up valuation.
    pub ai_craft_radius: i32,
    /// Instant MOVE only: max Chebyshev snap of start tile (default 2).
    /// Timed MovePath uses Haxe `MaxMovementQuadJumpDistanceBeforeForce` (quadDist ≤ 5)
    /// and ignores this field — do not raise it to “widen” timed jumps.
    pub move_jump_max_chebyshev: i32,
    /// Max intents applied per tick wake (fairness under self-play/AI flood).
    pub intent_drain_budget: u32,
    /// Ops series sample every N ticks (100 @ 20 Hz ≈ 5 s).
    pub ops_sample_every_ticks: u64,
    /// Flush ops journal every N seconds (default 300).
    pub ops_flush_secs: u64,
    /// Path for ops metrics journal under SaveFiles.
    pub ops_journal_path: PathBuf,
    /// AI NPC scheduler (default **on** — floor `npc_min` agents when enabled).
    pub npc_enabled: bool,
    /// When `npc_enabled`, floor population (Forager/Farmer/Hunter-style).
    /// Haxe `MinNumberOfAis` / adaptive floor.
    pub npc_min: u32,
    /// Adaptive AI population ceiling (Haxe `NumberOfAis`).
    pub npc_max: u32,
    /// Each NPC thinks every N ticks (stagger by p_id). Fallback floor when
    /// class reaction times are used (see `ai_reaction_time*`).
    pub ai_think_period_ticks: u32,
    /// Haxe `AiReactionTime` — Commoner AI react delay (seconds).
    pub ai_reaction_time: f32,
    /// Haxe `AiReactionTimeSerf`.
    pub ai_reaction_time_serf: f32,
    /// Haxe `AiReactionTimeNoble` (+ King/Emperor).
    pub ai_reaction_time_noble: f32,
    /// Haxe `AiReactionTimeFactorIfAngry` — multiplies reaction while angry.
    pub ai_reaction_time_factor_if_angry: f32,
    /// Observation radius (tiles) for AI brain snapshot.
    pub ai_observe_radius: i32,
    /// When true, MX/PU fan-out to all connected clients (ignore distance).
    pub broadcast_all_updates: bool,
    /// `SAY !shutdown` global countdown seconds before save + apocalypse (default 3).
    pub shutdown_countdown_secs: u32,
    /// Seconds to display apocalypse signal after save before orderly exit (default 3).
    pub shutdown_apocalypse_secs: u32,

    // --- CONFIG-SETTINGS / Haxe ServerSettings season + hot-reload ---

    /// Haxe `ServerSettings.EternalWinter` — force winter season while true.
    pub eternal_winter: bool,
    /// Haxe `ServerSettings.SeasonDuration` in years (1 year ≈ 60 real seconds).
    /// Converted to sim `season_length` via [`Self::season_length_secs`].
    pub season_duration_years: f32,
    /// Haxe `TimeHelper.ReadServerSettings` — when true, re-read `server.toml` every
    /// [`Self::settings_reload_every_ticks`] sim ticks.
    pub settings_hot_reload: bool,
    /// Haxe `tick % 200 == 0` cadence for settings re-read (default 200 @ 20 Hz ≈ 10 s).
    pub settings_reload_every_ticks: u64,

    // --- LOCKPICK-SETTINGS / Haxe ServerSettings.Lockpick* (live) ---

    /// Haxe `ServerSettings.LockpickSucessChance` — success roll band in percent.
    // Haxe: ServerSettings.LockpickSucessChance (typo Sucess preserved in Haxe name)
    pub lockpick_success_chance: f32,
    /// Haxe `ServerSettings.LockpickFailChance` — break-key band from top of 0..100 roll.
    pub lockpick_fail_chance: f32,
    /// Haxe `ServerSettings.LockpickExhaustionCost` — added to player exhaustion per attempt.
    pub lockpick_exhaustion_cost: f32,
    /// Haxe `ServerSettings.LockpickCoinCost` — coins deducted per attempt.
    pub lockpick_coin_cost: f32,

    // --- SETTINGS-FIELD-MAP gameplay knobs (Haxe ServerSettings statics; live) ---

    /// Haxe `ServerSettings.FoodUsePerSecond` — base food drain /s.
    // Haxe: ServerSettings.FoodUsePerSecond
    pub food_use_per_second: f32,
    /// Haxe `ServerSettings.HealingPerSecond`.
    // Haxe: ServerSettings.HealingPerSecond
    pub healing_per_second: f32,
    /// Haxe `ServerSettings.AgeingSecondsPerYear`.
    // Haxe: ServerSettings.AgeingSecondsPerYear
    pub ageing_seconds_per_year: f32,
    /// Haxe `ServerSettings.InitialPlayerMoveSpeed` (tiles/s).
    // Haxe: ServerSettings.InitialPlayerMoveSpeed
    pub initial_player_move_speed: f32,
    /// Haxe `ServerSettings.SpeedFactor` (global move mult).
    // Haxe: ServerSettings.SpeedFactor
    pub speed_factor: f32,
    /// Haxe `ServerSettings.YumBonus` — first-eat yum charge band.
    // Haxe: ServerSettings.YumBonus
    pub yum_bonus: f32,
    /// Haxe `ServerSettings.ChanceForOffspring` per animal move.
    // Haxe: ServerSettings.ChanceForOffspring
    pub chance_for_offspring: f32,
    /// Haxe `ServerSettings.ChanceForAnimalDying` per animal move.
    // Haxe: ServerSettings.ChanceForAnimalDying
    pub chance_for_animal_dying: f32,
    /// Haxe `ServerSettings.BiomeAnimalHitChance` (DoDamage miss gate).
    // Haxe: ServerSettings.BiomeAnimalHitChance = 0.0
    // MOSQUITO-MAPCHANCE
    pub biome_animal_hit_chance: f32,
    /// Haxe `ServerSettings.HungryWorkCost` base food gate.
    // Haxe: ServerSettings.HungryWorkCost
    pub hungry_work_cost: f32,
    /// Haxe `ServerSettings.BirthPrestigeFactor`.
    // Haxe: ServerSettings.BirthPrestigeFactor
    pub birth_prestige_factor: f32,
    /// Haxe `ServerSettings.AllyStrenghTooLowForPickup` (typo Strengh; 0 = disabled).
    // Haxe: ServerSettings.AllyStrenghTooLowForPickup
    pub ally_strength_too_low_for_pickup: f32,
    /// Haxe `ServerSettings.TimeConfirmNewFollower` — delayed I FOLLOW confirm (seconds).
    // Haxe: ServerSettings.TimeConfirmNewFollower
    // FOLLOW-HIRE-DELAY
    pub time_confirm_new_follower: f32,
    /// Haxe `ServerSettings.HireCost` base coins for I HIRE.
    // Haxe: ServerSettings.HireCost
    pub hire_cost: f32,
    /// Haxe `ServerSettings.HireCostIncreasePerPerson`.
    // Haxe: ServerSettings.HireCostIncreasePerPerson
    pub hire_cost_increase_per_person: f32,
    /// Haxe `ServerSettings.AutoFollowPlayer` — AI acquire closest human when sticky empty.
    // Haxe: ServerSettings.AutoFollowPlayer = false
    // AI-FOLLOW-ACQUIRE
    pub auto_follow_player: bool,
    /// Haxe `ServerSettings.PrestigeCostPerDamageForAlly`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly
    // PRESTIGE-ALLY-COST
    pub prestige_cost_per_damage_for_ally: f32,
    /// Haxe `ServerSettings.PrestigeCostPerDamageForChild`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForChild
    // C-SS-MORE
    pub prestige_cost_per_damage_for_child: f32,
    /// Haxe `ServerSettings.PrestigeCostPerDamageForElderly`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForElderly
    // C-SS-MORE
    pub prestige_cost_per_damage_for_elderly: f32,
    /// Haxe `ServerSettings.PrestigeCostPerDamageForCloseRelatives`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForCloseRelatives
    // C-SS-MORE
    pub prestige_cost_per_damage_for_close_relatives: f32,
    /// Haxe `ServerSettings.PrestigeCostPerDamageForWomenWithoutWeapon`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForWomenWithoutWeapon
    // C-SS-MORE
    pub prestige_cost_per_damage_for_women_without_weapon: f32,
    // --- C-SS-FULL-TABLE / settings_long_tail: FoodFactor + eat bands + YumFoodRestore ---
    /// Haxe `ServerSettings.FoodFactor` — global fill scale in compute_eat.
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
    /// Haxe `HungryWorkHeat` — heat per food when transition temperature &lt; 0.
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
    /// Humans may spawn as children of AI mothers. Default false (Eve if no human mother).
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
    // SETTINGS-LONG-TAIL
    pub ancestor_prestige_factor: f32,
    /// Haxe `DisplayScoreFactor`.
    // SETTINGS-LONG-TAIL
    pub display_score_factor: f32,
    /// Haxe `DisplayScoreOn` — extra age-58 / last-life prestige GMs.
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
    // SETTINGS-LONG-TAIL
    pub ai_total_score_factor: f32,
    /// Haxe `OldGraveDecayMali`.
    // SETTINGS-LONG-TAIL
    pub old_grave_decay_mali: f32,
    /// Haxe `CursedGraveMali`.
    // SETTINGS-LONG-TAIL
    pub cursed_grave_mali: f32,
    /// Haxe `MaxDistanceToBeConsideredAsClose`.
    pub max_distance_close: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForMapChanges`.
    pub max_distance_map_changes: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSay`.
    pub max_distance_say: i32,
    /// Haxe `SendMoveEveryXTicks` (`-1` = disabled; `> 0` = period).
    // Haxe: ServerSettings.SendMoveEveryXTicks = -1
    // SETTINGS-LONG-TAIL
    pub send_move_every_x_ticks: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCoseForMovement` (typo Cose) — PM fan radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCoseForMovement = 30
    // SETTINGS-LONG-TAIL
    pub max_distance_cose_for_movement: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSayAi` — AI sayHelper hear radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi = 20
    // SETTINGS-LONG-TAIL
    pub max_distance_say_ai: f32,
    /// Haxe `MaxDistanceToAutoExileAttacker` (quad-distance compare).
    pub max_distance_auto_exile_attacker: i32,
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
    /// Default **false** so normal play does not show coordinates.
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
    /// Haxe `AiMemoryMaxEntries` — PlayerSoul interaction FIFO.
    // Haxe: ServerSettings.AiMemoryMaxEntries = 20
    // SOUL-LIVE-CAPS
    pub ai_memory_max_entries: i32,
    /// Haxe `AiChatMemoryMaxEntries` — PlayerSoul chat FIFO.
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
    /// Haxe `FortificationCosePerHit` (coin cost = floor(fortValue * this)).
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
    /// Haxe `MaxAgeForAllowingDie` — SAY/client DIE allowed while age <= this.
    // Haxe: ServerSettings.MaxAgeForAllowingDie = 2
    // SETTINGS-LONG-TAIL
    pub max_age_for_allowing_die: f32,
    /// Haxe `PrestigeCostForDie` — account.score must be >= this to /DIE; not debited.
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

/// One configured twin peer host:port (no last_pong — that lives in the sim stub registry).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TwinPeerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            game_port: 8005,
            web_port: 8080,
            bind: "0.0.0.0".into(),
            max_players: 200,
            required_version: 437,
            client_version_strict: false,
            last_vanilla_id: gameplay_defaults::LAST_VANILLA_ID,
            open_life_client_name: gameplay_defaults::OPEN_LIFE_CLIENT_NAME.into(),
            content_path: PathBuf::from("content/OneLifeData7"),
            challenge_len: 48,
            tick_hz: 20,
            sim_speed: 1.0,
            enable_game_net: true,
            enable_web: true,
            verify_ohol_ticket: true,
            ticket_verify_url: "https://onehouronelife.com/ticketServer/server.php".into(),
            map_png_path: PathBuf::from("maps/mysteraV1Test.png"),
            save_directory: PathBuf::from("SaveFiles"),
            generate_map_if_missing: true,
            force_regenerate_map: false,
            natural_object_density: 0.4,
            selfplay_enabled: true,
            selfplay_agents: 3,
            craft_graph_seed_cap: 50_000,
            twin_peers: Vec::new(),
            timed_movement: true,
            ai_craft_radius: 50,
            // Instant MOVE snap only. Timed path uses Haxe quadDist ≤ 5 (not this field).
            move_jump_max_chebyshev: 2,
            intent_drain_budget: 64,
            ops_sample_every_ticks: 100,
            ops_flush_secs: 300,
            ops_journal_path: PathBuf::from("SaveFiles/ops_metrics.journal"),
            npc_enabled: true,
            npc_min: 20,
            npc_max: 40,
            ai_think_period_ticks: 10,
            ai_reaction_time: gameplay_defaults::AI_REACTION_TIME,
            ai_reaction_time_serf: gameplay_defaults::AI_REACTION_TIME_SERF,
            ai_reaction_time_noble: gameplay_defaults::AI_REACTION_TIME_NOBLE,
            ai_reaction_time_factor_if_angry: gameplay_defaults::AI_REACTION_TIME_FACTOR_IF_ANGRY,
            ai_observe_radius: 16,
            broadcast_all_updates: true,
            shutdown_countdown_secs: 3,
            shutdown_apocalypse_secs: 3,
            eternal_winter: false,
            season_duration_years: 7.5,
            settings_hot_reload: true,
            settings_reload_every_ticks: DEFAULT_SETTINGS_RELOAD_EVERY_TICKS,
            lockpick_success_chance: DEFAULT_LOCKPICK_SUCCESS_CHANCE,
            lockpick_fail_chance: DEFAULT_LOCKPICK_FAIL_CHANCE,
            lockpick_exhaustion_cost: DEFAULT_LOCKPICK_EXHAUSTION_COST,
            lockpick_coin_cost: DEFAULT_LOCKPICK_COIN_COST,
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
            exhaustion_healing_for_male_factor: gameplay_defaults::EXHAUSTION_HEALING_FOR_MALE_FACTOR,
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
            // C-SS-MORE-BATCH3
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
            max_distance_close: gameplay_defaults::MAX_DISTANCE_CLOSE,
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
            door_ids: crate::DOOR_IDS.to_vec(),
            ai_ignored_floor_ids: crate::AI_IGNORED_FLOOR_IDS.to_vec(),
            secret: gameplay_defaults::SECRET.to_string(),
            allow_debug_commands: true,
            debug_say_player_position: false,
            ai_time_to_wait_if_crafting_failed: gameplay_defaults::AI_TIME_TO_WAIT_IF_CRAFTING_FAILED,
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

/// Subset of config knobs that are safe to apply while the server is running.
///
/// Boot-only fields (ports, content path, map generate, tick_hz, …)
/// are intentionally excluded — those need a process restart.
///
/// `twin_peers` is live-safe: re-syncs the in-memory peer registry without sockets.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSettings {
    pub sim_speed: f32,
    pub timed_movement: bool,
    pub move_jump_max_chebyshev: i32,
    pub broadcast_all_updates: bool,
    pub intent_drain_budget: u32,
    pub shutdown_countdown_secs: u32,
    pub shutdown_apocalypse_secs: u32,
    pub client_version_strict: bool,
    /// Haxe `lastVanillaID` (`< 1` = mapping off).
    pub last_vanilla_id: i32,
    /// Haxe `OpenLifeClientName`.
    pub open_life_client_name: String,
    pub eternal_winter: bool,
    /// Sim seconds per season (from `season_duration_years * HAXE_YEAR_SECS`).
    pub season_length_secs: f32,
    pub npc_enabled: bool,
    pub npc_min: u32,
    pub npc_max: u32,
    /// Haxe `MaxPlayers` — living humans+AIs cap (LOGIN policy).
    pub max_players: u32,
    pub ai_think_period_ticks: u32,
    /// Haxe `AiReactionTime` (Commoner seconds).
    pub ai_reaction_time: f32,
    /// Haxe `AiReactionTimeSerf`.
    pub ai_reaction_time_serf: f32,
    /// Haxe `AiReactionTimeNoble`.
    pub ai_reaction_time_noble: f32,
    /// Haxe `AiReactionTimeFactorIfAngry`.
    pub ai_reaction_time_factor_if_angry: f32,
    pub ai_observe_radius: i32,
    pub ai_craft_radius: i32,
    pub settings_hot_reload: bool,
    pub settings_reload_every_ticks: u64,
    /// Multi-server twin peer endpoints (re-sync into `TwinRegistry` on apply).
    // TWIN-MULTI-SERVER: was boot-only seed; now live-reloaded
    pub twin_peers: Vec<TwinPeerConfig>,
    /// Haxe `LockpickSucessChance` (%).
    pub lockpick_success_chance: f32,
    /// Haxe `LockpickFailChance` (%).
    pub lockpick_fail_chance: f32,
    /// Haxe `LockpickExhaustionCost`.
    pub lockpick_exhaustion_cost: f32,
    /// Haxe `LockpickCoinCost`.
    pub lockpick_coin_cost: f32,
    // --- SETTINGS-FIELD-MAP gameplay (Haxe ServerSettings live Reflect) ---
    pub food_use_per_second: f32,
    pub healing_per_second: f32,
    pub ageing_seconds_per_year: f32,
    pub initial_player_move_speed: f32,
    pub speed_factor: f32,
    pub yum_bonus: f32,
    pub chance_for_offspring: f32,
    pub chance_for_animal_dying: f32,
    /// Haxe `BiomeAnimalHitChance`.
    // MOSQUITO-MAPCHANCE
    pub biome_animal_hit_chance: f32,
    pub hungry_work_cost: f32,
    pub birth_prestige_factor: f32,
    pub ally_strength_too_low_for_pickup: f32,
    /// Haxe `TimeConfirmNewFollower` — delayed I FOLLOW seconds.
    // Haxe: ServerSettings.TimeConfirmNewFollower
    // FOLLOW-HIRE-DELAY
    pub time_confirm_new_follower: f32,
    /// Haxe `HireCost`.
    pub hire_cost: f32,
    /// Haxe `HireCostIncreasePerPerson`.
    pub hire_cost_increase_per_person: f32,
    /// Haxe `AutoFollowPlayer` — AI auto-acquire closest human when sticky empty.
    // Haxe: ServerSettings.AutoFollowPlayer
    // AI-FOLLOW-ACQUIRE
    pub auto_follow_player: bool,
    /// Haxe `PrestigeCostPerDamageForAlly`.
    // PRESTIGE-ALLY-COST
    pub prestige_cost_per_damage_for_ally: f32,
    /// Haxe `PrestigeCostPerDamageForChild`.
    // C-SS-MORE
    pub prestige_cost_per_damage_for_child: f32,
    /// Haxe `PrestigeCostPerDamageForElderly`.
    // C-SS-MORE
    pub prestige_cost_per_damage_for_elderly: f32,
    /// Haxe `PrestigeCostPerDamageForCloseRelatives`.
    // C-SS-MORE
    pub prestige_cost_per_damage_for_close_relatives: f32,
    /// Haxe `PrestigeCostPerDamageForWomenWithoutWeapon`.
    // C-SS-MORE
    pub prestige_cost_per_damage_for_women_without_weapon: f32,
    // --- C-SS-FULL-TABLE food factor long-tail ---
    /// Haxe `FoodFactor`.
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
    pub yum_food_restore: f32,
    /// Haxe `LovedFoodRestore`.
    pub loved_food_restore: f32,
    /// Haxe `YumNewCravingChance`.
    pub yum_new_craving_chance: f32,
    /// Haxe `FoodReductionPerEating`.
    pub food_reduction_per_eating: f32,
    /// Haxe `FoodReductionFaktorForEatingMeh`.
    pub food_reduction_faktor_for_eating_meh: f32,
    /// Haxe `HealthLostWhenEatingMeh`.
    pub health_lost_when_eating_meh: f32,
    /// Haxe `HealthLostWhenEatingSuperMeh`.
    pub health_lost_when_eating_super_meh: f32,
    // --- C-SS-TAIL-KNOBS ---
    /// Haxe `FoodReductionFaktorForEatingHighQuailitFood`.
    pub food_reduction_faktor_for_eating_high_quality: f32,
    /// Haxe `GrownUpFoodStoreMax`.
    pub grown_up_food_store_max: f32,
    // --- C-SS-AGE-FOOD ---
    /// Haxe `NewBornFoodStoreMax`.
    pub new_born_food_store_max: f32,
    /// Haxe `OldAgeFoodStoreMax`.
    pub old_age_food_store_max: f32,
    /// Haxe `MinBiomeSpeedFactor`.
    pub min_biome_speed_factor: f32,
    /// Haxe `HitpointsSpeedFactor`.
    pub hitpoints_speed_factor: f32,
    /// Haxe `CombatReputationRestorePerYear`.
    pub combat_reputation_restore_per_year: f32,
    // --- C-SS-MORE-KNOBS / settings_batch2 ---
    /// Haxe `ExhaustionHealingFactor`.
    pub exhaustion_healing_factor: f32,
    /// Haxe `WoundDamageFactor`.
    pub wound_damage_factor: f32,
    /// Haxe `WoundHealingFactor`.
    // C-SS-WOUND-HEAL
    pub wound_healing_factor: f32,
    /// Haxe `ExhaustionHealingForMaleFaktor` (Haxe typo Faktor) — male exhaustion recovery mult only.
    // C-SS-MALE-HEAL
    pub exhaustion_healing_for_male_factor: f32,
    /// Haxe `TemperatureHitsDamageFactor`.
    // C-SS-TEMP-HEAL
    pub temperature_hits_damage_factor: f32,
    /// Haxe `TemperatureExhaustionDamageFactor`.
    // C-SS-TEMP-HEAL
    pub temperature_exhaustion_damage_factor: f32,
    /// Haxe `MaxMovementQuadJumpDistanceBeforeForce`.
    pub max_movement_quad_jump_distance_before_force: f32,
    /// Haxe `FoodRestoreFactorWhileFeeding`.
    pub food_restore_factor_while_feeding: f32,
    /// Haxe `MaxHasEatenForNextGeneration`.
    pub max_has_eaten_for_next_generation: f32,
    /// Haxe `HasEatenReductionForNextGeneration`.
    pub has_eaten_reduction_for_next_generation: f32,
    /// Haxe `CoinsOnWoundingFactor`.
    // WALLET-COINS
    pub coins_on_wounding_factor: f32,
    // --- C-SS-MORE-BATCH3 / settings_batch3 ---
    /// Haxe `CombatExhaustionCostPerAttack`.
    // C-SS-MORE-BATCH3
    pub combat_exhaustion_cost_per_attack: f32,
    /// Haxe `MinAgeToEat`.
    // C-SS-MORE-BATCH3
    pub min_age_to_eat: f32,
    /// Haxe `MaxChildAgeForBreastFeeding`.
    // C-SS-MORE-BATCH3
    pub max_child_age_for_breast_feeding: f32,
    /// Haxe `AllyConsideredClose`.
    // C-SS-MORE-BATCH3
    pub ally_considered_close: f32,
    /// Haxe `MinMovementAgeInSec`.
    // C-SS-MORE-BATCH3
    pub min_movement_age_in_sec: f32,
    // --- C-SS-MORE-BATCH4 / settings_batch4 ---
    /// Haxe `CursedReceiveDamageFactor`.
    // C-SS-MORE-BATCH4
    pub cursed_receive_damage_factor: f32,
    /// Haxe `CursedMakeDamageFactor`.
    // C-SS-MORE-BATCH4
    pub cursed_make_damage_factor: f32,
    /// Haxe `PickupBabyMaxDistance`.
    // C-SS-MORE-BATCH4
    pub pickup_baby_max_distance: f32,
    /// Haxe `InheritCoinsFactor`.
    // C-SS-MORE-BATCH4
    pub inherit_coins_factor: f32,
    /// Haxe `MinAgeFertile`.
    // C-SS-MORE-BATCH4
    pub min_age_fertile: f32,
    /// Haxe `MaxAgeFertile`.
    // C-SS-MORE-BATCH4
    pub max_age_fertile: f32,
    // --- C-SS-MORE-BATCH5 / settings_batch5 ---
    /// Haxe `WeaponCoolDownFactor`.
    // C-SS-MORE-BATCH5
    pub weapon_cooldown_factor: f32,
    /// Haxe `WeaponCoolDownFactorIfWounding`.
    // C-SS-MORE-BATCH5
    pub weapon_cooldown_factor_if_wounding: f32,
    /// Haxe `CloseEnemyWithWeaponSpeedFactor`.
    // C-SS-MORE-BATCH5
    pub close_enemy_with_weapon_speed_factor: f32,
    /// Haxe `ExhaustionOnJump`.
    // C-SS-MORE-BATCH5
    pub exhaustion_on_jump: f32,
    /// Haxe `HungryWorkHeat`.
    // C-SS-MORE-BATCH5
    pub hungry_work_heat: f32,
    /// Haxe `AISpeedFactorSerf`.
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_serf: f32,
    /// Haxe `AISpeedFactorCommoner`.
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_commoner: f32,
    /// Haxe `AISpeedFactorNoble`.
    // C-SS-MORE-BATCH5
    pub ai_speed_factor_noble: f32,
    /// Haxe `StartingEveAge`.
    // SETTINGS-LONG-TAIL
    pub starting_eve_age: f32,
    /// Haxe `EveOrAdamBirthChance`.
    // SETTINGS-LONG-TAIL
    pub eve_or_adam_birth_chance: f32,
    /// Haxe `SpawnAiAsEve`.
    // SETTINGS-LONG-TAIL
    pub spawn_ai_as_eve: bool,
    /// Humans may spawn as children of AI mothers.
    pub allow_humans_born_to_ais: bool,
    /// Haxe `MaxPlayersBeforeStartingAsChild`.
    // SETTINGS-LONG-TAIL
    pub max_players_before_starting_as_child: i32,
    /// Haxe `ObjDecayChance`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_chance: f32,
    /// Haxe `FloorDecayChance`.
    // SETTINGS-LONG-TAIL
    pub floor_decay_chance: f32,
    /// Haxe `ObjRespawnChance`.
    // SETTINGS-LONG-TAIL
    pub obj_respawn_chance: f32,
    /// Haxe `GrowBackPlantsIncreaseIfLowPopulation`.
    // SETTINGS-LONG-TAIL
    pub grow_back_plants_increase_if_low_population: f32,
    /// Haxe `GrowBackOriginalPlantsFactor`.
    // SETTINGS-LONG-TAIL
    pub grow_back_original_plants_factor: f32,
    /// Haxe `GrowNewPlantsFromExistingFactor`.
    // SETTINGS-LONG-TAIL
    pub grow_new_plants_from_existing_factor: f32,
    /// Haxe `SpringWildFoodRegrowChance`.
    // SETTINGS-LONG-TAIL
    pub spring_wild_food_regrow_chance: f32,
    /// Haxe `WinterWildFoodDecayChance`.
    // SETTINGS-LONG-TAIL
    pub winter_wild_food_decay_chance: f32,
    /// Haxe `HotSeasonTemperatureFactor`.
    // SETTINGS-LONG-TAIL
    pub hot_season_temperature_factor: f32,
    /// Haxe `ColdSeasonTemperatureFactor`.
    // SETTINGS-LONG-TAIL
    pub cold_season_temperature_factor: f32,
    /// Haxe `CursedGraveTime`.
    // SETTINGS-LONG-TAIL
    pub cursed_grave_time: f32,
    /// Haxe `AnimalDecayFactor`.
    // SETTINGS-LONG-TAIL
    pub animal_decay_factor: f32,
    /// Haxe `ObjDecayFactorForPermanentObjs`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_permanent: f32,
    /// Haxe `ObjDecayFactorForFood`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_food: f32,
    /// Haxe `ObjDecayFactorForClothing`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_clothing: f32,
    /// Haxe `ObjDecayFactorForWalls`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_for_walls: f32,
    /// Haxe `ObjDecayFactorPerTechLevel`.
    // SETTINGS-LONG-TAIL
    pub obj_decay_factor_per_tech_level: f32,
    /// Haxe `DecayFactorInDeepWater`.
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_deep_water: f32,
    /// Haxe `DecayFactorInMountain`.
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_mountain: f32,
    /// Haxe `DecayFactorInWalkableWater`.
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_walkable_water: f32,
    /// Haxe `DecayFactorInJungle`.
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_jungle: f32,
    /// Haxe `DecayFactorInSwamp`.
    // SETTINGS-LONG-TAIL
    pub decay_factor_in_swamp: f32,
    /// Haxe `ScoreFactor`.
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
    /// Haxe `SendMoveEveryXTicks` (`-1` = disabled).
    // SETTINGS-LONG-TAIL
    pub send_move_every_x_ticks: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCoseForMovement` (typo Cose).
    // SETTINGS-LONG-TAIL
    pub max_distance_cose_for_movement: i32,
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSayAi`.
    // SETTINGS-LONG-TAIL
    pub max_distance_say_ai: f32,
    /// Haxe `MaxDistanceToAutoExileAttacker`.
    pub max_distance_auto_exile_attacker: i32,
    /// Haxe `SpeedWithBothShoes`.
    // SETTINGS-LONG-TAIL
    pub speed_with_both_shoes: f32,
    /// Haxe `AgingFactorWhileStarvingToDeath`.
    // SETTINGS-LONG-TAIL
    pub aging_factor_while_starving: f32,
    /// Haxe `GrownUpAge`.
    // SETTINGS-LONG-TAIL
    pub grown_up_age: f32,
    /// Haxe `FoodUseChildFaktor`.
    // SETTINGS-LONG-TAIL
    pub food_use_child_faktor: f32,
    /// Haxe `AIFoodUseFactorSerf`.
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_serf: f32,
    /// Haxe `AIFoodUseFactorCommoner`.
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_commoner: f32,
    /// Haxe `AIFoodUseFactorNoble`.
    // SETTINGS-LONG-TAIL
    pub ai_food_use_factor_noble: f32,
    /// Haxe `EveFoodUseFactor`.
    // SETTINGS-LONG-TAIL
    pub eve_food_use_factor: f32,
    /// Haxe `AgingFactorHumanBornToAi`.
    pub aging_factor_human_born_to_ai: f32,
    /// Haxe `AgingFactorAiBornToHuman`.
    pub aging_factor_ai_born_to_human: f32,
    /// Haxe `EveDamageFactor`.
    // SETTINGS-LONG-TAIL
    pub eve_damage_factor: f32,
    /// Haxe `TargetWoundedDamageFactor`.
    // SETTINGS-LONG-TAIL
    pub target_wounded_damage_factor: f32,
    /// Haxe `MaleDamageFactor`.
    // SETTINGS-LONG-TAIL
    pub male_damage_factor: f32,
    /// Haxe `AnimalDamageFactor`.
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor: f32,
    /// Haxe `AnimalDamageFactorInWinter`.
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor_in_winter: f32,
    /// Haxe `AnimalDamageFactorIfAttacked`.
    // SETTINGS-LONG-TAIL
    pub animal_damage_factor_if_attacked: f32,
    /// Haxe `WeaponDamageFactor`.
    // SETTINGS-LONG-TAIL
    pub weapon_damage_factor: f32,
    /// Haxe `GraveBlockingDistance`.
    // SETTINGS-LONG-TAIL
    pub grave_blocking_distance: f32,
    /// Haxe `MaxPlayersBeforeActivatingGraveCurse`.
    // SETTINGS-LONG-TAIL
    pub max_players_before_activating_grave_curse: i32,
    /// Haxe `MaxPlayersBeforeForbidTouchGrave`.
    // GRAVE-TOUCH-PLAYERS
    pub max_players_before_forbid_touch_grave: i32,
    /// Haxe `CombatAngryTimeBeforeAttack`.
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
    // SETTINGS-LONG-TAIL
    pub ai_time_to_wait_if_crafting_failed: f32,
    /// Haxe `AiMaxSearchRadius`.
    // SETTINGS-LONG-TAIL
    pub ai_max_search_radius: i32,
    /// Haxe `AiMaxSearchIncrement`.
    // SETTINGS-LONG-TAIL
    pub ai_max_search_increment: i32,
    /// Haxe `AiIgnoreTimeTransitionsLongerThen`.
    // SETTINGS-LONG-TAIL
    pub ai_ignore_time_transitions_longer_then: f32,
    /// Haxe `AiMemoryMaxEntries`.
    // SOUL-LIVE-CAPS
    pub ai_memory_max_entries: i32,
    /// Haxe `AiChatMemoryMaxEntries`.
    // SOUL-LIVE-CAPS
    pub ai_chat_memory_max_entries: i32,
    /// Haxe `AlternativeOutcomePercentIncreasePerHit`.
    // SETTINGS-LONG-TAIL
    pub alternative_outcome_percent_increase_per_hit: f32,
    /// Haxe `AlternativeOutcomeHitsDecreaseOnSucess`.
    // SETTINGS-LONG-TAIL
    pub alternative_outcome_hits_decrease_on_success: f32,
    /// Haxe `FortificationCosePerHit`.
    // TH-ALT-LIVE-KNOBS
    pub fortification_cost_per_hit: f32,
    /// Haxe `ReduceAgeNeededToPickupObjects`.
    // MIN-PICKUP-AGE
    pub reduce_age_needed_to_pickup_objects: f32,
    /// Haxe `ChanceThatAnimalsCanPassBlockingBiome`.
    // SETTINGS-LONG-TAIL
    pub chance_animals_pass_blocking_biome: f32,
    /// Haxe `chancePreferredBiome`.
    // SETTINGS-LONG-TAIL
    pub chance_preferred_biome: f32,
    /// Haxe `CloseGraveSpeedMali`.
    // SETTINGS-LONG-TAIL
    pub close_grave_speed_mali: f32,
    /// Haxe `TemperatureSpeedImpact`.
    // SETTINGS-LONG-TAIL
    pub temperature_speed_impact: f32,
    /// Haxe `MinSpeedReductionPerContainedObj`.
    // SETTINGS-LONG-TAIL
    pub min_speed_reduction_per_contained_obj: f32,
    /// Haxe `LovedFoodUseChance`.
    // SETTINGS-LONG-TAIL
    pub loved_food_use_chance: f32,
    /// Haxe `MaxAgeForAllowingClothAndPrickupFromOthers`.
    // SETTINGS-LONG-TAIL
    pub max_age_for_allowing_cloth_and_pickup_from_others: f32,
    /// Haxe `MaxAgeForAllowingDie`.
    // SETTINGS-LONG-TAIL
    pub max_age_for_allowing_die: f32,
    /// Haxe `PrestigeCostForDie`.
    // SETTINGS-LONG-TAIL
    pub prestige_cost_for_die: f32,
    /// Haxe `StartingFamilyName`.
    // SETTINGS-LONG-TAIL
    pub starting_family_name: String,
    /// Haxe `StartingName`.
    // SETTINGS-LONG-TAIL
    pub starting_name: String,
    /// Haxe `FoundFamilyNeededPrestige`.
    // SETTINGS-LONG-TAIL
    pub found_family_needed_prestige: f32,
    /// Haxe `FoundFamilyCost`.
    // SETTINGS-LONG-TAIL
    pub found_family_cost: f32,
    /// Haxe `FoundFamilyNeededFollowers`.
    // SETTINGS-LONG-TAIL
    pub found_family_needed_followers: i32,
    /// Haxe `FoundFamilyBreakAllianceChance`.
    // SETTINGS-LONG-TAIL
    pub found_family_break_alliance_chance: f32,
    /// Haxe `PickupExhaustionGain`.
    // SETTINGS-LONG-TAIL
    pub pickup_exhaustion_gain: f32,
    /// Haxe `PickupFeedingFoodRestore`.
    // SETTINGS-LONG-TAIL
    pub pickup_feeding_food_restore: f32,
    /// Haxe `DeathWithFoodStoreMax`.
    // SETTINGS-LONG-TAIL
    pub death_with_food_store_max: f32,
    /// Haxe `FoodStoreMaxReductionWhileStarvingToDeath`.
    // SETTINGS-LONG-TAIL
    pub food_store_max_reduction_while_starving: f32,
    /// Haxe `TemperatureReductionPerDrinking`.
    // SETTINGS-LONG-TAIL
    pub temperature_reduction_per_drinking: f32,
    /// Haxe `MaxStoredWater`.
    // SETTINGS-LONG-TAIL
    pub max_stored_water: f32,
    /// Haxe `MaxJumpsPerTenSec`.
    // SETTINGS-LONG-TAIL
    pub max_jumps_per_ten_sec: f32,
    /// Haxe `TemperatureImpactPerSec`.
    // SETTINGS-LONG-TAIL
    pub temperature_impact_per_sec: f32,
    /// Haxe `TemperatureImpactPerSecIfGood`.
    // SETTINGS-LONG-TAIL
    pub temperature_impact_per_sec_if_good: f32,
    /// Haxe `TemperatureInWaterFactor`.
    // SETTINGS-LONG-TAIL
    pub temperature_in_water_factor: f32,
    /// Haxe `TemperatureImpactBelow`.
    // SETTINGS-LONG-TAIL
    pub temperature_impact_below: f32,
    /// Haxe `TemperatureImpactColorFactor`.
    // SETTINGS-LONG-TAIL
    pub temperature_impact_color_factor: f32,
    /// Haxe `AllowEatingOrFeedingIfIll`.
    // SETTINGS-LONG-TAIL
    pub allow_eating_or_feeding_if_ill: bool,
    /// Haxe `ResistanceAgainstFeverForEatingMushrooms`.
    // SETTINGS-LONG-TAIL
    pub resistance_against_fever_for_eating_mushrooms: f32,
    /// Haxe `ExhaustionYellowFeverPerSec`.
    // SETTINGS-LONG-TAIL
    pub exhaustion_yellow_fever_per_sec: f32,
    /// Haxe `MinHealthFoodStoreMaxFactor`.
    // SETTINGS-LONG-TAIL
    pub min_health_food_store_max_factor: f32,
    /// Haxe `MaxHealthFoodStoreMaxFactor`.
    // SETTINGS-LONG-TAIL
    pub max_health_food_store_max_factor: f32,
    /// Haxe `MinHealthAgingFactor`.
    // SETTINGS-LONG-TAIL
    pub min_health_aging_factor: f32,
    /// Haxe `MaxHealthAgingFactor`.
    // SETTINGS-LONG-TAIL
    pub max_health_aging_factor: f32,
    /// Haxe `MinHealthPerYear`.
    // SETTINGS-LONG-TAIL
    pub min_health_per_year: f32,
    /// Haxe `MaxAge`.
    // SETTINGS-LONG-TAIL
    pub max_age: f32,
    /// Haxe `AnimalDeadlyDistanceFactor`.
    // SETTINGS-LONG-TAIL
    pub animal_deadly_distance_factor: f32,
    /// Haxe `ChanceForAnimalDyingFactorIfInLovedBiome`.
    // SETTINGS-LONG-TAIL
    pub chance_for_animal_dying_factor_if_in_loved_biome: f32,
    /// Haxe `OffspringFactorIfAnimalPopIsLow`.
    // SETTINGS-LONG-TAIL
    pub offspring_factor_if_animal_pop_is_low: f32,
    /// Haxe `MaxOffspringFactor`.
    // SETTINGS-LONG-TAIL
    pub max_offspring_factor: f32,
    /// Haxe `OffspringFactorLowAnimalPopulationBelow`.
    // SETTINGS-LONG-TAIL
    pub offspring_factor_low_animal_population_below: f32,
}

impl ServerConfig {
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        let cfg: ServerConfig = toml::from_str(&text)?;
        Ok(cfg)
    }

    /// Strict load — errors if the file is missing or invalid (hot-reload path).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path)?;
        let cfg: ServerConfig = toml::from_str(&text)?;
        Ok(cfg)
    }

    pub fn write_default(path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let text = toml::to_string_pretty(&Self::default()).expect("serialize default config");
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text)?;
        Ok(())
    }

    pub fn game_addr(&self) -> String {
        format!("{}:{}", self.bind, self.game_port)
    }

    pub fn web_addr(&self) -> String {
        format!("{}:{}", self.bind, self.web_port)
    }

    pub fn world_save_path(&self) -> PathBuf {
        self.save_directory.join("world_v1.olw")
    }

    /// Versioned binary lineage index (`OLN1` / `lineages_v1.bin`).
    pub fn lineage_save_path(&self) -> PathBuf {
        self.save_directory.join("lineages_v1.bin")
    }

    /// Versioned binary soft-account book (`OLA1` / `accounts_v1.bin`).
    pub fn accounts_save_path(&self) -> PathBuf {
        self.save_directory.join("accounts_v1.bin")
    }

    /// Score-entry prestige queue (`SES1` / `score_entries_v1.bin`).
    /// Haxe ScoreEntry had TODO save-to-disk; Rust SES1 is separate from OLA1.
    pub fn score_entries_save_path(&self) -> PathBuf {
        self.save_directory.join("score_entries_v1.bin")
    }

    /// Session war/posse (`WPS1` / `war_posse_v1.bin`).
    /// Haxe had no disk for WAR/POSSE; Rust WPS1 keeps session maps across restart.
    pub fn war_posse_save_path(&self) -> PathBuf {
        self.save_directory.join("war_posse_v1.bin")
    }

    /// Sticky living players (`PLB1` / `players_v1.bin`).
    /// Haxe `GlobalPlayerInstance.WritePlayers` / `ReadPlayers` (PlayersN.bin).
    pub fn players_save_path(&self) -> PathBuf {
        self.save_directory.join("players_v1.bin")
    }

    /// World eaten-food statistics text dump (`FoodStats.txt`).
    /// Haxe `WorldMap.writeFoodStatistics` → `FoodStats{N}.txt` (Rust: fixed latest name).
    pub fn food_stats_save_path(&self) -> PathBuf {
        self.save_directory.join("FoodStats.txt")
    }

    /// World object-census text dump (`ObjectCounts.txt`).
    /// Haxe `WorldMap.write` TraceCountObjectsToDisk → `ObjectCounts{N}.txt` (Rust: fixed latest).
    /// Pure format/write + OBJECTCOUNTS-LIVE autosave share in `ol-sim`.
    pub fn object_counts_save_path(&self) -> PathBuf {
        self.save_directory.join("ObjectCounts.txt")
    }

    /// Frozen generation object census (Haxe `OriginalObjects.bin` / originalObjectsCount).
    pub fn original_census_save_path(&self) -> PathBuf {
        self.save_directory.join("original_census_v1.bin")
    }

    /// Self-play agent count clamped to **1..=3**.
    pub fn selfplay_agent_count(&self) -> u8 {
        self.selfplay_agents.clamp(1, 3)
    }

    /// Craft-graph seed cap (at least 1 so seed never panics on zero).
    pub fn craft_graph_cap(&self) -> usize {
        self.craft_graph_seed_cap.max(1)
    }

    /// Sim time dilation, clamped to non-negative (`0` freezes vitals dt).
    pub fn sim_speed_factor(&self) -> f32 {
        if self.sim_speed.is_finite() && self.sim_speed >= 0.0 {
            self.sim_speed
        } else {
            1.0
        }
    }

    /// Intent drain budget per tick wake (at least 1).
    pub fn intent_drain(&self) -> usize {
        self.intent_drain_budget.max(1) as usize
    }

    /// NPC min/max when enabled (min ≥ 1 when enabled path uses it; config may be 0).
    pub fn npc_bounds(&self) -> (u32, u32) {
        let min = self.npc_min;
        let max = self.npc_max.max(min);
        (min, max)
    }

    /// Haxe `SeasonDuration` → real seconds per season (`years * 60`).
    ///
    /// Clamped to at least 1 second so season math never divides by zero.
    pub fn season_length_secs(&self) -> f32 {
        let years = if self.season_duration_years.is_finite() && self.season_duration_years > 0.0 {
            self.season_duration_years
        } else {
            7.5
        };
        (years * HAXE_YEAR_SECS).max(1.0)
    }

    /// Sanitize lockpick chance/cost fields (Haxe statics; non-finite → defaults).
    // Haxe: ServerSettings.Lockpick*
    pub fn lockpick_success_chance_sanitized(&self) -> f32 {
        sanitize_nonneg_or(self.lockpick_success_chance, DEFAULT_LOCKPICK_SUCCESS_CHANCE)
    }

    pub fn lockpick_fail_chance_sanitized(&self) -> f32 {
        sanitize_nonneg_or(self.lockpick_fail_chance, DEFAULT_LOCKPICK_FAIL_CHANCE)
    }

    pub fn lockpick_exhaustion_cost_sanitized(&self) -> f32 {
        sanitize_nonneg_or(self.lockpick_exhaustion_cost, DEFAULT_LOCKPICK_EXHAUSTION_COST)
    }

    pub fn lockpick_coin_cost_sanitized(&self) -> f32 {
        sanitize_nonneg_or(self.lockpick_coin_cost, DEFAULT_LOCKPICK_COIN_COST)
    }

    /// Extract runtime-safe knobs (Haxe static fields that change mid-session).
    pub fn live_settings(&self) -> LiveSettings {
        LiveSettings {
            sim_speed: self.sim_speed_factor(),
            timed_movement: self.timed_movement,
            move_jump_max_chebyshev: self.move_jump_max_chebyshev.max(0),
            broadcast_all_updates: self.broadcast_all_updates,
            intent_drain_budget: self.intent_drain_budget.max(1),
            shutdown_countdown_secs: self.shutdown_countdown_secs.max(1),
            shutdown_apocalypse_secs: self.shutdown_apocalypse_secs.max(1),
            client_version_strict: self.client_version_strict,
            last_vanilla_id: self.last_vanilla_id,
            open_life_client_name: {
                let n = self.open_life_client_name.trim();
                if n.is_empty() {
                    gameplay_defaults::OPEN_LIFE_CLIENT_NAME.to_string()
                } else {
                    n.to_string()
                }
            },
            eternal_winter: self.eternal_winter,
            season_length_secs: self.season_length_secs(),
            npc_enabled: self.npc_enabled,
            npc_min: self.npc_min,
            npc_max: self.npc_max.max(self.npc_min),
            max_players: self.max_players.max(1),
            ai_think_period_ticks: self.ai_think_period_ticks.max(1),
            ai_reaction_time: sanitize_positive_or(
                self.ai_reaction_time,
                gameplay_defaults::AI_REACTION_TIME,
            ),
            ai_reaction_time_serf: sanitize_positive_or(
                self.ai_reaction_time_serf,
                gameplay_defaults::AI_REACTION_TIME_SERF,
            ),
            ai_reaction_time_noble: sanitize_positive_or(
                self.ai_reaction_time_noble,
                gameplay_defaults::AI_REACTION_TIME_NOBLE,
            ),
            ai_reaction_time_factor_if_angry: sanitize_positive_or(
                self.ai_reaction_time_factor_if_angry,
                gameplay_defaults::AI_REACTION_TIME_FACTOR_IF_ANGRY,
            ),
            ai_observe_radius: self.ai_observe_radius.max(4),
            ai_craft_radius: self.ai_craft_radius.max(8),
            settings_hot_reload: self.settings_hot_reload,
            settings_reload_every_ticks: self
                .settings_reload_every_ticks
                .max(1)
                .max(1), // keep ≥1
            twin_peers: self.twin_peers.clone(),
            lockpick_success_chance: self.lockpick_success_chance_sanitized(),
            lockpick_fail_chance: self.lockpick_fail_chance_sanitized(),
            lockpick_exhaustion_cost: self.lockpick_exhaustion_cost_sanitized(),
            lockpick_coin_cost: self.lockpick_coin_cost_sanitized(),
            food_use_per_second: sanitize_positive_or(
                self.food_use_per_second,
                gameplay_defaults::FOOD_USE_PER_SECOND,
            ),
            healing_per_second: sanitize_nonneg_or(
                self.healing_per_second,
                gameplay_defaults::HEALING_PER_SECOND,
            ),
            ageing_seconds_per_year: sanitize_positive_or(
                self.ageing_seconds_per_year,
                gameplay_defaults::AGEING_SECONDS_PER_YEAR,
            ),
            initial_player_move_speed: sanitize_positive_or(
                self.initial_player_move_speed,
                gameplay_defaults::INITIAL_PLAYER_MOVE_SPEED,
            ),
            speed_factor: sanitize_nonneg_or(
                self.speed_factor,
                gameplay_defaults::SPEED_FACTOR,
            ),
            yum_bonus: sanitize_nonneg_or(self.yum_bonus, gameplay_defaults::YUM_BONUS),
            chance_for_offspring: sanitize_nonneg_or(
                self.chance_for_offspring,
                gameplay_defaults::CHANCE_FOR_OFFSPRING,
            ),
            chance_for_animal_dying: sanitize_nonneg_or(
                self.chance_for_animal_dying,
                gameplay_defaults::CHANCE_FOR_ANIMAL_DYING,
            ),
            biome_animal_hit_chance: sanitize_nonneg_or(
                self.biome_animal_hit_chance,
                gameplay_defaults::BIOME_ANIMAL_HIT_CHANCE,
            ),
            hungry_work_cost: sanitize_nonneg_or(
                self.hungry_work_cost,
                gameplay_defaults::HUNGRY_WORK_COST,
            ),
            birth_prestige_factor: sanitize_nonneg_or(
                self.birth_prestige_factor,
                gameplay_defaults::BIRTH_PRESTIGE_FACTOR,
            ),
            ally_strength_too_low_for_pickup: sanitize_nonneg_or(
                self.ally_strength_too_low_for_pickup,
                gameplay_defaults::ALLY_STRENGTH_TOO_LOW_FOR_PICKUP,
            ),
            time_confirm_new_follower: sanitize_positive_or(
                self.time_confirm_new_follower,
                gameplay_defaults::TIME_CONFIRM_NEW_FOLLOWER,
            ),
            hire_cost: sanitize_nonneg_or(self.hire_cost, gameplay_defaults::HIRE_COST),
            hire_cost_increase_per_person: sanitize_nonneg_or(
                self.hire_cost_increase_per_person,
                gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,
            ),
            auto_follow_player: self.auto_follow_player,
            prestige_cost_per_damage_for_ally: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_ally,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,
            ),
            prestige_cost_per_damage_for_child: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_child,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CHILD,
            ),
            prestige_cost_per_damage_for_elderly: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_elderly,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ELDERLY,
            ),
            prestige_cost_per_damage_for_close_relatives: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_close_relatives,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_CLOSE_RELATIVES,
            ),
            prestige_cost_per_damage_for_women_without_weapon: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_women_without_weapon,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_WOMEN_WITHOUT_WEAPON,
            ),
            food_factor: sanitize_nonneg_or(self.food_factor, gameplay_defaults::FOOD_FACTOR),
            food_factor_eaten_more_than_eight_percent: sanitize_nonneg_or(
                self.food_factor_eaten_more_than_eight_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT,
            ),
            food_factor_eaten_more_than_ten_percent: sanitize_nonneg_or(
                self.food_factor_eaten_more_than_ten_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT,
            ),
            food_factor_eaten_less_than_five_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_five_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT,
            ),
            food_factor_eaten_less_than_three_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_three_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT,
            ),
            food_factor_eaten_less_than_one_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_one_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT,
            ),
            yum_food_restore: sanitize_nonneg_or(
                self.yum_food_restore,
                gameplay_defaults::YUM_FOOD_RESTORE,
            ),
            loved_food_restore: sanitize_nonneg_or(
                self.loved_food_restore,
                gameplay_defaults::LOVED_FOOD_RESTORE,
            ),
            yum_new_craving_chance: sanitize_nonneg_or(
                self.yum_new_craving_chance,
                gameplay_defaults::YUM_NEW_CRAVING_CHANCE,
            ),
            food_reduction_per_eating: sanitize_nonneg_or(
                self.food_reduction_per_eating,
                gameplay_defaults::FOOD_REDUCTION_PER_EATING,
            ),
            food_reduction_faktor_for_eating_meh: sanitize_nonneg_or(
                self.food_reduction_faktor_for_eating_meh,
                gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_MEH,
            ),
            health_lost_when_eating_meh: sanitize_nonneg_or(
                self.health_lost_when_eating_meh,
                gameplay_defaults::HEALTH_LOST_WHEN_EATING_MEH,
            ),
            health_lost_when_eating_super_meh: sanitize_nonneg_or(
                self.health_lost_when_eating_super_meh,
                gameplay_defaults::HEALTH_LOST_WHEN_EATING_SUPER_MEH,
            ),
            // C-SS-TAIL-KNOBS
            food_reduction_faktor_for_eating_high_quality: sanitize_nonneg_or(
                self.food_reduction_faktor_for_eating_high_quality,
                gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_HIGH_QUALITY,
            ),
            grown_up_food_store_max: sanitize_positive_or(
                self.grown_up_food_store_max,
                gameplay_defaults::GROWN_UP_FOOD_STORE_MAX,
            ),
            // C-SS-AGE-FOOD
            new_born_food_store_max: sanitize_positive_or(
                self.new_born_food_store_max,
                gameplay_defaults::NEW_BORN_FOOD_STORE_MAX,
            ),
            old_age_food_store_max: sanitize_positive_or(
                self.old_age_food_store_max,
                gameplay_defaults::OLD_AGE_FOOD_STORE_MAX,
            ),
            min_biome_speed_factor: sanitize_nonneg_or(
                self.min_biome_speed_factor,
                gameplay_defaults::MIN_BIOME_SPEED_FACTOR,
            ),
            hitpoints_speed_factor: sanitize_nonneg_or(
                self.hitpoints_speed_factor,
                gameplay_defaults::HITPOINTS_SPEED_FACTOR,
            ),
            combat_reputation_restore_per_year: sanitize_nonneg_or(
                self.combat_reputation_restore_per_year,
                gameplay_defaults::COMBAT_REPUTATION_RESTORE_PER_YEAR,
            ),
            // C-SS-MORE-KNOBS
            exhaustion_healing_factor: sanitize_nonneg_or(
                self.exhaustion_healing_factor,
                gameplay_defaults::EXHAUSTION_HEALING_FACTOR,
            ),
            wound_damage_factor: sanitize_nonneg_or(
                self.wound_damage_factor,
                gameplay_defaults::WOUND_DAMAGE_FACTOR,
            ),
            wound_healing_factor: sanitize_nonneg_or(
                self.wound_healing_factor,
                gameplay_defaults::WOUND_HEALING_FACTOR,
            ),
            // C-SS-MALE-HEAL
            exhaustion_healing_for_male_factor: sanitize_nonneg_or(
                self.exhaustion_healing_for_male_factor,
                gameplay_defaults::EXHAUSTION_HEALING_FOR_MALE_FACTOR,
            ),
            // C-SS-TEMP-HEAL
            temperature_hits_damage_factor: sanitize_nonneg_or(
                self.temperature_hits_damage_factor,
                gameplay_defaults::TEMPERATURE_HITS_DAMAGE_FACTOR,
            ),
            temperature_exhaustion_damage_factor: sanitize_nonneg_or(
                self.temperature_exhaustion_damage_factor,
                gameplay_defaults::TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR,
            ),
            max_movement_quad_jump_distance_before_force: sanitize_positive_or(
                self.max_movement_quad_jump_distance_before_force,
                gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE,
            ),
            food_restore_factor_while_feeding: sanitize_nonneg_or(
                self.food_restore_factor_while_feeding,
                gameplay_defaults::FOOD_RESTORE_FACTOR_WHILE_FEEDING,
            ),
            max_has_eaten_for_next_generation: sanitize_nonneg_or(
                self.max_has_eaten_for_next_generation,
                gameplay_defaults::MAX_HAS_EATEN_FOR_NEXT_GENERATION,
            ),
            has_eaten_reduction_for_next_generation: sanitize_nonneg_or(
                self.has_eaten_reduction_for_next_generation,
                gameplay_defaults::HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION,
            ),
            // WALLET-COINS
            coins_on_wounding_factor: sanitize_nonneg_or(
                self.coins_on_wounding_factor,
                gameplay_defaults::COINS_ON_WOUNDING_FACTOR,
            ),
            // C-SS-MORE-BATCH3
            combat_exhaustion_cost_per_attack: sanitize_nonneg_or(
                self.combat_exhaustion_cost_per_attack,
                gameplay_defaults::COMBAT_EXHAUSTION_COST_PER_ATTACK,
            ),
            min_age_to_eat: sanitize_nonneg_or(
                self.min_age_to_eat,
                gameplay_defaults::MIN_AGE_TO_EAT,
            ),
            max_child_age_for_breast_feeding: sanitize_nonneg_or(
                self.max_child_age_for_breast_feeding,
                gameplay_defaults::MAX_CHILD_AGE_FOR_BREAST_FEEDING,
            ),
            ally_considered_close: sanitize_positive_or(
                self.ally_considered_close,
                gameplay_defaults::ALLY_CONSIDERED_CLOSE,
            ),
            min_movement_age_in_sec: sanitize_nonneg_or(
                self.min_movement_age_in_sec,
                gameplay_defaults::MIN_MOVEMENT_AGE_IN_SEC,
            ),
            // C-SS-MORE-BATCH4
            cursed_receive_damage_factor: sanitize_positive_or(
                self.cursed_receive_damage_factor,
                gameplay_defaults::CURSED_RECEIVE_DAMAGE_FACTOR,
            ),
            cursed_make_damage_factor: sanitize_positive_or(
                self.cursed_make_damage_factor,
                gameplay_defaults::CURSED_MAKE_DAMAGE_FACTOR,
            ),
            pickup_baby_max_distance: sanitize_positive_or(
                self.pickup_baby_max_distance,
                gameplay_defaults::PICKUP_BABY_MAX_DISTANCE,
            ),
            inherit_coins_factor: sanitize_nonneg_or(
                self.inherit_coins_factor,
                gameplay_defaults::INHERIT_COINS_FACTOR,
            ),
            min_age_fertile: sanitize_nonneg_or(
                self.min_age_fertile,
                gameplay_defaults::MIN_AGE_FERTILE,
            ),
            max_age_fertile: sanitize_positive_or(
                self.max_age_fertile,
                gameplay_defaults::MAX_AGE_FERTILE,
            ),
            // C-SS-MORE-BATCH5
            weapon_cooldown_factor: sanitize_positive_or(
                self.weapon_cooldown_factor,
                gameplay_defaults::WEAPON_COOLDOWN_FACTOR,
            ),
            weapon_cooldown_factor_if_wounding: sanitize_positive_or(
                self.weapon_cooldown_factor_if_wounding,
                gameplay_defaults::WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
            ),
            close_enemy_with_weapon_speed_factor: sanitize_positive_or(
                self.close_enemy_with_weapon_speed_factor,
                gameplay_defaults::CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,
            ),
            exhaustion_on_jump: sanitize_nonneg_or(
                self.exhaustion_on_jump,
                gameplay_defaults::EXHAUSTION_ON_JUMP,
            ),
            hungry_work_heat: sanitize_nonneg_or(
                self.hungry_work_heat,
                gameplay_defaults::HUNGRY_WORK_HEAT,
            ),
            ai_speed_factor_serf: sanitize_positive_or(
                self.ai_speed_factor_serf,
                gameplay_defaults::AI_SPEED_FACTOR_SERF,
            ),
            ai_speed_factor_commoner: sanitize_positive_or(
                self.ai_speed_factor_commoner,
                gameplay_defaults::AI_SPEED_FACTOR_COMMONER,
            ),
            ai_speed_factor_noble: sanitize_positive_or(
                self.ai_speed_factor_noble,
                gameplay_defaults::AI_SPEED_FACTOR_NOBLE,
            ),
            // SETTINGS-LONG-TAIL
            starting_eve_age: sanitize_positive_or(
                self.starting_eve_age,
                gameplay_defaults::STARTING_EVE_AGE,
            ),
            eve_or_adam_birth_chance: sanitize_nonneg_or(
                self.eve_or_adam_birth_chance,
                gameplay_defaults::EVE_OR_ADAM_BIRTH_CHANCE,
            ),
            spawn_ai_as_eve: self.spawn_ai_as_eve,
            allow_humans_born_to_ais: self.allow_humans_born_to_ais,
            max_players_before_starting_as_child: self.max_players_before_starting_as_child,
            obj_decay_chance: sanitize_nonneg_or(
                self.obj_decay_chance,
                gameplay_defaults::OBJ_DECAY_CHANCE,
            ),
            floor_decay_chance: sanitize_nonneg_or(
                self.floor_decay_chance,
                gameplay_defaults::FLOOR_DECAY_CHANCE,
            ),
            obj_respawn_chance: sanitize_nonneg_or(
                self.obj_respawn_chance,
                gameplay_defaults::OBJ_RESPAWN_CHANCE,
            ),
            grow_back_plants_increase_if_low_population: sanitize_positive_or(
                self.grow_back_plants_increase_if_low_population,
                gameplay_defaults::GROW_BACK_PLANTS_INCREASE_IF_LOW_POPULATION,
            ),
            grow_back_original_plants_factor: sanitize_nonneg_or(
                self.grow_back_original_plants_factor,
                gameplay_defaults::GROW_BACK_ORIGINAL_PLANTS_FACTOR,
            ),
            grow_new_plants_from_existing_factor: sanitize_nonneg_or(
                self.grow_new_plants_from_existing_factor,
                gameplay_defaults::GROW_NEW_PLANTS_FROM_EXISTING_FACTOR,
            ),
            spring_wild_food_regrow_chance: sanitize_nonneg_or(
                self.spring_wild_food_regrow_chance,
                gameplay_defaults::SPRING_WILD_FOOD_REGROW_CHANCE,
            ),
            winter_wild_food_decay_chance: sanitize_nonneg_or(
                self.winter_wild_food_decay_chance,
                gameplay_defaults::WINTER_WILD_FOOD_DECAY_CHANCE,
            ),
            hot_season_temperature_factor: sanitize_nonneg_or(
                self.hot_season_temperature_factor,
                gameplay_defaults::HOT_SEASON_TEMPERATURE_FACTOR,
            ),
            cold_season_temperature_factor: sanitize_nonneg_or(
                self.cold_season_temperature_factor,
                gameplay_defaults::COLD_SEASON_TEMPERATURE_FACTOR,
            ),
            cursed_grave_time: sanitize_nonneg_or(
                self.cursed_grave_time,
                gameplay_defaults::CURSED_GRAVE_TIME,
            ),
            animal_decay_factor: sanitize_nonneg_or(
                self.animal_decay_factor,
                gameplay_defaults::ANIMAL_DECAY_FACTOR,
            ),
            obj_decay_factor_for_permanent: sanitize_nonneg_or(
                self.obj_decay_factor_for_permanent,
                gameplay_defaults::OBJ_DECAY_FACTOR_FOR_PERMANENT_OBJS,
            ),
            obj_decay_factor_for_food: sanitize_positive_or(
                self.obj_decay_factor_for_food,
                gameplay_defaults::OBJ_DECAY_FACTOR_FOR_FOOD,
            ),
            obj_decay_factor_for_clothing: sanitize_positive_or(
                self.obj_decay_factor_for_clothing,
                gameplay_defaults::OBJ_DECAY_FACTOR_FOR_CLOTHING,
            ),
            obj_decay_factor_for_walls: sanitize_positive_or(
                self.obj_decay_factor_for_walls,
                gameplay_defaults::OBJ_DECAY_FACTOR_FOR_WALLS,
            ),
            obj_decay_factor_per_tech_level: sanitize_positive_or(
                self.obj_decay_factor_per_tech_level,
                gameplay_defaults::OBJ_DECAY_FACTOR_PER_TECH_LEVEL,
            ),
            decay_factor_in_deep_water: sanitize_positive_or(
                self.decay_factor_in_deep_water,
                gameplay_defaults::DECAY_FACTOR_IN_DEEP_WATER,
            ),
            decay_factor_in_mountain: sanitize_positive_or(
                self.decay_factor_in_mountain,
                gameplay_defaults::DECAY_FACTOR_IN_MOUNTAIN,
            ),
            decay_factor_in_walkable_water: sanitize_positive_or(
                self.decay_factor_in_walkable_water,
                gameplay_defaults::DECAY_FACTOR_IN_WALKABLE_WATER,
            ),
            decay_factor_in_jungle: sanitize_positive_or(
                self.decay_factor_in_jungle,
                gameplay_defaults::DECAY_FACTOR_IN_JUNGLE,
            ),
            decay_factor_in_swamp: sanitize_positive_or(
                self.decay_factor_in_swamp,
                gameplay_defaults::DECAY_FACTOR_IN_SWAMP,
            ),
            score_factor: sanitize_nonneg_or(
                self.score_factor,
                gameplay_defaults::SCORE_FACTOR,
            ),
            ancestor_prestige_factor: sanitize_nonneg_or(
                self.ancestor_prestige_factor,
                gameplay_defaults::ANCESTOR_PRESTIGE_FACTOR,
            ),
            display_score_factor: sanitize_nonneg_or(
                self.display_score_factor,
                gameplay_defaults::DISPLAY_SCORE_FACTOR,
            ),
            display_score_on: self.display_score_on,
            max_coins_per_chest: sanitize_i32_nonneg(
                self.max_coins_per_chest,
                gameplay_defaults::MAX_COINS_PER_CHEST,
            ),
            max_coins_per_pouch: sanitize_i32_nonneg(
                self.max_coins_per_pouch,
                gameplay_defaults::MAX_COINS_PER_POUCH,
            ),
            chance_for_female_child: sanitize_nonneg_or(
                self.chance_for_female_child,
                gameplay_defaults::CHANCE_FOR_FEMALE_CHILD,
            ),
            chance_for_other_child_color: sanitize_nonneg_or(
                self.chance_for_other_child_color,
                gameplay_defaults::CHANCE_FOR_OTHER_CHILD_COLOR,
            ),
            chance_for_other_child_color_if_close_to_wrong_special_biome: sanitize_nonneg_or(
                self.chance_for_other_child_color_if_close_to_wrong_special_biome,
                gameplay_defaults::CHANCE_FOR_OTHER_CHILD_COLOR_IF_CLOSE_TO_WRONG_SPECIAL_BIOME,
            ),
            little_kids_per_mother: sanitize_i32_nonneg(
                self.little_kids_per_mother,
                gameplay_defaults::LITTLE_KIDS_PER_MOTHER,
            ),
            new_child_exhaustion_for_mother: sanitize_nonneg_or(
                self.new_child_exhaustion_for_mother,
                gameplay_defaults::NEW_CHILD_EXHAUSTION_FOR_MOTHER,
            ),
            ai_mother_birth_mali_for_human_child: sanitize_nonneg_or(
                self.ai_mother_birth_mali_for_human_child,
                gameplay_defaults::AI_MOTHER_BIRTH_MALI_FOR_HUMAN_CHILD,
            ),
            human_mother_birth_mali_for_ai_child: sanitize_nonneg_or(
                self.human_mother_birth_mali_for_ai_child,
                gameplay_defaults::HUMAN_MOTHER_BIRTH_MALI_FOR_AI_CHILD,
            ),
            spawn_at_last_dead: self.spawn_at_last_dead,
            temperature_own_tile_rate: sanitize_nonneg_or(
                self.temperature_own_tile_rate,
                gameplay_defaults::TEMPERATURE_OWN_TILE_RATE,
            ),
            temperature_balance_rate: sanitize_nonneg_or(
                self.temperature_balance_rate,
                gameplay_defaults::TEMPERATURE_BALANCE_RATE,
            ),
            temperature_local_heat_factor: sanitize_nonneg_or(
                self.temperature_local_heat_factor,
                gameplay_defaults::TEMPERATURE_LOCAL_HEAT_FACTOR,
            ),
            average_season_temperature_impact: sanitize_nonneg_or(
                self.average_season_temperature_impact,
                gameplay_defaults::AVERAGE_SEASON_TEMPERATURE_IMPACT,
            ),
            ai_total_score_factor: sanitize_nonneg_or(
                self.ai_total_score_factor,
                gameplay_defaults::AI_TOTAL_SCORE_FACTOR,
            ),
            old_grave_decay_mali: sanitize_nonneg_or(
                self.old_grave_decay_mali,
                gameplay_defaults::OLD_GRAVE_DECAY_MALI,
            ),
            cursed_grave_mali: sanitize_nonneg_or(
                self.cursed_grave_mali,
                gameplay_defaults::CURSED_GRAVE_MALI,
            ),
            max_distance_close: sanitize_i32_nonneg(
                self.max_distance_close,
                gameplay_defaults::MAX_DISTANCE_CLOSE,
            ),
            max_distance_map_changes: sanitize_i32_nonneg(
                self.max_distance_map_changes,
                gameplay_defaults::MAX_DISTANCE_MAP_CHANGES,
            ),
            max_distance_say: sanitize_i32_nonneg(
                self.max_distance_say,
                gameplay_defaults::MAX_DISTANCE_SAY,
            ),
            // Negative disables (Haxe product -1); 0 also off (`> 0` gate).
            send_move_every_x_ticks: self.send_move_every_x_ticks,
            max_distance_cose_for_movement: sanitize_i32_nonneg(
                self.max_distance_cose_for_movement,
                gameplay_defaults::MAX_DISTANCE_COSE_FOR_MOVEMENT,
            ),
            max_distance_say_ai: sanitize_positive_or(
                self.max_distance_say_ai,
                gameplay_defaults::MAX_DISTANCE_SAY_AI,
            ),
            max_distance_auto_exile_attacker: sanitize_i32_nonneg(
                self.max_distance_auto_exile_attacker,
                gameplay_defaults::MAX_DISTANCE_AUTO_EXILE_ATTACKER,
            ),
            speed_with_both_shoes: sanitize_positive_or(
                self.speed_with_both_shoes,
                gameplay_defaults::SPEED_WITH_BOTH_SHOES,
            ),
            aging_factor_while_starving: sanitize_positive_or(
                self.aging_factor_while_starving,
                gameplay_defaults::AGING_FACTOR_WHILE_STARVING,
            ),
            grown_up_age: sanitize_positive_or(
                self.grown_up_age,
                gameplay_defaults::GROWN_UP_AGE,
            ),
            food_use_child_faktor: sanitize_nonneg_or(
                self.food_use_child_faktor,
                gameplay_defaults::FOOD_USE_CHILD_FAKTOR,
            ),
            ai_food_use_factor_serf: sanitize_nonneg_or(
                self.ai_food_use_factor_serf,
                gameplay_defaults::AI_FOOD_USE_FACTOR_SERF,
            ),
            ai_food_use_factor_commoner: sanitize_nonneg_or(
                self.ai_food_use_factor_commoner,
                gameplay_defaults::AI_FOOD_USE_FACTOR_COMMONER,
            ),
            ai_food_use_factor_noble: sanitize_nonneg_or(
                self.ai_food_use_factor_noble,
                gameplay_defaults::AI_FOOD_USE_FACTOR_NOBLE,
            ),
            eve_food_use_factor: sanitize_nonneg_or(
                self.eve_food_use_factor,
                gameplay_defaults::EVE_FOOD_USE_FACTOR,
            ),
            aging_factor_human_born_to_ai: sanitize_positive_or(
                self.aging_factor_human_born_to_ai,
                gameplay_defaults::AGING_FACTOR_HUMAN_BORN_TO_AI,
            ),
            aging_factor_ai_born_to_human: sanitize_positive_or(
                self.aging_factor_ai_born_to_human,
                gameplay_defaults::AGING_FACTOR_AI_BORN_TO_HUMAN,
            ),
            eve_damage_factor: sanitize_nonneg_or(
                self.eve_damage_factor,
                gameplay_defaults::EVE_DAMAGE_FACTOR,
            ),
            target_wounded_damage_factor: sanitize_nonneg_or(
                self.target_wounded_damage_factor,
                gameplay_defaults::TARGET_WOUNDED_DAMAGE_FACTOR,
            ),
            male_damage_factor: sanitize_nonneg_or(
                self.male_damage_factor,
                gameplay_defaults::MALE_DAMAGE_FACTOR,
            ),
            animal_damage_factor: sanitize_nonneg_or(
                self.animal_damage_factor,
                gameplay_defaults::ANIMAL_DAMAGE_FACTOR,
            ),
            animal_damage_factor_in_winter: sanitize_nonneg_or(
                self.animal_damage_factor_in_winter,
                gameplay_defaults::ANIMAL_DAMAGE_FACTOR_IN_WINTER,
            ),
            animal_damage_factor_if_attacked: sanitize_nonneg_or(
                self.animal_damage_factor_if_attacked,
                gameplay_defaults::ANIMAL_DAMAGE_FACTOR_IF_ATTACKED,
            ),
            weapon_damage_factor: sanitize_nonneg_or(
                self.weapon_damage_factor,
                gameplay_defaults::WEAPON_DAMAGE_FACTOR,
            ),
            grave_blocking_distance: sanitize_nonneg_or(
                self.grave_blocking_distance,
                gameplay_defaults::GRAVE_BLOCKING_DISTANCE,
            ),
            max_players_before_activating_grave_curse: sanitize_i32_nonneg(
                self.max_players_before_activating_grave_curse,
                gameplay_defaults::MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE,
            ),
            max_players_before_forbid_touch_grave: sanitize_i32_nonneg(
                self.max_players_before_forbid_touch_grave,
                gameplay_defaults::MAX_PLAYERS_BEFORE_FORBID_TOUCH_GRAVE,
            ),
            combat_angry_time_before_attack: sanitize_nonneg_or(
                self.combat_angry_time_before_attack,
                gameplay_defaults::COMBAT_ANGRY_TIME_BEFORE_ATTACK,
            ),
            combat_angry_time_minimum: sanitize_finite_or(
                self.combat_angry_time_minimum,
                gameplay_defaults::COMBAT_ANGRY_TIME_MINIMUM,
            ),
            chance_for_domestic_animal_dying_factor: sanitize_nonneg_or(
                self.chance_for_domestic_animal_dying_factor,
                gameplay_defaults::CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR,
            ),
            door_ids: if self.door_ids.is_empty() {
                crate::DOOR_IDS.to_vec()
            } else {
                self.door_ids.clone()
            },
            ai_ignored_floor_ids: if self.ai_ignored_floor_ids.is_empty() {
                crate::AI_IGNORED_FLOOR_IDS.to_vec()
            } else {
                self.ai_ignored_floor_ids.clone()
            },
            secret: if self.secret.trim().is_empty() {
                gameplay_defaults::SECRET.to_string()
            } else {
                self.secret.clone()
            },
            allow_debug_commands: self.allow_debug_commands,
            debug_say_player_position: self.debug_say_player_position,
            ai_time_to_wait_if_crafting_failed: sanitize_nonneg_or(
                self.ai_time_to_wait_if_crafting_failed,
                gameplay_defaults::AI_TIME_TO_WAIT_IF_CRAFTING_FAILED,
            ),
            ai_max_search_radius: sanitize_i32_positive(
                self.ai_max_search_radius,
                gameplay_defaults::AI_MAX_SEARCH_RADIUS,
            ),
            ai_max_search_increment: sanitize_i32_positive(
                self.ai_max_search_increment,
                gameplay_defaults::AI_MAX_SEARCH_INCREMENT,
            ),
            ai_ignore_time_transitions_longer_then: sanitize_nonneg_or(
                self.ai_ignore_time_transitions_longer_then,
                gameplay_defaults::AI_IGNORE_TIME_TRANSITIONS_LONGER_THEN,
            ),
            ai_memory_max_entries: sanitize_i32_positive(
                self.ai_memory_max_entries,
                gameplay_defaults::AI_MEMORY_MAX_ENTRIES,
            ),
            ai_chat_memory_max_entries: sanitize_i32_positive(
                self.ai_chat_memory_max_entries,
                gameplay_defaults::AI_CHAT_MEMORY_MAX_ENTRIES,
            ),
            alternative_outcome_percent_increase_per_hit: sanitize_positive_or(
                self.alternative_outcome_percent_increase_per_hit,
                gameplay_defaults::ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT,
            ),
            alternative_outcome_hits_decrease_on_success: sanitize_nonneg_or(
                self.alternative_outcome_hits_decrease_on_success,
                gameplay_defaults::ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS,
            ),
            fortification_cost_per_hit: sanitize_nonneg_or(
                self.fortification_cost_per_hit,
                gameplay_defaults::FORTIFICATION_COST_PER_HIT,
            ),
            reduce_age_needed_to_pickup_objects: sanitize_nonneg_or(
                self.reduce_age_needed_to_pickup_objects,
                gameplay_defaults::REDUCE_AGE_NEEDED_TO_PICKUP_OBJECTS,
            ),
            chance_animals_pass_blocking_biome: sanitize_nonneg_or(
                self.chance_animals_pass_blocking_biome,
                gameplay_defaults::CHANCE_ANIMALS_PASS_BLOCKING_BIOME,
            ),
            chance_preferred_biome: sanitize_nonneg_or(
                self.chance_preferred_biome,
                gameplay_defaults::CHANCE_PREFERRED_BIOME,
            ),
            close_grave_speed_mali: sanitize_nonneg_or(
                self.close_grave_speed_mali,
                gameplay_defaults::CLOSE_GRAVE_SPEED_MALI,
            ),
            temperature_speed_impact: sanitize_nonneg_or(
                self.temperature_speed_impact,
                gameplay_defaults::TEMPERATURE_SPEED_IMPACT,
            ),
            min_speed_reduction_per_contained_obj: sanitize_nonneg_or(
                self.min_speed_reduction_per_contained_obj,
                gameplay_defaults::MIN_SPEED_REDUCTION_PER_CONTAINED_OBJ,
            ),
            loved_food_use_chance: sanitize_nonneg_or(
                self.loved_food_use_chance,
                gameplay_defaults::LOVED_FOOD_USE_CHANCE,
            ),
            max_age_for_allowing_cloth_and_pickup_from_others: sanitize_nonneg_or(
                self.max_age_for_allowing_cloth_and_pickup_from_others,
                gameplay_defaults::MAX_AGE_FOR_ALLOWING_CLOTH_AND_PICKUP_FROM_OTHERS,
            ),
            max_age_for_allowing_die: sanitize_nonneg_or(
                self.max_age_for_allowing_die,
                gameplay_defaults::MAX_AGE_FOR_ALLOWING_DIE,
            ),
            prestige_cost_for_die: sanitize_nonneg_or(
                self.prestige_cost_for_die,
                gameplay_defaults::PRESTIGE_COST_FOR_DIE,
            ),
            starting_family_name: sanitize_nonempty_trim(
                &self.starting_family_name,
                gameplay_defaults::STARTING_FAMILY_NAME,
            ),
            starting_name: sanitize_nonempty_trim(
                &self.starting_name,
                gameplay_defaults::STARTING_NAME,
            ),
            found_family_needed_prestige: sanitize_nonneg_or(
                self.found_family_needed_prestige,
                gameplay_defaults::FOUND_FAMILY_NEEDED_PRESTIGE,
            ),
            found_family_cost: sanitize_nonneg_or(
                self.found_family_cost,
                gameplay_defaults::FOUND_FAMILY_COST,
            ),
            found_family_needed_followers: sanitize_i32_nonneg(
                self.found_family_needed_followers,
                gameplay_defaults::FOUND_FAMILY_NEEDED_FOLLOWERS,
            ),
            found_family_break_alliance_chance: sanitize_nonneg_or(
                self.found_family_break_alliance_chance,
                gameplay_defaults::FOUND_FAMILY_BREAK_ALLIANCE_CHANCE,
            ),
            pickup_exhaustion_gain: sanitize_nonneg_or(
                self.pickup_exhaustion_gain,
                gameplay_defaults::PICKUP_EXHAUSTION_GAIN,
            ),
            pickup_feeding_food_restore: sanitize_nonneg_or(
                self.pickup_feeding_food_restore,
                gameplay_defaults::PICKUP_FEEDING_FOOD_RESTORE,
            ),
            death_with_food_store_max: sanitize_finite_or(
                self.death_with_food_store_max,
                gameplay_defaults::DEATH_WITH_FOOD_STORE_MAX,
            ),
            food_store_max_reduction_while_starving: sanitize_nonneg_or(
                self.food_store_max_reduction_while_starving,
                gameplay_defaults::FOOD_STORE_MAX_REDUCTION_WHILE_STARVING,
            ),
            temperature_reduction_per_drinking: sanitize_positive_or(
                self.temperature_reduction_per_drinking,
                gameplay_defaults::TEMPERATURE_REDUCTION_PER_DRINKING,
            ),
            max_stored_water: sanitize_positive_or(
                self.max_stored_water,
                gameplay_defaults::MAX_STORED_WATER,
            ),
            max_jumps_per_ten_sec: sanitize_positive_or(
                self.max_jumps_per_ten_sec,
                gameplay_defaults::MAX_JUMPS_PER_TEN_SEC,
            ),
            temperature_impact_per_sec: sanitize_positive_or(
                self.temperature_impact_per_sec,
                gameplay_defaults::TEMPERATURE_IMPACT_PER_SEC,
            ),
            temperature_impact_per_sec_if_good: sanitize_positive_or(
                self.temperature_impact_per_sec_if_good,
                gameplay_defaults::TEMPERATURE_IMPACT_PER_SEC_IF_GOOD,
            ),
            temperature_in_water_factor: sanitize_positive_or(
                self.temperature_in_water_factor,
                gameplay_defaults::TEMPERATURE_IN_WATER_FACTOR,
            ),
            temperature_impact_below: sanitize_nonneg_or(
                self.temperature_impact_below,
                gameplay_defaults::TEMPERATURE_IMPACT_BELOW,
            ),
            temperature_impact_color_factor: sanitize_nonneg_or(
                self.temperature_impact_color_factor,
                gameplay_defaults::TEMPERATURE_IMPACT_COLOR_FACTOR,
            ),
            allow_eating_or_feeding_if_ill: self.allow_eating_or_feeding_if_ill,
            resistance_against_fever_for_eating_mushrooms: sanitize_nonneg_or(
                self.resistance_against_fever_for_eating_mushrooms,
                gameplay_defaults::RESISTANCE_AGAINST_FEVER_FOR_EATING_MUSHROOMS,
            ),
            exhaustion_yellow_fever_per_sec: sanitize_nonneg_or(
                self.exhaustion_yellow_fever_per_sec,
                gameplay_defaults::EXHAUSTION_YELLOW_FEVER_PER_SEC,
            ),
            min_health_food_store_max_factor: sanitize_positive_or(
                self.min_health_food_store_max_factor,
                gameplay_defaults::MIN_HEALTH_FOOD_STORE_MAX_FACTOR,
            ),
            max_health_food_store_max_factor: sanitize_positive_or(
                self.max_health_food_store_max_factor,
                gameplay_defaults::MAX_HEALTH_FOOD_STORE_MAX_FACTOR,
            ),
            min_health_aging_factor: sanitize_positive_or(
                self.min_health_aging_factor,
                gameplay_defaults::MIN_HEALTH_AGING_FACTOR,
            ),
            max_health_aging_factor: sanitize_positive_or(
                self.max_health_aging_factor,
                gameplay_defaults::MAX_HEALTH_AGING_FACTOR,
            ),
            min_health_per_year: sanitize_positive_or(
                self.min_health_per_year,
                gameplay_defaults::MIN_HEALTH_PER_YEAR,
            ),
            max_age: sanitize_positive_or(self.max_age, gameplay_defaults::MAX_AGE),
            animal_deadly_distance_factor: sanitize_nonneg_or(
                self.animal_deadly_distance_factor,
                gameplay_defaults::ANIMAL_DEADLY_DISTANCE_FACTOR,
            ),
            chance_for_animal_dying_factor_if_in_loved_biome: sanitize_nonneg_or(
                self.chance_for_animal_dying_factor_if_in_loved_biome,
                gameplay_defaults::CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME,
            ),
            offspring_factor_if_animal_pop_is_low: sanitize_positive_or(
                self.offspring_factor_if_animal_pop_is_low,
                gameplay_defaults::OFFSPRING_FACTOR_IF_ANIMAL_POP_IS_LOW,
            ),
            max_offspring_factor: sanitize_positive_or(
                self.max_offspring_factor,
                gameplay_defaults::MAX_OFFSPRING_FACTOR,
            ),
            offspring_factor_low_animal_population_below: sanitize_positive_or(
                self.offspring_factor_low_animal_population_below,
                gameplay_defaults::OFFSPRING_FACTOR_LOW_ANIMAL_POPULATION_BELOW,
            ),
        }
    }

    /// Human-readable list of live fields that differ between two configs.
    pub fn live_diff_keys(old: &LiveSettings, new: &LiveSettings) -> Vec<&'static str> {
        let mut keys = Vec::new();
        let mut push = |name: &'static str, changed: bool| {
            if changed {
                keys.push(name);
            }
        };
        push("sim_speed", (old.sim_speed - new.sim_speed).abs() > f32::EPSILON);
        push("timed_movement", old.timed_movement != new.timed_movement);
        push(
            "move_jump_max_chebyshev",
            old.move_jump_max_chebyshev != new.move_jump_max_chebyshev,
        );
        push(
            "broadcast_all_updates",
            old.broadcast_all_updates != new.broadcast_all_updates,
        );
        push(
            "intent_drain_budget",
            old.intent_drain_budget != new.intent_drain_budget,
        );
        push(
            "shutdown_countdown_secs",
            old.shutdown_countdown_secs != new.shutdown_countdown_secs,
        );
        push(
            "shutdown_apocalypse_secs",
            old.shutdown_apocalypse_secs != new.shutdown_apocalypse_secs,
        );
        push(
            "client_version_strict",
            old.client_version_strict != new.client_version_strict,
        );
        push("eternal_winter", old.eternal_winter != new.eternal_winter);
        push(
            "season_length_secs",
            (old.season_length_secs - new.season_length_secs).abs() > 0.01,
        );
        push("npc_enabled", old.npc_enabled != new.npc_enabled);
        push("npc_min", old.npc_min != new.npc_min);
        push("npc_max", old.npc_max != new.npc_max);
        push("max_players", old.max_players != new.max_players);
        push(
            "last_vanilla_id",
            old.last_vanilla_id != new.last_vanilla_id,
        );
        push(
            "open_life_client_name",
            old.open_life_client_name != new.open_life_client_name,
        );
        push(
            "ai_think_period_ticks",
            old.ai_think_period_ticks != new.ai_think_period_ticks,
        );
        push(
            "ai_reaction_time",
            (old.ai_reaction_time - new.ai_reaction_time).abs() > f32::EPSILON,
        );
        push(
            "ai_reaction_time_serf",
            (old.ai_reaction_time_serf - new.ai_reaction_time_serf).abs() > f32::EPSILON,
        );
        push(
            "ai_reaction_time_noble",
            (old.ai_reaction_time_noble - new.ai_reaction_time_noble).abs() > f32::EPSILON,
        );
        push(
            "ai_reaction_time_factor_if_angry",
            (old.ai_reaction_time_factor_if_angry - new.ai_reaction_time_factor_if_angry).abs()
                > f32::EPSILON,
        );
        push(
            "ai_observe_radius",
            old.ai_observe_radius != new.ai_observe_radius,
        );
        push("ai_craft_radius", old.ai_craft_radius != new.ai_craft_radius);
        push(
            "settings_hot_reload",
            old.settings_hot_reload != new.settings_hot_reload,
        );
        push(
            "settings_reload_every_ticks",
            old.settings_reload_every_ticks != new.settings_reload_every_ticks,
        );
        push("twin_peers", old.twin_peers != new.twin_peers);
        push(
            "lockpick_success_chance",
            (old.lockpick_success_chance - new.lockpick_success_chance).abs() > f32::EPSILON,
        );
        push(
            "lockpick_fail_chance",
            (old.lockpick_fail_chance - new.lockpick_fail_chance).abs() > f32::EPSILON,
        );
        push(
            "lockpick_exhaustion_cost",
            (old.lockpick_exhaustion_cost - new.lockpick_exhaustion_cost).abs() > f32::EPSILON,
        );
        push(
            "lockpick_coin_cost",
            (old.lockpick_coin_cost - new.lockpick_coin_cost).abs() > f32::EPSILON,
        );
        // SETTINGS-FIELD-MAP gameplay batch
        push(
            "food_use_per_second",
            (old.food_use_per_second - new.food_use_per_second).abs() > f32::EPSILON,
        );
        push(
            "healing_per_second",
            (old.healing_per_second - new.healing_per_second).abs() > f32::EPSILON,
        );
        push(
            "ageing_seconds_per_year",
            (old.ageing_seconds_per_year - new.ageing_seconds_per_year).abs() > f32::EPSILON,
        );
        push(
            "initial_player_move_speed",
            (old.initial_player_move_speed - new.initial_player_move_speed).abs() > f32::EPSILON,
        );
        push(
            "speed_factor",
            (old.speed_factor - new.speed_factor).abs() > f32::EPSILON,
        );
        push(
            "yum_bonus",
            (old.yum_bonus - new.yum_bonus).abs() > f32::EPSILON,
        );
        push(
            "chance_for_offspring",
            (old.chance_for_offspring - new.chance_for_offspring).abs() > 1e-12,
        );
        push(
            "chance_for_animal_dying",
            (old.chance_for_animal_dying - new.chance_for_animal_dying).abs() > 1e-12,
        );
        push(
            "biome_animal_hit_chance",
            (old.biome_animal_hit_chance - new.biome_animal_hit_chance).abs() > 1e-12,
        );
        push(
            "hungry_work_cost",
            (old.hungry_work_cost - new.hungry_work_cost).abs() > f32::EPSILON,
        );
        push(
            "birth_prestige_factor",
            (old.birth_prestige_factor - new.birth_prestige_factor).abs() > f32::EPSILON,
        );
        push(
            "ally_strength_too_low_for_pickup",
            (old.ally_strength_too_low_for_pickup - new.ally_strength_too_low_for_pickup).abs()
                > f32::EPSILON,
        );
        push(
            "time_confirm_new_follower",
            (old.time_confirm_new_follower - new.time_confirm_new_follower).abs() > f32::EPSILON,
        );
        push(
            "hire_cost",
            (old.hire_cost - new.hire_cost).abs() > f32::EPSILON,
        );
        push(
            "hire_cost_increase_per_person",
            (old.hire_cost_increase_per_person - new.hire_cost_increase_per_person).abs()
                > f32::EPSILON,
        );
        push(
            "auto_follow_player",
            old.auto_follow_player != new.auto_follow_player,
        );
        push(
            "prestige_cost_per_damage_for_ally",
            (old.prestige_cost_per_damage_for_ally - new.prestige_cost_per_damage_for_ally).abs()
                > f32::EPSILON,
        );
        // C-SS-MORE PrestigeCost* non-ally
        push(
            "prestige_cost_per_damage_for_child",
            (old.prestige_cost_per_damage_for_child - new.prestige_cost_per_damage_for_child).abs()
                > f32::EPSILON,
        );
        push(
            "prestige_cost_per_damage_for_elderly",
            (old.prestige_cost_per_damage_for_elderly - new.prestige_cost_per_damage_for_elderly)
                .abs()
                > f32::EPSILON,
        );
        push(
            "prestige_cost_per_damage_for_close_relatives",
            (old.prestige_cost_per_damage_for_close_relatives
                - new.prestige_cost_per_damage_for_close_relatives)
                .abs()
                > f32::EPSILON,
        );
        push(
            "prestige_cost_per_damage_for_women_without_weapon",
            (old.prestige_cost_per_damage_for_women_without_weapon
                - new.prestige_cost_per_damage_for_women_without_weapon)
                .abs()
                > f32::EPSILON,
        );
        // C-SS-FULL-TABLE food factor long-tail
        push(
            "food_factor",
            (old.food_factor - new.food_factor).abs() > f32::EPSILON,
        );
        push(
            "food_factor_eaten_more_than_eight_percent",
            (old.food_factor_eaten_more_than_eight_percent
                - new.food_factor_eaten_more_than_eight_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_more_than_ten_percent",
            (old.food_factor_eaten_more_than_ten_percent
                - new.food_factor_eaten_more_than_ten_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_five_percent",
            (old.food_factor_eaten_less_than_five_percent
                - new.food_factor_eaten_less_than_five_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_three_percent",
            (old.food_factor_eaten_less_than_three_percent
                - new.food_factor_eaten_less_than_three_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_one_percent",
            (old.food_factor_eaten_less_than_one_percent
                - new.food_factor_eaten_less_than_one_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "yum_food_restore",
            (old.yum_food_restore - new.yum_food_restore).abs() > f32::EPSILON,
        );
        push(
            "loved_food_restore",
            (old.loved_food_restore - new.loved_food_restore).abs() > f32::EPSILON,
        );
        push(
            "yum_new_craving_chance",
            (old.yum_new_craving_chance - new.yum_new_craving_chance).abs() > f32::EPSILON,
        );
        push(
            "food_reduction_per_eating",
            (old.food_reduction_per_eating - new.food_reduction_per_eating).abs() > f32::EPSILON,
        );
        push(
            "food_reduction_faktor_for_eating_meh",
            (old.food_reduction_faktor_for_eating_meh
                - new.food_reduction_faktor_for_eating_meh)
                .abs()
                > f32::EPSILON,
        );
        push(
            "health_lost_when_eating_meh",
            (old.health_lost_when_eating_meh - new.health_lost_when_eating_meh).abs()
                > f32::EPSILON,
        );
        push(
            "health_lost_when_eating_super_meh",
            (old.health_lost_when_eating_super_meh - new.health_lost_when_eating_super_meh)
                .abs()
                > f32::EPSILON,
        );
        // C-SS-TAIL-KNOBS
        push(
            "food_reduction_faktor_for_eating_high_quality",
            (old.food_reduction_faktor_for_eating_high_quality
                - new.food_reduction_faktor_for_eating_high_quality)
                .abs()
                > f32::EPSILON,
        );
        push(
            "grown_up_food_store_max",
            (old.grown_up_food_store_max - new.grown_up_food_store_max).abs() > f32::EPSILON,
        );
        // C-SS-AGE-FOOD
        push(
            "new_born_food_store_max",
            (old.new_born_food_store_max - new.new_born_food_store_max).abs() > f32::EPSILON,
        );
        push(
            "old_age_food_store_max",
            (old.old_age_food_store_max - new.old_age_food_store_max).abs() > f32::EPSILON,
        );
        push(
            "min_biome_speed_factor",
            (old.min_biome_speed_factor - new.min_biome_speed_factor).abs() > f32::EPSILON,
        );
        push(
            "hitpoints_speed_factor",
            (old.hitpoints_speed_factor - new.hitpoints_speed_factor).abs() > f32::EPSILON,
        );
        push(
            "combat_reputation_restore_per_year",
            (old.combat_reputation_restore_per_year - new.combat_reputation_restore_per_year)
                .abs()
                > f32::EPSILON,
        );
        // C-SS-MORE-KNOBS
        push(
            "exhaustion_healing_factor",
            (old.exhaustion_healing_factor - new.exhaustion_healing_factor).abs() > f32::EPSILON,
        );
        push(
            "wound_damage_factor",
            (old.wound_damage_factor - new.wound_damage_factor).abs() > f32::EPSILON,
        );
        push(
            "wound_healing_factor",
            (old.wound_healing_factor - new.wound_healing_factor).abs() > f32::EPSILON,
        );
        push(
            "exhaustion_healing_for_male_factor",
            (old.exhaustion_healing_for_male_factor - new.exhaustion_healing_for_male_factor).abs()
                > f32::EPSILON,
        );
        // C-SS-TEMP-HEAL
        push(
            "temperature_hits_damage_factor",
            (old.temperature_hits_damage_factor - new.temperature_hits_damage_factor).abs()
                > f32::EPSILON,
        );
        push(
            "temperature_exhaustion_damage_factor",
            (old.temperature_exhaustion_damage_factor - new.temperature_exhaustion_damage_factor)
                .abs()
                > f32::EPSILON,
        );
        push(
            "max_movement_quad_jump_distance_before_force",
            (old.max_movement_quad_jump_distance_before_force
                - new.max_movement_quad_jump_distance_before_force)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_restore_factor_while_feeding",
            (old.food_restore_factor_while_feeding - new.food_restore_factor_while_feeding)
                .abs()
                > f32::EPSILON,
        );
        push(
            "max_has_eaten_for_next_generation",
            (old.max_has_eaten_for_next_generation - new.max_has_eaten_for_next_generation)
                .abs()
                > f32::EPSILON,
        );
        push(
            "has_eaten_reduction_for_next_generation",
            (old.has_eaten_reduction_for_next_generation
                - new.has_eaten_reduction_for_next_generation)
                .abs()
                > f32::EPSILON,
        );
        // WALLET-COINS
        push(
            "coins_on_wounding_factor",
            (old.coins_on_wounding_factor - new.coins_on_wounding_factor).abs() > f32::EPSILON,
        );
        // C-SS-MORE-BATCH3
        push(
            "combat_exhaustion_cost_per_attack",
            (old.combat_exhaustion_cost_per_attack - new.combat_exhaustion_cost_per_attack)
                .abs()
                > f32::EPSILON,
        );
        push(
            "min_age_to_eat",
            (old.min_age_to_eat - new.min_age_to_eat).abs() > f32::EPSILON,
        );
        push(
            "max_child_age_for_breast_feeding",
            (old.max_child_age_for_breast_feeding - new.max_child_age_for_breast_feeding)
                .abs()
                > f32::EPSILON,
        );
        push(
            "ally_considered_close",
            (old.ally_considered_close - new.ally_considered_close).abs() > f32::EPSILON,
        );
        push(
            "min_movement_age_in_sec",
            (old.min_movement_age_in_sec - new.min_movement_age_in_sec).abs() > f32::EPSILON,
        );
        // C-SS-MORE-BATCH4
        push(
            "cursed_receive_damage_factor",
            (old.cursed_receive_damage_factor - new.cursed_receive_damage_factor).abs()
                > f32::EPSILON,
        );
        push(
            "cursed_make_damage_factor",
            (old.cursed_make_damage_factor - new.cursed_make_damage_factor).abs() > f32::EPSILON,
        );
        push(
            "pickup_baby_max_distance",
            (old.pickup_baby_max_distance - new.pickup_baby_max_distance).abs() > f32::EPSILON,
        );
        push(
            "inherit_coins_factor",
            (old.inherit_coins_factor - new.inherit_coins_factor).abs() > f32::EPSILON,
        );
        push(
            "min_age_fertile",
            (old.min_age_fertile - new.min_age_fertile).abs() > f32::EPSILON,
        );
        push(
            "max_age_fertile",
            (old.max_age_fertile - new.max_age_fertile).abs() > f32::EPSILON,
        );
        // C-SS-MORE-BATCH5
        push(
            "weapon_cooldown_factor",
            (old.weapon_cooldown_factor - new.weapon_cooldown_factor).abs() > f32::EPSILON,
        );
        push(
            "weapon_cooldown_factor_if_wounding",
            (old.weapon_cooldown_factor_if_wounding - new.weapon_cooldown_factor_if_wounding)
                .abs()
                > f32::EPSILON,
        );
        push(
            "close_enemy_with_weapon_speed_factor",
            (old.close_enemy_with_weapon_speed_factor
                - new.close_enemy_with_weapon_speed_factor)
                .abs()
                > f32::EPSILON,
        );
        push(
            "exhaustion_on_jump",
            (old.exhaustion_on_jump - new.exhaustion_on_jump).abs() > f32::EPSILON,
        );
        push(
            "hungry_work_heat",
            (old.hungry_work_heat - new.hungry_work_heat).abs() > f32::EPSILON,
        );
        push(
            "ai_speed_factor_serf",
            (old.ai_speed_factor_serf - new.ai_speed_factor_serf).abs() > f32::EPSILON,
        );
        push(
            "ai_speed_factor_commoner",
            (old.ai_speed_factor_commoner - new.ai_speed_factor_commoner).abs() > f32::EPSILON,
        );
        push(
            "ai_speed_factor_noble",
            (old.ai_speed_factor_noble - new.ai_speed_factor_noble).abs() > f32::EPSILON,
        );
        push(
            "starting_eve_age",
            (old.starting_eve_age - new.starting_eve_age).abs() > f32::EPSILON,
        );
        push(
            "eve_or_adam_birth_chance",
            (old.eve_or_adam_birth_chance - new.eve_or_adam_birth_chance).abs() > 1e-12,
        );
        push(
            "spawn_ai_as_eve",
            old.spawn_ai_as_eve != new.spawn_ai_as_eve,
        );
        push(
            "allow_humans_born_to_ais",
            old.allow_humans_born_to_ais != new.allow_humans_born_to_ais,
        );
        push(
            "max_players_before_starting_as_child",
            old.max_players_before_starting_as_child != new.max_players_before_starting_as_child,
        );
        push(
            "obj_decay_chance",
            (old.obj_decay_chance - new.obj_decay_chance).abs() > 1e-12,
        );
        push(
            "floor_decay_chance",
            (old.floor_decay_chance - new.floor_decay_chance).abs() > 1e-12,
        );
        push(
            "obj_respawn_chance",
            (old.obj_respawn_chance - new.obj_respawn_chance).abs() > 1e-12,
        );
        push(
            "grow_back_plants_increase_if_low_population",
            (old.grow_back_plants_increase_if_low_population
                - new.grow_back_plants_increase_if_low_population)
                .abs()
                > 1e-12,
        );
        push(
            "grow_back_original_plants_factor",
            (old.grow_back_original_plants_factor - new.grow_back_original_plants_factor).abs()
                > 1e-12,
        );
        push(
            "grow_new_plants_from_existing_factor",
            (old.grow_new_plants_from_existing_factor
                - new.grow_new_plants_from_existing_factor)
                .abs()
                > 1e-12,
        );
        push(
            "spring_wild_food_regrow_chance",
            (old.spring_wild_food_regrow_chance - new.spring_wild_food_regrow_chance).abs()
                > 1e-12,
        );
        push(
            "winter_wild_food_decay_chance",
            (old.winter_wild_food_decay_chance - new.winter_wild_food_decay_chance).abs()
                > 1e-12,
        );
        push(
            "hot_season_temperature_factor",
            (old.hot_season_temperature_factor - new.hot_season_temperature_factor).abs()
                > 1e-12,
        );
        push(
            "cold_season_temperature_factor",
            (old.cold_season_temperature_factor - new.cold_season_temperature_factor).abs()
                > 1e-12,
        );
        push(
            "cursed_grave_time",
            (old.cursed_grave_time - new.cursed_grave_time).abs() > f32::EPSILON,
        );
        push(
            "animal_decay_factor",
            (old.animal_decay_factor - new.animal_decay_factor).abs() > 1e-12,
        );
        push(
            "obj_decay_factor_for_permanent",
            (old.obj_decay_factor_for_permanent - new.obj_decay_factor_for_permanent).abs()
                > 1e-12,
        );
        push(
            "obj_decay_factor_for_food",
            (old.obj_decay_factor_for_food - new.obj_decay_factor_for_food).abs() > 1e-12,
        );
        push(
            "obj_decay_factor_for_clothing",
            (old.obj_decay_factor_for_clothing - new.obj_decay_factor_for_clothing).abs() > 1e-12,
        );
        push(
            "obj_decay_factor_for_walls",
            (old.obj_decay_factor_for_walls - new.obj_decay_factor_for_walls).abs() > 1e-12,
        );
        push(
            "obj_decay_factor_per_tech_level",
            (old.obj_decay_factor_per_tech_level - new.obj_decay_factor_per_tech_level).abs()
                > 1e-12,
        );
        push(
            "decay_factor_in_deep_water",
            (old.decay_factor_in_deep_water - new.decay_factor_in_deep_water).abs() > 1e-12,
        );
        push(
            "decay_factor_in_mountain",
            (old.decay_factor_in_mountain - new.decay_factor_in_mountain).abs() > 1e-12,
        );
        push(
            "decay_factor_in_walkable_water",
            (old.decay_factor_in_walkable_water - new.decay_factor_in_walkable_water).abs()
                > 1e-12,
        );
        push(
            "decay_factor_in_jungle",
            (old.decay_factor_in_jungle - new.decay_factor_in_jungle).abs() > 1e-12,
        );
        push(
            "decay_factor_in_swamp",
            (old.decay_factor_in_swamp - new.decay_factor_in_swamp).abs() > 1e-12,
        );
        push(
            "score_factor",
            (old.score_factor - new.score_factor).abs() > 1e-12,
        );
        push(
            "ancestor_prestige_factor",
            (old.ancestor_prestige_factor - new.ancestor_prestige_factor).abs() > 1e-12,
        );
        push(
            "display_score_factor",
            (old.display_score_factor - new.display_score_factor).abs() > 1e-12,
        );
        push(
            "display_score_on",
            old.display_score_on != new.display_score_on,
        );
        push(
            "max_coins_per_chest",
            old.max_coins_per_chest != new.max_coins_per_chest,
        );
        push(
            "max_coins_per_pouch",
            old.max_coins_per_pouch != new.max_coins_per_pouch,
        );
        push(
            "chance_for_female_child",
            (old.chance_for_female_child - new.chance_for_female_child).abs() > 1e-12,
        );
        push(
            "chance_for_other_child_color",
            (old.chance_for_other_child_color - new.chance_for_other_child_color).abs() > 1e-12,
        );
        push(
            "chance_for_other_child_color_if_close_to_wrong_special_biome",
            (old.chance_for_other_child_color_if_close_to_wrong_special_biome
                - new.chance_for_other_child_color_if_close_to_wrong_special_biome)
                .abs()
                > 1e-12,
        );
        push(
            "little_kids_per_mother",
            old.little_kids_per_mother != new.little_kids_per_mother,
        );
        push(
            "new_child_exhaustion_for_mother",
            (old.new_child_exhaustion_for_mother - new.new_child_exhaustion_for_mother).abs()
                > 1e-12,
        );
        push(
            "ai_mother_birth_mali_for_human_child",
            (old.ai_mother_birth_mali_for_human_child - new.ai_mother_birth_mali_for_human_child)
                .abs()
                > 1e-12,
        );
        push(
            "human_mother_birth_mali_for_ai_child",
            (old.human_mother_birth_mali_for_ai_child - new.human_mother_birth_mali_for_ai_child)
                .abs()
                > 1e-12,
        );
        push(
            "spawn_at_last_dead",
            old.spawn_at_last_dead != new.spawn_at_last_dead,
        );
        push(
            "temperature_own_tile_rate",
            (old.temperature_own_tile_rate - new.temperature_own_tile_rate).abs() > 1e-12,
        );
        push(
            "temperature_balance_rate",
            (old.temperature_balance_rate - new.temperature_balance_rate).abs() > 1e-12,
        );
        push(
            "temperature_local_heat_factor",
            (old.temperature_local_heat_factor - new.temperature_local_heat_factor).abs() > 1e-12,
        );
        push(
            "average_season_temperature_impact",
            (old.average_season_temperature_impact - new.average_season_temperature_impact).abs()
                > 1e-12,
        );
        push(
            "ai_total_score_factor",
            (old.ai_total_score_factor - new.ai_total_score_factor).abs() > 1e-12,
        );
        push(
            "old_grave_decay_mali",
            (old.old_grave_decay_mali - new.old_grave_decay_mali).abs() > 1e-12,
        );
        push(
            "cursed_grave_mali",
            (old.cursed_grave_mali - new.cursed_grave_mali).abs() > 1e-12,
        );
        push(
            "max_distance_close",
            old.max_distance_close != new.max_distance_close,
        );
        push(
            "max_distance_map_changes",
            old.max_distance_map_changes != new.max_distance_map_changes,
        );
        push(
            "max_distance_say",
            old.max_distance_say != new.max_distance_say,
        );
        push(
            "send_move_every_x_ticks",
            old.send_move_every_x_ticks != new.send_move_every_x_ticks,
        );
        push(
            "max_distance_cose_for_movement",
            old.max_distance_cose_for_movement != new.max_distance_cose_for_movement,
        );
        push(
            "max_distance_say_ai",
            (old.max_distance_say_ai - new.max_distance_say_ai).abs() > 1e-12,
        );
        push(
            "max_distance_auto_exile_attacker",
            old.max_distance_auto_exile_attacker != new.max_distance_auto_exile_attacker,
        );
        push(
            "speed_with_both_shoes",
            (old.speed_with_both_shoes - new.speed_with_both_shoes).abs() > 1e-12,
        );
        push(
            "aging_factor_while_starving",
            (old.aging_factor_while_starving - new.aging_factor_while_starving).abs() > 1e-12,
        );
        push(
            "grown_up_age",
            (old.grown_up_age - new.grown_up_age).abs() > 1e-12,
        );
        push(
            "food_use_child_faktor",
            (old.food_use_child_faktor - new.food_use_child_faktor).abs() > 1e-12,
        );
        push(
            "ai_food_use_factor_serf",
            (old.ai_food_use_factor_serf - new.ai_food_use_factor_serf).abs() > 1e-12,
        );
        push(
            "ai_food_use_factor_commoner",
            (old.ai_food_use_factor_commoner - new.ai_food_use_factor_commoner).abs() > 1e-12,
        );
        push(
            "ai_food_use_factor_noble",
            (old.ai_food_use_factor_noble - new.ai_food_use_factor_noble).abs() > 1e-12,
        );
        push(
            "eve_food_use_factor",
            (old.eve_food_use_factor - new.eve_food_use_factor).abs() > 1e-12,
        );
        push(
            "aging_factor_human_born_to_ai",
            (old.aging_factor_human_born_to_ai - new.aging_factor_human_born_to_ai).abs()
                > 1e-12,
        );
        push(
            "aging_factor_ai_born_to_human",
            (old.aging_factor_ai_born_to_human - new.aging_factor_ai_born_to_human).abs()
                > 1e-12,
        );
        push(
            "eve_damage_factor",
            (old.eve_damage_factor - new.eve_damage_factor).abs() > 1e-12,
        );
        push(
            "target_wounded_damage_factor",
            (old.target_wounded_damage_factor - new.target_wounded_damage_factor).abs() > 1e-12,
        );
        push(
            "male_damage_factor",
            (old.male_damage_factor - new.male_damage_factor).abs() > 1e-12,
        );
        push(
            "animal_damage_factor",
            (old.animal_damage_factor - new.animal_damage_factor).abs() > 1e-12,
        );
        push(
            "animal_damage_factor_in_winter",
            (old.animal_damage_factor_in_winter - new.animal_damage_factor_in_winter).abs()
                > 1e-12,
        );
        push(
            "animal_damage_factor_if_attacked",
            (old.animal_damage_factor_if_attacked - new.animal_damage_factor_if_attacked).abs()
                > 1e-12,
        );
        push(
            "weapon_damage_factor",
            (old.weapon_damage_factor - new.weapon_damage_factor).abs() > 1e-12,
        );
        push(
            "grave_blocking_distance",
            (old.grave_blocking_distance - new.grave_blocking_distance).abs() > 1e-12,
        );
        push(
            "max_players_before_activating_grave_curse",
            old.max_players_before_activating_grave_curse
                != new.max_players_before_activating_grave_curse,
        );
        push(
            "max_players_before_forbid_touch_grave",
            old.max_players_before_forbid_touch_grave
                != new.max_players_before_forbid_touch_grave,
        );
        push(
            "combat_angry_time_before_attack",
            (old.combat_angry_time_before_attack - new.combat_angry_time_before_attack).abs()
                > 1e-12,
        );
        push(
            "combat_angry_time_minimum",
            (old.combat_angry_time_minimum - new.combat_angry_time_minimum).abs() > 1e-12,
        );
        push(
            "chance_for_domestic_animal_dying_factor",
            (old.chance_for_domestic_animal_dying_factor
                - new.chance_for_domestic_animal_dying_factor)
                .abs()
                > 1e-12,
        );
        push("door_ids", old.door_ids != new.door_ids);
        push(
            "ai_ignored_floor_ids",
            old.ai_ignored_floor_ids != new.ai_ignored_floor_ids,
        );
        push("secret", old.secret != new.secret);
        push(
            "allow_debug_commands",
            old.allow_debug_commands != new.allow_debug_commands,
        );
        push(
            "debug_say_player_position",
            old.debug_say_player_position != new.debug_say_player_position,
        );
        push(
            "ai_time_to_wait_if_crafting_failed",
            (old.ai_time_to_wait_if_crafting_failed - new.ai_time_to_wait_if_crafting_failed)
                .abs()
                > 1e-12,
        );
        push(
            "ai_max_search_radius",
            old.ai_max_search_radius != new.ai_max_search_radius,
        );
        push(
            "ai_max_search_increment",
            old.ai_max_search_increment != new.ai_max_search_increment,
        );
        push(
            "ai_memory_max_entries",
            old.ai_memory_max_entries != new.ai_memory_max_entries,
        );
        push(
            "ai_chat_memory_max_entries",
            old.ai_chat_memory_max_entries != new.ai_chat_memory_max_entries,
        );
        push(
            "ai_ignore_time_transitions_longer_then",
            (old.ai_ignore_time_transitions_longer_then
                - new.ai_ignore_time_transitions_longer_then)
                .abs()
                > 1e-12,
        );
        push(
            "alternative_outcome_percent_increase_per_hit",
            (old.alternative_outcome_percent_increase_per_hit
                - new.alternative_outcome_percent_increase_per_hit)
                .abs()
                > 1e-12,
        );
        push(
            "alternative_outcome_hits_decrease_on_success",
            (old.alternative_outcome_hits_decrease_on_success
                - new.alternative_outcome_hits_decrease_on_success)
                .abs()
                > 1e-12,
        );
        push(
            "fortification_cost_per_hit",
            (old.fortification_cost_per_hit - new.fortification_cost_per_hit).abs() > 1e-12,
        );
        push(
            "reduce_age_needed_to_pickup_objects",
            (old.reduce_age_needed_to_pickup_objects - new.reduce_age_needed_to_pickup_objects)
                .abs()
                > 1e-12,
        );
        push(
            "chance_animals_pass_blocking_biome",
            (old.chance_animals_pass_blocking_biome - new.chance_animals_pass_blocking_biome)
                .abs()
                > 1e-12,
        );
        push(
            "chance_preferred_biome",
            (old.chance_preferred_biome - new.chance_preferred_biome).abs() > 1e-12,
        );
        push(
            "close_grave_speed_mali",
            (old.close_grave_speed_mali - new.close_grave_speed_mali).abs() > 1e-12,
        );
        push(
            "temperature_speed_impact",
            (old.temperature_speed_impact - new.temperature_speed_impact).abs() > 1e-12,
        );
        push(
            "min_speed_reduction_per_contained_obj",
            (old.min_speed_reduction_per_contained_obj
                - new.min_speed_reduction_per_contained_obj)
                .abs()
                > 1e-12,
        );
        push(
            "loved_food_use_chance",
            (old.loved_food_use_chance - new.loved_food_use_chance).abs() > 1e-12,
        );
        push(
            "max_age_for_allowing_cloth_and_pickup_from_others",
            (old.max_age_for_allowing_cloth_and_pickup_from_others
                - new.max_age_for_allowing_cloth_and_pickup_from_others)
                .abs()
                > 1e-12,
        );
        push(
            "max_age_for_allowing_die",
            (old.max_age_for_allowing_die - new.max_age_for_allowing_die).abs() > 1e-12,
        );
        push(
            "prestige_cost_for_die",
            (old.prestige_cost_for_die - new.prestige_cost_for_die).abs() > 1e-12,
        );
        push(
            "starting_family_name",
            old.starting_family_name != new.starting_family_name,
        );
        push("starting_name", old.starting_name != new.starting_name);
        push(
            "found_family_needed_prestige",
            (old.found_family_needed_prestige - new.found_family_needed_prestige).abs() > 1e-12,
        );
        push(
            "found_family_cost",
            (old.found_family_cost - new.found_family_cost).abs() > 1e-12,
        );
        push(
            "found_family_needed_followers",
            old.found_family_needed_followers != new.found_family_needed_followers,
        );
        push(
            "found_family_break_alliance_chance",
            (old.found_family_break_alliance_chance - new.found_family_break_alliance_chance)
                .abs()
                > 1e-12,
        );
        push(
            "pickup_exhaustion_gain",
            (old.pickup_exhaustion_gain - new.pickup_exhaustion_gain).abs() > 1e-12,
        );
        push(
            "pickup_feeding_food_restore",
            (old.pickup_feeding_food_restore - new.pickup_feeding_food_restore).abs() > 1e-12,
        );
        push(
            "death_with_food_store_max",
            (old.death_with_food_store_max - new.death_with_food_store_max).abs() > 1e-12,
        );
        push(
            "food_store_max_reduction_while_starving",
            (old.food_store_max_reduction_while_starving
                - new.food_store_max_reduction_while_starving)
                .abs()
                > 1e-12,
        );
        push(
            "temperature_reduction_per_drinking",
            (old.temperature_reduction_per_drinking - new.temperature_reduction_per_drinking)
                .abs()
                > 1e-12,
        );
        push(
            "max_stored_water",
            (old.max_stored_water - new.max_stored_water).abs() > 1e-12,
        );
        push(
            "max_jumps_per_ten_sec",
            (old.max_jumps_per_ten_sec - new.max_jumps_per_ten_sec).abs() > 1e-12,
        );
        push(
            "temperature_impact_per_sec",
            (old.temperature_impact_per_sec - new.temperature_impact_per_sec).abs() > 1e-12,
        );
        push(
            "temperature_impact_per_sec_if_good",
            (old.temperature_impact_per_sec_if_good - new.temperature_impact_per_sec_if_good)
                .abs()
                > 1e-12,
        );
        push(
            "temperature_in_water_factor",
            (old.temperature_in_water_factor - new.temperature_in_water_factor).abs() > 1e-12,
        );
        push(
            "temperature_impact_below",
            (old.temperature_impact_below - new.temperature_impact_below).abs() > 1e-12,
        );
        push(
            "temperature_impact_color_factor",
            (old.temperature_impact_color_factor - new.temperature_impact_color_factor).abs()
                > 1e-12,
        );
        push(
            "allow_eating_or_feeding_if_ill",
            old.allow_eating_or_feeding_if_ill != new.allow_eating_or_feeding_if_ill,
        );
        push(
            "resistance_against_fever_for_eating_mushrooms",
            (old.resistance_against_fever_for_eating_mushrooms
                - new.resistance_against_fever_for_eating_mushrooms)
                .abs()
                > 1e-12,
        );
        push(
            "exhaustion_yellow_fever_per_sec",
            (old.exhaustion_yellow_fever_per_sec - new.exhaustion_yellow_fever_per_sec).abs()
                > 1e-12,
        );
        push(
            "min_health_food_store_max_factor",
            (old.min_health_food_store_max_factor - new.min_health_food_store_max_factor).abs()
                > 1e-12,
        );
        push(
            "max_health_food_store_max_factor",
            (old.max_health_food_store_max_factor - new.max_health_food_store_max_factor).abs()
                > 1e-12,
        );
        push(
            "min_health_aging_factor",
            (old.min_health_aging_factor - new.min_health_aging_factor).abs() > 1e-12,
        );
        push(
            "max_health_aging_factor",
            (old.max_health_aging_factor - new.max_health_aging_factor).abs() > 1e-12,
        );
        push(
            "min_health_per_year",
            (old.min_health_per_year - new.min_health_per_year).abs() > 1e-12,
        );
        push(
            "max_age",
            (old.max_age - new.max_age).abs() > 1e-12,
        );
        push(
            "animal_deadly_distance_factor",
            (old.animal_deadly_distance_factor - new.animal_deadly_distance_factor).abs()
                > 1e-12,
        );
        push(
            "chance_for_animal_dying_factor_if_in_loved_biome",
            (old.chance_for_animal_dying_factor_if_in_loved_biome
                - new.chance_for_animal_dying_factor_if_in_loved_biome)
                .abs()
                > 1e-12,
        );
        push(
            "offspring_factor_if_animal_pop_is_low",
            (old.offspring_factor_if_animal_pop_is_low - new.offspring_factor_if_animal_pop_is_low)
                .abs()
                > 1e-12,
        );
        push(
            "max_offspring_factor",
            (old.max_offspring_factor - new.max_offspring_factor).abs() > 1e-12,
        );
        push(
            "offspring_factor_low_animal_population_below",
            (old.offspring_factor_low_animal_population_below
                - new.offspring_factor_low_animal_population_below)
                .abs()
                > 1e-12,
        );
        keys
    }

    /// Canonical list of all LiveSettings field names (for force_reload coverage tests).
    pub fn live_settings_key_names() -> &'static [&'static str] {
        &[
            "sim_speed",
            "timed_movement",
            "move_jump_max_chebyshev",
            "broadcast_all_updates",
            "intent_drain_budget",
            "shutdown_countdown_secs",
            "shutdown_apocalypse_secs",
            "client_version_strict",
            "eternal_winter",
            "season_length_secs",
            "npc_enabled",
            "npc_min",
            "npc_max",
            "max_players",
            "last_vanilla_id",
            "open_life_client_name",
            "ai_think_period_ticks",
            "ai_reaction_time",
            "ai_reaction_time_serf",
            "ai_reaction_time_noble",
            "ai_reaction_time_factor_if_angry",
            "ai_observe_radius",
            "ai_craft_radius",
            "settings_hot_reload",
            "settings_reload_every_ticks",
            "twin_peers",
            "lockpick_success_chance",
            "lockpick_fail_chance",
            "lockpick_exhaustion_cost",
            "lockpick_coin_cost",
            "food_use_per_second",
            "healing_per_second",
            "ageing_seconds_per_year",
            "initial_player_move_speed",
            "speed_factor",
            "yum_bonus",
            "chance_for_offspring",
            "chance_for_animal_dying",
            "biome_animal_hit_chance",
            "hungry_work_cost",
            "birth_prestige_factor",
            "ally_strength_too_low_for_pickup",
            "time_confirm_new_follower",
            "hire_cost",
            "hire_cost_increase_per_person",
            "auto_follow_player",
            "prestige_cost_per_damage_for_ally",
            // C-SS-MORE PrestigeCost* non-ally
            "prestige_cost_per_damage_for_child",
            "prestige_cost_per_damage_for_elderly",
            "prestige_cost_per_damage_for_close_relatives",
            "prestige_cost_per_damage_for_women_without_weapon",
            // C-SS-FULL-TABLE food / yum restore family
            "food_factor",
            "food_factor_eaten_more_than_eight_percent",
            "food_factor_eaten_more_than_ten_percent",
            "food_factor_eaten_less_than_five_percent",
            "food_factor_eaten_less_than_three_percent",
            "food_factor_eaten_less_than_one_percent",
            "yum_food_restore",
            "loved_food_restore",
            "yum_new_craving_chance",
            "food_reduction_per_eating",
            "food_reduction_faktor_for_eating_meh",
            "health_lost_when_eating_meh",
            "health_lost_when_eating_super_meh",
            // C-SS-TAIL-KNOBS
            "food_reduction_faktor_for_eating_high_quality",
            "grown_up_food_store_max",
            // C-SS-AGE-FOOD
            "new_born_food_store_max",
            "old_age_food_store_max",
            "min_biome_speed_factor",
            "hitpoints_speed_factor",
            "combat_reputation_restore_per_year",
            // C-SS-MORE-KNOBS
            "exhaustion_healing_factor",
            "wound_damage_factor",
            "wound_healing_factor",
            // C-SS-MALE-HEAL / C-SS-MORE-BATCH3
            "exhaustion_healing_for_male_factor",
            // C-SS-TEMP-HEAL
            "temperature_hits_damage_factor",
            "temperature_exhaustion_damage_factor",
            "max_movement_quad_jump_distance_before_force",
            "food_restore_factor_while_feeding",
            "max_has_eaten_for_next_generation",
            "has_eaten_reduction_for_next_generation",
            // WALLET-COINS
            "coins_on_wounding_factor",
            // C-SS-MORE-BATCH3
            "combat_exhaustion_cost_per_attack",
            "min_age_to_eat",
            "max_child_age_for_breast_feeding",
            "ally_considered_close",
            "min_movement_age_in_sec",
            // C-SS-MORE-BATCH4
            "cursed_receive_damage_factor",
            "cursed_make_damage_factor",
            "pickup_baby_max_distance",
            "inherit_coins_factor",
            "min_age_fertile",
            "max_age_fertile",
            // C-SS-MORE-BATCH5
            "weapon_cooldown_factor",
            "weapon_cooldown_factor_if_wounding",
            "close_enemy_with_weapon_speed_factor",
            "exhaustion_on_jump",
            "hungry_work_heat",
            "ai_speed_factor_serf",
            "ai_speed_factor_commoner",
            "ai_speed_factor_noble",
            // SETTINGS-LONG-TAIL
            "starting_eve_age",
            "eve_or_adam_birth_chance",
            "spawn_ai_as_eve",
            "allow_humans_born_to_ais",
            "max_players_before_starting_as_child",
            "obj_decay_chance",
            "floor_decay_chance",
            "obj_respawn_chance",
            "grow_back_plants_increase_if_low_population",
            "grow_back_original_plants_factor",
            "grow_new_plants_from_existing_factor",
            "spring_wild_food_regrow_chance",
            "winter_wild_food_decay_chance",
            "hot_season_temperature_factor",
            "cold_season_temperature_factor",
            "cursed_grave_time",
            "animal_decay_factor",
            "obj_decay_factor_for_permanent",
            "obj_decay_factor_for_food",
            "obj_decay_factor_for_clothing",
            "obj_decay_factor_for_walls",
            "obj_decay_factor_per_tech_level",
            "decay_factor_in_deep_water",
            "decay_factor_in_mountain",
            "decay_factor_in_walkable_water",
            "decay_factor_in_jungle",
            "decay_factor_in_swamp",
            "score_factor",
            "ancestor_prestige_factor",
            "display_score_factor",
            "display_score_on",
            "max_coins_per_chest",
            "max_coins_per_pouch",
            "chance_for_female_child",
            "chance_for_other_child_color",
            "chance_for_other_child_color_if_close_to_wrong_special_biome",
            "little_kids_per_mother",
            "new_child_exhaustion_for_mother",
            "ai_mother_birth_mali_for_human_child",
            "human_mother_birth_mali_for_ai_child",
            "spawn_at_last_dead",
            "temperature_own_tile_rate",
            "temperature_balance_rate",
            "temperature_local_heat_factor",
            "average_season_temperature_impact",
            "ai_total_score_factor",
            "old_grave_decay_mali",
            "cursed_grave_mali",
            "max_distance_close",
            "max_distance_map_changes",
            "max_distance_say",
            "send_move_every_x_ticks",
            "max_distance_cose_for_movement",
            "max_distance_say_ai",
            "max_distance_auto_exile_attacker",
            "speed_with_both_shoes",
            "aging_factor_while_starving",
            "grown_up_age",
            "food_use_child_faktor",
            "ai_food_use_factor_serf",
            "ai_food_use_factor_commoner",
            "ai_food_use_factor_noble",
            "eve_food_use_factor",
            "aging_factor_human_born_to_ai",
            "aging_factor_ai_born_to_human",
            "eve_damage_factor",
            "target_wounded_damage_factor",
            "male_damage_factor",
            "animal_damage_factor",
            "animal_damage_factor_in_winter",
            "animal_damage_factor_if_attacked",
            "weapon_damage_factor",
            "grave_blocking_distance",
            "max_players_before_activating_grave_curse",
            "max_players_before_forbid_touch_grave",
            "combat_angry_time_before_attack",
            "combat_angry_time_minimum",
            "chance_for_domestic_animal_dying_factor",
            "door_ids",
            "ai_ignored_floor_ids",
            "secret",
            "allow_debug_commands",
            "debug_say_player_position",
            "ai_time_to_wait_if_crafting_failed",
            "ai_max_search_radius",
            "ai_max_search_increment",
            "ai_ignore_time_transitions_longer_then",
            "ai_memory_max_entries",
            "ai_chat_memory_max_entries",
            "alternative_outcome_percent_increase_per_hit",
            "alternative_outcome_hits_decrease_on_success",
            "fortification_cost_per_hit",
            "reduce_age_needed_to_pickup_objects",
            "chance_animals_pass_blocking_biome",
            "chance_preferred_biome",
            "close_grave_speed_mali",
            "temperature_speed_impact",
            "min_speed_reduction_per_contained_obj",
            "loved_food_use_chance",
            "max_age_for_allowing_cloth_and_pickup_from_others",
            "max_age_for_allowing_die",
            "prestige_cost_for_die",
            "starting_family_name",
            "starting_name",
            "found_family_needed_prestige",
            "found_family_cost",
            "found_family_needed_followers",
            "found_family_break_alliance_chance",
            "pickup_exhaustion_gain",
            "pickup_feeding_food_restore",
            "death_with_food_store_max",
            "food_store_max_reduction_while_starving",
            "temperature_reduction_per_drinking",
            "max_stored_water",
            "max_jumps_per_ten_sec",
            "temperature_impact_per_sec",
            "temperature_impact_per_sec_if_good",
            "temperature_in_water_factor",
            "temperature_impact_below",
            "temperature_impact_color_factor",
            "allow_eating_or_feeding_if_ill",
            "resistance_against_fever_for_eating_mushrooms",
            "exhaustion_yellow_fever_per_sec",
            "min_health_food_store_max_factor",
            "max_health_food_store_max_factor",
            "min_health_aging_factor",
            "max_health_aging_factor",
            "min_health_per_year",
            "max_age",
            "animal_deadly_distance_factor",
            "chance_for_animal_dying_factor_if_in_loved_biome",
            "offspring_factor_if_animal_pop_is_low",
            "max_offspring_factor",
            "offspring_factor_low_animal_population_below",
        ]
    }
}

#[inline]
fn sanitize_finite_or(v: f32, default: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        default
    }
}

fn sanitize_nonneg_or(v: f32, default: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else {
        default
    }
}

#[inline]
fn sanitize_i32_nonneg(v: i32, default: i32) -> i32 {
    if v >= 0 {
        v
    } else {
        default
    }
}

#[inline]
fn sanitize_i32_positive(v: i32, default: i32) -> i32 {
    if v >= 1 {
        v
    } else {
        default
    }
}

/// Positive finite (`> 0`) or default (food use, ageing, move speed).
#[inline]
fn sanitize_positive_or(v: f32, default: f32) -> f32 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        default
    }
}

/// Non-empty trimmed string or compiled default (StartingFamilyName / StartingName).
#[inline]
fn sanitize_nonempty_trim(value: &str, default: &str) -> String {
    let t = value.trim();
    if t.is_empty() {
        default.to_string()
    } else {
        t.to_string()
    }
}

/// Tracks `server.toml` mtime and reloads on the Haxe 200-tick cadence.
///
/// Haxe: `TimeHelper` `if (tick % 200 == 0 && ReadServerSettings) ServerSettings.readFromFile(false)`.
///
/// Rust uses mtime to avoid re-parse noise when the file is unchanged; when mtime
/// advances (or is unknown), the full TOML is re-read and live knobs are extracted.
#[derive(Debug, Clone)]
pub struct HotReloadTracker {
    path: PathBuf,
    last_mtime: Option<SystemTime>,
    last_live: LiveSettings,
    enabled: bool,
    every_ticks: u64,
    /// After a successful load; full config for callers that need NPC bounds etc.
    last_config: ServerConfig,
}

/// Outcome of a due hot-reload poll.
#[derive(Debug, Clone)]
pub struct HotReloadResult {
    pub config: ServerConfig,
    pub live: LiveSettings,
    /// Live field names that changed vs previous apply.
    pub changed_keys: Vec<&'static str>,
    /// True when the file was re-read (mtime advanced or first poll).
    pub reloaded_from_disk: bool,
}

impl HotReloadTracker {
    pub fn new(path: impl Into<PathBuf>, cfg: ServerConfig) -> Self {
        let path = path.into();
        let mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
        let live = cfg.live_settings();
        let enabled = cfg.settings_hot_reload;
        let every_ticks = cfg.settings_reload_every_ticks.max(1);
        Self {
            path,
            last_mtime: mtime,
            last_live: live,
            enabled,
            every_ticks,
            last_config: cfg,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn every_ticks(&self) -> u64 {
        self.every_ticks
    }

    pub fn last_live(&self) -> &LiveSettings {
        &self.last_live
    }

    pub fn last_config(&self) -> &ServerConfig {
        &self.last_config
    }

    /// Disable further reloads (Haxe `TimeHelper.ReadServerSettings = false` for debug toggles).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.last_live.settings_hot_reload = enabled;
        self.last_config.settings_hot_reload = enabled;
    }

    /// True when this sim tick is a settings-reload beat (Haxe `tick % 200 == 0`).
    pub fn is_due(&self, tick: u64) -> bool {
        self.enabled && self.every_ticks > 0 && tick > 0 && tick % self.every_ticks == 0
    }

    /// If due, re-read TOML when mtime changed (or always when mtime is unknown).
    ///
    /// Returns `Some` only when live knobs actually differ from the last apply.
    /// IO/parse failures return `Err` and leave the previous config in place.
    pub fn poll(&mut self, tick: u64) -> Result<Option<HotReloadResult>, ConfigError> {
        if !self.is_due(tick) {
            return Ok(None);
        }
        let meta = fs::metadata(&self.path)?;
        let mtime = meta.modified().ok();
        let mtime_changed = match (self.last_mtime, mtime) {
            (Some(prev), Some(now)) => now != prev,
            // Unknown mtime: re-read each due beat (closest to Haxe always-read).
            _ => true,
        };
        if !mtime_changed {
            return Ok(None);
        }

        let cfg = ServerConfig::load(&self.path)?;
        let live = cfg.live_settings();
        let changed_keys = ServerConfig::live_diff_keys(&self.last_live, &live);
        self.last_mtime = mtime;
        self.last_config = cfg.clone();
        // Honor reloaded enable/period immediately (including self-disable).
        self.enabled = live.settings_hot_reload;
        self.every_ticks = live.settings_reload_every_ticks.max(1);
        self.last_live = live.clone();

        if changed_keys.is_empty() {
            // File touched but live knobs identical — still report reload for logging callers.
            return Ok(Some(HotReloadResult {
                config: cfg,
                live,
                changed_keys,
                reloaded_from_disk: true,
            }));
        }

        Ok(Some(HotReloadResult {
            config: cfg,
            live,
            changed_keys,
            reloaded_from_disk: true,
        }))
    }

    /// Force re-read ignoring mtime (tests / operator tools).
    pub fn force_reload(&mut self) -> Result<HotReloadResult, ConfigError> {
        let cfg = ServerConfig::load(&self.path)?;
        let live = cfg.live_settings();
        let changed_keys = ServerConfig::live_diff_keys(&self.last_live, &live);
        self.last_mtime = fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        self.last_config = cfg.clone();
        self.enabled = live.settings_hot_reload;
        self.every_ticks = live.settings_reload_every_ticks.max(1);
        self.last_live = live.clone();
        Ok(HotReloadResult {
            config: cfg,
            live,
            changed_keys,
            reloaded_from_disk: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn ticket_default_on() {
        let c = ServerConfig::default();
        assert!(c.verify_ohol_ticket);
    }

    #[test]
    fn client_version_strict_default_off() {
        let c = ServerConfig::default();
        assert!(!c.client_version_strict);
        let back: ServerConfig = toml::from_str("client_version_strict = true").unwrap();
        assert!(back.client_version_strict);
    }

    #[test]
    fn selfplay_and_craft_defaults() {
        let c = ServerConfig::default();
        assert!(c.selfplay_enabled);
        assert_eq!(c.selfplay_agents, 3);
        assert_eq!(c.selfplay_agent_count(), 3);
        assert_eq!(c.craft_graph_seed_cap, 50_000);
        assert_eq!(c.craft_graph_cap(), 50_000);
        let clamped = ServerConfig {
            selfplay_agents: 99,
            craft_graph_seed_cap: 0,
            ..Default::default()
        };
        assert_eq!(clamped.selfplay_agent_count(), 3);
        assert_eq!(clamped.craft_graph_cap(), 1);
    }

    #[test]
    fn roundtrip_toml() {
        let c = ServerConfig {
            verify_ohol_ticket: false,
            selfplay_enabled: false,
            selfplay_agents: 1,
            craft_graph_seed_cap: 1_000,
            twin_peers: vec![TwinPeerConfig {
                host: "127.0.0.1".into(),
                port: 8006,
            }],
            eternal_winter: true,
            season_duration_years: 2.0,
            settings_hot_reload: false,
            settings_reload_every_ticks: 100,
            lockpick_success_chance: 25.0,
            lockpick_fail_chance: 15.0,
            lockpick_exhaustion_cost: 1.5,
            lockpick_coin_cost: 2.0,
            ..Default::default()
        };
        let s = toml::to_string(&c).unwrap();
        let back: ServerConfig = toml::from_str(&s).unwrap();
        assert!(!back.verify_ohol_ticket);
        assert!(!back.selfplay_enabled);
        assert_eq!(back.selfplay_agents, 1);
        assert_eq!(back.craft_graph_seed_cap, 1_000);
        assert_eq!(back.twin_peers.len(), 1);
        assert_eq!(back.twin_peers[0].host, "127.0.0.1");
        assert_eq!(back.twin_peers[0].port, 8006);
        assert!(back.eternal_winter);
        assert!((back.season_duration_years - 2.0).abs() < f32::EPSILON);
        assert!(!back.settings_hot_reload);
        assert_eq!(back.settings_reload_every_ticks, 100);
        assert!((back.lockpick_success_chance - 25.0).abs() < f32::EPSILON);
        assert!((back.lockpick_fail_chance - 15.0).abs() < f32::EPSILON);
        assert!((back.lockpick_exhaustion_cost - 1.5).abs() < f32::EPSILON);
        assert!((back.lockpick_coin_cost - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn twin_peers_default_empty() {
        let c = ServerConfig::default();
        assert!(c.twin_peers.is_empty());
        let back: ServerConfig = toml::from_str("").unwrap();
        assert!(back.twin_peers.is_empty());
    }

    #[test]
    fn movement_and_npc_defaults() {
        let c = ServerConfig::default();
        assert!(c.timed_movement);
        assert_eq!(c.ai_craft_radius, 50);
        assert_eq!(c.move_jump_max_chebyshev, 2);
        assert_eq!(c.intent_drain(), 64);
        assert!(c.npc_enabled);
        assert_eq!(c.npc_min, 20);
        assert_eq!(c.npc_max, 40);
        let on: ServerConfig = toml::from_str("timed_movement = true\nnpc_enabled = true").unwrap();
        assert!(on.timed_movement);
        assert!(on.npc_enabled);
        let off: ServerConfig = toml::from_str("npc_enabled = false").unwrap();
        assert!(!off.npc_enabled);
    }

    #[test]
    fn sim_speed_default_and_clamp() {
        let c = ServerConfig::default();
        assert!((c.sim_speed - 1.0).abs() < f32::EPSILON);
        assert!((c.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let fast = ServerConfig {
            sim_speed: 2.5,
            ..Default::default()
        };
        assert!((fast.sim_speed_factor() - 2.5).abs() < f32::EPSILON);
        let bad = ServerConfig {
            sim_speed: f32::NAN,
            ..Default::default()
        };
        assert!((bad.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let neg = ServerConfig {
            sim_speed: -3.0,
            ..Default::default()
        };
        assert!((neg.sim_speed_factor() - 1.0).abs() < f32::EPSILON);
        let zero = ServerConfig {
            sim_speed: 0.0,
            ..Default::default()
        };
        assert!((zero.sim_speed_factor() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sim_speed_toml_roundtrip() {
        let c = ServerConfig {
            sim_speed: 4.0,
            ..Default::default()
        };
        let s = toml::to_string(&c).unwrap();
        let back: ServerConfig = toml::from_str(&s).unwrap();
        assert!((back.sim_speed - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn season_duration_haxe_years_to_secs() {
        let c = ServerConfig::default();
        assert!(!c.eternal_winter);
        assert!((c.season_duration_years - 7.5).abs() < f32::EPSILON);
        // 7.5 years × 60 s/year = 450 s
        assert!((c.season_length_secs() - 450.0).abs() < 0.01);
        let short = ServerConfig {
            season_duration_years: 1.0,
            ..Default::default()
        };
        assert!((short.season_length_secs() - 60.0).abs() < 0.01);
        let bad = ServerConfig {
            season_duration_years: -1.0,
            ..Default::default()
        };
        assert!((bad.season_length_secs() - 450.0).abs() < 0.01);
    }

    #[test]
    fn live_settings_extracts_hot_knobs() {
        let c = ServerConfig {
            sim_speed: 2.0,
            timed_movement: false,
            eternal_winter: true,
            season_duration_years: 2.0,
            npc_max: 12,
            npc_min: 4,
            settings_hot_reload: true,
            settings_reload_every_ticks: 50,
            lockpick_success_chance: 20.0,
            lockpick_fail_chance: 30.0,
            lockpick_exhaustion_cost: 0.5,
            lockpick_coin_cost: 4.0,
            game_port: 9999, // boot-only — not in LiveSettings
            ..Default::default()
        };
        let live = c.live_settings();
        assert!((live.sim_speed - 2.0).abs() < f32::EPSILON);
        assert!(!live.timed_movement);
        assert!(live.eternal_winter);
        assert!((live.season_length_secs - 120.0).abs() < 0.01);
        assert_eq!(live.npc_max, 12);
        assert_eq!(live.npc_min, 4);
        assert_eq!(live.settings_reload_every_ticks, 50);
        assert!((live.lockpick_success_chance - 20.0).abs() < f32::EPSILON);
        assert!((live.lockpick_fail_chance - 30.0).abs() < f32::EPSILON);
        assert!((live.lockpick_exhaustion_cost - 0.5).abs() < f32::EPSILON);
        assert!((live.lockpick_coin_cost - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lockpick_defaults_and_sanitize() {
        let c = ServerConfig::default();
        assert!((c.lockpick_success_chance - 5.0).abs() < f32::EPSILON);
        assert!((c.lockpick_fail_chance - 10.0).abs() < f32::EPSILON);
        assert!((c.lockpick_exhaustion_cost - 3.0).abs() < f32::EPSILON);
        assert!((c.lockpick_coin_cost - 1.0).abs() < f32::EPSILON);
        let bad = ServerConfig {
            lockpick_success_chance: f32::NAN,
            lockpick_fail_chance: -1.0,
            lockpick_exhaustion_cost: f32::INFINITY,
            lockpick_coin_cost: -5.0,
            ..Default::default()
        };
        let live = bad.live_settings();
        assert!((live.lockpick_success_chance - 5.0).abs() < f32::EPSILON);
        assert!((live.lockpick_fail_chance - 10.0).abs() < f32::EPSILON);
        assert!((live.lockpick_exhaustion_cost - 3.0).abs() < f32::EPSILON);
        assert!((live.lockpick_coin_cost - 1.0).abs() < f32::EPSILON);
        // zero costs allowed (ops can disable coin tax)
        let zero = ServerConfig {
            lockpick_coin_cost: 0.0,
            lockpick_exhaustion_cost: 0.0,
            ..Default::default()
        }
        .live_settings();
        assert!((zero.lockpick_coin_cost - 0.0).abs() < f32::EPSILON);
        assert!((zero.lockpick_exhaustion_cost - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn live_diff_keys_lists_changes() {
        let a = ServerConfig::default().live_settings();
        let mut b = a.clone();
        b.sim_speed = 3.0;
        b.eternal_winter = true;
        b.lockpick_success_chance = 50.0;
        let keys = ServerConfig::live_diff_keys(&a, &b);
        assert!(keys.contains(&"sim_speed"));
        assert!(keys.contains(&"eternal_winter"));
        assert!(keys.contains(&"lockpick_success_chance"));
        assert!(!keys.contains(&"timed_movement"));
    }

    /// Changing only the four lockpick_* knobs reports all four (no other noise).
    // Haxe: ServerSettings.LockpickSucessChance / FailChance / ExhaustionCost / CoinCost
    #[test]
    fn live_diff_keys_all_four_lockpick_only() {
        let a = ServerConfig::default().live_settings();
        let mut b = a.clone();
        b.lockpick_success_chance = 25.0;
        b.lockpick_fail_chance = 15.0;
        b.lockpick_exhaustion_cost = 1.5;
        b.lockpick_coin_cost = 2.5;
        let keys = ServerConfig::live_diff_keys(&a, &b);
        assert!(keys.contains(&"lockpick_success_chance"));
        assert!(keys.contains(&"lockpick_fail_chance"));
        assert!(keys.contains(&"lockpick_exhaustion_cost"));
        assert!(keys.contains(&"lockpick_coin_cost"));
        assert_eq!(
            keys.iter().filter(|k| k.starts_with("lockpick_")).count(),
            4
        );
        assert!(!keys.contains(&"sim_speed"));
        assert!(!keys.contains(&"timed_movement"));
    }

    #[test]
    fn hot_reload_tracker_due_and_mtime() {
        let dir = std::env::temp_dir().join(format!(
            "ol_cfg_hot_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("server.toml");
        let mut cfg = ServerConfig::default();
        cfg.settings_reload_every_ticks = 10;
        cfg.sim_speed = 1.0;
        let text = toml::to_string_pretty(&cfg).unwrap();
        fs::write(&path, &text).unwrap();

        let mut tracker = HotReloadTracker::new(&path, cfg);
        assert!(tracker.enabled());
        assert!(!tracker.is_due(0));
        assert!(!tracker.is_due(9));
        assert!(tracker.is_due(10));
        assert!(tracker.is_due(20));

        // No mtime change → None
        assert!(tracker.poll(10).unwrap().is_none());

        // Change file + bump mtime
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut cfg2 = ServerConfig::default();
        cfg2.settings_reload_every_ticks = 10;
        cfg2.sim_speed = 2.5;
        cfg2.eternal_winter = true;
        fs::write(&path, toml::to_string_pretty(&cfg2).unwrap()).unwrap();
        // Ensure mtime advances on some FS
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(f, "\n# touch").unwrap();
        drop(f);

        let res = tracker.poll(20).unwrap().expect("should reload");
        assert!(res.reloaded_from_disk);
        assert!(res.changed_keys.contains(&"sim_speed") || res.changed_keys.contains(&"eternal_winter"));
        assert!((res.live.sim_speed - 2.5).abs() < f32::EPSILON);
        assert!(res.live.eternal_winter);
        let _ = fs::remove_dir_all(&dir);
    }

    /// `write_default` + `load_or_default` preserves LiveSettings equality.
    // Haxe: ServerSettings.writeToFile / readFromFile round-trip (subset)
    #[test]
    fn write_default_load_roundtrip_live_settings() {
        let dir = std::env::temp_dir().join(format!(
            "ol_cfg_wd_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("server.toml");
        ServerConfig::write_default(&path).unwrap();
        let loaded = ServerConfig::load_or_default(&path).unwrap();
        let a = ServerConfig::default().live_settings();
        let b = loaded.live_settings();
        assert_eq!(ServerConfig::live_diff_keys(&a, &b), Vec::<&str>::new());
        // Secrets never appear in default dump
        let text = fs::read_to_string(&path).unwrap();
        for secret in secret_omit_names() {
            assert!(
                !text.to_ascii_lowercase().contains(&secret.to_ascii_lowercase()),
                "secret name {secret} leaked into write_default"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// force_reload surfaces every LiveSettings key when all knobs change.
    // Haxe: ServerSettings.readFromFile Reflect.setField any static mid-session
    #[test]
    fn force_reload_reports_all_live_keys_when_all_change() {
        let dir = std::env::temp_dir().join(format!(
            "ol_cfg_fr_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("server.toml");
        let cfg0 = ServerConfig::default();
        fs::write(&path, toml::to_string_pretty(&cfg0).unwrap()).unwrap();
        let mut tracker = HotReloadTracker::new(&path, cfg0);

        // Mutate every live-affecting ServerConfig field away from default.
        let cfg1 = ServerConfig {
            sim_speed: 3.0,
            timed_movement: false,
            move_jump_max_chebyshev: 9,
            broadcast_all_updates: false,
            intent_drain_budget: 7,
            shutdown_countdown_secs: 9,
            shutdown_apocalypse_secs: 8,
            client_version_strict: true,
            last_vanilla_id: 4000,
            open_life_client_name: "OpenLifeClient".into(),
            eternal_winter: true,
            season_duration_years: 1.0,
            npc_enabled: false,
            npc_min: 1,
            npc_max: 2,
            max_players: 50,
            ai_think_period_ticks: 3,
            ai_reaction_time: 0.3,
            ai_reaction_time_serf: 0.4,
            ai_reaction_time_noble: 0.1,
            ai_reaction_time_factor_if_angry: 0.5,
            ai_observe_radius: 8,
            ai_craft_radius: 16,
            settings_hot_reload: true,
            settings_reload_every_ticks: 50,
            lockpick_success_chance: 40.0,
            lockpick_fail_chance: 20.0,
            lockpick_exhaustion_cost: 0.5,
            lockpick_coin_cost: 4.0,
            food_use_per_second: 0.2,
            healing_per_second: 0.25,
            ageing_seconds_per_year: 30.0,
            initial_player_move_speed: 5.0,
            speed_factor: 1.5,
            yum_bonus: 8.0,
            chance_for_offspring: 0.001,
            chance_for_animal_dying: 0.002,
            biome_animal_hit_chance: 0.25,
            hungry_work_cost: 9.0,
            birth_prestige_factor: 0.2,
            ally_strength_too_low_for_pickup: 0.5,
            time_confirm_new_follower: 7.0,
            hire_cost: 20.0,
            hire_cost_increase_per_person: 15.0,
            auto_follow_player: true, // default false → force_reload key
            prestige_cost_per_damage_for_ally: 2.5,
            prestige_cost_per_damage_for_child: 2.0,
            prestige_cost_per_damage_for_elderly: 3.0,
            prestige_cost_per_damage_for_close_relatives: 1.5,
            prestige_cost_per_damage_for_women_without_weapon: 1.25,
            food_factor: 0.7,
            food_factor_eaten_more_than_eight_percent: 0.4,
            food_factor_eaten_more_than_ten_percent: 0.3,
            food_factor_eaten_less_than_five_percent: 1.8,
            food_factor_eaten_less_than_three_percent: 2.2,
            food_factor_eaten_less_than_one_percent: 3.0,
            yum_food_restore: 0.4,
            loved_food_restore: 0.3,
            yum_new_craving_chance: 0.55,
            food_reduction_per_eating: 1.5,
            food_reduction_faktor_for_eating_meh: 0.35,
            health_lost_when_eating_meh: 0.9,
            health_lost_when_eating_super_meh: 2.5,
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
            cold_season_temperature_factor: 1.5,
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
            twin_peers: vec![TwinPeerConfig {
                host: "10.0.0.9".into(),
                port: 8009,
            }],
            // boot-only noise
            game_port: 9999,
            ..Default::default()
        };
        fs::write(&path, toml::to_string_pretty(&cfg1).unwrap()).unwrap();

        let res = tracker.force_reload().unwrap();
        let expected = ServerConfig::live_settings_key_names();
        for key in expected {
            // settings_hot_reload stays true → may not appear; every other live key must.
            if *key == "settings_hot_reload" {
                continue;
            }
            assert!(
                res.changed_keys.contains(key),
                "force_reload missing live key {key}; got {:?}",
                res.changed_keys
            );
        }
        assert!((res.live.food_use_per_second - 0.2).abs() < f32::EPSILON);
        assert!((res.live.yum_bonus - 8.0).abs() < f32::EPSILON);
        assert!((res.live.chance_for_offspring - 0.001).abs() < 1e-9);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Editing only food_use must not invent diffs on unrelated live keys.
    #[test]
    fn live_diff_food_use_only() {
        let a = ServerConfig::default().live_settings();
        let mut b = a.clone();
        b.food_use_per_second = 0.25;
        let keys = ServerConfig::live_diff_keys(&a, &b);
        assert_eq!(keys, vec!["food_use_per_second"]);
    }

    #[test]
    fn gameplay_defaults_match_haxe() {
        let c = ServerConfig::default();
        assert!((c.food_use_per_second - 0.10).abs() < f32::EPSILON);
        assert!((c.healing_per_second - 0.10).abs() < f32::EPSILON);
        assert!((c.ageing_seconds_per_year - 60.0).abs() < f32::EPSILON);
        assert!((c.initial_player_move_speed - 3.75).abs() < f32::EPSILON);
        assert!((c.speed_factor - 1.0).abs() < f32::EPSILON);
        assert!((c.yum_bonus - 5.0).abs() < f32::EPSILON);
        assert!((c.chance_for_offspring - 0.00005).abs() < 1e-12);
        assert!((c.hungry_work_cost - 5.0).abs() < f32::EPSILON);
        assert!((c.birth_prestige_factor - 0.4).abs() < f32::EPSILON);
        assert!((c.ally_strength_too_low_for_pickup - 0.0).abs() < f32::EPSILON);
        assert!((c.time_confirm_new_follower - 15.0).abs() < f32::EPSILON);
        assert!((c.hire_cost - 10.0).abs() < f32::EPSILON);
        assert!((c.hire_cost_increase_per_person - 10.0).abs() < f32::EPSILON);
        assert!((c.prestige_cost_per_damage_for_ally - 1.0).abs() < f32::EPSILON);
        // C-SS-MORE
        assert!((c.prestige_cost_per_damage_for_child - 5.0).abs() < f32::EPSILON);
        assert!((c.prestige_cost_per_damage_for_elderly - 1.0).abs() < f32::EPSILON);
        assert!((c.prestige_cost_per_damage_for_close_relatives - 0.5).abs() < f32::EPSILON);
        assert!((c.prestige_cost_per_damage_for_women_without_weapon - 0.5).abs() < f32::EPSILON);
        // C-SS-FULL-TABLE
        assert!((c.food_factor - 1.0).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_more_than_eight_percent - 0.8).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_more_than_ten_percent - 0.5).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_five_percent - 1.5).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_three_percent - 2.0).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_one_percent - 2.5).abs() < f32::EPSILON);
        assert!((c.yum_food_restore - 0.8).abs() < f32::EPSILON);
        assert!((c.loved_food_restore - 0.1).abs() < f32::EPSILON);
        assert!((c.yum_new_craving_chance - 0.2).abs() < f32::EPSILON);
        assert!((c.food_reduction_per_eating - 1.0).abs() < f32::EPSILON);
        assert!((c.food_reduction_faktor_for_eating_meh - 0.2).abs() < f32::EPSILON);
        assert!((c.health_lost_when_eating_meh - 0.5).abs() < f32::EPSILON);
        assert!((c.health_lost_when_eating_super_meh - 2.0).abs() < f32::EPSILON);
        // C-SS-TAIL-KNOBS Haxe defaults 20 / 0.2 / 3 / 0.8 / 2
        assert!((c.grown_up_food_store_max - 20.0).abs() < f32::EPSILON);
        // C-SS-AGE-FOOD Haxe defaults 4 / 10
        assert!((c.new_born_food_store_max - 4.0).abs() < f32::EPSILON);
        assert!((c.old_age_food_store_max - 10.0).abs() < f32::EPSILON);
        assert!((c.min_biome_speed_factor - 0.2).abs() < f32::EPSILON);
        assert!((c.hitpoints_speed_factor - 3.0).abs() < f32::EPSILON);
        assert!((c.food_reduction_faktor_for_eating_high_quality - 0.8).abs() < f32::EPSILON);
        assert!((c.combat_reputation_restore_per_year - 2.0).abs() < f32::EPSILON);
        // C-SS-MORE-KNOBS Haxe defaults 1.5 / 1 / 5 / 10 / 4 / 1
        assert!((c.exhaustion_healing_factor - 1.5).abs() < f32::EPSILON);
        assert!((c.wound_damage_factor - 1.0).abs() < f32::EPSILON);
        assert!((c.wound_healing_factor - 1.0).abs() < f32::EPSILON);
        // C-SS-MALE-HEAL Haxe default 1.2
        assert!((c.exhaustion_healing_for_male_factor - 1.2).abs() < f32::EPSILON);
        // C-SS-TEMP-HEAL Haxe defaults 0.5 / 0.2
        assert!((c.temperature_hits_damage_factor - 0.5).abs() < f32::EPSILON);
        assert!((c.temperature_exhaustion_damage_factor - 0.2).abs() < f32::EPSILON);
        assert!((c.max_movement_quad_jump_distance_before_force - 5.0).abs() < f32::EPSILON);
        assert!((c.food_restore_factor_while_feeding - 10.0).abs() < f32::EPSILON);
        assert!((c.max_has_eaten_for_next_generation - 4.0).abs() < f32::EPSILON);
        assert!((c.has_eaten_reduction_for_next_generation - 1.0).abs() < f32::EPSILON);
        // WALLET-COINS Haxe default 0.5
        assert!((c.coins_on_wounding_factor - 0.5).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH3 Haxe defaults 0.1 / 3 / 6 / 5 / 14
        assert!((c.combat_exhaustion_cost_per_attack - 0.1).abs() < f32::EPSILON);
        assert!((c.min_age_to_eat - 3.0).abs() < f32::EPSILON);
        assert!((c.max_child_age_for_breast_feeding - 6.0).abs() < f32::EPSILON);
        assert!((c.ally_considered_close - 5.0).abs() < f32::EPSILON);
        assert!((c.min_movement_age_in_sec - 14.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH4 Haxe defaults 1.2 / 0.5 / 1.9 / 0.8 / 14 / 42
        assert!((c.cursed_receive_damage_factor - 1.2).abs() < f32::EPSILON);
        assert!((c.cursed_make_damage_factor - 0.5).abs() < f32::EPSILON);
        assert!((c.pickup_baby_max_distance - 1.9).abs() < f32::EPSILON);
        assert!((c.inherit_coins_factor - 0.8).abs() < f32::EPSILON);
        assert!((c.min_age_fertile - 14.0).abs() < f32::EPSILON);
        assert!((c.max_age_fertile - 42.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH5 Haxe defaults 0.5 / 5 / 0.8 / 0.05 / 0.002 / 0.8 / 0.9 / 1
        assert!((c.weapon_cooldown_factor - 0.5).abs() < f32::EPSILON);
        assert!((c.weapon_cooldown_factor_if_wounding - 5.0).abs() < f32::EPSILON);
        assert!((c.close_enemy_with_weapon_speed_factor - 0.8).abs() < f32::EPSILON);
        assert!((c.exhaustion_on_jump - 0.05).abs() < f32::EPSILON);
        assert!((c.hungry_work_heat - 0.002).abs() < f32::EPSILON);
        assert!((c.ai_speed_factor_serf - 0.8).abs() < f32::EPSILON);
        assert!((c.ai_speed_factor_commoner - 0.9).abs() < f32::EPSILON);
        assert!((c.ai_speed_factor_noble - 1.0).abs() < f32::EPSILON);
        // SETTINGS-LONG-TAIL Haxe StartingEveAge = 14 / ObjDecay 0.00005 / FloorDecay 0.00001
        assert!((c.starting_eve_age - 14.0).abs() < f32::EPSILON);
        assert!((c.eve_or_adam_birth_chance - 0.025).abs() < 1e-12);
        assert!(!c.spawn_ai_as_eve);
        assert!(!c.allow_humans_born_to_ais);
        assert_eq!(c.max_players_before_starting_as_child, 0);
        assert!((c.obj_decay_chance - 0.00005).abs() < 1e-12);
        assert!((c.floor_decay_chance - 0.00001).abs() < 1e-12);
        assert!((c.obj_respawn_chance - 0.00006).abs() < 1e-12);
        assert!((c.grow_back_plants_increase_if_low_population - 2.0).abs() < 1e-12);
        assert!((c.grow_back_original_plants_factor - 0.02).abs() < 1e-12);
        assert!((c.grow_new_plants_from_existing_factor - 0.05).abs() < 1e-12);
        assert!((c.spring_wild_food_regrow_chance - 1.0).abs() < 1e-12);
        assert!((c.winter_wild_food_decay_chance - 1.5).abs() < 1e-12);
        assert!((c.hot_season_temperature_factor - 0.75).abs() < 1e-12);
        assert!((c.cold_season_temperature_factor - 0.75).abs() < 1e-12);
        assert!(c.display_score_on);
        assert_eq!(c.max_coins_per_chest, 200);
        assert_eq!(c.max_coins_per_pouch, 50);
        assert!((c.chance_for_female_child - 0.6).abs() < 1e-12);
        assert!((c.chance_for_other_child_color - 0.2).abs() < 1e-12);
        assert!(
            (c.chance_for_other_child_color_if_close_to_wrong_special_biome - 0.3).abs() < 1e-12
        );
        assert_eq!(c.little_kids_per_mother, 3);
        assert!((c.new_child_exhaustion_for_mother - 0.0).abs() < 1e-12);
        assert!((c.ai_mother_birth_mali_for_human_child - 3.0).abs() < 1e-12);
        assert!((c.human_mother_birth_mali_for_ai_child - 1.0).abs() < 1e-12);
        assert!(!c.spawn_at_last_dead);
        assert!((c.temperature_own_tile_rate - 0.05).abs() < 1e-12);
        assert!((c.temperature_balance_rate - 0.9).abs() < 1e-12);
        assert!((c.temperature_local_heat_factor - 0.005).abs() < 1e-12);
        assert!((c.average_season_temperature_impact - 0.2).abs() < 1e-12);
        assert!((c.cursed_grave_time - 12.0).abs() < f32::EPSILON);
        assert!((c.animal_decay_factor - 0.05).abs() < 1e-12);
        assert!((c.score_factor - 0.2).abs() < 1e-6);
        assert_eq!(c.send_move_every_x_ticks, -1);
        assert_eq!(c.max_distance_cose_for_movement, 30);
        assert!((c.max_distance_say_ai - 20.0).abs() < 1e-12);
        assert!((c.speed_with_both_shoes - 1.1).abs() < 1e-12);
        assert!((c.aging_factor_while_starving - 0.5).abs() < 1e-12);
        assert!((c.grown_up_age - 14.0).abs() < 1e-12);
        assert!((c.food_use_child_faktor - 1.0).abs() < 1e-12);
        assert!((c.ai_food_use_factor_serf - 0.8).abs() < 1e-12);
        assert!((c.ai_food_use_factor_commoner - 0.9).abs() < 1e-12);
        assert!((c.ai_food_use_factor_noble - 1.0).abs() < 1e-12);
        assert!((c.eve_food_use_factor - 1.0).abs() < 1e-12);
        assert!((c.eve_damage_factor - 1.0).abs() < 1e-12);
        assert!((c.target_wounded_damage_factor - 0.2).abs() < 1e-12);
        assert!((c.male_damage_factor - 1.2).abs() < 1e-12);
        assert!((c.animal_damage_factor - 1.5).abs() < 1e-12);
        assert!((c.animal_damage_factor_in_winter - 2.0).abs() < 1e-12);
        assert!((c.animal_damage_factor_if_attacked - 1.5).abs() < 1e-12);
        assert!((c.weapon_damage_factor - 1.0).abs() < 1e-12);
        assert!((c.grave_blocking_distance - 40.0).abs() < 1e-12);
        assert_eq!(c.max_players_before_activating_grave_curse, 0);
        assert_eq!(c.max_players_before_forbid_touch_grave, 9999);
        assert!((c.combat_angry_time_before_attack - 5.0).abs() < 1e-12);
        assert!((c.combat_angry_time_minimum + 60.0).abs() < 1e-12);
        assert!((c.chance_for_domestic_animal_dying_factor - 2.0).abs() < 1e-12);
        assert_eq!(c.door_ids, crate::DOOR_IDS);
        assert_eq!(c.ai_ignored_floor_ids, crate::AI_IGNORED_FLOOR_IDS);
        assert_eq!(c.secret, "JASON");
        assert!(c.allow_debug_commands);
        assert!(!c.debug_say_player_position);
        assert!((c.ai_time_to_wait_if_crafting_failed - 15.0).abs() < 1e-12);
        assert_eq!(c.ai_memory_max_entries, 20);
        assert_eq!(c.ai_chat_memory_max_entries, 100);
        assert_eq!(c.ai_max_search_radius, 60);
        assert_eq!(c.ai_max_search_increment, 30);
        assert!((c.ai_ignore_time_transitions_longer_then - 120.0).abs() < 1e-12);
        assert!((c.alternative_outcome_percent_increase_per_hit - 10.0).abs() < 1e-12);
        assert!((c.alternative_outcome_hits_decrease_on_success - 5.0).abs() < 1e-12);
        assert!((c.fortification_cost_per_hit - 1.0).abs() < 1e-12);
        assert!((c.reduce_age_needed_to_pickup_objects - 10.0).abs() < 1e-12);
        assert!((c.chance_animals_pass_blocking_biome - 0.03).abs() < 1e-12);
        assert!((c.chance_preferred_biome - 0.8).abs() < 1e-12);
        assert!((c.close_grave_speed_mali - 0.9).abs() < 1e-12);
        assert!((c.temperature_speed_impact - 1.0).abs() < 1e-12);
        assert!((c.min_speed_reduction_per_contained_obj - 0.98).abs() < 1e-12);
        assert!((c.loved_food_use_chance - 0.5).abs() < 1e-12);
        assert!((c.max_age_for_allowing_cloth_and_pickup_from_others - 10.0).abs() < 1e-12);
        assert!((c.max_age_for_allowing_die - 2.0).abs() < 1e-12);
        assert!((c.prestige_cost_for_die - 0.0).abs() < 1e-12);
        assert_eq!(c.starting_family_name, "SNOW");
        assert_eq!(c.starting_name, "SPOON");
        assert!((c.found_family_needed_prestige - 50.0).abs() < 1e-12);
        assert!((c.found_family_cost - 10.0).abs() < 1e-12);
        assert_eq!(c.found_family_needed_followers, 4);
        assert!((c.found_family_break_alliance_chance - 0.5).abs() < 1e-12);
        assert!((c.pickup_exhaustion_gain - 0.2).abs() < 1e-12);
        assert!((c.pickup_feeding_food_restore - 1.5).abs() < 1e-12);
        assert!((c.death_with_food_store_max + 0.1).abs() < 1e-12);
        assert!((c.food_store_max_reduction_while_starving - 5.0).abs() < 1e-12);
        assert!((c.temperature_reduction_per_drinking - 0.5).abs() < 1e-12);
        assert!((c.max_stored_water - 1.0).abs() < 1e-12);
        assert!((c.max_jumps_per_ten_sec - 10.0).abs() < 1e-12);
        assert!((c.temperature_impact_per_sec - 0.03).abs() < 1e-12);
        assert!((c.temperature_impact_per_sec_if_good - 0.06).abs() < 1e-12);
        assert!((c.temperature_in_water_factor - 1.5).abs() < 1e-12);
        assert!((c.temperature_impact_below - 0.6).abs() < 1e-12);
        assert!((c.temperature_impact_color_factor - 0.5).abs() < 1e-12);
        assert!(!c.allow_eating_or_feeding_if_ill);
        assert!((c.resistance_against_fever_for_eating_mushrooms - 0.2).abs() < 1e-12);
        assert!((c.exhaustion_yellow_fever_per_sec - 0.1).abs() < 1e-12);
        assert!((c.min_health_food_store_max_factor - 0.8).abs() < 1e-12);
        assert!((c.max_health_food_store_max_factor - 1.2).abs() < 1e-12);
        assert!((c.min_health_aging_factor - 0.5).abs() < 1e-12);
        assert!((c.max_health_aging_factor - 2.0).abs() < 1e-12);
        assert!((c.min_health_per_year - 1.0).abs() < 1e-12);
        assert!((c.max_age - 60.0).abs() < 1e-12);
        assert!((c.animal_deadly_distance_factor - 0.5).abs() < 1e-12);
        assert!((c.chance_for_animal_dying_factor_if_in_loved_biome - 0.1).abs() < 1e-12);
        assert!((c.offspring_factor_if_animal_pop_is_low - 10.0).abs() < 1e-12);
        assert!((c.max_offspring_factor - 1.0).abs() < 1e-12);
        assert!((c.offspring_factor_low_animal_population_below - 0.2).abs() < 1e-12);
        assert!((c.obj_decay_factor_for_food - 2.0).abs() < 1e-12);
        assert!((c.obj_decay_factor_for_clothing - 2.0).abs() < 1e-12);
        assert!((c.obj_decay_factor_for_walls - 0.2).abs() < 1e-12);
        assert!((c.obj_decay_factor_per_tech_level - 10.0).abs() < 1e-12);
        assert!((c.decay_factor_in_deep_water - 5.0).abs() < 1e-12);
        assert!((c.decay_factor_in_mountain - 3.0).abs() < 1e-12);
        assert!((c.decay_factor_in_walkable_water - 2.0).abs() < 1e-12);
        assert!((c.decay_factor_in_jungle - 2.0).abs() < 1e-12);
        assert!((c.decay_factor_in_swamp - 2.0).abs() < 1e-12);
    }

    #[test]
    fn winter_wild_food_decay_chance_and_hot_season_defaults() {
        let c = ServerConfig::default();
        assert!((c.winter_wild_food_decay_chance - 1.5).abs() < 1e-12);
        assert!((c.hot_season_temperature_factor - 0.75).abs() < 1e-12);
        assert!((c.cold_season_temperature_factor - 0.75).abs() < 1e-12);
        let live = c.live_settings();
        assert!((live.winter_wild_food_decay_chance - 1.5).abs() < 1e-12);
        assert!((live.hot_season_temperature_factor - 0.75).abs() < 1e-12);
        assert!((live.cold_season_temperature_factor - 0.75).abs() < 1e-12);
        let mut other = live.clone();
        other.winter_wild_food_decay_chance = 3.0;
        other.hot_season_temperature_factor = 1.5;
        other.cold_season_temperature_factor = 0.5;
        let keys = ServerConfig::live_diff_keys(&live, &other);
        assert!(keys.contains(&"winter_wild_food_decay_chance"));
        assert!(keys.contains(&"hot_season_temperature_factor"));
        assert!(keys.contains(&"cold_season_temperature_factor"));
    }

    #[test]
    fn field_map_critical_live_count() {
        let live = live_critical_names();
        assert!(live.len() >= 27, "live critical: {live:?}");
        assert!(live.contains(&"FoodFactor"));
        assert!(live.contains(&"YumFoodRestore"));
        assert!(live.contains(&"GrownUpFoodStoreMax"));
        // C-SS-AGE-FOOD
        assert!(live.contains(&"NewBornFoodStoreMax"));
        assert!(live.contains(&"OldAgeFoodStoreMax"));
        assert!(live.contains(&"CombatReputationRestorePerYear"));
        // C-SS-MORE-KNOBS
        assert!(live.contains(&"ExhaustionHealingFactor"));
        assert!(live.contains(&"WoundDamageFactor"));
        assert!(live.contains(&"WoundHealingFactor"));
        // C-SS-MALE-HEAL / C-SS-MORE-BATCH3
        assert!(live.contains(&"ExhaustionHealingForMaleFaktor"));
        assert!(live.contains(&"MaxMovementQuadJumpDistanceBeforeForce"));
        assert!(live.contains(&"FoodRestoreFactorWhileFeeding"));
        assert!(live.contains(&"MaxHasEatenForNextGeneration"));
        assert!(live.contains(&"CombatExhaustionCostPerAttack"));
        assert!(live.contains(&"MinAgeToEat"));
        assert!(live.contains(&"MaxChildAgeForBreastFeeding"));
        assert!(live.contains(&"AllyConsideredClose"));
        assert!(live.contains(&"MinMovementAgeInSec"));
        assert!(live.contains(&"HasEatenReductionForNextGeneration"));
        // C-SS-TEMP-HEAL
        assert!(live.contains(&"TemperatureHitsDamageFactor"));
        assert!(live.contains(&"TemperatureExhaustionDamageFactor"));
        // WALLET-COINS
        assert!(live.contains(&"CoinsOnWoundingFactor"));
        // C-SS-MORE-BATCH4
        assert!(live.contains(&"CursedReceiveDamageFactor"));
        assert!(live.contains(&"CursedMakeDamageFactor"));
        assert!(live.contains(&"PickupBabyMaxDistance"));
        assert!(live.contains(&"InheritCoinsFactor"));
        assert!(live.contains(&"MinAgeFertile"));
        assert!(live.contains(&"MaxAgeFertile"));
        // C-SS-MORE-BATCH5
        assert!(live.contains(&"WeaponCoolDownFactor"));
        assert!(live.contains(&"WeaponCoolDownFactorIfWounding"));
        assert!(live.contains(&"CloseEnemyWithWeaponSpeedFactor"));
        assert!(live.contains(&"ExhaustionOnJump"));
        assert!(live.contains(&"HungryWorkHeat"));
        assert!(live.contains(&"AISpeedFactorSerf"));
        assert!(live.contains(&"AISpeedFactorCommoner"));
        assert!(live.contains(&"AISpeedFactorNoble"));
        // SETTINGS-LONG-TAIL
        assert!(live.contains(&"StartingEveAge"));
        assert!(live.contains(&"EveOrAdamBirthChance"));
        assert!(live.contains(&"SpawnAiAsEve"));
        assert!(live.contains(&"ObjDecayChance"));
        assert!(live.contains(&"FloorDecayChance"));
        assert!(live.contains(&"CursedGraveTime"));
        assert!(live.contains(&"AnimalDecayFactor"));
        assert!(live.contains(&"ScoreFactor"));
        assert!(live.contains(&"DisplayScoreFactor"));
        assert!(live.contains(&"DisplayScoreOn"));
        assert!(live.contains(&"MaxCoinsPerChest"));
        assert!(live.contains(&"MaxCoinsPerPouch"));
        assert!(live.contains(&"ChanceForFemaleChild"));
        assert!(live.contains(&"ChanceForOtherChildColor"));
        assert!(live.contains(&"ChanceForOtherChildColorIfCloseToWrongSpecialBiome"));
        assert!(live.contains(&"LittleKidsPerMother"));
        assert!(live.contains(&"NewChildExhaustionForMother"));
        assert!(live.contains(&"AiMotherBirthMaliForHumanChild"));
        assert!(live.contains(&"HumanMotherBirthMaliForAiChild"));
        assert!(live.contains(&"SpwanAtLastDead"));
        assert!(live.contains(&"TemperatureOwnTileRate"));
        assert!(live.contains(&"TemperatureBalanceRate"));
        assert!(live.contains(&"TemperatureLocalHeatFactor"));
        assert!(live.contains(&"AverageSeasonTemperatureImpact"));
        assert!(live.contains(&"SendMoveEveryXTicks"));
        assert!(live.contains(&"MaxDistanceToBeConsideredAsCoseForMovement"));
        assert!(live.contains(&"MaxDistanceToBeConsideredAsCloseForSayAi"));
        assert!(live.contains(&"AIFoodUseFactorSerf"));
        assert!(live.contains(&"AIFoodUseFactorCommoner"));
        assert!(live.contains(&"AIFoodUseFactorNoble"));
        assert!(live.contains(&"EveFoodUseFactor"));
        assert!(live.contains(&"EveDamageFactor"));
        assert!(live.contains(&"TargetWoundedDamageFactor"));
        assert!(live.contains(&"MaleDamageFactor"));
        assert!(live.contains(&"AnimalDamageFactor"));
        assert!(live.contains(&"AnimalDamageFactorInWinter"));
        assert!(live.contains(&"AnimalDamageFactorIfAttacked"));
        assert!(live.contains(&"WeaponDamageFactor"));
        assert!(live.contains(&"GraveBlockingDistance"));
        assert!(live.contains(&"MaxPlayersBeforeActivatingGraveCurse"));
        assert!(live.contains(&"MaxPlayersBeforeForbidTouchGrave"));
        assert!(live.contains(&"CombatAngryTimeBeforeAttack"));
        assert!(live.contains(&"AiTimeToWaitIfCraftingFailed"));
        assert!(live.contains(&"AiMemoryMaxEntries"));
        assert!(live.contains(&"AiChatMemoryMaxEntries"));
        assert!(live.contains(&"AiMaxSearchRadius"));
        assert!(live.contains(&"AiMaxSearchIncrement"));
        assert!(live.contains(&"AiIgnoreTimeTransitionsLongerThen"));
        assert!(live.contains(&"AlternativeOutcomePercentIncreasePerHit"));
        assert!(live.contains(&"AlternativeOutcomeHitsDecreaseOnSucess"));
        assert!(live.contains(&"FortificationCosePerHit"));
        assert!(live.contains(&"ReduceAgeNeededToPickupObjects"));
        assert!(live.contains(&"ChanceThatAnimalsCanPassBlockingBiome"));
        assert!(live.contains(&"chancePreferredBiome"));
        assert!(live.contains(&"CloseGraveSpeedMali"));
        assert!(live.contains(&"TemperatureSpeedImpact"));
        assert!(live.contains(&"MinSpeedReductionPerContainedObj"));
        assert!(live.contains(&"LovedFoodUseChance"));
        assert!(live.contains(&"MaxAgeForAllowingClothAndPrickupFromOthers"));
        assert!(live.contains(&"MaxAgeForAllowingDie"));
        assert!(live.contains(&"PrestigeCostForDie"));
        assert!(live.contains(&"StartingFamilyName"));
        assert!(live.contains(&"StartingName"));
        assert!(live.contains(&"FoundFamilyNeededPrestige"));
        assert!(live.contains(&"FoundFamilyCost"));
        assert!(live.contains(&"FoundFamilyNeededFollowers"));
        assert!(live.contains(&"FoundFamilyBreakAllianceChance"));
        assert!(live.contains(&"PickupExhaustionGain"));
        assert!(live.contains(&"PickupFeedingFoodRestore"));
        assert!(live.contains(&"DeathWithFoodStoreMax"));
        assert!(live.contains(&"FoodStoreMaxReductionWhileStarvingToDeath"));
        assert!(live.contains(&"TemperatureReductionPerDrinking"));
        assert!(live.contains(&"MaxStoredWater"));
        assert!(live.contains(&"MaxJumpsPerTenSec"));
        assert!(live.contains(&"TemperatureImpactPerSec"));
        assert!(live.contains(&"TemperatureImpactPerSecIfGood"));
        assert!(live.contains(&"TemperatureInWaterFactor"));
        assert!(live.contains(&"TemperatureImpactBelow"));
        assert!(live.contains(&"TemperatureImpactColorFactor"));
        assert!(live.contains(&"AllowEatingOrFeedingIfIll"));
        assert!(live.contains(&"ResistanceAgainstFeverForEatingMushrooms"));
        assert!(live.contains(&"ExhaustionYellowFeverPerSec"));
        assert!(live.contains(&"MinHealthFoodStoreMaxFactor"));
        assert!(live.contains(&"MaxHealthFoodStoreMaxFactor"));
        assert!(live.contains(&"MinHealthAgingFactor"));
        assert!(live.contains(&"MaxHealthAgingFactor"));
        assert!(live.contains(&"MinHealthPerYear"));
        assert!(live.contains(&"MaxAge"));
        assert!(live.contains(&"AnimalDeadlyDistanceFactor"));
        assert!(live.contains(&"ChanceForAnimalDyingFactorIfInLovedBiome"));
        assert!(live.contains(&"OffspringFactorIfAnimalPopIsLow"));
        assert!(live.contains(&"MaxOffspringFactor"));
        assert!(live.contains(&"OffspringFactorLowAnimalPopulationBelow"));
        assert!(live.contains(&"ObjDecayFactorForFood"));
        assert!(live.contains(&"ObjDecayFactorForClothing"));
        assert!(live.contains(&"ObjDecayFactorForWalls"));
        assert!(live.contains(&"ObjDecayFactorPerTechLevel"));
        assert!(live.contains(&"DecayFactorInDeepWater"));
        assert!(live.contains(&"DecayFactorInMountain"));
        assert!(live.contains(&"DecayFactorInWalkableWater"));
        assert!(live.contains(&"DecayFactorInJungle"));
        assert!(live.contains(&"DecayFactorInSwamp"));
        assert!(live.contains(&"ObjRespawnChance"));
        assert!(live.contains(&"GrowBackPlantsIncreaseIfLowPopulation"));
        let residual = module_const_critical_names();
        assert!(!residual.contains(&"GrownUpFoodStoreMax"));
        assert!(!residual.contains(&"FoodFactor"));
        assert!(!residual.contains(&"ExhaustionHealingFactor"));
        assert!(!residual.contains(&"ExhaustionHealingForMaleFaktor"));
        assert!(!residual.contains(&"TemperatureHitsDamageFactor"));
        assert!(!residual.contains(&"TemperatureExhaustionDamageFactor"));
        assert!(!residual.contains(&"CursedReceiveDamageFactor"));
        assert!(!residual.contains(&"MinAgeFertile"));
        assert!(!residual.contains(&"WeaponCoolDownFactor"));
        assert!(!residual.contains(&"HungryWorkHeat"));
        assert!(!residual.contains(&"AISpeedFactorSerf"));
        assert!(!residual.contains(&"StartingEveAge"));
        assert!(!residual.contains(&"EveOrAdamBirthChance"));
        assert!(!residual.contains(&"SpawnAiAsEve"));
        assert!(!residual.contains(&"ObjDecayChance"));
        assert!(!residual.contains(&"FloorDecayChance"));
        assert!(!residual.contains(&"CursedGraveTime"));
        assert!(!residual.contains(&"AnimalDecayFactor"));
        assert!(!residual.contains(&"ScoreFactor"));
        assert!(!residual.contains(&"DisplayScoreOn"));
        assert!(!residual.contains(&"MaxCoinsPerChest"));
        assert!(!residual.contains(&"ChanceForFemaleChild"));
        assert!(!residual.contains(&"SpwanAtLastDead"));
        assert!(!residual.contains(&"TemperatureOwnTileRate"));
        assert!(!residual.contains(&"AverageSeasonTemperatureImpact"));
        assert!(!residual.contains(&"SendMoveEveryXTicks"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCoseForMovement"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCloseForSayAi"));
        assert!(!residual.contains(&"SpeedWithBothShoes"));
        assert!(!residual.contains(&"AgingFactorWhileStarvingToDeath"));
        assert!(!residual.contains(&"GrownUpAge"));
        assert!(!residual.contains(&"FoodUseChildFaktor"));
        assert!(!residual.contains(&"AIFoodUseFactorSerf"));
        assert!(!residual.contains(&"AIFoodUseFactorCommoner"));
        assert!(!residual.contains(&"AIFoodUseFactorNoble"));
        assert!(!residual.contains(&"EveFoodUseFactor"));
        assert!(!residual.contains(&"EveDamageFactor"));
        assert!(!residual.contains(&"TargetWoundedDamageFactor"));
        assert!(!residual.contains(&"MaleDamageFactor"));
        assert!(!residual.contains(&"AnimalDamageFactor"));
        assert!(!residual.contains(&"AnimalDamageFactorInWinter"));
        assert!(!residual.contains(&"AnimalDamageFactorIfAttacked"));
        assert!(!residual.contains(&"WeaponDamageFactor"));
        assert!(!residual.contains(&"GraveBlockingDistance"));
        assert!(!residual.contains(&"MaxPlayersBeforeActivatingGraveCurse"));
        assert!(!residual.contains(&"MaxPlayersBeforeForbidTouchGrave"));
        assert!(!residual.contains(&"CombatAngryTimeBeforeAttack"));
        assert!(!residual.contains(&"AiTimeToWaitIfCraftingFailed"));
        assert!(!residual.contains(&"AiMemoryMaxEntries"));
        assert!(!residual.contains(&"AiChatMemoryMaxEntries"));
        assert!(!residual.contains(&"AiMaxSearchRadius"));
        assert!(!residual.contains(&"AiMaxSearchIncrement"));
        assert!(!residual.contains(&"AiIgnoreTimeTransitionsLongerThen"));
        assert!(!residual.contains(&"AlternativeOutcomePercentIncreasePerHit"));
        assert!(!residual.contains(&"AlternativeOutcomeHitsDecreaseOnSucess"));
        assert!(!residual.contains(&"FortificationCosePerHit"));
        assert!(!residual.contains(&"ReduceAgeNeededToPickupObjects"));
        assert!(!residual.contains(&"ChanceThatAnimalsCanPassBlockingBiome"));
        assert!(!residual.contains(&"chancePreferredBiome"));
        assert!(!residual.contains(&"CloseGraveSpeedMali"));
        assert!(!residual.contains(&"TemperatureSpeedImpact"));
        assert!(!residual.contains(&"MaxAgeForAllowingDie"));
        assert!(!residual.contains(&"PrestigeCostForDie"));
        assert!(!residual.contains(&"StartingFamilyName"));
        assert!(!residual.contains(&"StartingName"));
        assert!(!residual.contains(&"FoundFamilyNeededPrestige"));
        assert!(!residual.contains(&"FoundFamilyCost"));
        assert!(!residual.contains(&"FoundFamilyNeededFollowers"));
        assert!(!residual.contains(&"FoundFamilyBreakAllianceChance"));
        assert!(!residual.contains(&"ObjRespawnChance"));
        assert!(!residual.contains(&"GrowBackPlantsIncreaseIfLowPopulation"));
    }
}
