package console;

import haxe.crypto.Base64;
import motion.Actuate;
#if openfl
import states.game.Object;
#end
import states.game.Player;
import states.game.Game;
class Program
{
    var game:Game;
    public var goal:Pos = new Pos();
    public var home:Pos = new Pos();
    public var setupGoal:Bool = false;
    public var range:Float = 1000;
    public var useRange:Int = 1;
    //first index for action second is the list
    var actions:Array<Array<Int>> = [];
    var actionIndex:Int = -1;
    var goals:Array<Pos> = [];
    //food 
    public var ate:Array<Int> = [];
    public function new(game:Game)
    {
        this.game = game;
    }
    public function stop()
    {
        trace("stop goal");
        //finished, now perform actions
        action();
        //clean up
        Player.main.goal = false;
        setupGoal = false;
    }
    /**
     * Go to goal location or new set location
     * @param x 
     * @param y 
     **/
    public function action()
    {
        if (setupGoal) return;
        //complete actions
        for (action in actions.shift())
        {
            switch (action)
            {
                case 0:
                //use
                
                case 1:
                //self, next int sets index

                case 2:
                //drop

                case 3:
                //jump

                case 4:
                //remove

                case 5:
                //pull, next int sets index
            }
        }
        //continue movement
        var dir = goals.pop();
        if (dir != null)
        {
            path(dir.x,dir.y);
        }
    }
    public function setHome(x:Null<Int>=0,y:Null<Int>=0):Program
    {
        if (x != null && y != null)
        {
            home.x = x;
            home.y = y;
        }else{
            //set home where player location is at
            if (Player.main != null)
            {
                home.x = Player.main.instance.x;
                home.y = Player.main.instance.y;
            }
        }
        return this;
    }
    private function addAction(i:Int):Bool
    {
        if (setupGoal)
        {
            actions[actionIndex].push(i);
            return true;
        }
        return false;
    }
    public function path(x:Null<Int>=null,y:Null<Int>=null):Program
    {
        if (x != null && y != null)
        {
            goal.x = x;
            goal.y = y;
            setupGoal = true;
        }
        if (setupGoal)
        {
            //set goal pathing
            goals.push(goal);
            //create action
            actionIndex++;
            actions[actionIndex] = [];
            //main
            Player.main.goal = true;
        }
        return this;
    }

    public function drop(x:Int=0,y:Int=0,c:Int=-1):Program
    {
        if (addAction(2)) return this;
        Main.client.send("DROP " + x + " " + y + " " + c);
        return this;
    }
    public function use(x:Int=0,y:Int=0):Program
    {
        if (addAction(0)) return this;
        Main.client.send("USE " + x + " " + y);
        return this;
    }
    public function find(name:String):Program
    {
        var get = id(name);
        var dis:Float = range;
        var cur:Float = 0;
        var id:Int = 0;
        for(y in game.data.map.object.dy...game.data.map.object.dy + game.data.map.object.lengthY())
        {
            for(x in game.data.map.object.dx...game.data.map.object.dx + game.data.map.object.lengthX())
            {
                    //array of objects in the tile
                    id = game.data.map.object.get(x,y);
                    if (get.indexOf(id) >= 0)
                    {
                        cur = Math.sqrt(Math.pow(Player.main.instance.y - y + game.data.map.y,2) + Math.pow(Player.main.instance.x - x + game.data.map.x,2));
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
            setupGoal = true;
            Main.console.print("Distance to goal",Std.string(dis));
        }else{
            setupGoal = false;
            Main.console.print("Max Range",Std.string(dis));
        }
        return this;
    }
    public function emote(e:Int):Program
    {
        //0-13
        Main.client.send("EMOT 0 0 " + e);
        return this;
    }
    public function say(string:String):Program
    {
        Main.client.send("SAY 0 0 " + string.toUpperCase());
        return this;
    }
    public function remove(x:Int,y:Int,index:Int=-1):Program
    {
        //remove an object from tilecontainer
        if (addAction(4)) return this;
        Main.client.send("REMV " + x + " " + y + " " + index);
        return this;
    }
    public function specialRemove(i:Int=-1):Program
    {
        return pull(i);
    }
    public function pull(i:Int,index:Int=-1):Program
    {
        if (addAction(5)) 
        {
            addAction(i);
            return this;
        }
        Main.client.send("SREMV " + i + " " + index);
        return this;
    }
    public function pickup():Program
    {
        use();
        return this;
    }
    public function self(index:Int=-1):Program
    {
        if (addAction(1))
        {
            addAction(index);
            return this;
        }
        Main.client.send("SELF 0 0 " + index);
        return this;
    }
    public function task(name:String):Program
    {
        switch(name.toLowerCase())
        {
            case "eat" | "food" | "hunger":
            find("food").path().use().self();
        }
        return this;
    }
    public function step(x:Int,y:Int):Program
    {
        //move player
        Player.main.step(x,y);
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
    private function getTiles(target:String):Array<Object>
    {
        var targets:Array<Object> = [];
        var list = id(target);
        if (list.length == 0) 
        {
            trace("unable to find target " + target);
            return targets;
        }
        var obj:Object;
        for (i in 0...game.objects.numTiles)
        {
            obj = cast game.objects.getTileAt(i);
            if (list.indexOf(obj.oid) >= 0)
            {
                targets.push(obj);
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