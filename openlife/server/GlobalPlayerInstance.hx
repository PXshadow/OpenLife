package openlife.server;
import haxe.ds.Vector;
import openlife.data.object.ObjectHelper;
import openlife.data.map.MapData;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using openlife.server.MoveExtender;

class GlobalPlayerInstance extends PlayerInstance {
    // holds additional ObjectInformation for the object held in hand / null if there is no additional object data
    public var heldObject:ObjectHelper; 

    // additional ObjectInformation for the object stored in backpack or other clothing. The size is 6 since 6 clothing slots
    public var clothingObjects:Vector<ObjectHelper> = new Vector(6); 

    // handles all the movement stuff
    public var me:MoveExtender = new MoveExtender();
    // is used since move and move update can change the player at the same time
    public var mutux = new Mutex();

    public var connection:Connection; 

    // remember that y is counted from bottom not from top
    public var gx:Int = 400; //global x offset from birth
    public var gy:Int = 300; //global y offset from birth 

    public function new(a:Array<String>)
    {
        super(a);
    }

    public function isClose(x:Int, y:Int, distance:Int = 1):Bool{    
        return (((this.x - x) * (this.x - x) <= distance) && ((this.y - y) * (this.y - y) <= distance));
    }

    /*
    SELF x y i#

    SELF is special case of USE action taken on self (to eat what we're holding
     or add/remove clothing).
     This differentiates between use actions on the object at our feet
     (same grid cell as us) and actions on ourself.
     If holding food i is ignored.
	 If not holding food, then SELF removes clothing, and i specifies
	 clothing slot:
     0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
    */
    public function self(x:Int, y:Int, clothingSlot:Int)
    {
        var doaction = false;
        var p_clothingSlot = -1;

        // TODO food on self

        if(this.o_id[0] != 0){
            var objectData = Server.objectDataMap[this.o_id[0]];
            //trace("OD: " + objectData.toFileString());

            if(objectData.clothing.charAt(0) == 'h'){
                p_clothingSlot = 0;
            }

            switch objectData.clothing.charAt(0) {
                case "h": p_clothingSlot = 0;
                case "t": p_clothingSlot = 1;
                case "s": p_clothingSlot = 2;
                //case "s": p_clothingSlot = 3; 
                case "b": p_clothingSlot = 4;
                case "p": p_clothingSlot = 5;
            }

            //trace('objectData.clothing: ${objectData.clothing}');
            //trace('p_clothingSlot:  ${p_clothingSlot}');
            //trace('clothingSlot:  ${clothingSlot}');
        }

        if(p_clothingSlot >= 0 || clothingSlot >=0){
            var array = this.clothing_set.split(";");

            if(array.length < 6){
                trace('Clothing string missing slots: ${this.clothing_set}' );
            }  

            // set  the index for shoes that come on the other feet
            if(p_clothingSlot == 2 && clothingSlot == -1){
                clothingSlot = 3;
            }else{
                clothingSlot = p_clothingSlot;
            }

            // TODO if the clothing are shoes and there are shoes allready on the first shoe but not on the second and if the index is not set
            
            if(clothingSlot >= 0){
                // switch clothing if there is a clothing on this slot
                var tmp = Std.parseInt(array[clothingSlot]);
                array[clothingSlot] = '${this.o_id[0]}';
                this.clothing_set = '${array[0]};${array[1]};${array[2]};${array[3]};${array[4]};${array[5]}';

                doaction = true;
                this.o_id = [tmp];
                this.action = 1;
                this.action_target_x = x;
                this.action_target_y = y;
                this.o_origin_x = x;
                this.o_origin_y = y;
                this.o_origin_valid = 0; // TODO ???

                //trace('this.clothing_set: ${this.clothing_set}');
            }

            //this.clothing_set = "0;0;0;0;0;0";
        }
        

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            //if(doaction) c.sendMapUpdate(x,y,newFloorId, tile_o_id[0], this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
    }
    
    public function remove(x:Int,y:Int,id:Null<Int>)
    {
        trace("remove " + x + " " + y + " id " + id);
        var newTileObject = Server.server.map.getObjectId(x + gx,y + gy);
        trace("tile: "  + newTileObject);
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
        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            if(doAction) c.sendMapUpdate(x, y, 0, newTileObject, p_id);
            c.send(FRAME);
        }
    }

    public function specialRemove(x:Int,y:Int,clothing:Int,id:Null<Int>)
    {
        for (c in Server.server.connections) // TODO only for visible players
            {
                c.send(PLAYER_UPDATE,[this.toData()]);
                c.send(FRAME);
            }
    }

    // even send Player Update / PU if nothing happend. Otherwise client will get stuck
    public function use(x:Int,y:Int) : Bool
    {
        var helper = new TransitionHelper(this, x, y);

        helper.use();

        return helper.sendUpdateToClient();
    }

    // even send Player Update / PU if nothing happend. Otherwise client will get stuck
    public function drop(x:Int,y:Int) : Bool
    {
        var helper = new TransitionHelper(this, x, y);
        
        if(helper.checkIfNotMovingAndCloseEnough() == false) return helper.sendUpdateToClient();

        helper.swapHandAndFloorObject();            
        
        return helper.sendUpdateToClient();
    }   
}

private class TransitionHelper{

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

    public var doAction:Bool;

    public function new(player:GlobalPlayerInstance, x:Int,y:Int)
    {
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

        // ObjectHelpers are for storing advanced dato like USES, CREATION TIME, OWNER
        this.tileObjectHelper = Server.server.map.getObjectHelper(tx,ty);

        trace("hand: " + this.handObject + " tile: " + this.tileObject + ' tx: $tx ty:$ty');
    }

    public function use() : Bool
    {
        // TODO check pickup age

        // TODO kill deadlyDistance

        // TODO feed baby

        // TODO last transitions

        // TODO Pile transitions
        
        if(this.checkIfNotMovingAndCloseEnough() == false) return false;

        // do actor + target = newActor + newTarget
        if(this.doTransitionIfPossible()) return true;

        // do nothing if tile Object is empty
        if(this.tileObject[0] == 0) return false;

        // do pickup if hand is empty
        if(this.handObject[0] == 0 && this.swapHandAndFloorObject()) return true;            
        
        // do container stuff
        return this.placeObjectInContainerOnGroundIfPossible();
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
        // TODO lastUseActorObject
        var lastUseActorObject = false;
        var lastUseTileObject = false;

        if(this.tileObjectHelper != null && this.tileObjectHelper.numberOfUses <= 2){
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




        // TODO Create HelperObject if newTargetObject has time transitions
        
        // create advanced object if USES > 0
        if(this.tileObjectHelper == null && newTargetObjectData.numUses > 0)
        {
            this.tileObjectHelper = new ObjectHelper(newTargetObjectData, this.player);
            Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);
            
            // a Pile starts with 2 uses not with the full
            // if the ObjectHelper is created through a reverse use, it must be a pile...
            if(transition.reverseUseTarget){
                trace("NEW PILE?");
                this.tileObjectHelper.numberOfUses = 2;
            } 

            trace('NEW OBJECT: numberOfUses: ' + this.tileObjectHelper.numberOfUses);

        }
        else{
            this.tileObjectHelper.objectData = newTargetObjectData; // ??? not sure if this is good 

            if(transition.reverseUseTarget)
            {
                this.tileObjectHelper.numberOfUses += 1;
                trace('numberOfUses: ' + this.tileObjectHelper.numberOfUses);
            } 
            else
            {
                this.tileObjectHelper.numberOfUses -= 1;

                trace("REMOVE ObjectHelper USES < 2");
                trace('numberOfUses: ' + this.tileObjectHelper.numberOfUses);

                if(this.tileObjectHelper.numberOfUses < 2) {
                    this.tileObjectHelper = null;
                    Server.server.map.setObjectHelper(tx,ty, this.tileObjectHelper);
                }
            }
        }

        this.doAction = true;
        return true;
    }

    public function swapHandAndFloorObject():Bool{

        var objectData = Server.objectDataMap[this.tileObject[0]];
        //trace("OD: " + objectData.toFileString());

        var permanent = (objectData != null) && (objectData.permanent == 1);

        if(permanent) return false;

        this.newTileObject = this.handObject;
        this.newHandObject = this.tileObject;

        this.doAction = true;
        return true;
    }

    public function placeObjectInContainerOnGroundIfPossible() : Bool {
        var objectData = Server.objectDataMap[this.tileObject[0]];

        trace("containable: " + objectData.containable + " desc: " + objectData.description + " numSlots: " + objectData.numSlots);

        // dont continue if tileObject is a container or if there is no space in it 
        if ((objectData.numSlots == 0 || MapData.numSlots(this.tileObject) >= objectData.numSlots)) return false;
        
        var handObjectData = Server.objectDataMap[this.handObject[0]];

        //if (handObjectData.slotSize >= objectData.containSize) {
        if (handObjectData.slotSize > objectData.containSize) return false;
        this.handObject = MapData.toContainer(handObject);
        this.newTileObject = this.tileObject.concat(this.handObject);
        this.newHandObject = [0];

        this.doAction = true;
        return true;
    }

    public function sendUpdateToClient() : Bool{

        // even send Player Update / PU if nothing happend. Otherwise client will get stuck
        if(this.doAction == false){
            player.connection.send(PLAYER_UPDATE,[player.toData()]);
            player.connection.send(FRAME);
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
            if(this.doAction) c.sendMapUpdate(x, y, this.newFloorId, this.newTileObject, player.p_id);
            c.send(FRAME);
        }

        player.action = 0;

        return true;
    }
}