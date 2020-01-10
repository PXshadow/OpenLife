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
    var version:Int = 0;
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
        socket.bind(new Host("localhost"),8005);
        socket.listen(10);
        //accept and add sockets
        while (true)
        {
            try {
                addSocket(socket.accept());
            } catch (e:Dynamic)
            {
                trace('e accept: $e');
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
        send(c,'$SERVER_INFO\n$current/$max\n$challenge\n$version\n');
        clients.push(c);
        Thread.create(function()
        {
            while (true)
            {
                try {
                    trace("input: " + socket.input.readAll().toString());
                }catch(e:Dynamic)
                {
                    if (Std.is(e, haxe.io.Eof) || Std.is(e, haxe.io.Error))
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
    private function loop(c:Client)
    {

    }
    
}