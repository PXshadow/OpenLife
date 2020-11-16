package openlife.server;
import openlife.data.transition.TransitionData;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import sys.thread.Mutex;

using openlife.server.MoveExtender;

class GlobalPlayerInstance extends PlayerInstance {
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

    

    public function use(x:Int,y:Int)
    {
        if(me.isMoveing()) {
            trace("USE: Player is still moving");
            return; 
        }

        if(this.isClose(x,y) == false) {
            trace('USE: object position is too far away p${this.x},p${this.y} o$x,o$y');
            return; 
        }

        var tx = x + gx;
        var ty = y + gy;

        var hand_o_id = this.o_id;
        var transitionSource = hand_o_id[0];

        trace("USE: hand_o_id: " + hand_o_id);

        var tile_o_id = Server.server.map.getObjectId(tx, ty);
        trace("USE: tile_o_id: " + tile_o_id);


        var doaction = false;
        trace("hand " + hand_o_id + " tile " + tile_o_id);

        if(tile_o_id[0] != 0){

            var transition = Server.transitionImporter.getTransition(hand_o_id[0], tile_o_id[0]);
            if(transition != null){

                trace('Found transition: a${transition.actorID} t${transition.targetID}');

                //transition source object id (or -1) if held object is result of a transition,
                if(transition.newActorID != hand_o_id[0]) transitionSource = -1;

                hand_o_id = [transition.newActorID];
                tile_o_id = [transition.newTargetID];

                doaction = true;
            }else{
                var objectData = Server.objectDataMap[tile_o_id[0]];
                //trace("OD: " + objectData.toFileString());

                var permanent = (objectData != null) && objectData.permanent == 1;
                // switch only if object not permanent and hand or tile is free
                if(permanent == false && (hand_o_id[0] == 0 || tile_o_id[0] == 0)) {

                    var tmp = hand_o_id;
                    hand_o_id = tile_o_id;
                    tile_o_id = tmp;

                    doaction = true;
                    
                }else{
                    trace("containable " + objectData.containable + " desc " + objectData.description + " numSlots " + objectData.numSlots);
                    if (objectData.numSlots > 0) { //TODO: MapData needs to give a correct numSlots back
                        var handObject = Server.objectDataMap[hand_o_id[0]];
                        if (handObject.slotSize >= objectData.containSize) {
                            tile_o_id = tile_o_id.concat(hand_o_id);
                            hand_o_id = [0];
                            doaction = true;
                        }
                    }
                }
            }
        }
        
        // TODO check pickup age

        // TODO kill deadlyDistance

        // TODO change movement speed

        // TODO feed baby

        // TODO floor

        // TODO last transitions

        var newFloorId = Server.server.map.getFloorId(tx, ty);

        if(doaction){

            Server.server.map.setObjectId(tx, ty, tile_o_id);
        
            this.o_id = hand_o_id;
            this.action = 1;
            this.o_origin_x = x;
            this.o_origin_y = y;
            this.o_origin_valid = 0;
            this.o_transition_source_id = transitionSource;
            this.action_target_x = x;
            this.action_target_y = y;
            this.forced = false;

        }

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            if(doaction) c.sendMapUpdate(x,y,newFloorId, tile_o_id, this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
        
        //this.o_origin_valid = 0;
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
    }

    public function specialRemove(x:Int,y:Int,clothing:Int,id:Null<Int>)
    {

    }
    public function drop(x:Int,y:Int)
    {
        if(me.isMoveing()) {
            trace("DROP: Player is still moving");
            return; 
        }

        if(this.isClose(x,y) == false) {
            trace('DROP: object position is too far away p${this.x},p${this.y} o$x,o$y');
            return; 
        }

        var tx = x + gx;
        var ty = y + gy;

        var newFloorId = Server.server.map.getFloorId(tx, ty);

        var tile_o_id = this.o_id;
        trace("DROP: o_id: " + tile_o_id);

        this.o_id = Server.server.map.getObjectId(tx, ty);
        Server.server.map.setObjectId(tx, ty, tile_o_id);
        
        this.action = 1;
        this.o_origin_x = x;
        this.o_origin_y = y;
        this.o_origin_valid = 0;
        this.action_target_x = x;
        this.action_target_y = y;
        this.forced = false;
        
        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            c.sendMapUpdate(x,y,newFloorId, tile_o_id, this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
    }
}