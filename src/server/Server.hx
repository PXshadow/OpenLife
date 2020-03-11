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
import haxe.io.Path;

typedef Client = {
   id:Int,
   socket:Socket,
}

class Server
{
    var dir:String;
    var num:Int = 0;
    var current:Int = 0;
    var max:Int = 200;
    var version:Int = 303;
    var challenge = "sdfmlk3490sadfm3ug9324";
    var settings:Settings;
    var socket:Socket;
    var clients:Array<Client> = [];
    public static function main()
    {
        Thread.create(function(){new Server();});
        Sys.sleep(0.5);
        Main.main();
    }
    public function new()
    {
        dir = Path.addTrailingSlash(Path.directory(Path.normalize(Sys.programPath())));
        trace("dir " + dir);
        //run
        socket = new Socket();
        socket.setBlocking(false);
        socket.bind(new Host("localhost"),8005);
        socket.listen(10);
        //accept and add sockets
        while (true)
        {
            try {
                addSocket(socket.accept());
            } catch (e:Dynamic)
            {
                if (e == haxe.io.Eof)
                {
                    trace('e accept: $e');
                }
            }
            Sys.sleep(1/8);
        }
    }
    private function send(c:Client,data:String)
    {
        c.socket.output.writeString('$data#');
        c.socket.output.flush();
    }
    private function addSocket(socket:Socket)
    {
        socket.setBlocking(false);
        trace("client connected " + socket.host().host.host);
        var c = {id:-1,socket: socket};
        send(c,'$SERVER_INFO\n$current/$max\n$challenge\n$version');
        clients.push(c);
        Thread.create(function()
        {
            while (true)
            {
                try {
                    process(c,socket.input.readUntil("#".code));
                }catch(e:Dynamic)
                {
                    if (e == haxe.io.Eof)
                    {
                        trace('e client: $e');
                        socket.close();
                        clients.remove(c);
                        return;
                    }
                }
                Sys.sleep(1/4);
            }
        });
    }
    private function process(c:Client,data:String)
    {
        //double face fixes the email padding issue
        var array = data.split(" ");
        trace(data);
        trace("array " + array);
        if (array.length == 0) return;
        var tag:ServerTag = array[0];
        message(c,tag,array.slice(1,array.length));
    }
    private function message(c:Client,tag:ServerTag,input:Array<String>)
    {
        function send(tag:ClientTag,data:String="")
        {
            this.send(c,'$tag\n$data');
        }
        switch (tag)
        {
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
        }
    }
}