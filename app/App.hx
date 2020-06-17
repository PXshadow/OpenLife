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
    var names = new IntMap<String>();
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
    override function says(id:Int, text:String, curse:Bool) {
        super.says(id, text, curse);
        trace('id $id say $text');
        if (text.indexOf("HI") > -1 || text.indexOf("HELLO") > -1 || text.indexOf("HEY") > -1)
        {
            program.say("HI");
            return;
        }
        if (text.indexOf("FOLLOW") > -1)
        {
            //follow script of mother
            return;
        }
        if (text.indexOf("UP") > -1)
        {
            program.say("UP");
            program.step(player,0,1);
            return;
        }
        if (text.indexOf("DOWN") > -1)
        {
            program.say("DOWN");
            program.step(player,0,-1);
            return;
        }
        if (text.indexOf("LEFT") > -1)
        {
            program.say("LEFT");
            program.step(player,-1,0);
            return;
        }
        if (text.indexOf("RIGHT") > -1)
        {
            program.say("RIGHT");
            program.step(player,1,0);
            return;
        }
        if (text.indexOf("USE") > -1)
        {
            program.say("USE");
            program.use(player.x,player.y);
            return;
        }
        program.say("HELLO " + names.get(id));
    }
    override function grave(x:Int, y:Int, id:Int) {
        super.grave(x, y, id);
        if (player.p_id == id)
        {
            trace("you have died!");
        }else{
            trace("player " + names.get(id) + " has died");
        }
    }
    override function playerName(id:Int, firstName:String, lastName:String) {
        super.playerName(id, firstName, lastName);
        trace("names " + firstName + " lastname " + lastName);
        names.set(id,firstName + " " + lastName);
    }
    override function mapChunk(instance:MapInstance) {
        super.mapChunk(instance);
        trace("instance " + instance.toString());
    }
    override function foodChange(store:Int, capacity:Int, ateId:Int, fillMax:Int, speed:Float, responsible:Int) {
        super.foodChange(store, capacity, ateId, fillMax, speed, responsible);
        if (store/capacity <= 0.25)
        {
            //less than 10% food
            program.say("F");
        }
    }
    override function playerUpdate(instances:Array<PlayerInstance>) {
        super.playerUpdate(instances);
        var inst:PlayerInstance;
        for (instance in instances)
        {
            inst = players.get(instance.p_id);
            players.set(instance.p_id,instance);
            if (inst != null)
            {
                if (!instance.forced)
                {
                    instance.x = inst.x;
                    instance.y = inst.y;
                }
                instance.forced = false;
            }
        }
        if (player == null)
        {
            player = instances.pop();
            //new player set
        }
    }
    
}