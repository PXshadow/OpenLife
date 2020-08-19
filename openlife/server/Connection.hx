package openlife.server;

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
    public function move(x:Int,y:Int,seq:Int,moves:Array<Pos>)
    {
        trace("moves " + moves);
        var total = 0.267;
        var eta = total;
        var trunc = 0;
        var last = moves.pop();
        player.x += last.x;
        player.y += last.y;
        moves.push(last);
        send(FRAME);
        send(PLAYER_MOVES_START,['${player.p_id} $x $y $total $eta $trunc ${moveString(moves)}']);
        send(FRAME);
        send(PLAYER_UPDATE,[player.toData()]);
        send(FRAME);
    }
    public function login()
    {
        send(ACCEPTED);
        server.connections.push(this);
        var map = "";
        for (i in 0...30 * 32) map += '4:0:0 ';
        map = map.substring(0,map.length - 1);
        var uncompressed = Bytes.ofString(map);
        var bytes = haxe.zip.Compress.run(uncompressed,-1);
        trace("un " + uncompressed.length + " compressed " + bytes.length);
        //return;
        send(MAP_CHUNK,["32 30 -16 -15",'${uncompressed.length} ${bytes.length}']);
        sock.output.write(bytes);
        send(VALLEY_SPACING,["40 40"]);
        player = new PlayerInstance([]);
        var id = 1;
        player.p_id = id;
        send(FRAME);
        send(PLAYER_UPDATE,[player.toData()]);
        send(FRAME);
        send(LINEAGE,['$id eve=$id']);
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