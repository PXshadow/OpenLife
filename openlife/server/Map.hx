package openlife.server;
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
        FractalNoise.setXYRandomSeed( 9877 );
        var x:Int = 0;
        var y:Int = 0;
        for (i in 0...length)
        {
            //trace("p " + x + " " + y + " fractuals " + FractalNoise.getXYRandom(x,y));
            var density = FractalNoise.getXYFractal(x,y,0.1,0.25);
            density = sigmoid(density,0.1);
            density *= 0.4;

            var rand = FractalNoise.getXYRandom(x,y);
            if (rand/20000 < density)
            {
                //set biome
                biome[i] = 3;
            }else{
                biome[i] = 0;
            }
            objects[i] = [0];
            floor[i] = 0;//898;
            if (++x > width)
            {
                x = 0;
                y++;
            }
        }
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
}