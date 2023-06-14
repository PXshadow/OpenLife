package openlife.auto;

import haxe.Exception;
import haxe.ds.Vector;
import openlife.auto.Pathfinder.Coordinate;
import openlife.client.ClientTag;
import openlife.data.Pos;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import openlife.macros.Macro;
import openlife.server.Biome.BiomeTag;
import openlife.server.GlobalPlayerInstance;
import openlife.server.Lineage.PrestigeClass;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;

class AiHelper {
	static final RAD:Int = MapData.RAD; // search radius

	private static function GetName(objId:Int):String {
		return ObjectData.getObjectData(objId).name;
	}

	public static function CalculateDistanceToPlayer(player:PlayerInterface, playerTo:PlayerInterface):Float {
		var rx = WorldMap.world.transformX(player, playerTo.tx);
		var ry = WorldMap.world.transformY(player, playerTo.ty);

		return CalculateQuadDistanceHelper(player.x, player.y, rx, ry);
	}

	public static function IsCloseToObject(player:PlayerInterface, obj:ObjectHelper, distance:Float):Bool {
		var quadDistance = CalculateQuadDistanceToObject(player, obj);
		return quadDistance <= Math.pow(distance, 2);
	}

	public static function CalculateQuadDistanceToObject(player:PlayerInterface, obj:ObjectHelper):Float {
		var rx = WorldMap.world.transformX(player, obj.tx);
		var ry = WorldMap.world.transformY(player, obj.ty);

		return CalculateQuadDistanceHelper(player.x, player.y, rx, ry);
	}

	public static function CalculateQuadDistanceBetweenObjects(player:PlayerInterface, obj1:ObjectHelper, obj2:ObjectHelper):Float {
		var rx1 = WorldMap.world.transformX(player, obj1.tx);
		var ry1 = WorldMap.world.transformY(player, obj1.ty);
		var rx2 = WorldMap.world.transformX(player, obj2.tx);
		var ry2 = WorldMap.world.transformY(player, obj2.ty);

		return CalculateQuadDistanceHelper(rx1, ry1, rx2, ry2);
	}

	public static function CalculateQuadDistanceHelper(baseX:Int, baseY:Int, toX:Int, toY:Int):Float {
		return (toX - baseX) * (toX - baseX) + (toY - baseY) * (toY - baseY);
	}

	// does consider round map
	public static function CalculateDistance(baseX:Int, baseY:Int, toX:Int, toY:Int):Float {
		var diffX = toX - baseX;
		var diffY = toY - baseY;
		var width = WorldMap.world.width;
		var height = WorldMap.world.height;

		if (diffX > width / 2) diffX -= width; // consider that world is round
		else if (diffX < -width / 2) diffX += width; // consider that world is round
		if (diffY > height / 2) diffY -= height; // consider that world is round
		else if (diffY < -height / 2) diffY += height; // consider that world is round

		return diffX * diffX + diffY * diffY;
	}

	public static function GetClosestObjectOwnedByPlayer(playerInterface:PlayerInterface, searchDistance:Int = 10):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, false, true));
	}

	public static function GetClosestHeatObject(playerInterface:PlayerInterface, searchDistance:Int = 2):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, true));
	}

	// searchDistance old: 16

	public static function GetClosestObjectById(player:PlayerInterface, objIdToSearch:Int, ignoreObj:ObjectHelper = null, searchDistance:Int = 40,
			minDistance:Int = 0):ObjectHelper {
		// return GetClosestObject(playerInterface, objData, searchDistance, ignoreObj);
		return GetClosestObjectToPosition(player.tx, player.ty, objIdToSearch, searchDistance, ignoreObj, player, null, minDistance);
	}

	public static function GetClosestObjectToTarget(player:PlayerInterface, target:ObjectHelper, objIdToSearch:Int, ignoreObj:ObjectHelper = null,
			searchDistance:Int = 40, minDistance:Int = 0):ObjectHelper {
		// return GetClosestObject(playerInterface, objData, searchDistance, ignoreObj);
		return GetClosestObjectToPosition(target.tx, target.ty, objIdToSearch, searchDistance, ignoreObj, player, null, minDistance);
	}

	public static function GetClosestObjectToHome(player:PlayerInterface, objIdToSearch:Int, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null):ObjectHelper {
		// if(objIdToSearch == 1121) trace('DEBUG7770: ${myPlayer.name}${myPlayer.id} Popcorn dist: ${searchDistance}');
		return GetClosestObjectToPosition(player.home.tx, player.home.ty, objIdToSearch, searchDistance, ignoreObj, player);
	}

	// Basket of Bones (356) // Basket of Soil 336 // Bowl of Soil 1137
	// Straw 227 // Wheat Sheaf 225
	static var needsNotFlooredPlace = [356, 336, 1137, 227, 225];
	// Basket of Bones (356) // Basket of Soil 336 // Bowl of Soil 1137
	// Straw 227 // Wheat Sheaf 225 // Stone Hoe 850 // Rabbit Bones 190 // Snare 160 // Yew Branch 132
	// Rabbit Fur 183 // Yew Branch 132 //
	// Flat Rock 291 (only if needed for forge)
	static var dontDropCloseHomeIds = [356, 336, 1137, 227, 225, 850, 190, 160, 132, 183, 132, 291];

	public static function GetClosestObjectToPosition(baseX:Int, baseY:Int, objIdToSearch:Int, searchDistance:Int = 40, ignoreObj:ObjectHelper = null,
			player:PlayerInterface = null, searchContained:Array<Int> = null, minDistance:Int = 0):ObjectHelper {
		var result = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = GetClosestObjectToPositionHelper(baseX, baseY, objIdToSearch, searchDistance, ignoreObj, player, searchContained,
			minDistance));
		WorldMap.world.mutex.release();

		// TODO: check if exception happened throw new Exception('Error GetClosestObjectToPosition');

		return result;
	}

	// objIdToSearch = -10 if searching non permanent
	// minDistance = -1 allows to be dropped close to oven even if it normaly should not (flat rock for forge)
	private static function GetClosestObjectToPositionHelper(baseX:Int, baseY:Int, objIdToSearch:Int, searchDistance:Int = 40, ignoreObj:ObjectHelper = null,
			player:PlayerInterface = null, searchContained:Array<Int> = null, minDistance:Int = 0):ObjectHelper {
		var world = WorldMap.world;
		var ai = player == null ? null : player.getAi();
		var ignoreFullPiles = ai == null ? false : ai.ignoreFullPiles;
		var closestObject = null;
		var bestDistance = 0.0;
		var closestBadPlace = null;
		var bestDistanceToBadPlace = 0.0;
		var searchEmptyPlace = ai != null && objIdToSearch == 0;
		var heldId = player == null ? -1 : player.heldObject.parentId;
		var searchNotFlooredPlace = searchEmptyPlace && needsNotFlooredPlace.contains(heldId);
		var objdataRoSearch = ObjectData.getObjectData(objIdToSearch);
		var allowContainerWithItems = objdataRoSearch == null ? false : objdataRoSearch.isGrave();
		var quadMinDistance = minDistance * minDistance;
		// TODO consider also other objects like Kiln, Forge, Fire?
		var mindistaceToTargetObj = player == null ? null : player.home;
		var minDistanceToTarget = dontDropCloseHomeIds.contains(heldId) ? 6 : 0; // to home
		var quadMinDistanceToTarget = minDistanceToTarget * minDistanceToTarget;

		// minDistance = -1 allows to be dropped close to oven even if it normaly should not (flat rock for forge)
		if (minDistance < 0) {
			quadMinDistance = 0;
			minDistanceToTarget = 0;
			quadMinDistanceToTarget = 0;
		}

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				var objData = world.getObjectDataAtPosition(tx, ty);
				var parentId = objData.parentId;

				// used for example to drop a basket with clay near kiln and switch with existing
				if (objIdToSearch == -10) {
					if (objData.isPermanent()) continue;
					if (objData.parentId == heldId) continue; // dont replace a basket with a basket
				} else if (parentId != objIdToSearch) continue;

				if (ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;

				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && ai.isObjectWithHostilePath(tx, ty)) continue;

				// search not full pile, for example to drop an item on a pile
				if (ignoreFullPiles && objData.numUses > 1) {
					var obj = world.getObjectHelper(tx, ty);
					if (obj.numberOfUses >= objData.numUses) continue;
				}

				// dont use carrots if seed is needed // 400 Carrot Row
				if (ai != null && parentId == 400 && ai.hasCarrotSeeds == false) {
					var obj = world.getObjectHelper(tx, ty);
					if (obj.numberOfUses < 3) continue;
				}

				if (searchNotFlooredPlace) {
					var floorId = world.getFloorId(tx, ty);
					if (floorId > 0) continue;
				}

				if (searchContained == null) {
					if (allowContainerWithItems == false && objData.numSlots > 0) {
						var obj = world.getObjectHelper(tx, ty);
						if (obj.containedObjects.length > 0) continue;
					}
				} else {
					if (objData.numSlots < 1) continue;
					var obj = world.getObjectHelper(tx, ty);
					if (obj.contains(searchContained) == false) continue;
				}

				var quadDistance = AiHelper.CalculateQuadDistanceHelper(tx, ty, baseX, baseY);

				if (quadDistance < quadMinDistance) continue; // for example used to not pikcup kindling close to fire to bring this kindling then to fire

				if (searchEmptyPlace && mindistaceToTargetObj != null && quadMinDistanceToTarget > 0) {
					var quadDistanceToHome = AiHelper.CalculateQuadDistanceHelper(tx, ty, mindistaceToTargetObj.tx, mindistaceToTargetObj.ty);
					if (quadDistanceToHome < quadMinDistanceToTarget) continue;
				}

				// dont drop stuff behind a tree
				var objDataBelow = world.getObjectDataAtPosition(tx, ty - 1);
				if (searchEmptyPlace && objDataBelow.isTree()) continue;

				// try not drop stuff in water or mountain
				if (searchEmptyPlace && IsBadBiomeForDrop(tx, ty)) {
					if (closestBadPlace != null && quadDistance > bestDistanceToBadPlace) continue;

					closestBadPlace = world.getObjectHelper(tx, ty);
					bestDistanceToBadPlace = quadDistance;

					continue;
				}

				if (closestObject != null && quadDistance > bestDistance) continue;

				bestDistance = quadDistance;
				closestObject = world.getObjectHelper(tx, ty);
			}
		}

		return closestObject != null ? closestObject : closestBadPlace;
	}

	public static function GetClosestObjectToPositionByIds(baseX:Int, baseY:Int, objIdsToSearch:Array<Int>, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null, player:PlayerInterface = null, minDistance:Int = 0, ignoreHostile = true):ObjectHelper {
		var result = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = GetClosestObjectToPositionByIdsHelper(baseX, baseY, objIdsToSearch, searchDistance, ignoreObj, player, minDistance,
			ignoreHostile));
		WorldMap.world.mutex.release();

		// TODO: check if exception happened throw new Exception('Error GetClosestObjectToPosition');

		return result;
	}

	private static function GetClosestObjectToPositionByIdsHelper(baseX:Int, baseY:Int, objIdsToSearch:Array<Int>, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null, player:PlayerInterface = null, minDistance:Int = 0, ignoreHostile = true):ObjectHelper {
		var world = WorldMap.world;
		var ai = player == null ? null : player.getAi();
		var ignoreFullPiles = ai == null ? false : ai.ignoreFullPiles;
		var closestObject = null;
		var bestDistance = 0.0;
		var quadMinDistance = minDistance * minDistance;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				var objData = world.getObjectDataAtPosition(tx, ty);
				var parentId = objData.parentId;

				if (objIdsToSearch.contains(parentId) == false) continue;

				if (ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;

				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && ignoreHostile && ai.isObjectWithHostilePath(tx, ty)) continue;

				// search not full pile, for example to drop an item on a pile
				if (ignoreFullPiles && objData.numUses > 1) {
					var obj = world.getObjectHelper(tx, ty);
					if (obj.numberOfUses >= objData.numUses) continue;
				}

				var quadDistance = AiHelper.CalculateQuadDistanceHelper(tx, ty, baseX, baseY);

				if (quadDistance < quadMinDistance) continue; // for example used to not pickup kindling close to fire to bring this kindling then to fire

				if (closestObject != null && quadDistance > bestDistance) continue;

				bestDistance = quadDistance;
				closestObject = world.getObjectHelper(tx, ty);
			}
		}

		return closestObject;
	}

	public static function GetClosestObject(playerInterface:PlayerInterface, objDataToSearch:ObjectData, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null, findClosestHeat:Bool = false, ownedByPlayer:Bool = false):ObjectHelper {
		var result = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = GetClosestObjectHelper(playerInterface, objDataToSearch, searchDistance, ignoreObj, findClosestHeat, ownedByPlayer));
		WorldMap.world.mutex.release();

		// TODO: check if exception happened throw new Exception('Error GetClosestObjectToPosition');

		return result;
	}

	// searchDistance old: 16
	private static function GetClosestObjectHelper(playerInterface:PlayerInterface, objDataToSearch:ObjectData, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null, findClosestHeat:Bool = false, ownedByPlayer:Bool = false):ObjectHelper {
		// var RAD = ServerSettings.AiMaxSearchRadius
		// if(objDataToSearch != null) trace('GetClosestObject: ${objDataToSearch.name} dis: $searchDistance ignoreObj: ${ignoreObj != null}');

		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var heldId = player.heldObject.parentId;
		var objIdToSearch = objDataToSearch == null ? -1 : objDataToSearch.parentId;
		var searchEmptyPlace = ai != null && objDataToSearch != null && objDataToSearch.parentId == 0;
		// 1137 Bowl of Soil // 356 Basket of Bones // 336 Basket of Soil
		// TODO 1101 Fertile Soil Pile
		var searchNotFlooredPlace = ai != null && objIdToSearch == 0 && (heldId == 1137 || heldId == 356 || heldId == 336);
		var baseX = player.tx;
		var baseY = player.ty;
		var closestBadPlaceforDrop = null;
		var bestDistanceToBadPlaceforDrop = 0.0;
		var closestObject = null;
		var bestDistance = 0.0;

		// if(objIdToSearch == 1121) trace('DEBUG770: ${playerInterface.name}${playerInterface.id} Popcorn dist: ${searchDistance}');

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				// var objData = world.getObjectDataAtPosition(tx, ty);
				// if(objIdToSearch == 1121 && objData.parentId == 1121) trace('DEBUG771: Popcorn');

				if (ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;

				// findClosestHeat == false / since its jused for player temerpature where it does not matter if blocked. Also otherwiese a mutex would be missing since isObjectNotReachable can be used from the AI at same time
				if (ai != null && findClosestHeat == false && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && findClosestHeat == false && ai.isObjectWithHostilePath(tx, ty)) continue;

				var objData = world.getObjectDataAtPosition(tx, ty);

				if (ownedByPlayer && objData.isOwned == false) continue; // cannot have a owner

				if (findClosestHeat && objData.heatValue == 0) continue;

				if (ownedByPlayer || findClosestHeat || objData.parentId == objIdToSearch) // compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs
				{
					var objDataBelow = world.getObjectDataAtPosition(tx, ty - 1);
					if (searchEmptyPlace && objDataBelow.isTree()) continue;

					if (searchNotFlooredPlace) {
						var floorId = world.getFloorId(tx, ty);
						if (floorId > 0) continue;
					}

					var obj = world.getObjectHelper(tx, ty);

					if (ownedByPlayer && obj.isOwnedByPlayer(playerInterface) == false) continue;

					if (obj.containedObjects.length > 0 && obj.isGrave() == false) continue; // for example ignore filled baskets

					var distance = AiHelper.CalculateQuadDistanceToObject(playerInterface, obj);

					if (searchEmptyPlace && IsBadBiomeForDrop(tx, ty)) {
						// try not drop stuff in water or mountain
						if (closestBadPlaceforDrop == null || distance < bestDistanceToBadPlaceforDrop) {
							closestBadPlaceforDrop = obj;
							bestDistanceToBadPlaceforDrop = distance;
							// trace('Bad Empty Space For drop distance: $distance');
						}
					} else {
						if (closestObject == null || distance < bestDistance) {
							closestObject = obj;
							bestDistance = distance;
							// if(searchEmptyPlace) trace('Empty Space For drop distance: $distance');
						}
					}
				}
			}
		}

		/*if(closestObject != null){
			//playerInterface.say('No Empty tile found!');
			var isBadBiome = IsBadBiomeForDrop(closestObject.tx, closestObject.ty); 
			trace('AI: isBadBiome: $isBadBiome ${closestObject.name}');
		}*/

		// if(closestObject !=null) trace('AI: bestdistance: $bestDistance ${closestObject.description}');

		if (closestObject == null && searchEmptyPlace) playerInterface.say('No Empty tile found!');

		return closestObject != null ? closestObject : closestBadPlaceforDrop;
	}

	public static function GetCloseBiome(playerInterface:PlayerInterface, biomes:Array<Int>, searchDistance = 30) {
		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var bestBiome = new ObjectHelper(null, 0);
		var bestDistance:Float = searchDistance * 1000;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				var biomeId = world.getBiomeId(tx, ty);

				if (biomes.contains(biomeId) == false) continue;

				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && ai.isObjectWithHostilePath(tx, ty)) continue;

				var distance = AiHelper.CalculateDistance(player.tx, player.ty, tx, ty);

				if (distance > bestDistance) continue;

				bestDistance = distance;
				bestBiome.tx = tx;
				bestBiome.ty = ty;
			}
		}

		return bestDistance < 0 ? null : bestBiome;
	}

	public static function IsBadBiomeForDrop(tx:Int, ty:Int):Bool {
		var biomeId = WorldMap.world.getBiomeId(tx, ty);
		if (biomeId == PASSABLERIVER || biomeId == OCEAN || biomeId == RIVER || biomeId == SNOWINGREY) return true;

		return false;
	}

	// TODO autogenerate
	// Pile of Mouflon Hides 3918 // Pile of Sheep Skins 3919
	public static var clothsWithPilesIds = [3918, 3919];

	public static function GetCloseClothings(playerInterface:PlayerInterface, searchDistance:Int = 8):Array<ObjectHelper> {
		var result = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = GetCloseClothingsHelper(playerInterface, searchDistance));
		WorldMap.world.mutex.release();

		// TODO: check if exception happened throw new Exception('Error GetClosestObjectToPosition');

		return result;
	}

	private static function GetCloseClothingsHelper(playerInterface:PlayerInterface, searchDistance:Int = 8):Array<ObjectHelper> {
		// var RAD = ServerSettings.AiMaxSearchRadius
		// if(objDataToSearch != null) trace('GetClosestObject: ${objDataToSearch.name} dis: $searchDistance ignoreObj: ${ignoreObj != null}');

		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var clothings = new Array<ObjectHelper>();
		// var bestDistance = 0.0;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && ai.isObjectWithHostilePath(tx, ty)) continue;

				var objData = world.getObjectDataAtPosition(tx, ty);

				// compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs
				// Pile of Mouflon Hides 3918
				if (objData.clothing.charAt(0) != "n" || clothsWithPilesIds.contains(objData.parentId)) {
					var obj = world.getObjectHelper(tx, ty);
					clothings.push(obj);
				}
			}
		}

		/*for(obj in clothings)
			{
				trace('Clothing: ${obj.name} ${obj.objectData.clothing} slot: ${obj.objectData.getClothingSlot()}');
		}*/

		return clothings;
	}

	public static function isStillExpectedItem(player:PlayerInterface, obj:ObjectHelper):Bool {
		var newobj = player.getWorld().getObjectHelper(obj.tx, obj.ty);
		return (obj.parentId == newobj.parentId);
	}

	public static function isEatableCheckAgain(player:PlayerInterface, obj:ObjectHelper):Bool {
		var obj = player.getWorld().getObjectHelper(obj.tx, obj.ty);
		return isEatable(player, obj);
	}

	public static function isEatable(player:PlayerInterface, obj:ObjectHelper):Bool {
		var objData = obj.objectData.dummyParent != null ? obj.objectData.dummyParent : obj.objectData;
		var originalFoodValue = objData.foodFromTarget == null ? objData.foodValue : objData.foodFromTarget.foodValue;

		// if (ServerSettings.DebugAi) trace('AI: ${obj.description} ${obj.id} numberOfUses: ${obj.numberOfUses} originalFoodValue: $originalFoodValue');

		return originalFoodValue > 0;
		// if(originalFoodValue < 0) return false;
		// if(player.food_store_max - player.food_store < Math.ceil(originalFoodValue / 4)) return false;
		// return true;
	}

	public static function CountCloseObjects(player:PlayerInterface, tx:Int, ty:Int, objId:Int, radius:Int = 10, countPiles:Bool = true) {
		var count = -1;

		WorldMap.world.mutex.acquire();
		Macro.exception(count = CountCloseObjectsHelper(player, tx, ty, objId, radius, countPiles));
		WorldMap.world.mutex.release();

		if (count < 0) throw new Exception('Error CountCloseObjects');

		return count;
	}

	private static function CountCloseObjectsHelper(player:PlayerInterface, tx:Int, ty:Int, objId:Int, radius:Int = 10, countPiles:Bool = true) {
		var world = player.getWorld();
		var objdataToSearch = ObjectData.getObjectData(objId);
		var pileObjId = objdataToSearch.getPileObjId();
		var count = 0;

		if (objId == 233) countPiles = false; // Wet Clay Bowl 233 --> otherwise Wet Clay Crock is counted, too
		if (objId == 300) pileObjId = 4102; // Big Charcoal Pile 300 --> Huge Charcoal Pile 4102

		for (tty in ty - radius...ty + radius) {
			for (ttx in tx - radius...tx + radius) {
				var objData = world.getObjectDataAtPosition(ttx, tty);
				if (objData.parentId == objId) count++;
				if (countPiles && objData.parentId == pileObjId) {
					var obj = world.getObjectHelper(ttx, tty);
					count += obj.numberOfUses;
					// trace('CountCloseObjects: ${objdataToSearch.name}: found Pile: numberOfUses: ${obj.numberOfUses}');
				}
			}
		}

		// trace('CountCloseObjects: ${objdataToSearch.name}: ${count}');

		return count;
	}

	public static function SearchBestFood(player:PlayerInterface, feedingPlayer:PlayerInterface = null, radius:Int = 40):ObjectHelper {
		var bestFood = null;

		// TODO might need player mutex because: player.getCountEaten(foodId);

		GlobalPlayerInstance.AcquireMutex();
		// WorldMap.world.mutex.acquire();
		Macro.exception(bestFood = SearchBestFoodHelper(player, feedingPlayer, radius));
		// WorldMap.world.mutex.release();
		GlobalPlayerInstance.ReleaseMutex();

		return bestFood;
	}

	public static function SearchBestFoodHelper(player:PlayerInterface, feedingPlayer:PlayerInterface = null, radius:Int = 40):ObjectHelper {
		var startTime = Sys.time();
		var feedOther = feedingPlayer != null;
		var ai = feedingPlayer != null ? feedingPlayer.getAi() : player.getAi();
		var baseX = player.tx;
		var baseY = player.ty;
		var world = player.getWorld();
		var bestFood = null;
		var bestDistance = 999999.0;
		var bestFoodValue = 0.1;
		var bestFoods = new Array<ObjectHelper>();
		var isStarving = player.food_store < 3;
		var starvingFactor:Float = isStarving ? 4 : 16;
		var gooseFound = false;
		var countCarrots = -1;
		var countCorn = -1;

		if (player.food_store < 0.5) starvingFactor = 2;
		if (player.food_store < -1) starvingFactor = 1.5;
		if (player.food_store < -1.5) starvingFactor = 1.2;

		// var biome = WorldMap.worldGetBiomeId(player.tx, player.ty);
		// var originalBiomeTemperature = Biome.getBiomeTemperature(biome);

		for (ty in baseY - radius...baseY + radius) {
			for (tx in baseX - radius...baseX + radius) {
				var objData = world.getObjectDataAtPosition(tx, ty);

				if (objData.id == 0) continue;
				if (objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata

				var parentId = objData.parentId;
				// if (feedOther && objData.id == 837) continue; // dont feed Psilocybe Mushroom to others
				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;

				// dont eat Cooked Goose if there is only one since needed for crafting knife
				if (parentId == 518 && gooseFound == false) {
					gooseFound = true;
					continue;
				}

				// Carrot 402 // Carrot Pile 2742 ==> Leep some carrots for pies
				if (parentId == 402 || parentId == 2742) {
					if (countCarrots < 0) countCarrots = CountCloseObjects(player, player.tx, player.ty, 402, 20);
					if (countCarrots < 4) continue;
				}

				// Carrot Row 400 // dont eat up carrots that are needed for seeds
				if (parentId == 400) {
					var obj = world.getObjectHelper(tx, ty);
					if (obj.numberOfUses < 2) continue;
				}

				// Shucked Ear of Corn 1114 // Pile of Shucked Corn 3901
				if (parentId == 1114 && parentId == 3901) {
					if (countCorn < 0) {
						countCorn = CountCloseObjects(player, player.tx, player.ty, 1114, 20);
						// Dried Ear of Corn 1115
						countCorn += CountCloseObjects(player, player.tx, player.ty, 1115, 20);
					}
					if (countCorn < 2) continue;
				}

				// dont use carrots if seed is needed // 400 Carrot Row
				if (ai != null && parentId == 400 && ai.hasCarrotSeeds == false) {
					var obj = world.getObjectHelper(tx, ty);
					if (obj.numberOfUses < 3) continue;
				}

				// var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
				// trace('search food $tx, $ty: foodvalue: ${objData.foodValue} bestdistance: $bestDistance distance: $distance ${obj.description}');

				// var tmp = ObjectData.getObjectData(31);
				// trace('berry food: ${tmp.foodValue}');

				var objData = objData.foodFromTarget == null ? objData : objData.foodFromTarget;
				var originalFoodValue = objData.foodValue;
				var foodId = objData.getFoodId();
				var foodValue:Float = originalFoodValue;

				if (foodValue <= 0) continue;
				if (feedOther && player.canFeedToMeObj(objData) == false) continue;
				if (player.canEatObj(objData) == false) continue;
				if (player.food_store_max - player.food_store < Math.ceil(originalFoodValue / 4)) continue;

				var countEaten = player.getCountEaten(foodId);
				var quadDistance = 16 + AiHelper.CalculateDistance(baseX, baseY, tx, ty);
				if (feedingPlayer != null) quadDistance += 1 + AiHelper.CalculateDistance(feedingPlayer.tx, feedingPlayer.ty, tx, ty);

				foodValue -= countEaten;
				var isYum = countEaten < ServerSettings.YumBonus;
				var isSuperMeh = foodValue < originalFoodValue / 2; // can eat if food_store < 0
				// trace('search food: best $bestDistance dist $distance ${obj.description}');

				if (isYum) foodValue *= starvingFactor;
				if (isSuperMeh) foodValue = originalFoodValue / starvingFactor;
				if (isSuperMeh && player.food_store > 3) foodValue = 0;
				if (foodId == player.getCraving()) foodValue *= starvingFactor;
				// if (foodId == player.getCraving()) foodValue *= Math.pow(starvingFactor, 2);

				if (quadDistance < 1) quadDistance = 1;
				// distance = Math.sqrt(distance);

				if (bestFood == null || foodValue / quadDistance > bestFoodValue / bestDistance) {
					var obj = world.getObjectHelper(tx, ty);

					// Ripe Onions
					if (obj.parentId == 2854 && obj.numberOfUses < 3) continue;

					if (ai != null) {
						if (quadDistance > 4 && IsDangerous(player, obj)) continue;
						// dont eat carrots if seed is needed // 400 Carrot Row
						if (obj.parentId == 400 && ai.hasCarrotSeeds == false && obj.numberOfUses < 3) continue;
						// Dont eat if no corn seeds // 1114 Shucked Ear of Corn
						if (obj.parentId == 1114 && ai.hasCornSeeds == false) continue;

						// if(tryGotoObj(player, obj) == false) continue;
					}

					bestFoods.push(obj);

					bestFood = obj;
					bestDistance = quadDistance;
					bestFoodValue = foodValue;

					// trace('search best food: d: $bestDistance f: $bestFoodValue yum: $isYum  ${obj.description}');
				}
			}
		}

		// TODO tryGoto not in mutex and use sorted list
		/*if (ai != null) {
			// TODO solve This has still the problem, that the second best food might be ignored... needs to make a list and then sort or maybe just do again searchfood?
			while (bestFoods.length > 0) {
				var food = bestFoods.pop();
				if (tryGotoObj(player, food)) {
					bestFood = food;
					break;
				}

				bestFood = null;
				if (ServerSettings.DebugAi) trace('AI: bestfood: cannot reach food! ms: ${Math.round((Sys.time() - startTime) * 1000)}');
				if ((Sys.time() - startTime) * 1000 > 100) break;
			}
		}*/

		if (bestFood != null) if (ServerSettings.DebugAi)
			trace('AI: ms: ${Math.round((Sys.time() - startTime) * 1000)} bestfood: $bestDistance ${bestFood.description} ${bestFood.id}'); else
			if (ServerSettings.DebugAi) trace('AI: ms: ${Math.round((Sys.time() - startTime) * 1000)} bestfood: NA');

		return bestFood;
	}

	public static function IsDangerous(player:PlayerInterface, object:ObjectHelper, radius:Int = 4):Bool {
		var result = false;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = IsDangerousHelper(player, object, radius));
		WorldMap.world.mutex.release();

		// TODO throw exception if failed

		return result;
	}

	private static function IsDangerousHelper(player:PlayerInterface, object:ObjectHelper, radius:Int = 4):Bool {
		var ai = player.getAi();
		var baseX = object.tx;
		var baseY = object.ty;

		if (ai == null) return false;

		for (ty in baseY - radius...baseY + radius) {
			for (tx in baseX - radius...baseX + radius) {
				var objData = WorldMap.world.getObjectDataAtPosition(tx, ty);
				if (objData.isAnimal() && objData.deadlyDistance > 0) return true;

				if (ai.isObjectWithHostilePath(tx, ty)) return true; // for example if the path is blocked through a wolf
			}
		}

		return false;
	}

	public static function tryGotoObj(player:PlayerInterface, obj:ObjectHelper):Bool {
		return gotoObj(player, obj, false);
	}

	public static function gotoObj(player:PlayerInterface, obj:ObjectHelper, move:Bool = true, checkIfDangerous = true, ?infos:haxe.PosInfos):Bool {
		// TODO stop going to target if too far???

		var ai = player.getAi();
		var distance = CalculateQuadDistanceToObject(player, obj);
		if (ai.lastGotoObj == obj && ai.lastGotoObjDistance <= distance && distance > 100) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${player.name + player.id} WARNING: GOAL MOR FAR AWAY! gotoObj: ${obj.name} held: ${player.heldObject.name} d: ${distance} old-d:${ai.lastGotoObjDistance} ${infos.methodName}');
			ai.addObjectWithHostilePath(obj);
			ai.lastGotoObj = null;
			return false;
		} else if (ServerSettings.DebugAi)
			trace('AAI: ${player.name + player.id} gotoObj: ${obj.name} held: ${player.heldObject.name} d: ${distance} ${infos.methodName}');

		ai.lastGotoObjDistance = distance;
		ai.lastGotoObj = obj;

		return gotoAdv(player, obj.tx, obj.ty, move, checkIfDangerous, infos);
	}

	public static function gotoAdv(player:PlayerInterface, tx:Int, ty:Int, move:Bool = true, checkIfDangerous = true, ?infos:haxe.PosInfos):Bool {
		var startTime = Sys.time();
		var ai = player.getAi();
		var rx = WorldMap.world.transformX(player, tx);
		var ry = WorldMap.world.transformY(player, ty);

		var dist = AiHelper.CalculateDistance(player.tx, player.ty, tx, ty);
		var blockedByAnimal = false;

		if (ai == null) {
			trace('gotoAdv: WARNING Ai is null!');
			return false;
		}

		var considerAnimals = checkIfDangerous && ai.didNotReachFood < 5 && ai.myPlayer.food_store > -1;
		var done = Goto(player, rx, ry, considerAnimals, move);

		if (done) return true;

		// check if blocked only by animals // used for not to consider the object as blocked
		if (considerAnimals && blockedByAnimal == false) {
			// blockedByAnimal = Goto(player, rx + xo, ry + yo, false, false);
			blockedByAnimal = Goto(player, rx, ry, false, false);
		}

		var passedTime = Math.round((Sys.time() - startTime) * 1000);

		// ai.time += passedTime / 1000;

		if (ServerSettings.DebugAi)
			trace('AI: ${player.id} ${player.name} GOTO failed! Ignore ${tx} ${ty} passedTime: $passedTime dist: $dist blockedByAnimal: ${blockedByAnimal}');
		// trace('AI: ${player.id} ${player.name} GOTO failed! Ignore ${tx} ${ty} passedTime: $passedTime');

		if (ai == null) {
			trace('gotoAdv2: WARNING Ai is null!');
			return false;
		}

		if (blockedByAnimal) ai.addHostilePath(tx, ty); else
			ai.addNotReachable(tx, ty);

		// if(blockedByAnimal) trace('blockedByAnimal!!!');

		ai.resetTargets();

		return false;
	}

	public static function TryGoto(playerInterface:PlayerInterface, x:Int, y:Int, considerAnimal:Bool = true):Bool {
		return Goto(playerInterface, x, y, considerAnimal, false);
	}

	/*public static function NewGoto(x:Int, y:Int, playerInterface:PlayerInterface, considerAnimal:Bool = true, move:Bool = true):Bool {
		var player = playerInterface.getPlayerInstance();
		var ai = playerInterface.getAi();
		var radius = 16;

		var blocked = false;
		var px = x - player.x;
		var py = y - player.y;
		var ii = 0;

		if (px == 0 && py == 0) return false; // no need to move

		// search non blocked tile to walk to
		for (i in 0...radius - 4) {
			ii = i;
			// trace('AAI: GOTO: From: ${player.x},${player.y} To: $x $y / FROM ${player.tx()},${player.ty()} To: ${x + player.gx},${y + player.gy}');

			var tmpRad = radius - i;

			if (px > tmpRad - 1) px = tmpRad - 1;
			if (py > tmpRad - 1) py = tmpRad - 1;
			if (px < -tmpRad) px = -tmpRad;
			if (py < -tmpRad) py = -tmpRad;

			// if (playerInterface.isBlocked(player.gx + x, player.gy + y)) trace('GOTO blocked');
			// if (ai != null && ai.isObjectNotReachable(player.gx + x, player.gy + y)) trace('GOTO not reachable');
			// var debugtext = '';
			blocked = false;
			if (playerInterface.isBlocked(px + player.tx, py + player.ty)) blocked = true; // trace('GOTO blocked');
			if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) blocked = true; // trace('GOTO not reachable');
			// if (playerInterface.isBlocked(px + player.tx, py + player.ty)) debugtext = 'blocked';
			// if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) debugtext = 'not reachable';
			if (blocked == false) break;
		}

		if (blocked) {
			// trace('Goto blocked!$debugtext ${player.tx},${player.ty} $px,$py');
			return false;
		}

		// trace('Goto: $px $py');
		var map = new MapCollision(AiHelper.CreateCollisionChunk(playerInterface, considerAnimal));

		// pathing
		// var path = new Pathfinder(map);
		var start = new Coordinate(radius, radius);
		var end = new Coordinate(px + RAD, py + RAD);
		// trace('goto: end $end');
		var newPathfinder = new PathfinderNew(map);
		var paths:Array<Coordinate> = newPathfinder.CreatePath(start, end);

		if (paths == null) {
			// trace('GOTO false ${player.tx},${player.ty} $px,$py $debugtext');

			// since path was cut it might try again if not added here
			if (ai != null) ai.addNotReachable(px + player.tx, py + player.ty);

			// if (onError != null) onError("can not generate path");
			// trace('AAI: ${player.p_id} CAN NOT GENERATE PATH');
			return false;
		}

		/*for(path in paths){
			trace(path);
	}*/
	/*

		var data:Array<Pos> = [];
		paths.shift();
		var tx:Int = start.x;
		var ty:Int = start.y;

		for (path in paths) {
			data.push(new Pos(path.x - tx, path.y - ty));
		}

		var ai = playerInterface.getAi();
		var globalPlayer = cast(player, GlobalPlayerInstance);
		if (move) playerInterface.move(globalPlayer.moveHelper.guessX(), globalPlayer.moveHelper.guessY(), ai.seqNum++, data);

		// trace('GOTO done $debugtext $px $py');

		return true;
	}*/
	// TODO goto uses global coordinates

	public static function Goto(playerInterface:PlayerInterface, x:Int, y:Int, considerAnimal:Bool = true, move:Bool = true):Bool {
		var result = false;
		if (ServerSettings.UseExperimentalMutex) GlobalPlayerInstance.ReleaseMutex();
		Macro.exception(result = GotoHelper(playerInterface, x, y, considerAnimal, move));
		if (ServerSettings.UseExperimentalMutex) GlobalPlayerInstance.AcquireMutex();
		return result;
	}

	public static function GotoHelper(playerInterface:PlayerInterface, x:Int, y:Int, considerAnimal:Bool = true, move:Bool = true):Bool {
		var player = playerInterface.getPlayerInstance();
		var ai = playerInterface.getAi();
		var tryMoveNearestTileFirst = ai == null ? false : ai.tryMoveNearestTileFirst;
		ai.path = null;

		// var goal:Pos;
		// var dest:Pos;
		// var init:Pos;

		// if (player.x == x && player.y == y || moving) return false;
		// set pos
		var px = x - player.x;
		var py = y - player.y;
		var blocked = false;
		var ii = 0;

		// search the first none blocked tile in case target is close to the edge
		var maxEdge = RAD - 2;

		if (px > maxEdge || px < -maxEdge || py > maxEdge || py < -maxEdge) {
			for (i in 0...RAD - 4) {
				ii = i;
				px = x - player.x;
				py = y - player.y;
				if (px > RAD) px = RAD;
				if (py > RAD) py = RAD;
				if (px < -RAD) px = -RAD;
				if (py < -RAD) py = -RAD;

				// trace('AAI: GOTO: From: ${player.x},${player.y} To: $x $y / FROM ${player.tx()},${player.ty()} To: ${x + player.gx},${y + player.gy}');

				if (px == 0 && py == 0) return false; // no need to move

				var tmpRad = RAD - i;

				if (px > tmpRad - 1) px = tmpRad - 1;
				if (py > tmpRad - 1) py = tmpRad - 1;
				if (px < -tmpRad) px = -tmpRad;
				if (py < -tmpRad) py = -tmpRad;

				// if (playerInterface.isBlocked(player.gx + x, player.gy + y)) trace('GOTO blocked');
				// if (ai != null && ai.isObjectNotReachable(player.gx + x, player.gy + y)) trace('GOTO not reachable');
				// var debugtext = '';
				blocked = false;
				if (playerInterface.isBlocked(px + player.tx, py + player.ty)) blocked = true; // trace('GOTO blocked');
				if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) blocked = true; // trace('GOTO not reachable');
				// if (playerInterface.isBlocked(px + player.tx, py + player.ty)) debugtext = 'blocked';
				// if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) debugtext = 'not reachable';
				if (blocked == false) break;
			}

			// trace('${player.name + player.p_id} Goto i: $ii blocked: $blocked ${player.tx},${player.ty} $px,$py');

			if (blocked) {
				// trace('${player.name + player.p_id} Goto blocked! ${player.tx},${player.ty} $px,$py');
				return false;
			}
		}

		// if blocked try if can move half way
		/*if(blocked){
				debugtext = ' half radius';
				px = x - player.x;
				py = y - player.y;

				var tmpRad = Math.round(RAD / 2);

				if (px > tmpRad - 1) px = tmpRad - 1;
				if (py > tmpRad - 1) py = tmpRad - 1;
				if (px < -tmpRad) px = -tmpRad;
				if (py < -tmpRad) py = -tmpRad;
				
				//trace('Goto blocked try with halve radius ${player.tx},${player.ty} $px,$py');
			}

			var blocked = false;
			if (playerInterface.isBlocked(px + player.tx, py + player.ty)) blocked = true; //trace('GOTO blocked');
			if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) blocked = true;//trace('GOTO not reachable');

			if(blocked)
			{
				//trace('Goto blocked!$debugtext ${player.tx},${player.ty} $px,$py');
				return false;
		}*/

		// if(debugtext.length > 0) trace('GOTO $debugtext not blocked');

		// cords
		var start = new Coordinate(RAD, RAD);

		// trace('Goto: $px $py');
		var chunk = AiHelper.CreateCollisionChunk(playerInterface, considerAnimal);

		if (chunk == null) throw new Exception('GOTO: ERROR: Could not generate map!');

		var map = new MapCollision(chunk);

		// pathing
		var debugCreateOldPath = ServerSettings.DebugCreateOldPath;
		var pathOld = debugCreateOldPath ? new Pathfinder(cast map) : null;
		var pathsOld:Array<Coordinate> = null;

		var newPathfinder:PathfinderNew = null;
		var paths:Array<Coordinate> = null;

		// move the end cords
		var tweakX:Int = 0;
		var tweakY:Int = 0;
		var quadDistX = px * px;
		var quadDistY = py * py;
		var distYIsBigger = quadDistX < quadDistY;
		var end = null;

		for (i in 0...5) {
			var ii = i;
			if (tryMoveNearestTileFirst && i == 0) ii = 1;
			if (tryMoveNearestTileFirst && i == 1) ii = 0;

			// first try closest tile
			if (distYIsBigger && ii == 1) ii = 2;
			if (distYIsBigger && ii == 2) ii = 1;

			switch (ii) {
				case 1:
					tweakX = px < 0 ? 1 : -1;
					tweakY = 0;
				case 2:
					tweakX = 0;
					tweakY = py < 0 ? 1 : -1;
				// even if more far away still try since other acceses might be blocked
				case 3:
					tweakX = px < 0 ? -1 : 1;
					tweakY = 0;
					if (px + tweakX > RAD - 1) continue;
					if (px + tweakX < -RAD) continue;
				case 4:
					tweakX = 0;
					tweakY = py < 0 ? -1 : 1;
					if (py + tweakY > RAD - 1) continue;
					if (py + tweakY < -RAD) continue;
			}

			// trace('AAI: ${player.name + player.p_id} i: $i ii: $ii px: $px tweakX: $tweakX py: $py tweakY: $tweakY');

			if (playerInterface.isBlocked(px + player.tx + tweakX, py + player.ty + tweakY)) continue;
			if (ai != null && ai.isObjectNotReachable(px + player.tx + tweakX, py + player.ty + tweakY)) continue;

			end = new Coordinate(px + RAD + tweakX, py + RAD + tweakY);

			// trace('${player.name + player.p_id} goto: end $end');

			pathsOld = pathOld != null ? pathOld.createPath(start, end, MANHATTAN, true) : null;
			// PathfinderNew.TryDifferentPaths(start, end, map);

			newPathfinder = new PathfinderNew(map);
			paths = newPathfinder.CreatePath(start, end);

			if (paths != null) break;
		}

		if (paths == null) {
			// trace('${player.name + player.p_id} GOTO false ${player.tx},${player.ty} $px,$py');

			// since path was cut it might try again if not added here
			// if(ai != null) ai.addNotReachable(px + player.tx, py + player.ty);

			// if (onError != null) onError("can not generate path");
			// trace('AAI: ${player.p_id} CAN NOT GENERATE PATH');
			if (pathsOld != null) trace('${player.name + player.p_id} GOTO: WARNING! new path method could not find a path while old could');
			return false;
		}

		var tx:Int = start.x;
		var ty:Int = start.y;

		var data:Array<Pos> = [];
		paths.shift();
		for (path in paths) {
			data.push(new Pos(path.x - tx, path.y - ty));
		}

		var dataOld:Array<Pos> = [];
		if (pathsOld != null) {
			pathsOld.shift();
			for (path in pathsOld) {
				dataOld.push(new Pos(path.x - tx, path.y - ty));
			}
		}

		if (newPathfinder.usedBruteForceIterations > 0) {
			// && player.p_id == 534970
			if (data.length > 10 && ServerSettings.DebugWritePathToFile) newPathfinder.WriteMapToFile(data, dataOld);

			if (data.length > 10 && ServerSettings.DebugAi)
				trace('${player.name + player.p_id} GOTO: done new: ${end.x} ${end.y} L: ${data.length} $data Iterations: ${newPathfinder.usedBruteForceIterations}');
			if (data.length > 10 && ServerSettings.DebugAi && debugCreateOldPath)
				trace('${player.name + player.p_id} GOTO: done old: ${end.x} ${end.y} L: ${dataOld.length} $dataOld');
		}

		// check if stuck
		/*if (data.length > 0 && ai.lastX == -data[data.length - 1].x && ai.lastY == -data[data.length - 1].y) {
			trace('${player.name + player.p_id} ${ai.lastProfession} GOTO Stuck? $px $py $data');
		}*/

		if (data.length > 0) {
			ai.lastX = data[data.length - 1].x;
			ai.lastY = data[data.length - 1].y;
		}

		if (data.length < 0) {
			trace('WARNING!!! No Path!');
			if (ServerSettings.debug) throw new Exception('No Path!');
			return false;
		}

		var ai = playerInterface.getAi();
		var globalPlayer = cast(player, GlobalPlayerInstance);
		if (move) playerInterface.move(globalPlayer.moveHelper.guessX(), globalPlayer.moveHelper.guessY(), ai.seqNum++, data);
		if (move) ai.path = data;
		// if (ServerSettings.DebugAi) trace('${player.name + player.p_id} GOTO done $px $py $data');

		return true;
	}

	private static function CreateCollisionChunk(playerInterface:PlayerInterface, considerAnimal:Bool):Vector<Bool> {
		var map:Vector<Bool> = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(map = CreateCollisionChunkHelper(playerInterface, considerAnimal));
		WorldMap.world.mutex.release();

		return map;
	}

	private static function CreateCollisionChunkHelper(playerInterface:PlayerInterface, considerAnimal:Bool):Vector<Bool> {
		var player:PlayerInstance = playerInterface.getPlayerInstance();
		var world = playerInterface.getWorld();
		var RAD = MapData.RAD;
		var vector = new Vector<Bool>((RAD * 2) * (RAD * 2));
		var int:Int = -1;
		var minY = player.ty - RAD;
		var minX = player.tx - RAD;
		var maxY = player.ty + RAD;
		var maxX = player.tx + RAD;

		for (y in minY...maxY) {
			for (x in minX...maxX) {
				int++;

				vector[int] = vector[int] || playerInterface.isBlocked(x, y);

				if (considerAnimal == false) continue;

				var obj = world.getObjectHelper(x, y, true);
				if (obj != null && obj.isAnimal() && obj.objectData.damage > 0) {
					// trace('Animal: ${obj.name}');
					var moves = obj.objectData.moves;
					var minYY = y - moves;
					var minXX = x - moves;
					var maxYY = y + moves + 1;
					var maxXX = x + moves + 1;
					if (minYY < minY) minYY = minY;
					if (minXX < minX) minXX = minX;
					if (maxYY > maxY) maxYY = maxY;
					if (maxXX > maxX) maxXX = maxX;

					for (yy in minYY...maxYY) {
						for (xx in minXX...maxXX) {
							var index = (xx - minX) + (yy - minY) * RAD * 2;
							if (index < 0) trace('Animal: $index < ${vector.length} $x,$y => $xx,$yy');
							if (index > vector.length - 1) trace('Animal: $index < ${vector.length} $x,$y => $xx,$yy');
							if (index < 0) index = 0;
							if (index > vector.length - 1) index = vector.length - 1;
							vector[index] = true;
						}
					}
				}

				// var obj = world.getObjectData(x, y);
				// vector[int] = obj.blocksWalking || world.isBiomeBlocking(x, y);
				// if(obj.blocksWalking()) trace('${player.tx()} ${player.ty()} $x $y ${obj.description}');
			}
		}

		// never block own position
		vector[RAD + RAD * 2 * RAD] = false;

		// trace(vector);

		return vector;
	}

	// TODO change so that AI considers high tech by itself
	public static function isHighTech(objId:Int):Bool {
		// if(objId == 62) return true; // Leaf
		// if(objId == 303) return true; // Forge
		if (objId == 2221) return true; // Newcomen Pump with Full Boiler
		if (objId == 2241) return true; // Newcomen Hammer with Full Boiler
		if (objId == 2274) return true; // Newcomen Bore with Full Boiler
		if (objId == 3076) return true; // Kerosene Wick Burner

		return false;
	}

	public static function SearchTransitions(playerInterface:PlayerInterface, objectIdToSearch:Int,
			ignoreHighTech:Bool = false):Map<Int, TransitionForObject> {
		var world = playerInterface.getWorld();
		var transitionsByObject = new Map<Int, TransitionData>();
		var transitionsForObject = new Map<Int, TransitionForObject>();

		var transitionsToProcess = new Array<Array<TransitionData>>();
		var steps = new Array<Int>();
		var wantedObjIds = new Array<Int>();
		var stepsCount = 1;

		transitionsToProcess.push(world.getTransitionByNewTarget(objectIdToSearch));
		transitionsToProcess.push(world.getTransitionByNewActor(objectIdToSearch));

		steps.push(stepsCount);
		steps.push(stepsCount);

		wantedObjIds.push(objectIdToSearch);
		wantedObjIds.push(objectIdToSearch);

		var count = 1;

		var startTime = Sys.time();

		while (transitionsToProcess.length > 0) {
			var transitions = transitionsToProcess.shift();
			stepsCount = steps.shift();
			var wantedObjId = wantedObjIds.shift();

			for (trans in transitions) {
				// if(trans.actorID == -1) continue; // TODO time
				if (trans.targetID == -1) continue; // TODO -1 target

				// ignore high tech stuff if most likely not needed
				if (ignoreHighTech && isHighTech(wantedObjId)) {
					trace('TEST1 IGNORE AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDescription(true));
					continue;
				}

				if (ShouldDebug(trans)) trace('TEST1 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDescription(true));

				if (trans.actorID == wantedObjId || trans.actorID == objectIdToSearch) continue;

				if (ShouldDebug(trans)) trace('TEST2 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDescription(true));
				if (trans.targetID == wantedObjId || trans.targetID == objectIdToSearch) continue;

				if (ShouldDebug(trans)) trace('TEST3 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDescription(true));

				// TODO this might exclude needed tasks
				// Allow transition if new actor or target is closer to wanted object
				/*var tmpActor = transitionsForObject[trans.actorID];
					var actorSteps = tmpActor != null ? tmpActor.steps : 10000;
					var tmpNewActor = transitionsForObject[trans.newActorID];
					var newActorSteps = tmpNewActor != null ? tmpNewActor.steps : 10000;

					var tmpTarget = transitionsForObject[trans.targetID];
					var targetSteps = tmpTarget != null ? tmpTarget.steps : 10000;
					var tmpNewTarget = transitionsForObject[trans.newTargetID];
					var newTargetSteps = tmpNewTarget != null ? tmpNewTarget.steps : 10000;

					if (trans.newActorID == objectIdToSearch) newActorSteps = 0;
					if (trans.newTargetID == objectIdToSearch) newTargetSteps = 0;

					// AI get stuck with <3288> actorSteps: 2 newActorSteps: 10000 targetSteps: 10000 newTargetSteps: 3 <67> + <96> = <0> + <3288>
					// if(actorSteps <= newActorSteps && targetSteps <= newTargetSteps) continue; // nothing is won
					if (actorSteps + targetSteps <= newActorSteps + newTargetSteps) continue; // nothing is won


					if (ShouldDebug(trans))
						trace('TEST4 AI craft steps: $stepsCount WANTED: <${wantedObjId}> actorSteps: $actorSteps newActorSteps: $newActorSteps targetSteps: $targetSteps newTargetSteps: $newTargetSteps '
						+ trans.getDesciption(true));
				 */

				// trace('TEST4 AI craft steps: $stepsCount WANTED: <${wantedObjId}> actorSteps: $actorSteps newActorSteps: $newActorSteps targetSteps: $targetSteps newTargetSteps: $newTargetSteps ' + trans.getDesciption(true));

				if (trans.actorID > 0 && transitionsByObject.exists(trans.actorID) == false) {
					// if(ShouldDebug(trans)) trace('TEST5 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));

					transitionsToProcess.push(world.getTransitionByNewTarget(trans.actorID));
					transitionsToProcess.push(world.getTransitionByNewActor(trans.actorID));

					steps.push(stepsCount + 1);
					steps.push(stepsCount + 1);

					wantedObjIds.push(trans.actorID);
					wantedObjIds.push(trans.actorID);
				}

				if (trans.targetID > 0 && transitionsByObject.exists(trans.targetID) == false) {
					// if(ShouldDebug(trans)) trace('TEST6 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));

					transitionsToProcess.push(world.getTransitionByNewTarget(trans.targetID));
					transitionsToProcess.push(world.getTransitionByNewActor(trans.targetID));

					steps.push(stepsCount + 1);
					steps.push(stepsCount + 1);

					wantedObjIds.push(trans.targetID);
					wantedObjIds.push(trans.targetID);
				}

				if (trans.actorID > 0) transitionsByObject[trans.actorID] = trans;
				if (trans.targetID > 0) transitionsByObject[trans.targetID] = trans;

				if (trans.actorID > 0) AddTransition(transitionsForObject, trans, trans.actorID, wantedObjId, stepsCount);
				if (trans.targetID > 0) AddTransition(transitionsForObject, trans, trans.targetID, wantedObjId, stepsCount);

				count++;
			}

			// if(count < 10000) trans.traceTransition('AI stepsCount: $stepsCount count: $count:', true);
			// if(count > 1000) break; // TODO remove
		}

		if (ServerSettings.DebugAiCrafting) trace('AI trans search: $count transtions found! ${Sys.time() - startTime}');

		/*for(key in transitionsForObject.keys())            
			{
				var trans = transitionsForObject[key].getDesciption();

				trace('AI Search: ${trans}');
		}*/

		return transitionsForObject;

		// var transitionsByOjectKeys = [for(key in transitionsByObject.keys()) key];
	}

	public static function ShouldDebug(trans:TransitionData):Bool {
		var debugObjId = ServerSettings.DebugAiCraftingObject;

		if (trans.actorID == debugObjId) return true;
		if (trans.targetID == debugObjId) return true;
		if (trans.newActorID == debugObjId) return true;
		return trans.newTargetID == debugObjId;
	}

	private static function AddTransition(transitionsForObject:Map<Int, TransitionForObject>, transition:TransitionData, objId:Int, wantedObjId:Int,
			steps:Int) {
		var transitionForObject = transitionsForObject[objId];

		if (transitionForObject == null) {
			transitionForObject = new TransitionForObject(objId, steps, wantedObjId, transition);
			transitionForObject.steps = steps;
			transitionForObject.bestTransition = transition;
			transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));
			// transitionForObject.transitions.push(transition);

			transitionsForObject[objId] = transitionForObject;

			return;
		}

		if (transitionForObject.steps > steps) {
			transitionForObject.steps = steps;
			transitionForObject.bestTransition = transition;
		}

		transitionForObject.transitions.push(new TransitionForObject(objId, steps, wantedObjId, transition));
		// transitionForObject.transitions.push(transition);
	}

	public static function DisplayCloseDeadlyAnimals(player:PlayerInterface, searchDistance:Int = 6) {
		GetCloseDeadlyAnimal(player, searchDistance, true);
	}

	public static function GetCloseDeadlyAnimal(player:PlayerInterface, searchDistance:Int = 6, display:Bool = false):ObjectHelper {
		var result = null;

		WorldMap.world.mutex.acquire();
		Macro.exception(result = GetCloseDeadlyAnimalHelper(player, searchDistance, display));
		WorldMap.world.mutex.release();

		// TODo throw exception if failed

		return result;
	}

	private static function GetCloseDeadlyAnimalHelper(player:PlayerInterface, searchDistance:Int = 6, display:Bool = false):ObjectHelper {
		// AiHelper.GetClosestObject
		var world = WorldMap.world;
		var playerInst = player.getPlayerInstance();
		var baseX = playerInst.tx;
		var baseY = playerInst.ty;

		var bestObj = null;
		var bestDist:Float = searchDistance * searchDistance;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				var obj = world.getObjectHelper(tx, ty, true);

				if (player.isAnimalNotDeadlyForMe(obj)) continue;
				/*if (obj == null) continue;
					if (obj.objectData.deadlyDistance == 0) continue;
					if (obj.objectData.damage == 0) continue;
					if (obj.isAnimal() == false) continue;
				 */

				var dist = AiHelper.CalculateQuadDistanceToObject(player, obj);

				if (display) if (dist > 16) cast(player, GlobalPlayerInstance).connection.send(ClientTag.LOCATION_SAYS,
					['${obj.tx - player.gx} ${obj.ty - player.gy} !']);

				if (dist > bestDist) continue;
				// var moveQuadDist = Math.pow(obj.objectData.moves + 1, 2);
				// trace('GetCloseDeadlyAnimal: $dist <= $bestDist moveQuadDist: $moveQuadDist ${obj.name}');
				// if (dist > Math.pow(obj.objectData.moves + 1, 2)) continue;
				if (dist > Math.pow(obj.objectData.moves, 2)) continue;

				bestDist = dist;
				bestObj = obj;
			}
		}

		return bestObj;
	}

	public static function GetCloseDeadlyPlayer(playerInter:PlayerInterface, searchDistance:Int = 8) {
		var bestPlayer = null;

		GlobalPlayerInstance.AcquireMutex();
		Macro.exception(bestPlayer = GetCloseDeadlyPlayerHelper(playerInter, searchDistance));
		GlobalPlayerInstance.ReleaseMutex();

		return bestPlayer;
	}

	private static function GetCloseDeadlyPlayerHelper(playerInter:PlayerInterface, searchDistance:Int = 8) {
		var player = cast(playerInter, GlobalPlayerInstance);
		var bestPlayer = null;
		var bestDist:Float = searchDistance * searchDistance;

		// if (player.angryTime > 4) return null;
		// TODO consider ally that can soon attack player

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			// if (p.isHoldingWeapon() == false && p.lostCombatPrestige < 4) continue;
			// if (p.angryTime > 4) continue;
			var dangerous = false;

			// consider dangerous if exiled and close to my home
			if (p.isExiledByAnyLeaderFrom(player)) {
				var quadDistanceToHome = CalculateQuadDistanceToObject(p, player.home);
				if (quadDistanceToHome < 1600) dangerous = true;
			}

			if (p.heldObject.isBloody()) dangerous = true;
			if (p.lostCombatPrestige > 4) dangerous = true;
			// TODO this is less agro but would allow to shoot first on a agro AI
			if (p.isHoldingWeapon() && (p.angryTime < 4 || player.angryTime < 4)) dangerous = true;
			// if (p.isHoldingWeapon() == false || (p.angryTime > 4 && player.angryTime > 4))) continue;
			if (dangerous == false) continue;
			if (p.isFriendly(player)) continue;

			var dist = AiHelper.CalculateDistanceToPlayer(player, p);
			if (p.lostCombatPrestige > 1) dist /= (15 + p.lostCombatPrestige) / 10;

			if (dist > bestDist) continue;

			bestDist = dist;
			bestPlayer = p;
		}

		return bestPlayer;
	}

	public static function GetCloseStarvingPlayer(playerInter:PlayerInterface, searchDistance:Int = 30) {
		var bestPlayer = null;

		GlobalPlayerInstance.AcquireMutex();
		Macro.exception(bestPlayer = GetCloseStarvingPlayerHelper(playerInter, searchDistance));
		GlobalPlayerInstance.ReleaseMutex();

		return bestPlayer;
	}

	private static function GetCloseStarvingPlayerHelper(player:PlayerInterface, searchDistance:Int = 40) {
		var globalplayer = cast(player, GlobalPlayerInstance); // TODO find better way / maybe use globalplayer also for client
		var bestPlayer:GlobalPlayerInstance = null;

		var maxDist = searchDistance * searchDistance;
		var bestQuadHungry:Float = 0;
		var minQuadHungry = 0.01;
		var isFertile = player.isFertile();
		var myPlayerIsNobleOrMore = cast(globalplayer.lineage.prestigeClass, Int) > cast(PrestigeClass.Commoner, Int);
		var ai = player.getAi();
		var isSmith = ai == null ? false : ai.profession['Smith'] > 0;

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.heldByPlayer != null) continue;
			if (isFertile && p.age < ServerSettings.MaxChildAgeForBreastFeeding) continue;

			var targetAi = p.getAi();
			var isTargetSmith = targetAi == null ? false : targetAi.profession['Smith'] > 0;
			var isNobleOrMore = cast(p.lineage.prestigeClass, Int) > cast(PrestigeClass.Commoner, Int);

			// && isNobleOrMore == false
			// dont feed Ai that can eat except it is a smith
			if (p.isAi() && p.age > ServerSettings.MinAgeToEat && p.isWounded() == false && p.hasYellowFever() == false && isTargetSmith == false) continue;

			var classFood = isNobleOrMore ? p.lineage.prestigeClass * 4 : p.lineage.prestigeClass * 2;
			var considerHungry = Math.min(classFood, p.food_store_max * 0.6);
			var hungry = considerHungry - p.food_store;
			var isAlly = p.isAlly(globalplayer);

			if (isAlly == false && p.angryTime < ServerSettings.CombatAngryTimeBeforeAttack / 2) continue;
			if (isAlly == false && p.lostCombatPrestige > 4) continue;
			if (p.isCloseRelative(globalplayer) == false
				|| player.getFollowPlayer() == p) hungry = hungry / 2 - 0.25; // prefer close relative
			if (isAlly == false) hungry = hungry / 2 - 0.2; // prefer ally
			if (isSmith) hungry = hungry / 2 - 0.2; // dont be distracted from smithing
			// if(isTargetSmith) hungry = hungry * 2 + 0.5; // feed smith
			// if (p.isAi() && isNobleOrMore == false) hungry / 2 - 0.2; // prefer Ai only if noble
			// if (myPlayerIsNobleOrMore && p.isAi()) hungry / 2 - 0.2; // if myPlayer is noble feed less AI
			if (hungry < 0) continue;

			var dist = AiHelper.CalculateDistanceToPlayer(player, p) + 1;
			if (dist > maxDist) continue;

			var quadHungry = Math.pow(hungry, 2) / dist;
			if (quadHungry < bestQuadHungry) continue;
			// trace('${p.name} class: ${p.lineage.prestigeClass} dist: $dist food: ${Math.ceil(p.food_store * 10) / 10} hungry: ${Math.ceil(hungry * 10) / 10} quadHungry: ${Math.ceil(quadHungry * 1000) / 1000}');
			if (quadHungry < minQuadHungry) continue;

			bestQuadHungry = quadHungry;
			bestPlayer = p;
		}

		return bestPlayer;
	}

	public static function GetCloseHungryChild(playerInter:PlayerInterface, searchDistance:Int = 40) {
		var bestPlayer = null;

		GlobalPlayerInstance.AcquireMutex();
		Macro.exception(bestPlayer = GetCloseHungryChildHelper(playerInter, searchDistance));
		GlobalPlayerInstance.ReleaseMutex();

		return bestPlayer;
	}

	private static function GetCloseHungryChildHelper(mother:PlayerInterface, searchDistance:Int = 40) {
		var bestPlayer:GlobalPlayerInstance = null;
		var maxDist = searchDistance * searchDistance;
		var bestQuadHungry:Float = 0;
		var considerHungry = 2.5;
		var minQuadHungry = 0.01;

		// TODO consider hits

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.age > ServerSettings.MaxChildAgeForBreastFeeding) continue;
			if (p.heldByPlayer != null) continue;

			var hungry = considerHungry - p.food_store;
			if (p.mother != mother) hungry = hungry / 2 - 0.25; // own children count more
			if (p.age > ServerSettings.MinAgeToEat && p.hasYellowFever() == false) hungry -= 0.5;
			if (hungry < 0) continue;

			var dist = AiHelper.CalculateDistanceToPlayer(mother, p) + 1;
			if (dist > maxDist) continue;
			var quadHungry = Math.pow(hungry, 3) / dist;
			if (quadHungry < minQuadHungry) continue;
			if (quadHungry < bestQuadHungry) continue;

			bestQuadHungry = quadHungry;
			bestPlayer = p;
		}

		return bestPlayer;
	}

	public static function GetMostDistantOwnChild(mother:PlayerInterface, minDist:Int = 10, searchDistance:Int = 50) {
		var worstPlayer = null;

		GlobalPlayerInstance.AcquireMutex();
		Macro.exception(worstPlayer = GetMostDistantOwnChildHelper(mother, minDist, searchDistance));
		GlobalPlayerInstance.ReleaseMutex();

		return worstPlayer;
	}

	private static function GetMostDistantOwnChildHelper(mother:PlayerInterface, minDist:Int = 10, searchDistance:Int = 50) {
		// var player = cast(playerInter, GlobalPlayerInstance);
		var worstPlayer:GlobalPlayerInstance = null;
		var worstDist:Float = 0;
		var maxQuadDist = searchDistance * searchDistance;
		var minQuadDist = minDist * minDist;

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.age > ServerSettings.MinAgeToEat) continue;
			if (p.heldByPlayer != null) continue;
			if (p.mother != mother) continue;

			var dist = AiHelper.CalculateDistanceToPlayer(mother, p);

			if (dist > maxQuadDist) continue;
			if (dist < minQuadDist) continue;
			if (dist < worstDist) continue;

			worstDist = dist;
			worstPlayer = p;
		}

		return worstPlayer;
	}

	public static function SearchNewHome(player:PlayerInterface):ObjectHelper {
		var world = WorldMap.world;
		var bestHome = null;
		var bestDistance = Math.pow(80, 2);
		var ovens = [for (obj in WorldMap.world.ovens) obj];

		// trace('AAI: ${player.name + player.id} searchNewHome: ovens: ${ovens.length}');

		for (possibleHome in ovens) {
			if (ObjectData.IsOven(possibleHome.id) == false) continue;

			var floorId = world.getFloorId(possibleHome.tx, possibleHome.ty);
			var originalBiomeId = world.getOriginalBiomeId(possibleHome.tx, possibleHome.ty);
			// TODO check loved biome
			// For ginger rock biome should be ok
			if (floorId < 1 && originalBiomeId == BiomeTag.SWAMP) continue;

			var quadDistance = AiHelper.CalculateQuadDistanceToObject(player, possibleHome);
			if (floorId > 0) quadDistance /= 2;
			if (quadDistance >= bestDistance) continue;

			bestDistance = quadDistance;
			bestHome = possibleHome;
		}

		if (bestHome != null) {
			// myPlayer.home = bestHome;
			// trace('AAI: ${player.name + player.id} searchNewHome dist: $bestDistance ${bestHome != null}');
		}

		return bestHome;
	}

	/*
		public static function GetCloseFire(player:PlayerInterface) : ObjectHelper {
			var firePlace= AiHelper.GetClosestObjectById(player, 346, null, 30); // 346 Large Slow Fire
			if(firePlace == null) firePlace = AiHelper.GetClosestObjectById(player, 83, null, 30); // 83 Large Fast Fire 
			if(firePlace == null) firePlace = AiHelper.GetClosestObjectById(player, 82, null, 30); // 82 Fire
			if(firePlace == null) firePlace = AiHelper.GetClosestObjectById(player, 85, null, 30); // 85 Hot Coals
			return firePlace;
	}*/
	public static function GetCloseFire(player:PlayerInterface, maxdist = 20):ObjectHelper {
		// AI: consider first large fast fire for heating purpose as home fire
		var firePlace = AiHelper.GetClosestObjectToHome(player, 83, maxdist); // 83 Large Fast Fire
		if (firePlace == null) firePlace = AiHelper.GetClosestObjectToHome(player, 346, maxdist); // 346 Large Slow Fire
		if (firePlace == null) firePlace = AiHelper.GetClosestObjectToHome(player, 82, maxdist); // 82 Fire
		if (firePlace == null) firePlace = AiHelper.GetClosestObjectToHome(player, 85, maxdist); // 85 Hot Coals
		return firePlace;
	}

	// time routine
	// update loop
	// map
}

class TransitionForObject {
	public var objId:Int;
	public var wantedObjId:Int;
	public var wantedObjs = new Array<TransitionForObject>();
	public var steps:Int;

	public var bestTransition:TransitionData;
	public var transitions:Array<TransitionForObject> = [];

	public var closestObject:ObjectHelper;
	public var closestObjectDistance:Float;
	public var closestObjectPlayerIndex:Float; // if object is held
	public var count = 0;

	public var secondObject:ObjectHelper; // in case you need two object like using two milkeed
	public var secondObjectDistance:Float;

	public var craftActor:Null<ObjectHelper> = null;
	public var craftTarget:Null<ObjectHelper> = null;
	public var craftFrom:TransitionForObject;
	public var craftTransFrom:TransitionData;
	public var bestCraftSteps:Int = -1;
	public var bestCraftDistance:Float = -1;

	public var isDone = false;
	public var usePile = false; // if true this object is the piled object

	// public var craftSteps:Int;

	public function new(objId:Int, steps:Int, wantedObjId:Int, transition:TransitionData) {
		this.objId = objId;
		this.wantedObjId = wantedObjId;
		this.steps = steps;
		this.bestTransition = transition;
	}

	public function getDesciption():String {
		var description = 'objId: $objId wantedObjId: $wantedObjId steps: $steps trans: ' + bestTransition.getDescription(true);
		return description;
	}
}

class IntemToCraft {
	public var ai:AiBase;
	public var itemToCraft:ObjectData;
	public var count:Int = 0; // how many items to craft
	public var countDone:Int = 0; // allready crafted
	public var countTransitionsDone:Int = 0; // transitions done while crafting
	public var done:Bool = false; // transitions done while crafting
	public var searchRadius = 0;
	public var maxSearchRadius = 0;
	public var searchCurrentPosition = true; // search for items on current player position.

	public var transActor:ObjectHelper = null;
	public var transTarget:ObjectHelper = null;

	public var transitionsByObjectId:Map<Int, TransitionForObject>;

	public var bestDistance:Float = 99999999999999999999999;
	public var bestTransition:TransitionData = null;

	public var craftingList = new Array<Int>(); // is not a complete list
	public var craftingTransitions = new Array<TransitionData>(); // is not a complete list

	public var startLocation:ObjectHelper = null;

	public var lastActorId = -1;
	public var lastTargetId = -1;
	public var lastNewActorId = -1;
	public var lastNewTargetId = -1;

	public function new() {
		itemToCraft = ObjectData.getObjectData(0);
	}

	public function clearTransitionsByObjectId() {
		// reset objects so that it can be filled again
		for (trans in transitionsByObjectId) {
			trans.count = 0;
			trans.closestObject = null;
			trans.closestObjectDistance = -1;
			trans.closestObjectPlayerIndex = -1;

			trans.secondObject = null;
			trans.closestObjectDistance = -1;

			trans.craftActor = null;
			trans.craftTarget = null;
			trans.isDone = false;
			trans.usePile = false;
			trans.wantedObjs = new Array<TransitionForObject>();
			trans.bestCraftSteps = -1;
			trans.bestCraftDistance = -1;
		}
	}
}

class Village {
	public var fireKeeper:GlobalPlayerInstance = null;
}
