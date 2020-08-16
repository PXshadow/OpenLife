package openlife.auto;

import openlife.data.object.ObjectData;
import haxe.ds.Vector;
import openlife.data.object.player.PlayerInstance;
import openlife.data.map.MapData;
import openlife.engine.Program;


/**
 * nick name auto, will be a powerful class that uses program, and transition data to do automatic tasks.
 */
class Automation
{
    var program:Program;
    var map:MapData;
    var player:PlayerInstance;
    var list:Vector<Int>;
    public var interp:Interpreter;
    public function new(program:Program,map:MapData,player:PlayerInstance,list:Vector<Int>=null)
    {
        this.program = program;
        this.map = map;
        this.player = player;
        this.list = list;
        interp = new Interpreter(list);
    }
}
typedef Auto = Automation; 