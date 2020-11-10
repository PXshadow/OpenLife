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

    public function use(x:Int,y:Int)
    {
        
        this.o_id = Server.server.map.get(x + gx,y + gy,true);
        this.action = 1;
        this.o_origin_x = x;
        this.o_origin_y = y;
        this.o_origin_valid = 0;
        this.action_target_x = x;
        this.action_target_y = y;
        for (c in Server.server.connections)
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            c.send(FRAME);
        }
        this.action = 0;
        this.forced = false;
        this.o_origin_valid = 0;
        
    }
    public function drop(x:Int,y:Int)
    {
        this.o_id = [0];
        this.action = 1;
        this.o_origin_x = x;
        this.o_origin_y = y;
        this.o_origin_valid = 0;
        this.action_target_x = x;
        this.action_target_y = y;
        for (c in Server.server.connections) // TODO only for visible players
        {
            c.send(PLAYER_UPDATE,[this.toData()]);
            c.send(FRAME);
        }
        this.action = 0;
        this.forced = false;
    }
}