package openlife.server;

import openlife.data.object.ObjectData.PersonColor;
import openlife.data.object.ObjectHelper;

@:enum abstract BiomeTag(Int) from Int to Int {
	public var GREEN = 0;
	public var SWAMP = 1;
	public var YELLOW = 2;
	public var GREY = 3;
	public var SNOW = 4;
	public var DESERT = 5;
	public var JUNGLE = 6;
	public var BORDERJUNGLE = 15; // 8 or 15
	public var SNOWINGREY = 21; // 7 // its snow on top of mountains which should not be walkable
	public var OCEAN = 9; // deep ocean
	public var PASSABLERIVER = 13;
	public var RIVER = 17; // TODO deep river which is not walkable
}

@:enum abstract BiomeMapColor(String) from String to String {
	public var CGREEN = "FFB5E61D";
	public var CSWAMP = "FF008080";
	public var CYELLOW = "FFFECC36"; // savannah
	public var CGREY = "FF808080"; // badlands // bevor it was: FF404040
	public var CSNOW = "FFFFFFFF";
	public var CDESERT = "FFDBAC4D";
	public var CJUNGLE = "FF007F0E";
	public var CBORDERJUNGLE = "FF007F00";
	public var CSAND = "FFefe4b0";
	public var CSNOWINGREY = "FF404040"; // its snow on top of mountains which should not be walkable
	public var COCEAN = "FF004080"; // deep ocean
	public var CRIVER = "FF0080FF"; // shallow water
	public var CPASSABLERIVER = "FF00E8FF"; // TODO use also for passable ocean? otherwise use biomeID: 22???
}

@:enum abstract BiomeSpeed(Float) from Float to Float {
	// var truncMovementSpeedDiff = 0.1;
	// considered as bad biome for horses if speed < 0.999
	// TODO make fast for specialists
	public var SGREEN = 1;
	public var SSWAMP = 0.8; // 0.6
	public var SYELLOW = 1;
	public var SGREY = 0.98;
	public var SSNOW = 0.98;
	public var SDESERT = 0.98;
	public var SJUNGLE = 0.98;
	public var SCBORDERJUNGLE = 0.98;
	public var SSNOWINGREY = 0.01;
	public var SOCEAN = 0.01;
	public var SRIVER = 0.01;
	public var SPASSABLERIVER = 0.8; // 0.6;
}

// Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest, and 0.5 is ideal.
@:enum abstract BiomeTemperature(Float) from Float to Float {
	public var TGREEN = 0.4;
	public var TSWAMP = 0.2;
	public var TYELLOW = 0.3;
	public var TGREY = 0.2;
	public var TSNOW = 0;
	public var TDESERT = 1; // loved by black but still little bit too hot
	public var TJUNGLE = 0.7; // perfect for brown
	public var TCBORDERJUNGLE = 0.6; // perfect for brown
	public var TSNOWINGREY = 0.0;
	public var TOCEAN = 0.0;
	public var TRIVER = 0.0;
	public var TPASSABLERIVER = 0.2;
}

class Biome {
	// Heat is the player's warmth between 0 and 1, where 0 is coldest, 1 is hottest, and 0.5 is ideal.
	public static function getBiomeTemperature(biomeTag:BiomeTag):Float {
		return switch biomeTag {
			case GREEN: TGREEN;
			case SWAMP: TSWAMP;
			case YELLOW: TYELLOW;
			case GREY: TGREY;
			case SNOW: TSNOW;
			case DESERT: TDESERT;
			case JUNGLE: TJUNGLE;
			case BORDERJUNGLE: TCBORDERJUNGLE;
			case SNOWINGREY: TSNOWINGREY;
			case OCEAN: TOCEAN;
			case RIVER: TRIVER;
			case PASSABLERIVER: TPASSABLERIVER;
			default: 0.5;
		}
	}

	public static function IsBiomeLovedbyColor(biome:BiomeTag, player:GlobalPlayerInstance) {
		var lovedBiome = GetLovedBiomeByPlayer(player);
		return lovedBiome == biome;
	}

	public static function GetLovedBiomeByPlayer(player:GlobalPlayerInstance):BiomeTag {
		var personColor = player.getColor();

		if (personColor == PersonColor.Ginger) return BiomeTag.SNOW;
		if (personColor == PersonColor.White) return BiomeTag.GREY;
		if (personColor == PersonColor.Brown) return BiomeTag.JUNGLE;
		if (personColor == PersonColor.Black) return BiomeTag.DESERT;

		return -1;
	}

	// TODO better set in ObjdData, since there could be more then one
	// TODO not used yet. Meant for getting biome experience if eaten

	/*public static function getBiomeByFood(food:ObjectHelper) : BiomeTag
		{
			return switch food.parentId {  
				case 768: DESERT; // Cactus Fruit   
				case 2143: JUNGLE; // banana
				case 4252: GREY; // Wild Garlic
				case 40: SNOW; // Wild Carrot
				default: -1; 
			}
	}*/
	public static function getLovedFoodIds(biomeTag:BiomeTag):Array<Int> {
		return switch biomeTag {
			case DESERT: [768, 197]; // Cactus Fruit // Cooked Rabbit
			case JUNGLE: [2143, 1880]; // banana // Mango Slices
			case GREY: [4252, 1242]; // Wild Garlic // Bowl of Sauerkraut
			case SNOW: [40, 2106]; // Wild Carrot // Cooked Fish
			default: [];
		}
	}
	
	public static function getBiomeAnimals(biomeTag:BiomeTag):Array<Int> {
		return switch biomeTag {
			case DESERT: [764]; // Rattle Snake
			case JUNGLE: [2156]; // Mosquito Swarm
			case GREY: [418]; // Wolf
			case SNOW: [418]; // Wolf
			default: [];
		}
	}

	public static function getLovedPlants(biomeTag:BiomeTag):Array<Int> {
		return switch biomeTag {
			case DESERT: [763]; // Fruiting Barrel Cactus
			case JUNGLE: [2142]; // Banana Plant
			case GREY: [4251]; // Wild Garlic (on ground)
			case SNOW: [39]; // Dug Wild Carrot
			default: [];
		}
	}

	public static function IsWater(biome:BiomeTag):Bool {
		return biome == OCEAN || biome == PASSABLERIVER || biome == RIVER;
	}

	public function new() {}
}
