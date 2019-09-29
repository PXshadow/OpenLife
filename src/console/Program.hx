package console;

import data.GameData;
import server.ServerTag;
import haxe.crypto.Base64;
import motion.Actuate;
#if openfl
import openfl.display.Tile;
#end
import game.Player;
class Program
{
    public var goal:Pos = new Pos();
    //bool to refine path
    public var refine:Bool = false;
    public var home:Pos = new Pos();
    //setup automation bool
    public var setup:Bool = false;
    public var range:Int = 30;
    public var useRange:Int = 1;
    //food 
    public var ate:Array<Int> = [];
    //queue messages to be sent later
    public var queue:Array<ServerTag> = [];
    var console:Console;
    var data:GameData;
    public function new(data:GameData,console:Console)
    {
        this.data = data;
        this.console = console;
    }
    public function send(tag:ServerTag,data:String)
    {
        /*if (setup)
        {
            //queue because path is currently being travled
            queue.push(tag);
        }else{*/
            Main.client.send(tag + " " + data);
        //}
    }
    public function stop()
    {
        trace("stop goal");
        //finished, now perform another action
        
        //clean up
        Main.player.goal = false;
        //setup = false;
    }
    public function setHome(x:Null<Int>=0,y:Null<Int>=0):Program
    {
        if (x != null && y != null)
        {
            home.x = x;
            home.y = y;
        }else{
            //set home where player location is at
            if (Main.player != null)
            {
                home.x = Main.player.instance.x;
                home.y = Main.player.instance.y;
            }
        }
        return this;
    }
    public function path(x:Null<Int>=null,y:Null<Int>=null):Program
    {
        if (x != null && y != null)
        {
            goal.x = x;
            goal.y = y;
            setup = true;
        }
        if (setup)
        {
            //main
            Main.player.goal = true;
            Main.player.motion();
            refine = true;
        }
        return this;
    }
    public function kill(x:Null<Int>=null,y:Null<Int>=0,id:Null<Int>=null)
    {
        if (x != null && y != null)
        {
            if (id != null)
            {
                //focus on id to kill on tile
                send(KILL, x + " " + y + " " + id);
            }else{
                send(KILL, x + " " + y);
            }
        }
    }
    //async return functions of data
    public function grave(x:Int,y:Int)
    {
        send(GRAVE, x + " " + y);
    }
    public function owner(x:Int,y:Int)
    {
        send(OWNER, x + " y " + y);
    }
    public function drop(x:Null<Int>=null,y:Null<Int>=0,c:Int=-1):Program
    {
        //setting held object down on empty grid square OR for adding something to a container
        if (x != null && y != null)
        {
            send(DROP, x + " " + y + " " + c);
        }else{
            if (Main.player != null) send(DROP, Main.player.instance.x + " " + Main.player.instance.y + " " + c);
        }
        return this;
    }
    public function grab()
    {
        //todo: grab entails logic to drop objects if present in hand to grab object located
        return use();
    }
    public function use(x:Null<Int>=null,y:Null<Int>=null):Program
    {
        if (x != null && y != null)
        {
            send(USE, x + " " + y);
        }else{
            if (Main.player != null) send(USE, Main.player.instance.x + " " + Main.player.instance.y);
        }
        return this;
    }
    public function find(name:String):Program
    {
        findList(id(name));
        return this;
    }
    private function findList(get:Array<Int>)
    {
        var dis:Float = range;
        var cur:Float = 0;
        var id:Int = 0;
        trace("find " + get);
        for(y in max(data.map.y,Main.player.instance.y - range)...min(data.map.y + data.map.height,Main.player.instance.y + range))
        {
            for(x in max(data.map.x,Main.player.instance.x - range)...min(data.map.x + data.map.width,Main.player.instance.x + range))
            {
                    //array of objects in the tile
                    id = data.map.object.get(x,y)[0];
                    if (get.indexOf(id) >= 0)
                    {
                        cur = Math.abs(Math.sqrt(Math.pow(Main.player.instance.y - y,2) + Math.pow(Main.player.instance.x - x,2)));
                        trace("cur " + cur);
                        if (cur < dis)
                        {
                            goal.y = y;
                            goal.x = x;
                            dis = cur;
                        }
                    }
            }
        }
        if (dis < range)
        {
            setup = true;
            console.print("distance",Std.string(dis));
        }else{
            setup = false;
            console.print("out of range",Std.string(dis));
        }
    }
    public function max(a:Int,b:Int):Int
    {
        if (a > b) return a;
        return b;
    }
    public function min(a:Int,b:Int):Int
    {
        if (a < b) return a;
        return b;
    }
    public function emote(e:Int):Program
    {
        //0-13
        send(EMOT,"0 0 " + e);
        return this;
    }
    public function say(string:String):Program
    {
        send(SAY,"0 0 " + string.toUpperCase());
        return this;
    }
    public function remove(x:Int,y:Int,index:Int=-1):Program
    {
        //remove an object from a container
        send(REMV, x + " " + y + " " + index);
        return this;
    }
    public function specialRemove(i:Int=-1):Program
    {
        return pull(i);
    }
    public function inventory(i:Int,index:Int=-1):Program
    {
        return pull(i,index);
    }
    public function pull(i:Int,index:Int=-1):Program
    {
        //remove object from clothing
        send(SREMV, i + " " + index);
        return this;
    }
    public function pickup():Program
    {
        return use();
    }
    public function baby(x:Null<Int>,y:Null<Int>):Program
    {
        //USE action taken on a baby to pick them up.
        if (x != null && y != null)
        {
            send(BABY, x + " " + y);
        }else{
            if (Main.player != null) send(BABY,Main.player.instance.x + " " + Main.player.instance.y);
        }
        return this;
    }
    public function jump():Program
    {
        send(JUMP,"0 0");
        return this;
    }
    public function ubaby(x:Int,y:Int,index:Int=-1):Program
    {
        //special case of SELF applied to a baby (to feed baby food or add/remove clothing from baby)
        //UBABY is used for healing wounded players.
        //UBABY x y i id#
        send(UBABY,x + " " + y + " " + index);
        return this;
    }
    public function self(index:Int=-1):Program
    {
        //use action on self (eat)
        if (Main.player != null) send(SELF, Main.player.instance.x + " " + Main.player.instance.y + index);
        return this;
    }
    public function eat():Program
    {
        return self();
    }
    public function get(name:String):Program
    {
        var list = id(name);
        if (list.indexOf(Main.player.instance.o_id) == -1)
        {
            //go and find
            findList(list);
            path();
        }
        return this;
    }
    public function task(name:String):Program
    {
        setup = false;
        //names need to be lower case and have no spaces, however when using task or find functions you can use spaces and upper case freely.
        switch(StringTools.replace(name.toLowerCase()," ",""))
        {
            case "eat" | "food" | "hunger":
            //find than path to, grab and then eat.
            find("food").path().grab().eat();
            case "sharpstone":
            //find stone than go to a hard rock and use against it to turn into a sharp stone
            find("stone").path().grab().find("big hard rock").use();
            case "waterbowl":
            //if water bowl in area grab
            find("water bowl").path().grab();
            if (!setup)
            {
                //find empty bowl instead and fill
                find("claybowl").path().grab().find("water source").path().use();
            }
            case "soilbowl":
            //if soil bowl in area grab
            find("soilbowl").path().grab();
            if (!setup)
            {
                //find empty bowl instead and fill
                find("clay bowl").path().grab().find("soil source").path().use();
            }
            case "berryfarm":
            var g:Pos = null;
            find("languishing berry bush");
            if (setup) g = goal;
            find("dry berry bush");
            if (setup)
            {
                //at least one went through
                if (g == null || distance(goal,g))
                {
                    //dry - needs water
                    g = goal;
                    task("water bowl");
                }else{
                    //languishing - needs soil
                    task("soil bowl");
                }
                //go to bush
                path(g.x,g.y).use();
            }
        }
        return this;
    }
    public function die():Program
    {
        send(DIE,"0 0");
        return this;
    }
    public function distance(a:Pos,b:Pos):Bool
    {
        if (Main.player == null) 
        {
            trace("distance calculation occuring before player has been initalized");
            return false;
        }
        //if a is shorter true, else false. distance from player
        a.x += -Main.player.instance.x;
        a.y += -Main.player.instance.y;
        b.x += -Main.player.instance.x;
        b.y += -Main.player.instance.y;
        if (a.x + a.y > b.x + b.y) return false;
        return true;
    }
    public function sub(a:Pos,b:Pos):Pos
    {
        var pos = new Pos();
        pos.x = a.x - b.y;
        pos.y = a.x - b.y;
        return pos;
    }
    public function step(x:Int,y:Int):Program
    {
        //move player
        Main.player.step(x,y);
        return this;
    }
    private function id(name:String):Array<Int>
    {
        //lower case and no spaces
        return switch(StringTools.replace(name.toLowerCase()," ",""))
        {
            case "milkweed":
            [50,51,52];
            case "bighardrock":
            [32];
            case "stone":
            [33];
            case "sharpstone":
            [34];
            case "berrybush":
            [30];
            case "berry" | "berries": 
            [31];
            case "food":
            var food = [
                31,//Gooseberry
                40,//Wild Carrot
                197,//Cooked Rabbit
                253,//Bowl of Gooseberries
                272,//Cooked Berry Pie
                273,//Cooked Carrot Pie
                274,//Cooked Rabbit Pie
                275,//Cooked Berry Carrot Pie
                276,//Cooked Berry Rabbit Pie
                277,//Cooked Rabbit Carrot Pie
                278,//Cooked Berry Carrot Rabbit Pie
                402,//Carrot
                518,//Cooked Goose
                570,//Cooked Mutton
            ];
            //remove non yum food
            for (remove in ate)
            {
                food.remove(remove);
            }
            //return food
            food;
            case "trees" | "tree":
            [
                760, //Dead Tree
                527, //Willow Tree
                49, //Juniper Tree

                2452, //Yule Tree
                2454, //Spent Yule Tree
                2455, //Yule Tree with Half Candles

                406, //Yew Tree
                153, //Yew Tree #branch

                530, //Bald Cypress Tree

                2142, //Banana Tree 
                2145, //Empty Banana Tree

                48, //Maple Tree
                63, //Maple Tree #branch

                2135, //Rubber Tree
                2136, //Slashed Rubber Tree

                45, //Lombardy Poplar Tree
                65, //Lombardy Poplar Tree

                99, //White Pine Tree
                100, //White Pine Tree with Needles
                2450, //Pine Tree with Candles
                2461, //Pine Tree with Cardinals
                2434, //Pine Tree with One Gardland
                2436, //Pine Tree with Two Gardland

                1874, //Wild Mango Tree
                1875, //Fruiting Domestic Mango Tree
                1876, //Languishing Domestic Mango Tree
                1922, //Dry Fertile Domestic Mango Tree
                1923, //Wet Fertile Domestic Mango Tree
            ];
            case "watersource":
            [
                511, //Pound
                662, //Shallow Well
                660, //Full Bucket of Water
                1099, //Partial Buket Of Water
            ];
            case "watersourcebucket":
            [
                663, //Deep Well
                1097, //Full Deep Well
                706, //Ice Hole
            ];
            case "bucket":
            [
                659, //Empty Bucket
                660, //Full Bucket of Water
                1099, //Partial Buket Of Water
            ];
            case "danger":
            //thanks Hetuw (Whatever)
            [
                2156, // Mosquito swarm

                764,  // Rattle Snake
                1385, //Attacking Rattle Snake
                
                1328, //Wild board with Piglet
                1333, //Attacking Wild Boar
                1334, //Attacking Wild Boar with Piglet
                1339, //Domestic Boar
                1341, //Domestic Boar with Piglet
                1347, //Attacking Boar# domestic
                1348, //Attacking Boar with Piglet# domestic

                418, //Wolf
                1630, //Semi-tame Wolf
                420, //Shot Wolf
                428, //Attacking Shot Wolf
                429, //Dying Semi-tame Wolf
                1761, //Dying Semi-tame Wolf
                1640, //Semi-tame Wolf# just fed
                1642, //Semi-tame Wolf# pregnant
                1636, //Semi-tame Wolf with Puppy#1
                1635, //Semi-tame Wolf with Puppy#2
                1631, //Semi-tame Wolf with Puppy#3
                1748, //Old Semi-tame Wolf
                1641, //@ Deadly Wolf

                628, //Grizzly Bear
                655, //Shot Grizzly Bear#2 attacking
                653, //Dying Shot Grizzly Bear #3
                631, //Hungry Grizzly Bear
                646, //@ Unshot Grizzly Bear
                635, //Shot Grizzly Bear#2
                645, //Shot Grizzly Bear
                632, //Shot Grizzle Bear#1
                637, //Shot Grizzle Bear#3
                654, //Shot Grizzle Bear#1 attacking

                1789, //Abused Pit Bull
                1747, //Mean Pit Bull
                1712, //Attacking Pit Bull
            ];
            default: 
            console.print(name,"Not found");
            [-1];
        }
    }
    #if openfl
    //visual
    public function apply(target:String,properties:Dynamic):Program
    {
        var array = getTiles(target);
        if (array.length > 0)
        {
            var fields = Reflect.fields(properties);
            trace("fields " + fields);
            for (obj in array)
            {
                for (field in fields)
                {
                    Reflect.setProperty(obj,field,Reflect.getProperty(properties,field));
                }
            }
        }
        return this;
    }
    public function tween(target:String,duration:Int=1,properties:Dynamic,repeat:Int=0,reflect:Bool=false,delay:Int=0):Program
    {
        var array = getTiles(target);
        if (array.length > 0)
        {
            for (obj in array)
            {
                Actuate.tween(obj,duration,properties).repeat(repeat).reflect(reflect).delay(delay);
            }
        }
        return this;
    }
    public function resetTween():Program
    {
        Actuate.reset();
        return this;
    }
    private function getTiles(target:String):Array<Tile>
    {
        var targets:Array<Tile> = [];
        var list = id(target);
        if (list.length == 0) return targets;
        var cx:Int = Main.player.instance.x;
        var cy:Int = Main.player.instance.y;
        for (j in cy - Main.objects.range...cy + Main.objects.range)
        {
            for (i in cx - Main.objects.range...cx + Main.objects.range)
            {
                if (list.indexOf(data.map.object.get(i,j)[0]) >= 0) for(tile in data.tileData.object.get(i,j)) targets.push(tile);
            }
        }
        return targets;
    }
    #end
}
class Pos
{
    public var x:Int;
    public var y:Int;
    public function new()
    {

    }
}