package openlife.settings;

import openlife.server.Lineage.PrestigeClass;
import openlife.server.TransitionHelper;
import openlife.server.Server;
import sys.FileSystem;
import haxe.macro.Expr.Catch;
import sys.io.File;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.server.Biome.BiomeTag;
import openlife.data.object.ObjectData;

using StringTools;

@:rtti
class ServerSettings
{
    // DEBUG: switch on / off
    public static var dumpOutput = false;
    public static var debug = true; // activates or deactivates try catch blocks and initial debug objects generation // deactivates saving
    public static var saveToDisk = true; 
    public static var AllowDebugCommmands = true; // can create objects with saying "!create ID" / "!create object" "!create object!" with ! indicating that object ends with "object" or test wounds with using "!hit" or "!heal"
    public static var DebugWrite = false; // WordMap writeToDisk
    public static var TraceCountObjects = false; // WorldMap
    public static var DebugSpeed = false; // MovementHelper
    
    // Mutex
    public static var useOneGlobalMutex = false; // if you want to try out if there a problems with mutexes / different threads
    public static var useOnePlayerMutex = true;
    public static var UseBlockingSockets = false;

    // DEBUG: used to trace connection.send commands 
    public static var TraceSendPlayerActions = false;  //  only trace player actions // ignores MX from animal, FX and PU from food / age
    public static var TraceSendNonPlayerActions = false;  //  only trace non player actions // traces only MX from animal, FX and PU from food / age

    // DEBUG: TransitionImporter // for debugging transitions
    public static var traceTransitionByActorId = 99934; // set to object id which you want to debug
    public static var traceTransitionByActorDescription = "!!!Basket of Soil"; // set to object description which you want to debug
    public static var traceTransitionByTargetId = 9992710; // set to object id which you want to debug
    public static var traceTransitionByTargetDescription = "!!!Basket of Soil"; // set to object description which you want to debug

    // DEBUG: Temperature
    public static var DebugTemperature = false;
    public static var TemperatureImpactBelow = 0.5; // take damage and display emote if temperature is below or above X from normal
    public static var TemperatureSpeedImpact = 0.9; // speed * X: double impact if extreme temperature
    
    // score
    public static var BirthPrestigeFactor:Float = 0.4; // TODO set 0.2 if fathers are implemented // on birth your starting prestige is factor X * total prestige 
    public static var AncestorPrestigeFactor:Float = 0.2; // if one dies the ancestors get factor X prestige of the dead
    public static var ScoreFactor:Float = 0.2; // new score influences total score with factor X. 
    public static var DisplayScoreFactor:Float = 1; // if display score multiply with factor X
    public static var OldGraveDecayMali:Float = 10; // prestige mali if bones decay without beeing proper burried
    public static var CursedGraveMali:Float = 2; // prestige mali if bones decay without beeing proper burried

    // message
    public static var SecondsBetweenMessages:Float = 5;

    // coins
    public static var InheritCoinsFactor:Float = 0.8; // on death X coins are inherited
    public static var MaxCoinDecayPerYear:Float = 5; 

    // birth    
    public static var EveOrAdamBirthChance = 0.05; // since each eve gets an adam the true chance is x2
    public static var NewChildExhaustionForMother = 15;
    public static var ChanceForFemaleChild = 0.6;
    public static var ChanceForOtherChildColor = 0.2;
    public static var ChanceForOtherChildColorIfCloseToWrongSpecialBiome = 0.3; // for example Black born in or close to Jungle
    public static var AiMotherBirthMaliForHumanChild = 3; // Means in average an AI mother for an human child is only considered after X children 
    public static var HumanMotherBirthMaliForAiChild = 1; // Means in average a human mother for an ai child is only considered after X children 

    // Graves
    public static var GraveBlockingDistance = 100; // cannot incrante close to blocking graves like bone pile
    public static var CloseGraveSpeedMali:Float = 0.8; // speed maili if close to blocking grave like bone pile 
    public static var CursedGraveTime:Float = 12; // hours a cursed grave continues to exist per curse token  

    // PlayerInstance
    public static var MaxPlayersBeforeStartingAsChild = 0; // -1
    public static var StartingEveAge = 14;  // 14
    public static var StartingFamilyName = "SNOW";
    public static var StartingName = "SPOON";
    public static var EveDamageFactor:Float = 0.5;  
    public static var EveFoodUseFactor:Float = 0.5;  
    public static var AgingSecondsPerYear = 60; // 60
    public static var ReduceAgeNeededToPickupObjects = 3; // reduces the needed age that an item can be picked up. But still it cant be used if age is too low
    public static var MaxAgeForAllowingClothAndPrickupFromOthers = 10;
    public static var MaxChildAgeForBreastFeeding = 6; // also used for considering a child when being attacked
    public static var FoodRestoreFactorWhileFeeding:Float = 10;
    public static var MinAgeFertile = 12;
    public static var MaxAgeFertile = 42;
    
    // save to disk
    public static var TicksBetweenSaving = 200;
    public static var TicksBetweenBackups = 20 * 60 * 60 * 8; // 20 * 60 * 60 * 8 = every 8 hours
    public static var MaxNumberOfBackups = 90;

    public static var MapFileName = "mysteraV1Test.png";  
    public static var SaveDirectory = "SaveFiles";
    public static var OriginalBiomesFileName    =   "OriginalBiomes";   // .bin is added
    public static var CurrentBiomesFileName     =   "CurrentBiomes";    // .bin is added
    public static var CurrentFloorsFileName     =   "CurrentFloors";    // .bin is added
    public static var OriginalObjectsFileName   =   "OriginalObjects";  // .bin is added
    public static var CurrentObjectsFileName    =   "CurrentObjects";   // .bin is added
    public static var CurrentObjHelpersFileName =   "CurrentObjHelper"; // .bin is added
    
    // worldMap
    public static var GenerateMapNew = false;
    public static var ChanceForLuckySpot = 0.1; // chance that during generation an object is lucky and tons more of that are generated close by
    public static var startingGx = 235; // 235; //270; // 360;
    public static var startingGy = 150; //200;//- 400; // server map is saved y inverse 
    public static var CreateGreenBiomeDistance = 5;
   
    // food stuff
    public static var FoodUsePerSecond = 0.10; // 0.2; // 5 sec per pip // normal game has around 0.143 (7 sec) with bad temperature and 0.048 (21 sec) with good 
    public static var FoodReductionPerEating:Float = 1;
    public static var MinAgeToEat = 3; // MinAgeToEat and MinAgeFor putting on cloths on their own
    public static var GrownUpFoodStoreMax = 20; // defaul vanilla: 20
    public static var NewBornFoodStoreMax = 4;
    public static var OldAgeFoodStoreMax = 10;
    public static var DeathWithFoodStoreMax:Float = -0.1; // Death through starvation if food store max reaches below XX 
    public static var IncreasedFoodNeedForChildren = 1.5; // children need XX food is below GrownUpAge
    public static var YumBonus = 3; // First time eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
    public static var YumFoodRestore = 0.8; // XX pipes are restored from a random eaten food. Zero are restored if random food is the current eaten food
    public static var YumNewCravingChance = 0.2; // XX chance that a new random craving is chosen even if there are existing ones
    public static var HasEatenReductionForNextGeneration:Float = 0.2; // used in InheritEatenFoodCounts

    // health
    //public static var MinHealthPerYear = 1; // for calulating aging / speed: MinHealthPerYear * age is reduced from health(yum_mulpiplier)
    public static var HealthFactor = 30; // Changes how much health(yum_mulpiplier) affects speed (From 0.8 to 1.2), aging  (From 0.5 to 2) and MaxFoodStore (From 1.5 to 0.5)
    
    // starving to death
    public static var AgingFactorWhileStarvingToDeath = 0.5; // if starving to death aging is slowed factor XX up to GrownUpAge, otherwise aging is speed up factor XX
    public static var GrownUpAge = 14; // is used for AgingFactorWhileStarvingToDeath and for increase food need for children
    //public static var StarvingToDeathMoveSpeedFactor = 0.75; // reduces speed if stored food is below 0 // TODO calculate % from max health???
    //public static var StarvingToDeathMoveSpeedFactorWhileHealthAboveZero = 0.9; // reduces speed if stored food is below 0 and health / yum multiplier > 0
    public static var FoodStoreMaxReductionWhileStarvingToDeath = 5; // (5) reduces food store max with factor XX for each food below 0

    public static var maxDistanceToBeConsideredAsClose = 100; //20; // only close players are updated with PU and MX and Movement 

    // for movement
    public static var InitialPlayerMoveSpeed:Float = 3.75; //3.75; // in Tiles per Second
    public static var SpeedFactor = 1; // MovementExtender // used to incease or deacrease speed factor X
    public static var MinMovementAgeInSec:Float = 14;
    public static var MinSpeedReductionPerContainedObj = 0.98;
    public static var ExhaustionOnMovementChange:Float = 0.25;
    public static var CloseEnemyWithWeaponSpeedFactor:Float = 0.8;
    
    public static var LetTheClientCheatLittleBitFactor = 1.1; // when considering if the position is reached, allow the client to cheat little bit, so there is no lag
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheat while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 
    public static var ChanceThatAnimalsCanPassBlockingBiome = 0.05;

    // hungry work
    public static var HungryWorkCost = 10;
    public static var HungryWorkToolCostFactor:Float = 1;
    public static var ExhaustionHealing:Float = 1; 
    public static var ExhaustionHealingForMaleFaktor:Float = 1.2; 

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for animal offsprings
    public static var ChanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring.   
    public static var ChanceForAnimalDying = 0.0005; // For each movement there is X chance that the animal dies  
    public static var MaxOffspringFactor = 1; // The population can only be at max X times the initial population

    // world decay / respawm
    public static var WorldTimeParts = 25; // TODO better auto calculate on time used // in each tick 1/XX DoTimeSuff is done for 1/XX part of the map. Map height should be dividable by XX * 10 
    public static var ObjRespawnChance = 0.001; // 0.002; 17 hours // In each 20sec (WorldTimeParts/20 * 10) there is a X chance to generate a new object if number is less then original objects
    public static var ObjDecayChance = 0.0002; // 0.001; (X0.08)
    public static var ObjDecayFactorOnFloor:Float = 0.1;
    public static var ObjDecayFactorForFood:Float = 10;

    // winter / summer
    public static var DebugSeason:Bool = false;
    public static var SeasonDuration = 5; // default: 5 // Season duration like winter in years
    public static var AverageSeasonTemperatureImpact = 0.2; 
    public static var WinterWildFoodDecayChance:Float = 1.5; //1.5; // per Season
    public static var SpringWildFoodRegrowChance:Float = 1; // per Season // use spring and summer
    public static var GrowBackOriginalPlantsFactor:Float = 0.4; // 0.1
    //public static var WinterFildWoodDecayChance = 0.2;

    // Ally
    public static var TimeConfirmNewFollower:Float = 15; // a new follower is confirmed after X seconds

    // combat
    public static var CombatAngryTimeBeforeAttack:Float = 5;
    public static var CombatExhaustionCostPerAttack:Float = 0.25;
    public static var WeaponCoolDownFactor:Float = 0.05;
    public static var MaleDamageFactor:Float = 1.2;
    public static var WeaponCoolDownFactorIfWounding:Float = 0.4;
    //public static var AnimalCoolDownFactorIfWounding:Float = 0.2;
    public static var WeaponDamageFactor:Float = 1;
    public static var WoundDamageFactor:Float = 1;
    public static var CursedDamageFactor:Float = 0.5;
    public static var TargetWoundedDamageFactor:Float = 0.5;
    public static var AllyConsideredClose:Int = 5;
    public static var WoundHealingTimeFactor:Float = 1.5;
    public static var PrestigeCostPerDamageForCloseRelatives:Float = 0.25; // For damaging children, mother, father, brother sister  
    public static var PrestigeCostPerDamageForAlly:Float = 0.5; // For damaging ally  
    public static var PrestigeCostPerDamageForChild:Float = 2;
    public static var AllyStrenghTooLowForPickup:Float = 0; // 0.8

    // AI
    public static var NumberOfAis:Int = 3;
    public static var AiReactionTime:Float = 0.5;
    public static var TimeToAiRebirth:Float = 15;
    public static var AiTotalScoreFactor:Float = 0.2; 
    public static var AiMaxSearchRadius:Int = 32;
    public static var AiMaxSearchIncrement:Int = 35; // 16

    // Debug AI
    public static var DebugAiCraftingObject:Int = 57; 
    
    // iron, tary spot spring cannot respawn or win lottery
    public static function CanObjectRespawn(obj:Int) : Bool
    {
        return (obj != 942 && obj !=3030 && obj != 2285 && obj != 3962 && obj != 503);
    }

    public static function PatchObjectData()
    {
        // allow some smithing on tables // TODO fix time transition for contained obj
        for(obj in ObjectData.importedObjectData)
        {
            if(obj.description.indexOf("+hungryWork") != -1)
            {
                obj.hungryWork = ServerSettings.HungryWorkCost;
            }

            if( obj.description.indexOf("on Flat Rock") != -1 )
            {
                obj.containSize = 2;
                obj.containable = true;
            }

            if( obj.description.indexOf("Well") != -1 || (obj.description.indexOf("Pump") != -1 && obj.description.indexOf("Pumpkin") == -1)
                || obj.description.indexOf("Vein") != -1 || obj.description.indexOf("Mine") != -1 || obj.description.indexOf("Iron Pit") != -1
                || obj.description.indexOf("Drilling") != -1 || obj.description.indexOf("Rig") != -1 || obj.description.indexOf("Ancient") != -1)
            {
                obj.decayFactor = -1;

                //trace('Settings: ${obj.description} ${obj.containSize}');
            }
            
            if(obj.description.indexOf("+owned") != -1) obj.isOwned = true;
            if(obj.description.indexOf("+tempOwned") != -1) obj.isOwned = true;
            if(obj.description.indexOf("+followerOwned") != -1) obj.isOwned = true;

            //if( obj.isOwned) trace('isOwned: ${obj.description}');

            //if(obj.containable) trace('${obj.description} ${obj.containSize}');
        }

        // set hungry work  
        // TODO use tool hungry work factor       
        ObjectData.getObjectData(34).hungryWork = 2 * HungryWorkToolCostFactor; // Sharp Stone
        ObjectData.getObjectData(334).hungryWork = 5 * HungryWorkToolCostFactor; // Steel Axe 
        ObjectData.getObjectData(502).hungryWork = 2 * HungryWorkToolCostFactor; // Shovel // TODO should be cheaper then sharp stone

        ObjectData.getObjectData(496).hungryWork = 10; // Dug Stump
        ObjectData.getObjectData(1011).hungryWork = 5; // Buried Grave
        ObjectData.getObjectData(357).hungryWork = 5; // Bone Pile
        
        ObjectData.getObjectData(213).hungryWork = 5; // Deep Tilled Row
        ObjectData.getObjectData(1136).hungryWork = 5; // Shallow Tilled Row

        //Shallow Well

        
        ObjectData.getObjectData(511).hungryWork = 2; // Pond
        ObjectData.getObjectData(1261).hungryWork = 2; // Canada Goose Pond with Egg
        ObjectData.getObjectData(141).hungryWork = 2;  //Canada Goose Pond
        ObjectData.getObjectData(142).hungryWork = 2; // Canada Goose Pond swimming
        ObjectData.getObjectData(143).hungryWork = 2; // Canada Goose Pond swimming, feather
        ObjectData.getObjectData(662).hungryWork = 2; // Shallow Well
        ObjectData.getObjectData(663).hungryWork = 5; // Deep Well 

        // Skewer

        //ObjectData.getObjectData(496).alternativeTransitionOutcome = 10; // Dug Stump
        

        // dont block walking
        ObjectData.getObjectData(231).blocksWalking = false; // Adobe Oven Base 
        ObjectData.getObjectData(237).blocksWalking = false; // Adobe Oven

        // Change map spawn chances
        ObjectData.getObjectData(769).mapChance *= 5; // Wild Horse
        ObjectData.getObjectData(942).mapChance *= 10; // Muddy Iron Vein
        ObjectData.getObjectData(2135).mapChance /= 4; // Rubber Tree
        ObjectData.getObjectData(530).mapChance /= 2; // Bald Cypress Tree
        ObjectData.getObjectData(121).mapChance *= 5; // Tule Reeds

        
        
        ObjectData.getObjectData(2156).mapChance *= 0.5; // Less UnHappy Mosquitos
        ObjectData.getObjectData(2156).biomes.push(BiomeTag.SWAMP); // Evil Mosquitos now also in Swamp
        
        // More Wolfs needs the world
        ObjectData.getObjectData(418).biomes.push(BiomeTag.YELLOW); // Happy Wolfs now also in Yellow biome :)
        ObjectData.getObjectData(418).biomes.push(BiomeTag.GREEN); // Happy Wolfs now also in Green biome :)
        ObjectData.getObjectData(418).mapChance *= 1.5; // More Happy Wolfs
        ObjectData.getObjectData(418).speedMult *= 1.5; // Boost Wolfs even more :)
        

        ObjectData.getObjectData(769).biomes.push(BiomeTag.GREEN); // Beautiful Horses now also in Green biome :)

        ObjectData.getObjectData(411).speedMult = 0.8; // Fertile Soil Reduced carring speed
        ObjectData.getObjectData(345).speedMult = 0.8; // Butt Log
        ObjectData.getObjectData(126).speedMult = 0.8; // Clay 
        ObjectData.getObjectData(127).speedMult = 0.8; // Adobe
        ObjectData.getObjectData(290).speedMult = 0.8; // Iron Ore
        ObjectData.getObjectData(314).speedMult = 0.8; // Wrought Iron
        ObjectData.getObjectData(326).speedMult = 0.8; // Steel Ingot
        ObjectData.getObjectData(838).mapChance = ObjectData.getObjectData(211).mapChance / 5; // Add some lovely mushrooms  
        ObjectData.getObjectData(838).biomes.push(BiomeTag.GREEN); // Add some lovely mushrooms 

         // nerve horse cart little bit :)
        ObjectData.getObjectData(778).speedMult = 1.50; // Horse-Drawn Cart
        ObjectData.getObjectData(3158).speedMult = 1.60; // Horse-Drawn Tire Cart
        
        ObjectData.getObjectData(484).speedMult = 0.85; // Hand Cart
        ObjectData.getObjectData(861).speedMult = 0.85; // // Old Hand Cart
        ObjectData.getObjectData(2172).speedMult = 0.9; // Hand Cart with Tires
        

        // nerve food
        //ObjectData.getObjectData(2143).foodValue = 6; // banana
        //ObjectData.getObjectData(31).foodValue = 4; // Gooseberry
        //ObjectData.getObjectData(2855).foodValue = 4; // Onion
        //ObjectData.getObjectData(808).foodValue = 4; // Wild Onion

        // boost hunted food
        ObjectData.getObjectData(197).foodValue = 18; // Cooked Rabbit 10 --> 18
        ObjectData.getObjectData(2190).foodValue = 24; // Turkey Slice on Plate 17 --> 24
        ObjectData.getObjectData(1285).foodValue = 15; // Omelette 12 --> 15

        ObjectData.getObjectData(518).useChance = 0.3; // Cooked Goose

        // soil should replace water as most needed ressource 
        ObjectData.getObjectData(624).numUses = 2; // // Composted Soil Uses: 3 Soil (Wheat, Berry, Dung) + water ==> 4 Soil 
        ObjectData.getObjectData(411).useChance = 0.5; // Fertile Soil Pit 9 uses --> 18
        
        // TODO let rows decay from time to time to increase soil need.


        // mark plants that decay and regrow
        // Wild Onion
        ObjectData.getObjectData(805).winterDecayFactor = 1; // Wild Onion 805 --> 808 (harvested)
        ObjectData.getObjectData(805).springRegrowFactor = 1; // Wild Onion 805 --> 808 (harvested)
        ObjectData.getObjectData(808).winterDecayFactor = 2; // Wild Onion 805 --> 808 (harvested)
        ObjectData.getObjectData(808).springRegrowFactor = 0.5; // Wild Onion 805 --> 808 (harvested)
        ObjectData.getObjectData(808).countsOrGrowsAs = 805; // Wild Onion 805 --> 808 (harvested)
        
        // Wild Carrot // TODO let seeds regrow
        ObjectData.getObjectData(36).winterDecayFactor = 1; // Seeding Wild Carrot
        ObjectData.getObjectData(36).springRegrowFactor = 1; // Seeding Wild Carrot
        ObjectData.getObjectData(404).winterDecayFactor = 1; // Wild Carrot wihout Seed
        ObjectData.getObjectData(404).springRegrowFactor = 0.5; // Wild Carrot wihout Seed
        ObjectData.getObjectData(404).countsOrGrowsAs = 36; // Wild Carrot wihout Seed
        ObjectData.getObjectData(40).winterDecayFactor = 2; // Wild Carrot
        ObjectData.getObjectData(40).springRegrowFactor = 0.5; // Wild Carrot / out
        ObjectData.getObjectData(40).countsOrGrowsAs = 36; // Wild Carrot / out
        ObjectData.getObjectData(39).winterDecayFactor = 2; // Dug Wild Carrot
        ObjectData.getObjectData(39).springRegrowFactor = 0.5; // Dug Wild Carrot
        ObjectData.getObjectData(39).countsOrGrowsAs = 36; // Dug Wild Carrot

        // Wild Garlic
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
        ObjectData.getObjectData(806).countsOrGrowsAs = 804; // Dug Burdock
        ObjectData.getObjectData(807).winterDecayFactor = 2; // Burdock Root
        ObjectData.getObjectData(807).springRegrowFactor = 0.5; // Burdock Root
        ObjectData.getObjectData(807).countsOrGrowsAs = 804; // Burdock Root

        // Milkweed
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

        //Sapling
        //ObjectData.getObjectData(136).winterDecayFactor = 0.5; // Sapling
        ObjectData.getObjectData(136).springRegrowFactor = 0.05; // Sapling
        ObjectData.getObjectData(138).winterDecayFactor = 2; // Cut Sapling Skewer
        ObjectData.getObjectData(138).springRegrowFactor = 0.5; // Cut Sapling Skewer
        ObjectData.getObjectData(138).countsOrGrowsAs = 136; // Flowering Milkweed

        // Wild Gooseberry Bush
        ObjectData.getObjectData(30).winterDecayFactor = 1; // 1.5; // Wild Gooseberry Bush
        ObjectData.getObjectData(30).springRegrowFactor = 1; // 1.6 // Wild Gooseberry Bush
        ObjectData.getObjectData(279).springRegrowFactor = 6; //1.8; // Empty Wild Gooseberry Bush
        //ObjectData.getObjectData(279).numUses = ObjectData.getObjectData(30).numUses; // Empty Wild Gooseberry Bush
        ObjectData.getObjectData(31).winterDecayFactor = 2; //Gooseberry

        //Domestic Gooseberry Bush
        ObjectData.getObjectData(391).winterDecayFactor = 1; // Domestic Gooseberry Bush
        ObjectData.getObjectData(391).springRegrowFactor = 0.1; // Domestic Gooseberry Bush

        ObjectData.getObjectData(750).speedMult = 0.75; // Bloody Knife
        ObjectData.getObjectData(3048).speedMult = 0.85; // Bloody War Sword
        ObjectData.getObjectData(749).speedMult = 0.6; // Bloody Yew Bow  
        
        ObjectData.getObjectData(750).neverDrop = true; // Bloody Knife
        ObjectData.getObjectData(3048).neverDrop = true; // Bloody War Sword
        ObjectData.getObjectData(749).neverDrop = true; // Bloody Yew Bow  

        //ObjectData.getObjectData(1378).permanent = 0; // Sterile Wool Pad 

        ObjectData.getObjectData(3047).prestigeClass = PrestigeClass.Noble; // War Sword 

        ObjectData.getObjectData(560).deadlyDistance = 1.5; // Knife  
        ObjectData.getObjectData(3047).deadlyDistance = 1.5; // War Sword 
        ObjectData.getObjectData(152).deadlyDistance = 4; // Bow and Arrow 
        ObjectData.getObjectData(1624).deadlyDistance = 4; // Bow and Arrow with Note 

        ObjectData.getObjectData(750).deadlyDistance = 1.5; // Bloody Knife
        ObjectData.getObjectData(3048).deadlyDistance = 1.5; // Bloody War Sword
        ObjectData.getObjectData(749).deadlyDistance = 4; // Bloody Yew Bow  

        // TODO allow damage with bloody weapon / needs support from client?
        ObjectData.getObjectData(560).damage = 4; // Knife  // damage per sec = 2
        ObjectData.getObjectData(3047).damage = 6; // War Sword // damage per sec = 3
        ObjectData.getObjectData(152).damage = 6; // Bow and Arrow  // 
        ObjectData.getObjectData(1624).damage = 10; // Bow and Arrow with Note  //

        // TODO Mouflon? // TODO moskito
        ObjectData.getObjectData(418).deadlyDistance = 1.5; // Wolfs 
        ObjectData.getObjectData(418).damage = 3.5;  // Wolfs 
        ObjectData.getObjectData(420).deadlyDistance = 1.5; // Shot Wolf
        ObjectData.getObjectData(420).damage = 5;  // Shot Wolf

        ObjectData.getObjectData(764).deadlyDistance = 1.1; // Rattle Snake
        ObjectData.getObjectData(764).damage = 2;  //  Rattle Snake
        ObjectData.getObjectData(764).woundFactor = 0.98;  //  Rattle Snake

        ObjectData.getObjectData(1323).deadlyDistance = 1.5; // Wild Boar 
        ObjectData.getObjectData(1323).damage = 3;  // Wild Boar 
        ObjectData.getObjectData(1328).deadlyDistance = 1.5; // Wild Boar with Piglet 
        ObjectData.getObjectData(1328).damage = 5;  // Wild Boar with Piglet 

        ObjectData.getObjectData(628).deadlyDistance = 1.5; // Grizzly Bear
        ObjectData.getObjectData(628).damage = 5;  // Grizzly Bear 
        ObjectData.getObjectData(632).deadlyDistance = 1.5; // Shot Grizzly Bear 1 
        ObjectData.getObjectData(632).damage = 6;  // Shot Grizzly Bear 1 
        ObjectData.getObjectData(635).deadlyDistance = 1.5; // Shot Grizzly Bear 2 
        ObjectData.getObjectData(635).damage = 7;  // Shot Grizzly Bear 2 
        ObjectData.getObjectData(637).deadlyDistance = 1.5; // Shot Grizzly Bear 3 
        ObjectData.getObjectData(637).damage = 8;  // Shot Grizzly Bear 3

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

        ObjectData.getObjectData(1377).damage = 0.25; // per sec Snake Bite
        ObjectData.getObjectData(1384).damage = 0.2; // per sec Clean Snake Bite
       
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

        for(objData in ObjectData.importedObjectData)
        {
            if(objData.description.contains('Sports Car'))
            {
                objData.isBoat = true;
                objData.speedMult = 3.5;
            }
        }

        //ObjectData.getObjectData(279).winterDecayFactor = 2; // Empty Wild Gooseberry Bush
        //ObjectData.getObjectData(279).springRegrowFactor = 0.5; // Empty Wild Gooseberry Bush
        //ObjectData.getObjectData(279).countsOrGrowsAs = 30; // Empty Wild Gooseberry Bush
        
        //var obj = ObjectData.getObjectData(624);

        //trace('${obj.description} uses: ${obj.numUses} chance: ${obj.useChance}');

        
        //var obj = ObjectData.getObjectData(411);

        //trace('${obj.description} uses: ${obj.numUses} chance: ${obj.useChance}');




        //trace('Insulation: ${ObjectData.getObjectData(128).getInsulation()}'); // Redd Skirt

        //ObjectData.getObjectData(31).writeToFile();

        //trace('Trace: ${ObjectData.getObjectData(8881).description}');
         
        
        
        //trace('Patch: ${ObjectData.getObjectData(942).description}');
        //if (obj.deadlyDistance > 0)
        //    obj.mapChance *= 0;  

        //trace('Permanent: ${ObjectData.getObjectData(750).permanent}');
        //trace('Permanent: ${ObjectData.getObjectData(3816).permanent}');
        
    }

    public static function PatchTransitions(transtions:TransitionImporter)        
    {   
        // TODO set through transions
        ObjectData.getObjectData(30).lastUseObject = 279; // Wild Gooseberry Bush ==> Empty Wild Gooseberry Bush
        ObjectData.getObjectData(279).undoLastUseObject = 30; // Empty Wild Gooseberry Bush ==> Wild Gooseberry Bush 

        var trans = transtions.getTransition(-1, 750); // Bloody Knife
        trans.autoDecaySeconds = 50; 
        trans.traceTransition("PatchTransitions: "); 

        var trans = transtions.getTransition(-1, 3048); // Bloody War Sword
        trans.autoDecaySeconds = 50; 
        trans.traceTransition("PatchTransitions: "); 

        var trans = transtions.getTransition(-1, 749); // Bloody Yew Bow  
        trans.autoDecaySeconds = 80; 
        trans.traceTransition("PatchTransitions: "); 

        var trans = transtions.getTransition(-1, 427); // Attacking Wolf
        trans.autoDecaySeconds = 3; 
        trans.move = 5;
        var trans = transtions.getTransition(-1, 428); // Attacking Shot Wolf
        trans.autoDecaySeconds = 3; 
        trans.move = 4;

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
        trans = new TransitionData(-1,797,0,0); // Stable Knife Wound --> Empty // on ground
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);
        trans = new TransitionData(-1,1380,0,0); // Clean Knife Wound --> 0
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);

        ObjectData.getObjectData(1363).alternativeTimeOutcome = 1381; // Bite Wound --> Clean Bite Wound 
        trans = new TransitionData(-1,1363,0,0); //  Bite Wound --> Empty
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);
        trans = new TransitionData(-1,1381,0,0); // Clean Bite Wound --> 0
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);

        ObjectData.getObjectData(1377).alternativeTimeOutcome = 1384; // Snake Bite -->  Clean Snake Bite
        trans = new TransitionData(-1,1377,0,0); //  Snake Bite --> Empty
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);
        trans = new TransitionData(-1,1384,0,0); //  Clean Snake Bite --> 0
        trans.autoDecaySeconds = 120 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);

        ObjectData.getObjectData(1366).alternativeTimeOutcome = 1383; // Hog Cut --> Clean Hog Cut
        trans = new TransitionData(-1,1364,0,0); // Hog Cut --> Empty
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);
        trans = new TransitionData(-1,1383,0,0); // Clean Hog Cut --> 0
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans); 

        ObjectData.getObjectData(1366).alternativeTimeOutcome = 1382; // Empty Arrow Wound --> Clean Arrow Wound
        trans = new TransitionData(-1,1366,0,0); // Empty Arrow Wound --> Empty
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);
        trans = new TransitionData(-1,1382,0,0); // Clean Arrow Wound --> 0
        trans.autoDecaySeconds = 30 * WoundHealingTimeFactor;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = transtions.getTransition(0, 798); 
        ObjectData.getObjectData(798).alternativeTimeOutcome = trans.newTargetID; // Arrow Wound --> Embedded Arrowhead Wound
        trans.newTargetID = 1367; // Arrow Wound --> Extracted Arrowhead Wound 
        transtions.addTransition("PatchTransitions: ", trans);
        
        trans = transtions.getTransition(0, 1367); 
        ObjectData.getObjectData(1367).alternativeTimeOutcome = trans.newTargetID; // Extracted Arrowhead Wound --> Gushing Empty Arrow Wound
        trans.newTargetID = 1366; // Extracted Arrowhead Wound --> Empty Arrow Wound
        transtions.addTransition("PatchTransitions: ", trans);

        for(trans in TransitionImporter.transitionImporter.transitions)
        {
            if(trans.actorID < -1) 
            {
                // trans.traceTransition("PatchTransitions: ", true);

                trans.actorID = 0; 
                transtions.addTransition("PatchTransitions: ", trans);
                trans.traceTransition("PatchTransitions: ");
            }

            if(trans.autoDecaySeconds == -168)
            {
                trans.autoDecaySeconds = -2; // use chance 20%: 10 uses / 120 min 0.08 Bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }

            if(trans.autoDecaySeconds == 9000) // 150 min like deep well 0.53 bowls per min
            {
                trans.autoDecaySeconds = 1200; // 20 min one bucket 12.5% (bucket has 10 uses): 80 uses / 20 min = 4 bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }

            if(trans.autoDecaySeconds == 2160) // 36 min like well  0.91 bowls per min
            {
                trans.autoDecaySeconds = 720; // 12 min one bowl 3%: 33 uses / 12 min = 2.7 bowls per min

                //trans.traceTransition("PatchTransitions: ", true);    
            }
        }

        // Original: Riding Horse: 770 + -1 = 0 + 1421
        trans = new TransitionData(770,0,0,1421);
        transtions.addTransition("PatchTransitions: ", trans);

        // TODO this should function somehow with categories???
        // original transition makes cart loose rubber if putting down horse cart
        //Original: 3158 + -1 = 0 + 1422 // Horse-Drawn Tire Cart + ???  -->  Empty + Escaped Horse-Drawn Cart --> must be: 3158 + -1 = 0 + 3161
        trans = transtions.getTransition(3158, -1);
        trans.newTargetID = 3161;
        trans.traceTransition("PatchTransitions: ");

         // original transition makes cart loose rubber if picking up horse cart
        //Original:  0 + 3161 = 778 + 0 //Empty + Escaped Horse-Drawn Tire Cart# just released -->  Horse-Drawn Cart + Empty
        trans = transtions.getTransition(0, 3161);
        trans.newActorID = 3158;
        trans.traceTransition("PatchTransitions: ");

        trans = transtions.getTransition(-1, 3161);
        trans.newTargetID = 3157;
        trans.traceTransition("PatchTransitions: ");

        trans = transtions.getTransition(0, 3157);
        trans.newActorID = 3158;
        trans.traceTransition("PatchTransitions: ");

        // let Tule Stumps (122) grow back
        trans = transtions.getTransition(-1, 122); 
        trans.newTargetID = 121; // 121 = Tule Reeds
        trans.traceTransition("PatchTransitions: ");

        

        // 141 Canada Goose Pond
        //1261 Canada Goose Pond with Egg // TODO let egg come back
        //Server.transitionImporter.


        // change decay time for grave 88 = Grave
        //trans = transtions.getTransition(-1, 88); 
        //trans.autoDecaySeconds = 10; 
        //trans.traceTransition("PatchTransitions: ");

        // should be fixed now with the rest of the -2 transitions
        //-2 + 141 = 0 + 143 // some how we have -2 transactions like hand + ghoose pond = ghoose pond with feathers 
        //trans = transtions.getTransition(-2, 141); 
        //trans.actorID = 0; 
        //transtions.addTransition("PatchTransitions: ", trans);
        //trans.traceTransition("PatchTransitions: ");
        

        // let get berrys back!
        trans = new TransitionData(-1,30,0,30); // Wild Gooseberry Bush
        
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 600;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(-1,279,0,30); // Empty Wild Gooseberry Bush --> // Wild Gooseberry Bush
        trans.reverseUseTarget = true; 
        trans.autoDecaySeconds = 600; 
        transtions.addTransition("PatchTransitions: ", trans);

        // let get bana back!
        trans = new TransitionData(-1,2142,0,2142); // Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(-1,2145,0,2142); // Empty Banana Plant --> Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000; 
        transtions.addTransition("PatchTransitions: ", trans);

        //  Wild Gooseberry Bush
        trans = new TransitionData(253,30,253,30); // Bowl of Gooseberries + Wild Gooseberry Bush --> Bowl of Gooseberries(+1) + Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(235,30,253,30); // Clay Bowl + Wild Gooseberry Bush --> Bowl of Gooseberries + Wild Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(253,30,253,279); // Bowl of Gooseberries + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans, false, true);

        trans = new TransitionData(235,30,253,279); // Clay Bowl + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Wild Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans, false, true);


        // Domestic Gooseberry Bush
        trans = new TransitionData(253,391,253,391); // Bowl of Gooseberries + Domestic Gooseberry Bush --> Bowl of Gooseberries(+1) + Domestic Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(235,391,253,391); // Clay Bowl + Domestic Gooseberry Bush --> Bowl of Gooseberries + Domestic Gooseberry Bush
        transtions.addTransition("PatchTransitions: ", trans);

        trans = new TransitionData(253,391,253,1135); // Bowl of Gooseberries + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Domestic Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition("PatchTransitions: ", trans, false, true);

        trans = new TransitionData(235,391,253,1135); // Clay Bowl + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Domestic Gooseberry  Bush
        transtions.addTransition("PatchTransitions: ", trans, false, true);


        // give wolfs some meat
        trans = transtions.getTransition(0, 423); // 423 Skinned Wolf 
        trans.newTargetID = 565;  // 565 Butchered Mouflon
        trans.traceTransition("PatchTransitions: "); 

        // allow to cook mutton on cloals
        trans = new TransitionData(569,85,570,85); // 569 Raw Mutton + 85 Hot Coals --> 570 Cooked Mutton + 85 Hot Coals
        transtions.addTransition("PatchTransitions: ", trans);

        // patch alternativeTransitionOutcomes
        
        var trans = transtions.getTransition(502, 338); // shovel plus Stump
        trans.alternativeTransitionOutcome.push(72); // Kindling
         
        ObjectData.getObjectData(342).alternativeTransitionOutcome.push(344); // Chopped Tree Big Log--> Fire Wood
        ObjectData.getObjectData(340).alternativeTransitionOutcome.push(344); // Chopped Tree --> Fire Wood
    }

    public static function writeToFile()
    {
        var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
        var dir = './${ServerSettings.SaveDirectory}/';
        var path = dir + "ServerSettings.txt";

        if(FileSystem.exists(dir) == false) FileSystem.createDirectory(dir);

        var writer = File.write(path, false);

        writer.writeString('Remove **default** if you dont want to use default value!\n');

        var count = 0;

        for (field in rtti.statics)
        {
            if('$field'.indexOf('CFunction') != -1 ) continue;
            count++;
            var value:Dynamic = Reflect.field(ServerSettings, field.name);

            //trace('ServerSettings: $count ${field.name} ${field.type} $value');
            
            if('${field.type}' == "CClass(String,[])")
            {
                writer.writeString('**default** ${field.name} = "$value"\n');
            }
            else
            {
                writer.writeString('**default** ${field.name} = $value\n');
            }
        }

        writer.writeString('**END**\n');

        writer.close();
    }

    public static function readFromFile(traceit:Bool = true) : Bool
    {
        var reader = null;

        try{
            //var rtti = haxe.rtti.Rtti.getRtti(ServerSettings);
            var dir = './${ServerSettings.SaveDirectory}/';
            reader = File.read(dir + "ServerSettings.txt", false);

            reader.readLine();

            var line = "";

            while(line.indexOf('**END**') == -1 )
            {
                line = reader.readLine();

                //trace('Read: ${line}');

                if(line.indexOf('**default**') != -1 ) continue;           

                var splitLine = line.split("=");                

                if(splitLine.length < 2) continue;

                splitLine[0] = StringTools.replace(splitLine[0], ' ', '');
                //splitLine[1] = StringTools.replace(splitLine[1], '\n', '');

                if(traceit) trace('Load Setting: ${splitLine[0]} = ${splitLine[1]}');

                var fieldName = splitLine[0];
                var value:Dynamic = splitLine[1];

                if(splitLine[1].indexOf('"') != -1 )
                {
                    var splitString = splitLine[1].split('"');

                    if(splitString.length < 3) continue;

                    value = splitString[1]; 
                }
                else 
                {
                    value = StringTools.replace(value, 'true', '1');
                    value = StringTools.replace(value, 'false', '0');
                    value = Std.parseFloat(value);
                }

                var oldValue:Dynamic = Reflect.field(ServerSettings, fieldName);

                Reflect.setField(ServerSettings, fieldName, value);

                var newValue:Dynamic = Reflect.field(ServerSettings, fieldName);

                if('$newValue' != '$oldValue') trace('Setting changed: ${fieldName} = ${newValue} // old value: $oldValue');
            }
        }
        catch(ex)
        {
            if(reader != null) reader.close();

            trace(ex);

            return false;
        }

        return true;

        //trace('Read Test: traceTransitionByTargetDescription: $traceTransitionByTargetDescription');
        //trace('Read Test: YumBonus: $YumBonus');
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