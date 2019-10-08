package console;

import haxe.Timer;
import data.GameData;
import server.ServerTag;
import haxe.crypto.Base64;
import data.ObjectCode;
#if openfl
import motion.Actuate;
import openfl.display.Tile;
#end
import game.Player;
class Program
{
    public var goal:Pos = new Pos();
    public var home:Pos = new Pos();
    //setup automation bool
    public var setup:Bool = false;
    public var useRange:Int = 1;
    var useRangeX:Int = 0;
    var useRangeY:Int = 0;
    //food 
    public var ate:Array<Int> = [];
    //used to wait for server response on events
    var repeatTimer:Timer;
    //automation buffer system
    var actions:Array<Action> = [];
    var action:Action = null;
    var console:Console;
    public var range:Int = 30;
    public function new(console:Console)
    {
        this.console = console;
    }
    public function send(tag:ServerTag,data:String,moveSensitive:Bool=true)
    {
        if (setup)
        {
            if (action != null)
            {
                action.tag.push(Reflect.copy(tag));
                action.data.push(Reflect.copy(data));
            }
        }else{
            Main.client.send(tag + " " + data);
        }
    }
    public function clean()
    {
        //clean up
        Main.player.goal = false;
        setup = false;
        goal = new Pos();
        action = null;
        actions = [];
    }
    public function update()
    {
        
    }
    private function preform(i:Int)
    {
        if (action == null || goal == null) return;
        Main.client.send(action.tag[i] + " " + Std.string(goal.x - useRangeX)  + " " + Std.string(goal.y - useRangeY));
    }
    public function end()
    {
        trace("end path");
        //Sys.sleep(0.5);
        //preform command(s) if any
        if (action != null)
        {
            trace("action " + action);
            if (action.tag != null) 
            {
                //finish other commands
                for (i in 0...action.tag.length)
                {
                    preform(i);
                }
                if (action.finish != null) action.finish();
                //use potential new action
                action = actions.shift();
                trace("new action " + action);
                if (action != null) 
                {
                    run();
                    return;
                }
            }
        }
        //clean if pass through
        clean();
    }
    public function run()
    {
        goal = null;
        setup = false;
        switch(action.type)
        {
            case 0:
            //nothing
            case 1:
            //find
            goto(action.property);
            case 2:
            //watch

            case 3:
            //hunt

            case 4:
            //fence

            default:
            //type not found
            return;
        }
        setup = true;
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
                home.x = Main.player.ix;
                home.y = Main.player.iy;
            }
        }
        return this;
    }
    public function path(refine:Bool):Program
    {
        if (refine)
        {
            var ix:Int = Main.player.ix - goal.x;
            var iy:Int = Main.player.iy - goal.y;
            if (Math.abs(ix) >= useRange)
            {
                if (ix > 0)
                {
                    useRangeX += useRange;
                }else{
                    useRangeX += -useRange;
                }
            }
            if (Math.abs(iy) >= useRange)
            {
                if (iy > 0)
                {
                    useRangeY += useRange;
                }else{
                    useRangeY += -useRange;
                }
            }
            //same or slightly faster if it's a direct vertical or horizontal (also deals with blocking objects)
            if (!block(goal.x + useRangeX,goal.y))
            {
                useRangeY = 0;
            }else{
                if (!block(goal.x,goal.y + useRangeY))
                {
                    useRangeX = 0;
                }else{
                    //potentially the same
                    if (!block(goal.x,goal.y))
                    {
                        useRangeX = 0;
                        useRangeY = 0;
                    }else{
                        //less efficent
                        if (!block(goal.x - useRangeX,goal.y))
                        {
                            useRangeX *= -1;
                            useRangeY = 0;
                        }else{
                            if (!block(goal.x,goal.y - useRangeY))
                            {
                                useRangeX = 0;
                                useRangeY *= -1;
                            }else{
                                //no area to stand
                            }
                        }
                    }
                }
            }
            goal.x += useRangeX;
            goal.y += useRangeY;
        }
        if (setup)
        {

        }else{
            Main.player.goal = true;
            Main.player.path();
            setup = true;
        }
        return this;
    }
    private function block(x:Int,y:Int):Bool
    {
        return Main.data.blocking.get(x + "." + y) == true ? true : false;
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
            if (Main.player != null) send(DROP, Main.player.ix + " " + Main.player.iy + " " + c);
        }
        return this;
    }
    public function use(x:Null<Int>=null,y:Null<Int>=null):Program
    {
        if (x != null && y != null)
        {
            send(USE, x + " " + y);
        }else{
            if (Main.player != null) send(USE, Main.player.ix + " " + Main.player.iy);
        }
        return this;
    }
    public function goto(name:String,refine:Bool=true,optimize:Bool=true):Program
    {
        if (type(1,name)) return this;
        trace("goto " + name);
        goal = findList(ObjectCode.id(name),optimize);
        if (goal == null && action.fail != null)
        {
            action.fail();
        }
        path(refine);
        return this;
    }
    public function events(finish:Void->Void,fail:Void->Void)
    {
        if (action != null) 
        {
            action.finish = finish;
            action.fail = fail;
        }
    }
    private function type(id:Int,property:String):Bool
    {
        //queue
        useRangeX = 0;
        useRangeY = 0;
        action = {type: id,property: property, tag: action != null ? action.tag : [],data: action != null ? action.data : [],finish: null,fail: null};
        if (setup) 
        {
            actions.push(action);
            return true;
        }
        return false;
    }
    private function findList(get:Array<Int>,optimize:Bool=true):Pos
    {
        var dis:Float = range;
        var cur:Float = 0;
        var id:Int = 0;
        var obj:Array<Int>;
        var pos = new Pos();
        trace("find " + get);
        //trace("x " + max(data.map.y,Main.player.iy - range) + " maxx " + min(data.map.y + data.map.height,Main.player.iy + range));
        for(y in Main.player.iy - range... Main.player.iy + range)
        {
            for(x in Main.player.ix - range...Main.player.ix + range)
            {
                    //array of objects in the tile
                    obj = Main.data.map.object.get(x,y);
                    if (obj == null) continue;
                    id = obj[0];
                    if (get.indexOf(id) >= 0)
                    {
                        cur = Math.abs(x - Main.player.ix) + Math.abs(y - Main.player.iy);
                        //trace("cur " + cur);
                        if (cur < dis)
                        {
                            pos.x = x;
                            pos.y = y;
                            if (!optimize) return pos;
                            dis = cur;
                        }
                    }
            }
        }
        if (dis < range)
        {
            trace("distance " + dis);
            //console.print("distance",Std.string(dis));
            return pos;
        }else{
            //console.print("out of range",Std.string(dis));
            trace("out of range");
            return null;
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
    public function baby(x:Null<Int>,y:Null<Int>):Program
    {
        //USE action taken on a baby to pick them up.
        if (x != null && y != null)
        {
            send(BABY, x + " " + y);
        }else{
            if (Main.player != null) send(BABY,Main.player.ix + " " + Main.player.iy);
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
        if (Main.player != null) send(SELF, Main.player.ix + " " + Main.player.iy + index);
        return this;
    }
    public function eat():Program
    {
        return self();
    }
    public function task(name:String)
    {
        setup = false;
        //names need to be lower case and have no spaces, however when using task or find functions you can use spaces and upper case freely.
        switch(StringTools.replace(name.toLowerCase()," ",""))
        {
            case "eat" | "food" | "hunger":
            //find, path to, use and then eat.
            goto("food").use().eat();
            case "sharpstone":
            //goto stone, go to a hard rock and use against it to turn into a sharp stone
            goto("stone").use().goto("big hard rock").use();
            case "waterbowl":

            case "soilbowl":
            
            case "basket":

            case "berryfarm":
            
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
        a.x += -Main.player.ix;
        a.y += -Main.player.iy;
        b.x += -Main.player.ix;
        b.y += -Main.player.iy;
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
        var list = ObjectCode.id(target);
        if (list.length == 0) return targets;
        var cx:Int = Main.player.ix;
        var cy:Int = Main.player.iy;
        for (j in cy - range...cy + range)
        {
            for (i in cx - range...cx + range)
            {
                if (list.indexOf(Main.data.map.object.get(i,j)[0]) >= 0) for(tile in Main.data.tileData.object.get(i,j)) targets.push(tile);
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
    public function clone():Pos
    {
        var pos = new Pos();
        pos.x = x;
        pos.y = y;
        return pos;
    }
}
//type, 0 = nothing, 1 = find, 2 = watch, 3 = hunt (wait untill object found), 4 = fence
//property a string that is added to a type function
//pos where the player should go
//tags holds a buffer of the messages sent
//data holds a buffer of optional data alongside messages
typedef Action = {type:Int,property:String,tag:Array<ServerTag>,data:Array<String>,finish:Void->Void,fail:Void->Void}