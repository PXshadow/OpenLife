package openlife.engine;
import openlife.auto.Pathfinder;
import openlife.auto.Pathfinder.Coordinate;
import openlife.data.object.player.PlayerInstance;
import openlife.data.object.ObjectData;
import openlife.data.map.MapData;
import openlife.data.Pos;
import openlife.client.Client;
import haxe.Timer;
import openlife.server.ServerTag;
import haxe.crypto.Base64;
private typedef Command = {tag:ServerTag,x:Int,y:Int,data:String}
class Program
{
    public var home:Pos = new Pos();
    public var goal:Pos;
    public var init:Pos;
    var client:Client;
    var buffer:Array<Command>;
    public var moving:Bool = false; //bool to make sure movement went through
    public var onComplete:Void->Void;
    public var onError:String->Void;
    public function new(client:Client)
    {
        buffer = [];
        this.client = client;
    }
    public function send(tag:ServerTag,x:Int,y:Int,data:String="")
    {
        trace('send: $tag $x $y $data');
        if (moving && tag != SAY && tag != EMOT)
        {
            trace("---added to buffer---");
            buffer.push({tag: tag,x: x,y: y,data: data});
            return;
        }
        client.send('$tag $x $y $data');
    }
    public function update(player:PlayerInstance)
    {
        if (!moving) return;
        moving = false;
        trace("updating");
        if (goal == null)
        {
            if (onError != null) onError("no goal set");
            return;
        }
        if (player.x != goal.x || player.y != goal.y)
        {
            //error
            if (onError != null) onError("did not make it to the goal");
            return;
        }
        //play buffer
        trace("play buffer, count: " + buffer.length);
        for (command in buffer)
        {
            send(command.tag,command.x,command.y,command.data);
        }
        buffer = [];
        if (onComplete != null) onComplete();
        goal = null;
        init = null;
        trace("UPDATE");
    }
    public function setHome(x:Int,y:Int):Program
    {
        home.x = x;
        home.y = y;
        return this;
    }
    public function kill(x:Null<Int>=null,y:Null<Int>=0,id:Null<Int>=null)
    {
        if (x != null && y != null)
        {
            if (id != null)
            {
                //focus on id to kill on tile
                send(KILL,x,y," " + id);
            }else{
                send(KILL,x,y);
            }
        }
    }
    public function force(x:Int,y:Int)
    {
        client.send('FORCE $x $y');
    }
    //async return functions of data
    public function grave(x:Int,y:Int)
    {
        send(GRAVE,x,y);
    }
    public function owner(x:Int,y:Int)
    {
        send(OWNER,x,y);
    }
    public function drop(x:Int,y:Int,c:Int=-1):Program
    {
        send(DROP,x,y," " + c);
        return this;
    }
    public function use(x:Null<Int>=null,y:Null<Int>=null):Program
    {
        send(USE,x,y);
        return this;
    }
    private inline function max(a:Int,b:Int):Int
    {
        if (a > b) return a;
        return b;
    }
    private inline function min(a:Int,b:Int):Int
    {
        if (a < b) return a;
        return b;
    }
    public function emote(e:Int):Program
    {
        //0-13
        send(EMOT,0,0," " + e);
        return this;
    }
    public function say(string:String):Program
    {
        send(SAY,0,0,string.toUpperCase());
        return this;
    }
    /**
     * is special case of removing an object from a container.
     i specifies the index of the container item to remove, or -1 to
	 remove top of stack.
     * @param x 
     * @param y 
     * @param index 
     * @return Program
     */
    public function remove(x:Int,y:Int,index:Int=-1):Program
    {
        //remove an object from a container
        send(REMV,x,y," " + index);
        return this;
    }
    /**
     * is special case of removing an object contained in a piece of worn 
      clothing.
     * @param i 
     * @return Program
     */
    public function specialRemove(i:Int=-1):Program
    {
        return pull(i);
    }
    public function inventory(i:Int,index:Int=-1):Program
    {
        return pull(i,index);
    }
    /**
     * Remove object from clothing
     * @param i 
     * @param index 
     * @return Program
     */
    public function pull(i:Int,index:Int=-1):Program
    {
        send(SREMV,0,0," " + i + " " + index);
        return this;
    }
    /**
     * USE action taken on a baby to pick them up.
     * @param x 
     * @param y 
     * @return Program
     */
    public function baby(x:Int,y:Int):Program
    {
        send(BABY,x,y);
        return this;
    }
    /**
     * Baby jump from arms
     * @return Program
     */
    public function jump():Program
    {
        send(JUMP,0,0);
        return this;
    }
    public function goto(x:Int,y:Int,player:PlayerInstance,map:MapData):Program
    {
        if (player.x == x && player.y == y || moving) return this;
        if (Math.abs(player.x - x) >= MapData.RAD || Math.abs(player.y - y) >= MapData.RAD)
        {
            trace("OUT OF RANGE");
            return this;
        }
        var sx = (x - player.x) + MapData.RAD;
        var sy = (y - player.y) + MapData.RAD;
        //cords
        var start = new Coordinate(MapData.RAD,MapData.RAD);
        var end = new Coordinate(sx,sy);
        //map
        var map = new MapCollision(map.collisionChunk(player));
        //pathing
        var path = new Pathfinder(map);
        var paths = path.createPath(start,end,MANHATTAN,true);
        if (paths == null) 
        {
            trace("CAN NOT GENERATE PATH");
            return this;
        }
        var data:Array<Pos> = [];
        paths.shift();
        var mx:Array<Int> = [];
        var my:Array<Int> = [];
        var tx:Int = start.x;
        var ty:Int = start.y;
        for (path in paths)
        {
            data.push(new Pos(path.x - tx,path.y - ty));
        }
        goal = new Pos(x,y);
        init = new Pos(player.x,player.y);
        trace("goal " + goal);
        movePlayer(player,data);
        return this;
    }
    private inline function movePlayer(player:PlayerInstance,paths:Array<Pos>)
    {
        var string = "";
        for (path in paths)
        {
            string += " " + path.x + " " + path.y;
        }
        string = string.substring(1);
        send(MOVE,${player.x},${player.y},'@${++player.done_moving_seqNum} $string');
        var path = paths.pop();
        player.x = player.x + path.x;
        player.y = player.y + path.y;
        player.forced = true;
        if (client.relayIn != null) 
        {
            var string = 'PU\n${player.toData()}\n#';
            client.relayIn.output.writeString(string);
        }
        moving = true;
    }
    /**
     * special case of SELF applied to a baby (to feed baby food or add/remove clothing from baby)
     * UBABY is used for healing wounded players.
     * @param x 
     * @param y 
     * @param index 
     * @return Program
     */
    public function ubaby(x:Int,y:Int,index:Int=-1):Program
    {
        send(UBABY,x,y," " + index);
        return this;
    }
    /**
     * Use action on self (eat) or add clothing
     * @param index 
     * @return Program
     */
    public function self(player:PlayerInstance,index:Int=-1):Program
    {
        send(SELF,player.x,player.y," " + index);
        return this;
    }
    public function die():Program
    {
        send(DIE,0,0);
        return this;
    }
    public function sub(a:Pos,b:Pos):Pos
    {
        var pos = new Pos();
        pos.x = a.x - b.y;
        pos.y = a.x - b.y;
        return pos;
    }
}