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
    public function goto(id:Array<Int>):Bool
    {
        var pos = select(id);
        if (pos == null) return false;
        program.goto(pos.x,pos.y);
        return true;
    }
    public function select(id:Array<Int>):Pos
    {
        var array = ObjectBake.dummies.get(id[0]);
        trace("array " + array);
        if (array != null) id = id.concat(array);
        var dis:Float = 2000;
        var pos:Pos = null;
        @:privateAccess for (y in program.player.y - MapData.RAD...program.player.y + MapData.RAD)
        {
            @:privateAccess for (x in program.player.x - MapData.RAD...program.player.x + MapData.RAD)
            {
                //trace("x: " + x + " y: " + y + " v: " + map.object.get(x,y));
                var array = @:privateAccess program.map.object.get(x,y);
                if (array != null) for (o in array)
                {
                    if (id.indexOf(o) > -1)
                    {
                        @:privateAccess var tdis = Math.abs(x - program.player.x) + Math.abs(y - program.player.y);
                        if (dis > tdis) 
                        {
                            dis = tdis;
                            pos = new Pos(x,y);
                        }
                    }
                }
            }
        }
        return pos;
    }
    public function get(id:Array<Int>):Array<Pos>
    {
        var list:Array<Pos> = [];
        @:privateAccess for (y in program.player.y - MapData.RAD...program.player.y + MapData.RAD)
        {
            @:privateAccess for (x in program.player.x - MapData.RAD...program.player.x + MapData.RAD)
            {
                //trace("x: " + x + " y: " + y + " v: " + map.object.get(x,y));
                var array = @:privateAccess program.map.object.get(x,y);
                if (array != null) for (o in array)
                {
                    if (id.indexOf(o) > -1)
                    {
                        list.push(new Pos(x,y));
                    }
                }
            }
        }
        return list;
    }
    public function food():Pos
    {
        @:privateAccess for (y in program.player.y - MapData.RAD...program.player.y + MapData.RAD)
        {
            @:privateAccess for (x in program.player.x - MapData.RAD...program.player.x + MapData.RAD)
            {
                //trace("x: " + x + " y: " + y + " v: " + map.object.get(x,y));
                var array = @:privateAccess program.map.object.get(x,y);
                if (array != null) for (id in array)
                {
                    var obj = new ObjectData(id);
                    if (obj.foodValue > 0)
                    {
                        return new Pos(x,y);
                    }
                }
            }
        }
        return null;
    }
}
typedef Auto = Automation; 