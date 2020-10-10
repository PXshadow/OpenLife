package openlife.server;
import openlife.server.tables.MapTable;
import sys.db.TableCreate;
import sys.db.Manager;
import openlife.resources.Resource;
#if (target.threaded)
import openlife.engine.Utility;
import openlife.engine.Engine;
import openlife.resources.ObjectBake;
import haxe.ds.Vector;
import openlife.data.Pos;
import openlife.client.ClientTag;
import sys.thread.Thread;
import haxe.Timer;
import haxe.io.Bytes;
import sys.net.Socket;
import openlife.settings.Settings;
import sys.net.Host;
import sys.FileSystem;
import sys.io.File;
import haxe.io.Path;

class Server
{
    public static var server:Server; 

    public var connections:Array<Connection> = [];
    var tick:Int = 0;
    public var index:Int = 1;
    public var map:Map;
    public var vector:Vector<Int>;
    public var dataVersionNumber:Int = 0;
    public static function main()
    {
        server = new Server();
    }
    public function new()
    {
        //initalize database
        Manager.initialize();
        Manager.cnx = sys.db.Sqlite.open("server.db");
        if (!TableCreate.exists(MapTable.manager))
        {
            TableCreate.create(MapTable.manager);
        }

        var row = MapTable.manager.select($p_id == 30,null);
        trace("row: " + row);
        if (row != null)
        {
            row.timestamp = Date.now();
            row.update();
        }
        var row = new MapTable();
        row.o_id = [0];
        row.p_id = 30;
        row.timestamp = Date.now();
        row.insert();
        trace("insert");

        Engine.dir = Utility.dir();
        vector = ObjectBake.objectList();
        dataVersionNumber = Resource.dataVersionNumber();
        trace('dataVersionNumber: $dataVersionNumber');
        map = new Map(this);
        trace("length " + vector.length);
        var thread = new ThreadServer(this,8005);
        Thread.create(function()
        {
            thread.create();
        });
        while (true)
        {
            @:privateAccess haxe.MainLoop.tick();
            update();
            tick++;
            Sys.sleep(1/20);
        }
    }
    private function update()
    {
        for (connection in connections)
        {
            connection.update();
        }
    }
    public function process(connection:Connection,string:String)
    {
        Sys.println(string); //log messages
        var index = string.indexOf(" ");
        if (index == -1) return;
        var tag = string.substring(0,index);
        string = string.substring(index + 1);
        var array = string.split(" ");
        if (array.length == 0) return;
        message(connection,tag,array,string);
    }
    private function message(header:ServerHeader,tag:ServerTag,input:Array<String>,string:String)
    {
        switch (tag)
        {
            case LOGIN:
            header.login();
            case RLOGIN:
            header.rlogin();
            case MOVE:
            var x = Std.parseInt(input[0]);
            var y = Std.parseInt(input[1]);
            var seq = Std.parseInt(input[2].substr(1));
            input = input.slice(3);
            var moves:Array<Pos> = [];
            for (i in 0...Std.int(input.length/2))
            {
                moves.push(new Pos(Std.parseInt(input[i * 2]),Std.parseInt(input[i * 2 + 1])));
            }
            header.player.move(x,y,seq,moves);
            case DIE:
            header.die();
            case KA:
            header.keepAlive();
            case EMOT:
            trace("data " + input);
            header.emote(Std.parseInt(input[2]));
            case USE:
            trace("USE!");
            header.use(Std.parseInt(input[0]),Std.parseInt(input[1]));
            case DROP:
            header.drop(Std.parseInt(input[0]),Std.parseInt(input[1]));
            case SAY:
            var text = string.substring(4);
            header.say(text);
            case FLIP:
            header.flip();
            default:
        }
    }
}
#end