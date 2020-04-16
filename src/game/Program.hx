package game;
import data.Pos;
import client.Client;
import haxe.Timer;
import data.GameData;
import server.ServerTag;
import haxe.crypto.Base64;
import data.object.ObjectCode;
#if openfl
import motion.Actuate;
import openfl.display.Tile;
#end
#if nativeGen @:nativeGen #end
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
    public function force()
    {
        //client.send("FORCE");
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
    public function step(x:Int,y:Int,seq:Int,mx:Int,my:Int):Program
    {
        send(MOVE,x,y,'@$seq $mx $my');
        return this;
    }
    public function move(x:Int,y:Int,seq:Int,list:Array<Pos>):Program
    {
        var moveString = "";
        for (pos in list) moveString += ' ${pos.x} ${pos.y}';
        send(MOVE,x,y,'@$seq$moveString');
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
    #if openfl
    //visual
    public function apply(target:String,properties:Dynamic):Program
    {
        var array = getTiles(target);
        if (array.length > 0)
        {
            var fields = Reflect.fields(properties);
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
        /*var targets:Array<Tile> = [];
        var list = ObjectCode.id(target);
        if (list.length == 0) return targets;
        var cx:Int = Main.player.ix;
        var cy:Int = Main.player.iy;
        for (j in cy - range...cy + range)
        {
            for (i in cx - range...cx + range)
            {
                if (list.indexOf(Game.data.map.object.get(i,j)[0]) >= 0) for(tile in Game.data.tileData.object.get(i,j)) targets.push(tile);
            }
        }
        return targets;*/
        return [];
    }
    #end
}