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
        sock.output.writeString('$REJECTED\n#');
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
            case LOGIN:
            sock.output.writeString('$ACCEPTED\n#');
            var map = "";
            for (i in 0...30 * 32) map += "3:0:0 ";
            map = map.substring(0,map.length - 1);
            var uncompressed = Bytes.ofString(map);
            trace("uncomp " + uncompressed);
            var ucl:Int = uncompressed.length;
            var bytes:Bytes = haxe.zip.Compress.run(uncompressed,0);
            var length:Int = bytes.length;
            send(MAP_CHUNK,["32 30 -16 -15",'$ucl $length']);
            sock.output.write(bytes);
            send(VALLEY_SPACING,["40 40"]);
            send(LINEAGE,["217004 eve=217004"]);
            var player = new openlife.data.object.player.PlayerInstance([]);
            player.p_id = 1;
            send(PLAYER_UPDATE,[player.toData()]);
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