package openlife.settings;

import openlife.auto.AiBase;
import haxe.Exception;
import openlife.data.object.ObjectData;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.server.Biome.BiomeTag;
import openlife.server.Lineage.PrestigeClass;
import sys.FileSystem;
import sys.io.File;

using StringTools;

@:rtti
class ServerSettings {
	public static var Secret = 'JASON';

	// DEBUG: switch on / off
	public static var dumpOutput = false;
	public static var debug = false; // activates or deactivates try catch blocks
	public static var AllowDebugObjectCreation = false; // allow debug objects creation with '!CREATEALL' and generates dubug object on start
	public static var DebugSend = false;
	public static var DebugIncomingCommands = false;
	public static var DebugTransitionHelper = false;
	public static var DebugMoveHelper = false;
	public static var DebugSpeed = false; // MovementHelper
	public static var DebugEating = false;
	public static var DebugCombat = false;
	public static var DebugPlayer = false;
	public static var DebugSayPlayerPosition = false;

	public static var AllowDebugCommmands = true; // can create objects with saying "!create ID" / "!create object" "!create object!" with ! indicating that object ends with "object" or test wounds with using "!hit" or "!heal"
	public static var DebugWrite = true; // WordMap writeToDisk
	public static var TraceCountObjects = false; // WorldMap
	public static var TraceCountObjectsToDisk = true; // WorldMap

	public static var DebugCaftingStepsForObjOrFood = false; // here you see which food or obj needs how much steps to craft

	// Debug AI
	public static var DebugAi:Bool = false;
	public static var DebugAiSay:Bool = false;
	public static var DebugProfession:Bool = false;
	public static var DebugAiGoto:Bool = false;
	public static var DebugAiCrafting:Bool = false;
	public static var DebugAiCraftingObject:Int = 999999; // 57;
	public static var AutoFollowAi:Bool = false;
	public static var AutoFollowPlayer:Bool = false;

	// Debug pathing
	public static var DebugCreateOldPath:Bool = false; // To compare new pathing method with old
	public static var DebugWritePathToFile:Bool = false; // wriths path map to SaveFiles/paths.txt

	// Save / Load
	public static var saveToDisk = true;
	public static var SavePlayers = true;
	public static var LoadPlayers = true;
	public static var LineageDeleteAgeFactor:Float = 1.5; // keep lineage for age * x days --> one died with 60 will be deleted after 90 days

	// Mutex
	public static var UseOneGlobalMutex = false; // if you want to try out if there a problems with mutexes / different threads
	public static var UseOneSingleMutex = true; // might get stuck if set true
	public static var UseBlockingSockets = false;
	public static var UseExperimentalMutex = false; // try out a new way of using mutexes

	// DEBUG: used to trace connection.send commands
	public static var TraceSendPlayerActions = false; //  only trace player actions // ignores MX from animal, FX and PU from food / age
	public static var TraceSendNonPlayerActions = false; //  only trace non player actions // traces only MX from animal, FX and PU from food / age

	// DEBUG: TransitionImporter // for debugging transitions
	public static var traceTransitionByActorId = 99934; // set to object id which you want to debug
	public static var traceTransitionByNewActorId = 99934; // set to object id which you want to debug
	public static var traceTransitionByActorDescription = "!!!Basket of Soil"; // set to object description which you want to debug
	public static var traceTransitionByTargetId = 9991872; // set to object id which you want to debug
	public static var traceTransitionByNewTargetId = 9991099; // set to object id which you want to debug
	public static var traceTransitionByTargetDescription = "!!!Basket of Soil"; // set to object description which you want to debug

	// Timing
	public static var PlayerResponseSleepTime:Float = 0.02; // secs between each player command

	// score
	public static var BirthPrestigeFactor:Float = 0.4; // TODO set 0.2 if fathers are implemented // on birth your starting prestige is factor X * total prestige
	public static var AncestorPrestigeFactor:Float = 0.2; // if one dies the ancestors get factor X prestige of the dead
	public static var ScoreFactor:Float = 0.2; // new score influences total score with factor X.
	public static var OldGraveDecayMali:Float = 20; // prestige mali if bones decay without beeing proper burried
	public static var CursedGraveMali:Float = 2; // TODO no need

	// Display
	public static var DisplayScoreOn:Bool = true; // only end of life
	public static var DisplayScoreFactor:Float = 1; // if display score multiply with factor X
	public static var DisplayYumAndMehFood = false;
	public static var DisplayPlayerNamesDistance = 30; // set zero to deactivate
	public static var DisplayPlayerNamesShowDistance = true;
	public static var DisplayPlayerNamesMaxPlayer = 1;
	public static var DisplayTemperatureHintsPerMinute:Float = 1;

	// message
	public static var SecondsBetweenMessages:Float = 5;

	// coins
	public static var InheritCoinsFactor:Float = 0.8; // on death X coins are inherited
	public static var CoinsOnWoundingFactor:Float = 0.5; // If you manage to wound somebody
	public static var MinPrestiegeFromCoinDecayPerYear:Float = 10; // 5

	// birth
	public static var NewChildExhaustionForMother = 0;
	public static var LittleKidsPerMother = 3;
	public static var ChanceForFemaleChild = 0.6;
	public static var ChanceForOtherChildColor = 0.2;
	public static var ChanceForOtherChildColorIfCloseToWrongSpecialBiome = 0.3; // for example Black born in or close to Jungle
	public static var AiMotherBirthMaliForHumanChild = 3; // Means in average an AI mother for an human child is only considered after X children
	public static var HumanMotherBirthMaliForAiChild = 1; // Means in average a human mother for an ai child is only considered after X children

	// Graves
	public static var GraveBlockingDistance = 40; // cannot incrante close to blocking graves like bone pile
	public static var CloseGraveSpeedMali:Float = 0.9; // speed maili if close to blocking grave like bone pile
	public static var CursedGraveTime:Float = 12; // 12 // hours a cursed grave continues to exist per curse token
	public static var MaxPlayersBeforeActivatingGraveCurse = 0; // 2
	public static var MaxPlayersBeforeForbidTouchGrave = 9999; // 2

	// PlayerInstance
	public static var MaxPlayers = 100; // AI plus Humans
	public static var MaxPlayersBeforeStartingAsChild = 0; // -1
	public static var StartingFamilyName = "SNOW";
	public static var StartingName = "SPOON";
	public static var AgeingSecondsPerYear = 60; // 60
	public static var ReduceAgeNeededToPickupObjects = 10; // reduces the needed age that an item can be picked up. But still it cant be used if age is too low
	public static var MaxAgeForAllowingClothAndPrickupFromOthers = 10;
	public static var MaxChildAgeForBreastFeeding = 6; // also used for considering a child when being attacked
	public static var PickupBabyMaxDistance:Float = 1.9;
	public static var PickupFeedingFoodRestore:Float = 1.5;
	public static var PickupExhaustionGain:Float = 0.2;
	public static var FoodRestoreFactorWhileFeeding:Float = 10;
	public static var MinAgeFertile = 14; // TODO only make lower then 14 if client allows it
	public static var MaxAgeFertile = 42;
	public static var MaxSayLength = 80;

	// save to disk
	public static var TicksBetweenSaving = 600; // 600 // 200// in ticks 200 = 10 sec
	public static var TicksBetweenBackups = 20 * 60 * 60 * 8; // 20 * 60 * 60 * 8 = every 8 hours
	public static var MaxNumberOfBackups = 10;

	public static var MapFileName = "mysteraV1Test.png";
	public static var SaveDirectory = "SaveFiles";
	public static var WebServerDirectory = "WebServer";
	public static var OriginalBiomesFileName = "OriginalBiomes"; // .bin is added
	public static var CurrentBiomesFileName = "CurrentBiomes"; // .bin is added
	public static var CurrentFloorsFileName = "CurrentFloors"; // .bin is added
	public static var OriginalObjectsFileName = "OriginalObjects"; // .bin is added
	public static var CurrentObjectsFileName = "CurrentObjects"; // .bin is added
	public static var CurrentObjHelpersFileName = "CurrentObjHelper"; // .bin is added

	// worldMap
	public static var GenerateMapNew = false;
	public static var ChanceForLuckySpot = 0.1; // chance that during generation an object is lucky and tons more of that are generated close by
	public static var CreateGreenBiomeDistance = 5;

	// Eve spawning
	public static var StartingEveAge = 14; // 14
	public static var SpwanAtLastDead = false;
	public static var SpawnAiAsEve = false; // Allows AIs to spawn as Eve even if there are valid mothers
	public static var EveOrAdamBirthChance = 0.025; // since each eve gets an adam the true chance is x2
	public static var startingGx = 235; // 235; //270; // 360;
	public static var startingGy = 150; // 200;//- 400; // server map is saved y inverse
	public static var EveDamageFactor:Float = 1; // Eve / Adam get less damage from animals but make also less damage
	public static var EveFoodUseFactor:Float = 1; // Eve / Adam life still in paradise, so they need less food

	// /DIE stuff
	public static var MaxAgeForAllowingDie:Float = 2;
	public static var PrestigeCostForDie:Float = 0;

	// food stuff / healing / exhaustion recover
	// 0.1 = in bad temperature 10 sec per pip + damage / in temperature good 20 sec per pipe
	// vanilla has around 0.143 (7 sec) with bad temperature and 0.048 (21 sec) with good
	public static var FoodUsePerSecond = 0.10;
	public static var HealingPerSecond = 0.10;
	public static var WoundHealingFactor:Float = 1;
	public static var ExhaustionHealingFactor:Float = 1.5; //
	public static var ExhaustionHealingForMaleFaktor:Float = 1.2;

	public static var FoodFactor:Float = 1; // 0.8 // reduces gained food value
	public static var FoodFactorEatenMoreThanEightPercent:Float = 0.8;
	public static var FoodFactorEatenMoreThanTenPercent:Float = 0.5;
	public static var FoodFactorEatenLessThanFivePercent:Float = 1.5;
	public static var FoodFactorEatenLessThanThreePercent:Float = 2;
	public static var FoodFactorEatenLessThanOnePercent:Float = 2.5;
	public static var FoodReductionPerEating:Float = 1;
	public static var FoodReductionFaktorForEatingMeh:Float = 0.2;
	public static var FoodReductionFaktorForEatingHighQuailitFood:Float = 0.8;
	public static var MaxAge = 60;
	public static var MinAgeToEat = 3; // MinAgeToEat and MinAgeFor putting on cloths on their own
	public static var GrownUpFoodStoreMax = 20; // defaul vanilla: 20
	public static var NewBornFoodStoreMax = 4;
	public static var OldAgeFoodStoreMax = 10;
	public static var DeathWithFoodStoreMax:Float = -0.1; // Death through starvation if food store max reaches below XX
	public static var FoodUseChildFaktor:Float = 1; // children need X times food if below GrownUpAge
	public static var YumBonus = 5; // old 5 // First time eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
	public static var MaxHasEatenForNextGeneration:Float = 4; // 2; // used in InheritEatenFoodCounts --> if food should still be yum at least one set one lower them YumBonus
	public static var HasEatenReductionForNextGeneration:Float = 1; // 0.2 // used in InheritEatenFoodCounts
	public static var YumFoodRestore = 0.8; // XX pipes are restored from a random eaten food. Zero are restored if random food is the current eaten food
	public static var LovedFoodRestore:Float = 0.1; // restore also some loved food like bana for brown
	public static var YumNewCravingChance = 0.2; // XX chance that a new random craving is chosen even if there are existing ones
	public static var HealthLostWhenEatingMeh:Float = 0.5;
	public static var HealthLostWhenEatingSuperMeh:Float = 2;

	// Biome Specialists
	public static var LovedFoodUseChance:Float = 0.5;
	public static var BiomeAnimalHitChance:Float = 0.0; // for example if a biome animal like a wolf can hit a white in mountain

	// Yellow Fever
	public static var ExhaustionYellowFeverPerSec = 0.1;
	public static var AllowEatingOrFeedingIfIll = false; // for example if you have yellow fever some one needs to feed you if false
	public static var ResistanceAgainstFeverForEatingMushrooms:Float = 0.2;

	// health
	public static var MinHealthPerYear = 1; // expected health per year for normal health
	public static var MinHealthFoodStoreMaxFactor:Float = 0.8; // 0.5
	public static var MaxHealthFoodStoreMaxFactor:Float = 1.2; // 1.5
	public static var MinHealthAgingFactor:Float = 0.5; // 0.5
	public static var MaxHealthAgingFactor:Float = 2; // 2

	// starving to death
	public static var AgingFactorWhileStarvingToDeath = 0.5; // if starving to death aging is slowed factor XX up to GrownUpAge, otherwise aging is speed up factor XX
	public static var GrownUpAge = 14; // is used for AgingFactorWhileStarvingToDeath and for increase food need for children
	public static var FoodStoreMaxReductionWhileStarvingToDeath = 5; // (5) reduces food store max with factor XX for each food below 0

	// TODO /LEADER crashes if client does not get player update
	public static var SendMoveEveryXTicks = -1; // -1 // default 90 // set negative to deactive. if MaxDistanceToBeConsideredAsClose it might be deactivated
	public static var MaxDistanceToBeConsideredAsClose = 2000000; // 20; // only close players are updated with PU Movement
	public static var MaxDistanceToBeConsideredAsCoseForMovement = 30; // 20; // only close players are updated with PU Movement
	public static var MaxDistanceToBeConsideredAsCloseForMapChanges = 10; // for MX
	public static var MaxDistanceToBeConsideredAsCloseForSay = 20; // if a player says something
	public static var MaxDistanceToBeConsideredAsCloseForSayAi = 20; // if a player says something
	public static var MaxDistanceToAutoExileAttacker = 15; // Exile attacker if seen by close by ally of attacker

	// for movement
	public static var GotoTimeOut:Int = 250;
	public static var InitialPlayerMoveSpeed:Float = 3.75; // vanilla: 3.75; // in Tiles per Second
	public static var SpeedFactor:Float = 1; // MovementExtender // used to incease or deacrease speed factor X
	// Factor 3: between 66% and 106% for 120% hitpoints
	// Factor 5: between 80% and 104% for 120% hitpoints
	public static var HitpointsSpeedFactor:Float = 3; // set 0 if hitpoints should have no speed influence
	public static var MinBiomeSpeedFactor:Float = 0.2; // For example if you happen to end up in a ocean or on a mountain
	public static var SpeedWithBothShoes:Float = 1.1; // if wearing both shoes get this speed boni
	public static var MinMovementAgeInSec:Float = 14;
	public static var MinSpeedReductionPerContainedObj = 0.98;
	public static var CloseEnemyWithWeaponSpeedFactor:Float = 0.8;
	public static var SemiHeavyItemSpeed:Float = 0.9; // slows down if carring iron / logs / soil etc.
	public static var MaxTimeBetweenMapChunks:Float = 3; // make sure that every X seconds at least one map chunk is send

	// since client does not seem to use exact positions allow little bit cheating / JUMPS
	public static var LetTheClientCheatLittleBitFactor = 1.1; // when considering if the position is reached, allow the client to cheat little bit, so there is no lag
	public static var MaxMovementQuadJumpDistanceBeforeForce:Float = 5; // if quadDistance between server and client position is bigger then X the client is forced to use server position
	public static var MaxJumpsPerTenSec:Float = 10; // limit how often a client can JUMP / cheat his position
	public static var ExhaustionOnJump:Float = 0.05;

	// hungry work
	public static var HungryWorkCost:Float = 5; // 10
	public static var HungryWorkHeat:Float = 0.002; // 0.005; // per food used
	public static var HungryWorkToolCostFactor:Float = 0;

	// property
	public static var MaxCoinsPerChest:Int = 200;
	public static var MaxCoinsPerPouch:Int = 50;
	public static var LockpickSucessChance:Float = 5; // in %
	public static var LockpickFailChance:Float = 10;
	public static var LockpickExhaustionCost:Float = 3;
	public static var LockpickCoinCost:Float = 1;

	// fortification
	public static var FortificationCosePerHit:Float = 1;

	// first the chance for success would be - then 10% then 20% usw ... 10 hits 100%
	public static var AlternativeOutcomePercentIncreasePerHit:Float = 10; // for example used for extra wood for trees or stone form iron mining
	// once succeeded in cutting the tree / mining the hits is ruced by 5
	public static var AlternativeOutcomeHitsDecreaseOnSucess = 5; // for example used for extra wood for trees or stone form iron mining

	public static var TeleportCost:Float = 5; // 10
	public static var HireCost:Float = 10;
	public static var HireCostIncreasePerPerson:Float = 10;

	public static var FoundFamilyNeededPrestige:Float = 50; // 100
	public static var FoundFamilyNeededFollowers:Float = 4;
	public static var FoundFamilyCost:Float = 10; // 100
	public static var FoundFamilyBreakAllianceChance:Float = 0.5; // 100

	// for animal movement
	public static var ChanceThatAnimalsCanPassBlockingBiome:Float = 0.03;
	public static var chancePreferredBiome:Float = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
	public static var AnimalDeadlyDistanceFactor:Float = 0.5; // How close a animal must be to make a hit
	public static var DomesticAnimalMoveUseChance:Float = 0.2;
	public static var FedDomesticAnimalMoveUseChance:Float = 0.66;

	// for animal offsprings and death
	public static var ChanceForOffspring:Float = 0.00005; // 0.00005;// 0.0005 // For each movement there is X chance to generate an offspring.
	public static var ChanceForAnimalDying:Float = 0.00005; // 0.05 // 0.00002 // 0.00025 // For each movement there is X chance that the animal dies
	public static var ChanceForDomesticAnimalDyingFactor:Float = 2; // Is used in combination with ChanceForAnimalDying
	public static var ChanceForAnimalDyingFactorIfInLovedBiome:Float = 0.1; // Animals die less if they are in their loved biome
	public static var OffspringFactorLowAnimalPopulationBelow:Float = 0.2;
	public static var OffspringFactorIfAnimalPopIsLow:Float = 10;
	public static var MaxOffspringFactor:Float = 1; // The population can only be at max X times the initial population

	// world decay / respawm
	public static var WorldTimeParts = 25; // TODO better auto calculate on time used // in each tick 1/XX DoTimeSuff is done for 1/XX part of the map. Map height should be dividable by XX * 10
	public static var ObjRespawnChance:Float = 0.00006; // 0.002; 17 hours // In each 20sec (WorldTimeParts/20 * 10) there is a X chance to generate a new object if number is less then original objects
	public static var ObjDecayChance:Float = 0.00005; // 0.00005;
	public static var FloorDecayChance:Float = 0.00001; // 0.00001
	public static var ObjDecayFactorOnFloor:Float = 0.2; // 0.1 // only used for fences
	public static var ObjDecayFactorForWalls:Float = 0.2;
	public static var ObjDecayFactorForPermanentObjs:Float = 0.2; // 0.1 // 0.05;
	public static var ObjDecayFactorForFood:Float = 2;
	public static var ObjDecayFactorForClothing:Float = 2;
	public static var AnimalDecayFactor:Float = 0.05;
	public static var ObjDecayFactorPerTechLevel:Float = 10; // 20 // for example a knife with 58 steps with a tech level of 20 holds round about 4 times longer
	public static var WoolClothDecayTime:Int = -24 * 30; // one month
	public static var RabbitFurClothDecayTime:Int = -24 * 2; // vanilla: -5

	public static var DecayFactorInDeepWater:Float = 5;
	public static var DecayFactorInMountain:Float = 3;
	public static var DecayFactorInWalkableWater:Float = 2;
	public static var DecayFactorInJungle:Float = 2;
	public static var DecayFactorInSwamp:Float = 2;

	// Temperature
	public static var DebugTemperature = false;
	public static var TemperatureHeatObjectFactor:Float = 1.5; // impact of fire and ice stuff
	public static var TemperatureHitsDamageFactor:Float = 0.5; // 0.25
	public static var TemperatureExhaustionDamageFactor:Float = 0.2;
	public static var TemperatureImpactPerSec:Float = 0.03;
	public static var TemperatureImpactPerSecIfGood:Float = 0.06; // if outside temperature is helping to et closer to optimal
	public static var TemperatureInWaterFactor:Float = 1.5;
	public static var TemperatureReductionPerDrinking:Float = 0.5;
	public static var MaxStoredWater:Float = 1;
	public static var TemperatureNaturalHeatInsulation:Float = 0.5; // gives X extra natural insulation against heat
	public static var TemperatureClothingInsulationFactor:Float = 5; // with 100% insulation 5X more time 10X with 200% insulation / only if temperature change is ositive
	public static var TemperatureImpactReduction:Float = 0.4; // reduces the impact of bad temperature if temperature is already bad
	public static var TemperatureHeatObjFactor:Float = 1; // increase or decrase impact of a head obj like fire
	public static var TemperatureLovedBiomeFactor:Float = 1;
	public static var TemperatureMaxLovedBiomeImpact:Float = 0.1; // max 0.1 better in loved biome (right color or borh parents right color)
	public static var TemperatureSpeedImpact:Float = 1.0; // 0.0 // speed * X: double impact if extreme temperature
	public static var TemperatureImpactBelow:Float = 0.6; // take damage and display emote if temperature is below or above X from normal
	public static var TemperatureImpactColorFactor:Float = 0.5; // set zero if super hot an super cold are equal for all colors
	public static var TemperatureClothingFactor:Float = 0.1; // 0.2

	public static var TemperatureShiftForBlack:Float = 0.1; // ideal temperature = 0.6
	public static var TemperatureShiftForBrown:Float = 0.05;
	public static var TemperatureShiftForWhite:Float = -0.05;
	public static var TemperatureShiftForGinger:Float = -0.1;

	// winter / summer
	public static var DebugSeason:Bool = false;
	public static var SeasonDuration:Float = 7.5; // default: 5 // Season duration like winter in years
	public static var SeasonBiomeChangeChancePerYear:Float = 2; // 5 // X means it spreads X tiles per year in average in each direction
	public static var SeasonBiomeRestoreFactor:Float = 2;
	public static var AverageSeasonTemperatureImpact:Float = 0.2;
	public static var HotSeasonTemperatureFactor:Float = 0.75; // 0.5
	public static var ColdSeasonTemperatureFactor:Float = 0.75; // 0.5

	public static var WinterWildFoodDecayChance:Float = 1.5; // 1.5; // per Season
	public static var SpringWildFoodRegrowChance:Float = 1; // per Season // use spring and summer
	public static var GrowBackPlantsIncreaseIfLowPopulation:Float = 2;
	public static var GrowBackOriginalPlantsFactor:Float = 0.02; // 0.05; // 0.05; // 0.4 // 0.1 // regrow from original plants per season
	public static var GrowNewPlantsFromExistingFactor:Float = 0.05; // 0.1; // 0.2 // offsprings per season per plant

	// public static var WinterFildWoodDecayChance = 0.2;
	// Ally
	public static var TimeConfirmNewFollower:Float = 15; // a new follower is confirmed after X seconds

	// combat
	public static var MinAiAgeForCombat = 8;
	public static var CombatAngryTimeBeforeAttack:Float = 5;
	public static var CombatAngryTimeMinimum:Float = -60;
	public static var CombatReputationRestorePerYear:Float = 2;
	public static var CombatExhaustionCostPerAttack:Float = 0.1;
	public static var MaleDamageFactor:Float = 1.2;
	public static var WeaponCoolDownFactor:Float = 0.5; // 0.05;
	public static var WeaponCoolDownFactorIfWounding:Float = 5; // 0.4;
	// public static var AnimalCoolDownFactorIfWounding:Float = 0.2;
	public static var AnimalDamageFactor:Float = 1.5; // 1.5
	public static var AnimalDamageFactorInWinter:Float = 2; // 2
	public static var AnimalDamageFactorIfAttacked:Float = 1.5;
	public static var WeaponDamageFactor:Float = 1;
	public static var WoundDamageFactor:Float = 1;
	public static var CursedReceiveDamageFactor:Float = 1.2;
	public static var CursedMakeDamageFactor:Float = 0.5;
	public static var TargetWoundedDamageFactor:Float = 0.2;
	public static var AllyConsideredClose:Int = 5;
	public static var WoundHealingTimeFactor:Float = 2;
	public static var AllyStrenghTooLowForPickup:Float = 0; // 0.8
	public static var PrestigeCostPerDamageForAlly:Float = 1; // 0. 5 // For damaging ally
	public static var PrestigeCostPerDamageForChild:Float = 5; // 2
	public static var PrestigeCostPerDamageForElderly:Float = 1; // for attacking elderly
	public static var PrestigeCostPerDamageForCloseRelatives:Float = 0.5; // 0.25// For attacking children, mother, father, brother sister
	public static var PrestigeCostPerDamageForWomenWithoutWeapon:Float = 0.5; // 0.25

	// AI
	public static var NumberOfAis:Int = 40; // 50
	public static var MinNumberOfAis:Int = 20; // even if server is too slow use MinNumberOfAis
	public static var MaxAiSkipedTicksBeforeReducingAIs:Int = 10;
	public static var NumberOfAiPx:Int = 0;
	public static var AiReactionTime:Float = 0.5; // 0.5; // 0.5; // Commoner
	public static var AiReactionTimeSerf:Float = 0.7;
	public static var AiReactionTimeNoble:Float = 0.2;
	public static var AiReactionTimeFactorIfAngry:Float = 0.2;
	public static var TimeToAiRebirthPerYear:Float = 10; // X seconds per not lived year = 60 - death age
	public static var AiTotalScoreFactor:Float = 0.8;
	public static var AiTimeToWaitIfCraftingFailed:Float = 15; // if item failed to craft dont craft for X seconds
	public static var AiMaxSearchRadius:Int = 60;
	public static var AiMaxSearchIncrement:Int = 30; // 16
	public static var AiIgnoreTimeTransitionsLongerThen:Int = 120; // 30
	public static var AgingFactorHumanBornToAi:Float = 3; // 3
	public static var AgingFactorAiBornToHuman:Float = 1.5;
	public static var AiNameEnding:String = 'X'; // A name ending / set '' if none
	public static var AIAllowBuildOven:Bool = false;
	public static var AIAllowBuilKiln:Bool = false;
	public static var AIMigrateVillagePopulationSize:Int = 10; // AI tries to migrate if the population reached this limit. Starving people are counted twice

	// Ai speed
	public static var AISpeedFactorSerf:Float = 0.8;
	public static var AISpeedFactorCommoner:Float = 0.9;
	public static var AISpeedFactorNoble:Float = 1;

	// Ai food use
	public static var AIFoodUseFactorSerf:Float = 0.8;
	public static var AIFoodUseFactorCommoner:Float = 0.9;
	public static var AIFoodUseFactorNoble:Float = 1;

	public static var objectIdArrays = new Map<Int, Array<Int>>();

	public static function CanObjectBeLuckySpot(obj:Int):Bool {
		// 942 Muddy Iron Vein (can now respawn but not be lucky spot)
		// 3962 Loose Muddy Iron Vein
		// 3961 Iron Vein (can now respawn but not be lucky spot)
		// 3030 Natural Spring
		// 2285 Tarry Spot
		// 503 Dug Big Rock
		return (obj != 3030 && obj != 2285 && obj != 503 && obj != 942 && obj != 3961 && obj != 3962);
	}

	// iron, tary spot spring cannot respawn or be lucky spot
	public static function CanObjectRespawn(obj:Int):Bool {
		// 942 Muddy Iron Vein (can now respawn but not be lucky spot)
		// 3962 Loose Muddy Iron Vein
		// 3961 Iron Vein (can now respawn but not be lucky spot)
		// 3030 Natural Spring // TODO
		// 2285 Tarry Spot // TODO
		// 503 Dug Big Rock
		return (obj != 3030 && obj != 2285 && obj != 503);
	}

	public static function writeToFile() {
		var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
		var dir = './${ServerSettings.SaveDirectory}/';
		var path = dir + "ServerSettings.txt";

		if (FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);

		var writer = File.write(path, false);

		writer.writeString('Remove **default** if you dont want to use default value!\n');

		var count = 0;

		for (field in rtti.statics) {
			if ('$field'.indexOf('CFunction') != -1) continue;
			count++;
			var value:Dynamic = Reflect.field(ServerSettings, field.name);

			// trace('ServerSettings: $count ${field.name} ${field.type} $value');

			if ('${field.type}' == "CClass(String,[])") {
				writer.writeString('**default** ${field.name} = "$value"\n');
			}
			else {
				writer.writeString('**default** ${field.name} = $value\n');
			}
		}

		writer.writeString('**END**\n');

		writer.close();
	}

	public static function readFromFile(traceit:Bool = true):Bool {
		var reader = null;

		try {
			// var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
			var dir = './${ServerSettings.SaveDirectory}/';
			reader = File.read(dir + "ServerSettings.txt", false);

			reader.readLine();

			var line = "";

			while (line.indexOf('**END**') == -1) {
				line = reader.readLine();

				// trace('Read: ${line}');

				if (line.indexOf('**default**') != -1) continue;

				var splitLine = line.split("=");

				if (splitLine.length < 2) continue;

				splitLine[0] = StringTools.replace(splitLine[0], ' ', '');
				// splitLine[1] = StringTools.replace(splitLine[1], '\n', '');

				if (traceit) trace('Load Setting: ${splitLine[0]} = ${splitLine[1]}');

				var fieldName = splitLine[0];
				var value:Dynamic = splitLine[1];

				if (splitLine[1].indexOf('"') != -1) {
					var splitString = splitLine[1].split('"');

					if (splitString.length < 3) continue;

					value = splitString[1];
				}
				else {
					value = StringTools.replace(value, 'true', '1');
					value = StringTools.replace(value, 'false', '0');
					value = Std.parseFloat(value);
				}

				var oldValue:Dynamic = Reflect.field(ServerSettings, fieldName);

				Reflect.setField(ServerSettings, fieldName, value);

				var newValue:Dynamic = Reflect.field(ServerSettings, fieldName);

				if ('$newValue' != '$oldValue') trace('Setting changed: ${fieldName} = ${newValue} // old value: $oldValue');
			}
		} catch (ex) {
			if (reader != null) reader.close();

			trace(ex);

			return false;
		}

		return true;

		// trace('Read Test: traceTransitionByTargetDescription: $traceTransitionByTargetDescription');
		// trace('Read Test: YumBonus: $YumBonus');
	}

	public static function PatchObjectData() {
		ObjectData.getObjectData(707).clothing = "n"; // ANTARCTIC FUR SEAL

		// allow some smithing on tables // TODO fix time transition for contained obj
		for (obj in ObjectData.importedObjectData) {
			/*if(obj.floorHugging){
				trace('floorHugging: ${obj.name}');
			}*/

			if (obj.description.contains('Chisel')) {
				// Steel Chisel 455
				if (objectIdArrays[455] == null) objectIdArrays[455] = new Array<Int>();
				objectIdArrays[455].push(obj.parentId);
				// trace('Steel Chisel: ${obj.name}');
			}

			if (obj.description.contains('Wall') || obj.description.contains('Door')) {
				obj.allowFloorPlacement = true;
				// trace('allowFloorPlacement: ${obj.name}');
			}

			if (obj.description.contains('groundOnly')) {
				obj.groundOnly = true;
				// trace('groundOnly: ${obj.name}');
			}

			if (obj.description.indexOf("+hungryWork") != -1) {
				obj.hungryWork = ServerSettings.HungryWorkCost;
			}

			// Allow for smithing // TODO allow only place on table?
			if (obj.description.indexOf("on Flat Rock") != -1 || obj.description.indexOf("flat rock") != -1) {
				obj.containSize = 2;
				obj.containable = true;
			}

			if (obj.description.contains("Mechanism")) {
				obj.containSize = 2;
				obj.containable = true;
				// trace('Mechanism: ${obj.name}');
			}

			/*if (obj.description.contains("Glass")) {
				obj.containSize = 2;
				obj.containable = true;
				trace('Glass: ${obj.name}');
			}*/

			if (obj.description.contains("Blowpipe")) {
				obj.containSize = 2;
				obj.containable = true;
				// trace('Blowpipe: ${obj.name}');
			}

			if (obj.description.contains("Crucible") && obj.description.contains("in Wooden") == false) {
				obj.containSize = 2;
				obj.containable = true;
				// trace('Crucible: ${obj.name}');
			}

			if (obj.description.indexOf("Steel") != -1) {
				// trace('Decays to: ${obj.name}');
				obj.decaysToObj = 862; // 862 Broken Steel Tool no wood // 858 Broken Steel Tool
			}

			ObjectData.getObjectData(917).decaysToObj = 862; // Key 917 --> roken Steel Tool no wood 862
			ObjectData.getObjectData(1003).decaysToObj = 862; // Lock Removal Key 1003 --> roken Steel Tool no wood 862

			// ObjectData.getObjectData(625).decayFactor /= ObjDecayFactorForPermanentObjs;
			ObjectData.getObjectData(625).decaysToObj = 1101; // Wet Compost Pile --> Fertile Soil Pile 1101

			// this might be override for some down below
			if (obj.description.indexOf("Well") != -1
				|| (obj.description.indexOf("Pump") != -1 && obj.description.indexOf("Pumpkin") == -1)
				|| obj.description.indexOf("Vein") != -1
				|| obj.description.indexOf("Mine") != -1
				|| obj.description.indexOf("Iron Pit") != -1
				|| obj.description.indexOf("Drilling") != -1
				|| obj.description.indexOf("Rig") != -1
				|| obj.description.indexOf("Cave") != -1
				|| obj.description.indexOf("Ancient") != -1) {
				obj.decayFactor = -1;

				// trace('Settings: ${obj.description} ${obj.containSize}');
			}

			if (obj.description.indexOf("+owned") != -1) obj.isOwned = true;
			if (obj.description.indexOf("+tempOwned") != -1) obj.isOwned = true;
			if (obj.description.indexOf("+followerOwned") != -1) obj.isOwned = true;

			// if( obj.isOwned) trace('isOwned: ${obj.description}');

			// if(obj.containable) trace('${obj.description} ${obj.containSize}');

			if (obj.description.contains('Shears')) {
				// trace('${obj.name} permanent: ${obj.permanent}');
				obj.permanent = 0;
				obj.containSize = 1;
				obj.containable = true;
			}

			SetClothingPrestige(obj);
		}

		ObjectData.getObjectData(0).containSize = 1; // Empty
		ObjectData.getObjectData(0).containable = true; // Empty

		ObjectData.getObjectData(356).containSize = 2; // Basket of Bones

		ObjectData.getObjectData(356).containSize = 2; // Basket of Bones
		ObjectData.getObjectData(356).containable = true; // Basket of Bones

		ObjectData.getObjectData(2188).containSize = 2; // Drum Sticks on Plate
		ObjectData.getObjectData(2188).containable = true; // Drum Sticks on Plate

		ObjectData.getObjectData(2192).containSize = 1; // Turkey Leg Bone
		ObjectData.getObjectData(2192).containable = true; // Turkey Leg Bone

		ObjectData.getObjectData(2191).containSize = 1; // Turkey Drumstick
		ObjectData.getObjectData(2191).containable = true; // Turkey Drumstick

		ObjectData.getObjectData(319).containSize = 2; // Unforged Sealed Steel Crucible
		ObjectData.getObjectData(319).containable = true; // Unforged Sealed Steel Crucible

		ObjectData.getObjectData(321).containSize = 2; // Hot Forged Steel Crucible
		ObjectData.getObjectData(321).containable = true; // Hot Forged Steel Crucible

		ObjectData.getObjectData(322).containSize = 2; // Forged Steel Crucible
		ObjectData.getObjectData(322).containable = true; // Forged Steel Crucible

		ObjectData.getObjectData(325).containSize = 2; // Crucible with Steel
		ObjectData.getObjectData(325).containable = true; // Crucible with Steel

		ObjectData.getObjectData(322).containSize = 2; // Forged Steel Crucible
		ObjectData.getObjectData(322).containable = true; // Forged Steel Crucible

		ObjectData.getObjectData(1528).containSize = 2; // Quenching Spring Steel 1528
		ObjectData.getObjectData(1528).containable = true; // Quenching Spring Steel 1528

		ObjectData.getObjectData(2574).containSize = 2; // Molten Glass
		ObjectData.getObjectData(2574).containable = true; // Molten Glass

		ObjectData.getObjectData(2578).containSize = 2; // Cool Glass
		ObjectData.getObjectData(2578).containable = true; // Cool Glass

		ObjectData.getObjectData(2573).containSize = 2; // Soda Lime Glass Batch
		ObjectData.getObjectData(2573).containable = true; // Soda Lime Glass Batch

		ObjectData.getObjectData(300).containSize = 2; // Big Charcoal Pile
		ObjectData.getObjectData(300).containable = true; // Big Charcoal Pile

		ObjectData.getObjectData(301).containSize = 2; // Small Charcoal Pile
		ObjectData.getObjectData(301).containable = true; // Small Charcoal Pile

		ObjectData.getObjectData(302).containSize = 1; // Charcoal
		ObjectData.getObjectData(302).containable = true; // Charcoal

		ObjectData.getObjectData(650).countsOrGrowsAs = 630; // Bear Cave empty --> Bear Cave
		ObjectData.getObjectData(647).countsOrGrowsAs = 630; // Bear Cave waking --> Bear Cave

		// decay
		ObjectData.getObjectData(2709).decayFactor = -1; // Large Slow Fire tut_only burns forever
		ObjectData.getObjectData(3112).decayFactor = -1; // Tarr Monument

		ObjectData.getObjectData(858).decaysToObj = 862; // Broken Steel Tool 858 ==> Broken Steel Tool no wood 862

		// try too decay stuff faster that creates some mess
		// TODO set decay for containers
		// ObjectData.getObjectData(292).decayFactor = 2; // Basket 292
		ObjectData.getObjectData(292).decaysToObj = 227; // Basket 292 -- Straw 227
		// ObjectData.getObjectData(1155).decayFactor = 2; // Split Potato Sprouts 1155
		// ObjectData.getObjectData(204).decayFactor = 10; // Two Rabbit Furs 204
		ObjectData.getObjectData(204).decaysToObj = 183; // Two Rabbit Furs 204 --> Rabbit Furs
		// ObjectData.getObjectData(4063).decayFactor = 5; // Pile of Yew Branches 4063
		ObjectData.getObjectData(4063).decaysToObj = 132; // Pile of Yew Branches 4063 ==> Yew Branch 132
		// ObjectData.getObjectData(1121).decayFactor = 2; // Popcorn 1121
		ObjectData.getObjectData(1121).decaysToObj = 235; //  Popcorn 1121 --> Clay Bowl

		// Make Chest decay as fast as not permanent objects
		ObjectData.getObjectData(986).decayFactor /= ObjDecayFactorForPermanentObjs; // Open Wooden Chest
		ObjectData.getObjectData(986).decaysToObj = 4910; // Open Wooden Chest ==> Wooden Box with Boards and Rope

		ObjectData.getObjectData(987).decayFactor /= ObjDecayFactorForPermanentObjs; // Closed Wooden Chest
		ObjectData.getObjectData(987).decaysToObj = 4910; // Closed Wooden Chest ==> Wooden Box with Boards and Rope

		ObjectData.getObjectData(4910).decayFactor /= ObjDecayFactorForPermanentObjs; // Wooden Box with Boards and Rope
		ObjectData.getObjectData(4910).decaysToObj = 2740; // Wooden Box with Boards and Rope ==> Wooden Box with Boards

		ObjectData.getObjectData(2740).decayFactor /= ObjDecayFactorForPermanentObjs; // Wooden Box with Boards
		ObjectData.getObjectData(2740).decaysToObj = 434; // Wooden Box with Boards ==> Wooden Box

		ObjectData.getObjectData(434).decayFactor /= ObjDecayFactorForPermanentObjs; // Wooden Box
		ObjectData.getObjectData(434).decaysToObj = 470; // Wooden Box ==> Boards

		ObjectData.getObjectData(470).decaysToObj = 847; // Boards ==> Broken Skewer

		ObjectData.getObjectData(292).decaysToObj = 860; // Basket ==> Broken Basket

		// set decay for ancient
		ObjectData.getObjectData(898).decayFactor = 0.02; // Ancient Stone Floor
		ObjectData.getObjectData(898).decaysToObj = 1853; // Ancient Stone Floor ==> Cut Stones

		ObjectData.getObjectData(895).decayFactor = 0.02; // Ancient Stone Wall (corner)
		ObjectData.getObjectData(895).decaysToObj = 1853; // Ancient Stone Wall ==> Cut Stones
		ObjectData.getObjectData(895).fortificationObjId = 881; // Cut Stones 881

		ObjectData.getObjectData(896).decayFactor = 0.02; // Ancient Stone Wall (horizontal)
		ObjectData.getObjectData(896).decaysToObj = 1853; // Ancient Stone Wall ==> Cut Stones
		ObjectData.getObjectData(896).fortificationObjId = 881; // Cut Stones 881

		ObjectData.getObjectData(897).decayFactor = 0.02; // Ancient Stone Wall (vertical)
		ObjectData.getObjectData(897).decaysToObj = 1853; // Ancient Stone FloWallor ==> Cut Stones
		ObjectData.getObjectData(897).fortificationObjId = 881; // Cut Stones 881

		ObjectData.getObjectData(33).fortificationValue = 1; // Stone 33
		ObjectData.getObjectData(67).fortificationValue = 2; // Long Straight Shaft 67
		ObjectData.getObjectData(127).fortificationValue = 5; // Adobe 127
		ObjectData.getObjectData(470).fortificationValue = 5; //  Boards 470

		ObjectData.getObjectData(237).fortificationObjId = 33; // Adobe Oven 237 --> Adobe 127
		ObjectData.getObjectData(238).fortificationObjId = 33; // Adobe Kiln 238 --> Adobe 127

		ObjectData.getObjectData(2962).fortificationObjId = 67; // Property Gate 2962 --> Long Straight Shaft 67

		ObjectData.getObjectData(550).fortificationObjId = 67; // Fence - horizontal 550  --> Long Straight Shaft 67
		ObjectData.getObjectData(549).fortificationObjId = 67; // Fence - vertical 549  --> Long Straight Shaft 67
		ObjectData.getObjectData(551).fortificationObjId = 67; // Fence - corner 551  --> Long Straight Shaft 67

		ObjectData.getObjectData(1845).hungryWork = 5; // Loose Fence - horizontal 1845
		ObjectData.getObjectData(1846).hungryWork = 5; // Loose Fence  - vertical 1846
		ObjectData.getObjectData(1847).hungryWork = 5; // Loose Fence - corner 1847

		ObjectData.getObjectData(2757).fortificationObjId = 470; // Springy Wooden Door - horizontal 2757 --> Boards 470
		ObjectData.getObjectData(2759).fortificationObjId = 470; // Springy Wooden Door - vertical 2759 --> Boards 470

		// decay for mango trees
		ObjectData.getObjectData(1875).decayFactor = 0.1; // Fruiting Domestic Mango Tree
		ObjectData.getObjectData(1875).decaysToObj = 1876; // Fruiting Domestic Mango Tree --> Languishing Domestic Mango Tree

		ObjectData.getObjectData(1876).decayFactor = 0.1; // Languishing Domestic Mango Tree

		// set custom decay for iron mines
		ObjectData.getObjectData(3961).decayFactor = -1; // Iron Vein

		ObjectData.getObjectData(942).countsOrGrowsAs = 3961; // Muddy Iron -->  Iron Vein

		ObjectData.getObjectData(3944).decaysToObj = 881; // Stripped Iron Vein --> Cut Stones
		ObjectData.getObjectData(3944).decayFactor = 0.1; // Stripped Iron Vein
		ObjectData.getObjectData(3944).countsOrGrowsAs = 3961; // Stripped Iron Vein --> Iron Vein

		ObjectData.getObjectData(3957).decaysToObj = 881; // Shallow Iron Pit --> Cut Stones
		ObjectData.getObjectData(3957).decayFactor = 0.1; // Shallow Iron Pit
		ObjectData.getObjectData(3957).countsOrGrowsAs = 3961; // Shallow Iron Pit --> Iron Vein

		ObjectData.getObjectData(3956).decaysToObj = 881; // Shallow Pit with Ore --> Cut Stones
		ObjectData.getObjectData(3956).decayFactor = 0.1; // Shallow Pit with Ore
		ObjectData.getObjectData(3956).countsOrGrowsAs = 3961; // Shallow Pit with Ore --> Iron Vein

		ObjectData.getObjectData(943).decaysToObj = 881; // Deep Iron Pit --> Cut Stones
		ObjectData.getObjectData(943).decayFactor = 0.1; // Deep Iron Pit
		ObjectData.getObjectData(943).countsOrGrowsAs = 3961; // Deep Iron Pit --> Iron Vein

		ObjectData.getObjectData(3958).decaysToObj = 881; // Deep Pit with Ore --> Cut Stones
		ObjectData.getObjectData(3958).decayFactor = 0.1; // Deep Pit with Ore
		ObjectData.getObjectData(3958).countsOrGrowsAs = 3961; // Deep Pit with Ore --> Iron Vein

		ObjectData.getObjectData(944).decaysToObj = 881; // Iron Mine --> Cut Stones
		ObjectData.getObjectData(944).decayFactor = 0.1; // Iron Mine
		ObjectData.getObjectData(944).countsOrGrowsAs = 3961; // Iron Mine --> Iron Vein

		ObjectData.getObjectData(3959).decaysToObj = 881; // Mine with Ore --> Cut Stones
		ObjectData.getObjectData(3959).decayFactor = 0.1; // Mine with Ore
		ObjectData.getObjectData(3959).countsOrGrowsAs = 3961; // Mine with Ore --> Iron Vein

		ObjectData.getObjectData(3960).decaysToObj = 881; // Collapsed Mine with Ore --> Cut Stones
		ObjectData.getObjectData(3960).decayFactor = 0.1; // Collapsed Mine with Ore
		ObjectData.getObjectData(3960).countsOrGrowsAs = 3961; // Collapsed Mine with Ore --> Iron Vein

		ObjectData.getObjectData(945).decaysToObj = 881; // Collapsed Iron Mine --> Cut Stones
		ObjectData.getObjectData(945).decayFactor = 0.5; // Collapsed Iron Mine
		ObjectData.getObjectData(945).countsOrGrowsAs = 3961; // Collapsed Iron Mine --> Iron Vein

		ObjectData.getObjectData(3130).decaysToObj = 881; // Ready Diesel Mining Pick without Bit
		ObjectData.getObjectData(3130).decayFactor = 0.1; // Diesel Mining Pick without Bit
		ObjectData.getObjectData(3130).countsOrGrowsAs = 3961; // Diesel Mining Pick without Bit --> Iron Vein

		ObjectData.getObjectData(3129).decaysToObj = 881; // Ready Diesel Mining Pick
		ObjectData.getObjectData(3129).decayFactor = 0.1; // Ready Diesel Mining Pick
		ObjectData.getObjectData(3129).countsOrGrowsAs = 3961; // Ready Diesel Mining Pick --> Iron Vein

		ObjectData.getObjectData(3131).decaysToObj = 881; // Diesel Mining Pick with Iron
		ObjectData.getObjectData(3131).decayFactor = 0.1; // Diesel Mining Pick with Iron
		ObjectData.getObjectData(3131).countsOrGrowsAs = 3961; // Diesel Mining Pick with Iron --> Iron Vein
		// TODO get engine back from mine
		// TODO collapse mine if it gave 12 iron

		// horse cart decay // TODO allow decay? set decay for horse cart
		ObjectData.getObjectData(484).decaysToObj = 483; // Hand Cart 484 --> Wheelbarrow 483
		ObjectData.getObjectData(483).decaysToObj = 471; // Wheelbarrow 483 --> Wooden Sledge 471

		ObjectData.getObjectData(3157).decaysToObj = 780; // Escaped Horse-Drawn Tire Cart --> Escaped Horse-Drawn Cart
		ObjectData.getObjectData(780).decaysToObj = 775; // Escaped Horse-Drawn Tire Cart --> Escaped Riding Horse
		ObjectData.getObjectData(775).decaysToObj = 769; // Escaped Riding Horse --> Wild Horse

		ObjectData.getObjectData(3159).decaysToObj = 779; // Hitched Horse-Drawn Tire Cart --> Hitched Horse-Drawn Cart
		ObjectData.getObjectData(3159).decayFactor = AnimalDecayFactor; // Hitched Horse-Drawn Tire Cart
		ObjectData.getObjectData(779).decaysToObj = 774; // Hitched Horse-Drawn Cart --> Hitched Riding Horse
		ObjectData.getObjectData(779).decayFactor = AnimalDecayFactor; // Hitched Horse-Drawn Cart
		ObjectData.getObjectData(774).decaysToObj = 4154; // Hitched Riding Horse --> Hitching Post
		ObjectData.getObjectData(774).decayFactor = AnimalDecayFactor; // Hitched Riding Horse

		ObjectData.getObjectData(1458).decaysToObj = 1900; // Domestic Cow 1458 --> Dead Cow 1900
		ObjectData.getObjectData(1458).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1488).decaysToObj = 1900; // Fed Domestic Cow 1488 --> Dead Cow 1900
		ObjectData.getObjectData(1488).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1454).decaysToObj = 1900; // Domestic Cow with Calf 1454 --> Dead Cow 1900
		ObjectData.getObjectData(1454).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1489).decaysToObj = 1900; // Milk Cow 1489 --> Dead Cow 1900
		ObjectData.getObjectData(1489).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1459).decaysToObj = 1487; // Domestic Calf 1459 --> Dead Domestic Calf 1487
		ObjectData.getObjectData(1459).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1462).decaysToObj = 1487; // Hungry Domestic Calf 1462 --> Dead Domestic Calf 1487
		ObjectData.getObjectData(1462).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(1485).decaysToObj = 1487; // Fed Domestic Calf --> Dead Domestic Calf 1487
		ObjectData.getObjectData(1485).decayFactor = AnimalDecayFactor;

		ObjectData.getObjectData(575).decaysToObj = 595; // Domestic Sheep --> Dead Sheep 595
		ObjectData.getObjectData(575).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(4213).decaysToObj = 595; // Fed Domestic Sheep 4213 --> Dead Sheep 595
		ObjectData.getObjectData(4213).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(600).decaysToObj = 595; // Domestic Sheep with Lamb 600 --> Dead Sheep 595
		ObjectData.getObjectData(600).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(576).decaysToObj = 597; // Shorn Domestic Sheep --> Dead Sheep shorn
		ObjectData.getObjectData(576).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(542).decaysToObj = 606; // Domestic Lamb 542 --> Dead Domestic Lamb 606
		ObjectData.getObjectData(542).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(604).decaysToObj = 606; // Hungry Domestic Lamb 604 --> Dead Domestic Lamb 606
		ObjectData.getObjectData(604).decayFactor = AnimalDecayFactor;

		ObjectData.getObjectData(418).decaysToObj = 422; // Wolf 418 --> Dead Wolf 422
		ObjectData.getObjectData(418).decayFactor = AnimalDecayFactor;
		ObjectData.getObjectData(420).decaysToObj = 421; // Shot Wolf 420 --> Dead Wolf with Arrow 421
		ObjectData.getObjectData(420).decayFactor = AnimalDecayFactor;

		// set floor decay
		ObjectData.getObjectData(1596).decayFactor = 0.1; // 1596 Stone Road
		ObjectData.getObjectData(1596).decaysToObj = 291; // 1596 Stone Road ==> 291 Flat Rock

		ObjectData.getObjectData(884).decayFactor = 0.1; // 884 Stone Floor
		ObjectData.getObjectData(884).decaysToObj = 881; // 884 Stone Floor ==> 881 Cut Stones

		ObjectData.getObjectData(888).decayFactor = 1; // 888 Bear Skin Rug
		ObjectData.getObjectData(888).decaysToObj = 884; // 888 Bear Skin Rug ==> Stone Floor

		ObjectData.getObjectData(3290).decayFactor = 0.1; // 3290 Pine Floor

		// set
		ObjectData.getObjectData(115).decayFactor = 2; // Pine Door (horizontal)
		ObjectData.getObjectData(115).decaysToObj = 96; // Pine Door (horizontal) ==> 96 Pine Needles

		ObjectData.getObjectData(119).decayFactor = 2; // Open Pine Door (horizontal)
		ObjectData.getObjectData(119).decaysToObj = 96; // Open Pine Door (horizontal) ==> 96 Pine Needles
		ObjectData.getObjectData(119).rValue = 0.2; // Open Pine Door (horizontal)

		ObjectData.getObjectData(116).decayFactor = 2; // Pine Door (vertical)
		ObjectData.getObjectData(116).decaysToObj = 96; // Pine Door (vertical) ==> 96 Pine Needles

		ObjectData.getObjectData(117).decayFactor = 2; // Open Pine Door (vertical)
		ObjectData.getObjectData(117).decaysToObj = 96; // Open Pine Door (vertical) ==> 96 Pine Needles
		ObjectData.getObjectData(117).rValue = 0.2; // Open Pine Door (vertical)

		ObjectData.getObjectData(111).decayFactor = 2; // Pine Wall corner
		ObjectData.getObjectData(111).decaysToObj = 96; // Pine Wall ==> 96 Pine Needles
		ObjectData.getObjectData(113).decayFactor = 2; // Pine Wall verticalPine
		ObjectData.getObjectData(113).decaysToObj = 96; // Pine Wall ==> 96 Pine Needles
		ObjectData.getObjectData(112).decayFactor = 2; // Pine Wall horizontalPine
		ObjectData.getObjectData(112).decaysToObj = 96; // Pine Wall ==> 96 Pine Needles

		ObjectData.getObjectData(3308).decayFactor = 2; // Marked Pine Wall corner
		ObjectData.getObjectData(3308).decaysToObj = 96; // Marked Pine Wall ==> 96 Pine Needles
		ObjectData.getObjectData(3309).decayFactor = 2; // Marked Pine Wall verticalPine
		ObjectData.getObjectData(3309).decaysToObj = 96; // Marked Pine Wall ==> 96 Pine Needles
		ObjectData.getObjectData(3310).decayFactor = 2; // Marked Pine Wall horizontalPine
		ObjectData.getObjectData(3310).decaysToObj = 96; // Marked Pine Wall ==> 96 Pine Needles

		// set doors
		// ObjectData.getObjectData(876).rValue = 0.9; // 75% // Wooden Door
		ObjectData.getObjectData(876).decaysToObj = 470; // Wooden Door (horizontal) ==> Boards
		ObjectData.getObjectData(878).decaysToObj = 470; // Open Wooden Door (horizontal) ==> Boards
		ObjectData.getObjectData(878).rValue = 0.2; // Open Wooden Door (horizontal)

		ObjectData.getObjectData(877).decaysToObj = 470; // Wooden Door (vertical) ==> Boards
		ObjectData.getObjectData(879).decaysToObj = 470; // Open Wooden Door (vertical) ==> Boards
		ObjectData.getObjectData(879).rValue = 0.2; // Open Wooden Door (vertical)

		// set stone wall decay
		ObjectData.getObjectData(885).decayFactor = 0.2; //  Stone Wall+cornerStone
		ObjectData.getObjectData(885).decaysToObj = 1853; //  Stone Wall+cornerStone ==> Cut Stones
		ObjectData.getObjectData(885).fortificationObjId = 881; // Cut Stones 881

		ObjectData.getObjectData(886).decayFactor = 0.2; //  Stone Wall+verticalStone
		ObjectData.getObjectData(886).decaysToObj = 1853; //  Stone Wall+verticalStone  ==> Cut Stones
		ObjectData.getObjectData(886).fortificationObjId = 881; // Cut Stones 881

		ObjectData.getObjectData(887).decayFactor = 0.2; //  Stone Wall+horizontalStone
		ObjectData.getObjectData(887).decaysToObj = 1853; //  Stone Wall+horizontalStone  ==> Cut Stones
		ObjectData.getObjectData(887).fortificationObjId = 881; // Cut Stones 881
		// trace('isPermanent ${ObjectData.getObjectData(155).isPermanent()}');

		// TODO split up decay of walled containers in box and wall
		ObjectData.getObjectData(3240).decayFactor = 0.2; // Wall Shelf
		ObjectData.getObjectData(3240).decaysToObj = 434; // Wall Shelf ==> Wooden Box
		ObjectData.getObjectData(3240).rValue = 0.98;

		ObjectData.getObjectData(3241).decayFactor = 0.2; // Wall Shelf with Slot Notches
		ObjectData.getObjectData(3241).decaysToObj = 1885; // Wall Shelf with Slot Notches ==> Plaster Wall
		ObjectData.getObjectData(3241).rValue = 0.98;

		ObjectData.getObjectData(3242).decayFactor = 0.2; // Wall Slot Shelf
		ObjectData.getObjectData(3242).decaysToObj = 3065; // Wall Slot Shelf ==> Wooden Slot Box
		ObjectData.getObjectData(3242).rValue = 0.98;
		// TODO colored wall containers

		// Adobe Wall
		ObjectData.getObjectData(154).decaysToObj = 889; //  Adobe Wall+corner  ==> Cracking Adobe Wall corner
		ObjectData.getObjectData(155).decaysToObj = 891; //  Adobe Wall+horizontalAdobe  ==> Cracking Adobe Wall
		ObjectData.getObjectData(156).decaysToObj = 890; //  Fixed Adobe Wall (vertical)  ==> Cracking Adobe Wall (vertival)

		// Plastered Walls
		ObjectData.getObjectData(1883).decaysToObj = 154; // Plaster Wall (corner) ==> 155 Adobe Wall (Vorner)
		ObjectData.getObjectData(1884).decaysToObj = 156; // Plaster Wall (auto vertical) ==> 156 Adobe Wall ( vertical)
		ObjectData.getObjectData(1885).decaysToObj = 155; // Plaster Wall (auto horizontal) ==> 155 Adobe Wall (horizontal))

		ObjectData.getObjectData(1883).decayFactor = 0.2;
		ObjectData.getObjectData(1884).decayFactor = 0.2;
		ObjectData.getObjectData(1885).decayFactor = 0.2;

		ObjectData.getObjectData(1883).rValue = 0.98;
		ObjectData.getObjectData(1884).rValue = 0.98;
		ObjectData.getObjectData(1885).rValue = 0.98;

		// TODO colored walls

		// set object decay
		ObjectData.getObjectData(1598).decayFactor = -1; // 1598 Iron Ore Pile
		ObjectData.getObjectData(1837).decayFactor = -1; // 1837 Stack of Steel Ingots

		// TODO set water right and add further wells like deep well
		ObjectData.getObjectData(662).decayFactor = 0.1; // 662 Shallow Well
		ObjectData.getObjectData(662).decaysToObj = 3030; // 662 Shallow Well ==> 3030 Natural Spring
		// ObjectData.getObjectData(662).useChance = 0.1; // original 0.03

		ObjectData.getObjectData(303).decaysToObj = 238; // 303 Forge ==> 238 Adobe Kiln
		ObjectData.getObjectData(303).decaysToObj = 238; // 305 Forge with Charcoal ==> 238 Adobe Kiln
		ObjectData.getObjectData(238).decaysToObj = 4201; // 303 Adobe Kiln ==> 4201 Adobe Rubble
		ObjectData.getObjectData(281).decaysToObj = 4201; // 281 Wood-filled Adobe Kiln ==> 4201 Adobe Rubble
		ObjectData.getObjectData(237).decaysToObj = 753; // 237 Adobe Oven ==> 753 Adobe Rubble
		ObjectData.getObjectData(247).decaysToObj = 753; // 247 Wood-filled Adobe Oven ==> 753 Adobe Rubble

		ObjectData.getObjectData(4201).decayFactor = 0.1; // 4201 Adobe Rubble
		ObjectData.getObjectData(4201).decaysToObj = 753; // 4201 Adobe Rubble
		ObjectData.getObjectData(753).decayFactor = 0.1; // 753 Adobe Rubble

		// high tech stuff // TODO add more or use a general solution
		ObjectData.getObjectData(2365).decaysToObj = 2385; // 2365 Diesel Engine --> 2385 Diesel Drive Assembly
		ObjectData.getObjectData(2385).decaysToObj = 2383; // 2385 Diesel Drive Assembl --> 2383 Diesel Crank Assembly

		ObjectData.getObjectData(2240).decaysToObj = 2243; // 2240 Newcomen Hammer --> 2243 Multipurpose Newcomen Engine
		ObjectData.getObjectData(2243).decaysToObj = 2245; // 2243 Multipurpose Newcomen Engine --> 2245 Newcomen Engine without Rope
		ObjectData.getObjectData(2245).decaysToObj = 2246; // 2245 Newcomen Engine without Rope --> 2246 Newcomen Engine Tower
		ObjectData.getObjectData(2365).decaysToObj = 2385; // 2365 Diesel Engine --> 2385 Diesel Drive Assembly

		ObjectData.getObjectData(2280).decaysToObj = 2243; // 2280 Newcomen Roller --> 2243 Multipurpose Newcomen Engine
		ObjectData.getObjectData(2263).decaysToObj = 2264; // 2263 Roller Mechanism --> 2264 Partial Pulley Mechanism

		ObjectData.getObjectData(2270).decaysToObj = 2243; // 2270 Newcomen Bore --> 2243 Multipurpose Newcomen Engine
		ObjectData.getObjectData(2268).decaysToObj = 2262; // 2268 Bore Mechanism --> 2262 Pulley Drive Mechanism

		ObjectData.getObjectData(2359).decaysToObj = 2243; // 2359 Newcomen Lathe --> 2243 Multipurpose Newcomen Engine
		ObjectData.getObjectData(2356).decaysToObj = 2262; // 2356 Lathe Mechanism --> 2262 Pulley Drive Mechanism

		ObjectData.getObjectData(2365).decaysToObj = 2243; // 2395 Crude Car with Empty Tank --> 2365 Diesel Engine

		ObjectData.getObjectData(4144).useChance = 0.8; // old 1 // Dug Potatoes 4144

		ObjectData.getObjectData(237).allowFloorPlacement = true; // Adobe Oven 237
		ObjectData.getObjectData(247).allowFloorPlacement = true; // Wood-filled Adobe Oven 247
		ObjectData.getObjectData(238).allowFloorPlacement = true; // Adobe Kiln 238
		ObjectData.getObjectData(281).allowFloorPlacement = true; // Wood-filled Adobe Kiln 281
		ObjectData.getObjectData(303).allowFloorPlacement = true; // Forge 303
		ObjectData.getObjectData(305).allowFloorPlacement = true; // Forge with Charcoal 305
		ObjectData.getObjectData(3371).allowFloorPlacement = true; // Table 3371
		ObjectData.getObjectData(434).allowFloorPlacement = true; // Wooden Box 434
		ObjectData.getObjectData(3065).allowFloorPlacement = true; // Wooden Slot Box 3065

		// set property stuff
		ObjectData.getObjectData(987).blocksRemove = true; // Closed Wooden Chest 987
		ObjectData.getObjectData(988).blocksRemove = true; // Locked Wooden Chest 988

		// set hungry work
		// TODO use tool hungry work factor
		/*
			ObjectData.getObjectData(34).hungryWork = 1 * HungryWorkToolCostFactor; // Sharp Stone
			ObjectData.getObjectData(334).hungryWork = 1 * HungryWorkToolCostFactor; // Steel Axe
			ObjectData.getObjectData(502).hungryWork = 1 * HungryWorkToolCostFactor; // Shovel // TODO should be cheaper then sharp stone
		 */
		// ObjectData.getObjectData(334).hungryWork = -1; // Steel Axe
		// ObjectData.getObjectData(502).hungryWork = -1; // Shovel

		// trace('useChance: ${ObjectData.getObjectData(502).useChance}');
		ObjectData.getObjectData(502).useChance = 0.05; // 0.1 // Shovel 502

		// Steel Hoe 857
		ObjectData.getObjectData(857).hungryWork = -2;
		ObjectData.getObjectData(857).useChance = 0.02; // vanilla: 8%

		// Stone Hoe 850
		ObjectData.getObjectData(850).useChance = 0.1; // vanilla: 20%

		ObjectData.getObjectData(1849).hungryWork = 5; // Buried Grave with Dug Stone

		ObjectData.getObjectData(123).hungryWork = 2; // Harvested Tule
		ObjectData.getObjectData(231).hungryWork = 10; // Adobe Oven Base

		ObjectData.getObjectData(1020).hungryWork = 2; // Snow Bank
		ObjectData.getObjectData(138).hungryWork = 2; // Cut Sapling Skewer
		ObjectData.getObjectData(3961).hungryWork = 5; // Iron Vein
		ObjectData.getObjectData(496).hungryWork = 4; // Dug Stump
		ObjectData.getObjectData(1011).hungryWork = 3; // Buried Grave
		// ObjectData.getObjectData(357).hungryWork = 5; // Bone Pile // Dont set!!!

		ObjectData.getObjectData(213).hungryWork = 3; // Deep Tilled Row
		ObjectData.getObjectData(1136).hungryWork = 3; // Shallow Tilled Row

		ObjectData.getObjectData(511).hungryWork = 2; // Pond
		ObjectData.getObjectData(1261).hungryWork = 2; // Canada Goose Pond with Egg
		ObjectData.getObjectData(141).hungryWork = 2; // Canada Goose Pond
		ObjectData.getObjectData(142).hungryWork = 2; // Canada Goose Pond swimming
		ObjectData.getObjectData(143).hungryWork = 2; // Canada Goose Pond swimming, feather
		ObjectData.getObjectData(662).hungryWork = 1; // Shallow Well
		ObjectData.getObjectData(663).hungryWork = 2; // Deep Well

		// reduce water in ponds / wells
		ObjectData.getObjectData(511).useChance = 0.5; // 0.2 // Pond
		ObjectData.getObjectData(1261).useChance = 0.5; // Canada Goose Pond with Egg
		ObjectData.getObjectData(141).useChance = 0.5; // Canada Goose Pond
		ObjectData.getObjectData(142).useChance = 0.5; // Canada Goose Pond swimming
		ObjectData.getObjectData(143).useChance = 0.5; // Canada Goose Pond swimming, feather
		ObjectData.getObjectData(662).useChance = 0.1; // 0.03 Shallow Well
		// ObjectData.getObjectData(663).useChance = 2; // Deep Well

		// ObjectData.getObjectData(496).alternativeTransitionOutcome = 10; // Dug Stump

		// let loved food grow in loved biomes
		ObjectData.getObjectData(4251).biomes.push(BiomeTag.GREY); // Wild Garlic is loved now by White
		// ObjectData.getObjectData(36).biomes.push(BiomeTag.SNOW); // Wild Carrot is loved now by Ginger

		// is set directly in map WorldMap generation
		// ObjectData.getObjectData(141).biomes.push(BiomeTag.PASSABLERIVER); // Canada Goose Pond
		// ObjectData.getObjectData(121).biomes.push(BiomeTag.PASSABLERIVER); // Tule Reeds
		ObjectData.getObjectData(141).secondTimeOutcome = 142; // Canada Goose Pond ==> Canada Goose Pond swimming
		ObjectData.getObjectData(141).secondTimeOutcomeTimeToChange = 30;

		ObjectData.getObjectData(142).secondTimeOutcome = 1261; // Canada Goose Pond swimming ==> Canada Goose Pond with Egg
		ObjectData.getObjectData(142).secondTimeOutcomeTimeToChange = 60 * 10;

		ObjectData.getObjectData(1261).secondTimeOutcome = 142; // Canada Goose Pond with Egg ==> Canada Goose Pond swimming
		ObjectData.getObjectData(1261).secondTimeOutcomeTimeToChange = 60 * 4;

		ObjectData.getObjectData(511).secondTimeOutcome = 142; // Pond ==> Canada Goose Pond swimming
		ObjectData.getObjectData(511).secondTimeOutcomeTimeToChange = 60 * 60 * 24;

		ObjectData.getObjectData(141).countsOrGrowsAs = 1261; // Canada Goose Pond
		ObjectData.getObjectData(142).countsOrGrowsAs = 1261; // Canada Goose Pond swimming
		ObjectData.getObjectData(510).countsOrGrowsAs = 1261; // Pond with Dead Goose plus arrow
		ObjectData.getObjectData(509).countsOrGrowsAs = 1261; // Pond with Dead Goose
		ObjectData.getObjectData(511).countsOrGrowsAs = 1261; // Pond
		ObjectData.getObjectData(512).countsOrGrowsAs = 1261; // Dry Pond

		ObjectData.getObjectData(409).countsOrGrowsAs = 125; // Clay Pit (partial) --> Clay Deposit

		ObjectData.getObjectData(404).countsOrGrowsAs = 1435; // Bison with Calf --> Bison

		ObjectData.getObjectData(1328).countsOrGrowsAs = 1323; // Wild Boar with Piglet --> Wild Boar

		ObjectData.getObjectData(762).countsOrGrowsAs = 761; // Flowering Barrel Cactus --> Barrel Cactus
		ObjectData.getObjectData(763).countsOrGrowsAs = 761; // Fruiting Barrel Cactus --> Barrel Cactus

		ObjectData.getObjectData(2145).countsOrGrowsAs = 2142; // Empty Banana Plant --> Banana Plant
		ObjectData.getObjectData(279).countsOrGrowsAs = 30; // Empty Wild Gooseberry Bush --> Wild Gooseberry Bush

		ObjectData.getObjectData(164).secondTimeOutcome = 173; // Rabbit Hole out,single ==> Rabbit Family Hole out
		ObjectData.getObjectData(164).secondTimeOutcomeTimeToChange = 90;

		ObjectData.getObjectData(173).secondTimeOutcome = 3566; // Rabbit Family Hole out ==> Fleeing Rabbit
		ObjectData.getObjectData(173).secondTimeOutcomeTimeToChange = 90;

		ObjectData.getObjectData(164).countsOrGrowsAs = 161; // Rabbit Hole out,single couts as Rabbit Hole
		ObjectData.getObjectData(173).countsOrGrowsAs = 161; // Rabbit Family Hole couts as Rabbit Hole

		ObjectData.getObjectData(3566).countsOrGrowsAs = 161; // Fleeing Rabbit
		// dont block walking TODO needs client change
		// ObjectData.getObjectData(231).blocksWalking = false; // Adobe Oven Base
		// ObjectData.getObjectData(237).blocksWalking = false; // Adobe Oven

		// Change map spawn chances
		ObjectData.getObjectData(3030).mapChance *= 3; // Natural Spring
		ObjectData.getObjectData(769).mapChance *= 2; // Wild Horse
		// ObjectData.getObjectData(769).biomes.push(BiomeTag.GREEN); // Beautiful Horses now also in Green biome :)

		// 3961 Iron Vein use spawn chance from Muddy Iron Vein X10
		ObjectData.getObjectData(3961).mapChance = ObjectData.getObjectData(942).mapChance *= 10;
		ObjectData.getObjectData(3961).biomes = ObjectData.getObjectData(942).biomes;
		// spawn Iron Vein instead of Muddy Iron Vein
		ObjectData.getObjectData(942).mapChance = 0; // Muddy Iron Vein // spawn 3961 Iron Vein instead
		ObjectData.getObjectData(942).biomes = [];

		// less iron in iron mine
		ObjectData.getObjectData(944).useChance = 0.5; // 0.20 Iron Mine
		ObjectData.getObjectData(3957).useChance = 1; // 0.5 Shallow Iron Pit

		ObjectData.getObjectData(2135).mapChance /= 4; // Rubber Tree
		ObjectData.getObjectData(530).mapChance /= 2; // Bald Cypress Tree
		ObjectData.getObjectData(121).mapChance *= 3; // Tule Reeds

		ObjectData.getObjectData(2156).mapChance *= 0.3; // Less UnHappy Mosquitos
		ObjectData.getObjectData(2156).biomes.push(BiomeTag.SWAMP); // Evil Mosquitos now also in Swamp

		// make sheeps stay close to Green biome
		ObjectData.getObjectData(575).biomes.push(BiomeTag.GREEN); // Domestic Sheep
		ObjectData.getObjectData(4213).biomes.push(BiomeTag.GREEN); // Fed Domestic Sheep
		ObjectData.getObjectData(576).biomes.push(BiomeTag.GREEN); // Shorn Domestic Sheep
		ObjectData.getObjectData(614).biomes.push(BiomeTag.GREEN); // Fed Shorn Domestic Sheep
		ObjectData.getObjectData(602).biomes.push(BiomeTag.GREEN); // Fed Domestic Lamb
		ObjectData.getObjectData(542).biomes.push(BiomeTag.GREEN); // Domestic Lamb
		ObjectData.getObjectData(604).biomes.push(BiomeTag.GREEN); // Hungry Domestic Lamb

		ObjectData.getObjectData(1489).biomes.push(BiomeTag.GREEN); // Milk Cow
		ObjectData.getObjectData(1492).biomes.push(BiomeTag.GREEN); // Dry Milk Cow
		ObjectData.getObjectData(1488).biomes.push(BiomeTag.GREEN); // Fed Domestic Cow
		ObjectData.getObjectData(1458).biomes.push(BiomeTag.GREEN); // Domestic Cow

		ObjectData.getObjectData(1485).biomes.push(BiomeTag.GREEN); // Fed Domestic Calf
		ObjectData.getObjectData(1459).biomes.push(BiomeTag.GREEN); // Domestic Calf

		ObjectData.getObjectData(770).biomes.push(BiomeTag.GREEN); // Riding Horse
		ObjectData.getObjectData(780).biomes.push(BiomeTag.GREEN); // Escaped Horse-Drawn Cart
		ObjectData.getObjectData(3157).biomes.push(BiomeTag.GREEN); // Escaped Horse-Drawn Tire Cart

		ObjectData.getObjectData(1256).biomes.push(BiomeTag.GREEN); // Domestic Goose 1256
		ObjectData.getObjectData(1278).biomes.push(BiomeTag.GREEN); // Fed Domestic Goose
		ObjectData.getObjectData(1255).biomes.push(BiomeTag.GREEN); // Gosling 1255

		// set loved biomes right
		ObjectData.getObjectData(1328).biomes = []; // Wild Boar with Piglet
		ObjectData.getObjectData(1328).biomes.push(BiomeTag.SWAMP); // Wild Boar with Piglet

		// Fleeing Rabbit 3566
		ObjectData.getObjectData(3566).biomes = [];
		ObjectData.getObjectData(3566).biomes.push(BiomeTag.YELLOW);

		ObjectData.getObjectData(631).biomes = []; // Hungry Grizzly Bear
		ObjectData.getObjectData(631).biomes.push(BiomeTag.GREY); // Hungry Grizzly Bear
		ObjectData.getObjectData(4762).biomes = []; // Sleepy Grizzly Bear 4762
		ObjectData.getObjectData(4762).biomes.push(BiomeTag.GREY); // Sleepy Grizzly Bear 4762
		ObjectData.getObjectData(632).biomes = []; // Shot Grizzly Bear
		ObjectData.getObjectData(632).biomes.push(BiomeTag.GREY); // Shot Grizzly Bear
		ObjectData.getObjectData(635).biomes = []; // Shot Grizzly Bear 2
		ObjectData.getObjectData(635).biomes.push(BiomeTag.GREY); // Shot Grizzly Bear 2

		// More Wolfs needs the world
		ObjectData.getObjectData(418).biomes.push(BiomeTag.YELLOW); // Happy Wolfs now also in Yellow biome :)
		// ObjectData.getObjectData(418).biomes.push(BiomeTag.GREEN); // Happy Wolfs now also in Green biome :)
		ObjectData.getObjectData(418).biomes.push(BiomeTag.SNOW); // Happy Wolfs now also in Snow biome :)
		ObjectData.getObjectData(418).mapChance *= 1.2; // more Happy Wolfs
		ObjectData.getObjectData(418).speedMult *= 1.5; // Boost Wolfs even more :)

		ObjectData.getObjectData(764).mapChance *= 5; // more snakes needs the world!
		ObjectData.getObjectData(764).permanent = 0; // Rattle Snake

		// Snake Skin Boot
		ObjectData.getObjectData(766).rValue = 1;
		// trace('Snake Skin Boot: ${ObjectData.getObjectData(766).getInsulation()} rv: ${ObjectData.getObjectData(766).rValue}');

		// Reduced carry speed
		ObjectData.getObjectData(411).speedMult = SemiHeavyItemSpeed; // Fertile Soil
		ObjectData.getObjectData(345).speedMult = SemiHeavyItemSpeed; // Butt Log
		// ObjectData.getObjectData(126).speedMult = SemiHeavyItemSpeed; // Clay (Players complained)
		// ObjectData.getObjectData(127).speedMult = SemiHeavyItemSpeed; // Adobe
		ObjectData.getObjectData(290).speedMult = SemiHeavyItemSpeed; // Iron Ore
		ObjectData.getObjectData(314).speedMult = SemiHeavyItemSpeed; // Wrought Iron
		ObjectData.getObjectData(326).speedMult = SemiHeavyItemSpeed; // Steel Ingot

		ObjectData.getObjectData(838).mapChance = ObjectData.getObjectData(211).mapChance / 5; // Add some lovely mushrooms
		ObjectData.getObjectData(838).biomes.push(BiomeTag.GREEN); // Add some lovely mushrooms

		// nerve horse cart little bit :)
		ObjectData.getObjectData(778).speedMult = 1.40; // Horse-Drawn Cart
		ObjectData.getObjectData(3158).speedMult = 1.50; // Horse-Drawn Tire Cart

		ObjectData.getObjectData(484).speedMult = 0.85; // Hand Cart
		ObjectData.getObjectData(861).speedMult = 0.85; // // Old Hand Cart
		ObjectData.getObjectData(2172).speedMult = 0.9; // Hand Cart with Tires

		ObjectData.getObjectData(253).reducesLongingFor = 31; // BOWL OF GOOSEBERRIES 253 --> Gooseberry 31
		ObjectData.getObjectData(31).higherQaulityFood = 253; // Gooseberry 31 --> BOWL OF GOOSEBERRIES 253
		ObjectData.getObjectData(272).reducesLongingFor = 253; // Cooked Berry Pie 272 --> BOWL OF GOOSEBERRIES 253
		ObjectData.getObjectData(253).higherQaulityFood = 272; // BOWL OF GOOSEBERRIES 253 --> Cooked Berry Pie 272

		// TODO treat popcorn same as bowl of popcorn
		ObjectData.getObjectData(1121).reducesLongingFor = 4895; // Bowl of Popcorn 1121 --> Popcorn
		ObjectData.getObjectData(4895).higherQaulityFood = 1121; // Popcorn --> Bowl of Popcorn 1121

		ObjectData.getObjectData(402).reducesLongingFor = 40; // Carrot 402 -->  Wild Carrot 40
		ObjectData.getObjectData(40).higherQaulityFood = 402;
		ObjectData.getObjectData(273).reducesLongingFor = 402; // Cooked Carrot Pie 273 --> Carrot 402
		ObjectData.getObjectData(402).higherQaulityFood = 273;

		ObjectData.getObjectData(2855).reducesLongingFor = 808; // Onion 2855 --> Wild Onion 808
		ObjectData.getObjectData(808).higherQaulityFood = 2855;
		ObjectData.getObjectData(2860).reducesLongingFor = 2855; // Chopped Onion on Plate 2860 --> Onion 2855
		ObjectData.getObjectData(2855).higherQaulityFood = 2860;

		ObjectData.getObjectData(2861).reducesLongingFor = 2836; // Chopped Tomato on Plate --> Tomato 2836
		ObjectData.getObjectData(2836).higherQaulityFood = 2861;

		ObjectData.getObjectData(803).reducesLongingFor = 570; // Cooked Mutton Pie 803 --> Cooked Mutton 570
		ObjectData.getObjectData(570).higherQaulityFood = 803;

		ObjectData.getObjectData(4081).reducesLongingFor = 1463; // Whole Milk Pouch 4081 --> Bowl of Whole Milk 1463
		ObjectData.getObjectData(1463).higherQaulityFood = 4081;
		ObjectData.getObjectData(3593).reducesLongingFor = 4081; // Whole Milk Bottle 3593 --> Whole Milk Pouch 4081
		ObjectData.getObjectData(4081).higherQaulityFood = 3593;

		ObjectData.getObjectData(4082).reducesLongingFor = 1481; // Skim Milk Pouch 4082 --> Bowl of Skim Milk 1481
		ObjectData.getObjectData(1481).higherQaulityFood = 4082;
		ObjectData.getObjectData(3596).reducesLongingFor = 4082; // Skim Milk Bottle 3596 --> Skim Milk Pouch 4082
		ObjectData.getObjectData(4082).higherQaulityFood = 3596;

		// nerve food
		ObjectData.getObjectData(768).foodValue = 4; // Cactus Fruit 768 // origional 8
		ObjectData.getObjectData(2143).foodValue = 5; // banana // origional 7
		ObjectData.getObjectData(31).foodValue = 2; // Gooseberry // origional 3
		ObjectData.getObjectData(253).foodValue = 2; // BOWL OF GOOSEBERRIES 253 // origional 3
		ObjectData.getObjectData(2855).foodValue = 3; // Onion // origional 5
		ObjectData.getObjectData(808).foodValue = 2; // Wild Onion // origional 4
		ObjectData.getObjectData(807).foodValue = 4; // Burdock Rootl 7
		ObjectData.getObjectData(40).foodValue = 4; // Wild Carrot // origional 5
		ObjectData.getObjectData(402).foodValue = 4; // Carrot // origional 5
		ObjectData.getObjectData(4252).foodValue = 2; // WILD GARLIC 4252 // origional 4
		ObjectData.getObjectData(2836).foodValue = 4; // Tomato  // origional 5
		// ObjectData.getObjectData(837).foodValue = 1; // PSILOCYBE MUSHROOM 837 // origional 1

		// boost hunted / cooked food
		ObjectData.getObjectData(197).foodValue = 25; // Cooked Rabbit 10 --> 25
		ObjectData.getObjectData(2190).foodValue = 20; // Turkey Slice on Plate 17 --> 20
		ObjectData.getObjectData(1285).foodValue = 15; // Omelette 12 --> 15
		ObjectData.getObjectData(1292).foodValue = 20; // Bowl of Cooked Beans 12 --> 20

		// ObjectData.getObjectData(197).useChance = 0.3; // Cooked Rabbit
		// ObjectData.getObjectData(2190).useChance = 0.3; // Turkey Slice on Plate
		// ObjectData.getObjectData(518).useChance = 0.3; // Cooked Goose
		// ObjectData.getObjectData(2143).useChance = 0.3; // Banana

		// soil should replace water as most needed ressource
		// composted soil has default 7 uses and each of it can be used twice for soil so in total 14
		ObjectData.getObjectData(624).numUses = 7; // default 7 Composted Soil Uses: 3 Soil (Wheat, Berry, Dung) + water ==> 4 Soil
		// ObjectData.getObjectData(411).useChance = 0.5; // Fertile Soil Pit 9 uses --> 18

		// Dry Compost Pile 623
		ObjectData.getObjectData(623).groundOnly = true;

		// TODO let rows decay from time to time to increase soil need.

		ObjectData.getObjectData(532).countsOrGrowsAs = 531; // 532 Mouflon with Lamb --> Mouflon

		// plants that decay and regrow
		// Wild Onion
		ObjectData.getObjectData(805).winterDecayFactor = 1; // Wild Onion 805 --> 808 (harvested)
		ObjectData.getObjectData(805).springRegrowFactor = 1; // Wild Onion 805 --> 808 (harvested)
		ObjectData.getObjectData(808).winterDecayFactor = 2; // Wild Onion 805 --> 808 (harvested)
		ObjectData.getObjectData(808).springRegrowFactor = 0.5; // Wild Onion 805 --> 808 (harvested)
		ObjectData.getObjectData(808).countsOrGrowsAs = 805; // Wild Onion (harvested) 808 --> Wild Onion 805

		// Wild Carrot // TODO let seeds regrow
		ObjectData.getObjectData(36).winterDecayFactor = 1; // Seeding Wild Carrot
		ObjectData.getObjectData(36).springRegrowFactor = 1; // Seeding Wild Carrot
		ObjectData.getObjectData(404).winterDecayFactor = 1; // Wild Carrot wihout Seed
		ObjectData.getObjectData(404).springRegrowFactor = 0.5; // Wild Carrot wihout Seed
		ObjectData.getObjectData(404).countsOrGrowsAs = 36; // Wild Carrot wihout Seed ==> Seeding Wild Carrot
		ObjectData.getObjectData(40).winterDecayFactor = 2; // Wild Carrot
		ObjectData.getObjectData(40).springRegrowFactor = 0.5; // Wild Carrot / out
		ObjectData.getObjectData(40).countsOrGrowsAs = 36; // Wild Carrot / out ==> Seeding Wild Carrot
		ObjectData.getObjectData(39).winterDecayFactor = 2; // Dug Wild Carrot
		ObjectData.getObjectData(39).springRegrowFactor = 0.5; // Dug Wild Carrot
		ObjectData.getObjectData(39).countsOrGrowsAs = 36; // Dug Wild Carrot ==> Seeding Wild Carrot

		// Wild Garlic
		ObjectData.getObjectData(4251).mapChance *= 5; // Wild Garlic
		ObjectData.getObjectData(4251).winterDecayFactor = 1; // Wild Garlic
		ObjectData.getObjectData(4251).springRegrowFactor = 1; // Wild Garlic
		ObjectData.getObjectData(4252).winterDecayFactor = 2; // Wild Garlic / out
		ObjectData.getObjectData(4252).springRegrowFactor = 0.5; // Wild Garlic / out
		ObjectData.getObjectData(4252).countsOrGrowsAs = 4251; // Wild Garlic / out

		// Burdock
		ObjectData.getObjectData(804).winterDecayFactor = 1; // Burdock
		ObjectData.getObjectData(804).springRegrowFactor = 1; // Burdock
		ObjectData.getObjectData(806).winterDecayFactor = 2; // Dug Burdock
		ObjectData.getObjectData(806).springRegrowFactor = 0.5; // Dug Burdock
		ObjectData.getObjectData(806).countsOrGrowsAs = 804; // Dug Burdock ==> Burdock
		ObjectData.getObjectData(807).winterDecayFactor = 2; // Burdock Root
		ObjectData.getObjectData(807).springRegrowFactor = 0.5; // Burdock Root
		ObjectData.getObjectData(807).countsOrGrowsAs = 804; // Burdock Root ==> Burdock

		// Milkweed
		ObjectData.getObjectData(50).mapChance *= 1.2; // Milkweed
		ObjectData.getObjectData(50).winterDecayFactor = 0; // Milkweed
		ObjectData.getObjectData(50).springRegrowFactor = 0.1; // Milkweed
		ObjectData.getObjectData(51).winterDecayFactor = 0; // Flowering Milkweed
		ObjectData.getObjectData(51).springRegrowFactor = 0.1; // Flowering Milkweed
		ObjectData.getObjectData(51).countsOrGrowsAs = 50; // Flowering Milkweed
		ObjectData.getObjectData(52).winterDecayFactor = 0; // Fruiting Milkweed
		ObjectData.getObjectData(52).springRegrowFactor = 0.1; // Fruiting Milkweed
		ObjectData.getObjectData(52).countsOrGrowsAs = 50; // Fruiting Milkweed
		ObjectData.getObjectData(57).winterDecayFactor = 2; // Milkweed Stalk
		ObjectData.getObjectData(57).springRegrowFactor = 0; // Milkweed Stalk

		// Sapling
		// ObjectData.getObjectData(136).winterDecayFactor = 0.5; // Sapling
		ObjectData.getObjectData(136).springRegrowFactor = 0.05; // Sapling
		ObjectData.getObjectData(138).winterDecayFactor = 2; // Cut Sapling Skewer
		ObjectData.getObjectData(138).springRegrowFactor = 0.5; // Cut Sapling Skewer
		ObjectData.getObjectData(138).countsOrGrowsAs = 136; // Cut Sapling Skewer ==> Sapling

		// Wild Gooseberry Bush
		ObjectData.getObjectData(30).winterDecayFactor = 1; // 1.5; // Wild Gooseberry Bush
		ObjectData.getObjectData(30).springRegrowFactor = 1; // 1.6 // Wild Gooseberry Bush
		ObjectData.getObjectData(279).springRegrowFactor = 6; // 1.8; // Empty Wild Gooseberry Bush
		// ObjectData.getObjectData(279).numUses = ObjectData.getObjectData(30).numUses; // Empty Wild Gooseberry Bush
		ObjectData.getObjectData(31).winterDecayFactor = 2; // Gooseberry

		// Domestic Gooseberry Bush
		// ObjectData.getObjectData(391).winterDecayFactor = 1; // Domestic Gooseberry Bush
		// ObjectData.getObjectData(391).springRegrowFactor = 0.2; // Domestic Gooseberry Bush
		ObjectData.getObjectData(1135).springRegrowFactor = 0.2; // Empty Domestic Gooseberry Bush

		ObjectData.getObjectData(750).speedMult = 0.75; // Bloody Knife
		ObjectData.getObjectData(3048).speedMult = 0.85; // Bloody War Sword
		ObjectData.getObjectData(749).speedMult = 0.6; // Bloody Yew Bow

		ObjectData.getObjectData(750).isBloody = true; // Bloody Knife
		ObjectData.getObjectData(3048).isBloody = true; // Bloody War Sword
		ObjectData.getObjectData(749).isBloody = true; // Bloody Yew Bow

		ObjectData.getObjectData(750).neverDrop = true; // Bloody Knife
		ObjectData.getObjectData(3048).neverDrop = true; // Bloody War Sword
		ObjectData.getObjectData(749).neverDrop = true; // Bloody Yew Bow

		// ObjectData.getObjectData(1378).permanent = 0; // Sterile Wool Pad

		ObjectData.getObjectData(1618).permanent = 0; // Written Paper &written 1618
		ObjectData.getObjectData(2100).permanent = 0; // Fishing Pole with Char
		ObjectData.getObjectData(2098).permanent = 0; // Fishing Pole with Old Boot

		ObjectData.getObjectData(3047).prestigeClass = PrestigeClass.Noble; // War Sword

		ObjectData.getObjectData(560).deadlyDistance = 1.5; // Knife
		ObjectData.getObjectData(3047).deadlyDistance = 1.5; // War Sword
		ObjectData.getObjectData(152).deadlyDistance = 4; // Bow and Arrow
		ObjectData.getObjectData(1624).deadlyDistance = 4; // Bow and Arrow with Note

		ObjectData.getObjectData(750).deadlyDistance = 1.5; // Bloody Knife
		ObjectData.getObjectData(3048).deadlyDistance = 1.5; // Bloody War Sword
		ObjectData.getObjectData(749).deadlyDistance = 4; // Bloody Yew Bow

		// Riding Horse
		ObjectData.getObjectData(770).damageProtectionFactor = 0.5; // 50% protection
		// Knife
		ObjectData.getObjectData(560).damageProtectionFactor = 0.8; // 20% protection
		// War Sword
		ObjectData.getObjectData(3047).damageProtectionFactor = 0.8; // 20% protection 36% for nobles

		// TODO more animals like Mouflon?
		ObjectData.getObjectData(1435).deadlyDistance = AnimalDeadlyDistanceFactor; // Bison
		ObjectData.getObjectData(1435).damage = 2; // Bison
		ObjectData.getObjectData(1438).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Bison
		ObjectData.getObjectData(1438).damage = 5; // Shot Bison
		ObjectData.getObjectData(1438).countsOrGrowsAs = 1435; // Shot Bison --> Bison
		ObjectData.getObjectData(1436).deadlyDistance = AnimalDeadlyDistanceFactor; // Bison with Calf
		ObjectData.getObjectData(1436).damage = 4; // Bison with Calf
		ObjectData.getObjectData(1436).countsOrGrowsAs = 1435; // Bison with Calf --> Bison
		ObjectData.getObjectData(1440).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Bison with Calf
		ObjectData.getObjectData(1440).damage = 6; // Shot Bison with Calf
		ObjectData.getObjectData(1440).countsOrGrowsAs = 1435; // Shot Bison with Calf --> Bison

		ObjectData.getObjectData(2156).deadlyDistance = AnimalDeadlyDistanceFactor; // 2156 Mosquito Swarm
		ObjectData.getObjectData(2156).damage = 1; // 2156 Mosquito Swarm

		ObjectData.getObjectData(418).deadlyDistance = AnimalDeadlyDistanceFactor; // Wolfs
		ObjectData.getObjectData(418).damage = 3; // 4 // 3 // Wolfs
		ObjectData.getObjectData(420).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Wolf
		ObjectData.getObjectData(420).damage = 5; // Shot Wolf

		ObjectData.getObjectData(764).deadlyDistance = AnimalDeadlyDistanceFactor; // Rattle Snake
		ObjectData.getObjectData(764).damage = 2; //  Rattle Snake
		ObjectData.getObjectData(764).woundFactor = 0.98; //  Rattle Snake

		ObjectData.getObjectData(1323).deadlyDistance = AnimalDeadlyDistanceFactor; // Wild Boar
		ObjectData.getObjectData(1323).damage = 3; // Wild Boar
		ObjectData.getObjectData(1328).deadlyDistance = AnimalDeadlyDistanceFactor; // Wild Boar with Piglet
		ObjectData.getObjectData(1328).damage = 5; // Wild Boar with Piglet

		ObjectData.getObjectData(628).deadlyDistance = AnimalDeadlyDistanceFactor; // Grizzly Bear
		ObjectData.getObjectData(628).damage = 5; // Grizzly Bear
		ObjectData.getObjectData(631).deadlyDistance = AnimalDeadlyDistanceFactor; // Hungry Grizzly Bear
		ObjectData.getObjectData(631).damage = 6; // Hungry Grizzly Bear
		ObjectData.getObjectData(653).deadlyDistance = AnimalDeadlyDistanceFactor; // Hungry Grizzly Bear attacking
		ObjectData.getObjectData(653).damage = 6; // Hungry Grizzly Bear attacking
		ObjectData.getObjectData(4762).deadlyDistance = AnimalDeadlyDistanceFactor; // Sleepy Grizzly Bear 4762
		ObjectData.getObjectData(4762).damage = 5; // Sleepy Grizzly Bear 4762

		ObjectData.getObjectData(632).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Grizzly Bear 1
		ObjectData.getObjectData(632).damage = 6; // Shot Grizzly Bear 1
		ObjectData.getObjectData(635).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Grizzly Bear 2
		ObjectData.getObjectData(635).damage = 7; // Shot Grizzly Bear 2
		ObjectData.getObjectData(637).deadlyDistance = AnimalDeadlyDistanceFactor; // Shot Grizzly Bear 3
		ObjectData.getObjectData(637).damage = 8; // Shot Grizzly Bear 3

		ObjectData.getObjectData(3816).damage = 0.1; // per sec Gushing Knife Wound
		ObjectData.getObjectData(797).damage = 0.05; // per sec Stable Knife Wound
		ObjectData.getObjectData(1380).damage = 0.03; // per sec Clean Knife Wound

		ObjectData.getObjectData(1625).damage = 0.07; // per sec Note Arrow Wound
		ObjectData.getObjectData(798).damage = 0.06; // per sec Arrow Wound
		ObjectData.getObjectData(1365).damage = 0.04; // per sec Embedded Arrowhead Wound
		ObjectData.getObjectData(1367).damage = 0.06; // per sec Extracted Arrowhead Wound
		ObjectData.getObjectData(3817).damage = 0.1; // per sec  Gushing Empty Arrow Wound
		ObjectData.getObjectData(1366).damage = 0.03; // per sec Empty Arrow Wound
		ObjectData.getObjectData(1382).damage = 0.03; // Clean Arrow Wound

		ObjectData.getObjectData(1363).damage = 0.05; // per sec Bite Wound
		ObjectData.getObjectData(1381).damage = 0.03; // per sec Clean Bite Wound

		ObjectData.getObjectData(1377).damage = 0.1; // per sec Snake Bite
		ObjectData.getObjectData(1384).damage = 0.05; // per sec Clean Snake Bite

		ObjectData.getObjectData(1364).damage = 0.05; // per Hog Cut
		ObjectData.getObjectData(1383).damage = 0.03; // per sec Clean Hog Cut

		ObjectData.getObjectData(797).alternativeTimeOutcome = 0; // Stable Knife Wound --> Empty

		ObjectData.getObjectData(1363).alternativeTimeOutcome = 0; // Bite Wound --> Empty

		ObjectData.getObjectData(798).alternativeTimeOutcome = 1367; // Arrow Wound --> Extracted Arrowhead Wound
		ObjectData.getObjectData(1367).alternativeTimeOutcome = 1366; // Extracted Arrowhead Wound --> Empty Arrow Wound
		ObjectData.getObjectData(1366).alternativeTimeOutcome = 0; // Empty Arrow Wound --> Empty

		ObjectData.getObjectData(2396).isBoat = true; // Running Crude Car
		ObjectData.getObjectData(2396).speedMult = 2.5;
		ObjectData.getObjectData(4655).isBoat = true; // Delivery Truck
		ObjectData.getObjectData(4655).speedMult = 1;

		// blocks domestic animal
		ObjectData.getObjectData(1851).decayFactor = ObjDecayFactorOnFloor; // Fence Gate
		ObjectData.getObjectData(1851).decaysToObj = 550; // Fence Gate ==> Fence
		ObjectData.getObjectData(1851).blocksDomesticAnimal = true; // Fence Gate
		ObjectData.getObjectData(1851).blocksAnimal = true; // Fence Gate
		ObjectData.getObjectData(1851).groundOnly = true; // Fence Gate
		ObjectData.getObjectData(1851).rValue = 0.01;

		ObjectData.getObjectData(558).rValue = 0.01; // Open Fence Gate

		// Springy Wooden Door 2762 - not installed
		ObjectData.getObjectData(2762).decaysToObj = 878; // Open Wooden Door Horizontal
		// ObjectData.getObjectData(2762).blocksDomesticAnimal = true;
		// ObjectData.getObjectData(2762).blocksAnimal = true;

		// Horizontal
		ObjectData.getObjectData(2757).rValue = 0.95; // Springy Wooden Door
		ObjectData.getObjectData(2757).decaysToObj = 876; // Springy Wooden Door ==> Wooden Door
		ObjectData.getObjectData(2757).blocksAnimal = true; // Springy Wooden Door

		ObjectData.getObjectData(2758).rValue = 0.2; // Springy Open Wooden Door
		ObjectData.getObjectData(2758).decaysToObj = 876; // Springy Open Wooden Door ==> Wooden Door
		ObjectData.getObjectData(2758).blocksAnimal = true;

		// Springy Wooden Door 2759 Vertical
		ObjectData.getObjectData(2759).decaysToObj = 879; // Open Wooden Door Vertical
		ObjectData.getObjectData(2759).blocksDomesticAnimal = true;
		ObjectData.getObjectData(2759).blocksAnimal = true;

		ObjectData.getObjectData(2760).rValue = 0.2; // Springy Open Wooden Door
		ObjectData.getObjectData(2760).decaysToObj = 879; // Springy Open Wooden Door ==> Open Wooden Door Vertical
		ObjectData.getObjectData(2760).blocksAnimal = true;

		// Hitched Wild Horse 772
		ObjectData.getObjectData(772).rValue = 0.01;
		// Hitched Tame Horse 773
		ObjectData.getObjectData(773).rValue = 0.01;
		// Hitched Riding Horse
		ObjectData.getObjectData(774).rValue = 0.01;
		// Hitched Horse-Drawn Cart 779
		ObjectData.getObjectData(779).rValue = 0.01;
		// Hitched Horse-Drawn Tire Cart 3159
		ObjectData.getObjectData(3159).rValue = 0.01;

		ObjectData.getObjectData(4154).decayFactor = ObjDecayFactorOnFloor; // Hitching Post
		ObjectData.getObjectData(4154).decaysToObj = 556; // Hitching Post  ==> Fence Kit
		ObjectData.getObjectData(4154).groundOnly = true; // Hitching Post
		ObjectData.getObjectData(4154).rValue = 0.01; // Hitching Post

		ObjectData.getObjectData(550).decayFactor = ObjDecayFactorOnFloor; // Fence
		ObjectData.getObjectData(550).decaysToObj = 556; // Fence  ==> Fence Kit
		ObjectData.getObjectData(550).groundOnly = true; // Fence
		ObjectData.getObjectData(550).rValue = 0.01;

		ObjectData.getObjectData(549).decayFactor = ObjDecayFactorOnFloor; // Fence + verticalFence
		ObjectData.getObjectData(549).decaysToObj = 556; //  Fence + verticalFence  ==> Fence Kit
		ObjectData.getObjectData(549).groundOnly = true; // Fence + verticalFence
		ObjectData.getObjectData(549).rValue = 0.01;

		ObjectData.getObjectData(551).decayFactor = ObjDecayFactorOnFloor; // Fence +cornerFence
		ObjectData.getObjectData(551).decaysToObj = 556; // Fence +cornerFence ==> Fence Kit
		ObjectData.getObjectData(551).groundOnly = true; // Fence +cornerFence
		ObjectData.getObjectData(551).rValue = 0.01;

		ObjectData.getObjectData(556).blocksDomesticAnimal = true; // Fence Kit

		// ObjectData.getObjectData(3862).decayFactor = ObjDecayFactorOnFloor; // Dung Box
		ObjectData.getObjectData(3862).decaysToObj = 434; // Dung Box ==> Wooden Box
		// ObjectData.getObjectData(3862).groundOnly = true; // Dung Box

		ObjectData.getObjectData(542).useChance = 0.1; // Domestic Lamb
		ObjectData.getObjectData(604).useChance = 0.1; // Hungry Domestic Lamb
		ObjectData.getObjectData(602).useChance = DomesticAnimalMoveUseChance; // Fed Domestic Lamb 602
		ObjectData.getObjectData(4213).useChance = FedDomesticAnimalMoveUseChance; // Fed Domestic Sheep
		ObjectData.getObjectData(600).useChance = FedDomesticAnimalMoveUseChance; // Domestic Sheep with Lamb

		ObjectData.getObjectData(1459).useChance = DomesticAnimalMoveUseChance; // Domestic Calf 1459
		ObjectData.getObjectData(1462).useChance = DomesticAnimalMoveUseChance; // Hungry Domestic Calf
		ObjectData.getObjectData(1485).useChance = DomesticAnimalMoveUseChance; // Fed Domestic Calf

		for (objData in ObjectData.importedObjectData) {
			if (objData.description.contains('Sports Car')) {
				objData.isBoat = true;
				objData.speedMult = 3.5;
			}
		}

		ObjectData.getObjectData(4647).unreleased = true; // Truck Chassis

		ObjectData.getObjectData(1605).numSlots = 0; // Stack of Baskets // TODO allow stacking of filled baskets

		// A symbol of kingship! Increases prestige from followers if worn.
		// Wolf Crown 695 // Leaf Crown 694  // Carrot Crown 693
		ObjectData.getObjectData(695).extraPrestigeFactor = 0.2;
		ObjectData.getObjectData(694).extraPrestigeFactor = 0.2;
		ObjectData.getObjectData(693).extraPrestigeFactor = 0.2;

		// Increase General prestige gain
		ObjectData.getObjectData(695).prestigeFactor = 1.5;
		ObjectData.getObjectData(694).prestigeFactor = 1.5;
		ObjectData.getObjectData(693).prestigeFactor = 1.5;

		ObjectData.getObjectData(692).prestigeFactor = 1; // Crown Blank

		ObjectData.getObjectData(700).clothing = "n"; // Leaf Crown with Leaf 700 is not to wear

		// ObjectData.getObjectData(279).winterDecayFactor = 2; // Empty Wild Gooseberry Bush
		// ObjectData.getObjectData(279).springRegrowFactor = 0.5; // Empty Wild Gooseberry Bush
		// ObjectData.getObjectData(279).countsOrGrowsAs = 30; // Empty Wild Gooseberry Bush

		// var obj = ObjectData.getObjectData(604); // Hungry Domestic Lamb

		// trace('${obj.description} uses: ${obj.numUses} chance: ${obj.useChance}');

		// var obj = ObjectData.getObjectData(624);

		// trace('${obj.description} uses: ${obj.numUses} chance: ${obj.useChance}');

		// var obj = ObjectData.getObjectData(411);

		// trace('${obj.description} uses: ${obj.numUses} chance: ${obj.useChance}');

		// trace('Insulation: ${ObjectData.getObjectData(128).getInsulation()}'); // Redd Skirt

		// ObjectData.getObjectData(31).writeToFile();

		// trace('Trace: ${ObjectData.getObjectData(8881).description}');

		// trace('Patch: ${ObjectData.getObjectData(942).description}');
		// if (obj.deadlyDistance > 0)
		//    obj.mapChance *= 0;

		// trace('Permanent: ${ObjectData.getObjectData(750).permanent}');
		// trace('Permanent: ${ObjectData.getObjectData(3816).permanent}');

		// var obj = ObjectData.getObjectData(82); // Fire 82
		// trace('Trace: ${obj.name} heat: ${obj.heatValue}');
	}

	public static function PatchTransitions(transitions:TransitionImporter) {
		/*for (trans in transitions.transitions) {
			var actorObjdata = ObjectData.getObjectData(trans.actorID);
			var targetObjdata = ObjectData.getObjectData(trans.targetID);
			if (actorObjdata.numUses > 1 && trans.noUseActor) trace('noUseActor: ' + trans.getDescription());
			if (targetObjdata.numUses > 1 && trans.noUseTarget) trace('noUseTarget: ' + trans.getDescription());
		}*/

		// TODO set through transions
		ObjectData.getObjectData(30).lastUseObject = 279; // Wild Gooseberry Bush ==> Empty Wild Gooseberry Bush
		ObjectData.getObjectData(279).undoLastUseObject = 30; // Empty Wild Gooseberry Bush ==> Wild Gooseberry Bush

		// Hungry Grizzly Bear attacking 653
		// Shot Grizzly Bear 1 632
		// Shot Grizzly Bear 2 635
		// Shot Grizzly Bear 3 637

		// var trans = transitions.getTransition(-1, 628); // Grizzly Bear 628
		// trans.autoDecaySeconds = 3; // default: 3

		var trans = transitions.getTransition(-1, 631); // Hungry Grizzly Bear 631
		trans.autoDecaySeconds = 2.5; // default: 3

		var trans = transitions.getTransition(-1, 761); // Barrel Cactus 761
		trans.autoDecaySeconds = 600; // default: 5 * 60 = 300

		var trans = transitions.getTransition(-1, 282); // Firing Adobe Kiln
		trans.autoDecaySeconds = 40; // default: 30

		var trans = transitions.getTransition(-1, 885); // Stone Wall (Corner) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10
		var trans = transitions.getTransition(-1, 886); // Stone Wall (vertical) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10
		var trans = transitions.getTransition(-1, 887); // Stone Wall (horizontal) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10
		var trans = transitions.getTransition(-1, 884); // Stone Floor ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10 // TODO implement time for floors

		// var trans = new TransitionData(96, 237, 0, 3290); // Pine Needles 96 + Adobe Oven 237  ==> Pine Floor 3290
		// transitions.addTransition("PatchTransitions: ", trans);

		// lower age for weapons since kids so or so make less damage since they have less health pipes
		ObjectData.getObjectData(151).minPickupAge = 10; // 12   // War Sword
		ObjectData.getObjectData(151).minPickupAge = 5; // 10   // Yew Bow
		ObjectData.getObjectData(560).minPickupAge = 2; // 8    // Knife

		// TODO allow damage with bloody weapon / needs support from client?
		ObjectData.getObjectData(560).damage = 5; // Knife
		ObjectData.getObjectData(3047).damage = 6; // War Sword // damage per sec = 3
		ObjectData.getObjectData(152).damage = 9; // Bow and Arrow
		ObjectData.getObjectData(1624).damage = 12; // Bow and Arrow with Note

		var trans = transitions.getTransition(-1, 750); // Bloody Knife
		trans.autoDecaySeconds = 3; // 15

		var trans = transitions.getTransition(-1, 3048); // Bloody War Sword
		trans.autoDecaySeconds = 2; // 10

		var trans = transitions.getTransition(-1, 749); // Bloody Yew Bow
		trans.autoDecaySeconds = 6; // 30

		var trans = transitions.getTransition(560, 180); // Knife 560 + Dead Rabbit 180
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(127, 282); // Adobe 127 + Firing Adobe Kiln 282
		trans.aiShouldIgnore = true; // only do manual

		var trans = transitions.getTransition(236, 2836); // Clay Plate 236 + Tomato 2836
		trans.aiShouldIgnore = true; // only do manual // dont loose the clay plates near tomatos

		// Make reapir fences more easy:
		// Mallet 467 + Fence Kit 556 -->  Mallet 467 + Fence 550
		var trans = new TransitionData(467, 556, 467, 550);
		transitions.addTransition("PatchTransitions: ", trans);

		// give more options to make Threads
		// Drop Spindle 579 + Ball of Thread 1125 -->  Mallet 467 + Thread 58
		var trans = new TransitionData(579, 1125, 579, 58);
		transitions.addTransition("PatchTransitions: ", trans);

		// Knife transitions for close combat
		var trans = new TransitionData(560, 418, 750, 422); // Knife 560 + Wolf ==> Bloody Knife + Dead Wolf
		transitions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(560, 1323, 750, 1332); // Knife + Wild Boar ==> Bloody Knife +  Dead Boar
		transitions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(560, 1328, 750, 1331); // Knife + Wild Boar with Piglet ==> Bloody Knife + Shot Boar with Piglet
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO add graphics for dead cow
		// Knife + Domestic Cow ==> Bloody Knife + Butchered Sheep 587
		// var trans = new TransitionData(560, 1458, 750, 587);
		// Knife + Domestic Cow ==> Knife + Dead Cow 1900
		var trans = new TransitionData(560, 1458, 560, 1900);
		// trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Knife + Domestic Calf 1459 ==> Knife + Dead Domestic Calf 1487
		var trans = new TransitionData(560, 1459, 560, 1487);
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Knife + Hungry Domestic Calf 1462 ==> Knife + Dead Domestic Calf 1487
		var trans = new TransitionData(560, 1462, 560, 1487);
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Knife + Dead Cow 1900 ==> Knife + Butchered Sheep 587
		var trans = new TransitionData(560, 1900, 560, 587);
		transitions.addTransition("PatchTransitions: ", trans);

		// Knife + Dead Domestic Calf 1487 ==> Knife + Butchered Sheep 587
		var trans = new TransitionData(560, 1487, 560, 587);
		trans.targetNumberOfUses = 2; // give only two meat
		transitions.addTransition("PatchTransitions: ", trans);

		// Sword transitions for close combat
		var trans = new TransitionData(3047, 418, 3048, 422); // War Sword + Wolf ==> Bloody War Sword + Dead Wolf
		transitions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(3047, 1323, 3048, 1332); // War Sword + Wild Boar ==> Bloody War Sword +  Dead Boar
		transitions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(3047, 1328, 3048, 1331); // War Sword + Wild Boar with Piglet ==> Bloody War Sword + Shot Boar with Piglet
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO add graphics for dead cow
		// War Sword + Domestic Cow ==> Bloody War Sword + Butchered Sheep 587
		var trans = new TransitionData(3047, 1458, 3048, 587);
		// trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = TransitionImporter.GetTransition(152, 0); // Bow and Arrow + 0
		trans.newActorID = 151; // Yew Bow instead of Yew Bow just shot
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = transitions.getTransition(-1, 400); // Carrot Row
		trans.autoDecaySeconds = 10 * 60; // 5 * 60

		// FIX bug that this bow cannot be used with quiver
		trans = new TransitionData(-1, 493, 0, 151); // Yew Bow just shot --> Yew Bow
		trans.autoDecaySeconds = 2;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = transitions.getTransition(-1, 427); // Attacking Wolf
		trans.autoDecaySeconds = 3;
		trans.move = 5;
		var trans = transitions.getTransition(-1, 428); // Attacking Shot Wolf
		trans.autoDecaySeconds = 3;
		trans.move = 2;

		var trans = transitions.getTransition(-1, 1385); // Attacking Rattle Snake
		trans.autoDecaySeconds = 3;

		var trans = transitions.getTransition(-1, 1333); // Attacking Wild Boar
		trans.autoDecaySeconds = 3;
		var trans = transitions.getTransition(-1, 1334); // Attacking Wild Boar with Piglet
		trans.autoDecaySeconds = 3;

		var trans = transitions.getTransition(-1, 653); // Hungry Grizzly Bear attacking
		trans.autoDecaySeconds = 3;
		// trace('Hungry Grizzly: ${trans.move}');
		var trans = transitions.getTransition(-1, 654); // Shot Grizzly Bear 1 attacking
		trans.autoDecaySeconds = 3;
		var trans = transitions.getTransition(-1, 655); // Shot Grizzly Bear 2 attacking
		trans.autoDecaySeconds = 3;
		var trans = transitions.getTransition(-1, 637); // Shot Grizzly Bear 3 attacking
		trans.autoDecaySeconds = 3;

		// wounds decay differenctly on ground vs on player
		ObjectData.getObjectData(797).alternativeTimeOutcome = 1380; // Stable Knife Wound --> Clean Knife Wound // on player
		trans = new TransitionData(-1, 797, 0, 0); // Stable Knife Wound --> Empty // on ground
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1380, 0, 0); // Clean Knife Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1363).alternativeTimeOutcome = 1381; // Bite Wound --> Clean Bite Wound
		trans = new TransitionData(-1, 1363, 0, 0); //  Bite Wound --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1381, 0, 0); // Clean Bite Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1377).alternativeTimeOutcome = 1384; // Snake Bite -->  Clean Snake Bite
		trans = new TransitionData(-1, 1377, 0, 0); //  Snake Bite --> Empty
		trans.autoDecaySeconds = 20 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1384, 0, 0); //  Clean Snake Bite --> 0
		trans.autoDecaySeconds = 600 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1366).alternativeTimeOutcome = 1383; // Hog Cut --> Clean Hog Cut
		trans = new TransitionData(-1, 1364, 0, 0); // Hog Cut --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1383, 0, 0); // Clean Hog Cut --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1366).alternativeTimeOutcome = 1382; // Empty Arrow Wound --> Clean Arrow Wound
		trans = new TransitionData(-1, 1366, 0, 0); // Empty Arrow Wound --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1382, 0, 0); // Clean Arrow Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transitions.addTransition("PatchTransitions: ", trans);

		// [1367] Extracted Arrowhead Wound
		trans = new TransitionData(-1, 1367, 0, 0); // Extracted Arrowhead Wound --> 0
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = transitions.getTransition(0, 798);
		ObjectData.getObjectData(798).alternativeTimeOutcome = trans.newTargetID; // Arrow Wound --> Embedded Arrowhead Wound
		trans.newTargetID = 1367; // Arrow Wound --> Extracted Arrowhead Wound
		transitions.addTransition("PatchTransitions: ", trans);

		trans = transitions.getTransition(0, 1367);
		ObjectData.getObjectData(1367).alternativeTimeOutcome = trans.newTargetID; // Extracted Arrowhead Wound --> Gushing Empty Arrow Wound
		trans.newTargetID = 1366; // Extracted Arrowhead Wound --> Empty Arrow Wound
		transitions.addTransition("PatchTransitions: ", trans);

		// More decay transitions
		trans = new TransitionData(-1, 798, 0, 1365); // 798 Arrow Wound --> 1365 Embedded Arrowhead Wound
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1365, 0, 0); // 1365 Embedded Arrowhead Wound --> 0
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 421, 0, 422); // 421 Dead Wolf with Arrow --> 422 Dead Wolf
		trans.autoDecaySeconds = -12;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 565, 0, 566); // 565 Butchered Mouflon --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 422, 0, 566); // 422 Dead Wolf --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 423, 0, 566); // 423 Skinned Wolf --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1340, 0, 1343); // 1340 Butchered Pig --> Pig Bones
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clear Bison
		/*trans = new TransitionData(-1, 1438, 0, 1435); // Shot Bison --> Bison
			trans.autoDecaySeconds = -2;
			transitions.addTransition("PatchTransitions: ", trans);
		 */

		ObjectData.getObjectData(1438).secondTimeOutcome = 1435; // Shot Bison --> Bison
		ObjectData.getObjectData(1438).secondTimeOutcomeTimeToChange = 30 * 60;

		ObjectData.getObjectData(1440).secondTimeOutcome = 1436; // Shot Bison with Calf --> Bison
		ObjectData.getObjectData(1440).secondTimeOutcomeTimeToChange = 30 * 60;

		// dead bison already exists
		trans = new TransitionData(-1, 1442, 0, 1444); // Dead Bison arrow 2 --> Dead Bison arrow 1
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1444, 0, 1446); // Dead Bison arrow 1 --> Dead Bison
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1444, 0, 1446); // Dead Bison arrow 1 --> Dead Bison
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1441, 0, 1443); // Dead Bison with Calf arrow 2  --> Dead Bison with Calf arrow 1
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1443, 0, 1445); // Dead Bison with Calf arrow 1  --> Dead Bison with Calf
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1445, 0, 1437); // Dead Bison with Calf  --> Bison Calf
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clear dead Turkey
		trans = new TransitionData(-1, 2176, 0, 2177); // Shot Turkey with Arrow  --> Shot Turkey
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2177, 0, 0); // Shot Turkey --> 0
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2179, 0, 0); // Shot Turkey no feathers --> 0
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clear up Boar
		trans = new TransitionData(-1, 1331, 0, 1335); // Shot Boar with Piglet --> Fleeing Wild Piglet
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1330, 0, 1332); // Shot Boar --> Dead Boar
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1332, 0, 1343); // Dead Boar --> Pig Bones
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		// Mouflon
		trans = new TransitionData(-1, 562, 0, 566); // Skinned Mouflon --> Mouflon Bones
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = transitions.getTransition(-1, 1343); // Pig Bones
		trans.autoDecaySeconds = -4; // default -2

		trans = transitions.getTransition(-1, 891); // 891 Cracking Adobe Wall
		trans.autoDecaySeconds = -24; // default -0.5

		trans = transitions.getTransition(-1, 155); // Adobe Wall
		trans.autoDecaySeconds = -24; // default -10

		for (trans in TransitionImporter.transitionImporter.transitions) {
			/*if(trans.tool){
				trace('DEBUG!!! TOOL: ${trans.getDesciption()}');
			}*/
			/*var targetObj = ObjectData.getObjectData(trans.targetID); 
				var targetChanged = trans.targetID != trans.newTargetID;
				if(trans.noUseTarget == false && targetChanged && targetObj.numUses > 1 && trans.actorID > 0 && trans.reverseUseTarget == false){
					trace('DEBUG!!! numUses > 1: ${trans.getDesciption()}');
			}*/

			if (trans.actorID < -1) {
				// trace('Debug ${trans.getDesciption()}');
				// trans.traceTransition("PatchTransitions: ", true);

				trans.actorID = 0;
				transitions.addTransition("PatchTransitions: ", trans);
				trans.traceTransition("PatchTransitions: ");
			}

			if (trans.autoDecaySeconds == -168) {
				trans.autoDecaySeconds = -24; // use chance 20%: 10 uses / 120 min 0.08 Bowls per min

				// trans.traceTransition("PatchTransitions: ", true);
			}

			// 150 min like deep well // 0.53 bowls per min
			if (trans.autoDecaySeconds == 9000) {
				trans.autoDecaySeconds = 1200; // 20 min one bucket // use chance: 12.5% = 8 Buckets (bucket has 10 Bowls of Water): 80 uses / 20 min = 4 bowls per min

				// trans.traceTransition("PatchTransitions: ", true);
			}

			// Shallow Well 662
			if (trans.autoDecaySeconds == 2160) {
				// 2160 =  36 min // 0.91 bowls per min
				// 720 = 12 min // use chance 3%: 33 uses / 12 min = 2.7 bowls per min
				trans.autoDecaySeconds = 2160; // 720
				// trans.traceTransition("PatchTransitions: ", true);
			}
		}

		// Fix pickup transitions

		// Escaped Horse-Drawn Cart just released
		trans = transitions.getTransition(0, 1422);
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Cart
		trans = transitions.getTransition(0, 780);
		trans.isPickupOrDrop = true;
		// Hitched Horse-Drawn Cart
		trans = transitions.getTransition(0, 779);
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Tire Cart released
		trans = transitions.getTransition(0, 3161);
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Tire Cart
		trans = transitions.getTransition(0, 3157);
		trans.isPickupOrDrop = true;
		// Hitched Horse-Drawn Tire Cart
		trans = transitions.getTransition(0, 3159);
		trans.isPickupOrDrop = true;

		// Written Paper 1618
		trans = transitions.getTransition(1618, -1);
		trans.isPickupOrDrop = true;

		// Rubber Ball 2170 + Paper with Charcoal Writing 1615
		trans = transitions.getTransition(2170, 1615);
		trans.noUseActor = true;
		// trace('clearing: ' + trans.getDescription(true));

		// Graves
		trans = transitions.getTransition(292, 87); // Basket + Fresh Grave
		trans.isPickupOrDrop = true;
		trans = transitions.getTransition(292, 88); // Basket + Grave
		trans.isPickupOrDrop = true;
		trans = transitions.getTransition(292, 89); // Basket + Old Grave
		trans.isPickupOrDrop = true;
		trans = transitions.getTransition(292, 357); // Basket + Bone Pile
		trans.isPickupOrDrop = true;

		trans = transitions.getTransition(356, -1); // Basket of Bones + 0
		trans.isPickupOrDrop = true;

		// Original: Riding Horse: 770 + -1 = 0 + 1421
		trans = new TransitionData(770, 0, 0, 1421);
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO this should function somehow with categories???
		// original transition makes cart loose rubber if putting down horse cart
		// Original: 3158 + -1 = 0 + 1422 // Horse-Drawn Tire Cart + ???  -->  Empty + Escaped Horse-Drawn Cart --> must be: 3158 + -1 = 0 + 3161
		trans = transitions.getTransition(3158, -1); // Horse-Drawn Tire Cart
		trans.newTargetID = 3161;
		trans.traceTransition("PatchTransitions: ");

		// trans = transitions.getTransition(3158, 550); // Horse-Drawn Tire Cart
		// trans.newTargetID = 3161;
		// trace('trans: ${trans.getDesciption()}');

		// original transition makes cart loose rubber if picking up horse cart

		// Original:  0 + 3161 = 778 + 0 //Empty + Escaped Horse-Drawn Tire Cart# just released -->  Horse-Drawn Cart + Empty
		// comes from pattern:  <0> + <1422> = <778> + <0> / EMPTY + Escaped Horse-Drawn Cart# just released -->  Horse-Drawn Cart + EMPTY
		trans = transitions.getTransition(0, 3161);
		trans.newActorID = 3158; // Horse-Drawn Tire Cart
		trans.traceTransition("PatchTransitions: ");

		trans = transitions.getTransition(-1, 3161); // Escaped Horse-Drawn Tire Cart just released
		trans.newTargetID = 3157; // Escaped Horse-Drawn Tire Cart
		trans.autoDecaySeconds = 20; // default 7
		trans.traceTransition("PatchTransitions: ");

		trans = transitions.getTransition(-1, 3157); // Escaped Horse-Drawn Tire Cart
		trans.move = 2; // default 4

		trans = transitions.getTransition(0, 3157);
		trans.newActorID = 3158; // Horse-Drawn Tire Cart
		trans.traceTransition("PatchTransitions: ");

		// let Tule Stumps (122) grow back
		trans = transitions.getTransition(-1, 122);
		trans.newTargetID = 121; // 121 = Tule Reeds
		trans.autoDecaySeconds = -6;
		trans.traceTransition("PatchTransitions: ");

		// Escaped Horse-Drawn Cart just released
		trans = transitions.getTransition(-1, 1422);
		trans.autoDecaySeconds = 15; // 7

		// Escaped Horse-Drawn Cart
		trans = transitions.getTransition(-1, 780);
		trans.move = 2; // default 4

		// Escaped Riding Horse just released 1421
		trans = transitions.getTransition(-1, 1421);
		trans.autoDecaySeconds = 20; // 7

		// Escaped Riding Horse 775
		trans = transitions.getTransition(-1, 775);
		trans.move = 3; // default 4

		// Escaped Horse-Drawn Tire Cart just released??????
		// trans = transitions.getTransition(-1, 1361);
		// trans.autoDecaySeconds = 30;  // 7

		trans = transitions.getTransition(3158, 4154); // Horse-Drawn Tire Cart + Hitching Post
		trans.newTargetID = 3159; // Hitched Horse-Drawn Tire Cart
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		trans = transitions.getTransition(3158, 550); // Horse-Drawn Tire Cart + Fence
		trans.newTargetID = 3159; // Hitched Horse-Drawn Tire Cart
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// 141 Canada Goose Pond
		// 1261 Canada Goose Pond with Egg // TODO let egg come back

		// change decay time for grave 88 = Grave
		// trans = transitions.getTransition(-1, 88);
		// trans.autoDecaySeconds = 10;
		// trans.traceTransition("PatchTransitions: ");

		// should be fixed now with the rest of the -2 transitions
		//-2 + 141 = 0 + 143 // some how we have -2 transactions like hand + ghoose pond = ghoose pond with feathers
		// trans = transitions.getTransition(-2, 141);
		// trans.actorID = 0;
		// transitions.addTransition("PatchTransitions: ", trans);
		// trans.traceTransition("PatchTransitions: ");

		// new bears needs the world
		trans = new TransitionData(-1, 650, 0, 630); // Bear Cave Empty --> Bear Cave
		trans.autoDecaySeconds = -48;
		transitions.addTransition("PatchTransitions: ", trans);

		// Steel Mining Pick 684 + Bear Cave 650 --> Steel Mining Pick 684 + Huge Charcoal Pile 4102
		trans = new TransitionData(684, 650, 684, 4102);
		// trans.aiShouldIgnore = true;
		trans.alternativeTransitionOutcome.push(300); // Big Charcoal Pile 300
		trans.hungryWorkCost = 10;
		transitions.addTransition("PatchTransitions: ", trans);

		// Steel Mining Pick 684 + Bear Cave 650 --> Broken Steel Tool 858+ Huge Charcoal Pile 4102
		trans = new TransitionData(684, 650, 858, 4102);
		// trans.aiShouldIgnore = true;
		trans.lastUseActor = true;
		trans.alternativeTransitionOutcome.push(300); // Big Charcoal Pile 300
		trans.hungryWorkCost = 10;
		transitions.addTransition("PatchTransitions: ", trans);

		// let get berrys back!
		trans = new TransitionData(-1, 30, 0, 30); // Wild Gooseberry Bush
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 279, 0, 30); // Empty Wild Gooseberry Bush --> // Wild Gooseberry Bush
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		// lets get banana back!
		trans = new TransitionData(-1, 2142, 0, 2142); // Banana Plant
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -3;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2145, 0, 2142); // Empty Banana Plant --> Banana Plant
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -3;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 227, 0, 0); // Straw 227
		trans.autoDecaySeconds = -4;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1115, 0, 0); // Dried Ear of Corn 1115
		trans.autoDecaySeconds = -4;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 3180, 0, 291); // Flat Rock with Rabbit Bait 3180 --> Flat Rock 291
		trans.autoDecaySeconds = -4;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1466, 0, 235); // Bowl of Leavened Dough --> Clay Bowl 235
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1201, 0, 236); // Plate of Squash Chunks with Seeds 1201 --> Clay Plate 236
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1202, 0, 236); // Plate of Squash Chunks 1202 --> Clay Plate 236
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1468, 0, 236); // Leavened Dough on Clay Plate 1468 --> Clay Plate 236
		trans.autoDecaySeconds = -1;
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO remove once Garlic is gone
		trans = new TransitionData(-1, 4255, 0, 848); // Mature Garlic --> Hardened Row
		trans.autoDecaySeconds = -12;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 4265, 0, 848); // Mature Garlic 4265--> Hardened Row
		trans.autoDecaySeconds = -12;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 577, 578, 576); // Shorn Domestic Sheep 577 --> Fleece 578 + Shorn Domestic Sheep 576
		trans.autoDecaySeconds = 2;
		trans.move = 1;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 4194, 1262, 1256); // Domestic Goose with Egg 4194 --> Cold Goose Egg 1262 + Domestic Goose 1256
		trans.autoDecaySeconds = 10;
		trans.move = 1;
		transitions.addTransition("PatchTransitions: ", trans);

		// trans = new TransitionData(-1, 577, 576, 578); // Shorn Domestic Sheep 577 --> Fleece 578 + Shorn Domestic Sheep 576
		// trans.autoDecaySeconds = 2;
		// trans.lastUseTarget = true;
		// transitions.addTransition("PatchTransitions: ", trans);

		// get some sharpie back
		trans = new TransitionData(135, 850, 135, 34); // Flint Chip + Stone Hoe --> Flint Chip + Sharp Stone
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(135, 71, 135, 34); // Flint Chip + Stone Hatchet --> Flint Chip + Sharp Stone
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// get some ropes back
		trans = new TransitionData(34, 850, 34, 92); // Sharp Stone + Stone Hoe --> Sharp Stone + Tied Long Shaft
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(850, 850, 850, 92); // Stone Hoe + Stone Hoe --> Stone Hoe + Tied Long Shaft
		trans.aiShouldIgnore = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(850, 235, 850, 126); // Stone Hoe  + Clay Bowl --> Stone Hoe + Clay
		trans.aiShouldIgnore = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(850, 236, 850, 126); // Stone Hoe  + Clay Plate --> Stone Hoe + Clay
		// trans.aiShouldIgnore = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(850, 292, 850, 124); // Stone Hoe  + Basket 292 --> Stone Hoe + Reed Bundle 124
		trans.aiShouldIgnore = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// trans = new TransitionData(0, 92, 59, 67); // 0 + Tied Long Shaft --> Rope + Long Straight Shaft
		// var trans = transitions.getTransition(135, 92); // Flint Chip 135 + Tied Long Shaft 92
		// transitions.addTransition("PatchTransitions: ", trans);

		// trans = new TransitionData(135, 71, 135, 70); // Flint Chip + Stone Hatchet --> Flint Chip + Tied Short Shaft
		trans = new TransitionData(34, 71, 34, 70); // Sharp Stone + Stone Hatchet --> Sharp Stone + Tied Short Shaft
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(866, 82, 0, 83); // Rag Loincloth + Fire --> 0 + Large Fast Fire
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(865, 82, 0, 83); // Rag Shirt + Fire --> 0 + Large Fast Fire
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(864, 82, 0, 83); // Rag Hat + Fire --> 0 + Large Fast Fire
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(869, 82, 0, 83); // Rag Shoe + Fire --> 0 + Large Fast Fire
		transitions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(34, 32, 33, 32); // Sharp Stone + Big Hard Rock --> Stone + Big Hard Rock
		trans.hungryWorkCost = 1;
		transitions.addTransition("PatchTransitions: ", trans);

		// Wild Gooseberry Bush

		// Bowl of Gooseberries + Wild Gooseberry Bush --> Bowl of Gooseberries(+1) + Wild Gooseberry Bush
		trans = new TransitionData(253, 30, 253, 30);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clay Bowl + Wild Gooseberry Bush --> Bowl of Gooseberries + Wild Gooseberry Bush
		trans = new TransitionData(235, 30, 253, 30);
		trans.reverseUseActor = true; // otherwise new bowl will be full with berries
		transitions.addTransition("PatchTransitions: ", trans);

		// Bowl of Gooseberries + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Wild Gooseberry Bush
		trans = new TransitionData(253, 30, 253, 279);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// Clay Bowl + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Wild Gooseberry Bush
		trans = new TransitionData(235, 30, 253, 279);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// Domestic Gooseberry Bush

		// Bowl of Gooseberries + Domestic Gooseberry Bush --> Bowl of Gooseberries(+1) + Domestic Gooseberry Bush
		trans = new TransitionData(253, 391, 253, 391);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clay Bowl + Domestic Gooseberry Bush --> Bowl of Gooseberries + Domestic Gooseberry Bush
		trans = new TransitionData(235, 391, 253, 391);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Bowl of Gooseberries + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Domestic Wild Gooseberry Bush
		trans = new TransitionData(253, 391, 253, 1135);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// Clay Bowl 235 + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Domestic Gooseberry  Bush
		trans = new TransitionData(235, 391, 253, 1135);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// Fill up Bowl of Dry Beans
		// Bowl of Dry Beans 1176 + Dry Bean Plants 1172
		trans = new TransitionData(1176, 1172, 1176, 1172);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Bowl of Dry Beans 1176 + Dry Bean Plants 1172 --> Bowl of Dry Beans 1176 + Hardened Row 848
		trans = new TransitionData(1176, 1172, 1176, 848);
		trans.reverseUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// Clay Bowl 235 + Dry Bean Plants 1172
		trans = new TransitionData(235, 1172, 1176, 1172);
		trans.reverseUseActor = true;
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Clay Bowl 235 + Dry Bean Plants 1172 --> Bowl of Dry Beans 1176 + Hardened Row 848
		trans = new TransitionData(235, 1172, 1176, 848);
		trans.reverseUseActor = true;
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans, false, true);

		// 0 + Bowl of Green Beans
		var trans = transitions.getTransition(0, 1175);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(0, 1175, false, true);
		trans.aiShouldIgnore = true;

		// Bowl of Dry Beans 1176 + Deep Tilled Row 213 --> Dry Planted Beans 1161
		trans = new TransitionData(1176, 213, 1176, 1161);
		transitions.addTransition("PatchTransitions: ", trans);

		// Bowl of Dry Beans 1176 + Deep Tilled Row 213 --> Clay Bowl 235 + Dry Planted Beans 1161
		trans = new TransitionData(1176, 213, 235, 1161);
		trans.lastUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans, true);

		// Bowl of Soaking Beans 1180 + Hot Adobe Oven 250 ==> Bowl of Cooked Beans 1292
		trans = new TransitionData(1180, 250, 1292, 250);
		transitions.addTransition("PatchTransitions: ", trans);

		// Fishing Pole without Hook + Bone Needle --> Fishing Pole + 0
		trans = new TransitionData(2092, 191, 2091, 0);
		transitions.addTransition("PatchTransitions: ", trans);

		// 0 + Fishing Pole with Old Boot --> Old Boot + Fishing Pole
		trans = new TransitionData(0, 2098, 2099, 2091);
		transitions.addTransition("PatchTransitions: ", trans);

		// 0 + Diesel Mining Pick without Bit --> Diesel Engine + Collapsed Iron Mine
		trans = new TransitionData(0, 3130, 2365, 945);
		transitions.addTransition("PatchTransitions: ", trans);

		// 0 + Ready Diesel Mining Pick --> Steel Chisel + Diesel Mining Pick without Bit
		trans = new TransitionData(0, 3129, 455, 3130);
		transitions.addTransition("PatchTransitions: ", trans);

		// 0 + Dry Diesel Water Pump --> Diesel Engine + Unpowered Pump Head
		trans = new TransitionData(0, 2388, 2365, 3964);
		transitions.addTransition("PatchTransitions: ", trans);

		// hungry work transitions
		var trans = transitions.getTransition(502, 122); // Shovel + Tule Stumps ==> Adobe
		trans.hungryWorkCost = 5;
		var trans = transitions.getTransition(0, 125); // 0 + Clay Deposit ==> Clay
		trans.hungryWorkCost = 3;
		var trans = transitions.getTransition(0, 409); // 0 + Clay Pit ==> Clay
		trans.hungryWorkCost = 3;
		var trans = transitions.getTransition(502, 32); // Shovel + Big Hard Rock ==> Dug Big Hard Rock
		trans.hungryWorkCost = 10;
		var trans = transitions.getTransition(291, 486); // Flat Rock + Floor Stakes ==> Stone Road
		trans.hungryWorkCost = 5;
		var trans = transitions.getTransition(684, 1596); // Steel Mining Pick + Stone Road ==> Flat Rock
		trans.hungryWorkCost = 5;
		// var trans = transitions.getTransition(33, 32); // Stone + Big Hard Rock ==> Sharp Stone
		// trans.hungryWorkCost = 5;

		// Steel Mining Pick 684 + Ancient Stone Wall H 896 ==> Stone Wall
		// trans = new TransitionData(684, 896, 684, 887);
		var trans = transitions.getTransition(684, 896);
		trans.hungryWorkCost = 10;
		trans.alternativeTransitionOutcome.push(0);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Steel Mining Pick 684 + Ancient Stone Wall C 896 ==> Stone Wall
		// trans = new TransitionData(684, 895, 684, 885);
		var trans = transitions.getTransition(684, 895);
		trans.hungryWorkCost = 10;
		trans.alternativeTransitionOutcome.push(0);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Steel Mining Pick 684 + Ancient Stone Wall V 897 ==> Stone Wall
		// trans = new TransitionData(684, 897, 684, 886);
		var trans = transitions.getTransition(684, 897);
		trans.hungryWorkCost = 10;
		trans.alternativeTransitionOutcome.push(0);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Short Shaft 69 + Fence 549 - vertical--> 0 + Fence Gate 4143
		trans = new TransitionData(69, 549, 0, 4143);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Steel Adze 462 + Springy Wooden Door - horizontal 2757
		var trans = transitions.getTransition(462, 2757);
		trans.hungryWorkCost = 5;
		trans.alternativeTransitionOutcome.push(0);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Steel Adze 462 + Springy Wooden Door - vertical 2759
		var trans = transitions.getTransition(462, 2759);
		trans.hungryWorkCost = 5;
		trans.alternativeTransitionOutcome.push(0);
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// most important allow kill moskitos
		// Firebrand + Mosquito Swarm --> Long Straight Shaft 67 + Ashes
		trans = new TransitionData(248, 2156, 67, 86);
		trans.hungryWorkCost = 5;
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Firebrand + Mosquito Swarm just bit --> 0 + Ashes
		trans = new TransitionData(248, 2157, 0, 86);
		trans.hungryWorkCost = 3;
		transitions.addTransition("PatchTransitions: ", trans, false, false);

		// Bowl of Gooseberries 253 + Carrot Pile 2742
		var trans = transitions.getTransition(253, 2742);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		var trans = transitions.getTransition(253, 2742, false, true);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		// Bowl of Gooseberries 253 + Pile of Wild Carrots 3978
		var trans = transitions.getTransition(253, 3978);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		// Bowl of Gooseberries 253 + Pile of Wild Carrots 3978
		var trans = transitions.getTransition(253, 3978, false, true);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		// Bowl of Soil + Fertile Soil Pile 1101
		var trans = transitions.getTransition(1137, 1101);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		var trans = transitions.getTransition(1137, 1101, false, true);
		trans.targetMinUseFraction = 0;

		// Clay Bowl 235 + Fertile Soil Pile 1101
		var trans = transitions.getTransition(235, 1101);
		trans.targetMinUseFraction = 0; // TODO how could it work in vanilla?

		var trans = transitions.getTransition(235, 1101, false, true);
		trans.targetMinUseFraction = 0;

		// Bowl of Soil + Hardened Row --> Shallow Tilled Row
		var trans = transitions.getTransition(1137, 848);
		trans.hungryWorkCost = -5; // dont let is cost hungry work

		// Mallet + Dug Big Rock with Chisel -- Split Big Rock
		var trans = transitions.getTransition(467, 508);
		trans.hungryWorkCost = 10;

		// give wolfs some meat // TODO change crafting maps
		var trans = transitions.getTransition(0, 423); // 423 Skinned Wolf
		trans.newTargetID = 565; // 565 Butchered Mouflon
		trans.targetNumberOfUses = 2; // give only two meat
		transitions.addTransition("PatchTransitions: ", trans);

		// Give Bears some meat
		var trans = transitions.getTransition(0, 657); // Skinned Bear with hide
		trans.newTargetID = 1340; // Butchered Pig 1340
		transitions.addTransition("PatchTransitions: ", trans);

		// Give Dead Bison some meat
		var trans = new TransitionData(560, 1446, 560, 1340); // Knife + Dead Bison 1446 --> Knife + Butchered Pig 1340
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = transitions.getTransition(0, 709); // 709 Skinned Seal with fur
		trans.newTargetID = 1340; // 1340 Butchered Pig
		transitions.addTransition("PatchTransitions: ", trans);

		// give bison some meat // TODO change crafting maps
		var trans = transitions.getTransition(0, 1444); // Dead Bison
		trans.newTargetID = 565; // 565 Butchered Mouflon
		transitions.addTransition("PatchTransitions: ", trans);

		// allow to cook mutton on coals
		trans = new TransitionData(569, 85, 570, 85); // 569 Raw Mutton + 85 Hot Coals --> 570 Cooked Mutton + 85 Hot Coals
		transitions.addTransition("PatchTransitions: ", trans);

		// patch alternativeTransitionOutcomes // TODO use prob categories instead
		var trans = transitions.getTransition(502, 338); // shovel 502 + Stump
		trans.alternativeTransitionOutcome.push(72); // Kindling

		var trans = transitions.getTransition(502, 408); // shovel 502 + Empty Clay Pit 408
		trans.alternativeTransitionOutcome.push(126); // Clay 126
		trans.hungryWorkCost = ServerSettings.HungryWorkCost;

		var trans = transitions.getTransition(502, 408); // shovel 502 + Empty Clay Pit 408
		trans.alternativeTransitionOutcome.push(126); // Clay 126
		trans.hungryWorkCost = ServerSettings.HungryWorkCost;

		// Allow to make kindling out of Skewers
		var trans = new TransitionData(334, 852, 334, 72); // Steel Axe + Weak Skewer --> Kindling
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(71, 852, 71, 72); // Stone Hatchet + Weak Skewer --> Kindling
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(344); // Chopped Softwood Tree --> Fire Wood

		// push twice so that it has twice the chance
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(344); // Chopped Softwood Tree --> Fire Wood

		// push tripple so that it has tripple the chance
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(344); // Chopped Softwood Tree --> Fire Wood

		// push Kindling
		// ObjectData.getObjectData(342).alternativeTransitionOutcome.push(72); // Chopped Tree Big Log--> Kindling
		// ObjectData.getObjectData(340).alternativeTransitionOutcome.push(72); // Chopped Tree --> Kindling
		// ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(72); // Chopped Softwood Tree --> Kindling

		// now push Butt Log
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(345); // Chopped Tree Big Log--> Butt Log
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(345); // Chopped Tree --> Butt Log
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(345); // Chopped Softwood Tree --> Butt Log

		// ObjectData.getObjectData(99).alternativeTransitionOutcome.push(344); // White Pine Tree --> Fire Wood
		// ObjectData.getObjectData(100).alternativeTransitionOutcome.push(344); // White Pine Tree with Needles --> Fire Wood

		ObjectData.getObjectData(3146).hungryWork = ServerSettings.HungryWorkCost; // Chopped Softwood Tree

		// Cut Stones 1853
		ObjectData.getObjectData(1853).hungryWork = ServerSettings.HungryWorkCost;
		ObjectData.getObjectData(1853).alternativeTransitionOutcome.push(33); // Stone
		ObjectData.getObjectData(1853).alternativeTransitionOutcome.push(0); // Stone
		ObjectData.getObjectData(1853).alternativeTransitionOutcome.push(0); // Stone

		// ObjectData.getObjectData(99).hungryWork = ServerSettings.HungryWorkCost; // White Pine Tree
		// ObjectData.getObjectData(100).hungryWork = ServerSettings.HungryWorkCost; // White Pine Tree with Needles

		// ObjectData.getObjectData(3944).alternativeTransitionOutcome.push(33); // Stripped Iron Vein --> Stone
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(33); // Iron Vein --> Stone
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(0); // Iron Vein --> 0
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(0); // Iron Vein --> 0

		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(33); // Shallow Pit with Ore --> Stone
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(33); // Shallow Pit with Ore --> Stone
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(291); // Shallow Pit with Ore --> Flat Rock

		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(0); // Deep Pit with Ore --> 0
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(0); // Deep Pit with Ore --> 0
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(0); // Deep Pit with Ore --> 0
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(0); // Deep Pit with Ore --> 0
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(33); // Deep Pit with Ore --> Stone
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(33); // Deep Pit with Ore --> Stone
		ObjectData.getObjectData(3958).alternativeTransitionOutcome.push(291); // Deep Pit with Ore --> Flat Rock

		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(0); // Mine with Ore --> 0
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Flat Rock
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(291); // Mine with Ore --> Flat Rock
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(503); // Mine with Ore --> Dug Big Rock

		// ObjectData.getObjectData(944).alternativeTransitionOutcome.push(291); // Iron Mine --> Flat Rock
		// TODO what to do with Diesel Mining Pick with Iron. It uses a time transition

		// allow more Stone Hoe to be used to dig graves // TODO make more HUNGRY WORK / TEST if they brake

		var trans = new TransitionData(850, 357, 850, 1011); // Stone Hoe + Bone Pile --> Stone Hoe + Buried Grave
		trans.tool = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 87, 850, 1011); // Stone Hoe + Fresh Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 88, 850, 1011); // Stone Hoe + Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 89, 850, 1011); // Stone Hoe + Old Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		trans.noUseActor = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// allow more options to kill animals
		var trans = new TransitionData(152, 427, 151, 420); // Bow and Arrow + Attacking Wolf --> Yew Bow + Shot Wolf
		transitions.addTransition("PatchTransitions: ", trans);

		// FIX bucket transition // TODO why is this one missing?
		// <394> + <1099> = <394> + <660> --> <394> + <1099> = <394> + <1099> // make bucket not full
		var trans = new TransitionData(394, 1099, 394, 1099);
		trans.targetRemains = true;
		TransitionImporter.transitionImporter.createAndaddCategoryTransitions(trans);

		// pond animations
		/*
			var trans = transitions.getTransition(-1, 141); // Canada Goose Pond
			trans.newTargetID = 142; // Canada Goose Pond swimming
			trans.autoDecaySeconds = 5;
			transitions.addTransition("PatchTransitions: ", trans);
		 */

		var trans = transitions.getTransition(-1, 142); // Canada Goose Pond swimming
		trans.newTargetID = 141; // Canada Goose Pond
		trans.autoDecaySeconds = 20;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = transitions.getTransition(-1, 2180); // longer clothing decay Rabbit Fur Hat with Feather
		trans.autoDecaySeconds = -240; // -5

		var trans = transitions.getTransition(-1, 712); // Sealskin Coat
		trans.autoDecaySeconds = -240; // -5

		var trans = transitions.getTransition(-1, 2181); // Straw Hat with Feather
		trans.autoDecaySeconds = -240; // -5

		// Mouflon Hide 564 --> vanilla: 10h
		// var trans = transitions.getTransition(-1, 593); // Sheep Skin 593
		// trans.autoDecaySeconds = -240; // -5

		// give more time
		var trans = transitions.getTransition(-1, 304); // Firing Forge 304
		trans.autoDecaySeconds = 40; // normal 30

		var trans = transitions.getTransition(-1, 61); // Juniper Tinder
		trans.autoDecaySeconds = 5 * 60;

		var trans = transitions.getTransition(-1, 62); // Leaf
		trans.autoDecaySeconds = 5 * 60; // normal 2 * 60

		var trans = transitions.getTransition(-1, 75); // Ember Shaft
		trans.autoDecaySeconds = 20;

		var trans = transitions.getTransition(-1, 248); // Firebrand
		trans.autoDecaySeconds = 1.5 * 60;

		var trans = transitions.getTransition(-1, 80); // Burning Tinder
		trans.autoDecaySeconds = 15;

		var trans = transitions.getTransition(-1, 249); // Burning Adobe Oven
		trans.autoDecaySeconds = 25;

		var trans = transitions.getTransition(-1, 1281); // Cooked Omelette
		trans.autoDecaySeconds = 20;

		var trans = transitions.getTransition(-1, 861); // Old Hand Cart
		trans.autoDecaySeconds = -12; // original: -0.5

		var trans = transitions.getTransition(-1, 846); // Broken Hand Cart
		trans.autoDecaySeconds = -2;

		var trans = TransitionImporter.GetTransition(-1, 330); // TIME + Hot Steel Ingot on Flat Rock
		trans.autoDecaySeconds = 20;

		var trans = TransitionImporter.GetTransition(-1, 252); // TIME + Bowl of Dough
		trans.autoDecaySeconds = 120;

		// var trans = TransitionImporter.GetTransition(-1, 1135); // TIME + Empty Domestic Gooseberry Bush
		// trans.autoDecaySeconds = 60  * 10;
		var trans = TransitionImporter.GetTransition(-1, 389); // TIME + Dying Gooseberry Bush
		trans.autoDecaySeconds = -48; // old -1

		var trans = new TransitionData(-1, 1284, 0, 291); // TIME + Cool Flat Rock --> 0 + Flat Rock
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		// Straw 227 + Rope 59 = 0 + Reed Skirt 128
		var trans = new TransitionData(227, 59, 0, 128);
		trans.autoDecaySeconds = -2;
		transitions.addTransition("PatchTransitions: ", trans);

		// Broken Steel Tool 858
		var transByActor = TransitionImporter.GetTransitionByNewActor(858);
		for (trans in transByActor) {
			var objData = ObjectData.getObjectData(trans.actorID);
			// trace('Broken Steel Tool: ${objData.name}');
			objData.decaysToObj = 858; // 858 Broken Steel Tool
			trans.aiShouldIgnore = true;
		}

		// Broken Steel Tool no wood 862
		var transByActor = TransitionImporter.GetTransitionByNewActor(862);
		for (trans in transByActor) {
			var objData = ObjectData.getObjectData(trans.actorID);
			// trace('Broken Steel Tool: ${objData.name}');
			objData.decaysToObj = 862; // Broken Steel Tool no wood 862
			trans.aiShouldIgnore = true;
		}

		var transByTarget = TransitionImporter.GetTransitionByTarget(3076);
		for (trans in transByTarget) {
			// Leaf 62
			if (trans.actorID == 62) continue;
			// Clump of Scrap Steel 930
			if (trans.actorID == 930) continue;
			// trace('Scrap Bowl: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		for (objData in ObjectData.importedObjectData) {
			if (objData.description.contains('Wool') == false) continue;
			// trace('Wool: ${objData.name}');

			var trans = TransitionImporter.GetTransition(-1, objData.parentId); // TIME + Wool???
			if (trans == null) continue;
			if (trans.autoDecaySeconds >= 0) continue;

			// trace('Wool: ${objData.name} decaytime: ${trans.autoDecaySeconds}');
			trans.autoDecaySeconds = ServerSettings.WoolClothDecayTime; // -5
		}

		for (objData in ObjectData.importedObjectData) {
			if (objData.description.contains('Rabbit Fur') == false) continue;

			var trans = TransitionImporter.GetTransition(-1, objData.parentId); // TIME + Wool???
			if (trans == null) continue;
			if (trans.autoDecaySeconds != -5) continue;

			// trace('Rabbit Fur: ${objData.name}');

			// trace('Wool: ${objData.name} decaytime: ${trans.autoDecaySeconds}');
			trans.autoDecaySeconds = ServerSettings.RabbitFurClothDecayTime;
		}

		// var trans = TransitionImporter.GetTransition(-1, 766); // TIME + Snake Skin Boot
		var trans = new TransitionData(-1, 766, 0, 0); // TIME + Snake Skin Boot
		trans.autoDecaySeconds = -24 * 30; // -5
		transitions.addTransition("PatchTransitions: ", trans);

		// var trans = TransitionImporter.GetTransition(-1, 2887); // TIME + Sandal
		// trans.autoDecaySeconds = -24 * 30; // -5

		var trans = TransitionImporter.GetTransition(-1, 866); // TIME + Rag Loincloth
		trans.autoDecaySeconds = -2; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 865); // TIME + Rag Shirt
		trans.autoDecaySeconds = -2; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 869); // TIME + Rag Shoe
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 864); // TIME + Rag Hat
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 2723); // TIME + Dry Juniper Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1872); // TIME + Dry Mango Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1802); // TIME + Dry Maple Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 4311); // TIME + Dry Bay Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 3069); // TIME + Dry Rubber Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 2723); // TIME + Dry Juniper Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1805); // TIME + Dry Yew Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1804); // TIME + Dry Pine Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1803); // TIME + Dry Poplar Sapling
		trans.autoDecaySeconds = -24; // -0.33

		var trans = TransitionImporter.GetTransition(-1, 1825); // TIME + Dry Ancient Yew Bonsai
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 1823); // TIME  + Dry Pruned Old Yew Bonsai
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 1820); // TIME + Dry Old Yew Bonsai
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 1818); // TIME + Dry Pruned Yew Bonsai
		trans.autoDecaySeconds = -24; // -0.5

		var trans = TransitionImporter.GetTransition(-1, 1814); // TIME + Dry Young Yew Bonsai in Bowl
		trans.autoDecaySeconds = -1; // 10min

		var trans = new TransitionData(462, 846, 462, 67); // Steel Adze + Broken Hand Cart ==> Steel Adze + Long Straight Shaft
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(0, 3425, 3425, 0); // Domestic Cow on Rope + 0 = Domestic Cow on Rope * 0
		transitions.addTransition("PatchTransitions: ", trans);

		// Set Max Use Target tranistions // uses now Min use fraction like vanilla
		/*var trans = TransitionImporter.GetTransition(253, 40); // Bowl of Gooseberries + Wild Carrot
			trans.isActorMaxUse = true;
			var trans = TransitionImporter.GetTransition(253, 3978); // Bowl of Gooseberries + Pile of Wild Carrots
			trans.isActorMaxUse = true;
			var trans = TransitionImporter.GetTransition(253, 3978, false, true); // Bowl of Gooseberries + Pile of Wild Carrots
			trans.isActorMaxUse = true;
			var trans = TransitionImporter.GetTransition(40, 253); // Wild Carrot + Bowl of Gooseberries
			trans.isTargetMaxUse = true;
			var trans = TransitionImporter.GetTransition(33, 1176); // Stone + Bowl of Dry Beans
			trans.isTargetMaxUse = true;
			var trans = TransitionImporter.GetTransition(181, 253); // Skinned Rabbit + Bowl of Gooseberries
			trans.isTargetMaxUse = true;
			var trans = TransitionImporter.GetTransition(402, 253); // Carrot + Bowl of Gooseberries
			trans.isTargetMaxUse = true;
		 */

		// Fed Mouflon Lamb 601
		var trans = TransitionImporter.GetTransition(-1, 601, false, true);
		// trace('Fed Mouflon Lamb: ${trans.newTargetID}');
		trans.newTargetID = 575; // Domestic Sheep 575
		transitions.addTransition("PatchTransitions: ", trans); // TODO remove Fed Mouflon Lamb to Mouflon from transition Map

		// new smithing transitions
		var trans = new TransitionData(1603, 235, 1603, 0); // Stack of Clay Bowls + Clay Bowl --> Stack of Clay Bowls +  0
		trans.reverseUseActor = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 316, 1602,
			319); // Stack of Clay Plates + Crucible with Iron and Charcoal --> Stack of Clay Plates +  Unforged Sealed Steel Crucible
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 316, 236,
			319); // Stack of Clay Plates + Crucible with Iron and Charcoal --> Clay Plate +  Unforged Sealed Steel Crucible
		trans.lastUseActor = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(236, 322, 1602, 325); // Clay Plate + Forged Steel Crucible --> CStack of Clay Plates + Crucible with Steel
		trans.reverseUseActor = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 322, 1602, 325); // Stack of Clay Plates + Forged Steel Crucible --> CStack of Clay Plates + Crucible with Steel
		trans.reverseUseActor = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 236, 1602, 0); // Stack of Clay Plates + Clay Plate --> CStack of Clay Plates + 0
		trans.reverseUseActor = true;
		trans.tool = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Butter Knife 1467 + Clay Bowl 235 --> Knife 560 + Bowl of Butter 1465
		var trans = new TransitionData(1467, 235, 560, 1465);
		trans.aiShouldIgnore = true;
		trans.reverseUseTarget = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Drop Spindle 579 + Small Ball of Yarn 581 --> Drop Spindle 579 + Rope 59
		var trans = new TransitionData(579, 581, 579, 59);
		transitions.addTransition("PatchTransitions: ", trans);

		// Smithing Hammer 441 + Stakes 107 -->  Smithing Hammer + Ember Leaf 77
		var trans = new TransitionData(441, 107, 441, 77);
		transitions.addTransition("PatchTransitions: ", trans);

		// Ember Leaf 77 + Straw 227 --> 0 + Smoldering Tinder
		var trans = new TransitionData(77, 227, 0, 78);
		transitions.addTransition("PatchTransitions: ", trans);

		// Straw 227 + Ashes 86 --> 0 + Smoldering Tinder
		var trans = new TransitionData(227, 86, 0, 78);
		transitions.addTransition("PatchTransitions: ", trans);

		// Straw 227 + Fire 82 --> 0 + Flash Fire 3029
		var trans = new TransitionData(227, 82, 0, 3029);
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Basket of Charcoal 298 + Fire 82 --> Basket + Large Slow Fire 346
		var trans = new TransitionData(298, 82, 292, 346);
		trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO change AI to use new 0 + Basket of Charcoal 298
		// Basket of Charcoal 298 + 0 --> Basket + Big Charcoal Pile 300
		var trans = new TransitionData(298, 0, 292, 300);
		transitions.addTransition("PatchTransitions: ", trans);

		// TODO change AI to use new 0 + Cooked Goose
		// Cooked Goose 517 + 0 --> Weak Skewer + Cooked Goose
		var trans = new TransitionData(517, 0, 852, 518);
		transitions.addTransition("PatchTransitions: ", trans);

		// Basket of Soil 336 + 0 --> Basket 292 + Fertile Soil Pile 1101
		var trans = new TransitionData(336, 0, 292, 1101);
		transitions.addTransition("PatchTransitions: ", trans);

		// TODo needs client change
		// var trans = new TransitionData(298, 317, 298, 316); // 298 Basket of Charcoal + 317 Crucible with Iron --> 298 +  316 Crucible with Iron and Charcoal
		// transitions.addTransition("PatchTransitions: ", trans);

		// TODO dont know why this was 2240 Newcomen Hammer instead?
		var trans = TransitionImporter.GetTransition(59, 2245); // Rope + Newcomen Engine without Rope
		trans.newTargetID = 2244; // Newcomen Engine without Shaft;

		// var trans = transitions.getTransition(560, 614); // Knife + Fed Shorn Domestic Sheep 614
		// trans.aiShouldIgnore = true;

		// Ai should ignore
		// TODO fix Ai craftig if Ai needs two threads for a rope it puts one thread in a bowl and gets it out again
		// this breals making a light pulb for a radio
		var trans = transitions.getTransition(58, 235); // Thread + Clay Bowl
		trans.aiShouldIgnore = true;

		// dont deconstruct tools
		var trans = transitions.getTransition(135, 74); // Flint Chip + Fire Bow Drill
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 74); // Knife + Fire Bow Drill
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(461, 3371); // Bow Saw + Table
		trans.aiShouldIgnore = true;

		// Forbid some transition to make Kindling
		var trans = transitions.getTransition(71, 67); // Stone Hatchet + Long Straight Shaft
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(334, 67); // Steel Axe + Long Straight Shaft
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(334, 2142); // Steel Axe + Banana Plant
		trans.aiShouldIgnore = true;
		trans.isForbidden = true;
		var trans = transitions.getTransition(334, 2145); // Steel Axe + Empty Banana Plant
		trans.aiShouldIgnore = true;
		trans.hungryWorkCost = 10;
		// var trans = transitions.getTransition(334, 239); // Steel Axe + Wooden Tongs
		// trans.aiShouldIgnore = true;
		// var trans = transitions.getTransition(71, 239); // Stone Hatchet + Wooden Tongs
		// trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(334, 583); // Steel Axe + Knitting Needles
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(71, 583); // Stone Hatchet + Knitting Needles
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(560, 575); // Knife + Domestic Sheep
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 4213); // Knife + Fed Domestic Sheep 4213
		trans.aiShouldIgnore = true;

		// Is deleted now
		// var trans = transitions.getTransition(568, 4213); // Shears 568 + Fed Domestic Sheep 4213
		// trans.aiShouldIgnore = true;

		// no last use actor?
		// var trans = transitions.getTransition(568, 4213, true); // Shears 568 + Fed Domestic Sheep 4213
		// trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(560, 576); // Knife + Shorn Domestic Sheep
		// trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(152, 531); // Bow and Arrow + Mouflon
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 541); // Knife + Domestic Mouflon
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(560, 151); // Knife + Yew Bow 151
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 708); // Knife + Clubbed Seal 708
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 242); // Knife + Ripe Wheat 242
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 121); // Knife + Tule Reeds
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(560, 2765); // Knife + Sugarcane 2765
		trans.aiShouldIgnore = true;

		// TODO might be good to save some hungry work?
		// var trans = transitions.getTransition(560, 136); // Knife + Sapling 136
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(2365, 3966); // 2365 Diesel Engine + 3966 Empty Scrap Box
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(345, 82); // Butt Log + Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(345, 83); // Butt Log + Large Fast Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(345, 3029); // Butt Log + Flash Fire
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(135, 151); // Flint Chip + Yew Bow
		// trans.aiShouldIgnore = true;

		// allow shovel again if it is better then sharp stone
		var trans = transitions.getTransition(502, 36); // Shovel + Seeding Wild Carrot
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(502, 404); // Shovel + Wild Carrot
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(502, 804); // Shovel + Burdock
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(67, 3065); // Long Straight Shaft + Wooden Slot Box
		trans.aiShouldIgnore = true; // this would give a thread Ai wants

		var trans = transitions.getTransition(0, 2244); // 0 + Newcomen Engine without Shaft
		trans.aiShouldIgnore = true; // Ai would kill for a rope

		var trans = transitions.getTransition(33, 127); // Stone + Adobe = 231 Adobe Oven Base
		if (AIAllowBuildOven == false) trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(127, 237); // Adobe + Adobe Oven = 238 Adobe Kiln
		if (AIAllowBuilKiln == false) trans.aiShouldIgnore = true;

		// Stop spread of Dough to get a bowl // TODO allow again for tortilla
		var trans = transitions.getTransition(252, 291); // Bowl of Dough + Flat Rock
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(252, 291, true); // Bowl of Dough + Flat Rock
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 1471); // 0 + Sliced Bread
		trans.aiShouldIgnore = true; // they make a mess to get the plate

		var trans = transitions.getTransition(0, 1471, false, true); // 0 + Sliced Bread
		trans.aiShouldIgnore = true; // they make a mess to get the plate

		// forbid burning stuff
		var trans = transitions.getTransition(516, 82); // Skewered Goose + Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(516, 83); // Skewered Goose + Large Fast Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(516, 346); // Skewered Goose + Large Slow Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(516, 3029); // Skewered Goose + Flash Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(185, 82); // Skewered Rabbit + Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(185, 83); // Skewered Rabbit + Large Fast Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(185, 346); // Skewered Rabbit + Large Slow Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(185, 3029); // Skewered Rabbit + Flash Fire
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(107, 279); // Stakes + Empty Wild Gooseberry Bush
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(71, 107); // Stone Hatchet + Stakes
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(71, 107, true); // Stone Hatchet + Stakes
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 107); // Steel Axe + Stakes
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(334, 107, true); // Steel Axet + Stakes
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(107, 392); // Stakes + Languishing Domestic Gooseberry Bush
		trans.aiShouldIgnore = true;

		var trans = TransitionImporter.GetTransition(502, 389); // Shovel + Dying Gooseberry Bush 389
		trans.aiShouldIgnore = true;

		// does not exist
		// var trans = TransitionImporter.GetTransition(502, 389, true); // Shovel + Dying Gooseberry Bush 389
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(139, 1136); // Skewer + Shallow Tilled Row
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(852, 1136); // Weak Skewer + Shallow Tilled Row
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(139, 1138); // Skewer + Fertile Soil
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(852, 1138); // Weak Skewer + Fertile Soil
		trans.aiShouldIgnore = true;

		// Forbid plowing of Soil Pile
		var trans = transitions.getTransition(139, 1101); // Skewer + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(852, 1101); // Weak Skewer + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(850, 1101); // Stone Hoe + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(857, 1101); // Steel Hoe + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		// stop picking up same soil again with basket
		var trans = transitions.getTransition(292, 1101); // Basket 292 + Fertile Soil Pile 1101
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 253); // 0 + Bowl of Gooseberries
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 253, false, true); // 0 + Bowl of Gooseberries
		trans.aiShouldIgnore = true;

		// let the kindling in the oven
		var trans = transitions.getTransition(0, 247); // 0 + Wood-filled Adobe Oven
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 281); // 0 + Wood-filled Adobe Kiln
		trans.aiShouldIgnore = true;

		// AI tries to empty popcorn to get a bowl
		var trans = transitions.getTransition(192, 1121); // Needle and Thread + Popcorn
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(192, 1121, false, true); // Needle and Thread + Popcorn
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 3308); // Steel Axe + Marked Pine Wall (corner)
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 3309); // Steel Axe + Marked Pine Wall (vertical)
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 3310); // Steel Axe + Marked Pine Wall (horizontal)
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 1876); // Steel Axe + Languishing Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 1922); // Steel Axe + Dry Fertile Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 1923); // Steel Axe + Wet Fertile Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(334, 344); // Steel Axe + Firewood 344
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(71, 344); // Stone Hatchet + Firewood 344
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(152, 0); // Bow and Arrow + 0
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 2268); // 0 + Bore Mechanism
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(288, 238); // Bellows 288 + Adobe Kiln 238
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(288, 299); // Bellows 288 + Adobe Kiln with Charcoal 299
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 303); // 0 + Forge = Adobe Kiln
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(292, 305); // Basket 292 + Forge with Charcoal 305
		trans.aiShouldIgnore = true;

		// AI wants to get rid of water to get empty bowls
		var trans = transitions.getTransition(1620, 382); // Wood Shavings 1620 + Bowl of Water 382
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(239, 309); // Wooden Tongs 239 + Hot Iron Bloom on Flat Rock 309
		trans.aiShouldIgnore = true;

		// protect smithing TODO allow for smithing or manually instruct
		// var trans = transitions.getTransition(0, 322); // 0 + Forged Steel Crucible
		// trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(0, 325); // 0 + Crucible with Steel
		// trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 316); // 0 + Crucible with Iron and Charcoal
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 318); // 0 + Crucible with Charcoal
		trans.aiShouldIgnore = true;

		// AI might use to empty bowl
		var trans = transitions.getTransition(33, 318); // Stone + Crucible with Charcoal
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 675); // 0 + Bowl of Limestone 675
		trans.aiShouldIgnore = true;

		// dont destroy knifes to get the material
		var trans = transitions.getTransition(441, 560); // Smithing Hammer 441 + Knife 560
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 185); // 0 + Skewered Rabbit 185
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 547); // 0 + Bowl of Carrot 547
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 254); // 0 + Bowl of Rabbit 254
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(0, 1247); // 0 + Bowl with Corn Kernels
		trans.aiShouldIgnore = true;

		var trans = transitions.getTransition(502, 32); // Shovel 502 + Big Hard Rock 32
		trans.alternativeTransitionOutcome.push(33); // Stone
		trans.aiShouldIgnore = true;

		// var trans = transitions.getTransition(441, 560, true); // Smithing Hammer 441 + Knife 560
		// trans.aiShouldIgnore = true;

		// currently creates a loop since ai keeps adding and removing thread
		var trans = transitions.getTransition(0, 2090); // 0 + Bound Deck of Cards 2090
		trans.aiShouldIgnore = true;

		// Oiled File Blank with Chisel 465
		var trans = transitions.getTransition(0, 465); // 0 + Bound Deck of Cards 465
		trans.aiShouldIgnore = true;

		// Mallet 467 + Fence Gate 1851 ==> Fence with Dislodged Shaft 1852
		var trans = transitions.getTransition(467, 1851);
		trans.aiShouldIgnore = true;

		// Bowl of Dry Beans 1176
		var trans = transitions.getTransition(0, 1176);
		trans.aiShouldIgnore = true;

		// Bowl of Dry Beans 1176
		var trans = transitions.getTransition(0, 1176, false, true);
		trans.aiShouldIgnore = true;

		// Dry Bean Plants 1172
		var trans = transitions.getTransition(0, 1172);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(0, 1172, false, true);
		trans.aiShouldIgnore = true;

		// Dry Bean Pod 1160 + Clay Bowl 235
		var trans = transitions.getTransition(1160, 235);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(1160, 235, false, true);
		trans.aiShouldIgnore = true;

		// Bowl of Dough 252 + Table
		var trans = transitions.getTransition(252, 3371);
		trans.switchNumberOfUses = true;
		trans.aiShouldIgnore = true;

		// Clay Bowl 235 + Table with Wheat Dough 4086
		var trans = transitions.getTransition(235, 4086);
		trans.switchNumberOfUses = true;
		trans.aiShouldIgnore = true;

		// Bowl of Soil 1137 + Fertile Soil Pile 1101
		// var trans = transitions.getTransition(1137, 1101);
		// trans.aiShouldIgnore = true;

		// Bowl of Soil 1137 + Fertile Soil 1138
		// var trans = transitions.getTransition(1137, 1138);
		// trans.aiShouldIgnore = true;

		// 0 + Bowl of Raw Pork 1354
		var trans = transitions.getTransition(0, 1354);
		trans.aiShouldIgnore = true;

		// Shredded Cabbage 1222 + Straw 227 --> Compost
		var trans = transitions.getTransition(1222, 227);
		trans.aiShouldIgnore = true;

		// Shovel 502 + Barrel Cactus 761
		var trans = transitions.getTransition(502, 761);
		trans.aiShouldIgnore = true;

		// Mallet 467 + Fence 550
		var trans = transitions.getTransition(467, 550);
		trans.aiShouldIgnore = true;

		// Mallet 467 + Fence 549 +verticalFence
		var trans = transitions.getTransition(467, 549);
		trans.aiShouldIgnore = true;

		// Steel Adze 462 + Wall Slot Shelf 3242
		var trans = transitions.getTransition(462, 3242);
		trans.aiShouldIgnore = true;

		// TODO fix reverse use transitions if full
		// Butter Knife 1467 + Bowl of Butter 1465 // TODO also lastuse
		// var trans = transitions.getTransition(1467, 1465);
		// trans.aiShouldIgnore = true;

		// Knife 560 // Bowl of Butter 1465
		var trans = transitions.getTransition(560, 1465);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(560, 1465, false, true);
		trans.aiShouldIgnore = true;

		// Mango Leaf 1878 + Domestic Cow // Since dead cow gives now meat Ai does all it can to kill them
		// var trans = transitions.getTransition(1878, 1458);
		// trans.aiShouldIgnore = true;

		// 0 + Potato in Water 1152
		var trans = transitions.getTransition(0, 1152);
		trans.aiShouldIgnore = true;

		// 0 + Bowl of Mutton 4056
		var trans = transitions.getTransition(0, 4056);
		trans.aiShouldIgnore = true;

		// 0 + Bowl of Wheat 245
		var trans = transitions.getTransition(0, 245);
		trans.aiShouldIgnore = true;

		// 0 + Plow Kit 4379 // creats cyle to get second steel blade for crafting sheers
		var trans = transitions.getTransition(0, 4379);
		trans.aiShouldIgnore = true;

		// Hungry Domestic Calf 1462 --> otherwise AI will wait untill calf dies to get Mutton
		var trans = transitions.getTransition(-1, 1462);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(-1, 1462, false, true);
		trans.aiShouldIgnore = true;

		// Wooden Tongs 239 + Hot Forged Steel Crucible 321
		var trans = transitions.getTransition(239, 321);
		trans.aiShouldIgnore = true;

		// TODO not sure if only limiting last use works
		// TODO Canada Goose Pond with Egg
		// Clay Bowl + Canada Goose Pond 141
		var trans = transitions.getTransition(235, 141, false, true);
		trans.aiShouldIgnore = true;

		// Empty Water Pouch 209 + Canada Goose Pond 141
		var trans = transitions.getTransition(209, 141, false, true);
		trans.aiShouldIgnore = true;

		// Clay Bowl + Canada Goose Pond - Swimming 142
		var trans = transitions.getTransition(235, 142, false, true);
		trans.aiShouldIgnore = true;

		// Empty Water Pouch 209 + Canada Goose Pond 142
		var trans = transitions.getTransition(209, 142, false, true);
		trans.aiShouldIgnore = true;

		// Marked Grave 1012
		var trans = transitions.getTransition(0, 1012);
		trans.aiShouldIgnore = true;

		// Buried Grave with Dug Stone 1849
		var trans = transitions.getTransition(0, 1849);
		trans.aiShouldIgnore = true;

		// Arrow Quiver 3948
		var trans = transitions.getTransition(0, 3948);
		trans.aiShouldIgnore = true;

		// Bowl of Popcorn
		var trans = transitions.getTransition(0, 1121, false, true);
		trans.aiShouldIgnore = true;
		var trans = transitions.getTransition(0, 1121, false, false);
		trans.aiShouldIgnore = true;

		// Partial Pulley Mechanism 2264
		var trans = transitions.getTransition(0, 2264);
		trans.aiShouldIgnore = true;

		// Multipurpose Newcomen Engine 2243
		var trans = transitions.getTransition(0, 2243);
		trans.aiShouldIgnore = true;

		// Wooden Sledge 471 + Wooden Box 434 // Ai seems to use this tranistion for making kindling but why?
		var trans = transitions.getTransition(471, 434);
		trans.aiShouldIgnore = true;

		// Bowl with Corn Kernels 1247 + Bucket of Corn 4110 (untill circular crafting is fixed)
		// var trans = transitions.getTransition(1247, 4110);
		// trans.aiShouldIgnore = true;

		// Bowl with Corn Kernels 1247 + Empty Bucket 659 (untill circular crafting is fixed)
		// var trans = transitions.getTransition(1247, 659);
		// trans.aiShouldIgnore = true;

		// TODO Bowl with Corn Kernels // forbid cirtular crafting --> tries to get empty bowl by putting Kernels in

		// Clay Bowl 235 // Shallow Well 662 // Bowl of Water 382
		// var trans = transitions.getTransition(235, 662);
		// trace('Bowl of Water: ' + trans.getDesciption());
		// trans.aiShouldIgnore = true;

		// lime
		// var trans = transitions.getTransition(677, 661); // Bowl of Plaster 677  + Stone Pile 661
		// trans.aiShouldIgnore = true;

		// Steel Adze 462
		var transBy = TransitionImporter.GetTransitionByActor(462);
		for (trans in transBy) {
			// Two Shafts
			if (trans.targetID == 557) continue;
			// Butt Log 345
			if (trans.targetID == 156) continue;
			// Mallet 467
			if (trans.targetID == 154) continue;
			// Broken Hand Cart 846
			if (trans.targetID == 846) continue;

			trans.aiShouldIgnore = true;
		}

		// Bowl of Plaster 677
		var transByTarget = TransitionImporter.GetTransitionByActor(677);
		for (trans in transByTarget) {
			// Adobe Wall H 155
			if (trans.targetID == 155) continue;
			// Adobe Wall V 156
			if (trans.targetID == 156) continue;
			// Adobe Wall C 154
			if (trans.targetID == 154) continue;

			trans.aiShouldIgnore = true;
		}

		// Steel Mining Pick 684
		var transByTarget = TransitionImporter.GetTransitionByActor(684);
		for (trans in transByTarget) {
			// Gold Vein 680
			if (trans.targetID == 680) continue;
			// Stripped Iron Vein 3944
			if (trans.targetID == 3944) continue;
			// Shallow Iron Pit 3957
			if (trans.targetID == 3957) continue;
			// Cut Stones 881
			if (trans.targetID == 881) continue;
			// Iron Mine
			if (trans.targetID == 944) continue;

			// trace('Steel Mining Pick ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// Scrap Bowl 3076 --> dont allow to scraft crafted metal
		var transByTarget = TransitionImporter.GetTransitionByTarget(3076);
		for (trans in transByTarget) {
			// Leaf 62
			if (trans.actorID == 62) continue;
			// Clump of Scrap Steel 930
			if (trans.actorID == 930) continue;
			// trace('Scrap Bowl: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// ignore time transitions that make 732 Ashes with Bowl since Ai uses that to get empty bowl
		// Time + Simmering Water 730 ==> Ashes with Bowl 732
		// FIX: better forbid Simmering Water 730 since Ai somehow makes still Simmering Water
		var transByTarget = TransitionImporter.GetTransitionByNewTarget(730);
		for (trans in transByTarget) {
			if (trans.actorID > -1) continue;
			// trace('Ashes with Bowl: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// TODO forbid for all
		// Deep Tilled Row 213 --> Forbid to use skewer
		var transByTarget = TransitionImporter.GetTransitionByNewTarget(213);
		for (trans in transByTarget) {
			// Skewer 139 // Weak Skewer 852
			if (trans.actorID != 139 && trans.actorID != 852) continue;

			// trace('Deep Tilled Row: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// Stakes 107 By new target
		var transByTarget = TransitionImporter.GetTransitionByNewTarget(107);
		for (trans in transByTarget) {
			// Short Shaft 69
			if (trans.targetID == 62) continue;
			// Pile of Stakes 4066
			if (trans.targetID == 4066) continue;
			// Stakes with Rope 3883
			// if(trans.targetID == 3883) continue;

			// trace('Stakes 107: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// Stakes 107 By new actor
		var transByTarget = TransitionImporter.GetTransitionByNewActor(107);
		for (trans in transByTarget) {
			// Pile of Stakes 4066
			if (trans.targetID == 4066) continue;

			// trace('Stakes 107: ' + trans.getDesciption());
			trans.aiShouldIgnore = true;
		}

		// Clay Plate 236 --> dont puy Raw Pie Crust 264 back in bowl to get plate
		var transByTarget = TransitionImporter.GetTransitionByNewActor(236);
		for (trans in transByTarget) {
			// Raw Pie Crust 264
			if (trans.actorID != 264) continue;
			trans.aiShouldIgnore = true;
			// trace('Raw Pie Crust: ignore: ${trans.aiShouldIgnore} ' + trans.getDescription(false));
		}

		// forbid burning stuff
		var transByTarget = TransitionImporter.GetTransitionByNewActor(520); // Burnt Goose
		for (trans in transByTarget) {
			// trace('Burnt Goose: ignore: ${trans.aiShouldIgnore} ' + trans.getDescription(false));
			trans.aiShouldIgnore = true;
		}

		// Bowl of Water 382 +  Bowl of Tomato Seeds = Clay Bowl 235 + Bowl of Water 382
		var trans = new TransitionData(382, 2828, 235, 382);
		// trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Full Water Pouch+ 210 + Bowl of Tomato Seeds 2828 = Empty Water Pouch 209 + Bowl of Water 382
		var trans = new TransitionData(210, 2828, 209, 382);
		// trans.aiShouldIgnore = true;
		transitions.addTransition("PatchTransitions: ", trans);

		// Time + Bear Cave - awake 648 --> Hungry Grizzly Bear 631
		var trans = TransitionImporter.GetTransition(-1, 648);
		trans.aiShouldIgnore = true;
		trans.newTargetID = 631;
		transitions.addTransition("PatchTransitions: ", trans);

		/*var transByTarget = TransitionImporter.GetTransitionByTarget(2828); // Bowl of Tomato Seeds
			for (trans in transByTarget) {
				trace('Bowl of Tomato Seeds: ' + trans.getDescription(true));
				trans.aiShouldIgnore = true;
		}*/

		// Shallow Well 662 // Bowl of Water 382
		/*var transByTarget = TransitionImporter.GetTransitionByNewActor(382);
			for(trans in transByTarget){
				if(trans.targetID != 662) continue;
				trace('Bowl of Water: ' + trans.getDescription());
		}*/

		// Bowl of Soil 1137
		/*var transByTarget = TransitionImporter.GetTransitionByNewActor(1137);
			for (trans in transByTarget) {
				trace('Bowl of Soil: ignore: ${trans.aiShouldIgnore} ' + trans.getDescription());
		}*/

		// Broken Steel Tool 858 --> no transitions found
		/*var transByTarget = TransitionImporter.GetTransitionByNewTarget(858);
			for(trans in transByTarget){
				// Skewer 139 // Weak Skewer 852
				//if(trans.actorID != 139 && trans.actorID != 852) continue;
				
				trace('Broken Steel Tool: ' + trans.getDesciption());
				trans.aiShouldIgnore = true; 
		}*/

		// var trans = transitions.getTransition(235, -1); // 235 Clay Bowl
		// trace('DEBUG: ${trans.getDesciption()}');

		// var trans = transitions.getTransition(253, -1); // Bowl of Gooseberries
		// trace('DEBUG!!: ${trans.getDesciption()}');

		// for debug random outcome transitions
		/*var trans = transitions.getTransition(-1, 1195); // TIME + Blooming Squash Plant 
			trans.autoDecaySeconds = 2;
			transitions.addTransition("PatchTransitions: ", trans);
		 */

		var trans = TransitionImporter.GetTransition(235, 662); // Clay Bowl + Shallow Well
		trace('DEBUG: ${trans.getDescription()}');
		if (trans.newActorID != 382) { // Bowl of Water 382
			throw new Exception('New actor should be: Bowl of Water 382');
		}

		// Steel Mining Pick 684 +  Gold Vein 680
		var trans = TransitionImporter.GetTransition(684, 680);
		trans.coinCost = 20;
		trans.hungryWorkCost = 10;
		trans.alternativeTransitionOutcome.push(0);
		trans.alternativeTransitionOutcome.push(33);
		trans.alternativeTransitionOutcome.push(33); // Stone 33
		trans.alternativeTransitionOutcome.push(291); // Flat Rock 291
		trans.alternativeTransitionOutcome.push(681); // Gold Flakes 681

		// Property Gate 296
		var transitions = TransitionImporter.GetTransitionByTarget(2962);
		for (trans in transitions) {
			trans.aiShouldIgnore = true;
			if (trans.actorID == 0) continue;

			trans.hungryWorkCost = 0.1; // allow only for owner

			// Skewer 139
			if (trans.actorID == 139) continue; // allow for owner

			// Weak Skewer 852
			if (trans.actorID == 852) continue; // allow for owner

			trans.hungryWorkCost = 5;
			trans.alternativeTransitionOutcome.push(0);

			// trace('Property Gate: ' + trans.getDescription(false));
		}

		/*// Bowl of Water 382
			var count = 0;
			var transByTarget = TransitionImporter.GetTransitionByNewActor(382);
			for(trans in transByTarget){
				//trace('Bowl of Water: ' + trans.getDesciption());
				count++;
			}

			trace('Bowl of Water tranistions2: $count');

			throw new Exception('stop'); */

		// var lastUseTransition = TransitionImporter.GetTransition(252, 236, true, false);
		// trace('Debug PAYETON4171: ${lastUseTransition.getDesciption()}');

		// var objData = ObjectData.getObjectData(885); // 885 Stone Wall
		// trace('${objData.name} getInsulation: ${objData.getInsulation()} rvalue: ${objData.rValue}');

		// var trans = TransitionImporter.GetTransition(544, -1, true, false);
		// trace('DEBUG: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(3425, -1, false, false); // Domestic Cow on Rope
		// trace('ON DEATH: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(0, 3948); // Arrow Quiver
		// trace('DEBUG: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(560, 418); // Knife + Wolf
		// trace('DEBUG: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(152, 0); // Bow and Arrow + 0
		// trace('DEBUG: ${trans.getDesciption()}');

		// var objData = ObjectData.getObjectData(887); // Stone Wall
		// trace('${objData.name} isPermanent ${objData.isPermanent()}');

		// var trans = TransitionImporter.GetTransition(660, 673); // Full Bucket of Water Bow and Arrow + Empty Cistern
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(382, 1790); // Bowl of Water + Dry Maple Sapling Cutting
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(382, 396); // Bowl of Water + Dry Planted Carrots
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(382, 2723); // Bowl of Water + Dry Juniper Sapling
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(382, 1042); // Bowl of Water + Dry Planted Rose Seed (RED)
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(-1, 1873); // TIME + Wet Mango Sapling
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// <-1> + <3132> = <0> + <3131> / TIME + Running Diesel Mining Pick#with Iron  -->  EMPTY + Diesel Mining Pick with Iron
		// var trans = TransitionImporter.GetTransition(-1, 3132);
		// trace('DEBUG!!!: ${trans.getDesciption(false)}');

		// var trans = TransitionImporter.GetTransition(283, -1); // Wooden Tongs with Fired Bowl
		// trace('DEBUG!!!: ${trans.getDesciption()}');

		// var trans = TransitionImporter.GetTransition(0, 31); // EMPTY + GOOSEBERRY 31
		// trace('DEBUG!!!: ${trans.getDescription()}');

		/*var transByTarget = TransitionImporter.GetTransitionByTarget(3371);
			for (trans in transByTarget) {
				// Skewer 139 // Weak Skewer 852
				// if(trans.actorID != 139 && trans.actorID != 852) continue;

				trace('DEBUG!!: ' + trans.getDescription());
				// trans.aiShouldIgnore = true;
		}*/

		// var trans = TransitionImporter.GetTransition(33, 3371); // Stone 33 + Table 3371
		// trace('DEBUG!!!: ${trans.getDescription()}');

		// Watered Wild Gooseberry Bush 3946
		/*var transByTarget = TransitionImporter.GetTransitionByNewTarget(3946);
			for (trans in transByTarget) {
				// Skewer 139 // Weak Skewer 852
				// if(trans.actorID != 139 && trans.actorID != 852) continue;

				trace('DEBUG!!: ' + trans.getDescription());
				// trans.aiShouldIgnore = true;
		}*/

		// Shallow Well 662
		var transByTarget = TransitionImporter.GetTransitionByActor(662);
		for (trans in transByTarget) {
			// trace('DEBUG!!: ' + trans.getDescription(true));
			trans.aiShouldIgnore = true; // ignore tapoutTrigger for Shallow Well
		}

		// Dry Shallow Well 664
		var transByTarget = TransitionImporter.GetTransitionByActor(664);
		for (trans in transByTarget) {
			// trace('DEBUG!!: ' + trans.getDescription(true));
			trans.aiShouldIgnore = true; // ignore tapoutTrigger for Shallow Well
		}

		//  Deep Well 663
		var transByTarget = TransitionImporter.GetTransitionByActor(663);
		for (trans in transByTarget) {
			// Skewer 139 // Weak Skewer 852
			// if(trans.actorID != 139 && trans.actorID != 852) continue;

			// trace('DEBUG!!: Deep Well ' + trans.getDescription(true));
			// ignore tapoutTrigger for Shallow Well
			trans.aiShouldIgnore = true;
		}

		// Full Deep Well 1097
		var transByTarget = TransitionImporter.GetTransitionByActor(1097);
		for (trans in transByTarget) {
			// Skewer 139 // Weak Skewer 852
			// if(trans.actorID != 139 && trans.actorID != 852) continue;

			// trace('DEBUG!!: Full Deep Wel ' + trans.getDescription(true));
			// ignore tapoutTrigger for Shallow Well
			trans.aiShouldIgnore = true;
		}

		// Deep Well - was empty 1861
		var transByTarget = TransitionImporter.GetTransitionByActor(1861);
		for (trans in transByTarget) {
			// Skewer 139 // Weak Skewer 852
			// if(trans.actorID != 139 && trans.actorID != 852) continue;

			// trace('DEBUG!!: Deep Well - was empty ' + trans.getDescription(true));
			trans.aiShouldIgnore = true;
		}

		InitWaterSourceIds();
		InitWateringTargets();

		// limit certain Transitions if there is too much of an item
		LimitTransitionsIfTooMuchOfObject(); // like making more Bowls

		LimitTransitionsIfTooFewOfObject(); // like Destroying Bows
	}

	public static var WaterSourceIds:Array<Int> = [];
	public static var BucketWaterSourceIds:Array<Int> = [];

	private static function InitWaterSourceIds() {
		// Bowl of Water 382 // Clay Bowl 235
		var transByTarget = TransitionImporter.GetTransitionByNewActor(382);
		for (trans in transByTarget) {
			if (trans.targetID < 1) continue; // like TIME
			if (trans.actorID != 235) continue;
			if (WaterSourceIds.contains(trans.targetID)) continue;
			// trace('InitWaterSourceIds: ' + trans.getDescription(false));
			var name = ObjectData.getObjectData(trans.targetID).name;

			// trace('InitWaterSourceIds: ' + name);

			WaterSourceIds.push(trans.targetID);
		}

		// Full Bucket of Water 660
		var transByTarget = TransitionImporter.GetTransitionByNewActor(660);
		for (trans in transByTarget) {
			if (trans.targetID < 1) continue; // like TIME
			if (trans.actorID != 659) continue; // Empty Bucket 659
			if (trans.actorID != 659) continue;

			if (BucketWaterSourceIds.contains(trans.targetID)) continue;
			// trace('InitWaterSourceIds: ' + trans.getDescription(false));
			var name = ObjectData.getObjectData(trans.targetID).name;

			// trace('InitWaterSourceIds: ' + name);

			BucketWaterSourceIds.push(trans.targetID);
		}
	}

	public static var WateringTargetsIds:Array<Int> = [];
	public static var WateringTargetsIdsWithoutCarrots:Array<Int> = [];
	// Full Water Pouch 210 // Full Water Pouch Pile 4094 // Adobe 127 // Bowl of Water 382
	// Full Bucket of Water 660 // Partial Bucket of Water 1099 // Watered Wild Gooseberry Bush
	public static var IgnoreToWaterNewTargets:Array<Int> = [210, 4094, 127, 382, 660, 1099, 3946];

	private static function InitWateringTargets() {
		// Full Water Pouch 210
		var transByTarget = TransitionImporter.GetTransitionByActor(210);
		for (trans in transByTarget) {
			if (trans.targetID < 1) continue; // like TIME
			if (IgnoreToWaterNewTargets.contains(trans.newTargetID)) continue;
			if (WateringTargetsIds.contains(trans.targetID)) continue;
			// trace('InitWateringTargets: ' + trans.getDescription(false));
			var name = ObjectData.getObjectData(trans.targetID).name;

			// trace('InitWateringTargets: ' + name);

			WateringTargetsIds.push(trans.targetID);

			// Dry Planted Carrots 396
			if (trans.newTargetID != 396) WateringTargetsIdsWithoutCarrots.push(trans.targetID);
		}
	}

	// TODO currently objects could be counted twice in crafting if current pos and home is searched
	private static function LimitTransitionsIfTooMuchOfObject() {
		LimitTransitionIfMaxReached(559, 69, 500); // Steel Blade 559 + Short Shaft 69 // Knife 560
		LimitTransitionIfMaxReached(69, 559, 500); // Short Shaft 69 + Steel Blade 559  // Knife 560

		LimitTransitionIfMaxReached(443, 69, 441); // Smithing Hammer Head 443 + Short Shaft 69 // Smithing Hammer 441
		LimitTransitionIfMaxReached(69, 443, 441); // Short Shaft 69 + Smithing Hammer Head 443  // Smithing Hammer 441

		LimitTransitionIfMaxReached(124, 124, 292, 5); // Reed Bundle 124 + Reed Bundle 124 // Basket 292
		LimitTransitionIfMaxReached(58, 124, 292, 5); // Thread 58 + Reed Bundle 124 // Basket 292
		LimitTransitionIfMaxReached(58, 123, 292, 5); // Thread 58 + Harvested Tule 123 // Basket 292
		LimitTransitionIfMaxReached(139, 227, 292, 5); // Skewer 139 + Straw 227 // Basket 292
		LimitTransitionIfMaxReached(852, 227, 292, 5); // Weak Skewer 852 + Straw 227 // Basket 292

		LimitObject(1121, 1122); // Limit Popcorn 1121 // Popping Corn 1122

		LimitObject(502, 500); // Shovel 502 // Steel Shovel Head 500

		LimitObject(570, 570, 5); // Cooked Mutton 570

		LimitObject(235, 283, 10); // Limit Clay Bowl 235 // Wooden Tongs with Fired Bowl 283

		LimitObject(236, 241, 5); // Limit Clay Plate 236 // Fired Plate in Wooden Tongs 241

		LimitObject(236, 241, 5); // Limit Clay Plate 236 // Fired Plate in Wooden Tongs 241

		LimitObject(183, 180, 20); // Rabbit Fur 183 // Dead Rabbit 180

		LimitObject(132, 132, 20); // Yew Branch 132

		LimitObject(64, 64, 120); // Straight Branch

		LimitObject(213, 1136, 20); // Deep Tilled Row 213 // Shallow Tilled Row 1136

		LimitObjectByNewTarget(2835, 2829, 9); // Fruiting Tomato Plant // Dry Planted Tomato Seed
		// LimitObjectByNewTarget(623, 623, 3); // Dry Compost Pile 623
		// LimitObjectByNewTarget(624, 623, 3); // Composted Soil 624 // Dry Compost Pile 623
		LimitObjectByNewTarget(625, 623, 3); // Wet Compost Pile 625 // Dry Compost Pile 623
		LimitObjectByNewTarget(625, 625, 3); // Wet Compost Pile 625

		// LimitObjectByNewTarget(402, 399, 10); // Limit Carrot 402 // Wet Planted Carrots 399
		// LimitObjectByNewTarget(402, 396, 10); // Limit Carrot 402 // Dry Planted Carrots 396

		// Bowl of Tomato Seeds 2828 // Bowl of Tomato Seed Pulp 2825
		LimitObjectByNewTarget(2828, 2825, 1);

		// Ripe Cucumber Plant 4232 // Dry Planted Cucumber Seeds 4225
		LimitObjectByNewTarget(4232, 4225, 5);

		LimitObjectByNewTarget(391, 216, 10); // // Domestic Gooseberry Bush 391 // Dry Planted Gooseberry Seed 216

		LimitObjectByNewTarget(242, 228, 30); // Dry Planted Wheat 228 // Ripe Wheat 242

		// Raw Stew Pot 1246
		LimitObjectByNewTarget(1246, 1246, 2); // Dry Planted Wheat 228 // Ripe Wheat 242

		// LimitObjectByTarget(297, 226, 1); // Threshed Wheat 297 // Threshed Wheat (with straw) 226
		// LimitObjectByTarget(4070, 242, 2); // Pile of Threshed Wheat 4070 // Ripe Wheat 242

		// LimitObjectByTarget(1115, 1112, 5); // Dried Ear of Corn 1115 // Corn Plant 1112

		// Stone 33 + Bowl of Wheat 245 / Raw Pie Crust 264 / Bowl of Dough 252
		LimitTransitionIfMaxReached(33, 245, 264, 3);

		// 1466 Bowl of Leavened Dough // 236 Clay Plate // Sliced Bread 1471
		LimitTransitionIfMaxReached(1466, 236, 1471, 2);

		// Clay Bow 235 + Partial Bucket of Water 1099 / Bowl of Water 382
		LimitTransitionIfMaxReached(235, 1099, 382, 3);

		//  Clay Plate 236 // Carved Turkey on Plate 2187 // Turkey Slice on Plate 2190
		LimitTransitionIfMaxReached(236, 2187, 2190, 1);

		for (i in 0...AiBase.rawPies.length)
			LimitObjectByNewTarget(AiBase.pies[i], AiBase.rawPies[i], 2);
	}

	private static function LimitTransitionIfMaxReached(actorId:Int, targetId:Int, id:Int, max:Int = 1) {
		var objData = ObjectData.getObjectData(id);
		objData.aiCraftMax = max;

		var trans = TransitionImporter.GetTransition(actorId, targetId);
		trans.ignoreIfMaxIsReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, true);
		if (trans != null) trans.ignoreIfMaxIsReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, false, true);
		if (trans != null) trans.ignoreIfMaxIsReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, true, true);
		if (trans != null) trans.ignoreIfMaxIsReachedObjectId = id;
	}

	private static function LimitObject(id:Int, limitNewActorId:Int, max:Int = 1) {
		var objData = ObjectData.getObjectData(id);
		objData.aiCraftMax = max;

		var transitions = TransitionImporter.GetTransitionByNewActor(limitNewActorId);
		for (trans in transitions) {
			if (trans.targetID == objData.getPileObjId()) continue; // Allow to take from piles
			trans.ignoreIfMaxIsReachedObjectId = id;
		}
	}

	private static function LimitObjectByTarget(id:Int, limiTargetId:Int, max:Int = 1) {
		var objData = ObjectData.getObjectData(id);
		objData.aiCraftMax = max;

		var transitions = TransitionImporter.GetTransitionByTarget(limiTargetId);
		for (trans in transitions) {
			trans.ignoreIfMaxIsReachedObjectId = id;
		}
	}

	private static function LimitObjectByNewTarget(id:Int, limitNewtargetId:Int, max:Int = 1) {
		var objData = ObjectData.getObjectData(id);
		objData.aiCraftMax = max;

		var transitions = TransitionImporter.GetTransitionByNewTarget(limitNewtargetId);
		for (trans in transitions) {
			trans.ignoreIfMaxIsReachedObjectId = id;
		}
	}

	private static function LimitTransitionsIfTooFewOfObject() {
		// Steel Axe + Wooden Tongs
		LimitTransitionIfMinNotReached(334, 239, 239, 3);
		// Stone Hatchet + Wooden Tongs
		LimitTransitionIfMinNotReached(71, 239, 239, 3);

		// Flint Chip 135 + Yew Bow 151
		LimitTransitionIfMinNotReached(135, 151, 151, 3); // Allow to destroy Bows if more than 3
		// Knife 560 + Yew Bow 151
		LimitTransitionIfMinNotReached(560, 151, 151, 3); // Allow to destroy Bows if more than 3

		// Bow and Arrow 152 + Mouflon 531
		LimitTransitionIfMinNotReached(152, 531, 531, 3); // Allow to kill Mouflon if needed are reached
		// Knife 560 +  Domestic Sheep 575
		LimitTransitionIfMinNotReached(560, 575, 575, 3); // Allow to kill Sheep if enough
		// Knife 560 +  Shorn Domestic Sheep 576
		LimitTransitionIfMinNotReached(560, 576, 576, 4); // Allow to kill Sheep if enough

		// Knife 560 + Domestic Cow 1458
		LimitTransitionIfMinNotReached(560, 1458, 1458, 4); // Allow to kill Cow if enough
		// War Sword 3047 + Domestic Cow 1458
		LimitTransitionIfMinNotReached(3047, 1458, 1458, 4); // Allow to kill Cow if enough
		// Mango Leaf 1878 + Domestic Cow 1458
		LimitTransitionIfMinNotReached(1878, 1458, 1458, 2); // Allow to kill Cow if enough

		// TODO: taking water is still possible / limit lastuse
		// Bow and Arrow 152 // Canada Goose Pond 141
		LimitTransitionIfMinNotReached(152, 141, 141, 2);

		// Crucible with Charcoal 318 // Charcoal 302
		LimitTransitionIfMinNotReached(318, 302, 318, 8);
		// Crucible with Charcoal 318 // Small Charcoal Pile
		LimitTransitionIfMinNotReached(318, 301, 318, 8);

		// Steel Axe 334 // Maple Tree - Branch 63
		LimitTransitionIfMinNotReached(334, 63, 63, 20);

		// Steel Axe 334 // Maple Tree 48
		LimitTransitionIfMinNotReached(334, 48, 48, 10);

		// Steel Axe 334 // Yew Tree - Branch 153
		LimitTransitionIfMinNotReached(334, 153, 153, 10);

		// Steel Axe 334 // Yew Tree - 153
		LimitTransitionIfMinNotReached(334, 406, 406, 5);

		// Bowl of Water 382 +  Bowl of Tomato Seeds 2828 = Clay Bowl 235 + Bowl of Water 382
		LimitTransitionIfMinNotReached(382, 2828, 2828, 3);

		// Full Water Pouch+ 210 + Bowl of Tomato Seeds 2828 = Empty Water Pouch 209 + Bowl of Water 382
		LimitTransitionIfMinNotReached(210, 2828, 2828, 3);

		// Flowering Milkweed 51 to get seeds
		LimitTransitionIfMinNotReached(0, 51, 51, 2);

		// Milkweed 50 to get seeds
		LimitTransitionIfMinNotReached(0, 50, 50, 2);

		// Stone Hoe  + Clay Plate
		LimitTransitionIfMinNotReached(850, 236, 236, 10);

		// Domestic Goose- held 1267 + Stump 338 // Domestic Goose 1256
		LimitTransitionIfMinNotReached(1267, 338, 1256, 3);
	}

	private static function LimitTransitionIfMinNotReached(actorId:Int, targetId:Int, id:Int, min:Int = 3) {
		var objData = ObjectData.getObjectData(id);
		objData.aiCraftMin = min;

		var trans = TransitionImporter.GetTransition(actorId, targetId);
		trans.igmoreIfMinIsNotReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, true, false);
		if (trans != null) trans.igmoreIfMinIsNotReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, false, true);
		if (trans != null) trans.igmoreIfMinIsNotReachedObjectId = id;
		var trans = TransitionImporter.GetTransition(actorId, targetId, true, true);
		if (trans != null) trans.igmoreIfMinIsNotReachedObjectId = id;
	}

	private static function SetClothingPrestige(clothing:ObjectData) {
		if (clothing.clothing.length > 1) clothing.clothing = StringTools.trim(clothing.clothing);
		if (clothing.clothing == 'n') return;

		// trace('Clothing: ${clothing.name} index: ${clothing.clothing} prestige: ${clothing.prestigeFactor}');

		if (clothing.description.startsWith('Red ')) {
			clothing.prestigeFactor += 0.5;
		}
		if (clothing.description.startsWith('Indigo ')) {
			clothing.prestigeFactor += 0.5;
		}
		if (clothing.description.startsWith('Green ')) {
			clothing.prestigeFactor += 0.5;
		}
		if (clothing.description.startsWith('Yellow ')) {
			clothing.prestigeFactor += 0.5;
		}
		if (clothing.description.startsWith('Black ')) {
			clothing.prestigeFactor += 0.5;
		}

		if (clothing.description.contains(' Rose')) {
			clothing.prestigeFactor += 0.5;
		}
		if (clothing.description.contains(' Feather')) {
			clothing.prestigeFactor += 0.2;
		}

		if (clothing.description.contains('Rag ')) {
			clothing.prestigeFactor /= 2;
		}
		if (clothing.description.contains('Old ')) {
			clothing.prestigeFactor /= 2;
		}

		if (clothing.description.contains('Cloak')) {
			clothing.prestigeFactor *= 2;
		}
		if (clothing.description.contains('Long Dress')) {
			clothing.prestigeFactor *= 2;
		}
	}
}
/**Actor Category: 1641 @ Deadly Wolf
	Trans: 1640 + 0 = 427 + 1363 
	Semi-tame Wolf# just fed 
	+ Empty  
	--> Attacking Wolf
	+ Bite Wound

	Actor Category: 1641 @ Deadly Wolf
	Trans: 1630 + 0 = 427 + 1363 
	Semi-tame Wolf
	+ Empty  
	--> Attacking Wolf
	+ Bite Wound
**/
/**public static function add(a:Int, b:Int, ?pos:PosInfos) {
	trace( 'Called from ${pos.className}');
	trace( 'Called from ${pos.methodName}');
	trace( 'Called from ${pos.fileName}');
	trace( 'Called from ${pos.lineNumber}');
	return a+b;
	}

	add( 1, 1 ); // "pos" will automatically be filled in by compile**/
