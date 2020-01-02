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

typedef Message = {
    tag:ServerTag,
    input:String,
}

class Server extends cpp.net.ThreadServer<Client,Message>
{
    var dir:String;
    var num:Int = 0;
    var current:Int = 0;
    var max:Int = 200;
    var version:Int = 0;
    var challenge = "sdfmlk3490sadfm3ug9324";
    var settings:Settings;
    public static function main()
    {
        trace("Server starting up");
        trace("Server using version " + 0);
        var server = new Server();
        Thread.create(function()
        {
            server.run("localhost",8005);
        });
        trace("Client starting up");
        Sys.sleep(1);
        var client = new client.Client();
        client.accept = function()
        {
            trace("accept");
            client.message = message;
            client.accept = null;
        }
        client.reject = function()
        {
            trace("reject");
            client.reject = null;
        }
        client.message = client.login;
        client.connect();
        while (true)
        {
            client.update();
            Sys.sleep(0.05);
        }
    }
    public static function message(tag:ClientTag,input:String)
    {
        trace('$tag $input');
    }
    public function new()
    {
        super();
        maxBufferSize = 200 * 8 * 2;
        maxSockPerThread = 200;

        dir = Path.addTrailingSlash(Path.directory(Path.normalize(Sys.programPath())));
        trace("dir " + dir);

        Manager.initialize();
        Manager.cnx = Sqlite.open(dir + "database.db");
        //create tables
        if (!TableCreate.exists(server.logs.Life.manager)) TableCreate.create(server.logs.Life.manager);

        trace("cnx " + Manager.cnx);
    }
    override function clientConnected(s:Socket):Client 
    {
        trace("new client connected");
        var c:Client = {id: -1,socket: s};
        send(c,'$SERVER_INFO\n$current/$max\n$challenge\n$version\n');
        return c;
    }
    override function clientDisconnected(c:Client) 
    {
        trace("client " + c.id + " disconnected");
    }
    private function send(c:Client,data:String)
    {
        trace(c.socket.peer().host + " send " + data + "#");
        c.socket.output.writeString(data + "#");
        c.socket.output.flush();
    }
    override function readClientMessage(c:Client, buf:Bytes, pos:Int, len:Int):{msg:Message, bytes:Int} {
        var complete = false;
        var cpos = pos;
        while (cpos <  (pos + len) && !complete)
        {
            complete = (buf.get(cpos++) == "#".code);
        }
        if (!complete) return null;
        var data = buf.getString(pos,cpos-pos);
        trace("data " + data);
        return null;
    }
    
}