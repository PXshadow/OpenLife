package openlife.server;

import openlife.server.WorldMap.BiomeTag;
import openlife.data.transition.TransitionData;
import openlife.data.object.ObjectData;
import openlife.data.object.ObjectHelper;

class TransitionHelper{

    public var x:Int;
    public var y:Int;

    public var tx:Int;
    public var ty:Int;

    public var player:GlobalPlayerInstance;

    public var handObject:Array<Int>;
    public var tileObject:Array<Int>;
    public var floorId:Int;
    public var transitionSource:Int;

    public var newHandObject:Array<Int>;
    public var newTileObject:Array<Int>;
    public var newFloorId:Int;
    public var newTransitionSource:Int;

    public var tileObjectHelper:ObjectHelper;
    public var handObjectHelper:ObjectHelper;

    public var handObjectData:ObjectData;
    public var tileObjectData:ObjectData;

    // if a transition is done, the MX (MAPUPDATE) needs to send a negative palyer id to indicate that its not a drop
    public var doTransition:Bool = true;
    public var doAction:Bool;

    public function new(player:GlobalPlayerInstance, x:Int,y:Int)
    {
        trace("try to acquire player mutex");
        player.mutux.acquire();
        trace("try to acquire map mutex");
        Server.server.map.mutex.acquire();

        this.player = player;

        this.x = x;
        this.y = y;

        this.tx = x + player.gx;
        this.ty = y + player.gy;

        this.handObject = player.o_id;
        this.tileObject = Server.server.map.getObjectId(tx, ty);
        this.floorId = Server.server.map.getFloorId(tx, ty);
        this.transitionSource = player.o_transition_source_id;
        
        this.newHandObject = this.handObject;
        this.newTileObject = this.tileObject;
        this.newFloorId = this.floorId;
        this.newTransitionSource = this.transitionSource;

        // TODO
        //this.handObjectHelper = this.player.heldObject;
        //trace('Read Hand object:');
        if(this.handObjectHelper == null) this.handObjectHelper = ObjectHelper.readObjectHelper(this.player, this.handObject);

        // ObjectHelpers are for storing advanced dato like USES, CREATION TIME, OWNER
        this.tileObjectHelper = Server.server.map.getObjectHelper(tx,ty);
        //trace('Read Tile object:');
        if(this.tileObjectHelper == null) this.tileObjectHelper = ObjectHelper.readObjectHelper(this.player, this.tileObject);

        this.handObjectData = handObjectHelper.objectData;
        this.tileObjectData = tileObjectHelper.objectData;

        trace("hand: " + this.handObject + " tile: " + this.tileObject + ' tx: $tx ty:$ty');

        trace('handObjectHelper: ' + handObjectHelper.writeObjectHelper([]));
        trace('tileObjectHelper: ' + tileObjectHelper.writeObjectHelper([]));
    }

    /*
    DROP x y c#

    DROP is for setting held object down on empty grid square OR
	 for adding something to a container
     c is -1 except when adding something to own clothing, then c
     indicates clothing with:
     0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack*/

     public function drop(clothingIndex:Int=-1) : Bool
    {
        // this is a drop and not a transition
        this.doTransition = false;
        
        if(this.checkIfNotMovingAndCloseEnough() == false) return false;

        // TODO drop hand object in container

        return this.swapHandAndFloorObject();            
    } 


    /*
    USE x y id i#

    USE  is for bare-hand or held-object action on target object in non-empty 
     grid square, including picking something up (if there's no bare-handed 
     action), and picking up a container.
     id parameter is optional, and is used by server to differentiate 
     intentional use-on-bare-ground actions from use actions (in case
     where target animal moved out of the way).
     i parameter is optional, and specifies a container slot to use a held
     object on (for example, using a knife to slice bread sitting on a table).
    */

    public function use() : Bool
    {
        // TODO intentional use with index, see description above

        // TODO use on container with index, see description above

        // TODO check pickup age

        // TODO kill deadlyDistance

        // TODO feed baby

        // TODO last transitions for handObject

        // TODO fix Pile animations

        if(this.checkIfNotMovingAndCloseEnough() == false) return false;

        // do actor + target = newActor + newTarget
        if(this.doTransitionIfPossible()) return true;

        // do nothing if tile Object is empty
        if(this.tileObject[0] == 0) return false;

        // do pickup if hand is empty
        if(this.handObject[0] == 0 && this.swapHandAndFloorObject()) return true;            
        
        // do container stuff
        return this.doContainerStuff();
    }

    public function checkIfNotMovingAndCloseEnough():Bool{
        if(player.me.isMoveing()) {
            trace("Player is still moving");
            return false; 
        }

        if(player.isClose(x,y) == false) {
            trace('Object position is too far away p${player.x},p${player.y} o$x,o$y');
            return false; 
        }

        return true;
    }

    public static function doTimeTransition(helper:ObjectHelper)
    {
        // TODO support moved objects

        Server.server.map.mutex.acquire();

        Server.server.map.timeObjectHelpers.remove(helper);

        var tx = helper.tx;
        var ty = helper.ty;

        var tileObject = Server.server.map.getObjectId(tx, ty);
        var floorId = Server.server.map.getFloorId(tx, ty);

        //trace('Time: tileObject: $tileObject');

        var transition = Server.transitionImporter.getTransition(-1, tileObject[0], false, false);

        if(transition == null)
        {
            // TODO should not happen
            trace('WARNING: Time: no transtion found! Maybe object was moved? tile: $tileObject helper: ${helper.id()} ${helper.description()}');
            Server.server.map.mutex.release();
            return;
        }

        if(doAnimalMovement(helper, transition))
        {
            Server.server.map.mutex.release();
            return;
        }

        var newTileObject = [transition.newTargetID];
        Server.server.map.setObjectId(tx, ty, newTileObject);
        Server.server.map.setObjectHelper(tx, ty, null);

        // TODO take care if newTileObject has time transition

        for (c in Server.server.connections)
        {      
            var player = c.player;
            
            // since player has relative coordinates, transform them for player
            var x = tx - player.gx;
            var y = ty - player.gy;

            // update only close players
            if(player.isClose(x,y, Server.maxDistanceToBeConsideredAsClose) == false) continue;

            c.sendMapUpdate(x, y, floorId, newTileObject, -1);
            c.send(FRAME);
        }

        Server.server.map.mutex.release();
    } 

    private static function doAnimalMovement(helper:ObjectHelper, timeTransition:TransitionData) : Bool
    {
        // TODO use chance for movement
        // TODO LAST movement
        // TODO check if movement is blocked
        // TODO collision detection if animal is deadly

        var moveDist = timeTransition.move;

        if(moveDist <= 0) return false;

        var worldmap = Server.server.map;

        var x = helper.tx;
        var y = helper.ty;

        for (i in 0...10)
        {
            var tx = helper.tx - moveDist + worldmap.randomInt(moveDist * 2);
            var ty = helper.ty - moveDist + worldmap.randomInt(moveDist * 2);
            
            var target = worldmap.getObjectHelper(tx, ty);
            //var obj = worldmap.getObjectId(tx, ty);
            //var objData = Server.objectDataMap[obj[0]];

            if(target.id() != 0) continue;

            if(target.blocksWalking()) continue;
            // dont move move on top of other moving stuff
            if(target.timeToChange != 0) continue;  
            if(target.groundObject != null) continue;
            // make sure that target is not the old tile
            if(tx == helper.tx && ty == helper.ty) continue;
            
            var targetBiome = worldmap.getBiomeId(tx,ty);
            if(targetBiome == BiomeTag.SNOWINGREY) continue;
            if(targetBiome == BiomeTag.OCEAN) continue;

            var isPreferredBiome = false;

            for(biome in helper.objectData.biomes){
                if(targetBiome == biome){
                    //trace('isPreferredBiome: $biome');
                    isPreferredBiome = true;
                } 
            }
            
            // lower the chances even more if on river
            //var isHardbiome = targetBiome == BiomeTag.RIVER || (targetBiome == BiomeTag.GREY) || (targetBiome == BiomeTag.SNOW) || (targetBiome == BiomeTag.DESERT);
            var isNotHardbiome =  isPreferredBiome || targetBiome == BiomeTag.GREEN || targetBiome == BiomeTag.YELLOW;

            var chancePreferredBiome = isNotHardbiome ? worldmap.chancePreferredBiome : (worldmap.chancePreferredBiome + 4) / 5;

            //trace('chance: $chancePreferredBiome isNotHardbiome: $isNotHardbiome biome: $targetBiome');

            // skip with chancePreferredBiome if this biome is not preferred
            if(isPreferredBiome == false && i < Math.round(chancePreferredBiome * 10) &&  worldmap.randomFloat() <= chancePreferredBiome) continue;

            // save what was on the ground, so that we can move on this tile and later restore it
            var oldTileObject = helper.groundObject == null ? [0]: helper.groundObject.writeObjectHelper([]);
            var newTileObject = helper.writeObjectHelper([]);

            //var des = helper.groundObject == null ? "NONE": helper.groundObject.description();
            //trace('MOVE: oldTile: $oldTileObject $des newTile: $newTileObject ${helper.description()}');

            // TODO only change after movement is finished
            helper.timeToChange = worldmap.calculateTimeToChange(timeTransition);
            helper.creationTimeInTicks = Server.server.tick;

            worldmap.setObjectHelper(x, y, helper.groundObject);
            worldmap.setObjectId(x,y, oldTileObject); // TODO move to setter

            var tmpGroundObject = helper.groundObject;
            helper.groundObject = target;
            worldmap.setObjectHelper(tx, ty, helper);

            var chanceForOffspring = isPreferredBiome ? worldmap.chanceForOffspring : worldmap.chanceForOffspring * (1 - chancePreferredBiome);

            // give extra birth chance bonus if population is very low
            if(worldmap.currentPopulation[newTileObject[0]] < worldmap.initialPopulation[newTileObject[0]] / 2) chanceForOffspring *=5;

            if(worldmap.currentPopulation[newTileObject[0]] < worldmap.initialPopulation[newTileObject[0]] * worldmap.maxOffspringFactor && worldmap.randomFloat() <= chanceForOffspring)
            {
                // TODO consider dead 
                worldmap.currentPopulation[newTileObject[0]] += 1;

                if(chanceForOffspring < worldmap.chanceForOffspring) trace('NEW: $newTileObject ${helper.description()}: ${worldmap.currentPopulation[newTileObject[0]]} ${worldmap.initialPopulation[newTileObject[0]]} chance: $chanceForOffspring biome: $targetBiome');

                oldTileObject = newTileObject;
                
                var newAnimal = ObjectHelper.readObjectHelper(null, newTileObject);
                newAnimal.timeToChange = worldmap.calculateTimeToChange(timeTransition);
                newAnimal.groundObject = tmpGroundObject;
                worldmap.setObjectHelper(x, y, newAnimal);
                //worldmap.setObjectId(x,y, newTileObject); // TODO move to setter
            }
            
            var floorId = Server.server.map.getFloorId(tx, ty);
            // TODO better speed calculation
            var speed = 5;

            for (c in Server.server.connections) 
            {            
                var player = c.player;
              
                 // since player has relative coordinates, transform them for player
                var fromX = x - player.gx;
                var fromY = y - player.gy;
                var targetX = tx - player.gx;
                var targetY = ty - player.gy;

                // update only close players
                if(player.isClose(targetX,targetY, Server.maxDistanceToBeConsideredAsClose) == false) continue;

                /*
                MX
                x y new_floor_id new_id p_id old_x old_y speed

                Optionally, a line can contain old_x, old_y, and speed.
                This indicates that the object came from the old coordinates and is moving
                with a given speed.
                */

                c.sendMapUpdateForMoving(targetX, targetY, floorId, newTileObject, -1, fromX, fromY, speed);
                c.sendMapUpdate(fromX, fromY, floorId, oldTileObject, -1);
                //c.sendMapUpdate(fromX, fromY, floorId, [0], -1);
                c.send(FRAME);
            }
            

            return true;
        }

        helper.creationTimeInTicks = Server.server.tick;

        return false;
    }    
        

    public function doTransitionIfPossible() : Bool
    {
        // TODO //public var useChance:Float = 0;

        // TODO lastUseActorObject
        var lastUseActorObject = false;
        var lastUseTileObject = false;

        trace('tileObjectData.numUses: ${tileObjectData.numUses} tileObjectHelper.numberOfUses: ${this.tileObjectHelper.numberOfUses} ${tileObjectData.description}'  );
        if(tileObjectData.numUses > 1 && this.tileObjectHelper.numberOfUses <= 1){
            lastUseTileObject = true;
            trace("lastUseTileObject = true");
        }

        var transition = Server.transitionImporter.getTransition(this.handObject[0], this.tileObject[0], lastUseActorObject, lastUseTileObject);

        var targetIsFloor = false;

        // check if there is a floor and no object is on the floor. otherwise the object may be overriden
        if((transition == null) && (this.floorId != 0) && (this.tileObject[0] == 0)){
            transition = Server.transitionImporter.getTransition(this.handObject[0], this.floorId);
            if(transition != null) targetIsFloor = true;
        }

        if(transition == null) return false;

        trace('Found transition: a${transition.actorID} t${transition.targetID}');

        var newTargetObjectData = Server.objectDataMap[transition.newTargetID];
        
        if(newTargetObjectData.floor && this.floorId != 0) return false;

        if(newTargetObjectData.floor)
        {
            if(targetIsFloor == false) this.newTileObject = [0];
            this.newFloorId = transition.newTargetID;
        }
        else
        {
            if(targetIsFloor) this.newFloorId = 0;
            this.newTileObject = [transition.newTargetID];
        }

        //transition source object id (or -1) if held object is result of a transition 
        //if(transition.newActorID != this.handObject[0]) this.newTransitionSource = -1;
        this.newTransitionSource = transition.targetID; // TODO ???

        this.newHandObject = [transition.newActorID];

        
        
        // TODO not sure if there needs to be done more if object changes type
        this.tileObjectHelper.objectData = newTargetObjectData; 

        // Add HelperObject to timeObjectHelpers if newTargetObject has time transitions
        var timeTransition = Server.transitionImporter.getTransition(-1, transition.newTargetID, false, false);
        if(timeTransition != null)
        {
            trace('TIME: has time transition: ${transition.newTargetID} ${newTargetObjectData.description} time: ${timeTransition.autoDecaySeconds}');

            tileObjectHelper.timeToChange = timeTransition.autoDecaySeconds;
            tileObjectHelper.tx = this.tx;
            tileObjectHelper.ty = this.ty;

            Server.server.map.timeObjectHelpers.push(tileObjectHelper);
            Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);
        }

        // create advanced object if USES > 0
        trace('tileObject: ${tileObject} newTileObject: ${newTileObject} newTargetObjectData.numUses: ${newTargetObjectData.numUses}');
        if(this.tileObject[0] != this.newTileObject[0] && newTargetObjectData.numUses > 1)
        {
            // a Pile starts with 2 uses not with the full
            // if the ObjectHelper is created through a reverse use, it must be a pile...
            if(transition.reverseUseTarget){
                trace("NEW PILE?");
                this.tileObjectHelper.numberOfUses = 1;
            } 
            
            Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);

            trace('Changed Target Object Type: numberOfUses: ' + this.tileObjectHelper.numberOfUses);

            // if a transition is done, the MX (MAPUPDATE) needs to send a negative palyer id to indicate that its not a drop
            this.doTransition = true;
            this.doAction = true;

            return true;

        }

        if(transition.reverseUseTarget)
        {
            this.tileObjectHelper.numberOfUses += 1;
            trace('numberOfUses: ' + this.tileObjectHelper.numberOfUses);
        } 
        else
        {
            this.tileObjectHelper.numberOfUses -= 1;

            
            trace('numberOfUses: ' + this.tileObjectHelper.numberOfUses);

            if(this.tileObjectHelper.numberOfUses < 1) {
                trace("REMOVE ObjectHelper USES < 1");
                this.tileObjectHelper = null;
                Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);
            }
            else{
                Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);
            }
        }
    
        // if a transition is done, the MX (MAPUPDATE) needs to send a negative palyer id to indicate that its not a drop
        this.doTransition = true;
        this.doAction = true;

        return true;
    }

    public function swapHandAndFloorObject():Bool{

        //trace("SWAP tileObjectData: " + tileObjectData.toFileString());
        
        var permanent = (tileObjectData != null) && (tileObjectData.permanent == 1);

        if(permanent) return false;

        // TODO for now picking up stuff that can change is forbidden
        //if(tileObjectHelper.timeToChange > 0) return false;

        this.newTileObject = this.handObject;
        this.newHandObject = this.tileObject;

        this.doAction = true;
        return true;
    }

    public function doContainerStuff() : Bool
    {
        trace("containable: " + tileObjectData.containable + " desc: " + tileObjectData.description + " numSlots: " + tileObjectData.numSlots);

        // TODO change container check
        //if ((objectData.numSlots == 0 || MapData.numSlots(this.tileObject) >= objectData.numSlots)) return false;
        if (tileObjectData.numSlots == 0) return false; 

        if(this.tileObjectHelper.containedObjects.length >= tileObjectData.numSlots) return false;
        
        // place hand object in container if container has enough space
        //if (handObjectData.slotSize >= objectData.containSize) {
        trace('handObjectData.slotSize: ${handObjectData.slotSize} tileObjectData.containSize: ${tileObjectData.containSize}');
        if (handObjectData.slotSize > tileObjectData.containSize) return false;

        trace('HO ${this.handObject[0]}');
        trace('HO Slot size: ${handObjectData.slotSize} TO: container Size size: ${tileObjectData.containSize}');

        this.newHandObject = [0];

        //trace('Hand object: ${handObjectHelper.writeObjectHelper([])}');

        tileObjectHelper.containedObjects.push(handObjectHelper);

        trace('Tile object: ${tileObject}');

        this.newTileObject = tileObjectHelper.writeObjectHelper([]);

        trace('New Tile object: ${newTileObject}');

        this.doAction = true;
        return true;
    }

    /*
    REMV x y i#

    REMV is special case of removing an object from a container.
     i specifies the index of the container item to remove, or -1 to
     remove top of stack.*/

    public function remove(index:Int)
    {
        trace("remove index " + index);

        // do nothing if tile Object is empty
        if(this.tileObject[0] == 0) return false;

        if(tileObjectHelper.containedObjects.length < 1) return false;            

        this.newHandObject = tileObjectHelper.removeContainedObject(index).writeObjectHelper([]);

        this.newTileObject = tileObjectHelper.writeObjectHelper([]);

        if(this.handObject [0] != 0){
            // TODO check if it hand item fits in container
            tileObjectHelper.containedObjects.push(handObjectHelper);
        }

        this.doAction = true;
        return true;

        /*
        //var newTileObject = Server.server.map.getObjectId(x + gx,y + gy);
        //trace("tile: "  + newTileObject);


        var doAction = false;
        if (newTileObject.length > 1) 
        {
            doAction = true;
            if (this.o_id[0] == 0)
            {
                //non swap
                trace("before: " + newTileObject);
                this.o_id = MapData.getObjectFromContainer(newTileObject);
                trace("after: " + newTileObject);
            }else{
                //swap
                trace("swap before: hand: " + o_id + " tile " + newTileObject);
                var hand = MapData.toContainer(o_id);
                newTileObject = newTileObject.concat(hand);
                hand = MapData.getObjectFromContainer(newTileObject);
                o_id = hand;
                trace("swap after: hand: " + o_id + " tile " + newTileObject);
            }
        }        
        */
    }

    public function sendUpdateToClient() : Bool
    {

        // even send Player Update / PU if nothing happend. Otherwise client will get stuck
        if(this.doAction == false){
            player.connection.send(PLAYER_UPDATE,[player.toData()]);
            player.connection.send(FRAME);

            trace("release player mutex");
            Server.server.map.mutex.release();
            trace("release map mutex");
            player.mutux.release();

            return false;
        }

        Server.server.map.setObjectId(this.tx, this.ty, this.newTileObject);
        Server.server.map.setFloorId(this.tx, this.ty, this.newFloorId);

        player.o_id = this.newHandObject;

        player.action = 1;

        // TODO set right
        player.o_origin_x = this.x;
        player.o_origin_y = this.y;
        player.o_origin_valid = 1; // what is this for???

        player.o_transition_source_id = this.newTransitionSource;
        player.action_target_x = this.x;
        player.action_target_y = this.y;
        player.forced = false;

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[player.toData()]);
            /*
            MX (MAP UPDATE)
            p_id is the player that was responsible for the change (in the case of an 
            object drop only), or -1 if change was not player triggered.  p_id < -1 means
            that the change was triggered by player -(p_id), but that the object
            wasn't dropped (transform triggered by a player action).
            */
            if(this.doAction){
                if(this.doTransition){
                    c.sendMapUpdate(x, y, this.newFloorId, this.newTileObject, (-1) * player.p_id);
                }
                else{
                    c.sendMapUpdate(x, y, this.newFloorId, this.newTileObject, player.p_id);
                }
            }
            c.send(FRAME);
        }

        player.action = 0;

        
        trace("release player mutex");
        Server.server.map.mutex.release();
        trace("release map mutex");
        player.mutux.release();

        return true;
    }
}