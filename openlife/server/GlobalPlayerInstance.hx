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
        trace("USE: hand_o_id: " + hand_o_id);

        var tile_o_id = Server.server.map.getObjectId(tx, ty);

        var doaction = true;

        if(tile_o_id[0] != 0){
            
            var transition = Server.transitionImporter.getTransition(hand_o_id[0], tile_o_id[0]);

            if(transition != null){

                trace('Found transition: a${transition.actorID} t${transition.targetID}');
                //for(trans in transitions){
                //    trace(trans.actorID);
                //}

                hand_o_id = [transition.newActorID];
                tile_o_id = [transition.newTargetID];

                // transition source object id (or -1) if held object is result of a transition,
                // this.o_transition_source_id

                //doaction = true;
            }
            else{
                var objectData = Server.objectDataMap[tile_o_id[0]];
                //trace("OD: " + objectData.toFileString());

                if(objectData.permanent != 0) doaction = false;
                else{
                    var tmp = hand_o_id;
                    hand_o_id = tile_o_id;
                    tile_o_id = tmp;
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
            this.action_target_x = x;
            this.action_target_y = y;
            this.forced = false;

        }

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            if(doaction) c.sendMapUpdate(x,y,newFloorId, tile_o_id[0], this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
        
        //this.o_origin_valid = 0;
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
        this.o_origin_x = 0;
        this.o_origin_y = 0;
        this.o_origin_valid = 0;
        this.action_target_x = x;
        this.action_target_y = y;
        this.forced = false;

        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            c.sendMapUpdate(x,y,newFloorId, tile_o_id[0], this.p_id);
            c.send(FRAME);
        }

        this.action = 0;
    }
}