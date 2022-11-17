package openlife.settings;

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
	public static var DebugAiGoto:Bool = false;
	public static var DebugAiCrafting:Bool = false;
	public static var DebugAiCraftingObject:Int = 999999; // 57;
	public static var AutoFollowAi:Bool = false;
	public static var AutoFollowPlayer:Bool = false;

	// Save / Load
	public static var saveToDisk = true;
	public static var SavePlayers = true;
	public static var LoadPlayers = true;

	// Mutex
	public static var UseOneGlobalMutex = false; // if you want to try out if there a problems with mutexes / different threads
	public static var UseOneSingleMutex = true; // might get stuck if set true
	public static var UseBlockingSockets = false;

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

	// score
	public static var BirthPrestigeFactor:Float = 0.4; // TODO set 0.2 if fathers are implemented // on birth your starting prestige is factor X * total prestige
	public static var AncestorPrestigeFactor:Float = 0.2; // if one dies the ancestors get factor X prestige of the dead
	public static var ScoreFactor:Float = 0.2; // new score influences total score with factor X.
	public static var OldGraveDecayMali:Float = 20; // prestige mali if bones decay without beeing proper burried
	public static var CursedGraveMali:Float = 2; // prestige mali if bones decay without beeing proper burried

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
	public static var MinPrestiegeFromCoinDecayPerYear:Float = 5;

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

	// PlayerInstance
	public static var MaxPlayersBeforeStartingAsChild = 0; // -1
	public static var MaxPlayersBeforeActivatingGraveCurse = 2; 
	public static var StartingFamilyName = "SNOW";
	public static var StartingName = "SPOON";
	public static var AgeingSecondsPerYear = 60; // 60
	public static var ReduceAgeNeededToPickupObjects = 3; // reduces the needed age that an item can be picked up. But still it cant be used if age is too low
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
	public static var EveOrAdamBirthChance = 0.05; // since each eve gets an adam the true chance is x2
	public static var startingGx = 235; // 235; //270; // 360;
	public static var startingGy = 150; // 200;//- 400; // server map is saved y inverse
	public static var EveDamageFactor:Float = 1; // Eve / Adam get less damage from animals but make also less damage
	public static var EveFoodUseFactor:Float = 1; // Eve / Adam life still in paradise, so they need less food

	// /DIE stuff
	public static var MaxAgeForAllowingDie:Float = 2;
	public static var PrestigeCostForDie:Float = 0;


	// food stuff / healing / exhaustion recover
	public static var FoodUsePerSecond = 0.10; // 0.2; // 5 sec per pip // normal game has around 0.143 (7 sec) with bad temperature and 0.048 (21 sec) with good
	public static var HealingPerSecond = 0.10;
	public static var WoundHealingFactor:Float = 1;
	public static var ExhaustionHealingFactor:Float = 2;
	public static var ExhaustionHealingForMaleFaktor:Float = 1.2;
	
	public static var FoodReductionPerEating:Float = 1;
	public static var FoodReductionFaktorForEatingMeh:Float = 0.2;
	public static var MinAgeToEat = 3; // MinAgeToEat and MinAgeFor putting on cloths on their own
	public static var GrownUpFoodStoreMax = 20; // defaul vanilla: 20
	public static var NewBornFoodStoreMax = 4;
	public static var OldAgeFoodStoreMax = 10;
	public static var DeathWithFoodStoreMax:Float = -0.1; // Death through starvation if food store max reaches below XX
	public static var FoodUseChildFaktor:Float = 1; // children need X times food if below GrownUpAge
	public static var YumBonus = 3; // First time eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
	public static var YumFoodRestore = 0.8; // XX pipes are restored from a random eaten food. Zero are restored if random food is the current eaten food
	public static var LovedFoodRestore:Float = 0.2; // restore also some loved food like bana for brown
	public static var YumNewCravingChance = 0.2; // XX chance that a new random craving is chosen even if there are existing ones
	public static var HealthLostWhenEatingMeh:Float = 0.5;
	public static var HealthLostWhenEatingSuperMeh:Float = 1;
	public static var MaxHasEatenForNextGeneration:Float = 2; // 2; // used in InheritEatenFoodCounts
	public static var HasEatenReductionForNextGeneration:Float = 1; // 0.2 // used in InheritEatenFoodCounts

	// Biome Specialists
	public static var LovedFoodUseChance:Float = 0.5;
	public static var BiomeAnimalHitChance:Float = 0.0; // for example if a biome animal like a wolf can hit a white in mountain

	// Yellow Fever
	public static var ExhaustionYellowFeverPerSec = 0.1;
	public static var AllowEatingOrFeedingIfIll = false; // for example if you have yellow fever some one needs to feed you if false
	public static var ResistanceAginstFeverForEatingMushrooms:Float = 0.2;

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
	public static var SendMoveEveryXTicks = -1; // default 90 // set negative to deactive. if MaxDistanceToBeConsideredAsClose it might be deactivated
	public static var MaxDistanceToBeConsideredAsClose = 2000000; // 20; // only close players are updated with PU Movement
	public static var MaxDistanceToBeConsideredAsCoseForMovement = 30; // 20; // only close players are updated with PU Movement
	public static var MaxDistanceToBeConsideredAsCloseForMapChanges = 10; // for MX
	public static var MaxDistanceToBeConsideredAsCloseForSay = 20; // if a player says something
	public static var MaxDistanceToBeConsideredAsCloseForSayAi = 20; // if a player says something

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
	public static var MaxTimeBetweenMapChunks:Float = 3;  // make sure that every X seconds at least one map chunk is send 

	// since client does not seem to use exact positions allow little bit cheating / JUMPS
	public static var LetTheClientCheatLittleBitFactor = 1.1; // when considering if the position is reached, allow the client to cheat little bit, so there is no lag
	public static var MaxMovementQuadJumpDistanceBeforeForce:Float = 5; // if quadDistance between server and client position is bigger then X the client is forced to use server position
	public static var MaxJumpsPerTenSec:Float = 10; // limit how often a client can JUMP / cheat his position
	public static var ExhaustionOnJump:Float = 0.05;	

	// hungry work
	public static var HungryWorkCost:Float = 5; // 10
	public static var HungryWorkHeat:Float = 0.002; // 0.005; // per food used
	public static var HungryWorkToolCostFactor:Float = 0;

	// first the chance for success would be - then 10% then 20% usw ... 10 hits 100%
	public static var AlternativeOutcomePercentIncreasePerHit:Float = 10; // for example used for extra wood for trees or stone form iron mining	
	// once succeeded in cutting the tree / mining the hits is ruced by 5
	public static var AlternativeOutcomeHitsDecreaseOnSucess = 5; // for example used for extra wood for trees or stone form iron mining


	public static var TeleportCost:Float = 1; // 10
	
	// for animal movement
	public static var ChanceThatAnimalsCanPassBlockingBiome:Float = 0.03;
	public static var chancePreferredBiome:Float = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
	public static var AnimalDeadlyDistanceFactor:Float = 0.5; // How close a animal must be to make a hit

	// for animal offsprings
	public static var ChanceForOffspring:Float = 0.00005; // 0.00005;// 0.0005 // For each movement there is X chance to generate an offspring.
	public static var ChanceForAnimalDying:Float = 0.00005; // 0.05 // 0.00002 // 0.00025 // For each movement there is X chance that the animal dies
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

	public static var DecayFactorInDeepWater:Float = 5;
	public static var DecayFactorInMountain:Float = 3;
	public static var DecayFactorInWalkableWater:Float = 2;
	public static var DecayFactorInJungle:Float = 2;
	public static var DecayFactorInSwamp:Float = 2;

	// Temperature
	public static var DebugTemperature = false;
	public static var TemperatureHeatObjectFactor:Float = 1.5; // impact of fire and ice stuff
	public static var DamageTemperatureFactor:Float = 0.5;
	public static var TemperatureImpactPerSec:Float = 0.03; 	
	public static var TemperatureImpactPerSecIfGood:Float = 0.06; // if outside temperature is helping to et closer to optimal 	
	public static var TemperatureInWaterFactor:Float = 1.5;
	public static var TemperatureNaturalHeatInsulation:Float = 0.5; // gives X extra natural insulation against heat 
	public static var TemperatureClothingInsulationFactor:Float = 5; // with 100% insulation 5X more time 10X with 200% insulation / only if temperature change is ositive
	public static var TemperatureImpactReduction:Float = 0.4; // reduces the impact of bad temperature if temperature is already bad 
	public static var TemperatureHeatObjFactor:Float = 1; // increase or decrase impact of a head obj like fire
	public static var TemperatureLovedBiomeFactor:Float = 1; 
	public static var TemperatureMaxLovedBiomeImpact:Float = 0.1; // max 0.1 better in loved biome (right color or borh parents right color)
	public static var TemperatureImpactBelow:Float = 0.6; // take damage and display emote if temperature is below or above X from normal
	public static var TemperatureSpeedImpact:Float = 1.0; // 0.0 // speed * X: double impact if extreme temperature
	
	public static var TemperatureShiftForBlack:Float = 0.1; // ideal temperature = 0.6
	public static var TemperatureShiftForBrown:Float = 0.05;
	public static var TemperatureShiftForWhite:Float = -0.05;
	public static var TemperatureShiftForGinger:Float = -0.1;

	// winter / summer
	public static var DebugSeason:Bool = false;
	public static var SeasonDuration:Float = 7.5; // default: 5 // Season duration like winter in years
	public static var SeasonBiomeChangeChancePerYear:Float = 2; //5 // X means it spreads X tiles per year in average in each direction
	public static var SeasonBiomeRestoreFactor:Float = 2;
	public static var AverageSeasonTemperatureImpact:Float = 0.2;
	public static var HotSeasonTemperatureFactor:Float = 0.5;
	public static var ColdSeasonTemperatureFactor:Float = 0.75; // 0.5

	public static var WinterWildFoodDecayChance:Float = 1.5; // 1.5; // per Season
	public static var SpringWildFoodRegrowChance:Float = 1; // per Season // use spring and summer
	public static var GrowBackPlantsIncreaseIfLowPopulation:Float = 2; 
	public static var GrowBackOriginalPlantsFactor:Float = 0.05; //0.05; // 0.4 // 0.1 // regrow from original plants per season
	public static var GrowNewPlantsFromExistingFactor:Float = 0.1; // 0.2 // offsprings per season per plant

	// public static var WinterFildWoodDecayChance = 0.2;
	// Ally
	public static var TimeConfirmNewFollower:Float = 15; // a new follower is confirmed after X seconds

	// combat
	public static var CombatAngryTimeBeforeAttack:Float = 5;
	public static var CombatExhaustionCostPerAttack:Float = 0.1;
	public static var WeaponCoolDownFactor:Float = 0.05;
	public static var MaleDamageFactor:Float = 1.2;
	public static var WeaponCoolDownFactorIfWounding:Float = 0.4;
	// public static var AnimalCoolDownFactorIfWounding:Float = 0.2;
	public static var AnimalDamageFactor:Float = 1.5; // 1.5
	public static var AnimalDamageFactorInWinter:Float = 2; // 2
	public static var AnimalDamageFactorIfAttacked:Float = 1.5;
	public static var WeaponDamageFactor:Float = 1;
	public static var WoundDamageFactor:Float = 1;
	public static var CursedDamageFactor:Float = 0.5;
	public static var TargetWoundedDamageFactor:Float = 0.5;
	public static var AllyConsideredClose:Int = 5;
	public static var WoundHealingTimeFactor:Float = 1.5;
	public static var AllyStrenghTooLowForPickup:Float = 0; // 0.8
	public static var PrestigeCostPerDamageForAlly:Float = 1; // 0. 5 // For damaging ally
	public static var PrestigeCostPerDamageForChild:Float = 5; // 2
	public static var PrestigeCostPerDamageForCloseRelatives:Float = 0.5; //0.25// For damaging children, mother, father, brother sister
	public static var PrestigeCostPerDamageForWomenWithoutWeapon:Float = 0.5; //0.25

	// AI
	public static var NumberOfAis:Int = 30;
	public static var NumberOfAiPx:Int = 0;
	public static var AiReactionTime:Float = 0.5; //0.5; // 0.5;
	public static var TimeToAiRebirthPerYear:Float = 10; // X seconds per not lived year = 60 - death age
	public static var AiTotalScoreFactor:Float = 0.5;
	public static var AiTimeToWaitIfCraftingFailed:Float = 15; // if item failed to craft dont craft for X seconds
	public static var AiMaxSearchRadius:Int = 60;
	public static var AiMaxSearchIncrement:Int = 30; // 16
	public static var AiIgnoreTimeTransitionsLongerThen:Int = 120; // 30
	public static var AgingFactorHumanBornToAi:Float = 3; // 3
	public static var AgingFactorAiBornToHuman:Float = 1.5;
	public static var AiNameEnding:String = 'X'; // A name ending / set '' if none	
	public static var AIAllowBuildOven:Bool = false;
	public static var AIAllowBuilKiln:Bool = false;

	// Ai speed
	public static var AISpeedFactorSerf:Float = 0.8;
	public static var AISpeedFactorCommoner:Float = 0.9;
	public static var AISpeedFactorNoble:Float = 1;

	// Ai food use
	public static var AIFoodUseFactorSerf:Float = 0.8;
	public static var AIFoodUseFactorCommoner:Float = 0.9;
	public static var AIFoodUseFactorNoble:Float = 1;

	public static function CanObjectBeLuckySpot(obj:Int):Bool {
		// 942 Muddy Iron Vein (can now respawn but not be lucky spot)
		// 3962 Loose Muddy Iron Vein
		// 3961 Iron Vein (can now respawn but not be lucky spot)
		// 3030 Natural Spring
		// 2285 Tarry Spot
		// 503 Dug Big Rock
		return (obj != 3030 && obj != 2285 && obj != 503 && obj != 942 && obj != 3961 && obj != 3962 );
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
			} else {
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
				} else {
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

			if(obj.description.contains('groundOnly')){
				obj.groundOnly = true;
				//trace('groundOnly: ${obj.name}');
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
				//trace('Mechanism: ${obj.name}');
			}

			/*if (obj.description.contains("Glass")) {
				obj.containSize = 2;
				obj.containable = true;
				trace('Glass: ${obj.name}');
			}*/	

			if (obj.description.contains("Blowpipe")) {
				obj.containSize = 2;
				obj.containable = true;
				//trace('Blowpipe: ${obj.name}');
			}

			if (obj.description.contains("Crucible") && obj.description.contains("in Wooden") == false) {
				obj.containSize = 2;
				obj.containable = true;
				//trace('Crucible: ${obj.name}');
			}
			
			if (obj.description.indexOf("Steel") != -1) {
				// trace('Decays to: ${obj.name}');
				obj.decaysToObj = 862; // 862 Broken Steel Tool no wood // 858 Broken Steel Tool
			}

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

			if(obj.description.contains('Shears')){
				//trace('${obj.name} permanent: ${obj.permanent}');
				obj.permanent = 0;
				obj.containSize = 1;
				obj.containable = true;
			}
		}

		ObjectData.getObjectData(0).containSize = 1; // Empty
		ObjectData.getObjectData(0).containable = true; // Empty

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

		ObjectData.getObjectData(2578).containSize = 2; //Cool Glass
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

		// set decay for ancient
		ObjectData.getObjectData(898).decayFactor = 0.02; // Ancient Stone Floor
		ObjectData.getObjectData(898).decaysToObj = 1853; // Ancient Stone Floor ==> Cut Stones

		ObjectData.getObjectData(895).decayFactor = 0.02; // Ancient Stone Wall (corner)
		ObjectData.getObjectData(895).decaysToObj = 1853; // Ancient Stone Wall ==> Cut Stones

		ObjectData.getObjectData(896).decayFactor = 0.02; // Ancient Stone Wall (horizontal)
		ObjectData.getObjectData(896).decaysToObj = 1853; // Ancient Stone Wall ==> Cut Stones

		ObjectData.getObjectData(897).decayFactor = 0.02; // Ancient Stone Wall (vertical)
		ObjectData.getObjectData(897).decaysToObj = 1853; // Ancient Stone FloWallor ==> Cut Stones

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

		ObjectData.getObjectData(3130).decaysToObj = 881;  // Ready Diesel Mining Pick without Bit
		ObjectData.getObjectData(3130).decayFactor =  0.1; // Diesel Mining Pick without Bit
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
		ObjectData.getObjectData(3157).decaysToObj = 780; // Escaped Horse-Drawn Tire Cart --> Escaped Horse-Drawn Cart
		ObjectData.getObjectData(780).decaysToObj = 775; // Escaped Horse-Drawn Tire Cart --> Escaped Riding Horse
		ObjectData.getObjectData(775).decaysToObj = 769; // Escaped Riding Horse --> Wild Horse

		ObjectData.getObjectData(3159).decaysToObj = 779; // Hitched Horse-Drawn Tire Cart --> Hitched Horse-Drawn Cart
		ObjectData.getObjectData(3159).decayFactor = 0.2; // Hitched Horse-Drawn Tire Cart
		ObjectData.getObjectData(779).decaysToObj = 774; // Hitched Horse-Drawn Cart --> Hitched Riding Horse
		ObjectData.getObjectData(779).decayFactor = 0.2; // Hitched Horse-Drawn Cart
		ObjectData.getObjectData(774).decaysToObj = 4154; // Hitched Riding Horse --> Hitching Post
		ObjectData.getObjectData(774).decayFactor = 0.2; // Hitched Riding Horse

		// set floor decay
		ObjectData.getObjectData(1596).decayFactor = 0.1; // 1596 Stone Road
		ObjectData.getObjectData(1596).decaysToObj = 291; // 1596 Stone Road ==> 291 Flat Rock

		ObjectData.getObjectData(884).decayFactor = 0.1; // 884 Stone Floor
		ObjectData.getObjectData(884).decaysToObj = 881; // 884 Stone Floor ==> 881 Cut Stones

		ObjectData.getObjectData(888).decayFactor = 0.5; // 888 Bear Skin Rug
		ObjectData.getObjectData(888).decaysToObj = 884; // 888 Bear Skin Rug ==> Stone Floor	

		ObjectData.getObjectData(3290).decayFactor = 2; // 3290 Pine Floor

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
		ObjectData.getObjectData(111).decaysToObj = 96; //Pine Wall ==> 96 Pine Needles
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
		//ObjectData.getObjectData(876).rValue = 0.9; // 75% // Wooden Door 
		ObjectData.getObjectData(876).decaysToObj = 470; // Wooden Door (horizontal) ==> Boards
		ObjectData.getObjectData(878).decaysToObj = 470; // Open Wooden Door (horizontal) ==> Boards
		ObjectData.getObjectData(878).rValue = 0.2; // Open Wooden Door (horizontal)

		ObjectData.getObjectData(877).decaysToObj = 470; // Wooden Door (vertical) ==> Boards
		ObjectData.getObjectData(879).decaysToObj = 470; // Open Wooden Door (vertical) ==> Boards
		ObjectData.getObjectData(879).rValue = 0.2; // Open Wooden Door (vertical)
		ObjectData.getObjectData(2757).rValue = 0.95; // Springy Wooden Door
		ObjectData.getObjectData(2757).decaysToObj = 876; // Springy Wooden Door ==> Wooden Door
		ObjectData.getObjectData(2757).blocksAnimal = true; // Springy Wooden Door

		// set stone wall decay	
		ObjectData.getObjectData(885).decayFactor = 0.2; //  Stone Wall+cornerStone
		ObjectData.getObjectData(885).decaysToObj = 1853; //  Stone Wall+cornerStone ==> Cut Stones
		ObjectData.getObjectData(886).decayFactor = 0.2; //  Stone Wall+verticalStone
		ObjectData.getObjectData(886).decaysToObj = 1853; //  Stone Wall+verticalStone  ==> Cut Stones
		ObjectData.getObjectData(887).decayFactor = 0.2; //  Stone Wall+horizontalStone
		ObjectData.getObjectData(887).decaysToObj = 1853; //  Stone Wall+horizontalStone  ==> Cut Stones
		//trace('isPermanent ${ObjectData.getObjectData(155).isPermanent()}');

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
		
		// set hungry work
		// TODO use tool hungry work factor
		/*
		ObjectData.getObjectData(34).hungryWork = 1 * HungryWorkToolCostFactor; // Sharp Stone
		ObjectData.getObjectData(334).hungryWork = 1 * HungryWorkToolCostFactor; // Steel Axe
		ObjectData.getObjectData(502).hungryWork = 1 * HungryWorkToolCostFactor; // Shovel // TODO should be cheaper then sharp stone
		*/
		//ObjectData.getObjectData(334).hungryWork = -1; // Steel Axe
		//ObjectData.getObjectData(502).hungryWork = -1; // Shovel 
		ObjectData.getObjectData(857).hungryWork = -2; // Steel Hoe

		ObjectData.getObjectData(1849).hungryWork = 10; // Buried Grave with Dug Stone

		ObjectData.getObjectData(123).hungryWork = 2; // Harvested Tule
		ObjectData.getObjectData(231).hungryWork = 10; // Adobe Oven Base

		ObjectData.getObjectData(1020).hungryWork = 2; // Snow Bank
		ObjectData.getObjectData(138).hungryWork = 2; // Cut Sapling Skewer
		ObjectData.getObjectData(3961).hungryWork = 5; // Iron Vein
		ObjectData.getObjectData(496).hungryWork = 5; // Dug Stump
		ObjectData.getObjectData(1011).hungryWork = 5; // Buried Grave
		//ObjectData.getObjectData(357).hungryWork = 5; // Bone Pile // Dont set!!!

		ObjectData.getObjectData(213).hungryWork = 3; // Deep Tilled Row
		ObjectData.getObjectData(1136).hungryWork = 3; // Shallow Tilled Row

		ObjectData.getObjectData(511).hungryWork = 2; // Pond
		ObjectData.getObjectData(1261).hungryWork = 2; // Canada Goose Pond with Egg
		ObjectData.getObjectData(141).hungryWork = 2; // Canada Goose Pond
		ObjectData.getObjectData(142).hungryWork = 2; // Canada Goose Pond swimming
		ObjectData.getObjectData(143).hungryWork = 2; // Canada Goose Pond swimming, feather
		ObjectData.getObjectData(662).hungryWork = 1; // Shallow Well
		ObjectData.getObjectData(663).hungryWork = 2; // Deep Well

		// ObjectData.getObjectData(496).alternativeTransitionOutcome = 10; // Dug Stump

		// let loved food grow in loved biomes
		ObjectData.getObjectData(4251).biomes.push(BiomeTag.GREY); // Wild Garlic is loved now by White
		//ObjectData.getObjectData(36).biomes.push(BiomeTag.SNOW); // Wild Carrot is loved now by Ginger

		// is set directly in map WorldMap generation
		// ObjectData.getObjectData(141).biomes.push(BiomeTag.PASSABLERIVER); // Canada Goose Pond
		// ObjectData.getObjectData(121).biomes.push(BiomeTag.PASSABLERIVER); // Tule Reeds
		ObjectData.getObjectData(141).secondTimeOutcome = 142; // Canada Goose Pond ==> Canada Goose Pond swimming
		ObjectData.getObjectData(141).secondTimeOutcomeTimeToChange = 30;

		ObjectData.getObjectData(142).secondTimeOutcome = 1261; // Canada Goose Pond swimming ==> Canada Goose Pond with Egg
		ObjectData.getObjectData(142).secondTimeOutcomeTimeToChange = 60 * 10;

		ObjectData.getObjectData(1261).secondTimeOutcome = 142; // Canada Goose Pond with Egg ==> Canada Goose Pond swimming
		ObjectData.getObjectData(1261).secondTimeOutcomeTimeToChange = 60 * 4;

		ObjectData.getObjectData(141).countsOrGrowsAs = 1261; // Canada Goose Pond
		ObjectData.getObjectData(142).countsOrGrowsAs = 1261; // Canada Goose Pond swimming
		ObjectData.getObjectData(510).countsOrGrowsAs = 1261; // Pond with Dead Goose plus arrow
		ObjectData.getObjectData(509).countsOrGrowsAs = 1261; // Pond with Dead Goose
		ObjectData.getObjectData(511).countsOrGrowsAs = 1261; // Pond
		ObjectData.getObjectData(512).countsOrGrowsAs = 1261; // Dry Pond

		ObjectData.getObjectData(409).countsOrGrowsAs = 125;  // Clay Pit (partial) --> Clay Deposit 

		ObjectData.getObjectData(404).countsOrGrowsAs = 1435;  // Bison with Calf --> Bison

		ObjectData.getObjectData(1328).countsOrGrowsAs = 1323;  // Wild Boar with Piglet --> Wild Boar

		ObjectData.getObjectData(762).countsOrGrowsAs = 761;  // Flowering Barrel Cactus --> Barrel Cactus
		ObjectData.getObjectData(763).countsOrGrowsAs = 761;  // Fruiting Barrel Cactus --> Barrel Cactus

		ObjectData.getObjectData(2145).countsOrGrowsAs = 2142;  // Empty Banana Plant --> Banana Plant
		ObjectData.getObjectData(279).countsOrGrowsAs = 30;  // Empty Wild Gooseberry Bush --> Wild Gooseberry Bush

		ObjectData.getObjectData(164).secondTimeOutcome = 173; // Rabbit Hole out,single ==> Rabbit Family Hole out
		ObjectData.getObjectData(164).secondTimeOutcomeTimeToChange = 90;

		ObjectData.getObjectData(173).secondTimeOutcome = 3566; // Rabbit Family Hole out ==> Fleeing Rabbit
		ObjectData.getObjectData(173).secondTimeOutcomeTimeToChange = 90;

		ObjectData.getObjectData(164).countsOrGrowsAs = 161; // Rabbit Hole out,single couts as Rabbit Hole
		ObjectData.getObjectData(173).countsOrGrowsAs = 161; // Rabbit Family Hole couts as Rabbit Hole

		ObjectData.getObjectData(3566).countsOrGrowsAs = 161; // Fleeing Rabbit
		// dont block walking TODO needs client change
		//ObjectData.getObjectData(231).blocksWalking = false; // Adobe Oven Base
		//ObjectData.getObjectData(237).blocksWalking = false; // Adobe Oven

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
		
		// set loved biomes right
		ObjectData.getObjectData(1328).biomes = []; // Wild Boar with Piglet
		ObjectData.getObjectData(1328).biomes.push(BiomeTag.SWAMP); // Wild Boar with Piglet

		ObjectData.getObjectData(631).biomes = []; // Hungry Grizzly Bear
		ObjectData.getObjectData(631).biomes.push(BiomeTag.GREY); // Hungry Grizzly Bear
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

		ObjectData.getObjectData(411).speedMult = SemiHeavyItemSpeed; // Fertile Soil Reduced carring speed
		ObjectData.getObjectData(345).speedMult = SemiHeavyItemSpeed; // Butt Log
		ObjectData.getObjectData(126).speedMult = SemiHeavyItemSpeed; // Clay
		ObjectData.getObjectData(127).speedMult = SemiHeavyItemSpeed; // Adobe
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

		// nerve food
		ObjectData.getObjectData(2143).foodValue = 6; // banana // origional 7
		ObjectData.getObjectData(31).foodValue = 4; // Gooseberry // origional 3
		ObjectData.getObjectData(2855).foodValue = 4; // Onion // origional 5
		ObjectData.getObjectData(808).foodValue = 4; // Wild Onion // origional 4
		ObjectData.getObjectData(807).foodValue = 5; // Burdock Rootl 7
		//ObjectData.getObjectData(40).foodValue = 5; // Wild Carrot // origional 5
		//ObjectData.getObjectData(402).foodValue = 5; // Carrot // origional 5

		// boost hunted food
		ObjectData.getObjectData(197).foodValue = 15; // Cooked Rabbit 10 --> 15
		ObjectData.getObjectData(2190).foodValue = 20; // Turkey Slice on Plate 17 --> 20
		ObjectData.getObjectData(1285).foodValue = 15; // Omelette 12 --> 15

		// ObjectData.getObjectData(197).useChance = 0.3; // Cooked Rabbit
		// ObjectData.getObjectData(2190).useChance = 0.3; // Turkey Slice on Plate
		// ObjectData.getObjectData(518).useChance = 0.3; // Cooked Goose
		// ObjectData.getObjectData(2143).useChance = 0.3; // Banana

		// soil should replace water as most needed ressource
		// composted soil has default 7 uses and each of it can be used twice for soil so in total 14
		ObjectData.getObjectData(624).numUses = 5; // default 7 Composted Soil Uses: 3 Soil (Wheat, Berry, Dung) + water ==> 4 Soil
		//ObjectData.getObjectData(411).useChance = 0.5; // Fertile Soil Pit 9 uses --> 18

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
		//ObjectData.getObjectData(391).winterDecayFactor = 1; // Domestic Gooseberry Bush
		ObjectData.getObjectData(391).springRegrowFactor = 0.2; // Domestic Gooseberry Bush
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
		ObjectData.getObjectData(418).damage = 4; // 3 // Wolfs
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
		ObjectData.getObjectData(1851).decaysToObj = 556; // Fence Gate ==> Fence Kit
		ObjectData.getObjectData(1851).blocksDomesticAnimal = true; // Fence Gate
		ObjectData.getObjectData(1851).blocksAnimal = true; // Fence Gate
		ObjectData.getObjectData(1851).groundOnly = true; // Fence Gate
		
		ObjectData.getObjectData(4154).decayFactor = ObjDecayFactorOnFloor; // Hitching Post
		ObjectData.getObjectData(4154).decaysToObj = 556; // Hitching Post  ==> Fence Kit
		ObjectData.getObjectData(4154).groundOnly = true; // Hitching Post

		ObjectData.getObjectData(550).decayFactor = ObjDecayFactorOnFloor; // Fence
		ObjectData.getObjectData(550).decaysToObj = 556; // Fence  ==> Fence Kit
		ObjectData.getObjectData(550).groundOnly = true; // Fence

		ObjectData.getObjectData(549).decayFactor = ObjDecayFactorOnFloor; // Fence + verticalFence
		ObjectData.getObjectData(549).decaysToObj = 556; //  Fence + verticalFence  ==> Fence Kit
		ObjectData.getObjectData(549).groundOnly = true; // Fence + verticalFence

		ObjectData.getObjectData(551).decayFactor = ObjDecayFactorOnFloor; // Fence +cornerFence
		ObjectData.getObjectData(551).decaysToObj = 556; // Fence +cornerFence ==> Fence Kit
		ObjectData.getObjectData(551).groundOnly = true; // Fence +cornerFence
		
		ObjectData.getObjectData(556).blocksDomesticAnimal = true; // Fence Kit

		ObjectData.getObjectData(3862).decayFactor = ObjDecayFactorOnFloor; // Dung Box
		ObjectData.getObjectData(3862).decaysToObj = 434; // Dung Box ==> Wooden Box
		ObjectData.getObjectData(3862).groundOnly = true; // Dung Box

		for (objData in ObjectData.importedObjectData) {
			if (objData.description.contains('Sports Car')) {
				objData.isBoat = true;
				objData.speedMult = 3.5;
			}
		}

		ObjectData.getObjectData(4647).unreleased = true; // Truck Chassis

		ObjectData.getObjectData(1605).numSlots = 0; //Stack of Baskets // TODO allow stacking of filled baskets

		// ObjectData.getObjectData(279).winterDecayFactor = 2; // Empty Wild Gooseberry Bush
		// ObjectData.getObjectData(279).springRegrowFactor = 0.5; // Empty Wild Gooseberry Bush
		// ObjectData.getObjectData(279).countsOrGrowsAs = 30; // Empty Wild Gooseberry Bush

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
	}

	public static function PatchTransitions(transtions:TransitionImporter) {
		// TODO set through transions
		ObjectData.getObjectData(30).lastUseObject = 279; // Wild Gooseberry Bush ==> Empty Wild Gooseberry Bush
		ObjectData.getObjectData(279).undoLastUseObject = 30; // Empty Wild Gooseberry Bush ==> Wild Gooseberry Bush

		var trans = transtions.getTransition(-1, 282); // Firing Adobe Kiln ==> Ancient
		trans.autoDecaySeconds = 40; // default: 30 

		var trans = transtions.getTransition(-1, 885); // Stone Wall (Corner) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10 
		var trans = transtions.getTransition(-1, 886); // Stone Wall (vertical) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10 
		var trans = transtions.getTransition(-1, 887); // Stone Wall (horizontal) ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10 
		var trans = transtions.getTransition(-1, 884); // Stone Floor ==> Ancient
		trans.autoDecaySeconds = -24 * 10; // default: -10 // TODO implement time for floors

		// lower age for weapons since kids so or so make less damage since they have less health pipes
		ObjectData.getObjectData(151).minPickupAge = 10; // 12   // War Sword
		ObjectData.getObjectData(151).minPickupAge = 5; // 10   // Yew Bow
		ObjectData.getObjectData(560).minPickupAge = 2; // 8    // Knife

		// TODO allow damage with bloody weapon / needs support from client?
		ObjectData.getObjectData(560).damage = 4; // Knife  // damage per sec = 2
		ObjectData.getObjectData(3047).damage = 6; // War Sword // damage per sec = 3
		ObjectData.getObjectData(152).damage = 6; // Bow and Arrow  //
		ObjectData.getObjectData(1624).damage = 10; // Bow and Arrow with Note  //
		
		var trans = transtions.getTransition(-1, 750); // Bloody Knife
		trans.autoDecaySeconds = 15;

		var trans = transtions.getTransition(-1, 3048); // Bloody War Sword
		trans.autoDecaySeconds = 10;

		var trans = transtions.getTransition(-1, 749); // Bloody Yew Bow
		trans.autoDecaySeconds = 30;

		// Knife transitions for close combat
		var trans = new TransitionData(560, 418, 750, 422); // Knife + Wolf ==> Bloody Knife + Dead Wolf
		transtions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(560, 1323, 750, 1332); // Knife + Wild Boar ==> Bloody Knife +  Dead Boar
		transtions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(560, 1328, 750, 1331); // Knife + Wild Boar with Piglet ==> Bloody Knife + Shot Boar with Piglet
		transtions.addTransition("PatchTransitions: ", trans);

		// Sword transitions for close combat
		var trans = new TransitionData(3047, 418, 3048, 422); // War Sword + Wolf ==> Bloody War Sword + Dead Wolf
		transtions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(3047, 1323, 3048, 1332); // War Sword + Wild Boar ==> Bloody War Sword +  Dead Boar
		transtions.addTransition("PatchTransitions: ", trans);
		var trans = new TransitionData(3047, 1328, 3048, 1331); // War Sword + Wild Boar with Piglet ==> Bloody War Sword + Shot Boar with Piglet
		transtions.addTransition("PatchTransitions: ", trans);
		
		var trans = TransitionImporter.GetTransition(152, 0); // Bow and Arrow + 0 
		trans.newActorID = 151; // Yew Bow instead of Yew Bow just shot
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = transtions.getTransition(-1, 400); // Carrot Row
		trans.autoDecaySeconds = 10 * 60; // 5 * 60

		// FIX bug that this bow cannot be used with quiver
		trans = new TransitionData(-1, 493, 0, 151); // Yew Bow just shot --> Yew Bow
		trans.autoDecaySeconds = 2;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = transtions.getTransition(-1, 427); // Attacking Wolf
		trans.autoDecaySeconds = 3;
		trans.move = 5;
		var trans = transtions.getTransition(-1, 428); // Attacking Shot Wolf
		trans.autoDecaySeconds = 3;
		trans.move = 2;

		var trans = transtions.getTransition(-1, 1385); // Attacking Rattle Snake
		trans.autoDecaySeconds = 3;

		var trans = transtions.getTransition(-1, 1333); // Attacking Wild Boar
		trans.autoDecaySeconds = 3;
		var trans = transtions.getTransition(-1, 1334); // Attacking Wild Boar with Piglet
		trans.autoDecaySeconds = 3;

		var trans = transtions.getTransition(-1, 653); // Hungry Grizzly Bear attacking
		trans.autoDecaySeconds = 3;
		var trans = transtions.getTransition(-1, 654); // Shot Grizzly Bear 1 attacking
		trans.autoDecaySeconds = 3;
		var trans = transtions.getTransition(-1, 655); // Shot Grizzly Bear 2 attacking
		trans.autoDecaySeconds = 3;
		var trans = transtions.getTransition(-1, 637); // Shot Grizzly Bear 3 attacking
		trans.autoDecaySeconds = 3;

		// wounds decay differenctly on ground vs on player
		ObjectData.getObjectData(797).alternativeTimeOutcome = 1380; // Stable Knife Wound --> Clean Knife Wound // on player
		trans = new TransitionData(-1, 797, 0, 0); // Stable Knife Wound --> Empty // on ground
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1380, 0, 0); // Clean Knife Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1363).alternativeTimeOutcome = 1381; // Bite Wound --> Clean Bite Wound
		trans = new TransitionData(-1, 1363, 0, 0); //  Bite Wound --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1381, 0, 0); // Clean Bite Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1377).alternativeTimeOutcome = 1384; // Snake Bite -->  Clean Snake Bite
		trans = new TransitionData(-1, 1377, 0, 0); //  Snake Bite --> Empty
		trans.autoDecaySeconds = 20 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1384, 0, 0); //  Clean Snake Bite --> 0
		trans.autoDecaySeconds = 600 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1366).alternativeTimeOutcome = 1383; // Hog Cut --> Clean Hog Cut
		trans = new TransitionData(-1, 1364, 0, 0); // Hog Cut --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1383, 0, 0); // Clean Hog Cut --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);

		ObjectData.getObjectData(1366).alternativeTimeOutcome = 1382; // Empty Arrow Wound --> Clean Arrow Wound
		trans = new TransitionData(-1, 1366, 0, 0); // Empty Arrow Wound --> Empty
		trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);
		trans = new TransitionData(-1, 1382, 0, 0); // Clean Arrow Wound --> 0
		trans.autoDecaySeconds = 90 * WoundHealingTimeFactor;
		transtions.addTransition("PatchTransitions: ", trans);

		// [1367] Extracted Arrowhead Wound
		trans = new TransitionData(-1, 1367, 0, 0); // Extracted Arrowhead Wound --> 0
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = transtions.getTransition(0, 798);
		ObjectData.getObjectData(798).alternativeTimeOutcome = trans.newTargetID; // Arrow Wound --> Embedded Arrowhead Wound
		trans.newTargetID = 1367; // Arrow Wound --> Extracted Arrowhead Wound
		transtions.addTransition("PatchTransitions: ", trans);

		trans = transtions.getTransition(0, 1367);
		ObjectData.getObjectData(1367).alternativeTimeOutcome = trans.newTargetID; // Extracted Arrowhead Wound --> Gushing Empty Arrow Wound
		trans.newTargetID = 1366; // Extracted Arrowhead Wound --> Empty Arrow Wound
		transtions.addTransition("PatchTransitions: ", trans);

		// More decay transitions
		trans = new TransitionData(-1, 798, 0, 1365); // 798 Arrow Wound --> 1365 Embedded Arrowhead Wound
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1365, 0, 0); // 1365 Embedded Arrowhead Wound --> 0
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 421, 0, 422); // 421 Dead Wolf with Arrow --> 422 Dead Wolf
		trans.autoDecaySeconds = -12;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 565, 0, 566); // 565 Butchered Mouflon --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 422, 0, 566); // 422 Dead Wolf --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 423, 0, 566); // 423 Skinned Wolf --> TODO 566 Mouflon Bones
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1340, 0, 1343); // 1340 Butchered Pig --> Pig Bones
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		// Clear Bison
		trans = new TransitionData(-1, 1438, 0, 1435); // Shot Bison --> Bison
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1440, 0, 1436); // Shot Bison with Calf --> Bison with Calf
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		// dead bison already exists
		trans = new TransitionData(-1, 1442, 0, 1444); // Dead Bison arrow 2 --> Dead Bison arrow 1
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);
		
		trans = new TransitionData(-1, 1444, 0, 1446); // Dead Bison arrow 1 --> Dead Bison
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1444, 0, 1446); // Dead Bison arrow 1 --> Dead Bison
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1441, 0, 1443); // Dead Bison with Calf arrow 2  --> Dead Bison with Calf arrow 1 
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1443, 0, 1445); // Dead Bison with Calf arrow 1  --> Dead Bison with Calf
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1445, 0, 1437); // Dead Bison with Calf  --> Bison Calf
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		// Clear dead Turkey
		trans = new TransitionData(-1, 2176, 0, 2177); // Shot Turkey with Arrow  --> Shot Turkey
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2177, 0, 0); // Shot Turkey --> 0
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2179, 0, 0); // Shot Turkey no feathers --> 0
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		// Clear up Boar
		trans = new TransitionData(-1, 1331, 0, 1335); // Shot Boar with Piglet --> Fleeing Wild Piglet
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1330, 0, 1332); // Shot Boar --> Dead Boar
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 1332, 0, 1343); // Dead Boar --> Pig Bones
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		// Mouflon
		trans = new TransitionData(-1, 562, 0, 566); // Skinned Mouflon --> Mouflon Bones
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = transtions.getTransition(-1, 1343); // Pig Bones
		trans.autoDecaySeconds = -4; // default -2

		trans = transtions.getTransition(-1, 891); // 891 Cracking Adobe Wall
		trans.autoDecaySeconds = -24; // default -0.5

		trans = transtions.getTransition(-1, 155); // Adobe Wall
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
				//trace('Debug ${trans.getDesciption()}');
				// trans.traceTransition("PatchTransitions: ", true);

				trans.actorID = 0;
				transtions.addTransition("PatchTransitions: ", trans);
				trans.traceTransition("PatchTransitions: ");
			}

			if (trans.autoDecaySeconds == -168) {
				trans.autoDecaySeconds = -2; // use chance 20%: 10 uses / 120 min 0.08 Bowls per min

				// trans.traceTransition("PatchTransitions: ", true);
			}

			if (trans.autoDecaySeconds == 9000) // 150 min like deep well 0.53 bowls per min
			{
				trans.autoDecaySeconds = 1200; // 20 min one bucket 12.5% (bucket has 10 uses): 80 uses / 20 min = 4 bowls per min

				// trans.traceTransition("PatchTransitions: ", true);
			}

			if (trans.autoDecaySeconds == 2160) // 36 min like well  0.91 bowls per min
			{
				trans.autoDecaySeconds = 720; // 12 min one bowl 3%: 33 uses / 12 min = 2.7 bowls per min

				// trans.traceTransition("PatchTransitions: ", true);
			}
		}

		// Fix pickup transitions

		// Escaped Horse-Drawn Cart just released
		trans = transtions.getTransition(0, 1422); 
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Cart
		trans = transtions.getTransition(0, 780); 
		trans.isPickupOrDrop = true;
		// Hitched Horse-Drawn Cart
		trans = transtions.getTransition(0, 779); 
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Tire Cart released
		trans = transtions.getTransition(0, 3161); 
		trans.isPickupOrDrop = true;
		// Escaped Horse-Drawn Tire Cart
		trans = transtions.getTransition(0, 3157); 
		trans.isPickupOrDrop = true;
		// Hitched Horse-Drawn Tire Cart
		trans = transtions.getTransition(0, 3159); 
		trans.isPickupOrDrop = true;

		// Graves
		trans = transtions.getTransition(292, 87); // Basket + Fresh Grave
		trans.isPickupOrDrop = true;
		trans = transtions.getTransition(292, 88); // Basket + Grave
		trans.isPickupOrDrop = true;	
		trans = transtions.getTransition(292, 89); // Basket + Old Grave
		trans.isPickupOrDrop = true;
		trans = transtions.getTransition(292, 357); // Basket + Bone Pile
		trans.isPickupOrDrop = true;

		trans = transtions.getTransition(356, -1); // Basket of Bones + 0
		trans.isPickupOrDrop = true; 
		
		// Original: Riding Horse: 770 + -1 = 0 + 1421
		trans = new TransitionData(770, 0, 0, 1421);
		transtions.addTransition("PatchTransitions: ", trans);

		// TODO this should function somehow with categories???
		// original transition makes cart loose rubber if putting down horse cart
		// Original: 3158 + -1 = 0 + 1422 // Horse-Drawn Tire Cart + ???  -->  Empty + Escaped Horse-Drawn Cart --> must be: 3158 + -1 = 0 + 3161
		trans = transtions.getTransition(3158, -1); // Horse-Drawn Tire Cart
		trans.newTargetID = 3161;
		trans.traceTransition("PatchTransitions: ");

		//trans = transtions.getTransition(3158, 550); // Horse-Drawn Tire Cart
		//trans.newTargetID = 3161;
		//trace('trans: ${trans.getDesciption()}');

		// original transition makes cart loose rubber if picking up horse cart

		// Original:  0 + 3161 = 778 + 0 //Empty + Escaped Horse-Drawn Tire Cart# just released -->  Horse-Drawn Cart + Empty
		// comes from pattern:  <0> + <1422> = <778> + <0> / EMPTY + Escaped Horse-Drawn Cart# just released -->  Horse-Drawn Cart + EMPTY
		trans = transtions.getTransition(0, 3161);
		trans.newActorID = 3158; // Horse-Drawn Tire Cart
		trans.traceTransition("PatchTransitions: ");

		trans = transtions.getTransition(-1, 3161); // Escaped Horse-Drawn Tire Cart just released
		trans.newTargetID = 3157; // Escaped Horse-Drawn Tire Cart
		trans.autoDecaySeconds = 20; // default 7
		trans.traceTransition("PatchTransitions: ");

		trans = transtions.getTransition(-1, 3157); // Escaped Horse-Drawn Tire Cart
		trans.move = 2; // default 4

		trans = transtions.getTransition(0, 3157);
		trans.newActorID = 3158; // Horse-Drawn Tire Cart
		trans.traceTransition("PatchTransitions: ");

		// let Tule Stumps (122) grow back
		trans = transtions.getTransition(-1, 122);
		trans.newTargetID = 121; // 121 = Tule Reeds
		trans.autoDecaySeconds = -6;
		trans.traceTransition("PatchTransitions: ");

		// Escaped Horse-Drawn Cart just released
		trans = transtions.getTransition(-1, 1422);
		trans.autoDecaySeconds = 15;  // 7

		// Escaped Horse-Drawn Cart
		trans = transtions.getTransition(-1, 780);
		trans.move = 2; // default 4

		// Escaped Horse-Drawn Tire Cart just released??????
		//trans = transtions.getTransition(-1, 1361);
		//trans.autoDecaySeconds = 30;  // 7

		trans = transtions.getTransition(3158, 4154); // Horse-Drawn Tire Cart + Hitching Post
		trans.newTargetID = 3159; // Hitched Horse-Drawn Tire Cart
		//trace('DEBUG!!!: ${trans.getDesciption()}');

		trans = transtions.getTransition(3158, 550); // Horse-Drawn Tire Cart + Fence
		trans.newTargetID = 3159; // Hitched Horse-Drawn Tire Cart
		//trace('DEBUG!!!: ${trans.getDesciption()}');

		// 141 Canada Goose Pond
		// 1261 Canada Goose Pond with Egg // TODO let egg come back

		// change decay time for grave 88 = Grave
		// trans = transtions.getTransition(-1, 88);
		// trans.autoDecaySeconds = 10;
		// trans.traceTransition("PatchTransitions: ");

		// should be fixed now with the rest of the -2 transitions
		//-2 + 141 = 0 + 143 // some how we have -2 transactions like hand + ghoose pond = ghoose pond with feathers
		// trans = transtions.getTransition(-2, 141);
		// trans.actorID = 0;
		// transtions.addTransition("PatchTransitions: ", trans);
		// trans.traceTransition("PatchTransitions: ");

		// new bears needs the world
		trans = new TransitionData(-1, 650, 0, 630); //Bear Cave Empty --> Bear Cave
		trans.autoDecaySeconds = -48;
		transtions.addTransition("PatchTransitions: ", trans);

		// let get berrys back!
		trans = new TransitionData(-1, 30, 0, 30); // Wild Gooseberry Bush
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 279, 0, 30); // Empty Wild Gooseberry Bush --> // Wild Gooseberry Bush
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -1;
		transtions.addTransition("PatchTransitions: ", trans);

		// let get bana back!
		trans = new TransitionData(-1, 2142, 0, 2142); // Banana Plant
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(-1, 2145, 0, 2142); // Empty Banana Plant --> Banana Plant
		trans.reverseUseTarget = true;
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

		// get some sharpie back
		trans = new TransitionData(135, 850, 135, 34); // Flint Chip + Stone Hoe --> Flint Chip + Sharp Stone
		trans.aiShouldIgnore = true;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(135, 71, 135, 34); // Flint Chip + Stone Hatchet --> Flint Chip + Sharp Stone
		trans.aiShouldIgnore = true;
		transtions.addTransition("PatchTransitions: ", trans);

		// get some ropes back	
		trans = new TransitionData(34, 850, 34, 92); // Sharp Stone + Stone Hoe --> Sharp Stone + Tied Long Shaft
		trans.aiShouldIgnore = true;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(850, 850, 850, 92); // Stone Hoe + Stone Hoe --> Stone Hoe + Tied Long Shaft
		trans.aiShouldIgnore = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(0, 92, 59, 67); // 0 + Tied Long Shaft --> Rope + Long Straight Shaft
		transtions.addTransition("PatchTransitions: ", trans);
		
		//trans = new TransitionData(135, 71, 135, 70); // Flint Chip + Stone Hatchet --> Flint Chip + Tied Short Shaft
		trans = new TransitionData(34, 71, 34, 70); // Sharp Stone + Stone Hatchet --> Sharp Stone + Tied Short Shaft
		trans.aiShouldIgnore = true;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(866, 82, 0, 83); // Rag Loincloth + Fire --> 0 + Large Fast Fire
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(865, 82, 0, 83); // Rag Shirt + Fire --> 0 + Large Fast Fire
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(864, 82, 0, 83); // Rag Hat + Fire --> 0 + Large Fast Fire
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(869, 82, 0, 83); // Rag Shoe + Fire --> 0 + Large Fast Fire
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(34, 32, 33, 32); // Sharp Stone + Big Hard Rock --> Stone + Big Hard Rock
		trans.hungryWorkCost = 1;
		transtions.addTransition("PatchTransitions: ", trans);

		//  Wild Gooseberry Bush
		trans = new TransitionData(253, 30, 253, 30); // Bowl of Gooseberries + Wild Gooseberry Bush --> Bowl of Gooseberries(+1) + Wild Gooseberry Bush
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans);

		// Clay Bowl + Wild Gooseberry Bush --> Bowl of Gooseberries + Wild Gooseberry Bush
		trans = new TransitionData(235, 30, 253, 30); 
		trans.reverseUseActor = true; // otherwise new bowl will be full with berries
		transtions.addTransition("PatchTransitions: ", trans);

		// Bowl of Gooseberries + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Wild Gooseberry Bush
		trans = new TransitionData(253, 30, 253, 279); 
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans, false, true);

		trans = new TransitionData(235, 30, 253, 279); // Clay Bowl + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Wild Gooseberry Bush
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans, false, true);

		// Domestic Gooseberry Bush
		trans = new TransitionData(253, 391, 253,
			391); // Bowl of Gooseberries + Domestic Gooseberry Bush --> Bowl of Gooseberries(+1) + Domestic Gooseberry Bush
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans);

		trans = new TransitionData(235, 391, 253, 391); // Clay Bowl + Domestic Gooseberry Bush --> Bowl of Gooseberries + Domestic Gooseberry Bush
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans);

		// Bowl of Gooseberries + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Domestic Wild Gooseberry Bush
		trans = new TransitionData(253, 391, 253,1135); 
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans, false, true);

		// Clay Bowl + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Domestic Gooseberry  Bush
		trans = new TransitionData(235, 391, 253, 1135); 
		trans.reverseUseActor = true;
		transtions.addTransition("PatchTransitions: ", trans, false, true);

		// Fishing Pole without Hook + Bone Needle --> Fishing Pole + 0
		trans = new TransitionData(2092, 191, 2091, 0); 
		transtions.addTransition("PatchTransitions: ", trans);

		// 0 + Fishing Pole with Old Boot --> Old Boot + Fishing Pole
		trans = new TransitionData(0, 2098, 2099, 2091); 
		transtions.addTransition("PatchTransitions: ", trans);

		// 0 + Diesel Mining Pick without Bit --> Diesel Engine + Collapsed Iron Mine
		trans = new TransitionData(0, 3130, 2365, 945); 
		transtions.addTransition("PatchTransitions: ", trans);

		// 0 + Ready Diesel Mining Pick --> Steel Chisel + Diesel Mining Pick without Bit
		trans = new TransitionData(0, 3129, 455, 3130); 
		transtions.addTransition("PatchTransitions: ", trans);

		// 0 + Dry Diesel Water Pump --> Diesel Engine + Unpowered Pump Head
		trans = new TransitionData(0, 2388, 2365, 3964); 
		transtions.addTransition("PatchTransitions: ", trans);

		// hungry work transitions
		var trans = transtions.getTransition(502, 122); // Shovel + Tule Stumps ==> Adobe
		trans.hungryWorkCost = 5;
		var trans = transtions.getTransition(0, 125); // 0 + Clay Deposit ==> Clay
		trans.hungryWorkCost = 3;
		var trans = transtions.getTransition(0, 409); // 0 + Clay Pit ==> Clay
		trans.hungryWorkCost = 3;
		var trans = transtions.getTransition(502, 32); // Shovel + Big Hard Rock ==> Dug Big Hard Rock
		trans.hungryWorkCost = 10;
		var trans = transtions.getTransition(291, 486); // Flat Rock + Floor Stakes ==> Stone Road
		trans.hungryWorkCost = 5;
		var trans = transtions.getTransition(684, 1596); // Steel Mining Pick + Stone Road ==> Flat Rock
		trans.hungryWorkCost = 5;
		//var trans = transtions.getTransition(33, 32); // Stone + Big Hard Rock ==> Sharp Stone
		//trans.hungryWorkCost = 5;

		// most important allow kill moskitos
		// Firebrand + Mosquito Swarm --> 0 + Ashes
		trans = new TransitionData(248, 2156, 0,86); 
		trans.hungryWorkCost = 5;
		transtions.addTransition("PatchTransitions: ", trans, false, false);
	
		// Firebrand + Mosquito Swarm just bit --> 0 + Ashes
		trans = new TransitionData(248, 2157, 0,86); 
		trans.hungryWorkCost = 5;
		transtions.addTransition("PatchTransitions: ", trans, false, false);

		// Bowl of Soil + Hardened Row -- Shallow Tilled Row
		var trans = transtions.getTransition(1137, 848); 
		trans.hungryWorkCost = -5; // dont let is cost hungry work

		// Mallet + Dug Big Rock with Chisel -- Split Big Rock
		var trans = transtions.getTransition(467, 508); 
		trans.hungryWorkCost = 10;

		// give wolfs some meat // TODO change crafting maps
		var trans = transtions.getTransition(0, 423); // 423 Skinned Wolf
		trans.newTargetID = 565; // 565 Butchered Mouflon
		trans.targetNumberOfUses = 2; // give only two meat
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = transtions.getTransition(0, 709); // 709 Skinned Seal with fur
		trans.newTargetID = 1340; // 1340 Butchered Pig
		transtions.addTransition("PatchTransitions: ", trans);

		// give bison some meat // TODO change crafting maps
		var trans = transtions.getTransition(0, 1444); // Dead Bison
		trans.newTargetID = 565; // 565 Butchered Mouflon
		transtions.addTransition("PatchTransitions: ", trans);

		// allow to cook mutton on coals
		trans = new TransitionData(569, 85, 570, 85); // 569 Raw Mutton + 85 Hot Coals --> 570 Cooked Mutton + 85 Hot Coals
		transtions.addTransition("PatchTransitions: ", trans);

		// patch alternativeTransitionOutcomes // TODO use prob categories instead
		var trans = transtions.getTransition(502, 338); // shovel plus Stump
		trans.alternativeTransitionOutcome.push(72); // Kindling

		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(344); // Chopped Softwood Tree --> Fire Wood

		// push twice so that it has twice the chance
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(344); // Chopped Softwood Tree --> Fire Wood

		// push Kindling
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(72); // Chopped Tree Big Log--> Kindling
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(72); // Chopped Tree --> Kindling
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(72); // Chopped Softwood Tree --> Kindling

		// now push Butt Log
		ObjectData.getObjectData(342).alternativeTransitionOutcome.push(345); // Chopped Tree Big Log--> Butt Log
		ObjectData.getObjectData(340).alternativeTransitionOutcome.push(345); // Chopped Tree --> Butt Log
		ObjectData.getObjectData(3146).alternativeTransitionOutcome.push(345); // Chopped Softwood Tree --> Butt Log
		
		//ObjectData.getObjectData(99).alternativeTransitionOutcome.push(344); // White Pine Tree --> Fire Wood
		//ObjectData.getObjectData(100).alternativeTransitionOutcome.push(344); // White Pine Tree with Needles --> Fire Wood
		
		ObjectData.getObjectData(3146).hungryWork = ServerSettings.HungryWorkCost; // Chopped Softwood Tree
		//ObjectData.getObjectData(99).hungryWork = ServerSettings.HungryWorkCost; // White Pine Tree
		//ObjectData.getObjectData(100).hungryWork = ServerSettings.HungryWorkCost; // White Pine Tree with Needles

		
		//ObjectData.getObjectData(3944).alternativeTransitionOutcome.push(33); // Stripped Iron Vein --> Stone
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(33); // Iron Vein --> Stone
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(0); // Iron Vein --> 0
		ObjectData.getObjectData(3961).alternativeTransitionOutcome.push(0); // Iron Vein --> 0

		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(0); // Shallow Pit with Ore --> 0
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(33); // Shallow Pit with Ore --> Stone
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(33); // Shallow Pit with Ore --> Stone
		ObjectData.getObjectData(3956).alternativeTransitionOutcome.push(291); // Shallow Pit with Ore --> Flat Rock

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
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Stone
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(33); // Mine with Ore --> Flat Rock
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(291); // Mine with Ore --> Flat Rock
		ObjectData.getObjectData(3959).alternativeTransitionOutcome.push(503); // Mine with Ore --> Dug Big Rock

		//ObjectData.getObjectData(944).alternativeTransitionOutcome.push(291); // Iron Mine --> Flat Rock
		// TODO what to do with Diesel Mining Pick with Iron. It uses a time transition

		// allow more Stone Hoe to be used to dig graves // TODO make more HUNGRY WORK / TEST if they brake
		
		var trans = new TransitionData(850, 357, 850, 1011); // Stone Hoe + Bone Pile --> Stone Hoe + Buried Grave
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 87, 850, 1011); // Stone Hoe + Fresh Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 88, 850, 1011); // Stone Hoe + Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(850, 89, 850, 1011); // Stone Hoe + Old Grave --> Stone Hoe + Buried Grave
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		// allow more options to kill animals
		var trans = new TransitionData(152, 427, 151, 420); // Bow and Arrow + Attacking Wolf --> Yew Bow + Shot Wolf
		transtions.addTransition("PatchTransitions: ", trans);

		// FIX bucket transition // TODO why is this one missing?
		// <394> + <1099> = <394> + <660> --> <394> + <1099> = <394> + <1099> // make bucket not full
		var trans = new TransitionData(394, 1099, 394, 1099);
		trans.targetRemains = true;
		TransitionImporter.transitionImporter.createAndaddCategoryTransitions(trans);

		// pond animations
		/*
			var trans = transtions.getTransition(-1, 141); // Canada Goose Pond
			trans.newTargetID = 142; // Canada Goose Pond swimming
			trans.autoDecaySeconds = 5;
			transtions.addTransition("PatchTransitions: ", trans);
		*/
		
		var trans = transtions.getTransition(-1, 142); // Canada Goose Pond swimming
		trans.newTargetID = 141; // Canada Goose Pond
		trans.autoDecaySeconds = 20;
		transtions.addTransition("PatchTransitions: ", trans);
		
		var trans = transtions.getTransition(-1, 2180); // longer clothing decay Rabbit Fur Hat with Feather
		trans.autoDecaySeconds = -24; // 5

		var trans = transtions.getTransition(-1, 712); // Sealskin Coat
		trans.autoDecaySeconds = -24; // 5

		// give more time
		var trans = transtions.getTransition(-1, 304); // Firing Forge 304
		trans.autoDecaySeconds = 40; // normal 30

		var trans = transtions.getTransition(-1, 61); // Juniper Tinder
		trans.autoDecaySeconds = 5 * 60;

		var trans = transtions.getTransition(-1, 62); // Leaf
		trans.autoDecaySeconds = 5 * 60; // normal 2 * 60

		var trans = transtions.getTransition(-1, 75); // Ember Shaft
		trans.autoDecaySeconds = 20;

		var trans = transtions.getTransition(-1, 248); // Firebrand
		trans.autoDecaySeconds = 1.5 * 60;

		var trans = transtions.getTransition(-1, 80); // Burning Tinder
		trans.autoDecaySeconds = 15;

		var trans = transtions.getTransition(-1, 249); // Burning Adobe Oven
		trans.autoDecaySeconds = 25;

		var trans = transtions.getTransition(-1, 1281); // Cooked Omelette
		trans.autoDecaySeconds = 20;

		var trans = transtions.getTransition(-1, 861); // Old Hand Cart
		trans.autoDecaySeconds = -12;

		var trans = transtions.getTransition(-1, 1281); // Cooked Omelette
		trans.autoDecaySeconds = 20;

		var trans = transtions.getTransition(-1, 846); // Broken Hand Cart
		trans.autoDecaySeconds = -2;

		var trans = TransitionImporter.GetTransition(-1, 330); // TIME + Hot Steel Ingot on Flat Rock
		trans.autoDecaySeconds = 20;
		
		var trans = TransitionImporter.GetTransition(-1, 252); // TIME + Bowl of Dough
		trans.autoDecaySeconds = 120;

		//var trans = TransitionImporter.GetTransition(-1, 1135); // TIME + Empty Domestic Gooseberry Bush
		//trans.autoDecaySeconds = 60  * 10;
		var trans = TransitionImporter.GetTransition(-1, 389); // TIME + Dying Gooseberry Bush
		trans.autoDecaySeconds = -2;
		
		var trans = new TransitionData(-1, 1284, 0, 291); // TIME + Cool Flat Rock --> 0 + Flat Rock
		trans.autoDecaySeconds = -2;
		transtions.addTransition("PatchTransitions: ", trans);

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
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(0, 3425, 3425, 0); // Domestic Cow on Rope + 0 = Domestic Cow on Rope * 0
		transtions.addTransition("PatchTransitions: ", trans);

		// Set Max Use Target tranistions
		var trans = TransitionImporter.GetTransition(33, 1176); // Stone + Bowl of Dry Beans
		trans.isTargetMaxUse = true;
		var trans = TransitionImporter.GetTransition(40, 253); // Wild Carrot + Bowl of Gooseberries
		trans.isTargetMaxUse = true;
		var trans = TransitionImporter.GetTransition(181, 253); // Skinned Rabbit + Bowl of Gooseberries
		trans.isTargetMaxUse = true;
		var trans = TransitionImporter.GetTransition(402, 253); // Carrot + Bowl of Gooseberries
		trans.isTargetMaxUse = true;


		// new smithing transitions
		var trans = new TransitionData(1603, 235, 1603, 0); // Stack of Clay Bowls + Clay Bowl --> Stack of Clay Bowls +  0
		trans.reverseUseActor = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 316, 1602, 319); // Stack of Clay Plates + Crucible with Iron and Charcoal --> Stack of Clay Plates +  Unforged Sealed Steel Crucible
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 316, 236, 319); // Stack of Clay Plates + Crucible with Iron and Charcoal --> Clay Plate +  Unforged Sealed Steel Crucible
		trans.lastUseActor = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(236, 322, 1602, 325); // Clay Plate + Forged Steel Crucible --> CStack of Clay Plates + Crucible with Steel
		trans.reverseUseActor = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 322, 1602, 325); // Stack of Clay Plates + Forged Steel Crucible --> CStack of Clay Plates + Crucible with Steel
		trans.reverseUseActor = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		var trans = new TransitionData(1602, 236, 1602, 0); // Stack of Clay Plates + Clay Plate --> CStack of Clay Plates + 0
		trans.reverseUseActor = true;
		trans.tool = true;
		transtions.addTransition("PatchTransitions: ", trans);

		// TODo needs client change
		//var trans = new TransitionData(298, 317, 298, 316); // 298 Basket of Charcoal + 317 Crucible with Iron --> 298 +  316 Crucible with Iron and Charcoal
		//transtions.addTransition("PatchTransitions: ", trans);

		// TODO dont know why this was 2240 Newcomen Hammer instead?
		var trans = TransitionImporter.GetTransition(59, 2245); // Rope + Newcomen Engine without Rope
		trans.newTargetID = 2244; // Newcomen Engine without Shaft;
		
		// Ai should ignore
		// TODO fix Ai craftig if Ai needs two threads for a rope it puts one thread in a bowl and gets it out again
		// this breals making a light pulb for a radio
		var trans = transtions.getTransition(58, 235); // Thread + Clay Bowl
		trans.aiShouldIgnore = true; 
		
		// dont deconstruct tools
		var trans = transtions.getTransition(135, 74); // Flint Chip + Fire Bow Drill
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(560, 74); // Knife + Fire Bow Drill
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(461, 3371); // Bow Saw + Table
		trans.aiShouldIgnore = true; 

		// Forbid some transition to make Kindling
		var trans = transtions.getTransition(71, 67); // Stone Hatchet + Long Straight Shaft
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(334, 67); // Steel Axe + Long Straight Shaft
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(334, 2142); // Steel Axe + Banana Plant
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(334, 2145); // Steel Axe + Empty Banana Plant
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(334, 239); // Steel Axe + Wooden Tongs
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(71, 239); // Stone Hatchet + Wooden Tongs
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(560, 575); // Knife + Domestic Sheep
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(560, 4213); // Knife + Fed Domestic Sheep 4213
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(560, 576); // Knife + Shorn Domestic Sheep
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(560, 541); // Knife + Domestic Mouflon
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(2365, 3966); // 2365 Diesel Engine + 3966 Empty Scrap Box
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(345, 82); // Butt Log + Fire
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(345, 83); // Butt Log + Large Fast Fire
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(345, 3029); // Butt Log + Flash Fire
		trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(135, 151); // Flint Chip + Yew Bow
		trans.aiShouldIgnore = true; 

		// allow shovel again if it is better then sharp stone
		var trans = transtions.getTransition(502, 36); // Shovel + Seeding Wild Carrot
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(502, 404); // Shovel + Wild Carrot
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(502, 804); // Shovel + Burdock
		trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(67, 3065); // Long Straight Shaft + Wooden Slot Box
		trans.aiShouldIgnore = true; // this would give a thread Ai wants

		var trans = transtions.getTransition(0, 2244); // 0 + Newcomen Engine without Shaft
		trans.aiShouldIgnore = true; // Ai would kill for a rope
		
		var trans = transtions.getTransition(33, 127); // Stone + Adobe = 231 Adobe Oven Base
		if(AIAllowBuildOven == false) trans.aiShouldIgnore = true; 
		var trans = transtions.getTransition(127, 237); // Adobe + Adobe Oven = 238 Adobe Kiln
		if(AIAllowBuilKiln == false) trans.aiShouldIgnore = true; 

		var trans = transtions.getTransition(0, 303); // 0 + Forge = Adobe Kiln
		trans.aiShouldIgnore = true; 

		// Stop spread of Dough to get a bowl // TODO allow again for tortilla
		var trans = transtions.getTransition(252, 291); // Bowl of Dough + Flat Rock
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(252, 291,true); // Bowl of Dough + Flat Rock
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 1471); // 0 + Sliced Bread
		trans.aiShouldIgnore = true; // they make a mess to get the plate

		var trans = transtions.getTransition(0, 1471,false,true); // 0 + Sliced Bread
		trans.aiShouldIgnore = true; // they make a mess to get the plate

		// forbid burning stuff
		var trans = transtions.getTransition(516, 82); // Skewered Goose + Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(516, 83); // Skewered Goose + Large Fast Fire
		trans.aiShouldIgnore = true;
		
		var trans = transtions.getTransition(516, 346); // Skewered Goose + Large Slow Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(516, 3029); // Skewered Goose + Flash Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(185, 82); // Skewered Rabbit + Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(185, 83); // Skewered Rabbit + Large Fast Fire
		trans.aiShouldIgnore = true;
		
		var trans = transtions.getTransition(185, 346); // Skewered Rabbit + Large Slow Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(185, 3029); // Skewered Rabbit + Flash Fire
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(107, 279); // Stakes + Empty Wild Gooseberry Bush
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(107, 392); // Stakes + Languishing Domestic Gooseberry Bush
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(139, 1136); // Skewer + Shallow Tilled Row
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(852, 1136); // Weak Skewer + Shallow Tilled Row
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(139, 1138); // Skewer + Fertile Soil
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(852, 1138); // Weak Skewer + Fertile Soil
		trans.aiShouldIgnore = true;

		// Forbid plowing of Soil Pile
		var trans = transtions.getTransition(139, 1101); // Skewer + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(852, 1101); // Weak Skewer + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(850, 1101); // Stone Hoe + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(857, 1101); // Steel Hoe + Fertile Soil Pile
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 253); // 0 + Bowl of Gooseberries
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 253, false, true); // 0 + Bowl of Gooseberries
		trans.aiShouldIgnore = true;

		// let the kindling in the oven
		var trans = transtions.getTransition(0, 247); // 0 + Wood-filled Adobe Oven
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 281); // 0 + Wood-filled Adobe Kiln
		trans.aiShouldIgnore = true;

		// AI tries to empty popcorn to get a bowl
		var trans = transtions.getTransition(192, 1121); // Needle and Thread + Popcorn 
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 3308); // Steel Axe + Marked Pine Wall (corner)
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 3309); // Steel Axe + Marked Pine Wall (vertical)
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 3310); // Steel Axe + Marked Pine Wall (horizontal)
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 1876); // Steel Axe + Languishing Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 1922); // Steel Axe + Dry Fertile Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(334, 1923); // Steel Axe + Wet Fertile Domestic Mango Tree
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 2268); // 0 + Bore Mechanism
		trans.aiShouldIgnore = true;

		// protect smithing TODO allow for smithing or manually instruct
		//var trans = transtions.getTransition(0, 322); // 0 + Forged Steel Crucible
		//trans.aiShouldIgnore = true;

		//var trans = transtions.getTransition(0, 325); // 0 + Crucible with Steel
		//trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 316); // 0 + Crucible with Iron and Charcoal
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 318); // 0 + Crucible with Charcoal
		trans.aiShouldIgnore = true;

		// lime
		var trans = transtions.getTransition(677, 661); // Bowl of Plaster 677  + Stone Pile 661
		trans.aiShouldIgnore = true;

		var trans = transtions.getTransition(0, 675); // 0 + Bowl of Limestone 675 
		trans.aiShouldIgnore = true;
		
		//var trans = transtions.getTransition(235, -1); // 235 Clay Bowl
		//trace('DEBUG: ${trans.getDesciption()}');

		//var trans = transtions.getTransition(253, -1); // Bowl of Gooseberries
		//trace('DEBUG!!: ${trans.getDesciption()}');

		// for debug random outcome transitions
		/*var trans = transtions.getTransition(-1, 1195); // TIME + Blooming Squash Plant 
			trans.autoDecaySeconds = 2;
			transtions.addTransition("PatchTransitions: ", trans);
		 */

		 //var lastUseTransition = TransitionImporter.GetTransition(252, 236, true, false);
		 //trace('Debug PAYETON4171: ${lastUseTransition.getDesciption()}');

		 //var objData = ObjectData.getObjectData(885); // 885 Stone Wall
		 //trace('${objData.name} getInsulation: ${objData.getInsulation()} rvalue: ${objData.rValue}');

		//var trans = TransitionImporter.GetTransition(544, -1, true, false);
		//trace('DEBUG: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(3425, -1, false, false); // Domestic Cow on Rope
		//trace('ON DEATH: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(0, 3948); // Arrow Quiver
		//trace('DEBUG: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(560, 418); // Knife + Wolf
		//trace('DEBUG: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(152, 0); // Bow and Arrow + 0
		//trace('DEBUG: ${trans.getDesciption()}');

		//var objData = ObjectData.getObjectData(887); // Stone Wall
		//trace('${objData.name} isPermanent ${objData.isPermanent()}');

		//var trans = TransitionImporter.GetTransition(660, 673); // Full Bucket of Water Bow and Arrow + Empty Cistern
		//trace('DEBUG!!!: ${trans.getDesciption()}');	

		//var trans = TransitionImporter.GetTransition(382, 1790); // Bowl of Water + Dry Maple Sapling Cutting
		//trace('DEBUG!!!: ${trans.getDesciption()}');	

		//var trans = TransitionImporter.GetTransition(382, 396); // Bowl of Water + Dry Planted Carrots
		//trace('DEBUG!!!: ${trans.getDesciption()}');
		
		//var trans = TransitionImporter.GetTransition(382, 2723); // Bowl of Water + Dry Juniper Sapling
		//trace('DEBUG!!!: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(382, 1042); // Bowl of Water + Dry Planted Rose Seed (RED)
		//trace('DEBUG!!!: ${trans.getDesciption()}');

		//var trans = TransitionImporter.GetTransition(-1, 1873); // TIME + Wet Mango Sapling
		//trace('DEBUG!!!: ${trans.getDesciption()}');
		
		//<-1> + <3132> = <0> + <3131> / TIME + Running Diesel Mining Pick#with Iron  -->  EMPTY + Diesel Mining Pick with Iron
		//var trans = TransitionImporter.GetTransition(-1, 3132);
		//trace('DEBUG!!!: ${trans.getDesciption(false)}');

		//var trans = TransitionImporter.GetTransition(283, -1); // Wooden Tongs with Fired Bowl
		//trace('DEBUG!!!: ${trans.getDesciption()}');
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