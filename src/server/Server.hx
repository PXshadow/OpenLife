package server;

import client.ClientTag;
import sys.thread.Thread;
import haxe.Timer;
import server.ServerTag;
import sys.db.Sqlite;
import haxe.io.Bytes;
import sys.net.Socket;
import settings.Settings;
import sys.net.Host;
import sys.FileSystem;
import sys.io.File;
import server.ThreadServer;
import haxe.io.Path;

class Server extends ThreadServer
{
    public static function main()
    {
        new Server();
    }
    public function new()
    {
        super();
        create();
    }
    override function connect(socket:Socket) {
        super.connect(socket);
        //send(c,'$SERVER_INFO\n$current/$max\n$challenge\n$version');
    }
}
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