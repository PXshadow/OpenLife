package openlife.auto;

import haxe.ds.Vector;
import openlife.auto.Pathfinder.Coordinate;
import openlife.data.Pos;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import openlife.server.GlobalPlayerInstance;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;

class AiHelper {
	static final RAD:Int = MapData.RAD; // search radius

	public static function CalculateDistanceToPlayer(player:PlayerInterface, playerTo:PlayerInterface):Float {
		return CalculateDistance(player.tx, player.ty, playerTo.tx, playerTo.ty);
	}

	public static function CalculateDistanceToObject(player:PlayerInterface, obj:ObjectHelper):Float {
		return CalculateDistance(player.tx, player.ty, obj.tx, obj.ty);
	}

	// TODO does not consider round map
	public static function CalculateDistance(baseX:Int, baseY:Int, toX:Int, toY:Int):Float {
		return (toX - baseX) * (toX - baseX) + (toY - baseY) * (toY - baseY);
	}

	public static function GetClosestObjectOwnedByPlayer(playerInterface:PlayerInterface, searchDistance:Int = 10):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, false, true));
	}

	public static function GetClosestHeatObject(playerInterface:PlayerInterface, searchDistance:Int = 2):ObjectHelper {
		return (GetClosestObject(playerInterface, null, searchDistance, null, true));
	}

	public static function GetClosestObjectById(playerInterface:PlayerInterface, objId:Int, ignoreObj:ObjectHelper = null):ObjectHelper {
		var objData = ObjectData.getObjectData(objId);
		return GetClosestObject(playerInterface, objData, ignoreObj);
	}

	private static function GetName(objId:Int):String {
		return ObjectData.getObjectData(objId).name;
	}

	public static function GetClosestObject(playerInterface:PlayerInterface, objDataToSearch:ObjectData, searchDistance:Int = 16,
			ignoreObj:ObjectHelper = null, findClosestHeat:Bool = false, ownedByPlayer:Bool = false):ObjectHelper {
		// var RAD = ServerSettings.AiMaxSearchRadius
		var ai = playerInterface.getAi();
		var world = playerInterface.getWorld();
		var player = playerInterface.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var closestObject = null;
		var bestDistance = 0.0;

		for (ty in baseY - searchDistance...baseY + searchDistance) {
			for (tx in baseX - searchDistance...baseX + searchDistance) {
				if (ignoreObj != null && ignoreObj.tx == tx && ignoreObj.ty == ty) continue;

				// findClosestHeat == false / since its jused for player temerpature where it does not matter if blocked. Also otherwiese a mutex would be missing since isObjectNotReachable can be used from the AI at same time
				if (ai != null && findClosestHeat == false && ai.isObjectNotReachable(tx, ty)) continue;

				var objData = world.getObjectDataAtPosition(tx, ty);

				if (ownedByPlayer && objData.isOwned == false) continue; // cannot have a owner

				if (findClosestHeat && objData.heatValue == 0) continue;

				if (ownedByPlayer
					|| findClosestHeat
					|| objData.parentId == objDataToSearch.parentId) // compare parent, because of dummy objects for obj with numberOfuses > 1 may have different IDs
				{
					var obj = world.getObjectHelper(tx, ty);

					if (ownedByPlayer && obj.isOwnedByPlayer(playerInterface) == false) continue;

					var distance = AiHelper.CalculateDistance(baseX, baseY, obj.tx, obj.ty);

					if (closestObject == null || distance < bestDistance) {
						closestObject = obj;
						bestDistance = distance;
					}
				}
			}
		}

		// if(closestObject !=null) trace('AI: bestdistance: $bestDistance ${closestObject.description}');

		return closestObject;
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

		if (ServerSettings.DebugAi) trace('AI: ${obj.description} ${obj.id} numberOfUses: ${obj.numberOfUses} originalFoodValue: $originalFoodValue');

		return originalFoodValue > 0;
		// if(originalFoodValue < 0) return false;
		// if(player.food_store_max - player.food_store < Math.ceil(originalFoodValue / 4)) return false;
		// return true;
	}

	public static function SearchBestFood(player:PlayerInterface, feedOther:Bool = false, radius:Int = 40):ObjectHelper {
		var startTime = Sys.time();
		var ai = player.getAi();
		var baseX = player.tx;
		var baseY = player.ty;
		var world = player.getWorld();
		var bestFood = null;
		var bestDistance = 999999.0;
		var bestFoodValue = 0.1;
		var bestFoods = new Array<ObjectHelper>();
		var isStarving = player.food_store < 2;
		var starvingFactor:Float = isStarving ? 4 : 25;

		if (player.food_store < 0.5) starvingFactor = 2;
		if (player.food_store < -1) starvingFactor = 1.5;
		if (player.food_store < -1.5) starvingFactor = 1.2;

		for (ty in baseY - radius...baseY + radius) {
			for (tx in baseX - radius...baseX + radius) {
				var objData = world.getObjectDataAtPosition(tx, ty);

				if (objData.id == 0) continue;
				if (objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata
				if (feedOther && objData.id == 837) continue; // dont feed Psilocybe Mushroom to others

				// var distance = calculateDistance(baseX, baseY, obj.tx, obj.ty);
				// trace('search food $tx, $ty: foodvalue: ${objData.foodValue} bestdistance: $bestDistance distance: $distance ${obj.description}');

				// var tmp = ObjectData.getObjectData(31);
				// trace('berry food: ${tmp.foodValue}');

				var originalFoodValue = objData.foodFromTarget == null ? objData.foodValue : objData.foodFromTarget.foodValue;
				var foodId = objData.getFoodId();
				var foodValue:Float = originalFoodValue;

				if (foodValue <= 0) continue;
				if (player.food_store_max - player.food_store < Math.ceil(foodValue / 4)) continue;

				var obj = world.getObjectHelper(tx, ty);
				var quadDistance = AiHelper.CalculateDistance(baseX, baseY, obj.tx, obj.ty);

				var countEaten = player.getCountEaten(foodId);
				foodValue -= countEaten;
				var isYum = countEaten < ServerSettings.YumBonus;
				var isSuperMeh = foodValue < originalFoodValue / 2; // can eat if food_store < 0
				// trace('search food: best $bestDistance dist $distance ${obj.description}');

				if (isYum) foodValue *= starvingFactor;
				if (isSuperMeh) foodValue = originalFoodValue / starvingFactor;
				if (isSuperMeh && player.food_store > 1) foodValue = 0;
				if (foodId == player.getCraving()) foodValue *= Math.pow(starvingFactor, 2);

				if (quadDistance < 0.5) quadDistance = 0.5;
				// distance = Math.sqrt(distance);

				if (bestFood == null || foodValue / quadDistance > bestFoodValue / bestDistance) {
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

		if (ai != null) {
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
		}

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

	public static function gotoObj(player:PlayerInterface, obj:ObjectHelper, move:Bool = true):Bool {
		return gotoAdv(player, obj.tx, obj.ty, move);
	}

	public static function gotoAdv(player:PlayerInterface, tx:Int, ty:Int, move:Bool = true):Bool {
		var startTime = Sys.time();
		var ai = player.getAi();
		var rand = 0;
		var x = tx - player.gx;
		var y = ty - player.gy;

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

			var done = Goto(player, x + xo, y + yo, move);

			if (done) return true;

			var passedTime = (Sys.time() - startTime) * 1000;

			if (passedTime > 500) {
				trace('AI: ${player.id}  ${player.name} GOTO failed after $i because of timeout $passedTime! Ignore ${tx} ${ty}');
				break;
			}
		}

		if (ServerSettings.DebugAiGoto) trace('AI: ${player.id} ${player.name} GOTO failed! Ignore ${tx} ${ty}');
		ai.addNotReachable(tx, ty);

		ai.resetTargets();

		return false;
	}

	public static function TryGoto(playerInterface:PlayerInterface, x:Int, y:Int):Bool {
		return Goto(playerInterface, x, y, false);
	}

	// TODO goto uses global coordinates
	public static function Goto(playerInterface:PlayerInterface, x:Int, y:Int, move:Bool = true):Bool {
		var player = playerInterface.getPlayerInstance();

		// var goal:Pos;
		// var dest:Pos;
		// var init:Pos;

		// if (player.x == x && player.y == y || moving) return false;
		// set pos
		var px = x - player.x;
		var py = y - player.y;

		// trace('AAI: GOTO: From: ${player.x},${player.y} To: $x $y / FROM ${player.tx()},${player.ty()} To: ${x + player.gx},${y + player.gy}');

		if (px == 0 && py == 0) return false; // no need to move

		if (px > RAD - 1) px = RAD - 1;
		if (py > RAD - 1) py = RAD - 1;
		if (px < -RAD) px = -RAD;
		if (py < -RAD) py = -RAD;
		// cords
		var start = new Coordinate(RAD, RAD);

		// trace('Goto: $px $py');

		var map = new MapCollision(AiHelper.CreateCollisionChunk(playerInterface));
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

		return true;
	}

	private static function CreateCollisionChunk(playerInterface:PlayerInterface):Vector<Bool> {
		var player:PlayerInstance = playerInterface.getPlayerInstance();
		var world = playerInterface.getWorld();
		var RAD = MapData.RAD;
		var vector = new Vector<Bool>((RAD * 2) * (RAD * 2));
		var int:Int = -1;

		for (y in player.ty - RAD...player.ty + RAD) {
			for (x in player.tx - RAD...player.tx + RAD) {
				int++;

				var obj = world.getObjectHelper(x, y);
				vector[int] = obj.blocksWalking() || world.isBiomeBlocking(x, y);

				// if(obj.blocksWalking()) trace('${player.tx()} ${player.ty()} $x $y ${obj.description}');
			}
		}

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

				// Allow transition if new actor or target is closer to wanted object
				var tmpActor = transitionsForObject[trans.actorID];
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

	public static function GetCloseDeadlyAnimal(player:PlayerInterface, searchDistance:Int = 5):ObjectHelper {
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

				if (obj == null) continue;
				if (obj.objectData.deadlyDistance == 0) continue;
				if (obj.objectData.damage == 0) continue;
				if (obj.isAnimal() == false) continue;

				var dist = AiHelper.CalculateDistanceToObject(player, obj);

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

	public static function GetCloseStarvingPlayer(player:PlayerInterface, searchDistance:Int = 30) {
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

			var considerHungry = Math.min(p.lineage.prestigeClass * 2, 1 + p.food_store_max * 0.8);
			var hungry = considerHungry - p.food_store;
			var isAlly = p.isAlly(globalplayer);

			if (isAlly == false && p.angryTime < ServerSettings.CombatAngryTimeBeforeAttack / 2) continue;
			if (p.isCloseRelative(globalplayer) == false
				|| player.getFollowPlayer() == p) hungry = hungry / 2 - 0.25; // prefer close relative
			if (isAlly == false) hungry = hungry / 2 - 0.2; // prefer ally
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

	public static function GetCloseHungryChild(mother:PlayerInterface, searchDistance:Int = 40) {
		var bestPlayer:GlobalPlayerInstance = null;
		var maxDist = searchDistance * searchDistance;
		var bestQuadHungry:Float = 0;
		var considerHungry = 2.5;
		var minQuadHungry = 0.01;

		for (p in GlobalPlayerInstance.AllPlayers) {
			if (p.deleted) continue;
			if (p.age > ServerSettings.MaxChildAgeForBreastFeeding) continue;
			if (p.heldByPlayer != null) continue;

			var hungry = considerHungry - p.food_store;
			if (p.mother != mother) hungry = hungry / 2 - 0.5; // own children count more
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

	public static function GetMostDistamtOwnChild(mother:PlayerInterface, minDist:Int = 10, searchDistance:Int = 50) {
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
