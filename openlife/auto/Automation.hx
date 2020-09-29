package openlife.auto;

import openlife.data.object.ObjectData;
import openlife.resources.ObjectBake;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapData;
import openlife.data.Pos;
import haxe.ds.Vector;
import openlife.engine.Program;


/**
 * nick name auto, will be a powerful class that uses program, and transition data to do automatic tasks.
 */
 @:expose("Automation")
class Automation
{
    public var program:Program;
    var list:Vector<Int>;
    public var interp:Interpreter;
    public function new(program:Program,list:Vector<Int>=null)
    {
        this.program = program;
        this.list = list;
        interp = new Interpreter(list);
    }
    public function goto(id:Array<Int>,buffer:Pos->Void)
    {
        var pos = select(id);
        if (pos == null) return;
        if (!program.goto(pos.x,pos.y)) return;
        buffer(pos);
    }
    public function foreach(func:(x:Int,y:Int,array:Array<Int>)->Bool,repeat:Bool=true)
    {
        @:privateAccess for (y in program.player.y - MapData.RAD...program.player.y + MapData.RAD)
        {
            @:privateAccess for (x in program.player.x - MapData.RAD...program.player.x + MapData.RAD)
            {
                var array = @:privateAccess program.map.object.get(x,y);
                if (array != null) 
                {
                    var bool = func(x,y,array);
                    if (!repeat && bool) return;
                }
            }
        }
    }
    public function select(id:Array<Int>):Pos
    {
        var array = ObjectBake.dummies.get(id[0]);
        if (array != null) id = id.concat(array);
        var dis:Float = 2000;
        var pos:Pos = null;
        foreach(function(x:Int,y:Int,array:Array<Int>)
        {
            for (i in 0...array.length)
            {
                if (id.indexOf(array[i]) > -1)
                {
                    @:privateAccess var tdis = Math.abs(x - program.player.x) + Math.abs(y - program.player.y);
                    if (dis > tdis) 
                    {
                        dis = tdis;
                        pos = new Pos(x,y);
                        if (i > 0) pos.y += new ObjectData(array[0]).noBackAcess ? -1 : 0;
                    }
                }
            }
            return true;
        });
        return pos;
    }
    public function get(id:Array<Int>):Array<Array<Pos>>
    {
        var obj:Array<Pos> = [];
        var pos:Array<Pos> = [];
        foreach(function(x:Int,y:Int,array:Array<Int>)
        {
            for (i in 0...array.length)
            {
                if (id.indexOf(array[i]) > -1)
                {
                    var p = new Pos(x,y);
                    pos.push(p);
                    if (i > 0) 
                    {
                        var sy = new ObjectData(array[0]).noBackAcess ? -1 : 0;
                        var p = p.clone();
                        p.y += sy;
                        obj.push(p);
                    }else{
                        obj.push(p);
                    }
                    
                }
            }
            return true;
        });
        return [pos,obj];
    }
    public function food()
    {
        trace("food!");
        var pos:Pos = null;
        foreach(function(x:Int, y:Int, array:Array<Int>)
        {
            for (i in 0...array.length)
            {
                var obj = new ObjectData(array[i]);
                if (obj.foodValue > 0)
                {
                    pos = new Pos(x,y);
                    var sy:Int = 0;
                    if (i > 0) sy = new ObjectData(array[0]).noBackAcess ? -1 : 0;
                    trace("pos " + pos);
                    //no repeat so this is last function call
                    if (!program.goto(pos.x,pos.y + sy)) return true;
                    program.use(pos.x,pos.y);
                    program.self();
                    return true;
                }
            }
            return false;
        },false);
    }
}
typedef Auto = Automation; 