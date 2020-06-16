package;

import haxe.ds.IntMap;
import openlife.data.object.player.PlayerInstance;
import openlife.engine.Program;
import sys.io.File;
import haxe.Json;
import openlife.data.map.MapInstance;
import openlife.engine.Engine;
import sys.FileSystem;

class App extends Engine
{
    var player:PlayerInstance;
    var players = new IntMap<PlayerInstance>();
    var program:Program;
    var count:Int = 0;
    public function new()
    {
        super();
        program = new Program(client);
        var bool:Bool = false;
        var config:Cred;
        if (FileSystem.exists("cred"))
        {
            Sys.println("Use existing cred config (y)es (n)o");
            bool = Sys.stdin().readLine() == "n" ? false : true;
            if (bool)
            {
                config = cast Json.parse(File.getContent("cred"));
                client.ip = config.ip;
                client.port = config.port;
                client.legacy = config.legacy;
                client.email = config.email;
                client.key = config.key;
            }
        }
        if (!bool)
        {
            Sys.println("Legacy authentication (y)es (n)o");
            client.legacy = Sys.stdin().readLine() == "y" ? true : false;
            //set credentioals
            if (!cred())
            {
                Sys.println("ip:");
                string = Sys.stdin().readLine();
                client.ip = string;
                Sys.println("port:");
                string = Sys.stdin().readLine();
                client.port = Std.parseInt(string);
                Sys.println("email:");
                string = Sys.stdin().readLine();
                client.email = string;
                Sys.println("key");
                string = Sys.stdin().readLine();
                client.key = string;
                //set config
                config = {email: client.email, key: client.key,ip: client.ip,port: client.port,legacy: client.legacy};
                File.saveContent("cred",Json.stringify(config));
            }
        }
        connect(false);
        while (true)
        {
            client.update();
            Sys.sleep(1/30);
            if (count++ > 30)
            {
                count = 0;
                trace('player step!');
                //every 2 seconds move main player left
                program.step(player.x,player.y,++player.done_moving_seqNum,-1,0);
            }
        }
    }
    override function mapChunk(instance:MapInstance) {
        super.mapChunk(instance);
        trace("instance " + instance.toString());
    }
    override function playerUpdate(instances:Array<PlayerInstance>) {
        super.playerUpdate(instances);
        for (instance in instances)
        {
            players.set(instance.p_id,instance);
            if (player != null && instance.p_id == player.p_id)
            {
                trace('MAIN PLAYER UPDATED\n$player');
            }
        }
        if (player == null)
        {
            player = instances.pop();
            trace('MAIN PLAYER\n$player');
        }
    }
    
}
typedef Cred = {legacy:Bool,email:String,key:String,ip:String,port:Int}