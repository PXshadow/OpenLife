package openlife.settings;

class ServerSettings
{
    public static var debug = true;
    public static var traceTransitionById = 1599;
    public static var traceTransitionByTargetDescription = "Kindling";

    public static var startingGx = 360;
    public static var startingGy = 600 - 400; // server map is saved y inverse 

    // for movement
    public static var chancePreferredBiome = 0.8; // Chance that the animal ignors the chosen target if its not from his original biome
    
    // for calulating offsprings
    public static var chanceForOffspring = 0.001; // For each movement there is X chance to generate an offspring  
    public static var maxOffspringFactor = 3; // The population can only be at max X times the initial population
}