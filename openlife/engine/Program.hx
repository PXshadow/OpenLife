package openlife.engine;
import openlife.client.ClientTag;
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
    public var init:Pos; //inital pos
    public var dest:Pos; //destination pos
    var client:Client;
    var buffer:Array<Command>;
    public var moving:Bool = false; //bool to make sure movement went through
    public var onComplete:Void->Void;
    public var onError:String->Void;
    var player:PlayerInstance;
    var map:MapData;
    public var RAD:Int = MapData.RAD;
    private function new(client:Client,map:MapData)
    {
        buffer = [];
        this.client = client;
        this.map = map;
        trace("map: " + map);
    }
    public function setPlayer(player:PlayerInstance)
    {
        this.player = player;
    }
    public function send(tag:ServerTag,x:Int,y:Int,data:String="")
    {
        //trace('send: $tag $x $y $data');
        if (moving && tag != SAY && tag != EMOT)
        {
            //trace("---added to buffer---");
            buffer.push({tag: tag,x: x,y: y,data: data});
            return;
        }
        client.send('$tag $x $y $data');
    }
    public function update(player:PlayerInstance)
    {
        if (!moving) return;
        trace("p " + player + " dest " + dest);
        if (player.x != dest.x || player.y != dest.y)
        {
            trace('did not make it to dest player: ' + player.x + " " + player.y);
            moving = false;
            //if (onError != null) onError("did not make it to dest");
            return;
        }
        moving = false;
        if (dest.x != goal.x || dest.y != goal.y)
        {
            //extension
            trace("path extension!");
            goto(goal.x,goal.y);
            return;
        }
        //play buffer
        trace("play buffer, count: " + buffer.length);
        for (command in buffer)
        {
            send(command.tag,command.x,command.y,command.data);
        }
        buffer = [];
        dest = null;
        goal = null;
        init = null;
        if (onComplete != null) onComplete();
        trace("UPDATE");
    }
    public function clear()
    {
        dest = null;
        init = null;
        goal = null;
        moving = false;
        player = null;
        buffer = [];
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
    public function use(x:Int,y:Int):Program
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
    public function goto(x:Int,y:Int):Bool
    {
        if (player.x == x && player.y == y || moving) return false;
        //set pos
        var px = x - player.x;
        var py = y - player.y;
        if (px > MapData.RAD - 1) px = MapData.RAD - 1;
        if (py > MapData.RAD - 1) py = MapData.RAD - 1;
        if (px < -MapData.RAD) px = -MapData.RAD;
        if (py < -MapData.RAD) py = -MapData.RAD;
        //cords
        var start = new Coordinate(MapData.RAD,MapData.RAD);
        //map
        trace("map " + map);
        var map = new MapCollision(map.collisionChunk(player));
        //pathing
        var path = new Pathfinder(map);
        var paths:Array<Coordinate> = null;
        //move the end cords
        var tweakX:Int = 0;
        var tweakY:Int = 0;
        for (i in 0...3)
        {
            switch(i)
            {
                case 1:
                tweakX = x - player.x < 0 ? 1 : -1;
                case 2:
                tweakX = 0;
                tweakY = y - player.y < 0 ? 1 : -1;
            }
            var end = new Coordinate(px + MapData.RAD + tweakX,py + MapData.RAD + tweakY);
            paths = path.createPath(start,end,MANHATTAN,true);
            if (paths != null) break;
        }
        if (paths == null) 
        {
            if (onError != null) onError("can not generate path");
            trace("CAN NOT GENERATE PATH");
            return false;
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
        if (px == goal.x - player.x && py == goal.y - player.y)
        {
            trace("shift goal!");
            //shift goal as well
            goal.x += tweakX;
            goal.y += tweakY;
        }
        dest = new Pos(px + player.x,py + player.y);
        init = new Pos(player.x,player.y);
        movePlayer(data);
        return true;
    }
    private function comparePos(dest:Pos,goal:Pos)
    {

    }
    private inline function movePlayer(paths:Array<Pos>)
    {
        var string = "";
        for (path in paths)
        {
            string += " " + path.x + " " + path.y;
        }
        string = string.substring(1);
        send(MOVE,${player.x},${player.y},'@${++player.done_moving_seqNum} $string');
        var path = paths.pop();
        /*player.x = player.x + path.x;
        player.y = player.y + path.y;
        player.forced = true;*/
        if (client.relayIn != null) 
        {
            var eta = (path.x + path.y)/3;
            var string = '$PLAYER_MOVES_START\n${player.p_id} ${player.x} ${player.y} $eta $eta 0 $string';
            Timer.delay(function()
            {
                player.x += path.x;
                player.y += path.y;
                player.forced = true;
                client.relayIn.output.writeString('$PLAYER_UPDATE\n${player.toData()}\n#');
                player.x += -path.x;
                player.y += -path.y;
                player.forced = false;
            },Std.int(eta * 1000));
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
    public function self(index:Int=-1):Program
    {
        send(SELF,player.x,player.y," " + index);
        return this;
    }
    public function eat():Program
    {
        return self();
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