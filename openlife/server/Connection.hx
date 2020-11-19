package openlife.server;
import openlife.data.map.MapData;
import format.swf.Data.PlaceObject;
#if (target.threaded)
import openlife.data.Pos;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.player.PlayerMove;
import openlife.client.ClientTag;
import sys.net.Socket;
import haxe.io.Bytes;

class Connection implements ServerHeader
{
    public var running:Bool = true;
    var sock:Socket;
    var server:Server;
    var tag:ServerTag;
    public var player:GlobalPlayerInstance;

    //public function get_player():GlobalPlayerInstance{return player;}

    /* public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>):Void{
        if(player == null){
            return;
        }

        // TODO some how it throws nulls no clue why. maybe so or so better to directly call functions on player???
        trace("player" + moves);
        trace("player" + seq);
        player.move(x,y,seq,moves);
    } */
/*
    public function use(x:Int,y:Int):Void{
        player.use(x,y);
    }
    public function drop(x:Int,y:Int):Void{
        player.drop(x,y);
    }
*/
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = server.dataVersionNumber;
        send(SERVER_INFO,["0/0",challenge,'$version']);
    }

    public function login()
        {
            send(ACCEPTED);
            server.connections.push(this);
    
            player = new GlobalPlayerInstance([]);
            
            player.connection = this;
            var id = server.index++;
            player.p_id = id;
            player.gx = 360;
            player.gy = 600 - 400; // server map is saved y inverse 
            player.move_speed = server.map.getBiomeSpeed(player.gx, player.gy) * PlayerInstance.initial_move_speed;
    
            Server.server.map.setObjectId(player.gx, player.gy, [33]);
            Server.server.map.setObjectId(player.gx+1, player.gy, [32]);
            Server.server.map.setObjectId(player.gx+2, player.gy, [486]);
            Server.server.map.setObjectId(player.gx+3, player.gy, [486]);
            Server.server.map.setObjectId(player.gx+4, player.gy, [677]);
            Server.server.map.setObjectId(player.gx+5, player.gy, [684]);
            Server.server.map.setObjectId(player.gx+6, player.gy, [677]);

            // add some clothing for testing
            Server.server.map.setObjectId(player.gx, player.gy+1, [2916]);
            Server.server.map.setObjectId(player.gx+1, player.gy+1, [2456]);
            Server.server.map.setObjectId(player.gx+2, player.gy+1, [766]);
            Server.server.map.setObjectId(player.gx+3, player.gy+1, [2919]);
            Server.server.map.setObjectId(player.gx+4, player.gy+1, [198]);
            Server.server.map.setObjectId(player.gx+5, player.gy+1, [2886]);
            Server.server.map.setObjectId(player.gx+6, player.gy+1, [586]);
            Server.server.map.setObjectId(player.gx+7, player.gy+1, [2951]);


            //test transitions of numUses + decay
            Server.server.map.setObjectId(player.gx,player.gy + 10,[238]);
            Server.server.map.setObjectId(player.gx,player.gy + 11,[1599]);

            //containers testing SREMV
            Server.server.map.setObjectId(player.gx - 4,player.gy + 10,[434]);
            Server.server.map.setObjectId(player.gx - 5,player.gy + 10,[292,2143,2143,2143]);
            Server.server.map.setObjectId(player.gx - 6,player.gy + 10,[292,2143,2143,2143]);
            Server.server.map.setObjectId(player.gx - 7,player.gy + 10,[292,33,2143,33]);
            Server.server.map.setObjectId(player.gx - 8,player.gy + 10,[2143,2143,2143]);
            Server.server.map.setObjectId(player.gx - 7,player.gy + 10,[3371,33,2143,33]);
            

            
            trace("move_speed: " + player.move_speed);
    
            sendMapChunk(0,0);
    
            var data:Array<String> = [];
            for (c in server.connections)
            {
                data.push(c.player.toData());
                if (c != this)
                {
                    c.send(PLAYER_UPDATE,[player.toData()]);
                    c.send(FRAME);
                }
            }
            send(PLAYER_UPDATE,data);
            send(FRAME);
            send(LINEAGE,['$id eve=$id']);
        }

    /*
    public function update()
    {
        player.handleUpdate();


        //if(server.tick % 20 == 0){
            //trace("Ticks: " + server.tick);
            // TODO needs to calculate the player position first
            // player.sendSpeedUpdate(this);
            //this.send(FRAME);
        //}
    }
    */

    public function close()
    {
        running = false;
        sock.close();
        server.connections.remove(this);
    }
    
    public function keepAlive()
    {

    }

    public function die()
    {
        server.connections.remove(this);
        sock.close();
    }

    public function say(text:String)
    {
        var curse = 0;
        var id = player.p_id;
        for (c in server.connections)
        {
            c.send(PLAYER_MOVES_START,[
                "p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN",
                "p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ... xdeltN ydeltN",
            ]);
            c.send(PLAYER_SAYS,['$id/$curse $text']);
            c.send(FRAME);
        }
    }

    public function flip()
    {
        
    }   

    public function sendMapChunk(x:Int,y:Int,width:Int = 32,height:Int = 30)
    {
        x -= Std.int(width / 2);
        y -= Std.int(height / 2);
              
        var map = server.map.getChunk(x + player.gx, y + player.gy, width, height).toString();
        var uncompressed = Bytes.ofString(map);
        var bytes = haxe.zip.Compress.run(uncompressed,-1);
        
        send(MAP_CHUNK,['$width $height $x $y','${uncompressed.length} ${bytes.length}']);
        sock.output.write(bytes);
        //send(VALLEY_SPACING,["40 40"]); // TODO what is this for?
        //send(FRAME);
    }

    /*
    MX
    x y new_floor_id new_id p_id
    #

    Or 

    MX
    x y new_floor_id new_id p_id old_x old_y speed
    #
    */
    public function sendMapUpdate(x:Int, y:Int, newFloorId:Int, newObjectId:Array<Int>, playerId:Int)
    {
        send(MAP_CHANGE,['$x $y $newFloorId ${MapData.stringID(newObjectId)} $playerId']);
    }
    
    public function emote(id:Int)
    {
        for (c in server.connections)
        {
            c.send(FRAME);
            c.send(PLAYER_EMOT,['${player.p_id} $id']);
        }
    }
    
    public function rlogin()
    {
        login();
    }

    public function send(tag:ClientTag,data:Array<String>=null)
    {
        var string = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';
        //trace("S: " + string);
        sock.output.writeString(string);
    }
}
#end