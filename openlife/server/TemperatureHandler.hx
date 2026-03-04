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
		// Get average base temperature from original and current biome
		var biomeTemperature = worldMap.getAverageBiomeTemperature(x, y);

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
	 * Balances temperature for a single tile with its neighbors (extracted from UpdateTileTemperature)
	 */
	private static function balanceTileTemperature(worldMap:WorldMap, x:Int, y:Int, deltaTime:Float, doLocalHeat:Bool = false) {
		var currentTemp = worldMap.getTileTemperature(x, y);
		if (currentTemp < 0) return;

		// Balance only temperature around players if tile is a floor
		var floorInsulation = getFloorInsulation(worldMap, x, y);
		if (doLocalHeat == false && floorInsulation < 0.1) return;

		// Dont apply heat exchange with walls / doors if balancing around a player to not loose heat twice
		var objectInsulation = getObjectInsulation(worldMap, x, y);
		if (doLocalHeat == false && objectInsulation > 0) return;

		var moveSpeedDeltaNeighbor = clamp(ServerSettings.TemperatureBalanceRate * deltaTime * (1 - objectInsulation), 0.0, 0.9);

		var localHeat = doLocalHeat ? getLocalHeat(worldMap, x, y) : 0;
		if ((localHeat > 0.01 && currentTemp > 0.6) || (localHeat < -0.01 && currentTemp < 0.2)) {
			localHeat *= 0.5;
			// let fire live longer if it can  burn low
			var obj = worldMap.getObjectHelper(x, y);
			if (obj.timeToChange > 2) {
				obj.timeToChange += deltaTime * 0.5;
				if (ServerSettings.DebugTemperature)
					trace('TileTemp: deltaTime: ${obj.name} ${obj.tx} ${obj.ty} time: ${Math.round(deltaTime * 1000) / 1000} timeToChange: ${Math.round(obj.timeToChange)}');
			}
		}

		localHeat *= deltaTime;

		var neighborTempDiff = 0.0;
		var neighborCount = 0;

		for (dy in -1...2) {
			for (dx in -1...2) {
				if (dx == 0 && dy == 0) continue;
				var neighborTemp = worldMap.getTileTemperature(x + dx, y + dy);
				if (neighborTemp < 0) continue;

				if (doLocalHeat == false) {
					// Dont apply heat exchange with walls / doors if balancing around a player to not loose heat twice
					var neighborObjectInsulation = getObjectInsulation(worldMap, x + dx, y + dy);
					if (neighborObjectInsulation > 0) continue;
				}

				neighborCount++;
				var tempDiffN = (currentTemp - neighborTemp) * moveSpeedDeltaNeighbor * 0.1;
				neighborTempDiff -= tempDiffN;

				worldMap.setTileTemperature(x + dx, y + dy, neighborTemp + localHeat + tempDiffN);
			}
		}

		var newTemp = currentTemp + neighborTempDiff + localHeat * 1.2;
		newTemp = clamp(newTemp, 0.0, 5.0);
		worldMap.setTileTemperature(x, y, newTemp);
	}

	/**
	 * Update temperature for a single tile
	 */
	public static function UpdateTileTemperature(worldMap:WorldMap, x:Int, y:Int, deltaTime:Float) {
		// Initialize if needed
		if (worldMap.getTileTemperature(x, y) < 0) {
			initializeTileTemperature(worldMap, x, y);
			return;
		}

		var currentTemp = worldMap.getTileTemperature(x, y);

		// Get average base temperature from original and current biome (biome + season)
		var biomeTemperature = worldMap.getAverageBiomeTemperature(x, y);
		var seasonImpact = TimeHelper.SeasonTemperatureImpact;
		if (seasonImpact > 0) seasonImpact *= ServerSettings.HotSeasonTemperatureFactor;
		if (seasonImpact < 0) seasonImpact *= ServerSettings.ColdSeasonTemperatureFactor;

		// Get local heat
		// var localHeat = getLocalHeat(worldMap, x, y);

		// Get insulations
		var floorInsulation = getFloorInsulation(worldMap, x, y);
		var objectInsulation = getObjectInsulation(worldMap, x, y);

		// Calculate insulation factor (multiplicative)
		var insulationFactor = (1 - floorInsulation) * (1 - objectInsulation);
		insulationFactor *= insulationFactor;
		insulationFactor = clamp(insulationFactor, 0, 1);

		// Calculate target temperature (what the tile "wants" to be)
		var targetTemp = biomeTemperature + seasonImpact;
		targetTemp = clamp(targetTemp, 0.0, 5.0);

		// Step 1: Move toward target, slowed by insulation
		var moveSpeed = ServerSettings.TemperatureOwnTileRate * insulationFactor;
		var moveSpeedDelta = clamp(moveSpeed * deltaTime, 0.0, 0.9);
		var newTemp = lerp(currentTemp, targetTemp, moveSpeedDelta);

		if (ServerSettings.DebugTemperature && x == 100 && y == 100) {
			trace('TileTemp[100,100]: deltaTime: ${Math.round(deltaTime * 1000) / 1000} moveSpeed: ${Math.round(moveSpeed * 1000) / 1000}');
		}

		// Step 2: Balance with neighbors (thermal diffusion) - using extracted function
		worldMap.setTileTemperature(x, y, newTemp);
		balanceTileTemperature(worldMap, x, y, deltaTime, true);
		newTemp = worldMap.getTileTemperature(x, y);

		// Clamp final temperature
		newTemp = clamp(newTemp, 0.0, 5.0);
		worldMap.setTileTemperature(x, y, newTemp);

		if (ServerSettings.DebugTemperature && x == 100 && y == 100) {
			trace('TileTemp[100,100]: ${Math.round(currentTemp * 1000) / 1000} -> ${Math.round(newTemp * 1000) / 1000} (target=${Math.round(targetTemp * 1000) / 1000}, ${Math.round(moveSpeedDelta * 1000) / 1000}, ins=${Math.round(insulationFactor * 1000) / 1000})');
		}
	}

	/**
	 * Balance temperature for all tiles within radius d around (x, y),
	 * iterating ring by ring from inside (ring 0) to outside (ring d).
	 * Uses Chebyshev distance so rings form expanding squares.
	 */
	public static function BalanceTemperatureArea(x:Int, y:Int, d:Int, deltaTime:Float) {
		var worldMap = WorldMap.get_world();

		// Ring 0: the center tile itself

		balanceTileTemperature(worldMap, x, y, deltaTime);

		// Rings 1..d: walk the perimeter of each Chebyshev ring
		for (ring in 1...d + 1) {
			// Top and bottom rows of the ring
			for (dx in -ring...ring + 1) {
				balanceTileTemperature(worldMap, x + dx, y - ring, deltaTime); // top
				balanceTileTemperature(worldMap, x + dx, y + ring, deltaTime); // bottom
			}
			// Left and right columns of the ring (excluding corners already covered)
			for (dy in -ring + 1...ring) {
				balanceTileTemperature(worldMap, x - ring, y + dy, deltaTime); // left
				balanceTileTemperature(worldMap, x + ring, y + dy, deltaTime); // right
			}
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
}
