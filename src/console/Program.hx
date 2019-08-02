package console;

#if openfl
import states.game.Object;
#end
import states.game.Player;
import states.game.Game;
class Program
{
    var game:Game;
    public var goal:Pos = new Pos();
    public var setupGoal:Bool = false;
    public var range:Float = 15000;
    public var useRange:Int = 1;
    //first index for action second is the list
    var actions:Array<Array<Int>> = [];
    var goals:Array<Pos> = [];
    //food 
    public var ate:Array<Int> = [];
    public function new(game:Game)
    {
        this.game = game;
    }
    public function stop()
    {
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
                //self

                case 2:

            }
        }
        //continue movement
        var dir = goals.pop();
        if (dir != null)
        {
            path(dir.x,dir.y);
        }
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
            Player.main.goal = true;
        }
        return this;
    }
    public function drop(x:Int=0,y:Int=0):Program
    {
        return this;
    }
    public function use(x:Int=0,y:Int=0):Program
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
            //set player pathing
            path();
            Main.console.print("Distance to goal",Std.string(dis));
        }else{
            setupGoal = false;
            Main.console.print("Max Range",Std.string(dis));
        }
        return this;
    }
    /**
     * Remove an object from a container
     * @return Program
     */
    public function remove():Program
    {
        return this;
    }
    /**
     * special case of removing an object contained in a piece of worn clothing.
     * @param i 0=hat, 1=tunic, 2=frontShoe, 3=backShoe, 4=bottom, 5=backpack
     * @return public function pickup():Program
     **/
    public function pull(i:Int):Program
    {
        return this;
    }
    /**
     * Same as Use
     * @return Program
     */
    public function pickup():Program
    {
        use();
        return this;
    }
    /**
     * -1 to eat what you are holdign
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
    public function task(name:String):Program
    {
        switch(name.toLowerCase())
        {
            case "eat":
            find("food").use().self();
        }
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
            case "milkweed":
            [50,51,52];
            case "big hard rock":
            [32];
            case "stone":
            [33];
            case "sharp stone":
            [34];
            case "berry bush":
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