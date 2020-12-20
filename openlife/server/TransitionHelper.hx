package openlife.server;

import sys.db.Object;
import openlife.settings.ServerSettings;
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

    //public var index:Int = -1; // Index in container or clothing index in case of drop

    //public var handObject:Array<Int>;
    //public var tileObject:Array<Int>;
    public var floorId:Int;
    public var transitionSource:Int;

    //public var newHandObject:Array<Int>;
    //public var newTileObject:Array<Int>;
    public var newFloorId:Int;
    public var newTransitionSource:Int;

    public var tileObjectHelper:ObjectHelper;
    //public var handObjectHelper:ObjectHelper;

    public var handObjectData:ObjectData;
    public var tileObjectData:ObjectData;

    // if a transition is done, the MX (MAPUPDATE) needs to send a negative palyer id to indicate that its not a drop
    public var doTransition:Bool = true;
    public var doAction:Bool;

    public static function doCommand(player:GlobalPlayerInstance, tag:ServerTag, x:Int, y:Int, index:Int = -1, target:Int = 0)
    {
        //trace("try to acquire player mutex");
        player.mutex.acquire();
        //trace("try to acquire map mutex");
        Server.server.map.mutex.acquire();

        if(ServerSettings.debug)
        {
            doCommandHelper(player, tag, x, y, index, target);
        }
        else{
            try
            {
                doCommandHelper(player, tag, x, y, index, target);
            } 
            catch(e)
            {                
                trace(e);

                // send PU so that player wont get stuck
                player.connection.send(PLAYER_UPDATE,[player.toData()]);
                player.connection.send(FRAME);
            }
        }

        //trace("release player mutex");
        Server.server.map.mutex.release();
        //trace("release map mutex");
        player.mutex.release();
    }  

    public static function doCommandHelper(player:GlobalPlayerInstance, tag:ServerTag, x:Int, y:Int, index:Int = -1, target:Int = 0)
    {
        var helper = new TransitionHelper(player, x, y);

        switch (tag)
        {
            case USE: 
                helper.use(target, index);
            case DROP:
                helper.drop(index); 
            case REMV:
                helper.remove(index);
            default:
        }

        helper.sendUpdateToClient();
    }

    public function new(player:GlobalPlayerInstance, x:Int,y:Int)
    {
        this.player = player;

        this.x = x;
        this.y = y;

        this.tx = x + player.gx;
        this.ty = y + player.gy;

        this.floorId = Server.server.map.getFloorId(tx, ty);
        this.transitionSource = player.o_transition_source_id;
        
        this.newFloorId = this.floorId;
        this.newTransitionSource = this.transitionSource;

        // ObjectHelpers are for storing advanced dato like USES, CREATION TIME, OWNER
        this.tileObjectHelper = Server.server.map.getObjectHelper(tx,ty);
       
        this.handObjectData = player.heldObject.objectData;
        this.tileObjectData = tileObjectHelper.objectData;

        //trace("hand: " + this.handObject + " tile: " + this.tileObject + ' tx: $tx ty:$ty');

        
        trace('handObjectHelper: ${handObjectData.description} ' + player.heldObject.writeObjectHelper([]));
        trace('tileObjectHelper: ${tileObjectData.description} ' + tileObjectHelper.writeObjectHelper([]));
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

        if(this.tileObjectData.minPickupAge > player.age)
        {
            trace('DROP: tileObjectData.minPickupAge: ${tileObjectData.minPickupAge} player.age: ${player.age}');
            return false;
        }
        
        if(this.checkIfNotMovingAndCloseEnough() == false) return false;

        // switch hand object in container with last object in container 
        if(this.doContainerStuff(true)) return true;

        // TODO adding something to own clothing using clothingIndex

        // TODO check if there is enough space for horse cart

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

    public function use(target:Int, index:Int) : Bool
    {
        // TODO intentional use with index, see description above

        // TODO use on container with index, see description above

        // TODO kill deadlyDistance

        // TODO feed baby

        // TODO hungry work

        // TODO noUseActor / noUseTarget

        // TODO dummy object for handObject like tools / axe 

        // TODO transitions on animals and caves

        if(this.tileObjectData.minPickupAge > player.age)
        {
            trace('tileObjectData.minPickupAge: ${tileObjectData.minPickupAge} player.age: ${player.age}');
            return false;
        }

        if (this.tileObjectHelper.objectData.tool) {
            player.connection.send(LEARNED_TOOL_REPORT,['0 ${this.tileObjectHelper.id()}']);
            trace("TOOL LEARNED! " + this.tileObjectHelper.id());
        }
        if(this.checkIfNotMovingAndCloseEnough() == false) return false;

        // do actor + target = newActor + newTarget
        if(this.doTransitionIfPossible()) return true;

        // do nothing if tile Object is empty
        if(this.tileObjectHelper.id() == 0) return false;

        // do pickup if hand is empty
        if(this.player.heldObject.id() == 0 && this.swapHandAndFloorObject()) return true;            
        
        // do container stuff
        return this.doContainerStuff(false, index);
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

    public function doTransitionIfPossible() : Bool
    {
        // dummies are objects with numUses > 1 numUse = maxUse is the original
        if(tileObjectData.dummy)
        {
            tileObjectData = tileObjectData.dummyParent;
        }
    
        var lastUseActorObject = false;
        var lastUseTileObject = false;

        trace('TRANS: handObjectData.numUses: ${handObjectData.numUses} heldObject.numberOfUses: ${this.player.heldObject.numberOfUses} ${handObjectData.description}'  );
        
        if(handObjectData.numUses > 1 && this.player.heldObject.numberOfUses <= 1)
        {
            // TODO ??? seems like for tools there is not always a last use transition
            //lastUseActorObject = true;
            //trace("lastUseActorObject = true");
        }

        trace('TRANS: tileObjectData.numUses: ${tileObjectData.numUses} tileObjectHelper.numberOfUses: ${this.tileObjectHelper.numberOfUses} ${tileObjectData.description}'  );

        if(tileObjectData.numUses > 1 && this.tileObjectHelper.numberOfUses <= 1)
        {
            lastUseTileObject = true;
            trace("TRANS: lastUseTileObject = true");
        }

        var transition = Server.transitionImporter.getTransition(this.player.heldObject.id(), this.tileObjectData.id, lastUseActorObject, lastUseTileObject);

        // sometimes ground is -1 not 0 like for Riding Horse: 770 + -1 = 0 + 1421 // TODO -1 --> 0 in transition importer???
        if(transition == null && tileObjectHelper.id() == 0)
        {
            transition = Server.transitionImporter.getTransition(this.player.heldObject.id(), -1, lastUseActorObject, lastUseTileObject);
        }

        var targetIsFloor = false;

        // check if there is a floor and no object is on the floor. otherwise the object may be overriden
        if((transition == null) && (this.floorId != 0) && (this.tileObjectData.id == 0))
        {
            transition = Server.transitionImporter.getTransition(this.player.heldObject.id(), this.floorId);
            if(transition != null) targetIsFloor = true;
        }

        if(transition == null) return false;

        trace('TRANS: Found transition: a${transition.actorID} t${transition.targetID} ');
        transition.traceTransition();

        var newTargetObjectData = ObjectData.getObjectData(transition.newTargetID);

        // TODO check also for handobject???
        // if it is a reverse transition, check if it would exceed max numberOfUses 
        if(transition.reverseUseTarget && this.tileObjectHelper.numberOfUses >= newTargetObjectData.numUses)
        {
            trace('TRANS: numberOfUses >= newTargetObjectData.numUses: try use maxUseTransition');
            transition = Server.transitionImporter.getTransition(this.player.heldObject.id(), this.tileObjectData.id, false, false, true);

            if(transition == null)
            {
                trace('TRANS: Cannot do reverse transition for taget: TileObject: ${this.tileObjectHelper.id()} numUses: ${this.tileObjectHelper.numberOfUses} newTargetObjectData.numUses: ${newTargetObjectData.numUses}');

                return false;
            }

            // for example a well site with max stones
            // 33 + 1096 = 0 + 1096 targetRemains: true
            // 33 + 1096 = 0 + 3963 targetRemains: false (maxUseTransition)
            trace('TRANS: use maxUseTransition');

            //transition = transition.maxUseTransition;

            // TODO must set newTargetObjectData???
        }

        // if it is a transition that picks up an object like 0 + 1422 = 778 + 0  (horse with cart) then switch the hole tile object to the hand object
        // TODO this may make trouble
        // 778 + -1 = 0 + 1422
        if((transition.actorID == 0 && transition.targetID != transition.newTargetID) || (transition.targetID == -1 && transition.newActorID == 0))
        {
            trace('TRANS: switch held object with tile object');

            var tmpHeldObject = player.heldObject;
            player.setHeldObject(this.tileObjectHelper);
            this.tileObjectHelper = tmpHeldObject;

            // reset creation time, so that horses wont esape instantly
            this.tileObjectHelper.creationTimeInTicks = TimeHelper.tick;
        }
            
        // dont allow to place another floor on existing floor
        if(newTargetObjectData.floor && this.floorId != 0) return false; 

        // do now the magic transformation
        player.transformHeldObject(transition.newActorID);
        this.tileObjectHelper.setId(transition.newTargetID);

        if(newTargetObjectData.floor)
        {
            if(targetIsFloor == false) this.tileObjectHelper.setId(0);
            this.newFloorId = transition.newTargetID;
        }
        else
        {
            if(targetIsFloor) this.newFloorId = 0;
        }

        //transition source object id (or -1) if held object is result of a transition 
        //if(transition.newActorID != this.handObject[0]) this.newTransitionSource = -1;
        //this.newTransitionSource = transition.targetID; // TODO ???
                    
        // TODO move to SetObjectHelper
        this.tileObjectHelper.timeToChange = ObjectHelper.CalculateTimeToChangeForObj(this.tileObjectHelper);

        DoChangeNumberOfUsesOnActor(this.player.heldObject, transition.actorID != transition.newActorID, transition.reverseUseActor);

        trace('TRANS: NewTileObject: ${newTargetObjectData.description} ${this.tileObjectHelper.id()} newTargetObjectData.numUses: ${newTargetObjectData.numUses}');

        // target did not change if it is same dummy
        //var targetChanged = tileObjectData.id != newTargetObjectData.id;

        DoChangeNumberOfUsesOnTarget(this.tileObjectHelper, transition.targetID != transition.newTargetID, transition.reverseUseTarget);

        // DO dummies for objects that have more then one numUses
        DoDummies(this.tileObjectHelper);

        // TODO do dummies for hand object???
    
        // if a transition is done, the MX (MAPUPDATE) needs to send a negative palyer id to indicate that its not a drop
        this.doTransition = true;
        this.doAction = true;

        return true;
    }
    
    private static function DoChangeNumberOfUsesOnActor(obj:ObjectHelper, idHasChanged:Bool, reverseUse:Bool)
    {
        if(idHasChanged) return;

        var objectData  = obj.objectData;

        if(reverseUse)
        {
            obj.numberOfUses += 1;
            trace('HandObject: numberOfUses: ' + obj.numberOfUses);
            return;
        } 

        trace('handObjectData.useChance: ${objectData.useChance}');

        if(objectData.useChance > 0 && WorldMap.calculateRandomFloat() > objectData.useChance) return;

        obj.numberOfUses -= 1;
        trace('HandObject: numberOfUses: ' + obj.numberOfUses);

        if(obj.numberOfUses > 0) return;

        // for example for a tool like axe lastUseActor: true
        var toolTransition = Server.transitionImporter.getTransition(obj.id(), -1, true, false);
        
        // for example for a water bowl lastUseActor: false
        if(toolTransition == null)
        {
            toolTransition = Server.transitionImporter.getTransition(obj.id(), -1, false, false);
        }

        if(toolTransition != null)
        {
            trace('Change Actor from: ${obj.id} to ${toolTransition.newActorID}');
            obj.setId(toolTransition.newActorID);
        }
    }

    private static function DoChangeNumberOfUsesOnTarget(obj:ObjectHelper, idHasChanged:Bool, reverseUse:Bool)
    {
        var objectData  = obj.objectData;

        if(idHasChanged && objectData.numUses > 1)
        {
            // a Pile starts with 1 uses not with the full numberOfUses
            // if the ObjectHelper is created through a reverse use, it must be a pile or a bucket... hopefully...
            if(reverseUse)
            {
                trace("TRANS: NEW PILE OR BUCKET?");
                obj.numberOfUses = 1;
            } 
            
            trace('TRANS: Changed Object Type: ${objectData.description} numberOfUses: ' + obj.numberOfUses);
            return;
        }

        if(reverseUse)
        {
            obj.numberOfUses += 1;
            trace('TRANS: ${objectData.description} numberOfUses: ' + obj.numberOfUses);
        } 
        else
        {
            trace('TRANS: ${objectData.description} objectData.useChance: ${objectData.useChance}');

            if(objectData.useChance <= 0 || WorldMap.calculateRandomFloat() < objectData.useChance)
            {
                obj.numberOfUses -= 1;
                trace('TRANS: ${objectData.description} numberOfUses: ' + obj.numberOfUses);
                //Server.server.map.setObjectHelper(tx,ty, obj); // deletes ObjectHelper in case it has no uses
            }
        }
    }

    private static function DoDummies(obj:ObjectHelper)
    {
        var objectData  = obj.objectData;

        if(objectData.numUses < 2) return;

        // in case of an maxUses object changing like a well site numOfUses can be too big
        if(obj.numberOfUses > objectData.numUses)
        {
            obj.numberOfUses = objectData.numUses;
        }

        if(obj.numberOfUses == objectData.numUses)
        {
            if(obj.objectData.dummy)
            {
                obj.objectData = obj.objectData.dummyParent;
            }
        }
        else
        {
            obj.objectData = objectData.dummyObjects[obj.numberOfUses-1];
            trace('dummy id: ${obj.objectData.id}');
        }
    }

    public function swapHandAndFloorObject():Bool{

        //trace("SWAP tileObjectData: " + tileObjectData.toFileString());
        
        var permanent = (tileObjectData != null) && (tileObjectData.permanent == 1);

        if(permanent) return false;

        var tmpTileObject = tileObjectHelper;

        this.tileObjectHelper = this.player.heldObject;
        this.player.setHeldObject(tmpTileObject);

        // transform object if put down like for horse transitions
        // 778 + -1 = 0 + 1422 
        // 770 + -1 = 0 + 1421
        var transition = Server.transitionImporter.getTransition(this.tileObjectHelper.id(), -1, false, false);
        if(transition != null)
        {
            trace('transform object ${tileObjectHelper.description()} in ${transition.newTargetID} / used when to put down horses');

            tileObjectHelper.setId(transition.newTargetID);
        }

        this.doAction = true;
        return true;
    }

    // DROP switches the object with the last object in the container and cycles throuh the objects / USE just put it in
    public function doContainerStuff(isDrop:Bool = false, index:Int = -1) : Bool
    {
        trace("containable: " + tileObjectData.containable + " desc: " + tileObjectData.description + " numSlots: " + tileObjectData.numSlots);

        // TODO change container check
        //if ((objectData.numSlots == 0 || MapData.numSlots(this.tileObject) >= objectData.numSlots)) return false;
        if (tileObjectData.numSlots == 0) return false; 

        var amountOfContainedObjects = tileObjectHelper.containedObjects.length;

        // if hand is empty then remove last object from container
        if(player.heldObject.id() == 0 && amountOfContainedObjects > 0)
        {
            trace("CALL REMOVE");
            if(remove(index)) return true;
            return false;
        }

        if(handObjectData.containable == false)
        {
            trace('handObject is not containable!');
            return false;
        }

        // place hand object in container if container has enough space
        //if (handObjectData.slotSize >= objectData.containSize) {
        trace('handObjectData.slotSize: ${handObjectData.slotSize} tileObjectData.containSize: ${tileObjectData.containSize}');
        if (handObjectData.slotSize > tileObjectData.containSize) return false;

        trace('Hand Object ${this.player.heldObject.id()}');
        trace('Hand Object Slot size: ${handObjectData.slotSize} TO: container Size: ${tileObjectData.containSize}');

        if(isDrop == false)            
        {
            if(amountOfContainedObjects >= tileObjectData.numSlots) return false;

            tileObjectHelper.containedObjects.push(this.player.heldObject);

            this.player.heldObject = ObjectHelper.readObjectHelper(player,[0]);

            this.doAction = true;
            return true;
        }

        var tmpObject = tileObjectHelper.removeContainedObject(-1);

        tileObjectHelper.containedObjects.insert(0 , this.player.heldObject);

        this.player.setHeldObject(tmpObject);

        trace('DROP SWITCH Hand object: ${player.heldObject.writeObjectHelper([])}');
        trace('DROP SWITCH New Tile object: ${tileObjectHelper.writeObjectHelper([])}');

        this.doAction = true;
        return true;
    }

    /*
    REMV x y i#

    REMV is special case of removing an object from a container.
     i specifies the index of the container item to remove, or -1 to
     remove top of stack.*/

    public function remove(index:Int) : Bool
    {
        trace("remove index " + index);

        // do nothing if tile Object is empty
        if(this.tileObjectHelper.id() == 0) return false;

        if(tileObjectHelper.containedObjects.length < 1)
        {
            if(index != -1) return false;

            // it may be a USE on a horse cart???
            //return doTransitionIfPossible();            
        }

        this.player.setHeldObject(tileObjectHelper.removeContainedObject(index));

        this.doAction = true;
        return true;
    }

    /*
            MX (MAP UPDATE)
            p_id is the player that was responsible for the change (in the case of an 
            object drop only), or -1 if change was not player triggered.  p_id < -1 means
            that the change was triggered by player -(p_id), but that the object
            wasn't dropped (transform triggered by a player action).
    */

    public function sendUpdateToClient() : Bool
    {
        // even send Player Update / PU if nothing happend. Otherwise client will get stuck
        if(this.doAction == false){
            player.connection.send(PLAYER_UPDATE,[player.toData()]);
            player.connection.send(FRAME);

            return false;
        }

        trace('NEW: handObjectHelper: ${player.heldObject.description()} ' + player.heldObject.writeObjectHelper([]));
        trace('NEW: tileObjectHelper: ${tileObjectHelper.description()} ' + tileObjectHelper.writeObjectHelper([]));

        Server.server.map.setFloorId(this.tx, this.ty, this.newFloorId);
        Server.server.map.setObjectHelper(this.tx, this.ty, this.tileObjectHelper);

        var newTileObject = this.tileObjectHelper.writeObjectHelper([]);

        player.o_id = this.player.heldObject.writeObjectHelper([]);

        player.action = 1;

        // TODO set right
        player.o_origin_x = this.x;
        player.o_origin_y = this.y;
        player.o_origin_valid = 1; // what is this for???

        player.o_transition_source_id = this.newTransitionSource;
        player.action_target_x = this.x;
        player.action_target_y = this.y;
        player.forced = false;

        for (c in Server.server.connections) 
        {
            // since player has relative coordinates, transform them for player
            var targetX = this.tx - c.player.gx;
            var targetY = this.ty - c.player.gy;

            // update only close players
            if(c.player.isClose(targetX,targetY, ServerSettings.maxDistanceToBeConsideredAsClose) == false) continue;

            c.send(PLAYER_UPDATE,[player.toRelativeData(c.player)]);
            
            if(this.doAction){
                if(this.doTransition){
                    c.sendMapUpdate(targetX, targetY, this.newFloorId, newTileObject, (-1) * player.p_id);
                }
                else{
                    c.sendMapUpdate(targetX, targetY, this.newFloorId, newTileObject, player.p_id);
                }
            }
            c.send(FRAME);
        }

        player.action = 0;

        return true;
    }
}