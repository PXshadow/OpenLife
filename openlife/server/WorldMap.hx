package openlife.server;
import format.png.Reader;
#if (target.threaded)
import openlife.data.map.MapData;
import haxe.ds.Vector;
import openlife.data.FractalNoise;
import openlife.data.object.ObjectData;

import haxe.io.Bytes;

@:enum abstract BiomeTag(Int) from Int to Int
{
    public var GREEN = 0;
    public var SWAMP = 1;
    public var YELLOW = 2;
    public var GREY = 3;
    public var SNOW = 4;
    public var DESERT= 5;
    public var JUNGLE = 6;  

    // TODO // SNOWINGREY is snow biome on top of mountains. This biome should be therefore harder to pass then snow
    public var SNOWINGREY = 7; 
    public var OCEAN = 9;  // TODO
    public var RIVER = 13;  // TODO 
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

    public var CSNOWINGREY = "FF808080";
    public var COCEAN = "FF21007F";  
    public var CRIVER = "FF0026FF"; 
}

@:enum abstract BiomeSpeed(Float) from Float to Float
{
    public var SGREEN = 1;  
    public var SSWAMP = 0.2;  
    public var SYELLOW = 1;
    public var SGREY = 0.2;//0.8;
    public var SSNOW = 0.5;
    public var SDESERT= 0.5;//0.5;
    public var SJUNGLE = 0.5;  

    public var SSNOWINGREY = 0.1;
    public var SOCEAN = 0.2;  
    public var SRIVER = 0.2;   
}

class WorldMap
{
    var objects:Vector<Array<Int>>;
    var floors:Vector<Int>;
    public var biomes:Vector<Int>;

    public var width:Int;
    public var height:Int;
    private var length:Int;
    private var seed:Int = 38383834;
    static inline final MULTIPLIER:Float = 48271.0;
    static inline final MAX_NUM:Int = 2147483647;
    static inline final MODULUS:Int = MAX_NUM;

    public function new()
    {

    }
    private function shuffleBiomeArray(array:Array<ObjectData>)
    {
        if (array.length == 0)
            return;
        var temp:ObjectData;
        var j = 0;
        var k = 0;
        for (i in 0...6)
        {
            j = randomInt(array.length - 1);
            k = randomInt(array.length - 1);
            temp = array[j].clone();
            array[j] = array[k];
            array[k] = temp;
        }
    }
    private function generateSeed():Int
    {
        return seed = Std.int((seed * MULTIPLIER) % MODULUS);
    }
    private function randomInt(x:Int=MAX_NUM):Int
    {
        return Math.floor(generateSeed() / MODULUS * (x + 1));
    }
    private function randomFloat():Float
    {
        return generateSeed() / MODULUS;
    }
    public function createVectors(length:Int)
    {
        objects = new Vector<Array<Int>>(length);
        floors = new Vector<Int>(length);
        biomes = new Vector<Int>(length);
        this.length = length;
    }

    // The Server and Client map is saved in an array with y starting from bottom, 
    // The Map is saved with y starting from top. Therefore the map is y inversed during generation from picture
    public function generate()
    {
        var pngDir = "./map.png";
        var pngmap = readPixels(pngDir);
        
        width = pngmap.width;
        height = pngmap.height;
        length = width * height;       
        
        trace('map width: ' + width);
        trace('map height: ' + height);

        createVectors(length);

        var biomeObjectData = generateBiomeObjectData();

        for (y in 0...height){
            for (x in 0...width) {
                if(y % 100 == 0 && x == 0){
                    trace('generating map up to y: ' + y);
                }
                
                //var p = pngmap.data.getInt32(4*(x+xOffset+(y+yOffset)*pngmap.width));
                var p = pngmap.data.getInt32(4*(x + ((height - 1) - y) * pngmap.width));

                // ARGB, each 0-255
                //var a:Int = p>>>24;
                //var r:Int = (p>>>16)&0xff;
                //var g:Int = (p>>>8)&0xff;
                //var b:Int = (p)&0xff;
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
                //biomeInt = x % 100;

                
                biomes[x+y*width] = biomeInt;
                objects[x+y*width] = [0];

                // TODO this is work around to make object creation faster
                if(x < 350 || x > 450) continue;
                if (randomFloat() > 0.4) continue;
                
                var set:Bool = false;

                for (obj in biomeObjectData[biomeInt]) {
                    if (set) continue;
                    if (randomFloat() > obj.mapChance) {
                        objects[x+y*width] = [obj.id];
                        set = true;
                    }
                }
            }
        }
    }

    function generateBiomeObjectData():Array<Array<ObjectData>>
    {
        var biomeObjectData:Array<Array<ObjectData>> = [];

        for (biomeInt in 0...20){
            var buffer:Array<ObjectData> = [];
            for (obj in Server.vector) {
                if (obj.mapChance == 0) continue;
                if (obj.biomes.indexOf(biomeInt) != -1)
                    buffer.push(obj);
            }
            shuffleBiomeArray(buffer); //seeded random
            biomeObjectData.push(buffer);
        }
        return biomeObjectData;
    }
    
    function readPixels(file:String):{data:Bytes, width:Int, height:Int} {
        var handle = sys.io.File.read(file, true);
        var d = new Reader(handle).read();
        var hdr = format.png.Tools.getHeader(d);
        var ret = {
            data:format.png.Tools.extract32(d),
            width:hdr.width,
            height:hdr.height
        };
        handle.close();
        return ret;
    }

    public function getChunk(x:Int,y:Int,width:Int,height:Int):WorldMap
    {
        var map = new WorldMap();
        var length = width * height;
        map.createVectors(length);
        for (px in 0...width)
        {
            for (py in 0...height)
            {
                var localIndex = px + py * width;
                var index = index(x + px, y + py); 
                
                map.biomes[localIndex] = biomes[index];
                map.floors[localIndex] = floors[index];
                map.objects[localIndex] = objects[index];
            }
        }
        return map;
    }

    public function getBiomeSpeed(x:Int, y:Int):Float 
    {
        var biomeType = biomes[index(x, y)];

        //trace('${ x },${ y }:BI ${ biomeType }');

        //return 0.2;

        return switch biomeType {
            case GREEN: SGREEN;
            case SWAMP: SSWAMP;
            case YELLOW: SYELLOW;
            case GREY: SGREY;
            case SNOW: SSNOW;
            case DESERT: SDESERT;
            case JUNGLE: SJUNGLE;
            case SNOWINGREY: SSNOWINGREY;
            case OCEAN: SOCEAN;
            case RIVER: SRIVER;
            default: 1;
        }
    } 

    public function getObjectId(x:Int, y:Int):Array<Int>
    {
        return objects[index(x,y)];
    }

    public function getFloorId(x:Int, y:Int):Int
    {
        return floors[index(x,y)];
    }

    public function setObjectId(x:Int,y:Int,id:Array<Int>)
    {
        objects[index(x,y)] = id;
        //if (floorBool) floors[index(x,y)] = id[0];
    }

    private inline function index(x:Int,y:Int):Int
    {
        // Dont know why yet, but y seems to be right if -1
        y -= 1;

        // make map round x wise
        x = x % this.width;
        if(x < 0) x += this.width; 
        //else if(x >= this.width) x -= this.width;

        // make map round y wise
        y = y % this.height;
        if(y < 0) y += this.height; 
        //else if(y >= this.height) y -= this.height;

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
            string += ' ${biomes[i]}:${floors[i]}:$obj';
        }
        return string.substr(1);
    }

    public function findClosest(){
        
    }
}
#end