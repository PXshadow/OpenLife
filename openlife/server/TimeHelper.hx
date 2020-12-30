package openlife.server;

import openlife.data.object.ObjectData;
import openlife.server.WorldMap.BiomeTag;
import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;

class TimeHelper
{
    private static var tickTime = 1 / 20;

    //private static var TimeHelper = new TimeHelper();

    public static var tick:Float = 0;       // some are skipped if server is too slow
    //public static var allTicks:Float = 0;   // these ticks will not be skipped, but can be slower then expected

    private static var lastTick:Float = 0;
    private static var serverStartingTime:Float;

    // TODO currently not needed, since for all objects on the map every second all time objects are generated
    //public var timeObjectHelpers:Array<ObjectHelper>; 
    private static var worldMapTimeStep = 0; // counts the time steps for doing map time stuff, since some ticks may be skiped because of server too slow

    //private function new(){}

    public static function CalculateTimeSinceTicksInSec(ticks:Float):Float
    {
        return (TimeHelper.tick - ticks) * TimeHelper.tickTime;
    }

    public static function DoTimeLoop()
    {
        serverStartingTime = Sys.time();
        var averageSleepTime = 0.0;
        var skipedTicks = 0;
        var timeSinceStartCountedFromTicks:Float = TimeHelper.tick * TimeHelper.tickTime;
        serverStartingTime -= timeSinceStartCountedFromTicks;  // pretend the server was started before to be aligned with ticks

        //trace('Server Startign time: sys.time: ${Sys.time()} serverStartingTime: $serverStartingTime timeSinceStartCountedFromTicks: $timeSinceStartCountedFromTicks');

        while (true)
        {
            TimeHelper.tick = Std.int(TimeHelper.tick + 1);

            var timeSinceStart:Float = Sys.time() - TimeHelper.serverStartingTime;
            timeSinceStartCountedFromTicks = TimeHelper.tick * TimeHelper.tickTime;
            

            // TODO what to do if server is too slow?
            if(TimeHelper.tick % 10 != 0  && timeSinceStartCountedFromTicks < timeSinceStart)
            {
                TimeHelper.tick = Std.int(TimeHelper.tick + 1);
                skipedTicks++;
            }
            if(TimeHelper.tick % 200 == 0)
            {
                averageSleepTime = Math.ceil(averageSleepTime / 200 * 1000) / 1000;
                //trace('Ticks: ${TimeHelper.tick}');
                trace('Connections: ${Server.server.connections.length} Time Counted From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Aaverage Sleep Time: $averageSleepTime');
                averageSleepTime = 0;
                skipedTicks = 0;

                //if(Server.server.connections.length > 0) Server.server.connections[0].player.doDeath();
            }

            @:privateAccess haxe.MainLoop.tick();

            Server.server.map.mutex.acquire(); 

            if(ServerSettings.debug)
            {
                @:privateAccess TimeHelper.DoTimeStuff();
            }
            else
            {
                try
                {
                    
                    @:privateAccess TimeHelper.DoTimeStuff();
                }
                catch(ex)
                {
                    trace(ex);   
                }
            }

            Server.server.map.mutex.release();


            timeSinceStart = Sys.time() - TimeHelper.serverStartingTime;

            if(timeSinceStartCountedFromTicks > timeSinceStart)
            {
                var sleepTime = timeSinceStartCountedFromTicks - timeSinceStart;
                averageSleepTime += sleepTime;

                //trace('sleep: ${sleepTime}');
                Sys.sleep(sleepTime);
            }
        }
    }

    public static function DoTimeStuff()
    {
        var timePassedInSeconds = CalculateTimeSinceTicksInSec(lastTick);

        TimeHelper.lastTick = tick;  

        for (connection in Server.server.connections)
        {
            updateAge(connection, timePassedInSeconds);

            updateFood(connection, timePassedInSeconds);

            MoveHelper.updateMovement(connection.player);
        }

        DoWorldMapTimeStuff(); // TODO currently it goes through the hole map each sec / this may later not work

        RespawnObjects();

        DecayObjects();

        var worldMap = Server.server.map; 
 
        // make sure they are not all at same tick!
        if((tick + 20) % ServerSettings.TicksBetweenSaving  == 0) worldMap.updateObjectCounts();
        if(ServerSettings.debug == false && tick % ServerSettings.TicksBetweenSaving == 0) Server.server.map.writeToDisk(false);
        if(ServerSettings.debug == false && (tick + 60) % ServerSettings.TicksBetweenBackups == Math.ceil(ServerSettings.TicksBetweenBackups / 2)) Server.server.map.writeBackup();
    }

    private static function updateAge(c:Connection, timePassedInSeconds:Float)
    {
        var tmpAge = c.player.age;
        var aging = timePassedInSeconds / c.player.age_r;

        var healthFactor = c.player.CalculateHealthFactor(false);
        var agingFactor:Float = 1;        

        if(c.player.age < ServerSettings.GrownUpAge)
        {
            agingFactor = healthFactor;
        }
        else
        {
            agingFactor = 1 / healthFactor;
        }

        if(c.player.food_store < 0)
        {
            if(c.player.age < ServerSettings.GrownUpAge)
            {
                agingFactor *= ServerSettings.AgingFactorWhileStarvingToDeath;
            } 
            else
            {
                agingFactor *= 1 / ServerSettings.AgingFactorWhileStarvingToDeath;
            }
        }

        c.player.age_r = ServerSettings.AgingSecondsPerYear * agingFactor;

        c.player.trueAge += aging;

        aging *= agingFactor;

        c.player.age += aging;
        
        if(Std.int(tmpAge) != Std.int(c.player.age))
        {
            trace('Age: ${c.player.age} TrueAge: ${c.player.trueAge} agingFactor: $agingFactor healthFactor: $healthFactor');

            c.player.food_store_max = CalculateFoodStoreMax(c.player);

            if(c.player.age > 60) c.player.doDeath('reason_age');

            //trace('update age: ${c.player.age} food_store_max: ${c.player.food_store_max}');
            c.player.sendFoodUpdate(false);

            Connection.SendUpdateToAllClosePlayers(c.player, false);
        }
    }

    public static function CalculateFoodStoreMax(p:GlobalPlayerInstance) : Float
    {
        var age = p.age;
        var food_store_max:Float = ServerSettings.GrownUpFoodStoreMax;

        if(age < 30) food_store_max = ServerSettings.NewBornFoodStoreMax + age / 30 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.NewBornFoodStoreMax);
        if(age > 50) food_store_max = ServerSettings.OldAgeFoodStoreMax + (60 - age) / 10 * (ServerSettings.GrownUpFoodStoreMax - ServerSettings.OldAgeFoodStoreMax);

        if(p.food_store < 0) food_store_max += ServerSettings.FoodStoreMaxReductionWhileStarvingToDeath * p.food_store;

        return food_store_max;
    }

    private static function updateFood(c:Connection, timePassedInSeconds:Float)
    {
        //trace('food_store: ${connection.player.food_store}');

        var tmpFood = Math.ceil(c.player.food_store);
        var tmpExtraFood = Math.ceil(c.player.yum_bonus);
        var tmpFoodStoreMax = Math.ceil(c.player.food_store_max);
        var foodDecay = timePassedInSeconds * ServerSettings.FoodUsePerSecond; 

        if(c.player.age < ServerSettings.GrownUpAge && c.player.food_store > 0) foodDecay *= ServerSettings.IncreasedFoodNeedForChildren;

        // if starving to death and there is some health left, reduce food need and heath
        if(c.player.food_store < 0 && c.player.yum_multiplier > 0)
        {
            c.player.yum_multiplier -= foodDecay;
            foodDecay /= 2;
        }

        if(c.player.yum_bonus > 0)
        {
            c.player.yum_bonus -= foodDecay;
        }
        else
        {
            c.player.food_store -= foodDecay;
        }

        c.player.food_store_max = CalculateFoodStoreMax(c.player);

        var hasChanged = tmpFood != Math.ceil(c.player.food_store) || tmpExtraFood != Math.ceil(c.player.yum_bonus);
        hasChanged = hasChanged || tmpFoodStoreMax != Math.ceil(c.player.food_store_max);

        if(hasChanged)
        {
            c.player.sendFoodUpdate(false);
            c.send(FRAME, null, false);

            if(c.player.food_store_max < ServerSettings.DeathWithFoodStoreMax)
            {
                c.player.doDeath('reason_hunger');

                Connection.SendUpdateToAllClosePlayers(c.player, false);
            }
        }
    }

    public static function DoWorldMapTimeStuff()
    {
        // devide in X steps
        var timeParts = ServerSettings.WorldTimeParts; 
        var worldMap = Server.server.map;

        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        worldMapTimeStep++;

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var obj = worldMap.getObjectId(x,y);

                if(obj[0] == 0) continue;      
                
                //RespawnPlant(obj[0]);

                var helper = worldMap.getObjectHelper(x,y,true); 

                if(helper != null)
                {              
                    // clear up not needed ObjectHelpers to save space
                    if(worldMap.deleteObjectHelperIfUseless(helper)) continue;

                    if(helper.timeToChange == 0) continue;

                    var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(helper.creationTimeInTicks);
                    var timeToChange = helper.timeToChange;

                    if(passedTime >= timeToChange)
                    {
                        //trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${timeToChange}');       

                        // TODO maybe better not delete by default...
                        worldMap.setObjectHelperNull(x,y);
                        
                        TimeHelper.doTimeTransition(helper);
                    }

                    continue;
                }

                var timeTransition = Server.transitionImporter.getTransition(-1, obj[0], false, false);
                if(timeTransition == null) continue;

                helper = worldMap.getObjectHelper(x,y); 
                helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);

                worldMap.setObjectHelper(x,y,helper);

                //trace('TIME: ${helper.objectData.description} neededTime: ${timeToChange}');  
                
                //var testObj = getObjectId(helper.tx, helper.ty);

                //trace('testObj: $testObj obj: $obj ${helper.tx},${helper.ty} i:$i index:${index(helper.tx, helper.ty)}');
            }
        }
    }  

    public static function RespawnObjects()
    {
        var timeParts = ServerSettings.WorldTimeParts * 10; 
        var worldMap = Server.server.map;
        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var obj = worldMap.getOriginalObjectId(x,y)[0];
                
                if(obj == 0) continue;

                if(ServerSettings.CanObjectRespawn(obj) == false) continue;

                if(worldMap.currentObjectsCount[obj] >= worldMap.originalObjectsCount[obj]) continue;

                if(worldMap.randomFloat() > ServerSettings.ObjRespawnChance) continue;

                var dist = 6;
                var tmpX = worldMap.randomInt(2*dist) - dist + x;
                var tmpY = worldMap.randomInt(2*dist) - dist + y;

                if(worldMap.getObjectId(tmpX, tmpY)[0] != 0) continue;

                if(worldMap.getObjectId(tmpX, tmpY-1)[0] != 0) continue; // make sure that obj does not spawn above one tile of existing obj

                var biomeId = worldMap.getBiomeId(tmpX,tmpY);
                var objData = ObjectData.getObjectData(obj);

                if(objData.isSpawningIn(biomeId) == false) continue;

                worldMap.setObjectId(tmpX, tmpY, [obj]);

                worldMap.currentObjectsCount[obj]++;

                Connection.SendMapUpdateToAllClosePlayers(tmpX, tmpY, [obj]);

                //trace('respawn object: ${objData.description} $obj');
            }
        }    
    }

    public static function DecayObjects()
    {
        // TODO decay stuff in containers
        // TODO decay stuff with number of uses > 1
        // TODO create custom decay transitions

        var timeParts = ServerSettings.WorldTimeParts * 10; 
        var worldMap = Server.server.map;
        var partSizeY = Std.int(worldMap.height / timeParts);
        var startY = (worldMapTimeStep % timeParts) * partSizeY;
        var endY = startY + partSizeY;

        //trace('startY: $startY endY: $endY worldMap.height: ${worldMap.height}');

        for (y in startY...endY)
        {
            for(x in 0...worldMap.width)
            {
                var obj = worldMap.getObjectId(x,y)[0];
                
                if(obj == 0) continue;

                if(ServerSettings.CanObjectRespawn(obj) == false) continue;

                var objectHelper = worldMap.getObjectHelper(x,y, true);

                if(objectHelper != null && objectHelper.containedObjects.length > 0) continue; 

                //if(worldMap.currentObjectsCount[obj] >= worldMap.originalObjectsCount[obj]) continue;

                var objData = ObjectData.getObjectData(obj);
                
                if(objData.decayFactor <= 0) continue;       

                var decayChance = ServerSettings.ObjDecayChance * objData.decayFactor;

                if(objData.foodValue > 0) decayChance *= ServerSettings.ObjDecayFactorForFood;

                if(worldMap.getFloorId(x,y) != 0) decayChance *= ServerSettings.ObjDecayFactorOnFloor;

                if(worldMap.randomFloat() > decayChance) continue;

                //if(objData.isSpawningIn(biomeId) == false) continue;

                worldMap.setObjectId(x,y, [0]);
                //worldMap.setObjectHelperNull(x, y);

                worldMap.currentObjectsCount[obj]--;

                Connection.SendMapUpdateToAllClosePlayers(x, y, [0]);

                //trace('decay object: ${objData.description} $obj');
            }
        }    
    }

    public static function RespawnPlant()
    {
        // TODO
    }

    public static function RespawnObj()
    {
    }

    public static function doTimeTransition(helper:ObjectHelper)
    {
        // TODO test time transition for maxUseTaget like Goose Pond:
        // -1 + 142 = 0 + 142
        // -1 + 142 = 0 + 141

        var tx = helper.tx;
        var ty = helper.ty;

        var tileObject = Server.server.map.getObjectId(tx, ty);
        var floorId = Server.server.map.getFloorId(tx, ty);

        //trace('Time: tileObject: $tileObject');

        var transition = Server.transitionImporter.getTransition(-1, tileObject[0], false, false);

        if(transition == null)
        {
            trace('WARNING: Time: no transtion found! Maybe object was moved? tile: $tileObject helper: ${helper.id} ${helper.description()}');
            return;
        }

        if(doAnimalMovement(helper, transition)) return;

        helper.id = transition.newTargetID;
        helper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(helper);
        helper.creationTimeInTicks = TimeHelper.tick;

        TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, transition, true);

        Server.server.map.setObjectHelper(tx, ty, helper);
        
        var newTileObject = helper.toArray();

        for (c in Server.server.connections)
        {      
            var player = c.player;
            
            // since player has relative coordinates, transform them for player
            var x = tx - player.gx;
            var y = ty - player.gy;

            // update only close players
            if(player.isClose(x,y, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

            c.sendMapUpdate(x, y, floorId, newTileObject, -1, false);
            c.send(FRAME, null, false);
        }
    } 

    /*
        MX
        x y new_floor_id new_id p_id old_x old_y speed

        Optionally, a line can contain old_x, old_y, and speed.
        This indicates that the object came from the old coordinates and is moving
        with a given speed.
    */

    private static function doAnimalMovement(helper:ObjectHelper, timeTransition:TransitionData) : Bool
    {
        // TODO use chance for movement
        // TODO LAST movement
        // TODO collision detection if animal is deadly

        var moveDist = timeTransition.move;

        if(moveDist <= 0) return false;

        moveDist += 1; // movement distance is plus 4 in original code if walking over objects

        var worldmap = Server.server.map;

        var fromTx = helper.tx;
        var fromTy = helper.ty;

        for (i in 0...20)
        {
            var toTx = helper.tx - moveDist + worldmap.randomInt(moveDist * 2);
            var toTy = helper.ty - moveDist + worldmap.randomInt(moveDist * 2);
            
            var target = worldmap.getObjectHelper(toTx, toTy);
            //var obj = worldmap.getObjectId(tx, ty);
            //var objData = Server.objectDataMap[obj[0]];

            if(target.id != 0) continue;

            if(target.blocksWalking()) continue;
            // dont move move on top of other moving stuff
            if(target.timeToChange != 0) continue;  
            if(target.groundObject != null) continue;
            // make sure that target is not the old tile
            if(toTx == helper.tx && toTy == helper.ty) continue;
            
            var targetBiome = worldmap.getBiomeId(toTx,toTy);
            if(targetBiome == BiomeTag.SNOWINGREY) continue;
            if(targetBiome == BiomeTag.OCEAN) continue;

            var isPreferredBiome = helper.objectData.isSpawningIn(targetBiome);

            // lower the chances even more if on river
            //var isHardbiome = targetBiome == BiomeTag.RIVER || (targetBiome == BiomeTag.GREY) || (targetBiome == BiomeTag.SNOW) || (targetBiome == BiomeTag.DESERT);
            var isNotHardbiome =  isPreferredBiome || targetBiome == BiomeTag.GREEN || targetBiome == BiomeTag.YELLOW;

            var chancePreferredBiome = isNotHardbiome ? ServerSettings.chancePreferredBiome : (ServerSettings.chancePreferredBiome + 4) / 5;

            //trace('chance: $chancePreferredBiome isNotHardbiome: $isNotHardbiome biome: $targetBiome');

            // skip with chancePreferredBiome if this biome is not preferred
            if(isPreferredBiome == false && i < Math.round(chancePreferredBiome * 10) &&  worldmap.randomFloat() <= chancePreferredBiome) continue;

            // limit movement if blocked
            target = calculateNonBlockedTarget(fromTx, fromTy, target);

            if(target == null) continue; // movement was fully bocked, search another target

            toTx = target.tx;
            toTy = target.ty;
    
            // save what was on the ground, so that we can move on this tile and later restore it
            var oldTileObject = helper.groundObject == null ? [0]: helper.groundObject.toArray();
            var newTileObject = helper.toArray();

            //var des = helper.groundObject == null ? "NONE": helper.groundObject.description();
            //trace('MOVE: oldTile: $oldTileObject $des newTile: $newTileObject ${helper.description()}');

            // TODO only change after movement is finished
            helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
            helper.creationTimeInTicks = TimeHelper.tick;

            worldmap.setObjectHelper(fromTx, fromTy, helper.groundObject);
            //worldmap.setObjectId(fromTx,fromTy, oldTileObject); // TODO move to setter

            var tmpGroundObject = helper.groundObject;
            helper.groundObject = target;
            worldmap.setObjectHelper(toTx, toTy, helper);

            var chanceForOffspring = isPreferredBiome ? ServerSettings.chanceForOffspring : ServerSettings.chanceForOffspring * Math.pow((1 - chancePreferredBiome), 2);

            // give extra birth chance bonus if population is very low
            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] / 2) chanceForOffspring *=5;

            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] * ServerSettings.maxOffspringFactor && worldmap.randomFloat() <= chanceForOffspring)
            {
                // TODO consider dead 
                worldmap.currentObjectsCount[newTileObject[0]] += 1;

                //if(chanceForOffspring < worldmap.chanceForOffspring) trace('NEW: $newTileObject ${helper.description()}: ${worldmap.currentPopulation[newTileObject[0]]} ${worldmap.initialPopulation[newTileObject[0]]} chance: $chanceForOffspring biome: $targetBiome');

                oldTileObject = newTileObject;
                
                var newAnimal = ObjectHelper.readObjectHelper(null, newTileObject);
                newAnimal.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
                newAnimal.groundObject = tmpGroundObject;
                worldmap.setObjectHelper(fromTx, fromTy, newAnimal);
                //worldmap.setObjectId(x,y, newTileObject); // TODO move to setter
            }
            
            var floorIdTarget = Server.server.map.getFloorId(toTx, toTy);
            var floorIdFrom = Server.server.map.getFloorId(fromTx, fromTy);

            var speed = ServerSettings.InitialPlayerMoveSpeed * helper.objectData.speedMult;

            for (c in Server.server.connections) 
            {            
                var player = c.player;
                
                // since player has relative coordinates, transform them for player
                var fromX = fromTx - player.gx;
                var fromY = fromTy - player.gy;
                var toX = toTx - player.gx;
                var toY = toTy - player.gy;

                // update only close players
                if(player.isClose(toX,toY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

                c.sendMapUpdateForMoving(toX, toY, floorIdTarget, newTileObject, -1, fromX, fromY, speed);
                c.sendMapUpdate(fromX, fromY, floorIdFrom, oldTileObject, -1, false);
                //c.sendMapUpdate(fromX, fromY, floorId, [0], -1);
                c.send(FRAME, null, false);
            }

            return true;
        }

        helper.creationTimeInTicks = TimeHelper.tick;

        return false;
    }    

    private static function calculateNonBlockedTarget(fromX:Int, fromY:Int, toTarget:ObjectHelper) : ObjectHelper
    {
        var tx = toTarget.tx;
        var ty = toTarget.ty;
        var tmpX = fromX;
        var tmpY = fromY;
        var tmpTarget = null;

        for(ii in 0...10)
        {                
            if(tmpX == tx && tmpY == ty) break;

            if(tx > tmpX)  tmpX += 1;
            else if(tx < tmpX)  tmpX -= 1;

            if(ty > tmpY)  tmpY += 1;
            else if(ty < tmpY)  tmpY -= 1;

            //trace('movement: $tmpX,$tmpY');

            var movementTileObj = WorldMap.worldGetObjectHelper(tmpX , tmpY); 
            //var movementBiome = WorldMap.worldGetBiomeId(tmpX , tmpY);  

            //var cannotMoveInBiome = movementBiome == BiomeTag.OCEAN ||  movementBiome == BiomeTag.SNOWINGREY;

            var isBiomeBlocking = WorldMap.isBiomeBlocking(tmpX, tmpY);

            if(isBiomeBlocking && ServerSettings.ChanceThatAnimalsCanPassBlockingBiome > 0) isBiomeBlocking = WorldMap.calculateRandomFloat() > ServerSettings.ChanceThatAnimalsCanPassBlockingBiome; 

            // TODO better patch in the objects, i dont see any reason why a rabit or a tree should block movement
            if(isBiomeBlocking || (movementTileObj.blocksWalking() 
                    && movementTileObj.description().indexOf("Tarry Spot") == -1
                    && movementTileObj.description().indexOf("Tree") == -1 && movementTileObj.description().indexOf("Rabbit") == -1  
                    && movementTileObj.description().indexOf("Iron") == -1 && movementTileObj.description().indexOf("Spring") == -1
                    && movementTileObj.description().indexOf("Sugarcane") == -1 && movementTileObj.description().indexOf("Pond") == -1
                    && movementTileObj.description().indexOf("Palm") == -1  && movementTileObj.description().indexOf("Plant") == -1))
            {
                //trace('movement blocked ${movementTile.description()} ${movementBiome}');
                break;
            }
            

            // TODO allow move on non empty ground
            if(movementTileObj.id == 0) tmpTarget = movementTileObj;
        }
   
        return tmpTarget;
    }
}