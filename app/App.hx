package;

import haxe.ds.IntMap;
import openlife.data.object.player.PlayerInstance;
import openlife.engine.Program;
import openlife.data.map.MapInstance;
import openlife.engine.Engine;

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
        Config.run(client,cred());
        connect(false);
        while (true)
        {
            client.update();
            Sys.sleep(1/30);
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
                //main player updated
            }
        }
        if (player == null)
        {
            player = instances.pop();
            //new player set
        }
    }
    
}