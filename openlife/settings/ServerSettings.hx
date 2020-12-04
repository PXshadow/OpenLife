package openlife.settings;

class ServerSettings
{
    // for debugging
    public static var debug = true; // activates or deactivates try catch blocks and initial debug objects generation 
    public static var traceTransitionById = 99972; // TransitionImporter
    public static var traceTransitionByTargetDescription = "!!!Gooseberry Bush"; // TransitionImporter

    public static var traceAmountGeneratedObjects = false; // WorldMap

    // real stuff
    public static var startingGx = 360;
    public static var startingGy = 600 - 400; // server map is saved y inverse 

    public static var maxDistanceToBeConsideredAsClose = 20; // only close players are updated with PU and MX and Movement 

    // for movement
    // TODO FIX this can make jumps if too small / ideally this should be 0 so that the client cannot cheap while moving
    public static var MaxMovementCheatingDistanceBeforeForce = 2; // if client player position is bigger then X, client is forced in PU to use server position 

    // for animal movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for calulating offsprings
    public static var chanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring  
    public static var maxOffspringFactor = 3; // The population can only be at max X times the initial population
}