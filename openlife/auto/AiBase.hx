package openlife.auto;

import openlife.server.Biome.BiomeTag;
import haxe.ds.Map;
import haxe.Exception;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.server.Connection;
import openlife.server.GlobalPlayerInstance;
import openlife.server.NamingHelper;
import openlife.server.ServerAi;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;
import sys.thread.Thread;

using StringTools;
using openlife.auto.AiHelper;

abstract class AiBase
{
	public static var lastTick:Float = 0;
	public static var tick:Float = 0;
	public static var jumpToAi:AiBase = null;
	public static var blockedByAI = new Map<Int, Float>();

	final RAD:Int = MapData.RAD; // search radius

    public var myPlayer(default, default):PlayerInterface;
    public var seqNum = 1;
	public var time:Float = 1;
	public var waitingTime:Float = 0; // if Ai is manual set to wait. Before that it is allowed to finish drop
	private var timeLookedForDeadlyAnimalAtHome:Float = -1;
	//public var seqNum = 1;

	var feedingPlayerTarget:PlayerInterface = null;

	var animalTarget:ObjectHelper = null;
	var escapeTarget:ObjectHelper = null;
	public var foodTarget:ObjectHelper = null;
	public var dropTarget:ObjectHelper = null;
	var removeFromContainerTarget:ObjectHelper = null;
	var expectedContainer:ObjectHelper = null; // needs to be set if removeFromContainerTarget is set

	public var useTarget:ObjectHelper = null;
	public var useActor:ObjectHelper = null; // to check if the right actor is in the hand


	var dropIsAUse:Bool = false;

	var itemToCraftId = -1;
	var itemToCraftName:String = null;
	var itemToCraft:IntemToCraft = new IntemToCraft();

	var isHungry = false;

	var playerToFollow:PlayerInterface;
	var autoStopFollow = true;
	var timeStartedToFolow:Float = 0;

	var children = new Array<PlayerInterface>();

	var notReachableObjects = new Map<Int, Float>();
	var objectsWithHostilePath = new Map<Int, Float>();

	var craftingTasks = new Array<Int>();

	// counts how often one could not reach food because of dedly animals
	public var didNotReachFood:Float = 0;
	var didNotReachAnimalTarget:Float = 0;

	public var movedOneTile = false;
	public var failedCraftings = new Map<Int,Float>(); // cleared on birth

	public var isHandlingTemperature = false;
	public var justArrived = false;

	public var isCaringForFire = false;
	public var hasCornSeeds = false;
	public var hasCarrotSeeds = false;

	public var wasIdle:Float = 0;
	public var lastProfession:String = null;
	public var profession:Map<String,Float> = [];
	public var lastCheckedTimes:Map<String,Float> = [];

	public var toPlant = -1;
	public var lastPie = -1;
	public var countPies = 0;
	public var tryMoveNearestTileFirst = true;

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

			var aiCount = Connection.getAis().length;

			if(AiBase.tick % 20 != 0 && aiCount < ServerSettings.NumberOfAis) {
				var ai = ServerAi.createNewServerAiWithNewPlayer();
				// ai.player.delete(); // delete, so that they wont all spawn at same time
			}

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
			
			// block foodtarget // droptarget // usetarget of all Ais that are moving
			CalculateBlockedByAi(); 

			for (ai in Connection.getAis()) {
				if (ai.player.deleted) Macro.exception(ai.doRebirth(timePassedInSeconds));
				if (ai.player.deleted) continue;
				RemoveBlockedByAi(ai);
				Macro.exception(ai.doTimeStuff(timePassedInSeconds));
				AddToBlockedByAi(ai);
			}

			if (timeSinceStartCountedFromTicks > timeSinceStart) {
				var sleepTime = timeSinceStartCountedFromTicks - timeSinceStart;
				averageSleepTime += sleepTime;

				// if(ServerSettings.DebugAi) trace('sleep: ${sleepTime}');
				Sys.sleep(sleepTime);
			}
		}
	}

	private static function CalculateBlockedByAi() {
		blockedByAI = new Map<Int,Float>();

		for (ai in Connection.getAis()) {
			if (ai.player.deleted) continue;
			AddToBlockedByAi(ai);
		}
	}

	private static function AddToBlockedByAi(ai:ServerAi) {
		if (ai.player.deleted) return;
		if (ai.player.age < 3) return; 
		if (ai.player.isWounded()) return;
		if (ai.player.isMoving() == false) return;

		if(AddTargetBlockedByAi(ai.ai.foodTarget)) return;
		if(AddTargetBlockedByAi(ai.ai.dropTarget)) return;
		if(AddTargetBlockedByAi(ai.ai.useTarget, ai.ai.myPlayer.heldObject)) return;		
	}

	private static function RemoveBlockedByAi(ai:ServerAi) {
		RemoveTargetBlockedByAi(ai.ai.foodTarget);
		RemoveTargetBlockedByAi(ai.ai.dropTarget);
		RemoveTargetBlockedByAi(ai.ai.useTarget);	
	}

	private static function RemoveTargetBlockedByAi(obj:ObjectHelper){
		if(obj == null) return;
		var index = WorldMap.world.index(obj.tx, obj.ty);
		blockedByAI.remove(index);
	}

	// Fire 82 // Large Fast Fire 83 // Hot Coals 85 // Large Slow Fire 346 // Flash Fire 3029
	// Adobe Oven 237 // Hot Adobe Oven 250 
	// Adobe Kiln 238 // Firing Adobe Kiln 282
	// Forge 303 // Firing Forge 304 
	// Firing Newcomen Hammer 2238
	//public static var DontBlockByAi = [82, 83, 85, 346, 3029, 237, 250, 238, 282, 303, 304, 2238];

	// TODO might make problems with counting since object blocked by is not counted
	public static function AddTargetBlockedByAi(target:ObjectHelper, heldObj:ObjectHelper = null){
		if(target == null) return false;
		if(target.numberOfUses > 1) return true;
		if(target.objectData.isAnimal()) return true;

		// if useTarget does not change it can be used by more like Hot Adobe Oven 250 
		// should fix, that AI can seal Kiln only one time, but can use it often for making bowls 
		if(heldObj != null){
			var trans = TransitionImporter.GetTransition(heldObj.parentId, target.parentId);
			if(trans != null && target.parentId == trans.newTargetID) return false;
		}
		//if(DontBlockByAi.contains(target.parentId)) return true;
		AddObjBlockedByAi(target);
		return true;
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
		
		dropTarget = null;
		foodTarget = null;		
		useTarget = null;

		itemToCraftId = -1;
		itemToCraft = new IntemToCraft();

		isHungry = false;

		playerToFollow = null;
		autoStopFollow = true;
		children = new Array<PlayerInterface>();
		failedCraftings = new Map<Int,Float>();
		isCaringForFire = false;
		//addTask(837); //Psilocybe Mushroom
        //addTask(134); //Flint Arrowhead
        //addTask(82); // Fire
        //addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow
        //addTask(152); // Bow and Arrow

        //addTask(140); // Tied Skewer

        //addTask(148); // Arrow
		//addTask(292); // 292 basket
        //addTask(149); // Headless Arrow
        //addTask(146); // Fletching
        //addTask(151); // Jew Bow 
        //addTask(151); // Jew Bow 
        //addTask(59); // Rope 
		//addTask(82); // Fire
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
		//craftItem(292, 1, true); 
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

		if(movedOneTile){
			movedOneTile = false;

			//trace('AI: moved one tile!');
			var animal = AiHelper.GetCloseDeadlyAnimal(myPlayer);
			var deadlyPlayer = AiHelper.GetCloseDeadlyPlayer(myPlayer);

			Macro.exception(if (didNotReachFood < 5) if (escape(animal, deadlyPlayer)) return);
		}

		// if(didNotReachFood > 0) didNotReachFood -= timePassedInSeconds * 0.02;
        if (time > 1) time = 1; // wait max 10 sec
		if (time > 0) return;
		time += ServerSettings.AiReactionTime; // 0.5; // minimum AI reacting time
		itemToCraft.searchCurrentPosition = true;
		itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;

		if(wasIdle > 0) wasIdle -= ServerSettings.AiReactionTime / 10;
		// keep only last profession
		cleanUpProfessions();

		//if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} account:  ${myPlayer.account.id}');

		cleanupBlockedObjects();
		this.tryMoveNearestTileFirst = true;

		if (ServerSettings.AutoFollowAi && myPlayer.isHuman()) {
			// if(ServerSettings.DebugAi) trace('HUMAN');
			time = 0.2;
			isMovingToPlayer(2, false);
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

		//myPlayer.say('1');
		var startTime = Sys.time();

		var animal = AiHelper.GetCloseDeadlyAnimal(myPlayer);
		var deadlyPlayer = AiHelper.GetCloseDeadlyPlayer(myPlayer);

		Macro.exception(if (didNotReachFood < 5) if (escape(animal, deadlyPlayer)) return);
		//Macro.exception(if (didNotReachFood < 5 || myPlayer.food_store < 1) checkIsHungryAndEat());
		Macro.exception(checkIsHungryAndEat());
	
		Macro.exception(if (isDropingItem()) return);
		
		// give use high prio if close so that for example a stone can be droped on a pile before food piclup
		if(useTarget != null){
			var distance = myPlayer.CalculateQuadDistanceToObject(useTarget);

			if(distance < 100){
				if(ServerSettings.DebugAiSay)
					myPlayer.say('Close Use: d: $distance ${useTarget.name}'); 
				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} Close Use: d: $distance ${useTarget.name} ${useTarget.tx} ${useTarget.ty} isMoving: ${myPlayer.isMoving()}');

				Macro.exception(if (isUsingItem()) return);
			}
		}

		// check if manual waiting time is set. For example received a STOP command
		if(waitingTime > 1){ 
			time += 1;
			waitingTime -= 1;
			if(waitingTime < 0) waitingTime = 0;
			return;
		}

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		Macro.exception(if (myPlayer.age < ServerSettings.MinAgeToEat && isHungry) {
			if(isMovingToPlayer(5)) return;
			// if close enough to mother wait before trying to move again
			// otherwise child wants to catch mother and mother child but both run around
			// TODO move to tile which is closest to target
			isMovingToPlayer(3);
			this.time += 2.5; 
			
			return;
		}); // go close to mother and wait for mother to feed
		Macro.exception(if (isChildAndHasMother()) {
			if (isMovingToPlayer(4)) return;
			Macro.exception(if (handleTemperature()) return);
		});
		Macro.exception(if (myPlayer.isWounded() || myPlayer.hasYellowFever()) {
			isMovingToPlayer(2);
			return;
		}); // do nothing then looking for player

		Macro.exception(if (handleDeath()) return);
		Macro.exception(if (isEating()) return);

		if (playerToFollow != null && autoStopFollow == false){
			var time = TimeHelper.CalculateTimeSinceTicksInSec(timeStartedToFolow);
			if(time > 60 * 5) autoStopFollow = true; // max follow player for 5 min
		}

		// Only follow Ai if still cannot eat // TODO allow follow AI in certain cirumstances
		if(playerToFollow != null && autoStopFollow && myPlayer.age > ServerSettings.MinAgeToEat * 2) {
			playerToFollow = null;			
		}

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: isEating ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		// should be below isUsingItem since a use can be used to drop an hold item on a pile to pickup a baby
		Macro.exception(if (isFeedingChild()) return); 
		Macro.exception(if (isPickingupFood()) return);
		Macro.exception(if (isFeedingPlayerInNeed()) return);
		Macro.exception(if (isStayingCloseToChild()) return);
		Macro.exception(if (isUsingItem()) return);
		Macro.exception(if (isRemovingFromContainer()) return);		
		Macro.exception(if (killAnimal(animal)) return);
		Macro.exception(if (isMovingToPlayer(autoStopFollow ? 10 : 5)) return); // if ordered to follow stay closer otherwise give some space to work

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		if (myPlayer.isMoving()) return;
		
		Macro.exception(if (searchNewHomeIfNeeded()) return);
		
		// High priortiy takes
		itemToCraft.searchCurrentPosition = false;
		if(this.profession['Smith'] >= 2) Macro.exception(if (doSmithing()) return);
		if(this.profession['Potter'] >= 10) Macro.exception(if (doPottery()) return);

		itemToCraft.maxSearchRadius = 30;
		if(this.profession['Baker'] > 1) Macro.exception(if (doBaking()) return);
		itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;
		
		Macro.exception(if (isHandlingFire()) return);
		Macro.exception(if (isPickingupCloths()) return);		
		Macro.exception(if (handleTemperature()) return);

		itemToCraft.searchCurrentPosition = true;
		Macro.exception(if (shortCraft(0, 400, 10)) return); // pull out the carrots 
		Macro.exception(if (makeSharpieFood(5)) return); 
		Macro.exception(if (isHandlingGraves()) return);
		Macro.exception(if (isMakingSeeds()) return);
		//if(craftItem(283)) return;

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');
				
		// if(playerToFollow == null) return; // Do stuff only if close to player TODO remove if testing AI without player

		if (itemToCraftId > 0 && itemToCraft.countDone < itemToCraft.count) {
			if (ServerSettings.DebugAi) trace('AI: craft ${GetName(itemToCraftId)} tasks: ${craftingTasks.length}!');
			Macro.exception(if (craftItem(itemToCraftId)) return);
		}

		if (craftingTasks.length > 0) {
			for (i in 0...craftingTasks.length) {
				itemToCraftId = craftingTasks.shift();
				Macro.exception(if (craftItem(itemToCraftId)) return);
				craftingTasks.push(itemToCraftId);
			}
		}
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');
		Macro.exception(if(craftHighPriorityClothing()) return);
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		// medium priorty tasks
		if(myPlayer.age > 20) Macro.exception(if(craftMediumPriorityClothing()) return);

		itemToCraft.searchCurrentPosition = false;
		//if(this.profession['Baker'] > 0) Macro.exception(if (doBaking()) return);
		//if(this.profession['Potter'] > 0) Macro.exception(if (doPottery()) return);		
		if(this.profession['Smith'] > 0) Macro.exception(if (doSmithing()) return);
		//if(this.profession['WaterBringer'] > 0) Macro.exception(if (doWatering()) return);
		//if(this.profession['BasicFarmer'] > 0) Macro.exception(if (doBasicFarming()) return);
		//if(this.profession['Shepherd'] > 0) Macro.exception(if (isSheepHerding()) return);
		
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		itemToCraft.maxSearchRadius = 30;
		Macro.exception(if(fillBerryBowlIfNeeded()) return);		
		Macro.exception(if(makePopcornIfNeeded()) return);
		itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;
		
		var jobByAge:Int = Math.round(myPlayer.age / 2); // job prio switches every second year
		
		itemToCraft.maxSearchRadius = 30;
		for(i in 0...5){
			jobByAge = (jobByAge + i) % 5;
			if(jobByAge == 0) Macro.exception(if(doWatering()) return);				
			else if(jobByAge == 1) Macro.exception(if(doBasicFarming()) return);
			else if(jobByAge == 2) Macro.exception(if(doBaking()) return);
			else if(jobByAge == 3) Macro.exception(if(doPottery()) return);
			else if(jobByAge == 4) Macro.exception(if(isSheepHerding()) return);
		}
		itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;
		
		Macro.exception(if(isCuttingWood()) return);
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: isCuttingWood ${Math.round((Sys.time() - startTime) * 1000)}ms ');
		Macro.exception(if(doSmithing()) return);
		Macro.exception(if(makeFireFood()) return);
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: makeFireFood ${Math.round((Sys.time() - startTime) * 1000)}ms ');
		itemToCraft.searchCurrentPosition = true;		

		var cravingId = myPlayer.getCraving();
		itemToCraftId = cravingId;
		// 31 Gooseberry // 1121 Popcorn
		if(itemToCraftId == 31 || itemToCraftId == 1121) itemToCraftId = -1; 
		Macro.exception(if (cravingId > 0) if (craftItem(itemToCraftId)) return);

		if(myPlayer.age > 30) Macro.exception(if(craftLowPriorityClothing()) return);
		
		itemToCraft.searchCurrentPosition = false;	
		
		Macro.exception(if(doAdvancedFarming()) return);
		Macro.exception(if(makeStuff()) return);

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		// if there is nothing to do go home
		Macro.exception(if(isMovingToHome(4)) return);

		// Drop held object before doing noting
		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop obj before doing nothing');
			dropHeldObject();
			return;
		}

		time += 2.5;
		wasIdle += 1;

		// before do nothing try all professions
		//this.profession['firekeeper'] = 1;
		/*this.profession['Lumberjack'] = 1;
		this.profession['WaterBringer'] = 1;
		this.profession['BasicFarmer'] = 1;
		this.profession['AdvancedFarmer'] = 1;	
		this.profession['Shepherd'] = 1;	
		this.profession['Baker'] = 1;
		this.profession['FoodServer'] = 1;
		this.profession['Potter'] = 1;
		this.profession['gravekeeper'] = 1;
		this.profession['Hunter'] = 1;
		this.profession['ClothMaker'] = 1;
		this.profession['FireFoodMaker'] = 1;
		//this.profession['BowlFiller'] = 1;
		this.profession['Smith'] = 1;*/
		
		if(myPlayer.age > ServerSettings.MinAgeToEat){
			var rand = WorldMap.calculateRandomFloat();
			if(rand < 0.05) myPlayer.say('say make xxx to give me some work!');
			else if(rand < 0.2) myPlayer.say('nothing to do...');
		}
	}

	private function GetCraftAndDropItemsCloseToObj(target:ObjectHelper, whichObjId:Int, maxCount = 1, dist = 5, craft = true) : Bool {
		if(myPlayer.heldObject.parentId == whichObjId){
			var quadDist = myPlayer.CalculateQuadDistanceToObject(target);
			if(quadDist > dist * dist) return myPlayer.gotoObj(target);
			dropHeldObject(0);
			return true;
		}

		var count = AiHelper.CountCloseObjects(myPlayer, target.tx, target.ty, whichObjId, dist);
		if(count < maxCount && GetOrCraftItem(whichObjId, craft, dist)) return true;
		return false;
	}
	
	private function isCuttingWood(maxPeople = 1) : Bool {		
		if(myPlayer.firePlace == null) return false;
		
		if(hasOrBecomeProfession('Lumberjack', maxPeople) == false) return false;

		// Firewood 344
		var count = AiHelper.CountCloseObjects(myPlayer, myPlayer.firePlace.tx, myPlayer.firePlace.ty, 344, 15); // Firewood 344
		if(count < 2) this.profession['Lumberjack'] = 1;
		
		// Firewood 344
		if(this.profession['Lumberjack'] < 2 && count < 5 && GetCraftAndDropItemsCloseToObj(myPlayer.firePlace, 344, 10)) return true; 
		this.profession['Lumberjack'] = 2;

		if(cleanUp()) return true;

		return false;
	}

	private function pileUp(objId:Int, dist:Int) : Bool {
		var home = myPlayer.home;
		var held = myPlayer.heldObject;
		var objData = ObjectData.getObjectData(objId);
		var pileId = objData.getPileObjId();

		held.tx = myPlayer.tx;
		held.ty = myPlayer.ty;

		if(pileId < 1) return false;

		if(held.parentId == objId){
			var pile = myPlayer.GetClosestObjectToTarget(held, pileId, 10);
			//if(pile != null) trace('CLEANUP Pile: ${pile.name} numberOfUses: ${pile.numberOfUses} numUses: ${pile.objectData.numUses}');
			if(pile != null && pile.numberOfUses >= pile.objectData.numUses) pile = null;
			if(pile == null) pile = myPlayer.GetClosestObjectToTarget(held, objId, dist);
			//if(pile != null) trace('CLEANUP Pile: ${pile.name}');

			return useHeldObjOnTarget(pile);
		}
	
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, objId, dist, false);
		if(count < 2) return false;
			
		return PickupItem(objId);
	}

	private function cleanUp() : Bool {
		var home = myPlayer.home;
		var held = myPlayer.heldObject;

		// Basket of Charcoal 298
		if(shortCraftOnGround(298)) return true;

		//trace('cleanUp!');

		var target = GetForge();
		var isforge = target == null ? false : true;
		if(target == null) target = home; 
		var closeObj = AiHelper.GetClosestObjectToTarget(myPlayer,target, 1836, 4); // Stack of Flat Rocks 1836
		if(ServerSettings.DebugAi && closeObj != null) trace('cleanUp: ${closeObj.name}');
		if(closeObj != null) if(shortCraftOnTarget(0,closeObj)) return true;
		

		// TODO for now GetClosestObjectToTarget considers only mindistance to home (oven) not to forge
		var target = home;
		var isforge = false;

		var count = isforge ? AiHelper.CountCloseObjects(myPlayer,target.tx, target.ty, 291, 4, false) : 0; // Flat Rock 291
		//var max = isforge ? 3 : 0;
		var max = 3; // for now allow 3 also for oven since forge can be close to oven // TODO change 
		var mindistance = isforge ? 2 : 0;
		if(count > max){			
			var closeObj = AiHelper.GetClosestObjectToTarget(myPlayer,target, 291, 4, mindistance); // Flat Rock 291
			if(ServerSettings.DebugAi && closeObj != null) trace('cleanUp: ${closeObj.name}');
			if(closeObj != null){
				if(dropHeldObject()) return true;
				return PickupObj(closeObj);
			}
		}

		// Long Straight Shaft 67 
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 67, 10);
		if(count > 5){
			// Stone Hatchet 71 + Long Straight Shaft 67 = Kindling
			if(shortCraft(71, 67, 20)) return true; 
		}

		// Weak Skewer 852
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 852, 10);
		if(count > 5){
			// Stone Hatchet 71 + Weak Skewer 852 = Kindling
			if(shortCraft(71, 852, 20)) return true;
			if(held.parentId == 852) return dropHeldObject(0);
			
			// 0 + // Weak Skewer Pile 4060
			if(shortCraft(0, 4060, 10)) return true; 
		}

		if(pileUp(227, 30)) return true; // Straw 227
		if(pileUp(1115, 30)) return true; // Dried Ear of Corn 1115
		
		// Wet Clay Nozzle 285
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 285, 20);
		if(count > 1 || (count > 0 && held.parentId == 285)){
			//  Wet Clay Nozzle 285 + Wet Clay Nozzle 285 = Clay
			//trace('CLEANUP  Wet Clay Nozzle ${count} held: ${held.name}');
			if(shortCraft(285, 285, 30, false)) return true; 
		}

		//trace('CLEANUP  Clay with Nozzle held: ${held.name}');
		//  0 + Clay with Nozzle 2110 = Wet Clay Nozzle 285
		if(shortCraft(0, 2110, 20)) return true; 

		//trace('Small Lump of Clay Nozzle held: ${held.name}');
		// Small Lump of Clay 3891
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 3891, 20);
		if(count > 1){
			//trace('CLEANUP CLAY');
			//  Small Lump of Clay 3891 + Small Lump of Clay 3891 = Clay
			if(shortCraft(3891, 3891, 20)) return true; 
		}

		return false;
	}

	private function doCriticalStuff() {
		// get basic kindling
		var count = AiHelper.CountCloseObjects(myPlayer, myPlayer.firePlace.tx, myPlayer.firePlace.ty, 72, 15); // Kindling 72
		if(count < 3) this.profession['firekeeper'] = 1;
		if(count < 5) this.profession['firekeeper'] = 2;
		
		// Kindling 72
		if(this.profession['firekeeper'] < 2 && count < 5 && GetCraftAndDropItemsCloseToObj(myPlayer.firePlace, 72, 10)) return true; 
		this.profession['firekeeper'] = 2;

		if(makeFireFood()) return true;

		if(cleanUp()) return true;
		
		// take care that there is at least some basic farming
		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 399, 30); // Wet Planted Carrots
		if(closeObj == null) if(craftItem(399)) return true; // Wet Planted Carrots

		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 1110, 30); // Wet Planted Corn Seed
		if(closeObj == null) if(craftItem(1110)) return true; // Wet Planted Corn Seed	

		// more kindling
		if(this.profession['firekeeper'] < 3 && count < 10 && GetCraftAndDropItemsCloseToObj(myPlayer.firePlace, 72, 10)) return true; 
		this.profession['firekeeper'] = 3;
		
		return false;
	}

	private function isHandlingFire(maxProfession = 1) : Bool {
		var firePlace = myPlayer.firePlace;
		var heldId = myPlayer.heldObject.parentId;

		firePlace = AiHelper.GetCloseFire(myPlayer);

		if(firePlace == null){
			if(firePlace == null){
				var bestAiForFire = getBestAiForObjByProfession('firekeeper', myPlayer.home);
				if(bestAiForFire != null && bestAiForFire.myPlayer.id == myPlayer.id){
					// make shafts and try not to borrow them // 67 Long Straight Shaft
					var shaft = AiHelper.GetClosestObjectToPosition(myPlayer.home.tx, myPlayer.home.ty, 67, 20);
					if(shaft == null) shaft = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 67, 40);
					if(shaft == null) if(craftItem(67)) return true;

					if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Make new Fire: ${myPlayer.home.tx},${myPlayer.home.ty}');
					return craftItem(82); // Fire
				}
				return false;
			}
			
			myPlayer.firePlace = firePlace;
		}

		if (this.isObjectNotReachable(firePlace.tx, firePlace.ty)) return false;
		if (this.isObjectWithHostilePath(firePlace.tx, firePlace.ty)) return false;

		//var objId = WorldMap.world.getObjectId(firePlace.tx, firePlace.ty)[0];
		var objAtPlace = WorldMap.world.getObjectHelper(firePlace.tx, firePlace.ty);
		myPlayer.firePlace = objAtPlace;
		var objId = objAtPlace.parentId;

		// 83 Large Fast Fire // 346 Large Slow Fire // 3029 Flash Fire
		if(objId == 83 || objId == 346 || objId == 3029){
			if(hasOrBecomeProfession('firekeeper', maxProfession) == false) return false;

			itemToCraft.maxSearchRadius = 30; // craft only close
			Macro.exception(if (doCriticalStuff()) return true);
			itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;
		
			return false;
		}

		var isUrgent = objId == 85 && hasOrBecomeProfession('firekeeper', 3); // 85 Hot Coals 
		var bestAiForFire = isUrgent ? this : getBestAiForObjByProfession('firekeeper', myPlayer.firePlace);
		if(bestAiForFire == null || bestAiForFire.myPlayer.id != myPlayer.id) return false;

		if (ServerSettings.DebugAi) 
			trace('AAI: ${myPlayer.name + myPlayer.id} Checking Fire: ${firePlace.name} objAtPlace: ${objAtPlace.name} ${myPlayer.firePlace.tx},${myPlayer.firePlace.ty}');

		// 85 Hot Coals // 72 Kindling
		if(objId == 85){			
			if(heldId == 72){
				var done = useHeldObjOnTarget(firePlace);
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} t: ${TimeHelper.tick} Fire: Has Kindling Use On ==> Hot Coals!  ${firePlace.name} objAtPlace: ${objAtPlace.name} $done');
				if(ServerSettings.DebugAiSay)
					myPlayer.say('Use Kindling on ${firePlace.name} $done'); // hot coals
				return done;
			}
			else{
				itemToCraft.maxSearchRadius = 30; // craft only close
				Macro.exception(if (makeFireFood(5)) return true);
				itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;

				if(ServerSettings.DebugAiSay)
					myPlayer.say('Get Kindling For ${firePlace.name}');
				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} t: ${TimeHelper.tick} Fire: Get Kindling ==> ${firePlace.name} ');
				
				return GetOrCraftItem(72);
			}
		} 
		
		// 82 Fire // 72 Kindling // 344 Firewood
		if(objId == 82){
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Fire: Get Wood or Kindling ==> Fire!');

			if(heldId == 72 || heldId == 344){
				if(ServerSettings.DebugAiSay)
					myPlayer.say('Use On Fire');
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Fire: Has Kindling Or Wood Use On ==> Fire');
				return useHeldObjOnTarget(firePlace);
			}
			else{
				if(ServerSettings.DebugAiSay)
					myPlayer.say('Get Wood For Fire');
				var done = GetOrCraftItem(344);
				if(done) return true;
				else return GetOrCraftItem(72);
			}
		}

		myPlayer.firePlace = null;

		return false;
	}

	/*private function getBestAiForFire(fire:ObjectHelper) : AiBase{
		var ais = Connection.getAis();
		var bestAi = null;
		var bestQuadDist:Float = -1;

		for(serverAi in ais){
			var ai = serverAi.ai;
			var p = serverAi.player;

			if(p.age < ServerSettings.MinAgeToEat) continue;
			if(p.age > 58) continue;
			if(p.isWounded()) continue;
			if(p.food_store < 2) continue;
			if(p.home != myPlayer.home) continue;

			if(ai.isCaringForFire == false && p.id != myPlayer.id) continue;

			var quadDist = p.CalculateQuadDistanceToObject(fire);

			// avoid that ai changes if looking for wood or making fire
			if(ai.isCaringForFire == false) quadDist += 1600;

			if(bestAi != null && quadDist >= bestQuadDist) continue;

			bestQuadDist = quadDist;
			bestAi = ai;
		}

		if(bestAi != null){
			this.isCaringForFire = false;
			bestAi.isCaringForFire = true;
		}

		return bestAi;
	}*/

	private function countProfession(profession:String) : Float{
		var ais = Connection.getAis();
		var count = 0;
		
		for(serverAi in ais){
			var ai = serverAi.ai;
			var p = serverAi.player;

			if(p.deleted) continue;
			if(p.age < ServerSettings.MinAgeToEat) continue;
			if(p.age > 58 && profession != 'gravekeeper') continue;
			if(p.isWounded()) continue;
			if(p.food_store < 2) continue;
			if(p.home.tx != myPlayer.home.tx && p.home.ty != myPlayer.home.ty) continue;

			var hasProfession = ai.profession[profession] > 0;

			if(hasProfession == false) continue;

			count++;
		}

		return count;
	}

	private function getBestAiForObjByProfession(profession:String, obj:ObjectHelper) : AiBase{
		var ais = Connection.getAis();
		var bestAi = null;
		var bestQuadDist:Float = -1;

		for(serverAi in ais){
			var ai = serverAi.ai;
			var p = serverAi.player;

			if(p.deleted) continue;
			if(p.age < ServerSettings.MinAgeToEat) continue;
			if(p.age > 58 && profession != 'gravekeeper') continue;
			if(p.isWounded()) continue;
			if(p.food_store < 2) continue;
			if(p.home.tx != myPlayer.home.tx || p.home.ty != myPlayer.home.ty) continue;

			var hasProfession = ai.profession[profession] > 0;

			if(hasProfession == false && p.id != myPlayer.id) continue;
			//if(profession != 'Potter' && ai.profession['Potter'] >= 10) continue;
			//if(profession != 'Baker' && ai.profession['Baker'] > 1) continue;

			var quadDist = p.CalculateQuadDistanceToObject(obj);

			// avoid that ai changes if looking for wood or making fire
			if(hasProfession == false) quadDist += 400;

			if(bestAi != null && quadDist >= bestQuadDist) continue;

			bestQuadDist = quadDist;
			bestAi = ai;
		}

		if(bestAi != null){
			this.profession[profession] = 0;
			bestAi.profession[profession] = 1;
		}

		return bestAi;
	}

	public function isMakingSeeds() {
		// TODO check once every X seconds
		// TODO check at home too
		var seeds = AiHelper.GetClosestObjectById(myPlayer, 1115, null, 30);  // Dried Ear of Corn
		if(seeds == null) seeds = AiHelper.GetClosestObjectById(myPlayer, 1247, null, 30);  // Bowl with Corn Kernels		
		if(seeds == null) seeds = AiHelper.GetClosestObjectById(myPlayer, 4106, null, 30);  // Dumped Corn Kernels 4106
		if(seeds == null) seeds = AiHelper.GetClosestObjectById(myPlayer, 4107, null, 30);  // Corn Kernel Pile 4107
		
		this.hasCornSeeds = seeds != null;

		var seeds = AiHelper.GetClosestObjectById(myPlayer, 401, null, 20); // Seeding Carrots
		if(seeds == null) seeds = AiHelper.GetClosestObjectById(myPlayer, 2745, null, 20); // Bowl of Carrot Seeds

		this.hasCarrotSeeds = seeds != null;
		
		// TODO make seeds
		return false;
	}

	//isCaringForFire

	private function useHeldObjOnTarget(target:ObjectHelper) : Bool{
		if(target == null) return false;
		if (this.isObjectNotReachable(target.tx, target.ty)) return false;
		if (this.isObjectWithHostilePath(target.tx, target.ty)) return false;

		this.useTarget = target;
		this.useActor = new ObjectHelper(null, myPlayer.heldObject.parentId);
		this.useActor.tx = target.tx;
		this.useActor.ty = target.ty;

		return true;
	}

	private function isRemovingItemFromContainer(){
		
	}

	private function removeItemFromContainer(container:ObjectHelper) : Bool{
		if (this.isObjectNotReachable(container.tx, container.ty)) return false;
		if (this.isObjectWithHostilePath(container.tx, container.ty)) return false;
		
		removeFromContainerTarget = container;
		expectedContainer = new ObjectHelper(null, container.id);
		expectedContainer.tx = removeFromContainerTarget.tx;
		expectedContainer.ty = removeFromContainerTarget.ty;
		return true;
	}

	private function isHandlingGraves() : Bool {

		if(myPlayer.heldObject.parentId == 356) return dropHeldObject(); // Basket of Bones 356

		var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(lastCheckedTimes['grave']);
		var isGravekeeper = this.profession['gravekeeper'] > 0;
		if(passedTime < 10 && isGravekeeper == false) return false;
		lastCheckedTimes['grave'] = TimeHelper.tick;

		// Basket of Bones 356
		if(shortCraft(0, 356, 20)) return true; 
		
		//myPlayer.say('check for graves!');

		var heldId = myPlayer.heldObject.parentId;
		var grave = AiHelper.GetClosestObjectById(myPlayer, 357, null, 20); // Bone Pile
		if(grave == null) grave = AiHelper.GetClosestObjectById(myPlayer, 88, null, 10); // 88 Grave 
		if(grave == null) grave = AiHelper.GetClosestObjectById(myPlayer, 89, null, 20); // 89 Old Grave 
		if(grave == null) return false;

		//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: found!');

		if (this.isObjectNotReachable(grave.tx, grave.ty)) return false;
		if (this.isObjectWithHostilePath(grave.tx, grave.ty)) return false;

		//myPlayer.say('graves found!');

		// cannot touch own grave
		var account = grave.getOwnerAccount();
		if(account != null && account.id == myPlayer.account.id) {
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: its my grave acountID: ${account.id}!');
			return false; 
		}

		if(grave.containedObjects.length > 0){
			if(dropHeldObject(0)){
				if(ServerSettings.DebugAiSay) myPlayer.say('drop for remove from grave');
				if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: drop heldobj for remove');
				return true;
			}
			if(ServerSettings.DebugAiSay) myPlayer.say('remove from grave');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: remove from grave');
			return removeItemFromContainer(grave);
		}

		if(this.myPlayer.age < 50 && this.profession['gravekeeper'] < 1){
			var bestPlayer = getBestAiForObjByProfession('gravekeeper', grave);
			if(bestPlayer == null || bestPlayer.myPlayer.id != myPlayer.id) return false;
		}

		this.profession['gravekeeper'] = 1; 

		// pickup bones
		var floorId = WorldMap.world.getFloorId(grave.tx, grave.ty);
		if(floorId < 1) {
			// move bones if too close to home
			var quadDist = AiHelper.CalculateQuadDistanceBetweenObjects(myPlayer, myPlayer.home, grave);
			if(quadDist < 25) floorId = 1;
		}

		if(floorId > 0){
			if(heldId == 292){ // Basket
				if(ServerSettings.DebugAiSay) myPlayer.say('use basket on bones');
				if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: use basket on bones');
				if(myPlayer.heldObject.containedObjects.length > 0) return dropHeldObject();
				return useHeldObjOnTarget(grave);
			} 
			if(ServerSettings.DebugAiSay) myPlayer.say('get basket for bones');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: get or craft basket');

			return GetOrCraftItem(292); // Basket
		}

		// 850 Stone Hoe // 502 = Shovel
		if(heldId == 850 || heldId == 502){
			if(ServerSettings.DebugAiSay) myPlayer.say('dig in bones');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: dig in bones');
			return useHeldObjOnTarget(grave);
		}

		if(ServerSettings.DebugAiSay) myPlayer.say('get shovel for grave');
		if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: try to get hoe');

		// 850 Stone Hoe
		var quadDist = myPlayer.CalculateQuadDistanceToObject(myPlayer.home);

		// 850 Stone Hoe
		if(quadDist < 900) if(GetOrCraftItem(850)) return true;
		else if(GetItem(850)) return true;

		if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: try to get shovel');
		
		return GetItem(502); // 502 = Shovel
	}

	private function handleDeath() : Bool {
		if(myPlayer.age < 58.5) return false;

		this.profession = new Map<String, Float>(); // clear all professions
		this.profession['gravekeeper'] = 1; 

		Macro.exception(if (isRemovingFromContainer()) return true);	

		var rand = WorldMap.calculateRandomFloat();
		if(rand < 0.05) myPlayer.say('Good bye!');
		else if(rand < 0.1) myPlayer.say('Jasonius is calling me. Take care!');

		if(myPlayer.isMoving()) return true;

		var quadDist = myPlayer.CalculateQuadDistanceToObject(myPlayer.home);
		if(quadDist < 400 && isHandlingGraves()) return true;
		if(isMovingToHome(5)) return true;

		time += 2;

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} ${myPlayer.age} good bye!');

		dropHeldObject();
		
		return true;
	}
	
	private function handleTemperature() : Bool {
		var goodPlace = null;
		var text = '';
		var needWarming = myPlayer.isSuperCold() || (isHandlingTemperature && myPlayer.heat < 0.4);
		var needCooling = myPlayer.isSuperHot() || (isHandlingTemperature && myPlayer.heat > 0.6);

		if(needCooling){
			//trace('AAI: ${myPlayer.name + myPlayer.id} handle heat: too hot');
			goodPlace = myPlayer.GetCloseBiome([BiomeTag.SNOW, BiomeTag.PASSABLERIVER]);
			if(goodPlace == null) goodPlace = myPlayer.coldPlace;
			text = 'cool';
		}
		else if(needWarming){
			//trace('AAI: ${myPlayer.name + myPlayer.id} handle heat: too cold');
			goodPlace = myPlayer.firePlace;
			text = 'heat at fire';			
		}

		if(goodPlace == null && needWarming){
			goodPlace = myPlayer.GetCloseBiome([BiomeTag.DESERT, BiomeTag.JUNGLE]);
			if(goodPlace == null) goodPlace = myPlayer.warmPlace;
			text = 'heat';	
		}

		if(goodPlace == null){
			isHandlingTemperature = false;
			justArrived = false;
			return false;
		}

		isHandlingTemperature = true;

		var quadDistance = myPlayer.CalculateQuadDistanceToObject(goodPlace);
		var biomeId = WorldMap.world.getBiomeId(goodPlace.tx, goodPlace.ty);		
		var temperature = myPlayer.lastTemperature;	

		if (quadDistance < 2){	
			if(justArrived == false){
				justArrived = true;
				this.time += 3; // just relax
				return true;
			}
				
			if(myPlayer.heat > 0.5 && myPlayer.lastTemperature > 0.45){
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} does not help: $text player heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
				myPlayer.coldPlace = null; // this place does not help
				return false; 
			} 
			if(myPlayer.heat < 0.5 && myPlayer.lastTemperature < 0.55){
				myPlayer.warmPlace = null; // this place does not help
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} does not help: $text player heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
				return false; // this place does not help
			} 

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} do: $text player heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature}  dist: $quadDistance wait b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
			this.time += 3; // just relax
			return true;
		}

		// make sure to go directly to tile not to nearest
		if(goodPlace != myPlayer.firePlace) this.tryMoveNearestTileFirst = false;
		var done = myPlayer.gotoObj(goodPlace);
		this.tryMoveNearestTileFirst = true;
	
		if (quadDistance < 2) this.time += 4; // if you cannot reach dont try running there too often

		if(ServerSettings.DebugAiSay) myPlayer.say('going to $text');

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} do: $text player heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} dist: $quadDistance goto: $done');
			
		return done;
	}

	/*private function doCarrots() : Bool {
		if(shortCraft(0, 400, 40)) return true; 

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 400); // Carrot Row
		//if(closeObj != null && closeObj.numberOfUses > 2) if(craftItem(402)) return true; // Carrot TODO carrot (would also make wild carrots to carrot with bowl)
		return false;
	}*/

	private function isSheepHerding(maxProfession = 1) {
		var home = myPlayer.home;
		var distance = 30;

		//if(craftItem(1113)) return true; // Ear of Corn
		if(hasOrBecomeProfession('Shepherd', maxProfession) == false) return false;

		// Domestic Sheep 575
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 575, 20);

		if(count < 10){
			// Bowl of Gooseberries and Carrot 258 + Hungry Domestic Lamb 604
			if(shortCraft(258, 604, distance)) return true;

			// Bowl of Gooseberries and Carrot 258 + Domestic Lamb 542
			if(shortCraft(258, 542, distance)) return true;
		}

		// Count all the Sheep Dung 899
		var countDung = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 899, distance);
		if(countDung > 0){
			// Composting Compost Pile 790
			var countCompost = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 790, distance);
			// Composted Soil 624
			countCompost += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 624, distance);

			// Composting Compost Pile 625
			if(countCompost < 5 && craftItem(790)) return true;

			// TODO pile dung
			// Shovel of Dung 900
			//return GetOrCraftItem(900);
		}

		// Feed: Bowl of Gooseberries and Carrot 258 + Shorn Domestic Sheep 576
		if(shortCraft(258, 576, distance)) return true;

		if(count < 10){
			// Bowl of Gooseberries and Carrot 258 + Domestic Sheep 575
			if(shortCraft(258, 575, distance)) return true;
		}

		if(count > 5 ){
			// Knife 560 + Shorn Domestic Sheep 576
			if(shortCraft(560, 576, distance)) return true;

			// Knife 560 + Domestic Sheep 575
			if(shortCraft(560, 575, distance)) return true;
		}

		this.profession['Shepherd'] = 0;

		return false;
	}

	private function doBasicFarming(maxProfession = 2) {
		var home = myPlayer.home;
		var heldObject = myPlayer.heldObject;
		var distance = 30;
		//var distance:Int = Math.round(10 + 10 * this.profession['BasicFarmer']);

		//if(craftItem(1113)) return true; // Ear of Corn
		if(hasOrBecomeProfession('BasicFarmer', maxProfession) == false) return false;

		if(shortCraft(0, 400, distance)) return true; // pull out the carrots 
		if(shortCraft(900, 625, distance)) return true; // Shovel of Dung 900 + Wet Compost Pile 625
		if(shortCraft(0, 1112, distance)) return true; // 0 + Corn Plant --> Ear of Corn
		if(shortCraft(34, 1113, distance)) return true; // Sharp Stone + Ear of Corn --> Shucked Ear of Corn

		if(shortCraft(139, 2832, distance)) return true; // Skewer + Tomato Sprout
		if(shortCraft(139, 4228, distance)) return true; // Skewer + Cucumber Sprout
		if(shortCraft(0, 2837, distance)) return true; // 0 + Hardened Row with Stake

		if(shortCraft(502, 1146, distance)) return true; // Shovel + Mature Potato Plants 1146
		if(shortCraft(0, 4144, distance)) return true; // Shovel + Dug Potatoes 4144

		// water
		//if(doWatering(1)) return true;

		// 1: Prepare Soil
		if(shortCraft(1137, 848, 30)) return true;
		//trace('Fertile Soil Pile!');
		// Basket of Soil
		if(shortCraftOnGround(336)) return true;		

		if(heldObject.parentId == 336) this.profession['BasicFarmer'] = 1; // need more soil

		// Fertile Soil Pile 1101
		var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1101, 15); 
		// Fertile Soil 1138
		count += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1138, 15); 

		if(count < 1) this.profession['BasicFarmer'] = 1;
		if(this.profession['BasicFarmer'] < 2){
			if (ServerSettings.DebugAiSay) myPlayer.say('BasicFarmer: soil: $count');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doBasicFarming:${profession['BasicFarmer']} soil: $count');			
			//var max = this.profession['BasicFarmer'] < 2 ? 3 : 1;
			if(count < 4) if(craftItem(336)) return true; // Basket of Soil
			else this.profession['BasicFarmer'] = 2;
		}
				
		if(this.profession['BasicFarmer'] < 2.5){
			// Domestic Gooseberry Bush
			var countBushes = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 391, distance);
			// Vigorous Domestic Gooseberry Bush 1134
			countBushes += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1134, distance);
			// Gooseberry Sprout
			countBushes += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 219, distance);
			// Wet Planted Gooseberry Seed
			countBushes += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 217, distance);
			
			if(countBushes < 9){
				if(heldObject.parentId == 1137){
					// Bowl of Soil 1137 + Dying Gooseberry Bush 389
					if(shortCraft(1137, 389, 30)) return true; 
					// Bowl of Soil 1137 + Languishing Domestic Gooseberry Bush 392
					if(shortCraft(1137, 392, 30)) return true; 
				}
				// TODO there seems to be a bug with maxuse transitions on pile of soil
				// Clay Bowl 235 + Fertile Soil Pile 1101 --> Bowl of Soil 1137
				if(shortCraft(235, 1101, 30)) return true; 
			}

			this.profession['BasicFarmer'] = 2.5;
		}

		var countRows = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1136, 30); // Shallow Tilled Row
		countRows += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 213, 30); // Deep Tilled Row 213

		if(countRows < 1) this.profession['BasicFarmer'] = 2;
		// 2: Prepare Shallow Tilled Rows
		if(this.profession['BasicFarmer'] < 3){
			var countBowls = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 235, 30); //  Clay Bowl 235
			if(heldObject.parentId == 235) countBowls += 1;
			
			if (ServerSettings.DebugAiSay) myPlayer.say('BasicFarmer: shallowrows: $countRows bowls: $countBowls');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doBasicFarming:${profession['BasicFarmer']} shallowrows: $countRows bowls: $countBowls');			

			if(countBowls < 1 && doPottery(3)) return true;

			if(countRows < 6){
				// TODO there seems to be a bug with maxuse transitions on pile of soil
				// Bowl of Soil 1137 + Hardened Row 848 --> Shallow Tilled Row
				if(heldObject.parentId == 1137 && shortCraft(1137, 848, 30)) return true; 
				// Clay Bowl 235 + Fertile Soil Pile 1101 --> Bowl of Soil 1137
				if(shortCraft(235, 1101, 30)) return true; 
			}
			else this.profession['BasicFarmer'] = 3;
		}

		// 3: Prepare Deep Tilled Rows
		if(this.profession['BasicFarmer'] < 4){
			if (ServerSettings.DebugAiSay) myPlayer.say('BasicFarmer: Prepare Deep Tilled Rows');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doBasicFarming:${profession['BasicFarmer']} Prepare Deep Tilled Rows');			
			if(shortCraft(850, 1136, 30)) return true; // Stone Hoe + Shallow Tilled Row --> Deep Tilled Row
			this.profession['BasicFarmer'] = 4;
		}

		if(this.profession['BasicFarmer'] < 5){			
			var countPlantedCarrots = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 399, 30); // Wet Planted Carrots 399
			countPlantedCarrots += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 396, 30); // Dry Planted Carrots 396
			if (ServerSettings.DebugAiSay) myPlayer.say('BasicFarmer: Planeted Carrots: $countPlantedCarrots');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doBasicFarming:${profession['BasicFarmer']} Planeted Carrots: $countPlantedCarrots');			
			//if(countPlanetCarrots < 5) if(craftItem(399)) return true; // Wet Planted Carrots
			if(countPlantedCarrots < 5) if(craftItem(396)) return true; // Dry Planted Carrots 396
			else this.profession['BasicFarmer'] = 5;
		}

		if(this.profession['BasicFarmer'] < 6){
			// let 5 wheat stay for seeds and so that it looks nice
			var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 242, 30); // Ripe Wheat
			if(count > 5) if(craftItem(224)) return true; // Harvested Wheat 

			var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 224, 30); // Harvested Wheat
			if(closeObj != null) if(craftItem(225)) return true; // Wheat Sheaf
			
			var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 225, 30); // Wheat Sheaf
			if(closeObj != null) if(craftItem(226)) return true; // Threshed Wheat	

			var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 229, 30); // Wet Planted Wheat 229
			count += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 228, 30); // Dry Planted Wheat 228
			if(count < 5) if(craftItem(228)) return true; // Dry Planted Wheat 228
			this.profession['BasicFarmer'] = 6;
		}

		if(this.profession['BasicFarmer'] < 7){
			var count = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1110, 30); // Wet Planted Corn Seed 1110
			count += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 1109, 30); // Dry Planted Corn Seed
			if(count < 3) if(craftItem(1109)) return true; // Dry Planted Corn Seed
			this.profession['BasicFarmer'] = 7;
		}

		this.profession['BasicFarmer'] = 1;

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 2831); // Wet Planted Tomato Seed
		//if(closeObj == null) if(craftItem(2831)) return true; // Wet Planted Tomato Seed

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 242, null, 20); // Ripe Wheat
		//if(closeObj != null) if(craftItem(224)) return true; // Harvested Wheat		

		// Composting Compost Pile 790
		var countCompost = AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 790, 30);
		// Composted Soil 624
		countCompost += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 624, 30);

		// Composting Compost Pile 790
		if(countCompost < 3 && craftItem(790)) return true; 

		// Wet Compost Pile 625
		countCompost += AiHelper.CountCloseObjects(myPlayer,home.tx, home.ty, 625, 30);

		// Wet Compost Pile 625
		if(countCompost < 3 && craftItem(625)) return true; 

		if(doWatering(3)) return true;

		Macro.exception(if (isSheepHerding(2)) return true);

		// check if there is a Tilled Row already before creating a new one
		var deepRow = AiHelper.GetClosestObjectToHome(myPlayer, 213, 20); // Deep Tilled Row
		if(deepRow == null) if(shortCraft(850, 1138, 30)) return true; // Stone Hoe + Fertile Soil --> Shallow Tilled Row
		//if(deepRow == null) closeObj = AiHelper.GetClosestObjectById(myPlayer, 1138, null, 20); // Fertile Soil
		//if(closeObj != null) if(craftItem(1136)) return true; // Shallow Tilled Row

		//if(myPlayer.age < 15 && makeFireWood()) return true;

		if(myPlayer.age < 20 && makeSharpieFood()) return true;

		this.profession['BasicFarmer'] = 0;

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doBasicFarming:${profession['BasicFarmer']} nothing to do');			

		return false;
	}

	/**private function doBasicFarming() {
		//if(craftItem(1113)) return true; // Ear of Corn
		if(shortCraft(0, 1112)) return true; // 0 + Corn Plant --> Ear of Corn

		if(hasOrBecomeProfession('BasicFarmer', 2) == false) return false;

		if(shortCraft(34, 1113)) return true; // Sharp Stone + Ear of Corn --> Shucked Ear of Corn

		if(shortCraft(139, 2832, 20)) return true; // Skewer + Tomato Sprout
		if(shortCraft(139, 4228, 20)) return true; // Skewer + Cucumber Sprout

		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 399, 20); // Wet Planted Carrots
		if(closeObj == null) if(craftItem(399)) return true; // Wet Planted Carrots

		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 1110, 20); // Wet Planted Corn Seed
		if(closeObj == null) if(craftItem(1110)) return true; // Wet Planted Corn Seed

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 2831); // Wet Planted Tomato Seed
		//if(closeObj == null) if(craftItem(2831)) return true; // Wet Planted Tomato Seed

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 242, null, 20); // Ripe Wheat
		//if(closeObj != null) if(craftItem(224)) return true; // Harvested Wheat
		
		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 224, 20); // Harvested Wheat
		if(closeObj != null) if(craftItem(225)) return true; // Wheat Sheaf
		
		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 225, 20); // Wheat Sheaf
		if(closeObj != null) if(craftItem(226)) return true; // Threshed Wheat

		//trace('Fertile Soil Pile!');
		if(shortCraftOnGround(336)) return true; // Basket of Soil

		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 1101, 20); // Fertile Soil Pile
		if(closeObj == null && craftItem(336)) return true; // Basket of Soil

		var closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 624,20); // Composted Soil
		if(closeObj == null) closeObj = AiHelper.GetClosestObjectToHome(myPlayer, 790, 20); // Composting Compost Pile
		if(closeObj == null && craftItem(790)) return true; // Composting Compost Pile

		//var hardenedRow = AiHelper.GetClosestObjectById(myPlayer, 848, null, 15); // Hardened Row
		//if(hardenedRow != null) if(craftItem(1136)) return true; // Shallow Tilled Row
		if(shortCraft(1137, 848, 15)) return true; // Bowl of Soil + Hardened Row --> Shallow Tilled Row

		//var closeObj = AiHelper.GetClosestObjectById(myPlayer, 1136, null, 20); // Shallow Tilled Row
		//if(closeObj != null) if(craftItem(213)) return true; // Deep Tilled Row
		if(shortCraft(850, 1136, 15)) return true; // Stone Hoe + Shallow Tilled Row --> Deep Tilled Row

		// check if there is a Tilled Row already before creating a new one
		var closeObj = null;
		var deepRow = AiHelper.GetClosestObjectToHome(myPlayer, 213, 20); // Deep Tilled Row
		if(deepRow == null) if(shortCraft(850, 1138, 15)) return true; // Stone Hoe + Fertile Soil --> Shallow Tilled Row
		//if(deepRow == null) closeObj = AiHelper.GetClosestObjectById(myPlayer, 1138, null, 20); // Fertile Soil
		//if(closeObj != null) if(craftItem(1136)) return true; // Shallow Tilled Row

		//if(myPlayer.age < 15 && makeFireWood()) return true;

		if(myPlayer.age < 20 && makeSharpieFood()) return true;

		this.profession['BasicFarmer'] = 0;

		return false;
	}**/

	private function shortCraftOnGround(actorId:Int){		
		if(myPlayer.heldObject.parentId == actorId){
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} shortCraftOnGround: wanted: ${actorId} ${myPlayer.heldObject.name}');
			var target = AiHelper.GetClosestObjectById(myPlayer, 0, null, 20);
			return useHeldObjOnTarget(target);
		} 
		return GetItem(actorId);	
	}

	private function shortCraft(actorId:Int, targetId:Int, distance:Int = 20, craftActorIfNeeded = true) : Bool {
		var target = AiHelper.GetClosestObjectById(myPlayer, targetId, null, distance);
		return shortCraftOnTarget(actorId, target, craftActorIfNeeded);
	}
	
	private function shortCraftOnTarget(actorId:Int, target:ObjectHelper, craftActorIfNeeded = true) : Bool {
		if(target == null) return false;
		var targetId = target.parentId;
		// dont use carrots if seed is needed // 400 Carrot Row
		if (targetId == 400 && hasCarrotSeeds == false && target.numberOfUses < 3) return false;

		if(myPlayer.heldObject.parentId == actorId) return useHeldObjOnTarget(target);

		var actorData = ObjectData.getObjectData(actorId);
		
		//if (ServerSettings.DebugAiSay) myPlayer.say('get ${actorData.name} to craft target: ${target.name}');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} shortCraft: wanted actor: ${actorData.name} + target: ${target.name} held: ${myPlayer.heldObject.name}');

		if(actorId == 0) return dropHeldObject();
		return GetOrCraftItem(actorId, craftActorIfNeeded);		
	}

	private function GetKiln() {
		var home = myPlayer.home;

		// Wood-filled Adobe Kiln 281
		var kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 281, 20, null, myPlayer);
		// Adobe Kiln 238
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 238 , 20, null, myPlayer); 
		// Firing Adobe Kiln 282
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 282, 20, null, myPlayer);
		// Sealed Adobe Kiln 294
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 294, 20, null, myPlayer);
		// Firing Adobe Kiln Sealed 293
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 293, 20, null, myPlayer);

		return kiln;
	}

	private function doPottery(maxPeople:Int = 1) : Bool {
		var home = myPlayer.home;

		if(hasOrBecomeProfession('Potter', maxPeople) == false) return false;
		if(home == null) return false;

		if(shortCraftOnGround(283)) return true; // Wooden Tongs with Fired Bowl
		if(shortCraftOnGround(241)) return true; // Fired Plate in Wooden Tongs
		//if(shortCraftOnGround(284)) return true; // Wet Bowl in Wooden Tongs
		//if(shortCraftOnGround(240)) return true; // Wet Plate in Wooden Tongs
		
		var countWetBowl = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 233, 15, false); // Wet Clay Bowl 233
		var countWetPlate = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 234, 15, false); // Wet Clay Plate 234
		
		countWetBowl += AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 284, 15, false); // Wet Bowl in Wooden Tongs
		countWetPlate += AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 240, 15, false); // Wet Plate in Wooden Tongs

		if(myPlayer.heldObject.parentId == 284) countWetBowl += 1; // Wet Bowl in Wooden Tongs
		if(myPlayer.heldObject.parentId == 240) countWetPlate += 1; // Wet Plate in Wooden Tongs

		// Firing Adobe Kiln 282
		var kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 282, 20, null, myPlayer);
		// Firing Forge 304
		var forgeOnFire = kiln != null ? null : AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 304, 20, null, myPlayer);
		
		if(kiln != null || forgeOnFire != null) {
			this.profession['Potter'] = 10;
			if(doPotteryOnFire(countWetBowl, countWetPlate)) return true;
		}

		if(shortCraft(0,294)) return true; // unseal Sealed Adobe Kiln 294 ==> Adobe Kiln with Charcoal
		if(shortCraft(292,299)) return true; // Basket 299 + Adobe Kiln with Charcoal 299 --> Adobe Kiln

		// Wood-filled Adobe Kiln 281
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 281, 20, null, myPlayer);
		// Adobe Kiln 238
		if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 238 , 20, null, myPlayer); 
		// Sealed Adobe Kiln 294
		//if(kiln == null) kiln = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 294, 20, null, myPlayer);

		if(this.profession['Potter'] < 2 && countWetBowl + countWetPlate < 4){
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 126, 20); // Clay 126
			if(count < 5 && gatherClay(kiln)) return true; // home is used if there is no kiln
		}

		this.profession['Potter'] = 2; // dont get new clay --> do some pottery first

		if(kiln == null) return false;

		var countBowl = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 235, 15); //  Clay Bowl 235
		var countPlate = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 236, 15); //  Clay Plate 236
		var maxtBowls = 5;
		var maxtPlates = 5;

		if(countBowl >= maxtBowls && countPlate >= maxtPlates && (countWetBowl + countWetPlate < 3)){
			this.profession['Potter'] = 0;
			return false;
		}

		var countClayOnFloor = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 126, 15, false); // Clay 126		

		countBowl += countWetBowl;
		countPlate += countWetPlate;

		var neededBols = countBowl > maxtBowls ? 0 : maxtBowls - countBowl;
		var neededPlates = countPlate > maxtPlates ? 0 : maxtPlates - countPlate;
		var neededClay = 0;

		neededClay += neededBols;
		neededClay += neededPlates;
		if(neededClay > 6) neededClay = 6;
		
		//if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doPottery: neededClay: $neededClay WetBowl: ${countWetBowl} WetPlate: $countWetPlate ');

		if(this.profession['Potter'] < 3 && countClayOnFloor < neededClay && countWetBowl + countWetPlate < 4){
			if (ServerSettings.DebugAiSay) myPlayer.say('Do Pottery get clay from pile $neededClay');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doPottery: get clay from pile');

			if(shortCraft(0,3905)) return true; // Pile of Clay 3905
		}

		this.profession['Potter'] = 3;

		if (ServerSettings.DebugAiSay) myPlayer.say('Do Pottery neededClay $neededClay');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doPottery: neededClay: $neededClay WetBowl: ${countWetBowl} WetPlate: $countWetPlate ');
		
		if(shortCraft(33,126)) return true; //Stone 33, Clay 126 --> Wet Clay Bowl 233
		if(countBowl > countPlate && shortCraft(33,233)) return true; //Stone 33, Wet Clay Bowl 233 --> Wet Clay Plate 234

		this.profession['Potter'] = 10;
		if(doPotteryOnFire(countWetBowl, countWetPlate)) return true;

		this.profession['Potter'] = 0;

		return false;
	}

	private function doPotteryOnFire(countWetBowl:Int, countWetPlate:Int) : Bool {
		if (ServerSettings.DebugAiSay) myPlayer.say('make bowl $countWetBowl');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doPottery: make bowl $countWetBowl');

		if(countWetBowl > 0 && craftItem(283)) return true; // Wooden Tongs with Fired Bowl

		if (ServerSettings.DebugAiSay) myPlayer.say('make Plate $countWetPlate');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doPottery: make Plate $countWetPlate');
		if(countWetPlate > 0 && craftItem(241)) return true; // Fired Plate in Wooden Tongs

		return false;
	}

	private function gatherClay(kiln:ObjectHelper) : Bool {
		var home = myPlayer.home;
		var heldObject = myPlayer.heldObject;
		//var heldContained = heldObject.containedObjects.length;
		
		if(home == null) return false;
		if(kiln != null) home = kiln;

		var distanceToHome = myPlayer.CalculateQuadDistanceToObject(home);
		var clayPit = AiHelper.GetClosestObjectById(myPlayer, 409, null, 80); // Clay Pit 409
		var clayDeposit = AiHelper.GetClosestObjectById(myPlayer, 125, null, 80); // Clay Deposit 125
		
		if(clayDeposit == null) clayDeposit = clayPit; // TODO use closest implement: GetClosestObjectByIds 
		var distanceToClayDeposit = clayDeposit == null ? -1 : myPlayer.CalculateQuadDistanceToObject(clayDeposit);
		//if(clayDeposit == null) return false;

		// holding Basket 292
		if(heldObject.parentId == 292){
			// bring basket home if full
			if(heldObject.containedObjects.length > 2){
				if(distanceToHome <= 100) return dropHeldObject();

				var done = myPlayer.gotoObj(home);

				if (ServerSettings.DebugAiSay) myPlayer.say('Bring basket home $done');
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: done: $done goto home: held: ${heldObject.name} d: $distanceToHome');
				return done;
			}

			// if basket is empty drop it near ClayDeposit
			if(clayDeposit == null) return false;
			
			if(distanceToClayDeposit <= 1){
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: drop Basket near deposit held: ${heldObject.name} d: $distanceToClayDeposit');
				return dropHeldObject(0);
			}

			var done = myPlayer.gotoObj(clayDeposit);

			if (ServerSettings.DebugAiSay) myPlayer.say('Drop basket near clay deposit $done');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: done: $done goto ClayDeposit: held: ${heldObject.name} d: $distanceToClayDeposit');
			return done;
		}

		var basket = null;
		
		if(distanceToHome <= 100){ // 100
			// if close to home search if there is a basket with clay to empty
			basket = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 292, 10, null, myPlayer, [126]); // Basket 292, Clay 126

			if(basket != null){
				if(heldObject.parentId != 0) return dropHeldObject(1, true); // allow to use piles for clay
				this.dropIsAUse = false; // TODO empty basket ???
				this.dropTarget = basket;

				jumpToAi = this;

				if (ServerSettings.DebugAiSay) myPlayer.say('empty basket');
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: empty basket: held: ${heldObject.name} d: $distanceToHome');

				return true;
			}
		}

		// search if there is a dropped clay basket to bring home
		// Basket 292, Clay 126
		basket = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 292, 20, null, myPlayer, [126]);

		// search if there is a basket to fill close to the clay deposit 
		if(basket == null && clayDeposit != null) basket = AiHelper.GetClosestObjectToPosition(clayDeposit.tx, clayDeposit.ty, 292, 5, null, myPlayer); // Basket 292

		// take care of full basket
		if(basket != null && basket.containedObjects.length > 2){
		
			if(heldObject.parentId != 0) return dropHeldObject(1);
			
			if (ServerSettings.DebugAiSay) myPlayer.say('pickup basket to bring home');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: pickup basket to bring home: held: ${heldObject.name} d: $distanceToHome');

			// pickup basket to bring home
			return useHeldObjOnTarget(basket);				
		}
		
		// holding Clay 126
		if(heldObject.parentId == 126){ 
			if(distanceToHome <= 100) return dropHeldObject(10, true); // allow to use piles for clay

			if(basket == null){
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: no basket to drop clay: held: ${heldObject.name} d: $distanceToHome');
				// have a free hand to not be slowed down by clay while getting a basket
				return dropHeldObject(10);			
			}

			if (ServerSettings.DebugAiSay) myPlayer.say('drop clay in basket');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: drop clay in basket: held: ${heldObject.name} d: $distanceToHome');

			return useHeldObjOnTarget(basket); // fill basket		
		}

		// check if there is loose clay to bring home
		if(distanceToHome > 225){
			var clay = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 126, 5, null, myPlayer); // Clay 126
			if(clay != null){
				dropIsAUse = false;
				dropTarget = clay;
				return true;
			}
		}

		if(clayDeposit == null) return false;

		if(basket == null){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: get basket: held: ${heldObject.name} d: $distanceToClayDeposit');
			return GetOrCraftItem(292); // get Basket
		}

		if(heldObject.parentId != 0) return dropHeldObject(10);

		if(distanceToClayDeposit > 1){
			var done = myPlayer.gotoObj(clayDeposit);

			if (ServerSettings.DebugAiSay) myPlayer.say('Goto clay deposit $done');
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: done: $done goto ClayDeposit: held: ${heldObject.name} d: $distanceToClayDeposit');
			return done;
		}

		if (ServerSettings.DebugAiSay) myPlayer.say('get clay from deposit');
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: get clay from deposit: held: ${heldObject.name} d: $distanceToClayDeposit');

		//jumpToAi = this;
		return useHeldObjOnTarget(clayDeposit);
	}

	/*
		if(craftItem(272)) return true; // Cooked Berry Pie
		if(craftItem(803)) return true; // Cooked Mutton Pie
		if(craftItem(273)) return true; // Cooked Carrot Pie
		if(craftItem(274)) return true; // Cooked Rabbit Pie
		if(craftItem(275)) return true; // Cooked Berry Carrot Pie
		if(craftItem(276)) return true; // Cooked Berry Rabbit Pie
		if(craftItem(277)) return true; // Cooked Rabbit Carrot Pie
		if(craftItem(278)) return true; // Cooked Berry Carrot Rabbit Pie
	*/

	private static var pies = [272, 803, 273, 274, 275, 276, 277, 278]; 
	private static var rawPies = [265, 802, 268, 270, 266, 271, 269, 267];

	private function doBaking(maxPeople:Int = 2) : Bool {
		var heldObject = myPlayer.heldObject;

		// Bowl of Dough 252 + Clay Plate 236 // keep last use for making bread
		if(heldObject.parentId == 252 && heldObject.numberOfUses > 1 && shortCraft(252, 236)) return true;

		if(hasOrBecomeProfession('Baker', maxPeople) == false) return false;
		var startTime = Sys.time();
		var home = myPlayer.home;
			
		var nextPie = lastPie > -1 ? lastPie : WorldMap.world.randomInt(pies.length -1);

		// 250 Hot Adobe Oven
		var hotOven = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 250, 20, null, myPlayer);
		var fireOven = null;

		// 265 Raw Berry Pie // 273 Raw Carrot Pie 
		var countRawPies = 0;
		if(hotOven == null){
			// Burning Adobe Oven 249
			fireOven = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 249, 20, null, myPlayer);

			for(id in rawPies){
				countRawPies += AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, id, 25);
			}
			//countRawPies = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 265, 40);
			//countRawPies += AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 273, 40);
		}
		
		if(hotOven != null || countRawPies > 2){
			this.profession['Baker'] = 2;

			if(hotOven == null && countRawPies > 0){
				if(fireOven == null && craftItem(249)) return true; // Burning Adobe Oven
				return false;
			}

			for(i in 0... pies.length){
				var index = (nextPie + i) % pies.length;
				lastPie = index;
				if(shortCraftOnTarget(rawPies[index], hotOven, false)) return true;
			}

			// Raw Bread Loaf 1469
			if(shortCraftOnTarget(1469, hotOven, false)) return true;
			// Raw Mutton 569
			if(shortCraftOnTarget(569, hotOven, false)) return true;
			// Raw Potato 1147
			if(shortCraftOnTarget(1147, hotOven, false)) return true;
		}
		
		if(hotOven != null && fireOven != null){
			// Adobe Oven 237
			var oven = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 237, 20, null, myPlayer);
			// Wood-filled Adobe Oven 247
			if(oven == null) oven = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 247, 20, null, myPlayer);
			if(oven == null){
				this.profession['Baker'] = 0;
				return false;
			}
		}

		var countPlates = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 236, 40); // Clay Plate
		var hasClosePlate = countPlates > 0;

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: doBaking ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		//if(hasClosePlate == false) return craftItem(236); // Clay Plate
		if(hasClosePlate == false) return false;

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: doBaking ${Math.round((Sys.time() - startTime) * 1000)}ms ');
		
		// 560 Knife
		if(this.profession['Baker'] < 3){
			var knife = myPlayer.heldObject.parentId == 560 ? myPlayer.heldObject : AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 560, 20, null, myPlayer);
			if(knife != null){
				// 1466 Bowl of Leavened Dough // 236 Clay Plate
				if(shortCraft(1466, 236, 20, false)) return true;
				// 1470 Baked Bread
				if(shortCraft(560, 1470, 20, false)) return true;

				var countBread = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 1471, 40); // Sliced Bread
				// 560 Knife // 1468 Leavened Dough on Clay Plate
				if(countBread < 3 && shortCraft(560, 1468, 20, false)) return true;			
			}
		}		

		this.profession['Baker'] = 3; // TODO set to 2 once in a while to check for bread stuff???
		
		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: doBaking ${Math.round((Sys.time() - startTime) * 1000)}ms ');	
		
		var countCarrotPies = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 273, 40); // Cooked Carrot Pie 273
		var countBerryPies = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 272, 40); // Cooked Berry Pie 272		
		var extraPies = countPies % 4;
		
		if(extraPies == 0){
			if(countCarrotPies < 2 && craftItem(268)) return true; // Raw Carrot Pie
		}

		if(extraPies == 2){
			if(countBerryPies < 2 && craftItem(265)) return true; // Raw Berry Pie
		}

		for(i in 0...pies.length){
			var index = (nextPie + i) % pies.length;
			lastPie = index;
			if(craftItem(rawPies[index])) return true;
		}

		if (ServerSettings.DebugAi && (Sys.time() - startTime) * 1000 > 100) trace('AI TIME WARNING: doBaking ${Math.round((Sys.time() - startTime) * 1000)}ms ');

		// check if there is something to fire oven
		/*if(hotOven == null){
			for(i in 0... pies.length){
				var index = (nextPie + i) % pies.length;
				lastPie = index;
				if(shortCraft(rawPies[index], pies[index])) return true;
			}
		}*/

		this.profession['Baker'] = 0;
	
		return false;
	}

	private function doWatering(maxPeople:Int = 1) : Bool {
		if(hasOrBecomeProfession('WaterBringer', maxPeople) == false) return false;

		if(shortCraft(382, 396)) return true; // Bowl of Water + Planted Carrots
		if(shortCraft(210, 396)) return true; // Full Water Pouch + Dry Planted Carrots

		if(shortCraft(382, 228)) return true; // Bowl of Water + Dry Planted Wheat
		if(shortCraft(210, 228)) return true; // Full Water Pouch + Dry Planted Wheat

		if(shortCraft(382, 393)) return true; // Bowl of Water + Dry Domestic Gooseberry Bush 393
		if(shortCraft(210, 393)) return true; // Full Water Pouch + Dry Domestic Gooseberry Bush 393

		if(shortCraft(382, 1109)) return true; // Bowl of Water + Dry Planted Corn Seed
		if(shortCraft(210, 1109)) return true; // Full Water Pouch + Dry Planted Corn Seed

		if(shortCraft(382, 2829)) return true; // Bowl of Water + Dry Planted Tomato Seed
		if(shortCraft(210, 2829)) return true; // Full Water Pouch + Dry Planted Tomato Seed

		if(shortCraft(382, 4225)) return true; // Bowl of Water + Dry Planted Cucumber Seeds
		if(shortCraft(210, 4225)) return true; // Full Water Pouch + Dry Planted Cucumber Seeds

		if(shortCraft(382, 2856)) return true; // Bowl of Water + Dry Planted Onion
		if(shortCraft(210, 2856)) return true; // Full Water Pouch + Dry Planted Onion

		if(shortCraft(382, 2851)) return true; // Bowl of Water + Dry Planted Onions
		if(shortCraft(210, 2851)) return true; // Full Water Pouch + Dry Planted Onions

		//if(craftItem(1110)) return true; // Wet Planted Corn Seed
		//if(craftItem(399)) return true; // Wet Planted Carrots
		//if(craftItem(229)) return true; // Wet Planted Wheat
		//if(craftItem(2857)) return true; // Wet Planted Onion
		//if(craftItem(2852)) return true; // Wet Planted Onions
		//if(craftItem(2831)) return true; // Wet Planted Tomato Seed
		//if(craftItem(4226)) return true; // Wet Planted Cucumber Seeds

		this.profession['WaterBringer'] = 0;

		return false;
	}

	private function GetGraveyard() {
		var home = myPlayer.home;
		// Marked Grave 1012
		var grave = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 1012, 25, null, myPlayer, 8);
		// Buried Grave 1011
		if(grave == null) grave = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 1011, 25, null, myPlayer, 8);

		return grave;
	}

	private function GetForge() {
		var home = myPlayer.home;

		// forge 303
		var forge = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 303, 20, null, myPlayer);

		// Forge with Charcoal 305
		if(forge == null) forge = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 305, 20, null, myPlayer);

		// Firing Forge 304
		if(forge == null) forge = AiHelper.GetClosestObjectToPosition(home.tx, home.ty, 304, 20, null, myPlayer);

		return forge;
	}

	private function doSmithing(maxPeople:Int = 1) : Bool {
		var home = myPlayer.home;

		if(hasOrBecomeProfession('Smith', maxPeople) == false) return false;

		var forge = GetForge();

		if(forge == null) return false;

		// Cold Iron Bloom on Flat Rock 312
		if(shortCraft(239, 312, 20, false)) return true;

		// Wrought Iron on Flat Rock 313
		if(shortCraft(0, 313, 20, false)) return true;

		// Steel Ingot on Flat Rock 335
		if(shortCraft(0, 335, 20, false)) return true;
		
		if(this.profession['Smith'] < 4){
			// TODO fix make space for them otherwise it might try again and again
			// Flat Rock 291
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 291, 10); 
			if(count < 2 && GetCraftAndDropItemsCloseToObj(forge,291,2,5)) return true;

			// Stone 33
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 33, 10); 
			if(count < 1 && GetCraftAndDropItemsCloseToObj(forge,33,1,5)) return true;
		}

		// TODO use forge as count target, but first fix that stuff is dropped close to forge

		// Huge Charcoal Pile 4102
		// Big Charcoal Pile 300
		if(this.profession['Smith'] < 1.5){
			// Basket of Charcoal 298
			if(shortCraftOnGround(298)) return true;
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 4102, 20); 
			count += AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 300, 20);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: Charcoal Pile count: ${count}');
			// Basket of Charcoal 298
			if(count < 2 && craftItem(298)) return true;
			this.profession['Smith'] = 1.5;	
		}

		// Steel Ingot 326
		var countSteel = AiHelper.CountCloseObjects(myPlayer, forge.tx, forge.ty, 326, 20); 
		// Unforged Sealed Steel Crucible 319
		var countCrucible = AiHelper.CountCloseObjects(myPlayer, forge.tx, forge.ty, 319, 20); 
		// Forged Steel Crucible 322
		var countForgedCrucible = AiHelper.CountCloseObjects(myPlayer, forge.tx, forge.ty, 322, 20); 
		
		if(countSteel < 1 || countForgedCrucible > 0){
			// Cool Steel Crucible in Wooden Tongs 324
			if(shortCraftOnGround(324)) return true;

			// Unforged Sealed Steel Crucible 319
			if(this.profession['Smith'] < 3.5 && countForgedCrucible < 1){
				// Big Charcoal Pile 300
				var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 300, 20); 
				// Basket of Charcoal 298
				if(count < 1 && craftItem(298)) return true;

				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: Unforged Sealed Steel Crucible count done: ${count}');
				if(countCrucible < 3 && GetCraftAndDropItemsCloseToObj(forge, 319, 3, 10)) return true;
				this.profession['Smith'] = 3.5;	
			}

			// Hot Steel Crucible in Wooden Tongs 323
			if(countCrucible > 0 && ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: Hot Steel Crucible count left: ${countCrucible}');
			if(countCrucible > 0 && craftItem(323)) return true;

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: Steel Ingot count: ${countSteel}');
			// Steel Ingot 326
			if(craftItem(326)) return true;				
			trace('doSmithing2: Steel Ingot count: ${countSteel}');
			this.profession['Smith'] = 3; // craft Crucible	
		}	

		if (countSteel > 1) this.profession['Smith'] = 4;

		// Wrought Iron 314
		if(this.profession['Smith'] < 3){
			var count = AiHelper.CountCloseObjects(myPlayer, forge.tx, forge.ty, 314, 20); 
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: Wrought Iron count: ${count}');
			if(count < 5){
				// Iron Ore 290
				if(this.profession['Smith'] < 2){
					var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 290, 20); 
					if(count < 5 && craftItem(290)) return true;
					this.profession['Smith'] = 2;	
				}
				// Wrought Iron 314
				if(craftItem(314)) return true;
			} 
			this.profession['Smith'] = 3;	
		}

		if (countSteel < 1){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} doSmithing: no steel');
			this.profession['Smith'] = 0;
			return false;
		}
		
		if(this.profession['Smith'] < 5){
			// Smithing Hammer
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 441, 30); 
			if(count < 1 && craftItem(441)) return true;
			this.profession['Smith'] = 5;
		}
		
		if(this.profession['Smith'] < 6){
			// Steel Mining Pick 684
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 684, 50); 
			if(count < 1 && craftItem(684)) return true;
			this.profession['Smith'] = 6;
		}

		// Shovel 502
		if(this.profession['Smith'] < 7){
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 502, 30); 
			if(count < 1 && craftItem(502)) return true;
			this.profession['Smith'] = 7;
		}

		if(this.profession['Smith'] < 8){
			// Steel Axe 334
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 334, 30); 
			if(count < 1 && craftItem(334)) return true;
			this.profession['Smith'] = 8;
		}

		// Steel Chisel 455
		var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 455, 30); 
		if(count < 1 && craftItem(455)) return true;

		// Steel File Blank
		var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 457, 30); 
		if(count < 1 && craftItem(457)) return true;

		// Knife 560
		var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 560, 30); 
		if(count < 1 && craftItem(560)) return true;

		this.profession['Smith'] = 0;

		return false;
	}

	private function doAdvancedFarming(maxPeople:Int = 2) : Bool {
		if(hasOrBecomeProfession('AdvancedFarmer', maxPeople) == false) return false;

		// 1109 Dry Planted Corn Seed
		// 396 Dry Planted Carrots
		// 2851 Dry Planted Onions
		// 2829 Dry Planted Tomato Seed
		// 4225 Dry Planted Cucumber Seeds
		var dryPlanted = [1109, 396, 2829, 396, 1109, 396, 2851, 396, 1109, 396, 4225];
		var home = myPlayer.home;
		var countBowls = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 235, 15); //  Clay Bowl 235
		if(countBowls < 1) return doPottery(3);

		// TODO more wet planted stuff
		/*if(craftItem(229)) return true; // Wet Planted Wheat
		if(craftItem(1162)) return true; // Wet Planted Beans
		if(craftItem(2857)) return true; // Wet Planted Onion
		if(craftItem(2852)) return true; // Wet Planted Onions
		if(craftItem(4263)) return true; // Wet Planted Garlic
		if(craftItem(399)) return true; // Wet Planted Carrots
		if(craftItem(1142)) return true; // Wet Planted Potatoes
		if(craftItem(1110)) return true; // Wet Planted Corn Seed
		// Wet Planted Gooseberry Seed 217
		*/

		// 228 Dry Planted Wheat
		// 396 Dry Planted Carrots
		// 2851 Dry Planted Onions
		// 2829 Dry Planted Tomato Seed
		// 4225 Dry Planted Cucumber Seeds
		// TODO other dry planted

		// stuff can be in more then once to increase chance
		
		var advancedPlants = [228, 396, 1110, 217, 1162, 228, 396, 1110, 2851, 228, 4225, 396, 2829, 1110, 2852, 228, 396, 4263, 228, 396, 396, 228, 1142, 228, 1110, 228];
		var rand = WorldMap.world.randomInt(advancedPlants.length - 1);
		
		toPlant = toPlant > 0 ? toPlant : rand;
		var nextPlant = toPlant + Math.round(myPlayer.age);

		for(i in 0...advancedPlants.length){
			var index = (nextPlant + i) % advancedPlants.length;
			var toPlant = advancedPlants[index];

			// Dry Bean Plants 1172
			if(toPlant == 1172){
				var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 1172, 30);
				if(count > 3){
					nextPlant +=1;
					continue;
				}
			}

			if(craftItem(toPlant)) return true;
		}

		/*var plantFrom = rand % 3 == 0 ? dryPlanted : advancedPlants;
		for(i in 0...plantFrom.length){
			var index = (rand + i) % plantFrom.length;
			if(craftItem(plantFrom[index])) return true;
		}*/

		if(craftItem(229)) return true; // Wet Planted Wheat	
		if(craftItem(399)) return true; // Wet Planted Carrots	
		if(craftItem(2831)) return true; // Wet Planted Tomato Seed
		if(craftItem(2857)) return true; // Wet Planted Onion
		if(craftItem(2852)) return true; // Wet Planted Onions
		
		/*
		if(craftItem(236)) return true; // Clay Plate
		// grow food that dont needs plates for processing

		var rand = WorldMap.world.randomInt(dryPlanted.length -1);

		for(i in 0...dryPlanted.length){
			var index = (rand + i) % dryPlanted.length;
			if(craftItem(dryPlanted[index])) return true;
		}*/
		
		this.profession['AdvancedFarmer'] = 0;
		return false;
	}

	private function makeStuff() : Bool {	
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} makeStuff!');

		if(makeSharpieFood()) return true;

		if(doBaking(2)) return true;
		if(doBasicFarming(2)) return true;
		Macro.exception(if (isSheepHerding(2)) return true);
		
		if(makeFireFood(2)) return true;

		if(craftItem(59)) return true; // Rope 
		//if(craftItem(58)) return true; // Thread
			
		if(craftItem(808)) return true; // Wild Onion
		if(craftItem(4252)) return true; // Wild Garlic
		
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} nothing to make!');

		return false;
	}

	private function makeSharpieFood(maxDistance:Int = 40) : Bool {
		var heldObjId = myPlayer.heldObject.parentId;
		// 40 Wild Carrot // 807 Burdock Root
		//if(maxDistance < 15 && (heldObjId == 40 || heldObjId == 807)) dropHeldObject(0);

		var isHoldingSharpStone = myPlayer.heldObject.parentId == 34; // 34 Sharp Stone

		if(shortCraft(0, 1112, maxDistance)) return true; // 0 + Corn Plant --> Ear of Corn
		if(shortCraft(34, 1113, maxDistance)) return true; // Sharp Stone + Ear of Corn --> Shucked Ear of Corn
		//if(craftItem(1114)) return true; // Shucked Ear of Corn

		var obj = AiHelper.GetClosestObjectById(myPlayer, 36, null, maxDistance); // Seeding Wild Carrot
		if(obj != null && isHoldingSharpStone == false) return GetOrCraftItem(34); 
		if(obj != null && craftItem(39)) return true; // Dug Wild Carrot // 40 Wild Carrot		
		
		var obj = AiHelper.GetClosestObjectById(myPlayer, 804, null, maxDistance); // Burdock
		if(obj != null && isHoldingSharpStone == false) return GetOrCraftItem(34); 
		if(obj != null && craftItem(806)) return true; // Dug Burdock
		
		return false;
	}

	private function fillBerryBowlIfNeeded() : Bool {
		var heldObj = myPlayer.heldObject;

		// 253 Bowl of Gooseberries
		if(heldObj.parentId == 253 && heldObj.numberOfUses >= heldObj.objectData.numUses) return false;

		// 30 Wild Gooseberry Bush
		var closeBush = AiHelper.GetClosestObjectById(myPlayer, 30);
		// 391 Domestic Gooseberry Bush
		if(closeBush == null) closeBush = AiHelper.GetClosestObjectById(myPlayer, 391);
		if(closeBush == null) return false;

		// Fill up the Bowl // 235 Clay Bowl // 253 Bowl of Gooseberries
		if(heldObj.parentId == 235 || heldObj.parentId == 253){
			if(ServerSettings.DebugAiSay) myPlayer.say('Fill Bowl on Bush');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Fill Bowl on Bush!');

			return useHeldObjOnTarget(closeBush);
		}

		// do nothing if there is a full Bowl of Gooseberries
		var closeBerryBowl = AiHelper.GetClosestObjectById(myPlayer, 253); // Bowl of Gooseberries
		if(closeBerryBowl != null && closeBerryBowl.numberOfUses >= closeBerryBowl.objectData.numUses) return false;

		var target = closeBerryBowl != null ? closeBerryBowl : myPlayer.home;
		var bestPlayer = getBestAiForObjByProfession('BowlFiller', target);
		if(bestPlayer == null || bestPlayer.myPlayer.id != myPlayer.id) return false;

		if(closeBerryBowl != null){
			this.dropTarget = closeBerryBowl; // pick it up to fill
			this.dropIsAUse = false;

			if(ServerSettings.DebugAiSay) myPlayer.say('Pickup Berry Bowl to Fill');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Pickup Berry Bowl to Fill!');

			return true; 
		}

		return GetItem(235); // Clay Bowl
	}

	private function makePopcornIfNeeded() : Bool {
		// do nothing if there is Popcorn
		var closePopcorn = AiHelper.GetClosestObjectToHome(myPlayer, 1121); // Popcorn
		if(closePopcorn != null) return false;

		var bestPlayer = getBestAiForObjByProfession('BowlFiller', myPlayer.home);
		if(bestPlayer == null || bestPlayer.myPlayer.id != myPlayer.id) return false;

		return craftItem(1121); // Popcorn
	}

	private function makeFireFood(maxPeople:Int = 1) : Bool {
		if(hasOrBecomeProfession('FireFoodMaker', maxPeople) == false) return false;

		var firePlace = myPlayer.firePlace;

		if(shortCraftOnGround(186)) return true; // Cooked Rabbit --> unskew the Cooked Rabbits

		// Hot Coals 85 // TODO consider time to change
		var hotCoals = AiHelper.GetClosestObjectToHome(myPlayer, 85, 30);
		
		// Cooked Mutton 570
		var countDoneMutton = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 570, 20);
		if(countDoneMutton < 2 && shortCraftOnTarget(569, hotCoals, false)) return true; // Raw Mutton 569 --> Cooked Mutton 570

		// Cooked Rabbit 197
		var countDoneRabbit = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 197, 20);
		if(countDoneRabbit < 2 && shortCraftOnTarget(185, hotCoals)) return true; // Skewered Rabbit 185 --> Cooked Rabbit 186

		// Bowl of Raw Pork 1354 --? Bowl of Carnitas
		if(shortCraftOnTarget(1354, hotCoals)) return true;

		// Kindling 72
		if(hotCoals == firePlace && shortCraftOnTarget(72,hotCoals)) return true; 

		// Fire 82
		if(firePlace == null) return craftItem(82);

		// 1284 Cool Flat Rock --> Ashes
		if(shortCraft(0, 1284, 20)) return true;

		var countPlates = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 236, 20);
		if(countPlates > 0 && craftItem(1285)) return true; // Omelette

		// Skinned Rabbit 181
		var countRawFireFood = AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 181, 25);
		// Skewered Rabbit 185
		countRawFireFood += AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 185, 25);
		// Raw Mutton 569
		countRawFireFood += AiHelper.CountCloseObjects(myPlayer, myPlayer.home.tx, myPlayer.home.ty, 569, 25);

		if(countRawFireFood > 1 && hotCoals == null && (countDoneRabbit < 1 || countDoneMutton < 1)){
			// look for second fire 82
			var fire = AiHelper.GetClosestObjectToHome(myPlayer, 82, 30, firePlace);
			if(fire == null) return craftItem(82);
		}
	
		// Raw Mutton 569
		if(craftItem(569)) return true;
		// Skinned Rabbit 181
		if(craftItem(181)) return true;

		this.profession['FireFoodMaker'] = 0;
		return false;
	}

	private function makeFireWood() : Bool {
		// TODO check at home
		var closeWood = AiHelper.GetClosestObjectById(myPlayer, 344); // Firewood
		if(closeWood == null) AiHelper.GetClosestObjectById(myPlayer, 1316); // Stack of Firewood
		var doCraft = closeWood == null || (closeWood.objectData.numUses > 1 && closeWood.numberOfUses < closeWood.objectData.numUses);
		if(doCraft && craftItem(344)) return true; // Firewood // TODO could unstack the stack again
		
		var closeKindling = AiHelper.GetClosestObjectById(myPlayer, 72); // Kindling
		if(closeKindling == null) AiHelper.GetClosestObjectById(myPlayer, 1599); // Kindling Pile
		var doCraft = closeKindling == null || (closeKindling.objectData.numUses > 1 && closeKindling.numberOfUses < closeKindling.objectData.numUses);
		if(doCraft && craftItem(72)) return true; // Kindling // TODO could unstack the stack again
		
		return false;
	}

	private function cleanUpProfessions(){
		if(lastProfession == null) return;
	
		for(key in profession.keys()){
			// keep old profession
			if(key == lastProfession) continue;
			if(key == 'FoodServer') continue;
			if(key == 'BowlFiller') continue; 
			if(key == 'firekeeper') continue; 
			if(key == 'gravekeeper') continue; 
			if(lastProfession == 'FoodServer') continue;
			if(lastProfession == 'BowlFiller') continue;
			if(lastProfession == 'firekeeper') continue;
			if(lastProfession == 'gravekeeper') continue; 
	
			profession[key] = 0;
		 
			//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} profession: ${key} --> ${lastProfession}');
		}
	}
	
	private function hasOrBecomeProfession(profession:String, max:Int = 1) : Bool {
		var hasProfession = this.profession[profession] > 0;
	
		if(hasProfession){
			this.lastProfession = profession; 
			return true;
		}
	
		var count = countProfession(profession);
		//trace('hasOrBecomeProfession: $profession count: $count');
		if (count >= max + wasIdle) return false;
		this.profession[profession] = 1;
		this.lastProfession = profession;
		return true;
	}

 	// 2886 Wooden Shoe 
 	// 2181 Straw Hat with Feather
	private function craftHighPriorityClothing() : Bool {
		// TODO consider heat / cold
		// TODO more advanced clothing
		// TODO try to look like the one you follow
		var color = myPlayer.getColor();
		var isWhiteOrGinger = (color == Ginger || color == White);

		// Bottom clothing
		// 200 Rabbit Fur Loincloth / bottom
		if(isWhiteOrGinger && craftClothIfNeeded(200)) return true;
		// 128 Reed Skirt / bottom
		if(craftClothIfNeeded(128)) return true; 

		return false;
}

private function craftMediumPriorityClothing() : Bool {
		if(hasOrBecomeProfession('ClothMaker', 1) == false) return false;

		//trace('craftMediumPriorityClothing');

		var objData = ObjectData.getObjectData(152); // Bow and Arrow
		var isOldEnoughForBow = myPlayer.age >= objData.minPickupAge;
		var color = myPlayer.getColor();
		var isWhiteOrGinger = (color == Ginger || color == White);

		if(isOldEnoughForBow){ 
			// Hunting gear 874 Empty Arrow Quiver
			if(craftClothIfNeeded(874)) return true; 
			if(fillUpQuiver()) return true;
		}

		// Shoes
		// 844 Fruit Boot ==> Black
		if(color == Black && craftClothIfNeeded(844)) return true;
		// 2887 Sandal ==> Black
		if(color == Black && craftClothIfNeeded(2887)) return true;
		// 766 Snake Skin Boot ==> Black
		if(color == Black && craftClothIfNeeded(766)) return true;
		// 586 Wool Booty
		if(isWhiteOrGinger && craftClothIfNeeded(586)) return true;
		// 203 Rabbit Fur Shoe
		if(isWhiteOrGinger && craftClothIfNeeded(203)) return true;

		// Chest clothing
		// 585 Wool Sweater ==> White / Chest
		if(color == White && craftClothIfNeeded(585)) return true; 
		// 564 Mouflon Hide ==> White / Chest // only hunt if old enough for bow
		if(color == White && isOldEnoughForBow && craftClothIfNeeded(564)) return true;
		// 712 Sealskin Coat ==> Ginger
		if(color == Ginger && craftClothIfNeeded(712)) return true;
		// 711 Seal Skin ==> Ginger
		if(color == Ginger && craftClothIfNeeded(711)) return true;	
		// 202 Rabbit Fur Coat / Chest
		if(isWhiteOrGinger && craftClothIfNeeded(202)) return true;
		// 201 Rabbit Fur Shawl / Chest
		if(isWhiteOrGinger && craftClothIfNeeded(201)) return true;	

		// head clothing
		// 584 Wool Hat  ==> White / Head
		if(color == White && craftClothIfNeeded(584)) return true; 

		this.profession['ClothMaker'] = 0;

		return false;
}
	
private function craftLowPriorityClothing() : Bool {
		if(hasOrBecomeProfession('ClothMaker', 1) == false) return false;

		var objData = ObjectData.getObjectData(152); // Bow and Arrow
		var color = myPlayer.getColor();
		var isWhiteOrGinger = (color == Ginger || color == White);
	
		// Hat cloting
		// 426 Wolf Hat ==> White
		if(color == White && craftClothIfNeeded(426)) return true;
		// 2180 Rabbit Fur Hat with Feather // TODO check minPickupAge directly in crafting
		if(isWhiteOrGinger && myPlayer.age >= objData.minPickupAge && craftClothIfNeeded(2180)) return true;
		// 199 Rabbit Fur Hat
		if(isWhiteOrGinger && craftClothIfNeeded(199)) return true;

		// Back clothing
		// 198 Backpack
		// TODO fix bug picking up backpack (AI drops item in it and then instead of picking up puts item out of it)
		//if(myPlayer.age > 25 && craftClothIfNeeded(198)) return true;

		this.profession['ClothMaker'] = 0;

		return false;
	}

	private function fillUpQuiver() : Bool {
		var heldId = myPlayer.heldObject.parentId;
		// Empty Arrow Quiver
		var quiver = myPlayer.getClothingById(874); 
		// Arrow Quiver
		if(quiver == null) quiver = myPlayer.getClothingById(3948);
		
		if(quiver != null){
			// Bow or Bow and Arrow
			if(heldId == 151 || heldId == 152){
				myPlayer.self(0,0,5);
				//if(ServerSettings.DebugAi) 
				if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} put Bow on Quiver!');
				return true;
			}

			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} find Bow for Quiver!');

			if(GetItem(152)) return true;  // Bow and Arrow
			//var obj = AiHelper.GetClosestObjectById(myPlayer, 152); // Bow and Arrow
			
			if(GetOrCraftItem(151)) return true;  // Get Yew Bow
		}

		// Empty Arrow Quiver
		var quiver = myPlayer.getClothingById(874); 
		// Arrow Quiver
		if(quiver == null) quiver = myPlayer.getClothingById(3948);
		// Arrow Quiver with Bow
		if(quiver == null) quiver = myPlayer.getClothingById(4151);

		if(quiver == null) return false;
		if(quiver.canAddToQuiver() == false) return false;

		// Arrow
		if(heldId == 148){
			myPlayer.self(0,0,5);
			if(ServerSettings.DebugAi) 
				trace('AAI: ${myPlayer.name + myPlayer.id} put Arrow in Quiver!');
			return true;
		}
		if(ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} get Arrow for Quiver!');

		return GetOrCraftItem(148); // Arrow
	}

	private function craftClothIfNeeded(clothId:Int) : Bool {
		var objData = ObjectData.getObjectData(clothId);
		var slot = objData.getClothingSlot();
		if(slot < 0) return false;
		var createCloth = myPlayer.clothingObjects[slot].id == 0;

		if(myPlayer.clothingObjects[slot].name.contains('RAG ')) createCloth = true;
		if(createCloth == false) return false;
		if(craftItem(clothId)){ 
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft clothing ${objData.name}');
			if(ServerSettings.DebugAiSay) myPlayer.say('Craft ${objData.name} to wear...');
			return true;
		}
		//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} could not craft clothing ${objData.name}');
		//if(ServerSettings.DebugAiSay) myPlayer.say('Could not craft ${objData.name} to wear...');
		return false;
	}

	public function say(player:PlayerInterface, curse:Bool, text:String) {
		if (myPlayer.id == player.id) return;
		if (player.isAi()) return;

		var quadDist = AiHelper.CalculateDistanceToPlayer(this.myPlayer, player);
		if(quadDist > Math.pow(ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi,2)) return;

		// if(ServerSettings.DebugAi) trace('AI ${text}');

		/*if (text.startsWith("TRANS")) {
			if (ServerSettings.DebugAi) trace('AI look for transitions: ${text}');

			var objectIdToSearch = 273; // 273 = Cooked Carrot Pie // 250 = Hot Adobe Oven

			AiHelper.SearchTransitions(myPlayer, objectIdToSearch);
		}*/

		if (text.contains("HOLA") || text.contains("HELLO") || text == "HI") {
			// HELLO WORLD

			// if(ServerSettings.DebugAi) trace('im a nice bot!');

			myPlayer.say('HOLA ${player.name}');
		}
        if (text.contains("ARE YOU AI") || text.contains("ARE YOU AN AI") || text == "AI?" || text == "AI") {
			// HELLO WORLD

			// if(ServerSettings.DebugAi) trace('im a nice bot!');
			var rand = WorldMap.world.randomInt(8);

			if(rand == 0){
				myPlayer.say('Im not a stupid AI!');
			} else if(rand == 1){
				myPlayer.say('Im an AI!');
			} else if(rand == 2){
				myPlayer.say('No');
			} else if(rand == 3){
				myPlayer.say('Sure');	
			} else if(rand == 4){
				myPlayer.say('yes i am');
			} else if(rand == 5){
				myPlayer.say('Yes, And you?');
			} else if(rand == 6){
				myPlayer.say('Why should I?');
			}
		}
		if (text == "JUMP") {
			myPlayer.say("JUMP");
			myPlayer.jump();
		}
		if (text.startsWith("MOVE")) {
			myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.say("YES CAPTAIN");
		}
		if (text.contains("FOLLOW ME") || text.startsWith("FOLLOW") || text.startsWith("COME")) {
			autoStopFollow = false; // otherwise if old enough ai would stop follow
			timeStartedToFolow = TimeHelper.tick; 
			playerToFollow = player;
			myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.say("IM COMMING");
		}
		else if (text.contains("STOP FOLLOW")) {
			playerToFollow = null;
			autoStopFollow = true;
			myPlayer.say("STOPED");
		}
		else if (text.startsWith("STOP") || text.startsWith("WAIT")) {
			playerToFollow = null;
			autoStopFollow = true;			
			//myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.Goto(myPlayer.x, myPlayer.y);
			dropHeldObject(0);
			waitingTime = 10;
			myPlayer.say("STOPING");
			//myPlayer.age -= 1;
		}
		else if (text.startsWith("DROP")) {			
			//myPlayer.Goto(player.tx + 1 - myPlayer.gx, player.ty - myPlayer.gy);
			myPlayer.Goto(myPlayer.x, myPlayer.y);
			dropHeldObject(0);
			waitingTime = 1;
			myPlayer.say("DROPING");
		}
		if (text.contains("GO HOME")) {			

			var quadDistance = myPlayer.CalculateQuadDistanceToObject(myPlayer.home);
			if (quadDistance < 3) {
				myPlayer.say("I AM HOME!");
				this.time += 5;
				return;
			}

			if(isMovingToHome()) myPlayer.say("GOING HOME!");
			else myPlayer.say("I CANNOT GO HOME!");
			this.time += 6;
		}
		else if (text.startsWith("HOME")) {		
			var newHome = AiHelper.SearchNewHome(myPlayer);

			if(newHome != null){
				if(myPlayer.home.tx != newHome.tx || myPlayer.home.ty != newHome.ty ) myPlayer.say('Have a new home! ${newHome.name}');
				else myPlayer.say('No mew home! ${newHome.name}');
				
				myPlayer.home = newHome;
			} 

			myPlayer.firePlace = AiHelper.GetCloseFire(myPlayer);
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
				itemToCraftId = id;
				// craftItem(id); // TODO use mutex if Ai does not use Globalplayermutex
				var obj = ObjectData.getObjectData(id);
				this.itemToCraftName = obj.name;
				myPlayer.say("Making " + obj.name);
			}
		}
	}

	public function searchFoodAndEat() {
		foodTarget = AiHelper.SearchBestFood(myPlayer);

		/*var objData = foodTarget.foodFromTarget == null ? foodTarget : foodTarget.foodFromTarget;
		if (foodTarget != null && myPlayer.canEat(objData) == false){			
			myPlayer.say('WARNING cant eat food!');
			if (ServerSettings.DebugAi && foodTarget != null) trace('AAI: ${myPlayer.name + myPlayer.id} WARNING cant eat food! new Foodtarget! ${foodTarget.name}');
			foodTarget = null;
			return;
		}*/

		if(ServerSettings.DebugAiSay){
			if(foodTarget == null) myPlayer.say('No food found...');
			else myPlayer.say('new food ${foodTarget.name}');
		}
		var heldObjName = myPlayer.heldObject.name;
		if (ServerSettings.DebugAi && foodTarget != null) trace('AAI: ${myPlayer.name + myPlayer.id} new Foodtarget! ${foodTarget.name} held: ${heldObjName}');
		if (ServerSettings.DebugAi && foodTarget == null) trace('AAI: ${myPlayer.name + myPlayer.id} no new Foodtarget!!! held: ${heldObjName}');
	}

	public function storeInQuiver() {
		var heldObject = myPlayer.heldObject;
		var heldObjId = heldObject.parentId;

		// Yew Bow
		if(heldObjId == 151){
			// Empty Arrow Quiver
			var quiver = myPlayer.getClothingById(874); 
			// Arrow Quiver
			if(quiver == null) quiver = myPlayer.getClothingById(3948);

			if(quiver != null){
				myPlayer.self(0,0,5);
				//if(ServerSettings.DebugAi) 
				trace('AAI: ${myPlayer.name + myPlayer.id} DROP: put bow on quiver!');
				return true;
			}
		}

		// Bow and Arrow
		if(heldObjId == 152){
			// Empty Arrow Quiver
			var quiver = myPlayer.getClothingById(874); 
			// Arrow Quiver
			if(quiver == null) quiver = myPlayer.getClothingById(3948);

			if(quiver != null &&  quiver.canAddToQuiver()){
				myPlayer.self(0,0,5);
				//if(ServerSettings.DebugAi) 
				trace('AAI: ${myPlayer.name + myPlayer.id} DROP: put Bow with Arrow on quiver!');
				return true;
			}
		}

		// Arrow
		if(heldObjId == 148){
			// Empty Arrow Quiver
			var quiver = myPlayer.getClothingById(874); 
			// Empty Arrow Quiver with Bow
			if(quiver == null) quiver = myPlayer.getClothingById(4149);
			// Arrow Quiver
			if(quiver == null) quiver = myPlayer.getClothingById(3948);
			// Arrow Quiver with Bow
			if(quiver == null) quiver = myPlayer.getClothingById(4151);

			if(quiver != null && quiver.canAddToQuiver()){
				myPlayer.self(0,0,5);
				//if(ServerSettings.DebugAi) 
				trace('AAI: ${myPlayer.name + myPlayer.id} DROP: put Arrow in quiver!');
				return true;
			}
		}

		return false;
	}

	public function dropNearOven() {
		var heldObject = myPlayer.heldObject;
		var heldObjId = heldObject.parentId;

	}

	public function dropNearKiln() {
		var heldObject = myPlayer.heldObject;
		var heldObjId = heldObject.parentId;

	}

	public function dropNearForge() {
		var heldObject = myPlayer.heldObject;
		var heldId = heldObject.parentId;

	}

	public function dropGraveyard() {
		var heldObject = myPlayer.heldObject;
		var heldObjId = heldObject.parentId;
	}

	// Kindling 72 // Firewood 344 
	// Dead Rabbit 180 // Skinned Rabbit 181 // Skewered Rabbit 185
	// Raw Potato 1147 // Baked Potato
	var dropNearFireItemIds = [72, 344, 180, 181, 185, 1147, 1148];

	// Clay Bowl 235 // Stack of Clay Bowls 1603 // Clay Plate 236 // Stack of Clay Plates 1602 
	// Bowl of Gooseberries 253 // Knife 560 // Bowl of Dough 252 
	// Baked Bread 1470 // Sliced Bread 1471
	// TODO drop somewhere save Shovel 502 // Shovel of Dung 900
	var dropNearOvenItemIds = [235, 1603, 236, 1602, 560, 252, 1470, 1471, 253, 502, 900];

	private function considerDropHeldObject(gotoTarget:ObjectHelper) {
		var heldObjId = myPlayer.heldObject.parentId;
		var dropTarget =  myPlayer.home;

		if (heldObjId == 2144) return dropHeldObject(); // 2144 Banana Peel
		if (heldObjId == 34) return dropHeldObject(); // 34 Sharp Stone

		// TODO other items for Kiln, smith, plates for oven
		// drop at once, since its normally dropped at fire. For exmple kindling, wood...
		if(dropNearFireItemIds.contains(heldObjId)){
			//if(myPlayer.firePlace != null) dropTarget = myPlayer.firePlace;
			return dropHeldObject();
		}

		// drop at once, since its normally dropped at home. For exmple pies, platees...
		if(dropNearOvenItemIds.contains(heldObjId) || pies.contains(heldObjId) || rawPies.contains(heldObjId)){
			//dropTarget = myPlayer.home; // drop near home which is normaly the oven	
			return dropHeldObject();
		}

		// TODO use actual drop target for heldObject like oven, kiln, forge instead of home 
		var quadDistanceToHome = AiHelper.CalculateQuadDistanceToObject(myPlayer, dropTarget);
		var quadDistanceToTarget = AiHelper.CalculateQuadDistanceToObject(myPlayer, gotoTarget);

		// check if target is closer then current position or in 5 tiles reach --> then take item to target
		if(quadDistanceToTarget < quadDistanceToHome + 25) return false;
		
		return dropHeldObject();
	}

	// TODO consider to not drop stuff close to home if super far away or starving
	// allowAllPiles --> some stuff like clay baskets and so on is normally not piled. Set true if it should be allowed to be piled. 
	// target is the target where heldObj shoudld be dropped close to
	public function dropHeldObject(maxDistanceToHome:Float = 40, allowAllPiles:Bool = false, target:ObjectHelper = null, ?infos:haxe.PosInfos) : Bool {
		if(target == null) target = myPlayer.home;

		var home = myPlayer.home;
		var dropCloseToPlayer = true;
		var heldObjId = myPlayer.heldObject.parentId;
		var searchDistance = 40;
		var mindistance = 0; // to oven
		var quadIsCloseEnoughDistanceToTarget = 400; // old 25 // does not go to home if close enough // if too low and not enough space around target its stuck
		var dropOnStart:Bool = mindistance < 1;
		var newDropTarget = null;
		var heldObject = myPlayer.heldObject;
		var heldId = heldObject.parentId;

		if (heldObjId == 0) return false;
		if (myPlayer.heldObject.isWound()) return false;
		if (myPlayer.heldObject == myPlayer.hiddenWound) return false; // you cannot drop a smal wound

		var pileId = heldObject.objectData.getPileObjId();

		// drop on ground to process
		// 225 Wheat Sheaf // 1113 Ear of Corn  // 292 Basket // 233 Wet Clay Bowl
		// For now allowed: 126 Clay // 236 Clay Plate
		//var dontUsePile = allowAllPiles ? [] : [225, 1113, 126, 236, 292, 233];
		var dontUsePile = allowAllPiles ? [] : [225, 1113, 292, 233];
		
		if (heldObjId == 2144) dropOnStart = false; // 2144 Banana Peel
		else if (heldObjId == 34) dropOnStart = false; // 34 Sharp Stone
		else if (heldObjId == 135) dropOnStart = false; // 135 Flint Chip
		else if (heldObjId == 57) dropOnStart = false; // 57 Milkweed Stalk
		else if (heldObjId == 3180) dropOnStart = false; // 3180 Flat Rock with Rabbit Bait

		if(ServerSettings.DebugAi) 
			trace('AAI: ${myPlayer.name + myPlayer.id} DROP: ${myPlayer.heldObject.name} to ${infos.methodName}');
				
		if(storeInQuiver()) return true;

		// Bowl of Dough 252 + Clay Plate 236 // keep last use for making bread
		if(heldObjId == 252 && heldObject.numberOfUses > 1 && shortCraft(252, 236,5)) return true;

		// Basket of Bones 356
		if(heldObjId == 356){
			var graveyard = GetGraveyard();
			if(graveyard != null){
				target = graveyard;
				dropCloseToPlayer = false;
			}
		}
	
		// Clay 126 ==> drop close to kiln if close, otherwise drop in basket
		if(heldObjId == 126){ 
			var kiln = GetKiln();
			if(kiln != null) {
				dropCloseToPlayer = false;
				target = kiln;
			}

			var distanceToKiln = myPlayer.CalculateQuadDistanceToObject(target);

			if(distanceToKiln > 400){ //225
				// search if there is a clay basket
				var basket = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 292, 10, null, myPlayer, [126]); // Basket 292, Clay 126
				if(basket == null) basket = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 292, 10, null, myPlayer); // Basket 292

				if(basket != null){
					if (ServerSettings.DebugAiSay) myPlayer.say('drop clay in basket');
					if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gatherClay: drop clay in basket: d: $distanceToKiln');

					return useHeldObjOnTarget(basket); // fill basket	
				}
			}
		}

		// Flat Rock 291 // Stone 33
		if(heldId == 291 || heldId == 33){
			var forge = GetForge();
			var maxItems = heldId == 291 ? 3 : 1;
			// TODO solve that flat stones wont pile up if piles are not counted
			var countPiles = heldId == 33; 
			//var countPiles = true;
			
			if(forge != null){
				dropCloseToPlayer = false;
				var count = AiHelper.CountCloseObjects(myPlayer, forge.tx, forge.ty, heldId, 3, countPiles);

				if(count < maxItems){
					if(heldId == 291) pileId = 0; 
					dropCloseToPlayer = false;
					target = forge;
					mindistance = -1; // allow be droped close
				}
			}
		}

		// Basket 292, Clay 126 ==> drop close to kiln
		if(heldId == 292 && heldObject.contains([126])){
			pileId = 0;
			var kiln = GetKiln();
			if(kiln != null){
				target = kiln;
				dropCloseToPlayer = false;
				newDropTarget = myPlayer.GetClosestObjectToTarget(target, 0, 5);

				// switch with close // -10 looks for non permanent that is not same like heldobj
				if(newDropTarget == null) newDropTarget = myPlayer.GetClosestObjectToTarget(target, -10, 20);	
				if(newDropTarget != null){
					this.dropIsAUse = false;
					this.dropTarget = newDropTarget;
					return true;
				}
			}
		}
		
		if(dropNearOvenItemIds.contains(heldId) || pies.contains(heldId) || rawPies.contains(heldId)){
			target = myPlayer.home; // drop near home which is normaly the oven	
			dropCloseToPlayer = false;
		}

		// TODO what is if super far away from oven?
		// Clay Plate 236 ==> make sure that are not piled plates near oven
		if(heldId == 236){
			var count = AiHelper.CountCloseObjects(myPlayer, target.tx, target.ty, heldId, 10, false);
			// pile if more then 5
			if(count < 5){
				pileId = 0; 

				newDropTarget = myPlayer.GetClosestObjectToTarget(target, 0, 5);

				// switch with close // -10 looks for non permanent that is not same like heldobj
				if(newDropTarget == null) newDropTarget = myPlayer.GetClosestObjectToTarget(target, -10, 20);	
					
				if(newDropTarget != null){
					this.dropIsAUse = false;
					this.dropTarget = newDropTarget;
					return true;
				}
			}
		}

		// drop at fire. For exmple kindling, wood...
		if(dropNearFireItemIds.contains(heldObjId)){
			if(myPlayer.firePlace != null) dropTarget = myPlayer.firePlace;
			dropCloseToPlayer = false;
		}

		// only bring stuff home if it is useful
		if(dropCloseToPlayer) dropOnStart = false;  

		if (dropOnStart && maxDistanceToHome > 0) {
			var quadMaxDistanceToHome = Math.pow(maxDistanceToHome, 2);
			var quadDistance = myPlayer.CalculateQuadDistanceToObject(target);

			// check if not too close or too far 
			if (quadDistance > quadIsCloseEnoughDistanceToTarget && quadDistance < quadMaxDistanceToHome) {
				var done = myPlayer.gotoObj(target);
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done drop goto ${target.name} $quadDistance');

				if(done){
					if(ServerSettings.DebugAiSay) myPlayer.say('Goto home!');
					return true;
				}
				
				if(ServerSettings.DebugAiSay) myPlayer.say('Cannot Goto home!');
			}
		}
	
		if(dropCloseToPlayer){
			target = new ObjectHelper(null, 0);
			target.tx = myPlayer.tx;
			target.ty = myPlayer.ty;	
		}

		if(dontUsePile.contains(heldId)) pileId = 0; 

		if(pileId > 0){
			newDropTarget = myPlayer.GetClosestObjectToTarget(target, pileId, 4 + mindistance); 
			if(newDropTarget != null && newDropTarget.numberOfUses >= newDropTarget.objectData.numUses) newDropTarget = null;
			//if(newDropTarget != null)  trace('AAI: ${myPlayer.name + myPlayer.id} drop on pile: $pileId');
		}
		
		// start a new pile?
		if(newDropTarget == null && pileId > 0) newDropTarget = myPlayer.GetClosestObjectToTarget(target, myPlayer.heldObject.id, 4 + mindistance, mindistance);

		// get empty tile
		if(newDropTarget == null) newDropTarget = myPlayer.GetClosestObjectToTarget(target, 0, searchDistance, mindistance);

		// dont drop on a pile if last transition removed it from similar pile // like picking a bowl from a pile to put it then back on a pile
		if(newDropTarget.id > 0 && itemToCraft.lastNewTargetId == newDropTarget.id){
			trace('AAI: ${myPlayer.name + myPlayer.id} ${newDropTarget.name} dont drop on pile where item was just taken from');
			newDropTarget = myPlayer.GetClosestObjectToTarget(target, 0, searchDistance, mindistance);			
		}

		var heldId = myPlayer.heldObject.parentId;
		// check if there is a gound transition
		// maybe better to opt in. since wet clay bowl in tongs shouls not make a use while while a fired one should
		var transition = null; 
		//var transition = TransitionImporter.GetTransition(heldId, 0); 
		//if(transition == null) transition = TransitionImporter.GetTransition(heldId, -1);

		// dont use drop if held is Basket of Bones (356) to empty it! // 336 Basket of Soil
		// 1137 Bowl of Soil // 186 Cooked Rabbit 
		// 283 Wooden Tongs with Fired Bowl // 241 Fired Plate in Wooden Tongs
		// Cool Steel Crucible in Wooden Tongs 324 // Hot Steel Crucible in Wooden Tongs 323			
		var dontUseDropForItems = [356, 336, 1137, 186, 283, 241, 324, 323];
		//if (newDropTarget.id == 0 &&  heldId != 356 && heldId != 336 && heldId != 1137){ 
		if (newDropTarget.id == 0 && dontUseDropForItems.contains(heldId) == false && transition == null){ 
			this.dropIsAUse = false;
			this.dropTarget = newDropTarget;
		}
		else{
			this.dropIsAUse = true;
			this.dropTarget = null;
			this.useTarget = newDropTarget;
			this.useActor = new ObjectHelper(null, myPlayer.heldObject.id);
		}
		
		// if(itemToCraft.transTarget.parentId == myPlayer.heldObject.parentId)

		if (ServerSettings.DebugAi && newDropTarget != null) trace('AAI: ${myPlayer.name + myPlayer.id} Drop ${myPlayer.heldObject.name} new target: ${newDropTarget.name}');
		if (ServerSettings.DebugAi && newDropTarget == null) trace('AAI: ${myPlayer.name + myPlayer.id} Drop ${myPlayer.heldObject.name} new target: null!!!');
		// x,y is relativ to birth position, since this is the center of the universe for a player
		// if(emptyTileObj != null) playerInterface.drop(emptyTileObj.tx - myPlayer.gx, emptyTileObj.ty - myPlayer.gy);

		return true;
	}

	public function isChildAndHasMother(){ // must not be his original mother
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

	private function killAnimal(animal:ObjectHelper) : Bool {
		if (animal == null && animalTarget == null){
			if(hasOrBecomeProfession('Hunter') == false) return false;

			var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(timeLookedForDeadlyAnimalAtHome);
			if(passedTime > 20){
				//trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: look for wolf at home');
				timeLookedForDeadlyAnimalAtHome = TimeHelper.tick;
				this.animalTarget = AiHelper.GetClosestObjectToPosition(myPlayer.home.tx, myPlayer.home.ty, 418, 20); // Wolf
			}

			if(this.animalTarget == null){ 
				var quiver = myPlayer.getClothingById(3948); // 3948 Arrow Quiver
				if(quiver == null) quiver = myPlayer.getClothingById(4151); // 4151 Arrow Quiver with Bow
				if(quiver == null) quiver = myPlayer.getClothingById(874); // 874 Empty Arrow Quiver
				if(quiver == null) quiver = myPlayer.getClothingById(4149); // 4149 Empty Arrow Quiver with Bow
				
				profession['Hunter'] = quiver == null ? 0 : 1;
				return false;
			}
		}

		if (foodTarget != null) return false;

		var objData = ObjectData.getObjectData(152); // Bow and Arrow
		if (myPlayer.age < objData.minPickupAge) return false;

		if (animalTarget != null && animalTarget.isKillableByBow() == false) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: Old target not killable with bow anymore: ${animalTarget.description}');
			animalTarget = null;
		}

		if (animalTarget == null && animal != null) {
			if (animal.isKillableByBow()) this.animalTarget = animal;
			else if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: Not killable with bow: ${animal.description}');
		}

		if (animalTarget == null) return false;

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} killAnimal: ${animalTarget.description}');

		// 151 Yew Bow
		if (myPlayer.heldObject.id == 151){
			// Arrow Quiver
			var quiver = myPlayer.getClothingById(3948);
			if(quiver != null){
				myPlayer.self(0,0,5);
				if(ServerSettings.DebugAi) 
					trace('AAI: ${myPlayer.name + myPlayer.id} get Arrow from Quiver!');
				return true;
			} 
		}

		if (myPlayer.heldObject.id == 0){
			// Arrow Quiver with Bow
			var quiver = myPlayer.getClothingById(4151);

			if(quiver != null){
				myPlayer.self(0,0,5);
				if(ServerSettings.DebugAi) 
					trace('AAI: ${myPlayer.name + myPlayer.id} get Bow from Quiver!');
				return true;
			} 
		}

		// Arrow
		if (myPlayer.heldObject.id == 148){
			// 4149 Empty Arrow Quiver with Bow
			var quiver = myPlayer.getClothingById(4149);
			// Arrow Quiver with Bow
			if(quiver == null) quiver = myPlayer.getClothingById(4151);

			if(quiver != null && quiver.canAddToQuiver()){
				myPlayer.self(0,0,5);
				if(ServerSettings.DebugAi) 
					trace('AAI: ${myPlayer.name + myPlayer.id} KillAnimal: put Arrow in Quiver!');
				return true;
			} 
		}

		if (myPlayer.heldObject.id != objData.id) {
			// 4149 Empty Arrow Quiver with Bow
			var quiver = myPlayer.getClothingById(4149);
			// Arrow Quiver with Bow
			if(quiver == null) quiver = myPlayer.getClothingById(4151);

			if(quiver == null) return GetOrCraftItem(152); // Bow and Arrow
			else return GetOrCraftItem(148); // Arrow
		}

		var distance = myPlayer.CalculateQuadDistanceToObject(animalTarget);
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

	private function PickupItem(objId:Int) : Bool {
		var home = myPlayer.home;
		var obj = myPlayer.GetClosestObjectToTarget(home, objId, 20);
		if(obj == null) return false;
		PickupObj(obj);
		return true;
	}

	private function PickupObj(obj:ObjectHelper) : Bool{
		if(obj.isPermanent()) return false;
		this.dropIsAUse = false;
		this.dropTarget = obj;
		this.useTarget = null;
		this.useActor = null;
		return true;
	}

	private function GetItem(objId:Int) : Bool {
		return GetOrCraftItem(objId, false);
	}

	private function GetOrCraftItem(objId:Int, craft:Bool = true, minDistance:Int = 0) : Bool {
		if (myPlayer.isMoving()) return true;
		var objdata = ObjectData.getObjectData(objId);
		var pileId = objdata.getPileObjId();
		var hasPile = pileId > 0;
		var maxSearchDistance = 40;
		var searchDistance:Int = hasPile ? 5 : maxSearchDistance;
		var obj = myPlayer.GetClosestObjectById(objId, null, searchDistance, minDistance);
		var pile = hasPile ? myPlayer.GetClosestObjectById(pileId, null, searchDistance, minDistance) : null; 

		var usePile = pile != null && obj == null;
		if (usePile) obj = pile;
		if (obj == null && hasPile) obj = myPlayer.GetClosestObjectById(objId, null, maxSearchDistance, minDistance);

		if (obj == null && craft == false) return false;

		if (obj == null) return craftItem(objId);

		if (ServerSettings.DebugAi) 
			trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: found ${obj.name} pile: $usePile');

		// If picking up from a pile or a container like Basket make sure that hand is empty
		// TODO consider drop after movement
		if ((usePile || obj.objectData.numSlots > 0) && dropHeldObject()) return true;

		if(usePile){
			this.dropIsAUse = true;
			this.dropTarget = null;
			this.useTarget = obj;
			this.useActor = new ObjectHelper(null, myPlayer.heldObject.id);
		}
		else{
			this.dropIsAUse = false;
			this.dropTarget = obj;
			this.useTarget = null;
			this.useActor = null;
		}

		return true;

		/*
		var distance = myPlayer.CalculateQuadDistanceToObject(obj);

		if (distance > 1) {
			var done = myPlayer.gotoObj(obj);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: done: $done goto pickup d: $distance');
			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);

			if (ServerSettings.DebugAi || done == false)
				trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: child drop for get item ${heldPlayer.name} $done');

			return true;
		}

		//if(obj.parentId == 292) trace('Pickup Baske usePile: $usePile');

		// if(ServerSettings.DebugAi) trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = false;
		if(usePile){
			done = myPlayer.use(obj.tx - myPlayer.gx, obj.ty - myPlayer.gy);
		}
		else done = myPlayer.drop(obj.tx - myPlayer.gx, obj.ty - myPlayer.gy);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: ${myPlayer.heldObject.name} done: $done pickup obj');

		return done;*/
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

		if (this.feedingPlayerTarget == null){
			profession['FoodServer'] = 0;
			this.feedingPlayerTarget = AiHelper.GetCloseStarvingPlayer(myPlayer);
		}
		if (this.feedingPlayerTarget == null) return false;

		var targetPlayer = this.feedingPlayerTarget;

		if (targetPlayer.food_store > targetPlayer.food_store_max * 0.85) {
			this.feedingPlayerTarget = null;
			return false;
		}

		if(hasOrBecomeProfession('FoodServer', 2) == false) return false;

		if (myPlayer.heldObject.objectData.foodValue < 1
			|| myPlayer.heldObject.id == 837) // dont feed 837 ==> Psilocybe Mushroom to others
		{
			foodTarget = AiHelper.SearchBestFood(targetPlayer, myPlayer);
			if(foodTarget == null){
				this.feedingPlayerTarget = null;
				return false;
			}
			return true;
		}

		if (targetPlayer.isDeleted()) {
			this.feedingPlayerTarget = null;
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} cannot feed ${targetPlayer.name} since is dead!');
			return false;
		}

		if (targetPlayer.getHeldByPlayer() != null) {
			this.feedingPlayerTarget = null;
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} cannot feed ${targetPlayer.name} since held by other player!');
			return false;
		}

		if (targetPlayer.canFeedToMe(myPlayer.heldObject) == false) {
			this.feedingPlayerTarget = null;
			// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name} cannot feed ${targetPlayer.name} ${myPlayer.heldObject.name}');
			trace('AAI: ${myPlayer.name + myPlayer.id} cannot feed ${targetPlayer.name} ${myPlayer.heldObject.name} foodvalue: ${myPlayer.heldObject.objectData.foodValue} foodpipes: ${Math.round(targetPlayer.food_store / 10)*10} foodspace: ${Math.round((targetPlayer.food_store_max - targetPlayer.food_store) * 10)/10}');
			// if droped it can be stuck in a cyle if it want for example craft carrot and picks it up again. return true instead of false might also solve this
			// if not dropped it can be stuck in a cyle try to feed BOWL OF GOOSEBERRIES again and again
			this.dropHeldObject(5); // since food might be too big or too bad to feed
			return true; // false
		}

		var distance = myPlayer.CalculateDistanceToPlayer(targetPlayer);

		if (distance > 10 && myPlayer.isMoving()) return true;

		if (distance > 1) {

			if(myPlayer.isMoving()){
				myPlayer.forceStopOnNextTile = true;
				return true;
			}

			var done = myPlayer.gotoAdv(targetPlayer.tx, targetPlayer.ty);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done goto feed starving ${targetPlayer.name} dist: $distance');
			return true;
		}

		if (targetPlayer.name == ServerSettings.StartingName && targetPlayer.age > 1.5) {
			var newName = myPlayer.isEveOrAdam() ? NamingHelper.GetRandomName(targetPlayer.isFemale()) : myPlayer.name;
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} newName: $newName');
			myPlayer.say('You are $newName');
		}

		var done = myPlayer.doOnOther(targetPlayer.tx - myPlayer.gx, targetPlayer.ty - myPlayer.gx, -1, targetPlayer.id);
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done feed starving ${targetPlayer.name}');
		time += 2; // wait 2 sec

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
					trace('AAI: ${myPlayer.name + myPlayer.id} cannot feed ==> child drop ${heldPlayer.name} food: ${heldPlayer.food_store} $done');
			}
			return false;
		}
		//if (foodTarget != null) return false;
		if (heldPlayer != null) {
			if (heldPlayer.name == ServerSettings.StartingName && (heldPlayer.mother == myPlayer || heldPlayer.age > 1.5)) {
				var newName = myPlayer.isEveOrAdam() ? NamingHelper.GetRandomName(heldPlayer.isFemale()) : myPlayer.name;
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
		if (distance > ServerSettings.PickupBabyMaxDistance - 0.02) {
			var done = myPlayer.gotoAdv(child.tx, child.ty);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done goto child to feed ${child.name}');

			return true;
		}

		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop obj for feeding child');
			dropHeldObject(0);
			return true;
		}

		var childX = child.tx - myPlayer.gx;
		var childY = child.ty - myPlayer.gy;

		if(ServerSettings.DebugAiSay) myPlayer.say('Pickup ${child.name}');
		var done = myPlayer.doBaby(childX, childY, child.id);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} child ${child.name} pickup $done');

		return true;
	}

	private function escape(animal:ObjectHelper, deadlyPlayer:GlobalPlayerInstance) {
		var startTime = Sys.time();

		if (animal == null && deadlyPlayer == null) return false;
		if (myPlayer.food_store < -1) return false;
		// if(myPlayer == null) throw new Exception('WARNING! PLAYER IS NULL!!!');
		// if (ServerSettings.DebugAi) trace('escape: animal: ${animal != null} deadlyPlayer: ${deadlyPlayer != null}');
		// hunt this animal
		if (animal != null && animal.isKillableByBow()) animalTarget = animal;
		// go for hunting
		if (myPlayer.isHoldingWeapon() && myPlayer.isWounded() == false) return false;

		var player = myPlayer.getPlayerInstance();
		var escapeDist = 3;
		var distAnimal = animal == null ? 99999999 : AiHelper.CalculateQuadDistanceToObject(myPlayer, animal);
		var distPlayer = deadlyPlayer == null ? 99999999 : AiHelper.CalculateDistanceToPlayer(myPlayer, deadlyPlayer);
		var escapePlayer = deadlyPlayer != null && distAnimal > distPlayer;
		if (ServerSettings.DebugAi) trace('escape: distAnimal: ${distAnimal} distPlayer: ${distPlayer}');
		var description = escapePlayer ? deadlyPlayer.name : animal.description;
		var escapeTx = escapePlayer ? deadlyPlayer.tx : animal.tx;
		var escapeTy = escapePlayer ? deadlyPlayer.ty : animal.ty;
		var newEscapetarget = new ObjectHelper(null, 0);

		if(ServerSettings.DebugAiSay) myPlayer.say('Escape ${description} ${Math.ceil(didNotReachFood)}!');
		//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} escape!');

		var done = false;
		var alwaysX = false;
		var alwaysY = false;
		var checkIfDangerous = true;
		Connection.debugText = 'AI:escape'; 

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

				done = myPlayer.gotoObj(newEscapetarget, true, false); // TODO consider deadly animals

				// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Escape $done $ii $i alwaysX: $alwaysX alwaysY $alwaysY es: ${newEscapetarget.tx},${newEscapetarget.ty}');

				if (done) break;
				if ((Sys.time() - startTime) * 1000 > 200) break;
			}

			if (done) break;
			if ((Sys.time() - startTime) * 1000 > 200) break;

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
		Connection.debugText = ''; 

		if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} escape! ${Math.round((Sys.time() - startTime) * 1000)}ms done: $done');

		return true;
	}

	// TODO consider backpack / contained objects
	// currently considers heldobject, close objects and objects close to home
	private function craftItem(objId:Int, count:Int = 1, ignoreHighTech:Bool = false):Bool {
		itemToCraft.ai = this;

		// To save time, craft only if this item crafting did not fail resently
		var player = myPlayer.getPlayerInstance();
		var failedTime = failedCraftings[objId];
		var passedTimeSinceFailed = TimeHelper.CalculateTimeSinceTicksInSec(failedTime);
		var waitTime =  ServerSettings.AiTimeToWaitIfCraftingFailed - passedTimeSinceFailed;
		
		if(waitTime > 0){
			//if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft item ${GetName(objId)} wait before trying again! ${waitTime}');	
			return false;
		}

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} t: ${TimeHelper.tick} craft item ${GetName(objId)}!');

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
			itemToCraft.lastActorId = -1;
			itemToCraft.lastTargetId = -1;
			itemToCraft.lastNewActorId = -1;
			itemToCraft.lastNewTargetId = -1;

			var startTime = Sys.time();
			itemToCraft.transitionsByObjectId = new Map<Int, TransitionForObject>();
			//itemToCraft.transitionsByObjectId = myPlayer.SearchTransitions(objId, ignoreHighTech);
			//if(ServerSettings.DebugAi) trace('AI: craft: FINISHED transitions1 ms: ${Math.round((Sys.time() - startTime) * 1000)}');


			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} new item to craft: ${itemToCraft.itemToCraft.description}!');
		}

		searchBestObjectForCrafting(itemToCraft);

		// set position where to craft the object
		if (itemToCraft.startLocation == null && itemToCraft.transTarget != null) {
			itemToCraft.startLocation = new ObjectHelper(null, 0);
			//itemToCraft.startLocation.tx = myPlayer.tx; // itemToCraft.transTarget.tx;
			//itemToCraft.startLocation.ty = myPlayer.ty; // itemToCraft.transTarget.ty;

			// use home as crafting startLocation so that stuff is hopefully droped at home 
			if(myPlayer.home != null && myPlayer.IsCloseToObject(myPlayer.home, 60)){
				itemToCraft.startLocation.tx = myPlayer.home.tx;
				itemToCraft.startLocation.ty = myPlayer.home.ty;
				//trace('AAI: ${myPlayer.name + myPlayer.id} craft: startLocation --> home');
			}
			else{
				itemToCraft.startLocation.tx = itemToCraft.transTarget.tx;
				itemToCraft.startLocation.ty = itemToCraft.transTarget.ty;

				//var quadDistance = myPlayer.home != null ? myPlayer.CalculateQuadDistanceToObject(myPlayer.home) : -1;
				//trace('AAI: ${myPlayer.name + myPlayer.id} craft: startLocation --> transTarget home: ${myPlayer.home != null} d: ${quadDistance}');
			}
		}

		if (itemToCraft.transActor == null) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} craft: FAILED ${itemToCraft.itemToCraft.description} did not find any item in search radius for crafting!');

			failedCraftings[objId] = TimeHelper.tick;
			// TODO give some help to find the needed Items

			if(itemToCraftName != null){
				myPlayer.say('Failed to craft $itemToCraftName');
				itemToCraftName = null;
			}

			return false;
		}

		// if(player.heldObject.parentId == itemToCraft.transActor.parentId)
		// check if actor is held already
		if (player.heldObject.parentId == itemToCraft.transActor.parentId || itemToCraft.transActor.id == 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft actor ${itemToCraft.transActor.name} is held already or Empty. Craft target ${itemToCraft.transTarget.name} ${itemToCraft.transTarget.id} held: ${player.heldObject.name}');

			if(ServerSettings.DebugAiSay) myPlayer.say('Goto target ' + itemToCraft.transTarget.name);

			if (itemToCraft.transActor.id == 0 && player.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} t: ${TimeHelper.tick} craft: drop heldobj at start since Empty is needed!');
				dropHeldObject();
				return true;
			}

			useTarget = itemToCraft.transTarget;
			useActor = itemToCraft.transActor;
			itemToCraft.transActor = null; // actor is allready in the hand

			return true;
		} 
		// if the actor is not yet held in hand

		// check if actor is TIME
		if (itemToCraft.transActor.id == -1) {
			var secondsUntillChange = itemToCraft.transTarget.timeUntillChange();

			if(secondsUntillChange < 10){
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft Actor is TIME target ${itemToCraft.transTarget.name} ');
				this.time += secondsUntillChange / 4;
				// TODO wait some time, or better get next obj
				
				if(ServerSettings.DebugAiSay) myPlayer.say('Wait for ${itemToCraft.transTarget.name}...');
				itemToCraft.transActor = null;
				return true;
			}

			itemToCraft.transActor = null;
			itemToCraft.transTarget = null;
			return false; // TODO make some other stuff???
		}
		
		// check if actor is PLAYER
		if (itemToCraft.transActor.id == -2) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft Actor is PLAYER ');

			// TODO PLAYER interaction not supported yet
			if(ServerSettings.DebugAiSay) myPlayer.say('Actor is player!?!');
			itemToCraft.transActor = null;
			return false;
		}

		// check if there is a close pile where the actor can be taken from
		var pileId = itemToCraft.transActor.objectData.getPileObjId();
		var pileData = pileId < 1 ? null : itemToCraft.transitionsByObjectId[pileId];
		var pile = pileData == null ? null : pileData.closestObject;

		if(pile != null){
			var quadDistanceToActor = AiHelper.CalculateQuadDistanceToObject(myPlayer, itemToCraft.transActor);
			var quadDistanceToPile = AiHelper.CalculateQuadDistanceToObject(myPlayer, pile);

			// be ready to go for not piled objects little bit more distant
			if(quadDistanceToActor < quadDistanceToPile * 1.5) pile = null;
		}

		if(pile == null){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft goto actor: ${itemToCraft.transActor.name}[${itemToCraft.transActor.id}]');
			if (ServerSettings.DebugAiSay) myPlayer.say('Goto actor ' + itemToCraft.transActor.name);

			dropTarget = itemToCraft.transActor;
		}
		else{
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft goto piled actor: ${itemToCraft.transActor.name}[${itemToCraft.transActor.id}]');
			if (ServerSettings.DebugAiSay) myPlayer.say('Goto piled actor ' + itemToCraft.transActor.name);

			useActor = new ObjectHelper(null, 0);
			useTarget = pile;
		}
		
		var isHoldingObject = myPlayer.isHoldingObject();
		if(isHoldingObject && considerDropHeldObject(itemToCraft.transTarget)){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop ${myPlayer.heldObject.name} to pickup ${itemToCraft.transActor.name}');
			return true;
		}

		// usemight drop item so not neededanymore ???
		/*if(pile != null)){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop ${myPlayer.heldObject.name} to pickup ${itemToCraft.transActor.name}');
			dropHeldObject();
			return true;
		}*/

		return true;
	}

	private function searchBestObjectForCrafting(itemToCraft:IntemToCraft):IntemToCraft {
		var startTime = Sys.time();
		itemToCraft.transActor = null;
		itemToCraft.transTarget = null;
		if(itemToCraft.maxSearchRadius < 1) itemToCraft.maxSearchRadius = ServerSettings.AiMaxSearchRadius;

		var objToCraftId = itemToCraft.itemToCraft.parentId;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;

		
		var player = myPlayer.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var radius = 0;

		while (radius < itemToCraft.maxSearchRadius) {
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

			var startTime = Sys.time();
			// add objects at home
			addObjectsForCrafting(myPlayer.home.tx, myPlayer.home.ty, radius, transitionsByObjectId, false);			
			if(itemToCraft.searchCurrentPosition) addObjectsForCrafting(baseX, baseY, radius, transitionsByObjectId, false);
			//if(myPlayer.firePlace != null) addObjectsForCrafting(myPlayer.firePlace.tx, myPlayer.firePlace.ty, radius, transitionsByObjectId);

			if(ServerSettings.DebugAi) trace('AI: craft: FINISHED objects ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius}');
			var startTime = Sys.time();

			/*itemToCraft.clearTransitionsByObjectId();
			addObjectsForCrafting(myPlayer.home.tx, myPlayer.home.ty, radius, transitionsByObjectId, false);
			if(itemToCraft.searchCurrentPosition) addObjectsForCrafting(baseX, baseY, radius, transitionsByObjectId, false);
			if(ServerSettings.DebugAi) trace('AI: craft: FINISHED objects2 ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius}');

			var startTime = Sys.time();*/

			searchBestTransitionTopDown(itemToCraft);

			if(ServerSettings.DebugAi) trace('AI: craft: FINISHED transitions ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius}');

            this.time += Sys.time() - startTime;

			if (itemToCraft.transActor != null) return itemToCraft;
		}

		return itemToCraft;
	}

	private function addObjectsForCrafting(baseX:Int, baseY:Int, radius:Int, transitionsByObjectId:Map<Int, TransitionForObject>, onlyRelevantObjects = true) {
		var world = myPlayer.getWorld();

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

				// TODO for all biome locked stuff
				// Fertile Soil 1138 // Hardened Row 848
				if (objData.parentId == 1138 || objData.parentId == 848){
					var biomeId = world.getBiomeId(tx,ty);
					if(biomeId == BiomeTag.SNOW || biomeId == BiomeTag.OCEAN) continue;
				}

				var trans = transitionsByObjectId[objData.parentId];

				// check if object can be used to craft item									
				if(trans == null){
					if(onlyRelevantObjects) continue; // object is not useful for crafting wanted object
					else{
						trans = new TransitionForObject(objData.parentId, 0, 0, null);
						transitionsByObjectId[objData.parentId] = trans;
					}
				}

				//var steps = trans.steps;
				var obj = world.getObjectHelper(tx, ty);				
				var objQuadDistance = myPlayer.CalculateQuadDistanceToObject(obj);

				// dont use carrots if seed is needed // 400 Carrot Row
				if (obj.parentId == 400 && hasCarrotSeeds == false && obj.numberOfUses < 3) continue;
				// Ignore not full Bowl of Gooseberries 253 otherwise it might get stuck in making a pie
				if (obj.parentId == 253 && obj.numberOfUses < objData.numUses) continue;
				// Dont eat if no corn seeds // 1114 Shucked Ear of Corn
				//if (obj.parentId == 1114 && this.hasCornSeeds == false) continue;

				// if objects from different positions like home are added, check if obj is allready added
				if (trans.closestObject != null && obj.tx == trans.closestObject.tx && obj.ty == trans.closestObject.ty) continue;
				if (trans.secondObject != null && obj.tx == trans.secondObject.tx && obj.ty == trans.secondObject.ty) continue;

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

		/*if (itemToCraft.transActor != null && itemToCraft.transActor.name == null)
			descActor += itemToCraft.transActor == null ? '' : ' ${itemToCraft.transActor.id} ${itemToCraft.transActor.description}';
		if (itemToCraft.transTarget != null && itemToCraft.transTarget.name == null)
			descTarget += itemToCraft.transTarget == null ? '' : ' ${itemToCraft.transTarget.id} ${itemToCraft.transTarget.description}';
		*/

		if (ServerSettings.DebugAiCrafting)
			trace('AI: ${itemToCraft.ai.myPlayer.name + itemToCraft.ai.myPlayer.id} craft: FOUND $count ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius} dist: ${itemToCraft.bestDistance} ${obj.name} --> $descActor + $descTarget');
	}

	private static function DoTransitionSearch(itemToCraft:IntemToCraft, wantedId:Int, objectsToSearch:Array<Int>, transitions:Array<TransitionData>):Bool {
		var found = false;
		var transitionsByObjectId = itemToCraft.transitionsByObjectId;
		var wanted = transitionsByObjectId[wantedId];
		var objToCraftId = itemToCraft.itemToCraft.parentId;
		var objToCraftPileId = itemToCraft.itemToCraft.getPileObjId();

		for (trans in transitions) {
			// if(ServerSettings.DebugAi) trace('Ai: craft: ' + trans.getDesciption());
			if (trans.actorID == wantedId || trans.actorID == objToCraftId) continue;
			if (trans.targetID == wantedId || trans.targetID == objToCraftId) continue;
			if (objToCraftPileId > 0 && trans.targetID == objToCraftPileId) continue;
			//if (trans.aiShouldIgnore) trace('Ígnore ${trans.getDesciption()}');
			if (trans.aiShouldIgnore) continue;
			// dont undo last transition // Should solve Taking a Rabit Fur from a pile if in the last transition the Ai put it on the pile
			if (trans.newActorID == itemToCraft.lastActorId && trans.newTargetID == itemToCraft.lastTargetId){
				//trace('Ignore transition since it undos last: ${trans.getDesciption()}');
				continue;
			}
			// ignore time transitions that make 732 Ashes with Bowl since Ai uses that to get empty bowl
			if (trans.actorID == -1 && trans.newTargetID == 732) continue;
			if (trans.targetID == -1){
				//trace('Ignore transition since target is -1 (player?): ${trans.getDesciption()}');
				continue;
			}

			// a oven needs 15 sec to warm up this is ok, but waiting for mushroom to grow is little bit too long!
			if (trans.calculateTimeToChange() > ServerSettings.AiIgnoreTimeTransitionsLongerThen) continue;

			//var actor = transitionsByObjectId[trans.actorID];
			//var target = transitionsByObjectId[trans.targetID];

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

			/*if (actor == null || target == null) {
				// if(ServerSettings.DebugAi) trace('Ai: craft: Skipped: ' + trans.getDesciption());
				continue;
			}*/

			// TODO should not be null must be bug in tansitions: Basket of Pig Bones + TIME  -->  Basket + Pig Bones#dumped
			// if(actor == null) transitionsByObjectId[trans.actorID] = new TransitionForObject(trans.actorID,0,0,null);
			// if(target == null) transitionsByObjectId[trans.targetID] = new TransitionForObject(trans.targetID,0,0,null);

			var actor = transitionsByObjectId[trans.actorID];
			var target = transitionsByObjectId[trans.targetID];

			if(actor == null){ 
				actor = new TransitionForObject(trans.actorID,0,0,null);
				transitionsByObjectId[trans.actorID] = actor;

				// check if there is a pile
				var objData = ObjectData.getObjectData(trans.actorID);
				var pileId = objData.getPileObjId();
				if (pileId > 0){
					var pile = transitionsByObjectId[pileId];
					if(pile != null){
						actor.usePile = true;
						actor.closestObject = pile.closestObject;
					}
				}
			}
			if(target == null){ 
				target = new TransitionForObject(trans.targetID,0,0,null);
				transitionsByObjectId[trans.targetID] = target;
			}

			var actorObj = actor.closestObject;
			var targetObj = actor == target ? actor.secondObject : target.closestObject;

            // TODO consider cyles like: put thread in claybowls to get a thread
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
				
				// If actor is not the wanted object but a pile
				if(actor.usePile){
					itemToCraft.transActor = new ObjectHelper(null,0);
					itemToCraft.transTarget = actorObj;

					trace('USE PILE: ${actorObj.name}');
				}
				else{
					itemToCraft.transActor = actorObj;
					itemToCraft.transTarget = targetObj;
				}
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
		var objData = ObjectData.getObjectData(objId);
		if(objData == null){
			trace('WARNING could not find objData to $objId');
			return 'NULL';
		}

		return objData.name;
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

					//if(ServerSettings.DebugAi) trace('Ai: craft TIME not wanted: ${GetName(objNoTimeWanted)} dist: $dist ${trans.getDesciption()}');
				} else {
					altActor = obj.craftActor;
					altTarget = obj.craftTarget;

					if (ServerSettings.DebugAi)
						trace('Ai: craft ${GetName(itemToCraft.itemToCraft.id)} TIME not wanted: ${GetName(objNoTimeWanted)} do first: ${GetName(doFirst)} trans: ${GetName(itemToCraft.transActor.id)} + ${GetName(itemToCraft.transTarget.id)}');
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

			textTrans += '${actor.name}[${actor.id}] + ${target.name}[${target.id}] $desc--> ';
		}

		// Clay with Nozzle 2110 // Small Lump of Clay 3891
		var doWarning = false;
		if(itemToCraft.craftingList.contains(2110) || itemToCraft.craftingList.contains(3891)){
			doWarning = true;
			text += ' WARNING! Nozzle';
			textTrans += ' WARNING! Nozzle';
		}

		var objToCraft = ObjectData.getObjectData(itemToCraft.itemToCraft.id);
		var myPlayer = itemToCraft.ai.myPlayer;
		if (doWarning || ServerSettings.DebugAiCrafting) trace('Ai: ${myPlayer.name + myPlayer.id} craft DONE items: ${itemToCraft.craftingList.length} ${objToCraft.name}: $text');
		if (doWarning || ServerSettings.DebugAiCrafting) trace('Ai: ${myPlayer.name + myPlayer.id} craft DONE trans: ${itemToCraft.craftingTransitions.length} ${objToCraft.name}: $textTrans');
	}

	private function isMovingToHome(maxDistance = 3):Bool {
		if(myPlayer.home == null) return false;
		maxDistance = maxDistance * maxDistance;

		var quadDistance = myPlayer.CalculateQuadDistanceToObject(myPlayer.home);

		if (quadDistance < maxDistance) return false;

		var dist = 2;
		var randX = WorldMap.calculateRandomInt(2 * dist) - dist;
		var randY = WorldMap.calculateRandomInt(2 * dist) - dist;
		var done = myPlayer.gotoAdv(myPlayer.home.tx + randX, myPlayer.home.ty + randY);
	
		if(ServerSettings.DebugAi) myPlayer.say('going home $done');

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} dist: $quadDistance goto home $done');

		return done;
	}

	private function searchNewHomeIfNeeded() : Bool {
		var world = WorldMap.world;
		var home = myPlayer.home;
		var obj = home == null ? [0] : world.getObjectId(home.tx, home.ty);
		
		// a home is where a oven is // TODO rebuild Oven if Rubble
		if(ObjectData.IsOven(obj[0]) || obj[0] == 753) return false; // 237 Adobe Oven // 753 Adobe Rubble

		var newHome = AiHelper.SearchNewHome(myPlayer);

		if(newHome != null) myPlayer.home = newHome;

		return false;
	}

	private function isMovingToPlayer(maxDistance = 3, followHuman:Bool = true):Bool {
		if(playerToFollow != null && playerToFollow.isDeleted()) playerToFollow = null;
		
		if (playerToFollow == null) {
			if (isChildAndHasMother()) {
				playerToFollow = myPlayer.getFollowPlayer();
			} else {
				if(ServerSettings.AutoFollowPlayer == false) return false;
				// get close human player
				playerToFollow = myPlayer.getWorld().getClosestPlayer(20, followHuman);
				// if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} follow player ${playerToFollow.p_id}');
			}
		}

		if (playerToFollow == null) return false;

		maxDistance = maxDistance * maxDistance;
		
		var quadDistance = myPlayer.CalculateDistanceToPlayer(playerToFollow);

		if (quadDistance < maxDistance) return false;

		if(myPlayer.isMoving()){
			//myPlayer.forceStopOnNextTile = true; // does not look nice, since its stops then continues again and again
			//return true;
			time += 1; // TODO can make the player look jumping, so give some extra time???
		}

		var dist = maxDistance >= 9 ? 2 : 1;
		var randX = WorldMap.calculateRandomInt(2 * dist) - dist;
		var randY = WorldMap.calculateRandomInt(2 * dist) - dist;
		var done = myPlayer.gotoAdv(playerToFollow.tx + randX, playerToFollow.ty + randY);

		if(myPlayer.age > ServerSettings.MinAgeToEat || ServerSettings.DebugAiSay) myPlayer.say('${playerToFollow.name}');

		if (myPlayer.isAi()) if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} age: ${Math.ceil(myPlayer.age * 10) / 10} dist: $quadDistance goto player $done');

		return done;
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

		// TODO support dropping in a container
		// If picking up a container like Basket make sure not to drop stuff in the container
		if(dropTarget.objectData.numSlots > 0 && dropHeldObject()) return true; 

		var distance = myPlayer.CalculateQuadDistanceToObject(dropTarget);
		// var myPlayer = myPlayer.getPlayerInstance();

		if (distance > 1) {
			var done = false;
			//for (i in 0...5) {
				done = myPlayer.gotoObj(dropTarget);

				//if (done) break;

				//dropTarget = myPlayer.GetClosestObjectById(0); // empty
			//}

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto drop: $done target: ${dropTarget.name} ${dropTarget.tx},${dropTarget.ty} distance: $distance');
			if (done == false) dropTarget = null;

			return true;			
		} 		

		var done = myPlayer.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);

		dropTarget = null;

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop $done now held: ${myPlayer.heldObject.name}');		

		return true;
	}

	private function isPickingupFood():Bool {
		if (foodTarget == null) return false;
		if (myPlayer.heldObject.parentId == foodTarget.parentId){
			foodTarget = null;
			return false;
		}

		// check if food is still eatable. Maybe some one eat it
		if (myPlayer.isEatableCheckAgain(foodTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} ${foodTarget.description} ${foodTarget.id} food changed meanwhile!');

			foodTarget = null;
			return true;
		}

		var isUse = foodTarget.isPermanent() || foodTarget.objectData.foodValue < 1;
		var isHoldingObject = myPlayer.isHoldingObject();

		// no matter if drop (switch) or use consider droppingheldObject before move
		if (isHoldingObject && considerDropHeldObject(foodTarget)) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} isPickingupFood: isUse: $isUse drop ${myPlayer.heldObject.name} since close to home or target less far away');
			return true;
		}

		if (myPlayer.isMoving()) return true;

		var distance = myPlayer.CalculateQuadDistanceToObject(foodTarget);
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

		// TODO pickup up droped object after eating
		if (isUse && isHoldingObject) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop held object to pickup food / after move');
			dropHeldObject(0);
			return true;
		}

		var done = false;

		// x,y is relativ to birth position, since this is the center of the universe for a player
		if(isUse) done = myPlayer.use(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy);
		else done = myPlayer.drop(foodTarget.tx - myPlayer.gx, foodTarget.ty - myPlayer.gy); // use drop for berry bowl
		
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

	private function isPickingupCloths() {
		if(myPlayer.age < ServerSettings.MinAgeToEat) return false;

		var switchCloths = shouldSwitchCloth(myPlayer.heldObject);
		
		if(switchCloths){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} switch cloth ${myPlayer.heldObject.name}');
			myPlayer.self();
			return true;
		}

		var clothings = myPlayer.GetCloseClothings();
		for(obj in clothings)
		{
			var switchCloths = shouldSwitchCloth(obj);

			if(switchCloths){
				dropTarget = obj;
				var slot = obj.objectData.getClothingSlot();
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} pickup clothing: ${obj.name} ${obj.objectData.clothing} slot: ${slot} current: ${myPlayer.clothingObjects[slot].name}');
				return true;
			}
		}
 
		return false;
	}

	private function shouldSwitchCloth(obj:ObjectHelper) {
		var slot = obj.objectData.getClothingSlot();
		
		if(slot < 0) return false;

		var switchCloths = myPlayer.clothingObjects[slot].id == 0;
		var isRag = obj.name.contains('RAG '); 

		// in case of shoes either one can be needed
		if(slot == 2) switchCloths = switchCloths || myPlayer.clothingObjects[3].id == 0;
		if(isRag == false && myPlayer.clothingObjects[slot].name.contains('RAG ')) switchCloths = true;

		return switchCloths;
	}

	private function isEating():Bool {
		if (myPlayer.age < ServerSettings.MinAgeToEat) return false;
		if (myPlayer.canEat(myPlayer.heldObject) == false) return false;
		if (isHungry == false && myPlayer.isHoldingYum() == false) return false;

		// dont eat Cooked Goose if there is only one since needed for crafting knife
		if (myPlayer.heldObject.parentId == 518){
			var home = myPlayer.home;
			var count = AiHelper.CountCloseObjects(myPlayer, home.tx, home.ty, 518, 20);
			if(count < 1) return false;
		}

		// var heldObjectIsEatable = myPlayer.heldObject.objectData.foodValue > 0;
		// if(heldObjectIsEatable == false) return false;

		var oldNumberOfUses = myPlayer.heldObject.numberOfUses;

		myPlayer.self(); // eat

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} Eat: held: ${myPlayer.heldObject.description}  newNumberOfUses: ${myPlayer.heldObject.numberOfUses} oldNumberOfUses: $oldNumberOfUses emptyFood: ${myPlayer.food_store_max - myPlayer.food_store}');

		this.didNotReachFood = 0;
		foodTarget = null;

		if (myPlayer.heldObject.objectData.foodValue <= 0) dropHeldObject(10); // drop for example banana peal
		return true;
	}

	private function checkIsHungryAndEat():Bool {
		var player = myPlayer.getPlayerInstance();
		var heldObject = myPlayer.heldObject;

		if (isHungry) {
			isHungry = player.food_store < player.food_store_max * 0.8;
		} else {
			//if(this.profession['Smith'] > 0)
			// Smithing Hammer 441
			var max = 3;
			if(heldObject.parentId == 441) max = 1; // dont be disturbed while smithing
			isHungry = player.food_store < Math.max(max, player.food_store_max * 0.3);
		}

		if (isHungry && foodTarget == null) searchFoodAndEat();

		if(ServerSettings.DebugAiSay) if (isHungry) myPlayer.say('F ${Math.round(myPlayer.getPlayerInstance().food_store)}'); // TODO for debugging
		if (isHungry && myPlayer.age < ServerSettings.MaxChildAgeForBreastFeeding) myPlayer.say('F');

		//if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} F ${Math.round(playerInterface.getPlayerInstance().food_store)} P:  ${myPlayer.x},${myPlayer.y} G: ${myPlayer.tx()},${myPlayer.ty()}');

		this.isCaringForFire = false; // food has priority
		return isHungry;
	}

	private function isUsingItem():Bool {
		if (useTarget == null) return false;

		var heldObject = myPlayer.heldObject;
		var isHoldingObject = myPlayer.isHoldingObject();

		if (myPlayer.isStillExpectedItem(useTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Use target changed meanwhile! ${useTarget.name}');
			useTarget = null;
			return false;
		}

		// only allow to go on with use if right actor is in the hand, or if actor will be empty
		if (heldObject.parentId != useActor.parentId) {

			if(useActor.parentId == 0){
				if(isHoldingObject && considerDropHeldObject(useTarget)) return true;
			}
			else {
				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} Use: not the right actor! ${myPlayer.heldObject.name} expected: ${useActor.name}');

				useTarget = null;
				useActor = null;
				// dropTarget = itemToCraft.transActor;

				dropHeldObject();

				return false;
			}
		}

		// TODO what about other actors wich need to be filled?
		// make sure that actor (Bowl of Gooseberries) is full 
		if(myPlayer.heldObject.parentId == 253 && heldObject.numberOfUses < heldObject.objectData.numUses){
			// TODO better check if(transition.tool == false && transition.reverseUseActor == false)
			// check if target is bush to allow still use to fill up 391 Domestic Gooseberry Bush
			if(useTarget.parentId != 30 && useTarget.parentId != 391) return fillBerryBowlIfNeeded();
		}

		/*if(transition.tool == false && transition.reverseUseActor == false){
			var numUses = player.heldObject.objectData.numUses;
			var heldUses = player.heldObject.numberOfUses;
			
			if(numUses > 1 && heldUses < numUses){
				if (ServerSettings.DebugTransitionHelper)
					trace('TRANS: ${player.name + player.id} ${player.heldObject.name} must have max uses ${heldUses} < ${numUses}');

				player.say('Must be full!', true);

				return false;
			}
		}*/

		if (myPlayer.isMoving()) return true;

		// TODO crafting does not yet consider if old enough to use a bow 
		// 152 Bow and Arrow
		if(myPlayer.heldObject.id == 152 && useTarget.isAnimal()){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Use: kill animal ${useTarget.description}');
			Macro.exception(if (killAnimal(useTarget)) return true);
		}

		var distance = myPlayer.CalculateQuadDistanceToObject(useTarget);
		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} Use: distance: $distance ${useTarget.description} ${useTarget.tx} ${useTarget.ty}');

		if (distance > 1) {
			var name = useTarget.name;
			var done = myPlayer.gotoObj(useTarget);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto useItem ${name} $done');

			if(ServerSettings.DebugAiSay){
				if (done) myPlayer.say('Goto ${name} for use!');
				else myPlayer.say('Cannot Goto ${name} for use!');
			}

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
		if (isHoldingObject && useActor.id == 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop obj to to have empty hand');
			dropHeldObject(0);
			return true;
		}
		
		var useActorName = myPlayer.heldObject.name; 
		var useActorId = myPlayer.heldObject.id;
		var useTargetId = useTarget.id;
		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = myPlayer.use(useTarget.tx - myPlayer.gx, useTarget.ty - myPlayer.gy);

		if (done) {
			// check if the use was part of a drop to put for example stone on a pile of stones
			if(dropIsAUse){
				dropIsAUse = false;
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} done: drop as a use!');
				if(foodTarget == null){
					useTarget = itemToCraft.transTarget;
					useActor = myPlayer.heldObject;
				}
				else{
					useTarget = null;
				}								

				return true;
			}	
			else{			
				var taregtObjectId = myPlayer.getWorld().getObjectId(useTarget.tx, useTarget.ty)[0];
				
				itemToCraft.done = true;
				itemToCraft.countTransitionsDone += 1;
				itemToCraft.lastActorId = useActorId;
				itemToCraft.lastTargetId = useTargetId;
				itemToCraft.lastNewActorId = myPlayer.heldObject.id;
				itemToCraft.lastNewTargetId = taregtObjectId;

				// if object to create is held by player or is on ground, then cound as done
				if (myPlayer.heldObject.parentId == itemToCraft.itemToCraft.parentId
					|| taregtObjectId == itemToCraft.itemToCraft.parentId){

					itemToCraft.countDone += 1;
					if(itemToCraftName != null && itemToCraft.itemToCraft.name == itemToCraftName) myPlayer.say('Finished $itemToCraftName');
					itemToCraftName = null; // is set if human gave order to craft
				}

				// in case its a pie, make next pie
				if(rawPies.contains(taregtObjectId)){
					countPies += 1;
					//lastPie += 1;
					if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} raw pie done: ${itemToCraft.itemToCraft.name} countPies: $countPies lastPie: $lastPie');
				}

				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} done: ${useActorName} + ${useTarget.name} ==> ${itemToCraft.itemToCraft.name} trans: ${itemToCraft.countTransitionsDone} finished: ${itemToCraft.countDone} FROM: ${itemToCraft.count}');
			}
		} else {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Use failed! Ignore: ${useTarget.name} ${useTarget.tx} ${useTarget.ty} ');

			// TODO check why use is failed... for now add to ignore list
			// TODO dont use on contained objects if result cannot contain (ignore in crafting search)
			this.addNotReachableObject(useTarget);
			useTarget = null;
			itemToCraft.transActor = null;
			itemToCraft.transTarget = null;
		}

		useTarget = null;
		dropIsAUse = false;

		return true;
	}

	private function isRemovingFromContainer():Bool {
		var target = this.removeFromContainerTarget;
		if (target == null) return false;
		
		if (myPlayer.isStillExpectedItem(expectedContainer) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Remove: expectedContainer changed meanwhile! ${expectedContainer.name}');
			removeFromContainerTarget = null;
			expectedContainer = null;
			return false;
		}

		if(target.containedObjects.length < 1){
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Remove: expectedContainer is already empty! ${expectedContainer.name}');
			removeFromContainerTarget = null;
			expectedContainer = null;
			return false;
		}

		// Drop object before move to pickup stuff. Otherwise she might run back to home to drop 
		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Remove: drop obj to have empty hand');
			dropHeldObject(); // TODO pickup after done
			return true;
		}

		// TODO allow in container transitions
		/*if (myPlayer.heldObject.id != 0) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Remove: ${myPlayer.heldObject.name} needs to be dropped!');

			removeFromContainerTarget = null;
			expectedContainer = null;

			dropHeldObject(true);

			return true;
		}*/
		if (myPlayer.isMoving()) return true;

		var distance = myPlayer.CalculateQuadDistanceToObject(target);
		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} Remove: $distance ${target.name} ${target.tx} ${target.ty}');

		if (distance > 1) {
			var name = target.name;
			var done = myPlayer.gotoObj(target);
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto container ${name} $done distance: $distance');

			if(ServerSettings.DebugAiSay){
				if (done) myPlayer.say('Goto ${name} for remove!');
				else{
					myPlayer.say('Cannot Goto ${name} for remove!');
					removeFromContainerTarget = null;
					expectedContainer = null;
					return false;
				}
			}

			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);
			if (ServerSettings.DebugAi || done == false) trace('AAI: ${myPlayer.name + myPlayer.id} Remove: drop player ${heldPlayer.name} $done');
			return true;
		}

		//myPlayer.say('remove!');
		
		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = myPlayer.remove(target.tx - myPlayer.gx, target.ty - myPlayer.gy);

		if (done) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Remove: done ${target.name} ==> ${myPlayer.heldObject}');
		} else {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Remove: failed! Ignore: ${target.name} ${target.tx} ${target.ty} ');

			this.addNotReachableObject(target);
		}

		removeFromContainerTarget = null;
		expectedContainer = null;

		return true;
	}

	public function addObjectWithHostilePath(obj:ObjectHelper) {
		if (obj == null) return;
		addHostilePath(obj.tx, obj.ty);
	}

	public function addHostilePath(tx:Int, ty:Int) {
		var index = WorldMap.world.index(tx, ty);
		objectsWithHostilePath[index] = 20; // block for 30 sec
	}

	public function isObjectWithHostilePath(tx:Int, ty:Int):Bool {
		var index = WorldMap.world.index(tx, ty);
		var notReachable = objectsWithHostilePath.exists(index);

		// if(notReachable) if(ServerSettings.DebugAi) trace('isObjectNotReachable: $notReachable $tx,$ty');

		return notReachable;
	}

	static public function AddObjBlockedByAi(obj:ObjectHelper, time:Float = 1) {
		var index = WorldMap.world.index(obj.tx, obj.ty);
		blockedByAI[index] = time;
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

		if(notReachable == false) notReachable = blockedByAI.exists(index);

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
