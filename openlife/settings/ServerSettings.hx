package openlife.settings;

import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.server.WorldMap.BiomeTag;
import openlife.data.object.ObjectData;

class ServerSettings
{
    // for debugging
    public static var debug = true; // activates or deactivates try catch blocks and initial debug objects generation 
    
    // used to trace connection.send commands //  only trace player actions // ignores MX from animal, FX and PU from food / age
    public static var TraceSendPlayerActions = true; 
    // used to trace connection.send commands //  only trace non player actions // traces only MX from animal, FX and PU from food / age
    public static var TraceSendNonPlayerActions = false;  

    public static var traceTransitionById = 99972; // TransitionImporter
    public static var traceTransitionByActorDescription = "!!!Bowl of Stew"; // TransitionImporter
    public static var traceTransitionByTargetDescription = "!!!Banana Plant"; // TransitionImporter

    public static var traceAmountGeneratedObjects = false; // WorldMap

    // PlayerInstance
    public static var StartingEveAge = 10;  // 13
    public static var AgingSecondsPerYear = 60; // 60

    // worldMap
    public static var ChanceForLuckySpot = 0.03; // chance that during generation an object is lucky and tons more of that are generated close by
    public static var MapFileName = "mysteraV1Test.png";    
    public static var startingGx = 235; //270; // 360;
    public static var startingGy = 150; //200;//- 400; // server map is saved y inverse 
    public static var CreateGreenBiomeDistance = 5;
   
    // food stuff
    public static var WorldTimeParts = 40; // in each tick 1/40 DoTimeSuff is done for 1/XX part of the map. Map height should be dividable by XX
    public static var MinAgeToEat = 3;
    public static var FoodUsePerSecond = 0.2; // 0.2; // 5 sec per pip // use 0.6 or something to test cravings...
    public static var GrownUpFoodStoreMax = 20;
    public static var NewBornFoodStoreMax = 4;
    public static var OldAgeFoodStoreMax = 10;
    public static var DeathWithFoodStoreMax = 0; // Death through starvation if food store max reaches below XX 
    public static var IncreasedFoodNeedForChildren = 2; // children need XX food is below GrownUpAge
    public static var YumBonus = 3; // First team eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
    public static var YumFoodRestore = 0.8; // XX pipes are restored from a random eaten food. Zero are restored if random food is the current eaten food
    public static var YumNewCravingChance = 0.2; // XX chance that a new random craving is chosen even if there are existing ones

    // health
    public static var HealthFactor = 40; // Changes how much health / yum_mulpiplier affects speed and aging. (From 0.5 to 1.2)  
    public static var MinHealthPerYear = 2; // For calculation Aging starts with a yum_mulpiplier of -20
    //if(health >= 0) healthFactor = (1.2  * health + ServerSettings.HealthFactor) / (health + ServerSettings.HealthFactor);
    //else healthFactor = (health - ServerSettings.HealthFactor) / ( 2 * health - ServerSettings.HealthFactor);
    
    // starving to death
    public static var AgingFactorWhileStarvingToDeath = 0.5; // if starving to death aging is slowed factor XX up to GrownUpAge, otherwise aging is speed up factor XX
    public static var GrownUpAge = 14; // is used for AgingFactorWhileStarvingToDeath and for increase food need for children
    public static var StarvingToDeathMoveSpeedFactor = 0.5; // reduces speed if stored food is below 0
    public static var FoodStoreMaxReductionWhileStarvingToDeath = 2; // reduces food store max with factor XX for each food below 0

    public static var maxDistanceToBeConsideredAsClose = 20; // only close players are updated with PU and MX and Movement 

    // for movement
    public static var InitialPlayerMoveSpeed:Float = 3.75; //3.75; // in Tiles per Second
    public static var SpeedFactor = 1; // MovementExtender // used to incease or deacrease speed factor X
    public static var MinSpeedReductionPerContainedObj = 0.96;
    
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheap while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 
    public static var ChanceThatAnimalsCanPassBlockingBiome = 0.05;

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for animal offsprings
    public static var chanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring  
    public static var maxOffspringFactor = 3; // The population can only be at max X times the initial population


    public static function PatchObjectData()
    {
        // Change map spawn chances
        ObjectData.getObjectData(942).mapChance *= 1.5; // Muddy Iron Vein
        ObjectData.getObjectData(2135).mapChance /= 3; // Rubber Tree
        ObjectData.getObjectData(2156).mapChance *= 0.5; // Less UnHappy Mosquitos
        
        ObjectData.getObjectData(418).biomes.push(BiomeTag.YELLOW); // Happy Wolfs now also in Yellow biome :)
        ObjectData.getObjectData(418).biomes.push(BiomeTag.GREEN); // Happy Wolfs now also in Green biome :)
        ObjectData.getObjectData(418).mapChance *= 1.5; // More Happy Wolfs

        ObjectData.getObjectData(418).speedMult = 1.5; // Boost Wolfs even more :)

        ObjectData.getObjectData(290).speedMult = 0.80; // Iron Ore
        ObjectData.getObjectData(838).mapChance = ObjectData.getObjectData(211).mapChance / 5; // Add some lovely mushrooms  
        ObjectData.getObjectData(838).biomes.push(BiomeTag.GREEN); // Add some lovely mushrooms 

         // horse cart little bit :)
        ObjectData.getObjectData(778).speedMult = 1.50; // Horse-Drawn Cart
        ObjectData.getObjectData(3158).speedMult = 1.60; // Horse-Drawn Tire Cart
        
        ObjectData.getObjectData(484).speedMult = 0.85; // Hand Cart
        ObjectData.getObjectData(861).speedMult = 0.85; // // Old Hand Cart
        ObjectData.getObjectData(2172).speedMult = 0.9; // Hand Cart with Tires


        //trace('Trace: ${ObjectData.getObjectData(8881).description}');
         
        
        //ObjectData.getObjectData(2143).foodValue = 1; // banana
        //ObjectData.getObjectData(31).foodValue = 1; // Gooseberry

        //trace('Patch: ${ObjectData.getObjectData(942).description}');
        //if (obj.deadlyDistance > 0)
        //    obj.mapChance *= 0;  
    }

    public static function PatchTransitions(transtions:TransitionImporter)
    {
        // let get berrys back!
        var trans = new TransitionData(-1,30,0,30); // Wild Gooseberry Bush
        
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 600;
        transtions.addTransition(trans);

        trans = new TransitionData(-1,279,0,30); // Empty Wild Gooseberry Bush --> // Wild Gooseberry Bush
        trans.reverseUseTarget = true; 
        trans.autoDecaySeconds = 600; 
        transtions.addTransition(trans);

        // let get bana back!
        trans = new TransitionData(-1,2142,0,2142); // Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000;
        transtions.addTransition(trans);

        trans = new TransitionData(-1,2145,0,2142); // Empty Banana Plant --> Banana Plant
        trans.reverseUseTarget = true;
        trans.autoDecaySeconds = 1000; 
        transtions.addTransition(trans);

        //  Wild Gooseberry Bush
        trans = new TransitionData(253,30,253,30); // Bowl of Gooseberries + Wild Gooseberry Bush --> Bowl of Gooseberries(+1) + Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition(trans);

        trans = new TransitionData(235,30,253,30); // Clay Bowl + Wild Gooseberry Bush --> Bowl of Gooseberries + Wild Gooseberry Bush
        transtions.addTransition(trans);

        trans = new TransitionData(253,30,253,279); // Bowl of Gooseberries + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition(trans, false, true);

        trans = new TransitionData(235,30,253,279); // Clay Bowl + Wild Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Wild Gooseberry Bush
        transtions.addTransition(trans, false, true);


        // Domestic Gooseberry Bush
        trans = new TransitionData(253,391,253,391); // Bowl of Gooseberries + Domestic Gooseberry Bush --> Bowl of Gooseberries(+1) + Domestic Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition(trans);

        trans = new TransitionData(235,391,253,391); // Clay Bowl + Domestic Gooseberry Bush --> Bowl of Gooseberries + Domestic Gooseberry Bush
        transtions.addTransition(trans);

        trans = new TransitionData(253,391,253,1135); // Bowl of Gooseberries + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries(+1) + Empty Domestic Wild Gooseberry Bush
        trans.reverseUseActor = true;
        transtions.addTransition(trans, false, true);

        trans = new TransitionData(235,391,253,1135); // Clay Bowl + Domestic Gooseberry Bush (Last) --> Bowl of Gooseberries + Empty Domestic Gooseberry  Bush
        transtions.addTransition(trans, false, true);
        
    }
}