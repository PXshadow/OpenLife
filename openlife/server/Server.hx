package openlife.server;
import openlife.data.transition.TransitionImporter;
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
import openlife.data.object.ObjectData;

using openlife.server.MoveExtender;

class Server
{
    public static var server:Server; 
    public static var tickTime = 1 / 20;
    public static var vector:Vector<ObjectData>;
    public static var objectDataMap:Map<Int, ObjectData> = [];
    public static var transitionMap:Map<Int, ObjectData> = [];

    public static var transitionImporter:TransitionImporter = new TransitionImporter();


    public var connections:Array<Connection> = [];
   
    public var tick:Int = 0;
    public var index:Int = 1;
    public var map:WorldMap;
    
    public var dataVersionNumber:Int = 0;

    public static function main()
    {
        Sys.println("Starting OpenLife Server"#if debug + " in debug mode" #end);
        server = new Server();
        while (true)
        {
            @:privateAccess haxe.MainLoop.tick();
            @:privateAccess server.update();
            server.tick++;
            Sys.sleep(tickTime);
        }
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
        /*var row = MapTable.manager.select($p_id == 30);
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
        trace("insert");*/

        Engine.dir = Utility.dir();
        var tmp = ObjectBake.objectList();
        vector = new Vector<ObjectData>(tmp.length);



        objectDataMap = [];

        for (i in 0...vector.length){
            var objectData = new ObjectData(tmp[i]);
            vector[i] = objectData;
            objectDataMap[objectData.id] = objectData;
        }

        transitionImporter.importTransitions();

        dataVersionNumber = Resource.dataVersionNumber();
        trace('dataVersionNumber: $dataVersionNumber');
        map = new WorldMap();
        map.generate();
        trace("length " + vector.length);
        var thread = new ThreadServer(this,8005);
        Thread.create(function()
        {
            thread.create();
        });
    }

    private function update()
    {
        //if(this.tick % 20 == 0) trace("Ticks: " + this.tick);

        for (connection in connections)
        {
            connection.player.updateMovement();
        }
    }

    public function process(connection:Connection,string:String)
    {
        //Sys.println(string); //log messages
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
            header.player.use(Std.parseInt(input[0]), Std.parseInt(input[1]));
            case SELF:
            header.player.self(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]));
            case DROP:
            header.player.drop(Std.parseInt(input[0]), Std.parseInt(input[1]));
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