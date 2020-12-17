package openlife.settings;

class ServerSettings
{
    // for debugging
    public static var debug = true; // activates or deactivates try catch blocks and initial debug objects generation 
    
    public static var TraceSend = true; // used to trace connection.send commands
    public static var TraceOnlyPlayerActions = true; // only trace player actions / ignores MX from animal, FX and PU from food / age

    public static var traceTransitionById = 99972; // TransitionImporter
    public static var traceTransitionByActorDescription = "!!!Bowl of Water"; // TransitionImporter
    public static var traceTransitionByTargetDescription = "!!!Steel Axe"; // TransitionImporter

    public static var traceAmountGeneratedObjects = false; // WorldMap
   
    // food stuff
    public static var MinAgeToEat = 3;
    public static var FoodUsePerSecond = 0.2; // 0.1;
    public static var GrownUpFood = 20;

    // PlayerInstance
    public static var StartingEveAge = 12;  // 12
    public static var SecondsPerYear = 8; // 60

    public static var startingGx = 360;
    public static var startingGy = 600 - 400; // server map is saved y inverse 

    public static var maxDistanceToBeConsideredAsClose = 15; // only close players are updated with PU and MX and Movement 

    // for movement
    public static var SpeedFactor = 2; // MovementExtender // used to incease or deacrease speed
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheap while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for animal offsprings
    public static var chanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring  
    public static var maxOffspringFactor = 3; // The population can only be at max X times the initial population
}