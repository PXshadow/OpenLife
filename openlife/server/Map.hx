package openlife.server;
#if (target.threaded)
import openlife.data.map.MapData;
import haxe.ds.Vector;
import openlife.data.FractalNoise;

import haxe.io.Bytes;
import format.png.Reader;
import format.png.Tools;

@:enum abstract BiomeTag(Int) from Int to Int
{
    public var GREEN = 0;
    public var SWAMP = 1;
    public var YELLOW = 2;
    public var GREY = 3;
    public var SNOW = 4;
    public var DESERT= 5;
    public var JUNGLE = 6;  

    public var SNOWINGREY = 7; // TODO 
    public var OCEAN = 9;  // TODO
    public var RIVER = 10;  // TODO 
}

@:enum abstract BiomeMapColor(String) from String to String
{
    public var CGREEN = "none";  // is auto generated
    public var CSWAMP = "none";  // is auto generated
    public var CYELLOW = "FFDCFF2D";
    public var CGREY = "FF404040";
    public var CSNOW = "FFFFFFFF";
    public var CDESERT= "FFFF0000";
    public var CJUNGLE = "FF007F0E";  

    public var CSNOWINGREY = "FF808080"; // TODO 
    public var COCEAN = "FF21007F";  // TODO
    public var CRIVER = "FF0026FF";  // TODO 
}

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
        
        generate();
    }
    public function generate()
    {
        var pngDir = "./map.png";
        var pngmap = readPixels(pngDir);
        trace ("hello3");
       

        //height = 100; //pngmap.height;
        //width = 100; //pngmap.width;
        //length = width * height;

        trace (length);

        objects = new Vector<Array<Int>>(length);
        floor = new Vector<Int>(length);
        biome = new Vector<Int>(length);

        var xOffset = 420;
        var yOffset = 300;

        for (y in 0...height){
            for (x in 0...width) {
                var p = pngmap.data.getInt32(4*(x+xOffset+(y+yOffset)*pngmap.width));
                // ARGB, each 0-255
                var a:Int = p>>>24;
                var r:Int = (p>>>16)&0xff;
                var g:Int = (p>>>8)&0xff;
                var b:Int = (p)&0xff;
                // Or, AARRGGBB in hex:
                var hex:String = StringTools.hex(p,8);
                
                

                var biomeInt;
                

                switch hex {
                    case CYELLOW: biomeInt = YELLOW;
                    case CGREY: biomeInt = GREY;
                    case CSNOW: biomeInt = SNOW;
                    case CDESERT: biomeInt = DESERT;
                    case CJUNGLE: biomeInt = JUNGLE;
                    case CSNOWINGREY: biomeInt = SNOWINGREY;
                    case COCEAN: biomeInt = OCEAN;
                    case CRIVER: biomeInt = RIVER;
                    default: biomeInt = GREEN;
                }
                if(biomeInt == GREEN){
                    //trace('${ x },${ y }:BI ${ biomeInt },${ r },${ g },${ b } - ${ StringTools.hex(p,8) }');
                }
                biome[x+y*width] = biomeInt;
            }
        }


        


        var x:Int = 0;
        var y:Int = 0;
        for (i in 0...length)
        {
// swamp = 1

            //biome[i] = i % 100;
            //biome[i] = SNOW;
            
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

    function readPixels(file:String):{data:Bytes, width:Int, height:Int} {
        var handle = sys.io.File.read(file, true);
        var d = new format.png.Reader(handle).read();
        var hdr = format.png.Tools.getHeader(d);
        var ret = {
            data:format.png.Tools.extract32(d),
            width:hdr.width,
            height:hdr.height
        };
        handle.close();
        return ret;
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
        //trace("length: ");
        //trace(length);
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