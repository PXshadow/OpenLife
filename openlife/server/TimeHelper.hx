package openlife.server;

import openlife.client.ClientTag;
import openlife.data.object.ObjectData;
import openlife.server.WorldMap.BiomeTag;
import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectHelper;
import openlife.settings.ServerSettings;

class TimeHelper
{
    private static var tickTime = 1 / 20;
    public static var tick:Float = 0;       // some are skipped if server is too slow
    //public static var allTicks:Float = 0;   // these ticks will not be skipped, but can be slower then expected

    public static var lastTick:Float = 0;
    private static var serverStartingTime:Float;

    private static var worldMapTimeStep = 0; // counts the time steps for doing map time stuff, since some ticks may be skiped because of server too slow

    public static function CalculateTimeSinceTicksInSec(ticks:Float):Float
    {
        return (TimeHelper.tick - ticks) * TimeHelper.tickTime;
    }

    public static function DoTimeLoop()
    {
        serverStartingTime = Sys.time();
        var averageSleepTime:Float = 0.0;
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
                trace('Connections: ${Connection.getConnections().length} AIs: ${Connection.getAis().length} Time Counted From Ticks: ${timeSinceStartCountedFromTicks} Time: ${Math.ceil(timeSinceStart)} Skiped Ticks: $skipedTicks Average Sleep Time: $averageSleepTime');
                averageSleepTime = 0;
                skipedTicks = 0;

                ServerSettings.readFromFile(false);

                //if(Server.server.connections.length > 0) Server.server.connections[0].player.doDeath();
            }

            @:privateAccess haxe.MainLoop.tick();

            //Server.server.map.mutex.acquire(); 

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
                    trace('WARNING' + ex);   
                }
            }

            //Server.server.map.mutex.release();


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

        for (c in Connection.getConnections())
        {            
            updateAge(c.player, timePassedInSeconds);

            updateFoodAndDoHealing(c.player, timePassedInSeconds);            

            MoveHelper.updateMovement(c.player);

            if(TimeHelper.tick % 40 == 0) updateTemperature(c.player);
        }
        
        
        for (ai in Connection.getAis())
        {
            updateAge(ai.me, timePassedInSeconds);

            updateFoodAndDoHealing(ai.me, timePassedInSeconds);            

            MoveHelper.updateMovement(ai.me);

            //if(TimeHelper.tick % 40 == 0) updateTemperature(ai.me);

            ai.doTimeStuff(timePassedInSeconds);
        }


        DoWorldMapTimeStuff(); // TODO currently it goes through the hole map each sec / this may later not work

        RespawnObjects();

        DecayObjects();

        var worldMap = Server.server.map; 
 
        // make sure they are not all at same tick!
        if((tick + 20) % ServerSettings.TicksBetweenSaving  == 0) worldMap.updateObjectCounts();
        if(ServerSettings.debug == false && tick % ServerSettings.TicksBetweenSaving == 0) Server.server.map.writeToDisk(false);
        if(ServerSettings.debug == false && (tick + 60) % ServerSettings.TicksBetweenBackups == Math.ceil(ServerSettings.TicksBetweenBackups / 2)) Server.server.map.writeBackup();

        /*
        if(tick % 100 == 0) 
        {
            if(Connection.getConnections().length > 0)
            {
                var c = Connection.getConnections()[0];
                var obj = ObjectData.personObjectData[personIndex];
                c.player.po_id = obj.id;
                
                Connection.SendUpdateToAllClosePlayers(c.player);
                if(tick % 200 == 0) c.send(ClientTag.DYING, ['${c.player.p_id}']);
                else c.send(ClientTag.HEALED, ['${c.player.p_id}']);

                c.sendGlobalMessage('Id ${obj.parentId} P${obj.person} ${obj.description}');

                personIndex++;
                //  418 + 0 = 427 + 1363 / @ Deadly Wolf + Empty  -->  Attacking Wolf + Bite Wound 
            }
        }
        */
    }

    static var personIndex = 0;

    private static function updateAge(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        var tmpAge = player.age;
        var aging = timePassedInSeconds / player.age_r;

        //trace('aging: ${aging}');

        //trace('player.age_r: ${player.age_r}');

        var healthFactor = player.CalculateHealthFactor(false);
        var agingFactor:Float = 1;    
        
        //trace('healthFactor: ${healthFactor}');

        if(player.age < ServerSettings.GrownUpAge)
        {
            agingFactor = healthFactor;
        }
        else
        {
            agingFactor = 1 / healthFactor;
        }

        if(player.food_store < 0)
        {
            if(player.age < ServerSettings.GrownUpAge)
            {
                agingFactor *= ServerSettings.AgingFactorWhileStarvingToDeath;
            } 
            else
            {
                agingFactor *= 1 / ServerSettings.AgingFactorWhileStarvingToDeath;
            }
        }

        player.age_r = ServerSettings.AgingSecondsPerYear * agingFactor;

        player.trueAge += aging;

        aging *= agingFactor;

        player.age += aging;
        
        if(Std.int(tmpAge) != Std.int(player.age))
        {
            trace('Age: ${player.age} TrueAge: ${player.trueAge} agingFactor: $agingFactor healthFactor: $healthFactor');

            player.food_store_max = player.calculateFoodStoreMax();

            if(player.age > 60) player.doDeath('reason_age');

            //trace('update age: ${player.age} food_store_max: ${player.food_store_max}');
            player.sendFoodUpdate(false);

            Connection.SendUpdateToAllClosePlayers(player, false);
        }
    }

    private static function updateFoodAndDoHealing(player:GlobalPlayerInstance, timePassedInSeconds:Float)
    {
        //trace('food_store: ${connection.player.food_store}');

        var tmpFood = Math.ceil(player.food_store);
        var tmpExtraFood = Math.ceil(player.yum_bonus);
        var tmpFoodStoreMax = Math.ceil(player.food_store_max);
        var foodDecay = timePassedInSeconds * ServerSettings.FoodUsePerSecond; 

        if(player.age < ServerSettings.GrownUpAge && player.food_store > 0) foodDecay *= ServerSettings.IncreasedFoodNeedForChildren;

        // do healing but increase food use
        if(player.hits > 0)
        {
            player.hits -= foodDecay;

            foodDecay *= 2;

            if(player.hits < 0) player.hits = 0; 

            if(player.woundedBy != 0 && player.hits < 1)
            {
                player.woundedBy = 0;
                if(player.connection != null) player.connection.send(ClientTag.HEALED, ['${player.p_id}']);
            }
        }

        foodDecay /= calculateTemperature(player);

        // if starving to death and there is some health left, reduce food need and heath
        if(player.food_store < 0 && player.yum_multiplier > 0)
        {
            player.yum_multiplier -= foodDecay;
            foodDecay /= 2;
        }

        if(player.yum_bonus > 0)
        {
            player.yum_bonus -= foodDecay;
        }
        else
        {
            player.food_store -= foodDecay;
        }

        player.food_store_max = player.calculateFoodStoreMax();

        var hasChanged = tmpFood != Math.ceil(player.food_store) || tmpExtraFood != Math.ceil(player.yum_bonus);
        hasChanged = hasChanged || tmpFoodStoreMax != Math.ceil(player.food_store_max);

        if(hasChanged)
        {
            player.sendFoodUpdate(false);
            if(player.connection != null) player.connection.send(FRAME, null, false);

            if(player.food_store_max < ServerSettings.DeathWithFoodStoreMax)
            {
                var reason = player.woundedBy == 0 ? 'reason_hunger': 'reason_killed_${player.woundedBy}';

                player.doDeath(reason);

                Connection.SendUpdateToAllClosePlayers(player, false);
            }
        }
    }

    private static function calculateTemperature(player:GlobalPlayerInstance) : Float
    {
        var biome = WorldMap.worldGetBiomeId(player.tx(), player.ty()); // TODO other biomes

        var biomeValue = biome == BiomeTag.JUNGLE ? 1.5 : 1; 

        var temperature = (biomeValue + player.calculateClothingInsulation()) ; // clothing insulation can be between 0 and 2 for now

        return temperature;
    }

    /*
        HX
        heat food_time indoor_bonus#

        Tells player about their current heat value, food drain time, and indoor bonus.

        Food drain time and indoor bonus are in seconds.

        Food drain time is total including bonus.
    */

    private static function updateTemperature(player:GlobalPlayerInstance)
    {
        var maxTemerature = 2.5; // TODO change

        var temperature = calculateTemperature(player);

        var foodDrainTime = (1 / ServerSettings.FoodUsePerSecond) * temperature;

        var heat = ((temperature - 1) / ((temperature - 1) + (maxTemerature -1))) / 2; // TODO change

        heat = Math.round(heat * 100) / 100;

        foodDrainTime = Math.round(foodDrainTime * 100) / 100;

        var message = '$heat $foodDrainTime 0';

        player.heat = heat;

        if(player.connection != null) player.connection.send(HEAT_CHANGE, [message]);

        // trace('Temerature update: temperature: $temperature mesage: $message');
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
                    if(helper.timeToChange == 0) // maybe timeToChange was forgotten to be set
                    {
                        var timeTransition = Server.transitionImporter.getTransition(-1, obj[0], false, false);

                        if(timeTransition == null) continue;

                        trace('WARNING: found helper without time transition: ${helper.description}');

                        helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
                    }

                    // clear up not needed ObjectHelpers to save space
                    if(worldMap.deleteObjectHelperIfUseless(helper)) continue; // uses worlmap mutex

                    var passedTime = TimeHelper.CalculateTimeSinceTicksInSec(helper.creationTimeInTicks);
                    var timeToChange = helper.timeToChange;

                    if(passedTime >= timeToChange)
                    {
                        //trace('TIME: ${helper.objectData.description} passedTime: $passedTime neededTime: ${timeToChange}');       

                        // TODO maybe better not delete by default...
                        // worldMap.setObjectHelperNull(x,y);
                        
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

        WorldMap.world.mutex.acquire();
        
        // just to be sure, that no other thread changed object meanwhile 
        
        if(helper != WorldMap.world.getObjectHelper(helper.tx, helper.ty))
        {
            WorldMap.world.mutex.release();

            trace("TIME: some one changed helper meanwhile");

            return;
        } 

        var sendUpdate = false;

        try
        {
            sendUpdate = doTimeTransitionHelper(helper);
        }
        catch(ex)
        {
            trace(ex);
        }

        WorldMap.world.mutex.release();

        if(sendUpdate == false) return;

        var newTileObject = helper.toArray();

        Connection.SendMapUpdateToAllClosePlayers(helper.tx, helper.ty, newTileObject);

        /*for (c in Server.server.connections)
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
        */
    } 

    public static function doTimeTransitionHelper(helper:ObjectHelper) : Bool
    {
        var tx = helper.tx;
        var ty = helper.ty;

        var tileObject = Server.server.map.getObjectId(tx, ty);

        //trace('Time: tileObject: $tileObject');

        var transition = Server.transitionImporter.getTransition(-1, tileObject[0], false, false);

        if(transition == null)
        {
            helper.timeToChange = 0;
            WorldMap.world.setObjectHelperNull(tx,ty);

            trace('WARNING: Time: no transtion found! Maybe object was moved? tile: $tileObject helper: ${helper.id} ${helper.description}');
            return false;
        }

        var newObjectData = ObjectData.getObjectData(transition.newTargetID);

        // for example if a grave with objects decays
        if(helper.containedObjects.length > newObjectData.slotSize)
        {
            // check in another 10 sec
            helper.timeToChange += 10;
            //WorldMap.world.setObjectHelper(tx,ty, helper);

            trace('time: do not decay newTarget cannot store contained objects! ${helper.description}');
            return false;
        }

        if(transition.move > 0)
        {
            doAnimalMovement(helper, transition);
            
            return false;
        }

        if(helper.isLastUse()) transition = Server.transitionImporter.getTransition(-1, helper.id, false, true);

        helper.id = transition.newTargetID;
        helper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(helper);
        helper.creationTimeInTicks = TimeHelper.tick;

        TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, transition, false);

        WorldMap.world.setObjectHelper(tx, ty, helper);

        return true;
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
        // TODO collision detection if animal is deadly
        // TODO chasing
        // TODO fleeing
        // TODO Offspring only if with child

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

            var objectData = helper.objectData.dummyParent == null ? helper.objectData : helper.objectData.dummyParent;

            var isPreferredBiome = objectData.isSpawningIn(targetBiome);

            //if(helper.objectData.dummyParent != null) trace('Animal Move: ${objectData.description} $isPreferredBiome');

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

            // 2710 + -1 = 767 + 769 // Wild Horse with Lasso + TIME  -->  Lasso# tool + Wild Horse
            var transition = Server.transitionImporter.getTransition(helper.parentId, -1, false, false);

            if(transition != null)
            {            
                var tmpGroundObject = helper.groundObject;
                helper.groundObject = new ObjectHelper(null, transition.newActorID);
                helper.groundObject.groundObject = tmpGroundObject;

                //trace('animal movement: found -1 transition: ${helper.description} --> ${helper.groundObject.description}');
            } 

            if(helper.isLastUse()) timeTransition = Server.transitionImporter.getTransition(-1, helper.id, false, true);

            helper.id = timeTransition.newTargetID;

            TransitionHelper.DoChangeNumberOfUsesOnTarget(helper, timeTransition, false);

    
            // save what was on the ground, so that we can move on this tile and later restore it
            var oldTileObject = helper.groundObject == null ? [0]: helper.groundObject.toArray();
            var newTileObject = helper.toArray();

            //var des = helper.groundObject == null ? "NONE": helper.groundObject.description();
            //trace('MOVE: oldTile: $oldTileObject $des newTile: $newTileObject ${helper.description()}');

            // TODO only change after movement is finished
            helper.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
            helper.creationTimeInTicks = TimeHelper.tick;

            worldmap.setObjectHelper(fromTx, fromTy, helper.groundObject);

            var tmpGroundObject = helper.groundObject;
            helper.groundObject = target;
            worldmap.setObjectHelper(toTx, toTy, helper);

            var chanceForOffspring = isPreferredBiome ? ServerSettings.ChanceForOffspring : ServerSettings.ChanceForOffspring * Math.pow((1 - chancePreferredBiome), 2);
            var chanceForAnimalDying = isPreferredBiome ? ServerSettings.ChanceForOffspring / 2: ServerSettings.ChanceForAnimalDying;

            // give extra birth chance bonus if population is very low
            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] / 2) chanceForOffspring *=5;

            if(worldmap.currentObjectsCount[newTileObject[0]] < worldmap.originalObjectsCount[newTileObject[0]] * ServerSettings.MaxOffspringFactor && worldmap.randomFloat() <= chanceForOffspring)
            {
                worldmap.currentObjectsCount[newTileObject[0]] += 1;

                //if(chanceForOffspring < worldmap.chanceForOffspring) trace('NEW: $newTileObject ${helper.description()}: ${worldmap.currentPopulation[newTileObject[0]]} ${worldmap.initialPopulation[newTileObject[0]]} chance: $chanceForOffspring biome: $targetBiome');

                oldTileObject = newTileObject;
                
                var newAnimal = ObjectHelper.readObjectHelper(null, newTileObject);
                newAnimal.timeToChange = ObjectHelper.CalculateTimeToChange(timeTransition);
                newAnimal.groundObject = tmpGroundObject;
                worldmap.setObjectHelper(fromTx, fromTy, newAnimal);
            }
            else if(worldmap.currentObjectsCount[newTileObject[0]] > worldmap.originalObjectsCount[newTileObject[0]] * ServerSettings.MaxOffspringFactor && worldmap.randomFloat() <= chanceForAnimalDying)
            {
                //trace('Animal DEAD: $newTileObject ${helper.description}: Count: ${worldmap.currentObjectsCount[newTileObject[0]]} Original Count: ${worldmap.originalObjectsCount[newTileObject[0]]} chance: $chanceForAnimalDying biome: $targetBiome');

                helper.id = 0;
                newTileObject = [0];
                worldmap.setObjectHelper(toTx, toTy, helper);
            }       

            var speed = ServerSettings.InitialPlayerMoveSpeed * objectData.speedMult;

            Connection.SendAnimalMoveUpdateToAllClosePlayers(fromTx, fromTy, toTx, toTy, oldTileObject, newTileObject, speed);

            /*for (c in Server.server.connections) 
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
                c.send(FRAME, null, false);
            }*/

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
                    && movementTileObj.description.indexOf("Tarry Spot") == -1
                    && movementTileObj.description.indexOf("Tree") == -1 && movementTileObj.description.indexOf("Rabbit") == -1  
                    && movementTileObj.description.indexOf("Iron") == -1 && movementTileObj.description.indexOf("Spring") == -1
                    && movementTileObj.description.indexOf("Sugarcane") == -1 && movementTileObj.description.indexOf("Pond") == -1
                    && movementTileObj.description.indexOf("Palm") == -1  && movementTileObj.description.indexOf("Plant") == -1))
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