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
        var start = new Coordinate(16,16);
        var sx = x - player.x + 16;
        var sy = y - player.y + 16;
        if (sx < 0) sx = 0;
        if (sy < 0) sy = 0;
        if (sx > 16 + 1) sx = 16 + 1;
        if (sy > 16 + 1) sy = 16 + 1;
        var end = new Coordinate(sx,sy);
        var path = new Pathfinder(new Map(map.collisionChunk(player)),1000);
        trace("path: " + start + " " + end);
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