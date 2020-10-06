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
    var player:PlayerInstance;
    var gx:Int = 0; //global x offset
    var gy:Int = 0; //global y offset
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = "350";
        send(SERVER_INFO,["0/0",challenge,version]);
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
    private function moveString(moves:Array<Pos>):String
    {
        var string = "";
        for (m in moves) string += " " + m.x + " " + m.y;
        return string.substr(1);
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
            c.send(PLAYER_SAYS,['$id/$curse $text']);
            c.send(FRAME);
        }
    }
    public function flip()
    {
        
    }
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>)
    {
        var total = (1/player.move_speed) * moves.length;
        trace("eta " + total);
        var eta = total;
        var trunc = 0;
        var last = moves.pop();
        player.x += last.x;
        player.y += last.y;
        moves.push(last);
        
        for (c in server.connections) 
        {
            c.send(PLAYER_MOVES_START,['${player.p_id} $x $y $total $eta $trunc ${moveString(moves)}']);
            c.send(PLAYER_UPDATE,[player.toData()]);
            c.send(FRAME);
        }
    }
    public function login()
    {
        send(ACCEPTED);
        server.connections.push(this);
        var map = server.map.toString();
        var uncompressed = Bytes.ofString(map);
        var bytes = haxe.zip.Compress.run(uncompressed,-1);
        //return;
        gx = 16;
        gy = 15;
        send(MAP_CHUNK,["32 30 -16 -15",'${uncompressed.length} ${bytes.length}']);
        sock.output.write(bytes);
        send(VALLEY_SPACING,["40 40"]);
        player = new PlayerInstance([]);
        var id = server.index++;
        player.p_id = id;
        player.o_id = [33];
        send(FRAME);
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
    public function use(x:Int,y:Int)
    {
        player.action = 1;
        trace("USE " + x + " " + y);
        player.o_id = server.map.get(x + gx,y + gy,true);
        player.forced = true;
        player.o_origin_x = x;
        player.o_origin_y = y;
        player.o_origin_valid = 1;
        player.action_target_x = x;
        player.action_target_y = y;
        for (c in server.connections)
        {
            c.send(PLAYER_UPDATE,[player.toData()]);
            c.send(FRAME);
        }
        player.action = 0;
        player.forced = false;
    }
    public function drop(x:Int,y:Int)
    {
        player.o_id = [0];
        player.action = 1;
        player.action_target_x = x;
        player.action_target_y = y;
        for (c in server.connections)
        {
            c.send(PLAYER_UPDATE,[player.toData()]);
            c.send(FRAME);
        }
        player.action = 0;
        player.forced = false;
    }
    public function rlogin()
    {
        login();
    }
    public function send(tag:ClientTag,data:Array<String>=null)
    {
        var string = data != null ? '$tag\n${data.join("\n")}\n#' : '$tag\n#';
        sock.output.writeString(string);
        //trace(string);
    }
}
#end