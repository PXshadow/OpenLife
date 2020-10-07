package openlife.server;
#if (target.threaded)
import openlife.data.map.MapData;
import haxe.ds.Vector;
import openlife.data.FractalNoise;
class Map
{
    var objects:Vector<Array<Int>>;
    var floor:Vector<Int>;
    var biome:Vector<Int>;
    public static inline var width:Int = 32;
    public static inline var height:Int = 30;
    private static inline var length:Int = width * height;
    var server:Server;
    public function new(server:Server)
    {
        this.server = server;
        objects = new Vector<Array<Int>>(length);
        floor = new Vector<Int>(length);
        biome = new Vector<Int>(length);
        generate();
    }
    public function generate()
    {
        var x:Int = 0;
        var y:Int = 0;
        for (i in 0...length)
        {
            biome[i] = 0;
            objects[i] = [0];
            floor[i] = 0;//898;
            if (++x > width)
            {
                x = 0;
                y++;
            }
        }
        //set(15,15,[32]);
        //set(16,15,[33]); //33
        //set(15,16,[121]);
        //set(16,16,[121]);
        set(16,20,[434,33,33,33]);

        for (x in 10...16) set(x,10,[2959]);
    }
    public function get(x:Int,y:Int,delete:Bool=false,floorBool:Bool=false):Array<Int>
    {
        var i = floorBool ? [floor[index(x,y)]] : objects[index(x,y)];
        if (delete) set(x,y,[0],floorBool);
        return i;
    }
    public function set(x:Int,y:Int,id:Array<Int>,floorBool:Bool=false)
    {
        if (!floorBool) objects[index(x,y)] = id;
        if (floorBool) floor[index(x,y)] = id[0];
    }
    private inline function index(x:Int,y:Int):Int
    {
        var i = x + y * width;
        return i;
    }
    private inline function sigmoid(input:Float,knee:Float):Float
    {
        var shifted = input * 2 -1;
        var sign = input < 0 ? -1 : 1;
        var k = -1 - knee;
        var abs = Math.abs(shifted);
        var out = sign * abs * k / (1 + k - abs);
        return (out + 1) * 0.5;
    }
    public function toString():String
    {
        var string = "";
        for (i in 0...length)
        {
            var obj = MapData.stringID(objects[i]);
            string += ' ${biome[i]}:${floor[i]}:$obj';
        }
        return string.substr(1);
    }
    public function findClosest(){
        
    }
}
#end