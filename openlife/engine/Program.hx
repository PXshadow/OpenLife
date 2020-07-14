package openlife.engine;
import openlife.data.map.MapData;
import openlife.data.Pos;
import openlife.client.Client;
import haxe.Timer;
import openlife.server.ServerTag;
import haxe.crypto.Base64;

class Program
{
    public var home:Pos = new Pos();
    var client:Client;
    public function new(client:Client)
    {
        this.client = client;
    }
    public function send(tag:ServerTag,x:Int,y:Int,data:String="")
    {
        trace('send: $tag $x $y $data');
        client.send('$tag $x $y $data');
    }
    public function update()
    {
        
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
    public function move(player:openlife.data.object.player.PlayerInstance,map:MapData,x:Int,y:Int):Program
    {
        trace('x ${player.x} y ${player.y}');
        if (Math.abs(player.x - x) > 8 || Math.abs(player.y - y) > 8)
        {
            trace("outside of 8 tile range");
            return this;
        }
        /*player.x = x;
        player.y = y;*/
        var currentX:Int = player.x;
        var currentY:Int = player.y;
        var dx:Int = 0;
        var dy:Int = 0;
        var mx:Array<Int> = [];
        var my:Array<Int> = [];
        var finish:Bool = false;
        //player 20 20
        //x y 30 30
        for (i in 0...8) //max tries to get to x and y 8
        {
            if (currentX != x)
            {
                dx = currentX < x ? 1 : -1;
            }
            if (currentY != y)
            {
                dy = currentY < y ? 1 : -1;
            }
            currentX += dx;
            currentY += dy;
            mx.push(currentX - player.x);
            my.push(currentY - player.y);
            trace('c $currentX $currentY');
            if (currentX == x && currentY == y)
            {
                finish = true;
                break;
            }
        }
        if (finish)
        {
            send(MOVE,${player.x},${player.y},'@${++player.done_moving_seqNum} ${mx.join(" ")} ${my.join(" ")}');
            player.x = x;
            player.y = y;
            player.forced = true;
            if (client.relay != null) 
            {
                //149 2404 0 1 0 0 0 0 0 0 -1 0.37 1 0 0 0 14.07 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 0
                //150 1009 0 0 0 00 0 0 -1 0 3 1 0 0 14.1 60 3.75 0 0 0 0 0
                var string = 'PU\n${player.toData()}\n#';
                trace("pu " + string);
                client.relay.output.writeString(string);
            }
        }else{
            trace("could not make it to destination");
        }
        return this;
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
        send(SELF,0,0," " + index);
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