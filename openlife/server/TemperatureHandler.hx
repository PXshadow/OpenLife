package openlife.server;

import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import haxe.ds.Vector;

/**
 * Tile-based temperature system
 * Handles temperature calculation for each tile based on:
 * - Biome base temperature
 * - Seasonal temperature impact
 * - Local heat sources (fire, oven, etc.)
 * - Floor and object insulation
 * - Temperature balancing with neighboring tiles
 */
class TemperatureHandler {
	// Temperature update step - processes a portion of the map each tick
	private static var temperatureUpdateStep = 0;

	/**
	 * Initialize tile temperature with base values
	 * Called when a tile first needs its temperature calculated
	 */
	private static function initializeTileTemperature(worldMap:WorldMap, x:Int, y:Int):Float {
		// Get base temperature from biome
		var biomeId = worldMap.getBiomeId(x, y);
		var biomeTemperature = Biome.getBiomeTemperature(biomeId);

		// Add seasonal impact
		var seasonImpact = TimeHelper.SeasonTemperatureImpact;

		// Add local heat from objects on this tile
		var localHeat = getLocalHeat(worldMap, x, y);

		// Calculate initial temperature
		var initialTemp = biomeTemperature + seasonImpact + localHeat;

		// Clamp to valid range
		initialTemp = clamp(initialTemp, 0.0, 2.0);

		// Store and return
		worldMap.setTileTemperature(x, y, initialTemp);

		// if (ServerSettings.DebugTemperature) {
		if (ServerSettings.DebugTemperature && x == 100 && y == 100) {
			trace('TemperatureHandler: Init TileTemp[$x,$y]: biome=$biomeTemperature season=$seasonImpact localHeat=$localHeat => $initialTemp');
		}

		return initialTemp;
	}

	/**
	 * Get local heat from objects on this tile
	 */
	private static function getLocalHeat(worldMap:WorldMap, x:Int, y:Int):Float {
		var objData = worldMap.getObjectDataAtPosition(x, y);
		var heat = objData.heatValue * ServerSettings.TemperatureLocalHeatFactor;
		// if (objData.id > 0 && heat != 0) trace('TemperatureHandler: ${objData.name} heat: ${heat}');
		return heat;
	}

	/**
	 * Get insulation from floor (using rValue)
	 */
	private static function getFloorInsulation(worldMap:WorldMap, x:Int, y:Int):Float {
		var floorId = worldMap.getFloorId(x, y);

		if (floorId <= 0) {
			return 0;
		}
		var objData = ObjectData.getObjectData(floorId);
		var insulation = objData.rValue;

		// if (objData.id > 0) trace('TemperatureHandler: Floor: ${objData.name} insulation: ${insulation}');
		return insulation;
	}

	/**
	 * Get insulation from objects on tile (walls, doors etc.)
	 */
	private static function getObjectInsulation(worldMap:WorldMap, x:Int, y:Int):Float {
		var objData = worldMap.getObjectDataAtPosition(x, y);
		var insulation = objData.rValue;
		if (objData.isWall() == false) return 0;
		// if (objData.id > 0 && insulation > 0) trace('TemperatureHandler: Object: ${objData.name} insulation: ${insulation}');
		return insulation;
	}

	/**
	 * Main update function - call this each tick from TimeHelper
	 * Processes a portion of the map per call (like DoWorldMapTimeStuff)
	 */
	/*public static function DoTileTemperatureTimeStuff(worldMap:WorldMap, deltaTime:Float) {
		var timeParts = ServerSettings.WorldTimeParts;

		var partSizeY = Std.int(worldMap.height / timeParts);
		var startY = (temperatureUpdateStep % timeParts) * partSizeY;
		var endY = startY + partSizeY;

		temperatureUpdateStep++;

		for (y in startY...endY) {
			for (x in 0...worldMap.width) {
				updateTileTemperature(worldMap, x, y, deltaTime);
			}
		}
	}*/
	/**
	 * Update temperature for a single tile
	 */
	public static function UpdateTileTemperature(worldMap:WorldMap, x:Int, y:Int, deltaTime:Float) {
		// var idx = worldMap.index(x, y);

		// Initialize if needed
		if (worldMap.getTileTemperature(x, y) < 0) {
			initializeTileTemperature(worldMap, x, y);
			return;
		}

		var currentTemp = worldMap.getTileTemperature(x, y);

		// Get base temperature (biome + season)
		var biomeId = worldMap.getBiomeId(x, y);
		var biomeTemperature = Biome.getBiomeTemperature(biomeId);
		var seasonImpact = TimeHelper.SeasonTemperatureImpact;
		if (seasonImpact > 0) seasonImpact *= ServerSettings.HotSeasonTemperatureFactor;
		if (seasonImpact < 0) seasonImpact *= ServerSettings.ColdSeasonTemperatureFactor;

		// Get local heat
		var localHeat = getLocalHeat(worldMap, x, y);

		// Get insulations
		var floorInsulation = getFloorInsulation(worldMap, x, y);
		var objectInsulation = getObjectInsulation(worldMap, x, y);
		// floorInsulation *= floorInsulation;

		// Calculate insulation factor (multiplicative)
		var insulationFactor = (1 - floorInsulation) * (1 - objectInsulation);
		insulationFactor *= insulationFactor;
		insulationFactor = clamp(insulationFactor, 0, 1);

		// Calculate target temperature (what the tile "wants" to be)
		// var targetTemp = biomeTemperature + seasonImpact + localHeat;
		var targetTemp = biomeTemperature + seasonImpact;
		targetTemp = clamp(targetTemp, 0.0, 5.0);

		// Step 1: Move toward target, slowed by insulation
		// Higher insulation = slower movement toward target
		var moveSpeed = ServerSettings.TemperatureOwnTileRate * insulationFactor;
		var moveSpeedDelta = clamp(moveSpeed * deltaTime, 0.0, 0.9);
		var newTemp = lerp(currentTemp, targetTemp, moveSpeedDelta);

		if (newTemp > 0.8) localHeat *= 0.5;
		localHeat *= deltaTime;

		// localHeat *= 0.8;

		if (ServerSettings.DebugTemperature && x == 100 && y == 100) {
			trace('TileTemp[100,100]: deltaTime: ${Math.round(deltaTime * 1000) / 1000} moveSpeed: ${Math.round(moveSpeed * 1000) / 1000}');
		}

		// Step 2: Balance with neighbors (thermal diffusion)
		var moveSpeedDeltaNeighbor = clamp(ServerSettings.TemperatureBalanceRate * deltaTime * (1 - objectInsulation), 0.0, 0.9);
		var neighborTempSum = 0.0;
		var neighborTempDiff = 0.0;
		var neighborCount = 0;

		// Get all 8 neighbors
		// var neighbors = getNeighbors(x, y, worldMap.width, worldMap.height);

		for (dy in -1...2) {
			for (dx in -1...2) {
				if (dx == 0 && dy == 0) continue; // Skip self
				var neighborTemp = worldMap.getTileTemperature(x + dx, y + dy);

				// If neighbor not initialized, skip it
				if (neighborTemp < 0) continue;

				neighborTempSum += neighborTemp;
				neighborCount++;

				var tempDiffN = (newTemp - neighborTemp) * moveSpeedDeltaNeighbor * 0.1;
				neighborTempDiff -= tempDiffN;

				if (localHeat != 0) {
					worldMap.setTileTemperature(x + dx, y + dy, neighborTemp + localHeat + tempDiffN);
				}
			}
		}

		newTemp += localHeat * 1.2;

		var averageNeighborTemp = neighborTempSum / neighborCount;
		var tempDiff = averageNeighborTemp - newTemp;

		// Apply balance rate
		// newTemp += tempDiff * moveSpeedDeltaNeighbor;
		newTemp += neighborTempDiff;

		// Clamp final temperature
		newTemp = clamp(newTemp, 0.0, 5.0);

		// Store result
		worldMap.setTileTemperature(x, y, newTemp);

		if (ServerSettings.DebugTemperature && x == 100 && y == 100) {
			// trace('TileTemp[100,100]: $currentTemp -> $newTemp (target=$targetTemp, $moveSpeedDelta, averageNeighborTemp=$averageNeighborTemp, $moveSpeedDeltaNeighbor ins=$insulationFactor)');
			trace('TileTemp[100,100]: ${Math.round(currentTemp * 1000) / 1000} -> ${Math.round(newTemp * 1000) / 1000} (target=${Math.round(targetTemp * 1000) / 1000}, ${Math.round(moveSpeedDelta * 1000) / 1000}, averageNeighborTemp=${Math.round(averageNeighborTemp * 1000) / 1000}, ${Math.round(moveSpeedDeltaNeighbor * 1000) / 1000}, neighborTempDiff ${Math.round(neighborTempDiff * 1000) / 1000}} ins=${Math.round(insulationFactor * 1000) / 1000})');
		}
	}

	/**
	 * Linear interpolation
	 */
	private static function lerp(from:Float, to:Float, t:Float):Float {
		return from + (to - from) * t;
	}

	/**
	 * Clamp value between min and max
	 */
	private static function clamp(value:Float, min:Float, max:Float):Float {
		if (value < min) return min;
		if (value > max) return max;
		return value;
	}

	/**
	 * Get temperature at player position (for player temperature calculation)
	 * This replaces the old biome-based calculation
	 */
	/*public static function getPlayerTileTemperature(worldMap:WorldMap, x:Int, y:Int):Float {
		var idx = worldMap.index(x, y);

		// Initialize if needed
		if (worldMap.tileTemperatures[idx] < 0) {
			return initializeTileTemperature(worldMap, x, y);
		}

		return worldMap.tileTemperatures[idx];
	}*/
}
