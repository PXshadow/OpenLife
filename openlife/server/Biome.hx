package openlife.server;

@:enum abstract BiomeTag(Int) from Int to Int
{
    public var GREEN = 0;
    public var SWAMP = 1;
    public var YELLOW = 2;
    public var GREY = 3;
    public var SNOW = 4;
    public var DESERT= 5;
    public var JUNGLE = 6; //6 
    public var BORDERJUNGLE = 15; // 8 or 15  

    public var SNOWINGREY = 21; //7 // its snow on top of mountains which should not be walkable
    public var OCEAN = 9;  //deep ocean
    public var PASSABLERIVER = 13;
    public var RIVER = 17;  // TODO deep river which is not walkable 
}

@:enum abstract BiomeMapColor(String) from String to String
{
    public var CGREEN = "FFB5E61D";  
    public var CSWAMP = "FF008080";  
    public var CYELLOW = "FFFECC36"; //savannah
    public var CGREY = "FF808080"; //badlands // bevor it was: FF404040
    public var CSNOW = "FFFFFFFF";
    public var CDESERT= "FFDBAC4D"; 
    public var CJUNGLE = "FF007F0E";
    public var CBORDERJUNGLE = "FF007F00";  
    
    public var CSAND = "FFefe4b0";

    public var CSNOWINGREY = "FF404040"; // its snow on top of mountains which should not be walkable
    public var COCEAN = "FF004080"; //deep ocean 
    public var CRIVER = "FF0080FF"; //shallow water
    public var CPASSABLERIVER = "FF00E8FF"; // TODO use also for passable ocean? otherwise use biomeID: 22??? 
}

@:enum abstract BiomeSpeed(Float) from Float to Float
{
    // var truncMovementSpeedDiff = 0.1;
    // considered as bad biome for horses if speed < 0.999
    // TODO make fast for specialists 
    public var SGREEN = 1;  
    public var SSWAMP = 0.6;  
    public var SYELLOW = 1;
    public var SGREY = 0.98; 
    public var SSNOW = 0.98; 
    public var SDESERT= 0.98; 
    public var SJUNGLE = 0.98;  
    public var SCBORDERJUNGLE = 0.98; 

    public var SSNOWINGREY = 0.01;
    public var SOCEAN = 0.01;  
    public var SRIVER = 0.01;
    public var SPASSABLERIVER = 0.6;   
}

@:enum abstract BiomeTemperature(Float) from Float to Float
{
    // var truncMovementSpeedDiff = 0.1;
    // considered as bad biome for horses if speed < 0.999
    // TODO make fast for specialists 
    public var TGREEN = 1;  
    public var TSWAMP = 0.5;  
    public var TYELLOW = 0.8;
    public var TGREY = 0.25; 
    public var TSNOW = 0; 
    public var TDESERT= 2; // perfect for black
    public var TJUNGLE = 1.5;  // perfect for brown
    public var TCBORDERJUNGLE = 1.5; // perfect for brown

    public var TSNOWINGREY = 0.0; 
    public var TOCEAN = 0.0;  
    public var TRIVER = 0.0;
    public var TPASSABLERIVER = 0.2;   
}

class Biome   
{
    public static function getBiomeTemperature(biomeTag:BiomeTag) : Float
    {
        return switch biomeTag {
            case GREEN: SGREEN;
            case SWAMP: SSWAMP;
            case YELLOW: SYELLOW;
            case GREY: SGREY;
            case SNOW: SSNOW;
            case DESERT: SDESERT;
            case JUNGLE: SJUNGLE;
            case BORDERJUNGLE: SCBORDERJUNGLE;
            case SNOWINGREY: SSNOWINGREY;
            case OCEAN: SOCEAN;
            case RIVER: SRIVER;
            case PASSABLERIVER: SPASSABLERIVER;
            default: 1;
        }
    }

    public function new()
    {

    }
}