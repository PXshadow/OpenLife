package openlife.auto;

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

	public static function CalculateQuadDistanceHelper(baseX:Int, baseY:Int, toX:Int, toY:Int):Float {
		return (toX - baseX) * (toX - baseX) + (toY - baseY) * (toY - baseY);
	}

	// does consider round map
	public static function CalculateDistance(baseX:Int, baseY:Int, toX:Int, toY:Int):Float {
		var diffX = toX - baseX;
		var diffY = toY - baseY;
		var width = WorldMap.world.width;
		var height = WorldMap.world.height;

		if(diffX > width / 2) diffX -= width; // consider that world is round
		else if(diffX < -width / 2) diffX += width; // consider that world is round
		if(diffY > height / 2) diffY -= height; // consider that world is round
		else if(diffY < -height / 2) diffY += height; // consider that world is round

		return diffX * diffX + diffY * diffY;
	}

	public static function GetClosestObjectOwnedByPlayer(playerInterface:PlayerInterface, searchDistance:Int = 10):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, false, true));
	}

	public static function GetClosestHeatObject(playerInterface:PlayerInterface, searchDistance:Int = 2):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, true));
	}

	// searchDistance old: 16
	public static function GetClosestObjectById(playerInterface:PlayerInterface, objId:Int, ignoreObj:ObjectHelper = null, searchDistance:Int = 40):ObjectHelper {
		var objData = ObjectData.getObjectData(objId);
		return GetClosestObject(playerInterface, objData, searchDistance, ignoreObj);
	}

	private static function GetName(objId:Int):String {
		return ObjectData.getObjectData(objId).name;
	}

	// searchDistance old: 16
	public static function GetClosestObject(playerInterface:PlayerInterface, objDataToSearch:ObjectData, searchDistance:Int = 40,
			ignoreObj:ObjectHelper = null, findClosestHeat:Bool = false, ownedByPlayer:Bool = false):ObjectHelper {
		// var RAD = ServerSettings.AiMaxSearchRadius
		//if(objDataToSearch != null) trace('GetClosestObject: ${objDataToSearch.name} dis: $searchDistance ignoreObj: ${ignoreObj != null}');
		
		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var objId = objDataToSearch == null ? -1 : objDataToSearch.parentId; 
		var searchEmptyPlace = ai != null && objDataToSearch != null && objDataToSearch.parentId == 0;
		// 1101 Fertile Soil Pile // 1137 Bowl of Soil // 356 Basket of Bones
		var searchNotFlooredPlace = ai != null && objDataToSearch != null && (objId == 1101 || objId == 1137 || objId == 356); 
		var baseX = player.tx;
		var baseY = player.ty;
		var closestBadPlaceforDrop = null;
		var bestDistanceToBadPlaceforDrop = 0.0;
		var closestObject = null;
		var bestDistance = 0.0;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				if (ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;

				// findClosestHeat == false / since its jused for player temerpature where it does not matter if blocked. Also otherwiese a mutex would be missing since isObjectNotReachable can be used from the AI at same time
				if (ai != null && findClosestHeat == false && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && findClosestHeat == false && ai.isObjectWithHostilePath(tx, ty)) continue;

				var objData = world.getObjectDataAtPosition(tx, ty);

				if (ownedByPlayer && objData.isOwned == false) continue; // cannot have a owner

				if (findClosestHeat && objData.heatValue == 0) continue;

				if (ownedByPlayer
					|| findClosestHeat
					|| objData.parentId == objDataToSearch.parentId) // compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs
				{
					var objDataBelow = world.getObjectDataAtPosition(tx, ty - 1);
					if (searchEmptyPlace && objDataBelow.isTree()) continue;

					if(searchNotFlooredPlace){
						var floorId = world.getFloorId(tx, ty);
						if(floorId > 0) continue;
					}

					var obj = world.getObjectHelper(tx, ty);

					if (ownedByPlayer && obj.isOwnedByPlayer(playerInterface) == false) continue;

					var distance = AiHelper.CalculateQuadDistanceToObject(playerInterface, obj);

					if(searchEmptyPlace && IsBadBiomeForDrop(tx, ty)){
						// try not drop stuff in water or mountain
						if (closestBadPlaceforDrop == null || distance < bestDistanceToBadPlaceforDrop) {
							closestBadPlaceforDrop = obj;
							bestDistanceToBadPlaceforDrop = distance;
							//trace('Bad Empty Space For drop distance: $distance');
						}
					}
					else{
						if (closestObject == null || distance < bestDistance) {
							closestObject = obj;
							bestDistance = distance;
							//if(searchEmptyPlace) trace('Empty Space For drop distance: $distance');
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

		if(closestObject == null && searchEmptyPlace) playerInterface.say('No Empty tile found!'); 

		return closestObject != null ? closestObject : closestBadPlaceforDrop;
	}

	public static function IsBadBiomeForDrop(tx:Int, ty:Int) : Bool{
		var biomeId = WorldMap.world.getBiomeId(tx, ty);
		if(biomeId == PASSABLERIVER || biomeId == OCEAN || biomeId == RIVER || biomeId == SNOWINGREY) return true;

		return false;
	}

	public static function GetCloseClothings(playerInterface:PlayerInterface, searchDistance:Int = 8):Array<ObjectHelper> {
		// var RAD = ServerSettings.AiMaxSearchRadius
		//if(objDataToSearch != null) trace('GetClosestObject: ${objDataToSearch.name} dis: $searchDistance ignoreObj: ${ignoreObj != null}');
		
		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var clothings = new Array<ObjectHelper>();
		//var bestDistance = 0.0;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {

				if (ai != null && ai.isObjectNotReachable(tx, ty)) continue;
				if (ai != null && ai.isObjectWithHostilePath(tx, ty)) continue;

				var objData = world.getObjectDataAtPosition(tx, ty);

				if (objData.clothing.charAt(0) != "n") // compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs
				{
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

		//if (ServerSettings.DebugAi) trace('AI: ${obj.description} ${obj.id} numberOfUses: ${obj.numberOfUses} originalFoodValue: $originalFoodValue');

		return originalFoodValue > 0;
		// if(originalFoodValue < 0) return false;
		// if(player.food_store_max - player.food_store < Math.ceil(originalFoodValue / 4)) return false;
		// return true;
	}

	public static function SearchBestFood(player:PlayerInterface, feedingPlayer:PlayerInterface = null, radius:Int = 40):ObjectHelper {
		var bestFood = null;

		// TODO might need player mutex because: player.getCountEaten(foodId);

		GlobalPlayerInstance.AcquireMutex();
		//WorldMap.world.mutex.acquire();
		Macro.exception(bestFood = SearchBestFoodHelper(player, feedingPlayer, radius));
		//WorldMap.world.mutex.release();
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

		if (player.food_store < 0.5) starvingFactor = 2;
		if (player.food_store < -1) starvingFactor = 1.5;
		if (player.food_store < -1.5) starvingFactor = 1.2;

		//var biome = WorldMap.worldGetBiomeId(player.tx, player.ty);
		//var originalBiomeTemperature = Biome.getBiomeTemperature(biome);

		for (ty in baseY - radius...baseY + radius) {
			for (tx in baseX - radius...baseX + radius) {
				var objData = world.getObjectDataAtPosition(tx, ty);

				if (objData.id == 0) continue;
				if (objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata
				//if (feedOther && objData.id == 837) continue; // dont feed Psilocybe Mushroom to others
				if(ai != null && ai.isObjectNotReachable(tx ,ty)) continue;

				// var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
				// trace('search food $tx, $ty: foodvalue: ${objData.foodValue} bestdistance: $bestDistance distance: $distance ${obj.description}');

				// var tmp = ObjectData.getObjectData(31);
				// trace('berry food: ${tmp.foodValue}');

				var objData = objData.foodFromTarget == null ? objData : objData.foodFromTarget;
				var originalFoodValue = objData.foodValue;
				var foodId = objData.getFoodId();
				var foodValue:Float = originalFoodValue;

				if (foodValue <= 0) continue;
				if(feedOther && player.canFeedToMeObj(objData) == false) continue;
				if (player.food_store_max - player.food_store < Math.ceil(originalFoodValue / 4)) continue;				

				var countEaten = player.getCountEaten(foodId);
				var quadDistance = 16 + AiHelper.CalculateDistance(baseX, baseY, tx, ty);
				if(feedingPlayer != null) quadDistance += 1 + AiHelper.CalculateDistance(feedingPlayer.tx, feedingPlayer.ty, tx, ty);

				foodValue -= countEaten;
				var isYum = countEaten < ServerSettings.YumBonus;
				var isSuperMeh = foodValue < originalFoodValue / 2; // can eat if food_store < 0
				// trace('search food: best $bestDistance dist $distance ${obj.description}');

				if (isYum) foodValue *= starvingFactor;
				if (isSuperMeh) foodValue = originalFoodValue / starvingFactor;
				if (isSuperMeh && player.food_store > 3) foodValue = 0;
				if (foodId == player.getCraving()) foodValue *= starvingFactor;
				//if (foodId == player.getCraving()) foodValue *= Math.pow(starvingFactor, 2);

				if (quadDistance < 1) quadDistance = 1;
				// distance = Math.sqrt(distance);

				if (bestFood == null || foodValue / quadDistance > bestFoodValue / bestDistance) {
					var obj = world.getObjectHelper(tx, ty);
					
					if (ai != null) {
						if (quadDistance > 4 && IsDangerous(player, obj)) continue;
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
		return gotoAdv(player, obj.tx, obj.ty, move, checkIfDangerous, infos);
	}

	public static function gotoAdv(player:PlayerInterface, tx:Int, ty:Int, move:Bool = true, checkIfDangerous = true, ?infos:haxe.PosInfos):Bool {
		var startTime = Sys.time();
		var ai = player.getAi();
		var rand = 0;
		var rx = WorldMap.world.transformX(player, tx);
		var ry = WorldMap.world.transformY(player, ty);
		//var x = tx - player.gx;
		//var y = ty - player.gy;

		var dist = AiHelper.CalculateDistance(player.tx, player.ty, tx, ty);
		var blockedByAnimal = false;

		if(ai == null){
			trace('gotoAdv: WARNING Ai is null!');
			return false;
		}

		var considerAnimals = checkIfDangerous && ai.didNotReachFood < 5 && ai.myPlayer.food_store > -1;

		for (i in 0...5) {
			var xo = 0;
			var yo = 0;

			if (rand > 4) break;

			if (rand == 1) xo = 1;
			if (rand == 2) yo = -1;
			if (rand == 3) xo = -1;
			if (rand == 4) yo = 1;

			rand++;

			if (player.isBlocked(tx + xo, ty + yo)) continue;
			if (ai.isObjectNotReachable(tx + xo, ty + yo)) continue;

			var done = Goto(player, rx + xo, ry + yo, considerAnimals, move);

			var passedTime = (Sys.time() - startTime) * 1000;

			if (done){
				ai.time += passedTime / 1000;
				return true;
			} 

			// check if blocked only by animals // used for not to consider the object as blocked
			if(considerAnimals && blockedByAnimal == false)
			{
				blockedByAnimal = Goto(player, rx + xo, ry + yo, false, false);
			}

			if (passedTime > ServerSettings.GotoTimeOut) {
				trace('AI: ${player.name + player.id} GOTO failed after $i because of timeout ${Math.round(passedTime)}ms! Ignore ${tx} ${ty} dist: ${Math.round(Math.sqrt(dist))} ${infos.methodName}');
				break;
			}
		}

		var passedTime = (Sys.time() - startTime) * 1000;
		
		ai.time += passedTime / 1000;

		if(ServerSettings.DebugAiGoto) trace('AI: ${player.id} ${player.name} GOTO failed! Ignore ${tx} ${ty} passedTime: $passedTime dist: $dist');
		//trace('AI: ${player.id} ${player.name} GOTO failed! Ignore ${tx} ${ty} passedTime: $passedTime');

		if(ai == null){
			trace('gotoAdv2: WARNING Ai is null!');
			return false;
		}
		
		if(blockedByAnimal) ai.addHostilePath(tx, ty);
		else ai.addNotReachable(tx, ty);

		//if(blockedByAnimal) trace('blockedByAnimal!!!');

		ai.resetTargets();

		return false;
	}

	public static function TryGoto(playerInterface:PlayerInterface, x:Int, y:Int, considerAnimal:Bool = true):Bool {
		return Goto(playerInterface, x, y, considerAnimal, false);
	}

	// TODO goto uses global coordinates
	public static function Goto(playerInterface:PlayerInterface, x:Int, y:Int, considerAnimal:Bool = true, move:Bool = true):Bool {
		var player = playerInterface.getPlayerInstance();
		var ai = playerInterface.getAi();

		// var goal:Pos;
		// var dest:Pos;
		// var init:Pos;

		// if (player.x == x && player.y == y || moving) return false;
		// set pos
		var px = 0;
		var py = 0;
		var blocked = false;
		var ii = 0;

		for(i in 0...RAD-4){
			ii = i;
			px = x - player.x;
			py = y - player.y;

			// trace('AAI: GOTO: From: ${player.x},${player.y} To: $x $y / FROM ${player.tx()},${player.ty()} To: ${x + player.gx},${y + player.gy}');
			
			if (px == 0 && py == 0) return false; // no need to move

			var tmpRad = RAD - i;

			if (px > tmpRad - 1) px = tmpRad - 1;
			if (py > tmpRad - 1) py = tmpRad - 1;
			if (px < -tmpRad) px = -tmpRad;
			if (py < -tmpRad) py = -tmpRad;

			//if (playerInterface.isBlocked(player.gx + x, player.gy + y)) trace('GOTO blocked');
			//if (ai != null && ai.isObjectNotReachable(player.gx + x, player.gy + y)) trace('GOTO not reachable');
			//var debugtext = '';
			blocked = false;
			if (playerInterface.isBlocked(px + player.tx, py + player.ty)) blocked = true; //trace('GOTO blocked');
			if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) blocked = true;//trace('GOTO not reachable');
			//if (playerInterface.isBlocked(px + player.tx, py + player.ty)) debugtext = 'blocked';
			//if (ai != null && ai.isObjectNotReachable(px + player.tx, py + player.ty)) debugtext = 'not reachable';
			if(blocked == false) break;
		}

		//trace('Goto i: $ii blocked: $blocked ${player.tx},${player.ty} $px,$py');

		if(blocked)
		{
			//trace('Goto blocked!$debugtext ${player.tx},${player.ty} $px,$py');
			return false;
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

		//if(debugtext.length > 0) trace('GOTO $debugtext not blocked');
		
		// cords
		var start = new Coordinate(RAD, RAD);

		// trace('Goto: $px $py');

		var map = new MapCollision(AiHelper.CreateCollisionChunk(playerInterface, considerAnimal));
		// pathing
		var path = new Pathfinder(cast map);
		var paths:Array<Coordinate> = null;
		// move the end cords
		var tweakX:Int = 0;
		var tweakY:Int = 0;

		for (i in 0...3) {
			switch (i) {
				case 1:
					tweakX = x - player.x < 0 ? 1 : -1;
				case 2:
					tweakX = 0;
					tweakY = y - player.y < 0 ? 1 : -1;
			}

			var end = new Coordinate(px + RAD + tweakX, py + RAD + tweakY);

			// trace('goto: end $end');

			paths = path.createPath(start, end, MANHATTAN, true);
			if (paths != null) break;
		}

		if (paths == null) {
			//trace('GOTO false ${player.tx},${player.ty} $px,$py $debugtext');

			// since path was cut it might try again if not added here
			if(ai != null) ai.addNotReachable(px + player.tx, py + player.ty);

			// if (onError != null) onError("can not generate path");
			// trace('AAI: ${player.p_id} CAN NOT GENERATE PATH');
			return false;
		}

		/*for(path in paths)
			{
				trace(path);
		}*/

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

		//trace('GOTO done $debugtext $px $py');

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

				vector[int] = vector[int] || playerInterface.isBlocked(x,y);

				if(considerAnimal == false) continue;

				var obj = world.getObjectHelper(x,y, true);
				if(obj != null && obj.isAnimal() && obj.objectData.damage > 0)
				{
					//trace('Animal: ${obj.name}');	
					var moves = obj.objectData.moves;
					var minYY = y - moves;
					var minXX = x - moves;
					var maxYY = y + moves + 1;
					var maxXX = x + moves + 1;
					if(minYY < minY) minYY = minY;
					if(minXX < minX) minXX = minX;
					if(maxYY > maxY) maxYY = maxY;
					if(maxXX > maxX) maxXX = maxX;
					
					for(yy in minYY...maxYY)
					{
						for(xx in minXX...maxXX)
						{
							var index = (xx - minX) + (yy - minY) * RAD * 2;
							if(index < 0) trace('Animal: $index < ${vector.length} $x,$y => $xx,$yy');
							if(index > vector.length -1) trace('Animal: $index < ${vector.length} $x,$y => $xx,$yy');
							if(index < 0) index = 0;
							if(index > vector.length -1) index = vector.length - 1;
							vector[index] = true;
						}
					}
				}

				//var obj = world.getObjectData(x, y);			
				//vector[int] = obj.blocksWalking || world.isBiomeBlocking(x, y);
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
					trace('TEST1 IGNORE AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));
					continue;
				}

				if (ShouldDebug(trans)) trace('TEST1 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));

				if (trans.actorID == wantedObjId || trans.actorID == objectIdToSearch) continue;

				if (ShouldDebug(trans)) trace('TEST2 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));
				if (trans.targetID == wantedObjId || trans.targetID == objectIdToSearch) continue;

				if (ShouldDebug(trans)) trace('TEST3 AI craft steps: $stepsCount WANTED: <${wantedObjId}> T: ' + trans.getDesciption(true));

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

	public static function DisplayCloseDeadlyAnimals(player:PlayerInterface, searchDistance:Int = 6){
		GetCloseDeadlyAnimal(player, searchDistance, true);
	}

	public static function GetCloseDeadlyAnimal(player:PlayerInterface, searchDistance:Int = 6, display:Bool = false):ObjectHelper {
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

				if(player.isAnimalNotDeadlyForMe(obj)) continue;
				/*if (obj == null) continue;
				if (obj.objectData.deadlyDistance == 0) continue;
				if (obj.objectData.damage == 0) continue;
				if (obj.isAnimal() == false) continue;
				*/
				
				var dist = AiHelper.CalculateQuadDistanceToObject(player, obj);

				if(display) if (dist > 16) cast(player, GlobalPlayerInstance).connection.send(ClientTag.LOCATION_SAYS, ['${obj.tx - player.gx} ${obj.ty - player.gy} !']);

				if (dist > bestDist) continue;
				// var moveQuadDist = Math.pow(obj.objectData.moves + 1, 2);
				// trace('GetCloseDeadlyAnimal: $dist <= $bestDist moveQuadDist: $moveQuadDist ${obj.name}');
				//if (dist > Math.pow(obj.objectData.moves + 1, 2)) continue;			
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

		if (player.angryTime > 4) return null;

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.isHoldingWeapon() == false) continue;
			if (p.isFriendly(player)) continue;
			if (p.angryTime > 4) continue;

			var dist = AiHelper.CalculateDistanceToPlayer(player, p);

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

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.heldByPlayer != null) continue;
			if (isFertile && p.age < ServerSettings.MaxChildAgeForBreastFeeding) continue;

			var isNobleOrMore = cast(p.lineage.prestigeClass, Int) > cast(PrestigeClass.Commoner, Int); 
			var classFood = isNobleOrMore ? p.lineage.prestigeClass * 4 : p.lineage.prestigeClass * 2;
			var considerHungry = Math.min(classFood, p.food_store_max * 0.6);
			var hungry = considerHungry - p.food_store;
			var isAlly = p.isAlly(globalplayer);

			if (isAlly == false && p.angryTime < ServerSettings.CombatAngryTimeBeforeAttack / 2) continue;
			if (p.isCloseRelative(globalplayer) == false
				|| player.getFollowPlayer() == p) hungry = hungry / 2 - 0.25; // prefer close relative
			if (isAlly == false) hungry = hungry / 2 - 0.2; // prefer ally
			if (p.isAi() && isNobleOrMore == false) hungry / 2 - 0.2; // prefer Ai only if noble
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

		// TODO consider ill
		// TODO consider hits

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.age > ServerSettings.MaxChildAgeForBreastFeeding) continue;
			if (p.heldByPlayer != null) continue;

			var hungry = considerHungry - p.food_store;
			if (p.mother != mother) hungry = hungry / 2 - 0.25; // own children count more
			if (p.age > ServerSettings.MinAgeToEat) hungry -= 0.5;
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

	public static function SearchNewHome(player:PlayerInterface) : ObjectHelper {
		var world = WorldMap.world;
		var bestHome = null;
		var bestDistance = Math.pow(80,2);
		var ovens = [for (obj in WorldMap.world.ovens) obj];

		for(possibleHome in ovens){
			if(ObjectData.IsOven(possibleHome.id) == false) continue;
			
			var originalBiomeId = world.getOriginalBiomeId(possibleHome.tx, possibleHome.ty);
			// TODO check loved biome
			// For ginger rock biome should be ok
			if(originalBiomeId == BiomeTag.SWAMP) continue;

			var quadDistance = AiHelper.CalculateQuadDistanceToObject(player, possibleHome);
			if(quadDistance >= bestDistance) continue;

			bestDistance = quadDistance;
			bestHome = possibleHome;
		}

		if(bestHome != null){
			//myPlayer.home = bestHome;
			trace('AAI: ${player.name + player.id} searchNewHome dist: $bestDistance ${bestHome != null}');
		}

		return bestHome;
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

	public var secondObject:ObjectHelper; // in case you need two object like using two milkeed
	public var secondObjectDistance:Float;

	public var craftActor:Null<ObjectHelper> = null;
	public var craftTarget:Null<ObjectHelper> = null;
	public var craftFrom:TransitionForObject;
	public var craftTransFrom:TransitionData;

	public var isDone = false;

	// public var craftSteps:Int;

	public function new(objId:Int, steps:Int, wantedObjId:Int, transition:TransitionData) {
		this.objId = objId;
		this.wantedObjId = wantedObjId;
		this.steps = steps;
		this.bestTransition = transition;
	}

	public function getDesciption():String {
		var description = 'objId: $objId wantedObjId: $wantedObjId steps: $steps trans: ' + bestTransition.getDesciption(true);
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

	public var transActor:ObjectHelper = null;
	public var transTarget:ObjectHelper = null;

	public var transitionsByObjectId:Map<Int, TransitionForObject>;

	public var bestDistance:Float = 99999999999999999999999;

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
			trans.closestObject = null;
			trans.closestObjectDistance = -1;
			trans.closestObjectPlayerIndex = -1;

			trans.secondObject = null;
			trans.closestObjectDistance = -1;

			trans.craftActor = null;
			trans.craftTarget = null;
			trans.isDone = false;
			trans.wantedObjs = new Array<TransitionForObject>();
		}
	}
}

class Village {
	public var fireKeeper:GlobalPlayerInstance = null;
}
