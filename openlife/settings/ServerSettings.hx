package openlife.settings;

class ServerSettings
{
    // for debugging
    public static var debug = true; // activates or deactivates try catch blocks and initial debug objects generation 
    
    // used to trace connection.send commands //  only trace player actions // ignores MX from animal, FX and PU from food / age
    public static var TraceSendPlayerActions = true; 
    // used to trace connection.send commands //  only trace non player actions // traces only MX from animal, FX and PU from food / age
    public static var TraceSendNonPlayerActions = false;  

    public static var traceTransitionById = 99972; // TransitionImporter
    public static var traceTransitionByActorDescription = "!!!Riding Horse"; // TransitionImporter
    public static var traceTransitionByTargetDescription = "!!!Steel Axe"; // TransitionImporter

    public static var traceAmountGeneratedObjects = false; // WorldMap
   
    // food stuff
    public static var MinAgeToEat = 3;
    public static var FoodUsePerSecond = 0.2; // 0.2;
    public static var GrownUpFoodStoreMax = 30;
    public static var NewBornFoodStoreMax = 4;
    public static var OldAgeFoodStoreMax = 10;
    public static var DeathWithFoodStoreMax = 0; // Death through starvation if food store max reaches below XX 
    public static var YumBonus = 3; // First team eaten you get XX yum boni, reduced one per eating. Food ist not yum after eating XX
    public static var IncreasedFoodNeedForChildren = 2; // children need XX food is below GrownUpAge

    // PlayerInstance
    public static var StartingEveAge = 3;  // 13
    public static var SecondsPerYear = 10; // 60
    
    // starving to death
    public static var AgingFactorWhileStarvingToDeath = 0.2; // if starving to death aging is slowed factor XX up to GrownUpAge, otherwise aging is speed up factor XX
    public static var GrownUpAge = 14; // is used for AgingFactorWhileStarvingToDeath and for increase food need for children
    public static var StarvingToDeathMoveSpeedFactor = 0.5; // reduces speed if stored food is below 0
    public static var FoodStoreMaxReductionWhileStarvingToDeath = 2; // reduces food store max with factor XX for each food below 0

    public static var startingGx = 360;
    public static var startingGy = 600 - 400; // server map is saved y inverse 

    public static var maxDistanceToBeConsideredAsClose = 20; // only close players are updated with PU and MX and Movement 

    // for movement
    public static var InitialPlayerMoveSpeed = 3.75; // in Tiles per Second
    public static var SpeedFactor = 1; // MovementExtender // used to incease or deacrease speed factor X
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheap while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for animal offsprings
    public static var chanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring  
    public static var maxOffspringFactor = 3; // The population can only be at max X times the initial population
}