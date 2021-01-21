package openlife.server;
import openlife.auto.MessageHandler;
import openlife.auto.Ai;
import openlife.settings.ServerSettings;
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

using openlife.server.MoveHelper;

class Server
{
    public static var server:Server; 
      
    public static var transitionMap:Map<Int, ObjectData> = [];
    public static var transitionImporter:TransitionImporter = new TransitionImporter();

    public var map:WorldMap; // THE WORLD
    
   

    public var playerIndex:Int = 2; // used for giving new IDs to players // better start with 2 since -1 has other use in MX update
    
    
    public var dataVersionNumber:Int = 0;

    public static function main()
    {
        Sys.println("Starting OpenLife Server"#if debug + " in debug mode" #end);

        if(ServerSettings.debug) trace('Debug Mode: ${ServerSettings.debug}');

        server = new Server();

        // add a new test bot // TODO remove later
        var ai = ServerAi.CreateNew();
        ai.myPlayer.age = 1;

        TimeHelper.DoTimeLoop();
    }

    public function new()
    {
        //SerializeHelper.createReadWriteFile();
        
        if(ServerSettings.readFromFile() == false)
        {
            var dir = './${ServerSettings.SaveDirectory}/';
            var path = dir + "ServerSettings.txt";

            if(FileSystem.exists(path) == false) ServerSettings.writeToFile();
        }

        if (ServerSettings.dumpOutput)
        {
            var dump = File.append("dump.txt",false);
            haxe.Log.trace = (v:Dynamic,?infos:haxe.PosInfos) -> {
                dump.writeString('$v\n');
            }
        }
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

        // do all the object inititalisation stuff
        Engine.dir = Utility.dir();
        dataVersionNumber = Resource.dataVersionNumber();
        trace('dataVersionNumber: $dataVersionNumber');
        
        if(ObjectData.ReadAllFromFile(dataVersionNumber) == false)
        {
            ObjectData.ImportObjectData();
            ObjectData.WriteAllToFile(dataVersionNumber);
        }

        ObjectData.CreatePersonArray();        
        ObjectData.CreateAndAddDummyObjectData();
        ObjectData.CreateFoodObjectArray();
        ServerSettings.PatchObjectData();

        ObjectData.GenerateBiomeObjectData();
        

        // do all the object transition inititalisation stuff
        trace("Import transitions...");
        transitionImporter.importCategories();
        transitionImporter.importTransitions();

        ServerSettings.PatchTransitions(transitionImporter);

        // do all the map inititalisation stuff
        map = new WorldMap();

        if(ServerSettings.GenerateMapNew)
        {
            map.generate();
            map.writeToDisk();
        }
        else
        {
            if(map.readFromDisk() == false)
            {
                trace('could not read World Map from disk! Start generating new map...');

                map.generate();
                map.writeToDisk();
            }
        }

        //prevent any blocking object on global starting position
        var startObj = map.getObjectHelper(ServerSettings.startingGx,ServerSettings.startingGy,false);
        if (startObj != null) 
        {
            if (startObj.blocksWalking())
                map.setObjectId(ServerSettings.startingGx,ServerSettings.startingGy,[0]);
        }
        // run run run Thread run run run
        var thread = new ThreadServer(this,8005);
        Thread.create(function()
        {
            thread.create();
        });
    }

    public function sendSay(text:String,curse:Bool)
    {
        
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

    private function message(header:Connection, tag:ServerTag,input:Array<String>,string:String)
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
            case SREMV:
                header.player.specialRemove(Std.parseInt(input[0]),Std.parseInt(input[1]),Std.parseInt(input[2]),input.length > 3 ? Std.parseInt(input[3]) : null);
            case REMV:
                TransitionHelper.doCommand(header.player, tag, Std.parseInt(input[0]),Std.parseInt(input[1]), Std.parseInt(input[2]));
            case USE:
                TransitionHelper.doCommand(header.player, tag, Std.parseInt(input[0]),Std.parseInt(input[1]), input.length > 3 ? Std.parseInt(input[3]) : -1 ,input.length > 2 ? Std.parseInt(input[2]) : 0);
            case DROP:
                if(header.player.heldPlayer == null)
                {
                    TransitionHelper.doCommand(header.player, tag, Std.parseInt(input[0]),Std.parseInt(input[1]), Std.parseInt(input[2]));
                }
                else{
                    header.player.dropPlayer();
                }
            case SELF:
                header.player.self(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]));
            case UBABY:
                header.player.doOnOther(Std.parseInt(input[0]), Std.parseInt(input[1]), Std.parseInt(input[2]), input.length > 3 ? Std.parseInt(input[3]) : -1);
            case SAY:
                var text = string.substring(4);
                header.player.say(text);
            case FLIP:
                header.flip();
            case PING:
                // PING x y unique_id#
                header.sendPong(input[2]);
            case BABY: // BABY x y# // BABY x y id#
                header.player.doBaby(Std.parseInt(input[0]), Std.parseInt(input[1]), input.length > 2 ? Std.parseInt(input[2]) : -1);
            default:
        }
    }
}
#end