package openlife.auto;

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
    var program:Program;
    var list:Vector<Int>;
    public var interp:Interpreter;
    public function new(program:Program,list:Vector<Int>=null)
    {
        this.program = program;
        this.list = list;
        interp = new Interpreter(list);
    }
    public function find(id:Array<Int>,map:MapData,player:PlayerInstance):Pos
    {
        var dis:Float = 2000;
        var pos:Pos = null;
        for (y in player.y - MapData.RAD...player.y + MapData.RAD)
        {
            for (x in player.x - MapData.RAD...player.x + MapData.RAD)
            {
                //trace("x: " + x + " y: " + y + " v: " + map.object.get(x,y));
                var array = map.object.get(x,y);
                if (array != null) for (o in array)
                {
                    if (id.indexOf(o) > -1)
                    {
                        var tdis = Math.abs(x - player.x) + Math.abs(y - player.y);
                        if (dis > tdis) 
                        {
                            dis = tdis;
                            pos = new Pos(x,y);
                        }
                        break;
                    }
                }
            }
        }
        return pos;
    }
}
typedef Auto = Automation; 