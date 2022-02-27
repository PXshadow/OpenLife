package openlife.auto;

import haxe.Exception;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.server.Connection;
import openlife.server.GlobalPlayerInstance;
import openlife.server.NamingHelper;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;
import sys.thread.Thread;

using StringTools;
using openlife.auto.AiHelper;
/*
abstract class AiBase
{
    public var seqNum = 1;
    public var myPlayer(default, default):PlayerInterface;

    public abstract function doTimeStuff(timePassedInSeconds:Float) : Void;

    public abstract function newChild(child:PlayerInterface) : Void;
    public abstract function say(player:PlayerInterface, curse:Bool, text:String) : Void;
    public abstract function finishedMovement() : Void;
    public abstract function newBorn() : Void;
    public abstract function emote(player:PlayerInstance, index:Int) : Void;
	public abstract function playerUpdate(player:PlayerInstance) : Void;
	public abstract function mapUpdate(targetX:Int, targetY:Int, isAnimal:Bool = false) : Void;
	public abstract function playerMove(player:PlayerInstance, targetX:Int, targetY:Int) : Void;

    public abstract function isObjectNotReachable(tx:Int, ty:Int):Bool;
    public abstract function addNotReachableObject(obj:ObjectHelper, time:Float = 90) : Void;
    public abstract function addNotReachable(tx:Int, ty:Int, time:Float = 90) : Void;
    public abstract function isObjectWithHostilePath(tx:Int, ty:Int):Bool; 

    public abstract function resetTargets() : Void;
}*/


abstract class AiBase
{
	public static var lastTick:Float = 0;
	public static var tick:Float = 0;

	final RAD:Int = MapData.RAD; // search radius

    public var myPlayer(default, default):PlayerInterface;
    public var seqNum = 1;
	public var time:Float = 1;

	//public var seqNum = 1;

	var feedingPlayerTarget:PlayerInterface = null;

	var animalTarget:ObjectHelper = null;
	var escapeTarget:ObjectHelper = null;
	var foodTarget:ObjectHelper = null;
	var dropTarget:ObjectHelper = null;
	var useTarget:ObjectHelper = null;
	var useActor:ObjectHelper = null; // to check if the right actor is in the hand

	var itemToCraftId = -1;
	var itemToCraft:IntemToCraft = new IntemToCraft();

	var isHungry = false;

	var playerToFollow:PlayerInterface;

	var children = new Array<PlayerInterface>();

	var notReachableObjects = new Map<Int, Float>();
	var objectsWithHostilePath = new Map<Int, Float>();

	var craftingTasks = new Array<Int>();

	// counts how often one could not reach food because of dedly animals
	var didNotReachFood:Float = 0;
	var didNotReachAnimalTarget:Float = 0;

	public static function StartAiThread() {
		Thread.create(RunAi);
	}

	private static function RunAi() {
		var skipedTicks = 0;
		var averageSleepTime:Float = 0;

		while (true) {
			AiBase.tick = Std.int(AiBase.tick + 1);

			var timeSinceStart:Float = Sys.time() - TimeHelper.serverStartingTime;
			var timeSinceStartCountedFromTicks = AiBase.tick * TimeHelper.tickTime;

			// TODO what to do if server is too slow?
			if (AiBase.tick % 10 != 0 && timeSinceStartCountedFromTicks < timeSinceStart) {
				AiBase.tick = Std.int(AiBase.tick + 1);
				skipedTicks++;
			}
			if (AiBase.tick % 200 == 0) {
				averageSleepTime = Math.ceil(averageSleepTime / 200 * 1000) / 1000;
				// trace('AIs: ${Connection.getAis().length} Tick: ${Ai.tick} Time From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime');
				trace('\nAIs: ${Connection.getAis().length} Time From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime ');
				averageSleepTime = 0;
				skipedTicks = 0;
			}

			var timePassedInSeconds = CalculateTimeSinceTicksInSec(lastTick);

			lastTick = tick;

			for (ai in Connection.getAis()) {
				if (ai.player.deleted) Macro.exception(ai.doRebirth(timePassedInSeconds));
				Macro.exception(ai.doTimeStuff(timePassedInSeconds));
			}

			if (timeSinceStartCountedFromTicks > timeSinceStart) {
				var sleepTime = timeSinceStartCountedFromTicks - timeSinceStart;
				averageSleepTime += sleepTime;

				// if(ServerSettings.DebugAi) trace('sleep: ${sleepTime}');
				Sys.sleep(sleepTime);
			}
		}
	}

	public static function CalculateTimeSinceTicksInSec(ticks:Float):Float {
		return (AiBase.tick - ticks) * TimeHelper.tickTime;
	}

	public function new(player:PlayerInterface) {
		this.myPlayer = player;
		// this.myPlayer = cast(playerInterface, GlobalPlayerInstance); // TODO support only client AI
	}

	public function resetTargets() {
		escapeTarget = null;
		foodTarget = null;
		useTarget = null;
		itemToCraft.transActor = null;
		itemToCraft.transTarget = null;
	}

	public function newBorn() {
		if (ServerSettings.DebugAi) trace('Ai: newborn!');

		foodTarget = null;
		dropTarget = null;
		useTarget = null;

		itemToCraftId = -1;
		itemToCraft = new IntemToCraft();

		isHungry = false;

		playerToFollow = null;
		children = new Array<PlayerInterface>();
		//addTask(837); //Psilocybe Mushroom
        //addTask(134); //Flint Arrowhead
        addTask(82); // Fire
        addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow

        //addTask(140); // Tied Skewer

        //addTask(148); // Arrow
        //addTask(149); // Headless Arrow
        //addTask(146); // Fletching
        //addTask(151); // Jew Bow 
        //addTask(151); // Jew Bow 
        //addTask(59); // Rope 
		addTask(82); // Fire
		// addTask(80); // Burning Tinder
		// addTask(78); // Smoldering Tinder
		// addTask(72); // Kindling
		// addTask(71); // Stone Hatchet

		// craftItem(71); // Stone Hatchet
		// craftItem(72); // Kindling
		// craftItem(82); // Fire
		// craftItem(58); // Thread
		// craftItem(74, 1, true); //Fire Bow Drill
		// craftItem(78, 1, true); // Smoldering Tinder
		// craftItem(808); // wild onion
		// craftItem(292, 1, true); // 292 basket
		// craftItem(224); // Harvested Wheat
		// craftItem(124); // Reed Bundle
		// craftItem(225); //Wheat Sheaf

		// craftItem(34,1); // 34 sharpstone
		// craftItem(224); // Harvested Wheat
		// craftItem(58); // Thread
	}

	// do time stuff here is called from TimeHelper
	public function doTimeStuff(timePassedInSeconds:Float) {
		time -= timePassedInSeconds;

		// if(didNotReachFood > 0) didNotReachFood -= timePassedInSeconds * 0.02;
        if(time > 5) time = 5; // wait max 5 sec
		if (time > 0) return;
		time += ServerSettings.AiReactionTime; // 0.5; // minimum AI reacting time

		cleanupBlockedObjects();

		if (ServerSettings.AutoFollowAi && myPlayer.isHuman()) {
			// if(ServerSettings.DebugAi) trace('HUMAN');
			time = 0.2;
			isMovingToPlayer(5, false);
			return;
		}

		if(myPlayer.isHuman())
		{
			trace('AAI: ${myPlayer.name + myPlayer.id} WARNING is human!');
			return;
		}

		if (myPlayer.getHeldByPlayer() != null) {
			// time += WorldMap.calculateRandomInt(); // TODO still jump and do stuff once in a while?
			return;
		}

		var animal = AiHelper.GetCloseDeadlyAnimal(myPlayer);
		var deadlyPlayer = AiHelper.GetCloseDeadlyPlayer(myPlayer);

		Macro.exception(if (didNotReachFood < 5) if (escape(animal, deadlyPlayer)) return);
		//Macro.exception(if (didNotReachFood < 5 || myPlayer.food_store < 1) checkIsHungryAndEat());
		Macro.exception(checkIsHungryAndEat());
		Macro.exception(if (isChildAndHasMother()) {
			if (isMovingToPlayer(10)) return;
		});
		Macro.exception(if (myPlayer.isWounded()) {
			isMovingToPlayer(5);
			return;
		}); // do nothing then looking for player

		Macro.exception(if (isDropingItem()) return);
		Macro.exception(if (myPlayer.age < ServerSettings.MinAgeToEat && isHungry) {
			isMovingToPlayer(5);
			return;
		}); // go close to mother and wait for mother to feed
		Macro.exception(if (isEating()) return);
		Macro.exception(if (isFeedingChild()) return);
		Macro.exception(if (isPickingupFood()) return);
		Macro.exception(if (isFeedingPlayerInNeed()) return);
		Macro.exception(if (isStayingCloseToChild()) return);
		Macro.exception(if (isUsingItem()) return);
		Macro.exception(if (killAnimal(animal)) return);
		Macro.exception(if (ServerSettings.AutoFollowPlayer && isMovingToPlayer()) return);

		if (myPlayer.isMoving()) return;

		// if(playerToFollow == null) return; // Do stuff only if close to player TODO remove if testing AI without player

		if (ServerSettings.DebugAi) trace('AI: craft ${GetName(itemToCraftId)} tasks: ${craftingTasks.length}!');

		if (itemToCraftId > 0 && itemToCraft.countDone < itemToCraft.count) {
			Macro.exception(if (craftItem(itemToCraftId)) return);
		}

		if (craftingTasks.length > 0) {
			for (i in 0...craftingTasks.length) {
				itemToCraftId = craftingTasks.shift();
				Macro.exception(if (craftItem(itemToCraftId)) return);
				craftingTasks.push(itemToCraftId);
			}
		}

		var cravingId = myPlayer.getCraving();
		itemToCraftId = cravingId;
		Macro.exception(if (cravingId > 0) if (craftItem(itemToCraftId)) return);

		myPlayer.say('nothing to do...');
	}

	public function say(player:PlayerInterface, curse:Bool, text:String) {
		if (myPlayer.id == player.id) return;

		// if(ServerSettings.DebugAi) trace('AI ${text}');

		if (text.contains("TRANS")) {
			if (ServerSettings.DebugAi) trace('AI look for transitions: ${text}');

			var objectIdToSearch = 273; // 273 = Cooked Carrot Pie // 250 = Hot Adobe Oven

			AiHelper.SearchTransitions(myPlayer, objectIdToSearch);
		}

		if (text.contains("HELLO") || text == "HI") {
			// HELLO WORLD

			// if(ServerSettings.DebugAi) trace('im a nice bot!');

			myPlayer.say('HOLA ${player.name}');
		}
        if (text.contains("ARE YOU AI") || text.contains("ARE YOU AN AI") || text == "AI?" || text == "AI") {
			// HELLO WORLD

			// if(ServerSettings.DebugAi) trace('im a nice bot!');

			myPlayer.say('Im not a stupid AI!');
		}
		if (text.contains("JUMP")) {
			myPlayer.say("JUMP");
			myPlayer.jump();
		}
		if (text.contains("MOVE")) {
			myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.say("YES CAPTAIN");
		}
		if (text.contains("FOLLOW ME")) {
			playerToFollow = player;
			myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.say("SURE CAPTAIN");
		}
		if (text.contains("STOP")) {
			playerToFollow = null;
			myPlayer.say("YES CAPTAIN");
		}
		/*if (text.contains("EAT!"))
			{
				AiHelper.SearchBestFood();
				searchFoodAndEat();
				myPlayer.say("YES CAPTAIN");
		}*/
		if (text.startsWith("MAKE") || text.startsWith("CRAFT")) {
			var id = GlobalPlayerInstance.findObjectByCommand(text);

			if (id > 0) {
				craftItem(id); // TODO use mutex if Ai does not use Globalplayermutex
				var obj = ObjectData.getObjectData(id);
				myPlayer.say("Making " + obj.name);
			}
		}
	}

	public function searchFoodAndEat() {
		foodTarget = AiHelper.SearchBestFood(myPlayer);
		if (ServerSettings.DebugAi && foodTarget != null) trace('AAI: ${myPlayer.name + myPlayer.id} new Foodtarget! ${foodTarget.name}');
		if (ServerSettings.DebugAi && foodTarget == null) trace('AAI: ${myPlayer.name + myPlayer.id} no new Foodtarget!!!');
	}

	public function dropHeldObject(dropOnStart:Bool = false) {
		// var myPlayer = myPlayer.getPlayerInstance();

		if (myPlayer.heldObject.id == 0) return;

		if (dropOnStart && itemToCraft.startLocation != null) {
			var distance = myPlayer.CalculateDistanceToObject(itemToCraft.startLocation);

			if (distance > 25) {
				var done = myPlayer.gotoObj(itemToCraft.startLocation);
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done drop goto start $distance');
				myPlayer.say('Goto start!');
				return;
			}
		}

		var emptyTileObj = myPlayer.GetClosestObjectById(0); // empty
		dropTarget = emptyTileObj;

		// if(itemToCraft.transTarget.parentId == myPlayer.heldObject.parentId)

		if (ServerSettings.DebugAi) trace('Drop ${emptyTileObj.tx} ${emptyTileObj.ty} $emptyTileObj');
		// x,y is relativ to birth position, since this is the center of the universe for a player
		// if(emptyTileObj != null) playerInterface.drop(emptyTileObj.tx - myPlayer.gx, emptyTileObj.ty - myPlayer.gy);
	}

	public function isChildAndHasMother() // must not be his original mother
	{
		var mother = myPlayer.getFollowPlayer();
		return (myPlayer.age < ServerSettings.MinAgeToEat && mother != null && mother.isDeleted() == false);
	}

	public function addTask(taskId:Int, atEnd:Bool = true) {
		if (taskId < 1) return;
        var index = this.craftingTasks.indexOf(taskId);
		if (index >= 0) return;
            
		if (atEnd) this.craftingTasks.push(taskId); else
			this.craftingTasks.unshift(taskId);
	}

	private function killAnimal(animal:ObjectHelper) {
		if (animal == null && animalTarget == null) return false;
		if (foodTarget != null) return false;

		var objData = ObjectData.getObjectData(152); // Bow and Arrow
		if (myPlayer.age < objData.minPickupAge) return false;

		if (animalTarget != null && animalTarget.isKillableByBow() == false) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: Old target not killable with bow anymore: ${animalTarget.description}');
			animalTarget = null;
		}

		if (animalTarget == null && animal != null) {
			if (animal.isKillableByBow()) this.animalTarget = animal; else if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: Not killable with bow: ${animal.description}');
		}

		if (animalTarget == null) return false;

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: ${animalTarget.description}');

		if (myPlayer.heldObject.id != objData.id) {
			return GetOrCraftItem(objData.id);
		}

		var distance = myPlayer.CalculateDistanceToObject(animalTarget);
		var range = objData.useDistance;

		if (distance > range * range || (range > 1.9 && distance < 1.5)) // check if too far or too close
		{
			var targetXY = new ObjectHelper(null, 0);

			targetXY.tx = animalTarget.tx > myPlayer.tx ? animalTarget.tx - range + 1 : animalTarget.tx + range - 1;
			targetXY.ty = animalTarget.ty > myPlayer.ty ? animalTarget.ty - range + 1 : animalTarget.ty + range - 1;

			var done = myPlayer.gotoObj(targetXY);

			if (done) didNotReachAnimalTarget = 0; else {
				didNotReachAnimalTarget++;
				if (didNotReachAnimalTarget >= 5) animalTarget = null;
			}

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal $distance goto animaltarget ${done}');

			return true;
		}

		var done = myPlayer.use(animalTarget.tx - myPlayer.gx, animalTarget.ty - myPlayer.gy);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: done: $done kill ${animalTarget.description}');

		didNotReachFood = 0;

		return true;
	}

	private function GetOrCraftItem(objId:Int, count:Int = 1):Bool {
		if (myPlayer.isMoving()) return false;

		var obj = AiHelper.GetClosestObjectById(myPlayer, objId);

		if (obj == null) return craftItem(objId, count);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: GetOrCraftItem found ${obj.name}');

		var distance = myPlayer.CalculateDistanceToObject(obj);

		if (distance > 1) {
			var done = myPlayer.gotoObj(obj);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal done: $done goto weapon');
			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);

			if (ServerSettings.DebugAi || done == false)
				trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal child drop for get item ${heldPlayer.name} $done');

			return true;
		}

		// if(ServerSettings.DebugAi) trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = myPlayer.drop(obj.tx - myPlayer.gx, obj.ty - myPlayer.gy);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal done: $done pickup obj from floor');

		return done;
	}

	private function cleanupBlockedObjects() {
		for (key in notReachableObjects.keys()) {
			var time = notReachableObjects[key];
			time -= ServerSettings.AiReactionTime;

			if (time <= 0) {
				notReachableObjects.remove(key);
				// if(ServerSettings.DebugAi) trace('Unblock: remove $key t: $time');
				continue;
			}

			// trace('Unblock: $key t: $time');

			notReachableObjects[key] = time;
		}

		for (key in objectsWithHostilePath.keys()) {
			var time = objectsWithHostilePath[key];
			time -= ServerSettings.AiReactionTime;

			if (time <= 0) {
				objectsWithHostilePath.remove(key);
				// if(ServerSettings.DebugAi) trace('Unblock: remove $key t: $time');
				continue;
			}

			// if(ServerSettings.DebugAi) trace('Unblock: $key t: $time');

			objectsWithHostilePath[key] = time;
		}
	}

	private function isFeedingPlayerInNeed() {
		if (myPlayer.age < ServerSettings.MinAgeToEat) return false;
		if (myPlayer.food_store < 2) return false;

		if (this.feedingPlayerTarget == null) this.feedingPlayerTarget = AiHelper.GetCloseStarvingPlayer(myPlayer);
		if (this.feedingPlayerTarget == null) return false;

		var targetPlayer = this.feedingPlayerTarget;

		if (targetPlayer.food_store > targetPlayer.food_store_max * 0.85) {
			this.feedingPlayerTarget = null;
			return false;
		}

		if (myPlayer.heldObject.objectData.foodValue < 1
			|| myPlayer.heldObject.id == 837) // dont feed 837 ==> Psilocybe Mushroom to others
		{
			foodTarget = AiHelper.SearchBestFood(targetPlayer, true);
			return true;
		}

		if (targetPlayer.isDeleted()) {
			this.feedingPlayerTarget = null;
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name} cannot feed ${targetPlayer.name} since is dead!');
			return false;
		}

		if (targetPlayer.getHeldByPlayer() != null) {
			this.feedingPlayerTarget = null;
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name} cannot feed ${targetPlayer.name} since held by other player!');
			return false;
		}

		if (targetPlayer.canFeedToMe(myPlayer.heldObject) == false) {
			this.feedingPlayerTarget = null;
			// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name} cannot feed ${targetPlayer.name} ${myPlayer.heldObject.name}');
			trace('AAI: ${myPlayer.name} cannot feed ${targetPlayer.name} ${myPlayer.heldObject.name} fs: ${targetPlayer.food_store}');
			return false;
		}

		var distance = myPlayer.CalculateDistanceToPlayer(targetPlayer);

		if (distance > 10 && myPlayer.isMoving()) return true;

		if (distance > 1) {
			var done = myPlayer.gotoAdv(targetPlayer.tx, targetPlayer.ty);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done goto feed starving ${targetPlayer.name} dist: $distance');
			return true;
		}

		var done = myPlayer.doOnOther(targetPlayer.tx - myPlayer.gx, targetPlayer.ty - myPlayer.gx, -1, targetPlayer.id);
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done feed starving ${targetPlayer.name}');

		return true;
	}

	private function isStayingCloseToChild() {
		if (myPlayer.isFertile() == false) return false;

		var child = AiHelper.GetMostDistantOwnChild(myPlayer);

		if (child == null) return false;

		var done = myPlayer.gotoAdv(child.tx, child.ty);
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done goto far away child ${child.name}');

		return true;
	}

	private function isFeedingChild() {
		if(myPlayer.food_store < 2) return false;
		
		var heldPlayer = myPlayer.getHeldPlayer();

		if (myPlayer.isFertile() == false || myPlayer.food_store < 1) {
			if (myPlayer.getHeldPlayer() != null) {
				var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);
				this.feedingPlayerTarget = null;
				myPlayer.say('I cannot feed you!');
				if (ServerSettings.DebugAi || done == false)
					trace('AAI: ${myPlayer.name} cannot feed ==> child drop ${heldPlayer.name} food: ${heldPlayer.food_store} $done');
			}
			return false;
		}
		//if (foodTarget != null) return false;
		if (heldPlayer != null) {
			if (heldPlayer.name == ServerSettings.StartingName && (heldPlayer.mother == myPlayer || heldPlayer.age > 1.5)) {
				var newName = myPlayer.isEveOrAdam() ? NamingHelper.GetRandomName(myPlayer.isFemale()) : myPlayer.name;
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} child newName: $newName');
				myPlayer.say('You are $newName');
			}

			if (heldPlayer.food_store > heldPlayer.getMaxChildFeeding() - 0.2) {
				var hungryChild = AiHelper.GetCloseHungryChild(myPlayer);

				// only drop child if there is another hungry child, or if the held child can walk, has near full hits and is not ill
				if (hungryChild != null
					|| (heldPlayer.age * 60 > ServerSettings.MinMovementAgeInSec && heldPlayer.hits < 1 && heldPlayer.isIll() == false)) {
					var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);
					this.feedingPlayerTarget = null;
					if (ServerSettings.DebugAi || done == false)
						trace('AAI: ${myPlayer.name + myPlayer.id} child drop ${heldPlayer.name} food: ${heldPlayer.food_store} max: ${heldPlayer.getMaxChildFeeding() - 0.2} $done');
					return true;
				}
			}
		}

		if (heldPlayer != null) return true;

		var child = AiHelper.GetCloseHungryChild(myPlayer);
		if (child == null) return false;

		this.feedingPlayerTarget = child;

		var childFollowPlayer = child.getFollowPlayer();
		if (childFollowPlayer == null || childFollowPlayer.isFertile() == false) {
			playerToFollow = myPlayer;
		}

		var distance = myPlayer.CalculateDistanceToPlayer(child);
		if (distance > 1) {
			var done = myPlayer.gotoAdv(child.tx, child.ty);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done goto child to feed ${child.name}');

			return true;
		}

		if (myPlayer.heldObject.id != 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop obj for feeding child');
			dropHeldObject();
			return true;
		}

		var childX = child.tx - myPlayer.gx;
		var childY = child.ty - myPlayer.gy;

		myPlayer.say('Pickup ${child.name}');
		var done = myPlayer.doBaby(childX, childY, child.id);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} child ${child.name} pickup $done');

		return true;
	}

	private function escape(animal:ObjectHelper, deadlyPlayer:GlobalPlayerInstance) {
		var startTime = Sys.time();

		if (animal == null && deadlyPlayer == null) return false;
		// if(myPlayer == null) throw new Exception('WARNING! PLAYER IS NULL!!!');
		if (ServerSettings.DebugAi) trace('escape: animal: ${animal != null} deadlyPlayer: ${deadlyPlayer != null}');
		// hunt this animal
		if (animal != null && animal.isKillableByBow()) animalTarget = animal;
		// go for hunting
		if (myPlayer.isHoldingWeapon() && myPlayer.isWounded() == false) return false;

		var player = myPlayer.getPlayerInstance();
		var escapeDist = 3;
		var distAnimal = animal == null ? 99999999 : AiHelper.CalculateDistanceToObject(myPlayer, animal);
		var distPlayer = deadlyPlayer == null ? 99999999 : AiHelper.CalculateDistanceToPlayer(myPlayer, deadlyPlayer);
		var escapePlayer = deadlyPlayer != null && distAnimal > distPlayer;
		if (ServerSettings.DebugAi) trace('escape: distAnimal: ${distAnimal} distPlayer: ${distPlayer}');
		var description = escapePlayer ? deadlyPlayer.name : animal.description;
		var escapeTx = escapePlayer ? deadlyPlayer.tx : animal.tx;
		var escapeTy = escapePlayer ? deadlyPlayer.ty : animal.ty;
		var newEscapetarget = new ObjectHelper(null, 0);

		myPlayer.say('Escape ${description} ${Math.ceil(didNotReachFood)}!');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} escape!');

		var done = false;
		var alwaysX = false;
		var alwaysY = false;
		var checkIfDangerous = true;

		for (ii in 0...5) {
			for (i in 0...5) {
				var escapeInLowerX = alwaysX || escapeTx > player.tx;
				var escapeInLowerY = alwaysY || escapeTy > player.ty;

                if (ii > 0)
                {
                    var rand = WorldMap.calculateRandomFloat();
                    if(rand < 0.2) escapeInLowerX = true;
                    else if(rand < 0.4) escapeInLowerX = false;

                    var rand = WorldMap.calculateRandomFloat();
                    if(rand < 0.2) escapeInLowerY = true;
                    else if(rand < 0.4) escapeInLowerY = false;
                }

				newEscapetarget.tx = escapeInLowerX ? player.tx - escapeDist : player.tx + escapeDist;
				newEscapetarget.ty = escapeInLowerY ? player.ty - escapeDist : player.ty + escapeDist;

				var randX = WorldMap.calculateRandomInt(1 + ii);
				var randY = WorldMap.calculateRandomInt(1 + ii);
				randX = escapeInLowerX ? -randX : randX;
				randY = escapeInLowerY ? -randY : randY;

				newEscapetarget.tx += randX;
				newEscapetarget.ty += randY;

				if (myPlayer.isBlocked(newEscapetarget.tx, newEscapetarget.ty)) continue;

				if (checkIfDangerous && AiHelper.IsDangerous(myPlayer, newEscapetarget)) continue;

				done = myPlayer.gotoObj(newEscapetarget);

				// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Escape $done $ii $i alwaysX: $alwaysX alwaysY $alwaysY es: ${newEscapetarget.tx},${newEscapetarget.ty}');

				if (done) break;
				if ((Sys.time() - startTime) * 1000 > 100) break;
			}

			if (done) break;
			if ((Sys.time() - startTime) * 1000 > 100) break;

			//alwaysX = WorldMap.calculateRandomFloat() < 0.5;
			//alwaysY = WorldMap.calculateRandomFloat() < 0.5;

			if (ii > 0) checkIfDangerous = false;

			// if(ServerSettings.DebugAi) trace('Escape $ii alwaysX: $alwaysX alwaysY $alwaysY');
		}

		if (useTarget != null || foodTarget != null || escapeTarget != null) {
			if (foodTarget != null) didNotReachFood++;

			addObjectWithHostilePath(useTarget);
			addObjectWithHostilePath(foodTarget);
			addObjectWithHostilePath(escapeTarget);
			useTarget = null;
			foodTarget = null;
			itemToCraft.transActor = null;
			itemToCraft.transTarget = null;
		}

		escapeTarget = newEscapetarget;

		return true;
	}

	// TODO consider held object / backpack / contained objects
	// TODO consider if object is reachable
	// TODO store transitions for crafting to have faster lookup
	// TODO consider too look for a natural spawned object with the fewest steps on the list
	private function craftItem(objId:Int, count:Int = 1, ignoreHighTech:Bool = false):Bool {
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft item ${GetName(objId)}!');

		var player = myPlayer.getPlayerInstance();

		if (itemToCraft.transActor != null && player.heldObject.parentId == itemToCraft.transActor.parentId) {
			useActor = itemToCraft.transActor;
			itemToCraft.transActor = null; // actor is allready in the hand
			var target = AiHelper.GetClosestObject(myPlayer, itemToCraft.transTarget.objectData);
			useTarget = target != null ? target : itemToCraft.transTarget; // since other search radius might be bigger

			// check if some one meanwhile changed use target
			if (myPlayer.isStillExpectedItem(useTarget)) return true;
		}

		if (itemToCraft.itemToCraft.parentId != objId) {
			if (itemToCraft.countDone < itemToCraft.count) // if taks was disturbed add it to que
				addTask(itemToCraft.itemToCraft.id, true);

			itemToCraft.startLocation = null;
			itemToCraft.itemToCraft = ObjectData.getObjectData(objId);
			itemToCraft.count = count;
			itemToCraft.countDone = 0;
			itemToCraft.countTransitionsDone = 0;
			itemToCraft.transitionsByObjectId = myPlayer.SearchTransitions(objId, ignoreHighTech);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} new item to craft: ${itemToCraft.itemToCraft.description}!');
		}

		searchBestObjectForCrafting(itemToCraft);

		// set position where to craft the object
		if (itemToCraft.startLocation == null && itemToCraft.transTarget != null) {
			itemToCraft.startLocation = new ObjectHelper(null, 0);
			itemToCraft.startLocation.tx = myPlayer.tx; // itemToCraft.transTarget.tx;
			itemToCraft.startLocation.ty = myPlayer.ty; // itemToCraft.transTarget.ty;
		}

		if (itemToCraft.transActor == null) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} craft: FAILED ${itemToCraft.itemToCraft.description} did not find any item in search radius for crafting!');

			// TODO give some help to find the needed Items
			return false;
		}

		// if(player.heldObject.parentId == itemToCraft.transActor.parentId)
		if (player.heldObject.parentId == itemToCraft.transActor.parentId || itemToCraft.transActor.id == 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft Actor is held already ${itemToCraft.transActor.id} or Empty '
				+ itemToCraft.transActor.name);

			if (itemToCraft.transTarget.name == null) if (ServerSettings.DebugAi)
				trace('AI: craft WARNING id: ${itemToCraft.transTarget} transTarget.name == null!');
			myPlayer.say('Goto target ' + itemToCraft.transTarget.name);

			if (itemToCraft.transActor.id == 0 && player.heldObject.id != 0) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop heldobj at start since Empty is needed!');
				dropHeldObject(true);
				return true;
			}

			useTarget = itemToCraft.transTarget;
			useActor = itemToCraft.transActor;
			itemToCraft.transActor = null; // actor is allready in the hand
		} else {
			// check if actor is TIME
			if (itemToCraft.transActor.id == -1) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft Actor is TIME ');
				this.time += 1;
				// TODO wait some time, or better get next obj
				myPlayer.say('Wait...');
				itemToCraft.transActor = null;
				return true;
			}
			// check if actor is PLAYER
			if (itemToCraft.transActor.id == -2) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft Actor is PLAYER ');

				// TODO PLAYER interaction not supported yet
				myPlayer.say('Actor is player!?!');
				itemToCraft.transActor = null;
				return false;
			}

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft goto actor: ${itemToCraft.transActor.id} '
				+ itemToCraft.transActor.name);

			myPlayer.say('Goto actor ' + itemToCraft.transActor.name);

			dropTarget = itemToCraft.transActor;

			if (player.heldObject.id != 0) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop obj to pickup ${itemToCraft.transActor.name}');
				dropHeldObject(true);
				return true;
			}
		}

		return true;
	}

	private function searchBestObjectForCrafting(itemToCraft:IntemToCraft):IntemToCraft {
		var startTime = Sys.time();
		itemToCraft.transActor = null;
		itemToCraft.transTarget = null;

		var objToCraftId = itemToCraft.itemToCraft.parentId;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;

		var world = myPlayer.getWorld();
		var player = myPlayer.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var radius = 0;

		// TODO dont cut down tables <3371> if not needed

		while (radius < ServerSettings.AiMaxSearchRadius) {
			radius += ServerSettings.AiMaxSearchIncrement;
			itemToCraft.searchRadius = radius;

			// if(ServerSettings.DebugAi) trace('AI: ${myPlayer.name + myPlayer.id} craft: search radius: $radius');

			// reset objects so that it can be filled again
			itemToCraft.clearTransitionsByObjectId();

			// check if held object can be used to craft item
			var trans = transitionsByObjectId[player.heldObject.parentId];

			if (trans != null) {
				trans.closestObject = player.heldObject;
				trans.closestObjectDistance = 0;
				trans.closestObjectPlayerIndex = 0; // held in hand
			}

			// go through all close by objects and map them to the best transition
			for (ty in baseY - radius...baseY + radius) {
				for (tx in baseX - radius...baseX + radius) {
					if (this.isObjectNotReachable(tx, ty)) continue;

					var objData = world.getObjectDataAtPosition(tx, ty);

					if (objData.id == 0) continue;
					if (objData.dummyParent != null) objData = objData.dummyParent; // use parent objectdata

					// Ignore container with stuff inside
					// TODO consider contained objects
					if (objData.numSlots > 0) {
						var container = world.getObjectHelper(tx, ty);
						if (container.containedObjects.length > 0) {
							// if(ServerSettings.DebugAi) trace('AI: search IGNORE container: ${objData.description}');
							continue;
						}
					}

					// check if object can be used to craft item
					var trans = transitionsByObjectId[objData.id];
					if (trans == null) continue; // object is not useful for crafting wanted object

					var steps = trans.steps;
					var obj = world.getObjectHelper(tx, ty);
					var objQuadDistance = myPlayer.CalculateDistanceToObject(obj);

					if (trans.closestObject == null || trans.closestObjectDistance > objQuadDistance) {
						if (objQuadDistance > 4 && AiHelper.IsDangerous(myPlayer, obj)) continue;

						trans.secondObject = trans.closestObject;
						trans.secondObjectDistance = trans.closestObjectDistance;

						trans.closestObject = obj;
						trans.closestObjectDistance = objQuadDistance;

						continue;
					}

					if (trans.secondObject == null || trans.secondObjectDistance > objQuadDistance) {
						if (objQuadDistance > 4 && AiHelper.IsDangerous(myPlayer, obj)) continue;

						trans.secondObject = obj;
						trans.secondObjectDistance = objQuadDistance;

						continue;
					}
				}
			}

			// if(ServerSettings.DebugAi) trace('AI: craft: FINISHED objects ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius}');

			searchBestTransitionTopDown(itemToCraft);

            this.time += Sys.time() - startTime;

			if (itemToCraft.transActor != null) return itemToCraft;
		}

		return itemToCraft;
	}

	private function searchBestTransitionTopDown(itemToCraft:IntemToCraft) {
		var objToCraftId = itemToCraft.itemToCraft.parentId;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;

		var world = myPlayer.getWorld();
		var startTime = Sys.time();
		var count = 1;
		var objectsToSearch = new Array<Int>();

		itemToCraft.bestDistance = 99999999999999999;

		objectsToSearch.push(objToCraftId);
		transitionsByObjectId[0] = new TransitionForObject(0, 0, 0, null);
		transitionsByObjectId[-1] = new TransitionForObject(-1, 0, 0, null);
		transitionsByObjectId[objToCraftId] = new TransitionForObject(objToCraftId, 0, 0, null);
		transitionsByObjectId[0].closestObject = new ObjectHelper(null, 0);
		transitionsByObjectId[-1].closestObject = new ObjectHelper(null, -1);
		transitionsByObjectId[0].isDone = true;
		transitionsByObjectId[-1].isDone = true;

		var objToCraft = ObjectData.getObjectData(objToCraftId);

		while (objectsToSearch.length > 0) {
			if (count > 30000) break;

			var wantedId = objectsToSearch.shift();
			var wanted = ObjectData.getObjectData(wantedId);
			// if(ServerSettings.DebugAi) trace('Ai: craft: count: $count todo: ${objectsToSearch.length} wanted: ${wanted.description}');
			// var obj = transitionsByObjectId[wantedId];
			// var desc = obj == null ? 'NA' : ObjectData.getObjectData(obj.wantedObjId).name;

			if (wanted.carftingSteps < 0) continue; // TODO should not be < 0 if all transitions work
			// if(wanted.carftingSteps > objToCraft.carftingSteps + 5 || wanted.carftingSteps < 0) continue;
			// traif(ServerSettings.DebugAi) tracece('Ai: craft: count: $count todo: ${objectsToSearch.length} wanted: ${wanted.description} --> $desc steps: ${wanted.carftingSteps} > ${objToCraft.carftingSteps}');

			count++;

			var found = false;
			var transitions = world.getTransitionByNewActor(wantedId);
			found = found || DoTransitionSearch(itemToCraft, wantedId, objectsToSearch, transitions);

			var transitions = world.getTransitionByNewTarget(wantedId);
			found = found || DoTransitionSearch(itemToCraft, wantedId, objectsToSearch, transitions);

			if (itemToCraft.transActor != null) break;
			if (itemToCraft.bestDistance < 100) break;
		}

		var obj = ObjectData.getObjectData(objToCraftId);
		var descActor = itemToCraft.transActor == null ? 'NA' : itemToCraft.transActor.name;
		var descTarget = itemToCraft.transTarget == null ? 'NA' : itemToCraft.transTarget.name;

		// TODO fix name == null
		if (itemToCraft.transActor != null && itemToCraft.transActor.name == null)
			descActor += itemToCraft.transActor == null ? '' : ' ${itemToCraft.transActor.id} ${itemToCraft.transActor.description}';
		if (itemToCraft.transTarget != null && itemToCraft.transTarget.name == null)
			descTarget += itemToCraft.transTarget == null ? '' : ' ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description}';

		if (ServerSettings.DebugAiCrafting)
			trace('AI: craft: FOUND $count ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius} dist: ${itemToCraft.bestDistance} ${obj.name} --> $descActor + $descTarget');
	}

	private static function DoTransitionSearch(itemToCraft:IntemToCraft, wantedId:Int, objectsToSearch:Array<Int>, transitions:Array<TransitionData>):Bool {
		var found = false;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;
		var wanted = transitionsByObjectId[wantedId];
		var objToCraftId = itemToCraft.itemToCraft.parentId;

		for (trans in transitions) {
			// if(ServerSettings.DebugAi) trace('Ai: craft: ' + trans.getDesciption());
			if (trans.actorID == wantedId || trans.actorID == objToCraftId) continue;
			if (trans.targetID == wantedId || trans.targetID == objToCraftId) continue;

			// a oven needs 15 sec to warm up this is ok, but waiting for mushroom to grow is little bit too long!
			if (trans.calculateTimeToChange() > ServerSettings.AiIgnoreTimeTransitionsLongerThen) continue;

			var actor = transitionsByObjectId[trans.actorID];
			var target = transitionsByObjectId[trans.targetID];

			// Allow transition if new actor or target is closer to wanted object
			/*var tmpActor = transitionsByObjectId[trans.actorID];
				var actorSteps = tmpActor != null && tmpActor.objId > 0 ? tmpActor.steps : 10000;
				var tmpNewActor = transitionsByObjectId[trans.newActorID];
				var newActorSteps = tmpNewActor != null ? tmpNewActor.steps : 10000;

				var tmpTarget = transitionsByObjectId[trans.targetID];
				var targetSteps = tmpTarget != null && tmpTarget.objId > 0 ? tmpTarget.steps : 10000;
				var tmpNewTarget = transitionsByObjectId[trans.newTargetID];
				var newTargetSteps = tmpNewTarget != null ? tmpNewTarget.steps : 10000;

				if(trans.newActorID == objToCraftId) newActorSteps = 0; 
				if(trans.newTargetID == objToCraftId) newTargetSteps = 0; */

			// if(actorSteps + targetSteps <= newActorSteps + newTargetSteps) continue; // nothing is won
			// if(ServerSettings.DebugAi) trace('AI craft WANTED: <${GetName(wantedId)}> actorSteps: $actorSteps newActorSteps: $newActorSteps targetSteps: $targetSteps newTargetSteps: $newTargetSteps ' + trans.getDesciption(true));

			if (actor == null || target == null) {
				// if(ServerSettings.DebugAi) trace('Ai: craft: Skipped: ' + trans.getDesciption());
				continue;
			}

			// TODO should not be null must be bug in tansitions: Basket of Pig Bones + TIME  -->  Basket + Pig Bones#dumped
			// if(actor == null) transitionsByObjectId[trans.actorID] = new TransitionForObject(trans.actorID,0,0,null);
			// if(target == null) transitionsByObjectId[trans.targetID] = new TransitionForObject(trans.targetID,0,0,null);

			var actor = transitionsByObjectId[trans.actorID];
			var target = transitionsByObjectId[trans.targetID];

			var actorObj = actor.closestObject;
			var targetObj = actor == target ? actor.secondObject : target.closestObject;

            // TODO consider something like put thread in claybowls to get a thread
			if (actorObj == null && actor.wantedObjs.contains(wanted) == false) { 
				actor.wantedObjs.push(wanted);
			}
			if (targetObj == null && target.wantedObjs.contains(wanted) == false) {
				target.wantedObjs.push(wanted);
			}

			if (actorObj == null && actor.isDone == false) {
				// if(ServerSettings.DebugAi) trace('Ai: craft: a: wanted: $wantedId -- > ${actor.wantedObjId}');
				actor.wantedObjId = wantedId;
				actor.isDone = true;
				objectsToSearch.push(actor.objId);
			}

			if (targetObj == null && target.isDone == false) {
				// if(ServerSettings.DebugAi) trace('Ai: craft: t: wanted: $wantedId -- > ${target.wantedObjId}');
				target.wantedObjId = wantedId;
				target.isDone = true;
				objectsToSearch.push(target.objId);
			}

			if (actorObj == null && actor.craftActor == null) continue;
			if (targetObj == null && target.craftActor == null) continue;

			found = true;

			if (actorObj == null) {
				actorObj = actor.craftActor;
				targetObj = actor.craftTarget;
			} else if (targetObj == null) {
				actorObj = target.craftActor;
				targetObj = target.craftTarget;
			}

			// var desc = wanted == null ? 'NA' : ObjectData.getObjectData(wanted.wantedObjId).name;
			if (wanted.craftActor == null) {
				wanted.craftActor = actorObj;
				wanted.craftTarget = targetObj;
				wanted.craftTransFrom = trans;

				// if(wanted.wantedObjId > 0) objectsToSearch.unshift(wanted.wantedObjId);
				for (obj in wanted.wantedObjs) {
					if (obj.craftActor != null) {
						// if(ServerSettings.DebugAi) trace('Ai: craft: removed steps: ${wanted.steps} wanted: ${GetName(wanted.objId)} --> ${GetName(obj.objId)}');
						wanted.wantedObjs.remove(obj);
						continue;
					}

					// if(ServerSettings.DebugAi) trace('Ai: craft: steps: ${wanted.steps} wanted: ${GetName(wanted.objId)} --> ${GetName(obj.objId)}');

					obj.craftFrom = wanted;

					// objectsToSearch.remove(obj.objId);
					if (objectsToSearch.contains(obj.objId) == false) objectsToSearch.unshift(obj.objId);
				}
			}

			if (wantedId != objToCraftId) continue;

			var dist = actor.closestObjectDistance;

			dist += AiHelper.CalculateDistance(wanted.craftActor.tx, wanted.craftActor.ty, wanted.craftTarget.tx, wanted.craftTarget.ty);

			// TODO to work it needs to allow to process further
			if (dist < itemToCraft.bestDistance) {
				itemToCraft.bestDistance = dist;
				itemToCraft.transActor = actorObj;
				itemToCraft.transTarget = targetObj;
			}

			var actor = transitionsByObjectId[actorObj.id];
			var target = transitionsByObjectId[targetObj.id];
			var actorSteps = actor == null ? -1 : actor.steps;
			var targetSteps = target == null ? -1 : target.steps;
			var steps = actorSteps > targetSteps ? actorSteps : targetSteps;
			var trans = TransitionImporter.GetTrans(actorObj, targetObj);
			var desc = trans == null ? '${itemToCraft.transActor.name} + ${itemToCraft.transTarget.name} Trans Not found!' : trans.getDesciption();
			var objToCraft = ObjectData.getObjectData(objToCraftId);

			CalculateSteps(itemToCraft);

			// if(ServerSettings.DebugAi) trace('Ai: craft: steps: $bestSteps Distance: $bestDistance bestActor: ${itemToCraft.transActor.description} / target: ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description} ' + bestTrans.getDesciption());
			// if(ServerSettings.DebugAi) trace('Ai: craft DONE: ${objToCraft.name} dist: $dist steps: ${steps} $desc');

			return true;
		}

		return found;
	}

	private static function GetName(objId:Int):String {
		return ObjectData.getObjectData(objId).name;
	}

	private static function CalculateSteps(itemToCraft:IntemToCraft) {
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;
		var text = '';
		var obj = transitionsByObjectId[itemToCraft.itemToCraft.id];
		var objNoTimeWantedIndex = -1;
		var index = 0;

		itemToCraft.craftingList = new Array<Int>();
		itemToCraft.craftingTransitions = new Array<TransitionData>();

		for (i in 0...100) {
			if (obj.craftFrom == null) break;
			if (itemToCraft.craftingList.contains(obj.craftFrom.objId)) break; // TODO look why there is a circle?

			itemToCraft.craftingList.unshift(obj.craftFrom.objId);
			if (obj.craftTransFrom != null) itemToCraft.craftingTransitions.unshift(obj.craftTransFrom);

			obj = transitionsByObjectId[obj.craftFrom.objId];
		}

		var altActor = null;
		var altTarget = null;

		for (wantedId in itemToCraft.craftingList) {
			var wantedObj = ObjectData.getObjectData(wantedId);
			var trans = wantedObj.getTimeTrans();
			var isTimeWanted = trans == null ? true : itemToCraft.craftingList.contains(trans.newTargetID);
			var desc = trans == null ? '' : 'TIME: $isTimeWanted ${trans.autoDecaySeconds} ';

			if (isTimeWanted == false) {
				objNoTimeWantedIndex = index;
				var objNoTimeWanted = itemToCraft.craftingList[objNoTimeWantedIndex];
				var trans = itemToCraft.craftingTransitions[objNoTimeWantedIndex];
				var doFirst = trans.actorID == objNoTimeWanted ? trans.targetID : trans.actorID;
				var obj = transitionsByObjectId[doFirst];
				var dist:Float = -1;

				if (obj.closestObject != null) {
					dist = obj.closestObjectDistance;
					doFirst = 0;

					// if(ServerSettings.DebugAi) trace('Ai: craft TIME not wanted: ${GetName(objNoTimeWanted)} do first: ${GetName(doFirst)} dist: $dist ${trans.getDesciption()}');
				} else {
					altActor = obj.craftActor;
					altTarget = obj.craftTarget;

					if (ServerSettings.DebugAi)
						trace('Ai: craft TIME not wanted: ${GetName(objNoTimeWanted)} do first: ${GetName(doFirst)} trans: ${GetName(itemToCraft.transActor.id)} + ${GetName(itemToCraft.transTarget.id)}');
				}
			}

			text += '${wantedObj.name} $desc--> ';
			index++;
		}

		// Do first not time critical transitions
		if (altActor != null) {
			itemToCraft.transActor = altActor;
			itemToCraft.transTarget = altTarget;
		}

		var textTrans = '';
		for (trans in itemToCraft.craftingTransitions) {
			var actor = ObjectData.getObjectData(trans.actorID);
			var target = ObjectData.getObjectData(trans.targetID);
			var isTimeWanted = itemToCraft.craftingList.contains(trans.newTargetID);
			var desc = trans.autoDecaySeconds == 0 ? '' : 'TIME: $isTimeWanted ${trans.autoDecaySeconds} ';

			textTrans += '${actor.name} + ${target.name} $desc--> ';
		}

		var objToCraft = ObjectData.getObjectData(itemToCraft.itemToCraft.id);
		if (ServerSettings.DebugAiCrafting) trace('Ai: craft DONE items: ${objToCraft.name}: ${itemToCraft.craftingList.length} $text');
		if (ServerSettings.DebugAiCrafting) trace('Ai: craft DONE trans: ${objToCraft.name}: ${itemToCraft.craftingTransitions.length} $textTrans');
	}

	private function isMovingToPlayer(maxDistance = 10, followHuman:Bool = true):Bool {
		if (playerToFollow == null) {
			if (isChildAndHasMother()) {
				playerToFollow = myPlayer.getFollowPlayer();
			} else {
				// get close human player
				playerToFollow = myPlayer.getWorld().getClosestPlayer(20, followHuman);

				if (playerToFollow == null) return false;

				// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} follow player ${playerToFollow.p_id}');
			}
		}
		var distance = myPlayer.CalculateDistanceToPlayer(playerToFollow);

		if (distance > maxDistance) {
			var dist = maxDistance >= 10 ? 2 : 1;
			var randX = WorldMap.calculateRandomInt(2 * dist) - dist;
			var randY = WorldMap.calculateRandomInt(2 * dist) - dist;

			var done = myPlayer.gotoAdv(playerToFollow.tx + randX, playerToFollow.ty + randY);
			myPlayer.say('${playerToFollow.name}');

			if (myPlayer.isAi()) if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} age: ${myPlayer.age} dist: $distance goto player $done');

			return true;
		}

		return false;
	}

	// returns true if in process of dropping item
	private function isDropingItem():Bool {
		if (dropTarget == null) return false;
		if (myPlayer.isStillExpectedItem(dropTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} dropTarget changed meanwhile! ${dropTarget.name}');
			dropTarget = null;
			return false;
		}
		if (myPlayer.isMoving()) return true;

		var distance = myPlayer.CalculateDistanceToObject(dropTarget);
		// var myPlayer = myPlayer.getPlayerInstance();

		if (distance > 1) {
			var done = false;
			for (i in 0...5) {
				done = myPlayer.gotoObj(dropTarget);

				if (done) break;

				dropTarget = myPlayer.GetClosestObjectById(0); // empty
			}

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto drop: $done ${dropTarget.name} distance: $distance');
		} else {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop ${myPlayer.heldObject.description}');

			myPlayer.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);

			dropTarget = null;
		}

		return true;
	}

	private function isPickingupFood():Bool {
		if (foodTarget == null) return false;

		/*var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
		if (heldObjectIsEatable) {
			foodTarget = null;
			return false;
		}*/

		// check if food is still eatable. Maybe some one eat it
		if (myPlayer.isEatableCheckAgain(foodTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} ${foodTarget.description} ${foodTarget.id} food changed meanwhile!');

			foodTarget = null;
			return true;
		}

		if (myPlayer.isMoving()) return true;

		var distance = myPlayer.CalculateDistanceToObject(foodTarget);
		if (distance > 1) {
			var done = myPlayer.gotoObj(foodTarget);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto food target: dist: $distance $done');

			if (done == false) foodTarget = null; // search another one

			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);

			if (ServerSettings.DebugAi || done == false) trace('AAI: ${myPlayer.name + myPlayer.id} child drop for eating ${heldPlayer.name} $done');

			return true;
		}

		// if(ServerSettings.DebugAi) trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

		if (myPlayer.heldObject.id != 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop held object to pickup food');
			dropHeldObject();
			return true;
		}

		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = myPlayer.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} pickup food: ${foodTarget.name} $done');

		if (done == false) {
			if (ServerSettings.DebugAi) trace('AI: food Use failed! Ignore ${foodTarget.tx} ${foodTarget.ty} ');

			// TODO check why use is failed... for now add to ignore list
			this.addNotReachableObject(foodTarget, 30);
			foodTarget = null;
			return true;
		}

		this.didNotReachFood = 0;
		foodTarget = null;

		return true;
	}

	private function isEating():Bool {
		if (myPlayer.age < ServerSettings.MinAgeToEat) return false;
		if (myPlayer.canEat(myPlayer.heldObject) == false) return false;
		if (isHungry == false && myPlayer.isHoldingYum() == false) return false;

		// var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
		// if(heldObjectIsEatable == false) return false;

		var oldNumberOfUses = myPlayer.heldObject.numberOfUses;

		myPlayer.self(); // eat

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} Eat: held: ${myPlayer.heldObject.description}  newNumberOfUses: ${myPlayer.heldObject.numberOfUses} oldNumberOfUses: $oldNumberOfUses emptyFood: ${myPlayer.food_store_max - myPlayer.food_store}');

		this.didNotReachFood = 0;
		foodTarget = null;

		if (myPlayer.heldObject.objectData.foodValue <= 0) dropHeldObject(); // drop for example banana peal
		return true;
	}

	private function checkIsHungryAndEat():Bool {
		var player = myPlayer.getPlayerInstance();

		if (isHungry) {
			isHungry = player.food_store < player.food_store_max * 0.85;
		} else {
			isHungry = player.food_store < Math.max(3, player.food_store_max * 0.3);
		}

		if (isHungry && foodTarget == null) searchFoodAndEat();

		if (isHungry) myPlayer.say('F ${Math.round(myPlayer.getPlayerInstance().food_store)}'); // TODO for debugging
		if (isHungry && myPlayer.age < ServerSettings.MaxChildAgeForBreastFeeding) myPlayer.say('F');

		//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} F ${Math.round(playerInterface.getPlayerInstance().food_store)} P:  ${myPlayer.x},${myPlayer.y} G: ${myPlayer.tx()},${myPlayer.ty()}');

		return isHungry;
	}

	private function isUsingItem():Bool {
		if (useTarget == null) return false;
		if (myPlayer.isStillExpectedItem(useTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Use target changed meanwhile! ${useTarget.name}');
			useTarget = null;
			return false;
		}
		// only allow to go on with use if right actor is in the hand, or if actor will be empty
		// if(myPlayer.heldObject.id != useActor.id && useActor.id != 0)
		if (myPlayer.heldObject.id != useActor.id) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Use: not the right actor! ${myPlayer.heldObject.name} expected: ${useActor.name}');

			useTarget = null;
			useActor = null;
			// dropTarget = itemToCraft.transActor;

			dropHeldObject(true);

			return false;
		}
		if (myPlayer.isMoving()) return true;

		var distance = myPlayer.CalculateDistanceToObject(useTarget);
		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} Use: distance: $distance ${useTarget.description} ${useTarget.tx} ${useTarget.ty}');

		if (distance > 1) {
			var name = useTarget.name;
			var done = myPlayer.gotoObj(useTarget);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto useItem ${name} $done');

			if (done) myPlayer.say('Goto ${name} for use!'); else
				myPlayer.say('Cannot Goto ${name} for use!');

			/*
				if(done == false)
				{
					if(ServerSettings.DebugAi) trace('AI: GOTO useItem failed! Ignore ${useTarget.tx} ${useTarget.ty} '); 
					this.addNotReachableObject(useTarget);
					useTarget = null;
					itemToCraft.transActor = null;
					itemToCraft.transTarget = null;
			}*/

			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);

			if (ServerSettings.DebugAi || done == false) trace('AAI: ${myPlayer.name + myPlayer.id} child drop for using ${heldPlayer.name} $done');

			return true;
		}

		// Drop object to pickup actor
		if (myPlayer.heldObject.id != 0 && useActor.id == 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop obj to to have empty hand');
			dropHeldObject();
			return true;
		}

		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = myPlayer.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

		if (done) {
			itemToCraft.done = true;
			itemToCraft.countTransitionsDone += 1;
			var taregtObjectId = myPlayer.getWorld().getObjectId(useTarget.tx, useTarget.ty)[0];
			// if object to create is held by player or is on ground, then cound as done
			if (myPlayer.heldObject.parentId == itemToCraft.itemToCraft.parentId
				|| taregtObjectId == itemToCraft.itemToCraft.parentId) itemToCraft.countDone += 1;

			if (ServerSettings.DebugAi)
				trace('AAI: done: ${useTarget.name} ==> ${itemToCraft.itemToCraft.name} trans: ${itemToCraft.countTransitionsDone} finished: ${itemToCraft.countDone} FROM: ${itemToCraft.count}');
		} else {
			if (ServerSettings.DebugAi) trace('AAI: Use failed! Ignore: ${useTarget.name} ${useTarget.tx} ${useTarget.ty} ');

			// TODO check why use is failed... for now add to ignore list
			// TODO dont use on contained objects if result cannot contain (ignore in crafting search)
			this.addNotReachableObject(useTarget);
			useTarget = null;
			itemToCraft.transActor = null;
			itemToCraft.transTarget = null;
		}

		useTarget = null;

		return true;
	}

	public function addObjectWithHostilePath(obj:ObjectHelper) {
		if (obj == null) return;
		var index = WorldMap.world.index(obj.tx, obj.ty);
		objectsWithHostilePath[index] = 20; // block for 30 sec
	}

	public function isObjectWithHostilePath(tx:Int, ty:Int):Bool {
		var index = WorldMap.world.index(tx, ty);
		var notReachable = objectsWithHostilePath.exists(index);

		// if(notReachable) if(ServerSettings.DebugAi) trace('isObjectNotReachable: $notReachable $tx,$ty');

		return notReachable;
	}

	public function addNotReachableObject(obj:ObjectHelper, time:Float = 90) {
		addNotReachable(obj.tx, obj.ty, time);
	}

	public function addNotReachable(tx:Int, ty:Int, time:Float = 90) {
		var index = WorldMap.world.index(tx, ty);
		// if(notReachableObjects.exists(index)) return;
		notReachableObjects[index] = time; // block for 25 sec
	}

	public function isObjectNotReachable(tx:Int, ty:Int):Bool {
		var index = WorldMap.world.index(tx, ty);
		var notReachable = notReachableObjects.exists(index);

		// if(notReachable) if(ServerSettings.DebugAi) trace('isObjectNotReachable: $notReachable $tx,$ty');

		return notReachable;
	}

	// is called once a movement is finished (client side it must be called manually after a PlayerUpdate)
	public function finishedMovement() {}

	public function emote(player:PlayerInstance, index:Int) {}

	public function playerUpdate(player:PlayerInstance) {}

	public function mapUpdate(targetX:Int, targetY:Int, isAnimal:Bool = false) {}

	public function playerMove(player:PlayerInstance, targetX:Int, targetY:Int) {}

	public function dying(sick:Bool) {}

	public function newChild(child:PlayerInterface) {
		this.children.push(child);
	}
}

/* // with this AI crafts also something if it cannot reach the goal. Is quite funny to try out :)
	private function searchBestTransition(itemToCraft:IntemToCraft)
	{
		var objToCraftId = itemToCraft.itemToCraft.parentId;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;
		var bestDistance = 0.0;
		var bestSteps = 0;
		var bestTrans = null; 

		// search for the best doable transition with actor and target
		for(trans in transitionsByObjectId)
		{
			if(trans.closestObject == null) continue;

			var bestTargetTrans = null; 
			var bestTargetObject = null; 
			var bestTargetDistance = -1.0;
			var bestTargetSteps = -1;

			//  search for the best doable transition with target
			for(targetTrans in trans.transitions)                
			{
				var actorID = targetTrans.bestTransition.actorID;
				var targetID = targetTrans.bestTransition.targetID;
				var isUsingTwo = targetTrans.bestTransition.actorID == targetTrans.bestTransition.targetID;
				var traceTrans = AiHelper.ShouldDebug(targetTrans.bestTransition);

				if(traceTrans) trace('Target1: ' + targetTrans.bestTransition.getDesciption(true));

				// check if there are allready two of this // TODO if only one is needed skip second
				var tmpWanted = transitionsByObjectId[targetTrans.wantedObjId];
				if(tmpWanted != null && targetTrans.wantedObjId != objToCraftId && tmpWanted.closestObjectDistance > -1 && (isUsingTwo == false || tmpWanted.secondObjectDistance > -1)) continue;

				if(traceTrans) trace('Target2: ' + targetTrans.bestTransition.getDesciption(true));

				if(actorID != 0 && actorID != trans.closestObject.parentId) continue;

				var tmpObject = transitionsByObjectId[targetTrans.bestTransition.targetID];

				if(traceTrans) trace('Target3: ' + targetTrans.bestTransition.getDesciption(true));

				if(tmpObject == null) continue;

				if(tmpObject.closestObject == null) continue;

				if(traceTrans) trace('Target4: ' + targetTrans.bestTransition.getDesciption(true));

				var tmpDistance = tmpObject.closestObjectDistance;
				var tmpTargetObject = tmpObject.closestObject;

				if(isUsingTwo) // like using two milkweed
				{
					trace('Target4: AI: using two ' + targetTrans.bestTransition.actorID);

					if(tmpObject.secondObject == null) continue;

					trace('Target4: AI: using two 2');

					tmpDistance = tmpObject.secondObjectDistance;
					tmpTargetObject = tmpObject.secondObject;
				}

				if(traceTrans) trace('Target5: ' + targetTrans.bestTransition.getDesciption(true));

				var steps = targetTrans.steps;

				if(bestTargetTrans == null || bestTargetSteps > steps || (bestTargetSteps == steps && tmpDistance < bestTargetDistance))
				{
					if(traceTrans) trace('Target6: bestTarget ' + targetTrans.bestTransition.getDesciption(true));
					bestTargetTrans = targetTrans;
					bestTargetDistance = tmpDistance;
					bestTargetSteps = steps;
					bestTargetObject = tmpTargetObject;
				}
			}

			if(bestTargetObject == null) continue;

			//var targetObject = bestTargetObject.closestObject;

			if(bestTargetObject == null) continue;

			//var steps = trans.steps;
			var obj = trans.closestObject;
			var distance = trans.closestObjectDistance + bestTargetDistance; // actor plus target distance

			var traceTrans = bestTargetTrans.bestTransition.newActorID == 57;
			if(traceTrans) trace('Target7: ' + bestTargetTrans.bestTransition.getDesciption(true));

			if(itemToCraft.transActor == null || bestSteps > bestTargetSteps  || (bestTargetSteps == bestSteps && distance < bestDistance))
			{
				if(traceTrans) trace('Target8: ' + bestTargetTrans.bestTransition.getDesciption(true));

				itemToCraft.transActor = obj;
				itemToCraft.transTarget = bestTargetObject;                    
				bestSteps = bestTargetSteps;
				bestDistance = distance;
				bestTrans = bestTargetTrans;

				//if(bestTargetObject.parentId == 50) trace('TEST6 actor: ${obj.description} target: ${bestTargetObject.description} ');
			}
		} 
		
		if(itemToCraft.transActor != null) trace('ai: craft: steps: $bestSteps Distance: $bestDistance bestActor: ${itemToCraft.transActor.description} / target: ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description} ' + bestTrans.getDesciption());
}*/
