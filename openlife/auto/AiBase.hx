package openlife.auto;

import haxe.Exception;
import openlife.data.map.MapData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;
import openlife.data.object.player.PlayerInstance;
import openlife.data.transition.TransitionData;
import openlife.data.transition.TransitionImporter;
import openlife.macros.Macro;
import openlife.server.Biome.BiomeTag;
import openlife.server.Connection;
import openlife.server.GlobalPlayerInstance;
import openlife.server.NamingHelper;
import openlife.server.ServerAi;
import openlife.server.TimeHelper;
import openlife.server.WorldMap;
import openlife.settings.ServerSettings;
import sys.db.Mysql;
import sys.thread.Thread;

using StringTools;
using openlife.auto.AiHelper;

abstract class AiBase
{
	public static var lastTick:Float = 0;
	public static var tick:Float = 0;

	final RAD:Int = MapData.RAD; // search radius

    public var myPlayer(default, default):PlayerInterface;
    public var seqNum = 1;
	public var time:Float = 1;
	public var waitingTime:Float = 0; // if Ai is manual set to wait. Before that it is allowed to finish drop

	//public var seqNum = 1;

	var feedingPlayerTarget:PlayerInterface = null;

	var animalTarget:ObjectHelper = null;
	var escapeTarget:ObjectHelper = null;
	var foodTarget:ObjectHelper = null;
	var dropTarget:ObjectHelper = null;
	var removeFromContainerTarget:ObjectHelper = null;
	var expectedContainer:ObjectHelper = null; // needs to be set if removeFromContainerTarget is set

	var useTarget:ObjectHelper = null;
	var useActor:ObjectHelper = null; // to check if the right actor is in the hand


	var dropIsAUse:Bool = false;

	var itemToCraftId = -1;
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
        if (time > 10) time = 10; // wait max 10 sec
		if (time > 0) return;
		time += ServerSettings.AiReactionTime; // 0.5; // minimum AI reacting time

		//if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} account:  ${myPlayer.account.id}');

		cleanupBlockedObjects();

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
				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} Close Use: distance: $distance ${useTarget.description} ${useTarget.tx} ${useTarget.ty}');

				Macro.exception(if (isUsingItem()) return);
			}
		}

		// check if manual waiting time is set. For example received a STOP command
		if(waitingTime > 0){ 
			time += waitingTime;
			waitingTime = 0;
			return;
		}

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

		// should be below isUsingItem since a use can be used to drop an hold item on a pile to pickup a baby
		Macro.exception(if (isFeedingChild()) return); 
		Macro.exception(if (isPickingupFood()) return);
		Macro.exception(if (isFeedingPlayerInNeed()) return);
		Macro.exception(if (isStayingCloseToChild()) return);
		Macro.exception(if (isUsingItem()) return);
		Macro.exception(if (isRemovingFromContainer()) return);		
		Macro.exception(if (killAnimal(animal)) return);
		Macro.exception(if (isMovingToPlayer(autoStopFollow ? 10 : 5)) return); // if ordered to follow stay closer otherwise give some space to work

		if (myPlayer.isMoving()) return;
		Macro.exception(if (searchNewHomeIfNeeded()) return);
		Macro.exception(if (isPickingupCloths()) return);
		Macro.exception(if (isHandlingFire()) return);
		Macro.exception(if (handleTemperature()) return);
		Macro.exception(if (makeSharpieFood(5)) return); 
		Macro.exception(if (isHandlingGraves()) return);
		Macro.exception(if (isMakingSeeds()) return);
				
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

		Macro.exception(if(craftHighPriorityClothing()) return);

		var cravingId = myPlayer.getCraving();
		itemToCraftId = cravingId;
		if(itemToCraftId == 31) itemToCraftId = -1; // Gooseberry
		Macro.exception(if (cravingId > 0) if (craftItem(itemToCraftId)) return);

		if(myPlayer.age > 10) Macro.exception(if(craftMediumPriorityClothing()) return);
		if(myPlayer.age > 20) Macro.exception(if(craftLowPriorityClothing()) return);
		
		Macro.exception(if(makeStuff()) return);

		// if there is nothing to do go home
		Macro.exception(if(isMovingToHome(4)) return);

		// Drop held object before doing noting
		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop obj before doing nothing');
			dropHeldObject();
			return;
		}

		time += 2.5;
		
		if(myPlayer.age > ServerSettings.MinAgeToEat){
			var rand = WorldMap.calculateRandomFloat();
			if(rand < 0.05) myPlayer.say('say make xxx to give me some work!');
			else if(rand < 0.2) myPlayer.say('nothing to do...');
		}
	}

	private function isHandlingFire() : Bool {
		var firePlace = myPlayer.firePlace;
		var heldId = myPlayer.heldObject.parentId;

		// make shafts and try not to steal them // 67 Long Straight Shaft
		var shaft = AiHelper.GetClosestObjectToPosition(myPlayer.home.tx, myPlayer.home.ty, 67, 20);
		if(shaft == null) shaft = AiHelper.GetClosestObjectToPosition(myPlayer.tx, myPlayer.ty, 67, 40);
		if(shaft == null) if(craftItem(67)) return true;

		if(firePlace == null){
			firePlace = AiHelper.GetCloseFire(myPlayer);

			if(firePlace == null){
				var bestAiForFire = getBestAiForFire(myPlayer.home);
				if(bestAiForFire != null && bestAiForFire.myPlayer.id == myPlayer.id){
					//if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} Make new Fire: ${myPlayer.home.tx},${myPlayer.home.ty}');
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

		// 83 Large Fast Fire // 346 Large Slow Fire
		if(objId == 83 || objId == 346) return false;

		var bestAiForFire = getBestAiForFire(myPlayer.firePlace);

		if(bestAiForFire == null || bestAiForFire.myPlayer.id != myPlayer.id) return false;

		//if (ServerSettings.DebugAi) 
			trace('AAI: ${myPlayer.name + myPlayer.id} Checking Fire: ${firePlace.name} objAtPlace: ${objAtPlace.name} ${myPlayer.firePlace.tx},${myPlayer.firePlace.ty}');

		// 85 Hot Coals // 72 Kindling
		if(objId == 85){			
			if(heldId == 72){
				var done = useHeldObjOnTarget(firePlace);
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} t: ${TimeHelper.tick} Fire: Has Kindling Use On ==> Hot Coals!  ${firePlace.name} objAtPlace: ${objAtPlace.name} $done');
				//if(ServerSettings.DebugAiSay)
				myPlayer.say('Use Kindling on ${firePlace.name} $done'); // hot coals
				return done;
			}
			else{
				//if(ServerSettings.DebugAiSay)
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
				//if(ServerSettings.DebugAiSay)
				myPlayer.say('Use On Fire');
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Fire: Has Kindling Or Wood Use On ==> Fire');
				return useHeldObjOnTarget(firePlace);
			}
			else{
				//if(ServerSettings.DebugAiSay)
				myPlayer.say('Get Wood For Fire');
				var done = GetOrCraftItem(344);
				if(done) return true;
				else return GetOrCraftItem(72);
			}
		}

		myPlayer.firePlace = null;

		return false;
	}

	private function getBestAiForFire(fire:ObjectHelper) : AiBase{
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
	}

	public function isMakingSeeds() {
		// TODO check once every X seconds
		// TODO check at home too
		var seeds = AiHelper.GetClosestObjectById(myPlayer, 1115, null, 20);  // Dried Ear of Corn
		if(seeds == null) seeds = AiHelper.GetClosestObjectById(myPlayer, 1247, null, 20);  // Bowl with Corn Kernels
		this.hasCornSeeds = seeds != null;
		
		// TODO make seeds
		return false;
	}

	//isCaringForFire

	private function useHeldObjOnTarget(target:ObjectHelper) : Bool{
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
		var heldId = myPlayer.heldObject.parentId;
		var grave = AiHelper.GetClosestObjectById(myPlayer, 88, null, 10); // 88 Grave
		if(grave == null) grave = AiHelper.GetClosestObjectById(myPlayer, 89, null, 10); // 89 Old Grave 
		if(grave == null) return false;

		if (this.isObjectNotReachable(grave.tx, grave.ty)) return false;
		if (this.isObjectWithHostilePath(grave.tx, grave.ty)) return false;

		// cannot touch own grave
		var account = grave.getOwnerAccount();
		if(account != null && account.id == myPlayer.account.id) {
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: its my grave acountID: ${account.id}!');
			return false; 
		}
		if(grave.containedObjects.length > 0){
			if(dropHeldObject(0)){
				myPlayer.say('drop for remove from grave');
				return true;
			}
			myPlayer.say('remove from grave');
			return removeItemFromContainer(grave);
		}

		// TODO move bones
		var floorId = WorldMap.world.getFloorId(grave.tx, grave.ty);
		if(grave.objectData.groundOnly && floorId > 0) return false;

		// 850 Stone Hoe // 502 = Shovel
		if(heldId == 850 || heldId == 502){
			if(ServerSettings.DebugAiSay) myPlayer.say('dig in bones');
			if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: dig in bones');
			return useHeldObjOnTarget(grave);
		}

		if(ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GRAVE: try to get hoe or shovel');

		// 850 Stone Hoe
		if(GetOrCraftItem(850) == false){
			return GetOrCraftItem(502); // 502 = Shovel
		}

		return true;
	}

	private function handleDeath() : Bool {
		if(myPlayer.age < 59) return false;

		var rand = WorldMap.calculateRandomFloat();
		if(rand < 0.1) myPlayer.say('Good bye!');
		else if(rand < 0.2) myPlayer.say('Jasonius is calling me. Take care!');

		if(myPlayer.isMoving()) return true;
		if(isMovingToHome(5)) return true;

		time += 2;
		if(isHandlingGraves()) return true;

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} ${myPlayer.age} good bye!');
		
		return true;
	}
	
	private function handleTemperature() : Bool {
		var goodPlace = null;
		var text = '';
		var needWarming = myPlayer.isSuperCold() || (isHandlingTemperature && myPlayer.heat < 0.4);

		if(myPlayer.isSuperHot() || (isHandlingTemperature && myPlayer.heat > 0.6)){
			//trace('AAI: ${myPlayer.name + myPlayer.id} handle heat: too hot');
			goodPlace = myPlayer.coldPlace;
			text = 'cool';
		}
		else if(needWarming){
			//trace('AAI: ${myPlayer.name + myPlayer.id} handle heat: too cold');
			goodPlace = myPlayer.firePlace;
			text = 'heat at fire';			
		}

		if(goodPlace == null && needWarming){
			goodPlace = myPlayer.warmPlace;
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
				trace('AAI: ${myPlayer.name + myPlayer.id} does not help: $text heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
				myPlayer.coldPlace = null; // this place does not help
				return false; 
			} 
			if(myPlayer.heat < 0.5 && myPlayer.lastTemperature < 0.55){
				myPlayer.warmPlace = null; // this place does not help
				trace('AAI: ${myPlayer.name + myPlayer.id} does not help: $text heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
				return false; // this place does not help
			} 

			trace('AAI: ${myPlayer.name + myPlayer.id} do: $text heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature}  dist: $quadDistance wait b: ${biomeId} yv: ${myPlayer.hasYellowFever()}');
			this.time += 3; // just relax
			return true;
		}

		var done = myPlayer.gotoObj(goodPlace);
	
		if (quadDistance < 2) this.time += 4; // if you cannot reach dont try running there too often

		if(ServerSettings.DebugAiSay) myPlayer.say('going to $text');

		if (ServerSettings.DebugAi)
			trace('AAI: ${myPlayer.name + myPlayer.id} do: $text heat: ${Math.round(myPlayer.heat * 100) / 100} temp: ${temperature} dist: $quadDistance goto: $done');
			
		return done;
	}
	

	private function makeStuff() : Bool {
		/*if(myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound){

			// 2144 == Banana Peel
			if(myPlayer.heldObject.id != 2144 && isMovingToHome(5)){
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gather: go home to drop ${myPlayer.heldObject.name}');
				return true;
			}

			// Drop held object at home 
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} gather: drop obj ${myPlayer.heldObject.name}');
			dropHeldObject();
			return true;
		}*/	
		
		// TODO try only craft stuff if there for better speed
		// TODO craft only stuff if not enough at home

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} makeStuff!');

		if(fillBerryBowlIfNeeded()) return true;
		if(makePopcornIfNeeded()) return true;

		if(myPlayer.age < 10 && makeSharpieFood()) return true;

		if(craftItem(1114)) return true; // Shucked Ear of Corn
		// TODO carrot (would also make wild carrots to carrot with bowl)

		var closeSoil = AiHelper.GetClosestObjectById(myPlayer, 1138); // Fertile Soil
		if(closeSoil != null) if(craftItem(213)) return true; // Deep Tilled Row

		//if(myPlayer.age < 15 && makeFireWood()) return true;
		if(myPlayer.age < 20 && makeFireFood()) return true;

		var closePlate = AiHelper.GetClosestObjectById(myPlayer, 236); // Clay Plate
		if(closePlate == null) closePlate = AiHelper.GetClosestObjectById(myPlayer, 1602); // Stack of Clay Plates
		var hasClosePlate = closePlate != null;

		if(hasClosePlate){
			var closeDough = AiHelper.GetClosestObjectById(myPlayer, 1466); // 1466 Bowl of Leavened Dough

			if(closeDough != null && craftItem(1469)) return true; // Raw Bread Loaf

			if(craftItem(1471)) return true; // Sliced Bread
			
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
			var pies = [272, 803, 273, 274, 275, 276, 277, 278];
			var rand = WorldMap.world.randomInt(pies.length -1);

			for(i in 0...pies.length){
				var index = (rand + i) % pies.length;
				if(craftItem(pies[index])) return true;
			}

			if(craftItem(1285)) return true; // Omelette

			// TODO more wet planted stuff
			/*if(craftItem(229)) return true; // Wet Planted Wheat
			if(craftItem(1162)) return true; // Wet Planted Beans
			if(craftItem(2857)) return true; // Wet Planted Onion
			if(craftItem(2852)) return true; // Wet Planted Onions
			if(craftItem(4263)) return true; // Wet Planted Garlic
			if(craftItem(399)) return true; // Wet Planted Carrots
			if(craftItem(1142)) return true; // Wet Planted Potatoes
			if(craftItem(1110)) return true; // Wet Planted Corn Seed
			*/

			// stuff can be in more then once to increase chance
			var wetPlanted = [229, 399, 1110, 1162, 229, 399, 1110, 2857, 229, 399, 1110, 2852, 229, 399, 4263, 229, 399, 399, 229, 1142, 229, 1110, 229];
			var rand = WorldMap.world.randomInt(wetPlanted.length -1);

			for(i in 0...wetPlanted.length){
				var index = (rand + i) % wetPlanted.length;
				if(craftItem(wetPlanted[index])) return true;
			}

			if(craftItem(229)) return true; // Wet Planted Wheat		
		}
		else{
			if(craftItem(236)) return true; // Clay Plate
			// grow food that dont needs plates for processing
			if(craftItem(399)) return true; // Wet Planted Carrots
			if(craftItem(1110)) return true; // Wet Planted Corn Seed
		}
	
		if(makeFireFood()) return true;
		if(makeSharpieFood()) return true;

		if(craftItem(59)) return true; // Rope 
		//if(craftItem(58)) return true; // Thread
			
		if(craftItem(808)) return true; // Wild Onion
		if(craftItem(4252)) return true; // Wild Garlic
		
		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} nothing to make!');

		return false;
	}

	private function makeSharpieFood(maxDistance:Int = 40) : Bool {
		var heldObjId = myPlayer.heldObject.parentId;
		if(maxDistance < 15 && (heldObjId == 40 || heldObjId == 807)) dropHeldObject(0);

		var isHoldingSharpStone = myPlayer.heldObject.parentId == 34; // 34 Sharp Stone

		var obj = AiHelper.GetClosestObjectById(myPlayer, 36, null, maxDistance); // Seeding Wild Carrot
		if(obj != null && isHoldingSharpStone == false) return GetOrCraftItem(34); 
		if(obj != null && craftItem(39)) return true; // Dug Wild Carrot // 40 Wild Carrot		
		
		var obj = AiHelper.GetClosestObjectById(myPlayer, 804, null, maxDistance); // Burdock
		if(obj != null && isHoldingSharpStone == false) return GetOrCraftItem(34); 
		if(obj != null && craftItem(806)) return true; // Dug Burdock // 807 Burdock Root
		
		return false;
	}

	private function fillBerryBowlIfNeeded() : Bool {
		var heldObj = myPlayer.heldObject;

		// 253 Bowl of Gooseberries
		if(heldObj.parentId == 253 && heldObj.numberOfUses >= heldObj.objectData.numUses) return false;
		// Fill up the Bowl // 235 Clay Bowl // 253 Bowl of Gooseberries
		if(heldObj.parentId == 235 || heldObj.parentId == 253){
			// 30 Wild Gooseberry Bush
			var closeBush = AiHelper.GetClosestObjectById(myPlayer, 30);
			// 391 Domestic Gooseberry Bush
			if(closeBush == null) closeBush = AiHelper.GetClosestObjectById(myPlayer, 391);
			if(closeBush == null) return false;

			myPlayer.say('Fill Bowl on Bush');

			return useHeldObjOnTarget(closeBush);
		}

		// do nothing if there is a full Bowl of Gooseberries
		var closeBerryBowl = AiHelper.GetClosestObjectById(myPlayer, 253); // Bowl of Gooseberries
		if(closeBerryBowl != null && closeBerryBowl.numberOfUses >= closeBerryBowl.objectData.numUses) return false;
		if(closeBerryBowl != null){
			this.dropTarget = closeBerryBowl; // pick it up to fill
			this.dropIsAUse = false;

			myPlayer.say('Pickup Berry Bowl to Fill');

			return true; 
		}

		return GetOrCraftItem(235); // Clay Bowl
	}

	private function makePopcornIfNeeded() : Bool {
		// do nothing if there is Popcorn
		var closePopcorn = AiHelper.GetClosestObjectById(myPlayer, 1121); // Popcorn
		if(closePopcorn != null) return false;

		return craftItem(1121); // Popcorn
	}

	private function makeFireFood() : Bool {
		/*var mutton = AiHelper.GetClosestObjectById(myPlayer, 569); // Raw Mutton

		if(mutton != null){
			var placeToCook = AiHelper.GetClosestObjectById(myPlayer, 250); // Hot Adobe Oven
			if (placeToCook == null) placeToCook = AiHelper.GetClosestObjectById(myPlayer, 85); // Hot Coals
		}*/

		if(craftItem(570)) return true; // Cooked Mutton
		if(craftItem(197)) return true; // Cooked Rabbit
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


 	// 2886 Wooden Shoe 
 	// 2181 Straw Hat with Feather
	private function craftHighPriorityClothing() : Bool {
		// TODO consider heat / cold
		// TODO more advanced clothing
		// TODO try to look like the one you follow
		// TODO consider minuseage in crafting itself
		var objData = ObjectData.getObjectData(152); // Bow and Arrow
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
		var objData = ObjectData.getObjectData(152); // Bow and Arrow
		var color = myPlayer.getColor();
		var isWhiteOrGinger = (color == Ginger || color == White);

		// Hunting gear 874 Empty Arrow Quiver
		if(craftClothIfNeeded(874)) return true; 
		if(fillUpQuiver()) return true;

		// Chest clothing
		// 564 Mouflon Hide ==> White / Chest
		if(color == White && myPlayer.age >= objData.minPickupAge && craftClothIfNeeded(564)) return true;
		// 712 Sealskin Coat ==> Ginger
		if(color == Ginger && craftClothIfNeeded(712)) return true;
		// 711 Seal Skin ==> Ginger
		if(color == Ginger && craftClothIfNeeded(711)) return true;	
		// 202 Rabbit Fur Coat / Chest
		if(isWhiteOrGinger && craftClothIfNeeded(202)) return true;
		// 201 Rabbit Fur Shawl / Chest
		if(isWhiteOrGinger && craftClothIfNeeded(201)) return true;	

		return false;
}
	
private function craftLowPriorityClothing() : Bool {
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

		// Shoes
		// 844 Fruit Boot ==> Black
		if(color == Black && craftClothIfNeeded(844)) return true;
		// 2887 Sandal ==> Black
		if(color == Black && craftClothIfNeeded(2887)) return true;
		// 766 Snake Skin Boot ==> Black
		if(color == Black && craftClothIfNeeded(766)) return true;
		// 203 Rabbit Fur Shoe
		if(isWhiteOrGinger && craftClothIfNeeded(203)) return true;

		// Back clothing
		// 198 Backpack
		// TODO fix bug picking up backpack (AI drops item in it and then instead of picking up puts item out of it)
		//if(myPlayer.age > 25 && craftClothIfNeeded(198)) return true;

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

		if (text.startsWith("TRANS")) {
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
				myPlayer.say("Making " + obj.name);
			}
		}
	}

	public function searchFoodAndEat() {
		foodTarget = AiHelper.SearchBestFood(myPlayer);
		if(ServerSettings.DebugAiSay){
			if(foodTarget == null) myPlayer.say('No food found...');
			else myPlayer.say('new food ${foodTarget.name}');
		}
		if (ServerSettings.DebugAi && foodTarget != null) trace('AAI: ${myPlayer.name + myPlayer.id} new Foodtarget! ${foodTarget.name}');
		if (ServerSettings.DebugAi && foodTarget == null) trace('AAI: ${myPlayer.name + myPlayer.id} no new Foodtarget!!!');
	}

	//public function dropHeldObject(dropOnStart:Bool = false, maxDistanceToHome:Float = 60) {
	public function dropHeldObject(maxDistanceToHome:Float = 40) : Bool {
		// var myPlayer = myPlayer.getPlayerInstance();
		var home = myPlayer.home;
		var dropOnStart:Bool = home != null;
		var heldObjId = myPlayer.heldObject.id;

		if (heldObjId == 0) return false;
		if (myPlayer.heldObject.isWound()) return false;
		if (myPlayer.heldObject == myPlayer.hiddenWound) return false; // you cannot drop a smal wound
		if (heldObjId == 2144) dropOnStart = false; // 2144 Banana Peel
		else if (heldObjId == 34) dropOnStart = false; // 34 Sharp Stone
		else if (heldObjId == 135) dropOnStart = false; // 135 Flint Chip
		else if (heldObjId == 57) dropOnStart = false; // 57 Milkweed Stalk
		
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
		
		//if (dropOnStart && maxDistanceToHome > 0 && itemToCraft.startLocation != null) {
		if (dropOnStart && maxDistanceToHome > 0) {
			var quadMaxDistanceToHome = Math.pow(maxDistanceToHome, 2);
			var distance = myPlayer.CalculateQuadDistanceToObject(home);

			// check if not too close or too far
			if (distance > 25 && quadMaxDistanceToHome < distance) {
				var done = myPlayer.gotoObj(home); // TODO better go directly to right place at home
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} $done drop goto home $distance');

				if(done){
					if(ServerSettings.DebugAiSay) myPlayer.say('Goto home!');
					return true;
				}
				
				if(ServerSettings.DebugAiSay) myPlayer.say('Cannot Goto home!');
			}
		}

		var newDropTarget = null;
		var pileId = myPlayer.heldObject.objectData.getPileObjId();
		if(pileId > 0){
			newDropTarget = myPlayer.GetClosestObjectById(pileId, 4); 
			if(newDropTarget != null && newDropTarget.numberOfUses >= newDropTarget.objectData.numUses) newDropTarget = null;
			//if(newDropTarget != null)  trace('AAI: ${myPlayer.name + myPlayer.id} drop on pile: $pileId');
		}

		// start a new pile?
		if(newDropTarget == null && pileId > 0) newDropTarget = myPlayer.GetClosestObjectById(myPlayer.heldObject.id, 4);
		// get empty tile
		if(newDropTarget == null) newDropTarget = myPlayer.GetClosestObjectById(0);

		// dont drop on a pile if last transition removed it from similar pile // like picking a bowl from a pile to put it then back on a pile
		if(newDropTarget.id > 0 && itemToCraft.lastNewTargetId == newDropTarget.id){
			trace('AAI: ${myPlayer.name + myPlayer.id} ${newDropTarget.name} dont drop on pile where item was just taken from');
			newDropTarget = myPlayer.GetClosestObjectById(0);			
		}

		if (newDropTarget.id == 0){
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

	private function killAnimal(animal:ObjectHelper) : Bool {
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

	private function GetItem(objId:Int) : Bool {
		return GetOrCraftItem(objId, 1, true);
	}

	private function GetOrCraftItem(objId:Int, count:Int = 1, dontCraft:Bool = false) : Bool {
		if (myPlayer.isMoving()) return true;
		var objdata = ObjectData.getObjectData(objId);
		var pileId = objdata.getPileObjId();
		var hasPile = pileId > 0;
		var maxSearchDistance = 40;
		var searchDistance:Int = hasPile ? 5 : maxSearchDistance;
		var obj = AiHelper.GetClosestObjectById(myPlayer, objId, null, searchDistance);
		var pile = hasPile ? myPlayer.GetClosestObjectById(pileId) : null; 

		var usePile = pile != null && obj == null;
		if (usePile) obj = pile;
		if (obj == null && hasPile) obj = AiHelper.GetClosestObjectById(myPlayer, objId, null, maxSearchDistance);

		if (obj == null && dontCraft) return false;

		if (obj == null) return craftItem(objId, count);

		if (ServerSettings.DebugAi) 
			trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: found ${obj.name} pile: $usePile');

		if (usePile) if(dropHeldObject()) return true;

		var distance = myPlayer.CalculateQuadDistanceToObject(obj);

		if (distance > 1) {
			var done = myPlayer.gotoObj(obj);

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: done: $done goto obj $distance');
			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);

			if (ServerSettings.DebugAi || done == false)
				trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: child drop for get item ${heldPlayer.name} $done');

			return true;
		}

		// if(ServerSettings.DebugAi) trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

		// x,y is relativ to birth position, since this is the center of the universe for a player
		var done = false;
		if(usePile){
			done = myPlayer.use(obj.tx - myPlayer.gx, obj.ty - myPlayer.gy);
		}
		else done = myPlayer.drop(obj.tx - myPlayer.gx, obj.ty - myPlayer.gy);

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} GetOrCraftItem: done: $done pickup obj');

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
			trace('AAI: ${myPlayer.name + myPlayer.id} cannot feed ${targetPlayer.name} ${myPlayer.heldObject.name} fs: ${Math.round(targetPlayer.food_store*10)/10}');
			// if droped it can be stuck in a cyle if it want for example craft carrot and picks it up again. return true instead of false might also solve this
			//this.dropHeldObject(); // since food might be too big to feed
			return false;
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
	// TODO consider if object is reachable
	// TODO store transitions for crafting to have faster lookup
	// TODO consider too look for a natural spawned object with the fewest steps on the list
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

			itemToCraft.transitionsByObjectId = myPlayer.SearchTransitions(objId, ignoreHighTech);


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
			return false;
		}

		// if(player.heldObject.parentId == itemToCraft.transActor.parentId)
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
		} else {
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

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft goto actor: ${itemToCraft.transActor.id} '
				+ itemToCraft.transActor.name);

			if(ServerSettings.DebugAiSay) myPlayer.say('Goto actor ' + itemToCraft.transActor.name);

			dropTarget = itemToCraft.transActor;

			if (player.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
				if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop ${myPlayer.heldObject.name} to pickup ${itemToCraft.transActor.name}');
				dropHeldObject();
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

		
		var player = myPlayer.getPlayerInstance();
		var baseX = player.tx;
		var baseY = player.ty;
		var radius = 0;

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

			// add objects add home
			addObjectsForCrafting(myPlayer.home.tx, myPlayer.home.ty, radius, transitionsByObjectId);

			if(myPlayer.firePlace != null) addObjectsForCrafting(myPlayer.firePlace.tx, myPlayer.firePlace.ty, radius, transitionsByObjectId);

			addObjectsForCrafting(baseX, baseY, radius, transitionsByObjectId);

			// if(ServerSettings.DebugAi) trace('AI: craft: FINISHED objects ms: ${Math.round((Sys.time() - startTime) * 1000)} radius: ${itemToCraft.searchRadius}');

			searchBestTransitionTopDown(itemToCraft);

            this.time += Sys.time() - startTime;

			if (itemToCraft.transActor != null) return itemToCraft;
		}

		return itemToCraft;
	}

	private function addObjectsForCrafting(baseX:Int, baseY:Int, radius:Int, transitionsByObjectId:Map<Int, TransitionForObject>) {
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

				// check if object can be used to craft item
				var trans = transitionsByObjectId[objData.id];
				if (trans == null) continue; // object is not useful for crafting wanted object

				var steps = trans.steps;
				var obj = world.getObjectHelper(tx, ty);				
				var objQuadDistance = myPlayer.CalculateQuadDistanceToObject(obj);

				// dont use carrots if seed is needed // 400 Carrot Row
				if (obj.parentId == 400 && obj.numberOfUses < 3) continue;
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
			if (trans.targetID == -1){
				//trace('Ignore transition since target is -1 (player?): ${trans.getDesciption()}');
				continue;
			}

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
		var myPlayer = itemToCraft.ai.myPlayer;
		if (ServerSettings.DebugAiCrafting) trace('Ai: ${myPlayer.name + myPlayer.id} craft DONE items: ${itemToCraft.craftingList.length} ${objToCraft.name}: $text');
		if (ServerSettings.DebugAiCrafting) trace('Ai: ${myPlayer.name + myPlayer.id} craft DONE trans: ${itemToCraft.craftingTransitions.length} ${objToCraft.name}: $textTrans');
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

		var distance = myPlayer.CalculateQuadDistanceToObject(dropTarget);
		// var myPlayer = myPlayer.getPlayerInstance();

		if (distance > 1) {
			var done = false;
			//for (i in 0...5) {
				done = myPlayer.gotoObj(dropTarget);

				//if (done) break;

				//dropTarget = myPlayer.GetClosestObjectById(0); // empty
			//}

			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} goto drop: $done ${dropTarget.name} distance: $distance');
			if (done == false) dropTarget = null;

			return true;			
		} 		

		var done = myPlayer.drop(dropTarget.tx - myPlayer.gx, dropTarget.ty - myPlayer.gy);

		dropTarget = null;

		if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop $done ${myPlayer.heldObject.description}');		

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

		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop held object at home to pickup food');
			dropHeldObject(20);
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

		// if(ServerSettings.DebugAi) trace('${foodTarget.tx} - ${myPlayer.tx()}, ${foodTarget.ty} - ${myPlayer.ty()}');

		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} drop held object to pickup food');
			// TODO pickup up again after eating
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
			isHungry = player.food_store < player.food_store_max * 0.8;
		} else {
			isHungry = player.food_store < Math.max(3, player.food_store_max * 0.3);
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
		if (myPlayer.isStillExpectedItem(useTarget) == false) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} Use target changed meanwhile! ${useTarget.name}');
			useTarget = null;
			return false;
		}
		// only allow to go on with use if right actor is in the hand, or if actor will be empty
		// if(myPlayer.heldObject.id != useActor.id && useActor.id != 0)
		if (myPlayer.heldObject.parentId != useActor.parentId) {
			if (ServerSettings.DebugAi)
				trace('AAI: ${myPlayer.name + myPlayer.id} Use: not the right actor! ${myPlayer.heldObject.name} expected: ${useActor.name}');

			useTarget = null;
			useActor = null;
			// dropTarget = itemToCraft.transActor;

			dropHeldObject();

			return false;
		}
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
		if (myPlayer.heldObject.id != 0 && myPlayer.heldObject != myPlayer.hiddenWound && useActor.id == 0) {
			if (ServerSettings.DebugAi) trace('AAI: ${myPlayer.name + myPlayer.id} craft: drop obj to to have empty hand');
			dropHeldObject();
			return true;
		}
		
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
					|| taregtObjectId == itemToCraft.itemToCraft.parentId) itemToCraft.countDone += 1;

				if (ServerSettings.DebugAi)
					trace('AAI: ${myPlayer.name + myPlayer.id} done: ${useTarget.name} ==> ${itemToCraft.itemToCraft.name} trans: ${itemToCraft.countTransitionsDone} finished: ${itemToCraft.countDone} FROM: ${itemToCraft.count}');
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

			//if(ServerSettings.DebugAiSay){
				if (done) myPlayer.say('Goto ${name} for remove!');
				else{
					myPlayer.say('Cannot Goto ${name} for remove!');
					removeFromContainerTarget = null;
					expectedContainer = null;
					return false;
				}
			//}

			return true;
		}

		var heldPlayer = myPlayer.getHeldPlayer();
		if (heldPlayer != null) {
			var done = myPlayer.dropPlayer(myPlayer.x, myPlayer.y);
			if (ServerSettings.DebugAi || done == false) trace('AAI: ${myPlayer.name + myPlayer.id} Remove: drop player ${heldPlayer.name} $done');
			return true;
		}

		myPlayer.say('remove!');
		
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
