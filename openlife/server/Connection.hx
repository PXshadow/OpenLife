package openlife.server;

import openlife.client.ClientTag;
import sys.net.Socket;
import haxe.io.Bytes;

class Connection
{
    public var running:Bool = true;
    var sock:Socket;
    var server:Server;
    var tag:ServerTag;
    public function new(sock:Socket,server:Server)
    {
        this.sock = sock;
        this.server = server;
        var challenge = "dsdjsiojdiasjiodsjiosd";
        var version = "350";
        send(SERVER_INFO,["0/0",challenge,version]);
    }
    public function close()
    {
        running = false;
        sock.close();
    }
    public function message(data:String)
    {
        trace('recieved: $data');
        var array = data.split(" ");
        if (array.length == 0) return;
        tag = array[0];
        var input = array.slice(1,array.length > 2 ? array.length - 1 : array.length);
        switch (tag)
        {
            case LOGIN | RLOGIN:
            //sock.output.writeString('$REJECTED\n#');
            sock.output.writeString('$ACCEPTED\n#');
            var map = "";
            for (i in 0...30 * 32) map += '4:0:30 ';
            map = map.substring(0,map.length - 1);
            var uncompressed = Bytes.ofString(map);
            var bytes = haxe.zip.Compress.run(uncompressed,-1);
            trace("un " + uncompressed.length + " compressed " + bytes.length);
            //return;
            send(MAP_CHUNK,["32 30 -16 -15",'${uncompressed.length} ${bytes.length}']);
            sock.output.write(bytes);
            send(VALLEY_SPACING,["40 40"]);
            var player = new openlife.data.object.player.PlayerInstance([]);
            var id = 1;
            player.p_id = id;
            send(PLAYER_UPDATE,[player.toData()]);
            send(LINEAGE,['$id eve=$id']);
            sock.output.writeString('$FRAME\n#');
            //send(PLAYER_UPDATE,["217055 19 0 0 0 0 0 0 0 0 -1 0.50 1 0 0 0 14.00 60.00 3.75 0;0;0;0;0;0 0 0 -1 0"]);
            default:
            trace('tag not found $tag');
        }
    }
    private function send(tag:ClientTag,data:Array<String>)
    {
        var string = '$tag\n${data.join("\n")}\n#';
        sock.output.writeString(string);
        trace(string);
    }
}