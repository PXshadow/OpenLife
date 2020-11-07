package openlife.server;
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;

class GlobalPlayerInstance extends PlayerInstance {
    public var connection:Connection; 

    public var gx:Int = 430; //global x offset from birth
    public var gy:Int = 440; //global y offset from birth

    private var tx:Int = 0;
    private var ty:Int = 0;

    public function new(a:Array<String>)
    {
        super(a);
    }

    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>)
    {
        trace("connection " + Server.server);
        var total = (1/this.move_speed) * moves.length;
        var eta = total;
        var trunc = 0;
        var last = moves.pop(); 
        //this.x += last.x; // TODO check if client movement is valid
        //this.y += last.y;
        this.x = x;
        this.y = y;


        moves.push(last);
/*
        var tgx = this.gx + this.x;
        var tgy = this.gy - this.y;

        trace("x: " + this.x);
        trace("y: " + this.y);
        trace("tx: " + this.tx);
        trace("ty: " + this.ty);
        trace("gx: " + tgx);
        trace("gy: " + tgy);
*/
        // TODO spacing / chunk loading in x direction is too slow with high speed
        var spacing = 4;

        if(this.x - tx> spacing || this.x - tx < -spacing || this.y - ty > spacing || this.y - ty < -spacing ) {          
            //trace("new chunk");
            
            this.tx = x;
            this.ty = y;

            connection.sendMapChunk(x,y);
        }
        
        for (c in Server.server.connections) 
        {
            var speed = PlayerInstance.initial_move_speed * Server.server.map.getBiomeSpeed(this.x + gx, this.y + gy);

            trace("speed: " + speed);

            //this.move_speed = 10;

            this.move_speed = speed;
            
            // TODO place sending logic in connection???
            c.send(PLAYER_MOVES_START,['${this.p_id} $x $y $total $eta $trunc ${moveString(moves)}']);
            c.send(PLAYER_UPDATE,[this.toData()]);
            c.send(FRAME);
        }
    }

    private function moveString(moves:Array<Pos>):String
        {
            var string = "";
            for (m in moves) string += " " + m.x + " " + m.y;
            return string.substr(1);
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