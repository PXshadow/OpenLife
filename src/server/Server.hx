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
import sys.db.Manager;
import sys.db.TableCreate;
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
    var version:Int = 301;
    var challenge = "sdfmlk3490sadfm3ug9324";
    var settings:Settings;
    var socket:Socket;
    var clients:Array<Client> = [];
    public static function main()
    {
        new Server();
    }
    public function new()
    {
        dir = Path.addTrailingSlash(Path.directory(Path.normalize(Sys.programPath())));
        trace("dir " + dir);

        Manager.initialize();
        Manager.cnx = Sqlite.open(dir + "database.db");
        //create tables
        if (!TableCreate.exists(server.logs.Life.manager)) TableCreate.create(server.logs.Life.manager);
        //run
        socket = new Socket();
        socket.setBlocking(false);
        socket.bind(new Host("localhost"),8005);
        socket.listen(10);
        //accept and add sockets
        while (true)
        {
            try {
                add(socket.accept());
            }catch(e:Dynamic)
            {
                if (e != "Blocking")
                {
                    trace('e accept: $e');
                }
            }
        }
    }
    private function add(s:Socket)
    {
        var c = {id:-1,socket: s};
        clients.push(c);
        //setup reader
        Thread.create(loop).sendMessage(c);
    }
    private function send(c:Client,data:String)
    {
        try {
            c.socket.output.writeString('$data#');
            c.socket.output.flush();
            trace("c " + data);
            //c.socket.output.close();
        }catch(e:Dynamic)
        {
            trace('e: $e');
            close(c);
            return;
        }
        //close(c);
    }
    private function close(c:Client)
    {
        c.socket.close();
        clients.remove(c);
    }
    private function loop()
    {
        var c:Client = cast Thread.readMessage(true);
        c.socket.setBlocking(false);
        var timer = new Timer(1000);
        timer.run = function()
            {
        send(c,'$SERVER_INFO\n$current/$max\n$challenge\n$version\n');
            }
        trace("create client loop");
        while (true)
        {
            try {
                trace("data " + c.socket.input.readAll().toString());
            }catch(e:Dynamic)
            {
                if (Std.is(e, haxe.io.Eof))
                {
                    trace('e client: $e');
                    close(c);
                    return;
                }
            }
            Sys.sleep(1/4);
        }
    }
    
}