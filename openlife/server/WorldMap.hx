package openlife.server;
import openlife.data.object.ObjectHelper;
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
    public var SSWAMP = 0.4;  
    public var SYELLOW = 1;
    public var SGREY = 0.9;
    public var SSNOW = 0.5;
    public var SDESERT= 0.7;//0.5;
    public var SJUNGLE = 0.7;  

    public var SSNOWINGREY = 0.1;
    public var SOCEAN = 0.2;  
    public var SRIVER = 0.2;   
}

class WorldMap
{
    var objects:Vector<Array<Int>>;
    var objectHelpers:Vector<ObjectHelper>; 
    var floors:Vector<Int>;
    var biomes:Vector<Int>;
    
    public var timeObjectHelpers:Array<ObjectHelper>; 

    public var width:Int;
    public var height:Int;
    private var length:Int;

    // stuff for random generator
    private var seed:Int = 38383834;
    static inline final MULTIPLIER:Float = 48271.0;
    static inline final MAX_NUM:Int = 2147483647;
    static inline final MODULUS:Int = MAX_NUM;

    // used for creation of the map
    private var biomeTotalChance:Map<Int,Float>; 
    private var biomeObjectData:Map<Int, Array<ObjectData>>;

    public function new()
    {

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

    private function createVectors(length:Int)
    {
        this.length = length;

        objects = new Vector<Array<Int>>(length);
        objectHelpers = new Vector<ObjectHelper>(length);
        floors = new Vector<Int>(length);
        biomes = new Vector<Int>(length);

        timeObjectHelpers = [];
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

        generateBiomeObjectData();
        
        var generatedObjects = 0;

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

                //if(x < 200 || x > 600) continue;
                if (randomFloat() > 0.4) continue;
                
                var set:Bool = false;

                var biomeData = biomeObjectData[biomeInt];

                if(biomeData == null) continue;

                var random = randomFloat() * this.biomeTotalChance[biomeInt]; 
                var sumChance = 0.0;
                
                for (obj in biomeData) {
                    if (set) continue;

                    var chance = obj.mapChance;
                    sumChance += chance;

                    if (random <= sumChance) {
                        objects[x+y*width] = [obj.id];

                        //trace('generate: bi: $biomeInt id: ${obj.id} rand: $random sc: $sumChance');
                        set = true;
                        generatedObjects++;
                    }
                }
            }
        }

        trace('generatedObjects: $generatedObjects');
    }

    function generateBiomeObjectData()
    {
        this.biomeObjectData = [];
        this.biomeTotalChance = [];

        for (obj in Server.vector) {
            if (obj.mapChance == 0) continue;

            for(biome in obj.biomes){

                var biomeData = this.biomeObjectData[biome];
                if(biomeData == null){
                    biomeData = [];
                    this.biomeObjectData[biome] = biomeData;
                    this.biomeTotalChance[biome] = 0;
                }
                biomeData.push(obj);
                this.biomeTotalChance[biome] += obj.mapChance;

                //var objectDataTarget = Server.objectDataMap[obj.id];
                //if(objectDataTarget != null) trace('biome: $biome c:${obj.mapChance} tc:${this.biomeTotalChance[biome]} ${objectDataTarget.description}');
                
            }
        }
        //shuffleBiomeArray(buffer); //seeded random
        //biomeObjectData.push(buffer);
        //}
        //return biomeObjectData;
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

    public function setObjectId(x:Int, y:Int, ids:Array<Int>)
    {
        objects[index(x,y)] = ids;
    }

    public function getObjectHelper(x:Int, y:Int):ObjectHelper
    {
        //trace('objectHelper: $x,$y');
        return objectHelpers[index(x,y)];
    }

    public function setObjectHelper(x:Int, y:Int, objectHelper:ObjectHelper)
    {
        //trace('objectHelper: $x,$y');
        objectHelpers[index(x,y)] = objectHelper;
    }

    public function getFloorId(x:Int, y:Int):Int
    {
        return floors[index(x,y)];
    }

    public function setFloorId(x:Int, y:Int, floor:Int)
    {
        floors[index(x,y)] = floor;
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