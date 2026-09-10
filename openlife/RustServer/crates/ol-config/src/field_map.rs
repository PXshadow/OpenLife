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
    /// Haxe-only (threading/mutex/RTTI). Rust does not use these.
    NotApplicable,
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
    is_door_id_in(item_id, DOOR_IDS)
}

/// Door check against a live id table (empty → compiled [`DOOR_IDS`]).
pub fn is_door_id_in(item_id: i32, ids: &[i32]) -> bool {
    if ids.is_empty() {
        DOOR_IDS.contains(&item_id)
    } else {
        ids.contains(&item_id)
    }
}

/// Haxe floor ids AI should ignore for count/drop/pickup-non-food.
// Haxe: ServerSettings.AiIgnoredFloorIds
#[inline]
pub fn is_ai_ignored_floor_id(floor_id: i32) -> bool {
    is_ai_ignored_floor_id_in(floor_id, AI_IGNORED_FLOOR_IDS)
}

/// AI ignored-floor check against a live id table (empty → compiled default).
pub fn is_ai_ignored_floor_id_in(floor_id: i32, ids: &[i32]) -> bool {
    if ids.is_empty() {
        AI_IGNORED_FLOOR_IDS.contains(&floor_id)
    } else {
        ids.contains(&floor_id)
    }
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
    /// Haxe `ChanceForAnimalDyingFactorIfInLovedBiome` (preferred biome).
    // Haxe: ServerSettings.ChanceForAnimalDyingFactorIfInLovedBiome = 0.1
    // SETTINGS-LONG-TAIL
    pub const CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME: f32 = 0.1;
    /// Haxe `OffspringFactorIfAnimalPopIsLow` — low-pop birth chance multiplier.
    // Haxe: ServerSettings.OffspringFactorIfAnimalPopIsLow = 10
    // SETTINGS-LONG-TAIL
    pub const OFFSPRING_FACTOR_IF_ANIMAL_POP_IS_LOW: f32 = 10.0;
    /// Haxe `MaxOffspringFactor` — pop cap as multiple of original.
    // Haxe: ServerSettings.MaxOffspringFactor = 1
    // SETTINGS-LONG-TAIL
    pub const MAX_OFFSPRING_FACTOR: f32 = 1.0;
    /// Haxe `OffspringFactorLowAnimalPopulationBelow` — low-pop birth threshold (fraction of original).
    // Haxe: ServerSettings.OffspringFactorLowAnimalPopulationBelow = 0.2
    // SETTINGS-LONG-TAIL
    pub const OFFSPRING_FACTOR_LOW_ANIMAL_POPULATION_BELOW: f32 = 0.2;
    /// Haxe `BiomeAnimalHitChance` (0 → biome animals almost never hit).
    // Haxe: ServerSettings.BiomeAnimalHitChance = 0.0
    // MOSQUITO-MAPCHANCE
    pub const BIOME_ANIMAL_HIT_CHANCE: f32 = 0.0;
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
    /// Haxe `AiReactionTime` — Commoner minimum AI react delay (seconds).
    // Haxe: ServerSettings.AiReactionTime = 0.5
    pub const AI_REACTION_TIME: f32 = 0.5;
    /// Haxe `AiReactionTimeSerf`.
    // Haxe: ServerSettings.AiReactionTimeSerf = 0.7
    pub const AI_REACTION_TIME_SERF: f32 = 0.7;
    /// Haxe `AiReactionTimeNoble` (+ King/Emperor).
    // Haxe: ServerSettings.AiReactionTimeNoble = 0.2
    pub const AI_REACTION_TIME_NOBLE: f32 = 0.2;
    /// Haxe `AiReactionTimeFactorIfAngry` — multiplies reaction while angry/terrified.
    // Haxe: ServerSettings.AiReactionTimeFactorIfAngry = 0.2
    pub const AI_REACTION_TIME_FACTOR_IF_ANGRY: f32 = 0.2;
    /// Haxe `StartingEveAge` — Eve/Adam spawn `age` / `trueAge` (years).
    // Haxe: ServerSettings.StartingEveAge = 14
    // SETTINGS-LONG-TAIL
    pub const STARTING_EVE_AGE: f32 = 14.0;
    /// Haxe `EveOrAdamBirthChance` — synthetic/NPC Eve vs mother spawn roll.
    // Haxe: ServerSettings.EveOrAdamBirthChance = 0.025
    // SETTINGS-LONG-TAIL
    pub const EVE_OR_ADAM_BIRTH_CHANCE: f32 = 0.025;
    /// Haxe `SpawnAiAsEve` — AIs/NPCs may take that Eve roll even if a mother exists.
    // Haxe: ServerSettings.SpawnAiAsEve = false
    // SETTINGS-LONG-TAIL
    pub const SPAWN_AI_AS_EVE: bool = false;
    /// Product setting: humans may be born to AI mothers. Default false.
    pub const ALLOW_HUMANS_BORN_TO_AIS: bool = false;
    /// Haxe `ObjDecayChance` — long-term object decay roll (per tile in band).
    // Haxe: ServerSettings.ObjDecayChance = 0.00005
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_CHANCE: f32 = 0.00005;
    /// Haxe `FloorDecayChance` — long-term floor decay roll.
    // Haxe: ServerSettings.FloorDecayChance = 0.00001
    // SETTINGS-LONG-TAIL
    pub const FLOOR_DECAY_CHANCE: f32 = 0.00001;
    /// Haxe `ObjRespawnChance` — RespawnObjects per empty original tile.
    // Haxe: ServerSettings.ObjRespawnChance = 0.00006
    // SETTINGS-LONG-TAIL
    pub const OBJ_RESPAWN_CHANCE: f32 = 0.00006;
    /// Haxe `GrowBackPlantsIncreaseIfLowPopulation` — spring original-plant boost if current < original/2.
    // Haxe: ServerSettings.GrowBackPlantsIncreaseIfLowPopulation = 2
    // SETTINGS-LONG-TAIL
    pub const GROW_BACK_PLANTS_INCREASE_IF_LOW_POPULATION: f32 = 2.0;
    /// Haxe `GrowBackOriginalPlantsFactor` — spring original-plant roll multiplier.
    // Haxe: ServerSettings.GrowBackOriginalPlantsFactor = 0.02
    // SETTINGS-LONG-TAIL
    pub const GROW_BACK_ORIGINAL_PLANTS_FACTOR: f32 = 0.02;
    /// Haxe `GrowNewPlantsFromExistingFactor` — offspring per season per living plant.
    // Haxe: ServerSettings.GrowNewPlantsFromExistingFactor = 0.05
    // SETTINGS-LONG-TAIL
    pub const GROW_NEW_PLANTS_FROM_EXISTING_FACTOR: f32 = 0.05;
    /// Haxe `MaxPlayersBeforeStartingAsChild` — living count allowing AI↔human Eve cross.
    // Haxe: ServerSettings.MaxPlayersBeforeStartingAsChild = 0
    // SETTINGS-LONG-TAIL
    pub const MAX_PLAYERS_BEFORE_STARTING_AS_CHILD: i32 = 0;
    /// Haxe `SpringWildFoodRegrowChance` — per-season spring chance recompute.
    // Haxe: ServerSettings.SpringWildFoodRegrowChance = 1
    // SETTINGS-LONG-TAIL
    pub const SPRING_WILD_FOOD_REGROW_CHANCE: f32 = 1.0;
    /// Haxe `WinterWildFoodDecayChance` — per-season winter chance recompute.
    // Haxe: ServerSettings.WinterWildFoodDecayChance = 1.5
    // SETTINGS-LONG-TAIL
    pub const WINTER_WILD_FOOD_DECAY_CHANCE: f32 = 1.5;
    /// Haxe `HotSeasonTemperatureFactor` — scale positive season impact on tile temp.
    // Haxe: ServerSettings.HotSeasonTemperatureFactor = 0.75
    // SETTINGS-LONG-TAIL
    pub const HOT_SEASON_TEMPERATURE_FACTOR: f32 = 0.75;
    /// Haxe `ColdSeasonTemperatureFactor` — scale negative season impact on tile temp.
    // Haxe: ServerSettings.ColdSeasonTemperatureFactor = 0.75
    // SETTINGS-LONG-TAIL
    pub const COLD_SEASON_TEMPERATURE_FACTOR: f32 = 0.75;
    /// Haxe `CursedGraveTime` — hours extra decay per overflowing sharp stone (id 34).
    // Haxe: ServerSettings.CursedGraveTime = 12
    // SETTINGS-LONG-TAIL
    pub const CURSED_GRAVE_TIME: f32 = 12.0;
    /// Haxe `AnimalDecayFactor` — long-term decayFactor for horse-cart / domestic / wolf ids.
    // Haxe: ServerSettings.AnimalDecayFactor = 0.05
    // SETTINGS-LONG-TAIL
    pub const ANIMAL_DECAY_FACTOR: f32 = 0.05;
    /// Haxe `ObjDecayFactorForPermanentObjs`.
    // Haxe: ServerSettings.ObjDecayFactorForPermanentObjs = 0.2
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_FACTOR_FOR_PERMANENT_OBJS: f32 = 0.2;
    /// Haxe `ObjDecayFactorForFood`.
    // Haxe: ServerSettings.ObjDecayFactorForFood = 2
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_FACTOR_FOR_FOOD: f32 = 2.0;
    /// Haxe `ObjDecayFactorForClothing`.
    // Haxe: ServerSettings.ObjDecayFactorForClothing = 2
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_FACTOR_FOR_CLOTHING: f32 = 2.0;
    /// Haxe `ObjDecayFactorForWalls`.
    // Haxe: ServerSettings.ObjDecayFactorForWalls = 0.2
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_FACTOR_FOR_WALLS: f32 = 0.2;
    /// Haxe `ObjDecayFactorPerTechLevel`.
    // Haxe: ServerSettings.ObjDecayFactorPerTechLevel = 10
    // SETTINGS-LONG-TAIL
    pub const OBJ_DECAY_FACTOR_PER_TECH_LEVEL: f32 = 10.0;
    /// Haxe `DecayFactorInDeepWater`.
    // Haxe: ServerSettings.DecayFactorInDeepWater = 5
    // SETTINGS-LONG-TAIL
    pub const DECAY_FACTOR_IN_DEEP_WATER: f32 = 5.0;
    /// Haxe `DecayFactorInMountain`.
    // Haxe: ServerSettings.DecayFactorInMountain = 3
    // SETTINGS-LONG-TAIL
    pub const DECAY_FACTOR_IN_MOUNTAIN: f32 = 3.0;
    /// Haxe `DecayFactorInWalkableWater`.
    // Haxe: ServerSettings.DecayFactorInWalkableWater = 2
    // SETTINGS-LONG-TAIL
    pub const DECAY_FACTOR_IN_WALKABLE_WATER: f32 = 2.0;
    /// Haxe `DecayFactorInJungle`.
    // Haxe: ServerSettings.DecayFactorInJungle = 2
    // SETTINGS-LONG-TAIL
    pub const DECAY_FACTOR_IN_JUNGLE: f32 = 2.0;
    /// Haxe `DecayFactorInSwamp`.
    // Haxe: ServerSettings.DecayFactorInSwamp = 2
    // SETTINGS-LONG-TAIL
    pub const DECAY_FACTOR_IN_SWAMP: f32 = 2.0;
    /// Haxe `ScoreFactor` — EMA weight of this life prestige into account score.
    // Haxe: ServerSettings.ScoreFactor = 0.2
    // SETTINGS-LONG-TAIL
    pub const SCORE_FACTOR: f32 = 0.2;
    /// Haxe `AncestorPrestigeFactor` — dead prestige × factor → ancestor score entry.
    // Haxe: ServerSettings.AncestorPrestigeFactor = 0.2
    // SETTINGS-LONG-TAIL
    pub const ANCESTOR_PRESTIGE_FACTOR: f32 = 0.2;
    /// Haxe `DisplayScoreFactor` — multiply displayed prestige in GM / last-life.
    // Haxe: ServerSettings.DisplayScoreFactor = 1
    // SETTINGS-LONG-TAIL
    pub const DISPLAY_SCORE_FACTOR: f32 = 1.0;
    /// Haxe `DisplayScoreOn` — extra age-58 / last-life prestige GMs.
    // Haxe: ServerSettings.DisplayScoreOn = true
    // SETTINGS-KNOB-TAIL
    pub const DISPLAY_SCORE_ON: bool = true;
    /// Haxe `MaxCoinsPerChest`.
    // Haxe: ServerSettings.MaxCoinsPerChest = 200
    // SETTINGS-KNOB-TAIL
    pub const MAX_COINS_PER_CHEST: i32 = 200;
    /// Haxe `MaxCoinsPerPouch`.
    // Haxe: ServerSettings.MaxCoinsPerPouch = 50
    // SETTINGS-KNOB-TAIL
    pub const MAX_COINS_PER_POUCH: i32 = 50;
    /// Haxe `ChanceForFemaleChild` — spawnAsChild roll; Eve founder uses `>= 0.5`.
    // Haxe: ServerSettings.ChanceForFemaleChild = 0.6
    // SETTINGS-KNOB-TAIL
    pub const CHANCE_FOR_FEMALE_CHILD: f32 = 0.6;
    /// Haxe `ChanceForOtherChildColor`.
    // Haxe: ServerSettings.ChanceForOtherChildColor = 0.2
    // SETTINGS-KNOB-TAIL
    pub const CHANCE_FOR_OTHER_CHILD_COLOR: f32 = 0.2;
    /// Haxe `ChanceForOtherChildColorIfCloseToWrongSpecialBiome`.
    // Haxe: ServerSettings.ChanceForOtherChildColorIfCloseToWrongSpecialBiome = 0.3
    // SETTINGS-KNOB-TAIL
    pub const CHANCE_FOR_OTHER_CHILD_COLOR_IF_CLOSE_TO_WRONG_SPECIAL_BIOME: f32 = 0.3;
    /// Haxe `LittleKidsPerMother` — mother fitness hard-reject at this many age≤MinAgeToEat kids.
    // Haxe: ServerSettings.LittleKidsPerMother = 3
    // SETTINGS-KNOB-TAIL
    pub const LITTLE_KIDS_PER_MOTHER: i32 = 3;
    /// Haxe `NewChildExhaustionForMother`.
    // Haxe: ServerSettings.NewChildExhaustionForMother = 0
    // SETTINGS-KNOB-TAIL
    pub const NEW_CHILD_EXHAUSTION_FOR_MOTHER: f32 = 0.0;
    /// Haxe `AiMotherBirthMaliForHumanChild`.
    // Haxe: ServerSettings.AiMotherBirthMaliForHumanChild = 3
    // SETTINGS-KNOB-TAIL
    pub const AI_MOTHER_BIRTH_MALI_FOR_HUMAN_CHILD: f32 = 3.0;
    /// Haxe `HumanMotherBirthMaliForAiChild`.
    // Haxe: ServerSettings.HumanMotherBirthMaliForAiChild = 1
    // SETTINGS-KNOB-TAIL
    pub const HUMAN_MOTHER_BIRTH_MALI_FOR_AI_CHILD: f32 = 1.0;
    /// Haxe `SpwanAtLastDead` (typo Spwan) — Eve origin is last-death startingGx/Gy.
    // Haxe: ServerSettings.SpwanAtLastDead = false
    // SETTINGS-KNOB-TAIL
    pub const SPAWN_AT_LAST_DEAD: bool = false;
    /// Haxe `TemperatureOwnTileRate` — tile lerp toward biome+season (per sec).
    // Haxe: ServerSettings.TemperatureOwnTileRate = 0.05
    // SETTINGS-KNOB-TAIL
    pub const TEMPERATURE_OWN_TILE_RATE: f32 = 0.05;
    /// Haxe `TemperatureBalanceRate` — neighbor thermal diffusion (per sec).
    // Haxe: ServerSettings.TemperatureBalanceRate = 0.9
    // SETTINGS-KNOB-TAIL
    pub const TEMPERATURE_BALANCE_RATE: f32 = 0.9;
    /// Haxe `TemperatureLocalHeatFactor` — fire/ice heatValue scale on tiles.
    // Haxe: ServerSettings.TemperatureLocalHeatFactor = 0.005
    // SETTINGS-KNOB-TAIL
    pub const TEMPERATURE_LOCAL_HEAT_FACTOR: f32 = 0.005;
    /// Haxe `AverageSeasonTemperatureImpact` — TimeHelper.DoSeason base magnitude.
    // Haxe: ServerSettings.AverageSeasonTemperatureImpact = 0.2
    // SETTINGS-KNOB-TAIL
    pub const AVERAGE_SEASON_TEMPERATURE_IMPACT: f32 = 0.2;
    /// Haxe `AiTotalScoreFactor` — AI `totalScore` scale.
    // Haxe: ServerSettings.AiTotalScoreFactor = 0.8
    // SETTINGS-LONG-TAIL
    pub const AI_TOTAL_SCORE_FACTOR: f32 = 0.8;
    /// Haxe `OldGraveDecayMali` — prestige mali when Old Grave (89) decays unburied.
    // Haxe: ServerSettings.OldGraveDecayMali = 20
    // SETTINGS-LONG-TAIL
    pub const OLD_GRAVE_DECAY_MALI: f32 = 20.0;
    /// Haxe `CursedGraveMali` — prestige mali per sharp-stone curse tick.
    // Haxe: ServerSettings.CursedGraveMali = 2
    pub const CURSED_GRAVE_MALI: f32 = 2.0;
    /// Haxe `MaxDistanceToBeConsideredAsClose` — PU interest (2e6 ≈ all players).
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsClose = 2000000
    pub const MAX_DISTANCE_CLOSE: i32 = 2_000_000;
    /// Haxe `MaxDistanceToBeConsideredAsCloseForMapChanges` — MX radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForMapChanges = 10
    pub const MAX_DISTANCE_MAP_CHANGES: i32 = 10;
    /// Haxe `lastVanillaID` — highest vanilla object id (`< 1` = mapping off).
    // Haxe: ServerSettings.lastVanillaID = -1
    pub const LAST_VANILLA_ID: i32 = -1;
    /// Haxe `OpenLifeClientName` — LOGIN `client_tag` substring that skips remap.
    // Haxe: ServerSettings.OpenLifeClientName = "OpenLife"
    pub const OPEN_LIFE_CLIENT_NAME: &str = "OpenLife";
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSay` — adult SAY radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSay = 20
    pub const MAX_DISTANCE_SAY: i32 = 20;
    /// Haxe `SendMoveEveryXTicks` — periodic `sendToMeAllClosePlayers(false,false)`.
    /// Negative (product `-1`) disables; `> 0` is the tick period.
    // Haxe: ServerSettings.SendMoveEveryXTicks = -1
    // SETTINGS-LONG-TAIL
    pub const SEND_MOVE_EVERY_X_TICKS: i32 = -1;
    /// Haxe `MaxDistanceToBeConsideredAsCoseForMovement` (typo Cose) — PM radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCoseForMovement = 30
    // SETTINGS-LONG-TAIL
    pub const MAX_DISTANCE_COSE_FOR_MOVEMENT: i32 = 30;
    /// Haxe `MaxDistanceToBeConsideredAsCloseForSayAi` — AI hear / sayHelper radius.
    // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi = 20
    // SETTINGS-LONG-TAIL
    pub const MAX_DISTANCE_SAY_AI: f32 = 20.0;
    /// Haxe `MaxDistanceToAutoExileAttacker` — compared to exact **quad** distance.
    // Haxe: ServerSettings.MaxDistanceToAutoExileAttacker = 15
    pub const MAX_DISTANCE_AUTO_EXILE_ATTACKER: i32 = 15;
    /// Haxe `SpeedWithBothShoes` — walk bonus when both shoes and not horse/car.
    // Haxe: ServerSettings.SpeedWithBothShoes = 1.1
    // SETTINGS-LONG-TAIL
    pub const SPEED_WITH_BOTH_SHOES: f32 = 1.1;
    /// Haxe `AgingFactorWhileStarvingToDeath` — youth ×factor, adult ×1/factor.
    // Haxe: ServerSettings.AgingFactorWhileStarvingToDeath = 0.5
    // SETTINGS-LONG-TAIL
    pub const AGING_FACTOR_WHILE_STARVING: f32 = 0.5;
    /// Haxe `GrownUpAge` — youth/adult split for starve aging + child food use.
    // Haxe: ServerSettings.GrownUpAge = 14
    // SETTINGS-LONG-TAIL
    pub const GROWN_UP_AGE: f32 = 14.0;
    /// Haxe `FoodUseChildFaktor` — child foodDecay when `age < GrownUpAge && food > 0`.
    // Haxe: ServerSettings.FoodUseChildFaktor = 1
    // SETTINGS-LONG-TAIL
    pub const FOOD_USE_CHILD_FAKTOR: f32 = 1.0;
    /// Haxe `AIFoodUseFactorSerf` — AI foodDecay class mult.
    // Haxe: ServerSettings.AIFoodUseFactorSerf = 0.8
    // SETTINGS-LONG-TAIL
    pub const AI_FOOD_USE_FACTOR_SERF: f32 = 0.8;
    /// Haxe `AIFoodUseFactorCommoner` — AI foodDecay class mult.
    // Haxe: ServerSettings.AIFoodUseFactorCommoner = 0.9
    // SETTINGS-LONG-TAIL
    pub const AI_FOOD_USE_FACTOR_COMMONER: f32 = 0.9;
    /// Haxe `AIFoodUseFactorNoble` — AI foodDecay class mult (Noble only; not King/Emperor).
    // Haxe: ServerSettings.AIFoodUseFactorNoble = 1
    // SETTINGS-LONG-TAIL
    pub const AI_FOOD_USE_FACTOR_NOBLE: f32 = 1.0;
    /// Haxe `EveFoodUseFactor` — unwounded Eve/Adam foodDecay mult.
    // Haxe: ServerSettings.EveFoodUseFactor = 1
    // SETTINGS-LONG-TAIL
    pub const EVE_FOOD_USE_FACTOR: f32 = 1.0;
    /// Haxe `AgingFactorHumanBornToAi` — infant human + AI mother.
    // Haxe: ServerSettings.AgingFactorHumanBornToAi = 3
    pub const AGING_FACTOR_HUMAN_BORN_TO_AI: f32 = 3.0;
    /// Haxe `AgingFactorAiBornToHuman` — infant AI + human mother.
    // Haxe: ServerSettings.AgingFactorAiBornToHuman = 1.5
    pub const AGING_FACTOR_AI_BORN_TO_HUMAN: f32 = 1.5;
    /// Haxe `EveDamageFactor` — Eve/Adam attacker and target DoDamage mult.
    // Haxe: ServerSettings.EveDamageFactor = 1
    // SETTINGS-LONG-TAIL
    pub const EVE_DAMAGE_FACTOR: f32 = 1.0;
    /// Haxe `TargetWoundedDamageFactor` — already-wounded target DoDamage mult.
    // Haxe: ServerSettings.TargetWoundedDamageFactor = 0.2
    // SETTINGS-LONG-TAIL
    pub const TARGET_WOUNDED_DAMAGE_FACTOR: f32 = 0.2;
    /// Haxe `MaleDamageFactor` — male attacker DoDamage mult.
    // Haxe: ServerSettings.MaleDamageFactor = 1.2
    // SETTINGS-LONG-TAIL
    pub const MALE_DAMAGE_FACTOR: f32 = 1.2;
    /// Haxe `AnimalDamageFactor` — animal-branch DoDamage (`attacker == null`).
    // Haxe: ServerSettings.AnimalDamageFactor = 1.5
    // SETTINGS-LONG-TAIL
    pub const ANIMAL_DAMAGE_FACTOR: f32 = 1.5;
    /// Haxe `AnimalDamageFactorInWinter` — extra animal DoDamage in winter.
    // Haxe: ServerSettings.AnimalDamageFactorInWinter = 2
    // SETTINGS-LONG-TAIL
    pub const ANIMAL_DAMAGE_FACTOR_IN_WINTER: f32 = 2.0;
    /// Haxe `AnimalDamageFactorIfAttacked` — extra animal DoDamage when `hits > 0`.
    // Haxe: ServerSettings.AnimalDamageFactorIfAttacked = 1.5
    // SETTINGS-LONG-TAIL
    pub const ANIMAL_DAMAGE_FACTOR_IF_ATTACKED: f32 = 1.5;
    /// Haxe `WeaponDamageFactor` — player-attacker DoDamage (`attacker != null`).
    // Haxe: ServerSettings.WeaponDamageFactor = 1
    // SETTINGS-LONG-TAIL
    pub const WEAPON_DAMAGE_FACTOR: f32 = 1.0;
    /// Haxe `GraveBlockingDistance` — bone-grave curse radius.
    // Haxe: ServerSettings.GraveBlockingDistance = 40
    // SETTINGS-LONG-TAIL
    pub const GRAVE_BLOCKING_DISTANCE: f32 = 40.0;
    /// Haxe `MaxPlayersBeforeActivatingGraveCurse` (0 = always when near).
    // Haxe: ServerSettings.MaxPlayersBeforeActivatingGraveCurse = 0
    // SETTINGS-LONG-TAIL
    pub const MAX_PLAYERS_BEFORE_ACTIVATING_GRAVE_CURSE: i32 = 0;
    /// Haxe `MaxPlayersBeforeForbidTouchGrave` (9999 = effectively off).
    // Haxe: ServerSettings.MaxPlayersBeforeForbidTouchGrave = 9999
    // GRAVE-TOUCH-PLAYERS
    pub const MAX_PLAYERS_BEFORE_FORBID_TOUCH_GRAVE: i32 = 9999;
    /// Haxe `CombatAngryTimeBeforeAttack` — spawn `angryTime` + UpdateEmotes soft combat.
    // Haxe: ServerSettings.CombatAngryTimeBeforeAttack = 5
    // SETTINGS-LONG-TAIL
    pub const COMBAT_ANGRY_TIME_BEFORE_ATTACK: f32 = 5.0;
    /// Haxe `CombatAngryTimeMinimum` — floor while killMode / last attacker armed.
    // Haxe: ServerSettings.CombatAngryTimeMinimum = -60
    pub const COMBAT_ANGRY_TIME_MINIMUM: f32 = -60.0;
    /// Haxe `ChanceForDomesticAnimalDyingFactor` (intended multiply; Haxe line was a no-op).
    // Haxe: ServerSettings.ChanceForDomesticAnimalDyingFactor = 2
    pub const CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR: f32 = 2.0;
    /// Haxe `Secret` — admin SAY `!S <secret>`.
    pub const SECRET: &str = "JASON";
    /// Haxe `AiTimeToWaitIfCraftingFailed` — failedCraftings cooldown seconds.
    // Haxe: ServerSettings.AiTimeToWaitIfCraftingFailed = 15
    // SETTINGS-LONG-TAIL
    pub const AI_TIME_TO_WAIT_IF_CRAFTING_FAILED: f32 = 15.0;
    /// Haxe `AiMaxSearchRadius`.
    // Haxe: ServerSettings.AiMaxSearchRadius = 60
    // SETTINGS-LONG-TAIL
    pub const AI_MAX_SEARCH_RADIUS: i32 = 60;
    /// Haxe `AiMaxSearchIncrement`.
    // Haxe: ServerSettings.AiMaxSearchIncrement = 30
    // SETTINGS-LONG-TAIL
    pub const AI_MAX_SEARCH_INCREMENT: i32 = 30;
    /// Haxe `AiMemoryMaxEntries` — PlayerSoul interaction FIFO cap.
    // Haxe: ServerSettings.AiMemoryMaxEntries = 20
    // SOUL-LIVE-CAPS
    pub const AI_MEMORY_MAX_ENTRIES: i32 = 20;
    /// Haxe `AiChatMemoryMaxEntries` — PlayerSoul chat FIFO cap.
    // Haxe: ServerSettings.AiChatMemoryMaxEntries = 100
    // SOUL-LIVE-CAPS
    pub const AI_CHAT_MEMORY_MAX_ENTRIES: i32 = 100;
    /// Haxe `AiIgnoreTimeTransitionsLongerThen` (Haxe typo Then).
    // Haxe: ServerSettings.AiIgnoreTimeTransitionsLongerThen = 120
    // SETTINGS-LONG-TAIL
    pub const AI_IGNORE_TIME_TRANSITIONS_LONGER_THEN: f32 = 120.0;
    /// Haxe `AlternativeOutcomePercentIncreasePerHit`.
    // Haxe: ServerSettings.AlternativeOutcomePercentIncreasePerHit = 10
    // SETTINGS-LONG-TAIL
    pub const ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT: f32 = 10.0;
    /// Haxe `AlternativeOutcomeHitsDecreaseOnSucess` (Haxe spelling).
    // Haxe: ServerSettings.AlternativeOutcomeHitsDecreaseOnSucess = 5
    // SETTINGS-LONG-TAIL
    pub const ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS: f32 = 5.0;
    /// Haxe `FortificationCosePerHit` (Haxe spelling).
    // Haxe: ServerSettings.FortificationCosePerHit = 1
    // TH-ALT-LIVE-KNOBS
    pub const FORTIFICATION_COST_PER_HIT: f32 = 1.0;
    /// Haxe `ReduceAgeNeededToPickupObjects` — subtract from minPickupAge for USE/DROP.
    // Haxe: ServerSettings.ReduceAgeNeededToPickupObjects = 10
    // MIN-PICKUP-AGE
    pub const REDUCE_AGE_NEEDED_TO_PICKUP_OBJECTS: f32 = 10.0;
    /// Haxe `ChanceThatAnimalsCanPassBlockingBiome`.
    // Haxe: ServerSettings.ChanceThatAnimalsCanPassBlockingBiome = 0.03
    // SETTINGS-LONG-TAIL
    pub const CHANCE_ANIMALS_PASS_BLOCKING_BIOME: f32 = 0.03;
    /// Haxe `chancePreferredBiome`.
    // Haxe: ServerSettings.chancePreferredBiome = 0.8
    // SETTINGS-LONG-TAIL
    pub const CHANCE_PREFERRED_BIOME: f32 = 0.8;
    /// Haxe `CloseGraveSpeedMali`.
    // Haxe: ServerSettings.CloseGraveSpeedMali = 0.9
    // SETTINGS-LONG-TAIL
    pub const CLOSE_GRAVE_SPEED_MALI: f32 = 0.9;
    /// Haxe `TemperatureSpeedImpact`.
    // Haxe: ServerSettings.TemperatureSpeedImpact = 1
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_SPEED_IMPACT: f32 = 1.0;
    /// Haxe `MinSpeedReductionPerContainedObj` — upper clamp per contained `speedMult`.
    // Haxe: ServerSettings.MinSpeedReductionPerContainedObj = 0.98
    // SETTINGS-LONG-TAIL
    pub const MIN_SPEED_REDUCTION_PER_CONTAINED_OBJ: f32 = 0.98;
    /// Haxe `LovedFoodUseChance` — bare-hand loved-plant extra harvest base chance.
    // Haxe: ServerSettings.LovedFoodUseChance = 0.5
    // SETTINGS-LONG-TAIL
    pub const LOVED_FOOD_USE_CHANCE: f32 = 0.5;
    /// Haxe `MaxAgeForAllowingClothAndPrickupFromOthers` (Haxe typo Prickup).
    // Haxe: ServerSettings.MaxAgeForAllowingClothAndPrickupFromOthers = 10
    // SETTINGS-LONG-TAIL
    pub const MAX_AGE_FOR_ALLOWING_CLOTH_AND_PICKUP_FROM_OTHERS: f32 = 10.0;
    /// Haxe `MaxAgeForAllowingDie` — SAY/client DIE allowed while age <= this.
    // Haxe: ServerSettings.MaxAgeForAllowingDie = 2
    // SETTINGS-LONG-TAIL
    pub const MAX_AGE_FOR_ALLOWING_DIE: f32 = 2.0;
    /// Haxe `PrestigeCostForDie` — account.score must be >= this to /DIE; not debited.
    // Haxe: ServerSettings.PrestigeCostForDie = 0
    // SETTINGS-LONG-TAIL
    pub const PRESTIGE_COST_FOR_DIE: f32 = 0.0;
    /// Haxe `StartingFamilyName` — DoNaming I AM found-new-family gate.
    // Haxe: ServerSettings.StartingFamilyName = "SNOW"
    // SETTINGS-LONG-TAIL
    pub const STARTING_FAMILY_NAME: &str = "SNOW";
    /// Haxe `StartingName` — DoNaming YOU ARE only if target first name is this.
    // Haxe: ServerSettings.StartingName = "SPOON"
    // SETTINGS-LONG-TAIL
    pub const STARTING_NAME: &str = "SPOON";
    /// Haxe `FoundFamilyNeededPrestige` — DoNaming I AM found-new prestige gate.
    // Haxe: ServerSettings.FoundFamilyNeededPrestige = 50
    // SETTINGS-LONG-TAIL
    pub const FOUND_FAMILY_NEEDED_PRESTIGE: f32 = 50.0;
    /// Haxe `FoundFamilyCost` — coins required and subtracted on found-new family.
    // Haxe: ServerSettings.FoundFamilyCost = 10
    // SETTINGS-LONG-TAIL
    pub const FOUND_FAMILY_COST: f32 = 10.0;
    /// Haxe `FoundFamilyNeededFollowers` — DoNaming I AM found-new same-family follower gate.
    // Haxe: ServerSettings.FoundFamilyNeededFollowers = 4
    // SETTINGS-LONG-TAIL
    pub const FOUND_FAMILY_NEEDED_FOLLOWERS: i32 = 4;
    /// Haxe `FoundFamilyBreakAllianceChance` — AI foundFamily I FOLLOW ME roll.
    // Haxe: ServerSettings.FoundFamilyBreakAllianceChance = 0.5
    // SETTINGS-LONG-TAIL
    pub const FOUND_FAMILY_BREAK_ALLIANCE_CHANCE: f32 = 0.5;
    /// Haxe `PickupExhaustionGain` — HOLD/doBaby exhaustion add.
    // Haxe: ServerSettings.PickupExhaustionGain = 0.2
    // SETTINGS-LONG-TAIL
    pub const PICKUP_EXHAUSTION_GAIN: f32 = 0.2;
    /// Haxe `PickupFeedingFoodRestore` — HOLD pickup food granted to baby.
    // Haxe: ServerSettings.PickupFeedingFoodRestore = 1.5
    // SETTINGS-LONG-TAIL
    pub const PICKUP_FEEDING_FOOD_RESTORE: f32 = 1.5;
    /// Haxe `DeathWithFoodStoreMax` — starve/wound death when food_max below this.
    // Haxe: ServerSettings.DeathWithFoodStoreMax = -0.1
    // SETTINGS-LONG-TAIL
    pub const DEATH_WITH_FOOD_STORE_MAX: f32 = -0.1;
    /// Haxe `FoodStoreMaxReductionWhileStarvingToDeath`.
    // Haxe: ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath = 5
    // SETTINGS-LONG-TAIL
    pub const FOOD_STORE_MAX_REDUCTION_WHILE_STARVING: f32 = 5.0;
    /// Haxe `TemperatureReductionPerDrinking` — doSelf drink heat drop / stored water add.
    // Haxe: ServerSettings.TemperatureReductionPerDrinking = 0.5
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_REDUCTION_PER_DRINKING: f32 = 0.5;
    /// Haxe `MaxStoredWater` — doSelf drink stored-water cap.
    // Haxe: ServerSettings.MaxStoredWater = 1
    // SETTINGS-LONG-TAIL
    pub const MAX_STORED_WATER: f32 = 1.0;
    /// Haxe `MaxJumpsPerTenSec` — MOVE jump rate-limit + jumpedTiles decay.
    // Haxe: ServerSettings.MaxJumpsPerTenSec = 10
    // SETTINGS-LONG-TAIL
    pub const MAX_JUMPS_PER_TEN_SEC: f32 = 10.0;
    /// Haxe `TemperatureImpactPerSec` — body heat step when ambient is not helping.
    // Haxe: ServerSettings.TemperatureImpactPerSec = 0.03
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_IMPACT_PER_SEC: f32 = 0.03;
    /// Haxe `TemperatureImpactPerSecIfGood` — faster heat step toward ideal.
    // Haxe: ServerSettings.TemperatureImpactPerSecIfGood = 0.06
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_IMPACT_PER_SEC_IF_GOOD: f32 = 0.06;
    /// Haxe `TemperatureInWaterFactor` — body heat step when standing in water.
    // Haxe: ServerSettings.TemperatureInWaterFactor = 1.5
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_IN_WATER_FACTOR: f32 = 1.5;
    /// Haxe `TemperatureImpactBelow` — super-hot/cold half-width from 0.5.
    // Haxe: ServerSettings.TemperatureImpactBelow = 0.6
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_IMPACT_BELOW: f32 = 0.6;
    /// Haxe `TemperatureImpactColorFactor` — person-color super-hot/cold offset scale.
    // Haxe: ServerSettings.TemperatureImpactColorFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub const TEMPERATURE_IMPACT_COLOR_FACTOR: f32 = 0.5;
    /// Haxe `AllowEatingOrFeedingIfIll` — yellow-fever eat/feed gate (false blocks).
    // Haxe: ServerSettings.AllowEatingOrFeedingIfIll = false
    // SETTINGS-LONG-TAIL
    pub const ALLOW_EATING_OR_FEEDING_IF_ILL: bool = false;
    /// Haxe `ResistanceAgainstFeverForEatingMushrooms` — isDrugs post-eat fever resist.
    // Haxe: ServerSettings.ResistanceAgainstFeverForEatingMushrooms = 0.2
    // SETTINGS-LONG-TAIL
    pub const RESISTANCE_AGAINST_FEVER_FOR_EATING_MUSHROOMS: f32 = 0.2;
    /// Haxe `ExhaustionYellowFeverPerSec` — yellow-fever food drain (`* 2`).
    // Haxe: ServerSettings.ExhaustionYellowFeverPerSec = 0.1
    // SETTINGS-LONG-TAIL
    pub const EXHAUSTION_YELLOW_FEVER_PER_SEC: f32 = 0.1;
    /// Haxe `MinHealthFoodStoreMaxFactor` — health food-max mali (below-median).
    // Haxe: ServerSettings.MinHealthFoodStoreMaxFactor = 0.8
    // SETTINGS-LONG-TAIL
    pub const MIN_HEALTH_FOOD_STORE_MAX_FACTOR: f32 = 0.8;
    /// Haxe `MaxHealthFoodStoreMaxFactor` — health food-max boni (above-median).
    // Haxe: ServerSettings.MaxHealthFoodStoreMaxFactor = 1.2
    // SETTINGS-LONG-TAIL
    pub const MAX_HEALTH_FOOD_STORE_MAX_FACTOR: f32 = 1.2;
    /// Haxe `MinHealthAgingFactor` — health aging mali (below-median).
    // Haxe: ServerSettings.MinHealthAgingFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub const MIN_HEALTH_AGING_FACTOR: f32 = 0.5;
    /// Haxe `MaxHealthAgingFactor` — health aging boni (above-median).
    // Haxe: ServerSettings.MaxHealthAgingFactor = 2
    // SETTINGS-LONG-TAIL
    pub const MAX_HEALTH_AGING_FACTOR: f32 = 2.0;
    /// Haxe `MinHealthPerYear` — expected health per year (medianPrestige floor ×30).
    // Haxe: ServerSettings.MinHealthPerYear = 1
    // SETTINGS-LONG-TAIL
    pub const MIN_HEALTH_PER_YEAR: f32 = 1.0;
    /// Haxe `MaxAge` — health-factor + food-capacity old-age band years.
    // Haxe: ServerSettings.MaxAge = 60
    // SETTINGS-LONG-TAIL
    pub const MAX_AGE: f32 = 60.0;
    /// Haxe `AnimalDeadlyDistanceFactor` — animal HIT range (tiles).
    // Haxe: ServerSettings.AnimalDeadlyDistanceFactor = 0.5
    // SETTINGS-LONG-TAIL
    pub const ANIMAL_DEADLY_DISTANCE_FACTOR: f32 = 0.5;
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
        haxe_name: "BiomeAnimalHitChance",
        rust_path: "server.toml biome_animal_hit_chance / LiveSettings / SimState.gameplay",
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
    // SETTINGS-LONG-TAIL: StartingEveAge Live (eve spawn / revive)
    FieldEntry {
        haxe_name: "StartingEveAge",
        rust_path: "server.toml starting_eve_age / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // SETTINGS-LONG-TAIL: EveOrAdamBirthChance / SpawnAiAsEve Live (synthetic Eve roll)
    FieldEntry {
        haxe_name: "EveOrAdamBirthChance",
        rust_path: "server.toml eve_or_adam_birth_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SpawnAiAsEve",
        rust_path: "server.toml spawn_ai_as_eve / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AllowHumansBornToAis",
        rust_path: "server.toml allow_humans_born_to_ais / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjDecayChance",
        rust_path: "server.toml obj_decay_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FloorDecayChance",
        rust_path: "server.toml floor_decay_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjRespawnChance",
        rust_path: "server.toml obj_respawn_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GrowBackPlantsIncreaseIfLowPopulation",
        rust_path: "server.toml grow_back_plants_increase_if_low_population / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GrowBackOriginalPlantsFactor",
        rust_path: "server.toml grow_back_original_plants_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GrowNewPlantsFromExistingFactor",
        rust_path: "server.toml grow_new_plants_from_existing_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxPlayersBeforeStartingAsChild",
        rust_path: "server.toml max_players_before_starting_as_child / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SpringWildFoodRegrowChance",
        rust_path: "server.toml spring_wild_food_regrow_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "WinterWildFoodDecayChance",
        rust_path: "server.toml winter_wild_food_decay_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HotSeasonTemperatureFactor",
        rust_path: "server.toml hot_season_temperature_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ColdSeasonTemperatureFactor",
        rust_path: "server.toml cold_season_temperature_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CursedGraveTime",
        rust_path: "server.toml cursed_grave_time / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AnimalDecayFactor",
        rust_path: "server.toml animal_decay_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ScoreFactor",
        rust_path: "server.toml score_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- long-tail inventory (ModuleConst / Deferred residual tables) ---
    FieldEntry {
        haxe_name: "OldGraveDecayMali",
        rust_path: "server.toml old_grave_decay_mali / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CursedGraveMali",
        rust_path: "server.toml cursed_grave_mali / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AncestorPrestigeFactor",
        rust_path: "server.toml ancestor_prestige_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DisplayScoreFactor",
        rust_path: "server.toml display_score_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DisplayScoreOn",
        rust_path: "server.toml display_score_on / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxCoinsPerChest",
        rust_path: "server.toml max_coins_per_chest / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxCoinsPerPouch",
        rust_path: "server.toml max_coins_per_pouch / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForFemaleChild",
        rust_path: "server.toml chance_for_female_child / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForOtherChildColor",
        rust_path: "server.toml chance_for_other_child_color / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForOtherChildColorIfCloseToWrongSpecialBiome",
        rust_path: "server.toml chance_for_other_child_color_if_close_to_wrong_special_biome / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LittleKidsPerMother",
        rust_path: "server.toml little_kids_per_mother / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "NewChildExhaustionForMother",
        rust_path: "server.toml new_child_exhaustion_for_mother / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiMotherBirthMaliForHumanChild",
        rust_path: "server.toml ai_mother_birth_mali_for_human_child / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "HumanMotherBirthMaliForAiChild",
        rust_path: "server.toml human_mother_birth_mali_for_ai_child / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SpwanAtLastDead",
        rust_path: "server.toml spawn_at_last_dead / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureOwnTileRate",
        rust_path: "server.toml temperature_own_tile_rate / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureBalanceRate",
        rust_path: "server.toml temperature_balance_rate / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureLocalHeatFactor",
        rust_path: "server.toml temperature_local_heat_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AverageSeasonTemperatureImpact",
        rust_path: "server.toml average_season_temperature_impact / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiTotalScoreFactor",
        rust_path: "server.toml ai_total_score_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // ScoreFactor promoted Live under SETTINGS-LONG-TAIL.
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsClose",
        rust_path: "server.toml max_distance_close / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForMapChanges",
        rust_path: "server.toml max_distance_map_changes / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "lastVanillaID",
        rust_path: "server.toml last_vanilla_id / LiveSettings / SimState.last_vanilla_id",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "OpenLifeClientName",
        rust_path: "server.toml open_life_client_name / LiveSettings / SimState.open_life_client_name",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForSay",
        rust_path: "server.toml max_distance_say / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SendMoveEveryXTicks",
        rust_path: "server.toml send_move_every_x_ticks / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCoseForMovement",
        rust_path: "server.toml max_distance_cose_for_movement / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForSayAi",
        rust_path: "server.toml max_distance_say_ai / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxDistanceToAutoExileAttacker",
        rust_path: "server.toml max_distance_auto_exile_attacker / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "SpeedWithBothShoes",
        rust_path: "server.toml speed_with_both_shoes / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AgingFactorWhileStarvingToDeath",
        rust_path: "server.toml aging_factor_while_starving / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GrownUpAge",
        rust_path: "server.toml grown_up_age / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodUseChildFaktor",
        rust_path: "server.toml food_use_child_faktor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AIFoodUseFactorSerf",
        rust_path: "server.toml ai_food_use_factor_serf / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AIFoodUseFactorCommoner",
        rust_path: "server.toml ai_food_use_factor_commoner / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AIFoodUseFactorNoble",
        rust_path: "server.toml ai_food_use_factor_noble / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "EveFoodUseFactor",
        rust_path: "server.toml eve_food_use_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AgingFactorHumanBornToAi",
        rust_path: "server.toml aging_factor_human_born_to_ai / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AgingFactorAiBornToHuman",
        rust_path: "server.toml aging_factor_ai_born_to_human / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "EveDamageFactor",
        rust_path: "server.toml eve_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TargetWoundedDamageFactor",
        rust_path: "server.toml target_wounded_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaleDamageFactor",
        rust_path: "server.toml male_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AnimalDamageFactor",
        rust_path: "server.toml animal_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AnimalDamageFactorInWinter",
        rust_path: "server.toml animal_damage_factor_in_winter / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AnimalDamageFactorIfAttacked",
        rust_path: "server.toml animal_damage_factor_if_attacked / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "WeaponDamageFactor",
        rust_path: "server.toml weapon_damage_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "GraveBlockingDistance",
        rust_path: "server.toml grave_blocking_distance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxPlayersBeforeActivatingGraveCurse",
        rust_path: "server.toml max_players_before_activating_grave_curse / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxPlayersBeforeForbidTouchGrave",
        rust_path: "server.toml max_players_before_forbid_touch_grave / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CombatAngryTimeBeforeAttack",
        rust_path: "server.toml combat_angry_time_before_attack / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CombatAngryTimeMinimum",
        rust_path: "server.toml combat_angry_time_minimum / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiTimeToWaitIfCraftingFailed",
        rust_path: "server.toml ai_time_to_wait_if_crafting_failed / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiMaxSearchRadius",
        rust_path: "server.toml ai_max_search_radius / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiMaxSearchIncrement",
        rust_path: "server.toml ai_max_search_increment / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiIgnoreTimeTransitionsLongerThen",
        rust_path: "server.toml ai_ignore_time_transitions_longer_then / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiMemoryMaxEntries",
        rust_path: "server.toml ai_memory_max_entries / LiveSettings / SimState.ai_memory_max_entries",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiChatMemoryMaxEntries",
        rust_path: "server.toml ai_chat_memory_max_entries / LiveSettings / SimState.ai_chat_memory_max_entries",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AlternativeOutcomePercentIncreasePerHit",
        rust_path: "server.toml alternative_outcome_percent_increase_per_hit / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AlternativeOutcomeHitsDecreaseOnSucess",
        rust_path: "server.toml alternative_outcome_hits_decrease_on_success / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FortificationCosePerHit",
        rust_path: "server.toml fortification_cost_per_hit / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ReduceAgeNeededToPickupObjects",
        rust_path: "server.toml reduce_age_needed_to_pickup_objects / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceThatAnimalsCanPassBlockingBiome",
        rust_path: "server.toml chance_animals_pass_blocking_biome / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "chancePreferredBiome",
        rust_path: "server.toml chance_preferred_biome / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "CloseGraveSpeedMali",
        rust_path: "server.toml close_grave_speed_mali / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureSpeedImpact",
        rust_path: "server.toml temperature_speed_impact / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinSpeedReductionPerContainedObj",
        rust_path: "server.toml min_speed_reduction_per_contained_obj / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "LovedFoodUseChance",
        rust_path: "server.toml loved_food_use_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxAgeForAllowingClothAndPrickupFromOthers",
        rust_path: "server.toml max_age_for_allowing_cloth_and_pickup_from_others / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxAgeForAllowingDie",
        rust_path: "server.toml max_age_for_allowing_die / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PrestigeCostForDie",
        rust_path: "server.toml prestige_cost_for_die / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "StartingFamilyName",
        rust_path: "server.toml starting_family_name / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "StartingName",
        rust_path: "server.toml starting_name / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoundFamilyNeededPrestige",
        rust_path: "server.toml found_family_needed_prestige / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoundFamilyCost",
        rust_path: "server.toml found_family_cost / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoundFamilyNeededFollowers",
        rust_path: "server.toml found_family_needed_followers / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoundFamilyBreakAllianceChance",
        rust_path: "server.toml found_family_break_alliance_chance / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PickupExhaustionGain",
        rust_path: "server.toml pickup_exhaustion_gain / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "PickupFeedingFoodRestore",
        rust_path: "server.toml pickup_feeding_food_restore / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DeathWithFoodStoreMax",
        rust_path: "server.toml death_with_food_store_max / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "FoodStoreMaxReductionWhileStarvingToDeath",
        rust_path: "server.toml food_store_max_reduction_while_starving / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureReductionPerDrinking",
        rust_path: "server.toml temperature_reduction_per_drinking / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxStoredWater",
        rust_path: "server.toml max_stored_water / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxJumpsPerTenSec",
        rust_path: "server.toml max_jumps_per_ten_sec / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureImpactPerSec",
        rust_path: "server.toml temperature_impact_per_sec / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureImpactPerSecIfGood",
        rust_path: "server.toml temperature_impact_per_sec_if_good / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureInWaterFactor",
        rust_path: "server.toml temperature_in_water_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureImpactBelow",
        rust_path: "server.toml temperature_impact_below / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "TemperatureImpactColorFactor",
        rust_path: "server.toml temperature_impact_color_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AllowEatingOrFeedingIfIll",
        rust_path: "server.toml allow_eating_or_feeding_if_ill / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ResistanceAgainstFeverForEatingMushrooms",
        rust_path: "server.toml resistance_against_fever_for_eating_mushrooms / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ExhaustionYellowFeverPerSec",
        rust_path: "server.toml exhaustion_yellow_fever_per_sec / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinHealthFoodStoreMaxFactor",
        rust_path: "server.toml min_health_food_store_max_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxHealthFoodStoreMaxFactor",
        rust_path: "server.toml max_health_food_store_max_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinHealthAgingFactor",
        rust_path: "server.toml min_health_aging_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxHealthAgingFactor",
        rust_path: "server.toml max_health_aging_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MinHealthPerYear",
        rust_path: "server.toml min_health_per_year / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxAge",
        rust_path: "server.toml max_age / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AnimalDeadlyDistanceFactor",
        rust_path: "server.toml animal_deadly_distance_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForAnimalDyingFactorIfInLovedBiome",
        rust_path: "server.toml chance_for_animal_dying_factor_if_in_loved_biome / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "OffspringFactorIfAnimalPopIsLow",
        rust_path: "server.toml offspring_factor_if_animal_pop_is_low / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "MaxOffspringFactor",
        rust_path: "server.toml max_offspring_factor / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "OffspringFactorLowAnimalPopulationBelow",
        rust_path: "server.toml offspring_factor_low_animal_population_below / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiReactionTime",
        rust_path: "server.toml ai_reaction_time / LiveSettings (Commoner)",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiReactionTimeSerf",
        rust_path: "server.toml ai_reaction_time_serf / LiveSettings",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiReactionTimeNoble",
        rust_path: "server.toml ai_reaction_time_noble / LiveSettings",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiReactionTimeFactorIfAngry",
        rust_path: "server.toml ai_reaction_time_factor_if_angry / LiveSettings",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ChanceForDomesticAnimalDyingFactor",
        rust_path: "server.toml chance_for_domestic_animal_dying_factor / LiveSettings (applied; Haxe line was unassigned)",
        home: SettingsHome::Live,
    },
    // AnimalDecayFactor promoted Live under SETTINGS-LONG-TAIL.
    FieldEntry {
        haxe_name: "ObjDecayFactorForPermanentObjs",
        rust_path: "server.toml obj_decay_factor_for_permanent / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjDecayFactorForFood",
        rust_path: "server.toml obj_decay_factor_for_food / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjDecayFactorForClothing",
        rust_path: "server.toml obj_decay_factor_for_clothing / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjDecayFactorForWalls",
        rust_path: "server.toml obj_decay_factor_for_walls / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "ObjDecayFactorPerTechLevel",
        rust_path: "server.toml obj_decay_factor_per_tech_level / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DecayFactorInDeepWater",
        rust_path: "server.toml decay_factor_in_deep_water / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DecayFactorInMountain",
        rust_path: "server.toml decay_factor_in_mountain / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DecayFactorInWalkableWater",
        rust_path: "server.toml decay_factor_in_walkable_water / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DecayFactorInJungle",
        rust_path: "server.toml decay_factor_in_jungle / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DecayFactorInSwamp",
        rust_path: "server.toml decay_factor_in_swamp / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // SETTINGS-KNOB-TAIL unused Haxe statics (no Rust reader; skip mutex/debug/secret/file-path):
    // DisplayYumAndMehFood, DisplayPlayerNames*, DisplayTemperatureHintsPerMinute,
    // SecondsBetweenMessages, MinPrestiegeFromCoinDecayPerYear, HungryWorkToolCostFactor,
    // TeleportCost, DomesticAnimalMoveUseChance, FedDomesticAnimalMoveUseChance,
    // ObjDecayFactorOnFloor, WoolClothDecayTime / RabbitFurClothDecayTime (patch constants),
    // TemperatureHeatObjectFactor / HeatObjFactor / ClothingFactor / LovedBiomeFactor /
    // ShiftFor* / NaturalHeatInsulation / ClothingInsulationFactor (player-heat consts),
    // SeasonBiomeChangeChancePerYear / SeasonBiomeRestoreFactor, MaxSayLength,
    // TicksBetweenSaving / TicksBetweenBackups / MaxNumberOfBackups, ChanceForLuckySpot,
    // CreateGreenBiomeDistance, MinAiAgeForCombat, WoundHealingTimeFactor, NumberOfAiPx,
    // TimeToAiRebirthPerYear, AiNameEnding, AIAllowBuildOven / AIAllowBuilKiln,
    // AIMigrateVillagePopulationSize, MaxAiSkipedTicksBeforeReducingAIs,
    // NewAccountsPerIpPerDay / TotalNewAccountsPerDay, LineageDeleteAgeFactor,
    // saveToDisk / SavePlayers / LoadPlayers, MaxTimeBetweenMapChunks,
    // LetTheClientCheatLittleBitFactor, SemiHeavyItemSpeed, GotoTimeOut,
    // AiCallsPerHour / AiMaxTokensForChat / MaxAIResponseperSay / AIWaitTimePer100Chars,
    // OriginalBiomesFileName / Current*FileName / WebServerDirectory / WebServerMainHtml.
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
    // StartingEveAge / EveOrAdamBirthChance / SpawnAiAsEve / StartingFamilyName /
    // StartingName / FoundFamilyNeededPrestige / FoundFamilyCost /
    // FoundFamilyNeededFollowers / FoundFamilyBreakAllianceChance promoted Live
    // under SETTINGS-LONG-TAIL (see Live block above).
    FieldEntry {
        haxe_name: "DoorIds",
        rust_path: "server.toml door_ids / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AiIgnoredFloorIds",
        rust_path: "server.toml ai_ignored_floor_ids / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
    },
    // --- secrets / LLM (never dump) ---
    FieldEntry {
        haxe_name: "Secret",
        rust_path: "server.toml secret / LiveSettings (admin !S; do not log value)",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "AllowDebugCommmands",
        rust_path: "server.toml allow_debug_commands / LiveSettings (gates DoDebugCommands)",
        home: SettingsHome::Live,
    },
    FieldEntry {
        haxe_name: "DebugSayPlayerPosition",
        rust_path: "server.toml debug_say_player_position / LiveSettings / SimState.gameplay",
        home: SettingsHome::Live,
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
    // --- Haxe threading; Rust is a single-writer sim + NetIntent (never port) ---
    FieldEntry {
        haxe_name: "UseOneGlobalMutex",
        rust_path: "N/A — Rust single-writer SimState, no Haxe mutex soup",
        home: SettingsHome::NotApplicable,
    },
    FieldEntry {
        haxe_name: "UseOneSingleMutex",
        rust_path: "N/A — Rust single-writer SimState",
        home: SettingsHome::NotApplicable,
    },
    FieldEntry {
        haxe_name: "UseExperimentalMutex",
        rust_path: "N/A — Rust single-writer SimState",
        home: SettingsHome::NotApplicable,
    },
    FieldEntry {
        haxe_name: "UseBlockingSockets",
        rust_path: "N/A — ol-net async accept; not Haxe blocking sockets",
        home: SettingsHome::NotApplicable,
    },
    FieldEntry {
        haxe_name: "PlayerResponseSleepTime",
        rust_path: "N/A — no per-connection Haxe sleep; intent drain budget instead",
        home: SettingsHome::NotApplicable,
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
        let mux = find_critical("UseOneGlobalMutex").expect("mutex N/A");
        assert_eq!(mux.home, SettingsHome::NotApplicable);
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
        // SETTINGS-LONG-TAIL: ObjDecayChance / FloorDecayChance Live
        assert!(!residual.contains(&"ObjDecayChance"));
        assert!(!residual.contains(&"FloorDecayChance"));
        assert!(!residual.contains(&"AnimalDecayFactor"));
        assert!(!residual.contains(&"ObjDecayFactorForPermanentObjs"));
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
        // SETTINGS-LONG-TAIL
        assert!(!residual.contains(&"StartingEveAge"));
        assert!(!residual.contains(&"EveOrAdamBirthChance"));
        assert!(!residual.contains(&"SpawnAiAsEve"));
        assert!(!residual.contains(&"ObjDecayChance"));
        assert!(!residual.contains(&"FloorDecayChance"));
        assert!(!residual.contains(&"CursedGraveTime"));
        assert!(!residual.contains(&"AnimalDecayFactor"));
        assert!(!residual.contains(&"ScoreFactor"));
        assert!(!residual.contains(&"AncestorPrestigeFactor"));
        assert!(!residual.contains(&"DisplayScoreFactor"));
        assert!(!residual.contains(&"AiTotalScoreFactor"));
        assert!(!residual.contains(&"OldGraveDecayMali"));
        assert!(!residual.contains(&"CursedGraveMali"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsClose"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCloseForMapChanges"));
        assert!(!residual.contains(&"lastVanillaID"));
        assert!(!residual.contains(&"OpenLifeClientName"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCloseForSay"));
        assert!(!residual.contains(&"SendMoveEveryXTicks"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCoseForMovement"));
        assert!(!residual.contains(&"MaxDistanceToBeConsideredAsCloseForSayAi"));
        assert!(!residual.contains(&"MaxDistanceToAutoExileAttacker"));
        assert!(!residual.contains(&"SpeedWithBothShoes"));
        assert!(!residual.contains(&"AgingFactorWhileStarvingToDeath"));
        assert!(!residual.contains(&"GrownUpAge"));
        assert!(!residual.contains(&"FoodUseChildFaktor"));
        assert!(!residual.contains(&"AIFoodUseFactorSerf"));
        assert!(!residual.contains(&"AIFoodUseFactorCommoner"));
        assert!(!residual.contains(&"AIFoodUseFactorNoble"));
        assert!(!residual.contains(&"EveFoodUseFactor"));
        assert!(!residual.contains(&"AgingFactorHumanBornToAi"));
        assert!(!residual.contains(&"AgingFactorAiBornToHuman"));
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
        assert!(!residual.contains(&"MinSpeedReductionPerContainedObj"));
        assert!(!residual.contains(&"LovedFoodUseChance"));
        assert!(!residual.contains(&"MaxAgeForAllowingClothAndPrickupFromOthers"));
        assert!(!residual.contains(&"MaxAgeForAllowingDie"));
        assert!(!residual.contains(&"PrestigeCostForDie"));
        assert!(!residual.contains(&"StartingFamilyName"));
        assert!(!residual.contains(&"StartingName"));
        assert!(!residual.contains(&"FoundFamilyNeededPrestige"));
        assert!(!residual.contains(&"FoundFamilyCost"));
        assert!(!residual.contains(&"FoundFamilyNeededFollowers"));
        assert!(!residual.contains(&"FoundFamilyBreakAllianceChance"));
        assert!(!residual.contains(&"PickupExhaustionGain"));
        assert!(!residual.contains(&"PickupFeedingFoodRestore"));
        assert!(!residual.contains(&"DeathWithFoodStoreMax"));
        assert!(!residual.contains(&"FoodStoreMaxReductionWhileStarvingToDeath"));
        assert!(!residual.contains(&"TemperatureReductionPerDrinking"));
        assert!(!residual.contains(&"MaxStoredWater"));
        assert!(!residual.contains(&"MaxJumpsPerTenSec"));
        assert!(!residual.contains(&"TemperatureImpactPerSec"));
        assert!(!residual.contains(&"TemperatureImpactPerSecIfGood"));
        assert!(!residual.contains(&"TemperatureInWaterFactor"));
        assert!(!residual.contains(&"TemperatureImpactBelow"));
        assert!(!residual.contains(&"TemperatureImpactColorFactor"));
        assert!(!residual.contains(&"AllowEatingOrFeedingIfIll"));
        assert!(!residual.contains(&"ResistanceAgainstFeverForEatingMushrooms"));
        assert!(!residual.contains(&"ExhaustionYellowFeverPerSec"));
        assert!(!residual.contains(&"MinHealthFoodStoreMaxFactor"));
        assert!(!residual.contains(&"MaxHealthFoodStoreMaxFactor"));
        assert!(!residual.contains(&"MinHealthAgingFactor"));
        assert!(!residual.contains(&"MaxHealthAgingFactor"));
        assert!(!residual.contains(&"MinHealthPerYear"));
        assert!(!residual.contains(&"MaxAge"));
        assert!(!residual.contains(&"AnimalDeadlyDistanceFactor"));
        assert!(!residual.contains(&"ChanceForAnimalDyingFactorIfInLovedBiome"));
        assert!(!residual.contains(&"OffspringFactorIfAnimalPopIsLow"));
        assert!(!residual.contains(&"MaxOffspringFactor"));
        assert!(!residual.contains(&"OffspringFactorLowAnimalPopulationBelow"));
        assert!(!residual.contains(&"DoorIds"));
        assert!(!residual.contains(&"AiIgnoredFloorIds"));
        assert!(!residual.contains(&"ChanceForDomesticAnimalDyingFactor"));
        assert!(!residual.contains(&"CombatAngryTimeMinimum"));
        assert!(!residual.contains(&"Secret"));
        assert!(!residual.contains(&"AllowDebugCommmands"));
        assert!(!residual.contains(&"ObjDecayFactorForFood"));
        assert!(!residual.contains(&"ObjDecayFactorForClothing"));
        assert!(!residual.contains(&"ObjDecayFactorForWalls"));
        assert!(!residual.contains(&"ObjDecayFactorPerTechLevel"));
        assert!(!residual.contains(&"DecayFactorInDeepWater"));
        assert!(!residual.contains(&"DecayFactorInMountain"));
        assert!(!residual.contains(&"DecayFactorInWalkableWater"));
        assert!(!residual.contains(&"DecayFactorInJungle"));
        assert!(!residual.contains(&"DecayFactorInSwamp"));
        assert!(!residual.contains(&"ObjRespawnChance"));
        assert!(!residual.contains(&"GrowBackPlantsIncreaseIfLowPopulation"));
        assert!(!residual.contains(&"GrowBackOriginalPlantsFactor"));
        assert!(!residual.contains(&"GrowNewPlantsFromExistingFactor"));
        assert!(!residual.contains(&"MaxPlayersBeforeStartingAsChild"));
        assert!(!residual.contains(&"SpringWildFoodRegrowChance"));
        assert!(!residual.contains(&"WinterWildFoodDecayChance"));
        assert!(!residual.contains(&"HotSeasonTemperatureFactor"));
        assert!(!residual.contains(&"ColdSeasonTemperatureFactor"));
        // SETTINGS-KNOB-TAIL
        assert!(!residual.contains(&"DisplayScoreOn"));
        assert!(!residual.contains(&"MaxCoinsPerChest"));
        assert!(!residual.contains(&"MaxCoinsPerPouch"));
        assert!(!residual.contains(&"ChanceForFemaleChild"));
        assert!(!residual.contains(&"ChanceForOtherChildColor"));
        assert!(!residual.contains(&"ChanceForOtherChildColorIfCloseToWrongSpecialBiome"));
        assert!(!residual.contains(&"LittleKidsPerMother"));
        assert!(!residual.contains(&"NewChildExhaustionForMother"));
        assert!(!residual.contains(&"AiMotherBirthMaliForHumanChild"));
        assert!(!residual.contains(&"HumanMotherBirthMaliForAiChild"));
        assert!(!residual.contains(&"SpwanAtLastDead"));
        assert!(!residual.contains(&"TemperatureOwnTileRate"));
        assert!(!residual.contains(&"TemperatureBalanceRate"));
        assert!(!residual.contains(&"TemperatureLocalHeatFactor"));
        assert!(!residual.contains(&"AverageSeasonTemperatureImpact"));
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
            // SETTINGS-LONG-TAIL
            "StartingEveAge",
            "EveOrAdamBirthChance",
            "SpawnAiAsEve",
            "ObjDecayChance",
            "FloorDecayChance",
            "CursedGraveTime",
            "AnimalDecayFactor",
            "ObjDecayFactorForPermanentObjs",
            "ScoreFactor",
            "AncestorPrestigeFactor",
            "DisplayScoreFactor",
            "AiTotalScoreFactor",
            "OldGraveDecayMali",
            "CursedGraveMali",
            "MaxDistanceToBeConsideredAsClose",
            "MaxDistanceToBeConsideredAsCloseForMapChanges",
            "lastVanillaID",
            "OpenLifeClientName",
            "MaxDistanceToBeConsideredAsCloseForSay",
            "SendMoveEveryXTicks",
            "MaxDistanceToBeConsideredAsCoseForMovement",
            "MaxDistanceToBeConsideredAsCloseForSayAi",
            "MaxDistanceToAutoExileAttacker",
            "SpeedWithBothShoes",
            "AgingFactorWhileStarvingToDeath",
            "GrownUpAge",
            "FoodUseChildFaktor",
            "AIFoodUseFactorSerf",
            "AIFoodUseFactorCommoner",
            "AIFoodUseFactorNoble",
            "EveFoodUseFactor",
            "AgingFactorHumanBornToAi",
            "AgingFactorAiBornToHuman",
            "EveDamageFactor",
            "TargetWoundedDamageFactor",
            "MaleDamageFactor",
            "AnimalDamageFactor",
            "AnimalDamageFactorInWinter",
            "AnimalDamageFactorIfAttacked",
            "WeaponDamageFactor",
            "GraveBlockingDistance",
            "MaxPlayersBeforeActivatingGraveCurse",
            "MaxPlayersBeforeForbidTouchGrave",
            "CombatAngryTimeBeforeAttack",
            "CombatAngryTimeMinimum",
            "ChanceForDomesticAnimalDyingFactor",
            "DoorIds",
            "AiIgnoredFloorIds",
            "Secret",
            "AllowDebugCommmands",
            "AiTimeToWaitIfCraftingFailed",
            "AiMemoryMaxEntries",
            "AiChatMemoryMaxEntries",
            "AiMaxSearchRadius",
            "AiMaxSearchIncrement",
            "AiIgnoreTimeTransitionsLongerThen",
            "AlternativeOutcomePercentIncreasePerHit",
            "AlternativeOutcomeHitsDecreaseOnSucess",
            "FortificationCosePerHit",
            "ReduceAgeNeededToPickupObjects",
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
            "GrowNewPlantsFromExistingFactor",
            "MaxPlayersBeforeStartingAsChild",
            "SpringWildFoodRegrowChance",
            "WinterWildFoodDecayChance",
            "HotSeasonTemperatureFactor",
            "ColdSeasonTemperatureFactor",
            // SETTINGS-KNOB-TAIL
            "DisplayScoreOn",
            "MaxCoinsPerChest",
            "MaxCoinsPerPouch",
            "ChanceForFemaleChild",
            "ChanceForOtherChildColor",
            "ChanceForOtherChildColorIfCloseToWrongSpecialBiome",
            "LittleKidsPerMother",
            "NewChildExhaustionForMother",
            "AiMotherBirthMaliForHumanChild",
            "HumanMotherBirthMaliForAiChild",
            "SpwanAtLastDead",
            "TemperatureOwnTileRate",
            "TemperatureBalanceRate",
            "TemperatureLocalHeatFactor",
            "AverageSeasonTemperatureImpact",
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
        // SETTINGS-LONG-TAIL Haxe StartingEveAge = 14
        assert!((gameplay_defaults::STARTING_EVE_AGE - 14.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::EVE_OR_ADAM_BIRTH_CHANCE - 0.025).abs() < 1e-12);
        assert!(!gameplay_defaults::SPAWN_AI_AS_EVE);
        assert!((gameplay_defaults::OBJ_DECAY_CHANCE - 0.00005).abs() < 1e-12);
        assert!((gameplay_defaults::FLOOR_DECAY_CHANCE - 0.00001).abs() < 1e-12);
        assert!((gameplay_defaults::OBJ_RESPAWN_CHANCE - 0.00006).abs() < 1e-12);
        assert!(
            (gameplay_defaults::GROW_BACK_PLANTS_INCREASE_IF_LOW_POPULATION - 2.0).abs() < 1e-12
        );
        assert!((gameplay_defaults::GROW_BACK_ORIGINAL_PLANTS_FACTOR - 0.02).abs() < 1e-12);
        assert!((gameplay_defaults::GROW_NEW_PLANTS_FROM_EXISTING_FACTOR - 0.05).abs() < 1e-12);
        assert_eq!(gameplay_defaults::MAX_PLAYERS_BEFORE_STARTING_AS_CHILD, 0);
        assert!((gameplay_defaults::SPRING_WILD_FOOD_REGROW_CHANCE - 1.0).abs() < 1e-12);
        assert!((gameplay_defaults::WINTER_WILD_FOOD_DECAY_CHANCE - 1.5).abs() < 1e-12);
        assert!((gameplay_defaults::HOT_SEASON_TEMPERATURE_FACTOR - 0.75).abs() < 1e-12);
        assert!((gameplay_defaults::CURSED_GRAVE_TIME - 12.0).abs() < f32::EPSILON);
        assert!((gameplay_defaults::ANIMAL_DECAY_FACTOR - 0.05).abs() < 1e-12);
        assert!((gameplay_defaults::SCORE_FACTOR - 0.2).abs() < 1e-6);
        assert!(
            (gameplay_defaults::MIN_SPEED_REDUCTION_PER_CONTAINED_OBJ - 0.98).abs() < 1e-12
        );
        assert!((gameplay_defaults::LOVED_FOOD_USE_CHANCE - 0.5).abs() < 1e-12);
        assert!(
            (gameplay_defaults::MAX_AGE_FOR_ALLOWING_CLOTH_AND_PICKUP_FROM_OTHERS - 10.0).abs()
                < 1e-12
        );
        assert!((gameplay_defaults::MAX_AGE_FOR_ALLOWING_DIE - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::PRESTIGE_COST_FOR_DIE - 0.0).abs() < 1e-12);
        assert_eq!(gameplay_defaults::STARTING_FAMILY_NAME, "SNOW");
        assert_eq!(gameplay_defaults::STARTING_NAME, "SPOON");
        assert!((gameplay_defaults::FOUND_FAMILY_NEEDED_PRESTIGE - 50.0).abs() < 1e-12);
        assert!((gameplay_defaults::FOUND_FAMILY_COST - 10.0).abs() < 1e-12);
        assert_eq!(gameplay_defaults::FOUND_FAMILY_NEEDED_FOLLOWERS, 4);
        assert!((gameplay_defaults::FOUND_FAMILY_BREAK_ALLIANCE_CHANCE - 0.5).abs() < 1e-12);
        assert!((gameplay_defaults::PICKUP_EXHAUSTION_GAIN - 0.2).abs() < 1e-12);
        assert!((gameplay_defaults::PICKUP_FEEDING_FOOD_RESTORE - 1.5).abs() < 1e-12);
        assert!((gameplay_defaults::DEATH_WITH_FOOD_STORE_MAX + 0.1).abs() < 1e-12);
        assert!(
            (gameplay_defaults::FOOD_STORE_MAX_REDUCTION_WHILE_STARVING - 5.0).abs() < 1e-12
        );
        assert!((gameplay_defaults::MIN_HEALTH_FOOD_STORE_MAX_FACTOR - 0.8).abs() < 1e-12);
        assert!((gameplay_defaults::MAX_HEALTH_FOOD_STORE_MAX_FACTOR - 1.2).abs() < 1e-12);
        assert!((gameplay_defaults::MIN_HEALTH_AGING_FACTOR - 0.5).abs() < 1e-12);
        assert!((gameplay_defaults::MAX_HEALTH_AGING_FACTOR - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::MIN_HEALTH_PER_YEAR - 1.0).abs() < 1e-12);
        assert!((gameplay_defaults::MAX_AGE - 60.0).abs() < 1e-12);
        assert!((gameplay_defaults::ANIMAL_DEADLY_DISTANCE_FACTOR - 0.5).abs() < 1e-12);
        assert!(
            (gameplay_defaults::CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME - 0.1).abs()
                < 1e-12
        );
        assert!((gameplay_defaults::OFFSPRING_FACTOR_IF_ANIMAL_POP_IS_LOW - 10.0).abs() < 1e-12);
        assert!((gameplay_defaults::MAX_OFFSPRING_FACTOR - 1.0).abs() < 1e-12);
        assert!(
            (gameplay_defaults::OFFSPRING_FACTOR_LOW_ANIMAL_POPULATION_BELOW - 0.2).abs() < 1e-12
        );
        assert!((gameplay_defaults::OBJ_DECAY_FACTOR_FOR_FOOD - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::OBJ_DECAY_FACTOR_FOR_CLOTHING - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::OBJ_DECAY_FACTOR_FOR_WALLS - 0.2).abs() < 1e-12);
        assert!((gameplay_defaults::OBJ_DECAY_FACTOR_PER_TECH_LEVEL - 10.0).abs() < 1e-12);
        assert!((gameplay_defaults::DECAY_FACTOR_IN_DEEP_WATER - 5.0).abs() < 1e-12);
        assert!((gameplay_defaults::DECAY_FACTOR_IN_MOUNTAIN - 3.0).abs() < 1e-12);
        assert!((gameplay_defaults::DECAY_FACTOR_IN_WALKABLE_WATER - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::DECAY_FACTOR_IN_JUNGLE - 2.0).abs() < 1e-12);
        assert!((gameplay_defaults::DECAY_FACTOR_IN_SWAMP - 2.0).abs() < 1e-12);
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
