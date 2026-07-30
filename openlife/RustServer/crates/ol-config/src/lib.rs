//! Server configuration loaded from TOML.
//!
//! Haxe: `openlife/settings/ServerSettings.hx` (`readFromFile` / `writeToFile`) +
//! `TimeHelper.ReadServerSettings` hot-reload every ~200 ticks.

#![forbid(unsafe_code)]

mod field_map;

pub use field_map::{
    find_critical, gameplay_defaults, is_ai_ignored_floor_id, is_door_id, live_critical_names,
    module_const_critical_names, secret_omit_names, FieldEntry, SettingsHome, AI_IGNORED_FLOOR_IDS,
    CRITICAL_FIELD_MAP, DOOR_IDS,
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
            npc_min: 3,
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
    pub eternal_winter: bool,
    /// Sim seconds per season (from `season_duration_years * HAXE_YEAR_SECS`).
    pub season_length_secs: f32,
    pub npc_enabled: bool,
    pub npc_min: u32,
    pub npc_max: u32,
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
            eternal_winter: self.eternal_winter,
            season_length_secs: self.season_length_secs(),
            npc_enabled: self.npc_enabled,
            npc_min: self.npc_min,
            npc_max: self.npc_max.max(self.npc_min),
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
        ]
    }
}

#[inline]
fn sanitize_nonneg_or(v: f32, default: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
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
        assert_eq!(c.npc_min, 3);
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
            eternal_winter: true,
            season_duration_years: 1.0,
            npc_enabled: false,
            npc_min: 1,
            npc_max: 2,
            ai_think_period_ticks: 3,
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
    }
}
