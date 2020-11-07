package openlife.server;
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

    public function use(x:Int,y:Int):Void{
        player.use(x,y);
    }
    public function drop(x:Int,y:Int):Void{
        player.drop(x,y);
    }

    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = server.dataVersionNumber;
        send(SERVER_INFO,["0/0",challenge,'$version']);
    }
    public function update()
    {
        
    }
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

        var tgx = player.gx + x;
        var tgy = player.gy - y;

        trace("x: " + x);
        trace("y: " + y);
        trace("gx: " + tgx);
        trace("gy: " + tgy);

        x -= Std.int(width / 2);
        y -= Std.int(height / 2);
              
        // x + gx and y + gy are exactly the pixel location in the map image
        var map = server.map.getChunk(x + player.gx, player.gy - y - (height-1), width, height).toString();
        var uncompressed = Bytes.ofString(map);
        var bytes = haxe.zip.Compress.run(uncompressed,-1);

       
        
        send(MAP_CHUNK,['$width $height $x $y','${uncompressed.length} ${bytes.length}']);
        sock.output.write(bytes);
        //send(VALLEY_SPACING,["40 40"]); // TODO what is this for?
        send(FRAME);
    }
    
    public function login()
    {
        send(ACCEPTED);
        server.connections.push(this);

        player = new GlobalPlayerInstance([]);
        player.connection = this;
        var id = server.index++;
        player.p_id = id;
        player.gx = 400;
        player.gy = 400;

        sendMapChunk(0,0);

        var data:Array<String> = [];//[player.toData()];
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
        sock.output.writeString(string);
    }
}
#end