package openlife.server;
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

        var tile_o_id = this.o_id;
        trace("USE: o_id: " + tile_o_id);

        var objectID = Server.server.map.getObjectId(tx, ty);

        var doaction = true;

        if(objectID[0] != 0){
            var objectData = Server.objectDataMap[objectID[0]];
            trace("OD: " + objectData.toFileString());

            if(objectData.permanent != 0) doaction = false;
        }
        
        // TODO check pickup age
        // TODO check if pickup is possible
        // TODO add transitions

        //deadlyDistance

        var newFloorId = Server.server.map.getFloorId(tx, ty);

        if(doaction){

            Server.server.map.setObjectId(tx, ty, tile_o_id);
        
            this.o_id = objectID;
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