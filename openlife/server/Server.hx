package openlife.server;
import openlife.data.Pos;
#if ((target.threaded) && !cs)
import openlife.client.ClientTag;
import sys.thread.Thread;
import haxe.Timer;
import sys.db.Sqlite;
import haxe.io.Bytes;
import sys.net.Socket;
import openlife.settings.Settings;
import sys.net.Host;
import sys.FileSystem;
import sys.io.File;
import haxe.io.Path;

class Server
{
    public static function main()
    {
        new Server();
    }
    public function new()
    {
        new ThreadServer(this,8005);
    }
    public function process(connection:Connection,string:String)
    {
        var array = string.split(" ");
        if (array.length == 0) return;
        var tag = array[0];
        var input = array.slice(1,array.length > 2 ? array.length - 1 : array.length);
        message(connection,tag,input);
    }
    private function message(header:ServerHeader,tag:ServerTag,input:Array<String>)
    {
        switch (tag)
        {
            case LOGIN:
            header.login();
            case RLOGIN:
            header.rlogin();
            case MOVE:
            trace("Input " + input);
            var x = Std.parseInt(input[0]);
            var y = Std.parseInt(input[1]);
            var seq = Std.parseInt(input[2].substr(1));
            input = input.slice(3);
            var moves:Array<Pos> = [];
            for (i in 0...Std.int(input.length/2))
            {
                moves.push(new Pos(Std.parseInt(input[i * 2]),Std.parseInt(input[i * 2 + 1])));
            }
            header.move(x,y,seq,moves);
            case KA:
            header.keepAlive();
            default:
        }
    }
}
#end
/*
case LOGIN:
            send(ACCEPTED);
            //send(MAP_CHUNK,new haxe.zip.Compress(0))
            var data:String = "";
            for (i in 0...32 * 30)
            {
                data += " 0:0:0";
            }
            data = data.substr(1);
            var uncompressed:Bytes = Bytes.ofString(data);
            var ucl:Int = uncompressed.length;
            var bytes:Bytes = haxe.zip.Compress.run(uncompressed,0);
            var length:Int = bytes.length;
            send(MAP_CHUNK,'32 30 0 0\n$ucl $length\n');
            c.socket.output.writeBytes(bytes,0,bytes.length);
            //c.socket.output.writeString(bytes.toString());
            var pu = new data.object.player.PlayerInstance([]).toData();
            trace("pu " + pu);
            send(PLAYER_UPDATE,pu);
            case null:
            trace('tag not found in data: $input');
            case KA:
            //keep alive
            default:
            trace('$tag not registered');
*/