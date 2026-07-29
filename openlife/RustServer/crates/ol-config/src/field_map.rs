//! Haxe `ServerSettings` field inventory (SETTINGS-FIELD-MAP / C-SS-FULL-TABLE).
//!
//! Haxe `writeToFile` / `readFromFile` RTTI-reflect every public static onto
//! `SaveFiles/ServerSettings.txt`. Rust maps a curated critical set into
//! `server.toml` + [`crate::LiveSettings`], documents the rest as module consts
//! or intentional omit (secrets / debug / deferred).
//!
//! // Haxe: ServerSettings.writeToFile / readFromFile
//! Chunks: SETTINGS-FIELD-MAP · C-SS-FULL-TABLE / settings_long_tail

/// Where a Haxe `ServerSettings` static lives in the Rust port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsHome {
    /// Hot-reloadable via `server.toml` → [`crate::LiveSettings`] → `apply_live_settings`.
    Live,
    /// Present on [`crate::ServerConfig`] / `server.toml` but boot-only (restart needed).
    BootToml,
    /// Module-level `const` in ol-sim / ol-content (Haxe default; not TOML yet).
    ModuleConst,
    /// Must never dump to settings files (API keys, secrets).
    SecretOmit,
    /// Documented deferral (TODO in Haxe or product-later).
    Deferred,
    /// Debug / trace toggles — not product config.
    DebugOnly,
}

/// One inventory row: Haxe static name → Rust home.
#[derive(Debug, Clone, Copy)]
pub struct FieldEntry {
    pub haxe_name: &'static str,
    /// Rust path hint (crate::module::SYMBOL or server.toml key).
    pub rust_path: &'static str,
    pub home: SettingsHome,
}

// ---------------------------------------------------------------------------
// Array tables (Haxe static Array<Int>) — not TOML-serialized yet
// ---------------------------------------------------------------------------

/// Haxe `ServerSettings.DoorIds` (pine + wooden doors open/closed pairs).
// Haxe: ServerSettings.DoorIds
pub const DOOR_IDS: &[i32] = &[115, 116, 117, 119, 876, 877, 879, 878];

/// Haxe `ServerSettings.AiIgnoredFloorIds` (Bear Skin Rug variants).
// Haxe: ServerSettings.AiIgnoredFloorIds
pub const AI_IGNORED_FLOOR_IDS: &[i32] = &[656, 888];

/// Haxe `ServerSettings.IsDoor`.
// Haxe: ServerSettings.IsDoor
#[inline]
pub fn is_door_id(item_id: i32) -> bool {
    DOOR_IDS.contains(&item_id)
}

/// Haxe floor ids AI should ignore for count/drop/pickup-non-food.
// Haxe: ServerSettings.AiIgnoredFloorIds
#[inline]
pub fn is_ai_ignored_floor_id(floor_id: i32) -> bool {
    AI_IGNORED_FLOOR_IDS.contains(&floor_id)
}

// ---------------------------------------------------------------------------
// Gameplay knobs (critical Haxe statics → LiveSettings / ServerConfig)
// ---------------------------------------------------------------------------

/// Default Haxe gameplay values promoted into config (SETTINGS-FIELD-MAP + C-SS-FULL-TABLE).
// Haxe: ServerSettings food/heal/move/yum/animal/score / FoodFactor bands
pub mod gameplay_defaults {
    pub const FOOD_USE_PER_SECOND: f32 = 0.10;
    pub const HEALING_PER_SECOND: f32 = 0.10;
    pub const AGEING_SECONDS_PER_YEAR: f32 = 60.0;
    pub const INITIAL_PLAYER_MOVE_SPEED: f32 = 3.75;
    pub const SPEED_FACTOR: f32 = 1.0;
    pub const YUM_BONUS: f32 = 5.0;
    pub const CHANCE_FOR_OFFSPRING: f32 = 0.00005;
    pub const CHANCE_FOR_ANIMAL_DYING: f32 = 0.00005;
    pub const HUNGRY_WORK_COST: f32 = 5.0;
    pub const BIRTH_PRESTIGE_FACTOR: f32 = 0.4;
    /// Haxe typo `AllyStrenghTooLowForPickup` (0 = gate disabled).
    pub const ALLY_STRENGTH_TOO_LOW_FOR_PICKUP: f32 = 0.0;
    /// Haxe `TimeConfirmNewFollower` — delayed I FOLLOW confirm seconds.
    // Haxe: ServerSettings.TimeConfirmNewFollower = 15
    // FOLLOW-HIRE-DELAY
    pub const TIME_CONFIRM_NEW_FOLLOWER: f32 = 15.0;
    /// Haxe `HireCost` base coins.
    // Haxe: ServerSettings.HireCost = 10
    pub const HIRE_COST: f32 = 10.0;
    /// Haxe `HireCostIncreasePerPerson`.
    // Haxe: ServerSettings.HireCostIncreasePerPerson = 10
    pub const HIRE_COST_INCREASE_PER_PERSON: f32 = 10.0;
    /// Haxe `AutoFollowPlayer` — AI auto-acquire closest human when sticky empty.
    // Haxe: ServerSettings.AutoFollowPlayer = false
    // AI-FOLLOW-ACQUIRE
    pub const AUTO_FOLLOW_PLAYER: bool = false;
    /// Haxe `PrestigeCostPerDamageForAlly`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly = 1
    // PRESTIGE-ALLY-COST
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_ALLY: f32 = 1.0;
    /// Haxe `PrestigeCostPerDamageForChild`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForChild = 5
    // C-SS-MORE
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_CHILD: f32 = 5.0;
    /// Haxe `PrestigeCostPerDamageForElderly`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForElderly = 1
    // C-SS-MORE
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_ELDERLY: f32 = 1.0;
    /// Haxe `PrestigeCostPerDamageForCloseRelatives`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForCloseRelatives = 0.5
    // C-SS-MORE
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_CLOSE_RELATIVES: f32 = 0.5;
    /// Haxe `PrestigeCostPerDamageForWomenWithoutWeapon`.
    // Haxe: ServerSettings.PrestigeCostPerDamageForWomenWithoutWeapon = 0.5
    // C-SS-MORE
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_WOMEN_WITHOUT_WEAPON: f32 = 0.5;

    // --- C-SS-FULL-TABLE / settings_long_tail: FoodFactor + eat bands + YumFoodRestore ---
    /// Haxe `ServerSettings.FoodFactor` — global fill scale in compute_eat.
    // Haxe: ServerSettings.FoodFactor = 1
    pub const FOOD_FACTOR: f32 = 1.0;
    /// Haxe `FoodFactorEatenMoreThanEightPercent`.
    pub const FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT: f32 = 0.8;
    /// Haxe `FoodFactorEatenMoreThanTenPercent`.
    pub const FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT: f32 = 0.5;
    /// Haxe `FoodFactorEatenLessThanFivePercent`.
    pub const FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT: f32 = 1.5;
    /// Haxe `FoodFactorEatenLessThanThreePercent`.
    pub const FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT: f32 = 2.0;
    /// Haxe `FoodFactorEatenLessThanOnePercent`.
    pub const FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT: f32 = 2.5;
    /// Haxe `YumFoodRestore` — random other-food hasEaten restore per eat.
    // Haxe: ServerSettings.YumFoodRestore = 0.8
    pub const YUM_FOOD_RESTORE: f32 = 0.8;
    /// Haxe `LovedFoodRestore`.
    // Haxe: ServerSettings.LovedFoodRestore = 0.1
    pub const LOVED_FOOD_RESTORE: f32 = 0.1;
    /// Haxe `YumNewCravingChance`.
    // Haxe: ServerSettings.YumNewCravingChance = 0.2
    pub const YUM_NEW_CRAVING_CHANCE: f32 = 0.2;
    /// Haxe `FoodReductionPerEating`.
    // Haxe: ServerSettings.FoodReductionPerEating = 1
    pub const FOOD_REDUCTION_PER_EATING: f32 = 1.0;
    /// Haxe `FoodReductionFaktorForEatingMeh`.
    // Haxe: ServerSettings.FoodReductionFaktorForEatingMeh = 0.2
    pub const FOOD_REDUCTION_FAKTOR_FOR_EATING_MEH: f32 = 0.2;
    /// Haxe `HealthLostWhenEatingMeh`.
    // Haxe: ServerSettings.HealthLostWhenEatingMeh = 0.5
    pub const HEALTH_LOST_WHEN_EATING_MEH: f32 = 0.5;
    /// Haxe `HealthLostWhenEatingSuperMeh`.
    // Haxe: ServerSettings.HealthLostWhenEatingSuperMeh = 2
    pub const HEALTH_LOST_WHEN_EATING_SUPER_MEH: f32 = 2.0;

    // --- C-SS-TAIL-KNOBS / settings_knobs ---
    /// Haxe `FoodReductionFaktorForEatingHighQuailitFood` (Haxe typo Quailit).
    // Haxe: ServerSettings.FoodReductionFaktorForEatingHighQuailitFood = 0.8
    // C-SS-TAIL-KNOBS
    pub const FOOD_REDUCTION_FAKTOR_FOR_EATING_HIGH_QUALITY: f32 = 0.8;
    /// Haxe `GrownUpFoodStoreMax`.
    // Haxe: ServerSettings.GrownUpFoodStoreMax = 20
    // C-SS-TAIL-KNOBS
    pub const GROWN_UP_FOOD_STORE_MAX: f32 = 20.0;
    /// Haxe `NewBornFoodStoreMax`.
    // Haxe: ServerSettings.NewBornFoodStoreMax = 4
    // C-SS-AGE-FOOD
    pub const NEW_BORN_FOOD_STORE_MAX: f32 = 4.0;
    /// Haxe `OldAgeFoodStoreMax`.
    // Haxe: ServerSettings.OldAgeFoodStoreMax = 10
    // C-SS-AGE-FOOD
    pub const OLD_AGE_FOOD_STORE_MAX: f32 = 10.0;
    /// Haxe `MinBiomeSpeedFactor`.
    // Haxe: ServerSettings.MinBiomeSpeedFactor = 0.2
    // C-SS-TAIL-KNOBS
    pub const MIN_BIOME_SPEED_FACTOR: f32 = 0.2;
    /// Haxe `HitpointsSpeedFactor` (0 = disable hitpoints speed influence).
    // Haxe: ServerSettings.HitpointsSpeedFactor = 3
    // C-SS-TAIL-KNOBS
    pub const HITPOINTS_SPEED_FACTOR: f32 = 3.0;
    /// Haxe `CombatReputationRestorePerYear`.
    // Haxe: ServerSettings.CombatReputationRestorePerYear = 2
    // C-SS-TAIL-KNOBS
    pub const COMBAT_REPUTATION_RESTORE_PER_YEAR: f32 = 2.0;

    // --- C-SS-MORE-KNOBS / settings_batch2 ---
    /// Haxe `ExhaustionHealingFactor`.
    // Haxe: ServerSettings.ExhaustionHealingFactor = 1.5
    // C-SS-MORE-KNOBS
    pub const EXHAUSTION_HEALING_FACTOR: f32 = 1.5;
    /// Haxe `WoundDamageFactor`.
    // Haxe: ServerSettings.WoundDamageFactor = 1
    // C-SS-MORE-KNOBS
    pub const WOUND_DAMAGE_FACTOR: f32 = 1.0;
    /// Haxe `WoundHealingFactor`.
    // Haxe: ServerSettings.WoundHealingFactor = 1
    // C-SS-WOUND-HEAL
    pub const WOUND_HEALING_FACTOR: f32 = 1.0;
    /// Haxe `ExhaustionHealingForMaleFaktor` (Haxe typo Faktor) — multiplies male exhaustion recovery only.
    // Haxe: ServerSettings.ExhaustionHealingForMaleFaktor = 1.2
    // C-SS-MALE-HEAL
    pub const EXHAUSTION_HEALING_FOR_MALE_FACTOR: f32 = 1.2;
    /// Haxe `TemperatureHitsDamageFactor`.
    // Haxe: ServerSettings.TemperatureHitsDamageFactor = 0.5
    // C-SS-TEMP-HEAL
    pub const TEMPERATURE_HITS_DAMAGE_FACTOR: f32 = 0.5;
    /// Haxe `TemperatureExhaustionDamageFactor`.
    // Haxe: ServerSettings.TemperatureExhaustionDamageFactor = 0.2
    // C-SS-TEMP-HEAL
    pub const TEMPERATURE_EXHAUSTION_DAMAGE_FACTOR: f32 = 0.2;
    /// Haxe `MaxMovementQuadJumpDistanceBeforeForce` (squared distance gate).
    // Haxe: ServerSettings.MaxMovementQuadJumpDistanceBeforeForce = 5
    // C-SS-MORE-KNOBS
    pub const MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE: f32 = 5.0;
    /// Haxe `FoodRestoreFactorWhileFeeding`.
    // Haxe: ServerSettings.FoodRestoreFactorWhileFeeding = 10
    // C-SS-MORE-KNOBS
    pub const FOOD_RESTORE_FACTOR_WHILE_FEEDING: f32 = 10.0;
    /// Haxe `MaxHasEatenForNextGeneration`.
    // Haxe: ServerSettings.MaxHasEatenForNextGeneration = 4
    // C-SS-MORE-KNOBS
    pub const MAX_HAS_EATEN_FOR_NEXT_GENERATION: f32 = 4.0;
    /// Haxe `HasEatenReductionForNextGeneration`.
    // Haxe: ServerSettings.HasEatenReductionForNextGeneration = 1
    // C-SS-MORE-KNOBS
    pub const HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION: f32 = 1.0;
    /// Haxe `CoinsOnWoundingFactor` — fraction of target coins stolen on wound/kill (+1 floor).
    // Haxe: ServerSettings.CoinsOnWoundingFactor = 0.5
    // WALLET-COINS
    pub const COINS_ON_WOUNDING_FACTOR: f32 = 0.5;

    // --- C-SS-MORE-BATCH3 / settings_batch3 ---
    /// Haxe `CombatExhaustionCostPerAttack`.
    // Haxe: ServerSettings.CombatExhaustionCostPerAttack = 0.1
    // C-SS-MORE-BATCH3
    pub const COMBAT_EXHAUSTION_COST_PER_ATTACK: f32 = 0.1;
    /// Haxe `MinAgeToEat` (years).
    // Haxe: ServerSettings.MinAgeToEat = 3
    // C-SS-MORE-BATCH3
    pub const MIN_AGE_TO_EAT: f32 = 3.0;
    /// Haxe `MaxChildAgeForBreastFeeding` (years).
    // Haxe: ServerSettings.MaxChildAgeForBreastFeeding = 6
    // C-SS-MORE-BATCH3
    pub const MAX_CHILD_AGE_FOR_BREAST_FEEDING: f32 = 6.0;
    /// Haxe `AllyConsideredClose` (tile radius for ally strength / anger).
    // Haxe: ServerSettings.AllyConsideredClose = 5
    // C-SS-MORE-BATCH3
    pub const ALLY_CONSIDERED_CLOSE: f32 = 5.0;
    /// Haxe `MinMovementAgeInSec` — reject MOVE when `age * 60 <` this.
    // Haxe: ServerSettings.MinMovementAgeInSec = 14
    // C-SS-MORE-BATCH3
    pub const MIN_MOVEMENT_AGE_IN_SEC: f32 = 14.0;

    // --- C-SS-MORE-BATCH4 / settings_batch4 ---
    /// Haxe `CursedReceiveDamageFactor` — target cursed takes more damage.
    // Haxe: ServerSettings.CursedReceiveDamageFactor = 1.2
    // C-SS-MORE-BATCH4
    pub const CURSED_RECEIVE_DAMAGE_FACTOR: f32 = 1.2;
    /// Haxe `CursedMakeDamageFactor` — cursed attacker deals less damage.
    // Haxe: ServerSettings.CursedMakeDamageFactor = 0.5
    // C-SS-MORE-BATCH4
    pub const CURSED_MAKE_DAMAGE_FACTOR: f32 = 0.5;
    /// Haxe `PickupBabyMaxDistance` — euclidean max for doBaby/BABY.
    // Haxe: ServerSettings.PickupBabyMaxDistance = 1.9
    // C-SS-MORE-BATCH4
    pub const PICKUP_BABY_MAX_DISTANCE: f32 = 1.9;
    /// Haxe `InheritCoinsFactor` — fraction of wallet credited as coinsInherited.
    // Haxe: ServerSettings.InheritCoinsFactor = 0.8
    // C-SS-MORE-BATCH4
    pub const INHERIT_COINS_FACTOR: f32 = 0.8;
    /// Haxe `MinAgeFertile` (years). Client may misbehave if min &lt; 14.
    // Haxe: ServerSettings.MinAgeFertile = 14 // TODO only make lower then 14 if client allows it
    // C-SS-MORE-BATCH4
    pub const MIN_AGE_FERTILE: f32 = 14.0;
    /// Haxe `MaxAgeFertile` (years, inclusive).
    // Haxe: ServerSettings.MaxAgeFertile = 42
    // C-SS-MORE-BATCH4
    pub const MAX_AGE_FERTILE: f32 = 42.0;

    // --- C-SS-MORE-BATCH5 / settings_batch5 ---
    /// Haxe `WeaponCoolDownFactor` — normal bloody cool-down mult.
    // Haxe: ServerSettings.WeaponCoolDownFactor = 0.5
    // C-SS-MORE-BATCH5
    pub const WEAPON_COOLDOWN_FACTOR: f32 = 0.5;
    /// Haxe `WeaponCoolDownFactorIfWounding` — first wound / kill cool-down mult.
    // Haxe: ServerSettings.WeaponCoolDownFactorIfWounding = 5
    // C-SS-MORE-BATCH5
    pub const WEAPON_COOLDOWN_FACTOR_IF_WOUNDING: f32 = 5.0;
    /// Haxe `CloseEnemyWithWeaponSpeedFactor`.
    // Haxe: ServerSettings.CloseEnemyWithWeaponSpeedFactor = 0.8
    // C-SS-MORE-BATCH5
    pub const CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR: f32 = 0.8;
    /// Haxe `ExhaustionOnJump` (× effective quadDist, humans only).
    // Haxe: ServerSettings.ExhaustionOnJump = 0.05
    // C-SS-MORE-BATCH5
    pub const EXHAUSTION_ON_JUMP: f32 = 0.05;
    /// Haxe `HungryWorkHeat` — default heat when transition temperature &lt; 0.
    // Haxe: ServerSettings.HungryWorkHeat = 0.002 // per food used
    // C-SS-MORE-BATCH5
    pub const HUNGRY_WORK_HEAT: f32 = 0.002;
    /// Haxe `AISpeedFactorSerf`.
    // Haxe: ServerSettings.AISpeedFactorSerf = 0.8
    // C-SS-MORE-BATCH5
    pub const AI_SPEED_FACTOR_SERF: f32 = 0.8;
    /// Haxe `AISpeedFactorCommoner`.
    // Haxe: ServerSettings.AISpeedFactorCommoner = 0.9
    // C-SS-MORE-BATCH5
    pub const AI_SPEED_FACTOR_COMMONER: f32 = 0.9;
    /// Haxe `AISpeedFactorNoble` (+ King/Emperor).
    // Haxe: ServerSettings.AISpeedFactorNoble = 1
    // C-SS-MORE-BATCH5
    pub const AI_SPEED_FACTOR_NOBLE: f32 = 1.0;
}

// ---------------------------------------------------------------------------
// Critical field inventory (snapshot; expand as more knobs go Live/BootToml)
// ---------------------------------------------------------------------------

/// Critical Haxe `ServerSettings` statics used by playable core + AI/combat.
///
/// Not every Haxe static (~345): debug/trace, path filenames, LLM secrets, and
/// rare patches stay ModuleConst / SecretOmit / DebugOnly until a dedicated chunk.
/// C-SS-FULL-TABLE expands the inventory table + promotes FoodFactor bands Live.
pub const CRITICAL_FIELD_MAP: &[FieldEntry] = &[
    // --- already Live (CONFIG-SETTINGS / LOCKPICK / NPC) ---
    FieldEntry {
        haxe_name: "EternalWinter",
        rust_path: "server.toml eternal_winter / LiveSettings",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SeasonDuration",
        rust_path: "server.toml season_duration_years / LiveSettings.season_length_secs",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LockpickSucessChance",
        rust_path: "server.toml lockpick_success_chance",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LockpickFailChance",
        rust_path: "server.toml lockpick_fail_chance",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LockpickExhaustionCost",
        rust_path: "server.toml lockpick_exhaustion_cost",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LockpickCoinCost",
        rust_path: "server.toml lockpick_coin_cost",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "NumberOfAis",
        rust_path: "server.toml npc_max",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinNumberOfAis",
        rust_path: "server.toml npc_min",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxPlayers",
        rust_path: "server.toml max_players",
        home: SettingsHome::BootToml,
    },
    FieldEntry {
        haxe_name: "MapFileName",
        rust_path: "server.toml map_png_path",
        home: SettingsHome::BootToml,
    },
    FieldEntry {
        haxe_name: "SaveDirectory",
        rust_path: "server.toml save_directory",
        home: SettingsHome::BootToml,
    },
    FieldEntry {
        haxe_name: "WebServerPort",
        rust_path: "server.toml web_port",
        home: SettingsHome::BootToml,
    },
    FieldEntry {
        haxe_name: "VerifyIfOholAccount",
        rust_path: "server.toml verify_ohol_ticket",
        home: SettingsHome::BootToml,
    },
    FieldEntry {
        haxe_name: "GenerateMapNew",
        rust_path: "server.toml force_regenerate_map",
        home: SettingsHome::BootToml,
    },
    // --- SETTINGS-FIELD-MAP batch: now Live gameplay knobs ---
    FieldEntry {
        haxe_name: "FoodUsePerSecond",
        rust_path: "server.toml food_use_per_second / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HealingPerSecond",
        rust_path: "server.toml healing_per_second / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AgeingSecondsPerYear",
        rust_path: "server.toml ageing_seconds_per_year / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "InitialPlayerMoveSpeed",
        rust_path: "server.toml initial_player_move_speed / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SpeedFactor",
        rust_path: "server.toml speed_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "YumBonus",
        rust_path: "server.toml yum_bonus / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForOffspring",
        rust_path: "server.toml chance_for_offspring / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForAnimalDying",
        rust_path: "server.toml chance_for_animal_dying / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HungryWorkCost",
        rust_path: "server.toml hungry_work_cost / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "BirthPrestigeFactor",
        rust_path: "server.toml birth_prestige_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AllyStrenghTooLowForPickup",
        rust_path: "server.toml ally_strength_too_low_for_pickup / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // FOLLOW-HIRE-DELAY: delayed follow confirm + hire cost live knobs
    FieldEntry {
        haxe_name: "TimeConfirmNewFollower",
        rust_path: "server.toml time_confirm_new_follower / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HireCost",
        rust_path: "server.toml hire_cost / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HireCostIncreasePerPerson",
        rust_path: "server.toml hire_cost_increase_per_person / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // AI-FOLLOW-ACQUIRE: empty-sticky AutoFollowPlayer closest human
    FieldEntry {
        haxe_name: "AutoFollowPlayer",
        rust_path: "server.toml auto_follow_player / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // PRESTIGE-ALLY-COST + C-SS-MORE PrestigeCost* Live
    FieldEntry {
        haxe_name: "PrestigeCostPerDamageForAlly",
        rust_path: "server.toml prestige_cost_per_damage_for_ally / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PrestigeCostPerDamageForChild",
        rust_path: "server.toml prestige_cost_per_damage_for_child / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PrestigeCostPerDamageForElderly",
        rust_path: "server.toml prestige_cost_per_damage_for_elderly / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PrestigeCostPerDamageForCloseRelatives",
        rust_path: "server.toml prestige_cost_per_damage_for_close_relatives / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PrestigeCostPerDamageForWomenWithoutWeapon",
        rust_path: "server.toml prestige_cost_per_damage_for_women_without_weapon / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-FULL-TABLE / settings_long_tail: FoodFactor bands + YumFoodRestore Live ---
    FieldEntry {
        haxe_name: "FoodFactor",
        rust_path: "server.toml food_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodFactorEatenMoreThanEightPercent",
        rust_path: "server.toml food_factor_eaten_more_than_eight_percent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodFactorEatenMoreThanTenPercent",
        rust_path: "server.toml food_factor_eaten_more_than_ten_percent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodFactorEatenLessThanFivePercent",
        rust_path: "server.toml food_factor_eaten_less_than_five_percent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodFactorEatenLessThanThreePercent",
        rust_path: "server.toml food_factor_eaten_less_than_three_percent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodFactorEatenLessThanOnePercent",
        rust_path: "server.toml food_factor_eaten_less_than_one_percent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "YumFoodRestore",
        rust_path: "server.toml yum_food_restore / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LovedFoodRestore",
        rust_path: "server.toml loved_food_restore / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "YumNewCravingChance",
        rust_path: "server.toml yum_new_craving_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodReductionPerEating",
        rust_path: "server.toml food_reduction_per_eating / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodReductionFaktorForEatingMeh",
        rust_path: "server.toml food_reduction_faktor_for_eating_meh / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HealthLostWhenEatingMeh",
        rust_path: "server.toml health_lost_when_eating_meh / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HealthLostWhenEatingSuperMeh",
        rust_path: "server.toml health_lost_when_eating_super_meh / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-TAIL-KNOBS / settings_knobs ---
    FieldEntry {
        // Haxe typo Quailit preserved
        haxe_name: "FoodReductionFaktorForEatingHighQuailitFood",
        rust_path: "server.toml food_reduction_faktor_for_eating_high_quality / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GrownUpFoodStoreMax",
        rust_path: "server.toml grown_up_food_store_max / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-AGE-FOOD / age_food_max ---
    FieldEntry {
        haxe_name: "NewBornFoodStoreMax",
        rust_path: "server.toml new_born_food_store_max / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "OldAgeFoodStoreMax",
        rust_path: "server.toml old_age_food_store_max / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinBiomeSpeedFactor",
        rust_path: "server.toml min_biome_speed_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HitpointsSpeedFactor",
        rust_path: "server.toml hitpoints_speed_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CombatReputationRestorePerYear",
        rust_path: "server.toml combat_reputation_restore_per_year / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-MORE-KNOBS / settings_batch2 ---
    FieldEntry {
        haxe_name: "ExhaustionHealingFactor",
        rust_path: "server.toml exhaustion_healing_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "WoundDamageFactor",
        rust_path: "server.toml wound_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "WoundHealingFactor",
        rust_path: "server.toml wound_healing_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ExhaustionHealingForMaleFaktor",
        rust_path: "server.toml exhaustion_healing_for_male_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-TEMP-HEAL / temp_heal_extra ---
    FieldEntry {
        haxe_name: "TemperatureHitsDamageFactor",
        rust_path: "server.toml temperature_hits_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureExhaustionDamageFactor",
        rust_path: "server.toml temperature_exhaustion_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxMovementQuadJumpDistanceBeforeForce",
        rust_path: "server.toml max_movement_quad_jump_distance_before_force / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodRestoreFactorWhileFeeding",
        rust_path: "server.toml food_restore_factor_while_feeding / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxHasEatenForNextGeneration",
        rust_path: "server.toml max_has_eaten_for_next_generation / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HasEatenReductionForNextGeneration",
        rust_path: "server.toml has_eaten_reduction_for_next_generation / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- WALLET-COINS ---
    FieldEntry {
        haxe_name: "CoinsOnWoundingFactor",
        rust_path: "server.toml coins_on_wounding_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-MORE-BATCH3 / settings_batch3 ---
    // ExhaustionHealingForMaleFaktor → Live under C-SS-MALE-HEAL (above).
    FieldEntry {
        haxe_name: "CombatExhaustionCostPerAttack",
        rust_path: "server.toml combat_exhaustion_cost_per_attack / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinAgeToEat",
        rust_path: "server.toml min_age_to_eat / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxChildAgeForBreastFeeding",
        rust_path: "server.toml max_child_age_for_breast_feeding / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AllyConsideredClose",
        rust_path: "server.toml ally_considered_close / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinMovementAgeInSec",
        rust_path: "server.toml min_movement_age_in_sec / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-MORE-BATCH4 / settings_batch4 ---
    FieldEntry {
        haxe_name: "CursedReceiveDamageFactor",
        rust_path: "server.toml cursed_receive_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CursedMakeDamageFactor",
        rust_path: "server.toml cursed_make_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PickupBabyMaxDistance",
        rust_path: "server.toml pickup_baby_max_distance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "InheritCoinsFactor",
        rust_path: "server.toml inherit_coins_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinAgeFertile",
        rust_path: "server.toml min_age_fertile / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxAgeFertile",
        rust_path: "server.toml max_age_fertile / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- C-SS-MORE-BATCH5 / settings_batch5 ---
    FieldEntry {
        haxe_name: "WeaponCoolDownFactor",
        rust_path: "server.toml weapon_cooldown_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "WeaponCoolDownFactorIfWounding",
        rust_path: "server.toml weapon_cooldown_factor_if_wounding / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CloseEnemyWithWeaponSpeedFactor",
        rust_path: "server.toml close_enemy_with_weapon_speed_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ExhaustionOnJump",
        rust_path: "server.toml exhaustion_on_jump / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HungryWorkHeat",
        rust_path: "server.toml hungry_work_heat / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AISpeedFactorSerf",
        rust_path: "server.toml ai_speed_factor_serf / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AISpeedFactorCommoner",
        rust_path: "server.toml ai_speed_factor_commoner / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AISpeedFactorNoble",
        rust_path: "server.toml ai_speed_factor_noble / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- long-tail inventory (ModuleConst / Deferred residual tables) ---
    FieldEntry {
        haxe_name: "OldGraveDecayMali",
        rust_path: "ol-sim score / grave prestige",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "CursedGraveMali",
        rust_path: "deferred (Haxe TODO no need)",
        home: SettingsHome::Deferred,
    },
    FieldEntry {
        haxe_name: "AncestorPrestigeFactor",
        rust_path: "ol-sim death_inherit / prestige",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "ScoreFactor",
        rust_path: "ol-sim score_entry",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsClose",
        rust_path: "ol-sim leadership / broadcast interest",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForMapChanges",
        rust_path: "ol-sim MX range",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForSay",
        rust_path: "ol-sim speech::ADULT_CHAT_RANGE / MAX_DISTANCE_CLOSE_FOR_SAY (20) + chat_range_for_age",
        home: SettingsHome::ModuleConst, // PO-MAX-DISTANCE: intentional ModuleConst (not LiveSettings)
    },
    FieldEntry {
        haxe_name: "SendMoveEveryXTicks",
        rust_path: "ol-sim PO-FAR-PLAYERS SEND_MOVE_EVERY_X_TICKS (-1 default)",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "AiReactionTime",
        rust_path: "ol-server npc think period proxy",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "AiReactionTimeSerf",
        rust_path: "ol-sim module const (class mult)",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "AiReactionTimeNoble",
        rust_path: "ol-sim module const (class mult)",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "ChanceForDomesticAnimalDyingFactor",
        rust_path: "ol-sim animal_pop (Haxe no-op bug port-as-is)",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "ObjDecayChance",
        rust_path: "ol-sim long_term::OBJ_DECAY_CHANCE",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "FloorDecayChance",
        rust_path: "ol-sim long_term::FLOOR_DECAY_CHANCE",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "AnimalDecayFactor",
        rust_path: "ol-content patch / long_term",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "ObjDecayFactorForPermanentObjs",
        rust_path: "ol-content lib_tail.inc",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "TemperatureImpactReduction",
        rust_path: "deferred (Haxe TODO seems bugged → 0.0)",
        home: SettingsHome::Deferred,
    },
    FieldEntry {
        haxe_name: "WorldTimeParts",
        rust_path: "deferred until TIME-WORLD auto-calc",
        home: SettingsHome::Deferred,
    },
    // MinAgeFertile / MaxAgeFertile / PickupBabyMaxDistance / InheritCoinsFactor
    // promoted Live under C-SS-MORE-BATCH4 (see Live block above).
    FieldEntry {
        haxe_name: "StartingEveAge",
        rust_path: "ol-sim eve_spawn",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "DoorIds",
        rust_path: "ol-config field_map::DOOR_IDS",
        home: SettingsHome::ModuleConst,
    },
    FieldEntry {
        haxe_name: "AiIgnoredFloorIds",
        rust_path: "ol-config field_map::AI_IGNORED_FLOOR_IDS",
        home: SettingsHome::ModuleConst,
    },
    // --- secrets / LLM (never dump) ---
    FieldEntry {
        haxe_name: "Secret",
        rust_path: "omit (not in server.toml dump)",
        home: SettingsHome::SecretOmit,
    },
    FieldEntry {
        haxe_name: "AiApiKey",
        rust_path: "env AI_API_KEY|XAI_API_KEY only (AI-PROVIDER; never server.toml)",
        home: SettingsHome::SecretOmit,
    },
    FieldEntry {
        haxe_name: "AiApiUrl",
        rust_path: "env AI_API_URL (AI-PROVIDER; never server.toml dump)",
        home: SettingsHome::SecretOmit,
    },
    FieldEntry {
        haxe_name: "AiDefaultModel",
        rust_path: "env AI_DEFAULT_MODEL (AI-PROVIDER)",
        home: SettingsHome::SecretOmit,
    },
    // --- debug-only ---
    FieldEntry {
        haxe_name: "debug",
        rust_path: "tracing / RUST_LOG (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "DebugAi",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "DebugTemperature",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "DebugSeason",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "dumpOutput",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "DebugCombat",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
    FieldEntry {
        haxe_name: "DebugEating",
        rust_path: "tracing (not TOML)",
        home: SettingsHome::DebugOnly,
    },
];

/// Names of critical fields still ModuleConst (not Live/BootToml).
pub fn module_const_critical_names() -> Vec<&'static str> {
    CRITICAL_FIELD_MAP
        .iter()
        .filter(|e| e.home == SettingsHome::ModuleConst)
        .map(|e| e.haxe_name)
        .collect()
}

/// Critical fields classified Live.
pub fn live_critical_names() -> Vec<&'static str> {
    CRITICAL_FIELD_MAP
        .iter()
        .filter(|e| e.home == SettingsHome::Live)
        .map(|e| e.haxe_name)
        .collect()
}

/// SecretOmit critical names (must never appear in write_default dump content).
pub fn secret_omit_names() -> Vec<&'static str> {
    CRITICAL_FIELD_MAP
        .iter()
        .filter(|e| e.home == SettingsHome::SecretOmit)
        .map(|e| e.haxe_name)
        .collect()
}

/// Lookup a Haxe name in the critical inventory.
pub fn find_critical(haxe_name: &str) -> Option<&'static FieldEntry> {
    CRITICAL_FIELD_MAP
        .iter()
        .find(|e| e.haxe_name.eq_ignore_ascii_case(haxe_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn door_ids_match_haxe() {
        assert!(is_door_id(115));
        assert!(is_door_id(876));
        assert!(!is_door_id(1));
        assert_eq!(DOOR_IDS.len(), 8);
    }

    #[test]
    fn ai_ignored_floors_match_haxe() {
        assert!(is_ai_ignored_floor_id(656));
        assert!(is_ai_ignored_floor_id(888));
        assert!(!is_ai_ignored_floor_id(0));
    }

    #[test]
    fn critical_map_has_food_and_lockpick_live() {
        let food = find_critical("FoodUsePerSecond").expect("FoodUsePerSecond");
        assert_eq!(food.home, SettingsHome::Live);
        let lp = find_critical("LockpickSucessChance").expect("lockpick");
        assert_eq!(lp.home, SettingsHome::Live);
        let key = find_critical("AiApiKey").expect("AiApiKey");
        assert_eq!(key.home, SettingsHome::SecretOmit);
    }

    #[test]
    fn module_const_critical_nonempty_residual() {
        let residual = module_const_critical_names();
        // Residual gap: many gameplay tables still ModuleConst (intentional).
        // C-SS-TAIL-KNOBS: GrownUp / MinBiome / Hitpoints / HighQuality / CombatRestore Live
        assert!(!residual.contains(&"GrownUpFoodStoreMax"));
        assert!(!residual.contains(&"MinBiomeSpeedFactor"));
        assert!(!residual.contains(&"HitpointsSpeedFactor"));
        assert!(!residual.contains(&"FoodReductionFaktorForEatingHighQuailitFood"));
        assert!(!residual.contains(&"CombatReputationRestorePerYear"));
        // C-SS-AGE-FOOD: NewBorn/OldAge Live
        assert!(!residual.contains(&"NewBornFoodStoreMax"));
        assert!(!residual.contains(&"OldAgeFoodStoreMax"));
        assert!(residual.contains(&"ObjDecayChance"));
        // C-SS-MORE: PrestigeCost* all Live (not residual ModuleConst)
        assert!(!residual.contains(&"PrestigeCostPerDamageForChild"));
        assert!(!residual.contains(&"PrestigeCostPerDamageForElderly"));
        assert!(!residual.contains(&"PrestigeCostPerDamageForCloseRelatives"));
        assert!(!residual.contains(&"PrestigeCostPerDamageForWomenWithoutWeapon"));
        assert!(!residual.contains(&"PrestigeCostPerDamageForAlly"));
        assert!(!residual.contains(&"FoodUsePerSecond"));
        // C-SS-FULL-TABLE: FoodFactor bands Live
        assert!(!residual.contains(&"FoodFactor"));
        assert!(!residual.contains(&"YumFoodRestore"));
        // C-SS-MORE-KNOBS
        assert!(!residual.contains(&"ExhaustionHealingFactor"));
        assert!(!residual.contains(&"WoundDamageFactor"));
        assert!(!residual.contains(&"WoundHealingFactor"));
        assert!(!residual.contains(&"MaxMovementQuadJumpDistanceBeforeForce"));
        assert!(!residual.contains(&"FoodRestoreFactorWhileFeeding"));
        assert!(!residual.contains(&"MaxHasEatenForNextGeneration"));
        assert!(!residual.contains(&"HasEatenReductionForNextGeneration"));
        // WALLET-COINS
        assert!(!residual.contains(&"CoinsOnWoundingFactor"));
        // C-SS-MORE-BATCH3
        assert!(!residual.contains(&"ExhaustionHealingForMaleFaktor"));
        assert!(!residual.contains(&"CombatExhaustionCostPerAttack"));
        assert!(!residual.contains(&"MinAgeToEat"));
        assert!(!residual.contains(&"MaxChildAgeForBreastFeeding"));
        assert!(!residual.contains(&"AllyConsideredClose"));
        assert!(!residual.contains(&"MinMovementAgeInSec"));
        // C-SS-MORE-BATCH4
        assert!(!residual.contains(&"CursedReceiveDamageFactor"));
        assert!(!residual.contains(&"CursedMakeDamageFactor"));
        assert!(!residual.contains(&"PickupBabyMaxDistance"));
        assert!(!residual.contains(&"InheritCoinsFactor"));
        assert!(!residual.contains(&"MinAgeFertile"));
        assert!(!residual.contains(&"MaxAgeFertile"));
        // C-SS-MORE-BATCH5
        assert!(!residual.contains(&"WeaponCoolDownFactor"));
        assert!(!residual.contains(&"WeaponCoolDownFactorIfWounding"));
        assert!(!residual.contains(&"CloseEnemyWithWeaponSpeedFactor"));
        assert!(!residual.contains(&"ExhaustionOnJump"));
        assert!(!residual.contains(&"HungryWorkHeat"));
        assert!(!residual.contains(&"AISpeedFactorSerf"));
        assert!(!residual.contains(&"AISpeedFactorCommoner"));
        assert!(!residual.contains(&"AISpeedFactorNoble"));
        // C-SS-TEMP-HEAL
        assert!(!residual.contains(&"TemperatureHitsDamageFactor"));
        assert!(!residual.contains(&"TemperatureExhaustionDamageFactor"));
    }

    #[test]
    fn live_critical_includes_gameplay_batch() {
        let live = live_critical_names();
        for name in [
            "FoodUsePerSecond",
            "HealingPerSecond",
            "InitialPlayerMoveSpeed",
            "YumBonus",
            "ChanceForOffspring",
            "HungryWorkCost",
            "BirthPrestigeFactor",
            "TimeConfirmNewFollower",
            "HireCost",
            "HireCostIncreasePerPerson",
            // PRESTIGE-ALLY-COST + C-SS-MORE
            "PrestigeCostPerDamageForAlly",
            "PrestigeCostPerDamageForChild",
            "PrestigeCostPerDamageForElderly",
            "PrestigeCostPerDamageForCloseRelatives",
            "PrestigeCostPerDamageForWomenWithoutWeapon",
            // C-SS-FULL-TABLE
            "FoodFactor",
            "FoodFactorEatenMoreThanEightPercent",
            "FoodFactorEatenMoreThanTenPercent",
            "FoodFactorEatenLessThanFivePercent",
            "FoodFactorEatenLessThanThreePercent",
            "FoodFactorEatenLessThanOnePercent",
            "YumFoodRestore",
            // C-SS-TAIL-KNOBS
            "GrownUpFoodStoreMax",
            "MinBiomeSpeedFactor",
            "HitpointsSpeedFactor",
            "FoodReductionFaktorForEatingHighQuailitFood",
            "CombatReputationRestorePerYear",
            // C-SS-MORE-KNOBS
            "ExhaustionHealingFactor",
            "WoundDamageFactor",
            "WoundHealingFactor",
            "MaxMovementQuadJumpDistanceBeforeForce",
            "FoodRestoreFactorWhileFeeding",
            "MaxHasEatenForNextGeneration",
            "HasEatenReductionForNextGeneration",
            // WALLET-COINS
            "CoinsOnWoundingFactor",
            // C-SS-MORE-BATCH3
            "ExhaustionHealingForMaleFaktor",
            "CombatExhaustionCostPerAttack",
            "MinAgeToEat",
            "MaxChildAgeForBreastFeeding",
            "AllyConsideredClose",
            "MinMovementAgeInSec",
            // C-SS-MORE-BATCH4
            "CursedReceiveDamageFactor",
            "CursedMakeDamageFactor",
            "PickupBabyMaxDistance",
            "InheritCoinsFactor",
            "MinAgeFertile",
            "MaxAgeFertile",
            // C-SS-MORE-BATCH5
            "WeaponCoolDownFactor",
            "WeaponCoolDownFactorIfWounding",
            "CloseEnemyWithWeaponSpeedFactor",
            "ExhaustionOnJump",
            "HungryWorkHeat",
            "AISpeedFactorSerf",
            "AISpeedFactorCommoner",
            "AISpeedFactorNoble",
            // C-SS-TEMP-HEAL
            "TemperatureHitsDamageFactor",
            "TemperatureExhaustionDamageFactor",
        ] {
            assert!(live.contains(&name), "missing live {name}");
        }
    }

    #[test]
    fn food_factor_defaults_match_haxe() {
        assert!((gameplay_defaults::FOOD_FACTOR - 1.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT - 0.8).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT - 0.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT - 1.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT - 2.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT - 2.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::YUM_FOOD_RESTORE - 0.8).abs() < f32::EPSILON);
        assert!((gameplay_defaults::LOVED_FOOD_RESTORE - 0.1).abs() < f32::EPSILON);
        assert!((gameplay_defaults::YUM_NEW_CRAVING_CHANCE - 0.2).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_REDUCTION_PER_EATING - 1.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_MEH - 0.2).abs() < f32::EPSILON);
        assert!((gameplay_defaults::HEALTH_LOST_WHEN_EATING_MEH - 0.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::HEALTH_LOST_WHEN_EATING_SUPER_MEH - 2.0).abs() < f32::EPSILON);
        // C-SS-TAIL-KNOBS Haxe defaults 20 / 0.2 / 3 / 0.8 / 2
        assert!((gameplay_defaults::GROWN_UP_FOOD_STORE_MAX - 20.0).abs() < f32::EPSILON);
        // C-SS-AGE-FOOD
        assert!((gameplay_defaults::NEW_BORN_FOOD_STORE_MAX - 4.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::OLD_AGE_FOOD_STORE_MAX - 10.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::MIN_BIOME_SPEED_FACTOR - 0.2).abs() < f32::EPSILON);
        assert!((gameplay_defaults::HITPOINTS_SPEED_FACTOR - 3.0).abs() < f32::EPSILON);
        assert!(
            (gameplay_defaults::FOOD_REDUCTION_FAKTOR_FOR_EATING_HIGH_QUALITY - 0.8).abs()
                < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::COMBAT_REPUTATION_RESTORE_PER_YEAR - 2.0).abs() < f32::EPSILON
        );
        // C-SS-MORE-KNOBS Haxe defaults 1.5 / 1 / 5 / 10 / 4 / 1
        assert!((gameplay_defaults::EXHAUSTION_HEALING_FACTOR - 1.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::WOUND_DAMAGE_FACTOR - 1.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::WOUND_HEALING_FACTOR - 1.0).abs() < f32::EPSILON);
        // C-SS-MALE-HEAL Haxe default 1.2
        assert!((gameplay_defaults::EXHAUSTION_HEALING_FOR_MALE_FACTOR - 1.2).abs() < f32::EPSILON);
        assert!(
            (gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE - 5.0).abs()
                < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::FOOD_RESTORE_FACTOR_WHILE_FEEDING - 10.0).abs() < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::MAX_HAS_EATEN_FOR_NEXT_GENERATION - 4.0).abs() < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::HAS_EATEN_REDUCTION_FOR_NEXT_GENERATION - 1.0).abs() < f32::EPSILON
        );
        // C-SS-MORE-BATCH3 Haxe defaults 1.2 / 0.1 / 3 / 6 / 5 / 14
        assert!(
            (gameplay_defaults::EXHAUSTION_HEALING_FOR_MALE_FACTOR - 1.2).abs() < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::COMBAT_EXHAUSTION_COST_PER_ATTACK - 0.1).abs() < f32::EPSILON
        );
        assert!((gameplay_defaults::MIN_AGE_TO_EAT - 3.0).abs() < f32::EPSILON);
        assert!(
            (gameplay_defaults::MAX_CHILD_AGE_FOR_BREAST_FEEDING - 6.0).abs() < f32::EPSILON
        );
        assert!((gameplay_defaults::ALLY_CONSIDERED_CLOSE - 5.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::MIN_MOVEMENT_AGE_IN_SEC - 14.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH4 Haxe defaults 1.2 / 0.5 / 1.9 / 0.8 / 14 / 42
        assert!(
            (gameplay_defaults::CURSED_RECEIVE_DAMAGE_FACTOR - 1.2).abs() < f32::EPSILON
        );
        assert!((gameplay_defaults::CURSED_MAKE_DAMAGE_FACTOR - 0.5).abs() < f32::EPSILON);
        assert!((gameplay_defaults::PICKUP_BABY_MAX_DISTANCE - 1.9).abs() < f32::EPSILON);
        assert!((gameplay_defaults::INHERIT_COINS_FACTOR - 0.8).abs() < f32::EPSILON);
        assert!((gameplay_defaults::MIN_AGE_FERTILE - 14.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::MAX_AGE_FERTILE - 42.0).abs() < f32::EPSILON);
        // C-SS-MORE-BATCH5 Haxe defaults 0.5 / 5 / 0.8 / 0.05 / 0.002 / 0.8 / 0.9 / 1
        assert!((gameplay_defaults::WEAPON_COOLDOWN_FACTOR - 0.5).abs() < f32::EPSILON);
        assert!(
            (gameplay_defaults::WEAPON_COOLDOWN_FACTOR_IF_WOUNDING - 5.0).abs() < f32::EPSILON
        );
        assert!(
            (gameplay_defaults::CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR - 0.8).abs() < f32::EPSILON
        );
        assert!((gameplay_defaults::EXHAUSTION_ON_JUMP - 0.05).abs() < f32::EPSILON);
        assert!((gameplay_defaults::HUNGRY_WORK_HEAT - 0.002).abs() < f32::EPSILON);
        assert!((gameplay_defaults::AI_SPEED_FACTOR_SERF - 0.8).abs() < f32::EPSILON);
        assert!((gameplay_defaults::AI_SPEED_FACTOR_COMMONER - 0.9).abs() < f32::EPSILON);
        assert!((gameplay_defaults::AI_SPEED_FACTOR_NOBLE - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn inventory_row_count_stable() {
        // Bump deliberately when expanding the map; guards accidental shrink.
        // C-SS-FULL-TABLE expanded FoodFactor Live + long-tail ModuleConst rows.
        assert!(
            CRITICAL_FIELD_MAP.len() >= 80,
            "field map shrank: {}",
            CRITICAL_FIELD_MAP.len()
        );
    }
}
