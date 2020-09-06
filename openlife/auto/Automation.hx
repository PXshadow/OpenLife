package openlife.auto;

import openlife.data.Pos;
import openlife.auto.Pathfinder.Coordinate;
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
    var following:Int;
    public function new(program:Program,map:MapData,player:PlayerInstance,list:Vector<Int>=null)
    {
        this.program = program;
        this.map = map;
        this.player = player;
        this.list = list;
        interp = new Interpreter(list);
    }
    public function goto(x:Int,y:Int)
    {
        if (player.x == x && player.y == y) return;
        if (Math.abs(player.x - x) >= MapData.RAD || Math.abs(player.y - y) >= MapData.RAD)
        {
            trace("OUT OF RANGE");
            return;
        }
        var start = new Coordinate(MapData.RAD,MapData.RAD);
        var sx = (x - player.x) + MapData.RAD;
        var sy = (y - player.y) + MapData.RAD;
        trace("sx " + sx + " sy " + sy);
        if (sx < 0) sx = 0;
        if (sy < 0) sy = 0;
        var end = new Coordinate(sx,sy);
        var map = new Map(map.collisionChunk(player));
        var path = new Pathfinder(map);
        trace("path:");
        for (y in 0...MapData.RAD * 2)
        {
            var string:String = "";
            for (x in 0...MapData.RAD * 2)
            {
                if (end.x == x && end.y == y)
                {
                    string += "?";
                    continue;
                }
                if (start.x == x && start.y == y)
                {
                    string += "@";
                    continue;
                }
                if (!map.isWalkable(x,y))
                {
                    string += "X";
                    continue;
                }
                string += " ";
            }
            Sys.println(string);
        }
        trace("start " + start + " end " + end);
        var paths = path.createPath(start,end,PRODUCT,true);
        if (paths == null) return;
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
        program.move(player,data);
    }
    public function test()
    {

    }
}
class Map implements openlife.auto.Pathfinder.MapHeader
{
    
    public var rows( default, null ):Int;
    public var cols( default, null ):Int;
    public var data:Vector<Bool>;

    public function new(data:Vector<Bool>)
    {
        this.data = data;
        cols = 32 + 1 * 0;
        rows = 32 + 1 * 0;
    }
    public function isWalkable( p_x:Int, p_y:Int ):Bool
    {
        return !data[p_x + p_y * (32)];
    }
}
typedef Auto = Automation; 