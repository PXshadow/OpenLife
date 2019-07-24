package console;

import states.game.Object;
import states.game.Player;
import states.game.Game;
class Program
{
    var game:Game;
    public var goal:Pos = new Pos();
    public var setupGoal:Bool = false;
    public var range:Float = 15000;
    public var useRange:Int = 1;
    public var dropAction:Bool = false;
    public var selfAction:Bool = false;
    public function new(game:Game)
    {
        this.game = game;
    }
    public function stop()
    {
        Player.main.goal = false;
        setupGoal = false;
        dropAction = false;
        selfAction = false;
    }
    /**
     * Go to goal location or new set location
     * @param x 
     * @param y 
     */
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
            Player.main.goal = true;
        }
        return this;
    }
    public function drop():Program
    {
        return this;
    }
    /**
     * Find Object within range and set goal
     * @param name 
     * @return Program
     */
    public function find(name:String):Program
    {
        var get = id(name);
        var dis:Float = range;
        var cur:Float = 0;
        for(y in 0...game.data.map.object.length)
        {
            for(x in 0...game.data.map.object[y].length)
            {
                for (i in 0...game.data.map.object[y][x].length)
                {
                    //array of objects in the tile
                    if (get.indexOf(game.data.map.object[y][x][i]) >= 0)
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
        }
        if (dis < range)
        {
            setupGoal = true;
            Main.console.print("Find Distant",Std.string(dis));
        }else{
            setupGoal = false;
            Main.console.print("Max Range",Std.string(dis));
        }
        return this;
    }
    public function pickup():Program
    {
        self();
        return this;
    }
    /**
     * -1 to pickup
     * @param index 
     * @return Program
     */
    public function self(index:Int=-1):Program
    {
        return this;
    }
    public function emote(index:Int,time:Int=1):Program
    {
        return this;
    }
    public function craft(name:String):Program
    {
        return this;
    }
    //move player
    public function step(x:Int,y:Int):Program
    {
        Player.main.step(x,y);
        return this;
    }
    private function id(name:String):Array<Int>
    {
        return switch(name.toLowerCase())
        {
            case "berry bush":
            [30];
            case "berry" | "berries": 
            [31];
            default: 
            [-1];
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
            case "water source":
            [
                511, //Pound
                662, //Shallow Well
                660, //Full Bucket of Water
                1099, //Partial Buket Of Water
            ];
            case "water source bucket":
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
        }
    }
    private function cat()
    {

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